# Phase 27: Bild-Support Backend + Editor-Upload - Pattern Map

**Mapped:** 2026-07-23
**Files analyzed:** 16 (8 new, 8 modified)
**Analogs found:** 16 / 16 (all matched; one carries a flagged divergence)

## Divergence Flag (read first)

The `mail_asset` entity copies the `application_document` entity **verbatim across all
layers EXCEPT storage**: `application_document` persists bytes on the **filesystem** via
`relative_path: Arc<str>` + a `DocumentStorage` dependency. `mail_asset` (IMG-01) stores
bytes **inline as a SQLite BLOB** (`bytes: Vec<u8>`). When copying the analog, the planner
MUST:

- Replace field `relative_path: Arc<str>` → `bytes: Vec<u8>`; add `uploaded_by: Arc<str>`;
  rename `size: i64` → `size_bytes: i64`; drop `application_id`.
- Drop the `DocumentStorage: DocumentStorage = document_storage` line from the
  `gen_service_impl!` deps macro. No `document_storage.save()` / `.load()` calls.
- INSERT/SELECT the `bytes` column directly — SQLx binds `Vec<u8>` ↔ `BLOB` natively
  (already proven by the `id`/`version` BLOB columns in the analog, `application_document.rs:16-26`).
- The `/bytes` download reads the entity's `bytes` field directly (no filesystem load).

Warning signs of a bad copy: a `relative_path` field, a `DocumentStorage` dep, or a
`.save(&path, …)` call in any `mail_asset` file.

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `genossi_dao/src/mail_asset.rs` (new) | model / DAO trait | CRUD | `genossi_dao/src/application_document.rs` | exact (minus storage field) |
| `genossi_dao_impl_sqlite/src/mail_asset.rs` (new) | model / DAO impl | CRUD (BLOB) | `genossi_dao_impl_sqlite/src/application_document.rs` | exact (minus storage field) |
| `genossi_service/src/mail_asset.rs` (new) | service trait | request-response | `genossi_service/src/application_document.rs` | exact |
| `genossi_service_impl/src/mail_asset.rs` (new) | service | CRUD + authz | `genossi_service_impl/src/application_document.rs` | role-match (BLOB, no storage) |
| `genossi_rest/src/mail_asset.rs` (new) | controller | file-I/O (multipart up / bytes down) | `genossi_rest/src/application_document.rs` | exact |
| `genossi_rest_types/src/lib.rs` (edit) | TO type | transform | `ApplicationDocumentTO` | exact |
| `migrations/sqlite/2026072x_create_mail_assets_table.sql` (new) | migration | DDL | `20260703000000_create_application_documents_table.sql` | role-match (BLOB column) |
| `genossi_bin/src/lib.rs` (edit) | config / DI | wiring | `application_document_service` wiring | exact |
| `genossi_rest/src/lib.rs` (edit) | route + `RestStateDef` | request-response | `application_document` route nest + trait method | exact |
| `genossi_mail/src/sanitize.rs` (edit) | utility | transform | current `sanitize_html` | role-match (ammonia default → Builder) |
| `genossi_mail/src/send.rs` (edit) | service / MIME factory | transform | `build_message` 4-branch matrix | role-match (add related branch) |
| `genossi_mail/src/render.rs` (edit) | utility | transform | `plain_from_html` / CID rewrite seam | role-match |
| `genossi-frontend/src/component/mail_compose/wysiwyg_toolbar.rs` (edit) | component | event-driven | existing toolbar buttons | exact |
| `genossi-frontend/src/component/mail_compose/wysiwyg_editor.rs` (edit) | component | event-driven | existing `onpaste`/`createLink` handlers | role-match (add ondrop) |
| `genossi-frontend/src/js.rs` (edit) | utility | transform | `exec_command_str` facade | exact |
| `genossi-frontend/src/api.rs` (edit) | service / API client | file-I/O | `upload_member_document` | exact |

