# Phase 19: E-Mail-Anhänge anzeigen - Context

**Gathered:** 2026-06-07
**Status:** Ready for planning

<domain>
## Phase Boundary

Eingehende E-Mail-Anhänge im Vorstands-Inbox sichtbar und herunterladbar machen.

**Konkret:**
- **Backend:** IMAP-Polling-Worker erweitert um Attachment-Persistenz; neue Entität `InboundMailAttachment` (DAO + Service + REST); Download-Endpoint mit optionalem Inline-Modus; einmaliger Backfill-Worker für Bestandsmails (best-effort via IMAP-Refetch).
- **Frontend:** Neue Components `InboxAttachmentList` + `InboxAttachmentListItem` unter `genossi-frontend/src/component/inbox/`; Detail-Pane-Section unter Body-Text mit Filename + Größe + Download/Preview; Inline-Preview für `image/*` und `application/pdf`; Hinweis bei nicht-recoverbaren Bestands-Mails.
- **Nicht in Scope:** Outbound-Attachment-Anzeige (existiert bereits via `MailRecipientAttachment`); Reply mit Attachment; HTML-Mail-Rendering generell; Volltext-Suche in Attachments; Virenscan; Reply-with-Forward.

</domain>

<decisions>
## Implementation Decisions

### Persistenz-Strategie
- **D-01:** Attachments werden **beim IMAP-Polling persistent** in `DocumentStorage` (Filesystem) gespeichert. Pattern analog zu outbound `MailRecipientAttachment`: neue Entität `InboundMailAttachment { id, inbound_mail_id, file_name, mime_type, size_bytes, relative_path, oversized }`. Worker parst Attachments via `mail-parser`'s `msg.attachments()`, ruft `storage.save(relative_path, bytes)`, schreibt DB-Row. Begründung: schneller Detail-View, IMAP-unabhängig, etabliertes Pattern; verwirft den aktuellen `attachment_count()`-only-MVP-Pfad (`genossi_mail/src/inbox.rs:162`).
- **D-02:** **Hard-Limit 10 MB pro Attachment.** Attachments > 10 MB werden nicht gespeichert — Metadaten-Row trotzdem angelegt mit `oversized=true`, `relative_path=NULL`. Frontend zeigt „Zu groß — bitte direkt im Mail-Client öffnen". Schutz gegen Disk-DoS via Spam-Attachments. Nicht konfigurierbar (Konstante); Default-Wert in `inbox.rs` neben anderen Limits.
- **D-03:** **Keine MIME-Type-Whitelist.** Alle Anhänge werden gespeichert. 10-MB-Limit + Vorstand-only-Permission reichen als Schutz. MIME-Type aus `mail-parser` wird beim Download als `Content-Type` zurückgegeben.
- **D-04:** **Storage-Pfad-Schema:** `inbound_mail_attachments/{inbound_mail_id}/{attachment_id}` (analog zu `static_documents/{id}` in `StaticDocument::relative_path()`). Pro Mail eigenes Verzeichnis erleichtert Cleanup (zukünftiger Soft-Delete der Mail).

### Backfill für Bestandsmails
- **D-05:** **Automatischer Backfill beim Server-Start, einmalig.** Beim Start nach der Migration: Hintergrund-Worker iteriert alle `InboundMail` mit `has_attachments=true` und keinen `InboundMailAttachment`-Rows; ruft IMAP-Refetch via `uid_validity`+`imap_uid`, parst, persistiert. Server bleibt voll funktional während Backfill läuft (Tokio-spawn). Tracing-Log am Start: `inbox_attachment_backfill: starting (N candidates)`.
- **D-06:** **Silent skip bei Backfill-Fehlern.** UID-Validity-Drift, gelöschte Mail im IMAP, Verbindungsfehler → `tracing::warn!`, weiter mit nächster Mail. Kein State-Tracking, keine Retry-Logik. Frontend zeigt für solche Mails: „Anhang nicht mehr verfügbar (im Mailserver gelöscht oder vor Phase 19 empfangen)" — gekoppelt an `has_attachments=true && attachments.is_empty()`.

### Endpoint-Design
- **D-07:** **Attachment-Liste embedded in `InboundMailDetailTO`** als Feld `attachments: Vec<InboundMailAttachmentTO>` mit `{ id, file_name, mime_type, size_bytes, oversized }`. Spart Round-Trip im Frontend beim Öffnen der Mail-Detailansicht. `has_attachments`-Flag bleibt bestehen (gesetzt aus `attachments.len() > 0 || legacy_flag`).
- **D-08:** **Download-Endpoint:** `GET /api/inbox/{mail_id}/attachments/{attachment_id}` — optionaler Query-Param `?disposition=inline` schaltet von `Content-Disposition: attachment; filename="…"` auf `inline; filename="…"` um. Default = attachment. Ein Endpoint, beide Modi. Content-Type aus DB-`mime_type`. Body via `DocumentStorage::load`.
- **D-09:** **Permission analog zum bestehenden `GET /api/inbox/{id}`** — Vorstand-only via gleicher Auth-Pfad wie alle anderen Inbox-Endpoints. Keine neue Permission-Granularität.
- **D-10:** **Kein Audit-Log für Attachment-Read/Download.** Konsistent mit existierendem Inbox-Pattern: `InboundMail` ist nicht auditiert (anders als Member/MemberDocument/Application). Read-Only-Daten brauchen keinen Hashchain-Eintrag. `InboundMailAttachment` implementiert daher **kein** `Auditable`-Trait.

