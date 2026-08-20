---
phase: 31-service-rest-versand-versand-guardrails
plan: 02
subsystem: service
tags: [application, mail, send, preview, guardrails, cr-02, mockall, dependency-injection]

# Dependency graph
requires:
  - phase: 31-service-rest-versand-versand-guardrails
    plan: 01
    provides: "render_application_content (pub pure kernel) + load_application_mail_config"
  - phase: 29-application-recipient-linkage
    provides: "get_application_communications DAO + RecipientInput.application_id"
provides:
  - "ApplicationService::send_mail — Result<(), ServiceError> with CR-02 ordering, single application-bound recipient, no silent ()"
  - "ApplicationService::preview_mail — synchronous preview via the shared render_application_content kernel (D-06)"
  - "ApplicationService::last_sent_at — server-side MAX aggregate over get_application_communications (APHIST-02)"
  - "ApplicationMailInput / ApplicationMailDraft / RenderedApplicationMail — cycle-free service-local mail I/O types"
  - "CommunicationDao dependency wired into ApplicationServiceImpl (gen_service_impl! + genossi_bin)"
affects: [31-03-rest-endpoints, mail-preview, application-mail-send]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "CR-02 ordering reused verbatim from confirm(): permission → NotFound → status guard, no user-attributable side effect before the permission check"
    - "Preview reuses the ONE render seam (render_application_content) — preview output is byte-identical to worker output (D-06)"
    - "Namespace separation on RecipientInput: member_id None + application_id Some (Pitfall 2)"

key-files:
  created: []
  modified:
    - genossi_service/src/application.rs
    - genossi_service_impl/src/application.rs
    - genossi_bin/src/lib.rs

key-decisions:
  - "send_mail commits the read transaction before the status/address/enqueue logic — the DAO lookup is the only DB work; the enqueue is job-queue based (no long-held tx)"
  - "last_sent_at loads find_by_id for 404-consistency before aggregating, so an unknown application yields EntityNotFound rather than a misleading None"
  - "MailServiceError is mapped to ServiceError::InternalError (500) at the send seam; there is no From<MailServiceError> impl, so the mapping is explicit"

patterns-established:
  - "A separate test builder (build_mail_service) supplies bare mocks for the confirm-only DAOs and customisable mail/config/communication mocks — keeps the mail tests independent of the confirm() fixture chain"

requirements-completed: [APMAIL-01, APMAIL-02, APCMP-01, APCMP-02, APHIST-02]

coverage:
  - id: T1
    description: "send_mail 403 permission denied: create_job never called (CR-02 no side effect)"
    requirement: APMAIL-02
    verification:
      - kind: unit
        ref: "genossi_service_impl/src/application.rs#test_send_mail_permission_denied_no_side_effect"
        status: pass
    human_judgment: false
  - id: T2
    description: "send_mail 404: unknown application → EntityNotFound; create_job never called"
    requirement: APMAIL-02
    verification:
      - kind: unit
        ref: "genossi_service_impl/src/application.rs#test_send_mail_not_found"
        status: pass
    human_judgment: false
  - id: T3
    description: "send_mail 409: status != Offen → Conflict; create_job never called"
    requirement: APCMP-01
    verification:
      - kind: unit
        ref: "genossi_service_impl/src/application.rs#test_send_mail_status_not_offen_conflicts"
        status: pass
    human_judgment: false
  - id: T4
    description: "send_mail 400: Offen but no email → ValidationError(field=email); no create_job"
    requirement: APCMP-02
    verification:
      - kind: unit
        ref: "genossi_service_impl/src/application.rs#test_send_mail_missing_address_validation_error"
        status: pass
    human_judgment: false
  - id: T5
    description: "send_mail happy path: exactly ONE recipient, member_id None + application_id Some (D-13, Pitfall 2)"
    requirement: APMAIL-01
    verification:
      - kind: unit
        ref: "genossi_service_impl/src/application.rs#test_send_mail_happy_single_application_recipient"
        status: pass
    human_judgment: false
  - id: T6
    description: "send_mail 500: create_job Err → InternalError propagates synchronously (D-01, no silent Ok)"
    requirement: APMAIL-02
    verification:
      - kind: unit
        ref: "genossi_service_impl/src/application.rs#test_send_mail_enqueue_error_propagates_500"
        status: pass
    human_judgment: false
  - id: T7
    description: "last_sent_at: MAX over outbound history; empty → None (OQ1 Option A)"
    requirement: APHIST-02
    verification:
      - kind: unit
        ref: "genossi_service_impl/src/application.rs#test_last_sent_at_returns_max_and_none"
        status: pass
    human_judgment: false
  - id: T8
    description: "preview_mail: shared kernel renders {{ open_amount }} to the format_eur_de amount"
    requirement: APMAIL-01
    verification:
      - kind: unit
        ref: "genossi_service_impl/src/application.rs#test_preview_mail_renders_open_amount"
        status: pass
    human_judgment: false

