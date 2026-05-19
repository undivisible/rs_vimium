mod background;
pub mod commands;
pub mod key_handler;
pub mod settings;
pub mod vomnibar;

use commands::KeyMapRegistry;
use crepuscularity_core::context::{TemplateContext, TemplateValue};
use crepuscularity_web::render_component_file_to_html;
use crepuscularity_webext::wasm::{
    runtime as browser_runtime, storage, tabs, windows, EventListenerGuard,
};
use once_cell::sync::Lazy;
use serde::Serialize;
use serde_json::{json, Value};
use settings::UserSettings;
use std::sync::Mutex;
use wasm_bindgen::prelude::*;

const UI_CREPUS: &str = include_str!("../views/ui.crepus");

static COMMAND_REGISTRY: Lazy<KeyMapRegistry> = Lazy::new(KeyMapRegistry::from_defaults);
static USER_SETTINGS: Lazy<Mutex<UserSettings>> = Lazy::new(|| Mutex::new(UserSettings::new()));
static USER_MAPPINGS: Lazy<
    Mutex<std::collections::HashMap<String, Option<commands::RegistryEntry>>>,
> = Lazy::new(|| Mutex::new(std::collections::HashMap::new()));

fn json_to_template(value: Value) -> TemplateValue {
    match value {
        Value::Bool(v) => TemplateValue::Bool(v),
        Value::Number(v) => {
            if let Some(i) = v.as_i64() {
                TemplateValue::Int(i)
            } else {
                TemplateValue::Float(v.as_f64().unwrap_or(0.0))
            }
        }
        Value::String(v) => TemplateValue::Str(v),
        Value::Array(vals) => TemplateValue::List(
            vals.into_iter()
                .map(|item| {
                    let mut ctx = TemplateContext::new();
                    if let Value::Object(fields) = item {
                        for (k, v) in fields {
                            ctx.set(k, json_to_template(v));
                        }
                    }
                    ctx
                })
                .collect(),
        ),
        _ => TemplateValue::Null,
    }
}

fn to_js(value: Value) -> Result<JsValue, JsValue> {
    value
        .serialize(&serde_wasm_bindgen::Serializer::json_compatible())
        .map_err(|e| JsValue::from_str(&e.to_string()))
}

fn from_js(value: JsValue) -> Value {
    serde_wasm_bindgen::from_value(value).unwrap_or(Value::Null)
}

#[wasm_bindgen]
pub fn runtime_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[wasm_bindgen]
pub fn render_popup(_state: JsValue) -> Result<JsValue, JsValue> {
    let mut ctx = TemplateContext::new();
    ctx.set("title", "rs_vimium");
    ctx.set("status", "Enabled");
    ctx.set("groups", json_to_template(shortcut_groups_json_val()));
    let html = render_component_file_to_html(UI_CREPUS, "Popup", &ctx)
        .map_err(|e| JsValue::from_str(&e))?;
    to_js(json!({"html": html, "css": POPUP_CSS}))
}

#[wasm_bindgen]
pub fn shortcut_groups_json() -> Result<JsValue, JsValue> {
    to_js(shortcut_groups_json_val())
}

fn shortcut_groups_json_val() -> Value {
    let commands = commands::all_commands();
    let mut groups: Value = json!([
        {"name": "Navigation", "items": []},
        {"name": "Vomnibar", "items": []},
        {"name": "Find", "items": []},
        {"name": "History", "items": []},
        {"name": "Tabs", "items": []},
        {"name": "Misc", "items": []},
    ]);

    let group_map: std::collections::HashMap<_, _> = [
        ("navigation", 0),
        ("vomnibar", 1),
        ("find", 2),
        ("history", 3),
        ("tabs", 4),
        ("misc", 5),
    ]
    .into_iter()
    .collect();

    for cmd in &commands {
        if let Some(&idx) = group_map.get(cmd.group.as_str()) {
            let item = json!({
                "keys": command_keys_for_name(&cmd.name),
                "label": cmd.desc,
                "details": cmd.details,
                "command": cmd.name,
                "advanced": cmd.advanced
            });
            if let Some(arr) = groups[idx].get_mut("items").and_then(Value::as_array_mut) {
                arr.push(item);
            }
        }
    }
    groups
}

fn command_keys_for_name(cmd_name: &str) -> String {
    let registry = &*COMMAND_REGISTRY;
    let mut keys: Vec<String> = registry
        .key_to_command
        .iter()
        .filter(|(_, v)| *v == cmd_name)
        .map(|(k, _)| k.clone())
        .collect();
    keys.sort_by_key(|k| k.len());
    if keys.is_empty() {
        "—".to_string()
    } else {
        keys.join(" / ")
    }
}

#[wasm_bindgen]
pub async fn settings_get() -> Result<JsValue, JsValue> {
    let stored = storage::sync()
        .get_json(Value::Null)
        .await
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    if let Ok(mut s) = USER_SETTINGS.lock() {
        s.merge(stored);
    }
    let settings = USER_SETTINGS
        .lock()
        .map(|s| s.settings.clone())
        .unwrap_or_default();
    to_js(json!({"ok": true, "settings": settings}))
}

#[wasm_bindgen]
pub async fn settings_seed() -> Result<JsValue, JsValue> {
    let stored = storage::sync()
        .get_json(Value::Null)
        .await
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    let seeded = if let Ok(mut s) = USER_SETTINGS.lock() {
        s.merge(stored);
        let pruned = settings::prune_defaults(&s.settings);
        let session_metadata = COMMAND_REGISTRY.session_metadata(&s.get_str("keyMappings"));
        if let Ok(mut mappings) = USER_MAPPINGS.lock() {
            *mappings = COMMAND_REGISTRY.parse_user_mappings(&s.get_str("keyMappings"));
        }
        Some((pruned, session_metadata))
    } else {
        None
    };
    if let Some((pruned, session_metadata)) = seeded {
        storage::sync()
            .set(&pruned)
            .await
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        storage::session()
            .set(&session_metadata)
            .await
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
    }
    to_js(json!({"ok": true}))
}

#[wasm_bindgen]
pub async fn settings_set(values: JsValue) -> Result<JsValue, JsValue> {
    let values = from_js(values);
    storage::sync()
        .set(&values)
        .await
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    settings_seed().await
}

#[wasm_bindgen]
pub async fn settings_clear() -> Result<JsValue, JsValue> {
    storage::sync()
        .clear()
        .await
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    settings_seed().await
}

#[wasm_bindgen]
pub async fn notify_settings_changed() -> Result<JsValue, JsValue> {
    let all_tabs = tabs::query(&tabs::QueryInfo::default())
        .await
        .map_err(|e| JsValue::from_str(&e.to_string()))?;
    for tab in all_tabs {
        let Some(tab_id) = tab.id else {
            continue;
        };
        let Some(url) = tab.url else {
            continue;
        };
        if !url.starts_with("http://") && !url.starts_with("https://") {
            continue;
        }
        let message = to_js(json!({"type": "settings:changed"}))?;
        let _ = tabs::send_message_value(tab_id, message).await;
    }
    to_js(json!({"ok": true}))
}

#[wasm_bindgen]
pub async fn send_runtime_message(message: JsValue) -> Result<JsValue, JsValue> {
    browser_runtime::send_message_value(message)
        .await
        .map_err(|e| JsValue::from_str(&e.to_string()))
}

#[wasm_bindgen]
pub fn content_key(state_val: JsValue, key: &str, editable: bool) -> Result<JsValue, JsValue> {
    let incoming = from_js(state_val);
    let state = key_handler::KeyState {
        mode: incoming
            .get("mode")
            .and_then(Value::as_str)
            .unwrap_or("normal")
            .to_string(),
        sequence: incoming
            .get("sequence")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
        count_text: incoming
            .get("countText")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
        input: incoming
            .get("input")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
    };

    let mappings = USER_MAPPINGS.lock().unwrap();
    let exclusion_state = current_exclusion_state();
    if !exclusion_state.is_enabled_for_url {
        return to_js(json!({
            "state": {
                "mode": state.mode,
                "sequence": state.sequence,
                "countText": state.count_text,
                "input": state.input
            },
            "effect": null,
            "prevent": false
        }));
    }
    let result = key_handler::handle_key(
        &state,
        key,
        editable,
        &COMMAND_REGISTRY,
        &mappings,
        &exclusion_state.pass_keys,
    );
    to_js(result)
}

#[wasm_bindgen]
pub fn render_help_overlay(show_advanced: bool) -> String {
    let groups = shortcut_groups_json_val();
    let mut html = String::from(
        r#"<div class="vc-overlay-header"><span>rs_vimium shortcuts</span><button class="vc-overlay-close" type="button">Esc</button></div><div class="vc-overlay-grid">"#,
    );
    for group in groups.as_array().into_iter().flatten() {
        let name = group.get("name").and_then(Value::as_str).unwrap_or("");
        let items = group.get("items").and_then(Value::as_array);
        if let Some(items) = items {
            let visible: Vec<_> = items
                .iter()
                .filter(|item| {
                    show_advanced
                        || !item
                            .get("advanced")
                            .and_then(Value::as_bool)
                            .unwrap_or(false)
                })
                .collect();
            if visible.is_empty() {
                continue;
            }
            html.push_str(&format!(
                r#"<div class="vc-group"><div class="vc-group-title">{name}</div>"#
            ));
            for item in &visible {
                let keys = item.get("keys").and_then(Value::as_str).unwrap_or("");
                let label = item.get("label").and_then(Value::as_str).unwrap_or("");
                html.push_str(&format!(
                    r#"<div class="vc-overlay-row"><span class="vc-overlay-key">{keys}</span><span class="vc-overlay-label">{label}</span></div>"#
                ));
            }
            html.push_str("</div>");
        }
    }
    html.push_str("</div>");
    html
}

#[wasm_bindgen]
pub fn command_list() -> Result<JsValue, JsValue> {
    to_js(shortcut_groups_json_val())
}

#[wasm_bindgen]
pub fn hint_label(index: usize) -> String {
    hint_labels_with_chars(index + 1, "asdfghjklqwertyuiopzxcvbnm")
        .pop()
        .unwrap_or_default()
}

pub fn hint_label_with_chars(index: usize, chars: &str) -> String {
    hint_labels_with_chars(index + 1, chars)
        .pop()
        .unwrap_or_default()
}

fn hint_labels_with_chars(count: usize, chars: &str) -> Vec<String> {
    let chars = chars
        .to_lowercase()
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .collect::<Vec<_>>();
    if count == 0 || chars.len() <= 1 {
        return Vec::new();
    }

    let mut hints = vec![String::new()];
    let mut offset = 0usize;
    while hints.len().saturating_sub(offset) < count || hints.len() == 1 {
        let hint = hints[offset].clone();
        offset += 1;
        for ch in &chars {
            hints.push(format!("{ch}{hint}"));
        }
    }

    let mut labels = hints
        .into_iter()
        .skip(offset)
        .take(count)
        .collect::<Vec<_>>();
    labels.sort();
    labels
        .into_iter()
        .map(|label| label.chars().rev().collect())
        .collect()
}

#[wasm_bindgen]
pub fn update_hint_state(labels: JsValue, current: &str, key: &str) -> Result<JsValue, JsValue> {
    let labels = from_js(labels);
    let next_input = format!("{}{}", current, key.to_lowercase());
    let labels_arr = labels.as_array().cloned().unwrap_or_default();
    let mut exact = None;
    let mut remaining = Vec::new();
    let mut dim = Vec::new();

    for (i, label) in labels_arr.iter().enumerate() {
        let label = label.as_str().unwrap_or("");
        let matched = label.starts_with(&next_input);
        dim.push(!matched);
        if matched {
            remaining.push(i);
        }
        if label == next_input {
            exact = Some(i);
        }
    }

    let selected = if remaining.len() == 1 {
        exact.or_else(|| remaining.first().copied())
    } else {
        None
    };
    to_js(json!({"input": next_input, "dim": dim, "selected": selected}))
}

