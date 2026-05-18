use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap};

#[derive(Debug, Clone, PartialEq)]
pub struct CommandEntry {
    pub name: String,
    pub desc: String,
    pub details: Option<String>,
    pub group: String,
    pub advanced: bool,
    pub background: bool,
    pub top_frame: bool,
    pub no_repeat: bool,
    pub repeat_limit: Option<i64>,
    pub options: Value,
}

pub fn all_commands() -> Vec<CommandEntry> {
    vec![
        c(
            "scrollDown",
            "Scroll down",
            "navigation",
            false,
            false,
            false,
            false,
            None,
            json!({}),
        ),
        c(
            "scrollUp",
            "Scroll up",
            "navigation",
            false,
            false,
            false,
            false,
            None,
            json!({}),
        ),
        c(
            "scrollToTop",
            "Scroll to the top of the page",
            "navigation",
            false,
            false,
            false,
            false,
            None,
            json!({}),
        ),
        c(
            "scrollToBottom",
            "Scroll to the bottom of the page",
            "navigation",
            false,
            false,
            false,
            false,
            None,
            json!({}),
        ),
        c(
            "scrollPageDown",
            "Scroll a half page down",
            "navigation",
            false,
            false,
            false,
            false,
            None,
            json!({}),
        ),
        c(
            "scrollPageUp",
            "Scroll a half page up",
            "navigation",
            false,
            false,
            false,
            false,
            None,
            json!({}),
        ),
        c(
            "scrollFullPageDown",
            "Scroll a full page down",
            "navigation",
            false,
            false,
            false,
            false,
            None,
            json!({}),
        ),
        c(
            "scrollFullPageUp",
            "Scroll a full page up",
            "navigation",
            false,
            false,
            false,
            false,
            None,
            json!({}),
        ),
        c(
            "scrollLeft",
            "Scroll left",
            "navigation",
            false,
            false,
            false,
            false,
            None,
            json!({}),
        ),
        ca(
            "scrollRight",
            "Scroll right",
            "navigation",
            true,
            false,
            false,
            false,
            None,
            json!({}),
        ),
        ca(
            "scrollToLeft",
            "Scroll all the way to the left",
            "navigation",
            true,
            false,
            false,
            false,
            None,
            json!({}),
        ),
        c(
            "scrollToRight",
            "Scroll all the way to the right",
            "navigation",
            false,
            false,
            false,
            false,
            None,
            json!({}),
        ),
        c(
            "reload",
            "Reload the page",
            "navigation",
            false,
            true,
            false,
            false,
            None,
            json!({"hard": "Perform a hard reload, forcing the browser to bypass its cache."}),
        ),
        c(
            "copyCurrentUrl",
            "Copy the current URL to the clipboard",
            "navigation",
            false,
            false,
            false,
            true,
            None,
            json!({}),
        ),
        c(
            "openCopiedUrlInCurrentTab",
            "Open the clipboard's URL in the current tab",
            "navigation",
            false,
            false,
            false,
            true,
            None,
            json!({}),
        ),
        c(
            "openCopiedUrlInNewTab",
            "Open the clipboard's URL in a new tab",
            "navigation",
            false,
            false,
            false,
            true,
            None,
            json!({"position": "Where to place the tab in the tab bar. One of `start`, `before`, `after`, `end`. `after` is the default."}),
        ),
        ca(
            "goUp",
            "Go up the URL hierarchy",
            "navigation",
            true,
            false,
            false,
            false,
            None,
            json!({}),
        ),
        ca(
            "goToRoot",
            "Go to the root of current URL hierarchy",
            "navigation",
            true,
            false,
            false,
            false,
            None,
            json!({}),
        ),
        c(
            "enterInsertMode",
            "Enter insert mode",
            "navigation",
            false,
            false,
            false,
            true,
            None,
            json!({}),
        ),
        c(
            "enterVisualMode",
            "Enter visual mode",
            "navigation",
            false,
            false,
            false,
            true,
            None,
            json!({}),
        ),
        ca(
            "enterVisualLineMode",
            "Enter visual line mode",
            "navigation",
            true,
            false,
            false,
            true,
            None,
            json!({}),
        ),
        ca(
            "passNextKey",
            "Pass the next key to the page",
            "navigation",
            true,
            false,
            false,
            false,
            None,
            json!({}),
        ),
        c(
            "focusInput",
            "Focus the first text input on the page",
            "navigation",
            false,
            false,
            false,
            false,
            None,
            json!({}),
        ),
        ca(
            "LinkHints.activateMode",
            "Open a link in the current tab",
            "navigation",
            true,
            false,
            false,
            false,
            None,
            json!({"action": "one of `hover`, `focus`, `copy-text`. When a link is selected, instead of clicking on the link, perform the specified action."}),
        ),
        c(
            "LinkHints.activateModeToOpenInNewTab",
            "Open a link in a new tab",
            "navigation",
            false,
            false,
            false,
            false,
            None,
            json!({}),
        ),
        c(
            "LinkHints.activateModeToOpenInNewForegroundTab",
            "Open a link in a new tab & switch to it",
            "navigation",
            false,
            false,
            false,
            false,
            None,
            json!({}),
        ),
        ca(
            "LinkHints.activateModeWithQueue",
            "Open multiple links in a new tab",
            "navigation",
            true,
            false,
            false,
            true,
            None,
            json!({}),
        ),
        ca(
            "LinkHints.activateModeToDownloadLink",
            "Download link url",
            "navigation",
            true,
            false,
            false,
            false,
            None,
            json!({}),
        ),
        ca(
            "LinkHints.activateModeToOpenIncognito",
            "Open a link in incognito window",
            "navigation",
            true,
            false,
            false,
            false,
            None,
            json!({}),
        ),
        ca(
            "LinkHints.activateModeToCopyLinkUrl",
            "Copy a link URL to the clipboard",
            "navigation",
            true,
            false,
            false,
            false,
            None,
            json!({}),
        ),
        ca(
            "goPrevious",
            "Follow the link labeled previous or <",
            "navigation",
            true,
            false,
            false,
            true,
            None,
            json!({}),
        ),
        ca(
            "goNext",
            "Follow the link labeled next or >",
            "navigation",
            true,
            false,
            false,
            true,
            None,
            json!({}),
        ),
        ca(
            "nextFrame",
            "Select the next frame on the page",
            "navigation",
            true,
            true,
            false,
            false,
            None,
            json!({}),
        ),
        ca(
            "mainFrame",
            "Select the page's main/top frame",
            "navigation",
            true,
            false,
            true,
            true,
            None,
            json!({}),
        ),
        cad(
            "Marks.activateCreateMode",
            "Create a new mark",
            "navigation",
            true,
            false,
            false,
            true,
            None,
            Some("Do this by typing the key bound to this command, and then a letter. This will set a mark bound to that letter. Lowercase letters are local marks and uppercase letters are global marks."),
            json!({"swap": "Swap global and local marks. This option exists because in a browser, global marks are generally more useful than local marks, and so it may be desirable to make lowercase letters represent global marks rather than local marks."}),
        ),
        ca(
            "Marks.activateGotoMode",
            "Jump to a mark",
            "navigation",
            true,
            false,
            false,
            true,
            None,
            json!({"swap": "Swap global and local marks. This option exists because in a browser, global marks are generally more useful than local marks, and so it may be desirable to make lowercase letters represent global marks rather than local marks."}),
        ),
        c(
            "Vomnibar.activate",
            "Open URL, bookmark or history entry",
            "vomnibar",
            false,
            false,
            true,
            false,
            None,
            json!({"query": "The text to prefill the Vomnibar with.", "keyword": "The keyword of a search engine defined in the \"Custom search engines\" section of the Vimium Options page. The Vomnibar will be scoped to use that search engine."}),
        ),
        c(
            "Vomnibar.activateInNewTab",
            "Open URL, bookmark or history entry in a new tab",
            "vomnibar",
            false,
            false,
            true,
            false,
            None,
            json!({"query": "The text to prefill the Vomnibar with.", "keyword": "The keyword of a search engine defined in the \"Custom search engines\" section of the Vimium Options page. The Vomnibar will be scoped to use that search engine."}),
        ),
        ca(
            "Vomnibar.activateBookmarks",
            "Open a bookmark",
            "vomnibar",
            true,
            false,
            true,
            false,
            None,
            json!({"query": "The text to prefill the Vomnibar with."}),
        ),
        ca(
            "Vomnibar.activateBookmarksInNewTab",
            "Open a bookmark in a new tab",
            "vomnibar",
            true,
            false,
            true,
            false,
            None,
            json!({"query": "The text to prefill the Vomnibar with."}),
        ),
        ca(
            "Vomnibar.activateTabSelection",
            "Search through your open tabs",
            "vomnibar",
            true,
            false,
            true,
            false,
            None,
            json!({}),
        ),
        ca(
            "Vomnibar.activateEditUrl",
            "Edit the current URL",
            "vomnibar",
            true,
            false,
            true,
            false,
            None,
            json!({}),
        ),
        ca(
            "Vomnibar.activateEditUrlInNewTab",
            "Edit the current URL and open in a new tab",
            "vomnibar",
            true,
            false,
            true,
            false,
            None,
            json!({}),
        ),
        c(
            "enterFindMode",
            "Enter find mode",
            "find",
            false,
            false,
            false,
            true,
            None,
            json!({}),
        ),
        c(
            "performFind",
            "Cycle forward to the next find match",
            "find",
            false,
            false,
            false,
            false,
            None,
            json!({}),
        ),
        c(
            "performBackwardsFind",
            "Cycle backward to the previous find match",
            "find",
            false,
            false,
            false,
            false,
            None,
            json!({}),
        ),
        ca(
            "findSelected",
            "Find the selected text",
            "find",
            true,
            false,
            false,
            false,
            None,
            json!({}),
        ),
        ca(
            "findSelectedBackwards",
            "Find the selected text, searching backwards",
            "find",
            true,
            false,
            false,
            false,
            None,
            json!({}),
        ),
        c(
            "goBack",
            "Go back in history",
            "history",
            false,
            false,
            false,
            false,
            None,
            json!({}),
        ),
        c(
            "goForward",
            "Go forward in history",
            "history",
            false,
            false,
            false,
            false,
            None,
            json!({}),
        ),
        c(
            "createTab",
            "Create new tab",
            "tabs",
            false,
            true,
            false,
            false,
            Some(20),
            json!({"(any url)": "Open this URL, rather than the browser's new tab page. E.g.: `map X createTab https://example.com`", "window": "Create the tab in a new window", "incognito": "Create the tab in an incognito window", "position": "Where to place the tab in the tab bar. One of `start`, `before`, `after`, `end`. `after` is the default."}),
        ),
        c(
            "previousTab",
            "Go one tab left",
            "tabs",
            false,
            true,
            false,
            false,
            None,
            json!({}),
        ),
        c(
            "nextTab",
            "Go one tab right",
            "tabs",
            false,
            true,
            false,
            false,
            None,
            json!({}),
        ),
        ca(
            "visitPreviousTab",
            "Go to previously-visited tab",
            "tabs",
            true,
            true,
            false,
            false,
            None,
            json!({}),
        ),
        c(
            "firstTab",
            "Go to the first tab",
            "tabs",
            false,
            true,
            false,
            false,
            None,
            json!({}),
        ),
        c(
            "lastTab",
            "Go to the last tab",
            "tabs",
            false,
            true,
            false,
            false,
            None,
            json!({}),
        ),
        ca(
            "duplicateTab",
            "Duplicate current tab",
            "tabs",
            true,
            true,
            false,
            false,
            Some(20),
            json!({}),
        ),
        ca(
            "togglePinTab",
            "Pin or unpin current tab",
            "tabs",
            true,
            true,
            false,
            false,
            None,
            json!({}),
        ),
        ca(
            "toggleMuteTab",
            "Mute or unmute current tab",
            "tabs",
            true,
            true,
            false,
            true,
            None,
            json!({"all": "Mute all tabs.", "other": "Mute every tab except the current one."}),
        ),
        c(
            "removeTab",
            "Close current tab",
            "tabs",
            false,
            true,
            false,
            false,
            Some(25),
            json!({}),
        ),
        ca(
            "restoreTab",
            "Restore closed tab",
            "tabs",
            true,
            true,
            false,
            false,
            Some(20),
            json!({}),
        ),
        ca(
            "moveTabToNewWindow",
            "Move tab to new window",
            "tabs",
            true,
            true,
            false,
            false,
            None,
            json!({"level": "The zoom level. This can be a range of [0.25, 5.0]. 1.0 is the default."}),
        ),
        ca(
            "closeTabsOnLeft",
            "Close tabs on the left",
            "tabs",
            true,
            true,
            false,
            false,
            None,
            json!({}),
        ),
        ca(
            "closeTabsOnRight",
            "Close tabs on the right",
            "tabs",
            true,
            true,
            false,
            false,
            None,
            json!({}),
        ),
        ca(
            "closeOtherTabs",
            "Close all other tabs",
            "tabs",
            true,
            true,
            false,
            true,
            None,
            json!({}),
        ),
        ca(
            "moveTabLeft",
            "Move tab to the left",
            "tabs",
            true,
            true,
            false,
            false,
            None,
            json!({}),
        ),
        ca(
            "moveTabRight",
            "Move tab to the right",
            "tabs",
            true,
            true,
            false,
            false,
            None,
            json!({}),
        ),
        ca(
            "setZoom",
            "Set zoom",
            "tabs",
            true,
            true,
            false,
            false,
            None,
            json!({}),
        ),
        ca(
            "zoomIn",
            "Zoom in",
            "tabs",
            true,
            true,
            false,
            false,
            None,
            json!({}),
        ),
        ca(
            "zoomOut",
            "Zoom out",
            "tabs",
            true,
            true,
            false,
            false,
            None,
            json!({}),
        ),
        ca(
            "zoomReset",
            "Reset zoom",
            "tabs",
            true,
            true,
            false,
            false,
            None,
            json!({}),
        ),
        ca(
            "toggleViewSource",
            "View page source",
            "misc",
            true,
            false,
            false,
            true,
            None,
            json!({}),
        ),
        c(
            "showHelp",
            "Show help",
            "misc",
            false,
            false,
            true,
            true,
            None,
            json!({}),
        ),
    ]
}

