# Phase 14: DAO/Domain Foundation - Pattern Map

**Mapped:** 2026-06-04
**Files analyzed:** 9 (1 NEW + 8 MODIFIED)
**Analogs found:** 9 / 9 (100% — alle Files haben einen v1.1-Codebase-Vorbilder)

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `genossi_service_impl/src/membership_adjust.rs` (NEW) | service-utility (pure-function module) | transform | `genossi_service_impl/src/member_action.rs:155-177` (`compute_dates`) | exact (pure-function pattern) |
| `genossi_service_impl/src/lib.rs` (MODIFIED) | config (module registration) | — | `genossi_service_impl/src/lib.rs:1-29` (existing `pub mod` declarations) | exact |
| `genossi_dao/src/repayment_entry.rs` (MODIFIED) | DAO trait (interface) | CRUD (read-filter) | `genossi_dao/src/repayment_entry.rs:138-150` (`find_by_phase_id`) | exact (same trait, sibling method) |
| `genossi_dao_impl_sqlite/src/repayment_entry.rs` (MODIFIED) | DAO impl (SQLite override + tests) | CRUD (read-filter) | `genossi_dao_impl_sqlite/src/repayment_entry.rs:71-91` (`dump_all`) + Z. 388-417 (`test_find_by_phase_id_filters_correctly`) | exact (same file, sibling SQL + test) |
| `genossi_service/src/member.rs` (MODIFIED) | service trait | request-response | `genossi_service/src/member.rs:110-114` (`get_all` signature) | exact |
| `genossi_service_impl/src/member.rs` (MODIFIED) | service impl | request-response | `genossi_service_impl/src/member.rs:90-111` (`get_all`) | exact |
| `genossi_rest/src/member.rs` (MODIFIED) | REST controller | request-response | `genossi_rest/src/member.rs:53-74` (`get_all_members`) + `genossi_rest/src/repayment_entry.rs:59-141` (`Query<>` + `IntoParams`) | exact composite |
| `genossi_rest_types/src/lib.rs` (MODIFIED) | TO/DTO | transform | `genossi_rest_types/src/lib.rs:2197-2230` (`AttendanceMemberTO`) | exact (Slim-TO with PII-guard) |
| `genossi_bin/tests/transfer_recipients_e2e.rs` (NEW) | e2e test | request-response | `genossi_bin/tests/repayment_letter_e2e.rs:143-202` (`create_member_with_exit_date_and_iban`) | exact (3-step exit_date setup) |

## Pattern Assignments

### `genossi_service_impl/src/membership_adjust.rs` (NEW, pure-function module)

**Analog:** `genossi_service_impl/src/member_action.rs:155-177` (`compute_dates`)

**Pure-Function-Pattern** (Vorbild, member_action.rs:155-177):
```rust
pub(crate) fn compute_dates(
    member: &genossi_dao::member::MemberEntity,
    actions: &[genossi_dao::member_action::MemberActionEntity],
) -> (time::Date, Option<time::Date>) {
    let join_date = actions
        .iter()
        .find(|a| a.action_type == ActionType::Eintritt)
        .map(|a| a.date)
        .unwrap_or(member.join_date);
    // ...
    (join_date, exit_date)
}
```

**Delta für Phase 14:**
- Sichtbarkeit `pub(crate)` (D-14-03) — identisch zu Vorbild.
- Signatur: `pub(crate) fn compute_effective_date(willensbekundung: Date) -> EffectiveDate`.
- Return-Type **Struct** `EffectiveDate { fiscal_year: i32, effective_date: Date }` (D-14-01, NICHT Tuple wie compute_dates).
- Derive: `#[derive(Debug, Clone, Copy, PartialEq, Eq)]` auf `EffectiveDate`.
- Ausführlicher `///`-Doc-Kommentar mit H1/H2-Verbands-Konvention (D-14-07).
- Implementation: `if month <= 6 → fiscal_year = year, else year+1`, `effective_date = Date::from_calendar_date(fiscal_year, December, 31).expect(...)`.

**Test-Pattern** (analog `member_action.rs::tests`, Inline `#[cfg(test)] mod tests`):
```rust
#[cfg(test)]
mod tests {
    use super::*;
    // 6 Edge-Case-Tests:
    // test_compute_effective_date_30_juni_is_h1
    // test_compute_effective_date_01_juli_is_h2
    // test_compute_effective_date_31_dezember_is_h2_next_year
    // test_compute_effective_date_01_januar_is_h1
    // test_compute_effective_date_schaltjahr_29_februar_is_h1
    // test_compute_effective_date_mittiges_datum_15_maerz_is_h1
}
```

