---
phase: 31-service-rest-versand-versand-guardrails
plan: 03
subsystem: api
tags: [application, mail, rest, axum, utoipa, preview, communications, guardrails, e2e]

# Dependency graph
requires:
  - phase: 31-service-rest-versand-versand-guardrails
    plan: 02
    provides: "ApplicationService::send_mail / preview_mail / last_sent_at + ApplicationMailInput/Draft/RenderedApplicationMail"
  - phase: 31-service-rest-versand-versand-guardrails
    plan: 01
    provides: "render_application_content shared render kernel (D-06)"
  - phase: 29-application-recipient-linkage
    provides: "CommunicationDao::get_application_communications + RecipientInput.application_id"
provides:
  - "POST /api/applications/{id}/mail — admin-gated single-recipient send (200/401/404/409/400), no free-text recipient (D-13)"
  - "POST /api/applications/{id}/mail/preview — synchronous preview via the shared application render kernel (D-06)"
  - "GET /api/applications/{id}/communications — admin-gated outbound applicant timeline (CommunicationEntryTO)"
  - "ApplicationTO.last_sent_at — server-side aggregated anti-double-send field, populated only by get_application (APHIST-02)"
  - "SendApplicationMailRequest / PreviewApplicationMailRequest / PreviewApplicationMailResponse TOs"
affects: [phase-32-frontend-wiring, mail-preview-ui, application-communication-timeline-ui]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Thin REST handlers mirror confirm_application: extract context+id, delegate to service, error_handler wraps the async block"
    - "Admin gate for communications runs via application_service().get() FIRST (Pitfall 3, T-31-01), NOT the ungated communication_rest handler"
    - "last_sent_at aggregated server-side on get_application (never client-computed)"

key-files:
  created: []
  modified:
    - genossi_rest_types/src/lib.rs
    - genossi_rest/src/application.rs
    - genossi_bin/tests/e2e_tests.rs

key-decisions:
  - "SendApplicationMailRequest has NO address/to/recipient field (D-13): recipient is always application.email, resolved server-side"
  - "MailDaoError from communication_dao mapped to RestError::InternalError inline (no From impl exists across the crate boundary)"
  - "ApplicationTO.last_sent_at defaults to None in From<&Application>; only get_application overwrites it — wire-compat for all other endpoints"

patterns-established:
  - "Guardrail proof at the wire level: an E2E test posts rogue to/address/recipient JSON fields and asserts the seeded recipient still carries application.email"
  - "communications timeline deserialized as serde_json::Value in the test to assert direction/subject without importing the mail-crate TO"

requirements-completed: [APMAIL-01, APMAIL-02, APCMP-01, APCMP-02, APHIST-02]

coverage:
  - id: D1
    description: "POST /{id}/mail happy path: exactly ONE application-bound recipient (member_id NULL, application_id set), address = application.email"
    requirement: APMAIL-01
    verification:
      - kind: e2e
        ref: "genossi_bin/tests/e2e_tests.rs#test_application_mail_send_happy_single_recipient"
        status: pass
    human_judgment: false
  - id: D2
    description: "POST /{id}/mail on a non-Offen application returns 409 over the real HTTP path"
    requirement: APCMP-01
    verification:
      - kind: e2e
        ref: "genossi_bin/tests/e2e_tests.rs#test_application_mail_send_non_offen_returns_409"
        status: pass
    human_judgment: false
  - id: D3
    description: "POST /{id}/mail on an application with no email returns 400 (ValidationError → BadRequest)"
    requirement: APCMP-02
    verification:
      - kind: e2e
        ref: "genossi_bin/tests/e2e_tests.rs#test_application_mail_send_no_address_returns_400"
        status: pass
    human_judgment: false
  - id: D4
    description: "GET /{id}/communications returns the outbound applicant timeline (one entry, direction=outbound), admin-gated via application_service().get()"
    requirement: APMAIL-02
    verification:
      - kind: e2e
        ref: "genossi_bin/tests/e2e_tests.rs#test_application_mail_communications_timeline"
        status: pass
    human_judgment: false
  - id: D5
    description: "get_application aggregates last_sent_at server-side: None before any outbound send, Some after"
    requirement: APHIST-02
    verification:
      - kind: e2e
        ref: "genossi_bin/tests/e2e_tests.rs#test_application_mail_last_sent_at_aggregation"
        status: pass
    human_judgment: false
  - id: D6
    description: "POST /{id}/mail/preview resolves {{ open_amount }} against the application context via the shared render kernel (shares 2 × 10000 cents = 200,00 €)"
    requirement: APMAIL-01
    verification:
      - kind: e2e
        ref: "genossi_bin/tests/e2e_tests.rs#test_application_mail_preview_renders_open_amount"
        status: pass
    human_judgment: false
  - id: D7
    description: "Guardrail (D-13): rogue to/address/recipient JSON fields are ignored — the recipient row still carries application.email"
    requirement: APCMP-02
    verification:
      - kind: e2e
        ref: "genossi_bin/tests/e2e_tests.rs#test_application_mail_send_ignores_freetext_recipient"
        status: pass
    human_judgment: false

# Metrics
duration: 10min
completed: 2026-08-20
status: complete
---

# Phase 31 Plan 03: REST + E2E Versand-Scheibe Summary

**Three admin-gated Application-mail routes (`POST /{id}/mail`, `POST /{id}/mail/preview`, `GET /{id}/communications`) plus the server-side aggregated `last_sent_at` field on `get_application` — thin handlers delegating to the 31-02 service, with the communications gate running through `application_service().get()` (not the ungated communication_rest handler), all proven by seven real-HTTP E2E tests.**

