---
phase: 16-service-rest-teil-rueckgabe-auto-anlegen-phase
verified: 2026-06-05T14:00:00Z
status: passed
score: 12/12 must-haves verified
overrides_applied: 0
re_verification:
  previous_status: gaps_found
  previous_score: 9/9 (CR-01 als human_needed eskaliert -> User-Entscheidung Option A -> gaps_found)
  gaps_closed:
    - "CR-01 Closed-Phase-Status-Guard in partial_repayment"
  gaps_remaining: []
  regressions: []
  new_truths_added:
    - "10: Closed-Phase wird mit ServiceError::Conflict (HTTP 409) abgelehnt, bevor sum-check oder audited_create laufen"
    - "11: Unit-Test test_partial_repayment_rejects_closed_phase pinnt expect_create().times(0) auf phase_dao + entry_dao und expect_find_by_member_and_phase().times(0) auf entry_dao"
    - "12: E2E-Test test_partial_repayment_closed_phase_returns_409 verifiziert die REST-Sequenz create -> /open -> /close -> partial-repayment -> HTTP 409 mit Body 'closed' + fiscal_year"
---

# Phase 16: Teil-Rückgabe Auto-Anlegen — Verifikationsbericht (Re-Verifikation nach Gap-Closure 16-05)

**Phase-Ziel:** Multi-Datensatz-Operation (RepaymentEntry-Insert + ggf. RepaymentPhase-Auto-Create). Auto-Fill-Skip-Pattern als Doppelbuchungs-Prävention.
**Verifiziert:** 2026-06-05T14:00:00Z
**Status:** passed
**Re-Verifikation:** Ja — nach Gap-Closure-Plan 16-05 (CR-01 geschlossen)

## Re-Verifikations-Übersicht

Vorherige Verifikation am 2026-06-05T10:00:00Z lieferte `gaps_found` (nach User-Entscheidung Option A am selben Tag). Einziger Gap: **CR-01 (Closed-Phase-Status-Guard fehlt in `partial_repayment`)**.

Plan 16-05 hat diesen Gap geschlossen durch drei Commits:

| Commit | Inhalt | Datei |
|--------|--------|-------|
| `87f97841` | `feat(16-05): reject Closed RepaymentPhase in partial_repayment with HTTP 409` | `genossi_service_impl/src/membership_adjust.rs` |
| `5b334cc9` | `test(16-05): add unit test test_partial_repayment_rejects_closed_phase` | `genossi_service_impl/src/membership_adjust.rs` |
| `4ec92404` | `test(16-05): add E2E test_partial_repayment_closed_phase_returns_409` | `genossi_bin/tests/membership_adjust_e2e.rs` |

Alle drei Commits via `jj log` verifiziert (siehe Step 0 Sanity-Check). Code-Read bestätigt Guard an erwarteter Position (`membership_adjust.rs:350-361`).

