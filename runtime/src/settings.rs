use serde_json::{json, Value};
use std::collections::HashMap;

pub const VIMIUM_NEW_TAB_URL: &str = "pages/new-tab.html";

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NewTabDestination {
    BrowserNewTabPage,
    VimiumNewTabPage,
    CustomUrl,
}

impl NewTabDestination {
    pub fn as_str(&self) -> &'static str {
        match self {
            NewTabDestination::BrowserNewTabPage => "browserNewTabPage",
            NewTabDestination::VimiumNewTabPage => "vimiumNewTabPage",
            NewTabDestination::CustomUrl => "customUrl",
        }
    }

    pub fn from_setting(s: &str) -> Self {
        match s {
            "browserNewTabPage" => NewTabDestination::BrowserNewTabPage,
            "customUrl" => NewTabDestination::CustomUrl,
            _ => NewTabDestination::VimiumNewTabPage,
        }
    }
}

pub fn default_settings() -> Value {
    json!({
        "enabled": true,
        "useCustomNewTab": true,
        "scrollStepSize": 60,
        "smoothScroll": true,
        "keyMappings": "# Insert your preferred key mappings here.",
        "linkHintCharacters": "sadfjklewcmpgh",
        "linkHintNumbers": "0123456789",
        "filterLinkHints": false,
        "hideHud": false,
        "hideUpdateNotifications": false,
        "userDefinedLinkHintCss": "div > .vimiumHintMarker {\nbackground: -webkit-gradient(linear, left top, left bottom, color-stop(0%,#FFF785), color-stop(100%,#FFC542));\nborder: 1px solid #E3BE23;\n}\n\ndiv > .vimiumHintMarker span {\ncolor: black;\nfont-weight: bold;\nfont-size: 12px;\n}\n\ndiv > .vimiumHintMarker > .matchingCharacter {\n}",
        "exclusionRules": [
            { "passKeys": "", "pattern": "https?://mail.google.com/*" }
        ],
        "previousPatterns": "prev,previous,back,older,<,\u{2039},\u{2190},\u{00ab},\u{226a},<<",
        "nextPatterns": "next,more,newer,>,\u{203a},\u{2192},\u{00bb},\u{226b},>>",
        "searchUrl": "https://www.google.com/search?q=",
        "searchEngines": "w: https://www.wikipedia.org/w/index.php?title=Special:Search&search=%s Wikipedia\n\n# More examples.\ng: https://www.google.com/search?q=%s Google\nl: https://www.google.com/search?q=%s&btnI I'm feeling lucky...\ny: https://www.youtube.com/results?search_query=%s Youtube\ngm: https://www.google.com/maps?q=%s Google maps\nd: https://duckduckgo.com/?q=%s DuckDuckGo\n",
        "newTabDestination": "vimiumNewTabPage",
        "newTabCustomUrl": "",
        "openVomnibarOnNewTabPage": true,
        "grabBackFocus": false,
        "regexFindMode": false,
        "waitForEnterForFilteredHints": true,
        "helpDialog_showAdvancedCommands": false,
        "ignoreKeyboardLayout": false
    })
}

pub struct UserSettings {
    pub settings: Value,
    pub defaults: Value,
}

impl Default for UserSettings {
    fn default() -> Self {
        Self::new()
    }
}

impl UserSettings {
    pub fn new() -> Self {
        UserSettings {
            settings: json!({}),
            defaults: default_settings(),
        }
    }

    pub fn merge(&mut self, stored: Value) {
        let defaults = default_settings();
        let mut merged = defaults.clone();
        let stored = migrate_settings(stored);
        if let Value::Object(map) = &stored {
            for (k, v) in map {
                merged[k] = v.clone();
            }
        }
        self.settings = merged;
    }

    pub fn get_bool(&self, key: &str) -> bool {
        self.settings
            .get(key)
            .and_then(Value::as_bool)
            .unwrap_or_else(|| {
                self.defaults
                    .get(key)
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
            })
    }

