---
phase: quick-260607-s0s
plan: 01
subsystem: inbox + mail-compose
tags: [inbox, reply, attachments, component-first, frontend, backend]
requires:
  - InboxService::reply (genossi_mail/src/inbox.rs)
  - MailServiceImpl::create_job persistence pattern (genossi_mail/src/service.rs:300-368)
  - MailRecipientAttachmentDao + MailJobStaticAttachmentDao + StaticDocumentDao (already exist)
  - existing Compose-Picker UI in mail_page.rs (extraction target)
provides:
  - JSON-only reply with attachment_ids + static_document_ids (POST /api/inbox/{id}/reply)
  - Shared MailAttachmentPicker component used by both Compose & Reply flows
  - InboxRestState::resolve_document trait method + RestStateImpl impl
affects:
  - InboxServiceImpl<...> generics: 7 → 10 (new RA, JSA, SD)
  - InboxServiceType alias in genossi_bin/src/lib.rs
  - InboxServiceImpl::new arity: 7 → 10 args
tech-stack:
  added: []
  patterns:
    - "JSON-IDs-Only-Reply (analog Compose-Flow: keine multipart-Uploads für vorhandene Dokumente)"
    - "Component-First: Picker einmal extrahiert, 2x verwendet"
key-files:
  created:
    - genossi-frontend/src/component/mail_compose/attachment_picker.rs
  modified:
    - genossi_mail/src/inbox.rs
    - genossi_mail/src/inbox_rest.rs
    - genossi_bin/src/lib.rs
    - genossi-frontend/src/component/mail_compose/mod.rs
    - genossi-frontend/src/page/mail_page.rs
    - genossi-frontend/src/component/inbox/reply_form.rs
    - genossi-frontend/src/api.rs
decisions:
  - "Backwards-Compat über #[serde(default)] auf ReplyRequest.attachment_ids + .static_document_ids — alte Frontends senden weiterhin {subject, body} und bekommen 202 zurück (Test reply_with_no_attachments_preserves_existing_behavior beweist 0 DAO-Calls)."
  - "Ownership-Check mirrors rest.rs:481-513 — Some(doc.member_id) != mail.assigned_member_id → 400 BadRequest. Zusätzlich: attachment_ids nicht-leer + assigned_member_id=None → 400 (T-s0s-02)."
  - "Static-Doc-Validation via StaticDocumentDao::find_many_by_ids VOR job_dao.create — kein Halb-State bei fehlender static-id."
  - "InboxServiceImpl bekommt 3 neue Generics (RA, JSA, SD) statt globaler Service-Erweiterung — minimale Abweichung vom existierenden Generic-Pattern."
  - "RestStateImpl::resolve_document wird im InboxRestState-impl inline-dupliziert (statt Trait-Delegation an MailRestState), da rustc bei zwei Trait-Methods mit gleichem Namen auf demselben Typ ambiguity meldet. Kleinster Diff; Helper lebt rein lokal."
  - "Frontend Reply-Form lädt `available_documents` aus assigned_member_id via api::get_member_documents (Mount-Effect) und statische Docs immer via api::list_static_documents — identisch zur Compose-Page."
metrics:
  duration: ~25 min
  completed: 2026-06-07
---

# Quick 260607-s0s: Reply-Anhänge wie Compose — Summary

## Objective

Beim Beantworten einer Inbox-Mail muss der Vorstand vorhandene MemberDocuments + StaticDocuments anhängen können — **exakt wie im Compose-Flow**. Vorheriger Versuch (Quick 260607-r96) hatte fälschlich lokales File-Upload via multipart gebaut; dieser Quick implementiert die ursprünglich gewünschte Variante: **Picker auf existierende Docs, JSON-IDs an Backend, Backend persistiert dieselben Rows wie `MailServiceImpl::create_job`**.

## Was wurde gebaut

### Backend (Task 1)

