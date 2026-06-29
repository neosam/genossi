# Feature Research — v1.4 Mail-Formatierung & Antrags-Dokumente

**Domain:** Admin/back-office mail composition + document attachment for a small German cooperative board (Vorstand, non-technical), inside an existing Rust/Axum + Dioxus app
**Researched:** 2026-06-29
**Confidence:** HIGH (grounded in existing Genossi codebase + standard email/WYSIWYG domain knowledge; no external research needed for these well-established patterns)

> Scope note: This research is strictly about the four named v1.4 features. It does NOT propose new unrelated scope. Complexity is rated against the *existing* Genossi infrastructure, which already does most of the heavy lifting.

## What already exists (foundation for all four features)

Verified by reading the codebase — this is why several "table stakes" are LOW complexity:

- **Mail sending** (`genossi_mail/src/worker.rs`): uses `lettre` with `SinglePart::plain` + `MultiPart::mixed()` for attachments. Text body already forced to `text/plain; charset=utf-8` (GMX-Android umlaut fix). A test already asserts current encoding is `quoted-printable` (~line 1002) — that test is the natural pivot point for the 8bit feature.
- **Templating**: `minijinja` (strict mode, `{% if X is defined %}` pattern) already renders subject/body with member + payout variables. Round-trip via serde_json/BTreeMap.
- **Frontend compose UI** (`genossi-frontend/src/component/mail_compose/`): already componentized — `body_editor.rs` (currently a plain `<textarea h-40>`), `subject_input.rs`, `template_selector.rs`, `template_preview.rs`, `template_var_buttons.rs`, `template_tester.rs`, `attachment_picker.rs`. The WYSIWYG work is **replacing one existing component**, not greenfield.
- **Attachments**: `attachment_picker` already selects existing MemberDocuments + StaticDocuments (per project memory — mail "attachments" = document selection, not local upload). Mail worker loads files via `DocumentStorage::load(relative_path)`.
- **Document storage** (`genossi_service_impl/src/document_storage.rs`): `FilesystemDocumentStorage` with `save(relative_path, data)` / `load`. `relative_path` convention = `{doc_id}.{ext}`. MemberDocument upload already exists via multipart (`genossi_rest/src/member_document.rs` + `member_document.rs::upload`).
- **MemberDocument entity**: `member_id, document_type, description, file_name, mime_type, relative_path, ...` — auditable.
- **Application activation** (`genossi_service_impl/src/application.rs` ~line 303): single-tx flow that `audited_create!`s Member + Eintritt + Aufstockung actions and `audited_update!`s Application to `Bestaetigt`, all under shared `APPLICATION_SERVICE_PROCESS`. This is exactly where attachment carry-over hooks in.

## Feature Landscape

