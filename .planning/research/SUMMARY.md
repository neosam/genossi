# Project Research Summary

**Project:** Genossi v1.4 — Mail-Formatierung & Antrags-Dokumente
**Domain:** HTML mail formatting (8bit + multipart/alternative), WYSIWYG editor (Dioxus/WASM), HTML sanitization, Application file upload with audited carryover
**Researched:** 2026-06-29
**Confidence:** HIGH

## Executive Summary

Genossi v1.4 adds HTML mail composition and application file storage to an existing Rust/Axum/SQLite + Dioxus-WASM cooperative management system. The research is grounded entirely in the existing codebase — almost everything needed is already present: `lettre 0.11` for MIME multipart, `axum` multipart feature, `web-sys`/`js-sys`, a fully componentized mail-compose UI, and a proven `DocumentStorage` + upload pattern in `member_document.rs`. The net new production dependency is exactly one crate: `ammonia 4.1` for server-side HTML sanitization.

The recommended approach is a four-phase sequential build for mail features (P1 -> P2 -> P3) with the application-document track (P4) running independently and parallelizable. P1 extracts a shared `mail_body.rs` helper that forces 8bit encoding across all three current send paths, eliminating quoted-printable `=` soft-breaks while fixing an existing charset bug in `service.rs`. P2 wires HTML mail backend: new nullable `body_html` columns, a separate HTML-autoescaping minijinja environment, server-side ammonia sanitization, and `multipart/alternative` emission. P3 replaces the existing `body_editor.rs` textarea with a `contenteditable`+`execCommand` component using the already-present `web-sys`. P4 adds a `MemberDocument`-mirrored `application_documents` entity, upload endpoint, and an audited carryover cascade inside the existing `confirm()` transaction.

The three open questions requiring explicit decisions before implementation: (a) whether the production SMTP relay advertises `8BITMIME` (determines whether 8bit can be the default or must be config-opt-in with quoted-printable fallback); (b) the plain-text fallback strategy (research recommends keeping the existing authored `body` as the text part, with HTML additive, avoiding `html2text`); and (c) the `DocumentType` for the carried-over application file (recommend `Other` with description "Originaler Mitgliedsantrag" to avoid collision with the `JoinDeclaration` singleton guard).

## Key Findings

### Recommended Stack

No new framework and no JS libraries are required. All MIME work uses `lettre 0.11` (already present) via explicit `SinglePart::builder().header(ContentTransferEncoding::EightBit)` construction — the convenience constructor `MultiPart::alternative_plain_html()` MUST NOT be used because it silently reverts to quoted-printable. The WYSIWYG editor uses `web-sys`/`js-sys` (already present) via `document.execCommand` and requires zero new frontend dependencies. File upload reuses the `axum` multipart feature (already active) and `DocumentStorage` trait.

**Core technologies:**
- `lettre 0.11` (existing): 8bit encoding + `multipart/alternative` — controlled via explicit `SinglePart::builder()`, NOT `MultiPart::alternative_plain_html()`
- `ammonia 4.1` (NEW, backend only): server-side HTML sanitization — whitelist-based, default config matches the editor's tag set; NEVER compiled to WASM; requires `rustc >= 1.80` (verify Nix toolchain)
- `web-sys`/`js-sys` (existing, frontend): `contenteditable` + `execCommand` WYSIWYG editor — `styleWithCSS` must be disabled before editing so browser emits semantic tags (`<b>`,`<i>`) not `<span style>` (which ammonia would strip)
- `axum` multipart (existing): application file upload — identical pattern to `member_document.rs:115`
- `minijinja 2.x` (existing): dual-render with separate environments — existing `strict_env()` for text/subject (unchanged), new HTML-autoescape env for HTML body only

### Expected Features

