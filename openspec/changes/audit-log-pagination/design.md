## Context

The audit log is a hash-chained, append-only ledger that grows whenever an audited entity (Member, MemberAction, MemberDocument, Application) changes. Each changed *field* produces one row, so a single member update with five changed fields creates five rows. Snapshots multiply this further. The current installation already holds 600+ rows after limited use; in normal operation this grows by hundreds to thousands per month.

The current `GET /api/audit` handler at `genossi_rest/src/audit_log.rs` calls `AuditLogDao::get_all_ordered`, then runs `Vec::retain` for each filter, then `entries.reverse()`, then serializes the entire vector. The frontend at `genossi-frontend/src/page/audit_log.rs` consumes the array, groups by `transaction_id` for visual zebra-striping, and renders one `<tr>` per entry.

Stakeholders are admins (the only role allowed to read the audit log). The page is currently slow with 600 rows and will become unusable.

## Goals / Non-Goals

**Goals:**
- Server-side pagination with database-level filtering on `GET /api/audit`.
- Frontend page navigation with configurable page size (25/50/100/200/500), classic page-number controls, and a stable visual transaction grouping that survives page boundaries.
- Keep the integrity story intact: hash chain verification continues to scan the full table.
- Keep the export pipeline intact: WebDAV/backup audit export continues to read all rows.
- Reusable pagination components, per the project's component-first principle.

**Non-Goals:**
- Cursor-based pagination, infinite scroll, or virtualized table rendering — explicitly chose classic pagination for predictability and deep-linkability.
- Changing the audit data model, the hash chain, or the `Auditable` trait.
- Changing `GET /api/audit/{entity_type}/{entity_id}` (per-entity history is bounded and not the bottleneck).
- Changing how the backup worker exports audit log entries.
- Aggregating rows per transaction in the API or the table layout — the table stays one-row-per-field; we only change how grouping is *colored*.

## Decisions

### Decision 1: Server-side pagination with offset (not cursor)

Use SQL `LIMIT ? OFFSET ?` with a separate `COUNT(*)` query for the total. Response envelope:

```json
{
  "entries": [ ... ],
  "total":   1234,
  "page":    0,
  "size":    50
}
```

**Why offset over cursor:**
- The UI requirement is *classic paging with page numbers* and a "jump to page N" feel, not infinite scroll. Offset matches that semantics directly.
- Offset lets us show "Page 7 of 23" trivially; cursor would require additional roundtrips.
- The known instability of offset (a row inserted between page loads can shift the offset) is acceptable for a write-once-then-read-many audit log: new entries always appear at page 0 (timestamp DESC). Browsing older pages does not race with new writes in any harmful way — at worst, the user sees the same row twice across a page boundary if a new entry is inserted, which is benign for read-only auditing.

**Why not cursor:**
- Page-number UI doesn't compose with cursors. We'd need a hybrid (cursor for sequential, offset for jumps), adding complexity for marginal benefit.

### Decision 2: Filters move from `Vec::retain` to SQL `WHERE`

The existing handler filters in memory after loading everything. With pagination this is incoherent — a filtered "page 1 of 10" cannot be derived from a slice of unfiltered data without either over-fetching or showing wrong totals. So filters must be applied in the database.

We introduce an `AuditQueryFilter` struct passed to both the new `query` and `count` DAO methods so the same filter shape drives both the page slice and the total count.

```rust
pub struct AuditQueryFilter {
    pub entity_type: Option<String>,
    pub entity_id:   Option<Uuid>,
    pub user_id:     Option<String>,
    pub action:      Option<String>,
    pub from:        Option<String>,  // ISO8601
    pub to:          Option<String>,
}
```

The DAO impl builds a parameterized SQL `WHERE` clause dynamically. We use string concatenation of `AND <col> = ?` fragments rather than a full query builder; this is small and self-contained.

### Decision 3: Trait additions, not replacements

`AuditLogDao` keeps `get_all_ordered`, `get_by_entity`, `get_latest_hash`, `create_entries` as-is. We *add* `query` and `count`. This is because:

- `get_all_ordered` is still needed by `verify_chain` and by the backup export — both legitimately need the full table.
- `get_by_entity` powers `GET /api/audit/{entity_type}/{entity_id}` which is unchanged.
- A clean, additive change keeps the diff narrow and avoids accidentally regressing the verify path.

```rust
async fn query(
    &self,
    filter: AuditQueryFilter,
    limit:  i64,
    offset: i64,
    tx:     Self::Transaction,
) -> Result<Arc<[AuditLogEntry]>, DaoError>;

async fn count(
    &self,
    filter: AuditQueryFilter,
    tx:     Self::Transaction,
) -> Result<i64, DaoError>;
```

`query` returns rows ordered by `timestamp DESC`, `id DESC` as tiebreaker so the order is stable within a single timestamp.

### Decision 4: Allowed page sizes are clamped server-side

