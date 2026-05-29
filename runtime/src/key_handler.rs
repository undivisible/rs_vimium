use crate::commands::{CommandEntry, KeyMapRegistry, RegistryEntry};
use serde_json::{json, Value};

pub const MODE_NORMAL: &str = "normal";
pub const MODE_HINTS: &str = "hints";
pub const MODE_VISUAL: &str = "visual";
pub const MODE_INSERT: &str = "insert";
pub const MODE_MARK: &str = "mark";
pub const MODE_FIND: &str = "find";
pub const MODE_VISUAL_LINE: &str = "visual-line";

pub const EFFECT_CLEAR_OVERLAYS: &str = "clear-overlays";
pub const EFFECT_SCROLL_STEP: &str = "scroll-step";
pub const EFFECT_SCROLL_TOP: &str = "scroll-top";
pub const EFFECT_SCROLL_BOTTOM: &str = "scroll-bottom";
pub const EFFECT_RELOAD: &str = "reload";
pub const EFFECT_COPY_URL: &str = "copy-url";
pub const EFFECT_PASS_NEXT_KEY: &str = "pass-next-key";
pub const EFFECT_HINTS_GENERAL: &str = "hints-general";
pub const EFFECT_HINTS_QUEUE: &str = "hints-queue";
pub const EFFECT_HINTS_DOWNLOAD: &str = "hints-download";
pub const EFFECT_HINTS_INCOGNITO: &str = "hints-incognito";
pub const EFFECT_HINTS_COPY_URL: &str = "hints-copy-url";
pub const EFFECT_CREATE_MARK: &str = "create-mark";
pub const EFFECT_GOTO_MARK: &str = "goto-mark";
pub const EFFECT_ENTER_VISUAL: &str = "enter-visual";
pub const EFFECT_INSERT_MODE: &str = "insert-mode";
pub const EFFECT_HALF_SCROLL: &str = "half-scroll";
pub const EFFECT_FULL_SCROLL: &str = "full-scroll";
pub const EFFECT_SCROLL_LEFT: &str = "scroll-left";
pub const EFFECT_SCROLL_RIGHT: &str = "scroll-right";
pub const EFFECT_GO_UP: &str = "go-up";
pub const EFFECT_GO_ROOT: &str = "go-root";
pub const EFFECT_FOCUS_INPUT: &str = "focus-input";
pub const EFFECT_HINTS: &str = "hints";
pub const EFFECT_VOMNIBAR: &str = "vomnibar";
pub const EFFECT_VOMNIBAR_BOOKMARKS: &str = "vomnibar-bookmarks";
pub const EFFECT_VOMNIBAR_TABS: &str = "vomnibar-tabs";
pub const EFFECT_VOMNIBAR_COMMANDS: &str = "vomnibar-commands";
pub const EFFECT_VOMNIBAR_EDIT_URL: &str = "vomnibar-edit-url";
pub const EFFECT_FOLLOW_PATTERN: &str = "follow-pattern";
pub const EFFECT_OPEN_CLIPBOARD: &str = "open-clipboard";
pub const EFFECT_VIEW_SOURCE: &str = "view-source";
pub const EFFECT_BACKGROUND: &str = "background";
pub const EFFECT_VISUAL_MOVE: &str = "visual-move";
pub const EFFECT_VISUAL_COPY: &str = "visual-copy";
pub const EFFECT_EXIT_VISUAL: &str = "exit-visual";
pub const EFFECT_STAY_VISUAL: &str = "stay-visual";
pub const EFFECT_CYCLE_FRAME: &str = "cycle-frame";
pub const EFFECT_FOCUS_MAIN_FRAME: &str = "focus-main-frame";
pub const EFFECT_FIND: &str = "find";
pub const EFFECT_FIND_NEXT: &str = "find-next";
pub const EFFECT_FIND_SELECTED: &str = "find-selected";
pub const EFFECT_HISTORY_BACK: &str = "history-back";
pub const EFFECT_HISTORY_FORWARD: &str = "history-forward";
pub const EFFECT_HELP: &str = "help";

