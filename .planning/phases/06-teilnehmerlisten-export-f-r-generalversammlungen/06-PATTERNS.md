# Phase 6: Teilnehmerlisten-Export für Generalversammlungen — Pattern Map

**Mapped:** 2026-05-17
**Files analyzed:** 14 new/modified files
**Analogs found:** 13 / 14 (1 file — `teilnehmerliste.typ` — extends an existing pattern, no exact analog)

---

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `genossi_service/src/attendance_export.rs` | service-trait | request-response | `genossi_service/src/attendance.rs` | exact |
| `genossi_service_impl/src/attendance_export.rs` | service-impl | transform | `genossi_service_impl/src/attendance.rs` | exact |
| `genossi_service_impl/src/attendance_export/csv_writer.rs` (or inline) | utility (CSV transform) | transform | `genossi_service_impl/src/pdf_generation.rs::build_inputs_application` | role-match |
| `genossi_service_impl/src/attendance_export/xlsx_writer.rs` (or inline) | utility (XLSX transform) | transform | `genossi_service_impl/src/pdf_generation.rs::build_inputs` | role-match |
| `genossi_service_impl/src/attendance_export/pdf_writer.rs` (or inline) | utility (PDF inputs) | transform | `genossi_service_impl/src/pdf_generation.rs::build_inputs` | exact |
| `templates/teilnehmerliste.typ` | typst-template | file-render | `templates/join_confirmation.typ` + `templates/_layout.typ` | role-match (no list-table analog yet) |
| `genossi_rest/src/attendance_export.rs` | rest-handler | file-download | `genossi_rest/src/member_document.rs::download_document` + `genossi_rest/src/attendance.rs` | exact (composite) |
| `genossi_rest_types/src/lib.rs` (extension) | dto | n/a | existing `AttendanceMemberTO` block | exact |
| `genossi_rest/src/lib.rs` (router nest) | route-registration | n/a | `genossi_rest/src/lib.rs:602-618` (attendance routes) | exact |
| `genossi_bin/src/lib.rs` (DI wiring) | di-wiring | n/a | `genossi_bin/src/lib.rs:179-198, 644-657` (AttendanceServiceImpl) | exact |
| `genossi-frontend/src/api.rs` (new export fn) | client-api | file-download (blob) | `genossi-frontend/src/api.rs:506-548` (`render_template_pdf`) | exact |
| `genossi-frontend/src/page/assembly_details.rs` (new tab) | page-internal-component | event-driven | `genossi-frontend/src/page/assembly_details.rs:206-303` (`TokensTab`) | exact |
| `genossi-frontend/src/i18n/{mod,de,en}.rs` (new keys) | i18n | n/a | existing `Key::AssemblyTab*` enum variants | exact |
| `genossi_bin/tests/e2e_tests.rs` (new tests) | test | request-response | `genossi_bin/tests/e2e_tests.rs:9322-9530` (attendance E2E) | exact |

---

## Pattern Assignments

### `genossi_service/src/attendance_export.rs` (service-trait, request-response)

**Analog:** `genossi_service/src/attendance.rs` (lines 1-50, the trait + `AttendanceStats` domain type)

**Imports pattern** (lines 8-17 of analog):
```rust
use async_trait::async_trait;
use mockall::automock;
use std::fmt::Debug;
use std::sync::Arc;
use uuid::Uuid;

use genossi_dao::attendance::AttendanceMemberRow;

use crate::permission::Authentication;
use crate::ServiceError;
```

**Trait skeleton pattern** (lines 47-50 of analog — note the `#[automock]` attribute for Plan-6 test mocks):
```rust
#[automock(type Context=(); type Transaction = genossi_dao::MockTransaction;)]
#[async_trait]
pub trait AttendanceService {
    type Context: ...;
    type Transaction: ...;
    // methods returning Result<_, ServiceError>
}
```

**For Phase 6:** Add a domain type `AttendanceExport { bytes: Vec<u8>, content_type: &'static str, filename: String }` (mirrors the `AttendanceStats` domain struct lines 27-32) and a trait method signature like:
```rust
async fn export(
    &self,
    assembly_id: Uuid,
    format: ExportFormat,         // enum Csv|Pdf|Xlsx
    include: ExportInclude,       // enum All|Present
    context: Authentication<Self::Context>,
) -> Result<AttendanceExport, ServiceError>;
```

---

### `genossi_service_impl/src/attendance_export.rs` (service-impl, transform)

**Analog:** `genossi_service_impl/src/attendance.rs` (the entire file — same shape, same deps, swap `AttendanceService` → `AttendanceExportService`, add a stricter permission funnel)

