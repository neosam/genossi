# Phase 8: RepaymentEntry + Auto-Befüllung — Pattern Map

**Mapped:** 2026-05-30
**Files analyzed:** 14 (8 NEW + 6 MODIFY)
**Analogs found:** 14 / 14 (alle haben starke Vorlagen in Phase 7 + Phase 1)

> **Lese-Reihenfolge für Planner und Executor:**
>
> 1. `08-CONTEXT.md` (User-Entscheidungen, Single Source of Truth für Decisions)
> 2. Dieses Dokument (PATTERNS.md) — sagt **WO** kopiert werden soll und **WAS** als 1:1-Vorlage gilt
> 3. Bei Bedarf: die jeweils unter "Analog" gelisteten Dateien in der Codebase
>
> **Goldene Regel für Phase 8:** Wo ein Pattern aus Phase 7 (`repayment_phase`) existiert, ist es die **erste Wahl**. Wo der Phase-1-Assembly-Pfad ein Pattern hat, das Phase 7 noch nicht brauchte (z.B. `tx.clone()` Multi-DAO-Coordination, Snapshot/Aggregat-Auto-Fill), kommt es daher. Frische Patterns dürfen nur, wo beide Vorlagen fehlen.

---

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| NEW `migrations/sqlite/{date}_create_repayment_entry_table.sql` | migration | schema-DDL | `migrations/sqlite/20260529190437_create_repayment_phase_table.sql` + `20260504000000_create_attendance_table.sql` (Indizes) | exact (Phase 7 für Tabellenform, Phase 3 für Index-Stil) |
| NEW `genossi_dao/src/repayment_entry.rs` | DAO trait + Entity + Auditable | CRUD + audit | `genossi_dao/src/repayment_phase.rs` | exact |
| NEW `genossi_dao_impl_sqlite/src/repayment_entry.rs` | DAO impl (SQLite) | CRUD | `genossi_dao_impl_sqlite/src/repayment_phase.rs` | exact |
| NEW `genossi_service/src/repayment_entry.rs` | Service trait + Domain-Typ + DTOs | request-response | `genossi_service/src/repayment_phase.rs` | exact |
| NEW `genossi_service_impl/src/repayment_entry.rs` | Service impl | CRUD + audit + validation + batch-tx | `genossi_service_impl/src/repayment_phase.rs` (Struktur) + `genossi_service_impl/src/assembly.rs:181-259` (Auto-Fill in `open_phase`) | exact (kombiniert) |
| MODIFY `genossi_service_impl/src/repayment_phase.rs` | Service impl Extension | event-driven (`open_phase`) + validation-guard (`close_phase`) | `genossi_service_impl/src/assembly.rs:181-259` (`open_assembly` Auto-Fill) + `assembly.rs:261-341` (`close_assembly` Status-Guard) | exact |
| NEW `genossi_rest/src/repayment_entry.rs` | REST handler + utoipa-Schema | request-response (REST, JSON) | `genossi_rest/src/repayment_phase.rs` (1:1) + `genossi_rest/src/attendance.rs` (für Batch-/Action-Endpoint-Pattern + lokales Error-Mapping falls nötig) | exact |
| MODIFY `genossi_rest_types/src/lib.rs` | Transfer-Objects + utoipa-Schemas | serde + utoipa | `genossi_rest_types/src/lib.rs:1144-1259` (RepaymentPhaseTO-Block) | exact |
| MODIFY `genossi_bin/src/lib.rs` | DI-Wiring | request-response (setup) | `genossi_bin/src/lib.rs:176-200, 701-713, 1311-1316` (RepaymentPhase-DI) | exact |
| MODIFY `genossi_rest/src/lib.rs` | Router + OpenAPI-Nest | request-response (setup) | `genossi_rest/src/lib.rs:20, 270, 438, 610-612` (RepaymentPhase-Registration) | exact |
| MODIFY `genossi_dao/src/lib.rs` | module declaration | n/a | bestehende `pub mod`-Reihe in alphabetischer Sortierung | exact |
| MODIFY `genossi_dao_impl_sqlite/src/lib.rs` | module declaration | n/a | bestehende `pub mod`-Reihe | exact |
| MODIFY `genossi_service/src/lib.rs` | module declaration | n/a | bestehende `pub mod`-Reihe | exact |
| MODIFY `genossi_service_impl/src/lib.rs` | module declaration | n/a | bestehende `pub mod`-Reihe | exact |
| MODIFY `genossi_bin/tests/e2e_tests.rs` | E2E test | request-response (full stack via HTTP) | `genossi_bin/tests/e2e_tests.rs:10553-10999` (RepaymentPhase-E2E-Block + `create_preparation_repayment_phase`-Helper) | exact |
| ~~MODIFY `genossi_service/src/validation.rs`~~ | (NICHT empfohlen) | n/a | Plan 07-03 hat bewusst **inline** `validate_phase_fields`-Helper in `genossi_service_impl/src/repayment_phase.rs:64-85` statt `validation.rs`-Erweiterung gewählt (`validation.rs` ist Mitglieder-Konsistenzberichte-Concern). Phase 8 sollte das gleiche Inline-Muster verwenden (Helper `validate_entry_fields(share_count_to_pay_out, member_current_shares)` direkt in `genossi_service_impl/src/repayment_entry.rs`). | n/a — siehe Begründung |

