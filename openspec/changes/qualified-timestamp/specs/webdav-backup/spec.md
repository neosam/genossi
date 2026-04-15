## MODIFIED Requirements

### Requirement: WebDAV backup worker
The system SHALL run a background worker that periodically uploads backup data to a WebDAV server. The worker SHALL read its configuration from the config store on each iteration and sleep for the configured interval between runs. When qualified timestamping is enabled (`tsa_enabled` = true), the worker SHALL perform an additional timestamp step after the regular backup: obtain the current audit_log latest hash, request a qualified timestamp from the configured TSA, store the token locally, and upload the .tsr file to the `audit-timestamps/` subdirectory on WebDAV.

#### Scenario: Worker runs when enabled
- **WHEN** `backup_webdav_enabled` config is set to `true` and all required config keys are present
- **THEN** the worker performs a full backup cycle (members, actions, documents), then performs the qualified timestamp step if `tsa_enabled` is true, and uploads results to the configured WebDAV server

#### Scenario: Worker skips when disabled
- **WHEN** `backup_webdav_enabled` config is set to `false` or missing
- **THEN** the worker sleeps for the configured interval without performing any backup or timestamp operations

#### Scenario: Worker sleeps for configured interval
- **WHEN** a backup cycle completes (success or failure)
- **THEN** the worker sleeps for `backup_interval_hours` hours (default: 24) before the next cycle

#### Scenario: Timestamp step fails but backup succeeds
- **WHEN** the regular backup succeeds but the TSA request fails
- **THEN** the worker logs the timestamp failure, sets timestamp status to "tsa_failed", but reports the overall backup as successful

#### Scenario: Timestamp step skipped when not configured
- **WHEN** `tsa_enabled` is false or not set
- **THEN** the worker performs only the regular backup without the timestamp step
