# Phase 17: service-rest-uebertrag-cascade - Pattern Map

**Mapped:** 2026-06-06
**Files analyzed:** 6 new/modified
**Analogs found:** 6 / 6

All Phase 17 files have direct local analogs in the **same files** they will modify — Phase 17 extends `MembershipAdjustService` incrementally (D-15-13 / C-17-CF-01). No new files. The closest analogs are the existing methods `cancel_membership` (Phase 15), `increase_shares` (Phase 15) and `partial_repayment` (Phase 16), which already implement the canonical single-tx-cascade pattern.

## File Classification

| Modified File | Role | Data Flow | Closest Analog | Match Quality |
|---------------|------|-----------|----------------|---------------|
| `genossi_service/src/membership_adjust.rs` | service-trait | request-response | same file, `partial_repayment` trait method (lines 65–72) | exact |
| `genossi_service_impl/src/membership_adjust.rs` | service-impl | CRUD + cascade | same file, `partial_repayment` impl (lines 283–467); `cancel_membership` (lines 78–165); `validate_partial_repayment_shares` (lines 540–568) | exact |
| `genossi_rest_types/src/lib.rs` | DTO | request-response | same file, `PartialRepaymentRequestTO` (lines 546–561); `PartialRepaymentResponseTO` (lines 563–574) | exact |
| `genossi_rest/src/membership_adjust.rs` | rest-handler | request-response | same file, `partial_repayment` handler (lines 129–184) | exact |
| `genossi_rest/src/member.rs` | route-wiring | request-response | same file, sub-routes 64–77 (cancel, increase-shares, partial-repayment) | exact |
| `genossi_bin/tests/membership_adjust_e2e.rs` *(new file)* | e2e-test | request-response | same crate, `membership_adjust_e2e.rs` (lines 1–889); race-test analog in `genossi_bin/tests/e2e_tests.rs:12474–12596` | exact |

> Note: CONTEXT.md mentions `genossi_bin/tests/e2e_tests.rs` for new E2E tests, but the actual Phase-15/16 E2E file is `genossi_bin/tests/membership_adjust_e2e.rs`. The race-test pattern lives in `e2e_tests.rs:12474–12596` (Phase 9 D-12). Planner should append Phase-17 tests to the existing `membership_adjust_e2e.rs` (consistent with Phase 16) and copy the race-test pattern into it. The CONTEXT pointer was off-by-one-file.

## Pattern Assignments

### 1. `genossi_service/src/membership_adjust.rs` (service-trait, request-response)

**Analog:** `genossi_service/src/membership_adjust.rs` (same file, `partial_repayment` method).

**Imports pattern** (lines 1–16) — already in place, add only `Authentication`, `Member`, `MemberAction`:
```rust
use async_trait::async_trait;
use mockall::automock;
use std::fmt::Debug;
use uuid::Uuid;

use crate::member::Member;
use crate::member_action::MemberAction;
use crate::permission::Authentication;
use crate::ServiceError;
```

**Trait extension pattern** (lines 65–72, `partial_repayment` as model) — Phase 17 adds a NEW method to the existing trait, NOT a new trait (C-17-CF-01 = D-15-13 incremental growth):
```rust
async fn partial_repayment(
    &self,
    member_id: Uuid,
    shares: i32,
    willensbekundung_date: time::Date,
    context: Authentication<Self::Context>,
    tx: Option<Self::Transaction>,
) -> Result<(Member, RepaymentEntry, Option<RepaymentPhase>), ServiceError>;
```

