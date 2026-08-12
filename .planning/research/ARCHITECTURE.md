# Architecture Research

**Domain:** Applicant-email integration into an existing layered Rust (DAO → Service → REST → Dioxus) codebase — Milestone v1.6 "Antragsteller-Kommunikation"
**Researched:** 2026-08-12
**Confidence:** HIGH (all findings from direct source reading of the current codebase; no external/uncertain sources)

## Standard Architecture

This is **integration research**, not greenfield. The system already has every building block the milestone needs — a mail queue+worker, a template engine, a per-member communication timeline, and an Application entity whose service already depends on `config_service` + `mail_service`. The job is to thread `application_id` through the mail path the same way `member_id` already flows, and to add a parallel template context. The strong recommendation is **reuse + extend, do not rebuild**.

### System Overview

```
┌──────────────────────────────────────────────────────────────────────┐
│                         FRONTEND (Dioxus WASM)                        │
│  ┌────────────────────┐   ┌────────────────────┐  ┌────────────────┐ │
│  │ application_detail  │   │ NEW MailCompose-   │  │ Communication- │ │
│  │  (MODIFY: +button,  │──▶│  Dialog (compose   │  │ Timeline       │ │
│  │   +timeline section)│   │  building blocks)  │  │ (REUSE as-is)  │ │
│  └─────────┬───────────┘   └─────────┬──────────┘  └───────┬────────┘ │
│            │  api::send_application_mail / get_application_communications│
├────────────┼──────────────┼───────────────────────────────┼──────────┤
│                              REST (Axum)                               │
│  genossi_rest/src/application.rs        genossi_mail/communication_rest │
│  ┌──────────────────────────┐          ┌───────────────────────────┐  │
│  │ NEW POST /{id}/mail       │          │ NEW GET                    │  │
│  │  (+ optional GET /{id}/    │          │ /api/applications/{id}/    │  │
│  │   communications wrapper)  │          │  communications            │  │
│  └────────────┬─────────────┘          └────────────┬──────────────┘  │
├───────────────┼──────────────────────────────────────┼────────────────┤
│                            SERVICE                                     │
│  genossi_service_impl/src/application.rs   genossi_mail/src/service.rs │
│  ┌───────────────────────────┐            ┌────────────────────────┐  │
│  │ NEW send_mail(id, ...)     │───────────▶│ create_job(...) REUSE  │  │
│  │  (mirrors send_confirma-   │            │  RecipientInput +      │  │
│  │   tion_mail; renders app   │            │  application_id (MOD)  │  │
│  │   context; queues job)     │            └────────────────────────┘  │
│  │ genossi_mail/template.rs:   │                                        │
│  │  NEW application_to_        │                                        │
│  │  template_context + generic │                                        │
│  │  validate core             │                                        │
│  └───────────────────────────┘                                        │
├───────────────────────────────────────────────────────────────────────┤
│                          DAO / SQLite                                  │
│  genossi_mail/src/dao.rs + dao_sqlite.rs                              │
│  ┌────────────────────────────────────────────────────────────────┐  │
│  │ mail_recipients: ADD application_id BLOB (nullable) + index      │  │
│  │ MailRecipient struct: + application_id: Option<Uuid>             │  │
│  │ CommunicationDao: + get_application_communications(app_id)       │  │
│  └────────────────────────────────────────────────────────────────┘  │
└───────────────────────────────────────────────────────────────────────┘
```

### Component Responsibilities

| Component | Responsibility | New / Modified / Reuse |
|-----------|----------------|------------------------|
| `mail_recipients` table | Store per-recipient linkage; already has nullable `member_id` | **MODIFY**: add nullable `application_id` + index |
| `MailRecipient` / `RecipientInput` | Carry recipient identity into the queue | **MODIFY**: add `application_id: Option<Uuid>` |
| `MailService::create_job` | Queue a job + recipients | **REUSE**: no signature change — `application_id` rides on `RecipientInput` per recipient |
| `CommunicationDao` | Timeline query | **MODIFY**: add `get_application_communications`; the `CommunicationEntry` struct is already subject-agnostic |
| `application_to_template_context` | Build minijinja `Value` for an Application | **NEW** in `genossi_mail/src/template.rs` |
| `ApplicationService::send_mail` | Resolve app, render template, queue mail with `application_id` | **NEW** (mirrors existing `send_confirmation_mail`) |
| REST `POST /api/applications/{id}/mail` | Compose endpoint | **NEW** in `genossi_rest/src/application.rs` |
| REST `GET /api/applications/{id}/communications` | Timeline endpoint | **NEW** in `genossi_mail/communication_rest.rs` |
| `CommunicationTimeline` (frontend) | Render a timeline from `Vec<CommunicationEntryTO>` | **REUSE as-is** — the component is prop-driven, NOT member-bound |
| `mail_compose/*` building blocks | Subject input, WYSIWYG, template selector | **REUSE**; assemble into a NEW compose dialog |
| `application_detail.rs` | Application detail modal | **MODIFY**: add send button + compose dialog + timeline section |

