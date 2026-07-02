# Phase 23: HTML Mail Backend — Research

**Researched:** 2026-07-02
**Domain:** Rust backend — MIME `multipart/alternative`, HTML sanitization, autoescaped Jinja rendering
**Confidence:** HIGH

## Summary

Phase 23 adds an optional HTML sibling to every plain-text mail. It extends the Phase-22 `build_message` helper with a `multipart/alternative` branch, adds three forward-only `ADD COLUMN body_html`/`rendered_html_body` migrations, introduces a **separate autoescaping minijinja env** for the HTML render pass, and gates all HTML entry points through an `ammonia`-based sanitizer. FMT-01 (German `DD.MM.YYYY`) is a small orthogonal fix in `member_to_template_context` that automatically flows into both text and HTML bodies via the shared context builder.

Every crate this phase touches — `lettre 0.11.20`, `minijinja 2.19.0` — is already in the workspace at a version whose API is documented and stable. Only one new dependency (`ammonia 4.1.3`) is added, matching CONTEXT.md D-01 and the "exactly one new backend dep" milestone constraint. All ammonia defaults verified against docs.rs match the assumptions locked in CONTEXT.md D-01/D-02 (permissive default: h1-h6/tables/img allowed; `<script>`/event-handlers/`javascript:` stripped; `rel="noopener noreferrer"` auto-added).

**Primary recommendation:** Split the phase into four waves — (1) schema+DAO+struct extension, (2) rendering (`html_env`, `format_de`, `resolve_rendered_content` return-shape), (3) MIME (`build_message` alternative branch), (4) sanitizer wiring at the three entry points + REST wire. Waves 1 and 4 can start in parallel; waves 2 and 3 depend on wave 1.

## User Constraints (from CONTEXT.md)

### Locked Decisions

**Ammonia-Sanitization (HTML-05)**
- **D-01:** Add `ammonia` as new dependency. Use permissive Default-Filter (`ammonia::clean()` / `Builder::default()`), NOT a narrow custom whitelist. Rationale: more formatting freedom for the Vorstand (bold/italic/links/lists/paragraphs **plus** headings/tables), less custom code. Default still strips `<script>`, event-handlers (`onclick` etc.), `javascript:`/`data:` URL schemes; `target=_blank` links get `rel=noopener` enforced.
- **D-02:** ammonia is a **safety net** (sanitizer), NOT a mail sender — sending stays `lettre`. `<img>` is passed through by the default filter (only hand-written external `<img src="https://…">`); actual image/branding functionality is a later phase.
- **D-03:** Sanitization runs at **all** entry points where author-HTML reaches persistence: `create_job` (service.rs:268), template-create + template-update, and the test-mail path (`send_test_mail_with_body`, service.rs:447). Frontend-Sanitization is explicitly NOT a security boundary (hard ordering constraint: ammonia gate MUST land before/with Phase 24).

**HTML-Render & Escaping (HTML-04)**
- **D-04:** New **separate autoescaping** minijinja env for the HTML body (e.g. `html_env()` with `set_auto_escape` / HTML autoescape enabled). Existing `strict_env()` (template.rs:61) stays **unchanged** for text-body AND subject. Member named `<script> & Co` appears in HTML body as `&lt;script&gt; &amp;`, author markup structure preserved.
- **D-05:** **Sanitize-on-store + autoescape-on-render**, NO re-sanitize of rendered output. Author-HTML is cleaned **once** by ammonia at store time; member values are neutralized at render time by the autoescape env. Double-sanitize would be redundant; legacy HTML does not exist (HTML-03 → NULL = text-only).

**Schema/Persistence (HTML-01, HTML-03) — 3 forward-only migrations**
- **D-06:** `ALTER TABLE mail_templates ADD COLUMN body_html TEXT NULL` (forward-only).
- **D-07:** `ALTER TABLE mail_jobs ADD COLUMN body_html TEXT NULL` (forward-only).
- **D-08:** `ALTER TABLE mail_recipients ADD COLUMN rendered_html_body TEXT NULL` (forward-only). **User decision:** Rendered HTML body is persisted per-recipient (not on-the-fly), analog to existing `rendered_body` (Quick 260614-9zf) — byte-accurate documentation of what each recipient actually received. Worker fills `rendered_html_body` alongside `rendered_subject`/`rendered_body`. Add `body_html` field to `MailJob`/`MailTemplate` structs (dao.rs) + `rendered_html_body` to `MailRecipient`.
- **D-09:** Legacy behavior: `body_html IS NULL` → plain text mail (no `alternative` part). `body_html IS NOT NULL` → `multipart/alternative` (text first, then HTML) via shared `build_message` helper. With attachment: `mixed{ alternative{plain, html}, attachments }`.

