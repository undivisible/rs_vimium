import { spawn, spawnSync } from "node:child_process";
import { createServer } from "node:http";
import { existsSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const args = new Set(process.argv.slice(2));
const skipBuild = args.has("--skip-build");
const skipVimium = args.has("--skip-vimium");
const samples = Number(process.env.RS_VIMIUM_BENCH_SAMPLES ?? "8");
const warmup = Number(process.env.RS_VIMIUM_BENCH_WARMUP ?? "2");
const linkCount = Number(process.env.RS_VIMIUM_BENCH_LINKS ?? "160");
const crepusBin = process.env.CREPUS_BIN || "crepus";
const chromeBin = process.env.CHROME_BIN || [
  "/Users/undivisible/Library/Caches/ms-playwright/chromium-1223/chrome-mac-arm64/Google Chrome for Testing.app/Contents/MacOS/Google Chrome for Testing",
  "/Users/undivisible/Library/Caches/ms-playwright/chromium-1217/chrome-mac-arm64/Google Chrome for Testing.app/Contents/MacOS/Google Chrome for Testing",
  "/Applications/Google Chrome for Testing.app/Contents/MacOS/Google Chrome for Testing",
  "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"
].find((path) => path && existsSync(path));
const vimiumPath = process.env.VIMIUM_PATH || "/tmp/vimium-bench-src";

if (!chromeBin) {
  throw new Error("Set CHROME_BIN to a Chrome or Chrome-for-Testing binary.");
}

if (!skipBuild) {
  const result = spawnSync(crepusBin, ["webext", "build", "--app", "."], {
    cwd: root,
    stdio: "inherit"
  });
  if (result.status !== 0) {
    process.exit(result.status ?? 1);
  }
}

const rsExtension = join(root, "dist", "unpacked");
if (!existsSync(join(rsExtension, "manifest.json"))) {
  throw new Error("dist/unpacked/manifest.json is missing. Run bun run build first.");
}
const cargoToml = readFileSync(join(root, "Cargo.toml"), "utf8");
const releaseProfile = {};
let inReleaseProfile = false;
for (const line of cargoToml.split("\n")) {
  const trimmed = line.trim();
  if (trimmed === "[profile.release]") {
    inReleaseProfile = true;
    continue;
  }
  if (inReleaseProfile && trimmed.startsWith("[") && trimmed.endsWith("]")) break;
  if (!inReleaseProfile || !trimmed.includes("=")) continue;
  const [key, value] = trimmed.split("=").map((part) => part.trim().replace(/^"|"$/g, ""));
  releaseProfile[key] = value;
}

function sleep(ms) {
  return new Promise((resolveSleep) => setTimeout(resolveSleep, ms));
}

function commandOutput(command, args = []) {
  const result = spawnSync(command, args, { encoding: "utf8" });
  return result.status === 0 ? result.stdout.trim() : null;
}

function machineInfo() {
  const memsize = Number(commandOutput("sysctl", ["-n", "hw.memsize"]));
  const osName = commandOutput("sw_vers", ["-productName"]);
  const osVersion = commandOutput("sw_vers", ["-productVersion"]);
  const osBuild = commandOutput("sw_vers", ["-buildVersion"]);
  return {
    model: commandOutput("sysctl", ["-n", "hw.model"]),
    arch: commandOutput("uname", ["-m"]),
    cpu: commandOutput("sysctl", ["-n", "machdep.cpu.brand_string"]),
    physical_cpus: Number(commandOutput("sysctl", ["-n", "hw.physicalcpu"])),
    logical_cpus: Number(commandOutput("sysctl", ["-n", "hw.logicalcpu"])),
    memory_gib: Number.isFinite(memsize) ? Number((memsize / 1024 / 1024 / 1024).toFixed(1)) : null,
    os: [osName, osVersion, osBuild ? `(${osBuild})` : null].filter(Boolean).join(" ")
  };
}

async function waitForHttp(url, attempts = 100) {
  for (let i = 0; i < attempts; i += 1) {
    try {
      const response = await fetch(url);
      if (response.ok) return response;
    } catch {}
    await sleep(100);
  }
  throw new Error(`Timed out waiting for ${url}`);
}

class Cdp {
  constructor(socket) {
    this.socket = socket;
    this.nextId = 1;
    this.pending = new Map();
    socket.onmessage = (event) => {
      const message = JSON.parse(event.data);
      if (!message.id || !this.pending.has(message.id)) return;
      const pending = this.pending.get(message.id);
      this.pending.delete(message.id);
      if (message.error) {
        pending.reject(new Error(JSON.stringify(message.error)));
      } else {
        pending.resolve(message.result);
      }
    };
  }

  send(method, params = {}, timeout = 15000) {
    const id = this.nextId;
    this.nextId += 1;
    this.socket.send(JSON.stringify({ id, method, params }));
    return new Promise((resolveSend, rejectSend) => {
      const timer = setTimeout(() => {
        this.pending.delete(id);
        rejectSend(new Error(`Timed out in ${method}`));
      }, timeout);
      this.pending.set(id, {
        resolve: (value) => {
          clearTimeout(timer);
          resolveSend(value);
        },
        reject: (error) => {
          clearTimeout(timer);
          rejectSend(error);
        }
      });
    });
  }
}

async function connectTarget(target) {
  const socket = new WebSocket(target.webSocketDebuggerUrl);
  await new Promise((resolveOpen, rejectOpen) => {
    socket.onopen = resolveOpen;
    socket.onerror = rejectOpen;
  });
  return { socket, cdp: new Cdp(socket) };
}

async function evaluate(cdp, expression, timeout = 15000) {
  const result = await cdp.send("Runtime.evaluate", {
    expression,
    returnByValue: true,
    awaitPromise: true
  }, timeout);
  if (result.exceptionDetails) {
    throw new Error(result.exceptionDetails.text || "Runtime.evaluate failed");
  }
  return result.result?.value;
}

async function dispatchKey(cdp, key, options = {}) {
  const code = options.code || (/^[a-z]$/i.test(key) ? `Key${key.toUpperCase()}` : key === "/" || key === "?" ? "Slash" : key);
  const vk = options.vk || (key === "/" || key === "?" ? 191 : key === "Escape" ? 27 : key.toUpperCase().charCodeAt(0));
  const base = {
    key,
    code,
    windowsVirtualKeyCode: vk,
    nativeVirtualKeyCode: vk,
    modifiers: options.modifiers || 0
  };
  await cdp.send("Input.dispatchKeyEvent", {
    ...base,
    type: "keyDown",
    text: options.text ?? (key.length === 1 ? key : ""),
    unmodifiedText: options.unmodifiedText ?? (key.length === 1 ? key : "")
  });
  await cdp.send("Input.dispatchKeyEvent", { ...base, type: "keyUp" });
}

function stats(values) {
  const clean = values.filter((value) => value != null && Number.isFinite(value)).sort((a, b) => a - b);
  const percentile = (p) => {
    if (clean.length === 0) return null;
    return clean[Math.min(clean.length - 1, Math.ceil(clean.length * p) - 1)];
  };
  const sum = clean.reduce((acc, value) => acc + value, 0);
  return {
    samples: values.map((value) => value == null ? null : Number(value.toFixed(3))),
    median_ms: clean.length ? Number(percentile(0.5).toFixed(3)) : null,
    p90_ms: clean.length ? Number(percentile(0.9).toFixed(3)) : null,
    min_ms: clean.length ? Number(clean[0].toFixed(3)) : null,
    max_ms: clean.length ? Number(clean.at(-1).toFixed(3)) : null,
    mean_ms: clean.length ? Number((sum / clean.length).toFixed(3)) : null,
    failures: values.length - clean.length
  };
}

async function repeat(name, fn) {
  const values = [];
  const bounded = async (index, isWarmup) => {
    return await Promise.race([
      fn(index, isWarmup),
      sleep(6000).then(() => null)
    ]);
  };
  for (let i = 0; i < warmup; i += 1) {
    try {
      await bounded(i, true);
    } catch {}
  }
  for (let i = 0; i < samples; i += 1) {
    try {
      values.push(await bounded(i, false));
    } catch {
      values.push(null);
    }
  }
  return { name, ...stats(values) };
}

function benchPageHtml() {
  const controls = Array.from({ length: linkCount }, (_, i) => {
    return `<a href="#link-${i}" style="display:block;padding:8px;border:1px solid #999">Link ${i}</a><button style="display:block;padding:8px">Button ${i}</button>`;
  }).join("");
  return `<!doctype html><html><head><meta charset="utf-8"><title>rs_vimium benchmark</title></head><body tabindex="0" style="font:16px system-ui;margin:24px"><p>needle alpha beta gamma delta</p><div style="display:grid;grid-template-columns:repeat(5,1fr);gap:10px">${controls}</div><div style="height:2200px"></div></body></html>`;
}

async function withServer(fn) {
  const server = createServer((_request, response) => {
    response.writeHead(200, { "content-type": "text/html" });
    response.end(benchPageHtml());
  });
  await new Promise((resolveListen) => server.listen(0, "127.0.0.1", resolveListen));
  try {
    return await fn(`http://127.0.0.1:${server.address().port}/bench.html`);
  } finally {
    server.close();
  }
}

function chromeArgs(port, profile, extensionPath, url = "about:blank") {
  return [
    `--remote-debugging-port=${port}`,
    `--user-data-dir=${profile}`,
    "--headless=new",
    "--no-first-run",
    "--no-default-browser-check",
    "--disable-gpu",
    "--window-size=1280,900",
    `--disable-extensions-except=${extensionPath}`,
    `--load-extension=${extensionPath}`,
    url
  ];
}

async function withChrome(extensionPath, initialUrl, fn) {
  const port = 10000 + Math.floor(Math.random() * 1000);
  const profile = mkdtempSync(join(tmpdir(), "rs-vimium-bench-"));
  const chrome = spawn(chromeBin, chromeArgs(port, profile, extensionPath, initialUrl), {
    stdio: ["ignore", "pipe", "pipe"]
  });
  const exited = new Promise((resolveExit) => chrome.once("exit", resolveExit));
  try {
    await waitForHttp(`http://127.0.0.1:${port}/json/version`);
    await sleep(1500);
    return await fn(port);
  } finally {
    chrome.kill("SIGTERM");
    await Promise.race([exited, sleep(1500)]);
    if (chrome.exitCode == null) {
      chrome.kill("SIGKILL");
      await Promise.race([exited, sleep(500)]);
    }
    rmSync(profile, { recursive: true, force: true });
  }
}

async function targetList(port) {
  return await (await fetch(`http://127.0.0.1:${port}/json/list`)).json();
}

async function findPageTarget(port, urlPrefix) {
  for (let i = 0; i < 40; i += 1) {
    const targets = await targetList(port);
    const target = targets.find((entry) => entry.type === "page" && entry.url.startsWith(urlPrefix)) || targets.find((entry) => entry.type === "page");
    if (target) return target;
    await sleep(100);
  }
  throw new Error("No page target found.");
}

async function preparePage(cdp, url, suffix) {
  await cdp.send("Page.navigate", { url: `${url}?sample=${suffix}` });
  for (let i = 0; i < 100; i += 1) {
    const ready = await evaluate(cdp, "document.readyState === 'complete'", 2500);
    if (ready) break;
    await sleep(50);
  }
  await sleep(900);
  await cdp.send("Page.bringToFront");
  await evaluate(cdp, "document.body.focus(); true", 2500);
}

async function contentBenchmarks(extensionPath, url, selectors) {
  return await withChrome(extensionPath, url, async (port) => {
    const pageTarget = await findPageTarget(port, url);
    const { socket, cdp } = await connectTarget(pageTarget);
    try {
      await cdp.send("Runtime.enable");
      await cdp.send("Page.enable");
      await cdp.send("Page.bringToFront");
      const mutationAction = async (name, selector, key, options = {}) => {
        return await repeat(name, async (index, isWarmup) => {
          await preparePage(cdp, url, `${name}-${isWarmup ? "warmup" : "sample"}-${index}`);
          await evaluate(cdp, `
            document.body.focus();
            globalThis.__rsVimiumBenchPromise = new Promise((resolve) => {
              const selector = ${JSON.stringify(selector)};
              const seenRoots = new WeakSet();
              const roots = [document];
              const start = performance.now();
              const observeRoot = (root, observer) => {
                if (seenRoots.has(root)) return;
                seenRoots.add(root);
                observer.observe(root, { childList: true, subtree: true, attributes: true });
              };
              const findDeep = (root) => {
                const found = root.querySelector(selector);
                if (found) return found;
                for (const element of root.querySelectorAll("*")) {
                  if (element.shadowRoot) roots.push(element.shadowRoot);
                }
                while (roots.length) {
                  const next = roots.shift();
                  if (next !== root) {
                    const deepFound = findDeep(next);
                    if (deepFound) return deepFound;
                  }
                }
                return null;
              };
              const done = () => {
                for (const element of document.querySelectorAll("*")) {
                  if (element.shadowRoot) observeRoot(element.shadowRoot, observer);
                }
                if (findDeep(document)) {
                  observer.disconnect();
                  resolve(performance.now() - start);
                }
              };
              const observer = new MutationObserver(done);
              observeRoot(document.documentElement, observer);
              done();
              setTimeout(() => {
                observer.disconnect();
                resolve(null);
              }, 2500);
            });
            true
          `, 2500);
          await dispatchKey(cdp, key, options);
          return await evaluate(cdp, "globalThis.__rsVimiumBenchPromise", 3500);
        });
      };
      const scroll = await repeat("scroll_j", async (index, isWarmup) => {
        await preparePage(cdp, url, `scroll_j-${isWarmup ? "warmup" : "sample"}-${index}`);
        await evaluate(cdp, `
          scrollTo(0, 0);
          document.body.focus();
          globalThis.__rsVimiumBenchPromise = new Promise((resolve) => {
            const start = performance.now();
            const done = () => {
              if (scrollY > 0) {
                removeEventListener("scroll", done, true);
                resolve(performance.now() - start);
              }
            };
            addEventListener("scroll", done, true);
            setTimeout(() => {
              removeEventListener("scroll", done, true);
              resolve(null);
            }, 1200);
          });
          true
        `, 2500);
        await dispatchKey(cdp, "j");
        return await evaluate(cdp, "globalThis.__rsVimiumBenchPromise", 2500);
      });
      return [
        scroll,
        await mutationAction("link_hints_f", selectors.hints, "f"),
        await mutationAction("vomnibar_o", selectors.vomnibar, "o"),
        await mutationAction("help_question", selectors.help, "?", { code: "Slash", vk: 191, text: "?", unmodifiedText: "/", modifiers: 8 }),
        await mutationAction("find_slash", selectors.find, "/", { code: "Slash", vk: 191, text: "/" })
      ];
    } finally {
      socket.close();
    }
  });
}

function stripJsonComments(source) {
  let output = "";
  let inString = false;
  let escape = false;
  for (let i = 0; i < source.length; i += 1) {
    const char = source[i];
    const next = source[i + 1];
    if (inString) {
      output += char;
      if (escape) {
        escape = false;
      } else if (char === "\\") {
        escape = true;
      } else if (char === "\"") {
        inString = false;
      }
      continue;
    }
    if (char === "\"") {
      inString = true;
      output += char;
      continue;
    }
    if (char === "/" && next === "/") {
      while (i < source.length && source[i] !== "\n") i += 1;
      output += "\n";
      continue;
    }
    output += char;
  }
  return output;
}

function prepareVimium() {
  if (skipVimium || !existsSync(join(vimiumPath, "manifest.json"))) return null;
  const prepared = join(tmpdir(), "vimium-unpacked-bench");
  rmSync(prepared, { recursive: true, force: true });
  const rsync = spawnSync("rsync", [
    "-a",
    `${vimiumPath}/`,
    `${prepared}/`,
    "--exclude",
    ".git",
    "--exclude",
    "dist",
    "--exclude",
    "tests",
    "--exclude",
    "test_harnesses",
    "--exclude",
    "build_scripts",
    "--exclude",
    "make.js",
    "--exclude",
    "deno.json",
    "--exclude",
    "deno.lock"
  ]);
  if (rsync.status !== 0) return null;
  const manifestPath = join(prepared, "manifest.json");
  const manifest = JSON.parse(stripJsonComments(readFileSync(manifestPath, "utf8")));
  writeFileSync(manifestPath, JSON.stringify(manifest, null, 2));
  return { path: prepared, version: manifest.version };
}

const rsSelectors = {
  hints: ".vc-hint",
  vomnibar: ".vc-vomnibar",
  help: ".vc-overlay",
  find: ".vc-find"
};
const vimiumSelectors = {
  hints: ".vimiumHintMarker,.internal-vimium-hint-marker",
  vomnibar: "iframe.vomnibar-frame",
  help: "iframe.vimium-help-dialog-frame",
  find: "iframe.vimium-hud-frame"
};

const startedAt = new Date().toISOString();
const vimium = prepareVimium();
const result = await withServer(async (url) => {
  console.error("benchmarking rs_vimium browser actions");
  const rsBrowser = await contentBenchmarks(rsExtension, url, rsSelectors);
  let vimiumBrowser = [];
  if (vimium) {
    console.error("benchmarking Vimium browser actions");
    vimiumBrowser = await contentBenchmarks(vimium.path, url, vimiumSelectors);
  }
  return {
    metadata: {
      started_at: startedAt,
      chrome: chromeBin,
      samples,
      warmup,
      link_count: linkCount,
      machine: machineInfo(),
      build_command: `${crepusBin} webext build --app .`,
      release_profile: releaseProfile,
      rs_vimium: JSON.parse(readFileSync(join(rsExtension, "manifest.json"), "utf8")).version,
      vimium: vimium ? vimium.version : null
    },
    rs_vimium: {
      browser_actions: rsBrowser
    },
    vimium: vimium ? {
      browser_actions: vimiumBrowser
    } : null
  };
});

console.log(JSON.stringify(result, null, 2));
