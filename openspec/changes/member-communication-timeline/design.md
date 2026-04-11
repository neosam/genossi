## Context

The member detail page currently shows member data, actions, and documents. Communication history (sent and received mails) is only accessible through separate inbox and mail job pages with no member-scoped filtering. Users who want to see what was communicated with a specific member must manually search through these pages.

The `mail_recipients` table already has an optional `member_id` FK, and `inbound_mails` has an `assigned_member_id` FK — both pointing to the `member` table. The data relationships exist; they just aren't surfaced through a unified query or endpoint.

## Goals / Non-Goals

**Goals:**
- Single API endpoint returning a merged, chronologically sorted list of all inbound and outbound communications for a member
- Backend-side merge so frontend (and future mobile app) stays logic-light
- Deep-linkable entries so users can open mail details in new tabs
- Reusable frontend component following the component-first principle

**Non-Goals:**
- Pagination (volume per member is manageable for now)
- Inline mail body preview (just subject + metadata in the list)
- Editing or replying from the communication timeline
- Full-text search across communications
- Aggregating communications not linked to a member (unassigned inbox mails)

## Decisions

### 1. SQL UNION approach for data merge

**Decision**: Use a single SQL query with UNION ALL to merge `mail_recipients` (joined with `mail_jobs`) and `inbound_mails`, ordered by date descending.

**Alternatives considered**:
- Two separate queries merged in Rust service layer — simpler SQL but requires in-memory sort and allocation
- Database view — cleaner but adds schema maintenance overhead for a single use case

**Rationale**: UNION ALL in a single query lets SQLite handle the sort efficiently and returns results in one round-trip. The query is straightforward since both sources have compatible columns (date, subject, status, ID).

### 2. Typed direction enum with direction-specific status

**Decision**: Use a Rust enum `CommunicationDirection` (`Inbound`/`Outbound`) serialized as lowercase string. Status values are passed through as-is per direction:
- Outbound: `pending` / `sent` / `failed` (from `mail_recipients.status`)
- Inbound: three boolean flags `done` / `replied` / `archived` (from `inbound_mails`)

**Alternatives considered**:
- Unified status enum across both directions — would lose information or require awkward mappings
- String-based direction — loses type safety

**Rationale**: Original status values carry the most meaning for each direction. A unified status would either be too generic or require lossy mapping. The frontend can render direction-specific status badges.

### 3. Response structure as flat list with tagged entries

**Decision**: Return a `Vec<CommunicationEntry>` where each entry contains:
- Common fields: `direction`, `date`, `subject`
- Direction-specific fields: `inbox_id` + `from_address` + inbound status flags (for inbound), `mail_job_id` + `recipient_id` + `to_address` + outbound status (for outbound)
- Optional fields are `None` for the other direction

**Alternatives considered**:
- Enum-based response with `Inbound(...)` / `Outbound(...)` variants — cleaner in Rust but produces nested JSON with serde tagging that's harder for frontends to consume
- Separate lists per direction — defeats the purpose of backend merge

**Rationale**: A flat structure with optional fields is the simplest to consume across clients (web, mobile). The `direction` field tells the client which optional fields are populated.

### 4. Code location in `genossi_mail` crate

**Decision**: Add the DAO method, service method, and REST handler in the `genossi_mail` crate since it owns the mail tables.

**Rationale**: The query touches `mail_recipients`, `mail_jobs`, and `inbound_mails` — all owned by `genossi_mail`. The member ID is just a filter parameter; no member-specific logic is needed.

### 5. Frontend deep links via standard anchor elements

**Decision**: Each communication entry renders as a `Link` component pointing to the existing inbox or mail job detail route. Standard `<a>` behavior gives users right-click → "open in new tab" for free.

**Rationale**: No custom JavaScript needed. The routes already exist (`/inbox/{id}` for inbound, `/mail/jobs/{id}` for outbound).

## Risks / Trade-offs

- **[Unlinked mails]** Outbound mails sent before `member_id` was populated on `mail_recipients` won't appear. → No mitigation needed; this is expected behavior. The link is set at send time.
- **[Performance with many mails]** If a member has hundreds of mails, the query could be slow. → Acceptable for now; add pagination later if needed. SQLite handles this volume fine with indexed FKs.
- **[No index on member_id columns]** The `member_id` and `assigned_member_id` columns may not be indexed. → Check during implementation and add indexes if missing.