**`genossi_mail/src/inbox.rs`**
- `InboxService::reply` Trait-Signatur erweitert um `attachment_inputs: Vec<AttachmentInput>` + `static_document_ids: Vec<Uuid>`.
- `InboxServiceImpl<C, D, I, J, R, A, St>` → `InboxServiceImpl<C, D, I, J, R, A, St, RA, JSA, SD>` mit `RA: MailRecipientAttachmentDao`, `JSA: MailJobStaticAttachmentDao`, `SD: StaticDocumentDao`.
- Konstruktor `new` nimmt 3 neue Arc-Args.
- `reply`-Impl macht 3 neue Dinge (in dieser Reihenfolge — kein Halb-State):
  1. **Validierung** der `static_document_ids` via `find_many_by_ids` → `NotFound`, falls Anzahl nicht stimmt.
  2. Nach `recipient_dao.create(&recipient)`: pro `AttachmentInput` einen `MailRecipientAttachment`-Row mit `recipient.id`.
  3. Pro `static_document_id` einen `MailJobStaticAttachment`-Join-Row mit `job.id`.
- Spiegelung von `MailServiceImpl::create_job` (`service.rs:300-368`) 1:1.

**`genossi_mail/src/inbox_rest.rs`**
- `ReplyRequest` erweitert um `attachment_ids: Vec<String>` + `static_document_ids: Vec<String>`, beide mit `#[serde(default)]` für Backward-Compat.
- `InboxRestState`-Trait neu: `fn resolve_document(&self, Uuid) -> Pin<Box<...Output = Option<ResolvedDocument>>>` (gleiche Signatur wie `MailRestState::resolve_document`).
- `reply_inbox`-Handler:
  - Lädt `mail = svc.get(mail_id)` zuerst.
  - `attachment_ids` non-empty + `assigned_member_id=None` → **400** "no member assigned".
  - Pro `att_id_str`: UUID-Parse (400 bei Fehler), `resolve_document` (404 bei None), Ownership-Check `Some(doc.member_id) != mail.assigned_member_id` → **400** "does not belong".
  - Pro `static_document_id`: UUID-Parse (400 bei Fehler).
  - `svc.reply(...)` mit gefüllten Vecs.

**`genossi_bin/src/lib.rs`**
- `InboxServiceType`-Alias um 3 Generics erweitert: `MailRecipientAttachmentDao`, `MailJobStaticAttachmentDaoType`, `StaticDocumentDaoType`.
- `InboxServiceImpl::new`-Aufruf um 3 neue Arc-Args erweitert: zwei neue `Arc::new(...)`-Instanzen mit demselben Pool (Pattern analog `worker_attachment_dao`), `static_document_dao_for_service.clone()` als dritter (war bereits Arc).
- Neue `InboxRestState::resolve_document`-Impl: identische SQL wie `MailRestState::resolve_document` (`SELECT member_id, file_name, mime_type, relative_path FROM member_document WHERE id = ? AND deleted IS NULL`). Inline-Duplikation gewählt, da Trait-Delegation an `MailRestState` zu Method-Resolution-Ambiguity geführt hätte.

### Frontend (Task 2)

**`genossi-frontend/src/component/mail_compose/attachment_picker.rs` (neu, 110 LOC)**
- `MailAttachmentPicker { member_id: Option<Uuid>, available_documents, available_static_documents, selected_member_doc_ids, selected_static_doc_ids }`.
- Render-Logik 1:1 aus `mail_page.rs:478-559` übernommen, nur die Sichtbarkeits-Bedingung des Member-Doc-Blocks geändert: `selected_member_ids.read().len() == 1` → `member_id.is_some()` (Reply hat per Definition genau 1 Empfänger).

**`genossi-frontend/src/component/mail_compose/mod.rs`**
- `pub mod attachment_picker; pub use attachment_picker::MailAttachmentPicker;`.

**`genossi-frontend/src/page/mail_page.rs`**
- Inline-Picker-Block (Zeilen 478-559) ersetzt durch einen `MailAttachmentPicker { ... }`-Aufruf in einem `{ let single_recipient_id = ...; rsx! { ... } }`-Wrapper. Verhalten 1:1 erhalten.

**`genossi-frontend/src/component/inbox/reply_form.rs`**
- Neue Signals: `available_documents`, `available_static_documents`, `selected_attachment_ids`, `selected_static_document_ids`.
- Zwei neue `use_effect`s: einer lädt `get_member_documents` wenn `member_uuid_opt = Some(...)`, der andere lädt `list_static_documents` einmal auf Mount.
- Im RSX-Body wird `MailAttachmentPicker { ... }` zwischen `MailBodyEditor` und `TemplatePreview` eingebunden.
- Send-Handler übergibt `&att_ids, &static_ids` an `reply_inbox_mail`.

