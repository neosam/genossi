## MODIFIED Requirements

### Requirement: WebDAV communication sync

The WebDAV backup worker SHALL synchronize communication files to a `kommunikation/` subfolder per member within the base backup directory, using an append-only strategy. The worker SHALL ensure the `kommunikation/` base directory exists before creating member subdirectories.

#### Scenario: New mails are synced
- **WHEN** a backup cycle runs and new mails exist that have not been synced before
- **THEN** the worker SHALL upload those mails as .txt files and mark them as synced

#### Scenario: Already-synced mails are skipped
- **WHEN** a mail has already been synced in a previous cycle
- **THEN** the worker SHALL NOT re-upload that mail

#### Scenario: Sync tracking persists across restarts
- **WHEN** the server restarts
- **THEN** previously synced mail IDs SHALL still be recognized (stored in database)

#### Scenario: Communication base directory is created before member directories
- **WHEN** the communication sync starts
- **THEN** the worker SHALL recursively create the `{base_dir}/kommunikation` directory path before iterating over individual member communications
- **AND** member subdirectory creation via MKCOL SHALL succeed because the parent directory exists
