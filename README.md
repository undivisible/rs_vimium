# rs_vimium

The hacker's browser in rust.

## Build

```sh
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
cargo check --target wasm32-unknown-unknown --manifest-path runtime/Cargo.toml
crepus webext build --app .
```

Load `/Users/undivisible/projects/rs_vimium/dist/unpacked` as an unpacked extension.

## Source Layout

- `webext.toml` is the extension manifest source of truth.
- `runtime/` contains the Rust/WASM runtime.
- `src/` contains extension CSS. Crepuscularity emits the MV3 host scripts at build time.
- `pages/` contains extension HTML entrypoints.
- `views/` contains Crepuscularity UI templates.
- `icons/` and `resources/` contain packaged extension assets.

## Upstream

This fork keeps `upstream` pointed at `philc/vimium` for behavior comparison. The maintained Rust rewrite lives at `undivisible/rs_vimium`.

## License

MPL-2.0. See `LICENSE`.
