---
phase: 18-frontend-component-first
plan: 06
subsystem: frontend-component
tags:
  - frontend
  - dioxus
  - component
  - modal
  - membership-adjust
dependency_graph:
  requires:
    - "18-01: 8 Phase-15/16/17 DTOs (MemberSlimTO, CancelMembershipRequestTO, ...)"
    - "18-02: ToastVariant + show_success_toast + SuccessToastContainer"
    - "18-03: MemberSearch members_override prop"
    - "18-04: FiscalYearDateInput + 46 i18n keys"
    - "18-05: 5 API client functions (cancel_membership, increase_shares, partial_repayment, transfer_shares, get_transfer_recipients)"
  provides:
    - MembershipAdjustModal Dioxus component
    - 4 pure helpers (compute_effective_date_mirror, is_voll_uebertrag, to_member_to, format_date_german)
    - ModalStep enum state machine (SubChoice + 4 ops)
    - Re-exports in component/mod.rs
  affects:
    - "18-07: Page integration (member_details.rs) — can now mount MembershipAdjustModal"
tech_stack:
  added: []
  patterns:
    - "Single-file modal with ModalStep enum + match-rsx for 5 sub-views (D-18-02, D-18-03)"
    - "I18n: Clone pattern for sub-functions (verified i18n/mod.rs:908)"
    - "Pure-helper-mirror of backend logic + 12 unit tests (PATTERNS.md L-7)"
    - "Dioxus button-reload-bug mitigation: r#type:\"button\" + onclick everywhere (C-18-CF-03)"
    - "DSGVO PII drop adapter (MemberSlimTO → MemberTO with all PII fields None)"
    - "Live-preview-as-confirmation (D-18-05, CANC-06): no second-step confirm dialog"
key_files:
  created:
    - genossi-frontend/src/component/membership_adjust_modal.rs
  modified:
    - genossi-frontend/src/component/mod.rs
decisions:
  - "Sub-Views as 4 dedicated private fn render_*_sub_view(i18n: I18n, ...) using I18n: Clone (verified i18n/mod.rs:908) — keeps Modal-body readable and avoids inline-match-arm complexity"
  - "Pure helpers exported (pub fn) instead of pub(crate) — enables future unit testing from outside the module and matches Phase-12 repayment_entry_paidout_confirm.rs convention"
  - "current_shares typed as i32 (matches MemberTO field type), not i64 — Plan referenced i64 originally but actual MemberTO.current_shares is i32 (rest-types/src/lib.rs:210)"
  - "Status field assertion in to_member_to_drops_pii_fields uses MemberStatusTO::Normal (matches struct default per impl Default for MemberStatusTO at rest-types/src/lib.rs:182-186)"
metrics:
  duration_minutes: 5
  completed_date: 2026-06-07
  tasks_completed: 2
  files_created: 1
  files_modified: 1
  tests_added: 12
  loc_modal: 1078
requirements_addressed:
  - UI-02
  - UI-04
  - CANC-06
---

# Phase 18 Plan 06: MembershipAdjustModal — Summary

**Single-file Dioxus modal component (1078 LOC) with ModalStep enum state machine and 4 operation sub-views (Cancel, PartialRepayment, Transfer, Upgrade) — the heart of Phase 18. Closes UI-02 (Modal as shared component), UI-04 (preview section), CANC-06 (preview-as-confirmation), SC-2 (default-today). All 12 pure-helper unit tests green, all Dioxus reload-bug negative gates passed.**

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | MembershipAdjustModal + 4 sub-views + 4 pure helpers + 12 tests | 29d96b3 | genossi-frontend/src/component/membership_adjust_modal.rs (1078 LOC) |
| 2 | Re-export in component/mod.rs (Phase-18 section) | 3a07cd1 | genossi-frontend/src/component/mod.rs |

## Modal Architecture

