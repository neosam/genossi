# Phase 2: Helfer-Token + Session + AuthContext::Helper - Pattern Map

**Mapped:** 2026-05-03
**Files analyzed:** 19 (9 new, 10 modified)
**Analogs found:** 17 / 19 (2 with no direct analog — public unauthenticated handler with cookie set + AuthContext-variant extension)

> **Phase 1 `assembly` aggregate is the canonical structural template** for the new `helper_token` aggregate. Wherever this document says "copy from assembly", the planner should reference the exact line ranges below.

## File Classification

| New / Modified File | Role | Data Flow | Closest Analog | Match Quality |
|---------------------|------|-----------|----------------|---------------|
| **NEW** `migrations/sqlite/<ts>_create_helper_token_table.sql` | migration | schema | `migrations/sqlite/20260502000000_create_assembly_table.sql` | exact (FK addition needed) |
| **NEW** `genossi_dao/src/helper_token.rs` | DAO trait + Entity + Auditable | CRUD + atomic-update | `genossi_dao/src/assembly.rs` | exact (extend with `atomic_redeem` + `set_session_id`) |
| **NEW** `genossi_dao_impl_sqlite/src/helper_token.rs` | DAO SQLite impl | CRUD + atomic-update + lookup | `genossi_dao_impl_sqlite/src/assembly.rs` | exact (atomic-redeem-via-RETURNING is novel) |
| **NEW** `genossi_service/src/helper_token.rs` | Service trait + DI deps | request-response | `genossi_service/src/assembly.rs` | role-match |
| **NEW** `genossi_service_impl/src/helper_token.rs` | Service impl + token-codegen + SHA256 + QR + redeem orchestration | CRUD + transform + event | `genossi_service_impl/src/assembly.rs` | role-match (codegen/QR/redeem are novel sub-patterns) |
| **NEW** `genossi_rest_types/src/helper_token.rs` (or appended block in `lib.rs`) | TOs (HelperTokenTO, HelperTokenCreateResponseTO, RedeemRequestTO, RedeemResponseTO) | DTO | `genossi_rest_types/src/lib.rs:1007-1141` (Assembly TOs) | exact |
| **NEW** `genossi_rest/src/helper_token.rs` | REST handlers (Vorstand) + nested router | request-response | `genossi_rest/src/assembly.rs` | exact |
| **NEW** `genossi_rest/src/helper_redeem.rs` (or nested block in `helper_token.rs`) | Public REST handler with Set-Cookie | request-response (no auth) | `genossi_rest/src/application.rs:117-204` (`public_join`) + `genossi_rest/src/session.rs:21-70` (Cookie-Build) | role-match (cookie-build code from `register_session`) |
| **NEW** `genossi_bin/tests/helper_token_e2e.rs` (or appended to `e2e_tests.rs`) | E2E tests (HLPR-04 race, HLPR-05 cascade, HLPR-07 audit) | test | `genossi_bin/tests/e2e_tests.rs:8361-8513` (`test_assembly_lifecycle_audit_chain_intact`) | exact (race-test pattern is novel for `tokio::join!`) |
| **MOD** `genossi_service/src/auth_types.rs` | enum-extension | type | `genossi_service/src/auth_types.rs:92-100` (existing `AuthContext`) | exact |
| **MOD** `genossi_service_impl/src/session.rs` | service-impl extension | request-response | `genossi_service_impl/src/session.rs:84-189` (existing `verify_user_session` + `extract_auth_context` + `ensure_user_and_create_session_with_claims`) | exact |
| **MOD** `genossi_service_impl/src/permission.rs` | service-impl match-arm | request-response | `genossi_service_impl/src/permission.rs:28-48` (existing `check_permission`) | exact |
| **MOD** `genossi_rest/src/auth_middleware.rs` | confirm extract path | middleware | already present at `genossi_rest/src/auth_middleware.rs:101-156` | no change required (path already delegates to `SessionService::extract_auth_context`) |
| **MOD** `genossi_bin/src/lib.rs` | DI wiring | DI | `genossi_bin/src/lib.rs:149-167,492-502` (Phase-1 `AssemblyService` wiring) | exact |
| **MOD** `genossi_rest/src/lib.rs` | router-nest + ApiDoc-merge + redeem-rate-layer | routing | `genossi_rest/src/lib.rs:566` (assembly nest), `:651-657` (public-join with `join_rate_layer`), `:475-485` (`join_rate_config`) | exact |
| **MOD** `genossi_dao/src/lib.rs` | re-export | module | `genossi_dao/src/lib.rs:1-13` | exact (add `pub mod helper_token;`) |
| **MOD** `genossi_dao_impl_sqlite/src/lib.rs` | re-export | module | `genossi_dao_impl_sqlite/src/lib.rs:1-14` | exact |
| **MOD** `genossi_service/src/lib.rs` | re-export | module | `genossi_service/src/lib.rs:1-18` | exact |
| **MOD** `genossi_service_impl/src/lib.rs` | re-export | module | `genossi_service_impl/src/lib.rs:1-24` | exact |
| **MOD** `genossi_rest_types/src/lib.rs` | TO additions | DTO | `genossi_rest_types/src/lib.rs:1007-1141` (Assembly TOs) | exact |
| **MOD** `Cargo.toml` (workspace) | dependency declaration | config | `Cargo.toml:23-48` | exact (add `qrcode = "0.14"` + `rand = "0.8"`; `sha2` already present in `genossi_service_impl/Cargo.toml:26`) |

---

## Pattern Assignments

### `migrations/sqlite/<ts>_create_helper_token_table.sql` (migration, schema)

**Analog:** `migrations/sqlite/20260502000000_create_assembly_table.sql` (entire file, 17 lines) + `migrations/sqlite/20260413000000_create_application_table.sql:1-19`

**Schema-pattern excerpt** (lines 1-17 of `assembly_table.sql`):
```sql
CREATE TABLE IF NOT EXISTS assembly (
    id BLOB PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    -- ... domain fields ...
    created TEXT NOT NULL,
    deleted TEXT,
    version BLOB NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_assembly_status ON assembly(status);
CREATE INDEX IF NOT EXISTS idx_assembly_deleted ON assembly(deleted);
CREATE INDEX IF NOT EXISTS idx_assembly_date ON assembly(date);
```

**Phase-2 deltas** (per RESEARCH §"Migration-Skeleton"):
- Add `FOREIGN KEY (assembly_id) REFERENCES assembly(id) ON DELETE RESTRICT`
- Add `FOREIGN KEY (session_id) REFERENCES session(id) ON DELETE SET NULL` (D-01; `session.id` is `TEXT PRIMARY KEY` — verified `migrations/sqlite/20250129000000_create_auth_tables.sql:48`)
- Add `CREATE UNIQUE INDEX idx_helper_token_token_hash ON helper_token(token_hash);` (atomic-redeem WHERE-clause + brute-force hardening)
- Add `CREATE INDEX idx_helper_token_assembly ON helper_token(assembly_id);` (Listing query)
- Add `CREATE INDEX idx_helper_token_deleted ON helper_token(deleted);` (Soft-delete filter)

