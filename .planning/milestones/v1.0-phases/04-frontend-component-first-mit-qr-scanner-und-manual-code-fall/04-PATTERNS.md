# Phase 4: Frontend (Component-First) mit QR-Scanner und Manual-Code-Fallback — Pattern Map

**Mapped:** 2026-05-04
**Files analyzed:** 19 (12 components + 4 pages + api.rs + router.rs + app.rs + i18n + Cargo.toml + input.css + js.rs)
**Analogs found:** 18/19 (eine echte Lücke: `connection_banner.rs` hat keinen direkten Analogen — Farbkonvention von `error_alert.rs`, sonst Greenfield)

> Dieses Dokument benennt **konkrete** Vorlagen aus `genossi-frontend/`, die der Planner pro neuer Datei in seine PLAN.md übernimmt. Excerpts sind absichtlich knapp und verweisen auf Datei + Zeilen.

---

## File Classification

### Components (12 neu in `genossi-frontend/src/component/`)

| Neue Datei | Role | Data Flow | Closest Analog | Match Quality |
|------------|------|-----------|----------------|---------------|
| `helper_shell.rs` | Layout-Wrapper-Component | render-only, props-driven | `component/footer.rs` (Layout-Pattern) + `app.rs:31-48` (Layout-Wrap) | role-match (kein dedizierter Layout-Wrapper-Vorgänger) |
| `qr_scanner.rs` | Component (Camera-Integration) | event-driven (frame loop) + JS-Bridge | `js.rs:1-22` (wasm-bindgen extern) + `page/templates.rs:179-183` (use_drop-Cleanup) | partial — Camera ist Greenfield, Cleanup-Pattern existiert |
| `manual_code_input.rs` | Form-Component | request-response (validate + submit) | `component/base_components.rs:204-228` (TextInput) + `component/application_create_form.rs:46-111` (Submit + Spinner) | role-match |
| `qr_card.rs` | Print-fähiger Display-Component | render-only + window.print() | `component/error_alert.rs` (kontrolliertes Layout-Pattern) + `component/footer.rs` (`print:hidden`-Konvention) | role-match (Print-CSS ist Greenfield-Phase-4) |
| `attendance_list.rs` | List-Component (Toggle + Polling) | request-response + polling | `component/application_list.rs` (List-Render) + `page/members.rs:104-120` (PUT + Toast on error) | exact (Liste mit Status-Spalte + Action) |
| `attendance_search.rs` | Search-Input mit Debounce | event-driven (debounced) | `component/member_search.rs:60-130` (Input + Filter) + `component/application_search.rs:67-69` (TimeoutFuture-Debounce) | exact |
| `live_counter.rs` | Polling-Component | streaming/polling (5s tick) | `service/auth.rs:35-46` (GlobalSignal-Pattern) + `page/members.rs:58-61` (TimeoutFuture-Pattern) | partial — Polling-Loop ist Greenfield, aber `TimeoutFuture` etabliert |
| `connection_banner.rs` | Sticky-Warning-Banner | event-driven (state-prop) | `component/error_alert.rs` (Color-/Box-Konvention) + `component/footer.rs:8` (`print:hidden`) | role-match nur für Style; Layout selbst neu |
| `assembly_list_row.rs` | List-Row-Component | render-only | `component/application_list.rs:53-77` (table row) — alternativ `<Link>`-Card-Style | exact |
| `assembly_status_badge.rs` | Badge-Component | render-only | `component/application_list.rs:7-27` (`status_badge_class` 1:1 wiederverwendbar) | **exact** |
| `tab_strip.rs` | Tab-Navigation-Component | event-driven (active-key) | `page/applications_page.rs:78-103` (existierende Inline-Tabs als Vorlage zum Extrahieren) | role-match (Tabs existieren inline, müssen in Component extrahiert werden) |
| (optional) `toast.rs` | Toast-Container-Component | event-driven | `page/members.rs:49-62` (`show_toast`-Helper bereits vorhanden) — Plan-Discretion ob extrahieren | role-match |

### Pages (4 neu in `genossi-frontend/src/page/`)

| Neue Datei | Role | Data Flow | Closest Analog | Match Quality |
|------------|------|-----------|----------------|---------------|
| `helper_login.rs` | Page (öffentlich, kein RequirePrivilege) | request-response + auto-redirect | `page/home.rs` (Mount-Effect + nav.replace) + `page/applications_page.rs:48-50` (Mount-Load) | role-match |
| `helper_attendance.rs` | Page | composition (3 Components + Toast) | `page/applications_page.rs:59-150` (Composition-Pattern) | exact (Komposition shared Components) |
| `assemblies.rs` | Page (admin) | request-response + Modal | `page/applications_page.rs` (Liste + Modal-Form) | **exact** |
| `assembly_details.rs` | Detail-Page (admin, 3 Tabs) | request-response + Tabs + Modal | `page/applications_page.rs:77-103` (Tab-Buttons-Pattern) + `page/member_details.rs` (Detail-Page-Aufbau) | role-match (Tab-Pattern existiert nur inline) |

### Top-level / Shared Edits (6 modifizierte Dateien)

| Modifizierte Datei | Role | Edit-Art | Closest Analog | Match Quality |
|--------------------|------|----------|----------------|---------------|
| `src/api.rs` | API-Client | +12 async fn | `api.rs:160-200` (Member-CRUD-Pattern) | **exact** |
| `src/router.rs` | Routing-Config | +4 Route-Variants | `router.rs:21-54` (existing pattern) | **exact** |
| `src/app.rs` | Layout-Branching | +Helper-Route-Branch | `app.rs:31-48` (current Layout-Wrap) | role-match (neue Branch-Logik) |
| `src/i18n/mod.rs` + `de.rs` + `en.rs` | i18n-Keys | +~50 Keys | `i18n/mod.rs:46-484` (Key-Enum) + `i18n/de.rs:5-50` (translate-Match) | **exact** |
| `src/js.rs` | JS-Bridge | +BarcodeDetector + ZXing-Loader | `js.rs:5-22` (wasm-bindgen extern für CodeMirror) | **exact** (gleicher Pattern, andere JS-API) |
| `Cargo.toml` | Build-Config | +6 web-sys Features | `Cargo.toml:39-63` (existing features-Liste) | **exact** |
| `input.css` | CSS | +`@media print`-Block | `input.css:5-25` (existing `@layer utilities`) | role-match (Print-Block ist neu) |
| `tailwind.config.js` | Tailwind-Config | +`safelist: ["qr-card"]` ggf. | `tailwind.config.js:20-28` (existing safelist) | **exact** |
| `assets/zxing.umd.min.js` + `.sha256` | Static-Asset | new vendor file | `assets/shifty.webp` (existing static asset, referenced via `asset!`-Macro) | role-match (Polyfill-JS ist Greenfield) |

---

## Pattern Assignments

### `src/component/attendance_list.rs` (component, request-response + polling)

