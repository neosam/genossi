---
phase: 01-assembly-aggregat-audit-hardening
verified: 2026-05-02T22:00:00Z
status: passed
score: 54/54 must-haves verified
overrides_applied: 0
roadmap_success_criteria: 5/5
fix_commits_verified: 11/11
build_status: green
test_status: 693 passed, 0 failed, 2 ignored
e2e_status: 218 passed, 0 failed
---

# Phase 01: Assembly-Aggregat + Audit-Hardening — Verification Report

**Phase Goal (ROADMAP.md):** Datenfundament + Lifecycle für das Assembly-Aggregat (Generalversammlung), inklusive papierloser Anwesenheits-Snapshot-Befüllung beim Open der GV. REST + Service + DAO + Migrationen + E2E-Tests.

**Verified:** 2026-05-02T22:00:00Z
**Status:** passed
**Re-verification:** No — initial verification

## Goal Achievement

### Roadmap Success Criteria (Source of Truth)

| # | Success Criterion (ROADMAP.md) | Status | Evidence |
|---|---|---|---|
| SC-1 | Vorstand kann eine GV mit Datum und Titel anlegen; sie startet im Status `Vorbereitung` (ASSY-01) | VERIFIED | `genossi_service_impl/src/assembly.rs:67-110` `create_assembly` setzt status=Preparation, opened_at=None, closed_at=None. E2E: `test_assembly_lifecycle_audit_chain_intact` POST /api/assembly → 201 + status=Preparation. Note: Wire-Format zeigt englisches `"Preparation"` statt deutsches `"Vorbereitung"` (per D-06/D-17 — bewusste Designentscheidung). |
| SC-2 | Vorstand kann eine GV öffnen — beim Öffnen wird ein Member-Universe-Snapshot persistiert (ASSY-02) | VERIFIED | `open_assembly` in `genossi_service_impl/src/assembly.rs:174-252` — atomare Tx, audited_update! + member_dao.all + filter (is_normal + join_date + exit_date) + snapshot_dao.create_batch + ein commit. E2E-Test verifiziert `snapshot_member_count == 2` nach 2 angelegten aktiven Mitgliedern. |
| SC-3 | Vorstand kann eine GV schließen; Status wechselt final auf `Geschlossen` (ASSY-03) | VERIFIED | `close_assembly` in `genossi_service_impl/src/assembly.rs:254-304`. Pitfall 3: Re-Open nach Close → Conflict. Verifiziert via `test_open_assembly_from_closed_returns_conflict` (StatusCode::CONFLICT). |
| SC-4 | GV-Daten bleiben nach Schluss persistent für Protokoll-Export und Statistik (ASSY-05) | VERIFIED | `assembly_member_snapshot` Tabelle mit Composite-PK (assembly_id, member_id) ohne `deleted` ist permanent (Migration 20260502000001). `count_by_assembly_id` in `get_assembly` liefert die Zahl auch nach Close zurück. |
| SC-5 | `GET /api/audit/verify` zeigt nach Lifecycle-Vorgängen eine intakte Hash-Chain; CI-E2E-Test grün (ASSY-07) | VERIFIED | `test_assembly_lifecycle_audit_chain_intact` (e2e_tests.rs:8361-8513) prüft `verify.valid==true`, `broken_links empty`, `total_entries >= 3`, und alle drei Process-Strings (`assembly.create`, `assembly.open`, `assembly.close`) im Audit-Log via HashSet-Contains. Test ist grün (3 passed, 0 failed). |

**Roadmap Score:** 5/5 Success Criteria VERIFIED.

### PLAN Frontmatter Truths

