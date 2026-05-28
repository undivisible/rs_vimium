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
| Date | 2026-05-28T01:03:34.634Z |
| Browser | Google Chrome for Testing 148.0.7778.96 |
| Machine | Mac17,9, Apple M5 Pro, arm64 |
| CPU cores | 15 physical, 15 logical |
| Memory | 48 GiB |
| OS | macOS 26.5 (25F71) |
| Samples | 8 measured, 2 warmup |
| Page size | 160 links, 160 buttons |
| rs_vimium | 1.2.3 |
| Vimium | 2.4.2 |

Browser-action TTA measures from key dispatch to observable scroll or DOM state.

| Action | rs_vimium median | rs_vimium p90 | Vimium median | Vimium p90 |
| --- | ---: | ---: | ---: | ---: |
| `j` scroll | 1.8 ms | 7.1 ms | 37.4 ms | 41.8 ms |
| `f` link hints | 4.0 ms | 4.4 ms | 35.8 ms | 44.9 ms |
| `o` vomnibar | 1.7 ms | 1.8 ms | 5.1 ms | 6.5 ms |
| `?` help | 2.1 ms | 2.5 ms | 5.3 ms | 6.5 ms |
| `/` find | 2.2 ms | 3.5 ms | 4.3 ms | 4.6 ms |

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