---

### `genossi_service_impl/src/lib.rs` (MODIFIED, module registration)

**Analog:** existing module declarations (lib.rs:1-29):
```rust
pub mod application;
pub mod assembly;
pub mod attendance;
// ...
pub mod member;
pub mod member_action;
pub mod member_document;
// ...
```

**Delta:** Eine neue Zeile in alphabetischer Sortierung:
```rust
pub mod membership_adjust;   // ← NEU, zwischen `member_import` und `pdf_generation`
```

Keine Re-Exports auf Crate-Ebene (D-14-03: `pub(crate)`).

---

### `genossi_dao/src/repayment_entry.rs` (MODIFIED, trait extension)

**Analog:** `genossi_dao/src/repayment_entry.rs:138-150` (`find_by_phase_id` Default-Impl):
```rust
async fn find_by_phase_id(
    &self,
    phase_id: Uuid,
    tx: Self::Transaction,
) -> Result<Arc<[RepaymentEntryEntity]>, DaoError> {
    let all_entities = self.dump_all(tx).await?;
    let filtered: Vec<RepaymentEntryEntity> = all_entities
        .iter()
        .filter(|e| e.phase_id == phase_id && e.deleted.is_none())
        .cloned()
        .collect();
    Ok(filtered.into())
}
```

**Delta für Phase 14** (D-14-08 + Open-Question 3 → MIT Default-Impl):
```rust
/// Liefert alle aktiven Eintraege einer Member-Phase-Kombination.
/// Foundation für Phase-16-Sum-Check + Auto-Fill-Skip-Pattern (PITFALLS Kat 1).
async fn find_by_member_and_phase(
    &self,
    member_id: Uuid,
    phase_id: Uuid,
    tx: Self::Transaction,
) -> Result<Arc<[RepaymentEntryEntity]>, DaoError> {
    let all_entities = self.dump_all(tx).await?;
    let filtered: Vec<RepaymentEntryEntity> = all_entities
        .iter()
        .filter(|e| e.member_id == member_id
                 && e.phase_id == phase_id
                 && e.deleted.is_none())
        .cloned()
        .collect();
    Ok(filtered.into())
}
```

**Mockall-Override-Falle (Pitfall 2 aus RESEARCH.md):** `#[automock(type Transaction = crate::MockTransaction;)]` (Z. 89) generiert MockRepaymentEntryDao — Default-Impl wird vom Mock IGNORIERT. Service-Unit-Tests MÜSSEN `dao.expect_find_by_member_and_phase().returning(...)` explizit setzen.

**Test (im selben Trait-Modul, im `#[cfg(test)] mod tests`-Block):**
Default-Impl-Test mit `MockRepaymentEntryDao`-Setup + MockTransaction. Vorbild: existierende Tests Z. 153-291 für Status-Roundtrip und Auditable-Diff.

---

### `genossi_dao_impl_sqlite/src/repayment_entry.rs` (MODIFIED, SQL override + tests)

**Analog 1 — SQL-Pattern:** `dump_all` (Z. 71-91):
```rust
async fn dump_all(
    &self,
    tx: Self::Transaction,
) -> Result<Arc<[RepaymentEntryEntity]>, DaoError> {
    let rows = sqlx::query_as::<_, RepaymentEntryDb>(
        "SELECT id, member_id, phase_id, share_count_to_pay_out, status, created, \
         deleted, version FROM repayment_entry \
         ORDER BY created ASC, id ASC",
    )
    .fetch_all(tx.tx.lock().await.as_mut())
    .await
    .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?;

    rows.iter()
        .map(RepaymentEntryEntity::try_from)
        .collect::<Result<Vec<_>, _>>()
        .map(|v| v.into())
}
```