**Imports + service-process const pattern** (lines 31-53 of analog):
```rust
use std::sync::Arc;
use async_trait::async_trait;
use uuid::Uuid;

use genossi_dao::assembly::{AssemblyDao, AssemblyEntity, AssemblyStatus};
use genossi_dao::assembly_member_snapshot::AssemblyMemberSnapshotDao;
use genossi_dao::attendance::{AttendanceDao, AttendanceMemberRow};
use genossi_dao::TransactionDao;

use genossi_service::permission::{Authentication, PermissionService};
use genossi_service::ServiceError;

use crate::gen_service_impl;

const ADMIN_PRIVILEGE: &str = "admin";
```

**`gen_service_impl!` macro pattern** (lines 55-64 of analog) — extends the underlying `gen_service_impl!` macro in `genossi_service_impl/src/macros.rs:1-41`:
```rust
gen_service_impl! {
    struct AttendanceServiceImpl: AttendanceService = AttendanceServiceDeps {
        AttendanceDao: AttendanceDao<Transaction = Self::Transaction> = attendance_dao,
        AssemblyDao: AssemblyDao<Transaction = Self::Transaction> = assembly_dao,
        MemberDao: MemberDao<Transaction = Self::Transaction> = member_dao,
        AssemblyMemberSnapshotDao: AssemblyMemberSnapshotDao<Transaction = Self::Transaction> = assembly_member_snapshot_dao,
        PermissionService: PermissionService<Context = Self::Context> = permission_service,
        TransactionDao: TransactionDao<Transaction = Self::Transaction> = transaction_dao,
    }
}
```

**Permission funnel pattern — adapted for Phase 6 strict Closed-only-Admin rule** (analog lines 79-115). The analog has 3 branches (`Full`, Helper, Admin). For Phase 6 we need only Admin + `Closed`-status-gate:
```rust
// genossi_service_impl/src/attendance.rs:79-115 (ANALOG to copy from)
async fn check_assembly_access(
    &self,
    assembly_id: Uuid,
    context: Authentication<Deps::Context>,
    tx: Deps::Transaction,
) -> Result<AssemblyEntity, ServiceError> {
    let assembly = self
        .assembly_dao
        .find_by_id(assembly_id, tx.clone())
        .await?
        .ok_or(ServiceError::EntityNotFound(assembly_id))?;

    match &context {
        Authentication::Full => Ok(assembly),
        Authentication::Context(ctx) => {
            if let Some(helper_aid) = ctx.as_helper() { /* helper branch */ }
            // Vorstand-branch via admin privilege.
            self.permission_service
                .check_permission(ADMIN_PRIVILEGE, context)
                .await?;
            Ok(assembly)
        }
    }
}
```

**Phase-6 modification:** drop the helper branch, add `if assembly.status != AssemblyStatus::Closed { return Err(ServiceError::Conflict(Arc::from("assembly_not_closed"))); }` after the admin check passes.

**Service-method body skeleton — `use_transaction → funnel → DAO → commit`** (analog lines 123-144, `list_members`):
```rust
async fn list_members(
    &self,
    assembly_id: Uuid,
    search: Option<String>,
    context: Authentication<Self::Context>,
) -> Result<Arc<[AttendanceMemberRow]>, ServiceError> {
    let tx = self.transaction_dao.use_transaction(None).await?;
    let _assembly = self
        .check_assembly_access(assembly_id, context, tx.clone())
        .await?;

    let rows = self
        .attendance_dao
        .list_members_for_assembly(assembly_id, search, tx.clone())
        .await?;

    self.transaction_dao.commit(tx).await?;
    Ok(rows)
}
```

**DAO call to reuse (no new DAO method needed):** `AttendanceDao::list_members_for_assembly(aid, None, tx)` — defined in `genossi_dao/src/attendance.rs:101-106`, implemented in `genossi_dao_impl_sqlite/src/attendance.rs:107-162` with the 7-field SELECT-whitelist already in place.

**Test-mock pattern** (analog lines 244-577) — copy the `TestTransaction`, `TestContext`, `mock! { pub TestAttendanceDao { ... } }`, `mock! { pub TestAssemblyDao { ... } }`, `mock! { pub TestPermissionService { ... } }`, `TestDeps`, `build_service()`, `tx_dao_with_commit()`, `tx_dao_no_commit()` infrastructure verbatim. The pattern is non-trivial (~330 lines of fixtures) — copy-then-adapt is faster than re-deriving.

---

### `templates/teilnehmerliste.typ` (typst-template, file-render)

**Analog (partial):** `templates/join_confirmation.typ` (lines 1-49) for the input-decoding + layout-include pattern; `templates/_layout.typ` (lines 1-39) for the `#let letter(...)` helper.

**Input-decoding pattern** (analog `join_confirmation.typ:1-17`):
```typst
// Available variables (via sys.inputs):
//   member.first_name, ... today
#import "_layout.typ": letter

#let member = json.decode(sys.inputs.at("member"))
#let today = sys.inputs.at("today")

#show: letter.with(
  title: "Beitrittsbestätigung",
  date: today,
)
```