## Recommended Project Structure

Changes stay inside existing crates/modules — no new crate.

```
genossi_mail/
├── src/
│   ├── dao.rs                 # MOD: MailRecipient.application_id; CommunicationDao method
│   ├── dao_sqlite.rs          # MOD: persist application_id; NEW query
│   ├── service.rs             # MOD: RecipientInput.application_id (create_job maps it through)
│   ├── template.rs            # NEW: application_to_template_context + generic validate core
│   ├── communication_rest.rs  # NEW: get_application_communications + application router
│   └── rest_templates.rs      # (only if applicant templates need a scoped list/validate)
migrations/sqlite/
│   └── 20260812xxxxxx_mail_recipients_add_application_id.sql   # NEW
genossi_service/
│   └── src/application.rs      # MOD: trait — add send_mail(...)
genossi_service_impl/
│   └── src/application.rs      # MOD: impl send_mail (reuse send_confirmation_mail scaffold)
genossi_rest/
│   ├── src/application.rs      # NEW: POST /{id}/mail handler + route + OpenAPI
│   └── src/lib.rs             # MOD: mount GET /api/applications/{id}/communications
genossi-frontend/
│   └── src/
│       ├── component/
│       │   ├── mail_compose/…          # REUSE building blocks
│       │   └── application_mail_dialog.rs   # NEW compose dialog component
│       ├── component/application_detail.rs  # MOD: button + timeline + dialog wiring
│       └── api.rs             # NEW: send_application_mail, get_application_communications
rest-types/
│   └── (add ApplicationMailRequest TO; CommunicationEntryTO already exists)
```

### Structure Rationale

- **Communication concerns stay in `genossi_mail/communication_rest.rs`:** it already owns `CommunicationEntryTO`, `CommunicationDao`, and the timeline abstraction. Adding an application variant there keeps the timeline single-sourced rather than forking a copy into `application.rs`.
- **The send action lives in `ApplicationService`, not `/api/mail`:** the service already holds `config_service` + `mail_service` and already has the proven `send_confirmation_mail` scaffold (`member_id: None` recipient). Sending from here means the endpoint resolves the `ApplicationEntity`, applies the `manage_members` permission check, computes the outstanding amount from config, and sets `recipient.application_id` in one authoritative place.

## Architectural Patterns

### Pattern 1: Recipient-scoped linkage (member_id ⟂ application_id)

**What:** `mail_recipients` already carries a nullable `member_id`. Add a sibling nullable `application_id`. A given recipient row links to a member **or** an applicant (mutually exclusive), never both. Timeline queries filter on whichever column is relevant.
**When to use:** any time an outbound mail must be attributable to a domain entity for history.
**Trade-offs:** two nullable FK-ish columns instead of a polymorphic `(subject_type, subject_id)` pair. Chosen because it mirrors the existing `member_id` convention exactly (lowest cognitive + migration cost) and SQLite has no real FK enforcement here anyway. A polymorphic column would be a gratuitous refactor of the working member path.

**Example:**
```rust
// genossi_mail/src/service.rs
pub struct RecipientInput {
    pub address: String,
    pub member_id: Option<Uuid>,
    pub application_id: Option<Uuid>,   // NEW — additive, defaults None everywhere
}
// create_job already loops recipients building MailRecipient — just copy the field.
```

### Pattern 2: Parallel template context, shared validate core

**What:** `member_to_template_context(&MemberEntity) -> Value` is the single source for member templates. Add `application_to_template_context(&ApplicationEntity, share_value_cents: i64) -> Value` producing the **same variable names where they overlap** (`first_name`, `last_name`, `salutation`, `title`, `email`, `shares`) plus applicant-specific `outstanding_amount` (German-formatted `shares × share_value_cents`). Do **not** invent a generic "one context for both" — the field sets genuinely differ (Member has ~20 fields incl. `member_number`, `join_date`, `current_shares`; Application has ~10 and no membership history).
**When to use:** distinct entities that share some but not all template variables.
**Trade-offs:** a little duplication in the two context builders vs. a leaky generic. Overlapping names keep the template author's mental model consistent (a salutation block written for members works for applicants).