**Delta für Phase 14:**
```rust
async fn find_by_member_and_phase(
    &self,
    member_id: Uuid,
    phase_id: Uuid,
    tx: Self::Transaction,
) -> Result<Arc<[RepaymentEntryEntity]>, DaoError> {
    let member_blob = member_id.as_bytes().to_vec();
    let phase_blob = phase_id.as_bytes().to_vec();
    let rows = sqlx::query_as::<_, RepaymentEntryDb>(
        "SELECT id, member_id, phase_id, share_count_to_pay_out, status, created, \
         deleted, version FROM repayment_entry \
         WHERE member_id = ? AND phase_id = ? AND deleted IS NULL \
         ORDER BY created ASC, id ASC",
    )
    .bind(member_blob)
    .bind(phase_blob)
    .fetch_all(tx.tx.lock().await.as_mut())
    .await
    .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?;

    rows.iter()
        .map(RepaymentEntryEntity::try_from)
        .collect::<Result<Vec<_>, _>>()
        .map(|v| v.into())
}
```

**Analog 2 — Test-Pattern:** `test_find_by_phase_id_filters_correctly` (Z. 388-417):
```rust
#[tokio::test]
async fn test_find_by_phase_id_filters_correctly() {
    let pool = setup_db().await;
    let dao = RepaymentEntryDaoImpl::new(pool.clone());
    let tx_dao = TransactionDaoImpl::new(pool);

    let phase_a = Uuid::new_v4();
    let phase_b = Uuid::new_v4();

    let mut e1 = sample_entity();
    e1.phase_id = phase_a;
    let mut e2 = sample_entity();
    e2.phase_id = phase_a;
    let mut e3 = sample_entity();
    e3.phase_id = phase_b;

    let tx = tx_dao.transaction().await.unwrap();
    dao.create(&e1, "test", tx.clone()).await.unwrap();
    dao.create(&e2, "test", tx.clone()).await.unwrap();
    dao.create(&e3, "test", tx.clone()).await.unwrap();

    let found_a = dao.find_by_phase_id(phase_a, tx.clone()).await.unwrap();
    assert_eq!(found_a.len(), 2, "phase_a should have exactly 2 entries");
    // ...
}
```

**Delta für Phase 14 — 2 neue Tests:**
- `test_find_by_member_and_phase_returns_empty_when_no_match` — leere Liste für (member_id, phase_id) ohne Entries.
- `test_find_by_member_and_phase_filters_correctly` — 3+ Entries mit verschiedenen (member, phase)-Kombinationen; nur die zu (member_X, phase_X) gehörigen kommen zurück. Inklusive ausgefilterte andere Phase (analog `phase_b`-Pattern) UND ausgefilterten anderen Member.

`sample_entity()`-Helfer (Z. 221-234) und `setup_db()` (Z. 197-219) wiederverwenden — keine Duplikation.

---

### `genossi_service/src/member.rs` (MODIFIED, service trait extension)

**Analog:** `MemberService::get_all` Trait-Signatur (Z. 110-114):
```rust
async fn get_all(
    &self,
    context: Authentication<Self::Context>,
    tx: Option<Self::Transaction>,
) -> Result<Arc<[Member]>, ServiceError>;
```

**Delta für Phase 14:**
```rust
async fn list_transfer_recipients(
    &self,
    exclude_member_id: Uuid,
    context: Authentication<Self::Context>,
    tx: Option<Self::Transaction>,
) -> Result<Arc<[Member]>, ServiceError>;
```

**Mockall-Override-Falle:** `#[automock(type Context=(); type Transaction = genossi_dao::MockTransaction;)]` (Z. 104) generiert `MockMemberService` automatisch. Service-Tests, die `MockMemberService` einsetzen, MÜSSEN die neue Methode auch mit `.expect_list_transfer_recipients()` setzen. Aber: REST-Tests (E2E) gehen über echten `MemberServiceImpl` — keine Auswirkung dort.

---

### `genossi_service_impl/src/member.rs` (MODIFIED, service impl)

**Analog:** `MemberServiceImpl::get_all` (Z. 90-111):
```rust
async fn get_all(
    &self,
    context: Authentication<Self::Context>,
    tx: Option<Self::Transaction>,
) -> Result<Arc<[Member]>, ServiceError> {
    let tx = self.transaction_dao.use_transaction(tx).await?;

    self.permission_service
        .check_permission(VIEW_MEMBERS_PRIVILEGE, context)
        .await?;

    let members = self
        .member_dao
        .all(tx.clone())
        .await?
        .iter()
        .map(Member::from)
        .collect();

    self.transaction_dao.commit(tx).await?;
    Ok(members)
}
```