#[wasm_bindgen]
pub fn resolve_navigable(query: &str) -> Result<JsValue, JsValue> {
    let settings = USER_SETTINGS.lock().unwrap();
    let engines = vomnibar::SearchEngines::from_settings(&settings);
    to_js(vomnibar::resolve_navigable(query, &engines))
}

#[wasm_bindgen]
pub async fn query_vomnibar(query: &str, mode: &str) -> Result<JsValue, JsValue> {
    let mode = match mode {
        "bookmarks" => vomnibar::VomnibarMode::Bookmarks,
        "tabs" => vomnibar::VomnibarMode::Tabs,
        _ => vomnibar::VomnibarMode::Full,
    };
    let result = vomnibar::query_vomnibar(query, mode)
        .await
        .map_err(|e| JsValue::from_str(&e))?;
    let items: Vec<Value> = result
        .items
        .into_iter()
        .map(|item| {
            json!({
                "title": item.title,
                "url": item.url,
                "kind": item.kind,
                "relevance": item.relevance,
            })
        })
        .collect();
    to_js(json!({"items": items}))
}

#[wasm_bindgen]
pub fn key_name(event_key: &str) -> String {
    key_handler::key_name(event_key)
}

fn key_name_from_event(event: &KeyboardEvent) -> String {
    let raw = key_handler::key_name(&event.key());
    if raw.is_empty()
        || matches!(
            raw.as_str(),
            "alt" | "control" | "meta" | "shift" | "altgraph"
        )
    {
        return String::new();
    }
    let has_non_shift_modifier = event.alt_key() || event.ctrl_key() || event.meta_key();
    let mut key = raw;
    if key.chars().count() == 1 {
        key = if event.shift_key() {
            key.to_uppercase()
        } else if has_non_shift_modifier {
            key.to_lowercase()
        } else {
            key
        };
    }
    let mut modifiers = Vec::new();
    if event.alt_key() {
        modifiers.push("a");
    }
    if event.ctrl_key() {
        modifiers.push("c");
    }
    if event.meta_key() {
        modifiers.push("m");
    }
    if event.shift_key() && key.chars().count() > 1 {
        modifiers.push("s");
    }
    let combined = if modifiers.is_empty() {
        key
    } else {
        let mut parts = modifiers
            .into_iter()
            .map(str::to_string)
            .collect::<Vec<_>>();
        parts.push(key);
        format!("<{}>", parts.join("-"))
    };
    CONTENT_STATE.with(|state| {
        state
            .borrow()
            .mapped_keys
            .get(&combined)
            .cloned()
            .unwrap_or(combined)
    })
}

#[wasm_bindgen]
pub fn is_search_query(query: &str) -> bool {
    vomnibar::SearchEngines::is_search_query(query)
}

async fn vimium_secret() -> Result<String, String> {
    let session = storage::session();
    let stored = session
        .get_json(json!({"vimiumSecret": ""}))
        .await
        .map_err(|e| format!("get session secret: {}", e))?;
    let mut secret = stored
        .get("vimiumSecret")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    if secret.is_empty() {
        secret = format!("{}:{}", js_sys::Date::now(), js_sys::Math::random());
        session
            .set(&json!({"vimiumSecret": secret}))
            .await
            .map_err(|e| format!("save session secret: {}", e))?;
    }
    Ok(secret)
}

fn global_mark_key(mark_name: &str) -> String {
    format!("vimiumGlobalMark|{}", mark_name)
}

async fn create_global_mark_background(msg: &Value) -> Result<(), String> {
    let mark_name = msg
        .get("markName")
        .and_then(Value::as_str)
        .ok_or_else(|| "missing mark name".to_string())?;
    let tab = background::active_tab()
        .await?
        .ok_or_else(|| "no active tab".to_string())?;
    let tab_id = tab.id.ok_or_else(|| "active tab has no id".to_string())?;
    let url = msg
        .get("url")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .or(tab.url)
        .unwrap_or_default()
        .split('#')
        .next()
        .unwrap_or("")
        .to_string();
    let mark = json!({
        "vimiumSecret": vimium_secret().await?,
        "markName": mark_name,
        "url": url,
        "tabId": tab_id,
        "scrollX": msg.get("scrollX").and_then(Value::as_f64).unwrap_or(0.0),
        "scrollY": msg.get("scrollY").and_then(Value::as_f64).unwrap_or(0.0),
    });
    storage::local()
        .set(&json!({global_mark_key(mark_name): mark}))
        .await
        .map_err(|e| format!("save global mark: {}", e))
}

async fn goto_global_mark_background(msg: &Value) -> Result<bool, String> {
    let mark_name = msg
        .get("markName")
        .and_then(Value::as_str)
        .ok_or_else(|| "missing mark name".to_string())?;
    let key = global_mark_key(mark_name);
    let stored = storage::local()
        .get_json(json!({key.clone(): null}))
        .await
        .map_err(|e| format!("get global mark: {}", e))?;
    let Some(mark) = stored.get(&key).filter(|value| !value.is_null()) else {
        return Ok(false);
    };
    let url = mark.get("url").and_then(Value::as_str).unwrap_or("");
    let scroll_x = mark.get("scrollX").and_then(Value::as_f64).unwrap_or(0.0);
    let scroll_y = mark.get("scrollY").and_then(Value::as_f64).unwrap_or(0.0);
    let current_secret = vimium_secret().await?;
    let same_session = mark
        .get("vimiumSecret")
        .and_then(Value::as_str)
        .is_some_and(|secret| secret == current_secret);
    if same_session {
        if let Some(tab_id) = mark.get("tabId").and_then(Value::as_i64) {
            if let Ok(tab) = tabs::update(
                tab_id,
                &tabs::UpdateProperties {
                    active: Some(true),
                    ..Default::default()
                },
            )
            .await
            {
                focus_mark_window(&tab).await;
                send_mark_scroll(tab_id, scroll_x, scroll_y).await;
                return Ok(true);
            }
        }
    }
    if let Some(tab) = find_mark_tab(url, scroll_x != 0.0 || scroll_y != 0.0).await? {
        let Some(tab_id) = tab.id else {
            return Ok(false);
        };
        let tab = tabs::update(
            tab_id,
            &tabs::UpdateProperties {
                active: Some(true),
                ..Default::default()
            },
        )
        .await
        .map_err(|e| format!("activate mark tab: {}", e))?;
        focus_mark_window(&tab).await;
        send_mark_scroll(tab_id, scroll_x, scroll_y).await;
        return Ok(true);
    }
    let tab = tabs::create(&tabs::CreateProperties {
        url: Some(url.to_string()),
        active: Some(true),
        ..Default::default()
    })
    .await
    .map_err(|e| format!("create mark tab: {}", e))?;
    focus_mark_window(&tab).await;
    if let Some(tab_id) = tab.id {
        send_mark_scroll(tab_id, scroll_x, scroll_y).await;
    }
    Ok(true)
}

async fn find_mark_tab(url: &str, mark_is_scrolled: bool) -> Result<Option<tabs::Tab>, String> {
    let query_url = if mark_is_scrolled {
        json!(url)
    } else {
        json!(format!("{}*", url))
    };
    let mut candidates = tabs::query(&tabs::QueryInfo {
        url: Some(query_url),
        ..Default::default()
    })
    .await
    .map_err(|e| format!("query mark tabs: {}", e))?;
    if candidates.is_empty() {
        return Ok(None);
    }
    if candidates.len() > 1 {
        let current = background::active_tab().await?.and_then(|tab| tab.id);
        candidates.retain(|tab| tab.id != current);
    }
    candidates.sort_by_key(|tab| tab.url.as_ref().map(|url| url.len()).unwrap_or(usize::MAX));
    Ok(candidates.into_iter().next())
}

async fn focus_mark_window(tab: &tabs::Tab) {
    if let Some(window_id) = tab.window_id {
        let _ = windows::update(
            window_id,
            &windows::UpdateInfo {
                focused: Some(true),
                ..Default::default()
            },
        )
        .await;
    }
}

async fn send_mark_scroll(tab_id: i64, scroll_x: f64, scroll_y: f64) {
    let _ = tabs::send_message_value(
        tab_id,
        to_js(json!({
            "type": "rs_vimium",
            "command": "set-scroll-position",
            "scrollX": scroll_x,
            "scrollY": scroll_y,
        }))
        .unwrap_or(JsValue::NULL),
    )
    .await;
}

#[wasm_bindgen]
pub async fn handle_background_message(message: JsValue) -> Result<JsValue, JsValue> {
    let msg = from_js(message);
    let msg_type = msg.get("type").and_then(Value::as_str).unwrap_or("");

    match msg_type {
        "settings:get" => settings_get().await,
        "rs_vimium" | "" => {
            let command = msg.get("command").and_then(Value::as_str).unwrap_or("");
            if command == "create-global-mark" {
                create_global_mark_background(&msg)
                    .await
                    .map_err(|e| JsValue::from_str(&e))?;
                return to_js(json!({"ok": true}));
            }
            if command == "goto-global-mark" {
                let jumped = goto_global_mark_background(&msg)
                    .await
                    .map_err(|e| JsValue::from_str(&e))?;
                return to_js(json!({"ok": jumped}));
            }
            if command == "focus-main-frame" {
                if let Some(tab_id) = background::active_tab()
                    .await
                    .map_err(|e| JsValue::from_str(&e))?
                    .and_then(|tab| tab.id)
                {
                    let _ = tabs::send_message_value(
                        tab_id,
                        to_js(json!({
                            "type": "rs_vimium",
                            "command": "focus-this-frame",
                            "topOnly": true,
                            "highlight": true,
                        }))?,
                    )
                    .await;
                }
                return to_js(json!({"ok": true}));
            }
            background::execute_background_command(command, &msg)
                .await
                .map_err(|e| JsValue::from_str(&e))?;
            to_js(json!({"ok": true}))
        }
        _ => {
            let command = msg.get("handler").and_then(Value::as_str).unwrap_or("");
            if command == "runBackgroundCommand" {
                let empty = json!({});
                let registry_entry = msg.get("registryEntry").unwrap_or(&empty);
                let cmd_name = registry_entry
                    .get("command")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                let background_command =
                    key_handler::background_command_for_registry_name(cmd_name).unwrap_or(cmd_name);
                background::execute_background_command(background_command, &msg)
                    .await
                    .map_err(|e| JsValue::from_str(&e))?;
                to_js(json!({"ok": true}))
            } else {
                to_js(json!({"ok": false, "error": format!("unknown message: {}", msg_type)}))
            }
        }
    }
}

pub const POPUP_CSS: &str = r#"
body{font-family:"JetBrains Mono",ui-monospace,SFMono-Regular,Consolas,monospace}
"#;
use std::cell::RefCell;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::{spawn_local, JsFuture};
use web_sys::{
    Document, Element, HtmlElement, HtmlInputElement, HtmlSelectElement, HtmlTextAreaElement,
    KeyboardEvent, Node, Range, Window,
};

thread_local! {
    static CONTENT_STATE: RefCell<ContentDomState> = RefCell::new(ContentDomState::default());
    static EVENT_GUARDS: RefCell<Vec<EventListenerGuard>> = const { RefCell::new(Vec::new()) };
}

#[derive(Default)]
struct ContentDomState {
    enabled: bool,
    key_state: key_handler::KeyState,
    hints: Vec<HintDom>,
    hint_input: String,
    hint_action: String,
    last_find_query: String,
    find_matches: Vec<FindMatch>,
    find_active_index: usize,
    settings: Value,
    mapped_keys: std::collections::HashMap<String, String>,
    activated_element: Option<Element>,
    pass_next_key: bool,
    local_marks: std::collections::HashMap<String, Value>,
    mark_mode: Option<MarkMode>,
    visual_line_mode: bool,
}