## Pattern Assignments

### `genossi_dao/src/mail_asset.rs` (DAO trait, CRUD)

**Analog:** `genossi_dao/src/application_document.rs`

**Entity struct** (analog lines 18-29 — swap storage field per IMG-01):
```rust
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MailAssetEntity {
    pub id: Uuid,
    pub filename: Arc<str>,
    pub mime_type: Arc<str>,
    pub size_bytes: i64,
    pub bytes: Vec<u8>,        // DIVERGENCE: inline BLOB, replaces relative_path
    pub uploaded_by: Arc<str>, // NEW: user id string
    pub created: time::PrimitiveDateTime,
    pub deleted: Option<time::PrimitiveDateTime>,
    pub version: Uuid,
}
```

**Trait + default impls** (analog lines 31-106): copy the `#[automock(...)]` + `#[async_trait]`
trait with the 3 required methods (`dump_all`, `create`, `update`) and the default `all` /
`find_by_id` (soft-delete filter `e.deleted.is_none()`). Drop `find_active_by_application_id`
(single-slot invariant is application-specific; not needed for `mail_asset`). NOT `Auditable`
— the analog explicitly does not implement it (analog doc-comment lines 15-17).

**Test fixture pattern** (analog lines 108-219): mirror the `FixtureDao` + `mock_tx()` +
`test_all_filters_soft_deleted` tests for the BLOB round-trip / soft-delete filter.

---

### `genossi_dao_impl_sqlite/src/mail_asset.rs` (DAO impl, CRUD-BLOB)

**Analog:** `genossi_dao_impl_sqlite/src/application_document.rs`

**FromRow mirror** (analog lines 15-26 — `Vec<u8>` for BLOB columns is native):
```rust
#[derive(Debug, sqlx::FromRow)]
struct MailAssetDb {
    id: Vec<u8>,
    filename: String,
    mime_type: String,
    size_bytes: i64,
    bytes: Vec<u8>,       // BLOB → Vec<u8>, native SQLx mapping (same as id/version)
    uploaded_by: String,
    created: String,
    deleted: Option<String>,
    version: Vec<u8>,
}
```

**TryFrom** (analog lines 28-44): `Uuid::from_slice(&db.id)?`, `Arc::from(...)`,
`parse_datetime(&db.created)?`. The `bytes` field maps straight through: `bytes: db.bytes.clone()`.

**dump_all SELECT** (analog lines 60-77): `SELECT id, filename, mime_type, size_bytes, bytes,
uploaded_by, created, deleted, version FROM mail_assets ORDER BY created`.

**create INSERT** (analog lines 79-117): identical bind pattern. Bind bytes with
`.bind(entity.bytes.clone())` — SQLx encodes `Vec<u8>` as BLOB (this is the single new bind vs
analog, which binds `relative_path` as a `String`). Reuse the ISO8601 `created` format at
analog lines 88-93.

**DAO round-trip test:** mirror analog's in-mem-SQLite `setup_db` pattern
(`application_document.rs:186` per RESEARCH); embed the new migration via `include_str!`.

---

### `genossi_service/src/mail_asset.rs` (service trait, request-response)

**Analog:** `genossi_service/src/application_document.rs`

Copy the trait shape: an `Upload…` input struct (fields `filename`, `mime_type`, `data: Vec<u8>`),
a domain-return struct (`MailAsset`), and `upload` / `download` / `get` methods each taking
`Authentication<Self::Context>` + `Option<Self::Transaction>`. Drop application-specific methods.

---

### `genossi_service_impl/src/mail_asset.rs` (service, CRUD + authz)

**Analog:** `genossi_service_impl/src/application_document.rs`

