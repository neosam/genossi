---
phase: 17-service-rest-uebertrag-cascade
plan: 04
subsystem: e2e/membership-adjust
tags: [rust, e2e, race-test, audit-verify, integration, transfer]
requires:
  - "POST /api/members/{from_id}/transfer-shares HTTP-Endpoint (Plan 17-03)"
  - "TransferSharesRequestTO + TransferSharesResponseTO DTOs (Plan 17-03)"
  - "MembershipAdjustService::transfer_shares Pipeline (Plan 17-02)"
  - "TRANSFER_PROCESS-Audit-Konstante = 'member-adjust.transfer' (Plan 17-01)"
  - "validate_transfer_inputs Pure-Function (Plan 17-01)"
  - "Phase-15 cancel-Endpoint (POST /api/members/{id}/cancel) — fuer Test 4 Setup"
  - "Phase-9 audit-log REST endpoints (GET /api/audit, GET /api/audit/verify)"
provides:
  - "8 neue E2E-Tests in genossi_bin/tests/membership_adjust_e2e.rs"
  - "Helper fn transfer_shares_body(to, shares, transfer_date) -> Value"
  - "Helper async fn assert_transfer_audit_trail(client, server, expected_action_count)"
  - "Verifikation des Phase-17-Stacks (REST + Service + DAO + Audit) end-to-end"
  - "Defense-in-Depth Race-Tests (Same- + Cross-Direction) gegen Concurrency-Bugs"
affects:
  - "Phase 18 (Frontend): kann sich auf den HTTP-Endpoint verlassen — alle 8 SC #5 Tests grün"
  - "Phase 17 ist abgeschlossen — alle 8 REQ-IDs (TRSF-01..05, TRSF-07, AUDT-02, PERM-03) sind via Service- + REST- + E2E-Tests abgedeckt"
tech-stack:
  added: []
  patterns:
    - "tokio::join! + 1ms Pool-Warm-up Race-Pattern (Vorbild: e2e_tests.rs Z. 12485-12596 Phase 9 D-12)"
    - "Sortierte Statuses + explizite NIE-Klauseln (Defense gegen [200, 200])"
    - "Cross-Direction-Konsistenz via summe-erhaltend-Check (5+5 -> 10) statt Single-Lock-Assumption"
    - "Audit-Trail-Doppel-Assertion: Hashchain-valid + tx_id-Count + entity_id-Count"
    - "Empty-Array-Schutz `len() >= 4` gegen Silent-Pass bei JSON-Schema-Mismatch (WARNING #4)"
key-files:
  created: []
  modified:
    - genossi_bin/tests/membership_adjust_e2e.rs
decisions:
  - "Plan-Truth #6 'single transaction_id' divergiert von Realität (build_audit_entries:65 generiert pro audited_*!-Aufruf neuen tx_id). Helper assertiert stattdessen drei robustere Atomarity-Kriterien: (a) Hashchain valid, (b) distinkte tx_ids = Anzahl audited_*!-Calls (Teil=4, Voll=5), (c) MemberAction-entity_ids = expected_action_count. Plan-Intention 'Atomarity' bleibt erhalten, aber semantisch korrekt."
  - "entity_type-Filter nutzt 'member_action' (snake_case per genossi_dao/src/member_action.rs:68), NICHT 'MemberAction' (Plan-Annahme). Korrektur dokumentiert im Helper-Doc-Kommentar."
  - "/api/audit?size=200 statt default 50 — sorgt für Empty-Array-Schutz auch wenn Audit-Tabelle wächst (Test-Isolation via in-memory-DB macht das defacto unkritisch, aber defense-in-depth)."
  - "Test 4 (recipient cancelled 409) wiederverwendet den existierenden cancel_body-Helper (Phase 15) statt eigene Body-Konstruktion — Pattern aus test_partial_repayment_cancelled_member_block_409 (Z. 746-780)."
  - "Race-Test-Konsistenz-Checks bleiben deterministisch ueber 3 aufeinanderfolgende Runs (Acceptance-Check)."
metrics:
  duration: "~45min"
  completed: "2026-06-06"
  tasks: 2
  files_modified: 1
  lines_added: 604
  e2e_tests_total: 8
  e2e_tests_full_suite: 26 # 18 prior + 8 new
---

# Phase 17 Plan 04: E2E-Test-Suite fuer Uebertrag — Summary

