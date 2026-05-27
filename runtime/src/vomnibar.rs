use crepuscularity_webext::wasm::{bookmarks, history, tabs};
use serde_json::{json, Value};
use std::collections::HashMap;

use crate::settings::UserSettings;

pub struct SearchEngines {
    pub google: String,
    pub custom: Vec<(String, String, String)>,
}

impl SearchEngines {
    pub fn from_settings(settings: &UserSettings) -> Self {
        let custom = settings
            .parse_search_engines()
            .into_iter()
            .map(|(k, (url, name))| (k, url, name))
            .collect();
        SearchEngines {
            google: settings.get_str("searchUrl"),
            custom,
        }
    }

    pub fn resolve(&self, query: &str) -> (String, String) {
        if let Some(colon_pos) = query.find(':') {
            let keyword = &query[..colon_pos];
            if let Some((_kw, url, _name)) = self.custom.iter().find(|(k, _, _)| k == keyword) {
                let search_term = &query[colon_pos + 1..];
                let resolved = url.replace("%s", &urlencode(search_term));
                return (resolved, query.to_string());
            }
        }
        let resolved = self.google.replace("%s", &urlencode(query));
        (resolved, query.to_string())
    }

    pub fn is_search_query(query: &str) -> bool {
        !query.contains('.') && !query.contains("://") && !query.contains(' ')
    }
}

fn urlencode(s: &str) -> String {
    s.bytes()
        .map(|b| {
            let ch = b as char;
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | '~') {
                ch.to_string()
            } else {
                format!("%{b:02X}")
            }
        })
        .collect()
}

#[derive(Debug, Clone)]
pub enum VomnibarMode {
    Full,
    Bookmarks,
    Tabs,
    Commands,
}

pub struct CompletionResult {
    pub items: Vec<CompletionItem>,
    pub prompt: String,
}

#[derive(Debug, Clone)]
pub struct CompletionItem {
    pub title: String,
    pub url: String,
    pub kind: String,
    pub relevance: f64,
    pub id: Option<i64>,
}

pub async fn query_vomnibar(query: &str, mode: VomnibarMode) -> Result<CompletionResult, String> {
    let mut items = Vec::new();

    match mode {
        VomnibarMode::Full => {
            if let Ok(bm) = query_bookmarks(query).await {
                items.extend(bm);
            }
            if let Ok(hist) = query_history(query).await {
                items.extend(hist);
            }
            if let Ok(tabs_items) = query_tabs(query).await {
                items.extend(tabs_items);
            }
        }
        VomnibarMode::Bookmarks => {
            items = query_bookmarks(query).await?;
        }
        VomnibarMode::Tabs => {
            items = query_tabs(query).await?;
        }
        VomnibarMode::Commands => {
            items = query_commands(query);
        }
    }

    items = scored_items(items, query);

    Ok(CompletionResult {
        items,
        prompt: query.to_string(),
    })
}

async fn query_bookmarks(query: &str) -> Result<Vec<CompletionItem>, String> {
    let results = bookmarks::get_recent(200)
        .await
        .map_err(|e| format!("bookmarks: {}", e))?;
    let flat = bookmarks::flatten_tree(&results);
    let query_lower = query.to_lowercase();
    let terms: Vec<&str> = if query_lower.is_empty() {
        vec![]
    } else {
        query_lower.split_whitespace().collect()
    };
    Ok(flat
        .into_iter()
        .filter_map(|node| {
            let url = node.url?;
            if !terms.is_empty() {
                let title_lower = node.title.to_lowercase();
                let url_lower = url.to_lowercase();
                if !terms.iter().any(|t| title_lower.contains(t) || url_lower.contains(t)) {
                    return None;
                }
            }
            Some(CompletionItem {
                title: node.title,
                url,
                kind: "bookmark".to_string(),
                relevance: 0.0,
                id: None,
            })
        })
        .collect())
}