**Layout-wrapper pattern** (`_layout.typ:5-38`) — the project uses a single `letter()` helper with A4-page, 11pt Liberation Sans, German lang setup:
```typst
#let letter(
  title: none,
  date: none,
  content,
) = {
  set page(paper: "a4", margin: (top: 3cm, bottom: 2.5cm, left: 2.5cm, right: 2cm))
  set text(font: "Liberation Sans", size: 11pt, lang: "de")
  set par(leading: 0.8em, justify: true)
  if date != none { align(right)[#date]; v(1cm) }
  if title != none { text(size: 14pt, weight: "bold")[#title]; v(0.5cm) }
  content
}
```

**Table pattern (no exact analog — `join_confirmation.typ:31-38` is a 2-column key-value table, NOT a list-table with repeating header).** Researcher recommendation: use `table.header(repeat: true)` for multi-page list. See `06-RESEARCH.md` §"Pattern 2: Typst-Template" for the verbatim 6-column-table excerpt. Pattern reference: typst.app/docs/reference/model/table/.

**Inputs the Service must build** — analog `pdf_generation.rs::build_inputs_application` (lines 256-359) and `build_inputs` (lines 361-505) — serialize a dict to JSON string and pass via `inputs.insert(Str::from("key"), Value::Str(...))`. For Phase 6 the keys will be `meta` (GV-title, GV-date, present-count, total-count) and `rows` (array of `{member_number, first_name, last_name, salutation, title, is_present}`).

---

### `genossi_rest/src/attendance_export.rs` (rest-handler, file-download)

**Analog (composite):**
1. `genossi_rest/src/attendance.rs` — handler skeleton with `#[instrument]`, `#[utoipa::path]`, `error_handler((async { ... }).await)`, `extract_auth_context`, OpenAPI route-doc, `generate_route()` function.
2. `genossi_rest/src/member_document.rs::download_document` (lines 232-267) — the file-download response with `Content-Disposition` + `Content-Type` + `Body::from(bytes)`.
3. `genossi_rest/src/http_util.rs:43-50` — the `content_disposition_attachment(filename)` RFC-6266 helper.

**Imports pattern** (`attendance.rs:23-40`):
```rust
use std::sync::Arc;

use axum::{
    body::Body,
    extract::{Path, Query, State},
    response::Response,
    routing::{get, put},
    Extension, Router,
};
use genossi_rest_types::{AttendanceMemberTO, AttendanceStatsTO};
use genossi_service::attendance::AttendanceService;
use genossi_service::ServiceError;
use serde::Deserialize;
use tracing::instrument;
use utoipa::{IntoParams, OpenApi, ToSchema};
use uuid::Uuid;

use crate::{error_handler, Context, RestError, RestStateDef};
```

**RestState-trait + dependency-getter pattern** (`attendance.rs:42-45`):
```rust
pub trait AttendanceRestState: Clone + Send + Sync + 'static {
    type AttendanceService: AttendanceService<Context = crate::ContextType> + Send + Sync + 'static;
    fn attendance_service(&self) -> Arc<Self::AttendanceService>;
}
```

For Phase 6: `AttendanceExportRestState` trait with `attendance_export_service()` getter.

**Differential error mapper pattern** (`attendance.rs:52-57`) — Phase 6 needs this too because D-13 says Vorstand-only and we want PermissionDenied → 403 (not 401):
```rust
fn map_attendance_error(e: ServiceError) -> RestError {
    match e {
        ServiceError::PermissionDenied => RestError::Forbidden("forbidden".to_string()),
        other => other.into(),
    }
}
```

For Phase 6: `ServiceError::Conflict(_)` → `RestError::Conflict(_)` (D-11: 409 for non-Closed assembly) is delegated to global `From<ServiceError>` in `genossi_rest/src/lib.rs`. Verify this mapping exists before relying on it.

**Query-parameter struct pattern** (`attendance.rs:63-68`):
```rust
#[derive(Debug, Deserialize, IntoParams, ToSchema)]
pub struct ListMembersQuery {
    #[serde(default)]
    pub q: Option<String>,
}
```

For Phase 6: `ExportQuery { #[serde(default)] include: ExportInclude }` with `ExportInclude { All, Present }` enum (default `All`).

