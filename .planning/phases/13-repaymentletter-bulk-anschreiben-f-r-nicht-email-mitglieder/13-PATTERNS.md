# Phase 13: RepaymentLetter-Bulk-Anschreiben fuer Nicht-Email-Mitglieder - Pattern Map

**Mapped:** 2026-06-01
**Files analyzed:** 13 (8 NEU, 5 MODIFY)
**Analogs found:** 13 / 13 (alle haben starke Vorbilder im Repo)

## File Classification

| File (NEU/MOD) | Role | Data Flow | Closest Analog | Match Quality |
|---|---|---|---|---|
| `genossi_service/src/repayment_letter.rs` (NEU) | Service-Trait | Request-Response | `genossi_service/src/repayment_entry.rs` | exact (Trait+Submission-Input-Pattern) |
| `genossi_service/src/repayment_context.rs` (NEU) | Resolver-Trait | Read-Aggregation | `genossi_service/src/repayment_phase.rs` (Domain-Type-Pattern) | role-match |
| `genossi_service_impl/src/repayment_letter.rs` (NEU) | Service-Impl | Read -> Render -> Persist (audited) -> Bundle | `genossi_service_impl/src/repayment_export.rs` (Funnel+Render+Direct-Download) + `genossi_service_impl/src/member_document.rs:115-150` (audited_create + relative_path) | exact-blend |
| `genossi_service_impl/src/repayment_context.rs` (NEU) | Resolver-Impl | Filter+SUM+Format | `genossi_mail/src/worker.rs:332-361` (Inline-Aggregation Phase 10) | exact |
| `genossi_rest/src/repayment_letter.rs` (NEU) | REST-Handler | Direct-Download PDF | `genossi_rest/src/repayment_export.rs:96-156` | exact |
| `genossi_service/src/member_document.rs:48-101` (MOD) | Type-Enum-Variant | n/a | Existierende `RepaymentMail`-Variante in derselben Datei (Phase 10 D-09) | exact |
| `genossi_service_impl/src/template_storage.rs:10-35` (MOD) | DEFAULT_TEMPLATES-Eintrag | n/a (build-time) | Existierender `auszahlungsliste.typ`-Eintrag in derselben Datei | exact |
| `genossi_service_impl/src/pdf_generation.rs:386-441` (MOD) | PdfGenerator-Methode | sync Typst-Render | `render_repayment_list` + `render_attendance_list:313-370` in derselben Datei | exact |
| `genossi_bin/src/lib.rs:291-316,862-881,1523-1531` (MOD) | DI-Wiring | n/a | Existierender `RepaymentExportServiceDependencies`-Block + `RestStateImpl::new()`-Wiring | exact |
| `genossi_rest/src/lib.rs:5,279,449,654-657` (MOD) | Route-Mount | n/a | Existierender Mount fuer `repayment_export::generate_export_route` | exact |
| `templates/defaults/auszahlungs_anschreiben.typ` (NEU) | Typst-Template (letter) | Render | `templates/zahlungsanfrage.typ` (Layout) + `templates/defaults/auszahlungsliste.typ` (sys.inputs-JSON-Pattern) | exact-blend |
| `genossi-frontend/src/component/repayment_entry_list.rs:220-296` (MOD) | Frontend Bulk-Button | Click-Bubble | Existierender Massenmail-Button in derselben Datei (Zeilen 226-249) | exact |
| `genossi-frontend/src/page/repayment_phase_details.rs:240-260` (MOD) | Page-Wiring + Browser-Save | Blob -> Download | `genossi-frontend/src/page/assembly_details.rs:355-395` (ExportTab Browser-Save) + `genossi-frontend/src/api.rs:1891-1937` (export_attendance_url) | exact |
| `genossi-frontend/src/api.rs` (MOD: add `generate_repayment_letters`) | API-Client | fetch -> blob -> url | `genossi-frontend/src/api.rs:1891-1937` (`export_attendance_url`) | exact |
| `genossi_bin/tests/e2e_tests.rs` (MOD: 8 neue Tests) | E2E-Tests | reqwest HTTP | `genossi_bin/tests/e2e_tests.rs:13364-13460` (`test_export_repayment_pdf_open_happy_path`) | exact |

## Pattern Assignments

### `genossi_service/src/repayment_letter.rs` (NEU, Service-Trait)

**Analog:** `genossi_service/src/repayment_entry.rs` (Trait + Submission-Input-Pattern, 1:1 wie auch Plan-Anker aus repayment_phase.rs).

**Imports pattern** (`repayment_entry.rs:22-30`):
```rust
use async_trait::async_trait;
use mockall::automock;
use std::fmt::Debug;
use std::sync::Arc;
use uuid::Uuid;

use crate::permission::Authentication;
use crate::ServiceError;
```

**Trait-Header mit `automock`** (`repayment_entry.rs:120-124`):
```rust
#[automock(type Context = (); type Transaction = genossi_dao::MockTransaction;)]
#[async_trait]
pub trait RepaymentEntryService {
    type Context: Clone + Debug + PartialEq + Eq + Send + Sync + 'static;
    type Transaction: genossi_dao::Transaction;
    // ...async fns
}
```

**Input-DTO-Pattern** (`repayment_entry.rs:114-118`):
```rust
#[derive(Clone, Debug)]
pub struct RepaymentEntryBatchStatusInput {
    pub entry_ids: Arc<[Uuid]>,
    pub target_status: RepaymentEntryStatus,
}
```

**Apply to Phase 13:** Trait `RepaymentLetterService` mit einer Methode `generate(phase_id: Uuid, entry_ids: Arc<[Uuid]>, context: Authentication<Self::Context>) -> Result<RepaymentLetterBundle, ServiceError>`. Output-Struct `RepaymentLetterBundle { bundle_bytes: Vec<u8>, filename: String, document_ids: Vec<Uuid> }` als Service-Layer-Return.

---

### `genossi_service/src/repayment_context.rs` (NEU, Resolver-Trait)

**Analog:** `genossi_service/src/repayment_phase.rs` (Domain-Type-Pattern fuer Public-Struct + Trait-Signatur).

**Apply to Phase 13:**
```rust
// Public output struct (analog `RepaymentPhase` in repayment_phase.rs:33-43)
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RepaymentContext {
    pub share_count: i32,
    pub payout_amount: String,  // German Euro "X,YZ" (Phase-10-D-04-konform)
    pub fiscal_year: i32,
}

#[automock(type Context = (); type Transaction = genossi_dao::MockTransaction;)]
#[async_trait]
pub trait RepaymentContextResolver {
    type Transaction: genossi_dao::Transaction;
    async fn resolve(
        &self,
        phase_id: Uuid,
        member_id: Uuid,
        tx: Self::Transaction,
    ) -> Result<RepaymentContext, ServiceError>;
}
```

---