#[derive(Debug, Clone, Default)]
pub struct KeyState {
    pub mode: String,
    pub sequence: String,
    pub count_text: String,
    pub input: String,
}

impl KeyState {
    pub fn new() -> Self {
        KeyState {
            mode: MODE_NORMAL.to_string(),
            sequence: String::new(),
            count_text: String::new(),
            input: String::new(),
        }
    }

    pub fn is_insert(&self) -> bool {
        self.mode == MODE_INSERT
    }

    pub fn is_hints(&self) -> bool {
        self.mode == MODE_HINTS
    }

    pub fn is_visual(&self) -> bool {
        self.mode == MODE_VISUAL
    }

    pub fn is_find(&self) -> bool {
        self.mode == MODE_FIND
    }

    pub fn is_mark(&self) -> bool {
        self.mode == MODE_MARK
    }
}

pub fn handle_key(
    state: &KeyState,
    key: &str,
    editable: bool,
    registry: &KeyMapRegistry,
    user_mappings: &std::collections::HashMap<String, Option<RegistryEntry>>,
    pass_keys: &str,
) -> Value {
    if key == "Esc" {
        return json!({
            "state": { "mode": MODE_NORMAL, "sequence": "", "countText": "", "input": "" },
            "effect": { "kind": EFFECT_CLEAR_OVERLAYS },
            "prevent": false
        });
    }

    if state.is_insert() || editable {
        return json!({ "state": state_to_val(state), "effect": null, "prevent": false });
    }

    if state.is_hints() {
        return json!({ "state": state_to_val(state), "effect": null, "prevent": false });
    }

    if state.is_visual() {
        return handle_visual_key(state, key, registry, user_mappings);
    }

    let mut sequence = state.sequence.clone();
    let mut count_text = state.count_text.clone();

    if is_pass_key(state, key, pass_keys, registry, user_mappings) {
        return json!({ "state": state_to_val(state), "effect": null, "prevent": false });
    }

    if key.len() == 1
        && key.as_bytes()[0].is_ascii_digit()
        && sequence.is_empty()
        && (key != "0" || !count_text.is_empty())
    {
        count_text.push_str(key);
        return json!({
            "state": { "mode": MODE_NORMAL, "sequence": "", "countText": count_text, "input": "" },
            "effect": null,
            "prevent": true
        });
    }

    sequence.push_str(key);

    let count = parse_count(&count_text);
    let is_prefix = registry.is_prefix(&sequence, user_mappings);
    let resolved = registry.resolve_command(&sequence, user_mappings);

    if let Some((cmd_name, entry)) = resolved {
        let effect = command_effect(cmd_name, count, entry);
        let effect_mode = effect_mode(&effect);
        json!({
            "state": { "mode": effect_mode, "sequence": "", "countText": "", "input": "" },
            "effect": effect,
            "prevent": true
        })
    } else if is_prefix {
        json!({
            "state": { "mode": MODE_NORMAL, "sequence": sequence, "countText": count_text, "input": "" },
            "effect": null,
            "prevent": true
        })
    } else {
        if !state.sequence.is_empty() {
            if let Some((cmd_name, entry)) = registry.resolve_command(key, user_mappings) {
                let effect = command_effect(cmd_name, 1, entry);
                let effect_mode = effect_mode(&effect);
                return json!({
                    "state": { "mode": effect_mode, "sequence": "", "countText": "", "input": "" },
                    "effect": effect,
                    "prevent": true
                });
            }
        }
        json!({
            "state": { "mode": MODE_NORMAL, "sequence": "", "countText": "", "input": "" },
            "effect": null,
            "prevent": false
        })
    }
}

fn is_pass_key(
    state: &KeyState,
    key: &str,
    pass_keys: &str,
    registry: &KeyMapRegistry,
    user_mappings: &std::collections::HashMap<String, Option<RegistryEntry>>,
) -> bool {
    !key.is_empty()
        && !pass_keys.is_empty()
        && state.count_text.is_empty()
        && !registry.has_continuation_mapping(&state.sequence, key, user_mappings)
        && pass_keys.contains(key)
}

