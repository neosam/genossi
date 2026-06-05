---
phase: 16-service-rest-teil-rueckgabe-auto-anlegen-phase
verified: 2026-06-05T10:00:00Z
status: human_needed
score: 9/9 must-haves verified
overrides_applied: 0
human_verification:
  - test: "Closed-Phase-Status-Guard: Lege eine RepaymentPhase an, oeffne sie (POST /api/repayment-phase/{id}/open), schliesse sie (POST /api/repayment-phase/{id}/close), dann ruf POST /api/members/{id}/partial-repayment mit einem Datum auf, das auf diese Phase zielt."
    expected: "HTTP 409 Conflict — Phase ist geschlossen, kein neuer Entry erlaubt. Alternativ falls Preparation absichtlich erlaubt ist: nur Closed soll 409 geben."
    why_human: "CR-01 aus 16-REVIEW.md identifiziert einen fehlenden Status-Guard in partial_repayment: Die Implementierung in membership_adjust.rs:344-348 filtert beim Phase-Lookup nicht nach Status. Eine geschlossene Phase wird stillschweigend wiederverwendet. Kein automatisierbarer Test in der aktuellen E2E-Suite deckt diesen Pfad ab."
---

# Phase 16: Teil-Rückgabe Auto-Anlegen — Verifikationsbericht

**Phase-Ziel:** Multi-Datensatz-Operation (RepaymentEntry-Insert + ggf. RepaymentPhase-Auto-Create). Auto-Fill-Skip-Pattern als Doppelbuchungs-Prävention.
**Verifiziert:** 2026-06-05T10:00:00Z
**Status:** human_needed
**Re-Verifikation:** Nein — initiale Verifikation

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidenz |
|---|-------|--------|---------|
| 1 | `MembershipAdjustService::partial_repayment(member_id, n, willensbekundung_date, context)` existiert und erzeugt RepaymentEntry in Zielphase via `compute_effective_date` | VERIFIED | `genossi_service/src/membership_adjust.rs:65-72` — Trait-Methode vorhanden. `genossi_service_impl/src/membership_adjust.rs:283-454` — Vollimplementierung mit `compute_effective_date` (Step 8) und `audited_create!(repayment_entry_dao, ...)` (Step 12). 10 Unittest-Tests bestehen. |
| 2 | Service-Layer-Sum-Check: `sum(open_entries.share_count) + n <= member.current_shares` vor Insert; HTTP 400 bei Verstoß | VERIFIED | `membership_adjust.rs:403-418` — Filter `status != PaidOut`, Summenbildung i32, Vergleich mit `member_entity.current_shares`, `ServiceError::ValidationError` bei Überschreitung. E2E-Test `test_partial_repayment_sum_check_block_400` bestanden (HTTP 400, Body enthält "sum of open repayments"). |
| 3 | Auto-Create Zielphase: wenn keine Phase für berechnetes fiscal_year existiert, wird eine in Status `Open` mit `share_value` aus Vorgänger oder `DEFAULT_SHARE_VALUE_CENT=10000` angelegt | VERIFIED | `membership_adjust.rs:353-393` — None-Branch konstruiert `RepaymentPhaseEntity { status: Open, opened_at: Some(now) }`. share_value aus `all_phases.first().map(|p| p.share_value).unwrap_or(DEFAULT_SHARE_VALUE_CENT)`. `audited_create!(repayment_phase_dao, ..., REPAYMENT_PHASE_CREATE_PROCESS, ...)`. E2E-Tests H2 (auto-create, `phase.status=="Open"`) und Default-share-value (10000) bestanden. |
| 4 | Auto-Fill-Skip-Pattern in `open_repayment_phase`: überspringt Member mit existierendem Entry in Zielphase | VERIFIED | `repayment_phase.rs:368-396` — Erste Aktion im `for member in targets`-Loop: `find_by_member_and_phase(member.id, id, tx.clone())`, `continue` bei `!is_empty()`. Kommentar referenziert D-16-03, PART-04, PITFALLS-Kat-1. Unit-Test `test_open_repayment_phase_skips_members_with_existing_entry` verifiziert `create().times(1)`. E2E-Test `test_partial_repayment_auto_fill_skip_after_v12` bestanden. |
| 5 | E2E-Test H1-Happy-Path | VERIFIED | `test_partial_repayment_happy_path_h1` — bestanden. 200, `entry.share_count_to_pay_out=1`, `phase=null` (bestehende Phase wiederverwendet). |
| 6 | E2E-Test H2-with-Auto-Create | VERIFIED | `test_partial_repayment_happy_path_h2_with_auto_create_phase` — bestanden. 200, `phase` nicht null, `phase.fiscal_year=today+1`, `phase.status=="Open"`. |
| 7 | E2E-Test Sum-Check-Block 400 | VERIFIED | `test_partial_repayment_sum_check_block_400` — bestanden. Body enthält "sum of open repayments". |
| 8 | E2E-Test Auto-Fill-Skip | VERIFIED | `test_partial_repayment_auto_fill_skip_after_v12` — bestanden. Genau 1 Entry für Member in Phase nach partial_repayment + phase open. |
| 9 | E2E-Test Voll-Rückgabe-Block 400 / Cancelled-Member 409 / Audit-Chain / Default-Share-Value | VERIFIED | Alle 4 weiteren E2E-Tests (`full_return_block_400`, `cancelled_member_block_409`, `audit_chain_verify`, `auto_creates_phase_with_default_share_value`) bestanden. 17 gesamt, 0 fehlgeschlagen. |

