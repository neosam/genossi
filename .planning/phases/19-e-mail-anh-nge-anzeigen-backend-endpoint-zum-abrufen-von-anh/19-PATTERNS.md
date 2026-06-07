# Phase 19: E-Mail-Anhänge anzeigen — Pattern Map

**Mapped:** 2026-06-07
**Files analyzed:** 22 (14 new + 8 modify)
**Analogs found:** 22 / 22 (100% match coverage — kein No-Analog-Bereich)

> Downstream-Konsumenten (Planner, Executor): Jede Datei unten hat einen
> **read_first**-Pointer in Form `path:lineStart-lineEnd`. Bevor Code
> geschrieben wird, exakt diesen Bereich lesen und das Pattern spiegeln.

---

## File Classification

| File (new=N / modify=M) | Role | Data Flow | Closest Analog | Match Quality |
|-------------------------|------|-----------|----------------|---------------|
| N `migrations/sqlite/20260608000000_create_inbound_mail_attachments_table.sql` | Migration | DDL | `migrations/sqlite/20260404000001_create_mail_recipient_attachments_table.sql` | role+flow exact |
| M `genossi_mail/src/dao.rs` (+ `InboundMailAttachment` + DAO trait) | DAO trait | CRUD | Existing `MailRecipientAttachment` block at `genossi_mail/src/dao.rs:88-105` | role+flow exact |
| M `genossi_mail/src/dao_sqlite.rs` (+ `InboundMailAttachmentDaoSqlite`) | DAO impl | CRUD | `MailRecipientAttachmentDaoSqlite` at `genossi_mail/src/dao_sqlite.rs:359-435` + test schema at `:1130-1141` | role+flow exact |
| M `genossi_mail/src/inbox.rs` (extend `parse_raw_mail`, add `ParsedAttachment`, add `persist_attachment`, add `run_attachment_backfill`) | Service | Stream + transform | Existing `parse_raw_mail` at `inbox.rs:165-260` + `InboxService` trait at `:266-283` + RESEARCH Pattern 1 (atomic save+DB at `static_document_service.rs:108-120`) | role+flow exact |
| M `genossi_mail/src/inbox_imap.rs` (add trait method `fetch_one_by_uid` + impl) | Service (IMAP client) | Request-response | Existing `fetch_since` impl at `inbox_imap.rs:115-147` + trait method at `inbox.rs:122-126` | role+flow exact |
| M `genossi_mail/src/inbox_rest.rs` (extend `InboundMailDetailTO` with `attachments`; add `download_attachment` handler; register route) | REST | Request-response + binary download | `to_detail_to` at `inbox_rest.rs:91-106` (TO conversion) + `download_document` at `genossi_rest/src/member_document.rs:232-267` (binary download) | role+flow exact |
| M `genossi_rest/src/http_util.rs` (add `content_disposition_inline`) | Helper | Pure transform | Existing `content_disposition_attachment` at `http_util.rs:43-50` + sanitizer at `:53-63` + tests at `:82-175` | role+flow exact |
| M `genossi_bin/src/lib.rs` (type alias `InboundMailAttachmentDaoType`; field `inbound_attachment_dao` on `RestStateImpl`; ctor wiring; `start_attachment_backfill_worker`) | Wiring | DI | Existing `worker_inbox_*` wiring at `lib.rs:580-1095, 1344-1351` (full pipeline: type-alias → field → ctor → spawn) | role+flow exact |
| M `genossi_bin/src/main.rs` (spawn backfill after `start_inbox_worker`) | Wiring | Bootstrap | Pattern: existing `rest_state.start_inbox_worker();` line (search same file) | role+flow exact |
| N `genossi-frontend/src/component/inbox/attachment_list.rs` | Frontend Component | Render | `genossi-frontend/src/component/inbox/mail_list_item.rs` (whole file, 43 lines) | role+flow exact |
| N `genossi-frontend/src/component/inbox/attachment_list_item.rs` | Frontend Component | Render | `genossi-frontend/src/component/inbox/mail_list_item.rs` (props + RSX skeleton) | role+flow exact |
| M `genossi-frontend/src/component/inbox/mod.rs` (register two new components) | Frontend Registry | Re-export | Existing `mod.rs` (whole file, 7 lines) | role+flow exact |
| M `genossi-frontend/src/page/inbox_page.rs` (delete `:331-335` MVP-hint; insert `InboxAttachmentList` between `:347 pre`-body and `:350 border-t`) | Frontend Page | Compose | The page already composes other inbox components (e.g. `InboxStatusBadge { … }` at `:329` and `:330`) | role+flow exact |
| M `genossi-frontend/src/api.rs` (extend `InboundMailDetailTO` with `attachments`; add `InboundMailAttachmentTO`; add `attachment_download_url` / `attachment_inline_url`) | Frontend API client | Transport | Existing `InboundMailDetailTO` at `api.rs:1364-1378` + URL-builder pattern at `:1397-1401` | role+flow exact |
| N `genossi-frontend/src/util/format.rs` | Frontend Util | Pure transform | No existing `util/` directory — **no in-repo analog**. Fallback: pattern is fully spec'd by RESEARCH `Size Formatter` code block (`19-RESEARCH.md` after line 992). | spec-driven |
| M `genossi-frontend/src/util/mod.rs` (declare `pub mod format;`) | Frontend Util Registry | Re-export | No `util/` dir yet — file is new and must be created. | spec-driven |
| M `genossi-frontend/src/main.rs` or `lib.rs` (declare `pub mod util;` after other module declarations) | Frontend Bootstrap | Module-Declaration | Pattern: existing `pub mod i18n;` / `pub mod component;` etc. (1-line addition) | role+flow exact |
| M `genossi-frontend/src/i18n/mod.rs` (add 7 `Key::InboxAttachments*` variants) | Frontend i18n | Enum extend | Existing `Key` enum at `i18n/mod.rs:45-100+`, look at `OpenInboxCount` / `OpenInboxNone` at `:504-505` as positional anchor | role+flow exact |
| M `genossi-frontend/src/i18n/de.rs` (7 De-translations) | Frontend i18n | Translation table | Existing `Key::OpenInboxCount` / `Key::OpenInboxNone` translations at `de.rs:436-437` | role+flow exact |
| M `genossi-frontend/src/i18n/en.rs` (7 En-translations) | Frontend i18n | Translation table | Symmetric to `de.rs:436-437` (find same anchor in `en.rs`) | role+flow exact |
| M `genossi_mail/src/inbox.rs` (`MockInboxImapClient` must auto-gain new `fetch_one_by_uid` method) | Test scaffolding | Mock | `#[automock]` macro at `inbox.rs:113` auto-generates Mock; just add method to trait → mock follows. | role+flow exact |
| M `genossi_bin/tests/e2e_tests.rs` (extend `seed_inbound_mail` helper at `:4646`; add E2E test for download endpoint) | E2E test | Seed + roundtrip | `seed_inbound_mail` at `e2e_tests.rs:4640-4810` (direct-DB-seed pattern) | role+flow exact |

