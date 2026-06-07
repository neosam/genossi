---
phase: 19-e-mail-anhaenge-anzeigen
plan: 19-03
slug: rest-endpoints
type: execute
wave: 3
depends_on: [19-02]
files_modified:
  - genossi_rest/src/http_util.rs
  - genossi_mail/src/inbox_rest.rs
  - genossi_bin/src/lib.rs
  - genossi_bin/tests/e2e_tests.rs
autonomous: true
requirements: []
must_haves:
  truths:
    - "`InboundMailDetailTO` carries `attachments: Vec<InboundMailAttachmentTO>` populated by service.list_attachments (D-07)"
    - "`GET /api/inbox/{mail_id}/attachments/{attachment_id}` returns bytes with `Content-Disposition: attachment; …` by default (D-08)"
    - "Same endpoint with `?disposition=inline` switches to `Content-Disposition: inline; …` — single endpoint, both modes (D-08)"
    - "Endpoint reuses the Vorstand-only auth middleware that already protects `GET /api/inbox/{id}` — no new permission code (D-09, T-04)"
    - "Service uses `find_by_id_and_mail` so requests for (mail_B, attachment_A) return 404 (T-03 IDOR mitigation)"
    - "Oversized attachments (relative_path NULL) → 410 GONE response (not 404 — semantic clarity: file was rejected at receive)"
    - "Content-Disposition filename encoded via http_util helpers — never raw concatenation (T-02)"
    - "RestState wires `inbound_attachment_dao` + extends `InboxRestState`-trait so the handler can reach DAO + storage"
  artifacts:
    - path: "genossi_rest/src/http_util.rs"
      provides: "content_disposition_inline helper"
      contains: "pub fn content_disposition_inline"
    - path: "genossi_mail/src/inbox_rest.rs"
      provides: "InboundMailAttachmentTO, extended InboundMailDetailTO, download_attachment handler, route registration"
      contains: "pub struct InboundMailAttachmentTO"
    - path: "genossi_bin/src/lib.rs"
      provides: "RestStateImpl wires inbound_attachment_dao + exposes via InboxRestState trait"
      contains: "inbound_attachment_dao"
    - path: "genossi_bin/tests/e2e_tests.rs"
      provides: "seed_inbound_mail_attachment helper + 4 E2E tests (default disposition, inline disposition, IDOR cross-mail, oversized 410)"
      contains: "fn seed_inbound_mail_attachment"
  key_links:
    - from: "GET /api/inbox/{id} (existing handler)"
      to: "service.list_attachments → DetailTO.attachments"
      via: "to_detail_to extension"
      pattern: "list_attachments"
    - from: "GET /api/inbox/{mail_id}/attachments/{attachment_id}"
      to: "DocumentStorage::load"
      via: "service.find_attachment → storage.load → axum Response"
      pattern: "download_attachment"
    - from: "Inbox router"
      to: "Vorstand-only auth middleware"
      via: "existing forbid_unauthenticated stack already covering /api/inbox/*"
      pattern: "/inbox.*attachments"

---

<objective>
Liefere den Download-Endpunkt + embed Attachment-Liste in `InboundMailDetailTO`,
und mappe das Ganze ins `genossi_bin`-Wiring + E2E-Tests.

Purpose: Frontend (Plan 19-05 / 19-06) braucht eine fertige, abgesicherte API
mit beiden Disposition-Modi. E2E-Tests verifizieren Vorstand-only-Auth,
Disposition-Switch, Cross-Mail-IDOR (T-03) und Oversized-Handling.

Output: 1 neuer http_util-Helper, erweiterte DetailTO, 1 neuer Axum-Handler
+ Route, RestStateImpl-Field + InboxRestState-Trait-Methoden, 4 E2E-Tests.
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
@.planning/phases/19-e-mail-anh-nge-anzeigen-backend-endpoint-zum-abrufen-von-anh/19-02-SUMMARY.md
@CLAUDE.md

<interfaces>
<!-- Pre-extracted analog API shapes — executor mirrors verbatim, no exploration. -->

From `genossi_rest/src/http_util.rs:43-50` (analog — mirror for inline):
```rust
pub fn content_disposition_attachment(filename: &str) -> String {
    let ascii_fallback = sanitize_ascii_filename(filename);
    let utf8_encoded = percent_encode_utf8(filename);
    format!(
        "attachment; filename=\"{}\"; filename*=UTF-8''{}",
        ascii_fallback, utf8_encoded
    )
}
```

