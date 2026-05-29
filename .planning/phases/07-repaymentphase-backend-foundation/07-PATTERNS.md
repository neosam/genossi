# Phase 7: RepaymentPhase Backend (Foundation) - Pattern Map

**Mapped:** 2026-05-29
**Files analyzed:** 13 (12 neu + 1 erweitert)
**Analogs found:** 13 / 13 (Assembly-Aggregat ist nahezu 1:1-Vorlage)

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `migrations/sqlite/YYYYMMDDHHMMSS_create_repayment_phase_table.sql` | migration | DDL | `migrations/sqlite/20260502000000_create_assembly_table.sql` | exact |
| `genossi_dao/src/repayment_phase.rs` | model + DAO trait | CRUD + soft-delete | `genossi_dao/src/assembly.rs` | exact |
| `genossi_dao_impl_sqlite/src/repayment_phase.rs` | DAO impl | CRUD over SQLite | `genossi_dao_impl_sqlite/src/assembly.rs` | exact |
| `genossi_service/src/repayment_phase.rs` | service trait | request-response | `genossi_service/src/assembly.rs` | exact |
| `genossi_service_impl/src/repayment_phase.rs` | service impl | CRUD + lifecycle | `genossi_service_impl/src/assembly.rs` | exact (simpler — keine Snapshot/Cascade) |
| `genossi_rest/src/repayment_phase.rs` | controller | request-response | `genossi_rest/src/assembly.rs` | exact (+ DELETE-Handler aus `genossi_rest/src/member.rs:207-222`) |
| `genossi_rest_types/src/lib.rs` (Erweiterung) | DTO + Schema | serde + utoipa | `AssemblyTO` block (Z. 1005-1141) | exact |
| `genossi_dao/src/lib.rs` (Modul-Decl) | config | static | `assembly` Eintrag Z. 2 | exact |
| `genossi_dao_impl_sqlite/src/lib.rs` (Modul-Decl) | config | static | `assembly` Eintrag Z. 2 | exact |
| `genossi_service/src/lib.rs` (Modul-Decl) | config | static | `assembly` Eintrag Z. 2 | exact |
| `genossi_service_impl/src/lib.rs` (Modul-Decl) | config | static | `assembly` Eintrag Z. 2 | exact |
| `genossi_rest/src/lib.rs` (Route + OpenAPI) | config | static | `assembly` Einträge Z. 2, 268, 435, 605 | exact |
| `genossi_bin/src/lib.rs` (DI-Wiring + RestState-Impl) | config | DI | `AssemblyServiceDependencies` + Wiring Z. 153-174, 658-671, 1258-1264 | exact |
| `genossi_bin/tests/e2e_tests.rs` (Erweiterung) | test | request-response E2E | `test_assembly_lifecycle_audit_chain_intact` Z. 8361-8514 | exact |

**Hinweis zu `ValidationService`:** Die in CONTEXT.md genannten Methoden `validate_fiscal_year`/`validate_share_value` werden **inline im `RepaymentPhaseServiceImpl`** als Helper-Funktionen realisiert (entspricht dem Pattern in `genossi_rest/src/assembly.rs:28-104` für REST-Validatoren, hier nur auf der Service-Seite). `genossi_service_impl/src/validation.rs` ist ein anderer Concern (Mitglieder-Konsistenzberichte, kein Field-Validator-Service) und wird nicht erweitert.

---

## Pattern Assignments

### 1) `migrations/sqlite/YYYYMMDDHHMMSS_create_repayment_phase_table.sql` (migration, DDL)

**Analog:** `migrations/sqlite/20260502000000_create_assembly_table.sql` (gesamte Datei, 17 Zeilen)
**Letzte vorhandene Migration:** `20260506000000_add_code_to_helper_token.sql` — Phase 7 nimmt nächste freie Sequenz (z.B. `20260529000000_create_repayment_phase_table.sql` oder gleichwertiges aktuelles Datum)

**1:1-Vorlage (assembly.sql, Z. 1-16):**
```sql
CREATE TABLE IF NOT EXISTS assembly (
    id BLOB PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    date TEXT NOT NULL,
    location TEXT,
    status TEXT NOT NULL DEFAULT 'Preparation',
    opened_at TEXT,
    closed_at TEXT,
    created TEXT NOT NULL,
    deleted TEXT,
    version BLOB NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_assembly_status ON assembly(status);
CREATE INDEX IF NOT EXISTS idx_assembly_deleted ON assembly(deleted);
CREATE INDEX IF NOT EXISTS idx_assembly_date ON assembly(date);
```

**Domain-Substitutionen:**
- Tabellenname `assembly` → `repayment_phase`
- `name TEXT NOT NULL` → entfällt
- `date TEXT NOT NULL` → `fiscal_year INTEGER NOT NULL` (D-11 / CONTEXT.md)
- `location TEXT` → `share_value INTEGER NOT NULL` (D-12, Cent als INTEGER)
- `idx_assembly_date` → `idx_repayment_phase_fiscal_year`
- `status TEXT NOT NULL DEFAULT 'Preparation'` und `opened_at`/`closed_at`/`created`/`deleted`/`version` bleiben identisch
- **Kein UNIQUE-Constraint auf `fiscal_year`** (D-08, mehrere Phasen pro GJ erlaubt)

---

### 2) `genossi_dao/src/repayment_phase.rs` (model + DAO trait, CRUD + soft-delete)

**Analog:** `genossi_dao/src/assembly.rs` (gesamte Datei)

**Status-Enum-Pattern** (assembly.rs Z. 9-42):
```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AssemblyStatus {
    Preparation,
    Open,
    Closed,
}

impl AssemblyStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            AssemblyStatus::Preparation => "Preparation",
            AssemblyStatus::Open => "Open",
            AssemblyStatus::Closed => "Closed",
        }
    }

    pub fn from_str(s: &str) -> Result<Self, DaoError> {
        match s {
            "Preparation" => Ok(AssemblyStatus::Preparation),
            "Open" => Ok(AssemblyStatus::Open),
            "Closed" => Ok(AssemblyStatus::Closed),
            _ => Err(DaoError::ParseError(Arc::from(format!(
                "Unknown assembly status: {}",
                s
            )))),
        }
    }
}

impl Default for AssemblyStatus {
    fn default() -> Self {
        AssemblyStatus::Preparation
    }
}
```

**Entity-Struct-Pattern** (assembly.rs Z. 44-56):
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

**Auditable-Impl-Pattern** (assembly.rs Z. 58-94) — `format_dt`-Closure mit Fehler-Logging und Sentinel-String NICHT vereinfachen (WR-08-Lesson):
```rust
impl crate::auditable::Auditable for AssemblyEntity {
    fn entity_type() -> &'static str {
        "assembly"
    }

    fn entity_id(&self) -> Uuid {
        self.id
    }

    fn audit_fields(&self) -> Vec<(&'static str, Option<String>)> {
        let format_dt = |dt: &time::PrimitiveDateTime| {
            dt.assume_utc()
                .format(&Iso8601::DEFAULT)
                .unwrap_or_else(|err| {
                    tracing::error!(
                        error = ?err,
                        entity = "assembly",
                        "Failed to format datetime for audit field"
                    );
                    "<invalid datetime>".to_string()
                })
        };
        vec![
            ("name", Some(self.name.to_string())),
            ("date", Some(format_dt(&self.date))),
            ("location", self.location.as_ref().map(|s| s.to_string())),
            ("status", Some(self.status.as_str().to_string())),
            ("opened_at", self.opened_at.as_ref().map(format_dt)),
            ("closed_at", self.closed_at.as_ref().map(format_dt)),
        ]
    }
}
```

