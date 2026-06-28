---
phase: 19-e-mail-anhaenge-anzeigen
plan: 19-02
slug: service-and-imap
type: execute
wave: 2
depends_on: [19-01]
files_modified:
  - genossi_mail/src/inbox.rs
  - genossi_mail/src/inbox_imap.rs
autonomous: true
requirements: []
must_haves:
  truths:
    - "`parse_raw_mail` returns `Vec<ParsedAttachment>` populated from `msg.attachments()` (D-01) — replaces the legacy attachment_count-only path"
    - "`persist_attachment` writes file FIRST, DB row SECOND, deletes file on DB-fail rollback (T-07)"
    - "10 MB hard cap (D-02) enforced in `persist_attachment` — oversized rows persist metadata only (relative_path=None, oversized=true), NO bytes written to disk (T-01)"
    - "Storage path schema is `inbound_mail_attachments/{inbound_mail_id}/{attachment_id}` — UUIDs only, never the attacker-controlled filename (T-02, D-04)"
    - "`InboxImapClient::fetch_one_by_uid` re-checks UIDVALIDITY against `expected_uid_validity` and returns Err on mismatch — silent skip on caller per D-06 (T-06)"
    - "Inbox poll worker persists attachments after the mail-create step (best-effort: persist failure logs warn + continues per D-06)"
    - "No MIME-type whitelist enforced (D-03) — all attachment parts persist; mail-parser MIME fallback is application/octet-stream when content-type header is absent"
    - "`InboxService::find_attachment` and `list_attachments` expose attachments to the REST layer (Plan 19-03 consumes)"
  artifacts:
    - path: "genossi_mail/src/inbox.rs"
      provides: "ParsedAttachment struct, extract_attachments fn, persist_attachment helper, InboxService::find_attachment + list_attachments + run_attachment_backfill (free fn)"
      contains: "pub struct ParsedAttachment"
    - path: "genossi_mail/src/inbox_imap.rs"
      provides: "AsyncImapClient::fetch_one_by_uid impl"
      contains: "fn fetch_one_by_uid"
  key_links:
    - from: "parse_raw_mail (inbox.rs)"
      to: "ParsedMail.attachments field"
      via: "msg.attachments() iterator + extract_attachments helper"
      pattern: "for .* in msg.attachments\\(\\)"
    - from: "persist_attachment (inbox.rs)"
      to: "DocumentStorage::save + InboundMailAttachmentDao::create"
      via: "save-then-DB pattern with delete rollback"
      pattern: "storage\\.save.*\\n.*if let Err.*\\n.*storage\\.delete"
    - from: "Inbox poll worker"
      to: "persist_attachment loop"
      via: "after `mail dao create` returns Ok, loop attachments, persist each"
      pattern: "for att in parsed.attachments"

---

<objective>
Erweitere die Mail-Parsing- + Worker-Logik um Attachment-Persistenz, baue
`InboxImapClient::fetch_one_by_uid` (für Backfill in Plan 19-04), und füge
die `InboxService`-Methoden hinzu, die der REST-Layer (Plan 19-03) braucht.

Purpose: Phase 19 Backbone — sobald der bestehende Poll-Worker eine Mail
erfolgreich persistiert hat, müssen die Attachments via Save-then-DB-Pattern
ebenfalls landen. Backfill (Plan 19-04) benötigt zusätzlich einen
Single-UID-IMAP-Refetch.

Output: `ParsedAttachment` + `extract_attachments` + 10-MB-Cap-Persistenz +
4 neue trait/free-fn-Items. Bestehende Inbox-Tests bleiben grün; neue
Unit-Tests verifizieren die Attachment-Pipeline (inkl. oversized-Skip + Rollback).
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
@.planning/phases/19-e-mail-anh-nge-anzeigen-backend-endpoint-zum-abrufen-von-anh/19-01-SUMMARY.md
@CLAUDE.md

<interfaces>
<!-- Pre-extracted from analog sources. Executor mirrors verbatim — no exploration needed. -->

