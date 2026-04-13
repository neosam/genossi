## MODIFIED Requirements

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
- `migrated` (bool, default false)
- `status` (MemberStatus enum, default Normal)
- `created` (DateTime, system-generated)
- `deleted` (Optional DateTime, for soft delete)
- `version` (UUID, for optimistic locking)

#### Scenario: Member stored with all fields
- **WHEN** a member is created with all fields provided
- **THEN** the system stores the member with a generated UUID, created timestamp, version UUID, `migrated` set to `false`, and `status` set to the provided value or `Normal` if omitted

#### Scenario: Member number uniqueness
- **WHEN** a member is created with a member_number that already exists
- **THEN** the system SHALL reject the creation with a validation error
