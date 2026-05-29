---
phase: 03-attendance-aggregat-cascade-invalidation
verified: 2026-05-04T00:00:00Z
status: passed
score: 9/9 must-haves verified
overrides_applied: 0
re_verification: false
gaps: []
deferred: []
human_verification: []
---

# Phase 3: Attendance-Aggregat + Cascade-Invalidation Verification Report

**Phase Goal:** Backend stellt reduzierte (DSGVO-konforme) Helfer-Mitgliederliste, idempotente Anwesenheits-Toggles, einen Live-Stats-Endpunkt und einen Vorstand-Post-Close-Edit-Endpoint bereit; das Schließen einer GV invalidiert kaskadierend alle zugehörigen Helfer-Sessions.
**Verified:** 2026-05-04
**Status:** PASSED
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths (aus ROADMAP.md Success Criteria)

| #  | Truth (aus ROADMAP SC) | Req | Status     | Evidence |
|----|------------------------|-----|------------|----------|
| 1  | `GET /api/attendance/:assembly_id/members` liefert ausschließlich Mitgliedsnummer, Name, Titel, Anrede — kein PII-Feld wie IBAN, Adresse, Geburtsdatum, Email | ATTN-01 | ✓ VERIFIED | E2E `test_attendance_members_response_has_no_pii_fields` (e2e_tests.rs:9437): Whitelist-Check (7 erlaubte Keys) + Blacklist-Check (12 verbotene PII-Keys) gegen realen HTTP-Response, alles grün |
| 2  | Substring-Suche auf Name oder Mitgliedsnummer funktioniert | ATTN-02 | ✓ VERIFIED | E2E `test_attendance_members_substring_search_filters_by_query_param` (e2e_tests.rs:9672): `?q=Müll` liefert nur Müller, Schmidt bleibt raus; DAO-LIKE in `attendance.rs` COLLATE NOCASE |
| 3  | Idempotenter `PUT /api/attendance/:aid/:mid` — 5× Aufrufen = genau 1 Row | ATTN-03 | ✓ VERIFIED | E2E `test_attendance_upsert_race_one_row_two_200ok` (e2e_tests.rs:9333): parallele `tokio::join!`-Requests → 2× 200 OK, UPSERT-SQL `ON CONFLICT(assembly_id, member_id) DO UPDATE` in `genossi_dao_impl_sqlite/src/attendance.rs`; DAO-Test `test_upsert_present_idempotent_5x_creates_one_row` |
| 4  | Idempotentes Austragen (anwesend → nicht-anwesend), ebenfalls idempotent in Rückrichtung | ATTN-04 | ✓ VERIFIED | E2E `test_vorstand_can_edit_attendance_after_close` (e2e_tests.rs:9606) enthält DELETE nach close; DAO-Test `test_soft_delete_on_nonexistent_row_is_ok` bestätigt No-Op; UPDATE-Soft-Delete ignoriert rows_affected |
| 5  | Anwesenheits-Markierungen werden NICHT in die Audit-Hashchain geschrieben | ATTN-05 | ✓ VERIFIED | `grep -c "audited_create|audited_update|audited_delete|Auditable" genossi_service_impl/src/attendance.rs` = 0; E2E `test_attendance_toggle_burst_does_not_pollute_audit_chain` (e2e_tests.rs:9518): count_before == count_after für entity_type=attendance + hash-chain valid nach 40 Toggles |
| 6  | Vorstand mit OIDC-Session ruft Helfer-View erfolgreich auf — Permission-Check akzeptiert beide Auth-Pfade | ATTN-06 | ✓ VERIFIED | `check_assembly_access` in `genossi_service_impl/src/attendance.rs:79`: zwei Branches — `ctx.as_helper() == Some(aid)` → Helper-Pfad, sonst `check_permission("admin", ctx)` → Vorstand-Pfad; Tests 1–7 in attendance.rs::tests; alle E2E-Tests laufen im mock_auth-admin-Kontext ohne Helper-Claim |
| 7  | `GET /api/assembly/:id/stats` liefert `{present, total}`; concurrent Doppel-Markierung erzeugt keinen Fehler und keinen Duplikat-Eintrag | ASSY-04, SYNC-02 | ✓ VERIFIED | Endpoint registriert in `genossi_rest/src/lib.rs:611-612` unter `/api/assembly/{assembly_id}/stats`; Handler `get_assembly_stats` in `genossi_rest/src/attendance.rs:196`; `AttendanceStatsTO { present, total }` in `genossi_rest_types/src/lib.rs:1671`; Race-Test `test_attendance_upsert_race_one_row_two_200ok` belegt 0 Duplikat-Rows |
| 8  | `close_assembly` invalidiert kaskadierend alle Helfer-Sessions; nach Schließen schlägt Helfer-Request mit 401 fehl | SC#8 | ✓ VERIFIED | `genossi_service_impl/src/assembly.rs:313-333`: `list_session_ids_for_assembly` + `permission_dao.delete_session`-Loop; E2E `test_close_assembly_cascade_invalidates_helper_sessions` (e2e_tests.rs:9375): direkte sqlx-Query auf `session`-Tabelle vor/nach close — session_count_before=1, session_count_after=0 |
| 9  | Vorstand kann nach GV-Schluss Anwesenheits-Einträge ergänzen/entfernen; GV-Status bleibt Geschlossen | ASSY-06 | ✓ VERIFIED | E2E `test_vorstand_can_edit_attendance_after_close` (e2e_tests.rs:9606): DELETE nach close → 200 OK; AssemblyStatus bleibt `Closed`; Service-Test `test_check_assembly_access_admin_pass_through_no_status_check` (ASSY-06 + D-20: Admin-Branch überspringt Status-Check) |

