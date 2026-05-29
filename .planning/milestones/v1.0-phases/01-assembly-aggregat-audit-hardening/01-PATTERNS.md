# Phase 1: Assembly-Aggregat + Audit-Hardening - Pattern Map

**Mapped:** 2026-05-02
**Files analyzed:** 17 (10 NEW + 7 MODIFY)
**Analogs found:** 17 / 17 (100 %)

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `migrations/sqlite/<ts>_create_assembly_table.sql` | migration | DDL | `migrations/sqlite/20260413000000_create_application_table.sql` | exact (Status-Aggregat mit Soft-Delete + Version) |
| `migrations/sqlite/<ts>_create_assembly_member_snapshot_table.sql` | migration | DDL | `migrations/sqlite/20260413000000_create_application_table.sql` | partial (composite-PK & FK ist neu — keine andere Migration besitzt Composite-PK + FK; nächstgelegener Stil ist application-Migration) |
| `genossi_dao/src/assembly.rs` | entity + trait + auditable | request-response (Trait) | `genossi_dao/src/application.rs` (1-207) | exact |
| `genossi_dao/src/assembly_member_snapshot.rs` | entity + trait | batch-insert + lookup | `genossi_dao/src/application.rs` (98-140) | role-match (kein Auditable, kein version, kein soft-delete) |
| `genossi_dao_impl_sqlite/src/assembly.rs` | DAO impl | CRUD + optimistic locking | `genossi_dao_impl_sqlite/src/application.rs` (1-234) | exact |
| `genossi_dao_impl_sqlite/src/assembly_member_snapshot.rs` | DAO impl | batch-insert + count | `genossi_dao_impl_sqlite/src/application.rs` (87-156) | role-match (insert-only, ohne update/version) |
| `genossi_service/src/assembly.rs` | service trait + DTOs | request-response | `genossi_service/src/application.rs` (1-154) | exact |
| `genossi_service_impl/src/assembly.rs` | service impl | CRUD + lifecycle + audit | `genossi_service_impl/src/application.rs` (1-402) | exact |
| `genossi_rest_types/src/lib.rs` (append AssemblyTO/Status/Requests) | DTOs | request-response | `genossi_rest_types/src/lib.rs` (804-1000, ApplicationTO-Block) | exact |
| `genossi_rest/src/assembly.rs` | REST handler | request-response | `genossi_rest/src/application.rs` (208-491) | exact |
| `genossi_rest/src/lib.rs` (modify: mod, ApiDoc, create_app bound, nest, start_server bound) | router config | request-response | `genossi_rest/src/lib.rs` selbst (Application-Wiring Zeilen 1, 250, 410-415, 559-562, 674-680) | exact (Self-Pattern) |
| `genossi_bin/src/lib.rs` (modify: type-aliases, Deps-struct, RestStateImpl-field, ::new(), AssemblyRestState-impl) | DI wiring | request-response | `genossi_bin/src/lib.rs` selbst (Application-Block 122-144, 308, 409+455, 533, 976-997) | exact (Self-Pattern) |
| `genossi_bin/tests/e2e_tests.rs` (append `test_assembly_lifecycle_audit_chain_intact`) | test | e2e HTTP | `genossi_bin/tests/e2e_tests.rs` (7499-7523, `test_audit_verify_after_operations`) | exact |
| `genossi_dao/src/lib.rs` (modify: mod-decl) | wiring | n/a | trivial | n/a |
| `genossi_dao_impl_sqlite/src/lib.rs` (modify: mod-decl) | wiring | n/a | trivial | n/a |
| `genossi_service/src/lib.rs` (modify: mod-decl) | wiring | n/a | trivial | n/a |
| `genossi_service_impl/src/lib.rs` (modify: mod-decl) | wiring | n/a | trivial | n/a |

---

## Pattern Assignments

### `migrations/sqlite/<ts>_create_assembly_table.sql` (migration, DDL)

**Analog:** `migrations/sqlite/20260413000000_create_application_table.sql`

**Full file as template** (Zeilen 1-19) — kompletter Aufbau wird übernommen, Tabellenname ändert sich:

```sql
CREATE TABLE IF NOT EXISTS application (
    id BLOB PRIMARY KEY NOT NULL,
    first_name TEXT NOT NULL,
    last_name TEXT NOT NULL,
    -- ... domain fields ...
    status TEXT NOT NULL DEFAULT 'Offen',
    created TEXT NOT NULL,
    deleted TEXT,
    version BLOB NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_application_status ON application(status);
CREATE INDEX IF NOT EXISTS idx_application_deleted ON application(deleted);
```

**Delta:** Default-Status englisch (`'Preparation'` statt `'Offen'` per D-06). Felder pro D-05: `name`, `date`, `location`, `status`, `opened_at`, `closed_at`, plus Standard `id`/`created`/`deleted`/`version`. Zusätzlicher Index auf `date` per RESEARCH §1.

---

### `migrations/sqlite/<ts>_create_assembly_member_snapshot_table.sql` (migration, DDL)

**Analog:** `migrations/sqlite/20260413000000_create_application_table.sql` (für allgemeine Migration-Struktur). Composite-PK + FK ist projekt-neu — gerechtfertigt durch RESEARCH §1.

**Pattern delta:** keine `id`/`version`/`deleted`/`created`-Spalten (Snapshot ist immutable per D-01). Stattdessen `PRIMARY KEY (assembly_id, member_id)` und `FOREIGN KEY` mit SQLite-Default `NO ACTION` (Soft-Delete-Norm respektiert).

**Skelett aus RESEARCH §1:**
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

---

### `genossi_dao/src/assembly.rs` (entity + trait + auditable, request-response)

**Analog:** `genossi_dao/src/application.rs`

**Imports pattern** (Zeilen 1-7):
```rust
use async_trait::async_trait;
use mockall::automock;
use std::sync::Arc;
use uuid::Uuid;

use crate::DaoError;
```
Delta: zusätzlich nichts; falls `Salutation` nicht gebraucht wird, weglassen.

