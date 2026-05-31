---
phase: 08-repaymententry-auto-bef-llung
verified: 2026-05-31T22:30:00Z
status: passed
score: 5/5 ROADMAP Success Criteria verified (alle Quality-Gaps geschlossen)
overrides_applied: 0
re_verification:
  previous_status: gaps_found
  previous_score: "5/5 ROADMAP SCs verified mit 2 BLOCKER-Quality-Gaps innerhalb SC#3/SC#4"
  gaps_closed:
    - "CR-01 (stale version response): RepaymentEntry::update + batch_toggle + alle 4 RepaymentPhase-Lifecycle-Methoden re-lesen jetzt nach audited_*! die Entity"
    - "CR-02 (404 vs 409 im Batch-Toggle): NotFound-Branch in batch_toggle_status mappt auf ServiceError::EntityNotFound → HTTP 404; OpenAPI dokumentiert 404 explizit; BatchFailureResponse-Schema-Doc grenzt 409-Scope ab"
    - "IN-04 (Test-Coverage-Lücke für 2nd-PUT-mit-Response-Version): 5 neue E2E-Regressionstests am Datei-Ende von e2e_tests.rs (4× CR-01-Lock-In + 1× CR-02-404-Lock-In)"
    - "BL-01 (Re-Read None Mapping): Alle sechs Re-Read-Sites mappen die strukturell-unmögliche `None`-Verzweigung auf ServiceError::InternalError → HTTP 500 (statt EntityNotFound → 404), plus 2 Negativtests"
  gaps_remaining: []
  regressions: []
human_verification: []
---

# Phase 8: RepaymentEntry + Auto-Befüllung — Re-Verification Report (Gap-Closure Complete)

**Phase Goal:** RepaymentEntry-Aggregat mit Auto-Befüllung beim Phase-Öffnen,
manueller Ergänzung, und Status-Toggle `offen ↔ angeschrieben` (ohne `ausbezahlt` —
kommt in Phase 9).

**Verified:** 2026-05-31T22:30:00Z
**Status:** passed
**Re-verification:** Yes — nach Gap-Closure-Wave (Plans 08-07/08/09/10 + BL-01-Fix)

## Re-Verification Summary

Die Initial-Verifikation (2026-05-31T18:00:00Z) bestätigte alle 5 ROADMAP Success
Criteria grundsätzlich, identifizierte aber 2 BLOCKER-Quality-Gaps (CR-01 stale
version response, CR-02 404 vs 409 in Batch-Toggle) und 1 Test-Coverage-Lücke
(IN-04 — kein 2nd-PUT-Test mit Response-Version). Eine post-Closure-Re-Review
identifizierte zusätzlich BL-01 (Re-Read-None-Mapping auf 404 statt 500).

Alle vier Gaps sind jetzt geschlossen:

| Gap | Fix-Plan | Status |
|-----|----------|--------|
| CR-01 (RepaymentEntry stale version) | 08-07 (Commits ee44b26, 2c0f503) | ✓ CLOSED |
| CR-01 (RepaymentPhase stale version, Phase-7-Erbe in 4 Lifecycle-Methoden) | 08-08 (Commits aba0c1e, 9305eac) | ✓ CLOSED |
| CR-02 (Batch-Toggle 404 vs 409) | 08-09 (Commits 9d52ebd, f029b6a, e0628aa) | ✓ CLOSED |
| IN-04 (Test-Coverage-Lücke 2nd-PUT) | 08-10 (Commit 0262b63) | ✓ CLOSED |
| BL-01 (Re-Read None → 500 statt 404) | bc022bf, 87539bb, 7ad522c | ✓ CLOSED |

## Goal Achievement