### `genossi_service_impl/src/repayment_letter.rs` (NEU, Service-Impl)

**Analog (Funnel + Direct-Download Trait-Bound):** `genossi_service_impl/src/repayment_export.rs:38-110`.

**`Deps`-Trait** (`repayment_export.rs:42-54`):
```rust
pub trait RepaymentExportServiceDeps: Send + Sync + 'static {
    type Context: Clone + std::fmt::Debug + Send + Sync + 'static;
    type Transaction: Transaction;
    type RepaymentPhaseDao: RepaymentPhaseDao<Transaction = Self::Transaction> + Send + Sync;
    type RepaymentEntryDao: RepaymentEntryDao<Transaction = Self::Transaction> + Send + Sync;
    type MemberDao: MemberDao<Transaction = Self::Transaction> + Send + Sync;
    type PermissionService: PermissionService<Context = Self::Context> + Send + Sync;
    type TransactionDao: TransactionDao<Transaction = Self::Transaction> + Send + Sync;
}
```

**Apply to Phase 13:** Erweitern um `MemberDocumentDao`, `AuditLogDao`, `UuidService`, `DocumentStorage`, `RepaymentContextResolver` (alle als `Send + Sync`-Trait-Bound). Struct-Felder `pdf_generator: Arc<PdfGenerator>` + `template_base: Arc<PathBuf>` 1:1 uebernehmen.

**Permission-Funnel mit Status-Gate** (`repayment_export.rs:77-110`):
```rust
async fn check_admin_and_phase_status(
    &self,
    phase_id: Uuid,
    context: Authentication<Deps::Context>,
    tx: Deps::Transaction,
) -> Result<RepaymentPhaseEntity, ServiceError> {
    // 1. Load (404 if missing).
    let phase = self.repayment_phase_dao
        .find_by_id(phase_id, tx)
        .await?
        .ok_or(ServiceError::EntityNotFound(phase_id))?;
    // 2. Admin gate (403). Authentication::Full short-circuits.
    match &context {
        Authentication::Full => {}
        Authentication::Context(_) => {
            self.permission_service
                .check_permission(ADMIN_PRIVILEGE, context).await?;
        }
    }
    // 3. Status gate (409): Open ODER Closed akzeptiert.
    match phase.status {
        RepaymentPhaseStatus::Open | RepaymentPhaseStatus::Closed => {}
        RepaymentPhaseStatus::Preparation => {
            return Err(ServiceError::Conflict(Arc::from("phase_not_active")));
        }
    }
    Ok(phase)
}
```

**Apply to Phase 13:** 1:1 wiederverwenden — Status-Gate-Error-Code-Konstante `"phase_not_active"` statt `"phase_not_exportable"` (D-13 / CONTEXT scope).

**Entry-Validation Pattern (Subset-Check, entry_phase_mismatch)** (eigener Code, basiert auf `repayment_export.rs:197-203`):
```rust
let phase_entries: Vec<RepaymentEntryEntity> = self
    .repayment_entry_dao
    .find_by_phase_id(phase_id, tx.clone())
    .await?
    .iter().cloned().collect();
let phase_entry_set: HashSet<Uuid> = phase_entries.iter().map(|e| e.id).collect();
let requested_set: HashSet<Uuid> = body.entry_ids.iter().copied().collect();
if !requested_set.is_subset(&phase_entry_set) {
    return Err(ServiceError::Conflict(Arc::from("entry_phase_mismatch")));
    // alternativ ValidationError fuer 400 - Plan-Discretion (RESEARCH Pitfall #3)
}
```

**audited_create! pro Brief + document_storage.save** (Vorbild: `genossi_service_impl/src/member_document.rs:115-150`):
```rust
// In Loop: pro Member 1 audited_create. NICHT parallel (RESEARCH Pitfall #4).
let doc_id = self.uuid_service.new_v4().await;
let extension = "pdf"; // konstant fuer Phase 13
let relative_path = format!("{}.{}", doc_id, extension);

let now = time::OffsetDateTime::now_utc();
let new_doc = MemberDocument {
    id: doc_id,
    member_id: member.id,
    document_type: DocumentType::RepaymentLetter,
    description: Some(Arc::from(
        format!("Anschreiben Auszahlung GJ {}", phase.fiscal_year).as_str()
    )),
    file_name: Arc::from(format!(
        "auszahlungs_anschreiben_{}_GJ_{}.pdf",
        member.member_number, phase.fiscal_year
    ).as_str()),
    mime_type: Arc::from("application/pdf"),
    relative_path: Arc::from(relative_path.as_str()),
    created: time::PrimitiveDateTime::new(now.date(), now.time()),
    deleted: None,
    version: self.uuid_service.new_v4().await,
    template_id: None,           // D-13 / CONTEXT D-LETT-04 — NULL fuer Brief
    mail_recipient_id: None,     // D-13 / CONTEXT D-LETT-04 — NULL fuer Brief
    status: None,                // D-13 / CONTEXT D-LETT-04 — NULL fuer Brief
};

// File-Save VOR audited_create (RESEARCH §Assumptions A2): kein verwaistes MemberDocument.
self.document_storage
    .save(&relative_path, &pdf_bytes)
    .await
    .map_err(|e| ServiceError::InternalError(Arc::from(format!(
        "document_storage save failed: {}", e
    ))))?;

let doc_entity: genossi_dao::member_document::MemberDocumentEntity = (&new_doc).into();
crate::audited_create!(
    self,
    self.member_document_dao,
    &doc_entity,
    PROCESS,        // const REPAYMENT_LETTER_PROCESS: &str = "repayment-letter-service";
    &user_id,
    tx
);
```