**Phase-17-specific signature** (adapt per CONTEXT D-17 / `In scope` section; note shares is `i32` to match `Member.current_shares: i32`, divergent from CONTEXT's stale `i64`):
```rust
/// Uebertraegt `shares` Anteile von `from_id` an `to_id`. Sofort wirksam,
/// kein H1/H2-Stichtag (TRSF-05). Bei Voll-Uebertrag (`from.current_shares == shares`)
/// wird zusaetzlich ein `MemberAction::Austritt(from)` mit
/// `transfer_member_id = Some(to_id)` erzeugt (D-17-01/03).
///
/// Returns tuple `(actions, from, to)`:
/// - `actions`: 2 (Teil) oder 3 (Voll) `MemberAction`-Eintraege (Abgabe, Empfang,
///   optional Austritt). C-17-CF-08 — domain-Werte, kein DTO-Wrapping.
/// - `from`/`to`: aktualisierte Member nach Tx-Commit.
async fn transfer_shares(
    &self,
    from_id: Uuid,
    to_id: Uuid,
    shares: i32,
    transfer_date: time::Date,
    context: Authentication<Self::Context>,
    tx: Option<Self::Transaction>,
) -> Result<(Vec<MemberAction>, Member, Member), ServiceError>;
```

**Mock impact:** `#[automock(...)]` already on the trait (line 22). The added method auto-gets a mock — no per-test rewrite needed for genossi-internal mocks.

---

### 2. `genossi_service_impl/src/membership_adjust.rs` (service-impl, CRUD + cascade)

**Analog:** `partial_repayment` (lines 283–467) for single-tx-cascade with multiple `audited_create!`/`audited_update!`; `cancel_membership` (lines 78–165) for the `recalc_dates` hook; `validate_partial_repayment_shares` (lines 540–568) for pure-function pattern.

**Process-string constant** (D-17-04, analog to lines 32–48):
```rust
/// Shared Audit-Process-String fuer ALLE Cascade-Writes (D-17-04 / AUDT-02).
/// Filter `WHERE process = 'member-adjust.transfer'` findet ALLE Writes eines
/// Uebertrag-Vorgangs (2 oder 3 MemberAction-Creates + 2 Member-Updates).
const TRANSFER_PROCESS: &str = "member-adjust.transfer";
```

**Pure-function pattern** (lines 540–568, `validate_partial_repayment_shares` as model — D-17-09):
```rust
/// Pure-Function range-validator fuer Uebertrag (D-17-09).
///
/// Wirft `ValidationError` bei:
/// - `from_id == to_id` (TRSF-07 self-transfer)
/// - `shares <= 0` (mindestens 1 Anteil)
/// - `shares > from_current_shares` (Voll-Uebertrag-Boundary inkl., n == current
///   ist GUELTIG — Voll-Uebertrag-Branch wird im Service ausgewertet, D-17-01)
pub(crate) fn validate_transfer_inputs(
    from_id: Uuid,
    to_id: Uuid,
    shares: i32,
    from_current_shares: i32,
) -> Vec<ValidationFailureItem> {
    let mut errors = Vec::new();
    if from_id == to_id {
        errors.push(ValidationFailureItem {
            field: Arc::from("to_member_id"),
            message: Arc::from("cannot transfer to self"),
        });
    }
    if shares <= 0 {
        errors.push(ValidationFailureItem {
            field: Arc::from("shares"),
            message: Arc::from("shares must be at least 1"),
        });
    }
    if shares > from_current_shares {
        errors.push(ValidationFailureItem {
            field: Arc::from("shares"),
            message: Arc::from(format!(
                "shares ({}) exceeds from.current_shares ({})",
                shares, from_current_shares
            )),
        });
    }
    errors
}
```

**Method skeleton** — pipeline pattern copied directly from `partial_repayment` (lines 283–467). Key cascade-pattern excerpts:

**Permission-funnel + Tx start** (lines 291–304, `partial_repayment`):
```rust
let tx = self.transaction_dao.use_transaction(tx).await?;

let user_id = self
    .permission_service
    .current_user_id(context.clone())
    .await?
    .unwrap_or_else(|| "SYSTEM".to_string());

// PERM-01 (ADMIN_PRIVILEGE-Funnel).
self.permission_service
    .check_permission(ADMIN_PRIVILEGE, context)
    .await?;
```

**Member-existence + Conflict-mapping** (lines 306–323, `partial_repayment`):
```rust
let member_entity = self
    .member_dao
    .find_by_id(member_id, tx.clone())
    .await?
    .ok_or(ServiceError::EntityNotFound(member_id))?;

// D-17-07 / PERM-03: recipient cancelled -> HTTP 409.
if to_entity.exit_date.is_some() {
    return Err(ServiceError::Conflict(Arc::from(
        "recipient already cancelled",
    )));
}
```

**Pure-validation guard** (lines 326–328, `partial_repayment` → `validate_partial_repayment_shares`):
```rust
if let Err(errs) = validate_transfer_inputs(
    from_id, to_id, shares, from_entity.current_shares,
) {
    return Err(ServiceError::ValidationError(errs));
}
```

**Re-use of Phase-15 date-validation** (D-17-09 / C-17-CF-05; lines 330–335 from `partial_repayment`):
```rust
let today = time::OffsetDateTime::now_utc().date();
let validation_errors = validate_willensbekundung_date(transfer_date, today);
if !validation_errors.is_empty() {
    return Err(ServiceError::ValidationError(validation_errors));
}
```

**Voll-Uebertrag detection** (D-17-01, pre-write service-check):
```rust
let will_become_zero = from_entity.current_shares - shares == 0;
```

**audited_create! for MemberAction with transfer_member_id** (lines 137–144 + 235–243 cancel/upgrade; this is the canonical pattern for Cascade-Action-Create):
```rust
let now = time::OffsetDateTime::now_utc();
let abgabe_action = MemberActionEntity {
    id: self.uuid_service.new_v4().await,
    member_id: from_entity.id,
    action_type: ActionType::UebertragungAbgabe,
    date: transfer_date,
    shares_change: -shares,           // Abgabe = negativ
    transfer_member_id: Some(to_entity.id),
    effective_date: None,             // TRSF-05 sofort wirksam, kein H1/H2
    comment: None,
    created: time::PrimitiveDateTime::new(now.date(), now.time()),
    deleted: None,
    version: self.uuid_service.new_v4().await,
};
crate::audited_create!(
    self,
    self.member_action_dao,
    &abgabe_action,
    TRANSFER_PROCESS,         // <-- shared D-17-04
    &user_id,
    tx
);
```

**audited_update! pattern for current_shares mutation** (lines 256–267 from `increase_shares` — critical optimistic-locking note: do NOT bump `entity.version` manually, the DAO does it internally):
```rust
// Optimistic-Locking-Note (Rule-1 fix from increase_shares):
// `MemberDao::update` (genossi_dao_impl_sqlite/src/member.rs:209-300) liest die
// ALTE Version aus `entity.version` (WHERE-Klausel) und generiert die NEUE
// Version INTERN. Deshalb wird `entity.version` hier NICHT gebumpt.
let mut from_updated = from_entity.clone();
from_updated.current_shares -= shares;

crate::audited_update!(
    self,
    self.member_dao,
    from_entity.id,
    &from_updated,
    TRANSFER_PROCESS,
    &user_id,
    tx
);
```

**Optional 3rd cascade action (Voll-Uebertrag-Austritt)** (D-17-01/03; uses `effective_date = Some(transfer_date)` per CONTEXT specifics + line 128 from `cancel_membership` pattern):
```rust
if will_become_zero {
    let austritt_action = MemberActionEntity {
        id: self.uuid_service.new_v4().await,
        member_id: from_entity.id,
        action_type: ActionType::Austritt,
        date: transfer_date,
        shares_change: 0,                              // CANC-03-Konvention
        transfer_member_id: Some(to_entity.id),        // D-17-03 (divergiert von Phase-15-CANC = None)
        effective_date: Some(transfer_date),           // TRSF-05 sofort wirksam
        comment: None,
        created: time::PrimitiveDateTime::new(now.date(), now.time()),
        deleted: None,
        version: self.uuid_service.new_v4().await,
    };
    crate::audited_create!(
        self,
        self.member_action_dao,
        &austritt_action,
        TRANSFER_PROCESS,
        &user_id,
        tx
    );
}
```

**`recalc_dates` Free-Function call** (lines 146–153 from `cancel_membership` — Phase 17 calls ONCE for `from.id` only, D-17-02):
```rust
// D-17-02 / CANC-04: exit_date wird via recalc_dates aus dem (optional erzeugten)
// Austritt-Action abgeleitet. KEINE direkte exit_date-Mutation. Bei Teil-
// Uebertrag No-Op (kein Austritt-Action in der Liste).
crate::member_action::recalc_dates(
    &*self.member_dao,
    &*self.member_action_dao,
    from_entity.id,
    tx.clone(),
)
.await?;
```

**Commit + re-read + return tuple** (lines 156–164 from `cancel_membership`):
```rust
let from_final = self
    .member_dao
    .find_by_id(from_entity.id, tx.clone())
    .await?
    .ok_or(ServiceError::EntityNotFound(from_entity.id))?;
let to_final = self
    .member_dao
    .find_by_id(to_entity.id, tx.clone())
    .await?
    .ok_or(ServiceError::EntityNotFound(to_entity.id))?;

self.transaction_dao.commit(tx).await?;

Ok((
    vec![abgabe_action_domain, empfang_action_domain, /* optional austritt */],
    Member::from(&from_final),
    Member::from(&to_final),
))
```

**Test-module pattern** (lines 689–741 for pure-function tests + 744+ for service-tests with mock):
```rust
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn validate_transfer_inputs_self_rejected() {
        let id = Uuid::new_v4();
        let errs = validate_transfer_inputs(id, id, 1, 5);
        assert!(errs.iter().any(|e| &*e.field == "to_member_id"));
    }
    // ... 5 weitere Edge-Cases analog Phase 16 validate_partial_repayment_shares_*
}
```

For service-tests (mock pattern), reuse the `mock!`/`TestDeps`/`build_service_part` scaffolding from lines 778–1015 — all required mocks already exist (MemberDao, MemberActionDao, AuditLogDao, PermissionService, TxDao). No new mock needed.

---

### 3. `genossi_rest_types/src/lib.rs` (DTO, request-response)

**Analog:** `PartialRepaymentRequestTO` (lines 546–561) for request DTO with `iso8601_date_required` serde; `PartialRepaymentResponseTO` (lines 563–574) for multi-field response wrapper.

**Request DTO pattern** (D-17 / specifics):
```rust
/// Request-Body fuer `POST /api/members/{from_id}/transfer-shares` (TRSF-01).
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct TransferSharesRequestTO {
    pub to_member_id: Uuid,
    /// Anzahl der zu uebertragenden Anteile (1..from.current_shares incl.;
    /// `n == from.current_shares` triggert Voll-Uebertrag mit Austritt-Cascade,
    /// D-17-01). Type `i32` konsistent mit `MemberEntity.current_shares`.
    #[schema(example = 2)]
    pub shares: i32,
    #[serde(
        serialize_with = "iso8601_date_required::serialize",
        deserialize_with = "iso8601_date_required::deserialize"
    )]
    #[schema(example = "2026-06-15")]
    pub transfer_date: time::Date,
}
```

**Response DTO pattern** (D-17 / Claude's-Discretion: benannt bevorzugt für OpenAPI):
```rust
/// Response-Body fuer `POST /api/members/{from_id}/transfer-shares` (C-17-CF-07).
///
/// `actions.len()` ist 2 (Teil) oder 3 (Voll-Uebertrag). Frontend braucht
/// `from` + `to` separat fuer Single-Round-Trip-Detail-Refresh (Phase 18).
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct TransferSharesResponseTO {
    pub actions: Vec<MemberActionTO>,
    pub from: MemberTO,
    pub to: MemberTO,
}
```

**Already-imported helpers** (no new module needed):
- `MemberTO` (line ~187)
- `MemberActionTO` (lines 421–502)
- `iso8601_date_required` (lines 84+)
- `Uuid` import already present

---

### 4. `genossi_rest/src/membership_adjust.rs` (rest-handler, request-response)

**Analog:** `partial_repayment` handler (lines 129–184) — same response-shape complexity (multi-field), same DTO-wrapping pattern.

**Module-doc + imports** (lines 1–32, add new DTOs only):
```rust
use genossi_rest_types::{
    /* existing */
    TransferSharesRequestTO, TransferSharesResponseTO,
};
```

**Handler pattern** (lines 129–184, `partial_repayment` as model — copy + adjust):
```rust
#[instrument(skip(rest_state))]
#[utoipa::path(
    post,
    tag = "Members",
    path = "/{from_id}/transfer-shares",
    params(("from_id" = Uuid, Path, description = "Sender Member ID")),
    request_body = TransferSharesRequestTO,
    responses(
        (status = 200, description = "Transfer successful (200 returns Voll- oder Teil-Uebertrag)", body = TransferSharesResponseTO),
        (status = 400, description = "Validation error (self-transfer, shares out of range, transfer_date out of bounds)"),
        // D-15-12 / Phase 15 Resolution: ServiceError::PermissionDenied -> 401.
        (status = 401, description = "Unauthorized — kein Login oder keine admin-Rolle"),
        (status = 404, description = "From or to member not found / soft-deleted"),
        // D-17-07 / PERM-03: recipient already cancelled.
        (status = 409, description = "Recipient already cancelled or optimistic-locking conflict"),
        (status = 500, description = "SQLITE_BUSY mid-cascade (Race-Test Verlierer-Pfad)"),
    ),
)]
pub async fn transfer_shares<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path(from_id): Path<Uuid>,
    Json(req): Json<TransferSharesRequestTO>,
) -> Response {
    error_handler(
        (async {
            let (actions, from, to) = rest_state
                .membership_adjust_service()
                .transfer_shares(
                    from_id,
                    req.to_member_id,
                    req.shares,
                    req.transfer_date,
                    crate::extract_auth_context(Some(context))?,
                    None,
                )
                .await?;
            let response = TransferSharesResponseTO {
                actions: actions.iter().map(MemberActionTO::from).collect(),
                from: MemberTO::from(&from),
                to: MemberTO::from(&to),
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

**ApiDoc registration** (lines 186–198, append `transfer_shares`):
```rust
#[derive(utoipa::OpenApi)]
#[openapi(
    paths(cancel_membership, increase_shares, partial_repayment, transfer_shares),
    components(schemas(
        CancelMembershipRequestTO,
        IncreaseSharesRequestTO,
        MembershipAdjustResponseTO,
        PartialRepaymentRequestTO,
        PartialRepaymentResponseTO,
        TransferSharesRequestTO,
        TransferSharesResponseTO,
    )),
    tags((name = "Members", description = "Phase 15-17 v1.2 membership-adjust endpoints"))
)]
pub struct ApiDoc;
```

---

### 5. `genossi_rest/src/member.rs` (route-wiring, request-response)

**Analog:** Lines 64–77 of `generate_route` — three existing `/{id}/...` sub-routes (cancel, increase-shares, partial-repayment) registered BEFORE the catch-all `/{id}` routes (D-14-08 / C-17-CF-06).

**Route registration pattern** (insert just before line 78, after `partial-repayment`):
```rust
        // Phase 17 v1.2 (D-17 / C-17-CF-06): Sub-Route fuer Uebertrag.
        // MUSS vor /{id} registriert sein (D-14-08-Lesson) — axum-Routing-Defense.
        .route(
            "/{from_id}/transfer-shares",
            post(crate::membership_adjust::transfer_shares::<RestState>),
        )
        // Path-parameter routes LAST.
        .route("/{id}", get(get_member::<RestState>))
