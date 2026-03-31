## ADDED Requirements

### Requirement: Member data model
The system SHALL store members with the following fields:
- `id` (UUID, system-generated, primary key)
- `member_number` (i64, unique, user-visible identifier)
- `first_name` (String, required)
- `last_name` (String, required)
- `email` (Optional String)
- `company` (Optional String)
- `comment` (Optional String)
- `street` (Optional String)
- `house_number` (Optional String)
- `postal_code` (Optional String)
- `city` (Optional String)
- `join_date` (Date, required)
- `shares_at_joining` (i32, required)
- `current_shares` (i32, required)
- `current_balance` (i64 in cents, required)
- `exit_date` (Optional Date)
- `bank_account` (Optional String)
- `created` (DateTime, system-generated)
- `deleted` (Optional DateTime, for soft delete)
- `version` (UUID, for optimistic locking)

#### Scenario: Member stored with all fields
- **WHEN** a member is created with all fields provided
- **THEN** the system stores the member with a generated UUID, created timestamp, and version UUID

#### Scenario: Member number uniqueness
- **WHEN** a member is created with a member_number that already exists
- **THEN** the system SHALL reject the creation with a validation error

### Requirement: Create member
The system SHALL allow authenticated users with `manage_members` privilege to create new members via `POST /api/members`.

#### Scenario: Successful member creation
- **WHEN** an authenticated user with `manage_members` privilege sends a POST request with valid member data
- **THEN** the system creates the member, assigns a UUID and version, sets the created timestamp, and returns the created member with HTTP 200

#### Scenario: Missing required fields
- **WHEN** a POST request is sent without `first_name`, `last_name`, `join_date`, or `member_number`
- **THEN** the system returns HTTP 400 with field-level validation errors

#### Scenario: Insufficient privileges
- **WHEN** a user without `manage_members` privilege attempts to create a member
- **THEN** the system returns HTTP 401 Unauthorized

### Requirement: Read all members
The system SHALL allow authenticated users with `view_members` privilege to list all active members via `GET /api/members`.

#### Scenario: List active members
- **WHEN** an authenticated user with `view_members` privilege sends a GET request
- **THEN** the system returns all members where `deleted` is NULL, with HTTP 200

#### Scenario: Deleted members excluded
- **WHEN** members exist with a non-null `deleted` timestamp
- **THEN** those members SHALL NOT appear in the list response

### Requirement: Read single member
The system SHALL allow authenticated users with `view_members` privilege to retrieve a single member via `GET /api/members/{id}`.

#### Scenario: Member found
- **WHEN** a GET request is sent with a valid member UUID
- **THEN** the system returns the member data with HTTP 200

#### Scenario: Member not found
- **WHEN** a GET request is sent with a UUID that does not exist or is soft-deleted
- **THEN** the system returns HTTP 404

### Requirement: Update member
The system SHALL allow authenticated users with `manage_members` privilege to update a member via `PUT /api/members/{id}`.

#### Scenario: Successful update
- **WHEN** a PUT request is sent with valid updated data and matching version
- **THEN** the system updates the member, generates a new version UUID, and returns the updated member with HTTP 200

#### Scenario: Version conflict
- **WHEN** a PUT request is sent with a version UUID that does not match the stored version
- **THEN** the system returns an error indicating a version conflict

#### Scenario: Member not found on update
- **WHEN** a PUT request is sent for a member UUID that does not exist
- **THEN** the system returns HTTP 404

### Requirement: Delete member (soft delete)
The system SHALL allow authenticated users with `manage_members` privilege to soft-delete a member via `DELETE /api/members/{id}`.

#### Scenario: Successful soft delete
- **WHEN** a DELETE request is sent for an existing member
- **THEN** the system sets the `deleted` timestamp on the member and returns HTTP 204

#### Scenario: Delete non-existent member
- **WHEN** a DELETE request is sent for a UUID that does not exist
- **THEN** the system returns HTTP 404

### Requirement: OpenAPI documentation
The system SHALL expose all member endpoints in the Swagger UI at `/swagger-ui/` with complete OpenAPI annotations including request/response schemas and examples.

#### Scenario: Swagger UI accessible
- **WHEN** a user navigates to `/swagger-ui/`
- **THEN** the Swagger UI loads and displays all member management endpoints with their schemas

### Requirement: Member list page
The frontend SHALL provide a page listing all members with their member number, name, and key details.

#### Scenario: View member list
- **WHEN** an authenticated user navigates to the members page
- **THEN** the system displays a table of all active members with member_number, last_name, first_name, city, current_shares, and join_date

#### Scenario: Navigate to member detail
- **WHEN** a user clicks on a member row in the list
- **THEN** the system navigates to the member detail page

### Requirement: Member detail page
The frontend SHALL provide a page to view and edit a single member's data.

#### Scenario: View existing member
- **WHEN** a user navigates to the detail page of an existing member
- **THEN** all member fields are displayed in a form

#### Scenario: Edit and save member
- **WHEN** a user modifies fields and clicks save
- **THEN** the system sends a PUT request and navigates back to the member list on success

#### Scenario: Create new member
- **WHEN** a user navigates to the member detail page without an ID (new member)
- **THEN** an empty form is displayed, and saving sends a POST request

### Requirement: RBAC privileges for member management
The system SHALL define two privileges: `view_members` and `manage_members`. The `admin` role SHALL have both privileges.

#### Scenario: Admin has full access
- **WHEN** a user with the `admin` role accesses member endpoints
- **THEN** all CRUD operations are permitted

#### Scenario: Unprivileged user denied
- **WHEN** a user without any member privileges accesses member endpoints
- **THEN** the system returns HTTP 401 for all operations