**File-download response pattern** (`member_document.rs:236-266`):
```rust
pub async fn download_document<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path((member_id, document_id)): Path<(Uuid, Uuid)>,
) -> Response {
    error_handler(
        (async {
            let (doc, _) = rest_state
                .member_document_service()
                .download(member_id, document_id, crate::extract_auth_context(Some(context))?, None)
                .await?;

            let data = rest_state
                .document_storage()
                .load(&doc.relative_path)
                .await
                .map_err(|e| RestError::InternalError(format!("Failed to load file: {}", e)))?;

            let content_disposition =
                crate::http_util::content_disposition_attachment(&doc.file_name);

            Ok(Response::builder()
                .status(200)
                .header("Content-Type", doc.mime_type.as_ref())
                .header("Content-Disposition", &content_disposition)
                .body(Body::from(data))
                .unwrap())
        })
        .await,
    )
}
```

**For Phase 6:** Compose with the `attendance.rs::list_attendance_members` shape (`Path<Uuid>` + `Query<...>`) and the `download_document` body shape. The handler:
1. Extracts `Path<(assembly_id, format)>` and `Query<ExportQuery>`.
2. Calls `attendance_export_service().export(aid, format, include, auth)` — returns `AttendanceExport { bytes, content_type, filename }`.
3. Builds the response with `content_disposition_attachment(&export.filename)` and `Content-Type: export.content_type`, body `Body::from(export.bytes)`.

**Route-builder pattern** (`attendance.rs:227-244`):
```rust
pub fn generate_attendance_route<RestState: RestStateDef + AttendanceRestState>(
) -> Router<RestState> {
    Router::new()
        .route("/members", get(list_attendance_members::<RestState>))
        .route(
            "/{member_id}",
            put(mark_attendance_present::<RestState>).delete(mark_attendance_absent::<RestState>),
        )
}
```

For Phase 6: `generate_export_route()` with `.route("/attendance-export/{format}", get(export_attendance::<RestState>))`.

**OpenAPI registration pattern** (`attendance.rs:246-257`):
```rust
#[derive(OpenApi)]
#[openapi(
    paths(list_attendance_members, mark_attendance_present, mark_attendance_absent, get_assembly_stats),
    components(schemas(AttendanceMemberTO, AttendanceStatsTO, ListMembersQuery)),
    tags((name = "Attendance", description = "..."))
)]
pub struct ApiDoc;
```

---

### `genossi_rest/src/lib.rs` (router nest, integration)

**Analog:** `genossi_rest/src/lib.rs:602-618` — the existing attendance route registrations follow the exact pattern Phase 6 needs (path-prefix `/api/assembly/{assembly_id}`):

```rust
// genossi_rest/src/lib.rs:602-618
.nest("/api/assembly", assembly::generate_route::<RestState>())
.nest(
    "/api/assembly/{assembly_id}/helper-tokens",
    helper_token::generate_route::<RestState>(),
)
.nest(
    "/api/assembly/{assembly_id}",
    attendance::generate_stats_route::<RestState>(),
)
.nest(
    "/api/attendance/{assembly_id}",
    attendance::generate_attendance_route::<RestState>(),
)
```

For Phase 6: insert `.nest("/api/assembly/{assembly_id}", attendance_export::generate_export_route::<RestState>())` near the existing assembly-namespaced routes.

Also extend the `create_app<RestState: ... >()` trait bound at `lib.rs:435` with `+ attendance_export::AttendanceExportRestState`.

Also extend the OpenAPI registry at `lib.rs:251-272` with `(path = "/api/assembly/{assembly_id}/attendance-export", api = attendance_export::ApiDoc)`.

Also add `pub mod attendance_export;` at the top (current module list at `lib.rs:3`).

---

### `genossi_bin/src/lib.rs` (DI wiring)

**Analog:** `genossi_bin/src/lib.rs:179-198` (the `AttendanceServiceDependencies` deps struct + type alias) and `lib.rs:644-657` (the actual `Arc::new(AttendanceServiceImpl { ... })` construction inside `RestStateImpl::new()`).

**Deps-struct pattern** (lines 179-198):
```rust
// Phase 3 Plan 06: AttendanceServiceImpl wiring (D-23). Six deps —
// deliberately NO UuidService and NO AuditLogDao (D-08, ATTN-05).
pub struct AttendanceServiceDependencies;

unsafe impl Send for AttendanceServiceDependencies {}
unsafe impl Sync for AttendanceServiceDependencies {}

impl genossi_service_impl::attendance::AttendanceServiceDeps for AttendanceServiceDependencies {
    type Context = Context;
    type Transaction = Transaction;
    type AttendanceDao = AttendanceDao;
    type AssemblyDao = AssemblyDao;
    type MemberDao = MemberDao;
    type AssemblyMemberSnapshotDao = AssemblyMemberSnapshotDao;
    type PermissionService = PermissionService;
    type TransactionDao = TransactionDao;
}

type AttendanceService =
    genossi_service_impl::attendance::AttendanceServiceImpl<AttendanceServiceDependencies>;
```