8 neue E2E-Tests in `genossi_bin/tests/membership_adjust_e2e.rs` verifizieren das Phase-17-Verhalten end-to-end über den realen HTTP-Server gegen in-memory SQLite. Deckt alle 8 REQ-IDs (TRSF-01..05, TRSF-07, AUDT-02, PERM-03) plus die beiden Race-Patterns aus D-17-06 ab. Phase 18 (Frontend) kann den Stack ohne Drift-Sorgen anbinden.

## Was wurde gebaut

### Task 1 — Helper + 5 Standard-E2E-Tests

Commit: `97116f4` — `test(17-04): E2E-Helper + 5 Standard-Tests fuer transfer-shares`

- `use uuid::Uuid;` Top-of-File hinzugefügt (für `fake_b_id = Uuid::new_v4()`).
- Helper `fn transfer_shares_body(to: &Uuid, shares: i32, transfer_date: &str) -> Value` (analog `partial_repayment_body`, baut `TransferSharesRequestTO`-JSON).
- 5 `#[tokio::test]`-Funktionen:

| # | Test                                                       | REQ-IDs              | Verifiziert                                                                  |
| - | ---------------------------------------------------------- | -------------------- | ---------------------------------------------------------------------------- |
| 1 | `test_transfer_shares_partial_happy_path`                  | TRSF-01, TRSF-04     | 5+1 → 3+3 Teil: 200, actions=2, from.exit_date=null                          |
| 2 | `test_transfer_shares_full_with_exit_date_cascade`         | TRSF-03, TRSF-05, D-17-01..03 | 3+1 → 0+4 Voll: 200, actions=3, last=Austritt mit transfer_member_id=b_id + effective_date=transfer_date, from.exit_date=transfer_date (recalc_dates Cascade) |
| 3 | `test_transfer_shares_self_transfer_400`                   | TRSF-07, D-17-08     | to_member_id == from_id → 400 mit "cannot transfer to self"                  |
| 4 | `test_transfer_shares_recipient_cancelled_409`             | PERM-03, D-17-07     | B vorab gekündigt + Transfer A→B → 409 mit "recipient already cancelled"     |
| 5 | `test_transfer_shares_recipient_not_found_404`             | D-17-10              | Fake-UUID → 404                                                              |

### Task 2 — Audit-Doppel-Assertion + 2 Race-Tests

Commit: `d20a38f` — `test(17-04): Audit-Verify + 2 Race-Tests (Same- + Cross-Direction)`

