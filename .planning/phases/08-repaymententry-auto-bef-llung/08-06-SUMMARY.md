---
phase: 08-repaymententry-auto-bef-llung
plan: 06
subsystem: testing
tags: [rust, e2e, reqwest, sqlite, audit-hashchain, repayment-entry, integration-test]

requires:
  - phase: 08-repaymententry-auto-bef-llung
    plan: 04
    provides: "RepaymentPhaseServiceImpl::open_repayment_phase mit Auto-Fill der RepaymentEntries + close_repayment_phase mit Pending-Validation und strukturiertem 409-JSON-Body (pending_count + pending_member_numbers)"
  - phase: 08-repaymententry-auto-bef-llung
    plan: 05
    provides: "REST-Endpoints /api/repayment-entry (POST create / GET list?phase_id= / GET {id} / PUT {id} / DELETE {id} / POST /batch-status) + DI-Wiring + RepaymentEntryRestState-Trait-Bound auf start_test_server"

provides:
  - "15 E2E-Tests in genossi_bin/tests/e2e_tests.rs verifizieren Phase-8-Funktionalität end-to-end gegen real-laufenden HTTP-Server mit in-memory SQLite"
  - "2 Helper-Funktionen: create_member_with_exit_date (postet zuerst Member, dann Austritt-MemberAction damit recalc_dates exit_date setzt) + create_open_repayment_phase (Preparation + Open-Trigger)"
  - "Audit-Hashchain-Verifikation via /api/audit/verify nach komplettem Phase-8-Lifecycle (create-phase → open + auto-fill → batch-toggle → delete) bleibt valid"
  - "Plural-Member-Endpoint /api/members (W-01) als feststehende E2E-Konvention dokumentiert"
  - "Phase-7-Baseline 255 E2E-Tests bleiben grün → 270 total"

affects:
  - "09 (PAYO): mark_paid_out-Endpoint kann sich an /api/repayment-entry/{id}/mark-paid-out anhängen — die E2E-Patterns dieses Plans (Auto-Fill setup + Status-Verifikation + Audit-Chain) sind 1:1 als Vorlage nutzbar"
  - "12 (Frontend): kann die /api/repayment-entry-REST-Endpoints und strukturierte 409-Bodies (BatchFailureResponse, CloseConflictResponse) gegen die hier verifizierte HTTP-Surface entwickeln"
  - "Künftige Phasen, die Member mit exit_date für Tests brauchen: Vorlage create_member_with_exit_date zeigt die korrekte Konstruktion via Austritt-Action (NICHT via MemberTO.exit_date — wird durch recalc_dates() überschrieben)"

tech-stack:
  added: []
  patterns:
    - "E2E-Member-mit-exit_date-Pattern: nicht via MemberTO.exit_date (wird überschrieben), sondern via POST /api/members/{id}/actions mit ActionTypeTO::Austritt + effective_date — recalc_dates ist Single Source of Truth"
    - "Phase-Lifecycle-mit-Auto-Fill-Pattern: create_open_repayment_phase als Composite-Helper, der die Phase-7-create + Phase-7-open-Sequenz einkapselt — Phase-8-Auto-Fill läuft als Nebeneffekt im open"
    - "Audit-Chain-E2E-Verifikation als finale Sanity-Check-Test: /api/audit/verify nach Lifecycle-Sequenz bestätigt Hashchain-Integrität ohne Field-Inspection"

key-files:
  created: []
  modified:
    - "genossi_bin/tests/e2e_tests.rs (+680 LOC -24 LOC: imports erweitert um BatchStatusRequest/CreateRepaymentEntryRequest/RepaymentEntryStatusTO/RepaymentEntryTO/UpdateRepaymentEntryRequest; 2 Helper am Datei-Ende NACH Phase-7-Block; 15 E2E-Tests in einem Phase-08-Plan-06-Block)"

