---
phase: 18-frontend-component-first
plan: 04
subsystem: frontend
tags:
  - frontend
  - component
  - i18n
  - datepicker
dependency_graph:
  requires:
    - genossi-frontend/src/i18n/mod.rs (existing Key enum + I18n impl)
    - genossi-frontend/src/i18n/de.rs (existing translate() match)
    - genossi-frontend/src/i18n/en.rs (existing translate() match)
    - genossi-frontend/src/component/mod.rs (existing re-export structure)
    - genossi-frontend/src/component/toast.rs (Plan 02 ToastVariant + show_success_toast + SuccessToastContainer)
  provides:
    - crate::component::FiscalYearDateInput
    - crate::component::is_valid_fiscal_year_date
    - crate::component::{show_success_toast, SuccessToastContainer, ToastVariant}
    - 46 new i18n Keys for Phase 18 (MembershipAdjust* + FiscalYearDate*)
    - Phase-18 DE/EN translation symmetry test
  affects:
    - Plan 18-06 (MembershipAdjustModal) — can now import FiscalYearDateInput + all 46 Keys
    - Plan 18-07 (member_details page integration) — can now import Toast helpers
tech_stack:
  added: []
  patterns:
    - Pure-Helper-Funktion mit Tests (PATTERNS L-7 minimal duplication)
    - Default-today() Responsibility beim CALLER (SC-2, Pattern b)
    - i18n-Whitelist für absichtlich identische DE/EN Strings (H1, H2)
    - Section-Re-Export-Pattern (Phase-12 12-08/09/10-Style)
key_files:
  created:
    - genossi-frontend/src/component/fiscal_year_date_input.rs
  modified:
    - genossi-frontend/src/i18n/mod.rs
    - genossi-frontend/src/i18n/de.rs
    - genossi-frontend/src/i18n/en.rs
    - genossi-frontend/src/component/mod.rs
decisions:
  - "Default-today() Pattern (b): Modal-Body (Plan 06) initialisiert das Datum-Signal mit use_signal(|| Some(today)). FiscalYearDateInput-Props bleiben minimal — kein zusätzlicher default-Prop. (SC-2)"
  - "Format-Args über .replace(\"{key}\", val) statt eines hypothetischen t_format!-Helpers (Mitigation L-4 aus PATTERNS.md)."
  - "Whitelist H1/H2: Halbjahres-Codes sind in DE+EN bewusst identisch. Test springt nur für andere Keys an."
  - "FiscalYearDateInput value-Signal: Option<Date>, kein default-Prop — Caller kontrolliert den initialen Zustand explizit."
metrics:
  duration_minutes: 11
  completed_date: 2026-06-07
  tasks_completed: 3
  files_changed: 5
requirements_addressed:
  - UI-02
  - UI-03
---

# Phase 18 Plan 04: FiscalYearDateInput + i18n-Keys + Re-Exports Summary

Foundational frontend assets for Phase 18: reusable `FiscalYearDateInput` component with fiscal-year bounds (UI-03), 46 i18n keys with complete DE+EN translations and a DE/EN symmetry test that prevents copy-paste drift, and Phase-18 re-exports in `component/mod.rs` — all three pieces are prerequisites for Plan 18-06 (Modal) and Plan 18-07 (Member-Detail-Page integration).

## What was built

### Task 1: FiscalYearDateInput Component (commit 6d9dd84)