---

## Pattern Assignments

### 1. `genossi_mail/src/dao.rs` — Add `InboundMailAttachment` struct + DAO trait

**read_first:** `genossi_mail/src/dao.rs:88-105` (existing `MailRecipientAttachment` + `MailRecipientAttachmentDao` trait — exact analog)

**Existing struct shape to copy** (lines 88-95):
```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MailRecipientAttachment {
    pub recipient_id: Uuid,
    pub document_id: Uuid,
    pub file_name: Arc<str>,
    pub mime_type: Arc<str>,
    pub relative_path: Arc<str>,
}
```

**Existing trait shape to copy** (lines 97-105):
```rust
#[automock]
#[async_trait]
pub trait MailRecipientAttachmentDao: Send + Sync + 'static {
    async fn create(&self, attachment: &MailRecipientAttachment) -> Result<(), MailDaoError>;
    async fn find_by_recipient_id(
        &self,
        recipient_id: Uuid,
    ) -> Result<Arc<[MailRecipientAttachment]>, MailDaoError>;
}
```

**New code to add** (next to the existing `MailRecipientAttachment` block):
```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InboundMailAttachment {
    pub id: Uuid,
    pub inbound_mail_id: Uuid,
    pub created: time::PrimitiveDateTime,
    pub file_name: Arc<str>,
    pub mime_type: Arc<str>,
    pub size_bytes: i64,
    pub relative_path: Option<Arc<str>>,   // NULL when oversized=true (D-02)
    pub oversized: bool,                   // D-02 hard 10 MB cap marker
}

#[automock]
#[async_trait]
pub trait InboundMailAttachmentDao: Send + Sync + 'static {
    async fn create(&self, attachment: &InboundMailAttachment) -> Result<(), MailDaoError>;
    async fn find_by_inbound_mail_id(
        &self,
        inbound_mail_id: Uuid,
    ) -> Result<Arc<[InboundMailAttachment]>, MailDaoError>;
    async fn find_by_id_and_mail(
        &self,
        mail_id: Uuid,
        attachment_id: Uuid,
    ) -> Result<Option<InboundMailAttachment>, MailDaoError>;
    async fn count_for_mail(&self, mail_id: Uuid) -> Result<i64, MailDaoError>;
}
```

**Notes:**
- No `version` / `deleted` fields — same as `MailRecipientAttachment` (`dao.rs:88-95`). Read-only entity.
- Not auditable (D-10) — do NOT implement `Auditable` trait.

---

### 2. `genossi_mail/src/dao_sqlite.rs` — `InboundMailAttachmentDaoSqlite`

**read_first:** `genossi_mail/src/dao_sqlite.rs:359-435` (analog impl) + `:1130-1141` (test-only CREATE TABLE) + `:32` for `format_datetime`/`parse_datetime` helpers

**Existing impl shape to copy** (lines 359-435):
```rust
#[derive(Debug, sqlx::FromRow)]
struct MailRecipientAttachmentDb {
    recipient_id: Vec<u8>,
    document_id: Vec<u8>,
    file_name: String,
    mime_type: String,
    relative_path: String,
}

impl TryFrom<&MailRecipientAttachmentDb> for MailRecipientAttachment {
    type Error = MailDaoError;
    fn try_from(db: &MailRecipientAttachmentDb) -> Result<Self, Self::Error> {
        Ok(MailRecipientAttachment {
            recipient_id: parse_uuid(&db.recipient_id)?,
            document_id: parse_uuid(&db.document_id)?,
            file_name: Arc::from(db.file_name.as_str()),
            mime_type: Arc::from(db.mime_type.as_str()),
            relative_path: Arc::from(db.relative_path.as_str()),
        })
    }
}

pub struct MailRecipientAttachmentDaoSqlite { pool: Arc<SqlitePool> }
impl MailRecipientAttachmentDaoSqlite { pub fn new(pool: Arc<SqlitePool>) -> Self { Self { pool } } }

#[async_trait]
impl MailRecipientAttachmentDao for MailRecipientAttachmentDaoSqlite {
    async fn create(&self, a: &MailRecipientAttachment) -> Result<(), MailDaoError> {
        sqlx::query(
            "INSERT INTO mail_recipient_attachments \
             (recipient_id, document_id, file_name, mime_type, relative_path) \
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(a.recipient_id.as_bytes().to_vec())
        .bind(a.document_id.as_bytes().to_vec())
        .bind(a.file_name.as_ref())
        .bind(a.mime_type.as_ref())
        .bind(a.relative_path.as_ref())
        .execute(self.pool.as_ref())
        .await
        .map_err(|e| MailDaoError::DatabaseError(Arc::from(e.to_string())))?;
        Ok(())
    }
    /* …find_by_recipient_id… */
}
```

**Key differences for new impl** (already pre-rendered in RESEARCH at lines 711-805):
- Add `id: Vec<u8>` PK + `inbound_mail_id: Vec<u8>` FK + `created: String` (use `format_datetime(&a.created)?` + `parse_datetime(&db.created)`).
- Add `size_bytes: i64` + `relative_path: Option<String>` (nullable) + `oversized: i64` (mapped `!= 0` ↔ bool).
- Implement three methods: `create`, `find_by_inbound_mail_id`, `find_by_id_and_mail`, `count_for_mail`.
- The exact 9-line `INSERT INTO` block, the `try_from`, and the `SELECT … WHERE inbound_mail_id = ?` are reproduced verbatim in RESEARCH `Code Examples → DAO: SQLite Implementation Pattern` (`19-RESEARCH.md` lines 711-805) — copy from there.