### Table Stakes (Users Expect These)

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| **multipart/alternative (text + HTML)** | Modern mail clients render HTML; a board sending member comms expects bold/links to show up. Plain-text fallback non-negotiable for accessibility + deliverability. | **MEDIUM** | `lettre` natively supports `MultiPart::alternative()`. Part-tree: `alternative(plain, html)`; when file attachments present, wrap as `mixed(alternative(...), attachments...)`. Touches `worker.rs` + `service.rs` build paths. Real cost = restructuring the existing `mixed`-only builder to nest correctly. |
| **Plain-text auto-derived from HTML** | Board should write once, not maintain two bodies. Non-technical users will not hand-author a text version. | **MEDIUM** | Decision flagged in the todo. Recommended: derive text from the WYSIWYG HTML server-side (strip tags, `<br>/<p>`→newline, `<a href>`→"text (url)", `<li>`→"- "). Server-side derive = one source of truth. Small HTML→text helper in `genossi_mail`. |
| **Template variables interpolate inside HTML** | Existing `{{ payout_amount }}` etc. must work in the HTML body too, or HTML mail regresses current text mail. | **MEDIUM** | minijinja auto-escaping is the trap: values rendered into HTML must be HTML-escaped (names with `&`, `<`), but the author's own markup must NOT be escaped. Autoescape on for `.html` templates, off for text. The single most error-prone sub-task — call it out for the roadmap. |
| **8bit encoding (no visible `=` soft-breaks)** | Current quoted-printable leaks `=`-line-wrap artifacts into some clients; board sees ugly mails. Direct user complaint driving the milestone. | **LOW–MEDIUM** | Set `ContentTransferEncoding::EightBit` on the SinglePart(s). Caveat: 8bit requires the SMTP relay to advertise `8BITMIME` (most do; verify production relay). Risk: non-8BITMIME relay rejects/re-encodes. Update the existing quoted-printable assertion test. |
| **Live preview of the formatted result** | Non-technical users cannot reason about HTML source; they need to see the rendered mail. `template_preview.rs` already exists for text. | **LOW** | For true WYSIWYG (contenteditable), the editor *is* the preview. For template-variable preview, extend `template_preview`/`template_tester` to render HTML. |
| **WYSIWYG toolbar: bold, italic, bullet/numbered list, link** | Literal milestone goal — board formats "fett/kursiv/Links/Listen" without HTML knowledge. | **MEDIUM–HIGH** | Dominant cost/risk of the milestone. See WYSIWYG section below. |
| **Link insertion UX (select text → add URL)** | Lists + links are the named formatting set; links are the fiddliest for non-technical users. | **MEDIUM** | Minimal: prompt for URL on a "link" button applied to selection; auto-prefix `https://`; validate. |
| **Sanitized HTML output** | Editor output becomes a mail body; must not carry junk markup or unsafe constructs; keeps derived text clean. | **MEDIUM** | Whitelist tags/attributes (`b/strong, i/em, a[href], ul/ol/li, p, br, h2/h3`). Sanitize **server-side** regardless of frontend. Prevents paste-garbage and client breakage. |
| **Single file upload on Application (scanned PDF)** | A membership application is one signed document; board scans it to one PDF. Storing it on the Application is the point of todo #2. | **LOW–MEDIUM** | Mirror existing MemberDocument multipart upload. Store via `DocumentStorage`, not DB (explicit in todo). Add `relative_path/file_name/mime_type` columns to Application (migration). Application already auditable → `audited_update!`. |
| **Auto-carry attachment to Member on activation** | Core value: "ohne erneutes manuelles Hochladen direkt am Mitglied verfügbar". Without it the upload feature is half-built. | **MEDIUM** | On activate (existing single-tx), if Application has an attachment: **copy** the file to a new MemberDocument `relative_path` and `audited_create!` a MemberDocument under `APPLICATION_SERVICE_PROCESS`. See carry-over semantics below. |
| **Attachment visible/downloadable on Application AND Member detail** | Board needs it where they work; Member-side download already exists for MemberDocuments. | **LOW** | Member side reuses existing MemberDocument UI. Application side needs a small view/download affordance (reuse Phase-19 attachment component per todo). |

### Differentiators (Competitive Advantage)

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| **Paste-from-Word/Outlook cleanup** | Board members will compose in Word/Outlook and paste. Raw paste injects `mso-` styles, smart quotes, nested spans → broken mail. Cleaning paste is the difference between "usable" and "support nightmare". | **MEDIUM** | Strip on paste: keep text + basic formatting, drop style/class/font tags. High real-world value for this exact user group — near-table-stakes here despite the label. |
| **Saved/reusable HTML templates** | Templating already exists for text; letting board save formatted HTML templates (Begrüßung, Antragsbestätigung) compounds the WYSIWYG value. | **MEDIUM** | Largely free — extend the existing template system to store an HTML body alongside text. Main work is the HTML/autoescape rendering already required for table stakes. |
| **Branding / letterhead (logo + footer in HTML)** | A consistent header/footer makes board mail look official to members. | **MEDIUM** | Needs image embedding decision (CID vs. hosted URL). Defer unless explicitly requested — adds CID/MIME complexity. |
| **Inline image embedding (CID)** | Logos/photos inside the body. | **HIGH** | `multipart/related` wrapping + CID references; significant MIME nesting on top of alternative+mixed. Anti-feature-adjacent for v1.4 — defer. |
| **Multiple files on one Application** | Some applications have addenda (ID copy, SEPA mandate). | **LOW (incremental)** | Start single-file (matches todo). The carry-over loop generalizes naturally if needed. Design the column/relation so multi is not a rewrite. |