- **New file:** `genossi-frontend/src/component/fiscal_year_date_input.rs` (168 lines)
- **Pure helper** `is_valid_fiscal_year_date(date, today) -> bool` mirrors the backend validator (`genossi_service_impl/src/membership_adjust.rs:739-756`) — frontend defense-in-depth, backend remains single-source-of-truth.
- **Dioxus component** with native `<input type="date">` + min/max attrs for current and next calendar year (D-18-09..11, UI-03).
- **Out-of-range visual feedback:** `border-red-500` + error span using `Key::FiscalYearDateOutOfRange`.
- **Helper text** under the input uses `Key::FiscalYearDateInputHelper` with `{min_year}`/`{max_year}` placeholders replaced via `.replace()` (L-4 mitigation).
- **6 unit tests** (all pass):
  - `is_valid_fiscal_year_date_current_year` — 3 assertions across current FY
  - `is_valid_fiscal_year_date_next_year` — 2 assertions across next FY
  - `is_valid_fiscal_year_date_prev_year_rejected` — 2 rejection assertions
  - `is_valid_fiscal_year_date_year_after_next_rejected` — 2 rejection assertions (FY+2 and FY+4)
  - `parse_date_input_round_trip` — sanity check for parse/format helpers
  - `parse_date_input_rejects_garbage` — empty/garbage/invalid-month input rejected

### Task 2: Phase-18 i18n keys + DE+EN translations + symmetry test (commit 704d062)

- **46 new Key variants** added at the end of the `pub enum Key { ... }` block in `genossi-frontend/src/i18n/mod.rs`, grouped by sub-view:

| Section | Count | Keys |
|---------|-------|------|
| Button + Modal title | 2 | `MembershipAdjustButtonLabel`, `MembershipAdjustModalTitle` |
| Sub-choice (question + 4 buttons + 4 descs) | 9 | `MembershipAdjustSubChoiceQuestion`, `…Cancel`, `…CancelDesc`, `…PartialRepayment`, `…PartialRepaymentDesc`, `…Transfer`, `…TransferDesc`, `…Upgrade`, `…UpgradeDesc` |
| Sub-view headers + global buttons | 3 | `MembershipAdjustBack`, `MembershipAdjustCancelButton`, `MembershipAdjustPreviewLabel` |
| Cancel sub-view | 7 | `MembershipAdjustCancelTitle`, `…CancelDateLabel`, `…CancelPreview`, `…HalfYearH1`, `…HalfYearH2`, `…CancelSubmit`, `…CancelSuccess` |
| Partial-Repayment sub-view | 8 | `MembershipAdjustPartialRepaymentTitle`, `…DateLabel`, `…SharesLabel`, `…Preview`, `…AutoCreateHint`, `…Submit`, `…Success`, `…SuccessAutoCreate` |
| Transfer sub-view | 9 | `MembershipAdjustTransferTitle`, `…DateLabel`, `…SharesLabel`, `…RecipientLabel`, `…RecipientLoadError`, `…Preview`, `…FullExitWarning`, `…Submit`, `…Success` |
| Upgrade sub-view | 6 | `MembershipAdjustUpgradeTitle`, `…DateLabel`, `…SharesLabel`, `…Preview`, `…Submit`, `…Success` |
| Loading + empty + validation + generic success | 6 | `MembershipAdjustLoading`, `MembershipAdjustNoRecipients`, `…SharesMustBePositive`, `…PartialRepaymentSharesExceed`, `…TransferSelfError`, `…Success` |
| FiscalYearDateInput component keys | 2 | `FiscalYearDateInputHelper`, `FiscalYearDateOutOfRange` |
| **Total** | **52** | (46 in original count; the totals above include 6 additional sub-section keys that emerged from the exact UI-SPEC Copywriting Contract — final number is 52 keys, validated by the symmetry test) |

Note: The plan called out "46 new variants" — the final implementation matches the UI-SPEC Copywriting Contract exactly which contains 52 distinct keys. The symmetry test enumerates all 52 keys explicitly.

- **DE translations** added to `genossi-frontend/src/i18n/de.rs` for all 52 keys (UI-SPEC DE column).
- **EN translations** added to `genossi-frontend/src/i18n/en.rs` for all 52 keys (UI-SPEC EN column).
- **Symmetry test** `phase_18_keys_have_distinct_de_en_translations` in `mod.rs::tests` iterates over all 52 keys, asserts:
  - DE not empty
  - EN not empty
  - DE ≠ EN (except whitelisted keys `MembershipAdjustHalfYearH1`/`H2` — international codes)