**Test schema to add** (sibling of `dao_sqlite.rs:1130-1141`):
```rust
sqlx::query(
    "CREATE TABLE inbound_mail_attachments (
        id BLOB PRIMARY KEY NOT NULL,
        inbound_mail_id BLOB NOT NULL REFERENCES inbound_mails(id),
        created TEXT NOT NULL,
        file_name TEXT NOT NULL,
        mime_type TEXT NOT NULL,
        size_bytes INTEGER NOT NULL,
        relative_path TEXT,
        oversized INTEGER NOT NULL DEFAULT 0
    )",
)
.execute(&pool)
.await
.expect("Failed to create inbound_mail_attachments table");
```

Insert directly after the `inbound_mails` table block at `dao_sqlite.rs:1167-1192` so unit tests can roundtrip the new entity.

---

### 3. `migrations/sqlite/20260608000000_create_inbound_mail_attachments_table.sql`

**read_first:** `migrations/sqlite/20260404000001_create_mail_recipient_attachments_table.sql` (whole file, 9 lines) AND `migrations/sqlite/20260409000001_create_inbound_mails_table.sql` (whole file, 22 lines)

**Existing recipient-attachment migration to mirror** (whole file):
```sql
CREATE TABLE IF NOT EXISTS mail_recipient_attachments (
    recipient_id BLOB NOT NULL REFERENCES mail_recipients(id),
    document_id BLOB NOT NULL REFERENCES member_document(id),
    file_name TEXT NOT NULL,
    mime_type TEXT NOT NULL,
    relative_path TEXT NOT NULL,
    PRIMARY KEY (recipient_id, document_id)
);
```

**Existing inbound_mails migration shows index-style** (whole file at `inbound_mails`-migration):
```sql
CREATE INDEX idx_inbound_mails_status ON inbound_mails(status);
```

**Filename rule:** Latest migration in tree is `20260603100000_mail_job_attach_repayment_letter.sql`. Use timestamp **strictly after** newest → `20260608000000_create_inbound_mail_attachments_table.sql` is well-formed (Phase 19 research date 2026-06-07; pick `20260608000000` or later).

**Content** (full file):
```sql
CREATE TABLE IF NOT EXISTS inbound_mail_attachments (
    id BLOB PRIMARY KEY NOT NULL,
    inbound_mail_id BLOB NOT NULL REFERENCES inbound_mails(id),
    created TEXT NOT NULL,
    file_name TEXT NOT NULL,
    mime_type TEXT NOT NULL,
    size_bytes INTEGER NOT NULL,
    relative_path TEXT,
    oversized INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX idx_inbound_mail_attachments_mail ON inbound_mail_attachments(inbound_mail_id);
```

---

### 4. `genossi_mail/src/inbox.rs` — Extend `parse_raw_mail`, add `ParsedAttachment`, `InboxService` methods, `run_attachment_backfill`

**read_first:**
- Existing `ParsedMail` struct at `inbox.rs:142-154`
- Existing `parse_raw_mail` at `inbox.rs:165-260` (especially the `has_attachments` line at `:208`)
- Existing `InboxService` trait at `inbox.rs:266-283` (extend with `find_attachment`)
- Existing `InboxImapClient` trait at `inbox.rs:113-136` (add `fetch_one_by_uid` here)
- Existing `InboxServiceImpl` struct at `inbox.rs:285-322`
- Existing `start_inbox_worker` function in same file (search for `pub async fn start_inbox_worker`)

**Critical extension target — `parse_raw_mail` line 208:**
```rust
let has_attachments = msg.attachment_count() > 0;   // OLD
```
Replace by populating a `Vec<ParsedAttachment>` (use code from `19-RESEARCH.md` lines 656-705, function `extract_attachments`, verbatim).

**New struct to add (next to `ParsedMail` at `:142`):**
```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedAttachment {
    pub file_name: String,
    pub mime_type: String,
    pub bytes: Vec<u8>,
}
```

**Trait additions (extend `InboxService` at `:266-283`):**
```rust
async fn find_attachment(
    &self,
    mail_id: Uuid,
    attachment_id: Uuid,
) -> Result<Option<crate::dao::InboundMailAttachment>, MailServiceError>;

async fn list_attachments(
    &self,
    mail_id: Uuid,
) -> Result<Arc<[crate::dao::InboundMailAttachment]>, MailServiceError>;
```

**InboxServiceImpl generic param extension:** Add `A: InboundMailAttachmentDao` and `St: DocumentStorage` to the `<C, D, I, J, R, A, St>` generic list (mirror the `<C, D, I, J, R>` shape at `:285-298`).

**New helper `persist_attachment` to add (private to the inbox module):**
Use RESEARCH Pattern 1 verbatim (`19-RESEARCH.md` lines 273-326). Key shape:
```rust
const ATTACHMENT_MAX_BYTES: u64 = 10 * 1024 * 1024; // D-02

async fn persist_attachment(
    storage: &dyn DocumentStorage,
    dao: &dyn InboundMailAttachmentDao,
    inbound_mail_id: Uuid,
    file_name: &str,
    mime_type: &str,
    bytes: &[u8],
) -> Result<InboundMailAttachment, MailServiceError> { /* save → DB → rollback on DB-fail */ }
```

**Source pattern for atomic save:** `genossi_mail/src/static_document_service.rs:108-120`:
```rust
// Filesystem first, DB second. On DB failure, attempt to clean up file.
let rel_path = doc.relative_path();
self.storage
    .save(&rel_path, &upload.data)
    .await
    .map_err(|e| StaticDocumentError::Storage(Arc::from(e.to_string())))?;

if let Err(e) = self.dao.create(&doc).await {
    let _ = self.storage.delete(&rel_path).await;
    return Err(e.into());
}
```

**New free function `run_attachment_backfill`:** body from RESEARCH Pattern 6 (`19-RESEARCH.md` lines 519-541), with: iterate `inbound_mails` where `has_attachments=true` AND `count_for_mail==0` → `imap_client.fetch_one_by_uid(uid_validity, imap_uid)` → `parse_raw_mail` → `persist_attachment` loop. `tracing::warn!` on every failure, `continue`. Log `starting (N candidates)` + `done (Y persisted, Z skipped)`.