From `genossi_mail/src/inbox.rs:113-136` (existing `InboxImapClient` trait — EXTEND with `fetch_one_by_uid`):
```rust
#[automock]
#[async_trait]
pub trait InboxImapClient: Send + Sync + 'static {
    async fn fetch_since(
        &self,
        config: &ImapConfig,
        min_uid: i64,
    ) -> Result<Vec<FetchedMessage>, MailServiceError>;
    // … existing methods …
}
```

From `genossi_mail/src/inbox.rs:142-154` (existing `ParsedMail` — ADD `attachments` field):
```rust
pub struct ParsedMail {
    pub from_address: String,
    pub subject: String,
    pub received_at: PrimitiveDateTime,
    pub body_text: String,
    pub has_html_body: bool,
    pub has_attachments: bool,
    // ... existing fields ...
}
```

From `genossi_mail/src/inbox.rs:208` — line to be REPLACED:
```rust
let has_attachments = msg.attachment_count() > 0;
```

From `genossi_mail/src/static_document_service.rs:108-120` (atomic save-then-DB analog):
```rust
let rel_path = doc.relative_path();
self.storage.save(&rel_path, &upload.data).await
    .map_err(|e| StaticDocumentError::Storage(Arc::from(e.to_string())))?;

if let Err(e) = self.dao.create(&doc).await {
    let _ = self.storage.delete(&rel_path).await;
    return Err(e.into());
}
```

From `genossi_mail/src/inbox_imap.rs:115-147` (analog `fetch_since` — mirror for `fetch_one_by_uid`):
```rust
async fn fetch_since(&self, config: &ImapConfig, min_uid: i64) -> Result<Vec<FetchedMessage>, MailServiceError> {
    let (mut session, _mailbox) = open_examine_session(config).await?;
    let start = (min_uid + 1).max(1);
    let range = format!("{}:*", start);
    let stream = session.uid_fetch(range, "(UID BODY.PEEK[])").await
        .map_err(|e| err(format!("IMAP uid_fetch: {}", e)))?;
    // ... collect + parse ...
    let _ = session.logout().await;
    Ok(out)
}
```

`DocumentStorage` trait (from `genossi_service/src/document_storage.rs`):
- `async fn save(&self, relative_path: &str, bytes: &[u8]) -> Result<(), StorageError>`
- `async fn load(&self, relative_path: &str) -> Result<Vec<u8>, StorageError>`
- `async fn delete(&self, relative_path: &str) -> Result<(), StorageError>`

`InboundMailAttachmentDao` (from Plan 19-01):
- `create`, `find_by_inbound_mail_id`, `find_by_id_and_mail`, `count_for_mail`
</interfaces>
</context>

<tasks>