**Keeping `validate_template` working:** `validate_template(subject, body, &[MemberEntity])` builds member contexts internally. Extract a generic core `validate_rendered(subject, body, contexts: &[Value]) -> Result<(), Vec<String>>` (strict-env syntax check + probe-render loop) and have the existing member wrapper delegate to it — **its public signature is unchanged**, so all existing tests and call sites keep compiling. Add an applicant wrapper `validate_application_template(subject, body, &[ApplicationEntity])` that builds application contexts (with a sample/dummy `share_value_cents`, mirroring the `dummy_repayment_context` sentinel pattern) and calls the same core.

**Example:**
```rust
// template.rs
fn validate_rendered(subject: &str, body: &str, contexts: &[Value]) -> Result<(), Vec<String>> { … }
pub fn validate_template(subject, body, members: &[MemberEntity]) -> Result<(), Vec<String>> {
    let ctxs = members.iter().map(member_to_template_context).collect::<Vec<_>>();
    validate_rendered(subject, body, &ctxs)   // signature preserved
}
```

### Pattern 3: Send-via-service, mirroring `send_confirmation_mail`

**What:** the closest existing pattern is `ApplicationServiceImpl::send_confirmation_mail(&app)` — it reads `share_value_cents`/bank config, computes the total, builds a body, and calls `mail_service.create_job(subject, body, None, vec![RecipientInput{ member_id: None }], …)`. The new `send_mail(id, req, ctx)` does the same but: (1) resolves the app by id, (2) checks `manage_members`, (3) renders a chosen template (or ad-hoc subject/body) against `application_to_template_context`, (4) sets `application_id: Some(id)` on the recipient.
**When to use:** any transactional/entity-scoped mail send.
**Trade-offs:** the send stays fire-and-forget (queue + worker) exactly like confirmation mail — no new delivery machinery.

## Data Flow

### Compose → Send Flow

```
[Vorstand clicks "E-Mail senden" on Application detail]
    ↓
MailComposeDialog (subject/body/template)  ── api::send_application_mail(app_id, req) ──▶
    ↓
POST /api/applications/{id}/mail  →  ApplicationService::send_mail
    ↓ (permission: manage_members; resolve app; read share_value_cents)
render template against application_to_template_context(app, share_value_cents)
    ↓
MailService::create_job(subject, body, body_html, [RecipientInput{ address: app.email, member_id: None, application_id: Some(id) }], …)
    ↓                                                        ↓
mail_jobs + mail_recipients rows (application_id set)   →  worker sends via SMTP (unchanged)
    ↓
202 Accepted (MailJobTO)
```

### Timeline Read Flow

```
[Application detail opens] ── api::get_application_communications(app_id) ──▶
GET /api/applications/{id}/communications → CommunicationDao::get_application_communications
    ↓  SELECT … FROM mail_recipients r JOIN mail_jobs j WHERE r.application_id = ?1 (outbound only)
Vec<CommunicationEntryTO>  ─▶  CommunicationTimeline { entries }   (component REUSED verbatim)
```

### Key Data Flows

1. **Linkage write:** `application_id` is set once, at `create_job` time, from `RecipientInput`. Everything downstream (worker, status updates) is agnostic to it.
2. **Applicant timeline is outbound-only:** inbound mails link via `inbound_mails.assigned_member_id` (members only). Applicants have no inbox assignment, so `get_application_communications` can drop the inbound `UNION` branch of the member query entirely — simpler and correct. (If inbound-to-applicant is ever wanted, it becomes a later, separate change.)

## Scaling Considerations

| Scale | Architecture Adjustments |
|-------|--------------------------|
| Current (single co-op, hundreds of applicants) | No change. The `WHERE application_id = ?` query with the new index is O(log n); the queue+worker already handles bulk member sends. |
| 10k+ applications | The added `idx_mail_recipients_application_id` index keeps timeline reads flat; no other change. |

### Scaling Priorities

1. **Index the new column** (mirror `idx_mail_recipients_member_id`) — the only performance-relevant step. Without it the timeline query full-scans `mail_recipients`.
2. Nothing else — the mail path is already queue-backed and worker-throttled.

## Anti-Patterns

### Anti-Pattern 1: Rendering the applicant body in the frontend and posting it to `/api/mail/send`