---

### `genossi_dao/src/helper_token.rs` (DAO trait + Entity + Auditable)

**Analog:** `genossi_dao/src/assembly.rs` (entire file, 252 lines)

**Entity struct pattern** (lines 44-56 of assembly.rs):
```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssemblyEntity {
    pub id: Uuid,
    pub name: Arc<str>,
    pub date: time::PrimitiveDateTime,
    pub location: Option<Arc<str>>,
    pub status: AssemblyStatus,
    pub opened_at: Option<time::PrimitiveDateTime>,
    pub closed_at: Option<time::PrimitiveDateTime>,
    pub created: time::PrimitiveDateTime,
    pub deleted: Option<time::PrimitiveDateTime>,
    pub version: Uuid,
}
```

**Auditable impl pattern** (lines 58-94 of assembly.rs):
```rust
impl crate::auditable::Auditable for AssemblyEntity {
    fn entity_type() -> &'static str { "assembly" }
    fn entity_id(&self) -> Uuid { self.id }
    fn audit_fields(&self) -> Vec<(&'static str, Option<String>)> {
        let format_dt = |dt: &time::PrimitiveDateTime| {
            dt.assume_utc()
                .format(&Iso8601::DEFAULT)
                .unwrap_or_else(|err| {
                    tracing::error!(error = ?err, entity = "assembly",
                        "Failed to format datetime for audit field");
                    "<invalid datetime>".to_string()
                })
        };
        vec![
            ("name", Some(self.name.to_string())),
            // ...
        ]
    }
}
```

**For `HelperTokenEntity` (D-06):** entity_type = `"helper_token"`. `audit_fields()` MUST include `assembly_id`, `memo`, `used_at`, `session_id`, `revoked_at` and MUST NOT include `token_hash`. Pattern preserved verbatim from RESEARCH §"`Auditable`-Impl für `HelperTokenEntity`".

**DAO trait pattern** (lines 96-138 of assembly.rs):
```rust
#[automock(type Transaction = crate::MockTransaction;)]
#[async_trait]
pub trait AssemblyDao {
    type Transaction: crate::Transaction;

    async fn dump_all(&self, tx: Self::Transaction) -> Result<Arc<[AssemblyEntity]>, DaoError>;
    async fn create(&self, entity: &AssemblyEntity, process: &str, tx: Self::Transaction) -> Result<(), DaoError>;
    async fn update(&self, entity: &AssemblyEntity, process: &str, tx: Self::Transaction) -> Result<(), DaoError>;
    async fn all(&self, tx: Self::Transaction) -> Result<Arc<[AssemblyEntity]>, DaoError> {
        // default impl filters deleted IS NULL
    }
    async fn find_by_id(&self, id: Uuid, tx: Self::Transaction) -> Result<Option<AssemblyEntity>, DaoError> {
        // default impl filters deleted IS NULL
    }
}
```

**Phase-2 additions** (D-25, RESEARCH §Pattern 1):
- `async fn atomic_redeem(&self, token_hash: &str, used_at: PrimitiveDateTime, tx: Self::Transaction) -> Result<Option<(Uuid, Uuid)>, DaoError>;` — performs `UPDATE ... RETURNING id, assembly_id`
- `async fn set_session_id(&self, token_id: Uuid, session_id: &str, tx: Self::Transaction) -> Result<(), DaoError>;` — second UPDATE in same TX (Pitfall 3)
- `async fn lookup_status(&self, token_hash: &str, tx: Self::Transaction) -> Result<Option<(Option<PrimitiveDateTime>, Option<PrimitiveDateTime>)>, DaoError>;` — for differential 404/410/403 (D-24)
- `async fn all_for_assembly(&self, assembly_id: Uuid, tx: Self::Transaction) -> Result<Arc<[HelperTokenEntity]>, DaoError>;` — for D-21 listing

---

### `genossi_dao_impl_sqlite/src/helper_token.rs` (DAO SQLite impl)

**Analog:** `genossi_dao_impl_sqlite/src/assembly.rs` (entire file, 369 lines)

**Imports + format_dt + parse_datetime pattern** (lines 1-30, 83-88 of assembly_impl.rs):
```rust
use async_trait::async_trait;
use genossi_dao::assembly::{AssemblyDao, AssemblyEntity, AssemblyStatus};
use genossi_dao::DaoError;
use sqlx::SqlitePool;
use std::sync::Arc;
use time::PrimitiveDateTime;
use uuid::Uuid;
use crate::TransactionImpl;

pub(crate) fn parse_datetime(s: &str) -> Result<PrimitiveDateTime, time::error::Parse> {
    // ISO8601 first, then SQLite default formats — do NOT re-roll, reuse this fn
}

fn format_dt(dt: &PrimitiveDateTime) -> Result<String, DaoError> {
    let format = &time::format_description::well_known::Iso8601::DEFAULT;
    dt.assume_utc()
        .format(format)
        .map_err(|e| DaoError::ParseError(Arc::from(e.to_string())))
}
```

**Note:** `parse_datetime` is `pub(crate)` — `helper_token.rs` SHOULD reuse it (same crate). Plan task: import `crate::assembly::parse_datetime`.

**`FromRow`-pattern** (lines 32-71 of assembly_impl.rs):
```rust
#[derive(Debug, sqlx::FromRow)]
struct AssemblyDb {
    id: Vec<u8>,         // BLOB → Vec<u8>
    name: String,
    // ... TEXT and BLOB columns ...
    deleted: Option<String>,
    version: Vec<u8>,
}

impl TryFrom<&AssemblyDb> for AssemblyEntity {
    type Error = DaoError;
    fn try_from(db: &AssemblyDb) -> Result<Self, Self::Error> {
        Ok(AssemblyEntity {
            id: Uuid::from_slice(&db.id)?,
            // ...
        })
    }
}
```

**`create` pattern with `INSERT` + bind** (lines 109-146 of assembly_impl.rs):
```rust
async fn create(
    &self,
    entity: &AssemblyEntity,
    _process: &str,
    tx: Self::Transaction,
) -> Result<(), DaoError> {
    let id = entity.id.as_bytes().to_vec();
    let version = entity.version.as_bytes().to_vec();
    // ... bind values ...
    sqlx::query("INSERT INTO assembly (...) VALUES (?, ?, ...)")
        .bind(id)
        // ... binds ...
        .execute(tx.tx.lock().await.as_mut())
        .await
        .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?;
    Ok(())
}
```