**gen_service_impl! deps** (analog lines 46-55 — DROP `DocumentStorage` line):
```rust
gen_service_impl! {
    struct MailAssetServiceImpl: MailAssetService = MailAssetServiceDeps {
        MailAssetDao: MailAssetDao<Transaction = Self::Transaction> = mail_asset_dao,
        // NO DocumentStorage line — bytes live inline in the entity.
        PermissionService: PermissionService<Context = Self::Context> = permission_service,
        UuidService: UuidService = uuid_service,
        TransactionDao: TransactionDao<Transaction = Self::Transaction> = transaction_dao,
    }
}
```

**CR-02 permission-first ordering** (analog lines 91-104) — the FIRST statement, before any
DAO/side-effect. Use the `"admin"` privilege string (IMG-02/04), not `"manage_members"`:
```rust
const ADMIN_PRIVILEGE: &str = "admin";
self.permission_service
    .check_permission(ADMIN_PRIVILEGE, context.clone())
    .await?;
let user_id = self.permission_service
    .current_user_id(context).await?
    .unwrap_or_else(|| "SYSTEM".to_string()); // → uploaded_by
```

**Size validation** (analog lines 107-112 — change limit to 5 MB, IMG-02):
```rust
const MAX_FILE_SIZE: usize = 5 * 1024 * 1024;
if upload.data.len() > MAX_FILE_SIZE {
    return Err(ServiceError::ValidationError(vec![ValidationFailureItem {
        field: Arc::from("file"),
        message: Arc::from("File size exceeds 5 MB limit"),
    }]));
}
```

**Create path** (analog lines 138-166 — simpler: no single-slot lookup, no `storage.save`):
`uuid_service.new_v4()` for id + version, `now_primitive()` (analog lines 71-74), build
`MailAssetEntity { bytes: upload.data, uploaded_by: user_id.into(), … }`, call `dao.create(...,
MAIL_ASSET_PROCESS, tx)`, then `transaction_dao.commit(tx)`. NO `document_storage.save(...)`.

**NEW to this file (magic-byte MIME sniff, IMG-05 / Pitfall 4)** — no analog; add a small helper
that inspects the first bytes: PNG `\x89PNG\r\n`, JPEG `\xFF\xD8\xFF`, GIF `GIF87a`/`GIF89a`.
Reject anything else; store the server-derived MIME, never the client's.

**Regression guard test:** mirror analog's `test_upload_permission_denied_has_no_side_effects`
(`application_document.rs:822` per RESEARCH) — `check_permission` fires with zero DAO calls.

---

### `genossi_rest/src/mail_asset.rs` (controller, multipart up / bytes down)

**Analog:** `genossi_rest/src/application_document.rs`

**Body limit + route** (analog lines 45-56 — 5 MB, path `/`):
```rust
const MAIL_ASSET_BODY_LIMIT: usize = 5 * 1024 * 1024;
Router::new()
    .route("/", post(upload_mail_asset::<RestState>)
        .layer(DefaultBodyLimit::max(MAIL_ASSET_BODY_LIMIT)))
    .route("/{id}/bytes", get(download_mail_asset_bytes::<RestState>))
```

**Multipart upload handler** (analog lines 89-163): copy the `while let Some(field)` loop
reading the `"file"` field into `file_data: Vec<u8>` + `file_name` (analog lines 97-119). Wrap in
`error_handler((async { … }).await)`. Diverge from analog by NOT deriving MIME from the extension
allow-list (analog lines 125-140) — pass raw bytes to the service, which magic-byte-sniffs. Return
`201` + `MailAssetTO` JSON (analog lines 154-159).

**Bytes download handler** (analog lines 181-223, bytes branch lines 206-219):
```rust
let (asset, bytes) = rest_state.mail_asset_service()
    .download(id, crate::extract_auth_context(Some(context))?, None).await?;
Response::builder()
    .status(200)
    .header("Content-Type", asset.mime_type.as_ref())
    .body(Body::from(bytes))
    .unwrap()
```
Drop the `?meta=1` branch and `Content-Disposition` attachment header (this is inline preview, not
a download). Admin-gate is enforced in the service (IMG-04).

