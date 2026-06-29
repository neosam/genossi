# Architecture Research

**Domain:** Mail formatting (8bit + HTML multipart) & Application file-attachment carryover — integration into the existing layered Rust/Axum/SQLx/Dioxus codebase (Genossi v1.4)
**Researched:** 2026-06-29
**Confidence:** HIGH (integration points are codebase-verified; lettre API verified against docs.rs)

> This is an **integration** architecture document for a subsequent milestone. The DAO→Service→REST→Frontend layering, audit-macro discipline, and Component-First frontend rule are already established and are **not** re-researched here. The focus is exactly four features and where each one touches existing code.

---

## Standard Architecture

### System Overview — what the four features touch

```
┌──────────────────────────────────────────────────────────────────────┐
│  FRONTEND (Dioxus WASM)                                                │
│  ┌────────────────────────┐        ┌──────────────────────────────┐   │
│  │ component/mail_compose/ │        │ page/applications_page.rs    │   │
│  │  + NEW wysiwyg_editor   │        │  + NEW application file-upload│  │
│  │    (produces HTML)      │        │    + attachment list/download │  │
│  └───────────┬────────────┘        └───────────────┬──────────────┘   │
├──────────────┼──────────────────────────────────────┼─────────────────┤
│  REST (Axum) │                                       │                 │
│  genossi_mail/src/rest.rs               genossi_rest/src/application.rs │
│   send-bulk / template (+ body_html)     + NEW POST /{id}/documents     │
│                                          (multipart, mirrors            │
│                                           member_document.rs:115)       │
├──────────────┼──────────────────────────────────────┼─────────────────┤
│  SERVICE     │                                       │                 │
│  genossi_mail: service.rs / worker.rs / digest.rs    genossi_service_  │
│   render.rs (dual render) + NEW mail_body.rs helper  impl/application.rs│
│   + NEW sanitize step (ammonia, server-side)          confirm() cascade│
│                                                       → audited MemberDoc│
├──────────────┼──────────────────────────────────────┼─────────────────┤
│  DAO (SQLx / SQLite)                                  │                 │
│  mail_templates(+body_html)  mail_jobs(+body_html)    + NEW             │
│  mail_recipients(+rendered_body_html?)                application_      │
│                                                        documents table   │
├──────────────────────────────────────────────────────┼─────────────────┤
│  FILESYSTEM  DocumentStorage (save/load/delete by relative_path)        │
│   member docs: "<uuid>.<ext>"   static: "static_documents/<uuid>"       │
│   + NEW application docs: "application_documents/<uuid>.<ext>"          │
└──────────────────────────────────────────────────────────────────────┘
```

### Component Responsibilities (new vs modified)

| Component | New / Modified | Responsibility | File |
|-----------|----------------|----------------|------|
| `mail_body` helper | **NEW** | Pure, sync MIME-body builder: 8bit text part, optional multipart/alternative (text+html), fold in attachments. Single source of truth for all send paths. | `genossi_mail/src/mail_body.rs` |
| `send_mail_for_recipient` | Modified | Call `mail_body` helper instead of inline `SinglePart::plain` + `MultiPart::mixed`. | `genossi_mail/src/worker.rs:627` |
| `send_test_mail` / `send_test_mail_with_body` | Modified | Replace `.body(...)` (currently no charset, no SinglePart) with the `mail_body` helper → fixes existing charset bug + adds 8bit. | `genossi_mail/src/service.rs:415,447` |
| `resolve_rendered_content` | Modified | Return `(subject, text_body, Option<html_body>)`; render the HTML template with the same context via a new autoescaping render fn. | `genossi_mail/src/render.rs:48` |
| `render_html_template` | **NEW** | minijinja render with HTML autoescape ON (variables escaped, template tags literal). | `genossi_mail/src/template.rs` |
| HTML sanitizer | **NEW** | Server-side allow-list sanitization of WYSIWYG HTML (ammonia). Applied at `create_job` and template create/update. | `genossi_mail/src/sanitize.rs` (or `service.rs`) |
| `wysiwyg_editor` | **NEW** | Reusable Dioxus component emitting HTML; feeds existing compose flow. | `genossi-frontend/src/component/` |
| `ApplicationDocument` entity + DAO | **NEW** | 0..n files attached to an Application; mirrors `MemberDocument` shape; **not** audited. | `genossi_dao/src/application_document.rs` (+ sqlite impl) |
| Application upload/list/download endpoints | **NEW** | multipart upload → DocumentStorage; mirrors `member_document.rs`. | `genossi_rest/src/application.rs` |
| `confirm()` carryover cascade | Modified | On activation, copy each ApplicationDocument file to a member-doc path and create a **`audited_create!`** MemberDocument in the same tx. | `genossi_service_impl/src/application.rs:280` |

