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
- `action_count` (i32, required, default 0): number of share actions from Excel import (excluding Eintritt)
- `created` (DateTime, system-generated)
- `deleted` (Optional DateTime, for soft delete)
- `version` (UUID, for optimistic locking)

#### Scenario: Member stored with all fields
- **WHEN** a member is created with all fields provided
- **THEN** the system stores the member with a generated UUID, created timestamp, and version UUID

#### Scenario: Member number uniqueness
- **WHEN** a member is created with a member_number that already exists
- **THEN** the system SHALL reject the creation with a validation error

## ADDED Requirements

### Requirement: Excel import reads action_count
The system SHALL read the "Anzahl Aktionen" column from Excel imports and store it in the `action_count` field. If the column is empty, the value defaults to 0.

#### Scenario: Action count imported
- **WHEN** an Excel row has "Anzahl Aktionen" value of 3
- **THEN** the imported member's `action_count` SHALL be 3

#### Scenario: Action count empty
- **WHEN** an Excel row has an empty "Anzahl Aktionen" column
- **THEN** the imported member's `action_count` SHALL be 0

### Requirement: Auto-migration on import
The system SHALL automatically create Eintritt and Aufstockung actions for members that meet auto-migration criteria during Excel import.

#### Scenario: Auto-migratable member
- **WHEN** an Excel row has `action_count == 0` AND `shares_at_joining == current_shares`
- **THEN** the system SHALL create an Eintritt action (date = join_date, shares_change = 0) and an Aufstockung action (date = join_date, shares_change = shares_at_joining)

#### Scenario: Non-auto-migratable member
- **WHEN** an Excel row has `action_count > 0` OR `shares_at_joining != current_shares`
- **THEN** the system SHALL NOT create any automatic actions