**Konsequenz:** Truth-Score erweitert sich von 9/9 auf 12/12 (3 neue Truths aus 16-05 hinzugefügt). Alle 12 Truths VERIFIED. Keine Regressions in den vorher VERIFIED-Truths. Status flippt von `gaps_found` auf `passed`.

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidenz |
|---|-------|--------|---------|
| 1 | `MembershipAdjustService::partial_repayment(member_id, n, willensbekundung_date, context)` existiert und erzeugt RepaymentEntry in Zielphase via `compute_effective_date` | VERIFIED | `genossi_service/src/membership_adjust.rs:65-72` — Trait-Methode vorhanden. `genossi_service_impl/src/membership_adjust.rs:283-454` — Vollimplementierung mit `compute_effective_date` und `audited_create!(repayment_entry_dao, ...)`. Unit-Tests bestehen. |
| 2 | Service-Layer-Sum-Check: `sum(open_entries.share_count) + n <= member.current_shares` vor Insert; HTTP 400 bei Verstoß | VERIFIED | `membership_adjust.rs:411-426` (verschoben um +8 Zeilen durch neuen Guard) — Filter `status != PaidOut`, Summenbildung i32, Vergleich mit `member_entity.current_shares`, `ServiceError::ValidationError` bei Überschreitung. E2E-Test `test_partial_repayment_sum_check_block_400` bestanden. |
| 3 | Auto-Create Zielphase: wenn keine Phase für berechnetes fiscal_year existiert, wird eine in Status `Open` mit `share_value` aus Vorgänger oder `DEFAULT_SHARE_VALUE_CENT=10000` angelegt | VERIFIED | `membership_adjust.rs:366-406` — None-Branch konstruiert `RepaymentPhaseEntity { status: Open, opened_at: Some(now) }`. share_value aus `all_phases.first().map(|p| p.share_value).unwrap_or(DEFAULT_SHARE_VALUE_CENT)`. `audited_create!(repayment_phase_dao, ..., REPAYMENT_PHASE_CREATE_PROCESS, ...)`. E2E-Tests H2 (auto-create) und Default-share-value (10000) bestanden. |
| 4 | Auto-Fill-Skip-Pattern in `open_repayment_phase`: überspringt Member mit existierendem Entry in Zielphase | VERIFIED | `repayment_phase.rs:368-396` — Erste Aktion im `for member in targets`-Loop: `find_by_member_and_phase(member.id, id, tx.clone())`, `continue` bei `!is_empty()`. Kommentar referenziert D-16-03, PART-04, PITFALLS-Kat-1. Unit-Test `test_open_repayment_phase_skips_members_with_existing_entry` verifiziert `create().times(1)`. E2E-Test `test_partial_repayment_auto_fill_skip_after_v12` bestanden. |
| 5 | E2E-Test H1-Happy-Path | VERIFIED | `test_partial_repayment_happy_path_h1` — bestanden. 200, `entry.share_count_to_pay_out=1`, `phase=null` (bestehende Phase wiederverwendet). |
| 6 | E2E-Test H2-with-Auto-Create | VERIFIED | `test_partial_repayment_happy_path_h2_with_auto_create_phase` — bestanden. 200, `phase` nicht null, `phase.fiscal_year=today+1`, `phase.status=="Open"`. |
| 7 | E2E-Test Sum-Check-Block 400 | VERIFIED | `test_partial_repayment_sum_check_block_400` — bestanden. Body enthält "sum of open repayments". |
| 8 | E2E-Test Auto-Fill-Skip | VERIFIED | `test_partial_repayment_auto_fill_skip_after_v12` — bestanden. Genau 1 Entry für Member in Phase nach partial_repayment + phase open. |
| 9 | E2E-Test Voll-Rückgabe-Block 400 / Cancelled-Member 409 / Audit-Chain / Default-Share-Value | VERIFIED | Alle 4 weiteren E2E-Tests (`full_return_block_400`, `cancelled_member_block_409`, `audit_chain_verify`, `auto_creates_phase_with_default_share_value`) bestanden. |
| 10 | **NEU (16-05):** Closed-Phase wird mit `ServiceError::Conflict` (HTTP 409) abgelehnt, BEVOR sum-check oder audited_create laufen | VERIFIED | `genossi_service_impl/src/membership_adjust.rs:350-361` — Guard sitzt zwischen Phase-Lookup (Z. 344-348) und Auto-Create-Match (Z. 366). `if let Some(ref existing) = target_phase_existing { if existing.status == RepaymentPhaseStatus::Closed { return Err(ServiceError::Conflict(...)) } }`. Conflict-Message: `"Phase for fiscal_year {} is closed (D-11.1)"`. Borrow-Pattern verhindert Move, `target_phase_existing` bleibt für Match nutzbar. |
| 11 | **NEU (16-05):** Unit-Test `test_partial_repayment_rejects_closed_phase` (mockall) pinnt Short-Circuit-Verhalten | VERIFIED | `genossi_service_impl/src/membership_adjust.rs:2097-2158` — `expect_create().times(0)` auf `repayment_phase_dao` (Z. 2119), `expect_create().times(0)` auf `repayment_entry_dao` (Z. 2124), `expect_find_by_member_and_phase().times(0)` auf `repayment_entry_dao` (Z. 2123). Assertiert `Err(ServiceError::Conflict(msg))` mit `msg.contains("closed")` und `msg.contains(target_fy)`. Spot-Check `cargo test -p genossi_service_impl --lib test_partial_repayment` → 11/11 passed. |
| 12 | **NEU (16-05):** E2E-Test `test_partial_repayment_closed_phase_returns_409` verifiziert REST-Sequenz Preparation → Open → Close → 409 | VERIFIED | `genossi_bin/tests/membership_adjust_e2e.rs:893-961` — `create_repayment_phase` (Preparation) → `POST /api/repayment-phase/{id}/open` → `POST /api/repayment-phase/{id}/close` → `POST /api/members/{id}/partial-repayment` → `assert_eq!(status, StatusCode::CONFLICT)` + `body_text.contains("closed")` + `body_text.contains(&target_fy.to_string())`. Spot-Check `cargo test --test membership_adjust_e2e` → 18 passed / 2 ignored (Phase-15-pre-existing-design) / 0 failed. |

