## Why

When viewing a member's details, there is no way to see what communication has taken place with that member. Users must manually search through the inbox and mail jobs to piece together the history. A unified communication timeline on the member detail page gives users an instant overview of all interactions — both sent and received — for any member.

## What Changes

- New backend endpoint `GET /api/members/{id}/communications` that merges outbound mail recipients and inbound mails into a single chronologically sorted list
- New DAO method to query both `mail_recipients` (joined with `mail_jobs`) and `inbound_mails` by member ID using SQL UNION
- New service method in `genossi_mail` to orchestrate the query
- New "Kommunikation" section on the member detail page showing the timeline
- Each entry deep-links to the corresponding inbox detail or mail job detail page (standard `<a>` links for new-tab support)
- Direction enum (`inbound`/`outbound`) with original status values preserved per direction

## Capabilities

### New Capabilities
- `member-communication-timeline`: Unified chronological view of all inbound and outbound mail communication for a specific member, served via a single API endpoint and displayed on the member detail page.

### Modified Capabilities
<!-- No existing spec-level requirements change. The mail-sending and member-management capabilities remain as-is; this adds a read-only aggregation view. -->

## Impact

- **Backend**: New endpoint in `genossi_mail` crate (DAO, service, REST layers). New types for the communication entry and direction enum.
- **Frontend**: New section in `member_details.rs`, new API function, new reusable component for the communication list.
- **Database**: Read-only queries against existing `mail_recipients`, `mail_jobs`, and `inbound_mails` tables. No schema changes needed.
- **REST types**: New shared types in `genossi_rest_types` or `genossi_mail` REST types for the communication response.
