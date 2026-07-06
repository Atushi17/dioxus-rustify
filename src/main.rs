#![allow(non_snake_case)]

mod models;
mod renderer;

use dioxus::prelude::*;
use models::ComponentNode;
use renderer::ComponentRenderer;


#[derive(Clone, Debug, PartialEq, Default)]
struct ShellPackage {
  default_page: Option<String>,
  hide_shell_on_pages: Vec<String>,
  hide_shell_on_routes: Vec<String>,
  on_load: Vec<serde_json::Value>,
  sidebar_children: Vec<ComponentNode>,
  topbar_children: Vec<ComponentNode>,
  footer_children: Vec<ComponentNode>,
}

const SMART_LAYOUT_SCRIPT: &str = r#"
(function() {
  'use strict';

  if (window.__smartLayoutEngineInitialized) return;
  window.__smartLayoutEngineInitialized = true;

  // Per-element observer data: WeakMap<elem, { observer: ResizeObserver, lastHeight: number }>
  const elemObservers = new WeakMap();

  // Per-container displacement registry: WeakMap<container, Map<child, number>>
  const displacementMap = new WeakMap();

  // Container child registry: WeakMap<container, Set<child>>
  const containerChildren = new WeakMap();

  // Expose mutationObserver to the IIFE scope
  let mutationObserver = null;

  function isAbsolutePos(elem) {
    if (!elem || !elem.isConnected) return false;
    return window.getComputedStyle(elem).position === 'absolute';
  }

  function horizontalOverlap(r1, r2) {
    return r1.left < r2.right && r1.right > r2.left;
  }

  function registerChild(child) {
    const parent = child.parentElement;
    if (!parent) return;
    if (!containerChildren.has(parent)) {
      containerChildren.set(parent, new Set());
    }
    containerChildren.get(parent).add(child);
  }

  function unregisterChild(child) {
    const parent = child.parentElement;
    if (!parent) return;
    const set = containerChildren.get(parent);
    if (set) set.delete(child);
  }

  function observeGlobalMutations() {
    if (!mutationObserver) return;
    mutationObserver.observe(document.body, {
      childList: true,
      subtree: true,
      attributes: true,
      attributeFilter: ['style']
    });
  }

  function applyDisplacement(elem, container, deltaY) {
    if (!displacementMap.has(container)) {
      displacementMap.set(container, new Map());
    }
    const containerDisps = displacementMap.get(container);
    const current = containerDisps.get(elem) || 0;
    const newShift = current + deltaY;
    containerDisps.set(elem, newShift);
    
    if (mutationObserver) mutationObserver.disconnect();
    elem.style.transform = newShift === 0 ? '' : `translateY(${newShift}px)`;
    observeGlobalMutations();
  }

  function restoreDisplacement(elem) {
    const parent = elem.parentElement;
    if (!parent) return;
    const containerDisps = displacementMap.get(parent);
    if (!containerDisps) return;
    const shift = containerDisps.get(elem);
    if (shift == null) return;
    const expected = shift === 0 ? '' : `translateY(${shift}px)`;
    if (elem.style.transform !== expected) {
      if (mutationObserver) mutationObserver.disconnect();
      elem.style.transform = expected;
      observeGlobalMutations();
    }
  }

  function pushDownSiblings(target, container, deltaY, targetOldBottom) {
    const children = containerChildren.get(container);
    if (!children) return;
    const targetRect = target.getBoundingClientRect();
    for (const sibling of children) {
      if (sibling === target || !sibling.isConnected) continue;
      const siblingRect = sibling.getBoundingClientRect();
      if (siblingRect.top >= targetOldBottom - 5 && horizontalOverlap(targetRect, siblingRect)) {
        applyDisplacement(sibling, container, deltaY);
      }
    }
  }

  function handleChildOverflow(child, parent) {
    if (!parent || !parent.isConnected) return 0;
    const parentStyle = window.getComputedStyle(parent);
    if (parentStyle.overflowY === 'auto' || parentStyle.overflowY === 'scroll' || parent.tagName === 'MAIN') {
      return 0;
    }
    const childRect = child.getBoundingClientRect();
    const parentRect = parent.getBoundingClientRect();
    const childBottomInParent = childRect.bottom - parentRect.top;
    const parentContentHeight = parent.offsetHeight;
    if (childBottomInParent > parentContentHeight + 1) {
      const overflow = childBottomInParent - parentContentHeight;
      
      if (mutationObserver) mutationObserver.disconnect();
      parent.style.height = `${parentContentHeight + overflow}px`;
      observeGlobalMutations();
      
      return overflow;
    }
    return 0;
  }

  function cascadeParentExpansion(expandedContainer, expandDelta) {
    if (!expandDelta || expandDelta <= 0) return;
    const grandParent = expandedContainer.parentElement;
    if (!grandParent || !grandParent.isConnected) return;

    const expandedRect = expandedContainer.getBoundingClientRect();
    const expandedOldBottom = expandedRect.bottom - expandDelta;

    pushDownSiblings(expandedContainer, grandParent, expandDelta, expandedOldBottom);

    const overflow = handleChildOverflow(expandedContainer, grandParent);
    if (overflow > 0) {
      cascadeParentExpansion(grandParent, overflow);
    }
  }

  function handleResize(target, newHeight) {
    const data = elemObservers.get(target);
    if (!data) return;

    const oldHeight = data.lastHeight;
    if (oldHeight === 0 || newHeight === oldHeight) {
      data.lastHeight = newHeight;
      return;
    }

    const deltaY = newHeight - oldHeight;
    data.lastHeight = newHeight;

    const parent = target.parentElement;
    if (!parent || !parent.isConnected) return;

    const targetRect = target.getBoundingClientRect();
    const targetOldBottom = targetRect.bottom - deltaY;

    pushDownSiblings(target, parent, deltaY, targetOldBottom);

    const overflow = handleChildOverflow(target, parent);
    if (overflow > 0) {
      cascadeParentExpansion(parent, overflow);
    }
  }

  function observeElement(elem) {
    if (!elem || !elem.isConnected) return;
    if (elemObservers.has(elem)) return;
    if (!isAbsolutePos(elem)) return;

    registerChild(elem);

    const ro = new ResizeObserver((entries) => {
      for (const entry of entries) {
        const h = entry.borderBoxSize?.[0]?.blockSize ?? elem.offsetHeight;
        handleResize(elem, h);
      }
    });
    ro.observe(elem);
    elemObservers.set(elem, { observer: ro, lastHeight: elem.offsetHeight });
  }

  function unobserveElement(elem) {
    if (!elemObservers.has(elem)) return;
    const data = elemObservers.get(elem);
    data.observer.disconnect();
    elemObservers.delete(elem);
    unregisterChild(elem);
  }

  function initLayoutEngine() {
    document.querySelectorAll('[id^="comp_"]').forEach(elem => {
      if (isAbsolutePos(elem)) observeElement(elem);
    });

    mutationObserver = new MutationObserver((mutations) => {
      for (const mutation of mutations) {
        if (mutation.type === 'attributes' && mutation.attributeName === 'style') {
          restoreDisplacement(mutation.target);
        }

        if (mutation.type === 'childList') {
          for (const node of mutation.addedNodes) {
            if (node.nodeType !== 1) continue;
            if (isAbsolutePos(node)) observeElement(node);
            node.querySelectorAll('[id^="comp_"]').forEach(child => {
              if (isAbsolutePos(child)) observeElement(child);
            });
          }
          for (const node of mutation.removedNodes) {
            if (node.nodeType !== 1) continue;
            unobserveElement(node);
            node.querySelectorAll('[id^="comp_"]').forEach(unobserveElement);
          }
        }
      }
    });

    observeGlobalMutations();
  }

  if (document.readyState === 'complete' || document.readyState === 'interactive') {
    initLayoutEngine();
  } else {
    document.addEventListener('DOMContentLoaded', initLayoutEngine);
  }
})();
"#;