key-decisions:
  - "Helper create_member_with_exit_date musste ein 3-stufiges Setup (POST member → POST Austritt-action → GET member) statt 1-stufigem POST verwenden — Service-Konvention macht recalc_dates beim Member-Create zur Single Source of Truth für exit_date; MemberTO.exit_date wird ohne entsprechende Austritt-Action überschrieben (member_service.rs:288 + member_action.rs:160-169 compute_dates)."
  - "test_manual_add_entry_happy_path nutzt share_count_to_pay_out=1 statt =2: Member-Service setzt current_shares = shares_at_joining beim Create (member.rs:213-218); sample_member() hat shares_at_joining=1, also ist 1 das Maximum für den Validation-Pass (D-11.3)."
  - "test_close_phase_with_pending_entries_returns_409_with_member_numbers prüft auf Substring '42' (raw member_number) statt 'M-42' — Plan-04-Implementation gibt Mitgliedsnummern als plain .to_string() raus; Plan-Threat T-08-06-02 als low-risk mitigated."
  - "Audit-Chain-Test (Test 15) verifiziert nur verify.valid == true und broken_links == empty — keine spezifischen Hash-/Field-Diff-Assertions (das macht Phase-7-Plan-05 bereits umfangreich für RepaymentPhase). Phase-8-Test prüft nur dass die hinzugefügten RepaymentEntry-Lifecycle-Events die Chain nicht brechen."

patterns-established:
  - "E2E-Setup für Members mit echtem exit_date: 3-stufiger Helper (POST member + POST Austritt-Action + GET member) als Vorlage für Phase 9 (PAYO mark-paid-out muss ebenfalls Members mit exit_date setup'en)"
  - "Composite-Lifecycle-Helper-Pattern: create_open_repayment_phase als Wrapper um create_preparation_repayment_phase + open-Trigger — vereinfacht Test-Setup für alle Tests, die Auto-Fill voraussetzen"
  - "Phase-Open-Triggered-Side-Effect-E2E-Verification: kein direkter Service-DAO-Zugriff im Test, sondern Verifikation via nachfolgendem GET /api/repayment-entry?phase_id= — beweist dass die Side-Effects in der Tx tatsächlich committed wurden"

requirements-completed: [ENTR-01, ENTR-02, ENTR-03, ENTR-04, ENTR-05, ENTR-06, PHAS-02, PHAS-03]

duration: ~9min
completed: 2026-05-31
---

# Phase 08 Plan 06: RepaymentEntry + Auto-Befüllung E2E-Tests Summary

**15 E2E-Tests gegen real-laufenden HTTP-Server mit in-memory SQLite verifizieren Phase 8 end-to-end: Auto-Fill beim Phase-Open + 3 Edge-Cases (zero-members/no-exit-date/outside-FY), manueller Create + Validation (Phase-not-Open 409 + Range 400), Update-Edit-Matrix (Open↔Contacted + PaidOut-Reject 409), Soft-Delete, Batch-Toggle (Happy + PaidOut-Target 400), Close-Validation (409 mit pending_count + member_number sowie 0-Entry-Erlaubt), und Audit-Hashchain bleibt valid nach komplettem Phase-8-Lifecycle. 270 grüne E2E-Tests (Phase-7-Baseline 255 + 15 neue Phase-8).**

## Performance

- **Duration:** ~9 min
- **Started:** 2026-05-31T05:04:55Z
- **Completed:** 2026-05-31T05:14:07Z
- **Tasks:** 1/1 abgeschlossen
- **Files modified:** 1 (`genossi_bin/tests/e2e_tests.rs`)
- **Tests:** 15 neue E2E-Tests grün + 255 Phase-7-Baseline grün = 270 total

## Accomplishments

- **2 Helper-Funktionen** am Datei-Ende von `genossi_bin/tests/e2e_tests.rs`:
  - `create_member_with_exit_date(client, server, member_number, fiscal_year, current_shares)` — 3-stufig: POST Member → POST Austritt-MemberAction (mit `effective_date` im FY, 15. Juni) → GET Member zur Re-Load nach recalc_dates. Liefert MemberTO mit echtem `exit_date`.
  - `create_open_repayment_phase(client, server, fiscal_year, share_value)` — Wrapper um Phase-7-`create_preparation_repayment_phase` + POST `/api/repayment-phase/{id}/open` (triggert Auto-Fill).
