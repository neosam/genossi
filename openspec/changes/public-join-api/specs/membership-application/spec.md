## ADDED Requirements

### Requirement: Application data model
The system SHALL store membership applications with the following fields:
- `id` (UUID, system-generated, primary key)
- `first_name` (String, required)
- `last_name` (String, required)
- `salutation` (Optional Enum: Herr, Frau, Firma)
- `email` (String, required)
- `street` (String, required)
- `house_number` (String, required)
- `postal_code` (String, required)
- `city` (String, required)
- `shares` (i32, required, minimum 1)
- `status` (ApplicationStatus enum: Offen, Bestätigt, Abgelehnt, default Offen)
- `created` (DateTime, system-generated)
- `deleted` (Optional DateTime, for soft delete)
- `version` (UUID, for optimistic locking)

#### Scenario: Application stored with all fields
- **WHEN** an application is submitted with first_name "Max", last_name "Mustermann", email "max@example.com", street "Musterstr.", house_number "1", postal_code "12345", city "Berlin", shares 1
- **THEN** the system stores the application with a generated UUID, status "Offen", current timestamp as created, and a generated version UUID

#### Scenario: Application stored with optional salutation
- **WHEN** an application is submitted with salutation "Herr" and all required fields
- **THEN** the system stores the application including the salutation

### Requirement: Public join endpoint
The system SHALL provide a public endpoint `POST /api/public/join` that accepts membership applications without user authentication. The endpoint SHALL require a valid API key in the `X-Api-Key` header.

#### Scenario: Successful application submission
- **WHEN** a POST request is sent to `/api/public/join` with a valid API key and all required fields
- **THEN** the system creates an application with status "Offen" and returns HTTP 201

#### Scenario: Missing API key
- **WHEN** a POST request is sent to `/api/public/join` without an `X-Api-Key` header
- **THEN** the system returns HTTP 401

#### Scenario: Invalid API key
- **WHEN** a POST request is sent to `/api/public/join` with an incorrect API key
- **THEN** the system returns HTTP 401

#### Scenario: Missing required field
- **WHEN** a POST request is sent with a valid API key but without the `email` field
- **THEN** the system returns HTTP 422 with an error indicating the missing field

#### Scenario: Shares below minimum
- **WHEN** a POST request is sent with shares set to 0
- **THEN** the system returns HTTP 422 with an error indicating shares must be at least 1

### Requirement: Confirmation mail on application
The system SHALL send a confirmation email to the applicant after a successful application submission. The email SHALL contain the applicant's name, the number of shares, the total amount to transfer (shares × share value from config), and the bank account details (IBAN, bank name, BIC) from the config store.

#### Scenario: Confirmation mail sent
- **WHEN** an application is successfully created for "Max Mustermann" with email "max@example.com" and 2 shares, and share_value_cents is 5000, bank_iban is "DE89...", bank_name is "GLS Bank"
- **THEN** the system queues a confirmation email to "max@example.com" containing the amount "100,00 €" and the bank details

#### Scenario: SMTP not configured
- **WHEN** an application is successfully created but SMTP configuration is missing
- **THEN** the application is still stored with status "Offen" and the mail sending failure is logged

### Requirement: List applications
The system SHALL provide an authenticated endpoint `GET /api/applications` that returns all applications. The endpoint SHALL support filtering by status. Access SHALL require the `manage_members` privilege.

#### Scenario: List all open applications
- **WHEN** an authenticated admin sends GET to `/api/applications?status=Offen`
- **THEN** the system returns all applications with status "Offen", ordered by created date descending

#### Scenario: List all applications without filter
- **WHEN** an authenticated admin sends GET to `/api/applications`
- **THEN** the system returns all non-deleted applications

#### Scenario: Unauthorized access
- **WHEN** an unauthenticated user sends GET to `/api/applications`
- **THEN** the system returns HTTP 401

### Requirement: Get single application
The system SHALL provide an authenticated endpoint `GET /api/applications/{id}` that returns a single application by ID. Access SHALL require the `manage_members` privilege.

#### Scenario: Get existing application
- **WHEN** an authenticated admin sends GET to `/api/applications/{id}` with a valid application ID
- **THEN** the system returns the application details

#### Scenario: Application not found
- **WHEN** an authenticated admin sends GET to `/api/applications/{id}` with a non-existent ID
- **THEN** the system returns HTTP 404

### Requirement: Confirm application
The system SHALL provide an authenticated endpoint `POST /api/applications/{id}/confirm` that confirms an application and creates a full member. The endpoint SHALL require the `manage_members` privilege. Upon confirmation:
- A new member SHALL be created with the next available member number
- `join_date` SHALL be set to the confirmation date
- `shares_at_joining` SHALL be set from the application's shares
- Automatic Eintritt and Aufstockung actions SHALL be created (same as normal member creation)
- The application status SHALL be set to "Bestätigt"

#### Scenario: Successful confirmation
- **WHEN** an admin confirms an open application for "Max Mustermann" with 2 shares
- **THEN** the system creates a member with the next available member number, join_date set to today, shares_at_joining=2, automatic Eintritt and Aufstockung actions, and sets the application status to "Bestätigt"

#### Scenario: Confirm already confirmed application
- **WHEN** an admin tries to confirm an application with status "Bestätigt"
- **THEN** the system returns HTTP 409 Conflict

#### Scenario: Confirm rejected application
- **WHEN** an admin tries to confirm an application with status "Abgelehnt"
- **THEN** the system returns HTTP 409 Conflict

### Requirement: Reject application
The system SHALL provide an authenticated endpoint `POST /api/applications/{id}/reject` that rejects an application. The endpoint SHALL require the `manage_members` privilege. The application status SHALL be set to "Abgelehnt".

#### Scenario: Successful rejection
- **WHEN** an admin rejects an open application
- **THEN** the system sets the application status to "Abgelehnt"

#### Scenario: Reject already confirmed application
- **WHEN** an admin tries to reject an application with status "Bestätigt"
- **THEN** the system returns HTTP 409 Conflict

### Requirement: API key configuration
The system SHALL store the public API key in the config store under the key `public_api_key` with value_type `secret`. The system SHALL provide an authenticated endpoint `POST /api/config/generate-api-key` that generates a new UUID v4 and stores it as `public_api_key`. Access SHALL require admin privileges.

#### Scenario: Generate new API key
- **WHEN** an admin calls POST `/api/config/generate-api-key`
- **THEN** the system generates a UUID v4, stores it as `public_api_key` (type secret) in the config store, and returns the generated key

#### Scenario: Regenerate API key
- **WHEN** an admin calls POST `/api/config/generate-api-key` and a key already exists
- **THEN** the system replaces the existing key with a newly generated UUID v4

### Requirement: Join configuration
The system SHALL read the following config store entries for processing applications:
- `share_value_cents` (int): Value of one share in cents
- `bank_iban` (string): IBAN for transfer
- `bank_name` (string): Bank name
- `bank_bic` (string, optional): BIC code
- `genossenschaft_name` (string): Name of the cooperative

#### Scenario: Config values used in confirmation mail
- **WHEN** an application is submitted and `share_value_cents` is 5000, `bank_iban` is "DE89...", `bank_name` is "GLS Bank", `genossenschaft_name` is "Muster eG"
- **THEN** the confirmation mail contains "Muster eG", the calculated amount, and bank details

#### Scenario: Missing required config
- **WHEN** an application is submitted but `share_value_cents` is not configured
- **THEN** the application is stored but the mail sending fails with a logged error