pub fn handle_visual_key(
    state: &KeyState,
    key: &str,
    registry: &KeyMapRegistry,
    user_mappings: &std::collections::HashMap<String, Option<RegistryEntry>>,
) -> Value {
    let mut sequence = state.sequence.clone();
    let count_text = state.count_text.clone();

    sequence.push_str(key);
    let count = parse_count(&count_text);

    let is_prefix = registry.is_prefix(&sequence, user_mappings);
    let resolved = registry.resolve_command(&sequence, user_mappings);

    if let Some((cmd_name, entry)) = resolved {
        let effect = visual_effect(cmd_name, count, entry);
        let post_mode = match effect.get("kind").and_then(Value::as_str) {
            Some("exit-visual") | Some("none") | None => MODE_NORMAL,
            Some("stay-visual") => MODE_VISUAL,
            _ => MODE_NORMAL,
        };
        json!({
            "state": { "mode": post_mode, "sequence": "", "countText": "", "input": "" },
            "effect": effect,
            "prevent": true
        })
    } else if is_prefix {
        json!({
            "state": { "mode": MODE_VISUAL, "sequence": sequence, "countText": count_text, "input": "" },
            "effect": null,
            "prevent": true
        })
    } else {
        json!({
            "state": { "mode": MODE_NORMAL, "sequence": "", "countText": "", "input": "" },
            "effect": null,
            "prevent": false
        })
    }
}

fn parse_count(count_text: &str) -> i64 {
    count_text
        .parse::<i64>()
        .ok()
        .filter(|v| *v > 0)
        .unwrap_or(1)
}

fn state_to_val(state: &KeyState) -> Value {
    json!({
        "mode": state.mode,
        "sequence": state.sequence,
        "countText": state.count_text,
        "input": state.input
    })
}

pub fn background_command_for_registry_name(cmd_name: &str) -> Option<&'static str> {
    match cmd_name {
        "createTab" => Some("create-tab"),
        "previousTab" => Some("previous-tab"),
        "nextTab" => Some("next-tab"),
        "visitPreviousTab" => Some("visit-previous-tab"),
        "firstTab" => Some("first-tab"),
        "lastTab" => Some("last-tab"),
        "duplicateTab" => Some("duplicate-tab"),
        "togglePinTab" => Some("toggle-pin"),
        "toggleMuteTab" => Some("toggle-mute"),
        "removeTab" => Some("remove-tab"),
        "restoreTab" => Some("restore-tab"),
        "moveTabToNewWindow" => Some("move-to-new-window"),
        "closeTabsOnLeft" => Some("close-tabs-left"),
        "closeTabsOnRight" => Some("close-tabs-right"),
        "closeOtherTabs" => Some("close-other-tabs"),
        "moveTabLeft" => Some("move-tab-left"),
        "moveTabRight" => Some("move-tab-right"),
        "setZoom" => Some("set-zoom"),
        "zoomIn" => Some("zoom-in"),
        "zoomOut" => Some("zoom-out"),
        "zoomReset" => Some("zoom-reset"),
        _ => None,
    }
}