### Anti-Features (Commonly Requested, Often Problematic)

| Feature | Why Requested | Why Problematic | Alternative |
|---------|---------------|-----------------|-------------|
| **Full drag-and-drop email builder (Mailchimp-style blocks, columns, responsive grid)** | "Make our mails look like a newsletter." | Massive scope; responsive HTML-email is its own discipline (tables, inline CSS, Outlook/Gmail/Apple quirks). Wildly out of proportion for a small board sending member notices. | Simple rich-text body (bold/italic/list/link) + optional fixed letterhead. Keep to the named feature set. |
| **Authoring two separate bodies (text AND HTML by hand)** | "Full control over both versions." | Non-technical users let the text version rot or leave it empty → broken fallback; double maintenance. | Auto-derive plain text from HTML (table stakes). One source of truth. |
| **Heavy JS WYSIWYG lib via wasm-bindgen (TinyMCE/CKEditor/Quill)** | "Use a battle-tested editor." | Bundling a large JS editor into a Dioxus/WASM app adds JS interop surface, asset weight, version coupling; output still needs sanitizing. Overkill for 4 formatting commands. | Thin Dioxus `contenteditable` wrapper using `execCommand`/Selection via `web-sys` for the ~5 commands; a tiny vetted lib only if execCommand proves too painful. One reusable component. |
| **Storing uploaded application files in the DB (BLOB)** | "Simpler, one place." | DB bloat, backup size, contradicts existing `DocumentStorage` filesystem pattern. | Filesystem via `DocumentStorage` (explicit in todo). |
| **Move (not copy) the file on activation** | "Avoid duplicate storage." | Destroys the Application's record of what was submitted; breaks audit story and re-display on the Application; fragile if activation is re-examined. | **Copy** to a new MemberDocument relative_path. Application keeps its attachment; Member gets an independent one. Disk cost negligible. |
| **Rich HTML inbox digest immediately** | "Make the digest pretty too." | `digest.rs` builds a plain string body today; HTML-ifying it is extra surface with low payoff for an internal board-only mail. | Land HTML infra for member-facing comms first; HTML digest is a trivial follow-on once `multipart/alternative` exists. Don't gate v1.4 on it. |

## Feature Dependencies

```
8bit encoding (worker.rs/service.rs)
    └── independent, smallest; can ship first as a self-contained slice

multipart/alternative (text + HTML) backend
    ├──requires──> HTML→text derive helper (one source of truth)
    └──requires──> minijinja HTML-autoescape handling (escape vars, not markup)

WYSIWYG editor (frontend, replaces body_editor.rs)
    ├──produces──> sanitized HTML  ──feeds──> multipart/alternative HTML part
    ├──enhanced by──> paste-from-Word cleanup
    └──enhanced by──> live preview (editor is its own preview)

Saved HTML templates ──enhances──> WYSIWYG + multipart/alternative (reuses autoescape work)

Application file upload (new columns + DocumentStorage)
    └──requires──> activation carry-over (copy → MemberDocument, audited_create!)
                       └──reuses──> existing Application activation single-tx flow
                       └──reuses──> existing MemberDocument display on Member detail
```

### Dependency Notes

- **HTML mail backend is the keystone for three sub-features** (HTML part, template HTML rendering, eventual HTML digest). Build the MIME part-tree + autoescape once, correctly.
- **WYSIWYG depends on the backend accepting an HTML body**, but they can be built in parallel if the API contract (send both text+html) is fixed early. The editor's output format (sanitized HTML subset) must match what the backend sanitizes / derives text from.
- **8bit is fully independent** — no dependency on HTML; a standalone first slice that de-risks the milestone early. When both HTML and text parts exist, the encoding choice applies per-part.
- **Application upload and carry-over are a self-contained vertical** with zero dependency on the mail features — a parallel workstream / separate phase.
- **The two halves of the milestone (mail formatting vs. application documents) are independent** and can be sequenced either way.

## MVP Definition

