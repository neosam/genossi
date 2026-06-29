# Phase 15: Service+REST: Kündigung + Aufstockung — Pattern Map

**Mapped:** 2026-06-04
**Files in Scope:** 7 (5 neu/erweitert, 2 reine Erweiterungen)
**Analogs:** 7/7 (vollständige Abdeckung, alle Patterns existieren in v1.1/Phase 14 bereits)

---

## Datei-Klassifikation

| Datei | Rolle | Data-Flow | Closest Analog | Match-Qualität |
|-------|-------|-----------|----------------|----------------|
| `genossi_service/src/membership_adjust.rs` (NEU) | Service-Trait | request-response | `genossi_service/src/member_action.rs` | exakt |
| `genossi_service/src/lib.rs` (1 Zeile) | Modul-Registrierung | n/a | `genossi_service/src/lib.rs:12 (member_action)` | exakt |
| `genossi_service_impl/src/membership_adjust.rs` (ERWEITERN) | Service-Impl + Pure-Fn | CRUD/transactional | `genossi_service_impl/src/member_action.rs:289-356` | exakt |
| `genossi_service_impl/src/member_action.rs:180-203` (REFACTOR) | Free-Function-Refactor | helper | `compute_dates` (Z.155-177) | exakt |
| `genossi_rest/src/member.rs` (ERWEITERN) oder NEU `membership_adjust.rs` | REST-Handler | request-response | `genossi_rest/src/member.rs:117-143` | exakt |
| `genossi_rest_types/src/lib.rs` (NEU DTOs) | Request/Response DTO | n/a | `MemberSlimTO`-Block (Z.348-376), `MemberActionTO` (Z.420-461) | exakt |
| `genossi_bin/src/lib.rs` (DI-Wiring) | DI-Wiring + RestStateDef | n/a | `MemberActionServiceDependencies` (Z.461-478) + RestStateImpl-Slot (Z.577, Z.682-691, Z.1064, Z.1756, Z.1780-1782) | exakt |
| `genossi_bin/tests/cancel_membership_e2e.rs` (NEU) | E2E-Test | HTTP request-response | `genossi_bin/tests/transfer_recipients_e2e.rs:1-265` | exakt |

---

## Pattern-Zuweisungen

### Pattern 1: Service-Trait-Datei `genossi_service/src/membership_adjust.rs` (NEU)

**Rolle:** Service-Trait (async_trait + automock + Context/Transaction associated types)
**Analog:** `genossi_service/src/member_action.rs:1-134`
**Verbatim kopieren:** Imports, `#[automock]`-Annotation, `#[async_trait]`-Trait-Definition mit Context/Transaction-associated-Types, Method-Signaturen-Stil (`Authentication<Self::Context>`, `Option<Self::Transaction>`).
**Adaptieren:** Methoden auf nur `cancel_membership` + `increase_shares` reduzieren (D-15-13 inkrementelles Wachsen), Doc-Comments auf Deutsch (D-15-09 Konvention), Re-Use des bestehenden `MemberAction`/`Member`-Domain-Typs (kein neuer Request-Domain-Typ).

**Imports verbatim (Z.1-9):**
```rust
use async_trait::async_trait;
use genossi_dao::member_action::{ActionType, MemberActionEntity};
use mockall::automock;
use std::fmt::Debug;
use std::sync::Arc;
use uuid::Uuid;

use crate::permission::Authentication;
use crate::ServiceError;
```

**Trait-Skeleton verbatim (Z.80-105):**
```rust
#[automock(type Context=(); type Transaction = genossi_dao::MockTransaction;)]
#[async_trait]
pub trait MemberActionService {
    type Context: Clone + Debug + PartialEq + Eq + Send + Sync + 'static;
    type Transaction: genossi_dao::Transaction;

    async fn get_by_member(
        &self,
        member_id: Uuid,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<Arc<[MemberAction]>, ServiceError>;
    // ...
}
```

**Adaption für Phase 15 (D-15-15):**
```rust
#[automock(type Context=(); type Transaction = genossi_dao::MockTransaction;)]
#[async_trait]
pub trait MembershipAdjustService {
    type Context: Clone + Debug + PartialEq + Eq + Send + Sync + 'static;
    type Transaction: genossi_dao::Transaction;

    async fn cancel_membership(
        &self,
        member_id: Uuid,
        willensbekundung_date: time::Date,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<(genossi_service::member_action::MemberAction, genossi_service::member::Member), ServiceError>;

    async fn increase_shares(
        &self,
        member_id: Uuid,
        shares: i32,
        willensbekundung_date: time::Date,
        context: Authentication<Self::Context>,
        tx: Option<Self::Transaction>,
    ) -> Result<(genossi_service::member_action::MemberAction, genossi_service::member::Member), ServiceError>;
}
```

**Hinweis:** `MemberAction`-Domain-Typ aus `genossi_service::member_action` re-exportieren oder Pfad qualifizieren. Im `genossi_service_impl/src/membership_adjust.rs` kann es per `use genossi_service::member_action::MemberAction;` importiert werden.

---

### Pattern 2: Modul-Registrierung `genossi_service/src/lib.rs`

**Rolle:** Modul-Index
**Analog:** `genossi_service/src/lib.rs:12` (`pub mod member_action;`)
**Verbatim kopieren:** Alphabetische Sortierung der `pub mod`-Statements.
**Adaptieren:** `pub mod membership_adjust;` zwischen `member_import` (Z.14) und `permission` (Z.15) einfügen.

**Aktueller Zustand (Z.11-15):**
```rust
pub mod member;
pub mod member_action;
pub mod member_document;
pub mod member_import;
pub mod permission;
```

