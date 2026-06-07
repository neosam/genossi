---
phase: 19-e-mail-anhaenge-anzeigen
plan: 19-06
slug: frontend-page-wiring
type: execute
wave: 5
depends_on: [19-05]
files_modified:
  - genossi-frontend/src/page/inbox_page.rs
autonomous: false
requirements: []
must_haves:
  truths:
    - "Existing MVP-amber-hint block in `inbox_page.rs:331-335` is deleted (D-11)"
    - "`InboxAttachmentList` component invoked exactly once after the body `<pre>` and BEFORE the assignment `border-t` divider (D-11, D-13)"
    - "Component receives `mail_id`, `attachments`, and `has_legacy_attachments` props correctly derived from `InboundMailDetailTO`"
    - "Page-file contains NO inline RSX iteration over `attachments` — pure delegation to the component (Component-First, D-13)"
    - "WASM build (`dx build` / `cargo check --target wasm32-unknown-unknown`) succeeds"
    - "Manual smoke test (Vorstand-Login → Inbox → Mail mit Attachment auswählen) shows attachments list with Download + Preview actions per UI-SPEC §Action Matrix"
  artifacts:
    - path: "genossi-frontend/src/page/inbox_page.rs"
      provides: "Page composes InboxAttachmentList; MVP-hint removed"
      contains: "InboxAttachmentList"
  key_links:
    - from: "inbox_page.rs"
      to: "InboxAttachmentList component"
      via: "Single RSX call between body <pre> and assignment <div>"
      pattern: "InboxAttachmentList \\{"

---

<objective>
Schalte die Phase-19-Anhang-Anzeige im Detail-Pane scharf: lösche den
"nicht anzeigbar im MVP"-Hinweis und ersetze ihn durch einen einzigen
`InboxAttachmentList`-Component-Aufruf. Page-Wiring ONLY — keine eigene
RSX-Logik im Page-File.

Purpose: Vollendet den Phase-19-UI-Pfad. Vorstand sieht ab Server-Restart
(Backfill aus Plan 19-04 läuft) sofort die echten Anhänge mit Download/Preview.

Output: Genau eine modifizierte Datei (`inbox_page.rs`), ein Checkpoint zur
manuellen Sichtprüfung in echtem `dx serve`-Dev-Setup.
</objective>

<execution_context>
@/home/neosam/programming/rust/projects/genossi3/.claude/get-shit-done/workflows/execute-plan.md
@/home/neosam/programming/rust/projects/genossi3/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/PROJECT.md
@.planning/ROADMAP.md
@.planning/STATE.md
@.planning/phases/19-e-mail-anh-nge-anzeigen-backend-endpoint-zum-abrufen-von-anh/19-CONTEXT.md
@.planning/phases/19-e-mail-anh-nge-anzeigen-backend-endpoint-zum-abrufen-von-anh/19-RESEARCH.md
@.planning/phases/19-e-mail-anh-nge-anzeigen-backend-endpoint-zum-abrufen-von-anh/19-PATTERNS.md
@.planning/phases/19-e-mail-anh-nge-anzeigen-backend-endpoint-zum-abrufen-von-anh/19-UI-SPEC.md
@.planning/phases/19-e-mail-anh-nge-anzeigen-backend-endpoint-zum-abrufen-von-anh/19-05-SUMMARY.md
@genossi-frontend/CLAUDE.md

<interfaces>
<!-- Pre-extracted analog. -->

From `genossi-frontend/src/page/inbox_page.rs:295-360` (detail-pane structure):
- The `<pre>` body element ends at around `:347` with `"{d.body_text}"`
- The assignment section divider `div { class: "border-t pt-2 mt-2" }` starts at `:350`
- The MVP-hint block to DELETE is at `:331-335`:
  ```rust
  if d.has_attachments {
      div { class: "text-xs text-amber-700",
          "📎 Diese Mail enthält Anhänge (nicht anzeigbar im MVP)"
      }
  }
  ```

From Plan 19-05 (already shipped):
- Component imported via `use crate::component::inbox::InboxAttachmentList;` (verify: existing inbox page may already have a `use crate::component::inbox::...` block; add to it)
- Props: `mail_id: String`, `attachments: Vec<InboundMailAttachmentTO>`, `has_legacy_attachments: bool`