#[allow(clippy::too_many_arguments)]
fn c(
    name: &str,
    desc: &str,
    group: &str,
    advanced: bool,
    background: bool,
    top_frame: bool,
    no_repeat: bool,
    repeat_limit: Option<i64>,
    options: Value,
) -> CommandEntry {
    CommandEntry {
        name: name.to_string(),
        desc: desc.to_string(),
        details: None,
        group: group.to_string(),
        advanced,
        background,
        top_frame,
        no_repeat,
        repeat_limit,
        options,
    }
}

#[allow(clippy::too_many_arguments)]
fn cd(
    name: &str,
    desc: &str,
    group: &str,
    advanced: bool,
    background: bool,
    top_frame: bool,
    no_repeat: bool,
    repeat_limit: Option<i64>,
    details: Option<&str>,
    options: Value,
) -> CommandEntry {
    let mut entry = c(
        name,
        desc,
        group,
        advanced,
        background,
        top_frame,
        no_repeat,
        repeat_limit,
        options,
    );
    entry.details = details.map(ToString::to_string);
    entry
}

#[allow(clippy::too_many_arguments)]
fn ca(
    name: &str,
    desc: &str,
    group: &str,
    advanced: bool,
    background: bool,
    top_frame: bool,
    no_repeat: bool,
    repeat_limit: Option<i64>,
    options: Value,
) -> CommandEntry {
    let mut entry = c(
        name,
        desc,
        group,
        advanced,
        background,
        top_frame,
        no_repeat,
        repeat_limit,
        options,
    );
    entry.advanced = advanced;
    entry
}