### Launch With (v1.4 core)

- [ ] **8bit encoding** — direct complaint; smallest self-contained win; ship first.
- [ ] **multipart/alternative (HTML + auto-derived plain text)** — the keystone; includes HTML→text helper and minijinja autoescape-correctness.
- [ ] **Template variables work inside HTML body** — otherwise HTML mail regresses existing payout/member templating.
- [ ] **WYSIWYG editor component** (bold, italic, bullet + numbered list, link) — named feature set; replaces `body_editor.rs`; output sanitized to a tag whitelist.
- [ ] **Application single-file upload** (scanned PDF) via DocumentStorage + new Application columns + multipart upload (mirror MemberDocument).
- [ ] **Auto-carry to Member on activation** (copy → MemberDocument via `audited_create!` in existing activation tx) with edge cases below handled.
- [ ] **View/download of the attachment on Application and Member detail.**

### Add After Validation (v1.x)

- [ ] **Paste-from-Word cleanup** — strongly recommended for v1.4 given the user group; acceptable as a fast follow if it threatens timeline.
- [ ] **Saved HTML templates** — extend existing template store to hold HTML; cheap once autoescape exists.
- [ ] **HTML inbox digest** — trivial reuse of the new alternative-part builder.
- [ ] **Multiple files per Application** — generalize the carry-over loop.

### Future Consideration (v2+)

- [ ] **Branding/letterhead + inline image embedding (CID / multipart/related)** — defer; meaningful MIME complexity.
- [ ] **Drag-and-drop email builder** — explicitly an anti-feature for this product.

## Activation Carry-Over Semantics (concrete spec for downstream)

Highest-edge-case sub-feature. Behavior the board expects:

- **Copy, not move.** Application retains its attachment; activation creates an **independent** MemberDocument (new `id`, new `relative_path = {new_doc_id}.{ext}`, file bytes copied via `DocumentStorage::load` then `save`). Rationale: audit integrity + re-display on Application; disk cost negligible.
- **Same transaction.** The copy + `audited_create!(MemberDocument)` happen inside the existing single activation tx under `APPLICATION_SERVICE_PROCESS`, so the hash-chain ties "Application bestätigt" and "MemberDocument angelegt" together. If the file copy fails, the whole activation rolls back (no half-activated member).
- **Naming / typing.** MemberDocument `document_type` = a stable value like `"membership_application"` (Antragsoriginal); `description` e.g. "Originaler Mitgliedsantrag"; `file_name` carried from the upload. Recognizable in the existing MemberDocument list.
- **Audit visibility.** Via `audited_create!`, the new MemberDocument appears in the audit log with the activating user as actor — consistent with how Member/Eintritt/Aufstockung are already logged in the same flow.

### Edge cases (must be specified in requirements)

- **Activation with NO attachment** — must succeed normally; carry-over skipped silently (common case for existing applications). No error, no empty MemberDocument.
- **Re-activation / non-Offen status** — already guarded: activate returns `Conflict` unless status is `Offen`. Double-carry-over cannot happen via the normal path; no duplicate MemberDocument.
- **File missing on disk at activation time** (DB row points to a deleted file) — decide: fail activation (safest, atomic) vs. activate + warn. Recommend fail-and-roll-back so the board isn't silently missing the document; surface a clear error.
- **Replacing the Application attachment before activation** — uploading a second file should overwrite/supersede the first (single-file model), not accumulate orphans. Define overwrite semantics.
- **Application rejected (`Abgelehnt`)** — no member, no carry-over; attachment stays only on the Application.
- **Large file / wrong type** — enforce size limit + MIME/extension allow-list (PDF, common image types) at upload, mirroring/extending MemberDocument upload validation. Scanned PDF is the primary case.
- **Soft-delete interaction** — MemberDocument carries standard `deleted`/`version` fields; carry-over creates a live (non-deleted) doc.

## WYSIWYG: minimal viable spec for non-technical board users