#### Plan 01-01 (DAO Foundation) — 10 truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1.1 | Migration `assembly` Tabelle existiert mit 10 Spalten (D-05) | VERIFIED | `migrations/sqlite/20260502000000_create_assembly_table.sql:1-12` enthält id, name, date, location, status, opened_at, closed_at, created, deleted, version. |
| 1.2 | Migration `assembly_member_snapshot` mit Composite-PK ohne id/version/deleted/created (D-01) | VERIFIED | `migrations/sqlite/20260502000001_create_assembly_member_snapshot_table.sql:12-19` `PRIMARY KEY (assembly_id, member_id)`, nur 3 Datenfelder. |
| 1.3 | Snapshot-Schema enthält ausschließlich (assembly_id, member_id, captured_at) (D-03) | VERIFIED | `genossi_dao/src/assembly_member_snapshot.rs:8-13` Entity hat genau 3 Felder. |
| 1.4 | Migration-Dateinamen englisch und folgen YYYYMMDDHHMMSS_create_assembly_table.sql (D-15) | VERIFIED | Beide Files am korrekten Pfad mit englischen Namen. |
| 1.5 | Alle neuen Code-Identifier englisch: Assembly, AssemblyEntity, AssemblyDao, etc. (D-16) | VERIFIED | Grep-bestätigt; keine deutschen Identifier (`Vorbereitung`, etc.). |
| 1.6 | AssemblyStatus Enum hat exakt 3 englische Varianten Preparation/Open/Closed (D-06, D-17) | VERIFIED | `genossi_dao/src/assembly.rs:9-14` mit `as_str()` als "Preparation"/"Open"/"Closed". `test_assembly_status_strings_are_english` grün. |
| 1.7 | AssemblyEntity implementiert Auditable mit entity_type=`assembly` und 6 audit_fields (D-10) | VERIFIED | `genossi_dao/src/assembly.rs:58-94`. `test_auditable_fields_count_and_excludes` prüft `fields.len() == 6` und `!field_names.contains("id"/"version"/"created"/"deleted")`. |
| 1.8 | AssemblyDao Trait + Impl folgt Application-Pattern mit Optimistic-Locking | VERIFIED | `genossi_dao_impl_sqlite/src/assembly.rs:148-205` UPDATE WHERE id = ? AND version = ? AND deleted IS NULL; `test_update_with_version_mismatch_returns_conflict` grün. |
| 1.9 | AssemblyMemberSnapshotDao mit create/create_batch/find_by_assembly_id/count_by_assembly_id, KEIN Auditable | VERIFIED | `genossi_dao/src/assembly_member_snapshot.rs:15-45` — Trait hat alle 4 Methoden. Kein `impl Auditable`. |
| 1.10 | Snapshot DAO-Impl wirft DaoError::DatabaseError bei Composite-PK-Verletzung (Pitfall 5) | VERIFIED | `genossi_dao_impl_sqlite/src/assembly_member_snapshot.rs:66-88`; `test_create_duplicate_snapshot_returns_db_error` grün. |

**Plan 01-01 Score:** 10/10

#### Plan 01-02 (REST Types) — 8 truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 2.1 | AssemblyStatusTO Enum mit Varianten Preparation, Open, Closed | VERIFIED | `genossi_rest_types/src/lib.rs:1008-1012`. |
| 2.2 | AssemblyTO mit allen Feldern aus AssemblyEntity, ISO8601-Datetime-Serde | VERIFIED | `genossi_rest_types/src/lib.rs:1037-1078`. iso8601_datetime auf 5 Datetime-Feldern. |
| 2.3 | AssemblyDetailTO enthält assembly + snapshot_member_count: u64 | VERIFIED | `genossi_rest_types/src/lib.rs:1098-1101`. |
| 2.4 | CreateAssemblyRequest hat name, date, location | VERIFIED | `genossi_rest_types/src/lib.rs:1113`. |
| 2.5 | UpdateAssemblyRequest hat name, date, location, version (Optimistic-Locking) | VERIFIED | `genossi_rest_types/src/lib.rs:1128-1140` mit `pub version: Uuid` (kein Option, kein default). |
| 2.6 | Bidirektionale From-Impls AssemblyStatus <-> AssemblyStatusTO | VERIFIED | `genossi_rest_types/src/lib.rs:1014-1033`. |
| 2.7 | From<&genossi_service::assembly::Assembly> for AssemblyTO existiert | VERIFIED | `genossi_rest_types/src/lib.rs:1080-1095`. |
| 2.8 | Alle TOs haben #[derive(ToSchema)] für OpenAPI | VERIFIED | Grep zeigt ToSchema auf allen 5 Types. |