**`update` with optimistic-locking + soft-delete-filter** (lines 148-205 of assembly_impl.rs):
```rust
// Pre-condition: row must exist and not be soft-deleted (NotFound vs Conflict separation)
let exists = sqlx::query_scalar::<_, i32>(
    "SELECT COUNT(*) FROM assembly WHERE id = ? AND deleted IS NULL",
).bind(id.clone()).fetch_one(tx.tx.lock().await.as_mut()).await?;
if exists == 0 { return Err(DaoError::NotFound); }

let rows_affected = sqlx::query(
    "UPDATE assembly SET ... version = ? WHERE id = ? AND version = ? AND deleted IS NULL",
).bind(...).execute(...).await?.rows_affected();

if rows_affected == 0 {
    return Err(DaoError::ConflictError(Arc::from("Version mismatch")));
}
```

**Phase-2 atomic-redeem pattern** (RESEARCH §"Pattern 1" — verbatim):
```rust
#[derive(Debug, sqlx::FromRow)]
struct RedeemRow { id: Vec<u8>, assembly_id: Vec<u8> }

async fn atomic_redeem(
    &self,
    token_hash: &str,
    used_at: PrimitiveDateTime,
    tx: TransactionImpl,
) -> Result<Option<(Uuid, Uuid)>, DaoError> {
    let used_at_str = format_dt(&used_at)?;
    let row: Option<RedeemRow> = sqlx::query_as::<_, RedeemRow>(
        "UPDATE helper_token SET used_at = ? \
         WHERE token_hash = ? AND used_at IS NULL AND revoked_at IS NULL AND deleted IS NULL \
         RETURNING id, assembly_id",
    )
    .bind(used_at_str)
    .bind(token_hash)
    .fetch_optional(tx.tx.lock().await.as_mut())
    .await
    .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?;
    Ok(match row {
        Some(r) => Some((Uuid::from_slice(&r.id)?, Uuid::from_slice(&r.assembly_id)?)),
        None => None,
    })
}
```

**Critical:** Use `query_as::<_, RedeemRow>(...)` (NOT `query_as!` macro) — Pitfall 1 in RESEARCH (SQLx RETURNING-nullability bug). Use `fetch_optional` (NOT `fetch_one`) — Anti-Pattern in RESEARCH.

**Unit-test pattern** (lines 208-368 of assembly_impl.rs): set up in-memory pool, create FK-prerequisite rows (e.g. assembly + session for helper_token), run create/find/update/atomic_redeem assertions.

---

### `genossi_service/src/helper_token.rs` (Service trait + DTOs + DI deps)

**Analog:** `genossi_service/src/assembly.rs` (entire file, 240 lines)

**Domain-type + From<&Entity> pattern** (lines 19-69 of service/assembly.rs):
```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Assembly {
    pub id: Uuid,
    pub name: Arc<str>,
    // ... mirror of Entity but with Arc<str> for strings ...
}

impl From<&AssemblyEntity> for Assembly {
    fn from(entity: &AssemblyEntity) -> Self { /* field-by-field copy */ }
}

impl From<&Assembly> for AssemblyEntity {
    fn from(a: &Assembly) -> Self { /* reverse */ }
}
```

**Submission-input + Service-trait pattern** (lines 71-153 of service/assembly.rs):
```rust
#[derive(Clone, Debug)]
pub struct AssemblySubmission {
    pub name: Arc<str>,
    // ... only fields the caller controls; service sets id/version/created/status ...
}

#[automock(type Context=(); type Transaction = genossi_dao::MockTransaction;)]
#[async_trait]
pub trait AssemblyService {
    type Context: Clone + Debug + PartialEq + Eq + Send + Sync + 'static;
    type Transaction: genossi_dao::Transaction;

    async fn create_assembly(
        &self,
        submission: &AssemblySubmission,
        context: Authentication<Self::Context>,
    ) -> Result<Assembly, ServiceError>;
    // ...
}
```

**Phase-2 method shape** (per CONTEXT D-21/D-22):
- `async fn create_helper_token(&self, assembly_id: Uuid, submission: &HelperTokenSubmission, ctx: Authentication<...>) -> Result<HelperTokenCreated, ServiceError>` — returns entity+code+qr_svg as a domain struct
- `async fn list_for_assembly(&self, assembly_id: Uuid, ctx: ...) -> Result<Arc<[HelperToken]>, ServiceError>`
- `async fn revoke_helper_token(&self, assembly_id: Uuid, token_id: Uuid, ctx: ...) -> Result<HelperToken, ServiceError>`
- `async fn redeem_helper_token(&self, code: &str) -> Result<HelperRedeemSuccess, ServiceError>` — **NO ctx** (public path, D-22)

`HelperRedeemSuccess { session_id: Arc<str>, assembly_id: Uuid, expires_at: i64 }` — used by REST handler to set cookie.

---

### `genossi_service_impl/src/helper_token.rs` (Service impl + codegen + SHA256 + QR + redeem)

**Analog:** `genossi_service_impl/src/assembly.rs` (lines 25-348 — service body; lines 350+ are tests)

**`gen_service_impl!`-DI-skeleton** (lines 50-60 of service_impl/assembly.rs):
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

**For `HelperTokenServiceImpl`** (per CONTEXT §"Integration Points"):
```rust
gen_service_impl! {
    struct HelperTokenServiceImpl: HelperTokenService = HelperTokenServiceDeps {
        HelperTokenDao: HelperTokenDao<Transaction = Self::Transaction> = helper_token_dao,
        AssemblyDao: AssemblyDao<Transaction = Self::Transaction> = assembly_dao,  // D-18 status check + D-23 lifecycle guard
        AuditLogDao: AuditLogDao<Transaction = Self::Transaction> = audit_log_dao, // D-07 audit
        PermissionService: PermissionService<Context = Self::Context> = permission_service, // admin check
        PermissionDao: PermissionDao<Transaction = Self::Transaction> = permission_dao, // D-17 ensure_user_exists
        SessionService: SessionService = session_service, // D-15/17 session creation
        UuidService: UuidService = uuid_service,
        TransactionDao: TransactionDao<Transaction = Self::Transaction> = transaction_dao,
    }
}
```

**Process-string convention** (lines 44-48 of service_impl/assembly.rs):
```rust
const ASSEMBLY_PROCESS_CREATE: &str = "assembly.create";
const ASSEMBLY_PROCESS_OPEN: &str = "assembly.open";
const ADMIN_PRIVILEGE: &str = "admin";
```
**For Phase 2 (D-07, D-26):** `const HELPER_TOKEN_PROCESS_CREATE: &str = "helper_token.create";`