---

## Architectural Patterns (the decisions to make)

### (a) Shared "build mail body" helper — location & signature

**Problem today:** Three send paths build the lettre body three different ways and are inconsistent:
- `worker.rs::send_mail_for_recipient` (line 627): builds `SinglePart::plain(body)`; if attachments, `MultiPart::mixed().singlepart(text).singlepart(attachment…)`. Correct charset, but auto transfer-encoding = quoted-printable (the `=` soft-breaks v1.4 wants gone).
- `service.rs::send_test_mail` (415) and `send_test_mail_with_body` (447): use `Message::builder().body(String)` — **no `SinglePart::plain`, so no `charset=utf-8`** (the exact GMX-umlaut bug the worker tests guard against). The digest worker calls `send_test_mail_with_body`, so it inherits this.

**Decision — place a pure, sync helper in a new module `genossi_mail/src/mail_body.rs`.** It is the single source of truth, unit-testable without SMTP, and consumed by all three paths. Keep file I/O (DocumentStorage `.load`) in the async caller; pass already-loaded attachment parts in.

**Proposed signature:**

```rust
use lettre::message::{Message, MessageBuilder, MultiPart, SinglePart};
use lettre::message::header::{ContentType, ContentTransferEncoding};

/// One assembled MIME body, ready to attach to a MessageBuilder.
pub enum MailContent {
    Single(SinglePart),   // plain text only, no attachments
    Multi(MultiPart),     // alternative (text+html) and/or mixed (attachments)
}

/// Build the body. `text` is always required (the plain-text fallback part).
/// `html` Some → multipart/alternative. `attachments` non-empty → wrap in mixed.
/// Both text and html parts are emitted with charset=utf-8 and 8bit encoding.
pub fn build_mail_content(
    text: &str,
    html: Option<&str>,
    attachments: Vec<SinglePart>,   // pre-built via lettre Attachment::new(..).body(bytes, ct)
) -> MailContent;

impl MailContent {
    /// Fold the content into the builder, choosing singlepart vs multipart.
    pub fn into_message(self, builder: MessageBuilder)
        -> Result<Message, lettre::error::Error>;
}
```

**8bit forcing** (verified on docs.rs — `ContentTransferEncoding::EightBit` exists, but lettre auto-selects quoted-printable unless you set it manually):

```rust
fn text_part(s: &str) -> SinglePart {
    SinglePart::builder()
        .header(ContentType::parse("text/plain; charset=utf-8").unwrap())
        .header(ContentTransferEncoding::EightBit)
        .body(s.to_string())
}
// html part identical but ContentType::parse("text/html; charset=utf-8")
// alternative: MultiPart::alternative().singlepart(text_part).singlepart(html_part)
```

Note: lettre also offers `MultiPart::alternative_plain_html(plain, html)`, but it does **not** let you set 8bit, so build the two `SinglePart`s explicitly and combine with `MultiPart::alternative()`.

**Trade-off / pitfall:** 8bit requires the SMTP relay to advertise `8BITMIME`. Most do; if a configured relay does not, lettre will still send 8bit bytes and the server may reject or silently mangle. **Flag as a deploy-time verification item** (test against the production relay early in the HTML-mail phase). This is the one genuine risk in feature (a).