**Plan 01-02 Score:** 8/8

#### Plan 01-03 (Service Layer) — 13 truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 3.1 | AssemblyService Trait definiert 6 Methoden | VERIFIED | `genossi_service/src/assembly.rs:101-154`. |
| 3.2 | create_assembly setzt Preparation, opened_at=None, closed_at=None, audited_create! mit "assembly.create" | VERIFIED | `genossi_service_impl/src/assembly.rs:67-110`. |
| 3.3 | open_assembly prüft entity.status == Preparation, sonst Conflict | VERIFIED | `genossi_service_impl/src/assembly.rs:200-205`. |
| 3.4 | open_assembly setzt status=Open, opened_at=now, audited_update! mit "assembly.open" | VERIFIED | `genossi_service_impl/src/assembly.rs:210-221`. |
| 3.5 | open_assembly befüllt snapshot atomar mit count_active-Filter (D-02) | VERIFIED | `genossi_service_impl/src/assembly.rs:231-248` — is_normal + join_date + exit_date filter. WR-02 fix: `join_date <= opened_date` zusätzlich (verifiziert via `test_open_assembly_excludes_future_joiner_from_snapshot`). |
| 3.6 | close_assembly prüft entity.status == Open, sonst Conflict | VERIFIED | `genossi_service_impl/src/assembly.rs:278-283`. |
| 3.7 | close_assembly setzt status=Closed, closed_at=now, audited_update! mit "assembly.close" | VERIFIED | `genossi_service_impl/src/assembly.rs:285-298`. |
| 3.8 | close_assembly enthält KEINEN HelperSession-Cascade-Code | VERIFIED | Grep findet nur einen Match in einem D-09-Reminder-Comment, kein Code-Pfad. |
| 3.9 | update_assembly prüft entity.status == Preparation, sonst Conflict (D-07) | VERIFIED | `genossi_service_impl/src/assembly.rs:144-150`. |
| 3.10 | update_assembly prüft entity.version == request.version, sonst Conflict | VERIFIED | `genossi_service_impl/src/assembly.rs:151-154`. |
| 3.11 | Alle 6 Service-Methoden rufen permission_service.check_permission("admin") auf | VERIFIED | Grep `ADMIN_PRIVILEGE` zeigt 7 Vorkommen (1 const + 6 call sites). |
| 3.12 | open_assembly verwendet eine einzige use_transaction(None), tx.clone() für Sub-Calls, EIN commit (Pitfall 2) | VERIFIED | `genossi_service_impl/src/assembly.rs:180` (use_transaction) + `:250` (commit) — innerhalb des open_assembly-Blocks genau einer von jedem. |
| 3.13 | Snapshot-DAO-Aufrufe verwenden NICHT audited_create! (Pitfall 1) | VERIFIED | Grep `assembly_member_snapshot_dao.*audited_` ergibt 0 Treffer. |

**Plan 01-03 Score:** 13/13