From `genossi_mail/src/inbox_rest.rs:37-51` (existing `InboundMailDetailTO`):
```rust
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
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

From `genossi_mail/src/inbox_rest.rs:91-106` (`to_detail_to` — extend to take attachments arg):
```rust
fn to_detail_to(mail: &InboundMail, assigned_name: Option<String>) -> InboundMailDetailTO {
    InboundMailDetailTO {
        id: mail.id.to_string(),
        // ... existing field mappings ...
    }
}
```

From `genossi_mail/src/inbox_rest.rs:126-139` (map_error — use this, NOT genossi_rest::error_handler):
```rust
fn map_error(e: MailServiceError) -> Response {
    let (code, msg) = match e {
        MailServiceError::NotFound => (StatusCode::NOT_FOUND, "not found".to_string()),
        MailServiceError::ConfigMissing(k) => (StatusCode::SERVICE_UNAVAILABLE, format!("missing config: {}", k)),
        MailServiceError::SmtpError(m) => (StatusCode::BAD_GATEWAY, m.to_string()),
        MailServiceError::DataAccess(m) => (StatusCode::INTERNAL_SERVER_ERROR, m.to_string()),
        MailServiceError::TemplateValidation(m) => (StatusCode::BAD_REQUEST, m.to_string()),
        MailServiceError::BadRequest(m) => (StatusCode::BAD_REQUEST, m.to_string()),
    };
    (code, msg).into_response()
}
```

From `genossi_rest/src/member_document.rs:232-267` (download-handler analog):
```rust
let data = rest_state.document_storage().load(&doc.relative_path).await
    .map_err(|e| RestError::InternalError(format!("Failed to load file: {}", e)))?;
let content_disposition = crate::http_util::content_disposition_attachment(&doc.file_name);
Ok(Response::builder().status(200)
    .header("Content-Type", doc.mime_type.as_ref())
    .header("Content-Disposition", &content_disposition)
    .body(Body::from(data)).unwrap())