```

**Critical ordering rule** (lines 30–36, the "Pitfall 1" comment that documents D-14-08):
```rust
// Pitfall 1 (Phase 14 RESEARCH §"Sub-Route-Ordering"):
// Literal sub-routes MUST be declared before any `/{id}` path-parameter
// route, because axum matches routes in declaration order.
```

**Path-param-naming consistency:** Phase-17 uses `{from_id}` (sender = path param, recipient = body field) which matches the canonical-refs in CONTEXT.md (`POST /api/members/{from_id}/transfer-shares`). The other sub-routes use `{id}`; mixing is fine as the path parameter is local to the route handler.

---

### 6. `genossi_bin/tests/membership_adjust_e2e.rs` (e2e-test, request-response)

**Primary analog:** `genossi_bin/tests/membership_adjust_e2e.rs` lines 1–889 (Phase 15 + 16 test file). Setup helpers, member-helpers, audit-chain-verify patterns, and the `setup()` fn already exist.
**Race-test analog:** `genossi_bin/tests/e2e_tests.rs:12474–12596` (Phase 9 D-12 / `test_mark_paid_out_race_one_succeeds_one_conflicts`).

#### Test-Setup re-use (no copying needed)

Already defined in `membership_adjust_e2e.rs:39–150`:
- `async fn setup() -> TestServer` (lines 39–53)
- `fn sample_member(member_number, first_name) -> MemberTO` (lines 59–87)
- `async fn create_active_member(...)` (lines 89–109)
- `async fn put_member_current_shares(client, server, member, target_shares) -> MemberTO` (lines 487–511) — needed for `from.current_shares = 3` test setups.
- `fn today_march_15()` / `today_august_15()` / `current_year_dec_31()` (lines 128–150)

#### Body-helper pattern (analog to lines 477–482, `partial_repayment_body`):
```rust
fn transfer_shares_body(to: &Uuid, shares: i32, transfer_date: &str) -> Value {
    serde_json::json!({
        "to_member_id": to.to_string(),
        "shares": shares,
        "transfer_date": transfer_date,
    })
}
```

#### Happy-Path test pattern (lines 543–573, `test_partial_repayment_happy_path_h1`):
```rust
#[tokio::test]
async fn test_transfer_shares_partial_happy_path() {
    let server = setup().await;
    let client = reqwest::Client::new();
    let a = create_active_member(&client, &server, 1200, "FromA").await;
    let a = put_member_current_shares(&client, &server, &a, 5).await;
    let b = create_active_member(&client, &server, 1201, "ToB").await;
    let a_id = a.id.expect("id");
    let b_id = b.id.expect("id");

    let transfer_date = today_march_15();

    let resp = client
        .post(server.url(&format!("/api/members/{}/transfer-shares", a_id)))
        .json(&transfer_shares_body(&b_id, 2, &transfer_date.to_string()))
        .send()
        .await
        .expect("POST transfer-shares");
    assert_eq!(resp.status(), StatusCode::OK);

    let body: Value = resp.json().await.expect("decode");
    // Teil-Uebertrag = 2 Actions, kein Austritt.
    assert_eq!(body["actions"].as_array().expect("actions").len(), 2);
    assert_eq!(body["from"]["current_shares"], 3);
    assert!(
        body["from"]["exit_date"].is_null(),
        "Teil-Uebertrag darf kein exit_date setzen"
    );
    assert_eq!(body["to"]["current_shares"], 6); // 1 + new +5
}
```

#### Voll-Uebertrag test (D-17-01/02/03 — exit_date cascade):
```rust
#[tokio::test]
async fn test_transfer_shares_full_with_exit_date_cascade() {
    // setup A with current_shares=2, B with current_shares=1; transfer shares=2.
    // assert body["actions"].len() == 3 (Abgabe, Empfang, Austritt)
    // assert body["from"]["exit_date"] == transfer_date.to_string()
    // assert body["from"]["current_shares"] == 0
}
```

#### Self-Transfer 400 (D-17-08):
```rust
#[tokio::test]
async fn test_transfer_shares_self_transfer_400() {
    // POST /api/members/{a_id}/transfer-shares with body.to_member_id == a_id
    // assert StatusCode::BAD_REQUEST + body contains "cannot transfer to self"
}
```

#### Recipient-cancelled 409 (D-17-07 / PERM-03; analog to `test_partial_repayment_cancelled_member_block_409` lines 746–780):
```rust
#[tokio::test]
async fn test_transfer_shares_recipient_cancelled_409() {
    // 1) create A, B; cancel B (POST /api/members/{b_id}/cancel)
    // 2) POST transfer-shares A->B
    // assert StatusCode::CONFLICT + body contains "recipient already cancelled"
}
```

#### Recipient-not-found 404:
```rust
#[tokio::test]
async fn test_transfer_shares_recipient_not_found_404() {
    // POST with to_member_id = Uuid::new_v4() (never created)
    // assert StatusCode::NOT_FOUND
}
```

#### Audit-Pair-Verify (D-17-05 — Doppel-Assertion):

**Analog:** `test_partial_repayment_audit_chain_verify` (lines 782–832) + `audit/verify` endpoint pattern.

```rust
#[tokio::test]
async fn test_transfer_shares_audit_pair_verify_doppel_assertion() {
    // 1) Trigger transfer (2 shares, partial)
    // 2) GET /api/audit?process=member-adjust.transfer  -> entries-list
    // 3) Assert (a) all entries share ONE transaction_id (atomarity)
    // 4) Assert (b) count of MemberAction entities created == 2 (partial)
    // 5) GET /api/audit/verify -> assert valid == true
}
```

For Voll-Uebertrag-Variant assert count == 3. Helper-Fn `assert_transfer_audit_trail(client, server, process, expected_action_count)` strongly encouraged per CONTEXT D-17-05 specifics.

#### Race-Test (Same-Direction) — CRITICAL pattern from `e2e_tests.rs:12484–12596`

**Direct excerpt** (lines 12511–12565, copy verbatim and adapt):

```rust
// Mini-Sleep um Pool-Connection-Warm-up zu stabilisieren (RESEARCH Pitfall #11).
tokio::time::sleep(std::time::Duration::from_millis(1)).await;