**Sync Render nach Commit (Pitfall #8 / D-LETT-Render-Reihenfolge)** (`repayment_export.rs:226-252`):
```rust
// Pitfall #8: Commit Tx VOR PdfGenerator::render_* (sync method).
self.transaction_dao.commit(tx).await?;

let bytes = self.pdf_generator.render_repayment_list(
    "auszahlungsliste.typ",
    &self.template_base,
    &phase,
    &enriched_rows,
)?;
```

**Apply to Phase 13:** Render-Reihenfolge gemaess RESEARCH Pitfall #2: **Lese-Tx -> Read Phase/Entries/Members -> Commit -> Render N + Save N Files in-memory -> neue Schreibe-Tx mit N `audited_create!` -> Commit -> Bundle-Render -> Return**. Plan-Discretion (CONTEXT D-13-08 Empfehlung).

**Resolver-Call** (neuer Code, Pattern aus `genossi_mail/src/worker.rs:332-361`):
```rust
// Pro unique member_id:
let ctx = self.repayment_context_resolver
    .resolve(phase_id, member.id, tx.clone())
    .await?;
// ctx.share_count, ctx.payout_amount, ctx.fiscal_year fuer PdfGenerator-Inputs.
```

**Service-Process-Konstante** (`repayment_export.rs:37-40`):
```rust
const ADMIN_PRIVILEGE: &str = "admin";
const EXPORT_TARGET: &str = "repayment_export";  // tracing target
```

**Apply to Phase 13:**
```rust
const ADMIN_PRIVILEGE: &str = "admin";
const REPAYMENT_LETTER_PROCESS: &str = "repayment-letter-service";  // audit process id
const LETTER_TARGET: &str = "repayment_letter";  // tracing target
```

---

### `genossi_service_impl/src/repayment_context.rs` (NEU, Resolver-Impl)

**Analog:** `genossi_mail/src/worker.rs:332-361` — die Inline-Aggregation aus Phase 10 (CONTEXT D-13-10 / RESEARCH §State of the Art).

**Filter + SUM + Euro-Format Pattern** (`worker.rs:332-361`):
```rust
// D-06 filter: deleted IS NULL AND status IN (Open, Contacted).
// PaidOut and Declined explicitly excluded.
let relevant: Vec<_> = entries
    .iter()
    .filter(|e| {
        e.deleted.is_none()
            && e.member_id == member.id
            && matches!(
                e.status,
                RepaymentEntryStatus::Open | RepaymentEntryStatus::Contacted,
            )
    })
    .collect();

if !relevant.is_empty() {
    let share_count: i32 = relevant.iter().map(|e| e.share_count_to_pay_out).sum();
    let cents: i64 = (share_count as i64) * (phase.share_value);
    // German locale "X,YZ" (Plan 10.05-aligned formatting).
    let payout_amount = format!("{},{:02}", cents / 100, cents % 100);
    // ...merge in template-context
}
```

**Apply to Phase 13:** Den Aggregations-Block in eine pure Function `aggregate_for_member(phase: &RepaymentPhaseEntity, entries: &[RepaymentEntryEntity], member_id: Uuid) -> Option<RepaymentContext>` extrahieren — direkt testbar ohne Mocks. Der `RepaymentContextResolverImpl::resolve` ruft DAO-Reads + delegiert zur pure fn. **Wichtig:** Filter `Open | Contacted` 1:1 spiegeln, sonst drift Brief vs. Mail (RESEARCH A4).

**Deps-Trait** (`repayment_export.rs:42-54` als Vorbild, abgespeckt):
```rust
pub trait RepaymentContextResolverDeps: Send + Sync + 'static {
    type Transaction: Transaction;
    type RepaymentPhaseDao: RepaymentPhaseDao<Transaction = Self::Transaction> + Send + Sync;
    type RepaymentEntryDao: RepaymentEntryDao<Transaction = Self::Transaction> + Send + Sync;
}
```

---

### `genossi_rest/src/repayment_letter.rs` (NEU, REST-Handler)

**Analog:** `genossi_rest/src/repayment_export.rs:1-178` — Direct-Download + State-Trait + ApiDoc.

**Imports + Error-Mapping** (`repayment_export.rs:11-44`):
```rust
use axum::{
    body::Body,
    extract::{Path, State},
    response::Response,
    routing::post,
    Extension, Json, Router,
};
use serde::Deserialize;
use tracing::instrument;
use utoipa::{OpenApi, ToSchema};
use uuid::Uuid;

use genossi_service::ServiceError;
use crate::{error_handler, extract_auth_context, http_util, Context, RestError, RestStateDef};

/// D-11 / Phase 6 D-13: PermissionDenied -> Forbidden(403).
fn map_letter_error(e: ServiceError) -> RestError {
    match e {
        ServiceError::PermissionDenied => RestError::Forbidden("forbidden".to_string()),
        other => other.into(),
    }
}
```

**State-Trait** (`repayment_export.rs:84-90`):
```rust
pub trait RepaymentExportRestState: Clone + Send + Sync + 'static {
    type RepaymentExportService: RepaymentExportService<Context = crate::ContextType>
        + Send + Sync + 'static;
    fn repayment_export_service(&self) -> Arc<Self::RepaymentExportService>;
}
```

**Direct-Download-Handler mit Utoipa + error_handler** (`repayment_export.rs:96-156`):
```rust
#[utoipa::path(
    get,
    path = "/api/repayment-phase/{phase_id}/export/{format}",
    params(("phase_id" = Uuid, Path, description = "RepaymentPhase UUID")),
    responses(
        (status = 200, description = "PDF-Bytes ...", content_type = "application/pdf"),
        (status = 400, description = "Unbekanntes Format ..."),
        (status = 401, description = "Session ungueltig ..."),
        (status = 403, description = "Auth gueltig, aber kein Vorstand ..."),
        (status = 404, description = "RepaymentPhase mit dieser ID nicht gefunden"),
        (status = 409, description = "RepaymentPhase im Status Preparation ..."),
    ),
    tag = "RepaymentExport"
)]
#[instrument(skip(rest_state))]
pub async fn export_repayment<RestState: RestStateDef + RepaymentExportRestState>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path((phase_id, format_str)): Path<(Uuid, String)>,
    Query(query): Query<ExportQuery>,
) -> Response {
    error_handler(
        (async {
            let auth = extract_auth_context(Some(context))?;
            // ...format validation, service call...
            let export = rest_state
                .repayment_export_service()
                .export(phase_id, format, include, auth)
                .await
                .map_err(map_export_error)?;
            let cd = http_util::content_disposition_attachment(&export.filename);
            Ok(Response::builder()
                .status(200)
                .header("Content-Type", export.content_type)
                .header("Content-Disposition", &cd)
                .body(Body::from(export.bytes))
                .unwrap())
        })
        .await,
    )
}
```

**Apply to Phase 13:** Methode = `POST` statt `GET`. JSON-Body `GenerateLettersRequest { entry_ids: Vec<Uuid> }` mit `#[derive(Debug, Deserialize, ToSchema)]`. Empty-Check `if body.entry_ids.is_empty() -> 400 BadRequest`. Reuse `http_util::content_disposition_attachment` 1:1.

**Route-Generator** (`repayment_export.rs:163-169`):
```rust
pub fn generate_export_route<RestState: RestStateDef + RepaymentExportRestState>(
) -> Router<RestState> {
    Router::new().route(
        "/{phase_id}/export/{format}",
        get(export_repayment::<RestState>),
    )
}
```

**Apply to Phase 13:** Route `/{phase_id}/letters/generate` mit `post(generate_letters::<RestState>)`.

**ApiDoc** (`repayment_export.rs:171-178`):
```rust
#[derive(OpenApi)]
#[openapi(
    paths(export_repayment),
    components(schemas(ExportQuery, ExportIncludeQuery)),
    tags((name = "RepaymentExport",
          description = "Phase 11: PDF-Export ..."))
)]
pub struct ApiDoc;
```

---

### `genossi_service/src/member_document.rs:48-101` (MODIFY)

**Analog:** Existierende `RepaymentMail`-Variante in derselben Datei (Phase 10 D-09).

**Variante-Eintrag** (`member_document.rs:48-58`):
```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DocumentType {
    JoinDeclaration,
    JoinConfirmation,
    ShareIncrease,
    Other,
    // Phase 10 D-09: persistent anchor for repayment-mail send events.
    RepaymentMail,
    // ★ Phase 13 hier hinzufuegen:
    // RepaymentLetter,
}
```

**as_str / from_str / is_singleton / template_path** (`member_document.rs:60-101`):
```rust
impl DocumentType {
    pub fn as_str(&self) -> &str {
        match self {
            DocumentType::JoinDeclaration => "join_declaration",
            DocumentType::JoinConfirmation => "join_confirmation",
            DocumentType::ShareIncrease => "share_increase",
            DocumentType::Other => "other",
            DocumentType::RepaymentMail => "repayment_mail",
            // ★ Phase 13: DocumentType::RepaymentLetter => "repayment_letter",
        }
    }
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "join_declaration" => Some(DocumentType::JoinDeclaration),
            // ... existing arms ...
            "repayment_mail" => Some(DocumentType::RepaymentMail),
            // ★ Phase 13: "repayment_letter" => Some(DocumentType::RepaymentLetter),
            _ => None,
        }
    }
    pub fn is_singleton(&self) -> bool {
        matches!(
            self,
            DocumentType::JoinDeclaration | DocumentType::JoinConfirmation
        )
        // ★ RepaymentLetter NICHT in singleton-Liste (D-13-08)
    }
    pub fn template_path(&self) -> Option<&str> {
        match self {
            DocumentType::JoinConfirmation => Some("join_confirmation.typ"),
            DocumentType::JoinDeclaration => Some("join_declaration.typ"),
            DocumentType::ShareIncrease => None,
            DocumentType::Other => None,
            DocumentType::RepaymentMail => None,
            // ★ Phase 13: DocumentType::RepaymentLetter => None,
        }
    }
}
```

**Tests-Pattern** (`member_document.rs:261-289` zeigt die 4 Test-Funktionen pro Variant: `as_str`, `from_str`, `is_singleton`, `template_path`). 1:1 fuer RepaymentLetter spiegeln, ABER `is_singleton` muss `assert!(!...)` lauten und die Begruendung-Kommentar auf D-13-08 referenzieren.

---

### `genossi_service_impl/src/template_storage.rs:10-35` (MODIFY)

**Analog:** Existierender `auszahlungsliste.typ`-Eintrag in derselben Datei (Zeilen 27-34).

**DEFAULT_TEMPLATES-Eintrag-Pattern** (`template_storage.rs:27-34`):
```rust
// Phase 11 (EXPO-01..03): Auszahlungslisten-Export PDF template.
// Required by RepaymentExportServiceImpl::export(ExportFormat::Pdf) - Plan 11.03.
// Without this entry the PDF-Export branch fails with a "template not
// found" InternalError on a fresh installation (Pitfall #1 RESEARCH §Common Pitfalls).
DefaultTemplate {
    path: "auszahlungsliste.typ",
    content: include_bytes!("../../templates/defaults/auszahlungsliste.typ"),
},
```

**Apply to Phase 13:** Neuer Eintrag direkt unter dem `auszahlungsliste.typ`-Eintrag:
```rust
// Phase 13 (D-13-05): RepaymentLetter Default-Template.
// UI-editierbar via /templates-Editor; DEFAULT_TEMPLATES liefert nur Initial-Wert.
DefaultTemplate {
    path: "auszahlungs_anschreiben.typ",
    content: include_bytes!("../../templates/defaults/auszahlungs_anschreiben.typ"),
},
```

---

### `genossi_service_impl/src/pdf_generation.rs` (MODIFY: neue Methoden)

**Analog:** `render_repayment_list` (`pdf_generation.rs:386-441`) + `build_inputs_repayment` (`pdf_generation.rs:776-826`).

**Render-Method-Skelett** (`pdf_generation.rs:386-441`):
```rust
pub fn render_repayment_list(
    &self,
    template_path: &str,
    template_base: &Path,
    phase: &RepaymentPhaseEntity,
    rows: &[RepaymentExportRow],
) -> Result<Vec<u8>, ServiceError> {
    let full_path = template_base.join(template_path);
    let source_text = std::fs::read_to_string(&full_path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            ServiceError::InternalError(Arc::from(format!(
                "template not found: {}", full_path.display()
            )))
        } else {
            ServiceError::InternalError(Arc::from(format!("template io error: {}", e)))
        }
    })?;

    let inputs = build_inputs_repayment(phase, rows);

    let world = TemplateWorld::new(
        &source_text, template_path, template_base.to_path_buf(),
        inputs, &self.fonts, &self.book, &self.package_cache,
    );

    let result = typst::compile::<PagedDocument>(&world);
    let document = match result.output {
        Ok(doc) => doc,
        Err(diagnostics) => {
            let messages: Vec<String> = diagnostics.iter()
                .map(|d| format!("{}", d.message)).collect();
            return Err(ServiceError::InternalError(Arc::from(format!(
                "typst compile errors: {}", messages.join("\n")
            ))));
        }
    };
    // ... typst_pdf::pdf(&document, ...) -> Vec<u8>
}
```

**Apply to Phase 13:**
1. Neue Methode `render_repayment_letter(template_path, template_base, phase, member, ctx) -> Result<Vec<u8>, ServiceError>` (1 Member, 1 PDF). 1:1 dasselbe Read/Compile/Serialise-Skelett.
2. Neue Methode `render_repayment_letters_bundle(template_path, template_base, phase, recipients: &[(MemberEntity, RepaymentContext)]) -> Result<Vec<u8>, ServiceError>` (N Member, 1 PDF mit `#pagebreak()` zwischen Briefen). Bundle-Template iteriert ueber `recipients`-JSON-Array (RESEARCH §Open Questions 1).

**build_inputs Pattern** (`pdf_generation.rs:776-826`):
```rust
fn build_inputs_repayment(phase: &RepaymentPhaseEntity, rows: &[RepaymentExportRow]) -> Dict {
    let mut inputs = Dict::new();
    let date_str = time::OffsetDateTime::now_utc().date().to_string();

    let total_cents: i64 = rows.iter()
        .map(|r| (r.share_count as i64) * phase.share_value).sum();
    let total_amount_str = format!("{},{:02}", total_cents / 100, total_cents % 100);

    let meta = serde_json::json!({
        "title": format!("Auszahlungsliste Geschaeftsjahr {}", phase.fiscal_year),
        "date": date_str,
        "fiscal_year": phase.fiscal_year,
        "row_count": rows.len(),
        "total_amount_str": total_amount_str,
        "phase_id": phase.id.to_string(),
    });
    let meta_json = serde_json::to_string(&meta).expect("meta json serialisable");
    inputs.insert(Str::from("meta"), Value::Str(Str::from(meta_json.as_str())));

    let row_values: Vec<serde_json::Value> = rows.iter().map(|r| {
        serde_json::json!({
            "member_number": r.member_number,
            "name": r.name,
            "iban": r.iban,
            "share_count": r.share_count,
            "amount_str": r.amount_str,
            "purpose": r.purpose,
        })
    }).collect();
    let rows_json = serde_json::to_string(&serde_json::Value::Array(row_values))
        .expect("rows json serialisable");
    inputs.insert(Str::from("rows"), Value::Str(Str::from(rows_json.as_str())));

    inputs
}
```

**Apply to Phase 13:** Neue Helper `build_inputs_repayment_letter(phase, member, ctx)` und `build_inputs_repayment_letters_bundle(phase, recipients)`. Member-JSON enthaelt `bank_account: Option<&str>` (None -> JSON null fuer Typst `#if member.bank_account != none`-Switch, RESEARCH Pitfall #5).

---

### `genossi_bin/src/lib.rs` (MODIFY: DI-Wiring)

**Analog:** `RepaymentExportServiceDependencies`-Block (Z. 291-316) + `RestStateImpl::new()` (Z. 862-881) + `RestStateImpl-Trait-Impl` (Z. 1523-1531).

**Deps-Alias-Pattern** (`lib.rs:291-316`):
```rust
// Phase 11 (EXPO-01..03, EXPO-05): RepaymentExportServiceImpl DI-Aliases.
pub struct RepaymentExportServiceDependencies;
unsafe impl Send for RepaymentExportServiceDependencies {}
unsafe impl Sync for RepaymentExportServiceDependencies {}

impl genossi_service_impl::repayment_export::RepaymentExportServiceDeps
    for RepaymentExportServiceDependencies
{
    type Context = Context;
    type Transaction = Transaction;
    type RepaymentPhaseDao = RepaymentPhaseDao;
    type RepaymentEntryDao = RepaymentEntryDao;
    type MemberDao = MemberDao;
    type PermissionService = PermissionService;
    type TransactionDao = TransactionDao;
}

type RepaymentExportService = genossi_service_impl::repayment_export::RepaymentExportServiceImpl<
    RepaymentExportServiceDependencies,
>;
```

**Apply to Phase 13:** Zwei neue Deps-Aliases:
- `RepaymentLetterServiceDependencies` mit den 5 Export-Deps PLUS `MemberDocumentDao`, `AuditLogDao`, `UuidService`, `DocumentStorage`, `RepaymentContextResolver`-Trait-Bound
- `RepaymentContextResolverDependencies` mit `Transaction`, `RepaymentPhaseDao`, `RepaymentEntryDao`

**Service-Wiring in `RestStateImpl::new()`** (`lib.rs:862-881`):
```rust
// Phase 11 (EXPO-01..03, EXPO-05): RepaymentExportServiceImpl
// Re-uses the existing `pdf_generator` and `template_storage` Arcs.
let repayment_export_service = Arc::new(
    genossi_service_impl::repayment_export::RepaymentExportServiceImpl::<
        RepaymentExportServiceDependencies,
    > {
        transaction_dao: transaction_dao.clone(),
        permission_service: permission_service.clone(),
        repayment_phase_dao: repayment_phase_dao.clone(),
        repayment_entry_dao: repayment_entry_dao.clone(),
        member_dao: member_dao.clone(),
        pdf_generator: pdf_generator.clone(),
        template_base: Arc::new(template_storage.base_path().to_path_buf()),
    },
);
```

**Apply to Phase 13:** Direkt nach `repayment_export_service`-Block:
- `repayment_context_resolver = Arc::new(...)` mit `RepaymentPhaseDao` + `RepaymentEntryDao`-Arcs
- `repayment_letter_service = Arc::new(...)` mit allen 10 Deps (`document_storage.clone()`, `audit_log_dao.clone()`, `uuid_service.clone()`, `member_document_dao.clone()`, `pdf_generator.clone()`, `template_base.clone()`, `repayment_context_resolver.clone()`)

**RestState-Trait-Impl** (`lib.rs:1523-1531`):
```rust
impl genossi_rest::repayment_export::RepaymentExportRestState for RestStateImpl {
    type RepaymentExportService = RepaymentExportService;
    fn repayment_export_service(&self) -> Arc<Self::RepaymentExportService> {
        self.repayment_export_service.clone()
    }
}
```

**Apply to Phase 13:** Analoger Block fuer `RepaymentLetterRestState`. Zusaetzlich `repayment_letter_service: Arc<RepaymentLetterService>` als Feld in `RestStateImpl` (Z. 549-552-Pattern) und im Konstruktor-Return (Z. 973-988-Pattern).

---

### `genossi_rest/src/lib.rs` (MODIFY: Route-Mount)

**Analog:** `repayment_export`-Mount-Block (`lib.rs:5, 279, 449, 654-657`).

**Module-Mount** (`lib.rs:5`):
```rust
pub mod repayment_export;
// ★ Phase 13: pub mod repayment_letter;
```

**ApiDoc-Nest** (`lib.rs:279`):
```rust
(path = "/api/repayment-phase/{phase_id}/export", api = repayment_export::ApiDoc),
// ★ Phase 13: (path = "/api/repayment-phase/{phase_id}/letters", api = repayment_letter::ApiDoc),
```

**Trait-Bound auf `create_app`** (`lib.rs:449`):
```rust
+ repayment_export::RepaymentExportRestState
// ★ Phase 13: + repayment_letter::RepaymentLetterRestState
```

**Router-Mount** (`lib.rs:654-657`):
```rust
.nest(
    "/api/repayment-phase",
    repayment_export::generate_export_route::<RestState>(),
)
// ★ Phase 13: zusaetzlicher Mount direkt darunter
// .nest("/api/repayment-phase", repayment_letter::generate_letter_route::<RestState>())
// Axum 0.8.3 merged das mit den existierenden Mounts unter /api/repayment-phase
// (unique segments /{phase_id}/letters/generate).
```

---

### `templates/defaults/auszahlungs_anschreiben.typ` (NEU, Typst-Template)

**Analog (Layout):** `templates/zahlungsanfrage.typ` (letter-pro:3.0.0/letter-simple, Falzmarken, Sender/Recipient/Subject, Logo, Signatur).
**Analog (sys.inputs-JSON-Kontext):** `templates/defaults/auszahlungsliste.typ:25-33`.

**Layout-Skelett** (`templates/zahlungsanfrage.typ:1-70` — vollstaendig):
```typst
#import "@preview/letter-pro:3.0.0": letter-simple

#set text(lang: "de")
#let application = json.decode(sys.inputs.at("application"))
#let today = sys.inputs.at("today")
#let name = [#application.first_name #application.last_name]

#let anrede = if application.salutation == "Herr" {
    "Lieber"
  } else if application.salutation == "Frau" {
    "Liebe"
  } else {
    "Hallo"
  }

#show: letter-simple.with(
  sender: (
    name: "nebenan & unverpackt Muenchen W. eG",
    address: "Willibaldstr. 18, 80687 Muenchen",
    extra: [
      Telefon: #link("tel:08954637600")[+089 - 54 63 76 00]\
      Mitgliederverwaltung: #link("mailto:mv@nebenan-unverpackt.de")[mv\@nebenan-unverpackt.de]\
    ],
  ),
  recipient: [
    #name \
    #application.street #application.house_number \
    #application.postal_code #application.city
  ],
  date: [#today],
  subject: "Eintrittsbestaetigung",
  folding-marks: true
)

#place(top + left, dx: -0.55cm, dy: -0.5cm, image("nebenan-unverpackt-logo.svg", width: 5cm))

#line(length: 16.5cm, stroke: 0.5pt + gray)

#table(
  columns: (1fr, 1fr),
  stroke: none,
  [*Name:*], [#name],
  [*Gezeichnete Anteile:*], [#application.shares],
)

#line(length: 16.5cm, stroke: 0.5pt + gray)
#v(1cm)

#anrede #name,

herzlich willkommen ...

Herzliche Gruesse,

Carolin Weidmann, Dina Beier und Simon Goller
```

**Apply to Phase 13:** Layout 1:1 uebernehmen, aber:
- `application` -> `member` (mit `member_number`, `salutation`, `title`, `first_name`, `last_name`, `street`, `house_number`, `postal_code`, `city`, `bank_account`)
- Neues JSON `repayment` (`share_count`, `payout_amount`, `fiscal_year`)
- Subject: `"Auszahlung deiner Anteile"`
- Reference-Block (Tabelle): Mitgliedsnummer, Anteile zur Auszahlung (`#repayment.share_count`), Auszahlungsbetrag (`#repayment.payout_amount` `€`)
- **IBAN-Switch mit `#if member.bank_account != none ... else ...`** (RESEARCH Pattern 5, vollstaendiger Block):
  ```typst
  #if member.bank_account != none [
    Wir ueberweisen den Betrag in Hoehe von #repayment.payout_amount EUR auf deine
    hinterlegte IBAN: *#member.bank_account*.
  ] else [
    *Wir haben keine IBAN von dir hinterlegt* — bitte teile sie uns unter
    #link("mailto:mv@nebenan-unverpackt.de")[mv\@nebenan-unverpackt.de] mit,
    damit wir dir den Betrag in Hoehe von #repayment.payout_amount EUR ueberweisen koennen.
  ]
  ```
- Signatur-Block identisch ("Herzliche Gruesse, Carolin Weidmann, Dina Beier und Simon Goller")

**Bundle-Template (Plan-Discretion, RESEARCH §Open Questions 1):** Empfehlung — separates `auszahlungs_anschreiben_bundle.typ` mit `#for (i, r) in recipients.enumerate() { ... #if i < recipients.len() - 1 [#pagebreak()] }` und Single-Letter-Logik via `#import`.

---

### `genossi-frontend/src/component/repayment_entry_list.rs:220-296` (MODIFY)

**Analog:** Existierender Massenmail-Button in derselben Datei (Z. 226-249).

**Bulk-Button-Pattern (Massenmail)** (`repayment_entry_list.rs:226-249`):
```rust
button {
    r#type: "button",  // ★ Phase 12 D-01 Pflicht
    class: if selected_count == 0 {
        "bg-gray-200 text-gray-500 px-3 py-2 rounded text-sm cursor-not-allowed min-h-[44px]"
    } else {
        "bg-blue-600 hover:bg-blue-700 text-white px-3 py-2 rounded text-sm min-h-[44px]"
    },
    disabled: selected_count == 0,
    onclick: move |_| {
        let selected_set = selected_ids.read().clone();
        let member_ids: Vec<Uuid> = entries
            .read()
            .iter()
            .filter(|e| selected_set.contains(&e.id))
            .map(|e| e.member_id)
            .collect();
        on_mail_request.call(member_ids);
    },
    "{i18n.t(Key::RepaymentEntryBulkMailButton)} ({selected_count})"
}
```

**Apply to Phase 13:** Neuer Button direkt unter Massenmail-Button. **Wichtiger Unterschied:** Phase 13 schickt `entry_ids` (NICHT member_ids — Server aggregiert serverseitig via Resolver, CONTEXT D-13-03/04):
```rust
button {
    r#type: "button",
    class: if selected_count == 0 { /* disabled-style */ } else { /* purple/active style */ },
    disabled: selected_count == 0,
    onclick: move |_| {
        let ids = selected_ids.read().clone();  // direkt entry_ids
        on_letter_request.call(ids);
    },
    "{i18n.t(Key::RepaymentEntryBulkLetterButton)} ({selected_count})"
}
```

**Neue EventHandler-Prop** (analog `on_mail_request: EventHandler<Vec<Uuid>>` Z. 140):
```rust
on_letter_request: EventHandler<Vec<Uuid>>,  // entry_ids, NICHT member_ids
```

**Grep-Gate-Check:** Plan muss in Acceptance verifizieren:
```bash
rg 'button\s*\{' genossi-frontend/src/component/repayment_entry_list.rs | rg -v 'r#type:' | wc -l
# expected: 0
```

---

### `genossi-frontend/src/page/repayment_phase_details.rs:240-260` (MODIFY)

**Analog (Bubble-Wiring):** Bestehendes `on_mail_request`-Handler in derselben Datei (Z. 244-258).
**Analog (Browser-Save):** `genossi-frontend/src/page/assembly_details.rs:355-395` (ExportTab Browser-Save).

**Page-Handler Massenmail** (`repayment_phase_details.rs:244-258`):
```rust
on_mail_request: move |ids: Vec<uuid::Uuid>| {
    if ids.is_empty() { return; }
    let url = build_mail_redirect_url(phase_id, &ids);
    if let Some(window) = web_sys::window() {
        let _ = window.location().set_href(&url);
    }
},
```

**Browser-Save Pattern** (`assembly_details.rs:362-395`):
```rust
spawn(async move {
    let cfg = CONFIG.read().clone();
    match api::export_attendance_url(&cfg, assembly_id, &fmt, &inc).await {
        Ok(blob_url) => {
            if let Some(window) = web_sys::window() {
                if let Some(document) = window.document() {
                    if let Ok(elem) = document.create_element("a") {
                        let _ = elem.set_attribute("href", &blob_url);
                        let dl_filename = format!("gv-{}-teilnehmer.{}", date_for_dl, fmt);
                        let _ = elem.set_attribute("download", &dl_filename);
                        if let Ok(html_elem) = elem.dyn_into::<web_sys::HtmlElement>() {
                            html_elem.click();
                        }
                    }
                    // T-06-16 mitigation: release the blob URL after click.
                    let _ = web_sys::Url::revoke_object_url(&blob_url);
                }
            }
        }
        Err(e) => {
            // AppError.status mapping to i18n key...
        }
    }
});
```

**Apply to Phase 13:** Neuer `on_letter_request`-Handler in der `RepaymentEntryList`-Instanz. Innerhalb spawn:
1. `api::generate_repayment_letters(&cfg, phase_id, ids).await` (POST + JSON-Body + .blob() + create_object_url)
2. Bei Ok(blob_url): `<a>`-Element-Trick mit Filename `format!("auszahlungs_anschreiben_GJ_{}.pdf", phase.fiscal_year)`, `revoke_object_url` danach
3. Bei Err(e): `show_toast(&mut toast_messages, &mut toast_counter, e.message)`
4. Bei Erfolg zusaetzlicher Toast: "N Briefe erzeugt. Vergiss nicht, die Eintraege anschliessend als angeschrieben zu markieren." (CONTEXT D-13-09 / Frontend Discretion)

---

### `genossi-frontend/src/api.rs` (MODIFY: neue Funktion `generate_repayment_letters`)

**Analog:** `export_attendance_url` (`api.rs:1891-1937`).

**fetch-+-blob-+-create_object_url Pattern** (`api.rs:1891-1937`):
```rust
pub async fn export_attendance_url(
    config: &Config,
    assembly_id: Uuid,
    format: &str,
    include: &str,
) -> Result<String, AppError> {
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;

    let url = format!(
        "{}/api/assembly/{}/attendance-export/{}?include={}",
        config.backend, assembly_id, format, include
    );

    let mut opts = web_sys::RequestInit::new();
    opts.set_method("GET");

    let request = web_sys::Request::new_with_str_and_init(&url, &opts)
        .map_err(|e| AppError::new(None, "Verbindungsfehler", Some(format!("{:?}", e))))?;

    let window = web_sys::window()
        .ok_or_else(|| AppError::new(None, "Verbindungsfehler", None))?;
    let resp_value = JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| AppError::new(None, "Verbindungsfehler", Some(format!("{:?}", e))))?;

    let resp: web_sys::Response = resp_value.dyn_into()
        .map_err(|_| AppError::new(None, "Verbindungsfehler", None))?;

    if !resp.ok() {
        return Err(map_web_response_error(&resp).await);
    }

    let blob = JsFuture::from(resp.blob().unwrap()).await
        .map_err(|e| AppError::new(None, "Verbindungsfehler", Some(format!("{:?}", e))))?;
    let blob: web_sys::Blob = blob.dyn_into()
        .map_err(|_| AppError::new(None, "Verbindungsfehler", None))?;

    let blob_url = web_sys::Url::create_object_url_with_blob(&blob)
        .map_err(|e| AppError::new(None, "Verbindungsfehler", Some(format!("{:?}", e))))?;

    Ok(blob_url)
}
```

**Apply to Phase 13:** Neue Funktion `generate_repayment_letters(config, phase_id, entry_ids: Vec<Uuid>) -> Result<String, AppError>` mit identischer Struktur, ABER:
- `opts.set_method("POST")`
- `headers.set("Content-Type", "application/json")`
- `let body = serde_json::json!({ "entry_ids": entry_ids }).to_string()`; `opts.set_body(&JsValue::from_str(&body))`
- URL: `"{}/api/repayment-phase/{}/letters/generate"`
- Restliche Blob-Logik 1:1 spiegeln

---

### `genossi_bin/tests/e2e_tests.rs` (MODIFY: 8 neue Tests)

**Analog:** `test_export_repayment_pdf_open_happy_path` (`e2e_tests.rs:13364-13460`).

**E2E-Test-Skelett** (`e2e_tests.rs:13422-13460`):
```rust
let phase = create_open_repayment_phase(&client, &server, fiscal_year, 12000).await;

let resp = client
    .get(server.url(&format!(
        "/api/repayment-phase/{}/export/pdf?include=open",
        phase.id
    )))
    .send()
    .await
    .expect("GET export failed");
assert_eq!(resp.status(), StatusCode::OK);
assert_eq!(
    resp.headers().get("content-type").and_then(|h| h.to_str().ok()).unwrap_or_default(),
    "application/pdf"
);
let cd = resp.headers().get("content-disposition")
    .and_then(|h| h.to_str().ok()).unwrap_or_default().to_string();
assert!(cd.contains(&format!("auszahlung-{}-open.pdf", fiscal_year)));

let bytes = resp.bytes().await.expect("read bytes");
assert!(bytes.starts_with(b"%PDF-"));
assert!(bytes.len() > 1000);
```

**Apply to Phase 13:** 8 Tests (analog CONTEXT.md §E2E-Tests + RESEARCH §Validation):
1. **Happy Path 3-Entries-2-Member** -> 2 MemberDocuments + 1 Bundle-PDF (response bytes start with `%PDF-`)
2. **Multi-Entry-Aggregation** (2 entries fuer 1 Member) -> 1 MemberDocument; share_count = SUM (List-Endpoint pruefen)
3. **Permission-Denied** (Helper-Auth -> 403)
4. **Status-Gate** (Phase `Preparation` -> 409 `phase_not_active`)
5. **entry_phase_mismatch** (entry_ids fremder Phase -> 400 oder 409 je nach Mapping)
6. **IBAN-NULL** (Member ohne `bank_account` -> PDF rendert OK)
7. **Audit-Hashchain valide** (`GET /api/audit/verify` -> 200, Pattern aus e2e_tests.rs:7517,7543)
8. **Idempotenz** (2x derselbe Call -> 2 MemberDocuments)

POST-Body-Pattern (existing `client.post(...).json(...)`):
```rust
let body = serde_json::json!({ "entry_ids": [entry1.id, entry2.id, entry3.id] });
let resp = client
    .post(server.url(&format!("/api/repayment-phase/{}/letters/generate", phase.id)))
    .json(&body)
    .send().await.expect("POST letters/generate failed");
```

---

## Shared Patterns

### Permission-Funnel (load -> admin -> status)

**Source:** `genossi_service_impl/src/repayment_export.rs:77-110`
**Apply to:** `RepaymentLetterServiceImpl::generate` (CRITICAL: gleiche Reihenfolge, sonst Status-Leak an non-admin per Pitfall #2).

```rust
async fn check_admin_and_phase_status(
    &self, phase_id: Uuid,
    context: Authentication<Deps::Context>,
    tx: Deps::Transaction,
) -> Result<RepaymentPhaseEntity, ServiceError> {
    // 1. Load (404 if missing)
    let phase = self.repayment_phase_dao.find_by_id(phase_id, tx).await?
        .ok_or(ServiceError::EntityNotFound(phase_id))?;
    // 2. Admin gate (403). Authentication::Full short-circuits.
    match &context {
        Authentication::Full => {}
        Authentication::Context(_) => {
            self.permission_service
                .check_permission(ADMIN_PRIVILEGE, context).await?;
        }
    }
    // 3. Status gate (409): Open ODER Closed akzeptiert.
    match phase.status {
        RepaymentPhaseStatus::Open | RepaymentPhaseStatus::Closed => {}
        RepaymentPhaseStatus::Preparation => {
            return Err(ServiceError::Conflict(Arc::from("phase_not_active")));
        }
    }
    Ok(phase)
}
```

### audited_create! fuer MemberDocument

**Source:** `genossi_service_impl/src/member_document.rs:139-147` + `audit_macros.rs:5-36`
**Apply to:** Jeder Brief-Erzeugungs-Loop-Iteration im Letter-Service. **Sequential await** (kein `futures::join_all`, RESEARCH Pitfall #4).

```rust
let doc_entity: genossi_dao::member_document::MemberDocumentEntity = (&new_doc).into();
crate::audited_create!(
    self,
    self.member_document_dao,
    &doc_entity,
    REPAYMENT_LETTER_PROCESS,  // const &str
    &user_id,                  // current_user_id via permission_service
    tx
);
```

**Voraussetzung:** Letter-Service-Struct hat `audit_log_dao: Arc<...>` UND `uuid_service: Arc<...>` als Felder (vom Macro intern erwartet, `audit_macros.rs:3`).

### Direct-Download HTTP-Response

**Source:** `genossi_rest/src/repayment_export.rs:147-152` + `http_util::content_disposition_attachment`
**Apply to:** REST-Handler-Response.

```rust
let cd = http_util::content_disposition_attachment(&result.filename);
Ok(Response::builder()
    .status(200)
    .header("Content-Type", "application/pdf")
    .header("Content-Disposition", &cd)
    .body(Body::from(result.bundle_bytes))
    .unwrap())
```

### Soft-Delete-Filter via DAO-Default-Impl

**Source:** `repayment_export.rs:197-213` (Defense-in-Depth: trotz DAO-Default-Filter zusaetzlich `entry.deleted.is_some() -> continue`).
**Apply to:** Alle DAO-Reads im Letter-Service. Resolver muss `entry.deleted.is_none()` explizit filtern (RESEARCH Pitfall #6).

### Euro-Format Pattern (Phase 10 D-04)

**Source:** `genossi_mail/src/worker.rs:351-353` + `genossi_service_impl/src/repayment_export.rs:148-151`
**Apply to:** `RepaymentContext::payout_amount`.

```rust
let cents: i64 = (share_count as i64) * (phase.share_value);
let payout_amount = format!("{},{:02}", cents / 100, cents % 100);
// "X,YZ" — KEIN Tausenderpunkt, KEIN Euro-Symbol, KEIN .abs() (Domain-Constraint share_count >= 0)
```

### Frontend Button-Pattern (Phase 12 D-01)

**Source:** `genossi-frontend/src/component/repayment_entry_list.rs:220-296` (alle Bulk-Buttons in der Datei).
**Apply to:** Neuer "Anschreiben erzeugen"-Button.

```rust
button {
    r#type: "button",   // ★ MANDATORY (sonst Page-Reload-Bug, Phase 12 D-01)
    class: if selected_count == 0 { /* disabled */ } else { /* active */ },
    disabled: selected_count == 0,
    onclick: move |_| { /* handler */ },
    "Label ({selected_count})"
}
```

**Grep-Gate (Plan-Acceptance):**
```bash
rg 'button\s*\{' genossi-frontend/src/component/repayment_entry_list.rs \
    genossi-frontend/src/page/repayment_phase_details.rs \
    | rg -v 'r#type:'
# expected: empty (Phase 12 D-02)
```

### Frontend Browser-Save (Blob -> Download)

**Source:** `genossi-frontend/src/page/assembly_details.rs:362-395` (ExportTab) — `create_element("a") -> set_attribute("href", blob_url) -> set_attribute("download", filename) -> .click() -> revoke_object_url`.
**Apply to:** `repayment_phase_details.rs::on_letter_request`-Handler.

### Test-Mocks Hand-Rolled mock!-Block

**Source:** `genossi_service_impl/src/repayment_export.rs:282-572` (hand-rolled `mock!`-Bloecke statt `automock`, weil DAOs zusaetzliche Methoden haben).
**Apply to:** Letter-Service Unit-Tests (`mock! TestPhaseDao`, `TestEntryDao`, `TestMemberDao`, `TestMemberDocumentDao`, `TestPermissionService`, `TestTxDao`, `TestDocumentStorage`, `TestUuidService`, `TestAuditLogDao`, `TestRepaymentContextResolver`).

### Pure-Function-Tests fuer aggregations-/grouping-Logik

**Source:** `repayment_export.rs:118-171` (`filter_and_enrich_rows`) + `repayment_export.rs:718-820` (mehrere pure-fn-Tests).
**Apply to:** Resolver-Pure-Function `aggregate_for_member(...)` und Letter-Service-Helper `group_entries_by_member(...)` — direkt testbar ohne Mocks.

---

## No Analog Found

Keine. Alle 13 Files haben starke Vorbilder im Repo (Confidence HIGH per RESEARCH-Summary).

Einzige Plan-Discretion-Stelle ohne direktes Code-Vorbild: **Bundle-PDF-Render-Strategie** (separates `auszahlungs_anschreiben_bundle.typ` mit `#for ... #pagebreak()` vs. lopdf-Merge). RESEARCH empfiehlt Typst-Loop (kein neues Dependency); RESEARCH §Open Questions 1 dokumentiert das.

## Metadata

**Analog search scope:**
- `genossi_service/src/`, `genossi_service_impl/src/` (Service Trait + Impl Vorlagen)
- `genossi_rest/src/` (REST-Handler + Route-Mount-Pattern)
- `genossi_bin/src/lib.rs` (DI-Wiring Phase 11)
- `genossi_mail/src/worker.rs` (Phase-10-Inline-Aggregation, Resolver-Vorbild)
- `genossi_service_impl/src/pdf_generation.rs` + `template_storage.rs` + `document_storage.rs`
- `templates/` und `templates/defaults/` (Layout + sys.inputs-Pattern)
- `genossi-frontend/src/component/`, `src/page/`, `src/api.rs`
- `genossi_bin/tests/e2e_tests.rs` (E2E-Test-Vorbild fuer PDF-Download-Pattern)

**Files scanned:** 13 Source-Files + 2 Templates + 1 E2E-Test-File + 2 Frontend-Files

**Pattern extraction date:** 2026-06-01