**Score:** 9/9 Truths verified

### Deferred Items

Keine — alle Phase-3-Requirements sind in Phase 3 vollständig implementiert.

### Required Artifacts

| Artifact | Erwartet | Status | Nachweis |
|----------|----------|--------|----------|
| `migrations/sqlite/20260504000000_create_attendance_table.sql` | Composite-PK-Tabelle, FK, Index | ✓ VERIFIED | Datei existiert; `CREATE TABLE IF NOT EXISTS attendance` mit PK (assembly_id, member_id), FK ON DELETE RESTRICT, partial index `idx_attendance_assembly_present` |
| `genossi_dao/src/attendance.rs` | AttendanceEntity + AttendanceMemberRow + AttendanceDao-Trait | ✓ VERIFIED | Datei existiert; 5-Felder-Entity (kein id/version), 7-Felder-PII-Whitelist-Row, 5-Methoden-Trait mit `#[automock]`; 3/3 Unit-Tests grün |
| `genossi_dao_impl_sqlite/src/attendance.rs` | AttendanceDaoImpl + 6 Modul-Tests | ✓ VERIFIED | Datei existiert; UPSERT `ON CONFLICT(assembly_id, member_id) DO UPDATE`; COLLATE NOCASE-Substring-Search; 6/6 Tests grün |
| `genossi_dao/src/helper_token.rs` | +`list_session_ids_for_assembly`-Method | ✓ VERIFIED | `grep -c 'fn list_session_ids_for_assembly'` = 1; Filter `session_id IS NOT NULL AND deleted IS NULL` |
| `genossi_dao_impl_sqlite/src/helper_token.rs` | SQLx-Impl + 3 Tests | ✓ VERIFIED | Impl existiert; 3/3 Tests grün (redeemed-only, empty-for-unknown, cross-assembly isolation) |
| `genossi_service/src/claim_context.rs` | `ClaimContext::as_helper()` Trait-Default + AuthenticatedContext-Override | ✓ VERIFIED | 2 Treffer für `fn as_helper`; Default → None; Override parst `{"kind":"helper","assembly_id":...}`; 7/7 Tests grün |
| `genossi_service/src/attendance.rs` | AttendanceService Trait + AttendanceStats Domain-Type | ✓ VERIFIED | Trait mit 4 Methoden (list_members, mark_present, mark_absent, stats); `#[automock]` generiert MockAttendanceService; 3/3 Tests grün |
| `genossi_rest_types/src/lib.rs` | AttendanceMemberTO (7 Felder) + AttendanceStatsTO + From-Impls | ✓ VERIFIED | Structs bei Zeilen 1634/1671; PII-Guard-Test `test_attendance_member_to_does_not_contain_pii_keys` grün; From<&AttendanceMemberRow> Konversion ohne MemberTO-Pfad |
| `genossi_service_impl/src/attendance.rs` | AttendanceServiceImpl + check_assembly_access + 4 Endpoint-Methods | ✓ VERIFIED | Datei existiert (1141 Zeilen); `check_assembly_access` als Permission-Funnel für alle 4 Methoden; 14/14 Unit-Tests grün |
| `genossi_service_impl/src/assembly.rs` | close_assembly Cascade-Erweiterung | ✓ VERIFIED | `list_session_ids_for_assembly` (Z. 313) → `commit` (Z. 322) → `delete_session`-Loop (Z. 330); 4 neue Cascade-Tests grün + Phase-1-Regression grün |
| `genossi_bin/src/lib.rs` | DI-Wiring AttendanceServiceImpl + AssemblyServiceDeps-Erweiterung | ✓ VERIFIED | `AttendanceServiceDependencies`, `attendance_service: Arc<AttendanceService>`, `impl AttendanceRestState for RestStateImpl` vorhanden |
| `genossi_rest/src/attendance.rs` | 4 Handler + 2 Router + ApiDoc + map_attendance_error | ✓ VERIFIED | Datei existiert; alle Handler vorhanden; `map_attendance_error` mappt PermissionDenied → Forbidden(403); ApiDoc registriert in lib.rs; 4/4 Unit-Tests grün |
| `genossi_bin/tests/e2e_tests.rs` | 6 E2E-Tests | ✓ VERIFIED | 6 Test-Funktionen vorhanden; 234/234 E2E-Tests grün (6 neue + alle Phase-1/2-Tests) |