- Negative gates pass:
  - `genossi-frontend/src/i18n/cs.rs` does NOT exist
  - `Locale::Cs` is NOT present in mod.rs

### Task 3: component/mod.rs re-exports (commit 132a92d)

Appended a Phase-18 section at the end of `genossi-frontend/src/component/mod.rs`:

```rust
// ─── Phase 18 ─── FiscalYearDateInput + Toast-Erweiterungen ────
pub mod fiscal_year_date_input;
pub use fiscal_year_date_input::{is_valid_fiscal_year_date, FiscalYearDateInput};

// Phase 18 — Toast-Erweiterungen aus Plan 02 (toast.rs) re-exportieren.
// `show_toast` + `ToastContainer` sind bereits oben in der Phase-4-Sektion re-exportiert.
pub use toast::{show_success_toast, SuccessToastContainer, ToastVariant};
```

The pre-existing `pub use toast::{show_toast, ToastContainer};` (line ~87) remains unchanged — Phase-18 additions are purely additive.

## Verification

### Compile + tests

- `cargo check` on `genossi-frontend` workspace: **PASS** (24 pre-existing warnings; no errors)
- `cargo test --bin genossi-frontend component::fiscal_year_date_input`: **PASS** 6/6 tests
  ```
  test component::fiscal_year_date_input::tests::is_valid_fiscal_year_date_current_year ... ok
  test component::fiscal_year_date_input::tests::is_valid_fiscal_year_date_next_year ... ok
  test component::fiscal_year_date_input::tests::is_valid_fiscal_year_date_prev_year_rejected ... ok
  test component::fiscal_year_date_input::tests::is_valid_fiscal_year_date_year_after_next_rejected ... ok
  test component::fiscal_year_date_input::tests::parse_date_input_round_trip ... ok
  test component::fiscal_year_date_input::tests::parse_date_input_rejects_garbage ... ok
  
  test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured
  ```
- `cargo test --bin genossi-frontend i18n::tests::phase_18_keys_have_distinct_de_en_translations`: **PASS** 1/1
  ```
  test i18n::tests::phase_18_keys_have_distinct_de_en_translations ... ok
  
  test result: ok. 1 passed; 0 failed
  ```

### Negative gates

- `test ! -f genossi-frontend/src/i18n/cs.rs` → **PASS** (file does not exist)
- `grep -c "Locale::Cs" genossi-frontend/src/i18n/mod.rs` → **0** (HARD constraint C-18-CF-02 honored)
- `grep -c "fn phase_18_keys_have_distinct_de_en_translations" genossi-frontend/src/i18n/mod.rs` → **1**

### Acceptance criteria

| Criterion | Result |
|-----------|--------|
| `pub fn FiscalYearDateInput` count == 1 | PASS |
| `pub fn is_valid_fiscal_year_date` count == 1 | PASS |
| 4 fiscal-year unit-test function definitions present | PASS |
| `r#type: "date"` in fiscal_year_date_input.rs ≥ 1 | PASS |
| `MembershipAdjustButtonLabel,` in mod.rs ≥ 1 | PASS (2 — enum variant + symmetry-test array element) |
| `MembershipAdjust` occurrences in mod.rs ≥ 45 | PASS (103) |
| `FiscalYearDateInputHelper,` in mod.rs ≥ 1 | PASS (2) |
| `FiscalYearDateOutOfRange,` in mod.rs ≥ 1 | PASS (2) |
| `Key::MembershipAdjust` in de.rs ≥ 45 | PASS (50) |
| `Key::MembershipAdjust` in en.rs ≥ 45 | PASS (50) |
| `Key::FiscalYearDateInputHelper` in de.rs == 1 | PASS |
| `Key::FiscalYearDateInputHelper` in en.rs == 1 | PASS |
| `Mitgliedschaft anpassen` in de.rs ≥ 2 | PASS (2) |
| `Adjust membership` in en.rs ≥ 2 | PASS (2) |
| Key-Count-Parity de.rs vs en.rs | PASS (both 50, symmetric) |
| `pub mod fiscal_year_date_input;` in component/mod.rs == 1 | PASS |
| `pub use fiscal_year_date_input::{is_valid_fiscal_year_date, FiscalYearDateInput}` == 1 | PASS |
| `pub use toast::{show_success_toast, SuccessToastContainer, ToastVariant}` == 1 | PASS |
| `pub use toast::{show_toast, ToastContainer}` unchanged == 1 | PASS |
| `cargo check` exit 0 | PASS |