fn main() {
    dioxus::launch(App);
}

#[derive(Clone, Debug, PartialEq)]
struct RuntimeConfig {
  api_base_url: String,
  compile_id: String,
  project_id: String,
  hosted_base_path: String,
  website_title: String,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct RouteMapEntry {
  pub path: String,
  pub page_id: String,
}

fn parse_component_node(value: &serde_json::Value) -> Option<ComponentNode> {
  serde_json::from_value::<ComponentNode>(value.clone()).ok()
}

fn parse_shell_region_children(value: Option<&serde_json::Value>) -> Vec<ComponentNode> {
  let Some(val) = value else { return Vec::new(); };
  
  if let Some(arr) = val.get("children").and_then(|v| v.as_array()) {
    arr.iter()
      .filter_map(|child| serde_json::from_value::<ComponentNode>(child.clone()).ok())
      .collect()
  } else {
    if let Ok(node) = serde_json::from_value::<ComponentNode>(val.clone()) {
      vec![node]
    } else {
      Vec::new()
    }
  }
}

fn parse_shell_package(pkg: &serde_json::Value) -> ShellPackage {
  let shell = pkg.get("shell").and_then(|value| value.as_object());
  let Some(shell) = shell else {
    return ShellPackage::default();
  };

  let parse_string_vec = |key: &str| -> Vec<String> {
    shell
      .get(key)
      .and_then(|value| value.as_array())
      .map(|items| {
        items
          .iter()
          .filter_map(|item| item.as_str().map(|s| s.to_string()))
          .collect::<Vec<_>>()
      })
      .unwrap_or_default()
  };

  ShellPackage {
    default_page: shell
      .get("default_page")
      .or_else(|| shell.get("defaultPage"))
      .and_then(|value| value.as_str())
      .map(|s| s.to_string()),
    hide_shell_on_pages: parse_string_vec("hide_shell_on_pages"),
    hide_shell_on_routes: parse_string_vec("hide_shell_on_routes"),
    on_load: shell
      .get("on_load")
      .and_then(|value| value.as_array())
      .cloned()
      .unwrap_or_default(),
    sidebar_children: parse_shell_region_children(shell.get("sidebar")),
    topbar_children: parse_shell_region_children(shell.get("topbar")),
    footer_children: parse_shell_region_children(shell.get("footer")),
  }
}

fn runtime_config() -> RuntimeConfig {
  let api_base_url = option_env!("API_BASE_URL")
    .unwrap_or("http://localhost:8080")
    .trim_end_matches('/')
    .to_string();
  let compile_id = option_env!("COMPILE_ID").unwrap_or("").trim().to_string();
  let project_id = option_env!("PROJECT_ID").unwrap_or("").trim().to_string();
  let hosted_base_path = option_env!("HOSTED_BASE_PATH")
    .unwrap_or("")
    .trim()
    .to_string();
  let website_title = option_env!("WEBSITE_TITLE")
    .unwrap_or("Rustify-Renderer")
    .trim()
    .to_string();

  RuntimeConfig {
    api_base_url,
    compile_id,
    project_id,
    hosted_base_path,
    website_title,
  }
}

fn normalize_base_path(base: &str) -> String {
  let trimmed = base.trim();
  if trimmed.is_empty() {
    return String::new();
  }

  let normalized = normalized_path(trimmed);
  if normalized == "/" {
    String::new()
  } else {
    normalized.trim_end_matches('/').to_string()
  }
}

fn normalized_path(path: &str) -> String {
  let cleaned = path.trim();
  if cleaned.is_empty() {
    return "/".to_string();
  }
  if cleaned.starts_with('/') {
    cleaned.to_string()
  } else {
    format!("/{}", cleaned)
  }
}

fn split_segments(path: &str) -> Vec<&str> {
  path.trim_matches('/')
    .split('/')
    .filter(|s| !s.is_empty())
    .collect::<Vec<_>>()
}

fn route_matches(pattern: &str, path: &str) -> bool {
  let lhs = split_segments(pattern);
  let rhs = split_segments(path);

  if lhs.len() != rhs.len() {
    return false;
  }

  lhs.iter().zip(rhs.iter()).all(|(lp, rp)| {
    (lp.starts_with('{') && lp.ends_with('}')) || (lp.starts_with(':')) || lp == rp
  })
}

fn path_candidates(path: &str, hosted_base: &str) -> Vec<String> {
  let mut normalized = normalized_path(path);
  let base = normalize_base_path(hosted_base);
  if !base.is_empty() && normalized.starts_with(&base) {
    let stripped = normalized[base.len()..].to_string();
    normalized = if stripped.is_empty() {
      "/".to_string()
    } else {
      normalized_path(&stripped)
    };
  }
  let segments = split_segments(&normalized);

  if segments.is_empty() {
    return vec!["/".to_string()];
  }

  let mut out = Vec::with_capacity(segments.len() + 1);
  out.push(normalized.clone());
  for i in 1..segments.len() {
    out.push(format!("/{}", segments[i..].join("/")));
  }
  out.push("/".to_string());
  out
}

fn extract_routes_from_package(pkg: &serde_json::Value) -> (Vec<RouteMapEntry>, Option<String>) {
  let mut routes = Vec::new();

  let mut default_route = None;

  let routes_value = pkg.get("routes");
  let routes_array = routes_value
    .and_then(|value| value.get("routes"))
    .and_then(|value| value.as_array())
    .or_else(|| routes_value.and_then(|value| value.as_array()));

  if let Some(routes_obj) = routes_value.and_then(|value| value.as_object()) {
    default_route = routes_obj
      .get("default_route")
      .or_else(|| routes_obj.get("defaultRoute"))
      .and_then(|value| value.as_str())
      .map(normalized_path);
  }

  if let Some(items) = routes_array {
    for route in items {
      let path = route
        .get("path")
        .and_then(|value| value.as_str())
        .unwrap_or("");
      let page_id = route
        .get("page_id")
        .or_else(|| route.get("pageId"))
        .and_then(|value| value.as_str())
        .unwrap_or("");
      let page = route
        .get("page")
        .and_then(|value| value.as_str())
        .unwrap_or("");

      let id = if !page_id.is_empty() {
        page_id.to_string()
      } else {
        page.to_string()
      };

      if !id.is_empty() && !path.is_empty() {
        routes.push(RouteMapEntry {
          path: normalized_path(path),
          page_id: id,
        });
      }
    }
  }

  (routes, default_route)
}

fn decode_url_param(param: &str) -> String {
    let mut bytes = Vec::new();
    let mut chars = param.as_bytes().iter().peekable();
    while let Some(&b) = chars.next() {
        if b == b'%' {
            if let (Some(&h), Some(&l)) = (chars.next(), chars.next()) {
                if let Ok(hex) = String::from_utf8(vec![h, l]) {
                    if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                        bytes.push(byte);
                        continue;
                    }
                }
            }
        }
        bytes.push(b);
    }
    String::from_utf8_lossy(&bytes).into_owned()
}

