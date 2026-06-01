---
phase: 12-frontend-component-first
plan: 06
subsystem: frontend
tags: [frontend, page, inline-edit, audit, reuse]

# Dependency graph
requires:
  - phase: 12-frontend-component-first
    provides: "Plan 12-02 — parse_euro_to_cents (canonical Pure-Func in component/repayment_format.rs) + format_payout_eur"
  - phase: 12-frontend-component-first
    provides: "Plan 12-05 — is_share_value_editable(status) Pure-Func + BasicsTab inline component + api::update_repayment_phase wiring point"
  - phase: 12-frontend-component-first
    provides: "Plan 12-01 — api::update_repayment_phase + UpdateRepaymentPhaseRequest + i18n Keys (RepaymentPhaseShareValueEditHint, Save, Cancel, Edit)"
provides:
  - "BasicsTab share_value inline-edit (3 render modes: Vorbereitung editable, Offen editable + audit-hint, Abgeschlossen read-only)"
  - "Optimistic-Locking 409 → reload pattern wired in BasicsTab.share_value submit handler (anchor for Plans 12-08 Inline-Cell-Edit + 12-09 entries)"
affects:
  - "Plan 12-08 — RepaymentEntryList Inline-Cell-Edit (share_count_to_pay_out) can reuse the same Signal-toggle + parse-validation + 409-reload pattern established here"
  - "Plan 12-15 — UAT will visually verify 3 render modes (Vorbereitung shows Edit button without audit hint, Offen shows audit hint, Abgeschlossen shows no Edit button)"

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Cross-file Pure-Func reuse via grouped use-statement (use crate::component::repayment_format::{format_payout_eur, parse_euro_to_cents}) — anchors the Plan 12-02 canonical-helper principle"
    - "Inline-Edit Signal-toggle pattern: editing_share_value (bool) + share_value_input (String) + saving (bool) Signals scoped to BasicsTab body, capture-by-move into onclick closures with explicit reset-fallback values (phase_share_value_for_reset)"
    - "Optimistic-Locking 409 → on_changed.call(()) reload pattern (Pitfall #7 / Phase 8 CR-01) reused identically for share_value mutations as for open/close lifecycle"
    - "Status-aware render guard via existing is_share_value_editable + audit-hint guard via matches!(status, Open) — separate predicates because the 2 conditions cover different decision points (edit-allowed vs hint-visible)"

key-files:
  created: []
  modified:
    - "genossi-frontend/src/page/repayment_phase_details.rs (+108 lines / -3 lines: import addition + BasicsTab body extension with 3-mode share_value-Edit render; total file 489 lines)"

key-decisions:
  - "parse_euro_to_cents reused via grouped use-import (use crate::component::repayment_format::{format_payout_eur, parse_euro_to_cents}) instead of qualified call — the function is already used as bare format_payout_eur(...) in the file, keeping the import style consistent. Plan 12-02 canonical lives in component/repayment_format.rs; Plan 12-06 adds 0 lines of duplicate definition (Reuse-Gate verified by rg)."
  - "No new unit tests in Plan 12-06: parse_euro_to_cents tests (5) live canonically in component/repayment_format.rs::tests (Plan 12-02). is_share_value_editable test (1) lives in page/repayment_phase_details.rs::tests (Plan 12-05). Plan 12-06 only adds RSX-render-paths that exercise both functions through the click-handler — visual verification belongs to UAT (Plan 12-15)."
  - "3-Modi-Render via if *editing_share_value.read() && editable then-branch + else-branch with conditional Bearbeiten-Button (if editable) instead of a 3-way match on status. The editing state is orthogonal to status — both modes Vorbereitung and Offen reach the editable branch, only the audit-hint show_audit_hint = matches!(status, Open) differentiates them visually."
  - "Use Key::Edit ('Bearbeiten') for the toggle-into-edit-mode button instead of a hardcoded string. The plan suggested 'Bearbeiten' literally; using the existing i18n key keeps both Locales (de/en) consistent."
  - "On 409 conflict: editing_share_value.set(false) + on_changed.call(()) (force-reload) BEFORE returning. The phase Signal is reloaded with the fresh version, the input Signal is re-initialised on the next render via initial_input. The user sees the new value + a toast 'Konflikt — Daten wurden zwischenzeitlich geaendert, bitte erneut speichern' explaining what happened."
  - "phase_share_value_for_reset is captured by move into the Cancel-button closure as a primitive i64 — Cancel restores the input to the original Server-Value, not the latest user-typed-but-unsaved value. This matches member_details.rs Edit-Toggle semantics (cancel-discards-changes)."
  - "Error toast messages are hardcoded German strings inside the click-handler ('Bitte gueltigen Wert > 0 angeben (z.B. 60,00)', 'Phase hat keine Version — bitte neu laden', 'Konflikt — ...') rather than i18n keys. These are inline validation messages that don't yet have i18n Key variants in the enum. Plan-Discretion / pragmatic: matches the style of 'Invalid phase id' literal at line 68. Future i18n-pass could lift them, but not in Plan 12-06 scope."