**What people do:** reuse the generic single-send endpoint by pre-rendering the template client-side.
**Why it's wrong:** (1) `/api/mail/send` sets `member_id: None` and has no `application_id` slot, so the mail never appears in the applicant timeline; (2) template rendering + `share_value_cents` config would leak to the client; (3) bypasses the `manage_members` permission check that the application service enforces.
**Do this instead:** send through `ApplicationService::send_mail`, which renders server-side and stamps `application_id`.

### Anti-Pattern 2: Auditing the new mail linkage / GV-style over-engineering

**What people do:** add `Auditable` + audit macros to `mail_recipients` because "everything writes are audited."
**Why it's wrong:** per CLAUDE.md only Member, MemberAction, MemberDocument, Application are audited; mail jobs/recipients are **not** audited today, and the milestone constraint explicitly exempts new GV entities. Adding audit here breaks the established boundary and the hash-chain scope.
**Do this instead:** treat `mail_recipients.application_id` as a plain additive column, consistent with the un-audited `member_id`.

### Anti-Pattern 3: A polymorphic `(subject_type, subject_id)` rewrite of `mail_recipients`

**What people do:** "unify" member + applicant linkage into one polymorphic column.
**Why it's wrong:** it refactors the working, tested member timeline for zero functional gain and forces a data migration of existing rows.
**Do this instead:** add a sibling nullable `application_id`, mirroring `member_id`.

### Anti-Pattern 4: Forking `CommunicationTimeline` for applicants

**What people do:** copy the timeline component because "it's for members."
**Why it's wrong:** the component takes `entries: Vec<CommunicationEntryTO>` and is entirely presentation — the member coupling lives only in the *API call* in `member_details.rs`, not the component. Forking violates the component-first principle in CLAUDE.md.
**Do this instead:** reuse `CommunicationTimeline` unchanged; only the data source (a different fetch) differs.

## Integration Points

### New vs. Modified — explicit, per layer

| Layer | File | New / Modified | Change |
|-------|------|----------------|--------|
| Migration | `migrations/sqlite/20260812xxxxxx_…add_application_id.sql` | NEW | `ALTER TABLE mail_recipients ADD COLUMN application_id BLOB;` + `CREATE INDEX idx_mail_recipients_application_id` |
| DAO | `genossi_mail/src/dao.rs` | MODIFY | `MailRecipient.application_id: Option<Uuid>`; `CommunicationDao::get_application_communications(app_id)` |
| DAO | `genossi_mail/src/dao_sqlite.rs` | MODIFY | persist/read `application_id` in recipient create + row mapping; NEW query (outbound-only) |
| Service | `genossi_mail/src/service.rs` | MODIFY | `RecipientInput.application_id`; `create_job` maps it onto `MailRecipient` (no signature change) |
| Service | `genossi_mail/src/template.rs` | NEW/MODIFY | NEW `application_to_template_context`; extract generic `validate_rendered` core; NEW `validate_application_template`; keep `validate_template` signature |
| Service | `genossi_service/src/application.rs` | MODIFY | add `send_mail(...)` to `ApplicationService` trait |
| Service | `genossi_service_impl/src/application.rs` | MODIFY | impl `send_mail` (reuse `send_confirmation_mail` scaffold: config read, context render, `create_job` with `application_id`) |
| REST | `genossi_rest/src/application.rs` | NEW | `POST /{id}/mail` handler, route in `generate_route`, OpenAPI path/schema |
| REST | `genossi_mail/src/communication_rest.rs` | NEW | `get_application_communications` handler + application router + OpenAPI |
| REST | `genossi_rest/src/lib.rs` | MODIFY | `.nest("/api/applications/{application_id}/communications", …)` + register OpenAPI doc |
| Types | `rest-types` | NEW | `ApplicationMailRequest` TO (subject/body/body_html/template_id); `CommunicationEntryTO` already exists and is reused |
| Frontend | `genossi-frontend/src/api.rs` | NEW | `send_application_mail(config, app_id, req)`; `get_application_communications(config, app_id)` |
| Frontend | `genossi-frontend/src/component/application_mail_dialog.rs` | NEW | compose dialog assembling `MailSubjectInput` + `WysiwygEditor` + `TemplateSelector` + send |
| Frontend | `genossi-frontend/src/component/application_detail.rs` | MODIFY | "E-Mail senden" button, dialog wiring, `CommunicationTimeline` section (reused) |

### Internal Boundaries