pub fn command_effect(cmd_name: &str, count: i64, entry: Option<&CommandEntry>) -> Value {
    let bkg = entry.is_some_and(|e| e.background);
    let no_repeat = entry.is_some_and(|e| e.no_repeat);
    let count = entry
        .and_then(|entry| entry.options.get("count"))
        .and_then(Value::as_i64)
        .map(|option_count| count.max(1) * option_count.max(1))
        .unwrap_or(count);
    let count = if no_repeat { 1 } else { count };

    match cmd_name {
        "scrollDown" => {
            json!({"kind": EFFECT_SCROLL_STEP, "axis": "y", "direction": 1, "count": count})
        }
        "scrollUp" => {
            json!({"kind": EFFECT_SCROLL_STEP, "axis": "y", "direction": -1, "count": count})
        }
        "scrollToTop" => json!({"kind": EFFECT_SCROLL_TOP, "count": count}),
        "scrollToBottom" => json!({"kind": EFFECT_SCROLL_BOTTOM}),
        "scrollPageDown" => json!({"kind": EFFECT_HALF_SCROLL, "direction": 1, "count": count}),
        "scrollPageUp" => json!({"kind": EFFECT_HALF_SCROLL, "direction": -1, "count": count}),
        "scrollFullPageDown" => json!({"kind": EFFECT_FULL_SCROLL, "direction": 1, "count": count}),
        "scrollFullPageUp" => json!({"kind": EFFECT_FULL_SCROLL, "direction": -1, "count": count}),
        "scrollLeft" => {
            json!({"kind": EFFECT_SCROLL_STEP, "axis": "x", "direction": -1, "count": count})
        }
        "scrollRight" => {
            json!({"kind": EFFECT_SCROLL_STEP, "axis": "x", "direction": 1, "count": count})
        }
        "scrollToLeft" => json!({"kind": EFFECT_SCROLL_LEFT}),
        "scrollToRight" => json!({"kind": EFFECT_SCROLL_RIGHT}),
        "reload" => {
            let hard = entry
                .and_then(|entry| entry.options.get("hard"))
                .and_then(Value::as_bool)
                .unwrap_or(false);
            if bkg {
                json!({"kind": EFFECT_BACKGROUND, "command": "reload", "hard": hard})
            } else {
                json!({"kind": EFFECT_RELOAD, "hard": hard})
            }
        }
        "copyCurrentUrl" => json!({"kind": EFFECT_COPY_URL}),
        "openCopiedUrlInCurrentTab" => json!({"kind": EFFECT_OPEN_CLIPBOARD, "newTab": false}),
        "openCopiedUrlInNewTab" => json!({"kind": EFFECT_OPEN_CLIPBOARD, "newTab": true}),
        "goUp" => json!({"kind": EFFECT_GO_UP, "count": count}),
        "goToRoot" => json!({"kind": EFFECT_GO_ROOT}),
        "enterInsertMode" => json!({"kind": EFFECT_INSERT_MODE}),
        "enterVisualMode" => json!({"kind": EFFECT_ENTER_VISUAL, "mode": MODE_VISUAL}),
        "enterVisualLineMode" => json!({"kind": EFFECT_ENTER_VISUAL, "mode": MODE_VISUAL_LINE}),
        "focusInput" => json!({"kind": EFFECT_FOCUS_INPUT, "count": count}),
        "LinkHints.activateModeToOpenInNewTab" => {
            json!({"kind": "hints", "newTab": true, "foreground": false})
        }
        "LinkHints.activateModeToOpenInNewForegroundTab" => {
            json!({"kind": "hints", "newTab": true, "foreground": true})
        }
        "LinkHints.activateMode" => json!({"kind": EFFECT_HINTS_GENERAL}),
        "LinkHints.activateModeWithQueue" => json!({"kind": EFFECT_HINTS_QUEUE}),
        "LinkHints.activateModeToDownloadLink" => json!({"kind": EFFECT_HINTS_DOWNLOAD}),
        "LinkHints.activateModeToOpenIncognito" => json!({"kind": EFFECT_HINTS_INCOGNITO}),
        "LinkHints.activateModeToCopyLinkUrl" => json!({"kind": EFFECT_HINTS_COPY_URL}),
        "goPrevious" => json!({"kind": EFFECT_FOLLOW_PATTERN, "pattern": "previous"}),
        "goNext" => json!({"kind": EFFECT_FOLLOW_PATTERN, "pattern": "next"}),
        "nextFrame" => json!({"kind": EFFECT_CYCLE_FRAME, "direction": 1}),
        "mainFrame" => json!({"kind": EFFECT_FOCUS_MAIN_FRAME}),
        "Marks.activateCreateMode" => json!({"kind": EFFECT_CREATE_MARK}),
        "Marks.activateGotoMode" => json!({"kind": EFFECT_GOTO_MARK}),
        "Vomnibar.activate" => {
            json!({"kind": EFFECT_VOMNIBAR, "newTab": false, "options": entry.map(|entry| entry.options.clone()).unwrap_or_else(|| json!({}))})
        }
        "Vomnibar.activateInNewTab" => {
            json!({"kind": EFFECT_VOMNIBAR, "newTab": true, "options": entry.map(|entry| entry.options.clone()).unwrap_or_else(|| json!({}))})
        }
        "Vomnibar.activateBookmarks" => {
            json!({"kind": EFFECT_VOMNIBAR_BOOKMARKS, "newTab": false, "options": entry.map(|entry| entry.options.clone()).unwrap_or_else(|| json!({}))})
        }
        "Vomnibar.activateBookmarksInNewTab" => {
            json!({"kind": EFFECT_VOMNIBAR_BOOKMARKS, "newTab": true, "options": entry.map(|entry| entry.options.clone()).unwrap_or_else(|| json!({}))})
        }
        "Vomnibar.activateTabSelection" => json!({"kind": EFFECT_VOMNIBAR_TABS}),
        "Vomnibar.activateCommandSelection" => {
            json!({"kind": EFFECT_VOMNIBAR_COMMANDS})
        }
        "Vomnibar.activateEditUrl" => {
            json!({"kind": EFFECT_VOMNIBAR_EDIT_URL, "newTab": false, "options": entry.map(|entry| entry.options.clone()).unwrap_or_else(|| json!({}))})
        }
        "Vomnibar.activateEditUrlInNewTab" => {
            json!({"kind": EFFECT_VOMNIBAR_EDIT_URL, "newTab": true, "options": entry.map(|entry| entry.options.clone()).unwrap_or_else(|| json!({}))})
        }
        "enterFindMode" => json!({"kind": EFFECT_FIND}),
        "performFind" => json!({"kind": EFFECT_FIND_NEXT, "reverse": false}),
        "performBackwardsFind" => json!({"kind": EFFECT_FIND_NEXT, "reverse": true}),
        "findSelected" => json!({"kind": EFFECT_FIND_SELECTED, "reverse": false}),
        "findSelectedBackwards" => json!({"kind": EFFECT_FIND_SELECTED, "reverse": true}),
        "goBack" => json!({"kind": EFFECT_HISTORY_BACK}),
        "goForward" => json!({"kind": EFFECT_HISTORY_FORWARD}),
        "createTab" | "previousTab" | "nextTab" | "visitPreviousTab" | "firstTab" | "lastTab"
        | "duplicateTab" | "togglePinTab" | "toggleMuteTab" | "removeTab" | "restoreTab"
        | "moveTabToNewWindow" | "closeTabsOnLeft" | "closeTabsOnRight" | "closeOtherTabs"
        | "moveTabLeft" | "moveTabRight" | "setZoom" | "zoomIn" | "zoomOut" | "zoomReset" => {
            json!({"kind": EFFECT_BACKGROUND, "command": background_command_for_registry_name(cmd_name), "count": count, "options": entry.map(|entry| entry.options.clone()).unwrap_or_else(|| json!({}))})
        }
        "passNextKey" => json!({"kind": EFFECT_PASS_NEXT_KEY}),
        "toggleViewSource" => json!({"kind": EFFECT_VIEW_SOURCE}),
        "showHelp" => json!({"kind": EFFECT_HELP}),
        _ => {
            if bkg {
                json!({"kind": EFFECT_BACKGROUND, "command": cmd_name})
            } else {
                json!({"kind": "none"})
            }
        }
    }
}