**Must have (table stakes):**
- 8bit Content-Transfer-Encoding on all send paths (shared `mail_body.rs` helper) — direct user complaint
- `multipart/alternative` with correct MIME nesting: `mixed(alternative(plain, html), attachments...)`
- Separate minijinja HTML-autoescape environment — member variables escaped, template markup literal
- Server-side ammonia sanitization at write (template/job create+update), ammonia gate mandatory before WYSIWYG phase ships
- WYSIWYG editor: bold, italic, bullet list, numbered list, link insert — replaces `body_editor.rs` textarea; Component-First (one reusable component shared by compose + reply flows)
- Real plain-text fallback always present in multipart (spam/accessibility/audit requirement) — existing authored `body` is the text part
- Application single-file upload (PDF) stored via `DocumentStorage` in `application_documents` table (NOT DB BLOB)
- Audited carryover of application file to `MemberDocument` on `confirm()` — same transaction as existing Member/MemberAction/Application cascade; uses `audited_create!`
- Attachment view/download on Application detail page
- CR-02 permission-ordering fix in all new upload/carryover methods (`check_permission()` before `current_user_id()`)

**Should have (competitive):**
- Paste-from-Word cleanup (strip `mso-`/`style`/`class`/`font` on paste) — high real-world value for non-technical board
- Saved HTML templates (extend existing template store to hold `body_html`; autoescape work is already done)

**Defer (v2+):**
- Branding/letterhead + inline CID images (`multipart/related` — significant MIME complexity)
- Multiple files per Application (design table to allow it, ship single-file first)
- HTML inbox digest (trivial once `multipart/alternative` helper exists)
- Drag-and-drop email builder (explicit anti-feature for this product)

### Architecture Approach

The milestone is an integration exercise within the existing DAO -> Service -> REST -> Frontend layer pattern. All new components follow established patterns: `mail_body.rs` is a pure sync module; `application_documents` entity mirrors `MemberDocument` shape exactly; the upload endpoint mirrors `member_document.rs:115`; and the activation carryover extends `ApplicationServiceImpl::confirm()` using `audited_create!` inside the existing single transaction.

**Major new/modified components:**
1. `genossi_mail/src/mail_body.rs` (NEW) — pure MIME body builder; single source of truth for all three send paths; `build_mail_content(text, html: Option, attachments) -> MailContent`
2. `genossi_mail/src/render.rs` + `template.rs` (modified) — `resolve_rendered_content` returns `(subject, text_body, Option<html_body>)`; new `render_html_template` with HTML-autoescape env
3. `genossi_mail/src/sanitize.rs` (NEW) — ammonia wrapper at create/update; URL scheme allowlist (http/https/mailto); strips `style`/`class`/event handlers
4. `genossi-frontend/src/component/wysiwyg_editor.rs` (NEW) — `contenteditable` + `execCommand` via `web-sys`; `styleWithCSS` disabled on mount; read-on-blur not two-way-bound; toolbar buttons have `r#type: "button"`
5. `genossi_dao/src/application_document.rs` + SQLite impl (NEW) — mirrors MemberDocument DAO; NOT audited (per milestone constraint; the carryover MemberDocument IS audited)
6. `genossi_rest/src/application.rs` (modified) — adds `POST/GET/GET/{doc_id}/DELETE /{id}/documents`; sub-routes registered BEFORE `/{id}` catch-all (v1.2 route-ordering lesson)
7. `genossi_service_impl/src/application.rs::confirm()` (modified) — file copy (FS-before-DB) then `audited_create!(MemberDocument)` per ApplicationDocument; `ApplicationServiceDeps` gains `MemberDocumentDao` + `DocumentStorage`; wired in `genossi_bin/src/lib.rs`

**Two SQLite migrations needed (forward-only, no down migrations):**
- `20260630000000_mail_html_body.sql`: nullable `body_html` on `mail_templates` + `mail_jobs` (NULL = plain-text send, fully backward-compatible)
- `20260630000100_create_application_documents_table.sql`: new `application_documents` table with index on `application_id`

### Critical Pitfalls

