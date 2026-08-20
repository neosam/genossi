---
phase: 31-service-rest-versand-versand-guardrails
verified: 2026-08-20T19:41:11Z
status: passed
score: 9/9 must-haves verified
behavior_unverified: 0
overrides_applied: 0
---

# Phase 31: Service + REST Versand (Versand + Guardrails) Verification Report

**Phase Goal:** `ApplicationService::send_mail` → `Result<_, ServiceError>` (nicht das stille `()`-Pattern); Status-Guard `Offen`-only (409); `POST /api/applications/{id}/mail` + `POST /api/applications/{id}/mail/preview` + `GET /api/applications/{id}/communications`, admin-only; „zuletzt gesendet"-Daten (`last_sent_at`); Service- und E2E-Tests.

**Verified:** 2026-08-20T19:41:11Z
**Status:** passed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths (ROADMAP Success Criteria, merged with PLAN must_haves)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | `POST /api/applications/{id}/mail` versendet an `application.email` (`RecipientInput{member_id:None, application_id:Some}`), admin-only; Nicht-Admins → 403 | ✓ VERIFIED | `genossi_service_impl/src/application.rs:711-782` (`send_mail`, permission-check first, `RecipientInput{address, member_id:None, application_id:Some(app.id)}`); REST handler `genossi_rest/src/application.rs:511-539`; E2E `test_application_mail_send_happy_single_recipient` (genossi_bin/tests/e2e_tests.rs:15737) asserts exactly 1 recipient row with `member_id IS NULL AND application_id=?` and `to_address == "max@example.com"`; unit test `test_send_mail_permission_denied_no_side_effect` proves 403-path calls `create_job().never()` |
| 2 | `send_mail` gibt echten Erfolg/Fehler zurück (`Result<_, ServiceError>`), nie stilles 200-ohne-Versand | ✓ VERIFIED | Trait signature `genossi_service/src/application.rs:200` returns `Result<(), ServiceError>`; impl propagates 403/404/409/400/500 synchronously via `?`/`return Err`, no `tracing::error!+return ()` swallow pattern; unit test `test_send_mail_enqueue_error_propagates_500` proves enqueue failure surfaces as `InternalError`, not a silent `Ok` |
| 3 | Versand nur bei Status `Offen`; sonst HTTP 409 | ✓ VERIFIED | `genossi_service_impl/src/application.rs:730-736` status guard returns `ServiceError::Conflict`; E2E `test_application_mail_send_non_offen_returns_409` (reject then send) asserts `StatusCode::CONFLICT` over real HTTP |
| 4 | Service liefert „zuletzt gesendet am …" pro Application (aus outbound-Historie) | ✓ VERIFIED | `ApplicationService::last_sent_at` (`genossi_service/src/application.rs:223`, impl `genossi_service_impl/src/application.rs:841-877`) returns `MAX` over `get_application_communications(id).date`; `get_application` handler (`genossi_rest/src/application.rs:325-362`) populates `ApplicationTO.last_sent_at`; E2E `test_application_mail_last_sent_at_aggregation` proves `None` before, `Some` after a seeded send |
| 5 | Kein Massenversand-/Freitext-Empfänger-Pfad; Empfänger immer die Application selbst; kein Open-/Click-Tracking | ✓ VERIFIED | `SendApplicationMailRequest` (`genossi_rest_types/src/lib.rs`) has no address/to/recipient field; `send_mail` builds exactly one `RecipientInput` server-side from `app.email`; E2E `test_application_mail_send_ignores_freetext_recipient` posts rogue `to`/`address`/`recipient` JSON fields and asserts the stored recipient row still carries `application.email` |
| 6 | `POST /api/applications/{id}/mail/preview` liefert die aufgelöste Vorschau über den Application-Renderer (D-06, Preview == Worker-Output) | ✓ VERIFIED | `preview_mail` (`genossi_service_impl/src/application.rs:788-829`) calls the SAME kernel `genossi_mail::render::render_application_content` used by the worker (`genossi_mail/src/render.rs:161`, dispatched via `resolve_rendered_content`'s application branch at line 243); E2E `test_application_mail_preview_renders_open_amount` asserts `{{ open_amount }}` resolves to `format_eur_de(shares × share_value_cents)` = `"200,00 €"` |
| 7 | `GET /api/applications/{id}/communications` liefert outbound-Historie, admin-gegatet via `application_service().get()` (nicht der ungegatete `communication_rest`-Handler) | ✓ VERIFIED | `get_application_communications` handler (`genossi_rest/src/application.rs:599-632`) calls `application_service().get(id, ...)` FIRST (line 611-614), then `communication_dao().get_application_communications(id)`; E2E `test_application_mail_communications_timeline` seeds one outbound entry and asserts `direction=="outbound"`, `subject=="Zahlungserinnerung"` over the real HTTP path |
| 8 | Ein `mail_recipient` mit `application_id` rendert Betreff/Body/body_html über `application_to_template_context`; `open_amount` erscheint korrekt formatiert | ✓ VERIFIED | `genossi_mail/src/render.rs` Application branch (line 243) + `render_application_content` (line 161); render-crate test `resolve_rendered_content_application_branch_renders_open_amount` passes (independently re-run: 23/23 `render::` tests green) |
| 9 | `render_application_content` ist der eine geteilte reine Kernel (pub), von Worker UND Preview konsumiert (ein Renderer-Seam, D-06) | ✓ VERIFIED | `pub fn render_application_content` (`genossi_mail/src/render.rs:161`) is called from `resolve_rendered_content`'s application branch (worker/backfill path) AND directly from `ApplicationServiceImpl::preview_mail` (`genossi_service_impl/src/application.rs:809`) — exactly one call site pattern, no second render path found |

**Score:** 9/9 truths verified (0 present-but-behavior-unverified)

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `genossi_mail/src/template.rs` | `ApplicationResolver` trait (automock) | ✓ VERIFIED | `pub trait ApplicationResolver: Send + Sync + 'static` at line 25 |
| `genossi_mail/src/render.rs` | `ApplicationMailConfig`, `load_application_mail_config`, `render_application_content`, application branch | ✓ VERIFIED | All present; `recipient.application_id` dispatch at line 243; 4 new tests, all pass |
| `genossi_mail/src/worker.rs` / `backfill.rs` | Both call-sites updated with `AR`/`CS` args | ✓ VERIFIED | `cargo build -p genossi_bin` compiles with wired args (confirmed via test run) |
| `genossi_bin/src/lib.rs` | `PoolApplicationResolver` + worker/backfill wiring | ✓ VERIFIED | `PoolApplicationResolver` struct + `impl genossi_mail::template::ApplicationResolver`; wired at both `start_mail_worker` and `start_rendered_backfill_worker` call sites |
| `genossi_service/src/application.rs` | `send_mail`/`preview_mail`/`last_sent_at` trait methods + I/O types | ✓ VERIFIED | Lines 200/212/223 (methods), 98/108/117 (`ApplicationMailInput`/`ApplicationMailDraft`/`RenderedApplicationMail`) |
| `genossi_service_impl/src/application.rs` | Impl + `CommunicationDao` dependency | ✓ VERIFIED | `CommunicationDao: genossi_mail::dao::CommunicationDao = communication_dao` in `gen_service_impl!` (line 42); three method impls at 711/788/841 |
| `genossi_rest_types/src/lib.rs` | `SendApplicationMailRequest`/`PreviewApplicationMailRequest`/`PreviewApplicationMailResponse`, `ApplicationTO.last_sent_at` | ✓ VERIFIED | Referenced/used consistently in REST handlers and E2E tests; no address/to/recipient field on send request (guardrail test confirms) |
| `genossi_rest/src/application.rs` | Three handlers + routes + OpenAPI registration + `last_sent_at` wiring in `get_application` | ✓ VERIFIED | Handlers at 511/553/599; routes registered in `generate_route` (644-654); `ApiDoc.paths(...)` includes all three (line 666-668) + 4 new schemas (676-679); `get_application` populates `last_sent_at` (345-354) |
| `genossi_bin/tests/e2e_tests.rs` | E2E tests for guards, timeline, last_sent_at, guardrails | ✓ VERIFIED | 7 named tests present and independently re-run green (`test_application_mail_send_happy_single_recipient`, `_non_offen_returns_409`, `_no_address_returns_400`, `_communications_timeline`, `_last_sent_at_aggregation`, `_preview_renders_open_amount`, `_send_ignores_freetext_recipient`) |

### Key Link Verification

| From | To | Via | Status | Details |
|------|-----|-----|--------|---------|
| `resolve_rendered_content` (worker.rs, backfill.rs) | `render_application_content` | Application branch dispatch on `recipient.application_id` | ✓ WIRED | Line 243 `genossi_mail/src/render.rs`; both call sites (worker + backfill) pass required `AR`/`CS` args (build green) |
| `ApplicationServiceImpl::send_mail` | `MailService::create_job` | Single `RecipientInput` with `member_id: None, application_id: Some` | ✓ WIRED | `genossi_service_impl/src/application.rs:762-777`; E2E DB assertion confirms exactly 1 row, correct namespace |
| `ApplicationServiceImpl::preview_mail` | `genossi_mail::render::render_application_content` | Direct call, shared kernel | ✓ WIRED | Line 809-816; identical function used by worker's `resolve_rendered_content` |
| `get_application_communications` handler | `ApplicationService::get()` (admin gate) | Called BEFORE `communication_dao()` | ✓ WIRED | `genossi_rest/src/application.rs:611-618`; Pitfall-3 ordering confirmed by reading code, not just comment |
| `get_application` handler | `ApplicationService::last_sent_at` | Populates `ApplicationTO.last_sent_at` | ✓ WIRED | `genossi_rest/src/application.rs:345-354` |
| `genossi_rest::application::generate_route` | Axum router | `.route("/{id}/mail", ...)`, `.route("/{id}/mail/preview", ...)`, `.route("/{id}/communications", ...)` | ✓ WIRED | Lines 646-654; nested under existing `/api/applications` (no new top-level nest needed) |

### Behavioral Spot-Checks (independently re-run by verifier, not trusted from SUMMARY)

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Application render branch (worker path) | `nix develop --command cargo test -p genossi_mail render::` | 23 passed, 0 failed | ✓ PASS |
| Service-layer send/preview/last_sent_at guards | `nix develop --command cargo test -p genossi_service_impl application::` | 12 passed, 0 failed (8 new + 4 pre-existing confirm tests) | ✓ PASS |
| REST + E2E application-mail tests | `nix develop --command cargo test -p genossi_bin --test e2e_tests application_mail` | 7 passed, 0 failed | ✓ PASS |
| Git commits referenced in SUMMARYs exist | `git cat-file -e <hash>` for all 8 hashes | All 8 present in `git log` | ✓ PASS |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|--------------|--------|----------|
| APMAIL-01 | 31-01, 31-02, 31-03 | Admin can send single email to Offen application via `POST /{id}/mail`, correct RecipientInput namespace | ✓ SATISFIED | Marked `[x]` in REQUIREMENTS.md line 14; code + E2E evidence above |
| APMAIL-02 | 31-02, 31-03 | Real success/error (`Result<_, ServiceError>`), no silent 200-OK-without-send | ✓ SATISFIED | Marked `[x]` in REQUIREMENTS.md line 15; `send_mail` signature + 500-propagation unit test |
| APCMP-01 | 31-02, 31-03 | Send only when status Offen, else 409 | ✓ SATISFIED | Marked `[x]` in REQUIREMENTS.md line 34; 409 status guard + E2E test |
| APCMP-02 | 31-02, 31-03 | Content scoped to own application, no mass send, no tracking | ✓ SATISFIED | Marked `[x]` in REQUIREMENTS.md line 35; no-recipient-field schema + guardrail E2E test |
| APHIST-02 | 31-02, 31-03 | "zuletzt gesendet" prominently available (anti-double-send guard data) | ✓ SATISFIED | Marked `[x]` in REQUIREMENTS.md line 29; `last_sent_at` field wired into `get_application`, E2E test proves aggregation |

**Orphaned requirements check:** REQUIREMENTS.md traceability table (lines 85-89) maps exactly APMAIL-01, APMAIL-02, APCMP-01, APCMP-02, APHIST-02 to Phase 31 — identical to the plan-declared requirement set. No orphaned requirement IDs found.

### Anti-Patterns Found

None. Scanned all 9 phase-modified files for `TBD`/`FIXME`/`XXX`/`TODO`/`HACK`/`PLACEHOLDER`/"not yet implemented" markers — the only `XXX` hits are literal test-fixture BIC codes (`COBADEFFXXX`), not debt markers.

### Human Verification Required

None. All must-haves are backed by either passing automated tests (independently re-run) or direct code inspection of synchronous, non-behavior-ambiguous logic (permission ordering, status guards, request schema shape).

### Gaps Summary

No gaps found. All 9 derived observable truths (merging the 5 ROADMAP Success Criteria with the PLAN frontmatter must_haves across all three plans) are verified against the actual codebase — not just SUMMARY.md claims. All 5 requirement IDs (APMAIL-01, APMAIL-02, APCMP-01, APCMP-02, APHIST-02) are satisfied with concrete evidence, and the REQUIREMENTS.md traceability table shows no orphaned or missing entries for this phase. Independently re-running the render/service/E2E test suites (42 tests total across three crates) confirms all pass, and 8 referenced commit hashes exist in git history.

---

_Verified: 2026-08-20T19:41:11Z_
_Verifier: Claude (gsd-verifier)_
