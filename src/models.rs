use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub enum LayoutMode {
    #[default]
    Absolute,
    FlexStack,
    CSSGrid,
    Section,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AnchorConstraints {
    pub anchor_top_to: Option<String>,
    pub anchor_offset_y: Option<f64>,
    pub anchor_left_to: Option<String>,
    pub anchor_offset_x: Option<f64>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ComponentNode {
    #[serde(default = "generate_unique_id")]
    pub id: String,
    #[serde(rename = "type")]
    pub component_type: String,

    pub template_key: Option<String>,
    #[serde(default)]
    pub props: NodeProps,
    #[serde(default)]
    pub children: Vec<ComponentNode>,
    #[serde(default)]
    pub is_dynamic: bool,
    pub placement: Option<Placement>,
    pub placement_desktop: Option<Placement>,
    pub placement_tablet: Option<Placement>,
    pub placement_mobile: Option<Placement>,
    pub computed_styles: Option<HashMap<String, String>>,

    // Layout engine fields
    #[serde(default)]
    pub layout_mode: LayoutMode,
    pub anchor: Option<AnchorConstraints>,
    pub size_mode: Option<String>,
    pub grid_area: Option<String>,

    // Event action arrays (open Value to match home.json shape exactly)
    #[serde(default)]
    pub on_click: Vec<serde_json::Value>,
    #[serde(default)]
    pub on_load: Vec<serde_json::Value>,
    #[serde(default)]
    pub on_change: Vec<serde_json::Value>,
    #[serde(default)]
    pub on_submit: Vec<serde_json::Value>,
}

/// Extracts a CSS string from a serde_json::Value style property.
/// Handles both `"16px"` (String) and `1` (Number → "1") gracefully.
pub fn val_to_css(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(b)   => b.to_string(),
        other                        => other.to_string(),
    }
}


#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
pub struct Placement {
    pub top: Option<String>,
    pub right: Option<String>,
    pub bottom: Option<String>,
    pub left: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NodeProps {
    // Flex Container Props
    pub direction: Option<String>,

    // Header/Text/Alert Props
    pub text: Option<String>,
    pub level: Option<u8>,
    pub content: Option<String>,
    pub variant: Option<String>,
    pub title: Option<String>,
    pub message: Option<String>,
    pub popup: Option<bool>,

    // Button & Input Props
    pub label: Option<String>,
    pub placeholder: Option<String>,
    pub bind: Option<String>,
    pub required: Option<bool>,

    // TimeViewer Props
    pub live: Option<bool>,
    pub show_seconds: Option<bool>,
    pub use12_hour: Option<bool>,
    pub time_zone: Option<String>,

    // Accordion Props
    pub data_source: Option<String>,
    pub data: Option<serde_json::Value>,
    pub title_field: Option<String>,
    pub content_field: Option<String>,
    pub allow_multiple: Option<bool>,

    // Stepper Props
    pub steps_list: Option<Vec<String>>,
    pub active_step_bind: Option<String>,
    pub linear: Option<bool>,

    // Tabs Props
    pub default_tab: Option<String>,
    pub value: Option<serde_json::Value>,
    pub content_mode: Option<String>,

    // Nav link target
    pub target: Option<String>,

    // Chart Props
    pub chart_type: Option<String>,
    pub request_body: Option<serde_json::Value>,
    pub method: Option<String>,

    // TagInput Props
    pub suggestions_bind: Option<String>,
    pub max_tags: Option<usize>,

    // OtpInput Props
    pub length: Option<usize>,

    // ColorPicker Props
    pub swatches: Option<Vec<String>>,

    // Table Props
    pub columns: Option<serde_json::Value>,
    pub rows: Option<serde_json::Value>,
    pub footer_buttons: Option<Vec<serde_json::Value>>,

    // Layout Engine Additions
    pub repeater_data: Option<String>,
    pub columns_template: Option<String>,
    pub rows_template: Option<String>,
    pub gap: Option<String>,

    // Layout Styling
    #[serde(default, deserialize_with = "deserialize_style_map")]
    pub style: Option<HashMap<String, String>>,

    // Catch-all mapping for dynamic properties
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

impl NodeProps {
    pub fn to_style_string(&self) -> String {
        self.style.as_ref().map(|styles| {
            styles.iter()
                .map(|(k, v)| format!("{}:{};", convert_camel_to_kebab(k), v))
                .collect::<Vec<String>>()
                .join("")
        }).unwrap_or_default()
    }
}

pub fn convert_camel_to_kebab(s: &str) -> String {
    let mut kebab = String::new();
    for c in s.chars() {
        if c.is_ascii_uppercase() {
            kebab.push('-');
            kebab.push(c.to_ascii_lowercase());
        } else {
            kebab.push(c);
        }
    }
    kebab
}

pub fn generate_unique_id() -> String {
    #[cfg(target_arch = "wasm32")]
    {
        let time = js_sys::Date::new_0().get_time();
        let rand = js_sys::Math::random() * 100000.0;
        format!("comp_{}_{:.0}", time, rand)
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        let id = COUNTER.fetch_add(1, Ordering::SeqCst);
        format!("comp_desktop_{}", id)
    }
}

pub fn create_default_component(comp_type: &str) -> ComponentNode {
    let mut props = NodeProps::default();
    let mut style: HashMap<String, String> = HashMap::new();

    // Helper closure to keep inserts concise
    macro_rules! s {
        ($val:expr) => { $val.to_string() };
    }


    // Setup base defaults depending on type
    match comp_type {
        "Flex" => {
            props.direction = Some("column".to_string());
            style.insert("width".to_string(),         s!("100%"));
            style.insert("minHeight".to_string(),     s!("120px"));
            style.insert("padding".to_string(),       s!("16px"));
            style.insert("border".to_string(),        s!("1px dashed #cbd5e1"));
            style.insert("backgroundColor".to_string(), s!("#f8fafc"));
            style.insert("borderRadius".to_string(),  s!("8px"));
        }
        "Card" => {
            props.title = Some("New Visual Card".to_string());
            style.insert("width".to_string(),     s!("100%"));
            style.insert("minHeight".to_string(), s!("150px"));
            style.insert("border".to_string(),    s!("1px solid #e2e8f0"));
        }
        "Container" => {
            style.insert("width".to_string(),           s!("100%"));
            style.insert("padding".to_string(),         s!("20px"));
            style.insert("backgroundColor".to_string(), s!("#ffffff"));
        }
        "Stack" => {
            props.extra.insert("direction".to_string(), serde_json::Value::String("column".to_string()));
            props.extra.insert("spacing".to_string(),   serde_json::Value::String("12px".to_string()));
            style.insert("width".to_string(), s!("100%"));
        }
        "Grid" => {
            props.extra.insert("columnsCount".to_string(), serde_json::Value::Number(3.into()));
            props.extra.insert("gap".to_string(),          serde_json::Value::String("16px".to_string()));
            style.insert("width".to_string(), s!("100%"));
        }
        "Button" => { props.label = Some("Action Button".to_string()); }
        "Link" => {
            props.text = Some("Clickable Link".to_string());
            props.extra.insert("href".to_string(), serde_json::Value::String("#".to_string()));
        }
        "Heading" => { props.text = Some("Section Heading".to_string()); props.level = Some(2); }
        "Text" => { props.content = Some("Editable block text...".to_string()); props.variant = Some("p".to_string()); }
        "Alert" => {
            props.variant = Some("info".to_string());
            props.title = Some("Notice".to_string());
            props.message = Some("Default notification content.".to_string());
        }
        "Badge" => { props.label = Some("Status Tag".to_string()); props.variant = Some("info".to_string()); }
        "Input" => { props.label = Some("Input Field".to_string()); props.placeholder = Some("Type content...".to_string()); }
        "Textarea" => { props.label = Some("Long Textarea".to_string()); props.placeholder = Some("Write details...".to_string()); }
        "Checkbox" => { props.label = Some("Verify checklist item".to_string()); }
        "Toggle" | "Switch" => { props.label = Some("Toggle Switch".to_string()); }
        "Select" => { props.label = Some("Select Option".to_string()); }
        "DatePicker" => { props.label = Some("Choose Date".to_string()); }
        "TimePicker" => { props.label = Some("Choose Time".to_string()); }
        "Iframe" => { props.extra.insert("src".to_string(), serde_json::Value::String("https://dioxuslabs.com".to_string())); }
        "YearCalendar" => { style.insert("width".to_string(), s!("100%")); }
        "TimeViewer" => { props.label = Some("Live Clock".to_string()); props.live = Some(true); }
        "StarRating" => { style.insert("padding".to_string(), s!("8px")); }
        _ => {}
    }

    props.style = Some(style);

    ComponentNode {
        id: generate_unique_id(),
        component_type: comp_type.to_string(),
        template_key: None,
        props,
        children: Vec::new(),
        is_dynamic: false,
        placement: None,
        placement_desktop: None,
        placement_tablet: None,
        placement_mobile: None,
        computed_styles: None,
        layout_mode: LayoutMode::Absolute,
        anchor: None,
        size_mode: None,
        grid_area: None,
        on_click: vec![],
        on_load: vec![],
        on_change: vec![],
        on_submit: vec![],
    }
}


pub fn is_container(comp_type: &str) -> bool {
    matches!(comp_type, "Flex" | "Card" | "Layout" | "Container" | "Stack" | "Grid" | "Tabs" | "Tab" | "Form" | "DynamicCardGrid")
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
pub struct AccordionItem {
    pub title: Option<String>,
    pub content: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
pub struct ChartItem {
    pub name: Option<String>,
    pub value: Option<f64>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Page {
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub path: String,
    #[serde(default)]
    pub route_path: Option<String>,
    pub root: ComponentNode,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RouteRecord {
    #[serde(default)]
    pub path: String,
    #[serde(default)]
    pub page_id: String,
    #[serde(default)]
    pub page: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RoutesIndex {
    #[serde(default)]
    pub routes: Vec<RouteRecord>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CompiledPackage {
    #[serde(default)]
    pub routes: Option<RoutesIndex>,
    #[serde(default)]
    pub pages: HashMap<String, Page>,
}

pub fn deserialize_style_map<'de, D>(deserializer: D) -> Result<Option<HashMap<String, String>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    struct StyleMapVisitor;

    impl<'de> serde::de::Visitor<'de> for StyleMapVisitor {
        type Value = Option<HashMap<String, String>>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a map containing string keys and string, number, or boolean values")
        }

        fn visit_none<E>(self) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(None)
        }

        fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            deserializer.deserialize_map(self)
        }

        fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
        where
            A: serde::de::MapAccess<'de>,
        {
            let mut result = HashMap::new();
            while let Some((key, value)) = map.next_entry::<String, serde_json::Value>()? {
                let str_val = match value {
                    serde_json::Value::String(s) => s,
                    serde_json::Value::Number(n) => n.to_string(),
                    serde_json::Value::Bool(b)   => b.to_string(),
                    other                        => other.to_string(),
                };
                result.insert(key, str_val);
            }
            Ok(Some(result))
        }
    }

    deserializer.deserialize_option(StyleMapVisitor)
}