**Analog:** `genossi-frontend/src/component/application_list.rs` (Liste-Rendering) + `genossi-frontend/src/page/members.rs:104-120` (Toggle/PUT mit Toast-on-Error)

**Imports-Pattern** (Vorlage `application_list.rs:1-5`):

```rust
use dioxus::prelude::*;
use uuid::Uuid;

use crate::api::{self, /* AttendanceMemberTO */};
use crate::i18n::{use_i18n, Key};
use crate::service::config::CONFIG;
```

**List-Render mit Status-Badge** (Vorlage `application_list.rs:53-77`):

```rust
for app in applications.iter() {
    {
        let id = app.id;
        let status_text = status_label(&i18n, &app.status);
        let badge_class = status_badge_class(&app.status);
        rsx! {
            tr {
                class: "border-b hover:bg-gray-50 cursor-pointer",
                onclick: move |_| on_select.call(id),
                td { class: "py-3 px-4", "{app.first_name}" }
                td { class: "py-3 px-4", "{app.last_name}" }
                td { class: "py-3 px-4",
                    span { class: "{badge_class}", "{status_text}" }
                }
            }
        }
    }
}
```

**Toggle-with-loading-State + Toast-on-Error-Pattern** (Vorlage `page/members.rs:104-145`):

```rust
match api::update_member(&config, edited_member).await {
    Ok(updated) => {
        // commit fresh state (here: row.present_at = updated)
        row_saved.write().insert(member_id, true);
    }
    Err(e) => {
        let msg = format!("{}", e);
        row_errors.write().insert(member_id, msg.clone());
        show_toast(toast_messages, toast_counter, msg);
    }
}
```

**Deviationen für `attendance_list.rs`:**
- Card/Row-Variante statt Table (mobile-first, 44px-Touch-Target — UI-SPEC §5)
- **Kein `present_at` toggle vor 200-OK** (D-17: Loading-State während Request, Häkchen erst nach Success)
- Refresh via `refresh_signal: ReadOnlySignal<u64>` (Polling-Tick + nach jedem 200-OK incrementen)
- PII-Reduktion: nur 5 sichtbare Felder (member_number, salutation, title, first_name, last_name) — siehe UI-SPEC §"AttendanceList row"

**Required deps:** keine neuen — `gloo_timers`, `dioxus`, `uuid`, `reqwest` bereits vorhanden.

---

### `src/component/attendance_search.rs` (component, event-driven debounced)

**Analog:** `genossi-frontend/src/component/member_search.rs` + Debounce-Snippet aus `application_search.rs:67-69`

**Input + Filter-Pattern** (Vorlage `member_search.rs:60-103`):

```rust
input {
    class: "w-full px-3 py-2 border border-gray-300 rounded-md focus:ring-2 focus:ring-blue-500",
    r#type: "text",
    placeholder: "Name oder Nummer suchen...",
    value: "{query}",
    oninput: move |e| {
        query.set(e.value().clone());
    },
}
```

**Debounce-Pattern** (Vorlage `application_search.rs:66-71`):

```rust
onfocusout: move |_| {
    spawn(async move {
        gloo_timers::future::TimeoutFuture::new(150).await;
        show_dropdown.set(false);
    });
}
```

**Deviationen für `attendance_search.rs`:**
- `oninput`-Handler triggert Debounce-Spawn (500ms statt 150ms; siehe UI-SPEC §6 Behavior)
- Latest-Cancel-Pattern via Signal-Generation-Counter — wenn beim Fire der Wert nicht mehr aktuell, kein `on_change.call()`
- Kein Dropdown — nur Pulse-Dot rechts während Debounce-Window
- Keine globalen MEMBERS-Reads; Such-Query an Parent (AttendanceList) via `on_change`-EventHandler

---

### `src/component/live_counter.rs` (component, streaming/polling)

**Analog:** `service/auth.rs:35-46` (GlobalSignal + Loader-Pattern) + `page/members.rs:58-61` (Spawn + TimeoutFuture)

**use_future-Polling-Loop** (NEUER Pattern für Phase 4 — Vorlage indirekt aus RESEARCH.md §"Pattern 1"):

```rust
use_future(move || async move {
    loop {
        let config = CONFIG.read().clone();
        match api::get_assembly_stats(&config, assembly_id).await {
            Ok(s) => { stats.set(Some(s)); consecutive_failures.set(0); }
            Err(_) => { consecutive_failures.with_mut(|n| *n += 1); }
        }
        TimeoutFuture::new(5_000).await;
    }
});
```

**Container-Layout** (Vorlage Tailwind-Konvention aus `error_alert.rs:10`):

```rust
div { class: "bg-white border border-gray-200 rounded-lg p-6 mb-4 flex items-baseline justify-between",
    span { class: "text-sm font-medium text-gray-500 uppercase tracking-wider", "Anwesenheit" }
    span { class: "text-4xl font-bold text-gray-900", "{display}" }
}
```

**Display-State-Logik** (Vorlage RESEARCH.md §"Polling-Pattern"-Sektion):

```rust
let display = match (&*stats.read(), *consecutive_failures.read()) {
    (None, _) => i18n.t(Key::AttendanceCounterUnknown).to_string(),
    (Some(s), 0..=1) => format!("{} von {} anwesend", s.x_present, s.y_total),
    (Some(s), _) => format!("— von {} anwesend", s.y_total),
};
```

**Deviationen für `live_counter.rs`:**
- ConnState-Emit via `on_connection_state: EventHandler<ConnState>` für ConnectionBanner-Sibling
- Kein GlobalSignal — lokales `use_signal` reicht (LiveCounter ist Component-scoped)
- Stop bei Unmount automatisch (Dioxus-Hook-Drop)

**Required deps:** `gloo_timers::future::TimeoutFuture` bereits in Cargo.toml.

---

### `src/component/qr_scanner.rs` (component, JS-Bridge + Camera)

**Analog:** `genossi-frontend/src/js.rs:5-22` (wasm-bindgen extern) + `page/templates.rs:179-183` (use_drop-Cleanup)

**JS-Bridge-Pattern** (Vorlage `js.rs:1-22`):

```rust
use js_sys::{wasm_bindgen::JsValue, Date};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(catch, js_namespace = window, js_name = createTypstEditor)]
    pub fn create_typst_editor(...) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(js_namespace = window, js_name = destroyEditor)]
    pub fn destroy_editor(editor_id: &JsValue);
}
```

**use_drop-Cleanup-Pattern** (Vorlage `page/templates.rs:179-183` — **bestätigter use_drop in Codebase**):

```rust
use_drop(move || {
    if let Some(id) = editor_id.read().as_ref() {
        js::destroy_editor(id);
    }
});
```