**Construction pattern in `RestStateImpl::new()`** (lines 644-657):
```rust
// Phase 3 Plan 06 (D-23): AttendanceServiceImpl with 6 deps —
// AttendanceDao, AssemblyDao, MemberDao, AssemblyMemberSnapshotDao,
// PermissionService, TransactionDao. No UuidService, no AuditLogDao
// (D-08, ATTN-05 — attendance is not audited).
let attendance_dao = Arc::new(AttendanceDao::new(pool.clone()));
let attendance_service =
    Arc::new(genossi_service_impl::attendance::AttendanceServiceImpl {
        attendance_dao,
        assembly_dao: assembly_dao.clone(),
        member_dao: member_dao.clone(),
        assembly_member_snapshot_dao,
        permission_service: permission_service.clone(),
        transaction_dao: transaction_dao.clone(),
    });
```

**For Phase 6:** Add `AttendanceExportServiceDependencies` struct + impl + type-alias right after `AttendanceServiceDependencies`. Construct an `Arc::new(AttendanceExportServiceImpl { ... })` right after the existing `attendance_service` construction at line 657. Then add `attendance_export_service` to the `Self { ... }` struct-builder at line 732.

**Caveat:** `assembly_member_snapshot_dao` is _moved_ into the existing `attendance_service` construction. Phase 6 needs to clone it (see line 633 comment `// Phase 3 Plan 06: cloned (not moved) so AttendanceServiceImpl below can share the same Arc.`). Repeat that idiom — change line 654's `assembly_member_snapshot` to `assembly_member_snapshot_dao.clone()` (verify the variable name in current code first).

---

### `genossi-frontend/src/api.rs` (client-api, blob download)

**Analog:** `genossi-frontend/src/api.rs:506-548` (`render_template_pdf`) — exact blob-fetch-pattern needed for Phase 6.

**Full blob-download pattern** (lines 506-548):
```rust
pub async fn render_template_pdf(
    config: &Config,
    path: &str,
    member_id: Uuid,
) -> Result<String, AppError> {
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;

    let url = template_render_url(config, path, member_id);
    info!("Rendering template PDF: {url}");

    let mut opts = web_sys::RequestInit::new();
    opts.set_method("POST");

    let request = web_sys::Request::new_with_str_and_init(&url, &opts)
        .map_err(|e| AppError::new(None, "Verbindungsfehler", Some(format!("{:?}", e))))?;

    let window = web_sys::window()
        .ok_or_else(|| AppError::new(None, "Verbindungsfehler", None))?;
    let resp_value = JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| AppError::new(None, "Verbindungsfehler", Some(format!("{:?}", e))))?;

    let resp: web_sys::Response = resp_value
        .dyn_into()
        .map_err(|_| AppError::new(None, "Verbindungsfehler", None))?;

    if !resp.ok() {
        return Err(map_web_response_error(&resp).await);
    }

    let blob = JsFuture::from(resp.blob().unwrap())
        .await
        .map_err(|e| AppError::new(None, "Verbindungsfehler", Some(format!("{:?}", e))))?;

    let blob: web_sys::Blob = blob
        .dyn_into()
        .map_err(|_| AppError::new(None, "Verbindungsfehler", None))?;

    let blob_url = web_sys::Url::create_object_url_with_blob(&blob)
        .map_err(|e| AppError::new(None, "Verbindungsfehler", Some(format!("{:?}", e))))?;

    Ok(blob_url)
}
```

**For Phase 6:** Copy this verbatim. Method changes from `POST` to `GET`. URL is `format!("{}/api/assembly/{aid}/attendance-export/{format}?include={include}", config.backend, ...)`. Return signature stays `Result<String, AppError>` — the caller builds an `<a download="...">` and clicks it.

**Trigger-download via anchor click** (the SPEC at `06-UI-SPEC.md` §"Success Path" describes this, but no existing analog in the codebase — the existing pattern returns the blob URL and the caller is expected to handle the `<a download>` click). For Phase 6 the page-component can either (a) return the URL and let the page build/click the anchor, or (b) build the anchor inside `api.rs` via `web-sys` `HtmlAnchorElement`. Recommendation: keep the api.rs shape (return URL) so the analog stays clean; the page-component does the `document.create_element("a")` + `set_download(...)` + `.click()` flow.

**Existing read-only API helpers next to the new one** — pattern at `api.rs:1803-1862` (`list_attendance_members`, `mark_present`, `mark_absent`, `get_assembly_stats`). The new `export_attendance` fn goes right after `get_assembly_stats` to keep all attendance-related API in one block.

---

### `genossi-frontend/src/page/assembly_details.rs` (page-internal-component, event-driven)

**Analog:** `genossi-frontend/src/page/assembly_details.rs:206-303` (`TokensTab`) — a page-internal smart wrapper that owns local signals + fetch + spawn pattern. UI-SPEC explicitly chose this analog (D-20, §"Component Decision").

