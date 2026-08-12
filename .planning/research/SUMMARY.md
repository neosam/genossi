# Project Research Summary

**Project:** Genossi — Milestone v1.6 "Antragsteller-Kommunikation"
**Domain:** Applicant-facing transactional email (payment reminders) in an existing layered Rust/Axum/SQLx/SQLite + Dioxus-WASM member-management app
**Researched:** 2026-08-12
**Confidence:** HIGH

## Executive Summary

This milestone is pure integration work, not greenfield. Genossi already has a complete, battle-tested mail subsystem - a template engine (minijinja), a mail queue+worker, a per-member communication timeline, and even the exact precedent for sending mail to a not-yet-member (`send_confirmation_mail`, which already sends to an `Application` with `member_id: None`). The job for v1.6 is to generalize that one existing path into a user-triggered, single-send "E-Mail senden" action on the Application-Detail page, backed by a new `application_id` linkage so a per-applicant communication history can exist. No new crates, no version bumps - the stack conclusion is literally "add nothing."

The domain itself is narrow and legally anchored: an applicant who has signed a Beitrittserklärung already has a contractual obligation to pay their Geschäftsanteil (§§ 15/15a GenG), so a payment reminder about *their own declared payment* is lawful under Art. 6(1)(b) DSGVO without separate consent - but only while their status is `Offen`, and only about their own obligation. This content-scoping is the single most important boundary: it rules out bulk sends, auto-dunning, tracking, newsletters, and anything not tied to the applicant's own application. The feature set is deliberately small: single compose-and-send, reusable applicant-context templates, a computed (never stored) "offener Betrag," and a communication timeline - all built by threading `application_id` through the mail path exactly as `member_id` already flows.

The main risk is not "can we build this" but "will we quietly copy the wrong parts of the existing member-mail machinery." Research surfaced two load-bearing hazards: (1) the current template context/validator is hard-typed to `MemberEntity` with strict-undefined rendering, so an application-context render must get its own context builder and validator, not a copy with fields deleted; and (2) history does **not** automatically carry over when an applicant is confirmed into a member - `confirm()` mints a brand-new member UUID with no link back to the application, so the linkage design decided in the DAO/schema phase must explicitly solve carry-over (back-fill at confirm, union-at-read, or a stored link), or the Vorstand loses reminder history at the exact moment it matters. Everything else (status-guard against reminding rejected/converted applicants, `Result`-returning send instead of the silent-`()` confirmation-mail pattern, correct euro-cent arithmetic, and Component-First frontend reuse) is well-understood and low-risk, following established codebase conventions.

## Key Findings

### Recommended Stack

**Add nothing.** All required capabilities are pure reuse of dependencies already present: `minijinja = "2"` for templates (add a sibling `application_to_template_context` next to `member_to_template_context`), `sqlx = "0.8"` for a schema migration adding a nullable `application_id BLOB` to `mail_recipients` (mirrors existing nullable `member_id`), `lettre`/the existing worker for delivery (untouched), and `dioxus = "0.6.3"` + existing `mail_compose/` + `communication_timeline.rs` components for the frontend. The only infrastructure change in the entire milestone is one SQL migration plus `cargo sqlx prepare`.

**Core technologies (unchanged versions, reused as-is):**
- minijinja `2` - subject/body rendering; context-agnostic, only needs a new context-builder function
- sqlx `0.8` - new nullable column + one new UNION-style query, cloning `get_member_communications`
- axum + utoipa `0.8.3`/`5.0` - two new routes following existing handler/OpenAPI patterns
- dioxus `0.6.3` - new compose dialog assembled from existing building blocks, no new UI library

### Expected Features

**Must have (table stakes, v1.6 launch scope):**
- Single transactional email to one Application (`recipient = application.email`), admin-only, via `POST /api/applications/{id}/mail`
- Compose dialog reusing `mail_compose/` + WYSIWYG - no forked UI
- Reusable applicant templates (Anrede, Name, Titel, Anzahl Anteile, offener Betrag) via `application_to_template_context`
- "Offener Betrag" computed on the fly (`shares × share_value_cents`) - never stored on Application
- Per-applicant communication timeline (`application_id`-linked), `GET /api/applications/{id}/communications`, reusing `communication_timeline.rs` unmodified
- Prominent "last email sent on ..." display - the core anti-spam/anti-duplicate guardrail
- Confirm-before-send + live preview with resolved placeholders
- Graceful "no email address" handling (disabled/annotated button, never silent failure)
- One shipped default German "Zahlungserinnerung" template