1. **minijinja autoescape OFF for nameless templates** — `strict_env()` uses `template_from_str` (no file name), so autoescape is never triggered. Reusing it for HTML bodies silently injects raw member names into HTML. Prevention: separate `render_html_template` with explicit HTML autoescape callback. Unit test required: `last_name = "<script>alert(1)</script> & Co"` must appear as `&lt;script&gt;` in output.

2. **8bit sent to non-8BITMIME relay** — `lettre` does NOT negotiate or downgrade 8bit; it sends whatever is set at build time. On a 7bit-only relay, high bytes are stripped or rejected. Prevention: 8bit must be config-opt-in with quoted-printable default until relay is verified.

3. **No server-side HTML sanitization (stored XSS)** — REST endpoint accepts arbitrary `body` strings independently of the UI; the admin app accesses all member PII/IBANs/audit log. Prevention: ammonia gate mandatory at service/REST write boundary. Frontend-only sanitization is never sufficient. ammonia and minijinja autoescape are complementary, not redundant.

4. **Broken `multipart/alternative` nesting with attachments** — bolting HTML onto the existing `MultiPart::mixed().singlepart(text)` attachment loop produces a wrong tree. Prevention: `mail_body.rs` always emits `alternative(plain, html)` nested inside `mixed` with attachments; plain part first in the alternative.

5. **File-before-DB ordering violated in carryover** — `FilesystemDocumentStorage::save` is not transactional with SQLite. `audited_create!(MemberDocument)` before the file copy can produce a DB row pointing at a missing file. Prevention: copy file first (FS), then `audited_create!` in tx; on tx rollback, best-effort delete the written file.

6. **`styleWithCSS` not disabled -> ammonia strips formatting** — browsers emit `<span style="font-weight:bold">` by default; ammonia strips `style` attributes. Prevention: `document.exec_command("styleWithCSS", false, "false")` on editor mount so browser emits `<b>`/`<i>` that ammonia's whitelist preserves.

7. **`contenteditable` <-> Dioxus signal desync** — two-way binding per `oninput` triggers Dioxus re-render resetting caret to position 0. Prevention: read `innerHTML` via `onmounted` ref on blur/before-submit only; toolbar buttons use `r#type: "button"` + onclick (project button-reload-bug pattern).

8. **CR-02 permission ordering repeated in new methods** — existing `confirm()` calls `current_user_id()` before `check_permission()`. New upload/carryover code must call `check_permission()` first via `gen_auth_admin!` pattern.

## Implications for Roadmap

Based on research, suggested phase structure:

### Phase 1: 8bit + Shared Mail-Body Helper
**Rationale:** Smallest, most self-contained slice; fixes the direct user complaint (`=` soft-breaks) and an existing charset bug in `service.rs::send_test_mail` (no `charset=utf-8`). Establishes the single MIME-body builder that all later mail phases reuse — without it, P2/P3 risk three divergent HTML implementations. Zero schema changes.
**Delivers:** `genossi_mail/src/mail_body.rs` with `build_mail_content(text, html, attachments)`; refactored `worker.rs::send_mail_for_recipient`, `service.rs::send_test_mail`, `service.rs::send_test_mail_with_body`; 8bit encoding config flag (default: quoted-printable, opt-in: 8bit); updated encoding-assertion tests.
**Avoids:** Pitfall 2 (8BITMIME opt-in), Pitfall 4 (correct nesting via helper), backward-compat regression (QP default unchanged).

### Phase 2: HTML Mail Backend
**Rationale:** Keystone for three sub-features (HTML part, template HTML rendering, eventual HTML digest). Must land before the frontend editor (P3) has a `body_html` wire to post to. The ammonia sanitization gate MUST land here — before any HTML is accepted and stored.
**Delivers:** SQLite migration (nullable `body_html` on `mail_templates`, `mail_jobs`); `render_html_template` with HTML-autoescape env; `genossi_mail/src/sanitize.rs` (ammonia gate at write); `worker.rs` emitting `multipart/alternative` via P1 helper; REST endpoints accepting `body_html`; extended `validate_template*` probing HTML template; `List-Unsubscribe` header on bulk jobs.
**Uses:** `ammonia 4.1` (only new production dependency added to this milestone); `lettre 0.11` `MultiPart::alternative()` + explicit 8bit `SinglePart::builder()`.
**Implements:** Dual-render architecture (same context object, two minijinja envs, two template columns).
**Avoids:** Pitfalls 1 (autoescape), 3 (MIME nesting), 4 (text fallback always present), 5 (server-side sanitization in place before WYSIWYG ships), 11 (List-Unsubscribe).

