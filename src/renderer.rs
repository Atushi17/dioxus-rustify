use crate::models::{convert_camel_to_kebab, is_container, val_to_css, ComponentNode};
use dioxus::prelude::*;

#[cfg(target_arch = "wasm32")]
use crate::normalize_href_for_navigate;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;

fn icon_label_to_glyph(name: &str) -> &'static str {
    match name {
        "LayoutDashboard" => "▦",
        "ArrowBigUpIcon" => "↗",
        "BookPlus" => "+",
        "Briefcase" => "▣",
        "Ticket" => "◫",
        "ListTodo" => "☰",
        "Activity" => "◉",
        "BookHeadphones" => "◍",
        "Mail" => "✉",
        "Dock" => "▤",
        "Voicemail" => "◌",
        "BookUser" => "◍",
        "LogIn" => "🔑",
        "UserPlus" => "👤+",
        "User" => "👤",
        "LogOut" => "🚪",
        "ChevronDown" => "▼",
        "ChevronRight" => "▶",
        "Menu" => "☰",
        "Search" => "🔍",
        "Settings" => "⚙",
        "HelpCircle" => "❓",
        "Plus" => "＋",
        _ => "•",
    }
}

fn normalize_logo_src(input: &str) -> String {
    if input.is_empty()
        || input.starts_with("http://")
        || input.starts_with("https://")
        || input.starts_with('/')
    {
        return input.to_string();
    }
    format!("/{}", input)
}

fn is_image_source(src: &str) -> bool {
    let src = src.trim().to_lowercase();
    src.starts_with('/')
        || src.starts_with("http://")
        || src.starts_with("https://")
        || src.starts_with("data:")
        || src.starts_with("blob:")
        || src.contains('/')
        || src.contains('\\')
        || src.ends_with(".png")
        || src.ends_with(".jpg")
        || src.ends_with(".jpeg")
        || src.ends_with(".svg")
        || src.ends_with(".webp")
        || src.ends_with(".gif")
        || src.ends_with(".ico")
}

fn normalize_media_source(raw_src: &str) -> String {
    let src = raw_src.trim();
    if src.is_empty()
        || src.starts_with("data:")
        || src.starts_with("blob:")
        || src.starts_with("mailto:")
        || src.starts_with("tel:")
    {
        return src.to_string();
    }

    if src.starts_with("http://") || src.starts_with("https://") {
        return src.to_string();
    }

    let base = option_env!("API_BASE_URL")
        .unwrap_or("http://localhost:8080")
        .trim_end_matches('/')
        .to_string();

    let compile_id = option_env!("COMPILE_ID").unwrap_or("");
    let clean_src = src.strip_prefix('/').unwrap_or(src);

    if !compile_id.is_empty() {
        format!("{}/api/compiler/asset/{}/{}", base, compile_id, clean_src)
    } else {
        format!("{}/{}", base, clean_src)
    }
}

fn get_css_pixel_number(value: &str) -> Option<f64> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    let clean = if trimmed.to_lowercase().ends_with("px") {
        &trimmed[..trimmed.len() - 2]
    } else {
        trimmed
    };
    clean.parse::<f64>().ok()
}

fn get_component_fallback_size(comp_type: &str) -> (f64, f64) {
    match comp_type.to_lowercase().as_str() {
        "button" | "input" | "select" | "searchinput" | "textarea" | "checkbox" | "toggle"
        | "switch" => (120.0, 48.0),
        "text" | "heading" => (120.0, 32.0),
        "image" => (320.0, 200.0),
        "chart" => (520.0, 300.0),
        "card" | "container" | "box" | "flex" | "grid" | "layout" => (360.0, 160.0),
        "table" | "dynamictable" => (560.0, 300.0),
        _ => (120.0, 48.0),
    }
}

fn calculate_freeform_extent(children: &[ComponentNode]) -> Option<f64> {
    if children.is_empty() {
        return None;
    }

    let padding = 24.0;
    let mut max_bottom = 0.0;

    for child in children {
        let style_map = match child.props.style {
            Some(ref s) => s,
            None => continue,
        };

        let is_absolute = style_map
            .get("position")
            .map(|v| v.to_lowercase() == "absolute")
            .unwrap_or(false);

        if !is_absolute {
            continue;
        }

        let top = style_map
            .get("top")
            .and_then(|v| get_css_pixel_number(v))
            .unwrap_or(0.0);

        let height = style_map
            .get("height")
            .or_else(|| style_map.get("minHeight"))
            .and_then(|v| get_css_pixel_number(v))
            .unwrap_or_else(|| get_component_fallback_size(&child.component_type).1);

        let bottom = top + height + padding;
        if bottom > max_bottom {
            max_bottom = bottom;
        }
    }

    if max_bottom > 0.0 {
        Some(max_bottom)
    } else {
        None
    }
}

fn calculate_recursive_freeform_extent(children: &[ComponentNode], current_top: f64) -> f64 {
    let padding = 24.0;
    let mut max_bottom = current_top;

    for child in children {
        let style_map = match child.props.style {
            Some(ref s) => s,
            None => continue,
        };

        let is_absolute = style_map
            .get("position")
            .map(|v| v.to_lowercase() == "absolute")
            .unwrap_or(false)
            || child.placement.is_some();

        let child_top = style_map
            .get("top")
            .cloned()
            .or_else(|| child.placement.as_ref().and_then(|p| p.top.clone()))
            .and_then(|v| get_css_pixel_number(&v))
            .unwrap_or(0.0);

        let height = style_map
            .get("height")
            .or_else(|| style_map.get("minHeight"))
            .and_then(|v| get_css_pixel_number(v))
            .unwrap_or_else(|| get_component_fallback_size(&child.component_type).1);

        let absolute_top = if is_absolute {
            current_top + child_top
        } else {
            current_top
        };

        let child_max_bottom = if !child.children.is_empty() {
            calculate_recursive_freeform_extent(&child.children, absolute_top)
        } else {
            absolute_top + height + padding
        };

        let child_extent = (absolute_top + height + padding).max(child_max_bottom);
        if child_extent > max_bottom {
            max_bottom = child_extent;
        }
    }

    max_bottom
}

fn calculate_recursive_freeform_width_extent(children: &[ComponentNode], current_left: f64) -> f64 {
    let padding = 24.0;
    let mut max_right = current_left;

    for child in children {
        let style_map = match child.props.style {
            Some(ref s) => s,
            None => continue,
        };

        let is_absolute = style_map
            .get("position")
            .map(|v| v.to_lowercase() == "absolute")
            .unwrap_or(false)
            || child.placement.is_some();

        let child_left = style_map
            .get("left")
            .cloned()
            .or_else(|| child.placement.as_ref().and_then(|p| p.left.clone()))
            .and_then(|v| get_css_pixel_number(&v))
            .unwrap_or(0.0);

        let width = style_map
            .get("width")
            .or_else(|| style_map.get("minWidth"))
            .and_then(|v| get_css_pixel_number(v))
            .unwrap_or_else(|| get_component_fallback_size(&child.component_type).0);

        let absolute_left = if is_absolute {
            current_left + child_left
        } else {
            current_left
        };

        let child_max_right = if !child.children.is_empty() {
            calculate_recursive_freeform_width_extent(&child.children, absolute_left)
        } else {
            absolute_left + width + padding
        };

        let child_extent = (absolute_left + width + padding).max(child_max_right);
        if child_extent > max_right {
            max_right = child_extent;
        }
    }

    max_right
}

#[derive(Clone, Debug, PartialEq)]
struct ModalSizingStyles {
    dialog_style: String,
    body_style: String,
}

const MODAL_VIEWPORT_WIDTH_CAP: &str = "calc(100vw - 24px)";
const MODAL_VIEWPORT_HEIGHT_CAP: &str = "calc(100vh - 24px)";
const MODAL_BODY_HEIGHT_CAP: &str = "calc(100vh - 134px)";
const MODAL_CHROME_HEIGHT: f64 = 110.0;

fn css_min_with_cap(value: &str, cap: &str) -> String {
    format!("min({}, {})", value, cap)
}

fn css_px_min_with_cap(value: f64, cap: &str) -> String {
    css_min_with_cap(&format!("{:.0}px", value), cap)
}

fn modal_sizing_styles(node: &ComponentNode) -> ModalSizingStyles {
    let children_bottom = calculate_recursive_freeform_extent(&node.children, 0.0);
    let children_right = calculate_recursive_freeform_width_extent(&node.children, 0.0);
    let style_map = node.props.style.clone().unwrap_or_default();

    let mut dialog_style = format!(
        "background-color: #ffffff; border-radius: 16px; box-shadow: 0 20px 25px -5px rgba(0,0,0,0.1), 0 10px 10px -5px rgba(0,0,0,0.04); border: 1px solid rgba(0,0,0,0.05); overflow: hidden; display: flex; flex-direction: column; animation: scaleIn 0.2s ease-out; max-width: {}; max-height: {}; box-sizing: border-box;",
        MODAL_VIEWPORT_WIDTH_CAP, MODAL_VIEWPORT_HEIGHT_CAP
    );

    if let Some(w) = style_map.get("width") {
        if let Some(w_val) = get_css_pixel_number(w) {
            dialog_style.push_str(&format!(
                "width: {};",
                css_px_min_with_cap(w_val.max(children_right), MODAL_VIEWPORT_WIDTH_CAP)
            ));
        } else {
            dialog_style.push_str(&format!(
                "width: {};",
                css_min_with_cap(w, MODAL_VIEWPORT_WIDTH_CAP)
            ));
        }
    } else {
        let base_w = children_right.max(720.0);
        dialog_style.push_str(&format!(
            "width: {};",
            css_px_min_with_cap(base_w, MODAL_VIEWPORT_WIDTH_CAP)
        ));
    }

    if let Some(h) = style_map.get("height") {
        if let Some(h_val) = get_css_pixel_number(h) {
            dialog_style.push_str(&format!(
                "height: {};",
                css_px_min_with_cap(h_val, MODAL_VIEWPORT_HEIGHT_CAP)
            ));
        } else {
            dialog_style.push_str(&format!(
                "height: {};",
                css_min_with_cap(h, MODAL_VIEWPORT_HEIGHT_CAP)
            ));
        }
    }

    if let Some(min_h) = style_map.get("minHeight") {
        if let Some(min_h_val) = get_css_pixel_number(min_h) {
            let min_needed_h = children_bottom + MODAL_CHROME_HEIGHT;
            dialog_style.push_str(&format!(
                "min-height: {};",
                css_px_min_with_cap(min_h_val.max(min_needed_h), MODAL_VIEWPORT_HEIGHT_CAP)
            ));
        } else {
            dialog_style.push_str(&format!(
                "min-height: {};",
                css_min_with_cap(min_h, MODAL_VIEWPORT_HEIGHT_CAP)
            ));
        }
    } else if children_bottom > 0.0 {
        let min_needed_h = children_bottom + MODAL_CHROME_HEIGHT;
        dialog_style.push_str(&format!(
            "min-height: {};",
            css_px_min_with_cap(min_needed_h, MODAL_VIEWPORT_HEIGHT_CAP)
        ));
    }

    if let Some(min_w) = style_map.get("minWidth") {
        if let Some(min_w_val) = get_css_pixel_number(min_w) {
            dialog_style.push_str(&format!(
                "min-width: {};",
                css_px_min_with_cap(min_w_val.max(children_right), MODAL_VIEWPORT_WIDTH_CAP)
            ));
        } else {
            dialog_style.push_str(&format!(
                "min-width: {};",
                css_min_with_cap(min_w, MODAL_VIEWPORT_WIDTH_CAP)
            ));
        }
    } else if children_right > 0.0 {
        dialog_style.push_str(&format!(
            "min-width: {};",
            css_px_min_with_cap(children_right, MODAL_VIEWPORT_WIDTH_CAP)
        ));
    }

    let mut body_style = format!(
        "position: relative; padding: 20px; display: flex; flex-direction: column; gap: 14px; overflow: auto; flex: 1 1 auto; min-height: 0; min-width: 0; max-height: {}; box-sizing: border-box;",
        MODAL_BODY_HEIGHT_CAP
    );
    if children_bottom > 0.0 {
        body_style.push_str(&format!(
            "height: {};",
            css_px_min_with_cap(children_bottom, MODAL_BODY_HEIGHT_CAP)
        ));
    }

    ModalSizingStyles {
        dialog_style,
        body_style,
    }
}

fn resolve_min_height(
    style_map: Option<&std::collections::HashMap<String, String>>,
    max_bottom: f64,
) -> String {
    let mut base_val = None;
    if let Some(map) = style_map {
        if let Some(val) = map.get("minHeight").or_else(|| map.get("height")) {
            base_val = Some(val.clone());
        }
    }

    match base_val {
        Some(val) => {
            if let Some(pixel_val) = get_css_pixel_number(&val) {
                format!("{:.0}px", pixel_val.max(max_bottom))
            } else {
                format!("max({}, {:.0}px)", val, max_bottom)
            }
        }
        None => format!("{:.0}px", max_bottom),
    }
}

fn strip_guide_borders(style_map: &mut std::collections::HashMap<String, String>, comp_type: &str) {
    let type_lower = comp_type.to_lowercase();
    if type_lower == "layout"
        || type_lower == "container"
        || type_lower == "flex"
        || type_lower == "grid"
        || type_lower == "box"
    {
        let has_dashed_or_dotted = style_map.iter().any(|(k, v)| {
            let k_lower = k.to_lowercase();
            let v_lower = v.to_lowercase();
            (k_lower == "border" && (v_lower.contains("dashed") || v_lower.contains("dotted")))
                || (k_lower == "borderstyle" && (v_lower == "dashed" || v_lower == "dotted"))
                || (k_lower == "outline"
                    && (v_lower.contains("dashed") || v_lower.contains("dotted")))
                || (k_lower == "outlinestyle" && (v_lower == "dashed" || v_lower == "dotted"))
        });

        if has_dashed_or_dotted {
            style_map.remove("border");
            if let Some(bs) = style_map.get("borderStyle").map(|v| v.to_lowercase()) {
                if bs == "dashed" || bs == "dotted" {
                    style_map.remove("borderStyle");
                }
            }
            style_map.remove("outline");
            if let Some(os) = style_map.get("outlineStyle").map(|v| v.to_lowercase()) {
                if os == "dashed" || os == "dotted" {
                    style_map.remove("outlineStyle");
                }
            }
        }
    }
}

fn should_render_as_flow_layout(
    node: &ComponentNode,
    is_repeater_child: bool,
    is_flow_context: bool,
) -> bool {
    if is_repeater_child {
        return true;
    }

    let has_explicit_absolute = node
        .props
        .style
        .as_ref()
        .and_then(|s| s.get("position"))
        .map(|v| v.to_lowercase() == "absolute")
        .unwrap_or(false);

    !has_explicit_absolute && is_flow_context
}

fn modal_on_open_actions(node: &ComponentNode) -> Vec<serde_json::Value> {
    node.on_load.clone()
}

#[derive(Clone, Debug, PartialEq)]
struct DropdownOption {
    label: String,
    value: String,
}

fn node_extra_str<'a>(node: &'a ComponentNode, keys: &[&str]) -> Option<&'a str> {
    keys.iter()
        .find_map(|key| node.props.extra.get(*key).and_then(|value| value.as_str()))
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn dropdown_label_field(node: &ComponentNode) -> String {
    node_extra_str(
        node,
        &[
            "labelField",
            "label_field",
            "optionLabel",
            "option_label",
            "optionLabelKey",
            "option_label_key",
            "optionsLabelKey",
            "options_label_key",
            "labelKey",
            "label_key",
        ],
    )
    .unwrap_or("label")
    .to_string()
}

fn dropdown_value_field(node: &ComponentNode) -> String {
    node_extra_str(
        node,
        &[
            "valueField",
            "value_field",
            "optionValue",
            "option_value",
            "optionValueKey",
            "option_value_key",
            "optionsValueKey",
            "options_value_key",
            "valueKey",
            "value_key",
        ],
    )
    .unwrap_or("value")
    .to_string()
}

fn dropdown_data_source_path(node: &ComponentNode) -> Option<String> {
    node.props
        .data_source
        .clone()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            node_extra_str(
                node,
                &[
                    "dataSource",
                    "data_source",
                    "optionsSource",
                    "options_source",
                    "optionsBind",
                    "options_bind",
                    "optionSource",
                    "option_source",
                    "optionsDataSource",
                    "options_data_source",
                    "itemsSource",
                    "items_source",
                    "source",
                ],
            )
            .map(ToString::to_string)
        })
}

fn dropdown_target_key(node: &ComponentNode) -> String {
    node_extra_str(node, &["targetKey", "target_key"])
        .map(ToString::to_string)
        .or_else(|| {
            dropdown_data_source_path(node)
                .map(|path| get_actual_data_path(&path))
                .filter(|path| !path.is_empty() && !path.contains("item."))
                .and_then(|path| path.split('.').last().map(ToString::to_string))
        })
        .unwrap_or_else(|| "result".to_string())
}

fn dropdown_load_actions(node: &ComponentNode) -> Vec<serde_json::Value> {
    let mut actions = node.on_load.clone();
    let Some(api_url) = node_extra_str(
        node,
        &[
            "apiUrl",
            "api_url",
            "optionsApiUrl",
            "options_api_url",
            "fetchUrl",
            "fetch_url",
        ],
    ) else {
        return actions;
    };

    let method = node
        .props
        .method
        .clone()
        .or_else(|| node_extra_str(node, &["method"]).map(ToString::to_string))
        .unwrap_or_else(|| "GET".to_string());
    let body = node
        .props
        .request_body
        .clone()
        .or_else(|| node.props.extra.get("requestBody").cloned())
        .or_else(|| node.props.extra.get("request_body").cloned())
        .unwrap_or_else(|| serde_json::Value::Object(Default::default()));

    actions.push(serde_json::json!({
        "type": "API_CALL",
        "payload": {
            "url": api_url,
            "method": method,
            "body": body,
            "targetKey": dropdown_target_key(node),
        }
    }));
    actions
}

fn dropdown_value_to_string(value: &serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::Null => None,
        serde_json::Value::Array(_) | serde_json::Value::Object(_) => Some(value.to_string()),
        _ => Some(json_value_to_display_string(value)),
    }
}

fn dropdown_object_field(
    map: &serde_json::Map<String, serde_json::Value>,
    preferred: &str,
    fallbacks: &[&str],
) -> Option<String> {
    map.get(preferred)
        .and_then(dropdown_value_to_string)
        .or_else(|| {
            fallbacks
                .iter()
                .find_map(|key| map.get(*key).and_then(dropdown_value_to_string))
        })
        .filter(|value| !value.is_empty())
}

fn dropdown_option_from_value(
    value: &serde_json::Value,
    label_field: &str,
    value_field: &str,
) -> Option<DropdownOption> {
    match value {
        serde_json::Value::String(_)
        | serde_json::Value::Number(_)
        | serde_json::Value::Bool(_) => {
            let display = json_value_to_display_string(value);
            Some(DropdownOption {
                label: display.clone(),
                value: display,
            })
        }
        serde_json::Value::Object(map) => {
            let option_value = dropdown_object_field(
                map,
                value_field,
                &[
                    "value", "id", "key", "code", "slug", "name", "label", "title",
                ],
            )?;
            let option_label = dropdown_object_field(
                map,
                label_field,
                &[
                    "label",
                    "name",
                    "title",
                    "text",
                    "display",
                    "displayName",
                    "value",
                    "id",
                ],
            )
            .unwrap_or_else(|| option_value.clone());

            Some(DropdownOption {
                label: option_label,
                value: option_value,
            })
        }
        _ => None,
    }
}

