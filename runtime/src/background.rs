use crepuscularity_webext::wasm::{runtime as browser_runtime, storage, tabs, windows};
use serde_json::{json, Value};

use crate::settings::{NewTabDestination, UserSettings};

pub async fn active_tab() -> Result<Option<tabs::Tab>, String> {
    let tabs_list = tabs::query(&tabs::QueryInfo {
        active: Some(true),
        current_window: Some(true),
        ..Default::default()
    })
    .await
    .map_err(|e| format!("query tabs: {}", e))?;
    Ok(tabs_list.into_iter().next())
}

pub async fn execute_background_command(command: &str, _args: &Value) -> Result<(), String> {
    match command {
        "create-tab" => {
            let settings = load_settings().await?;
            let options = command_options(_args);
            let option_url = first_url_option(options);
            let url = if let Some(url) = option_url {
                Some(resolve_new_tab_url(&url))
            } else {
                match settings.new_tab_destination() {
                    NewTabDestination::BrowserNewTabPage => None,
                    _ => Some(resolve_new_tab_url(&settings.new_tab_url())),
                }
            };
            if option_bool(options, "window") || option_bool(options, "incognito") {
                let _ = windows::create(&windows::CreateData {
                    url: url.clone().map(Value::String),
                    focused: Some(true),
                    incognito: option_bool(options, "incognito").then_some(true),
                    ..Default::default()
                })
                .await
                .map_err(|e| e.to_string())?;
                return Ok(());
            }
            let index = create_tab_index(options).await?;
            let count = command_count(_args).min(20);
            for offset in 0..count {
                tabs::create(&tabs::CreateProperties {
                    url: url.clone(),
                    index: index.map(|index| index + offset),
                    ..Default::default()
                })
                .await
                .map_err(|e| e.to_string())?;
            }
        }
        "open-url" => {
            let url = _args
                .get("url")
                .and_then(Value::as_str)
                .unwrap_or("about:blank");
            tabs::create(&tabs::CreateProperties {
                url: Some(url.to_string()),
                active: Some(_args.get("active").and_then(Value::as_bool).unwrap_or(true)),
                ..Default::default()
            })
            .await
            .map_err(|e| e.to_string())?;
        }
        "previous-tab" => activate_relative_tab(-command_count(_args)).await?,
        "next-tab" => activate_relative_tab(command_count(_args)).await?,
        "visit-previous-tab" => {
            let storage_data = storage::session()
                .get_json(json!({"previousTabIds": []}))
                .await
                .map_err(|e| format!("get session: {}", e))?;
            if let Some(ids) = storage_data.get("previousTabIds").and_then(Value::as_array) {
                if let Some(id) = ids.first().and_then(Value::as_i64) {
                    tabs::update(
                        id,
                        &tabs::UpdateProperties {
                            active: Some(true),
                            ..Default::default()
                        },
                    )
                    .await
                    .map_err(|e| e.to_string())?;
                }
            }
        }
        "first-tab" => activate_edge_tab(false).await?,
        "last-tab" => activate_edge_tab(true).await?,
        "duplicate-tab" => {
            for _ in 0..command_count(_args).min(20) {
                if let Some(tab) = active_tab().await? {
                    if let Some(id) = tab.id {
                        tabs::duplicate(id).await.map_err(|e| e.to_string())?;
                    }
                }
            }
        }
        "toggle-pin" => {
            if let Some(tab) = active_tab().await? {
                if let Some(id) = tab.id {
                    let pinned = !tab.pinned.unwrap_or(false);
                    tabs::update(
                        id,
                        &tabs::UpdateProperties {
                            pinned: Some(pinned),
                            ..Default::default()
                        },
                    )
                    .await
                    .map_err(|e| e.to_string())?;
                }
            }
        }
        "toggle-mute" => {
            if let Some(tab) = active_tab().await? {
                if let Some(id) = tab.id {
                    let session = storage::session();
                    let stored = session
                        .get_json(json!({"mutedTabIds": []}))
                        .await
                        .map_err(|e| format!("get session: {}", e))?;
                    let mut muted_ids: Vec<i64> = stored
                        .get("mutedTabIds")
                        .and_then(Value::as_array)
                        .map(|a| a.iter().filter_map(Value::as_i64).collect())
                        .unwrap_or_default();
                    let currently_muted = muted_ids.contains(&id);
                    tabs::update(
                        id,
                        &tabs::UpdateProperties {
                            muted: Some(!currently_muted),
                            ..Default::default()
                        },
                    )
                    .await
                    .map_err(|e| e.to_string())?;
                    if currently_muted {
                        muted_ids.retain(|m| *m != id);
                    } else {
                        muted_ids.push(id);
                    }
                    session
                        .set(&json!({"mutedTabIds": muted_ids}))
                        .await
                        .map_err(|e| format!("save muted: {}", e))?;
                }
            }
        }
        "remove-tab" => {
            for _ in 0..command_count(_args).min(25) {
                if let Some(tab) = active_tab().await? {
                    if let Some(id) = tab.id {
                        if let Some(url) = &tab.url {
                            storage::sync()
                                .set(&json!({ "lastClosedTabUrl": url }))
                                .await
                                .map_err(|e| format!("save url: {}", e))?;
                        }
                        tabs::remove(id).await.map_err(|e| e.to_string())?;
                    }
                }
            }
        }
        "restore-tab" => {
            let saved = storage::sync()
                .get_json(json!({"lastClosedTabUrl": ""}))
                .await
                .map_err(|e| format!("get url: {}", e))?;
            if let Some(url) = saved.get("lastClosedTabUrl").and_then(Value::as_str) {
                if !url.is_empty() {
                    for _ in 0..command_count(_args).min(20) {
                        tabs::create(&tabs::CreateProperties {
                            url: Some(url.to_string()),
                            active: Some(true),
                            ..Default::default()
                        })
                        .await
                        .map_err(|e| e.to_string())?;
                    }
                }
            }
        }
        "move-to-new-window" => {
            if let Some(tab) = active_tab().await? {
                if let Some(id) = tab.id {
                    let win = windows::create(&windows::CreateData {
                        tab_id: Some(id),
                        ..Default::default()
                    })
                    .await
                    .map_err(|e| e.to_string())?;
                    let _ = win;
                }
            }
        }
        "close-tabs-left" => {
            let all = query_current_window().await?;
            let active_idx = all
                .iter()
                .find(|t| t.active.unwrap_or(false))
                .and_then(|t| t.index)
                .unwrap_or(0);
            for tab in &all {
                if let (Some(idx), Some(id)) = (tab.index, tab.id) {
                    if idx < active_idx && !tab.pinned.unwrap_or(false) {
                        let _ = tabs::remove(id).await;
                    }
                }
            }
        }
        "close-tabs-right" => {
            let all = query_current_window().await?;
            let active_idx = all
                .iter()
                .find(|t| t.active.unwrap_or(false))
                .and_then(|t| t.index)
                .unwrap_or(0);
            for tab in &all {
                if let (Some(idx), Some(id)) = (tab.index, tab.id) {
                    if idx > active_idx && !tab.pinned.unwrap_or(false) {
                        let _ = tabs::remove(id).await;
                    }
                }
            }
        }
        "close-other-tabs" => {
            let all = query_current_window().await?;
            let active_id = all
                .iter()
                .find(|t| t.active.unwrap_or(false))
                .and_then(|t| t.id);
            for tab in &all {
                if let Some(id) = tab.id {
                    if Some(id) != active_id && !tab.pinned.unwrap_or(false) {
                        let _ = tabs::remove(id).await;
                    }
                }
            }
        }
        "move-tab-left" => {
            if let Some(tab) = active_tab().await? {
                if let (Some(id), Some(idx)) = (tab.id, tab.index) {
                    if idx > 0 {
                        tabs::move_tab(id, idx - 1)
                            .await
                            .map_err(|e| e.to_string())?;
                    }
                }
            }
        }
        "move-tab-right" => {
            if let Some(tab) = active_tab().await? {
                if let (Some(id), Some(idx)) = (tab.id, tab.index) {
                    tabs::move_tab(id, idx + 1)
                        .await
                        .map_err(|e| e.to_string())?;
                }
            }
        }
        "zoom-in" => {
            if let Some(tab) = active_tab().await? {
                if let Some(id) = tab.id {
                    let current = tabs::get_zoom(id).await.unwrap_or(1.0);
                    tabs::set_zoom(id, (current + 0.25 * command_count(_args) as f64).min(5.0))
                        .await
                        .map_err(|e| e.to_string())?;
                }
            }
        }
        "zoom-out" => {
            if let Some(tab) = active_tab().await? {
                if let Some(id) = tab.id {
                    let current = tabs::get_zoom(id).await.unwrap_or(1.0);
                    tabs::set_zoom(id, (current - 0.25 * command_count(_args) as f64).max(0.25))
                        .await
                        .map_err(|e| e.to_string())?;
                }
            }
        }
        "zoom-reset" => {
            if let Some(tab) = active_tab().await? {
                if let Some(id) = tab.id {
                    tabs::set_zoom(id, 1.0).await.map_err(|e| e.to_string())?;
                }
            }
        }
        "set-zoom" => {
            if let Some(tab) = active_tab().await? {
                if let Some(id) = tab.id {
                    if let Some(level) = command_options(_args)
                        .get("level")
                        .and_then(|value| value.as_f64().or_else(|| value.as_str()?.parse().ok()))
                    {
                        tabs::set_zoom(id, level.clamp(0.25, 5.0))
                            .await
                            .map_err(|e| e.to_string())?;
                    }
                }
            }
        }
        "reload" => {
            if let Some(tab) = active_tab().await? {
                if let Some(id) = tab.id {
                    let _hard = command_arg(_args, "hard")
                        .and_then(Value::as_bool)
                        .unwrap_or(false);
                    tabs::reload(id).await.map_err(|e| e.to_string())?;
                }
            }
        }
        _ => return Err(format!("unknown background command: {}", command)),
    }
    Ok(())
}

