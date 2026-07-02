---
phase: 24-wysiwyg-frontend-editor
plan: 04
subsystem: test
tags: [e2e, wysiwyg, body_html, uat, ammonia-gate, sanitize-on-store, backward-compat, phase-close]

# Dependency graph
requires:
  - phase: 24-wysiwyg-frontend-editor plan 01
    provides: preview_mail body_html render seam + InboxService::reply body_html sanitize-on-store gate
  - phase: 24-wysiwyg-frontend-editor plan 02
    provides: WysiwygEditor Dioxus component (browser-side surface UAT walks through)
  - phase: 24-wysiwyg-frontend-editor plan 03
    provides: 3 migrated compose flows emitting body_html end-to-end (Massenmail, Inbox-Reply, Mail-Template)
provides:
  - preview_body_html_round_trips_to_response e2e test (pins Plan 24-01 Task 1 preview seam)
  - inbox_reply_body_html_sanitized_and_persisted e2e test (pins Plan 24-01 Task 2 sanitize-on-store gate)
  - 24-UAT-CHECKLIST.md — 12-step Vorstand-facing browser verification checklist covering EDIT-01..05
affects: []

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Preview seam e2e pattern — POST /api/mail/preview with body_html, assert rendered <b>Max</b> proves autoescape env + member interpolation in one shot"
    - "Reply sanitize e2e pattern — POST /api/inbox/{id}/reply → GET /api/mail/jobs/{job_id}, assert <script> stripped + <p>/<b> preserved on the persisted MailJob"
    - "Skip-serializing-if backward-compat pattern — omit body_html on the request, assert the response object has NO body_html key on the wire (mirrors Phase 23 bulk_mail_body_html_none_stays_backward_compatible)"
    - "UAT hard-fail gate identification — steps 3/4/5 (styleWithCSS, paste-plain, in-app modal) explicitly flagged as the ammonia+D-06 invariants"

key-files:
  created:
    - .planning/phases/24-wysiwyg-frontend-editor/24-UAT-CHECKLIST.md
  modified:
    - genossi_bin/tests/e2e_tests.rs

key-decisions:
  - "e2e tests reuse existing setup + setup_with_pool helpers — no new test infra (per plan action: 'do NOT add new test-infra')"
  - "Inbox reply test uses the full e2e HTTP path (not the service-level fallback) because seed_inbound_mail + /reply → /jobs/{id} chain is already fully wired — the fallback path is only needed if helpers are missing"
  - "UAT checklist explicitly flags steps 3/4/5 as HARD FAIL GATES (styleWithCSS=false Bold check, paste-plain check, in-app modal check) — these are the ammonia allow-list + D-06 invariants; other steps are recoverable/deferrable"
  - "Auto-mode checkpoint approval: Task 4 (human-verify) is auto-approved per this run's auto-mode directive; the browser-interactive UAT items are deferred to a live Vorstand smoke test session, the automated regression portion of the checkpoint (cargo test workspace) was run and passed with only the documented pre-existing Phase 22 failure"

patterns-established:
  - "Every WYSIWYG body_html seam gets a paired e2e test: (a) Some path proves render/sanitize, (b) None path proves skip-serializing-if backward-compat"
  - "UAT checklists document the browser behaviors automated tests cannot cover (contenteditable execCommand output, paste event handling, modal vs native prompt distinction)"

requirements-completed: [EDIT-01, EDIT-02, EDIT-03, EDIT-04, EDIT-05]

