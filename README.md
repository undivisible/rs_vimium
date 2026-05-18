# rs_vimium

The hacker's browser in rust. Built with [Crepuscularity](https://github.com/tschk/crepuscularity).

## Build

```sh
crepus webext build --app .
```

Load `/Users/undivisible/projects/rs_vimium/dist/unpacked` as an unpacked extension.

## Source Layout

- `webext.toml` is the extension manifest source of truth.
- `runtime/` contains the Rust/WASM runtime.
- `src/` contains `.css.crepus` style assets rendered by `crepus webext build`.
- `pages/` contains local extension pages written as `.crepus` templates.
- `views/` contains Crepuscularity UI templates.
- `icons/` and `resources/` contain packaged extension assets.

## Upstream

This fork keeps `upstream` pointed at `philc/vimium` for behavior comparison. The maintained Rust rewrite lives at `undivisible/rs_vimium`.

## License

MPL-2.0. See `LICENSE`.