**Service-method pattern** (lines 67-110 of service_impl/assembly.rs — `create_assembly`):
```rust
async fn create_assembly(...) -> Result<Assembly, ServiceError> {
    let tx = self.transaction_dao.use_transaction(None).await?;     // optional-tx pattern
    let user_id = self.permission_service
        .current_user_id(context.clone()).await?
        .unwrap_or_else(|| "SYSTEM".to_string());
    self.permission_service
        .check_permission(ADMIN_PRIVILEGE, context).await?;

    let now = time::OffsetDateTime::now_utc();
    let created = time::PrimitiveDateTime::new(now.date(), now.time());
    let entity = AssemblyEntity { id: self.uuid_service.new_v4().await, /* ... */ };

    crate::audited_create!(self, self.assembly_dao, &entity,
                           ASSEMBLY_PROCESS_CREATE, &user_id, tx);

    self.transaction_dao.commit(tx).await?;
    Ok(Assembly::from(&entity))
}
```

**Lifecycle-guard pattern** (lines 142-155 of service_impl/assembly.rs):
```rust
let mut entity = self.assembly_dao.find_by_id(id, tx.clone()).await?
    .ok_or(ServiceError::EntityNotFound(id))?;

if entity.status != AssemblyStatus::Preparation {
    return Err(ServiceError::Conflict(Arc::from(format!(
        "Cannot update assembly: status is '{}', expected 'Preparation' (D-07)",
        entity.status.as_str()
    ))));
}
```
**For Phase 2 revoke (D-23):** check `assembly.status in {Preparation, Open}`. For Phase 2 create: check `assembly.status in {Preparation, Open}` too (Vorstand should not create tokens for Closed GVs).

**Audit-macro invocation** (lines 99-106 of service_impl/assembly.rs):
```rust
crate::audited_create!(
    self,
    self.assembly_dao,
    &entity,
    ASSEMBLY_PROCESS_CREATE,
    &user_id,
    tx
);
```
**Macro definition:** `genossi_service_impl/src/audit_macros.rs:6-36`. Expects `self.audit_log_dao` and `self.uuid_service` (provided by `gen_service_impl!`).

**Phase-2 novel sub-patterns** (RESEARCH §Patterns 3-4):

```rust
// Pattern 3: Crockford-Base32 generator (free function in helper_token.rs)
use rand::{RngCore, rngs::OsRng};
const CROCKFORD_ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
pub fn generate_crockford_code(len: usize) -> String {
    let mut buf = vec![0u8; len];
    OsRng.fill_bytes(&mut buf);
    buf.iter().map(|&b| CROCKFORD_ALPHABET[(b & 0x1f) as usize] as char).collect()
}

// SHA256 token-hash (sha2 already in Cargo.toml)
use sha2::{Sha256, Digest};
pub fn sha256_hex(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    format!("{:x}", hasher.finalize())
}

// Pattern 4: QR-SVG render
use qrcode::{QrCode, EcLevel, render::svg};
pub fn render_qr_svg(payload: &str) -> Result<String, ServiceError> {
    let code = QrCode::with_error_correction_level(payload.as_bytes(), EcLevel::Q)
        .map_err(|e| ServiceError::InternalError(Arc::from(format!("QR generate: {}", e))))?;
    Ok(code.render::<svg::Color>().build())
}
```

**Redeem-orchestration pattern** (RESEARCH §Architecture Diagram, REDEEM PATH steps 1-9):
```rust
async fn redeem_helper_token(&self, code: &str) -> Result<HelperRedeemSuccess, ServiceError> {
    // 1. Format-validation (10 chars Crockford alphabet) → ServiceError::ValidationError on miss
    // 2. token_hash = sha256_hex(code)
    let tx = self.transaction_dao.use_transaction(None).await?;
    let now = time::OffsetDateTime::now_utc();
    let now_pdt = time::PrimitiveDateTime::new(now.date(), now.time());
    // 3. atomic_redeem
    let result = self.helper_token_dao.atomic_redeem(&token_hash, now_pdt, tx.clone()).await?;
    let (token_id, assembly_id) = match result {
        Some(t) => t,
        None => {
            // Differential lookup → return ServiceError variants the REST layer maps to 404/410/403
            let status = self.helper_token_dao.lookup_status(&token_hash, tx.clone()).await?;
            // Plan finalises the ServiceError → RestError mapping
        }
    };
    // 4. assembly_dao.find_by_id → status must be Open (D-24 → 403 if not)
    // 5. user_id = format!("helper:{}", token_id)  [D-17]
    // 6. claims = json!({"kind":"helper","assembly_id": assembly_id.to_string()})  [D-16]
    // 7. session = self.session_service.ensure_user_and_create_session_with_claims(&user_id, 86400, Some(claims_json))?  [D-18]
    // 8. self.helper_token_dao.set_session_id(token_id, &session.session_id, tx.clone()).await?  [Pitfall 3: same TX]
    self.transaction_dao.commit(tx).await?;
    // 9. Return HelperRedeemSuccess { session_id, assembly_id, expires_at }
}
```

---

### `genossi_rest_types/src/helper_token.rs` (or appended to `lib.rs`)

**Analog:** `genossi_rest_types/src/lib.rs:1007-1141` (Assembly TOs)

**Status enum + bidirectional From pattern** (lines 1007-1034):
```rust
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum AssemblyStatusTO { Preparation, Open, Closed }

impl From<&genossi_dao::assembly::AssemblyStatus> for AssemblyStatusTO { /* match */ }
impl From<&AssemblyStatusTO> for genossi_dao::assembly::AssemblyStatus { /* match */ }
```

**For Phase-2 derived status (D-02):**
```rust
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum HelperTokenStatusTO { Open, Used, Revoked }
// Derived in From<&HelperToken> by inspecting used_at / revoked_at columns.
```

**TO struct with iso8601_datetime pattern** (lines 1036-1078):
```rust
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct AssemblyTO {
    pub id: Uuid,
    pub name: String,
    #[serde(serialize_with = "iso8601_datetime::serialize",
            deserialize_with = "iso8601_datetime::deserialize", default)]
    pub date: Option<time::PrimitiveDateTime>,
    // ... all PrimitiveDateTime fields use the iso8601_datetime serde module ...
    pub status: AssemblyStatusTO,
    pub version: Option<Uuid>,
}

impl From<&genossi_service::assembly::Assembly> for AssemblyTO {
    fn from(a: &genossi_service::assembly::Assembly) -> Self { /* field-by-field */ }
}
```

**Request struct pattern** (lines 1112-1141):
```rust
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateAssemblyRequest {
    #[schema(example = "GV 2026")]
    pub name: String,
    #[serde(serialize_with = "iso8601_datetime::serialize",
            deserialize_with = "iso8601_datetime::deserialize", default)]
    pub date: Option<time::PrimitiveDateTime>,
    pub location: Option<String>,
}
```

**Phase-2 TOs needed:**
- `HelperTokenTO` — id, assembly_id, memo, status (HelperTokenStatusTO), used_at, revoked_at, created, version
- `HelperTokenCreateResponseTO` — `token: HelperTokenTO`, `code: String`, `qr_svg: String` (D-21 — only-once)
- `CreateHelperTokenRequest` — `memo: String`
- `RedeemRequest` — `code: String`
- `RedeemResponse` — `assembly_id: Uuid`, `expires_at: String` (ISO8601)