```

From `genossi_bin/src/lib.rs:1344-1351` (existing spawn pattern + RestStateImpl construction):
- Type alias `InboundMailDaoType` is defined at `:580` — add `InboundMailAttachmentDaoType` next to it
- `RestStateImpl` field block — add `inbound_attachment_dao: Arc<InboundMailAttachmentDaoType>`
- Constructor wiring at `:1079-1095`
- `impl InboxRestState for RestStateImpl` — extend to expose `inbound_attachment_dao()` + `document_storage()`

`InboxRestState` trait shape is at `inbox_rest.rs:112-120` — add 2 accessor methods.
</interfaces>
</context>

<tasks>

<task type="auto" tdd="true">
  <name>Task 1: content_disposition_inline helper + 4 unit tests</name>
  <files>genossi_rest/src/http_util.rs</files>
  <read_first>
    - genossi_rest/src/http_util.rs:1-180 (whole module — sanitize_ascii_filename + percent_encode_utf8 + existing attachment helper + existing tests block)
    - .planning/phases/19-e-mail-anh-nge-anzeigen-backend-endpoint-zum-abrufen-von-anh/19-RESEARCH.md §Pattern 5 (lines 491-509)
    - .planning/phases/19-e-mail-anh-nge-anzeigen-backend-endpoint-zum-abrufen-von-anh/19-PATTERNS.md §7
  </read_first>
  <behavior>
    - New public fn `content_disposition_inline(filename: &str) -> String` mirrors `content_disposition_attachment` exactly, only the `kind` string changes
    - Uses the same `sanitize_ascii_filename` + `percent_encode_utf8` helpers (no duplication)
    - Output format: `inline; filename="<ASCII>"; filename*=UTF-8''<percent-encoded>`
    - 4 new unit tests cover: simple filename, umlaut filename, quote-in-filename, newline/CR-LF in filename (T-05 header-injection guard mirrors existing attachment tests)
  </behavior>
  <action>
    **Step 1 — Add the new fn** immediately after the existing `content_disposition_attachment` (around line 50):
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

    **Step 2 — Add 4 unit tests** in the existing `#[cfg(test)] mod tests` block (mirror naming of the existing attachment tests):
    - `test_inline_simple_filename`: `content_disposition_inline("invoice.pdf")` contains `inline; filename="invoice.pdf"` and `filename*=UTF-8''`
    - `test_inline_umlaut_filename`: `content_disposition_inline("Rückzahlung.pdf")` — ASCII fallback contains `_` for the umlaut OR the exact sanitized form used by the existing attachment test (mirror that test's expected output); UTF-8 part contains `%C3%BC` for ü
    - `test_inline_quote_in_filename`: `content_disposition_inline("a\"b.pdf")` — ASCII fallback has the `"` removed/replaced (mirror attachment-test's quote-handling logic)
    - `test_inline_newline_in_filename`: `content_disposition_inline("a\r\nb.pdf")` — CR/LF percent-encoded or stripped (header-injection guard, T-05) — assert result does NOT contain literal `\r` or `\n` characters
  </action>
  <verify>
    <automated>cargo test -p genossi_rest http_util::tests::test_inline_simple_filename http_util::tests::test_inline_umlaut_filename http_util::tests::test_inline_quote_in_filename http_util::tests::test_inline_newline_in_filename 2>&amp;1 | tee /tmp/19-03-task1.log; grep -q "test result: ok. 4 passed" /tmp/19-03-task1.log</automated>
  </verify>
  <acceptance_criteria>
    - `grep -c "pub fn content_disposition_inline" genossi_rest/src/http_util.rs` returns 1
    - `grep -c "inline; filename=" genossi_rest/src/http_util.rs` returns ≥ 1 (the format-string in the new fn)
    - `grep -c "test_inline_simple_filename\\|test_inline_umlaut_filename\\|test_inline_quote_in_filename\\|test_inline_newline_in_filename" genossi_rest/src/http_util.rs` returns 4
    - `cargo test -p genossi_rest http_util` exits 0 (old + 4 new tests pass)
  </acceptance_criteria>
  <done>
    Helper exists, sibling of attachment-Helper, 4 Tests grün inkl. Header-Injection-Guard.
  </done>
</task>

<task type="auto" tdd="true">
  <name>Task 2: DetailTO extension + download_attachment handler + route + RestState wiring</name>
  <files>
    genossi_mail/src/inbox_rest.rs,
    genossi_bin/src/lib.rs
  </files>
  <read_first>
    - genossi_mail/src/inbox_rest.rs:37-51 (existing `InboundMailDetailTO`)
    - genossi_mail/src/inbox_rest.rs:91-106 (`to_detail_to`)
    - genossi_mail/src/inbox_rest.rs:112-120 (`InboxRestState` trait — extend with 2 accessors)
    - genossi_mail/src/inbox_rest.rs:126-139 (`map_error` — reuse, do NOT use genossi_rest::error_handler)
    - genossi_mail/src/inbox_rest.rs:170-220 (existing handlers — get_inbox at :170-195 + Router::new chain — register new route here)
    - genossi_rest/src/member_document.rs:220-267 (download-handler analog)
    - genossi_bin/src/lib.rs:575-600 (type aliases — add InboundMailAttachmentDaoType here)
    - genossi_bin/src/lib.rs:660-680 (RestStateImpl field block — add inbound_attachment_dao field)
    - genossi_bin/src/lib.rs:1079-1095 (ctor wiring — instantiate + assign new DAO)
    - genossi_bin/src/lib.rs (search `impl InboxRestState for RestStateImpl` — extend trait impl)
    - .planning/phases/19-e-mail-anh-nge-anzeigen-backend-endpoint-zum-abrufen-von-anh/19-RESEARCH.md §Pattern 3 (lines 396-426) + §Pattern 4 (lines 436-489)
    - .planning/phases/19-e-mail-anh-nge-anzeigen-backend-endpoint-zum-abrufen-von-anh/19-PATTERNS.md §6 + §8
  </read_first>
  <behavior>
    - New TO `InboundMailAttachmentTO { id: String, file_name: String, mime_type: String, size_bytes: i64, oversized: bool }` derived with `Clone, Debug, Serialize, Deserialize, ToSchema` (D-07)
    - `InboundMailDetailTO` gains `pub attachments: Vec<InboundMailAttachmentTO>` field
    - Free fn `to_attachment_to(&InboundMailAttachment) -> InboundMailAttachmentTO` converts entity → TO
    - `to_detail_to` signature changes to `to_detail_to(mail, assigned_name, attachments)` — caller must populate before invoking
    - `get_inbox` handler is updated: after fetching the mail, also call `state.inbox_service().list_attachments(mail_uuid)`, map each to TO, and pass into `to_detail_to`
    - New handler `download_attachment` lives in `inbox_rest.rs`:
      - Extract: `State<S: InboxRestState>`, `Path((mail_id, attachment_id))`, `Query(q: DispositionQuery)` where `DispositionQuery { disposition: Option<String> }`
      - Parse both UUIDs (400 on parse-error)
      - Call `state.inbox_service().find_attachment(mail_uuid, att_uuid)` — Ok(None) → 404, Err → `map_error`
      - If `att.oversized || att.relative_path.is_none()` → return `(StatusCode::GONE, "attachment was rejected for size at receive")` (410 GONE per UI feedback; semantic difference from 404)
      - Else `state.document_storage().load(&rel_path)` → on `StorageError::NotFound` return 404, on other Err return 500
      - Build `Content-Disposition` via match on `q.disposition.as_deref()`: `Some("inline")` → `http_util::content_disposition_inline`, anything else → `http_util::content_disposition_attachment`
      - Response: `Response::builder().status(200).header("Content-Type", att.mime_type.as_ref()).header("Content-Disposition", header).body(Body::from(bytes)).unwrap()`
    - Route registered on the existing Router chain in `inbox_rest.rs`: `.route("/{mail_id}/attachments/{attachment_id}", get(download_attachment))`
    - `#[utoipa::path]` annotation: tag "inbox", params (mail_id Path String, attachment_id Path String, disposition Query Option<String>), responses (200 binary, 401, 404, 410)
    - `InboxRestState` trait extended with `fn inbound_attachment_dao(&self) -> Arc<dyn InboundMailAttachmentDao>` AND `fn document_storage(&self) -> Arc<dyn DocumentStorage>` (or whatever name already exists — verify by reading the trait at :112-120). The service is what the handler calls directly via `state.inbox_service()`.
    - `RestStateImpl` (in `genossi_bin/src/lib.rs`): new type alias, new field `inbound_attachment_dao: Arc<InboundMailAttachmentDaoType>`, ctor builds the DAO from the pool, wires it into the `InboxServiceImpl::new(...)` call so the service can use it (Plan 19-02 added the dependency)
  </behavior>
  <action>
    **Step 1 — Add new TO + extend DetailTO** in `genossi_mail/src/inbox_rest.rs` just before the existing `InboundMailDetailTO`:
    ```rust
    #[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
    pub struct InboundMailAttachmentTO {
        pub id: String,
        pub file_name: String,
        pub mime_type: String,
        pub size_bytes: i64,
        pub oversized: bool,
    }
    ```
    Then in `InboundMailDetailTO` add the field `pub attachments: Vec<InboundMailAttachmentTO>,` (next to `has_attachments`).

    **Step 2 — Add converter** as a free fn near `to_detail_to`:
    ```rust
    fn to_attachment_to(a: &crate::dao::InboundMailAttachment) -> InboundMailAttachmentTO {
        InboundMailAttachmentTO {
            id: a.id.to_string(),
            file_name: a.file_name.to_string(),
            mime_type: a.mime_type.to_string(),
            size_bytes: a.size_bytes,
            oversized: a.oversized,
        }
    }
    ```

    **Step 3 — Update `to_detail_to`** signature to:
    ```rust
    fn to_detail_to(
        mail: &InboundMail,
        assigned_name: Option<String>,
        attachments: Vec<InboundMailAttachmentTO>,
    ) -> InboundMailDetailTO { /* … existing field map, plus: */ attachments, /* … */ }
    ```

    **Step 4 — Update `get_inbox` handler** (currently around :170-195): after the existing service call that loads the mail, add:
    ```rust
    let atts = state.inbox_service().list_attachments(mail_uuid).await.map_err(map_error)?;
    let attachment_tos: Vec<_> = atts.iter().map(to_attachment_to).collect();
    ```
    Then pass `attachment_tos` as the third arg to `to_detail_to(...)`. (If the existing handler uses `match` instead of `?`, mirror the match style — read the actual code before editing.)

    **Step 5 — Add `download_attachment` handler** near the bottom of `inbox_rest.rs`, after the existing handlers. Full body:
    ```rust
    #[derive(Deserialize)]
    struct DispositionQuery {
        disposition: Option<String>,
    }

    #[utoipa::path(
        get,
        path = "/{mail_id}/attachments/{attachment_id}",
        tag = "inbox",
        params(
            ("mail_id" = String, Path, description = "Inbound mail id"),
            ("attachment_id" = String, Path, description = "Attachment id"),
            ("disposition" = Option<String>, Query, description = "inline | attachment (default attachment)"),
        ),
        responses(
            (status = 200, description = "Binary attachment bytes"),
            (status = 401, description = "Unauthenticated"),
            (status = 404, description = "Not found"),
            (status = 410, description = "Attachment was rejected as oversized at receive"),
        ),
    )]
    async fn download_attachment<S: InboxRestState>(
        State(state): State<S>,
        Path((mail_id, attachment_id)): Path<(String, String)>,
        Query(q): Query<DispositionQuery>,
    ) -> Response {
        use axum::http::StatusCode;
        use axum::body::Body;
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
        if att.oversized || att.relative_path.is_none() {
            return (StatusCode::GONE, "attachment was rejected for size at receive").into_response();
        }
        let rel_path = att.relative_path.as_ref().expect("checked above").to_string();
        let bytes = match state.document_storage().load(&rel_path).await {
            Ok(b) => b,
            Err(genossi_service::document_storage::StorageError::NotFound) =>
                return (StatusCode::NOT_FOUND, "file not found").into_response(),
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("storage: {}", e)).into_response(),
        };
        let header = match q.disposition.as_deref() {
            Some("inline") => crate::http_util_inline(&att.file_name),
            _ => crate::http_util_attachment(&att.file_name),
        };
        Response::builder()
            .status(200)
            .header("Content-Type", att.mime_type.as_ref())
            .header("Content-Disposition", header)
            .body(Body::from(bytes))
            .unwrap()
    }
    ```

    NOTE: the `crate::http_util_*` placeholders in the body MUST be replaced with calls through the `InboxRestState` trait. **`genossi_rest` already depends on `genossi_mail`** (verified: `genossi_rest/Cargo.toml` declares `genossi_mail = { path = "../genossi_mail" }`). Adding the reverse dependency would cause a circular crate dependency, so direct `use genossi_rest::http_util::…` is NOT possible inside `genossi_mail`.

    **Locked approach — extend `InboxRestState` trait with two accessor methods** (sibling to `document_storage()`):
    ```rust
    fn content_disposition_attachment(&self, filename: &str) -> String;
    fn content_disposition_inline(&self, filename: &str) -> String;
    ```
    In the handler body, replace the placeholders with:
    ```rust
    let header = match q.disposition.as_deref() {
        Some("inline") => state.content_disposition_inline(&att.file_name),
        _ => state.content_disposition_attachment(&att.file_name),
    };
    ```
    `RestStateImpl` (in `genossi_bin/src/lib.rs`) implements both methods by delegating to `genossi_rest::http_util::content_disposition_attachment` / `_inline`. `genossi_bin` already depends on both `genossi_mail` and `genossi_rest` — no Cargo.toml change needed.

    **Step 6 — Register the route** in the existing Router::new() chain in `inbox_rest.rs` (find the line that mounts `get_inbox` on `/{id}`). Add:
    ```rust
    .route("/{mail_id}/attachments/{attachment_id}", get(download_attachment))
    ```

    **Step 7 — Extend `InboxRestState` trait** (at `:112-120`) with THREE new accessor methods:
    ```rust
    fn document_storage(&self) -> std::sync::Arc<dyn genossi_service::document_storage::DocumentStorage>;
    fn content_disposition_attachment(&self, filename: &str) -> String;
    fn content_disposition_inline(&self, filename: &str) -> String;
    ```
    The two `content_disposition_*` methods exist so the handler in `genossi_mail` can build the header WITHOUT importing `genossi_rest` (which would create a circular crate dep — see Step 5 note). `RestStateImpl` in `genossi_bin` (Step 8) implements both by delegating to `genossi_rest::http_util::content_disposition_attachment` / `_inline`.

    (Verify whether `inbox_service()` already returns a service that has `find_attachment` + `list_attachments` from Plan 19-02. It does — those were added to the service trait.)

    **Step 8 — Wire `genossi_bin/src/lib.rs`**:
    - At `:580` (next to `type InboundMailDaoType = …`) add:
      ```rust
      type InboundMailAttachmentDaoType = genossi_mail::dao_sqlite::InboundMailAttachmentDaoSqlite;
      ```
    - At the `RestStateImpl` struct field block (~:660-680) add: `inbound_attachment_dao: Arc<InboundMailAttachmentDaoType>,`
    - At the constructor (~:1079-1095), add: `let inbound_attachment_dao = Arc::new(InboundMailAttachmentDaoType::new(pool.clone()));`
    - Pass the new DAO into the `InboxServiceImpl::new(...)` call (Plan 19-02 added it as a required ctor arg)
    - Add to the `RestStateImpl { … }` struct-literal: `inbound_attachment_dao,`
    - In `impl InboxRestState for RestStateImpl`: add three new methods:
      ```rust
      fn document_storage(&self) -> Arc<dyn DocumentStorage> { self.document_storage.clone() }
      fn content_disposition_attachment(&self, filename: &str) -> String {
          genossi_rest::http_util::content_disposition_attachment(filename)
      }
      fn content_disposition_inline(&self, filename: &str) -> String {
          genossi_rest::http_util::content_disposition_inline(filename)
      }
      ```
      (use the existing `document_storage` field at :614 per PATTERNS.md §8; `genossi_rest` is already a dep of `genossi_bin`, so no Cargo.toml change.)

    **Step 9 — Add E2E helper** `seed_inbound_mail_attachment` in `genossi_bin/tests/e2e_tests.rs` near `seed_inbound_mail` (around :4646). Signature:
    ```rust
    async fn seed_inbound_mail_attachment(
        pool: &SqlitePool,
        storage_path: &Path,
        mail_id: Uuid,
        file_name: &str,
        mime: &str,
        bytes: &[u8],
        oversized: bool,
    ) -> Uuid { /* INSERT into inbound_mail_attachments; if !oversized: write file to storage_path/inbound_mail_attachments/{mail_id}/{att_id}; return att_id */ }
    ```

    **Step 10 — Add 4 E2E tests** in `e2e_tests.rs`:

    E2E A — `test_get_inbox_detail_includes_attachments`:
    1. Bootstrap test server (existing helper)
    2. Seed mail + 2 attachments (one normal, one oversized=true)
    3. Authenticate as Vorstand (existing helper)
    4. `GET /api/inbox/{mail_id}` → assert 200, JSON body `attachments.len() == 2`, fields correct (`oversized` matches)

    E2E B — `test_download_attachment_default_disposition_is_attachment`:
    1. Seed mail + 1 attachment with bytes `b"hello world"` and mime `text/plain`
    2. `GET /api/inbox/{mid}/attachments/{aid}` (no query param)
    3. Assert: status 200, `Content-Type: text/plain`, `Content-Disposition` starts with `attachment;`, body bytes == `b"hello world"`

    E2E C — `test_download_attachment_inline_query_switches_disposition`:
    1. Same seed as B
    2. `GET /api/inbox/{mid}/attachments/{aid}?disposition=inline`
    3. Assert: status 200, `Content-Disposition` starts with `inline;`, body unchanged

    E2E D — `test_download_attachment_cross_mail_returns_404` (T-03 IDOR):
    1. Seed mail A + attachment A1 (with bytes); seed mail B (no attachments)
    2. `GET /api/inbox/{mail_B_id}/attachments/{a1_id}` → assert status 404
    3. Positive control: `GET /api/inbox/{mail_A_id}/attachments/{a1_id}` → assert 200

    E2E E — `test_download_attachment_oversized_returns_410`:
    1. Seed mail + 1 attachment with `oversized=true, relative_path=NULL` (no bytes on disk)
    2. `GET /api/inbox/{mid}/attachments/{aid}` → assert status 410 GONE
  </action>
  <verify>
    <automated>cargo test -p genossi_bin --test e2e_tests test_get_inbox_detail_includes_attachments test_download_attachment_default_disposition_is_attachment test_download_attachment_inline_query_switches_disposition test_download_attachment_cross_mail_returns_404 test_download_attachment_oversized_returns_410 -- --nocapture 2>&amp;1 | tee /tmp/19-03-task2.log; grep -q "test result: ok. 5 passed" /tmp/19-03-task2.log &amp;&amp; cargo check -p genossi_bin 2>&amp;1 | tee /tmp/19-03-check.log; ! grep -q "^error" /tmp/19-03-check.log</automated>
  </verify>
  <acceptance_criteria>
    - `grep -c "pub struct InboundMailAttachmentTO" genossi_mail/src/inbox_rest.rs` returns 1
    - `grep -c "pub attachments: Vec<InboundMailAttachmentTO>" genossi_mail/src/inbox_rest.rs` returns 1
    - `grep -c "fn to_attachment_to" genossi_mail/src/inbox_rest.rs` returns 1
    - `grep -c "async fn download_attachment" genossi_mail/src/inbox_rest.rs` returns 1
    - `grep -c "/{mail_id}/attachments/{attachment_id}" genossi_mail/src/inbox_rest.rs` returns ≥ 1 (route registration)
    - `grep -c "StatusCode::GONE" genossi_mail/src/inbox_rest.rs` returns ≥ 1 (oversized → 410)
    - `grep -c "DispositionQuery" genossi_mail/src/inbox_rest.rs` returns ≥ 2 (struct + extractor use)
    - `grep -c "InboundMailAttachmentDaoType" genossi_bin/src/lib.rs` returns ≥ 3 (alias + field type + ctor)
    - `grep -c "inbound_attachment_dao" genossi_bin/src/lib.rs` returns ≥ 3 (field decl + ctor assign + struct-literal)
    - `grep -c "test_get_inbox_detail_includes_attachments\\|test_download_attachment_default_disposition_is_attachment\\|test_download_attachment_inline_query_switches_disposition\\|test_download_attachment_cross_mail_returns_404\\|test_download_attachment_oversized_returns_410" genossi_bin/tests/e2e_tests.rs` returns ≥ 5
    - `cargo check -p genossi_mail` exits 0
    - `cargo check -p genossi_bin` exits 0
    - `cargo test -p genossi_bin --test e2e_tests` exits 0 (5 new + existing tests pass)
  </acceptance_criteria>
  <done>
    Endpoint live, DetailTO trägt Attachments, Wiring vollständig, 5 E2E-Tests grün inkl. IDOR + Disposition-Switch + Oversized-410.
  </done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| Client → Axum router | Untrusted UUIDs + disposition query param |
| Service → Storage | rel_path string read from DB column |

## STRIDE Threat Register

| Threat ID | Category | Component | Disposition | Mitigation Plan |
|-----------|----------|-----------|-------------|-----------------|
| T-02 | Tampering (header injection) | download_attachment handler | mitigate | `Content-Disposition` filename routed through `http_util::content_disposition_attachment` / `_inline` — both helpers percent-encode UTF-8 + strip CRLF (verified by unit tests in Task 1). |
| T-03 | Information Disclosure (cross-mail IDOR) | download_attachment handler + service.find_attachment | mitigate | Service uses DAO `find_by_id_and_mail(mail_id, attachment_id)` — mismatched mail/attachment pair returns Ok(None) → 404. E2E D verifies. |
| T-04 | Spoofing (unauthorized download) | Router middleware | mitigate | Route registered on the existing `/api/inbox` router that already runs the Vorstand-only `forbid_unauthenticated` middleware (D-09). No new permission code. E2E suite uses authenticated client — adding unauthed assertion is OPTIONAL but recommended; verify existing E2E pattern covers 401 already. |

(T-01, T-05, T-06, T-07 owned by other plans.)
</threat_model>

<verification>
- `cargo check -p genossi_rest` exits 0
- `cargo check -p genossi_mail` exits 0
- `cargo check -p genossi_bin` exits 0
- `cargo test -p genossi_rest http_util` exits 0 (4 new inline tests + existing)
- `cargo test -p genossi_bin --test e2e_tests` exits 0 (5 new tests + existing)
- OpenAPI annotation present on `download_attachment` (`#[utoipa::path]` block)
- Route `/{mail_id}/attachments/{attachment_id}` registered on the existing inbox Router
- `grep -c "audited_create\\|audited_update\\|audited_delete" genossi_mail/src/inbox_rest.rs` returns 0 (D-10 — no audit on read endpoint)
</verification>

<success_criteria>
- DetailTO carries `attachments` populated by service
- Single endpoint serves both dispositions via `?disposition=inline`
- 410 GONE for oversized rows
- IDOR cross-mail test passes
- Vorstand-only auth reused (D-09)
- All filename-encoding goes through http_util (T-02)
- 5 E2E tests pass
</success_criteria>

<output>
After completion, create `.planning/phases/19-e-mail-anh-nge-anzeigen-backend-endpoint-zum-abrufen-von-anh/19-03-SUMMARY.md`
</output>