**DAO-Trait-Pattern** (assembly.rs Z. 96-138):
```rust
#[automock(type Transaction = crate::MockTransaction;)]
#[async_trait]
pub trait AssemblyDao {
    type Transaction: crate::Transaction;

    async fn dump_all(&self, tx: Self::Transaction) -> Result<Arc<[AssemblyEntity]>, DaoError>;

    async fn create(
        &self,
        entity: &AssemblyEntity,
        process: &str,
        tx: Self::Transaction,
    ) -> Result<(), DaoError>;

    async fn update(
        &self,
        entity: &AssemblyEntity,
        process: &str,
        tx: Self::Transaction,
    ) -> Result<(), DaoError>;

    async fn all(&self, tx: Self::Transaction) -> Result<Arc<[AssemblyEntity]>, DaoError> {
        let all_entities = self.dump_all(tx).await?;
        let active_entities: Vec<AssemblyEntity> = all_entities
            .iter()
            .filter(|e| e.deleted.is_none())
            .cloned()
            .collect();
        Ok(active_entities.into())
    }

    async fn find_by_id(
        &self,
        id: Uuid,
        tx: Self::Transaction,
    ) -> Result<Option<AssemblyEntity>, DaoError> {
        let all_entities = self.dump_all(tx).await?;
        Ok(all_entities
            .iter()
            .find(|e| e.id == id && e.deleted.is_none())
            .cloned())
    }
}
```

**Domain-Substitutionen für RepaymentPhase:**
- `AssemblyStatus` → `RepaymentPhaseStatus` (gleiche drei Varianten Preparation/Open/Closed)
- Fehlertext `"Unknown assembly status"` → `"Unknown repayment phase status"`
- `AssemblyEntity` → `RepaymentPhaseEntity`
- `name: Arc<str>` → entfällt
- `date: PrimitiveDateTime` → `fiscal_year: i32`
- `location: Option<Arc<str>>` → `share_value: i64`
- Auditable `entity_type()` → `"repayment_phase"`
- Auditable `audit_fields()`: `["fiscal_year", "share_value", "status", "opened_at", "closed_at"]` (5 statt 6; **keine** id/version/created/deleted)
- `format_dt`-Closure-Tracing-Label `entity = "assembly"` → `entity = "repayment_phase"`
- `AssemblyDao` → `RepaymentPhaseDao`
- **Test-Cases im `mod tests`-Block (assembly.rs Z. 140-251) komplett mit-übernehmen** — insbesondere `test_*_status_roundtrip`, `test_*_status_strings_are_english`, `test_*_status_invalid_string`, `test_*_status_default_is_preparation`, `test_auditable_entity_type_is_*`, `test_auditable_fields_count_and_excludes` (Anzahl auf 5 anpassen), `test_auditable_diff_detects_status_change`.

---

### 3) `genossi_dao_impl_sqlite/src/repayment_phase.rs` (DAO impl, CRUD über SQLite)

**Analog:** `genossi_dao_impl_sqlite/src/assembly.rs` (gesamte Datei)

**DB-Row + TryFrom-Pattern** (assembly.rs Z. 32-71):
```rust
#[derive(Debug, sqlx::FromRow)]
struct AssemblyDb {
    id: Vec<u8>,
    name: String,
    date: String,
    location: Option<String>,
    status: String,
    opened_at: Option<String>,
    closed_at: Option<String>,
    created: String,
    deleted: Option<String>,
    version: Vec<u8>,
}

impl TryFrom<&AssemblyDb> for AssemblyEntity {
    type Error = DaoError;

    fn try_from(db: &AssemblyDb) -> Result<Self, Self::Error> {
        Ok(AssemblyEntity {
            id: Uuid::from_slice(&db.id)?,
            name: Arc::from(db.name.as_str()),
            date: parse_datetime(&db.date)?,
            location: db.location.as_deref().map(Arc::from),
            status: AssemblyStatus::from_str(&db.status)?,
            opened_at: db.opened_at.as_ref().map(|s| parse_datetime(s)).transpose()?,
            closed_at: db.closed_at.as_ref().map(|s| parse_datetime(s)).transpose()?,
            created: parse_datetime(&db.created)?,
            deleted: db.deleted.as_ref().map(|d| parse_datetime(d)).transpose()?,
            version: Uuid::from_slice(&db.version)?,
        })
    }
}
```

**`parse_datetime`-Helper wiederverwenden** (assembly.rs Z. 14-30):
- `parse_datetime` ist `pub(crate)` — Phase 7 sollte einen **eigenen** identischen Helper anlegen (kein Cross-Modul-`pub(crate)`-Refactor), oder die Funktion in `genossi_dao_impl_sqlite/src/lib.rs` als crate-shared exportieren. Pattern-Konsistenz: jedes DAO-Impl hat seinen eigenen `parse_datetime`-Aufruf, aber die Funktion lebt in `assembly.rs`. **Empfehlung:** in `repayment_phase.rs` exakt die gleiche Funktion erneut definieren (DRY-Verstoß bewusst), oder per `use crate::assembly::parse_datetime;` importieren. Planner entscheidet.

**`format_dt`-Helper-Pattern** (assembly.rs Z. 83-88):
```rust
fn format_dt(dt: &PrimitiveDateTime) -> Result<String, DaoError> {
    let format = &time::format_description::well_known::Iso8601::DEFAULT;
    dt.assume_utc()
        .format(format)
        .map_err(|e| DaoError::ParseError(Arc::from(e.to_string())))
}
```

**`dump_all`-Pattern** (assembly.rs Z. 94-107):
```rust
async fn dump_all(&self, tx: Self::Transaction) -> Result<Arc<[AssemblyEntity]>, DaoError> {
    let rows = sqlx::query_as::<_, AssemblyDb>(
        "SELECT id, name, date, location, status, opened_at, closed_at, created, deleted, version \
         FROM assembly ORDER BY date DESC",
    )
    .fetch_all(tx.tx.lock().await.as_mut())
    .await
    .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?;

    rows.iter()
        .map(AssemblyEntity::try_from)
        .collect::<Result<Vec<_>, _>>()
        .map(|v| v.into())
}
```
**ORDER BY-Anpassung:** `ORDER BY date DESC` → `ORDER BY fiscal_year DESC, created DESC` (siehe CONTEXT.md `<specifics>` "Frontend zeigt Phasen sortiert nach `fiscal_year DESC, created DESC`")

