# Rustify - JSON-to-UI Layout Visualizer (Dioxus Web)

A high-fidelity layout visualization engine built with **Rust** and **Dioxus**, designed to parse, bind, and render complex hierarchical JSON layouts. The visualizer compiles completely to **WebAssembly (WASM)** and now fetches compiled package/page JSON at runtime based on build-time environment configuration.

---

## 🚀 Key Features

*   **100% Component Directory Coverage**: Native visual layouts for all **73 component types** documented in the UI builder directory (ranging from app sidebars to SVG contribution heatmaps and interactive Kanban boards).
*   **Compiled Package Runtime Fetch**: Loads package JSON from compiler endpoints using `COMPILE_ID` + `API_BASE_URL` and lazy-loads missing pages on navigation.
*   **Extensible Schema Mapping**: Uses flattened catch-all HashMaps to capture arbitrary properties without schema rigidity or deserialization blockages.
*   **Theme Variable Extraction**: Dynamically parses computed styling custom keys (e.g. `--primary`, `--radius`) to inject global `:root` variable stylesheets on mounting.
*   **Stateful Micro-Interactions**:
    *   **Live Clock**: Real-time ticker fetching system timings asynchronously via `gloo-timers`.
    *   **Auto-Focus OTP Fields**: Shifting digit focuses on character entries.
    *   **Interactions**: Selectable tab panels, collapsible accordions, rating stars, task movements, and comment inputs.

---

## 🛠️ The Tech Stack

*   **Framework**: [Dioxus (v0.7)](https://dioxuslabs.com/) (React-like Virtual DOM structure compiled to WASM).
*   **WASM Bindings**: `wasm-bindgen`, `js-sys`, `web-sys` (native browser bindings for focusing elements and querying date structures).
*   **Timers**: `gloo-timers` (concurrency-safe async interval polling).
*   **Parsing**: `serde` & `serde_json`.

---

## 📂 Project Structure

```text
antigravity-web/
├── Cargo.toml          # Project dependencies (Serde, Dioxus, Gloo-timers, Web-sys)
├── Dioxus.toml         # Build target configuration settings
├── assets/
│   └── main.css        # Default style overrides
└── src/
    ├── main.rs         # Application entry point & theme extractor
    ├── models.rs       # Deserialization structures & string converter utilities
    └── renderer.rs     # Switchboard mapping all 73 component visualizers recursively
```

---

## 🏗️ Supported Components (73 Categories)

### 1. Structural Layout Containers
- `Layout` (page blocks)
- `Flex` (axis layouts)
- `Grid` (responsive grid layouts)
- `Box` (spacers/wrappers)
- `Container` (centered containers)
- `Stack` (linear stack arrays)
- `Card` (bordered boards)
- `List` (template repeaters)
- `Divider` (`hr` sections)

### 2. Navigation Panels & Shells
- `Sidebar` (side navigation menu layouts)
- `Topbar` (horizontal top menu headers)
- `Aside` (sidebar sliding drawers)
- `Bar` (edge utility tools)
- `FilterBar` (query headers)
- `Tabs` & `Tab` (headers switching content panes)
- `Breadcrumbs` (historical paths)

### 3. Core Text & Indicators
- `Heading` (h1-h6 tags)
- `Text` (typography blocks)
- `Alert` (severity message banners)
- `Badge` (status tags)
- `StatusBadge` (alert level status pills)
- `Link` (anchor links)

### 4. Interactive Form Inputs
- `Input` & `SearchInput` (with magnifying glass tags)
- `Textarea` (expanding multi-line textboxes)
- `Checkbox` (boolean checklist indicators)
- `Toggle` & `Switch` (sliding boolean switch controls)
- `Select` (dropdown filters)
- `DatePicker` & `TimePicker` (form popovers)
- `TagInput` (removable tag creator list)
- `OtpInput` (shifting auto-focus digit fields)
- `ColorPicker` (theme color selections)
- `MultiSelectDropdown` (pill option dropdowns)
- `SignaturePad` (signing tablets mockup)
- `RichTextEditor` (WYSIWYG formats)

### 5. Media elements
- `Image` & `ImageGallery` (thumbnail catalogs)
- `ImageWithAnnotations` (hotspot diagrams)
- `Video` & `Audio` (player layouts)
- `PlaylistPlayer` (track playlists)
- `Iframe` (secure external widgets)

### 6. Interactive Dashboards & Grids
- `KanbanBoard` (lanes with drag status cards)
- `YearCalendar` (SVG contribution density activity grids)
- `CalendarViewer` (scheduling monthly grids)
- `TimeViewer` (digital clock tickers)
- `Timeline` (step trackers)
- `WizardStepper` (linear form indicators)
- `Chat` & `ChatViewer` (bubble dialog panels)
- `CommentSection` (social reply lists)
- `ActivityFeed` (logs)
- `CodeSandbox` (code previews)
- `PdfViewer` (reading mockups)
- `StarRating` (rating stars review)
- `QrCodeGenerator` (downloadable QR tags)

---

## ⚙️ Collision Resolutions

During construction, styling name conflicts across JSON schemas were resolved:
1.  **Table vs Textarea `rows`**: `rows` is deserialized as a generic JSON Value to prevent Serde sequence errors when parsing Textarea line lengths (`3`) versus Table cell objects.
2.  **Tabs vs Toggle `value`**: `value` is genericized to support both Tab strings (`"overview"`) and Toggle status booleans (`false`).
3.  **Table vs Kanban `columns`**: `columns` is genericized to support list strings (`["Item", "Status"]`) and list maps (`[{"id": "todo", "title": "To Do"}]`).
4.  **Container Collapse**: Placed container-level base styles (like `min-height: 100vh`) after the absolute layout rules from JSON, preventing height collapses.

---

## ⚡ Development & Setup

### 1. Prerequisites
Ensure you have Rust and Dioxus CLI (`dx`) installed:
```bash
cargo install dioxus-cli --locked
```

### 2. Environment Configuration
Create `.env` from `.env.example` and set required values:

```bash
cp .env.example .env
```

```env
API_BASE_URL=http://localhost:8080
COMPILE_ID=<your-compile-id>
PROJECT_ID=<optional-project-id>
HOSTED_BASE_PATH=<optional-mounted-subpath-like-/user_page>
```

Notes:
- `COMPILE_ID` is required. If missing, the app shows a startup error screen.
- `.env` is injected at compile-time by `build.rs`; changing `.env` requires a rebuild/restart.
- Current endpoint contract is compile-centric:
    - `GET /api/compiler/package/{compileId}`
    - `GET /api/compiler/page/{compileId}/{pageId}`
- `PROJECT_ID` is preserved for backend compatibility but not required by current endpoint composition.

### 3. Run the Dev Server
To start the application locally with hot-reloading:
```bash
dx serve --port 8090
```

Open your browser and navigate to:
👉 **`http://localhost:8090`**

### 4. Multipage Runtime Behavior
- Initial startup fetches the compiled package and builds route mappings.
- The current browser path is matched in this order: exact route, parameterized route, default `/`.
- If hosted under a prefixed path, route matching strips `HOSTED_BASE_PATH` before route resolution.
- If a matched page is not preloaded in the package, the engine lazy-fetches it and caches it in memory.
- `NAVIGATE` actions perform SPA transitions (`history.pushState`) and dispatch `popstate` to refresh rendered page content without hard reload.
