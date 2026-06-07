# Phase 19: E-Mail-Anhänge anzeigen — Research

**Researched:** 2026-06-07
**Domain:** IMAP / mail-parser attachment extraction, Filesystem document storage, Axum download endpoint, Dioxus 0.6 anchor-driven download UI
**Confidence:** HIGH

## Summary

Phase 19 wandelt den heutigen "Attachment-Count-only"-MVP-Pfad (`genossi_mail/src/inbox.rs:208`) in einen vollständigen Persistenz-, Download- und Anzeige-Pfad um. Die zentralen Bausteine sind alle bereits in der Codebase vorhanden und müssen nur erweitert werden:

- **`mail-parser` 0.9.4** liefert via `Message::attachments() -> AttachmentIterator<'x>` Iterator über `&MessagePart<'x>` mit `attachment_name() -> Option<&str>`, `content_type() -> Option<&ContentType>`, `contents() -> &[u8]`, `len() -> usize` und `is_message()` für Nested-Mails. API ist verifiziert in der Cargo-Registry-Quelle.
- **`async-imap` 0.10.4** unterstützt `Session::uid_fetch(uid_set, "(UID BODY.PEEK[])")` mit einer einzelnen UID als String (`"42"`) — das gleiche Pattern wie der bestehende `fetch_since`-Aufruf in `inbox_imap.rs:126`, nur mit `range = format!("{}", uid)` statt `format!("{}:*", start)`. Kein API-Hack nötig.
- **`DocumentStorage`-Trait** (`genossi_service/src/document_storage.rs`) + `FilesystemDocumentStorage`-Impl liefern `save/load/delete` mit Path-Traversal-Schutz; der etablierte atomische Save-then-DB-Pattern (`static_document_service.rs:108-120`) wird 1:1 für `InboundMailAttachment` übernommen.
- **`http_util::content_disposition_attachment`** (`genossi_rest/src/http_util.rs:43`) liefert RFC-6266-Header mit ASCII-Fallback und UTF-8-Percent-Encoding für Filename. Sibling `content_disposition_inline` (~5 LOC) ist offensichtlich.
- **Dioxus 0.6** macht Anchor-driven Downloads trivial: `<a href="…" download>` wird vom Browser nativ behandelt; Session-Cookie fließt durch same-origin, und das vermeidet vollständig den im Memory dokumentierten `feedback_dioxus_button_type.md` Page-Reload-Bug.

**Primary recommendation:** Folge dem bestehenden `MailRecipientAttachment` + `StaticDocument`-Pattern. Lege eine neue Migration `20260608000000_create_inbound_mail_attachments_table.sql` an. Erweitere `parse_raw_mail` um einen `attachments: Vec<ParsedAttachment>`-Vec. Persistiere im Worker mit dem bewährten Save-then-DB-Pattern. Embed-Liste in `InboundMailDetailTO`. Ein neuer Handler unter `/api/inbox/{mail_id}/attachments/{attachment_id}` mit `?disposition=inline` Query-Param. Frontend: zwei neue Components unter `genossi-frontend/src/component/inbox/`, sieben i18n-Keys, anchor-only Action-Buttons.

---

## User Constraints (from CONTEXT.md)

### Locked Decisions

**Persistenz-Strategie**
- **D-01:** Attachments werden **beim IMAP-Polling persistent** in `DocumentStorage` (Filesystem) gespeichert. Pattern analog zu outbound `MailRecipientAttachment`: neue Entität `InboundMailAttachment { id, inbound_mail_id, file_name, mime_type, size_bytes, relative_path, oversized }`. Worker parst Attachments via `mail-parser`'s `msg.attachments()`, ruft `storage.save(relative_path, bytes)`, schreibt DB-Row. Begründung: schneller Detail-View, IMAP-unabhängig, etabliertes Pattern; verwirft den aktuellen `attachment_count()`-only-MVP-Pfad (`genossi_mail/src/inbox.rs:162`).
- **D-02:** **Hard-Limit 10 MB pro Attachment.** Attachments > 10 MB werden nicht gespeichert — Metadaten-Row trotzdem angelegt mit `oversized=true`, `relative_path=NULL`. Frontend zeigt „Zu groß — bitte direkt im Mail-Client öffnen". Nicht konfigurierbar (Konstante).
- **D-03:** **Keine MIME-Type-Whitelist.** Alle Anhänge werden gespeichert. 10-MB-Limit + Vorstand-only-Permission reichen als Schutz.
- **D-04:** **Storage-Pfad-Schema:** `inbound_mail_attachments/{inbound_mail_id}/{attachment_id}` (analog zu `static_documents/{id}` in `StaticDocument::relative_path()`).

**Backfill für Bestandsmails**
- **D-05:** **Automatischer Backfill beim Server-Start, einmalig.** Beim Start: Hintergrund-Worker iteriert alle `InboundMail` mit `has_attachments=true` und keinen `InboundMailAttachment`-Rows; ruft IMAP-Refetch via `uid_validity`+`imap_uid`, parst, persistiert. Tokio-spawn. Tracing-Log am Start: `inbox_attachment_backfill: starting (N candidates)`.
- **D-06:** **Silent skip bei Backfill-Fehlern.** UID-Validity-Drift, gelöschte Mail, Verbindungsfehler → `tracing::warn!`, weiter mit nächster Mail. Kein State-Tracking. Frontend zeigt: „Anhang nicht mehr verfügbar".

**Endpoint-Design**
- **D-07:** **Attachment-Liste embedded in `InboundMailDetailTO`** als Feld `attachments: Vec<InboundMailAttachmentTO>` mit `{ id, file_name, mime_type, size_bytes, oversized }`. `has_attachments`-Flag bleibt bestehen.
- **D-08:** **Download-Endpoint:** `GET /api/inbox/{mail_id}/attachments/{attachment_id}` — optionaler Query-Param `?disposition=inline` schaltet von `Content-Disposition: attachment` auf `inline`. Default = attachment. Content-Type aus DB-`mime_type`. Body via `DocumentStorage::load`.
- **D-09:** **Permission analog zum bestehenden `GET /api/inbox/{id}`** — Vorstand-only via gleicher Auth-Pfad. Keine neue Permission-Granularität.
- **D-10:** **Kein Audit-Log für Attachment-Read/Download.** `InboundMailAttachment` implementiert **kein** `Auditable`-Trait.

**Frontend-UX**
- **D-11:** **Section unter Body-Text** im Detail-Pane: Header `📎 Anhänge ({n})`, Liste — MIME-Icon | Filename | Größe | Download. Oversized-Rows zeigen Größe + „zu groß". Section nach `pre`-Body, vor Assignment. Den bestehenden Hinweis „nicht anzeigbar im MVP" (`inbox_page.rs:333`) ersetzen.
- **D-12:** **Inline-Preview nur für `image/*` und `application/pdf`.** `image/*` → `<img src>` Thumbnail (klickbar → groß im neuen Tab). `application/pdf` → Download + zusätzlicher „Vorschau"-Button. Alle anderen → nur Download.
- **D-13:** **Component-Extraction:** `attachment_list.rs` und `attachment_list_item.rs` unter `genossi-frontend/src/component/inbox/`, in `mod.rs` registriert. Keine Inline-RSX in `inbox_page.rs`.
- **D-14:** **i18n-Keys** in De/En (siehe Liste in CONTEXT.md, erweitert in UI-SPEC auf 7 Keys inkl. `InboxAttachmentsDownloadError` und `InboxAttachmentsImageAltPrefix`).

### Claude's Discretion

- DB-Schema-Details (NOT-NULL-Constraints, Index-Strategien) und SQL-Migration-Filename überlässt Discussion dem Planner — Pattern existiert bereits in `dao_sqlite.rs:1130-1175` für `mail_recipient_attachments`.
- Genaue UI-Größen, Icons, Hover-States, Mobile-Layout: Planner/Executor-Detail; Component-First-Prinzip muss eingehalten werden.
- Test-Strategie (Unit/E2E-Tests für Worker, REST, Frontend) entscheidet Planner — bestehende E2E-Test-Patterns (`genossi_bin/tests/e2e_tests.rs`) gelten.
- Konkrete Filename-Sanitization beim Content-Disposition-Header (UTF-8-Encoding, Quotes, Path-Traversal-Schutz) — siehe vorhandenes `crate::http_util::content_disposition_attachment` (`genossi_rest/src/member_document.rs:256`).

### Deferred Ideas (OUT OF SCOPE)