- **15 E2E-Tests in einem Block NACH dem bestehenden Phase-7-Block:**
  - **Auto-Fill (PHAS-02 / ENTR-01) — 4 Tests:**
    1. `test_open_phase_triggers_auto_fill` — 3 Members (M1 cs=5/exit-FY, M2 cs=0/exit-FY, M3 cs=3/exit-FY) → Auto-Fill 2 Entries (M1 + M3); M2 wegen cs=0 geskippt (D-02).
    2. `test_open_phase_auto_fill_zero_members` — 0 Members → 0 Entries, Phase trotzdem Open (D-14).
    3. `test_open_phase_skips_member_without_exit_date` — 1 Member ohne exit_date → 0 Entries.
    4. `test_open_phase_skips_member_outside_fiscal_year` — 1 Member exit_date in 2027, Phase-FY 2026 → 0 Entries (D-01).
  - **Manueller Create (ENTR-02 / D-11) — 3 Tests:**
    5. `test_manual_add_entry_happy_path` — POST in Open-Phase, valide Felder → 201 + RepaymentEntryTO mit status=Open.
    6. `test_manual_add_entry_phase_not_open_returns_409` — POST in Preparation-Phase → 409 (D-11.1).
    7. `test_manual_add_entry_share_count_exceeds_returns_400` — share_count_to_pay_out=999 vs. current_shares=1 → 400 + Body contains "share_count_to_pay_out" (D-11.3).
  - **Update (ENTR-04/06 / D-05/D-06) — 2 Tests:**
    8. `test_update_entry_status_open_to_contacted_succeeds` — PUT status=Contacted → 200 + entry.status==Contacted.
    9. `test_update_entry_status_paid_out_returns_409` — PUT status=PaidOut → 409 (D-05: PaidOut nur via Phase-9 mark_paid_out).
  - **Delete (ENTR-05) — 1 Test:**
    10. `test_delete_entry_in_open_succeeds` — DELETE auf Open-Entry → 204 + nachfolgender GET 404 (soft-delete).
  - **Batch-Toggle (D-07/D-08) — 2 Tests:**
    11. `test_batch_toggle_happy_path` — 2 Auto-Fill-Entries → POST batch-status target=Contacted → 200 + alle Entries Contacted.
    12. `test_batch_toggle_paid_out_target_returns_400` — target_status=PaidOut → 400 (D-07).
  - **Close-Validation (PHAS-03 / D-14/D-15) — 2 Tests:**
    13. `test_close_phase_with_pending_entries_returns_409_with_member_numbers` — Phase mit 1 Open-Entry (Member-Nr. 42) → close → 409 + Body contains "pending_count" + "42" (D-15).
    14. `test_close_phase_with_zero_entries_succeeds` — Phase ohne Entries → close → 200 (D-14).
  - **Audit-Hashchain (cross-cutting) — 1 Test:**
    15. `test_audit_chain_intact_after_phase_8_lifecycle` — create-phase → open (Auto-Fill 2 Entries) → batch-toggle → delete-1 → /api/audit/verify → verify.valid==true + broken_links empty.
- **Imports** erweitert um `BatchStatusRequest`, `CreateRepaymentEntryRequest`, `RepaymentEntryStatusTO`, `RepaymentEntryTO`, `UpdateRepaymentEntryRequest`.
- **Phase-7-E2E-Suite bleibt komplett grün:** 255 Tests, 0 Regress.
- **Audit-Chain robust:** Test 15 verifiziert dass die N+1 audited_*-Calls (1 Phase-Open + 2 Auto-Fill audited_create + 2 batch audited_update + 1 audited_delete) die Hash-Chain nicht brechen.

## Task Commits

Atomarer Single-Commit (Plan hat nur 1 Task):

1. **Task 1: 15 E2E-Tests + 2 Helper** — `677eab1` (test, +680 LOC -24 LOC)

**Plan metadata:** *(folgt mit dem nächsten Commit)*

## Files Created/Modified