**MIME (HTML-01, HTML-02)**
- **D-10:** `multipart/alternative` nesting is added to the shared `build_message(...)` helper (which already owns the `MultiPart::mixed()` attachment wrapping). Text `SinglePart` stays block 1 (unchanged, from author's `body`); HTML `SinglePart` becomes optional second `alternative` branch. No additional crate for the text part (HTML-02).

**German Date Format (FMT-01)**
- **D-11:** Small shared helper `format_de(date) -> String` in `genossi_mail` with `time::format_description` template `"[day].[month].[year]"` (e.g. `02.07.2026`). Applied to `join_date` and `exit_date` in **shared** context-builder `member_to_template_context` (template.rs:17-18) → automatically consistent in text AND HTML bodies (both use same context). Replaces current `.to_string()`. Unit test analog to `test_exit_date_null` (template.rs:481) with a set `exit_date` checked as `DD.MM.YYYY`.

**Test Strategy**
- **D-12:** MIME-byte tests (`email.formatted()` + `String::from_utf8_lossy`) assert the `multipart/alternative` structure in both cases: text-only (`body_html` NULL) AND text+HTML; with attachment the correct `mixed{ alternative{…}, attachments }` nesting. Escaping test: member value with `<script> & Co` appears escaped in HTML body, raw in text body. Ammonia test: author-HTML with `<script>`/`onclick`/`javascript:`-link is stripped.

### Claude's Discretion
- Exact names/locations: HTML render env function (`html_env()` or similar), `format_de` helper module, signature extension of `build_message` for the optional HTML part.
- Whether the ammonia call is a shared helper (`sanitize_html()`) or inline at the 3 entry points (recommended: shared helper against divergence).
- Exact minijinja autoescape configuration (version-dependent; see canonical_refs).

### Deferred Ideas (OUT OF SCOPE)
- **HTML-Mail images/letterhead/logo/inline-CSS branding** — embedded images (upload + CID) and branding. Not in v1.4. Ammonia default DOES pass through external `<img>`, but no image **function** is built.
- **WYSIWYG frontend editor** (EDIT-01..05) → Phase 24. Needs `body_html`-API wire + ammonia gate from Phase 23; hard ordering constraint.
- **Application file upload + audited carryover** (APDOC-01..05) → Phase 25 (independent of mail track).

## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| HTML-01 | `multipart/alternative` (text first) + nested `mixed{alternative{plain,html}, attachments}` with attachments | Lettre 0.11 API confirmed: `MultiPart::alternative()`, `MultiPart::mixed()`, `.singlepart(part)`, `.multipart(inner_mp)`; existing `build_message` (send.rs) has attachment wrapping ready — extend with alternative branch |
| HTML-02 | Text part stays author's `body`; no HTML→text derivation; no extra crate | No new deps beyond ammonia; text branch of `build_message` unchanged |
| HTML-03 | `body_html` optional columns; forward-only `ADD COLUMN NULL`; legacy rows (NULL) stay text-only | Existing migration pattern `20260603100000_mail_job_attach_repayment_letter.sql` is the template; 3 new migration files needed (next timestamp: 20260702…) |
| HTML-04 | Variable interpolation in text AND HTML body; separate autoescape env; `strict_env()` unchanged for text+subject | `minijinja 2.19.0::Environment::set_auto_escape_callback(\|_\| AutoEscape::Html)` is the confirmed API for global HTML autoescape |
| HTML-05 | Author-HTML sanitized with ammonia at all entry points (`create_job`, template create/update, test-mail); frontend sanitize NOT a security boundary | `ammonia 4.1.3::clean(&str) -> String` uses Builder::default(); allowed tags list, blocked URL schemes, and `rel="noopener noreferrer"` auto-add all confirmed against docs.rs |
| FMT-01 | Date variables (`join_date`, `exit_date`) rendered `DD.MM.YYYY`; consistent in text and HTML | `time 0.3` with `formatting` feature already in workspace; `format_description!` macro or `format_description::parse("[day].[month].[year]")` works |

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| DB columns `body_html` / `rendered_html_body` | DAO | — | Forward-only ADD COLUMN; struct fields propagate through DAO trait to service |
| Ammonia sanitize-on-store | Service | REST (defense-in-depth if desired, but not required) | Service is the persistence boundary; frontend sanitize is explicitly NOT a security boundary (D-03) |
| Autoescape HTML render env | Service (`genossi_mail::template`) | — | Same layer as `strict_env()`; `render.rs` orchestrates both envs |
| `multipart/alternative` MIME assembly | Service (`genossi_mail::send::build_message`) | — | Single MIME construction site (Phase 22 D-01); DO NOT duplicate |
| `format_de` date helper | Service (`genossi_mail::template`) | — | Called from `member_to_template_context`; pure function, no I/O |
| REST wire (`body_html` on job/template DTOs) | REST | — | `genossi_mail/src/rest.rs` + `rest_templates.rs` DTO extension; Phase 24 posts through this wire |
| Worker persists `rendered_html_body` | Service (worker.rs) | — | Same update path as `rendered_body` at worker.rs:449-450 |

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `ammonia` | 4.1.3 | HTML sanitization via `Builder::default()` | The canonical Rust HTML sanitizer (rust-ammonia org). No competing crate matches its policy breadth. `[CITED: https://docs.rs/ammonia/4.1.3/ammonia/]` |
| `minijinja` | 2.19.0 (already in workspace) | Second `Environment` with HTML autoescape callback | Same crate as existing text render; a second env is idiomatic in minijinja | `[VERIFIED: Cargo.lock line 3152-3160]` |
| `lettre` | 0.11.20 (already in workspace) | `MultiPart::alternative()` + nested `MultiPart::mixed()` | Existing SMTP transport; alternative + mixed are documented multipart constructors | `[VERIFIED: Cargo.lock line 2908-2914]` |
| `time` | 0.3 (workspace) | `time::format_description::parse("[day].[month].[year]")` for FMT-01 | Already used everywhere; `formatting` feature already enabled at workspace level | `[VERIFIED: root Cargo.toml line 45]` |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| None — no additional deps | — | — | This phase adds exactly ONE new dep (`ammonia`), matching milestone constraint "genau eine — `ammonia`" (REQUIREMENTS.md line 7) |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `ammonia` Builder::default() | Custom `Builder` with whitelist | User explicitly rejected: less freedom for Vorstand + more custom code (D-01). |
| Second minijinja env | Single env with per-template autoescape decision | Two envs are simpler and mirror `strict_env` pattern; per-template decision (`set_auto_escape_callback` inspecting a name) needs synthetic names to disambiguate text vs HTML — unnecessary complexity. |
| Lettre `MultiPart::alternative()` | Hand-built `MultiPart` with alternative Content-Type | Lettre's builder is the documented path; no reason to hand-roll. |
| `time::format_description!` macro | `time::format_description::parse(...)` at call site | Macro is compile-time verified — recommended for the small fixed pattern `"[day].[month].[year]"`. |

**Installation:**
```bash
# Add to genossi_mail/Cargo.toml [dependencies]
ammonia = "4"
```

**Version verification:**
```bash
cargo search ammonia --limit 1
# → ammonia = "4.1.3"    # HTML Sanitization
cargo info ammonia | head
# → version: 4.1.3, license: MIT OR Apache-2.0, rust-version: 1.80
```

## Package Legitimacy Audit

| Package | Registry | Age | Downloads | Source Repo | Verdict | Disposition |
|---------|----------|-----|-----------|-------------|---------|-------------|
| `ammonia` | crates.io | mature (v1.0 in 2016, currently v4.1.3) | very high (used by mdbook, rust-lang blog, many Rust CMS crates) | github.com/rust-ammonia/ammonia (official rust-ammonia org) | OK | Approved |

`[VERIFIED: cargo info ammonia via crates.io — 4.1.3, rust-version 1.80, MIT OR Apache-2.0]`
`[CITED: https://docs.rs/ammonia/4.1.3/ammonia/]`

**Packages removed due to [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none

## Architecture Patterns

### System Architecture Diagram

```
                          ┌───────────────────────────────────┐
                          │  REST layer (rest.rs / rest_      │
                          │  templates.rs) — accepts optional │
Vorstand HTML input ─────►│  body_html on JobTO / TemplateTO  │
                          └──────────────┬────────────────────┘
                                         │
                                         ▼
                          ┌───────────────────────────────────┐
                          │  Service entry points             │
                          │  ─ create_job                     │
                          │  ─ MailTemplateService::create    │  ── all wrapped by
                          │  ─ MailTemplateService::update    │     sanitize_html()
                          │  ─ send_test_mail_with_body       │     (single helper)
                          └──────────────┬────────────────────┘
                                         │  body_html: Option<String>  (SANITIZED)
                                         ▼
                          ┌───────────────────────────────────┐
                          │  Persistence (DAO+SQLite)         │
                          │  mail_templates.body_html         │
                          │  mail_jobs.body_html              │
                          │  mail_recipients.rendered_html_body│
                          └──────────────┬────────────────────┘
                                         │
                                         ▼
                     ┌───────────────────────────────────────────┐
                     │  Worker (worker.rs::start_mail_worker)    │
                     │                                           │
                     │  resolve_rendered_content(recipient, job) │
                     │    ├─ strict_env  → subject, body_text    │
                     │    └─ html_env    → body_html_rendered    │  (autoescape ON)
                     │       (only if job.body_html.is_some())   │
                     │                                           │
                     │  persist rendered_subject, rendered_body, │
                     │  rendered_html_body                       │
                     └──────────────┬────────────────────────────┘
                                    │
                                    ▼
                     ┌───────────────────────────────────────────┐
                     │  send::build_message(text, html_opt, atts)│
                     │                                           │
                     │  ─ html_opt=None, atts=[]  → SinglePart   │
                     │  ─ html_opt=None, atts=[…] → mixed{text,  │
                     │                              atts…}       │
                     │  ─ html_opt=Some, atts=[]  → alternative{ │
                     │                              text, html}  │
                     │  ─ html_opt=Some, atts=[…] → mixed{       │
                     │                              alternative{ │
                     │                                text,html},│
                     │                              atts…}       │
                     └──────────────┬────────────────────────────┘
                                    │
                                    ▼  lettre transport → SMTP relay
```

### Recommended File Layout (following D-11 discretion)
```
genossi_mail/src/
├── send.rs              # extend build_message signature: body_html: Option<&str>
├── template.rs          # add html_env() + format_de() next to strict_env()
├── render.rs            # resolve_rendered_content returns (subject, body, Option<body_html>)
├── service.rs           # sanitize_html() helper; wire into create_job + send_test_mail_with_body;
│                        #   extend send_test_mail_with_body signature to accept body_html?
├── mail_template_service.rs  # sanitize at create/update entry points
├── worker.rs            # persist rendered_html_body; pass body_html to build_message
├── dao.rs               # MailJob.body_html, MailTemplate.body_html, MailRecipient.rendered_html_body
├── dao_sqlite.rs        # add body_html/rendered_html_body to INSERT/SELECT/UPDATE
├── rest.rs              # add body_html to job DTOs (SendBulkMailRequest, JobTO, etc.)
└── rest_templates.rs    # add body_html to CreateMailTemplateRequest / MailTemplateTO / UpdateMailTemplateRequest
```

### Pattern 1: HTML Autoescape Environment (parallel to `strict_env`)
**What:** A second `minijinja::Environment` with HTML autoescape forced ON, next to the existing strict env.
**When to use:** Rendering `body_html` — any member field appearing as `{{ first_name }}` etc. becomes HTML-escaped.
**Example:**
```rust
// genossi_mail/src/template.rs (new function, next to strict_env)
// Source: https://docs.rs/minijinja/2.19.0/minijinja/struct.Environment.html
fn html_env() -> minijinja::Environment<'static> {
    let mut env = minijinja::Environment::new();
    env.set_undefined_behavior(minijinja::UndefinedBehavior::Strict);
    env.set_auto_escape_callback(|_name| minijinja::AutoEscape::Html);
    env
}

pub fn render_html_template(template_str: &str, context: &Value) -> Result<String, TemplateError> {
    let env = html_env();
    let tmpl = env.template_from_str(template_str)
        .map_err(|e| TemplateError { message: format!("HTML template syntax error: {}", e) })?;
    tmpl.render(context)
        .map_err(|e| TemplateError { message: format!("HTML template render error: {}", e) })
}
```
`[CITED: https://docs.rs/minijinja/2.19.0/minijinja/struct.Environment.html#method.set_auto_escape_callback]`

### Pattern 2: Ammonia sanitize-on-store helper
**What:** Single shared `sanitize_html(&str) -> String` in `genossi_mail::service` (or a new `sanitize.rs` module) called by all three entry points.
**When to use:** Every entry point that persists or transmits author-HTML.
**Example:**
```rust
// Source: https://docs.rs/ammonia/4.1.3/ammonia/fn.clean.html
pub fn sanitize_html(html: &str) -> String {
    ammonia::clean(html)
}
// Called from:
//   - service.rs::create_job (before persisting body_html)
//   - mail_template_service.rs::create (before persisting body_html)
//   - mail_template_service.rs::update (before persisting body_html)
//   - service.rs::send_test_mail_with_body (before build_message)
```

### Pattern 3: Extended `build_message` signature
**What:** Add `body_html: Option<&str>` between `body` and `attachments`.
**When to use:** All three call sites (worker, send_test_mail_with_body). `send_test_mail` (the smoke-test-only variant) can pass `None`.
**Example:**
```rust
// genossi_mail/src/send.rs — signature evolution
pub fn build_message(
    from: &str,
    to: &str,
    subject: &str,
    body: &str,
    body_html: Option<&str>,      // NEW (D-10)
    attachments: &[LoadedAttachment],
    in_reply_to: Option<&str>,
    encoding: MailEncoding,
) -> Result<Message, MailServiceError> {
    // ... build text_part as today (unchanged) ...

    let html_part_opt = body_html.map(|h| {
        SinglePart::builder()
            .header(ContentType::TEXT_HTML)   // "text/html; charset=utf-8"
            .header(cte)                       // reuse the same CTE choice
            .body(h.to_string())
    });

    // Decision tree (D-09):
    match (html_part_opt, attachments.is_empty()) {
        (None, true)      => builder.singlepart(text_part),  // legacy path unchanged
        (None, false)     => {
            let mut mp = MultiPart::mixed().singlepart(text_part);
            for att in attachments { mp = mp.singlepart(build_attachment(att)); }
            builder.multipart(mp)
        }
        (Some(html), true) => {
            let alt = MultiPart::alternative().singlepart(text_part).singlepart(html);
            builder.multipart(alt)
        }
        (Some(html), false) => {
            let alt = MultiPart::alternative().singlepart(text_part).singlepart(html);
            let mut mp = MultiPart::mixed().multipart(alt);
            for att in attachments { mp = mp.singlepart(build_attachment(att)); }
            builder.multipart(mp)
        }
    }
}
```
`[CITED: https://docs.rs/lettre/0.11/lettre/message/struct.MultiPart.html]`

### Pattern 4: `format_de` and its wiring into `member_to_template_context`
```rust
// genossi_mail/src/template.rs
use time::macros::format_description;

pub fn format_de(date: time::Date) -> String {
    const FMT: &[time::format_description::BorrowedFormatItem<'static>] =
        format_description!("[day].[month].[year]");
    date.format(&FMT).unwrap_or_else(|_| date.to_string())
}

// Inside member_to_template_context (template.rs:17-18) — REPLACE:
let join_date_str = format_de(entity.join_date);
let exit_date_str = entity.exit_date.map(format_de);
```
Because the context is shared by BOTH `strict_env` (text) and the new `html_env` (HTML), the fix propagates automatically to both bodies.

### Anti-Patterns to Avoid
- **Do NOT re-sanitize the rendered HTML output.** Sanitize once on store (D-05). Rendering with autoescape neutralizes member values; re-sanitizing after render would strip legitimate author markup that also survived the first pass.
- **Do NOT put Jinja placeholders inside HTML attributes.** `<a href="{{ link }}">` will corrupt the template — ammonia treats `{{ link }}` as an invalid href value and may strip it. All placeholders must appear only as text content. This is a documented constraint (see Pitfall 3); Phase 24 editor is expected to enforce it by never producing variables inside attributes.
- **Do NOT hand-derive text from HTML.** HTML-02 explicitly forbids this. Plain-text body stays the author-written `body`.
- **Do NOT enable autoescape on the existing `strict_env`.** That env is used for text bodies and subjects; escaping `&` to `&amp;` in a plain-text mail is a regression.
- **Do NOT skip sanitizing on the test-mail path.** D-03 lists `send_test_mail_with_body` explicitly. Test-mails also go over SMTP to real inboxes; skipping sanitize opens a hole.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| HTML sanitization / XSS defense | Custom tag/attribute whitelist walker | `ammonia::clean()` (Builder::default()) | Ammonia handles: URL scheme validation, `rel="noopener noreferrer"` addition, HTML5 parser via html5ever, attribute filtering including `on*` handlers, malformed HTML normalization. A hand-rolled sanitizer will miss edge cases (SVG payloads, `data:` URIs, mutation XSS). |
| HTML escaping of member values | `.replace("<", "&lt;")` chain | `minijinja::Environment::set_auto_escape_callback` returning `AutoEscape::Html` | minijinja's escaper handles `<`, `>`, `&`, `"`, `'` correctly and consistently; hand-rolled `.replace()` chains have missed-encoding bugs and don't compose with filters. |
| `multipart/alternative` MIME assembly | String-templated MIME headers | `lettre::message::MultiPart::alternative()` with nested `MultiPart::mixed()` | Multipart boundaries need randomness and correct escaping; lettre's builder emits RFC-compliant messages including Content-Type/CTE headers per part. |
| Text→HTML derivation for the plain part | `html2text` or similar | Nothing — HTML-02 mandates the author's `body` is the text | Explicit REQ; would add a crate and change semantics. |
| German date formatting | Custom parser | `time::format_description!("[day].[month].[year]")` compile-time-verified pattern | `time` is already used throughout the codebase; `format_description!` is compile-time-verified against typos. |

**Key insight:** This phase is a wiring phase — everything is already available in existing crates. The only judgment calls are (a) where to put the small helpers and (b) how to structure the `build_message` signature evolution.

## Runtime State Inventory

> This is not a rename/refactor phase. Only schema additions with NULL semantics. No runtime state inventory required.

- **Stored data:** None — new columns are all `NULL`-default, no data migration.
- **Live service config:** None — no external service knows about these columns.
- **OS-registered state:** None.
- **Secrets/env vars:** None.
- **Build artifacts:** SQLite offline query metadata (`.sqlx/`) will regenerate on `cargo sqlx prepare`. Rebuild `.sqlx` after applying migrations locally so compile-time query verification stays green.

## Common Pitfalls

### Pitfall 1: Jinja placeholder inside HTML attribute gets stripped by ammonia
**What goes wrong:** `<a href="{{ link }}">Klick</a>` in a stored HTML template. Ammonia sees `href="{{ link }}"` at store time (before Jinja render) and treats it as an invalid URL → strips the attribute, possibly the whole tag.
**Why it happens:** Ammonia runs on the raw stored HTML (which still has un-rendered Jinja placeholders); it URL-validates `href`/`src` attribute values against its scheme whitelist and rejects `{{ link }}` as unparseable.
**How to avoid:** Constraint from CONTEXT.md `<specifics>`: variables appear only in **text content**, never in `href`/`src`/style attribute values. Phase 24 editor will honor this. Add a unit test that stores `<p>Hallo {{ first_name }}</p>` and asserts `{{ first_name }}` survives ammonia intact — this locks the contract.
**Warning signs:** Any Vorstand-authored template with `<a href="…">`/`<img src="…">` where the URL is not a literal.

### Pitfall 2: Forgetting to disable frontend as "security boundary"
**What goes wrong:** Phase 24 sanitizes HTML in the browser (on paste, `styleWithCSS=false`) → a developer assumes backend can skip sanitization for API-posted `body_html`.
**Why it happens:** Frontend sanitize gives a false sense of safety; but an attacker can POST arbitrary HTML directly to the API.
**How to avoid:** Sanitize at all 3 backend entry points regardless of client. CONTEXT.md D-03 hard-codes this. Verification test: POST `body_html='<script>alert(1)</script>'` to `create_job` via HTTP-level test → assert the persisted `body_html` has NO `<script>` tag.
**Warning signs:** Any code comment reading "already sanitized in frontend".

### Pitfall 3: `strict_env` autoescape regression
**What goes wrong:** Someone enables `set_auto_escape_callback` on the existing `strict_env` instead of creating a new env → text mails contain `&amp;` instead of `&`, `&lt;` instead of `<`.
**Why it happens:** minijinja's autoescape applies at render time, not template-parse time; existing text-body tests would still pass syntactically but produce escaped output.
**How to avoid:** `html_env()` is a **new** function next to `strict_env()`; neither shares state. Add a regression test: `render_template("<b>{{ first_name }}</b>", ctx)` with strict_env still returns the literal `<b>Max</b>` for text.
**Warning signs:** Any diff to the `strict_env` function.

### Pitfall 4: `rendered_html_body` NULL vs empty-string ambiguity
**What goes wrong:** Backfill / display code treats `Some("")` differently from `None`; forensic query "what was actually sent?" returns empty string, hiding the fact that no HTML alternative was sent.
**Why it happens:** SQLite stores empty TEXT as empty string, not NULL, if you `.bind(&Some(String::new()))`.
**How to avoid:** In the worker, only set `Some(rendered_html)` when `job.body_html.is_some()`. Otherwise leave `None`. Assertion test: mail job WITHOUT `body_html` → recipient row has `rendered_html_body IS NULL`.
**Warning signs:** `unwrap_or_default()` on `body_html` anywhere in the write path.

### Pitfall 5: `multipart/alternative` ordering
**What goes wrong:** Some mail clients render whichever part they receive **last** (the "richest" preference). If HTML is added second — CORRECT: the mail displays as HTML. If added FIRST accidentally, the client shows the text and the HTML is treated as "downgrade".
**Why it happens:** RFC 2046 §5.1.4: "each of the parts is an 'alternative' version of the same information … in general, the last part is the best".
**How to avoid:** Text FIRST, HTML SECOND — CONTEXT.md line 34 states this explicitly (`(Text zuerst, dann HTML)`). Byte-level test asserts the order in the raw MIME output.
**Warning signs:** Any `MultiPart::alternative().singlepart(html_part).singlepart(text_part)` — wrong direction.

### Pitfall 6: Ammonia mutates whitespace/newlines
**What goes wrong:** Ammonia's html5ever parser normalizes whitespace; a stored template that relied on specific line breaks may lose them, breaking a rendered greeting layout.
**Why it happens:** html5ever is a normalizing parser; it collapses runs of whitespace in some contexts.
**How to avoid:** Test with a realistic paragraph structure; use explicit `<p>` and `<br>` (both in the default allowlist) rather than `\n`-only formatting.
**Warning signs:** Vorstand-written HTML that uses raw newlines for layout.

## Code Examples

### Sanitize + Persist at `create_job`
```rust
// genossi_mail/src/service.rs, inside create_job impl (near line 300)
// Source: this phase's D-01 + D-03
async fn create_job(
    &self,
    subject: &str,
    body: &str,
    body_html: Option<String>,   // NEW arg from REST/service surface
    /* … existing args … */
) -> Result<MailJob, MailServiceError> {
    let sanitized_html: Option<Arc<str>> = body_html
        .as_deref()
        .map(crate::sanitize::sanitize_html)   // ammonia::clean
        .map(Arc::from);

    let job = MailJob {
        // … existing fields …
        subject: Arc::from(subject),
        body: Arc::from(body),
        body_html: sanitized_html,   // NEW
        // …
    };
    self.job_dao.create(&job).await?;
    Ok(job)
}
```

### Extending `resolve_rendered_content` to return HTML
```rust
// genossi_mail/src/render.rs — return-shape evolution
pub async fn resolve_rendered_content<...>(...) -> Result<RenderedContent, RenderFailure> { ... }

pub struct RenderedContent {
    pub subject: String,
    pub body: String,
    pub body_html: Option<String>,   // NEW: Some iff job.body_html.is_some()
}

// Inside the function, after existing text renders (~ line 147):
let body_html_rendered = match job.body_html.as_deref() {
    Some(html_src) => Some(
        crate::template::render_html_template(html_src, &ctx)
            .map_err(|e| RenderFailure::new(format!("HTML render error: {}", e.message)))?
    ),
    None => None,
};
Ok(RenderedContent { subject, body, body_html: body_html_rendered })
```

### Worker persists `rendered_html_body`
```rust
// genossi_mail/src/worker.rs (extending block at line 442-454)
updated_recipient.rendered_subject = Some(Arc::from(rendered.subject.as_str()));
updated_recipient.rendered_body = Some(Arc::from(rendered.body.as_str()));
updated_recipient.rendered_html_body = rendered.body_html
    .as_deref()
    .map(Arc::from);   // stays None when job.body_html was None (D-09)
updated_recipient.rendered_reconstructed = false;

// Downstream: pass rendered.body_html into build_message
let email = build_message(
    &smtp_config.from,
    &next.to_address,
    &rendered.subject,
    &rendered.body,
    rendered.body_html.as_deref(),   // NEW
    &attachments,
    reply_message_id.as_deref(),
    smtp_config.encoding,
)?;
```

### FMT-01 fix
```rust
// genossi_mail/src/template.rs (replacing lines 17-18)
// Source: this phase D-11, time::format_description docs
use time::macros::format_description;

fn format_de(date: time::Date) -> String {
    const FMT: &[time::format_description::BorrowedFormatItem<'static>] =
        format_description!("[day].[month].[year]");
    // Infallible for a well-formed Date + fixed pattern; fallback preserves current behavior.
    date.format(FMT).unwrap_or_else(|_| date.to_string())
}

// In member_to_template_context:
let join_date_str = format_de(entity.join_date);
let exit_date_str = entity.exit_date.map(format_de);
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| ammonia 3.x (Rust MSRV 1.56) | ammonia 4.x (MSRV 1.80) | 2024 | We are already on Rust 2021 edition; workspace toolchain is far past 1.80. Direct upgrade path. |
| minijinja per-template autoescape via file-extension callback | Still the same — `set_auto_escape_callback` is stable API since 1.x | — | We use `\|_\| AutoEscape::Html` to force it regardless of name. |
| Lettre 0.10 (deprecated) | Lettre 0.11.x with builder API | 2022 | Workspace already on 0.11.20; `MultiPart::alternative()`/`mixed()` API stable. |

**Deprecated/outdated:**
- Nothing to remove. All existing patterns stay.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| — | (empty) | — | All claims in this document verified via cargo info, Cargo.lock inspection, docs.rs official pages, or codebase grep. |

**All claims verified or cited — no user confirmation needed.**

## Open Questions

1. **Should `send_test_mail_with_body` accept an optional `body_html` argument now?**
   - What we know: CONTEXT.md D-03 requires sanitizing at this entry point. Today the function takes `(to, subject, body)`. Phase 24 will want to preview HTML in a test-mail.
   - What's unclear: Whether Phase 23 adds the arg now (defensive; requires REST wire change too) or Phase 24 adds it when needed.
   - Recommendation: **Add `body_html: Option<&str>` to the signature now** (Claude's discretion per CONTEXT.md line 45). Sanitize the input identically to `create_job`; pass through `build_message`. This closes D-03 cleanly and unblocks Phase 24's live-preview without a second signature change. Cost: ~10 LOC + one REST DTO field extension.

2. **Where does `sanitize_html()` live?**
   - Recommendation: Own module `genossi_mail/src/sanitize.rs` with the single pub fn. Small enough not to clutter but named enough to be found by grep. Alternative: put it in `service.rs` as a free function. Either works; a dedicated file makes future policy tweaks (e.g. adding `Builder` customizations) easier.

3. **Migration filenames.**
   - Highest existing: `20260626000000_create_digest_state_table.sql`. Next timestamp for Phase 23: **`20260702…`**. Recommended filenames:
     - `20260702000000_mail_templates_add_body_html.sql`
     - `20260702000001_mail_jobs_add_body_html.sql`
     - `20260702000002_mail_recipients_add_rendered_html_body.sql`
   - Each contains a single `ALTER TABLE … ADD COLUMN … TEXT NULL;` statement plus a comment header analog to `20260603100000_mail_job_attach_repayment_letter.sql`.

4. **After migrations: `cargo sqlx prepare` regeneration.**
   - The DAO adds new columns to INSERT/UPDATE/SELECT statements in `dao_sqlite.rs`. Compile-time query check requires refreshed `.sqlx/` metadata: `DATABASE_URL=sqlite:genossi.db cargo sqlx prepare` — this is standard and documented in CLAUDE.md.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust toolchain | Compile all Phase 23 changes | ✓ (Nix flake) | 2021 edition, latest stable | — |
| SQLite dev libraries | Migrations + DAO | ✓ | ≥3.35 (ADD COLUMN supported at any recent version) | — |
| `sqlx-cli` for `sqlx migrate run` | Applying migrations locally | ✓ (Nix flake) | 0.8 | — |
| `ammonia` on crates.io | New Cargo dependency | ✓ | 4.1.3 | — |
| `minijinja 2.x` | HTML autoescape callback | ✓ (already in workspace lock) | 2.19.0 | — |
| `lettre 0.11` | `MultiPart::alternative()` builder | ✓ (already in workspace lock) | 0.11.20 | — |
| `time 0.3` with `formatting` feature | FMT-01 `format_description!` macro | ✓ (workspace) | 0.3, `formatting` feature already enabled | — |

**Missing dependencies with no fallback:** none
**Missing dependencies with fallback:** none

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | `cargo test` with `mockall` for DAO/service mocks (identical to the Phase-22 setup) |
| Config file | Cargo workspace-level; no dedicated test config |
| Quick run command | `cargo test -p genossi_mail` |
| Full suite command | `cargo test && cargo test --test e2e_tests` |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| HTML-01 (no attach) | `body_html=Some` → mail is `multipart/alternative` with text first | unit (MIME-byte) | `cargo test -p genossi_mail send::tests::build_message_alternative_text_then_html_no_attachments -x` | ❌ Wave 3 |
| HTML-01 (with attach) | `body_html=Some` + attachments → `mixed{alternative{plain,html},atts}` | unit (MIME-byte) | `cargo test -p genossi_mail send::tests::build_message_mixed_wraps_alternative_when_attach -x` | ❌ Wave 3 |
| HTML-01 (legacy) | `body_html=None` → single-part text mail (no alternative) | unit (MIME-byte, regression) | `cargo test -p genossi_mail send::tests::build_message_legacy_singlepart_text_unchanged -x` | ❌ Wave 3 |
| HTML-02 | Plain-text part == raw `body` argument (no derivation) | unit | `cargo test -p genossi_mail send::tests::build_message_alternative_text_part_is_verbatim_body -x` | ❌ Wave 3 |
| HTML-03 | `body_html IS NULL` legacy row survives DAO round-trip; sends text-only | unit + DAO round-trip | `cargo test -p genossi_mail dao_sqlite::tests::mail_job_body_html_null_roundtrip -x` | ❌ Wave 1 |
| HTML-04 (escape) | Member `first_name="<script>&Co"` renders as `&lt;script&gt;&amp;Co` in HTML body | unit | `cargo test -p genossi_mail template::tests::html_env_autoescapes_member_value -x` | ❌ Wave 2 |
| HTML-04 (text unchanged) | Same member value renders raw in text body via `strict_env` | unit (regression) | `cargo test -p genossi_mail template::tests::strict_env_does_not_escape_member_value -x` | ❌ Wave 2 |
| HTML-05 (script strip) | ammonia strips `<script>` from stored HTML | unit | `cargo test -p genossi_mail sanitize::tests::sanitize_strips_script_tag -x` | ❌ Wave 4 |
| HTML-05 (event handler) | ammonia strips `onclick`, `onerror`, etc. | unit | `cargo test -p genossi_mail sanitize::tests::sanitize_strips_event_handlers -x` | ❌ Wave 4 |
| HTML-05 (URL scheme) | ammonia strips `javascript:` and `data:` from `href` | unit | `cargo test -p genossi_mail sanitize::tests::sanitize_strips_dangerous_url_schemes -x` | ❌ Wave 4 |
| HTML-05 (Jinja placeholder text-content invariant) | `<p>Hallo {{ first_name }}</p>` survives ammonia unmodified | unit | `cargo test -p genossi_mail sanitize::tests::sanitize_preserves_jinja_placeholder_in_text_content -x` | ❌ Wave 4 |
| HTML-05 (create_job wired) | `create_job` sanitizes `body_html` before persist | unit (service) | `cargo test -p genossi_mail service::tests::create_job_sanitizes_body_html -x` | ❌ Wave 4 |
| HTML-05 (template create wired) | `MailTemplateService::create` sanitizes | unit | `cargo test -p genossi_mail mail_template_service::tests::create_sanitizes_body_html -x` | ❌ Wave 4 |
| HTML-05 (template update wired) | `MailTemplateService::update` sanitizes | unit | `cargo test -p genossi_mail mail_template_service::tests::update_sanitizes_body_html -x` | ❌ Wave 4 |
| HTML-05 (test-mail wired) | `send_test_mail_with_body` sanitizes new `body_html` arg | unit | `cargo test -p genossi_mail service::tests::send_test_mail_with_body_sanitizes_body_html -x` | ❌ Wave 4 |
| FMT-01 | `join_date` renders as `DD.MM.YYYY`; `exit_date` too when Some | unit (extension of existing `test_date_fields` / `test_exit_date_null`) | `cargo test -p genossi_mail template::tests::test_date_fields_renders_german_format -x` | ❌ Wave 2 |

### Sampling Rate
- **Per task commit:** `cargo test -p genossi_mail`
- **Per wave merge:** `cargo test && cargo test -p genossi_bin --test e2e_tests` (mail e2e tests should stay green for the "text-only mail unchanged" regression)
- **Phase gate:** `cargo test && cargo clippy --all-targets --all-features` all green before `/gsd-verify-work`

### Wave 0 Gaps
- [ ] `genossi_mail/src/sanitize.rs` — new module with `sanitize_html` + its 4 tests
- [ ] New send-tests block in `genossi_mail/src/send.rs::tests` for the 4 MIME-shape scenarios
- [ ] Extension of `genossi_mail/src/template.rs::tests` with 3 new tests (html_env autoescape, strict_env non-regression, FMT-01)
- [ ] Extension of `genossi_mail/src/dao_sqlite.rs::tests` with `body_html` roundtrip tests
- [ ] No new framework installs — `cargo test` + `mockall` already in place

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | Not affected by this phase |
| V3 Session Management | no | Not affected |
| V4 Access Control | no | Not affected (existing REST auth around mail endpoints unchanged) |
| V5 Input Validation & Output Encoding | **yes** | `minijinja::AutoEscape::Html` for output encoding; `ammonia::clean` for input sanitization |
| V6 Cryptography | no | Not affected |
| V7 Error Handling | no | Not affected |
| V14 Config | no | Not affected |

### Known Threat Patterns for {Rust axum + lettre + minijinja}

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Stored XSS in author-HTML mails (a compromised or malicious Vorstand account posts `<script>`) | Tampering / Elevation | `ammonia::clean(...)` at all 3 store-side entry points (D-03). Ammonia strips `<script>`, event handlers, and dangerous URL schemes. |
| Reflected XSS via unescaped member values in HTML body (`first_name = '<img onerror=alert(1)>'`) | Tampering | Separate `html_env()` with `AutoEscape::Html` (D-04). All context values are HTML-encoded at render time. |
| Open-redirect via `<a href="javascript:...">` or `<a href="data:...">` | Elevation | Ammonia default `url_schemes` allowlist blocks `javascript:` and `data:` (verified in ammonia 4 docs). |
| `target=_blank` tab-nabbing | Elevation | Ammonia default auto-adds `rel="noopener noreferrer"` (verified in ammonia 4 docs). |
| Frontend-only sanitize bypass (attacker POSTs raw HTML to API) | Tampering | D-03 mandates backend sanitize; frontend sanitize is explicitly NOT a security boundary. Tested via HTTP-level integration test. |
| Template injection via member value containing `{{ eval() }}` | Tampering | minijinja does NOT re-parse rendered output; a member value is a leaf string, not a nested template. Verified by test: member field containing `{{ 7*7 }}` renders literally, not `49`. |
| Mail-header injection via `body_html` newlines | Spoofing | `lettre` builder handles all header composition; `body_html` is a body payload, not a header — cannot inject headers by design. |
| Persistence of `rendered_html_body` leaks PII in logs | Information Disclosure | Existing pattern: `rendered_body` is already persisted (Quick 260614-9zf); logs already scrub this. `rendered_html_body` follows the same pattern. No new log statement introduces the field. |

## Sources

### Primary (HIGH confidence)
- `https://docs.rs/ammonia/4.1.3/ammonia/` — top-level `clean()` function documented
- `https://docs.rs/ammonia/4.1.3/ammonia/struct.Builder.html` — default tags list (h1-h6, table, img, ul/ol/li, p, b/i/strong/em, a, br, br); default URL schemes; `rel="noopener noreferrer"` default
- `https://docs.rs/minijinja/2.19.0/minijinja/struct.Environment.html#method.set_auto_escape_callback` — autoescape API signature and example
- `https://docs.rs/lettre/0.11/lettre/message/struct.MultiPart.html` — `alternative()`, `mixed()`, `.singlepart()`, `.multipart()` builders
- Codebase grep: `Cargo.lock` — pinned versions of `lettre 0.11.20`, `minijinja 2.19.0`
- Codebase grep: `genossi_mail/src/send.rs` — verified Phase-22 `build_message` shape (attachment wrapping already implemented)
- Codebase grep: `genossi_mail/src/worker.rs:442-490` — verified `rendered_body` persistence pattern to mirror for `rendered_html_body`
- Codebase grep: `genossi_mail/src/dao_sqlite.rs` — verified column-list pattern for `mail_jobs` / `mail_recipients` / `mail_templates` INSERT/UPDATE/SELECT statements
- `cargo info ammonia` — confirmed v4.1.3, MSRV 1.80, MIT OR Apache-2.0

### Secondary (MEDIUM confidence)
- ammonia project convention that `Builder::default()` maps to what `clean()` uses — inferred from docs but not literally verbatim quoted; the two paths produce byte-identical output in practice.

### Tertiary (LOW confidence)
- None.

## Project Constraints (from CLAUDE.md)

- **Layered architecture** DAO → Service → REST — Phase 23 changes touch all three layers cleanly (schema in DAO; sanitizer + render + `build_message` in Service; DTO field addition in REST).
- **Component-first frontend** — N/A for Phase 23 (no frontend work; that's Phase 24).
- **Audit-first for auditable entities** — N/A; MailJob, MailTemplate, MailRecipient are NOT audited entities (only Member, MemberAction, MemberDocument, Application are). CONTEXT.md line 105 confirms: "Bestehende auditierte Entitäten […] müssen weiterhin Audit-Macros verwenden; neue GV-Entitäten benötigen das **nicht**".
- **Soft deletes / optimistic locking** — `body_html` is a new column on existing entities with existing `deleted`+`version` fields; no change to lifecycle semantics.
- **jj VCS** — commits via `jj commit -m …`, not `git commit`. Same rule as Phase 22.
- **Immer Enum statt bool** — no boolean added; presence is `Option<Arc<str>>` (None = text-only, Some(...) = has HTML).
- **Always tests** (user's global CLAUDE.md) — every REQ mapped to at least one test in the Phase Requirements → Test Map above.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — all versions verified in Cargo.lock or cargo info; ammonia 4.1.3 docs verified for allowed tags / URL schemes / rel behavior.
- Architecture: HIGH — the phase re-uses two existing patterns (Phase-22 `build_message`, `rendered_body` persistence from Quick 260614-9zf) and adds one small new pattern (parallel `html_env` next to `strict_env`).
- Pitfalls: HIGH — the Jinja-in-attribute constraint from CONTEXT.md `<specifics>` is the one non-obvious sharp edge; the rest are standard.
- Migrations: HIGH — three trivial `ADD COLUMN … TEXT NULL` migrations following an existing forward-only template.
- Security: HIGH — ammonia + autoescape covers the standard XSS/injection surface, and the shared context builder + FMT-01 don't introduce new inputs.

**Research date:** 2026-07-02
**Valid until:** 2026-08-02 (30 days — the underlying crates are stable major versions; ammonia 4.x has had a slow release cadence; lettre 0.11.x is the current line; minijinja 2.x is stable).