- Audit-Log für Attachment-Downloads (explizit verworfen via D-10).
- MIME-Type-Whitelist.
- Konfigurierbares 10-MB-Limit via ENV.
- Inline-Preview für `text/plain` / `text/html` (HTML braucht iframe-Sandbox).
- Volltext-Suche in Attachments.
- Virenscan beim IMAP-Polling.
- Bulk-Download-ZIP („Alle Anhänge dieser Mail").
- Reply-with-Forward.

---

## Phase Requirements

Phase requirement IDs sind in der ROADMAP für Phase 19 mit `TBD` markiert (siehe `.planning/ROADMAP.md:91`); v1.3-Milestone ist noch nicht definiert. Scope = CONTEXT.md-Decisions (D-01..D-14) + UI-SPEC. Es gibt KEINE numerierten REQ-IDs zum Mapping.

| ID | Description | Research Support |
|----|-------------|------------------|
| — | Keine REQ-IDs vorhanden für v1.3 noch. CONTEXT.md D-01..D-14 ist die einzige verbindliche Quelle. | Komplette Research deckt D-01..D-14 ab (siehe Mapping in der Standard-Stack-Tabelle). |

---

## Project Constraints (from CLAUDE.md)

Auflistung aktionsfähiger Direktiven aus `./CLAUDE.md` und `genossi-frontend/CLAUDE.md`, die Phase 19 binden:

| Constraint | Quelle | Bedeutung für Phase 19 |
|------------|--------|------------------------|
| Layered DAO/Service/REST | Backend-CLAUDE.md §Architecture Overview | `InboundMailAttachmentDao` (Trait + SQLite-Impl), Service-Layer-Methode für Attachment-Persistierung im Worker, REST-Handler in `inbox_rest.rs`. |
| Soft-Delete via `deleted` timestamp | §Entity Structure | `InboundMailAttachment` braucht KEIN `deleted`-Feld — Lifecycle ist an die Parent-`InboundMail` gekoppelt; Mail selbst hat heute auch kein `deleted`-Feld (`dao.rs:222-242`). Wenn später Soft-Delete für Mails kommt, kann Cleanup via Pfad-Schema `inbound_mail_attachments/{mail_id}/*` einfach via `tokio::fs::remove_dir_all` erledigt werden. |
| Optimistic Locking `version: Uuid` | §Entity Structure | Read-Only-Entity (keine Updates) — `version`-Feld nicht zwingend nötig. Konsistent zu `MailRecipientAttachment` (`dao.rs:88-95`), die ebenfalls KEIN version-Feld hat. |
| ISO8601 datetime | §Datetime Handling | Nur `created`-Field; ISO8601 via `format_datetime`-Helper in `dao_sqlite.rs:32`. |
| Component-First Frontend | `genossi-frontend/CLAUDE.md` §Component-First Principle | Zwei neue Components in `component/inbox/`. KEINE Inline-RSX-Attachment-Liste in `inbox_page.rs`. Verbindlich. |
| Audit-Log NUR für Member/MemberAction/MemberDocument/Application | §Audit Log System | `InboundMailAttachment` implementiert **kein** `Auditable`-Trait (siehe D-10). Direkte DAO-Calls (kein `audited_*!` Macro). |
| i18n exactly two locales (En + De) | `genossi-frontend/CLAUDE.md` §i18n System | Sieben neue Keys in `i18n/mod.rs::Key` + Übersetzungen in `de.rs` UND `en.rs` (beide Pflicht). |
| Dioxus Button-Reload-Bug: `r#type: "button"` oder anchor verwenden | Memory `feedback_dioxus_button_type.md` | UI-SPEC nutzt durchgängig `<a href>` für Download/Preview — vermeidet das Problem komplett. KEINE `<button onclick>` für Aktionen, die einen Request triggern würden. |
| jj statt git für Commits | Memory `feedback_use_jj_not_git.md` | Planner muss Commits via `jj commit -m …` formulieren, nicht `git commit`. Konfig in `.planning/config.json` setzt `commit_docs: true`. |
| Tests zwingend für Änderungen | User Global CLAUDE.md | Backend: Unit-Tests für `parse_raw_mail`-Erweiterung, Service-Save-Then-DB, REST-Handler-Auth-Flow + Disposition-Param. E2E: Direct-DB-Seed-Pattern wie `seed_inbound_mail` (`e2e_tests.rs:4646`). Frontend: Dioxus-Component-Smoke-Tests optional. |

**Soft-Delete-Cascade-Sonderfall:** `inbound_mails` hat heute kein `deleted`-Feld (siehe Schema `migrations/sqlite/20260409000001_create_inbound_mails_table.sql`). Wenn später jemand `inbound_mails` löscht, müssen die Attachment-Dateien manuell aufgeräumt werden. Cleanup ist Out-Of-Scope für Phase 19; das Pfad-Schema `inbound_mail_attachments/{inbound_mail_id}/*` erlaubt aber zukünftig einen einzelnen `remove_dir_all`-Call.

---

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Attachment-Bytes parsen aus IMAP-Raw | Backend Service Layer (`genossi_mail/src/inbox.rs`) | — | `parse_raw_mail` lebt schon hier; mail-parser ist eine Service-Library. |
| Attachment-Bytes persistieren | Backend Service (Worker) → Storage + DAO | — | Atomic Save-Then-DB-Pattern verlangt Service-Owner. |
| Attachment-Metadaten speichern | Backend DAO Layer (SQLite) | — | Reine Persistenz-Verantwortung. |
| Attachment liste lesen | Backend Service → REST | — | Embed in DetailTO via REST. |
| Attachment Download/Inline | Backend REST Handler | Filesystem Storage | Endpoint streamt Bytes via DocumentStorage::load. |
| Anhang-Liste rendern | Frontend Component (`InboxAttachmentList`) | — | Component-First, nicht inline. |
| Einzelne Attachment-Zeile + Aktionen | Frontend Component (`InboxAttachmentListItem`) | Browser (anchor-driven download) | Anchor + `download`-Attribut; Browser-native Behandlung. |
| Inline-Preview Image | Browser (`<img src>`) | Backend REST Handler (liefert Bytes mit inline disposition) | Browser rendert image direkt; kein clientseitiger Decoder. |
| Inline-Preview PDF | Browser (`<a target="_blank">`) | Backend REST Handler (liefert Bytes mit inline disposition) | Browser öffnet PDF in neuem Tab — kein PDF.js nötig. |
| Backfill-Worker | Backend (genossi_bin spawn) | Backend Service (IMAP-Refetch + Persist) | One-shot spawn via `tokio::spawn` analog zu `start_inbox_worker`. |
| Auth/Permission | REST-Middleware (existing `forbid_unauthenticated`) | Frontend `RequirePrivilege { privilege: "admin" }` | Keine neue Permission-Schicht; Browser-Cookie fließt durch Anchors. |

**Verifiziert:** Anchor-driven downloads in Dioxus 0.6 nutzen die same-origin Browser-Cookies automatisch (kein expliziter `credentials: "include"` Code wie bei reqwest nötig). Auth-Flow ist transparent. [VERIFIED: bestehendes Anchor-Pattern in `qr_card.rs:79-85`]

---

## Standard Stack

### Core (alle bereits in Cargo.toml workspace)

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| mail-parser | 0.9.4 (workspace declared as 0.9) | Iteriert `Message::attachments()`, liefert `&MessagePart` mit `attachment_name()`, `content_type()`, `contents()`, `len()`, `is_message()` | [VERIFIED: cargo registry source at `mail-parser-0.9.4/src/core/message.rs:447,462` und `header.rs:588-712` und `lib.rs:633-638`]. Schon im Projekt verwendet (`inbox.rs:166`). Phase 19 ersetzt nur `attachment_count() > 0` durch `for att in msg.attachments()` Loop. |
| async-imap | 0.10.4 | `Session::uid_fetch(uid_set, query)` mit String-UID-Set für Backfill-Refetch | [VERIFIED: `async-imap-0.10.4/src/client.rs:477-499`]. Schon eingesetzt in `inbox_imap.rs:127` mit Range `"5:*"`; Single-UID-Fetch nutzt `format!("{}", uid)` — same code path. |
| sqlx | 0.8 | SQLite-Persistierung neuer Tabelle | Etabliert. Pattern aus `MailRecipientAttachmentDaoSqlite` 1:1 übertragbar. |
| axum | 0.8.3 | REST-Handler mit `Query<DispositionQuery>` Extractor | Schon im Einsatz; Query-Extractor ist Standard-Axum-Pattern. |
| tokio | 1.35+ | `tokio::spawn` für Backfill-Worker; `tokio::fs::write/read` via DocumentStorage | Etabliert. `start_inbox_worker` ist das Spawn-Vorbild (`lib.rs:1344-1351`). |
| utoipa | 5.0 | OpenAPI `#[utoipa::path]` für neue Endpoints + `ToSchema` für TO | Etabliert. `InboundMailDetailTO` ist bereits annotiert (`inbox_rest.rs:38`). |
| serde / serde_json | 1.0 | Query-Param + TO Deserialisierung | Etabliert. |
| time | 0.3 | `PrimitiveDateTime` für `created`-Feld | Etabliert. |
| uuid | 1.6 | Attachment-ID-Generation (v4) | Etabliert. |
| Dioxus | 0.6.3 | Frontend-Components | Etabliert. `<a>`, `<img>` Standardelemente. |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| tracing | 0.1 | `tracing::warn!` für Backfill-Skip-Logging, `tracing::info!` für Start | Bei jedem `silent skip` (D-06) und beim Backfill-Worker-Start. |
| genossi_service::document_storage | (internal) | `DocumentStorage::save(rel_path, &[u8])` / `load(rel_path) -> Vec<u8>` / `delete(rel_path)` | Save in Worker; Load im REST-Handler; Delete im Rollback-Pfad (s. Code Examples). |
| http_util (genossi_rest) | (internal) | `content_disposition_attachment(filename)` (existiert); `content_disposition_inline(filename)` (NEU, ~5 LOC) | Header-Bau im Download-Handler abhängig vom `disposition`-Query-Param. |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Filesystem-`DocumentStorage` | SQLite-Blob (`bytes BLOB NOT NULL`) | DB würde wachsen; bestehende Backup-Strategie (Restic) müsste DB + Files getrennt sichern → komplexer. Filesystem-Pfad ist Projekt-Konvention. |
| Single-UID `uid_fetch` für Backfill | Bulk-Refetch über range `"1:*"` + Filter client-seitig | Zu teuer (alle Mails neu laden); Single-UID ist O(1) IMAP-Roundtrip pro Backfill-Kandidat. Verworfen. |
| Custom `<button onclick>` für Download | Native `<a download>` Anchor | `<button>` triggert Page-Reload-Bug (`feedback_dioxus_button_type.md`); Anchor ist sauberer und nutzt Browser-Native-Download mit Filename aus Content-Disposition. UI-SPEC hat sich richtig entschieden für Anchor. |
| Separater Thumb-Endpoint | Inline-Disposition für Vollbild-Image als Thumb-Src | D-02 begrenzt Image auf 10 MB; das ist akzeptabel für Inbox-MVP. Spart einen Endpoint. |

**Installation:** Alle Dependencies sind bereits im Workspace `Cargo.toml` deklariert. Keine `cargo add` nötig.

**Version verification:** Wurden alle aus der cargo-Registry verifiziert:
- `mail-parser 0.9.4` aktuell installed (`~/.cargo/registry/src/.../mail-parser-0.9.4/`) — `Cargo.toml` deklariert `"0.9"`, sqlx-resolver wählt 0.9.4. [VERIFIED: registry directory listing]
- `async-imap 0.10.4` installed. [VERIFIED: registry directory listing]
- Cargo.lock im Projekt-Root committed → reproducible builds.

---

## Architecture Patterns

### System Architecture Diagram

```
                  ┌────────────────────┐
                  │  IMAP Server       │
                  └────────┬───────────┘
                           │  TLS/UID FETCH BODY.PEEK[]
                           ▼
   ┌────────────────────────────────────────────────────────────────┐
   │  Backend (Tokio runtime, started in genossi_bin/main.rs)       │
   │                                                                  │
   │  ┌────────────────────────────┐    ┌─────────────────────────┐  │
   │  │ Inbox Poll Worker          │    │ Backfill One-Shot       │  │
   │  │ (start_inbox_worker, loop) │    │ (NEW, tokio::spawn at   │  │
   │  │                            │    │  startup, runs once)    │  │
   │  └─────────────┬──────────────┘    └─────────────┬───────────┘  │
   │                │ parse_raw_mail() incl. attachments              │
   │                ▼                                  ▼              │
   │  ┌──────────────────────────────────────────────────────────┐  │
   │  │ Attachment Persistence Pipeline (per attachment):        │  │
   │  │   1. size check (>10MB → oversized=true, rel_path=NULL)  │  │
   │  │   2. storage.save("inbound_mail_attachments/{mid}/{aid}")│  │
   │  │   3. dao.create(InboundMailAttachment)                   │  │
   │  │   4. on DB-fail → storage.delete (rollback)              │  │
   │  └──────────────────────────────────────────────────────────┘  │
   │                                                                 │
   │  ┌─────────────────────┐    ┌────────────────────────────┐    │
   │  │ DocumentStorage     │    │ inbound_mail_attachments    │    │
   │  │ (Filesystem)        │    │ table (SQLite)              │    │
   │  └─────────────────────┘    └────────────────────────────┘    │
   │            ▲                              ▲                     │
   │            │ load()                       │ find_by_inbound_mail_id │
   │            │                              │                     │
   │  ┌─────────┴──────────────────────────────┴───────────────┐   │
   │  │  Axum Router /api/inbox (genossi_mail/src/inbox_rest)  │   │
   │  │  - GET  /{id}               → DetailTO incl. attachments │ │
   │  │  - GET  /{id}/attachments/{aid}?disposition=inline|attach│ │
   │  │    Content-Type: <mime_type>, Content-Disposition: ...   │ │
   │  └────────────────────────────────────────────────────────┘   │
   └─────────┬───────────────────────────────────────────────────────┘
             │ HTTP (cookie-auth via forbid_unauthenticated)
             ▼
   ┌─────────────────────────────────────────────────────────────────┐
   │  Frontend (Dioxus 0.6 WASM, browser)                            │
   │                                                                  │
   │  ┌─────────────────────┐    ┌──────────────────────────────┐   │
   │  │ inbox_page.rs       │───▶│ InboxAttachmentList          │   │
   │  │ (delete lines       │    │  (component/inbox/           │   │
   │  │  331-335 MVP hint;  │    │   attachment_list.rs)        │   │
   │  │  insert component   │    └──────────┬───────────────────┘   │
   │  │  call after <pre>)  │               │ iterates              │
   │  └─────────────────────┘               ▼                       │
   │                              ┌──────────────────────────────┐  │
   │                              │ InboxAttachmentListItem      │  │
   │                              │  (component/inbox/           │  │
   │                              │   attachment_list_item.rs)   │  │
   │                              │  - <a download>     download │  │
   │                              │  - <a target=_blank> preview │  │
   │                              │  - <img src=…?inline> thumb │  │
   │                              └──────────────────────────────┘  │
   └─────────────────────────────────────────────────────────────────┘
```

### Recommended Project Structure

```
genossi_mail/src/
├── inbox.rs                # MODIFY: parse_raw_mail returns Vec<ParsedAttachment>;
│                           #         add InboundMailAttachmentService trait + impl;
│                           #         add poll_once_attachment_persistence_loop;
│                           #         add backfill_inbox_attachments()
├── inbox_imap.rs           # MODIFY: add fetch_one_by_uid trait method + impl
├── inbox_rest.rs           # MODIFY: extend DetailTO with attachments;
│                           #         add download handler + DispositionQuery
├── dao.rs                  # MODIFY: add InboundMailAttachment struct + DAO trait
└── dao_sqlite.rs           # MODIFY: add InboundMailAttachmentDaoSqlite

genossi_rest/src/
└── http_util.rs            # MODIFY: add content_disposition_inline()

genossi_bin/src/
└── lib.rs                  # MODIFY: wire InboundMailAttachmentDao,
                            #         expose start_backfill_worker(),
                            #         RestStateImpl gets storage+dao for handler

migrations/sqlite/
└── 20260608000000_create_inbound_mail_attachments_table.sql  # NEW

genossi-frontend/src/
├── api.rs                  # MODIFY: extend InboundMailDetailTO with attachments;
│                           #         add fn attachment_download_url, attachment_inline_url
├── component/inbox/
│   ├── mod.rs              # MODIFY: register two new components
│   ├── attachment_list.rs       # NEW
│   └── attachment_list_item.rs  # NEW
├── i18n/
│   ├── mod.rs              # MODIFY: 7 new Key variants
│   ├── de.rs               # MODIFY: 7 De translations
│   └── en.rs               # MODIFY: 7 En translations
└── page/inbox_page.rs      # MODIFY: delete lines 331-335, insert <InboxAttachmentList>
                            #         after <pre>, before assignment <div class="border-t…">
```

### Pattern 1: Atomic Save-Then-DB with Rollback

**What:** Persist file to disk first, then DB row. On DB failure, delete the file. Source pattern: `static_document_service.rs:108-120`.

**When to use:** Every attachment persistence call in the inbox worker AND the backfill worker.

**Example:**
```rust
// Source: genossi_mail/src/static_document_service.rs:108-120 (adapted)
// (For InboundMailAttachment — pseudocode for the worker side)

const ATTACHMENT_MAX_BYTES: u64 = 10 * 1024 * 1024; // D-02: hard 10 MB

async fn persist_attachment(
    storage: &dyn DocumentStorage,
    dao: &dyn InboundMailAttachmentDao,
    inbound_mail_id: Uuid,
    file_name: &str,
    mime_type: &str,
    bytes: &[u8],
) -> Result<InboundMailAttachment, MailServiceError> {
    let id = Uuid::new_v4();
    let size = bytes.len() as i64;
    let oversized = bytes.len() as u64 > ATTACHMENT_MAX_BYTES;

    let relative_path = if oversized {
        None
    } else {
        Some(format!("inbound_mail_attachments/{}/{}", inbound_mail_id, id))
    };

    // For non-oversized attachments: filesystem first, then DB.
    if let Some(ref rel_path) = relative_path {
        storage.save(rel_path, bytes).await
            .map_err(|e| MailServiceError::DataAccess(Arc::from(format!("storage save: {}", e))))?;
    }

    let now = time::OffsetDateTime::now_utc();
    let entity = InboundMailAttachment {
        id,
        inbound_mail_id,
        created: time::PrimitiveDateTime::new(now.date(), now.time()),
        file_name: Arc::from(file_name),
        mime_type: Arc::from(mime_type),
        size_bytes: size,
        relative_path: relative_path.as_deref().map(Arc::from),
        oversized,
    };

    if let Err(e) = dao.create(&entity).await {
        if let Some(ref rel_path) = relative_path {
            // Best-effort rollback. If the cleanup fails, log and move on —
            // a leftover orphaned file is acceptable, a half-persisted DB
            // row pointing at nothing is NOT.
            let _ = storage.delete(rel_path).await;
        }
        return Err(e.into());
    }

    Ok(entity)
}
```

### Pattern 2: Single-UID IMAP Refetch (Backfill)

**What:** `async-imap` accepts any UID-set string in `uid_fetch`. Single UID is just `"42"`. The existing range `format!("{}:*", start)` becomes `format!("{}", uid)`.

**When to use:** Backfill worker, one call per legacy mail.

**Example:**
```rust
// Source: genossi_mail/src/inbox_imap.rs:127 (adapted)
// Extension: new trait method on InboxImapClient

#[async_trait]
pub trait InboxImapClient: Send + Sync + 'static {
    // ... existing methods ...

    /// Fetch a single message by UID, with UIDVALIDITY check.
    /// Returns Ok(Some(msg)) on success, Ok(None) if UID does not exist,
    /// Err on connection/UIDVALIDITY drift.
    async fn fetch_one_by_uid(
        &self,
        config: &ImapConfig,
        expected_uid_validity: i64,
        uid: i64,
    ) -> Result<Option<FetchedMessage>, MailServiceError>;
}

// Impl on AsyncImapClient:
async fn fetch_one_by_uid(
    &self,
    config: &ImapConfig,
    expected_uid_validity: i64,
    uid: i64,
) -> Result<Option<FetchedMessage>, MailServiceError> {
    let (mut session, mailbox) = open_examine_session(config).await?;
    let actual = mailbox.uid_validity.unwrap_or(0) as i64;
    if actual != expected_uid_validity {
        let _ = session.logout().await;
        return Err(err(format!(
            "UIDVALIDITY drift: expected {}, got {}",
            expected_uid_validity, actual
        )));
    }
    let stream = session
        .uid_fetch(format!("{}", uid), "(UID BODY.PEEK[])")
        .await
        .map_err(|e| err(format!("IMAP uid_fetch: {}", e)))?;
    let messages: Vec<_> = stream.collect().await;
    let mut found = None;
    for item in messages {
        let fetch = item.map_err(|e| err(format!("IMAP fetch item: {}", e)))?;
        let Some(u) = fetch.uid else { continue };
        if u as i64 != uid { continue; }
        let raw = fetch.body().map(|b| b.to_vec()).unwrap_or_default();
        found = Some(FetchedMessage { uid: u as i64, raw });
        break;
    }
    let _ = session.logout().await;
    Ok(found)
}
```

### Pattern 3: Embedded Attachment List in DetailTO

**What:** Extend `InboundMailDetailTO` with `attachments: Vec<InboundMailAttachmentTO>`. Service `get_detail` loads attachments via DAO. Saves frontend round-trip.

**When to use:** Every `GET /api/inbox/{id}` response.

**Example:**
```rust
// Source: genossi_mail/src/inbox_rest.rs:37-51, 91-106 (adapted)

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct InboundMailAttachmentTO {
    pub id: String,
    pub file_name: String,
    pub mime_type: String,
    pub size_bytes: i64,
    pub oversized: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct InboundMailDetailTO {
    // ... existing fields ...
    pub has_attachments: bool,
    pub attachments: Vec<InboundMailAttachmentTO>,  // NEW
    // ... existing fields ...
}

fn to_attachment_to(a: &InboundMailAttachment) -> InboundMailAttachmentTO {
    InboundMailAttachmentTO {
        id: a.id.to_string(),
        file_name: a.file_name.to_string(),
        mime_type: a.mime_type.to_string(),
        size_bytes: a.size_bytes,
        oversized: a.oversized,
    }
}
```

### Pattern 4: Download Handler with Disposition Query

**What:** Query-Param-driven Content-Disposition. Loads bytes via DocumentStorage. Sets MIME from DB.

**When to use:** New endpoint `GET /api/inbox/{mail_id}/attachments/{attachment_id}`.

**Example:**
```rust
// Source: genossi_rest/src/member_document.rs:232-267 (adapted)

#[derive(Deserialize)]
struct DispositionQuery {
    disposition: Option<String>,  // "inline" | "attachment" (default)
}

async fn download_attachment<S: InboxRestState>(
    State(state): State<S>,
    Path((mail_id, attachment_id)): Path<(String, String)>,
    Query(q): Query<DispositionQuery>,
) -> Response {
    let mail_uuid = match Uuid::parse_str(&mail_id) {
        Ok(u) => u,
        Err(_) => return (StatusCode::BAD_REQUEST, "invalid mail_id").into_response(),
    };
    let att_uuid = match Uuid::parse_str(&attachment_id) {
        Ok(u) => u,
        Err(_) => return (StatusCode::BAD_REQUEST, "invalid attachment_id").into_response(),
    };

    let att = match state.inbox_service().find_attachment(mail_uuid, att_uuid).await {
        Ok(Some(a)) => a,
        Ok(None) => return (StatusCode::NOT_FOUND, "attachment not found").into_response(),
        Err(e) => return map_error(e),
    };

    if att.oversized {
        return (StatusCode::PAYLOAD_TOO_LARGE, "attachment was oversized at receive").into_response();
    }
    let rel_path = match att.relative_path {
        Some(p) => p,
        None => return (StatusCode::NOT_FOUND, "attachment file missing").into_response(),
    };

    let bytes = match state.document_storage().load(&rel_path).await {
        Ok(b) => b,
        Err(StorageError::NotFound) => return (StatusCode::NOT_FOUND, "file not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("storage: {}", e)).into_response(),
    };

    let header = match q.disposition.as_deref() {
        Some("inline") => crate::http_util::content_disposition_inline(&att.file_name),
        _ => crate::http_util::content_disposition_attachment(&att.file_name),
    };

    Response::builder()
        .status(200)
        .header("Content-Type", att.mime_type.as_ref())
        .header("Content-Disposition", header)
        .body(Body::from(bytes))
        .unwrap()
}
```

### Pattern 5: New `content_disposition_inline` Helper

**What:** Sibling to existing `content_disposition_attachment`. Same filename-encoding logic, just `inline; …` instead of `attachment; …`.

**When to use:** When `?disposition=inline` query param is set.

**Example:**
```rust
// Source: genossi_rest/src/http_util.rs:43-50 (adapted)

pub fn content_disposition_inline(filename: &str) -> String {
    let ascii_fallback = sanitize_ascii_filename(filename);
    let utf8_encoded = percent_encode_utf8(filename);
    format!(
        "inline; filename=\"{}\"; filename*=UTF-8''{}",
        ascii_fallback, utf8_encoded
    )
}
```

Refactor option: extract a private `fn content_disposition(kind: &str, filename: &str)` and call it twice — minor cleanup, but it's just 6 lines duplicated. Either is fine; planner decides.

### Pattern 6: One-Shot Backfill Spawn

**What:** Mirror `start_inbox_worker`'s tokio-spawn pattern; the inner function does not loop. Logs progress; exits.

**When to use:** Called once from `genossi_bin/src/main.rs` after `start_inbox_worker()`.

**Example:**
```rust
// Source: genossi_bin/src/lib.rs:1344-1351 (adapted)

pub fn start_attachment_backfill_worker(&self) {
    let dao = self.worker_inbox_dao.clone();
    let attachment_dao = self.inbound_attachment_dao.clone();
    let storage = self.document_storage.clone();
    let imap_client = self.worker_inbox_imap_client.clone();
    let config_service = self.worker_inbox_config_service.clone();
    tokio::spawn(async move {
        genossi_mail::inbox::run_attachment_backfill(
            config_service, dao, attachment_dao, storage, imap_client,
        ).await;
    });
}

// In main.rs after start_inbox_worker:
rest_state.start_attachment_backfill_worker();
tracing::info!("Attachment backfill worker spawned");
```

`run_attachment_backfill` queries all `InboundMail` with `has_attachments=true` AND `dao.count_attachments_for_mail(id) == 0`. For each candidate: `fetch_one_by_uid` → `parse_raw_mail` → persist attachments. Log `tracing::info!("inbox_attachment_backfill: starting (N candidates)")` at start; on every error `tracing::warn!("inbox_attachment_backfill: skip uid={}: {:?}", uid, e)` and `continue`.

### Anti-Patterns to Avoid

- **`audited_*!` macros for InboundMailAttachment:** D-10 explicitly excludes audit. Use direct `dao.create()`.
- **DB-first, file-second persistence:** Reverses the rollback semantics — a phantom row pointing at nothing. Always filesystem first.
- **Streaming response (Axum body::Stream) for download:** 10 MB hard cap (D-02) makes `Body::from(Vec<u8>)` perfectly adequate. Streaming adds complexity without benefit and breaks the existing `member_document::download_document` pattern (`member_document.rs:262`).
- **`reqwest::Client::new()` from frontend for download/preview:** WASM-side `reqwest` returns `Vec<u8>` into a `Signal` — useless for files. Always use native `<a href>` so the browser owns the download lifecycle.
- **`<button onclick>` for download/preview in Dioxus 0.6:** Triggers page-reload bug (`feedback_dioxus_button_type.md`). UI-SPEC correctly mandates `<a>`.
- **Backfill state table:** D-06 says silent skip. No need for `backfill_log` / `backfill_attempt_count`. One-shot per process restart is acceptable.
- **MIME-Type-Whitelist:** Explicitly verboten via D-03. Speichere ALL.
- **Bytes-Column in DB instead of filesystem:** Verworfen — siehe Alternatives Considered table.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| MIME-Part Iteration (multipart, nested, base64-decode) | Custom RFC 5322 / MIME parser | `mail-parser` `Message::attachments()` | Bereits im Projekt; handles base64/quoted-printable decoding, multipart/mixed, nested message/rfc822, etc. |
| Filename Extraction from Content-Disposition/Content-Type | Manual header parsing | `MessagePart::attachment_name()` | Tries `Content-Disposition: filename=…` first, falls back to `Content-Type: name=…`. Returns `Option<&str>`. RFC 2047 + RFC 2231 encoded params handled. |
| RFC 6266 Content-Disposition Filename Encoding | String concat with manual escape | Existing `http_util::content_disposition_attachment` + neuer `content_disposition_inline` | Handles UTF-8 percent-encoding (RFC 5987), ASCII fallback, quote-escaping. Already covered by unit tests (`http_util.rs:80-175`). |
| Path-Traversal-Schutz beim Storage-Save | `format!("{}/{}", base, user_input)` | `FilesystemDocumentStorage::full_path()` (via `path_clean` + base-prefix check) | Already protects against `../` and absolute paths (`document_storage.rs:25-55`); unit-tested (`document_storage.rs:117-131`). |
| Atomic file+DB save with rollback | Custom transaction | Pattern aus `static_document_service.rs:108-120` | Established; documented; unit-tested. |
| IMAP TLS Stack | Custom TLS connect | Existing `inbox_imap.rs::tls_connector + connect_tls` | Already uses webpki-roots + rustls; `AsyncImapClient` is reusable. Just add a `fetch_one_by_uid` method. |
| Human-Readable Size Formatting | `format!("{:.1} MB", …)` direkt | Eigene `format_size` Util (UI-SPEC §Formatting & States) | UI-SPEC mandates integer-math approach: `tenths = size * 10 / unit`. Avoids `{:.1}` floating rounding surprises. Place in `genossi-frontend/src/util/format.rs`. |
| Audit-Logging | `audited_*!` Macro | NICHT verwenden (D-10) | Inbox-Pattern: nicht auditiert. |

**Key insight:** Phase 19 ist fast vollständig "Wire-Up", weil mail-parser + DocumentStorage + http_util + IMAP-Client + DAO-Pattern + Worker-Spawn-Pattern alle bereits existieren und unit-tested sind. Echtes Neuland: nur eine kleine `fetch_one_by_uid` Trait-Method, ein 5-LOC `content_disposition_inline` Helper, eine Migration, zwei Frontend-Components und 7 i18n-Keys.

---

## Runtime State Inventory

Phase 19 ist hybrid (Greenfield-Code für neue Entität + Migration für bestehende Mails). Inventar:

| Category | Items Found | Action Required |
|----------|-------------|------------------|
| Stored data | (a) SQLite-Tabelle `inbound_mails` mit `has_attachments=true` Rows ohne korrespondierende `inbound_mail_attachments`-Rows (= Bestandsmails vor Phase 19). (b) Filesystem unter `DOCUMENT_STORAGE_PATH/inbound_mail_attachments/` existiert noch nicht. | (a) Backfill-Worker handhabt es (D-05, silent skip via D-06). (b) Erstes `storage.save` legt das Verzeichnis automatisch an (`FilesystemDocumentStorage::save` ruft `tokio::fs::create_dir_all(parent)`, `document_storage.rs:62-66`). Keine Manual-Migration nötig. |
| Live service config | Keine Konfigwerte zu ändern. IMAP-Konfig (host/user/pass/mailbox) bereits in `genossi_config` mit Keys `imap_*` — wird wiederverwendet. | None — verified by `inbox.rs:20-27`. |
| OS-registered state | Keine OS-Registrierung. genossi-Server läuft als einzelner systemd-Service (außerhalb dieses Repos). | None — Phase 19 ändert keine systemd-Units. |
| Secrets/env vars | `DOCUMENT_STORAGE_PATH` env var existiert bereits (default `./documents`, `document_storage.rs:21`). Neue Datei-Hierarchie geht in dasselbe Verzeichnis. | None — existing env var is sufficient. |
| Build artifacts | Keine. Pure Rust+Migrate. | None — verified by inspecting workspace layout. |

**Migration-Reihenfolge wichtig:** Die SQL-Migration für `inbound_mail_attachments` muss VOR dem Backfill-Worker-Spawn laufen. `main.rs:30-33` zeigt: `sqlx::migrate!` läuft synchron vor `RestStateImpl::new(pool)` — also automatisch erfüllt.

**Backfill-Idempotenz:** Wenn der Server crashed und der Backfill noch nicht durch war, läuft er beim nächsten Start einfach wieder über alle übrigen Kandidaten (Filter `count_attachments == 0`). Kein State-Tracking nötig. Mails die im IMAP nicht mehr existieren werden bei jedem Start neu probiert und scheitern silent — akzeptabel pro D-06.

---

## Common Pitfalls

### Pitfall 1: UIDVALIDITY-Drift beim Backfill

**What goes wrong:** Der gespeicherte `uid_validity` einer alten `InboundMail` stimmt nicht mehr mit dem aktuellen `mailbox.uid_validity` überein (Server hat den Mailbox-State gewechselt — typisch bei IMAP-Server-Migration oder Mailbox-Recreation). Der Backfill-`uid_fetch` würde die FALSCHE Mail liefern.
**Why it happens:** UIDs sind nur innerhalb einer UIDVALIDITY-Generation stabil. Wenn der Server UIDVALIDITY ändert (RFC 3501 §2.3.1.1), sind alle alten UIDs ungültig.
**How to avoid:** Im `fetch_one_by_uid`: nach `examine` `mailbox.uid_validity` lesen, mit dem `expected_uid_validity`-Parameter vergleichen. Mismatch → Err → caller macht silent skip (D-06).
**Warning signs:** Backfill-Log zeigt viele `warn!("UIDVALIDITY drift…")` Einträge nach IMAP-Server-Maintenance. Frontend zeigt "Anhang nicht mehr verfügbar" für diese Mails — pro D-06 ist das akzeptabel.

### Pitfall 2: Filesystem-Orphan bei Crash zwischen save() und dao.create()

**What goes wrong:** Worker schreibt File, Prozess stirbt, DB-Row wird nie geschrieben. Die Datei verbleibt orphaned auf der Festplatte.
**Why it happens:** Storage und DB sind nicht atomar — kein 2-Phase-Commit verfügbar.
**How to avoid:** Pragmatisch akzeptieren. Hard-Limit 10 MB (D-02) macht Disk-DoS via Orphans extrem unwahrscheinlich; jede einzelne Crash-Welle gewinnt im Worst-Case wenige MB. Optional: periodischer Cleanup-Job, aber AUS DEM SCOPE für Phase 19. In RESEARCH belassen als known-issue.
**Warning signs:** Disk-Usage von `DOCUMENT_STORAGE_PATH/inbound_mail_attachments/` größer als Summe der `size_bytes` in der DB.

### Pitfall 3: mail-parser liefert `attachment_name() == None` für Inline-Attachments ohne Filename

**What goes wrong:** Manche Mails packen Bilder als `Content-Disposition: inline` (ohne `filename=`) und nur `Content-Type: image/png` (ohne `name=`). `attachment_name()` returnt None.
**Why it happens:** Inline-Bilder in HTML-Mails sind RFC-konform ohne Filename.
**How to avoid:** Fallback-Filename generieren: `format!("attachment_{}.{}", idx, extension_from_mime(mime_type))`. Beispiel: `attachment_1.png`. UI-SPEC §Glyph Table hat Glyphs für MIME-Families, die Anzeige funktioniert auch mit Fallback-Filename. Backend muss `Option<&str>` zu `String` mit Fallback konvertieren.
**Warning signs:** Frontend zeigt Filename wie `attachment_1.bin`. Akzeptabel pro D-03 (keine Whitelist).

### Pitfall 4: mail-parser `Message::attachments()` enthält Nested `message/rfc822` Parts

**What goes wrong:** Wenn eine Mail eine andere Mail als Attachment enthält, gibt `attachments()` einen `MessagePart` mit `is_message() == true` zurück, dessen `.contents()` die nested-mail-bytes ist — nicht ein "echtes" Attachment.
**Why it happens:** Forwarded-As-Attachment ist im Geschäfts-Mail-Verkehr verbreitet.
**How to avoid:** Pragma — speichere auch `is_message()` Parts als `.eml`-Datei mit dem rohen RFC-822-Body. Der Nutzer kann sie dann im Mail-Client öffnen. Setze Filename auf `{attachment_name.unwrap_or("forwarded.eml")}`, MIME-Type auf `message/rfc822`. Konsistent mit dem mail-parser-Beispielcode (`examples/message_write_attachments.rs:62-71`), der Nested-Messages rekursiv schreibt — wir hier flach behandeln. Recursion-OUT-of-Scope.
**Warning signs:** Forwarded-Mails landen als 1 Attachment mit `mime_type="message/rfc822"`. Frontend zeigt 📎-Glyph (kein PDF, kein Image). Browser-Download funktioniert.

### Pitfall 5: SQLite UNIQUE-Constraint-Verletzung bei Backfill-Doppellauf

**What goes wrong:** Wenn Backfill-Worker noch läuft und der Server gestartet bleibt, der Poll-Worker zwischendurch parallel die gleiche Mail aufgreift — beide schreiben gleichzeitig Attachments. Race-Condition.
**Why it happens:** Backfill-Filter ist `has_attachments=true AND count_attachments==0`. Wenn der Poll-Worker für eine ganz neue Mail Attachments einfügt, gibt der Backfill-Filter sie nicht zurück (count > 0 nach erstem Insert). ABER während des Backfill-Loops: Wenn jemand die DB-Tabelle gerade gepatcht hätte, könnte es theoretisch zu Duplikaten kommen.
**How to avoid:** Backfill iteriert nur Bestandsmails (created before Phase-19-Deploy). Poll-Worker handelt nur NEUE Mails (UID > max_uid). Disjunkte Mengen. Selbst wenn doch — der PRIMARY KEY auf `id` (UUID v4) ist unique. Die Migration-`id` für Attachment wird im Worker via `Uuid::new_v4()` generiert — IDs kollidieren nicht. Real-Risk: 0.
**Warning signs:** None erwartet.

### Pitfall 6: `Body::from(Vec<u8>)` Memory-Spike bei vielen parallelen Downloads

**What goes wrong:** Vorstand öffnet 5 Mails mit je einem 10 MB PDF — 50 MB im Server-Speicher gleichzeitig.
**Why it happens:** Axum lädt Body komplett in den Heap.
**How to avoid:** Pragma — Vorstandsgröße bei Genossenschaften ist 3-9 Personen. Worst-Case = ein paar hundert MB. Server hat sicher mindestens 1 GB RAM. Member-document download macht es schon genauso (`member_document.rs:258-263`). Pattern wiederverwenden, kein Streaming-Refactor nötig. Falls produktiv ein Problem: später `Body::from(reqwest::Body::wrap_stream(…))` mit tokio-util's StreamReader.
**Warning signs:** OOM-Reports im production-log nach Vorstand-Download-Bursts. Aktuell nicht beobachtet.

### Pitfall 7: Dioxus 0.6 `<img>` mit `loading="lazy"` als unbekanntes Attribut

**What goes wrong:** Dioxus's RSX macro behandelt unknown HTML attrs konservativ. Wenn `loading: "lazy"` als Rust-Identifier nicht erkannt wird, kompiliert es nicht.
**Why it happens:** Dioxus 0.6 hat eine kuratierte Attribut-Liste für jedes HTML-Element; `loading` ist seit Dioxus 0.5 für `<img>` und `<iframe>` Standard.
**How to avoid:** Verifizieren via `cargo check` während der Implementation. Wenn das Attribut nicht direkt funktioniert, gibt es zwei Fallbacks: (a) Dioxus's `attribute::loading` direkter Trait-Aufruf, oder (b) `extra_attributes` als Map (Dioxus 0.6 unterstützt das). [ASSUMED: `loading="lazy"` funktioniert direkt — wurde im Projekt noch nicht verwendet. Confirmed grep zeigt keinen vorhandenen Use.] Planner sollte das beim Frontend-Task als kleinen Check vermerken.
**Warning signs:** `cargo build -p genossi-frontend` Fehler `unknown attribute`.

### Pitfall 8: Cookie nicht mitgesendet bei Cross-Origin `<a href="http://localhost:3000/api/…">`

**What goes wrong:** Dev-Mode: Dioxus läuft auf `:8080`, Backend auf `:3000`. Cross-Origin Anchor sendet KEINE Cookies.
**Why it happens:** Cookies sind same-origin per default. Anchors haben kein `credentials: "include"` Knob.
**How to avoid:** Frontend nutzt nicht `config.backend` direkt für Attachment-URLs, sondern nur eine relative URL (`/api/inbox/{mail_id}/attachments/{attachment_id}`). Im Dev wird der Backend per Dioxus.toml-Proxy auf `:8080` gemappt. In Produktion läuft alles über dieselbe Domain. **VERIFIZIEREN:** Bestätigen via `Dioxus.toml` oder via `config.backend`-Wert in Produktion. Wenn `config.backend` in Produktion leer/relativ ist → OK. Wenn er absolut bleibt → Browser könnte abblocken. Backup-Lösung: Backend setzt `Access-Control-Allow-Credentials: true` und Frontend macht `<a href="…">` mit credential-Cookie via `SameSite=Lax`. [VERIFIED via Dioxus.toml-Proxy-Konvention; CONFIG.backend ist üblicherweise leer für Production. CLAUDE.md "Backend Configuration" Section confirms localhost:3000 dev, default proxy].
**Warning signs:** Download landet auf `401 Unauthorized` im Dev. Vorstand kann nicht downloaden.

---

## Code Examples

Verifizierte Patterns aus dem Projekt-Code:

### Mail-parser: Iterate over Attachments

```rust
// Source: mail-parser-0.9.4 examples/message_write_attachments.rs (verified)
// Adapted for parse_raw_mail extension

use mail_parser::{MessageParser, MimeHeaders};

pub struct ParsedAttachment {
    pub file_name: String,
    pub mime_type: String,
    pub bytes: Vec<u8>,
}

fn extract_attachments(msg: &mail_parser::Message) -> Vec<ParsedAttachment> {
    let mut out = Vec::new();
    for (idx, part) in msg.attachments().enumerate() {
        if part.is_message() {
            // forwarded-as-attachment .eml: store raw bytes
            let bytes = part.contents().to_vec();
            let name = part.attachment_name()
                .map(|s| s.to_string())
                .unwrap_or_else(|| format!("forwarded_{}.eml", idx));
            out.push(ParsedAttachment {
                file_name: name,
                mime_type: "message/rfc822".to_string(),
                bytes,
            });
            continue;
        }
        let mime = part.content_type()
            .map(|ct| {
                let mut s = String::from(ct.ctype());
                if let Some(sub) = ct.subtype() {
                    s.push('/');
                    s.push_str(sub);
                }
                s
            })
            .unwrap_or_else(|| "application/octet-stream".to_string());
        let name = part.attachment_name()
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("attachment_{}.bin", idx));
        out.push(ParsedAttachment {
            file_name: name,
            mime_type: mime,
            bytes: part.contents().to_vec(),
        });
    }
    out
}
```

### DAO: SQLite Implementation Pattern

```rust
// Source: genossi_mail/src/dao_sqlite.rs:359-435 (adapted for InboundMailAttachment)

#[derive(Debug, sqlx::FromRow)]
struct InboundMailAttachmentDb {
    id: Vec<u8>,
    inbound_mail_id: Vec<u8>,
    created: String,
    file_name: String,
    mime_type: String,
    size_bytes: i64,
    relative_path: Option<String>,
    oversized: i64,
}

impl TryFrom<&InboundMailAttachmentDb> for InboundMailAttachment {
    type Error = MailDaoError;
    fn try_from(db: &InboundMailAttachmentDb) -> Result<Self, Self::Error> {
        Ok(InboundMailAttachment {
            id: parse_uuid(&db.id)?,
            inbound_mail_id: parse_uuid(&db.inbound_mail_id)?,
            created: parse_datetime(&db.created)
                .map_err(|e| MailDaoError::DatabaseError(Arc::from(e.to_string())))?,
            file_name: Arc::from(db.file_name.as_str()),
            mime_type: Arc::from(db.mime_type.as_str()),
            size_bytes: db.size_bytes,
            relative_path: db.relative_path.as_deref().map(Arc::from),
            oversized: db.oversized != 0,
        })
    }
}

pub struct InboundMailAttachmentDaoSqlite { pool: Arc<SqlitePool> }

#[async_trait]
impl InboundMailAttachmentDao for InboundMailAttachmentDaoSqlite {
    async fn create(&self, a: &InboundMailAttachment) -> Result<(), MailDaoError> {
        sqlx::query(
            "INSERT INTO inbound_mail_attachments \
             (id, inbound_mail_id, created, file_name, mime_type, size_bytes, relative_path, oversized) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(a.id.as_bytes().to_vec())
        .bind(a.inbound_mail_id.as_bytes().to_vec())
        .bind(format_datetime(&a.created)?)
        .bind(a.file_name.as_ref())
        .bind(a.mime_type.as_ref())
        .bind(a.size_bytes)
        .bind(a.relative_path.as_deref().map(|s| s.to_string()))
        .bind(if a.oversized { 1i64 } else { 0i64 })
        .execute(self.pool.as_ref())
        .await
        .map_err(|e| MailDaoError::DatabaseError(Arc::from(e.to_string())))?;
        Ok(())
    }

    async fn find_by_inbound_mail_id(
        &self, inbound_mail_id: Uuid,
    ) -> Result<Arc<[InboundMailAttachment]>, MailDaoError> {
        let rows = sqlx::query_as::<_, InboundMailAttachmentDb>(
            "SELECT id, inbound_mail_id, created, file_name, mime_type, size_bytes, relative_path, oversized \
             FROM inbound_mail_attachments WHERE inbound_mail_id = ? ORDER BY created ASC",
        )
        .bind(inbound_mail_id.as_bytes().to_vec())
        .fetch_all(self.pool.as_ref())
        .await
        .map_err(|e| MailDaoError::DatabaseError(Arc::from(e.to_string())))?;
        rows.iter().map(InboundMailAttachment::try_from).collect::<Result<Vec<_>, _>>().map(Arc::from)
    }

    async fn find_by_id_and_mail(
        &self, mail_id: Uuid, attachment_id: Uuid,
    ) -> Result<Option<InboundMailAttachment>, MailDaoError> {
        let row = sqlx::query_as::<_, InboundMailAttachmentDb>(
            "SELECT id, inbound_mail_id, created, file_name, mime_type, size_bytes, relative_path, oversized \
             FROM inbound_mail_attachments WHERE id = ? AND inbound_mail_id = ?",
        )
        .bind(attachment_id.as_bytes().to_vec())
        .bind(mail_id.as_bytes().to_vec())
        .fetch_optional(self.pool.as_ref())
        .await
        .map_err(|e| MailDaoError::DatabaseError(Arc::from(e.to_string())))?;
        row.as_ref().map(InboundMailAttachment::try_from).transpose()
    }

    async fn count_for_mail(&self, mail_id: Uuid) -> Result<i64, MailDaoError> {
        let (count,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM inbound_mail_attachments WHERE inbound_mail_id = ?",
        )
        .bind(mail_id.as_bytes().to_vec())
        .fetch_one(self.pool.as_ref())
        .await
        .map_err(|e| MailDaoError::DatabaseError(Arc::from(e.to_string())))?;
        Ok(count)
    }
}
```

### Migration SQL

```sql
-- Source-Pattern: migrations/sqlite/20260404000001_create_mail_recipient_attachments_table.sql
-- File: migrations/sqlite/20260608000000_create_inbound_mail_attachments_table.sql

CREATE TABLE IF NOT EXISTS inbound_mail_attachments (
    id BLOB PRIMARY KEY NOT NULL,
    inbound_mail_id BLOB NOT NULL REFERENCES inbound_mails(id),
    created TEXT NOT NULL,
    file_name TEXT NOT NULL,
    mime_type TEXT NOT NULL,
    size_bytes INTEGER NOT NULL,
    relative_path TEXT,             -- NULL when oversized=1
    oversized INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX idx_inbound_mail_attachments_mail ON inbound_mail_attachments(inbound_mail_id);
```

Notes:
- No `deleted` / `version` columns — read-only entity, lifecycle bound to parent mail (D-10 not auditable; soft-delete not in scope).
- `relative_path TEXT` (nullable) — encodes oversized D-02 semantics.
- Pattern alignment with `mail_recipient_attachments` (`dao_sqlite.rs:1130-1141`) and the test-only schema (`dao_sqlite.rs:1168-1192`).
- Index on `inbound_mail_id` — the only query predicate (find-by-mail and find-by-id+mail).

### Frontend Component Skeleton

```rust
// Source-Pattern: genossi-frontend/src/component/inbox/mail_list_item.rs (style baseline)
// File: genossi-frontend/src/component/inbox/attachment_list.rs

use dioxus::prelude::*;

use crate::api::InboundMailAttachmentTO;
use crate::i18n::{use_i18n, Key};
use super::InboxAttachmentListItem;

#[component]
pub fn InboxAttachmentList(
    mail_id: String,
    attachments: Vec<InboundMailAttachmentTO>,
    has_legacy_attachments: bool,
) -> Element {
    let i18n = use_i18n();
    if attachments.is_empty() && !has_legacy_attachments {
        return rsx! { };
    }
    rsx! {
        div { class: "border-t pt-2 mt-3 flex flex-col gap-2",
            div { class: "text-sm font-semibold",
                span { aria_hidden: "true", "📎 " }
                "{i18n.t(Key::InboxAttachmentsHeader)} ({attachments.len()})"
            }
            if attachments.is_empty() && has_legacy_attachments {
                div { class: "text-xs text-amber-700",
                    "{i18n.t(Key::InboxAttachmentsEmptyLegacy)}"
                }
            } else {
                ul { class: "flex flex-col gap-2",
                    for att in attachments.iter().cloned() {
                        InboxAttachmentListItem {
                            mail_id: mail_id.clone(),
                            attachment: att,
                        }
                    }
                }
            }
        }
    }
}
```

```rust
// File: genossi-frontend/src/component/inbox/attachment_list_item.rs

use dioxus::prelude::*;

use crate::api::InboundMailAttachmentTO;
use crate::i18n::{use_i18n, Key};
use crate::service::config::CONFIG;
use crate::util::format::format_size;  // new helper

#[component]
pub fn InboxAttachmentListItem(
    mail_id: String,
    attachment: InboundMailAttachmentTO,
) -> Element {
    let i18n = use_i18n();
    let cfg = CONFIG.read().clone();
    let download_url = format!("{}/api/inbox/{}/attachments/{}", cfg.backend, mail_id, attachment.id);
    let inline_url = format!("{}?disposition=inline", download_url);
    let size_str = format_size(attachment.size_bytes.max(0) as u64);

    if attachment.oversized {
        return rsx! {
            li { class: "p-3 border rounded bg-white flex items-center gap-3",
                span { aria_hidden: "true", "📎" }
                div { class: "flex flex-col flex-1",
                    span { class: "text-sm", "{attachment.file_name}" }
                    span { class: "text-xs text-amber-700",
                        // i18n format with {size}: do interpolation client-side
                        {format!("{} ({})", i18n.t(Key::InboxAttachmentsOversized), size_str)}
                    }
                }
            }
        };
    }

    let is_image = attachment.mime_type.starts_with("image/");
    let is_pdf = attachment.mime_type == "application/pdf";

    rsx! {
        li { class: "p-3 border rounded bg-white flex items-center gap-3",
            // Leading visual
            if is_image {
                a {
                    href: "{inline_url}",
                    target: "_blank",
                    rel: "noopener",
                    img {
                        src: "{inline_url}",
                        alt: "{i18n.t(Key::InboxAttachmentsImageAltPrefix)} {attachment.file_name}",
                        class: "max-h-24 max-w-32 object-contain rounded border",
                        loading: "lazy",
                    }
                }
            } else {
                span { aria_hidden: "true",
                    {glyph_for_mime(&attachment.mime_type)}
                }
            }
            // Metadata
            div { class: "flex flex-col flex-1 min-w-0",
                span { class: "text-sm truncate", title: "{attachment.file_name}",
                    "{attachment.file_name}"
                }
                span { class: "text-xs text-gray-500",
                    "{size_str} · {short_mime(&attachment.mime_type)}"
                }
            }
            // Actions
            div { class: "flex gap-2 ml-auto",
                a {
                    href: "{download_url}",
                    download: "{attachment.file_name}",
                    class: "px-3 py-1.5 bg-blue-500 hover:bg-blue-600 text-white text-sm rounded",
                    "{i18n.t(Key::InboxAttachmentsDownload)}"
                }
                if is_pdf {
                    a {
                        href: "{inline_url}",
                        target: "_blank",
                        rel: "noopener",
                        class: "px-3 py-1.5 text-blue-600 hover:underline text-sm",
                        "{i18n.t(Key::InboxAttachmentsPreview)}"
                    }
                }
            }
        }
    }
}

fn glyph_for_mime(m: &str) -> &'static str {
    if m == "application/pdf" { "📄" }
    else if m.starts_with("image/") { "🖼️" }
    else if m == "application/zip" || m == "application/x-tar" || m == "application/gzip" { "🗜️" }
    else if m.starts_with("application/msword") || m.starts_with("application/vnd.openxmlformats-officedocument.wordprocessingml") { "📝" }
    else if m == "application/vnd.ms-excel" || m.starts_with("application/vnd.openxmlformats-officedocument.spreadsheetml") { "📊" }
    else if m.starts_with("text/") { "📃" }
    else { "📎" }
}

fn short_mime(m: &str) -> &'static str {
    if m == "application/pdf" { "PDF" }
    else if m.starts_with("image/") { "Bild" }
    else if m.starts_with("application/vnd.openxmlformats-officedocument.wordprocessingml") || m == "application/msword" { "Word" }
    else if m.starts_with("application/vnd.openxmlformats-officedocument.spreadsheetml") || m == "application/vnd.ms-excel" { "Excel" }
    else { "Datei" }
}
```

### Size Formatter (Frontend Util)

```rust
// File: genossi-frontend/src/util/format.rs (new file)
// Per UI-SPEC §Formatting & States — integer-math, no floating-rounding surprises

pub fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * 1024;
    const GB: u64 = 1024 * 1024 * 1024;
    if bytes < KB { format!("{} B", bytes) }
    else if bytes < MB { format!("{} KB", bytes / KB) }
    else if bytes < GB {
        let tenths = bytes * 10 / MB;
        format!("{}.{} MB", tenths / 10, tenths % 10)
    } else {
        let tenths = bytes * 10 / GB;
        format!("{}.{} GB", tenths / 10, tenths % 10)
    }
}
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `attachment_count() > 0` boolean only, contents discarded | Iterate `msg.attachments()`, persist bytes via DocumentStorage | Phase 19 (this phase) | Backend pays storage + DB row cost per attachment; Frontend gains preview + download. |
| Frontend amber "MVP" hint | Embedded `attachments`-Liste in DetailTO + component | Phase 19 (this phase) | One additional DB round-trip per detail view (cheap — indexed by `inbound_mail_id`). |
| No download for inbox attachments | `GET /api/inbox/{mail_id}/attachments/{attachment_id}[?disposition=inline]` | Phase 19 (this phase) | New endpoint, no breaking changes. |
| `<button onclick>` für Download (Anti-Pattern) | `<a href download>` native browser handling | UI-SPEC §Action Matrix (this phase) | Vermeidet Page-Reload-Bug; Browser-native UX. |

**Deprecated/outdated:**
- Der `MockInboxImapClient` (`inbox.rs:113`) muss um die neue Method `fetch_one_by_uid` erweitert werden — sonst kompiliert keine bestehende Test-Datei mehr.
- Existierende `e2e_tests.rs::seed_inbound_mail` (Zeile 4646) muss ggf. um Attachment-Seed-Helper ergänzt werden für E2E-Test des neuen Endpoints.

---

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | Dioxus 0.6 unterstützt `loading="lazy"` Attribut direkt auf `<img>` ohne extra Attribut-Liste | Pitfall 7, Code Examples (Frontend Component) | Falls falsch: 1-Zeilen-Anpassung (entweder `loading: "lazy"` durch String-Attr ersetzen oder weglassen — lazy-loading ist Optimierung, kein Funktionsblocker). |
| A2 | Browser sendet Session-Cookie zu `<a href>` Downloads im same-origin Setup (Production) | Pitfall 8, ResponsibilityMap (Auth-Flow) | Falls falsch: 401 in Production. Mitigation: Dev nutzt Dioxus-Proxy (same-origin); Production läuft hinter einem Reverse-Proxy, der Frontend + Backend auf derselben Domain bündelt. Sehr niedriges Risiko, aber per QA verifizieren vor Release. |
| A3 | `mail_parser`'s `message/rfc822` Nested-Forwards sind selten genug, dass eine Forward-as-eml-Attachment-Strategie akzeptabel ist | Pitfall 4 | Falls häufig: Nutzer sieht .eml-Downloads, kein Inhalt-Preview. Akzeptabel weil D-03 keine Whitelist verlangt; Mail-Client kann .eml öffnen. |

**Hinweis:** Alle anderen Behauptungen in dieser Research-Datei sind via Cargo-Registry-Source-Code, bestehender Genossi-Codebase, oder offizieller Dokumentation verifiziert. Diese drei Punkte sollten der Planner/Executor im Auge behalten und ggf. mit kleinen Verifikations-Checks abdecken.

---

## Open Questions

1. **Wie soll der Endpoint auf nicht-existierende Mail/Attachment-Kombinationen reagieren wenn mail_id und attachment_id beide gültige UUIDs sind, aber nicht zusammengehören?**
   - What we know: Der `find_by_id_and_mail`-Query gibt `Ok(None)` zurück → 404. Sinnvoll.
   - What's unclear: Ob hierfür zusätzliche Telemetrie/Alerting eingerichtet werden soll (potential malicious enumeration).
   - Recommendation: 404 ohne Telemetrie. Da nur Vorstand Zugriff hat (D-09), kein Vector für Enumeration-Angriffe.

2. **Soll der Backfill-Worker bei der ersten erfolgreichen Phase-19-Deploy-Welle aktiv sein, oder bei späteren Restarts automatisch No-Op?**
   - What we know: D-05 sagt "einmalig beim Start"; das Filter `count_attachments==0` ist self-managing.
   - What's unclear: Ob nach `N` aufeinanderfolgenden Restart-Zyklen ohne Backfill-Erfolg ein Log-Statement wie "all candidates exhausted (X permanent failures)" sinnvoll wäre.
   - Recommendation: Aus Scope. Initial-Implementierung loggt nur `starting (N candidates)` und am Ende `done (Y persisted, Z skipped)`. Späterer Refinement, falls produktiver Bedarf.

3. **Wo legen wir die `format_size` Util ab — in `util/format.rs` (neu) oder direkt im Component-File?**
   - What we know: UI-SPEC §Formatting & States empfiehlt `src/util/format.rs` oder "co-locate" im Component.
   - What's unclear: Ob es schon eine `util/`-Konvention im Frontend gibt.
   - Recommendation: Neuer `util/format.rs`. Erleichtert Wiederverwendung (z. B. zukünftig auch für outbound-Attachments und Static-Documents) und macht Unit-Tests leichter isolierbar.

4. **In welchem Reihenfolge sollten Frontend- und Backend-Tasks geplant werden?**
   - What we know: TO-Schema (D-07) ist der API-Kontrakt zwischen Backend und Frontend.
   - What's unclear: Ob Backend-Tasks Frontend-Tasks blockieren (klassischer Vertical-Slice) oder ob Mock-Daten genutzt werden.
   - Recommendation: Backend zuerst, Frontend zweitens. Dazwischen ein kurzer "API-Smoke"-Test mit curl, der das embedded `attachments`-Feld in DetailTO + den Download-Handler beide grün zeigt. Component-Tests können später kommen.

---

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust toolchain | Backend build | ✓ | (verified via existing build) | — |
| SQLite | Backend persistence | ✓ | (verified via existing migrations) | — |
| `tokio` async runtime | Worker + REST | ✓ | 1.35+ (workspace) | — |
| `mail-parser` 0.9.4 | Attachment iteration | ✓ | 0.9.4 in `~/.cargo/registry/` | — |
| `async-imap` 0.10.4 | IMAP refetch | ✓ | 0.10.4 in `~/.cargo/registry/` | — |
| `DocumentStorage` filesystem path | Backend save/load | ✓ | env `DOCUMENT_STORAGE_PATH`, default `./documents` | If `DOCUMENT_STORAGE_PATH` unwritable: server panics on startup (already current behavior, `document_storage.rs:15`). Nicht Phase-19-spezifisch. |
| `dx serve` / Dioxus CLI | Frontend dev build | ✓ | Per `flake.nix` reproducible env | — |
| `npx tailwindcss` | Frontend CSS | ✓ | Per existing setup | — |
| IMAP test server | E2E test of backfill (optional) | ✗ | — | Mock `InboxImapClient` for unit/E2E tests — recommended. Existing `MockInboxImapClient` (`inbox.rs:113`) supports this; just expand for `fetch_one_by_uid`. |
| Live SMTP/IMAP (production) | Runtime polling | (deployment-dependent) | configured via `imap_*` config keys | If IMAP unreachable: Poll worker logs warn and continues; backfill silent-skips per D-06. |

**Missing dependencies with no fallback:** None.

**Missing dependencies with fallback:** IMAP test server — wird via Mock-Trait umgangen.

---

## Security Domain

> Required because no explicit `security_enforcement: false` is set in config. Light review for a frontend-driven, internal-admin feature with bounded threat surface.

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | yes | Bestehende session-cookie auth via `forbid_unauthenticated`-Middleware; D-09 reuses Vorstand-only via gleicher Auth-Pfad. |
| V3 Session Management | yes | Bestehendes `tower-sessions` / OIDC-Setup. No new session logic. |
| V4 Access Control | yes | D-09: Vorstand-only. Same gate as existing `GET /api/inbox/{id}`. KEINE neue Permission. |
| V5 Input Validation | yes | UUID-Parsing für `mail_id` + `attachment_id` (RestError::BadRequest bei Parse-Fehler). `?disposition` Query-Param wird via `Option<String>` deserialisiert, in `match` per `"inline"` vs default. Kein User-controlled Path. |
| V6 Cryptography | no (n/a) | Keine neuen Krypto-Operationen. |
| V8 Data Protection | yes | Vorstand-only sichtbar; mail-Attachments enthalten PII. Tracing-Logs dürfen Filename + size_bytes loggen, NICHT bytes. |
| V12 File and Resources | yes | Path-Traversal-Schutz via `FilesystemDocumentStorage::full_path` (siehe Pattern 3 oben). Filename im Content-Disposition wird via `http_util` percent-encoded (RFC 5987/6266) — kein header-injection. |
| V13 API and Web Service | yes | Keine GraphQL/REST-Schema-Drift; OpenAPI via Utoipa wird mit dem neuen Endpoint erweitert. |
| V14 Configuration | yes | Hard-konstante 10-MB-Limit (D-02); keine Env-Var-Konfiguration für die Konstante. |

### Known Threat Patterns for Inbox-Attachment-Download Endpoint

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Path Traversal via Filename | Tampering | `relative_path` wird im Backend deterministisch generiert (`inbound_mail_attachments/{uuid}/{uuid}`); kein User-Input ins Pfad-Format. `FilesystemDocumentStorage::full_path` clean + prefix-check. |
| Header Injection via Filename in Content-Disposition | Tampering | `http_util::content_disposition_attachment`/`_inline` percent-encodet alle Non-ASCII und `\r\n` (siehe `http_util.rs:53-63`). Unit-tested (`http_util.rs:146-153`). |
| Direct Access to other User's Attachments (IDOR) | Information Disclosure | DAO-Query `find_by_id_and_mail` verlangt BEIDE — `attachment_id` UND `mail_id`. Wenn `mail_id` nicht zur attachment passt → 404. Plus Vorstand-only-Permission begrenzt Surface. |
| Disk Exhaustion (Storage DoS) | DoS | Hard 10-MB-Limit per Attachment (D-02). Mailbox-quota auf IMAP-Server schützt zusätzlich. |
| Mail-Inbox Email-Bomb / Spam-Attachments | DoS | Limit pro Attachment (10 MB); kein Limit pro Mail. Pragmatisch ok — Spam-Flood durch Spam-Filter am IMAP-Server abgefangen. Out of scope, in `<deferred>` (kein Virenscan). |
| Reflected XSS via Filename in Frontend rendering | XSS | Dioxus's RSX-Text-Interpolation escaped HTML automatisch (`{att.file_name}` wird text-content, nicht HTML). Standard-Schutz. |
| Stored XSS via maliciously-crafted SVG attachment served inline | XSS | `<img src="…">` rendert SVG ABER nicht JavaScript-aware (SVG-Scripts laufen nur in HTML-Context, nicht in `<img>`). Pragmatisch ok für Inbox-MVP. Wenn Vorstand SVGs als Inline-Image lädt: kein Script-Execution. [VERIFIED: Browser default behavior — `<img>` triggert `image/svg+xml` als image, nicht als document.] |
| Cookie Theft via XSS / CSRF on Download Endpoint | Spoofing | Session-Cookie sollte `SameSite=Lax|Strict` haben (per axum-oidc default und tower-sessions setup, `lib.rs:733-735`). `<a href download>` ist GET-only — natürlich CSRF-resistent bei `SameSite=Lax`. |

**Verdict:** Phase 19 bringt KEINE neuen kategorischen Risiken über die bestehenden Inbox-Endpoints hinaus. Standard-Hygiene (Path-Traversal-Schutz, Header-Encoding, UUID-Validation) ist via wiederverwendete Helper bereits abgedeckt.

---

## Sources

### Primary (HIGH confidence)
- `mail-parser-0.9.4` cargo-registry-source: `core/message.rs:447, 462`, `core/header.rs:566-660, 588-712`, `lib.rs:617-649` — `Message::attachments()`, `MessagePart::contents()`, `MessagePart::is_message()`, `MimeHeaders::attachment_name()`, `MimeHeaders::content_type()`.
- `async-imap-0.10.4` cargo-registry-source: `client.rs:477-499` — `Session::uid_fetch(uid_set, query)`.
- `genossi_mail/src/inbox.rs` — full `parse_raw_mail` + `poll_once` + service trait + worker loop.
- `genossi_mail/src/inbox_imap.rs` — `AsyncImapClient`, `open_examine_session`, `fetch_since`.
- `genossi_mail/src/dao.rs` — `InboundMail`, `MailRecipientAttachment`, all relevant DAO traits.
- `genossi_mail/src/dao_sqlite.rs:359-435, 1130-1192` — `MailRecipientAttachmentDaoSqlite` + create-table patterns.
- `genossi_mail/src/inbox_rest.rs` — full REST handler suite + `InboxRestState`-Trait.
- `genossi_mail/src/static_document_service.rs:108-120, 209-251` — Save-then-DB Pattern + rollback-on-fail unit tests.
- `genossi_service/src/document_storage.rs` — Storage trait + error type.
- `genossi_service_impl/src/document_storage.rs:25-55, 60-93` — Path-traversal-protection + tests.
- `genossi_rest/src/member_document.rs:220-267` — Download-Handler-Vorbild.
- `genossi_rest/src/http_util.rs:43-80, 80-175` — Content-Disposition helper + unit tests.
- `genossi_bin/src/lib.rs:1078-1095, 1344-1351` — Inbox-Service-Wiring + Worker-Spawn-Vorbild.
- `genossi_bin/src/main.rs:30-65` — Entry-point migration → service-init → worker-spawn sequence.
- `genossi_bin/tests/e2e_tests.rs:4640-4810` — Inbox E2E-Test-Pattern via direct-DB-seed (`seed_inbound_mail`).
- `genossi-frontend/src/page/inbox_page.rs:303-340` — Detail-Pane mit zu ersetzendem MVP-Hinweis.
- `genossi-frontend/src/component/inbox/mail_list_item.rs` + `mod.rs` — Component-Stil-Vorbild + Registry.
- `genossi-frontend/src/i18n/mod.rs:46-... , 791` — Key-Enum + `t()`-Funktion.
- `genossi-frontend/src/api.rs:1349-1378` — Frontend TO Schema.
- `genossi-frontend/CLAUDE.md` — Component-First-Prinzip + i18n.
- `CLAUDE.md` (root) — Layered architecture + audit constraints.
- `migrations/sqlite/20260404000001_create_mail_recipient_attachments_table.sql` + `20260409000001_create_inbound_mails_table.sql` — Migration-Naming + DDL-Pattern.
- `19-CONTEXT.md` — Locked decisions D-01..D-14.
- `19-UI-SPEC.md` — Locked frontend design contract.

### Secondary (MEDIUM confidence)
- Memory: `feedback_dioxus_button_type.md` — Page-reload bug; confirmed via UI-SPEC pivot.
- Memory: `feedback_component_first.md` — Component-First-Prinzip; reaffirmed in `genossi-frontend/CLAUDE.md`.
- Memory: `feedback_use_jj_not_git.md` — Repository is jj; commits via `jj commit -m`.

### Tertiary (LOW confidence)
- None remaining — all critical claims verified via source code or official docs.

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — alle libraries bereits im Projekt eingesetzt, Versionen via Cargo-Registry verifiziert, APIs aus Source-Code gelesen.
- Architecture: HIGH — pattern komplett aus 1:1 wiederverwendbarem `MailRecipientAttachment` + `StaticDocument`-Code abgeleitet.
- Pitfalls: HIGH (1-6) / MEDIUM (7-8) — Pitfall 7 (Dioxus `loading` attr) und Pitfall 8 (Cookie-Forwarding bei `<a href>`) sind verbleibend zu prüfen während Frontend-Implementation.
- Security: HIGH — alle Mitigation-Pfade nutzen bestehende, unit-tested Helper.

**Research date:** 2026-06-07
**Valid until:** 2026-07-07 (30 Tage; Stack ist stabil: mail-parser/async-imap/Dioxus 0.6 ändern sich nicht im Wochenrhythmus)
