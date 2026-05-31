# Phase 11: Export (PDF) — Pattern Map

**Mapped:** 2026-05-31
**Files analyzed:** 10 (5 NEW + 5 MODIFIED)
**Analogs found:** 10 / 10 (alle EXACT-Matches — Phase 6 ist 1:1-Vorbild)

> **Lese-Hinweis für den Planner:** RESEARCH.md hat bereits ausführliche `file:line`-Excerpts (Patterns 1–4, Q1, Q3, Q6–Q15). Dieses Dokument **dupliziert nicht** — es liefert eine kompakte Mapping-Tabelle "Phase-11-Datei → Phase-6-Analog → Pattern → Excerpt-Anker" plus Cross-Cutting-Pattern-Übersicht. Für tiefere Excerpts → RESEARCH.md §"Code Examples".

---

## File Classification

| Phase-11-Datei | Status | Rolle | Datenfluss | Closest Analog | Match Quality |
|---|---|---|---|---|---|
| `templates/defaults/auszahlungsliste.typ` | NEW | template | render (Typst→PDF) | `templates/defaults/teilnehmerliste.typ` | exact |
| `genossi_service/src/repayment_export.rs` | NEW | service-trait + domain types | request-response | `genossi_service/src/attendance_export.rs` | exact |
| `genossi_service_impl/src/repayment_export.rs` | NEW | service-impl + permission funnel | request-response (read-only) | `genossi_service_impl/src/attendance_export.rs` | exact |
| `genossi_rest/src/repayment_export.rs` | NEW | REST-handler + OpenAPI | request-response | `genossi_rest/src/attendance_export.rs` | exact |
| `genossi_bin/tests/e2e_tests.rs` (Append) | NEW (test fns) | E2E-test | HTTP-client | `e2e_tests.rs:10128..10570` (Phase 6) + `10578..11150` (Phase 9/10 helpers) | exact |
| `genossi_service_impl/src/template_storage.rs` | MODIFY | config-registry | static-data | bestehende Einträge `template_storage.rs:10-27` | exact (additiv) |
| `genossi_service_impl/src/pdf_generation.rs` | MODIFY | renderer | transform (Entity→Bytes) | `PdfGenerator::render_attendance_list` `pdf_generation.rs:279-336` + `build_inputs_attendance` `:602-652` | exact |
| `genossi_service/src/lib.rs` + `genossi_service_impl/src/lib.rs` + `genossi_rest/src/lib.rs` (module-deklaration) | MODIFY | module-root | re-export | jeweils `pub mod attendance_export;` Zeilen | exact (additiv) |
| `genossi_rest/src/lib.rs::create_app` | MODIFY | router + ApiDoc | request-response | `lib.rs:271-275` (ApiDoc) + `lib.rs:440-444` (bounds) + `lib.rs:640-643` (mount) + `lib.rs:763-767` (bounds duplicate) | exact (additiv) |
| `genossi_bin/src/lib.rs::RestStateImpl` | MODIFY | DI-wiring | dependency-injection | `bin/lib.rs:266-289` (Deps-Trait), `:520-522` (Feld), `:815-830` (Construction), `:918-922` (Insertion), `:1460-1468` (RestState-Impl) | exact (additiv) |

---

## Pattern Assignments

### 1. `templates/defaults/auszahlungsliste.typ` (NEW, template, render)

**Analog:** `templates/defaults/teilnehmerliste.typ`

**Vorbild-Excerpt** (siehe RESEARCH.md §Pattern 4):
- `_layout.typ`-Import: `teilnehmerliste.typ:7`
- `json.decode(sys.inputs.at("..."))`-Pattern: `teilnehmerliste.typ:9-10`
- `letter`-Show-Rule: `teilnehmerliste.typ:12-15`
- Repeat-Header-Tabelle mit `table.header(repeat: true, ...)`: `teilnehmerliste.typ:27-43`
- `..rows.map(r => (...)).flatten()`-Spread-Pattern: `teilnehmerliste.typ:35-42`

**Anpassungen für Phase 11:**
- 6 Spalten statt 6 (anderer Schema): `[*Nr.*], [*Name*], [*IBAN*], [*Anteile*], [*Betrag*], [*Verwendungszweck*]`
- `align: (right, left, left, right, right, left)`, `columns: (auto, 1fr, auto, auto, auto, 1fr)` (siehe CONTEXT D-Discretion "Typst-Template-Layout")
- meta-Felder: `fiscal_year`, `row_count` (statt `present`/`total`)
- Konkretes Template-Skelett ist in RESEARCH.md §Pattern 4 (Z. 318-360) vollständig ausformuliert

### 2. `genossi_service/src/repayment_export.rs` (NEW, service-trait, request-response)

**Analog:** `genossi_service/src/attendance_export.rs` (158 LOC, 1:1-Vorbild)

**Imports-Pattern** (`attendance_export.rs:17-23`):
```rust
use async_trait::async_trait;
use mockall::automock;
use std::fmt::Debug;
use uuid::Uuid;

use crate::permission::Authentication;
use crate::ServiceError;
```

