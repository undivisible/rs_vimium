mod background;
pub mod commands;
pub mod key_handler;
pub mod settings;
pub mod vomnibar;

use commands::KeyMapRegistry;
use crepuscularity_core::context::{TemplateContext, TemplateValue};
use crepuscularity_web::render_component_file_to_html;
use crepuscularity_webext::wasm::{runtime as browser_runtime, storage, tabs, EventListenerGuard};
use once_cell::sync::Lazy;
use serde::Serialize;
use serde_json::{json, Value};
use settings::UserSettings;
use std::sync::Mutex;
use wasm_bindgen::prelude::*;

const UI_CREPUS: &str = include_str!("../views/ui.crepus");

static COMMAND_REGISTRY: Lazy<KeyMapRegistry> = Lazy::new(KeyMapRegistry::from_defaults);
static USER_SETTINGS: Lazy<Mutex<UserSettings>> = Lazy::new(|| Mutex::new(UserSettings::new()));
static USER_MAPPINGS: Lazy<Mutex<std::collections::HashMap<String, String>>> =
    Lazy::new(|| Mutex::new(std::collections::HashMap::new()));

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
    let pruned = if let Ok(mut s) = USER_SETTINGS.lock() {
        s.merge(stored);
        let pruned = settings::prune_defaults(&s.settings);
        if let Ok(mut mappings) = USER_MAPPINGS.lock() {
            *mappings = COMMAND_REGISTRY.parse_user_mappings(&s.get_str("keyMappings"));
        }
        Some(pruned)
    } else {
        None
    };
    if let Some(pruned) = pruned {
        storage::sync()
            .set(&pruned)
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
    let result = key_handler::handle_key(&state, key, editable, &COMMAND_REGISTRY, &mappings);
    to_js(result)
}

