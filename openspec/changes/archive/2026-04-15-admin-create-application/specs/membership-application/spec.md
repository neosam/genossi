## ADDED Requirements

### Requirement: Admin create application endpoint
The system SHALL provide an authenticated endpoint `POST /api/applications` that creates a new membership application. The endpoint SHALL require the `manage_members` privilege. Required fields: `first_name` (String), `last_name` (String), `shares` (i32, minimum 1). Optional fields: `salutation` (Enum), `email` (String), `street` (String), `house_number` (String), `postal_code` (String), `city` (String), `send_mail` (bool, default: `false`). Only fields that are required in the member entity (first_name, last_name, shares) SHALL be required here.

#### Scenario: Successful admin creation with minimal fields
- **WHEN** an authenticated admin sends POST to `/api/applications` with first_name "Max", last_name "Mustermann", shares 1, and no other fields
- **THEN** the system creates an application with status "Offen" and returns HTTP 201, and no confirmation mail is sent

#### Scenario: Successful admin creation with all fields
- **WHEN** an authenticated admin sends POST to `/api/applications` with all fields including email, street, house_number, postal_code, city, and `send_mail` set to `false`
- **THEN** the system creates an application with status "Offen" and returns HTTP 201, and no confirmation mail is sent

#### Scenario: Successful admin creation with mail
- **WHEN** an authenticated admin sends POST to `/api/applications` with all required fields, email "max@example.com", and `send_mail` set to `true`
- **THEN** the system creates an application with status "Offen", returns HTTP 201, and sends a confirmation mail to the applicant

#### Scenario: send_mail true without email
- **WHEN** an authenticated admin sends POST to `/api/applications` with `send_mail` set to `true` but no `email` field
- **THEN** the system returns HTTP 422 with an error indicating that email is required when send_mail is true

#### Scenario: Missing required field
- **WHEN** an authenticated admin sends POST to `/api/applications` without the `first_name` field
- **THEN** the system returns HTTP 422 with an error indicating the missing field

#### Scenario: Shares below minimum
- **WHEN** an authenticated admin sends POST to `/api/applications` with shares set to 0
- **THEN** the system returns HTTP 422 with an error indicating shares must be at least 1

#### Scenario: Unauthorized access
- **WHEN** an unauthenticated user sends POST to `/api/applications`
- **THEN** the system returns HTTP 401

## MODIFIED Requirements

### Requirement: Public join endpoint
The system SHALL provide a public endpoint `POST /api/public/join` that accepts membership applications without user authentication. The endpoint SHALL require a valid API key in the `X-Api-Key` header. The endpoint SHALL always send a confirmation mail after successful submission.

#### Scenario: Successful application submission
- **WHEN** a POST request is sent to `/api/public/join` with a valid API key and all required fields
- **THEN** the system creates an application with status "Offen", returns HTTP 201, and sends a confirmation mail to the applicant

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
The system SHALL send a confirmation email to the applicant when `send_mail` is `true`. The email SHALL contain the applicant's name, the number of shares, the total amount to transfer (shares x share value from config), and the bank account details (IBAN, bank name, BIC) from the config store. The public join endpoint SHALL always set `send_mail` to `true`. The admin create endpoint SHALL default `send_mail` to `false`.

#### Scenario: Confirmation mail sent via public endpoint
- **WHEN** an application is successfully created via `POST /api/public/join` for "Max Mustermann" with email "max@example.com" and 2 shares
- **THEN** the system queues a confirmation email to "max@example.com"

#### Scenario: No confirmation mail via admin endpoint (default)
- **WHEN** an admin creates an application via `POST /api/applications` without specifying `send_mail`
- **THEN** no confirmation email is sent

#### Scenario: Confirmation mail via admin endpoint (opted in)
- **WHEN** an admin creates an application via `POST /api/applications` with `send_mail: true`
- **THEN** the system queues a confirmation email to the applicant

#### Scenario: SMTP not configured
- **WHEN** an application is created with `send_mail: true` but SMTP configuration is missing
- **THEN** the application is still stored with status "Offen" and the mail sending failure is logged