**Nach Phase 15 (1 Zeile hinzufügen):**
```rust
pub mod member;
pub mod member_action;
pub mod member_document;
pub mod member_import;
pub mod membership_adjust;     // NEU Phase 15
pub mod permission;
```

---

### Pattern 3: Service-Impl `genossi_service_impl/src/membership_adjust.rs` (ERWEITERN)

**Rolle:** Service-Impl (CRUD + audit + recalc_dates-Hook)
**Analog für Service-Method-Sequence:** `genossi_service_impl/src/member_action.rs:289-356` (`create`)
**Analog für `gen_service_impl!`:** `genossi_service_impl/src/member_action.rs:21-30`
**Analog für Pure-Function-Validation:** `genossi_service_impl/src/member_action.rs:76-153` (`validate_action`)
**Verbatim kopieren:** Transaction-Lifecycle (`use_transaction` → `commit`), Permission-Funnel, `audited_create!`-Macro-Call, `recalc_dates`-Hook, Member-Existence-Check.
**Adaptieren:** `MANAGE_MEMBERS_PRIVILEGE` → `ADMIN_PRIVILEGE` (D-15-01, PERM-01); `MEMBER_ACTION_SERVICE_PROCESS` → `"member-adjust.cancel"` / `"member-adjust.upgrade"` (D-15-02); kein `recalc_migrated` für `increase_shares` (Planner-Check, siehe CONTEXT-Reusable-Assets-Hinweis); `transfer_member_id = None` (UPGD-02); zusätzlich `audited_update!(member_dao, ...)` für `current_shares`-Mutation in `increase_shares` (D-15-03).

#### 3a) `gen_service_impl!` Pattern (verbatim mit Adaption)

**Verbatim aus `member_action.rs:21-30`:**
```rust
gen_service_impl! {
    struct MemberActionServiceImpl: MemberActionService = MemberActionServiceDeps {
        MemberActionDao: MemberActionDao<Transaction = Self::Transaction> = member_action_dao,
        MemberDao: MemberDao<Transaction = Self::Transaction> = member_dao,
        AuditLogDao: AuditLogDao<Transaction = Self::Transaction> = audit_log_dao,
        PermissionService: PermissionService<Context = Self::Context> = permission_service,
        UuidService: UuidService = uuid_service,
        TransactionDao: TransactionDao<Transaction = Self::Transaction> = transaction_dao,
    }
}
```

**Adaption für Phase 15 (identisches Deps-Set):**
```rust
gen_service_impl! {
    struct MembershipAdjustServiceImpl: MembershipAdjustService = MembershipAdjustServiceDeps {
        MemberActionDao: MemberActionDao<Transaction = Self::Transaction> = member_action_dao,
        MemberDao: MemberDao<Transaction = Self::Transaction> = member_dao,
        AuditLogDao: AuditLogDao<Transaction = Self::Transaction> = audit_log_dao,
        PermissionService: PermissionService<Context = Self::Context> = permission_service,
        UuidService: UuidService = uuid_service,
        TransactionDao: TransactionDao<Transaction = Self::Transaction> = transaction_dao,
    }
}
```

#### 3b) Process-Konstanten (D-15-02)

**Analog Z.17-19 in `member_action.rs`:**
```rust
const MEMBER_ACTION_SERVICE_PROCESS: &str = "member-action-service";
const VIEW_MEMBERS_PRIVILEGE: &str = "view_members";
const MANAGE_MEMBERS_PRIVILEGE: &str = "manage_members";
```

**Phase-15-Adaption (D-15-01, D-15-02):**
```rust
const CANCEL_PROCESS: &str = "member-adjust.cancel";
const UPGRADE_PROCESS: &str = "member-adjust.upgrade";
// ADMIN_PRIVILEGE bereits in genossi_service::permission::ADMIN_PRIVILEGE (Z.28) verfügbar
```

#### 3c) Full-Service-Method-Sequence verbatim (`MemberActionService::create` Z.289-356)

```rust
async fn create(
    &self,
    item: &MemberAction,
    context: Authentication<Self::Context>,
    tx: Option<Self::Transaction>,
) -> Result<MemberAction, ServiceError> {
    let tx = self.transaction_dao.use_transaction(tx).await?;

    let user_id = self
        .permission_service
        .current_user_id(context.clone())
        .await?
        .unwrap_or_else(|| "SYSTEM".to_string());

    self.permission_service
        .check_permission(MANAGE_MEMBERS_PRIVILEGE, context)
        .await?;

    let validation_errors = validate_action(item);
    if !validation_errors.is_empty() {
        return Err(ServiceError::ValidationError(validation_errors));
    }

    // Verify member exists
    self.member_dao
        .find_by_id(item.member_id, tx.clone())
        .await?
        .ok_or(ServiceError::EntityNotFound(item.member_id))?;

    // Verify transfer member exists if set
    if let Some(transfer_id) = item.transfer_member_id {
        self.member_dao
            .find_by_id(transfer_id, tx.clone())
            .await?
            .ok_or(ServiceError::EntityNotFound(transfer_id))?;
    }

    let now = time::OffsetDateTime::now_utc();
    let new_action = MemberAction {
        id: self.uuid_service.new_v4().await,
        member_id: item.member_id,
        action_type: item.action_type.clone(),
        date: item.date,
        shares_change: item.shares_change,
        transfer_member_id: item.transfer_member_id,
        effective_date: item.effective_date,
        comment: item.comment.clone(),
        created: time::PrimitiveDateTime::new(now.date(), now.time()),
        deleted: None,
        version: self.uuid_service.new_v4().await,
    };

    let action_entity: genossi_dao::member_action::MemberActionEntity = (&new_action).into();
    crate::audited_create!(
        self,
        self.member_action_dao,
        &action_entity,
        MEMBER_ACTION_SERVICE_PROCESS,
        &user_id,
        tx
    );

    self.recalc_dates(new_action.member_id, tx.clone()).await?;
    self.recalc_migrated(new_action.member_id, tx.clone()).await?;

    self.transaction_dao.commit(tx).await?;
    Ok(new_action)
}
```