---

### 5. `genossi_mail/src/inbox_imap.rs` — Add `fetch_one_by_uid` method

**read_first:** `genossi_mail/src/inbox_imap.rs:115-147` (existing `fetch_since` impl, which is the closest range-fetch analog) + `:70-80` (`open_examine_session` helper)

**Existing `fetch_since` shape to mirror** (lines 115-147):
```rust
async fn fetch_since(
    &self,
    config: &ImapConfig,
    min_uid: i64,
) -> Result<Vec<FetchedMessage>, MailServiceError> {
    let (mut session, _mailbox) = open_examine_session(config).await?;

    let start = (min_uid + 1).max(1);
    let range = format!("{}:*", start);

    let stream = session
        .uid_fetch(range, "(UID BODY.PEEK[])")
        .await
        .map_err(|e| err(format!("IMAP uid_fetch: {}", e)))?;

    let messages: Vec<_> = stream.collect().await;
    let mut out = Vec::new();
    for item in messages {
        let fetch = item.map_err(|e| err(format!("IMAP fetch item: {}", e)))?;
        let uid = match fetch.uid {
            Some(u) => u as i64,
            None => continue,
        };
        if uid <= min_uid { continue; }
        let raw = fetch.body().map(|b| b.to_vec()).unwrap_or_default();
        out.push(FetchedMessage { uid, raw });
    }

    let _ = session.logout().await;
    Ok(out)
}
```

**New trait method + impl:** Use RESEARCH Pattern 2 (`19-RESEARCH.md` lines 336-388) verbatim. Key differences:
- Add `expected_uid_validity: i64` parameter and check `mailbox.uid_validity` after `open_examine_session`; mismatch → `Err` (caller silent-skips per D-06).
- Use `format!("{}", uid)` as the `uid_set` argument (single-UID).
- Return `Result<Option<FetchedMessage>, MailServiceError>` (None if UID not present).

**Trait declaration goes into `genossi_mail/src/inbox.rs` at the `InboxImapClient` trait (lines 113-136).** Adding a method to a `#[automock]`-decorated trait auto-extends `MockInboxImapClient` (`inbox.rs:113`); no extra mock file edit required.

---

### 6. `genossi_mail/src/inbox_rest.rs` — Extend DetailTO + add download handler

**read_first:**
- Existing `InboundMailDetailTO` at `inbox_rest.rs:37-51`
- Existing `to_detail_to` at `inbox_rest.rs:91-106`
- Existing `get_inbox` handler at `inbox_rest.rs:170-195` (auth/permission pattern)
- Existing `InboxRestState` trait at `inbox_rest.rs:112-120` (must extend with `inbound_attachment_dao()` + `document_storage()`)
- Download handler analog: `genossi_rest/src/member_document.rs:232-267`

**Existing `to_detail_to` to extend** (lines 91-106):
```rust
fn to_detail_to(mail: &InboundMail, assigned_name: Option<String>) -> InboundMailDetailTO {
    InboundMailDetailTO {
        id: mail.id.to_string(),
        from_address: mail.from_address.to_string(),
        subject: mail.subject.to_string(),
        received_at: format_dt(&mail.received_at),
        body_text: mail.body_text.to_string(),
        has_attachments: mail.has_attachments,
        has_html_body: mail.has_html_body,
        replied: mail.replied,
        done: mail.done,
        archived: mail.archived,
        assigned_member_id: mail.assigned_member_id.map(|id| id.to_string()),
        assigned_member_name: assigned_name,
    }
}
```

**Required changes:**
- Add new TO `InboundMailAttachmentTO` with fields `{ id, file_name, mime_type, size_bytes, oversized }` (D-07).
- Add field `pub attachments: Vec<InboundMailAttachmentTO>` to `InboundMailDetailTO`.
- Add free fn `to_attachment_to(&InboundMailAttachment) -> InboundMailAttachmentTO`.
- Change `to_detail_to` to take `attachments: Vec<InboundMailAttachmentTO>` as 3rd arg.
- Update `get_inbox` handler (`:180-195`) to also fetch attachments via service then pass them in.

Exact TO/conversion code is in RESEARCH Pattern 3 (`19-RESEARCH.md` lines 400-426).

**Download-handler analog (`genossi_rest/src/member_document.rs:232-267` verbatim):**
```rust
pub async fn download_document<RestState: RestStateDef>(
    rest_state: State<RestState>,
    Extension(context): Extension<Context>,
    Path((member_id, document_id)): Path<(Uuid, Uuid)>,
) -> Response {
    error_handler(
        (async {
            let (doc, _) = rest_state
                .member_document_service()
                .download(member_id, document_id,
                    crate::extract_auth_context(Some(context))?, None,
                ).await?;

            let data = rest_state
                .document_storage()
                .load(&doc.relative_path)
                .await
                .map_err(|e| RestError::InternalError(format!("Failed to load file: {}", e)))?;

            let content_disposition =
                crate::http_util::content_disposition_attachment(&doc.file_name);

            Ok(Response::builder()
                .status(200)
                .header("Content-Type", doc.mime_type.as_ref())
                .header("Content-Disposition", &content_disposition)
                .body(Body::from(data))
                .unwrap())
        })
        .await,
    )
}
```

**New `download_attachment` handler:** see RESEARCH Pattern 4 (`19-RESEARCH.md` lines 436-489).
Differences vs member_document:
- New `#[derive(Deserialize)] struct DispositionQuery { disposition: Option<String> }`.
- Add `Query(q): Query<DispositionQuery>` axum extractor.
- Match `q.disposition.as_deref()` → `Some("inline")` → `http_util::content_disposition_inline(&file_name)`, else `content_disposition_attachment`.
- Use `inbox_rest::map_error` (`inbox_rest.rs:126-139`), NOT `error_handler` (different error pipeline in `genossi_mail/src/inbox_rest.rs` vs `genossi_rest/src/member_document.rs` — they use slightly different error mapping; mirror the analog `get_inbox` at `:180-195` for the `map_error` path).
- 404 when `find_by_id_and_mail` returns `Ok(None)`, 410 GONE or 404 when `oversized==true || relative_path is None`.

