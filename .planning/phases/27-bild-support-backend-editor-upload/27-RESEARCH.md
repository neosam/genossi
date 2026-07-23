# Phase 27: Bild-Support Backend + Editor-Upload - Research

**Researched:** 2026-07-23
**Domain:** Rust backend (new SQLite BLOB entity + REST multipart upload + ammonia hardening + CID renderer + lettre multipart/related) + Dioxus WASM frontend (WYSIWYG image upload)
**Confidence:** HIGH (all recommendations grounded in the actual codebase with file:line citations; lettre + ammonia APIs cross-checked against docs.rs)

## Summary

Phase 27 adds inline image support to the existing HTML-mail pipeline. It is ~80% backend. The requirement IDs IMG-01..IMG-09 lock most design decisions verbatim (entity fields, MIME whitelist, size limits, CID scheme, MIME structure, admin-only, no audit). This research maps every requirement to an existing analog in the codebase so the planner can replicate proven patterns rather than invent new ones.

Three analogs carry almost all the weight: (1) **`application_document`** (Phase 25) is the exact DAO/Service/REST template for a *non-audited* entity with optimistic locking and soft-delete — but it stores bytes on the **filesystem** via `relative_path`, whereas IMG-01 requires bytes stored **inline as a SQLite BLOB**. This is the single genuinely new pattern in the phase (no existing entity stores binary inline; SQLx handles `Vec<u8>` ↔ `BLOB` natively, already proven for the `id`/`version` columns). (2) **`build_message`** in `genossi_mail/src/send.rs:52` is the single MIME factory; its existing 4-branch `(html_body, attachments)` matrix must grow a `multipart/related` layer for images while keeping the no-image path byte-identical (IMG-09). (3) **`sanitize.rs`** currently calls the permissive `ammonia::clean()` default, which *already allows* `<img src="https://…">` — IMG-05 forces a switch to a custom `ammonia::Builder` that strips `src`/`data:`/external-http and allows only `data-genossi-asset-id`.

**Primary recommendation:** Build `mail_asset` as a standalone non-audited entity in the **main** layers (`genossi_dao` → `genossi_dao_impl_sqlite` → `genossi_service` → `genossi_service_impl` → `genossi_rest`), copying `application_document` verbatim but swapping `relative_path: Arc<str>` + `DocumentStorage` for `bytes: Vec<u8>` stored inline in the BLOB column (no filesystem). Keep the CID transform + `multipart/related` assembly inside `genossi_mail` (`send.rs`/`render.rs`), fed loaded asset bytes the same way `LoadedAttachment` feeds `build_message` today. Do NOT thread a new DAO generic through `MailServiceImpl` — the send path fetches asset bytes via the main-layer service, mirroring how the worker loads attachments via `DocumentStorage`.

## User Constraints

> No `CONTEXT.md` exists for this phase. Constraints below are extracted from `.planning/REQUIREMENTS.md` (IMG-01..09), `.planning/STATE.md`, `./CLAUDE.md`, and `./genossi-frontend/CLAUDE.md`. The planner should treat IMG-01..09 as locked decisions.

### Locked Decisions (from REQUIREMENTS.md IMG-01..09, verbatim)

- **IMG-01**: New `mail_asset` entity (SQLite BLOB storage: `id, created, deleted, version, filename, mime_type, size_bytes, bytes, uploaded_by`) with DAO/Service/REST — **NO audit log** (analog to Application-Doc pattern for non-core entities).
- **IMG-02**: `POST /api/mail/assets` accepts `multipart/form-data` with PNG/JPEG/GIF, max 5 MB/image, returns `mail_asset.id`; **admin-only** (`admin` role).
- **IMG-03**: Vorstand can insert images in the WYSIWYG editor via **Drag&Drop** OR toolbar button; editor inserts `<img data-genossi-asset-id="…" src="/api/mail/assets/{id}/bytes">`.
- **IMG-04**: `GET /api/mail/assets/{id}/bytes` returns bytes for editor preview; **admin-only** (no public access, no CID bypass).
- **IMG-05**: Harden `sanitize.rs` `<img>` rule — allow ONLY `data-genossi-asset-id`; strip `src`/other attributes, no external HTTP, no `data:` URI, no SVG.
- **IMG-06**: Renderer transforms `<img data-genossi-asset-id="X">` → `<img src="cid:asset-X@genossi">` and attaches bytes as `multipart/related` inline part with matching `Content-ID`; mail structure becomes `multipart/mixed → multipart/related → multipart/alternative`.
- **IMG-07**: Test-mail send (existing endpoint) supports images identically — Vorstand sees images in the test mail.
- **IMG-08**: Total mail size checked against 25 MB at render time; overflow gives a clear error (no later SMTP reject).
- **IMG-09**: Backward-compat — existing image-less templates (v1.4) still send WITHOUT the `multipart/related` wrapper.

### Claude's Discretion (research recommends)

- Exact placement of the CID transform (recommend: in `genossi_mail`, after HTML render / before/inside `build_message`).
- Whether the asset entity lives in main layers vs `genossi_mail` crate (recommend: **main layers**, see Architecture).
- MIME validation approach: Content-Type header vs magic-byte sniffing (recommend: **magic-byte sniffing**, IMG-05 security intent).
- Admin gate mechanism: `require_admin` route-layer vs in-service `check_permission("admin", …)` (recommend: in-service, mirrors `application_document`).

### Deferred Ideas (OUT OF SCOPE)

- Phase 28 Desktop/Mobile preview (PREV-01..05) — but note IMG-04's `/bytes` endpoint is Phase 28's dependency; keep it clean.
- Image resizing / recompression, EXIF stripping, thumbnail generation — not in IMG-01..09.
- Asset garbage-collection / orphan cleanup — not required.

## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| IMG-01 | `mail_asset` entity, SQLite BLOB, no audit | Copy `application_document` DAO/Service/REST (§Architecture Pattern 1); swap filesystem for inline `Vec<u8>` BLOB (§Don't Hand-Roll, §Pitfall 1) |
| IMG-02 | `POST /api/mail/assets` multipart, PNG/JPEG/GIF ≤5 MB, admin | Copy multipart handler `genossi_rest/src/application_document.rs:89`; admin gate `genossi_service_impl/src/attendance_export.rs:55`; magic-byte MIME sniff (§Pitfall 4) |
| IMG-03 | WYSIWYG drag&drop + toolbar image button | Extend `wysiwyg_toolbar.rs` (add button) + `wysiwyg_editor.rs` (ondrop handler); `insertHTML` execCommand (new js helper); FormData upload `api.rs:339` |
| IMG-04 | `GET /api/mail/assets/{id}/bytes`, admin | Mirror `download_application_document` `genossi_rest/src/application_document.rs:181` (bytes branch) |
| IMG-05 | Harden ammonia `<img>` rule | Replace `ammonia::clean()` with custom `Builder` (§Architecture Pattern 3, §Pitfall 2) |
| IMG-06 | CID transform + `multipart/related` | Extend `build_message` `genossi_mail/src/send.rs:52`; lettre `MultiPart::related()` + `Attachment::new_inline` (§Code Examples) |
| IMG-07 | Test-mail supports images | `send_test_mail_with_body` `genossi_mail/src/service.rs:515` shares the same `build_message` path — no separate logic |
| IMG-08 | 25 MB total-size check pre-SMTP | Sum bytes at assembly time in `build_message` or a pre-check; return error before `transport.send` (§Pitfall 5) |
| IMG-09 | Backward-compat, no related wrapper when no images | `build_message` branches on "images present?"; keep existing 4 branches byte-identical (§Pitfall 3) |

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| `mail_asset` persistence (BLOB) | Database / DAO (`genossi_dao_impl_sqlite`) | Service (`genossi_service_impl`) | Optimistic-lock + soft-delete owned by DAO; validation/permission by service — same split as `application_document` |
| Image upload (multipart parse, MIME validate, size limit) | API / REST (`genossi_rest`) | Service | Multipart extraction + body limit is an Axum REST concern; MIME/size validation belongs in service for testability |
| Admin authorization | Service (`check_permission`) | REST (`require_admin` optional) | CR-02 pins permission-first ordering in the service layer (project-wide invariant) |
| HTML sanitize (`<img>` rule) | Service/mail (`genossi_mail/src/sanitize.rs`) | — | Server-side only, single store-boundary choke-point (never WASM) |
| CID transform + MIME assembly | Mail (`genossi_mail/src/send.rs`, `render.rs`) | — | `build_message` is the single MIME factory; images are a MIME concern |
| Editor image insert + drag&drop + upload | Frontend / Browser (`genossi-frontend`) | API client (`api.rs`) | contenteditable + FormData are browser concerns; component-first per frontend CLAUDE.md |
| `/bytes` preview delivery | API / REST | Service | Byte streaming from BLOB; admin-gated |

## Standard Stack

No new dependencies. All required crates are already in the workspace (STATE.md line 48: "Keine neue Backend-Dependency").

### Core (existing, reused)
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `lettre` | 0.11.20 | MIME assembly incl. `multipart/related` + `Content-ID` inline parts | Already the single mail sender; supports `MultiPart::related()` + `Attachment::new_inline` [CITED: docs.rs/lettre/0.11.20] |
| `ammonia` | 4 | HTML sanitize; custom `Builder` for `<img>` restriction | Already the store-boundary sanitizer (`sanitize.rs`); server-side only [VERIFIED: Cargo.lock:47] |
| `sqlx` | 0.8 | SQLite BLOB round-trip (`Vec<u8>` ↔ `BLOB`) | Already binds `Vec<u8>` for `id`/`version` columns [VERIFIED: genossi_dao_impl_sqlite/src/application_document.rs:16-26] |
| `axum` | 0.8.3 | `Multipart` extractor + `DefaultBodyLimit` | Proven multipart upload at `genossi_rest/src/application_document.rs:89` [VERIFIED: codebase] |
| `uuid` | 1.6 | Entity IDs (BLOB) | Standard entity pattern |
| `web-sys` / `wasm-bindgen` | 0.3 / 0.2 | `FormData`, `File`, drag&drop events | Proven upload at `genossi-frontend/src/api.rs:351` [VERIFIED: codebase] |