// D-17-06: Beide POSTs parallel via tokio::join! (KEIN sequenzieller await):
let (resp_a, resp_b) = tokio::join!(
    client.post(&url).json(&body).send(),
    client.post(&url).json(&body).send(),
);
let r_a = resp_a.unwrap();
let r_b = resp_b.unwrap();
let status_a = r_a.status();
let status_b = r_b.status();
let body_a = r_a.text().await.unwrap_or_default();
let body_b = r_b.text().await.unwrap_or_default();

let mut statuses = [status_a, status_b];
statuses.sort_by_key(|s| s.as_u16());

assert_eq!(
    statuses[0],
    StatusCode::OK,
    "SC #5 / D-17-06: genau ein Race-Aufruf muss 200 sein; got {:?} (A={:?}, B={:?})",
    statuses, body_a, body_b
);
assert!(
    statuses[1] == StatusCode::CONFLICT || statuses[1] == StatusCode::INTERNAL_SERVER_ERROR,
    "SC #5 / D-17-06: Race-Verlierer muss 409 ODER 500 sein; got {:?}",
    statuses
);
// NIE [200, 200] — waere Double-Cascade.
assert!(
    !(status_a == StatusCode::OK && status_b == StatusCode::OK),
    "SC #5 / D-17-06: NIE [200, 200] (waere Double-Cascade). Got [{}, {}]",
    status_a, status_b
);
```

#### Race-Test (Cross-Direction) — D-17-06 second variant

Cross-direction (A→B + B→A) has DIFFERENT acceptance rules per D-17-06: `[(200, 200), (200, 409|500)]` accepted, `[409|500, 409|500]` NOT accepted (Total-Deadlock). Plus post-check sum-invariant:

```rust
// Both pre-warmup: 1ms sleep (Pool-Warm-up Pitfall #11).
tokio::time::sleep(std::time::Duration::from_millis(1)).await;

