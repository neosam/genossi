# Phase 9: Auszahlungs-Buchung (atomisch + auditiert) — Pattern Map

**Mapped:** 2026-05-31
**Files analyzed:** 7 (5 Code + 1 Test + 1 Docs)
**Analogs found:** 7 / 7 (alle direkt im Repo; keine externen Vorlagen)
**Sprache:** Deutsch für Erklärungen, Code-Auszüge auf Englisch/Rust

---

## File Classification

| Neue/Änderung Datei | Rolle | Daten-Fluss | Closest Analog | Match-Qualität |
|---------------------|-------|-------------|----------------|----------------|
| `genossi_service/src/repayment_entry.rs` | trait def | request-response | `genossi_service/src/repayment_entry.rs` (Phase 8 Trait, gleiche Datei — bestehende Methoden-Signaturen-Pattern für `update_repayment_entry`, `batch_toggle_status`) + `genossi_service/src/repayment_phase.rs` (für reine Action-Endpoints `open_repayment_phase`, `close_repayment_phase`) | exact |
| `genossi_service_impl/src/repayment_entry.rs` (Service-Impl + Konstante + Mock-Ergänzung) | service impl | CRUD + cross-entity cascade | `RepaymentEntryServiceImpl::update_repayment_entry` (Z. 169–291, Re-Read-Pattern) **+** `RepaymentEntryServiceImpl::batch_toggle_status` (Z. 379–512, Multi-Step in Single-Tx) **+** `MemberActionServiceImpl::create` (Z. 284–352, audited_create + recalc_migrated) | exact |
| `genossi_service_impl/src/member_action.rs` | utility (visibility) | n/a | derselbe File Z. 32 (`pub(crate) fn compute_migration_status`) | exact — nur Visibility-Wechsel |
| `genossi_rest/src/repayment_entry.rs` (POST `/mark-paid-out` Handler + Route + ApiDoc) | REST handler | request-response action endpoint (no body) | `open_repayment_phase` in `genossi_rest/src/repayment_phase.rs:234–268` **+** bestehende Handler-Struktur in `genossi_rest/src/repayment_entry.rs:144–177` (Path-Extractor + `error_handler`-Wrapper) | exact |
| `genossi_bin/src/lib.rs` (DI-Wiring) | config / DI | n/a | `genossi_bin/src/lib.rs:216–237` (`RepaymentEntryServiceDependencies`-Block) **+** Z. 765–775 (`RepaymentEntryServiceImpl{}`-Konstruktion) | exact |
| `genossi_bin/tests/e2e_tests.rs` (4 E2E-Tests) | E2E test | full HTTP request/response | `test_helper_token_redeem_race_one_succeeds_one_fails` in `e2e_tests.rs:8783–8821` (Race via `tokio::join!`) **+** bestehende Setup-Helper aus Phase 8 (`create_member_with_exit_date`, `create_open_repayment_phase`) | exact |
| `.planning/REQUIREMENTS.md` | docs | n/a | PAYO-01..04 Zeilen — selbe Datei | exact |

---

## Pattern Assignments

### `genossi_service/src/repayment_entry.rs` (trait def, request-response)

**Analog:** Bestehende Trait-Methoden in derselben Datei (für Pattern-Konsistenz) sowie `genossi_service/src/repayment_phase.rs` für reine Action-Endpoint-Signaturen (`open_repayment_phase`, `close_repayment_phase`).

**Imports + Trait-Annotation** (`genossi_service/src/repayment_entry.rs:22-30`):

```rust
use async_trait::async_trait;
use genossi_dao::repayment_entry::{RepaymentEntryEntity, RepaymentEntryStatus};
use mockall::automock;
use std::fmt::Debug;
use std::sync::Arc;
use uuid::Uuid;

use crate::permission::Authentication;
use crate::ServiceError;
```

Keine neuen Imports nötig.

**Trait-Header und Associated-Types** (`genossi_service/src/repayment_entry.rs:120-124`):

```rust
#[automock(type Context = (); type Transaction = genossi_dao::MockTransaction;)]
#[async_trait]
pub trait RepaymentEntryService {
    type Context: Clone + Debug + PartialEq + Eq + Send + Sync + 'static;
    type Transaction: genossi_dao::Transaction;
```

`#[automock]` generiert `MockRepaymentEntryService` mit `expect_mark_paid_out()` automatisch — keine manuelle Mock-Erweiterung im Trait-Modul nötig.

**Method-Signatur-Vorlage** (`genossi_service/src/repayment_entry.rs:138-143`, `update_repayment_entry`):

```rust
/// Update share_count_to_pay_out and/or status of an existing entry.
/// Edit-Matrix (D-05/D-06/ENTR-04). PaidOut als Target → 409.
/// Optimistic locking via version. Audit-process `repayment-entry.update`.
async fn update_repayment_entry(
    &self,
    id: Uuid,
    update: &RepaymentEntryUpdate,
    context: Authentication<Self::Context>,
) -> Result<RepaymentEntry, ServiceError>;
```

Phase 9 ergänzt direkt darunter (oder am Ende des Traits) eine `mark_paid_out`-Methode **ohne** Update-Body-Parameter (Action-Endpoint, vgl. Phase 7 D-03):

```rust
/// Mark a RepaymentEntry as PaidOut with atomic Cascade:
/// (1) audited_create! MemberAction::Verkauf with shares_change = -N,
/// (2) audited_update! Member.current_shares -= N + action_count += 1,
/// (3) audited_update! RepaymentEntry.status = PaidOut.
/// All three writes commit in a single SQLite-Tx with shared process
/// `repayment-entry.mark-paid-out`. Final per PAYO-04 (no toggle-back).
/// Pre-Conditions: Entry.status ∈ {Open, Contacted}, Phase.status == Open,
/// Member.current_shares >= Entry.share_count_to_pay_out (PAYO-03).
/// Audit-process `repayment-entry.mark-paid-out`. Requires `admin`.
async fn mark_paid_out(
    &self,
    id: Uuid,
    context: Authentication<Self::Context>,
) -> Result<RepaymentEntry, ServiceError>;
```

**Compile-Test-Erweiterung** (`genossi_service/src/repayment_entry.rs:272-283`):

```rust
#[test]
fn test_mock_repayment_entry_service_compiles() {
    let mut mock = MockRepaymentEntryService::new();
    let _ = mock.expect_create_repayment_entry();
    let _ = mock.expect_update_repayment_entry();
    let _ = mock.expect_delete_repayment_entry();
    let _ = mock.expect_get_repayment_entry();
    let _ = mock.expect_list_repayment_entries_by_phase();
    let _ = mock.expect_batch_toggle_status();
    // Phase 9 — neu:
    let _ = mock.expect_mark_paid_out();
}
```