### Key Link Verification

| From | To | Via | Status | Nachweis |
|------|----|-----|--------|----------|
| `close_assembly` | `HelperTokenDao::list_session_ids_for_assembly` | cascade discovery | ✓ WIRED | `assembly.rs:313` ruft `self.helper_token_dao.list_session_ids_for_assembly(id, tx.clone()).await?` |
| `close_assembly` | `PermissionDao::delete_session` | cascade loop | ✓ WIRED | `assembly.rs:330` ruft `self.permission_dao.delete_session(sid.as_ref()).await` in for-loop |
| `AttendanceServiceImpl::mark_present/absent` | `AttendanceDao::is_in_snapshot` | snapshot membership gate | ✓ WIRED | `attendance.rs:154+194` — is_in_snapshot vor upsert/soft_delete; Tests 9+11 |
| `check_assembly_access` | `ClaimContext::as_helper()` | helper discrimination | ✓ WIRED | `attendance.rs:97` ruft `ctx.as_helper()` im Context-Branch |
| `list_attendance_members` | `AttendanceService::list_members` | REST → Service | ✓ WIRED | `genossi_rest/src/attendance.rs:86+` übergibt aid, search, ctx an `rest_state.attendance_service().list_members(...)` |
| `genossi_rest::create_app` | `attendance::generate_attendance_route` + `generate_stats_route` | Router::nest | ✓ WIRED | `lib.rs:602-612`: beide `.nest()`-Aufrufe vorhanden |
| `RestStateImpl` | `AttendanceServiceImpl` | DI wiring | ✓ WIRED | `genossi_bin/src/lib.rs:649-660`: `AttendanceServiceImpl { attendance_dao, assembly_dao, ... }` + `impl AttendanceRestState for RestStateImpl` |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Real Data | Status |
|----------|---------------|--------|-----------|--------|
| `list_attendance_members` | `Arc<[AttendanceMemberRow]>` aus `AttendanceService::list_members` | `AttendanceDaoImpl::list_members_for_assembly` → LEFT JOIN auf attendance + assembly_member_snapshot | SQLite-Query mit real gebundenen UUIDs | ✓ FLOWING |
| `get_assembly_stats` | `AttendanceStats { present, total }` | `count_present_by_assembly` (COUNT WHERE deleted IS NULL) + `count_by_assembly_id` (Snapshot-Count) | Beide echte SQLite-COUNT-Queries | ✓ FLOWING |
| `close_assembly` cascade | `Vec<Arc<str>>` Session-IDs | `list_session_ids_for_assembly` → `SELECT session_id FROM helper_token WHERE assembly_id=? AND session_id IS NOT NULL AND deleted IS NULL` | Echte DB-Query; E2E-Test prüft session-Tabelle direkt per sqlx | ✓ FLOWING |
| `AttendanceMemberTO` | Aus `AttendanceMemberRow` | `From<&AttendanceMemberRow>` ohne MemberTO-Zwischenschicht | 7-Spalten-Whitelist-SELECT aus DAO, PII-Guard aktiv | ✓ FLOWING |

### Behavioral Spot-Checks

| Behavior | Ergebnis | Status |
|----------|----------|--------|
| `cargo test -p genossi_dao attendance` | 3/3 grün | ✓ PASS |
| `cargo test -p genossi_dao_impl_sqlite attendance` | 6/6 grün | ✓ PASS |
| `cargo test -p genossi_dao_impl_sqlite list_session_ids_for_assembly` | 3/3 grün | ✓ PASS |
| `cargo test -p genossi_service claim_context --features utoipa` | 7/7 grün | ✓ PASS |
| `cargo test -p genossi_service_impl attendance` | 14/14 grün | ✓ PASS |
| `cargo test -p genossi_service_impl assembly` (inkl. Cascade-Tests) | 22/22 grün (5 cascade, 17 Phase-1/2) | ✓ PASS |
| `cargo test -p genossi_rest --lib attendance` | 4/4 grün | ✓ PASS |
| `cargo test --test e2e_tests` | 234/234 grün (6 neue + 228 Phase-1/2) | ✓ PASS |
| `cargo test --workspace` | alle grün, 0 FAILED | ✓ PASS |

