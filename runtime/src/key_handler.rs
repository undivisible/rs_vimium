use crate::commands::{CommandEntry, KeyMapRegistry, RegistryEntry};
use serde_json::{json, Value};

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
            mode: "normal".to_string(),
            sequence: String::new(),
            count_text: String::new(),
            input: String::new(),
        }
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
            "state": { "mode": "normal", "sequence": "", "countText": "", "input": "" },
            "effect": { "kind": "clear-overlays" },
            "prevent": false
        });
    }

    if state.mode == "insert" || editable {
        return json!({ "state": state_to_val(state), "effect": null, "prevent": false });
    }

    if state.mode == "hints" {
        return json!({ "state": state_to_val(state), "effect": null, "prevent": false });
    }

    if state.mode == "visual" {
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
            "state": { "mode": "normal", "sequence": "", "countText": count_text, "input": "" },
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
            "state": { "mode": "normal", "sequence": sequence, "countText": count_text, "input": "" },
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
            "state": { "mode": "normal", "sequence": "", "countText": "", "input": "" },
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
            Some("exit-visual") | Some("none") | None => "normal",
            Some("stay-visual") => "visual",
            _ => "normal",
        };
        json!({
            "state": { "mode": post_mode, "sequence": "", "countText": "", "input": "" },
            "effect": effect,
            "prevent": true
        })
    } else if is_prefix {
        json!({
            "state": { "mode": "visual", "sequence": sequence, "countText": count_text, "input": "" },
            "effect": null,
            "prevent": true
        })
    } else {
        json!({
            "state": { "mode": "normal", "sequence": "", "countText": "", "input": "" },
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
        "scrollDown" => json!({"kind": "scroll-step", "axis": "y", "direction": 1, "count": count}),
        "scrollUp" => json!({"kind": "scroll-step", "axis": "y", "direction": -1, "count": count}),
        "scrollToTop" => json!({"kind": "scroll-top", "count": count}),
        "scrollToBottom" => json!({"kind": "scroll-bottom"}),
        "scrollPageDown" => json!({"kind": "half-scroll", "direction": 1, "count": count}),
        "scrollPageUp" => json!({"kind": "half-scroll", "direction": -1, "count": count}),
        "scrollFullPageDown" => json!({"kind": "full-scroll", "direction": 1, "count": count}),
        "scrollFullPageUp" => json!({"kind": "full-scroll", "direction": -1, "count": count}),
        "scrollLeft" => {
            json!({"kind": "scroll-step", "axis": "x", "direction": -1, "count": count})
        }
        "scrollRight" => {
            json!({"kind": "scroll-step", "axis": "x", "direction": 1, "count": count})
        }
        "scrollToLeft" => json!({"kind": "scroll-left"}),
        "scrollToRight" => json!({"kind": "scroll-right"}),
        "reload" => {
            let hard = entry
                .and_then(|entry| entry.options.get("hard"))
                .and_then(Value::as_bool)
                .unwrap_or(false);
            if bkg {
                json!({"kind": "background", "command": "reload", "hard": hard})
            } else {
                json!({"kind": "reload", "hard": hard})
            }
        }
        "copyCurrentUrl" => json!({"kind": "copy-url"}),
        "openCopiedUrlInCurrentTab" => json!({"kind": "open-clipboard", "newTab": false}),
        "openCopiedUrlInNewTab" => json!({"kind": "open-clipboard", "newTab": true}),
        "goUp" => json!({"kind": "go-up", "count": count}),
        "goToRoot" => json!({"kind": "go-root"}),
        "enterInsertMode" => json!({"kind": "insert-mode"}),
        "enterVisualMode" => json!({"kind": "enter-visual", "mode": "visual"}),
        "enterVisualLineMode" => json!({"kind": "enter-visual", "mode": "visual-line"}),
        "focusInput" => json!({"kind": "focus-input", "count": count}),
        "LinkHints.activateModeToOpenInNewTab" => {
            json!({"kind": "hints", "newTab": true, "foreground": false})
        }
        "LinkHints.activateModeToOpenInNewForegroundTab" => {
            json!({"kind": "hints", "newTab": true, "foreground": true})
        }
        "LinkHints.activateMode" => json!({"kind": "hints-general"}),
        "LinkHints.activateModeWithQueue" => json!({"kind": "hints-queue"}),
        "LinkHints.activateModeToDownloadLink" => json!({"kind": "hints-download"}),
        "LinkHints.activateModeToOpenIncognito" => json!({"kind": "hints-incognito"}),
        "LinkHints.activateModeToCopyLinkUrl" => json!({"kind": "hints-copy-url"}),
        "goPrevious" => json!({"kind": "follow-pattern", "pattern": "previous"}),
        "goNext" => json!({"kind": "follow-pattern", "pattern": "next"}),
        "nextFrame" => json!({"kind": "cycle-frame", "direction": 1}),
        "mainFrame" => json!({"kind": "focus-main-frame"}),
        "Marks.activateCreateMode" => json!({"kind": "create-mark"}),
        "Marks.activateGotoMode" => json!({"kind": "goto-mark"}),
        "Vomnibar.activate" => {
            json!({"kind": "vomnibar", "newTab": false, "options": entry.map(|entry| entry.options.clone()).unwrap_or_else(|| json!({}))})
        }
        "Vomnibar.activateInNewTab" => {
            json!({"kind": "vomnibar", "newTab": true, "options": entry.map(|entry| entry.options.clone()).unwrap_or_else(|| json!({}))})
        }
        "Vomnibar.activateBookmarks" => {
            json!({"kind": "vomnibar-bookmarks", "newTab": false, "options": entry.map(|entry| entry.options.clone()).unwrap_or_else(|| json!({}))})
        }
        "Vomnibar.activateBookmarksInNewTab" => {
            json!({"kind": "vomnibar-bookmarks", "newTab": true, "options": entry.map(|entry| entry.options.clone()).unwrap_or_else(|| json!({}))})
        }
        "Vomnibar.activateTabSelection" => json!({"kind": "vomnibar-tabs"}),
        "Vomnibar.activateCommandSelection" => {
            json!({"kind": "vomnibar-commands"})
        }
        "Vomnibar.activateEditUrl" => {
            json!({"kind": "vomnibar-edit-url", "newTab": false, "options": entry.map(|entry| entry.options.clone()).unwrap_or_else(|| json!({}))})
        }
        "Vomnibar.activateEditUrlInNewTab" => {
            json!({"kind": "vomnibar-edit-url", "newTab": true, "options": entry.map(|entry| entry.options.clone()).unwrap_or_else(|| json!({}))})
        }
        "enterFindMode" => json!({"kind": "find"}),
        "performFind" => json!({"kind": "find-next", "reverse": false}),
        "performBackwardsFind" => json!({"kind": "find-next", "reverse": true}),
        "findSelected" => json!({"kind": "find-selected", "reverse": false}),
        "findSelectedBackwards" => json!({"kind": "find-selected", "reverse": true}),
        "goBack" => json!({"kind": "history-back"}),
        "goForward" => json!({"kind": "history-forward"}),
        "createTab" | "previousTab" | "nextTab" | "visitPreviousTab" | "firstTab" | "lastTab"
        | "duplicateTab" | "togglePinTab" | "toggleMuteTab" | "removeTab" | "restoreTab"
        | "moveTabToNewWindow" | "closeTabsOnLeft" | "closeTabsOnRight" | "closeOtherTabs"
        | "moveTabLeft" | "moveTabRight" | "setZoom" | "zoomIn" | "zoomOut" | "zoomReset" => {
            json!({"kind": "background", "command": background_command_for_registry_name(cmd_name), "count": count, "options": entry.map(|entry| entry.options.clone()).unwrap_or_else(|| json!({}))})
        }
        "passNextKey" => json!({"kind": "pass-next-key"}),
        "toggleViewSource" => json!({"kind": "view-source"}),
        "showHelp" => json!({"kind": "help"}),
        _ => {
            if bkg {
                json!({"kind": "background", "command": cmd_name})
            } else {
                json!({"kind": "none"})
            }
        }
    }
}

fn visual_effect(cmd_name: &str, _count: i64, _entry: Option<&CommandEntry>) -> Value {
    match cmd_name {
        "j" | "scrollDown" => {
            json!({"kind": "visual-move", "direction": "forward", "granularity": "line"})
        }
        "k" | "scrollUp" => {
            json!({"kind": "visual-move", "direction": "backward", "granularity": "line"})
        }
        "h" | "scrollLeft" => {
            json!({"kind": "visual-move", "direction": "backward", "granularity": "character"})
        }
        "l" | "scrollRight" => {
            json!({"kind": "visual-move", "direction": "forward", "granularity": "character"})
        }
        "w" => json!({"kind": "visual-move", "direction": "forward", "granularity": "vimword"}),
        "b" => json!({"kind": "visual-move", "direction": "backward", "granularity": "vimword"}),
        "e" => json!({"kind": "visual-move", "direction": "forward", "granularity": "vimword-end"}),
        "0" => {
            json!({"kind": "visual-move", "direction": "backward", "granularity": "lineboundary"})
        }
        "$" => {
            json!({"kind": "visual-move", "direction": "forward", "granularity": "lineboundary"})
        }
        "gg" | "scrollToTop" => {
            json!({"kind": "visual-move", "direction": "backward", "granularity": "document-boundary"})
        }
        "G" | "scrollToBottom" => {
            json!({"kind": "visual-move", "direction": "forward", "granularity": "document-boundary"})
        }
        "y" => json!({"kind": "visual-copy", "stay": "exit-visual"}),
        "Esc" => json!({"kind": "exit-visual"}),
        _ => json!({"kind": "stay-visual"}),
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
        "hints" | "hints-general" | "hints-queue" | "hints-download" | "hints-incognito"
        | "hints-copy-url" => "hints".to_string(),
        "create-mark" | "goto-mark" => "normal".to_string(),
        "enter-visual" => effect
            .get("mode")
            .and_then(Value::as_str)
            .unwrap_or("visual")
            .to_string(),
        "insert-mode" => "insert".to_string(),
        _ => "normal".to_string(),
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
            mode: "normal".to_string(),
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
