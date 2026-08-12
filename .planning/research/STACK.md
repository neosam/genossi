# Stack Research

**Domain:** Applicant email communication (single-recipient mail + Application-context templates + per-applicant communication timeline) inside an existing Rust/Axum/SQLx/SQLite + Dioxus-WASM app
**Researched:** 2026-08-12
**Confidence:** HIGH

## Bottom Line

**Add nothing.** This milestone requires **zero new crates and zero version bumps.** All three new capabilities are pure reuse of dependencies already present and battle-tested in `genossi_mail` and `genossi-frontend`:

- **(a) minijinja templates against an Application context** → reuse `minijinja = "2"` (already in `genossi_mail`), plus the existing `strict_env()` / `html_env()` / `render_template()` / `render_html_template()` infrastructure in `genossi_mail/src/template.rs`. Only NEW code needed: a sibling function `application_to_template_context(&ApplicationEntity) -> minijinja::Value`, mirroring the existing `member_to_template_context`.
- **(b) Communication history linked to an `application_id`** → reuse `sqlx = "0.8"`. The only structural change is a **SQL migration** adding a nullable `application_id BLOB` column to the existing `mail_recipients` table (mirrors the existing nullable `member_id BLOB`). No crate.
- **(c) Compose dialog reusing Dioxus mail-compose components** → reuse `dioxus = "0.6.3"` and the entire existing `genossi-frontend/src/component/mail_compose/` suite + `communication_timeline.rs`. No crate.

The best precedent is already in the codebase: `send_confirmation_mail` (`genossi_service_impl/src/application.rs:44`) already sends a mail to an applicant via `MailService::create_job` with `RecipientInput { member_id: None }`. The new "send email to a single Application" endpoint is a generalization of that exact path.

## Recommended Stack

### Core Technologies (all already present — no changes)

| Technology | Version (current) | Purpose | Why it already covers this milestone |
|------------|-------------------|---------|--------------------------------------|
| minijinja | `2` (`genossi_mail/Cargo.toml:27`) | Template rendering (subject/body, text + autoescaped HTML) | `strict_env`/`html_env` + `render_template`/`render_html_template` are context-agnostic — they take any `minijinja::Value`. Rendering an Application context is identical to rendering a Member context; only a new context-builder function is needed. |
| sqlx | `0.8` (features `runtime-tokio, sqlite, time`) | Async SQLite access, migrations | Adding `application_id` to `mail_recipients` and a `get_application_communications` query are ordinary SQLx work. The existing `get_member_communications` UNION query (`dao_sqlite.rs:1042`) is the exact template to clone. |
| lettre | `0.11` (rustls/tokio1) | SMTP send | Untouched. `create_job` + worker already deliver to any `to_address`; applicant email is just `application.email`. |
| dioxus | `0.6.3` (features `web, router`) | WASM frontend | Untouched. Reuse existing compose components + timeline component on the Application detail page. |
| axum + utoipa | `0.8.3` / `5.0` | REST endpoints + OpenAPI | New routes (`POST /api/applications/{id}/mail`, `GET /api/applications/{id}/communications`) use the same handler/router/OpenAPI patterns already in `genossi_mail/src/rest.rs` and `communication_rest.rs`. |

### Supporting Libraries (already present — reused as-is)

| Library | Version (current) | Purpose | Reuse in this milestone |
|---------|-------------------|---------|-------------------------|
| ammonia | `4` | HTML sanitization at every mail entry point | Applicant HTML body flows through the same `sanitize_html` path — no change. |
| html2text | `0.17` | Derive plaintext part from rendered HTML | Same render-layer behavior applies to applicant mails automatically. |
| serde / serde_json | `1.0` | TOs + the `merge_repayment_context` round-trip pattern | The Application context builder can reuse the same `serde_json` → `BTreeMap` merge idiom if extra computed fields (e.g. `outstanding_amount`) are added. |
| time | `0.3` | Dates / `format_de` German date formatting | Reuse `format_de` for any date placeholders; euro-string formatting mirrors the existing `payout_amount` "X,YZ" pattern. |
| uuid | `1.6` | Entity IDs incl. `application_id` binding | Bind `application_id` as BLOB exactly like `member_id`. |

### Development Tools (unchanged)

| Tool | Purpose | Notes |
|------|---------|-------|
| sqlx-cli | Create + run the `application_id` migration | `sqlx migrate add add_application_id_to_mail_recipients --source migrations/sqlite`; run + `cargo sqlx prepare` per project convention. |
| Dioxus CLI (`dx`) | Frontend build/serve | No config change. |
| cargo fmt / clippy | Lint | No config change. |

## Installation

```bash
# No dependency installation required.
# The ONLY infrastructure change is a database migration:

sqlx migrate add add_application_id_to_mail_recipients --source migrations/sqlite
# → edit the generated .sql to: ALTER TABLE mail_recipients ADD COLUMN application_id BLOB;
#   (+ optional partial index mirroring idx_mail_recipients_member_id)

# Refresh offline query cache after adding the new query:
DATABASE_URL=sqlite:genossi.db cargo sqlx prepare
```

## Integration Points (what actually changes, all with existing crates)