# Metrics
duration: 11min
completed: 2026-08-20
status: complete
---

# Phase 31 Plan 02: Service-Versand (send_mail / preview_mail / last_sent_at) Summary

**ApplicationService gains three methods — `send_mail` (job-queue send with a real `Result`, CR-02 guards, single application-bound recipient, no silent `()`), `preview_mail` (synchronous preview via the shared `render_application_content` kernel), and `last_sent_at` (server-side MAX over the applicant outbound timeline) — plus a new `CommunicationDao` dependency, all unit-tested with mockall.**

## Performance

- **Duration:** ~11 min
- **Started:** 2026-08-20T21:08Z (after 31-01 completion commit)
- **Completed:** 2026-08-20T21:19Z
- **Tasks:** 3
- **Files modified:** 3

## Accomplishments
- `send_mail` follows the `confirm()` CR-02 ordering exactly: permission FIRST, then 404, then the `Offen`-only status guard (409), then missing-address validation (400). It stamps exactly ONE `RecipientInput { address: application.email, member_id: None, application_id: Some(app.id) }` and calls `create_job`; an enqueue `Err` propagates as `InternalError` (500). No path returns a silent `()`.
- `preview_mail` renders the draft through the SAME pure `render_application_content` kernel the worker uses (D-06) — the service resolves `load_application_mail_config` once and hands it in. Status-independent, never enqueues.
- `last_sent_at` returns `MAX(entry.date)` over `get_application_communications(id)` via a new `CommunicationDao` dependency (OQ1 Option A) — `date = COALESCE(sent_at, created)`, no client-side math, no new DAO query.
- Three cycle-free service-local types (`ApplicationMailInput`, `ApplicationMailDraft`, `RenderedApplicationMail`) — no `genossi_mail` type re-imported into `genossi_service` (avoids the dependency cycle).
- `CommunicationDao` dependency wired through `gen_service_impl!`, the explicit `ApplicationServiceDeps` impl in `genossi_bin`, and both `ApplicationServiceImpl` construction sites.

## Task Commits

Each task was committed atomically:

1. **Task 1: ApplicationService trait — send_mail/preview_mail/last_sent_at + mail I/O types** — `62d9420` (feat)
2. **Task 2: Impl + CommunicationDao dependency + genossi_bin wiring** — `c86254d` (feat)
3. **Task 3: Service unit tests (mockall) — guards, single-recipient, last_sent_at, preview** — `da75377` (test)

_Task 3 is marked `tdd="true"`. As in Plan 31-01, the production code (Task 2) and its tests share the same compilation unit and the methods must exist for the test module to compile, so a tests-only RED commit could not compile in isolation. The eight tests were written against the Task 2 implementation and all pass; the 403/409/400 guards each assert `create_job().never()` (CR-02 no-side-effect proof) and the 500-enqueue test proves synchronous error propagation._

## Files Created/Modified
- `genossi_service/src/application.rs` — three mail I/O types + three trait methods (with CR-02 / no-silent-`()` doc contracts); `MockApplicationService` gains the methods via `#[automock]`.
- `genossi_service_impl/src/application.rs` — `CommunicationDao` in `gen_service_impl!`; `send_mail`/`preview_mail`/`last_sent_at` impls; eight new mockall unit tests + a dedicated `build_mail_service` builder.
- `genossi_bin/src/lib.rs` — `CommunicationDao` associated type on the explicit `ApplicationServiceDeps` impl + `communication_dao` field on the `ApplicationServiceImpl` construction (`CommunicationDaoSqlite`).