**Router registration** (find existing `Router::new().route("/", get(list_inbox))` chain in same file; add):
```rust
.route("/{mail_id}/attachments/{attachment_id}", get(download_attachment))
```

**Permission:** Same as `get_inbox` — request already passes the Vorstand-only guard via `forbid_unauthenticated` middleware (D-09). No new permission code.

---

### 7. `genossi_rest/src/http_util.rs` — Add `content_disposition_inline`

**read_first:** `genossi_rest/src/http_util.rs:1-80` (whole helper module: sanitizer + ASCII fallback + percent-encoder + existing `attachment` builder + tests)

**Existing helper to mirror** (lines 39-50):
```rust
/// Build an RFC 6266 `Content-Disposition: attachment` header value.
pub fn content_disposition_attachment(filename: &str) -> String {
    let ascii_fallback = sanitize_ascii_filename(filename);
    let utf8_encoded = percent_encode_utf8(filename);
    format!(
        "attachment; filename=\"{}\"; filename*=UTF-8''{}",
        ascii_fallback, utf8_encoded
    )
}
```

**New helper to add (immediately after existing `content_disposition_attachment`):**
```rust
/// Build an RFC 6266 `Content-Disposition: inline` header value.
/// Same filename-encoding rules as `content_disposition_attachment`.
pub fn content_disposition_inline(filename: &str) -> String {
    let ascii_fallback = sanitize_ascii_filename(filename);
    let utf8_encoded = percent_encode_utf8(filename);
    format!(
        "inline; filename=\"{}\"; filename*=UTF-8''{}",
        ascii_fallback, utf8_encoded
    )
}
```

**Tests to add** (mirror the existing test block at `:82-175`):
- `test_inline_simple_filename` → contains `"inline; filename=\"foo.pdf\""`
- `test_inline_umlaut_filename` → ASCII fallback + UTF-8 percent
- `test_inline_quote_in_filename` → quotes replaced by `_`
- `test_inline_newline_in_filename` → `\r\n` percent-encoded

---

### 8. `genossi_bin/src/lib.rs` — Wiring: type alias, RestStateImpl field, ctor, backfill spawn

**read_first:**
- `genossi_bin/src/lib.rs:575-600` (type aliases — see `InboundMailDaoType`, `InboxImapClientType`, `InboxServiceType`)
- `genossi_bin/src/lib.rs:663-666` (existing `worker_inbox_*` fields on `RestStateImpl`)
- `genossi_bin/src/lib.rs:1079-1095` (ctor wiring for `inbox_dao` + `inbox_imap_client`)
- `genossi_bin/src/lib.rs:1344-1351` (existing `start_inbox_worker` — verbatim spawn pattern for new backfill)

**Existing type alias to mirror** (line 580):
```rust
type InboundMailDaoType = genossi_mail::dao_sqlite::InboundMailDaoSqlite;
```
**Add next to it:**
```rust
type InboundMailAttachmentDaoType =
    genossi_mail::dao_sqlite::InboundMailAttachmentDaoSqlite;
```

**Existing field shape to mirror** (lines 663-666):
```rust
// Inbox worker dependencies
worker_inbox_config_service: Arc<ConfigService>,
worker_inbox_dao: Arc<InboundMailDaoType>,
worker_inbox_imap_client: Arc<InboxImapClientType>,
```
**Add the new field on `RestStateImpl`:**
```rust
inbound_attachment_dao: Arc<InboundMailAttachmentDaoType>,
```

**Existing ctor wiring to mirror** (lines 1079-1095):
```rust
let inbox_dao = Arc::new(InboundMailDaoType::new(pool.clone()));
let inbox_imap_client = Arc::new(InboxImapClientType::new());
// …
let worker_inbox_config_dao = ConfigDao::new(pool.clone());
let worker_inbox_config_service = Arc::new(ConfigService::new(worker_inbox_config_dao));
let worker_inbox_dao = Arc::new(InboundMailDaoType::new(pool.clone()));
let worker_inbox_imap_client = Arc::new(InboxImapClientType::new());
```
**Add next to it:**
```rust
let inbound_attachment_dao =
    Arc::new(InboundMailAttachmentDaoType::new(pool.clone()));
```

Then add to the `RestStateImpl { … }` struct literal in the ctor.

**Existing spawn shape to mirror** (lines 1344-1351):
```rust
pub fn start_inbox_worker(&self) {
    let config_service = self.worker_inbox_config_service.clone();
    let dao = self.worker_inbox_dao.clone();
    let imap_client = self.worker_inbox_imap_client.clone();
    tokio::spawn(async move {
        genossi_mail::inbox::start_inbox_worker(config_service, dao, imap_client).await;
    });
}
```
**Add immediately below:**
```rust
pub fn start_attachment_backfill_worker(&self) {
    let config_service = self.worker_inbox_config_service.clone();
    let dao = self.worker_inbox_dao.clone();
    let attachment_dao = self.inbound_attachment_dao.clone();
    let storage = self.document_storage.clone();
    let imap_client = self.worker_inbox_imap_client.clone();
    tokio::spawn(async move {
        genossi_mail::inbox::run_attachment_backfill(
            config_service, dao, attachment_dao, storage, imap_client,
        ).await;
    });
}
```

**InboxRestState impl** (search `impl InboxRestState for RestStateImpl` in same file): extend to expose `inbound_attachment_dao()` and `document_storage()` if not already (verify; `document_storage` already exists per field at `:614`).

---

### 9. `genossi_bin/src/main.rs` — Spawn backfill

**read_first:** `genossi_bin/src/main.rs:30-65` (existing migration → service-init → worker-spawn sequence). Search for `rest_state.start_inbox_worker();` line.

**Add immediately after that line:**
```rust
rest_state.start_attachment_backfill_worker();
tracing::info!("Attachment backfill worker spawned");
```

---

### 10. Frontend Component `genossi-frontend/src/component/inbox/attachment_list.rs` (NEW)

**read_first:** `genossi-frontend/src/component/inbox/mail_list_item.rs` (whole file, 43 lines — style baseline, prop shape, `#[component]` macro usage)