### Frontend-UX
- **D-11:** **Section unter Body-Text** im Detail-Pane (`inbox_page.rs`): Header `📎 Anhänge ({n})`, dann Liste — MIME-Icon | Filename | formatierte Größe (`12 KB` / `1.4 MB`) | Download-Button. Oversized-Rows zeigen Größe + „zu groß" statt Download-Button. Section nach `pre`-Body, vor Assignment-Section. Den bestehenden Frontend-Hinweis „nicht anzeigbar im MVP" (`inbox_page.rs:333`) ersetzen.
- **D-12:** **Inline-Preview nur für `image/*` und `application/pdf`.** Frontend-Item rendert je nach `mime_type`:
  - `image/*` → `<img src="…?disposition=inline" />` als Mini-Preview-Thumbnail (klickbar → öffnet groß im neuen Tab).
  - `application/pdf` → Download-Button + zusätzlicher „Vorschau"-Button, der `<embed>` oder neuen Tab triggert (entscheidet Planner).
  - Alle anderen MIME-Types → nur Download-Button.
- **D-13:** **Component-Extraction:** Neue Components unter `genossi-frontend/src/component/inbox/`:
  - `attachment_list.rs` — `InboxAttachmentList { mail_id, attachments }` rendert Section + Header + iteriert.
  - `attachment_list_item.rs` — `InboxAttachmentListItem { mail_id, attachment }` rendert eine Zeile inkl. Preview-Logik.
  Beide registriert in `component/inbox/mod.rs`. `inbox_page.rs` bekommt nur Component-Aufruf, keine Inline-RSX-Liste (Component-First-Prinzip, siehe `feedback_component_first.md`).
- **D-14:** **i18n-Keys:** Neue Keys in `i18n/mod.rs`-Enum + Übersetzungen in De/En (zwei Locales, siehe `genossi-frontend/CLAUDE.md`):
  - `inbox.attachments.header` → „Anhänge" / „Attachments"
  - `inbox.attachments.empty_legacy` → „Anhang vor Phase 19 empfangen — bitte im Mail-Client öffnen" / „Attachment received before Phase 19 — open in your mail client"
  - `inbox.attachments.oversized` → „Zu groß ({size}) — bitte im Mail-Client öffnen" / „Too large ({size}) — open in your mail client"
  - `inbox.attachments.download` → „Herunterladen" / „Download"
  - `inbox.attachments.preview` → „Vorschau" / „Preview"

