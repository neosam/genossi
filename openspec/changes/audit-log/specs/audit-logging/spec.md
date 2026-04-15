## ADDED Requirements

### Requirement: Audit log data model
The system SHALL store audit log entries with the following fields:
- `id` (UUID, system-generated, primary key)
- `timestamp` (DateTime, system-generated)
- `user_id` (String, the authenticated user who made the change)
- `process` (String, the service/process that performed the operation)
- `transaction_id` (UUID, groups related field changes in one operation)
- `entity_type` (String, e.g. "member", "member_action", "member_document", "application")
- `entity_id` (UUID, the ID of the changed entity)
- `action` (String, one of "create", "update", "delete")
- `field_name` (String, the name of the changed field)
- `old_value` (Optional String, the previous value, None for creates)
- `new_value` (Optional String, the new value, None for deletes)
- `prev_hash` (String, SHA256 hash of the previous audit log entry, empty string for the first entry)
- `entry_hash` (String, SHA256 hash computed over all fields including prev_hash)

The audit_log table SHALL be append-only. Entries SHALL NOT be updated or deleted.

#### Scenario: Audit entry stored with all fields
- **WHEN** an audited operation is performed
- **THEN** the system stores an audit log entry with timestamp, user_id, process, transaction_id, entity info, action, field change details, and hash chain values

#### Scenario: Append-only enforcement
- **WHEN** an attempt is made to update or delete an audit log entry
- **THEN** the system SHALL NOT provide any mechanism to modify or remove existing entries

### Requirement: Auditable trait
The system SHALL provide an `Auditable` trait that entities implement to enable automatic field extraction and diff computation. The trait SHALL provide:
- `entity_type()` returning the entity type string
- `entity_id()` returning the entity UUID
- `audit_fields()` returning a list of (field_name, Option<String>) pairs
- `diff()` with a default implementation that compares audit_fields pairwise

The following entities SHALL implement `Auditable`:
- `MemberEntity` with entity_type "member"
- `MemberActionEntity` with entity_type "member_action"
- `MemberDocumentEntity` with entity_type "member_document"
- `ApplicationEntity` with entity_type "application"

Only real data fields SHALL be included in `audit_fields`. The following fields SHALL be excluded: `id`, `version`, `created`, `deleted`.

All values SHALL be converted to String representation: numbers via `.to_string()`, Option fields as `Some("value")` or `None`, dates as ISO format, enums via `.as_str()`.

#### Scenario: Member entity audit fields
- **WHEN** `audit_fields()` is called on a MemberEntity
- **THEN** the result contains entries for member_number, first_name, last_name, salutation, title, email, company, comment, street, house_number, postal_code, city, join_date, shares_at_joining, current_shares, current_balance, action_count, migrated, exit_date, bank_account, status

#### Scenario: Diff computation with changes
- **WHEN** `diff()` is called with two entities where first_name and email differ
- **THEN** the result contains exactly two AuditFieldChange entries for first_name and email with old and new values

#### Scenario: Diff computation without changes
- **WHEN** `diff()` is called with two identical entities
- **THEN** the result is an empty list

#### Scenario: Excluded fields not in audit
- **WHEN** `audit_fields()` is called on any entity
- **THEN** the result does not contain entries for id, version, created, or deleted

### Requirement: Hash chain integrity
The system SHALL compute a SHA256 hash for each audit log entry. The hash input SHALL be a deterministic concatenation of all fields in a fixed order:
`SHA256(timestamp | user_id | process | transaction_id | entity_type | entity_id | action | field_name | old_value | new_value | prev_hash)`

Each entry's `prev_hash` SHALL be the `entry_hash` of the immediately preceding audit log entry. The first entry SHALL use an empty string as `prev_hash`.

When multiple fields change in a single transaction, entries SHALL be ordered alphabetically by `field_name` and chained sequentially.

#### Scenario: First audit entry
- **WHEN** the first audit log entry is created in an empty audit log
- **THEN** the entry's prev_hash is an empty string and entry_hash is SHA256 of all fields with empty prev_hash

#### Scenario: Chain continuation
- **WHEN** a new audit log entry is created after existing entries
- **THEN** the entry's prev_hash equals the entry_hash of the most recent existing entry

#### Scenario: Multi-field transaction ordering
- **WHEN** a transaction changes fields "email", "city", and "first_name"
- **THEN** entries are created in order: city, email, first_name (alphabetical) and each entry's prev_hash links to the previous entry's hash

#### Scenario: Chain verification succeeds
- **WHEN** the hash chain is verified and no entries have been tampered with
- **THEN** the verification reports success with no broken links

#### Scenario: Chain verification detects tampering
- **WHEN** the hash chain is verified and an entry in the middle has been modified
- **THEN** the verification reports the broken link with the entry ID where the chain breaks

### Requirement: Transaction grouping
The system SHALL assign the same `transaction_id` (UUID) to all audit log entries that result from a single service operation. This allows grouping related field changes.

#### Scenario: Update with multiple field changes
- **WHEN** a member update changes first_name, last_name, and email in a single operation
- **THEN** all three audit log entries share the same transaction_id

#### Scenario: Create logs all initial values
- **WHEN** a new member is created
- **THEN** all non-None audit fields are logged as separate entries with action "create", old_value None, and the same transaction_id

#### Scenario: Delete logs entity state
- **WHEN** a member is soft-deleted
- **THEN** an audit entry with action "delete" is created for each audited field, with old_value set and new_value None, all sharing the same transaction_id

### Requirement: Audit macros
The system SHALL provide macros `audited_create!`, `audited_update!`, and `audited_delete!` that atomically perform the DAO operation and log the audit entries within the same database transaction.

- `audited_create!(self, dao, entity, process, user_id, tx)` SHALL call dao.create, then log all non-None fields as new
- `audited_update!(self, dao, entity_id, new_entity, process, user_id, tx)` SHALL load the old entity via dao.find_by_id, call dao.update, compute the diff, and log only changed fields
- `audited_delete!(self, dao, entity_id, process, user_id, tx)` SHALL load the entity, set the deleted timestamp, call dao.update, and log all fields as deleted

The macros SHALL expect `self` to have an `audit_log_dao` field.

#### Scenario: audited_update with changes
- **WHEN** `audited_update!` is called and the old entity differs from the new entity in 2 fields
- **THEN** the DAO update is performed, 2 audit log entries are created with action "update", and all happen in the same transaction

#### Scenario: audited_update without changes
- **WHEN** `audited_update!` is called and the old entity is identical to the new entity
- **THEN** the DAO update is performed but no audit log entries are created

#### Scenario: audited_create logs all fields
- **WHEN** `audited_create!` is called for a new entity with 10 non-None fields
- **THEN** the DAO create is performed and 10 audit log entries are created with action "create"

### Requirement: System user for internal operations
The system SHALL use the user_id "SYSTEM" for audit log entries created by internal operations where no authenticated user context is available (Authentication::Full).

#### Scenario: Internal operation audit
- **WHEN** an operation is performed with Authentication::Full (no user context)
- **THEN** audit log entries are created with user_id "SYSTEM"