> **Hinweis zur fehlenden `validation.rs`-Erweiterung:** Das CONTEXT.md listet `MODIFY: genossi_service/src/validation.rs (new validator)` als zu modifizierende Datei. Phase 7 hat aber **dezidiert** den umgekehrten Weg gewählt (Plan 07-03 D-04 in STATE.md: „Inline-Field-Validator statt Erweiterung von validation.rs — validation.rs ist anderer Concern (Mitglieder-Konsistenzberichte)"). PATTERNS.md empfiehlt dem Planner, dieser etablierten Konvention zu folgen und den Validator inline in `genossi_service_impl/src/repayment_entry.rs` zu platzieren. Endgültige Entscheidung beim Planner.

---

## Pattern Assignments

### 1. Migration: `migrations/sqlite/{date}_create_repayment_entry_table.sql`

**Analog:** `migrations/sqlite/20260529190437_create_repayment_phase_table.sql` (Tabellenstruktur) + `migrations/sqlite/20260504000000_create_attendance_table.sql` (Indizes-Pattern + FK-Doku-Konvention)

**Tabellen-DDL-Pattern** (Phase-7-Vorlage, Z. 1-15):
```sql
CREATE TABLE IF NOT EXISTS repayment_phase (
    id BLOB PRIMARY KEY NOT NULL,
    fiscal_year INTEGER NOT NULL,
    share_value INTEGER NOT NULL,
    status TEXT NOT NULL DEFAULT 'Preparation',
    opened_at TEXT,
    closed_at TEXT,
    created TEXT NOT NULL,
    deleted TEXT,
    version BLOB NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_repayment_phase_status ON repayment_phase(status);
CREATE INDEX IF NOT EXISTS idx_repayment_phase_deleted ON repayment_phase(deleted);
```

**FK-Dokumentation als Kommentar** (Phase-3-Konvention aus `attendance`-Migration, Z. 4-9):
```sql
-- NOTE (WR-03): FOREIGN KEY clauses below are DOCUMENTARY only.
-- This codebase does not enable `PRAGMA foreign_keys=ON`. The Service layer
-- performs an explicit membership check before any INSERT, which is the
-- operative protection. The FK clauses document the intended referential
-- semantics for future operators reading the schema.
```

**Konkrete Phase-8-Anwendung** (Planner adaptiert):
- Spalten: `id BLOB PK`, `member_id BLOB NOT NULL`, `phase_id BLOB NOT NULL`, `share_count_to_pay_out INTEGER NOT NULL CHECK(share_count_to_pay_out > 0)`, `status TEXT NOT NULL DEFAULT 'Open'`, `created TEXT NOT NULL`, `deleted TEXT`, `version BLOB NOT NULL`
- FKs als Dokumentation (kein `PRAGMA foreign_keys=ON`): `FOREIGN KEY (member_id) REFERENCES member(id) ON DELETE RESTRICT`, `FOREIGN KEY (phase_id) REFERENCES repayment_phase(id) ON DELETE RESTRICT`
- Indizes (CONTEXT.md Claude's Discretion): `idx_repayment_entry_phase` auf `phase_id`, `idx_repayment_entry_phase_status` auf `(phase_id, status)`, `idx_repayment_entry_deleted` auf `deleted`
- KEIN UNIQUE-Constraint auf `(member_id, phase_id)` — ENTR-03 explizit erlaubt mehrere Einträge pro Member+Phase

---

### 2. DAO trait + Entity: `genossi_dao/src/repayment_entry.rs`

**Analog:** `genossi_dao/src/repayment_phase.rs` (1:1-Vorlage)

**Status-Enum-Pattern** (Z. 9-42):
```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RepaymentPhaseStatus {
    Preparation,
    Open,
    Closed,
}

impl RepaymentPhaseStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            RepaymentPhaseStatus::Preparation => "Preparation",
            RepaymentPhaseStatus::Open => "Open",
            RepaymentPhaseStatus::Closed => "Closed",
        }
    }

    pub fn from_str(s: &str) -> Result<Self, DaoError> {
        match s {
            "Preparation" => Ok(RepaymentPhaseStatus::Preparation),
            "Open" => Ok(RepaymentPhaseStatus::Open),
            "Closed" => Ok(RepaymentPhaseStatus::Closed),
            _ => Err(DaoError::ParseError(Arc::from(format!(
                "Unknown repayment phase status: {}",
                s
            )))),
        }
    }
}

impl Default for RepaymentPhaseStatus { ... }
```

→ Phase 8 ersetzt durch `RepaymentEntryStatus { Open, Contacted, PaidOut }` (CONTEXT D-05: **alle drei Varianten von Anfang an in DAO**, auch wenn Phase 8 selbst noch keinen PaidOut-Toggle erlaubt; Default `Open`).

**Entity-Pattern** (Z. 44-55):
```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RepaymentPhaseEntity {
    pub id: Uuid,
    pub fiscal_year: i32,
    pub share_value: i64,
    pub status: RepaymentPhaseStatus,
    pub opened_at: Option<time::PrimitiveDateTime>,
    pub closed_at: Option<time::PrimitiveDateTime>,
    pub created: time::PrimitiveDateTime,
    pub deleted: Option<time::PrimitiveDateTime>,
    pub version: Uuid,
}
```

→ Phase 8 ersetzt durch `RepaymentEntryEntity { id, member_id: Uuid, phase_id: Uuid, share_count_to_pay_out: i32, status: RepaymentEntryStatus, created, deleted, version }` (kein opened_at/closed_at — das gehört zur Phase, nicht zum Entry).

**Auditable-Impl-Pattern** (Z. 57-92):
```rust
impl crate::auditable::Auditable for RepaymentPhaseEntity {
    fn entity_type() -> &'static str {
        "repayment_phase"
    }

    fn entity_id(&self) -> Uuid {
        self.id
    }

    fn audit_fields(&self) -> Vec<(&'static str, Option<String>)> {
        vec![
            ("fiscal_year", Some(self.fiscal_year.to_string())),
            ("share_value", Some(self.share_value.to_string())),
            ("status", Some(self.status.as_str().to_string())),
            ("opened_at", self.opened_at.as_ref().map(format_dt)),
            ("closed_at", self.closed_at.as_ref().map(format_dt)),
        ]
    }
}
```

→ Phase 8: `entity_type() = "repayment_entry"`, `audit_fields` enthält **genau** `member_id`, `phase_id`, `share_count_to_pay_out`, `status` (NICHT `id`/`version`/`created`/`deleted` per Konvention; CONTEXT in-scope §3).

> **Wichtig — Audit-Field-Reihenfolge ist frozen** (Phase 7 Plan 07-01-Lektion in STATE.md): "Audit-fields-Reihenfolge … ist frozen — Plan 03 Service-Tests müssen diese Reihenfolge per Unit-Test einfrieren, weil spätere Reihenfolge-Änderung historische Audit-Einträge brechen würde." Phase 8 muss einen analogen Test schreiben (`test_auditable_fields_count_and_excludes_metadata`, Vorlage `genossi_dao/src/repayment_phase.rs:218-245`).

**DAO-Trait-Pattern mit `#[automock]`** (Z. 94-142):
```rust
#[automock(type Transaction = crate::MockTransaction;)]
#[async_trait]
pub trait RepaymentPhaseDao {
    type Transaction: crate::Transaction;

    async fn dump_all(&self, tx: Self::Transaction) -> Result<Arc<[RepaymentPhaseEntity]>, DaoError>;

    async fn create(&self, entity: &RepaymentPhaseEntity, process: &str, tx: Self::Transaction) -> Result<(), DaoError>;

    async fn update(&self, entity: &RepaymentPhaseEntity, process: &str, tx: Self::Transaction) -> Result<(), DaoError>;

    async fn all(&self, tx: Self::Transaction) -> Result<Arc<[RepaymentPhaseEntity]>, DaoError> {
        let all_entities = self.dump_all(tx).await?;
        let active_entities: Vec<RepaymentPhaseEntity> = all_entities
            .iter().filter(|e| e.deleted.is_none()).cloned().collect();
        Ok(active_entities.into())
    }

    async fn find_by_id(&self, id: Uuid, tx: Self::Transaction) -> Result<Option<RepaymentPhaseEntity>, DaoError> {
        let all_entities = self.dump_all(tx).await?;
        Ok(all_entities.iter().find(|e| e.id == id && e.deleted.is_none()).cloned())
    }
}
```

→ Phase 8 ergänzt **eine** Domain-Methode für Listing und Close-Validation (Planner darf entscheiden: DAO-Method oder Service-Layer-Filter über `all`). Empfehlung: **`async fn find_by_phase_id(&self, phase_id: Uuid, tx) -> Result<Arc<[RepaymentEntryEntity]>, DaoError>` als Default-Impl auf dem Trait** (gleicher Filter-Stil wie `all`/`find_by_id`). Begründung: Phase-7-Konvention von "DAO-Default-Impls für einfache Filter über `dump_all`" (siehe `member.rs:172` `count_active`, `member.rs:160` `find_by_member_number`).

**Tests-Pattern** (Z. 144-258, **Phase 8 muss diese Test-Suite spiegeln**):
- `test_repayment_entry_status_roundtrip` (alle 3 Varianten via `as_str` / `from_str`)
- `test_repayment_entry_status_strings_are_english` (CONTEXT D-05: Statusstrings analog Phase 7 D-01 in Englisch)
- `test_repayment_entry_status_invalid_string` (deutsche Strings dürfen NICHT parsen)
- `test_auditable_entity_type_is_repayment_entry`
- `test_auditable_fields_count_and_excludes_metadata` (genau 4 Felder: member_id, phase_id, share_count_to_pay_out, status)
- `test_auditable_diff_detects_status_change`

---

### 3. DAO impl (SQLite): `genossi_dao_impl_sqlite/src/repayment_entry.rs`

**Analog:** `genossi_dao_impl_sqlite/src/repayment_phase.rs` (1:1-Vorlage)

**Imports + Db-Row-Pattern** (Z. 1-28):
```rust
use async_trait::async_trait;
use genossi_dao::repayment_phase::{
    RepaymentPhaseDao, RepaymentPhaseEntity, RepaymentPhaseStatus,
};
use genossi_dao::DaoError;
use sqlx::SqlitePool;
use std::sync::Arc;
use time::PrimitiveDateTime;
use uuid::Uuid;

use crate::assembly::parse_datetime;   // cross-module helper REUSE — NICHT duplizieren
use crate::TransactionImpl;

#[derive(Debug, sqlx::FromRow)]
struct RepaymentPhaseDb {
    id: Vec<u8>,
    fiscal_year: i64,   // SQLite INTEGER ist 8-Byte; in TryFrom guarded auf i32 gecastet
    share_value: i64,
    status: String,
    opened_at: Option<String>,
    closed_at: Option<String>,
    created: String,
    deleted: Option<String>,
    version: Vec<u8>,
}
```

→ Phase 8: `RepaymentEntryDb { id: Vec<u8>, member_id: Vec<u8>, phase_id: Vec<u8>, share_count_to_pay_out: i64 (guarded auf i32), status: String, created: String, deleted: Option<String>, version: Vec<u8> }`. `parse_datetime` aus `crate::assembly` wiederverwenden (Phase-7-Lektion Plan 07-02 in STATE.md).

**TryFrom-Pattern mit guarded i32-Cast** (Z. 30-58):
```rust
impl TryFrom<&RepaymentPhaseDb> for RepaymentPhaseEntity {
    type Error = DaoError;

    fn try_from(db: &RepaymentPhaseDb) -> Result<Self, Self::Error> {
        Ok(RepaymentPhaseEntity {
            id: Uuid::from_slice(&db.id)?,
            fiscal_year: i32::try_from(db.fiscal_year).map_err(|e| {
                DaoError::ParseError(Arc::from(format!(
                    "fiscal_year out of i32 range: {}", e
                )))
            })?,
            share_value: db.share_value,
            status: RepaymentPhaseStatus::from_str(&db.status)?,
            opened_at: db.opened_at.as_ref().map(|s| parse_datetime(s)).transpose()?,
            closed_at: db.closed_at.as_ref().map(|s| parse_datetime(s)).transpose()?,
            created: parse_datetime(&db.created)?,
            deleted: db.deleted.as_ref().map(|d| parse_datetime(d)).transpose()?,
            version: Uuid::from_slice(&db.version)?,
        })
    }
}
```

→ Phase 8: `share_count_to_pay_out: i32::try_from(db.share_count_to_pay_out).map_err(...)?` (gleiche T-07-02-05-Mitigation). `member_id`/`phase_id` via `Uuid::from_slice(&db.member_id)?`.

**`DaoImpl::new(Arc<SqlitePool>)`-Constructor** (Z. 61-68, **Phase-7-Lektion Plan 07-02 in STATE.md**):
```rust
pub struct RepaymentPhaseDaoImpl {
    pub pool: Arc<SqlitePool>,
}

impl RepaymentPhaseDaoImpl {
    pub fn new(pool: Arc<SqlitePool>) -> Self {
        Self { pool }
    }
}
```

→ Phase 8: identisch — `RepaymentEntryDaoImpl::new(pool: Arc<SqlitePool>)`. `genossi_bin/src/lib.rs` erwartet diese Signatur.

**dump_all/create/update mit Pre-Exists-Check + Optimistic-Locking** (Z. 78-198) — komplettes Pattern für Phase 8 zu spiegeln:
```rust
async fn dump_all(&self, tx: Self::Transaction) -> Result<Arc<[RepaymentPhaseEntity]>, DaoError> {
    let rows = sqlx::query_as::<_, RepaymentPhaseDb>(
        "SELECT id, fiscal_year, share_value, status, opened_at, closed_at, created, \
         deleted, version FROM repayment_phase \
         ORDER BY fiscal_year DESC, created DESC",
    )
    .fetch_all(tx.tx.lock().await.as_mut())
    .await
    .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?;

    rows.iter().map(RepaymentPhaseEntity::try_from)
        .collect::<Result<Vec<_>, _>>()
        .map(|v| v.into())
}

async fn create(...) -> Result<(), DaoError> {
    // INSERT INTO repayment_phase (id, fiscal_year, ...) VALUES (?, ?, ...)
    ...
}

async fn update(&self, entity: ..., tx: ...) -> Result<(), DaoError> {
    // Pre-Exists-Check trennt NotFound von ConflictError (Phase-7-Lektion Plan 07-02 D-03 in STATE.md)
    let exists = sqlx::query_scalar::<_, i32>(
        "SELECT COUNT(*) FROM repayment_phase WHERE id = ? AND deleted IS NULL",
    )
    .bind(id.clone())
    .fetch_one(tx.tx.lock().await.as_mut())
    .await
    .map_err(|e| DaoError::DatabaseError(Arc::from(e.to_string())))?;

    if exists == 0 {
        return Err(DaoError::NotFound);
    }

    // UPDATE ... WHERE id = ? AND version = ? AND deleted IS NULL
    let rows_affected = sqlx::query(...)
        .execute(tx.tx.lock().await.as_mut())
        .await?
        .rows_affected();

    if rows_affected == 0 {
        return Err(DaoError::ConflictError(Arc::from("Version mismatch")));
    }
    Ok(())
}
```

→ Phase 8: `ORDER BY` für RepaymentEntries (Listing über `dump_all`/`all`) — Empfehlung `ORDER BY created ASC` oder `ORDER BY (SELECT member_number FROM member WHERE id = repayment_entry.member_id) ASC` für deterministische Audit-Reihenfolge (CONTEXT Claude's Discretion: "Sortierung der `audited_create!`-Calls für deterministische Audit-Reihenfolge"). Pre-Exists-Check + Optimistic-Locking 1:1 spiegeln.

**Tests-Pattern** (Z. 201-366):
- `setup_db()` mit inline CREATE TABLE DDL (NICHT `include_str!` auf Migration — Phase-7-Konvention)
- `test_create_and_find_repayment_entry`
- `test_update_repayment_entry_with_version_mismatch_returns_conflict`
- `test_update_repayment_entry_unknown_id_returns_not_found`
- `test_update_repayment_entry_succeeds_then_version_changes`

---

### 4. Service trait: `genossi_service/src/repayment_entry.rs`

**Analog:** `genossi_service/src/repayment_phase.rs` (1:1-Vorlage, inkl. Doc-Header-Stil)

**Domain-Typ + From-Impls-Pattern** (Z. 33-75):
```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RepaymentPhase {
    pub id: Uuid,
    pub fiscal_year: i32,
    pub share_value: i64,
    pub status: RepaymentPhaseStatus,
    pub opened_at: Option<time::PrimitiveDateTime>,
    pub closed_at: Option<time::PrimitiveDateTime>,
    pub created: time::PrimitiveDateTime,
    pub deleted: Option<time::PrimitiveDateTime>,
    pub version: Uuid,
}

impl From<&RepaymentPhaseEntity> for RepaymentPhase { ... feldweise ... }
impl From<&RepaymentPhase> for RepaymentPhaseEntity { ... feldweise ... }
```

→ Phase 8: `RepaymentEntry { id, member_id, phase_id, share_count_to_pay_out, status, created, deleted, version }` + bidirektionale `From`-Impls.

**Submission/Update-DTO-Pattern** (Z. 82-101):
```rust
#[derive(Clone, Debug)]
pub struct RepaymentPhaseSubmission {
    pub fiscal_year: i32,
    pub share_value: i64,
}

#[derive(Clone, Debug)]
pub struct RepaymentPhaseUpdate {
    pub fiscal_year: i32,
    pub share_value: i64,
    pub version: Uuid,    // optimistic locking, Pflicht
}
```

→ Phase 8:
- `RepaymentEntrySubmission { phase_id: Uuid, member_id: Uuid, share_count_to_pay_out: i32 }`
- `RepaymentEntryUpdate { share_count_to_pay_out: Option<i32>, status: Option<RepaymentEntryStatus>, version: Uuid }` (CONTEXT Claude's Discretion D-12: "Wenn ein Feld nicht im Body steht, bleibt es unverändert"; **PaidOut als Target → Service liefert 409**)
- `RepaymentEntryBatchStatusInput { entry_ids: Arc<[Uuid]>, target_status: RepaymentEntryStatus }` (PaidOut als target → Service liefert 400)

**Service-Trait mit `#[automock]`** (Z. 103-178):
```rust
#[automock(type Context = (); type Transaction = genossi_dao::MockTransaction;)]
#[async_trait]
pub trait RepaymentPhaseService {
    type Context: Clone + Debug + PartialEq + Eq + Send + Sync + 'static;
    type Transaction: genossi_dao::Transaction;

    async fn create_repayment_phase(&self, submission: ..., context: Authentication<Self::Context>) -> Result<RepaymentPhase, ServiceError>;
    async fn update_repayment_phase(&self, id: Uuid, update: ..., context: ...) -> Result<RepaymentPhase, ServiceError>;
    async fn open_repayment_phase(&self, id: Uuid, context: ...) -> Result<RepaymentPhase, ServiceError>;
    async fn close_repayment_phase(&self, id: Uuid, context: ...) -> Result<RepaymentPhase, ServiceError>;
    async fn delete_repayment_phase(&self, id: Uuid, context: ...) -> Result<(), ServiceError>;
    async fn get_repayment_phase(&self, id: Uuid, context: ...) -> Result<RepaymentPhase, ServiceError>;
    async fn get_all_repayment_phases(&self, context: ...) -> Result<Arc<[RepaymentPhase]>, ServiceError>;
}
```

→ Phase 8 Service-Trait-Methoden (CONTEXT D-12 plus zwei Listing-/Batch-Methoden):
- `create_repayment_entry(&self, submission: &RepaymentEntrySubmission, ctx) -> Result<RepaymentEntry, ServiceError>`
- `update_repayment_entry(&self, id: Uuid, update: &RepaymentEntryUpdate, ctx) -> Result<RepaymentEntry, ServiceError>`
- `delete_repayment_entry(&self, id: Uuid, ctx) -> Result<(), ServiceError>`
- `get_repayment_entry(&self, id: Uuid, ctx) -> Result<RepaymentEntry, ServiceError>`
- `list_repayment_entries_by_phase(&self, phase_id: Uuid, ctx) -> Result<Arc<[RepaymentEntry]>, ServiceError>`
- `batch_toggle_status(&self, input: &RepaymentEntryBatchStatusInput, ctx) -> Result<Arc<[RepaymentEntry]>, ServiceError>` (D-08 all-or-nothing in einer Tx)

**Tests-Pattern** (Z. 180-265): `entity_to_repayment_entry_roundtrip`, `test_repayment_entry_submission_constructible`, `test_repayment_entry_update_requires_version`, `test_mock_repayment_entry_service_compiles` (mit `expect_*` für jede Trait-Methode).

---

### 5. Service impl: `genossi_service_impl/src/repayment_entry.rs`

**Analog:** `genossi_service_impl/src/repayment_phase.rs` (Struktur, Process-Konstanten, Validation-Helper, `gen_service_impl!`, Test-Mock-Setup)

**Doc-Header-Pattern** (Z. 1-25): Lifecycle-Übersicht, Edit-Matrix, Audit-Disziplin-Warnung, Pattern-Anker — Phase 8 spiegelt 1:1 mit RepaymentEntry-Inhalt.

**Process-Konstanten + gen_service_impl-Pattern** (Z. 42-57):
```rust
const REPAYMENT_PHASE_PROCESS_CREATE: &str = "repayment-phase.create";
const REPAYMENT_PHASE_PROCESS_UPDATE: &str = "repayment-phase.update";
const REPAYMENT_PHASE_PROCESS_OPEN:   &str = "repayment-phase.open";
const REPAYMENT_PHASE_PROCESS_CLOSE:  &str = "repayment-phase.close";
const REPAYMENT_PHASE_PROCESS_DELETE: &str = "repayment-phase.delete";
const ADMIN_PRIVILEGE: &str = "admin";

gen_service_impl! {
    struct RepaymentPhaseServiceImpl: RepaymentPhaseService = RepaymentPhaseServiceDeps {
        RepaymentPhaseDao: RepaymentPhaseDao<Transaction = Self::Transaction> = repayment_phase_dao,
        AuditLogDao:       AuditLogDao<Transaction = Self::Transaction>       = audit_log_dao,
        PermissionService: PermissionService<Context = Self::Context>          = permission_service,
        UuidService:       UuidService                                          = uuid_service,
        TransactionDao:    TransactionDao<Transaction = Self::Transaction>     = transaction_dao,
    }
}
```

→ Phase 8 Process-Konstanten:
- `REPAYMENT_ENTRY_PROCESS_CREATE = "repayment-entry.create"`
- `REPAYMENT_ENTRY_PROCESS_UPDATE = "repayment-entry.update"`
- `REPAYMENT_ENTRY_PROCESS_DELETE = "repayment-entry.delete"`
- `REPAYMENT_ENTRY_PROCESS_BATCH_TOGGLE = "repayment-entry.batch-toggle"`
- (Auto-Fill nutzt **`REPAYMENT_PHASE_PROCESS_OPEN`** — die N Auto-Fill-Creates sind Folge des Phase-Open-Akts, gleiche `transaction_id`-Gruppierung)

**Deps für Phase-8-Service-Impl** (CONTEXT Integration Points):
```rust
gen_service_impl! {
    struct RepaymentEntryServiceImpl: RepaymentEntryService = RepaymentEntryServiceDeps {
        RepaymentEntryDao:    RepaymentEntryDao<Transaction = Self::Transaction>    = repayment_entry_dao,
        RepaymentPhaseDao:    RepaymentPhaseDao<Transaction = Self::Transaction>    = repayment_phase_dao,  // für Phase-Status-Check D-11.1
        MemberDao:            MemberDao<Transaction = Self::Transaction>            = member_dao,            // für Member-existiert+aktiv-Check D-11.2 + current_shares-Range-Check D-11.3
        AuditLogDao:          AuditLogDao<Transaction = Self::Transaction>          = audit_log_dao,
        PermissionService:    PermissionService<Context = Self::Context>             = permission_service,
        UuidService:          UuidService                                             = uuid_service,
        TransactionDao:       TransactionDao<Transaction = Self::Transaction>        = transaction_dao,
    }
}
```

**Inline-Field-Validator-Pattern** (Z. 59-85, Phase-7-Lektion Plan 07-03 in STATE.md):
```rust
fn validate_phase_fields(fiscal_year: i32, share_value: i64) -> Result<(), ServiceError> {
    let mut errors: Vec<ValidationFailureItem> = Vec::new();
    if !(2000..=2100).contains(&fiscal_year) {
        errors.push(ValidationFailureItem {
            field: Arc::from("fiscal_year"),
            message: Arc::from(format!("must be in 2000..=2100, got {}", fiscal_year)),
        });
    }
    if share_value <= 0 {
        errors.push(ValidationFailureItem {
            field: Arc::from("share_value"),
            message: Arc::from("must be > 0 (Cent)"),
        });
    }
    if !errors.is_empty() {
        return Err(ServiceError::ValidationError(errors));
    }
    Ok(())
}
```

→ Phase 8 inline-Validator (statt `validation.rs`-Erweiterung — siehe Hinweis in Klassifikations-Tabelle oben):
```rust
fn validate_entry_create(
    share_count_to_pay_out: i32,
    member_current_shares: i32,
) -> Result<(), ServiceError> {
    // D-11.3: > 0 AND ≤ Member.current_shares
    let mut errors: Vec<ValidationFailureItem> = Vec::new();
    if share_count_to_pay_out <= 0 {
        errors.push(ValidationFailureItem {
            field: Arc::from("share_count_to_pay_out"),
            message: Arc::from("must be > 0"),
        });
    }
    if share_count_to_pay_out > member_current_shares {
        errors.push(ValidationFailureItem {
            field: Arc::from("share_count_to_pay_out"),
            message: Arc::from(format!(
                "must be ≤ member current_shares ({}), got {}",
                member_current_shares, share_count_to_pay_out
            )),
        });
    }
    if !errors.is_empty() { return Err(ServiceError::ValidationError(errors)); }
    Ok(())
}
```

**Service-Methoden-Pattern: `create_repayment_phase`** (Z. 92-139) — Vorlage für `create_repayment_entry`:
```rust
async fn create_repayment_phase(...) -> Result<RepaymentPhase, ServiceError> {
    let tx = self.transaction_dao.use_transaction(None).await?;

    // (1) Permission + User-ID für Audit
    let user_id = self.permission_service.current_user_id(context.clone()).await?
        .unwrap_or_else(|| "SYSTEM".to_string());
    self.permission_service.check_permission(ADMIN_PRIVILEGE, context).await?;

    // (2) Validation BEFORE entity construction — Test-Verification via .expect_create().times(0)
    validate_phase_fields(submission.fiscal_year, submission.share_value)?;

    // (3) Entity-Build
    let now = time::OffsetDateTime::now_utc();
    let created = time::PrimitiveDateTime::new(now.date(), now.time());
    let entity = RepaymentPhaseEntity {
        id: self.uuid_service.new_v4().await,
        ...
        status: RepaymentPhaseStatus::Preparation,
        created,
        deleted: None,
        version: self.uuid_service.new_v4().await,
    };

    // (4) Audited write
    crate::audited_create!(
        self, self.repayment_phase_dao, &entity,
        REPAYMENT_PHASE_PROCESS_CREATE, &user_id, tx
    );

    self.transaction_dao.commit(tx).await?;
    Ok(RepaymentPhase::from(&entity))
}
```

→ Phase 8 `create_repayment_entry`: zusätzliche Pre-Validations (CONTEXT D-11):
1. **Phase laden** via `self.repayment_phase_dao.find_by_id(submission.phase_id, tx.clone()).await?.ok_or(EntityNotFound)` → wenn `status != Open` → `ServiceError::Conflict("Phase status is '...', expected 'Open' (D-11.1)")`
2. **Member laden** via `self.member_dao.find_by_id(submission.member_id, tx.clone()).await?.ok_or(EntityNotFound)` (D-11.2; `find_by_id` filtert `deleted IS NULL` per Default-Impl `member.rs:133-143`)
3. **Range-Check** `validate_entry_create(submission.share_count_to_pay_out, member.current_shares)?` (D-11.3)
4. Initial-Status `RepaymentEntryStatus::Open` (CONTEXT in-scope §1: "Eingänge stehen mit Lifecycle Open ↔ Contacted")
5. `audited_create!`

**Service-Methoden-Pattern: `update_repayment_phase` Edit-Matrix** (Z. 141-220) — Vorlage für `update_repayment_entry`:
```rust
// (1) Tx + Permission + User-ID (gleich wie create)
// (2) WR-04: duplicate find_by_id für Edit-Matrix-Guard und version-check BEFORE mutation
let mut entity = self.repayment_phase_dao.find_by_id(id, tx.clone()).await?
    .ok_or(ServiceError::EntityNotFound(id))?;

// (3) Edit-Matrix-Check VOR version-check (Phase-7-Lektion Plan 07-03 in STATE.md: 
//     "atomare D-07-Ablehnung liefert semantisch klarere Fehlermeldung als generisches 'Version mismatch'")
match entity.status {
    RepaymentPhaseStatus::Closed => return Err(ServiceError::Conflict(Arc::from("Cannot update: phase is Closed (D-04)"))),
    RepaymentPhaseStatus::Open => {
        if entity.fiscal_year != update.fiscal_year {
            return Err(ServiceError::Conflict(Arc::from("Cannot change fiscal_year: phase is Open (D-04/D-07)")));
        }
    }
    RepaymentPhaseStatus::Preparation => { /* all fields editable */ }
}

// (4) Optimistic locking
if entity.version != update.version {
    return Err(ServiceError::Conflict(Arc::from("Version mismatch")));
}

// (5) Re-validate on update
validate_phase_fields(update.fiscal_year, update.share_value)?;

// (6) Apply diff + audited_update!
entity.fiscal_year = update.fiscal_year;
entity.share_value = update.share_value;

crate::audited_update!(self, self.repayment_phase_dao, id, &entity, REPAYMENT_PHASE_PROCESS_UPDATE, &user_id, tx);
self.transaction_dao.commit(tx).await?;
Ok(RepaymentPhase::from(&entity))
```

→ Phase 8 `update_repayment_entry` (CONTEXT D-12 + D-05 Edit-Matrix):
- Wenn `entity.status == PaidOut` → 409 ("Cannot update: entry is PaidOut; final per PAYO-04 (Phase 9)")
- Wenn `update.share_count_to_pay_out.is_some()` und `entity.status == Open|Contacted`: range-validate gegen aktuelle `Member.current_shares`
- Wenn `update.status.is_some()`:
  - Target = `PaidOut` → 409 ("PaidOut transition must use Phase-9 mark_paid_out endpoint" — D-05 Hinweis)
  - Target ∈ {Open, Contacted} und current ∈ {Open, Contacted} → erlaubt (D-06 bidirektional)
  - Andere Kombinationen → 409
- Optimistic-Locking via `entity.version != update.version` → "Version mismatch"
- Apply Diff für die Optional-Felder, dann `audited_update!`

**Service-Methoden-Pattern: `delete_repayment_phase` (audited_delete)** (Z. 339-381) — Vorlage für `delete_repayment_entry`:
```rust
async fn delete_repayment_phase(...) -> Result<(), ServiceError> {
    let tx = ...;
    let user_id = ...;
    self.permission_service.check_permission(ADMIN_PRIVILEGE, context).await?;

    // D-09: Pre-Guard
    let entity = self.repayment_phase_dao.find_by_id(id, tx.clone()).await?
        .ok_or(ServiceError::EntityNotFound(id))?;
    if entity.status != RepaymentPhaseStatus::Preparation {
        return Err(ServiceError::Conflict(Arc::from(format!(
            "Cannot delete: status is '{}', expected 'Preparation' (D-09)",
            entity.status.as_str()
        ))));
    }

    crate::audited_delete!(self, self.repayment_phase_dao, id, REPAYMENT_PHASE_PROCESS_DELETE, &user_id, tx);
    self.transaction_dao.commit(tx).await?;
    Ok(())
}
```

→ Phase 8 `delete_repayment_entry` (CONTEXT ENTR-05): Guard `entity.status != PaidOut` → 409 ("Cannot delete: entry is PaidOut (ENTR-05)"), sonst `audited_delete!`.

**Service-Methoden-Pattern: `batch_toggle_status` (NEU — kein direkter Phase-7-Analog, kombiniert audited_update + tx.clone-Multi-DAO aus assembly.rs)**:

Vorlage 1 — Single-Tx-Multi-DAO-Pattern (`assembly.rs:181-258`):
```rust
let tx = self.transaction_dao.use_transaction(None).await?;
// ... mehrere DAO-Calls auf tx.clone() ...
crate::audited_update!(self, self.dao_a, id, &entity, PROCESS, &user_id, tx);
// ...
self.dao_b.batch_insert(&items, PROCESS, tx.clone()).await?;
// EIN commit am Ende
self.transaction_dao.commit(tx).await?;
```

Vorlage 2 — Audit-pro-Update (Phase 8 D-08 "N einzelne audited_update! in einer Tx"):
```rust
// Pseudo-code Phase 8:
async fn batch_toggle_status(&self, input, ctx) -> Result<Arc<[RepaymentEntry]>, ServiceError> {
    let tx = self.transaction_dao.use_transaction(None).await?;
    let user_id = ...;
    self.permission_service.check_permission(ADMIN_PRIVILEGE, ctx).await?;

    // D-07: PaidOut as target → 400 BadRequest
    if input.target_status == RepaymentEntryStatus::PaidOut {
        return Err(ServiceError::ValidationError(vec![ValidationFailureItem {
            field: Arc::from("target_status"),
            message: Arc::from("PaidOut not allowed via batch-status; use Phase-9 mark_paid_out"),
        }]));
    }

    // D-08: all-or-nothing — erster Fehler → Tx-Rollback durch Drop, 409 mit Detail
    let mut updated: Vec<RepaymentEntry> = Vec::with_capacity(input.entry_ids.len());
    for (idx, entry_id) in input.entry_ids.iter().enumerate() {
        let mut entity = self.repayment_entry_dao.find_by_id(*entry_id, tx.clone()).await?
            .ok_or_else(|| ServiceError::Conflict(Arc::from(format!(
                "Batch failed at index {}: entry {} not found", idx, entry_id
            ))))?;

        // Source-Status-Check: nur Open ↔ Contacted erlaubt (D-06)
        if !matches!(entity.status, RepaymentEntryStatus::Open | RepaymentEntryStatus::Contacted) {
            return Err(ServiceError::Conflict(Arc::from(format!(
                "Batch failed at index {}: entry {} status is '{}', expected Open or Contacted",
                idx, entry_id, entity.status.as_str()
            ))));
        }

        entity.status = input.target_status.clone();
        crate::audited_update!(
            self, self.repayment_entry_dao, *entry_id, &entity,
            REPAYMENT_ENTRY_PROCESS_BATCH_TOGGLE, &user_id, tx
        );
        updated.push(RepaymentEntry::from(&entity));
    }

    self.transaction_dao.commit(tx).await?;
    Ok(updated.into())
}
```

> **PaidOut-Guard ist Pflicht** im PUT- *und* im Batch-Pfad (D-05, D-07). Beide Pfade müssen denselben Error-Stil liefern, damit Frontend-Tests stabil bleiben.

**Test-Mock-Pattern** (Z. 421-694, **Phase 8 muss diese Test-Infrastruktur dupliziert anlegen**):

```rust
mod tests {
    use super::*;
    use async_trait::async_trait;
    use genossi_dao::audit_log::{AuditLogEntry, AuditQueryFilter};
    use genossi_dao::{DaoError, Transaction};
    use genossi_service::permission::MockContext;
    use mockall::mock;

    #[derive(Clone, Debug)]
    pub struct TestTransaction;
    #[async_trait]
    impl Transaction for TestTransaction { ... }

    mock! {
        pub TestTxDao {}
        #[async_trait]
        impl TransactionDao for TestTxDao { ... }
    }

    mock! {
        pub TestRepaymentPhaseDao {}
        #[async_trait]
        impl RepaymentPhaseDao for TestRepaymentPhaseDao { type Transaction = TestTransaction; ... }
    }

    mock! { pub TestAuditLogDao { ... } }
    mock! { pub TestPermissionService { ... } }

    // ... TestDeps, build_service, phase_in_status, make_*_dao_quiet helpers ...
}
```

→ Phase 8 spiegelt das Pattern, ergänzt `MockTestRepaymentEntryDao` + `MockTestMemberDao` (hand-rolled — Cross-Modul-`automock`-Sharing-Problematik aus Phase-3-Plan-03-Lektion in STATE.md).

**Test-Cases (mindestens)** spiegelnd den Phase-7-Test-Pattern + Phase-8-spezifische Cases:

| Test | Vorlage | Phase-8-Konkretisierung |
|------|---------|--------------------------|
| `test_create_entry_validation_rejects_share_count_zero_or_negative` | `test_create_repayment_phase_validation_rejects_share_value_zero` (Z. 729-756) | D-11.3 |
| `test_create_entry_validation_rejects_share_count_exceeds_member_current_shares` | (Phase-8-eigener) | D-11.3, Member-Lookup-Dependency |
| `test_create_entry_rejects_when_phase_not_open` | (Phase-8-eigener: Conflict mit "Phase status … expected Open") | D-11.1 |
| `test_create_entry_rejects_when_member_not_found` | analog `EntityNotFound`-Test | D-11.2 |
| `test_create_entry_success` | `test_create_repayment_phase_success` (Z. 789-817) | Happy-Path |
| `test_update_entry_paid_out_returns_conflict` | `test_update_repayment_phase_in_closed_returns_conflict` (Z. 821-855) | D-05 |
| `test_update_entry_status_to_paid_out_via_put_returns_conflict` | (Phase-8-eigener) | D-05, D-07 |
| `test_update_entry_status_open_to_contacted_succeeds` | `test_update_repayment_phase_share_value_change_in_open_succeeds` (Z. 897-928) | D-06 |
| `test_update_entry_version_mismatch_returns_conflict` | `test_update_repayment_phase_version_mismatch_returns_conflict` (Z. 931-966) | optimistic-locking |
| `test_delete_entry_in_paid_out_returns_conflict` | `test_delete_repayment_phase_in_open_returns_conflict` (Z. 1050-1077) | ENTR-05 |
| `test_delete_entry_in_open_succeeds` | `test_delete_repayment_phase_in_preparation_succeeds` (Z. 1079-1106) | ENTR-05 |
| `test_batch_toggle_paid_out_target_returns_validation_error` | (Phase-8-eigener) | D-07 |
| `test_batch_toggle_all_or_nothing_on_failure` | (Phase-8-eigener: ein Fehler im 3. Entry → keiner geändert) | D-08 |
| `test_batch_toggle_success` | (Phase-8-eigener: alle N audited_update! laufen, gemeinsame Tx) | D-08 |

---

### 6. EXTENSION: `genossi_service_impl/src/repayment_phase.rs` (MODIFY)

**Analog für `open_repayment_phase`-Erweiterung:** `genossi_service_impl/src/assembly.rs:181-259`
**Analog für `close_repayment_phase`-Erweiterung:** `genossi_service_impl/src/assembly.rs:261-291` (Status-Guard) plus eigene Pending-Aggregation

> **Deps-Erweiterung Pflicht:** `RepaymentPhaseServiceImpl` braucht zusätzlich `RepaymentEntryDao` + `MemberDao` (CONTEXT Integration Points). Das ändert `RepaymentPhaseServiceDependencies` in `genossi_bin/src/lib.rs:181-200` UND die `gen_service_impl!`-Deklaration in `genossi_service_impl/src/repayment_phase.rs:49-57`. Wiring-Reihenfolge in `genossi_bin/src/lib.rs::new()`: zuerst `repayment_entry_dao` bauen, dann es als `Arc::clone` an `RepaymentPhaseServiceImpl` UND `RepaymentEntryServiceImpl` weitergeben (Pattern aus Phase 3 Plan 05 `helper_token_dao`-Sharing, STATE.md).

**Auto-Fill in `open_phase` — Code-Vorlage** (`assembly.rs:181-258`):
```rust
async fn open_assembly(&self, id: Uuid, context: Authentication<Self::Context>) -> Result<Assembly, ServiceError> {
    // Pitfall 2: ONE transaction, ONE commit at the end. tx.clone() for sub-calls.
    let tx = self.transaction_dao.use_transaction(None).await?;

    let user_id = self.permission_service.current_user_id(context.clone()).await?
        .unwrap_or_else(|| "SYSTEM".to_string());
    self.permission_service.check_permission(ADMIN_PRIVILEGE, context).await?;

    // WR-04: duplicate find_by_id is intentional and required for the state-guard.
    let mut entity = self.assembly_dao.find_by_id(id, tx.clone()).await?
        .ok_or(ServiceError::EntityNotFound(id))?;

    // Pitfall 3: state-transition guard.
    if entity.status != AssemblyStatus::Preparation {
        return Err(ServiceError::Conflict(Arc::from(format!(
            "Cannot open assembly: status is '{}', expected 'Preparation'",
            entity.status.as_str()
        ))));
    }

    let now_offset = time::OffsetDateTime::now_utc();
    let now_pdt = time::PrimitiveDateTime::new(now_offset.date(), now_offset.time());
    let opened_date = now_offset.date();
    entity.status = AssemblyStatus::Open;
    entity.opened_at = Some(now_pdt);

    crate::audited_update!(self, self.assembly_dao, id, &entity, ASSEMBLY_PROCESS_OPEN, &user_id, tx);

    // D-02: count_active filter
    // member_dao.all() already filters deleted IS NULL.
    let all_members = self.member_dao.all(tx.clone()).await?;
    let snapshot_entities: Vec<AssemblyMemberSnapshotEntity> = all_members
        .iter()
        .filter(|m| m.status.is_normal())
        .filter(|m| m.join_date <= opened_date)
        .filter(|m| m.exit_date.map_or(true, |d| d > opened_date))
        .map(|m| AssemblyMemberSnapshotEntity {
            assembly_id: id,
            member_id: m.id,
            captured_at: now_pdt,
        })
        .collect();

    // Pitfall 1: snapshot inserts deliberately bypass audit macros — the snapshot
    // is data, not a lifecycle event. The act of opening is audited above.
    self.assembly_member_snapshot_dao
        .create_batch(&snapshot_entities, ASSEMBLY_PROCESS_OPEN, tx.clone())
        .await?;

    self.transaction_dao.commit(tx).await?;
    Ok(Assembly::from(&entity))
}
```

→ Phase 8 `open_repayment_phase`-Erweiterung (CONTEXT D-01/D-02/D-03/D-04):
1. **Bestehender Phase-7-Pre-Block bleibt** (Tx, Permission, find_by_id, Status-Guard `!= Preparation` → 409, Status-Mutation auf `Open`, `audited_update!` mit `REPAYMENT_PHASE_PROCESS_OPEN`)
2. **NEU NACH dem `audited_update!`** (innerhalb derselben Tx):

```rust
// PHAS-02 / ENTR-01 (Phase 8): Auto-Befüllung der RepaymentEntries.
// Innerhalb derselben Tx wie der Status-Übergang Preparation→Open.
// Pattern: assembly.rs:181-258 (Single-Tx-Multi-DAO via tx.clone()).
//
// UNTERSCHIED zu Assembly-Snapshot: hier KEIN batch_create_without_audit,
// sondern N einzelne audited_create! (D-03). Phase-9-Cascade hängt an
// entity_id+version pro Entry — RepaymentEntries sind Lifecycle-Träger,
// nicht "nur Daten".

let fiscal_year = entity.fiscal_year;
let fy_start = time::Date::from_calendar_date(fiscal_year, time::Month::January, 1)
    .map_err(|e| ServiceError::InternalError(Arc::from(format!("invalid fiscal_year date: {}", e))))?;
let fy_end = time::Date::from_calendar_date(fiscal_year, time::Month::December, 31)
    .map_err(|e| ServiceError::InternalError(Arc::from(format!("invalid fiscal_year date: {}", e))))?;

// D-02: strikter Member-Filter — kein is_normal()-Filter (Ausgeschiedene haben oft Status != Normal,
// das ist genau die Zielgruppe). member_dao.all() filtert bereits deleted IS NULL.
let all_members = self.member_dao.all(tx.clone()).await?;
let mut targets: Vec<&MemberEntity> = all_members
    .iter()
    .filter(|m| m.exit_date.map_or(false, |d| d >= fy_start && d <= fy_end))
    .filter(|m| m.current_shares > 0)
    .collect();

// Discretion: deterministische Audit-Reihenfolge (CONTEXT Claude's Discretion)
targets.sort_by_key(|m| m.member_number);

for member in targets {
    let now_offset = time::OffsetDateTime::now_utc();
    let now_pdt = time::PrimitiveDateTime::new(now_offset.date(), now_offset.time());
    let new_entry = RepaymentEntryEntity {
        id: self.uuid_service.new_v4().await,
        member_id: member.id,
        phase_id: id,
        share_count_to_pay_out: member.current_shares,
        status: RepaymentEntryStatus::Open,
        created: now_pdt,
        deleted: None,
        version: self.uuid_service.new_v4().await,
    };
    crate::audited_create!(
        self, self.repayment_entry_dao, &new_entry,
        REPAYMENT_PHASE_PROCESS_OPEN,   // gleiche Prozesskonstante wie der Phase-Open-Akt
        &user_id, tx
    );
}

self.transaction_dao.commit(tx).await?;
Ok(RepaymentPhase::from(&entity))
```

**Close-Validation in `close_phase` — Code-Vorlage** (`assembly.rs:261-341` für Struktur, NEUER Logik für Pending-Aggregation):

→ Phase 8 `close_repayment_phase`-Erweiterung (CONTEXT D-13/D-14/D-15):
- **NEU VOR dem Status-Übergang auf `Closed`** (innerhalb der bestehenden Tx, nach dem find_by_id + Status-Guard `!= Open`):

```rust
// PHAS-03 (Phase 8): Pending-Entry-Validation.
// D-13: "pending entry" = status != PaidOut AND deleted IS NULL
let entries = self.repayment_entry_dao.find_by_phase_id(id, tx.clone()).await?;
let pending: Vec<&RepaymentEntryEntity> = entries.iter()
    .filter(|e| e.deleted.is_none())
    .filter(|e| e.status != RepaymentEntryStatus::PaidOut)
    .collect();

if !pending.is_empty() {
    // D-15: 409 Conflict mit pending_count + Mitgliedsnummern-Liste (max 20)
    // Mitgliedsnummern (NICHT UUIDs) — Vorstand denkt in Mitgliedsnummern
    let all_members = self.member_dao.all(tx.clone()).await?;
    let member_number_by_id: std::collections::HashMap<Uuid, i64> = all_members.iter()
        .map(|m| (m.id, m.member_number))
        .collect();

    let mut pending_numbers: Vec<i64> = pending.iter()
        .filter_map(|e| member_number_by_id.get(&e.member_id).copied())
        .collect();
    pending_numbers.sort();   // deterministisch
    let total = pending_numbers.len();

    // ServiceError-Variante: wir nutzen eine NEUE Variante oder verpacken die Details
    // im Conflict-Message-Arc — Planner entscheidet. Empfehlung: dedizierte Variante
    // ServiceError::ConflictWithDetails { code, payload: serde_json::Value } ODER
    // strukturiertes Conflict-Message via serde_json::to_string der Detail-Struktur.
    // REST-Layer mapped diese auf 409 mit serialisiertem CloseConflictResponse-TO.
    return Err(/* ServiceError::Conflict mit strukturierten pending-Daten */);
}

// Erst danach: bestehender Phase-7-Block (Status auf Closed, closed_at, audited_update!, commit)
```

> **Architekt-Anmerkung für Planner:** Die `ServiceError`-Variante muss strukturierte Daten transportieren können (pending_count + member_numbers). Der saubere Weg ist eine neue Variante `ServiceError::CloseConflict { pending_count: usize, pending_member_numbers: Vec<i64> }` oder ein Tuple-Conflict-Pattern. Alternativ: JSON-encoded `Arc<str>` im bestehenden `Conflict(Arc<str>)`, REST-Layer parst es zurück. **Phase-7-Konvention erlaubt KEINE neuen ServiceError-Varianten ohne Architekt-Approval** (siehe Phase-7-Plan-04-Lektion: 5-Deps-DI ist "simpler-than-Assembly"-Pattern, große Refactors sind tech-debt-würdig). Planner kann den pragmatischen JSON-in-Arc-Weg gehen oder einen Architekt-Hinweis im Plan dokumentieren.

---

### 7. REST handler: `genossi_rest/src/repayment_entry.rs`

**Analog:** `genossi_rest/src/repayment_phase.rs` (1:1-Vorlage); Action-/Batch-Endpoint-Pattern aus `genossi_rest/src/assembly.rs:142-279`

**RestState-Trait-Pattern** (`repayment_phase.rs:42-49`):
```rust
pub trait RepaymentPhaseRestState: Clone + Send + Sync + 'static {
    type RepaymentPhaseService: RepaymentPhaseService<Context = crate::ContextType>
        + Send + Sync + 'static;

    fn repayment_phase_service(&self) -> Arc<Self::RepaymentPhaseService>;
}
```

→ Phase 8: `pub trait RepaymentEntryRestState { type RepaymentEntryService: RepaymentEntryService<...>; fn repayment_entry_service(&self) -> Arc<Self::RepaymentEntryService>; }`

**Handler-Pattern POST mit utoipa-Annotation** (`repayment_phase.rs:112-151`):
```rust
#[instrument(skip(rest_state))]
#[utoipa::path(
    post,
    tag = "RepaymentEntries",
    path = "",
    request_body = CreateRepaymentEntryRequest,
    responses(
        (status = 201, description = "Created", body = RepaymentEntryTO),
        (status = 400, description = "Validation Error (share_count_to_pay_out)"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Member or Phase not found"),
        (status = 409, description = "Conflict (Phase not Open)"),
    ),
)]
pub async fn create_repayment_entry<RestState: RestStateDef + RepaymentEntryRestState>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Json(body): Json<CreateRepaymentEntryRequest>,
) -> Response {
    error_handler(
        (async {
            let auth = crate::extract_auth_context(Some(context))?;
            let submission = RepaymentEntrySubmission {
                phase_id: body.phase_id,
                member_id: body.member_id,
                share_count_to_pay_out: body.share_count_to_pay_out,
            };
            let entry = rest_state.repayment_entry_service()
                .create_repayment_entry(&submission, auth).await?;
            let to = RepaymentEntryTO::from(&entry);
            Ok(Response::builder().status(201)
                .header("Content-Type", "application/json")
                .body(Body::new(serde_json::to_string(&to)?))
                .unwrap())
        }).await,
    )
}
```

**Handler-Pattern GET-with-Query (Listing-Filter `?phase_id=`)** (CONTEXT D-09/D-10):

Vorlage 1 — `repayment_phase.rs:80-110` (GET ohne Query); Vorlage 2 für Query-Param: `axum::extract::Query`. Pattern:
```rust
use axum::extract::Query;
use serde::Deserialize;

#[derive(Deserialize, utoipa::IntoParams)]
pub struct ListEntriesQuery {
    pub phase_id: Uuid,
}

#[instrument(skip(rest_state))]
#[utoipa::path(
    get,
    tag = "RepaymentEntries",
    path = "",
    params(ListEntriesQuery),
    responses(
        (status = 200, body = [RepaymentEntryTO]),
        (status = 401),
    ),
)]
pub async fn list_repayment_entries<RestState: RestStateDef + RepaymentEntryRestState>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Query(q): Query<ListEntriesQuery>,
) -> Response {
    error_handler((async {
        let auth = crate::extract_auth_context(Some(context))?;
        let entries = rest_state.repayment_entry_service()
            .list_repayment_entries_by_phase(q.phase_id, auth).await?;
        let to_list: Vec<RepaymentEntryTO> = entries.iter().map(RepaymentEntryTO::from).collect();
        Ok(Response::builder().status(200)
            .header("Content-Type", "application/json")
            .body(Body::new(serde_json::to_string(&to_list)?))
            .unwrap())
    }).await)
}
```

**Handler-Pattern Action-Endpoint (Batch-Toggle als POST)** (`repayment_phase.rs:234-268` Open/Close-Pattern + Body von `create`):
```rust
#[instrument(skip(rest_state))]
#[utoipa::path(
    post,
    tag = "RepaymentEntries",
    path = "/batch-status",
    request_body = BatchStatusRequest,
    responses(
        (status = 200, body = [RepaymentEntryTO]),
        (status = 400, description = "Validation Error (PaidOut as target_status)"),
        (status = 401),
        (status = 409, description = "Conflict — first failing entry rolled back transaction"),
    ),
)]
pub async fn batch_toggle_status<RestState: RestStateDef + RepaymentEntryRestState>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Json(body): Json<BatchStatusRequest>,
) -> Response {
    error_handler((async {
        let auth = crate::extract_auth_context(Some(context))?;
        let input = RepaymentEntryBatchStatusInput {
            entry_ids: body.entry_ids.into(),
            target_status: (&body.target_status).into(),
        };
        let updated = rest_state.repayment_entry_service()
            .batch_toggle_status(&input, auth).await?;
        let to_list: Vec<RepaymentEntryTO> = updated.iter().map(RepaymentEntryTO::from).collect();
        Ok(Response::builder().status(200)
            .header("Content-Type", "application/json")
            .body(Body::new(serde_json::to_string(&to_list)?))
            .unwrap())
    }).await)
}
```

**Router-Pattern** (`repayment_phase.rs:337-351`):
```rust
pub fn generate_route<RestState: RestStateDef + RepaymentEntryRestState>() -> Router<RestState> {
    Router::new()
        .route("/", get(list_repayment_entries::<RestState>).post(create_repayment_entry::<RestState>))
        .route("/{id}", get(get_repayment_entry::<RestState>)
                            .put(update_repayment_entry::<RestState>)
                            .delete(delete_repayment_entry::<RestState>))
        .route("/batch-status", post(batch_toggle_status::<RestState>))
}
```

**Pflicht: `batch-status` VOR `/{id}` routen** (Axum-Router-Reihenfolge) — sonst frisst `/{id}` das Wort "batch-status" als Uuid-Parse-Versuch und liefert 400.

**OpenAPI-Doc** (`repayment_phase.rs:353-371`):
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
    ),
    components(schemas(
        RepaymentEntryTO,
        RepaymentEntryStatusTO,
        CreateRepaymentEntryRequest,
        UpdateRepaymentEntryRequest,
        BatchStatusRequest,
        CloseConflictResponse,    // schon hier registrieren, damit Swagger ihn als referenced schema kennt — REST-Layer sendet ihn aus dem close_repayment_phase-Handler (kein eigener handler hier)
    ))
)]
pub struct ApiDoc;
```

**Error-Mapping:** Phase 8 nutzt das **globale** `From<ServiceError> for RestError`-Mapping (`genossi_rest/src/lib.rs:97-113`) — gleich wie Phase 7. **KEIN lokales `map_*_error`** wie in Phase 3 Attendance. Begründung: keine 403-Differenzierung in Phase 8 (alles ist admin-only via OIDC).

**REST-Tests-Pattern** (`repayment_phase.rs:373-414`):
- `test_validate_create_repayment_entry_request_ok`
- `test_validate_update_repayment_entry_request_ok`
- `test_apidoc_compiles`

---

### 8. Transfer Objects: `genossi_rest_types/src/lib.rs` (MODIFY, Append)

**Analog:** `genossi_rest_types/src/lib.rs:1144-1259` (RepaymentPhase-TO-Block, 1:1-Vorlage)

**Status-TO-Pattern** (Z. 1157-1184):
```rust
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum RepaymentPhaseStatusTO {
    Preparation,
    Open,
    Closed,
}

impl From<&genossi_dao::repayment_phase::RepaymentPhaseStatus> for RepaymentPhaseStatusTO { ... match-Pattern ... }
impl From<&RepaymentPhaseStatusTO> for genossi_dao::repayment_phase::RepaymentPhaseStatus { ... match-Pattern ... }
```

→ Phase 8: `RepaymentEntryStatusTO { Open, Contacted, PaidOut }` + bidirektionale `From`-Impls.

**Entity-TO-Pattern mit ISO8601-Serde + skip_serializing_if** (Z. 1186-1237):
```rust
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct RepaymentPhaseTO {
    #[schema(example = "123e4567-e89b-12d3-a456-426614174000")]
    pub id: Uuid,
    #[schema(example = 2026)]
    pub fiscal_year: i32,
    #[schema(example = 12000)]
    pub share_value: i64,
    pub status: RepaymentPhaseStatusTO,
    #[serde(serialize_with = "iso8601_datetime::serialize",
            deserialize_with = "iso8601_datetime::deserialize", default)]
    pub opened_at: Option<time::PrimitiveDateTime>,
    // ... closed_at, created, deleted analog ...
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub version: Option<Uuid>,
}