**`create`-Pattern** (assembly.rs Z. 109-146) und **`update`-Pattern mit Version-Bump + Pre-Exists-Check + Version-Mismatch-Detection** (assembly.rs Z. 148-205):
```rust
async fn update(
    &self,
    entity: &AssemblyEntity,
    _process: &str,
    tx: Self::Transaction,
) -> Result<(), DaoError> {
    let id = entity.id.as_bytes().to_vec();
    let old_version = entity.version.as_bytes().to_vec();
    let new_version = Uuid::new_v4().as_bytes().to_vec();
    // ... bind locals ...

    // Pre-condition: row must exist and not be soft-deleted.
    let exists = sqlx::query_scalar::<_, i32>(
        "SELECT COUNT(*) FROM assembly WHERE id = ? AND deleted IS NULL",
    )
    .bind(id.clone())
    .fetch_one(tx.tx.lock().await.as_mut())
    .await
    .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?;

    if exists == 0 {
        return Err(DaoError::NotFound);
    }

    let rows_affected = sqlx::query(
        "UPDATE assembly SET name = ?, date = ?, location = ?, status = ?, \
         opened_at = ?, closed_at = ?, deleted = ?, version = ? \
         WHERE id = ? AND version = ? AND deleted IS NULL",
    )
    // ... binds ...
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
**Wichtig:** WHERE-Clause `deleted IS NULL` muss bei `audited_delete!` (D-09) ausgehoben werden — beim Soft-Delete wird das `deleted`-Feld gesetzt, dann ist die Row für nachfolgende Updates "weg". Phase 7 hat keinen Hard-Restore — Code-Pfad muss aber kein Sonderfall: das `audited_delete!`-Macro ruft `update()` mit dem Feld bereits gesetzt — und der Pre-Exists-Check `deleted IS NULL` muss erfolgreich sein, weil der Update den Übergang `deleted IS NULL → deleted = now()` macht (assembly.rs Z. 168-178 / SQL hat `WHERE id = ? AND deleted IS NULL` für Pre-Exists und `WHERE id = ? AND version = ? AND deleted IS NULL` für das eigentliche UPDATE — beide treffen die Pre-Delete-Row).

**Domain-Substitutionen:**
- Spalten `name`/`date`/`location` → `fiscal_year`/`share_value` (jeweils i64-Bind)
- `name: String` → `fiscal_year: i64` (sqlx liest SQLite INTEGER als i64; cast zu i32 in `TryFrom`)
- `location: Option<String>` → `share_value: i64`
- Test-Cases (assembly.rs Z. 208-368) mit `setup_db`-Helper, `make_*`-Builder und drei Roundtrip-Tests (`test_create_and_find`, `test_update_with_version_mismatch_returns_conflict`, `test_update_unknown_id_returns_not_found`, `test_update_succeeds_then_version_changes`) **komplett mit-übernehmen**.

---

### 4) `genossi_service/src/repayment_phase.rs` (service trait)

**Analog:** `genossi_service/src/assembly.rs` (Z. 1-154)

**Domain-Type-Pattern mit From-Roundtrip** (assembly.rs Z. 23-69):
```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Assembly {
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

impl From<&AssemblyEntity> for Assembly { /* feldweise */ }
impl From<&Assembly> for AssemblyEntity { /* feldweise */ }
```

**Submission/Update-DTO-Pattern** (assembly.rs Z. 72-89):
```rust
#[derive(Clone, Debug)]
pub struct AssemblySubmission {
    pub name: Arc<str>,
    pub date: time::PrimitiveDateTime,
    pub location: Option<Arc<str>>,
}

#[derive(Clone, Debug)]
pub struct AssemblyUpdate {
    pub name: Arc<str>,
    pub date: time::PrimitiveDateTime,
    pub location: Option<Arc<str>>,
    pub version: Uuid,
}
```

**Service-Trait-Pattern mit `#[automock]`** (assembly.rs Z. 101-154):
```rust
#[automock(type Context=(); type Transaction = genossi_dao::MockTransaction;)]
#[async_trait]
pub trait AssemblyService {
    type Context: Clone + Debug + PartialEq + Eq + Send + Sync + 'static;
    type Transaction: genossi_dao::Transaction;

    async fn create_assembly(&self, submission: &AssemblySubmission,
        context: Authentication<Self::Context>) -> Result<Assembly, ServiceError>;
    async fn update_assembly(&self, id: Uuid, update: &AssemblyUpdate,
        context: Authentication<Self::Context>) -> Result<Assembly, ServiceError>;
    async fn open_assembly(&self, id: Uuid,
        context: Authentication<Self::Context>) -> Result<Assembly, ServiceError>;
    async fn close_assembly(&self, id: Uuid,
        context: Authentication<Self::Context>) -> Result<Assembly, ServiceError>;
    async fn get_assembly(&self, id: Uuid,
        context: Authentication<Self::Context>) -> Result<AssemblyDetail, ServiceError>;
    async fn get_all_assemblies(&self,
        context: Authentication<Self::Context>) -> Result<Arc<[Assembly]>, ServiceError>;
}
```

**Domain-Substitutionen:**
- `Assembly`/`AssemblyEntity`/`AssemblyStatus` → `RepaymentPhase`/`RepaymentPhaseEntity`/`RepaymentPhaseStatus`
- `AssemblyDetail` (mit `snapshot_member_count`) → **entfällt** (Phase 7 hat keinen aggregierten Detail-Typ — `get_repayment_phase` liefert direkt `RepaymentPhase`)
- `name`/`date`/`location` → `fiscal_year: i32`/`share_value: i64`
- Trait-Methoden:
  - `create_repayment_phase` (Eingabe `RepaymentPhaseSubmission { fiscal_year, share_value }`)
  - `update_repayment_phase` (Eingabe `RepaymentPhaseUpdate { fiscal_year, share_value, version }`, Service-Layer prüft Lock-Matrix D-04)
  - `open_repayment_phase` (kein Body, nur ID)
  - `close_repayment_phase` (kein Body, nur ID)
  - **NEU:** `delete_repayment_phase(id, context) -> Result<(), ServiceError>` — siehe D-09 (nur in Preparation erlaubt)
  - `get_repayment_phase` (return `RepaymentPhase`, kein Detail-Wrapper)
  - `get_all_repayment_phases`
- Test-Cases (assembly.rs Z. 156-239) mit `make_entity`, `entity_to_*_roundtrip`, `test_mock_*_compiles`, `test_*_submission_constructible`, `test_*_update_requires_version` **mit-übernehmen**.

---

### 5) `genossi_service_impl/src/repayment_phase.rs` (service impl, CRUD + Lifecycle)

**Analog:** `genossi_service_impl/src/assembly.rs` (Z. 1-385) — Phase 7 ist eine **vereinfachte** Variante ohne Snapshot/Cascade-Logik

**Prozesskonstanten** (assembly.rs Z. 46-50):
```rust
const ASSEMBLY_PROCESS_CREATE: &str = "assembly.create";
const ASSEMBLY_PROCESS_OPEN: &str = "assembly.open";
const ASSEMBLY_PROCESS_CLOSE: &str = "assembly.close";
const ASSEMBLY_PROCESS_UPDATE: &str = "assembly.update";
const ADMIN_PRIVILEGE: &str = "admin";
```
**Für Phase 7:** `REPAYMENT_PHASE_PROCESS_CREATE`/`_OPEN`/`_CLOSE`/`_UPDATE`/`_DELETE` (jeweils `"repayment-phase.create"` etc.) — siehe CONTEXT.md Z. 139 (Konstanten-Konvention).

**`gen_service_impl!`-Wiring-Pattern** (assembly.rs Z. 52-67):
```rust
gen_service_impl! {
    struct AssemblyServiceImpl: AssemblyService = AssemblyServiceDeps {
        AssemblyDao: AssemblyDao<Transaction = Self::Transaction> = assembly_dao,
        AssemblyMemberSnapshotDao: ...,
        MemberDao: MemberDao<Transaction = Self::Transaction> = member_dao,
        AuditLogDao: AuditLogDao<Transaction = Self::Transaction> = audit_log_dao,
        PermissionService: PermissionService<Context = Self::Context> = permission_service,
        UuidService: UuidService = uuid_service,
        TransactionDao: TransactionDao<Transaction = Self::Transaction> = transaction_dao,
        HelperTokenDao: ...,
        PermissionDao: ...,
    }
}
```
**Für Phase 7 nur 5 Deps** (NICHT Snapshot, NICHT MemberDao, NICHT HelperTokenDao, NICHT PermissionDao):
```rust
gen_service_impl! {
    struct RepaymentPhaseServiceImpl: RepaymentPhaseService = RepaymentPhaseServiceDeps {
        RepaymentPhaseDao: RepaymentPhaseDao<Transaction = Self::Transaction> = repayment_phase_dao,
        AuditLogDao: AuditLogDao<Transaction = Self::Transaction> = audit_log_dao,
        PermissionService: PermissionService<Context = Self::Context> = permission_service,
        UuidService: UuidService = uuid_service,
        TransactionDao: TransactionDao<Transaction = Self::Transaction> = transaction_dao,
    }
}
```

**`create_*`-Methode mit `audited_create!`** (assembly.rs Z. 74-117):
```rust
async fn create_assembly(
    &self,
    submission: &AssemblySubmission,
    context: Authentication<Self::Context>,
) -> Result<Assembly, ServiceError> {
    let tx = self.transaction_dao.use_transaction(None).await?;

    let user_id = self
        .permission_service
        .current_user_id(context.clone())
        .await?
        .unwrap_or_else(|| "SYSTEM".to_string());
    self.permission_service
        .check_permission(ADMIN_PRIVILEGE, context)
        .await?;

    let now = time::OffsetDateTime::now_utc();
    let created = time::PrimitiveDateTime::new(now.date(), now.time());

    let entity = AssemblyEntity {
        id: self.uuid_service.new_v4().await,
        name: submission.name.clone(),
        date: submission.date,
        location: submission.location.clone(),
        status: AssemblyStatus::Preparation,
        opened_at: None,
        closed_at: None,
        created,
        deleted: None,
        version: self.uuid_service.new_v4().await,
    };

    crate::audited_create!(
        self,
        self.assembly_dao,
        &entity,
        ASSEMBLY_PROCESS_CREATE,
        &user_id,
        tx
    );

    self.transaction_dao.commit(tx).await?;
    Ok(Assembly::from(&entity))
}
```
**Phase-7-Erweiterung:** **vor `entity = ...`** Field-Validation einbauen:
```rust
// D-11/D-12: validate inputs before construction
let mut errors: Vec<genossi_service::ValidationFailureItem> = Vec::new();
if !(2000..=2100).contains(&submission.fiscal_year) {
    errors.push(genossi_service::ValidationFailureItem {
        field: Arc::from("fiscal_year"),
        message: Arc::from(format!("must be in 2000..=2100, got {}", submission.fiscal_year)),
    });
}
if submission.share_value <= 0 {
    errors.push(genossi_service::ValidationFailureItem {
        field: Arc::from("share_value"),
        message: Arc::from("must be > 0 (Cent)"),
    });
}
if !errors.is_empty() {
    return Err(ServiceError::ValidationError(errors));
}
```

**`update_*`-Methode mit Status-Guard + Version-Check + audited_update!** (assembly.rs Z. 119-179) — die WR-04-Doppelt-Lese-Logik (Z. 136-144) ist wichtig und bewusst, **NICHT** entfernen:
```rust
async fn update_assembly(
    &self,
    id: Uuid,
    update: &AssemblyUpdate,
    context: Authentication<Self::Context>,
) -> Result<Assembly, ServiceError> {
    let tx = self.transaction_dao.use_transaction(None).await?;

    let user_id = self.permission_service.current_user_id(context.clone()).await?
        .unwrap_or_else(|| "SYSTEM".to_string());
    self.permission_service.check_permission(ADMIN_PRIVILEGE, context).await?;

    // WR-04: duplicate read is intentional — required for state-guard
    // BEFORE we mutate entity. audited_update! does its own load internally
    // for the diff. Both reads run in the same tx → same snapshot.
    let mut entity = self
        .assembly_dao
        .find_by_id(id, tx.clone())
        .await?
        .ok_or(ServiceError::EntityNotFound(id))?;

    // D-07: only Preparation is editable.
    if entity.status != AssemblyStatus::Preparation {
        return Err(ServiceError::Conflict(Arc::from(format!(
            "Cannot update assembly: status is '{}', expected 'Preparation' (D-07)",
            entity.status.as_str()
        ))));
    }
    if entity.version != update.version {
        return Err(ServiceError::Conflict(Arc::from("Version mismatch")));
    }

    entity.name = update.name.clone();
    entity.date = update.date;
    entity.location = update.location.clone();

    crate::audited_update!(
        self,
        self.assembly_dao,
        id,
        &entity,
        ASSEMBLY_PROCESS_UPDATE,
        &user_id,
        tx
    );

    self.transaction_dao.commit(tx).await?;
    Ok(Assembly::from(&entity))
}
```

**Phase-7-Erweiterung der State-Machine (D-04, D-07):** statt `if entity.status != Preparation { Conflict }` muss `update_repayment_phase` die **Edit-Matrix** prüfen:
```rust
// D-04: edit matrix.
//   Preparation -> fiscal_year + share_value EDIT
//   Open        -> share_value EDIT, fiscal_year LOCKED
//   Closed      -> alles LOCKED
match entity.status {
    RepaymentPhaseStatus::Closed => {
        return Err(ServiceError::Conflict(Arc::from(
            "Cannot update: phase is Closed (D-04)",
        )));
    }
    RepaymentPhaseStatus::Open => {
        // D-07: atomically reject any change to fiscal_year
        if entity.fiscal_year != update.fiscal_year {
            return Err(ServiceError::Conflict(Arc::from(
                "Cannot change fiscal_year: phase is Open (D-04/D-07)",
            )));
        }
    }
    RepaymentPhaseStatus::Preparation => { /* alles editierbar */ }
}
if entity.version != update.version {
    return Err(ServiceError::Conflict(Arc::from("Version mismatch")));
}
// Field-Level-Validation (fiscal_year range, share_value > 0) wie bei create_*
// Mutationen anwenden
entity.fiscal_year = update.fiscal_year;
entity.share_value = update.share_value;
```

**`open_*`-Methode mit Lifecycle-Guard + audited_update!** (assembly.rs Z. 181-259) — **Phase 7 hat KEINE Snapshot-Logik**, die Logik unter Z. 230-256 (Member-Filter + Snapshot-Insert) entfällt:
```rust
async fn open_assembly(&self, id: Uuid, context: Authentication<Self::Context>) -> Result<Assembly, ServiceError> {
    let tx = self.transaction_dao.use_transaction(None).await?;
    let user_id = self.permission_service.current_user_id(context.clone()).await?
        .unwrap_or_else(|| "SYSTEM".to_string());
    self.permission_service.check_permission(ADMIN_PRIVILEGE, context).await?;

    let mut entity = self.assembly_dao.find_by_id(id, tx.clone()).await?
        .ok_or(ServiceError::EntityNotFound(id))?;

    // State-transition guard (D-06: keine Reverse-Transition)
    if entity.status != AssemblyStatus::Preparation {
        return Err(ServiceError::Conflict(Arc::from(format!(
            "Cannot open assembly: status is '{}', expected 'Preparation'",
            entity.status.as_str()
        ))));
    }

    let now_offset = time::OffsetDateTime::now_utc();
    let now_pdt = time::PrimitiveDateTime::new(now_offset.date(), now_offset.time());
    entity.status = AssemblyStatus::Open;
    entity.opened_at = Some(now_pdt);

    crate::audited_update!(
        self, self.assembly_dao, id, &entity,
        ASSEMBLY_PROCESS_OPEN, &user_id, tx
    );

    self.transaction_dao.commit(tx).await?;
    Ok(Assembly::from(&entity))
}
```
**Phase-7-Anmerkung im Code:** Kommentar einbauen, dass Phase 8 hier die Auto-Befüllung der RepaymentEntries hinzufügen wird (siehe CONTEXT.md `<domain>` Out-of-scope).

**`close_*`-Methode** (assembly.rs Z. 261-341): identisches Pattern wie `open_*`, nur Guard `entity.status != Open → Conflict`. **Phase 7 hat KEINE Cascade-Logik** — Z. 307-338 (Helper-Session-Cascade) entfällt vollständig. Simpler:
```rust
if entity.status != RepaymentPhaseStatus::Open {
    return Err(ServiceError::Conflict(Arc::from(format!(
        "Cannot close repayment phase: status is '{}', expected 'Open'",
        entity.status.as_str()
    ))));
}
entity.status = RepaymentPhaseStatus::Closed;
entity.closed_at = Some(now_pdt);
crate::audited_update!(self, self.repayment_phase_dao, id, &entity,
    REPAYMENT_PHASE_PROCESS_CLOSE, &user_id, tx);
self.transaction_dao.commit(tx).await?;
```
**Phase-7-Anmerkung im Code:** Kommentar, dass Phase 8 hier die Validation "alle RepaymentEntries paid_out" einfügt (PHAS-03).

**`delete_*`-Methode mit `audited_delete!`** (Pattern von `genossi_service_impl/src/member.rs:354-383`):
```rust
async fn delete_repayment_phase(
    &self,
    id: Uuid,
    context: Authentication<Self::Context>,
) -> Result<(), ServiceError> {
    let tx = self.transaction_dao.use_transaction(None).await?;
    let user_id = self.permission_service.current_user_id(context.clone()).await?
        .unwrap_or_else(|| "SYSTEM".to_string());
    self.permission_service.check_permission(ADMIN_PRIVILEGE, context).await?;

    // D-09: soft-delete nur in Preparation
    let entity = self.repayment_phase_dao.find_by_id(id, tx.clone()).await?
        .ok_or(ServiceError::EntityNotFound(id))?;
    if entity.status != RepaymentPhaseStatus::Preparation {
        return Err(ServiceError::Conflict(Arc::from(format!(
            "Cannot delete: status is '{}', expected 'Preparation' (D-09)",
            entity.status.as_str()
        ))));
    }

    crate::audited_delete!(
        self,
        self.repayment_phase_dao,
        id,
        REPAYMENT_PHASE_PROCESS_DELETE,
        &user_id,
        tx
    );

    self.transaction_dao.commit(tx).await?;
    Ok(())
}
```

**`get_*`-Methode** (assembly.rs Z. 343-368): direktes `find_by_id` + Permission-Check. Phase 7 OHNE Snapshot-Detail-Wrapper — gibt `RepaymentPhase` direkt zurück.

**`get_all_*`-Methode** (assembly.rs Z. 370-384):
```rust
async fn get_all_assemblies(&self, context: Authentication<Self::Context>) -> Result<Arc<[Assembly]>, ServiceError> {
    let tx = self.transaction_dao.use_transaction(None).await?;
    self.permission_service.check_permission(ADMIN_PRIVILEGE, context).await?;
    let entities = self.assembly_dao.all(tx.clone()).await?;
    let assemblies: Arc<[Assembly]> = entities.iter().map(Assembly::from).collect();
    self.transaction_dao.commit(tx).await?;
    Ok(assemblies)
}
```

**Test-Modul (assembly.rs Z. 387-...):** Über 600 Zeilen Unit-Tests mit `TestTransaction`, `mock!` für `TestTxDao`/`TestAssemblyDao`/`TestSnapshotDao`/`TestMemberDao`/etc. Pattern komplett übernehmen — **Phase 7 braucht weniger Mocks** (nur `TestTxDao`, `TestRepaymentPhaseDao`, `TestAuditLogDao`, `TestPermissionService`, `TestUuidService`). Wichtige Tests: `test_update_assembly_version_mismatch_returns_conflict` (assembly.rs Z. 1036-1075 lt. CONTEXT.md `<canonical_refs>`) als Vorlage für `test_update_repayment_phase_version_mismatch_returns_conflict`.

---

### 6) `genossi_rest/src/repayment_phase.rs` (controller, request-response)

**Analog:** `genossi_rest/src/assembly.rs` (Z. 1-381) + `genossi_rest/src/member.rs:195-222` für DELETE

**RestState-Trait-Pattern** (assembly.rs Z. 20-24):
```rust
pub trait AssemblyRestState: Clone + Send + Sync + 'static {
    type AssemblyService: AssemblyService<Context = crate::ContextType> + Send + Sync + 'static;
    fn assembly_service(&self) -> Arc<Self::AssemblyService>;
}
```

**Request-Validatoren-Pattern** (assembly.rs Z. 28-104) — **Phase 7 braucht weniger Validatoren** (Field-Level-Validation passiert serverseitig im Service-Layer per `ValidationError`). REST-Layer kann eine **minimale** `validate_create_repayment_phase_request` machen, die nur strukturelle Pflichtfelder prüft (z.B. `fiscal_year != 0`).

**List/Create/Get/Update/Open/Close-Handler-Pattern** (assembly.rs Z. 108-347) — alle sechs Handler folgen dem **identischen Aufbau**:
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
    error_handler(
        (async {
            let auth = crate::extract_auth_context(Some(context))?;
            validate_create_assembly_request(&body).map_err(|errs| { /* 400 */ })?;
            let date = body.date.ok_or_else(|| RestError::BadRequest("date required".into()))?;
            let submission = AssemblySubmission { /* ... */ };
            let assembly = rest_state.assembly_service().create_assembly(&submission, auth).await?;
            let to = AssemblyTO::from(&assembly);
            Ok(Response::builder()
                .status(201)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&to)?))
                .unwrap())
        })
        .await,
    )
}
```

**Open/Close-Lifecycle-Handler (NUR ID, KEIN BODY, KEIN VERSION-CHECK — D-03)** (assembly.rs Z. 277-347):
```rust
#[instrument(skip(rest_state))]
#[utoipa::path(
    post,
    tag = "Assemblies",
    path = "/{id}/open",
    params(("id" = Uuid, Path, description = "Assembly ID")),
    responses(
        (status = 200, description = "Opened", body = AssemblyTO),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Not found"),
        (status = 409, description = "Conflict (status not Preparation)"),
    ),
)]
pub async fn open_assembly<RestState: RestStateDef + AssemblyRestState>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path(id): Path<Uuid>,
) -> Response {
    error_handler(
        (async {
            let auth = crate::extract_auth_context(Some(context))?;
            let assembly = rest_state.assembly_service().open_assembly(id, auth).await?;
            let to = AssemblyTO::from(&assembly);
            Ok(Response::builder().status(200)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&to)?)).unwrap())
        }).await,
    )
}
```

**DELETE-Handler-Pattern** (member.rs Z. 195-222) — Assembly hat KEINEN DELETE, daher Pattern aus Member kopieren:
```rust
#[instrument(skip(rest_state))]
#[utoipa::path(
    delete,
    tag = "RepaymentPhases",
    path = "/{id}",
    params(("id" = Uuid, Path, description = "RepaymentPhase ID")),
    responses(
        (status = 204, description = "RepaymentPhase deleted (soft-delete)"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Not found"),
        (status = 409, description = "Conflict (status not Preparation, D-09)"),
    ),
)]
pub async fn delete_repayment_phase<RestState: RestStateDef + RepaymentPhaseRestState>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path(id): Path<Uuid>,
) -> Response {
    error_handler(
        (async {
            rest_state
                .repayment_phase_service()
                .delete_repayment_phase(id, crate::extract_auth_context(Some(context))?)
                .await?;
            Ok(Response::builder().status(204).body(Body::empty()).unwrap())
        })
        .await,
    )
}
```

**Router-Composition-Pattern** (assembly.rs Z. 349-361):
```rust
pub fn generate_route<RestState: RestStateDef + AssemblyRestState>() -> Router<RestState> {
    Router::new()
        .route("/", get(list_assemblies::<RestState>).post(create_assembly::<RestState>))
        .route("/{id}",
            get(get_assembly::<RestState>)
                .put(update_assembly::<RestState>),
        )
        .route("/{id}/open", post(open_assembly::<RestState>))
        .route("/{id}/close", post(close_assembly::<RestState>))
}
```
**Phase-7-Erweiterung:** zusätzliches `.delete(delete_repayment_phase::<RestState>)` am `/{id}`-Endpoint (analog `member.rs:34`):
```rust
.route("/{id}",
    get(get_repayment_phase::<RestState>)
        .put(update_repayment_phase::<RestState>)
        .delete(delete_repayment_phase::<RestState>),
)
```

**OpenAPI-ApiDoc-Pattern** (assembly.rs Z. 363-381):
```rust
#[derive(OpenApi)]
#[openapi(
    paths(
        list_assemblies, create_assembly, get_assembly,
        update_assembly, open_assembly, close_assembly
    ),
    components(schemas(
        AssemblyTO, AssemblyStatusTO, AssemblyDetailTO,
        CreateAssemblyRequest, UpdateAssemblyRequest
    ))
)]
pub struct ApiDoc;
```
**Phase 7:** zusätzlich `delete_repayment_phase` in `paths()`, `AssemblyDetailTO` raus (Phase 7 hat keinen Detail-Typ), `RepaymentPhaseTO`/`RepaymentPhaseStatusTO`/`CreateRepaymentPhaseRequest`/`UpdateRepaymentPhaseRequest` rein.

---

### 7) `genossi_rest_types/src/lib.rs` (Erweiterung: RepaymentPhase TO/Schema)

**Analog:** `AssemblyTO` Block (Z. 1005-1141, ~145 Zeilen)

**Status-TO-Pattern mit bidirektionalem From** (Z. 1007-1034):
```rust
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum AssemblyStatusTO {
    Preparation,
    Open,
    Closed,
}