fn resolve_new_tab_url(url: &str) -> String {
    if url.contains("://") || url.starts_with("about:") {
        return url.to_string();
    }
    browser_runtime::get_url(url).unwrap_or_else(|_| url.to_string())
}

fn command_options(args: &Value) -> &Value {
    args.get("options")
        .or_else(|| args.get("args").and_then(|args| args.get("options")))
        .unwrap_or(&Value::Null)
}

fn command_arg<'a>(args: &'a Value, key: &str) -> Option<&'a Value> {
    args.get(key)
        .or_else(|| args.get("args").and_then(|args| args.get(key)))
}

fn command_count(args: &Value) -> i64 {
    command_arg(args, "count")
        .and_then(Value::as_i64)
        .unwrap_or(1)
        .max(1)
}

fn option_bool(options: &Value, key: &str) -> bool {
    options.get(key).and_then(Value::as_bool).unwrap_or(false)
}

fn first_url_option(options: &Value) -> Option<String> {
    options.as_object()?.iter().find_map(|(key, value)| {
        (value.as_bool() == Some(true) && (key.contains("://") || key.starts_with("about:")))
            .then(|| key.to_string())
    })
}

async fn create_tab_index(options: &Value) -> Result<Option<i64>, String> {
    let Some(position) = options.get("position").and_then(Value::as_str) else {
        return Ok(None);
    };
    let Some(active) = active_tab().await? else {
        return Ok(None);
    };
    let tabs = query_current_window().await?;
    let active_index = active.index.unwrap_or(0);
    let last_index = tabs
        .iter()
        .filter_map(|tab| tab.index)
        .max()
        .unwrap_or(active_index);
    let index = match position {
        "start" => 0,
        "before" => active_index,
        "after" => active_index + 1,
        "end" => last_index + 1,
        _ => return Ok(None),
    };
    Ok(Some(index))
}