**Adaption für `cancel_membership` (CANC-01..05, D-15-01..08, D-15-15):**
- `MANAGE_MEMBERS_PRIVILEGE` → `genossi_service::permission::ADMIN_PRIVILEGE`
- `MEMBER_ACTION_SERVICE_PROCESS` → `CANCEL_PROCESS`
- Statt `validate_action(item)` → `validate_willensbekundung_date(willensbekundung_date, today)` (Pure-Function, D-15-05..08)
- Plus: Already-Cancelled-Check via `member.exit_date.is_some()` → `ServiceError::Conflict(Arc::from("member already cancelled"))` (D-15-12, Planner-Discretion)
- `new_action.action_type = ActionType::Austritt`
- `new_action.shares_change = 0`
- `new_action.date = willensbekundung_date`
- `new_action.effective_date = Some(compute_effective_date(willensbekundung_date).effective_date)` (CANC-02, D-14-04..07)
- `new_action.transfer_member_id = None`
- `recalc_migrated` ist optional für Austritt (Planner-Check; analog `member_action.rs:352-353` ja, weil Austritt action_count beeinflusst)
- Return-Type: `(MemberAction, Member)` statt nur `MemberAction` — Member nach `recalc_dates` neu laden für Frontend-Refresh (D-15-11)

**Adaption für `increase_shares` (UPGD-01..04, D-15-03):**
- Selbe Permission/Process/Validation-Sequence
- `new_action.action_type = ActionType::Aufstockung`
- `new_action.shares_change = shares` (positiv, > 0 Validation Planner-Discretion)
- `new_action.date = willensbekundung_date`
- `new_action.effective_date = None` (UPGD-02 sofort wirksam)
- `new_action.transfer_member_id = None`
- **Already-Cancelled-Block (UPGD-04):** `if member.exit_date.is_some() { return Err(ServiceError::ValidationError(...)) }` oder `Conflict` (Planner-Discretion → HTTP 400 per Roadmap)
- **Zusätzlich `audited_update!` für `Member.current_shares` (D-15-03):**

```rust
let mut updated_member_entity = member_entity.clone();  // member_entity aus find_by_id oben
updated_member_entity.current_shares += shares;
updated_member_entity.version = self.uuid_service.new_v4().await;

crate::audited_update!(
    self,
    self.member_dao,
    member_entity.id,
    &updated_member_entity,
    UPGRADE_PROCESS,
    &user_id,
    tx
);
```

- `recalc_dates` für Aufstockung NICHT nötig (exit_date verändert sich nicht); `recalc_migrated` optional (Planner-Check — Aufstockung beeinflusst action_count, also vermutlich ja)

#### 3d) Pure-Function `validate_willensbekundung_date` (D-15-05..08)

**Analog für Struktur:** `compute_effective_date` (Z.21-33 in existing `membership_adjust.rs`) und `validate_action` (Z.76-153 in `member_action.rs`).

**Pattern aus `validate_action` Z.82-86 (ValidationFailureItem-Build):**
```rust
errors.push(ValidationFailureItem {
    field: Arc::from("shares_change"),
    message: Arc::from("Status actions must have shares_change = 0"),
});
```

**Phase-15-Impl (D-15-05, D-15-06, D-15-08):**
```rust
/// Validiert das Willensbekundungsdatum gegen die Kalender-Jahr-Bounds (D-15-06).
///
/// Erlaubt: aktuelles Geschäftsjahr (today.year()) und nächstes Geschäftsjahr (today.year() + 1).
/// Edge-Cases siehe Tests im `tests`-Submodul.
pub(crate) fn validate_willensbekundung_date(
    date: time::Date,
    today: time::Date,
) -> Vec<ValidationFailureItem> {
    let current_fy = today.year();
    let next_fy = current_fy + 1;
    if date.year() == current_fy || date.year() == next_fy {
        Vec::new()
    } else {
        vec![ValidationFailureItem {
            field: Arc::from("willensbekundung_date"),
            message: Arc::from(format!(
                "must be in fiscal year {} or {}",
                current_fy, next_fy
            )),
        }]
    }
}
```

**Service-Caller-Pattern (D-15-07):**
```rust
let today = time::OffsetDateTime::now_utc().date();
let errors = validate_willensbekundung_date(willensbekundung_date, today);
if !errors.is_empty() {
    return Err(ServiceError::ValidationError(errors));
}
```

**Edge-Case-Tests verbatim-Pattern (analog `compute_effective_date`-Tests Z.50-114):**
```rust
#[test]
fn test_validate_willensbekundung_aktuelles_jahr_valid() {
    let today = time::Date::from_calendar_date(2026, time::Month::March, 15).unwrap();
    let date = time::Date::from_calendar_date(2026, time::Month::June, 15).unwrap();
    let errors = validate_willensbekundung_date(date, today);
    assert!(errors.is_empty());
}
```

---

### Pattern 4: `recalc_dates`-Refactor zu Free-Function (D-15-04)