    pub fn get_str(&self, key: &str) -> String {
        self.settings
            .get(key)
            .and_then(Value::as_str)
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| {
                self.defaults
                    .get(key)
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string()
            })
    }

    pub fn get_int(&self, key: &str) -> i64 {
        self.settings
            .get(key)
            .and_then(Value::as_i64)
            .unwrap_or_else(|| self.defaults.get(key).and_then(Value::as_i64).unwrap_or(0))
    }

    pub fn get_float(&self, key: &str) -> f64 {
        self.settings
            .get(key)
            .and_then(Value::as_f64)
            .unwrap_or_else(|| {
                self.defaults
                    .get(key)
                    .and_then(Value::as_f64)
                    .unwrap_or(0.0)
            })
    }

    pub fn new_tab_destination(&self) -> NewTabDestination {
        NewTabDestination::from_setting(&self.get_str("newTabDestination"))
    }

    pub fn new_tab_url(&self) -> String {
        match self.new_tab_destination() {
            NewTabDestination::BrowserNewTabPage => String::new(),
            NewTabDestination::VimiumNewTabPage => VIMIUM_NEW_TAB_URL.to_string(),
            NewTabDestination::CustomUrl => {
                let url = self.get_str("newTabCustomUrl");
                if url.is_empty() {
                    VIMIUM_NEW_TAB_URL.to_string()
                } else {
                    url
                }
            }
        }
    }

    pub fn parse_search_engines(&self) -> HashMap<String, (String, String)> {
        let engines_text = self.get_str("searchEngines");
        let mut engines = HashMap::new();
        for line in engines_text.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            if let Some(colon_pos) = trimmed.find(':') {
                let keyword = trimmed[..colon_pos].trim().to_string();
                let rest = trimmed[colon_pos + 1..].trim();
                let parts: Vec<&str> = rest.rsplitn(2, ' ').collect();
                if parts.len() == 2 {
                    let url = parts[1].trim();
                    let name = parts[0].trim();
                    engines.insert(keyword, (url.to_string(), name.to_string()));
                }
            }
        }
        engines
    }

    pub fn parse_exclusion_rules(&self) -> Vec<ExclusionRule> {
        let rules = self.settings.get("exclusionRules");
        match rules {
            Some(Value::Array(arr)) => arr
                .iter()
                .filter_map(|r| {
                    Some(ExclusionRule {
                        pattern: r.get("pattern")?.as_str()?.to_string(),
                        pass_keys: r.get("passKeys")?.as_str().unwrap_or("").to_string(),
                    })
                })
                .collect(),
            _ => vec![],
        }
    }

    pub fn enabled_state_for_url(&self, url: &str) -> ExclusionState {
        enabled_state_for_url(url, &self.parse_exclusion_rules())
    }
}

#[derive(Debug, Clone)]
pub struct ExclusionRule {
    pub pattern: String,
    pub pass_keys: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExclusionState {
    pub is_enabled_for_url: bool,
    pub pass_keys: String,
}

pub fn enabled_state_for_url(url: &str, rules: &[ExclusionRule]) -> ExclusionState {
    let matching_rules = rules
        .iter()
        .filter(|rule| !rule.pattern.is_empty() && exclusion_pattern_matches(&rule.pattern, url))
        .collect::<Vec<_>>();
    for rule in &matching_rules {
        if rule.pass_keys.is_empty() {
            return ExclusionState {
                is_enabled_for_url: false,
                pass_keys: String::new(),
            };
        }
    }
    if matching_rules.is_empty() {
        return ExclusionState {
            is_enabled_for_url: true,
            pass_keys: String::new(),
        };
    }
    let pass_keys = distinct_characters(
        &matching_rules
            .iter()
            .flat_map(|rule| rule.pass_keys.split_whitespace())
            .collect::<String>(),
    );
    ExclusionState {
        is_enabled_for_url: !pass_keys.is_empty(),
        pass_keys,
    }
}

fn distinct_characters(text: &str) -> String {
    let mut seen = Vec::new();
    let mut result = String::new();
    for ch in text.chars() {
        if !seen.contains(&ch) {
            seen.push(ch);
            result.push(ch);
        }
    }
    result
}

fn exclusion_pattern_matches(pattern: &str, url: &str) -> bool {
    #[derive(Clone, Copy)]
    enum Token {
        Literal(char),
        Optional(char),
        Any,
    }

    let chars = pattern.chars().collect::<Vec<_>>();
    let mut tokens = Vec::new();
    let mut index = 0;
    while index < chars.len() {
        let ch = chars[index];
        if ch == '*' {
            tokens.push(Token::Any);
            index += 1;
        } else if chars.get(index + 1) == Some(&'?') {
            tokens.push(Token::Optional(ch));
            index += 2;
        } else if ch == '?' {
            return false;
        } else {
            tokens.push(Token::Literal(ch));
            index += 1;
        }
    }

    let url_chars = url.chars().collect::<Vec<_>>();
    let mut current = vec![false; url_chars.len() + 1];
    current[0] = true;
    for token in tokens {
        let mut next = vec![false; url_chars.len() + 1];
        match token {
            Token::Literal(expected) => {
                for pos in 0..url_chars.len() {
                    if current[pos] && url_chars[pos] == expected {
                        next[pos + 1] = true;
                    }
                }
            }
            Token::Optional(expected) => {
                for pos in 0..=url_chars.len() {
                    if current[pos] {
                        next[pos] = true;
                        if pos < url_chars.len() && url_chars[pos] == expected {
                            next[pos + 1] = true;
                        }
                    }
                }
            }
            Token::Any => {
                let mut reachable = false;
                for pos in 0..=url_chars.len() {
                    reachable |= current[pos];
                    next[pos] = reachable;
                }
            }
        }
        current = next;
    }
    current[url_chars.len()]
}

pub fn prune_defaults(settings: &Value) -> Value {
    let defaults = default_settings();
    match settings {
        Value::Object(map) => {
            let mut pruned = serde_json::Map::new();
            for (k, v) in map {
                if let Some(dv) = defaults.get(k) {
                    if v != dv {
                        pruned.insert(k.clone(), v.clone());
                    }
                } else {
                    pruned.insert(k.clone(), v.clone());
                }
            }
            Value::Object(pruned)
        }
        _ => settings.clone(),
    }
}

pub fn migrate_settings(settings: Value) -> Value {
    let settings = migrate_pre_2_0(settings);
    let settings = migrate_pre_2_4(settings);
    migrate_pre_2_4_1(settings)
}

fn migrate_pre_2_0(settings: Value) -> Value {
    let Value::Object(map) = settings else {
        return settings;
    };
    if map.contains_key("settingsVersion") {
        return Value::Object(map);
    }
    let mut migrated = serde_json::Map::new();
    for (key, value) in map {
        if key == "passNextKeyKeys" {
            continue;
        }
        let decoded = match value {
            Value::String(raw) => serde_json::from_str(&raw).unwrap_or(Value::String(raw)),
            other => other,
        };
        migrated.insert(key, decoded);
    }
    Value::Object(migrated)
}

fn migrate_pre_2_4(settings: Value) -> Value {
    let Value::Object(mut map) = settings else {
        return settings;
    };
    let should_migrate = map
        .get("settingsVersion")
        .and_then(Value::as_str)
        .is_some_and(|version| compare_versions(version, "2.4") < 0);
    if !should_migrate {
        return Value::Object(map);
    }

    let new_tab_url = map
        .get("newTabUrl")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);
    match new_tab_url.as_deref() {
        None | Some("about:newtab") => {
            map.insert(
                "newTabDestination".to_string(),
                Value::String(NewTabDestination::BrowserNewTabPage.as_str().to_string()),
            );
        }
        Some("pages/blank.html") => {
            map.insert(
                "newTabDestination".to_string(),
                Value::String(NewTabDestination::VimiumNewTabPage.as_str().to_string()),
            );
        }
        Some(url) => {
            map.insert(
                "newTabDestination".to_string(),
                Value::String(NewTabDestination::CustomUrl.as_str().to_string()),
            );
            map.insert(
                "newTabCustomUrl".to_string(),
                Value::String(url.to_string()),
            );
        }
    }
    map.remove("newTabUrl");
    Value::Object(map)
}