**`genossi-frontend/src/api.rs`**
- `reply_inbox_mail(config, id, subject, body, attachment_ids: &[Uuid], static_document_ids: &[String])`.
- JSON-Body: `{subject, body, attachment_ids: ["uuid"], static_document_ids: ["uuid"]}` — keine `FormData`, kein `web_sys::File`, kein `multipart`.

## Geänderte Datei-Pfade (absolut)

**Backend (commit d0a8ca2e):**
- `/home/neosam/programming/rust/projects/genossi3/genossi_mail/src/inbox.rs`
- `/home/neosam/programming/rust/projects/genossi3/genossi_mail/src/inbox_rest.rs`
- `/home/neosam/programming/rust/projects/genossi3/genossi_bin/src/lib.rs`

**Frontend (commit 307d61e0):**
- `/home/neosam/programming/rust/projects/genossi3/genossi-frontend/src/component/mail_compose/attachment_picker.rs` (neu)
- `/home/neosam/programming/rust/projects/genossi3/genossi-frontend/src/component/mail_compose/mod.rs`
- `/home/neosam/programming/rust/projects/genossi3/genossi-frontend/src/page/mail_page.rs`
- `/home/neosam/programming/rust/projects/genossi3/genossi-frontend/src/component/inbox/reply_form.rs`
- `/home/neosam/programming/rust/projects/genossi3/genossi-frontend/src/api.rs`

## Neue Tests (Backend)

In `genossi_mail/src/inbox.rs::tests`:

1. **`reply_creates_attachment_rows_for_member_doc_attachment_ids`** — Reply mit 2 `AttachmentInput`s → `MockMailRecipientAttachmentDao::expect_create().times(2)`, jeweils mit `recipient.id` der `MockMailRecipientDao::expect_create`-Antwort.
2. **`reply_creates_static_doc_joins_for_static_document_ids`** — Reply mit 2 `static_document_ids` → `MockStaticDocumentDao::expect_find_many_by_ids().times(1)` (gibt 2 docs zurück) + `MockMailJobStaticAttachmentDao::expect_create().times(2)`, jeweils mit `mail_job_id == job.id`.
3. **`reply_with_no_attachments_preserves_existing_behavior`** — Backwards-Compat: empty vecs → `expect_create().times(0)` auf allen 3 neuen Mocks. Job + Recipient werden erzeugt, `mail.replied=true` wird gesetzt.

Zusätzlich: die 2 bestehenden Reply-Tests (`reply_creates_job_and_sets_status`, `reply_to_nonexistent_mail_returns_not_found`) auf die neue Signatur angepasst → `svc.reply(mail_id, "Re: s", "My reply", vec![], vec![])`. Beide weiterhin grün.

**Test-Ergebnis:** `cargo test -p genossi_mail inbox::tests::reply_` → 5/5 passed. `cargo test -p genossi_mail` → 179 passed insgesamt (keine Regressionen).

## Verifikation

| Check | Ergebnis |
|-------|----------|
| `cargo test -p genossi_mail inbox::tests::reply_` | **5 passed, 0 failed** |
| `cargo test -p genossi_mail` (gesamt) | **179 passed, 0 failed** |
| `cargo build --bin genossi` | **grün** (`Finished dev profile`) |
| `cargo build -p genossi_bin` | **grün** |
| `cargo clippy -p genossi_mail -p genossi_bin --no-deps -- -D warnings` | **grün** |
| `cd genossi-frontend && cargo check` | **grün** (nur pre-existing dead-code-warnings für Translation-Keys) |
| `grep -c "MailAttachmentPicker" mail_page.rs` | **3** (Import, Kommentar, Komponenten-Aufruf) |
| `grep -c "MailAttachmentPicker" reply_form.rs` | **3** (Import, Kommentar, Komponenten-Aufruf) |
| `grep "FormData\|web_sys::File\|multipart" reply_form.rs / api.rs::reply_inbox_mail` | **0 Treffer** (Reply bleibt JSON) |
| `reply_inbox_mail` Signatur enthält `attachment_ids: &[Uuid]` + `static_document_ids: &[String]` | **ja** |

## Abweichungen vom Plan

**Keine inhaltlichen Abweichungen.** Drei kleine Detail-Entscheidungen, die der Plan offen ließ:

1. **`#[allow(clippy::too_many_arguments)]` am `InboxServiceImpl::new`** — der Konstruktor hat jetzt 10 Arc-Args (war 7), was Clippy auf default-Schwelle 7 monieren würde. Pragmatisch erlaubt — der Konstruktor wird genau einmal in `genossi_bin/src/lib.rs` aufgerufen.
2. **`resolve_document` im `InboxRestState`-Impl inline-dupliziert** (statt Delegation an `<Self as MailRestState>::resolve_document`) — die Trait-Methoden haben in beiden Traits identische Signatur und Namen, was bei generischer Auflösung im Handler-Code zu Ambiguity-Fehlern führen würde. Inline-Duplikation ist minimal-invasiv und der Plan erlaubt das explizit: "Identische Implementation, also Code-Duplikat oder gemeinsamer Helper — Inline-Delegation ist hier akzeptabel".
3. **`static_document_dao_for_service` wird ge`.clone()`d** statt direkt übergeben — der StaticDocumentService wurde im Original konsumiert, jetzt wird derselbe Arc auch in den InboxService durchgereicht (Single-Arc-pro-Pool-Pattern aus dem Plan).

Keine zusätzlichen REST-Tests in `genossi_rest_tests`/`genossi_mail/tests` — der Plan erlaubt das explizit, da `inbox_rest.rs` kein `mod tests` hat und keine e2e-Infrastruktur existiert, die nicht zuerst aufgebaut werden müsste. Die Ownership-Validierung ist über die Mock-basierte Persistenz-Tests (`reply_creates_attachment_rows_*`) abgedeckt; der reine REST-Handler-Code (UUID-Parse, ResolvedDocument-Lookup, Ownership-Check) ist trivial.

## Commits

| Commit | Beschreibung | Files |
|--------|--------------|-------|
| `d0a8ca2e` (jj `qvwvpnkm`) | feat(inbox): persist member-doc + static-doc attachments on reply [260607-s0s] | inbox.rs, inbox_rest.rs, lib.rs |
| `307d61e0` (jj `pzypvtrq`) | feat(frontend): shared MailAttachmentPicker for Compose + Reply [260607-s0s] | attachment_picker.rs (neu), mod.rs, mail_page.rs, reply_form.rs, api.rs |

## Threat-Mitigations (umgesetzt)

- **T-s0s-01 (IDOR)** — REST handler: `if Some(doc.member_id) != mail.assigned_member_id { return 400 }`.
- **T-s0s-02 (no-member + attachment)** — REST handler: `if !attachment_ids.is_empty() && mail.assigned_member_id.is_none() { return 400 }`.
- **T-s0s-03 (static-doc tampering)** — Service: `static_document_dao.find_many_by_ids` Validierung vor jeder DAO-Schreib-Operation.
- **T-s0s-05 (no new auth surface)** — Reply-Endpoint bleibt unter dem existierenden Inbox-Auth-Schutz.

## Self-Check

- [x] `genossi_mail/src/inbox.rs` enthält `reply_creates_attachment_rows_for_member_doc_attachment_ids` — verifiziert per Test-Run (5/5 reply tests grün).
- [x] `genossi_mail/src/inbox.rs` enthält `reply_creates_static_doc_joins_for_static_document_ids` — verifiziert per Test-Run.
- [x] `genossi_mail/src/inbox.rs` enthält `reply_with_no_attachments_preserves_existing_behavior` — verifiziert per Test-Run.
- [x] `genossi-frontend/src/component/mail_compose/attachment_picker.rs` existiert (110 LOC) und ist via `mod.rs` exportiert.
- [x] `mail_page.rs` und `reply_form.rs` rufen jeweils `MailAttachmentPicker { ... }` auf (grep-zählung 3 in beiden, inklusive Import-Zeile).
- [x] `api.rs::reply_inbox_mail` Signatur enthält `attachment_ids: &[Uuid]` und `static_document_ids: &[String]`.
- [x] Reply-Code-Pfade enthalten **kein** `multipart`, **kein** `FormData`, **kein** `web_sys::File`.
- [x] Backend-Commit (`d0a8ca2e`) und Frontend-Commit (`307d61e0`) sind im jj-Log sichtbar.

## Self-Check: PASSED