**Page-internal component pattern** (lines 211-303):
```rust
#[component]
fn TokensTab(assembly_id: Uuid, on_error: EventHandler<String>) -> Element {
    let i18n = use_i18n();
    let mut tokens = use_signal(Vec::<HelperTokenTO>::new);
    let mut loading = use_signal(|| true);
    let mut show_create = use_signal(|| false);
    // ...

    let load = move || {
        spawn(async move {
            loading.set(true);
            let config = CONFIG.read().clone();
            match api::list_helper_tokens(&config, assembly_id).await {
                Ok(list) => tokens.set(list),
                Err(e) => on_error.call(e.message),
            }
            loading.set(false);
        });
    };

    use_effect(move || { load(); });

    rsx! {
        div { class: "flex justify-between items-start mb-4",
            h2 { class: "text-xl font-semibold", "{i18n.t(Key::HelperTokens)}" }
            button {
                r#type: "button",
                class: "bg-blue-600 hover:bg-blue-700 text-white px-4 py-2 rounded min-h-[44px]",
                onclick: move |_| show_create.set(true),
                "{i18n.t(Key::HelperTokenCreate)}"
            }
        }
        // ...
    }
}
```

**Tab registration pattern** (lines 82-138) — Phase 6 adds a 4th tab conditional on `assembly.status == Closed`:
```rust
// In AssemblyDetails(): build tab_defs based on status.
let tab_defs = if matches!(a.status, AssemblyStatusTO::Closed) {
    vec![
        TabDef { key: "basics", label: i18n.t(Key::AssemblyTabBasics).to_string() },
        TabDef { key: "tokens", label: i18n.t(Key::AssemblyTabTokens).to_string() },
        TabDef { key: "attendance", label: i18n.t(Key::AssemblyTabAttendance).to_string() },
        TabDef { key: "export", label: i18n.t(Key::AssemblyTabExport).to_string() },
    ]
} else {
    // existing 3-tab list
};
```

**For Phase 6:** Define an `#[component] fn ExportTab(assembly: AssemblyTO, on_error: EventHandler<String>) -> Element` below `TokensTab` (i.e., between line 303 and the EOF). Local signals: `selected_format: Signal<String>` (default `"pdf"`), `selected_include: Signal<String>` (default `"all"`), `submitting: Signal<bool>`. On submit: `spawn(async move { ... api::export_attendance(...) ... build <a download>, click, revoke URL })`.

**Submit-button styling** (analog `assembly_details.rs:238-243`):
```rust
button {
    r#type: "button",
    class: "bg-blue-600 hover:bg-blue-700 text-white px-4 py-2 rounded min-h-[44px]",
    onclick: move |_| { ... },
    "{i18n.t(Key::SomeLabel)}"
}
```

The UI-SPEC `06-UI-SPEC.md` §"Component Inventory" (lines 214-238) is the canonical class-list — copy from there, the page-component should match exactly.

---

### `genossi_bin/tests/e2e_tests.rs` (E2E tests)

**Analog:** `genossi_bin/tests/e2e_tests.rs:9322-9530` — the entire Phase-3 attendance E2E test block, especially the `create_open_assembly_with_members` helper.

**Setup-server pattern** (lines 24-38):
```rust
async fn setup() -> genossi_rest::test_server::test_support::TestServer {
    let pool = Arc::new(
        SqlitePool::connect("sqlite::memory:")
            .await
            .expect("Failed to create in-memory database"),
    );

    sqlx::migrate!("../migrations/sqlite")
        .run(&*pool)
        .await
        .expect("Failed to run migrations");

    let rest_state = RestStateImpl::new(pool);
    start_test_server(rest_state).await
}
```

**Test-helper to seed a Closed assembly with members** — extend `create_open_assembly_with_members` (lines 9322-9355) with a second variant `create_closed_assembly_with_members(client, server, n_members)` that POSTs to `/api/assembly/{aid}/close` after opening. Pattern for close: see line 9440 `client.post(server.url(&format!("/api/assembly/{}/close", assembly_id)))`.

**E2E test shape — file-download response assertion** — no current analog (Phase-3 endpoints all return JSON). Pattern reference is the request-response shape at lines 9361-9397 with these adaptations for binary response:
```rust
let resp = client.get(server.url(&format!("/api/assembly/{}/attendance-export/pdf?include=all", aid))).send().await.unwrap();
assert_eq!(resp.status(), StatusCode::OK);
assert_eq!(
    resp.headers().get("content-type").unwrap(),
    "application/pdf"
);
let cd = resp.headers().get("content-disposition").unwrap().to_str().unwrap();
assert!(cd.contains("filename=\"gv-"));
assert!(cd.contains("teilnehmer.pdf\""));
let bytes = resp.bytes().await.unwrap();
assert!(bytes.starts_with(b"%PDF"), "must be a PDF magic-number");
```