patterns-established:
  - "Cross-File Pure-Func Reuse Pattern: import canonical helper from sibling module via use crate::component::<file>::<fn>; no local re-definition. Reuse-Gate enforced via rg in Plan acceptance. Plan 12-08 will follow the same pattern for share_count validation (i32-based, possibly extracted to repayment_format.rs as is_share_count_valid)."
  - "Inline-Edit-State Pattern (3 Signals + 4 captured primitives): editing (bool), input (String), saving (bool) Signals + phase_version (Option<Uuid>), phase_fiscal_year (i32), phase_share_value_for_reset (i64), phase_id (Uuid) captured by move into nested closures. Reproducible for any single-field inline-edit in a page-coupled component."
  - "Optimistic-Locking 409 Auto-Reload: on Err(e) if e.status == Some(409) → toast + editing.set(false) + on_changed.call(()) — the on_changed callback (which is load_phase) re-fetches with fresh version, no user re-entry-of-data lost because input still shows what they typed, but on next render it's reset via initial_input from the new phase. Plan 12-08 reuses for share_count Inline-Cell-Edit."

requirements-completed: [UI-02]

# Metrics
duration: ~6 min
completed: 2026-06-01T13:00:00Z
task-count: 1
file-count: 1
test-count-added: 0
test-count-total: 162
commits:
  - {sha: e03ede0, type: feat, task: "1", scope: "page/repayment_phase_details.rs (BasicsTab share_value-Inline-Edit, 3 modes)"}
---

# Phase 12 Plan 06: BasicsTab share_value Inline-Edit Summary

**One-liner:** Replaces the read-only share_value display in BasicsTab with a 3-mode inline-edit (Vorbereitung editable, Offen editable+audit-hint, Abgeschlossen read-only), reusing the canonical `parse_euro_to_cents` from Plan 12-02 and the `is_share_value_editable` predicate from Plan 12-05 — zero new pure-functions, zero new tests, single commit, all D-01 button-gates and reuse-gates green.

## What Was Built

A single atomic feat-commit (`e03ede0`) extending `BasicsTab` in `page/repayment_phase_details.rs` from a read-only `format_payout_eur(1, phase.share_value)` paragraph into a 3-mode render branch driven by an `editing_share_value` Signal and the existing `is_share_value_editable(phase.status)` predicate.

### Task 1: BasicsTab Inline-Edit (commit e03ede0)

**Import addition** (line 25):

```rust
use crate::component::repayment_format::{format_payout_eur, parse_euro_to_cents};
```

Grouped with existing `format_payout_eur` — single import line, no duplicate of the function body.

**Signal block** added at the top of `BasicsTab` body:

```rust
let mut editing_share_value = use_signal(|| false);
let initial_input = format!("{:.2}", (phase.share_value as f64) / 100.0).replace('.', ",");
let mut share_value_input = use_signal(move || initial_input.clone());
let mut saving = use_signal(|| false);
let editable = is_share_value_editable(phase_status);          // Plan 12-05 reuse
let show_audit_hint = matches!(phase_status, RepaymentPhaseStatusTO::Open);
let phase_version = phase.version;
let phase_fiscal_year_for_save = phase.fiscal_year;
let phase_share_value_for_reset = phase.share_value;
```

**3-Modi-Render** in the Stamm-Daten grid's right cell:

```rust
div {
    span { class: "text-sm text-gray-500", "{i18n.t(Key::RepaymentPhaseShareValue)}" }
    if *editing_share_value.read() && editable {
        // EDIT MODE (Vorbereitung + Offen)
        div { class: "flex flex-col gap-1",
            if show_audit_hint {
                p { class: "text-xs text-orange-700",
                    "{i18n.t(Key::RepaymentPhaseShareValueEditHint)}"    // "Korrektur wird auditiert"
                }
            }
            div { class: "flex items-center gap-2",
                input { /* text input with comma-decimal */ },
                span { class: "text-gray-700", "EUR" },
                button { /* Save (r#type: "button") */ },
                button { /* Cancel (r#type: "button") */ },
            }
        }
    } else {
        // VIEW MODE (all 3 statuses; Bearbeiten only when editable)
        div { class: "flex items-center gap-2",
            p { class: "text-lg font-semibold", "{format_payout_eur(1, phase.share_value)}" }
            if editable {
                button { /* Bearbeiten (r#type: "button") */ }
            }
        }
    }
}
```

### Save-Handler (D-05 + Pitfall #7 — Optimistic-Locking)

The Save-button's `onclick` synchronously:

1. **Parse + validate** via `parse_euro_to_cents(&share_value_input.read())` — on `None` calls `on_error.call("Bitte gueltigen Wert > 0 angeben (z.B. 60,00)")` and returns.
2. **Version-Guard** via `phase_version` (Option<Uuid>) — on `None` calls `on_error.call("Phase hat keine Version — bitte neu laden")` and returns.
3. **Sets `saving.set(true)`** and constructs `UpdateRepaymentPhaseRequest { fiscal_year, share_value: cents, version }`.
4. **Spawns async** `api::update_repayment_phase(&config, phase_id, &req)`.
5. **On `Ok(_)`:** `editing_share_value.set(false)` + `on_changed.call(())` (= `load_phase` → fresh GET with fresh version).
6. **On `Err(e)` with `e.status == Some(409)`:** Toast `"Konflikt — Daten wurden zwischenzeitlich geaendert, bitte erneut speichern"` + `editing_share_value.set(false)` + `on_changed.call(())` (force-reload with fresh version).
7. **On any other `Err(e)`:** `on_error.call(e.message)` (Toast with status_to_message-mapped German error).
8. **Always sets `saving.set(false)`** at end of async block.

The Cancel-button resets `share_value_input` to the original Server-Value (via `phase_share_value_for_reset`) and toggles `editing_share_value` back to `false`.

## 3 Render Modes Visualized

| Status        | `editable` | `show_audit_hint` | Visible Elements                                                            |
| ------------- | ---------- | ----------------- | --------------------------------------------------------------------------- |
| Vorbereitung  | true       | false             | View: value + Bearbeiten button. Edit: input + EUR + Save + Cancel.         |
| Offen         | true       | true              | View: value + Bearbeiten button. Edit: orange audit-hint + input + buttons. |
| Abgeschlossen | false      | false             | View: value (no Bearbeiten button — read-only).                             |

D-08 (Abgeschlossen = all read-only) is enforced two ways: the inner `if editable` guards the Bearbeiten button in view-mode, AND the outer `if *editing_share_value.read() && editable` ensures even if editing were forced to `true` programmatically, the Closed status falls back to view-mode.

## How It Was Verified