struct HintDom {
    label: String,
    marker: Element,
    target: Element,
}

#[derive(Clone)]
struct FindMatch {
    node: Node,
    start: u32,
    end: u32,
}

#[derive(Clone, Copy)]
enum MarkMode {
    Create,
    Goto,
}

fn win() -> Option<Window> {
    web_sys::window()
}

fn doc() -> Option<Document> {
    win()?.document()
}

fn is_editable_target(target: Option<web_sys::EventTarget>) -> bool {
    let Some(target) = target else {
        return false;
    };
    let Ok(element) = target.dyn_into::<Element>() else {
        return false;
    };
    let tag = element.tag_name().to_lowercase();
    element.has_attribute("contenteditable")
        || matches!(tag.as_str(), "input" | "textarea" | "select")
}

fn setting_value(key: &str, fallback: Value) -> Value {
    CONTENT_STATE.with(|state| {
        state
            .borrow()
            .settings
            .get(key)
            .cloned()
            .unwrap_or(fallback)
    })
}

fn setting_bool(key: &str, fallback: bool) -> bool {
    setting_value(key, Value::Bool(fallback))
        .as_bool()
        .unwrap_or(fallback)
}

fn setting_f64(key: &str, fallback: f64) -> f64 {
    setting_value(key, json!(fallback))
        .as_f64()
        .unwrap_or(fallback)
}

fn current_exclusion_state() -> settings::ExclusionState {
    let rules = CONTENT_STATE.with(|state| {
        state
            .borrow()
            .settings
            .get("exclusionRules")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
    });
    let rules = rules
        .into_iter()
        .filter_map(|rule| {
            Some(settings::ExclusionRule {
                pattern: rule.get("pattern")?.as_str()?.to_string(),
                pass_keys: rule
                    .get("passKeys")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
            })
        })
        .collect::<Vec<_>>();
    location_href()
        .map(|url| settings::enabled_state_for_url(&url, &rules))
        .unwrap_or(settings::ExclusionState {
            is_enabled_for_url: true,
            pass_keys: String::new(),
        })
}

fn clear_selector(selector: &str) {
    let Some(document) = doc() else {
        return;
    };
    if let Ok(nodes) = document.query_selector_all(selector) {
        for i in 0..nodes.length() {
            if let Some(node) = nodes.item(i) {
                if let Some(parent) = node.parent_node() {
                    let _ = parent.remove_child(&node);
                }
            }
        }
    }
}

fn clear_hints() {
    CONTENT_STATE.with(|state| {
        for hint in &state.borrow().hints {
            hint.marker.remove();
        }
        let mut state = state.borrow_mut();
        state.hints.clear();
        state.hint_input.clear();
        if state.key_state.mode == "hints" {
            state.key_state.mode = "normal".to_string();
        }
    });
}

fn clear_overlays() {
    clear_hints();
    clear_find_highlights();
    clear_selector(".vc-overlay,.vc-find,.vc-hint,.vc-vomnibar,.vc-hud");
}

fn set_text(el: &Element, text: &str) {
    el.set_text_content(Some(text));
}

fn append(parent: &Element, child: &Element) {
    let _ = parent.append_child(child);
}

fn show_help() {
    clear_overlays();
    let Some(document) = doc() else {
        return;
    };
    let Ok(overlay) = document.create_element("section") else {
        return;
    };
    overlay.set_class_name("vc-overlay");
    overlay.set_inner_html(&render_help_overlay(setting_bool(
        "helpDialog_showAdvancedCommands",
        false,
    )));
    if let Some(root) = document.document_element() {
        append(&root, &overlay);
    }
}

fn show_hud(text: &str) {
    clear_selector(".vc-hud");
    let Some(document) = doc() else {
        return;
    };
    let Ok(hud) = document.create_element("div") else {
        return;
    };
    hud.set_class_name("vc-hud");
    set_text(&hud, text);
    if let Some(root) = document.document_element() {
        append(&root, &hud);
    }
}

fn base_url() -> String {
    win()
        .and_then(|window| window.location().href().ok())
        .unwrap_or_default()
        .split('#')
        .next()
        .unwrap_or("")
        .to_string()
}

fn current_mark_position() -> Value {
    let (scroll_x, scroll_y, hash) = if let Some(window) = win() {
        (
            window.scroll_x().unwrap_or(0.0),
            window.scroll_y().unwrap_or(0.0),
            window.location().hash().unwrap_or_default(),
        )
    } else {
        (0.0, 0.0, String::new())
    };
    json!({"scrollX": scroll_x, "scrollY": scroll_y, "hash": hash})
}

fn local_mark_key(key: &str) -> String {
    format!("vimiumMark|{}|{}", base_url(), key)
}

fn storage_get(key: &str) -> Option<String> {
    let storage =
        js_sys::Reflect::get(&js_sys::global(), &JsValue::from_str("localStorage")).ok()?;
    let method = js_sys::Reflect::get(&storage, &JsValue::from_str("getItem"))
        .ok()?
        .dyn_into::<js_sys::Function>()
        .ok()?;
    method
        .call1(&storage, &JsValue::from_str(key))
        .ok()
        .and_then(|value| value.as_string())
}

fn storage_set(key: &str, value: &str) {
    let Ok(storage) = js_sys::Reflect::get(&js_sys::global(), &JsValue::from_str("localStorage"))
    else {
        return;
    };
    let Ok(method_value) = js_sys::Reflect::get(&storage, &JsValue::from_str("setItem")) else {
        return;
    };
    let Ok(method) = method_value.dyn_into::<js_sys::Function>() else {
        return;
    };
    let _ = method.call2(&storage, &JsValue::from_str(key), &JsValue::from_str(value));
}

fn previous_position_registers() -> [&'static str; 2] {
    ["`", "'"]
}

fn set_previous_position() {
    let position = current_mark_position();
    CONTENT_STATE.with(|state| {
        let mut state = state.borrow_mut();
        for register in previous_position_registers() {
            state
                .local_marks
                .insert(register.to_string(), position.clone());
        }
    });
}

fn is_global_mark(event: &KeyboardEvent, key: &str) -> bool {
    event.shift_key() && !previous_position_registers().contains(&key)
}

fn create_local_mark(key: &str) {
    let position = current_mark_position();
    CONTENT_STATE.with(|state| {
        state
            .borrow_mut()
            .local_marks
            .insert(key.to_string(), position.clone());
    });
    storage_set(&local_mark_key(key), &position.to_string());
    show_hud(&format!("Created local mark \"{}\".", key));
}

fn goto_local_mark(key: &str) {
    let position = CONTENT_STATE
        .with(|state| state.borrow().local_marks.get(key).cloned())
        .or_else(|| {
            storage_get(&local_mark_key(key)).and_then(|value| serde_json::from_str(&value).ok())
        });
    if let Some(position) = position {
        set_previous_position();
        scroll_to_mark(&position);
        show_hud(&format!("Jumped to local mark \"{}\".", key));
    } else {
        show_hud(&format!("Local mark not set \"{}\".", key));
    }
}

fn scroll_to_mark(position: &Value) {
    let scroll_x = position
        .get("scrollX")
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    let scroll_y = position
        .get("scrollY")
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    let hash = position.get("hash").and_then(Value::as_str).unwrap_or("");
    if let Some(window) = win() {
        if !hash.is_empty() && scroll_x == 0.0 && scroll_y == 0.0 {
            let _ = window.location().set_hash(hash.trim_start_matches('#'));
        } else {
            window.scroll_to_with_x_and_y(scroll_x, scroll_y);
        }
    }
}

fn handle_mark_key(key: &str, event: &KeyboardEvent) {
    let mode = CONTENT_STATE.with(|state| state.borrow_mut().mark_mode.take());
    CONTENT_STATE.with(|state| {
        state.borrow_mut().key_state.mode = "normal".to_string();
    });
    let Some(mode) = mode else {
        return;
    };
    if key == "Esc" || key.chars().count() != 1 {
        show_hud("Mark cancelled.");
        return;
    }
    if is_global_mark(event, key) {
        match mode {
            MarkMode::Create => create_global_mark(key.to_string()),
            MarkMode::Goto => goto_global_mark(key.to_string()),
        }
    } else {
        match mode {
            MarkMode::Create => create_local_mark(key),
            MarkMode::Goto => goto_local_mark(key),
        }
    }
}

fn create_global_mark(key: String) {
    let position = current_mark_position();
    spawn_local(async move {
        let msg = to_js(json!({
            "type": "rs_vimium",
            "command": "create-global-mark",
            "markName": key,
            "scrollX": position.get("scrollX").and_then(Value::as_f64).unwrap_or(0.0),
            "scrollY": position.get("scrollY").and_then(Value::as_f64).unwrap_or(0.0),
            "url": base_url(),
        }))
        .unwrap_or(JsValue::NULL);
        match send_runtime_message(msg).await {
            Ok(_) => show_hud("Created global mark."),
            Err(_) => show_hud("Global mark failed."),
        }
    });
}

fn goto_global_mark(key: String) {
    spawn_local(async move {
        let msg = to_js(json!({
            "type": "rs_vimium",
            "command": "goto-global-mark",
            "markName": key,
        }))
        .unwrap_or(JsValue::NULL);
        match send_runtime_message(msg).await {
            Ok(resp) => {
                let response = from_js(resp);
                if response.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    show_hud("Jumped to global mark.");
                } else {
                    show_hud("Global mark not set.");
                }
            }
            Err(_) => show_hud("Global mark failed."),
        }
    });
}

fn visible(element: &Element) -> bool {
    let rect = element.get_bounding_client_rect();
    rect.width() > 0.0 && rect.height() > 0.0
}

fn element_f64(element: &Element, key: &str) -> f64 {
    js_sys::Reflect::get(element.as_ref(), &JsValue::from_str(key))
        .ok()
        .and_then(|value| value.as_f64())
        .unwrap_or(0.0)
}

fn set_element_f64(element: &Element, key: &str, value: f64) {
    let _ = js_sys::Reflect::set(
        element.as_ref(),
        &JsValue::from_str(key),
        &JsValue::from_f64(value),
    );
}

fn viewport_size(axis: &str) -> f64 {
    let Some(window) = win() else {
        return 600.0;
    };
    let value = if axis == "x" {
        window.inner_width()
    } else {
        window.inner_height()
    };
    value.ok().and_then(|v| v.as_f64()).unwrap_or(600.0)
}

fn location_href() -> Option<String> {
    win()?.location().href().ok()
}

fn set_location_href(url: &str) {
    if let Some(window) = win() {
        let _ = window.location().set_href(url);
    }
}

fn special_scrolling_element(document: &Document) -> Option<Element> {
    let host = win()?.location().host().ok()?;
    let selector = match host.as_str() {
        "twitter.com" => "div.permalink-container div.permalink[role=main]",
        "reddit.com" | "new.reddit.com" | "www.reddit.com" => "#overlayScrollContainer",
        "web.telegram.org" => ".MessageList",
        _ => return None,
    };
    document.query_selector(selector).ok().flatten()
}

fn scrolling_element() -> Option<Element> {
    let document = doc()?;
    special_scrolling_element(&document)
        .or_else(|| document.scrolling_element())
        .or_else(|| document.body().map(Into::into))
}

fn axis_names(axis: &str) -> (&'static str, &'static str, &'static str) {
    if axis == "x" {
        ("scrollLeft", "scrollWidth", "clientWidth")
    } else {
        ("scrollTop", "scrollHeight", "clientHeight")
    }
}

fn should_scroll_element(element: &Element, axis: &str) -> bool {
    let Some(window) = win() else {
        return true;
    };
    let Ok(Some(style)) = window.get_computed_style(element) else {
        return true;
    };
    let overflow = style
        .get_property_value(if axis == "x" {
            "overflow-x"
        } else {
            "overflow-y"
        })
        .unwrap_or_default();
    let visibility = style.get_property_value("visibility").unwrap_or_default();
    let display = style.get_property_value("display").unwrap_or_default();
    overflow != "hidden" && visibility != "hidden" && visibility != "collapse" && display != "none"
}