**Score:** 12/12 Truths verifiziert (9 vorher + 3 neu aus 16-05)

### Deferred Items

Keine — alle Phase-16-Must-Haves sind in dieser Phase adressiert.

### Required Artifacts

| Artifact | Erwartet | Status | Details |
|----------|----------|--------|---------|
| `genossi_service/src/membership_adjust.rs` | Trait mit `partial_repayment` | VERIFIED | Trait-Methode vorhanden, PART-06 dokumentiert |
| `genossi_rest_types/src/lib.rs` | `PartialRepaymentRequestTO` + `PartialRepaymentResponseTO` | VERIFIED | Zeile 548 und 569, ISO8601-Date, `phase: Option<RepaymentPhaseTO>` mit `skip_serializing_if` |
| `genossi_service_impl/src/membership_adjust.rs` | Konstanten + Validator + 11 Tests + Impl + **Closed-Guard (16-05)** | VERIFIED | `PARTIAL_REPAYMENT_PROCESS`, `DEFAULT_SHARE_VALUE_CENT=10000`, `validate_partial_repayment_shares`, 11 partial_repayment-Unittests (10 alt + `test_partial_repayment_rejects_closed_phase`), Closed-Phase-Status-Guard Z. 350-361 |
| `genossi_service_impl/src/repayment_phase.rs` | Skip-Pattern in Auto-Fill-Loop | VERIFIED | Zeile 368-396, D-16-03/PART-04/PITFALLS-Kat-1 referenziert |
| `genossi_rest/src/membership_adjust.rs` | REST-Handler + ApiDoc | VERIFIED | `partial_repayment`-Handler, OpenAPI 200/400/401/404/409 |
| `genossi_rest/src/member.rs` | Sub-Route vor `/{id}` | VERIFIED | `/{id}/partial-repayment` vor `/{id}` Catch-All |
| `genossi_bin/src/lib.rs` | DI-Wiring RepaymentPhaseDao + RepaymentEntryDao | VERIFIED | Zeile 503-504, 733-734, 741 |
| `genossi_bin/tests/membership_adjust_e2e.rs` | E2E-Tests (8 alt + 1 neu aus 16-05) | VERIFIED | 9 `test_partial_repayment_*` Funktionen inkl. `test_partial_repayment_closed_phase_returns_409` (Z. 893-961) |

### Key Link Verification