---

### `genossi_rest/src/helper_token.rs` (Vorstand handlers + nested router)

**Analog:** `genossi_rest/src/assembly.rs` (entire file, 521 lines)

**RestState trait pattern** (lines 20-24 of rest/assembly.rs):
```rust
pub trait AssemblyRestState: Clone + Send + Sync + 'static {
    type AssemblyService: AssemblyService<Context = crate::ContextType> + Send + Sync + 'static;
    fn assembly_service(&self) -> Arc<Self::AssemblyService>;
}
```

**Validation-helper pattern** (lines 28-66 — `validate_required_field`, `validate_optional_max_len`):
```rust
fn validate_required_field(errors: &mut Vec<ValidationFailureItem>, field: &str, value: &str, max_len: usize) {
    if value.is_empty() { errors.push(/*missing*/); }
    else if value.chars().count() > max_len { /* WR-05: chars not bytes */ }
}
```

**For Phase 2:** validate `memo` (e.g. `validate_required_field(errors, "memo", &body.memo, 256)`).

**Handler pattern with `#[instrument]` + `#[utoipa::path]` + `error_handler`** (lines 108-188 — `create_assembly`):
```rust
#[instrument(skip(rest_state))]
#[utoipa::path(
    post,
    tag = "Assemblies",
    path = "",
    request_body = CreateAssemblyRequest,
    responses(
        (status = 201, description = "Created", body = AssemblyTO),
        (status = 401, description = "Unauthorized"),
        (status = 422, description = "Validation Error"),
    ),
)]
pub async fn create_assembly<RestState: RestStateDef + AssemblyRestState>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Json(body): Json<CreateAssemblyRequest>,
) -> Response {
    error_handler((async {
        let auth = crate::extract_auth_context(Some(context))?;
        validate_create_assembly_request(&body).map_err(|errs| {
            RestError::BadRequest(/* ... */)
        })?;
        let submission = AssemblySubmission { /* ... */ };
        let assembly = rest_state.assembly_service().create_assembly(&submission, auth).await?;
        Ok(Response::builder()
            .status(201).header("Content-Type", "application/json")
            .body(Body::new(serde_json::to_string(&AssemblyTO::from(&assembly))?))
            .unwrap())
    }).await)
}
```

**Router-build pattern** (lines 349-361):
```rust
pub fn generate_route<RestState: RestStateDef + AssemblyRestState>() -> Router<RestState> {
    Router::new()
        .route("/", get(list_assemblies::<RestState>).post(create_assembly::<RestState>))
        .route("/{id}", get(get_assembly::<RestState>).put(update_assembly::<RestState>))
        .route("/{id}/open", post(open_assembly::<RestState>))
        .route("/{id}/close", post(close_assembly::<RestState>))
}
```

**ApiDoc pattern** (lines 363-381):
```rust
#[derive(OpenApi)]
#[openapi(
    paths(list_assemblies, create_assembly, get_assembly, update_assembly,
          open_assembly, close_assembly),
    components(schemas(AssemblyTO, AssemblyStatusTO, AssemblyDetailTO,
                       CreateAssemblyRequest, UpdateAssemblyRequest))
)]
pub struct ApiDoc;
```

**Phase-2 endpoint shape** (per D-21 — nested under assembly):
- `POST /api/assembly/{assembly_id}/helper-tokens` → `create_helper_token` handler returning `HelperTokenCreateResponseTO`
- `GET /api/assembly/{assembly_id}/helper-tokens` → `list_helper_tokens` returning `Vec<HelperTokenTO>`
- `POST /api/assembly/{assembly_id}/helper-tokens/{token_id}/revoke` → `revoke_helper_token`

Path-extractor for nested route uses `Path((assembly_id, token_id)): Path<(Uuid, Uuid)>` pattern.

---

### `genossi_rest/src/helper_redeem.rs` (Public redeem handler with Set-Cookie)

**Analog A — public unauthenticated handler:** `genossi_rest/src/application.rs:117-204` (`public_join`)

```rust
#[instrument(skip(state, headers))]
#[utoipa::path(
    post,
    tag = "Public Join",
    path = "/api/public/join",
    request_body = PublicJoinRequest,
    responses(
        (status = 201, description = "Application submitted", body = PublicJoinResponse),
        (status = 422, description = "Validation error", body = ValidationErrorResponse),
        (status = 429, description = "Rate limit exceeded"),
    ),
)]
pub async fn public_join<S: ApplicationRestState>(
    State(state): State<S>,
    headers: HeaderMap,
    Json(body): Json<PublicJoinRequest>,
) -> Response {
    error_handler((async { /* no extract_auth_context call */ }).await)
}

pub fn generate_public_route<S: ApplicationRestState>() -> Router<S> {
    Router::new().route("/join", post(public_join::<S>))
}
```

**For Phase-2 redeem:** No `Extension(context)`, no `extract_auth_context`. Body `Json<RedeemRequest>`. Service returns `HelperRedeemSuccess`. Handler must build a Set-Cookie header.

**Analog B — Set-Cookie construction:** `genossi_rest/src/session.rs:28-58` (`register_session`):
```rust
use tower_cookies::Cookie;
let cookies = request.extensions().get::<Cookies>()
    .expect("Cookies extension not set");

let cookie = Cookie::build(Cookie::new("app_session", session_id))
    .path("/")
    .expires(expires)        // OffsetDateTime
    .http_only(true)
    .same_site(tower_cookies::cookie::SameSite::Strict)
    .secure(true);
cookies.add(cookie.into());
```

**Plan-detail:** Response body contains `RedeemResponse { assembly_id, expires_at }`. Cookie attached via `tower_cookies::Cookies` extension or via raw `Set-Cookie` header on the `Response::builder()`. Lifetime `Max-Age=86400` (D-18 + RESEARCH Open Q3).

**Differential RestError-mapping** (D-24):
- `ServiceError::ValidationError` → 400 (already maps to `BadRequest`, see `lib.rs:94-100`)
- `ServiceError::EntityNotFound` → 404 (already maps in `lib.rs:93`)
- New variant required for **410 Gone** (used) and **403 Forbidden** (revoked / assembly !Open) — RESEARCH suggests adding a `RestError::Gone(String)` and `RestError::Forbidden(String)`, OR mapping via two new `ServiceError` variants. Plan finalises.

---

### `genossi_bin/tests/helper_token_e2e.rs` (or appended to `e2e_tests.rs`)

**Analog A — full setup + lifecycle assertions:** `genossi_bin/tests/e2e_tests.rs:24-38` (`setup()`) + `:8361-8513` (`test_assembly_lifecycle_audit_chain_intact`)