coverage:
  - id: D1
    description: "preview_mail with body_html renders through autoescape env AND substitutes member first_name — proven by asserting rendered response contains '<b>Max</b>' for a seeded Member with first_name='Max' + body_html='<p>Hallo <b>{{ first_name }}</b></p>'"
    requirement: EDIT-05
    verification:
      - kind: automated_ui
        ref: "cargo test -p genossi_bin --test e2e_tests preview_body_html_round_trips_to_response"
        status: pass
    human_judgment: false
  - id: D2
    description: "preview_mail without body_html on the request yields a response JSON with NO body_html key — skip_serializing_if wire backward-compat"
    requirement: EDIT-05
    verification:
      - kind: automated_ui
        ref: "cargo test -p genossi_bin --test e2e_tests preview_body_html_round_trips_to_response (Pass 2)"
        status: pass
    human_judgment: false
  - id: D3
    description: "inbox reply with body_html='<script>alert(1)</script><p>Reply <b>ok</b></p>' persists sanitized HTML on the MailJob: <script> stripped, <p>/<b>ok</b> preserved"
    requirement: EDIT-01
    verification:
      - kind: automated_ui
        ref: "cargo test -p genossi_bin --test e2e_tests inbox_reply_body_html_sanitized_and_persisted"
        status: pass
    human_judgment: false
  - id: D4
    description: "inbox reply WITHOUT body_html leaves MailJob.body_html = None — Phase 24 D-01 backward-compat with pre-Phase-24 frontends"
    requirement: EDIT-01
    verification:
      - kind: automated_ui
        ref: "cargo test -p genossi_bin --test e2e_tests inbox_reply_body_html_sanitized_and_persisted (Pass 2)"
        status: pass
    human_judgment: false
  - id: D5
    description: "24-UAT-CHECKLIST.md documents 12 browser-side verification steps covering EDIT-01..05; steps 3 (styleWithCSS=false Bold), 4 (paste-plain), 5 (in-app modal) explicitly flagged as HARD FAIL GATES"
    requirement: EDIT-02
    verification:
      - kind: automated_ui
        ref: "grep -c '\\- \\[' .planning/phases/24-wysiwyg-frontend-editor/24-UAT-CHECKLIST.md → 12 (≥ 10 target)"
        status: pass
      - kind: automated_ui
        ref: "grep -oE 'EDIT-0[1-5]' 24-UAT-CHECKLIST.md | sort -u → 5 unique (all EDIT requirements referenced)"
        status: pass
    human_judgment: false
  - id: D6
    description: "UAT checklist Setup section documents the launch flow: cargo run --features mock_auth + tailwindcss watch + dx serve (per run-rust-backend-and-frontend project skill)"
    requirement: EDIT-04
    verification:
      - kind: automated_ui
        ref: "grep -c 'dx serve\\|cargo run\\|mock_auth' 24-UAT-CHECKLIST.md → 3+"
        status: pass
    human_judgment: false
  - id: D7
    description: "Workspace regression: cargo test --workspace passes with only the pre-existing Phase-22 test_mail_preview_repayment_no_entries_does_not_default_to_one failure — no new regressions from Phase 24"
    requirement: EDIT-03
    verification:
      - kind: automated_ui
        ref: "cargo test --workspace --exclude genossi-frontend → 305 pass, 1 fail (pre-existing per STATE.md)"
        status: pass
    human_judgment: false
  - id: D8
    description: "Vorstand smoke test (browser-interactive UAT walkthrough of the 12 checklist steps) — auto-mode checkpoint auto-approved for this executor run; deferred to a live human smoke session before merge"
    requirement: EDIT-01
    verification:
      - kind: manual
        ref: "24-UAT-CHECKLIST.md steps 1-12 — deferred to user smoke-test (see Deferred Verification section below)"
        status: deferred
    human_judgment: true

# Metrics
duration: ~28min
completed: 2026-07-02
status: complete
---

# Phase 24 Plan 04: WYSIWYG Wave 4 — E2E Tests + UAT Checklist Summary

**Two new e2e tests pin Plan 24-01's backend seams (preview HTML round-trip + inbox reply sanitize-on-store), and a 12-step Vorstand-facing UAT checklist covers the browser-side behaviors automated tests cannot reach. Auto-mode approved the human-verify checkpoint after confirming the automated regression portion (cargo test --workspace: 305 pass, 1 pre-existing Phase 22 failure, 0 new regressions).**

## Performance

- **Duration:** ~28 min
- **Completed:** 2026-07-02
- **Tasks:** 4 (Task 4 = auto-approved checkpoint)
- **Files created:** 1 (24-UAT-CHECKLIST.md)
- **Files modified:** 1 (genossi_bin/tests/e2e_tests.rs — 2 tests added, ~200 lines)

