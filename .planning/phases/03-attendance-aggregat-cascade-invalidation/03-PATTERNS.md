# Phase 3: Attendance-Aggregat + Cascade-Invalidation - Pattern Map

**Mapped:** 2026-05-03
**Files analyzed:** 14 (7 NEW + 7 MODIFIED)
**Analogs found:** 14 / 14

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| **NEW** `migrations/sqlite/20260504000000_create_attendance_table.sql` | migration | schema-DDL | `migrations/sqlite/20260502000001_create_assembly_member_snapshot_table.sql` | exact (lightweight join-table) |
| **NEW** `genossi_dao/src/attendance.rs` | DAO trait + entity | CRUD + UPSERT | `genossi_dao/src/assembly_member_snapshot.rs` (lightweight) + `genossi_dao/src/helper_token.rs` (mutating + automock) | exact-blend |
| **NEW** `genossi_dao_impl_sqlite/src/attendance.rs` | DAO impl | UPSERT + soft-delete + LEFT JOIN | `genossi_dao_impl_sqlite/src/helper_token.rs` (sqlx::query bind pattern, atomic_redeem RETURNING) | exact |
| **NEW** `genossi_service/src/attendance.rs` | service trait + domain types | request-response | `genossi_service/src/assembly.rs` (trait + Assembly/AssemblyDetail domain types) | exact |
| **NEW** `genossi_service_impl/src/attendance.rs` | service impl | permission-funnel + CRUD | `genossi_service_impl/src/assembly.rs` (gen_service_impl macro + audited_update) + `genossi_service_impl/src/permission.rs:28-48` (check_permission match-shape) | role-match |
| **NEW** `genossi_rest/src/attendance.rs` | REST controller | request-response | `genossi_rest/src/assembly.rs` (RestState trait, generate_route, ApiDoc) + `genossi_rest/src/helper_token.rs:280-348` (differential error mapping) | exact-blend |
| **NEW** TOs in `genossi_rest_types/src/lib.rs` (`AttendanceMemberTO`, `AttendanceStatsTO`) | DTO | serialization | `genossi_rest_types/src/lib.rs:1037-1110` (`AssemblyTO` + `AssemblyDetailTO` `From<&...>` pattern) | role-match (no datetime fields) |
| **MODIFY** `genossi_dao/src/helper_token.rs` (+ `list_session_ids_for_assembly`) | DAO trait extension | read-only list | `genossi_dao/src/helper_token.rs:169-173` (`all_for_assembly` already exists, same shape) | exact |
| **MODIFY** `genossi_dao_impl_sqlite/src/helper_token.rs` (+ impl) | DAO impl extension | SELECT WHERE | `genossi_dao_impl_sqlite/src/helper_token.rs:290-307` (`all_for_assembly` impl) | exact |
| **MODIFY** `genossi_service_impl/src/assembly.rs:50-60, 254-304` (Cascade) | service extension | mutating + cascade | itself — lines 254-304 are the existing `close_assembly` body | exact (extension of self) |
| **MODIFY** `genossi_service/src/claim_context.rs` (+ `as_helper`) | trait extension | predicate | `genossi_service/src/claim_context.rs:1-29` (`has_claims` default-impl pattern) | exact |
| **MODIFY** `genossi_rest/src/lib.rs` (route nest + ApiDoc) | router config | configuration | `genossi_rest/src/lib.rs:266-267, 549-552` (existing `nest` + `generate_route()` pattern for assembly + helper_token) | exact |
| **MODIFY** `genossi_bin/src/lib.rs` (DI wiring + RestState impl) | composition root | DI | `genossi_bin/src/lib.rs:596-604` (AssemblyServiceImpl wiring) + `1156-1162` (`AssemblyRestState` impl) | exact |
| **MODIFY** `genossi_bin/tests/e2e_tests.rs` (5 new test cases) | E2E test | full HTTP | `e2e_tests.rs:8781-8819` (HLPR-04 race) + `9171-9279` (HLPR-07 audit verify) + `3447-3465` (`setup_with_pool` for direct DB queries) | exact |

---

## Pattern Assignments

### `migrations/sqlite/20260504000000_create_attendance_table.sql` (migration, schema-DDL)

**Analog:** `migrations/sqlite/20260502000001_create_assembly_member_snapshot_table.sql`
**Plus contrast:** `migrations/sqlite/20260503000000_create_helper_token_table.sql` (FK ON DELETE RESTRICT pattern)

**WR-03 FK-Note pattern** (snapshot migration lines 1-11) — copy verbatim into new migration header to document FK-enforcement caveat:
```sql
-- NOTE (WR-03): FOREIGN KEY clauses below are DOCUMENTARY only.
-- This codebase does not enable `PRAGMA foreign_keys=ON` ...
```

**Composite-PK pattern** (lines 12-19 of the snapshot migration):
```sql
CREATE TABLE IF NOT EXISTS assembly_member_snapshot (
    assembly_id BLOB NOT NULL,
    member_id BLOB NOT NULL,
    captured_at TEXT NOT NULL,
    PRIMARY KEY (assembly_id, member_id),
    FOREIGN KEY (assembly_id) REFERENCES assembly(id),
    FOREIGN KEY (member_id) REFERENCES member(id)
);

CREATE INDEX IF NOT EXISTS idx_assembly_member_snapshot_assembly_id
    ON assembly_member_snapshot(assembly_id);
```

**FK ON DELETE RESTRICT pattern** (helper_token migration line 22):
```sql
FOREIGN KEY (assembly_id) REFERENCES assembly(id) ON DELETE RESTRICT,
```