impl From<&genossi_dao::assembly::AssemblyStatus> for AssemblyStatusTO {
    fn from(s: &genossi_dao::assembly::AssemblyStatus) -> Self {
        use genossi_dao::assembly::AssemblyStatus;
        match s {
            AssemblyStatus::Preparation => AssemblyStatusTO::Preparation,
            AssemblyStatus::Open => AssemblyStatusTO::Open,
            AssemblyStatus::Closed => AssemblyStatusTO::Closed,
        }
    }
}

impl From<&AssemblyStatusTO> for genossi_dao::assembly::AssemblyStatus { /* invers */ }
```

**Haupt-TO-Pattern mit ISO8601-Datetime-Serde + Utoipa-Schema** (Z. 1036-1095):
```rust
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct AssemblyTO {
    #[schema(example = "123e4567-e89b-12d3-a456-426614174000")]
    pub id: Uuid,
    #[schema(example = "GV 2026")]
    pub name: String,
    #[serde(
        serialize_with = "iso8601_datetime::serialize",
        deserialize_with = "iso8601_datetime::deserialize",
        default
    )]
    pub date: Option<time::PrimitiveDateTime>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    #[schema(example = "Vereinsheim")]
    pub location: Option<String>,
    pub status: AssemblyStatusTO,
    #[serde(serialize_with = "iso8601_datetime::serialize",
            deserialize_with = "iso8601_datetime::deserialize", default)]
    pub opened_at: Option<time::PrimitiveDateTime>,
    #[serde(serialize_with = "iso8601_datetime::serialize",
            deserialize_with = "iso8601_datetime::deserialize", default)]
    pub closed_at: Option<time::PrimitiveDateTime>,
    #[serde(serialize_with = "iso8601_datetime::serialize",
            deserialize_with = "iso8601_datetime::deserialize", default)]
    pub created: Option<time::PrimitiveDateTime>,
    #[serde(serialize_with = "iso8601_datetime::serialize",
            deserialize_with = "iso8601_datetime::deserialize", default)]
    pub deleted: Option<time::PrimitiveDateTime>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub version: Option<Uuid>,
}