**Abweichung vom Analog:** Keine. Action-Endpoint ohne Submission-DTO ist exakt das `open_repayment_phase`/`close_repayment_phase`-Pattern aus Phase 7 — nur dass die Service-Trait-Methode hier in `RepaymentEntryService` lebt und `Result<RepaymentEntry, ...>` statt `Result<RepaymentPhase, ...>` returnt.

---

### `genossi_service_impl/src/repayment_entry.rs` — `gen_service_impl!`-Block-Erweiterung

**Analog:** Phase-8-Block in derselben Datei (`genossi_service_impl/src/repayment_entry.rs:48-58`).

**Aktueller Stand:**

```rust
gen_service_impl! {
    struct RepaymentEntryServiceImpl: RepaymentEntryService = RepaymentEntryServiceDeps {
        RepaymentEntryDao: RepaymentEntryDao<Transaction = Self::Transaction> = repayment_entry_dao,
        RepaymentPhaseDao: RepaymentPhaseDao<Transaction = Self::Transaction> = repayment_phase_dao,
        MemberDao: MemberDao<Transaction = Self::Transaction> = member_dao,
        AuditLogDao: AuditLogDao<Transaction = Self::Transaction> = audit_log_dao,
        PermissionService: PermissionService<Context = Self::Context> = permission_service,
        UuidService: UuidService = uuid_service,
        TransactionDao: TransactionDao<Transaction = Self::Transaction> = transaction_dao,
    }
}
```

**Phase-9-Erweiterung — eine neue Dep-Zeile** (nach `MemberDao`-Zeile, vor `AuditLogDao`):

```rust
        MemberDao: MemberDao<Transaction = Self::Transaction> = member_dao,
        MemberActionDao: MemberActionDao<Transaction = Self::Transaction> = member_action_dao,  // <-- NEU (Phase 9)
        AuditLogDao: AuditLogDao<Transaction = Self::Transaction> = audit_log_dao,
```

**Import-Ergänzung** im selben File (nach Z. 26-27):

```rust
use genossi_dao::member::MemberDao;
use genossi_dao::member_action::{ActionType, MemberActionDao, MemberActionEntity};  // <-- NEU
use genossi_dao::repayment_entry::{RepaymentEntryDao, RepaymentEntryEntity, RepaymentEntryStatus};
```

**Konstanten-Ergänzung** (nach Z. 45):

```rust
const REPAYMENT_ENTRY_PROCESS_BATCH_TOGGLE: &str = "repayment-entry.batch-toggle";
const REPAYMENT_ENTRY_PROCESS_MARK_PAID_OUT: &str = "repayment-entry.mark-paid-out";  // <-- NEU (D-01)
const ADMIN_PRIVILEGE: &str = "admin";
```

---

### `genossi_service_impl/src/repayment_entry.rs` — `mark_paid_out`-Impl (service impl, cross-entity cascade)

**Primärer Analog:** `RepaymentEntryServiceImpl::update_repayment_entry` (Z. 169–291) für Re-Read-Pattern; `batch_toggle_status` (Z. 379–512) für Multi-Step-Cascade in einer Tx; `MemberActionServiceImpl::create` (Z. 284–352) für audited_create + recalc.

**Tx-Begin + Permission-Check-Pattern** (`repayment_entry.rs:175-184`):

```rust
let tx = self.transaction_dao.use_transaction(None).await?;

let user_id = self
    .permission_service
    .current_user_id(context.clone())
    .await?
    .unwrap_or_else(|| "SYSTEM".to_string());
self.permission_service
    .check_permission(ADMIN_PRIVILEGE, context)
    .await?;
```

Phase 9 reproduziert das 1:1 (CONTEXT.md `<canonical_refs>` "Cascade-Owner", D-09 Schritt 1+2).

**Entity-Load + Status-Guard-Pattern** (`repayment_entry.rs:186-208`):

```rust
let mut entity = self
    .repayment_entry_dao
    .find_by_id(id, tx.clone())
    .await?
    .ok_or(ServiceError::EntityNotFound(id))?;

// D-05: PaidOut ist final
if entity.status == RepaymentEntryStatus::PaidOut {
    return Err(ServiceError::Conflict(Arc::from(
        "Cannot update: entry is PaidOut; final per PAYO-04 (Phase 9)",
    )));
}
```

