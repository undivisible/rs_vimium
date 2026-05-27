# rs_vimium

> The hacker's browser, in Rust.

Keyboard-first browser runtime and web superpowers built in Rust/WASM with [Crepuscularity](https://github.com/semitechnological/crepuscularity).

## Build

```sh
crepus webext build --app .
```

Load `dist/unpacked/` as an unpacked extension in `chrome://extensions`.

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

The benchmark builds `dist/unpacked`, launches a fresh Chrome for Testing profile, measures exported WASM functions from an extension page, and measures browser-visible TTAs on a deterministic local page. Set `CHROME_BIN` to choose a browser binary. Set `VIMIUM_PATH` to compare against an upstream Vimium checkout, or pass `--skip-vimium`.

Latest local run:

| Field | Value |
| --- | --- |
| Date | 2026-05-27T23:52:33.034Z |
| Browser | Chrome for Testing 148.0.7778.96 |
| Samples | 8 measured, 2 warmup |
| Page size | 160 links, 160 buttons |
| rs_vimium | 1.2.2 |
| Vimium | 2.4.2 |

Browser-action TTA measures from key dispatch to observable scroll or DOM state.

| Action | rs_vimium median | rs_vimium p90 | Vimium median | Vimium p90 |
| --- | ---: | ---: | ---: | ---: |
| `j` scroll | 15.9 ms | 16.3 ms | 24.4 ms | 24.9 ms |
| `f` link hints | 4.0 ms | 4.5 ms | 7.1 ms | 8.6 ms |
| `o` vomnibar | 0.9 ms | 1.0 ms | not observed | not observed |
| `?` help | not observed | not observed | not observed | not observed |
| `/` find | not observed | not observed | not observed | not observed |

The unobserved rows are headless-browser observability gaps, not claims that the feature is missing or slow. The script records them as failures instead of inventing a timing.

WASM export microbenchmarks run inside an extension page against the release WASM module.

| Export | Median | p90 |
| --- | ---: | ---: |
| `runtime_version` | 0.0 ms | 0.0 ms |
| `render_popup` | 4.9 ms | 5.0 ms |
| `shortcut_groups_json` | 0.2 ms | 0.2 ms |
| `content_key("j")` | 0.0 ms | 0.1 ms |
| `content_key("f")` | 0.0 ms | 0.1 ms |
| `content_key("o")` | 0.0 ms | 0.0 ms |
| `content_key("?")` | 0.0 ms | 0.1 ms |
| `render_help_overlay(false)` | 0.9 ms | 0.9 ms |
| `render_help_overlay(true)` | 2.7 ms | 2.8 ms |
| `command_list` | 0.2 ms | 0.4 ms |
| `hint_label(0)` | 0.0 ms | 0.0 ms |
| `hint_label(500)` | 0.0 ms | 0.1 ms |
| `update_hint_state` | 0.0 ms | 0.2 ms |
| `resolve_navigable("example.com")` | 0.0 ms | 0.0 ms |
| `resolve_navigable("hello world")` | 0.0 ms | 0.0 ms |
| `key_name("Escape")` | 0.0 ms | 0.0 ms |
| `key_name(" ")` | 0.0 ms | 0.0 ms |
| `is_search_query("example.com")` | 0.0 ms | 0.0 ms |
| `is_search_query("hello world")` | 0.0 ms | 0.0 ms |
| `settings_get` | 0.1 ms | 14.0 ms |
| `settings_seed` | 0.9 ms | 1.8 ms |
| `settings_set` | 0.8 ms | 1.1 ms |
| `settings_clear` | 3.0 ms | 3.1 ms |
| `query_vomnibar("tab", "commands")` | 0.1 ms | 0.1 ms |
| `handle_background_message("settings:get")` | 0.1 ms | 0.2 ms |
| `handle_background_message("query-vomnibar")` | 0.0 ms | 0.1 ms |

Extension page timings:

| Page | Median | p90 | Failures |
| --- | ---: | ---: | ---: |
| Popup | 3.2 ms | 4.2 ms | 0 |
| Options | 0.0 ms | 0.0 ms | 7 |
| New tab | 0.1 ms | 11.4 ms | 0 |

Values shown as `0.0 ms` completed below the timer precision available in the browser context.

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

## Upstream

`upstream` tracks [philc/vimium](https://github.com/philc/vimium) for behavior comparison. This repo is the maintained Rust rewrite.

## License

MPL-2.0 — see `LICENSE`.