### Observable Truths (ROADMAP Success Criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Migration legt `repayment_entry`-Tabelle an (kein Composite-PK, eigene UUID; `member_id`, `phase_id`, `share_count_to_pay_out INTEGER`, `status TEXT`, `created`, `deleted`, `version`) | ✓ VERIFIED | `migrations/sqlite/20260530203550_create_repayment_entry_table.sql` (Initial-Verifikation bestätigt, kein Change) |
| 2 | Phase-Öffnen (`open_phase`) befüllt atomar Einträge für alle Mitglieder mit `exit_date BETWEEN ? AND ?` (Geschäftsjahres-Range) — `share_count_to_pay_out = Member.current_shares`-Snapshot | ✓ VERIFIED | `genossi_service_impl/src/repayment_phase.rs:280-355` Auto-Fill-Block unverändert in derselben Tx; alle 6 Unit-Tests + 4 E2E-Tests grün |
| 3 | Manuelles `create_entry` über REST funktioniert; mehrere Einträge pro Mitglied+Phase im selben State verifiziert durch Integration-Test | ✓ VERIFIED (Quality-Gap CR-01 jetzt geschlossen) | POST `/api/repayment-entry` Handler + E2E-Tests grün. **CR-01 Fix:** `update_repayment_entry` re-liest Entity nach `audited_update!` (Z. 265-290); E2E-Test `test_update_entry_followup_put_uses_response_version_returns_200` (e2e_tests.rs:11677) verifiziert dass `version_after_put1 != create_version` und 2. PUT mit der frischen Version 200 liefert |
| 4 | Status-Toggle `offen ↔ angeschrieben` ist multi-select-fähig (Batch-Endpoint); Audit-Eintrag pro Toggle | ✓ VERIFIED (Quality-Gaps CR-01 + CR-02 jetzt geschlossen) | Batch-Endpoint funktional; **CR-01 Fix:** Re-Read pro Iteration nach `audited_update!` (Z. 482-507); **CR-02 Fix:** NotFound-Branch mappt auf `ServiceError::EntityNotFound` → 404 (Z. 441-453); OpenAPI dokumentiert 404 + 409 explizit (rest/repayment_entry.rs:264-265); E2E-Tests `test_batch_toggle_followup_put_uses_response_versions` (e2e_tests.rs:11751) + `test_batch_toggle_with_unknown_entry_id_returns_404` (e2e_tests.rs:11908) verifizieren beide Pfade |
| 5 | `close_phase` (PHAS-03) blockt mit 409 Conflict wenn mindestens ein Eintrag nicht `ausbezahlt` ODER `deleted IS NULL` ist — E2E-Test deckt Negative-Path | ✓ VERIFIED | Pending-Validation-Block unverändert; **CR-01 Fix:** `close_repayment_phase` re-liest Phase-Entity (Z. 520-540); E2E-Tests grün |

**Score:** 5/5 ROADMAP Success Criteria komplett erfüllt — alle Quality-Gaps innerhalb SC#3 und SC#4 sind im Codebase nachweislich geschlossen.

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `migrations/sqlite/20260530203550_create_repayment_entry_table.sql` | DDL + 3 Indizes + FK-Doku | ✓ VERIFIED | Unverändert (Initial-Verifikation) |
| `genossi_dao/src/repayment_entry.rs` | Enum + Entity + Auditable + Trait + Mock | ✓ VERIFIED | Unverändert |
| `genossi_dao_impl_sqlite/src/repayment_entry.rs` | SQLite-DaoImpl mit Pre-Exists + Optimistic-Locking | ✓ VERIFIED | Unverändert |
| `genossi_service/src/repayment_entry.rs` | Trait + DTOs | ✓ VERIFIED | Unverändert |
| `genossi_service_impl/src/repayment_entry.rs` | Service-Impl mit Audit-Macros + Re-Read + korrektes Error-Mapping | ✓ VERIFIED (Gaps geschlossen) | 23 Unit-Tests grün (war 19 — 4 neu: 2× CR-01 Re-Read, 1× CR-02 NotFound-Mapping, 1× BL-01 Re-Read None→InternalError); Re-Read-Pattern in `update_repayment_entry` + `batch_toggle_status`; NotFound-Branch mappt auf `EntityNotFound`; Re-Read-None mappt auf `InternalError` (HTTP 500) |
| `genossi_service_impl/src/repayment_phase.rs` | Auto-Fill + Pending-Validation + Re-Read in 4 Lifecycle-Methoden | ✓ VERIFIED (Gaps geschlossen) | 26 Tests grün (war 23 — 3 neu: 2× CR-01 Re-Read für update + open, 1× BL-01 Re-Read None→InternalError); Re-Read-Pattern in `create_repayment_phase` (find_by_id(entity.id)), `update_repayment_phase`, `open_repayment_phase` (NACH /PHAS-02, VOR commit), `close_repayment_phase`; Auto-Fill-Block in `open_repayment_phase` strukturell unverändert |
| `genossi_rest_types/src/lib.rs` | 7 TOs inkl. BatchFailureResponse + CloseConflictResponse + erweiterte Scope-Doc | ✓ VERIFIED | 7 Unit-Tests grün; `BatchFailureResponse` Doc-Comment erweitert um "NOT used for: missing or soft-deleted entry_ids. Those cases yield HTTP 404..." |
| `genossi_rest/src/repayment_entry.rs` | 6 Handler + Router + ApiDoc + 404-Response für /batch-status | ✓ VERIFIED | 3 Unit-Tests grün; OpenAPI für POST `/batch-status` listet `status = 404` mit "missing or soft-deleted" + "NOT BatchFailureResponse" + "see 404"-Cross-Reference im 409-Description |
| `genossi_rest/src/lib.rs` | Modul + Nest + Trait-Bounds | ✓ VERIFIED | Unverändert |
| `genossi_rest/src/test_server.rs` | Trait-Bound erweitert | ✓ VERIFIED | Unverändert |
| `genossi_bin/src/lib.rs` | DI-Wiring + RestState-Bridge | ✓ VERIFIED | Unverändert |
| `genossi_bin/tests/e2e_tests.rs` | 15 Phase-8-E2E-Tests + 5 neue Regression-Tests | ✓ VERIFIED (IN-04 geschlossen) | 275 E2E-Tests grün (war 270 — 5 neue Regressionstests am Datei-Ende: `test_update_entry_followup_put_uses_response_version_returns_200`, `test_batch_toggle_followup_put_uses_response_versions`, `test_open_phase_response_version_usable_for_followup_update`, `test_update_phase_response_version_usable_for_followup_update`, `test_batch_toggle_with_unknown_entry_id_returns_404`) |

