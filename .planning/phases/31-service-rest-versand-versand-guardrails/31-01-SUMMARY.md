---
phase: 31-service-rest-versand-versand-guardrails
plan: 01
subsystem: api
tags: [mail, minijinja, template-rendering, application, worker, dependency-injection]

# Dependency graph
requires:
  - phase: 30-application-template-context
    provides: application_to_template_context + validate_application_template (pure applicant ctx builder)
  - phase: 29-application-recipient-linkage
    provides: MailRecipient.application_id column + RecipientInput.application_id
provides:
  - "ApplicationResolver trait (automock) in genossi_mail::template — loads Application by id in the render path"
  - "ApplicationMailConfig + load_application_mail_config — resolves the 5 genossenschaft payment-config keys with real errors"
  - "render_application_content — the ONE shared pure applicant render kernel (pub), consumed by Worker now and Preview in 31-02"
  - "Application-Zweig in resolve_rendered_content (application_id has priority, reads only application_id)"
  - "PoolApplicationResolver in genossi_bin wired into live worker + startup backfill"
affects: [31-02-service-rest-preview, 31-03, application-mail-send, mail-preview]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Renderer-Seam: single pure kernel (render_application_content) shared between worker and preview (D-06)"
    - "PoolApplicationResolver mirrors PoolMemberResolver — DAO-backed resolver impl for the render layer"
    - "Namespace separation: Application-Zweig reads ONLY recipient.application_id, member path untouched (Pitfall 2)"

key-files:
  created: []
  modified:
    - genossi_mail/src/template.rs
    - genossi_mail/src/render.rs
    - genossi_mail/src/worker.rs
    - genossi_mail/src/backfill.rs
    - genossi_bin/src/lib.rs

key-decisions:
  - "load_application_mail_config propagates every config error as MailServiceError (no tracing::error!+return swallow) — a missing config fails the render so the recipient is marked failed, never silently mis-rendered"
  - "Application-Zweig sits at the very top of resolve_rendered_content and returns early — member/passthrough branches stay byte-identical"
  - "body_html is NOT re-sanitized in the renderer (D-05); ammonia already ran at the create_job store boundary"
  - "New resolver args appended LAST on both start_mail_worker (AR) and run_rendered_backfill (AR, CS) to keep all existing positional args stable (Pitfall 4)"

patterns-established:
  - "Applicant render kernel is pure/sync (no config.get) — caller resolves config once and passes ApplicationMailConfig in, enabling the Preview (31-02) to reuse the exact same code path"

requirements-completed: [APMAIL-01]

coverage:
  - id: D1
    description: "A mail_recipient with application_id renders subject/body/body_html via application_to_template_context; open_amount = format_eur_de(share_value_cents × shares) appears correctly formatted"
    requirement: APMAIL-01
    verification:
      - kind: unit
        ref: "genossi_mail/src/render.rs#resolve_rendered_content_application_branch_renders_open_amount"
        status: pass
      - kind: unit
        ref: "genossi_mail/src/render.rs#resolve_rendered_content_application_branch_takes_priority"
        status: pass
    human_judgment: false
  - id: D2
    description: "Missing application-mail config fails the render (Err(RenderFailure)) instead of silently mis-rendering"
    requirement: APMAIL-01
    verification:
      - kind: unit
        ref: "genossi_mail/src/render.rs#resolve_rendered_content_application_branch_missing_config_errs"
        status: pass
    human_judgment: false
  - id: D3
    description: "render_application_content is the shared pure kernel: derives plain body from rendered HTML, no re-sanitize"
    requirement: APMAIL-01
    verification:
      - kind: unit
        ref: "genossi_mail/src/render.rs#render_application_content_derives_plain_body_from_html"
        status: pass
    human_judgment: false
  - id: D4
    description: "genossi_bin wiring: PoolApplicationResolver + ConfigService feed the live worker and startup backfill so the Application-Zweig renders in production"
    requirement: APMAIL-01
    verification:
      - kind: integration
        ref: "nix develop --command cargo build -p genossi_bin"
        status: pass
    human_judgment: false

# Metrics
duration: 7min
completed: 2026-08-20
status: complete
---

# Phase 31 Plan 01: Application-Renderer-Seam Summary

**resolve_rendered_content gains an Application branch that renders per-recipient via application_to_template_context through one shared pure kernel (render_application_content), wired into the live worker + startup backfill via PoolApplicationResolver.**

## Performance

- **Duration:** 7 min
- **Started:** 2026-08-20T18:59:32Z
- **Completed:** 2026-08-20T19:06:45Z
- **Tasks:** 2
- **Files modified:** 5