#### Plan 01-04 (REST Handlers + DI) — 14 truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 4.1 | Endpoint POST /api/assembly existiert und ruft service.create_assembly | VERIFIED | `genossi_rest/src/assembly.rs:152-188`. |
| 4.2 | POST /api/assembly/{id}/open existiert | VERIFIED | `genossi_rest/src/assembly.rs:290-311`. |
| 4.3 | POST /api/assembly/{id}/close existiert | VERIFIED | `genossi_rest/src/assembly.rs:326-347`. |
| 4.4 | PUT /api/assembly/{id} existiert | VERIFIED | `genossi_rest/src/assembly.rs:237-275`. |
| 4.5 | GET /api/assembly liefert Liste der Assemblies | VERIFIED | `genossi_rest/src/assembly.rs:118-138`. |
| 4.6 | GET /api/assembly/{id} liefert AssemblyDetailTO mit snapshot_member_count | VERIFIED | `genossi_rest/src/assembly.rs:202-220`. |
| 4.7 | Alle Handler delegieren Permission-Check an Service-Layer | VERIFIED | Handler verwenden `extract_auth_context`; Service ruft `check_permission("admin")` (siehe 3.11). |
| 4.8 | RestState wiring registriert AssemblyServiceImpl mit allen 7 Deps | VERIFIED | `genossi_bin/src/lib.rs:494-500` — alle 7 Felder. |
| 4.9 | Router nestet /api/assembly route | VERIFIED | `genossi_rest/src/lib.rs:566` `.nest("/api/assembly", assembly::generate_route::<RestState>())`. |
| 4.10 | OpenAPI-ApiDoc nestet /api/assembly | VERIFIED | `genossi_rest/src/lib.rs:252` `(path = "/api/assembly", api = assembly::ApiDoc)`. |
| 4.11 | Validierung: name nicht leer, name max 256 chars, location max 256 chars | VERIFIED | `genossi_rest/src/assembly.rs:28-66` — WR-05 fix verwendet `chars().count()`. Tests: `test_validate_create_assembly_request_unicode_counts_chars_not_bytes`. |
| 4.12 | POST /api/assembly returnt HTTP 201 bei Erfolg | VERIFIED | `genossi_rest/src/assembly.rs:181` `.status(201)`. E2E verifiziert `StatusCode::CREATED`. |
| 4.13 | PUT, GET, open, close returnen HTTP 200 bei Erfolg | VERIFIED | Grep `.status(200)` mehrfach in den vier Handlern. |
| 4.14 | Lifecycle-Conflict-Fälle returnen HTTP 409 | VERIFIED | E2E-Tests `test_close_assembly_from_preparation_returns_conflict` und `test_open_assembly_from_closed_returns_conflict` asserten `StatusCode::CONFLICT`. |

**Plan 01-04 Score:** 14/14

#### Plan 01-05 (E2E Tests) — 9 truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 5.1 | E2E-Test test_assembly_lifecycle_audit_chain_intact existiert | VERIFIED | `genossi_bin/tests/e2e_tests.rs:8361`. |
| 5.2 | Test führt Sequenz aus: POST /api/assembly (201) -> open (200) -> close (200) | VERIFIED | `e2e_tests.rs:8371-8463`. |
| 5.3 | Test ruft GET /api/audit/verify, prüft valid==true und broken_links empty | VERIFIED | `e2e_tests.rs:8466-8486`. |
| 5.4 | Test assertet total_entries >= 3 | VERIFIED | `e2e_tests.rs:8482-8486`. |
| 5.5 | Test prüft Process-Strings 'assembly.create', 'assembly.open', 'assembly.close' im Audit-Log | VERIFIED | `e2e_tests.rs:8488-8512` HashSet-Contains für alle 3. |
| 5.6 | Test verifiziert opened.status==Open, closed.status==Closed, opened_at/closed_at present | VERIFIED | `e2e_tests.rs:8423-8463`. |
| 5.7 | Negativ-Test test_close_assembly_from_preparation_returns_conflict deckt Pitfall 3 | VERIFIED | `e2e_tests.rs:8517-8547`, asserts `StatusCode::CONFLICT`. |
| 5.8 | Negativ-Test test_open_assembly_from_closed_returns_conflict deckt Pitfall 3 | VERIFIED | `e2e_tests.rs:8551-8590`, asserts `StatusCode::CONFLICT`. |
| 5.9 | Tests verwenden setup() mit In-Memory-SQLite, mock_auth | VERIFIED | Alle drei Tests starten mit `let server = setup().await;`. |

**Plan 01-05 Score:** 9/9

### Aggregated Truth Score

**Total: 54/54 truths VERIFIED**

## Required Artifacts (Three Levels)