**Status-Enum + Default** (Zeilen 9-42, exakte Vorlage für `AssemblyStatus`):
```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ApplicationStatus {
    Offen,
    Bestaetigt,
    Abgelehnt,
}

impl ApplicationStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            ApplicationStatus::Offen => "Offen",
            ApplicationStatus::Bestaetigt => "Bestaetigt",
            ApplicationStatus::Abgelehnt => "Abgelehnt",
        }
    }

    pub fn from_str(s: &str) -> Result<Self, DaoError> {
        match s {
            "Offen" => Ok(ApplicationStatus::Offen),
            "Bestaetigt" => Ok(ApplicationStatus::Bestaetigt),
            "Abgelehnt" => Ok(ApplicationStatus::Abgelehnt),
            _ => Err(DaoError::ParseError(Arc::from(format!(
                "Unknown application status: {}",
                s
            )))),
        }
    }
}

impl Default for ApplicationStatus {
    fn default() -> Self {
        ApplicationStatus::Offen
    }
}
```
Delta: englische Werte `Preparation`/`Open`/`Closed` (D-06, D-17, Pitfall 4). Default = `Preparation`.

**Auditable-Impl** (Zeilen 63-96, RESEARCH §3 Adaption mit ISO8601-DateTime-Formatter):
```rust
impl crate::auditable::Auditable for ApplicationEntity {
    fn entity_type() -> &'static str {
        "application"
    }

    fn entity_id(&self) -> Uuid {
        self.id
    }

    fn audit_fields(&self) -> Vec<(&'static str, Option<String>)> {
        vec![
            ("first_name", Some(self.first_name.to_string())),
            ("last_name", Some(self.last_name.to_string())),
            ("salutation", self.salutation.as_ref().map(|s| s.as_str().to_string())),
            // ... weitere Felder ...
            ("status", Some(self.status.as_str().to_string())),
        ]
    }
}
```
Delta: `entity_type() = "assembly"`. `audit_fields()` exakt 6 Felder per D-10: `name`, `date`, `location`, `status`, `opened_at`, `closed_at`. Optional-DateTime-Felder benötigen Format-Closure (RESEARCH §3, Beispiel 3): `let format_dt = |dt: &time::PrimitiveDateTime| dt.assume_utc().format(&Iso8601::DEFAULT).unwrap_or_default()`.

**DAO-Trait mit Default-Methoden** (Zeilen 98-140):
```rust
#[automock(type Transaction = crate::MockTransaction;)]
#[async_trait]
pub trait ApplicationDao {
    type Transaction: crate::Transaction;

    async fn dump_all(&self, tx: Self::Transaction) -> Result<Arc<[ApplicationEntity]>, DaoError>;
    async fn create(&self, entity: &ApplicationEntity, process: &str, tx: Self::Transaction) -> Result<(), DaoError>;
    async fn update(&self, entity: &ApplicationEntity, process: &str, tx: Self::Transaction) -> Result<(), DaoError>;

    async fn all(&self, tx: Self::Transaction) -> Result<Arc<[ApplicationEntity]>, DaoError> {
        let all_entities = self.dump_all(tx).await?;
        let active_entities: Vec<ApplicationEntity> = all_entities
            .iter()
            .filter(|e| e.deleted.is_none())
            .cloned()
            .collect();
        Ok(active_entities.into())
    }

    async fn find_by_id(&self, id: Uuid, tx: Self::Transaction) -> Result<Option<ApplicationEntity>, DaoError> {
        let all_entities = self.dump_all(tx).await?;
        Ok(all_entities
            .iter()
            .find(|e| e.id == id && e.deleted.is_none())
            .cloned())
    }
}
```
Delta: `AssemblyDao` ist 1:1 strukturgleich.

**Test-Pattern für Auditable-Field-Count** (Zeilen 170-185):
```rust
#[test]
fn test_auditable_fields_count() {
    let entity = make_application();
    let fields = entity.audit_fields();
    assert_eq!(fields.len(), 11);
    let field_names: Vec<&str> = fields.iter().map(|(name, _)| *name).collect();
    assert!(!field_names.contains(&"id"));
    assert!(!field_names.contains(&"version"));
    assert!(!field_names.contains(&"created"));
    assert!(!field_names.contains(&"deleted"));
}
```
Delta: `assert_eq!(fields.len(), 6)` für Assembly. Plus zweiter Test für Status-Roundtrip (`from_str("Open").unwrap() == AssemblyStatus::Open`, Pitfall 4).

---

### `genossi_dao/src/assembly_member_snapshot.rs` (entity + trait, batch-insert + lookup)

**Analog:** `genossi_dao/src/application.rs` (98-140, abgespecktes Trait)

**Pattern delta:** Snapshot-Entity hat KEINE `id`/`version`/`deleted`/`created`-Spalten — Felder per D-01: `assembly_id`, `member_id`, `captured_at`. Trait nutzt `#[automock]` wie alle DAOs, hat aber **keine** `dump_all`/`update`-Methoden. Methoden gemäß RESEARCH §5: `create`, `create_batch` (optional), `find_by_assembly_id`, `count_by_assembly_id`. Kein Auditable-Trait-Impl (Pitfall 1: Snapshot ist Daten, kein Lifecycle-Event).

**Imports** wie `application.rs:1-7`. Trait-Skelett aus RESEARCH §5:
```rust
#[automock(type Transaction = crate::MockTransaction;)]
#[async_trait]
pub trait AssemblyMemberSnapshotDao {
    type Transaction: crate::Transaction;
    async fn create(&self, entity: &AssemblyMemberSnapshotEntity, process: &str, tx: Self::Transaction) -> Result<(), DaoError>;
    async fn create_batch(&self, entities: &[AssemblyMemberSnapshotEntity], process: &str, tx: Self::Transaction) -> Result<(), DaoError>;
    async fn find_by_assembly_id(&self, assembly_id: Uuid, tx: Self::Transaction) -> Result<Arc<[AssemblyMemberSnapshotEntity]>, DaoError>;
    async fn count_by_assembly_id(&self, assembly_id: Uuid, tx: Self::Transaction) -> Result<u64, DaoError>;
}
```