**Delta für Phase 14:**
```rust
async fn list_transfer_recipients(
    &self,
    exclude_member_id: Uuid,
    context: Authentication<Self::Context>,
    tx: Option<Self::Transaction>,
) -> Result<Arc<[Member]>, ServiceError> {
    let tx = self.transaction_dao.use_transaction(tx).await?;

    // ADMIN_PRIVILEGE statt VIEW_MEMBERS_PRIVILEGE (D-14-11)
    self.permission_service
        .check_permission(ADMIN_PRIVILEGE, context)
        .await?;

    let members: Arc<[Member]> = self
        .member_dao
        .all(tx.clone())                     // Default-Impl filtert deleted IS NULL
        .await?
        .iter()
        .filter(|e| e.exit_date.is_none() && e.id != exclude_member_id)
        .map(Member::from)
        .collect();

    self.transaction_dao.commit(tx).await?;
    Ok(members)
}
```

**Import-Pattern für ADMIN_PRIVILEGE** (Open-Question 1 → Empfehlung Import):
```rust
use genossi_service::permission::ADMIN_PRIVILEGE;  // ← NEU, oben bei den Imports
```
Alternative: Lokale Re-Deklaration `const ADMIN_PRIVILEGE: &str = "admin";` analog `repayment_phase.rs:50`. Planner entscheidet — Import ist sauberer (kanonische Quelle).

**Test-Pattern (Service-Layer):** 3 Unit-Tests mit `MockMemberDao` + `MockPermissionService` (mockall). Vorbild: bestehende Tests in `genossi_service_impl/src/member.rs` (Z. 200+). 3 Test-Cases (D-14-14):
1. Happy: 3 aktive Members, exclude_self = member_A → 2 zurück.
2. Alle gekündigt (`exit_date = Some(...)`) → leere Liste.
3. Nur Self (1 Member, alle anderen exit_date) → leere Liste.

---

### `genossi_rest/src/member.rs` (MODIFIED, REST handler + router)

**Analog 1 — Router-Sub-Route-Ordering** (Z. 28-40):
```rust
pub fn generate_route<RestState: RestStateDef>() -> Router<RestState> {
    Router::new()
        .route("/", get(get_all_members::<RestState>))
        .route("/{id}", get(get_member::<RestState>))            // ← /{id} liegt VOR /import
        .route("/", post(create_member::<RestState>))
        .route("/{id}", put(update_member::<RestState>))
        .route("/{id}", delete(delete_member::<RestState>))
        .route("/import", post(import_members::<RestState>))
        .route(
            "/not-reached-by/{job_id}",
            get(get_members_not_reached_by::<RestState>),
        )
}
```

**KRITISCH (Pitfall 1 aus RESEARCH.md):** `/transfer-recipients` MUSS **VOR** `/{id}` deklariert werden, sonst frisst Axum's UUID-Parser den String "transfer-recipients". Die bestehende Reihenfolge in `member.rs:28-40` ist suboptimal (`/import` ist nach `/{id}` und funktioniert nur weil POST/GET kollidieren) — aber für GET-Sub-Routes ist die VOR-Position zwingend. Vorbild korrekter Reihenfolge: STATE.md Plan-08-05-Entry mit `/batch-status`-Pattern.

**Delta für Phase 14 Router:**
```rust
pub fn generate_route<RestState: RestStateDef>() -> Router<RestState> {
    Router::new()
        .route("/", get(get_all_members::<RestState>))
        .route("/transfer-recipients", get(get_transfer_recipients::<RestState>))  // ← NEU, VOR /{id}
        .route("/{id}", get(get_member::<RestState>))
        .route("/", post(create_member::<RestState>))
        .route("/{id}", put(update_member::<RestState>))
        .route("/{id}", delete(delete_member::<RestState>))
        .route("/import", post(import_members::<RestState>))
        .route(
            "/not-reached-by/{job_id}",
            get(get_members_not_reached_by::<RestState>),
        )
}
```

**Analog 2 — Handler-Pattern (get_all_members, Z. 42-74):**
```rust
#[instrument(skip(rest_state))]
#[utoipa::path(
    get,
    tag = "Members",
    path = "",
    responses(
        (status = 200, description = "Get all members", body = [MemberTO]),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn get_all_members<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
) -> Response {
    error_handler(
        (async {
            let members: Arc<[MemberTO]> = rest_state
                .member_service()
                .get_all(crate::extract_auth_context(Some(context))?, None)
                .await?
                .iter()
                .map(MemberTO::from)
                .collect();
            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&members)?))
                .unwrap())
        })
        .await,
    )
}
```