#[allow(clippy::too_many_arguments)]
fn cad(
    name: &str,
    desc: &str,
    group: &str,
    advanced: bool,
    background: bool,
    top_frame: bool,
    no_repeat: bool,
    repeat_limit: Option<i64>,
    details: Option<&str>,
    options: Value,
) -> CommandEntry {
    let mut entry = cd(
        name,
        desc,
        group,
        advanced,
        background,
        top_frame,
        no_repeat,
        repeat_limit,
        details,
        options,
    );
    entry.advanced = advanced;
    entry
}

pub type RegistryEntry = CommandEntry;

#[derive(Debug, Clone)]
pub struct KeyMapping {
    pub key_sequence: Vec<String>,
    pub command_name: String,
    pub options: Value,
    pub registry: RegistryEntry,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ParsedKeyMappings {
    pub key_to_command: HashMap<String, String>,
    pub key_to_registry: HashMap<String, Option<RegistryEntry>>,
    pub key_to_mapped_key: HashMap<String, String>,
    pub validation_errors: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct KeyMapRegistry {
    pub key_to_command: BTreeMap<String, String>,
    pub key_to_registry: BTreeMap<String, RegistryEntry>,
    pub commands_by_name: HashMap<String, RegistryEntry>,
}

impl KeyMapRegistry {
    pub fn from_defaults() -> Self {
        let mut registry = KeyMapRegistry {
            key_to_command: BTreeMap::new(),
            key_to_registry: BTreeMap::new(),
            commands_by_name: HashMap::new(),
        };

        let cmds = all_commands();
        for cmd in &cmds {
            registry
                .commands_by_name
                .insert(cmd.name.clone(), cmd.clone());
        }

        let defaults = default_key_bindings();
        for (key_seq, cmd_name, options) in &defaults {
            let joined = key_seq.join("");
            registry
                .key_to_command
                .insert(joined.clone(), cmd_name.clone());
            if let Some(entry) = registry.commands_by_name.get(cmd_name) {
                let mut entry = entry.clone();
                entry.options = options.clone();
                registry.key_to_registry.insert(joined, entry);
            }
        }

        registry
    }

    pub fn parse_user_mappings(&self, config_text: &str) -> HashMap<String, Option<RegistryEntry>> {
        self.parse_key_mappings(config_text).key_to_registry
    }

    pub fn parse_key_mappings(&self, config_text: &str) -> ParsedKeyMappings {
        self.parse_key_mappings_from(config_text, ParsedKeyMappings::default())
    }

    pub fn parse_effective_key_mappings(&self, config_text: &str) -> ParsedKeyMappings {
        let mut parsed = ParsedKeyMappings::default();
        for (key, command) in &self.key_to_command {
            parsed.key_to_command.insert(key.clone(), command.clone());
        }
        for (key, entry) in &self.key_to_registry {
            parsed
                .key_to_registry
                .insert(key.clone(), Some(entry.clone()));
        }
        self.parse_key_mappings_from(config_text, parsed)
    }

    fn parse_key_mappings_from(
        &self,
        config_text: &str,
        mut parsed: ParsedKeyMappings,
    ) -> ParsedKeyMappings {
        for line in parse_lines(config_text) {
            let tokens: Vec<&str> = line.split_whitespace().collect();
            let action = tokens[0].to_lowercase();
            match action.as_str() {
                "map" => {
                    if tokens.len() < 3 {
                        parsed
                            .validation_errors
                            .push(format!("map requires at least 2 arguments on line {line}"));
                        continue;
                    }
                    let key_seq = parse_key_sequence(tokens[1]).join("");
                    let cmd_name = tokens[2].to_string();
                    if let Some(command_info) = self.commands_by_name.get(&cmd_name) {
                        let option_string = command_options_text(&line);
                        let options = parse_command_options(&option_string);
                        if let Some(error) = validate_command_options(command_info, &options) {
                            parsed.validation_errors.push(error);
                            continue;
                        }
                        let mut registry_entry = command_info.clone();
                        registry_entry.options = options;
                        parsed.key_to_command.insert(key_seq.clone(), cmd_name);
                        parsed.key_to_registry.insert(key_seq, Some(registry_entry));
                    } else {
                        parsed.validation_errors.push(format!(
                            "{cmd_name} is not a valid command in the line: {line}"
                        ));
                    }
                }
                "unmap" => {
                    if tokens.len() != 2 {
                        parsed
                            .validation_errors
                            .push(format!("Incorrect usage for unmap in the line: {line}"));
                        continue;
                    }
                    let key_seq = parse_key_sequence(tokens[1]).join("");
                    parsed.key_to_command.insert(key_seq.clone(), String::new());
                    parsed.key_to_registry.insert(key_seq.clone(), None);
                    parsed.key_to_mapped_key.remove(&key_seq);
                }
                "unmapall" => {
                    parsed.key_to_command.clear();
                    parsed.key_to_registry.clear();
                    parsed.key_to_mapped_key.clear();
                }
                "mapkey" => {
                    if tokens.len() != 3 {
                        parsed
                            .validation_errors
                            .push(format!("Incorrect usage for mapKey in the line: {line}"));
                        continue;
                    }
                    let from_key = parse_key_sequence(tokens[1]);
                    let to_key = parse_key_sequence(tokens[2]);
                    if from_key.len() == 1 && to_key.len() == 1 {
                        parsed
                            .key_to_mapped_key
                            .insert(from_key[0].clone(), to_key[0].clone());
                    } else {
                        parsed.validation_errors.push(format!(
                            "mapkey only supports mapping keys which are single characters. Line: {line}"
                        ));
                    }
                }
                _ => {
                    parsed.validation_errors.push(format!(
                        "{action} is not a valid config command in line: {line}"
                    ));
                }
            }
        }

        parsed
    }

    pub fn resolve_command<'a>(
        &'a self,
        sequence: &str,
        user_mappings: &'a HashMap<String, Option<RegistryEntry>>,
    ) -> Option<(&'a str, Option<&'a RegistryEntry>)> {
        if let Some(cmd_override) = user_mappings.get(sequence) {
            let Some(entry) = cmd_override else {
                return None;
            };
            return Some((entry.name.as_str(), Some(entry)));
        }
        if let Some(cmd_name) = self.key_to_command.get(sequence) {
            let entry = self.key_to_registry.get(sequence);
            return Some((cmd_name.as_str(), entry));
        }
        None
    }

    pub fn is_prefix(
        &self,
        sequence: &str,
        user_mappings: &HashMap<String, Option<RegistryEntry>>,
    ) -> bool {
        if user_mappings.get(sequence).is_some_and(Option::is_none) {
            return false;
        }
        for key in self.key_to_command.keys() {
            if key.starts_with(sequence) && key != sequence {
                return true;
            }
        }
        for (key, entry) in user_mappings {
            if key.starts_with(sequence) && key != sequence && entry.is_some() {
                return true;
            }
        }
        false
    }

    pub fn has_continuation_mapping(
        &self,
        sequence: &str,
        key: &str,
        user_mappings: &HashMap<String, Option<RegistryEntry>>,
    ) -> bool {
        if sequence.is_empty() {
            return false;
        }
        let candidate = format!("{sequence}{key}");
        self.resolve_command(&candidate, user_mappings).is_some()
            || self.is_prefix(&candidate, user_mappings)
    }

    pub fn session_metadata(&self, config_text: &str) -> Value {
        let parsed = self.parse_effective_key_mappings(config_text);
        let pass_next_key_keys = parsed
            .key_to_registry
            .iter()
            .filter_map(|(key, entry)| {
                let entry = entry.as_ref()?;
                (entry.name == "passNextKey" && key.len() > 1).then(|| key.clone())
            })
            .collect::<Vec<_>>();
        let mut command_to_options_to_keys = serde_json::Map::new();
        for (key, entry) in &parsed.key_to_registry {
            let Some(entry) = entry else {
                continue;
            };
            let option_string = format_option_string(&entry.options);
            let command_entry = command_to_options_to_keys
                .entry(entry.name.clone())
                .or_insert_with(|| json!({}));
            if let Some(options_map) = command_entry.as_object_mut() {
                let keys = options_map
                    .entry(option_string)
                    .or_insert_with(|| json!([]));
                if let Some(keys) = keys.as_array_mut() {
                    keys.push(Value::String(key.clone()));
                }
            }
        }
        json!({
            "mapKeyRegistry": parsed.key_to_mapped_key,
            "normalModeKeyStateMapping": normal_mode_key_state_mapping(&parsed),
            "useVimLikeEscape": !parsed.key_to_registry.contains_key("<c-[>"),
            "passNextKeyKeys": pass_next_key_keys,
            "commandToOptionsToKeys": command_to_options_to_keys
        })
    }
}

pub fn parse_lines(text: &str) -> Vec<String> {
    text.replace("\\\n", "")
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#') && !line.starts_with('"'))
        .map(ToOwned::to_owned)
        .collect()
}

fn command_options_text(line: &str) -> String {
    let mut whitespace_count = 0;
    let mut in_whitespace = false;
    for (index, ch) in line.char_indices() {
        if ch.is_whitespace() && !in_whitespace {
            whitespace_count += 1;
            in_whitespace = true;
            if whitespace_count == 3 {
                return line[index..].trim().to_string();
            }
        } else if !ch.is_whitespace() {
            in_whitespace = false;
        }
    }
    String::new()
}

pub fn parse_command_options(mut option_string: &str) -> Value {
    let mut options = serde_json::Map::new();
    while !option_string.is_empty() {
        let trimmed = option_string.trim_start();
        option_string = trimmed;
        if option_string.is_empty() {
            break;
        }
        let (key, value, consumed) =
            if let Some((key, value, consumed)) = parse_quoted_option(option_string) {
                (key, Value::String(value), consumed)
            } else if let Some((key, value, consumed)) = parse_unquoted_option(option_string) {
                (key, Value::String(value), consumed)
            } else if let Some((key, consumed)) = parse_flag_option(option_string) {
                (key, Value::Bool(true), consumed)
            } else {
                options.insert(option_string.to_string(), Value::Bool(true));
                break;
            };
        options.insert(key, value);
        option_string = &option_string[consumed..];
    }
    if let Some(count) = options.get("count").cloned() {
        let parsed = match count {
            Value::String(raw) => raw.parse::<i64>().ok(),
            Value::Number(raw) => raw.as_i64(),
            _ => None,
        };
        if let Some(count) = parsed {
            options.insert("count".to_string(), Value::Number(count.into()));
        } else {
            options.remove("count");
        }
    }
    Value::Object(options)
}

fn parse_quoted_option(text: &str) -> Option<(String, String, usize)> {
    let equals = text.find("=\"")?;
    let key = &text[..equals];
    if !is_option_name(key) {
        return None;
    }
    let value_start = equals + 2;
    let value_end = text[value_start..].find('"')? + value_start;
    let consumed = value_end + 1;
    if text[consumed..]
        .chars()
        .next()
        .is_some_and(|ch| !ch.is_whitespace())
    {
        return None;
    }
    Some((
        key.to_string(),
        text[value_start..value_end].to_string(),
        consumed,
    ))
}

fn parse_unquoted_option(text: &str) -> Option<(String, String, usize)> {
    let token_end = text.find(char::is_whitespace).unwrap_or(text.len());
    let token = &text[..token_end];
    let equals = token.find('=')?;
    let key = &token[..equals];
    if !is_option_name(key) || token[equals + 1..].is_empty() {
        return None;
    }
    Some((key.to_string(), token[equals + 1..].to_string(), token_end))
}

fn parse_flag_option(text: &str) -> Option<(String, usize)> {
    let token_end = text.find(char::is_whitespace).unwrap_or(text.len());
    let token = &text[..token_end];
    (!token.is_empty() && !token.contains('"')).then(|| (token.to_string(), token_end))
}

fn is_option_name(key: &str) -> bool {
    !key.is_empty() && key.chars().all(|ch| ch.is_ascii_alphabetic())
}

fn validate_command_options(command_info: &CommandEntry, options: &Value) -> Option<String> {
    let options = options.as_object()?;
    let mut allowed = command_info
        .options
        .as_object()
        .map(|map| map.keys().cloned().collect::<Vec<_>>())
        .unwrap_or_default();
    if !command_info.no_repeat {
        allowed.push("count".to_string());
    }
    for option in options.keys() {
        if allowed.iter().any(|allowed| allowed == option) {
            continue;
        }
        if allowed.iter().any(|allowed| allowed == "(any url)") {
            if option.contains("://") {
                continue;
            }
            return Some(format!(
                "Command {} does not support option {}. Is this meant to be a valid URL?",
                command_info.name, option
            ));
        }
        return Some(format!(
            "Command {} does not support option {}",
            command_info.name, option
        ));
    }
    None
}

fn format_option_string(options: &Value) -> String {
    options
        .as_object()
        .map(|map| {
            map.iter()
                .map(|(key, value)| {
                    if value == &Value::Bool(true) {
                        key.clone()
                    } else if let Some(value) = value.as_str() {
                        format!("{key}={value}")
                    } else {
                        format!("{key}={value}")
                    }
                })
                .collect::<Vec<_>>()
                .join(" ")
        })
        .unwrap_or_default()
}

fn normal_mode_key_state_mapping(parsed: &ParsedKeyMappings) -> Value {
    let mut root = serde_json::Map::new();
    let mut keys = parsed
        .key_to_registry
        .keys()
        .cloned()
        .collect::<Vec<String>>();
    keys.sort();
    for key in keys {
        let Some(Some(entry)) = parsed.key_to_registry.get(&key) else {
            continue;
        };
        let sequence = parse_key_sequence(&key);
        insert_key_state_mapping(&mut root, &sequence, entry);
    }
    Value::Object(root)
}

fn insert_key_state_mapping(
    mapping: &mut serde_json::Map<String, Value>,
    sequence: &[String],
    entry: &RegistryEntry,
) {
    let Some((key, rest)) = sequence.split_first() else {
        return;
    };
    if rest.is_empty() {
        mapping.insert(
            key.clone(),
            json!({
                "command": entry.name,
                "noRepeat": entry.no_repeat,
                "repeatLimit": entry.repeat_limit,
                "background": entry.background,
                "topFrame": entry.top_frame,
                "options": entry.options
            }),
        );
        return;
    }
    let child = mapping.entry(key.clone()).or_insert_with(|| json!({}));
    if child.get("command").is_some() {
        return;
    }
    if let Some(child) = child.as_object_mut() {
        insert_key_state_mapping(child, rest, entry);
    }
}

pub fn parse_key_sequence(key: &str) -> Vec<String> {
    if key.is_empty() {
        return vec![];
    }
    if let Some((special, rest)) = split_special_key(key) {
        let mut parts: Vec<&str> = special.split('-').collect();
        let key_part = parts.pop().unwrap_or_default();
        let mut modifiers: Vec<String> = parts.iter().map(|part| part.to_lowercase()).collect();
        modifiers.sort();
        let normalized_key = if key_part.len() == 1 {
            key_part.to_string()
        } else {
            key_part.to_lowercase()
        };
        let mut normalized = vec![format!(
            "<{}>",
            modifiers
                .into_iter()
                .chain(std::iter::once(normalized_key))
                .collect::<Vec<_>>()
                .join("-")
        )];
        normalized.extend(parse_key_sequence(rest));
        return normalized;
    }

    let mut chars = key.chars();
    let first = chars.next().map(|ch| ch.to_string()).unwrap_or_default();
    let rest = chars.as_str();
    let mut parsed = vec![first];
    parsed.extend(parse_key_sequence(rest));
    parsed
}

fn split_special_key(key: &str) -> Option<(&str, &str)> {
    let rest = key.strip_prefix('<')?;
    let end = rest.find('>')?;
    let candidate = &rest[..end];
    if is_named_key(candidate) || is_modified_key(candidate) {
        Some((candidate, &rest[end + 1..]))
    } else {
        None
    }
}

fn is_named_key(key: &str) -> bool {
    key.len() >= 2
        && key
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_alphabetic())
        && key.chars().all(|ch| ch.is_ascii_alphanumeric())
}