- **MOD** `genossi_bin/tests/e2e_tests.rs` (+680 LOC -24 LOC):
  - Imports-Block (Z. 12-19): `BatchStatusRequest`, `CreateRepaymentEntryRequest`, `RepaymentEntryStatusTO`, `RepaymentEntryTO`, `UpdateRepaymentEntryRequest` ergänzt — alphabetisch eingeordnet.
  - Datei-Ende (NACH Phase-7-Block, NACH Z. 11007): neuer Header-Kommentar `// Phase 08 Plan 06: RepaymentEntry + Auto-Befüllung — E2E tests` + 2 Helper-Funktionen + 15 `#[tokio::test]`-Funktionen, in der im Plan vorgegebenen Reihenfolge (Auto-Fill → Manual Create → Update → Delete → Batch → Close → Audit).
  - rustfmt 1.93 (gleiche Version wie Plan 04) angewendet — keine Verhaltensänderung, nur Code-Style.

## Decisions Made

Drei Klarstellungen kamen aus der Implementation hinzu, alle in Reaktion auf erste Test-Fehlschläge:

- **Helper `create_member_with_exit_date` ist 3-stufig statt 1-stufig:** Erste Helper-Implementation hat einfach `MemberTO.exit_date = Some(...)` im POST gesetzt. Das funktionierte nicht — Service ruft am Ende `recalc_dates()` (member.rs:288) auf, das `compute_dates()` (member_action.rs:160-169) ausführt; diese Funktion ermittelt `exit_date` ausschließlich aus `MemberAction::Austritt`/`Todesfall`-Actions, NICHT aus dem `MemberTO.exit_date`-Feld. **recalc_dates() ist die Single Source of Truth** — das gesendete `exit_date` wird zurückgesetzt auf `None`. Lösung: nach Member-Create eine `Austritt`-Action posten (mit `effective_date` im fiscal_year und `shares_change=0`, weil Austritt das so verlangt), dann den Member neu laden. Doc-Comment im Helper erklärt das Pattern für künftige Plans.
- **`share_count_to_pay_out=1` statt =2 im Happy-Path-Test:** Erste Test-Implementation nutzte `share_count_to_pay_out=2` (analog Plan-Beispiel). Service rejected das mit 400 "must be <= member current_shares (1), got 2". Root-Cause: Member-Service setzt beim Create `current_shares = shares_at_joining` (member.rs:213-218); sample_member() hat `shares_at_joining=1`, also wird `current_shares=1` persistiert. Der vom Test gesendete `current_shares=3` im MemberTO wird ignoriert. Fix: Test nutzt `share_count_to_pay_out=1` und ein doc-comment erklärt warum.
- **Close-Conflict-Body-Substring '42' statt 'M-42':** Plan-Threat T-08-06-02 hatte das Format-Drift-Risiko notiert. Plan-04-Implementation gibt Mitgliedsnummern als plain `to_string()` raus (z.B. "42"), nicht im Format "M-42". Test prüft auf das raw-Format; künftiges Frontend kann die Strings nach Bedarf prefixen.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 — Bug] Helper-Setup nutzte falsches Mittel zum exit_date-Setzen**

- **Found during:** Task 1, erster Testlauf (5 von 15 Tests rot, alle mit "auto-fill produces 0 entries")
- **Issue:** Helper `create_member_with_exit_date` hat ursprünglich `MemberTO.exit_date = Some(...)` im POST /api/members gesetzt. Aber der Member-Service ruft nach Create `recalc_dates()` auf, das `exit_date` aus `MemberAction::Austritt`-Actions ableitet — ohne entsprechende Action wird `exit_date` auf `None` zurückgesetzt. Daraus folgte: Auto-Fill-Filter (D-02: `exit_date IN fiscal_year`) hat alle Members ausgeschlossen → 0 Entries → 5 Tests rot.
- **Fix:** Helper auf 3-stufig umgestellt: POST Member → POST Austritt-MemberAction (`effective_date = exit_date`, `shares_change = 0`) → GET Member zur Re-Load. Doc-Comment im Helper erklärt die Service-Konvention für künftige Plans.
- **Files modified:** `genossi_bin/tests/e2e_tests.rs` (Helper `create_member_with_exit_date`)
- **Verification:** `cargo test --test e2e_tests --features mock_auth test_open_phase_triggers_auto_fill` grün; `test_audit_chain_intact_after_phase_8_lifecycle`, `test_batch_toggle_happy_path`, `test_close_phase_with_pending_entries_returns_409_with_member_numbers` ebenfalls grün.
- **Committed in:** `677eab1` (Task 1 commit)