let (resp_ab, resp_ba) = tokio::join!(
    client.post(&url_ab).json(&body_a_to_b).send(),
    client.post(&url_ba).json(&body_b_to_a).send(),
);
// ... validate statuses ...

// Post-Check: Anteile-Summe erhalten.
let a_after: Value = client.get(server.url(&format!("/api/members/{}", a_id))).send().await.unwrap().json().await.unwrap();
let b_after: Value = client.get(server.url(&format!("/api/members/{}", b_id))).send().await.unwrap().json().await.unwrap();
let total_after = a_after["current_shares"].as_i64().unwrap() + b_after["current_shares"].as_i64().unwrap();
assert_eq!(total_after, a_start + b_start, "Cross-Race: Anteile-Summe muss erhalten bleiben");

// Audit-Chain bleibt valid.
let verify: VerifyResponseTO = client.get(server.url("/api/audit/verify")).send().await.unwrap().json().await.unwrap();
assert!(verify.valid, "Cross-Race: Audit-Hashchain muss valid bleiben");
```

(Note: `VerifyResponseTO` is in `genossi_rest_types` line 1751.)

## Shared Patterns

### Permission Funnel (D-15-01 / PERM-01)

**Source:** `genossi_service_impl/src/membership_adjust.rs:93–96` (cancel_membership), 184–186 (increase_shares), 302–304 (partial_repayment).
**Apply to:** `transfer_shares` (first op after `use_transaction` + `current_user_id`).

```rust
self.permission_service
    .check_permission(ADMIN_PRIVILEGE, context)
    .await?;