**Rolle:** Helper-Function-Refactor (in-place, kein Behavior-Change)
**Analog für aktuelle Methode:** `genossi_service_impl/src/member_action.rs:180-203`
**Analog für Pure-Function-Konvention:** `compute_dates` (Z.155-177, `pub(crate)` Free-Function mit generischen Argumenten).
**Verbatim kopieren:** Body-Logik (find_by_id → find_by_member_id → compute_dates → update_dates).
**Adaptieren:** Methode wird zu Free-Function mit expliziten Generic-Bounds; bestehende Methode `MemberActionServiceImpl::recalc_dates` bleibt als Delegations-Wrapper bestehen (kein Behavior-Change in `MemberActionService`).

**Aktuelle Methode verbatim (Z.180-203):**
```rust
impl<Deps: MemberActionServiceDeps> MemberActionServiceImpl<Deps> {
    async fn recalc_dates(
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

        let (join_date, exit_date) = compute_dates(&member, &actions);

        self.member_dao
            .update_dates(member_id, join_date, exit_date, tx)
            .await?;

        Ok(())
    }
```

**Vorgeschlagene Free-Function-Signatur (D-15-04, neu in `member_action.rs`):**
```rust
pub(crate) async fn recalc_dates<Md, Mad, Tx>(
    member_dao: &Md,
    member_action_dao: &Mad,
    member_id: Uuid,
    tx: Tx,
) -> Result<(), ServiceError>
where
    Md: genossi_dao::member::MemberDao<Transaction = Tx>,
    Mad: genossi_dao::member_action::MemberActionDao<Transaction = Tx>,
    Tx: genossi_dao::Transaction + Clone,
{
    let member = member_dao
        .find_by_id(member_id, tx.clone())
        .await?
        .ok_or(ServiceError::EntityNotFound(member_id))?;

    let actions = member_action_dao
        .find_by_member_id(member_id, tx.clone())
        .await?;

    let (join_date, exit_date) = compute_dates(&member, &actions);

    member_dao
        .update_dates(member_id, join_date, exit_date, tx)
        .await?;

    Ok(())
}
```

**Bestehende Methode wird Wrapper (kein Behavior-Change):**
```rust
impl<Deps: MemberActionServiceDeps> MemberActionServiceImpl<Deps> {
    async fn recalc_dates(
        &self,
        member_id: Uuid,
        tx: Deps::Transaction,
    ) -> Result<(), ServiceError> {
        recalc_dates(&*self.member_dao, &*self.member_action_dao, member_id, tx).await
    }
    // recalc_migrated bleibt unverändert (Planner-Check: Phase 15 braucht es ggf. ähnlich)
}
```

**Konsumption in `membership_adjust.rs::cancel_membership`:**
```rust
// Nach audited_create!:
crate::member_action::recalc_dates(
    &*self.member_dao,
    &*self.member_action_dao,
    member_id,
    tx.clone(),
).await?;
```

---

### Pattern 5: `audited_create!` + `audited_update!` Macro-Usage

**Rolle:** Audit-Wrapper für DAO-Mutations
**Analog für `audited_create!`:** `member_action.rs:342-349` (in `MemberActionService::create`)
**Analog für `audited_update!`:** `member_action.rs:383-391` (in `MemberActionService::update`)
**Verbatim kopieren:** Macro-Aufruf-Reihenfolge `(self, dao_field, &entity, PROCESS_CONST, &user_id, tx)`.
**Adaptieren:** Process-String pro Operation; bei `audited_update!` muss `entity_id` als 4. Argument vor `&new_entity` stehen.

**`audited_create!` verbatim (Z.342-349 `member_action.rs`):**
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

**Macro-Signatur Reminder (`audit_macros.rs:6`):**
```
audited_create!($self:expr, $dao:expr, $entity:expr, $process:expr, $user_id:expr, $tx:expr)
```

**Phase-15-Anwendung für `cancel_membership` (MemberAction::Austritt anlegen):**
```rust
let action_entity: genossi_dao::member_action::MemberActionEntity = (&new_action).into();
crate::audited_create!(
    self,
    self.member_action_dao,
    &action_entity,
    CANCEL_PROCESS,                  // "member-adjust.cancel"
    &user_id,
    tx
);
```

**`audited_update!` verbatim (Z.383-391 `member_action.rs`):**
```rust
let action_entity: genossi_dao::member_action::MemberActionEntity = item.into();
crate::audited_update!(
    self,
    self.member_action_dao,
    item.id,
    &action_entity,
    MEMBER_ACTION_SERVICE_PROCESS,
    &user_id,
    tx
);
```

**Macro-Signatur Reminder (`audit_macros.rs:43`):**
```
audited_update!($self:expr, $dao:expr, $entity_id:expr, $new_entity:expr, $process:expr, $user_id:expr, $tx:expr)
```

**Phase-15-Anwendung für `increase_shares` (Member.current_shares-Mutation, D-15-03):**
```rust
let mut updated = member_entity.clone();
updated.current_shares += shares;
updated.version = self.uuid_service.new_v4().await;

crate::audited_update!(
    self,
    self.member_dao,
    member_entity.id,                // entity_id
    &updated,                        // new_entity
    UPGRADE_PROCESS,                 // "member-adjust.upgrade"
    &user_id,
    tx
);
```

**Wichtig (CONTEXT-Reusable-Assets):** Macro ruft hardkodiert `$dao.update($new_entity, $process, $tx.clone())` auf (`audit_macros.rs:53`). Daher MUSS der generische `MemberDao::update`-Pfad (`genossi_dao/src/member.rs:111`) genutzt werden — keine targeted `update_current_shares`-Methode (AUDT-01 Grep-Gate).

---

### Pattern 6: REST-Handler `genossi_rest/src/member.rs` oder neuer `membership_adjust.rs`