### Key Link Verification

| From | To | Via | Status | Details |
|------|-----|-----|--------|---------|
| `RestStateImpl.repayment_entry_service` | RepaymentEntryServiceImpl mit 7 Arc-shared Deps | gen_service_impl! + Arc::clone | ✓ WIRED | Workspace baut clean |
| Router POST /api/repayment-entry/batch-status | batch_toggle_status Handler | `.route("/batch-status", post(...))` | ✓ WIRED | Router-Reihenfolge VOR `/{id}` |
| `open_repayment_phase` Auto-Fill | `member_dao.all` + N `audited_create!` auf repayment_entry_dao | gleiche Tx | ✓ WIRED | E2E grün |
| `close_repayment_phase` Pending-Check | `repayment_entry_dao.find_by_phase_id` + `member_dao.all` | filter + JSON-Body | ✓ WIRED | E2E grün |
| `update_repayment_entry` → Client | RepaymentEntryTO mit DAO-frischer version | `RepaymentEntry::from(&refreshed)` (CR-01 Fix) | ✓ WIRED | E2E `test_update_entry_followup_put_uses_response_version_returns_200` verifiziert `version_after_put1 != create_version` + 200 auf 2. PUT |
| `batch_toggle_status` → Client | Vec<RepaymentEntryTO> mit DAO-frischen Versionen | `updated.push(RepaymentEntry::from(&refreshed))` (CR-01 Fix) | ✓ WIRED | E2E `test_batch_toggle_followup_put_uses_response_versions` verifiziert dass Folge-PUT mit `updated_batch[0].version` 200 liefert |
| `batch_toggle_status` NotFound | `ServiceError::EntityNotFound(*entry_id)` → HTTP 404 | `.ok_or(ServiceError::EntityNotFound(*entry_id))` (CR-02 Fix) | ✓ WIRED | E2E `test_batch_toggle_with_unknown_entry_id_returns_404` verifiziert StatusCode::NOT_FOUND auf Mixed-Validity-Batch |
| `update_*` Re-Read None | `ServiceError::InternalError` → HTTP 500 | `.ok_or_else(\|\| ServiceError::InternalError(...))` (BL-01 Fix) | ✓ WIRED | 2 Negativtests verifizieren via mockall `Ok(None)` als Re-Read-Response |
| 4 RepaymentPhase-Lifecycle → Client | RepaymentPhase mit DAO-frischen Versionen | `Ok(RepaymentPhase::from(&refreshed))` (CR-01 Fix) | ✓ WIRED | 4× `Ok(RepaymentPhase::from(&refreshed))` im File (create, update, open, close); 1× `Ok(RepaymentPhase::from(&entity))` verbleibend in `get_repayment_phase` (read-only, korrekt); E2E `test_open_phase_response_version_usable_for_followup_update` + `test_update_phase_response_version_usable_for_followup_update` verifizieren |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|---------------|--------|--------------------|--------|
| `RepaymentEntryDaoImpl::dump_all` | rows | sqlx::query_as gegen repayment_entry | Ja | ✓ FLOWING |
| Auto-Fill-Block | targets | member_dao.all() + filter on exit_date/current_shares | Ja | ✓ FLOWING |
| Pending-Validation-Block | pending | repayment_entry_dao.find_by_phase_id(id) + filter | Ja | ✓ FLOWING |
| `update_repayment_entry` Response | refreshed.version | DAO post-update find_by_id (CR-01 Fix) | Ja — DAO-frische UUID, E2E-verifiziert via `assert_ne!(version_after_put1, create_version)` | ✓ FLOWING |
| `batch_toggle_status` Response | updated[i].version | Re-Read pro Iteration (CR-01 Fix) | Ja — N frische UUIDs, E2E-verifiziert via 2. PUT mit `updated_batch[0].version` | ✓ FLOWING |
| 4 RepaymentPhase-Lifecycle Responses | refreshed.version | DAO post-write find_by_id (CR-01 Fix) | Ja — E2E-verifiziert via PUT mit `opened.version` + `updated.version` | ✓ FLOWING |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Workspace baut clean | `cargo build --workspace` | Finished `dev` profile (1 unused-imports-Warning in genossi_bin pre-existing) | ✓ PASS |
| Service-Impl Tests RepaymentEntry | `cargo test -p genossi_service_impl --lib repayment_entry` | 23 passed; 0 failed | ✓ PASS |
| Service-Impl Tests RepaymentPhase | `cargo test -p genossi_service_impl --lib repayment_phase` | 26 passed; 0 failed | ✓ PASS |
| Service-Impl Tests komplett | `cargo test -p genossi_service_impl --lib` | (gemäss Kontext) 270 passed | ✓ PASS |
| REST Smoke Tests | `cargo test -p genossi_rest --lib repayment_entry` | 3 passed | ✓ PASS |
| REST-Types Tests | `cargo test -p genossi_rest_types repayment_entry` | 7 passed | ✓ PASS |
| E2E komplett | `cargo test --test e2e_tests --features mock_auth` | **275 passed; 0 failed** | ✓ PASS |
| 5 neue IN-04 Regressionstests einzeln | `cargo test --test e2e_tests --features mock_auth -- <5 names>` | 5 passed | ✓ PASS |
| Audit-Disziplin RepaymentEntry | `grep -c "self\.repayment_entry_dao\.update(" repayment_entry.rs` | 0 (alle Writes via audited_*! Macros) | ✓ PASS |
| Audit-Disziplin RepaymentPhase | `grep -cE "self\.repayment_phase_dao\.(create\|update)\(" repayment_phase.rs` | 0 | ✓ PASS |
| Acceptance 08-07: updated.push(&refreshed) >= 1, (&entity) == 0 | grep | 1 / 0 | ✓ PASS |
| Acceptance 08-08: Ok(RepaymentPhase::from(&refreshed)) >= 4 | grep | 4 (zusätzlich 1× &entity legitim in read-only get_repayment_phase) | ✓ PASS |
| Acceptance 08-09: ok_or(EntityNotFound(*entry_id)) >= 1, conflict_body("entry not found") == 0 | grep | 1 / 0 | ✓ PASS |
| OpenAPI 404 für /batch-status dokumentiert | grep "missing or soft-deleted" rest/repayment_entry.rs | 1 | ✓ PASS |
| BatchFailureResponse-Doc grenzt 409-Scope ab | grep "NOT used for: missing or soft-deleted" rest_types/lib.rs | 1 | ✓ PASS |
| BL-01 Re-Read None Mapping (InternalError) | grep "BL-01 Fix" beide Service-Files | 6 (1× update_entry, 1× batch_toggle, 4× Phase-Lifecycle) | ✓ PASS |
| BL-01 Negativtests | `test_update_repayment_entry_rereads_none_yields_internal_error` + `test_update_repayment_phase_rereads_none_yields_internal_error` | beide vorhanden + grün | ✓ PASS |