```

### Transaction Lifecycle (Single-Tx-Cascade)

**Source:** `genossi_service_impl/src/membership_adjust.rs:85, 162` (cancel_membership begin + commit); `292, 456` (partial_repayment begin + commit).
**Apply to:** `transfer_shares` (all `audited_*!` calls in between use `tx.clone()`).

```rust
let tx = self.transaction_dao.use_transaction(tx).await?;
// ... all DAO calls with tx.clone() ...
self.transaction_dao.commit(tx).await?;
```

### Audit-Process-String Constant + audited_*! Macro Compliance (AUDT-01 / D-15-02 / D-17-04)

**Source:** `genossi_service_impl/src/membership_adjust.rs:32, 35, 38` (3 existing constants) + audit-macros at `audit_macros.rs:1–80`.
**Apply to:** ALL writes in `transfer_shares` (2 or 3 `audited_create!` + 2 `audited_update!`) MUST use `TRANSFER_PROCESS` for AUDT-02 verlinkung.

### Pure-Function Validation Pattern (D-15-05 / D-17-09)

**Source:** `genossi_service_impl/src/membership_adjust.rs:512–528` (`validate_willensbekundung_date`), 540–568 (`validate_partial_repayment_shares`), 692–741 (test module).
**Apply to:** `validate_transfer_inputs`. Pattern signature `pub(crate) fn ... -> Vec<ValidationFailureItem>` or `-> Result<(), Vec<ValidationFailureItem>>` for early-return. CONTEXT D-17-09 says `Vec<ValidationFailureItem>` (matches `validate_willensbekundung_date`).

### Soft-Delete-Aware Member Loading

**Source:** `genossi_service_impl/src/membership_adjust.rs:106–110, 204–208, 307–311`.
**Apply to:** Both `from` and `to` loads in `transfer_shares`:

```rust
let entity = self.member_dao
    .find_by_id(id, tx.clone())
    .await?
    .ok_or(ServiceError::EntityNotFound(id))?;