## Performance

- **Duration:** ~10 min
- **Started:** 2026-08-20T21:21Z (after 31-02 completion commit)
- **Completed:** 2026-08-20T21:31Z
- **Tasks:** 3
- **Files modified:** 3

## Accomplishments
- `SendApplicationMailRequest` (deliberately WITHOUT any recipient field — D-13), `PreviewApplicationMailRequest`, `PreviewApplicationMailResponse` TOs plus an optional `last_sent_at: Option<String>` on `ApplicationTO` (`skip_serializing_if=None`); `From<&Application>` sets it to `None` so every other endpoint stays byte-compatible.
- Three thin handlers on `genossi_rest/src/application.rs` following the `confirm_application` pattern (`#[instrument]` + `#[utoipa::path]` + `error_handler`): `send_application_mail`, `preview_application_mail`, `get_application_communications`. Routes hang on the existing `generate_route` nest under `/api/applications` (no new `nest` in `lib.rs`), and all three handlers + four schemas are registered in `ApiDoc`.
- The communications handler enforces the admin gate via `application_service().get(id, context)?` FIRST (Pitfall 3, T-31-01) — Permission→401, existence→404 — and only then reads `communication_dao().get_application_communications(id)`. It does NOT mirror the ungated `communication_rest` handler.
- `get_application` now calls `application_service().last_sent_at(id, ...)` and formats the result as ISO8601 into `ApplicationTO.last_sent_at` (APHIST-02 anti-double-send guard, server-side aggregated).
- Seven E2E tests over the real HTTP path prove: send happy/single-recipient/namespace (member_id NULL + application_id set), 409 non-Offen, 400 missing-address, communications timeline, last_sent_at aggregation, `{{ open_amount }}` preview render, and the no-free-text-recipient guardrail.

## Task Commits

Each task was committed atomically:

1. **Task 1: Request/Response TOs + last_sent_at on ApplicationTO** — `1a7342f` (feat)
2. **Task 2: Three REST handlers + routes + OpenAPI + last_sent_at wiring** — `4ef53d4` (feat)
3. **Task 3: E2E tests — guards, timeline, last_sent_at, preview, guardrail** — `9e27d12` (test)

_Task 3 is marked `tdd="true"`. As in Plans 31-01 and 31-02, the production handlers (Task 2) must exist for the E2E test crate to compile, so a tests-only RED commit could not compile in isolation. The seven tests were written against the Task 2 handlers and all pass on first run (7 passed, 0 failed); the 409/400 tests prove the guards over the real HTTP path and the guardrail test proves the wire-level no-free-text-recipient invariant._

## Files Created/Modified
- `genossi_rest_types/src/lib.rs` — three mail TOs (send-request without recipient field) + optional `last_sent_at` on `ApplicationTO`; `From<&Application>` sets `last_sent_at: None`.
- `genossi_rest/src/application.rs` — three handlers + three routes + `ApiDoc` path/schema registration + `last_sent_at` population in `get_application`; imports for the new TOs, service I/O types, and `CommunicationEntryTO`.
- `genossi_bin/tests/e2e_tests.rs` — two test helpers (`create_offen_application`, `seed_application_mail_config`) + seven `application_mail` E2E tests.

## Decisions Made
- `SendApplicationMailRequest` carries only `subject`/`body`/`body_html`/`template_id` — no `address`/`to`/`recipient` (D-13); the recipient is always `application.email`, resolved in the service.
- `communication_dao().get_application_communications(...)` returns `Result<_, MailDaoError>`; since no `From<MailDaoError> for RestError` exists across the crate boundary, the handler maps it inline to `RestError::InternalError`.
- `last_sent_at` is populated only by `get_application`; every other endpoint (list/create/confirm/reject/update) continues to emit `None` via the `From<&Application>` default — no wire break.

## Deviations from Plan

None — plan executed exactly as written. The three plan files were the only files touched; each build/test/fmt ran through `nix develop --command …` and only the changed files were formatted with file-scoped `rustfmt` (per the toolchain notes, to avoid the workspace-wide fmt footgun).

## Issues Encountered
- `node`/`cargo`/`rustfmt` are not on the base PATH; every build/test/fmt ran through `nix develop --command …` (toolchain constraint). `node` for the GSD tooling was sourced from the Nix devshell store path.

## User Setup Required
None — no external service configuration; the change is REST wiring + TOs + E2E tests (no new dependency).

## Next Phase Readiness
- The three endpoints + `last_sent_at` field are live and OpenAPI-registered under `/api/applications`; Phase 32 can wire the frontend send/preview/timeline UI against them (D-06 preview endpoint is ready).
- Guards are enforced in the service (31-02) and now proven over the HTTP transport: PermissionDenied→401, EntityNotFound→404, Conflict→409, ValidationError→400, InternalError→500.

## Self-Check: PASSED

- Files verified present: `31-03-SUMMARY.md`, `genossi_rest_types/src/lib.rs`, `genossi_rest/src/application.rs`, `genossi_bin/tests/e2e_tests.rs`
- Commits verified in git log: `1a7342f`, `4ef53d4`, `9e27d12`
- `nix develop --command cargo test -p genossi_bin --test e2e_tests application_mail` → 7 passed, 0 failed
- `nix develop --command cargo build -p genossi_rest` + `-p genossi_bin` → compile

---
*Phase: 31-service-rest-versand-versand-guardrails*
*Completed: 2026-08-20*