Phase 9 übernimmt dieses Pattern für Schritt 3 (Entry-Load + Status-Guard `∈ {Open, Contacted}`) und Schritt 4 (Phase-Load + `status == Open`). **Abweichung:** Bei Phase-`None` ist das ein referentieller Inkonsistenz-Fehler (Entry zeigt auf nicht-existente Phase), nicht ein User-NotFound — mappen auf `ServiceError::InternalError` (siehe RESEARCH Pitfall #5).

**Validation-Inline-Pattern** (`repayment_entry.rs:67-91`, `validate_entry_create`):

```rust
let mut errors: Vec<ValidationFailureItem> = Vec::new();
if share_count_to_pay_out > member_current_shares {
    errors.push(ValidationFailureItem {
        field: Arc::from("share_count_to_pay_out"),
        message: Arc::from(format!(
            "must be <= member current_shares ({}), got {}",
            member_current_shares, share_count_to_pay_out
        )),
    });
}
if !errors.is_empty() {
    return Err(ServiceError::ValidationError(errors));
}
```

Phase 9 PAYO-03 nutzt das gleiche Schema, aber inline (nicht via Helper-Funktion) — CONTEXT D-13 erlaubt beides; Empfehlung Inline weil nur ein einziger Check (vs. zwei in `validate_entry_create`).

**audited_create!-Aufruf** (Vorlage `MemberActionServiceImpl::create`, `member_action.rs:336-344`):

```rust
let action_entity: genossi_dao::member_action::MemberActionEntity = (&new_action).into();
crate::audited_create!(
    self,
    self.member_action_dao,
    &action_entity,
    MEMBER_ACTION_SERVICE_PROCESS,
    &user_id,
    tx
);
```

**Phase-9-Abweichung:** Statt `MEMBER_ACTION_SERVICE_PROCESS` wird `REPAYMENT_ENTRY_PROCESS_MARK_PAID_OUT` benutzt (D-01: gemeinsamer Process-String für alle 3 Cascade-Writes). Statt `new_action.into()` baut Phase 9 das `MemberActionEntity` direkt (kein Service-zu-Service-Detour, D-08). Concrete builder (aus RESEARCH §"Cascade Walkthrough" Z. 500-512):

```rust
let now = time::OffsetDateTime::now_utc();
let today: time::Date = now.date();
let created = time::PrimitiveDateTime::new(now.date(), now.time());
let comment_str = format!("Anteils-Rückzahlung Phase {}", phase.fiscal_year);
let action_entity = MemberActionEntity {
    id: self.uuid_service.new_v4().await,
    member_id: entry.member_id,
    action_type: ActionType::Verkauf,
    date: today,
    shares_change: -entry.share_count_to_pay_out,
    transfer_member_id: None,
    effective_date: None,
    comment: Some(Arc::from(comment_str)),
    created,
    deleted: None,
    version: self.uuid_service.new_v4().await,
};
```

**audited_update!-Aufruf** (Vorlage `repayment_entry.rs:255-263`):

```rust
crate::audited_update!(
    self,
    self.repayment_entry_dao,
    id,
    &entity,
    REPAYMENT_ENTRY_PROCESS_UPDATE,
    &user_id,
    tx
);
```

Phase 9 ruft dieses Macro 2× auf: einmal für `member_dao`/`entry.member_id`/`&member_new`, einmal für `repayment_entry_dao`/`id`/`&entry_new` — beide mit `REPAYMENT_ENTRY_PROCESS_MARK_PAID_OUT`.

**Re-Read-Pattern (BL-01 Phase 8)** (`repayment_entry.rs:265-291`, **Source-Comments mitkopieren**):

```rust
// CR-01 Fix: Re-read to get the new version UUID generated by the DAO.
// Pattern mirrors MemberServiceImpl::update (member.rs:343-348).
// The DAO writes a fresh version via `Uuid::new_v4()` on every UPDATE
// (repayment_entry_dao_impl_sqlite.rs); without this re-read the Service
// returns the pre-update entity, leaving the client with a stale version
// that produces 409 on every follow-up PUT.
//
// BL-01 Fix: Re-Read runs in the SAME transaction as the audited_update!
// above — soft-delete in the same Tx is impossible (single-writer per
// service method), so `None` here is an internal consistency error
// (DAO regression, Tx-Isolation break, corrupted id), NOT a user-facing
// "entity vanished" race. Map to InternalError → HTTP 500, never 404.
let refreshed = self
    .repayment_entry_dao
    .find_by_id(id, tx.clone())
    .await?
    .ok_or_else(|| {
        ServiceError::InternalError(Arc::from(format!(
            "Re-Read after audited_update! returned None for RepaymentEntry {} — \
             internal consistency error (same-tx invariant violated)",
             id
        )))
    })?;
```

Phase 9 reproduziert das **wortgleich** für (a) Re-Read Member nach Schritt 7 und (b) Re-Read RepaymentEntry nach Schritt 9. Die Source-Comments müssen mit, weil sie das Same-Tx-Invariant dokumentieren und der Phase-8-Review-Block-Anker waren.

**recalc_migrated-Pattern** (`member_action.rs:200-224`):

```rust
async fn recalc_migrated(
    &self,
    member_id: Uuid,
    tx: Deps::Transaction,
) -> Result<(), ServiceError> {
    let member = self
        .member_dao
        .find_by_id(member_id, tx.clone())
        .await?
        .ok_or(ServiceError::EntityNotFound(member_id))?;

    let actions = self
        .member_action_dao
        .find_by_member_id(member_id, tx.clone())
        .await?;

    let status = compute_migration_status(&member, &actions);
    let migrated = status.status == MigrationState::Migrated;

    self.member_dao
        .update_migrated(member_id, migrated, tx)
        .await?;

    Ok(())
}
```

**Phase-9-Abweichung (D-10 Option a):** Phase 9 lebt im `RepaymentEntryServiceImpl<Deps>`-Crate — der Helper wird inline in `mark_paid_out` repliziert (~18 LOC), weil ein zweiter Inherent-Helper auf diesem Service-Impl nicht lohnt für eine Einmal-Verwendung. `compute_migration_status` wird über `crate::member_action::compute_migration_status(...)` aufgerufen (nachdem die Visibility auf `pub` geändert wurde) — Fully-qualified Path, weil `crate::member_action` ein anderes Sub-Modul ist. Beim `member.dao.find_by_id`-`None`-Fall: `InternalError` (nicht `EntityNotFound`), weil Tx-Invariant.

**Commit-Pattern** (`repayment_entry.rs:289-290`):

```rust
self.transaction_dao.commit(tx).await?;
Ok(RepaymentEntry::from(&refreshed))
```

Phase 9 1:1.

---

### `genossi_service_impl/src/member_action.rs` — Visibility-Wechsel `pub(crate) → pub`

**Analog:** Selber File Z. 32, eine Zeile.

**Aktueller Stand** (`member_action.rs:32`):

```rust
pub(crate) fn compute_migration_status(
    member: &genossi_dao::member::MemberEntity,
    actions: &[genossi_dao::member_action::MemberActionEntity],
) -> MigrationStatus {
```

**Phase-9-Änderung** (1 Zeile, in-place):

```rust
pub fn compute_migration_status(
    member: &genossi_dao::member::MemberEntity,
    actions: &[genossi_dao::member_action::MemberActionEntity],
) -> MigrationStatus {
```

**Abweichung:** Keine. Bewusste Sichtbarkeitsentscheidung gemäß D-10/RESEARCH Frage 2 Option (a). Funktion ist pure, kein Tx, keine I/O — kein Sicherheits- oder Korrektheits-Risiko durch Pub-Machen.

---

### `genossi_rest/src/repayment_entry.rs` — Handler + Route + ApiDoc (REST handler, action endpoint without body)

**Primärer Analog:** `open_repayment_phase` in `genossi_rest/src/repayment_phase.rs:234–268` (Action-Endpoint ohne Body) + Struktur-Vorlagen aus derselben Datei (`genossi_rest/src/repayment_entry.rs:144-177` für Path-Extractor-Setup).

**utoipa-Annotation-Vorlage** (`repayment_phase.rs:234-246`):

```rust
#[instrument(skip(rest_state))]
#[utoipa::path(
    post,
    tag = "RepaymentPhases",
    path = "/{id}/open",
    params(("id" = Uuid, Path, description = "RepaymentPhase ID")),
    responses(
        (status = 200, description = "Opened", body = RepaymentPhaseTO),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Not found"),
        (status = 409, description = "Conflict (status not Preparation)"),
    ),
)]
```

**Phase-9-Abweichung:** (a) `tag = "RepaymentEntries"` statt `"RepaymentPhases"`; (b) Path `/{id}/mark-paid-out`; (c) **zusätzliche** Status-Codes 400 (PAYO-03 ValidationError) und 500 (BL-01 Re-Read-None). RESEARCH §"REST Handler Sketch" hat die volle Phase-9-utoipa-Annotation komplett spezifiziert (Z. 611-636); diese ist Referenz.

**Handler-Body-Pattern** (`repayment_phase.rs:247-268`):

```rust
pub async fn open_repayment_phase<RestState: RestStateDef + RepaymentPhaseRestState>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path(id): Path<Uuid>,
) -> Response {
    error_handler(
        (async {
            let auth = crate::extract_auth_context(Some(context))?;
            let phase = rest_state
                .repayment_phase_service()
                .open_repayment_phase(id, auth)
                .await?;
            let to = RepaymentPhaseTO::from(&phase);
            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&to)?))
                .unwrap())
        })
        .await,
    )
}
```

**Phase-9-Abweichung:** (a) Trait-Bound `RepaymentEntryRestState`, (b) Service-Aufruf `repayment_entry_service().mark_paid_out(id, auth)`, (c) Response-DTO `RepaymentEntryTO::from(&entry)`. Alles strukturell identisch.

**Route-Registration-Pattern** (`repayment_phase.rs:349-350`):

```rust
.route("/{id}/open", post(open_repayment_phase::<RestState>))
.route("/{id}/close", post(close_repayment_phase::<RestState>))
```

**Phase-9-Anwendung** (in `repayment_entry.rs:302-316`, **am Ende des Builder-Chains**, NACH `/{id}`-Route):

```rust
pub fn generate_route<RestState: RestStateDef + RepaymentEntryRestState>() -> Router<RestState> {
    Router::new()
        .route("/batch-status", post(batch_toggle_status::<RestState>))
        .route(
            "/",
            get(list_repayment_entries::<RestState>).post(create_repayment_entry::<RestState>),
        )
        .route(
            "/{id}",
            get(get_repayment_entry::<RestState>)
                .put(update_repayment_entry::<RestState>)
                .delete(delete_repayment_entry::<RestState>),
        )
        // Phase 9 — NEU:
        .route("/{id}/mark-paid-out", post(mark_paid_out::<RestState>))
}
```

**Pitfall:** Reihenfolge ist hier egal (Axum matcht nach Pfad-Spezifität, und `/{id}/mark-paid-out` ist spezifischer als `/{id}`), **aber** die Konvention der Codebase setzt Action-Endpoints ans Ende (analog `repayment_phase.rs`). Plan folgt der Konvention.

**ApiDoc-Erweiterung-Pattern** (`repayment_entry.rs:318-338`):

```rust
#[derive(OpenApi)]
#[openapi(
    paths(
        list_repayment_entries,
        create_repayment_entry,
        get_repayment_entry,
        update_repayment_entry,
        delete_repayment_entry,
        batch_toggle_status,
        // Phase 9 — NEU:
        mark_paid_out,
    ),
    components(schemas(
        RepaymentEntryTO,
        RepaymentEntryStatusTO,
        CreateRepaymentEntryRequest,
        UpdateRepaymentEntryRequest,
        BatchStatusRequest,
        CloseConflictResponse,
        BatchFailureResponse,
    ))
)]
pub struct ApiDoc;
```

**Pitfall:** `mark_paid_out` hat KEINEN Request-Body → kein `request_body = ...`-Eintrag in der `#[utoipa::path]`-Annotation. Vorbild `open_repayment_phase` (keine `request_body`-Zeile).

**Keine neuen Components/Schemas nötig** — Response = bestehender `RepaymentEntryTO`.

---

### `genossi_bin/src/lib.rs` — DI-Wiring (config / DI)

**Analog:** Bestehender `RepaymentEntryServiceDependencies`-Block + Konstruktor-Aufruf in derselben Datei.

**Deps-Struct-Pattern** (`genossi_bin/src/lib.rs:216-237`):

```rust
pub struct RepaymentEntryServiceDependencies;

unsafe impl Send for RepaymentEntryServiceDependencies {}
unsafe impl Sync for RepaymentEntryServiceDependencies {}

impl genossi_service_impl::repayment_entry::RepaymentEntryServiceDeps
    for RepaymentEntryServiceDependencies
{
    type Context = Context;
    type Transaction = Transaction;
    type RepaymentEntryDao = RepaymentEntryDao;
    type RepaymentPhaseDao = RepaymentPhaseDao;
    type MemberDao = MemberDao;
    type AuditLogDao = AuditLogDao;
    type PermissionService = PermissionService;
    type UuidService = UuidService;
    type TransactionDao = TransactionDao;
}
```

**Phase-9-Erweiterung** (zwischen `MemberDao` und `AuditLogDao`):

```rust
    type MemberDao = MemberDao;
    type MemberActionDao = MemberActionDao;  // <-- NEU (Phase 9)
    type AuditLogDao = AuditLogDao;
```

`MemberActionDao` (Type-Alias auf `genossi_dao_impl_sqlite::member_action::MemberActionDaoImpl` o.ä.) ist bereits in der Datei vorhanden — wird auch von `MemberActionService`, `MemberImportService`, `ApplicationService`, `Member`/`Service` etc. benutzt.

**Konstruktor-Pattern** (`genossi_bin/src/lib.rs:765-775`):

```rust
let repayment_entry_service = Arc::new(
    genossi_service_impl::repayment_entry::RepaymentEntryServiceImpl {
        repayment_entry_dao: repayment_entry_dao.clone(),
        repayment_phase_dao: repayment_phase_dao.clone(),
        member_dao: member_dao.clone(),
        audit_log_dao: audit_log_dao.clone(),
        permission_service: permission_service.clone(),
        uuid_service: uuid_service.clone(),
        transaction_dao: transaction_dao.clone(),
    },
);
```

**Phase-9-Erweiterung** (eine Zeile zwischen `member_dao` und `audit_log_dao`):

```rust
let repayment_entry_service = Arc::new(
    genossi_service_impl::repayment_entry::RepaymentEntryServiceImpl {
        repayment_entry_dao: repayment_entry_dao.clone(),
        repayment_phase_dao: repayment_phase_dao.clone(),
        member_dao: member_dao.clone(),
        member_action_dao: member_action_dao.clone(),  // <-- NEU (Phase 9)
        audit_log_dao: audit_log_dao.clone(),
        permission_service: permission_service.clone(),
        uuid_service: uuid_service.clone(),
        transaction_dao: transaction_dao.clone(),
    },
);
```

**Abweichung:** Keine. `member_action_dao` ist bereits auf Z. 563 als `Arc::new(MemberActionDao::new(pool.clone()))` definiert und wird an `MemberServiceImpl`, `MemberActionServiceImpl`, `ValidationServiceImpl`, `MemberImportServiceImpl`, `ApplicationServiceImpl` geteilt — Phase 9 hängt sich nur an. Pattern-konsistent mit W-02 (single DAO instance per process).

---

### `genossi_bin/tests/e2e_tests.rs` — 4 neue E2E-Tests (E2E test, full HTTP)

**Analog für Race-Test:** `test_helper_token_redeem_race_one_succeeds_one_fails` in `e2e_tests.rs:8783-8821`.

**Race-Pattern (Vollexample, `e2e_tests.rs:8783-8821`):**

```rust
/// HLPR-04: Two parallel redeem requests on the same code via tokio::join!
/// must end up with exactly one 200 (success) and one 410 Gone (already_used).
#[tokio::test]
async fn test_helper_token_redeem_race_one_succeeds_one_fails() {
    let server = setup().await;
    let client = reqwest::Client::new();
    let assembly_id = create_open_assembly_for_helper_test(&client, &server).await;
    let (_token_id, code) =
        create_helper_token_for_test(&client, &server, assembly_id, "Carla").await;

    let url = server.url("/api/helper/redeem");
    let body_a = serde_json::json!({ "code": code.clone() });
    let body_b = serde_json::json!({ "code": code.clone() });

    let (resp_a, resp_b) = tokio::join!(
        client.post(&url).json(&body_a).send(),
        client.post(&url).json(&body_b).send(),
    );
    let status_a = resp_a.unwrap().status();
    let status_b = resp_b.unwrap().status();

    let mut statuses = [status_a, status_b];
    statuses.sort_by_key(|s| s.as_u16());
    assert_eq!(statuses[0], StatusCode::OK, "one must succeed; got {:?}", statuses);
    assert_eq!(statuses[1], StatusCode::GONE, "the other must be 410; got {:?}", statuses);
}
```

**Phase-9-Abweichungen:**

1. `assert_eq!(statuses[1], StatusCode::CONFLICT, ...)` statt `StatusCode::GONE` — der zweite mark_paid_out scheitert mit 409 (Version-Mismatch via Re-Read auf den Entry; siehe RESEARCH Frage 1).
2. POST hat KEINEN Body — kein `.json(&body)`, sondern direkt `client.post(&url).send()`.
3. URL: `/api/repayment-entry/{entry_id}/mark-paid-out`.
4. Setup-Helper: `create_open_repayment_phase` + `create_member_with_exit_date` aus Phase-8-E2E-Set 08-06 (bereits in der Datei vorhanden — keine neuen Helper nötig).

**Test-Funktions-Skelett (Phase 9):**

```rust
/// PAYO-01 SC #5: Two parallel mark_paid_out requests on the same entry via
/// tokio::join! must end up with exactly one 200 (success) and one 409
/// (version-mismatch via DAO UPDATE ... WHERE version = ?). Belegs
/// race-defense at the End-to-End level.
#[tokio::test]
async fn test_mark_paid_out_race_one_succeeds_one_conflicts() {
    let server = setup().await;
    let client = reqwest::Client::new();
    // setup phase + member + entry via Phase-8-helpers
    let phase_id = create_open_repayment_phase(&client, &server).await;
    let member_id = create_member_with_exit_date(&client, &server).await;
    let entry_id = create_open_entry(&client, &server, phase_id, member_id, 3).await;

    let url = server.url(&format!("/api/repayment-entry/{}/mark-paid-out", entry_id));

    let (resp_a, resp_b) = tokio::join!(
        client.post(&url).send(),
        client.post(&url).send(),
    );
    let status_a = resp_a.unwrap().status();
    let status_b = resp_b.unwrap().status();

    let mut statuses = [status_a, status_b];
    statuses.sort_by_key(|s| s.as_u16());
    assert_eq!(statuses[0], StatusCode::OK, "one must succeed; got {:?}", statuses);
    assert_eq!(statuses[1], StatusCode::CONFLICT, "other must be 409; got {:?}", statuses);
}
```

**Analog für Happy-Path / PAYO-03 / PAYO-04:** Standardstruktur bestehender Phase-8-E2E-Tests in derselben Datei. RESEARCH §"E2E-Tests" Z. 695-707 hat alle 4 Test-Signaturen vorgegeben:

1. `test_mark_paid_out_happy_path_cascade` — POST → 200; verify GET Entry status=PaidOut; GET Member current_shares -=3; GET `/api/audit/verify` → valid:true (RESEARCH Frage 10).
2. `test_mark_paid_out_validates_insufficient_shares` — POST → 400 mit `share_count_to_pay_out`-Field.
3. `test_mark_paid_out_blocks_double_payout` — POST → 200; zweiter POST → 409 mit „already paid out".
4. `test_mark_paid_out_race_one_succeeds_one_conflicts` — siehe oben.

**Abweichung:** Plan reduziert auf 4 E2E-Tests (statt 5 in CONTEXT D-09); der Phase-Status-Guard-Test (E2E #4 aus CONTEXT) wird in Unit-Test verschoben (RESEARCH Pitfall #10 — Setup zu komplex über REST).

---

### `genossi_service_impl/src/repayment_entry.rs` — Test-Mock-Erweiterung (test scaffolding)

**Analog:** Bestehende `mock! { pub TestRepaymentEntryDao { ... } }`-Blöcke in derselben Datei (`repayment_entry.rs:556-718` — 5 hand-rolled Mocks).

**Mock-Block-Pattern** (`repayment_entry.rs:556-592`):

```rust
mock! {
    pub TestRepaymentEntryDao {}
    #[async_trait]
    impl RepaymentEntryDao for TestRepaymentEntryDao {
        type Transaction = TestTransaction;
        async fn dump_all(
            &self,
            tx: TestTransaction,
        ) -> Result<Arc<[RepaymentEntryEntity]>, DaoError>;
        async fn create(
            &self,
            entity: &RepaymentEntryEntity,
            process: &str,
            tx: TestTransaction,
        ) -> Result<(), DaoError>;
        // ... weitere Methoden
    }
}
```

**Phase-9-Anwendung — neuer `TestMemberActionDao`-Mock** (anhand `MemberActionDao`-Trait `genossi_dao/src/member_action.rs:99-155`, voll spezifiziert in RESEARCH Frage 9):

```rust
mock! {
    pub TestMemberActionDao {}
    #[async_trait]
    impl MemberActionDao for TestMemberActionDao {
        type Transaction = TestTransaction;
        async fn dump_all(
            &self,
            tx: TestTransaction,
        ) -> Result<Arc<[MemberActionEntity]>, DaoError>;
        async fn create(
            &self,
            entity: &MemberActionEntity,
            process: &str,
            tx: TestTransaction,
        ) -> Result<(), DaoError>;
        async fn update(
            &self,
            entity: &MemberActionEntity,
            process: &str,
            tx: TestTransaction,
        ) -> Result<(), DaoError>;
        async fn all(
            &self,
            tx: TestTransaction,
        ) -> Result<Arc<[MemberActionEntity]>, DaoError>;
        async fn find_by_id(
            &self,
            id: Uuid,
            tx: TestTransaction,
        ) -> Result<Option<MemberActionEntity>, DaoError>;
        async fn find_by_member_id(
            &self,
            member_id: Uuid,
            tx: TestTransaction,
        ) -> Result<Arc<[MemberActionEntity]>, DaoError>;
    }
}
```

**TestDeps-Erweiterung** (`repayment_entry.rs:824-835`):

```rust
struct TestDeps;
impl RepaymentEntryServiceDeps for TestDeps {
    type Context = MockContext;
    type Transaction = TestTransaction;
    type RepaymentEntryDao = MockTestRepaymentEntryDao;
    type RepaymentPhaseDao = MockTestRepaymentPhaseDao;
    type MemberDao = MockTestMemberDao;
    type MemberActionDao = MockTestMemberActionDao;  // <-- NEU (Phase 9)
    type AuditLogDao = MockTestAuditLogDao;
    type PermissionService = MockTestPermissionService;
    type UuidService = StaticUuidService;
    type TransactionDao = MockTestTxDao;
}
```

**`build_service`-Helper-Pattern** (`repayment_entry.rs:939-954`):

```rust
fn build_service(
    entry_dao: MockTestRepaymentEntryDao,
    phase_dao: MockTestRepaymentPhaseDao,
    member_dao: MockTestMemberDao,
    perm_service: MockTestPermissionService,
) -> RepaymentEntryServiceImpl<TestDeps> {
    RepaymentEntryServiceImpl {
        repayment_entry_dao: Arc::new(entry_dao),
        repayment_phase_dao: Arc::new(phase_dao),
        member_dao: Arc::new(member_dao),
        audit_log_dao: Arc::new(make_audit_log_dao_quiet()),
        permission_service: Arc::new(perm_service),
        uuid_service: Arc::new(StaticUuidService),
        transaction_dao: Arc::new(setup_mock_tx_dao()),
    }
}
```

**Phase-9-Erweiterung** — neuer Parameter `action_dao: MockTestMemberActionDao`, neue Konstruktor-Zeile:

```rust
fn build_service(
    entry_dao: MockTestRepaymentEntryDao,
    phase_dao: MockTestRepaymentPhaseDao,
    member_dao: MockTestMemberDao,
    action_dao: MockTestMemberActionDao,  // <-- NEU
    perm_service: MockTestPermissionService,
) -> RepaymentEntryServiceImpl<TestDeps> {
    RepaymentEntryServiceImpl {
        repayment_entry_dao: Arc::new(entry_dao),
        repayment_phase_dao: Arc::new(phase_dao),
        member_dao: Arc::new(member_dao),
        member_action_dao: Arc::new(action_dao),  // <-- NEU
        audit_log_dao: Arc::new(make_audit_log_dao_quiet()),
        permission_service: Arc::new(perm_service),
        uuid_service: Arc::new(StaticUuidService),
        transaction_dao: Arc::new(setup_mock_tx_dao()),
    }
}
```

**Sequence-Pattern für Re-Read-Tests** (`repayment_entry.rs:1294-1360`, `test_update_entry_status_open_to_contacted_succeeds`):

```rust
let mut entry_dao = MockTestRepaymentEntryDao::new();
let mut seq = mockall::Sequence::new();
let pre_call_1 = entity.clone();
entry_dao
    .expect_find_by_id()
    .times(1)
    .in_sequence(&mut seq)
    .returning(move |_, _| Ok(Some(pre_call_1.clone())));
let pre_call_2 = entity.clone();
entry_dao
    .expect_find_by_id()
    .times(1)
    .in_sequence(&mut seq)
    .returning(move |_, _| Ok(Some(pre_call_2.clone())));
entry_dao
    .expect_update()
    .times(1)
    .in_sequence(&mut seq)
    .withf(|e: &RepaymentEntryEntity, _process, _tx| {
        e.status == RepaymentEntryStatus::Contacted
    })
    .returning(|_, _, _| Ok(()));
let post_call = post_entity.clone();
entry_dao
    .expect_find_by_id()
    .times(1)
    .in_sequence(&mut seq)
    .returning(move |_, _| Ok(Some(post_call.clone())));
```

**Phase-9-Anwendung:** Für `test_mark_paid_out_happy_path` muss eine Sequence mit MEHR Aufrufen aufgebaut werden (Cascade hat: Entry-Load, Phase-Load, Member-Load, audited_create-internal-getLatestHash, audited_update Member, Member-Re-Read, audited_update Entry, Entry-Re-Read, recalc_migrated-Member-Load, recalc_migrated-Actions-Load). RESEARCH Pitfall #6 weist explizit auf die Notwendigkeit von `mockall::Sequence` hin.

**Abweichung:** Keine vom Pattern. Komplexität ist höher als Phase-8-Update (3× find_by_id), Phase 9 hat ~6 find_by_id-Calls; Sequenz muss entsprechend länger sein. Plan kann ggf. `make_member_action_dao_quiet`-Helper bauen für ein einfacheres Default-Verhalten.

---

### `.planning/REQUIREMENTS.md` — PAYO-01..04 Mark als implemented (docs)

**Analog:** Selbe Datei, bestehende `[x]`-Marks an PHAS-01..05, ENTR-01..06, ASSY-…

**Phase-9-Änderung:** Nur **nach** `/gsd-verify-phase 9` setzen — nicht im Implementation-Step. (CONTEXT „File 7" + RESEARCH File-Manifest #15.)

**Pattern:**

```markdown
- [x] **PAYO-01** Auszahlungs-Cascade: ...
- [x] **PAYO-02** ...
- [x] **PAYO-03** Validation: ...
- [x] **PAYO-04** PaidOut ist final ...
```

**Abweichung:** Keine.

---

## Shared Patterns

### Authentication + Permission-Check

**Source:** `genossi_service_impl/src/repayment_entry.rs:177-184` (alle bestehenden Service-Methoden in dieser Datei).
**Apply to:** `mark_paid_out`-Implementation.

```rust
let user_id = self
    .permission_service
    .current_user_id(context.clone())
    .await?
    .unwrap_or_else(|| "SYSTEM".to_string());
self.permission_service
    .check_permission(ADMIN_PRIVILEGE, context)
    .await?;
```

Konstante `ADMIN_PRIVILEGE` lebt bereits in derselben Datei (Z. 46).

**Pitfall (RESEARCH Pitfall #13):** `"SYSTEM"`-String taucht im Audit-Log nur dann auf, wenn der Permission-Check VOR diesem Fallback fehlschlagen würde — was nicht passiert. Pattern bleibt für Konsistenz übernommen.

### Error Handling

**Source:** Globales `From<ServiceError> for RestError` in `genossi_rest/src/lib.rs:97-113`.
**Apply to:** Alle neuen REST-Handler (`mark_paid_out`).

Mappings:

- `ServiceError::ValidationError(_)` → `RestError::BadRequest(...)` → HTTP 400 (PAYO-03)
- `ServiceError::EntityNotFound(_)` → `RestError::NotFound` → HTTP 404
- `ServiceError::Conflict(s)` → `RestError::Conflict(s)` → HTTP 409 (PAYO-04, Phase-Status, Race-Verlierer)
- `ServiceError::InternalError(_)` → `RestError::InternalError(...)` → HTTP 500 (BL-01 Re-Read-None)
- `ServiceError::PermissionDenied` → `RestError::Unauthorized` → HTTP 401

**Kein lokaler `map_*_error`-Override** in der Phase-9-REST-Datei — global reicht (Phase-8-Pattern).

### Audit-Macros (Hash-Chain)

**Source:** `genossi_service_impl/src/audit_macros.rs:5-80`.
**Apply to:** `mark_paid_out`-Cascade (3 Aufrufe: 1× `audited_create!` + 2× `audited_update!`).

```rust
crate::audited_create!(self, self.<dao>, &entity, PROCESS_CONST, &user_id, tx);
crate::audited_update!(self, self.<dao>, entity_id, &new_entity, PROCESS_CONST, &user_id, tx);
```

**Wichtig:** Alle drei Aufrufe verwenden denselben `PROCESS_CONST` = `REPAYMENT_ENTRY_PROCESS_MARK_PAID_OUT` (D-01 — Phase-8-D-03-Pragma). Jeder Macro-Aufruf erzeugt intern eine **neue** `transaction_id` (siehe `audit_log.rs:65`); Identifikation als „ein Geschäftsvorfall" geschieht über den gemeinsamen `process`-String + Hash-Chain-Sequenz.

### Re-Read-Pattern nach `audited_update!`

**Source:** `genossi_service_impl/src/repayment_entry.rs:265-291` (Phase-8 BL-01-Fix) + `genossi_service_impl/src/member.rs:343-348` (Member-Layer-Vorlage).
**Apply to:** Alle `audited_update!`-Aufrufe in `mark_paid_out` (Member-Re-Read nach Schritt 7; Entry-Re-Read nach Schritt 9).

```rust
let refreshed = self
    .<dao>
    .find_by_id(<id>, tx.clone())
    .await?
    .ok_or_else(|| {
        ServiceError::InternalError(Arc::from(format!(
            "Re-Read after audited_update! returned None for <Entity> {} — \
             internal consistency error (same-tx invariant violated)",
            <id>
        )))
    })?;
```

**Pitfall:** `None`-Branch MUSS `ServiceError::InternalError` zurückgeben, NICHT `ServiceError::EntityNotFound`. Konkrete Begründung steht in den Source-Comments von `repayment_entry.rs:272-276` und MUSS mitkopiert werden.

### Tx-Atomarität (Drop-Rollback)

**Source:** `genossi_dao_impl_sqlite/src/transaction.rs:47-57` (implizites Rollback bei Drop) + Phase-8-Cascade-Patterns.
**Apply to:** `mark_paid_out` — **ein einziger `commit` am Ende**, keine Zwischenschritte. Bei Fehler via `?` wird Tx gedropped, sqlx rollt automatisch zurück.

```rust
// ... 11 cascade-steps ...
self.transaction_dao.commit(tx).await?;
Ok(RepaymentEntry::from(&entry_refreshed))
```

---

## No Analog Found

Keine. Alle Phase-9-Patterns haben direkte Anker in Phase-7/8-Code oder bestehenden Service-Implementierungen.

---

## Deviation Notes (Plan-Discretion)

Diese Liste enthält Stellen, an denen der Plan bewusst vom nächstgelegenen Analog abweicht. Der Planner soll sie als Begründungs-Anker für seine PLAN.md-Action-Sections verwenden.

| # | Datei / Stelle | Analog-Pattern | Phase-9-Abweichung | Begründung |
|---|----------------|----------------|--------------------|------------|
| 1 | `RepaymentEntryServiceImpl::mark_paid_out` — Cascade-Owner | `MemberActionServiceImpl::create` ruft `MemberActionService` für Action-Erzeugung indirekt auf | Phase 9 ruft `audited_create!` direkt mit `member_action_dao` (kein Service-zu-Service) | D-08: Direct-DAO-Zugriff hält Tx-Atomarität deterministisch, vermeidet doppelte Permission-Checks (Phase-8-D-03-Pragma) |
| 2 | `mark_paid_out` — Audit-Process-String | `MEMBER_ACTION_SERVICE_PROCESS = "member-action-service"` (eigener String pro Service) | Alle 3 Cascade-Writes nutzen denselben String `REPAYMENT_ENTRY_PROCESS_MARK_PAID_OUT = "repayment-entry.mark-paid-out"` | D-01: SC #3 verlangt „identifizierbar als ein Geschäftsvorfall" — gemeinsamer Process-String + same-tx + Hash-Chain reichen |
| 3 | `mark_paid_out` — Phase-Load-NotFound-Mapping | `find_by_id`-`None` → `ServiceError::EntityNotFound` (üblich für User-NotFound) | Phase-`None` bei vorhandenem Entry → `ServiceError::InternalError` | RESEARCH Pitfall #5: Entry zeigt auf nicht-existente Phase → referentielle Inkonsistenz, kein User-NotFound |
| 4 | `mark_paid_out` — Re-Read-`None`-Mapping | `find_by_id`-`None` → `ServiceError::EntityNotFound` (allgemein) | Re-Read-`None` nach `audited_update!` → `ServiceError::InternalError` | Phase-8 BL-01-Pattern (`repayment_entry.rs:272-287`): same-Tx-Invariant garantiert Existenz, `None` = DAO-Regression → 500 statt 404 |
| 5 | `mark_paid_out` — Validation-Helper | `validate_entry_create` (`repayment_entry.rs:67-91`) prüft `>0 AND ≤current_shares` | Phase 9 macht Inline-Check (NUR `<=current_shares`, weil DB-CHECK `>0` bereits sicherstellt) | D-13: Reuse nicht 1:1 passend; CONTEXT erlaubt beide Optionen; Inline ist schlanker für nur einen Check |
| 6 | `mark_paid_out` — recalc_migrated | `MemberActionServiceImpl::recalc_migrated` (`member_action.rs:200-224`) als Inherent-Method | Phase 9 repliziert die Logik inline in `mark_paid_out` (~18 LOC); `compute_migration_status` über fully-qualified Path | D-10 Option (a): Trait-Methoden-Aufnahme zieht Service-zu-Service-Dep + Mock-Aufwand nach sich; Inline pragmatischer für Einmal-Verwendung |
| 7 | `compute_migration_status` — Visibility | `pub(crate)` (bewusste Modul-Scope-Kapselung) | `pub` (Cross-Crate-Zugriff aus `mark_paid_out`-Caller) | D-10 / RESEARCH Frage 2 Option (a): Funktion ist pure, kein Sicherheits-/Korrektheits-Risiko, kein Trait-Modeling-Argument |
| 8 | REST-Handler `mark_paid_out` — Status-Codes | `open_repayment_phase` doc-strings haben nur 401/404/409 | Phase-9-utoipa-Annotation hat zusätzlich 400 + 500 | PAYO-03 (Validation) + BL-01 (Re-Read-None) erfordern eindeutige Mappings; CONTEXT D-06 |
| 9 | E2E-Test-Suite — Anzahl | CONTEXT D-09 fordert 5 E2E-Tests | Plan reduziert auf 4 (Phase-Status-Guard wird in Unit-Test verschoben) | RESEARCH Pitfall #10: E2E-Setup für „Phase in Preparation + Entry existiert" ist nur über direkten DB-Insert möglich; Unit-Test (Mock-Phase) ist sauberer |
| 10 | E2E-Race-Test — Erwarteter Status-Code | `test_helper_token_redeem_race…` erwartet `[200, 410]` | Phase-9-Race erwartet `[200, 409]` | RESEARCH Frage 1: Race-Verlierer scheitert mit Version-Mismatch im DAO-UPDATE → `ConflictError` → 409 |

---

## Metadata

**Analog search scope:**
- `genossi_service/src/repayment_entry.rs` (Trait-Vorlagen)
- `genossi_service/src/repayment_phase.rs` (Action-Endpoint-Trait-Method-Vorlagen)
- `genossi_service_impl/src/repayment_entry.rs` (Cascade-Owner, Re-Read, Mock-Setup)
- `genossi_service_impl/src/member_action.rs` (audited_create + recalc_migrated, compute_migration_status)
- `genossi_service_impl/src/member.rs` (Re-Read-Original-Vorlage)
- `genossi_service_impl/src/audit_macros.rs` (alle 3 Macros)
- `genossi_rest/src/repayment_entry.rs` (Handler-Struktur, ApiDoc, generate_route)
- `genossi_rest/src/repayment_phase.rs` (Action-Endpoint-Handler-Vorlagen `open_repayment_phase`, `close_repayment_phase`)
- `genossi_bin/src/lib.rs` (Deps-Block + Konstruktor-Stelle)
- `genossi_bin/tests/e2e_tests.rs` (Race-Test-Pattern `tokio::join!`)
- `genossi_dao/src/member_action.rs` (MemberActionDao-Trait-Signatur)

**Files scanned:** 11 direkte Anker-Files; weitere via grep-trail bereits in RESEARCH §"References" dokumentiert.

**Pattern extraction date:** 2026-05-31

---

## PATTERN MAPPING COMPLETE

- **Phase:** 9 — Auszahlungs-Buchung (atomisch + auditiert)
- **Files classified:** 7
- **Analogs found:** 7 / 7 (100% Match-Quality „exact" für alle)

### Coverage

- Files mit exact analog: **7**
- Files mit role-match analog: **0**
- Files mit no analog: **0**

### Key Patterns Identified

- **Action-Endpoint ohne Body** (`/{id}/mark-paid-out` analog `/{id}/open`, `/{id}/close`) — Phase-7-D-02/D-03-Pattern, exakt reproduzierbar aus `genossi_rest/src/repayment_phase.rs:234-268`
- **Re-Read nach `audited_update!` mit `InternalError`-Fallback** — Phase-8-BL-01-Pattern aus `genossi_service_impl/src/repayment_entry.rs:265-291`, **Source-Comments mitkopieren**
- **Multi-Step-Cascade in einer SQLite-Tx mit gemeinsamem Process-String** — D-01-Pragma; 3 Macro-Aufrufe (`audited_create!` + 2× `audited_update!`), einziger `commit` am Ende, implizites Drop-Rollback bei `?`-Fehler
- **Direct-DAO-Zugriff für Cross-Entity-Cascades** (statt Service-zu-Service) — D-08, gleicher Pragma wie Phase-8-Auto-Fill in `open_phase`
- **mockall::Sequence für Re-Read-Tests** — Vorlage `test_update_entry_status_open_to_contacted_succeeds` (`repayment_entry.rs:1294-1360`); Phase-9-Cascade hat ~6 sequenzielle `find_by_id`-Calls
- **Race-Test via `tokio::join!`** mit Sort-Assertion `[200, 409]` — Anker `e2e_tests.rs:8783-8821` (Helper-Token-Race), Status-Code-Wechsel von 410 auf 409 (Version-Mismatch statt already-used)

### File Created

`/home/neosam/programming/rust/projects/genossi3/.planning/phases/09-auszahlungs-buchung-atomisch-auditiert/09-PATTERNS.md`

### Deviation Notes: **10**

Siehe Tabelle „Deviation Notes" — alle 10 Abweichungen sind explizit per Decision-ID (D-01, D-08, D-10, D-13) oder per Research-Pitfall-Referenz (Pitfall #5, #6, #10, RESEARCH Frage 1, 2) begründet und planner-actionable.

### Ready for Planning

Pattern mapping komplett. Planner kann jetzt PLAN-Action-Sections direkt mit konkreten file:line-Verweisen auf die hier extrahierten Code-Excerpts und Deviation Notes versehen. Alle 7 Phase-9-Files haben einen 1:1-Analog im bestehenden Workspace; **keine externen Vorlagen, keine spekulative Architektur, keine offenen Pattern-Fragen.**