fn extract_route_params(pattern: &str, path: &str) -> serde_json::Value {
  let mut params = serde_json::Map::new();
  let lhs = split_segments(pattern);
  let rhs = split_segments(path);

  if lhs.len() == rhs.len() {
    for (lp, rp) in lhs.iter().zip(rhs.iter()) {
      if lp.starts_with(':') && lp.len() > 1 {
        let key = &lp[1..];
        params.insert(key.to_string(), serde_json::Value::String(decode_url_param(rp)));
      } else if lp.starts_with('{') && lp.ends_with('}') && lp.len() > 2 {
        let key = &lp[1..lp.len() - 1];
        params.insert(key.to_string(), serde_json::Value::String(decode_url_param(rp)));
      }
    }
  }

  serde_json::Value::Object(params)
}

fn page_id_from_location(
  path: &str,
  routes: &[RouteMapEntry],
  hosted_base: &str,
  default_route: Option<&str>,
) -> Option<String> {
  for candidate in path_candidates(path, hosted_base) {
    if let Some(hit) = routes.iter().find(|r| r.path == candidate) {
      return Some(hit.page_id.clone());
    }
  }

  for candidate in path_candidates(path, hosted_base) {
    if let Some(hit) = routes.iter().find(|r| route_matches(&r.path, &candidate)) {
      return Some(hit.page_id.clone());
    }
  }

  if let Some(default_route) = default_route {
    if let Some(hit) = routes.iter().find(|r| r.path == default_route) {
      return Some(hit.page_id.clone());
    }
  }

  routes
    .iter()
    .find(|r| r.path == "/")
    .map(|r| r.page_id.clone())
}