**Deviationen für `qr_scanner.rs`:**
- Statt CodeMirror-Editor: `BarcodeDetector` (extern type) + ZXing-Polyfill via `document::eval`-`<script>`-Inject
- `use_drop` muss `MediaStream` killen via Iteration `stream.get_tracks()` + `track.stop()` (siehe RESEARCH.md §"Pattern 2")
- `getUserMedia({video: {facingMode: 'environment'}})` in async-Closure beim Mount
- `<video playsinline muted autoplay>` (iOS-Quirk — RESEARCH.md §"Pitfall 3")
- Feature-Detection via `js_sys::Reflect::has(&window, &JsValue::from_str("BarcodeDetector"))`

**JS-Bridge-Erweiterung in `src/js.rs` (Append-only):**

```rust
#[wasm_bindgen]
extern "C" {
    pub type BarcodeDetector;
    #[wasm_bindgen(constructor)]
    pub fn new(options: &JsValue) -> BarcodeDetector;
    #[wasm_bindgen(method)]
    pub fn detect(this: &BarcodeDetector, source: &JsValue) -> js_sys::Promise;
}

pub fn has_barcode_detector() -> bool {
    let window = match web_sys::window() { Some(w) => w, None => return false };
    js_sys::Reflect::has(&window, &JsValue::from_str("BarcodeDetector")).unwrap_or(false)
}
```

**Required deps (Cargo.toml — D-20):**
```toml
features = [
    # ... existing ...
    "MediaDevices", "MediaStream", "MediaStreamTrack",
    "MediaStreamConstraints", "MediaTrackConstraints", "HtmlVideoElement",
]
```

---

### `src/component/manual_code_input.rs` (component, form/validation)

**Analog:** `component/base_components.rs:204-228` (TextInput) + `component/application_create_form.rs:46-111` (Submit-Form mit Spinner)

**Input-Pattern** (Vorlage `base_components.rs:212-227`):

```rust
input {
    class: "border-2 border-gray-200 p-2 min-w-60",
    "type": "text",
    value: props.value,
    disabled: props.disabled,
    oninput: move |event| {
        if let Some(on_change) = &props.on_change {
            let value = event.data.value();
            on_change.call(ImStr::from(value));
        }
    },
}
```

**Submit-mit-Disabled-Pattern** (Vorlage `application_create_form.rs:48-111`):

```rust
form {
    onsubmit: move |evt| {
        evt.prevent_default();
        spawn(async move {
            submitting.set(true);
            error.set(None);
            // ... validation ...
            match api::create_application(&config, &request).await {
                Ok(_) => on_created.call(()),
                Err(e) => error.set(Some(format!("{}", e))),
            }
            submitting.set(false);
        });
    },
}
```

**Deviationen für `manual_code_input.rs`:**
- Klassen statt `base_components`-Default: `font-mono text-2xl tracking-widest text-center uppercase` (UI-SPEC §3)
- Live-Filter im `oninput`: nur Crockford-Base32-Charset, auto-uppercase, `take(10)` cap
- Submit-Button disabled-Logik: `!is_valid_helper_code(&value()) || submitting`
- **Cargo-testbare Pure-Function** `is_valid_helper_code(s: &str) -> bool` exportieren (Test-Pattern wie `member_search.rs::filter_members` mit `#[cfg(test)] mod tests`)
- `CROCKFORD_ALPHABET = "0123456789ABCDEFGHJKMNPQRSTVWXYZ"` (32 Chars; siehe RESEARCH.md §"Pitfall 9")

---

### `src/component/qr_card.rs` (component, render + print)

**Analog:** `component/error_alert.rs` (kontrolliertes Inline-Box-Layout) + `component/footer.rs:8` (`print:hidden`)

**Layout-Pattern** (Vorlage Custom-CSS-Class + Tailwind-Mix):

```rust
div { class: "qr-card bg-white border border-gray-300 rounded-lg p-6 shadow-sm flex flex-col items-center gap-4 max-w-sm mx-auto",
    h2 { class: "text-lg font-semibold text-gray-800", "Helfer-Code für {memo}" }
    div { class: "w-64 h-64", dangerous_inner_html: "{qr_svg}" }
    p { class: "font-mono text-2xl font-semibold tracking-widest text-gray-900 select-all", "{code}" }
    button {
        class: "bg-blue-600 hover:bg-blue-700 text-white px-4 py-2 rounded print:hidden",
        onclick: move |_| { web_sys::window().unwrap().print(); },
        "Drucken"
    }
}
```

**Print-CSS** (in `input.css` ergänzen — UI-SPEC §"QrCard print contract"):

```css
@media print {
    body * { visibility: hidden; }
    .qr-card, .qr-card * { visibility: visible; }
    .qr-card { position: absolute; left: 50%; top: 50%; transform: translate(-50%, -50%); ... }
    @page { size: A4 portrait; margin: 16mm; }
}
```

**Deviationen für `qr_card.rs`:**
- `dangerous_inner_html: "{qr_svg}"` — **trusted producer** (Backend Phase 2 D-21), kein User-Input-Risiko (siehe RESEARCH.md §"Pitfall 8")
- Custom-Class `qr-card` MUSS in `tailwind.config.js` `safelist` aufgenommen werden, falls Tailwind-Purge sie nicht im RSX entdeckt (siehe RESEARCH.md §"Pitfall 6")
- `web_sys::Window::print` benötigt `Window`-Feature → bereits aktiv

---

### `src/component/connection_banner.rs` (component, sticky-warning)

**Analog:** `component/error_alert.rs:10` (Color-/Box-Konvention; **kein direkter Pattern für sticky-top warning**)

**Color-Pattern-Inspiration** (Vorlage `error_alert.rs:10`):

```rust
div { class: "bg-red-100 border border-red-400 text-red-700 px-4 py-3 rounded mb-4 relative",
    // ...
}
```

**Print-Hide-Konvention** (Vorlage `footer.rs:8`):

```rust
class: "bg-gray-800 text-gray-400 py-4 px-4 text-center text-sm print:hidden"
```

**Deviationen für `connection_banner.rs` (NEUES Component-Pattern in dieser Codebase):**
- **NEU:** `sticky top-0 z-30 w-full px-4 py-2 print:hidden` (UI-SPEC §"Connection-Banner colors")
- Amber statt Red — explizit: `bg-amber-100 text-amber-900 border-b-2 border-amber-400` (warning, nicht error)
- Glyph `\u{26A0}` ("⚠") + kleiner Spinner auf der rechten Seite
- Conditional Render: nur wenn `state == ConnState::Lost` (props-driven, kein eigenes Polling)
- **Plan-Hinweis:** safelist die amber-Klassen oder verwende sie inline im RSX (Tailwind-Scanner findet sie dann automatisch)

---

### `src/component/assembly_status_badge.rs` (component, render-only badge)

**Analog:** `component/application_list.rs:7-27` — **EXAKTES** Pattern, nur andere Status-Werte

**Vorlage-Excerpt** (`application_list.rs:7-27`):