**setup pattern** (lines 24-38):
```rust
async fn setup() -> genossi_rest::test_server::test_support::TestServer {
    let pool = Arc::new(SqlitePool::connect("sqlite::memory:").await.expect(...));
    sqlx::migrate!("../migrations/sqlite").run(&*pool).await.expect("migrations");
    let rest_state = RestStateImpl::new(pool);
    start_test_server(rest_state).await
}
```

**audit-chain-verify pattern** (lines 8466-8513):
```rust
let response = client.get(server.url("/api/audit/verify")).send().await.unwrap();
let verify: VerifyResponseTO = response.json().await.unwrap();
assert!(verify.valid && verify.broken_links.is_empty());

let response = client.get(server.url(&format!("/api/audit/{}/{}", "helper_token", token_id)))
    .send().await.unwrap();
let entries: Vec<AuditLogEntryTO> = response.json().await.unwrap();
assert!(entries.iter().any(|e| e.process == "helper_token.create"));
```

**Note on audit endpoint:** Use `/api/audit/{entity_type}/{entity_id}` (lines 55-58 of `genossi_rest/src/audit_log.rs`) — there is also `/api/audit?entity_type=helper_token` for the paged view, but Pitfall 4 in RESEARCH warns: **`AuditQueryFilter` has no `process` field**. Filter by `entity_type`, then check `process` in returned entries.

**Analog B — race-test pattern:** RESEARCH §Pattern 5 (verbatim, no existing race test in codebase to copy):
```rust
let url = server.url("/api/helper/redeem");
let body_a = serde_json::json!({"code": code.clone()});
let body_b = serde_json::json!({"code": code.clone()});
let (resp_a, resp_b) = tokio::join!(
    client.post(&url).json(&body_a).send(),
    client.post(&url).json(&body_b).send(),
);
let mut statuses = [resp_a.unwrap().status(), resp_b.unwrap().status()];
statuses.sort();
assert_eq!(statuses, [StatusCode::OK, StatusCode::GONE]);
```

**Cascade-test pattern (HLPR-05):** redeem token → call helper-protected endpoint (Phase 2: any endpoint that asserts `AuthContext::Helper` → `PermissionDenied` is enough; even calling `GET /api/members` and asserting `403`/`401` works) → close assembly via `POST /api/assembly/{id}/close` → call same endpoint again → assert `401`. This works because of D-18 status-check in `verify_user_session`.

**Pitfall 5 (RESEARCH):** In `mock_auth` build (`#![cfg(feature = "mock_auth")]` at top of `e2e_tests.rs:1`), `MockSessionServiceImpl::extract_auth_context` returns `Some(AuthContext::Mock(MockContext::default()))` unconditionally. Phase-2 plan must extend `MockSessionServiceImpl` so that helper-cookie sessions actually round-trip. RESEARCH Open Q1 recommends a small extension that recognises `app_session` cookies whose claims indicate `kind=helper`. Plan must list this as an explicit sub-task.

---

### `genossi_service/src/auth_types.rs` (MOD — add `AuthContext::Helper` variant)

**Existing code** (lines 92-100):
```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthContext {
    Mock(MockContext),
    #[cfg(feature = "oidc")]
    Oidc(Arc<str>),
}
```

**Phase-2 modification (D-14):** add a third arm — **NO `cfg`-gate**, both feature builds know it.
```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthContext {
    Mock(MockContext),
    #[cfg(feature = "oidc")]
    Oidc(Arc<str>),
    Helper { session_id: Arc<str>, assembly_id: uuid::Uuid },
}
```

**Compiler-cascade:** every `match auth_context { ... }` site in the codebase must add a `Helper { .. }` arm. Use `grep -rn "AuthContext::" --include='*.rs'` to enumerate. Phase-2 plan must list each match site as an explicit sub-task; D-20 says all add `=> Err(ServiceError::PermissionDenied)` for Phase 2.

---

### `genossi_service_impl/src/session.rs` (MOD — extend `extract_auth_context` and `verify_user_session`)

**Analog (existing):** `genossi_service_impl/src/session.rs:84-189`

**`verify_user_session` shape** (lines 84-120):
```rust
async fn verify_user_session(&self, session_id: &str) -> Result<Option<UserSession>, ServiceError> {
    let session = self.permission_dao.get_session(session_id).await?;
    if let Some(session_entity) = session {
        let now = time::OffsetDateTime::now_utc().unix_timestamp();
        if session_entity.expires < now { /* delete + None */ }
        const INACTIVITY_TIMEOUT_SECS: i64 = 30 * 24 * 60 * 60;
        if now - session_entity.last_used_at > INACTIVITY_TIMEOUT_SECS { /* delete + None */ }
        self.permission_dao.touch_session(session_id, now).await?;
        Ok(Some(UserSession { /* ... */ }))
    } else { Ok(None) }
}
```

**`extract_auth_context` existing path** (lines 141-159):
```rust
async fn extract_auth_context(&self, session_id: Option<String>) -> Result<Option<AuthContext>, ServiceError> {
    match session_id {
        Some(sid) => {
            if let Some(session) = self.verify_user_session(&sid).await? {
                Ok(Some(AuthContext::Mock(MockContext { user_id: session.user_id })))
            } else { Ok(None) }
        }
        None => Ok(None),
    }
}
```