```
MembershipAdjustModal(member, today, on_close, on_success)
│
├── ModalStep::SubChoice    → render_sub_choice (4 flat Buttons in grid grid-cols-1 sm:grid-cols-2 gap-4)
├── ModalStep::Cancel       → render_cancel_sub_view    (api::cancel_membership)
├── ModalStep::PartialRepay → render_partial_sub_view   (api::partial_repayment)
├── ModalStep::Transfer     → render_transfer_sub_view  (api::transfer_shares + api::get_transfer_recipients)
└── ModalStep::Upgrade      → render_upgrade_sub_view   (api::increase_shares)

Shared State:
- date_signal: Signal<Option<time::Date>>  — initialized with Some(today) → SC-2 default-today
- shares_signal: Signal<i32>
- recipient_id_signal: Signal<Option<Uuid>>
- submitting: Signal<bool>
- error_signal: Signal<Option<AppError>>  → displayed via ErrorAlert above sub-view body
```

## Test Output (`cargo test --bin genossi-frontend component::membership_adjust_modal`)

```
running 12 tests
test component::membership_adjust_modal::tests::compute_effective_h1_june_30_boundary ... ok
test component::membership_adjust_modal::tests::compute_effective_h1_leap_year_feb_29 ... ok
test component::membership_adjust_modal::tests::compute_effective_h1_mid_year ... ok
test component::membership_adjust_modal::tests::compute_effective_h2_july_1_boundary ... ok
test component::membership_adjust_modal::tests::compute_effective_h1_year_start ... ok
test component::membership_adjust_modal::tests::compute_effective_h2_year_end ... ok
test component::membership_adjust_modal::tests::format_date_german_simple ... ok
test component::membership_adjust_modal::tests::to_member_to_drops_pii_fields ... ok
test component::membership_adjust_modal::tests::format_date_german_year_end ... ok
test component::membership_adjust_modal::tests::voll_uebertrag_lt_returns_false ... ok
test component::membership_adjust_modal::tests::voll_uebertrag_eq_returns_true ... ok
test component::membership_adjust_modal::tests::voll_uebertrag_zero_shares_returns_false ... ok

test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 235 filtered out
```

## Acceptance Criteria Verification

### Positive Gates

| Criterion | Required | Found | Status |
|-----------|----------|-------|--------|
| `pub fn MembershipAdjustModal` | == 1 | 1 | PASS |
| `pub fn compute_effective_date_mirror` | == 1 | 1 | PASS |
| `pub fn is_voll_uebertrag` | == 1 | 1 | PASS |
| `pub fn to_member_to` | == 1 | 1 | PASS |
| `pub fn format_date_german` | == 1 | 1 | PASS |
| `enum ModalStep` | == 1 | 1 | PASS |
| ModalStep variants used | >= 10 | 18 | PASS |
| **SC-2:** `use_signal::<Option<time::Date>>(\|\| Some(today))` | == 1 | 1 | PASS |
| `r#type: "button"` (Dioxus reload-bug mitigation) | >= 8 | 17 | PASS |
| API calls (5 functions) | >= 5 | 5 | PASS |
| `members_override:` (Plan-03 integration) | >= 1 | 1 | PASS |
| `FiscalYearDateInput {` (Plan-04 integration, 4 sub-views) | >= 4 | 4 | PASS |
| `bg-blue-50 border border-blue-200 rounded` (preview box) | >= 4 | 4 | PASS |
| `text-orange-700` (Voll-Übertrag warning) | >= 1 | 1 | PASS |
| `i18n.clone()` (sub-function pattern) | >= 4 | 5 | PASS |

### Negative Gates (anti-patterns absent)

| Anti-pattern | Required | Found | Status |
|--------------|----------|-------|--------|
| `r#type: "submit"` (form-reload anti-pattern) | == 0 | 0 | PASS |
| `onsubmit` (form-submit anti-pattern) | == 0 | 0 | PASS |
| `t_format!` / `t_with_args` (non-existent macros — L-4) | == 0 | 0 | PASS |

### Task 2: component/mod.rs Re-Exports