### (b) minijinja dual-render — render two templates, do NOT derive text from HTML

**Decision: render an authored text template AND an authored HTML template against the *same* context. Do not derive plain text from rendered HTML.**

Rationale:
- The existing `body` field is already a clean, authored, jinja-variable text template (`render.rs` + `template.rs` are built around it, strict-env, repayment-var merge). It is the guaranteed `text/plain` fallback. Keep it.
- HTML→text derivation (e.g. `html2text`) is lossy, adds a dependency, and produces worse output than the already-authored text. There is no upside.
- The same `minijinja::Value` context (`member_to_template_context` + `merge_repayment_context`) feeds both renders with zero new context plumbing.

**Change to `resolve_rendered_content`** (`render.rs:48`): return `(String, String, Option<String>)` = (subject, text_body, html_body?). Render the HTML template only when the job carries an HTML template (`job.body_html`). Worker passes the `Option<String>` straight into `build_mail_content`.

**Autoescape:** `render_template` (template.rs:67) uses a non-autoescaping env — correct for plain text. Add `render_html_template` using an env with HTML autoescape ON so member values (`{{ first_name }}`) are HTML-escaped while the template's own markup stays literal. Repayment vars (`payout_amount` etc.) are also escaped harmlessly.

**Plain-text fallback source:** the existing `body` column / `job.body`. The WYSIWYG flow should *seed* `body` with a tag-stripped text version of the HTML at compose time so the Vorstand authors once, but `body` remains the authoritative text part (round-trippable, editable, audit-clear). This keeps both renders symmetric and strict-env-validatable via the existing `validate_template` / `validate_template_with_repayment` helpers (extend them to also probe the HTML template).

### (c) Data model & migrations (forward-only SQLite)

**Mail HTML — add nullable columns, mirroring the existing `20260601000000_extend_mail_job_template_phase.sql` pattern (ALTER TABLE … ADD COLUMN … NULL, no down-migration).**

```sql
-- NEW migration e.g. 20260630000000_mail_html_body.sql
ALTER TABLE mail_templates ADD COLUMN body_html TEXT NULL;   -- authored HTML template
ALTER TABLE mail_jobs      ADD COLUMN body_html TEXT NULL;   -- snapshot at send-time, mirrors `body`
-- optional, for "what did this recipient receive" parity with rendered_subject/body:
ALTER TABLE mail_recipients ADD COLUMN rendered_body_html TEXT NULL;
```

Why columns, not "derive at send-time from a stored format": the codebase **already snapshots** subject+body into `mail_jobs` at `create_job` and per-recipient rendered output into `mail_recipients`. HTML must follow the same snapshot discipline (a template edit after a job is queued must not change what goes out). So `body_html` is a stored authored template on `mail_templates`, snapshotted onto `mail_jobs` exactly like `body`. Legacy rows = NULL → single-part text mail (fully backward compatible: digest, confirmation mail, old jobs all keep NULL and render as today).

**Application attachment — NEW table (not a single column).** An applicant may submit more than one document, and the codebase's consistent pattern is an entity table with soft-delete + version (mirrors `MemberDocument`). A `member_documents`-style table is the lowest-surprise choice.

```sql
-- NEW migration e.g. 20260630000100_create_application_documents_table.sql
CREATE TABLE application_documents (
    id             BLOB PRIMARY KEY,
    application_id BLOB NOT NULL,
    document_type  TEXT NOT NULL,      -- e.g. "other"/"join_declaration"
    description    TEXT NULL,
    file_name      TEXT NOT NULL,
    mime_type      TEXT NOT NULL,
    relative_path  TEXT NOT NULL,      -- "application_documents/<uuid>.<ext>"
    created        TEXT NOT NULL,
    deleted        TEXT NULL,
    version        BLOB NOT NULL
);
CREATE INDEX idx_application_documents_application_id ON application_documents(application_id);
```