impl From<&genossi_service::assembly::Assembly> for AssemblyTO {
    fn from(a: &genossi_service::assembly::Assembly) -> Self {
        Self {
            id: a.id,
            name: a.name.to_string(),
            date: Some(a.date),
            location: a.location.as_ref().map(|s| s.to_string()),
            status: AssemblyStatusTO::from(&a.status),
            opened_at: a.opened_at,
            closed_at: a.closed_at,
            created: Some(a.created),
            deleted: a.deleted,
            version: Some(a.version),
        }
    }
}
```

**Create/Update-Request-Pattern** (Z. 1112-1141):
```rust
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateAssemblyRequest {
    #[schema(example = "GV 2026")]
    pub name: String,
    #[serde(serialize_with = "iso8601_datetime::serialize",
            deserialize_with = "iso8601_datetime::deserialize", default)]
    pub date: Option<time::PrimitiveDateTime>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    #[schema(example = "Vereinsheim")]
    pub location: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct UpdateAssemblyRequest {
    #[schema(example = "GV 2026")]
    pub name: String,
    #[serde(serialize_with = "iso8601_datetime::serialize",
            deserialize_with = "iso8601_datetime::deserialize", default)]
    pub date: Option<time::PrimitiveDateTime>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    #[schema(example = "Vereinsheim")]
    pub location: Option<String>,
    pub version: Uuid,
}
```

**Domain-Substitutionen für RepaymentPhase:**
- `AssemblyStatusTO` → `RepaymentPhaseStatusTO` (Preparation/Open/Closed identisch)
- `AssemblyTO`:
  - `name: String` → entfällt
  - `date: Option<PrimitiveDateTime>` → `fiscal_year: i32` (Pflichtfeld, kein Option, kein serde-skip)
  - `location: Option<String>` → `share_value: i64` (Pflichtfeld)
  - `status`/`opened_at`/`closed_at`/`created`/`deleted`/`version` bleiben
  - `#[schema(example = "GV 2026")]` → `#[schema(example = 2026)]` für `fiscal_year`, `#[schema(example = 12000)]` für `share_value` (CONTEXT.md "Claude's Discretion: OpenAPI-Beispielwerte")