fn route_for_page(page_id: &str, routes: &[RouteMapEntry]) -> Option<String> {
  routes
    .iter()
    .find(|route| route.page_id == page_id)
    .map(|route| route.path.clone())
}

#[cfg(target_arch = "wasm32")]
async fn fetch_json(url: &str) -> Result<serde_json::Value, String> {
  use wasm_bindgen::JsCast;
  use wasm_bindgen_futures::JsFuture;
  use web_sys::Response;

  let window = web_sys::window().ok_or_else(|| "window unavailable".to_string())?;
  let resp_val = JsFuture::from(window.fetch_with_str(url))
    .await
    .map_err(|e| format!("fetch failed: {:?}", e))?;

  let resp: Response = resp_val
    .dyn_into()
    .map_err(|_| "response cast failed".to_string())?;

  if !resp.ok() {
    return Err(format!("HTTP {} for {}", resp.status(), url));
  }

  let json = JsFuture::from(resp.json().map_err(|_| "json() failed".to_string())?)
    .await
    .map_err(|e| format!("json await failed: {:?}", e))?;

  let text = js_sys::JSON::stringify(&json)
    .map_err(|_| "JSON stringify failed".to_string())?
    .as_string()
    .ok_or_else(|| "stringify produced non-string".to_string())?;

  serde_json::from_str::<serde_json::Value>(&text)
    .map_err(|e| format!("json parse failed: {}", e))
}

#[cfg(not(target_arch = "wasm32"))]
async fn fetch_json(_url: &str) -> Result<serde_json::Value, String> {
  Err("runtime fetch is only supported on wasm32 target".to_string())
}

#[cfg(target_arch = "wasm32")]
fn current_location_path() -> String {
  web_sys::window()
    .and_then(|w| w.location().pathname().ok())
    .unwrap_or_else(|| "/".to_string())
}

#[cfg(not(target_arch = "wasm32"))]
fn current_location_path() -> String {
  "/".to_string()
}

fn package_url(cfg: &RuntimeConfig) -> String {
  format!("{}/api/compiler/package/{}", cfg.api_base_url, cfg.compile_id)
}

fn page_url(cfg: &RuntimeConfig, page_id: &str) -> String {
  format!(
    "{}/api/compiler/page/{}/{}",
    cfg.api_base_url, cfg.compile_id, page_id
  )
}

pub(crate) fn normalize_href_for_navigate(input: &str) -> String {
  normalize_href_for_navigate_with_base(input, option_env!("HOSTED_BASE_PATH").unwrap_or(""))
}

pub(crate) fn normalize_route(route: &str) -> String {
  let trimmed = route.trim();
  if trimmed.is_empty() {
    "/".to_string()
  } else if trimmed.starts_with('/') {
    trimmed.to_string()
  } else {
    format!("/{}", trimmed)
  }
}