---

### `genossi_dao_impl_sqlite/src/assembly.rs` (DAO impl, CRUD + optimistic locking)

**Analog:** `genossi_dao_impl_sqlite/src/application.rs`

**parse_datetime + DB-row TryFrom-Pattern** (Zeilen 12-75, exakte Vorlage):
```rust
fn parse_datetime(s: &str) -> Result<PrimitiveDateTime, time::error::Parse> {
    if let Ok(dt) = PrimitiveDateTime::parse(s, &time::format_description::well_known::Iso8601::DEFAULT) {
        return Ok(dt);
    }
    let sqlite_format = time::format_description::parse(
        "[year]-[month]-[day] [hour]:[minute]:[second].[subsecond]",
    ).unwrap();
    if let Ok(dt) = PrimitiveDateTime::parse(s, &sqlite_format) {
        return Ok(dt);
    }
    let sqlite_simple = time::format_description::parse(
        "[year]-[month]-[day] [hour]:[minute]:[second]",
    ).unwrap();
    PrimitiveDateTime::parse(s, &sqlite_simple)
}

#[derive(Debug, sqlx::FromRow)]
struct ApplicationDb {
    id: Vec<u8>,
    first_name: String,
    // ...
    status: String,
    created: String,
    deleted: Option<String>,
    version: Vec<u8>,
}

impl TryFrom<&ApplicationDb> for ApplicationEntity { /* ... */ }
```
Delta: `AssemblyDb` mit `name: String, date: String, location: Option<String>, status: String, opened_at: Option<String>, closed_at: Option<String>, created: String, deleted: Option<String>, version: Vec<u8>, id: Vec<u8>`.

**create-Method** (Zeilen 107-156, INSERT-Pattern):
```rust
async fn create(&self, entity: &ApplicationEntity, _process: &str, tx: Self::Transaction) -> Result<(), DaoError> {
    let id = entity.id.as_bytes().to_vec();
    let version = entity.version.as_bytes().to_vec();
    let format = &time::format_description::well_known::Iso8601::DEFAULT;
    let created = entity.created.assume_utc().format(format)
        .map_err(|e| DaoError::ParseError(Arc::from(e.to_string())))?;
    // ... bind-vars ...

    sqlx::query(
        "INSERT INTO application (id, first_name, ..., status, created, version) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(id)
    // ... binds in column order ...
    .execute(tx.tx.lock().await.as_mut())
    .await
    .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?;
    Ok(())
}
```
Delta: Kolumnen analog Migration; `opened_at` und `closed_at` werden als Option<formatted-String> gebunden.

**update-Method mit Optimistic Locking** (Zeilen 158-233, kritischer Pattern):
```rust
async fn update(&self, entity: &ApplicationEntity, _process: &str, tx: Self::Transaction) -> Result<(), DaoError> {
    let id = entity.id.as_bytes().to_vec();
    let old_version = entity.version.as_bytes().to_vec();
    let new_version = Uuid::new_v4().as_bytes().to_vec();
    // ... bindings ...

    let exists = sqlx::query_scalar::<_, i32>(
        "SELECT COUNT(*) FROM application WHERE id = ? AND deleted IS NULL",
    )
    .bind(id.clone())
    .fetch_one(tx.tx.lock().await.as_mut())
    .await
    .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?;
    if exists == 0 { return Err(DaoError::NotFound); }

    let rows_affected = sqlx::query(
        "UPDATE application SET first_name = ?, ..., version = ? \
         WHERE id = ? AND version = ? AND deleted IS NULL",
    )
    /* binds */
    .bind(new_version).bind(id).bind(old_version)
    .execute(tx.tx.lock().await.as_mut())
    .await
    .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?
    .rows_affected();
    if rows_affected == 0 {
        return Err(DaoError::ConflictError(Arc::from("Version mismatch")));
    }
    Ok(())
}
```

---

### `genossi_dao_impl_sqlite/src/assembly_member_snapshot.rs` (DAO impl, batch-insert + count)

**Analog:** `genossi_dao_impl_sqlite/src/application.rs` (87-156, nur Insert-Teil)

**Pattern delta:** Kein `update`. `create_batch` baut eine `INSERT ... VALUES (?,?,?), (?,?,?), ...` mit dynamischer Placeholder-Generation. `count_by_assembly_id` nutzt `SELECT COUNT(*) FROM assembly_member_snapshot WHERE assembly_id = ?` (D-04). Reuse `parse_datetime`-Helper aus `assembly.rs` Modul. SQLite-Constraint-Verstoss bei doppelter `(assembly_id, member_id)` → `DaoError::DatabaseError` (Pitfall 5: erwünscht, schlägt Tx zurück).

---

### `genossi_service/src/assembly.rs` (service trait + DTOs, request-response)

**Analog:** `genossi_service/src/application.rs`

**Imports + DTO-Pattern** (Zeilen 1-72):
```rust
use async_trait::async_trait;
use genossi_dao::application::{ApplicationEntity, ApplicationStatus};
use mockall::automock;
use std::fmt::Debug;
use std::sync::Arc;
use uuid::Uuid;

use crate::ServiceError;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Application {
    pub id: Uuid,
    pub first_name: Arc<str>,
    // ...
    pub status: ApplicationStatus,
    pub created: time::PrimitiveDateTime,
    pub deleted: Option<time::PrimitiveDateTime>,
    pub version: Uuid,
}

impl From<&ApplicationEntity> for Application { /* clone-fields */ }
impl From<&Application> for ApplicationEntity { /* clone-fields */ }
```
Delta: `Assembly` mit Feldern aus D-05; `From<&AssemblyEntity> for Assembly` und Rückrichtung.