### Requirements Coverage

Phase 8 Plan-Frontmatter `requirements` aggregiert über alle 10 Pläne (08-01..08-10):
ENTR-01, ENTR-02, ENTR-03, ENTR-04, ENTR-05, ENTR-06, PHAS-02, PHAS-03.
REQUIREMENTS.md Z. 13-14, 20-25, 89-90, 93-98 mappt exakt diese 8 IDs auf Phase 8.

| Requirement | Source Plan(s) | Description | Status | Evidence |
|-------------|----------------|-------------|--------|----------|
| ENTR-01 | 08-01, 08-02, 08-04, 08-06 | Auto-Fill beim Phase-Öffnen; current_shares-Snapshot | ✓ SATISFIED | Migration + Auto-Fill-Block + E2E `test_open_phase_triggers_auto_fill` |
| ENTR-02 | 08-03, 08-05, 08-06, 08-07, 08-10 | Manueller Add via REST | ✓ SATISFIED (CR-01-Quality-Gap geschlossen) | POST-Handler + E2E + CR-01 Re-Read-Fix + IN-04 Regression-Test verifiziert E2E |
| ENTR-03 | 08-01, 08-03 | Mehrere Einträge pro Mitglied+Phase | ✓ SATISFIED | Migration ohne UNIQUE-Constraint |
| ENTR-04 | 08-03, 08-05, 08-06 | share_count-Edit nur in {Open, Contacted} | ✓ SATISFIED | Unit-Test + Doppel-Guard in update_repayment_entry |
| ENTR-05 | 08-03, 08-05, 08-06 | Soft-Delete nur wenn Status != PaidOut | ✓ SATISFIED | Pre-Check vor audited_delete!; E2E + Unit-Tests |
| ENTR-06 | 08-03, 08-05, 08-06, 08-07, 08-09, 08-10 | Multi-select Status-Toggle (offen↔angeschrieben) | ✓ SATISFIED (CR-01 + CR-02 Quality-Gaps geschlossen) | Batch-Endpoint + audited_update! in 1 Tx + CR-01 Re-Read + CR-02 404-Mapping + 2 IN-04 Regression-Tests |
| PHAS-02 | 08-04, 08-05, 08-06, 08-08, 08-10 | Open-Phase-Auto-Fill | ✓ SATISFIED (CR-01-Quality-Gap geschlossen) | Auto-Fill in derselben Tx (unverändert) + CR-01 Re-Read der Phase-Entity in `open_repayment_phase` (NACH /PHAS-02, VOR commit) + IN-04 `test_open_phase_response_version_usable_for_followup_update` |
| PHAS-03 | 08-04, 08-05, 08-06, 08-08 | Close blockt bei pending entries | ✓ SATISFIED (CR-01-Quality-Gap geschlossen) | Pending-Validation-Block (unverändert) + CR-01 Re-Read in `close_repayment_phase` |