**Apply to attendance table:**
- Composite PK `(assembly_id, member_id)` (D-04, automatically UNIQUE — required for `ON CONFLICT(...)`-target).
- FK ON DELETE RESTRICT for both `assembly_id` and `member_id` (Discretion #2 in RESEARCH).
- Add `marked_at TEXT NOT NULL`, `marked_by_user_id TEXT NOT NULL`, `deleted TEXT` columns (D-01).
- Optional partial index `CREATE INDEX ... ON attendance(assembly_id) WHERE deleted IS NULL` for `count_present_by_assembly` perf (RESEARCH §Discretion-Auflösung 1).

---

### `genossi_dao/src/attendance.rs` (DAO trait + entity, CRUD + UPSERT)

**Analog:** `genossi_dao/src/assembly_member_snapshot.rs` (entity shape) blended with `genossi_dao/src/helper_token.rs` (trait shape with `#[automock]` + custom mutating methods)

**Imports pattern** (assembly_member_snapshot.rs lines 1-6):
```rust
use async_trait::async_trait;
use mockall::automock;
use std::sync::Arc;
use uuid::Uuid;

use crate::DaoError;
```

**Entity-without-id pattern** (assembly_member_snapshot.rs lines 8-13):
```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssemblyMemberSnapshotEntity {
    pub assembly_id: Uuid,
    pub member_id: Uuid,
    pub captured_at: time::PrimitiveDateTime,
}
```
Apply: `AttendanceEntity` adds `marked_at`, `marked_by_user_id: Arc<str>`, `deleted: Option<PrimitiveDateTime>` (D-01). NO `id`/`version`. NO `Auditable` impl (D-08).

**Trait + automock pattern** (assembly_member_snapshot.rs lines 15-45):
```rust
#[automock(type Transaction = crate::MockTransaction;)]
#[async_trait]
pub trait AssemblyMemberSnapshotDao {
    type Transaction: crate::Transaction;

    async fn create(
        &self,
        entity: &AssemblyMemberSnapshotEntity,
        process: &str,
        tx: Self::Transaction,
    ) -> Result<(), DaoError>;
    // ...
}
```

**Custom mutating method shape (no Auditable, no version)** — adopt from `helper_token.rs:133-148` (`atomic_redeem`/`set_session_id` show the right shape for non-CRUD methods that don't go through `audited_*!` macros):
```rust
async fn atomic_redeem(
    &self,
    token_hash: &str,
    used_at: time::PrimitiveDateTime,
    tx: Self::Transaction,
) -> Result<Option<(Uuid, Uuid)>, DaoError>;
```

**Apply to AttendanceDao** — five trait methods (RESEARCH §Konkrete Code-Recommendations lines 793-840):
- `upsert_present(assembly_id, member_id, marked_at, marked_by_user_id, tx) -> Result<(), DaoError>` (D-05)
- `soft_delete(assembly_id, member_id, deleted_at, tx) -> Result<(), DaoError>` (D-06)
- `list_members_for_assembly(assembly_id, search: Option<&str>, tx) -> Result<Arc<[AttendanceMemberRow]>, DaoError>` (D-25)
- `count_present_by_assembly(assembly_id, tx) -> Result<u64, DaoError>` (ASSY-04)
- `is_in_snapshot(assembly_id, member_id, tx) -> Result<bool, DaoError>` (D-27)

**Smoke-test pattern** (assembly_member_snapshot.rs lines 47-66):
```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_snapshot_entity_has_three_fields_only() {
        // Construct + assert one field — compile-time contract.
    }
}
```

---

### `genossi_dao_impl_sqlite/src/attendance.rs` (DAO impl, UPSERT + soft-delete + LEFT JOIN)

**Analog:** `genossi_dao_impl_sqlite/src/helper_token.rs`

**Format-datetime helper pattern** (helper_token.rs uses module-level `format_dt`; RESEARCH §Konkrete Code-Recommendations line 857):
```rust
fn format_dt(dt: &PrimitiveDateTime) -> Result<String, DaoError> {
    let format = &time::format_description::well_known::Iso8601::DEFAULT;
    dt.assume_utc()
        .format(format)
        .map_err(|e| DaoError::ParseError(Arc::from(e.to_string())))
}
```

**SQLx-bind pattern (no compile-time macros — Pitfall 2)** (helper_token.rs lines 125-142):
```rust
sqlx::query(
    "INSERT INTO helper_token (id, assembly_id, memo, ...) \
     VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
)
.bind(id)
.bind(assembly_id)
// ...
.execute(tx.tx.lock().await.as_mut())
.await
.map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?;
```

Apply to UPSERT body (D-05; RESEARCH §Pattern 1):
```sql
INSERT INTO attendance (assembly_id, member_id, marked_at, marked_by_user_id, deleted)
VALUES (?, ?, ?, ?, NULL)
ON CONFLICT(assembly_id, member_id) DO UPDATE SET
    marked_at = excluded.marked_at,
    marked_by_user_id = excluded.marked_by_user_id,
    deleted = NULL
```

**RETURNING pattern (NOT used by attendance — Pitfall 1)** — for reference, helper_token.rs:206-236 shows `query_as::<_, RowType>(...).fetch_optional(...)`. Phase 3 deliberately avoids RETURNING.

**JOIN + LEFT JOIN + LIKE-with-NULL-marker pattern** (verbatim SQL in RESEARCH §Pattern 3, lines 322-346):
```sql
SELECT m.id, m.member_number, m.first_name, m.last_name, m.salutation, m.title,
       CASE WHEN a.assembly_id IS NOT NULL AND a.deleted IS NULL THEN 1 ELSE 0 END AS is_present
FROM assembly_member_snapshot s
JOIN member m ON m.id = s.member_id AND m.deleted IS NULL
LEFT JOIN attendance a ON a.assembly_id = s.assembly_id AND a.member_id = m.id
WHERE s.assembly_id = ?
  AND ( ? IS NULL
        OR (m.last_name || ' ' || m.first_name) LIKE ? COLLATE NOCASE
        OR CAST(m.member_number AS TEXT) LIKE ?
      )
ORDER BY m.last_name COLLATE NOCASE, m.first_name COLLATE NOCASE
```

**`query_scalar` pattern for COUNT** — adopt from helper_token.rs:167-173:
```rust
let exists = sqlx::query_scalar::<_, i32>(
    "SELECT COUNT(*) FROM helper_token WHERE id = ? AND deleted IS NULL",
)
.bind(id.clone())
.fetch_one(tx.tx.lock().await.as_mut())
.await
.map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?;
```

**Idempotent UPDATE-no-error pattern (D-06)** — `soft_delete` ignores `rows_affected()` (anders als helper_token.rs:252-254 wo NotFound returned wird).

---

### `genossi_service/src/attendance.rs` (service trait + domain types, request-response)

**Analog:** `genossi_service/src/assembly.rs`

**Imports pattern** (assembly.rs lines 9-17):
```rust
use async_trait::async_trait;
use genossi_dao::assembly::{AssemblyEntity, AssemblyStatus};
use mockall::automock;
use std::fmt::Debug;
use std::sync::Arc;
use uuid::Uuid;

use crate::permission::Authentication;
use crate::ServiceError;
```

**Domain-type-with-Arc<str>-fields pattern** (assembly.rs lines 23-35):
```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Assembly {
    pub id: Uuid,
    pub name: Arc<str>,
    // ...
}
```
Apply: `AttendanceMember` and `AttendanceStats` as service-layer domain types. Or — given D-23/D-24 — re-use `AttendanceMemberRow` from DAO directly (fewer types, since service doesn't transform fields, just permission-gates + DAO-passthrough).

**Trait with #[automock] + Context+Transaction associated types** (assembly.rs lines 101-154):
```rust
#[automock(type Context=(); type Transaction = genossi_dao::MockTransaction;)]
#[async_trait]
pub trait AssemblyService {
    type Context: Clone + Debug + PartialEq + Eq + Send + Sync + 'static;
    type Transaction: genossi_dao::Transaction;

    async fn close_assembly(
        &self,
        id: Uuid,
        context: Authentication<Self::Context>,
    ) -> Result<Assembly, ServiceError>;
    // ...
}
```

**Apply to AttendanceService** — 4 methods per D-22:
```rust
async fn list_members(&self, assembly_id: Uuid, search: Option<String>,
                      context: Authentication<Self::Context>) -> Result<Arc<[AttendanceMember]>, ServiceError>;
async fn mark_present(&self, assembly_id: Uuid, member_id: Uuid,
                      context: Authentication<Self::Context>) -> Result<(), ServiceError>;
async fn mark_absent(&self, assembly_id: Uuid, member_id: Uuid,
                     context: Authentication<Self::Context>) -> Result<(), ServiceError>;
async fn stats(&self, assembly_id: Uuid,
               context: Authentication<Self::Context>) -> Result<AttendanceStats, ServiceError>;
```

---

### `genossi_service_impl/src/attendance.rs` (service impl, permission-funnel + CRUD)

**Analog:** `genossi_service_impl/src/assembly.rs` (gen_service_impl macro + service-method shape) + `genossi_service_impl/src/permission.rs:28-48` (check_permission match-shape for `check_assembly_access`)

**Module-level constants** (assembly.rs lines 44-48):
```rust
const ASSEMBLY_PROCESS_CREATE: &str = "assembly.create";
// ...
const ADMIN_PRIVILEGE: &str = "admin";
```
Apply: NO process constants for attendance (D-08 — no audit). Only `const ADMIN_PRIVILEGE: &str = "admin";` (D-19). Optionally re-use `genossi_service::permission::ADMIN_PRIVILEGE`.

**`gen_service_impl!` skeleton** (assembly.rs lines 50-60):
```rust
gen_service_impl! {
    struct AssemblyServiceImpl: AssemblyService = AssemblyServiceDeps {
        AssemblyDao: AssemblyDao<Transaction = Self::Transaction> = assembly_dao,
        AssemblyMemberSnapshotDao: AssemblyMemberSnapshotDao<Transaction = Self::Transaction> = assembly_member_snapshot_dao,
        MemberDao: MemberDao<Transaction = Self::Transaction> = member_dao,
        AuditLogDao: AuditLogDao<Transaction = Self::Transaction> = audit_log_dao,
        PermissionService: PermissionService<Context = Self::Context> = permission_service,
        UuidService: UuidService = uuid_service,
        TransactionDao: TransactionDao<Transaction = Self::Transaction> = transaction_dao,
    }
}
```

**Apply to AttendanceServiceImpl** (D-23 — six deps, NO `UuidService`, NO `AuditLogDao`):
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

**Service-method body skeleton** (assembly.rs `close_assembly` lines 254-304):
```rust
async fn close_assembly(
    &self,
    id: Uuid,
    context: Authentication<Self::Context>,
) -> Result<Assembly, ServiceError> {
    let tx = self.transaction_dao.use_transaction(None).await?;

    let user_id = self.permission_service.current_user_id(context.clone()).await?
        .unwrap_or_else(|| "SYSTEM".to_string());
    self.permission_service.check_permission(ADMIN_PRIVILEGE, context).await?;

    let mut entity = self.assembly_dao.find_by_id(id, tx.clone()).await?
        .ok_or(ServiceError::EntityNotFound(id))?;

    if entity.status != AssemblyStatus::Open {
        return Err(ServiceError::Conflict(Arc::from(format!(...))));
    }
    // ...
    self.transaction_dao.commit(tx).await?;
    Ok(Assembly::from(&entity))
}
```

**Permission-Funnel pattern** (permission.rs lines 28-48, the match-shape on `Authentication<...>`):
```rust
async fn check_permission(
    &self,
    privilege: &str,
    context: Authentication<Self::Context>,
) -> Result<(), ServiceError> {
    match context {
        Authentication::Full => Ok(()),
        Authentication::Context(ctx) => {
            let current_user = self.user_service.current_user(ctx).await?;
            if self.permission_dao.has_privilege(&current_user, privilege).await? {
                Ok(())
            } else {
                Err(ServiceError::PermissionDenied)
            }
        }
    }
}
```

**Apply to `check_assembly_access`** (D-17/D-18; RESEARCH §Pattern 6):
- Same match-shape on `Authentication<Self::Context>`.
- For `Authentication::Context(ctx)`: call `ctx.as_helper()` (NEW trait method on `ClaimContext`, see below) → if `Some((_, helper_aid))`, do match-vs-endpoint-aid + assembly.status==Open check; if `None`, fall through to `permission_service.check_permission(ADMIN_PRIVILEGE, context)`.
- Returns `Result<AssemblyEntity, ServiceError>` so callers don't re-load assembly (RESEARCH §Konkrete Code-Recommendations lines 397-445).

**Service-impl tests pattern** (assembly.rs lines 350-700+) — handgeschriebene `mock! { pub TestXxxDao { ... } }` blocks gegen `TestTransaction`. Beispiel für TestAssemblyDao (assembly.rs lines 397-408):
```rust
mock! {
    pub TestAssemblyDao {}
    #[async_trait]
    impl AssemblyDao for TestAssemblyDao {
        type Transaction = TestTransaction;
        async fn dump_all(&self, tx: TestTransaction) -> Result<Arc<[AssemblyEntity]>, DaoError>;
        async fn create(&self, entity: &AssemblyEntity, process: &str, tx: TestTransaction) -> Result<(), DaoError>;
        async fn update(&self, entity: &AssemblyEntity, process: &str, tx: TestTransaction) -> Result<(), DaoError>;
        async fn all(&self, tx: TestTransaction) -> Result<Arc<[AssemblyEntity]>, DaoError>;
        async fn find_by_id(&self, id: Uuid, tx: TestTransaction) -> Result<Option<AssemblyEntity>, DaoError>;
    }
}
```
Apply: AttendanceServiceImpl unit-tests need `TestAttendanceDao` + reuse-from-assembly-tests `TestAssemblyDao`/`TestSnapshotDao`/`TestMemberDao`/`TestPermissionService`/`TestTxDao`. RESEARCH §Pitfall 4 documents handwritten-mock necessity.

---

### `genossi_rest/src/attendance.rs` (REST controller, request-response)

**Analog:** `genossi_rest/src/assembly.rs` (router-shape, RestState trait, ApiDoc) + `genossi_rest/src/helper_token.rs:280-348` (differential error mapping)

**Imports pattern** (assembly.rs lines 1-18):
```rust
use axum::{
    body::Body,
    extract::{Path, State},
    response::Response,
    routing::{get, post},
    Extension, Json, Router,
};
use genossi_rest_types::{...};
use std::sync::Arc;
use tracing::instrument;
use utoipa::OpenApi;
use uuid::Uuid;

use crate::{error_handler, Context, RestError, RestStateDef};
```

**RestState-trait pattern** (assembly.rs lines 20-24):
```rust
pub trait AssemblyRestState: Clone + Send + Sync + 'static {
    type AssemblyService: AssemblyService<Context = crate::ContextType> + Send + Sync + 'static;

    fn assembly_service(&self) -> Arc<Self::AssemblyService>;
}
```

**Handler pattern with `#[instrument]` + `#[utoipa::path]` + `error_handler` wrapper** (assembly.rs lines 313-347, `close_assembly`):
```rust
#[instrument(skip(rest_state))]
#[utoipa::path(
    post,
    tag = "Assemblies",
    path = "/{id}/close",
    params(("id" = Uuid, Path, description = "Assembly ID")),
    responses(
        (status = 200, description = "Closed", body = AssemblyTO),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Not found"),
        (status = 409, description = "Conflict (status not Open)"),
    ),
)]
pub async fn close_assembly<RestState: RestStateDef + AssemblyRestState>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path(id): Path<Uuid>,
) -> Response {
    error_handler(
        (async {
            let auth = crate::extract_auth_context(Some(context))?;
            let assembly = rest_state.assembly_service().close_assembly(id, auth).await?;
            // ...
        }).await,
    )
}
```

**Path-with-tuple-extraction pattern** (NOT shown in assembly.rs — invent from idiom): `Path((assembly_id, member_id)): Path<(Uuid, Uuid)>` for `PUT /api/attendance/{aid}/{mid}`. Reference RESEARCH §Pattern 7 lines 459-498.

**Differential ServiceError → RestError mapping pattern** (helper_token.rs lines 287-313, the redeem-helper-token handler):
```rust
let result = rest_state.helper_token_service().redeem_helper_token(&body.code).await;

let success = match result {
    Ok(s) => s,
    Err(ServiceError::ValidationError(_)) => return Err(RestError::BadRequest(...)),
    Err(ServiceError::EntityNotFound(_)) => return Err(RestError::NotFound),
    Err(ServiceError::Conflict(payload)) => {
        let p = payload.as_ref();
        if p == "already_used" { return Err(RestError::Gone(...)); }
        else if p == "revoked" || p == "assembly_not_open" { return Err(RestError::Forbidden(...)); }
        else { return Err(RestError::Conflict(p.to_string())); }
    }
    Err(other) => return Err(other.into()),
};
```

**Apply to attendance** — extract a small helper `fn map_attendance_error(e: ServiceError) -> RestError` per Conflict 1 in RESEARCH (D-26 wants 403 for `PermissionDenied`, but global mapping in `lib.rs:106` returns 401):
```rust
fn map_attendance_error(e: ServiceError) -> RestError {
    match e {
        ServiceError::PermissionDenied => RestError::Forbidden("forbidden".to_string()),
        other => other.into(),
    }
}
```
**RestError::Forbidden already exists at `genossi_rest/src/lib.rs:84`** — no new variant needed.

**Router-Builder pattern** (assembly.rs lines 349-361):
```rust
pub fn generate_route<RestState: RestStateDef + AssemblyRestState>() -> Router<RestState> {
    Router::new()
        .route("/", get(list_assemblies::<RestState>).post(create_assembly::<RestState>))
        .route("/{id}", get(get_assembly::<RestState>).put(update_assembly::<RestState>))
        .route("/{id}/open", post(open_assembly::<RestState>))
        .route("/{id}/close", post(close_assembly::<RestState>))
}
```

**Apply to attendance** — note that `/api/assembly/{aid}/stats` (D-21) lebt unter assembly-namespace (not attendance). Two options:
- (a) Two `generate_route()` functions: `attendance::generate_attendance_route()` for `/api/attendance/...` and `attendance::generate_stats_route()` for `/api/assembly/{aid}/stats`.
- (b) One mounted at `/api/attendance/...` and a single inline `.route(...)` registration of the stats handler in `genossi_rest/src/lib.rs` Router.

**ApiDoc pattern** (assembly.rs lines 363-381):
```rust
#[derive(OpenApi)]
#[openapi(
    paths(list_assemblies, create_assembly, get_assembly, update_assembly, open_assembly, close_assembly),
    components(schemas(AssemblyTO, AssemblyStatusTO, AssemblyDetailTO, CreateAssemblyRequest, UpdateAssemblyRequest))
)]
pub struct ApiDoc;
```

**Validation-helper pattern** (assembly.rs lines 28-103) — Phase 3 endpoints take ZERO validated bodies (PUT/DELETE with empty body, GET with optional query). Validation overhead minimal. Could skip dedicated `validate_*` functions; RestError::BadRequest only for malformed query params.

---

### TOs in `genossi_rest_types/src/lib.rs` (DTO, serialization)

**Analog:** `AssemblyTO` + `AssemblyDetailTO` patterns (lines 1037-1110)

**TO-with-no-datetime pattern** — RESEARCH §Pitfall 5 stresses Phase 3 has NO `PrimitiveDateTime` fields in TOs. Don't copy `AssemblyTO`'s elaborate `#[serde(with = "iso8601_datetime")]` block.

**`From<&Service-Type>`-Pattern** (lib.rs:1080-1095):
```rust
impl From<&genossi_service::assembly::Assembly> for AssemblyTO {
    fn from(a: &genossi_service::assembly::Assembly) -> Self {
        Self {
            id: a.id,
            name: a.name.to_string(),
            // ...
        }
    }
}
```

**Apply to AttendanceMemberTO** (RESEARCH §Konkrete Code-Recommendations lines 1226-1265):
- Convert from `&genossi_dao::attendance::AttendanceMemberRow` (NOT from `MemberTO` — Pitfall 6 explicit verbot).
- 7 fields exactly: `member_number: i64`, `first_name: String`, `last_name: String`, `salutation: Option<String>`, `title: Option<String>`, `is_present: bool`, `member_id: Uuid`.
- Use `#[derive(Serialize, Deserialize, ToSchema)]` (same as `AssemblyTO`).
- `#[serde(skip_serializing_if = "Option::is_none", default)]` for optional fields.

**`AttendanceStatsTO`** — minimal struct: `present: u64, total: u64`.

---

### `genossi_dao/src/helper_token.rs` (MODIFY: + `list_session_ids_for_assembly`)

**Analog within same file:** `all_for_assembly` (lines 169-173):
```rust
async fn all_for_assembly(
    &self,
    assembly_id: Uuid,
    tx: Self::Transaction,
) -> Result<Arc<[HelperTokenEntity]>, DaoError>;
```

**Apply** — add a new trait method (RESEARCH §Konkrete Code-Recommendations lines 1023-1033):
```rust
/// Cascade-Discovery for AssemblyServiceImpl::close_assembly (Phase 3 D-12).
async fn list_session_ids_for_assembly(
    &self,
    assembly_id: Uuid,
    tx: Self::Transaction,
) -> Result<Vec<Arc<str>>, DaoError>;
```

**Pitfall 4 — handwritten-mock impact:** `MockHelperTokenDao` (auto-generated by `#[automock]`) gets the new method automatically. But the handwritten `mock! { TestHelperTokenDao { ... } }` in `genossi_service_impl/src/assembly.rs:tests` (Plan 3 modifications) MUST list the new method (RESEARCH §Pitfall 4 lines 1311-1325).

---

### `genossi_dao_impl_sqlite/src/helper_token.rs` (MODIFY: + impl)

**Analog within same file:** `all_for_assembly` impl (lines 290-307):
```rust
async fn all_for_assembly(
    &self,
    assembly_id: Uuid,
    tx: Self::Transaction,
) -> Result<Arc<[HelperTokenEntity]>, DaoError> {
    let aid = assembly_id.as_bytes().to_vec();
    let rows = sqlx::query_as::<_, HelperTokenDb>(
        "SELECT id, assembly_id, memo, ... FROM helper_token \
         WHERE assembly_id = ? AND deleted IS NULL ORDER BY created DESC",
    )
    .bind(aid)
    .fetch_all(tx.tx.lock().await.as_mut())
    .await
    .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?;
    // ...
}
```

**Apply** — RESEARCH §Konkrete Code-Recommendations lines 1037-1054:
```rust
async fn list_session_ids_for_assembly(
    &self,
    assembly_id: Uuid,
    tx: Self::Transaction,
) -> Result<Vec<Arc<str>>, DaoError> {
    let aid = assembly_id.as_bytes().to_vec();
    let rows: Vec<String> = sqlx::query_scalar(
        "SELECT session_id FROM helper_token \
         WHERE assembly_id = ? AND session_id IS NOT NULL AND deleted IS NULL",
    )
    .bind(aid)
    .fetch_all(tx.tx.lock().await.as_mut())
    .await
    .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?;
    Ok(rows.into_iter().map(|s| Arc::from(s.as_str())).collect())
}
```

---

### `genossi_service_impl/src/assembly.rs:50-60, 254-304` (MODIFY: Cascade extension)

**Analog:** itself — extend the existing `gen_service_impl!` block + extend the existing `close_assembly` body.

**Existing `gen_service_impl!` block** (lines 50-60) — add 2 deps:
```rust
gen_service_impl! {
    struct AssemblyServiceImpl: AssemblyService = AssemblyServiceDeps {
        // ... existing 7 deps ...
        // NEU Phase 3 (D-16):
        HelperTokenDao: HelperTokenDao<Transaction = Self::Transaction> = helper_token_dao,
        // NEU Phase 3 (Cascade calls delete_session, on PermissionDao):
        PermissionDao: PermissionDao = permission_dao,
    }
}
```

**Existing `close_assembly` body** (lines 254-304) — extend after `audited_update!` block (line 290-298). Apply per RESEARCH §Pattern 5 + DECISION CONFLICT 2 (commit BEFORE pool-based delete_session calls):
```rust
crate::audited_update!(self, self.assembly_dao, id, &entity, ASSEMBLY_PROCESS_CLOSE, &user_id, tx);

// D-11/D-12: Phase 3 cascade extension.
let session_ids = self.helper_token_dao
    .list_session_ids_for_assembly(id, tx.clone())
    .await?;

// CONFLICT 2 resolution: commit BEFORE delete_session (pool-based, would deadlock).
self.transaction_dao.commit(tx).await?;

// D-13/D-14: Continue-on-Error.
for sid in session_ids.iter() {
    if let Err(e) = self.permission_dao.delete_session(sid.as_ref()).await {
        tracing::warn!(error=?e, session_id=%sid, assembly_id=%id, "cascade delete_session failed");
    }
}

Ok(Assembly::from(&entity))
```

**`PermissionDao::delete_session` signature** (genossi_dao/src/permission.rs:90):
```rust
async fn delete_session(&self, session_id: &str) -> Result<(), DaoError>;
```
NO `tx` argument — pool-based (RESEARCH §Pattern 5 + Conflict 2).

**Pool-vs-TX deadlock-precedent:** `genossi_service_impl/src/helper_token.rs:316-325` documents the same pattern (commit redeem-tx BEFORE create_session-pool-call).

**Test extension** — Plan 3 must:
- Add `TestHelperTokenDao` mock-block to assembly.rs:tests (analog to existing `TestAssemblyDao` lines 397-408).
- Add `TestPermissionDao` mock-block.
- Wire both into `TestDeps` impl.
- Existing test `test_close_assembly_from_preparation_returns_conflict` (assembly.rs:838-859) bleibt grün — short-circuit BEVOR cascade reached.

---

### `genossi_service/src/claim_context.rs` (MODIFY: + `as_helper`)

**Analog within same file:** existing `has_claims` pattern (lines 1-29).

**Existing pattern:**
```rust
pub trait ClaimContext {
    fn has_claims(&self) -> bool;
}

impl ClaimContext for crate::auth_types::AuthenticatedContext {
    fn has_claims(&self) -> bool { claim_utils::has_claims(self) }
}

impl ClaimContext for crate::permission::MockContext {
    fn has_claims(&self) -> bool { false }
}

impl ClaimContext for () {
    fn has_claims(&self) -> bool { false }
}
```

**Apply** — RESEARCH §Open Question 1 + Recommendation:
```rust
pub trait ClaimContext {
    fn has_claims(&self) -> bool;

    /// Helper-discrimination: returns `Some((session_id, assembly_id))` if this
    /// context represents a redeemed Helfer-Token, else None.
    /// Default: None (mock_auth + automock contexts never carry helper claims).
    fn as_helper(&self) -> Option<(std::sync::Arc<str>, uuid::Uuid)> { None }
}
```

- `MockContext::as_helper()` → keep default `None` (RESEARCH §Pitfall: mock_auth helper-discrimination via cookie-format, not context).
- `AuthenticatedContext::as_helper()` → parse `self.claims` JSON for `kind == "helper"`, extract `session_id` (= context user_id) + `assembly_id` from claims (Phase-2-D-16-format).
- `()` → keep default `None`.

**Default-impl-keeps-tests-green:** A6 in RESEARCH Assumptions Log — adding a default-impl method to a trait does not break existing implementors.

---

### `genossi_rest/src/lib.rs` (MODIFY: route nest + ApiDoc)

**Analog within same file:** existing nest patterns (lines 266-269 for ApiDoc, 549-552 for Router).

**ApiDoc nest pattern** (lib.rs:266-267):
```rust
(path = "/api/assembly", api = assembly::ApiDoc),
(path = "/api/assembly/{assembly_id}/helper-tokens", api = helper_token::ApiDoc),
```
Apply (Phase 3 — add 2 entries):
```rust
(path = "/api/attendance/{assembly_id}", api = attendance::ApiDoc),
// stats lives under /api/assembly namespace per D-21 — either:
//   (a) extend assembly::ApiDoc to register the stats handler, OR
//   (b) create attendance::StatsApiDoc separately for /api/assembly/{aid}/stats
```

**Router nest pattern** (lib.rs:549-552):
```rust
.nest("/api/assembly", assembly::generate_route())
```
Apply:
```rust
.nest("/api/attendance/{assembly_id}", attendance::generate_route())
// And for stats (depending on (a) vs (b) above) — either inline route or separate nest.
```

**Auth-Middleware applies automatically** — `nest("/api/...", ...)` inherits the `context_extractor` middleware (lib.rs:345-358). Endpoints with `Extension<Context>` get the Phase-2-Helper-or-OIDC-Context populated by the middleware chain.

---

### `genossi_bin/src/lib.rs` (MODIFY: DI wiring + RestState impl)

**Analog within same file:** AssemblyServiceImpl wiring (lines 596-604) + AssemblyRestState impl (lines 1156-1162).

**Service-Wiring pattern** (lib.rs:596-604):
```rust
let assembly_member_snapshot_dao = Arc::new(AssemblyMemberSnapshotDao::new(pool.clone()));
let assembly_service = Arc::new(genossi_service_impl::assembly::AssemblyServiceImpl {
    assembly_dao: assembly_dao.clone(),
    assembly_member_snapshot_dao,
    member_dao: member_dao.clone(),
    audit_log_dao: audit_log_dao.clone(),
    permission_service: permission_service.clone(),
    uuid_service: uuid_service.clone(),
    transaction_dao: transaction_dao.clone(),
});
```

**Apply for AttendanceServiceImpl** (after assembly_service, before helper_token_service since Phase 3 extends helper_token-related deps):
```rust
let attendance_dao = Arc::new(AttendanceDaoImpl::new(pool.clone()));
let attendance_service = Arc::new(genossi_service_impl::attendance::AttendanceServiceImpl {
    attendance_dao,
    assembly_dao: assembly_dao.clone(),
    member_dao: member_dao.clone(),
    assembly_member_snapshot_dao: assembly_member_snapshot_dao.clone(),
    permission_service: permission_service.clone(),
    transaction_dao: transaction_dao.clone(),
});
```
Note: `assembly_member_snapshot_dao` currently moved into `assembly_service` (line 598) — Plan must `.clone()` it before that move.

**Cascade-extension for AssemblyServiceImpl** — modify the existing assembly_service constructor (line 596-604) to add `helper_token_dao` and `permission_dao` fields (already constructed in lib.rs lines 422-426 for permission_dao, 610 for helper_token_dao — but the latter is constructed AFTER assembly_service, so order of declarations needs swapping).

**Type-alias pattern** (lib.rs:170-171, 193-194, 247-248):
```rust
type AssemblyService =
    genossi_service_impl::assembly::AssemblyServiceImpl<AssemblyServiceDependencies>;
```

**Deps-Type pattern** (lib.rs:147-168 for AssemblyServiceDependencies — see existing struct + impl):
```rust
pub struct AssemblyServiceDependencies;
unsafe impl Send for AssemblyServiceDependencies {}
unsafe impl Sync for AssemblyServiceDependencies {}

impl genossi_service_impl::assembly::AssemblyServiceDeps for AssemblyServiceDependencies {
    type Context = Context;
    type Transaction = Transaction;
    type AssemblyDao = AssemblyDao;
    // ... add HelperTokenDao + PermissionDao for Phase 3
}
```

**RestState impl pattern** (lib.rs:1156-1162):
```rust
impl genossi_rest::assembly::AssemblyRestState for RestStateImpl {
    type AssemblyService = AssemblyService;
    fn assembly_service(&self) -> Arc<Self::AssemblyService> {
        self.assembly_service.clone()
    }
}
```

**Apply for AttendanceRestState** — add to `RestStateImpl` struct (line 376-416) the field `attendance_service: Arc<AttendanceService>`, add to `Self { ... }` (line 680-718), then implement `AttendanceRestState` trait at the bottom.

---

### `genossi_bin/tests/e2e_tests.rs` (MODIFY: 5 new test cases)

**Analog within same file:** `tokio::join!` race-test (HLPR-04, lines 8784-8819), audit-verify pattern (HLPR-07, lines 9171-9279), `setup_with_pool` for direct DB queries (lines 3447-3465), `create_open_assembly_for_helper_test` + `create_helper_token_for_test` helpers (lines 8603-8663).

**Race-test pattern (HLPR-04, lines 8781-8819)** — verbatim shape for SYNC-02 race test:
```rust
#[tokio::test]
async fn test_helper_token_redeem_race_one_succeeds_one_fails() {
    let server = setup().await;
    let client = reqwest::Client::new();
    let assembly_id = create_open_assembly_for_helper_test(&client, &server).await;
    let (_token_id, code) = create_helper_token_for_test(&client, &server, assembly_id, "Carla").await;

    let url = server.url("/api/helper/redeem");
    let body_a = serde_json::json!({ "code": code.clone() });

    let (resp_a, resp_b) = tokio::join!(
        client.post(&url).json(&body_a).send(),
        client.post(&url).json(&body_b).send(),
    );
    let status_a = resp_a.unwrap().status();
    let status_b = resp_b.unwrap().status();

    let mut statuses = [status_a, status_b];
    statuses.sort_by_key(|s| s.as_u16());
    assert_eq!(statuses[0], StatusCode::OK, ...);
    assert_eq!(statuses[1], StatusCode::GONE, ...);
}
```

**Apply for SYNC-02 attendance UPSERT-race** (RESEARCH §Discretion 4 lines 660-696):
- Two parallel `client.put(server.url(&format!("/api/attendance/{}/{}", aid, mid))).send()` via `tokio::join!`.
- Both expect `200 OK` (unlike HLPR-04 where one is GONE — UPSERT is idempotent, redeem is not).
- Verify `stats.present == 1` after the race.

**Audit-Verify pattern (HLPR-07, lines 9241-9256)** — for ATTN-05:
```rust
let response = client.get(server.url("/api/audit/verify")).send().await.unwrap();
assert_eq!(response.status(), StatusCode::OK);
let verify: genossi_rest_types::VerifyResponseTO = response.json().await.unwrap();
assert!(verify.valid, "audit hash chain must be valid");
assert!(verify.broken_links.is_empty(), "broken_links must be empty; got {:?}", verify.broken_links);
```

**Paged-audit-listing pattern (HLPR-07, lines 9258-9278)**:
```rust
let response = client.get(server.url("/api/audit?entity_type=helper_token")).send().await.unwrap();
let paged: serde_json::Value = response.json().await.unwrap();
let paged_entries = paged["entries"].as_array().unwrap();
```
Apply for ATTN-05: query `entity_type=attendance` — assertion is that `paged_entries.len()` is **unchanged** before/after 100 toggles (no audit entries added).

**Direct-DB-query setup pattern (`setup_with_pool`, lines 3447-3465)** — for cascade-test (SC#8):
```rust
async fn setup_with_pool() -> (TestServer, Arc<SqlitePool>) {
    let pool = Arc::new(SqlitePool::connect("sqlite::memory:").await.unwrap());
    sqlx::migrate!("../migrations/sqlite").run(&*pool).await.unwrap();
    let rest_state = RestStateImpl::new(pool.clone());
    let server = start_test_server(rest_state).await;
    (server, pool)
}
```
Apply: cascade-test uses `setup_with_pool` to assert direct DB state via `sqlx::query_scalar(...).bind(...).fetch_one(&*pool).await` — verify session row absence after `close_assembly`. Pattern verbatim per RESEARCH §Pitfall 8 lines 1455-1497.

**Setup helpers reuse:**
- `create_open_assembly_for_helper_test` (lines 8603-8637) — creates GV, opens it. Direct reuse.
- `create_helper_token_for_test` (lines 8640-8663) — creates token, redeems via separate POST. Direct reuse.

**Apply — 5 new test cases:**
1. **SYNC-02 race** (`test_attendance_upsert_race_one_row_two_200ok`) — see RESEARCH lines 663-695.
2. **SC#8 cascade** (`test_close_assembly_cascade_invalidates_helper_sessions`) — see RESEARCH lines 1454-1494.
3. **ATTN-01 PII-leak** (`test_attendance_member_to_has_no_pii_fields`) — see RESEARCH lines 1361-1383, whitelist+blacklist key-iteration on JSON response.
4. **ATTN-05 hash-chain stability** (`test_attendance_toggle_burst_does_not_pollute_audit_chain`) — see RESEARCH lines 1395-1433.
5. **ASSY-06 Vorstand-Post-Close-Edit** (`test_vorstand_can_edit_attendance_after_close`) — see RESEARCH lines 1504-1540.

---

## Shared Patterns

### Authentication Extraction
**Source:** `genossi_rest/src/lib.rs:50-74`
**Apply to:** All NEW REST handlers in `genossi_rest/src/attendance.rs`
```rust
let auth = crate::extract_auth_context(Some(context))?;
```
This call returns `Result<Authentication<...>, RestError>` — already correctly mapped to `RestError::Unauthorized` for missing context. Phase 3 attendance handlers use this for ALL 4 endpoints (list_members, mark_present, mark_absent, stats).

### Error Handling
**Source:** `genossi_rest/src/lib.rs:75-111`
**Apply to:** All NEW REST handlers + the `map_attendance_error` helper
```rust
pub enum RestError {
    NotFound, BadRequest(String), Conflict(String), Unauthorized,
    UnsupportedMediaType(String), InternalError(String),
    Forbidden(String),  // <-- already exists, used by Phase 3 D-26
    Gone(String),
}

impl From<genossi_service::ServiceError> for RestError {
    fn from(e: genossi_service::ServiceError) -> Self {
        match e {
            ServiceError::EntityNotFound(_) => RestError::NotFound,
            ServiceError::ValidationError(items) => RestError::BadRequest(...),
            ServiceError::PermissionDenied => RestError::Unauthorized,  // <-- 401 by default!
            ServiceError::Conflict(msg) => RestError::Conflict(msg.to_string()),
            _ => RestError::InternalError(format!("{:?}", e)),
        }
    }
}
```
**CRITICAL Phase-3 deviation:** D-26 wants `PermissionDenied → 403 Forbidden`. Use a local `map_attendance_error` differential (Pattern 7 in RESEARCH) — DO NOT mutate the global `From<ServiceError>` impl (would break Phase 1+2 endpoints).

### Permission-Privilege constant
**Source:** `genossi_service/src/permission.rs::ADMIN_PRIVILEGE` (re-exported from various places, also `genossi_service_impl/src/assembly.rs:48` defines a local one)
**Apply to:** `AttendanceServiceImpl` admin-branch in `check_assembly_access` (D-19 — no new privilege constant per ATTN-06)
```rust
const ADMIN_PRIVILEGE: &str = "admin";  // or `use genossi_service::permission::ADMIN_PRIVILEGE;`
```

### Logging
**Source:** `genossi_rest/src/assembly.rs:108` and similar — `#[instrument(skip(rest_state))]`
**Apply to:** All 4 NEW REST handlers in `genossi_rest/src/attendance.rs`
```rust
#[instrument(skip(rest_state))]
pub async fn mark_attendance_present<RestState: ...>(...) -> Response { ... }
```

### Tx-Lifecycle
**Source:** `genossi_service_impl/src/assembly.rs:259, 302` — `use_transaction(None)` + `commit(tx)` bracket
**Apply to:** All 4 service methods in `AttendanceServiceImpl`
```rust
let tx = self.transaction_dao.use_transaction(None).await?;
// ... DAO calls with tx.clone() ...
self.transaction_dao.commit(tx).await?;
```

### Pool-vs-TX-Caveat (Cascade-only)
**Source:** `genossi_service_impl/src/helper_token.rs:316-325` (Phase-2 documented the same pattern)
**Apply to:** Cascade-loop in `AssemblyServiceImpl::close_assembly` (D-15 → CONFLICT 2 resolution)
- Commit the close-tx BEFORE iterating `permission_dao.delete_session(...)` calls.
- Use `tracing::warn!` on per-session errors (Continue-on-Error per RESEARCH Discretion 5).

### `From<&Entity>` TO conversions
**Source:** `genossi_rest_types/src/lib.rs:1080-1095` (`From<&Assembly> for AssemblyTO`)
**Apply to:** `From<&AttendanceMemberRow> for AttendanceMemberTO` — directly from DAO-row, NOT from `MemberTO` (D-24 + Pitfall 6).

---

## No Analog Found

All Phase-3 files have a strong analog in the existing codebase. No file requires inventing patterns from RESEARCH.md only.

| File | Why an analog exists |
|------|----------------------|
| All NEW files | Phase 1+2 codebase already provides every Pattern (lightweight join-table, UPSERT idiom, trait+automock, gen_service_impl, REST-handler shape, differential error mapping, race-test, audit-verify, setup_with_pool). |

---

## Metadata

**Analog search scope:**
- `genossi_dao/src/` (all files matching `*.rs`)
- `genossi_dao_impl_sqlite/src/` (all files matching `*.rs`)
- `genossi_service/src/` and `genossi_service_impl/src/`
- `genossi_rest/src/` (handler + lib.rs)
- `genossi_rest_types/src/lib.rs`
- `genossi_bin/src/lib.rs` and `genossi_bin/tests/e2e_tests.rs`
- `migrations/sqlite/` (Phase-1+2 most recent migrations)

**Files scanned:** 14 analogs read end-to-end or via targeted ranges.

**Pattern extraction date:** 2026-05-03

**Decision-Conflict Notes:**
- **Conflict 1** (D-26 403 vs. global 401-mapping): Plan must use local `map_attendance_error` helper in `genossi_rest/src/attendance.rs` (Pattern 7 in RESEARCH). `RestError::Forbidden` already exists at `genossi_rest/src/lib.rs:84`.
- **Conflict 2** (D-15 single-tx-cascade vs. pool-based `delete_session`): Plan must follow Phase-2 commit-before-pool pattern (`genossi_service_impl/src/helper_token.rs:316-325`). Cascade reorder: list_session_ids INSIDE tx → commit → for-loop delete_session OUTSIDE tx with Continue-on-Error.

**Open Question 1 Recommendation:** `ClaimContext::as_helper(&self) -> Option<(Arc<str>, Uuid)>` with `None` default-impl. Pattern from existing `has_claims` method (claim_context.rs:4-7).

---

*Phase: 3 — Attendance-Aggregat + Cascade-Invalidation*
*Pattern map written: 2026-05-03*