## Accomplishments

- **Task 1 — `preview_body_html_round_trips_to_response`:** New `#[tokio::test]` at the end of `e2e_tests.rs`. Two-pass assertion pattern.
  - **Pass 1 (render + interpolation proof):** POST `/api/mail/preview` with `subject`, `body: "Hallo {{ first_name }}"`, `member_id`, `body_html: "<p>Hallo <b>{{ first_name }}</b></p>"`. Seeded Member has `first_name = "Max"`. Assertion (a) `response.body == "Hallo Max"` (plain path unchanged). Assertion (b) `response.body_html` is Some. Assertion (c) rendered `body_html` contains `<b>Max</b>` — proves the autoescape env round-trip AND the member-variable interpolation in a single test.
  - **Pass 2 (backward-compat proof):** POST the same request WITHOUT `body_html` key. Assert response JSON does NOT contain `body_html` key (or it's null) — proves `#[serde(default, skip_serializing_if = "Option::is_none")]` wire-shape preservation. Mirrors Phase 23's `bulk_mail_body_html_none_stays_backward_compatible`.

- **Task 2 — `inbox_reply_body_html_sanitized_and_persisted`:** New `#[tokio::test]` using `setup_with_pool()` + `seed_inbound_mail()` helpers (no new test infra). Full e2e HTTP path.
  - **Pass 1 (sanitize gate proof):** Seed an inbound mail via `seed_inbound_mail` (UID 42, `customer@example.com`, `Anfrage`). POST `/api/inbox/{mail_id}/reply` with `body_html: "<script>alert(1)</script><p>Reply <b>ok</b></p>"`. Assert HTTP 202 (per `inbox_rest.rs::reply_inbox` line 515). Extract `job_id` from `ReplyResponseTO`. GET `/api/mail/jobs/{job_id}` → `MailJobDetailTO`. Assert (a) `detail.job.body_html.unwrap()` does NOT contain `<script>`, (b) it DOES contain `<p>` + `<b>ok</b>`. Proves ammonia gate held at store boundary + safe author markup preserved.
  - **Pass 2 (None backward-compat proof):** Seed a second inbound mail (UID 43). POST `/reply` WITHOUT `body_html` key. Assert `MailJob.body_html.is_none()` on the persisted job + `job.body == "Text-only reply."` (plain body preserved).

- **Task 3 — `.planning/phases/24-wysiwyg-frontend-editor/24-UAT-CHECKLIST.md`:** New 12-step Vorstand-facing checklist. Structure:
  - **`## Setup`** — how to launch backend (`cargo run --features mock_auth --bin genossi`) + frontend (`dx serve` + `tailwindcss --watch`) per `run-rust-backend-and-frontend` project skill.
  - **`## Verification Steps`** — 12 numbered checkbox items covering all 3 compose flows (Massenmail, Inbox-Reply, Mail-Template editor). Each item lists: click/type action, expected outcome, DevTools inspection instructions, EDIT-XX requirement. Steps 3 (styleWithCSS=false Bold check), 4 (paste-plain check), 5 (in-app modal check) explicitly flagged as ⚠️ **HARD FAIL GATES** — those pin the ammonia allow-list + D-06 invariants that make the sanitize-on-store gate work at all.
  - **`## Known limitations`** — documents execCommand deprecation status, TemplateVarButtons signal-sync 1-render lag (Plan 24-03 §Decisions), TemplateSelector clears body_html on select, initial-body-with-footer/quote paths leave body_html empty on mount.
  - **`## Regression check`** — automation commands: `cargo test -p genossi_mail --lib`, `cargo test -p genossi_bin --test e2e_tests`, `cargo build`, and frontend `cargo check --target wasm32-unknown-unknown` + `cargo test --bin genossi-frontend`. Expected counts documented (306 e2e tests including the 2 new ones, 284+ frontend).
  - **`## Sign-off`** — Vorstand smoke tester signature block with hard-fail-gates checkbox.

- **Task 4 — Human-verify checkpoint (⚡ auto-approved per auto-mode directive):** The interactive browser walkthrough of the 12 UAT steps is deferred to a live Vorstand smoke-test session. The **automated regression portion** of the checkpoint was executed and passed: `cargo test --workspace --exclude genossi-frontend` → **305 pass, 1 fail** (documented pre-existing Phase 22 `test_mail_preview_repayment_no_entries_does_not_default_to_one` — NOT a Phase 24 regression per STATE.md). `cargo build` clean. Both new tests included in the pass count.

## Task Commits

Each task committed atomically via jj:

1. **Task 1: e2e preview_body_html_round_trips_to_response** — `cfa37941cb24` (test)
2. **Task 2: e2e inbox_reply_body_html_sanitized_and_persisted** — `36defe925031` (test)
3. **Task 3: 24-UAT-CHECKLIST.md** — `db9d879c36d6` (docs)
4. **Task 4:** auto-approved checkpoint — no code commit; regression portion executed inline (workspace test run).

## Decisions Made

- **Full HTTP e2e path for the reply test (not the service-level fallback).** The plan's action offered a fallback: if seeding an `InboundMail` was too heavy, drop to a service-level test in `genossi_mail/src/inbox.rs`. Verification: `seed_inbound_mail(pool, uid, from, subject)` already exists at `e2e_tests.rs:4796` and is used by 15+ existing tests; and the `POST /api/inbox/{id}/reply` → 202 → GET `/api/mail/jobs/{job_id}` chain is fully wired end-to-end (no worker needed for the assertion — we only verify the persisted `MailJob.body_html`). The full HTTP path is the higher-value test, so used it directly.
- **Both `body_html` tests share the two-pass Some/None pattern.** Every new body_html seam in Phase 24 comes with a paired assertion: (a) Some path proves the render/sanitize behavior, (b) None path proves the `skip_serializing_if` backward-compat wire shape. This is the canonical Phase 23/24 pattern (mirrors `bulk_mail_body_html_sanitized_and_persisted` + `bulk_mail_body_html_none_stays_backward_compatible`).
- **UAT checklist explicitly names 3 HARD FAIL GATES.** Steps 3 (styleWithCSS=false → Bold produces `<b>`, not span-style), 4 (paste from Word yields plain text only), and 5 (link toolbar opens in-app modal, not `window.prompt`) are flagged as hard-fail gates. If any of these fails, the phase must not merge. Other steps (e.g. step 11 template load, step 12 round-trip) may be deferred to a follow-up with human sign-off. This ordering reflects the plan's threat model (T-24-11 + T-24-12 are `accept`-disposed manual checks, but step 3/4/5 are the invariants the ammonia gate needs to hold).
- **Auto-mode approval of the human-verify checkpoint.** Per this run's `<auto_mode_directive>`: AUTO_MODE=true, so `checkpoint:human-verify` is auto-approved unless it has `gate="blocking-human"` or is a package-legitimacy checkpoint. This checkpoint is `gate="blocking"` (regular blocking, not `blocking-human`) and is not package-legitimacy → auto-approve applies. The **automated regression portion** was still executed (workspace test run, build gate) — only the **browser-interactive UAT walkthrough** is deferred to a live user smoke-test session. This deferral is documented explicitly below under "Deferred Verification".

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 — Blocking] `String::as_ref()` type inference ambiguity in assertion**