fn visual_effect(cmd_name: &str, _count: i64, _entry: Option<&CommandEntry>) -> Value {
    match cmd_name {
        "j" | "scrollDown" => {
            json!({"kind": EFFECT_VISUAL_MOVE, "direction": "forward", "granularity": "line"})
        }
        "k" | "scrollUp" => {
            json!({"kind": EFFECT_VISUAL_MOVE, "direction": "backward", "granularity": "line"})
        }
        "h" | "scrollLeft" => {
            json!({"kind": EFFECT_VISUAL_MOVE, "direction": "backward", "granularity": "character"})
        }
        "l" | "scrollRight" => {
            json!({"kind": EFFECT_VISUAL_MOVE, "direction": "forward", "granularity": "character"})
        }
        "w" => {
            json!({"kind": EFFECT_VISUAL_MOVE, "direction": "forward", "granularity": "vimword"})
        }
        "b" => {
            json!({"kind": EFFECT_VISUAL_MOVE, "direction": "backward", "granularity": "vimword"})
        }
        "e" => {
            json!({"kind": EFFECT_VISUAL_MOVE, "direction": "forward", "granularity": "vimword-end"})
        }
        "0" => {
            json!({"kind": EFFECT_VISUAL_MOVE, "direction": "backward", "granularity": "lineboundary"})
        }
        "$" => {
            json!({"kind": EFFECT_VISUAL_MOVE, "direction": "forward", "granularity": "lineboundary"})
        }
        "gg" | "scrollToTop" => {
            json!({"kind": EFFECT_VISUAL_MOVE, "direction": "backward", "granularity": "document-boundary"})
        }
        "G" | "scrollToBottom" => {
            json!({"kind": EFFECT_VISUAL_MOVE, "direction": "forward", "granularity": "document-boundary"})
        }
        "y" => json!({"kind": EFFECT_VISUAL_COPY, "stay": EFFECT_EXIT_VISUAL}),
        "Esc" => json!({"kind": EFFECT_EXIT_VISUAL}),
        _ => json!({"kind": EFFECT_STAY_VISUAL}),
    }
}

