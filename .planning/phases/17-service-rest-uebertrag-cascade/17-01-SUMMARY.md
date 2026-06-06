---
phase: 17-service-rest-uebertrag-cascade
plan: 01
subsystem: service/membership-adjust
tags: [rust, axum, service-trait, validation, pure-function, transfer]
requires:
  - "genossi_service::membership_adjust::MembershipAdjustService trait (existing) -- extended"
  - "genossi_service_impl::membership_adjust impl block (existing) -- extended"
  - "ValidationFailureItem from genossi_service (reused)"
provides:
  - "MembershipAdjustService::transfer_shares trait method signature (frozen for Plan 17-02)"
  - "TRANSFER_PROCESS constant = 'member-adjust.transfer' (shared audit process string)"
  - "validate_transfer_inputs pure function (DAO-free, deterministic, 7 unit tests green)"
affects:
  - "Plan 17-02 (Pipeline-Impl): fills the unimplemented!() body, calls validate_transfer_inputs"
  - "Plan 17-03 (REST): maps ValidationError -> 400 via existing From<ServiceError>"
  - "Plan 17-04 (E2E): asserts process='member-adjust.transfer' in audit log"
tech-stack:
  added: []
  patterns:
    - "additive trait extension (D-15-13)"
    - "stub-impl with unimplemented!() to keep cargo build green between plans"
    - "pure-function range-validator next to validate_partial_repayment_shares (D-17-09)"
    - "shared audit-process-string constant for cascade writes (AUDT-02)"
key-files:
  created: []
  modified:
    - genossi_service/src/membership_adjust.rs
    - genossi_service_impl/src/membership_adjust.rs
decisions:
  - "Trait-Methode transfer_shares ist additiv im bestehenden #[automock]-Trait; keine neue Trait-Datei (D-15-13 inkrementelles Wachsen)"
  - "Stub-Impl mit unimplemented!('Plan 17-02 implements the 15-step pipeline') -- cargo build bleibt gruen, Trait-Drift unmoeglich"
  - "Shares-Typ ist i32 (NICHT i64 wie CONTEXT D-17 vermutete) -- konsistent mit MemberEntity.current_shares + RepaymentEntryEntity.share_count_to_pay_out (Pattern-Mapper-Korrektur)"
  - "validate_transfer_inputs sammelt ALLE Verletzungen (kein early-return) -- ermoeglicht UI-Feedback fuer mehrere Felder gleichzeitig"
  - "#[allow(dead_code)] auf TRANSFER_PROCESS + validate_transfer_inputs -- Plan 17-02 entfernt das, weil dann beide vom Pipeline-Body referenziert werden"
metrics:
  duration: "~25min"
  completed: "2026-06-06"
  tasks: 2
  files_modified: 2
---

# Phase 17 Plan 01: Service-Trait + Pure-Function-Validator Summary

Erweitert den `MembershipAdjustService`-Trait um die `transfer_shares`-Methode (additive Signatur), legt die geteilte Audit-Process-Konstante `TRANSFER_PROCESS = "member-adjust.transfer"` an und implementiert die Pure-Function `validate_transfer_inputs` mit allen 7 Edge-Case-Unit-Tests. Stub-Impl mit `unimplemented!()` haelt `cargo build` gruen; Plan 17-02 fuellt die 15-step-Pipeline ohne Trait-Drift.

## Was wurde gebaut

### Task 1 — Trait-Methode + TRANSFER_PROCESS-Konstante

- `MembershipAdjustService::transfer_shares` wurde nach `partial_repayment` in das bestehende `#[automock]`-`#[async_trait]`-Trait eingefuegt (genossi_service/src/membership_adjust.rs, Zeilen 74–91).
  - Doc-Kommentar dokumentiert C-17-CF-08 (domain-Werte, kein DTO-Wrapping), TRSF-05 (sofort wirksam, kein H1/H2), D-17-01/03 (Voll-Uebertrag-Branch mit `transfer_member_id = Some(to_id)`).
  - Rueckgabe-Tuple: `(Vec<MemberAction>, Member, Member)` — 2 oder 3 Actions je nach Teil/Voll-Uebertrag.
- Konstante `const TRANSFER_PROCESS: &str = "member-adjust.transfer"` nach `PARTIAL_REPAYMENT_PROCESS` (genossi_service_impl/src/membership_adjust.rs, Zeilen 40–44).
  - `#[allow(dead_code)]` markiert die Konstante als bewussten Uebergangs-Zustand, weil sie erst Plan 17-02 verwendet.