- **Found during:** Task 2 (compile step)
- **Issue:** Wrote `assert_eq!(detail.job.body.as_ref(), "Text-only reply.")` — rustc emits `error[E0283]: type annotations needed` because `String` implements `AsRef<OsStr>`, `AsRef<[u8]>`, `AsRef<Path>`, and `AsRef<str>` and the assertion target `&str` doesn't disambiguate.
- **Fix:** Dropped the `.as_ref()`: `assert_eq!(detail.job.body, "Text-only reply.")` — `PartialEq<&str> for String` handles the comparison directly.
- **Files modified:** `genossi_bin/tests/e2e_tests.rs`
- **Verification:** `cargo test -p genossi_bin --test e2e_tests inbox_reply_body_html_sanitized_and_persisted` — passes.
- **Committed in:** `36defe925031` (Task 2 commit).

---

**Total deviations:** 1 auto-fixed (1 Rule 3 blocking type-inference tweak — mechanical `.as_ref()` drop).
**Impact on plan:** Isolated to the introducing task; no scope creep. The plan's action was structurally correct; the deviation was a low-level Rust type-inference nudge that the plan didn't spell out because it's an idiomatic 1-character fix.

## Deferred Verification

The following UAT items require a live browser session with a running backend + frontend + real DevTools interaction. **All are deferred to a user smoke-test session** — the executor cannot script contenteditable behavior, execCommand output inspection, paste event synthesis, or visual modal-vs-prompt discrimination.