## Decisions Made
- `send_mail` commits the short read transaction right after `find_by_id`; the send itself is job-queue based, so no DB transaction is held across the enqueue.
- `last_sent_at` performs a `find_by_id` for 404-consistency before aggregating, so an unknown application returns `EntityNotFound` rather than a misleading `None`.
- `MailServiceError` → `ServiceError::InternalError` is mapped explicitly at the send seam (no `From` impl exists), keeping the crate boundary clean.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `CommunicationDao` trait had to be brought into scope in the impl file**
- **Found during:** Task 2 build
- **Issue:** `self.communication_dao.get_application_communications(...)` failed to resolve — the trait method needs `genossi_mail::dao::CommunicationDao` in scope.
- **Fix:** Added `use genossi_mail::dao::CommunicationDao as CommunicationDaoTrait;`.
- **Files modified:** `genossi_service_impl/src/application.rs`
- **Committed in:** `c86254d`

**2. [Rule 3 - Blocking] `genossi_bin` has an explicit `ApplicationServiceDeps` impl (not just the struct literal)**
- **Found during:** Task 2 build
- **Issue:** The plan's `read_first` pointed only at the `ApplicationServiceImpl { ... }` construction (Z. 988-1007), but `genossi_bin` also has a hand-written `impl ApplicationServiceDeps for ApplicationServiceDependencies` (Z. 181) that must declare the new associated type.
- **Fix:** Added `type CommunicationDao = genossi_mail::dao_sqlite::CommunicationDaoSqlite;` to that impl in addition to the `communication_dao` field on the construction.
- **Files modified:** `genossi_bin/src/lib.rs`
- **Committed in:** `c86254d`

**3. [Rule 3 - Tooling] Used file-scoped `rustfmt` instead of `cargo fmt -- <file>`**
- **Found during:** all tasks (formatting step)
- **Issue:** `cargo fmt -- <file>` reformats the whole workspace (documented in 31-01-SUMMARY and the project toolchain notes).
- **Fix:** Formatted only the changed files with `nix develop --command rustfmt --edition 2021 <file>`.
- **Files modified:** none beyond the 3 intended plan files.

---

**Total deviations:** 3 (all Rule 3 blocking/tooling; no behavioral change, no scope creep)
**Impact on plan:** None on behavior. Deviations 1 & 2 were required wiring the plan's file map under-specified; deviation 3 avoids the workspace-wide fmt footgun.

## Issues Encountered
- `node`/`cargo` are not on the base PATH; every build/test/fmt ran through `nix develop --command …` per the toolchain constraint.
- `MailJob` has 15 fields; the happy-path `create_job` mock needed a `dummy_mail_job()` fixture to return a valid `Ok(MailJob)`.

## Verification
- `nix develop --command cargo build -p genossi_service --features utoipa` → compiles (trait in isolation).
- `nix develop --command cargo build -p genossi_bin` → compiles (wiring).
- `nix develop --command cargo test -p genossi_service_impl` → 445 passed, 0 failed, 2 ignored (12 in the `application::` module, 8 of them new).

## User Setup Required
None — no external service configuration; the change is internal service logic + wiring (no new dependency).

## Next Phase Readiness
- Plan 31-03 (REST) can now call `send_mail` / `preview_mail` / `last_sent_at` behind the admin gate: `POST /{id}/mail`, `POST /{id}/mail/preview`, `GET /{id}/communications`, and hang the `last_sent_at` field on `get_application` (OQ2, D-08).
- The service enforces every guard synchronously, so the REST layer maps `ServiceError` → HTTP status via the existing `From<ServiceError> for RestError` (PermissionDenied→403, EntityNotFound→404, Conflict→409, ValidationError→400/422, InternalError→500).

## Self-Check: PASSED

- Files verified present: `31-02-SUMMARY.md`, `genossi_service/src/application.rs`, `genossi_service_impl/src/application.rs`, `genossi_bin/src/lib.rs`
- Commits verified in git log: `62d9420`, `c86254d`, `da75377`
- `nix develop --command cargo test -p genossi_service_impl` → 445 passed, 0 failed
- `nix develop --command cargo build -p genossi_bin` → compiles

---
*Phase: 31-service-rest-versand-versand-guardrails*
*Completed: 2026-08-20*
