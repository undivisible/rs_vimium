import init, * as runtime from "../vendor/runtime.js";

const wasmBytes = await fetch("../vendor/runtime_bg.wasm").then((response) => response.arrayBuffer());
await init({ module_or_path: wasmBytes });
await runtime.settings_seed();

const settingKeys = [
  "scrollStepSize",
  "smoothScroll",
  "keyMappings",
  "linkHintCharacters",
  "filterLinkHints",
  "hideHud",
  "searchUrl",
  "searchEngines",
  "newTabDestination",
  "newTabCustomUrl",
  "grabBackFocus",
  "regexFindMode",
  "waitForEnterForFilteredHints",
  "helpDialog_showAdvancedCommands",
];

const boolKeys = [
  "smoothScroll",
  "filterLinkHints",
  "hideHud",
  "grabBackFocus",
  "regexFindMode",
  "waitForEnterForFilteredHints",
  "helpDialog_showAdvancedCommands",
];

let currentSettings = {};

async function load() {
  try {
    const response = await runtime.settings_get();
    currentSettings = response.settings || {};
  } catch (_error) {
    currentSettings = {};
  }
  render();
}

function render() {
  for (const key of settingKeys) {
    const element = document.getElementById(key);
    if (!element) continue;
    if (boolKeys.includes(key)) {
      element.checked = !!currentSettings[key];
    } else {
      element.value = currentSettings[key] ?? "";
    }
  }
}

function collect() {
  const output = {};
  for (const key of settingKeys) {
    const element = document.getElementById(key);
    if (!element) continue;
    if (boolKeys.includes(key)) {
      output[key] = element.checked;
    } else if (element.type === "number") {
      output[key] = parseInt(element.value, 10) || 0;
    } else {
      output[key] = element.value;
    }
  }
  return output;
}

function setStatus(message, isError = false) {
  const status = document.getElementById("status");
  status.textContent = message;
  status.className = "status" + (isError ? " error" : "");
}

document.getElementById("saveBtn").addEventListener("click", async () => {
  try {
    await chrome.storage.sync.set(collect());
    await runtime.settings_seed();
    await load();
    setStatus("Settings saved.");
  } catch (error) {
    setStatus("Error: " + error, true);
  }
});

document.getElementById("resetBtn").addEventListener("click", async () => {
  if (!confirm("Reset all settings to defaults?")) return;
  try {
    const defaults = {
      scrollStepSize: 60,
      smoothScroll: true,
      keyMappings: "# Insert your preferred key mappings here.",
      linkHintCharacters: "sadfjklewcmpgh",
      filterLinkHints: false,
      hideHud: false,
      searchUrl: "https://www.google.com/search?q=",
      searchEngines: "w: https://www.wikipedia.org/w/index.php?title=Special:Search&search=%s Wikipedia",
      newTabDestination: "vimiumNewTabPage",
      newTabCustomUrl: "",
      grabBackFocus: false,
      regexFindMode: false,
      waitForEnterForFilteredHints: true,
      helpDialog_showAdvancedCommands: false,
    };
    await chrome.storage.sync.clear();
    await chrome.storage.sync.set(defaults);
    await runtime.settings_seed();
    await load();
    setStatus("Settings reset to defaults.");
  } catch (error) {
    setStatus("Error: " + error, true);
  }
});

document.getElementById("exportBtn").addEventListener("click", () => {
  const blob = new Blob([JSON.stringify(collect(), null, 2)], { type: "application/json" });
  const link = document.createElement("a");
  link.href = URL.createObjectURL(blob);
  link.download = "vimium-settings.json";
  link.click();
  URL.revokeObjectURL(link.href);
});

await load();