pub fn go_up_url(url: &str, count: i64) -> Option<String> {
    let count = count.max(1) as usize;
    let trimmed = url.strip_suffix('/').unwrap_or(url);
    let mut parts = trimmed.split('/').collect::<Vec<_>>();
    if parts.len() <= 3 {
        return None;
    }
    let keep = parts.len().saturating_sub(count).max(3);
    parts.truncate(keep);
    Some(parts.join("/"))
}

pub fn root_url(url: &str) -> Option<String> {
    let scheme_end = url.find("://")?;
    let after_scheme = scheme_end + 3;
    let host_end = url[after_scheme..]
        .find('/')
        .map(|idx| after_scheme + idx)
        .unwrap_or(url.len());
    Some(url[..host_end].to_string())
}

pub fn effect_mode(effect: &Value) -> String {
    match effect.get("kind").and_then(Value::as_str).unwrap_or("") {
        EFFECT_SCROLL_STEP
        | EFFECT_HINTS_GENERAL
        | EFFECT_HINTS_QUEUE
        | EFFECT_HINTS_DOWNLOAD
        | EFFECT_HINTS_INCOGNITO
        | EFFECT_HINTS_COPY_URL
        | EFFECT_HINTS => MODE_HINTS.to_string(),
        EFFECT_CREATE_MARK | EFFECT_GOTO_MARK => MODE_NORMAL.to_string(),
        EFFECT_ENTER_VISUAL => effect
            .get("mode")
            .and_then(Value::as_str)
            .unwrap_or(MODE_VISUAL_LINE)
            .to_string(),
        EFFECT_INSERT_MODE => MODE_INSERT.to_string(),
        _ => MODE_NORMAL.to_string(),
    }
}

