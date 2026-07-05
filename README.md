# rs_vimium

> The hacker's browser, in Rust.

Keyboard-first browser runtime and web superpowers built in Rust/WASM with [Crepuscularity](https://github.com/semitechnological/crepuscularity).

## Build

```sh
bun run build
```

Load `dist/unpacked/` as an unpacked extension in `chrome://extensions`.
Set `CREPUS_BIN=/path/to/crepus` for the benchmark script if you want a different CLI binary.
For Firefox output, run `crepus webext build --app . --browser firefox` and load `dist/firefox/manifest.json` in `about:debugging#/runtime/this-firefox`.

The web-extension build uses the workspace release profile:

| Setting | Value |
| --- | --- |
| `opt-level` | `s` |
| `lto` | `true` |
| `codegen-units` | `1` |
| `strip` | `true` |

## Benchmark

```sh
bun run benchmark
```

The benchmark builds `dist/unpacked`, launches fresh Chrome for Testing profiles, and compares user-facing browser-action TTAs between rs_vimium and Vimium on a deterministic local page. Set `CHROME_BIN` to choose a browser binary. Set `VIMIUM_PATH` to compare against a Vimium checkout, or pass `--skip-vimium`.

Latest local run:

| Field | Value |
| --- | --- |
| Date | 2026-07-05T10:24:32.757Z |
| Browser | Google Chrome for Testing 148.0.7778.96 |
| Machine | Mac17,9, Apple M5 Pro, arm64 |
| CPU cores | 15 physical, 15 logical |
| Memory | 48 GiB |
| OS | macOS 27.0 (26A5368g) |
| Samples | 8 measured, 2 warmup |
| Page size | 160 links, 160 buttons |
| rs_vimium | 1.2.4 |
| Vimium | not run |

Browser-action TTA measures from key dispatch to observable scroll or DOM state.

| Action | rs_vimium median | rs_vimium p90 | Vimium median | Vimium p90 |
| --- | ---: | ---: | ---: | ---: |
| `j` scroll | 2.5 ms | 6.8 ms | n/a | n/a |
| `f` link hints | 11.0 ms | 14.8 ms | n/a | n/a |
| `o` vomnibar | 2.4 ms | 4.5 ms | n/a | n/a |
| `?` help | 8.3 ms | 12.9 ms | n/a | n/a |
| `/` find | 3.1 ms | 5.5 ms | n/a | n/a |

## Layout

| Path | Role |
| --- | --- |
| `crepus.toml` | Extension manifest (capabilities, content scripts, pages) |
| `runtime/` | Rust/WASM (`popup_main`, `content_main`, `options_main`, `new_tab_main`) |
| `pages/` | Extension pages (`.crepus` → HTML at build time) |
| `views/ui.crepus` | In-page UI templates (hints, vomnibar, find bar, help overlay) |
| `src/content.css.crepus` | Content-script styles (compiled to `src/content.css`) |
| `resources/tlds.txt` | TLD list for URL detection |
| `icons/` | Toolbar and extension icons |

## New tab page

With default settings, rs_vimium overrides the browser new-tab page (`chrome_url_overrides.newtab` → `pages/new-tab.html`). Change this under **Options → New tab page**.

## License

MPL-2.0 — see `LICENSE`.