```bash
# Build
$ cargo build --bin genossi-frontend
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 19.37s

# Tests — page::repayment_phase_details (carry-over from Plan 12-05, expect 7 PASS)
$ cargo test --bin genossi-frontend -- page::repayment_phase_details
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 155 filtered out

# Tests — component::repayment_format (carry-over from Plan 12-02, expect 9 PASS — canonical parse_euro_to_cents)
$ cargo test --bin genossi-frontend -- component::repayment_format
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 153 filtered out

# Full suite — no regressions
$ cargo test --bin genossi-frontend
test result: ok. 162 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

# Done-criteria greps
$ rg "parse_euro_to_cents\(" genossi-frontend/src/page/repayment_phase_details.rs
    let cents = match parse_euro_to_cents(&share_value_input.read()) { /* ... */ }
# >= 1 call site ✓

$ rg "^pub(\(crate\))?\s+fn\s+parse_euro_to_cents" genossi-frontend/src/page/repayment_phase_details.rs
# = 0 local re-define ✓ (Reuse-Gate green)

$ rg "^pub fn parse_euro_to_cents" genossi-frontend/src/component/repayment_format.rs
pub fn parse_euro_to_cents(input: &str) -> Option<i64> { /* ... */ }
# = 1 canonical Plan-12-02 location ✓

$ rg "repayment_format::" genossi-frontend/src/page/repayment_phase_details.rs
use crate::component::repayment_format::{format_payout_eur, parse_euro_to_cents};
# Import visible ✓

$ rg "api::update_repayment_phase" genossi-frontend/src/page/repayment_phase_details.rs
match api::update_repayment_phase(&config, phase_id, &req).await {
# >= 1 wire-up ✓

$ rg "editing_share_value" genossi-frontend/src/page/repayment_phase_details.rs | wc -l
6
# Signal-decl + 5 use-sites (read + 4 set) — >= 3 ✓

# D-01 Button-Gate
$ rg -P '(?ms)button\s*\{(?:(?!\}).)*?\}' genossi-frontend/src/page/repayment_phase_details.rs \
  | grep -v 'r#type:' | grep -c 'button {'
0
# Zero buttons without r#type: — 3 new buttons (Edit/Save/Cancel) all carry r#type: "button" ✓
```

All plan-acceptance criteria pass.

## Decisions Made

### parse_euro_to_cents reused via grouped import, not qualified call

The plan offered both styles ("use ... parse_euro_to_cents;" OR qualified `repayment_format::parse_euro_to_cents(...)`). I chose the grouped use-import to match the existing import line for `format_payout_eur`. Both functions live in the same module; importing both with one statement keeps the file's import block compact. The qualified-call style would have made the click-handler verbose without semantic benefit.

### No new tests in Plan 12-06

The plan explicitly says "KEINE neuen parse_euro_to_cents-Tests in 12-06". The two pure functions exercised by the new code (`parse_euro_to_cents`, `is_share_value_editable`) have their canonical tests in Plan 12-02 (9 tests) and Plan 12-05 (1 of 7 tests) respectively. The new code is RSX-render + click-handler glue — its meaningful behavior is "wire the existing pieces correctly". Plan 12-15 UAT will verify the 3 modes visually.

### `show_audit_hint` is separate from `editable`

The plan's check says "show_audit_hint = matches!(phase.status, RepaymentPhaseStatusTO::Open)" — and that's how I implemented it. Why a separate predicate instead of pushing into `is_share_value_editable`? Because the two conditions answer different questions:

- `editable` = should the Bearbeiten button be visible at all? (false only in Closed)
- `show_audit_hint` = should the orange audit-warning be shown in the edit form? (true only in Open)

