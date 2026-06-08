---
id: 260608-jb1
status: complete
mode: quick
type: execute
completed: 2026-06-08T12:17:06Z
files_modified:
  - genossi_service_impl/src/membership_adjust.rs
  - genossi_bin/tests/membership_adjust_e2e.rs
files_moved:
  - from: .planning/debug/kuendigung-repayment-missing.md
    to: .planning/debug/resolved/kuendigung-repayment-missing.md
---

# Quick 260608-jb1 — Summary

**One-liner:** Symmetrischer RepaymentEntry-Fix für `cancel_membership` und `transfer_shares` Voll-Übertrag, analog `partial_repayment` Step 9+12 — neue Audit-Process-Strings für forensische Unterscheidbarkeit, idempotent über Skip-Pattern.

## Was repariert wurde

1. **`cancel_membership`** (`genossi_service_impl/src/membership_adjust.rs:160-251`):
   Neuer Phase-Resolve+Entry-Create-Block zwischen Re-Read und Tx-Commit. Skip wenn `updated_entity.current_shares == 0`. D-11.1 Status-Guard liefert HTTP 409 bei Closed-Phase. Auto-Phase-Create mit `DEFAULT_SHARE_VALUE_CENT`-Fallback. Skip-Pattern via `find_by_member_and_phase().is_empty()` für Idempotenz mit existierenden Entries (z.B. aus `partial_repayment`).

2. **`transfer_shares` Voll-Übertrag-Branch** (`membership_adjust.rs:692-790`):
   Symmetrischer Block nach Re-Read+vor commit, nur wenn `will_become_zero`. Kritisch: `share_count_to_pay_out = shares` (Parameter, Wert VOR Decrement), NICHT `from_final.current_shares` (= 0 nach Update). fiscal_year via `compute_effective_date(transfer_date)` abgeleitet.

3. **`partial_repayment` Step-5-Kommentar** (`membership_adjust.rs:320-326`):
   Alte "v1.1-PaidOut-Cascade"-Erläuterung war irreführend — aktualisiert auf neuen Fix-Pfad.

## Neue Process-Strings

| Process | Auslöser | Audit-Filter |
|---------|----------|--------------|
| `member-adjust.cancel.repayment` | RepaymentEntry-Create durch `cancel_membership` | `WHERE process = 'member-adjust.cancel.repayment'` |
| `member-adjust.transfer-full.repayment` | RepaymentEntry-Create durch Voll-Übertrag | `WHERE process = 'member-adjust.transfer-full.repayment'` |

Forensisch unterscheidbar von:
- `member-adjust.partial-repayment` (Teil-Rückgabe)
- `repayment-phase.open` (Auto-Fill beim Preparation→Open-Transition)
- `repayment-phase.create` (Auto-Anlegen der Phase)

## Test-Counts

| Test-Suite | Vor | Nach | Delta |
|---|---|---|---|
| `genossi_bin --test membership_adjust_e2e` (E2E) | 26 passed, 2 ignored | 31 passed, 2 ignored | **+5 neue Tests** |
| `genossi_service_impl membership_adjust::service_tests` (Unit) | 29 passed | 29 passed | +0 (3 bestehende Tests um Mocks erweitert) |
| `cargo test` (Workspace) | siehe Hinweis | 298 passed, 1 pre-existing fail | keine Regression |

**Neue E2E-Tests** (alle grün):
1. `test_cancel_membership_creates_repayment_entry_when_phase_open` — Phase open → Entry mit share_count=current_shares angelegt
2. `test_cancel_membership_auto_creates_phase_when_none_exists` — keine Phase → Auto-Create mit DEFAULT_SHARE_VALUE_CENT + Entry
3. `test_cancel_membership_closed_phase_returns_409` — Closed-Phase → 409, exit_date bleibt null (Tx rollback)
4. `test_cancel_membership_skips_when_entry_exists` — pre-existierender Entry aus partial_repayment → kein Duplikat, Wert bleibt
5. `test_transfer_shares_full_creates_repayment_entry` — Voll-Übertrag → Entry für entleerten Sender mit share_count=shares (3, nicht 0)

## Verifikation

- `cargo test`: 298 passed im Workspace, 1 pre-existing failure (siehe "Auffälligkeiten" unten)
- `cargo test -p genossi_bin --test membership_adjust_e2e --features mock_auth`: 31 passed, 2 ignored (alle 5 neuen + alle bestehenden)
- `cargo test -p genossi_service_impl membership_adjust::service_tests`: 29 passed
- `cargo clippy --all-targets --all-features`: clean (keine neuen Warnings)
- `cargo fmt -- --check`: `cargo fmt` ist im Toolchain-Profile nicht direkt verfügbar; manuelle Prüfung der hinzugefügten Blöcke via `/nix/store/.../rustfmt --edition 2021 --check` zeigt nur **pre-existing** fmt-Drift in unveränderten Code-Pfaden (Trait-Impl-Linie 87, etc.), keine fmt-Verletzung in unseren Additionen.