**Analog 3 — Query<>-Pattern + IntoParams (`genossi_rest/src/repayment_entry.rs:59-141`):**
```rust
#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct ListEntriesQuery {
    pub phase_id: Uuid,
}

pub async fn list_repayment_entries<RestState: RestStateDef + RepaymentEntryRestState>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Query(q): Query<ListEntriesQuery>,
) -> Response {
    error_handler(
        (async {
            let auth = crate::extract_auth_context(Some(context))?;
            let entries = rest_state
                .repayment_entry_service()
                .list_repayment_entries_by_phase(q.phase_id, auth)
                .await?;
            // ...
        })
        .await,
    )
}
```

**Delta — Phase 14 Handler:**
```rust
use axum::extract::Query;
use serde::Deserialize;
use utoipa::IntoParams;
use genossi_rest_types::MemberSlimTO;

#[derive(Debug, Deserialize, IntoParams)]
pub struct TransferRecipientsQuery {
    /// UUID des aktuellen Mitglieds — wird aus der Ergebnis-Liste ausgefiltert.
    pub exclude_self: Uuid,
}

#[instrument(skip(rest_state))]
#[utoipa::path(
    get,
    tag = "Members",
    path = "/transfer-recipients",
    params(TransferRecipientsQuery),
    responses(
        (status = 200, description = "Aktive Transfer-Empfänger (ohne self)", body = [MemberSlimTO]),
        (status = 400, description = "Invalid exclude_self UUID format"),
        (status = 401, description = "Unauthorized — kein Login oder keine admin-Rolle"),  // 401, NICHT 403 (Pitfall 4)
        (status = 500, description = "Internal server error"),
    ),
)]
pub async fn get_transfer_recipients<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Query(query): Query<TransferRecipientsQuery>,
) -> Response {
    error_handler(
        (async {
            let members: Vec<MemberSlimTO> = rest_state
                .member_service()
                .list_transfer_recipients(
                    query.exclude_self,
                    crate::extract_auth_context(Some(context))?,
                    None,
                )
                .await?
                .iter()
                .map(MemberSlimTO::from)
                .collect();
            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&members)?))
                .unwrap())
        })
        .await,
    )
}
```

**Analog 4 — OpenAPI-Registrierung (Z. 331-345):**
```rust
#[derive(OpenApi)]
#[openapi(
    paths(
        get_all_members, get_member, create_member, update_member, delete_member,
        import_members, get_members_not_reached_by
    ),
    components(schemas(MemberTO, genossi_rest_types::SalutationTO, /* ... */)),
    tags((name = "Members", description = "Member management endpoints"))
)]
pub struct ApiDoc;
```

**Delta:**
- `paths(...)` um `get_transfer_recipients` ergänzen.
- `components(schemas(...))` um `genossi_rest_types::MemberSlimTO` ergänzen.

---

### `genossi_rest_types/src/lib.rs` (MODIFIED, neuer Slim-TO)

**Analog:** `AttendanceMemberTO` (Z. 2197-2230):
```rust
/// **VERBOTEN:** Inserting an `impl From<&MemberTO> for AttendanceMemberTO`
/// would silently propagate new MemberTO fields (e.g. future `iban` /
/// `email` / `bank_account`) and violate ATTN-01. Conversion runs
/// EXCLUSIVELY through `From<&genossi_dao::attendance::AttendanceMemberRow>`
/// -- an explicit 7-field DTO from the DAO layer with the same whitelist.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct AttendanceMemberTO {
    pub member_number: i64,
    pub first_name: String,
    pub last_name: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub salutation: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub title: Option<String>,
    pub is_present: bool,
    pub member_id: Uuid,
}

impl From<&genossi_dao::attendance::AttendanceMemberRow> for AttendanceMemberTO {
    fn from(r: &genossi_dao::attendance::AttendanceMemberRow) -> Self { /* ... */ }
}
```