Client sends `size`, server clamps to one of `{25, 50, 100, 200, 500}`; values outside that set fall back to the default (50). This prevents a malicious or buggy client from requesting `size=1_000_000` and effectively defeating pagination.

`page` is clamped to `>= 0`. If `page * size >= total`, the response returns an empty `entries` array with the correct `total`, so the UI can recover to the last valid page.

### Decision 5: Indexing — start with what we have, evaluate, add only if needed

The existing `audit_log` table already has:
- `idx_audit_log_entity (entity_type, entity_id)`
- `idx_audit_log_transaction (transaction_id)`
- `idx_audit_log_timestamp (timestamp)`
- `idx_audit_log_user (user_id)`

For unfiltered pagination ordered by timestamp DESC, the timestamp index is sufficient (SQLite scans it in reverse). For filtered + ordered queries (e.g. `WHERE entity_type = ? ORDER BY timestamp DESC`), SQLite's optimizer will pick the more selective index but cannot use both. At 600 rows this is invisible; at 100k+ rows a composite `(entity_type, timestamp)` would help.

**Plan:** add the migration in this change but conservatively — one composite index `(entity_type, timestamp)` because filtering by entity_type is the most common use case. Other composites can be added later when query plans actually warrant it. We do not drop existing indexes.

### Decision 6: Zebra-striping derived from `transaction_id`

Currently the frontend zebra-stripes rows by alternating between transaction *groups* using the loop index. With pagination, a transaction split across a page boundary would render half on one color and half on another.

Switch to: color is determined by `transaction_id` itself, e.g. `if (hash(transaction_id) % 2) == 0 { "" } else { "bg-gray-50" }`. UUID v4 has enough entropy in any byte that taking `id.as_bytes()[0] & 1` is fine. This is deterministic per transaction and identical regardless of which page renders the row.

Alternative considered: aggregate the page payload server-side so transactions never span pages (return up to N transactions, not N rows). Rejected — variable row counts complicate the UI and the page-size selector loses meaning.

### Decision 7: Reusable pagination components

Per the project's component-first principle (`feedback_component_first`), extract:

- `genossi-frontend/src/component/pagination_controls.rs` — `PaginationControls { current_page, total_pages, on_page_change }`.
- `genossi-frontend/src/component/page_size_select.rs` — `PageSizeSelect { current_size, on_size_change }` with the fixed list `[25, 50, 100, 200, 500]`.

These will be reused later for any list view that needs paging (member list, mail queue, etc.) and prevent the audit log from getting a bespoke control.

### Decision 8: Filter changes reset to page 0

Whenever any filter input changes (and the user hits Search) or the page size changes, current page resets to 0. This prevents the "I was on page 7 of unfiltered, then filtered to 3 results, and now I see an empty page" footgun.

## Risks / Trade-offs

- **[Risk]** Breaking API change to `GET /api/audit` response shape → **Mitigation:** the only consumer is the in-tree frontend; we update both in the same change. No external API consumers documented.
- **[Risk]** OFFSET pagination drifts when new rows arrive between page loads → **Mitigation:** Acceptable for read-only audit history. New rows always appear at page 0; browsing older pages may show one row's worth of duplication at a page boundary, which is harmless. Documented in user-facing docs only if users complain.
- **[Risk]** Forgetting to migrate `verify_chain` and `get_audit_by_entity` could regress them inadvertently → **Mitigation:** Tests for both endpoints exist or will be added; new DAO methods are additive so the old methods stay untouched.
- **[Risk]** `COUNT(*)` over a large filtered set adds latency → **Mitigation:** the indexes cover the filter columns; at our growth rate this stays cheap. If counts ever become slow, we can switch to an estimate or cache.
- **[Risk]** Frontend pagination state and filter state interaction bugs → **Mitigation:** invariant "filter change ⇒ page = 0" enforced in one place (the load function).
- **[Trade-off]** Classic page numbers vs. infinite scroll → chose classic for predictability and deep-link friendliness; loses the "fluid feel" of infinite scroll.
- **[Trade-off]** Keeping zebra striping but stable across pages vs. dropping zebra in favor of explicit transaction headers → kept the lighter visual approach to avoid scope creep.

## Migration Plan

1. Add SQL migration creating the composite index on `(entity_type, timestamp)`. Idempotent / safe on existing data.
2. Add new DAO methods. Existing methods unchanged.
3. Add new envelope type to `genossi_rest_types`.
4. Update REST handler: switch to envelope response, take `page` and `size`, push filters into DAO call.
5. Update OpenAPI annotations / Swagger schema.
6. Update frontend `api::get_audit_log` signature.
7. Add new components and i18n keys.
8. Update audit log page to use envelope + components.
9. Run `cargo sqlx prepare` to regenerate offline query data.

**Rollback:** revert; the new index is harmless if left in place. No data migration involved.

## Open Questions

- None blocking. Whether to add additional composite indexes (e.g. `(user_id, timestamp)`) can be deferred until query plans warrant it.
