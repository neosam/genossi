---
phase: 12-frontend-component-first
plan: 05
subsystem: frontend
tags: [frontend, page, detail, tabs, lifecycle, tdd, component-first]

# Dependency graph
requires:
  - phase: 12-frontend-component-first
    provides: "Plan 12-01 — api::get/open/close_repayment_phase + RepaymentPhaseTO + RepaymentPhaseStatusTO + CloseConflictResponse + AppError"
  - phase: 12-frontend-component-first
    provides: "Plan 12-02 — RepaymentPhaseStatusBadge + format_payout_eur"
  - phase: 12-frontend-component-first
    provides: "Plan 12-03 — Route::RepaymentPhaseDetails registered; stub page exists"
provides:
  - "#[component] pub fn RepaymentPhaseDetails(id: String) -> Element — Detail-Page UI-02"
  - "Inline BasicsTab #[component] with Lifecycle-Action-Tile (Öffnen + Schließen + Confirm-Modal)"
  - "EntriesTab + ExportTab as inline-fn stubs with status-driven 'Phase noch nicht geöffnet' hint"
  - "parse_close_conflict(err: &AppError) -> Option<CloseConflictResponse> — reusable 409-body-parse pattern (relevant for Plan 12-10 BatchFailureResponse handling)"
  - "is_share_value_editable(status) — pub(crate) helper Plan 12-06 reuses for inline-edit guard"
affects:
  - "Plan 12-06 — share_value-Inline-Edit replaces read-only display in BasicsTab; reuses is_share_value_editable"
  - "Plan 12-08 — RepaymentEntryList replaces EntriesTab TODO-stub body"
  - "Plan 12-14 — Export-Tab Include-Filter + PDF-Download replaces ExportTab TODO-stub body"
  - "Plan 12-10 — BatchFailureResponse handling can reuse parse_close_conflict's pattern (status+detail-body deserialize)"

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Pure-Function-Helper (status predicates + parse_close_conflict) in page-file with #[cfg(test)] mod tests — analog to filter_members / sort_phases_default Pattern"
    - "TDD RED→GREEN sequence for both Task 1 and Task 2 (separate test/feat commits)"
    - "TabStrip with fixed tab-set (D-06) — first detail-page that does NOT branch the tab list on status"
    - "409 detail-body deserialization via serde_json::from_str on AppError.detail — reusable pattern for 12-10 (BatchFailureResponse)"
    - "Optimistic-Locking reload via on_changed callback (Phase 8 CR-01 anchor)"
    - "ErrorAlert with detail-expand for structured 409 responses (vs Toast-only for generic errors)"

key-files:
  created: []
  modified:
    - "genossi-frontend/src/page/repayment_phase_details.rs (384 lines: full detail page + BasicsTab inline + parse_close_conflict + 7 tests; replaces 13-line stub from Plan 12-03)"

key-decisions:
  - "BasicsTab as inline-component in page file (NOT extracted to component/repayment_basics_tab.rs): Plan-Discretion (plan calls this optional). Component is ~120 LOC, only consumed by RepaymentPhaseDetails — single caller justifies inline. Plan 12-06 will extend it inline. Component-First is NOT violated because the component is locally defined and not duplicated across pages."
  - "parse_close_conflict as pure-function helper with 4 unit tests (non-409, missing detail, valid body, garbled body) — reusable test-anchor for Plan 12-10's BatchFailureResponse pattern."
  - "TabStrip uses fixed 3-tab list IMMER (D-06) instead of conditional tab-push like assembly_details.rs's 4th tab. Vorstand learns the layout once across all phase states."
  - "ErrorAlert with detail-expand (showing pending member numbers) instead of Toast-only for the 409 CloseConflictResponse case (D-04 + Open-Question 5 resolution). Toast fallback for non-409 errors and for garbled 409 bodies (parse_close_conflict returns None)."
  - "After open/close success, on_changed = || load_phase() re-fetches the phase from the backend (Phase 8 CR-01 anchor). The Response-body's version is NOT trusted — backend may have bumped version atomically after the service-layer returned its stale copy."
  - "D-09: No auto-tab-switch after open. on_changed only reloads phase state; active_tab stays on 'basics'. Vorstand navigates to 'entries' manually."