**Submission/Update-Inputs** (Zeilen 74-103):
```rust
#[derive(Clone, Debug)]
pub struct ApplicationSubmission { /* domain fields, no version */ }

#[derive(Clone, Debug)]
pub struct ApplicationUpdate {
    /* domain fields */
    pub version: Uuid,  // Optimistic Locking
}
```
Delta: `AssemblySubmission { name, date, location }`. `AssemblyUpdate { name, date, location, version }`. Plus `AssemblyDetail { assembly: Assembly, snapshot_member_count: u64 }` (RESEARCH §6).

**Service-Trait** (Zeilen 105-154):
```rust
#[automock(type Context=(); type Transaction = genossi_dao::MockTransaction;)]
#[async_trait]
pub trait ApplicationService {
    type Context: Clone + Debug + PartialEq + Eq + Send + Sync + 'static;
    type Transaction: genossi_dao::Transaction;

    async fn submit(&self, submission: &ApplicationSubmission, send_mail: bool) -> Result<Application, ServiceError>;
    async fn list(&self, status_filter: Option<ApplicationStatus>, context: crate::permission::Authentication<Self::Context>) -> Result<Arc<[Application]>, ServiceError>;
    async fn get(&self, id: Uuid, context: crate::permission::Authentication<Self::Context>) -> Result<Application, ServiceError>;
    async fn confirm(&self, id: Uuid, context: crate::permission::Authentication<Self::Context>) -> Result<Application, ServiceError>;
    async fn reject(&self, id: Uuid, context: crate::permission::Authentication<Self::Context>) -> Result<Application, ServiceError>;
    async fn update_application(&self, id: Uuid, update: &ApplicationUpdate, context: crate::permission::Authentication<Self::Context>) -> Result<Application, ServiceError>;
}
```
Delta: Methodennamen per RESEARCH §6 — `create_assembly`, `update_assembly`, `open_assembly`, `close_assembly`, `get_assembly` (returnt `AssemblyDetail`), `get_all_assemblies`. Alle erfordern `context: Authentication<Self::Context>` (kein public-submit wie bei Application).

---

### `genossi_service_impl/src/assembly.rs` (service impl, CRUD + lifecycle + audit)

**Analog:** `genossi_service_impl/src/application.rs`

**Imports + Process-Konstanten** (Zeilen 1-21):
```rust
use async_trait::async_trait;
use genossi_dao::application::{ApplicationDao, ApplicationStatus};
use genossi_dao::audit_log::AuditLogDao;
use genossi_dao::TransactionDao;
use genossi_service::application::{Application, ApplicationService, ApplicationSubmission, ApplicationUpdate};
use genossi_service::permission::{Authentication, PermissionService};
use genossi_service::uuid_service::UuidService;
use genossi_service::{ServiceError, ValidationFailureItem};
use std::sync::Arc;
use uuid::Uuid;

use crate::gen_service_impl;

const APPLICATION_SERVICE_PROCESS: &str = "application-service";
const MANAGE_MEMBERS_PRIVILEGE: &str = "manage_members";
```
Delta: `ASSEMBLY_PROCESS_CREATE = "assembly.create"`, `ASSEMBLY_PROCESS_OPEN = "assembly.open"`, `ASSEMBLY_PROCESS_CLOSE = "assembly.close"`, `ASSEMBLY_PROCESS_UPDATE = "assembly.update"` (RESEARCH §6 / D-11). `ADMIN_PRIVILEGE = "admin"` (D-14).

**`gen_service_impl!`-Block** (Zeilen 23-35):
```rust
gen_service_impl! {
    struct ApplicationServiceImpl: ApplicationService = ApplicationServiceDeps {
        ApplicationDao: ApplicationDao<Transaction = Self::Transaction> = application_dao,
        AuditLogDao: AuditLogDao<Transaction = Self::Transaction> = audit_log_dao,
        MemberDao: MemberDao<Transaction = Self::Transaction> = member_dao,
        MemberActionDao: MemberActionDao<Transaction = Self::Transaction> = member_action_dao,
        PermissionService: PermissionService<Context = Self::Context> = permission_service,
        UuidService: UuidService = uuid_service,
        TransactionDao: TransactionDao<Transaction = Self::Transaction> = transaction_dao,
        ConfigService: genossi_config::service::ConfigService = config_service,
        MailService: genossi_mail::service::MailService = mail_service,
    }
}
```
Delta (RESEARCH §6): Deps für Assembly: `AssemblyDao`, `AssemblyMemberSnapshotDao`, `MemberDao` (für Snapshot-Befüllung), `AuditLogDao`, `PermissionService`, `UuidService`, `TransactionDao`. Kein ConfigService/MailService nötig.

**create-Method-Pattern** (Zeilen 145-222) — Vorlage für `create_assembly`:
```rust
let tx = self.transaction_dao.use_transaction(None).await?;
let now = time::OffsetDateTime::now_utc();
let created = time::PrimitiveDateTime::new(now.date(), now.time());

let entity = genossi_dao::application::ApplicationEntity {
    id: self.uuid_service.new_v4().await,
    /* ... fields from submission ... */
    status: ApplicationStatus::Offen,
    created,
    deleted: None,
    version: self.uuid_service.new_v4().await,
};

crate::audited_create!(self, self.application_dao, &entity, APPLICATION_SERVICE_PROCESS, "PUBLIC", tx);

self.transaction_dao.commit(tx).await?;
let app = Application::from(&entity);
Ok(app)
```
Delta für `create_assembly`: status = `AssemblyStatus::Preparation`, `opened_at: None`, `closed_at: None`. Process-string `"assembly.create"`, user_id über `permission_service.current_user_id(context.clone())` statt fix `"PUBLIC"` (D-14: alles admin-only).