**Rolle:** REST-Handler (Axum POST, JSON, Tuple-Response)
**Analog für POST-Handler:** `genossi_rest/src/member_action.rs:120-147` (`create_member_action`)
**Analog für Sub-Route-Mounting + Reihenfolge:** `genossi_rest/src/member.rs:29-55` (D-14-08 Lesson)
**Analog für Query-Param-Handler:** `genossi_rest/src/member.rs:117-143` (`get_transfer_recipients`)
**Verbatim kopieren:** `error_handler` + async-block Wrapper, `extract_auth_context(Some(context))?`, `serde_json::to_string`-Response-Build, `#[instrument(skip(rest_state))]`, `#[utoipa::path(...)]`-Annotation-Stil.
**Adaptieren:** Sub-Routes `/{id}/cancel` und `/{id}/increase-shares` MÜSSEN vor `/{id}`-catch-all deklariert werden (D-14-08); `Path((member_id,))` für Sub-Routes; Permission-Status-Codes 200/400/401/404/409 (D-15-12).

#### 6a) Route-Mounting verbatim (Z.29-55 `member.rs`):
```rust
pub fn generate_route<RestState: RestStateDef>() -> Router<RestState> {
    // Pitfall 1 (Phase 14 RESEARCH §"Sub-Route-Ordering"):
    // Literal sub-routes MUST be declared before any `/{id}` path-parameter
    // route, because axum matches routes in declaration order.
    Router::new()
        .route("/", get(get_all_members::<RestState>))
        // Literal sub-routes FIRST — MUST be declared before /{id} (Pitfall 1).
        .route(
            "/transfer-recipients",
            get(get_transfer_recipients::<RestState>),
        )
        .route("/import", post(import_members::<RestState>))
        .route(
            "/not-reached-by/{job_id}",
            get(get_members_not_reached_by::<RestState>),
        )
        // Path-parameter routes LAST.
        .route("/{id}", get(get_member::<RestState>))
        .route("/", post(create_member::<RestState>))
        .route("/{id}", put(update_member::<RestState>))
        .route("/{id}", delete(delete_member::<RestState>))
}
```

**Adaption für Phase 15 (D-15-09, D-14-08):**

Phase-15-Sub-Routes haben dynamischen `{id}` — sind aber nicht literal (sie kollidieren mit `/{id}/...` nicht direkt). **Wichtig:** Axum matched POST/GET-Methoden getrennt; `/{id}/cancel` ist POST, `/{id}` ist GET/PUT/DELETE — kein direkter Konflikt. Aber: `/{id}/actions` (existing `member_action::generate_route`-Mount) und neue `/{id}/cancel` müssen ohne Konflikt nebeneinander leben (`.nest()` vs `.route()`).

**Konkret zu ergänzen (vor `.route("/{id}", ...)`-Block):**
```rust
        .route(
            "/{id}/cancel",
            post(cancel_membership::<RestState>),
        )
        .route(
            "/{id}/increase-shares",
            post(increase_shares::<RestState>),
        )
```

#### 6b) Handler-Skeleton verbatim aus `member_action.rs:120-147` (`create_member_action`):

```rust
#[instrument(skip(rest_state))]
#[utoipa::path(
    post,
    tag = "Member Actions",
    path = "",
    params(
        ("member_id" = Uuid, Path, description = "Member ID"),
    ),
    request_body = MemberActionTO,
    responses(
        (status = 200, description = "Create action", body = MemberActionTO),
        (status = 400, description = "Validation error"),
        (status = 404, description = "Member not found"),
        (status = 401, description = "Unauthorized"),
    ),
)]
pub async fn create_member_action<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path(member_id): Path<Uuid>,
    Json(mut action): Json<MemberActionTO>,
) -> Response {
    action.member_id = member_id;
    error_handler(
        (async {
            let action = MemberActionTO::from(
                &rest_state
                    .member_action_service()
                    .create(
                        &(&action).into(),
                        crate::extract_auth_context(Some(context))?,
                        None,
                    )
                    .await?,
            );
            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&action)?))
                .unwrap())
        })
        .await,
    )
}
```

#### 6c) Phase-15-Adaption (`cancel_membership` Handler):

```rust
#[instrument(skip(rest_state))]
#[utoipa::path(
    post,
    tag = "Members",
    path = "/{id}/cancel",
    params(
        ("id" = Uuid, Path, description = "Member ID"),
    ),
    request_body = CancelMembershipRequestTO,
    responses(
        (status = 200, description = "Cancellation successful", body = MembershipAdjustResponseTO),
        (status = 400, description = "Validation error (date bounds, etc.)"),
        (status = 401, description = "Unauthorized — kein Login oder keine admin-Rolle"),
        (status = 404, description = "Member not found"),
        (status = 409, description = "Member already cancelled"),
    ),
)]
pub async fn cancel_membership<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path(member_id): Path<Uuid>,
    Json(req): Json<CancelMembershipRequestTO>,
) -> Response {
    error_handler(
        (async {
            let (action, member) = rest_state
                .membership_adjust_service()
                .cancel_membership(
                    member_id,
                    req.willensbekundung_date,
                    crate::extract_auth_context(Some(context))?,
                    None,
                )
                .await?;
            let response = MembershipAdjustResponseTO {
                action: MemberActionTO::from(&action),
                member: MemberTO::from(&member),
            };
            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&response)?))
                .unwrap())
        })
        .await,
    )
}
```

**Wichtige Hinweise:**
- `permission_denied` → 401 (Pitfall 4 aus `member.rs:110-113`): das globale `From<ServiceError> for RestError`-Mapping macht `PermissionDenied → Unauthorized (401)`. **KEIN** 403-Eintrag in `responses(...)`.
- `MembershipAdjustResponseTO` ist Planner-Discretion (D-15-11) — alternativ `serde_json::json!({"action": ..., "member": ...})`.

