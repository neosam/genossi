## Purpose

Export the complete audit log as a CSV file to the WebDAV backup location on every backup cycle. This ensures the audit-log trail (and the hash chain that accompanies it) is preserved off-site alongside the other backup artifacts (member lists, actions, documents, communication), providing a complete evidence package on external storage.

## Requirements

### Requirement: Audit log CSV export
The backup worker SHALL export the complete audit log as a CSV file named `audit-log.csv` in the backup root directory on WebDAV. The CSV SHALL contain all columns: id, timestamp, user_id, process, transaction_id, entity_type, entity_id, action, field_name, old_value, new_value, prev_hash, entry_hash. The file SHALL be overwritten on each backup cycle.

#### Scenario: Audit log exported
- **WHEN** the backup worker runs a backup cycle and audit log entries exist
- **THEN** the worker writes `audit-log.csv` to the backup root directory containing all audit log entries ordered by rowid ascending

#### Scenario: Audit log empty
- **WHEN** the backup worker runs a backup cycle and no audit log entries exist
- **THEN** the worker writes `audit-log.csv` with only the header row

#### Scenario: CSV format
- **WHEN** the audit log CSV is generated
- **THEN** each row contains pipe-delimited hash fields (prev_hash, entry_hash) that allow independent verification of the hash chain
