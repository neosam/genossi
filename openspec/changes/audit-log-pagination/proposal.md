## Why

The audit log already contains 600+ entries (heavily inflated by snapshot entries) and grows append-only forever. The current implementation loads all entries from the database, filters them in memory, serializes the entire set as JSON, and renders one row per changed field in a single `<table>`. This makes the audit log page slow now and untenable in the medium term — payload size, WASM heap usage, and DOM size all scale linearly with total audit history.

## What Changes

- **BREAKING**: `GET /api/audit` response changes from a JSON array `Vec<AuditLogEntryTO>` to an envelope object `{ entries, total, page, size }` to support pagination.
- `GET /api/audit` accepts new query parameters `page` (0-based) and `size` (limited to 25/50/100/200/500). Existing filter parameters (`entity_type`, `entity_id`, `user_id`, `action`, `from`, `to`) are now applied at the database layer, not in memory.
- `AuditLogDao` gains two methods: `query(filter, limit, offset, tx)` returning the page slice, and `count(filter, tx)` returning the total matching that filter.
- New SQLite indexes on `audit_log` to keep paginated, filtered queries cheap as the table grows.
- Frontend audit log page gains a page-size selector (25/50/100/200/500), classic page-number navigation (First / Prev / pages / Next / Last) and a total-count display. Filter changes reset to page 0.
- Frontend zebra-striping per transaction is derived from the `transaction_id` (hash mod 2), so visual grouping stays consistent when a transaction spans a page boundary.
- `GET /api/audit/verify` and `GET /api/audit/{entity_type}/{entity_id}` are **unchanged** — they continue to operate over the full set / full entity history.
- The backup export pipeline is **unchanged** — it continues to export all audit entries.

## Capabilities

### New Capabilities
<!-- None — this change modifies existing capabilities. -->

### Modified Capabilities
- `audit-api`: the "Get all audit log entries with filtering" requirement changes its response shape (envelope with pagination metadata) and gains pagination query parameters; filtering becomes server-side / database-backed.
- `audit-ui`: the "Audit log page" requirement gains page-navigation controls, a page-size selector, a total-count display, and transaction-id-derived zebra-striping that is stable across page boundaries.

## Impact

- **Backend**
  - `genossi_dao::audit_log::AuditLogDao` trait — new `query` + `count` methods (existing `get_all_ordered`, `get_by_entity`, `get_latest_hash`, `create_entries` retained)
  - `genossi_dao_impl_sqlite::audit_log` — implement new methods with SQL `WHERE` + `LIMIT`/`OFFSET` + `COUNT(*)`
  - New migration in `migrations/sqlite/` adding indexes on `audit_log` (timestamp DESC, plus combined indexes for common filter dimensions)
  - `genossi_rest::audit_log::get_audit_log` — pagination params, envelope response, drop in-memory filtering
  - `genossi_rest_types` — new `PagedAuditLogTO` (or similar) envelope type, mirrored in OpenAPI / Swagger
- **Frontend**
  - `genossi-frontend/src/api.rs` — `get_audit_log` signature now returns the envelope; pass page/size
  - `genossi-frontend/src/page/audit_log.rs` — wire pagination state, reset on filter change
  - New reusable components under `genossi-frontend/src/component/`: `PaginationControls`, `PageSizeSelect` (per CLAUDE.md component-first principle)
  - i18n keys for pagination labels in `en.rs`, `de.rs`, `cs.rs`
- **Out of scope / explicitly unchanged**
  - Hash chain, audit storage model, `audit-logging` capability
  - `GET /api/audit/verify`
  - `GET /api/audit/{entity_type}/{entity_id}`
  - Backup / WebDAV audit export pipeline