async fn query_history(query: &str) -> Result<Vec<CompletionItem>, String> {
    let results = history::search(&history::HistorySearchQuery {
        text: query.to_string(),
        max_results: Some(200),
        ..Default::default()
    })
    .await
    .map_err(|e| format!("history: {}", e))?;
    Ok(results
        .into_iter()
        .filter_map(|item| {
            let url = item.url?;
            Some(CompletionItem {
                title: item.title.unwrap_or_default(),
                url,
                kind: "history".to_string(),
                relevance: 0.0,
                id: None,
            })
        })
        .collect())
}

async fn query_tabs(query: &str) -> Result<Vec<CompletionItem>, String> {
    let all = tabs::query(&tabs::QueryInfo {
        ..Default::default()
    })
    .await
    .map_err(|e| format!("tabs: {}", e))?;
    let query_lower = query.to_lowercase();
    let terms: Vec<&str> = if query_lower.is_empty() {
        vec![]
    } else {
        query_lower.split_whitespace().collect()
    };
    Ok(all
        .into_iter()
        .filter_map(|tab| {
            let url = tab.url?;
            if !terms.is_empty() {
                let title_lower = tab.title.as_deref().unwrap_or("").to_lowercase();
                let url_lower = url.to_lowercase();
                if !terms.iter().any(|t| title_lower.contains(t) || url_lower.contains(t)) {
                    return None;
                }
            }
            Some(CompletionItem {
                title: tab.title.unwrap_or_default(),
                url,
                kind: "tab".to_string(),
                relevance: 0.0,
                id: tab.id,
            })
        })
        .collect())
}

fn query_commands(query: &str) -> Vec<CompletionItem> {
    let commands = crate::commands::all_commands();
    let query_lower = query.to_lowercase();
    let terms: Vec<&str> = if query_lower.is_empty() {
        vec![]
    } else {
        query_lower.split_whitespace().collect()
    };
    commands
        .into_iter()
        .filter_map(|cmd| {
            if !terms.is_empty() {
                let name_lower = cmd.name.to_lowercase();
                let desc_lower = cmd.desc.to_lowercase();
                if !terms.iter().any(|t| name_lower.contains(t) || desc_lower.contains(t)) {
                    return None;
                }
            }
            Some(CompletionItem {
                title: cmd.desc,
                url: cmd.name,
                kind: "command".to_string(),
                relevance: 0.0,
                id: None,
            })
        })
        .collect()
}