**Lifecycle-Method mit Status-Guard + Audit** (Zeilen 268-402, `confirm`-Method als kritische Vorlage für `open_assembly`/`close_assembly`):
```rust
async fn confirm(&self, id: Uuid, context: Authentication<Self::Context>) -> Result<Application, ServiceError> {
    let tx = self.transaction_dao.use_transaction(None).await?;

    let user_id = self.permission_service
        .current_user_id(context.clone())
        .await?
        .unwrap_or_else(|| "SYSTEM".to_string());

    self.permission_service
        .check_permission(MANAGE_MEMBERS_PRIVILEGE, context)
        .await?;

    let mut entity = self.application_dao
        .find_by_id(id, tx.clone())
        .await?
        .ok_or(ServiceError::EntityNotFound(id))?;

    if entity.status != ApplicationStatus::Offen {
        return Err(ServiceError::Conflict(Arc::from(format!(
            "Application status is '{}', expected 'Offen'",
            entity.status.as_str()
        ))));
    }

    /* domain mutations */
    entity.status = ApplicationStatus::Bestaetigt;
    crate::audited_update!(self, self.application_dao, id, &entity, APPLICATION_SERVICE_PROCESS, &user_id, tx);

    self.transaction_dao.commit(tx).await?;
    Ok(Application::from(&entity))
}
```
Delta für `open_assembly` (RESEARCH §10, Pitfall 2): Status-Guard `entity.status != AssemblyStatus::Preparation`. Mutate (`status = Open`, `opened_at = now_pdt`). `audited_update!` mit Process `"assembly.open"`. **Atomar** in derselben Tx: Snapshot-Befüllung mit Filter aus RESEARCH §4 (`member_dao.all` → filter `is_normal()` + `exit_date.map_or(true, |d| d > opened_date)` → `assembly_member_snapshot_dao.create_batch(...)`). EINE Tx, ein Commit am Ende. Delta für `close_assembly`: Guard `!= Open`, mutate `status = Closed, closed_at = now_pdt`, Process `"assembly.close"`, kein Snapshot-Block.

---

### `genossi_rest_types/src/lib.rs` (modify: append AssemblyStatusTO/AssemblyTO/Requests)

**Analog:** `genossi_rest_types/src/lib.rs:804-1000` (Application-Block)

**Status-TO + From-Impls** (Zeilen 806-831, exakte Vorlage):
```rust
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum ApplicationStatusTO {
    Offen,
    Bestaetigt,
    Abgelehnt,
}

impl From<&ApplicationStatus> for ApplicationStatusTO {
    fn from(s: &ApplicationStatus) -> Self {
        match s {
            ApplicationStatus::Offen => ApplicationStatusTO::Offen,
            ApplicationStatus::Bestaetigt => ApplicationStatusTO::Bestaetigt,
            ApplicationStatus::Abgelehnt => ApplicationStatusTO::Abgelehnt,
        }
    }
}

impl From<&ApplicationStatusTO> for ApplicationStatus {
    fn from(s: &ApplicationStatusTO) -> Self {
        match s {
            ApplicationStatusTO::Offen => ApplicationStatus::Offen,
            ApplicationStatusTO::Bestaetigt => ApplicationStatus::Bestaetigt,
            ApplicationStatusTO::Abgelehnt => ApplicationStatus::Abgelehnt,
        }
    }
}
```
Delta: `AssemblyStatusTO { Preparation, Open, Closed }` (D-17). Bidirektionale `From`-Impls zwischen `AssemblyStatus` und `AssemblyStatusTO`.

**TO-Struct mit ISO8601-Datetime-Serde** (Zeilen 833-877):
```rust
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct ApplicationTO {
    #[schema(example = "123e4567-e89b-12d3-a456-426614174000")]
    pub id: Uuid,
    #[schema(example = "Max")]
    pub first_name: String,
    /* ... */
    pub status: ApplicationStatusTO,
    #[serde(
        serialize_with = "iso8601_datetime::serialize",
        deserialize_with = "iso8601_datetime::deserialize",
        default
    )]
    pub created: Option<time::PrimitiveDateTime>,
    #[serde(
        serialize_with = "iso8601_datetime::serialize",
        deserialize_with = "iso8601_datetime::deserialize",
        default
    )]
    pub deleted: Option<time::PrimitiveDateTime>,
    pub version: Option<Uuid>,
}

impl From<&genossi_service::application::Application> for ApplicationTO {
    fn from(a: &genossi_service::application::Application) -> Self {
        Self {
            id: a.id,
            /* ... */
            status: ApplicationStatusTO::from(&a.status),
            created: Some(a.created),
            deleted: a.deleted,
            version: Some(a.version),
        }
    }
}
```
Delta: `AssemblyTO` mit Feldern: `id`, `name`, `date` (Option<PrimitiveDateTime> mit iso8601_datetime), `location`, `status`, `opened_at`, `closed_at` (alle Datetime-Felder mit `iso8601_datetime`-Serde), `created`, `deleted`, `version`. Plus `AssemblyDetailTO { assembly: AssemblyTO, snapshot_member_count: u64 }`. `From<&genossi_service::assembly::Assembly>`-Impl und `From<&AssemblyDetail>`-Impl.

**Request-DTOs** (Zeilen 942-1000, Vorlage `AdminCreateApplicationRequest`/`UpdateApplicationRequest`):
Delta: `CreateAssemblyRequest { name, date, location }`. `UpdateAssemblyRequest { name, date, location, version }`. Beide mit `ToSchema`-Derive und ISO8601-Date-Field.

---

### `genossi_rest/src/assembly.rs` (REST handler, request-response)

**Analog:** `genossi_rest/src/application.rs`