| Von | Zu | Via | Status | Details |
|-----|----|-----|--------|---------|
| `membership_adjust.rs::partial_repayment` | `repayment_entry_dao::find_by_member_and_phase` | Sum-Check vor audited_create! | WIRED | Position seit Guard-Insert um +8 Zeilen verschoben (Z. 406-409) |
| `membership_adjust.rs::partial_repayment` | `audited_create!(repayment_entry_dao, PARTIAL_REPAYMENT_PROCESS)` | Entry-Erstellung | WIRED | Z. ~445 |
| `membership_adjust.rs::partial_repayment (auto-create branch)` | `audited_create!(repayment_phase_dao, "repayment-phase.create")` | Inlined Phase-Create | WIRED | Z. 395-402, nur erreichbar wenn `target_phase_existing.is_none()` |
| `membership_adjust.rs::partial_repayment (NEW Closed-Guard)` | `ServiceError::Conflict` → REST 409 | `if let Some(ref existing) = target_phase_existing { if existing.status == Closed { return Err(...) } }` | WIRED | Z. 350-361. Guard läuft NUR bei `Some(_)`, daher Auto-Create-Branch unaffected. ServiceError::Conflict → RestError::Conflict → HTTP 409 (via `genossi_rest/src/lib.rs`). |
| `member.rs::generate_route` | `membership_adjust::partial_repayment` | `.route("/{id}/partial-repayment", post(...))` | WIRED | VOR `/{id}` Catch-All |
| `genossi_bin/src/lib.rs::RestStateImpl::new` | `MembershipAdjustServiceImpl` construction | `repayment_phase_dao: repayment_phase_dao.clone()` | WIRED | Z. 733-754 |
| `repayment_phase.rs::open_repayment_phase` | `repayment_entry_dao::find_by_member_and_phase` | Per-Member Skip-Check | WIRED | Z. 389-395 |
| **NEU (16-05):** `test_partial_repayment_closed_phase_returns_409` | `POST /api/repayment-phase/{id}/close` + `POST /api/members/{id}/partial-repayment` | reqwest-Sequenz | WIRED | E2E-Test Z. 893-961 verbindet REST-Lifecycle-Endpunkte mit partial_repayment-Endpunkt |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Quelle | Echte Daten | Status |
|----------|--------------|--------|-------------|--------|
| `partial_repayment` REST-Handler | `(member, entry, phase)` | `membership_adjust_service().partial_repayment(...)` | DAO-Queries via audited_create! + find_by_member_and_phase + member_dao.find_by_id | FLOWING |
| `open_repayment_phase` Loop | `existing_entries` | `repayment_entry_dao.find_by_member_and_phase(member.id, id, tx)` | SQLite-Query (Phase 14 Implementierung) | FLOWING |
| **NEU (16-05):** Closed-Guard | `existing.status` | `all_phases.iter().find(|p| p.fiscal_year == effective.fiscal_year).cloned()` (Z. 344-348) | DAO-Query via `repayment_phase_dao.all(tx.clone())`; Status aus `RepaymentPhaseStatus`-Enum (Phase-14-Domain) | FLOWING |

### Behavioral Spot-Checks

| Verhalten | Ergebnis | Status |
|-----------|---------|--------|
| `cargo build --quiet` | exit 0 (vom Orchestrator vorab verifiziert) | PASS |
| `cargo test -p genossi_service_impl --lib test_partial_repayment` | 11 passed, 0 failed (vom Orchestrator vorab verifiziert) | PASS |
| `cargo test --test membership_adjust_e2e` | 18 passed / 2 ignored / 0 failed (vom Orchestrator vorab verifiziert) | PASS |
| `cargo test -p genossi_service_impl --lib` (gesamt) | 389/389 passed (vom Orchestrator vorab verifiziert) | PASS |

**Hinweis zu den 2 ignored E2E-Tests:** Vom Phase-15-Design getragen — `test_cancel_membership_permission_denied` und `test_increase_shares_permission_denied` sind unter `mock_auth` per `#[ignore]` ausgeschlossen, weil der mock_auth-Context-Extractor stets einen Admin-DEVUSER injiziert (siehe `genossi_bin/tests/membership_adjust_e2e.rs:13-23`). Service-Layer-Coverage existiert. KEIN Phase-16-Regression.

### Requirements Coverage

| Requirement | Plan | Beschreibung | Status | Evidenz |
|-------------|------|-------------|--------|---------|
| PART-01 | 01, 02, 04 | Vorstand kann Teil-Rückgabe mit Anteils-Anzahl n auslösen | SATISFIED | REST-Endpoint `POST /api/members/{id}/partial-repayment` existiert und ist vollständig verdrahtet |
| PART-02 | 02, 04 | System berechnet H1/H2-Stichtag (Ziel-fiscal_year) | SATISFIED | `compute_effective_date` in Service-Impl; E2E-Tests H1/H2 bestanden |
| PART-03 | 02, 04 | System erzeugt RepaymentEntry in Zielphase mit share_count_to_pay_out=n, Status Open | SATISFIED | `audited_create!(repayment_entry_dao, ..., PARTIAL_REPAYMENT_PROCESS)`, entry.status=Open; E2E verifiziert |
| PART-04 | 03, **05** | System validiert Sum-Check; Auto-Fill-Skip verhindert Duplikate; **(16-05) Closed-Phase-Lifecycle-Guard** | SATISFIED | Skip-Pattern in `repayment_phase.rs:368-396` (Plan 03); Closed-Phase-Guard in `membership_adjust.rs:350-361` (Plan 05) ergänzt Lifecycle-Sicherheit auf der inversen Seite (Closed statt Open-with-existing-entry). E2E `test_partial_repayment_auto_fill_skip_after_v12` + `test_partial_repayment_closed_phase_returns_409` bestanden. |
| PART-05 | 02, 04 | System legt Ziel-RepaymentPhase automatisch an wenn nicht vorhanden | SATISFIED | Auto-Create-Branch in `membership_adjust.rs:366-406`; nur bei `target_phase_existing.is_none()` aktiviert. E2E H2-Auto-Create und Default-Share-Value bestanden. |
| PART-06 | 01, 02, 04 | System erzeugt KEINE MemberAction und reduziert NICHT current_shares direkt | SATISFIED | grep-Verifikation: keine `member_dao.update`/`audited_update!(member_dao)` in partial_repayment; AUDT-01 Grep-Gate: 0 direkte DAO-Writes außerhalb audited_create! |