fn can_scroll_element(element: &Element, axis: &str, delta: f64) -> bool {
    if !should_scroll_element(element, axis) {
        return false;
    }
    let (scroll_pos, scroll_size, client_size) = axis_names(axis);
    let pos = element_f64(element, scroll_pos);
    let max = (element_f64(element, scroll_size) - element_f64(element, client_size)).max(0.0);
    if delta < 0.0 {
        pos > 0.0
    } else if delta > 0.0 {
        pos < max
    } else {
        max > 0.0
    }
}

fn active_scroll_element() -> Option<Element> {
    CONTENT_STATE
        .with(|state| state.borrow().activated_element.clone())
        .or_else(|| doc().and_then(|document| document.active_element()))
        .or_else(scrolling_element)
}

fn find_scrollable_element(start: Element, axis: &str, delta: f64) -> Option<Element> {
    let root = scrolling_element();
    let mut current = Some(start);
    while let Some(element) = current {
        if can_scroll_element(&element, axis, delta) {
            return Some(element);
        }
        if root.as_ref().is_some_and(|root| root == &element) {
            return root;
        }
        current = element.parent_element().or_else(|| root.clone());
    }
    root
}

fn scroll_element_by(axis: &str, amount: f64) {
    let Some(start) = active_scroll_element() else {
        return;
    };
    let Some(element) = find_scrollable_element(start, axis, amount) else {
        return;
    };
    let (scroll_pos, _, _) = axis_names(axis);
    let before = element_f64(&element, scroll_pos);
    set_element_f64(&element, scroll_pos, before + amount);
    if !visible(&element) {
        CONTENT_STATE.with(|state| {
            state.borrow_mut().activated_element = scrolling_element();
        });
    }
}

fn scroll_element_to(axis: &str, target: f64) {
    let Some(start) = active_scroll_element() else {
        return;
    };
    let delta = if target <= 0.0 { -1.0 } else { 1.0 };
    let Some(element) = find_scrollable_element(start, axis, delta) else {
        return;
    };
    let (scroll_pos, scroll_size, client_size) = axis_names(axis);
    let max = (element_f64(&element, scroll_size) - element_f64(&element, client_size)).max(0.0);
    set_element_f64(&element, scroll_pos, target.clamp(0.0, max));
}

fn focus_input(count: i64) {
    let Some(document) = doc() else {
        return;
    };
    let selector = "input:not([disabled]):not([readonly]):not([type='hidden']):not([type='checkbox']):not([type='radio']):not([type='submit']):not([type='button']):not([type='reset']),textarea:not([disabled]):not([readonly]),[contenteditable=''],[contenteditable='true'],[contenteditable='TRUE']";
    let Ok(nodes) = document.query_selector_all(selector) else {
        return;
    };
    let mut inputs = Vec::new();
    for i in 0..nodes.length() {
        let Some(node) = nodes.item(i) else {
            continue;
        };
        let Ok(element) = node.dyn_into::<Element>() else {
            continue;
        };
        if !visible(&element) {
            continue;
        }
        let tab_index = js_sys::Reflect::get(element.as_ref(), &JsValue::from_str("tabIndex"))
            .ok()
            .and_then(|value| value.as_f64())
            .unwrap_or(0.0) as i64;
        inputs.push((i, tab_index, element));
    }
    inputs.sort_by(|(left_i, left_tab, _), (right_i, right_tab, _)| {
        match (*left_tab > 0, *right_tab > 0) {
            (true, true) if left_tab != right_tab => left_tab.cmp(right_tab),
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => left_i.cmp(right_i),
        }
    });
    if inputs.is_empty() {
        show_hud("There are no inputs to focus.");
        return;
    }
    let selected = (count.max(1) as usize).min(inputs.len()) - 1;
    let element = &inputs[selected].2;
    if let Some(input) = element.dyn_ref::<HtmlInputElement>() {
        let _ = input.focus();
        input.select();
    } else if let Some(textarea) = element.dyn_ref::<HtmlTextAreaElement>() {
        let _ = textarea.focus();
        textarea.select();
    } else if let Some(html) = element.dyn_ref::<HtmlElement>() {
        let _ = html.focus();
    }
}

fn element_text_for_match(element: &Element) -> String {
    let mut text = element.text_content().unwrap_or_default();
    for key in ["value", "title", "aria-label"] {
        if let Ok(value) = js_sys::Reflect::get(element.as_ref(), &JsValue::from_str(key)) {
            if let Some(value) = value.as_string() {
                text.push(' ');
                text.push_str(&value);
            }
        }
        if let Some(value) = element.get_attribute(key) {
            text.push(' ');
            text.push_str(&value);
        }
    }
    text.to_lowercase()
}

fn rel_target(value: &str) -> Option<Element> {
    let document = doc()?;
    for tag in ["link", "a", "area"] {
        let elements = document.query_selector_all(tag).ok()?;
        for i in 0..elements.length() {
            let Some(node) = elements.item(i) else {
                continue;
            };
            let Ok(element) = node.dyn_into::<Element>() else {
                continue;
            };
            if element
                .get_attribute("rel")
                .is_some_and(|rel| rel.to_lowercase() == value)
            {
                return Some(element);
            }
        }
    }
    None
}

fn pattern_link(patterns: &str) -> Option<Element> {
    let document = doc()?;
    let patterns = patterns
        .split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(str::to_lowercase)
        .collect::<Vec<_>>();
    if patterns.is_empty() {
        return None;
    }
    let nodes = document
        .query_selector_all("a,area,[onclick],[role='link'],[class*='button']")
        .ok()?;
    let mut candidates = Vec::new();
    for i in (0..nodes.length()).rev() {
        let Some(node) = nodes.item(i) else {
            continue;
        };
        let Ok(element) = node.dyn_into::<Element>() else {
            continue;
        };
        if !should_scroll_element(&element, "y") {
            continue;
        }
        let text = element_text_for_match(&element);
        let Some(pattern_index) = patterns.iter().position(|pattern| text.contains(pattern)) else {
            continue;
        };
        let words = text.split_whitespace().count().max(1);
        candidates.push((words, pattern_index, candidates.len(), element));
    }
    candidates.sort_by_key(|a| (a.0, a.1, a.2));
    candidates
        .into_iter()
        .map(|(_, _, _, element)| element)
        .next()
}

fn follow_element(element: Element) {
    if element.tag_name().eq_ignore_ascii_case("link") {
        if let Some(url) = element.get_attribute("href") {
            set_location_href(&url);
        }
        return;
    }
    element.scroll_into_view();
    if let Some(html) = element.dyn_ref::<HtmlElement>() {
        html.click();
    } else if let Some(url) = element.get_attribute("href") {
        set_location_href(&url);
    }
}

fn follow_pattern(pattern: &str) {
    let (rel, setting) = if pattern == "previous" {
        ("prev", "previousPatterns")
    } else {
        ("next", "nextPatterns")
    };
    let patterns = setting_value(setting, Value::String(String::new()))
        .as_str()
        .unwrap_or("")
        .to_string();
    if let Some(target) = rel_target(rel).or_else(|| pattern_link(&patterns)) {
        follow_element(target);
    }
}

async fn send_open_url(url: String, new_tab: bool) {
    if new_tab {
        let _ = send_runtime_message(
            to_js(json!({"type":"rs_vimium", "command": "open-url", "url": url, "active": true}))
                .unwrap_or(JsValue::NULL),
        )
        .await;
    } else {
        set_location_href(&url);
    }
}

fn clipboard_object() -> Option<JsValue> {
    let window = win()?;
    let navigator = js_sys::Reflect::get(window.as_ref(), &JsValue::from_str("navigator")).ok()?;
    js_sys::Reflect::get(&navigator, &JsValue::from_str("clipboard")).ok()
}

fn clipboard_call(name: &str, arg: Option<JsValue>) -> Option<js_sys::Promise> {
    let clipboard = clipboard_object()?;
    let function = js_sys::Reflect::get(&clipboard, &JsValue::from_str(name))
        .ok()?
        .dyn_into::<js_sys::Function>()
        .ok()?;
    let value = if let Some(arg) = arg {
        function.call1(&clipboard, &arg).ok()?
    } else {
        function.call0(&clipboard).ok()?
    };
    value.dyn_into::<js_sys::Promise>().ok()
}

fn copy_current_url() {
    let Some(url) = location_href() else {
        return;
    };
    let label = if url.len() > 40 {
        format!("{}...", &url[..38])
    } else {
        url.clone()
    };
    spawn_local(async move {
        if let Some(promise) = clipboard_call("writeText", Some(JsValue::from_str(&url))) {
            let _ = JsFuture::from(promise).await;
            show_hud(&format!("Yanked {}", label));
        }
    });
}

fn open_clipboard(new_tab: bool) {
    spawn_local(async move {
        let Some(promise) = clipboard_call("readText", None) else {
            return;
        };
        let Ok(value) = JsFuture::from(promise).await else {
            return;
        };
        let Some(url) = value.as_string() else {
            return;
        };
        if !url.trim().is_empty() {
            send_open_url(url, new_tab).await;
        }
    });
}

fn toggle_view_source() {
    let Some(current) = location_href() else {
        return;
    };
    let url = current
        .strip_prefix("view-source:")
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| format!("view-source:{current}"));
    spawn_local(async move {
        send_open_url(url, true).await;
    });
}

fn activate_hints(action: &str) {
    clear_overlays();
    let Some(document) = doc() else {
        return;
    };
    let selector = "a[href],button,input:not([type='hidden']),textarea,select,summary,[role='button'],[onclick],[contenteditable='true'],[tabindex]:not([tabindex='-1'])";
    let Ok(nodes) = document.query_selector_all(selector) else {
        return;
    };
    let scroll_x = win().and_then(|w| w.scroll_x().ok()).unwrap_or(0.0);
    let scroll_y = win().and_then(|w| w.scroll_y().ok()).unwrap_or(0.0);
    let mut targets = Vec::new();
    let chars = setting_value(
        "linkHintCharacters",
        Value::String("sadfjklewcmpgh".to_string()),
    )
    .as_str()
    .unwrap_or("sadfjklewcmpgh")
    .to_string();
    for i in 0..nodes.length().min(600) {
        let Some(node) = nodes.item(i) else {
            continue;
        };
        let Ok(target) = node.dyn_into::<Element>() else {
            continue;
        };
        if !visible(&target) {
            continue;
        }
        let rect = target.get_bounding_client_rect();
        targets.push((target, rect.left(), rect.top()));
    }
    let labels = hint_labels_with_chars(targets.len(), &chars);
    let mut hints = Vec::new();
    for ((target, left, top), label) in targets.into_iter().zip(labels) {
        let Ok(marker) = document.create_element("span") else {
            continue;
        };
        marker.set_class_name("vc-hint");
        set_text(&marker, &label);
        if let Some(style) = marker.dyn_ref::<HtmlElement>().map(|el| el.style()) {
            let _ = style.set_property("left", &format!("{}px", (left + scroll_x).max(2.0)));
            let _ = style.set_property("top", &format!("{}px", (top + scroll_y).max(2.0)));
        }
        if let Some(root) = document.document_element() {
            append(&root, &marker);
        }
        hints.push(HintDom {
            label,
            marker,
            target,
        });
    }
    CONTENT_STATE.with(|state| {
        let mut state = state.borrow_mut();
        state.key_state.mode = "hints".to_string();
        state.hints = hints;
        state.hint_input.clear();
        state.hint_action = action.to_string();
    });
}