**Imports + RestState-Trait** (Zeilen 1-33):
```rust
use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::HeaderMap,
    response::Response,
    routing::{get, post},
    Extension, Json, Router,
};
use genossi_dao::application::ApplicationStatus;
use genossi_rest_types::{
    AdminCreateApplicationRequest, ApplicationStatusTO, ApplicationTO, /*...*/
};
use genossi_service::application::{ApplicationService, ApplicationSubmission, ApplicationUpdate};
use std::sync::Arc;
use tracing::instrument;
use utoipa::OpenApi;
use uuid::Uuid;

use crate::{error_handler, Context, RestError, RestStateDef};

pub trait ApplicationRestState: Clone + Send + Sync + 'static {
    type ApplicationService: ApplicationService<Context = crate::ContextType>
        + Send + Sync + 'static;
    fn application_service(&self) -> Arc<Self::ApplicationService>;
    fn get_config_value(&self, key: &str) -> std::pin::Pin<Box<dyn std::future::Future<Output = Option<String>> + Send + '_>>;
}
```
Delta: `AssemblyRestState` ohne `get_config_value` (kein Config nötig). Nur `assembly_service(&self) -> Arc<Self::AssemblyService>`.

**Validation-Helper** (Zeilen 42-113):
```rust
fn validate_required_field(errors: &mut Vec<ValidationFailureItem>, field: &str, value: &str, max_len: usize) {
    if value.is_empty() {
        errors.push(ValidationFailureItem { field: field.to_string(), message: "missing".to_string() });
    } else if value.len() > max_len {
        errors.push(ValidationFailureItem { field: field.to_string(), message: format!("too long (max {})", max_len) });
    }
}

pub fn validate_join_request(body: &PublicJoinRequest) -> Result<(), Vec<ValidationFailureItem>> {
    let mut errors = Vec::new();
    validate_required_field(&mut errors, "first_name", &body.first_name, 128);
    /* ... */
    if errors.is_empty() { Ok(()) } else { Err(errors) }
}
```
Delta: `validate_assembly_request(name)` mit `name` not-empty + max-len 256, `location` optional max-len 256.

**Handler-Pattern mit `error_handler` + `#[utoipa::path]` + `#[instrument]`** (Zeilen 208-256, `list_applications` als exakte Vorlage):
```rust
#[instrument(skip(rest_state))]
#[utoipa::path(
    get,
    tag = "Applications",
    path = "",
    params(("status" = Option<String>, Query, description = "Filter")),
    responses(
        (status = 200, description = "List applications", body = [ApplicationTO]),
        (status = 401, description = "Unauthorized"),
    ),
)]
pub async fn list_applications<RestState: RestStateDef + ApplicationRestState>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Query(query): Query<ApplicationListQuery>,
) -> Response {
    error_handler(
        (async {
            let apps: Arc<[ApplicationTO]> = rest_state
                .application_service()
                .list(status_filter, crate::extract_auth_context(Some(context))?)
                .await?
                .iter()
                .map(ApplicationTO::from)
                .collect();
            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&apps)?))
                .unwrap())
        })
        .await,
    )
}
```
Delta: 6 Handler per RESEARCH §9: `list_assemblies` (GET `/`), `create_assembly` (POST `/`, status 201), `get_assembly` (GET `/{id}` returnt `AssemblyDetailTO`), `update_assembly` (PUT `/{id}`), `open_assembly` (POST `/{id}/open`), `close_assembly` (POST `/{id}/close`). Alle mit `tag = "Assemblies"`. Alle mit `Extension<Context>` und `crate::extract_auth_context(Some(context))?`.

**Router-Generator** (Zeilen 479-491):
```rust
pub fn generate_route<RestState: RestStateDef + ApplicationRestState>() -> Router<RestState> {
    Router::new()
        .route("/", get(list_applications::<RestState>).post(create_application::<RestState>))
        .route("/{id}", get(get_application::<RestState>).put(update_application::<RestState>))
        .route("/{id}/confirm", post(confirm_application::<RestState>))
        .route("/{id}/reject", post(reject_application::<RestState>))
}
```
Delta: routes pro D-13 — `/` (GET+POST), `/{id}` (GET+PUT), `/{id}/open` (POST), `/{id}/close` (POST).

**ApiDoc** (Zeilen 493-511):
```rust
#[derive(OpenApi)]
#[openapi(
    paths(list_applications, create_application, get_application, update_application, confirm_application, reject_application),
    components(schemas(ApplicationTO, ApplicationStatusTO, AdminCreateApplicationRequest, UpdateApplicationRequest, PublicJoinResponse))
)]
pub struct ApiDoc;
```
Delta: `paths(list_assemblies, create_assembly, get_assembly, update_assembly, open_assembly, close_assembly)`. `components(schemas(AssemblyTO, AssemblyStatusTO, AssemblyDetailTO, CreateAssemblyRequest, UpdateAssemblyRequest))`.

**Validation-Tests** (Zeilen 525-616) — Vorlage für `mod tests` mit `valid_request` + Feldfehler-Tests; Adaption für `validate_assembly_request`.

---

### `genossi_rest/src/lib.rs` (modify: register module + ApiDoc + bound + nest)

**Analog (Self-Pattern):** Eigene Datei, Application-Wiring an mehreren Stellen.

**Module-Declaration** (Zeile 1):
```rust
pub mod application;
```
Delta: `pub mod assembly;` einfügen (alphabetisch zwischen `application` und `audit_log`).

**ApiDoc nest** (Zeile 234-256):
```rust
nest(
    (path = "/api/auth", api = auth::ApiDoc),
    /* ... */
    (path = "/api/applications", api = application::ApiDoc),
    (path = "/api/audit", api = audit_log::ApiDoc),
    /* ... */
)
```
Delta: neue Zeile `(path = "/api/assembly", api = assembly::ApiDoc),`.

**create_app Type-Bound** (Zeile 410-415):
```rust
pub async fn create_app<
    RestState: RestStateDef
        + public_stats::PublicStatsState
        + application::ApplicationRestState
        + audit_log::AuditRestState
        + audit_timestamp::TimestampRestState,
>(
    rest_state: RestState,
) -> Router {
```
Delta: `+ assembly::AssemblyRestState`.

**Router .nest** (Zeilen 559-562):
```rust
.nest("/api/applications", application::generate_route::<RestState>())
.nest("/api/audit", audit_log::generate_route::<RestState>())
```
Delta: `.nest("/api/assembly", assembly::generate_route::<RestState>())`.