#[cfg(target_arch = "wasm32")]
fn normalize_href_for_navigate_with_base(input: &str, hosted_base_path: &str) -> String {
  if input.starts_with("http://") || input.starts_with("https://") {
    return input.to_string();
  }

  let mut raw = normalized_path(input);
  let base = normalize_base_path(hosted_base_path);
  if !base.is_empty() {
    raw = if raw == "/" {
      base
    } else {
      format!("{}{}", base, raw)
    };
  }

  if let Some(window) = web_sys::window() {
    if let Ok(pathname) = window.location().pathname() {
      let current = split_segments(&pathname);
      let target = split_segments(&raw);

      for i in 0..current.len() {
        let prefix = &current[..i];
        let mut joined = Vec::with_capacity(prefix.len() + target.len());
        joined.extend_from_slice(prefix);
        joined.extend_from_slice(&target);

        let candidate = if joined.is_empty() {
          "/".to_string()
        } else {
          format!("/{}", joined.join("/"))
        };

        if pathname.starts_with(&candidate) {
          return candidate;
        }
      }
    }
  }

  raw
}

#[cfg(not(target_arch = "wasm32"))]
fn normalize_href_for_navigate_with_base(input: &str, hosted_base_path: &str) -> String {
  let raw = normalized_path(input);
  let base = normalize_base_path(hosted_base_path);
  if base.is_empty() {
    raw
  } else if raw == "/" {
    base
  } else {
    format!("{}{}", base, raw)
  }
}