fn dropdown_options_from_value(
    value: &serde_json::Value,
    label_field: &str,
    value_field: &str,
) -> Vec<DropdownOption> {
    if let Some(arr) = value.as_array() {
        return arr
            .iter()
            .filter_map(|item| dropdown_option_from_value(item, label_field, value_field))
            .collect();
    }

    if let Some(arr) = extract_array(value, "") {
        return arr
            .iter()
            .filter_map(|item| dropdown_option_from_value(item, label_field, value_field))
            .collect();
    }

    value
        .as_object()
        .map(|map| {
            map.iter()
                .filter_map(|(key, value)| {
                    dropdown_value_to_string(value).map(|label| DropdownOption {
                        label,
                        value: key.clone(),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

fn dropdown_static_options(node: &ComponentNode) -> Option<&serde_json::Value> {
    node.props
        .extra
        .get("options")
        .or_else(|| node.props.extra.get("items"))
        .or_else(|| node.props.extra.get("choices"))
        .or_else(|| node.props.extra.get("values"))
        .or(node.props.data.as_ref())
}

fn dropdown_source_value(
    path: &str,
    local_item: Option<&serde_json::Value>,
    global_data: &serde_json::Value,
) -> Option<serde_json::Value> {
    let actual_path = get_actual_data_path(path);
    let path_lower = actual_path.to_lowercase();

    if path_lower == "item" {
        return local_item.cloned();
    }

    if path_lower.starts_with("item.") {
        return local_item.and_then(|item| resolve_json_value_path(item, &actual_path[5..]));
    }

    resolve_json_value_path(global_data, &actual_path).or_else(|| {
        actual_path
            .split('.')
            .last()
            .and_then(|last| resolve_json_value_path(global_data, last))
    })
}

fn dropdown_options_for_node(
    node: &ComponentNode,
    local_item: Option<&serde_json::Value>,
    global_data: &serde_json::Value,
) -> Vec<DropdownOption> {
    let label_field = dropdown_label_field(node);
    let value_field = dropdown_value_field(node);

    if let Some(value) = dropdown_static_options(node) {
        let options = dropdown_options_from_value(value, &label_field, &value_field);
        if !options.is_empty() {
            return options;
        }
    }

    if let Some(path) = dropdown_data_source_path(node) {
        if let Some(value) = dropdown_source_value(&path, local_item, global_data) {
            let options = dropdown_options_from_value(&value, &label_field, &value_field);
            if !options.is_empty() {
                return options;
            }
        }
    }

    if let Some(target_key) = node_extra_str(node, &["targetKey", "target_key"]) {
        if let Some(value) = dropdown_source_value(target_key, local_item, global_data) {
            return dropdown_options_from_value(&value, &label_field, &value_field);
        }
    }

    Vec::new()
}

fn dropdown_selected_values(value: Option<&serde_json::Value>) -> Vec<String> {
    match value {
        Some(serde_json::Value::Array(values)) => {
            values.iter().filter_map(dropdown_value_to_string).collect()
        }
        Some(value) => dropdown_value_to_string(value).into_iter().collect(),
        None => Vec::new(),
    }
}

fn dropdown_values_to_json(values: &[String]) -> serde_json::Value {
    serde_json::Value::Array(
        values
            .iter()
            .map(|value| serde_json::Value::String(value.clone()))
            .collect(),
    )
}

fn dropdown_option_label(options: &[DropdownOption], value: &str) -> String {
    options
        .iter()
        .find(|option| option.value == value)
        .map(|option| option.label.clone())
        .unwrap_or_else(|| value.to_string())
}

fn action_array_field(action: &serde_json::Value, keys: &[&str]) -> Option<Vec<serde_json::Value>> {
    let payload = action.get("payload");
    keys.iter()
        .find_map(|key| {
            action
                .get(*key)
                .or_else(|| payload.and_then(|payload| payload.get(*key)))
                .and_then(|value| value.as_array())
                .cloned()
        })
        .filter(|actions| !actions.is_empty())
}

fn action_success_actions(action: &serde_json::Value) -> Option<Vec<serde_json::Value>> {
    action_array_field(action, &["onSuccess", "on_success"])
}

fn action_error_actions(action: &serde_json::Value) -> Option<Vec<serde_json::Value>> {
    action_array_field(action, &["onError", "on_error"])
}

fn is_submit_form_action(action: &serde_json::Value) -> bool {
    action
        .get("type")
        .and_then(|value| value.as_str())
        .map(|action_type| action_type.eq_ignore_ascii_case("SUBMIT_FORM"))
        .unwrap_or(false)
}

fn button_has_submit_form_action(actions: &[serde_json::Value]) -> bool {
    actions.iter().any(is_submit_form_action)
}

fn button_click_actions(actions: &[serde_json::Value]) -> Vec<serde_json::Value> {
    actions
        .iter()
        .filter(|action| !is_submit_form_action(action))
        .cloned()
        .collect()
}

fn form_submit_scope(
    form_values: serde_json::Value,
    local_item: Option<serde_json::Value>,
) -> serde_json::Value {
    let mut scope = match local_item {
        Some(serde_json::Value::Object(map)) => map,
        Some(value) => {
            let mut map = serde_json::Map::new();
            map.insert("item".to_string(), value);
            map
        }
        None => serde_json::Map::new(),
    };

    scope.insert("form".to_string(), form_values.clone());
    scope.insert("formData".to_string(), form_values.clone());
    scope.insert("formValues".to_string(), form_values);
    serde_json::Value::Object(scope)
}

fn form_control_name(node: &ComponentNode) -> String {
    node_extra_str(node, &["name", "fieldName", "field_name"])
        .map(ToString::to_string)
        .or_else(|| {
            node.props.bind.as_deref().and_then(|bind| {
                normalized_bind_expression(bind)
                    .and_then(|path| path.split('.').last().map(ToString::to_string))
            })
        })
        .filter(|name| !name.trim().is_empty())
        .unwrap_or_else(|| node.id.clone())
}

fn form_event_values_to_json(values: Vec<(String, FormValue)>) -> serde_json::Value {
    let mut map = serde_json::Map::new();
    for (key, value) in values {
        if key.trim().is_empty() {
            continue;
        }

        let value = match value {
            FormValue::Text(text) => serde_json::Value::String(text),
            FormValue::File(_) => serde_json::Value::Null,
        };

        match map.get_mut(&key) {
            Some(serde_json::Value::Array(existing)) => existing.push(value),
            Some(existing) => {
                let previous = std::mem::replace(existing, serde_json::Value::Null);
                *existing = serde_json::Value::Array(vec![previous, value]);
            }
            None => {
                map.insert(key, value);
            }
        }
    }

    serde_json::Value::Object(map)
}

fn is_item_visible(
    item: &serde_json::Value,
    global_data: &serde_json::Value,
    local_item: Option<&serde_json::Value>,
) -> bool {
    if let Some(required_role_val) = item.get("requiredRole") {
        if let Some(required_role) = required_role_val.as_str() {
            let role = global_data
                .get("authSession")
                .and_then(|session| session.get("role"))
                .and_then(|role_val| role_val.as_str())
                .unwrap_or("");
            let allowed: Vec<&str> = required_role.split('|').map(|s| s.trim()).collect();
            if !allowed.is_empty() && !allowed.iter().any(|r| *r == role) {
                return false;
            }
        }
    }

    let visible_if = item
        .get("visibleIf")
        .or_else(|| item.get("when"))
        .or_else(|| item.get("visibleWhen"))
        .and_then(|v| v.as_str());

    if let Some(expr) = visible_if {
        let trimmed = expr.trim();
        if trimmed.is_empty() {
            return true;
        }

        let clean_expr = trimmed
            .strip_prefix("{{")
            .and_then(|s| s.strip_suffix("}}"))
            .unwrap_or(trimmed)
            .trim();
        let is_negated = clean_expr.starts_with('!');
        let path = if is_negated {
            &clean_expr[1..]
        } else {
            clean_expr
        };

        let resolved = if path.starts_with("item.") {
            let sub_path = &path[5..];
            local_item.and_then(|item| resolve_json_path(item, sub_path))
        } else if path == "item" {
            local_item.map(|item| match item {
                serde_json::Value::String(s) => s.clone(),
                other => other.to_string(),
            })
        } else if path.starts_with("data.") {
            let sub_path = &path[5..];
            resolve_json_path(global_data, sub_path)
        } else {
            local_item
                .and_then(|item| resolve_json_path(item, path))
                .or_else(|| resolve_json_path(global_data, path))
        };

        let is_truthy = match resolved {
            Some(ref s) => {
                !s.is_empty() && s != "false" && s != "null" && s != "undefined" && s != "0"
            }
            None => false,
        };

        if is_negated {
            return !is_truthy;
        } else {
            return is_truthy;
        }
    }

    true
}

fn is_component_visible(
    node: &ComponentNode,
    global_data: &serde_json::Value,
    local_item: Option<&serde_json::Value>,
) -> bool {
    let mut map = serde_json::Map::new();
    for (k, v) in &node.props.extra {
        map.insert(k.clone(), v.clone());
    }
    is_item_visible(&serde_json::Value::Object(map), global_data, local_item)
}

fn nav_item_actions(item: &serde_json::Value) -> Vec<serde_json::Value> {
    if let Some(actions) = item.get("onClick").and_then(|v| v.as_array()) {
        if !actions.is_empty() {
            return actions.clone();
        }
    }
    if let Some(actions) = item.get("onClickAction").and_then(|v| v.as_array()) {
        if !actions.is_empty() {
            return actions.clone();
        }
    }

    let action_type = item
        .get("actionType")
        .or_else(|| item.get("action_type"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_uppercase();

    if action_type == "OPEN_MODAL" {
        if let Some(modal_id) = item
            .get("modalId")
            .or_else(|| item.get("modal_id"))
            .and_then(|v| v.as_str())
        {
            return vec![serde_json::json!({
                "type": "OPEN_MODAL",
                "payload": { "modalId": modal_id }
            })];
        }
    }

    if action_type == "API_CALL" {
        let url = item
            .get("workflowUrl")
            .or_else(|| item.get("url"))
            .and_then(|v| v.as_str())
            .unwrap_or("/api/run/my_workflow");
        let method = item
            .get("method")
            .and_then(|v| v.as_str())
            .unwrap_or("POST");
        let body = item
            .get("body")
            .filter(|value| value.is_object())
            .cloned()
            .unwrap_or_else(|| serde_json::json!({}));
        let target_key = item
            .get("targetKey")
            .or_else(|| item.get("target_key"))
            .and_then(|v| v.as_str())
            .unwrap_or("result");
        return vec![serde_json::json!({
            "type": "API_CALL",
            "payload": {
                "url": url,
                "method": method,
                "body": body,
                "targetKey": target_key
            }
        })];
    }

    let target = item
        .get("pageId")
        .or_else(|| item.get("page_id"))
        .or_else(|| item.get("target"))
        .and_then(|v| v.as_str())
        .unwrap_or("");

    if target.is_empty() {
        Vec::new()
    } else if target.starts_with('/') {
        vec![serde_json::json!({ "type": "NAVIGATE", "payload": { "to": target } })]
    } else {
        vec![serde_json::json!({ "type": "NAVIGATE", "payload": { "pageId": target } })]
    }
}

fn bar_button_style(item: &serde_json::Value, is_top: bool, theme: &str) -> String {
    let variant = item
        .get("variant")
        .and_then(|v| v.as_str())
        .unwrap_or("ghost")
        .to_lowercase();
    let border_color = if theme.eq_ignore_ascii_case("dark") {
        "#2a3565"
    } else {
        "#e5e7eb"
    };
    let muted = if theme.eq_ignore_ascii_case("dark") {
        "#cbd5e1"
    } else {
        "#475569"
    };
    let default_bg = if theme.eq_ignore_ascii_case("dark") {
        "rgba(26,37,85,0.4)"
    } else {
        "#f8fafc"
    };
    let (bg, color, border) = match variant.as_str() {
        "primary" => (
            if theme.eq_ignore_ascii_case("dark") {
                "#4a90e2"
            } else {
                "#07175e"
            },
            "#ffffff",
            "transparent",
        ),
        "danger" => ("#dc3545", "#ffffff", "transparent"),
        "outline" => ("transparent", muted, border_color),
        "default" => (default_bg, muted, border_color),
        _ => ("transparent", muted, "transparent"),
    };
    format!(
        "display: inline-flex; align-items: center; justify-content: {}; gap: 8px; width: {}; min-height: 36px; padding: 8px 12px; border-radius: 8px; border: 1px solid {}; background: {}; color: {}; font-size: 0.9rem; font-weight: 500; cursor: pointer; text-align: {}; white-space: nowrap; box-sizing: border-box; transition: all 0.2s ease;",
        if is_top { "center" } else { "flex-start" },
        if is_top { "auto" } else { "100%" },
        border,
        bg,
        color,
        if is_top { "center" } else { "left" },
    )
}

fn json_value_to_display_string(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Null => String::new(),
        other => other.to_string(),
    }
}

fn json_value_to_bool(value: &serde_json::Value) -> Option<bool> {
    match value {
        serde_json::Value::Bool(b) => Some(*b),
        serde_json::Value::Number(n) => n.as_f64().map(|v| v != 0.0),
        serde_json::Value::String(s) => match s.trim().to_lowercase().as_str() {
            "true" | "1" | "yes" | "on" => Some(true),
            "false" | "0" | "no" | "off" | "" => Some(false),
            _ => None,
        },
        _ => None,
    }
}

fn normalized_bind_expression(bind: &str) -> Option<String> {
    let trimmed = bind.trim();
    if trimmed.is_empty() {
        return None;
    }

    let unwrapped = trimmed
        .strip_prefix("{{")
        .and_then(|value| value.strip_suffix("}}"))
        .unwrap_or(trimmed)
        .trim();

    if unwrapped.is_empty() {
        None
    } else {
        Some(unwrapped.to_string())
    }
}

fn resolve_node_bind_value(
    node: &ComponentNode,
    local_item: Option<&serde_json::Value>,
    global_data: &serde_json::Value,
) -> Option<serde_json::Value> {
    let expr = normalized_bind_expression(node.props.bind.as_deref()?)?;
    let wrapped = serde_json::Value::String(format!("{{{{{}}}}}", expr));
    Some(resolve_json_templates(&wrapped, local_item, global_data))
}

fn set_bind_path_value(bind: &str, root: &mut serde_json::Value, value: serde_json::Value) {
    let Some(expr) = normalized_bind_expression(bind) else {
        return;
    };

    let expr_lower = expr.to_lowercase();
    if expr_lower == "item"
        || expr_lower.starts_with("item.")
        || expr_lower == "row"
        || expr_lower.starts_with("row.")
        || expr_lower == "rowdata"
        || expr_lower.starts_with("rowdata.")
    {
        return;
    }

    set_json_value_at_path(root, &expr, value);
}

fn set_node_bind_value(
    node: &ComponentNode,
    root: &mut serde_json::Value,
    value: serde_json::Value,
) {
    if let Some(bind) = node.props.bind.as_deref() {
        set_bind_path_value(bind, root, value);
    }
}

fn apply_bound_value_to_node(node: &mut ComponentNode, value: &serde_json::Value) {
    node.props
        .extra
        .insert("__boundValue".to_string(), value.clone());

    let display = json_value_to_display_string(value);
    match node.component_type.as_str() {
        "Text" => node.props.content = Some(display),
        "Heading" => node.props.text = Some(display),
        "Button" | "Badge" | "StatusBadge" => node.props.label = Some(display),
        "Alert" => node.props.message = Some(display),
        "Card" | "Header" | "Author" | "Sidebar" | "Topbar" | "DrawerPanel" | "Modal"
        | "TargetKpiCard" => node.props.title = Some(display),
        "Link" | "nav-button" => node.props.text = Some(display),
        "Image" | "Video" | "Audio" | "Iframe" => {
            node.props
                .extra
                .insert("src".to_string(), serde_json::Value::String(display));
        }
        "Avatar" => {
            let key = if is_image_source(&display) {
                "src"
            } else {
                "fallback"
            };
            node.props
                .extra
                .insert(key.to_string(), serde_json::Value::String(display));
        }
        "Table" | "DynamicTable" => {
            if value.is_array() {
                node.props.rows = Some(value.clone());
            }
        }
        "Chart" => {
            node.props.data = Some(value.clone());
        }
        "GaugeChart" | "ProgressRing" => {
            node.props.extra.insert("value".to_string(), value.clone());
        }
        "Input"
        | "SearchInput"
        | "Textarea"
        | "Select"
        | "DatePicker"
        | "TimePicker"
        | "Checkbox"
        | "Toggle"
        | "Switch"
        | "TagInput"
        | "OtpInput"
        | "ColorPicker"
        | "MultiSelectDropdown"
        | "RichTextEditor"
        | "SignaturePad" => {}
        _ => {
            if node.props.text.is_some() {
                node.props.text = Some(display);
            } else if node.props.content.is_some() {
                node.props.content = Some(display);
            } else if node.props.label.is_some() {
                node.props.label = Some(display);
            } else if node.props.title.is_some() {
                node.props.title = Some(display);
            } else {
                node.props.content = Some(display);
            }
        }
    }
}

#[derive(Clone, Copy)]
pub struct GlobalDataState(pub Signal<serde_json::Value>);

#[derive(Clone, Copy)]
pub struct RepeaterItemState(pub Signal<Option<serde_json::Value>>);

#[derive(Clone, Copy)]
pub struct ParentLayoutContext(pub bool);

#[derive(Clone, Copy)]
pub struct ActiveBreakpointContext(pub Signal<String>);

#[derive(Clone)]
pub struct PageLayoutModeContext(pub crate::models::LayoutMode);

#[derive(Props, Clone, PartialEq)]
pub struct ComponentRendererProps {
    pub node: ComponentNode,
    #[props(default)]
    pub selected_id: Option<Signal<Option<String>>>,
    #[props(default)]
    pub on_select: Option<EventHandler<String>>,
    #[props(default)]
    pub on_drop: Option<EventHandler<(String, f64, f64)>>,
    #[props(default)]
    pub on_delete: Option<EventHandler<String>>,
    #[props(default)]
    pub on_resize_start: Option<EventHandler<(String, String, f64, f64)>>,
    #[props(default)]
    pub on_drag_start: Option<EventHandler<(String, f64, f64)>>,
    #[props(default)]
    pub active_breakpoint: Option<String>,
    #[props(default)]
    pub is_repeater_child: bool,
    #[props(default)]
    pub is_flow_context: bool,
}

#[component]
pub fn ComponentRenderer(props: ComponentRendererProps) -> Element {
    let node = props.node.clone();

    // ---- UNCONDITIONAL HOOKS (must all run before any early returns) ----
    let data_state = use_context::<GlobalDataState>().0;
    let local_item_signal = use_context::<RepeaterItemState>().0;
    let local_item = local_item_signal.read().clone();
    let parent_is_flow = use_context::<ParentLayoutContext>().0;

    let child_is_flow = if node.id == "root" {
        node.layout_mode != crate::models::LayoutMode::Absolute
    } else {
        true
    };
    use_context_provider(|| ParentLayoutContext(child_is_flow));
    // ---- END UNCONDITIONAL HOOKS ----

    // Now safe to do conditional logic
    if !is_component_visible(&node, &data_state.read(), local_item.as_ref()) {
        return rsx! {};
    }

    let repeater_enabled = node
        .props
        .extra
        .get("repeaterEnabled")
        .and_then(|v| v.as_bool())
        .unwrap_or_else(|| node.component_type == "Repeater");

    let repeater_ds = node
        .props
        .extra
        .get("repeaterDataSource")
        .and_then(|v| v.as_str())
        .or_else(|| node.props.repeater_data.as_deref())
        .unwrap_or("");

    if repeater_enabled && !repeater_ds.is_empty() && !props.is_repeater_child {
        let mut path = repeater_ds.to_string();
        if path.starts_with("data.") {
            path = path[5..].to_string();
        }

        let global_data = data_state.read();
        #[cfg(target_arch = "wasm32")]
        web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&format!(
            "Repeater check: path = '{}', global_data = {:?}",
            path, *global_data
        )));
        let items_opt = resolve_array_path(&global_data, &path);
        #[cfg(target_arch = "wasm32")]
        web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&format!(
            "Repeater resolved items count: {:?}",
            items_opt.as_ref().map(|v| v.len())
        )));

        if let Some(items) = items_opt {
            let columns = node
                .props
                .extra
                .get("repeaterColumns")
                .and_then(|v| {
                    v.as_u64()
                        .or_else(|| v.as_str().and_then(|s| s.parse::<u64>().ok()))
                })
                .unwrap_or(1) as usize;

            let gap = node
                .props
                .extra
                .get("repeaterGap")
                .and_then(|v| v.as_str())
                .unwrap_or("12px");

            let mut container_styles = String::new();

            let active_bp = props
                .active_breakpoint
                .clone()
                .unwrap_or_else(|| "desktop".to_string());
            let active_placement = match active_bp.as_str() {
                "tablet" => node
                    .placement_tablet
                    .clone()
                    .or_else(|| node.placement.clone()),
                "mobile" => node
                    .placement_mobile
                    .clone()
                    .or_else(|| node.placement_tablet.clone())
                    .or_else(|| node.placement.clone()),
                _ => node
                    .placement_desktop
                    .clone()
                    .or_else(|| node.placement.clone()),
            };

            if let Some(ref p) = active_placement {
                container_styles.push_str("position: absolute;");
                if let Some(ref t) = p.top {
                    container_styles.push_str(&format!("top: {};", t));
                }
                if let Some(ref l) = p.left {
                    container_styles.push_str(&format!("left: {};", l));
                }
            }

            if let Some(ref style_map) = node.props.style {
                if let Some(w) = style_map.get("width") {
                    container_styles.push_str(&format!("width: {};", w));
                }
                if let Some(h) = style_map.get("height") {
                    container_styles.push_str(&format!("height: {};", h));
                }
                if let Some(mh) = style_map.get("minHeight") {
                    container_styles.push_str(&format!("min-height: {};", mh));
                }
                if let Some(pos) = style_map.get("position") {
                    if pos == "absolute" && active_placement.is_none() {
                        container_styles.push_str("position: absolute;");
                    }
                }
                if let Some(top) = style_map.get("top") {
                    if active_placement.is_none() {
                        container_styles.push_str(&format!("top: {};", top));
                    }
                }
                if let Some(left) = style_map.get("left") {
                    if active_placement.is_none() {
                        container_styles.push_str(&format!("left: {};", left));
                    }
                }
            }

            container_styles.push_str(&format!(
                "display: grid; grid-template-columns: repeat({}, 1fr); gap: {}; overflow-y: auto; box-sizing: border-box;",
                columns, gap
            ));

            return rsx! {
                div {
                    style: "{container_styles}",
                    for (idx, item) in items.into_iter().enumerate() {
                        RepeaterItemWrapper {
                            key: "{node.id}_{idx}",
                            node: node.clone(),
                            item: item,
                            selected_id: props.selected_id,
                            on_select: props.on_select.clone(),
                            on_drop: props.on_drop.clone(),
                            on_delete: props.on_delete.clone(),
                            on_resize_start: props.on_resize_start.clone(),
                            on_drag_start: props.on_drag_start.clone(),
                            active_breakpoint: props.active_breakpoint.clone()
                        }
                    }
                }
            };
        }
    }

    rsx! {
        ComponentRendererInner {
            node: node,
            selected_id: props.selected_id,
            on_select: props.on_select,
            on_drop: props.on_drop,
            on_delete: props.on_delete,
            on_resize_start: props.on_resize_start,
            on_drag_start: props.on_drag_start,
            active_breakpoint: props.active_breakpoint,
            is_repeater_child: props.is_repeater_child,
            is_flow_context: parent_is_flow || props.is_flow_context
        }
    }
}

#[component]
fn RepeaterItemWrapper(
    node: ComponentNode,
    item: serde_json::Value,
    selected_id: Option<Signal<Option<String>>>,
    on_select: Option<EventHandler<String>>,
    on_drop: Option<EventHandler<(String, f64, f64)>>,
    on_delete: Option<EventHandler<String>>,
    on_resize_start: Option<EventHandler<(String, String, f64, f64)>>,
    on_drag_start: Option<EventHandler<(String, f64, f64)>>,
    active_breakpoint: Option<String>,
) -> Element {
    let mut local_item_sig = use_signal(|| Some(item.clone()));
    if local_item_sig.read().as_ref() != Some(&item) {
        *local_item_sig.write() = Some(item.clone());
    }
    use_context_provider(|| RepeaterItemState(local_item_sig));

    rsx! {
        ComponentRendererInner {
            node: node,
            selected_id,
            on_select,
            on_drop,
            on_delete,
            on_resize_start,
            on_drag_start,
            active_breakpoint,
            is_repeater_child: true,
            is_flow_context: true
        }
    }
}

#[component]
#[allow(unused_variables)]
pub fn ComponentRendererInner(props: ComponentRendererProps) -> Element {
    let mut node = props.node;
    let selected_id = props.selected_id;
    let on_select = props.on_select;
    let on_drop = props.on_drop;
    let on_delete = props.on_delete;
    let on_resize_start = props.on_resize_start;
    let on_drag_start = props.on_drag_start;
    let active_breakpoint = props.active_breakpoint.clone();

    // Pull dynamic states from context providers
    let mut data_state = use_context::<GlobalDataState>().0;
    let local_item_sig = use_context::<RepeaterItemState>().0;
    let local_item = local_item_sig.read().clone();
    let global_data_snapshot = data_state.read().clone();
    if let Some(bound_value) =
        resolve_node_bind_value(&node, local_item.as_ref(), &global_data_snapshot)
    {
        apply_bound_value_to_node(&mut node, &bound_value);
    }

    let is_editor =
        selected_id.is_some() && on_select.is_some() && on_drop.is_some() && on_delete.is_some();
    let is_sel = if is_editor {
        selected_id.unwrap().read().as_ref() == Some(&node.id)
    } else {
        false
    };
    let border_outline = if is_sel {
        "2px solid #3b82f6"
    } else {
        "1px dashed rgba(59, 130, 246, 0.15)"
    };

    let page_layout_mode = use_context::<PageLayoutModeContext>().0;
    let is_flow_layout =
        should_render_as_flow_layout(&node, props.is_repeater_child, props.is_flow_context);

    let max_bottom = if !node.children.is_empty() {
        calculate_freeform_extent(&node.children)
    } else {
        None
    };

    let mut styles = if is_flow_layout {
        let mut clean_styles = if let Some(ref style_map) = node.props.style {
            let mut map = style_map.clone();
            map.remove("position");
            map.remove("left");
            map.remove("right");
            map.remove("top");
            map.remove("bottom");
            map.remove("zIndex");
            if props.is_repeater_child {
                map.insert("width".to_string(), "100%".to_string());
            }
            if let Some(mb) = max_bottom {
                let resolved = resolve_min_height(Some(style_map), mb);
                map.insert("minHeight".to_string(), resolved);
                map.remove("height");
            }
            strip_guide_borders(&mut map, &node.component_type);
            crate::models::NodeProps {
                style: Some(map),
                ..Default::default()
            }
            .to_style_string()
        } else {
            let mut s = if let Some(mb) = max_bottom {
                format!("min-height:{:.0}px;", mb)
            } else {
                String::new()
            };
            if props.is_repeater_child {
                s.push_str("width:100%;");
            }
            s
        };
        clean_styles.push_str("position:relative;");
        clean_styles
    } else {
        if let Some(ref style_map) = node.props.style {
            let mut map = style_map.clone();
            if let Some(mb) = max_bottom {
                let resolved = resolve_min_height(Some(style_map), mb);
                map.insert("minHeight".to_string(), resolved);
                map.remove("height");
            }
            strip_guide_borders(&mut map, &node.component_type);
            crate::models::NodeProps {
                style: Some(map),
                ..Default::default()
            }
            .to_style_string()
        } else {
            if let Some(mb) = max_bottom {
                format!("min-height:{:.0}px;", mb)
            } else {
                String::new()
            }
        }
    };
    let mut wrapper_styles = String::new();

    let active_bp = use_context::<ActiveBreakpointContext>().0.read().clone();
    let active_placement = match active_bp.as_str() {
        "tablet" => node
            .placement_tablet
            .clone()
            .or_else(|| node.placement.clone()),
        "mobile" => node
            .placement_mobile
            .clone()
            .or_else(|| node.placement_tablet.clone())
            .or_else(|| node.placement.clone()),
        _ => node
            .placement_desktop
            .clone()
            .or_else(|| node.placement.clone()),
    };

    if is_editor {
        wrapper_styles.push_str(&format!("position: relative; border: {}; cursor: pointer; box-sizing: border-box; user-select: none;", border_outline));
        if !is_flow_layout {
            if let Some(ref p) = active_placement {
                if node.id != "root" {
                    wrapper_styles.push_str("position: absolute; align-self: flex-start;");
                    if let Some(ref t) = p.top {
                        wrapper_styles.push_str(&format!("top: {};", t));
                    }
                    if let Some(ref l) = p.left {
                        wrapper_styles.push_str(&format!("left: {};", l));
                    }
                }

                let style_map = node.props.style.clone().unwrap_or_default();
                if let Some(w) = style_map.get("width") {
                    wrapper_styles.push_str(&format!("width: {};", w));
                }
                if let Some(h) = style_map.get("height") {
                    wrapper_styles.push_str(&format!("height: {};", h));
                }
                if let Some(z) = style_map.get("zIndex") {
                    wrapper_styles.push_str(&format!("z-index: {};", z));
                }

                let has_w = style_map.contains_key("width");
                let has_h = style_map.contains_key("height");
                let w_inner = if has_w { "100%" } else { "auto" };
                let h_inner = if has_h { "100%" } else { "auto" };

                // For the inner element content, force it to fill the absolute wrapper only if dimensions are specified
                styles = format!("position: relative; width: {}; height: {}; box-sizing: border-box; margin: 0; display: block;", w_inner, h_inner);
                if let Some(bg) = style_map.get("backgroundColor") {
                    styles.push_str(&format!("background-color: {};", bg));
                }
                if let Some(pad) = style_map.get("padding") {
                    styles.push_str(&format!("padding: {};", pad));
                }
                if let Some(bor) = style_map.get("border") {
                    styles.push_str(&format!("border: {};", bor));
                }
                if let Some(col) = style_map.get("color") {
                    styles.push_str(&format!("color: {};", col));
                }
                if let Some(rad) = style_map.get("borderRadius") {
                    styles.push_str(&format!("border-radius: {};", rad));
                }
                if let Some(font) = style_map.get("fontSize") {
                    styles.push_str(&format!("font-size: {};", font));
                }
                if let Some(weight) = style_map.get("fontWeight") {
                    styles.push_str(&format!("font-weight: {};", weight));
                }
            } else {
                wrapper_styles.push_str("margin: 4px;");
            }
        }
    } else {
        if !is_flow_layout {
            if let Some(ref p) = active_placement {
                if node.id != "root" {
                    let has_top = node
                        .props
                        .style
                        .as_ref()
                        .map(|s| s.contains_key("top") || s.contains_key("Top"))
                        .unwrap_or(false);
                    let has_left = node
                        .props
                        .style
                        .as_ref()
                        .map(|s| s.contains_key("left") || s.contains_key("Left"))
                        .unwrap_or(false);
                    let has_position = node
                        .props
                        .style
                        .as_ref()
                        .map(|s| s.contains_key("position") || s.contains_key("Position"))
                        .unwrap_or(false);

                    if !has_position && (!has_top || !has_left) {
                        styles.push_str("position: absolute;");
                    }
                    styles.push_str("align-self: flex-start;");

                    if !has_top {
                        if let Some(ref t) = p.top {
                            styles.push_str(&format!("top: {};", t));
                        }
                    }
                    if !has_left {
                        if let Some(ref l) = p.left {
                            styles.push_str(&format!("left: {};", l));
                        }
                    }
                }
            }
        }
    }

    let element_content = match node.component_type.as_str() {
        "Flex" => {
            let dir = node
                .props
                .direction
                .clone()
                .unwrap_or_else(|| "row".to_string());
            let base_style = if node.id == "root" {
                "position: relative; display: flex; box-sizing: border-box; overflow: auto; width: 100%; min-height: 100vh; padding: 24px;"
            } else {
                "display: flex; box-sizing: border-box;"
            };
            let flex_style = format!("flex-direction: {}; {} {}", dir, styles, base_style);

            let actions = node.on_click.clone();
            let has_actions = !actions.is_empty();

            rsx! {
                div {
                    id: "{node.id}",
                    style: "{flex_style}",
                    onclick: move |_| {
                        if has_actions {
                            let acts = actions.clone();
                            let item = local_item.clone();
                            spawn(async move {
                                execute_actions(acts, data_state, item).await;
                            });
                        }
                    },
                    for child in node.children {
                        ComponentRenderer {
                            node: child,
                            selected_id,
                            on_select,
                            on_drop,
                            on_delete,
                            on_resize_start,
                            on_drag_start,
                            active_breakpoint: active_breakpoint.clone()
                        }
                    }
                }
            }
        }

        "Card" => {
            let title = node
                .props
                .title
                .clone()
                .unwrap_or_else(|| "Card Title".to_string());
            let base_style = "position: relative; background-color: #ffffff; border-radius: var(--radius, 12px); padding: 20px; border: 1px solid rgba(0,0,0,0.08); font-family: 'Outfit', sans-serif; box-shadow: 0 4px 20px rgba(0,0,0,0.02); display: flex; flex-direction: column; gap: 12px;";
            let card_style = format!("{} {}", base_style, styles);

            let actions = node.on_click.clone();
            let has_actions = !actions.is_empty();

            rsx! {
                div {
                    id: "{node.id}",
                    style: "{card_style}",
                    onclick: move |_| {
                        if has_actions {
                            let acts = actions.clone();
                            let item = local_item.clone();
                            spawn(async move {
                                execute_actions(acts, data_state, item).await;
                            });
                        }
                    },
                    if !title.is_empty() {
                        div {
                            style: "font-weight: 700; font-size: 16px; color: #0f172a; border-bottom: 1px solid #f1f5f9; padding-bottom: 8px; margin-bottom: 4px;",
                            "{title}"
                        }
                    }
                    for child in node.children {
                        ComponentRenderer {
                            node: child,
                            selected_id,
                            on_select,
                            on_drop,
                            on_delete,
                            on_resize_start,
                            on_drag_start,
                            active_breakpoint: active_breakpoint.clone()
                        }
                    }
                }
            }
        }

        "DrawerPanel" => rsx! {
        Hooked_DrawerPanel {
            node: node.clone(),
            styles: styles.clone(),
            selected_id,
            on_select,
            on_drop,
            on_delete,
            on_resize_start,
            on_drag_start
                }
            },

        "Modal" => rsx! {
        Hooked_Modal {
            node: node.clone(),
            styles: styles.clone(),
            selected_id,
            on_select,
            on_drop,
            on_delete,
            on_resize_start,
            on_drag_start
                }
            },

        "Text" => {
            let content = if let Some(ref bind_expr) = node.props.bind {
                let wrapped = format!("{{{{{}}}}}", bind_expr);
                resolve_string_templates(&wrapped, local_item.as_ref(), &data_state.read())
            } else {
                let raw_content = node.props.content.clone().unwrap_or_default();
                resolve_string_templates(&raw_content, local_item.as_ref(), &data_state.read())
            };
            let variant = node
                .props
                .variant
                .clone()
                .unwrap_or_else(|| "p".to_string());

            let base_style = "font-family: 'Outfit', sans-serif; margin: 0; line-height: 1.6;";
            let text_style = format!("{} {}", base_style, styles);

            match variant.as_str() {
                "span" => rsx! { span { id: "{node.id}", style: "{text_style}", "{content}" } },
                "div" => rsx! { div { id: "{node.id}", style: "{text_style}", "{content}" } },
                "b" | "strong" => {
                    rsx! { strong { id: "{node.id}", style: "{text_style}", "{content}" } }
                }
                _ => rsx! { p { id: "{node.id}", style: "{text_style}", "{content}" } },
            }
        }

        "Heading" => {
            let raw_text = node.props.text.clone().unwrap_or_default();
            let text = resolve_string_templates(&raw_text, local_item.as_ref(), &data_state.read());
            let level = node.props.level.unwrap_or(1);
            let heading_style = format!("font-family: 'Outfit', sans-serif; color: var(--primary, #030213); font-weight: 700; margin: 0; {}", styles);

            match level {
                1 => rsx! { h1 { id: "{node.id}", style: "{heading_style}", "{text}" } },
                2 => rsx! { h2 { id: "{node.id}", style: "{heading_style}", "{text}" } },
                3 => rsx! { h3 { id: "{node.id}", style: "{heading_style}", "{text}" } },
                4 => rsx! { h4 { id: "{node.id}", style: "{heading_style}", "{text}" } },
                5 => rsx! { h5 { id: "{node.id}", style: "{heading_style}", "{text}" } },
                _ => rsx! { h6 { id: "{node.id}", style: "{heading_style}", "{text}" } },
            }
        }

        "Alert" => {
            let variant = node
                .props
                .variant
                .clone()
                .unwrap_or_else(|| "info".to_string());
            let title = node
                .props
                .title
                .clone()
                .unwrap_or_else(|| "Notice".to_string());
            let message = node
                .props
                .message
                .clone()
                .unwrap_or_else(|| "Alert details message.".to_string());

            let (bg, border, text_color, icon) = match variant.as_str() {
                "success" => ("#f0fdf4", "#bbf7d0", "#166534", "✅"),
                "warning" => ("#fffbeb", "#fef3c7", "#92400e", "⚠️"),
                "error" => ("#fef2f2", "#fee2e2", "#991b1b", "🚨"),
                _ => ("#f0f9ff", "#e0f2fe", "#075985", "ℹ️"),
            };

            let base_style = format!("background-color: {}; border: 1px solid {}; color: {}; border-radius: var(--radius, 10px); padding: 14px 16px; font-family: 'Outfit', sans-serif; display: flex; gap: 12px; align-items: flex-start; box-shadow: 0 2px 8px rgba(0,0,0,0.01);", bg, border, text_color);
            let alert_style = format!("{} {}", base_style, styles);

            rsx! {
                div {
                    id: "{node.id}",
                    style: "{alert_style}",
                    span { style: "font-size: 18px;", "{icon}" }
                    div {
                        style: "display: flex; flex-direction: column; gap: 3px;",
                        span { style: "font-weight: 700; font-size: 14px; text-transform: uppercase; letter-spacing: 0.02em;", "{title}" }
                        span { style: "font-size: 13px; opacity: 0.9;", "{message}" }
                    }
                }
            }
        }

        "StatusBadge" => {
            let label = node
                .props
                .label
                .clone()
                .unwrap_or_else(|| "Active".to_string());
            let variant = node
                .props
                .variant
                .clone()
                .unwrap_or_else(|| "success".to_string());

            let (bg, text_color) = match variant.as_str() {
                "error" | "failed" => ("#fee2e2", "#991b1b"),
                "warning" | "pending" => ("#fef3c7", "#92400e"),
                "info" | "processing" => ("#e0f2fe", "#075985"),
                _ => ("#dcfce7", "#166534"),
            };

            let base_style = format!("background-color: {}; color: {}; padding: 4px 10px; border-radius: 9999px; font-size: 11px; font-weight: 700; display: inline-flex; align-items: center; justify-content: center; text-transform: uppercase; letter-spacing: 0.04em; font-family: 'Outfit', sans-serif;", bg, text_color);
            let badge_style = format!("{} {}", base_style, styles);

            rsx! {
                span {
                    id: "{node.id}",
                    style: "{badge_style}",
                    "{label}"
                }
            }
        }

        "Breadcrumbs" => {
            let base_style = "display: flex; align-items: center; gap: 8px; font-family: 'Outfit', sans-serif; font-size: 13px; color: #64748b;";
            let breadcrumb_style = format!("{} {}", base_style, styles);

            rsx! {
                div {
                    id: "{node.id}",
                    style: "{breadcrumb_style}",
                    span { style: "cursor: pointer; transition: color 0.2s;", "Home" }
                    span { "/" }
                    span { style: "cursor: pointer; transition: color 0.2s;", "Dashboard" }
                    span { "/" }
                    span { style: "color: #0f172a; font-weight: 600;", "Visualizer" }
                }
            }
        }

        "Input" | "SearchInput" => {
            let label = node
                .props
                .label
                .clone()
                .unwrap_or_else(|| "Label".to_string());
            let placeholder = node
                .props
                .placeholder
                .clone()
                .unwrap_or_else(|| "Enter value...".to_string());
            let is_search = node.component_type == "SearchInput";
            let has_bind = node
                .props
                .bind
                .as_ref()
                .map(|value| !value.trim().is_empty())
                .unwrap_or(false);
            let input_value = node
                .props
                .extra
                .get("__boundValue")
                .map(json_value_to_display_string)
                .unwrap_or_default();
            let input_name = form_control_name(&node);
            let bind_node = node.clone();

            let base_style = "display: flex; flex-direction: column; gap: 6px; font-family: 'Outfit', sans-serif; font-size: 14px;";
            let input_container_style = format!("{} {}", base_style, styles);

            rsx! {
                div {
                    id: "{node.id}",
                    style: "{input_container_style}",
                    label { style: "font-weight: 600; color: #475569;", "{label}" }
                    div {
                        style: "position: relative; display: flex; align-items: center;",
                        if is_search {
                            span {
                                style: "position: absolute; left: 12px; color: #94a3b8; font-size: 14px;",
                                "🔍"
                            }
                        }
                        input {
                            r#type: "text",
                            name: "{input_name}",
                            placeholder: "{placeholder}",
                            value: if has_bind { input_value.clone() },
                            style: if is_search { "padding: 10px 12px 10px 36px;" } else { "padding: 10px 12px;" },
                            class: "styled-field",
                            style: "width: 100%; border-radius: var(--radius, 8px); border: 1px solid rgba(0,0,0,0.08); background-color: #ffffff; color: #1e293b; outline: none; box-sizing: border-box; font-size: 14px;",
                            oninput: move |evt| {
                                let mut data = data_state.write();
                                set_node_bind_value(&bind_node, &mut data, serde_json::Value::String(evt.value()));
                            },
                        }
                    }
                }
            }
        }

        "Textarea" => {
            let label = node
                .props
                .label
                .clone()
                .unwrap_or_else(|| "Comment".to_string());
            let placeholder = node
                .props
                .placeholder
                .clone()
                .unwrap_or_else(|| "Enter content here...".to_string());
            let has_bind = node
                .props
                .bind
                .as_ref()
                .map(|value| !value.trim().is_empty())
                .unwrap_or(false);
            let textarea_value = node
                .props
                .extra
                .get("__boundValue")
                .map(json_value_to_display_string)
                .unwrap_or_default();
            let textarea_name = form_control_name(&node);
            let bind_node = node.clone();

            let base_style = "display: flex; flex-direction: column; gap: 6px; font-family: 'Outfit', sans-serif; font-size: 14px;";
            let area_container_style = format!("{} {}", base_style, styles);

            rsx! {
                div {
                    id: "{node.id}",
                    style: "{area_container_style}",
                    label { style: "font-weight: 600; color: #475569;", "{label}" }
                    textarea {
                        name: "{textarea_name}",
                        placeholder: "{placeholder}",
                        value: if has_bind { textarea_value.clone() },
                        style: "padding: 10px 12px; min-height: 80px; border-radius: var(--radius, 8px); border: 1px solid rgba(0,0,0,0.08); background-color: #ffffff; color: #1e293b; outline: none; font-family: inherit; font-size: 14px; box-sizing: border-box; resize: vertical;",
                        oninput: move |evt| {
                            let mut data = data_state.write();
                            set_node_bind_value(&bind_node, &mut data, serde_json::Value::String(evt.value()));
                        },
                    }
                }
            }
        }

        "Select" => rsx! {
            Hooked_Select {
                node: node.clone(),
                styles: styles.clone(),
                selected_id,
                on_select,
                on_drop,
                on_delete
            }
        },

        "Checkbox" => rsx! {
        Hooked_Checkbox {
            node: node.clone(),
            styles: styles.clone(),
            selected_id,
            on_select,
            on_drop,
            on_delete
                }
            },

        "Toggle" => rsx! {
        Hooked_Toggle {
            node: node.clone(),
            styles: styles.clone(),
            selected_id,
            on_select,
            on_drop,
            on_delete
                }
            },

        "Form" => {
            let base_style = "display: flex; flex-direction: column; gap: 14px; box-sizing: border-box; width: 100%;";
            let form_style = format!("{} {}", base_style, styles);
            let actions = node.on_submit.clone();
            let bind = node.props.bind.clone();
            let submit_item = local_item.clone();

            rsx! {
                form {
                    id: "{node.id}",
                    style: "{form_style}",
                    onsubmit: move |e| {
                        e.prevent_default();
                        let form_values = form_event_values_to_json(e.values());
                        if let Some(bind_path) = bind.as_deref() {
                            let mut data = data_state.write();
                            set_json_value_at_path(&mut data, bind_path, form_values.clone());
                        }
                        if !actions.is_empty() {
                            let acts = actions.clone();
                            let scope = form_submit_scope(form_values, submit_item.clone());
                            spawn(async move {
                                execute_actions(acts, data_state, Some(scope)).await;
                            });
                        }
                    },
                    for child in node.children {
                        ComponentRenderer {
                            node: child,
                            selected_id,
                            on_select,
                            on_drop,
                            on_delete,
                            on_resize_start,
                            on_drag_start
                        }
                    }
                }
            }
        }

        "SignaturePad" => rsx! {
        Hooked_SignaturePad {
            node: node.clone(),
            styles: styles.clone(),
            selected_id,
            on_select,
            on_drop,
            on_delete
                }
            },

        "MultiSelectDropdown" => rsx! {
        Hooked_MultiSelectDropdown {
            node: node.clone(),
            styles: styles.clone(),
            selected_id,
            on_select,
            on_drop,
            on_delete
                }
            },

        "Button" => rsx! {
        Hooked_Button {
            node: node.clone(),
            styles: styles.clone(),
            selected_id,
            on_select,
            on_drop,
            on_delete
                }
            },

        "nav-button" => {
            let text = node
                .props
                .text
                .clone()
                .unwrap_or_else(|| "Link".to_string());
            let target = node.props.target.clone().unwrap_or_else(|| "#".to_string());

            let base_style = "display: inline-flex; align-items: center; font-family: 'Outfit', sans-serif; font-size: 14px; font-weight: 600; text-decoration: none; color: #475569; transition: color 0.15s; padding: 6px 12px; border-radius: 6px;";
            let nav_style = format!("{} {}", base_style, styles);

            rsx! {
                a {
                    id: "{node.id}",
                    href: "{target}",
                    style: "{nav_style}",
                    "{text}"
                }
            }
        }

        "Tabs" => rsx! {
        Hooked_Tabs {
            node: node.clone(),
            styles: styles.clone(),
            selected_id,
            on_select,
            on_drop,
            on_delete,
            on_resize_start,
            on_drag_start
                }
            },

        "Tab" => {
            // Re-render handled inline by Tabs container wrapper.
            rsx! {
                div {
                    id: "{node.id}",
                    style: "display: none;",
                }
            }
        }

        "Table" => {
            // Static table: columns is Vec<String>, rows is Vec<HashMap>
            let columns: Vec<String> = node
                .props
                .columns
                .clone()
                .and_then(|v| serde_json::from_value(v).ok())
                .unwrap_or_default();
            let rows: Vec<std::collections::HashMap<String, serde_json::Value>> = node
                .props
                .rows
                .clone()
                .and_then(|v| serde_json::from_value(v).ok())
                .unwrap_or_default();
            let base_style = "border-collapse: collapse; text-align: left; background-color: #ffffff; border-radius: var(--radius, 12px); overflow: hidden; border: 1px solid var(--border, rgba(0,0,0,0.08)); font-family: 'Outfit', sans-serif; font-size: 14px; box-shadow: 0 10px 25px -5px rgba(0, 0, 0, 0.05);";
            let table_style = format!("{} {}", base_style, styles);
            rsx! {
                table {
                    id: "{node.id}",
                    style: "{table_style}",
                    thead {
                        style: "background: linear-gradient(to right, #f8fafc, #f1f5f9); border-bottom: 1px solid rgba(0,0,0,0.08);",
                        tr {
                            for col in &columns {
                                th { style: "padding: 14px 20px; font-weight: 600; color: #64748b; text-transform: uppercase; font-size: 11px; letter-spacing: 0.08em;", "{col}" }
                            }
                        }
                    }
                    tbody {
                        for (i, row) in rows.iter().enumerate() {
                            {
                                let row_bg = if i % 2 == 1 { "background-color: #f8fafc;" } else { "background-color: #ffffff;" };
                                rsx! {
                                    tr { key: "{i}", style: "{row_bg}",
                                        for col in &columns {
                                            { let cv = row.get(col).map(|v| match v { serde_json::Value::String(s) => s.clone(), o => o.to_string() }).unwrap_or_default();
                                              rsx! { td { style: "padding: 14px 20px; color: #1e293b; border-bottom: 1px solid rgba(0,0,0,0.04);", "{cv}" } } }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        "DynamicTable" => rsx! {
            Hooked_DynamicTable {
                node: node.clone(),
                styles: styles.clone(),
                selected_id,
                on_select,
                on_drop,
                on_delete,
                on_resize_start,
                on_drag_start,
            }
        },

        "HierarchyTable" => rsx! {
        Hooked_HierarchyTable {
            node: node.clone(),
            styles: styles.clone(),
            selected_id,
            on_select,
            on_drop,
            on_delete
                }
            },

        "DatePicker" | "TimePicker" => {
            let label = node
                .props
                .label
                .clone()
                .unwrap_or_else(|| "Select Option".to_string());
            let placeholder = node.props.placeholder.clone().unwrap_or_default();
            let is_date = node.component_type == "DatePicker";
            let input_type = if is_date { "date" } else { "time" };
            let required = node.props.required.unwrap_or(false);
            let has_bind = node
                .props
                .bind
                .as_ref()
                .map(|value| !value.trim().is_empty())
                .unwrap_or(false);
            let input_value = node
                .props
                .extra
                .get("__boundValue")
                .map(json_value_to_display_string)
                .unwrap_or_default();
            let bind_node = node.clone();

            let base_style = "display: flex; flex-direction: column; gap: 6px; font-family: 'Outfit', sans-serif; font-size: 14px;";
            let container_style = format!("{} {}", base_style, styles);

            rsx! {
                div {
                    id: "{node.id}",
                    style: "{container_style}",
                    label {
                        style: "font-weight: 600; color: #475569;",
                        "{label}"
                        if required {
                            span { style: "color: #ef4444; margin-left: 2px;", "*" }
                        }
                    }
                    div {
                        style: "position: relative; display: flex; align-items: center;",
                        input {
                            r#type: "{input_type}",
                            placeholder: "{placeholder}",
                            required: required,
                            value: if has_bind { input_value.clone() },
                            style: "padding: 12px 14px; border-radius: var(--radius, 10px); border: 1px solid rgba(0,0,0,0.08); background-color: #ffffff; color: #1e293b; outline: none; transition: all 0.2s; width: 100%; box-sizing: border-box; font-family: inherit; font-size: 14px; box-shadow: inset 0 1px 2px rgba(0,0,0,0.02);",
                            oninput: move |evt| {
                                let mut data = data_state.write();
                                set_node_bind_value(&bind_node, &mut data, serde_json::Value::String(evt.value()));
                            },
                        }
                    }
                }
            }
        }

        "TimeViewer" => rsx! {
        Hooked_TimeViewer {
            node: node.clone(),
            styles: styles.clone(),
            selected_id,
            on_select,
            on_drop,
            on_delete
                }
            },

        "Chart" => {
            let chart_type = node
                .props
                .chart_type
                .clone()
                .unwrap_or_else(|| "bar".to_string());

            if chart_type == "progressbar" {
                let data_val = node.props.data.clone().unwrap_or(serde_json::Value::Null);
                let label = data_val
                    .get("label")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Progress")
                    .to_string();
                let val = data_val
                    .get("value")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0);
                let max = data_val
                    .get("max")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(100.0);
                let pct = if max > 0.0 { (val / max) * 100.0 } else { 0.0 };

                let has_height = styles.contains("height:") || styles.contains("Height:");
                let height_style = if has_height {
                    ""
                } else {
                    "height: 320px; min-height: 320px;"
                };

                let bar_style = format!(
                    "position: relative; width: 100%; {} font-family: 'Outfit', sans-serif; display: flex; flex-direction: column; justify-content: center; padding: 24px; box-sizing: border-box; {}",
                    height_style,
                    styles
                );

                return rsx! {
                    div {
                        id: "{node.id}",
                        style: "{bar_style}",
                        div {
                            style: "width: 100%;",
                            div {
                                style: "display: flex; justify-content: space-between; margin-bottom: 8px; font-size: 14px; font-weight: 500; color: #475569;",
                                span { "{label}" }
                                span { "{val}/{max}" }
                            }
                            div {
                                style: "width: 100%; height: 32px; background-color: #e2e8f0; border-radius: 9999px; overflow: hidden; display: flex; align-items: stretch;",
                                div {
                                    style: "height: 100%; border-radius: 9999px; background: linear-gradient(90deg, #4a90e2 0%, #50e3c2 100%); transition: width 0.3s ease-out; width: {pct}%;",
                                }
                            }
                        }
                    }
                };
            }

            let chart_data: Vec<crate::models::ChartItem> = node
                .props
                .data
                .clone()
                .and_then(|d| serde_json::from_value(d).ok())
                .unwrap_or_else(|| {
                    vec![
                        crate::models::ChartItem {
                            name: Some("Jan".to_string()),
                            value: Some(40.0),
                        },
                        crate::models::ChartItem {
                            name: Some("Feb".to_string()),
                            value: Some(60.0),
                        },
                        crate::models::ChartItem {
                            name: Some("Mar".to_string()),
                            value: Some(55.0),
                        },
                        crate::models::ChartItem {
                            name: Some("Apr".to_string()),
                            value: Some(80.0),
                        },
                    ]
                });

            let has_height = styles.contains("height:") || styles.contains("Height:");
            let height_style = if has_height {
                ""
            } else {
                "height: 320px; min-height: 320px;"
            };
            let base_style = format!("background-color: #ffffff; border-radius: var(--radius, 14px); padding: 20px; border: 1px solid rgba(0,0,0,0.08); font-family: 'Outfit', sans-serif; box-shadow: 0 10px 30px rgba(0,0,0,0.04); display: flex; flex-direction: column; overflow: hidden; {}", height_style);
            let chart_style = format!("{} {}", base_style, styles);

            let max_val = chart_data
                .iter()
                .map(|item| item.value.unwrap_or(0.0))
                .fold(0.0f64, |a, b| a.max(b));
            let max_val = if max_val == 0.0 { 100.0 } else { max_val };

            let svg_w = 460.0;
            let svg_h = 220.0;
            let margin_left = 40.0;
            let margin_right = 20.0;
            let margin_top = 20.0;
            let margin_bottom = 30.0;
            let chart_w = svg_w - margin_left - margin_right;
            let chart_h = svg_h - margin_top - margin_bottom;

            rsx! {
                div {
                    id: "{node.id}",
                    style: "{chart_style}",
                    span {
                        style: "font-weight: 700; color: #1e293b; font-size: 15px; margin-bottom: 12px; text-transform: uppercase; letter-spacing: 0.04em;",
                        "Data Trends ({chart_type})"
                    }
                    svg {
                        width: "100%",
                        height: "100%",
                        view_box: "0 0 {svg_w} {svg_h}",
                        style: "flex-grow: 1; overflow: visible;",

                        defs {
                            linearGradient {
                                id: "chart-bar-grad",
                                x1: "0",
                                y1: "0",
                                x2: "0",
                                y2: "1",
                                stop { offset: "0%", stop_color: "#6366f1" }
                                stop { offset: "100%", stop_color: "#4f46e5" }
                            }
                            linearGradient {
                                id: "chart-line-area",
                                x1: "0",
                                y1: "0",
                                x2: "0",
                                y2: "1",
                                stop { offset: "0%", stop_color: "#3b82f6", stop_opacity: "0.25" }
                                stop { offset: "100%", stop_color: "#3b82f6", stop_opacity: "0.0" }
                            }
                        }

                        for j in 0..=4 {
                            {
                                let y_val = margin_top + (j as f64 * (chart_h / 4.0));
                                let grid_label = (max_val - (j as f64 * (max_val / 4.0))) as i32;
                                rsx! {
                                    line {
                                        x1: "{margin_left}",
                                        y1: "{y_val}",
                                        x2: "{svg_w - margin_right}",
                                        y2: "{y_val}",
                                        stroke: "#e2e8f0",
                                        stroke_width: "1",
                                        stroke_dasharray: "3,3",
                                    }
                                    text {
                                        x: "{margin_left - 8.0}",
                                        y: "{y_val + 4.0}",
                                        fill: "#94a3b8",
                                        font_size: "10",
                                        text_anchor: "end",
                                        font_family: "monospace",
                                        "{grid_label}"
                                    }
                                }
                            }
                        }

                        if chart_type == "bar" {
                            {
                                let bar_width = (chart_w / chart_data.len() as f64) * 0.6;
                                let col_width = chart_w / chart_data.len() as f64;

                                rsx! {
                                    g {
                                        for (i, item) in chart_data.iter().enumerate() {
                                            {
                                                let val = item.value.unwrap_or(0.0);
                                                let name = item.name.clone().unwrap_or_default();
                                                let bar_h = (val / max_val) * chart_h;
                                                let x_pos = margin_left + (i as f64 * col_width) + (col_width - bar_width) / 2.0;
                                                let y_pos = margin_top + chart_h - bar_h;

                                                rsx! {
                                                    rect {
                                                        x: "{x_pos}",
                                                        y: "{y_pos}",
                                                        width: "{bar_width}",
                                                        height: "{bar_h}",
                                                        fill: "url(#chart-bar-grad)",
                                                        rx: "3",
                                                    }
                                                    text {
                                                        x: "{x_pos + bar_width / 2.0}",
                                                        y: "{y_pos - 6.0}",
                                                        fill: "#4f46e5",
                                                        font_size: "10",
                                                        font_weight: "600",
                                                        text_anchor: "middle",
                                                        "{val}"
                                                    }
                                                    text {
                                                        x: "{margin_left + (i as f64 * col_width) + col_width / 2.0}",
                                                        y: "{margin_top + chart_h + 16.0}",
                                                        fill: "#64748b",
                                                        font_size: "10",
                                                        text_anchor: "middle",
                                                        "{name}"
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        } else {
                            {
                                let col_width = chart_w / (chart_data.len() - 1).max(1) as f64;
                                let points_str = chart_data.iter().enumerate().map(|(i, item)| {
                                    let val = item.value.unwrap_or(0.0);
                                    let x = margin_left + (i as f64 * col_width);
                                    let y = margin_top + chart_h - ((val / max_val) * chart_h);
                                    format!("{},{}", x, y)
                                }).collect::<Vec<String>>().join(" ");

                                let first_x = margin_left;
                                let last_x = margin_left + chart_w;
                                let base_y = margin_top + chart_h;
                                let area_points_str = format!("{},{} {} {},{}", first_x, base_y, points_str, last_x, base_y);

                                rsx! {
                                    g {
                                        polygon {
                                            points: "{area_points_str}",
                                            fill: "url(#chart-line-area)",
                                        }
                                        polyline {
                                            points: "{points_str}",
                                            fill: "none",
                                            stroke: "#3b82f6",
                                            stroke_width: "2.5",
                                        }
                                        for (i, item) in chart_data.iter().enumerate() {
                                            {
                                                let val = item.value.unwrap_or(0.0);
                                                let name = item.name.clone().unwrap_or_default();
                                                let x = margin_left + (i as f64 * col_width);
                                                let y = margin_top + chart_h - ((val / max_val) * chart_h);

                                                rsx! {
                                                    circle { cx: "{x}", cy: "{y}", r: "4", fill: "#ffffff", stroke: "#3b82f6", stroke_width: "2" }
                                                    text {
                                                        x: "{x}",
                                                        y: "{y - 8.0}",
                                                        fill: "#2563eb",
                                                        font_size: "10",
                                                        font_weight: "600",
                                                        text_anchor: "middle",
                                                        "{val}"
                                                    }
                                                    text {
                                                        x: "{x}",
                                                        y: "{margin_top + chart_h + 16.0}",
                                                        fill: "#64748b",
                                                        font_size: "10",
                                                        text_anchor: "middle",
                                                        "{name}"
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        "GaugeChart" => {
            let title = node
                .props
                .title
                .clone()
                .unwrap_or_else(|| "System Load".to_string());
            let value = node
                .props
                .extra
                .get("value")
                .and_then(|v| v.as_f64())
                .unwrap_or(72.0);

            // Needle angle calculation
            let angle = -90.0 + (value / 100.0) * 180.0;
            let needle_transform = format!("rotate({}, 100, 90)", angle);

            let base_style = "background-color: #ffffff; border-radius: 12px; padding: 16px; border: 1px solid rgba(0,0,0,0.08); font-family: 'Outfit', sans-serif; display: flex; flex-direction: column; align-items: center; width: 220px; box-shadow: 0 4px 12px rgba(0,0,0,0.02);";
            let gc_style = format!("{} {}", base_style, styles);

            rsx! {
                div {
                    id: "{node.id}",
                    style: "{gc_style}",
                    span { style: "font-weight: 700; font-size: 13px; color: #475569; margin-bottom: 6px; text-transform: uppercase;", "{title}" }
                    svg {
                        width: "160",
                        height: "100",
                        view_box: "0 0 200 110",
                        // Gauge arc background
                        path {
                            d: "M 20 90 A 80 80 0 0 1 180 90",
                            fill: "none",
                            stroke: "#f1f5f9",
                            stroke_width: "16",
                            stroke_linecap: "round",
                        }
                        // Gauge active arc
                        path {
                            d: "M 20 90 A 80 80 0 0 1 180 90",
                            fill: "none",
                            stroke: "#3b82f6",
                            stroke_width: "16",
                            stroke_linecap: "round",
                            stroke_dasharray: "251.2",
                            stroke_dashoffset: "{251.2 - (value / 100.0) * 251.2}",
                        }
                        // Needle
                        polygon {
                            points: "97,90 100,20 103,90",
                            fill: "#0f172a",
                            transform: "{needle_transform}",
                        }
                        // Pivot
                        circle { cx: "100", cy: "90", r: "8", fill: "#0f172a" }
                    }
                    span { style: "font-size: 20px; font-weight: 800; color: #1e293b; margin-top: -6px;", "{value}%" }
                }
            }
        }

        "ProgressRing" => {
            let value = node
                .props
                .extra
                .get("value")
                .and_then(|v| v.as_f64())
                .unwrap_or(65.0);
            let circumference = 2.0 * std::f64::consts::PI * 34.0;
            let offset = circumference - (value / 100.0) * circumference;

            let base_style = "background-color: #ffffff; border-radius: 12px; padding: 14px; border: 1px solid rgba(0,0,0,0.08); font-family: 'Outfit', sans-serif; display: flex; align-items: center; gap: 12px; width: 180px; box-shadow: 0 4px 10px rgba(0,0,0,0.02);";
            let pr_style = format!("{} {}", base_style, styles);

            rsx! {
                div {
                    id: "{node.id}",
                    style: "{pr_style}",
                    svg {
                        width: "80",
                        height: "80",
                        view_box: "0 0 80 80",
                        style: "transform: rotate(-90deg);",
                        circle {
                            cx: "40",
                            cy: "40",
                            r: "34",
                            fill: "none",
                            stroke: "#f1f5f9",
                            stroke_width: "6",
                        }
                        circle {
                            cx: "40",
                            cy: "40",
                            r: "34",
                            fill: "none",
                            stroke: "#10b981",
                            stroke_width: "6",
                            stroke_dasharray: "{circumference}",
                            stroke_dashoffset: "{offset}",
                            stroke_linecap: "round",
                        }
                    }
                    div {
                        style: "display: flex; flex-direction: column; gap: 2px;",
                        span { style: "font-size: 18px; font-weight: 800; color: #0f172a;", "{value}%" }
                        span { style: "font-size: 11px; color: #94a3b8; font-weight: 500;", "Task Ratio" }
                    }
                }
            }
        }

        "TargetKpiCard" => {
            let title = node
                .props
                .title
                .clone()
                .unwrap_or_else(|| "Conversion Rate".to_string());
            let actual = node
                .props
                .extra
                .get("actual")
                .and_then(|v| v.as_str())
                .unwrap_or("4.8%");
            let target = node
                .props
                .extra
                .get("target")
                .and_then(|v| v.as_str())
                .unwrap_or("5.0%");

            let base_style = "background-color: #ffffff; border-radius: 12px; padding: 18px; border: 1px solid rgba(0,0,0,0.08); font-family: 'Outfit', sans-serif; display: flex; flex-direction: column; width: 200px; box-shadow: 0 4px 10px rgba(0,0,0,0.02);";
            let tk_style = format!("{} {}", base_style, styles);

            rsx! {
                div {
                    id: "{node.id}",
                    style: "{tk_style}",
                    span { style: "font-size: 11px; color: #64748b; font-weight: 700; text-transform: uppercase; letter-spacing: 0.05em; margin-bottom: 6px;", "{title}" }
                    span { style: "font-size: 26px; font-weight: 800; color: #0f172a;", "{actual}" }
                    div {
                        style: "display: flex; justify-content: space-between; font-size: 12px; color: #94a3b8; border-top: 1px solid #f1f5f9; padding-top: 6px; margin-top: 8px;",
                        span { "Target: " }
                        span { style: "font-weight: 600; color: #475569;", "{target}" }
                    }
                }
            }
        }

        "Image" => {
            let src = node
                .props
                .extra
                .get("src")
                .and_then(|v| v.as_str())
                .unwrap_or("https://picsum.photos/400/250");
            let alt = node
                .props
                .extra
                .get("alt")
                .and_then(|v| v.as_str())
                .unwrap_or("visualizer-asset");

            let base_style = "max-width: 100%; border-radius: var(--radius, 10px); object-fit: cover; display: block; border: 1px solid rgba(0,0,0,0.05);";
            let img_style = format!("{} {}", base_style, styles);

            rsx! {
                img {
                    id: "{node.id}",
                    src: "{src}",
                    alt: "{alt}",
                    style: "{img_style}",
                }
            }
        }

        "Video" => {
            let src = node
                .props
                .extra
                .get("src")
                .and_then(|v| v.as_str())
                .unwrap_or("https://www.w3schools.com/html/mov_bbb.mp4");

            let base_style = "max-width: 100%; border-radius: var(--radius, 12px); overflow: hidden; background-color: #0f172a; box-shadow: 0 10px 25px rgba(0,0,0,0.08); display: block;";
            let video_style = format!("{} {}", base_style, styles);

            rsx! {
                video {
                    id: "{node.id}",
                    src: "{src}",
                    controls: true,
                    style: "{video_style}",
                }
            }
        }

        "Audio" => {
            let src = node
                .props
                .extra
                .get("src")
                .and_then(|v| v.as_str())
                .unwrap_or("https://www.w3schools.com/html/horse.mp3");

            let base_style = "max-width: 100%; border-radius: var(--radius, 8px); padding: 8px; background-color: #f8fafc; border: 1px solid #e2e8f0; display: inline-flex; align-items: center; box-shadow: 0 4px 6px rgba(0,0,0,0.02);";
            let audio_style = format!("{} {}", base_style, styles);

            rsx! {
                audio {
                    id: "{node.id}",
                    src: "{src}",
                    controls: true,
                    style: "{audio_style}",
                }
            }
        }

        "ImageGallery" => rsx! {
        Hooked_ImageGallery {
            node: node.clone(),
            styles: styles.clone(),
            selected_id,
            on_select,
            on_drop,
            on_delete
                }
            },

        "ImageWithAnnotations" => rsx! {
        Hooked_ImageWithAnnotations {
            node: node.clone(),
            styles: styles.clone(),
            selected_id,
            on_select,
            on_drop,
            on_delete
                }
            },

        "PlaylistPlayer" => rsx! {
        Hooked_PlaylistPlayer {
            node: node.clone(),
            styles: styles.clone(),
            selected_id,
            on_select,
            on_drop,
            on_delete
                }
            },

        "KanbanBoard" => rsx! {
        Hooked_KanbanBoard {
            node: node.clone(),
            styles: styles.clone(),
            selected_id,
            on_select,
            on_drop,
            on_delete
                }
            },

        "Timeline" => {
            let events = vec![
                ("3:00 PM", "Security check initialized", "Completed"),
                ("3:45 PM", "File annotations merged", "Active"),
                ("4:00 PM", "Production build scheduled", "Pending"),
            ];

            let base_style = "display: flex; flex-direction: column; font-family: 'Outfit', sans-serif; gap: 16px;";
            let timeline_style = format!("{} {}", base_style, styles);

            rsx! {
                div {
                    id: "{node.id}",
                    style: "{timeline_style}",
                    for (time, title, status) in events {
                        {
                            let circle_color = if status == "Completed" {
                                "#10b981"
                            } else if status == "Active" {
                                "#3b82f6"
                            } else {
                                "#cbd5e1"
                            };
                            rsx! {
                                div {
                                    key: "{title}",
                                    style: "display: flex; gap: 12px; position: relative;",
                                    div {
                                        style: "display: flex; flex-direction: column; align-items: center;",
                                        div {
                                            style: "width: 12px; height: 12px; border-radius: 50%; background-color: {circle_color}; border: 2px solid #ffffff; box-shadow: 0 2px 4px rgba(0,0,0,0.1); z-index: 2;",
                                        }
                                        div {
                                            style: "width: 2px; flex-grow: 1; background-color: #cbd5e1; margin-top: 4px; margin-bottom: -4px; z-index: 1;",
                                        }
                                    }
                                    div {
                                        style: "display: flex; flex-direction: column; gap: 2px; padding-bottom: 12px;",
                                        span { style: "font-family: monospace; font-size: 11px; font-weight: 700; color: #3b82f6;", "{time}" }
                                        span { style: "font-size: 13px; font-weight: 600; color: #1e293b;", "{title}" }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        "Chat" | "ChatViewer" => rsx! {
        Hooked_ChatViewer {
            node: node.clone(),
            styles: styles.clone(),
            selected_id,
            on_select,
            on_drop,
            on_delete
                }
            },

        "ActivityFeed" => {
            let feeds = vec![
                ("Atushi17", "pushed 2 commits to main branch", "3 min ago"),
                ("Sarah", "updated Accordion state styles", "15 min ago"),
                (
                    "CI System",
                    "wasm pipeline compilation complete",
                    "1 hr ago",
                ),
            ];

            let base_style = "background-color: #ffffff; border-radius: 12px; padding: 18px; border: 1px solid rgba(0,0,0,0.08); font-family: 'Outfit', sans-serif; display: flex; flex-direction: column; gap: 12px; width: 320px; box-shadow: 0 4px 10px rgba(0,0,0,0.02);";
            let feed_style = format!("{} {}", base_style, styles);

            rsx! {
                div {
                    id: "{node.id}",
                    style: "{feed_style}",
                    span { style: "font-weight: 700; color: #0f172a; font-size: 14px; text-transform: uppercase; letter-spacing: 0.05em; border-bottom: 1px solid #f1f5f9; padding-bottom: 8px;", "Activity Log" }
                    div {
                        style: "display: flex; flex-direction: column; gap: 12px;",
                        for (user, desc, time) in feeds {
                            div {
                                key: "{desc}",
                                style: "display: flex; gap: 10px; align-items: flex-start;",
                                div {
                                    style: "width: 28px; height: 28px; border-radius: 50%; background-color: #e2e8f0; display: flex; align-items: center; justify-content: center; font-weight: bold; font-size: 11px; color: #64748b;",
                                    "{user.chars().next().unwrap_or('?')}"
                                }
                                div {
                                    style: "display: flex; flex-direction: column; gap: 2px;",
                                    span { style: "font-size: 12px; color: #334155; line-height: 1.4;", strong { "{user} " } "{desc}" }
                                    span { style: "font-size: 10px; color: #94a3b8; font-weight: 500;", "{time}" }
                                }
                            }
                        }
                    }
                }
            }
        }

        "CommentSection" => rsx! {
        Hooked_CommentSection {
            node: node.clone(),
            styles: styles.clone(),
            selected_id,
            on_select,
            on_drop,
            on_delete
                }
            },

        "CodeSandbox" => {
            let base_style = "background-color: #0f172a; border-radius: 12px; padding: 14px; border: 1px solid rgba(255,255,255,0.05); font-family: monospace; display: flex; flex-direction: column; gap: 10px; width: 340px; box-shadow: 0 10px 20px rgba(0,0,0,0.15); color: #e2e8f0; font-size: 12px;";
            let box_style = format!("{} {}", base_style, styles);
            let sandbox_code =
                "fn main() {\n    println!(\"Hello Dioxus WASM Layout Engine!\");\n}";

            rsx! {
                div {
                    id: "{node.id}",
                    style: "{box_style}",
                    div {
                        style: "display: flex; justify-content: space-between; border-bottom: 1px solid #334155; padding-bottom: 6px;",
                        span { style: "color: #38bdf8; font-weight: bold;", "index.rs" }
                        span { style: "color: #64748b;", "Sandbox Workspace" }
                    }
                    pre {
                        style: "margin: 0; color: #10b981; overflow-x: auto; line-height: 1.4;",
                        "{sandbox_code}"
                    }
                }
            }
        }

        "QrCodeGenerator" => {
            let base_style = "background-color: #ffffff; border-radius: 12px; padding: 16px; border: 1px solid rgba(0,0,0,0.08); font-family: 'Outfit', sans-serif; display: flex; flex-direction: column; align-items: center; gap: 10px; width: 160px; box-shadow: 0 4px 12px rgba(0,0,0,0.02);";
            let qr_style = format!("{} {}", base_style, styles);

            rsx! {
                div {
                    id: "{node.id}",
                    style: "{qr_style}",
                    span { style: "font-weight: 700; color: #475569; font-size: 11px; text-transform: uppercase;", "QR Verification" }
                    div {
                        style: "width: 100px; height: 100px; background-color: #0f172a; padding: 8px; border-radius: 6px; display: flex; flex-wrap: wrap; gap: 4px; box-sizing: border-box;",
                        for i in 0..16 {
                            {
                                let qr_bg = if i % 3 == 0 || i == 7 || i == 11 { "#ffffff" } else { "transparent" };
                                rsx! {
                                    div {
                                        key: "{i}",
                                        style: "width: 20px; height: 20px; background-color: {qr_bg}; border-radius: 2px;",
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        "StarRating" => rsx! {
        Hooked_StarRating {
            node: node.clone(),
            styles: styles.clone(),
            selected_id,
            on_select,
            on_drop,
            on_delete
                }
            },

        "FileUpload" | "FolderUpload" => rsx! {
        Hooked_FolderUpload {
            node: node.clone(),
            styles: styles.clone(),
            selected_id,
            on_select,
            on_drop,
            on_delete
                }
            },

        "DynamicCardGrid" => {
            let base_style = "display: grid; grid-template-columns: repeat(auto-fit, minmax(280px, 1fr)); gap: 16px; width: 100%; box-sizing: border-box;";
            let grid_style = format!("{} {}", base_style, styles);

            rsx! {
                div {
                    id: "{node.id}",
                    style: "{grid_style}",
                    for child in node.children {
                        ComponentRenderer {
                            node: child,
                            selected_id,
                            on_select,
                            on_drop,
                            on_delete,
                            on_resize_start,
                            on_drag_start
                        }
                    }
                }
            }
        }

        "Accordion" => rsx! {
        Hooked_Accordion {
            node: node.clone(),
            styles: styles.clone(),
            selected_id,
            on_select,
            on_drop,
            on_delete
                }
            },

        "WizardStepper" => rsx! {
        Hooked_WizardStepper {
            node: node.clone(),
            styles: styles.clone(),
            selected_id,
            on_select,
            on_drop,
            on_delete
                }
            },

        "TagInput" => rsx! {
        Hooked_TagInput {
            node: node.clone(),
            styles: styles.clone(),
            selected_id,
            on_select,
            on_drop,
            on_delete
                }
            },

        "OtpInput" => rsx! {
        Hooked_OtpInput {
            node: node.clone(),
            styles: styles.clone(),
            selected_id,
            on_select,
            on_drop,
            on_delete
                }
            },

        "ColorPicker" => rsx! {
        Hooked_ColorPicker {
            node: node.clone(),
            styles: styles.clone(),
            selected_id,
            on_select,
            on_drop,
            on_delete
                }
            },

        "RichTextEditor" => rsx! {
        Hooked_RichTextEditor {
            node: node.clone(),
            styles: styles.clone(),
            selected_id,
            on_select,
            on_drop,
            on_delete
                }
            },

        "Layout" => {
            let layout_type = node
                .props
                .extra
                .get("layoutType")
                .or_else(|| node.props.extra.get("layout_type"))
                .and_then(|v| v.as_str())
                .unwrap_or("block")
                .to_lowercase();

            let (display_style, flex_dir) = match layout_type.as_str() {
                "grid" => ("display: grid;", String::new()),
                "flex" => {
                    let dir = node
                        .props
                        .direction
                        .clone()
                        .unwrap_or_else(|| "column".to_string());
                    ("display: flex;", format!("flex-direction: {};", dir))
                }
                _ => ("display: block;", String::new()),
            };

            let base_style = format!(
                "{} {} width: 100%; box-sizing: border-box; position: relative;",
                display_style, flex_dir
            );
            let layout_style = format!("{} {}", base_style, styles);

            let actions = node.on_click.clone();
            let has_actions = !actions.is_empty();

            rsx! {
                div {
                    id: "{node.id}",
                    style: "{layout_style}",
                    onclick: move |_| {
                        if has_actions {
                            let acts = actions.clone();
                            let item = local_item.clone();
                            spawn(async move {
                                execute_actions(acts, data_state, item).await;
                            });
                        }
                    },
                    for child in node.children {
                        ComponentRenderer {
                            node: child,
                            selected_id,
                            on_select,
                            on_drop,
                            on_delete,
                            on_resize_start,
                            on_drag_start
                        }
                    }
                }
            }
        }

        "Box" => {
            let base_style = "box-sizing: border-box; display: block;";
            let box_style = format!("{} {}", base_style, styles);

            let actions = node.on_click.clone();
            let has_actions = !actions.is_empty();

            rsx! {
                div {
                    id: "{node.id}",
                    style: "{box_style}",
                    onclick: move |_| {
                        if has_actions {
                            let acts = actions.clone();
                            let item = local_item.clone();
                            spawn(async move {
                                execute_actions(acts, data_state, item).await;
                            });
                        }
                    },
                    for child in node.children {
                        ComponentRenderer {
                            node: child,
                            selected_id,
                            on_select,
                            on_drop,
                            on_delete,
                            on_resize_start,
                            on_drag_start
                        }
                    }
                }
            }
        }

        "Divider" => {
            let divider_style = format!("border: 0; border-top: 1px solid rgba(0,0,0,0.08); margin: 16px 0; width: 100%; {}", styles);
            rsx! {
                hr {
                    id: "{node.id}",
                    style: "{divider_style}",
                }
            }
        }

        "Grid" => {
            let cols = node
                .props
                .extra
                .get("columnsCount")
                .and_then(|v| v.as_u64())
                .unwrap_or(3);
            let gap = node
                .props
                .extra
                .get("gap")
                .and_then(|v| v.as_str())
                .unwrap_or("16px");
            let grid_style = format!("display: grid; grid-template-columns: repeat({}, 1fr); gap: {}; width: 100%; box-sizing: border-box; {}", cols, gap, styles);

            let actions = node.on_click.clone();
            let has_actions = !actions.is_empty();

            rsx! {
                div {
                    id: "{node.id}",
                    style: "{grid_style}",
                    onclick: move |_| {
                        if has_actions {
                            let acts = actions.clone();
                            let item = local_item.clone();
                            spawn(async move {
                                execute_actions(acts, data_state, item).await;
                            });
                        }
                    },
                    for child in node.children {
                        ComponentRenderer {
                            node: child,
                            selected_id,
                            on_select,
                            on_drop,
                            on_delete,
                            on_resize_start,
                            on_drag_start
                        }
                    }
                }
            }
        }

        "Container" => {
            let base_style = "max-width: 1200px; margin: 0 auto; padding: 0 16px; width: 100%; box-sizing: border-box; display: block; position: relative;";
            let container_style = format!("{} {}", base_style, styles);

            let actions = node.on_click.clone();
            let has_actions = !actions.is_empty();

            rsx! {
                div {
                    id: "{node.id}",
                    style: "{container_style}",
                    onclick: move |_| {
                        if has_actions {
                            let acts = actions.clone();
                            let item = local_item.clone();
                            spawn(async move {
                                execute_actions(acts, data_state, item).await;
                            });
                        }
                    },
                    for child in node.children {
                        ComponentRenderer {
                            node: child,
                            selected_id,
                            on_select,
                            on_drop,
                            on_delete,
                            on_resize_start,
                            on_drag_start
                        }
                    }
                }
            }
        }

        "Stack" => {
            let direction = node
                .props
                .extra
                .get("direction")
                .and_then(|v| v.as_str())
                .unwrap_or("column");
            let gap = node
                .props
                .extra
                .get("spacing")
                .and_then(|v| v.as_str())
                .or_else(|| node.props.extra.get("gap").and_then(|v| v.as_str()))
                .unwrap_or("12px");
            let stack_style = format!("display: flex; flex-direction: {}; gap: {}; width: 100%; box-sizing: border-box; {}", direction, gap, styles);

            let actions = node.on_click.clone();
            let has_actions = !actions.is_empty();

            rsx! {
                div {
                    id: "{node.id}",
                    style: "{stack_style}",
                    onclick: move |_| {
                        if has_actions {
                            let acts = actions.clone();
                            let item = local_item.clone();
                            spawn(async move {
                                execute_actions(acts, data_state, item).await;
                            });
                        }
                    },
                    for child in node.children {
                        ComponentRenderer {
                            node: child,
                            selected_id,
                            on_select,
                            on_drop,
                            on_delete,
                            on_resize_start,
                            on_drag_start
                        }
                    }
                }
            }
        }

        "List" => {
            let base_style = "display: flex; flex-direction: column; gap: 8px; width: 100%; box-sizing: border-box;";
            let list_style = format!("{} {}", base_style, styles);
            rsx! {
                div {
                    id: "{node.id}",
                    style: "{list_style}",
                    for child in node.children {
                        ComponentRenderer {
                            node: child,
                            selected_id,
                            on_select,
                            on_drop,
                            on_delete,
                            on_resize_start,
                            on_drag_start
                        }
                    }
                }
            }
        }

        "Sidebar" => {
            let title = node
                .props
                .title
                .clone()
                .unwrap_or_else(|| "Navigation Sidebar".to_string());
            let base_style = "width: 260px; height: 100vh; background-color: #0f172a; color: #ffffff; border-right: 1px solid rgba(255,255,255,0.05); display: flex; flex-direction: column; font-family: 'Outfit', sans-serif; box-sizing: border-box;";
            let sidebar_style = format!("{} {}", base_style, styles);
            rsx! {
                div {
                    id: "{node.id}",
                    style: "{sidebar_style}",
                    div {
                        style: "padding: 20px; border-bottom: 1px solid rgba(255,255,255,0.05); display: flex; align-items: center; gap: 10px;",
                        span { style: "font-size: 22px;", "⚡" }
                        span { style: "font-weight: 800; font-size: 16px; tracking: 0.05em; color: #38bdf8;", "{title}" }
                    }
                    div {
                        style: "flex-grow: 1; padding: 16px; display: flex; flex-direction: column; gap: 8px; overflow-y: auto;",
                        for child in node.children {
                            ComponentRenderer {
                            node: child,
                            selected_id,
                            on_select,
                            on_drop,
                            on_delete,
                            on_resize_start,
                            on_drag_start
                        }
                    }
                    }
                    div {
                        style: "padding: 16px; border-top: 1px solid rgba(255,255,255,0.05); display: flex; align-items: center; gap: 10px; background-color: #020617;",
                        div {
                            style: "width: 32px; height: 32px; border-radius: 50%; background-color: #3b82f6; display: flex; align-items: center; justify-content: center; font-weight: bold; font-size: 13px;",
                            "U"
                        }
                        div {
                            style: "display: flex; flex-direction: column; gap: 1px;",
                            span { style: "font-size: 12px; font-weight: 600;", "Developer" }
                            span { style: "font-size: 10px; color: #94a3b8;", "admin@system.io" }
                        }
                    }
                }
            }
        }

        "Topbar" => {
            let title = node
                .props
                .title
                .clone()
                .unwrap_or_else(|| "Management Dashboard".to_string());
            let base_style = "height: 64px; width: 100%; background-color: #ffffff; border-bottom: 1px solid rgba(0,0,0,0.06); display: flex; align-items: center; justify-content: space-between; padding: 0 24px; font-family: 'Outfit', sans-serif; box-sizing: border-box;";
            let topbar_style = format!("{} {}", base_style, styles);
            rsx! {
                div {
                    id: "{node.id}",
                    style: "{topbar_style}",
                    div {
                        style: "display: flex; align-items: center; gap: 16px;",
                        span { style: "font-weight: 700; color: #0f172a; font-size: 16px;", "{title}" }
                    }
                    div {
                        style: "display: flex; align-items: center; gap: 16px;",
                        for child in node.children {
                            ComponentRenderer {
                            node: child,
                            selected_id,
                            on_select,
                            on_drop,
                            on_delete,
                            on_resize_start,
                            on_drag_start
                        }
                    }
                    }
                }
            }
        }

        "Aside" => {
            let base_style = "width: 300px; background-color: #f8fafc; border-left: 1px solid #e2e8f0; padding: 20px; display: flex; flex-direction: column; gap: 16px; font-family: 'Outfit', sans-serif; box-sizing: border-box;";
            let aside_style = format!("{} {}", base_style, styles);
            rsx! {
                div {
                    id: "{node.id}",
                    style: "{aside_style}",
                    for child in node.children {
                        ComponentRenderer {
                            node: child,
                            selected_id,
                            on_select,
                            on_drop,
                            on_delete,
                            on_resize_start,
                            on_drag_start
                        }
                    }
                }
            }
        }

        "Bar" => {
            let theme = node
                .props
                .extra
                .get("theme")
                .and_then(|v| v.as_str())
                .unwrap_or("light")
                .to_string();

            let nav_items = node
                .props
                .extra
                .get("items")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();

            let title = node
                .props
                .title
                .clone()
                .or_else(|| {
                    node.props
                        .extra
                        .get("title")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                })
                .unwrap_or_else(|| "Navigation".to_string());

            let logo = node
                .props
                .extra
                .get("logo")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let logo_position = node
                .props
                .extra
                .get("logoPosition")
                .and_then(|v| v.as_str())
                .unwrap_or("inline")
                .to_string();

            let logo_size = node
                .props
                .extra
                .get("logoSize")
                .and_then(|v| v.as_u64())
                .map(|v| v as usize)
                .unwrap_or_else(|| if logo_position == "top" { 48 } else { 20 });

            let position = node
                .props
                .extra
                .get("position")
                .and_then(|v| v.as_str())
                .unwrap_or("top")
                .to_lowercase();
            let position = if position == "left" || position == "right" {
                position
            } else {
                "top".to_string()
            };
            let is_top = position == "top";

            let width = node
                .props
                .extra
                .get("width")
                .and_then(|v| v.as_str().map(|s| s.to_string()))
                .or_else(|| {
                    node.props
                        .style
                        .as_ref()
                        .and_then(|style| style.get("width").cloned())
                })
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| "250px".to_string());

            let height = node
                .props
                .style
                .as_ref()
                .and_then(|style| style.get("height"))
                .map(|s| s.to_string())
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| "64px".to_string());

            let bg = if theme.eq_ignore_ascii_case("dark") {
                "#0a0f2e"
            } else {
                "#ffffff"
            };
            let border_color = if theme.eq_ignore_ascii_case("dark") {
                "#2a3565"
            } else {
                "#e5e7eb"
            };
            let text_color = if theme.eq_ignore_ascii_case("dark") {
                "#e2e8f0"
            } else {
                "#0f172a"
            };

            let visible_items: Vec<serde_json::Value> = nav_items
                .into_iter()
                .filter(|item| is_item_visible(item, &data_state.read(), local_item.as_ref()))
                .collect();

            let logo_min_width = if is_top { "12px" } else { "0" };
            let logo_min_height = if is_top { "0" } else { "12px" };
            let logo_display_size = if is_top { 24 } else { logo_size };
            let title_font_size = if is_top { "1.25rem" } else { "1rem" };
            let items_flex_direction = if is_top { "row" } else { "column" };
            let items_align_items = if is_top { "center" } else { "stretch" };
            let items_gap = if is_top { "8px" } else { "6px" };
            let dropdown_width = if is_top { "auto" } else { "100%" };
            let menu_position = if is_top { "absolute" } else { "relative" };
            let menu_top = if is_top { "calc(100% + 6px)" } else { "auto" };
            let menu_left = if is_top { "0" } else { "auto" };
            let menu_width = if is_top { "max-content" } else { "100%" };
            let menu_box_shadow = if is_top {
                "0 12px 24px rgba(15,23,42,0.12)"
            } else {
                "none"
            };

            let nav_style = format!(
                "display: flex; flex-direction: {}; align-items: {}; justify-content: flex-start; gap: {}; padding: {}; height: {}; width: {}; min-width: {}; max-width: {}; background-color: {}; color: {}; border-bottom: {}; border-right: {}; border-left: {}; box-sizing: border-box; box-shadow: {}; overflow: visible; {}",
                if is_top { "row" } else { "column" },
                if is_top { "center" } else { "stretch" },
                if is_top { "16px" } else { "10px" },
                if is_top { "0 24px" } else { "14px" },
                if is_top { height.as_str() } else { "100%" },
                if is_top { "100%" } else { width.as_str() },
                if is_top { "0" } else { width.as_str() },
                if is_top { "none" } else { width.as_str() },
                bg,
                text_color,
                if is_top { format!("1px solid {}", border_color) } else { "none".to_string() },
                if position == "left" { format!("1px solid {}", border_color) } else { "none".to_string() },
                if position == "right" { format!("1px solid {}", border_color) } else { "none".to_string() },
                if is_top { "0 1px 3px 0 rgba(0,0,0,0.05)" } else { "none" },
                styles
            );

            rsx! {
                nav {
                    id: "{node.id}",
                    style: "{nav_style}",

                    if !logo.is_empty() && !is_top && logo_position == "top" {
                        div {
                            style: "display: flex; justify-content: center; align-items: center; width: 100%; margin-bottom: 10px; padding: 4px 0;",
                            if is_image_source(&logo) {
                                img {
                                    src: "{normalize_media_source(&logo)}",
                                    alt: "logo",
                                    style: "max-height: {logo_size}px; width: auto; display: block;"
                                }
                            } else {
                                span {
                                    style: "font-size: {logo_size}px; color: #3b82f6; display: inline-flex; align-items: center; justify-content: center;",
                                    "{icon_label_to_glyph(&logo)}"
                                }
                            }
                        }
                    }

                    if !title.is_empty() || !logo.is_empty() {
                        div {
                            style: "display: flex; align-items: center; justify-content: flex-start; gap: 10px; min-width: 0; flex-shrink: 0;",
                            div {
                                style: "display: flex; align-items: center; justify-content: flex-start; gap: 10px; min-width: 0; color: {text_color}; font-weight: 800; font-size: {title_font_size};",
                                if !logo.is_empty() && (is_top || logo_position != "top") {
                                    if is_image_source(&logo) {
                                        img {
                                            src: "{normalize_media_source(&logo)}",
                                            alt: "logo",
                                            style: "max-height: {logo_display_size}px; width: auto; display: block;"
                                        }
                                    } else {
                                        span {
                                            style: "font-size: {logo_display_size}px; color: #3b82f6; display: inline-flex; align-items: center; justify-content: center;",
                                            "{icon_label_to_glyph(&logo)}"
                                        }
                                    }
                                }
                                if !title.is_empty() {
                                    span {
                                        style: "overflow: hidden; text-overflow: ellipsis;",
                                        "{title}"
                                    }
                                }
                            }
                        }
                    }

                    div {
                        style: "display: flex; flex: 1; flex-direction: {items_flex_direction}; align-items: {items_align_items}; gap: {items_gap}; min-width: 0;",
                        for (index, item) in visible_items.iter().enumerate() {
                            {
                                let item_type = item.get("type").and_then(|v| v.as_str()).unwrap_or("nav").to_lowercase();
                                match item_type.as_str() {
                                    "spacer" => rsx! {
                                        span {
                                            key: "spacer-{index}",
                                            style: "flex: 1; min-width: {logo_min_width}; min-height: {logo_min_height};"
                                        }
                                    },
                                    "avatar" => {
                                        let name_raw = item.get("name").and_then(|v| v.as_str()).unwrap_or("A");
                                        let name = resolve_string_templates(name_raw, local_item.as_ref(), &data_state.read());
                                        let src_raw = item.get("src").and_then(|v| v.as_str()).unwrap_or("");
                                        let src = resolve_string_templates(src_raw, local_item.as_ref(), &data_state.read());
                                        let size = item.get("size").and_then(|v| v.as_str()).unwrap_or("36");
                                        let initial = name.chars().next().unwrap_or('A').to_string();

                                        rsx! {
                                            span {
                                                key: "avatar-{index}",
                                                style: "display: inline-flex; align-items: center; justify-content: center;",
                                                span {
                                                    style: "width: {size}px; height: {size}px; border-radius: 999px; overflow: hidden; background: #cbd5e1; color: #0f172a; display: inline-flex; align-items: center; justify-content: center; font-weight: 700;",
                                                    if !src.is_empty() {
                                                        img {
                                                            src: "{src}",
                                                            alt: "{name}",
                                                            style: "width: 100%; height: 100%; object-fit: cover;"
                                                        }
                                                    } else {
                                                        span { "{initial}" }
                                                    }
                                                }
                                            }
                                        }
                                    },
                                    "dropdown" => {
                                        let label_raw = item.get("label").and_then(|v| v.as_str()).unwrap_or("Menu");
                                        let label = resolve_string_templates(label_raw, local_item.as_ref(), &data_state.read());
                                        let icon = item.get("icon").and_then(|v| v.as_str()).unwrap_or("ChevronDown").to_string();
                                        let menu_items = item.get("items").or_else(|| item.get("menuItems")).and_then(|v| v.as_array()).cloned().unwrap_or_default();

                                        rsx! {
                                            details {
                                                key: "dropdown-{index}",
                                                style: "position: relative; width: {dropdown_width};",
                                                summary {
                                                    style: "list-style: none; user-select: none; outline: none; cursor: pointer; {bar_button_style(item, is_top, &theme)}",
                                                    if !icon.is_empty() {
                                                        span {
                                                            style: "display: inline-flex; align-items: center; justify-content: center; margin-right: 8px;",
                                                            "{icon_label_to_glyph(&icon)}"
                                                        }
                                                    }
                                                    span { "{label}" }
                                                }
                                                div {
                                                    role: "menu",
                                                    style: "position: {menu_position}; top: {menu_top}; left: {menu_left}; z-index: 20; min-width: 180px; width: {menu_width}; padding: 6px; display: flex; flex-direction: column; gap: 4px; border-radius: 8px; border: 1px solid {border_color}; background: {bg}; box-shadow: {menu_box_shadow}; box-sizing: border-box;",
                                                    for (sub_index, menu_item) in menu_items.iter().enumerate() {
                                                        {
                                                            let label_raw = menu_item.get("label").and_then(|v| v.as_str()).unwrap_or("Item");
                                                            let label = resolve_string_templates(label_raw, local_item.as_ref(), &data_state.read());
                                                            let icon = menu_item.get("icon").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                                            let actions = nav_item_actions(menu_item);
                                                            let local_item_click = local_item.clone();

                                                            rsx! {
                                                                button {
                                                                    key: "dropdown-item-{sub_index}",
                                                                    r#type: "button",
                                                                    style: "{bar_button_style(menu_item, is_top, &theme)}",
                                                                    onclick: move |_| {
                                                                        let acts = actions.clone();
                                                                        let ctx = local_item_click.clone();
                                                                        spawn(async move {
                                                                            execute_actions(acts, data_state, ctx).await;
                                                                        });
                                                                    },
                                                                    if !icon.is_empty() {
                                                                        span {
                                                                            style: "display: inline-flex; align-items: center; justify-content: center; margin-right: 8px;",
                                                                            "{icon_label_to_glyph(&icon)}"
                                                                        }
                                                                    }
                                                                    span { "{label}" }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    },
                                    _ => {
                                        let label_raw = item.get("label").and_then(|v| v.as_str()).unwrap_or("");
                                        let label_fallback = match item.get("type").and_then(|v| v.as_str()).unwrap_or("nav").to_lowercase().as_str() {
                                            "login" => "Login",
                                            "signup" => "Sign up",
                                            "profile" => "Profile",
                                            "logout" => "Logout",
                                            "button" => "Button",
                                            _ => "Item",
                                        };
                                        let label_str = if label_raw.is_empty() { label_fallback } else { label_raw };
                                        let label = resolve_string_templates(label_str, local_item.as_ref(), &data_state.read());

                                        let icon_raw = item.get("icon").and_then(|v| v.as_str()).unwrap_or("");
                                        let icon_fallback = match item.get("type").and_then(|v| v.as_str()).unwrap_or("nav").to_lowercase().as_str() {
                                            "login" => "LogIn",
                                            "signup" => "UserPlus",
                                            "profile" => "User",
                                            "logout" => "LogOut",
                                            _ => "",
                                        };
                                        let icon = if icon_raw.is_empty() { icon_fallback } else { icon_raw };

                                        let content_mode = item.get("contentMode").and_then(|v| v.as_str()).unwrap_or("both").to_lowercase();
                                        let show_icon = !icon.is_empty() && content_mode != "text";
                                        let show_text = content_mode != "icon";
                                        let actions = nav_item_actions(&item);
                                        let local_item_click = local_item.clone();

                                        rsx! {
                                            button {
                                                key: "item-{index}",
                                                r#type: "button",
                                                style: "{bar_button_style(item, is_top, &theme)}",
                                                onclick: move |_| {
                                                    let acts = actions.clone();
                                                    let ctx = local_item_click.clone();
                                                    spawn(async move {
                                                        execute_actions(acts, data_state, ctx).await;
                                                    });
                                                },
                                                if show_icon {
                                                    span {
                                                        style: "display: inline-flex; align-items: center; justify-content: center; margin-right: 8px;",
                                                        "{icon_label_to_glyph(&icon)}"
                                                    }
                                                }
                                                if show_text {
                                                    span { "{label}" }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        "FilterBar" => {
            let base_style = "display: flex; flex-wrap: wrap; align-items: center; justify-content: space-between; gap: 12px; background-color: #f8fafc; padding: 12px 18px; border-radius: var(--radius, 10px); border: 1px solid #e2e8f0; width: 100%; box-sizing: border-box;";
            let filter_style = format!("{} {}", base_style, styles);
            rsx! {
                div {
                    id: "{node.id}",
                    style: "{filter_style}",
                    for child in node.children {
                        ComponentRenderer {
                            node: child,
                            selected_id,
                            on_select,
                            on_drop,
                            on_delete,
                            on_resize_start,
                            on_drag_start
                        }
                    }
                }
            }
        }

        "Link" => {
            let text = node
                .props
                .text
                .clone()
                .or_else(|| node.props.content.clone())
                .unwrap_or_else(|| "Link Target".to_string());
            let href = node
                .props
                .extra
                .get("href")
                .and_then(|v| v.as_str())
                .unwrap_or("#");
            let base_style = "color: #3b82f6; font-weight: 600; text-decoration: none; cursor: pointer; transition: color 0.15s; font-family: 'Outfit', sans-serif;";
            let link_style = format!("{} {}", base_style, styles);
            rsx! {
                a {
                    id: "{node.id}",
                    href: "{href}",
                    style: "{link_style}",
                    "{text}"
                }
            }
        }

        "Badge" => {
            let label = node
                .props
                .label
                .clone()
                .unwrap_or_else(|| "New".to_string());
            let variant = node
                .props
                .variant
                .clone()
                .unwrap_or_else(|| "info".to_string());
            let (bg, text_color) = match variant.as_str() {
                "success" => ("#dcfce7", "#166534"),
                "warning" => ("#fef3c7", "#92400e"),
                "error" => ("#fee2e2", "#991b1b"),
                _ => ("#dbeafe", "#1e40af"),
            };
            let base_style = format!("background-color: {}; color: {}; padding: 2px 8px; border-radius: 9999px; font-size: 11px; font-weight: 600; display: inline-flex; align-items: center; justify-content: center; font-family: 'Outfit', sans-serif;", bg, text_color);
            let badge_style = format!("{} {}", base_style, styles);
            rsx! {
                span {
                    id: "{node.id}",
                    style: "{badge_style}",
                    "{label}"
                }
            }
        }

        "Avatar" => {
            let src = node.props.extra.get("src").and_then(|v| v.as_str());
            let alt = node
                .props
                .extra
                .get("alt")
                .and_then(|v| v.as_str())
                .unwrap_or("user avatar");
            let fallback = node
                .props
                .extra
                .get("fallback")
                .and_then(|v| v.as_str())
                .unwrap_or("U");
            let base_style = "width: 40px; height: 40px; border-radius: 50%; display: flex; align-items: center; justify-content: center; background-color: #e2e8f0; color: #475569; font-weight: 700; font-family: 'Outfit', sans-serif; overflow: hidden; border: 1px solid rgba(0,0,0,0.05); box-sizing: border-box;";
            let avatar_style = format!("{} {}", base_style, styles);
            rsx! {
                div {
                    id: "{node.id}",
                    style: "{avatar_style}",
                    if let Some(url) = src {
                        img {
                            src: "{url}",
                            alt: "{alt}",
                            style: "width: 100%; height: 100%; object-fit: cover;"
                        }
                    } else {
                        span { "{fallback}" }
                    }
                }
            }
        }

        "Author" => {
            let name = node
                .props
                .title
                .clone()
                .unwrap_or_else(|| "Author Profile".to_string());
            let subtitle = node
                .props
                .extra
                .get("role")
                .and_then(|v| v.as_str())
                .unwrap_or("Lead Engineer");
            let bio = node
                .props
                .content
                .clone()
                .unwrap_or_else(|| "Contributing designer & system architect.".to_string());
            let avatar_url = node
                .props
                .extra
                .get("avatarUrl")
                .and_then(|v| v.as_str())
                .unwrap_or("https://picsum.photos/id/64/100");

            let base_style = "background-color: #ffffff; border-radius: 12px; padding: 16px; border: 1px solid rgba(0,0,0,0.08); font-family: 'Outfit', sans-serif; display: flex; gap: 14px; align-items: flex-start; max-width: 320px; box-shadow: 0 4px 10px rgba(0,0,0,0.02);";
            let author_style = format!("{} {}", base_style, styles);
            rsx! {
                div {
                    id: "{node.id}",
                    style: "{author_style}",
                    img {
                        src: "{avatar_url}",
                        style: "width: 48px; height: 48px; border-radius: 50%; object-fit: cover; border: 1px solid rgba(0,0,0,0.05);"
                    }
                    div {
                        style: "display: flex; flex-direction: column; gap: 2px;",
                        span { style: "font-weight: 700; color: #0f172a; font-size: 14px;", "{name}" }
                        span { style: "font-size: 11px; color: #3b82f6; font-weight: 600; text-transform: uppercase; letter-spacing: 0.04em;", "{subtitle}" }
                        p { style: "font-size: 12px; color: #64748b; line-height: 1.5; margin: 4px 0 0 0;", "{bio}" }
                    }
                }
            }
        }

        "Header" => {
            let title = node
                .props
                .title
                .clone()
                .or_else(|| node.props.text.clone())
                .unwrap_or_else(|| "Header Section".to_string());
            let description = node.props.content.clone();
            let base_style = "width: 100%; border-bottom: 1px solid rgba(0,0,0,0.08); padding-bottom: 12px; margin-bottom: 16px; font-family: 'Outfit', sans-serif;";
            let header_style = format!("{} {}", base_style, styles);
            rsx! {
                div {
                    id: "{node.id}",
                    style: "{header_style}",
                    h3 { style: "font-size: 18px; font-weight: 700; color: #0f172a; margin: 0 0 4px 0;", "{title}" }
                    if let Some(desc) = description {
                        p { style: "font-size: 13px; color: #64748b; margin: 0;", "{desc}" }
                    }
                }
            }
        }

        "Switch" => rsx! {
        Hooked_Switch {
            node: node.clone(),
            styles: styles.clone(),
            selected_id,
            on_select,
            on_drop,
            on_delete
                }
            },

        "Iframe" => {
            let src = node
                .props
                .extra
                .get("src")
                .and_then(|v| v.as_str())
                .unwrap_or("https://dioxuslabs.com");
            let base_style = "border-radius: var(--radius, 12px); border: 1px solid rgba(0,0,0,0.08); overflow: hidden; width: 100%; min-height: 300px; box-shadow: 0 4px 10px rgba(0,0,0,0.02);";
            let iframe_style = format!("{} {}", base_style, styles);
            rsx! {
                iframe {
                    id: "{node.id}",
                    src: "{src}",
                    style: "{iframe_style}",
                }
            }
        }

        "YearCalendar" => {
            let base_style = "background-color: #ffffff; border-radius: 12px; padding: 20px; border: 1px solid rgba(0,0,0,0.08); font-family: 'Outfit', sans-serif; display: flex; flex-direction: column; gap: 12px; width: 100%; max-width: 680px; box-shadow: 0 4px 12px rgba(0,0,0,0.02); overflow-x: auto; box-sizing: border-box;";
            let cal_style = format!("{} {}", base_style, styles);
            rsx! {
                div {
                    id: "{node.id}",
                    style: "{cal_style}",
                    span { style: "font-weight: 700; color: #475569; font-size: 13px; text-transform: uppercase;", "Contribution Activity Feed" }
                    svg {
                        width: "600",
                        height: "100",
                        view_box: "0 0 600 100",
                        style: "overflow: visible;",

                        g {
                            for col in 0..48 {
                                g {
                                    key: "{col}",
                                    transform: "translate({col * 12}, 0)",
                                    for row in 0..7 {
                                        {
                                            let cell_id = col * 7 + row;
                                            let fill_color = match cell_id % 7 {
                                                1 | 3 => "#86efac",
                                                2 => "#22c55e",
                                                5 => "#15803d",
                                                _ => "#ebedf0",
                                            };
                                            rsx! {
                                                rect {
                                                    key: "{row}",
                                                    x: "0",
                                                    y: "{row * 12}",
                                                    width: "10",
                                                    height: "10",
                                                    fill: "{fill_color}",
                                                    rx: "2",
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        "SearchFilters" => {
            let base_style = "display: flex; gap: 10px; align-items: center; width: 100%; box-sizing: border-box;";
            let sf_style = format!("{} {}", base_style, styles);
            rsx! {
                div {
                    id: "{node.id}",
                    style: "{sf_style}",
                    div {
                        style: "position: relative; flex-grow: 1;",
                        span { style: "position: absolute; left: 12px; top: 12px; color: #94a3b8; font-size: 14px;", "🔍" }
                        input {
                            r#type: "text",
                            placeholder: "Search active filters...",
                            style: "width: 100%; padding: 10px 12px 10px 36px; border-radius: 8px; border: 1px solid rgba(0,0,0,0.08); background-color: #ffffff; color: #1e293b; outline: none; font-size: 14px; box-sizing: border-box;",
                        }
                    }
                    button {
                        style: "padding: 10px 14px; background-color: #f1f5f9; color: #475569; border: 1px solid #cbd5e1; border-radius: 8px; font-weight: 600; cursor: pointer; font-size: 14px; display: inline-flex; align-items: center; gap: 6px;",
                        "⚙️ Filters"
                    }
                }
            }
        }

        // Unknown components fallback
        unknown => rsx! {
            div {
                id: "{node.id}",
                style: "{styles} padding: 12px; border: 1px dashed #ef4444; border-radius: var(--radius, 8px); background-color: #fef2f2; color: #b91c1c; font-family: 'Outfit', sans-serif; font-size: 13px;",
                b { "Unknown Component: {unknown}" }
                span { style: "display: block; font-size: 11px; color: #7f1d1d; margin-top: 4px; font-family: monospace;", "ID: {node.id}" }
            }
        },
    };

    let is_editor =
        selected_id.is_some() && on_select.is_some() && on_drop.is_some() && on_delete.is_some();

    if is_editor {
        let sel_sig = selected_id.unwrap();
        let select_handler = on_select.unwrap();
        let drop_handler = on_drop.unwrap();
        let delete_handler = on_delete.unwrap();

        let is_sel = sel_sig.read().as_ref() == Some(&node.id);
        let border_outline = if is_sel {
            "2px solid #3b82f6"
        } else {
            "1px dashed rgba(59, 130, 246, 0.15)"
        };
        let is_drop_target = is_container(&node.component_type);

        let select_id = node.id.clone();
        let select_id_for_mousedown = select_id.clone();
        let select_id_for_click = select_id.clone();
        let select_id_for_drag = select_id.clone();
        let drop_id = node.id.clone();
        let delete_id = node.id.clone();
        let is_not_root = node.id != "root";

        rsx! {
            div {
                style: "{wrapper_styles}",
                draggable: "false",
                onmousedown: move |e| {
                    if is_not_root {
                        e.prevent_default();
                        e.stop_propagation();
                        select_handler.call(select_id_for_mousedown.clone());
                        if let Some(ref handler) = on_drag_start {
                            let coords = e.client_coordinates();
                            handler.call((select_id_for_drag.clone(), coords.x, coords.y));
                        }
                    }
                },
                onclick: move |e| {
                    e.stop_propagation();
                    select_handler.call(select_id_for_click.clone());
                },
                ondragover: move |e| {
                    if is_drop_target {
                        e.prevent_default();
                    }
                },
                ondrop: move |e| {
                    if is_drop_target {
                        e.stop_propagation();
                        let coords = e.client_coordinates();
                        drop_handler.call((drop_id.clone(), coords.x, coords.y));
                    }
                },

                if is_not_root {
                    button {
                        style: "position: absolute; top: -10px; right: -10px; background-color: #ef4444; color: #ffffff; border: none; border-radius: 50%; width: 22px; height: 22px; cursor: pointer; display: flex; align-items: center; justify-content: center; font-size: 12px; font-weight: bold; z-index: 1000; box-shadow: 0 2px 4px rgba(0,0,0,0.15);",
                        onclick: move |e| {
                            e.stop_propagation();
                            delete_handler.call(delete_id.clone());
                        },
                        "×"
                    }
                }

                {element_content}
            }
        }
    } else {
        element_content
    }
}

#[component]
fn Hooked_DrawerPanel(
    node: ComponentNode,
    styles: String,
    selected_id: Option<Signal<Option<String>>>,
    on_select: Option<EventHandler<String>>,
    on_drop: Option<EventHandler<(String, f64, f64)>>,
    on_delete: Option<EventHandler<String>>,
    #[props(default)] on_resize_start: Option<EventHandler<(String, String, f64, f64)>>,
    #[props(default)] on_drag_start: Option<EventHandler<(String, f64, f64)>>,
) -> Element {
    let title = node
        .props
        .title
        .clone()
        .unwrap_or_else(|| "Menu Panel".to_string());
    let mut open = use_signal(|| false);

    let base_style = "display: flex; flex-direction: column; font-family: 'Outfit', sans-serif;";
    let drawer_style = format!("{} {}", base_style, styles);

    let drawer_position_style = if open() {
        "position: fixed; top: 0; right: 0; width: 320px; height: 100vh; background-color: #ffffff; box-shadow: -10px 0 30px rgba(0,0,0,0.1); transform: translateX(0); transition: transform 0.3s ease-out; z-index: 1000; display: flex; flex-direction: column; border-left: 1px solid #e2e8f0;"
    } else {
        "position: fixed; top: 0; right: -340px; width: 320px; height: 100vh; background-color: #ffffff; transform: translateX(100%); transition: transform 0.3s ease-in; z-index: 1000; display: flex; flex-direction: column;"
    };

    rsx! {
        div {
            id: "{node.id}",
            style: "{drawer_style}",
            button {
                style: "padding: 8px 16px; background-color: #0f172a; color: #ffffff; border-radius: 8px; border: none; font-weight: 600; cursor: pointer; display: inline-flex; align-items: center; gap: 8px;",
                onclick: move |_| *open.write() = !open(),
                "📂 {title}"
            }

            // Sliding Drawer Content
            div {
                style: "{drawer_position_style}",
                div {
                    style: "padding: 20px; border-bottom: 1px solid #f1f5f9; display: flex; justify-content: space-between; align-items: center; background-color: #f8fafc;",
                    span { style: "font-weight: 700; color: #0f172a; font-size: 16px;", "{title}" }
                    button {
                        style: "background: none; border: none; font-size: 20px; cursor: pointer; color: #94a3b8;",
                        onclick: move |_| *open.write() = false,
                        "×"
                    }
                }
                div {
                    style: "padding: 20px; flex-grow: 1; overflow-y: auto; display: flex; flex-direction: column; gap: 12px;",
                    for child in node.children {
                        ComponentRenderer {
                            node: child,
                            selected_id,
                            on_select,
                            on_drop,
                            on_delete,
                            on_resize_start,
                            on_drag_start
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn Hooked_Modal(
    node: ComponentNode,
    styles: String,
    selected_id: Option<Signal<Option<String>>>,
    on_select: Option<EventHandler<String>>,
    on_drop: Option<EventHandler<(String, f64, f64)>>,
    on_delete: Option<EventHandler<String>>,
    #[props(default)] on_resize_start: Option<EventHandler<(String, String, f64, f64)>>,
    #[props(default)] on_drag_start: Option<EventHandler<(String, f64, f64)>>,
) -> Element {
    let title = node
        .props
        .title
        .clone()
        .unwrap_or_else(|| "Dialog Window".to_string());
    let mut open = use_signal(|| false);
    let data_state = use_context::<GlobalDataState>().0;
    let local_item_sig = use_context::<RepeaterItemState>().0;
    let local_item = local_item_sig.read().clone();
    let on_open_actions = modal_on_open_actions(&node);

    let modal_sizing = modal_sizing_styles(&node);
    let dialog_style = modal_sizing.dialog_style;
    let body_style = modal_sizing.body_style;

    let modal_style = "position: absolute; width: 0; height: 0; overflow: visible; z-index: 9999; margin: 0; padding: 0; border: none;";

    // Register event listeners on the DOM element for open-modal / close-modal
    let modal_id = node.id.clone();
    #[cfg(not(target_arch = "wasm32"))]
    let _ = (&modal_id, &data_state, &on_open_actions, &local_item);

    use_effect(move || {
        #[cfg(target_arch = "wasm32")]
        {
            use wasm_bindgen::prelude::Closure;
            use wasm_bindgen::JsCast;

            let actions_for_open = on_open_actions.clone();
            let item_for_open = local_item.clone();
            let window = match web_sys::window() {
                Some(w) => w,
                None => return,
            };
            let document = match window.document() {
                Some(d) => d,
                None => return,
            };
            let el = match document.get_element_by_id(&modal_id) {
                Some(e) => e,
                None => return,
            };

            let mut open_signal = open.clone();
            let on_open = Closure::wrap(Box::new(move || {
                *open_signal.write() = true;
                if !actions_for_open.is_empty() {
                    let actions = actions_for_open.clone();
                    let item = item_for_open.clone();
                    spawn(async move {
                        execute_actions(actions, data_state, item).await;
                    });
                }
            }) as Box<dyn FnMut()>);

            let mut close_signal = open.clone();
            let on_close = Closure::wrap(Box::new(move || {
                *close_signal.write() = false;
            }) as Box<dyn FnMut()>);

            let _ =
                el.add_event_listener_with_callback("open-modal", on_open.as_ref().unchecked_ref());
            let _ = el
                .add_event_listener_with_callback("close-modal", on_close.as_ref().unchecked_ref());

            on_open.forget();
            on_close.forget();
        }
    });

    rsx! {
        div {
            id: "{node.id}",
            style: "{modal_style}",
            "data-modal-open": "{open()}",

            if open() {
                div {
                    style: "position: fixed; top: 0; left: 0; width: 100vw; height: 100vh; padding: 12px; box-sizing: border-box; background-color: rgba(15, 23, 42, 0.4); backdrop-filter: blur(4px); display: flex; align-items: center; justify-content: center; z-index: 2000; animation: fadeIn 0.2s ease-out;",
                    div {
                        style: "{dialog_style}",

                        // Modal Header
                        div {
                            style: "padding: 16px 20px; border-bottom: 1px solid #f1f5f9; display: flex; justify-content: space-between; align-items: center; background-color: #f8fafc;",
                            span { style: "font-weight: 700; color: #0f172a; font-size: 16px;", "{title}" }
                            button {
                                style: "background: none; border: none; font-size: 18px; cursor: pointer; color: #94a3b8;",
                                onclick: move |_| *open.write() = false,
                                "×"
                            }
                        }

                        // Modal Body
                        div {
                            style: "{body_style}",
                            for child in node.children {
                                ComponentRenderer {
                                    node: child,
                                    selected_id,
                                    on_select,
                                    on_drop,
                                    on_delete,
                                    on_resize_start,
                                    on_drag_start
                                }
                            }
                        }

                        // Modal Footer
                        div {
                            style: "padding: 14px 20px; border-top: 1px solid #f1f5f9; background-color: #f8fafc; display: flex; justify-content: flex-end; gap: 8px;",
                            button {
                                style: "padding: 8px 14px; background-color: #f1f5f9; color: #334155; border-radius: 6px; border: 1px solid #e2e8f0; font-weight: 600; cursor: pointer;",
                                onclick: move |_| *open.write() = false,
                                "Close"
                            }
                            button {
                                style: "padding: 8px 14px; background-color: #3b82f6; color: #ffffff; border-radius: 6px; border: none; font-weight: 600; cursor: pointer;",
                                onclick: move |_| *open.write() = false,
                                "Confirm"
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn Hooked_Checkbox(
    node: ComponentNode,
    styles: String,
    selected_id: Option<Signal<Option<String>>>,
    on_select: Option<EventHandler<String>>,
    on_drop: Option<EventHandler<(String, f64, f64)>>,
    on_delete: Option<EventHandler<String>>,
) -> Element {
    let label = node
        .props
        .label
        .clone()
        .unwrap_or_else(|| "Accept Terms".to_string());
    let mut data_state = use_context::<GlobalDataState>().0;
    let bound_bool = node
        .props
        .extra
        .get("__boundValue")
        .and_then(json_value_to_bool);
    let mut checked = use_signal(move || bound_bool.unwrap_or(false));
    let is_checked = bound_bool.unwrap_or_else(|| checked());
    let bind_node = node.clone();

    let base_style = "display: flex; align-items: center; gap: 8px; font-family: 'Outfit', sans-serif; font-size: 14px; cursor: pointer; user-select: none;";
    let check_style = format!("{} {}", base_style, styles);

    rsx! {
        label {
            id: "{node.id}",
            style: "{check_style}",
            input {
                r#type: "checkbox",
                checked: is_checked,
                style: "width: 16px; height: 16px; cursor: pointer;",
                onchange: move |_| {
                    let next = !is_checked;
                    *checked.write() = next;
                    let mut data = data_state.write();
                    set_node_bind_value(&bind_node, &mut data, serde_json::Value::Bool(next));
                },
            }
            span { style: "color: #334155; font-weight: 500;", "{label}" }
        }
    }
}

#[component]
fn Hooked_Toggle(
    node: ComponentNode,
    styles: String,
    selected_id: Option<Signal<Option<String>>>,
    on_select: Option<EventHandler<String>>,
    on_drop: Option<EventHandler<(String, f64, f64)>>,
    on_delete: Option<EventHandler<String>>,
) -> Element {
    let label = node
        .props
        .label
        .clone()
        .unwrap_or_else(|| "Status Enabled".to_string());
    let mut data_state = use_context::<GlobalDataState>().0;
    let bound_bool = node
        .props
        .extra
        .get("__boundValue")
        .and_then(json_value_to_bool);
    let mut active = use_signal(move || bound_bool.unwrap_or(false));
    let is_active = bound_bool.unwrap_or_else(|| active());
    let bind_node = node.clone();

    let base_style = "display: flex; align-items: center; gap: 10px; font-family: 'Outfit', sans-serif; font-size: 14px; cursor: pointer; user-select: none;";
    let toggle_style = format!("{} {}", base_style, styles);

    let switch_bg = if is_active { "#3b82f6" } else { "#cbd5e1" };
    let knob_transform = if is_active {
        "translateX(18px)"
    } else {
        "translateX(0px)"
    };

    rsx! {
        div {
            id: "{node.id}",
            style: "{toggle_style}",
            onclick: move |_| {
                let next = !is_active;
                *active.write() = next;
                let mut data = data_state.write();
                set_node_bind_value(&bind_node, &mut data, serde_json::Value::Bool(next));
            },
            div {
                style: "position: relative; width: 38px; height: 20px; background-color: {switch_bg}; border-radius: 9999px; transition: background-color 0.2s;",
                div {
                    style: "position: absolute; top: 2px; left: 2px; width: 16px; height: 16px; background-color: #ffffff; border-radius: 50%; transition: transform 0.2s; transform: {knob_transform}; box-shadow: 0 1px 3px rgba(0,0,0,0.1);",
                }
            }
            span { style: "color: #334155; font-weight: 500;", "{label}" }
        }
    }
}

#[component]
fn Hooked_SignaturePad(
    node: ComponentNode,
    styles: String,
    selected_id: Option<Signal<Option<String>>>,
    on_select: Option<EventHandler<String>>,
    on_drop: Option<EventHandler<(String, f64, f64)>>,
    on_delete: Option<EventHandler<String>>,
) -> Element {
    let label = node
        .props
        .label
        .clone()
        .unwrap_or_else(|| "Signature Required".to_string());
    let mut signed = use_signal(|| false);

    let base_style = "display: flex; flex-direction: column; gap: 6px; font-family: 'Outfit', sans-serif; width: 320px;";
    let sig_style = format!("{} {}", base_style, styles);

    let draw_color = if signed() { "#0f172a" } else { "#cbd5e1" };

    rsx! {
        div {
            id: "{node.id}",
            style: "{sig_style}",
            span { style: "font-weight: 600; color: #475569; font-size: 14px;", "{label}" }
            div {
                style: "height: 100px; border-radius: 8px; border: 1px dashed #cbd5e1; background-color: #f8fafc; position: relative; display: flex; align-items: center; justify-content: center; cursor: crosshair;",
                onclick: move |_| *signed.write() = true,

                if signed() {
                    svg {
                        width: "240",
                        height: "60",
                        view_box: "0 0 240 60",
                        path {
                            d: "M 10 30 Q 30 10, 60 40 T 120 20 T 180 35 T 230 10",
                            fill: "none",
                            stroke: "{draw_color}",
                            stroke_width: "2.5",
                        }
                    }
                } else {
                    span { style: "font-size: 12px; color: #94a3b8;", "Click here to draw signature" }
                }
            }
            div {
                style: "display: flex; justify-content: space-between; font-size: 12px;",
                button {
                    r#type: "button",
                    style: "background: none; border: none; color: #ef4444; font-weight: 600; cursor: pointer;",
                    onclick: move |_| *signed.write() = false,
                    "Clear"
                }
                span { style: "color: #94a3b8;", "Secure Verification" }
            }
        }
    }
}

#[component]
fn Hooked_Select(
    node: ComponentNode,
    styles: String,
    selected_id: Option<Signal<Option<String>>>,
    on_select: Option<EventHandler<String>>,
    on_drop: Option<EventHandler<(String, f64, f64)>>,
    on_delete: Option<EventHandler<String>>,
) -> Element {
    let label = node
        .props
        .label
        .clone()
        .unwrap_or_else(|| "Select Option".to_string());
    let placeholder = node.props.placeholder.clone().unwrap_or_default();
    let required = node.props.required.unwrap_or(false);
    let has_bind = node
        .props
        .bind
        .as_ref()
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false);
    let select_value = node
        .props
        .extra
        .get("__boundValue")
        .map(json_value_to_display_string)
        .unwrap_or_default();
    let select_name = form_control_name(&node);
    let bind_node = node.clone();

    let mut data_state = use_context::<GlobalDataState>().0;
    let local_item_sig = use_context::<RepeaterItemState>().0;
    let local_item = local_item_sig.read().clone();
    let load_actions = dropdown_load_actions(&node);
    let effect_item = local_item.clone();

    use_effect(move || {
        let actions = load_actions.clone();
        let item = effect_item.clone();
        if actions.is_empty() {
            return;
        }

        spawn(async move {
            execute_actions(actions, data_state, item).await;
        });
    });

    let global_data = data_state.read().clone();
    let options = dropdown_options_for_node(&node, local_item.as_ref(), &global_data);

    let base_style = "display: flex; flex-direction: column; gap: 6px; font-family: 'Outfit', sans-serif; font-size: 14px;";
    let select_container_style = format!("{} {}", base_style, styles);

    rsx! {
        div {
            id: "{node.id}",
            style: "{select_container_style}",
            label {
                style: "font-weight: 600; color: #475569;",
                "{label}"
                if required {
                    span { style: "color: #ef4444; margin-left: 2px;", "*" }
                }
            }
            select {
                name: "{select_name}",
                value: if has_bind { select_value.clone() },
                required: required,
                style: "padding: 10px 12px; border-radius: var(--radius, 8px); border: 1px solid rgba(0,0,0,0.08); background-color: #ffffff; color: #1e293b; outline: none; font-family: inherit; font-size: 14px; cursor: pointer; box-sizing: border-box; width: 100%;",
                onchange: move |evt| {
                    let mut data = data_state.write();
                    set_node_bind_value(&bind_node, &mut data, serde_json::Value::String(evt.value()));
                },
                if !placeholder.is_empty() {
                    option {
                        value: "",
                        disabled: true,
                        "{placeholder}"
                    }
                }
                if options.is_empty() {
                    option {
                        value: "",
                        disabled: true,
                        "No options available"
                    }
                } else {
                    for option in options {
                        option {
                            key: "{option.value}",
                            value: "{option.value}",
                            "{option.label}"
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn Hooked_MultiSelectDropdown(
    node: ComponentNode,
    styles: String,
    selected_id: Option<Signal<Option<String>>>,
    on_select: Option<EventHandler<String>>,
    on_drop: Option<EventHandler<(String, f64, f64)>>,
    on_delete: Option<EventHandler<String>>,
) -> Element {
    let label = node
        .props
        .label
        .clone()
        .unwrap_or_else(|| "Select Categories".to_string());
    let has_bind = node
        .props
        .bind
        .as_ref()
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false);
    let initial_selected = dropdown_selected_values(node.props.extra.get("__boundValue"));
    let mut selected = use_signal(move || initial_selected.clone());
    let mut show_opts = use_signal(|| false);
    let bind_node = node.clone();

    let mut data_state = use_context::<GlobalDataState>().0;
    let local_item_sig = use_context::<RepeaterItemState>().0;
    let local_item = local_item_sig.read().clone();
    let load_actions = dropdown_load_actions(&node);
    let effect_item = local_item.clone();

    use_effect(move || {
        let actions = load_actions.clone();
        let item = effect_item.clone();
        if actions.is_empty() {
            return;
        }

        spawn(async move {
            execute_actions(actions, data_state, item).await;
        });
    });

    let global_data = data_state.read().clone();
    let options = dropdown_options_for_node(&node, local_item.as_ref(), &global_data);
    let selected_values = if has_bind {
        dropdown_selected_values(node.props.extra.get("__boundValue"))
    } else {
        selected()
    };
    let selected_tags: Vec<(String, String)> = selected_values
        .iter()
        .map(|value| (value.clone(), dropdown_option_label(&options, value)))
        .collect();

    let base_style = "display: flex; flex-direction: column; gap: 6px; font-family: 'Outfit', sans-serif; font-size: 14px; position: relative;";
    let ms_style = format!("{} {}", base_style, styles);

    rsx! {
        div {
            id: "{node.id}",
            style: "{ms_style}",
            label { style: "font-weight: 600; color: #475569;", "{label}" }
            div {
                style: "padding: 8px 10px; border-radius: 8px; border: 1px solid rgba(0,0,0,0.08); background-color: #ffffff; min-height: 40px; display: flex; flex-wrap: wrap; gap: 6px; align-items: center; cursor: pointer; box-sizing: border-box;",
                onclick: move |_| *show_opts.write() = !show_opts(),

                for (tag_value, tag_label) in selected_tags {
                    {
                        let remove_value = tag_value.clone();
                        let current_values = selected_values.clone();
                        let remove_bind_node = bind_node.clone();
                        rsx! {
                            div {
                                key: "{tag_value}",
                                style: "background-color: #eff6ff; color: #1d4ed8; border: 1px solid #bfdbfe; border-radius: 6px; padding: 2px 6px; font-size: 12px; display: flex; align-items: center; gap: 4px; font-weight: 600;",
                                span { "{tag_label}" }
                                span {
                                    style: "font-weight: bold; font-size: 10px; opacity: 0.8; margin-left: 2px;",
                                    onclick: move |e| {
                                        e.stop_propagation();
                                        let mut tags = if has_bind {
                                            current_values.clone()
                                        } else {
                                            selected.read().clone()
                                        };
                                        tags.retain(|tag| tag != &remove_value);
                                        *selected.write() = tags.clone();
                                        if has_bind {
                                            let mut data = data_state.write();
                                            set_node_bind_value(&remove_bind_node, &mut data, dropdown_values_to_json(&tags));
                                        }
                                    },
                                    "×"
                                }
                            }
                        }
                    }
                }
                if selected_values.is_empty() {
                    span { style: "color: #94a3b8; font-size: 13px;", "Select options..." }
                }
            }

            if show_opts() {
                div {
                    style: "position: absolute; top: 100%; left: 0; right: 0; background-color: #ffffff; border: 1px solid #e2e8f0; border-radius: 8px; box-shadow: 0 4px 12px rgba(0,0,0,0.08); z-index: 100; margin-top: 4px; overflow: hidden; display: flex; flex-direction: column;",
                    if options.is_empty() {
                        div {
                            style: "padding: 10px 12px; font-size: 13px; color: #94a3b8;",
                            "No options available"
                        }
                    } else {
                        for opt in options {
                            {
                                let is_sel = selected_values.contains(&opt.value);
                                let bg_c = if is_sel { "#f1f5f9" } else { "#ffffff" };
                                let text_c = if is_sel { "#1d4ed8" } else { "#334155" };
                                let f_weight = if is_sel { "600" } else { "normal" };
                                let opt_value = opt.value.clone();
                                let opt_label = opt.label.clone();
                                let current_values = selected_values.clone();
                                let option_bind_node = bind_node.clone();
                                rsx! {
                                    div {
                                        key: "{opt_value}",
                                        style: "padding: 10px 12px; cursor: pointer; font-size: 13px; background-color: {bg_c}; color: {text_c}; font-weight: {f_weight};",
                                        onclick: move |_| {
                                            let mut tags = if has_bind {
                                                current_values.clone()
                                            } else {
                                                selected.read().clone()
                                            };
                                            if tags.contains(&opt_value) {
                                                tags.retain(|tag| tag != &opt_value);
                                            } else {
                                                tags.push(opt_value.clone());
                                            }
                                            *selected.write() = tags.clone();
                                            if has_bind {
                                                let mut data = data_state.write();
                                                set_node_bind_value(&option_bind_node, &mut data, dropdown_values_to_json(&tags));
                                            }
                                            *show_opts.write() = false;
                                        },
                                        "{opt_label}"
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn Hooked_Button(
    node: ComponentNode,
    styles: String,
    selected_id: Option<Signal<Option<String>>>,
    on_select: Option<EventHandler<String>>,
    on_drop: Option<EventHandler<(String, f64, f64)>>,
    on_delete: Option<EventHandler<String>>,
) -> Element {
    let data_state = use_context::<GlobalDataState>().0;
    let local_item_sig = use_context::<RepeaterItemState>().0;
    let local_item = local_item_sig.read().clone();

    let raw_label = node
        .props
        .label
        .clone()
        .unwrap_or_else(|| "Button".to_string());
    let label = resolve_string_templates(&raw_label, local_item.as_ref(), &data_state.read());
    let variant = node
        .props
        .variant
        .clone()
        .unwrap_or_else(|| "primary".to_string());
    let raw_actions = node.on_click.clone();
    let actions = button_click_actions(&raw_actions);
    let button_type = if button_has_submit_form_action(&raw_actions) && actions.is_empty() {
        "submit".to_string()
    } else {
        node_extra_str(&node, &["type", "buttonType", "button_type"])
            .unwrap_or("button")
            .to_string()
    };

    let (bg, text_c, border_c) = match variant.as_str() {
        "danger" => (
            "linear-gradient(135deg,#dc2626,#b91c1c)",
            "#ffffff",
            "transparent",
        ),
        "ghost" => ("transparent", "#475569", "1px solid #e2e8f0"),
        "outline" => ("transparent", "#3b82f6", "1px solid #3b82f6"),
        "success" => (
            "linear-gradient(135deg,#059669,#047857)",
            "#ffffff",
            "transparent",
        ),
        _ => (
            "linear-gradient(135deg,#030213,#1e1b4b)",
            "#ffffff",
            "transparent",
        ),
    };
    let base_style = format!("cursor: pointer; display: inline-flex; align-items: center; justify-content: center; font-weight: 600; font-family: 'Outfit', sans-serif; transition: all 0.2s; border-radius: var(--radius,10px); outline: none; border: {}; background: {}; color: {}; padding: 10px 20px; user-select: none; box-shadow: 0 4px 10px rgba(3,2,19,0.12);", border_c, bg, text_c);
    let btn_style = format!("{} {}", base_style, styles);

    rsx! {
        button {
            id: "{node.id}",
            r#type: "{button_type}",
            style: "{btn_style}",
            onclick: move |_| {
                if !actions.is_empty() {
                    let acts = actions.clone();
                    let item = local_item.clone();
                    spawn(async move {
                        execute_actions(acts, data_state, item).await;
                    });
                }
            },
            "{label}"
        }
    }
}

/// Dispatches a list of actions sequentially.
/// Each action is a JSON object matching the home.json event shape:
/// `{ "type": "API_CALL", "payload": { ... } }` (legacy) or flat builder shape.
pub(crate) fn execute_actions(
    actions: Vec<serde_json::Value>,
    data_state: Signal<serde_json::Value>,
    local_item: Option<serde_json::Value>,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + 'static>> {
    Box::pin(async move {
        execute_actions_impl(actions, data_state, local_item).await;
    })
}

async fn execute_actions_impl(
    actions: Vec<serde_json::Value>,
    mut data_state: Signal<serde_json::Value>,
    local_item: Option<serde_json::Value>,
) {
    for action in actions {
        let action = resolve_json_templates(&action, local_item.as_ref(), &data_state.read());
        let action_type = action.get("type").and_then(|v| v.as_str()).unwrap_or("");

        // Support both `payload.*` wrapper (home.json) and flat builder schema
        let payload = action
            .get("payload")
            .cloned()
            .unwrap_or_else(|| action.clone());
        let success_actions = action_success_actions(&action);
        let error_actions = action_error_actions(&action);
        let mut action_failed = false;

        match action_type {
            "API_CALL" => {
                let url = payload
                    .get("url")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let method = payload
                    .get("method")
                    .and_then(|v| v.as_str())
                    .unwrap_or("POST")
                    .to_string();
                let body = payload
                    .get("body")
                    .cloned()
                    .unwrap_or(serde_json::Value::Object(Default::default()));

                if !url.is_empty() {
                    let mut parsed = serde_json::Value::Null;
                    let mut success = false;

                    #[cfg(target_arch = "wasm32")]
                    {
                        use wasm_bindgen::JsValue;
                        use wasm_bindgen_futures::JsFuture;
                        use web_sys::{Request, RequestInit, RequestMode, Response};

                        let opts = RequestInit::new();
                        opts.set_method(&method);
                        opts.set_mode(RequestMode::Cors);
                        if method != "GET" {
                            let body_str = serde_json::to_string(&body).unwrap_or_default();
                            opts.set_body(&JsValue::from_str(&body_str));
                        }

                        if let Ok(request) = Request::new_with_str_and_init(&url, &opts) {
                            let _ = request.headers().set("Content-Type", "application/json");

                            if let Some(window) = web_sys::window() {
                                if let Ok(resp_val) =
                                    JsFuture::from(window.fetch_with_request(&request)).await
                                {
                                    if let Ok(resp) = resp_val.dyn_into::<Response>() {
                                        if let Ok(json_promise) = resp.json() {
                                            if let Ok(json_val) = JsFuture::from(json_promise).await
                                            {
                                                if let Ok(json_str) =
                                                    js_sys::JSON::stringify(&json_val)
                                                {
                                                    if let Ok(p) = serde_json::from_str(
                                                        &String::from(json_str),
                                                    ) {
                                                        parsed = p;
                                                        success = true;
                                                        web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&format!(
                                                            "API CALL succeeded: url = {}, parsed = {:?}", url, parsed
                                                        )));
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    if !success {
                        let workflow_name = body
                            .get("workflow_name")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        let master_name = body
                            .get("master_name")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");

                        if workflow_name == "get_mongo_master_with_cond" && master_name == "email" {
                            let mock_email_list = serde_json::json!([
                                {
                                    "from_name": "R K",
                                    "from_email": "rk@assisto.tech",
                                    "to": [
                                        {
                                            "name": "Demo Assisto",
                                            "email": "Demo@assisto.tech"
                                        }
                                    ],
                                    "subject": "renew policy",
                                    "body": "renew policy\n\nI want to take a policy for my honda city car up 16r 4997 pls tell me quote"
                                },
                                {
                                    "from_name": "Demo Assisto",
                                    "from_email": "Demo@assisto.tech",
                                    "to": [
                                        {
                                            "name": "R K",
                                            "email": "rk@assisto.tech"
                                        }
                                    ],
                                    "subject": "renew policy",
                                    "body": "Dear Customer,\n\nThank you for contacting us.\n\nTo proceed with your request, we require the following missing information:\n\n- policy_no\n- year of manufacture"
                                }
                            ]);

                            let mock_email_detail = serde_json::json!({
                                "id": "1",
                                "email": "rk@assisto.tech",
                                "subject": "renew policy",
                                "sender": "R K <rk@assisto.tech>",
                                "receiver": "Demo Assisto <Demo@assisto.tech>",
                                "message": "renew policy\n\nI want to take a policy for my honda city car up 16r 4997 pls tell me quote",
                                "emails": mock_email_list.clone()
                            });

                            let mock_re_result = serde_json::json!({
                                "functions_get_mongo_master_output_data": [
                                    mock_email_detail.clone()
                                ]
                            });

                            parsed = mock_re_result;

                            // Also write emaildetails directly to global state so that both formats resolve
                            {
                                let mut data = data_state.write();
                                if let Some(obj) = data.as_object_mut() {
                                    obj.insert("emaildetails".to_string(), mock_email_detail);
                                }
                            }

                            #[cfg(target_arch = "wasm32")]
                            web_sys::console::log_1(&wasm_bindgen::JsValue::from_str("API CALL failed or offline. Applied master email mock payload to re_result and emaildetails."));
                        } else {
                            parsed = serde_json::json!({});
                        }
                    }

                    let fail_on_status_failed = payload
                        .get("failOnStatusFailed")
                        .or_else(|| payload.get("fail_on_status_failed"))
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                    let status_failed = parsed
                        .get("status")
                        .and_then(|v| v.as_str())
                        .map(|status| status.eq_ignore_ascii_case("failed"))
                        .unwrap_or(false);

                    if fail_on_status_failed && status_failed {
                        action_failed = true;
                    } else {
                        let target_key = payload
                            .get("targetKey")
                            .or_else(|| payload.get("target_key"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("result");

                        // Insert into the global data state
                        {
                            let mut data = data_state.write();
                            if let Some(obj) = data.as_object_mut() {
                                obj.insert(target_key.to_string(), parsed);
                            } else {
                                let mut map = serde_json::Map::new();
                                map.insert(target_key.to_string(), parsed);
                                *data = serde_json::Value::Object(map);
                            }
                        }
                    }
                }
            }

            "NAVIGATE" => {
                let page_id = payload
                    .get("pageId")
                    .or_else(|| payload.get("page_id"))
                    .or_else(|| payload.get("to"))
                    .or_else(|| payload.get("target"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                if !page_id.is_empty() {
                    let force_refresh = payload
                        .get("forceRefresh")
                        .or_else(|| payload.get("force_refresh"))
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);

                    // Process routeParams
                    if let Some(params) = payload
                        .get("routeParams")
                        .or_else(|| payload.get("route_params"))
                        .or_else(|| payload.get("params"))
                    {
                        let resolved_params =
                            resolve_json_templates(params, local_item.as_ref(), &data_state.read());
                        let mut data = data_state.write();
                        set_json_value_at_path(&mut data, "data.routeParams", resolved_params);
                    }

                    // Process dataMappings
                    if let Some(mappings) = payload
                        .get("dataMappings")
                        .or_else(|| payload.get("data_mappings"))
                        .and_then(|v| v.as_array())
                    {
                        for mapping in mappings {
                            if let Some(path) = mapping.get("path").and_then(|v| v.as_str()) {
                                if let Some(val) = mapping.get("value") {
                                    let resolved_val = resolve_json_templates(
                                        val,
                                        local_item.as_ref(),
                                        &data_state.read(),
                                    );
                                    let mut data = data_state.write();
                                    set_json_value_at_path(&mut data, path, resolved_val);
                                }
                            }
                        }
                    }

                    let target_route = if page_id.starts_with('/') {
                        page_id.to_string()
                    } else {
                        let routes_opt = data_state
                            .read()
                            .get("__routes")
                            .and_then(|v| v.as_array())
                            .cloned();

                        let mut found_route = format!("/{}", page_id);
                        if let Some(routes) = routes_opt {
                            for r in routes {
                                let r_page_id = r
                                    .get("page_id")
                                    .or_else(|| r.get("pageId"))
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("");
                                let r_path = r.get("path").and_then(|v| v.as_str()).unwrap_or("");
                                if r_page_id == page_id && !r_path.is_empty() {
                                    found_route = r_path.to_string();
                                    break;
                                }
                            }
                        }
                        found_route
                    };

                    #[cfg(target_arch = "wasm32")]
                    if let Some(window) = web_sys::window() {
                        let next = normalize_href_for_navigate(&target_route);
                        if force_refresh {
                            let _ = window.location().set_href(&next);
                        } else {
                            if let Ok(history) = window.history() {
                                let _ = history.push_state_with_url(
                                    &wasm_bindgen::JsValue::NULL,
                                    "",
                                    Some(&next),
                                );
                                if let Ok(evt) = web_sys::Event::new("popstate") {
                                    let _ = window.dispatch_event(&evt);
                                }
                                window.scroll_to_with_x_and_y(0.0, 0.0);
                            }
                        }
                    }
                }
            }

            "OPEN_MODAL" => {
                let modal_id = payload
                    .get("modalId")
                    .or_else(|| payload.get("modal_id"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                if !modal_id.is_empty() {
                    // Process dataMappings
                    if let Some(mappings) = payload
                        .get("dataMappings")
                        .or_else(|| payload.get("data_mappings"))
                        .and_then(|v| v.as_array())
                    {
                        for mapping in mappings {
                            if let Some(path) = mapping.get("path").and_then(|v| v.as_str()) {
                                if let Some(val) = mapping.get("value") {
                                    let resolved_val = resolve_json_templates(
                                        val,
                                        local_item.as_ref(),
                                        &data_state.read(),
                                    );
                                    let mut data = data_state.write();
                                    set_json_value_at_path(&mut data, path, resolved_val);
                                }
                            }
                        }
                    }

                    #[cfg(target_arch = "wasm32")]
                    if let Some(window) = web_sys::window() {
                        if let Some(document) = window.document() {
                            if let Some(el) = document.get_element_by_id(modal_id) {
                                if let Ok(evt) = web_sys::Event::new("open-modal") {
                                    let _ = el.dispatch_event(&evt);
                                }
                            }
                        }
                    }
                }
            }

            "CLOSE_MODAL" =>
            {
                #[cfg(target_arch = "wasm32")]
                if let Some(window) = web_sys::window() {
                    if let Some(document) = window.document() {
                        if let Ok(modals) = document.query_selector_all("[data-modal-open='true']")
                        {
                            for i in 0..modals.length() {
                                if let Some(el) = modals.item(i) {
                                    if let Ok(html_el) = el.dyn_into::<web_sys::Element>() {
                                        if let Ok(evt) = web_sys::Event::new("close-modal") {
                                            let _ = html_el.dispatch_event(&evt);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            "TOAST" => {
                let message = payload
                    .get("message")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Done");
                #[cfg(target_arch = "wasm32")]
                if let Some(window) = web_sys::window() {
                    let _ = window.alert_with_message(message);
                }
            }

            _ => {}
        }

        if action_failed {
            if let Some(arr) = error_actions {
                execute_actions(arr, data_state, local_item.clone()).await;
            }
        } else if let Some(arr) = success_actions {
            execute_actions(arr, data_state, local_item.clone()).await;
        }
    }
}

fn get_actual_data_path(path: &str) -> String {
    let raw = path.trim();
    if raw.starts_with("data.") {
        raw[5..].to_string()
    } else if raw.starts_with("ui.") {
        raw[3..].to_string()
    } else {
        raw.to_string()
    }
}

fn set_json_value_at_path(root: &mut serde_json::Value, path: &str, value: serde_json::Value) {
    let actual_path = get_actual_data_path(path);
    if actual_path.is_empty() {
        return;
    }

    let parts: Vec<&str> = actual_path.split('.').collect();
    let mut current = root;

    for (i, part) in parts.iter().enumerate() {
        if i == parts.len() - 1 {
            if let Some(obj) = current.as_object_mut() {
                obj.insert(part.to_string(), value);
            } else {
                let mut map = serde_json::Map::new();
                map.insert(part.to_string(), value);
                *current = serde_json::Value::Object(map);
            }
            return;
        }

        if !current.is_object() {
            *current = serde_json::Value::Object(serde_json::Map::new());
        }

        if current.get(*part).is_none() {
            current.as_object_mut().unwrap().insert(
                part.to_string(),
                serde_json::Value::Object(serde_json::Map::new()),
            );
        }

        current = current.get_mut(*part).unwrap();
    }
}

fn resolve_json_value_path(val: &serde_json::Value, path: &str) -> Option<serde_json::Value> {
    let mut current = val;
    let parts = path.split('.');

    for part in parts {
        if part.contains('[') && part.contains(']') {
            let open_idx = part.find('[')?;
            let close_idx = part.find(']')?;
            let key = &part[..open_idx];
            let idx_str = &part[open_idx + 1..close_idx];
            let idx = idx_str.parse::<usize>().ok()?;

            if !key.is_empty() {
                current = current.get(key)?;
            }
            current = current.get(idx)?;
        } else {
            current = current.get(part)?;
        }
    }
    Some(current.clone())
}

fn resolve_string_templates(
    template: &str,
    local_item: Option<&serde_json::Value>,
    global_data: &serde_json::Value,
) -> String {
    if !template.contains("{{") {
        return template.to_string();
    }

    let mut result = template.to_string();
    while let Some(start) = result.find("{{") {
        if let Some(end) = result[start..].find("}}") {
            let actual_end = start + end;
            let full_expr = &result[start..actual_end + 2];
            let path_expr = result[start + 2..actual_end].trim();
            let path_lower = path_expr.to_lowercase();

            let resolved_val = if path_lower.starts_with("item.") {
                let sub_path = &path_expr[5..];
                local_item
                    .and_then(|item| resolve_json_path(item, sub_path))
                    .unwrap_or_else(|| "".to_string())
            } else if path_lower == "item" {
                local_item
                    .map(|item| match item {
                        serde_json::Value::String(s) => s.clone(),
                        other => other.to_string(),
                    })
                    .unwrap_or_default()
            } else if path_lower.starts_with("rowdata.") {
                let sub_path = &path_expr[8..];
                local_item
                    .and_then(|item| resolve_json_path(item, sub_path))
                    .unwrap_or_else(|| "".to_string())
            } else if path_lower == "rowdata" {
                local_item
                    .map(|item| match item {
                        serde_json::Value::String(s) => s.clone(),
                        other => other.to_string(),
                    })
                    .unwrap_or_default()
            } else if path_lower.starts_with("row.") {
                let sub_path = &path_expr[4..];
                local_item
                    .and_then(|item| resolve_json_path(item, sub_path))
                    .unwrap_or_else(|| "".to_string())
            } else if path_lower == "row" {
                local_item
                    .map(|item| match item {
                        serde_json::Value::String(s) => s.clone(),
                        other => other.to_string(),
                    })
                    .unwrap_or_default()
            } else if path_expr.starts_with("data.") {
                let sub_path = &path_expr[5..];
                resolve_json_path(global_data, sub_path).unwrap_or_else(|| "".to_string())
            } else {
                local_item
                    .and_then(|item| resolve_json_path(item, path_expr))
                    .or_else(|| resolve_json_path(global_data, path_expr))
                    .unwrap_or_else(|| "".to_string())
            };

            result = result.replace(full_expr, &resolved_val);
        } else {
            break;
        }
    }
    result
}

fn resolve_json_path(val: &serde_json::Value, path: &str) -> Option<String> {
    let mut current = val;
    let parts = path.split('.');

    for part in parts {
        if part.contains('[') && part.contains(']') {
            let open_idx = part.find('[')?;
            let close_idx = part.find(']')?;
            let key = &part[..open_idx];
            let idx_str = &part[open_idx + 1..close_idx];
            let idx = idx_str.parse::<usize>().ok()?;

            if !key.is_empty() {
                current = current.get(key)?;
            }
            current = current.get(idx)?;
        } else {
            current = current.get(part)?;
        }
    }

    match current {
        serde_json::Value::String(s) => Some(s.clone()),
        serde_json::Value::Number(n) => Some(n.to_string()),
        serde_json::Value::Bool(b) => Some(b.to_string()),
        serde_json::Value::Null => Some("".to_string()),
        other => Some(other.to_string()),
    }
}

fn resolve_json_templates(
    val: &serde_json::Value,
    local_item: Option<&serde_json::Value>,
    global_data: &serde_json::Value,
) -> serde_json::Value {
    match val {
        serde_json::Value::String(s) => {
            let trimmed = s.trim();
            if trimmed.starts_with("{{") && trimmed.ends_with("}}") {
                let expr = trimmed[2..trimmed.len() - 2].trim();
                let expr_lower = expr.to_lowercase();

                let resolved = if expr_lower.starts_with("item.") {
                    let sub_path = &expr[5..];
                    local_item.and_then(|item| resolve_json_value_path(item, sub_path))
                } else if expr_lower == "item" {
                    local_item.cloned()
                } else if expr_lower.starts_with("rowdata.") {
                    let sub_path = &expr[8..];
                    local_item.and_then(|item| resolve_json_value_path(item, sub_path))
                } else if expr_lower == "rowdata" {
                    local_item.cloned()
                } else if expr_lower.starts_with("row.") {
                    let sub_path = &expr[4..];
                    local_item.and_then(|item| resolve_json_value_path(item, sub_path))
                } else if expr_lower == "row" {
                    local_item.cloned()
                } else if expr_lower.starts_with("data.") {
                    let sub_path = &expr[5..];
                    resolve_json_value_path(global_data, sub_path)
                } else {
                    local_item
                        .and_then(|item| resolve_json_value_path(item, expr))
                        .or_else(|| resolve_json_value_path(global_data, expr))
                };

                if let Some(v) = resolved {
                    return v;
                }
            }
            serde_json::Value::String(resolve_string_templates(s, local_item, global_data))
        }
        serde_json::Value::Object(map) => {
            let mut resolved_map = serde_json::Map::new();
            for (k, v) in map {
                resolved_map.insert(
                    k.clone(),
                    resolve_json_templates(v, local_item, global_data),
                );
            }
            serde_json::Value::Object(resolved_map)
        }
        serde_json::Value::Array(arr) => {
            let resolved_arr = arr
                .iter()
                .map(|item| resolve_json_templates(item, local_item, global_data))
                .collect();
            serde_json::Value::Array(resolved_arr)
        }
        other => other.clone(),
    }
}

pub(crate) fn resolve_array_path(
    val: &serde_json::Value,
    path: &str,
) -> Option<Vec<serde_json::Value>> {
    let mut current = val;
    let parts = path.split('.');

    for part in parts {
        if part.contains('[') && part.contains(']') {
            let open_idx = part.find('[')?;
            let close_idx = part.find(']')?;
            let key = &part[..open_idx];
            let idx_str = &part[open_idx + 1..close_idx];
            let idx = idx_str.parse::<usize>().ok()?;

            if !key.is_empty() {
                current = current.get(key)?;
            }
            current = current.get(idx)?;
        } else {
            current = current.get(part)?;
        }
    }

    current.as_array().cloned()
}

// ── DynamicTable ──────────────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq)]
struct ColDef {
    key: String,
    label: String,
}

fn extract_array(parsed: &serde_json::Value, tkey: &str) -> Option<Vec<serde_json::Value>> {
    // 1. Try dot-path lookup first
    if !tkey.is_empty() {
        let mut current = parsed;
        let mut found = true;
        for part in tkey.split('.') {
            if let Some(next) = current.get(part) {
                current = next;
            } else {
                found = false;
                break;
            }
        }
        if found {
            if let Some(arr) = current.as_array() {
                return Some(arr.clone());
            }
        }
    }

    // 2. If dot-path failed or didn't point to an array, check if parsed itself is an array
    if let Some(arr) = parsed.as_array() {
        return Some(arr.clone());
    }

    // 3. Check if the parsed object is an Object, and search keys
    if let Some(obj) = parsed.as_object() {
        for key in &[
            "data",
            "rows",
            "result",
            "items",
            "output_data",
            "outputData",
        ] {
            if let Some(val) = obj.get(*key) {
                if let Some(arr) = val.as_array() {
                    return Some(arr.clone());
                }
            }
        }
        for (k, val) in obj {
            let k_lower = k.to_lowercase();
            if k_lower.contains("data") || k_lower.contains("rows") || k_lower.contains("output") {
                if let Some(arr) = val.as_array() {
                    return Some(arr.clone());
                }
            }
        }
        for (_k, val) in obj {
            if let Some(arr) = val.as_array() {
                return Some(arr.clone());
            }
        }
    }
    None
}

#[component]
fn TableRowActionWrapper(
    action_node: ComponentNode,
    row_data: serde_json::Value,
    selected_id: Option<Signal<Option<String>>>,
    on_select: Option<EventHandler<String>>,
    on_drop: Option<EventHandler<(String, f64, f64)>>,
    on_delete: Option<EventHandler<String>>,
    on_resize_start: Option<EventHandler<(String, String, f64, f64)>>,
    on_drag_start: Option<EventHandler<(String, f64, f64)>>,
) -> Element {
    let sig = use_signal(|| Some(row_data));
    use_context_provider(|| RepeaterItemState(sig));

    rsx! {
        ComponentRenderer {
            node: action_node,
            selected_id,
            on_select,
            on_drop,
            on_delete,
            on_resize_start,
            on_drag_start,
        }
    }
}

fn resolve_cell_value(row: &serde_json::Value, key: &str) -> serde_json::Value {
    if key.contains('.') {
        let parts = key.split('.');
        let mut current = row;
        for part in parts {
            if let Some(next) = current.get(part) {
                current = next;
            } else {
                return serde_json::Value::Null;
            }
        }
        current.clone()
    } else {
        row.get(key).cloned().unwrap_or(serde_json::Value::Null)
    }
}

#[component]
fn Hooked_DynamicTable(
    node: ComponentNode,
    styles: String,
    #[props(default)] selected_id: Option<Signal<Option<String>>>,
    #[props(default)] on_select: Option<EventHandler<String>>,
    #[props(default)] on_drop: Option<EventHandler<(String, f64, f64)>>,
    #[props(default)] on_delete: Option<EventHandler<String>>,
    #[props(default)] on_resize_start: Option<EventHandler<(String, String, f64, f64)>>,
    #[props(default)] on_drag_start: Option<EventHandler<(String, f64, f64)>>,
) -> Element {
    // --- Parse config from props extra/columns, checking both direct and extra fields ---
    let api_url = node
        .props
        .extra
        .get("apiUrl")
        .and_then(|v| v.as_str())
        .or_else(|| node.props.extra.get("api_url").and_then(|v| v.as_str()))
        .unwrap_or("")
        .to_string();

    let method = node
        .props
        .method
        .clone()
        .or_else(|| {
            node.props
                .extra
                .get("method")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        })
        .unwrap_or_else(|| "POST".to_string());

    let target_key = node
        .props
        .extra
        .get("targetKey")
        .and_then(|v| v.as_str())
        .or_else(|| node.props.extra.get("target_key").and_then(|v| v.as_str()))
        .unwrap_or("result")
        .to_string();

    let req_body = node
        .props
        .request_body
        .clone()
        .or_else(|| node.props.extra.get("requestBody").cloned())
        .or_else(|| node.props.extra.get("request_body").cloned())
        .unwrap_or_else(|| serde_json::Value::Object(Default::default()));

    let show_search = node
        .props
        .extra
        .get("showSearchBar")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    let search_ph = node
        .props
        .extra
        .get("searchPlaceholder")
        .and_then(|v| v.as_str())
        .unwrap_or("Search...")
        .to_string();
    let page_size = node
        .props
        .extra
        .get("pageSize")
        .and_then(|v| v.as_u64())
        .unwrap_or(20) as usize;
    let on_load_actions = node.on_load.clone();

    // Column definitions: array of {key, label} objects
    let col_defs: Vec<ColDef> = node
        .props
        .columns
        .clone()
        .and_then(|v| v.as_array().cloned())
        .unwrap_or_default()
        .into_iter()
        .filter_map(|c| {
            let key = c.get("key").and_then(|v| v.as_str())?.to_string();
            let label = c
                .get("label")
                .and_then(|v| v.as_str())
                .unwrap_or(&key)
                .to_string();
            Some(ColDef { key, label })
        })
        .collect();

    let row_action_node: Option<ComponentNode> = node
        .props
        .extra
        .get("rowActionComponent")
        .and_then(|v| serde_json::from_value(v.clone()).ok());

    // --- State ---
    let data_state = use_context::<GlobalDataState>().0;
    let local_item_sig = use_context::<RepeaterItemState>().0;
    let local_item = local_item_sig.read().clone();

    let mut rows: Signal<Vec<serde_json::Value>> = use_signal(Vec::new);
    let mut loading = use_signal(|| true);
    let mut search_query = use_signal(String::new);
    let mut current_page = use_signal(|| 0usize);
    let mut error_msg = use_signal(|| Option::<String>::None);

    // --- Fetch on mount ---
    let fetch_url = api_url.clone();
    let fetch_method = method.clone();
    let fetch_body = req_body.clone();
    let fetch_tkey = target_key.clone();
    use_effect(move || {
        let url = fetch_url.clone();
        let meth = fetch_method.clone();
        let body = fetch_body.clone();
        let tkey = fetch_tkey.clone();
        // Fire onLoad actions first
        let ol = on_load_actions.clone();
        let item = local_item.clone();
        spawn(async move {
            execute_actions(ol, data_state, item).await;
            if url.is_empty() {
                *loading.write() = false;
                return;
            }

            #[cfg(target_arch = "wasm32")]
            {
                use wasm_bindgen::JsValue;
                use wasm_bindgen_futures::JsFuture;
                use web_sys::{Request, RequestInit, RequestMode, Response};

                let opts = RequestInit::new();
                opts.set_method(&meth);
                opts.set_mode(RequestMode::Cors);
                if meth != "GET" {
                    let body_str = serde_json::to_string(&body).unwrap_or_default();
                    opts.set_body(&JsValue::from_str(&body_str));
                }
                let request = match Request::new_with_str_and_init(&url, &opts) {
                    Ok(r) => r,
                    Err(e) => {
                        *error_msg.write() = Some(format!("Request error: {:?}", e));
                        *loading.write() = false;
                        return;
                    }
                };
                let _ = request.headers().set("Content-Type", "application/json");
                let window = match web_sys::window() {
                    Some(w) => w,
                    None => {
                        *loading.write() = false;
                        return;
                    }
                };
                let resp_val = match JsFuture::from(window.fetch_with_request(&request)).await {
                    Ok(v) => v,
                    Err(e) => {
                        *error_msg.write() = Some(format!("Fetch error: {:?}", e));
                        *loading.write() = false;
                        return;
                    }
                };
                let resp: Response = match resp_val.dyn_into() {
                    Ok(r) => r,
                    Err(_) => {
                        *loading.write() = false;
                        return;
                    }
                };
                let json_val = match JsFuture::from(resp.json().unwrap()).await {
                    Ok(v) => v,
                    Err(_) => {
                        *loading.write() = false;
                        return;
                    }
                };
                let json_str = match js_sys::JSON::stringify(&json_val) {
                    Ok(s) => String::from(s),
                    Err(_) => {
                        *loading.write() = false;
                        return;
                    }
                };
                let parsed: serde_json::Value =
                    serde_json::from_str(&json_str).unwrap_or(serde_json::Value::Null);
                if let Some(arr) = extract_array(&parsed, &tkey) {
                    *rows.write() = arr;
                }
            }

            #[cfg(not(target_arch = "wasm32"))]
            {
                *rows.write() = vec![];
            }

            *loading.write() = false;
        });
    });

    // --- Derived: filtered rows ---
    let q = search_query.read().to_lowercase();
    let filtered: Vec<serde_json::Value> = rows
        .read()
        .iter()
        .filter(|row| {
            if q.is_empty() {
                return true;
            }
            col_defs.iter().any(|cd| {
                let cell_val = resolve_cell_value(row, &cd.key);
                val_to_css(&cell_val).to_lowercase().contains(&q)
            })
        })
        .cloned()
        .collect();

    let total_pages = if page_size == 0 {
        1
    } else {
        ((filtered.len() + page_size - 1).max(1) / page_size).max(1)
    };
    let active_page = current_page().min(total_pages - 1);
    let page_rows = if page_size == 0 {
        filtered.clone()
    } else {
        let page_start = active_page * page_size;
        let page_end = (page_start + page_size).min(filtered.len());
        if filtered.is_empty() || page_start >= filtered.len() {
            vec![]
        } else {
            filtered[page_start..page_end].to_vec()
        }
    };

    let has_action_col = row_action_node.is_some();
    let action_col_w = node
        .props
        .extra
        .get("actionColumnWidth")
        .and_then(|v| v.as_str())
        .unwrap_or("120px")
        .to_string();

    let wrapper_style = format!("background:#fff; border-radius:12px; border:1px solid rgba(0,0,0,0.08); font-family:'Outfit',sans-serif; box-shadow:0 10px 25px -5px rgba(0,0,0,0.06); overflow:hidden; display:flex; flex-direction:column; {}", styles);

    rsx! {
        div {
            id: "{node.id}",
            style: "{wrapper_style}",

            // Search bar
            if show_search {
                div {
                    style: "padding:12px 16px; border-bottom:1px solid rgba(0,0,0,0.06); display:flex; align-items:center; gap:8px; background:#f8fafc;",
                    span { style: "color:#94a3b8; font-size:14px;", "🔍" }
                    input {
                        r#type: "text",
                        placeholder: "{search_ph}",
                        value: "{search_query}",
                        style: "border:none; outline:none; background:transparent; font-size:13px; font-family:'Outfit',sans-serif; color:#1e293b; flex:1;",
                        oninput: move |e| {
                            *search_query.write() = e.value();
                            *current_page.write() = 0;
                        },
                    }
                    if !search_query.read().is_empty() {
                        span {
                            style: "color:#94a3b8; font-size:11px; white-space:nowrap;",
                            "{filtered.len()} results"
                        }
                    }
                }
            }

            // Loading skeleton
            if loading() {
                div {
                    style: "padding: 40px; display:flex; flex-direction:column; gap:12px;",
                    for _ in 0..5usize {
                        div {
                            style: "height:20px; background:linear-gradient(90deg,#f1f5f9 25%,#e2e8f0 50%,#f1f5f9 75%); border-radius:6px; animation:shimmer 1.5s infinite;",
                        }
                    }
                }
            } else if let Some(ref err) = error_msg() {
                div {
                    style: "padding:24px; color:#dc2626; font-size:13px;",
                    "⚠ {err}"
                }
            } else {
                // Table
                div { style: "overflow-x:auto;",
                    table {
                        style: "width:100%; border-collapse:collapse; font-size:13px;",
                        thead {
                            style: "background:linear-gradient(to right,#f8fafc,#f1f5f9); border-bottom:2px solid rgba(0,0,0,0.08);",
                            tr {
                                for cd in &col_defs {
                                    th {
                                        style: "padding:12px 16px; font-weight:700; color:#64748b; text-transform:uppercase; font-size:10px; letter-spacing:0.08em; text-align:left; white-space:nowrap;",
                                        "{cd.label}"
                                    }
                                }
                                if has_action_col {
                                    th {
                                        style: format!("padding:12px 16px; font-weight:700; color:#64748b; text-transform:uppercase; font-size:10px; letter-spacing:0.08em; text-align:left; width:{};", action_col_w),
                                        "Actions"
                                    }
                                }
                            }
                        }
                        tbody {
                            {
                                let total_cols = col_defs.len() + if has_action_col { 1 } else { 0 };
                                rsx! {
                                    if page_rows.is_empty() {
                                        tr {
                                            td {
                                                colspan: "{total_cols}",
                                                style: "padding:40px; text-align:center; color:#94a3b8; font-size:13px;",
                                                "No data found."
                                            }
                                        }
                                    }
                                }
                            }
                            for (i, row) in page_rows.iter().enumerate() {
                                {
                                    let row_bg = if i % 2 == 1 { "background-color:#f8fafc;" } else { "background-color:#ffffff;" };
                                    let row_data = row.clone();
                                    rsx! {
                                        tr {
                                            key: "{i}",
                                            style: "{row_bg} transition:background 0.1s;",
                                            for cd in &col_defs {
                                                {
                                                    let cell_val_raw = resolve_cell_value(&row_data, &cd.key);
                                                    let cell_val = val_to_css(&cell_val_raw);
                                                    rsx! {
                                                        td {
                                                            style: "padding:12px 16px; color:#1e293b; border-bottom:1px solid rgba(0,0,0,0.04); max-width:200px; overflow:hidden; text-overflow:ellipsis; white-space:nowrap;",
                                                            "{cell_val}"
                                                        }
                                                    }
                                                }
                                            }
                                            if let Some(ref action_node) = row_action_node {
                                                td {
                                                    style: "padding:8px 16px; border-bottom:1px solid rgba(0,0,0,0.04);",
                                                    TableRowActionWrapper {
                                                        action_node: action_node.clone(),
                                                        row_data: row_data.clone(),
                                                        selected_id,
                                                        on_select,
                                                        on_drop,
                                                        on_delete,
                                                        on_resize_start,
                                                        on_drag_start,
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // Pagination
                if total_pages > 1 {
                    div {
                        style: "padding:12px 16px; border-top:1px solid rgba(0,0,0,0.06); display:flex; align-items:center; justify-content:space-between; background:#f8fafc;",
                        span {
                            style: "font-size:12px; color:#64748b;",
                            "Page {current_page()+1} of {total_pages}  ·  {filtered.len()} total"
                        }
                        div {
                            style: "display:flex; gap:6px;",
                            button {
                                style: format!("padding:6px 12px; border-radius:6px; font-size:12px; font-weight:600; cursor:pointer; border:1px solid #e2e8f0; background:#fff; color:{};", if current_page() == 0 {"#cbd5e1"} else {"#475569"}),
                                disabled: current_page() == 0,
                                onclick: move |_| { if current_page() > 0 { *current_page.write() -= 1; } },
                                "← Prev"
                            }
                            button {
                                style: format!("padding:6px 12px; border-radius:6px; font-size:12px; font-weight:600; cursor:pointer; border:1px solid #e2e8f0; background:#fff; color:{};", if current_page() + 1 >= total_pages {"#cbd5e1"} else {"#475569"}),
                                disabled: current_page() + 1 >= total_pages,
                                onclick: move |_| { if current_page() + 1 < total_pages { *current_page.write() += 1; } },
                                "Next →"
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn Hooked_Tabs(
    node: ComponentNode,
    styles: String,
    selected_id: Option<Signal<Option<String>>>,
    on_select: Option<EventHandler<String>>,
    on_drop: Option<EventHandler<(String, f64, f64)>>,
    on_delete: Option<EventHandler<String>>,
    #[props(default)] on_resize_start: Option<EventHandler<(String, String, f64, f64)>>,
    #[props(default)] on_drag_start: Option<EventHandler<(String, f64, f64)>>,
) -> Element {
    // Filter out children that are Tabs/Tab
    let tabs_meta: Vec<(String, String)> = node
        .children
        .iter()
        .filter(|c| c.component_type == "Tab")
        .enumerate()
        .map(|(idx, c)| {
            let label = c
                .props
                .label
                .clone()
                .unwrap_or_else(|| format!("Tab {}", idx + 1));
            let value = c
                .props
                .value
                .clone()
                .and_then(|v| match v {
                    serde_json::Value::String(s) => Some(s),
                    other => Some(other.to_string()),
                })
                .unwrap_or_else(|| format!("tab{}", idx));
            (label, value)
        })
        .collect();

    let default_tab = {
        let raw_default = node.props.default_tab.clone().unwrap_or_default();
        if tabs_meta.iter().any(|(_, val)| val == &raw_default) {
            raw_default
        } else {
            tabs_meta
                .first()
                .map(|(_, val)| val.clone())
                .unwrap_or_else(|| "tab0".to_string())
        }
    };
    let mut active_tab = use_signal(move || default_tab.clone());

    let base_style = "display: flex; flex-direction: column; font-family: 'Outfit', sans-serif;";
    let tab_container_style = format!("{} {}", base_style, styles);

    let active_tab_node = node
        .children
        .iter()
        .filter(|c| c.component_type == "Tab")
        .find(|c| {
            let val = c
                .props
                .value
                .clone()
                .and_then(|v| match v {
                    serde_json::Value::String(s) => Some(s),
                    other => Some(other.to_string()),
                })
                .unwrap_or_default();
            val == active_tab()
        })
        .cloned()
        .or_else(|| {
            node.children
                .iter()
                .find(|c| c.component_type == "Tab")
                .cloned()
        });

    rsx! {
        div {
            id: "{node.id}",
            style: "{tab_container_style}",

            // Tab Headers
            div {
                style: "display: flex; border-bottom: 2px solid #e2e8f0; margin-bottom: 14px; gap: 8px;",
                for (label, val) in tabs_meta {
                    {
                        let is_active = active_tab() == val;
                        let active_border = if is_active { "#3b82f6" } else { "transparent" };
                        let active_text = if is_active { "#3b82f6" } else { "#64748b" };
                        let val_clone = val.clone();

                        rsx! {
                            div {
                                key: "{val}",
                                style: "padding: 10px 16px; font-weight: 600; cursor: pointer; border-bottom: 3px solid {active_border}; color: {active_text}; transition: all 0.2s; font-size: 14px; margin-bottom: -2.5px;",
                                onclick: move |_| *active_tab.write() = val_clone.clone(),
                                "{label}"
                            }
                        }
                    }
                }
            }

            // Tab Contents (Only render the child Tab that matches active value)
            if let Some(tab_node) = active_tab_node {
                div {
                    key: "{tab_node.id}",
                    style: "position: relative; min-height: 160px; box-sizing: border-box; width: 100%; animation: fadeIn 0.2s ease-out;",
                    for nested_child in tab_node.children {
                        ComponentRenderer {
                            node: nested_child,
                            selected_id,
                            on_select,
                            on_drop,
                            on_delete,
                            on_resize_start,
                            on_drag_start
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn Hooked_HierarchyTable(
    node: ComponentNode,
    styles: String,
    selected_id: Option<Signal<Option<String>>>,
    on_select: Option<EventHandler<String>>,
    on_drop: Option<EventHandler<(String, f64, f64)>>,
    on_delete: Option<EventHandler<String>>,
) -> Element {
    let columns: Vec<String> = node
        .props
        .columns
        .clone()
        .and_then(|v| serde_json::from_value(v).ok())
        .unwrap_or_else(|| vec!["Item".to_string(), "Status".to_string()]);
    let mut expanded = use_signal(|| vec![true, false]);

    let base_style = "border-collapse: collapse; text-align: left; background-color: #ffffff; border-radius: var(--radius, 12px); overflow: hidden; border: 1px solid rgba(0,0,0,0.08); font-family: 'Outfit', sans-serif; font-size: 14px; box-shadow: 0 4px 12px rgba(0,0,0,0.03);";
    let h_table_style = format!("{} {}", base_style, styles);

    let expand_icon0 = if expanded()[0] { "▼" } else { "▶" };
    let expand_icon1 = if expanded()[1] { "▼" } else { "▶" };

    rsx! {
        table {
            id: "{node.id}",
            style: "{h_table_style}",
            thead {
                tr {
                    style: "background-color: #f8fafc; border-bottom: 1px solid rgba(0,0,0,0.08);",
                    for col in &columns {
                        th { style: "padding: 12px 18px; font-weight: 700; color: #64748b; font-size: 12px;", "{col}" }
                    }
                }
            }
            tbody {
                // Level 0 Row
                tr {
                    style: "background-color: #ffffff; font-weight: 600;",
                    td {
                        style: "padding: 12px 18px; display: flex; align-items: center; gap: 8px; cursor: pointer; color: #0f172a;",
                        onclick: move |_| {
                            let mut e = expanded.read().clone();
                            e[0] = !e[0];
                            *expanded.write() = e;
                        },
                        span { "{expand_icon0}" }
                        "📦 Root Project Branch"
                    }
                    td { style: "padding: 12px 18px; color: #166534;", "OK" }
                }
                // Level 1 Child Rows
                if expanded()[0] {
                    tr {
                        style: "background-color: #f8fafc; font-size: 13px;",
                        td {
                            style: "padding: 10px 18px 10px 36px; color: #334155;",
                            "📄 index.css"
                        }
                        td { style: "padding: 10px 18px; color: #166534;", "Loaded" }
                    }
                    tr {
                        style: "background-color: #f8fafc; font-size: 13px;",
                        td {
                            style: "padding: 10px 18px 10px 36px; display: flex; align-items: center; gap: 8px; cursor: pointer; color: #334155;",
                            onclick: move |_| {
                                let mut e = expanded.read().clone();
                                e[1] = !e[1];
                                *expanded.write() = e;
                            },
                            span { "{expand_icon1}" }
                            "📁 modules/"
                        }
                        td { style: "padding: 10px 18px; color: #075985;", "Active" }
                    }
                }
                // Level 2 Child Rows
                if expanded()[0] && expanded()[1] {
                    tr {
                        style: "background-color: #f1f5f9; font-size: 12px; color: #475569;",
                        td {
                            style: "padding: 8px 18px 8px 56px;",
                            "⚙️ core.rs"
                        }
                        td { style: "padding: 8px 18px; color: #166534;", "Synced" }
                    }
                }
            }
        }
    }
}

#[component]
fn Hooked_TimeViewer(
    node: ComponentNode,
    styles: String,
    selected_id: Option<Signal<Option<String>>>,
    on_select: Option<EventHandler<String>>,
    on_drop: Option<EventHandler<(String, f64, f64)>>,
    on_delete: Option<EventHandler<String>>,
) -> Element {
    let label = node
        .props
        .label
        .clone()
        .unwrap_or_else(|| "Time".to_string());
    let live = node.props.live.unwrap_or(true);
    let show_seconds = node.props.show_seconds.unwrap_or(false);
    let use12_hour = node.props.use12_hour.unwrap_or(true);

    let mut time_str = use_signal(move || {
        #[cfg(target_arch = "wasm32")]
        {
            let date = js_sys::Date::new_0();
            let hours = date.get_hours();
            let minutes = date.get_minutes();
            let seconds = date.get_seconds();

            let ampm = if hours >= 12 { "PM" } else { "AM" };
            let display_hours = if use12_hour {
                let h = hours % 12;
                if h == 0 {
                    12
                } else {
                    h
                }
            } else {
                hours
            };

            let min_str = format!("{:02}", minutes);
            let sec_str = if show_seconds {
                format!(":{:02}", seconds)
            } else {
                "".to_string()
            };
            let ampm_str = if use12_hour {
                format!(" {}", ampm)
            } else {
                "".to_string()
            };

            format!("{:02}:{}{}{}", display_hours, min_str, sec_str, ampm_str)
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            "12:00 PM".to_string()
        }
    });

    if live {
        use_future(move || async move {
            loop {
                gloo_timers::future::TimeoutFuture::new(1000).await;
                let time_now = {
                    #[cfg(target_arch = "wasm32")]
                    {
                        let date = js_sys::Date::new_0();
                        let hours = date.get_hours();
                        let minutes = date.get_minutes();
                        let seconds = date.get_seconds();

                        let ampm = if hours >= 12 { "PM" } else { "AM" };
                        let display_hours = if use12_hour {
                            let h = hours % 12;
                            if h == 0 {
                                12
                            } else {
                                h
                            }
                        } else {
                            hours
                        };

                        let min_str = format!("{:02}", minutes);
                        let sec_str = if show_seconds {
                            format!(":{:02}", seconds)
                        } else {
                            "".to_string()
                        };
                        let ampm_str = if use12_hour {
                            format!(" {}", ampm)
                        } else {
                            "".to_string()
                        };

                        format!("{:02}:{}{}{}", display_hours, min_str, sec_str, ampm_str)
                    }
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        "12:00 PM".to_string()
                    }
                };
                *time_str.write() = time_now;
            }
        });
    }

    let base_style = "display: flex; flex-direction: column; align-items: center; justify-content: center; background: linear-gradient(135deg, #1e293b 0%, #0f172a 100%); border-radius: var(--radius, 12px); padding: 16px; border: 1px solid rgba(255,255,255,0.05); font-family: 'Outfit', sans-serif; box-shadow: 0 10px 25px rgba(15, 23, 42, 0.15); color: #ffffff;";
    let time_viewer_style = format!("{} {}", base_style, styles);

    rsx! {
        div {
            id: "{node.id}",
            style: "{time_viewer_style}",
            span {
                style: "font-size: 11px; text-transform: uppercase; letter-spacing: 0.1em; color: #94a3b8; margin-bottom: 4px; font-weight: 600;",
                "{label}"
            }
            span {
                style: "font-size: 24px; font-weight: 700; font-family: monospace; letter-spacing: 0.02em; color: #38bdf8; text-shadow: 0 0 10px rgba(56, 189, 248, 0.2);",
                "{time_str}"
            }
        }
    }
}

#[component]
fn Hooked_ImageGallery(
    node: ComponentNode,
    styles: String,
    selected_id: Option<Signal<Option<String>>>,
    on_select: Option<EventHandler<String>>,
    on_drop: Option<EventHandler<(String, f64, f64)>>,
    on_delete: Option<EventHandler<String>>,
) -> Element {
    let mut active_idx = use_signal(|| 0);
    let images = vec![
        "https://picsum.photos/id/10/400/250",
        "https://picsum.photos/id/20/400/250",
        "https://picsum.photos/id/30/400/250",
        "https://picsum.photos/id/40/400/250",
    ];

    let base_style = "background-color: #ffffff; border-radius: 12px; padding: 16px; border: 1px solid rgba(0,0,0,0.08); font-family: 'Outfit', sans-serif; display: flex; flex-direction: column; gap: 12px; width: 340px; box-shadow: 0 4px 15px rgba(0,0,0,0.03);";
    let gallery_style = format!("{} {}", base_style, styles);

    let active_src = images[active_idx()];

    rsx! {
        div {
            id: "{node.id}",
            style: "{gallery_style}",
            // Large Preview
            img {
                src: "{active_src}",
                style: "width: 100%; height: 180px; border-radius: 8px; object-fit: cover; border: 1px solid rgba(0,0,0,0.05); transition: opacity 0.2s;",
            }
            // Thumbnail Grid
            div {
                style: "display: flex; gap: 8px;",
                for (i, img) in images.iter().enumerate() {
                    {
                        let is_active = i == active_idx();
                        let border_col = if is_active { "#3b82f6" } else { "transparent" };
                        let transform_style = if is_active { "scale(1.05)" } else { "none" };
                        rsx! {
                            img {
                                key: "{img}",
                                src: "{img}",
                                style: "width: 60px; height: 40px; border-radius: 4px; object-fit: cover; cursor: pointer; border: 2px solid {border_col}; transform: {transform_style}; transition: all 0.2s;",
                                onclick: move |_| *active_idx.write() = i,
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn Hooked_ImageWithAnnotations(
    node: ComponentNode,
    styles: String,
    selected_id: Option<Signal<Option<String>>>,
    on_select: Option<EventHandler<String>>,
    on_drop: Option<EventHandler<(String, f64, f64)>>,
    on_delete: Option<EventHandler<String>>,
) -> Element {
    let mut active_annotation = use_signal(|| None::<usize>);
    let img_src = "https://picsum.photos/id/29/400/250";

    let base_style = "position: relative; border-radius: 12px; overflow: hidden; max-width: 100%; display: block; border: 1px solid rgba(0,0,0,0.08); font-family: 'Outfit', sans-serif;";
    let container_style = format!("{} {}", base_style, styles);

    // Mock Hotspots
    let hotspots = vec![
        (20.0, 30.0, "Camera Lens Sensor"),
        (70.0, 50.0, "Focus Control Dial"),
    ];

    rsx! {
        div {
            id: "{node.id}",
            style: "{container_style}",
            img {
                src: "{img_src}",
                style: "width: 100%; height: auto; display: block;",
            }

            // Render Hotspots overlay
            for (idx, (y, x, text)) in hotspots.iter().enumerate() {
                {
                    let is_active = active_annotation() == Some(idx);
                    let bg_color = if is_active { "#ef4444" } else { "#3b82f6" };
                    let border_style = if is_active { "0 0 0 4px rgba(239, 68, 68, 0.2)" } else { "0 0 0 4px rgba(59, 130, 246, 0.2)" };

                    rsx! {
                        div {
                            key: "{idx}",
                            style: "position: absolute; top: {y}%; left: {x}%; width: 14px; height: 14px; border-radius: 50%; background-color: {bg_color}; cursor: pointer; box-shadow: {border_style}; display: flex; align-items: center; justify-content: center; transform: translate(-50%, -50%); transition: all 0.2s; z-index: 10;",
                            onclick: move |_| {
                                if active_annotation() == Some(idx) {
                                    *active_annotation.write() = None;
                                } else {
                                    *active_annotation.write() = Some(idx);
                                }
                            },

                            if is_active {
                                div {
                                    style: "position: absolute; bottom: 22px; left: 50%; transform: translateX(-50%); background-color: #0f172a; color: #ffffff; padding: 4px 8px; border-radius: 4px; font-size: 10px; white-space: nowrap; box-shadow: 0 4px 6px rgba(0,0,0,0.1);",
                                    "{text}"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn Hooked_PlaylistPlayer(
    node: ComponentNode,
    styles: String,
    selected_id: Option<Signal<Option<String>>>,
    on_select: Option<EventHandler<String>>,
    on_drop: Option<EventHandler<(String, f64, f64)>>,
    on_delete: Option<EventHandler<String>>,
) -> Element {
    let tracks = vec![
        "Symphony No. 5 in C Minor",
        "Clair de Lune",
        "Canon in D Major",
    ];
    let mut track_idx = use_signal(|| 0);

    let base_style = "background-color: #ffffff; border-radius: 12px; padding: 16px; border: 1px solid rgba(0,0,0,0.08); font-family: 'Outfit', sans-serif; display: flex; flex-direction: column; gap: 10px; width: 280px; box-shadow: 0 4px 10px rgba(0,0,0,0.02);";
    let list_style = format!("{} {}", base_style, styles);

    let current_track = &tracks[track_idx()];

    rsx! {
        div {
            id: "{node.id}",
            style: "{list_style}",
            div {
                style: "background-color: #f8fafc; padding: 12px; border-radius: 8px; border: 1px solid #f1f5f9; display: flex; align-items: center; gap: 10px;",
                span { style: "font-size: 20px;", "🎵" }
                div {
                    style: "display: flex; flex-direction: column; min-width: 0;",
                    span { style: "font-size: 13px; font-weight: 700; color: #0f172a; text-overflow: ellipsis; overflow: hidden; white-space: nowrap;", "{current_track}" }
                    span { style: "font-size: 11px; color: #94a3b8; font-weight: 500;", "Classic Collection" }
                }
            }
            div {
                style: "display: flex; flex-direction: column; gap: 4px;",
                for (i, t) in tracks.iter().enumerate() {
                    {
                        let is_active = i == track_idx();
                        let bg_c = if is_active { "#f1f5f9" } else { "transparent" };
                        let text_c = if is_active { "#3b82f6" } else { "#475569" };
                        rsx! {
                            div {
                                key: "{t}",
                                style: "padding: 8px 10px; border-radius: 6px; cursor: pointer; font-size: 12px; font-weight: 600; background-color: {bg_c}; color: {text_c}; display: flex; justify-content: space-between;",
                                onclick: move |_| *track_idx.write() = i,
                                span { "{t}" }
                                if is_active { span { "Playing" } }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn Hooked_KanbanBoard(
    node: ComponentNode,
    styles: String,
    selected_id: Option<Signal<Option<String>>>,
    on_select: Option<EventHandler<String>>,
    on_drop: Option<EventHandler<(String, f64, f64)>>,
    on_delete: Option<EventHandler<String>>,
) -> Element {
    let mut todo = use_signal(|| {
        vec![
            "Draft Layout Specs".to_string(),
            "Verify SVG Assets".to_string(),
        ]
    });
    let mut prog = use_signal(|| vec!["Build Dioxus Models".to_string()]);
    let mut done = use_signal(|| vec!["Setup Dev Env".to_string()]);

    let base_style =
        "display: flex; gap: 14px; font-family: 'Outfit', sans-serif; box-sizing: border-box;";
    let board_style = format!("{} {}", base_style, styles);

    rsx! {
        div {
            id: "{node.id}",
            style: "{board_style}",

            // Column Todo
            div {
                style: "flex: 1; background-color: #f8fafc; border-radius: 12px; padding: 12px; border: 1px solid #e2e8f0; display: flex; flex-direction: column; gap: 10px; min-width: 140px;",
                span { style: "font-weight: 700; color: #475569; font-size: 13px; text-transform: uppercase;", "📌 To Do ({todo().len()})" }
                for item in todo() {
                    div {
                        key: "{item}",
                        style: "background-color: #ffffff; padding: 10px 12px; border-radius: 8px; border: 1px solid rgba(0,0,0,0.05); font-size: 12px; color: #1e293b; font-weight: 500; cursor: pointer; box-shadow: 0 2px 4px rgba(0,0,0,0.01);",
                        onclick: move |_| {
                            let mut t = todo.read().clone();
                            t.retain(|x| x != &item);
                            *todo.write() = t;
                            prog.write().push(item.clone());
                        },
                        "{item}"
                    }
                }
            }

            // Column In Progress
            div {
                style: "flex: 1; background-color: #f8fafc; border-radius: 12px; padding: 12px; border: 1px solid #e2e8f0; display: flex; flex-direction: column; gap: 10px; min-width: 140px;",
                span { style: "font-weight: 700; color: #3b82f6; font-size: 13px; text-transform: uppercase;", "⚡ Progress ({prog().len()})" }
                for item in prog() {
                    div {
                        key: "{item}",
                        style: "background-color: #ffffff; padding: 10px 12px; border-radius: 8px; border: 1px solid rgba(59,130,246,0.1); font-size: 12px; color: #1e293b; font-weight: 500; cursor: pointer; box-shadow: 0 2px 4px rgba(0,0,0,0.01);",
                        onclick: move |_| {
                            let mut p = prog.read().clone();
                            p.retain(|x| x != &item);
                            *prog.write() = p;
                            done.write().push(item.clone());
                        },
                        "{item}"
                    }
                }
            }

            // Column Completed
            div {
                style: "flex: 1; background-color: #f8fafc; border-radius: 12px; padding: 12px; border: 1px solid #e2e8f0; display: flex; flex-direction: column; gap: 10px; min-width: 140px;",
                span { style: "font-weight: 700; color: #10b981; font-size: 13px; text-transform: uppercase;", "✅ Done ({done().len()})" }
                for item in done() {
                    div {
                        key: "{item}",
                        style: "background-color: #ffffff; padding: 10px 12px; border-radius: 8px; border: 1px solid rgba(16,185,129,0.08); font-size: 12px; color: #94a3b8; text-decoration: line-through; font-weight: 500; cursor: pointer; box-shadow: 0 2px 4px rgba(0,0,0,0.01);",
                        onclick: move |_| {
                            let mut d = done.read().clone();
                            d.retain(|x| x != &item);
                            *done.write() = d;
                            todo.write().push(item.clone());
                        },
                        "{item}"
                    }
                }
            }
        }
    }
}

#[component]
fn Hooked_ChatViewer(
    node: ComponentNode,
    styles: String,
    selected_id: Option<Signal<Option<String>>>,
    on_select: Option<EventHandler<String>>,
    on_drop: Option<EventHandler<(String, f64, f64)>>,
    on_delete: Option<EventHandler<String>>,
) -> Element {
    let mut messages = use_signal(|| {
        vec![
            (
                "Sarah".to_string(),
                "Hi there! Can we test the component state?".to_string(),
                false,
            ),
            (
                "Me".to_string(),
                "Absolutely, Dioxus WebAssembly captures updates immediately!".to_string(),
                true,
            ),
        ]
    });
    let mut text = use_signal(|| "".to_string());

    let base_style = "background-color: #ffffff; border-radius: 12px; padding: 14px; border: 1px solid rgba(0,0,0,0.08); font-family: 'Outfit', sans-serif; display: flex; flex-direction: column; width: 320px; height: 260px; box-shadow: 0 10px 25px rgba(0,0,0,0.03);";
    let chat_style = format!("{} {}", base_style, styles);

    rsx! {
        div {
            id: "{node.id}",
            style: "{chat_style}",
            // Message History Box
            div {
                style: "flex-grow: 1; overflow-y: auto; display: flex; flex-direction: column; gap: 10px; padding: 6px 2px;",
                for (user, msg, is_me) in messages() {
                    {
                        let self_align = if is_me { "align-self: flex-end; background-color: #3b82f6; color: #ffffff;" } else { "align-self: flex-start; background-color: #f1f5f9; color: #1e293b;" };
                        rsx! {
                            div {
                                key: "{msg}",
                                style: "padding: 8px 12px; border-radius: 10px; max-width: 80%; font-size: 12.5px; line-height: 1.5; font-weight: 500; {self_align}",
                                span { style: "font-weight: 700; font-size: 10px; opacity: 0.8; display: block; margin-bottom: 2px;", "{user}" }
                                span { "{msg}" }
                            }
                        }
                    }
                }
            }

            // Input Box
            div {
                style: "display: flex; gap: 6px; border-top: 1px solid #f1f5f9; padding-top: 8px; margin-top: 4px;",
                input {
                    placeholder: "Type message...",
                    value: "{text}",
                    style: "flex-grow: 1; padding: 6px 10px; border-radius: 6px; border: 1px solid #cbd5e1; outline: none; font-size: 13px;",
                    oninput: move |evt| *text.write() = evt.value(),
                    onkeydown: move |evt| {
                        if evt.key().to_string() == "Enter" {
                            let content = text.read().trim().to_string();
                            if !content.is_empty() {
                                messages.write().push(("Me".to_string(), content, true));
                                *text.write() = "".to_string();
                            }
                        }
                    }
                }
                button {
                    style: "padding: 6px 12px; background-color: #3b82f6; color: #ffffff; border: none; border-radius: 6px; font-weight: 600; cursor: pointer; font-size: 12px;",
                    onclick: move |_| {
                        let content = text.read().trim().to_string();
                        if !content.is_empty() {
                            messages.write().push(("Me".to_string(), content, true));
                            *text.write() = "".to_string();
                        }
                    },
                    "Send"
                }
            }
        }
    }
}

#[component]
fn Hooked_CommentSection(
    node: ComponentNode,
    styles: String,
    selected_id: Option<Signal<Option<String>>>,
    on_select: Option<EventHandler<String>>,
    on_drop: Option<EventHandler<(String, f64, f64)>>,
    on_delete: Option<EventHandler<String>>,
) -> Element {
    let mut comments = use_signal(|| {
        vec![(
            "Sarah Jones".to_string(),
            "Visual canvas scaling works cleanly!".to_string(),
        )]
    });
    let mut text = use_signal(|| "".to_string());

    let base_style = "background-color: #ffffff; border-radius: 12px; padding: 16px; border: 1px solid rgba(0,0,0,0.08); font-family: 'Outfit', sans-serif; display: flex; flex-direction: column; gap: 12px; width: 320px; box-shadow: 0 4px 10px rgba(0,0,0,0.02);";
    let c_style = format!("{} {}", base_style, styles);

    rsx! {
        div {
            id: "{node.id}",
            style: "{c_style}",
            span { style: "font-weight: 700; color: #0f172a; font-size: 14px; border-bottom: 1px solid #f1f5f9; padding-bottom: 8px;", "Comments ({comments().len()})" }
            div {
                style: "display: flex; flex-direction: column; gap: 10px; max-height: 140px; overflow-y: auto;",
                for (user, msg) in comments() {
                    div {
                        key: "{msg}",
                        style: "background-color: #f8fafc; padding: 8px 10px; border-radius: 8px; border: 1px solid #f1f5f9; font-size: 12px;",
                        span { style: "font-weight: 700; color: #0f172a; display: block; margin-bottom: 2px;", "{user}" }
                        span { style: "color: #475569;", "{msg}" }
                    }
                }
            }
            div {
                style: "display: flex; gap: 6px; border-top: 1px solid #f1f5f9; padding-top: 10px;",
                input {
                    placeholder: "Add comment...",
                    value: "{text}",
                    style: "flex-grow: 1; padding: 6px 8px; border-radius: 6px; border: 1px solid #cbd5e1; font-size: 12px; outline: none;",
                    oninput: move |evt| *text.write() = evt.value(),
                }
                button {
                    style: "padding: 6px 10px; background-color: #0f172a; color: #ffffff; border: none; border-radius: 6px; cursor: pointer; font-size: 11px; font-weight: 600;",
                    onclick: move |_| {
                        let c = text.read().trim().to_string();
                        if !c.is_empty() {
                            comments.write().push(("Me".to_string(), c));
                            *text.write() = "".to_string();
                        }
                    },
                    "Post"
                }
            }
        }
    }
}

#[component]
fn Hooked_StarRating(
    node: ComponentNode,
    styles: String,
    selected_id: Option<Signal<Option<String>>>,
    on_select: Option<EventHandler<String>>,
    on_drop: Option<EventHandler<(String, f64, f64)>>,
    on_delete: Option<EventHandler<String>>,
) -> Element {
    let mut rating = use_signal(|| 3);

    let base_style =
        "display: flex; flex-direction: column; gap: 4px; font-family: 'Outfit', sans-serif;";
    let star_style = format!("{} {}", base_style, styles);

    rsx! {
        div {
            id: "{node.id}",
            style: "{star_style}",
            span { style: "font-weight: 600; color: #475569; font-size: 13px;", "User Rating" }
            div {
                style: "display: flex; gap: 4px;",
                for i in 1..=5 {
                    {
                        let is_active = i <= rating();
                        let star_color = if is_active { "#f59e0b" } else { "#cbd5e1" };
                        rsx! {
                            span {
                                key: "{i}",
                                style: "font-size: 22px; cursor: pointer; color: {star_color}; transition: color 0.15s;",
                                onclick: move |_| *rating.write() = i,
                                "★"
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn Hooked_FolderUpload(
    node: ComponentNode,
    styles: String,
    selected_id: Option<Signal<Option<String>>>,
    on_select: Option<EventHandler<String>>,
    on_drop: Option<EventHandler<(String, f64, f64)>>,
    on_delete: Option<EventHandler<String>>,
) -> Element {
    let is_folder = node.component_type == "FolderUpload";
    let label = if is_folder {
        "Folder Upload"
    } else {
        "File Upload"
    };
    let mut uploaded = use_signal(|| false);

    let base_style =
        "display: flex; flex-direction: column; gap: 6px; font-family: 'Outfit', sans-serif;";
    let upload_style = format!("{} {}", base_style, styles);

    rsx! {
        div {
            id: "{node.id}",
            style: "{upload_style}",
            span { style: "font-weight: 600; color: #475569; font-size: 14px;", "{label}" }
            div {
                style: "height: 90px; border: 2px dashed #cbd5e1; border-radius: 8px; background-color: #f8fafc; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 4px; cursor: pointer;",
                onclick: move |_| *uploaded.write() = true,

                if uploaded() {
                    span { style: "font-size: 12px; color: #166534; font-weight: 600;", "✓ Asset Uploaded Successfully" }
                } else {
                    span { style: "font-size: 20px;", "📤" }
                    span { style: "font-size: 11px; color: #94a3b8;", "Drag & drop files here" }
                }
            }
        }
    }
}

#[component]
fn Hooked_Accordion(
    node: ComponentNode,
    styles: String,
    selected_id: Option<Signal<Option<String>>>,
    on_select: Option<EventHandler<String>>,
    on_drop: Option<EventHandler<(String, f64, f64)>>,
    on_delete: Option<EventHandler<String>>,
) -> Element {
    let items: Vec<crate::models::AccordionItem> = node
        .props
        .data
        .clone()
        .and_then(|d| serde_json::from_value(d).ok())
        .unwrap_or_else(|| {
            vec![
                crate::models::AccordionItem {
                    title: Some("Accordion Section 1".to_string()),
                    content: Some(
                        "Provide high quality custom properties directly in Rust structs."
                            .to_string(),
                    ),
                },
                crate::models::AccordionItem {
                    title: Some("Accordion Section 2".to_string()),
                    content: Some(
                        "Dioxus compiles recursively into clean reactive virtual DOM elements."
                            .to_string(),
                    ),
                },
            ]
        });

    let mut open_indices = use_signal(|| {
        let mut initial = vec![];
        if !items.is_empty() {
            initial.push(0); // Open first item by default
        }
        initial
    });

    let allow_multiple = node.props.allow_multiple.unwrap_or(false);
    let base_style = "background-color: #ffffff; border-radius: var(--radius, 12px); overflow: hidden; border: 1px solid rgba(0,0,0,0.08); font-family: 'Outfit', sans-serif; box-shadow: 0 4px 15px rgba(0,0,0,0.03);";
    let accordion_style = format!("{} {}", base_style, styles);

    rsx! {
        div {
            id: "{node.id}",
            style: "{accordion_style}",
            for (i, item) in items.iter().enumerate() {
                {
                    let is_open = open_indices.read().contains(&i);
                    let title = item.title.clone().unwrap_or_else(|| format!("Section {}", i + 1));
                    let content = item.content.clone().unwrap_or_default();

                    let header_bg = if is_open { "#f8fafc" } else { "#ffffff" };
                    let chevron_transform = if is_open { "rotate(180deg)" } else { "rotate(0)" };

                    rsx! {
                        div {
                            key: "{i}",
                            style: "border-bottom: 1px solid rgba(0,0,0,0.06);",

                            // Accordion Header
                            div {
                                style: "padding: 16px 20px; display: flex; justify-content: space-between; align-items: center; cursor: pointer; user-select: none; background-color: {header_bg}; transition: background-color 0.2s;",
                                onclick: move |_| {
                                    let mut indices = open_indices.read().clone();
                                    if indices.contains(&i) {
                                        indices.retain(|&idx| idx != i);
                                    } else {
                                        if !allow_multiple {
                                            indices.clear();
                                        }
                                        indices.push(i);
                                    }
                                    *open_indices.write() = indices;
                                },
                                span {
                                    style: "font-weight: 600; color: #1e293b;",
                                    "{title}"
                                }
                                span {
                                    style: "font-size: 12px; transition: transform 0.2s; transform: {chevron_transform}; color: #64748b;",
                                    "▼"
                                }
                            }

                            // Accordion Content
                            if is_open {
                                div {
                                    style: "padding: 16px 20px; background-color: #ffffff; color: #475569; font-size: 14px; line-height: 1.6; border-top: 1px solid rgba(0,0,0,0.03); animation: slideDown 0.2s ease-out;",
                                    "{content}"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn Hooked_WizardStepper(
    node: ComponentNode,
    styles: String,
    selected_id: Option<Signal<Option<String>>>,
    on_select: Option<EventHandler<String>>,
    on_drop: Option<EventHandler<(String, f64, f64)>>,
    on_delete: Option<EventHandler<String>>,
) -> Element {
    let steps = node.props.steps_list.clone().unwrap_or_else(|| {
        vec![
            "Step 1".to_string(),
            "Step 2".to_string(),
            "Step 3".to_string(),
        ]
    });
    let mut active_step = use_signal(|| 0);

    let base_style = "background-color: #ffffff; border-radius: var(--radius, 12px); border: 1px solid rgba(0,0,0,0.08); padding: 20px; font-family: 'Outfit', sans-serif; box-shadow: 0 4px 15px rgba(0,0,0,0.03);";
    let stepper_style = format!("{} {}", base_style, styles);

    let pct = ((active_step() as f64) / ((steps.len() - 1).max(1) as f64)) * 100.0;
    let width_style = format!("width: {}%;", pct);

    rsx! {
        div {
            id: "{node.id}",
            style: "{stepper_style}",

            // Stepper track
            div {
                style: "display: flex; align-items: center; justify-content: space-between; position: relative; width: 100%;",

                // Connector background line
                div {
                    style: "position: absolute; top: 15px; left: 0; right: 0; height: 3px; background-color: #e2e8f0; z-index: 1;",
                }
                // Completed active connector line
                div {
                    style: "position: absolute; top: 15px; left: 0; height: 3px; background-color: #3b82f6; z-index: 1; transition: width 0.3s ease; {width_style}",
                }

                for (i, step_name) in steps.iter().enumerate() {
                    {
                        let is_completed = i < active_step();
                        let is_active = i == active_step();

                        let circle_style = if is_completed {
                            "background-color: #3b82f6; border-color: #3b82f6; color: #ffffff;"
                        } else if is_active {
                            "background-color: #ffffff; border-color: #3b82f6; color: #3b82f6; box-shadow: 0 0 0 4px rgba(59, 130, 246, 0.15);"
                        } else {
                            "background-color: #ffffff; border-color: #cbd5e1; color: #64748b;"
                        };

                        let label_weight = if is_active { "600" } else { "500" };
                        let label_color = if is_active { "#1e293b" } else { "#64748b" };

                        rsx! {
                            div {
                                key: "{i}",
                                style: "display: flex; flex-direction: column; align-items: center; position: relative; z-index: 2; cursor: pointer;",
                                onclick: move |_| {
                                    *active_step.write() = i;
                                },

                                // Number Circle
                                div {
                                    style: "width: 32px; height: 32px; border-radius: 50%; border: 2px solid; display: flex; align-items: center; justify-content: center; font-weight: 600; font-size: 14px; transition: all 0.3s; {circle_style}",
                                    if is_completed {
                                        "✓"
                                    } else {
                                        "{i + 1}"
                                    }
                                }

                                // Step label
                                span {
                                    style: "margin-top: 8px; font-size: 12px; font-weight: {label_weight}; color: {label_color}; text-align: center; white-space: nowrap;",
                                    "{step_name}"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn Hooked_TagInput(
    node: ComponentNode,
    styles: String,
    selected_id: Option<Signal<Option<String>>>,
    on_select: Option<EventHandler<String>>,
    on_drop: Option<EventHandler<(String, f64, f64)>>,
    on_delete: Option<EventHandler<String>>,
) -> Element {
    let label = node
        .props
        .label
        .clone()
        .unwrap_or_else(|| "Tags".to_string());
    let placeholder = node
        .props
        .placeholder
        .clone()
        .unwrap_or_else(|| "Add tag...".to_string());
    let max_tags = node.props.max_tags.unwrap_or(10);

    let mut tags = use_signal(|| vec!["React".to_string(), "Rust".to_string(), "WASM".to_string()]);
    let mut current_input = use_signal(|| "".to_string());

    let base_style = "display: flex; flex-direction: column; gap: 6px; font-family: 'Outfit', sans-serif; font-size: 14px;";
    let tag_input_style = format!("{} {}", base_style, styles);

    let tags_len = tags().len();

    rsx! {
        div {
            id: "{node.id}",
            style: "{tag_input_style}",
            label {
                style: "font-weight: 600; color: #475569;",
                "{label} ({tags_len}/{max_tags})"
            }
            div {
                style: "display: flex; flex-wrap: wrap; gap: 6px; padding: 8px 10px; border-radius: var(--radius, 10px); border: 1px solid rgba(0,0,0,0.08); background-color: #ffffff; min-height: 44px; align-items: center; box-sizing: border-box;",

                // Render tags
                for (idx, tag) in tags().iter().enumerate() {
                    div {
                        key: "{idx}",
                        style: "display: inline-flex; align-items: center; gap: 4px; background-color: #f1f5f9; color: #334155; padding: 4px 8px; border-radius: 6px; font-size: 12px; font-weight: 500; border: 1px solid #e2e8f0;",
                        span { "{tag}" }
                        span {
                            style: "cursor: pointer; font-size: 11px; font-weight: bold; color: #94a3b8; transition: color 0.15s; margin-left: 2px;",
                            onclick: move |_| {
                                let mut t = tags.read().clone();
                                t.remove(idx);
                                *tags.write() = t;
                            },
                            "×"
                        }
                    }
                }

                // Text input
                if tags().len() < max_tags {
                    input {
                        placeholder: "{placeholder}",
                        style: "border: none; outline: none; padding: 4px; flex-grow: 1; color: #1e293b; font-family: inherit; font-size: 14px; min-width: 80px;",
                        value: "{current_input}",
                        oninput: move |evt| {
                            *current_input.write() = evt.value();
                        },
                        onkeydown: move |evt| {
                            let key_str = evt.key().to_string();
                            if key_str == "Enter" {
                                let val = current_input.read().trim().to_string();
                                if !val.is_empty() && !tags.read().contains(&val) {
                                    let mut t = tags.read().clone();
                                    t.push(val);
                                    *tags.write() = t;
                                    *current_input.write() = "".to_string();
                                }
                            } else if key_str == "Backspace" && current_input.read().is_empty() && !tags.read().is_empty() {
                                let mut t = tags.read().clone();
                                t.pop();
                                *tags.write() = t;
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn Hooked_OtpInput(
    node: ComponentNode,
    styles: String,
    selected_id: Option<Signal<Option<String>>>,
    on_select: Option<EventHandler<String>>,
    on_drop: Option<EventHandler<(String, f64, f64)>>,
    on_delete: Option<EventHandler<String>>,
) -> Element {
    let label = node
        .props
        .label
        .clone()
        .unwrap_or_else(|| "OTP Verification".to_string());
    let length = node.props.length.unwrap_or(6);

    let mut otp_values = use_signal(move || vec!["".to_string(); length]);

    let base_style = "display: flex; flex-direction: column; gap: 8px; font-family: 'Outfit', sans-serif; align-items: flex-start;";
    let otp_style = format!("{} {}", base_style, styles);

    let node_id = node.id.clone();

    rsx! {
        div {
            id: "{node.id}",
            style: "{otp_style}",
            label {
                style: "font-weight: 600; color: #475569; font-size: 14px;",
                "{label}"
            }
            div {
                style: "display: flex; gap: 8px;",
                for i in 0..length {
                    {
                        let current_val = otp_values.read()[i].clone();
                        let input_id = format!("{}_otp_{}", node_id, i);
                        let next_id = if i + 1 < length { Some(format!("{}_otp_{}", node_id, i + 1)) } else { None };
                        let prev_id = if i > 0 { Some(format!("{}_otp_{}", node_id, i - 1)) } else { None };

                        rsx! {
                            input {
                                id: "{input_id}",
                                key: "{i}",
                                r#type: "text",
                                maxlength: "1",
                                value: "{current_val}",
                                style: "width: 44px; height: 44px; text-align: center; font-size: 20px; font-weight: 700; border-radius: var(--radius, 10px); border: 1px solid rgba(0,0,0,0.08); background-color: #ffffff; color: #030213; outline: none; transition: all 0.2s; box-shadow: inset 0 1px 2px rgba(0,0,0,0.01);",
                                oninput: move |evt| {
                                    let val = evt.value();
                                    let mut current_vals = otp_values.read().clone();
                                    let last_char = val.chars().last().map(|c| c.to_string()).unwrap_or_default();
                                    current_vals[i] = last_char.clone();
                                    *otp_values.write() = current_vals;

                                    // Auto-focus next input field
                                    if !last_char.is_empty() {
                                        if let Some(ref next_id_str) = next_id {
                                            #[cfg(target_arch = "wasm32")]
                                            {
                                                let window = web_sys::window().unwrap();
                                                let document = window.document().unwrap();
                                                if let Some(next_el) = document.get_element_by_id(next_id_str) {
                                                    if let Ok(html_el) = next_el.dyn_into::<web_sys::HtmlElement>() {
                                                        let _ = html_el.focus();
                                                    }
                                                }
                                            }
                                        }
                                    }
                                },
                                onkeydown: move |evt| {
                                    let key_str = evt.key().to_string();
                                    if key_str == "Backspace" {
                                        let mut current_vals = otp_values.read().clone();
                                        if current_vals[i].is_empty() {
                                            if let Some(ref prev_id_str) = prev_id {
                                                // Auto-focus previous input field on Backspace if empty
                                                #[cfg(target_arch = "wasm32")]
                                                {
                                                    let window = web_sys::window().unwrap();
                                                    let document = window.document().unwrap();
                                                    if let Some(prev_el) = document.get_element_by_id(prev_id_str) {
                                                        if let Ok(html_el) = prev_el.dyn_into::<web_sys::HtmlElement>() {
                                                            let _ = html_el.focus();
                                                        }
                                                    }
                                                }
                                            }
                                        } else {
                                            current_vals[i] = "".to_string();
                                            *otp_values.write() = current_vals;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn Hooked_ColorPicker(
    node: ComponentNode,
    styles: String,
    selected_id: Option<Signal<Option<String>>>,
    on_select: Option<EventHandler<String>>,
    on_drop: Option<EventHandler<(String, f64, f64)>>,
    on_delete: Option<EventHandler<String>>,
) -> Element {
    let label = node
        .props
        .label
        .clone()
        .unwrap_or_else(|| "Theme Color".to_string());
    let swatches = node.props.swatches.clone().unwrap_or_else(|| {
        vec![
            "#3b82f6".to_string(),
            "#10b981".to_string(),
            "#f59e0b".to_string(),
            "#ef4444".to_string(),
            "#8b5cf6".to_string(),
        ]
    });

    let mut selected = use_signal(|| {
        swatches
            .first()
            .cloned()
            .unwrap_or_else(|| "#3b82f6".to_string())
    });

    let base_style =
        "display: flex; flex-direction: column; gap: 6px; font-family: 'Outfit', sans-serif;";
    let cp_style = format!("{} {}", base_style, styles);

    rsx! {
        div {
            id: "{node.id}",
            style: "{cp_style}",
            label {
                style: "font-weight: 600; color: #475569; font-size: 14px;",
                "{label}: "
                span {
                    style: "color: {selected}; font-weight: bold; font-family: monospace;",
                    "{selected}"
                }
            }
            div {
                style: "display: flex; align-items: center; gap: 8px; padding: 6px 0;",
                for color in swatches {
                    {
                        let is_active = selected() == color;
                        let border_color = if is_active { "#030213" } else { "transparent" };
                        let box_shadow = if is_active { "0 0 0 2px rgba(3, 2, 19, 0.15)" } else { "0 2px 4px rgba(0,0,0,0.06)" };
                        let transform = if is_active { "scale(1.15)" } else { "scale(1)" };

                        rsx! {
                            div {
                                key: "{color}",
                                style: "width: 28px; height: 28px; border-radius: 50%; background-color: {color}; cursor: pointer; transition: all 0.2s; border: 2px solid; border-color: {border_color}; box-shadow: {box_shadow}; transform: {transform};",
                                onclick: move |_| {
                                    *selected.write() = color.clone();
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn Hooked_RichTextEditor(
    node: ComponentNode,
    styles: String,
    selected_id: Option<Signal<Option<String>>>,
    on_select: Option<EventHandler<String>>,
    on_drop: Option<EventHandler<(String, f64, f64)>>,
    on_delete: Option<EventHandler<String>>,
) -> Element {
    let label = node
        .props
        .label
        .clone()
        .unwrap_or_else(|| "Description".to_string());
    let placeholder = node.props.placeholder.clone().unwrap_or_default();

    let mut content = use_signal(|| "".to_string());
    let mut bold = use_signal(|| false);
    let mut italic = use_signal(|| false);
    let mut underline = use_signal(|| false);

    let base_style =
        "display: flex; flex-direction: column; gap: 6px; font-family: 'Outfit', sans-serif;";
    let rte_style = format!("{} {}", base_style, styles);

    let bold_bg = if bold() { "#e2e8f0" } else { "transparent" };
    let italic_bg = if italic() { "#e2e8f0" } else { "transparent" };
    let underline_bg = if underline() {
        "#e2e8f0"
    } else {
        "transparent"
    };

    let text_weight = if bold() { "bold" } else { "normal" };
    let text_style = if italic() { "italic" } else { "normal" };
    let text_decor = if underline() { "underline" } else { "none" };

    rsx! {
        div {
            id: "{node.id}",
            style: "{rte_style}",
            label {
                style: "font-weight: 600; color: #475569; font-size: 14px;",
                "{label}"
            }
            div {
                style: "border-radius: var(--radius, 12px); border: 1px solid rgba(0,0,0,0.08); background-color: #ffffff; overflow: hidden; box-shadow: 0 4px 15px rgba(0,0,0,0.02); display: flex; flex-direction: column;",

                // Editor Toolbar
                div {
                    style: "display: flex; align-items: center; gap: 4px; padding: 8px 12px; background-color: #f8fafc; border-bottom: 1px solid rgba(0,0,0,0.06);",
                    button {
                        r#type: "button",
                        style: "background-color: {bold_bg}; border: none; padding: 6px 10px; font-weight: bold; border-radius: 6px; cursor: pointer; color: #334155; font-size: 12px; transition: background-color 0.15s;",
                        onclick: move |_| *bold.write() = !bold(),
                        "B"
                    }
                    button {
                        r#type: "button",
                        style: "background-color: {italic_bg}; border: none; padding: 6px 10px; font-style: italic; border-radius: 6px; cursor: pointer; color: #334155; font-size: 12px; transition: background-color 0.15s;",
                        onclick: move |_| *italic.write() = !italic(),
                        "I"
                    }
                    button {
                        r#type: "button",
                        style: "background-color: {underline_bg}; border: none; padding: 6px 10px; text-decoration: underline; border-radius: 6px; cursor: pointer; color: #334155; font-size: 12px; transition: background-color 0.15s;",
                        onclick: move |_| *underline.write() = !underline(),
                        "U"
                    }
                    div {
                        style: "height: 16px; width: 1px; background-color: #e2e8f0; margin: 0 4px;",
                    }
                    span {
                        style: "font-size: 11px; color: #64748b; font-weight: 500;",
                        "WYSIWYG Mode"
                    }
                }

                // Text Area editor content
                textarea {
                    placeholder: "{placeholder}",
                    value: "{content}",
                    oninput: move |evt| {
                        *content.write() = evt.value();
                    },
                    style: "min-height: 120px; padding: 12px 14px; border: none; outline: none; font-family: inherit; font-size: 14px; color: #1e293b; resize: vertical; line-height: 1.5; font-weight: {text_weight}; font-style: {text_style}; text-decoration: {text_decor};",
                }
            }
        }
    }
}

#[component]
fn Hooked_Switch(
    node: ComponentNode,
    styles: String,
    selected_id: Option<Signal<Option<String>>>,
    on_select: Option<EventHandler<String>>,
    on_drop: Option<EventHandler<(String, f64, f64)>>,
    on_delete: Option<EventHandler<String>>,
) -> Element {
    let label = node
        .props
        .label
        .clone()
        .unwrap_or_else(|| "Active Status".to_string());
    let mut data_state = use_context::<GlobalDataState>().0;
    let bound_bool = node
        .props
        .extra
        .get("__boundValue")
        .and_then(json_value_to_bool);
    let mut active = use_signal(move || bound_bool.unwrap_or(false));
    let is_active = bound_bool.unwrap_or_else(|| active());
    let bind_node = node.clone();
    let base_style = "display: flex; align-items: center; gap: 10px; font-family: 'Outfit', sans-serif; font-size: 14px; cursor: pointer; user-select: none;";
    let switch_style = format!("{} {}", base_style, styles);
    let switch_bg = if is_active { "#10b981" } else { "#cbd5e1" };
    let knob_transform = if is_active {
        "translateX(18px)"
    } else {
        "translateX(0px)"
    };
    rsx! {
        div {
            id: "{node.id}",
            style: "{switch_style}",
            onclick: move |_| {
                let next = !is_active;
                *active.write() = next;
                let mut data = data_state.write();
                set_node_bind_value(&bind_node, &mut data, serde_json::Value::Bool(next));
            },
            div {
                style: "position: relative; width: 38px; height: 20px; background-color: {switch_bg}; border-radius: 9999px; transition: background-color 0.2s;",
                div {
                    style: "position: absolute; top: 2px; left: 2px; width: 16px; height: 16px; background-color: #ffffff; border-radius: 50%; transition: transform 0.2s; transform: {knob_transform}; box-shadow: 0 1px 3px rgba(0,0,0,0.1);",
                }
            }
            span { style: "color: #334155; font-weight: 500;", "{label}" }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_node(component_type: &str, bind: &str) -> ComponentNode {
        let mut node = crate::models::create_default_component(component_type);
        node.props.bind = Some(bind.to_string());
        node
    }

    #[test]
    fn node_bind_display_resolves_global_values_for_non_text_components() {
        let mut node = test_node("Button", "data.profile.name");
        node.props.label = Some("Fallback label".to_string());
        let global = serde_json::json!({
            "profile": {
                "name": "Ada Lovelace"
            }
        });

        let display = resolve_node_bind_value(&node, None, &global)
            .map(|value| json_value_to_display_string(&value));

        assert_eq!(display.as_deref(), Some("Ada Lovelace"));
    }

    #[test]
    fn node_bind_display_resolves_local_item_values() {
        let node = test_node("Badge", "item.status");
        let local_item = serde_json::json!({
            "status": "Approved"
        });
        let global = serde_json::json!({});

        let display = resolve_node_bind_value(&node, Some(&local_item), &global)
            .map(|value| json_value_to_display_string(&value));

        assert_eq!(display.as_deref(), Some("Approved"));
    }

    #[test]
    fn node_bind_value_preserves_array_values_for_data_components() {
        let node = test_node("Table", "data.orders");
        let global = serde_json::json!({
            "orders": [
                { "id": 1, "total": 24 },
                { "id": 2, "total": 42 }
            ]
        });

        let value = resolve_node_bind_value(&node, None, &global).expect("bound value");
        assert_eq!(value.as_array().map(Vec::len), Some(2));
    }

    #[test]
    fn node_bind_setter_writes_to_global_data_path() {
        let node = test_node("Input", "data.form.email");
        let mut global = serde_json::json!({});

        set_node_bind_value(&node, &mut global, serde_json::json!("user@example.com"));

        assert_eq!(global["form"]["email"], "user@example.com");
    }

    #[test]
    fn bound_input_keeps_json_label_and_stores_bound_value_separately() {
        let mut node = test_node("Input", "data.form.email");
        node.props.label = Some("Email Address".to_string());

        apply_bound_value_to_node(&mut node, &serde_json::json!("user@example.com"));

        assert_eq!(node.props.label.as_deref(), Some("Email Address"));
        assert_eq!(
            node.props.extra.get("__boundValue"),
            Some(&serde_json::json!("user@example.com"))
        );
    }

    #[test]
    fn repeater_child_with_absolute_position_still_uses_flow_layout() {
        let mut node = test_node("Card", "");
        node.props.style = Some(std::collections::HashMap::from([
            ("position".to_string(), "absolute".to_string()),
            ("top".to_string(), "0px".to_string()),
            ("left".to_string(), "0px".to_string()),
        ]));

        assert!(should_render_as_flow_layout(&node, true, false));
    }

    #[test]
    fn modal_open_collects_on_load_actions_for_fetching_passed_data() {
        let mut node = test_node("Modal", "");
        node.on_load = vec![serde_json::json!({
            "type": "API_CALL",
            "payload": {
                "url": "/api/ticket/{{data.selectedTicket.id}}",
                "method": "GET",
                "targetKey": "selectedTicketDetails"
            }
        })];

        assert_eq!(modal_on_open_actions(&node), node.on_load);
    }

    #[test]
    fn modal_sizing_caps_large_freeform_content_to_viewport_scroll_area() {
        let mut modal = test_node("Modal", "");
        modal.props.style = Some(std::collections::HashMap::from([
            ("width".to_string(), "80%".to_string()),
            ("height".to_string(), "80%".to_string()),
            ("minHeight".to_string(), "200px".to_string()),
        ]));

        let mut form = test_node("Form", "");
        form.props.style = Some(std::collections::HashMap::from([
            ("position".to_string(), "relative".to_string()),
            ("height".to_string(), "954px".to_string()),
            ("width".to_string(), "100%".to_string()),
        ]));

        let mut submit = test_node("Button", "");
        submit.props.style = Some(std::collections::HashMap::from([
            ("position".to_string(), "absolute".to_string()),
            ("top".to_string(), "1183px".to_string()),
            ("left".to_string(), "2.75%".to_string()),
        ]));
        form.children = vec![submit];
        modal.children = vec![form];

        let sizing = modal_sizing_styles(&modal);

        assert!(sizing
            .dialog_style
            .contains("max-width: calc(100vw - 24px);"));
        assert!(sizing
            .dialog_style
            .contains("max-height: calc(100vh - 24px);"));
        assert!(sizing
            .dialog_style
            .contains("min-height: min(1365px, calc(100vh - 24px));"));
        assert!(sizing.body_style.contains("overflow: auto;"));
        assert!(sizing.body_style.contains("min-height: 0;"));
        assert!(sizing
            .body_style
            .contains("height: min(1255px, calc(100vh - 134px));"));
    }

    #[test]
    fn dropdown_options_use_static_json_objects_with_custom_fields() {
        let mut node = test_node("Select", "");
        node.props.extra.insert(
            "options".to_string(),
            serde_json::json!([
                { "id": "open", "name": "Open Ticket" },
                { "id": "resolved", "name": "Resolved Ticket" }
            ]),
        );
        node.props.extra.insert(
            "labelField".to_string(),
            serde_json::Value::String("name".to_string()),
        );
        node.props.extra.insert(
            "valueField".to_string(),
            serde_json::Value::String("id".to_string()),
        );

        let options = dropdown_options_for_node(&node, None, &serde_json::json!({}));

        assert_eq!(
            options,
            vec![
                DropdownOption {
                    label: "Open Ticket".to_string(),
                    value: "open".to_string()
                },
                DropdownOption {
                    label: "Resolved Ticket".to_string(),
                    value: "resolved".to_string()
                },
            ]
        );
    }

    #[test]
    fn dropdown_options_resolve_global_data_source() {
        let mut node = test_node("Select", "");
        node.props.data_source = Some("data.ticketStatuses".to_string());
        let global = serde_json::json!({
            "ticketStatuses": [
                { "label": "Open", "value": "open" },
                { "label": "Pending", "value": "pending" }
            ]
        });

        let options = dropdown_options_for_node(&node, None, &global);

        assert_eq!(
            options,
            vec![
                DropdownOption {
                    label: "Open".to_string(),
                    value: "open".to_string()
                },
                DropdownOption {
                    label: "Pending".to_string(),
                    value: "pending".to_string()
                },
            ]
        );
    }

    #[test]
    fn dropdown_load_actions_include_on_load_and_api_config() {
        let mut node = test_node("Select", "");
        node.on_load = vec![serde_json::json!({
            "type": "API_CALL",
            "payload": {
                "url": "/api/bootstrap",
                "targetKey": "bootstrap"
            }
        })];
        node.props.extra.insert(
            "apiUrl".to_string(),
            serde_json::Value::String("/api/statuses".to_string()),
        );
        node.props.extra.insert(
            "targetKey".to_string(),
            serde_json::Value::String("ticketStatuses".to_string()),
        );

        let actions = dropdown_load_actions(&node);

        assert_eq!(actions.len(), 2);
        assert_eq!(actions[0]["payload"]["targetKey"], "bootstrap");
        assert_eq!(actions[1]["type"], "API_CALL");
        assert_eq!(actions[1]["payload"]["url"], "/api/statuses");
        assert_eq!(actions[1]["payload"]["targetKey"], "ticketStatuses");
    }

    #[test]
    fn dropdown_options_support_live_options_bind_aliases() {
        let mut node = test_node("Select", "");
        node.props.extra.insert(
            "optionsBind".to_string(),
            serde_json::Value::String(
                "data.my_options.functions_get_mongo_master_output_data".to_string(),
            ),
        );
        node.props.extra.insert(
            "optionsLabelKey".to_string(),
            serde_json::Value::String("first_name".to_string()),
        );
        node.props.extra.insert(
            "optionsValueKey".to_string(),
            serde_json::Value::String("employee_id".to_string()),
        );
        let global = serde_json::json!({
            "my_options": {
                "functions_get_mongo_master_output_data": [
                    { "first_name": "Asha", "employee_id": "EMP-1" },
                    { "first_name": "Dev", "employee_id": "EMP-2" }
                ]
            }
        });

        let options = dropdown_options_for_node(&node, None, &global);

        assert_eq!(
            options,
            vec![
                DropdownOption {
                    label: "Asha".to_string(),
                    value: "EMP-1".to_string()
                },
                DropdownOption {
                    label: "Dev".to_string(),
                    value: "EMP-2".to_string()
                },
            ]
        );
    }

    #[test]
    fn action_callbacks_support_top_level_builder_shape() {
        let action = serde_json::json!({
            "type": "API_CALL",
            "payload": { "targetKey": "result" },
            "onSuccess": [{ "type": "CLOSE_MODAL", "payload": {} }],
            "onError": [{ "type": "TOAST", "payload": { "type": "error" } }]
        });

        assert_eq!(
            action_success_actions(&action),
            Some(vec![
                serde_json::json!({ "type": "CLOSE_MODAL", "payload": {} })
            ])
        );
        assert_eq!(
            action_error_actions(&action),
            Some(vec![
                serde_json::json!({ "type": "TOAST", "payload": { "type": "error" } })
            ])
        );
    }

    #[test]
    fn submit_form_action_is_removed_from_button_click_workflow() {
        let actions = vec![
            serde_json::json!({ "type": "SUBMIT_FORM", "payload": {} }),
            serde_json::json!({ "type": "TOAST", "payload": { "message": "Clicked" } }),
        ];

        assert!(button_has_submit_form_action(&actions));
        assert_eq!(
            button_click_actions(&actions),
            vec![serde_json::json!({ "type": "TOAST", "payload": { "message": "Clicked" } })]
        );
    }

    #[test]
    fn form_submit_scope_contains_form_aliases() {
        let scope = form_submit_scope(
            serde_json::json!({
                "remarks": "Assign this",
                "selected_id": "EMP-1"
            }),
            None,
        );

        assert_eq!(scope["form"]["remarks"], "Assign this");
        assert_eq!(scope["formData"]["selected_id"], "EMP-1");
        assert_eq!(scope["formValues"]["selected_id"], "EMP-1");
    }
}