pub fn scored_items(items: Vec<CompletionItem>, query: &str) -> Vec<CompletionItem> {
    let query_lower = query.to_lowercase();
    let terms: Vec<&str> = if query_lower.is_empty() {
        vec![]
    } else {
        query_lower.split_whitespace().collect()
    };

    let mut dedup: HashMap<String, CompletionItem> = HashMap::new();

    for mut item in items {
        let title_lower = item.title.to_lowercase();
        let url_lower = item.url.to_lowercase();
        let mut score = terms.iter().map(|term| {
            let mut s = 0.0;
            if title_lower.starts_with(term) {
                s += 2.0;
            } else if title_lower.contains(term) {
                s += 1.0;
            }
            if url_lower.starts_with(term) {
                s += 1.5;
            } else if url_lower.contains(term) {
                s += 0.5;
            }
            s
        }).sum::<f64>();

        if title_lower == query_lower {
            score += 3.0;
        }

        item.relevance = score;

        let url = item.url.clone();
        let should_insert = match dedup.get(&url) {
            Some(existing) => {
                item.relevance > existing.relevance
                    || (item.relevance == existing.relevance && item.kind == "tab")
            }
            None => true,
        };
        if should_insert {
            dedup.insert(url, item);
        }
    }

    let mut scored: Vec<CompletionItem> = dedup.into_values().collect();

    if !query_lower.is_empty() {
        scored.retain(|item| item.relevance > 0.0);
    }

    scored.sort_by(|a, b| {
        b.relevance
            .partial_cmp(&a.relevance)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    scored.into_iter().take(15).collect()
}

pub fn resolve_navigable(query: &str, engines: &SearchEngines) -> Value {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        return json!({"kind": "none"});
    }

    if trimmed.contains("://") {
        return json!({"kind": "url", "url": trimmed});
    }

    if let Some(colon_pos) = trimmed.find(':') {
        let keyword = &trimmed[..colon_pos];
        if engines.custom.iter().any(|(k, _, _)| k == keyword) {
            let (url, display) = engines.resolve(trimmed);
            return json!({"kind": "url", "url": url, "display": display});
        }
    }

    if SearchEngines::is_search_query(trimmed) {
        let (url, display) = engines.resolve(trimmed);
        return json!({"kind": "url", "url": url, "display": display});
    }

    let with_scheme = if !trimmed.starts_with("http") {
        format!("https://{}", trimmed)
    } else {
        trimmed.to_string()
    };

    json!({"kind": "url", "url": with_scheme})
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::UserSettings;

    fn make_settings(engines_text: &str) -> UserSettings {
        let mut s = UserSettings::new();
        s.merge(json!({"searchEngines": engines_text}));
        s
    }

    #[test]
    fn urlencode_keeps_alphanumeric_and_safe_chars() {
        assert_eq!(urlencode("hello"), "hello");
        assert_eq!(urlencode("test-1_2.3~"), "test-1_2.3~");
    }

    #[test]
    fn urlencode_encodes_spaces_and_special_chars() {
        assert_eq!(urlencode("hello world"), "hello%20world");
        assert_eq!(urlencode("a/b=c"), "a%2Fb%3Dc");
        assert_eq!(urlencode("русский"), "%D1%80%D1%83%D1%81%D1%81%D0%BA%D0%B8%D0%B9");
    }

    #[test]
    fn search_engine_resolve_uses_google_by_default() {
        let engines = SearchEngines {
            google: "https://google.com/search?q=%s".to_string(),
            custom: vec![],
        };
        assert_eq!(
            engines.resolve("hello world"),
            ("https://google.com/search?q=hello%20world".to_string(), "hello world".to_string())
        );
    }

    #[test]
    fn search_engine_resolve_uses_custom_keyword() {
        let engines = SearchEngines {
            google: "https://google.com/search?q=%s".to_string(),
            custom: vec![
                ("w".to_string(), "https://en.wikipedia.org/wiki/%s".to_string(), "Wikipedia".to_string()),
            ],
        };
        assert_eq!(
            engines.resolve("w:rust"),
            ("https://en.wikipedia.org/wiki/rust".to_string(), "w:rust".to_string())
        );
    }

    #[test]
    fn search_engine_resolve_falls_back_when_keyword_not_found() {
        let engines = SearchEngines {
            google: "https://google.com/search?q=%s".to_string(),
            custom: vec![("w".to_string(), "https://wiki/%s".to_string(), "Wiki".to_string())],
        };
        let (url, _) = engines.resolve("x:test");
        assert!(url.contains("google.com"));
    }

    #[test]
    fn is_search_query_rejects_urls_with_dots() {
        assert!(SearchEngines::is_search_query("cargo"));
        assert!(SearchEngines::is_search_query("rust-wasm"));
        assert!(!SearchEngines::is_search_query("example.com"));
        assert!(!SearchEngines::is_search_query("https://x"));
        assert!(!SearchEngines::is_search_query("hello world"));
    }

    #[test]
    fn scored_items_ranks_by_title_and_url_match() {
        let items = vec![
            CompletionItem { title: "Rust Book".into(), url: "https://doc.rust-lang.org/book".into(), kind: "history".into(), relevance: 0.0, id: None },
            CompletionItem { title: "Rust By Example".into(), url: "https://doc.rust-lang.org/rust-by-example".into(), kind: "history".into(), relevance: 0.0, id: None },
            CompletionItem { title: "Python Docs".into(), url: "https://docs.python.org".into(), kind: "history".into(), relevance: 0.0, id: None },
        ];
        let scored = scored_items(items, "rust");
        assert!(!scored.is_empty());
        assert!(scored.iter().all(|item| item.relevance > 0.0));
        assert_eq!(scored.len(), 2);
        let titles: Vec<&str> = scored.iter().map(|i| i.title.as_str()).collect();
        assert!(titles.contains(&"Rust Book"));
        assert!(titles.contains(&"Rust By Example"));
        assert!(scored[0].relevance >= scored[1].relevance);
    }

    #[test]
    fn scored_items_deduplicates_by_url_keeping_higher_scored() {
        let items = vec![
            CompletionItem { title: "Site".into(), url: "https://docs.rs".into(), kind: "history".into(), relevance: 0.0, id: None },
            CompletionItem { title: "Rust Docs".into(), url: "https://docs.rs".into(), kind: "bookmark".into(), relevance: 0.0, id: None },
        ];
        let scored = scored_items(items, "rust");
        assert_eq!(scored.len(), 1);
        assert_eq!(scored[0].title, "Rust Docs");
    }

    #[test]
    fn scored_items_keeps_tab_on_dedup_tie() {
        let items = vec![
            CompletionItem { title: "Tab Page".into(), url: "https://example.com".into(), kind: "tab".into(), relevance: 0.0, id: Some(5) },
            CompletionItem { title: "History Page".into(), url: "https://example.com".into(), kind: "history".into(), relevance: 0.0, id: None },
        ];
        let scored = scored_items(items, "page");
        assert_eq!(scored.len(), 1);
        assert_eq!(scored[0].kind, "tab");
    }

    #[test]
    fn scored_items_limits_to_15() {
        let items = (0..30)
            .map(|i| CompletionItem {
                title: format!("Page {}", i),
                url: format!("https://example.com/{}", i),
                kind: "history".into(),
                relevance: (30 - i) as f64,
                id: None,
            })
            .collect();
        let scored = scored_items(items, "");
        assert_eq!(scored.len(), 15);
    }

    #[test]
    fn resolve_navigable_empty_query_is_none() {
        let engines = SearchEngines {
            google: "https://google.com/search?q=%s".into(),
            custom: vec![],
        };
        assert_eq!(resolve_navigable("", &engines), json!({"kind": "none"}));
    }

    #[test]
    fn resolve_navigable_with_scheme_is_direct_url() {
        let engines = SearchEngines {
            google: "https://google.com/search?q=%s".into(),
            custom: vec![],
        };
        let r = resolve_navigable("https://example.com/path", &engines);
        assert_eq!(r["kind"], "url");
        assert_eq!(r["url"], "https://example.com/path");
    }

    #[test]
    fn resolve_navigable_adds_https_for_url_like_queries() {
        let engines = SearchEngines {
            google: "https://google.com/search?q=%s".into(),
            custom: vec![],
        };
        let r = resolve_navigable("example.com/path", &engines);
        assert_eq!(r["kind"], "url");
        assert_eq!(r["url"], "https://example.com/path");
    }

    #[test]
    fn resolve_navigable_uses_custom_engine_keyword() {
        let engines = SearchEngines {
            google: "https://google.com/search?q=%s".into(),
            custom: vec![("gh".into(), "https://github.com/search?q=%s".into(), "GitHub".into())],
        };
        let r = resolve_navigable("gh:rust", &engines);
        assert_eq!(r["kind"], "url");
        assert_eq!(r["url"], "https://github.com/search?q=rust");
    }

    #[test]
    fn resolve_navigable_searches_single_words() {
        let engines = SearchEngines {
            google: "https://google.com/search?q=%s".into(),
            custom: vec![],
        };
        let r = resolve_navigable("cargo", &engines);
        assert_eq!(r["kind"], "url");
        assert!(r["url"].as_str().unwrap().starts_with("https://google.com/search?q="));
    }

    #[test]
    fn search_engines_from_settings_parses_custom_engines() {
        let s = make_settings("w: https://en.wikipedia.org/w/index.php?search=%s Wikipedia\ngh: https://github.com/search?q=%s GitHub");
        let engines = SearchEngines::from_settings(&s);
        assert_eq!(engines.custom.len(), 2);
        assert!(engines.custom.iter().any(|(k, _, _)| k == "w"));
        assert!(engines.custom.iter().any(|(k, _, _)| k == "gh"));
    }
}