**start_server Type-Bound** (Zeile 674-680, gleiches Bound-Add wie create_app).

---

### `genossi_bin/src/lib.rs` (modify: type-aliases, Deps-struct, RestStateImpl, ::new(), AssemblyRestState-impl)

**Analog (Self-Pattern):** Application-Wiring an mehreren Stellen — exakter Spiegel.

**Type-Aliases + Deps-Struct** (Zeilen 122-144):
```rust
type ApplicationDao = genossi_dao_impl_sqlite::application::ApplicationDaoImpl;

pub struct ApplicationServiceDependencies;
unsafe impl Send for ApplicationServiceDependencies {}
unsafe impl Sync for ApplicationServiceDependencies {}

impl ApplicationServiceDeps for ApplicationServiceDependencies {
    type Context = Context;
    type Transaction = Transaction;
    type ApplicationDao = ApplicationDao;
    type AuditLogDao = AuditLogDao;
    type MemberDao = MemberDao;
    type MemberActionDao = MemberActionDao;
    type PermissionService = PermissionService;
    type UuidService = UuidService;
    type TransactionDao = TransactionDao;
    type ConfigService = ConfigService;
    type MailService = MailServiceType;
}

type ApplicationService =
    genossi_service_impl::application::ApplicationServiceImpl<ApplicationServiceDependencies>;
```
Delta: nahe Application-Block einfügen:
```rust
type AssemblyDao = genossi_dao_impl_sqlite::assembly::AssemblyDaoImpl;
type AssemblyMemberSnapshotDao = genossi_dao_impl_sqlite::assembly_member_snapshot::AssemblyMemberSnapshotDaoImpl;

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

**RestStateImpl-Field** (Zeile 308): `application_service: Arc<ApplicationService>,` → analog `assembly_service: Arc<AssemblyService>,` ergänzen.

**::new() Service-Wiring** (Zeilen 409, 455-466):
```rust
let application_dao = Arc::new(ApplicationDao::new(pool.clone()));
/* ... */
let application_service = Arc::new(genossi_service_impl::application::ApplicationServiceImpl {
    application_dao,
    audit_log_dao: audit_log_dao.clone(),
    member_dao: member_dao.clone(),
    member_action_dao: member_action_dao.clone(),
    permission_service: permission_service.clone(),
    uuid_service: uuid_service.clone(),
    transaction_dao: transaction_dao.clone(),
    config_service: config_service_for_app,
    mail_service: mail_service.clone(),
});
```
Delta: nahe Application-Block ergänzen:
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
Plus `assembly_service,` in `Self { ... }`-Block (Zeile ~533).

**RestState-Trait-Impl** (Zeilen 976-997, exakte Vorlage):
```rust
impl genossi_rest::application::ApplicationRestState for RestStateImpl {
    type ApplicationService = ApplicationService;

    fn application_service(&self) -> Arc<Self::ApplicationService> {
        self.application_service.clone()
    }

    fn get_config_value(&self, key: &str) -> std::pin::Pin<Box<dyn std::future::Future<Output = Option<String>> + Send + '_>> {
        let config_service = self.config_service.clone();
        let key = key.to_string();
        Box::pin(async move {
            use genossi_config::service::ConfigService;
            match config_service.get(&key).await {
                Ok(entry) => Some(entry.value.to_string()),
                Err(_) => None,
            }
        })
    }
}
```
Delta:
```rust
impl genossi_rest::assembly::AssemblyRestState for RestStateImpl {
    type AssemblyService = AssemblyService;
    fn assembly_service(&self) -> Arc<Self::AssemblyService> {
        self.assembly_service.clone()
    }
}
```

**`initialize_audit_snapshot()`-Erweiterung** (RESEARCH §12 Punkt 7): Optional, weil bei erstem Phase-1-Deploy noch keine Assemblies existieren. Empfehlung: Block analog Application-Block (ab Zeile ~681) hinzufügen, damit künftige Snapshots korrekt verbucht werden.

---

### `genossi_bin/tests/e2e_tests.rs` (modify: append assembly lifecycle test)

**Analog:** `genossi_bin/tests/e2e_tests.rs:7499-7523` (`test_audit_verify_after_operations`)

**setup() reuse** (Zeilen 23-37):
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
Wiederverwenden, nicht duplizieren.

**Audit-Verify-Test-Struktur** (Zeilen 7499-7523):
```rust
#[tokio::test]
async fn test_audit_verify_after_operations() {
    let server = setup().await;
    let client = reqwest::Client::new();

    let response = client
        .post(server.url("/api/members"))
        .json(&sample_member())
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let response = client
        .get(server.url("/api/audit/verify"))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let result: genossi_rest_types::VerifyResponseTO = response.json().await.unwrap();
    assert!(result.valid);
    assert!(result.total_entries > 0);
    assert!(result.broken_links.is_empty());
}
```
Delta (RESEARCH §11 + Code-Beispiel 6): Sequenz Create → Open → Close ausführen, dann Verify. Status-Codes: 201, 200, 200. Plus zusätzlich `GET /api/audit/{entity_type}/{id}` (oder Process-Filter) prüfen, dass alle drei Process-Strings (`assembly.create`, `assembly.open`, `assembly.close`) in den Einträgen vorkommen. `total_entries >= 3`.

**Negativ-Tests (empfohlen):** `test_open_assembly_from_closed_returns_conflict`, `test_close_assembly_from_preparation_returns_conflict` — folgen dem Pattern aus Pitfall 3.

---

### Module-Registrierungen (Trivial-Modifikationen)

| Datei | Delta |
|-------|-------|
| `genossi_dao/src/lib.rs` | `pub mod assembly; pub mod assembly_member_snapshot;` |
| `genossi_dao_impl_sqlite/src/lib.rs` | `pub mod assembly; pub mod assembly_member_snapshot;` |
| `genossi_service/src/lib.rs` | `pub mod assembly;` |
| `genossi_service_impl/src/lib.rs` | `pub mod assembly;` |

Pattern: alphabetische Einsortierung zwischen `application` und `audit_log` (Stil-Hinweis: `genossi_rest/src/lib.rs:1-22` zeigt sortierte Liste).

---

## Shared Patterns

### Authentication / Permission-Check
**Source:** `genossi_service_impl/src/application.rs:281-283` (Service-Layer-Check) und `genossi_rest/src/application.rs:241` (REST-Handler-Extract)
**Apply to:** Alle 6 Assembly-Handler + alle 6 Service-Methoden

```rust
// Im Service:
let user_id = self.permission_service
    .current_user_id(context.clone())
    .await?
    .unwrap_or_else(|| "SYSTEM".to_string());