### Phase 3: WYSIWYG Frontend Editor
**Rationale:** Depends on P2's `body_html` field existing. Replaces a single existing component (`body_editor.rs` textarea) — not greenfield. Component-First mandate: one reusable `wysiwyg_editor` component shared by compose and reply flows.
**Delivers:** `genossi-frontend/src/component/wysiwyg_editor.rs` (contenteditable + execCommand via web-sys; `styleWithCSS` disabled on mount; read-on-blur; toolbar: bold/italic/bullet/numbered/link); updated `body_editor.rs` to use the new component; paste cleanup (strip `mso-`/`style`/`class`); seeding of plain-text `body` signal from HTML at compose time.
**Uses:** `web-sys`/`js-sys` (existing); zero new frontend dependencies.
**Avoids:** Pitfalls 6 (`styleWithCSS` + ammonia compatibility), 7 (`dangerous_inner_html` only for server-sanitized data), 10 (no per-keystroke signal write; r#type:button on toolbar).
**Fallback:** If `contenteditable` proves too inconsistent in UAT, switch to Markdown-toolbar + `pulldown-cmark` (backend); ammonia gate remains unchanged; no new frontend deps.

### Phase 4: Application Upload + Audited Carryover
**Rationale:** Fully independent of mail features — can run in parallel with P1-P3. Contains the audit-critical `confirm()` cascade extension and the CR-02 permission-ordering fix. Best isolated as its own slice with its own UAT.
**Delivers:** SQLite migration (`application_documents` table); `ApplicationDocument` DAO + SQLite impl (not audited); `ApplicationDocumentService` (upload/list/download/delete, admin-only); REST endpoints `POST/GET/GET/{doc_id}/DELETE /api/applications/{id}/documents` (sub-routes before `/{id}` catch-all); `confirm()` cascade with FS-before-DB file copy + `audited_create!(MemberDocument)` in existing activation tx under `APPLICATION_SERVICE_PROCESS`; `ApplicationServiceDeps` extended with `MemberDocumentDao` + `DocumentStorage` wired in `genossi_bin/src/lib.rs`; frontend upload UI on applications detail; CR-02 `gen_auth_admin!` fix applied to all new methods.
**Avoids:** Pitfalls 7 (FS-before-DB ordering), 8 (carryover inside status guard prevents duplicate), 9 (admin-only upload, content sniffing, UUID storage path, CR-02 fix).

### Phase Ordering Rationale

- P1 before P2: shared helper prevents three divergent HTML implementations; P2 cannot reuse what does not exist.
- P2 before P3: frontend editor needs the `body_html` API wire; ammonia gate must exist before any HTML is accepted from the editor.
- P4 is independent: shares no code with the mail track; running in parallel with P2 or P3 is the most efficient timeline.
- Ammonia gate (P2) must land strictly before or with WYSIWYG (P3) — never after. Hard ordering constraint.

### Research Flags

Phases with standard patterns (skip research during planning):
- **Phase 1:** Pure refactor of existing lettre code; lettre API verified against docs.rs; all integration points cited to file:line.
- **Phase 4 (upload/DAO/REST):** Exact mirror of `member_document.rs`; copy-and-adapt, no unknowns.

Phases needing deeper attention during planning:
- **Phase 2 (minijinja dual-env):** Autoescape footgun requires explicit unit testing before shipping; verify `validate_template` helpers correctly reject invalid HTML templates.
- **Phase 3 (contenteditable in Dioxus):** Highest-uncertainty technical area; consider a proof-of-concept component before full planning. Markdown-toolbar fallback documented if UAT reveals issues.
- **Phase 4 (activation carryover atomicity):** FS-before-DB ordering and rollback cleanup need explicit test coverage: rollback test (no dangling row), re-confirm test (Conflict + 1 doc).

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | All APIs verified against codebase + docs.rs. `ContentTransferEncoding::EightBit` confirmed to exist; `SinglePart::eight_bit()` confirmed NOT to exist in 0.11 (was lettre 0.10). `ammonia 4.1` confirmed on crates.io. |
| Features | HIGH | Grounded in codebase verification + existing todo files. Scope is conservative — extends proven paths. |
| Architecture | HIGH | Integration points cited to exact file:line. Data model matches established entity patterns. Migration strategy follows existing forward-only migration files. |
| Pitfalls | HIGH | Each pitfall cites the specific code location where it occurs or can occur. lettre 8BITMIME non-negotiation behavior verified against library design. |

**Overall confidence:** HIGH

### Gaps to Address

Three open questions that must be resolved before or during requirements definition:

- **8BITMIME relay support (open question a):** Verify whether the production SMTP relay advertises `8BITMIME` in its EHLO. If not, 8bit must remain permanently config-opt-in with quoted-printable default. Resolution: test against the relay during P1 UAT before enabling 8bit.

- **Plain-text fallback strategy (open question b):** Research recommends keeping the existing authored `body` as the text/plain part and making `body_html` strictly additive. This avoids `html2text`/`pulldown-cmark` dependencies and keeps the text part high-quality. The alternative (derive plain from HTML) adds a dependency and produces lower-quality text. Confirm product preference before designing the P3 compose UI.

- **DocumentType for carried-over application file (open question c):** Research recommends `DocumentType::Other` with `description = "Originaler Mitgliedsantrag"` to avoid collision with the `JoinDeclaration` singleton guard (used for the generated join-declaration PDF). If product wants the scan under `JoinDeclaration`, the singleton guard in `member_document.rs:114` must be explicitly handled. Confirm intent before implementing P4 carryover.

## Sources

### Primary (HIGH confidence)
- Genossi codebase (verified 2026-06-29): `genossi_mail/src/worker.rs`, `service.rs`, `render.rs`, `template.rs`, `digest.rs`, `inbox.rs`; `genossi_rest/src/member_document.rs:115/209/242`, `application.rs:479`; `genossi_service_impl/src/application.rs:280`, `document_storage.rs`; `genossi_dao/src/member_document.rs`, `auditable.rs`; `genossi-frontend/src/component/mail_compose/*`; `Cargo.toml:36,60`; migration `20260601000000_extend_mail_job_template_phase.sql`
- `.planning/todos/pending/2026-06-28-html-mail-support-statt-nur-textmails.md`, `.planning/todos/pending/2026-06-27-originalen-mitgliedsantrag-als-datei-attachment-an-applicati.md`
- `.planning/PROJECT.md` — milestone v1.4 definition, audit constraints, CR-02 carry-forward, route-ordering lesson

### Secondary (MEDIUM confidence)
- docs.rs `lettre 0.11` — `ContentTransferEncoding` variants, `SinglePart` constructors verified
- crates.io `ammonia 4.1` — html5ever-based, whitelist defaults, URL scheme allowlist behavior verified
- minijinja 2.x docs — autoescape behavior for `template_from_str` (nameless = no autoescape) verified
- Project memory: `feedback_dioxus_button_type.md`, `feedback_component_first.md`

### Tertiary (MEDIUM — domain knowledge)
- RFC 1341/1521 — Content-Transfer-Encoding rules for multipart; `alternative` ordering (least-preferred first)
- lettre 8BITMIME non-negotiation — based on library design; verify against production relay
- MDN — `document.execCommand` deprecated but stable; `styleWithCSS` behavior across browsers

---
*Research completed: 2026-06-29*
*Ready for roadmap: yes*