| Artifact | Exists | Substantive | Wired | Data Flows | Status |
|----------|--------|-------------|-------|------------|--------|
| `migrations/sqlite/20260502000000_create_assembly_table.sql` | ✓ | ✓ | ✓ (sqlx::migrate!) | ✓ | VERIFIED |
| `migrations/sqlite/20260502000001_create_assembly_member_snapshot_table.sql` | ✓ | ✓ | ✓ | ✓ | VERIFIED |
| `genossi_dao/src/assembly.rs` | ✓ | ✓ (252 LOC, 7 tests) | ✓ (registered + imported) | ✓ | VERIFIED |
| `genossi_dao/src/assembly_member_snapshot.rs` | ✓ | ✓ (66 LOC) | ✓ | ✓ | VERIFIED |
| `genossi_dao_impl_sqlite/src/assembly.rs` | ✓ | ✓ (368 LOC, 4 tests) | ✓ | ✓ | VERIFIED |
| `genossi_dao_impl_sqlite/src/assembly_member_snapshot.rs` | ✓ | ✓ (306 LOC, 5 tests) | ✓ | ✓ | VERIFIED |
| `genossi_rest_types/src/lib.rs` (Assembly types) | ✓ | ✓ (5 types + From impls) | ✓ | ✓ | VERIFIED |
| `genossi_service/src/assembly.rs` | ✓ | ✓ (240 LOC, 6 tests) | ✓ | ✓ | VERIFIED |
| `genossi_service_impl/src/assembly.rs` | ✓ | ✓ (1041 LOC, 8 tests) | ✓ (DI in genossi_bin) | ✓ | VERIFIED |
| `genossi_rest/src/assembly.rs` | ✓ | ✓ (521 LOC, 11 tests) | ✓ (router + ApiDoc) | ✓ | VERIFIED |
| `genossi_rest/src/lib.rs` (modifications) | ✓ | ✓ (5 hooks: mod, ApiDoc, 2 bounds, .nest) | ✓ | ✓ | VERIFIED |
| `genossi_rest/src/test_server.rs` | ✓ | ✓ (bound updated) | ✓ | ✓ | VERIFIED |
| `genossi_bin/src/lib.rs` (DI wiring) | ✓ | ✓ (type aliases, deps struct, field, ::new(), impl) | ✓ | ✓ | VERIFIED |
| `genossi_bin/tests/e2e_tests.rs` (3 new tests) | ✓ | ✓ | ✓ | ✓ | VERIFIED |

## Key Link Verification

| From | To | Via | Status |
|------|-----|-----|--------|
| `AssemblyEntity` | `Auditable trait` | `impl crate::auditable::Auditable for AssemblyEntity` | WIRED |
| `AssemblyDaoImpl::update` | Optimistic-Locking | `WHERE id = ? AND version = ? AND deleted IS NULL` | WIRED |
| `AssemblyServiceImpl::open_assembly` | atomare Tx + Snapshot | `use_transaction(None)` + `audited_update!` + `snapshot_dao.create_batch` + `commit` | WIRED |
| `AssemblyServiceImpl::create_assembly` | Audit-Hashchain | `audited_create!` mit `"assembly.create"` | WIRED |
| `genossi_rest/src/lib.rs router` | `/api/assembly` | `.nest("/api/assembly", assembly::generate_route::<RestState>())` | WIRED |
| `RestStateImpl` | `AssemblyService` | `impl genossi_rest::assembly::AssemblyRestState for RestStateImpl` | WIRED |
| Handler `open_assembly` | `AssemblyServiceImpl::open_assembly` | `rest_state.assembly_service().open_assembly(id, auth).await` | WIRED |
| `From<DaoError> for ServiceError` | `Conflict mapping` (CR-01) | `DaoError::ConflictError(msg) => ServiceError::Conflict(msg)` | WIRED |

## Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Workspace builds clean | `SQLX_OFFLINE=true cargo build --workspace` | Finished `dev` profile (only pre-existing warnings) | PASS |
| All assembly tests pass | `cargo test ... assembly` (across 5 crates) | 8 + 9 + 11 + 6 + 8 = 42 assembly tests passed | PASS |
| All e2e tests pass | `cargo test --test e2e_tests` | 218 passed, 0 failed | PASS |
| Workspace test totals | `cargo test --workspace` | 693 passed, 0 failed, 2 ignored | PASS |
| New e2e tests pass | `cargo test --test e2e_tests assembly` | 3 passed, 0 failed | PASS |

## Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| ASSY-01 | 01-01, 01-02, 01-03, 01-04 | GV anlegen mit Datum/Titel, Initial-Status `Vorbereitung` | SATISFIED | DAO + Service + REST + E2E. Note: Status-String ist englisch `"Preparation"` per D-06/D-17. |
| ASSY-02 | 01-01, 01-02, 01-03, 01-04 | GV öffnen + Member-Universe-Snapshot persistieren | SATISFIED | `open_assembly` mit atomarer Tx + count_active-Filter + snapshot create_batch. |
| ASSY-03 | 01-02, 01-03, 01-04 | GV schließen, Status `Geschlossen`. Helfer-Sessions invalidiert in Phase 3. | SATISFIED (Phase-1-Scope) | `close_assembly` setzt Status. Helfer-Sessions sind Phase 2/3, korrekt deferred. |
| ASSY-05 | 01-01, 01-02, 01-03, 01-04 | GV-Daten + Snapshot persistent für Protokoll-Export | SATISFIED | Snapshot-Tabelle ohne `deleted`, `count_by_assembly_id` liefert nach Close. |
| ASSY-07 | 01-03, 01-05 | Lifecycle-Vorgänge via bestehender Audit-Hashchain protokolliert | SATISFIED | E2E-Test verifiziert `verify.valid==true`, alle 3 Process-Strings im Audit-Log. |

**Note:** ASSY-04 (Live-Counter) und ASSY-06 (Post-Close-Edit) sind explicit **deferred to Phase 3** per ROADMAP.md. Phase 1 PLAN frontmatter listet sie korrekt nicht. ROADMAP-Phase-3-Section adressiert beide explizit.

**Coverage Score:** 5/5 in-scope ASSY requirements SATISFIED.

## CR-01 + WR-01..WR-09 Fix Commits Verification

All 11 fix commits from `01-REVIEW-FIX.md` are present and verified:

| Commit | Issue | Status |
|--------|-------|--------|
| `3f7eb1f` | CR-01 — DaoError::ConflictError → ServiceError::Conflict | VERIFIED in `genossi_service/src/lib.rs:70` + 3 unit tests |
| `b26c75b` | WR-01 — `assert_ne!(req.version, Uuid::nil())` | VERIFIED in `genossi_rest/src/assembly.rs:494-498` |
| `593e736` | WR-02 — `join_date <= opened_date` filter | VERIFIED in `genossi_service_impl/src/assembly.rs:235` + `test_open_assembly_excludes_future_joiner_from_snapshot` |
| `bdd1375` | WR-03 — FK doc comments | VERIFIED in migration NOTE block + DAO doc comment |
| `8b66456` | WR-04 — duplicate find_by_id documented | VERIFIED via 3 WR-04 comments in `assembly.rs:129, 191, 270` |
| `1d69aa6` | WR-05 — `chars().count()` UTF-8 length | VERIFIED in `genossi_rest/src/assembly.rs:39, 58` + 2 unicode tests |
| `d2ed7f2` | WR-06 — `get_assembly` snapshot count tests | VERIFIED via `test_get_assembly_returns_snapshot_member_count` (unit) + extended e2e (`snapshot_member_count == 2`) |
| `3505182` | WR-07 — `deleted` field doc | VERIFIED in module-level doc comment lines 11-23 |
| `cae6c0c` | WR-08 — `format_dt` logs error + sentinel | VERIFIED in `genossi_dao/src/assembly.rs:73-84` (`tracing::error!` + `"<invalid datetime>"`) |
| `2121f7d` | WR-09 — UTC `Z` suffix in e2e date strings | VERIFIED — all three e2e tests use `"2026-06-15T18:00:00.000000000Z"` |
| `072f31e` | CR-01 follow-up — `test_action_update_version_conflict` expects 409 | VERIFIED — `e2e_tests.rs:1444` asserts `StatusCode::CONFLICT` |