| Criterion | Required | Found | Status |
|-----------|----------|-------|--------|
| `pub mod membership_adjust_modal;` | == 1 | 1 | PASS |
| `MembershipAdjustModal` mentions | >= 2 | 2 | PASS |
| `compute_effective_date_mirror` re-export | == 1 | 1 | PASS |
| `is_voll_uebertrag` re-export | == 1 | 1 | PASS |
| `to_member_to` re-export | == 1 | 1 | PASS |
| `format_date_german` re-export | == 1 | 1 | PASS |
| Plan-04 `pub mod fiscal_year_date_input;` unchanged | == 1 | 1 | PASS |
| Plan-04 `pub use toast::{show_success_toast` unchanged | == 1 | 1 | PASS |

### Build & Test Gates

- `cargo check --bin genossi-frontend` — exit 0 (50 pre-existing dead-code warnings on unused Key variants, no errors)
- `cargo test --bin genossi-frontend component::membership_adjust_modal` — **12 passed; 0 failed**

## SC-2 Default-today() — Verified

Line 124 of `membership_adjust_modal.rs`:
```rust
let date_signal = use_signal::<Option<time::Date>>(|| Some(today));
```

The `date_signal` is shared across all 4 sub-views (Cancel, PartialRepayment, Transfer, Upgrade) and persists when the user navigates back to SubChoice and re-enters a different sub-view (D-18 Claude's-Discretion). The FiscalYearDateInput component receives this signal directly via `value: date_signal` — the date input renders with today already pre-filled on every sub-view's first display.

## Plan 07 Page Integration — Unblocked

Plan 07 (member_details.rs page integration) can now:

```rust
use crate::component::{
    MembershipAdjustModal,
    Modal,
    show_success_toast, SuccessToastContainer,
};
use crate::auth::RequirePrivilege;

// In MemberDetails page body:
let mut show_adjust_modal = use_signal(|| false);
let today = /* computed via js_sys::Date */;

rsx! {
    RequirePrivilege {
        privilege: "admin",
        button {
            r#type: "button",
            onclick: move |_| show_adjust_modal.set(true),
            "{i18n.t(Key::MembershipAdjustButtonLabel)}"
        }
    }

    if *show_adjust_modal.read() {
        Modal {
            MembershipAdjustModal {
                member: member.read().clone(),
                today: today,
                on_close: move |_| show_adjust_modal.set(false),
                on_success: move |_| {
                    show_adjust_modal.set(false);
                    spawn(async move {
                        refresh_members().await;
                        show_success_toast(...);
                    });
                },
            }
        }
    }
}
```

## Sub-View Submit-Pattern Consistency

Each of the 4 operation sub-views uses the identical submit pattern (Dioxus reload-bug mitigation C-18-CF-03):

```rust
button {
    r#type: "button",  // NEVER "submit"
    class: "px-6 py-2 bg-red-600 hover:bg-red-700 text-white rounded font-semibold disabled:bg-gray-300 disabled:cursor-not-allowed",
    disabled: disabled,  // !is_valid || is_submitting
    onclick: move |_| {
        let Some(id) = member_id else { return; };
        let Some(d) = date_for_submit else { return; };
        submitting.set(true);
        error_signal.set(None);
        spawn(async move {
            let config = CONFIG.read().clone();
            match api::<operation>(...).await {
                Ok(_resp) => {
                    submitting.set(false);
                    on_success.call(());
                }
                Err(e) => {
                    submitting.set(false);
                    error_signal.set(Some(e));
                }
            }
        });
    },
    if is_submitting { "\u{2026}" } else { "{submit_label}" }
}
```

## TRSF-07 Self-Transfer Block (Frontend mirror)

The Transfer sub-view detects `recipient_id_val == Some(from_id)` and renders an orange inline error (`MembershipAdjustTransferSelfError`). The submit button is disabled in this state. The backend `TRSF-07` Service-Layer guard remains authoritative — frontend is defense-in-depth only.

## D-18-07 Voll-Übertrag Detection

The Transfer sub-view computes `is_voll_uebertrag(shares_now, current)` live on every render. When `true` AND the preview text is non-empty, an additional orange warning row appears INSIDE the preview box (`text-orange-700 font-bold`):

> ⚠ Voll-Übertrag — {from_name} tritt am {transfer_date} aus

**Does NOT block submit** — Vorstand may consciously trigger Voll-Übertrag (the backend creates 3 actions instead of 2 in that case).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] `current_shares` typed as `i32`, not `i64`**