**Should have (v1.x follow-up, not this milestone):** template/subject recorded on the timeline entry; body snapshot / deep-link to exact sent content; second-tier reminder template variants.

**Explicit anti-features (out of scope, push back if requested):** bulk/mass reminder to all "Offen" applicants; automated dunning schedule/auto-escalation; open/click tracking pixels; newsletter/marketing content to applicants (§7 UWG risk); formal legal Mahnung mechanics (Verzug/interest/fees); a stored payment-status field on Application; attachments/generated PDFs to applicants; free-text arbitrary-recipient mailer; reply/inbox threading into the applicant timeline.

### Architecture Approach

Thread `application_id` through the existing mail pipeline exactly the way `member_id` already flows, without touching the working member path. `RecipientInput`/`MailRecipient` gain a sibling nullable `application_id: Option<Uuid>` (never overload `member_id`). A new `ApplicationService::send_mail` mirrors the proven `send_confirmation_mail` scaffold (resolve entity -> permission check -> render -> `create_job`) but must return `Result<MailJob, ServiceError>` instead of swallowing errors. `CommunicationDao` gets one new outbound-only query (applicants have no inbound-mail assignment). The frontend adds one new compose-dialog component and modifies `application_detail.rs` to host a send button + reused timeline - the timeline component itself needs zero changes since it's already prop-driven and entity-agnostic.