**Bundle-Struct mit manuellem Debug** (`attendance_export.rs:58-72`) — **wichtig wegen `bytes_len`-Debug-Pattern (Pitfall #6)**:
```rust
pub struct AttendanceExport {
    pub bytes: Vec<u8>,
    pub content_type: &'static str,
    pub filename: String,
}
impl std::fmt::Debug for AttendanceExport { /* druckt bytes_len, nicht bytes */ }
```
→ Phase 11: 1:1 als `RepaymentExport` kopieren.

**ExportInclude-Enum mit `Default`-Impl** (`attendance_export.rs:34-49`):
```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExportInclude { All, Present }

impl Default for ExportInclude { fn default() -> Self { ExportInclude::All } }
```
→ Phase 11: `enum ExportInclude { Open, All, Paid }` + `Default = Open` (D-03).

**Trait mit `#[automock]`** (`attendance_export.rs:74-96`):
```rust
#[automock(type Context=(); type Transaction = genossi_dao::MockTransaction;)]
#[async_trait]
pub trait AttendanceExportService {
    type Context: Clone + Debug + Send + Sync + 'static;
    type Transaction: genossi_dao::Transaction;
    async fn export(&self, assembly_id: Uuid, format: ExportFormat,
                    include: ExportInclude, context: Authentication<Self::Context>)
        -> Result<AttendanceExport, ServiceError>;
}
```
→ Phase 11: Signatur identisch, nur `assembly_id` → `phase_id`, Bundle-Typ wechselt.

**ExportFormat-Enum** (`attendance_export.rs:27-31`): Phase 11 reduziert auf **nur `Pdf`** (D-12).

**Unit-Test-Pattern**: `attendance_export.rs:98-156` zeigt 5 kleine Tests (Default-Check, Varianten-Count, Bundle-Konstruktion, Automock-Konstruktion) — direkt für Phase 11 mit angepassten Enum-Varianten übernehmen.

---

### 3. `genossi_service_impl/src/repayment_export.rs` (NEW, service-impl, request-response read-only)

**Analog:** `genossi_service_impl/src/attendance_export.rs` (1199 LOC — Phase 11 wird ~30-40% davon, weil XLSX+CSV+PII-Whitelist+Helper-Branch entfallen)

**Konstanten** (`attendance_export.rs:51-58`):
```rust
const ADMIN_PRIVILEGE: &str = "admin";
const EXPORT_TARGET: &str = "attendance_export";
```
→ Phase 11: `EXPORT_TARGET = "repayment_export"`, ADMIN_PRIVILEGE bleibt `"admin"` (CONTEXT Discretion: "Permission-Privilege-String: Phase 6 verwendet `admin`").

**Deps-Trait** (`attendance_export.rs:60-72`):
```rust
pub trait AttendanceExportServiceDeps: Send + Sync + 'static {
    type Context: Clone + std::fmt::Debug + Send + Sync + 'static;
    type Transaction: Transaction;
    type AttendanceDao: AttendanceDao<Transaction = Self::Transaction> + Send + Sync;
    type AssemblyDao: AssemblyDao<Transaction = Self::Transaction> + Send + Sync;
    type PermissionService: PermissionService<Context = Self::Context> + Send + Sync;
    type TransactionDao: TransactionDao<Transaction = Self::Transaction> + Send + Sync;
}
```
→ Phase 11: ersetze `AttendanceDao`+`AssemblyDao` durch `RepaymentPhaseDao`+`RepaymentEntryDao`+`MemberDao` (3 DAOs statt 2). Full target in RESEARCH §Q15 Z. 858-868.

**Impl-Struct mit Non-Trait-Feldern** (`attendance_export.rs:74-83`):
```rust
pub struct AttendanceExportServiceImpl<Deps: AttendanceExportServiceDeps> {
    pub transaction_dao: Arc<Deps::TransactionDao>,
    pub permission_service: Arc<Deps::PermissionService>,
    pub assembly_dao: Arc<Deps::AssemblyDao>,
    pub attendance_dao: Arc<Deps::AttendanceDao>,
    pub pdf_generator: Arc<PdfGenerator>,
    pub template_base: Arc<PathBuf>,
}
```
→ Phase 11: drei DAOs + `pdf_generator` + `template_base`. Felder-Liste siehe RESEARCH §Q15 Z. 837-848.

**Permission-Funnel `check_admin_and_closed`** (`attendance_export.rs:100-131`) — **kritischer Reihenfolge-Pattern (Pitfall #2):**
```rust
async fn check_admin_and_closed(...) -> Result<AssemblyEntity, ServiceError> {
    // 1. Load (404 if missing)
    let assembly = self.assembly_dao.find_by_id(assembly_id, tx).await?
        .ok_or(ServiceError::EntityNotFound(assembly_id))?;
    // 2. Admin gate (403). Authentication::Full short-circuits.
    match &context {
        Authentication::Full => {}
        Authentication::Context(_) => {
            self.permission_service.check_permission(ADMIN_PRIVILEGE, context).await?;
        }
    }
    // 3. Status gate (409).
    if assembly.status != AssemblyStatus::Closed {
        return Err(ServiceError::Conflict(Arc::from("assembly_not_closed")));
    }
    Ok(assembly)
}
```
→ Phase 11: Funktionsname `check_admin_and_phase_status`; Status-Set ZWEI Stati (`Open | Closed`); Error-String `"phase_not_exportable"`. Voller Code in RESEARCH §Pattern 1 Z. 238-272.

**`export(...)`-Impl-Body** (`attendance_export.rs:141-237`) — Strukturierter Read-Only-Flow:
```rust
let tx = self.transaction_dao.use_transaction(None).await?;
let assembly = self.check_admin_and_closed(assembly_id, context, tx.clone()).await?;
let mut rows = self.attendance_dao.list_members_for_assembly(...).await?...;
// Filter in-memory
if matches!(include, ExportInclude::Present) { rows.retain(|r| r.is_present); }
self.transaction_dao.commit(tx).await?;  // ← commit BEFORE pdf_generator (Pitfall #8)
// ... compute date_str, tracing::info!, match format → render
Ok(AttendanceExport { bytes, content_type, filename: format!(...) })
```
→ Phase 11:
- 3 DAO-Reads in derselben Tx: `RepaymentPhaseDao::find_by_id` (im Funnel), `RepaymentEntryDao::find_by_phase_id`, dann pro Entry `MemberDao::find_by_id` (N+1, OK siehe RESEARCH §Q5)
- In-memory Filter nach `include` (D-01/D-02)
- Sort vor Enrichment (RESEARCH §Q12 Z. 736-745 Excerpt)
- Pre-Compute `amount_str` + `purpose`-String pro Row (RESEARCH §Q12 Z. 697-726)
- `match format { Pdf => self.pdf_generator.render_repayment_list(...) }` — kein Csv/Xlsx-Arm
- Filename: `format!("auszahlung-{}-{}.pdf", phase.fiscal_year, include_str)` (RESEARCH §Q6)

**`tracing::info!`-Pattern** (`attendance_export.rs:196-204`) — siehe RESEARCH §Q11 für Phase-11-Variante (`phase_id`, `fiscal_year`, `format`, `include`, `rows`).

**Grep-Gate-Test** (`attendance_export.rs:1167-1198`) — siehe RESEARCH §Q10 für vollen Code. **MUSS in Phase 11 mit angepasstem `include_str!("repayment_export.rs")` repliziert werden** (EXPO-05).

**Test-Setup-Pattern** (Mocks): `attendance_export.rs:748-784` zeigt `test_non_admin_returns_permission_denied`-Pattern — **403-vor-409-Reihenfolge-Test** ist Pflicht für Phase 11 (Pitfall #2).

---

### 4. `genossi_rest/src/repayment_export.rs` (NEW, REST-handler, request-response)

**Analog:** `genossi_rest/src/attendance_export.rs` (269 LOC, 1:1-Vorbild)

**RestState-Trait** (`attendance_export.rs:43-53`):
```rust
pub trait AttendanceExportRestState: Clone + Send + Sync + 'static {
    type AttendanceExportService: AttendanceExportService<Context = crate::ContextType>
        + Send + Sync + 'static;
    fn attendance_export_service(&self) -> Arc<Self::AttendanceExportService>;
}
```
→ Phase 11: `RepaymentExportRestState` + `repayment_export_service()`.

**`map_export_error` (CRITICAL — D-11)** (`attendance_export.rs:59-64`):
```rust
fn map_export_error(e: ServiceError) -> RestError {
    match e {
        ServiceError::PermissionDenied => RestError::Forbidden("forbidden".to_string()),
        other => other.into(),
    }
}
```
→ Phase 11: **1:1 identisch übernehmen**. PermissionDenied → 403 (NICHT 401). Andere Varianten delegieren ans globale `From<ServiceError>`.

**Query-Param mit `#[derive(Default)]`** (`attendance_export.rs:69-95`):
```rust
#[derive(Debug, Default, Deserialize, IntoParams, ToSchema)]
pub struct ExportQuery {
    #[serde(default)]
    pub include: ExportIncludeQuery,
}

#[derive(Debug, Default, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum ExportIncludeQuery {
    #[default]
    All,
    Present,
}

impl From<ExportIncludeQuery> for ExportInclude { ... }
```
→ Phase 11: Varianten `Open` (default), `All`, `Paid`. Vollständig in RESEARCH §Q13 Z. 769-793.

**Handler-Body** (`attendance_export.rs:122-168`) — **Format-Whitelist im Handler (D-12, Pitfall #3):**
```rust
#[instrument(skip(rest_state))]
pub async fn export_attendance<RestState: ...>(...) -> Response {
    error_handler((async {
        let auth = crate::extract_auth_context(Some(context))?;
        let format = match format_str.as_str() {
            "csv" => ExportFormat::Csv,
            "pdf" => ExportFormat::Pdf,
            "xlsx" => ExportFormat::Xlsx,
            other => return Err(RestError::BadRequest(format!("unknown export format: {}", other))),
        };
        let include: ExportInclude = query.include.into();
        let export = rest_state.attendance_export_service()
            .export(assembly_id, format, include, auth).await
            .map_err(map_export_error)?;
        let cd = crate::http_util::content_disposition_attachment(&export.filename);
        Ok(Response::builder()
            .status(200)
            .header("Content-Type", export.content_type)
            .header("Content-Disposition", &cd)
            .body(Body::from(export.bytes))
            .unwrap())
    }).await)
}
```
→ Phase 11: **NUR `"pdf" => ExportFormat::Pdf`-Arm**; alles andere (auch `csv`/`xlsx`) → 400. `http_util::content_disposition_attachment` 1:1 reusen (kein Selbstbau, RESEARCH §"Don't Hand-Roll").

**Router-Funktion** (`attendance_export.rs:174-180`):
```rust
pub fn generate_export_route<RestState: ...>() -> Router<RestState> {
    Router::new().route("/{assembly_id}/attendance-export/{format}",
                        get(export_attendance::<RestState>))
}
```
→ Phase 11: Route `/{phase_id}/export/{format}`; Mount-Prefix `/api/repayment-phase` (siehe Cross-Cutting #4).

**OpenAPI-ApiDoc** (`attendance_export.rs:182-189`):
```rust
#[derive(OpenApi)]
#[openapi(
    paths(export_attendance),
    components(schemas(ExportQuery, ExportIncludeQuery)),
    tags((name = "AttendanceExport", description = "..."))
)]
pub struct ApiDoc;
```
→ Phase 11: identisches Pattern; Tag `"RepaymentExport"`. **Bundle braucht KEIN `ToSchema`** (Body ist binary).

**Unit-Tests** (`attendance_export.rs:191-268`) — sieben kleine Tests für `map_export_error`-Varianten, Query-Default, Deserialisierungs-Roundtrip, `From`-Mapping. **Pflicht für Phase 11**, identische Struktur mit angepassten Enum-Varianten.

**`#[utoipa::path]`-Annotation** (`attendance_export.rs:104-121`) — komplette Response-Code-Liste (200/400/401/403/404/409) als Vorbild übernehmen.

---

### 5. `genossi_bin/tests/e2e_tests.rs` (NEW test-fns, E2E, HTTP-client)

**Analoga:**
- Phase 6 PDF-Test: `e2e_tests.rs:10128-10178` `test_export_pdf_closed_returns_pdf_magic_bytes`
- Phase 6 Format-Whitelist: `e2e_tests.rs:10372-10402` `test_export_unknown_format_returns_400`
- Phase 6 409-Conflict: `e2e_tests.rs:10293-10371` `test_export_open_assembly_returns_409_conflict` + `test_export_preparation_assembly_returns_409_conflict`
- Phase 6 Filename: `e2e_tests.rs:10443-10491` `test_export_filename_schema_matches_date`
- Phase 6 Include-Filter: `e2e_tests.rs:10403-10442` `test_export_include_present_filters_absent_members`
- Phase 9/10 Repayment-Setup-Helper: `e2e_tests.rs:10578` `create_preparation_repayment_phase`, `:11043` `create_member_with_exit_date`, `:11109` `create_open_repayment_phase`

**`setup_with_templates()`-Helper** (`e2e_tests.rs:2672-2694`) — **Pflicht für PDF-Tests (Pitfall #1)**:
```rust
async fn setup_with_templates() -> TestServer {
    let pool = Arc::new(SqlitePool::connect("sqlite::memory:").await.unwrap());
    sqlx::migrate!("../migrations/sqlite").run(&*pool).await.unwrap();
    let rest_state = RestStateImpl::new(pool);
    rest_state.template_storage().provision_defaults().await.unwrap();
    start_test_server(rest_state).await
}
```
→ Phase 11: 1:1 reusen — provisioniert nach `template_storage.rs`-Edit auch `auszahlungsliste.typ`.

**PDF-Happy-Path-Excerpt** (`e2e_tests.rs:10128-10178`):
```rust
#[tokio::test]
async fn test_export_pdf_closed_returns_pdf_magic_bytes() {
    let server = setup_with_templates().await;
    let client = reqwest::Client::new();
    let (aid, _, _, _) = create_closed_assembly_with_members(&client, &server, 5, 2).await;
    let resp = client.get(server.url(&format!(
        "/api/assembly/{}/attendance-export/pdf?include=all", aid))).send().await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(resp.headers().get("content-type").unwrap(), "application/pdf");
    let cd = resp.headers().get("content-disposition").unwrap().to_str().unwrap();
    assert!(cd.contains("teilnehmer.pdf"));
    let bytes = resp.bytes().await.unwrap();
    assert!(bytes.starts_with(b"%PDF-"));
}
```
→ Phase 11: URL-Schema `/api/repayment-phase/{phase_id}/export/pdf?include=open`; Filename-Assertion `auszahlung-2026-open.pdf`; `%PDF-`-Magic-Bytes-Check identisch.

**Phase-11-Test-Liste (6+ Tests, CONTEXT in-scope):**
1. PDF-Happy-Path (Vorbild: `:10128`)
2. 403 ohne Vorstand-Auth (Vorbild: Phase 6 hat ähnlichen Test — Helper-Auth → 403; Phase 11 nutzt non-admin-Context)
3. 400 bei unbekanntem Format (Vorbild: `:10372`); zusätzlicher Assertion: `?format=csv` → 400 (D-12)
4. `?include=open|all|paid` jeweils ein Test (Vorbild: `:10403` Include-Filter-Logik)
5. 409 bei `RepaymentPhase` in `Preparation` (Vorbild: `:10324`)
6. 404 bei unbekannter `phase_id` (Vorbild: Phase 6 hat impliziten Test, hier expliziter)
7. Leere IBAN (Member.bank_account NULL) → leere Spalte (D-06; eigenes Helper-Setup für Member ohne IBAN, RESEARCH §Q9 Z. 619-630)
8. Status-Leak-Test: non-admin auf Preparation-Phase → **403, NICHT 409** (Pitfall #2)
9. Audit-Verify-Test: `/api/audit/verify` muss nach Export-Calls weiter valide sein (D-11 EXPO-05)

**Setup-Helper-Wiederverwendung** (RESEARCH §Q9 — wichtig für Member-ohne-IBAN):
- `create_open_repayment_phase(client, server, 2026, 12000)` → `RepaymentPhaseTO` in Status `Open`
- `create_preparation_repayment_phase(...)` → für 409-Test
- `create_member_with_exit_date(client, server, member_number, fiscal_year, share_count)` → Members mit IBAN (Default `sample_member()`)
- **NEU für Phase 11**: Helper für Member ohne IBAN (PUT `/api/members/{id}` mit `bank_account: None` + version-Bump-Roundtrip; siehe RESEARCH §Q9 Skizze)

---

### 6. `genossi_service_impl/src/template_storage.rs` (MODIFY, config-registry, static-data)

**Analog:** Bestehende `DEFAULT_TEMPLATES`-Einträge (`template_storage.rs:10-27`)

**Vorbild-Excerpt:**
```rust
const DEFAULT_TEMPLATES: &[DefaultTemplate] = &[
    DefaultTemplate { path: "_layout.typ", content: include_bytes!("../../templates/defaults/_layout.typ") },
    DefaultTemplate { path: "join_confirmation.typ", content: include_bytes!("../../templates/defaults/join_confirmation.typ") },
    DefaultTemplate { path: "teilnehmerliste.typ", content: include_bytes!("../../templates/defaults/teilnehmerliste.typ") },
];
```

**Edit für Phase 11:** Neuen Eintrag am Ende des Array anhängen (vor `];`):
```rust
// Phase 11 (EXPO-01..03): Auszahlungslisten-Export PDF template.
// Required by RepaymentExportServiceImpl::export(ExportFormat::Pdf).
DefaultTemplate {
    path: "auszahlungsliste.typ",
    content: include_bytes!("../../templates/defaults/auszahlungsliste.typ"),
},
```
**Pitfall #1**: Ohne diesen Eintrag schreibt `provision_defaults()` das Template nicht auf Disk → 500 mit "template not found".

---

### 7. `genossi_service_impl/src/pdf_generation.rs` (MODIFY, renderer, transform)

**Analoga:**
- `render_attendance_list` (`pdf_generation.rs:279-336`)
- `build_inputs_attendance` (`pdf_generation.rs:602-652`)

**Vorbild-Excerpt für neue Methode `render_repayment_list`** (RESEARCH §Q1 Z. 449-479 enthält vollständige Signatur). Phase-11-Variante:
```rust
pub fn render_repayment_list(
    &self,
    template_path: &str,                  // "auszahlungsliste.typ"
    template_base: &Path,
    phase: &RepaymentPhaseEntity,
    rows: &[RepaymentExportRow],          // neue Service-Impl-lokale Struct
) -> Result<Vec<u8>, ServiceError> {
    // Identische Struktur wie render_attendance_list:
    // 1. fs::read_to_string mit NotFound→InternalError-Map (pdf_generation.rs:289-298)
    // 2. let inputs = build_inputs_repayment(phase, rows);
    // 3. TemplateWorld::new mit gleichen Args (pdf_generation.rs:302-310)
    // 4. typst::compile::<PagedDocument>(&world)  (pdf_generation.rs:312)
    // 5. Match auf result.output → typst_pdf::pdf(&document, &options)  (pdf_generation.rs:314-335)
}
```

**Vorbild für `build_inputs_repayment`** (`pdf_generation.rs:602-652` für `build_inputs_attendance`):
```rust
fn build_inputs_attendance(assembly, rows, present, total) -> Dict {
    let mut inputs = Dict::new();
    // ... date-format, meta-json, rows-json
    let meta = serde_json::json!({ "title": ..., "date": ..., "present": ..., "total": ... });
    inputs.insert(Str::from("meta"), Value::Str(Str::from(serde_json::to_string(&meta).unwrap().as_str())));
    let row_values: Vec<serde_json::Value> = rows.iter().map(|r| serde_json::json!({...})).collect();
    inputs.insert(Str::from("rows"), Value::Str(Str::from(serde_json::to_string(&Array(row_values)).unwrap().as_str())));
    inputs
}
```
→ Phase 11: `meta` enthält `fiscal_year`, `row_count`, `share_value_cent`, `title`, `date`; `rows` enthält `member_number`, `name`, `iban`, `share_count`, `amount_str`, `purpose` (pre-computed im Service, NICHT im Renderer).

**`RepaymentExportRow`-Struct** (RESEARCH §Q1 Z. 497-505 — Service-Impl-lokal, NICHT im DAO):
```rust
struct RepaymentExportRow {
    member_number: i64,
    name: String,
    iban: String,
    share_count: i32,
    amount_str: String,
    purpose: String,
}
```
**Lokationsfrage (Planner-Discretion):** Struct kann in `genossi_service_impl/src/repayment_export.rs` leben (mit Sub-`pub(crate) use` in `pdf_generation.rs`) ODER in `pdf_generation.rs` direkt. Phase-6-Konvention: domain-spezifische Row-Typen leben im DAO (`AttendanceMemberRow` in `genossi_dao::attendance`); für Phase 11 reicht service-impl-lokal, weil es keine DB-Repräsentation hat (nur Render-Bundle).

---

### 8. Module-Root-Modifications (3 `lib.rs`-Dateien, MODIFY, additiv)

**Analog:** Bestehende `pub mod attendance_export;`-Zeilen.

| Datei | Edit | Position |
|---|---|---|
| `genossi_service/src/lib.rs` | `pub mod repayment_export;` | nach `pub mod attendance_export;` |
| `genossi_service_impl/src/lib.rs` | `pub mod repayment_export;` | nach `pub mod attendance_export;` |
| `genossi_rest/src/lib.rs` | `pub mod repayment_export;` | nach `pub mod attendance_export;` (`lib.rs:4`) |

---

### 9. `genossi_rest/src/lib.rs::create_app` (MODIFY, router + ApiDoc + bounds)

**Analoga:**
- ApiDoc-Nests: `lib.rs:271-275`
- `RestState`-Bounds in `create_app`: `lib.rs:440-444`
- Router-Mount: `lib.rs:640-643`
- `RestState`-Bounds-Duplikat: `lib.rs:763-767`

**Vorbild-Excerpts:**

ApiDoc-Nests (`lib.rs:271-275`):
```rust
(path = "/api/repayment-phase", api = repayment_phase::ApiDoc),
(path = "/api/repayment-entry", api = repayment_entry::ApiDoc),
// ...
(path = "/api/assembly/{assembly_id}/attendance-export", api = attendance_export::ApiDoc),
```
→ Phase 11: Neue Zeile `(path = "/api/repayment-phase/{phase_id}/export", api = repayment_export::ApiDoc),` (RESEARCH §Q8 Z. 591-594).

RestState-Bounds (`lib.rs:440-444` und `:763-767`):
```rust
+ repayment_phase::RepaymentPhaseRestState
+ repayment_entry::RepaymentEntryRestState
+ attendance_export::AttendanceExportRestState
```
→ Phase 11: `+ repayment_export::RepaymentExportRestState` in BEIDEN Stellen einfügen.

Router-Mount (`lib.rs:640-643`):
```rust
.nest("/api/assembly", attendance_export::generate_export_route::<RestState>(),)
```
→ Phase 11:
```rust
.nest("/api/repayment-phase", repayment_export::generate_export_route::<RestState>(),)
```
**Wichtig (RESEARCH §Q15 Z. 894-905):** Es gibt bereits ein `.nest("/api/repayment-phase", repayment_phase::generate_route::<RestState>())` (`lib.rs:613-614`). Axum 0.8.3 merged zwei `.nest()`-Aufrufe auf denselben Prefix; Routen-Pfade `/` vs. `/{phase_id}/export/{format}` kollidieren nicht. Planner soll im PLAN.md explizit auf die Merge-Strategie hinweisen + E2E-Test deckt beide-Routen-koexistieren-Smoke-Test ab.

---

### 10. `genossi_bin/src/lib.rs::RestStateImpl` (MODIFY, DI-wiring)

**Analoga:**
- Deps-Type-Alias-Block: `bin/lib.rs:266-289`
- Feld auf `RestStateImpl`: `bin/lib.rs:520-522`
- Construction in `new()`: `bin/lib.rs:815-830`
- Insertion in Struct-Initializer: `bin/lib.rs:918-922`
- RestState-Trait-Impl: `bin/lib.rs:1460-1468`

**Vorbild-Excerpts:**

Deps-Type-Alias (`bin/lib.rs:271-289`):
```rust
pub struct AttendanceExportServiceDependencies;
unsafe impl Send for AttendanceExportServiceDependencies {}
unsafe impl Sync for AttendanceExportServiceDependencies {}
impl genossi_service_impl::attendance_export::AttendanceExportServiceDeps
    for AttendanceExportServiceDependencies {
    type Context = Context;
    type Transaction = Transaction;
    type AttendanceDao = AttendanceDao;
    type AssemblyDao = AssemblyDao;
    type PermissionService = PermissionService;
    type TransactionDao = TransactionDao;
}
type AttendanceExportService = genossi_service_impl::attendance_export::AttendanceExportServiceImpl<
    AttendanceExportServiceDependencies,
>;
```
→ Phase 11: vollständig in RESEARCH §Q15 Z. 853-872 (5 Type-Aliases: 3 DAOs statt 2).

Feld (`bin/lib.rs:520-522`):
```rust
// Phase 6 Plan 03: AttendanceExportServiceImpl exposed to REST handlers via
// AttendanceExportRestState (D-DI wiring).
attendance_export_service: Arc<AttendanceExportService>,
```
→ Phase 11: `repayment_export_service: Arc<RepaymentExportService>,` (analog kommentiert).

Construction (`bin/lib.rs:815-830`):
```rust
// Phase 6 Plan 03: AttendanceExportServiceImpl (D-01..D-18 backend).
// Re-uses the existing `pdf_generator` (line 585) and `template_storage` (line 583) Arcs.
let attendance_export_service = Arc::new(
    genossi_service_impl::attendance_export::AttendanceExportServiceImpl {
        transaction_dao: transaction_dao.clone(),
        permission_service: permission_service.clone(),
        assembly_dao: assembly_dao.clone(),
        attendance_dao: attendance_dao.clone(),
        pdf_generator: pdf_generator.clone(),
        template_base: Arc::new(template_storage.base_path().to_path_buf()),
    },
);
```
→ Phase 11: Felder `repayment_phase_dao`, `repayment_entry_dao`, `member_dao` (alle bereits in `RestStateImpl` für Phase 7-10 verfügbar, RESEARCH §Q15 Z. 824-831). Voller Code in RESEARCH §Q15 Z. 837-848.

RestState-Trait-Impl (`bin/lib.rs:1462-1468`):
```rust
impl genossi_rest::attendance_export::AttendanceExportRestState for RestStateImpl {
    type AttendanceExportService = AttendanceExportService;
    fn attendance_export_service(&self) -> Arc<Self::AttendanceExportService> {
        self.attendance_export_service.clone()
    }
}
```
→ Phase 11: analog mit `repayment_export`-Namen, vollständig in RESEARCH §Q15 Z. 877-883.

---

## Shared Patterns (Cross-Cutting)

### S1. Permission-Funnel-Order (Pitfall #2 — kritisch)

**Source:** `genossi_service_impl/src/attendance_export.rs:100-131`
**Apply to:** Service-Impl (`repayment_export.rs`)
**Pattern:** `load (404) → permission_check (403) → status_check (409)`. Status-Check NIE vor Permission-Check (sonst Information-Leak).
**Test-Assertion:** Non-admin gegen Preparation-Phase muss 403 liefern, NICHT 409. E2E-Test #8.

### S2. `map_export_error` — PermissionDenied → 403

**Source:** `genossi_rest/src/attendance_export.rs:59-64`
**Apply to:** REST-Handler (`repayment_export.rs`)
**Pattern:** Lokale Mapping-Funktion pro REST-Modul; `PermissionDenied → Forbidden(403)`; alle anderen Varianten delegieren ans globale `From<ServiceError>`.
**Critical:** Globales `From<ServiceError>` mappt `PermissionDenied → Unauthorized(401)` — daher MUSS `map_export_error` lokal überschreiben (Frontend trennt "kein Admin" 403 von "Session ungültig" 401).

### S3. Format-Whitelist im REST (Pitfall #3 — kritisch)

**Source:** `genossi_rest/src/attendance_export.rs:135-145`
**Apply to:** REST-Handler (`repayment_export.rs`)
**Pattern:** `match format_str.as_str()` VOR Service-Call; nur valide Formate durchgereicht; alles andere → `RestError::BadRequest`.
**Phase-11-Spezifikum:** NUR `"pdf"`-Arm. `"csv"` und `"xlsx"` müssen ins `other => 400`-Arm fallen (D-12). E2E-Test prüft `?format=csv → 400`.

### S4. Read-Only-Service ohne Audit-Macros (EXPO-05 / D-11)

**Source:** `genossi_service_impl/src/attendance_export.rs:1167-1198` (Grep-Gate-Test)
**Apply to:** Service-Impl (`repayment_export.rs`)
**Pattern:** Self-Reference-Trick via `format!("{}!", "audited_create")` (sonst invalidiert das Test-Source-File seine eigenen Assertions). `tracing::info!` mit `target = "<service_name>"` ersetzt Audit-Eintrag (Pattern: `attendance_export.rs:57-58, 196-204`).
**Test:** Inline-Unit-Test im Service-Impl + optional `rg`-Shell-Gate in CI (CONTEXT erwähnt beides).

### S5. Filename Server-Generated im Bundle (D-15, kein User-Input)

**Source:** `genossi_service_impl/src/attendance_export.rs:187-194, 231-235`
**Apply to:** Service-Impl + REST-Handler
**Pattern:** Service erzeugt Filename via `format!`; Handler liest nur `export.filename` und reicht an `http_util::content_disposition_attachment(&export.filename)`. Kein Pfad von User-Input zur `Content-Disposition`-Header (Path-Injection-Prevention).
**Phase-11-Schema (ROADMAP SC #2):** `auszahlung-{fiscal_year}-{include}.pdf`.

### S6. `content_disposition_attachment` für RFC-6266-Header

**Source:** `genossi_rest/src/http_util.rs:43`
**Apply to:** REST-Handler (`repayment_export.rs`)
**Pattern:** **NIE selbst bauen.** Helper handhabt UTF-8-Filename, Quote-Escape, RFC-6266-konforme `filename=` + `filename*=UTF-8''`-Doppelangabe.

### S7. PDF-Renderer: tx-Commit VOR `pdf_generator.render_*` (Pitfall #8)

**Source:** `genossi_service_impl/src/attendance_export.rs:175` (`commit(tx)` BEFORE PDF render)
**Apply to:** Service-Impl (`repayment_export.rs`)
**Pattern:** `pdf_generator.render_*` ist SYNC (`fn`, kein `async`); Tx muss VORHER committed sein. Keine DAO-Calls NACH dem Render.

### S8. `setup_with_templates()` für PDF-E2E-Tests (Pitfall #1)

**Source:** `genossi_bin/tests/e2e_tests.rs:2672-2694`
**Apply to:** Alle Phase-11-PDF-E2E-Tests
**Pattern:** Setup-Variante mit `template_storage().provision_defaults().await`. Sonst 500 "template not found" für PDF-Branch. Trivialer Test (z.B. 400 für unknown-format) darf `setup()` ohne Templates benutzen.

### S9. `RepaymentExportRow` Pre-Computing im Service, NICHT im Renderer

**Source:** RESEARCH §Q12 + CONTEXT Discretion "Betrag-Rendering"
**Apply to:** Service-Impl
**Pattern:** Service rechnet `amount_str = format!("{},{:02}", cents / 100, cents % 100)` und baut `purpose = format!("Anteilsrückzahlung GJ {} {} {} {}", fy, mn, fn, ln)` (D-04 wörtlich). Renderer (Typst-Template) bekommt nur fertige Strings. Phase 10 D-04 ist Vorbild für Euro-Format (RESEARCH §"State of the Art").

### S10. Stable Sort vor Enrichment-Filter-Map

**Source:** RESEARCH §Q12 Z. 736-745
**Apply to:** Service-Impl
**Pattern:** Sortiere **Entries** (nicht Rows) vor dem Enrichment-`filter_map`. Rust `sort_by` ist stable. Sort-Schlüssel: `(member.member_number, entry.created)`. D-09.

---

## No Analog Found

Keine. Phase 11 ist ein vollständiges Replikations-Diff von Phase 6 (siehe RESEARCH §Summary). Alle 10 Datei-Edits haben ein direktes Vorbild.

---

## Metadata

**Analog search scope:**
- `genossi_service/src/` (Service-Traits)
- `genossi_service_impl/src/` (Service-Impls, audit-macros, pdf_generation, template_storage)
- `genossi_rest/src/` (REST-Handler, http_util, lib.rs Routing)
- `genossi_bin/src/` (DI-Wiring)
- `genossi_bin/tests/` (E2E-Tests + Helper)
- `templates/defaults/` (Typst-Templates)

**Files scanned:** 6 (Phase-6-Vorbilder, alle exakt geprüft via Read mit `file:line`-Bestätigung)
**Read-Coverage:**
- `genossi_service/src/attendance_export.rs` (vollständig, 158 LOC)
- `genossi_service_impl/src/attendance_export.rs` (Z. 1-240 — Permission-Funnel, Export-Body)
- `genossi_rest/src/attendance_export.rs` (vollständig, 269 LOC)
- `genossi_service_impl/src/pdf_generation.rs` (Z. 270-336 render_attendance_list; Z. 595-680 build_inputs_attendance)
- `genossi_service_impl/src/template_storage.rs` (Z. 1-100)
- `templates/defaults/teilnehmerliste.typ` (vollständig, 43 LOC)
- `genossi_rest/src/lib.rs` (grep + Z. 635-645 — Mount-Stelle)
- `genossi_bin/src/lib.rs` (grep + Z. 266-289 Deps, Z. 815-830 Construction, Z. 1460-1468 RestState-Impl)
- `genossi_bin/tests/e2e_tests.rs` (Z. 2672-2694 setup_with_templates, Z. 10120-10180 PDF-Happy-Path)

**Pattern-Extraktion-Strategie:** Jede konkrete Pattern-Excerpt-Stelle ist via `file:line` referenziert; volle Code-Bodies leben in RESEARCH.md §"Code Examples" Q1, Q3, Q6-Q15 — dieses Dokument duplicatet NICHT. Planner-Workflow:
1. PLAN-Action liest PATTERNS.md "Pattern Assignment" → bekommt `file:line`-Anker
2. Falls vollständiger Code-Body gebraucht: RESEARCH.md §Q-Sektion
3. Falls Verifikation gebraucht: direktes `Read` auf den `file:line`-Anker

**Pattern extraction date:** 2026-05-31