**Delta für Phase 14 (D-14-12):**
```rust
/// Reduzierte Darstellung eines Mitglieds für Empfänger-Search (TRSF-06).
///
/// **PII-Leak-Guard (Pattern aus AttendanceMemberTO):** Diese Struct hat
/// EXAKT 6 Felder. KEIN `impl From<&MemberTO> for MemberSlimTO` — sonst würden
/// neue MemberTO-Felder (email, bank_account, street) durchrutschen. Konversion
/// EXKLUSIV via `From<&genossi_service::member::Member>` aus dem Service-Layer.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct MemberSlimTO {
    pub id: Uuid,
    pub member_number: i64,   // i64 NICHT i32 — verifiziert in genossi_service::member::Member::member_number (Z. 14)
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub salutation: Option<SalutationTO>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub title: Option<String>,
    pub first_name: String,
    pub last_name: String,
}

impl From<&genossi_service::member::Member> for MemberSlimTO {
    fn from(m: &genossi_service::member::Member) -> Self {
        Self {
            id: m.id,
            member_number: m.member_number,
            salutation: m.salutation.as_ref().map(SalutationTO::from),
            title: m.title.as_deref().map(String::from),
            first_name: m.first_name.to_string(),
            last_name: m.last_name.to_string(),
        }
    }
}
```

**Field-Order-Konvention (Specifics-Block):** Frontend-Display-Reihenfolge (Mitgliedsnummer → Anrede → Titel → Vorname → Nachname); Feld-Reihenfolge im Struct spiegelt das.

**KEINE sensiblen Felder:** kein `email`, `bank_account`, `street`, `current_shares`, `current_balance`.

---

### `genossi_bin/tests/transfer_recipients_e2e.rs` (NEW, E2E test)

**Analog:** `repayment_letter_e2e.rs:143-202` (`create_member_with_exit_date_and_iban`) — **kritischer 3-Schritt-Setup für exit_date** (Pitfall 3):

```rust
async fn create_member_with_exit_date_and_iban(
    client: &reqwest::Client,
    server: &TestServer,
    member_number: i64,
    fiscal_year: i32,
    iban: Option<&str>,
) -> MemberTO {
    // 1) Member anlegen.
    let m = sample_member_with_iban(member_number, iban);
    let response = client.post(server.url("/api/members")).json(&m).send().await
        .expect("create_member POST failed");
    let created: MemberTO = response.json().await.expect("decode MemberTO");
    let member_id = created.id.expect("created member must have id");

    // 2) Austritt-Action posten — setzt exit_date.
    let exit_date = time::Date::from_calendar_date(fiscal_year, time::Month::June, 15).unwrap();
    let austritt = MemberActionTO {
        id: None, member_id,
        action_type: ActionTypeTO::Austritt,
        date: exit_date,
        shares_change: 0,
        transfer_member_id: None,
        effective_date: Some(exit_date),
        comment: Some("Phase 13 E2E setup".to_string()),
        created: None, deleted: None, version: None,
    };
    client.post(server.url(&format!("/api/members/{}/actions", member_id)))
        .json(&austritt).send().await.expect("POST Austritt action failed");

    // 3) Member frisch laden (recalc_dates hat exit_date gesetzt).
    let response = client.get(server.url(&format!("/api/members/{}", member_id)))
        .send().await.expect("GET member failed");
    response.json().await.expect("decode MemberTO")
}
```

**Delta für Phase 14:**
- Neue Datei `genossi_bin/tests/transfer_recipients_e2e.rs`.
- Zwei Helfer-Funktionen: `create_active_member` (1-step: POST `/api/members`), `create_cancelled_member` (3-step: aus Analog kopiert).
- Ein Test `test_transfer_recipients_filters_self_and_cancelled`:
  - 3 Members anlegen: m_active, m_cancelled, m_self (alle aktiv) → m_cancelled bekommt Austritt-Action.
  - GET `/api/members/transfer-recipients?exclude_self={m_self.id}` → erwarten: nur `[m_active]`.
  - Assertions: `recipients.len() == 1`, `recipients[0].id == m_active.id`.

**Auth-Setup:** v1.1-Pattern via `start_test_server(rest_state)` — Workspace-default ohne `--features oidc` aktiviert mock_auth mit Vorstands-Privileg.

**Status-Assertion:** `assert_eq!(resp.status(), StatusCode::OK);` — KEIN Test für 401 nötig (mock_auth ist immer admin).

---

## Shared Patterns

### Permission-Funnel-Reihenfolge

**Source:** `repayment_phase.rs:99-108` (kanonisches Pattern)
**Apply to:** `genossi_service_impl/src/member.rs::list_transfer_recipients`

```rust
let tx = self.transaction_dao.use_transaction(tx).await?;

self.permission_service
    .check_permission(ADMIN_PRIVILEGE, context)
    .await?;

// ... DAO-Calls + Filter ...

self.transaction_dao.commit(tx).await?;
```