```

### Error-Status Mapping (D-17-10)

**Source:** `genossi_rest/src/error.rs` (`From<ServiceError> for RestError`) and the `responses(...)` Utoipa annotations.
**Apply to:** New handler. The mapping is automatic (codebase-wide); planner only needs to list the right status codes in `#[utoipa::path(responses(...))]`.

| ServiceError | RestError | HTTP |
|--------------|-----------|------|
| `Unauthorized` | `Unauthorized` | 401 |
| `PermissionDenied` | `Unauthorized` (codebase-mapping, NOT 403, per D-15-12) | 401 |
| `ValidationError(Vec)` | `BadRequest` | 400 |
| `EntityNotFound(uuid)` | `NotFound` | 404 |
| `Conflict(msg)` | `Conflict` | 409 |
| `DataAccess` (DAO error) | `InternalError` | 500 |

### REST-Sub-Route Ordering (D-14-08 / C-17-CF-06)

**Source:** `genossi_rest/src/member.rs:30–82`.
**Apply to:** Insert `/{from_id}/transfer-shares` AFTER line 77 (`partial-repayment`), BEFORE line 79 (`/{id}` catch-all). The Pitfall-1 comment at lines 30–36 documents the rule.

### ISO8601-Date Serde (C-17-CF-07)

**Source:** `genossi_rest_types/src/lib.rs:84+` (`iso8601_date_required` module), used in lines 514–517, 525–528, 549–552.
**Apply to:** `TransferSharesRequestTO.transfer_date`.