#[component]
fn App() -> Element {
  let config = runtime_config();
  let _ = (&config.project_id, &config.hosted_base_path);

  let mut package_state = use_signal(|| None::<Result<serde_json::Value, String>>);
  let mut route_entries = use_signal(Vec::<RouteMapEntry>::new);
  let mut default_route = use_signal(|| None::<String>);
  let mut shell_state = use_signal(ShellPackage::default);
  let mut page_cache = use_signal(std::collections::HashMap::<String, ComponentNode>::new);
  let mut current_page_id = use_signal(|| None::<String>);
  let mut page_error = use_signal(|| None::<String>);
  let mut current_path = use_signal(|| current_location_path());
  let mut in_flight_fetches = use_signal(std::collections::HashSet::<String>::new);

  let mut data_state = use_signal(|| serde_json::Value::Object(Default::default()));
  use_context_provider(|| renderer::GlobalDataState(data_state));
  let default_item = use_signal(|| None);
  use_context_provider(|| renderer::RepeaterItemState(default_item));
  use_context_provider(|| renderer::ParentLayoutContext(false));
  use_context_provider(|| renderer::PageLayoutModeContext(models::LayoutMode::Absolute));
  let bp_signal = use_signal(|| "desktop".to_string());
  use_context_provider(|| renderer::ActiveBreakpointContext(bp_signal));

  // RouteEntriesContext for child components
  #[derive(Clone, Copy)]
  pub struct RouteEntriesContext(pub Signal<Vec<RouteMapEntry>>);
  use_context_provider(|| RouteEntriesContext(route_entries));

  // Run shell on_load actions exactly once when the shell package is first loaded.
  // We use a run-once guard signal to avoid re-firing on subsequent data_state writes.
  let mut shell_loaded = use_signal(|| false);
  {
    use_effect(move || {
      // Only reactive on shell_state. Use peek for data_state to avoid subscription.
      let shell = shell_state();
      if !shell.on_load.is_empty() && !shell_loaded() {
        shell_loaded.set(true);
        let ol = shell.on_load.clone();
        spawn(async move {
          renderer::execute_actions(ol, data_state, None).await;
        });
      }
    });
  }

  {
    let config_clone = config.clone();
    use_effect(move || {
      let cfg = config_clone.clone();
      spawn(async move {
        if cfg.compile_id.is_empty() {
          package_state.set(Some(Err(
            "COMPILE_ID is missing. Set COMPILE_ID in .env and rebuild.".to_string(),
          )));
          return;
        }

        let url = package_url(&cfg);
        let fetched = fetch_json(&url).await;
        match fetched {
          Ok(value) => {
            package_state.set(Some(Ok(value)));
          }
          Err(e) => {
            package_state.set(Some(Err(e)));
          }
        }
      });
    });
  }

  {
    use_effect(move || {
      if let Some(Ok(pkg)) = package_state() {
        let (routes, default_route_path) = extract_routes_from_package(&pkg);
        route_entries.set(routes.clone());
        
        // Save routes under "__routes" in global state
        {
          let mut ds = data_state.write();
          if let Some(obj) = ds.as_object_mut() {
            if let Ok(routes_val) = serde_json::to_value(&routes) {
              obj.insert("__routes".to_string(), routes_val);
            }
          }
        }

        default_route.set(default_route_path);
        shell_state.set(parse_shell_package(&pkg));
        page_cache.set(std::collections::HashMap::new());
        page_error.set(None);
      }
    });
  }

  // Register popstate listener on the browser window
  {
    use_effect(move || {
      #[cfg(target_arch = "wasm32")]
      {
        use wasm_bindgen::closure::Closure;
        use wasm_bindgen::JsCast;
        if let Some(window) = web_sys::window() {
          let cb = Closure::wrap(Box::new(move |_: web_sys::Event| {
            if let Some(w) = web_sys::window() {
              if let Ok(pathname) = w.location().pathname() {
                current_path.set(pathname);
              }
            }
          }) as Box<dyn FnMut(web_sys::Event)>);
          
          let _ = window.add_event_listener_with_callback("popstate", cb.as_ref().unchecked_ref());
          cb.forget();
        }
      }
    });
  }

  {
    let hosted_base_path = config.hosted_base_path.clone();
    use_effect(move || {
      if !route_entries().is_empty() {
        let path = current_path();
        let candidates = path_candidates(&path, &hosted_base_path);
        
        let mut matched_route = None;
        for candidate in candidates.iter() {
          if let Some(hit) = route_entries().iter().find(|r| &r.path == candidate) {
            matched_route = Some((hit.page_id.clone(), hit.path.clone(), candidate.clone()));
            break;
          }
        }
        if matched_route.is_none() {
          for candidate in candidates.iter() {
            if let Some(hit) = route_entries().iter().find(|r| route_matches(&r.path, candidate)) {
              matched_route = Some((hit.page_id.clone(), hit.path.clone(), candidate.clone()));
              break;
            }
          }
        }
        if matched_route.is_none() {
          if let Some(def_route) = default_route() {
            if let Some(hit) = route_entries().iter().find(|r| r.path == def_route) {
              matched_route = Some((hit.page_id.clone(), hit.path.clone(), def_route.clone()));
            }
          }
        }

        if let Some((new_page_id, pattern, actual_path)) = matched_route {
          let route_params = extract_route_params(&pattern, &actual_path);
          {
            let mut data = data_state.write();
            if let Some(obj) = data.as_object_mut() {
              obj.insert("routeParams".to_string(), route_params);
            }
          }

          // Guard check: Only change state if moving to a completely new page ID
          if current_page_id.peek().as_ref() != Some(&new_page_id) {
            page_cache.write().clear();
            page_error.set(None);
            current_page_id.set(Some(new_page_id));
          }
        } else {
          // Fallback handling for 404/unmatched paths
          if current_page_id.peek().is_some() {
            current_page_id.set(None);
          }
        }
      }
    });
  }

  {
    let config_clone = config.clone();
    use_effect(move || {
      let cfg = config_clone.clone();
      if let Some(page_id) = current_page_id() {
        if !page_cache.peek().contains_key(&page_id) && !in_flight_fetches.peek().contains(&page_id) {
          in_flight_fetches.write().insert(page_id.clone());
          let id = page_id.clone();
          spawn(async move {
            let url = page_url(&cfg, &id);
            match fetch_json(&url).await {
              Ok(value) => match serde_json::from_value::<ComponentNode>(value) {
                Ok(page) => {
                  page_cache.write().insert(id.clone(), page);
                  page_error.set(None);
                }
                Err(e) => {
                  page_error.set(Some(format!(
                    "failed to parse page {}: {}",
                    id, e
                  )));
                }
              },
              Err(e) => {
                page_error.set(Some(format!(
                  "failed to fetch page {}: {}",
                  id, e
                )));
              }
            }
            in_flight_fetches.write().remove(&id);
          });
        }
      }
    });
  }

  let package_result = package_state();
  match package_result {
    None => rsx! {
      document::Title { "{config.website_title}" }
      LoadingScreen {
        title: "Loading compiled package...",
        detail: "Connecting to compiler endpoint and fetching multipage schema."
      }
    },
    Some(Err(e)) => rsx! {
      document::Title { "{config.website_title}" }
      ErrorScreen {
        title: "Failed to load compiled package",
        detail: e
      }
    },
    Some(Ok(_)) => {
      let page_id_opt = current_page_id();
      let current_path_str = current_path();
      
      if let Some(page_id) = page_id_opt {
        if let Some(page) = page_cache().get(&page_id).cloned() {
          let node = page.clone();
          let global_css = generate_global_css(&node);
          let shell = shell_state();
          let current_route = route_for_page(&page_id, &route_entries());
          
          let hide_shell = shell.hide_shell_on_pages.iter().any(|hidden| hidden == &page_id)
            || current_route.as_ref().map(|route| shell.hide_shell_on_routes.iter().any(|hidden| normalize_route(hidden) == normalize_route(route))).unwrap_or(false);




          let (left_sidebar, right_sidebar): (Vec<ComponentNode>, Vec<ComponentNode>) = shell.sidebar_children.clone().into_iter().partition(|node| {
            node.props.extra.get("position")
              .and_then(|v| v.as_str())
              .unwrap_or("left") != "right"
          });

          let show_topbar = !shell.topbar_children.is_empty();
          let show_left_sidebar = !left_sidebar.is_empty();
          let show_right_sidebar = !right_sidebar.is_empty();
          let show_footer = !shell.footer_children.is_empty();

          let shell_style = "display: flex; flex-direction: column; height: 100vh; width: 100vw; overflow: hidden; background: #f3f4f6; color: #0f172a;";
          let page_renderer_style = "position: relative; width: 100%; min-height: 100%; box-sizing: border-box;";

          rsx! {
            document::Title { "{config.website_title}" }
            document::Link {
              rel: "stylesheet",
              href: "https://fonts.googleapis.com/css2?family=Outfit:wght@300;400;500;600;700;800&display=swap"
            }

            style {
              "{global_css}"
            }

            if !hide_shell {
              div {
                class: "app-shell",
                style: "{shell_style}",
                
                if show_topbar {
                  header {
                    style: "flex-shrink: 0; z-index: 30; background: #ffffff; border-bottom: 1px solid #e5e7eb; box-sizing: border-box; min-width: 0; min-height: 56px; display: flex; align-items: stretch;",
                    div {
                      style: "width: 100%; min-width: 0;",
                      for child in shell.topbar_children.iter() {
                        ComponentRenderer { node: child.clone() }
                      }
                    }
                  }
                }

                div {
                  style: "display: flex; flex: 1; min-height: 0; min-width: 0; overflow: hidden;",
                  
                  if show_left_sidebar {
                    aside {
                      class: "hide-scrollbar",
                      style: "width: 250px; flex: 0 0 250px; background: #ffffff; color: #475569; border-right: 1px solid #e5e7eb; overflow-y: auto; z-index: 20; min-width: 0;",
                      div {
                        style: "display: flex; flex-direction: column; min-height: 100%; width: 100%;",
                        for child in left_sidebar.iter() {
                          ComponentRenderer { node: child.clone() }
                        }
                      }
                    }
                  }

                  main {
                    style: "flex: 1; min-width: 0; min-height: 0; overflow-x: hidden; overflow-y: auto; position: relative;",
                    div {
                      style: "container-type: inline-size; width: 100%; min-height: 100%;",
                      div {
                        class: "page-renderer",
                        style: "{page_renderer_style}",
                        PageWrapper {
                          key: "{current_path_str}",
                          node: node.clone(),
                          page_id: page_id.clone(),
                          data_state: data_state,
                        }
                      }
                    }
                  }

                  if show_right_sidebar {
                    aside {
                      class: "hide-scrollbar",
                      style: "width: 250px; flex: 0 0 250px; background: #ffffff; color: #475569; border-left: 1px solid #e5e7eb; overflow-y: auto; z-index: 20; min-width: 0;",
                      div {
                        style: "display: flex; flex-direction: column; min-height: 100%; width: 100%;",
                        for child in right_sidebar.iter() {
                          ComponentRenderer { node: child.clone() }
                        }
                      }
                    }
                  }
                }

                if show_footer {
                  footer {
                    style: "flex-shrink: 0; background: #ffffff; border-top: 1px solid #e5e7eb; padding: 8px 16px; box-sizing: border-box; min-width: 0;",
                    for child in shell.footer_children.iter() {
                      ComponentRenderer { node: child.clone() }
                    }
                  }
                }
              }
            } else {
              PageWrapper {
                key: "{current_path_str}",
                node: node.clone(),
                page_id: page_id.clone(),
                data_state: data_state,
              }
            }
            script {
              "{SMART_LAYOUT_SCRIPT}"
            }
          }
        } else if let Some(err) = page_error() {
          rsx! {
            document::Title { "{config.website_title}" }
            ErrorScreen {
              title: "Page fetch failed",
              detail: err
            }
          }
        } else {
          rsx! {
            document::Title { "{config.website_title}" }
            LoadingScreen {
              title: "Loading Page...",
              detail: format!("Fetching layout configuration for page ID: {}.", page_id)
            }
          }
        }
      } else {
        rsx! {
          document::Title { "{config.website_title}" }
          ErrorScreen {
            title: "Page Not Found (404)",
            detail: format!("No route matches the path: {}.", current_path_str)
          }
        }
      }
    }
  }
}