impl From<&genossi_service::repayment_phase::RepaymentPhase> for RepaymentPhaseTO {
    fn from(p: &genossi_service::repayment_phase::RepaymentPhase) -> Self {
        Self {
            id: p.id, fiscal_year: p.fiscal_year, share_value: p.share_value,
            status: RepaymentPhaseStatusTO::from(&p.status),
            opened_at: p.opened_at, closed_at: p.closed_at,
            created: Some(p.created), deleted: p.deleted,
            version: Some(p.version),
        }
    }
}
```

→ Phase 8:
```rust
pub struct RepaymentEntryTO {
    pub id: Uuid,
    pub member_id: Uuid,
    pub phase_id: Uuid,
    pub share_count_to_pay_out: i32,
    pub status: RepaymentEntryStatusTO,
    #[serde(serialize_with = "iso8601_datetime::serialize", deserialize_with = "iso8601_datetime::deserialize", default)]
    pub created: Option<time::PrimitiveDateTime>,
    #[serde(serialize_with = "iso8601_datetime::serialize", deserialize_with = "iso8601_datetime::deserialize", default)]
    pub deleted: Option<time::PrimitiveDateTime>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub version: Option<Uuid>,
}
```

**Create-/Update-Request-Pattern** (Z. 1239-1259):
```rust
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateRepaymentPhaseRequest {
    pub fiscal_year: i32,
    pub share_value: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct UpdateRepaymentPhaseRequest {
    pub fiscal_year: i32,
    pub share_value: i64,
    pub version: Uuid,   // Pflicht
}
```

→ Phase 8:
```rust
pub struct CreateRepaymentEntryRequest {
    pub phase_id: Uuid,
    pub member_id: Uuid,
    pub share_count_to_pay_out: i32,
}

pub struct UpdateRepaymentEntryRequest {
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub share_count_to_pay_out: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub status: Option<RepaymentEntryStatusTO>,    // Open|Contacted; PaidOut → 409 vom Service
    pub version: Uuid,
}

pub struct BatchStatusRequest {
    pub entry_ids: Vec<Uuid>,
    pub target_status: RepaymentEntryStatusTO,   // Open|Contacted; PaidOut → 400 vom Service
}

pub struct CloseConflictResponse {
    pub error: String,
    pub pending_count: usize,
    pub pending_member_numbers: Vec<String>,   // ["M-001", "M-042", "...+N weitere"]
}
```

> **CloseConflictResponse-Mitgliedsnummer-Format-Hinweis** (CONTEXT D-15): Format `"M-{padded_member_number}"` ist Pseudocode — Planner darf den exakten Stringformat festlegen, muss aber konsistent mit Frontend-Erwartung sein (Phase 12). Vorschlag: einfach `member_number.to_string()` ohne `M-`-Prefix, plus optional `+N weitere`-Suffix als 21. Element wenn `total > 20`. Phase 7 PHAS-04 hat keinen vergleichbaren Detail-Body.

**TO-Tests-Pattern** (Z. 1689-1779) — Phase 8 muss diese Suite spiegeln:
- `test_repayment_entry_status_to_roundtrip` (alle 3 Varianten via TO ↔ DAO)
- `test_repayment_entry_to_from_domain` (alle Felder mirror-1:1)
- `test_create_repayment_entry_request_serde` (JSON-Roundtrip)
- `test_update_repayment_entry_request_optional_fields` (alle Felder optional außer `version`)
- `test_batch_status_request_serde`
- `test_close_conflict_response_serializes_with_pending_numbers`

---

### 9. DI-Wiring: `genossi_bin/src/lib.rs` (MODIFY)

**Analog:** `genossi_bin/src/lib.rs:176-200, 701-713, 1311-1316` (RepaymentPhase-Wiring, 1:1-Vorlage)

**Type-Alias + Dependencies-Struct + Deps-Trait-Impl** (Z. 176-200):
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

type RepaymentPhaseService = genossi_service_impl::repayment_phase::RepaymentPhaseServiceImpl<RepaymentPhaseServiceDependencies>;
```

→ Phase 8: **ZWEI** Änderungen:
1. **Neuer** Block für `RepaymentEntryServiceDependencies` (analog mit 7 Deps wie oben in §5 spezifiziert)
2. **Bestehender** `RepaymentPhaseServiceDependencies`-Block erweitert um die zwei zusätzlichen Deps:
   ```rust
   impl genossi_service_impl::repayment_phase::RepaymentPhaseServiceDeps for RepaymentPhaseServiceDependencies {
       type Context = Context;
       type Transaction = Transaction;
       type RepaymentPhaseDao = RepaymentPhaseDao;
       type RepaymentEntryDao = RepaymentEntryDao;   // NEU
       type MemberDao = MemberDao;                    // NEU
       type AuditLogDao = AuditLogDao;
       type PermissionService = PermissionService;
       type UuidService = UuidService;
       type TransactionDao = TransactionDao;
   }
   ```

**Wiring im `RestStateImpl::new()`** (Z. 701-713):
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

→ Phase 8 (Reihenfolge wichtig — CONTEXT Integration Points):
```rust
// 1) RepaymentEntryDao ZUERST (vor RepaymentPhaseService)
let repayment_entry_dao = Arc::new(RepaymentEntryDao::new(pool.clone()));

// 2) RepaymentPhaseService NEU mit RepaymentEntryDao + MemberDao
let repayment_phase_dao = Arc::new(RepaymentPhaseDao::new(pool.clone()));
let repayment_phase_service = Arc::new(
    genossi_service_impl::repayment_phase::RepaymentPhaseServiceImpl {
        repayment_phase_dao: repayment_phase_dao.clone(),
        repayment_entry_dao: repayment_entry_dao.clone(),   // NEU — Phase-8-Auto-Fill + Close-Validation
        member_dao: member_dao.clone(),                      // NEU — Member-Filter im Auto-Fill + Member-Number-Lookup im Close-Conflict
        audit_log_dao: audit_log_dao.clone(),
        permission_service: permission_service.clone(),
        uuid_service: uuid_service.clone(),
        transaction_dao: transaction_dao.clone(),
    },
);

// 3) RepaymentEntryService mit allen Deps
let repayment_entry_service = Arc::new(
    genossi_service_impl::repayment_entry::RepaymentEntryServiceImpl {
        repayment_entry_dao,                                 // moved (kein weiterer Consumer)
        repayment_phase_dao: repayment_phase_dao.clone(),    // shared mit RepaymentPhaseService
        member_dao: member_dao.clone(),
        audit_log_dao: audit_log_dao.clone(),
        permission_service: permission_service.clone(),
        uuid_service: uuid_service.clone(),
        transaction_dao: transaction_dao.clone(),
    },
);
```

> **Sharing-Pattern:** `repayment_phase_dao` UND `repayment_entry_dao` werden zwischen den beiden Services Arc-shared — identisch zu Phase 3 Plan 05 `helper_token_dao`-Sharing (STATE.md), Phase 3 Plan 06 `assembly_member_snapshot_dao`-Sharing.

**Struct-Field-Pattern** (Z. 473-474):
```rust
// Phase 7 Plan 04: RepaymentPhase backend foundation.
repayment_phase_service: Arc<RepaymentPhaseService>,
```

→ Phase 8 ergänzt:
```rust
// Phase 8: RepaymentEntry + Auto-Befüllung
repayment_entry_service: Arc<RepaymentEntryService>,
```

**`Self { ... }`-Init-Pattern** (Z. 832): `repayment_phase_service,` → Phase 8 ergänzt `repayment_entry_service,`

**RestState-Impl-Bridge-Pattern** (Z. 1309-1316):
```rust
impl genossi_rest::repayment_phase::RepaymentPhaseRestState for RestStateImpl {
    type RepaymentPhaseService = RepaymentPhaseService;
    fn repayment_phase_service(&self) -> Arc<Self::RepaymentPhaseService> {
        self.repayment_phase_service.clone()
    }
}
```

→ Phase 8 ergänzt:
```rust
impl genossi_rest::repayment_entry::RepaymentEntryRestState for RestStateImpl {
    type RepaymentEntryService = RepaymentEntryService;
    fn repayment_entry_service(&self) -> Arc<Self::RepaymentEntryService> {
        self.repayment_entry_service.clone()
    }
}
```

---

### 10. Router/OpenAPI: `genossi_rest/src/lib.rs` (MODIFY)

**Analog:** `genossi_rest/src/lib.rs:20, 270, 438, 610-612` (RepaymentPhase-Registration, 1:1-Vorlage)

**4 Patches:**
1. **Z. 20** `pub mod repayment_phase;` → ergänze `pub mod repayment_entry;`
2. **Z. 270** OpenAPI-Nest:
   ```rust
   (path = "/api/repayment-phase", api = repayment_phase::ApiDoc),
   ```
   → ergänze:
   ```rust
   (path = "/api/repayment-entry", api = repayment_entry::ApiDoc),
   ```
3. **Z. 438** Trait-Bound auf `create_app`:
   ```rust
   + repayment_phase::RepaymentPhaseRestState
   ```
   → ergänze nach dieser Zeile:
   ```rust
   + repayment_entry::RepaymentEntryRestState
   ```
4. **Z. 756** (Trait-Bound auf `start_server`): gleicher Patch wie 3.
5. **Z. 610-612** Router-Mount:
   ```rust
   .nest(
       "/api/repayment-phase",
       repayment_phase::generate_route::<RestState>(),
   )
   ```
   → ergänze direkt darunter:
   ```rust
   .nest(
       "/api/repayment-entry",
       repayment_entry::generate_route::<RestState>(),
   )
   ```

**Trait-Bound auch in `genossi_rest/src/test_server.rs`** (Phase-7-Plan-04-Pattern — STATE.md): die Trait-Bound `RepaymentEntryRestState` muss auch zur `start_test_server`-Signatur ergänzt werden, damit die E2E-Tests den Service-Pfad erreichen. Vorlage: dieselbe Datei, Phase-7-Eintrag.

---

### 11. Module declarations (4 lib.rs Dateien)

**Analog für jede:** Phase-7-Eintrag `repayment_phase` an alphabetisch korrekter Stelle.

| Datei | Bestehender Phase-7-Eintrag | Phase-8-Ergänzung |
|-------|---------------------------|--------------------|
| `genossi_dao/src/lib.rs` | Z. 14 `pub mod repayment_phase;` | direkt darüber: `pub mod repayment_entry;` |
| `genossi_dao_impl_sqlite/src/lib.rs` | Z. 13 `pub mod repayment_phase;` | direkt darüber: `pub mod repayment_entry;` |
| `genossi_service/src/lib.rs` | Z. 15 `pub mod repayment_phase;` | direkt darüber: `pub mod repayment_entry;` |
| `genossi_service_impl/src/lib.rs` | Z. 16 `pub mod repayment_phase;` | direkt darüber: `pub mod repayment_entry;` |

**Konvention:** Alphabetische Sortierung — `repayment_entry` kommt VOR `repayment_phase`.

---

### 12. E2E-Tests: `genossi_bin/tests/e2e_tests.rs` (MODIFY)

**Analog:** `genossi_bin/tests/e2e_tests.rs:10553-10999` (Phase-7-Lifecycle-Tests + `create_preparation_repayment_phase`-Helper)

**Test-Setup-Pattern** (Z. 25-39 — `setup()` mit In-Memory-SQLite + Migration):
```rust
async fn setup() -> genossi_rest::test_server::test_support::TestServer {
    let pool = Arc::new(SqlitePool::connect("sqlite::memory:").await.expect("..."));
    sqlx::migrate!("../migrations/sqlite").run(&*pool).await.expect("Failed to run migrations");
    let rest_state = RestStateImpl::new(pool);
    start_test_server(rest_state).await
}
```

→ Phase 8 nutzt `setup()` direkt — Phase-8-Migration läuft automatisch über `sqlx::migrate!`. **Kein** neuer Setup-Helper nötig.

**Sample-Helper-Pattern** (Z. 41-69 `sample_member()` + Z. 677-689 `create_test_member`):
```rust
fn sample_member() -> MemberTO {
    MemberTO {
        id: None,
        member_number: 1,
        first_name: "Max".to_string(),
        // ...
        current_shares: 3,
        exit_date: None,
        // ...
    }
}

async fn create_test_member(client, server) -> MemberTO {
    let response = client.post(server.url("/api/members")).json(&sample_member()).send().await.unwrap();
    response.json().await.unwrap()
}
```

→ Phase 8 ergänzt:
```rust
/// Phase 8 helper — Member mit explizitem exit_date für Auto-Fill-Test.
async fn create_member_with_exit_date(
    client: &reqwest::Client,
    server: &genossi_rest::test_server::test_support::TestServer,
    member_number: i64,
    fiscal_year: i32,
    current_shares: i32,
) -> MemberTO {
    let mut m = sample_member();
    m.member_number = member_number;
    m.current_shares = current_shares;
    m.shares_at_joining = current_shares;
    m.exit_date = Some(time::Date::from_calendar_date(fiscal_year, time::Month::June, 15).unwrap());
    let response = client.post(server.url("/api/members")).json(&m).send().await.unwrap();
    response.json().await.unwrap()
}

/// Phase 8 helper — Phase erzeugen + öffnen in einem.
async fn create_open_repayment_phase(...) -> RepaymentPhaseTO { ... }
```

**Phase-7-Vorlage `create_preparation_repayment_phase`** (Z. 10559-10581) — 1:1 als Vorlage:
```rust
async fn create_preparation_repayment_phase(
    client: &reqwest::Client,
    server: &genossi_rest::test_server::test_support::TestServer,
    fiscal_year: i32,
    share_value: i64,
) -> RepaymentPhaseTO {
    let body = serde_json::json!({
        "fiscal_year": fiscal_year,
        "share_value": share_value,
    });
    let response = client.post(server.url("/api/repayment-phase")).json(&body).send().await.unwrap();
    assert_eq!(response.status(), StatusCode::CREATED, "create repayment-phase must return 201");
    response.json().await.unwrap()
}
```

**Lifecycle-Audit-Test-Pattern** (Z. 10583-10650+, `test_repayment_phase_lifecycle_audit_chain_intact`) — Vorlage für Phase-8-E2E:

→ Phase 8 E2E-Tests (CONTEXT in-scope §E2E + Plan-Discretion zu Test-Naming):

| Test | Fokus | Phase-7-Pattern-Vorlage |
|------|-------|--------------------------|
| `test_open_phase_triggers_auto_fill` | 3 Members mit exit_date im FY (1 ohne current_shares > 0); nach `open` exakt 2 Entries + `/api/audit` zeigt N+1 Einträge | `test_repayment_phase_lifecycle_audit_chain_intact` Audit-Verification-Block |
| `test_open_phase_auto_fill_zero_members` | 0 Members mit exit_date; nach `open` 0 Entries, Phase ist trotzdem `Open` | analog Z. 10800+ Edge-Case-Pattern |
| `test_manual_add_entry_happy_path` | POST mit gültigem Body → 201 mit RepaymentEntryTO | `test_create_and_get_member` Z. 84-114 |
| `test_manual_add_entry_phase_not_open_returns_409` | Phase in `Preparation`, POST → 409 | analog Status-Guard-Tests Z. 10860+ |
| `test_manual_add_entry_share_count_exceeds_returns_400` | POST mit `share_count_to_pay_out > Member.current_shares` → 400 + Body-Substring "share_count_to_pay_out" | analog `test_repayment_phase_create_validation_returns_400` |
| `test_update_entry_status_open_to_contacted_succeeds` | PUT mit `status: "Contacted"` → 200 | `test_update_repayment_phase_share_value_change_in_open_succeeds` |
| `test_update_entry_status_paid_out_returns_409` | PUT mit `status: "PaidOut"` → 409 + Substring "Phase 9" oder "mark_paid_out" | (Phase-8-eigen) |
| `test_delete_entry_in_open_succeeds` | DELETE auf Open-Entry → 204 + nachfolgender GET 404 | analog `test_delete_repayment_phase` |
| `test_batch_toggle_happy_path` | POST mit 3 entry_ids, target=Contacted → 200 + alle 3 jetzt Contacted | (Phase-8-eigen) |
| `test_batch_toggle_all_or_nothing_failure` | 3 entry_ids, 2. ist bereits PaidOut → 409 + alle 3 unverändert (Tx-Rollback verifizieren via GET) | (Phase-8-eigen) |
| `test_close_phase_with_pending_entries_returns_409_with_member_numbers` | Phase mit 2 Open + 1 PaidOut + 1 deleted Entries → close → 409 + Body enthält `pending_count: 2` + `pending_member_numbers: ["1", "5"]` | (Phase-8-eigen, neues 409-Body-Pattern) |
| `test_close_phase_with_all_paid_out_or_deleted_succeeds` | Phase mit 1 PaidOut + 1 deleted Entry → close → 200 | (Phase-8-eigen) |
| `test_close_phase_with_zero_entries_succeeds` | Phase ohne Entries → close → 200 (D-14) | (Phase-8-eigen) |
| `test_audit_chain_intact_after_phase_8_lifecycle` | Vollzyklus create-phase → open(triggers auto-fill) → batch-toggle → mark-one-paid-out → delete-one → close-phase → `/api/audit/verify` returnt `valid=true` | `test_repayment_phase_lifecycle_audit_chain_intact` Z. 10583-10800 |

> **PaidOut-Setup im Test:** Phase 8 hat keinen `mark_paid_out`-Endpoint (das ist Phase 9). Für Tests, die `PaidOut`-Entries voraussetzen (z.B. `test_close_phase_with_all_paid_out_or_deleted_succeeds`), muss der Test-Helper entweder direkt auf den DAO zugreifen (was im E2E-Test nicht möglich ist) oder die Test-Fixture muss anders konstruiert sein. **Planner-Discretion:** Phase 8 kann den `test_close_with_all_paid_out`-Test als TODO/skipped markieren bis Phase 9 oder einen Test-only-DB-Schreib-Helper anlegen (analog `create_preparation_repayment_phase`).

---

## Shared Patterns

### 1. Authentication & Authorization (alle neuen Endpoints)

**Source:** `genossi_service_impl/src/repayment_phase.rs:47, 104-106`
**Apply to:** Alle Phase-8-Service-Methoden (create, update, delete, get, list, batch_toggle, open_phase-Erweiterung, close_phase-Erweiterung)

```rust
const ADMIN_PRIVILEGE: &str = "admin";

// Innerhalb jeder Service-Methode:
let user_id = self.permission_service.current_user_id(context.clone()).await?
    .unwrap_or_else(|| "SYSTEM".to_string());
self.permission_service.check_permission(ADMIN_PRIVILEGE, context).await?;
```

> **Reihenfolge:** ZUERST `current_user_id` für den Audit-Trail, DANN `check_permission`. Phase 7 macht es so, weil ein PermissionDenied-Error trotzdem den `user_id` zur Verfügung hat (für ggf. spätere Audit-Anomaly-Logs).

---

### 2. Audit-Disziplin

**Source:** `genossi_service_impl/src/audit_macros.rs` (audited_create!, audited_update!, audited_delete!)
**Apply to:** Alle Schreib-Operationen auf RepaymentEntry — auch die N Auto-Fill-Inserts im `open_repayment_phase`

**Phase-7-Grep-Gate** (STATE.md Plan 07-03 D-03):
> "Audit-Disziplin-Grep-Gate als Pre-Merge-Check für T-07-03-01 Repudiation-Defense — Filter `grep -v '^//' | grep -v '^*'` … 0 direkte DAO-create/update außerhalb `audited_*!`-Macros verifiziert"

Phase 8 muss denselben Gate vor Merge ausführen:
```bash
# In genossi_service_impl/src/repayment_entry.rs UND in der MODIFIED repayment_phase.rs:
grep -n "repayment_entry_dao\.\(create\|update\)\b" genossi_service_impl/src/*.rs \
  | grep -v "^.*://" \
  | grep -v "^.*: \*"
# Erwartet: NUR Aufrufe innerhalb von audited_*!-Macro-Expansion ODER innerhalb der Macro-Bodies selbst.
```

---

### 3. Single-Transaction-Multi-DAO

**Source:** `genossi_service_impl/src/assembly.rs:181-258` (open_assembly)
**Apply to:** `open_repayment_phase` (Auto-Fill), `close_repayment_phase` (Pending-Validation), `create_repayment_entry` (Phase-Lookup + Member-Lookup + audited_create), `batch_toggle_status` (N audited_update)

Pattern:
1. `let tx = self.transaction_dao.use_transaction(None).await?;`
2. Permission + User-ID (gleiche tx noch nicht touched)
3. Lese-DAOs mit `tx.clone()` (alle sehen denselben Snapshot)
4. Schreib-DAOs via `audited_*!`-Macros (sie nutzen `tx.clone()` intern)
5. **EIN** `self.transaction_dao.commit(tx).await?;` am Ende
6. Error in einer Sub-Operation → Tx-Rollback durch Drop, kein Cleanup-Code nötig (Phase-7-Konvention)

**Achtung — Pool-vs-Tx-Deadlock-Falle aus Phase-3-Plan-05** (STATE.md): Falls Phase 8 später `permission_dao.delete_session` o.ä. pool-basiert aufruft (was hier NICHT der Fall ist), muss `commit` VOR der Pool-Operation passieren. Phase 8 hat keine pool-basierten Operations — sicher.

---

### 4. Error-Handling (Global)

**Source:** `genossi_rest/src/lib.rs:97-113` (`From<ServiceError> for RestError`)
**Apply to:** Alle REST-Handler — KEIN lokaler `map_*_error` (Phase-7-Pattern Plan 07-04)

Mapping:
- `ValidationError` → 400 BadRequest
- `EntityNotFound` → 404
- `Conflict` → 409
- `PermissionDenied` → 401
- `Unauthorized` → 401
- `InternalError` → 500

**Begründung Phase 8 keinen lokalen Override:** Wie Phase 7 ist Phase 8 Vorstand-only ohne Helper-Differenzierung — kein 403-Bedarf.

---

### 5. Optimistic-Locking via Stale-Retry-Pattern (für E2E)

**Source:** `genossi_bin/tests/e2e_tests.rs` Phase-7-Block (STATE.md Plan 07-05 Architekt-Korrektur):
> "Optimistic-Locking via Stale-Retry-Pattern statt direkter Version-Bump-Assertion — codebase-weite Service-Konvention (Assembly, RepaymentPhase, Member) gibt nach `audited_update!` die LOKALE entity mit alter Version zurück; DAO bumpt die DB-Version atomar … Stale-retry-PUT mit alter Version → 409 `Version mismatch` verifiziert die DB-Konsistenz end-to-end."

**Apply to:** Phase 8 E2E `test_update_entry_version_mismatch_returns_conflict` muss diese Stale-Retry-Methode verwenden (NICHT direkte Version-Bump-Assertion auf der API-Response).

---

## No Analog Found

| File | Role | Data Flow | Reason |
|------|------|-----------|--------|
| `BatchStatusRequest` + `batch_toggle_status`-Handler | TO + handler | request-response | Es gibt keinen bestehenden Batch-Toggle-Endpoint in der Codebase, der **all-or-nothing in einer Tx** ist. `genossi_mail/src/mass_mail.rs` (falls vorhanden) ist Best-Effort (CONTEXT explizit als bewusster Pattern-Bruch dokumentiert). Phase 8 kombiniert N `audited_update!` in einer Tx — ein NEUES Pattern, das Phase 9 (`mark_paid_out` mit MemberAction-Cascade) und Phase 10 (Mass-Mail) als Vorlage nehmen können (siehe CONTEXT specifics §4). |
| `CloseConflictResponse`-strukturierter 409-Body | TO + ServiceError-Variante | error-encoding | Phase 7 hat keinen 409 mit strukturiertem Detail-Body — alle bisherigen Conflicts sind `Arc<str>`-basiert. Planner muss entscheiden, ob (a) eine NEUE `ServiceError`-Variante mit Daten oder (b) JSON-encoded `Arc<str>`-Hack genutzt wird. Architekt-Anmerkung in §6 oben. |
| `find_by_phase_id` als DAO-Method | DAO query | CRUD-extension | Phase 7 hat keine `find_by_*_id`-Methode mit Filter — nur `find_by_id`, `find_by_member_number`. Empfehlung: als **Default-Impl auf dem DAO-Trait** in `genossi_dao/src/repayment_entry.rs` (Pattern aus `member.rs:172` `count_active`, alles über `dump_all` + In-Memory-Filter — Performance OK für Genossi-Größenordnung, CONTEXT in-scope §Established Patterns). |

---

## Metadata

**Analog search scope:**
- `genossi_dao/src/` (Phase-7-Vorlage `repayment_phase.rs`, `auditable.rs`, `member.rs`)
- `genossi_dao_impl_sqlite/src/` (Phase-7-Vorlage `repayment_phase.rs`, Phase-3-Index-Vorlage)
- `genossi_service/src/` (Phase-7-Vorlage `repayment_phase.rs`)
- `genossi_service_impl/src/` (Phase-7-Vorlage + `audit_macros.rs` + `macros.rs` + `assembly.rs` für Auto-Fill)
- `genossi_rest/src/` (Phase-7-Vorlage `repayment_phase.rs`, `assembly.rs` für Action-Endpoint utoipa-Patterns)
- `genossi_rest_types/src/lib.rs` (Phase-7-Block Z. 1144-1259)
- `genossi_bin/src/lib.rs` (Phase-7-Wiring Z. 176-200, 701-713, 1311-1316)
- `genossi_rest/src/lib.rs` (Phase-7-Registration Z. 20, 270, 438, 610-612)
- `genossi_bin/tests/e2e_tests.rs` (Phase-7-E2E-Block Z. 10553-10999)
- `migrations/sqlite/` (Phase-7-Migration `20260529190437_create_repayment_phase_table.sql`, Phase-3-Index-Pattern `20260504000000_create_attendance_table.sql`)

**Files scanned:** 14 (alle haben mindestens einen starken Phase-7-Analog; vier Code-Pfade kommen aus Phase-1-Assembly)

**Pattern extraction date:** 2026-05-30

**Cross-Phase-Lessons aktiv:**
- Phase-3-Plan-03 hand-rolled-Mock-Sync-Pflicht (Service-Tests in `repayment_entry.rs::tests`)
- Phase-3-Plan-05 DAO-Sharing-via-Arc::clone (RepaymentEntryDao zwischen Phase- und Entry-Service)
- Phase-3-Plan-06 lokaler Error-Mapper-Override **NICHT** angewendet (kein 403-Bedarf in Phase 8)
- Phase-7-Plan-01 Audit-Field-Reihenfolge frozen + per Unit-Test eingefroren
- Phase-7-Plan-02 `Arc<SqlitePool>`-Constructor + Pre-Exists-Check-Pattern + parse_datetime-Reuse
- Phase-7-Plan-03 Inline-Field-Validator statt validation.rs + Edit-Matrix vor Version-Check + Audit-Disziplin-Grep-Gate
- Phase-7-Plan-04 Minimal-Deps-DI + Trait-Bound-Ergänzung in test_server.rs + audit_log_dao Arc-shared
- Phase-7-Plan-05 Stale-Retry-Pattern statt direkter Version-Bump-Assertion in E2E