**Existing component (whole file) to copy style from:**
```rust
use dioxus::prelude::*;
use super::InboxStatusBadge;

#[component]
pub fn InboxMailListItem(
    subject: String,
    from_address: String,
    received_at: String,
    replied: bool,
    done: bool,
    archived: bool,
    has_attachments: bool,
    assigned_label: String,
    selected: bool,
    on_click: EventHandler<()>,
) -> Element {
    let row_class = if selected { "p-3 cursor-pointer bg-blue-50" } else { "p-3 cursor-pointer hover:bg-gray-50" };
    rsx! {
        li { class: "{row_class}", onclick: move |_| on_click.call(()),
            div { class: "flex justify-between",
                span { class: "font-medium truncate", "{subject}" }
                InboxStatusBadge { replied: replied, done: done, archived: archived }
            }
            div { class: "text-sm text-gray-600 truncate", "{from_address}" }
            div { class: "flex justify-between text-xs text-gray-500",
                span { "{received_at}" }
                span {
                    if has_attachments { "📎 " } else { "" }
                    "{assigned_label}"
                }
            }
        }
    }
}
```

**New component shape** (exact RSX skeleton from RESEARCH Code Examples lines 837-879):
```rust
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

**Tailwind classes pulled from UI-SPEC §Spacing/Color tables** — no new tokens.

---

### 11. Frontend Component `genossi-frontend/src/component/inbox/attachment_list_item.rs` (NEW)

**read_first:** Same as #10 (`mail_list_item.rs`), plus the URL-builder pattern in `genossi-frontend/src/api.rs:1397-1401`:
```rust
pub async fn get_inbox_detail(config: &Config, id: &str) -> Result<InboundMailDetailTO, AppError> {
    let url = format!("{}/api/inbox/{}", config.backend, id);
    let response = check_response(reqwest::get(url).await?).await?;
    Ok(response.json().await?)
}
```

**Full new component:** Use RESEARCH Code Examples lines 882-987 verbatim — it already covers:
- Oversized early-return branch (D-02).
- `is_image` / `is_pdf` MIME-routing (D-12).
- `<a href download>` for primary action (avoids `feedback_dioxus_button_type.md` reload bug).
- `<a target="_blank" rel="noopener">` for PDF preview / image thumbnail wrapping.
- Glyph table from UI-SPEC §Glyph Table (function `glyph_for_mime`).
- Short MIME label (function `short_mime`).
- All Tailwind classes (`p-3 border rounded bg-white flex items-center gap-3`, `max-h-24 max-w-32 object-contain`, `bg-blue-500 hover:bg-blue-600 text-white`, `text-amber-700`).
- i18n: `Key::InboxAttachmentsDownload`, `Key::InboxAttachmentsPreview`, `Key::InboxAttachmentsOversized`, `Key::InboxAttachmentsImageAltPrefix`.
- `loading: "lazy"` attribute on `<img>` (Pitfall 7 — verify with `cargo check`).

**Verify before implementing:**
- `cargo check -p genossi-frontend` to confirm `loading: "lazy"` and `aria_hidden` attributes accepted by Dioxus 0.6 RSX.

---

### 12. `genossi-frontend/src/component/inbox/mod.rs` — Register two new components

**read_first:** `genossi-frontend/src/component/inbox/mod.rs` (whole file, 7 lines)

**Existing file contents:**
```rust
pub mod mail_list_item;
pub mod reply_form;
pub mod status_badge;

pub use mail_list_item::InboxMailListItem;
pub use reply_form::InboxReplyForm;
pub use status_badge::InboxStatusBadge;
```

**Add (preserving alphabetical-ish order):**
```rust
pub mod attachment_list;
pub mod attachment_list_item;
// existing mods…

pub use attachment_list::InboxAttachmentList;
pub use attachment_list_item::InboxAttachmentListItem;
// existing pubs…
```

---

### 13. `genossi-frontend/src/page/inbox_page.rs` — Delete MVP-hint, insert component

**read_first:** `genossi-frontend/src/page/inbox_page.rs:295-360` (Detail-Pane wrapper, body `<pre>`, assignment section start)

**Block to delete** (lines 331-335):
```rust
if d.has_attachments {
    div { class: "text-xs text-amber-700",
        "📎 Diese Mail enthält Anhänge (nicht anzeigbar im MVP)"
    }
}
```

**Insertion point:** Between the `<pre>` body (currently ends at line 347 `"{d.body_text}"`) and the assignment `div { class: "border-t pt-2 mt-2" }` block (line 350).

**Insert** (Component-First: NO inline RSX):
```rust
InboxAttachmentList {
    mail_id: d.id.clone(),
    attachments: d.attachments.clone(),
    has_legacy_attachments: d.attachments.is_empty() && d.has_attachments,
}
```

**Anti-pattern reminder:** The page must NOT contain `<li>`, `<img>`, or `<a download>` for attachments. Pure delegation. See `genossi-frontend/CLAUDE.md` §Component-First Principle (in this conversation's reminder block).

---

### 14. `genossi-frontend/src/api.rs` — TO + URL builders

**read_first:** `genossi-frontend/src/api.rs:1364-1378` (existing `InboundMailDetailTO`) + `:1397-1401` (existing URL-format pattern)

**Existing TO to extend** (lines 1364-1378):
```rust
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct InboundMailDetailTO {
    pub id: String,
    pub from_address: String,
    pub subject: String,
    pub received_at: String,
    pub body_text: String,
    pub has_attachments: bool,
    pub has_html_body: bool,
    pub replied: bool,
    pub done: bool,
    pub archived: bool,
    pub assigned_member_id: Option<String>,
    pub assigned_member_name: Option<String>,
}
```

**Add new TO above DetailTO:**
```rust
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct InboundMailAttachmentTO {
    pub id: String,
    pub file_name: String,
    pub mime_type: String,
    pub size_bytes: i64,
    pub oversized: bool,
}
```

**Add field to DetailTO:**
```rust
pub attachments: Vec<InboundMailAttachmentTO>,
```

**URL builders** (mirror style of `get_inbox_detail` at `:1397`):
```rust
pub fn attachment_download_url(config: &Config, mail_id: &str, attachment_id: &str) -> String {
    format!("{}/api/inbox/{}/attachments/{}", config.backend, mail_id, attachment_id)
}