```rust
fn status_label(i18n: &crate::i18n::I18n, status: &ApplicationStatusTO) -> String {
    match status {
        ApplicationStatusTO::Offen => i18n.t(Key::StatusOffen).to_string(),
        ApplicationStatusTO::Bestaetigt => i18n.t(Key::StatusBestaetigt).to_string(),
        ApplicationStatusTO::Abgelehnt => i18n.t(Key::StatusAbgelehnt).to_string(),
    }
}

fn status_badge_class(status: &ApplicationStatusTO) -> &'static str {
    match status {
        ApplicationStatusTO::Offen => "bg-yellow-100 text-yellow-800 px-2 py-1 rounded text-xs font-medium",
        ApplicationStatusTO::Bestaetigt => "bg-green-100 text-green-800 px-2 py-1 rounded text-xs font-medium",
        ApplicationStatusTO::Abgelehnt => "bg-red-100 text-red-800 px-2 py-1 rounded text-xs font-medium",
    }
}
```

**Deviationen für `assembly_status_badge.rs`:**
- Andere Enum: `AssemblyStatusTO::Preparation` / `Open` / `Closed`
- Andere Farben: gray (Preparation) / green (Open) / blue (Closed) — UI-SPEC §"Status-Badge palette"
- Andere i18n-Keys: `AssemblyStatusPreparation` / `AssemblyStatusOpen` / `AssemblyStatusClosed`
- **In Component verpacken** (`#[component] pub fn AssemblyStatusBadge(status: AssemblyStatusTO) -> Element`) — `application_list.rs` macht es als private Funktion in der Liste; Phase 4 macht eine eigene Component, weil sie an zwei Stellen reused wird (Liste + Detail-Header).

---

### `src/component/assembly_list_row.rs` (component, list-row + Link)

**Analog:** `component/application_list.rs:53-77` (table-row-Pattern)

**Vorlage-Excerpt:** wie oben (`application_list.rs:53-77`).

**Deviationen für `assembly_list_row.rs`:**
- Card-Style statt Table (UI-SPEC §9): `flex items-center justify-between bg-white border border-gray-200 rounded-lg px-4 py-3 mb-2 hover:bg-gray-50 transition-colors`
- `<Link to=Route::AssemblyDetails { id: ... }>` — Pattern aus `nav_group.rs:35-39`
- Status-Badge via `<AssemblyStatusBadge status=... />` Composition

---

### `src/component/tab_strip.rs` (component, tab-navigation)

**Analog:** `page/applications_page.rs:78-103` — Tab-Pattern existiert **inline** in der Page, muss in Component **extrahiert** werden

**Vorlage-Excerpt** (`applications_page.rs:78-103`):

```rust
div { class: "flex space-x-1 mb-6 border-b",
    for (value, label_key) in tabs.iter() {
        {
            let value = value.to_string();
            let is_active = *active_tab.read() == value;
            let tab_class = if is_active {
                "px-4 py-2 border-b-2 border-blue-500 text-blue-600 font-medium text-sm"
            } else {
                "px-4 py-2 text-gray-500 hover:text-gray-700 text-sm"
            };
            rsx! {
                button {
                    class: "{tab_class}",
                    onclick: { let value = value.clone(); move |_| { active_tab.set(value.clone()); load(); } },
                    {i18n.t(label_key.clone())}
                }
            }
        }
    }
}
```

**Deviationen für `tab_strip.rs`:**
- Component-Props: `tabs: Vec<TabDef>`, `active_key: String`, `on_change: EventHandler<String>`, `children: Element` (Body-Slot)
- Body-Slot via `children` (nicht via Branch in Caller) — sauberer pro UI-SPEC §10
- `print:hidden` für Strip; Body druckt mit
- Component-First: das **identische** inline-Pattern in `applications_page.rs` darf in einem späteren Refactor (out-of-scope für Phase 4) auf den neuen `<TabStrip>` migrieren — Plan-Discretion ob als Phase-4-Bonus.

---

### `src/component/helper_shell.rs` (component, layout-wrapper)

**Analog:** `app.rs:31-48` (current Layout-Wrap) + `component/footer.rs` (Layout-Bestandteil)

**Vorlage-Excerpt** (`app.rs:33-47`):

```rust
div { class: "flex flex-col min-h-screen",
    DropdownBase {}
    div { class: "flex-1",
        Auth { authenticated: rsx! { Router::<Route> {} }, unauthenticated: rsx! { TopBar {} NotAuthenticated {} } }
    }
    Footer {}
}
```

**Deviationen für `helper_shell.rs`:**
- **Kein TopBar/Footer** (D-07 + Datenschutz)
- Eigener schmaler Header: `bg-white border-b border-gray-200 px-4 py-3 flex items-center justify-between print:hidden` mit GV-Name + LogOut-Button
- `min-h-screen bg-gray-50 flex flex-col` als Root
- Main-Body: `flex-1 px-4 py-6 max-w-3xl mx-auto w-full` (mobile-first, schmaler Content)
- Props: `assembly_name: Option<String>`, `on_logout: EventHandler<()>`, `children: Element`

---

### `src/page/helper_login.rs` (page, public, auto-redirect)

**Analog:** `page/home.rs` (Auto-Redirect via `use_effect` + `nav.replace`) + `page/applications_page.rs:48-50` (Mount-Load)

**Auto-Redirect-Pattern** (Vorlage `page/home.rs:1-15`):

```rust
use crate::router::Route;
use dioxus::prelude::*;

#[component]
pub fn Home() -> Element {
    let nav = navigator();
    use_effect(move || {
        nav.replace(Route::Members {});
    });
    rsx! { div {} }
}
```

**Deviationen für `helper_login.rs`:**
- Auto-Redirect nur **nach** API-Call (`/api/helper/session` — Plan-Discretion D-06): wenn 200, nav zu HelperAttendance; wenn 401, render Login-UI
- **Wrap in `<HelperShell>` ohne assembly_name** (None bis Login)
- Composition: `<QrScanner>` als Modal-Overlay + `<ManualCodeInput>` parallel (UI-SPEC §"Helper login")
- Inline-Error unter ManualCodeInput (nicht Toast — Login-Flow ist gated)
- POST `/api/helper/redeem` auf Submit; bei 200: nav.push(`Route::HelperAttendance{}`)
- **KEIN `<RequirePrivilege>`** — Helfer hat keine OIDC-Privilege

---

### `src/page/helper_attendance.rs` (page, helper-only)

**Analog:** `page/applications_page.rs:59-150` (Composition + RequirePrivilege-Wrap)

**Composition-Pattern** (Vorlage `applications_page.rs:115-150`):

```rust
if *loading.read() {
    p { class: "text-gray-500 text-center py-8", {i18n.t(Key::Loading)} }
} else {
    div { class: "bg-white rounded-lg shadow",
        ApplicationList { applications: applications.read().clone(), on_select: ... }
    }
}
```