pub fn key_name(event_key: &str) -> String {
    match event_key {
        "Escape" => "Esc".to_string(),
        "ArrowLeft" => "left".to_string(),
        "ArrowUp" => "up".to_string(),
        "ArrowRight" => "right".to_string(),
        "ArrowDown" => "down".to_string(),
        " " => "space".to_string(),
        "\n" => "enter".to_string(),
        _ if event_key.len() == 1 => event_key.to_string(),
        _ => event_key.to_string().to_lowercase(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scroll_step_effects_defer_to_runtime_setting() {
        assert_eq!(
            command_effect("scrollDown", 3, None),
            json!({"kind": "scroll-step", "axis": "y", "direction": 1, "count": 3})
        );
        assert_eq!(
            command_effect("scrollLeft", 2, None),
            json!({"kind": "scroll-step", "axis": "x", "direction": -1, "count": 2})
        );
    }

    #[test]
    fn reload_effect_keeps_hard_option_from_registry_entry() {
        let registry = KeyMapRegistry::from_defaults();
        let entry = registry.key_to_registry.get("R");
        assert_eq!(
            command_effect("reload", 1, entry),
            json!({"kind": "background", "command": "reload", "hard": true})
        );
    }

    #[test]
    fn command_count_option_multiplies_typed_count() {
        let registry = KeyMapRegistry::from_defaults();
        let mappings = registry.parse_user_mappings("map q scrollDown count=5");
        let entry = mappings.get("q").and_then(Option::as_ref);
        assert_eq!(
            command_effect("scrollDown", 2, entry),
            json!({"kind": "scroll-step", "axis": "y", "direction": 1, "count": 10})
        );
    }

    #[test]
    fn prefix_fallback_runs_root_mapping_without_old_count() {
        let registry = KeyMapRegistry::from_defaults();
        let mappings = std::collections::HashMap::new();
        let state = KeyState {
            mode: MODE_NORMAL.to_string(),
            sequence: "g".to_string(),
            count_text: "7".to_string(),
            input: String::new(),
        };
        assert_eq!(
            handle_key(&state, "j", false, &registry, &mappings, "")
                .get("effect")
                .and_then(|effect| effect.get("count"))
                .and_then(Value::as_i64),
            Some(1)
        );
    }

    #[test]
    fn pass_keys_are_passed_only_at_root_without_count_prefix() {
        let registry = KeyMapRegistry::from_defaults();
        let mappings = std::collections::HashMap::new();
        let state = KeyState::new();
        let result = handle_key(&state, "j", false, &registry, &mappings, "j");
        assert_eq!(Some(false), result.get("prevent").and_then(Value::as_bool));
        assert!(result.get("effect").is_some_and(Value::is_null));

        let mut state = KeyState::new();
        state.count_text = "2".to_string();
        let result = handle_key(&state, "j", false, &registry, &mappings, "j");
        assert_eq!(Some(true), result.get("prevent").and_then(Value::as_bool));
        assert_eq!(
            Some("scroll-step"),
            result
                .get("effect")
                .and_then(|effect| effect.get("kind"))
                .and_then(Value::as_str)
        );

        let mut state = KeyState::new();
        state.sequence = "g".to_string();
        let result = handle_key(&state, "t", false, &registry, &mappings, "t");
        assert_eq!(Some(true), result.get("prevent").and_then(Value::as_bool));
        assert_eq!(
            Some("background"),
            result
                .get("effect")
                .and_then(|effect| effect.get("kind"))
                .and_then(Value::as_str)
        );
    }

    #[test]
    fn go_up_url_matches_vimium_path_hierarchy() {
        assert_eq!(
            go_up_url("https://example.com/a/b/c", 1).as_deref(),
            Some("https://example.com/a/b")
        );
        assert_eq!(
            go_up_url("https://example.com/a/b/c/", 2).as_deref(),
            Some("https://example.com/a")
        );
        assert_eq!(go_up_url("https://example.com", 1), None);
    }

    #[test]
    fn root_url_preserves_origin() {
        assert_eq!(
            root_url("https://example.com/a/b?x=1").as_deref(),
            Some("https://example.com")
        );
        assert_eq!(
            root_url("http://example.com:8080/a").as_deref(),
            Some("http://example.com:8080")
        );
    }
}