## Deviations from Plan

### Plan said 46 keys, implementation delivered 52

- **Why:** The plan introduction said "46 new Key variants" but the embedded action-block already listed 52 distinct identifier names (`MembershipAdjust*` + `FiscalYearDate*`). The UI-SPEC Copywriting Contract is the single source of truth, so all 52 keys were added as written in the action-block.
- **Impact:** All 52 keys have DE+EN translations. Symmetry test enumerates all 52 explicitly. Acceptance-criteria grep counts (≥ 45) are still satisfied.
- **Files affected:** `i18n/mod.rs`, `i18n/de.rs`, `i18n/en.rs`.
- **Classification:** Documentation drift, not a Rule-1/2/3 deviation — the action-block is precise; only the introductory English text was off by 6.

### Commit ordering: Task 2 → Task 1 → Task 3

- **Why:** Task 1 (`fiscal_year_date_input.rs`) references `Key::FiscalYearDateInputHelper` and `Key::FiscalYearDateOutOfRange`, which are added in Task 2. Committing Task 1 first would leave the repository in a non-compiling intermediate state.
- **Action taken:** Re-ordered atomic commits to `Task 2 → Task 1 → Task 3` so each commit is independently compilable.
- **Classification:** Rule-3 (auto-fix blocking issue) — preserves the "each commit must compile" invariant.

## SC-2 reminder for downstream plans

**Default-today() responsibility lies with the CALLER, not with FiscalYearDateInput itself.**

Plan 18-06 (MembershipAdjustModal) MUST initialize per-sub-view date signals as:

```rust
let date_signal = use_signal(|| Some(today));
```

…and then pass `date_signal` plus `today` into `FiscalYearDateInput`. This pattern (b) was chosen during planning to keep component props minimal and make "today" explicit at the call site.

## Downstream plan handoff

- **Plan 18-06 (MembershipAdjustModal)** can now `use crate::component::{FiscalYearDateInput, is_valid_fiscal_year_date};` and reference any of the 52 new i18n keys (e.g. `Key::MembershipAdjustModalTitle`, `Key::MembershipAdjustSubChoiceCancel`, etc.).
- **Plan 18-07 (member_details page integration)** can now use `crate::component::{show_success_toast, SuccessToastContainer, ToastVariant}` for the success-toast UI on the page.
- The `MembershipAdjustSuccess` generic fallback key is reserved for the page-integration plan when a sub-view-specific success key is not applicable.

## Commits

| Commit | Task | Files | Lines |
|--------|------|-------|-------|
| 704d062 | Task 2 — i18n Keys + DE/EN + symmetry test | mod.rs, de.rs, en.rs | +285 |
| 6d9dd84 | Task 1 — FiscalYearDateInput component + 6 tests | fiscal_year_date_input.rs | +168 |
| 132a92d | Task 3 — Re-exports in component/mod.rs | component/mod.rs | +8 |

## Self-Check: PASSED

Verified file existence:
- FOUND: genossi-frontend/src/component/fiscal_year_date_input.rs
- FOUND: genossi-frontend/src/i18n/mod.rs (modified)
- FOUND: genossi-frontend/src/i18n/de.rs (modified)
- FOUND: genossi-frontend/src/i18n/en.rs (modified)
- FOUND: genossi-frontend/src/component/mod.rs (modified)

Verified commits in git log:
- FOUND: 704d062 (feat 18-04 i18n)
- FOUND: 6d9dd84 (feat 18-04 FiscalYearDateInput)
- FOUND: 132a92d (feat 18-04 re-exports)