**Score:** 9/9 Truths verifiziert

### Deferred Items

Keine — alle Phase-16-Must-Haves sind in dieser Phase adressiert.

### Required Artifacts

| Artifact | Erwartet | Status | Details |
|----------|----------|--------|---------|
| `genossi_service/src/membership_adjust.rs` | Trait mit `partial_repayment` | VERIFIED | Zeile 65-72, `shares: i32`, Rückgabe `Result<(Member, RepaymentEntry, Option<RepaymentPhase>), ServiceError>`, PART-06 dokumentiert |
| `genossi_rest_types/src/lib.rs` | `PartialRepaymentRequestTO` + `PartialRepaymentResponseTO` | VERIFIED | Zeile 548 und 569. `shares: i32`, `willensbekundung_date: time::Date` mit `iso8601_date_required`, `phase: Option<RepaymentPhaseTO>` mit `skip_serializing_if` |
| `genossi_service_impl/src/membership_adjust.rs` | Konstanten + Validator + 10 Tests + Impl | VERIFIED | `PARTIAL_REPAYMENT_PROCESS`, `DEFAULT_SHARE_VALUE_CENT=10000`, `validate_partial_repayment_shares`, 10 Unittests, vollständige `partial_repayment`-Implementierung |
| `genossi_service_impl/src/repayment_phase.rs` | Skip-Pattern in Auto-Fill-Loop | VERIFIED | Zeile 368-396, D-16-03/PART-04/PITFALLS-Kat-1 referenziert, Unit-Test vorhanden |
| `genossi_rest/src/membership_adjust.rs` | REST-Handler + ApiDoc | VERIFIED | `pub async fn partial_repayment`, Zeile 129-184, OpenAPI-Annotationen 200/400/401/404/409 |
| `genossi_rest/src/member.rs` | Sub-Route vor `/{id}` | VERIFIED | Zeile 75: `"/{id}/partial-repayment"` vor Zeile 79: `"/{id}"` |
| `genossi_bin/src/lib.rs` | DI-Wiring RepaymentPhaseDao + RepaymentEntryDao | VERIFIED | Zeile 503-504 (Deps impl), Zeile 733-734 (Arc::new-Deklarationen), Zeile 741 (Construction) |
| `genossi_bin/tests/membership_adjust_e2e.rs` | 8 E2E-Tests | VERIFIED | Alle 8 Testfunktionen vorhanden und bestanden |

### Key Link Verification