<task type="auto" tdd="true">
  <name>Task 1: ParsedAttachment + parse_raw_mail extension + persist_attachment + fetch_one_by_uid</name>
  <files>
    genossi_mail/src/inbox.rs,
    genossi_mail/src/inbox_imap.rs
  </files>
  <read_first>
    - genossi_mail/src/inbox.rs:113-136 (existing `InboxImapClient` trait — ADD method here so `MockInboxImapClient` auto-extends)
    - genossi_mail/src/inbox.rs:142-260 (existing `ParsedMail` + `parse_raw_mail` — especially line 208 attachment_count usage)
    - genossi_mail/src/inbox.rs:266-322 (existing `InboxService` trait + `InboxServiceImpl` — extend with find_attachment + list_attachments)
    - genossi_mail/src/inbox_imap.rs:1-194 (whole file — copy `fetch_since` pattern + `open_examine_session` for `fetch_one_by_uid`)
    - genossi_mail/src/static_document_service.rs:108-120 (atomic save-then-DB analog)
    - .planning/phases/19-e-mail-anh-nge-anzeigen-backend-endpoint-zum-abrufen-von-anh/19-RESEARCH.md §Code Examples → Mail-parser Iterate (lines 656-705) + Pattern 1 (lines 273-326) + Pattern 2 (lines 336-388)
    - .planning/phases/19-e-mail-anh-nge-anzeigen-backend-endpoint-zum-abrufen-von-anh/19-PATTERNS.md §4 + §5
    - .planning/phases/19-e-mail-anh-nge-anzeigen-backend-endpoint-zum-abrufen-von-anh/19-CONTEXT.md D-01..D-06
  </read_first>
  <behavior>
    - `pub struct ParsedAttachment { pub file_name: String, pub mime_type: String, pub bytes: Vec<u8> }` added next to `ParsedMail`
    - `ParsedMail` gains `pub attachments: Vec<ParsedAttachment>` field (keep `has_attachments: bool` for backward compat)
    - `parse_raw_mail` populates the new vec via `extract_attachments(&msg)` helper (RESEARCH lines 669-705); `has_attachments` becomes `!parsed.attachments.is_empty()`
    - Fallback filenames for nameless attachments: `format!("attachment_{}.bin", idx)`; for `is_message()` parts: `format!("forwarded_{}.eml", idx)` with mime `"message/rfc822"` (Pitfall 3 + 4)
    - Default mime: `"application/octet-stream"` if `content_type()` is None
    - `const ATTACHMENT_MAX_BYTES: u64 = 10 * 1024 * 1024;` (D-02 hard cap, NOT configurable)
    - `persist_attachment` helper (private fn, takes `&dyn DocumentStorage`, `&dyn InboundMailAttachmentDao`, mail_id, file_name, mime, bytes): generates `id = Uuid::new_v4()`; if `bytes.len() > MAX` → oversized=true + relative_path=None + NO storage.save call; else storage.save → dao.create with rollback (storage.delete) on DB-fail
    - `fetch_one_by_uid(&self, &ImapConfig, expected_uid_validity: i64, uid: i64) -> Result<Option<FetchedMessage>, MailServiceError>` added to `InboxImapClient` trait AND impl on `AsyncImapClient`
    - Impl re-checks `mailbox.uid_validity` after `open_examine_session`; mismatch → Err (caller silent-skips per D-06)
    - Uses `format!("{}", uid)` as the uid_set argument (single-UID)
    - Returns `Ok(None)` if UID stream is empty
    - `InboxService` trait gets `async fn find_attachment(mail_id, attachment_id) -> Result<Option<InboundMailAttachment>, MailServiceError>` and `async fn list_attachments(mail_id) -> Result<Arc<[InboundMailAttachment]>, MailServiceError>`
    - `InboxServiceImpl` generic-param list extended to include `A: InboundMailAttachmentDao` and `St: DocumentStorage` so the impl can wire attachment dao + storage; field naming `attachment_dao: Arc<A>`, `storage: Arc<St>`
    - Existing poll-worker entry (inside `inbox.rs`, search for the place where `dao.create` is called for the parent mail) gains an inner attachment-persistence loop AFTER successful mail-create: for each `att` in `parsed.attachments`, call `persist_attachment`; on Err → `tracing::warn!` + continue (D-06)
  </behavior>
  <action>
    **Step 1 — Add `ParsedAttachment` struct** in `genossi_mail/src/inbox.rs` immediately next to the `ParsedMail` definition (line ~142):
    ```rust
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct ParsedAttachment {
        pub file_name: String,
        pub mime_type: String,
        pub bytes: Vec<u8>,
    }
    ```

    **Step 2 — Extend `ParsedMail`** with `pub attachments: Vec<ParsedAttachment>` (keep all existing fields).

    **Step 3 — Add the `extract_attachments` helper** as a private fn in `inbox.rs` (copy verbatim from RESEARCH lines 669-705):
    ```rust
    fn extract_attachments(msg: &mail_parser::Message) -> Vec<ParsedAttachment> {
        let mut out = Vec::new();
        for (idx, part) in msg.attachments().enumerate() {
            if part.is_message() {
                let bytes = part.contents().to_vec();
                let name = part.attachment_name().map(|s| s.to_string())
                    .unwrap_or_else(|| format!("forwarded_{}.eml", idx));
                out.push(ParsedAttachment {
                    file_name: name,
                    mime_type: "message/rfc822".to_string(),
                    bytes,
                });
                continue;
            }
            let mime = part.content_type().map(|ct| {
                let mut s = String::from(ct.ctype());
                if let Some(sub) = ct.subtype() { s.push('/'); s.push_str(sub); }
                s
            }).unwrap_or_else(|| "application/octet-stream".to_string());
            let name = part.attachment_name().map(|s| s.to_string())
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

    **Step 4 — Modify `parse_raw_mail`**: replace line 208 (`let has_attachments = msg.attachment_count() > 0;`) with:
    ```rust
    let attachments = extract_attachments(&msg);
    let has_attachments = !attachments.is_empty();
    ```
    And add `attachments,` to the `ParsedMail { … }` struct-literal at the end of the function.

    **Step 5 — Add the `persist_attachment` helper** (private free fn in `inbox.rs`, copy verbatim from RESEARCH lines 273-326). Important fixes:
    - Use `InboundMailAttachment` from `crate::dao::InboundMailAttachment` (the type from Plan 19-01)
    - Use `InboundMailAttachmentDao` from `crate::dao::InboundMailAttachmentDao`
    - Use `DocumentStorage` from `genossi_service::document_storage::DocumentStorage`
    - On `dao.create` failure, log via `tracing::warn!("persist_attachment rollback: storage.delete failed: {:?}", _e)` if the delete itself errors (keep best-effort)
    - Add the constant ABOVE the fn: `const ATTACHMENT_MAX_BYTES: u64 = 10 * 1024 * 1024; // D-02`

    **Step 6 — Add `fetch_one_by_uid`** to the `InboxImapClient` trait declaration in `genossi_mail/src/inbox.rs:113-136`:
    ```rust
    async fn fetch_one_by_uid(
        &self,
        config: &ImapConfig,
        expected_uid_validity: i64,
        uid: i64,
    ) -> Result<Option<FetchedMessage>, MailServiceError>;
    ```
    (Adding to the `#[automock]` trait auto-extends `MockInboxImapClient` — no further mock edit needed.)

    **Step 7 — Implement `fetch_one_by_uid`** on `AsyncImapClient` in `genossi_mail/src/inbox_imap.rs` (place immediately after the existing `fetch_since` impl, around line 147). Copy verbatim from RESEARCH lines 356-387 (Pattern 2). Replace local helper names if needed (e.g. `err(...)` is already in that file — reuse).

    **Step 8 — Extend `InboxService` trait** (in `inbox.rs:266-283`) with:
    ```rust
    async fn find_attachment(
        &self,
        mail_id: Uuid,
        attachment_id: Uuid,
    ) -> Result<Option<crate::dao::InboundMailAttachment>, MailServiceError>;

    async fn list_attachments(
        &self,
        mail_id: Uuid,
    ) -> Result<std::sync::Arc<[crate::dao::InboundMailAttachment]>, MailServiceError>;
    ```

    **Step 9 — Extend `InboxServiceImpl`** (around `:285-322`):
    - Add two new type parameters to the generic list: `A: InboundMailAttachmentDao` and `St: DocumentStorage`
    - Add two new fields: `attachment_dao: std::sync::Arc<A>`, `storage: std::sync::Arc<St>`
    - Extend the `pub fn new(...)` constructor to accept + assign both new dependencies
    - Implement the two new trait methods: `find_attachment` calls `self.attachment_dao.find_by_id_and_mail(mail_id, attachment_id)` and maps `MailDaoError` → `MailServiceError::DataAccess`; `list_attachments` calls `self.attachment_dao.find_by_inbound_mail_id(mail_id)` similarly

    **Step 10 — Extend the poll-worker entry** (search `inbox.rs` for the code path where `dao.create(&inbound_mail).await` succeeds inside the polling loop — likely in `poll_once` or `start_inbox_worker` inner loop). After successful mail-create, loop:
    ```rust
    for att in parsed.attachments.iter() {
        if let Err(e) = persist_attachment(
            self.storage.as_ref(),
            self.attachment_dao.as_ref(),
            inbound_mail.id,
            &att.file_name,
            &att.mime_type,
            &att.bytes,
        ).await {
            tracing::warn!(
                "inbox_poll: persist_attachment failed for mail {}: {:?}",
                inbound_mail.id, e
            );
            // D-06: continue, best-effort
        }
    }
    ```

    **Step 11 — Add 3 unit tests** in `inbox.rs::tests`:

    Test A — `test_parse_raw_mail_extracts_attachments`:
    - Construct minimal RFC 822 raw with 1 multipart attachment (e.g. inline PNG base64-encoded) via mail-parser writer or hardcoded raw string
    - Call `parse_raw_mail`
    - Assert `parsed.attachments.len() == 1`, `parsed.attachments[0].file_name == "test.png"`, `parsed.attachments[0].mime_type == "image/png"`, `parsed.attachments[0].bytes` length > 0
    - Assert `parsed.has_attachments == true`

    Test B — `test_persist_attachment_oversized_skips_storage`:
    - Use a `MockInboundMailAttachmentDao` (auto-generated from Plan 19-01) + a `MockDocumentStorage` (from genossi_service)
    - Set up: `mock_storage.expect_save().times(0);` (NO save expected for oversized)
    - Set up: `mock_dao.expect_create().times(1).returning(|_| Ok(()));`
    - Build bytes = `vec![0u8; (10 * 1024 * 1024 + 1) as usize]` (1 byte over limit)
    - Call `persist_attachment(...)` — assert Ok with `oversized=true`, `relative_path.is_none()`

    Test C — `test_persist_attachment_rollback_on_db_fail`:
    - `mock_storage.expect_save().times(1).returning(|_, _| Ok(()));`
    - `mock_storage.expect_delete().times(1).returning(|_| Ok(()));`
    - `mock_dao.expect_create().times(1).returning(|_| Err(MailDaoError::DatabaseError(Arc::from("simulated"))));`
    - Call `persist_attachment(...)` with 1 KB bytes → assert `Err(MailServiceError::DataAccess(_))` (or whatever the conversion yields)
    - Mock assertions verify save was called AND delete was called (rollback)
  </action>
  <verify>
    <automated>cargo test -p genossi_mail inbox::tests::test_parse_raw_mail_extracts_attachments inbox::tests::test_persist_attachment_oversized_skips_storage inbox::tests::test_persist_attachment_rollback_on_db_fail -- --nocapture 2>&amp;1 | tee /tmp/19-02-task1.log; grep -q "test result: ok. 3 passed" /tmp/19-02-task1.log &amp;&amp; cargo check -p genossi_mail 2>&amp;1 | tee /tmp/19-02-check.log; ! grep -q "^error" /tmp/19-02-check.log</automated>
  </verify>
  <acceptance_criteria>
    - `grep -c "pub struct ParsedAttachment" genossi_mail/src/inbox.rs` returns 1
    - `grep -c "pub attachments: Vec<ParsedAttachment>" genossi_mail/src/inbox.rs` returns 1
    - `grep -c "fn extract_attachments" genossi_mail/src/inbox.rs` returns 1
    - `grep -c "const ATTACHMENT_MAX_BYTES" genossi_mail/src/inbox.rs` returns 1 (exactly one declaration of the 10-MB cap)
    - `grep -c "10 \* 1024 \* 1024" genossi_mail/src/inbox.rs` returns ≥ 1 (D-02 value present)
    - `grep -c "fn persist_attachment" genossi_mail/src/inbox.rs` returns 1
    - `grep -c "msg.attachment_count() > 0" genossi_mail/src/inbox.rs` returns 0 (old MVP path removed)
    - `grep -c "fetch_one_by_uid" genossi_mail/src/inbox.rs` returns ≥ 1 (trait method)
    - `grep -c "fn fetch_one_by_uid" genossi_mail/src/inbox_imap.rs` returns 1 (impl)
    - `grep -c "UIDVALIDITY drift" genossi_mail/src/inbox_imap.rs` returns ≥ 1 (T-06 mitigation in error message)
    - `grep -c "async fn find_attachment" genossi_mail/src/inbox.rs` returns ≥ 2 (trait decl + impl)
    - `grep -c "async fn list_attachments" genossi_mail/src/inbox.rs` returns ≥ 2 (trait decl + impl)
    - `grep -c "persist_attachment(" genossi_mail/src/inbox.rs` returns ≥ 2 (definition + call inside poll worker)
    - `cargo test -p genossi_mail` exits 0 (all old + 3 new tests pass)
    - `cargo check -p genossi_mail --tests` exits 0
  </acceptance_criteria>
  <done>
    Mail-parser-Erweiterung läuft, persist_attachment hat 10-MB-Cap + Rollback, fetch_one_by_uid prüft UIDVALIDITY, Poll-Worker persistiert Attachments mit warn-on-Fail-continue, 3 neue Tests grün.
  </done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| IMAP server → parser | Untrusted attachment bytes from spam mails cross this boundary |