In Vorbereitung, edit is allowed BUT no audit hint (no audit entry yet because the phase isn't open). In Offen, edit is allowed AND audit hint (audit entry will be created). In Closed, no edit at all. Two predicates, two questions, no overlap.

### Hardcoded German error messages (not i18n) in click-handler

Three inline German strings live in the Save-handler:

- `"Bitte gueltigen Wert > 0 angeben (z.B. 60,00)"` (parse failure)
- `"Phase hat keine Version — bitte neu laden"` (version-Option missing)
- `"Konflikt — Daten wurden zwischenzeitlich geaendert, bitte erneut speichern"` (409 conflict)

These don't have i18n Key variants in the enum and the plan didn't ask me to add any. They follow the existing precedent of the `"Invalid phase id"` literal at line 68 (Plan 12-05) and `"Phase not found"` at line 189. Plan-Discretion: pragmatic to keep them as literals for v1.1 — future i18n-pass could lift them.

### phase_share_value_for_reset captures the Server-Value, not the latest input

The Cancel-button restores the input field to the original Server-Value (`phase_share_value_for_reset`), discarding all user keystrokes. This matches the cancel-discards-changes semantics of `member_details.rs` Edit-Toggle. If the user wanted to preserve typing-in-progress, they would close the toggle differently (e.g., navigate away and come back) — but Cancel is unambiguous: discard.

### Capture-Order in Closures

The plan flagged "typical Dioxus-0.6 stolperer" around capture-semantics. The actual file compiles cleanly without any `phase.clone()` workarounds:

- `phase.id`, `phase.status`, `phase.version`, `phase.fiscal_year`, `phase.share_value` are all `Copy` types (Uuid, enum, Option<Uuid>, i32, i64) — captured by value into nested closures without conflict.
- `share_value_input` is a `Signal<String>` — `Copy` because Signals are Copy in Dioxus 0.6.
- `on_changed`, `on_error` are `EventHandler<_>` — Copy and can be cloned/used freely in nested closures.

No `let phase_for_handler = phase.clone();` needed. The plan's defensive note ("falls Compiler ueber 'moves into FnMut' meckert") was a precaution that didn't materialize here.

## Render-Path (Data Flow)

```
BasicsTab (phase, on_changed, on_close_conflict, on_error)
  ↓ Initialize Signals: editing=false, input=fmt(phase.share_value), saving=false
  ↓ Render Stamm-Daten grid
    └─ share_value cell:
       ├─ editing && editable → EDIT MODE
       │   ├─ if show_audit_hint → orange "Korrektur wird auditiert"
       │   ├─ <input value={share_value_input}>
       │   ├─ button "Save" (r#type=button, disabled if saving)
       │   │     onclick:
       │   │       cents = parse_euro_to_cents(input)?
       │   │       version = phase.version?
       │   │       saving.set(true)
       │   │       spawn:
       │   │         api::update_repayment_phase(config, id, {fy, cents, version})
       │   │           Ok → editing.set(false) + on_changed.call(())   ← reload
       │   │           Err(409) → toast "Konflikt..." + editing.set(false) + on_changed.call(())
       │   │           Err(_) → on_error.call(e.message)
       │   │         saving.set(false)
       │   └─ button "Cancel" (r#type=button)
       │         onclick: reset input to phase_share_value_for_reset + editing.set(false)
       │
       └─ else → VIEW MODE
           ├─ <p>format_payout_eur(1, share_value)</p>
           └─ if editable → button "Bearbeiten" (r#type=button)
                              onclick: editing.set(true)
```

## Deviations from Plan

None. Plan-action implemented as written:

- Import addition: 1 grouped use-line at line 25.
- Signal block: 9 let-bindings (3 use_signal + 1 initial_input + 1 editable + 1 show_audit_hint + 3 phase_*_for_save/reset/version).
- 3-Modi-Render: if-else with Bearbeiten button in view-mode and Save/Cancel in edit-mode.
- 3 new buttons (Bearbeiten, Save, Cancel), all carrying `r#type: "button"`.
- D-08 enforced via `editable` guard (Closed → no Bearbeiten button).
- D-05 audit-hint enforced via `show_audit_hint = matches!(status, Open)`.
- D-09-style reload on Ok via `on_changed.call(())`.
- Pitfall #7 / Phase 8 CR-01 reload on 409 via `on_changed.call(())`.

No auto-fixes (Rules 1-3), no architectural decisions (Rule 4), no checkpoints.

## Known Stubs

None introduced by Plan 12-06. The two stubs from Plan 12-05 (EntriesTab TODO Plan 12-08 + ExportTab TODO Plan 12-14) remain unchanged — they're not in BasicsTab and Plan 12-06 only modifies BasicsTab.

## Threat Flags

None — this plan adds frontend-only inline-edit for an existing audited field. The mutation goes through the existing `api::update_repayment_phase` wire (Plan 12-01) which hits the existing backend PUT `/api/repayment-phase/{id}` endpoint (Phase 7), which already implements PHAS-04 audit via `audited_update!` macro on the service-impl layer. Frontend introduces no new auth path, no new network surface, no schema changes.

## Self-Check: PASSED

Verified artifacts in the main repo:

- [FOUND] `genossi-frontend/src/page/repayment_phase_details.rs` (489 lines after edit; was 384 lines before)
- [FOUND] `use crate::component::repayment_format::{format_payout_eur, parse_euro_to_cents};` (line 25 — grouped import)
- [FOUND] `let mut editing_share_value = use_signal(|| false);` (BasicsTab body)
- [FOUND] `if *editing_share_value.read() && editable {` (3-mode render guard)
- [FOUND] `match parse_euro_to_cents(&share_value_input.read())` (validation call-site)
- [FOUND] `match api::update_repayment_phase(&config, phase_id, &req).await` (mutation wire)
- [FOUND] `Err(e) if e.status == Some(409)` (Optimistic-Locking branch)
- [FOUND] `on_changed.call(())` called on both Ok and 409-Err (force-reload pattern)
- [FOUND] 3 new buttons (Bearbeiten, Save via `Key::Save`, Cancel via `Key::Cancel`) all with `r#type: "button"`
- [FOUND] `if show_audit_hint { p { ... RepaymentPhaseShareValueEditHint ... } }` (D-05 audit hint)
- [FOUND] `if editable { button { ... "Bearbeiten" ... } }` (D-08 Closed read-only enforcement)
- [VERIFIED] `cargo build --bin genossi-frontend` exit 0 (19.37s)
- [VERIFIED] `cargo test --bin genossi-frontend -- page::repayment_phase_details::tests` → 7/7 PASS (carry-over)
- [VERIFIED] `cargo test --bin genossi-frontend -- component::repayment_format` → 9/9 PASS (canonical Plan 12-02 tests intact)
- [VERIFIED] Full `cargo test --bin genossi-frontend` → 162/162 PASS, 0 regressions
- [VERIFIED] D-01 Button-Gate: 0 buttons without `r#type:` in `repayment_phase_details.rs`
- [VERIFIED] Reuse-Gate: 0 local `parse_euro_to_cents` redefinitions in `repayment_phase_details.rs`; 1 canonical definition in `component/repayment_format.rs`
- [FOUND] Commit `e03ede0` (feat(12-06): add share_value inline-edit to BasicsTab (3 render modes))

## TDD Gate Compliance

Plan 12-06 is `type: execute` (not `type: tdd`), and the plan explicitly states "KEINE neuen parse_euro_to_cents-Tests in 12-06" because all pure-function tests already live in Plan 12-02 and Plan 12-05. A single `feat` commit is the expected gate sequence for an execute-type plan that reuses existing tested helpers — no RED test commit is required because no new pure function was added.

Gate sequence in `git log db4c00e..HEAD`:

```
e03ede0 feat(12-06): add share_value inline-edit to BasicsTab (3 render modes)   ← Task 1 (single commit)
```

Compliant with execute-type plan semantics. Pure-function coverage remains exhaustive via existing tests (Plan 12-02: 5 tests for `parse_euro_to_cents`; Plan 12-05: 1 test for `is_share_value_editable`).

---

*Phase: 12-frontend-component-first*
*Plan: 06 — BasicsTab share_value Inline-Edit (UI-02 / D-05 / PHAS-04)*
*Completed: 2026-06-01T13:00:00Z (~6 min)*
