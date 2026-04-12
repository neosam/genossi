## Context

Genossi has no data export functionality. Users can view member data in the UI and import from Excel, but cannot create backups. The system stores three types of data relevant for backup:

1. **Member records** with shares that evolve over time through actions
2. **Member actions** (Eintritt, Austritt, Aufstockung, Verkauf, etc.) that track the history
3. **Member documents** (PDFs, images) stored on the filesystem with metadata in SQLite

The `current_shares` field on members reflects the current state. Historical share values must be reconstructed from actions. `current_balance` is excluded from export because it requires Verlustvortrag calculation which is not yet implemented.

## Goals / Non-Goals

**Goals:**
- Enable authorized users to download a complete backup of member data
- Provide historically accurate member list at any given cutoff date (Stichtag)
- Export all member actions with readable member names
- Bundle all member documents into a single downloadable archive
- Protect backup endpoints with a dedicated privilege

**Non-Goals:**
- Full database dump (only member-related data)
- Incremental/differential backups
- Scheduled automatic backups
- Balance (Guthaben) reconstruction — requires Verlustvortrag which is not implemented
- Selective export (e.g., only specific members) — all-or-nothing for now
- Import from exported CSV (this is a one-way export)

## Decisions

### 1. CSV generation: Manual formatting vs. `csv` crate

**Decision:** Use the `csv` crate for CSV generation.

**Rationale:** Manual CSV formatting is error-prone with quoting, escaping commas in names/addresses, and Unicode handling. The `csv` crate handles all edge cases correctly and is the de-facto standard in the Rust ecosystem.

### 2. ZIP streaming: `zip` crate with streaming response

**Decision:** Use the `zip` crate to write ZIP entries into an Axum streaming response body.

**Alternatives considered:**
- `async-zip`: More async-native but less mature
- Tar/gzip: Less familiar to end users on Windows

**Rationale:** The `zip` crate is mature and well-tested. We stream the ZIP directly into the HTTP response using Axum's streaming body support, avoiding temp files on disk. Documents are read from the filesystem via the existing `DocumentStorage` trait and written entry-by-entry into the ZIP stream.

### 3. Share calculation at Stichtag: Query-time aggregation

**Decision:** Calculate shares at Stichtag using a SQL query that sums `shares_change` from all non-deleted actions with `date <= stichtag`, added to `shares_at_joining`.

**Alternatives considered:**
- Application-level calculation: Load all members and actions, compute in Rust
- Materialized snapshots: Pre-compute and store historical snapshots

**Rationale:** SQL aggregation is efficient and doesn't require loading all actions into memory. The query joins members with their actions and groups by member, filtering actions by date. This keeps the logic in a single query and scales well.

### 4. Member filtering at Stichtag

**Decision:** A member appears in the Stichtag export if:
- `join_date <= stichtag`
- `exit_date IS NULL` OR `exit_date > stichtag`
- `deleted IS NULL`
- `status != 'FehlerhaftErfasst'`

**Rationale:** This reflects who was an active, valid member at the given date. Soft-deleted and erroneously recorded members are excluded as they are not real members.

### 5. Document ZIP structure

**Decision:** Organize ZIP entries by member number and name:
```
<member_number>_<last_name>_<first_name>/
  <document_type>_<file_name>
```

**Rationale:** Member number prefix ensures unique directories and natural sort order. Including the name makes the archive human-browsable without needing to cross-reference IDs.

### 6. Privilege model

**Decision:** Single new privilege `export_backup` that gates all three export endpoints. Added via migration and assigned to the `admin` role.

**Rationale:** Backup export is an all-or-nothing administrative function. Splitting into three privileges would add complexity without practical benefit — anyone who can export the member list should also be able to export actions and documents.

### 7. Frontend: Dedicated page vs. dropdown on members page

**Decision:** Dedicated `/backup` page accessible via TopBar navigation.

**Rationale:** Backup is a deliberate administrative action, not a quick data view operation. A dedicated page provides space for the Stichtag date picker, download buttons with status feedback, and a warning about large document downloads. It keeps the already-complex members page unchanged.

## Risks / Trade-offs

**[Large ZIP downloads may time out on slow connections]** → Streaming avoids server-side memory pressure, but the client must maintain the connection. A progress indicator in the browser's native download UI is sufficient for now. A background-job approach can be added later if needed.

**[Share calculation assumes complete action history]** → If actions were manually deleted or the database was modified outside the app, the calculated shares at Stichtag may not match historical reality. This is acceptable — the system already validates action consistency via migration status.

**[No resumable downloads]** → If a document ZIP download fails mid-stream, the user must restart. Acceptable for the initial implementation given the streaming approach avoids server-side temp files.

**[CSV encoding]** → CSV will use UTF-8 with BOM for Excel compatibility on Windows. Some older tools may not handle this correctly, but UTF-8 BOM is the modern standard for CSV.