fn update_hints(key: &str) {
    CONTENT_STATE.with(|state| {
        let mut state = state.borrow_mut();
        state.hint_input.push_str(&key.to_lowercase());
        let input = state.hint_input.clone();
        let mut selected = None;
        let mut remaining = 0usize;
        for (i, hint) in state.hints.iter().enumerate() {
            let matched = hint.label.starts_with(&input);
            let _ = hint
                .marker
                .class_list()
                .toggle_with_force("vc-hint-dim", !matched);
            if matched {
                remaining += 1;
                selected = Some(i);
            }
            if hint.label == input && remaining == 1 {
                selected = Some(i);
            }
        }
        if remaining == 1 {
            if let Some(hint) = selected.and_then(|idx| state.hints.get(idx)) {
                let target = hint.target.clone();
                let action = state.hint_action.clone();
                drop(state);
                activate_hint_target(target, &action);
                clear_hints();
            }
        }
    });
}

fn activate_hint_target(target: Element, action: &str) {
    let href = target
        .get_attribute("href")
        .or_else(|| target.get_attribute("src"));
    match action {
        "new-tab" | "foreground-tab" | "queue" | "incognito" => {
            if let Some(url) = href {
                spawn_local(async move {
                    send_open_url(url, true).await;
                });
            } else if let Some(el) = target.dyn_ref::<HtmlElement>() {
                let _ = el.focus();
                el.click();
            }
        }
        "copy-url" => {
            if let Some(url) = href {
                spawn_local(async move {
                    if let Some(promise) =
                        clipboard_call("writeText", Some(JsValue::from_str(&url)))
                    {
                        let _ = JsFuture::from(promise).await;
                        show_hud("Yanked link URL.");
                    }
                });
            }
        }
        _ => {
            if let Some(el) = target.dyn_ref::<HtmlElement>() {
                let _ = el.focus();
                el.click();
            } else if let Some(url) = href {
                set_location_href(&url);
            }
        }
    }
}