**Audit:** `ApplicationDocument` is a NEW entity → per the milestone constraint it does **not** need the `Auditable` trait / audit macros (same exemption as the GV entities). The **carryover MemberDocument is audited** (MemberDocument is an audited entity), and the `confirm()` Application status flip already uses `audited_update!`.

### (d) Application attachment — endpoints + activation cascade

**Upload endpoint — mirror the proven member-document upload.** The exact pattern to copy is `genossi_rest/src/member_document.rs:115` (`upload_document`): `Multipart` extractor, `while multipart.next_field()`, extension whitelist via `lookup_allowed_mime` / `allowed_extensions`, `DefaultBodyLimit::max(50 MB)` layer (member_document.rs:41,49), service creates the entity, **then the REST handler calls `document_storage().save(&relative_path, &data)`** (member_document.rs:209). Add to `genossi_rest/src/application.rs`:

```
POST   /api/applications/{id}/documents          (multipart upload)
GET    /api/applications/{id}/documents          (list)
GET    /api/applications/{id}/documents/{doc_id} (download — mirror member_document.rs:242)
DELETE /api/applications/{id}/documents/{doc_id} (soft-delete)
```

**Route-ordering pitfall:** the existing router (`application.rs:479`) has `/{id}` (GET/PUT), `/{id}/confirm`, `/{id}/reject`. The new `/{id}/documents` and `/{id}/documents/{doc_id}` are more-specific and safe, but follow the v1.2 lesson (PROJECT.md Key Decisions: "all sub-routes BEFORE `/{id}` catch-all") — register them and add E2E asserts that `/{id}/documents` is not parsed as a UUID.