pub fn attachment_inline_url(config: &Config, mail_id: &str, attachment_id: &str) -> String {
    format!("{}/api/inbox/{}/attachments/{}?disposition=inline", config.backend, mail_id, attachment_id)
}
```

(Component can inline these strings via `CONFIG.read().clone().backend` directly per RESEARCH Code Examples line 898 — either approach works; planner chooses.)

---

### 15. Frontend Util `genossi-frontend/src/util/format.rs` (NEW)

**read_first:** RESEARCH Code Examples lines 992-1010 (`format_size` implementation). **No in-repo analog** — directory does not exist.

**Full file contents** (verbatim from RESEARCH):
```rust
/// Format a byte count into a human-readable string.
/// Integer-math to avoid floating rounding surprises.
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn bytes_under_kb() { assert_eq!(format_size(42), "42 B"); assert_eq!(format_size(999), "999 B"); }
    #[test] fn kb_range_integer() { assert_eq!(format_size(12 * 1024), "12 KB"); assert_eq!(format_size(1023 * 1024), "1023 KB"); }
    #[test] fn mb_range_one_decimal() { assert_eq!(format_size(1_468_006), "1.4 MB"); assert_eq!(format_size(9_8 * 1024 * 1024 / 10), "9.8 MB"); }
    #[test] fn gb_range_one_decimal() { let b = 12 * 1024_u64 * 1024 * 1024 / 10; assert_eq!(format_size(b), "1.2 GB"); }
}
```

Test cases derived from UI-SPEC §Formatting & States table.

---

### 16. Frontend Util Registry `genossi-frontend/src/util/mod.rs` (NEW)

**read_first:** None — new directory.

**Full file:**
```rust
pub mod format;
```

---

### 17. Frontend Bootstrap `genossi-frontend/src/main.rs` (or `lib.rs`) — Declare `util` module

**read_first:** `genossi-frontend/src/main.rs` (search for existing `pub mod i18n;` / `pub mod component;` block)

**Add (alphabetical order):**
```rust
pub mod util;
```

---

### 18. `genossi-frontend/src/i18n/mod.rs` — Add 7 Key variants

**read_first:** `genossi-frontend/src/i18n/mod.rs:45-100+` (existing `Key` enum) — positional anchor: search for `OpenInboxCount` / `OpenInboxNone` at `:504-505` to find the inbox-related grouping.

**Existing nearby variants** (`:504-505`):
```rust
OpenInboxCount,
OpenInboxNone,
```

**Add (UI-SPEC §Copywriting Contract — 7 keys, including 2 auxiliary `…DownloadError` + `…ImageAltPrefix`):**
```rust
InboxAttachmentsHeader,
InboxAttachmentsDownload,
InboxAttachmentsPreview,
InboxAttachmentsEmptyLegacy,
InboxAttachmentsOversized,
InboxAttachmentsDownloadError,
InboxAttachmentsImageAltPrefix,
```

---

### 19. `genossi-frontend/src/i18n/de.rs` — De translations

**read_first:** `genossi-frontend/src/i18n/de.rs:436-437` (existing `OpenInboxCount` / `OpenInboxNone` translations)

**Existing nearby translations:**
```rust
Key::OpenInboxCount => "{} offene Mails".into(),
Key::OpenInboxNone => "Keine offenen Mails".into(),
```

**Add (copy from UI-SPEC §Copywriting Contract — DE column):**
```rust
Key::InboxAttachmentsHeader => "Anhänge".into(),
Key::InboxAttachmentsDownload => "Herunterladen".into(),
Key::InboxAttachmentsPreview => "Vorschau".into(),
Key::InboxAttachmentsEmptyLegacy =>
    "Anhang vor Phase 19 empfangen — bitte im Mail-Client öffnen".into(),
Key::InboxAttachmentsOversized =>
    "Zu groß — bitte im Mail-Client öffnen".into(),
Key::InboxAttachmentsDownloadError =>
    "Anhang konnte nicht geladen werden — bitte erneut versuchen".into(),
Key::InboxAttachmentsImageAltPrefix => "Vorschau für".into(),
```

(The `{size}` placeholder in `InboxAttachmentsOversized` is interpolated client-side via `format!("{} ({})", i18n.t(…), size_str)` per RESEARCH Code Examples line 910.)

---

### 20. `genossi-frontend/src/i18n/en.rs` — En translations

**read_first:** `genossi-frontend/src/i18n/en.rs` — find corresponding `OpenInboxCount` / `OpenInboxNone` lines.

**Add (UI-SPEC §Copywriting Contract — EN column):**
```rust
Key::InboxAttachmentsHeader => "Attachments".into(),
Key::InboxAttachmentsDownload => "Download".into(),
Key::InboxAttachmentsPreview => "Preview".into(),
Key::InboxAttachmentsEmptyLegacy =>
    "Attachment received before Phase 19 — open in your mail client".into(),
Key::InboxAttachmentsOversized =>
    "Too large — open in your mail client".into(),
Key::InboxAttachmentsDownloadError =>
    "Could not load attachment — please try again".into(),
Key::InboxAttachmentsImageAltPrefix => "Preview of".into(),
```

---

### 21. `MockInboxImapClient` — Auto-extends via `#[automock]`

**read_first:** `genossi_mail/src/inbox.rs:113` (`#[automock]` on `InboxImapClient` trait)

**Mechanism:** `#[automock]` macro from `mockall` auto-generates `MockInboxImapClient` based on the trait at compile time. Adding `fetch_one_by_uid` to the trait → mock auto-extends. Test files that previously called `MockInboxImapClient::new()` continue to compile (unset expectations are fine for unused methods).

**Test wiring:** If any existing unit test does `.expect_…()` for ALL methods, add `.expect_fetch_one_by_uid().returning(|_, _, _| Ok(None));` to keep them green. Otherwise no change.

---

### 22. `genossi_bin/tests/e2e_tests.rs` — Seed helper + roundtrip tests

**read_first:** `genossi_bin/tests/e2e_tests.rs:4640-4810` (existing `seed_inbound_mail` helper + sample inbox E2E tests)