self.permission_service
    .check_permission(MANAGE_MEMBERS_PRIVILEGE, context)
    .await?;
```
**Delta für Assembly:** Privilege-String `"admin"` per D-14, nicht `"manage_members"`.

```rust
// Im REST-Handler:
crate::extract_auth_context(Some(context))?
```

### Audit-Logging
**Source:** `genossi_service_impl/src/audit_macros.rs` (audited_create/audited_update/audited_delete)
**Apply to:** `create_assembly` (audited_create!), `open_assembly`/`close_assembly`/`update_assembly` (audited_update!)

```rust
// Vorlage genossi_service_impl/src/application.rs:204-211
crate::audited_create!(
    self,
    self.application_dao,
    &entity,
    APPLICATION_SERVICE_PROCESS,
    "PUBLIC",
    tx
);

// Vorlage genossi_service_impl/src/application.rs:390-398
crate::audited_update!(
    self,
    self.application_dao,
    id,
    &entity,
    APPLICATION_SERVICE_PROCESS,
    &user_id,
    tx
);
```
**Delta:** Process-String pro Lifecycle-Action: `"assembly.create"`, `"assembly.open"`, `"assembly.close"`, `"assembly.update"` (D-11). user_id immer aus `current_user_id`, nicht `"PUBLIC"`. **Nicht** den Snapshot-Insert auditieren (Pitfall 1).

### Error Handling
**Source:** `genossi_service_impl/src/application.rs:291-296` (Status-Conflict) und `genossi_rest/src/lib.rs` (RestError → HTTP-Status)
**Apply to:** Alle Lifecycle-Methoden (open/close/update) brauchen Status-Guard

```rust
if entity.status != ApplicationStatus::Offen {
    return Err(ServiceError::Conflict(Arc::from(format!(
        "Application status is '{}', expected 'Offen'",
        entity.status.as_str()
    ))));
}
```
**Delta:** Per Lifecycle:
- `open_assembly`: `entity.status != AssemblyStatus::Preparation`
- `close_assembly`: `entity.status != AssemblyStatus::Open`
- `update_assembly`: `entity.status != AssemblyStatus::Preparation`

`ServiceError::Conflict` → HTTP 409.

### Transaction-Atomarität
**Source:** `genossi_service_impl/src/application.rs:273+400` (Tx-Begin + EINMAL-Commit am Ende, `tx.clone()` für Sub-Calls)
**Apply to:** Besonders kritisch in `open_assembly` (Pitfall 2)

```rust
let tx = self.transaction_dao.use_transaction(None).await?;
/* alle Sub-Calls mit tx.clone() */
self.transaction_dao.commit(tx).await?;
```
**Delta für `open_assembly`:** Update + Audit + Snapshot-Inserts MÜSSEN in derselben Tx, ein Commit ganz am Ende.

### ISO8601-Datetime-Serde
**Source:** `genossi_rest_types/src/lib.rs:864-876` (iso8601_datetime-Modul)
**Apply to:** `AssemblyTO`, `CreateAssemblyRequest`, `UpdateAssemblyRequest`, `AssemblyDetailTO`

```rust
#[serde(
    serialize_with = "iso8601_datetime::serialize",
    deserialize_with = "iso8601_datetime::deserialize",
    default
)]
pub created: Option<time::PrimitiveDateTime>,
```

### Optimistic-Locking
**Source:** `genossi_dao_impl_sqlite/src/application.rs:202-230`
**Apply to:** `AssemblyDaoImpl::update`

```rust
let rows_affected = sqlx::query("UPDATE ... WHERE id = ? AND version = ? AND deleted IS NULL")
    /* binds */
    .bind(new_version).bind(id).bind(old_version)
    .execute(...).await?
    .rows_affected();
if rows_affected == 0 {
    return Err(DaoError::ConflictError(Arc::from("Version mismatch")));
}
```

---

## No Analog Found

Keine — alle 17 Files haben einen direkten oder strukturell-passenden Analog im Bestand. Snapshot-DAO ist der einzige Punkt, an dem das übliche Aggregat-Pattern abgespeckt wird (kein version, kein soft-delete, kein Auditable), aber die SQLx-Insert-Logik selbst übernimmt das `ApplicationDaoImpl::create`-Pattern direkt.

---

## Metadata

**Analog search scope:**
- `genossi_dao/src/` (application.rs, member.rs, auditable.rs)
- `genossi_dao_impl_sqlite/src/` (application.rs)
- `genossi_service/src/` (application.rs)
- `genossi_service_impl/src/` (application.rs, audit_macros.rs, macros.rs)
- `genossi_rest/src/` (application.rs, lib.rs)
- `genossi_rest_types/src/lib.rs`
- `genossi_bin/src/lib.rs`
- `genossi_bin/tests/e2e_tests.rs`
- `migrations/sqlite/20260413000000_create_application_table.sql`

**Files scanned:** 12 analogs deep-read

**Pattern extraction date:** 2026-05-02

**Confidence:** HIGH — Phase 1 ist eine 1:1-Replikation des Application-Aggregats mit drei Domänen-Deltas (englische Status-Werte, Snapshot-DAO-Beifang, atomare Open-Tx) und einer Lifecycle-Variante (linear statt offen↔final). Alle Patterns sind im Bestand bereits aktiv und durch Audit-System & E2E-Tests abgesichert.