**Reihenfolge zwingend:** `use_transaction` → `check_permission` → DAO-Calls → `commit`. Falls Permission fehlt, wird die Transaction nicht commited (implicit rollback via Drop).

### Error-Handling-Wrapper

**Source:** `genossi_rest/src/lib.rs:107-117` (globales `From<ServiceError> for RestError`)
**Apply to:** `genossi_rest/src/member.rs::get_transfer_recipients`

Kein lokaler Override nötig. Mapping:
- `ServiceError::PermissionDenied` → `RestError::Unauthorized` → **HTTP 401** (NICHT 403, Pitfall 4)
- `ServiceError::DataAccess` → `RestError::InternalError` → HTTP 500
- `ServiceError::EntityNotFound` → `RestError::NotFound` → HTTP 404

Utoipa-Annotation MUSS 401 listen (nicht 403), siehe Pitfall 4 in RESEARCH.md.

### Soft-Delete-Filter

**Source:** `genossi_dao/src/repayment_entry.rs:138-150` (`find_by_phase_id` Default-Impl) + Member-`all`-Default-Impl
**Apply to:**
- `find_by_member_and_phase`-Default-Impl: `e.deleted.is_none()` im Filter.
- SQL-Override: `AND deleted IS NULL` in WHERE-Klausel.
- `list_transfer_recipients`: nutzt `member_dao.all(tx)` → bereits gefiltert via Default-Impl der `all`-Methode.

### Arc<[T]>-Return-Type

**Source:** Etabliert in DAO und Service-Layer (Konvention)
**Apply to:**
- DAO: `Arc<[RepaymentEntryEntity]>` (D-14-09).
- Service: `Arc<[Member]>` (D-14-13).
- REST: konvertiert zu `Vec<MemberSlimTO>` für JSON-Serialisierung (klare Layer-Trennung).

### Mockall-Override-Falle (DEFENSIVE)

**Source:** `repayment_phase.rs:976-989` (dokumentiertes Pattern)
**Apply to:** Alle neuen Service- und DAO-Trait-Methoden in Phase 14.

`#[automock]` und `mock!` IGNORIEREN Trait-Default-Impl. Service-Unit-Tests, die `MockRepaymentEntryDao` einsetzen, MÜSSEN `.expect_find_by_member_and_phase()` setzen — auch wenn die Trait-Default-Impl via `dump_all` funktionieren würde.

## No Analog Found

Keine. Alle 9 Files haben einen direkten v1.1-Codebase-Vorbild. Phase 14 ist **rein erweiternde** Foundation; keine neuen Patterns nötig.

## Metadata

**Analog search scope:**
- `genossi_dao/src/` (DAO-Trait-Patterns)
- `genossi_dao_impl_sqlite/src/` (SQLite-Override + In-Memory-Test-Patterns)
- `genossi_service/src/` (Service-Trait-Signaturen)
- `genossi_service_impl/src/` (Service-Impl mit Permission-Funnel + Pure-Function-Vorbild)
- `genossi_rest/src/` (REST-Handler + Router + Utoipa)
- `genossi_rest_types/src/` (TO-Konvertierung + Slim-TO-PII-Guard)
- `genossi_bin/tests/` (E2E-Test-Setup + 3-Schritt-exit_date-Pattern)

**Files scanned (verified excerpts extracted from):**
- `genossi_service_impl/src/member_action.rs` (Z. 140-200)
- `genossi_service_impl/src/member.rs` (Z. 1-120)
- `genossi_service_impl/src/lib.rs` (Z. 1-33)
- `genossi_service_impl/src/repayment_phase.rs` (Z. 95-120)
- `genossi_service/src/member.rs` (Z. 1-208)
- `genossi_service/src/permission.rs` (Z. 20-35)
- `genossi_dao/src/repayment_entry.rs` (Z. 1-292)
- `genossi_dao_impl_sqlite/src/repayment_entry.rs` (Z. 1-418)
- `genossi_rest/src/member.rs` (Z. 1-120, 320-346)
- `genossi_rest/src/repayment_entry.rs` (Z. 1-145)
- `genossi_rest_types/src/lib.rs` (Z. 108-260, 2180-2260)
- `genossi_bin/tests/repayment_letter_e2e.rs` (Z. 100-210)

**Pattern extraction date:** 2026-06-04