- **Found during:** Task 1 (compile-time error before commit)
- **Issue:** The plan's reference snippet used `let current = member.current_shares as i32;` (implying a cast from `i64`). The actual `MemberTO.current_shares` field at `rest-types/src/lib.rs:210` is already `i32`. Casting `i32 as i32` triggers a clippy warning and is dead code.
- **Fix:** Removed `as i32` casts in all 4 sub-views. Used `let current = member.current_shares;` directly. Comparison arithmetic stays correct (signed `i32`).
- **Files modified:** `genossi-frontend/src/component/membership_adjust_modal.rs`
- **Commit:** 29d96b3

**2. [Rule 2 - Critical] `#[allow(clippy::too_many_arguments)]` on render-helpers**

- **Found during:** Task 1 (compile clean, but clippy would lint)
- **Issue:** The 4 `render_*_sub_view` functions take 8-10 args each (i18n + member + today + signals + handlers). Clippy default lints functions with >7 args.
- **Fix:** Added `#[allow(clippy::too_many_arguments)]` attribute. The arg count is intentional — these are private sub-view dispatchers, not public API. Refactoring into a struct would add no value because each arg is consumed exactly once.
- **Files modified:** `genossi-frontend/src/component/membership_adjust_modal.rs`
- **Commit:** 29d96b3

**3. [Rule 3 - Blocking] Removed unused import `PartialRepaymentResponseTO`/`TransferSharesResponseTO`**

- **Found during:** None — the original plan listed these in the `use rest_types::{...}` line, but they are only used as return types of the async API calls (where they're inferred). Importing them caused unused-import warnings.
- **Fix:** Pruned the use-list to only the types actually named in code: `MemberSlimTO, MemberStatusTO, MemberTO`.
- **Files modified:** `genossi-frontend/src/component/membership_adjust_modal.rs`
- **Commit:** 29d96b3

## Threat Surface

No new threat flags introduced. Plan's threat model (T-18-06-01..07) is fully addressed:

- T-18-06-01 (Self-Transfer-Bypass): mitigated via `inline_self_err` UI feedback + Backend TRSF-07 guard
- T-18-06-02 (Negative shares): mitigated via `inline_shares_err` + Backend D-15-06/D-16-07 range validation
- T-18-06-03 (PII disclosure): mitigated via `to_member_to` adapter dropping all PII fields
- T-18-06-04 (XSS): accepted — Dioxus RSX `"{preview_text}"` renders as text node, no HTML injection
- T-18-06-05 (Repudiation): accepted — backend audited_*! macros are authoritative
- T-18-06-06 (DoS via double-submit): mitigated via `submitting.set(true)` + button-disabled
- T-18-06-07 (Privilege escalation): accepted — backend endpoints are ADMIN_PRIVILEGE-protected

## Known Stubs

None. All 4 sub-views are wired end-to-end:
- Form inputs drive shared signals
- Live preview updates on every signal change
- Submit calls real API functions
- Success calls `on_success.call(())` (parent-controlled refresh + toast)
- Error displays via ErrorAlert (modal stays open for retry)

## Self-Check: PASSED

Files created:
- FOUND: `/home/neosam/programming/rust/projects/genossi3/genossi-frontend/src/component/membership_adjust_modal.rs` (1078 LOC)

Files modified:
- FOUND: `/home/neosam/programming/rust/projects/genossi3/genossi-frontend/src/component/mod.rs`

Commits in git log:
- FOUND: 29d96b3 (Task 1 — modal + tests)
- FOUND: 3a07cd1 (Task 2 — re-export)

Verifications:
- `cargo check --bin genossi-frontend` — exit 0
- `cargo test --bin genossi-frontend component::membership_adjust_modal` — 12 passed; 0 failed

---
*Phase: 18-frontend-component-first*
*Plan: 06*
*Completed: 2026-06-07*
