## Purpose

Inbox reply capability -- allows admins to reply to inbound mail directly from the inbox, with proper email threading headers.

## Requirements

### Requirement: Reply to inbound mail
The system SHALL allow admins to reply to an inbound mail via `POST /api/inbox/{id}/reply` with a JSON body containing `subject` and `body` fields. The endpoint SHALL create a `mail_job` with `reply_to_inbound_mail_id` set to the inbound mail's ID, create a single `mail_recipient` with the inbound mail's `from_address`, and set the inbound mail's status to `replied`.

#### Scenario: Successful reply
- **WHEN** `POST /api/inbox/{id}/reply` is called with `{"subject": "Re: Frage", "body": "Hier die Antwort."}`
- **THEN** the system creates a mail job with `reply_to_inbound_mail_id` set, creates one mail recipient with `to_address` equal to the inbound mail's `from_address`, sets the inbound mail status to `replied`, and returns the created mail job

#### Scenario: Reply to non-existent inbound mail
- **WHEN** `POST /api/inbox/{id}/reply` is called with an ID that does not exist
- **THEN** the system returns 404

#### Scenario: Reply to already-replied mail
- **WHEN** `POST /api/inbox/{id}/reply` is called for a mail with status `replied`
- **THEN** the system creates a new reply job (multiple replies to the same mail are allowed)

### Requirement: Inbound mail replied status
The system SHALL support a `replied` status on inbound mails. The status `replied` SHALL be reachable from `new` or `assigned`. After `replied`, the mail MAY still be archived or ignored.

#### Scenario: Status displayed in list
- **WHEN** the inbox list is fetched and a mail has status `replied`
- **THEN** the API returns `"status": "replied"` and the frontend displays the replied status with a distinct visual indicator

#### Scenario: Archive after reply
- **WHEN** an inbound mail has status `replied` and the admin archives it
- **THEN** the system sets the status to `archived`

### Requirement: Inbound mail stores own Message-ID
The system SHALL parse and store the `Message-ID` header from incoming mails in a `message_id` field on `inbound_mails`. The value SHALL be normalized (angle brackets stripped).

#### Scenario: Mail with Message-ID header
- **WHEN** an inbound mail is received with header `Message-ID: <abc.123@example.com>`
- **THEN** the system stores `message_id = "abc.123@example.com"` on the inbound mail record

#### Scenario: Mail without Message-ID header
- **WHEN** an inbound mail is received without a Message-ID header
- **THEN** the system stores `message_id = NULL`

### Requirement: Outbound reply sets In-Reply-To header
The system SHALL set `In-Reply-To` and `References` headers on outbound mails when the mail job has a `reply_to_inbound_mail_id` and the referenced inbound mail has a non-NULL `message_id`.

#### Scenario: Reply with known Message-ID
- **WHEN** the mail worker sends a mail for a job with `reply_to_inbound_mail_id` set, and the inbound mail has `message_id = "abc.123@example.com"`
- **THEN** the outbound mail includes headers `In-Reply-To: <abc.123@example.com>` and `References: <abc.123@example.com>`

#### Scenario: Reply without known Message-ID
- **WHEN** the mail worker sends a mail for a job with `reply_to_inbound_mail_id` set, but the inbound mail has `message_id = NULL`
- **THEN** the outbound mail is sent without `In-Reply-To` and `References` headers

### Requirement: mail_jobs reply_to_inbound_mail_id field
The system SHALL add a nullable `reply_to_inbound_mail_id` (BLOB) column to the `mail_jobs` table. When set, it indicates that this job is a reply to the referenced inbound mail.

#### Scenario: Regular mail job
- **WHEN** a bulk mail job is created via the existing mail sending flow
- **THEN** `reply_to_inbound_mail_id` is NULL

#### Scenario: Reply mail job
- **WHEN** a reply is created via `POST /api/inbox/{id}/reply`
- **THEN** `reply_to_inbound_mail_id` is set to the inbound mail's UUID