**Required tests for Phase 6 (mirror Phase-3 coverage):**
1. PDF/CSV/XLSX format paths return 200 + correct Content-Type + Content-Disposition + non-empty body.
2. CSV body starts with UTF-8 BOM `0xEF 0xBB 0xBF`.
3. CSV uses `;` separator (decode + assert).
4. Export against `Preparation`-status assembly returns 409.
5. Export against `Open`-status assembly returns 409.
6. `?include=present` filters to anwesende only.
7. Default `?include` is `all` (omit query param).
8. PDF body starts with `%PDF` magic bytes; XLSX body starts with `PK\x03\x04` (ZIP magic).
9. Filename schema: `gv-YYYY-MM-DD-teilnehmer.{ext}` matches expectations.

---

## Shared Patterns

### Authentication / Authorization

**Source:** `genossi_rest/src/lib.rs` (`extract_auth_context`, `forbid_unauthenticated`, `context_extractor` middleware applied at lines 629-636); `genossi_service_impl/src/attendance.rs:79-115` (the access-funnel pattern).

**Apply to:** All new REST handlers in `attendance_export.rs` and the service-impl in `attendance_export.rs`.

```rust
// genossi_rest layer:
let auth = crate::extract_auth_context(Some(context))?;

// genossi_service_impl layer (inside the service method):
let tx = self.transaction_dao.use_transaction(None).await?;
let _assembly = self.check_assembly_access(assembly_id, context, tx.clone()).await?;
// ... DAO calls ...
self.transaction_dao.commit(tx).await?;
```

### Error Handling

**Source:** `genossi_rest/src/lib.rs` (`error_handler`, `RestError`, `From<ServiceError>` impl); `genossi_rest/src/attendance.rs:52-57` (differential mapping for 403).

**Apply to:** All new REST handlers.

```rust
// Wrap all handler logic:
error_handler(
    (async {
        // ... service calls + response building ...
        Ok(Response::builder().status(200)...)
    })
    .await,
)
```

**Status-code mapping for Phase 6:**
- `ServiceError::EntityNotFound(_)` → 404 (global mapping)
- `ServiceError::Conflict(_)` → 409 (verify global mapping — D-11)
- `ServiceError::PermissionDenied` → 403 (use the local `map_attendance_error` pattern; D-13)
- `ServiceError::Unauthorized` / session-related → 401 (global mapping)

### Logging / Tracing

**Source:** `#[instrument(skip(rest_state))]` attribute applied to every REST handler in `genossi_rest/src/attendance.rs` (lines 70, 113, 148, 183).

**Apply to:** Every new REST handler in `attendance_export.rs`.

```rust
#[instrument(skip(rest_state))]
#[utoipa::path(get, tag = "AttendanceExport", path = "/attendance-export/{format}", ...)]
pub async fn export_attendance<RestState: ...>(...) -> Response { ... }
```

**Additionally for D-18** (info-log with `gv_id`, `format`, `include`, `user_id`): use `tracing::info!(target: "attendance_export", aid = %assembly_id, format = ?format, include = ?include, user = ?user_id, "export_attendance");` once at the start of the service method (after the user-id has been resolved). Pattern reference: `genossi_service_impl/src/attendance.rs:169-173` (`current_user_id` resolution).

### Filename Sanitation + Content-Disposition

**Source:** `genossi_rest/src/http_util.rs:43-50` (`content_disposition_attachment` — RFC-6266 with ASCII fallback + UTF-8 percent-encoding).

**Apply to:** The Phase-6 export handler when building the response.

```rust
let content_disposition = crate::http_util::content_disposition_attachment(&export.filename);
Response::builder()
    .status(200)
    .header("Content-Type", export.content_type)
    .header("Content-Disposition", &content_disposition)
    .body(Body::from(export.bytes))
    .unwrap()
```

Phase-6 filename is `gv-YYYY-MM-DD-teilnehmer.{ext}` — pure ASCII per D-15, so the percent-encoded UTF-8 part is identical to the ASCII fallback. No surprises.

### Test Mocking (Service-Layer)

**Source:** `genossi_service_impl/src/attendance.rs:244-577` — the entire test fixture block.

**Apply to:** Unit tests in `genossi_service_impl/src/attendance_export.rs`. Copy: `TestTransaction` (lines 255-269), `TestContext` (lines 274-287), `mock! { pub TestTxDao { ... } }` (lines 289-301), `mock! { pub TestAssemblyDao { ... } }` (lines 303-314), `mock! { pub TestAttendanceDao { ... } }` (lines 316-354), `mock! { pub TestPermissionService { ... } }` (lines 414-507), `TestDeps` (lines 510-521), `build_service(...)` helper (lines 540-555), `tx_dao_with_commit()` / `tx_dao_no_commit()` (lines 557-577).

