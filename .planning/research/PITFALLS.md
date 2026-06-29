# Pitfalls Research

**Domain:** Adding HTML/8bit mail formatting + WYSIWYG editor + application-document upload/carryover to a Rust/Axum/SQLite + Dioxus-WASM membership system (Genossi v1.4)
**Researched:** 2026-06-29
**Confidence:** HIGH (grounded in this repo's actual mail/render/storage/application code; lettre 0.11 + minijinja behavior verified against source)

> Scope note: every pitfall below is specific to ADDING the four v1.4 features to THIS codebase. The relevant existing code is cited so the roadmapper can map each pitfall to a concrete phase and write a verification check.

---

## Critical Pitfalls

### Pitfall 1: Reusing the existing strict minijinja env for HTML bodies → unescaped member data (XSS + broken HTML)

**What goes wrong:**
`genossi_mail/src/template.rs::strict_env()` builds `Environment::new()` with `UndefinedBehavior::Strict` but **never sets autoescape**. minijinja only auto-enables HTML escaping when a template is loaded with an `.html`/`.htm`/`.xml` *name*; templates here are loaded with `template_from_str` (no name), so **autoescape is OFF**. That is correct for the current plain-text mails. The moment the same `render_template()` path produces an HTML body, `{{ last_name }}` for a member named `Müller & <b>Co</b>` or `O'Brien <script>…</script>` is injected verbatim into HTML — corrupting layout at best, and at worst rendering attacker/member-controlled markup in the mail and in any in-app HTML preview.

**Why it happens:**
The render pipeline is shared (worker live-send AND startup backfill both call `render_template`, per `render.rs`). The "obvious" implementation reuses it for HTML too. Autoescape being name-driven is a silent minijinja footgun: nothing errors, output just isn't escaped.

**How to avoid:**
- Add a **separate HTML environment** with `env.set_auto_escape_callback(|_| AutoEscape::Html)` (or `set_autoescape`) used ONLY for HTML bodies. Keep `strict_env()` exactly as-is for plain text and subject lines.
- Critical mental model: the WYSIWYG/editor HTML is the **template source** (literal markup, passes through), while member fields are **variables** (must be escaped). Autoescape escapes variable output, not literal template text — so autoescape ON is exactly right and does NOT escape the editor's `<b>`/`<a>` tags.
- Subjects must NEVER be HTML-escaped (they are header text) — keep subject rendering on the plain env.
- Add a unit test mirroring `template.rs` style: render an HTML template with a member whose `last_name = "<script>alert(1)</script> & Co"` and assert the output contains `&lt;script&gt;` and `&amp;`.

**Warning signs:**
Mail HTML breaks for members with `&`, `<`, `>`, `"`, `'` in name/company/comment/address; raw `<` appears in rendered preview; security review flags `dangerous_inner_html` fed by un-sanitized render output.

**Phase to address:** HTML-mail backend phase (the phase that introduces `multipart/alternative`).

---

### Pitfall 2: 8bit encoding without SMTP 8BITMIME negotiation → corrupted/rejected mail

**What goes wrong:**
The current send path (`worker.rs::send_mail_for_recipient`) uses `SinglePart::plain(body)`, and lettre auto-selects `Content-Transfer-Encoding: quoted-printable` (the existing tests at `worker.rs:1001` and `:1095` assert QP-or-base64). To get "8bit, no `=` soft-breaks", you must explicitly set the `ContentTransferEncoding::EightBit` header on the part. **lettre's SMTP transport does NOT inspect the server's EHLO `8BITMIME` capability and does NOT downgrade** an 8bit part to quoted-printable when the relay lacks 8BITMIME. If the configured relay (operator-supplied SMTP — unknown server) does not advertise 8BITMIME, raw 8-bit bytes either get silently mangled (high bit stripped → broken umlauts: `Müller` → `M?ller`) or the message is rejected.

**Why it happens:**
Developers assume the mail library negotiates transfer encoding like a browser negotiates content. lettre does not; encoding is a build-time choice on the message, independent of the live SMTP session.

**How to avoid:**
- Treat 8bit as **opt-in, configurable, with a safe default**. Add a config flag (reuse the existing Config system, like `mail_send_interval_seconds`) e.g. `mail_text_encoding = quoted-printable|8bit`, defaulting to the current QP behavior so production is unchanged until the operator opts in.
- Document/verify the production relay (`shifty.nebenan-unverpackt.de`'s configured SMTP) advertises 8BITMIME before enabling 8bit. If feasible, log the EHLO capabilities once at startup so the operator can confirm.
- Keep the body strictly UTF-8 (it already is). 8bit + UTF-8 is fine ONLY across an 8BITMIME path.
- Update the two encoding-assertion tests so they don't hard-fail when 8bit is the chosen mode — make the assertion mode-aware.

**Warning signs:**
Umlauts arrive corrupted at some recipients but not others (relay-dependent); SMTP 554/500 rejects after enabling 8bit; line-length-related `data` errors.

**Phase to address:** 8bit-encoding phase (do this phase FIRST and in isolation — it is the smallest, highest-deliverability-risk change and a clean place to add the encoding-mode config that the HTML phase also benefits from).

---

### Pitfall 3: Broken multipart nesting for HTML + attachments → unreadable mail or lost text fallback

**What goes wrong:**
The current code builds `MultiPart::mixed().singlepart(text)` then appends attachments. The correct structure for "text + HTML + optional attachments" is nested:
- No attachments: `multipart/alternative` { text/plain, text/html } — **text part FIRST**, html SECOND (clients pick the last part they understand).
- With attachments: `multipart/mixed` { `multipart/alternative` { plain, html }, attachment, … }.

If you flatten this (e.g. put html as a sibling of attachments under `mixed`, or omit the alternative wrapper), clients show the raw HTML source, show only the attachment, or drop the HTML. Reversing alternative order makes clients prefer plain text and the formatting is never seen.

**Why it happens:**
The existing attachment loop (`worker.rs:675-697`) is written for the single-text-part case; bolting HTML on without restructuring produces a wrong tree. MIME `alternative` ordering semantics are non-obvious.

**How to avoid:**
- Build a helper that always emits `alternative(plain, html)` and wraps it in `mixed` only when attachments exist. Always include BOTH parts (see Pitfall 4).
- Keep the existing `SinglePart::plain` with explicit `charset=utf-8` (the comment at `worker.rs:653` and the tests guard this — preserve it for the plain leg).
- Test the serialized message bytes (the existing tests already do `email.formatted()` string assertions — extend them) for: presence of `multipart/alternative`, plain-before-html order, and that attachments sit under `mixed`.

**Warning signs:**
Recipients see HTML tags as literal text; HTML mail with a PDF attached shows only the PDF; some clients (Apple Mail vs GMX vs Outlook) render differently.

**Phase to address:** HTML-mail backend phase.

---

### Pitfall 4: Missing or mismatched plain-text alternative → spam score + accessibility/regression

**What goes wrong:**
Sending HTML-only mail (no text/plain leg), or a text leg that is empty / says "view in HTML", sharply raises spam score and breaks plain-text clients and screen readers. Worse for this project: an empty or placeholder text part is a **silent regression** of the current plain-text product, and the audited `MemberDocument` record (created in `worker.rs::try_create_member_document_audited`) would no longer reflect a meaningful body.

**Why it happens:**
WYSIWYG produces HTML; generating a faithful plain-text twin is extra work and easy to skip.

**How to avoid:**
- Always generate a real text/plain alternative. Either (a) keep authoring the plain body as today and let HTML be additive, or (b) derive plain text from the editor model (strip tags, convert `<a href>` to `text (url)`, lists to `- ` lines). Do NOT ship `text = ""`.
- Decide explicitly what the audited `MemberDocument` stores (plain text is the sensible canonical record; document the decision).

**Warning signs:**
Spamassassin `MIME_HTML_ONLY` / `MPART_ALT_DIFF` hits; blank previews in plain-text clients; audit records with empty bodies.

**Phase to address:** HTML-mail backend phase (text-fallback generation) + WYSIWYG phase (plain-text derivation from the editor model).

---

### Pitfall 5: Server-side HTML sanitization missing or done only in the frontend (stored XSS)

**What goes wrong:**
The WYSIWYG editor emits HTML that gets stored (mail template body / job body) and later (a) rendered into outgoing mail and (b) displayed back in the Dioxus admin UI. If sanitization happens only in the WASM frontend, an attacker (or a paste-from-Word blob, or a crafted API call — the API is reachable independently of the UI) can store `<script>`, `<img onerror>`, `<a href="javascript:…">`, `<iframe>`, `<style>`, event handlers, etc. This is classic **stored XSS** in the board-facing admin app, which is the highest-trust surface in the system (it can read all member PII, IBANs, audit log).

**Why it happens:**
"The editor only produces safe HTML" is false — the editor is a UI convenience, not a security boundary. The REST endpoint accepts arbitrary `body` strings (see `MailTemplateService::create/update` which take `body: &str` with zero sanitization today).

**How to avoid:**
- **Sanitize on the server, at the service/REST boundary, as the mandatory gate.** Add the `ammonia` crate (not currently a dependency) and sanitize HTML on write (template create/update, mail job create) AND/OR on render. Sanitizing on write keeps stored data clean; sanitizing on render is defense-in-depth — do both if cheap, but the write-side gate is non-negotiable.
- Configure ammonia tightly: allow only formatting tags the editor actually produces (`b/strong, i/em, u, a, ul/ol/li, p, br, span`), strip all event handlers, allow `href` only with `http/https/mailto` schemes (ammonia's `url_schemes` allowlist — this kills `javascript:`/`data:` URIs), force `rel="noopener noreferrer"` on links.
- Note `minijinja` autoescape (Pitfall 1) protects member *variables*; it does NOT sanitize the *template body itself* (the editor HTML is literal template text and passes through un-escaped by design). So autoescape and ammonia are complementary, not redundant — you need both.
- Treat existing plain-text templates as plain: do not run ammonia over a template that is declared plain-text (it would mangle `<` in legitimate text like `a < b`). Track a per-template/per-job content-type flag.

**Warning signs:**
A stored template renders `<script>` when previewed; pen-test of `POST /api/mail/templates` with `<img src=x onerror=alert(1)>` survives round-trip; `javascript:` links present in stored bodies.

**Phase to address:** HTML-mail backend phase (server-side ammonia gate) — must land BEFORE or WITH the WYSIWYG phase, never after.

---

### Pitfall 6: Rendering inbound/stored HTML in Dioxus via `dangerous_inner_html` without sanitization

**What goes wrong:**
The inbox already parses `raw_html_body: Option<String>` and `has_html_body` (`genossi_mail/src/inbox.rs:182-183`), and the inbox page currently only shows `body_text` (`inbox_page.rs:333`). v1.4's HTML focus will tempt rendering inbound HTML, or rendering the WYSIWYG preview, via Dioxus `dangerous_inner_html`. Inbound mail HTML is **fully attacker-controlled** (anyone can email the cooperative). Piping it into `dangerous_inner_html` is direct XSS in the board app. Dioxus normally HTML-escapes `{interpolation}` (noted in `mail_recipient_rendered_content.rs:9`); `dangerous_inner_html` bypasses that — it is the only XSS vector in the WASM UI and is already used for QR SVG (`qr_card.rs:63`, where input is controlled).

**Why it happens:**
"We need to show formatting" → reach for `dangerous_inner_html`. The QR-card precedent makes it look blessed.

**How to avoid:**
- Never feed un-sanitized HTML to `dangerous_inner_html`. Sanitize on the **server** before it reaches the frontend (the frontend is WASM — bundling/maintaining a sanitizer there is worse than doing it server-side where ammonia already lives after Pitfall 5).
- For inbound mail specifically: prefer continuing to show `body_text`; if HTML display is wanted, render server-sanitized HTML, and consider an iframe sandbox / stripping remote `<img>` (tracking-pixel + privacy concern) and all links-to-scripts.
- Keep a single documented rule: "HTML reaches `dangerous_inner_html` only after passing the server ammonia gate."

**Warning signs:**
New `dangerous_inner_html` call sites whose data originates from inbound mail or user input; a test email with `<script>`/`<img onerror>` executes in the inbox view.

**Phase to address:** WYSIWYG/preview phase (admin-authored preview) and any inbox-HTML phase. If inbound-HTML rendering is in scope, make it its own gated decision.

---

### Pitfall 7: Orphaned files / partial carryover on application activation (file-vs-DB atomicity)

**What goes wrong:**
`ApplicationService::confirm()` (`application.rs:280-420`) runs a single SQLite transaction: create Member + Eintritt + Aufstockung + update Application, all audited. v1.4 adds "copy the uploaded application file into a `MemberDocument` on activation." Filesystem writes (via `FilesystemDocumentStorage::save`) are **not transactional with SQLite**. Two failure shapes:
1. File copied to disk, then the DB tx rolls back → orphaned file, no DB row, no audit (disk leak; the project already has a noted orphan/leak class of bug, e.g. Phase 13 `std::mem::forget(tempdir)`).
2. DB row committed referencing a `relative_path`, but the file copy failed/was skipped → a `MemberDocument` that 404s on download (`DocumentStorage::load` → `StorageError::NotFound`).

**Why it happens:**
Mixing a non-transactional resource (FS) into a carefully-atomic DB cascade. The temptation is to `save()` the file inside the tx block.

**How to avoid:**
- Prefer **reusing the already-on-disk application file** rather than re-uploading: if the upload at application-attach time already stored the bytes at a stable path, the carryover can create a copy BEFORE `commit`, with cleanup-on-rollback. Pick one ordering and stick to it:
  - Recommended: write the new member-document file to disk first (idempotent, content-addressed by new UUID path), THEN run the DB tx; on tx error, best-effort `delete()` the just-written file (log on failure). Net effect: a rolled-back activation may transiently leak one file, but never produces a dangling DB row (the worse, user-visible failure).
- Verify the file exists/loads before creating the row (cheap `load`/metadata check) so you never persist a 404 document.
- Add the `MemberDocument` create to the SAME audited cascade (it is an audited entity) using the existing `audited_create!` pattern, sharing `APPLICATION_SERVICE_PROCESS` so the carryover is forensically linked to the activation.

**Warning signs:**
`MemberDocument` rows whose download 404s; `documents/` dir grows with files no row references; audit log shows a member doc create with no corresponding activation.

**Phase to address:** Application-document phase (the carryover sub-feature).

---

### Pitfall 8: Duplicate carryover on re-activation / re-confirm

**What goes wrong:**
If activation is ever retried, or a rejected/re-opened application is confirmed again, the carryover could attach the original document to the member twice (the project explicitly allows multiple `MemberDocument` rows; there is no uniqueness constraint, mirroring the deliberate `RepaymentEntry` no-unique-PK decision). The current `confirm()` guards `status != Offen → Conflict` (`application.rs:303`), which today blocks double-confirm — but the carryover MUST be added strictly inside that guard, and any future "re-open" path must be considered.

**Why it happens:**
The status guard exists for the member-creation cascade; a new sub-feature can accidentally be wired before/outside it, or a retry-on-transient-error loop can re-run the side effect.

**How to avoid:**
- Place the carryover entirely within the existing `status == Offen` guarded block, in the same tx, so it cannot run twice for an already-confirmed application.
- If idempotency beyond the status guard is desired, key the carried `MemberDocument` by a deterministic marker (e.g. `document_type = "join_application"` + source application id in description) and skip if one already exists — analogous to `find_repayment_letter_for_recipient`'s fingerprint pattern (`worker.rs:84`).
- Add an E2E test: confirm once → 1 doc; confirm again → `Conflict`, still 1 doc.

**Warning signs:**
Two identical application PDFs on one member; audit log shows two carryover creates for one application.

**Phase to address:** Application-document phase.

---

### Pitfall 9: Unauthenticated / unvalidated file upload (the application submit path is PUBLIC)

**What goes wrong:**
`ApplicationService::submit()` is called with actor `"PUBLIC"` (`application.rs:222`) — applications can be created without auth. If the file-upload endpoint is naively attached to the public submission flow, you get an **unauthenticated arbitrary-file-upload**: disk-fill DoS, malware storage, oversized multipart memory blowups. Even on an admin-only upload, missing limits/validation are dangerous: Axum multipart will buffer large bodies; a client-supplied filename used in the storage path enables path traversal; a spoofed `Content-Type` lets an executable masquerade as a PDF.

**Why it happens:**
The milestone text ("Vorstand hinterlegt den Antrag") implies admin-only, but the existing application create is public, so the boundary is ambiguous and easy to get wrong. File validation is tedious and often deferred.

**How to avoid:**
- Make the upload endpoint **admin-only** (privilege `manage_members`, same as the other application mutations), attaching the file to an existing application — do NOT bolt it onto the public `submit`.
- **Fix the permission-ordering carry-forward (CR-02)** here: existing methods call `current_user_id()` BEFORE `check_permission()` (`application.rs:287-294`). New upload/carryover methods must check permission first (extract the planned `gen_auth_admin!` helper) to avoid the documented side-channel + `"SYSTEM"` audit-fallback smell.
- Enforce a max upload size (Axum `DefaultBodyLimit` / multipart field limit) and reject early.
- Validate the file type by **content sniffing**, not the client `Content-Type` or extension (check magic bytes; allow PDF + common image types only). Store the validated/normalized mime, not the client-claimed one.
- **Never put the client filename in the storage path.** Generate a server-side UUID path (the codebase already uses `static_documents/<uuid>` and `MemberDocument.relative_path` conventions). `FilesystemDocumentStorage::full_path` has path-clean traversal protection (`document_storage.rs:25-59`) — rely on it, but still derive the path from a UUID, keeping the original filename only as the display `file_name`.

**Warning signs:**
Upload reachable without a session; 100 MB upload OOMs the server; a `.pdf` that is actually HTML/JS; storage paths containing user-controlled segments.

**Phase to address:** Application-document phase (upload sub-feature) — and it is the right place to land the CR-02 `gen_auth_admin!` fix.

---

### Pitfall 10: WYSIWYG ↔ Dioxus signal desync and paste-from-Word junk

**What goes wrong:**
`contenteditable` maintains its own DOM; Dioxus owns a virtual DOM driven by signals. If you bind `contenteditable` naively and also write the signal back on every `oninput`, Dioxus re-renders and resets the caret to position 0 (classic contenteditable cursor jump), or the editor and signal diverge so the body that gets sent ≠ what the board saw. Pasting from Word/Outlook injects huge `<o:p>`, `mso-` styles, `<font>`, nested `<span style>` garbage that bloats the stored body and defeats narrow sanitization.

**Why it happens:**
contenteditable is notoriously hard to make declarative; Dioxus's reactive model fights the browser's direct DOM mutation. This bites EVERY contenteditable integration.

**How to avoid:**
- Do not round-trip the signal into the DOM on every keystroke. Read the editor's HTML on demand (on blur / before submit) rather than two-way binding every input.
- Follow the existing Dioxus button/reload lesson (`r#type: "button"` + onclick, never form-submit — project memory `feedback_dioxus_button_type.md`) so the editor's toolbar buttons don't reload the page.
- Strongly consider a small, battle-tested JS editor (lightweight contenteditable lib) wrapped as ONE Dioxus component (Component-First — `genossi-frontend/src/component/`), with a clean `value`/`onchange` boundary, rather than hand-rolling contenteditable in RSX.
- Clean paste: strip on paste (or rely fully on the server ammonia gate to discard `mso-`/`<font>`/`style` — ensure the ammonia config strips `style` and `class` unless explicitly needed).
- Accessibility: provide labels, keyboard operability for toolbar, and an HTML/source fallback; contenteditable without ARIA is unusable for screen readers.

**Warning signs:**
Caret jumps to start while typing; sent body differs from on-screen; stored bodies balloon to tens of KB after a paste; toolbar click reloads the page.

**Phase to address:** WYSIWYG phase.

---

### Pitfall 11: Missing List-Unsubscribe and bulk-send deliverability hygiene

**What goes wrong:**
The system does **bulk** sends (Massenmail to members; `worker.rs` loops recipients). HTML bulk mail without `List-Unsubscribe` (and ideally `List-Unsubscribe-Post` for one-click) scores higher as spam and, for larger lists, risks the configured relay's reputation. Long HTML lines (>~990 chars, common in WYSIWYG output) violate RFC 5322 line-length limits and, on a non-8BITMIME path, force re-encoding or get the message rejected.

**Why it happens:**
Transactional-mail mindset carried into bulk; lettre won't add `List-Unsubscribe` for you; WYSIWYG emits unwrapped long lines.

**How to avoid:**
- Add `List-Unsubscribe` (mailto and/or URL) on bulk jobs. For a small cooperative an unsubscribe *mailbox*/contact may suffice operationally, but the header still helps deliverability.
- Ensure the HTML part uses a transfer encoding that handles long lines safely (quoted-printable wraps automatically; if you choose 8bit per Pitfall 2, confirm the relay accepts long lines or wrap them).
- This is a deliverability hygiene item, not a correctness blocker — but flag it so the roadmapper doesn't ship HTML bulk mail blind.

**Warning signs:**
Mails landing in spam after the HTML switch; relay rate-limiting/reputation warnings; `data` command errors on long-line messages.

**Phase to address:** HTML-mail backend phase (headers) — verify during the HTML send phase's UAT against a real client.

---

## Technical Debt Patterns

| Shortcut | Immediate Benefit | Long-term Cost | When Acceptable |
|----------|-------------------|----------------|-----------------|
| Sanitize HTML only in the WASM frontend | Fast; no new Rust dep | Stored XSS via direct API calls; security boundary bypassed | Never — server-side ammonia is mandatory |
| Reuse `strict_env()` for HTML bodies | One render path | Unescaped member PII/markup in mail + previews | Never for HTML; keep it for plain text |
| Enable 8bit globally, no config flag | Simpler code | Corrupted umlauts / rejects on non-8BITMIME relays; production regression | Never — must be opt-in with QP default |
| `save()` the carryover file inside the DB tx | Looks atomic | Orphan files on rollback; or 404 docs | Never — order FS-before-DB with rollback cleanup |
| Use client filename in storage path | Preserves name | Path traversal; collisions | Never — UUID path, filename as display only |
| Trust client `Content-Type` for uploads | Less code | Spoofed executables stored as "PDF" | Only with content sniffing added later (track as debt) |
| Skip plain-text alternative (HTML-only) | Half the authoring | Spam score, a11y, audit-record regression | Never |
| Hand-roll contenteditable in RSX | No JS dep | Caret bugs, signal desync, paste junk, a11y gaps | Prototype only; productionize as a wrapped component |
| Keep `current_user_id()` before `check_permission()` in new upload methods | Matches existing code | Carries forward CR-02 side-channel + SYSTEM-audit smell | Never in new code — fix via `gen_auth_admin!` |

## Integration Gotchas

| Integration | Common Mistake | Correct Approach |
|-------------|----------------|------------------|
| lettre 0.11 SMTP | Assuming it negotiates/downgrades 8BITMIME | It does not; choose encoding at build time, gate 8bit behind relay capability/config |
| lettre MultiPart | Flattening html as sibling of attachments under `mixed` | Nest `alternative(plain,html)` inside `mixed` with attachments; plain part first |
| lettre SinglePart | Dropping the explicit `charset=utf-8` on the plain part | Keep `SinglePart::plain` (preserves charset; guarded by tests `worker.rs:978`,`:1062`) |
| minijinja 2.x | Expecting autoescape from `template_from_str` | Name-less templates are NOT auto-escaped; set an explicit HTML autoescape env |
| ammonia | Default allowlist too permissive (allows `img`, `class`, remote refs) | Restrict to editor's tags; allowlist url schemes to http/https/mailto; strip `style`/`on*` |
| Axum multipart | No body-size limit; buffering full upload in memory | `DefaultBodyLimit` + size check; validate before persist |
| FilesystemDocumentStorage | Assuming FS write joins the SQLite tx | It doesn't; sequence FS-then-DB with rollback cleanup; verify file loadable before row commit |
| Dioxus `dangerous_inner_html` | Feeding inbound/user HTML directly | Only server-sanitized HTML; default to escaped `{text}` |
| IMAP inbound `raw_html_body` | Rendering attacker-controlled HTML to look feature-complete | Keep `body_text` default; sanitize + sandbox if HTML display is truly required |

## Performance Traps

| Trap | Symptoms | Prevention | When It Breaks |
|------|----------|------------|----------------|
| Buffering large uploads fully in memory | RAM spike per upload | Body-size limit; reject early | A single multi-MB scan on a small self-hosted box |
| ammonia-sanitizing on every render in the bulk loop | Slower per-recipient send | Sanitize once on write; render is already per-recipient (`worker.rs` loop) | Large repayment bulk sends (hundreds of members) |
| HTML body bloat from paste-from-Word | Multi-KB rows; bigger mails | Strip on paste + ammonia `style`/`class` removal | After board pastes formatted Word content |
| Re-reading application file from disk per carryover unnecessarily | Extra IO at activation | Single read; one copy | Negligible at this scale, but avoid in a loop |

## Security Mistakes

| Mistake | Risk | Prevention |
|---------|------|------------|
| No server-side HTML sanitization | Stored XSS in the highest-trust admin app (PII/IBAN/audit access) | Mandatory ammonia gate at service/REST write |
| Rendering inbound mail HTML unsanitized | XSS from anyone who can email the co-op | Sanitize server-side; prefer text; sandbox if rendered |
| `javascript:`/`data:` URIs in links | Script execution / phishing via stored links | ammonia url-scheme allowlist (http/https/mailto only) |
| Public/unauthenticated file upload | DoS, malware storage | Admin-only endpoint; auth before side effects |
| Client filename in storage path | Path traversal / overwrite | UUID-based server path; rely on `full_path` clean-check |
| Content-Type spoofing | Executable stored as PDF | Magic-byte sniffing; store validated mime |
| Member data unescaped in HTML mail | Injection / mail corruption / phishing | minijinja HTML autoescape for variables |
| New upload code repeats CR-02 ordering | Permission side-channel + `"SYSTEM"` audit attribution | Check permission before `current_user_id`/work (`gen_auth_admin!`) |
| Carryover not audited | Document on member with no audit trail (Application/MemberDocument are audited) | Use `audited_create!` in the activation cascade |

## UX Pitfalls

| Pitfall | User Impact | Better Approach |
|---------|-------------|-----------------|
| Caret jumps while typing in editor | Board can't write a mail | Don't two-way-bind contenteditable per keystroke |
| Toolbar button reloads page | Lost draft (project has a history of this exact bug) | `r#type:"button"` + onclick, never form-submit |
| HTML preview differs from what recipients see | Board sends mis-formatted mail | Render preview through the SAME server render+sanitize path |
| No plain-text fallback shown to plain clients | Some members see blank/garbled mail | Always generate real plain text |
| Inbound HTML mail shows nothing or raw tags | Board can't read replies | Show sanitized HTML or fall back to `body_text` (current behavior) |
| Editor not keyboard/screen-reader accessible | Excludes some board members | ARIA labels, keyboard toolbar, source fallback |

## "Looks Done But Isn't" Checklist

- [ ] **HTML mail:** Often missing the text/plain alternative — verify both parts present and ordered plain-then-html in `email.formatted()`.
- [ ] **HTML escaping:** Often missing variable escaping — verify a member named `<script> & Co` renders as `&lt;script&gt; &amp;` in the HTML body.
- [ ] **Sanitization:** Often only in frontend — verify `POST` of `<img src=x onerror=alert(1)>` to the template/job API is stripped server-side.
- [ ] **8bit:** Often missing relay-capability gate — verify default stays quoted-printable and 8bit is config-opt-in; confirm relay 8BITMIME before enabling.
- [ ] **Upload:** Often missing size limit + content sniffing — verify oversized and spoofed-type uploads are rejected; auth required.
- [ ] **Storage path:** Often uses client filename — verify path is UUID-derived and traversal attempts return `ValidationError`.
- [ ] **Carryover atomicity:** Often leaves orphans/404s — verify rollback leaves no dangling DB row, and a confirmed app's document downloads.
- [ ] **Re-confirm:** Often duplicates — verify second confirm returns Conflict and produces no second document.
- [ ] **Audit:** Often skipped for carryover — verify an `audited_create!` entry exists for the member document linked to the activation process string.
- [ ] **Backward compat:** Often regresses plain text — verify existing plain templates, the application confirmation mail (`application.rs:108`), the digest worker, and reply mails still send as before with default config.
- [ ] **List-Unsubscribe:** Often absent on bulk — verify header present on bulk HTML jobs.
- [ ] **dangerous_inner_html:** Often un-audited new call sites — grep for new uses and confirm each is fed only server-sanitized data.

## Recovery Strategies

| Pitfall | Recovery Cost | Recovery Steps |
|---------|---------------|----------------|
| Stored XSS in templates/jobs | MEDIUM | Add ammonia gate; one-time sanitize sweep of existing stored bodies; audit who could have injected |
| 8bit corrupting mail in production | LOW | Flip config back to quoted-printable (default); resend affected mails |
| Orphaned files | LOW | Reconciliation job: list `documents/` vs `MemberDocument.relative_path`; delete unreferenced |
| 404 documents (row without file) | MEDIUM | Identify rows whose `load()` fails; re-derive from source application file or soft-delete + re-carry |
| Duplicate carryover | LOW | Soft-delete the duplicate `MemberDocument` (existing soft-delete pattern) |
| HTML-only mail in spam | LOW | Add text/plain part + List-Unsubscribe; warm relay reputation |
| contenteditable desync shipping wrong bodies | MEDIUM | Switch to read-on-submit; add test comparing editor output to payload |

## Pitfall-to-Phase Mapping

| Pitfall | Prevention Phase | Verification |
|---------|------------------|--------------|
| 8bit without 8BITMIME (P2) | 8bit-encoding phase (first) | Config defaults to QP; encoding-mode tests pass; relay 8BITMIME confirmed |
| Unescaped member data in HTML (P1) | HTML-mail backend phase | Test: malicious member name appears escaped |
| Broken multipart nesting (P3) | HTML-mail backend phase | `email.formatted()` shows `alternative` in `mixed`, plain-first |
| Missing text fallback (P4) | HTML-mail backend + WYSIWYG | Non-empty text/plain part asserted |
| No server-side sanitization (P5) | HTML-mail backend phase | API injection test stripped; ammonia config reviewed |
| `dangerous_inner_html` XSS (P6) | WYSIWYG/preview (+ inbox-HTML if scoped) | Grep call sites; `<script>` email doesn't execute |
| Orphan/partial carryover (P7) | Application-document phase | Rollback test: no dangling row; doc downloads |
| Duplicate carryover (P8) | Application-document phase | Re-confirm → Conflict, 1 doc |
| Unauthenticated/unsafe upload (P9) | Application-document phase | Auth required; size/type/path tests; CR-02 ordering fixed |
| contenteditable desync / paste junk (P10) | WYSIWYG phase | Caret/typing test; sent==shown; paste sanitized |
| Missing List-Unsubscribe / long lines (P11) | HTML-mail backend phase | Header present; real-client deliverability check |
| Plain-text backward-compat regression (cross-cutting) | Every phase (UAT gate) | Existing plain mails/digest/reply unchanged with default config |

## Sources

- This repository (HIGH): `genossi_mail/src/worker.rs` (send path, encoding tests, audited MemberDocument), `genossi_mail/src/render.rs` + `template.rs` (minijinja `strict_env`, no autoescape), `genossi_service_impl/src/document_storage.rs` (path-clean traversal guard), `genossi_service_impl/src/application.rs` (confirm() activation cascade, PUBLIC submit, CR-02 ordering), `genossi_mail/src/inbox.rs` (`raw_html_body`/`has_html_body`), `genossi-frontend/src/page/inbox_page.rs` + `component/qr_card.rs` + `component/mail_recipient_rendered_content.rs` (`dangerous_inner_html` usage), `Cargo.toml` (lettre 0.11, no ammonia).
- `.planning/PROJECT.md` (HIGH): v1.4 goal, constraints, CR-02 carry-forward, soft-delete/audit/Component-First patterns.
- Project memory (HIGH): `feedback_dioxus_button_type.md` (button-reload bug), `feedback_component_first.md`.
- lettre 0.11 behavior re: 8BITMIME non-negotiation and build-time transfer encoding (MEDIUM — based on library design; verify against the configured production relay before enabling 8bit).
- General mail/MIME/XSS domain knowledge: MIME multipart/alternative ordering, RFC 5322 line limits, List-Unsubscribe, ammonia allowlisting (MEDIUM-HIGH).

---
*Pitfalls research for: HTML/8bit mail + WYSIWYG + application-document carryover in Genossi (Rust/Axum/SQLite + Dioxus-WASM)*
*Researched: 2026-06-29*