From Plan 19-05 Task 1 (already shipped):
- `InboundMailDetailTO` has `pub attachments: Vec<InboundMailAttachmentTO>` field (Plan 19-05 Step 7 added this)
</interfaces>
</context>

<tasks>

<task type="auto">
  <name>Task 1: Delete MVP-hint + insert InboxAttachmentList call</name>
  <files>genossi-frontend/src/page/inbox_page.rs</files>
  <read_first>
    - genossi-frontend/src/page/inbox_page.rs:1-50 (top-of-file imports — check what `use crate::component::inbox::...` already imports)
    - genossi-frontend/src/page/inbox_page.rs:295-360 (detail-pane block — surround lines 331-335 + verify <pre> body location around :347 + assignment border-t at :350)
    - .planning/phases/19-e-mail-anh-nge-anzeigen-backend-endpoint-zum-abrufen-von-anh/19-UI-SPEC.md §Page Integration (table comparing Before/After)
    - .planning/phases/19-e-mail-anh-nge-anzeigen-backend-endpoint-zum-abrufen-von-anh/19-PATTERNS.md §13
  </read_first>
  <behavior>
    - Remove the exact 5-line MVP-hint block at `:331-335`
    - Add `use crate::component::inbox::InboxAttachmentList;` to the existing import block (if `crate::component::inbox` is already wildcard-imported via `use crate::component::inbox::*;`, no new use-line is needed — verify)
    - Insert ONE component call between the `<pre>` body (ends `:347`) and the assignment `border-t` divider (starts `:350`):
      ```rust
      InboxAttachmentList {
          mail_id: d.id.clone(),
          attachments: d.attachments.clone(),
          has_legacy_attachments: d.attachments.is_empty() && d.has_attachments,
      }
      ```
    - NO inline `for` loop over attachments anywhere in `inbox_page.rs`
    - NO inline `<img>`, `<li>`, `<a download>` for attachments anywhere in `inbox_page.rs`
    - WASM build succeeds
  </behavior>
  <action>
    **Step 1 — Read & confirm exact line numbers** (line numbers in the planning docs are 2026-06-07 snapshots; current file may have shifted slightly). Use the textual markers to find the right block:

    1. Find the MVP-hint block by grepping for `"nicht anzeigbar im MVP"`:
       ```bash
       grep -n "nicht anzeigbar im MVP" genossi-frontend/src/page/inbox_page.rs
       ```
    2. Find the body `<pre>` block (it likely contains `"{d.body_text}"`):
       ```bash
       grep -n '"{d.body_text}"' genossi-frontend/src/page/inbox_page.rs
       ```
    3. Find the assignment `border-t` divider that comes immediately after the body:
       ```bash
       grep -n 'class: "border-t' genossi-frontend/src/page/inbox_page.rs
       ```

    **Step 2 — Add import** at the top of `inbox_page.rs`. If a `use crate::component::inbox::{…, …};` already lists components, add `InboxAttachmentList` to that list. Otherwise add a new line:
    ```rust
    use crate::component::inbox::InboxAttachmentList;
    ```

    **Step 3 — Delete the MVP-hint block** (currently `:331-335`):
    ```rust
    if d.has_attachments {
        div { class: "text-xs text-amber-700",
            "📎 Diese Mail enthält Anhänge (nicht anzeigbar im MVP)"
        }
    }
    ```
    The whole 5-line `if` block must be removed.

    **Step 4 — Insert the component invocation** between the `<pre>` closing brace (after `"{d.body_text}"`) and the assignment-section `div { class: "border-t …" }`:
    ```rust
    InboxAttachmentList {
        mail_id: d.id.clone(),
        attachments: d.attachments.clone(),
        has_legacy_attachments: d.attachments.is_empty() && d.has_attachments,
    }
    ```

    The `has_legacy_attachments` expression is critical: it triggers the "legacy hint" branch in the component only when the backend reports `has_attachments=true` but no attachment rows exist (D-06 — backfill couldn't recover bytes).

    **Step 5 — Verify WASM build** with:
    ```bash
    cargo check -p genossi-frontend --target wasm32-unknown-unknown
    ```

    Anti-pattern reminder: do NOT add inline RSX iteration over `d.attachments` here. The Page just delegates to the component (Component-First, D-13 + `feedback_component_first.md`).
  </action>
  <verify>
    <automated>cargo check -p genossi-frontend --target wasm32-unknown-unknown 2>&amp;1 | tee /tmp/19-06-task1-check.log; ! grep -q "^error" /tmp/19-06-task1-check.log &amp;&amp; grep -q "InboxAttachmentList" genossi-frontend/src/page/inbox_page.rs &amp;&amp; ! grep -q "nicht anzeigbar im MVP" genossi-frontend/src/page/inbox_page.rs</automated>
  </verify>
  <acceptance_criteria>
    - `grep -c "nicht anzeigbar im MVP" genossi-frontend/src/page/inbox_page.rs` returns 0 (MVP hint deleted)
    - `grep -c "InboxAttachmentList" genossi-frontend/src/page/inbox_page.rs` returns ≥ 2 (1 use-import + 1 invocation, or 1 invocation if wildcard-imported)
    - `grep -c "mail_id: d.id.clone()" genossi-frontend/src/page/inbox_page.rs` returns ≥ 1 (component prop)
    - `grep -c "has_legacy_attachments" genossi-frontend/src/page/inbox_page.rs` returns ≥ 1
    - `grep -c "d.attachments.is_empty() && d.has_attachments" genossi-frontend/src/page/inbox_page.rs` returns ≥ 1 (the legacy expression)
    - `grep -c "for .* in .*attachments" genossi-frontend/src/page/inbox_page.rs` returns 0 (Component-First — NO inline iteration)
    - `grep -c "<img\|img {" genossi-frontend/src/page/inbox_page.rs | grep -v "0" >/dev/null && false || true` (no inline img tags for attachments — if file has other `img` for unrelated reasons, that's OK; this check is a non-blocking signal)
    - `cargo check -p genossi-frontend --target wasm32-unknown-unknown` exits 0
  </acceptance_criteria>
  <done>
    `inbox_page.rs` ist Component-First: ein einziger Component-Call, MVP-Hinweis weg, WASM-Compile grün.
  </done>
</task>

<task type="checkpoint:human-verify" gate="blocking">
  <name>Task 2: Manuelle Sichtprüfung im Dev-Server</name>
  <what-built>
    Phase 19 ist nun End-to-End fertig: Migration angelegt, Persistenz im Poll-Worker
    aktiv, Download-Endpoint live, Backfill-Worker startet beim Server-Boot, Frontend
    rendert die Anhänge-Liste mit Download + Preview im Detail-Pane.
  </what-built>
  <how-to-verify>
    Setup:
    1. Sicherstellen, dass eine echte Inbox mit Mails konfiguriert ist (`imap_*` config keys gesetzt) — falls nicht, manuell eine `inbound_mails`-Row + `inbound_mail_attachments`-Rows via SQL seeden:
       ```sql
       INSERT INTO inbound_mail_attachments (id, inbound_mail_id, created, file_name, mime_type, size_bytes, relative_path, oversized)
       VALUES (randomblob(16), <existing_mail_id_blob>, '2026-06-07T12:00:00.000Z', 'rechnung.pdf', 'application/pdf', 12345, 'inbound_mail_attachments/<mid>/<aid>', 0);
       ```
       Datei manuell unter `$DOCUMENT_STORAGE_PATH/inbound_mail_attachments/<mid>/<aid>` ablegen (z.B. eine kleine PDF/PNG).

    Backend starten:
    ```bash
    DATABASE_URL=sqlite:genossi.db cargo run --bin genossi
    ```
    Im Log sollte erscheinen:
    - `inbox_attachment_backfill: starting (N candidates)` (mind. 0)
    - `inbox_attachment_backfill: done (Y persisted, Z skipped)`

    Frontend starten:
    ```bash
    cd genossi-frontend && npx tailwindcss -i ./input.css -o ./assets/tailwind.css --watch &
    dx serve --hot-reload
    ```

    Im Browser:
    1. Vorstand-Login via OIDC (oder mock-auth feature-flag).
    2. Inbox aufrufen.
    3. Eine Mail mit Attachment auswählen.
    4. Im Detail-Pane unter dem Body sollte erscheinen:
       - Section-Header `📎 Anhänge (N)` in `text-sm font-semibold`
       - Pro Attachment eine `<li>`-Zeile mit Filename + Größe + MIME-Label
       - Bei `image/*`: Thumbnail `<img>` (max-h-24) klickbar → öffnet groß in neuem Tab
       - Bei `application/pdf`: Download-Button (blau) + zweiter `Vorschau`-Link
       - Bei anderen MIME: nur Download-Button
    5. Klick auf `Herunterladen` → Browser startet nativen Download mit korrektem Filename (Content-Disposition: attachment).
    6. Klick auf `Vorschau` bei PDF → öffnet im neuen Tab inline (Content-Disposition: inline).
    7. Filename-Edge-Case: falls eine Mail Anhänge mit Umlauten im Filename hat (z.B. `Rückzahlung.pdf`) → Download-Filename im Browser zeigt UTF-8 korrekt.

    Empty/Legacy-Case verifizieren:
    - In der DB: `UPDATE inbound_mails SET has_attachments = 1 WHERE id = <mail_ohne_attachment_rows>;` (Anzeige zeigt amber Hinweis `"Anhang vor Phase 19 empfangen — bitte im Mail-Client öffnen"`).

    Oversized-Case verifizieren:
    - In der DB: `INSERT INTO inbound_mail_attachments (id, inbound_mail_id, created, file_name, mime_type, size_bytes, relative_path, oversized) VALUES (randomblob(16), <mid>, …, 'big.zip', 'application/zip', 12345678, NULL, 1);`
    - Im Frontend: Zeile zeigt amber Hinweis `"Zu groß — bitte im Mail-Client öffnen"` ohne Download-Button.

    Browser Devtools:
    - Network-Tab: Download-Request liefert 200 + Content-Type passend + Content-Disposition korrekt.
    - Cross-Mail-IDOR: manuell `https://localhost:3000/api/inbox/<falsche_mail_id>/attachments/<attachment_id>` → 404 (oder im Browser sichtbar als 404).
  </how-to-verify>
  <resume-signal>
    Tippe `approved` wenn alles funktioniert. Bei Befunden:
    - "filename umlaut broken" → Backend http_util-Encoding nochmal prüfen
    - "preview not opening" → Browser PDF-MIME-Handler prüfen, Backend Content-Type
    - "oversized still shows download button" → Component-Branch prüfen (Plan 19-05 Task 2)
    - "legacy hint never appears" → has_legacy_attachments-Expression im inbox_page prüfen
    - "404 on every download in dev" → Pitfall 8 (Cookie-Forwarding bei Cross-Origin); Dioxus.toml-Proxy-Setup verifizieren
  </resume-signal>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| Page → Component | Detail-TO already validated by Plan 19-03 backend; page just clones into props |

## STRIDE Threat Register

| Threat ID | Category | Component | Disposition | Mitigation Plan |
|-----------|----------|-----------|-------------|-----------------|
| (none new) | — | — | — | All threats already mitigated in Plans 19-01 through 19-05; this plan adds zero new code paths beyond the single component invocation. |

(All HIGH threats T-01..T-04, T-06 + MEDIUM T-05, T-07, LOW T-08 are mitigated in the upstream plans.)
</threat_model>

<verification>
- `cargo check -p genossi-frontend --target wasm32-unknown-unknown` exits 0
- MVP hint string `"nicht anzeigbar im MVP"` removed from `inbox_page.rs`
- `InboxAttachmentList` invocation present with all three props
- NO inline RSX iteration over `attachments` in `inbox_page.rs` (Component-First gate)
- Manual smoke test signed off by Vorstand (Task 2 checkpoint)
</verification>

<success_criteria>
- Phase 19 end-to-end complete: backend persists, REST serves, backfill recovers legacy, frontend renders
- Vorstand can download + preview attachments in echtem Dev-Build
- Component-First wahrt: 0 RSX-Iterations im Page-File
- Manuelle Sichtprüfung dokumentiert (UI-SPEC §Checker Sign-Off Dimensions PASS)
</success_criteria>

<output>
After completion, create `.planning/phases/19-e-mail-anh-nge-anzeigen-backend-endpoint-zum-abrufen-von-anh/19-06-SUMMARY.md`
</output>