#[component]
fn PageWrapper(
  node: ComponentNode,
  page_id: String,
  data_state: Signal<serde_json::Value>,
) -> Element {
  let on_load_actions = node.on_load.clone();
  let has_on_load = !on_load_actions.is_empty();
  use_effect(move || {
    if has_on_load {
      let ol = on_load_actions.clone();
      spawn(async move {
        renderer::execute_actions(ol, data_state, None).await;
      });
    }
  });

  use_context_provider(|| renderer::PageLayoutModeContext(node.layout_mode.clone()));

  rsx! {
    ComponentRenderer { node: node }
  }
}

#[component]
fn LoadingScreen(title: String, detail: String) -> Element {
  rsx! {
    div {
      style: "padding: 24px; color: #0f172a; background: linear-gradient(160deg, #f8fafc 0%, #eef2ff 100%); border: 1px solid #dbeafe; border-radius: 12px; font-family: sans-serif; max-width: 560px; margin: 40px auto; box-shadow: 0 8px 24px rgba(15,23,42,0.08);",
      h3 { style: "margin-top: 0; font-weight: 700;", "{title}" }
      p { style: "font-size: 14px; margin-bottom: 0;", "{detail}" }
    }
  }
}

#[component]
fn ErrorScreen(title: String, detail: String) -> Element {
  rsx! {
    div {
      style: "padding: 24px; color: #b91c1c; background-color: #fef2f2; border: 1px solid #fee2e2; border-radius: 8px; font-family: sans-serif; max-width: 560px; margin: 40px auto; box-shadow: 0 4px 12px rgba(0,0,0,0.05);",
      h3 { style: "margin-top: 0; font-weight: 700;", "{title}" }
      p { style: "font-family: monospace; font-size: 13px; margin-bottom: 0;", "{detail}" }
    }
  }
}