---

### Pattern 7: Request/Response DTOs `genossi_rest_types/src/lib.rs`

**Rolle:** Serde-DTOs für REST-Body
**Analog für Request-DTO mit Date-Feld:** `MemberSlimTO` (Z.348-376) und `MemberActionTO` (Z.420-461)
**Analog für ISO8601-Date-Required-Serde:** `MemberTO::join_date` (Z.186-191)
**Verbatim kopieren:** `#[derive(Debug, Serialize, Deserialize, ToSchema, ...)]`-Block, `#[serde(with = "iso8601_date_required")]` für Required-Date-Felder.
**Adaptieren:** Neue Struct-Namen + Felder, ISO8601-Date-Required-Serde benutzen (Pflichtfeld, nicht Optional).

**Verbatim Date-Feld-Pattern (`MemberTO::join_date` Z.186-191):**
```rust
    #[serde(
        serialize_with = "iso8601_date_required::serialize",
        deserialize_with = "iso8601_date_required::deserialize"
    )]
    #[schema(example = "2024-01-15")]
    pub join_date: time::Date,
```

**Phase-15-Adaption für `CancelMembershipRequestTO` (D-15-10):**
```rust
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct CancelMembershipRequestTO {
    #[serde(
        serialize_with = "iso8601_date_required::serialize",
        deserialize_with = "iso8601_date_required::deserialize"
    )]
    #[schema(example = "2026-06-15")]
    pub willensbekundung_date: time::Date,
}
```

**Phase-15-Adaption für `IncreaseSharesRequestTO`:**
```rust
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct IncreaseSharesRequestTO {
    #[serde(
        serialize_with = "iso8601_date_required::serialize",
        deserialize_with = "iso8601_date_required::deserialize"
    )]
    #[schema(example = "2026-06-15")]
    pub willensbekundung_date: time::Date,
    #[schema(example = 2)]
    pub shares: i32,
}
```

**Phase-15-Adaption für Response (D-15-11, optional benannt):**
```rust
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct MembershipAdjustResponseTO {
    pub action: MemberActionTO,
    pub member: MemberTO,
}
```

**Hinweis:** Roadmap-Success-Criteria erlaubt anonymen `serde_json::json!({"action": ..., "member": ...})` als Alternative. Benannter Typ macht OpenAPI-Schema klarer und ist für Frontend-Builder (Phase 18) einfacher.

---

### Pattern 8: DI-Wiring `genossi_bin/src/lib.rs`

**Rolle:** Dependency-Injection-Konfiguration und RestStateDef-Slot
**Analog für Deps-Struct:** `MemberActionServiceDependencies` (Z.461-478)
**Analog für RestStateImpl-Slot:** Z.577 (Feld), Z.682-691 (Konstruktion), Z.1064 (Init), Z.1756 (RestStateDef-type alias), Z.1780-1782 (Trait-Methoden-Impl)
**Verbatim kopieren:** Deps-Struct mit `unsafe impl Send/Sync` + Deps-Trait-Impl + Type-Alias.
**Adaptieren:** Service-Name auf `MembershipAdjust*`. Selbes Deps-Set wie `MemberActionService`.

**Verbatim Deps-Struct (Z.461-478):**
```rust
pub struct MemberActionServiceDependencies;

unsafe impl Send for MemberActionServiceDependencies {}
unsafe impl Sync for MemberActionServiceDependencies {}

impl MemberActionServiceDeps for MemberActionServiceDependencies {
    type Context = Context;
    type Transaction = Transaction;
    type MemberActionDao = MemberActionDao;
    type MemberDao = MemberDao;
    type AuditLogDao = AuditLogDao;
    type PermissionService = PermissionService;
    type UuidService = UuidService;
    type TransactionDao = TransactionDao;
}

type MemberActionService =
    genossi_service_impl::member_action::MemberActionServiceImpl<MemberActionServiceDependencies>;
```

**Phase-15-Adaption (nach Z.478 einfügen, D-15-16):**
```rust
pub struct MembershipAdjustServiceDependencies;

unsafe impl Send for MembershipAdjustServiceDependencies {}
unsafe impl Sync for MembershipAdjustServiceDependencies {}

impl genossi_service_impl::membership_adjust::MembershipAdjustServiceDeps
    for MembershipAdjustServiceDependencies
{
    type Context = Context;
    type Transaction = Transaction;
    type MemberActionDao = MemberActionDao;
    type MemberDao = MemberDao;
    type AuditLogDao = AuditLogDao;
    type PermissionService = PermissionService;
    type UuidService = UuidService;
    type TransactionDao = TransactionDao;
}

type MembershipAdjustService = genossi_service_impl::membership_adjust::MembershipAdjustServiceImpl<
    MembershipAdjustServiceDependencies,
>;
```

**RestStateImpl-Feld verbatim (Z.577):**
```rust
    member_action_service: Arc<MemberActionService>,
```

**Phase-15-Ergänzung (neben Z.577 einfügen):**
```rust
    membership_adjust_service: Arc<MembershipAdjustService>,
```

**Konstruktion in `new()` verbatim (Z.682-691):**
```rust
        let member_action_service = Arc::new(
            genossi_service_impl::member_action::MemberActionServiceImpl {
                member_action_dao: member_action_dao.clone(),
                member_dao: member_dao.clone(),
                audit_log_dao: audit_log_dao.clone(),
                permission_service: permission_service.clone(),
                uuid_service: uuid_service.clone(),
                transaction_dao: transaction_dao.clone(),
            },
        );
```