Alle 6 Phase-16-Requirements (PART-01..06) durch Code-Evidence gedeckt.

### Anti-Patterns Found

| Datei | Zeile | Muster | Schweregrad | Auswirkung |
|-------|-------|--------|-------------|------------|
| `genossi_service_impl/src/membership_adjust.rs` | 295-303 | `current_user_id()` vor `check_permission()` — möglicher Side-Channel | Warning (CR-02 aus REVIEW.md — explizit OUT-OF-SCOPE per workflow config, advisory finding nicht Teil dieser Phase) | Konsistentes Pattern aus Phase 15 übernommen. Kein Authentication-Bypass, aber forensische Lücke bei SYSTEM-Fallback. Vom User aus Scope von 16-05 ausgeschlossen. |
| `genossi_service_impl/src/membership_adjust.rs` | 319-322 | PII in Conflict-Message: `exit_date={:?}` | Warning (WR-02 aus REVIEW.md) | Exit-Datum im HTTP-409-Body. Admin-only-Endpoint, begrenzte Exposition. Nicht-blockierend. |

**Hinweis:** CR-02 ist explizit als separates Advisory-Finding in 16-REVIEW.md dokumentiert und vom User out-of-scope für Plan 16-05 markiert. Es bleibt als Carry-Forward für eine spätere Phase, blockiert aber das Phase-16-Ziel NICHT (Permission-Reordering ist eine Hardening-Verbesserung, keine Korrektheits-Bug am Phase-Ziel). Die übrigen WR-01, WR-03, WR-04, WR-05, WR-06 aus 16-REVIEW.md sind ebenfalls advisory.

### Human Verification Required

Keine. Die ursprüngliche Human-Verification-Item aus der Initial-Verifikation (Closed-Phase-Status-Guard) ist durch den deterministischen E2E-Test `test_partial_repayment_closed_phase_returns_409` (Z. 893-961) mechanisch reproduzierbar gemacht. 16-HUMAN-UAT.md ist auf `status: resolved` gesetzt.

### Gaps Summary

Keine offenen Gaps für das Phase-16-Ziel. Der ursprüngliche Gap (CR-01 Closed-Phase-Status-Guard) ist durch Plan 16-05 vollständig geschlossen:

- **Service-Layer:** Guard in `membership_adjust.rs:350-361` (Commit `87f97841`)
- **Unit-Test:** `test_partial_repayment_rejects_closed_phase` in `membership_adjust.rs:2097-2158` (Commit `5b334cc9`)
- **E2E-Test:** `test_partial_repayment_closed_phase_returns_409` in `membership_adjust_e2e.rs:893-961` (Commit `4ec92404`)

Der Re-Review (16-REVIEW.md, supersedes commits 62a9dbda + 87f97841) bestätigt: CR-01 geschlossen. Kein neuer BLOCKER durch die Änderung eingeführt. CR-02 bleibt als explizit out-of-scope-Carry-Forward bestehen und ist NICHT Teil dieser Verifikation.

---

_Verifiziert: 2026-06-05T14:00:00Z_
_Verifier: Claude (gsd-verifier)_
_Re-Verifikation nach Plan 16-05 (CR-01 Gap-Closure)_