- `AssemblyDetailTO` → **entfällt** (kein Snapshot in Phase 7)
- `CreateAssemblyRequest` → `CreateRepaymentPhaseRequest { fiscal_year: i32, share_value: i64 }`
- `UpdateAssemblyRequest` → `UpdateRepaymentPhaseRequest { fiscal_year: i32, share_value: i64, version: Uuid }`
- Test-Roundtrip-Block (Z. 1443-1550+) entsprechend mit-übernehmen.

---

### 8) Modul-Deklarationen in `lib.rs`-Dateien

| Datei | Zeile | Pattern (in alphabetischer Reihenfolge) |
|-------|-------|------------------------------------------|
| `genossi_dao/src/lib.rs` | nach Z. 13 (`pub mod permission;`) | `pub mod repayment_phase;` |
| `genossi_dao_impl_sqlite/src/lib.rs` | nach Z. 12 (`pub mod permission;`) | `pub mod repayment_phase;` |
| `genossi_service/src/lib.rs` | nach Z. 14 (`pub mod permission;`) | `pub mod repayment_phase;` |
| `genossi_service_impl/src/lib.rs` | nach Z. 15 (`pub mod permission;`) | `pub mod repayment_phase;` |

Alphabetische Sortierung halten (alle existierenden lib.rs-Dateien sind alphabetisch sortiert; `repayment_phase` kommt zwischen `permission` und `session`/`uuid_service`).

---