**Deviationen für `helper_attendance.rs`:**
- Wrap in `<HelperShell assembly_name=Some(...)>`
- 3-Components-Stack: `<LiveCounter>` + `<ConnectionBanner>` + `<AttendanceSearch>` + `<AttendanceList>`
- Refresh-Signal-Wiring: gemeinsames `refresh_signal: Signal<u64>` zwischen `LiveCounter` und `AttendanceList` (Plan-Discretion D-15)
- Toast-Container für Toggle-Errors (siehe `members.rs:49-62` Pattern)
- **KEIN `<RequirePrivilege>`** — Auth-Gate erfolgt via Cookie-Validierung (Backend liefert 401 → redirect zu `/helper`)

---

### `src/page/assemblies.rs` (page, admin)

**Analog:** `page/applications_page.rs` — **EXAKT** dasselbe Pattern (List + Modal-Form)

**Vorlage-Excerpt** (`applications_page.rs:59-138`):

```rust
RequirePrivilege {
    privilege: "admin",
    fallback: rsx! { AccessDeniedPage { required_privilege: "admin".to_string() } },
    TopBar {}
    div { class: "container mx-auto px-4 py-6",
        div { class: "flex justify-between items-start mb-4",
            div { h1 { class: "text-2xl font-bold mb-1", {i18n.t(Key::Applications)} } }
            button {
                class: "bg-blue-500 hover:bg-blue-600 text-white px-4 py-2 rounded text-sm",
                onclick: move |_| show_create_form.set(true),
                {i18n.t(Key::CreateApplication)}
            }
        }
        // List + Create-modal + Detail-modal
        if *show_create_form.read() {
            ApplicationCreateForm { on_close: ..., on_created: ... }
        }
    }
}
```

**Deviationen für `assemblies.rs`:**
- Liste rendert via `<AssemblyListRow>` (Phase-4-Component) statt `<ApplicationList>`
- Create-Modal: simplere Form (Name, Datum, Ort) — Pattern direkt aus `application_create_form.rs:46-111`
- Empty-State: zentrierte Card mit `AssemblyEmpty`-Heading + CTA (UI-SPEC §"Assemblies list")
- Status-Filter-Tabs OPTIONAL (Plan-Discretion — nicht hard-required durch UI-SPEC)

---

### `src/page/assembly_details.rs` (page, admin, 3 tabs)

**Analog:** `page/applications_page.rs:78-103` (Tab-Pattern inline) + `page/member_details.rs` (Detail-Page-Aufbau, scrollt aktuell statt Tabs)

**Tab-Switch-Pattern wird via `<TabStrip>` Component abstrahiert** — die inline-Tabs aus `applications_page.rs` sind die Vorlage für den neuen `tab_strip.rs`-Component.

**Deviationen für `assembly_details.rs`:**
- `<RequirePrivilege privilege="admin">` Wrap
- Header: `<Header>{name}</Header>` + `<AssemblyStatusBadge status=... />`
- 3 Tabs via `<TabStrip>`:
  - **Stamm-Daten** — Edit-Form (Pattern aus `application_form.rs`); disabled wenn Status != Preparation; "GV öffnen"/"GV schließen" Confirm-Dialogs (Pattern aus `application_detail.rs:24-50`)
  - **Helfer-Tokens** — Liste + Create-Modal + just-created `<QrCard>` Inline-Anzeige (One-Time-Show; Backend liefert qr_svg + code nur einmal — Phase 2 D-21)
  - **Anwesenheit** — Branch: wenn Status `Preparation`: Hinweis-Text; sonst die 3 shared Components wie auf `helper_attendance.rs` (ATTN-06 Component-Reuse)
- Active-Tab via lokalem `use_signal::<String>("basics")`

---

### `src/api.rs` (modified, +12 async fn)

**Analog:** `genossi-frontend/src/api.rs:160-200` (Member-CRUD) — **identisches Pattern für alle Phase-4-API-Calls**

**Vorlage-Excerpt** (`api.rs:160-200`):

```rust
// GET
pub async fn get_members(config: &Config) -> Result<Vec<MemberTO>, AppError> {
    info!("Fetching members");
    let url = format!("{}/api/members", config.backend);
    let response = check_response(reqwest::get(url).await?).await?;
    Ok(response.json().await?)
}

// POST
pub async fn create_member(config: &Config, member: MemberTO) -> Result<MemberTO, AppError> {
    info!("Creating member");
    let url = format!("{}/api/members", config.backend);
    let response = reqwest::Client::new().post(url).json(&member).send().await?;
    let response = check_response(response).await?;
    Ok(response.json().await?)
}

// PUT
pub async fn update_member(config: &Config, member: MemberTO) -> Result<MemberTO, AppError> {
    info!("Updating member {:?}", member.id);
    let id = member.id.unwrap();
    let url = format!("{}/api/members/{id}", config.backend);
    let response = reqwest::Client::new().put(url).json(&member).send().await?;
    let response = check_response(response).await?;
    Ok(response.json().await?)
}

// DELETE
pub async fn delete_member(config: &Config, id: Uuid) -> Result<(), AppError> {
    info!("Deleting member {id}");
    let url = format!("{}/api/members/{id}", config.backend);
    let response = reqwest::Client::new().delete(url).send().await?;
    check_response(response).await?;
    Ok(())
}
```

**Phase-4 neue Funktionen (alle nach diesem Schema):**
- `redeem_helper_token(config, code) -> Result<RedeemResponseTO, AppError>` — POST `/api/helper/redeem`
- `list_assemblies(config) -> Result<Vec<AssemblyTO>, AppError>` — GET
- `get_assembly(config, id)` / `create_assembly(config, req)` / `update_assembly(config, id, req)` / `open_assembly(config, id)` / `close_assembly(config, id)`
- `list_helper_tokens(config, aid)` / `create_helper_token(config, aid, memo) -> Result<HelperTokenCreateResponseTO, AppError>` (returns `{token, code, qr_svg}`) / `revoke_helper_token(config, aid, tid)`
- `list_attendance_members(config, aid, search) -> Result<Vec<AttendanceMemberTO>, AppError>`
- `mark_present(config, aid, mid)` / `mark_absent(config, aid, mid)` (idempotent — Phase 3 ATTN-03/04)
- `get_assembly_stats(config, aid) -> Result<AssemblyStatsTO, AppError>`
- (optional) `get_helper_session(config) -> Result<HelperSessionTO, AppError>` — wenn neuer Backend-Endpoint angelegt wird (D-06 Discretion)

**Plan-Task:** `status_to_message` in `api.rs:49-62` ergänzen um **Status 410 ("Eingelöst")** — aktuell nicht mapped. Plan-Discretion ob als Override im `redeem_helper_token`-Aufruf oder im zentralen Mapping.

---

### `src/router.rs` (modified, +4 Route-Variants)

**Analog:** `router.rs:21-54` (existing Route-Enum)

**Vorlage-Excerpt:**

```rust
#[derive(Clone, Routable, Debug, PartialEq)]
pub enum Route {
    #[route("/")]
    Home {},
    #[route("/members")]
    Members {},
    #[route("/members/:id")]
    MemberDetails { id: String },
    // ...
}
```