fn is_modified_key(key: &str) -> bool {
    let mut parts = key.split('-').collect::<Vec<_>>();
    if parts.len() < 2 {
        return false;
    }
    let key_part = parts.pop().unwrap_or_default();
    !key_part.is_empty()
        && (key_part.chars().count() == 1 || is_named_key(key_part))
        && parts
            .iter()
            .all(|part| part.len() == 1 && part.chars().all(|ch| ch.is_ascii_alphabetic()))
}

fn default_key_bindings() -> Vec<(Vec<String>, String, Value)> {
    let m = |k: &str, c: &str| (parse_key_sequence(k), c.to_string(), json!({}));
    let mo = |k: &str, c: &str, o: Value| (parse_key_sequence(k), c.to_string(), o);
    vec![
        m("?", "showHelp"),
        m("j", "scrollDown"),
        m("k", "scrollUp"),
        m("h", "scrollLeft"),
        m("l", "scrollRight"),
        m("gg", "scrollToTop"),
        m("G", "scrollToBottom"),
        m("zH", "scrollToLeft"),
        m("zL", "scrollToRight"),
        m("<c-e>", "scrollDown"),
        m("<c-y>", "scrollUp"),
        m("d", "scrollPageDown"),
        m("u", "scrollPageUp"),
        m("r", "reload"),
        mo("R", "reload", json!({"hard": true})),
        m("gs", "toggleViewSource"),
        m("i", "enterInsertMode"),
        m("v", "enterVisualMode"),
        m("V", "enterVisualLineMode"),
        m("yy", "copyCurrentUrl"),
        m("p", "openCopiedUrlInCurrentTab"),
        m("P", "openCopiedUrlInNewTab"),
        m("[[", "goPrevious"),
        m("]]", "goNext"),
        m("gi", "focusInput"),
        m("f", "LinkHints.activateMode"),
        m("F", "LinkHints.activateModeToOpenInNewTab"),
        m("<a-f>", "LinkHints.activateModeWithQueue"),
        m("yf", "LinkHints.activateModeToCopyLinkUrl"),
        m("gf", "nextFrame"),
        m("gF", "mainFrame"),
        m("gu", "goUp"),
        m("gU", "goToRoot"),
        m("m", "Marks.activateCreateMode"),
        m("`", "Marks.activateGotoMode"),
        m("o", "Vomnibar.activate"),
        m("O", "Vomnibar.activateInNewTab"),
        m("b", "Vomnibar.activateBookmarks"),
        m("B", "Vomnibar.activateBookmarksInNewTab"),
        m("T", "Vomnibar.activateTabSelection"),
        m("ge", "Vomnibar.activateEditUrl"),
        m("gE", "Vomnibar.activateEditUrlInNewTab"),
        m("/", "enterFindMode"),
        m("n", "performFind"),
        m("N", "performBackwardsFind"),
        m("*", "findSelected"),
        m("#", "findSelectedBackwards"),
        m("H", "goBack"),
        m("L", "goForward"),
        m("t", "createTab"),
        m("J", "previousTab"),
        m("K", "nextTab"),
        m("gT", "previousTab"),
        m("gt", "nextTab"),
        m("^", "visitPreviousTab"),
        m("g0", "firstTab"),
        m("g$", "lastTab"),
        m("yt", "duplicateTab"),
        m("x", "removeTab"),
        m("X", "restoreTab"),
        m("W", "moveTabToNewWindow"),
        m("<a-p>", "togglePinTab"),
        m("<a-m>", "toggleMuteTab"),
        m("zi", "zoomIn"),
        m("zo", "zoomOut"),
        m("z0", "zoomReset"),
        m("<<", "moveTabLeft"),
        m(">>", "moveTabRight"),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_table_matches_legacy_count_and_basic_metadata() {
        let commands = all_commands();
        assert_eq!(73, commands.len());
        let by_name = commands
            .iter()
            .map(|command| (command.name.as_str(), command))
            .collect::<HashMap<_, _>>();
        assert_eq!("Scroll down", by_name["scrollDown"].desc);
        assert_eq!("navigation", by_name["LinkHints.activateMode"].group);
        assert!(by_name["reload"].background);
        assert!(by_name["mainFrame"].top_frame);
        assert!(by_name["copyCurrentUrl"].no_repeat);
        assert_eq!(Some(20), by_name["createTab"].repeat_limit);
        assert!(by_name["createTab"].options.get("(any url)").is_some());
    }

    #[test]
    fn default_key_mappings_include_legacy_bindings() {
        let registry = KeyMapRegistry::from_defaults();
        assert_eq!(68, registry.key_to_command.len());
        assert_eq!(
            Some("scrollDown"),
            registry.key_to_command.get("j").map(String::as_str)
        );
        assert_eq!(
            Some("scrollDown"),
            registry.key_to_command.get("<c-e>").map(String::as_str)
        );
        assert_eq!(
            Some("LinkHints.activateMode"),
            registry.key_to_command.get("f").map(String::as_str)
        );
        assert_eq!(
            Some("LinkHints.activateModeWithQueue"),
            registry.key_to_command.get("<a-f>").map(String::as_str)
        );
        assert_eq!(
            Some("zoomReset"),
            registry.key_to_command.get("z0").map(String::as_str)
        );
        assert_eq!(
            Some(true),
            registry
                .key_to_registry
                .get("R")
                .and_then(|entry| entry.options.get("hard"))
                .and_then(Value::as_bool)
        );
    }

    #[test]
    fn parse_key_sequence_normalizes_like_legacy_parser() {
        assert_eq!(vec!["a"], parse_key_sequence("a"));
        assert_eq!(vec!["A"], parse_key_sequence("A"));
        assert_eq!(vec!["<c-a>"], parse_key_sequence("<C-a>"));
        assert_eq!(vec!["<c-A>"], parse_key_sequence("<C-A>"));
        assert_eq!(vec!["<a-c-m-A>"], parse_key_sequence("<m-c-a-A>"));
        assert_eq!(vec!["<space>"], parse_key_sequence("<Space>"));
        assert_eq!(vec!["<", "<space>"], parse_key_sequence("<<space>"));
        assert_eq!(vec!["<", "a", ">"], parse_key_sequence("<a>"));
    }

    #[test]
    fn parse_map_unmap_unmapall_and_mapkey() {
        let registry = KeyMapRegistry::from_defaults();
        let parsed = registry.parse_key_mappings(
            r#"
            map a scrollDown
            map <C-Space> scrollUp count=5
            mapkey x y
            unmap a
            map b scrollToTop
            "#,
        );
        assert_eq!(Some(""), parsed.key_to_command.get("a").map(String::as_str));
        assert_eq!(Some(None), parsed.key_to_registry.get("a").cloned());
        assert_eq!(
            Some("scrollUp"),
            parsed.key_to_command.get("<c-space>").map(String::as_str)
        );
        assert_eq!(
            Some(5),
            parsed
                .key_to_registry
                .get("<c-space>")
                .and_then(Option::as_ref)
                .and_then(|entry| entry.options.get("count"))
                .and_then(Value::as_i64)
        );
        assert_eq!(
            Some("scrollToTop"),
            parsed.key_to_command.get("b").map(String::as_str)
        );
        assert_eq!(
            Some("y"),
            parsed.key_to_mapped_key.get("x").map(String::as_str)
        );
        assert!(parsed.validation_errors.is_empty());

        let parsed = registry.parse_key_mappings("mapkey a b\nunmapall\nmapkey b c");
        assert!(parsed.key_to_command.is_empty());
        assert!(parsed.key_to_registry.is_empty());
        assert_eq!(
            Some("c"),
            parsed.key_to_mapped_key.get("b").map(String::as_str)
        );
    }

    #[test]
    fn parse_command_options_supports_flags_values_and_urls() {
        let registry = KeyMapRegistry::from_defaults();
        let parsed = registry.parse_key_mappings(
            r#"
            map a reload hard
            map b Vomnibar.activate query="two words"
            map c createTab https://example.com/path
            "#,
        );
        assert!(parsed.validation_errors.is_empty());
        assert_eq!(
            Some(true),
            parsed
                .key_to_registry
                .get("a")
                .and_then(Option::as_ref)
                .and_then(|entry| entry.options.get("hard"))
                .and_then(Value::as_bool)
        );
        assert_eq!(
            Some("two words"),
            parsed
                .key_to_registry
                .get("b")
                .and_then(Option::as_ref)
                .and_then(|entry| entry.options.get("query"))
                .and_then(Value::as_str)
        );
        assert_eq!(
            Some(true),
            parsed
                .key_to_registry
                .get("c")
                .and_then(Option::as_ref)
                .and_then(|entry| entry.options.get("https://example.com/path"))
                .and_then(Value::as_bool)
        );
    }

    #[test]
    fn resolve_command_honors_unmap_and_prefixes() {
        let registry = KeyMapRegistry::from_defaults();
        let mappings = registry.parse_user_mappings("unmap j\nmap aa scrollDown");
        assert!(registry.resolve_command("j", &mappings).is_none());
        assert!(!registry.is_prefix("j", &mappings));
        assert!(registry.resolve_command("a", &mappings).is_none());
        assert!(registry.is_prefix("a", &mappings));
        assert_eq!(
            Some("scrollDown"),
            registry
                .resolve_command("aa", &mappings)
                .map(|(name, _)| name)
        );
    }
}