**Fix Commits Score:** 11/11 commits verified, 0 missing.

## Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| (none) | — | — | — | No new anti-patterns introduced by Phase 01. |

**Note on existing warnings:**
- `genossi_rest/src/permission.rs:780`: Pre-existing unused import (not introduced by Phase 01).
- `genossi_rest/src/lib.rs:27`: Pre-existing unused import (not introduced by Phase 01).
- `genossi_bin/src/lib.rs:606`: Pre-existing unused import in `initialize_audit_snapshot` (not introduced by Phase 01).

These warnings predate Phase 01 and are out of scope. They do not affect goal achievement.

## Deferred Items (Phase 3 scope per ROADMAP.md)

| # | Item | Addressed In | Evidence |
|---|------|-------------|----------|
| 1 | ASSY-04 — Live-Counter `X von Y anwesend` (Stats-Endpoint) | Phase 3 | ROADMAP.md SC-7 of Phase 3: "GET /api/assembly/:id/stats liefert {present, total} (ASSY-04, SYNC-02)" |
| 2 | ASSY-06 — Post-Close-Edit-Endpoint | Phase 3 | ROADMAP.md SC-9 of Phase 3: "Vorstand kann nach GV-Schluss Anwesenheits-Einträge ergänzen oder entfernen (ASSY-06)" |
| 3 | HelperSession Cascade-Invalidation in close_assembly | Phase 3 | ROADMAP.md SC-8 of Phase 3: "close_assembly invalidiert kaskadierend alle Helfer-Sessions dieser GV". Phase 1 D-09 deliberately omits this; service has explicit comment. |
| 4 | DELETE-Endpoint und audited_delete! für Assembly | Phase 2/3 (future) | WR-07 fix dokumentiert das explizit als deferred. Schema-Field `deleted` ist Vorbereitung. |

## Human Verification Required

None — all phase 1 truths are verifiable programmatically (DAO unit tests, service mock tests, REST validation tests, E2E tests with real HTTP). No UI/visual/real-time/external-service surface in this phase.

## Gaps Summary

**No gaps found.** Phase 01 goal is achieved end-to-end:

1. **Datenfundament:** Migrationen + DAOs für `assembly` und `assembly_member_snapshot` — vollständig, Optimistic-Locking + Composite-PK durch Tests belegt.
2. **Lifecycle:** Service-Layer mit Preparation→Open→Closed-Guards, Optimistic-Locking auf Update, atomarer Open-Tx mit Snapshot-Befüllung — durch 8 Mock-Tests belegt.
3. **Snapshot-Befüllung:** count_active-Filter mit zusätzlichem `join_date <= opened_date` (WR-02 fix) — durch dedizierten Mock-Test belegt; e2e-Test verifiziert `snapshot_member_count == 2` nach 2 angelegten Members.
4. **REST + DI:** 6 Endpoints, OpenAPI-Schema, korrekte Status-Codes (201/200/409/404), Service-Layer-Permission-Check.
5. **E2E-Tests:** 3 Tests in `genossi_bin/tests/e2e_tests.rs` belegen Lifecycle + Audit-Hashchain-Verify + Process-Identifier (`assembly.create`, `assembly.open`, `assembly.close`).
6. **Audit-Hardening:** CR-01 fix mappt DAO-ConflictError → Service-Conflict → HTTP 409; alle 9 Warnings adressiert; Folge-Commit hat eine vorherige bug-for-bug-Test-Assertion korrigiert.

**Build:** `cargo build --workspace` grün.
**Tests:** 693 passed, 0 failed, 2 ignored across 22 crates. 218 e2e tests passed.

Phase 01 is **complete and ready** for Phase 02 (Helfer-Token + Session + AuthContext::Helper).

---

_Verified: 2026-05-02T22:00:00Z_
_Verifier: Claude (gsd-verifier)_