**Phase-4 zusätzliche Variants** (vor den existierenden — Reihenfolge wichtig für Matcher):

```rust
#[route("/helper")]
Helper {},
#[route("/helper/attendance")]
HelperAttendance {},
#[route("/assemblies")]
Assemblies {},
#[route("/assemblies/:id")]
AssemblyDetails { id: String },
```

---

### `src/app.rs` (modified, helper-route-branch)

**Analog:** `app.rs:31-48` (current Layout-Wrap)

**Vorlage-Excerpt:**

```rust
rsx! {
    document::Stylesheet { href: "/assets/tailwind.css" }
    div { class: "flex flex-col min-h-screen",
        DropdownBase {}
        div { class: "flex-1",
            Auth {
                authenticated: rsx! { Router::<Route> {} },
                unauthenticated: rsx! { TopBar {} NotAuthenticated {} },
            }
        }
        Footer {}
    }
}
```

**Deviationen für Phase-4-Edit:**
- Plan-Discretion (D-05/D-06): **Option A** — if/else basiert auf `use_route()` (näher am bestehenden Pattern); **Option B** — Dioxus-Router `#[layout(...)]`-Annotations auf `Route`-Varianten (idiomatischer)
- Empfehlung Researcher: **Option B** (Layout-Annotations) — siehe RESEARCH.md §"Routing & Auth-Guard Pattern"
- Helper-Routes (`/helper`, `/helper/attendance`) müssen den `<Auth>`-Wrapper **umgehen** — Helfer hat keine OIDC-Auth, sondern Cookie-Session
- Ohne `<TopBar>` und `<Footer>` für Helper-Routes (D-07)

---

### `src/i18n/mod.rs` + `de.rs` + `en.rs` (modified, +~50 keys)

**Analog:** `i18n/mod.rs:46-484` (Key-Enum) + `i18n/de.rs:5-50` (translate-Match)

**Vorlage-Excerpt** (`i18n/mod.rs:46-77`):

```rust
#[derive(Clone, Debug, PartialEq)]
pub enum Key {
    AppTitle,
    Loading,
    // ...
    Members,
    Permissions,
    // ...
}
```

`i18n/de.rs:5-15`:

```rust
pub fn translate(key: Key) -> Rc<str> {
    match key {
        Key::AppTitle => "Genossi".into(),
        Key::Loading => "Laden...".into(),
        // ...
    }
}
```

**Phase-4 neue Keys** (alle aus UI-SPEC §"i18n Key Inventory" — vollständige Liste dort):
- `Assemblies`, `Assembly`, `AssemblyCreate`, `AssemblyName`, `AssemblyDate`, `AssemblyLocation`, `AssemblyOpen`, `AssemblyClose`
- `AssemblyStatusPreparation`, `AssemblyStatusOpen`, `AssemblyStatusClosed`
- `AssemblyEmpty`, `AssemblyEmptyHint`, `AssemblyOpenConfirmTitle`, `AssemblyOpenConfirmText`, `AssemblyCloseConfirmTitle`, `AssemblyCloseConfirmText`
- `AssemblyTabBasics`, `AssemblyTabTokens`, `AssemblyTabAttendance`, `AssemblyAttendanceNotOpenYet`
- `HelperTokens`, `HelperTokenCreate`, `HelperTokenMemo`, `HelperTokenMemoPlaceholder`, `HelperTokenStatusOpen`, `HelperTokenStatusUsed`, `HelperTokenStatusRevoked`, `HelperTokenRevoke`, `HelperTokenPrint`, `HelperTokenCardTitle`, `HelperTokenCardManualHint`, `HelperTokenRedeemed`, `HelperTokenWarning`
- `HelperLoginTitle`, `HelperLoginSubtitle`, `HelperLoginScanQR`, `HelperLoginScanning`, `HelperLoginManualHeading`, `HelperLoginManualPlaceholder`, `HelperLoginSubmit`
- `HelperLoginCameraDenied`, `HelperLoginCameraNotAvailable`, `HelperLoginInvalidFormat`
- `HelperLoginErrorNotFound`, `HelperLoginErrorAlreadyUsed`, `HelperLoginErrorAssemblyClosed`, `HelperLoginErrorRateLimit`
- `HelperShellLogout`, `HelperShellAssemblyHeading`
- `AttendanceSearch`, `AttendanceSearchHint`, `AttendanceCounterLong`, `AttendanceCounterLongLoading`, `AttendanceCounterUnknown`, `AttendanceEmpty`, `AttendanceEmptyHint`, `AttendanceTogglePresent`, `AttendanceToggleAbsent`, `AttendanceToggleSavingHint`, `AttendanceConnectionLost`, `AttendanceConnectionRestored`

**Wichtig — Locale-Pflicht:** **JEDE** neue Key MUSS in `de.rs` UND `en.rs` übersetzt werden. Es gibt **nur zwei Locales** (`Locale::En`, `Locale::De`) — kein `cs.rs`.

---

### `src/js.rs` (modified, +BarcodeDetector + ZXing-Loader)

**Analog:** `js.rs:5-22` (CodeMirror-Bridge) — **EXAKT dasselbe Pattern**

**Vorlage-Excerpt:**

```rust
use js_sys::{wasm_bindgen::JsValue, Date};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(catch, js_namespace = window, js_name = createTypstEditor)]
    pub fn create_typst_editor(...) -> Result<JsValue, JsValue>;
    // ...
}
```

**Phase-4 Erweiterung (Append-only):**

```rust
#[wasm_bindgen]
extern "C" {
    pub type BarcodeDetector;
    #[wasm_bindgen(constructor)]
    pub fn new(options: &JsValue) -> BarcodeDetector;
    #[wasm_bindgen(method)]
    pub fn detect(this: &BarcodeDetector, source: &JsValue) -> js_sys::Promise;
}

pub fn has_barcode_detector() -> bool {
    let window = match web_sys::window() { Some(w) => w, None => return false };
    js_sys::Reflect::has(&window, &JsValue::from_str("BarcodeDetector")).unwrap_or(false)
}
```

ZXing-Polyfill-Lazy-Loader: via `dioxus::document::eval()` (Pattern in Codebase noch nicht verwendet) — siehe RESEARCH.md §"QR-Scanner Integration Plan".

---

### `Cargo.toml` (modified, +6 web-sys features)

**Analog:** `Cargo.toml:39-63` (existing features-Liste)

**Vorlage-Excerpt:**

```toml
[dependencies.web-sys]
version = "0.3"
features = [
    "Window", "Navigator", "Document", "Element", "HtmlElement",
    "HtmlTextAreaElement", "HtmlSelectElement", "HtmlInputElement",
    # ... 22 features ...
    "Url",
]
```

**Phase-4-Append:**

```toml
"MediaDevices",
"MediaStream",
"MediaStreamTrack",
"MediaStreamConstraints",
"MediaTrackConstraints",
"HtmlVideoElement",
```