---

### `genossi_rest_types/src/lib.rs` (TO type)

**Analog:** `ApplicationDocumentTO` (search this file). Copy the struct + `impl From<&MailAsset>`.
The upload response only needs `{ id }` (IMG-02) — a minimal TO with `id` (and optionally
`filename`/`mime_type`/`size_bytes`) is enough. Reuse the ISO8601 datetime serde already in this
file for `created`/`deleted` if exposed.

---

### `migrations/sqlite/2026072x_create_mail_assets_table.sql` (migration)

**Analog:** `migrations/sqlite/20260703000000_create_application_documents_table.sql`

Copy the table shape; `id` / `version` as `BLOB`, `created`/`deleted` as TEXT. The bytes column is
`bytes BLOB NOT NULL` (DIVERGENCE from analog's `relative_path TEXT`). Columns:
`id BLOB PK, filename TEXT, mime_type TEXT, size_bytes INTEGER, bytes BLOB, uploaded_by TEXT,
created TEXT, deleted TEXT NULL, version BLOB`. No partial unique index (that was the
single-slot invariant, not needed here).

---

### `genossi_mail/src/sanitize.rs` (utility, transform — IMG-05)

**Analog:** current `sanitize_html` (this file, lines 35-37) — replace the permissive
`ammonia::clean(html)` default with a custom `Builder`.

```rust
// Build ONCE (OnceLock) — Builder construction is not free.
ammonia::Builder::default()
    .rm_tag_attributes("img", &["src", "srcset", "alt", "width", "height", "loading"])
    .add_tag_attributes("img", &["data-genossi-asset-id"])
    .rm_url_schemes(&["data"])
    .clean(html)
    .to_string()
```

**Test pattern** (this file, lines 39-167): the existing tests (script strip, event-handler strip,
url-scheme strip, Jinja survival, list/heading survival at lines 110-167) MUST stay green. ADD
tests: `<img data-genossi-asset-id="abc">` survives (Pitfall 2 — if `add_tag_attributes` does not
whitelist `data-*` in ammonia 4, fall back to `add_generic_attribute_prefixes`); `<img
src="https://…">` → src stripped; `<img src="data:…">` stripped; `<svg>` stripped.

---

### `genossi_mail/src/send.rs` (MIME factory, transform — IMG-06/08/09)

**Analog:** `build_message` (this file, lines 52-160) — the existing 4-branch
`(html_part_opt, attachments.is_empty())` matrix (lines 110-159).

**Inline-image input struct** (mirror `LoadedAttachment`, lines 26-31):
```rust
#[derive(Clone)]
pub struct LoadedInlineImage {
    pub cid: String,        // bare, e.g. "asset-1@genossi"
    pub mime_type: Arc<str>,
    pub bytes: Vec<u8>,
}
```

**related branch** (RESEARCH Pattern 4, lettre `MultiPart::related()` + `Attachment::new_inline`):
```rust
let alternative = MultiPart::alternative().singlepart(text_part).singlepart(html_part);
let mut related = MultiPart::related().multipart(alternative);
for img in inline_images {
    let inline = Attachment::new_inline(img.cid.clone())
        .body(img.bytes.clone(), ContentType::parse(&img.mime_type)?);
    related = related.singlepart(inline);
}
```

**IMG-09 backward-compat (critical):** branch on "inline_images empty?" BEFORE building. Empty →
run the EXISTING 4-branch matrix byte-identically (lines 110-159). Non-empty → the new related
branch, wrapped in `multipart/mixed` only if document `attachments` also present. Do NOT modify the
existing `build_message` tests (lines 162+, e.g. `build_message_qp_...`, and per RESEARCH
`build_message_legacy_singlepart_text_unchanged` / `build_message_alternative_text_then_html_no_attachments`)
— they are the regression safety net.

**IMG-08:** sum `inline_images` bytes + `attachments` bytes; if `> 25 MB` return
`MailServiceError` before assembly. Error-mapping pattern: `MailServiceError::SmtpError(Arc::from(...))`
is used throughout this file (lines 65-70, 114) — reuse a validation/bad-request variant.

**Content-ID matching (Pitfall 6):** `Attachment::new_inline("asset-1@genossi")` → lettre emits
`Content-ID: <asset-1@genossi>`; the HTML must say `src="cid:asset-1@genossi"` (no brackets). Same
cid string in both places; de-dup by asset id.

---

### `genossi_mail/src/render.rs` (utility, transform — CID rewrite)

**Analog:** `plain_from_html` seam (RESEARCH cites `render.rs:207`/`:224`).

Add a pure `fn rewrite_img_cids(html) -> (rewritten_html, Vec<AssetRef>)` (RESEARCH Open Question 3
recommends a pure, unit-testable function). Run the CID rewrite ONLY for the HTML part. For the
plain-text derivation via `plain_from_html`, strip `<img>` FIRST so no `cid:`/`<img>` leaks into
plain text (RESEARCH Anti-Pattern).

---

### `genossi-frontend/src/component/mail_compose/wysiwyg_toolbar.rs` (component, event-driven — IMG-03)

**Analog:** existing toolbar buttons (this file, lines 52-152).

Copy a button verbatim (e.g. the Unordered-list button, lines 120-135). Every button MUST have
`onmousedown: move |evt| { evt.prevent_default(); }` (selection-preserve; enforced by the grep-gate
test at lines 290-345) plus `onclick` that calls `focus_editor(&editor_id_X)`, runs the command,
and calls `on_command.call(())`. Add a fresh `editor_id_m = editor_id.clone();` (clone pattern at
lines 36-47). The image button opens a hidden `<input type=file>` (or triggers the upload flow),
then inserts `<img data-genossi-asset-id="{id}" src="/api/mail/assets/{id}/bytes">` via the new
`insertHTML` helper.

---

### `genossi-frontend/src/component/mail_compose/wysiwyg_editor.rs` (component, event-driven — IMG-03 drop)

**Analog:** the `onpaste` handler (this file, lines 86-105) and the `createLink` restore-selection
block (lines 108-133).

Add `ondragover: move |evt| evt.prevent_default()` and `ondrop` on the contenteditable `div`
(lines 68-106). In `ondrop`: `evt.prevent_default()` first (mirror `onpaste` line 89), downcast to
`web_sys::DragEvent`, read `data_transfer().files()`, upload each via `api::upload_mail_asset`, then
`exec_command_str(&doc, "insertHTML", &img_html)` + `sync_from_dom(&on_change)` (mirror lines
101-104).

---

### `genossi-frontend/src/js.rs` (utility, transform)

**Analog:** `exec_command_str` (this file, lines 198-...). `insertHTML` is a valid `execCommand`
name — reuse the existing `exec_command_str` facade directly with `"insertHTML"`; no new function
needed unless a wrapper is preferred.

---

### `genossi-frontend/src/api.rs` (API client, file-I/O — IMG-02)

**Analog:** `upload_member_document` (this file, lines 339-393).

```rust
pub async fn upload_mail_asset(config: &Config, file: web_sys::File)
    -> Result<MailAssetTO, AppError>
```
Copy the `FormData` + `RequestInit(POST)` + `fetch_with_request` + `map_web_response_error` + 
`serde_wasm_bindgen::from_value` flow (lines 346-392). The one field is
`form_data.append_with_blob_and_filename("file", &file, &file.name())` (line 362). URL:
`{}/api/mail/assets`.

---

### `genossi_bin/src/lib.rs` + `genossi_rest/src/lib.rs` (DI wiring + route)

**Analog:** `application_document_service` wiring (RESEARCH cites `genossi_bin/src/lib.rs:559, 845,
936`) + `RestStateDef` trait method (`genossi_rest/src/lib.rs:229, 261`) + route nest
(`genossi_rest/src/lib.rs:626`).

Wire `mail_asset_dao` (no `document_storage`) → `MailAssetServiceImpl` → `RestStateImpl`. Add
`fn mail_asset_service(&self)` to `RestStateDef`. Nest the router under `/api/mail/assets`. The
global `forbid_unauthenticated` layer (`genossi_rest/src/lib.rs:721`) already applies; admin-gate is
in the service.

## Shared Patterns

### Admin Authorization (CR-02 permission-first)
**Source:** `genossi_service_impl/src/application_document.rs:91-104`
**Apply to:** every `mail_asset` service method (`upload`, `download`, `get`).
`check_permission("admin", context.clone()).await?` is the FIRST statement — no side effects (not
even `current_user_id`) before it. Regression guard test at `application_document.rs:822`.

### Error Handling (REST → error_handler wrapper)
**Source:** `genossi_rest/src/application_document.rs:95-161`
**Apply to:** all `mail_asset` REST handlers. Wrap handler logic in
`error_handler((async { … }).await)`; return `RestError::BadRequest(...)` on multipart failures,
`RestError::UnsupportedMediaType(...)` on bad MIME (415), `RestError::NotFound` on missing asset.

### Optimistic-lock + soft-delete (DAO)
**Source:** `genossi_dao/src/application_document.rs:59-89`
**Apply to:** `mail_asset` DAO `update` (version WHERE clause → `DaoError::ConflictError` on
mismatch) and the default `all` / `find_by_id` (filter `deleted.is_none()`).

### Datetime handling
**Source:** `genossi_dao_impl_sqlite/src/application_document.rs:39, 88-93` +
`genossi_rest_types` ISO8601 serde
**Apply to:** `mail_asset` DAO (parse `created`/`deleted` via `parse_datetime`, format created via
`Iso8601::DEFAULT`) and TO serialization.

### Toolbar selection-preserve invariant
**Source:** `genossi-frontend/src/component/mail_compose/wysiwyg_toolbar.rs:55` + grep-gate test
lines 290-345
**Apply to:** the new image toolbar button — MUST include
`onmousedown: move |evt| { evt.prevent_default(); }` or the grep-gate test fails.

## No Analog Found

None. All 16 files have a codebase analog. The single genuinely new code fragments (each grafted
into an existing analog, not a new file):

| Fragment | Host file | Reason no direct analog |
|----------|-----------|-------------------------|
| Inline BLOB storage (`bytes: Vec<u8>`) | `mail_asset.rs` (dao / dao_impl / service) | No existing entity stores binary inline; all documents use filesystem `relative_path`. SQLx `Vec<u8>` ↔ BLOB is proven for id/version columns. |
| Magic-byte MIME sniff (PNG/JPEG/GIF) | `genossi_service_impl/src/mail_asset.rs` | Analog derives MIME from filename extension; IMG-05 needs content inspection (no SVG/polyglot). |
| Custom ammonia `Builder` `<img>` rule | `genossi_mail/src/sanitize.rs` | Current sanitizer uses the permissive `ammonia::clean()` default. |
| `multipart/related` + CID inline branch | `genossi_mail/src/send.rs` / `render.rs` | Existing matrix stops at `multipart/alternative`. |

## Metadata

**Analog search scope:** `genossi_dao`, `genossi_dao_impl_sqlite`, `genossi_service`,
`genossi_service_impl`, `genossi_rest`, `genossi_rest_types`, `genossi_mail`, `genossi-frontend`,
`migrations/sqlite`, `genossi_bin`
**Files scanned (read for excerpts):** `application_document.rs` (dao, dao_impl_sqlite, service_impl,
rest), `sanitize.rs`, `send.rs`, `api.rs`, `wysiwyg_toolbar.rs`, `wysiwyg_editor.rs`, `js.rs` (grep)
**Pattern extraction date:** 2026-07-23