### Claude's Discretion
- DB-Schema-Details (NOT-NULL-Constraints, Index-Strategien) und SQL-Migration-Filename überlässt Discussion dem Planner — Pattern existiert bereits in `dao_sqlite.rs:1130-1175` für `mail_recipient_attachments`.
- Genaue UI-Größen, Icons, Hover-States, Mobile-Layout: Planner/Executor-Detail; Component-First-Prinzip muss eingehalten werden.
- Test-Strategie (Unit/E2E-Tests für Worker, REST, Frontend) entscheidet Planner — bestehende E2E-Test-Patterns (`genossi_bin/tests/e2e_tests.rs`) gelten.
- Konkrete Filename-Sanitization beim Content-Disposition-Header (UTF-8-Encoding, Quotes, Path-Traversal-Schutz) — siehe vorhandenes `crate::http_util::content_disposition_attachment` (`genossi_rest/src/member_document.rs:256`).

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Bestehende Mail-Subsystem-Referenzen (Pflichtlektüre)
- `genossi_mail/src/inbox.rs` — `parse_raw_mail()`, `FetchedMessage`, `ParsedMail`, `InboxService`-Trait; **Zeile 162** dokumentiert den MVP-Verzicht („count attachments (bool only, contents discarded)"), den Phase 19 aufhebt.
- `genossi_mail/src/inbox_imap.rs` — Echte IMAP-Client-Impl via `async-imap`+`tokio-rustls`; `open_examine_session`, `fetch_uids_since`. Wiederverwendet beim Backfill (UID-basierter Refetch).
- `genossi_mail/src/dao.rs` §`InboundMail` (Zeile 222-242) — Bestehende `inbound_mails`-Entität inkl. `uid_validity`, `imap_uid` (für Backfill-Lookup). Pattern für neue `InboundMailAttachment` analog §`MailRecipientAttachment` (Zeile 88-105).
- `genossi_mail/src/dao_sqlite.rs` §`MailRecipientAttachmentDaoSqlite` (Zeile 359-432) — Vorbild-Implementation für SQLite-DAO; **Zeile 1130-1175** zeigt CREATE-TABLE-Pattern.
- `genossi_mail/src/inbox_rest.rs` — `InboundMailTO`/`InboundMailDetailTO`, `InboxRestState`-Trait, OpenAPI-Doc-Setup; hier erweitert um `attachments`-Feld + Download-Handler.
- `genossi_mail/src/service.rs` §`MailServiceError` — Error-Mapping-Pattern.

### File-Storage-Pattern (Pflichtlektüre)
- `genossi_service/src/document_storage.rs` — `DocumentStorage`-Trait (load/save/delete), `StorageError`-Enum, `MockDocumentStorage`.
- `genossi_service_impl/src/document_storage.rs` — `FilesystemDocumentStorage`-Impl.
- `genossi_rest/src/member_document.rs` Zeile 232-267 — **Download-Handler-Vorbild**: `Content-Type`+`Content-Disposition`-Setup via `crate::http_util::content_disposition_attachment`.
- `genossi_rest/src/http_util.rs` — `content_disposition_attachment()` Helper (Filename-Encoding).
- `genossi_mail/src/static_document_service.rs` — Upload-Pattern (atomisches save+DB mit Rollback-Pfad).

### Frontend-Inbox-Referenzen (Pflichtlektüre)
- `genossi-frontend/src/page/inbox_page.rs` Zeile 270-340 — Bestehender Detail-Pane-Aufbau; **Zeile 331-335** zeigt aktuellen „nicht anzeigbar im MVP"-Hinweis, der ersetzt werden muss.
- `genossi-frontend/src/component/inbox/mail_list_item.rs` — Bestehender Component-Stil (Props, Styling, Tailwind-Klassen). Neue Attachment-Components folgen demselben Stil.
- `genossi-frontend/src/component/inbox/mod.rs` — Registry für neue Components.
- `genossi-frontend/CLAUDE.md` §Component-First Principle — Verbindlich: keine Inline-RSX-Duplikate.
- `genossi-frontend/src/i18n/mod.rs`, `de.rs`, `en.rs` — Zwei Locales, beide Pflicht (siehe `genossi-frontend/CLAUDE.md` §i18n).

### Projekt-Konventionen (Pflichtlektüre)
- `CLAUDE.md` §Architecture Overview — Layered-DAO/Service/REST.
- `.planning/codebase/ARCHITECTURE.md`, `.planning/codebase/CONVENTIONS.md` — Aktive Naming/Code-Patterns.
- `.planning/PROJECT.md` — Constraints für Tech-Stack + Audit-Pflicht (`InboundMailAttachment` **nicht** auditiert — siehe D-10 + `.planning/PROJECT.md` Audit-Sektion: nur Member/MemberAction/MemberDocument/Application).
- Migrations: `migrations/sqlite/` — Bestehendes Format: `YYYYMMDDHHMMSS_description.sql`; neue Migration analog `20260403000004_create_mail_recipient_attachments_table.sql`.

### Aus Memory (gelernte Lektionen)
- `~/.claude/projects/.../memory/feedback_component_first.md` — Component-First-Prinzip durchsetzen (D-13).
- `~/.claude/projects/.../memory/feedback_use_jj_not_git.md` — Projekt nutzt `jj` statt `git` für Commits.
- `~/.claude/projects/.../memory/feedback_dioxus_button_type.md` — Bei neuen Buttons mit onclick: `r#type: "button"` setzen, sonst Page-Reload.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- **`mail-parser` `Message::attachments()`** — bereits in `parse_raw_mail()`-Pfad genutzt (Zeile 208 ruft nur `attachment_count()`). Phase 19 erweitert die Funktion: iteriert `msg.attachments()`, baut neuen `ParsedAttachment { file_name, mime_type, size_bytes, bytes }`-Vec, gibt ihn in `ParsedMail` zurück.
- **`DocumentStorage::save(relative_path, &[u8])`** — Atomisches Persistieren. Pattern: zuerst Storage-save, dann DB-create (mit Rollback bei DB-Fail), siehe `static_document_service.rs:210-260`.
- **`http_util::content_disposition_attachment(file_name)`** — Header-Helper inkl. UTF-8-Encoding und Filename-Sanitization. Für Inline-Modus müssten wir einen analogen `content_disposition_inline(file_name)` ergänzen.
- **`InboxImapClient::fetch_uids_since` / `open_examine_session`** — Backfill ruft pro Mail eine separate IMAP-Session auf — funktional vorhanden, evtl. neue Method `fetch_one_by_uid(uid_validity, imap_uid) → FetchedMessage` ergänzen (kleinerer Scope).

### Established Patterns
- **DAO-Trait-Pattern:** `InboundMailAttachmentDao` mit `create`, `find_by_inbound_mail_id` (analog `MailRecipientAttachmentDao`); SQLite-Impl in `dao_sqlite.rs` mit `TryFrom<&InboundMailAttachmentDb>` für Uuid-BLOB-Konvertierung.
- **REST-Handler-Pattern:** `axum::extract`-State+Path; Error-Mapping via `map_error`; OpenAPI-Doc-Macros (`#[utoipa::path]`).
- **TO-Konvertierung:** `to_detail_to()` in `inbox_rest.rs` wird um `attachments: Vec<InboundMailAttachmentTO>` erweitert; eigene `to_attachment_to(&entity)`-Free-Function dazu.
- **Component-First Frontend:** Components in `src/component/inbox/`, Pages nutzen nur Component-Aufrufe.
- **Worker-Pattern:** Existierender IMAP-Poller läuft als Tokio-Task (siehe `genossi_bin/src/lib.rs`); Backfill-Worker folgt demselben Spawn-Pattern.

### Integration Points
- **`InboxService::poll`** (oder äquivalenter Worker-Entry in `inbox.rs`/`worker.rs`) — wird erweitert: nach `parse_raw_mail` + Mail-create folgt Attachment-Persistenz-Schleife mit 10-MB-Limit-Check.
- **`InboxRestState`-Trait** — bekommt Zugriff auf `InboundMailAttachmentDao` + `DocumentStorage` (vermutlich über bestehende `RestStateDef::document_storage()`).
- **`RestStateImpl`-DI in `genossi_bin/src/lib.rs`** — neuer DAO + Service-Wire; Backfill-Worker spawnen.
- **Frontend `api.rs`** — neue Helper `fetch_attachment_download_url(mail_id, attachment_id)` + `fetch_attachment_inline_url(mail_id, attachment_id)`.
- **Bestehender `has_attachments`-Flag in `InboundMail`** — bleibt erhalten (Backward-Kompatibilität + Inbox-Liste zeigt 📎-Icon). Bedeutung leicht angepasst: „Mail enthielt mind. 1 Attachment beim Empfang". `attachments`-Feld in `DetailTO` ist autoritativ für UI-Listendarstellung.

</code_context>

<specifics>
## Specific Ideas

- Inbox-Detail-Mockup vom Nutzer bestätigt (siehe Preview im DISCUSSION-LOG): Section direkt unter Body, vor Assignment.
- Endpoint-Schema vom Nutzer bestätigt:
  - `GET /api/inbox/{mail_id}` → DetailTO inkl. `attachments: [{ id, file_name, mime_type, size_bytes, oversized }]`
  - `GET /api/inbox/{mail_id}/attachments/{attachment_id}` → Bytes mit Content-Disposition: attachment (default) oder `?disposition=inline`
- Component-Layout-Struktur konkret: `src/component/inbox/attachment_list.rs` + `attachment_list_item.rs`, beide registriert in `mod.rs`.
- Backfill ist Best-Effort: Mails, die im IMAP nicht mehr existieren (validity-drift oder gelöscht), bleiben mit „Anhang nicht mehr verfügbar"-Hinweis im Frontend.

</specifics>

<deferred>
## Deferred Ideas

- **Audit-Log für Attachment-Downloads** — explizit verworfen für Phase 19 (D-10), konsistent zur Inbox-Konvention. Bei Compliance-Anforderung später separate Phase nachschieben.
- **MIME-Type-Whitelist** — verworfen; falls Spam-Welle kommt, in Folgephase reaktiv ergänzbar.
- **Konfigurierbares 10-MB-Limit via ENV** — verworfen, Hard-Konstante. Falls produktiver Bedarf entsteht (z.B. Verbands-PDFs > 10 MB), in eigener Mini-Phase nachziehen.
- **Inline-Preview für `text/plain` / `text/html`** — verworfen; HTML braucht iframe-Sandbox, eigene Sicherheitsentscheidung wert.
- **Volltext-Suche in Attachments** — Out of Scope (neues Capability).
- **Virenscan beim IMAP-Polling** — Out of Scope; setzt ClamAV o.ä. voraus, eigene Phase.
- **Bulk-Download-ZIP („Alle Anhänge dieser Mail")** — Out of Scope, in v1.3-Backlog erwähnenswert.
- **Reply-with-Forward (Anhang an Reply weitergeben)** — Out of Scope.

</deferred>

---

*Phase: 19-e-mail-anhaenge-anzeigen*
*Context gathered: 2026-06-07*
