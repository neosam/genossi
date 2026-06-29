# Phase 17 Deferred Items

Out-of-scope discoveries protokolliert während der Phase-17-Ausführung. Diese
Issues wurden NICHT von Phase-17-Änderungen verursacht und sind nicht durch
Phase-17 zu fixen.

## 1. Pre-existing Failure: `test_mail_preview_repayment_no_entries_does_not_default_to_one`

- **Location:** `genossi_bin/tests/e2e_tests.rs:13964`
- **Last touched by:** Commit `1e48b2f` — `test(mail): add E2E regression tests for repayment preview (#260602-c19)`
- **Status:** Bereits vor Phase-17-Start rot. Test schlägt mit panic
  `'errors must be array'` fehl — nicht durch Phase-17-Änderungen
  verursacht (nur `genossi_bin/tests/membership_adjust_e2e.rs` wurde modifiziert).
- **Action:** Sollte in einer separaten Quick-Fix-Iteration für das Mail-Subsystem
  adressiert werden.

## 2. `--all-features` build failure on `transfer_recipients_e2e`

- **Location:** `genossi_bin/tests/transfer_recipients_e2e.rs`
- **Status:** Bei `cargo test --all-features` Compile-Error (vermutlich
  `oidc`-Feature-Konflikt). Mit `--features mock_auth` kompiliert es sauber.
- **Action:** Feature-Kompatibilität in einer separaten Iteration prüfen.

## 3. Disk Space Critical

- **Status:** `/home/neosam/programming` ist zu 100% gefüllt (929/950 GB).
  Build verfügt im Worktree über teilweise <1GB freien Speicher, was
  `cargo test --workspace --all-features` verhindert.
- **Workaround Phase-17:** `CARGO_TARGET_DIR=/home/neosam/programming/rust/projects/genossi3/target`
  zeigt auf shared Main-Repo-Target; vermeidet 3-4 GB doppeltes Build-Cache.
- **Action:** Old worktree-target-dirs prunen oder allgemein größere Festplatte.