#[wasm_bindgen]
pub fn render_help_overlay(show_advanced: bool) -> String {
    let groups = shortcut_groups_json_val();
    let mut html = String::from(
        r#"<div class="vc-overlay-header"><span>vimium-crepus shortcuts</span><button class="vc-overlay-close" type="button">Esc</button></div><div class="vc-overlay-grid">"#,
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
    const CHARS: &[u8] = b"asdfghjklqwertyuiopzxcvbnm";
    let mut label = String::new();
    let mut value = index;
    loop {
        label.insert(0, CHARS[value % CHARS.len()] as char);
        if value < CHARS.len() {
            break;
        }
        value = value / CHARS.len() - 1;
    }
    label
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

    let selected = exact.or_else(|| {
        if remaining.len() == 1 {
            remaining.first().copied()
        } else {
            None
        }
    });
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

#[wasm_bindgen]
pub fn is_search_query(query: &str) -> bool {
    vomnibar::SearchEngines::is_search_query(query)
}

#[wasm_bindgen]
pub async fn handle_background_message(message: JsValue) -> Result<JsValue, JsValue> {
    let msg = from_js(message);
    let msg_type = msg.get("type").and_then(Value::as_str).unwrap_or("");

    match msg_type {
        "settings:get" => settings_get().await,
        "vimium-crepus" | "" => {
            let command = msg.get("command").and_then(Value::as_str).unwrap_or("");
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
body{margin:0;min-width:400px;font-family:Inter,ui-sans-serif,system-ui,-apple-system,BlinkMacSystemFont,"Segoe UI",sans-serif;background:#111315;color:#f5f0e6}
.vc-popup{display:flex;flex-direction:column;gap:16px;padding:16px;background:linear-gradient(180deg,#16191d 0%,#101214 100%)}
.vc-header{display:flex;align-items:flex-start;justify-content:space-between;gap:12px}
.vc-title{margin:0;font-size:20px;line-height:1;font-weight:800;letter-spacing:0}
.vc-status{font-size:12px;color:#9ca58d;white-space:nowrap}
.vc-grid{display:grid;grid-template-columns:1fr;gap:10px}
.vc-group{border:1px solid rgba(245,240,230,.11);border-radius:8px;background:rgba(255,255,255,.035);overflow:hidden}
.vc-group-title{padding:8px 10px;font-size:11px;font-weight:700;text-transform:uppercase;color:#c7b46a;border-bottom:1px solid rgba(245,240,230,.09)}
.vc-row{display:grid;grid-template-columns:72px 1fr;gap:10px;align-items:center;padding:8px 10px;border-bottom:1px solid rgba(245,240,230,.06)}
.vc-row:last-child{border-bottom:0}
.vc-keys{font-family:"SFMono-Regular",Consolas,monospace;font-size:11px;color:#141414;background:#d8c66f;border-radius:4px;padding:3px 5px;text-align:center}
.vc-label{font-size:12px;color:#ddd7c9;line-height:1.35}
.vc-footer{font-size:11px;line-height:1.4;color:#8d9483}
"#;
use std::cell::RefCell;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use web_sys::{
    Document, Element, HtmlElement, HtmlInputElement, HtmlTextAreaElement, KeyboardEvent, Window,
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
    settings: Value,
}

struct HintDom {
    label: String,
    marker: Element,
    target: Element,
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

fn visible(element: &Element) -> bool {
    let rect = element.get_bounding_client_rect();
    rect.width() > 0.0 && rect.height() > 0.0
}

fn activate_hints() {
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
    let mut hints = Vec::new();
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
        let Ok(marker) = document.create_element("span") else {
            continue;
        };
        marker.set_class_name("vc-hint");
        let label = hint_label(hints.len());
        set_text(&marker, &label);
        if let Some(style) = marker.dyn_ref::<HtmlElement>().map(|el| el.style()) {
            let _ = style.set_property("left", &format!("{}px", (rect.left() + scroll_x).max(2.0)));
            let _ = style.set_property("top", &format!("{}px", (rect.top() + scroll_y).max(2.0)));
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
    });
}

fn update_hints(key: &str) {
    let labels = CONTENT_STATE.with(|state| {
        state
            .borrow()
            .hints
            .iter()
            .map(|hint| hint.label.clone())
            .collect::<Vec<_>>()
    });
    let Ok(labels_js) = to_js(json!(labels)) else {
        return;
    };
    let current = CONTENT_STATE.with(|state| state.borrow().hint_input.clone());
    let Ok(next_js) = update_hint_state(labels_js, &current, key) else {
        return;
    };
    let next = from_js(next_js);
    CONTENT_STATE.with(|state| {
        let mut state = state.borrow_mut();
        state.hint_input = next
            .get("input")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        if let Some(dim) = next.get("dim").and_then(Value::as_array) {
            for (i, item) in dim.iter().enumerate() {
                if let Some(hint) = state.hints.get(i) {
                    let _ = hint
                        .marker
                        .class_list()
                        .toggle_with_force("vc-hint-dim", item.as_bool().unwrap_or(false));
                }
            }
        }
        if let Some(selected) = next
            .get("selected")
            .and_then(Value::as_u64)
            .map(|v| v as usize)
        {
            if let Some(hint) = state.hints.get(selected) {
                if let Some(el) = hint.target.dyn_ref::<HtmlElement>() {
                    el.click();
                }
            }
            drop(state);
            clear_hints();
        }
    });
}

fn show_vomnibar() {
    clear_overlays();
    let Some(document) = doc() else {
        return;
    };
    let Ok(bar) = document.create_element("div") else {
        return;
    };
    bar.set_class_name("vc-vomnibar");
    bar.set_inner_html(r#"<div class="vc-vomnibar-box"><input class="vc-vomnibar-input" type="search" autocomplete="off" placeholder="Search bookmarks, history, and tabs"><ul class="vc-vomnibar-list"></ul></div>"#);
    if let Some(root) = document.document_element() {
        append(&root, &bar);
    }
    if let Ok(Some(input)) = bar.query_selector("input") {
        if let Some(input) = input.dyn_ref::<HtmlElement>() {
            let _ = input.focus();
        }
    }
}

fn show_find() {
    clear_overlays();
    let Some(document) = doc() else {
        return;
    };
    let Ok(form) = document.create_element("form") else {
        return;
    };
    form.set_class_name("vc-find");
    form.set_inner_html(r#"<input type="search" autocomplete="off" class="vc-find-input"><button type="submit" class="vc-find-btn">Find</button>"#);
    if let Some(root) = document.document_element() {
        append(&root, &form);
    }
    if let Ok(Some(input)) = form.query_selector("input") {
        if let Some(input) = input.dyn_ref::<HtmlElement>() {
            let _ = input.focus();
        }
    }
}

fn apply_content_effect(effect: Value) {
    let kind = effect.get("kind").and_then(Value::as_str).unwrap_or("");
    match kind {
        "scroll" => {
            let x = effect.get("x").and_then(Value::as_f64).unwrap_or(0.0);
            let y = effect.get("y").and_then(Value::as_f64).unwrap_or(0.0);
            if let Some(w) = win() {
                w.scroll_by_with_x_and_y(x, y);
            }
        }
        "half-scroll" => {
            let dir = effect
                .get("direction")
                .and_then(Value::as_f64)
                .unwrap_or(1.0);
            let count = effect.get("count").and_then(Value::as_f64).unwrap_or(1.0);
            if let Some(w) = win() {
                let h = w
                    .inner_height()
                    .ok()
                    .and_then(|v| v.as_f64())
                    .unwrap_or(600.0);
                w.scroll_by_with_x_and_y(0.0, h * 0.55 * dir * count);
            }
        }
        "full-scroll" => {
            let dir = effect
                .get("direction")
                .and_then(Value::as_f64)
                .unwrap_or(1.0);
            let count = effect.get("count").and_then(Value::as_f64).unwrap_or(1.0);
            if let Some(w) = win() {
                let h = w
                    .inner_height()
                    .ok()
                    .and_then(|v| v.as_f64())
                    .unwrap_or(600.0);
                w.scroll_by_with_x_and_y(0.0, h * 0.9 * dir * count);
            }
        }
        "scroll-top" => {
            if let Some(w) = win() {
                w.scroll_to_with_x_and_y(0.0, 0.0);
            }
        }
        "scroll-bottom" => {
            if let Some(w) = win() {
                w.scroll_to_with_x_and_y(0.0, 1_000_000.0);
            }
        }
        "scroll-left" => {
            if let Some(w) = win() {
                w.scroll_to_with_x_and_y(0.0, w.scroll_y().unwrap_or(0.0));
            }
        }
        "scroll-right" => {
            if let Some(w) = win() {
                w.scroll_to_with_x_and_y(1_000_000.0, w.scroll_y().unwrap_or(0.0));
            }
        }
        "clear-overlays" => clear_overlays(),
        "help" => show_help(),
        "hints" | "hints-general" | "hints-queue" | "hints-download" | "hints-incognito"
        | "hints-copy-url" => activate_hints(),
        "find" => show_find(),
        "vomnibar" | "vomnibar-bookmarks" | "vomnibar-tabs" | "vomnibar-edit-url" => {
            show_vomnibar()
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
        "background" => {
            let command = effect
                .get("command")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            spawn_local(async move {
                let _ = send_runtime_message(
                    to_js(json!({"type":"vimium-crepus", "command": command}))
                        .unwrap_or(JsValue::NULL),
                )
                .await;
            });
        }
        "pass-next-key" => show_hud("Pass next key..."),
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
            CONTENT_STATE.with(|state| {
                let mut state = state.borrow_mut();
                state.settings = settings;
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
        }
    }) {
        EVENT_GUARDS.with(|guards| guards.borrow_mut().push(guard));
    }
    let Some(document) = doc() else {
        return;
    };
    let closure = Closure::<dyn FnMut(KeyboardEvent)>::wrap(Box::new(
        move |event: KeyboardEvent| {
            let key = key_handler::key_name(&event.key());
            let hints_mode = CONTENT_STATE.with(|state| state.borrow().key_state.mode == "hints");
            if hints_mode && key != "Esc" {
                event.prevent_default();
                event.stop_propagation();
                if key.len() == 1 {
                    update_hints(&key);
                }
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
}