patterns-established:
  - "Status-Predicate Functions: pure fn should_show_X(status) -> bool via matches!() — Plan 12-06 will use is_share_value_editable for the inline-edit guard, Plans 12-08+ may add similar predicates for entry-status-driven render"
  - "409 Detail-Body Parse: if err.status == Some(409) && err.detail.is_some() then serde_json::from_str::<TypedResponse>(detail).ok() — Plan 12-10 reuses for BatchFailureResponse"
  - "Inline Sub-Component Pattern: Page-local #[component] sub-component (BasicsTab here, TokensTab in assembly_details.rs) where the sub-component is page-coupled and not shared across pages — does NOT violate Component-First"
  - "TODO-Stub-Tab-Body: stub-text 'TODO Plan 12-XX: <component-name> für phase_id={id}' as marker for future plans to grep-replace (RepaymentEntryList for 12-08, Export-Tab for 12-14)"

requirements-completed: [UI-02]

# Metrics
duration: ~7 min
completed: 2026-06-01T12:15:04Z
task-count: 2
file-count: 1
test-count-added: 7
test-count-total: 162
commits:
  - {sha: 7fe332d, type: test, task: "1 RED", scope: "page/repayment_phase_details.rs (status predicate tests)"}
  - {sha: e54b09c, type: feat, task: "1 GREEN", scope: "page/repayment_phase_details.rs (matches!() implementations)"}
  - {sha: daeeca8, type: test, task: "2 RED", scope: "page/repayment_phase_details.rs (parse_close_conflict tests)"}
  - {sha: b019b33, type: feat, task: "2 GREEN", scope: "page/repayment_phase_details.rs (full detail page + BasicsTab inline)"}
---

# Phase 12 Plan 05: Repayment Phase Detail Page (UI-02) Summary

**One-liner:** Full 3-Tab detail page for `/repayment-phases/{id}` with inline BasicsTab (Lifecycle Open/Close + Confirm-Modal), parse_close_conflict pattern for 409 body-deserialization (anchor for Plan 12-10), and status-driven render predicates that Plan 12-06 reuses for the share_value inline-edit guard.

## What Was Built

Two TDD-tracked tasks committed as 4 atomic commits — Task 1 RED + GREEN for the pure status predicates, Task 2 RED + GREEN for the parse_close_conflict helper plus the full detail page UI on top.

### Task 1: Status-driven render predicates (commits 7fe332d → e54b09c)

Three `matches!()`-based pure functions inside `page/repayment_phase_details.rs`:

- `should_show_open_button(status) -> bool` — true only for `Preparation` (D-03 + D-08)
- `should_show_close_button(status) -> bool` — true only for `Open` (D-03 + D-08)
- `is_share_value_editable(status) -> bool` — false only for `Closed` (D-05 + D-08)

`is_share_value_editable` is marked `pub(crate)` because Plan 12-06's share_value-Inline-Edit will reuse it directly — single canonical guard, no duplicate matches scattered across page files.

**Test coverage:** 3/3 PASS (one test per predicate, asserting all three status variants exhaustively).

### Task 2: Detail-Page UI + parse_close_conflict (commits daeeca8 → b019b33)

Replaces the 13-line Plan-12-03 stub with a 384-line full detail-page implementation:

**`RepaymentPhaseDetails(id: String)`** — public page component, mounted under `/repayment-phases/:id`:

- Parses `id: String` via `Uuid::from_str` (early-return red error on parse failure, same as `assembly_details.rs`)
- `RequirePrivilege { privilege: "admin", fallback: AccessDeniedPage }` — Vorstand-only (D-25)
- `TopBar` + container with three render branches:
  - **Loading:** `p { ... "{i18n.t(Key::Loading)}" }` placeholder
  - **Loaded:** Header (title with `fiscal_year` + `RepaymentPhaseStatusBadge`) + optional 409 ErrorAlert + `TabStrip` body
  - **Not-found:** centered red "Phase not found" text
