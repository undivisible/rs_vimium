# rs_vimium

> The hacker's browser, in Rust.

Keyboard-first browser runtime and web superpowers built in Rust/WASM with [Crepuscularity](https://github.com/semitechnological/crepuscularity).

## Build

```sh
crepus webext build --app .
```

Load `dist/unpacked/` as an unpacked extension in `chrome://extensions`.

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