### Supporting (existing)
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `html2text` | (workspace) | Derive plain-text from HTML for the alternative part | Already used in `render.rs:224 plain_from_html`; strip `<img>` before deriving plain [VERIFIED: genossi_mail/src/render.rs] |
| `serde_json` | 1.0 | Error bodies (415 UnsupportedMediaType) | Pattern at `application_document.rs:133` |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Inline BLOB in SQLite | Filesystem via `DocumentStorage` (like `application_document`) | IMG-01 explicitly says BLOB; filesystem would leave orphan files and complicate the `/bytes` path. Inline BLOB is simpler for ≤5 MB images and matches the locked requirement. |
| Custom `ammonia::Builder` | Post-process `ammonia::clean()` output with string replace | String-munging HTML is fragile (the exact anti-pattern in §Don't Hand-Roll). Builder is the sanctioned API. |
| Magic-byte MIME sniff | Trust `Content-Type` header / file extension | Header/extension is spoofable; IMG-05's security intent (no SVG, no polyglot) needs content inspection. |

**Installation:** None. `cargo build` uses existing workspace deps.

**Version verification:**
- `lettre 0.11.20` [VERIFIED: Cargo.lock] — `MultiPart::related()`, `Attachment::new_inline(String)`, `Attachment::new_inline_with_name(String, String)` all present [CITED: docs.rs/lettre/0.11.20/lettre/message/struct.MultiPart.html, struct.Attachment.html]
- `ammonia 4` [VERIFIED: Cargo.lock:47] — `Builder::new()`, `add_tags`, `rm_tag_attributes`, `add_tag_attributes`, `add_generic_attribute_prefixes`, `url_schemes`/`rm_url_schemes` present [CITED: docs.rs/ammonia/4/ammonia/struct.Builder.html]

## Package Legitimacy Audit

> No external packages are installed in this phase. All crates are pre-existing workspace dependencies. Legitimacy gate is N/A.

| Package | Registry | Age | Downloads | Source Repo | Verdict | Disposition |
|---------|----------|-----|-----------|-------------|---------|-------------|
| lettre 0.11.20 | crates.io | mature | high | github.com/lettre/lettre | OK | Already in use — no install |
| ammonia 4 | crates.io | mature | high | github.com/rust-ammonia/ammonia | OK | Already in use — no install |

**Packages removed due to [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none

## Architecture Patterns

### System Architecture Diagram

```
UPLOAD FLOW (IMG-02, IMG-03)
  Browser: WYSIWYG editor
    ├─ toolbar image button → hidden <input type=file> → web_sys::File
    └─ drag&drop → ondrop → DataTransfer → web_sys::File
         │
         ▼  FormData (api.rs)
  POST /api/mail/assets  [admin gate]
         │
         ▼
  genossi_rest: Multipart extractor → bytes + filename
         │  (DefaultBodyLimit 5 MB)
         ▼
  Service: check_permission("admin") FIRST → magic-byte MIME sniff (PNG/JPEG/GIF)
           → size ≤ 5 MB → new_v4 id → MailAssetEntity{bytes: Vec<u8>, …}
         │
         ▼  dao.create (INSERT … bytes BLOB)
  SQLite: mail_assets row (bytes stored inline)
         │
         ▼  returns { id }
  Editor inserts <img data-genossi-asset-id="{id}" src="/api/mail/assets/{id}/bytes">

PREVIEW FLOW (IMG-04)
  Editor <img src="/api/mail/assets/{id}/bytes">
         ▼  GET /api/mail/assets/{id}/bytes  [admin gate]
  Service.download(id) → SELECT bytes → Response(Content-Type: mime, body: bytes)

SEND FLOW (IMG-06, IMG-07, IMG-08, IMG-09)
  Stored body_html (sanitized, contains <img data-genossi-asset-id="X">, NO src)
         ▼  render_html_template (jinja)
  rendered HTML
         ▼  collect asset ids from data-genossi-asset-id attrs
         ├─ fetch bytes per id (main-layer service)  → check TOTAL ≤ 25 MB (IMG-08)
         └─ transform <img data-genossi-asset-id="X"> → <img src="cid:asset-X@genossi">
         ▼
  build_message:
     images present?
       NO  → existing 4-branch matrix (BYTE-IDENTICAL, IMG-09)
       YES → multipart/mixed
                └─ multipart/related
                     ├─ multipart/alternative { text/plain, text/html(cid-rewritten) }
                     └─ inline parts: Attachment::new_inline("asset-X@genossi").body(bytes, mime)
                └─ (existing document attachments, if any)
         ▼
  lettre transport.send  → Thunderbird/Outlook resolve cid: → inline image
```

File-to-implementation mapping is in the Component Responsibilities table below, not the diagram.

### Component Responsibilities

| Capability | File to create/edit | Copy-from analog |
|------------|---------------------|------------------|
| Entity + DAO trait | `genossi_dao/src/mail_asset.rs` (new) | `genossi_dao/src/application_document.rs` |
| SQLite DAO impl | `genossi_dao_impl_sqlite/src/mail_asset.rs` (new) | `genossi_dao_impl_sqlite/src/application_document.rs` |
| Service trait | `genossi_service/src/mail_asset.rs` (new) | `genossi_service/src/application_document.rs` |
| Service impl | `genossi_service_impl/src/mail_asset.rs` (new) | `genossi_service_impl/src/application_document.rs` |
| REST handlers | `genossi_rest/src/mail_asset.rs` (new) | `genossi_rest/src/application_document.rs` |
| TO type | `genossi_rest_types/src/lib.rs` (edit) | `ApplicationDocumentTO` |
| Migration | `migrations/sqlite/2026072x000000_create_mail_assets_table.sql` (new) | `20260703000000_create_application_documents_table.sql` |
| DI wiring | `genossi_bin/src/lib.rs` (edit) | `application_document_service` wiring (line 559, 845, 936) + `RestStateDef` trait (`genossi_rest/src/lib.rs:229,261`) |
| Route nest | `genossi_rest/src/lib.rs:626` region (edit) | `.nest("/api/mail/...", …)` |
| Sanitize `<img>` rule | `genossi_mail/src/sanitize.rs` (edit) | current `sanitize_html` |
| CID transform + related MIME | `genossi_mail/src/send.rs` + `render.rs` (edit) | `build_message` matrix |
| Editor image button + drop | `wysiwyg_toolbar.rs`, `wysiwyg_editor.rs` (edit) | existing toolbar buttons |
| `insertHTML` exec helper | `genossi-frontend/src/js.rs` (edit) | `exec_command_str` |
| FormData upload | `genossi-frontend/src/api.rs` (edit) | `upload_member_document` (line 339) |

### Recommended Project Structure

No new crate. New files slot into existing crates following the `application_document` layering. Route registers under `/api/mail/assets` in `genossi_rest/src/lib.rs`.

### Pattern 1: Non-Audited BLOB Entity (analog `application_document`)

**What:** A minimal DAO entity with `dump_all`/`create`/`update` + default `all`/`find_by_id`; optimistic-lock via `version`; soft-delete via `deleted`. NO `Auditable` impl.
**When to use:** IMG-01 — `mail_asset` is a non-core entity, no audit (STATE.md line 48).
**Key deviation from analog:** store `bytes: Vec<u8>` inline, not `relative_path` + filesystem.

```rust
// Source: genossi_dao/src/application_document.rs:18-29 (analog; swap fields per IMG-01)
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MailAssetEntity {
    pub id: Uuid,
    pub filename: Arc<str>,
    pub mime_type: Arc<str>,
    pub size_bytes: i64,
    pub bytes: Vec<u8>,          // NEW: inline BLOB, not relative_path
    pub uploaded_by: Arc<str>,   // NEW: user id string
    pub created: time::PrimitiveDateTime,
    pub deleted: Option<time::PrimitiveDateTime>,
    pub version: Uuid,
}
```

SQLx BLOB round-trip (the `Vec<u8>` column binds/fetches exactly like the existing `id`/`version` BLOB columns):

```rust
// Source: genossi_dao_impl_sqlite/src/application_document.rs:16-26 (FromRow mirror)
#[derive(Debug, sqlx::FromRow)]
struct MailAssetDb {
    id: Vec<u8>,
    filename: String,
    mime_type: String,
    size_bytes: i64,
    bytes: Vec<u8>,      // BLOB → Vec<u8>, native SQLx mapping
    uploaded_by: String,
    created: String,
    deleted: Option<String>,
    version: Vec<u8>,
}
// INSERT binds .bind(entity.bytes.clone())  — SQLx encodes Vec<u8> as BLOB.
```

### Pattern 2: Admin Permission Gate (CR-02 ordering)

**What:** `check_permission(ADMIN_PRIVILEGE, context)` is the FIRST statement in every service method, BEFORE any DAO/side-effect. `application_document` uses `"manage_members"`; IMG-02/04 say `admin` — use the `"admin"` string.
**When to use:** Every `mail_asset` service method (upload, download).

```rust
// Source: genossi_service_impl/src/attendance_export.rs:55 + application_document.rs:91-104
const ADMIN_PRIVILEGE: &str = "admin"; // identical string used project-wide

// FIRST line of upload()/download() — CR-02 ordering, no side effects before authz:
self.permission_service
    .check_permission(ADMIN_PRIVILEGE, context.clone())
    .await?;
let user_id = self.permission_service
    .current_user_id(context).await?
    .unwrap_or_else(|| "SYSTEM".to_string()); // → uploaded_by
```

Regression guard: `application_document.rs:822 test_upload_permission_denied_has_no_side_effects` proves `check_permission` fires with zero DAO calls. Replicate this test for `mail_asset`.

### Pattern 3: Custom ammonia Builder for `<img>` (IMG-05)

**What:** Replace `ammonia::clean()` (permissive default that allows `<img src="https://…">`) with a `Builder` that allows `<img>` with ONLY `data-genossi-asset-id`, strips `src`/`srcset`/`alt`/etc., forbids `data:` scheme, and drops SVG.

```rust
// Source: docs.rs/ammonia/4/ammonia/struct.Builder.html [CITED]
// Build ONCE (lazy static / OnceLock) — Builder construction is not free.
let cleaned = ammonia::Builder::default()
    // allow <img> but only the asset-id data attr; remove default src/alt/width/height
    .rm_tag_attributes("img", &["src", "srcset", "alt", "width", "height", "loading"])
    .add_tag_attributes("img", &["data-genossi-asset-id"])
    // never allow data: URIs anywhere
    .rm_url_schemes(&["data"])
    // SVG is not in ammonia's default tag set — verify it stays stripped
    .clean(html)
    .to_string();
```

**Verification tasks (Wave 0 / Validation):** `<img src="https://evil/x.png">` → src stripped; `<img data-genossi-asset-id="abc">` → survives; `<img src="data:image/svg+xml,…">` → stripped; `<svg>…</svg>` → stripped; existing lists/headings (Phase 26) still survive. Note ammonia treats `data-*` attributes specially — a task MUST assert `data-genossi-asset-id` survives (see Pitfall 2).

### Pattern 4: lettre multipart/related with CID (IMG-06)

**What:** Wrap the existing `multipart/alternative` in a `multipart/related`, add inline parts. [CITED: docs.rs/lettre/0.11.20]

```rust
// Source: docs.rs/lettre/0.11.20/lettre/message/{struct.MultiPart,struct.Attachment}.html
use lettre::message::{Attachment, MultiPart, SinglePart, header::ContentType};

let alternative = MultiPart::alternative()
    .singlepart(text_part)      // text/plain (cid rewritten? NO — plain has no <img>)
    .singlepart(html_part);     // text/html with src="cid:asset-X@genossi"

let mut related = MultiPart::related().multipart(alternative);
for asset in loaded_assets {
    // content_id is the BARE id (no angle brackets); lettre sets Content-ID: <asset-X@genossi>
    let inline = Attachment::new_inline(format!("asset-{}@genossi", asset.local_num))
        .body(asset.bytes.clone(), ContentType::parse(&asset.mime_type)?);
    related = related.singlepart(inline);
}
// then wrap in multipart/mixed if document attachments exist, else related is the body
```

### Anti-Patterns to Avoid

- **Threading a new `MailAssetDao` generic through `MailServiceImpl`** (`genossi_bin/src/lib.rs:629`): the mail service is generic over 6 DAOs already; adding a 7th to fetch asset bytes couples the mail crate to a new entity. Instead, keep `mail_asset` in the main layers and have the send path load bytes like the worker loads attachments (via a service call before `build_message`), passing them as a `LoadedInlineImage` slice analogous to `LoadedAttachment` (`send.rs:27`).
- **Running the CID transform BEFORE `plain_from_html`** (`render.rs:207`): `plain_from_html` (html2text) runs on the HTML to derive the text part. If you rewrite `src="cid:…"` first, the `<img>` may leak into plain text. Strip `<img>` for the plain-text derivation; do the CID rewrite only for the HTML part inside/adjacent to `build_message`.
- **Storing `src` in the DB** (IMG-05): only `data-genossi-asset-id` is persisted; `src` is injected at editor-preview render time (`/bytes` URL) and at send time (`cid:` URL). Never persist a resolvable `src`.
- **Inline RSX for the image button** (frontend CLAUDE.md): add the button inside the existing `WysiwygToolbar` component, not as page-level RSX.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| MIME multipart assembly | Manual boundary strings / header concatenation | `lettre` `MultiPart::related()` + `Attachment::new_inline` | RFC 2046/2387 boundary + Content-ID quoting is error-prone; lettre is already the sender |
| HTML sanitize / `<img>` filtering | Regex or string-replace on `<img …>` | `ammonia::Builder` | HTML parsing via regex is the classic footgun; ammonia is the existing choke-point |
| CID `<img>` rewrite | Full HTML re-serialize | Targeted attribute rewrite on the sanitized HTML (ids are known-safe post-sanitize) | Sanitized HTML guarantees only `data-genossi-asset-id` present; a bounded transform is safe |
| MIME type detection | Trust `Content-Type` / extension | Magic-byte sniff (PNG `89 50 4E 47`, JPEG `FF D8 FF`, GIF `47 49 46 38`) | IMG-05 forbids SVG/polyglots; header is spoofable |
| BLOB persistence | Base64-in-TEXT column | SQLx native `Vec<u8>` ↔ `BLOB` | SQLite BLOB is binary-native; base64 wastes 33% + bind overhead |
| Multipart upload parsing | Manual body reading | Axum `Multipart` extractor + `DefaultBodyLimit` | Proven at `application_document.rs:89` |

**Key insight:** Every hard part of this phase already has a blessed library in the workspace. The only genuinely new code is (a) an inline-BLOB entity (SQLx handles the binary), (b) a custom ammonia Builder (one function), and (c) the `related` MIME branch in `build_message` (extends an existing matrix).

## Runtime State Inventory

> This is a greenfield feature-add (new entity + new endpoints), NOT a rename/refactor/migration. Runtime-state inventory is largely N/A, but noted for completeness:

| Category | Items Found | Action Required |
|----------|-------------|------------------|
| Stored data | None — new `mail_assets` table only; existing tables untouched | New migration only |
| Live service config | None — no external service embeds mail-asset state | none |
| OS-registered state | None | none |
| Secrets/env vars | None — no new env var; reuses SMTP config | none |
| Build artifacts | Frontend WASM rebuild after `api.rs`/component edits (`dx build`) | Standard rebuild |

**Backward-compat surface (IMG-09):** existing `mail_jobs.body_html` / `mail_templates.body_html` rows that contain no `<img>` MUST still render byte-identically through `build_message`. This is a code invariant, not stored state — covered by §Pitfall 3.

## Common Pitfalls

### Pitfall 1: SQLite BLOB inline storage is new — no existing precedent
**What goes wrong:** Every existing document entity (`application_document`, `member_document`, inbound attachments) stores bytes on the **filesystem** via `relative_path`; none stores binary inline. Copying `application_document` blindly pulls in `DocumentStorage`, which IMG-01 does NOT want.
**Why it happens:** The analog is 95% right but the storage tier differs.
**How to avoid:** Drop `DocumentStorage` from the deps macro; store `bytes: Vec<u8>` in the entity; `INSERT … bytes` binds the `Vec<u8>` directly (SQLx BLOB). The `id`/`version` columns already prove `Vec<u8>` ↔ BLOB works (`application_document.rs:16`).
**Warning signs:** A `relative_path` field, a `DocumentStorage` dependency, or a `document_storage.save()` call in the `mail_asset` service = wrong pattern.

### Pitfall 2: ammonia strips `data-genossi-asset-id` unless explicitly allowed
**What goes wrong:** ammonia's default drops all `data-*` attributes. If the custom Builder allows `<img>` but forgets `add_tag_attributes("img", &["data-genossi-asset-id"])`, the asset reference is stripped on store and images silently break.
**Why it happens:** `data-*` is not generic-allowed by default; `add_tag_attributes` (not `add_generic_attribute_prefixes`) is the precise lever for a single named attr on one tag.
**How to avoid:** A round-trip test: `sanitize(r#"<img data-genossi-asset-id="abc">"#)` MUST contain `data-genossi-asset-id="abc"`. Also test that `src`/`data:`/`https:` variants are stripped. If `add_tag_attributes` proves not to whitelist `data-*` in ammonia 4, fall back to `add_generic_attribute_prefixes(&["data-genossi-asset-id"])` or `add_generic_attributes` — the Wave 0 test decides which.
**Warning signs:** Editor preview works (uses `/bytes` src) but sent mail has no image (asset-id was stripped at store boundary).

### Pitfall 3: Backward-compat byte-identity (IMG-09)
**What goes wrong:** Adding a `multipart/related` layer changes the MIME tree for ALL html mails, breaking the "no related wrapper when no images" requirement and risking regressions in the existing `build_message` tests (`send.rs:394-557`).
**Why it happens:** Naively always wrapping in `related`.
**How to avoid:** Branch on "does the rendered HTML contain `data-genossi-asset-id`?" BEFORE building. No match → the existing 4-branch matrix runs unchanged (byte-identical). Match → the new `related` branch. Keep every existing `send.rs` test green; add new tests only for the images-present path.
**Warning signs:** `build_message_legacy_singlepart_text_unchanged` (`send.rs:531`) or `build_message_alternative_text_then_html_no_attachments` (`send.rs:394`) start failing.

### Pitfall 4: Content-Type header trust vs magic-byte sniff (IMG-05 security)
**What goes wrong:** Validating MIME by the client's `Content-Type` (or filename extension) lets a Vorstand-account attacker upload an SVG (XSS vector) or polyglot renamed to `.png`.
**Why it happens:** `application_document.rs:108` intentionally *ignores* client MIME and derives from extension — but extension is still spoofable and IMG-05 has a stronger security intent (no SVG at all).
**How to avoid:** Sniff magic bytes: PNG `\x89PNG\r\n`, JPEG `\xFF\xD8\xFF`, GIF `GIF87a`/`GIF89a`. Reject anything else with 415. Store the server-derived MIME, never the client's.
**Warning signs:** SVG upload succeeds; a `.png` with `<svg>` bytes is accepted.

### Pitfall 5: 25 MB check placement (IMG-08) — before assembly, not after
**What goes wrong:** Checking size after `build_message` or letting SMTP reject means a confusing late failure.
**Why it happens:** Total size is only knowable once all asset bytes are loaded.
**How to avoid:** After collecting the per-image byte counts (and existing document-attachment sizes), sum them BEFORE calling `transport.send`; if `> 25 MB` return a clear `MailServiceError::BadRequest`/validation error. Place this in the send path (worker `send_mail_for_recipient` `worker.rs:637` and `send_test_mail_with_body` `service.rs:515`) or inside `build_message` before returning. Account for base64 inflation (~1.37×) when comparing against the 25 MB wire limit — decide whether 25 MB is raw-byte or encoded (recommend raw-byte sum with a documented margin; confirm with user — [ASSUMED] the 25 MB is raw payload, not encoded).
**Warning signs:** SMTP 552 "message too large" instead of an app-level error.

### Pitfall 6: CID uniqueness + Content-ID matching (Thunderbird/Outlook)
**What goes wrong:** If two `<img>` reference the same asset, or the `cid:` in HTML doesn't exactly match the `Content-ID` header (angle brackets, `@` domain), clients show a broken image.
**Why it happens:** lettre wraps `new_inline("asset-X@genossi")` as `Content-ID: <asset-X@genossi>`; the HTML must say `src="cid:asset-X@genossi"` (no brackets). A per-mail numbering scheme (`asset-1`, `asset-2`, …) mapped from the asset UUID keeps CIDs short and unique.
**How to avoid:** Build a `Vec<(cid_string, bytes, mime)>` while rewriting the HTML so the exact same cid string is used in both places. De-dup by asset id so one part serves N references. Test the assembled `.formatted()` output contains matching `cid:` and `Content-ID:` tokens.
**Warning signs:** "broken image" in Thunderbird/Outlook despite the part being present (success criterion #3).

## Code Examples

### Multipart upload handler (copy + adapt)
```rust
// Source: genossi_rest/src/application_document.rs:89-163 (VERIFIED codebase)
// Adapt: field "file" → bytes; then magic-byte sniff for PNG/JPEG/GIF;
// 5 MB DefaultBodyLimit; return { id } JSON.
const MAIL_ASSET_BODY_LIMIT: usize = 5 * 1024 * 1024;
// route: post(upload_mail_asset).layer(DefaultBodyLimit::max(MAIL_ASSET_BODY_LIMIT))
// handler reads `field.bytes().await?.to_vec()` for the "file" field.
```

### Bytes download (copy the bytes branch)
```rust
// Source: genossi_rest/src/application_document.rs:206-219 (VERIFIED codebase)
let (asset, bytes) = rest_state.mail_asset_service().download(id, auth, None).await?;
Response::builder()
    .status(200)
    .header("Content-Type", asset.mime_type.as_ref())
    .body(Body::from(bytes))
    .unwrap()
```

### Frontend FormData upload (copy + adapt)
```rust
// Source: genossi-frontend/src/api.rs:339-393 (VERIFIED codebase)
// pub async fn upload_mail_asset(config, file: web_sys::File) -> Result<MailAssetTO, AppError>
// form_data.append_with_blob_and_filename("file", &file, &file.name())?;
// POST {backend}/api/mail/assets ; parse { id } from JSON.
```

### Editor image insert (new js helper + drop handler)
```rust
// js.rs: add exec_command_str-style helper for "insertHTML" (only exec_command_str exists today
//   at genossi-frontend/src/js.rs:198 — insertHTML is a valid execCommand name, reuse the facade).
// After upload returns id:
//   let html = format!(r#"<img data-genossi-asset-id="{id}" src="/api/mail/assets/{id}/bytes">"#);
//   crate::js::exec_command_str(&doc, "insertHTML", &html);
//   sync_from_dom(&on_change);
// Drag&drop: add ondrop + ondragover(prevent_default) on the contenteditable div
//   (wysiwyg_editor.rs:68); read DataTransfer files → same upload path.
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `ammonia::clean()` permissive default (allows `<img src=https://…>`) | Custom `ammonia::Builder` restricting `<img>` to `data-genossi-asset-id` | This phase (IMG-05) | Tightens the store-boundary; must not regress Phase 26 list/heading survival |
| `multipart/mixed → alternative` (Phase 22/23) | `multipart/mixed → related → alternative` when images present (Phase 27) | This phase (IMG-06) | Only for image mails; no-image path unchanged (IMG-09) |
| Document bytes on filesystem (`relative_path` + `DocumentStorage`) | Inline SQLite BLOB (`bytes: Vec<u8>`) for mail assets | This phase (IMG-01) | New storage tier for this entity only |

**Deprecated/outdated:** none. `document.execCommand` is deprecated per MDN but is the project's locked editor mechanism (Phase 24 EDIT-09, STATE.md); continue using it for consistency (`insertHTML` command).

## Validation Architecture

> `config.json` sets `workflow.nyquist_validation: false`, so formal Nyquist mapping is not required. This lightweight section is included because the research focus flagged non-trivial testable seams. The project's own convention (STATE.md, CLAUDE.md) is unit + integration + e2e tests, and the global rule "Always make sure you have tests for the changes" applies.

### Test Framework
| Property | Value |
|----------|-------|
| Framework | `cargo test` (built-in) + `mockall` for service mocks + in-memory SQLite for DAO |
| Config file | none (workspace `Cargo.toml`) |
| Quick run command | `cargo test -p genossi_mail` / `cargo test -p genossi_service_impl` |
| Full suite command | `cargo test --workspace` |

### Phase Requirements → Test Map
| Req | Behavior | Test Type | Command | Exists? |
|-----|----------|-----------|---------|---------|
| IMG-01 | BLOB round-trip create/find/soft-delete | DAO integration (in-mem SQLite) | `cargo test -p genossi_dao_impl_sqlite mail_asset` | ❌ Wave 0 (mirror `application_document.rs:247`) |
| IMG-02 | admin gate + MIME sniff + 5 MB reject | service unit (mockall) | `cargo test -p genossi_service_impl mail_asset` | ❌ Wave 0 (mirror perm-denied test `application_document.rs:822`) |
| IMG-05 | ammonia allows only `data-genossi-asset-id`, strips src/data:/svg | unit | `cargo test -p genossi_mail sanitize` | ❌ Wave 0 (extend `sanitize.rs` tests) |
| IMG-06 | related structure + matching cid/Content-ID | unit on `.formatted()` | `cargo test -p genossi_mail build_message` | ❌ Wave 0 (mirror `send.rs:394` byte-offset assertions) |
| IMG-08 | 25 MB overflow → error before send | service/send unit | `cargo test -p genossi_mail` | ❌ Wave 0 |
| IMG-09 | no images → no related wrapper, existing tests green | regression | `cargo test -p genossi_mail send` | ✅ existing `send.rs:531`/`:394` (must stay green) |
| IMG-03/04 | editor insert + `/bytes` preview | manual UAT (browser) + e2e for `/bytes` | `dx serve` walkthrough; `cargo test --test e2e_*` | ❌ Wave 0 (e2e for /bytes; UAT for editor) |

### Sampling
- **Per task commit:** `cargo test -p <edited-crate>`
- **Per wave merge:** `cargo test --workspace`
- **Phase gate:** full workspace green; browser UAT for IMG-03 (drag&drop, toolbar, broken-image check in a real client per success criterion #3).

### Wave 0 Gaps
- [ ] `genossi_dao_impl_sqlite/src/mail_asset.rs` DAO round-trip test (embed migration via `include_str!`, mirror `application_document.rs:186 setup_db`)
- [ ] `genossi_service_impl/src/mail_asset.rs` mockall tests incl. CR-02 perm-denied guard
- [ ] `genossi_mail/src/sanitize.rs` `<img>` allowlist tests (survive `data-genossi-asset-id`; strip src/data:/svg; Phase 26 lists still survive)
- [ ] `genossi_mail/src/send.rs` related-structure test + backward-compat regression (existing tests must not change)
- [ ] e2e test for `GET /api/mail/assets/{id}/bytes` (admin-gated, correct Content-Type)

*Existing `build_message` tests (`send.rs`) are the backward-compat safety net — do not modify them.*

## Security Domain

> `security_enforcement` not explicitly disabled in config → enabled. This phase handles untrusted file upload + author HTML, so security is central.

### Applicable ASVS Categories
| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | yes | Global `forbid_unauthenticated` layer (`genossi_rest/src/lib.rs:721`) |
| V4 Access Control | yes | `check_permission("admin", …)` FIRST in service (IMG-02/04); CR-02 ordering |
| V5 Input Validation | yes | Magic-byte MIME sniff (PNG/JPEG/GIF only), 5 MB/image + 25 MB total, no SVG |
| V6 Cryptography | no | No new crypto |
| V12 File/Resource | yes | Reject non-image content; store server-derived MIME; BLOB avoids path traversal |
| V14 HTML Output | yes | ammonia `Builder` strips `src`/`data:`/SVG; only `data-genossi-asset-id` persisted |

### Known Threat Patterns
| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| SVG-as-image XSS | Tampering/Elevation | Magic-byte sniff rejects SVG; ammonia drops `<svg>` and `<img src="data:…">` |
| Polyglot file (PNG+HTML) | Tampering | Content-inspection MIME sniff, not extension/header trust |
| `data:` URI exfiltration in `<img>` | Info disclosure | ammonia `rm_url_schemes(&["data"])` + only `data-genossi-asset-id` allowed |
| Unauthorized asset access via `/bytes` | Info disclosure | admin-gate `/bytes` (IMG-04: no public, no CID bypass) |
| Oversized upload DoS | DoS | `DefaultBodyLimit::max(5 MB)` + service size check |
| Stored `src` pointing at attacker URL | SSRF/tracking | `src` never persisted; injected server-side only |

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | The 25 MB limit (IMG-08) is a **raw payload** sum, not base64-encoded wire size | Pitfall 5 | If it's the encoded/wire size, images ~18 MB raw could exceed the real SMTP limit; user should confirm whether 25 MB is pre- or post-encoding |
| A2 | `ammonia 4` `add_tag_attributes("img", &["data-genossi-asset-id"])` whitelists the `data-*` attribute (vs needing `add_generic_attribute_prefixes`) | Pattern 3 / Pitfall 2 | If wrong, asset-id is stripped; the Wave 0 round-trip test catches it and the fallback (`add_generic_attribute_prefixes`) applies — low risk, self-correcting |
| A3 | Storing `bytes: Vec<u8>` inline is acceptable at ≤5 MB/image scale (no perf/DB-bloat concern for a Genossenschaft's mail volume) | Pattern 1 | If DB size becomes a concern, a later phase could move to filesystem; not a Phase 27 blocker |
| A4 | The `admin` privilege string (not `manage_members`) is the correct gate per IMG-02/04 "admin-Rolle" | Pattern 2 | If the intended gate is `manage_members`, swap the constant; both patterns exist in the codebase — trivial change |
| A5 | CID transform on the HTML part + `<img>` stripped for the plain-text derivation is the correct split | Anti-Patterns / render.rs | If plain text should keep an image placeholder, adjust; low risk |

## Open Questions (RESOLVED)

> RESOLVED during planning: Q1 → 27-03 injects a main-layer byte-loader and passes `Vec<LoadedInlineImage>` to an extended `build_message` (the recommended option). Q2 → user decision **D-02**: the 25 MB check is against the **base64-encoded wire size** (overrides the [ASSUMED] raw-byte A1). Q3 → 27-03 implements the CID transform as a pure `rewrite_img_cids` function.

1. **Where exactly does the send path load asset bytes?**
   - What we know: worker (`worker.rs:637`) and test-mail (`service.rs:515`) both call `build_message`; both already load attachment bytes before the call.
   - What's unclear: whether `mail_asset` bytes are loaded via a new `MailAssetService` call injected into the worker/service, or fetched inside a new `genossi_mail` helper that takes a DAO.
   - Recommendation: inject the main-layer `MailAssetService` (or a byte-loader closure) into the send path and pass a `Vec<LoadedInlineImage>` to an extended `build_message`, mirroring `LoadedAttachment`. Avoid adding a DAO generic to `MailServiceImpl` (Anti-Pattern).

2. **Is 25 MB raw or encoded?** (see A1) — planner should surface to the user or default to raw-byte sum with a documented ~30% margin.

3. **Should the CID transform + asset-id collection be a pure, unit-testable function in `genossi_mail`?**
   - Recommendation: yes — a `fn rewrite_img_cids(html) -> (rewritten_html, Vec<AssetRef>)` pure function is trivially testable and keeps `build_message` clean.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust toolchain | all backend | ✓ | 2021 ed | — |
| SQLite (in-mem for tests) | DAO tests | ✓ | via sqlx 0.8 | — |
| Dioxus CLI (`dx`) | frontend build/UAT | assumed ✓ | 0.6.x | tests still compile without browser |
| SMTP server | IMG-07 real send | optional | — | UAT deferred to Vorstand session; unit tests assert MIME structure without sending |
| Real mail client (Thunderbird/Outlook/Nextcloud webmail) | success criterion #3 (broken-image check) | manual | — | Cannot be automated; belongs in UAT checklist |

**Missing dependencies with no fallback:** none block implementation. The "renders in real client" criterion (#3) is inherently manual UAT.

## Sources

### Primary (HIGH confidence)
- Codebase (VERIFIED via Read): `genossi_dao/src/application_document.rs`, `genossi_dao_impl_sqlite/src/application_document.rs`, `genossi_service_impl/src/application_document.rs`, `genossi_rest/src/application_document.rs`, `genossi_mail/src/send.rs`, `genossi_mail/src/sanitize.rs`, `genossi_mail/src/service.rs`, `genossi_mail/src/render.rs`, `genossi_mail/src/worker.rs`, `genossi_mail/src/rest.rs`, `genossi_mail/src/dao.rs`, `genossi-frontend/src/component/mail_compose/wysiwyg_editor.rs`, `wysiwyg_toolbar.rs`, `genossi-frontend/src/api.rs`, `genossi_rest/src/auth_middleware.rs`, `genossi_rest/src/lib.rs`, `genossi_bin/src/lib.rs`, `migrations/sqlite/*`, `.planning/REQUIREMENTS.md`, `.planning/STATE.md`
- [CITED: docs.rs/lettre/0.11.20/lettre/message/struct.MultiPart.html] — `MultiPart::related()`
- [CITED: docs.rs/lettre/0.11.20/lettre/message/struct.Attachment.html] — `Attachment::new_inline(String)`, `new_inline_with_name`, `.body(content, ContentType)`
- [CITED: docs.rs/ammonia/4/ammonia/struct.Builder.html] — `add_tags`, `rm_tag_attributes`, `add_tag_attributes`, `add_generic_attribute_prefixes`, `url_schemes`/`rm_url_schemes`

### Secondary (MEDIUM confidence)
- `Cargo.lock` (VERIFIED): lettre 0.11.20, ammonia 4 present

### Tertiary (LOW confidence)
- None. All claims are codebase-verified or docs-cited.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — no new deps; all APIs cross-checked against docs.rs + codebase usage
- Architecture: HIGH — `application_document` is a near-exact analog; deltas (BLOB, ammonia Builder, related MIME) precisely identified with file:line
- Pitfalls: HIGH — derived from concrete codebase invariants (existing `send.rs` tests, CR-02 ordering, sanitize default behavior)
- The one genuinely novel element (inline BLOB) is well-understood: SQLx `Vec<u8>` ↔ BLOB already proven for id/version columns

**Research date:** 2026-07-23
**Valid until:** ~2026-08-22 (stable stack; lettre/ammonia APIs unlikely to shift). Re-verify ammonia `data-*` attribute behavior (A2) at implementation via the Wave 0 test.
