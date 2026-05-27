# New Tab Customization & Release Workflow — Implementation Plan

**Goal:** Add configurable default search engine, input/background styling, release workflow.

**Architecture:** New tab settings in `storage.sync` + `storage.local` (image data). Panel controls in crepus template, wired via Rust. Search engine URL passed through shared state to `new_tab_resolve_url_with_bang`.

---

### Task 1: Setting defaults & keys

**Files:** `runtime/src/settings.rs:31-57`, `runtime/src/lib.rs:3135-3160`

Add defaults: `newTabSearchEngine: "duckduckgo"`, `newTabSearchEngineUrl`, `newTabDarkInput: false`, `newTabAccentColor: ""`, `newTabBgType: "none"`, `newTabBgColor: ""`, `newTabBgImageUrl: ""`. Add all to `SETTING_KEYS`, `newTabDarkInput` to `BOOL_KEYS`.

### Task 2: Preset search engine data

**Files:** `runtime/src/lib.rs` near `NEW_TAB_BANGS`

Add `PresetSearchEngine` struct + `PRESET_SEARCH_ENGINES` const (DuckDuckGo, Google, Bing, Brave, Startpage). Add `resolve_search_engine(settings) -> &str` and `search_engine_display_name(settings) -> String` helpers.

### Task 3: Update crepus template

**Files:** `pages/new-tab.crepus`

Expand `.settings-panel` with sections: Display, Search (select + custom URL input), Appearance (dark input checkbox, accent color picker), Background (type select, color picker, image URL input, file upload). Add CSS for new controls, `.hidden` toggles, dark input class, background image classes.

### Task 4: Wire settings panel in Rust

**Files:** `runtime/src/lib.rs` — rewrite `install_new_tab_preferences`

Load all settings, apply to DOM, save on change. Use doc-level change delegation. File upload via FileReader → storage.local. `apply_new_tab_appearance()` reads all settings and applies styles/classes to `<main>` and `#search-shell`.

### Task 5: Integrate search engine into navigation

**Files:** `runtime/src/lib.rs` — `setup_new_tab`, `new_tab_resolve_url_with_bang`

Store search engine URL in Rc<RefCell>. Use it in `new_tab_resolve_url_with_bang` for non-bang queries. Update placeholder to reflect current engine name.

### Task 6: Release workflow

**Files:** Create `.github/workflows/release.yml`

On tag v*: setup Rust, `cargo install crepus`, `crepus webext build --app .`, zip dist/unpacked → release asset.

### Task 7: Bump version & git ops

**Files:** `crepus.toml`

`1.1.0` → `1.2.0`. Commit, tag `v1.2.0`, push.