Keine orphaned Requirements: alle in REQUIREMENTS.md auf Phase 8 gemappten IDs sind in den Plan-Frontmatter-`requirements`-Feldern reflektiert.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Status |
|------|------|---------|----------|--------|
| `genossi_service_impl/src/repayment_entry.rs` | 266, 444 (vor Fix) | Service returnt entity vor DAO-update (stale version) | 🛑 Blocker (CR-01) | ✓ RESOLVED — Re-Read in Z. 265-290 + Z. 482-507 |
| `genossi_service_impl/src/repayment_phase.rs` | 140, 221, 355, 459 (vor Fix) | Gleiche stale-version-Bug-Klasse | 🛑 Blocker (CR-01) | ✓ RESOLVED — 4× Re-Read (Z. 139-160, 244-262, 397-417, 525-543) |
| `genossi_service_impl/src/repayment_entry.rs` | 416 (vor Fix) | "entry not found" als Conflict (statt EntityNotFound) | 🛑 Blocker (CR-02) | ✓ RESOLVED — Z. 441-453 mappt auf EntityNotFound |
| `genossi_rest/src/repayment_entry.rs` | 260-265 (vor Fix) | OpenAPI für /batch-status listete 404 nicht | ⚠️ Warning (CR-02-Symptom) | ✓ RESOLVED — Z. 264-265 dokumentiert 404 + 409 mit Cross-References |
| `genossi_service_impl/src/repayment_entry.rs` | 275, 489 (post-CR-01-Fix) | Re-Read None → EntityNotFound (→ HTTP 404) statt InternalError (→ HTTP 500) | 🛑 Blocker (BL-01) | ✓ RESOLVED — Beide Stellen + 4× Phase-Lifecycle mappen auf InternalError |
| `genossi_service_impl/src/repayment_phase.rs` | 148, 239, 387, 499 (post-CR-01-Fix) | Gleiche BL-01-Bug-Klasse | 🛑 Blocker (BL-01) | ✓ RESOLVED — Z. 155-160, 255-260, 412-417, 534-539 mappen auf InternalError |
| `genossi_bin/tests/e2e_tests.rs` | 11334-11421 (vor Fix) | Kein 2nd-PUT-Test mit version aus 1. PUT | 🛑 Blocker (IN-04) | ✓ RESOLVED — 5 neue Tests am Datei-Ende (Z. 11677, 11751, 11814, 11856, 11908) |
| `genossi_service_impl/src/repayment_entry.rs` | (existierend, nicht Teil dieser Phase) | Dead-code-zweiter target_status-Match (WR-02 aus Initial-Review) | ℹ️ Info | Nicht Scope dieser Verifikation; harmlos |
| `genossi_service_impl/src/repayment_phase.rs` | (existierend) | Redundanter `.filter(|e| e.deleted.is_none())` (WR-03 aus Initial-Review) | ℹ️ Info | Nicht Scope; harmlos |
| `genossi_dao_impl_sqlite/src/repayment_entry.rs` | (existierend) | `as i64`-Widening ohne debug_assert (WR-04) | ℹ️ Info | Nicht Scope; harmlos |
| `genossi_rest/src/repayment_entry.rs` | 78 | OpenAPI 404-Description "Member or Phase not found" passt nicht zur Impl (WR-05 aus REVIEW.md) | ⚠️ Warning | Bekannt aus REVIEW.md; nicht Gap-Closure-Scope dieser Re-Verifikation (Code-Review-Warning, kein Blocker) |
| `genossi_service_impl/src/repayment_entry.rs` | ~228 | `update_repayment_entry` liefert 404 mit member_id, wenn Member soft-deleted (WR-04 aus REVIEW.md) | ⚠️ Warning | Bekannt; nicht Gap-Closure-Scope; semantisches Refinement-Item für später |