| Von | Zu | Via | Status | Details |
|-----|----|-----|--------|---------|
| `membership_adjust.rs::partial_repayment` | `repayment_entry_dao::find_by_member_and_phase` | Sum-Check vor audited_create! | WIRED | Zeile 398-401 |
| `membership_adjust.rs::partial_repayment` | `audited_create!(repayment_entry_dao, PARTIAL_REPAYMENT_PROCESS)` | Entry-Erstellung | WIRED | Zeile 432-439 |
| `membership_adjust.rs::partial_repayment (auto-create branch)` | `audited_create!(repayment_phase_dao, "repayment-phase.create")` | Inlined Phase-Create | WIRED | Zeile 382-389 |
| `member.rs::generate_route` | `membership_adjust::partial_repayment` | `.route("/{id}/partial-repayment", post(...))` | WIRED | Zeile 73-77, VOR `/{id}` Catch-All |
| `genossi_bin/src/lib.rs::RestStateImpl::new` | `MembershipAdjustServiceImpl` construction | `repayment_phase_dao: repayment_phase_dao.clone()` | WIRED | Zeile 733-754 |
| `repayment_phase.rs::open_repayment_phase` | `repayment_entry_dao::find_by_member_and_phase` | Per-Member Skip-Check | WIRED | Zeile 389-395 |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Quelle | Echte Daten | Status |
|----------|--------------|--------|-------------|--------|
| `partial_repayment` REST-Handler | `(member, entry, phase)` | `membership_adjust_service().partial_repayment(...)` | DAO-Queries via audited_create! + find_by_member_and_phase + member_dao.find_by_id | FLOWING |
| `open_repayment_phase` Loop | `existing_entries` | `repayment_entry_dao.find_by_member_and_phase(member.id, id, tx)` | SQLite-Query (Phase 14 Implementierung) | FLOWING |

### Behavioral Spot-Checks

| Verhalten | Ergebnis | Status |
|-----------|---------|--------|
| `cargo build --workspace --features mock_auth` | exit 0 | PASS |
| `cargo test -p genossi_service_impl --lib test_partial_repayment` | 10 passed, 0 failed | PASS |
| `cargo test -p genossi_service_impl --lib validate_partial_repayment_shares` | 7 passed, 0 failed | PASS |
| `cargo test --test membership_adjust_e2e --features mock_auth` | 17 passed, 2 ignored, 0 failed | PASS |

### Requirements Coverage

| Requirement | Plan | Beschreibung | Status | Evidenz |
|-------------|------|-------------|--------|---------|
| PART-01 | 01, 02, 04 | Vorstand kann Teil-Rückgabe mit Anteils-Anzahl n auslösen | SATISFIED | REST-Endpoint `POST /api/members/{id}/partial-repayment` existiert und ist vollständig verdrahtet |
| PART-02 | 02, 04 | System berechnet H1/H2-Stichtag (Ziel-fiscal_year) | SATISFIED | `compute_effective_date` in Step 8 der Implementierung; E2E-Tests H1/H2 bestanden |
| PART-03 | 02, 04 | System erzeugt RepaymentEntry in Zielphase mit share_count_to_pay_out=n, Status Open | SATISFIED | `audited_create!(repayment_entry_dao, ..., PARTIAL_REPAYMENT_PROCESS)`, entry.status=Open; E2E verifiziert |
| PART-04 | 03 | System validiert Sum-Check; Auto-Fill-Skip verhindert Duplikate | SATISFIED | Skip-Pattern in `repayment_phase.rs:368-396`; E2E `test_partial_repayment_auto_fill_skip_after_v12` bestanden |
| PART-05 | 02, 04 | System legt Ziel-RepaymentPhase automatisch an wenn nicht vorhanden | SATISFIED | Auto-Create-Branch in `membership_adjust.rs:356-393`; E2E H2-Auto-Create und Default-Share-Value bestanden |
| PART-06 | 01, 02, 04 | System erzeugt KEINE MemberAction und reduziert NICHT current_shares direkt | SATISFIED | grep-Verifikation: keine `member_dao.update`/`audited_update!(member_dao)` in partial_repayment; AUDT-01 Grep-Gate: 0 direkte DAO-Writes außerhalb audited_create! |

### Anti-Patterns Found