### Race-Test Atomicity Pattern (D-17-06 / Phase-9 D-12)

**Source:** `genossi_bin/tests/e2e_tests.rs:12474–12596`.
**Apply to:** Both same-direction and cross-direction race tests. Key elements:
1. `tokio::time::sleep(Duration::from_millis(1)).await` BEFORE `tokio::join!` (Pool-Warm-up — Pitfall #11).
2. `tokio::join!(...)` for parallel POSTs.
3. Sort statuses: `statuses.sort_by_key(|s| s.as_u16())`.
4. Accept 409 OR 500 as race-loser (both indicate "second tx didn't commit"; 500 covers `SQLITE_BUSY`).
5. NIE-Klauseln (`assert!(!(status_a == OK && status_b == OK))`) — that's the actual D-17-06 guarantee.
6. Post-check `/api/audit/verify` → `valid == true`.

## No Analog Found

None. All Phase-17 work has direct analogs in existing files within the same module.

## Metadata

**Analog search scope:**
- `genossi_service/src/membership_adjust.rs`
- `genossi_service_impl/src/membership_adjust.rs`
- `genossi_service_impl/src/member_action.rs` (recalc_dates, compute_dates)
- `genossi_service_impl/src/audit_macros.rs`
- `genossi_rest_types/src/lib.rs`
- `genossi_rest/src/membership_adjust.rs`
- `genossi_rest/src/member.rs`
- `genossi_bin/tests/membership_adjust_e2e.rs`
- `genossi_bin/tests/e2e_tests.rs:12474–12596` (Phase 9 race pattern)
- `genossi_dao/src/member_action.rs` (ActionType, MemberActionEntity)

**Files scanned:** 10
**Pattern extraction date:** 2026-06-06

## PATTERN MAPPING COMPLETE

**Phase:** 17 - service-rest-uebertrag-cascade
**Files classified:** 6
**Analogs found:** 6 / 6

### Coverage
- Files with exact analog: 6
- Files with role-match analog: 0
- Files with no analog: 0

### Key Patterns Identified
- All Phase-17 work extends existing files in-place (incremental trait growth, D-15-13)
- Cascade-single-tx pattern: `partial_repayment` (Phase 16, 2+ writes) is the closest analog; `cancel_membership` provides the `recalc_dates` hook
- Pure-function validation pattern is canonical: `validate_willensbekundung_date` (re-used) + `validate_partial_repayment_shares` (template for `validate_transfer_inputs`)
- Race-test pattern lives in `e2e_tests.rs:12474–12596` (Phase-9-D-12) and must be copied into the new `membership_adjust_e2e.rs` race tests
- E2E test infrastructure (`setup()`, `create_active_member`, `put_member_current_shares`, `today_march_15`) is fully re-usable from `membership_adjust_e2e.rs` Phase-15/16 helpers

### File Created
`.planning/phases/17-service-rest-uebertrag-cascade/17-PATTERNS.md`

### Ready for Planning
Pattern mapping complete. Planner can now reference exact lines + excerpts in PLAN.md files.