- Helper `async fn assert_transfer_audit_trail(client, server, expected_action_count: usize)`:
  - Fragt `GET /api/audit?size=200` ab und filtert auf `process == "member-adjust.transfer"`.
  - Empty-Array-Schutz: `assert!(transfer_entries.len() >= 4)` (WARNING #4 Silent-Pass-Defense).
  - Hashchain-Verifikation via `GET /api/audit/verify` (`valid == true`).
  - Distinkte `transaction_id`-Anzahl = erwartete Anzahl `audited_*!`-Aufrufe (Teil=4, Voll=5).
  - Distinkte MemberAction-`entity_id`s = expected_action_count (Teil=2, Voll=3).
- 3 weitere `#[tokio::test]`-Funktionen:

| # | Test                                                                   | REQ-IDs       | Verifiziert                                                                                                   |
| - | ---------------------------------------------------------------------- | ------------- | ------------------------------------------------------------------------------------------------------------- |
| 6 | `test_transfer_shares_audit_pair_verify_doppel_assertion`              | AUDT-02, D-17-05 | Teil-Uebertrag → assert_transfer_audit_trail(..., 2): Hashchain valid, 4 tx_ids, 2 MemberActions             |
| 7 | `test_transfer_shares_race_same_direction_sqlite_busy`                 | D-17-06       | A=4, B=1. tokio::join! 2× identische POSTs shares=2. Sortierte Statuses == [200, 409 \| 500]; NIE [200, 200]. A.current_shares == 2 nach Race. Audit-Chain valid. |
| 8 | `test_transfer_shares_race_cross_direction_consistency_check`          | D-17-06, T-17-04-07 | A=5, B=5. tokio::join! A→B + B→A shares=2. Erlaubt [200, 200] (kein Lock-Konflikt) ODER [200, 409 \| 500]. VERBOTEN [409 \| 500, 409 \| 500]. A+B == 10 nach Race. Audit-Chain valid. |

## Plan-Drift-Notizen (Rule 1 Auto-Fixes)

Der Plan-Truth-Block enthielt zwei Aussagen, die nicht zur realen Codebase passten. Beide wurden mit dokumentierten Auto-Fixes korrigiert:

### Drift 1: "single transaction_id" — falsche Atomarity-Semantik

**Plan-Annahme (Truth #6, D-17-05):** "alle Audit-Eintraege für process='member-adjust.transfer' teilen sich genau EINE transaction_id".

**Realität:** `genossi_service_impl/src/audit_log.rs:65` (`build_audit_entries`):
```rust
let transaction_id = uuid_fn();  // NEUE UUID pro audited_*!-Aufruf
```
Jede Macro-Invocation (`audited_create!` / `audited_update!`) generiert einen NEUEN `transaction_id`. `transaction_id` gruppiert nur die Field-Level-Rows EINER Macro-Invocation, nicht den gesamten Service-Pipeline-Aufruf. Diese Semantik ist im Audit-Subsystem so designed und auch in `audit_log.rs:321` als Eigenschaft getestet (`assert_eq!(entries[0].transaction_id, entries[1].transaction_id);` — innerhalb eines build_create_entries-Aufrufs).

**Auto-Fix (Rule 1):** Helper assertiert stattdessen drei robustere Atomarity-Kriterien:
1. **Audit-Hashchain bleibt valid** — bester Beweis, dass alle Writes in EINER DB-Transaction comitted wurden (Partial-Commit bräche die Hashchain).
2. **Distinkte `transaction_id`s = Anzahl `audited_*!`-Aufrufe** (Teil=4: abgabe + empfang + from-update + to-update; Voll=5: + austritt) — verifiziert dass jeder erwartete Write tatsächlich ausgeführt wurde.
3. **Distinkte MemberAction-`entity_id`s = expected_action_count** (Plan-Truth #6 Teil-(b) bleibt unverändert erhalten).

Dokumentiert mit ausführlichem Doc-Kommentar an `assert_transfer_audit_trail`.

### Drift 2: `entity_type == "MemberAction"` vs. `"member_action"`

**Plan-Annahme:** Filter auf `entity_type == "MemberAction"`.

**Realität:** `genossi_dao/src/member_action.rs:68`:
```rust
fn entity_type() -> &'static str { "member_action" }
```
Snake-case, nicht PascalCase. Alle anderen `Auditable`-Implementierungen folgen dem gleichen Pattern (`member`, `application`, etc.).

**Auto-Fix (Rule 1):** Filter angepasst auf `"member_action"`, mit Inline-Doc-Kommentar zur Vermeidung von Future-Drift.

## Test-Schema-Verifikation (Pre-Step / WARNING #4)

Vor dem Schreiben des Helpers wurde das Audit-JSON-Schema verifiziert über `genossi_rest_types/src/lib.rs:1741-1797`:

```rust
pub struct AuditLogEntryTO {
    pub id: Uuid,
    pub timestamp: String,
    pub user_id: String,
    pub process: String,
    pub transaction_id: Uuid,
    pub entity_type: String,
    pub entity_id: Uuid,
    pub action: String,
    pub field_name: String,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
}

pub struct PagedAuditLogTO {
    pub entries: Vec<AuditLogEntryTO>,
    pub total: i64,
    pub page: i64,
    pub size: i64,
}
```

JSON-Root hat `entries: [...]`, jeder Entry hat `transaction_id`, `entity_type`, `entity_id`, `process`. Default-Page-Size=50 — wir fragen `?size=200` an, um den Empty-Array-Schutz robust zu halten.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Plan-Truth "single transaction_id" widerspricht Audit-Subsystem-Design**

- **Found during:** Task 2 — initialer Test-Run von `test_transfer_shares_audit_pair_verify_doppel_assertion`
- **Issue:** Plan-Helper-Spezifikation assertierte `entries[0].transaction_id == entries[i].transaction_id` für alle Phase-17-Audit-Entries. Realität: jeder `audited_*!`-Macro-Aufruf generiert eine eigene `transaction_id` (siehe Drift 1 oben).
- **Fix:** Helper umstrukturiert auf drei robustere Atomarity-Kriterien (Hashchain-valid, distinkte-tx-id-Count, MemberAction-Count). Plan-Intention bleibt erhalten, semantisch korrekt verifiziert.
- **Files modified:** `genossi_bin/tests/membership_adjust_e2e.rs` (assert_transfer_audit_trail Helper-Body + Doc-Kommentar)
- **Commit:** `d20a38f` (Task 2)

**2. [Rule 1 - Bug] `entity_type`-Filter PascalCase vs. snake_case**

- **Found during:** Task 2 — initialer Test-Run nach Drift-1-Fix
- **Issue:** Helper filterte auf `entity_type == "MemberAction"`. Realität: `MemberActionEntity::entity_type() == "member_action"`.
- **Fix:** Filter angepasst auf `"member_action"`; Doc-Kommentar dokumentiert die Quelle (`genossi_dao/src/member_action.rs:68`).
- **Files modified:** `genossi_bin/tests/membership_adjust_e2e.rs` (assert_transfer_audit_trail Filter-Zeile)
- **Commit:** `d20a38f` (Task 2)

**3. [Rule 3 - Blocker] `SQLX_OFFLINE=true` weiterhin nötig**

- **Found during:** Task 1 Build
- **Issue:** Wie in Plan 17-01..03 — `genossi_dao_impl_sqlite` braucht SQLite-DB für SQLx-Compile-Time-Checks; im Worktree existiert kein `genossi.db`.
- **Fix:** `SQLX_OFFLINE=true` als Env-Variable bei `cargo build`/`cargo test`/`cargo clippy`. Kein Code-Change.
- **Files modified:** none
- **Impact:** Konsistent mit Plan 17-01..03 — keine neue Auswirkung.

**4. [Rule 3 - Blocker] Disk Space Critical + Worktree-Workaround**

- **Found during:** Task 1 Build
- **Issue:** `/home/neosam/programming` ist zu 100% gefüllt. Build im Worktree mit lokalem `target/`-Verzeichnis schlägt mit `No space left on device` fehl. Auch Worktree-Edits werden gegen ein nicht-git-getracktes File-Copy gemacht (Pattern bekannt aus Plan 17-02-Summary).
- **Fix:** `CARGO_TARGET_DIR=/home/neosam/programming/rust/projects/genossi3/target` zeigt auf Main-Repo-Target; vermeidet 3-4 GB doppelte Build-Artefakte. Nach jedem `Edit` im Worktree `cp <worktree-Pfad> <main-repo-Pfad>` ausgeführt; danach `git add`/`git commit` im Main-Repo (Pattern aus Plan 17-02).
- **Files modified:** none (Umgebungs-Workflow)
- **Impact:** Workspace-Test mit `--all-features` konnte nicht ausgeführt werden (auch wegen pre-existing `transfer_recipients_e2e`-Konflikt, der mit `--features mock_auth` aber sauber kompiliert).

### Out-of-Scope Discoveries (NICHT von Phase-17-Änderungen verursacht)

Dokumentiert in `.planning/phases/17-service-rest-uebertrag-cascade/deferred-items.md`:

1. **Pre-existing Failure** `test_mail_preview_repayment_no_entries_does_not_default_to_one` in `e2e_tests.rs:13964` — `errors must be array` panic. Letzter Touch in `1e48b2f` (Phase Mail-Quick-Fix).
2. **`--all-features` Build-Fehler** auf `transfer_recipients_e2e.rs` — vermutlich `oidc`-Feature-Konflikt; mit `--features mock_auth` ok.

## Verification Results

| Check                                                                                          | Erwartung                  | Result                              |
| ---------------------------------------------------------------------------------------------- | -------------------------- | ----------------------------------- |
| `cargo test --test membership_adjust_e2e -p genossi_bin test_transfer_shares`                  | 8 passed                   | **8 passed; 0 failed; 0 ignored**  |
| `cargo test --test membership_adjust_e2e -p genossi_bin` (full suite)                          | alle gruen                 | **26 passed; 0 failed; 2 ignored** (Phase-15 perm) |
| Race-Determinismus (3 Runs, `test_transfer_shares_race`)                                       | je 2 passed                | **3/3 Runs: 2 passed each**         |
| `cargo clippy --test membership_adjust_e2e -p genossi_bin`                                     | 0 Fehler                   | **0**                               |
| `grep -c 'fn transfer_shares_body'`                                                            | 1                          | **1**                               |
| `grep -c 'test_transfer_shares_…' (5 Task-1-Tests)`                                            | 5                          | **5**                               |
| `grep -c '"Austritt"'`                                                                         | >=1                        | **2**                               |
| `grep -c 'recipient already cancelled'`                                                        | >=1                        | **2**                               |
| `grep -c 'cannot transfer to self'`                                                            | >=1                        | **2**                               |
| `grep -c 'fn assert_transfer_audit_trail'`                                                     | 1                          | **1**                               |
| `grep -c 'tokio::join!'`                                                                       | >=2                        | **4** (je Race-Test 2 Refs via Tuple) |
| `grep -c 'Duration::from_millis(1)'`                                                           | >=2                        | **2**                               |
| `grep -c 'audit/verify'`                                                                       | >=3                        | **8**                               |
| `grep -c 'member-adjust.transfer'`                                                             | >=1                        | **1**                               |
| WARNING #4 Empty-Array-Schutz: `grep -c 'expected >=4 audit rows'`                             | >=1                        | **1**                               |
| WARNING #5 NIE-Klauseln: `grep -cF 'NIE [200, 200]'`                                           | >=1                        | **2**                               |
| WARNING #5 NIE-Klauseln: `grep -cE 'NIE Total-Deadlock\|Anteile-Summe'`                        | >=1                        | **5**                               |

## Phase-17-Closure-Notiz

Alle 8 REQ-IDs sind jetzt vollständig abgedeckt:

| REQ-ID  | Service-Tests (Plan 17-02) | REST-Tests (Plan 17-03) | E2E-Tests (Plan 17-04) |
| ------- | -------------------------- | ----------------------- | ---------------------- |
| TRSF-01 | Test 1 (partial happy)     | Handler + 200-Response   | Test 1 (partial happy) |
| TRSF-02 | Mock-Filter TRANSFER_PROCESS | DTO + Tag                | (implizit via Audit-Tests) |
| TRSF-03 | Test 2 (full branch)       | DTO + Path               | Test 2 (full + Austritt) |
| TRSF-04 | Tests 1+2 (member updates) | DTO `from`/`to`-Response | Tests 1+2 (current_shares-Asserts) |
| TRSF-05 | Test 2 (effective_date)    | DTO `transfer_date`     | Test 2 (effective_date-Assert) |
| TRSF-07 | Test 6 (self-transfer)     | DTO `to_member_id`-Vali | Test 3 (self-transfer 400) |
| AUDT-02 | grep-gate für TRANSFER_PROCESS | (implizit via Service)   | Test 6 (Audit-Doppel-Assertion) |
| PERM-03 | Test 7 (recipient cancelled) | 409-Response             | Test 4 (recipient cancelled 409) |

Plus die D-17-06-Race-Patterns: Test 7 (Same-Direction SQLITE_BUSY) + Test 8 (Cross-Direction Konsistenz) sichern Concurrency-Defense.

Phase 18 (Frontend) kann den HTTP-Endpoint ohne weitere Backend-Änderungen anbinden.

## Success Criteria Status

- [x] Alle 8 E2E-Tests aus CONTEXT.md SC #5 implementiert und grün
- [x] Helper `transfer_shares_body` + `assert_transfer_audit_trail` existieren (1× je)
- [x] Race-Tests sind deterministisch (3 Runs ohne Flake)
- [x] Phase 17 vollständig abgeschlossen — Phase 18 (Frontend) kann auf den HTTP-Endpoint zugreifen
- [x] v1.1-Audit-Hashchain bleibt valid (jeder Test endet mit `audit/verify` → valid=true)
- [x] cargo test --test membership_adjust_e2e -p genossi_bin: 26 passed / 0 failed

## Self-Check: PASSED

- File `genossi_bin/tests/membership_adjust_e2e.rs` → FOUND (1536 Zeilen; +604 in Phase 17)
  - `fn transfer_shares_body` → FOUND
  - `fn assert_transfer_audit_trail` → FOUND
  - 8 `test_transfer_shares_*` Funktionen → FOUND
- File `.planning/phases/17-service-rest-uebertrag-cascade/17-04-SUMMARY.md` → FOUND (diese Datei)
- File `.planning/phases/17-service-rest-uebertrag-cascade/deferred-items.md` → FOUND
- Commit `97116f4` → FOUND in git log (test(17-04): E2E-Helper + 5 Standard-Tests fuer transfer-shares)
- Commit `d20a38f` → FOUND in git log (test(17-04): Audit-Verify + 2 Race-Tests (Same- + Cross-Direction))