- **Toolbar (exactly the named set, nothing more):** Bold, Italic, Bullet list, Numbered list, Insert link. Optionally a paragraph/heading toggle and "remove formatting". Resist font/color/size — bloats output and breaks mail rendering.
- **Implementation lean:** thin Dioxus `contenteditable` wrapper as a single reusable component (replacing `body_editor.rs`), driving the ~5 commands via `web-sys`/Selection (or `execCommand` as a pragmatic baseline). Avoid bundling a heavy JS editor (anti-feature).
- **Link UX:** select text → click link → prompt for URL → auto-prefix `https://` if scheme missing → validate. Show links visibly styled in the editor.
- **Paste handling:** intercept paste, strip styles/classes/`mso-*`/font tags, keep text + whitelisted formatting. Smart-quote normalization optional.
- **Output contract:** sanitized HTML limited to `{b/strong, i/em, a[href], ul, ol, li, p, br, h2, h3}`. Sanitize **server-side too** — never trust frontend output for the mail body.
- **Fallback derivation:** same sanitized HTML is the single source; plain text derived from it server-side.

## Feature Prioritization Matrix

| Feature | User Value | Implementation Cost | Priority |
|---------|------------|---------------------|----------|
| 8bit encoding | MEDIUM (direct complaint) | LOW | P1 |
| multipart/alternative + text-derive | HIGH | MEDIUM | P1 |
| Template vars in HTML (autoescape-correct) | HIGH | MEDIUM | P1 |
| WYSIWYG editor (bold/italic/list/link) | HIGH | MEDIUM–HIGH | P1 |
| Output sanitization | HIGH (safety) | MEDIUM | P1 |
| Application single-file upload | HIGH | LOW–MEDIUM | P1 |
| Activation carry-over (copy→MemberDocument) | HIGH | MEDIUM | P1 |
| Attachment view/download (App + Member) | MEDIUM | LOW | P1 |
| Paste-from-Word cleanup | HIGH (this user group) | MEDIUM | P1/P2 |
| Saved HTML templates | MEDIUM | MEDIUM | P2 |
| HTML inbox digest | LOW | LOW | P2 |
| Multiple files per Application | LOW | LOW | P3 |
| Branding/letterhead + CID images | MEDIUM | HIGH | P3 |
| Drag-and-drop email builder | LOW | HIGH | Never (anti-feature) |

## Complexity & risk callouts for the roadmap

- **Highest risk: minijinja autoescape in HTML templates.** Escaping member-provided values (names with `&`/`<`) while preserving author markup is the easiest place to ship an XSS-y or visually-broken mail. Flag for deeper attention.
- **Second risk: WYSIWYG output stability** across the contenteditable lifecycle in Dioxus/WASM (cursor jumps, re-render clobbering the DOM — the project already has a Dioxus button-reload-bug lesson; expect similar `contenteditable` quirks). Component-First mandate applies.
- **8bit deliverability:** confirm the production SMTP relay advertises `8BITMIME` before flipping the default; otherwise gate it.
- **Everything else is mostly extension of proven code paths** (lettre multipart, DocumentStorage, audited_create!, existing upload + activation flows), which is why most items are LOW/MEDIUM rather than HIGH.

## Sources

- Genossi codebase (verified 2026-06-29): `genossi_mail/src/worker.rs` (lettre MultiPart/SinglePart, quoted-printable test), `genossi_mail/src/digest.rs`, `genossi_service_impl/src/application.rs` (activation single-tx), `genossi_service_impl/src/document_storage.rs`, `genossi_service_impl/src/member_document.rs`, `genossi_dao/src/member_document.rs`, `genossi-frontend/src/component/mail_compose/*` (incl. current `body_editor.rs` textarea).
- `.planning/todos/pending/2026-06-28-html-mail-support-statt-nur-textmails.md`, `.planning/todos/pending/2026-06-27-originalen-mitgliedsantrag-als-datei-attachment-an-applicati.md`
- `.planning/PROJECT.md` (milestone v1.4 definition, existing-feature inventory, Component-First + audit constraints)
- Standard email/MIME + WYSIWYG-for-non-technical-users domain knowledge (multipart/alternative, 8BITMIME, paste sanitization).

---
*Feature research for: admin mail formatting + application document attachment (Genossi v1.4)*
*Researched: 2026-06-29*