### Requirements Coverage

| Requirement | Pläne | Beschreibung | Status | Nachweis |
|-------------|-------|-------------|--------|----------|
| ASSY-04 | 03-04, 03-05, 03-06 | Live-Counter `{present, total}` für offene GV | ✓ SATISFIED | `AttendanceStats` Domain-Type + `AttendanceStatsTO`; Handler `get_assembly_stats`; E2E Tests #1 + #5 |
| ASSY-06 | 03-05, 03-06 | Vorstand kann nach GV-Schluss Anwesenheit editieren | ✓ SATISFIED | Admin-Branch in `check_assembly_access` überspringt Status-Check (D-20); E2E Test #5 (`test_vorstand_can_edit_attendance_after_close`) |
| ATTN-01 | 03-01, 03-04, 03-06 | Helfer-View nur 4 Felder (Mitgliedsnummer, Name, Titel, Anrede) + member_id + is_present | ✓ SATISFIED | `AttendanceMemberTO` 7-Feld-Whitelist; PII-Guard-E2E-Test mit Whitelist+Blacklist; Whitelist-SELECT im DAO |
| ATTN-02 | 03-01, 03-04, 03-06 | Substring-Suche auf Name oder Mitgliedsnummer | ✓ SATISFIED | `list_members_for_assembly` LIKE COLLATE NOCASE im DAO; E2E Test #6 (`?q=Müll`) |
| ATTN-03 | 03-01, 03-05, 03-06 | Idempotenter PUT (Anwesend markieren) | ✓ SATISFIED | UPSERT SQL `ON CONFLICT DO UPDATE`; Race-E2E-Test #1 mit `tokio::join!` |
| ATTN-04 | 03-01, 03-05, 03-06 | Idempotentes DELETE (Abwesenheit markieren) | ✓ SATISFIED | UPDATE-Soft-Delete rows_affected ignoriert; DAO-Test `test_soft_delete_on_nonexistent_row_is_ok` |
| ATTN-05 | 03-01, 03-05, 03-06 | Keine Audit-Einträge für Anwesenheits-Toggles | ✓ SATISFIED | 0 `audited_*!`-Aufrufe in `attendance.rs`; E2E Test #4 (`count_before == count_after`, hash-chain valid) |
| ATTN-06 | 03-02, 03-03, 03-05 | Vorstand-Zugang zu Helfer-View ohne QR-Token | ✓ SATISFIED | Permission-Funnel akzeptiert beide Auth-Pfade (`as_helper()` + `check_permission("admin")`); Service-Tests 1–7 |
| SYNC-02 | 03-01, 03-05, 03-06 | Doppel-Markierung durch zwei Helfer: kein Fehler, kein Duplikat | ✓ SATISFIED | Atomarer SQLite-UPSERT (single SQL statement); Race-E2E-Test #1 bestätigt 1 Row + 2× 200 OK |

### Anti-Patterns Found

| Datei | Muster | Schwere | Impact |
|-------|--------|---------|--------|
| `genossi_rest/src/lib.rs` | 2 vorbestehende unused-import Warnings | Info | Pre-existing, nicht Phase-3-verursacht |
| `genossi_service_impl/src/timestamp.rs:316` | `unused import: DaoError` | Info | Pre-existing, nicht Phase-3-verursacht |

Keine Blocker. Keine Stubs. Keine Placeholder-Implementierungen.

### Human Verification Required

Keine — alle Acceptance Criteria sind programmatisch verifizierbar und durch automatisierte Tests (Unit + Integration + E2E) abgedeckt.

## Gaps Summary

Keine Gaps. Alle 9 Phase-3-Requirements (ASSY-04, ASSY-06, ATTN-01..06, SYNC-02) plus SC#8 (Cascade-Invalidation) sind vollständig implementiert und durch Codebase-Evidenz belegt:

- 9 DAO-Tests (genossi_dao + genossi_dao_impl_sqlite)
- 3 Cascade-Discovery-DAO-Tests (genossi_dao_impl_sqlite::helper_token)
- 7 ClaimContext-Tests (genossi_service)
- 9 Wire-Type-Tests (genossi_rest_types)
- 14 AttendanceServiceImpl-Unit-Tests
- 5 AssemblyServiceImpl Cascade-Tests (+ Phase-1-Regression grün)
- 4 REST-Layer-Unit-Tests (genossi_rest)
- 6 E2E-Integrationstests (genossi_bin) — alle 9 Phase-3-Requirements je abgedeckt

Gesamt: **234 E2E-Tests grün**, **0 Failures** im gesamten Workspace.

---

_Verified: 2026-05-04_
_Verifier: Claude (gsd-verifier)_