| # | Item | Requirement | Auto-approvable? | Notes |
|---|------|-------------|-----------------|-------|
| 1 | Editor mounts without console errors | EDIT-01 | No (browser console) | ⚡ Auto-approved; smoke-test recommended |
| 2 | Plain-text on_change tuple flows correctly | EDIT-01, EDIT-03 | No (DevTools inspection) | ⚡ Auto-approved; smoke-test recommended |
| 3 | styleWithCSS=false → Bold produces `<b>` | EDIT-02 | **No — HARD FAIL GATE** | ⚡ Auto-approved but MUST be re-verified live before merge |
| 4 | Paste from Word yields plain text only | EDIT-04 | **No — HARD FAIL GATE** | ⚡ Auto-approved but MUST be re-verified live before merge |
| 5 | Link button opens in-app modal | EDIT-01, D-06 | **No — HARD FAIL GATE** | ⚡ Auto-approved but MUST be re-verified live before merge |
| 6 | Link wraps selection | EDIT-01, D-06 | No (Selection Range test) | ⚡ Auto-approved; smoke-test recommended |
| 7 | Invalid link URL rejected (Insert button disabled) | EDIT-02 | No (UI state test) | ⚡ Auto-approved; smoke-test recommended |
| 8 | TemplatePreview renders live HTML with member vars | EDIT-05 | No (visual render check) | ⚡ Auto-approved; smoke-test recommended |
| 9 | Sent bulk-mail is multipart/alternative | EDIT-03 | **No — needs real SMTP + test MUA** | ⚡ Auto-approved; explicit warning: only against test SMTP inbox, never real member emails |
| 10 | Inbox-Reply matches Compose behavior | EDIT-01, EDIT-02, EDIT-04, EDIT-05 | No (browser check) | ⚡ Auto-approved; smoke-test recommended |
| 11 | Mail-Template edit loads body_html + toolbar works | EDIT-01, EDIT-03 | No (browser check) | ⚡ Auto-approved; smoke-test recommended |
| 12 | Mail-Template body_html round-trips through save/reload | EDIT-03 | No (browser + Network check) | ⚡ Auto-approved; smoke-test recommended |

**Recommended live-smoke session:** Before the Vorstand consumes Phase 24 in production, run at minimum steps 3, 4, 5 (the HARD FAIL GATES) in a real browser session. Steps 3 and 4 are the invariants the ammonia allow-list needs; step 5 is the D-06 in-app-modal invariant. Step 9 (multipart/alternative bulk send) should be run against a test SMTP inbox at least once to prove the wire-level shape end-to-end.

## Issues Encountered

- **jj backend.** Project uses jj VCS with git backend. All 3 code commits went through `jj describe -m "..." && jj new` (the canonical Genossi commit pattern per `feedback_use_jj_not_git.md`). No `git commit` issued.
- **Pre-existing Phase 22 test failure carried over.** `test_mail_preview_repayment_no_entries_does_not_default_to_one` fails on the "errors must be array" assertion — this failure is documented in STATE.md and predates Phase 24. It was NOT caused by any Phase 24 change. Explicitly called out in the UAT checklist regression section.
- **Long compile times for the first e2e test run.** First `cargo test -p genossi_bin --test e2e_tests` invocation after the e2e file edit takes ~2:15 (full re-compile of genossi_mail + genossi_service_impl + genossi_rest + genossi_bin). Subsequent runs (incremental) drop to ~30–60s.

## Threat Flags

No new attack surface introduced beyond the plan's `<threat_model>` entries:

- **T-24-10 (Tampering — e2e sanitize-in-store test)** — mitigated: `inbox_reply_body_html_sanitized_and_persisted` is now the regression pin. If a future refactor drops the `sanitize_body_html_opt` call from `InboxServiceImpl::reply`, this test fails immediately.
- **T-24-11 (Tampering — UAT styleWithCSS=false)** — accepted: manual browser check (step 3, HARD FAIL GATE). Cannot be automated without wasm-bindgen-test browser CI, which is not scoped for this milestone.
- **T-24-12 (Tampering — UAT paste-plain)** — accepted: manual browser check (step 4, HARD FAIL GATE). Same rationale as T-24-11; ammonia gate at the backend is defense-in-depth regardless.

## Known Stubs

None. The two new e2e tests fully assert the visible behavior; the UAT checklist is a documentation artifact (not code) and its 12 steps are concrete browser instructions with expected outcomes, not placeholders.

## Self-Check: PASSED

Files exist:
- `[FOUND] .planning/phases/24-wysiwyg-frontend-editor/24-04-SUMMARY.md` (this file)
- `[FOUND] .planning/phases/24-wysiwyg-frontend-editor/24-UAT-CHECKLIST.md`

Commits exist in jj log:
- `[FOUND] cfa37941cb24` (Task 1 — test preview_body_html_round_trips_to_response)
- `[FOUND] 36defe925031` (Task 2 — test inbox_reply_body_html_sanitized_and_persisted)
- `[FOUND] db9d879c36d6` (Task 3 — docs 24-UAT-CHECKLIST.md)

Automated verification:
- `cargo test -p genossi_bin --test e2e_tests preview_body_html_round_trips_to_response` → **pass** ✓
- `cargo test -p genossi_bin --test e2e_tests inbox_reply_body_html_sanitized_and_persisted` → **pass** ✓
- `cargo test -p genossi_bin --test e2e_tests body_html` → **5 pass, 0 fail** (2 new + 3 pre-existing) ✓
- `cargo test --workspace --exclude genossi-frontend` → **305 pass, 1 fail** (documented pre-existing Phase 22, NOT a Phase 24 regression) ✓
- `cargo build` (workspace) → **clean** ✓
- `grep -c '\- \[' 24-UAT-CHECKLIST.md` → **12** (≥ 10 target) ✓
- `grep -oE 'EDIT-0[1-5]' 24-UAT-CHECKLIST.md | sort -u` → **5 unique** (all EDIT-01..05 covered) ✓

## Phase 24 Close-Out

With Plan 24-04 done, **Phase 24 is complete** across all 4 waves:

- **Wave 1 (Plan 24-01):** Backend body_html echo on `preview_mail` + inbox reply sanitize gate + frontend api mirror + web-sys features + 19 MailEditor* i18n keys.
- **Wave 2 (Plan 24-02):** `WysiwygEditor` Dioxus component (contenteditable + 13-button toolbar + in-app link modal + execCommand facade + `is_valid_link_url`).
- **Wave 3 (Plan 24-03):** All 3 `MailBodyEditor` call sites (Massenmail, Inbox-Reply, Mail-Template editor) migrated to `WysiwygEditor` with body_html signals wired end-to-end; `TemplatePreview` renders backend HTML via `dangerous_inner_html`; `body_editor.rs` deleted.
- **Wave 4 (Plan 24-04, this plan):** 2 e2e tests pin the backend seams + 12-step UAT checklist covers browser-side behaviors + workspace regression check confirms 0 new failures.

**Requirements completed by Phase 24:** EDIT-01, EDIT-02, EDIT-03, EDIT-04, EDIT-05 (all five milestone requirements for the WYSIWYG editor).

**Handoff for merge:** Recommend a live Vorstand smoke-test session running at minimum UAT steps 3, 4, 5 (HARD FAIL GATES) before merging to the production branch. The automated tests + workspace regression confirm the wire and the store-boundary behavior; the browser-side execCommand/paste/modal behavior needs one real-DevTools eyeball before Genossenschaft-facing rollout.

---
*Phase: 24-wysiwyg-frontend-editor*
*Completed: 2026-07-02*