## Commits (jj change-ids + commit hashes)

| Change | Hash | Beschreibung |
|---|---|---|
| `mzvykumz` | `a6d7455419ab` | docs(quick-260608-jb1): create debug session and execution plan |
| `mxlntopr` | `0e81e066174d` | test(membership_adjust): add 4 failing e2e tests for cancel_membership RepaymentEntry creation [RED] |
| `nwtqlksz` | `666122310652` | fix(membership_adjust): cancel_membership creates RepaymentEntry in open phase [GREEN] |
| `kwlopwmv` | `5070af33a329` | test(membership_adjust): add failing e2e test for transfer_shares full-transfer [RED] |
| `qnstnkqv` | `e856ff4b3724` | fix(membership_adjust): transfer_shares Voll-Uebertrag creates RepaymentEntry [GREEN] |
| `tlzwuktr` | `d524500d8f4f` | test(membership_adjust): update service unit tests + resolve debug session |

## Auffälligkeiten / Deferred Issues

### Pre-existing Failure (NICHT durch diesen Fix verursacht)

**`test_mail_preview_repayment_no_entries_does_not_default_to_one`** in `genossi_bin/tests/e2e_tests.rs:14186`

- Symptom: `errors must be array` panic auf Line 14228.
- Verifikation: Test failt bereits auf `main` (commit `d389be47`) — getestet via `jj new d389be47 && cargo test test_mail_preview_repayment_no_entries_does_not_default_to_one`.
- Scope: Test gehört zu Quick `260602-c19-fix-mail-preview-repayment-kontext-share`-Familie (siehe v1.2-Closure deferred-items in STATE.md Zeile 47), nicht zu diesem Fix.
- Action: Out-of-scope per SCOPE BOUNDARY-Rule. Sollte in v1.3-Backlog reviewed werden.

### Cargo-fmt-Toolchain

`cargo fmt` ist nicht im `$PATH` direkt verfügbar (Nix-Toolchain). Manuelle Prüfung via Nix-Store-Pfad zeigt nur pre-existing fmt-Drift in unveränderten Code-Pfaden. Unsere additionen sind sauber formatiert. Bei Bedarf kann ein nachgelagerter `cargo fmt` Lauf den pre-existing Drift normalisieren — out-of-scope für diesen Bug-Fix.

## Referenzen

- Debug-Session (resolved): `.planning/debug/resolved/kuendigung-repayment-missing.md`
- Plan: `.planning/quick/260608-jb1-bug-fix-gekuendigte-mitglieder-in-repaym/260608-jb1-PLAN.md`
- Vorbild-Code: `genossi_service_impl/src/membership_adjust.rs:288-471` (`partial_repayment` Step 9+12)
- Idempotenz-Mit-Pattern: `genossi_service_impl/src/repayment_phase.rs:319-423` (`open_repayment_phase` Auto-Fill + Skip)

## Self-Check: PASSED

- [x] `genossi_service_impl/src/membership_adjust.rs` enthält `CANCEL_REPAYMENT_PROCESS` und `TRANSFER_FULL_REPAYMENT_PROCESS` constants
- [x] `cancel_membership` block ist zwischen Re-Read und commit eingefügt (Z. ~169-252)
- [x] `transfer_shares` Voll-Übertrag-Block ist zwischen Re-Read und commit eingefügt (Z. ~695-790)
- [x] 5 neue E2E-Tests existieren in `genossi_bin/tests/membership_adjust_e2e.rs` und sind alle grün
- [x] 3 bestehende Service-Unit-Tests (`test_cancel_membership_happy_path_h1/h2`, `test_transfer_shares_full_branch_creates_austritt`) wurden um Mock-Expectations für die neuen DAO-Calls erweitert und bleiben grün
- [x] `.planning/debug/kuendigung-repayment-missing.md` existiert NICHT mehr
- [x] `.planning/debug/resolved/kuendigung-repayment-missing.md` existiert mit `status: resolved` und Resolution-Section
- [x] `cargo clippy --all-targets --all-features` clean
- [x] `cargo test` keine neuen Regressionen (1 pre-existing failure dokumentiert)
- [x] Audit-Hashchain bleibt valid (durch bestehende `test_cancel_membership_audit_chain_verify` und `test_transfer_shares_audit_pair_verify_doppel_assertion` Tests verifiziert)