**Required test cases for Phase 6:**
1. `Authentication::Full` bypasses status-check → reaches DAO.
2. Non-admin Context → `PermissionDenied`.
3. Assembly status `Preparation` → `Conflict("assembly_not_closed")`.
4. Assembly status `Open` → `Conflict("assembly_not_closed")`.
5. Assembly status `Closed` + admin → success path returns `AttendanceExport`.
6. `EntityNotFound(aid)` when assembly does not exist.
7. `include=Present` filters rows where `is_present=false`.
8. Each format (CSV/PDF/XLSX) produces the correct `content_type` and a non-empty `bytes` Vec.
9. CSV body starts with the BOM `[0xEF, 0xBB, 0xBF]`.
10. Filename matches `gv-{date}-teilnehmer.{ext}` for each format.

### i18n (Translation Keys)

**Source:** `genossi-frontend/src/i18n/mod.rs` (Key enum), `de.rs` + `en.rs` (translation tables).

**Apply to:** Every new UI string from `06-UI-SPEC.md` §"Copywriting Contract" (lines 99-138). Add `Key::AssemblyTabExport`, `Key::AttendanceExportHeading`, `Key::AttendanceExportSubheading`, ... (full key-list in the UI-SPEC).

Both `de.rs` and `en.rs` MUST be updated together (project-rule from `genossi-frontend/CLAUDE.md` lines 76-82).

---

## No Analog Found

| File | Role | Data Flow | Reason |
|------|------|-----------|--------|
| `templates/teilnehmerliste.typ` (list-table section) | typst-template | file-render | Existing templates (`join_confirmation.typ`, `zahlungsanfrage.typ`) use 2-column key-value tables, NOT multi-row lists with repeating header. Researcher provides the new pattern (`table.header(repeat: true)`) in `06-RESEARCH.md` §"Pattern 2". The layout-wrapper (`#import "_layout.typ": letter`) and the JSON-input-decoding pattern ARE analogs. |

**Recommendation for planner:** Use `06-RESEARCH.md` §"Pattern 2: Typst-Template" as the verbatim template-body source for the new file. The `_layout.typ` import + `json.decode(sys.inputs.at(...))` lines come from `join_confirmation.typ`.

---

## Metadata

**Analog search scope:**
- `genossi_dao/`, `genossi_dao_impl_sqlite/` — DAO traits and SQLite implementations
- `genossi_service/`, `genossi_service_impl/` — service traits and impls
- `genossi_rest/`, `genossi_rest_types/` — REST handlers, TOs, http-util, lib.rs router
- `genossi_bin/` — DI wiring + e2e tests
- `genossi-frontend/` — Dioxus pages, components, api, i18n
- `templates/` — existing Typst templates

**Files scanned (concrete reads):**
- `genossi_dao/src/attendance.rs` (full, 180 lines)
- `genossi_dao_impl_sqlite/src/attendance.rs` (full, 584 lines)
- `genossi_service_impl/src/attendance.rs` (full, 1140 lines)
- `genossi_rest/src/attendance.rs` (full, 300 lines)
- `genossi_rest/src/member_document.rs` (lines 220-310 — download handler)
- `genossi_rest/src/http_util.rs` (full, 175 lines)
- `genossi_service_impl/src/pdf_generation.rs` (lines 1-505 — PdfGenerator, build_inputs)
- `templates/_layout.typ` (full)
- `templates/join_confirmation.typ` (full)
- `templates/zahlungsanfrage.typ` (full)
- `genossi_service_impl/src/macros.rs` (full, 41 lines)
- `genossi_rest_types/src/lib.rs` (lines 1690-1815 — AttendanceMemberTO + tests)
- `genossi_rest/src/lib.rs` (lines 400-650 — router + middleware)
- `genossi_bin/src/lib.rs` (lines 175-300 + 630-760 — DI wiring)
- `genossi-frontend/src/api.rs` (lines 500-605 + 1800-1865 — blob download + attendance)
- `genossi-frontend/src/page/assembly_details.rs` (full, 303 lines)
- `genossi_bin/tests/e2e_tests.rs` (lines 1-60 + 9300-9530 — setup + attendance E2E)

**Skipped (too tangential):**
- `genossi_rest/src/template.rs`, `genossi_rest/src/static_document.rs` — file-download patterns that don't fit cleanly (template uses POST + dynamic-render, static_document uses different storage abstraction). The `member_document.rs::download_document` analog is closer.
- Frontend components in `genossi-frontend/src/component/` — Phase 6 keeps the export-block inline per UI-SPEC D-20.

**Pattern extraction date:** 2026-05-17