- D-04 + Open-Question 5: when `close_conflict: Signal<Option<CloseConflictResponse>>` is `Some`, render an `ErrorAlert` with `pending_count` in message and member-number list in expandable detail (reuses `ErrorAlert`'s built-in `Details anzeigen`/`Details ausblenden` toggle)
- D-06: `TabStrip` with three FIXED `TabDef`s — Basics / Entries / Export — visible in ALL phase statuses (no conditional 4th tab like `assembly_details.rs`)
- Tab body branches on `active_key`:
  - `"basics"` → `BasicsTab` (always rendered)
  - `"entries"` in `Preparation` → "Phase noch nicht geöffnet" hint (i18n `RepaymentEntriesNotOpenYet`)
  - `"entries"` in `Open`/`Closed` → `TODO Plan 12-08: RepaymentEntryList für phase_id={id}` stub
  - `"export"` in `Preparation` → "Phase noch nicht geöffnet" hint (i18n `RepaymentExportNotOpenYet`)
  - `"export"` in `Open`/`Closed` → `TODO Plan 12-14: Export-Tab für phase_id={id}` stub
- `ToastContainer` as sibling for transient API errors

**`BasicsTab`** — inline page-local `#[component]` (not extracted to `component/`):

- Stamm-Daten grid: `fiscal_year` + `share_value` (formatted via `format_payout_eur(1, phase.share_value)`) — read-only in this wave; Plan 12-06 adds the inline-edit
- D-03: **Lifecycle-Action-Tile (only)** as large rounded cards:
  - Öffnen-Tile (`should_show_open_button`): blue card with explanation "Beim Öffnen werden alle Vorjahres-Austritte als Einträge angelegt"; button calls `api::open_repayment_phase` directly, no confirm (D-07 says Öffnen has no confirm because it's reversible via entry-edits)
  - Schließen-Tile (`should_show_close_button`): red card with explanation "Schließen geht nur, wenn alle Einträge ausbezahlt oder gelöscht sind"; button opens Confirm-Modal (D-07)
- D-07 Confirm-Modal: title + text via i18n (`RepaymentPhaseCloseConfirmTitle/Text`), Cancel + red Schließen button; the red Schließen button:
  - Closes the modal first (synchronous)
  - Calls `api::close_repayment_phase` async
  - On success → `on_changed.call(())` triggers a re-fetch (Phase 8 CR-01)
  - On error → `parse_close_conflict(&err)` — if `Some(cc)` then `on_close_conflict.call(cc)` (renders ErrorAlert with details); else `on_error.call(err.message)` (Toast fallback)

**`parse_close_conflict(err: &AppError) -> Option<CloseConflictResponse>`** — pure helper with 4 unit tests:

```rust
fn parse_close_conflict(err: &AppError) -> Option<CloseConflictResponse> {
    if err.status != Some(409) {
        return None;
    }
    let body = err.detail.as_deref()?;
    serde_json::from_str::<CloseConflictResponse>(body).ok()
}
```

Returns `None` for non-409, missing detail, AND garbled JSON. Plan 12-10 will reuse this pattern for `BatchFailureResponse` deserialization (also a 409 with structured body).

**D-09 anchor:** Both Open and Close success-paths call `on_changed.call(())` — which is `load_phase` (re-fetch from backend). The handler does **not** call `active_tab.set(...)`. Tab stays on 'basics' after open; Vorstand navigates to 'entries' manually.

**D-01 Button-Gate:** all 4 button-tags (Öffnen, Schließen, Cancel-confirm, Bestätigen-Schließen) carry `r#type: "button"` explicitly. The plus an ErrorAlert dismiss-button (from the `ErrorAlert` component itself — Phase 4 D-01 compliant).

**Test coverage Task 2:** 4/4 PASS (non-409, missing-detail, valid-body, garbled-body).

**Combined test coverage:** 7/7 PASS in `page::repayment_phase_details::tests`. Full suite: 162/162 PASS.

## Render-Path (Data Flow)

```
RepaymentPhaseDetails (mount)
  ↓ use_effect → load_phase
  ↓ spawn → api::get_repayment_phase(&config, phase_id)
  ↓ Ok(p) → phase.set(Some(p))
  ↓ Render-Tree:
    RequirePrivilege
      TopBar
      Container
        Branch: Loading | Loaded | NotFound
          Loaded:
            Header (title + RepaymentPhaseStatusBadge)
            Optional: ErrorAlert {close_conflict 409 details}
            TabStrip
              "basics" → BasicsTab (status-aware Lifecycle-Tile)
                → Öffnen click → spawn api::open_repayment_phase → Ok → on_changed (= load_phase)
                                                                  → Err → on_error (Toast)
                → Schließen click → show_close_confirm.set(true)
                  → Modal Confirm → spawn api::close_repayment_phase
                                  → Ok → on_changed (= load_phase)
                                  → Err → parse_close_conflict
                                          → Some(cc) → on_close_conflict (ErrorAlert)
                                          → None → on_error (Toast)
              "entries" → {Preparation: hint | Open/Closed: TODO Plan 12-08 stub}
              "export"  → {Preparation: hint | Open/Closed: TODO Plan 12-14 stub}
      ToastContainer
```

## How It Was Verified

```bash
# Test results
$ cd genossi-frontend && cargo test --bin genossi-frontend -- page::repayment_phase_details::tests
test page::repayment_phase_details::tests::close_button_only_in_open ... ok
test page::repayment_phase_details::tests::parse_close_conflict_returns_none_on_non_409 ... ok
test page::repayment_phase_details::tests::parse_close_conflict_returns_none_when_detail_missing ... ok
test page::repayment_phase_details::tests::parse_close_conflict_returns_none_on_garbled_body ... ok
test page::repayment_phase_details::tests::open_button_only_in_preparation ... ok
test page::repayment_phase_details::tests::parse_close_conflict_returns_some_on_valid_body ... ok
test page::repayment_phase_details::tests::share_value_readonly_in_closed ... ok
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 155 filtered out

$ cargo test --bin genossi-frontend
test result: ok. 162 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

# Done-criteria greps
$ rg "TODO Plan 12-05" genossi-frontend/src/page/repayment_phase_details.rs
(no output — 0 occurrences, expected after stub-replacement)

$ rg "TabStrip \{|TabDef \{" genossi-frontend/src/page/repayment_phase_details.rs | wc -l
4   # 1 TabStrip mount + 3 TabDef constructions

$ rg "api::get_repayment_phase|api::open_repayment_phase|api::close_repayment_phase" genossi-frontend/src/page/repayment_phase_details.rs | wc -l
3   # all 3 lifecycle API calls wired

$ rg "RepaymentPhaseStatusBadge \{" genossi-frontend/src/page/repayment_phase_details.rs | wc -l
1   # header badge

$ rg "parse_close_conflict" genossi-frontend/src/page/repayment_phase_details.rs | wc -l
10   # 1 def + 1 call-site + 4 test-fn names + 4 doc/test invocations — well >= 2

$ rg "show_close_confirm" genossi-frontend/src/page/repayment_phase_details.rs | wc -l
5   # signal decl + 4 use-sites — >= 3

$ rg "RequirePrivilege" genossi-frontend/src/page/repayment_phase_details.rs | wc -l
2   # import + use

# D-01 Button-Gate (zero buttons without r#type:)
$ rg -P '(?ms)button\s*\{(?:(?!\}).)*?\}' genossi-frontend/src/page/repayment_phase_details.rs \
    | grep -v 'r#type:' | grep -c 'button {'
0
```

All plan-acceptance criteria pass.

## Decisions Made

### BasicsTab as inline-component (not extracted to `component/`)

The plan explicitly calls this Plan-Discretion: "Optional-Verfeinerung: BasicsTab könnte in eigene Datei `genossi-frontend/src/component/repayment_basics_tab.rs` extrahiert werden. Aktueller Inline-Stil ist akzeptabel". I chose inline because:

- The component is ~120 LOC — small enough to comprehend in-page
- Only one caller (RepaymentPhaseDetails) — extraction adds indirection without reuse-benefit
- Plan 12-06 will extend it inline (share_value-Inline-Edit) — extraction now would just delay that edit
- `assembly_details.rs` has the same pattern with its `TokensTab` inline sub-component as anchor

Component-First is NOT violated because BasicsTab is locally defined and not duplicated across pages. If a second page ever needs the same lifecycle-tile, Plan 12+ can extract then.

### parse_close_conflict as pure-function (Pattern for Plan 12-10)

The plan asks for parse_close_conflict to be a pure function with 4 unit tests. This is intentional — Plan 12-10 (PaidOut-Confirm with BatchFailureResponse) will need an identical 409+detail-body deserialize pattern. By naming, signature, and test-style being explicit here, Plan 12-10 can grep for parse_close_conflict and clone the pattern to parse_batch_failure. This is the same "test-as-reference-doc" pattern Plan 12-02 established with format_payout_eur.

### D-09 No-Auto-Tab-Switch

After Öffnen-Success, the page does NOT auto-switch to the Einträge-Tab. Rationale: cross-component tab-state mutation is structurally complex (would need to pipe `active_tab.set(...)` into the `on_changed` callback chain). The plan prefers simplicity — Vorstand clicks 'Einträge' tab manually after Öffnen. The visible state-change (status badge flips from Vorbereitung to Offen) is enough feedback.

### Optimistic-Locking Reload via on_changed (Phase 8 CR-01 anchor)

Both `api::open_repayment_phase` and `api::close_repayment_phase` return the updated `RepaymentPhaseTO` body, but we discard it and call `load_phase` → `api::get_repayment_phase` again. Rationale: Phase 8 Plan 10 documented that the backend bumps `version` atomically in the DAO, but the service-layer returns the **stale local entity**. Trusting the Response-body's version would cause 409-conflicts on the next mutation. Re-fetch is the safe pattern. Plan 12-06 (share_value-Inline-Edit) must follow the same pattern.

### Test-count: 7 instead of plan-stated 6

The plan's done-criterion says "6 PASS (3 status + 3 parse)" but the action-block lists 4 parse_close_conflict tests (non-409, missing-detail, valid-body, garbled-body). I implemented all 4 because they cover the meaningful branches. The "6 PASS" was a minor plan-write inconsistency; "≥ 6" is what matters and 7 satisfies it.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Worktree git-toplevel resolves to main repo — wrong file paths committed initially**

- **Found during:** Task 1 RED (initial commit attempt)
- **Issue:** Spawn cwd is `.claude/worktrees/agent-aa326f655b0036f15/` which is gitignored. When I edited `genossi-frontend/src/page/repayment_phase_details.rs` at that cwd, the file was created under `.claude/worktrees/.../genossi-frontend/...` in the working tree, but git's toplevel resolved to the main repo (`/home/neosam/programming/rust/projects/genossi3`). My first commit (`git add -f genossi-frontend/...`) landed the file at the worktree subpath inside the repo, not at the canonical project path. Plan 12-03 SUMMARY documented this exact issue: "Alle Edits wurden direkt im Main-Repo-Pfad ausgeführt".
- **Fix:** Reset the first misplaced commit (`git reset --soft HEAD~1; git restore --staged .`), then performed all edits at `/home/neosam/programming/rust/projects/genossi3/genossi-frontend/src/page/repayment_phase_details.rs` (main repo absolute path). All 4 final commits land at the canonical path. The local worktree-only edit to `Cargo.toml` (adding `[workspace]` table for the worktree build) was reverted before committing.
- **Files modified:** `genossi-frontend/src/page/repayment_phase_details.rs` (canonical project path)
- **Verification:** `git show --name-only HEAD~3..HEAD` shows all 4 commits at `genossi-frontend/src/page/repayment_phase_details.rs` (without the worktree prefix).

**Total deviations:** 1 auto-fixed (Rule 3 - blocking).

All other plan-action code was implemented as written.

## Known Stubs

The plan intentionally leaves two stubs in the EntriesTab and ExportTab bodies — these are NOT bugs but deliberate hand-offs:

| Stub Location | Marker | Resolved By |
|---|---|---|
| `repayment_phase_details.rs` EntriesTab (Open/Closed branch) | `"TODO Plan 12-08: RepaymentEntryList für phase_id={...}"` | Plan 12-08 |
| `repayment_phase_details.rs` ExportTab (Open/Closed branch) | `"TODO Plan 12-14: Export-Tab für phase_id={...}"` | Plan 12-14 |

Both stubs render only when the phase is in `Open` or `Closed` status. In `Preparation` status, both tabs show the i18n `RepaymentEntriesNotOpenYet` / `RepaymentExportNotOpenYet` hint — that text is the FINAL behavior for Preparation (D-06) and does NOT change in Plan 12-08 / 12-14.

These stubs are tracked Plan-level (in the plan frontmatter `affects:` field and in `key-decisions` here). The grep-test `rg "TODO Plan 12-08\|TODO Plan 12-14" genossi-frontend/src/page/repayment_phase_details.rs` should return 2 lines after this plan and 0 lines after Plan 12-14 completes.

## Threat Flags

None — this plan only adds a frontend detail page that consumes existing backend endpoints. No new network surface, no new auth path, no schema changes.

## Self-Check: PASSED

Verified artifacts in the main repo:

- [FOUND] `genossi-frontend/src/page/repayment_phase_details.rs` (384 lines, >= 200 plan-minimum)
- [FOUND] `#[component] pub fn RepaymentPhaseDetails(id: String) -> Element`
- [FOUND] `#[component] fn BasicsTab(phase, on_changed, on_close_conflict, on_error)`
- [FOUND] `pub(crate) fn is_share_value_editable(status) -> bool` (Plan 12-06 reuse)
- [FOUND] `fn parse_close_conflict(err: &AppError) -> Option<CloseConflictResponse>` (Plan 12-10 reuse-pattern)
- [FOUND] `should_show_open_button` and `should_show_close_button` pure predicates
- [FOUND] 7 `#[test]`-Markierte Tests in tests-submodul
- [VERIFIED] `cargo build` exit 0 from `genossi-frontend/`
- [VERIFIED] `cargo test --bin genossi-frontend -- page::repayment_phase_details::tests` → 7/7 PASS
- [VERIFIED] Full `cargo test --bin genossi-frontend` → 162/162 PASS
- [VERIFIED] D-01 Button-Gate: 0 buttons without `r#type:`
- [VERIFIED] Component-First: `RepaymentPhaseStatusBadge {` appears 1× (header), `Modal` reused (no inline modal-styling)
- [VERIFIED] API wiring: 3× `api::get/open/close_repayment_phase`
- [VERIFIED] Stub-marker `TODO Plan 12-05` removed from file (0 occurrences)
- [VERIFIED] TODO-markers for next plans present: 1× `TODO Plan 12-08`, 1× `TODO Plan 12-14`
- [FOUND] Commit `7fe332d` (test(12-05): add failing tests for status-driven render predicates)
- [FOUND] Commit `e54b09c` (feat(12-05): implement status-driven render predicates)
- [FOUND] Commit `daeeca8` (test(12-05): add failing tests for parse_close_conflict body-parse)
- [FOUND] Commit `b019b33` (feat(12-05): implement 3-Tab detail page + Schließen-Confirm + 409 body-parse)

## TDD Gate Compliance

- **Task 1 RED gate:** `7fe332d` — 3 tests fail (stubs return false). RED confirmed.
- **Task 1 GREEN gate:** `e54b09c` — 3 tests PASS after `matches!()` implementations.
- **Task 2 RED gate:** `daeeca8` — `parse_close_conflict` stub returns None for all inputs; 1 of 4 new tests fails (the valid-body case). The 3 None-expected tests pass coincidentally — TDD-valid because the meaningful behavior (Some on valid body) is RED.
- **Task 2 GREEN gate:** `b019b33` — full `parse_close_conflict` + UI implementation; all 7 tests PASS.
- **REFACTOR gate:** none — implementation was minimal and matched the codebase style; no refactor commit needed.

Gate sequence in `git log a0eeb12..HEAD`:
```
7fe332d test(12-05): add failing tests for status-driven render predicates  ← Task 1 RED
e54b09c feat(12-05): implement status-driven render predicates              ← Task 1 GREEN
daeeca8 test(12-05): add failing tests for parse_close_conflict body-parse  ← Task 2 RED
b019b33 feat(12-05): implement 3-Tab detail page + Schließen-Confirm + 409 body-parse  ← Task 2 GREEN
```

Strict test→feat→test→feat — exemplary TDD-gate compliance.

---

*Phase: 12-frontend-component-first*
*Plan: 05 — Repayment Phase Detail Page (UI-02)*
*Completed: 2026-06-01T12:15:04Z (~7 min)*
