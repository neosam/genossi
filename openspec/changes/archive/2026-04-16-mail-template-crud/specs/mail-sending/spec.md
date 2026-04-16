## ADDED Requirements

### Requirement: Load template for mail composition
The system SHALL allow clients to retrieve a saved mail template by ID via `GET /api/mail/templates/{id}` and use its subject and body fields to pre-fill the mail sending form. The mail sending endpoints (`/api/mail/send`, `/api/mail/send-bulk`) remain unchanged — they continue to accept subject and body directly.

#### Scenario: Client loads template before sending
- **WHEN** a client fetches `GET /api/mail/templates/{id}` and then calls `POST /api/mail/send-bulk` with the returned subject and body
- **THEN** the system sends the mail using the template content, exactly as if the subject and body were typed manually