| Parser → Storage | Filenames from mail headers reach DAO + storage path |
| IMAP server → backfill | UID + UIDVALIDITY must match expected values from DB |

## STRIDE Threat Register

| Threat ID | Category | Component | Disposition | Mitigation Plan |
|-----------|----------|-----------|-------------|-----------------|
| T-01 | Denial of Service | persist_attachment | mitigate | 10 MB hard cap (`ATTACHMENT_MAX_BYTES`) — oversized rows record metadata only, NO bytes hit disk. Test `test_persist_attachment_oversized_skips_storage` asserts storage.save NOT called for oversized. |
| T-02 | Tampering (path traversal) | persist_attachment | mitigate | Storage path is `inbound_mail_attachments/{inbound_mail_id}/{attachment_id}` — both UUIDs, NEVER the attacker-supplied filename. Filename only flows into DB column + Content-Disposition (sanitized by http_util in Plan 19-03). |
| T-06 | Tampering (UID drift) | fetch_one_by_uid | mitigate | `fetch_one_by_uid` calls `open_examine_session` and reads `mailbox.uid_validity`; mismatch with `expected_uid_validity` → Err. Caller (Plan 19-04) silent-skips per D-06. |
| T-07 | Information Disclosure (orphaned files) | persist_attachment | mitigate | Save-then-DB pattern: on DB-fail, `storage.delete(rel_path).await` is invoked. Test `test_persist_attachment_rollback_on_db_fail` asserts delete called. Worst-case orphan = bounded by D-02 (≤10 MB per attempt). |

(T-03, T-04, T-05, T-08 are owned by other plans.)
</threat_model>

<verification>
- `cargo check -p genossi_mail --tests` exits 0
- `cargo test -p genossi_mail` exits 0
- `MockInboxImapClient` auto-extended via `#[automock]` — downstream tests in other modules continue to compile
- No literal `msg.attachment_count() > 0` remains in inbox.rs (D-01 MVP path removed)
</verification>

<success_criteria>
- ParsedAttachment + extract_attachments + 10-MB-cap persist_attachment in place
- fetch_one_by_uid trait method + impl with UIDVALIDITY check
- InboxService::find_attachment + list_attachments wired through to attachment_dao
- Poll worker persists attachments after mail-create with best-effort tracing::warn
- 3 unit tests pass + all existing inbox tests still green
- No Audit macro usage for InboundMailAttachment (D-10 — `grep -c "audited_create" genossi_mail/src/inbox.rs` does not introduce new matches around persist_attachment)
</success_criteria>

<output>
After completion, create `.planning/phases/19-e-mail-anh-nge-anzeigen-backend-endpoint-zum-abrufen-von-anh/19-02-SUMMARY.md`
</output>