**Phase-2 extension (D-15/16/18/19 + RESEARCH §Pattern 2):**
- Parse `session.claims` JSON; if `kind == "helper"`:
  - Look up `assembly_dao.find_by_id(parsed.assembly_id)` (this requires injecting `AssemblyDao` and `TransactionDao` into `SessionServiceImpl`'s deps — RESEARCH recommendation accepted)
  - If `assembly.status == Open` → return `AuthContext::Helper { session_id, assembly_id }`
  - Else → `permission_dao.delete_session(&sid)` + return `Ok(None)`
- Early-return on `claims.is_none()` to avoid extra DB roundtrip on regular OIDC requests (Pitfall 2)

**`ensure_user_and_create_session_with_claims`** already exists and is reusable verbatim (lines 175-189, originally for "inventur token" auto-register — D-17 reuses identical pattern).

**`MockSessionServiceImpl` extension** (lines 582-591 — Pitfall 5):
```rust
async fn extract_auth_context(&self, session_id: Option<String>) -> Result<Option<AuthContext>, ServiceError> {
    if session_id.is_some() {
        Ok(Some(AuthContext::Mock(MockContext::default())))
    } else { Ok(None) }
}
```
Phase 2 must add a Helper-cookie recognition path here so E2E tests for HLPR-05 actually exercise the cascade. RESEARCH Open Q1 suggests recognising a cookie format like `app_session=helper:<assembly_uuid>:<token_id>` and synthesising `AuthContext::Helper {...}`. Plan finalises.

---

### `genossi_service_impl/src/permission.rs` (MOD — Helper match-arm)

**Existing `check_permission`** (lines 28-48):
```rust
async fn check_permission(&self, privilege: &str, context: Authentication<Self::Context>)
    -> Result<(), ServiceError>
{
    match context {
        Authentication::Full => Ok(()),
        Authentication::Context(ctx) => {
            let current_user = self.user_service.current_user(ctx).await?;
            if self.permission_dao.has_privilege(&current_user, privilege).await? {
                Ok(())
            } else { Err(ServiceError::PermissionDenied) }
        }
    }
}
```

**Phase-2 modification (D-20):** the `Authentication<Self::Context>` enum is unchanged, but every `AuthContext::Helper { .. }` extracted before `Authentication::Context(ctx)` is constructed must short-circuit to `Err(PermissionDenied)`. This happens **upstream** in `genossi_rest/src/lib.rs::extract_auth_context` (lines 51-73) which converts `Context` into `Authentication`. Plan must extend that conversion so a `Helper`-context → `Err(RestError::Unauthorized)` (or a new `Err(Forbidden)` to give a cleaner 403 — D-20 wording is `PermissionDenied`).

**Alternative wiring:** if `AuthenticatedContext` (the OIDC `Context` type) gets a Helper-flag in claims, the `check_permission` path can branch in this `match` block. Plan/researcher chose the clean separation: handle Helper in REST extraction.

---

### `genossi_rest/src/auth_middleware.rs` (MOD — verify path)

**Existing `extract_context_from_headers`** (lines 101-134) already calls `session_service.extract_auth_context(Some(session_id))` — **no change required** as long as `SessionServiceImpl::extract_auth_context` (above) returns the right `AuthContext::Helper` variant.

Plan-detail: confirm that the returned `AuthContext::Helper { .. }` propagates correctly through `request.extensions_mut().insert(auth_context)` (line 32) without any `Context` type-conversion truncating the variant. The `Context` type alias at `genossi_rest/src/lib.rs:44-47` is `MockContext` (mock_auth) or `Option<AuthenticatedContext>` (oidc). **The middleware stores `Option<AuthContext>` in extensions, not `Context`** (line 32, line 175). Helper-variant survives.

---

### `genossi_bin/src/lib.rs` (MOD — DI wiring for HelperTokenServiceImpl)

**Analog (existing `AssemblyServiceDependencies` block):** lines 149-167 + lines 492-502.

**Pattern (lines 149-167):**
```rust
pub struct AssemblyServiceDependencies;
unsafe impl Send for AssemblyServiceDependencies {}
unsafe impl Sync for AssemblyServiceDependencies {}

impl genossi_service_impl::assembly::AssemblyServiceDeps for AssemblyServiceDependencies {
    type Context = Context;
    type Transaction = Transaction;
    type AssemblyDao = AssemblyDao;
    type AssemblyMemberSnapshotDao = AssemblyMemberSnapshotDao;
    type MemberDao = MemberDao;
    type AuditLogDao = AuditLogDao;
    type PermissionService = PermissionService;
    type UuidService = UuidService;
    type TransactionDao = TransactionDao;
}

type AssemblyService = genossi_service_impl::assembly::AssemblyServiceImpl<AssemblyServiceDependencies>;
```

**Instantiation (lines 492-502):**
```rust
let assembly_dao = Arc::new(AssemblyDao::new(pool.clone()));
let assembly_member_snapshot_dao = Arc::new(AssemblyMemberSnapshotDao::new(pool.clone()));
let assembly_service = Arc::new(genossi_service_impl::assembly::AssemblyServiceImpl {
    assembly_dao,
    assembly_member_snapshot_dao,
    member_dao: member_dao.clone(),
    audit_log_dao: audit_log_dao.clone(),
    permission_service: permission_service.clone(),
    uuid_service: uuid_service.clone(),
    transaction_dao: transaction_dao.clone(),
});
```

**Add to `RestStateImpl` struct (lines 313-352)**, the constructor (`new`), and a new `RestStateDef` accessor (`fn helper_token_service(&self) -> Arc<HelperTokenService>`).

For Phase-2 `HelperTokenServiceDependencies`: same shape as above plus `PermissionDao = PermissionDao` (D-17) and `SessionService = SessionService` (D-15).

**Note on SessionService deps (RESEARCH §Pattern 2):** if Phase 2 adds `assembly_dao` and `transaction_dao` into `SessionServiceImpl` deps for D-18, the `SessionServiceDependencies` struct (lines 88-102, behind `cfg(feature = "oidc")`) needs the same two type-fields and the `let session_service = Arc::new(SessionServiceImpl { permission_dao, assembly_dao, transaction_dao })` call needs updating.

---

### `genossi_rest/src/lib.rs` (MOD — router + ApiDoc + redeem-rate-layer)

**Existing nest pattern** (line 566):
```rust
.nest("/api/assembly", assembly::generate_route::<RestState>())
```

**Existing public route + per-route rate-limit** (lines 475-485, 651-657):
```rust
let join_rate_config = Arc::new(GovernorConfigBuilder::default()
    .per_second(12).burst_size(5).finish().unwrap());
let join_rate_layer = GovernorLayer { config: join_rate_config };

// ... later:
let join_router = application::generate_public_route::<RestState>().layer(join_rate_layer);
let app = app
    .nest("/api/public", public_stats::generate_route::<RestState>())
    .nest("/api/public", join_router)
    .with_state(rest_state.clone());
```

**Phase-2 Vorstand router** — nest under existing assembly route OR add a sibling:
```rust
.nest("/api/assembly/{assembly_id}/helper-tokens", helper_token::generate_route::<RestState>())
```

**Phase-2 public-redeem route** (D-22, no auth) — analog to `join_router`:
```rust
let redeem_rate_config = Arc::new(GovernorConfigBuilder::default()
    .per_second(6).burst_size(10).finish().unwrap());  // Plan picks numbers (Discretion §Rate-Limiting)
let redeem_rate_layer = GovernorLayer { config: redeem_rate_config };

let redeem_router = helper_redeem::generate_public_route::<RestState>().layer(redeem_rate_layer);
let app = app
    // ...
    .nest("/api/helper", redeem_router)
    .with_state(rest_state.clone());
```

**ApiDoc-merge pattern** (lines 233-258):
```rust
#[derive(OpenApi)]
#[openapi(
    nest(
        // ...
        (path = "/api/assembly", api = assembly::ApiDoc),
        (path = "/api/audit", api = audit_log::ApiDoc),
        // ADD: (path = "/api/assembly/{assembly_id}/helper-tokens", api = helper_token::ApiDoc),
        // ADD: (path = "/api/helper", api = helper_redeem::ApiDoc),  // or merge inline
    )
)]
pub struct ApiDoc;
```

**Plan trait extension:** `pub fn create_app<RestState: RestStateDef + ... + helper_token::HelperTokenRestState>` — add the new trait bound to `create_app` and `start_server` (lines 412-419, 678-687).

---

### `genossi_dao/src/lib.rs` (MOD — re-export)

**Existing** (lines 1-12):
```rust
pub mod application;
pub mod assembly;
// ...
pub mod permission;
pub mod user_preference;
```

**Phase-2 addition:** add `pub mod helper_token;` alphabetically.

---

### `genossi_dao_impl_sqlite/src/lib.rs`, `genossi_service/src/lib.rs`, `genossi_service_impl/src/lib.rs`, `genossi_rest_types/src/lib.rs`

Same `pub mod helper_token;` addition pattern. For `genossi_service_impl/src/lib.rs`, follow line 23-24 pattern if a `pub use` re-export is desirable: `pub use helper_token::HelperTokenServiceImpl;`.

---

### `Cargo.toml` (workspace) (MOD — add `qrcode` and `rand`)

**Existing `[workspace.dependencies]`** (lines 23-48): `axum`, `sqlx`, `tower-cookies`, `tower-sessions`, etc.

**Phase-2 additions** (RESEARCH §"Standard Stack"):
```toml
qrcode = "0.14"
rand = { version = "0.8", default-features = false, features = ["std", "std_rng", "getrandom"] }
```

In `genossi_service_impl/Cargo.toml`: add `qrcode = { workspace = true }` and `rand = { workspace = true }`. `sha2 = "0.10"` is already at line 26 (verified).

---

## Shared Patterns

### Authentication (Vorstand admin endpoints)
**Source:** `genossi_rest/src/lib.rs:51-58` (`extract_auth_context`) + `genossi_service_impl/src/permission.rs:28-48` (`check_permission`)
**Apply to:** `create_helper_token`, `list_helper_tokens`, `revoke_helper_token` handlers

```rust
let auth = crate::extract_auth_context(Some(context))?;
// ... service call passes auth ...
self.permission_service.check_permission(ADMIN_PRIVILEGE, context).await?;
```

### Public (no-auth) endpoint with cookie attachment
**Source:** `genossi_rest/src/application.rs:117-204` (`public_join`) + `genossi_rest/src/session.rs:28-58` (cookie-build)
**Apply to:** `redeem_helper_token` handler

The handler does not extract `Context`. After service call, build a `Cookie` with `tower_cookies::Cookie::build(...).path("/").http_only(true).same_site(SameSite::Strict).secure(true).max_age(Duration::seconds(86400))` and either insert via `Cookies` extension or via raw `Set-Cookie` header on `Response::builder()`.

### Error handling (REST)
**Source:** `genossi_rest/src/lib.rs:75-163` (`RestError` enum, `From<ServiceError>`, `error_handler`)
**Apply to:** all handlers via `error_handler((async { ... }).await)`

Plan-detail (D-24): if differential 404/410/403 require new mappings, extend `RestError` with `Gone(String)` and `Forbidden(String)` variants OR introduce two new `ServiceError` variants — plan finalises which side owns the discrimination.

### Validation (REST)
**Source:** `genossi_rest/src/assembly.rs:28-66` (`validate_required_field`, `validate_optional_max_len`)
**Apply to:** `validate_create_helper_token_request` (memo length), `validate_redeem_request` (code length 10 + Crockford-alphabet; D-09/D-24)

```rust
fn validate_required_field(errors: &mut Vec<ValidationFailureItem>, field: &str, value: &str, max_len: usize) {
    if value.is_empty() { /*missing*/ }
    else if value.chars().count() > max_len { /* WR-05: chars not bytes */ }
}
```

### Audit logging (Service)
**Source:** `genossi_service_impl/src/audit_macros.rs:1-36` (`audited_create!`)
**Apply to:** `create_helper_token` only (D-07/D-08 — redeem and revoke NOT audited)

```rust
crate::audited_create!(self, self.helper_token_dao, &entity, HELPER_TOKEN_PROCESS_CREATE, &user_id, tx);
```

### Optional-Transaction in Service
**Source:** `genossi_service_impl/src/assembly.rs:72`
**Apply to:** every Phase-2 service method
```rust
let tx = self.transaction_dao.use_transaction(None).await?;
// ... operations on tx.clone() ...
self.transaction_dao.commit(tx).await?;
```

### Soft-delete-filter in DAO
**Source:** `genossi_dao_impl_sqlite/src/assembly.rs:168-178`
**Apply to:** `helper_token_dao_impl::find_by_id`, `all`, `update`, `atomic_redeem`, `lookup_status`, `all_for_assembly`

```sql
... WHERE id = ? AND deleted IS NULL
... WHERE token_hash = ? AND used_at IS NULL AND revoked_at IS NULL AND deleted IS NULL
```

### Optimistic-locking in DAO update
**Source:** `genossi_dao_impl_sqlite/src/assembly.rs:180-202`
**Apply to:** `helper_token_dao_impl::update` (used by `audited_update!` if Phase 3 enables revoke audit; harmless if unused in Phase 2)

```sql
UPDATE ... SET ... version = ? WHERE id = ? AND version = ? AND deleted IS NULL
```
Returns `0 rows_affected` → `Err(DaoError::ConflictError("Version mismatch"))`.

### TO/Entity ISO8601 datetime serde
**Source:** `genossi_rest_types/src/lib.rs:9-44` (`iso8601_datetime` module — `serialize` + `deserialize`)
**Apply to:** every `Option<PrimitiveDateTime>` field on `HelperTokenTO`, `HelperTokenCreateResponseTO`, `RedeemResponse`

```rust
#[serde(serialize_with = "iso8601_datetime::serialize",
        deserialize_with = "iso8601_datetime::deserialize", default)]
pub used_at: Option<time::PrimitiveDateTime>,
```

---

## No Analog Found

| File | Role | Data Flow | Reason |
|------|------|-----------|--------|
| `AuthContext::Helper`-variant addition itself | enum-extension | type | Adding a new enum variant has no analog in codebase — D-14 is a fresh extension. Pattern from RESEARCH §Pattern 2 must be applied. |
| Race-test `tokio::join!` over two `reqwest::post` against the same redeem endpoint | E2E test | concurrent | No existing race-test in `e2e_tests.rs` (8595 lines, all serial). RESEARCH §Pattern 5 supplies the verbatim pattern. |

For both: planner uses RESEARCH content directly.

## Metadata

**Analog search scope:** `genossi_dao/src/`, `genossi_dao_impl_sqlite/src/`, `genossi_service/src/`, `genossi_service_impl/src/`, `genossi_rest/src/`, `genossi_rest_types/src/`, `genossi_bin/src/`, `genossi_bin/tests/`, `migrations/sqlite/`, `Cargo.toml`
**Files scanned:** ~30 source files (Phase-1 `assembly` aggregate + auth/session/permission stack + `application` for public endpoint pattern + `audit_log` REST + e2e tests)
**Pattern extraction date:** 2026-05-03
