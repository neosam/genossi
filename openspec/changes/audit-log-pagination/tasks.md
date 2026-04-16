## 1. Database

- [x] 1.1 Create migration `migrations/sqlite/<ts>_audit_log_pagination_index.sql` adding composite index `idx_audit_log_entity_type_timestamp ON audit_log(entity_type, timestamp)`
- [x] 1.2 Verify on a local DB that existing migrations + the new one apply cleanly

## 2. DAO Layer

- [x] 2.1 Add `AuditQueryFilter` struct to `genossi_dao::audit_log` (fields: entity_type, entity_id, user_id, action, from, to — all `Option`)
- [x] 2.2 Add `query(filter, limit, offset, tx)` method to `AuditLogDao` trait
- [x] 2.3 Add `count(filter, tx)` method to `AuditLogDao` trait
- [x] 2.4 Update `MockAuditLogDao` (mockall expectations) to cover the new methods
- [x] 2.5 Implement `query` in `genossi_dao_impl_sqlite::audit_log` with dynamic `WHERE` clause, `ORDER BY timestamp DESC, id DESC`, and `LIMIT ? OFFSET ?`
- [x] 2.6 Implement `count` in `genossi_dao_impl_sqlite::audit_log` with the same dynamic `WHERE` clause and `SELECT COUNT(*)`
- [x] 2.7 Add unit tests in `genossi_dao_impl_sqlite` covering: no filter, each filter dimension, combined filters, pagination edges (page 0, last page, page beyond total), empty result, stable ordering with duplicate timestamps
- [x] 2.8 ~~Run `DATABASE_URL=sqlite:genossi.db cargo sqlx prepare`~~ — not needed; audit_log impl uses runtime `sqlx::query`/`query_as`, not the compile-time-checked `query!` macro

## 3. Service Layer

- [x] 3.1 Confirmed no service-level changes are needed — the audit REST handler talks to `AuditLogDao` directly via the rest state, no service wrapper to update
- [x] 3.2 No `genossi_service_impl` adjustments needed

## 4. REST Types

- [x] 4.1 Add `PagedAuditLogTO { entries: Vec<AuditLogEntryTO>, total: i64, page: i64, size: i64 }` to `genossi_rest_types`
- [x] 4.2 Add `ToSchema` derives so it shows up in OpenAPI/Swagger

## 5. REST Layer

- [x] 5.1 Extend `AuditQueryParams` in `genossi_rest::audit_log` with `page` and `size`
- [x] 5.2 Implement size clamping (allowed set: 25/50/100/200/500, default 50) and page clamping (>= 0) in the handler
- [x] 5.3 Build `AuditQueryFilter` from the validated query params
- [x] 5.4 Replace `get_all_ordered` + `Vec::retain` + `reverse()` with `count` + `query` calls; build the `PagedAuditLogTO` envelope
- [x] 5.5 Update `#[utoipa::path]` attributes (params, response body type, schema registration in `ApiDoc`)
- [x] 5.6 Confirmed `verify_chain` and `get_audit_by_entity` handlers are unchanged
- [x] 5.7 Updated existing e2e tests + added 4 new pagination tests (default, explicit page/size, size clamping, page beyond total, filter + total)

## 6. Backend Wiring

- [x] 6.1 Confirmed no changes needed in `genossi_bin/src/lib.rs` — verified by passing e2e tests

## 7. Frontend Types & API

- [x] 7.1 Add `PagedAuditLogTO` to `rest-types` mirror that the frontend consumes
- [x] 7.2 Update `genossi-frontend/src/api.rs::get_audit_log` signature to accept page/size and return `PagedAuditLogTO`
- [x] 7.3 Updated only call site (`page/audit_log.rs`); `cargo check` confirms no other callers

## 8. Frontend Reusable Components

- [x] 8.1 Create `genossi-frontend/src/component/page_size_select.rs` with props `current_size`, `on_size_change`; allowed values `[25, 50, 100, 200, 500]`
- [x] 8.2 Create `genossi-frontend/src/component/pagination_controls.rs` with props `current_page`, `total_pages`, `on_page_change`; render First / Prev / numbered pages (with ellipsis for long ranges) / Next / Last
- [x] 8.3 Add both components to the component module's `mod.rs` exports
- [x] 8.4 Added 6 unit tests on `page_strip` (the page-number layout function); all green

## 9. Frontend Audit Log Page

- [x] 9.1 Add `current_page` and `page_size` signals to `genossi-frontend/src/page/audit_log.rs`; initialize to 0 and 50
- [x] 9.2 Add `total` signal to track total entries from the response
- [x] 9.3 Update `load_entries` closure to read page/size, send them in the request, and write `entries` + `total` from the envelope
- [x] 9.4 Wire the new `PageSizeSelect` and `PaginationControls` components into the page layout (above and below the table)
- [x] 9.5 Make `on_filter` reset `current_page` to 0 before triggering load
- [x] 9.6 Make page-size change reset `current_page` to 0 before triggering load
- [x] 9.7 Replace transaction-grouping zebra-stripes with deterministic color from `transaction_id` (`transaction_id.as_bytes()[0] & 1`)
- [x] 9.8 Display "Seite X / Y · Z Einträge" near the controls

## 10. Internationalization

- [x] 10.1 Add i18n keys for pagination labels (`PaginationFirst`, `PaginationPrev`, `PaginationNext`, `PaginationLast`, `PageSize`, `PageOfTotal`, `TotalEntries`) to `genossi-frontend/src/i18n/mod.rs`
- [x] 10.2 Translate keys in `en.rs` and `de.rs` (the active locales — `cs.rs` is from a legacy unused module not wired into `mod.rs`'s `Locale` enum)

## 11. Validation

- [ ] 11.1 `cargo fmt` — **deferred to user**, the `cargo fmt`/`rustfmt` binaries are not present in this Nix-provisioned toolchain
- [ ] 11.2 `cargo clippy --all-targets --all-features` clean — **deferred to user**, `cargo-clippy` not installed in this toolchain
- [x] 11.3 `cargo test --workspace --exclude genossi-frontend` all green (570+ tests pass)
- [x] 11.4 `cargo test -p genossi_bin --test e2e_tests` all green (197 tests, including 4 new pagination tests + 1 updated test)
- [ ] 11.5 Manual smoke test: start backend + `dx serve`, log in as admin, navigate to `/audit`, verify pagination, page-size change, all filters reset to page 0, total count visible, transaction zebra-striping stable across page boundaries
- [ ] 11.6 Manual check: verify `GET /api/audit/verify` still returns the correct `total_entries` matching the unfiltered count
- [ ] 11.7 Manual check: confirm WebDAV backup audit export still produces a complete export (no behavior change expected)