**Phase-15-Adaption (parallel dazu konstruieren, danach):**
```rust
        let membership_adjust_service = Arc::new(
            genossi_service_impl::membership_adjust::MembershipAdjustServiceImpl {
                member_action_dao: member_action_dao.clone(),
                member_dao: member_dao.clone(),
                audit_log_dao: audit_log_dao.clone(),
                permission_service: permission_service.clone(),
                uuid_service: uuid_service.clone(),
                transaction_dao: transaction_dao.clone(),
            },
        );
```

**Self-Init verbatim (Z.1064):**
```rust
            member_action_service,
```

**Phase-15-Ergänzung (im `Self { ... }`-Block ergänzen):**
```rust
            membership_adjust_service,
```

**RestStateDef-Trait-Methode verbatim (Z.1780-1782):**
```rust
    fn member_action_service(&self) -> Arc<Self::MemberActionService> {
        self.member_action_service.clone()
    }
```

**Phase-15-Ergänzung:**
- In `genossi_rest/src/lib.rs` (RestStateDef-Trait Z.194-254): neuer Associated-Type `MembershipAdjustService: genossi_service::membership_adjust::MembershipAdjustService<Context = ContextType> + Send + Sync + 'static;` + Trait-Method `fn membership_adjust_service(&self) -> Arc<Self::MembershipAdjustService>;`.
- In `genossi_bin/src/lib.rs::impl RestStateDef` (Z.1751-): `type MembershipAdjustService = MembershipAdjustService;` + Method-Impl `fn membership_adjust_service(&self) -> Arc<Self::MembershipAdjustService> { self.membership_adjust_service.clone() }`.
- In `genossi_rest/src/lib.rs::OpenApi::nest`-Block (Z.256-): KEIN neuer nest-Eintrag (Sub-Routes sind innerhalb `/api/members`).

---

### Pattern 9: E2E-Test `genossi_bin/tests/cancel_membership_e2e.rs` (NEU)

**Rolle:** End-to-End-Test mit echtem HTTP-Server + In-Memory-SQLite
**Analog:** `genossi_bin/tests/transfer_recipients_e2e.rs:1-265` (vollständig)
**Verbatim kopieren:** `#![cfg(feature = "mock_auth")]`, `setup()` (Z.32-46), `sample_member()` (Z.52-80), `create_active_member()` (Z.84-103), HTTP-Client-Pattern, `assert_eq!(resp.status(), StatusCode::...)`.
**Adaptieren:** Test-Namen analog Phase-15-Specifics (`test_cancel_membership_happy_path_h1`, ...), POST-Bodies an `CancelMembershipRequestTO`/`IncreaseSharesRequestTO`, Response-Parsing in `MembershipAdjustResponseTO`.

**Setup verbatim (Z.27-46):**
```rust
#![cfg(feature = "mock_auth")]

use genossi_bin::RestStateImpl;
use genossi_rest::test_server::test_support::{start_test_server, TestServer};
use genossi_rest_types::{ActionTypeTO, MemberActionTO, MemberSlimTO, MemberTO};
use reqwest::StatusCode;
use sqlx::SqlitePool;
use std::sync::Arc;

async fn setup() -> TestServer {
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

**Active-Member-Helper verbatim (Z.84-103):**
```rust
async fn create_active_member(
    client: &reqwest::Client,
    server: &TestServer,
    member_number: i64,
    first_name: &str,
) -> MemberTO {
    let m = sample_member(member_number, first_name);
    let response = client
        .post(server.url("/api/members"))
        .json(&m)
        .send()
        .await
        .expect("create_member POST failed");
    assert!(
        response.status().is_success(),
        "create_member expected 2xx, got {}",
        response.status()
    );
    response.json().await.expect("decode MemberTO")
}
```

**Phase-15-Adaption für `test_cancel_membership_happy_path_h1`:**
```rust
#[tokio::test]
async fn test_cancel_membership_happy_path_h1() {
    let server = setup().await;
    let client = reqwest::Client::new();
    let m = create_active_member(&client, &server, 1001, "Kuendigung").await;
    let member_id = m.id.expect("created member must have id");

    let body = serde_json::json!({
        "willensbekundung_date": "2026-03-15"  // H1
    });
    let resp = client
        .post(server.url(&format!("/api/members/{}/cancel", member_id)))
        .json(&body)
        .send()
        .await
        .expect("POST cancel failed");

    assert_eq!(resp.status(), StatusCode::OK);
    let response: serde_json::Value = resp.json().await.expect("decode response");
    let action = &response["action"];
    let member = &response["member"];
    // H1: 15.03.2026 → effective_date = 31.12.2026
    assert_eq!(action["effective_date"], "2026-12-31");
    assert_eq!(action["action_type"], "Austritt");
    assert_eq!(action["shares_change"], 0);
    // Member.exit_date wurde von recalc_dates gesetzt
    assert_eq!(member["exit_date"], "2026-12-31");
}
```

**Already-Cancelled-Test (D-15-12, HTTP 409 per Roadmap-Success-Criteria):**
```rust
#[tokio::test]
async fn test_cancel_membership_already_cancelled() {
    let server = setup().await;
    let client = reqwest::Client::new();
    let m = create_active_member(&client, &server, 1001, "Kuendigung").await;
    let member_id = m.id.expect("created member must have id");

    let body = serde_json::json!({"willensbekundung_date": "2026-03-15"});

    // First cancel: 200
    let resp1 = client
        .post(server.url(&format!("/api/members/{}/cancel", member_id)))
        .json(&body).send().await.expect("first cancel");
    assert_eq!(resp1.status(), StatusCode::OK);

    // Second cancel: 409
    let resp2 = client
        .post(server.url(&format!("/api/members/{}/cancel", member_id)))
        .json(&body).send().await.expect("second cancel");
    assert_eq!(resp2.status(), StatusCode::CONFLICT);
}
```

---

## Shared Patterns

### Permission-Funnel
**Source:** `genossi_service/src/permission.rs:28` (`ADMIN_PRIVILEGE = "admin"`) und `genossi_service_impl/src/member_action.rs:303-305`
**Apply to:** Alle `MembershipAdjustService`-Methoden (PERM-01)
```rust
self.permission_service
    .check_permission(genossi_service::permission::ADMIN_PRIVILEGE, context)
    .await?;