| Boundary | Communication | Notes |
|----------|---------------|-------|
| REST → Service (send) | `ApplicationService::send_mail` | Keeps permission + config + linkage server-side (Anti-Pattern 1) |
| Service → MailService | `create_job` with `RecipientInput{ application_id }` | Only touch-point where the linkage is written |
| DAO → REST (timeline) | `CommunicationDao::get_application_communications` → `CommunicationEntryTO` | `CommunicationEntry` is already subject-agnostic; no TO change |
| Frontend component ↔ data | `CommunicationTimeline { entries }` | Component reused verbatim; only the fetch differs |

## Reuse-Clean vs. Refactor-Warranted (for the Roadmapper)

- **Clean reuse (no change):** `CommunicationTimeline` component, `CommunicationEntry`/`CommunicationEntryTO` (already subject-agnostic), `MailService::create_job` signature, the queue+worker+SMTP send path, `mail_compose/*` building blocks, the `send_confirmation_mail` scaffold as a template.
- **Additive, low-risk:** `mail_recipients.application_id` column + index; `MailRecipient`/`RecipientInput` field; `ApplicationService::send_mail`; the two new REST endpoints; frontend api + dialog.
- **Warranted small abstraction:** extract `validate_rendered(contexts)` core so both member and applicant validation share it **without** changing `validate_template`'s signature. This is the one refactor that touches existing tested code — flag it for care (the member path has ~40 template tests that must stay green).
- **Explicitly rejected refactors:** polymorphic recipient linkage; a unified generic template context; auditing the mail linkage.

## Suggested Build Order (dependency-respecting)

1. **Phase A — DAO/schema foundation.** Migration (`application_id` + index) → `MailRecipient`/`RecipientInput` field → `create_job` maps it → `CommunicationDao::get_application_communications` + sqlite query + DAO tests. *(Schema/DAO before service — the service can't set a field the DAO doesn't persist.)*
2. **Phase B — Template context.** `application_to_template_context`; extract `validate_rendered`; `validate_application_template`; unit tests. *(Template context before both the service send and the frontend, since both render/validate against it.)*
3. **Phase C — Service + REST.** `ApplicationService::send_mail` (renders app context, queues job with `application_id`); `POST /api/applications/{id}/mail`; `GET /api/applications/{id}/communications`; mount routes + OpenAPI; service/e2e tests.
4. **Phase D — Frontend dialog.** api.rs calls; NEW `application_mail_dialog` composing mail_compose blocks; MODIFY `application_detail.rs` to add the send button + a communications section reusing `CommunicationTimeline`.

Rationale: **migration/DAO before service** (field must persist first), **template context before frontend and before the service send** (both depend on it), **backend endpoints before the frontend dialog** (the dialog needs something to call). Phases A/B are independent and could run in parallel; C depends on both; D depends on C.

## Sources

- `genossi_mail/src/service.rs` — `MailService::create_job`, `RecipientInput{ address, member_id }` (HIGH)
- `genossi_mail/src/template.rs` — `member_to_template_context`, `validate_template`, `merge_repayment_context`, strict/html envs, `dummy_repayment_context` (HIGH)
- `genossi_mail/src/dao.rs` / `dao_sqlite.rs` — `CommunicationEntry`, `CommunicationDao`, `get_member_communications` UNION query, `mail_recipients.member_id` (HIGH)
- `genossi_mail/src/communication_rest.rs` — `CommunicationEntryTO`, `CommunicationRestState`, `generate_route` (HIGH)
- `genossi_service_impl/src/application.rs` — `send_confirmation_mail` (`member_id: None`, config read, amount compute), `gen_service_impl!` deps incl. `config_service` + `mail_service` (HIGH)
- `genossi_dao/src/application.rs` — `ApplicationEntity` fields, `Auditable` impl (HIGH)
- `genossi_rest/src/application.rs` + `genossi_rest/src/lib.rs` — `generate_route`, `.nest("/api/applications…")`, communication route mounting at line 648 (HIGH)
- `genossi-frontend/src/component/{application_detail,communication_timeline,mail_compose/mod}.rs`, `page/member_details.rs` — prop-driven timeline, compose building blocks, member send/fetch pattern (HIGH)
- `migrations/sqlite/20260403000004_create_mail_recipients_table.sql`, `20260411000001_add_member_communication_indexes.sql` — existing `member_id` column + index to mirror (HIGH)

---
*Architecture research for: applicant-email integration (Genossi v1.6)*
*Researched: 2026-08-12*