### Behavioral Test Coverage (E2E)

| Scenario | Test | Status |
|----------|------|--------|
| 2. PUT auf RepaymentEntry mit Response-Version | `test_update_entry_followup_put_uses_response_version_returns_200` | ✓ PASS (200, `version_after_put1 != create_version`) |
| Einzel-PUT nach Batch-Toggle mit Response-Version | `test_batch_toggle_followup_put_uses_response_versions` | ✓ PASS (200) |
| PUT update_phase mit `open.version` | `test_open_phase_response_version_usable_for_followup_update` | ✓ PASS (200, share_value=13000 persisted) |
| 2. PUT update_phase mit 1.-PUT-Response-Version | `test_update_phase_response_version_usable_for_followup_update` | ✓ PASS (200, share_value=15000 persisted) |
| Batch mit unbekannter UUID → 404 | `test_batch_toggle_with_unknown_entry_id_returns_404` | ✓ PASS (StatusCode::NOT_FOUND) |

### Human Verification Required

Keine. Die früheren beiden Human-Verification-Items (manueller curl-Test für Folge-PUT, Multi-Tab-Stale-ID) sind durch die 5 neuen E2E-Regressionstests automatisiert abgedeckt:

- `test_update_entry_followup_put_uses_response_version_returns_200` deckt das curl-Szenario ab (mit explizitem `assert_ne!` zwischen create-Version und post-PUT-Version + 200-Assert).
- `test_batch_toggle_with_unknown_entry_id_returns_404` deckt das Multi-Tab-Stale-ID-Szenario ab (Mixed-Validity-Batch erzeugt durch eine `Uuid::new_v4()` einen "nicht-existenten" Entry-ID).