**Major components:**
1. `mail_recipients` schema + `RecipientInput`/`MailRecipient` - additive `application_id` column/field (DAO layer)
2. `application_to_template_context` + extracted generic `validate_rendered` core in `genossi_mail/src/template.rs` (template layer, keeps `validate_template`'s signature unchanged for the ~40 existing member template tests)
3. `ApplicationService::send_mail` in `genossi_service_impl/src/application.rs` - the authoritative place permission, status-guard, config-read, and linkage all happen (service layer)
4. Two new REST endpoints (`POST /api/applications/{id}/mail`, `GET /api/applications/{id}/communications`) following existing handler/OpenAPI conventions (REST layer)
5. New `application_mail_dialog` component assembling existing `mail_compose/*` blocks + reused `CommunicationTimeline`, wired into `application_detail.rs` (frontend)

### Critical Pitfalls

1. **Member-typed template context + strict-undefined rendering** - `member_to_template_context`/`validate_template` are hard-typed to `MemberEntity`, and minijinja's strict env turns any undefined variable into a hard render error. Build a dedicated `application_to_template_context` exposing only fields Applications actually have (no `member_number`/`current_balance`/etc.), and a matching `validate_application_template`. Decide explicitly whether templates are a shared pool or a separate "Antragsteller" type - a shared pool needs context-filtered variable buttons and dual-context validation or it's a latent render bomb.
2. **History carry-over gap at `confirm()`** - `confirm()` mints a brand-new member UUID with no stored link to the originating application, so a naive design leaves the new member's timeline empty even though reminders were sent as an applicant. This must be decided in the DAO/schema phase (back-fill `mail_recipients.member_id` at confirm, union-at-read, or a stored link column) and verified with an e2e test: reminder -> confirm -> visible in member timeline.
3. **Silent no-send** - `send_confirmation_mail` is explicitly the reference code, but it returns `()` and swallows every failure (`tracing::error!` + `return`). The user-triggered reminder send must return `Result<MailJob, ServiceError>` and propagate real errors so the REST layer never returns 200-OK-but-nothing-sent for a financially significant mail.
4. **No status guard** - nothing currently stops emailing a rejected, already-confirmed (now-member), or soft-deleted applicant. Gate the send on `status == Offen` (mirroring the existing `confirm`/`reject` 409 pattern) - this is also the DSGVO lawful-basis boundary, not just a UX nicety.
5. **Faking the linkage via `member_id`** - never stuff an application's UUID into `RecipientInput.member_id` to make the timeline "just work"; it poisons the member namespace (`find_sent_member_ids_by_job_id` and future member-id assumptions). Always add and use a separate nullable `application_id` column.

## Implications for Roadmap

Based on combined research, the dependency-respecting build order from ARCHITECTURE.md and PITFALLS.md converges cleanly onto **four phases**, matching the "Suggested Build Order" already validated in ARCHITECTURE.md:

### Phase 1: DAO / Schema Foundation (P-HIST)
**Rationale:** The service can't set a field the DAO doesn't persist, and the load-bearing history-linkage design decision (Pitfall 2 + Pitfall 4: carry-over at `confirm()`) must be settled before anything is built on top of it.
**Delivers:** Migration adding nullable `application_id BLOB` + index to `mail_recipients`; `MailRecipient`/`RecipientInput.application_id` field; `create_job` mapping it through; `CommunicationDao::get_application_communications` (outbound-only UNION branch); explicit decision + implementation for member-timeline carry-over at confirm.
**Addresses:** Per-applicant communication history, "last sent on ..." (FEATURES.md table stakes)
**Avoids:** Pitfall 2 (fake `member_id` linkage), Pitfall 4 (history carry-over gap), Pitfall 6 (migration/column-list traps - grep every `mail_recipients` SQL string), Pitfall 10 (don't track reminder state on the audited `ApplicationEntity`)

### Phase 2: Template Context (P-TMPL, P-AMT)
**Rationale:** Both the service send and the frontend preview depend on the rendering context existing; can run in parallel with Phase 1.
**Delivers:** `application_to_template_context(&ApplicationEntity, share_value_cents)`; extracted generic `validate_rendered` core (keeping `validate_template`'s signature unchanged); `validate_application_template`; a single `format_eur_de` helper handling negative/zero/thousands-separator correctly; default seeded "Zahlungserinnerung" template.
**Addresses:** Reusable applicant templates, computed "offener Betrag" (FEATURES.md table stakes + differentiators)
**Avoids:** Pitfall 1 (member-typed strict-render bomb), Pitfall 7 (euro arithmetic edge cases), Pitfall 8 (data minimization - no member financial fields in the context)

### Phase 3: Service + REST (P-SEND)
**Rationale:** Depends on both Phase 1 (linkage) and Phase 2 (context/formatting); this is where permission, status-guard, and DSGVO lawful-basis enforcement live.
**Delivers:** `ApplicationService::send_mail` returning `Result<MailJob, ServiceError>` (never the silent-`()` pattern); status guard restricting sends to `Offen`; `POST /api/applications/{id}/mail`; `GET /api/applications/{id}/communications`; service/e2e tests.
**Addresses:** Single-send core deliverable, admin-only gate, confirm-before-send precondition (FEATURES.md P1 items)
**Avoids:** Pitfall 3 (silent no-send), Pitfall 5 (no status guard -> unlawful/duplicate sends), Pitfall 8 (DSGVO basis), Pitfall 11 (optimistic-lock version convention, only if any application write is added)

### Phase 4: Frontend Dialog (P-FE)
**Rationale:** Needs a real endpoint to call; last because it's pure consumption of Phases 1-3.
**Delivers:** `api.rs` functions for the two new endpoints (dedicated, not rerouted member fns); new `application_mail_dialog` component assembling `mail_compose/*` blocks with an application-scoped variable-button set; `application_detail.rs` modified to add send button + reused `CommunicationTimeline` section; live preview + confirm-before-send UX; disabled-while-pending send button.
**Addresses:** Compose dialog reuse, live preview, last-sent display, graceful no-email handling (FEATURES.md P1 items)
**Avoids:** Pitfall 9 (duplicate sends / Dioxus `form onsubmit` reload footgun - use `div`+`onclick`+`r#type:"button"`), Pitfall 12 (member-bound variable buttons/API fns leaking into the application dialog)

### Phase Ordering Rationale

- **Schema before service before frontend** is a hard dependency chain: the `application_id` field must exist and be persisted before the service can stamp it, and the service endpoints must exist before the dialog has anything to call.
- **Template context is decoupled from schema** (Phase 1 and 2 can run in parallel) but both must land before Phase 3, since the send service renders through the template context and writes the linkage.
- **History carry-over (Pitfall 4) is deliberately placed in Phase 1**, not left as a frontend afterthought, because it's a data-model decision (which column, which timing) that would require a migration change if discovered late.
- **The `Result`-returning send (Pitfall 3) is called out in Phase 3's plan explicitly** so nobody defaults to copying `send_confirmation_mail`'s `()`-returning error-swallowing pattern just because it's the cited reference code.

### Research Flags

Phases likely needing deeper research during planning:
- **None flagged as needing external research.** All four phases have HIGH-confidence, directly-verified precedent in the existing codebase (ARCHITECTURE.md and PITFALLS.md cite exact file:line references throughout). This is integration work against known internal patterns, not unfamiliar territory.

Phases with standard, well-documented patterns (safe to skip `--research-phase`):
- **Phase 1 (DAO/Schema):** Directly clones `get_member_communications` and the existing `member_id` column convention.
- **Phase 2 (Template Context):** Directly clones `member_to_template_context` shape and `merge_repayment_context` euro-formatting idiom.
- **Phase 3 (Service/REST):** Directly clones `send_confirmation_mail` scaffold (with the one explicit correction: error propagation instead of swallowing) and existing REST handler/OpenAPI patterns.
- **Phase 4 (Frontend):** Directly clones `mail_compose/` + `communication_timeline.rs` + `member_details.rs`'s send/fetch wiring pattern, with the known `div`+`onclick`+`r#type:"button"` fix already documented in project memory.

One open product decision to surface explicitly during requirements/discuss-phase (not a research gap, a decision): **shared vs. separate template pool for member and applicant templates**, and **which linkage strategy resolves history carry-over at `confirm()`** (back-fill / union-at-read / stored link column) - both are flagged as open in PROJECT.md and should be settled before Phase 1/2 planning locks in schema details.

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | Verified directly against `Cargo.toml` files and existing source (`template.rs`, `service.rs`, `dao_sqlite.rs`); "add nothing" conclusion is unambiguous |
| Features | HIGH (domain/legal), MEDIUM (exact template-variable set) | Legal grounding (GenG, DSGVO Art. 6(1)(b), UWG distinction) is well-sourced; exact placeholder list and share-value source are flagged as small product decisions for discuss-phase |
| Architecture | HIGH | Every component/pattern verified against direct source reading with file:line citations; no external/uncertain sources used |
| Pitfalls | HIGH | Every pitfall grounded in actual v1.6-relevant code paths already read for this research; includes locked-test awareness (`test_auditable_fields_count`, strict-env guard tests) |

**Overall confidence:** HIGH

### Gaps to Address

- **Template pool structure (shared vs. separate "Antragsteller" type):** flagged in both FEATURES.md and PITFALLS.md as an explicit open design question in PROJECT.md - resolve during requirements/discuss-phase, before Phase 2 planning finalizes the context-builder/validator shape.
- **History carry-over strategy at `confirm()`:** three viable approaches identified (back-fill, union-at-read, stored link column) with trade-offs but no single research-mandated winner - this is a genuine product/architecture decision to make explicitly in Phase 1 planning, not defer.
- **`share_value_cents` source for "offener Betrag":** config value vs. RepaymentPhase default (`DEFAULT_SHARE_VALUE_CENT=10000`) vs. per-Genossenschaft setting - small decision, but it's a money figure in an outbound email, so it must be deliberate rather than assumed during Phase 2 planning.

## Sources

### Primary (HIGH confidence - direct codebase reading)
- `genossi_mail/Cargo.toml`, root `Cargo.toml` - current dependency versions
- `genossi_mail/src/template.rs` - `member_to_template_context`, strict/html envs, `validate_template`, `merge_repayment_context`, locked guard tests
- `genossi_mail/src/service.rs` - `MailService::create_job`, `RecipientInput`, explicit-address privacy rule for test-sends
- `genossi_mail/src/dao.rs`, `dao_sqlite.rs` - `MailRecipient`, `get_member_communications` UNION query, duplicated column-list locations
- `genossi_service_impl/src/application.rs` - `send_confirmation_mail` (reference scaffold and its error-swallowing anti-pattern), `confirm()` (new-member minting, optimistic-lock warning), `Auditable` field-count lock
- `genossi_dao/src/application.rs` - `ApplicationEntity` fields, `ApplicationStatus`, soft-delete filtering
- `genossi-frontend/src/component/{communication_timeline.rs, mail_compose/*, application_detail.rs}`, `page/member_details.rs` - reusable component boundaries
- `migrations/sqlite/20260403000004_create_mail_recipients_table.sql` - existing `member_id` column to mirror
- `.planning/PROJECT.md`, `CLAUDE.md` - milestone scope, audit rules, Component-First and forward-only-SQLite constraints
- Project memory: `dioxus-form-onsubmit-page-reload.md` - documented Dioxus form-reload footgun

### Secondary (MEDIUM confidence - German legal/domain sourcing)
- Genossenschaftsgesetz (GenG) §§ 7a, 15, 15a, 15b - Beitrittserklärung as binding written obligation
- IT-Recht Kanzlei / SLK Rechtsanwälte - Warenkorb-Erinnerung case law, distinguished as inapplicable (no pre-existing obligation) vs. this milestone's transactional basis
- Datenschutzbeauftragter Hamburg - member/customer email lawful-basis guidance

---
*Research completed: 2026-08-12*
*Ready for roadmap: yes*