| Datei | Zeile | Muster | Schweregrad | Auswirkung |
|-------|-------|--------|-------------|------------|
| `genossi_service_impl/src/membership_adjust.rs` | 344-348 | Phase-Lookup ohne Status-Filter — geschlossene Phase wird stillschweigend wiederverwendet | Warning (CR-01 aus REVIEW.md — advisory finding) | Entry in geschlossener Phase widerspricht D-11.1 Lifecycle-Invariante. Kein Blocker für Phase-Ziel da kein E2E-Test diesen Pfad stimuliert, aber architektonisches Risiko. |
| `genossi_service_impl/src/membership_adjust.rs` | 295-303 | `current_user_id()` vor `check_permission()` — möglicher Side-Channel | Warning (CR-02 aus REVIEW.md — advisory finding) | Konsistentes Pattern aus Phase 15 übernommen. Kein Authentication-Bypass, aber forensische Lücke bei SYSTEM-Fallback. |
| `genossi_service_impl/src/membership_adjust.rs` | 319-322 | PII in Conflict-Message: `exit_date={:?}` | Warning (WR-02 aus REVIEW.md) | Exit-Datum im HTTP-409-Body. Admin-only-Endpoint, begrenzte Exposition. |

**Hinweis:** Die REVIEW.md-Findings sind advisory (nicht blockierend für das Phase-Ziel laut REVIEW.md-Klassifizierung). CR-01 (Closed-Phase ohne Status-Guard) ist das einzige Finding, das eine menschliche Verifikation erfordert — daher Status `human_needed`.

### Human Verification Required

**1. Closed-Phase-Status-Guard (CR-01 aus 16-REVIEW.md)**

**Test:** Führe folgende HTTP-Sequenz gegen einen laufenden Server aus:
1. `POST /api/repayment-phase` — Phase für FY 2026 anlegen (gibt Preparation zurück)
2. `POST /api/repayment-phase/{phase_id}/open` — Phase öffnen
3. `POST /api/repayment-phase/{phase_id}/close` — Phase schließen
4. Mitglied anlegen mit `current_shares=3`
5. `POST /api/members/{member_id}/partial-repayment` mit `willensbekundung_date="2026-03-15"` (H1 → FY 2026), `shares=1`

**Expected:** HTTP 409 Conflict mit Meldung, dass die Phase für fiscal_year 2026 geschlossen ist. Alternativ bei Absicht (Preparation erlaubt): nur `Closed`-Status soll abgelehnt werden.

**Warum Human:** Die Service-Implementierung in `membership_adjust.rs:344-348` enthält keinen Status-Filter beim Phase-Lookup. Die aktuelle E2E-Testsuite (`membership_adjust_e2e.rs`) testet diesen Closed-Phase-Pfad nicht. CR-01 aus der Code-Review identifiziert dies als Blocker-Finding — aber die Verifikation, ob das Verhalten intentional ist (Preparation-Phasen erlauben, Closed-Phasen ablehnen), erfordert eine Domänen-Entscheidung durch den Entwickler.

### Gaps Summary

Keine strukturellen Gaps bezüglich des Phase-16-Ziels. Alle 9 Truths sind VERIFIED, alle 8 E2E-Tests bestehen, alle 6 Requirement-IDs (PART-01..06) sind durch Code-Evidence gedeckt.

Der Status `human_needed` ergibt sich ausschließlich aus CR-01 der advisory Code-Review: Die `partial_repayment`-Implementierung wiederverwendet eine Zielphase unabhängig von deren Status (Preparation, Open, **Closed**). Ob das gewollt ist oder eine Status-Prüfung ergänzt werden muss, ist eine Domänen-Entscheidung.

**Empfehlung:** Wenn Closed-Phasen durch `partial_repayment` nicht beschreibbar sein sollen, füge den folgenden Guard vor dem Auto-Create-Block in `membership_adjust.rs:350` ein:
```rust
if let Some(ref phase) = target_phase_existing {
    if phase.status == RepaymentPhaseStatus::Closed {
        return Err(ServiceError::Conflict(Arc::from(format!(
            "Phase for fiscal_year {} is already closed (D-11.1)",
            effective.fiscal_year
        ))));
    }
}
```
Ergänze dazu Unit-Test `test_partial_repayment_rejects_closed_phase` und E2E-Test.

---

_Verifiziert: 2026-06-05T10:00:00Z_
_Verifier: Claude (gsd-verifier)_