**Activation cascade — the audit-critical piece.** Today `ApplicationServiceImpl::confirm` (application.rs:280) opens one tx, creates Member + two MemberActions + flips Application status, all via `audited_create!`/`audited_update!` sharing `APPLICATION_SERVICE_PROCESS` + `user_id`. To carry files over, extend `ApplicationServiceDeps` with **`MemberDocumentDao`** and **`DocumentStorage`** (DocumentStorage is currently only wired into `MemberDocumentService` / mail; it must be added to the application service's deps and `RestStateImpl::new()` in `genossi_bin/src/lib.rs`).

Sequence inside `confirm`, after the member is created:

```
for app_doc in application_document_dao.find_by_application_id(id, tx):
    1. bytes = document_storage.load(&app_doc.relative_path)          // before write
    2. new_path = "<new_uuid>.<ext>"                                  // member-doc namespace
    3. document_storage.save(&new_path, &bytes)                       // copy file (non-transactional)
    4. build MemberDocumentEntity { member_id, document_type, relative_path: new_path, .. }
    5. audited_create!(self, self.member_document_dao, &entity,
                       APPLICATION_SERVICE_PROCESS, &user_id, tx)      // same tx, same process
```

**Why copy the file (load+save) rather than reuse the ApplicationDocument's path:** the MemberDocument must own its file independently — a later soft-delete/cleanup of the application or its document must not orphan the member's document. Copy **before** `audited_create!` so a storage failure leaves at most a harmless orphan file (GC-able), never a DB row pointing at a missing file. This matches the existing upload flow's failure class (DB row then storage save).

**DocumentType for the carried-over doc:** recommend `DocumentType::Other` with `description = "Originaler Mitgliedsantrag"`. Reason: `JoinDeclaration` is a **singleton** type (`is_singleton()` true, member_document.rs:114) and is also the type of the *generated* `join_declaration.typ` PDF — a carryover under `JoinDeclaration` would collide with the singleton guard if a declaration was generated. `Other` avoids the collision and reads clearly in the audit log. (Alternative: `JoinDeclaration` if product wants it to occupy that slot — but then handle the singleton conflict.)

---

## Data Flow

### HTML mass-mail (compose → send)

```
Vorstand types in WYSIWYG (HTML) + text seed
        ↓  POST /api/mail/send-bulk { subject, body(text), body_html, recipients, ... }
REST rest.rs → MailService::create_job (sanitize body_html server-side via ammonia)
        ↓  persist mail_jobs.{subject, body, body_html} + mail_recipients(pending)
worker.rs loop → resolve_rendered_content(recipient, job)  [render.rs]
        ↓  (subject, text_body, Option<html_body>)  — same ctx, dual render
build_mail_content(text, html, attachments)  [mail_body.rs]
        ↓  multipart/alternative (8bit text + 8bit html) [+ mixed if attachments]
lettre transport.send → SMTP (relay must support 8BITMIME)
        ↓  audited MemberDocument anchor (existing Phase-10 path, unchanged)
```

### Application attachment carryover

```
Vorstand uploads file on Application detail
        ↓  POST /api/applications/{id}/documents (multipart)
REST application.rs → ApplicationDocumentService.create + document_storage.save
        ↓  row in application_documents (NOT audited)
... later: Vorstand confirms application ...
        ↓  POST /api/applications/{id}/confirm
ApplicationServiceImpl::confirm (one tx)
   create Member (audited) → MemberActions (audited)
   → for each app_doc: copy file + audited_create! MemberDocument  ← NEW
   → audited_update! Application status=Bestaetigt
        ↓  commit
```

---

## Recommended Build Order (phases)

Ordering respects DAO→Service→REST→Frontend and isolates the audit-bearing work. Features (a)+(b)+(c) are sequential (each builds on the shared helper / schema); feature (d) is independent and can run in parallel.

| Phase | Scope | Depends on | Layer span | Audit? |
|-------|-------|-----------|------------|--------|
| **P1 — 8bit + shared mail-body helper** | NEW `mail_body.rs`; refactor `worker.rs::send_mail_for_recipient`, `service.rs::send_test_mail(_with_body)` to use it; force 8bit. Fixes existing test/digest charset bug as a bonus. | — | Service (mail) only, no schema | No |
| **P2 — HTML mail backend** | Migration (`body_html` on mail_templates+mail_jobs, optional `rendered_body_html`); DAO field plumbing; `create_job`/template signatures; `render_html_template` + dual `resolve_rendered_content`; worker emits multipart/alternative; REST send-bulk/template carry `body_html`; **server-side sanitize (ammonia)**; extend `validate_template*` to probe HTML. | P1 | DAO→Service→REST | No |
| **P3 — WYSIWYG editor (frontend)** | Reusable Dioxus `wysiwyg_editor` component → HTML into compose flow; preview; seed plain-text `body` from HTML. Component-First (no inline RSX). | P2 (needs `body_html` wire) | Frontend | No |
| **P4 — Application attachment + carryover** | Migration (`application_documents`); NEW DAO + sqlite impl; `ApplicationDocumentService` (upload/list/download/delete); add `MemberDocumentDao`+`DocumentStorage` to `ApplicationServiceDeps` and wire in `genossi_bin/src/lib.rs`; **`confirm()` cascade with `audited_create!` MemberDocument**; REST endpoints (mirror member_document.rs); frontend upload UI on applications_page. | independent (can parallel P1–P3) | DAO→Service→REST→Frontend | **Yes** (carryover MemberDocument) |

**Why this order:** P1 is a low-risk pure refactor that establishes the one body-builder all later mail work reuses; doing it first prevents three divergent HTML implementations. P2 cannot precede P1 (it needs the helper). P3 needs P2's `body_html` field to exist. P4 shares nothing with the mail features and touches the audit-critical confirm cascade, so it is best isolated as its own slice with its own UAT.

---

## Anti-Patterns (specific to this integration)

### Building HTML/8bit inline in each send path
**Don't** add HTML/8bit logic separately in `worker.rs`, `service.rs`, and the digest. They will drift (they already have — the `.body()` charset bug). **Do** route every send through `mail_body.rs::build_mail_content`.

### Deriving plain text from rendered HTML
**Don't** drop the authored text template and `html2text` the HTML. **Do** render both authored templates against the same context; `body` stays the authoritative text fallback.

### Trusting the WYSIWYG HTML from the client
**Don't** persist/send the editor's HTML unsanitized, and don't rely on WASM-side sanitization as the security boundary. **Do** sanitize server-side with an allow-list (ammonia, native-only) at `create_job` and template create/update — the email is sent to members, so the boundary is the backend.

### Interpolating member values into HTML without autoescape
**Don't** reuse the non-autoescaping `render_template` for the HTML part. **Do** use an HTML-autoescaping env so `{{ first_name }}` etc. are escaped while template markup stays literal.

### Skipping audit macros on the carryover MemberDocument
**Don't** call `member_document_dao.create()` directly in `confirm()`. **Do** use `audited_create!` with the shared `APPLICATION_SERVICE_PROCESS` + `user_id` + the confirm tx — MemberDocument is an audited entity and the cascade must stay forensically consistent.

### DB-write-before-file-copy in the cascade
**Don't** `audited_create!` the MemberDocument and then copy the file — a copy failure leaves a DB row pointing at a missing file. **Do** copy (load+save) first, then `audited_create!` in the tx.

---

## Integration Points

### External / infrastructure

| Service | Integration | Notes / gotchas |
|---------|-------------|-----------------|
| SMTP relay (lettre 0.11) | `MultiPart::alternative()` of two 8bit `SinglePart`s; `ContentTransferEncoding::EightBit`, `ContentType … charset=utf-8` set manually | **Verify relay advertises 8BITMIME** before shipping P2 — 8bit on a 7bit-only relay risks rejection/mangling. `MultiPart::alternative_plain_html` exists but can't set 8bit. |
| DocumentStorage (filesystem) | `save/load/delete(relative_path)` — non-transactional | Carryover must copy-before-commit; reuse `application_documents/<uuid>.<ext>` namespace convention. |
| ammonia (NEW dep) | server-side HTML sanitization | Native-only (html5ever) — add to `genossi_mail`/server crate, **never** the WASM frontend. |

### Internal boundaries

| Boundary | Communication | Notes |
|----------|---------------|-------|
| worker / service / digest → `mail_body` | direct sync call | New single source of truth; pure + unit-testable without SMTP. |
| `render.rs` → `template.rs` | `render_template` + NEW `render_html_template` | Same context object; HTML render adds autoescape only. |
| `ApplicationService` → `MemberDocumentDao` + `DocumentStorage` | NEW deps via `gen_service_impl!` | Must be wired in `genossi_bin/src/lib.rs RestStateImpl::new()`. |
| REST application docs → service | multipart, mirrors `member_document.rs:115` | Reuse `lookup_allowed_mime`/`allowed_extensions`/`DefaultBodyLimit`. |

---

## Sources

- Codebase (HIGH): `genossi_mail/src/{worker.rs:627,service.rs:415/447,render.rs:48,template.rs:67,digest.rs}`, `genossi_rest/src/{application.rs:479,member_document.rs:115/209/242}`, `genossi_service_impl/src/{application.rs:280,member_document.rs:53}`, `genossi_service/src/member_document.rs:49` (DocumentType), `genossi_dao/src/auditable.rs`, migration `20260601000000_extend_mail_job_template_phase.sql`.
- lettre 0.11 docs (MEDIUM-HIGH, verified): `MultiPart::alternative_plain_html` and `ContentTransferEncoding::EightBit` confirmed on docs.rs (`/lettre/0.11/lettre/message/`).
- Project context (HIGH): `.planning/PROJECT.md` (v1.4 goal, audit constraints, route-ordering lesson), `CLAUDE.md` (audit system, Component-First, layering).

---
*Architecture research for: Genossi v1.4 — Mail-Formatierung & Antrags-Dokumente*
*Researched: 2026-06-29*
