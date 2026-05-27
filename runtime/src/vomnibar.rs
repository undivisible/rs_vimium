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
            match query_bookmarks(query).await {
                Ok(bm) => items.extend(bm),
                Err(_) => {}
            }
            match query_history(query).await {
                Ok(hist) => items.extend(hist),
                Err(_) => {}
            }
            match query_tabs(query).await {
                Ok(tabs_items) => items.extend(tabs_items),
                Err(_) => {}
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
            Some(CompletionItem {
                title: item.title.unwrap_or_default(),
                url: item.url?,
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
