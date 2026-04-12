## ADDED Requirements

### Requirement: WebDAV backup config keys
The config store SHALL accept the following keys for WebDAV backup configuration:
- `backup_webdav_enabled` (bool): whether automatic backup is active
- `backup_webdav_url` (string): WebDAV base URL
- `backup_webdav_username` (string): WebDAV username
- `backup_webdav_password` (secret): WebDAV password/app token
- `backup_webdav_directory` (string): target directory name on WebDAV server
- `backup_interval_hours` (int): interval between backup runs in hours
- `backup_last_run` (string): ISO8601 timestamp of last backup run (written by worker)
- `backup_last_status` (string): status message of last backup run (written by worker)

#### Scenario: Store backup WebDAV URL
- **WHEN** `PUT /api/config/backup_webdav_url` is called with `{"value": "https://cloud.example/remote.php/dav/files/user/", "value_type": "string"}`
- **THEN** the config entry is stored

#### Scenario: Store backup password as secret
- **WHEN** `PUT /api/config/backup_webdav_password` is called with `{"value": "app-token-xyz", "value_type": "secret"}`
- **THEN** the config entry is stored and the value is masked as `***` when read via `GET /api/config`

#### Scenario: Store backup interval
- **WHEN** `PUT /api/config/backup_interval_hours` is called with `{"value": "24", "value_type": "int"}`
- **THEN** the config entry is stored after validating the value is a valid integer