### Gaps Summary

**Keine Gaps verbleibend.** Phase 8 liefert vollständig alle 5 ROADMAP-Success-Criteria und alle 8 Requirements (ENTR-01..06 + PHAS-02 + PHAS-03). Alle vier Gap-Closure-Pläne (08-07/08/09/10 + BL-01-Hotfix) sind im Codebase nachweislich umgesetzt:

1. **CR-01 (stale version response) — vollständig geschlossen:** Re-Read-Pattern in 6 Service-Methoden (RepaymentEntry: update + batch_toggle; RepaymentPhase: create + update + open + close). Pattern 1:1 aus MemberServiceImpl::update (member.rs:343-348) übernommen. Audit-Disziplin (`grep -c "self\.repayment_*_dao\.(create\|update)("` == 0 in beiden Files) bleibt intakt.

2. **CR-02 (404 vs 409 im Batch-Toggle) — vollständig geschlossen:** Service-Layer mappt NotFound auf `ServiceError::EntityNotFound` (aggregat-konsistent mit get/update/delete); OpenAPI für POST `/batch-status` listet 404 explizit mit Cross-Reference zu 409; BatchFailureResponse-Doc-Comment grenzt 409-Scope explizit ab.

3. **IN-04 (Test-Coverage-Lücke) — vollständig geschlossen:** 5 E2E-Regressionstests am Datei-Ende von e2e_tests.rs verifizieren positiv: 4× CR-01-Folge-PUT-mit-Response-Version + 1× CR-02-NotFound-im-Batch. Vor 08-07/08-Fix wären die 4 CR-01-Tests rot (409 statt 200); vor 08-09-Fix wäre der CR-02-Test rot (409 statt 404). Test-Coverage-Lücke ist damit zementiert geschlossen.

4. **BL-01 (Re-Read-None-Mapping) — vollständig geschlossen:** Alle 6 Re-Read-Sites mappen die strukturell-unmögliche None-Verzweigung auf `ServiceError::InternalError` (→ HTTP 500), nicht auf `EntityNotFound` (→ HTTP 404). 2 Negativtests via mockall-Sequence verifizieren das Verhalten (`test_update_repayment_entry_rereads_none_yields_internal_error` + `test_update_repayment_phase_rereads_none_yields_internal_error`).

**Test-Status:** 23 RepaymentEntry-Tests + 26 RepaymentPhase-Tests + 3 REST-Smoke-Tests + 7 REST-Types-Tests + **275 E2E-Tests** — alle grün; 0 failed.

**Audit-Disziplin:** Sauber. Kein direkter DAO-write-Call außerhalb der `audited_*!`-Macros.

**Optimistic-Locking-Vertrag:** End-to-end positiv abgesichert. Folge-PUTs mit Response-Versionen liefern 200 (nicht 409).

**Aggregat-Konsistenz:** Hergestellt. Alle Methoden im RepaymentEntry-Aggregat (get/update/delete/batch_toggle) returnen 404 für missing/soft-deleted Entry-IDs; 409 ist reserviert für echte Domain-Konflikte.

**REST-API-Vertrag:** OpenAPI dokumentiert 404 und 409 für /batch-status getrennt. Frontend-generated Clients können Stale-ID (404) vs Domain-Konflikt (409) typgerecht unterscheiden.

**Empfehlung:** Phase 8 ist verifiziert und ready für Phase 9 (PayoutCascade). Die in der initialen REVIEW.md identifizierten WR-04 (Member-soft-delete liefert 404 statt 409) und WR-05 (`create_repayment_entry` Phase→409 vs Member→404 Inkonsistenz) sind Code-Quality-Refinements ohne BLOCKER-Charakter — sie können in einer späteren Hardening-Iteration adressiert werden, ohne Phase 9 zu blockieren.

---

*Re-Verified: 2026-05-31T22:30:00Z*
*Verifier: Claude (gsd-verifier)*
*Previous Verification: 2026-05-31T18:00:00Z (gaps_found — 2 BLOCKER + 1 Test-Coverage-Lücke + 1 follow-up BL-01)*