**WICHTIG — KEIN `BarcodeDetector`-Feature:** Das ist NICHT in web-sys 0.3.97 enthalten (Plan-Pitfall 1) — wird über `js.rs`-extern-Block gemacht.

---

### `input.css` (modified, +`@media print`-Block)

**Analog:** `input.css:5-25` (existing `@layer utilities`)

**Vorlage-Excerpt:**

```css
@tailwind base;
@tailwind components;
@tailwind utilities;

@layer utilities {
    .no-scrollbar::-webkit-scrollbar { display: none; }
    .no-scrollbar { -ms-overflow-style: none; scrollbar-width: none; }
    .scale-down-50 { zoom: 0.5; }
    /* ... */
}
```

**Phase-4-Append** (UI-SPEC §"QrCard Print contract"):

```css
@media print {
    body * { visibility: hidden; }
    .qr-card, .qr-card * { visibility: visible; }
    .qr-card {
        position: absolute; left: 50%; top: 50%;
        transform: translate(-50%, -50%);
        box-shadow: none; border: none; max-width: 80mm;
    }
    .qr-card .w-64 { width: 60mm; height: 60mm; }
    .qr-card .font-mono { font-size: 16pt; letter-spacing: 0.15em; }
    @page { size: A4 portrait; margin: 16mm; }
}

/* Optional: ensure background colors print */
body { -webkit-print-color-adjust: exact; print-color-adjust: exact; }
```

---

### `tailwind.config.js` (potentially modified, +safelist entries)

**Analog:** `tailwind.config.js:20-28` (existing safelist)

**Vorlage-Excerpt:**

```js
safelist: [
    "bg-red-200", "print:bg-white", "cursor-not-allowed",
    "text-green-800", "text-red-800",
    "bg-missingColor", "bg-blockedColor"
]
```

**Phase-4-Append (falls Tailwind-Purge sie nicht im RSX entdeckt — RESEARCH.md §"Pitfall 6"):**

```js
safelist: [
    // ... existing ...
    "qr-card",
    "bg-amber-100", "text-amber-900", "border-amber-400",
    "animate-spin", "animate-pulse",
]
```

**Plan-Task:** Verifikation nach `dx build`: `grep qr-card dist/assets/tailwind.css` → muss matchen.

---

### `assets/zxing.umd.min.js` + `assets/zxing.umd.min.js.sha256` (NEW assets)

**Analog:** `assets/shifty.webp` (existing static asset, referenced via `asset!` macro in `page/not_authenticated.rs:18`)

**Vorlage-Excerpt** (`page/not_authenticated.rs:18`):

```rust
img { src: asset!("/assets/shifty.webp") }
```

**Phase-4 manganis-Pattern für JS-Asset:**

```rust
// In qr_scanner.rs oder einer separaten js_loader.rs
const ZXING: manganis::Asset = manganis::asset!("/assets/zxing.umd.min.js");

// Lazy-Load via document::eval beim ersten Klick auf "QR-Code scannen":
dioxus::document::eval(&format!(r#"
    if (!window.__zxing_loaded) {{
        const s = document.createElement('script');
        s.src = '{}';
        document.head.appendChild(s);
        window.__zxing_loaded = true;
    }}
"#, ZXING));
```

**Vendoring-Procedure** (Plan-Task — UI-SPEC §"ZXing-JS Vendoring Procedure" + RESEARCH.md):

```bash
mkdir -p genossi-frontend/assets/
curl -sL https://unpkg.com/@zxing/library@0.21.3/umd/index.min.js \
    -o genossi-frontend/assets/zxing.umd.min.js
sha256sum genossi-frontend/assets/zxing.umd.min.js \
    > genossi-frontend/assets/zxing.umd.min.js.sha256
```

---

## Shared Patterns

### Authentication / Auth-Guard

**Source:** `genossi-frontend/src/auth.rs:25-48`

**Apply to:** Alle Vorstand-Pages (`assemblies.rs`, `assembly_details.rs`)

```rust
#[component]
pub fn RequirePrivilege(props: RequirePrivilegeProps) -> Element {
    let auth = AUTH.read().clone();
    match auth.auth_info {
        Some(auth_info) if auth_info.has_privilege(props.privilege) => props.children,
        _ => props.fallback.unwrap_or_else(|| rsx! {
            div { class: "text-red-600 p-4", "Access denied. Required privilege: {props.privilege}" }
        }),
    }
}
```

**Verwendung** (Vorlage `applications_page.rs:60-63`):

```rust
RequirePrivilege {
    privilege: "admin",
    fallback: rsx! { AccessDeniedPage { required_privilege: "admin".to_string() } },
    // ... page content ...
}
```

**Hinweis:** Helfer-Routes nutzen NICHT `RequirePrivilege` — Auth erfolgt via Helfer-Cookie (HTTP-Only, vom Browser auto-attached). Bei 401 von Backend: redirect zu `/helper`.

---

### Error Handling

**Source:** `genossi-frontend/src/api.rs:14-100`

**Apply to:** Alle neuen API-Funktionen in `api.rs` (D-22)

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct AppError {
    pub status: Option<u16>,
    pub message: String,
    pub detail: Option<String>,
}

fn status_to_message(status: u16) -> &'static str {
    match status {
        400 => "Ungültige Anfrage",
        401 => "Keine Berechtigung — bitte erneut anmelden",
        // ... vollständige Liste in api.rs:49-62 ...
    }
}