fn show_vomnibar(mode: &str, new_tab: bool, prefill: String) {
    clear_overlays();
    let Some(document) = doc() else {
        return;
    };
    let Ok(bar) = document.create_element("div") else {
        return;
    };
    bar.set_class_name("vc-vomnibar");
    bar.set_inner_html(r#"<div class="vc-vomnibar-box"><input class="vc-vomnibar-input" type="search" autocomplete="off" autocapitalize="none" spellcheck="false" placeholder="Search bookmarks, history, and tabs"><ul class="vc-vomnibar-list"></ul></div>"#);
    if let Some(root) = document.document_element() {
        append(&root, &bar);
    }
    let input = bar
        .query_selector("input")
        .ok()
        .flatten()
        .and_then(|el| el.dyn_into::<HtmlInputElement>().ok());
    let list = bar.query_selector("ul").ok().flatten();
    let Some(input) = input else {
        return;
    };
    let Some(list) = list else {
        return;
    };
    input.set_value(&prefill);
    let _ = input.focus();
    if !prefill.is_empty() || mode != "full" {
        refresh_vomnibar_list(input.value(), mode.to_string(), list.clone());
    }
    install_vomnibar_input(input, list, mode.to_string(), new_tab);
}

fn install_vomnibar_input(input: HtmlInputElement, list: Element, mode: String, new_tab: bool) {
    let input_for_input = input.clone();
    let list_for_input = list.clone();
    let mode_for_input = mode.clone();
    let input_closure = Closure::<dyn FnMut(web_sys::Event)>::wrap(Box::new(move |_event| {
        refresh_vomnibar_list(
            input_for_input.value(),
            mode_for_input.clone(),
            list_for_input.clone(),
        );
    }));
    let _ = input.add_event_listener_with_callback("input", input_closure.as_ref().unchecked_ref());
    input_closure.forget();

    let input_for_key = input.clone();
    let list_for_key = list.clone();
    let key_closure =
        Closure::<dyn FnMut(KeyboardEvent)>::wrap(Box::new(move |event: KeyboardEvent| {
            let key = key_name_from_event(&event);
            match key.as_str() {
                "Esc" => {
                    event.prevent_default();
                    event.stop_propagation();
                    clear_overlays();
                }
                "down" => {
                    event.prevent_default();
                    event.stop_propagation();
                    move_vomnibar_selection(&list_for_key, 1);
                }
                "up" => {
                    event.prevent_default();
                    event.stop_propagation();
                    move_vomnibar_selection(&list_for_key, -1);
                }
                "enter" => {
                    event.prevent_default();
                    event.stop_propagation();
                    let selected_url = selected_vomnibar_url(&list_for_key);
                    let query = input_for_key.value();
                    spawn_local(async move {
                        let url = selected_url.or_else(|| {
                            resolve_navigable(&query).ok().and_then(|value| {
                                from_js(value)
                                    .get("url")
                                    .and_then(Value::as_str)
                                    .map(str::to_string)
                            })
                        });
                        if let Some(url) = url {
                            send_open_url(url, new_tab).await;
                            clear_overlays();
                        }
                    });
                }
                _ => {}
            }
        }));
    let _ = input.add_event_listener_with_callback("keydown", key_closure.as_ref().unchecked_ref());
    key_closure.forget();

    let list_for_click = list.clone();
    let click_closure =
        Closure::<dyn FnMut(web_sys::Event)>::wrap(Box::new(move |event: web_sys::Event| {
            let Some(item) = vomnibar_item_from_target(event.target()) else {
                return;
            };
            let Some(url) = item.get_attribute("data-url") else {
                return;
            };
            let _ = list_for_click.set_attribute("data-selected", "0");
            event.prevent_default();
            event.stop_propagation();
            spawn_local(async move {
                send_open_url(url, new_tab).await;
                clear_overlays();
            });
        }));
    let _ = list.add_event_listener_with_callback("click", click_closure.as_ref().unchecked_ref());
    click_closure.forget();
}

fn refresh_vomnibar_list(query: String, mode: String, list: Element) {
    spawn_local(async move {
        if let Ok(result) = query_vomnibar(&query, &mode).await {
            let value = from_js(result);
            let items = value
                .get("items")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            render_vomnibar_items(&list, &items);
        }
    });
}

fn render_vomnibar_items(list: &Element, items: &[Value]) {
    let html = items
        .iter()
        .enumerate()
        .map(|(index, item)| {
            let title = item.get("title").and_then(Value::as_str).unwrap_or("");
            let url = item.get("url").and_then(Value::as_str).unwrap_or("");
            let kind = item.get("kind").and_then(Value::as_str).unwrap_or("");
            format!(
                r#"<li class="vc-vomnibar-item{}" data-url="{}"><span class="vc-vomnibar-kind">{}</span><span class="vc-vomnibar-title">{}</span><span class="vc-vomnibar-url">{}</span></li>"#,
                if index == 0 { " vc-vomnibar-selected" } else { "" },
                html_attr(url),
                html_text(kind),
                html_text(title),
                html_text(url)
            )
        })
        .collect::<String>();
    list.set_inner_html(&html);
    let _ = list.set_attribute("data-selected", "0");
}

fn vomnibar_items(list: &Element) -> Option<web_sys::NodeList> {
    list.query_selector_all(".vc-vomnibar-item").ok()
}

fn move_vomnibar_selection(list: &Element, delta: i32) {
    let Some(items) = vomnibar_items(list) else {
        return;
    };
    let len = items.length();
    if len == 0 {
        return;
    }
    let current = list
        .get_attribute("data-selected")
        .and_then(|value| value.parse::<i32>().ok())
        .unwrap_or(0);
    let next = (current + delta).rem_euclid(len as i32);
    set_vomnibar_selection(list, &items, next as u32);
}

fn set_vomnibar_selection(list: &Element, items: &web_sys::NodeList, selected: u32) {
    for i in 0..items.length() {
        if let Some(item) = items
            .item(i)
            .and_then(|node| node.dyn_into::<Element>().ok())
        {
            let _ = item
                .class_list()
                .toggle_with_force("vc-vomnibar-selected", i == selected);
            if i == selected {
                item.scroll_into_view();
            }
        }
    }
    let _ = list.set_attribute("data-selected", &selected.to_string());
}

fn selected_vomnibar_url(list: &Element) -> Option<String> {
    let items = vomnibar_items(list)?;
    let selected = list
        .get_attribute("data-selected")
        .and_then(|value| value.parse::<u32>().ok())
        .unwrap_or(0);
    items
        .item(selected)?
        .dyn_into::<Element>()
        .ok()?
        .get_attribute("data-url")
}

fn vomnibar_item_from_target(target: Option<web_sys::EventTarget>) -> Option<Element> {
    let mut element = target?.dyn_into::<Element>().ok();
    while let Some(current) = element {
        if current.class_list().contains("vc-vomnibar-item") {
            return Some(current);
        }
        element = current.parent_element();
    }
    None
}

fn html_text(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn html_attr(text: &str) -> String {
    html_text(text).replace('"', "&quot;")
}

fn show_find() {
    clear_overlays();
    let Some(document) = doc() else {
        return;
    };
    CONTENT_STATE.with(|state| {
        let mut state = state.borrow_mut();
        state.key_state.mode = "find".to_string();
        state.find_matches.clear();
        state.find_active_index = 0;
    });
    let Ok(panel) = document.create_element("div") else {
        return;
    };
    panel.set_class_name("vc-find");
    panel.set_inner_html(
        r#"<input type="search" autocomplete="off" autocapitalize="none" spellcheck="false" class="vc-find-input"><span class="vc-find-count">0/0</span>"#,
    );
    if let Some(root) = document.document_element() {
        append(&root, &panel);
    }
    if let Ok(Some(input_el)) = panel.query_selector("input") {
        if let Some(input) = input_el.dyn_ref::<HtmlInputElement>() {
            let previous = CONTENT_STATE.with(|state| state.borrow().last_find_query.clone());
            input.set_value(&previous);
            let _ = input.focus();
            if !previous.is_empty() {
                run_find_in_place(&previous);
                let _ = input.focus();
            }
            let input_for_input = input.clone();
            let input_closure =
                Closure::<dyn FnMut(web_sys::Event)>::wrap(Box::new(move |_event| {
                    let query = input_for_input.value();
                    CONTENT_STATE.with(|state| {
                        state.borrow_mut().last_find_query = query.clone();
                    });
                    run_find_in_place(&query);
                }));
            let _ = input
                .add_event_listener_with_callback("input", input_closure.as_ref().unchecked_ref());
            input_closure.forget();

            let key_closure =
                Closure::<dyn FnMut(KeyboardEvent)>::wrap(Box::new(move |event: KeyboardEvent| {
                    let key = key_name_from_event(&event);
                    match key.as_str() {
                        "enter" => {
                            event.prevent_default();
                            event.stop_propagation();
                            CONTENT_STATE.with(|state| {
                                let query = input_value_from_event_target(&event)
                                    .unwrap_or_else(find_query);
                                let mut state = state.borrow_mut();
                                state.last_find_query = query;
                                state.key_state.mode = "normal".to_string();
                            });
                            if let Some(target) = event
                                .target()
                                .and_then(|target| target.dyn_into::<HtmlElement>().ok())
                            {
                                let _ = target.blur();
                            }
                        }
                        "Esc" => {
                            CONTENT_STATE.with(|state| {
                                state.borrow_mut().key_state.mode = "normal".to_string();
                            });
                            clear_overlays();
                        }
                        _ => {}
                    }
                }));
            let _ = input
                .add_event_listener_with_callback("keydown", key_closure.as_ref().unchecked_ref());
            key_closure.forget();
        }
    }
}

fn handle_find_key(key: &str, event: &KeyboardEvent) {
    match key {
        "Esc" => {
            CONTENT_STATE.with(|state| {
                state.borrow_mut().key_state.mode = "normal".to_string();
            });
            clear_overlays();
        }
        "enter" => {
            CONTENT_STATE.with(|state| {
                state.borrow_mut().key_state.mode = "normal".to_string();
            });
            if let Some(target) = event
                .target()
                .and_then(|target| target.dyn_into::<HtmlElement>().ok())
            {
                let _ = target.blur();
            }
        }
        _ => {}
    }
}

fn find_query() -> String {
    if let Some(document) = doc() {
        if let Ok(Some(input)) = document.query_selector(".vc-find-input") {
            if let Some(input) = input.dyn_ref::<HtmlInputElement>() {
                let value = input.value();
                if !value.is_empty() {
                    return value;
                }
            }
        }
    }
    CONTENT_STATE.with(|state| state.borrow().last_find_query.clone())
}

fn input_value_from_event_target(event: &KeyboardEvent) -> Option<String> {
    event
        .target()?
        .dyn_into::<HtmlInputElement>()
        .ok()
        .map(|input| input.value())
}

fn find_next(reverse: bool) {
    let query = find_query();
    if query.is_empty() {
        show_find();
        return;
    }
    let has_matches = CONTENT_STATE.with(|state| !state.borrow().find_matches.is_empty());
    if !has_matches {
        run_find_in_place(&query);
    }
    cycle_find(reverse);
}

fn run_find_in_place(query: &str) {
    let matches = collect_find_matches(query);
    CONTENT_STATE.with(|state| {
        let mut state = state.borrow_mut();
        state.find_matches = matches;
        state.find_active_index = 0;
    });
    render_find_highlights();
}

fn cycle_find(reverse: bool) {
    CONTENT_STATE.with(|state| {
        let mut state = state.borrow_mut();
        let len = state.find_matches.len();
        if len == 0 {
            return;
        }
        if reverse {
            state.find_active_index = (state.find_active_index + len - 1) % len;
        } else {
            state.find_active_index = (state.find_active_index + 1) % len;
        }
    });
    highlight_active_find();
}

fn collect_find_matches(query: &str) -> Vec<FindMatch> {
    if query.is_empty() {
        return Vec::new();
    }
    let ignore_case = !query.chars().any(|ch| ch.is_uppercase());
    let needle = if ignore_case {
        query.to_lowercase()
    } else {
        query.to_string()
    };
    let mut matches = Vec::new();
    if let Some(body) = doc().and_then(|document| document.body()) {
        collect_find_matches_in_node(&body.into(), &needle, ignore_case, &mut matches);
    }
    matches
}

fn collect_find_matches_in_node(
    node: &Node,
    needle: &str,
    ignore_case: bool,
    matches: &mut Vec<FindMatch>,
) {
    if node.node_type() == Node::TEXT_NODE {
        let text = node.text_content().unwrap_or_default();
        for (start, end) in find_match_spans(&text, needle, ignore_case) {
            matches.push(FindMatch {
                node: node.clone(),
                start,
                end,
            });
        }
        return;
    }
    if node.node_type() != Node::ELEMENT_NODE {
        return;
    }
    let element = node.dyn_ref::<Element>();
    if matches!(
        element.map(|el| el.tag_name().to_lowercase()).as_deref(),
        Some("script" | "style" | "noscript" | "textarea" | "input")
    ) {
        return;
    }
    if let Some(element) = element {
        if element.get_attribute("class").is_some_and(|classes| {
            classes
                .split_whitespace()
                .any(|class| class.starts_with("vc-"))
        }) {
            return;
        }
        if !visible_or_contents(element) {
            return;
        }
    }
    let children = node.child_nodes();
    for i in 0..children.length() {
        if let Some(child) = children.item(i) {
            collect_find_matches_in_node(&child, needle, ignore_case, matches);
        }
    }
}

fn find_match_spans(text: &str, needle: &str, ignore_case: bool) -> Vec<(u32, u32)> {
    if needle.is_empty() {
        return Vec::new();
    }
    let haystack: Vec<char> = if ignore_case {
        text.to_lowercase().chars().collect()
    } else {
        text.chars().collect()
    };
    let needle: Vec<char> = needle.chars().collect();
    if needle.is_empty() || needle.len() > haystack.len() {
        return Vec::new();
    }
    let original: Vec<char> = text.chars().collect();
    let mut utf16_offsets = Vec::with_capacity(original.len() + 1);
    let mut offset = 0u32;
    utf16_offsets.push(offset);
    for ch in &original {
        offset += ch.len_utf16() as u32;
        utf16_offsets.push(offset);
    }
    let mut spans = Vec::new();
    for i in 0..=(haystack.len() - needle.len()) {
        if haystack[i..i + needle.len()] == needle[..] {
            spans.push((utf16_offsets[i], utf16_offsets[i + needle.len()]));
        }
    }
    spans
}

fn visible_or_contents(element: &Element) -> bool {
    if visible(element) {
        return true;
    }
    win()
        .and_then(|window| window.get_computed_style(element).ok().flatten())
        .and_then(|style| style.get_property_value("display").ok())
        .as_deref()
        == Some("contents")
}

fn highlight_active_find() {
    render_find_highlights();
    scroll_active_find_to_view();
}

fn active_find_range() -> Option<Range> {
    let active = CONTENT_STATE.with(|state| {
        let state = state.borrow();
        state.find_matches.get(state.find_active_index).cloned()
    });
    let active = active?;
    let document = doc()?;
    let Ok(range) = document.create_range() else {
        return None;
    };
    let _ = range.set_start(&active.node, active.start);
    let _ = range.set_end(&active.node, active.end);
    Some(range)
}

fn scroll_active_find_to_view() {
    let Some(range) = active_find_range() else {
        return;
    };
    scroll_range_to_view(&range);
}

fn current_selection_range() -> Option<Range> {
    let selection = win()?.get_selection().ok()??;
    if selection.range_count() == 0 {
        return None;
    }
    selection.get_range_at(0).ok()
}

fn render_find_highlights() {
    clear_find_highlights();
    let Some(active_index) = CONTENT_STATE.with(|state| {
        let state = state.borrow();
        if state.find_matches.is_empty() {
            None
        } else {
            Some(state.find_active_index)
        }
    }) else {
        set_find_counter(0, 0);
        return;
    };
    let ranges = CONTENT_STATE.with(|state| state.borrow().find_matches.clone());
    set_find_counter(active_index + 1, ranges.len());
    let inactive = js_sys::Array::new();
    let active = js_sys::Array::new();
    for (index, item) in ranges.iter().enumerate() {
        let Some(range) = range_for_find_match(item) else {
            continue;
        };
        if index == active_index {
            active.push(&range);
        } else {
            inactive.push(&range);
        }
    }
    set_css_highlight("rs-vimium-find", &inactive);
    set_css_highlight("rs-vimium-find-active", &active);
    scroll_active_find_to_view();
}

fn range_for_find_match(item: &FindMatch) -> Option<Range> {
    let range = doc()?.create_range().ok()?;
    let _ = range.set_start(&item.node, item.start);
    let _ = range.set_end(&item.node, item.end);
    Some(range)
}

fn set_css_highlight(name: &str, ranges: &js_sys::Array) {
    let global = js_sys::global();
    let Ok(highlight_ctor) = js_sys::Reflect::get(&global, &JsValue::from_str("Highlight"))
        .and_then(|value| value.dyn_into::<js_sys::Function>())
    else {
        return;
    };
    let Ok(highlight) = js_sys::Reflect::construct(&highlight_ctor, ranges) else {
        return;
    };
    if let Some(highlights) = css_highlights() {
        if let Ok(set) = js_sys::Reflect::get(&highlights, &JsValue::from_str("set"))
            .and_then(|value| value.dyn_into::<js_sys::Function>())
        {
            let _ = set.call2(&highlights, &JsValue::from_str(name), &highlight);
        }
    }
}

fn clear_find_highlights() {
    let Some(highlights) = css_highlights() else {
        return;
    };
    if let Ok(delete) = js_sys::Reflect::get(&highlights, &JsValue::from_str("delete"))
        .and_then(|value| value.dyn_into::<js_sys::Function>())
    {
        let _ = delete.call1(&highlights, &JsValue::from_str("rs-vimium-find"));
        let _ = delete.call1(&highlights, &JsValue::from_str("rs-vimium-find-active"));
    }
}

fn css_highlights() -> Option<JsValue> {
    let css = js_sys::Reflect::get(&js_sys::global(), &JsValue::from_str("CSS")).ok()?;
    js_sys::Reflect::get(&css, &JsValue::from_str("highlights")).ok()
}

fn scroll_range_to_view(range: &Range) {
    let rect = range.get_bounding_client_rect();
    let Some(window) = win() else {
        return;
    };
    let height = window
        .inner_height()
        .ok()
        .and_then(|value| value.as_f64())
        .unwrap_or(600.0);
    if rect.top() < 0.0 || rect.bottom() > height {
        let target =
            window.scroll_y().unwrap_or(0.0) + rect.top() + rect.height() / 2.0 - height / 2.0;
        window.scroll_to_with_x_and_y(window.scroll_x().unwrap_or(0.0), target);
    }
}

fn set_find_counter(active: usize, total: usize) {
    let Some(document) = doc() else {
        return;
    };
    if let Ok(Some(counter)) = document.query_selector(".vc-find-count") {
        counter.set_text_content(Some(&format!("{active}/{total}")));
    }
}

fn selection_value() -> Option<JsValue> {
    win()?.get_selection().ok()??.dyn_into::<JsValue>().ok()
}

fn selection_text() -> String {
    let Some(selection) = selection_value() else {
        return String::new();
    };
    js_sys::Reflect::get(&selection, &JsValue::from_str("toString"))
        .ok()
        .and_then(|value| value.dyn_into::<js_sys::Function>().ok())
        .and_then(|function| function.call0(&selection).ok())
        .and_then(|value| value.as_string())
        .unwrap_or_default()
}

fn selection_call(name: &str, args: &[JsValue]) -> bool {
    let Some(selection) = selection_value() else {
        return false;
    };
    let Ok(value) = js_sys::Reflect::get(&selection, &JsValue::from_str(name)) else {
        return false;
    };
    let Ok(function) = value.dyn_into::<js_sys::Function>() else {
        return false;
    };
    function
        .apply(&selection, &js_sys::Array::from_iter(args.iter().cloned()))
        .is_ok()
}

fn selection_modify(alter: &str, direction: &str, granularity: &str) -> bool {
    selection_call(
        "modify",
        &[
            JsValue::from_str(alter),
            JsValue::from_str(direction),
            JsValue::from_str(granularity),
        ],
    )
}

fn first_text_node(node: &Node) -> Option<(Node, u32)> {
    if node.node_type() == Node::TEXT_NODE {
        let text = node.text_content().unwrap_or_default();
        if !text.trim().is_empty() {
            let offset = text.chars().take_while(|ch| ch.is_whitespace()).count() as u32;
            return Some((node.clone(), offset));
        }
    }
    if node.node_type() == Node::ELEMENT_NODE {
        if let Some(element) = node.dyn_ref::<Element>() {
            if matches!(
                element.tag_name().to_lowercase().as_str(),
                "script" | "style" | "noscript" | "textarea" | "input"
            ) || !visible_or_contents(element)
            {
                return None;
            }
        }
    }
    let children = node.child_nodes();
    for i in 0..children.length() {
        if let Some(child) = children.item(i) {
            if let Some(found) = first_text_node(&child) {
                return Some(found);
            }
        }
    }
    None
}

fn establish_visual_selection(line_mode: bool) {
    let has_range = win()
        .and_then(|window| window.get_selection().ok())
        .flatten()
        .is_some_and(|selection| selection.range_count() > 0 && !selection_text().is_empty());
    if !has_range {
        let Some(document) = doc() else {
            return;
        };
        let Some(body) = document.body() else {
            return;
        };
        let body_node: Node = body.into();
        let Some((node, offset)) = first_text_node(&body_node) else {
            show_hud("Create a selection before entering visual mode.");
            return;
        };
        let Ok(range) = document.create_range() else {
            return;
        };
        let _ = range.set_start(&node, offset);
        let _ = range.set_end(&node, offset);
        if let Some(selection) = win()
            .and_then(|window| window.get_selection().ok())
            .flatten()
        {
            let _ = selection.remove_all_ranges();
            let _ = selection.add_range(&range);
        }
        let _ = selection_modify("extend", "forward", "character");
    }
    if line_mode {
        let _ = selection_modify("extend", "backward", "lineboundary");
        let _ = selection_modify("extend", "forward", "lineboundary");
    }
}

fn apply_visual_move(effect: &Value) {
    let direction = effect
        .get("direction")
        .and_then(Value::as_str)
        .unwrap_or("forward");
    let granularity = effect
        .get("granularity")
        .and_then(Value::as_str)
        .unwrap_or("character");
    let granularity = match granularity {
        "vimword" | "vimword-end" => "word",
        "document-boundary" => "documentboundary",
        other => other,
    };
    let count = effect
        .get("count")
        .and_then(Value::as_i64)
        .unwrap_or(1)
        .max(1);
    for _ in 0..count {
        let _ = selection_modify("extend", direction, granularity);
    }
    let line_mode = CONTENT_STATE.with(|state| state.borrow().visual_line_mode);
    if line_mode {
        let _ = selection_modify("extend", direction, "lineboundary");
    }
    if let Some(range) = current_selection_range() {
        scroll_range_to_view(&range);
    }
}

fn copy_selection() {
    let text = selection_text();
    let len = text.chars().count();
    if text.is_empty() {
        show_hud("Nothing selected.");
        return;
    }
    spawn_local(async move {
        if let Some(promise) = clipboard_call("writeText", Some(JsValue::from_str(&text))) {
            let _ = JsFuture::from(promise).await;
        }
        show_hud(&format!(
            "Yanked {} character{}.",
            len,
            if len == 1 { "" } else { "s" }
        ));
    });
}

fn exit_visual_mode(collapse_to_focus: bool) {
    if collapse_to_focus {
        let _ = selection_call("collapseToEnd", &[]);
    } else {
        let _ = selection_call("collapseToStart", &[]);
    }
    CONTENT_STATE.with(|state| {
        let mut state = state.borrow_mut();
        state.key_state.mode = "normal".to_string();
        state.visual_line_mode = false;
    });
}

fn is_top_frame() -> bool {
    let Some(window) = win() else {
        return true;
    };
    let top = js_sys::Reflect::get(window.as_ref(), &JsValue::from_str("top")).ok();
    top.as_ref()
        .is_none_or(|top| js_sys::Object::is(window.as_ref(), top))
}

fn focus_this_frame(highlight: bool) {
    if let Some(window) = win() {
        let _ = window.focus();
    }
    if let Some(document) = doc() {
        if let Some(body) = document.body() {
            let _ = body.focus();
        }
    }
    if highlight {
        show_hud("Frame focused.");
    }
}

fn apply_content_effect(effect: Value) {
    let kind = effect.get("kind").and_then(Value::as_str).unwrap_or("");
    match kind {
        "scroll" => {
            let x = effect.get("x").and_then(Value::as_f64).unwrap_or(0.0);
            let y = effect.get("y").and_then(Value::as_f64).unwrap_or(0.0);
            if x != 0.0 {
                scroll_element_by("x", x);
            }
            if y != 0.0 {
                scroll_element_by("y", y);
            }
        }
        "scroll-step" => {
            let axis = effect.get("axis").and_then(Value::as_str).unwrap_or("y");
            let dir = effect
                .get("direction")
                .and_then(Value::as_f64)
                .unwrap_or(1.0);
            let count = effect.get("count").and_then(Value::as_f64).unwrap_or(1.0);
            scroll_element_by(axis, setting_f64("scrollStepSize", 60.0) * dir * count);
        }
        "half-scroll" => {
            let dir = effect
                .get("direction")
                .and_then(Value::as_f64)
                .unwrap_or(1.0);
            let count = effect.get("count").and_then(Value::as_f64).unwrap_or(1.0);
            scroll_element_by("y", viewport_size("y") * 0.5 * dir * count);
        }
        "full-scroll" => {
            let dir = effect
                .get("direction")
                .and_then(Value::as_f64)
                .unwrap_or(1.0);
            let count = effect.get("count").and_then(Value::as_f64).unwrap_or(1.0);
            scroll_element_by("y", viewport_size("y") * dir * count);
        }
        "scroll-top" => {
            let count = effect.get("count").and_then(Value::as_f64).unwrap_or(1.0);
            scroll_element_to(
                "y",
                (count - 1.0).max(0.0) * setting_f64("scrollStepSize", 60.0),
            );
        }
        "scroll-bottom" => scroll_element_to("y", f64::MAX),
        "scroll-left" => scroll_element_to("x", 0.0),
        "scroll-right" => scroll_element_to("x", f64::MAX),
        "clear-overlays" => clear_overlays(),
        "help" => show_help(),
        "hints" | "hints-general" | "hints-queue" | "hints-download" | "hints-incognito"
        | "hints-copy-url" => {
            let action = match kind {
                "hints"
                    if effect
                        .get("newTab")
                        .and_then(Value::as_bool)
                        .unwrap_or(false) =>
                {
                    if effect
                        .get("foreground")
                        .and_then(Value::as_bool)
                        .unwrap_or(false)
                    {
                        "foreground-tab"
                    } else {
                        "new-tab"
                    }
                }
                "hints-queue" => "queue",
                "hints-incognito" => "incognito",
                "hints-copy-url" => "copy-url",
                "hints-download" => "download",
                _ => "current",
            };
            activate_hints(action);
        }
        "find" => show_find(),
        "find-next" => find_next(
            effect
                .get("reverse")
                .and_then(Value::as_bool)
                .unwrap_or(false),
        ),
        "vomnibar" | "vomnibar-bookmarks" | "vomnibar-tabs" | "vomnibar-edit-url" => {
            let mode = match kind {
                "vomnibar-bookmarks" => "bookmarks",
                "vomnibar-tabs" => "tabs",
                _ => "full",
            };
            let options = effect.get("options").unwrap_or(&Value::Null);
            let prefill = options
                .get("query")
                .and_then(Value::as_str)
                .map(str::to_string)
                .or_else(|| (kind == "vomnibar-edit-url").then(location_href).flatten())
                .unwrap_or_default();
            show_vomnibar(
                mode,
                effect
                    .get("newTab")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                prefill,
            );
        }
        "reload" => {
            if let Some(w) = win() {
                let _ = w.location().reload();
            }
        }
        "history-back" => {
            if let Some(w) = win() {
                let _ = w.history().and_then(|h| h.back());
            }
        }
        "history-forward" => {
            if let Some(w) = win() {
                let _ = w.history().and_then(|h| h.forward());
            }
        }
        "go-up" => {
            let count = effect.get("count").and_then(Value::as_i64).unwrap_or(1);
            if let Some(url) = location_href().and_then(|url| key_handler::go_up_url(&url, count)) {
                set_location_href(&url);
            }
        }
        "go-root" => {
            if let Some(url) = location_href().and_then(|url| key_handler::root_url(&url)) {
                set_location_href(&url);
            }
        }
        "focus-input" => focus_input(effect.get("count").and_then(Value::as_i64).unwrap_or(1)),
        "follow-pattern" => {
            follow_pattern(
                effect
                    .get("pattern")
                    .and_then(Value::as_str)
                    .unwrap_or("next"),
            );
        }
        "copy-url" => copy_current_url(),
        "open-clipboard" => open_clipboard(
            effect
                .get("newTab")
                .and_then(Value::as_bool)
                .unwrap_or(false),
        ),
        "view-source" => toggle_view_source(),
        "create-mark" => {
            CONTENT_STATE.with(|state| {
                let mut state = state.borrow_mut();
                state.mark_mode = Some(MarkMode::Create);
                state.key_state.mode = "mark".to_string();
            });
            show_hud("Create mark...");
        }
        "goto-mark" => {
            CONTENT_STATE.with(|state| {
                let mut state = state.borrow_mut();
                state.mark_mode = Some(MarkMode::Goto);
                state.key_state.mode = "mark".to_string();
            });
            show_hud("Go to mark...");
        }
        "enter-visual" => {
            let line_mode = effect
                .get("mode")
                .and_then(Value::as_str)
                .is_some_and(|mode| mode == "visual-line");
            CONTENT_STATE.with(|state| {
                let mut state = state.borrow_mut();
                state.key_state.mode = "visual".to_string();
                state.visual_line_mode = line_mode;
            });
            establish_visual_selection(line_mode);
            show_hud(if line_mode {
                "Visual line mode."
            } else {
                "Visual mode."
            });
        }
        "visual-move" => apply_visual_move(&effect),
        "visual-copy" => {
            copy_selection();
            exit_visual_mode(false);
        }
        "exit-visual" => exit_visual_mode(true),
        "cycle-frame" => focus_this_frame(true),
        "focus-main-frame" => {
            if is_top_frame() {
                focus_this_frame(true);
            } else {
                spawn_local(async {
                    let _ = send_runtime_message(
                        to_js(json!({"type":"rs_vimium", "command": "focus-main-frame"}))
                            .unwrap_or(JsValue::NULL),
                    )
                    .await;
                });
            }
        }
        "background" => {
            let command = effect
                .get("command")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            let args = effect.clone();
            spawn_local(async move {
                let _ = send_runtime_message(
                    to_js(json!({"type":"rs_vimium", "command": command, "args": args}))
                        .unwrap_or(JsValue::NULL),
                )
                .await;
            });
        }
        "pass-next-key" => {
            CONTENT_STATE.with(|state| {
                state.borrow_mut().pass_next_key = true;
            });
            show_hud("Pass next key...");
        }
        _ => {}
    }
}

fn refresh_content_settings() {
    spawn_local(async {
        let _ = settings_seed().await;
        if let Ok(resp) = settings_get().await {
            let settings = from_js(resp)
                .get("settings")
                .cloned()
                .unwrap_or_else(|| json!({}));
            let mapped_keys = COMMAND_REGISTRY
                .parse_effective_key_mappings(
                    settings
                        .get("keyMappings")
                        .and_then(Value::as_str)
                        .unwrap_or(""),
                )
                .key_to_mapped_key;
            CONTENT_STATE.with(|state| {
                let mut state = state.borrow_mut();
                state.settings = settings;
                state.mapped_keys = mapped_keys;
                state.enabled = true;
            });
        }
    });
}

#[wasm_bindgen]
pub fn content_main() {
    CONTENT_STATE.with(|state| {
        let mut state = state.borrow_mut();
        state.enabled = true;
        state.key_state = key_handler::KeyState::new();
    });
    refresh_content_settings();
    if let Ok(guard) = browser_runtime::on_message_value(|message, _sender| {
        let msg = from_js(message);
        if msg.get("type").and_then(Value::as_str) == Some("settings:changed") {
            refresh_content_settings();
            return;
        }
        if msg.get("type").and_then(Value::as_str) == Some("rs_vimium") {
            match msg.get("command").and_then(Value::as_str).unwrap_or("") {
                "set-scroll-position" if is_top_frame() => {
                    set_previous_position();
                    scroll_to_mark(&msg);
                }
                "focus-this-frame" => {
                    let top_only = msg.get("topOnly").and_then(Value::as_bool).unwrap_or(false);
                    if !top_only || is_top_frame() {
                        focus_this_frame(
                            msg.get("highlight")
                                .and_then(Value::as_bool)
                                .unwrap_or(false),
                        );
                    }
                }
                _ => {}
            }
        }
    }) {
        EVENT_GUARDS.with(|guards| guards.borrow_mut().push(guard));
    }
    let Some(document) = doc() else {
        return;
    };
    for event_name in ["click", "DOMActivate"] {
        let closure =
            Closure::<dyn FnMut(web_sys::Event)>::wrap(Box::new(move |event: web_sys::Event| {
                if let Some(element) = event.target().and_then(|target| target.dyn_into().ok()) {
                    CONTENT_STATE.with(|state| {
                        state.borrow_mut().activated_element = Some(element);
                    });
                }
            }));
        let _ =
            document.add_event_listener_with_callback(event_name, closure.as_ref().unchecked_ref());
        closure.forget();
    }
    let closure = Closure::<dyn FnMut(KeyboardEvent)>::wrap(Box::new(
        move |event: KeyboardEvent| {
            let key = key_name_from_event(&event);
            if key.is_empty() {
                return;
            }
            let pass_next = CONTENT_STATE.with(|state| {
                let mut state = state.borrow_mut();
                let pass_next = state.pass_next_key;
                state.pass_next_key = false;
                pass_next
            });
            if pass_next {
                return;
            }
            let find_mode = CONTENT_STATE.with(|state| state.borrow().key_state.mode == "find");
            if find_mode {
                if matches!(key.as_str(), "Esc" | "enter") {
                    event.prevent_default();
                    event.stop_propagation();
                    handle_find_key(&key, &event);
                }
                return;
            }
            let hints_mode = CONTENT_STATE.with(|state| state.borrow().key_state.mode == "hints");
            if hints_mode && key != "Esc" {
                event.prevent_default();
                event.stop_propagation();
                if key.len() == 1 {
                    update_hints(&key);
                }
                return;
            }
            let mark_mode = CONTENT_STATE.with(|state| state.borrow().mark_mode.is_some());
            if mark_mode {
                event.prevent_default();
                event.stop_propagation();
                handle_mark_key(&key, &event);
                return;
            }
            let editable = is_editable_target(event.target());
            let state_js = CONTENT_STATE.with(|state| {
            let state = &state.borrow().key_state;
            to_js(json!({"mode": state.mode, "sequence": state.sequence, "countText": state.count_text, "input": state.input})).unwrap_or(JsValue::NULL)
        });
            let Ok(result_js) = content_key(state_js, &key, editable) else {
                return;
            };
            let result = from_js(result_js);
            if let Some(next) = result.get("state") {
                CONTENT_STATE.with(|state| {
                    let mut state = state.borrow_mut();
                    state.key_state.mode = next
                        .get("mode")
                        .and_then(Value::as_str)
                        .unwrap_or("normal")
                        .to_string();
                    state.key_state.sequence = next
                        .get("sequence")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string();
                    state.key_state.count_text = next
                        .get("countText")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string();
                    state.key_state.input = next
                        .get("input")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string();
                });
            }
            if result
                .get("prevent")
                .and_then(Value::as_bool)
                .unwrap_or(false)
            {
                event.prevent_default();
                event.stop_propagation();
            }
            if let Some(effect) = result.get("effect").cloned() {
                apply_content_effect(effect);
            }
        },
    ));
    let _ = document.add_event_listener_with_callback_and_bool(
        "keydown",
        closure.as_ref().unchecked_ref(),
        true,
    );
    closure.forget();
}

const SETTING_KEYS: &[&str] = &[
    "scrollStepSize",
    "smoothScroll",
    "keyMappings",
    "linkHintCharacters",
    "filterLinkHints",
    "hideHud",
    "searchUrl",
    "searchEngines",
    "newTabDestination",
    "newTabCustomUrl",
    "grabBackFocus",
    "regexFindMode",
    "waitForEnterForFilteredHints",
    "helpDialog_showAdvancedCommands",
];

const BOOL_KEYS: &[&str] = &[
    "smoothScroll",
    "filterLinkHints",
    "hideHud",
    "grabBackFocus",
    "regexFindMode",
    "waitForEnterForFilteredHints",
    "helpDialog_showAdvancedCommands",
];

fn set_status(message: &str, is_error: bool) {
    let Some(document) = doc() else {
        return;
    };
    let Some(status) = document.get_element_by_id("status") else {
        return;
    };
    status.set_text_content(Some(message));
    status.set_class_name(if is_error { "status error" } else { "status" });
}

fn render_options(settings: &Value) {
    let Some(document) = doc() else {
        return;
    };
    for key in SETTING_KEYS {
        let Some(element) = document.get_element_by_id(key) else {
            continue;
        };
        if BOOL_KEYS.contains(key) {
            if let Some(input) = element.dyn_ref::<HtmlInputElement>() {
                input.set_checked(settings.get(*key).and_then(Value::as_bool).unwrap_or(false));
            }
        } else if let Some(input) = element.dyn_ref::<HtmlInputElement>() {
            input.set_value(
                &settings
                    .get(*key)
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned)
                    .unwrap_or_else(|| {
                        settings.get(*key).map(Value::to_string).unwrap_or_default()
                    }),
            );
        } else if let Some(select) = element.dyn_ref::<HtmlSelectElement>() {
            select.set_value(settings.get(*key).and_then(Value::as_str).unwrap_or(""));
        } else if let Some(textarea) = element.dyn_ref::<HtmlTextAreaElement>() {
            textarea.set_value(settings.get(*key).and_then(Value::as_str).unwrap_or(""));
        }
    }
}

fn collect_options() -> Value {
    let Some(document) = doc() else {
        return json!({});
    };
    let mut map = serde_json::Map::new();
    for key in SETTING_KEYS {
        let Some(element) = document.get_element_by_id(key) else {
            continue;
        };
        if BOOL_KEYS.contains(key) {
            if let Some(input) = element.dyn_ref::<HtmlInputElement>() {
                map.insert((*key).to_string(), json!(input.checked()));
            }
        } else if let Some(input) = element.dyn_ref::<HtmlInputElement>() {
            if input.type_() == "number" {
                map.insert(
                    (*key).to_string(),
                    json!(input.value().parse::<i64>().unwrap_or(0)),
                );
            } else {
                map.insert((*key).to_string(), json!(input.value()));
            }
        } else if let Some(select) = element.dyn_ref::<HtmlSelectElement>() {
            map.insert((*key).to_string(), json!(select.value()));
        } else if let Some(textarea) = element.dyn_ref::<HtmlTextAreaElement>() {
            map.insert((*key).to_string(), json!(textarea.value()));
        }
    }
    Value::Object(map)
}

fn load_options() {
    spawn_local(async {
        let _ = settings_seed().await;
        match settings_get().await {
            Ok(resp) => {
                let settings = from_js(resp)
                    .get("settings")
                    .cloned()
                    .unwrap_or_else(|| json!({}));
                render_options(&settings);
            }
            Err(error) => set_status(&format!("Error: {error:?}"), true),
        }
    });
}

#[wasm_bindgen]
pub fn options_main() {
    load_options();
    let Some(document) = doc() else {
        return;
    };
    if let Some(save) = document.get_element_by_id("saveBtn") {
        let closure = Closure::<dyn FnMut(web_sys::Event)>::wrap(Box::new(move |_event| {
            let values = collect_options();
            spawn_local(async move {
                match settings_set(to_js(values).unwrap_or(JsValue::NULL)).await {
                    Ok(_) => {
                        let _ = notify_settings_changed().await;
                        load_options();
                        set_status("Settings saved.", false);
                    }
                    Err(error) => set_status(&format!("Error: {error:?}"), true),
                }
            });
        }));
        let _ = save.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref());
        closure.forget();
    }
    if let Some(reset) = document.get_element_by_id("resetBtn") {
        let closure = Closure::<dyn FnMut(web_sys::Event)>::wrap(Box::new(move |_event| {
            spawn_local(async move {
                match settings_clear().await {
                    Ok(_) => {
                        let _ = notify_settings_changed().await;
                        load_options();
                        set_status("Settings reset to defaults.", false);
                    }
                    Err(error) => set_status(&format!("Error: {error:?}"), true),
                }
            });
        }));
        let _ = reset.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref());
        closure.forget();
    }
}