fn migrate_pre_2_4_1(settings: Value) -> Value {
    let Value::Object(mut map) = settings else {
        return settings;
    };
    let should_migrate = map
        .get("settingsVersion")
        .and_then(Value::as_str)
        .is_some_and(|version| {
            compare_versions(version, "2.4") >= 0 && compare_versions(version, "2.4.1") < 0
        });
    if should_migrate {
        let destination = map.get("newTabDestination").and_then(Value::as_str);
        if destination.is_none()
            || destination == Some(NewTabDestination::VimiumNewTabPage.as_str())
        {
            map.insert(
                "newTabDestination".to_string(),
                Value::String(NewTabDestination::BrowserNewTabPage.as_str().to_string()),
            );
        }
    }
    Value::Object(map)
}

fn compare_versions(a: &str, b: &str) -> i8 {
    let parse = |version: &str| {
        version
            .split('.')
            .map(|part| part.parse::<i64>().unwrap_or(0))
            .collect::<Vec<_>>()
    };
    let left = parse(a);
    let right = parse(b);
    let width = left.len().max(right.len());
    for index in 0..width {
        let l = left.get(index).copied().unwrap_or(0);
        let r = right.get(index).copied().unwrap_or(0);
        if l < r {
            return -1;
        }
        if l > r {
            return 1;
        }
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_legacy_basics() {
        let defaults = default_settings();
        assert_eq!(
            Some(60),
            defaults.get("scrollStepSize").and_then(Value::as_i64)
        );
        assert_eq!(
            Some(true),
            defaults.get("smoothScroll").and_then(Value::as_bool)
        );
        assert_eq!(
            Some("sadfjklewcmpgh"),
            defaults.get("linkHintCharacters").and_then(Value::as_str)
        );
        assert_eq!(
            Some("vimiumNewTabPage"),
            defaults.get("newTabDestination").and_then(Value::as_str)
        );
        assert_eq!(
            Some(false),
            defaults
                .get("ignoreKeyboardLayout")
                .and_then(Value::as_bool)
        );
    }

    #[test]
    fn merge_decodes_pre_2_0_json_values_and_removes_session_key() {
        let mut settings = UserSettings::new();
        settings.merge(json!({
            "scrollStepSize": "123",
            "smoothScroll": "false",
            "passNextKeyKeys": ["<c-x>"]
        }));
        assert_eq!(123, settings.get_int("scrollStepSize"));
        assert!(!settings.get_bool("smoothScroll"));
        assert!(settings.settings.get("passNextKeyKeys").is_none());
    }

    #[test]
    fn migrate_pre_2_4_new_tab_url() {
        let migrated = migrate_settings(json!({
            "settingsVersion": "2.3",
            "newTabUrl": "https://example.com"
        }));
        assert_eq!(
            Some("customUrl"),
            migrated.get("newTabDestination").and_then(Value::as_str)
        );
        assert_eq!(
            Some("https://example.com"),
            migrated.get("newTabCustomUrl").and_then(Value::as_str)
        );
        assert!(migrated.get("newTabUrl").is_none());

        let migrated = migrate_settings(json!({
            "settingsVersion": "2.3",
            "newTabUrl": "pages/blank.html"
        }));
        assert_eq!(
            Some("vimiumNewTabPage"),
            migrated.get("newTabDestination").and_then(Value::as_str)
        );
    }

    #[test]
    fn migrate_pre_2_4_1_restores_browser_new_tab_default() {
        let migrated = migrate_settings(json!({
            "settingsVersion": "2.4.0"
        }));
        assert_eq!(
            Some("browserNewTabPage"),
            migrated.get("newTabDestination").and_then(Value::as_str)
        );

        let migrated = migrate_settings(json!({
            "settingsVersion": "2.4.0",
            "newTabDestination": "customUrl"
        }));
        assert_eq!(
            Some("customUrl"),
            migrated.get("newTabDestination").and_then(Value::as_str)
        );
    }

    #[test]
    fn prune_defaults_keeps_only_non_defaults_and_unknown_keys() {
        let pruned = prune_defaults(&json!({
            "scrollStepSize": 60,
            "smoothScroll": false,
            "unknown": "kept"
        }));
        assert!(pruned.get("scrollStepSize").is_none());
        assert_eq!(
            Some(false),
            pruned.get("smoothScroll").and_then(Value::as_bool)
        );
        assert_eq!(Some("kept"), pruned.get("unknown").and_then(Value::as_str));
    }

    #[test]
    fn new_tab_url_resolves_destinations() {
        let mut settings = UserSettings::new();
        assert_eq!("pages/new-tab.html", settings.new_tab_url());

        settings.merge(json!({
            "newTabDestination": "browserNewTabPage"
        }));
        assert_eq!("", settings.new_tab_url());

        settings.merge(json!({
            "newTabDestination": "customUrl",
            "newTabCustomUrl": "https://example.com"
        }));
        assert_eq!("https://example.com", settings.new_tab_url());
    }

    #[test]
    fn exclusion_rules_disable_or_pass_distinct_keys() {
        let rules = vec![
            ExclusionRule {
                pattern: "http*://mail.google.com/*".to_string(),
                pass_keys: String::new(),
            },
            ExclusionRule {
                pattern: "http*://www.facebook.com/*".to_string(),
                pass_keys: "abab".to_string(),
            },
            ExclusionRule {
                pattern: "http*://www.facebook.com/*".to_string(),
                pass_keys: "cdcd".to_string(),
            },
            ExclusionRule {
                pattern: "http*://www.example.com/*".to_string(),
                pass_keys: "a bb c bba a".to_string(),
            },
        ];

        assert_eq!(
            ExclusionState {
                is_enabled_for_url: false,
                pass_keys: String::new()
            },
            enabled_state_for_url("https://mail.google.com/mail/u/0", &rules)
        );
        assert_eq!(
            ExclusionState {
                is_enabled_for_url: true,
                pass_keys: "abcd".to_string()
            },
            enabled_state_for_url("https://www.facebook.com/something", &rules)
        );
        assert_eq!(
            ExclusionState {
                is_enabled_for_url: true,
                pass_keys: "abc".to_string()
            },
            enabled_state_for_url("http://www.example.com/pages", &rules)
        );
        assert_eq!(
            ExclusionState {
                is_enabled_for_url: true,
                pass_keys: String::new()
            },
            enabled_state_for_url("http://www.twitter.com/pages", &rules)
        );
    }

    #[test]
    fn malformed_exclusion_patterns_do_not_disable_pages() {
        let rules = vec![ExclusionRule {
            pattern: "http*://www.bad-regexp.com/*[a-".to_string(),
            pass_keys: String::new(),
        }];
        assert_eq!(
            ExclusionState {
                is_enabled_for_url: true,
                pass_keys: String::new()
            },
            enabled_state_for_url("http://www.bad-regexp.com/pages", &rules)
        );
    }
}