### 9) `genossi_rest/src/lib.rs` — Route + OpenAPI-Registry

**Analog:** Assembly-Einträge an folgenden Stellen:

**Modul-Deklaration** (Z. 2):
```rust
pub mod assembly;
```
→ `pub mod repayment_phase;`

**OpenAPI-`nest`-Eintrag** (Z. 268):
```rust
(path = "/api/assembly", api = assembly::ApiDoc),
```
→ `(path = "/api/repayment-phase", api = repayment_phase::ApiDoc),`

**`create_app<RestState>`-Trait-Bound** (Z. 435):
```rust
+ assembly::AssemblyRestState
```
→ zusätzlich `+ repayment_phase::RepaymentPhaseRestState`

**Router-`.nest`-Eintrag** (Z. 605):
```rust
.nest("/api/assembly", assembly::generate_route::<RestState>())
```
→ neuer `.nest("/api/repayment-phase", repayment_phase::generate_route::<RestState>())` (D-14: Singular)

---

### 10) `genossi_bin/src/lib.rs` — Dependency-Injection

**Analog:** Assembly-Wiring an drei Stellen:

**Typ-Alias und Deps-Struct** (Z. 127, 153-174):
```rust
type AssemblyDao = genossi_dao_impl_sqlite::assembly::AssemblyDaoImpl;

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
    type HelperTokenDao = HelperTokenDao;
    type PermissionDao = PermissionDao;
}

type AssemblyService =
    genossi_service_impl::assembly::AssemblyServiceImpl<AssemblyServiceDependencies>;
```
**Für Phase 7 minimaler:**
```rust
type RepaymentPhaseDao = genossi_dao_impl_sqlite::repayment_phase::RepaymentPhaseDaoImpl;

pub struct RepaymentPhaseServiceDependencies;
unsafe impl Send for RepaymentPhaseServiceDependencies {}
unsafe impl Sync for RepaymentPhaseServiceDependencies {}

impl genossi_service_impl::repayment_phase::RepaymentPhaseServiceDeps
    for RepaymentPhaseServiceDependencies
{
    type Context = Context;
    type Transaction = Transaction;
    type RepaymentPhaseDao = RepaymentPhaseDao;
    type AuditLogDao = AuditLogDao;
    type PermissionService = PermissionService;
    type UuidService = UuidService;
    type TransactionDao = TransactionDao;
}

type RepaymentPhaseService =
    genossi_service_impl::repayment_phase::RepaymentPhaseServiceImpl<RepaymentPhaseServiceDependencies>;
```

**Service-Konstruktion in `RestStateImpl::new()`** (Z. 658-671):
```rust
let assembly_service = Arc::new(genossi_service_impl::assembly::AssemblyServiceImpl {
    assembly_dao: assembly_dao.clone(),
    assembly_member_snapshot_dao: assembly_member_snapshot_dao.clone(),
    member_dao: member_dao.clone(),
    audit_log_dao: audit_log_dao.clone(),
    permission_service: permission_service.clone(),
    uuid_service: uuid_service.clone(),
    transaction_dao: transaction_dao.clone(),
    helper_token_dao: helper_token_dao.clone(),
    permission_dao: permission_dao.clone(),
});
```
**Für Phase 7:**
```rust
let repayment_phase_dao = Arc::new(RepaymentPhaseDao::new(pool.clone()));
let repayment_phase_service = Arc::new(
    genossi_service_impl::repayment_phase::RepaymentPhaseServiceImpl {
        repayment_phase_dao,
        audit_log_dao: audit_log_dao.clone(),
        permission_service: permission_service.clone(),
        uuid_service: uuid_service.clone(),
        transaction_dao: transaction_dao.clone(),
    },
);
```
Field im `Self { ... }`-Aggregat ergänzen (Z. 789 zwischen `assembly_service` und `helper_token_service`).

**RestState-Trait-Impl** (Z. 1258-1264):
```rust
impl genossi_rest::assembly::AssemblyRestState for RestStateImpl {
    type AssemblyService = AssemblyService;

    fn assembly_service(&self) -> Arc<Self::AssemblyService> {
        self.assembly_service.clone()
    }
}
```
→ exakt das analoge `impl genossi_rest::repayment_phase::RepaymentPhaseRestState for RestStateImpl` ergänzen.

**Struct-Field in `RestStateImpl`** (vor `Self { ... }` in Z. 446-Bereich):
```rust
pub struct RestStateImpl {
    // ...
    assembly_service: Arc<AssemblyService>,
    repayment_phase_service: Arc<RepaymentPhaseService>,  // NEU
    // ...
}
```

---

### 11) `genossi_bin/tests/e2e_tests.rs` — E2E-Test (Lifecycle + Audit-Hashchain)

**Analog:** `test_assembly_lifecycle_audit_chain_intact` (Z. 8361-8514)

**Pattern: Create → Open → Update share_value → Close → /api/audit/verify** (Auszüge):
```rust
#[tokio::test]
async fn test_assembly_lifecycle_audit_chain_intact() {
    let server = setup().await;
    let client = reqwest::Client::new();

    // 1) Create assembly (status=Preparation)
    let create_body = serde_json::json!({
        "name": "GV 2026",
        "date": "2026-06-15T18:00:00.000000000Z",
        "location": "Vereinsheim",
    });
    let response = client.post(server.url("/api/assembly"))
        .json(&create_body).send().await.unwrap();
    assert_eq!(response.status(), StatusCode::CREATED, "create should return 201");
    let created: AssemblyTO = response.json().await.unwrap();
    let assembly_id = created.id;
    assert_eq!(created.status, AssemblyStatusTO::Preparation);

    // 2) Open assembly
    let response = client.post(server.url(&format!("/api/assembly/{}/open", assembly_id)))
        .send().await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let opened: AssemblyTO = response.json().await.unwrap();
    assert_eq!(opened.status, AssemblyStatusTO::Open);
    assert!(opened.opened_at.is_some());

    // 3) Close assembly
    let response = client.post(server.url(&format!("/api/assembly/{}/close", assembly_id)))
        .send().await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // 4) Verify audit hash chain intact
    let response = client.get(server.url("/api/audit/verify"))
        .send().await.unwrap();
    let verify: VerifyResponseTO = response.json().await.unwrap();
    assert!(verify.valid, "Audit hash chain must be valid after lifecycle");
    assert!(verify.broken_links.is_empty());
    assert!(verify.total_entries >= 3);

    // 5) Verify each lifecycle process appears in the audit log
    let response = client.get(server.url(&format!("/api/audit/assembly/{}", assembly_id)))
        .send().await.unwrap();
    let entries: Vec<AuditLogEntryTO> = response.json().await.unwrap();
    let processes: HashSet<&str> = entries.iter().map(|e| e.process.as_str()).collect();
    assert!(processes.contains("assembly.create"));
    assert!(processes.contains("assembly.open"));
    assert!(processes.contains("assembly.close"));
}
```

**Conflict-Test-Pattern** (e2e_tests.rs Z. 8517-8551 `test_close_assembly_from_preparation_returns_conflict`, Z. 8551-... `test_open_assembly_from_closed_returns_conflict`):
```rust
let response = client.post(server.url(&format!("/api/assembly/{}/close", created.id)))
    .send().await.unwrap();
assert_eq!(response.status(), StatusCode::CONFLICT,
    "close from Preparation must return 409");
```