## Accomplishments
- `ApplicationResolver` trait (automock) mirrors `MemberResolver`; `render_application_content` is the single pure applicant render kernel (`pub`), ready for the Preview consumer in Plan 31-02 (D-06).
- Application branch in `resolve_rendered_content` takes priority and reads ONLY `recipient.application_id` — member/passthrough branches stay byte-identical (Pitfall 2, T-31-04).
- `load_application_mail_config` resolves the five genossenschaft config keys with real `MailServiceError` propagation (no swallow); `open_amount` renders as `format_eur_de(share_value_cents × shares)`.
- `PoolApplicationResolver` wired into `start_mail_worker` (17th positional arg) and `start_rendered_backfill_worker` (+ a fresh `ConfigService`), so the branch renders in production.

## Task Commits

Each task was committed atomically:

1. **Task 1: Application-Renderer-Seam in genossi_mail (Trait + Kernel + Zweig + beide Call-Sites)** - `9065f8d` (feat)
2. **Task 2: genossi_bin-Wiring — PoolApplicationResolver + Worker/Backfill** - `7d258ea` (feat)

_Task 1 carries production code + tests together: the signature change to `resolve_rendered_content` ripples through both call sites and the existing render tests in the same compilation unit, so a tests-only RED commit could not compile. Tests were written alongside the implementation and all pass._

## Files Created/Modified
- `genossi_mail/src/template.rs` - `ApplicationResolver` trait (automock, mirror of MemberResolver)
- `genossi_mail/src/render.rs` - `ApplicationMailConfig`, `load_application_mail_config`, `render_application_content` (pure kernel), Application branch in `resolve_rendered_content`, 4 new tests
- `genossi_mail/src/worker.rs` - `AR` generic + `application_resolver` arg on `start_mail_worker`; passes `application_resolver`/`config_service` into the render call
- `genossi_mail/src/backfill.rs` - `AR`/`CS` generics + args on `run_rendered_backfill`; 3 tests updated to thread the new args
- `genossi_bin/src/lib.rs` - `PoolApplicationResolver` (ApplicationDao-backed) + wiring into worker and backfill

## Decisions Made
- Config errors are propagated (not swallowed) so a mis-configured send fails loudly per-recipient rather than shipping a broken mail.
- The applicant kernel is pure/sync so the Preview (31-02) can call the exact same code path with a config it resolved itself — one renderer seam, no second render path (D-06).
- No re-sanitize of `body_html` in the renderer (D-05); ammonia is authoritative at the `create_job` store boundary.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking tooling] Used `rustfmt` directly for genossi_bin instead of `cargo fmt -- <file>`**
- **Found during:** Task 1 verification (formatting step)
- **Issue:** The plan's `cargo fmt -- <file>` command does NOT scope to the listed files — `cargo fmt` reformats the entire workspace (the `--` args are rustfmt flags, not a file filter). Running it dirtied ~24 unrelated files.
- **Fix:** Reverted all fmt-noise files via `git checkout --` (kept only the 4 intended mail files), then formatted `genossi_bin/src/lib.rs` with `nix develop --command rustfmt --edition 2021 genossi_bin/src/lib.rs`, which is genuinely file-scoped.
- **Files modified:** none beyond the 5 intended plan files
- **Verification:** `git status --short` shows only the 5 intended source files + STATE.md (orchestrator-owned) modified after cleanup.
- **Committed in:** covered by task commits (no extra files leaked)

---

**Total deviations:** 1 (tooling adjustment to avoid workspace-wide fmt footgun)
**Impact on plan:** No behavioral change; the formatting outcome for the changed files is identical, unrelated files untouched. No scope creep.

## Issues Encountered
- `node` is not on the base PATH; all gsd-tools + build/test commands were run through the Nix devshell (`nix develop --command …`) per the project's toolchain constraint.
- Avoided a top-level `use std::sync::Arc;` in `render.rs` (would collide with the test module's own `use std::sync::Arc;` via `use super::*`) by using fully-qualified `std::sync::Arc::from(...)` in `load_application_mail_config`.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- `render_application_content` is exposed `pub` and is the single seam Plan 31-02's Preview will consume — no second render path to reconcile.
- Live worker and startup backfill both carry a real `ApplicationResolver` + `ConfigService`, so an applicant send with `recipient.application_id = Some(...)` renders end-to-end once the REST/service send path (later plans) enqueues it.

## Self-Check: PASSED

- Files verified present: `31-01-SUMMARY.md`, `genossi_mail/src/render.rs`, `genossi_bin/src/lib.rs`
- Commits verified in git log: `9065f8d`, `7d258ea`
- `nix develop --command cargo test -p genossi_mail` → 309 passed, 0 failed
- `nix develop --command cargo build -p genossi_bin` → compiles

---
*Phase: 31-service-rest-versand-versand-guardrails*
*Completed: 2026-08-20*