#[wasm_bindgen]
pub fn popup_main() {
    if let Ok(output) = render_popup(JsValue::NULL) {
        let value = from_js(output);
        if let Some(css) = value.get("css").and_then(Value::as_str) {
            if let Some(document) = doc() {
                if let Ok(style) = document.create_element("style") {
                    style.set_text_content(Some(css));
                    if let Some(head) = document.head() {
                        let _ = head.append_child(&style);
                    }
                }
            }
        }
        if let Some(html) = value.get("html").and_then(Value::as_str) {
            if let Some(root) = doc().and_then(|d| d.get_element_by_id("root")) {
                root.set_inner_html(html);
            }
        }
    }
    if let Some(document) = doc() {
        if let Ok(script) = document.create_element("script") {
            let _ = script.set_attribute("src", "../vendor/unocss.js");
            let _ = script.set_attribute("defer", "");
            if let Some(head) = document.head() {
                let _ = head.append_child(&script);
            }
        }
    }
}

#[wasm_bindgen]
pub fn new_tab_main() {
    use wasm_bindgen_futures::JsFuture;
    use web_sys::HtmlInputElement;

    let Some(document) = doc() else { return };
    let Some(window) = win() else { return };
    let Some(input) = document.get_element_by_id("search-input") else { return };
    let Some(input_el) = input.dyn_ref::<HtmlInputElement>() else { return };
    let Some(list) = document.get_element_by_id("suggest-list") else { return };

    let _ = input_el.set_attribute("placeholder", "Search DuckDuckGo...");
    let _ = input_el.focus();

    let input_clone = input_el.clone();
    let list_clone = list.clone();
    let win_clone = window.clone();
    let closure = Closure::<dyn FnMut(web_sys::Event)>::wrap(Box::new(move |_ev| {
        let q = input_clone.value();
        if q.trim().is_empty() {
            list_clone.set_inner_html("");
            return;
        }
        let list2 = list_clone.clone();
        let win2 = win_clone.clone();
        spawn_local(async move {
            let url = format!("https://duckduckgo.com/ac/?q={}&type=list", js_sys::encode_uri_component(&q));
            let Ok(req) = web_sys::Request::new_with_str(&url) else { return };
            let Ok(resp_val) = JsFuture::from(win2.fetch_with_request(&req)).await else { return };
            let Ok(resp) = resp_val.dyn_into::<web_sys::Response>() else { return };
            let Ok(text_promise) = resp.text() else { return };
            let Ok(text_val) = JsFuture::from(text_promise).await else { return };
            let Some(text) = text_val.as_string() else { return };
            let items: Vec<String> = serde_json::from_str(&text).unwrap_or_default();
            let mut html = String::new();
            for item in &items {
                let safe = item.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;");
                html.push_str(&format!(
                    r#"<li class="cursor-pointer px-3 py-2 text-sm text-black border-b border-[#e5e5e5] hover:bg-black hover:text-white" role="option">{safe}</li>"#
                ));
            }
            list2.set_inner_html(&html);
        });
    }));
    let _ = input_el.add_event_listener_with_callback("input", closure.as_ref().unchecked_ref());
    closure.forget();

    let input2 = input_el.clone();
    let form = input_el.form();
    let closure2 = Closure::<dyn FnMut(web_sys::Event)>::wrap(Box::new(move |ev: web_sys::Event| {
        ev.prevent_default();
        let q = input2.value().trim().to_string();
        if !q.is_empty() {
            let url = format!("https://duckduckgo.com/?q={}", js_sys::encode_uri_component(&q));
            let _ = window.location().set_href(&url);
        }
    }));
    if let Some(f) = form {
        let _ = f.add_event_listener_with_callback("submit", closure2.as_ref().unchecked_ref());
    }
    closure2.forget();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hint_labels_match_vimium_prefix_free_generation() {
        assert_eq!(hint_labels_with_chars(2, "ab"), vec!["a", "b"]);
        assert_eq!(hint_labels_with_chars(3, "ab"), vec!["aa", "b", "ab"]);
    }

    #[test]
    fn hint_labels_do_not_assign_prefixes() {
        let labels = hint_labels_with_chars(80, "sadfjklewcmpgh");
        for (i, label) in labels.iter().enumerate() {
            for (j, other) in labels.iter().enumerate() {
                if i != j {
                    assert!(!other.starts_with(label), "{label} prefixes {other}");
                }
            }
        }
    }
}