// Extract CSS variables (starting with '--') from the root computedStyles
// and bundle them with premium hover transitions, focus outlines, and keyframe animations.
fn generate_global_css(node: &ComponentNode) -> String {
    let mut css = String::new();
    css.push_str(":root {\n");
    
    if let Some(ref computed) = node.computed_styles {
        for (k, v) in computed {
            if k.starts_with("--") {
                // Clean up string formatting if nested quotes exist
                let clean_val = v.trim_matches('"');
                css.push_str(&format!("  {}: {};\n", k, clean_val));
            }
        }
    }
    css.push_str("}\n\n");

    // Add global body styling and micro-interaction visual rules
    css.push_str(r#"
body {
    margin: 0;
    padding: 0;
    background-color: var(--background, #f8fafc);
    color: var(--foreground, #0f172a);
    font-family: 'Outfit', -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
    -webkit-font-smoothing: antialiased;
    -moz-osx-font-smoothing: grayscale;
}

/* Scrollbar styling */
::-webkit-scrollbar {
    width: 8px;
    height: 8px;
}
::-webkit-scrollbar-track {
    background: #f1f5f9;
}
::-webkit-scrollbar-thumb {
    background: #cbd5e1;
    border-radius: 4px;
}
::-webkit-scrollbar-thumb:hover {
    background: #94a3b8;
}

/* Animations */
@keyframes slideDown {
    from {
        opacity: 0;
        transform: translateY(-6px);
    }
    to {
        opacity: 1;
        transform: translateY(0);
    }
}

/* Global focus state animations for inputs */
input:focus, textarea:focus, select:focus {
    border-color: #3b82f6 !important;
    box-shadow: 0 0 0 3px rgba(59, 130, 246, 0.15) !important;
    transform: translateY(-1px);
}

/* Interactive elements hover scaling */
button, input, textarea, select {
    transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1);
}

button:hover {
    transform: translateY(-1px) scale(1.01);
    box-shadow: 0 6px 20px rgba(3, 2, 19, 0.18) !important;
}

button:active {
    transform: translateY(1px) scale(0.98);
}
"#);

    css
}

#[test]
fn test_parse_repeater() {
  let payload = include_str!("../../home.json");
  let parsed_node: ComponentNode = serde_json::from_str(payload).unwrap();
    fn find_repeater(node: &ComponentNode) {
        if node.props.extra.get("repeaterEnabled").and_then(|v| v.as_bool()).unwrap_or(false) {
            println!("FOUND CARD REPEATER: {:?}", node.id);
            println!("REPEATER DS: {:?}", node.props.extra.get("repeaterDataSource"));
        }
        for child in &node.children {
            find_repeater(child);
        }
    }
    find_repeater(&parsed_node);
}

#[test]
fn test_resolve_array_path() {
    let response_str = r#"{"functions_get_mongo_master_output_data":[{"email":"rk@assisto.tech","subject":"renew policy","emails":[{"from_name":"R K"}]}]}"#;
    let response_json: serde_json::Value = serde_json::from_str(response_str).unwrap();
    
    let mut global_data = serde_json::Value::Object(Default::default());
    global_data.as_object_mut().unwrap().insert("re_result".to_string(), response_json);
    
    let path = "re_result.functions_get_mongo_master_output_data[0].emails";
    
    let result = renderer::resolve_array_path(&global_data, path);
    println!("RESOLVED RESULT: {:?}", result);
    assert!(result.is_some());
    let arr = result.unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0].get("from_name").unwrap().as_str().unwrap(), "R K");
}