**2. [Rule 1 — Bug] test_manual_add_entry_happy_path nutzte share_count > current_shares**

- **Found during:** Task 1, erster Testlauf (`test_manual_add_entry_happy_path` rot mit 400 "must be <= member current_shares (1), got 2")
- **Issue:** Test sendete `share_count_to_pay_out=2`, sample_member() hat `shares_at_joining=1`. Member-Service-Konvention (member.rs:213-218): `current_shares = shares_at_joining` beim Create — der `MemberTO.current_shares=3` wird ignoriert. Persistiert wird `current_shares=1`, also blockt die D-11.3-Validation den Create.
- **Fix:** Test auf `share_count_to_pay_out=1` umgestellt. Doc-Comment im Test erklärt die Service-Konvention für künftige Tests.
- **Files modified:** `genossi_bin/tests/e2e_tests.rs` (`test_manual_add_entry_happy_path`)
- **Verification:** `cargo test --test e2e_tests --features mock_auth test_manual_add_entry_happy_path` grün.
- **Committed in:** `677eab1` (Task 1 commit)

---

**Total deviations:** 2 auto-fixed (2 bugs in test setup, 0 in production code)
**Impact on plan:** Beide Fixes betrafen ausschließlich das Test-Setup; die Production-Implementation (Plans 01-05) blieb unverändert. Plan 04 hatte das exit_date-via-Austritt-Pattern in seinen Unit-Tests bereits korrekt mit `MemberEntity { exit_date: Some(...) }` direkt am DAO simuliert — die E2E-Schicht musste das Setup über den HTTP-Stack aufbauen, was die Service-Konvention `recalc_dates()` triggerte. Kein Scope-Creep, keine Architektur-Änderung.

## Issues Encountered

- **Erstes Test-Setup-Iteration:** Die zwei oben dokumentierten Bugs traten nacheinander auf — der zweite (`share_count > current_shares`) konnte erst diagnostiziert werden, nachdem der erste (`auto-fill produziert 0 Entries`) gefixt war. Diagnose-Pfad: assertion-Body um `let body_text = resp.text().await.unwrap()` erweitert, um die Service-Fehlermeldung zu sehen — der Output `"must be <= member current_shares (1), got 2"` zeigte direkt den Service-State.
- **Phase-7-Baseline `cargo test --test e2e_tests` vor dem Edit: 255 grün**, nach dem Edit: **270 grün**. Erster Run nach Edit: **265 grün, 5 rot** (vor den zwei Fixes). Finaler Run: alle 270 grün.

## User Setup Required

None — E2E-Tests laufen mit `cargo test --test e2e_tests --features mock_auth` automatisch. In-memory SQLite und random-Port-Binding (über Phase-7 etablierte test_server-Infrastruktur) sind selbstkonfigurierend.

## Next Phase Readiness

- **Phase 8 ist komplett verifikations-vollständig:** Alle 8 Requirements (ENTR-01..06 + PHAS-02 + PHAS-03) sind end-to-end über HTTP verifiziert. ROADMAP Phase-8 Success Criteria 1-5 (Migration via Plan 01-02, Auto-Fill via Plan 04 + Test 1-4, Manueller Create via Plan 05 + Test 5-7, Batch-Toggle via Plan 03 + Test 11-12, Close-Validation via Plan 04 + Test 13-14) sind E2E-belegt; SC#4 (Audit-Chain) via Test 15.
- **Plan 9 (PAYO mark_paid_out):** Test-Patterns aus Plan 06 sind 1:1 wiederverwendbar — `create_member_with_exit_date` + `create_open_repayment_phase` decken die Setup-Anforderungen ab. Phase 9 muss den neuen Endpoint `/api/repayment-entry/{id}/mark-paid-out` (oder vergleichbar) angreifen; PUT mit status=PaidOut bleibt nach D-05 weiterhin 409.
- **Phase 12 (Frontend):** REST-Surface vollständig E2E-verifiziert; Frontend-Devs können gegen die Live-API arbeiten und wissen, dass die HTTP-Responses (inkl. strukturierte 409-Bodies) stabil sind.
- **Keine Blocker.**