async fn load_settings() -> Result<UserSettings, String> {
    let stored = storage::sync()
        .get_json(json!({"enabled": true}))
        .await
        .map_err(|e| format!("get settings: {}", e))?;
    let mut settings = UserSettings::new();
    settings.merge(stored);
    Ok(settings)
}

pub async fn query_current_window() -> Result<Vec<tabs::Tab>, String> {
    tabs::query(&tabs::QueryInfo {
        current_window: Some(true),
        ..Default::default()
    })
    .await
    .map_err(|e| format!("query tabs: {}", e))
}

pub async fn activate_relative_tab(delta: i64) -> Result<(), String> {
    let all = query_current_window().await?;
    if all.is_empty() {
        return Ok(());
    }
    let current = all
        .iter()
        .find(|tab| tab.active.unwrap_or(false))
        .and_then(|tab| tab.index)
        .unwrap_or(0);
    let next_index = (current + delta).rem_euclid(all.len() as i64);
    if let Some(tab_id) = all
        .iter()
        .find(|tab| tab.index == Some(next_index))
        .and_then(|tab| tab.id)
    {
        tabs::update(
            tab_id,
            &tabs::UpdateProperties {
                active: Some(true),
                ..Default::default()
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub async fn activate_edge_tab(last: bool) -> Result<(), String> {
    let all = query_current_window().await?;
    let tab_id = if last {
        all.last().and_then(|t| t.id)
    } else {
        all.first().and_then(|t| t.id)
    };
    if let Some(id) = tab_id {
        tabs::update(
            id,
            &tabs::UpdateProperties {
                active: Some(true),
                ..Default::default()
            },
        )
        .await
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_options_from_args() {
        let args = json!({"options": {"key": "val"}});
        let opts = command_options(&args);
        assert_eq!(opts.get("key").and_then(Value::as_str), Some("val"));
    }

    #[test]
    fn command_options_from_nested_args() {
        let args = json!({"args": {"options": {"key": "val"}}});
        let opts = command_options(&args);
        assert_eq!(opts.get("key").and_then(Value::as_str), Some("val"));
    }

    #[test]
    fn command_options_returns_null_when_missing() {
        assert!(command_options(&json!({})).is_null());
    }

    #[test]
    fn command_arg_reads_direct_and_nested() {
        let args = json!({"count": 5, "args": {"hard": true}});
        assert_eq!(command_arg(&args, "count").and_then(Value::as_i64), Some(5));
        assert_eq!(command_arg(&args, "hard").and_then(Value::as_bool), Some(true));
        assert_eq!(command_arg(&args, "missing"), None);
    }

    #[test]
    fn command_count_defaults_to_one() {
        assert_eq!(command_count(&json!({})), 1);
        assert_eq!(command_count(&json!({"count": 0})), 1);
        assert_eq!(command_count(&json!({"count": -5})), 1);
        assert_eq!(command_count(&json!({"count": 3})), 3);
    }

    #[test]
    fn option_bool_returns_false_by_default() {
        assert!(!option_bool(&json!({}), "flag"));
        assert!(!option_bool(&json!({"flag": false}), "flag"));
        assert!(option_bool(&json!({"flag": true}), "flag"));
    }

    #[test]
    fn first_url_option_finds_urls_in_options() {
        let opts = json!({"https://example.com": true, "window": true});
        assert_eq!(first_url_option(&opts), Some("https://example.com".to_string()));
    }

    #[test]
    fn first_url_option_finds_about_urls() {
        let opts = json!({"about:blank": true, "position": "start"});
        assert_eq!(first_url_option(&opts), Some("about:blank".to_string()));
    }

    #[test]
    fn first_url_option_skips_false_flags() {
        let opts = json!({"https://example.com": false});
        assert_eq!(first_url_option(&opts), None);
    }

    #[test]
    fn resolve_new_tab_url_keeps_absolute_urls() {
        assert_eq!(resolve_new_tab_url("https://example.com"), "https://example.com");
        assert_eq!(resolve_new_tab_url("http://example.com/path"), "http://example.com/path");
        assert_eq!(resolve_new_tab_url("about:blank"), "about:blank");
    }
}