- Stub-Impl `async fn transfer_shares(...) -> ... { unimplemented!("Plan 17-02 implements the 15-step pipeline") }` (genossi_service_impl/src/membership_adjust.rs, Zeilen 470–489).

**Commit:** `6c03524` — feat(17-01): Trait-Methode transfer_shares + TRANSFER_PROCESS-Konstante deklarieren

### Task 2 — Pure-Function `validate_transfer_inputs` + 7 Edge-Case-Tests

- `pub(crate) fn validate_transfer_inputs(from_id, to_id, shares, from_current_shares) -> Vec<ValidationFailureItem>` (genossi_service_impl/src/membership_adjust.rs, Zeilen 592–636).
  - Sammelt ALLE Verletzungen statt early-return (D-17-09: deterministisches Test-Verhalten, UI kann mehrere Felder gleichzeitig markieren).
  - Self-transfer (`from_id == to_id`) -> `field = "to_member_id"` mit Message `"cannot transfer to self"` (TRSF-07 / D-17-08).
  - `shares <= 0` -> `field = "shares"`, Message `"shares must be at least 1"`.
  - `shares > from_current_shares` -> `field = "shares"`, Message enthaelt `"exceeds from.current_shares"`.
  - **Voll-Uebertrag-Boundary** (`shares == from_current_shares`) ist GUELTIG (returns empty `Vec`) — Voll-Uebertrag-Branch wird im Service-Body von Plan 17-02 ausgewertet.
- 7 Unit-Tests (alle gruen, kein DAO-Mock noetig):
  1. `test_validate_transfer_n_zero_invalid` — n=0 -> 1 Error mit `field=shares`, `at least 1`.
  2. `test_validate_transfer_n_negative_invalid` — n=-1 -> 1 Error mit `field=shares`.
  3. `test_validate_transfer_n_equal_current_shares_valid` — n=5, current=5 -> empty Vec.
  4. `test_validate_transfer_n_exceeds_current_shares_invalid` — n=6, current=5 -> 1 Error mit `field=shares`, `exceeds`.
  5. `test_validate_transfer_self_invalid` — from_id == to_id -> Error mit `field=to_member_id`, `cannot transfer to self`.
  6. `test_validate_transfer_n_one_valid` — n=1, current=5 (Teil-Uebertrag) -> empty Vec.
  7. `test_validate_transfer_multiple_violations_accumulate` — self-transfer UND shares=0 -> 2 Errors.

**Commit:** `4e44083` — test(17-01): Pure-Function validate_transfer_inputs + 7 Edge-Case-Tests

## Referenz-Punkte fuer Plan 17-02

Plan 17-02 ersetzt den Stub-Body durch die 15-step Pipeline und entfernt die beiden `#[allow(dead_code)]`-Marker. Plan 17-02 muss kein Trait, keine Konstante, keine Validation neu definieren — alle Vertrags-Punkte sind bereits eingefroren:

- **Stub-Impl-Body:** genossi_service_impl/src/membership_adjust.rs Zeilen 470–489 (Stub-Doc + `unimplemented!("Plan 17-02 implements the 15-step pipeline")`).
- **Trait-Signatur:** genossi_service/src/membership_adjust.rs Zeilen 74–91 (exakt) — Plan 17-02 muss die identische Signatur implementieren.
- **Audit-Process-String:** `TRANSFER_PROCESS` (Zeile 44) verwenden in `audited_create!`/`audited_update!`-Macros fuer ALLE 4–5 Cascade-Writes (MemberAction-Abgabe, MemberAction-Empfang, optional MemberAction-Austritt, Member-Update-From, Member-Update-To).
- **Validation-Pipeline-Schritt:** `validate_transfer_inputs(from_id, to_id, shares, from_entity.current_shares)` als Schritt 6 (nach Permission-Funnel + Member-Existence-Check fuer beide Members) aufrufen; bei `!errors.is_empty()` -> `Err(ServiceError::ValidationError(errors))`.

## Imports / Use-Statements

Es war KEINE neue use-Statement noetig:

- `genossi_service/src/membership_adjust.rs`: alle Typen (`Uuid`, `time::Date`, `Member`, `MemberAction`, `Authentication`, `Self::Context/Transaction`, `ServiceError`) sind bereits durch die `partial_repayment`/`cancel_membership`-Methoden importiert.
- `genossi_service_impl/src/membership_adjust.rs`: `Uuid`, `Date`, `Arc`, `ValidationFailureItem`, `Member`, `MemberAction`, `Authentication`, `ServiceError` sind bereits am Top-of-File. Die neue Pure-Function nutzt `uuid::Uuid` (vollqualifiziert in den Tests) und greift via `super::validate_transfer_inputs` auf die Funktion zu.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocker] `SQLX_OFFLINE=true` fuer cargo build noetig**

- **Found during:** Task 1 verification
- **Issue:** `cargo build --workspace --all-features` schlug fehl mit "unable to open database file" — `genossi_dao_impl_sqlite` braucht eine SQLite-DB fuer SQLx-Compile-Time-Checks; im Worktree existiert kein `genossi.db`.
- **Fix:** Build und Test mit `SQLX_OFFLINE=true` ausgefuehrt — `.sqlx/`-Verzeichnis liefert die Offline-Query-Daten. Kein Code-Change noetig.
- **Files modified:** none (Umgebungs-Variable, kein Repository-State)
- **Impact:** Plan 17-02 wird ebenfalls `SQLX_OFFLINE=true` brauchen, sofern die Worktree-Umgebung gleich bleibt.

**2. [Rule 2 - Critical] `#[allow(dead_code)]` auf `TRANSFER_PROCESS` und `validate_transfer_inputs`**

- **Found during:** Task 1+2 build
- **Issue:** Rust `cargo build --all-features` markiert nicht verwendete `fn` als Warning (`dead_code` Lint); `const` ist betroffen, falls die Visibility erlaubt aber kein Caller existiert. Plan 17-02 verwendet beide, aber zwischen Plan 17-01 und 17-02 sind sie ungenutzt.
- **Fix:** `#[allow(dead_code)]` Attribut mit Kommentar `// Plan 17-02 verwendet die Konstante in der Pipeline-Impl.` / `// Plan 17-02 ruft die Funktion aus der Pipeline auf.` — laeuft praezise dokumentiert + selbst-aufraeumend (Plan 17-02 entfernt die Marker, sobald Code referenziert).
- **Files modified:** genossi_service_impl/src/membership_adjust.rs
- **Commit:** in Tasks 1+2 enthalten

## Verification Results

- `cargo test -p genossi_service_impl --lib membership_adjust::tests::test_validate_transfer` -> 7 PASS / 0 FAIL / 0 IGNORED ([siehe Task 2 oben](#task-2--pure-function-validate_transfer_inputs--7-edge-case-tests))
- `cargo build --workspace --all-features` (mit `SQLX_OFFLINE=true`) -> exits 0
- `grep -c 'async fn transfer_shares' genossi_service/src/membership_adjust.rs` -> `1`
- `grep -c 'const TRANSFER_PROCESS: &str = "member-adjust.transfer"' genossi_service_impl/src/membership_adjust.rs` -> `1`
- `grep -c 'unimplemented!' genossi_service_impl/src/membership_adjust.rs` -> `1` (Stub-Marker)
- `grep -c 'pub(crate) fn validate_transfer_inputs' genossi_service_impl/src/membership_adjust.rs` -> `1`
- `grep -c '"cannot transfer to self"' genossi_service_impl/src/membership_adjust.rs` -> `2` (Funktion + Test-Assertion)

## Success Criteria Status

- [x] MembershipAdjustService-Trait hat `transfer_shares` mit exakter D-17/CONTEXT-Signatur
- [x] TRANSFER_PROCESS-Konstante deklariert
- [x] validate_transfer_inputs Pure-Function mit 7 gruenen Unit-Tests
- [x] Stub-Impl erlaubt `cargo build --workspace` gruen
- [x] Keine Trait-Drift mehr moeglich, wenn Plan 17-02 die Pipeline fuellt

## Self-Check: PASSED

- File `genossi_service/src/membership_adjust.rs` -> FOUND (mit `async fn transfer_shares` an Zeile 83)
- File `genossi_service_impl/src/membership_adjust.rs` -> FOUND (mit `TRANSFER_PROCESS` Zeile 44, `validate_transfer_inputs` Zeile 607, `unimplemented!` Zeile 488)
- Commit `6c03524` -> FOUND in git log
- Commit `4e44083` -> FOUND in git log
