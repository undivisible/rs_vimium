# New Tab Customization & Release Workflow

## Overview

Extend the rs_vimium new tab page with user-configurable defaults and visual
customization — search engine, input styling, background color/image — plus a
GitHub release workflow to distribute the unpacked extension as a zip.

## Settings storage

| Key | Type | Storage | Purpose |
|-----|------|---------|---------|
| `newTabSearchEngine` | string | sync | Preset name (`google`, `duckduckgo`, `bing`, `brave`, `startpage`, `custom`) |
| `newTabSearchEngineUrl` | string | sync | Custom search URL template (when preset = `custom`) |
| `newTabDarkInput` | bool | sync | Toggle dark (black) input background |
| `newTabAccentColor` | string | sync | Hex color for input accent/border |
| `newTabBgType` | string | sync | `none` / `color` / `image` |
| `newTabBgColor` | string | sync | Solid background hex color |
| `newTabBgImageUrl` | string | sync | External image URL |
| `newTabBgImageData` | string | **local** | Base64 data URL from file upload |

New tab page reads these from `storage.sync` + `storage.local` on load and
applies them as inline styles on `<main>`.

## UI — New tab settings panel

Expands the existing `<details class="settings-menu">` panel with sections:

```
Settings
─── Display ───
☐ Show clock and date
☐ Show bookmarks
─── Search ───
Search engine:  [DuckDuckGo ▾]
Custom URL:     [________________]  ← visible when preset = custom
─── Appearance ───
☐ Dark input
Accent color:   [■ picker]
─── Background ───
Type:           [Color ▾]
Color:          [■ picker]
Image URL:      [________________]  ← visible when type = image
Upload file:    [Choose File]
```

Controls are rendered in Rust DOM (`setup_new_tab`) by extending the existing
`install_new_tab_preferences` function.

### Preset search engines

| Label | URL template |
|-------|-------------|
| DuckDuckGo | `https://duckduckgo.com/?q=%s` |
| Google | `https://www.google.com/search?q=%s` |
| Bing | `https://www.bing.com/search?q=%s` |
| Brave | `https://search.brave.com/search?q=%s` |
| Startpage | `https://www.startpage.com/do/dsearch?query=%s` |

When the user selects a preset, store the key as `newTabSearchEngine` and
set `newTabSearchEngineUrl` to the corresponding template automatically.

### Bang search still works

Bangs (`!g`, `!w`, etc.) take priority over the default search engine. The
default engine is only used when no bang is present and the query is not a URL.

### Placeholder

When a default engine is configured (non-custom preset or custom URL filled),
the placeholder reads `"Search Google…" / "Search DuckDuckGo…"` etc. When a
bang is active the bang name takes over (`"searching Google"`).

## Apply background styles

On new tab load / setting change, apply styles to the `<main>` element:

```js
if (bgType === 'color') {
  main.style.backgroundColor = bgColor;
  main.style.backgroundImage = 'none';
} else if (bgType === 'image') {
  const url = bgImageUrl || bgImageData;
  main.style.backgroundImage = `url(${url})`;
  main.style.backgroundSize = 'cover';
  main.style.backgroundPosition = 'center';
} else {
  main.style.backgroundColor = '';
  main.style.backgroundImage = '';
}
```

Dark input toggle adds/removes a CSS class on `#search-shell` fieldset.

## Release workflow

`.github/workflows/release.yml` — triggered on tag push matching `v*`:

1. Checkout repo
2. Install Rust + `cargo install crepus`
3. Run `crepus webext build --app .`
4. Zip `dist/unpacked/` → `rs-vimium-v{VERSION}.zip`
5. Create GitHub Release with the zip as an asset