**Test additions:**
1. `seed_inbound_mail_attachment(pool, mail_id, file_name, mime, bytes, oversized)` — direct SQL insert into `inbound_mail_attachments` + parallel call to `DocumentStorage::save` (skip save when oversized).
2. `test_get_inbox_detail_includes_attachments` — seed mail + 2 attachments → `GET /api/inbox/{id}` → assert `attachments.len() == 2`, fields correct.
3. `test_download_attachment_default_returns_attachment_disposition` — seed + small bytes → `GET /api/inbox/{mid}/attachments/{aid}` → assert `Content-Type` matches, `Content-Disposition` starts with `attachment;`, body bytes equal.
4. `test_download_attachment_inline_query_param_switches_disposition` — same as above but `?disposition=inline` → assert `Content-Disposition` starts with `inline;`.
5. `test_download_attachment_oversized_returns_payload_too_large_or_404` — seed with `oversized=true, relative_path=NULL` → endpoint returns 4xx (planner picks 404 or 413 per Pattern 4).
6. `test_download_attachment_404_when_mail_id_mismatch` — IDOR check: attachment exists under mail A, request as `(mail_B, attachment_A)` → 404.

---

## Shared Patterns

### Authentication / Permission Guard
**Source:** `genossi_rest/src/lib.rs:430-440` (cookie middleware + `forbid_unauthenticated` already wired to `/api/inbox` router).
**Apply to:** Both `GET /api/inbox/{id}` extension AND new `GET /api/inbox/{mail_id}/attachments/{attachment_id}`.
**Concrete:** The new handler is registered on the existing `Router::new()` chain inside `inbox_rest.rs` — automatically inherits the same middleware. No new permission code required. (D-09)

### Error Mapping (REST → HTTP)
**Source:** `genossi_mail/src/inbox_rest.rs:126-139`
```rust
fn map_error(e: MailServiceError) -> Response {
    let (code, msg) = match e {
        MailServiceError::NotFound => (StatusCode::NOT_FOUND, "not found".to_string()),
        MailServiceError::ConfigMissing(k) =>
            (StatusCode::SERVICE_UNAVAILABLE, format!("missing config: {}", k)),
        MailServiceError::SmtpError(m) => (StatusCode::BAD_GATEWAY, m.to_string()),
        MailServiceError::DataAccess(m) => (StatusCode::INTERNAL_SERVER_ERROR, m.to_string()),
        MailServiceError::TemplateValidation(m) => (StatusCode::BAD_REQUEST, m.to_string()),
        MailServiceError::BadRequest(m) => (StatusCode::BAD_REQUEST, m.to_string()),
    };
    (code, msg).into_response()
}
```
**Apply to:** Use `map_error(e)` in `download_attachment` for all service-returned errors.

### Atomic Save-then-DB with Rollback
**Source:** `genossi_mail/src/static_document_service.rs:108-120` (shown above in §4)
**Apply to:** `persist_attachment` helper in `genossi_mail/src/inbox.rs` (for both new poll-path attachments AND backfill).

### Component-First (Frontend)
**Source:** `genossi-frontend/CLAUDE.md` §Component-First Principle (reminder block).
**Apply to:** Pages compose components — `inbox_page.rs:331-335` block deleted, replaced by `InboxAttachmentList { … }` call. No inline RSX for attachment row/list logic anywhere outside `component/inbox/attachment_list*.rs`.

### Dioxus Anchor (avoid Page-Reload bug)
**Source:** Memory `feedback_dioxus_button_type.md` + UI-SPEC §Action Matrix.
**Apply to:** All Download/Preview actions in `attachment_list_item.rs` are `<a href="…" download>` / `<a target="_blank">`, NOT `<button onclick>`.

### i18n (Two-Locale Discipline)
**Source:** `genossi-frontend/CLAUDE.md` §i18n System (only `Locale::En` + `Locale::De`; no `cs.rs`).
**Apply to:** All 7 new keys MUST exist in both `de.rs` AND `en.rs`. Hardcoded strings forbidden (UI-SPEC §Anti-Patterns).

### `#[utoipa::path]` OpenAPI Decoration
**Source:** `genossi_mail/src/inbox_rest.rs:170-179` (existing `get_inbox` handler)
**Apply to:** New `download_attachment` handler — annotate with:
- `get`, `path = "/{mail_id}/attachments/{attachment_id}"`, `tag = "inbox"`
- `params(("mail_id" = String, Path, …), ("attachment_id" = String, Path, …), ("disposition" = Option<String>, Query, …))`
- `responses((status = 200, …), (status = 401, …), (status = 404, …), (status = 410, …))`

---

## No Analog Found

**None.** Every new file or modification has a concrete in-repo analog (or, in two cases — `util/format.rs` and `util/mod.rs` — a spec-driven RESEARCH source block). The Phase 19 scope is almost pure "wire-up" of existing patterns.

---

## Metadata

**Analog search scope:**
- Backend: `genossi_mail/src/`, `genossi_rest/src/`, `genossi_service/src/`, `genossi_service_impl/src/`, `genossi_bin/src/`, `migrations/sqlite/`
- Frontend: `genossi-frontend/src/component/`, `genossi-frontend/src/page/`, `genossi-frontend/src/i18n/`, `genossi-frontend/src/api.rs`
- Tests: `genossi_bin/tests/e2e_tests.rs`

**Files scanned (read or grepped):**
- `genossi_mail/src/dao.rs` (lines 80-250), `dao_sqlite.rs` (lines 340-805 + 1100-1200)
- `genossi_mail/src/inbox.rs` (lines 1-330), `inbox_imap.rs` (lines 1-194), `inbox_rest.rs` (lines 1-220)
- `genossi_mail/src/static_document_service.rs` (lines 90-260)
- `genossi_rest/src/member_document.rs` (lines 220-270), `http_util.rs` (full file)
- `genossi_bin/src/lib.rs` (lines 575-1095, 1336-1440)
- `genossi-frontend/src/component/inbox/mod.rs`, `mail_list_item.rs`
- `genossi-frontend/src/page/inbox_page.rs` (lines 295-360)
- `genossi-frontend/src/api.rs` (lines 1340-1450)
- `genossi-frontend/src/i18n/mod.rs` (lines 1-100, 504-505), `de.rs` (lines 425-450)
- `migrations/sqlite/20260404000001_create_mail_recipient_attachments_table.sql`, `20260409000001_create_inbound_mails_table.sql`
- Most recent migration check: `20260603100000_mail_job_attach_repayment_letter.sql` → new migration timestamp must be later (e.g. `20260608000000_…`).

**Pattern extraction date:** 2026-06-07