```

### Transaction-Lifecycle
**Source:** `genossi_service_impl/src/member_action.rs:295, 355` (in `create`)
**Apply to:** Alle Service-Methoden mit DAO-Calls
```rust
// Start
let tx = self.transaction_dao.use_transaction(tx).await?;
// ... DAO-Calls mit tx.clone() ...
// End
self.transaction_dao.commit(tx).await?;
```

### current_user_id Pattern
**Source:** `genossi_service_impl/src/member_action.rs:297-301`
**Apply to:** Alle auditierten Service-Methoden
```rust
let user_id = self
    .permission_service
    .current_user_id(context.clone())
    .await?
    .unwrap_or_else(|| "SYSTEM".to_string());
```

### Member-Existence-Check
**Source:** `genossi_service_impl/src/member_action.rs:313-316`
**Apply to:** `cancel_membership`, `increase_shares` (CONTEXT specifics: 1. Member laden 2. exit_date prüfen 3. updaten)
```rust
let member_entity = self
    .member_dao
    .find_by_id(member_id, tx.clone())
    .await?
    .ok_or(ServiceError::EntityNotFound(member_id))?;
```

### ServiceError → RestError → HTTP-Status-Mapping
**Source:** `genossi_rest/src/member.rs:110-113` (Pitfall 4)
**Apply to:** Alle Handler — KEIN 403 in `responses(...)` listen; `PermissionDenied → 401`. `ValidationError → 400`, `Conflict → 409`, `EntityNotFound → 404`.

### error_handler-Wrapper
**Source:** `genossi_rest/src/member.rs:72-88` und `member_action.rs:127-146`
**Apply to:** Alle REST-Handler
```rust
error_handler(
    (async {
        // ... Service-Call + Response-Build ...
        Ok(Response::builder()
            .status(200)
            .header("Content-Type", "application/json")
            .body(Body::new(serde_json::to_string(&result)?))
            .unwrap())
    })
    .await,
)
```

### ValidationFailureItem-Build
**Source:** `genossi_service_impl/src/member_action.rs:82-86`
**Apply to:** `validate_willensbekundung_date` und alle künftigen Validation-Helpers
```rust
ValidationFailureItem {
    field: Arc::from("willensbekundung_date"),
    message: Arc::from(format!("must be in fiscal year {} or {}", current_fy, next_fy)),
}
```

### Pure-Function-Konvention (D-15-05, D-14-03)
- `pub(crate)` Visibility
- Free-Function, kein `&self`
- Deterministische Inputs (Datum, today als Parameter — kein interner `OffsetDateTime::now_utc()`)
- `#[cfg(test)] mod tests` im selben Modul
- Edge-Case-Tests als 6+ deterministische `#[test]`-Funktionen (Konvention-Vorbild: `compute_effective_date` Z.50-114)

### Soft-Delete-Filter und Date-Field-Patterns
- `Member.exit_date IS NOT NULL` → gekündigt (Already-Cancelled-Heuristik aus `claude_discretion`)
- `MemberAction.effective_date: Option<Date>` ist nullable; Aufstockung-Action setzt `None` (UPGD-02)

---

## Keine Analoga gefunden

Keine — alle Pattern-Quellen existieren in v1.1 / Phase 14 bereits. Phase 15 ist ein Pattern-Reuse-Phase, kein Pattern-Establishment.

| Datei | Rolle | Grund |
|-------|-------|-------|
| _(keine)_ | _(keine)_ | _(keine)_ |

---

## Metadata

**Analog-Suche-Scope:**
- `genossi_service/src/` (Service-Traits)
- `genossi_service_impl/src/` (Service-Impls + Pure-Functions + Macros)
- `genossi_rest/src/` (REST-Handler + Router-Mounting)
- `genossi_rest_types/src/` (DTOs)
- `genossi_dao/src/` (DAO-Trait-Signaturen)
- `genossi_bin/src/lib.rs` (DI-Wiring + RestStateImpl)
- `genossi_bin/tests/` (E2E-Pattern)

**Files gescannt:**
- `genossi_service/src/member_action.rs` (134 lines)
- `genossi_service_impl/src/member_action.rs` (1048 lines, Auszüge)
- `genossi_service_impl/src/membership_adjust.rs` (116 lines)
- `genossi_service_impl/src/audit_macros.rs` (128 lines)
- `genossi_service_impl/src/macros.rs` (42 lines)
- `genossi_rest/src/member.rs` (~200 lines aus 700+)
- `genossi_rest/src/member_action.rs` (160 lines aus 290+)
- `genossi_rest/src/lib.rs` (RestStateDef-Trait-Block + OpenAPI-nest)
- `genossi_rest_types/src/lib.rs` (Auszüge bei DTO-Mustern)
- `genossi_bin/src/lib.rs` (DI-Wiring-Auszüge)
- `genossi_bin/tests/transfer_recipients_e2e.rs` (265 lines)

**Pattern-Extraktion-Datum:** 2026-06-04
**Phase:** 15-service-rest-kuendigung-aufstockung