async fn check_response(response: reqwest::Response) -> Result<reqwest::Response, AppError> {
    if response.status().is_success() {
        Ok(response)
    } else {
        Err(map_response_error(response).await)
    }
}
```

**Helfer-Login-Override:** Mapping 404/410/403/400/429 zu i18n-Keys `HelperLoginError*` direkt im Aufrufer (nicht im zentralen Mapping) — UI-SPEC §"Error state — Redeem".

---

### Toast-Notification

**Source:** `genossi-frontend/src/page/members.rs:49-62`

**Apply to:** `helper_attendance.rs` (Toggle-Errors), `assembly_details.rs` (Form-Errors), und ggf. `assemblies.rs`

```rust
fn show_toast(
    toast_messages: &mut Signal<Vec<(u64, String)>>,
    toast_counter: &mut Signal<u64>,
    msg: String,
) {
    let id = *toast_counter.read();
    *toast_counter.write() += 1;
    toast_messages.write().push((id, msg));
    let mut toast_messages = toast_messages.clone();
    spawn(async move {
        TimeoutFuture::new(5_000).await;
        toast_messages.write().retain(|(tid, _)| *tid != id);
    });
}
```

**Plan-Discretion:** `show_toast` aus `members.rs:49-62` extrahieren in einen `component/toast.rs` oder `service/toast.rs` (Component-First-Prinzip — wenn ≥3 Pages Toasts brauchen, MUSS extrahiert werden).

---

### Debounce-Pattern

**Source:** `genossi-frontend/src/component/application_search.rs:67-69` (+ `member_search.rs:65-68`)

**Apply to:** `attendance_search.rs`

```rust
spawn(async move {
    gloo_timers::future::TimeoutFuture::new(150).await;
    // do something (e.g. close dropdown, fire on_change)
});
```

**Phase-4-Variation:** 500ms statt 150ms; mit Latest-Cancel-Counter um stale fires zu vermeiden.

---

### Modal

**Source:** `genossi-frontend/src/component/modal.rs`

**Apply to:** `assemblies.rs` (Create-Form), `assembly_details.rs` (Token-Create + Confirm-Dialogs für GV öffnen/schließen + Token-Revoke)

```rust
#[component]
pub fn Modal(props: ModalProps) -> Element {
    rsx! {
        div { class: "fixed inset-0 z-10 bg-black bg-opacity-50 flex justify-center items-center md:p-4",
            div { class: "bg-white w-full max-w-3/4 max-h-[90vh] p-8 overflow-y-auto rounded-lg shadow-lg",
                div { class: "", { props.children } }
            }
        }
    }
}
```

**Verwendung** (Vorlage `application_create_form.rs:30-50`): Header mit Title + Close-X, Inner-Form, Inline-Error-Box, Submit/Cancel-Buttons.

---

### Lifecycle-Cleanup (`use_drop`)

**Source:** `genossi-frontend/src/page/templates.rs:179-183`

**Apply to:** `qr_scanner.rs` (MediaStream stoppen)

```rust
use_drop(move || {
    if let Some(id) = editor_id.read().as_ref() {
        js::destroy_editor(id);
    }
});
```

**Phase-4-Variation:** statt `destroy_editor` → für jeden Track `track.stop()` ausführen (siehe RESEARCH.md §"Pattern 2").

---

### i18n-Konsumption

**Source:** `genossi-frontend/src/i18n/mod.rs:622-624` (`use_i18n`-Hook) + `application_list.rs:5,31` (Verwendung)

**Apply to:** ALLE neuen Components/Pages

```rust
use crate::i18n::{use_i18n, Key};

#[component]
pub fn SomePage() -> Element {
    let i18n = use_i18n();
    // ...
    rsx! {
        h1 { {i18n.t(Key::Applications)} }
    }
}
```

**Hinweis:** Helfer-View ist **deutsch fix** (D-19); aber das wird über das i18n-System gehandhabt — Plan-Discretion ob Locale lokal in HelperShell auf `Locale::De` festsetzen oder System-Default-Detection lassen.

---

### Spawn-Async-on-Event

**Source:** `genossi-frontend/src/component/application_create_form.rs:48-111` (Form-Submit)

**Apply to:** Alle Submit-Handler in Phase-4 (Helper-Login-Submit, Assembly-Create, Token-Create, Token-Revoke, Toggle-Click)

```rust
onsubmit: move |evt| {
    evt.prevent_default();
    spawn(async move {
        submitting.set(true);
        error.set(None);
        let config = CONFIG.read().clone();
        match api::create_application(&config, &request).await {
            Ok(_) => on_created.call(()),
            Err(e) => error.set(Some(format!("{}", e))),
        }
        submitting.set(false);
    });
}
```

---

## No Analog Found

Files with no close match in the codebase (Planner sollte RESEARCH.md-Patterns referenzieren):

| File | Role | Data Flow | Reason |
|------|------|-----------|--------|
| `component/connection_banner.rs` | sticky-warning-banner | event-driven | Kein sticky-top warning-Banner-Pattern in Codebase. Farb-Konvention von `error_alert.rs`, aber Layout (sticky, amber, sticky-top, print-hidden) ist neu. |
| `component/live_counter.rs` (polling-loop) | polling | streaming/polling | `use_future` mit Endless-Loop-Pattern existiert nicht in Codebase (alle bisherigen `use_effect`-Calls sind one-shot). RESEARCH.md §"Pattern 1" liefert die Vorlage. |
| `component/qr_scanner.rs` (Camera-Lifecycle) | streaming + JS-bridge | event-driven | `getUserMedia` + MediaStream-Lifecycle ist Greenfield. JS-Bridge-Pattern aus `js.rs` ist Vorlage; Camera-Stream selbst ist neu. RESEARCH.md §"QR-Scanner Integration Plan" liefert kompletten Lifecycle-Plan. |
| Print-CSS für QrCard (`input.css`) | print | render-only | `@media print`-Block neu — `print:hidden`-Klassen werden bereits in Codebase genutzt (z.B. `top_bar.rs:122`, `footer.rs:8`), aber kein `body * { visibility: hidden }`-Pattern. |

---

## Metadata

**Analog search scope:**
- `genossi-frontend/src/component/*.rs` (alle 25+ Component-Files)
- `genossi-frontend/src/page/*.rs` (alle 16 Page-Files)
- `genossi-frontend/src/api.rs`, `auth.rs`, `app.rs`, `router.rs`, `js.rs`
- `genossi-frontend/src/i18n/mod.rs`, `de.rs`
- `genossi-frontend/src/service/auth.rs`
- `genossi-frontend/Cargo.toml`, `tailwind.config.js`, `input.css`

**Files scanned:** ~50 Frontend-Files (Read + Grep)

**Pattern extraction date:** 2026-05-04

**Component-First-Konsistenz-Check:**
- 12 neue Components in `src/component/` ✓ (alle laut UI-SPEC §"Component Skeletons")
- 4 neue Pages in `src/page/` — komponieren nur, kein inline-RSX-Duplikat zwischen Helfer- und Vorstand-Anwesenheits-View ✓
- ATTN-06 erfüllt durch Reuse von `attendance_list.rs` + `attendance_search.rs` + `live_counter.rs` in beiden Pages ✓

**Critical Patterns Identified:**
- **Status-Badge-Mechanik 1:1 übertragbar:** `application_list.rs:7-27` ist exakte Vorlage für `assembly_status_badge.rs` (nur Enum + Farben anpassen)
- **API-CRUD-Pattern strikt einheitlich:** alle 12 neuen API-Funktionen folgen dem `api.rs:160-200`-Schema (info!-Log + URL-Build + reqwest + check_response + json)
- **Tab-Pattern muss extrahiert werden:** `applications_page.rs:78-103` zeigt das inline-Tab-Pattern, das in `tab_strip.rs` ausgelagert wird (Component-First)
- **`use_drop` ist im Codebase etabliert:** `templates.rs:179-183` ist die direkte Vorlage für `qr_scanner.rs`-MediaStream-Cleanup
- **Toast-Mechanik existiert, ist aber nicht extrahiert:** `members.rs:49-62` ist Pattern-Vorlage; Plan-Discretion ob in `component/toast.rs` ausgelagert (empfohlen, weil ≥3 Phase-4-Pages Toasts brauchen)

## PATTERN MAPPING COMPLETE
