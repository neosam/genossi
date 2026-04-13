## 1. Backend Types

- [x] 1.1 Add `CommunicationDirection` enum (`Inbound`/`Outbound`) with serde serialization as lowercase string in `genossi_mail`
- [x] 1.2 Add `CommunicationEntry` struct with common fields (`direction`, `date`, `subject`) and direction-specific optional fields (`inbox_id`, `from_address`, `done`, `replied`, `archived` for inbound; `mail_job_id`, `recipient_id`, `to_address`, `status` for outbound)

## 2. Backend DAO

- [x] 2.1 Add `get_member_communications(member_id: Uuid) -> Vec<CommunicationEntry>` method to an appropriate DAO trait in `genossi_mail`
- [x] 2.2 Implement the SQLite version using SQL UNION ALL query joining `mail_recipients` + `mail_jobs` (outbound) and `inbound_mails` (inbound), filtered by member ID, excluding soft-deleted entries, ordered by date descending
- [x] 2.3 Check if indexes exist on `mail_recipients.member_id` and `inbound_mails.assigned_member_id`; add migration if missing

## 3. Backend Service

- [x] 3.1 Add `get_member_communications(member_id: Uuid)` method to the mail service that delegates to the DAO
- [x] 3.2 Add unit tests for the service method using mocks

## 4. Backend REST

- [x] 4.1 Add `GET /api/members/{member_id}/communications` handler in `genossi_mail` REST layer returning `Vec<CommunicationEntry>` as JSON
- [x] 4.2 Add OpenAPI documentation for the new endpoint via utoipa
- [x] 4.3 Add integration/e2e test: create a member, send mail to them, create an assigned inbound mail, then verify the communications endpoint returns both entries sorted correctly

## 5. Frontend Types & API

- [x] 5.1 Add `CommunicationDirection` enum and `CommunicationEntry` struct to frontend REST types
- [x] 5.2 Add `get_member_communications(config, member_id)` API function in `api.rs`

## 6. Frontend Component & Page

- [x] 6.1 Create reusable `CommunicationTimeline` component in `src/component/` that renders a list of `CommunicationEntry` items with direction indicator, date, subject, status, and deep link
- [x] 6.2 Add i18n keys for "Kommunikation", direction labels, and status labels
- [x] 6.3 Integrate `CommunicationTimeline` into `member_details.rs` as a new section (visible only for existing members), loading data on mount