1. **`genossi_mail/src/template.rs`** — add `application_to_template_context(&ApplicationEntity) -> Value` next to `member_to_template_context`. Placeholders available directly from `ApplicationEntity` (`genossi_dao/src/application.rs`): `salutation`, `title`, `first_name`, `last_name`, `shares`. Computed: `outstanding_amount = shares × share_value_cents` (config lookup, arithmetic only — `share_value_cents` is already read in `send_confirmation_mail`). Reuse `strict_env`/`html_env`.
2. **`mail_recipients` table + `MailRecipient`/`RecipientInput`** — add nullable `application_id`. `RecipientInput` (`service.rs:54`) gains `application_id: Option<Uuid>`. Persisted alongside `member_id`.
3. **`CommunicationDao`** — add `get_application_communications(application_id)` cloning the outbound half of the existing UNION query, filtering `WHERE r.application_id = ?1`. (Inbound side stays member-only this milestone — applicants have no `assigned_member_id`; timeline is outbound-only, which matches scope.)
4. **REST** — `POST /api/applications/{id}/mail` (render Application template server-side, then `create_job` with `RecipientInput { address, member_id: None, application_id: Some(id) }` — mirrors `send_confirmation_mail`) + `GET /api/applications/{id}/communications` (clone `communication_rest.rs`).
5. **Frontend** — Application detail page: "E-Mail senden" button + dialog reusing `component/mail_compose/*` and `communication_timeline.rs` (button precedent: `member_details.rs`).

## Alternatives Considered

| Recommended | Alternative | When the alternative would matter |
|-------------|-------------|-----------------------------------|
| Server-side render the Application template in the REST handler, then `create_job` with pre-rendered subject/body + `member_id: None` (exactly like `send_confirmation_mail`) | Extend the mail **worker** to resolve `application_id` → `ApplicationEntity` → context at send time (parallel to its member-resolution path) | Only if you need deferred/bulk applicant sends with per-recipient late binding. Out of scope this milestone (explicitly single-send, no bulk) — the simpler pre-rendered path avoids touching the worker entirely. |
| Reuse the shared Member/Application template pool with a common placeholder subset | Separate "Application template type" | If applicant templates diverge strongly from member templates. This is an open design question in PROJECT.md — either way it is a data/logic decision, **not** a dependency decision. |
| Add nullable `application_id` column to `mail_recipients` | New polymorphic `subject_type`/`subject_id` pair | Only if a third linkable entity appears later. Two nullable FK columns (`member_id`, `application_id`) is the lower-risk, precedent-following choice for exactly two subject types. |

## What NOT to Use / What NOT to Add

| Avoid | Why | Instead |
|-------|-----|---------|
| Any new templating crate (tera, handlebars, askama) | `minijinja` is already the project standard, strict-mode + autoescape env already built, and `{% if X is defined %}` guard pattern is established (PROJECT.md Key Decisions) | Reuse `minijinja = "2"`. |
| Bumping `minijinja` past `2` | No feature in this milestone needs it; the codebase deliberately notes `context! { ..base }`-spread is unsupported in 2.19 and works around it via serde_json merge | Keep `= "2"`; reuse `merge_repayment_context`'s merge idiom if needed. |
| A new ORM / query builder for the `application_id` join | SQLx raw `query_as` UNION is already the pattern for the communication timeline | Clone `get_member_communications`. |
| New Dioxus component library / CSS framework for the dialog | Compose components + Tailwind already exist and are Component-First-mandated | Reuse `component/mail_compose/*` + `communication_timeline.rs`. |
| Storing `outstanding_amount` as a new Application field | PROJECT.md constraint: applications have no payment-status field; amount is computed, not stored | Compute `shares × share_value_cents` at render time. |

## Version Compatibility

| Package | Status | Notes |
|---------|--------|-------|
| minijinja `2` | Unchanged | Strict + autoescape envs and `context!` macro used are all stable in 2.x; no upgrade required or recommended. |
| sqlx `0.8` (sqlite) | Unchanged | `ALTER TABLE ... ADD COLUMN` + nullable BLOB fully supported; matches existing `member_id` handling. Run `cargo sqlx prepare` after adding the new query. |
| dioxus `0.6.3` | Unchanged | Frontend is on `0.6.3`; note the known `dx build --release` / `wasm-bindgen-cli` tooling debt (PROJECT.md v1.0 Phase 04) is pre-existing and unrelated to this milestone. |
| SQLite | Unchanged | Forward-only migrations; project already avoids `DROP COLUMN` (SQLite < 3.35). Adding a nullable column is a safe forward migration; existing rows get `application_id = NULL`. |

## Sources

- `genossi_mail/Cargo.toml`, root `Cargo.toml` — verified current versions: minijinja 2, sqlx 0.8, lettre 0.11, ammonia 4, html2text 0.17, dioxus 0.6.3. (HIGH)
- `genossi_mail/src/template.rs` — verified `member_to_template_context`, `strict_env`/`html_env`, `render_template`/`render_html_template`, `merge_repayment_context` context-agnostic render path. (HIGH)
- `genossi_mail/src/service.rs` — verified `MailService::create_job` + `RecipientInput { address, member_id: Option<Uuid> }`. (HIGH)
- `genossi_mail/src/dao.rs`, `dao_sqlite.rs:1042` — verified `MailRecipient` shape and `get_member_communications` UNION query. (HIGH)
- `migrations/sqlite/20260403000004_create_mail_recipients_table.sql` — verified `mail_recipients` has nullable `member_id BLOB`, **no** `application_id`. (HIGH)
- `genossi_dao/src/application.rs` — verified `ApplicationEntity` fields (salutation/title/first_name/last_name/email/shares) available as placeholders. (HIGH)
- `genossi_service_impl/src/application.rs:44` — verified `send_confirmation_mail` already sends applicant mail via `create_job` with `member_id: None` (precedent). (HIGH)
- `genossi-frontend/src/component/mail_compose/` + `communication_timeline.rs` — verified existing compose + timeline components to reuse. (HIGH)

---
*Stack research for: applicant email communication (v1.6) — pure reuse, no new dependencies*
*Researched: 2026-08-12*