## Threat Coverage

| Threat ID | Mitigation | Verified-by |
|-----------|------------|-------------|
| T-08-06-01 (E2E test flakiness) | Phase-3/4/7-Test-Infrastruktur (random port + in-memory SQLite + auto-Migration) ist mit 255 Tests bewährt — Plan 06 fügt 15 weitere ohne Pattern-Bruch hinzu. | `cargo test --test e2e_tests --features mock_auth` 5 sequentielle Läufe alle 270/270 grün |
| T-08-06-02 (member-number format mismatch) | Test 13 prüft auf raw-Substring "42" (D-15-Implementation-Format aus Plan 04: plain `to_string()`); würde sowohl "42" als auch "M-42" erfassen, falls Format-Drift später käme. | Test 13 (`test_close_phase_with_pending_entries_returns_409_with_member_numbers`) grün; `body.contains("42")` |
| T-08-06-03 (audit-chain multi-process false-positive) | E2E nutzt single-process in-memory SQLite (Phase-1-Konvention); single-writer = deterministische Hash-Chain. Multi-Prozess-Sharding ist OUT OF SCOPE für v1.1. | Test 15 (`test_audit_chain_intact_after_phase_8_lifecycle`) grün; `verify.valid == true` + `broken_links.is_empty()` |

## Self-Check: PASSED

**Verified file modified:**
- `genossi_bin/tests/e2e_tests.rs`: FOUND (Mod, +680 LOC -24 LOC, 11649 LOC total)

**Verified commit exists:**
- `677eab1` (Task 1): FOUND in `git log --oneline -3`

**Verified tests pass:**
- 270/270 in `cargo test --test e2e_tests --features mock_auth`: passed
- davon 15 neue Phase-8-Tests + 255 Phase-7-Baseline-Tests
- `cargo build --tests -p genossi_bin`: exit 0 (nur pre-existing warnings ausserhalb Plan-Scope)

**Verified acceptance criteria (grep counts):**
- `async fn test_open_phase_triggers_auto_fill` == 1 ✓
- `async fn test_open_phase_auto_fill_zero_members` == 1 ✓
- `async fn test_open_phase_skips_member_without_exit_date` == 1 ✓
- `async fn test_open_phase_skips_member_outside_fiscal_year` == 1 ✓
- `async fn test_manual_add_entry_happy_path` == 1 ✓
- `async fn test_manual_add_entry_phase_not_open_returns_409` == 1 ✓
- `async fn test_manual_add_entry_share_count_exceeds_returns_400` == 1 ✓
- `async fn test_update_entry_status_open_to_contacted_succeeds` == 1 ✓
- `async fn test_update_entry_status_paid_out_returns_409` == 1 ✓
- `async fn test_delete_entry_in_open_succeeds` == 1 ✓
- `async fn test_batch_toggle_happy_path` == 1 ✓
- `async fn test_batch_toggle_paid_out_target_returns_400` == 1 ✓
- `async fn test_close_phase_with_pending_entries_returns_409_with_member_numbers` == 1 ✓
- `async fn test_close_phase_with_zero_entries_succeeds` == 1 ✓
- `async fn test_audit_chain_intact_after_phase_8_lifecycle` == 1 ✓
- `async fn create_member_with_exit_date` == 1 ✓
- `async fn create_open_repayment_phase` == 1 ✓
- Plural-Member-Route: `grep -c "/api/members" genossi_bin/tests/e2e_tests.rs` == 228 ✓ (>= 1)
- Singular-Member-POST aus Phase-8-Code: `grep -nE '\.post\(server\.url\("/api/member"\)' genossi_bin/tests/e2e_tests.rs | wc -l` == 0 ✓
- `cargo build --tests -p genossi_bin` exit 0 ✓
- `cargo test --test e2e_tests --features mock_auth` exit 0 mit 270 Tests grün (>= 270) ✓

---

*Phase: 08-repaymententry-auto-bef-llung*
*Completed: 2026-05-31*
