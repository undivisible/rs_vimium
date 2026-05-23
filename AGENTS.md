# rs_vimium

Vimium browser extension in Rust. Built with Crepuscularity framework.

## Build

```sh
crepus webext build --app .
# Then load dist/unpacked in chrome://extensions
```

## Structure

```
crepus.toml          # Extension manifest source of truth
runtime/             # Rust WASM runtime
  Cargo.toml         #   workspace member
  src/
    lib.rs           #   WASM exports (popup_main, content_main, options_main, handle_background_message)
    background.rs    #   Tab/window management commands
    commands.rs      #   Key binding registry, parser, default mappings
    key_handler.rs   #   Key event processing, effect dispatch
    settings.rs      #   User settings merge/prune
    vomnibar.rs      #   Search/bookmark/history omnibar
  views/
    ui.crepus        #   Crepus templates (Popup, Panel, Vomnibar, Find, AnywhereShell)
src/
  content.css.crepus #   Content script CSS (hint markers, HUD, overlay, find bar, vomnibar)
  file_urls.css      #   File URL overrides
pages/
  popup.crepus       #   Toolbar popup shell (#root → popup_main)
  new-tab.crepus     #   New tab page (chrome_url_overrides)
  options.crepus     #   Options page template
views/
  ui.crepus          #   UI templates (same as runtime/views/)
icons/               #   Extension icons
resources/           #   Static resources (tlds.txt)
dist/unpacked/       #   Build output (gitignored)
```

## WASM Exports (from `#[wasm_bindgen]` in lib.rs)

| Function | Purpose |
|---|---|
| `popup_main()` | Renders popup with keyboard shortcuts |
| `options_main()` | Binds options page event handlers |
| `content_main()` | Sets up keyboard handler, find/hints/vomnibar |
| `handle_background_message(msg)` | Routes runtime messages (settings, commands) |
| `content_key(state, key, editable)` | Processes a key event in the content script |
| `render_help_overlay(show_advanced)` | Generates help overlay HTML |
| `settings_get/set/clear()` | Storage operations |
| `query_vomnibar(query, mode)` | Search bookmarks/history/tabs |
| `hint_label(index)` / `update_hint_state(...)` | Hint label generation |
| `resolve_navigable(query)` | Parse URL/search query |
| `key_name(event_key)` | Normalize KeyboardEvent.key |
| `render_popup(state)` | Render Popup template to HTML |

## JS Files (from crepuscularity-webext assets, embedded at compile time)

| File | Role |
|---|---|
| `background.js` | MV3 service worker — imports WASM, delegates messages to `handle_background_message` |
| `content.js` | Content script — imports WASM, calls `content_main()` for vimium key handling |
| `popup.js` | Popup — imports WASM, calls `popup_main()` to render shortcut list |
| `options.js` | Options — imports WASM, calls `options_main()` to bind form handlers |
| `browser-shim.js` | Cross-browser API wrapper (chrome/browser) |
| `popup.css` | Popup styles (BEM class names) |
| `popup.html` | Popup shell (pre-rendered or dynamic) |

## Conventions

- **No custom JS in extension project** — all logic in Rust WASM or crepuscularity-webext framework assets
- **Styling** via unocss utility classes in `.crepus` templates where possible; fallback to inline `<style>` when needed
- **Key bindings** defined in `commands.rs::default_key_bindings()`, parsed from vimium-compatible syntax
- **Messages** use `"type": "rs_vimium"` for extension-internal communication
- **All frames** get the content script; keyboard handling respects frame isolation
- **Settings** stored in `storage.sync`, seeded on install via `settings_seed()`


<claude-mem-context>
# Memory Context

# claude-mem status

This project has no memory yet. The current session will seed it; subsequent sessions will receive auto-injected context for relevant past work.

Memory injection starts on your second session in a project.

`/learn-codebase` is available if the user wants to front-load the entire repo into memory in a single pass (~5 minutes on a typical repo, optional). Otherwise memory builds passively as work happens.

Live activity: http://localhost:37701
How it works: `/how-it-works`

This message disappears once the first observation lands.
</claude-mem-context>