**Phase-7-Test-Anforderungen** (siehe CONTEXT.md `<domain>` "E2E-Test"):
1. `test_repayment_phase_lifecycle_audit_chain_intact` — create → open → **update share_value (D-04: share_value EDIT in Open)** → close, dann `/api/audit/verify` valide, Prozesse `"repayment-phase.create"`/`".open"`/`".update"`/`".close"` im Audit-Log
2. `test_update_repayment_phase_fiscal_year_in_open_returns_conflict` (D-04 / D-07): create → open → PUT mit verändertem `fiscal_year` → 409
3. `test_close_repayment_phase_from_preparation_returns_conflict` (D-05/D-06)
4. `test_open_repayment_phase_from_closed_returns_conflict` (D-06)
5. `test_delete_repayment_phase_in_open_returns_conflict` (D-09): create → open → DELETE → 409
6. `test_validation_fiscal_year_out_of_range_returns_400` (D-11): POST mit `fiscal_year=1999` → 400 (Service maps `ValidationError` zu `BadRequest`, siehe `genossi_rest/src/lib.rs:101-107`)
7. `test_validation_share_value_zero_returns_400` (D-12): POST mit `share_value=0` → 400

**Domain-Substitutionen im JSON-Body:**
```rust
let create_body = serde_json::json!({
    "fiscal_year": 2026,
    "share_value": 12000,
});
```

---

## Shared Patterns

### Authentication / Authorization

**Source:** `genossi_service_impl/src/assembly.rs:81-88`
**Apply to:** Alle 6 RepaymentPhase-Service-Methoden
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
Hinweis: `ADMIN_PRIVILEGE = "admin"` (assembly.rs Z. 50).

### REST-Auth-Extraction

**Source:** `genossi_rest/src/assembly.rs:124, 159, 209, 245, 297, 333`
**Apply to:** Alle 7 RepaymentPhase-REST-Handler
```rust
let auth = crate::extract_auth_context(Some(context))?;
```

### Error-Handler-Wrapper

**Source:** `genossi_rest/src/assembly.rs:122-137` (pattern: `error_handler((async { ... }).await)`)
**Apply to:** Alle 7 RepaymentPhase-REST-Handler — handlers konvertieren `RestError` via `From<ServiceError>` (`genossi_rest/src/lib.rs:97-113`):
- `ServiceError::EntityNotFound(_)` → `404`
- `ServiceError::ValidationError(items)` → `400` mit `"field: message, field: message"`
- `ServiceError::PermissionDenied` → `401`
- `ServiceError::Conflict(msg)` → `409` mit msg
- alles andere → `500`

### Audit-Logging-Pattern

**Source:** `genossi_service_impl/src/audit_macros.rs` + `genossi_service_impl/src/assembly.rs:106-113, 167-175, 220-228, 297-305`
**Apply to:** RepaymentPhase create/update/open/close/delete

| Action | Macro | Process-Konstante |
|--------|-------|-------------------|
| create | `audited_create!` | `"repayment-phase.create"` |
| update (PUT) | `audited_update!` | `"repayment-phase.update"` |
| open (POST /open) | `audited_update!` | `"repayment-phase.open"` |
| close (POST /close) | `audited_update!` | `"repayment-phase.close"` |
| delete (DELETE) | `audited_delete!` | `"repayment-phase.delete"` |

Aufruf-Form (exakt wie assembly.rs Z. 106-113):
```rust
crate::audited_create!(
    self,
    self.repayment_phase_dao,
    &entity,
    REPAYMENT_PHASE_PROCESS_CREATE,
    &user_id,
    tx
);
```
**Voraussetzung:** Im Service-Impl-Struct müssen `audit_log_dao: Arc<...>` und `uuid_service: Arc<...>` Felder vorhanden sein (vom `gen_service_impl!`-Macro automatisch).

### Field-Level-Validation

**Source:** `ServiceError::ValidationError(Vec<ValidationFailureItem>)` aus `genossi_service/src/lib.rs:29, 38-42` + `From<ServiceError> for RestError`-Mapping in `genossi_rest/src/lib.rs:101-107` (→ 400 BadRequest mit Feld-Hinweisen)
**Apply to:** `create_repayment_phase` und `update_repayment_phase` (vor jeder Mutation)

Pattern (eigene Implementation, da `genossi_service_impl/src/validation.rs` ein anderes Subsystem ist):
```rust
let mut errors: Vec<genossi_service::ValidationFailureItem> = Vec::new();
if !(2000..=2100).contains(&input.fiscal_year) {
    errors.push(genossi_service::ValidationFailureItem {
        field: Arc::from("fiscal_year"),
        message: Arc::from(format!("must be in 2000..=2100, got {}", input.fiscal_year)),
    });
}
if input.share_value <= 0 {
    errors.push(genossi_service::ValidationFailureItem {
        field: Arc::from("share_value"),
        message: Arc::from("must be > 0 (Cent)"),
    });
}
if !errors.is_empty() {
    return Err(ServiceError::ValidationError(errors));
}
```

### Lifecycle-Guard-Pattern (D-05 Conflict)

**Source:** `genossi_service_impl/src/assembly.rs:152-157, 207-211, 285-289`
**Apply to:** `update_repayment_phase`, `open_repayment_phase`, `close_repayment_phase`, `delete_repayment_phase`
```rust
if entity.status != EXPECTED_STATUS {
    return Err(ServiceError::Conflict(Arc::from(format!(
        "Cannot <action>: status is '{}', expected '<EXPECTED>'",
        entity.status.as_str()
    ))));
}
```

### Optimistic-Locking-Pattern (PUT)

**Source:** `genossi_service_impl/src/assembly.rs:159-161`
**Apply to:** NUR `update_repayment_phase` (D-03: lifecycle-Endpoints prüfen KEIN version-Field)
```rust
if entity.version != update.version {
    return Err(ServiceError::Conflict(Arc::from("Version mismatch")));
}
```

### ISO8601-Datetime-Serde

**Source:** `genossi_rest_types/src/lib.rs:10` (custom module `iso8601_datetime`)
**Apply to:** Alle `opened_at`/`closed_at`/`created`/`deleted`-Felder in `RepaymentPhaseTO`
```rust
#[serde(
    serialize_with = "iso8601_datetime::serialize",
    deserialize_with = "iso8601_datetime::deserialize",
    default
)]
pub opened_at: Option<time::PrimitiveDateTime>,
```

### Tracing-Instrumentation

**Source:** `genossi_rest/src/assembly.rs:108, 140, 190, 222, 277, 313`
**Apply to:** Alle REST-Handler
```rust
#[instrument(skip(rest_state))]
```

---

## No Analog Found

Keine. **Alle** Phase-7-Dateien haben einen direkten Analog im Assembly-Aggregat (Phase 1 Plan 03 hat das Pattern etabliert). DELETE-Pattern kommt aus `genossi_rest/src/member.rs:195-222` + `genossi_service_impl/src/member.rs:354-383`, beide ebenfalls geprüft.

---

## Metadata

**Analog search scope:**
- `migrations/sqlite/` (alle Migrationen)
- `genossi_dao/src/` (assembly, auditable, member, application)
- `genossi_dao_impl_sqlite/src/` (assembly)
- `genossi_service/src/` (assembly, lib)
- `genossi_service_impl/src/` (assembly, audit_macros, macros, member, validation)
- `genossi_rest/src/` (assembly, member, lib)
- `genossi_rest_types/src/lib.rs` (Z. 1000-1142 AssemblyTO-Block)
- `genossi_bin/src/lib.rs` (Assembly-Wiring Z. 120-200, 565-720, 780-825, 1258-1265)
- `genossi_bin/tests/e2e_tests.rs` (Assembly-Lifecycle-E2E Z. 8350-8600)

**Files scanned:** 14 (alle Treffer hatten direkten Analog im Assembly-Aggregat)
**Pattern extraction date:** 2026-05-29
