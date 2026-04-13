## ADDED Requirements

### Requirement: WebDAV backup config section on config page
The frontend config page SHALL include a "WebDAV Backup" section, visually consistent with the existing SMTP and IMAP sections. The section SHALL allow administrators to configure all WebDAV backup settings.

#### Scenario: Section visible to admins
- **WHEN** an admin user opens the config page
- **THEN** a "WebDAV Backup" section is displayed with input fields for all backup settings

#### Scenario: Section not visible to non-admins
- **WHEN** a non-admin user attempts to access the config page
- **THEN** the access denied page is shown (existing behavior)

### Requirement: WebDAV URL input
The section SHALL include a text input for the WebDAV base URL with label and placeholder showing the expected format.

#### Scenario: Enter WebDAV URL
- **WHEN** the admin enters `https://cloud.example/remote.php/dav/files/user/` in the URL field
- **THEN** the value is stored as config key `backup_webdav_url` with value_type `string`

### Requirement: WebDAV credentials inputs
The section SHALL include text inputs for username and a password input for the app token. The password field SHALL show a "gesetzt" (set) indicator when a password is already stored, consistent with the SMTP password field behavior.

#### Scenario: Enter credentials
- **WHEN** the admin enters username and password and saves
- **THEN** `backup_webdav_username` is stored as `string` and `backup_webdav_password` is stored as `secret`

#### Scenario: Password already set indicator
- **WHEN** `backup_webdav_password` exists in the config
- **THEN** the password field shows "Passwort gesetzt" indicator and the field is empty (not pre-filled)

### Requirement: Target directory input
The section SHALL include a text input for the target directory name on the WebDAV server.

#### Scenario: Enter target directory
- **WHEN** the admin enters `genossi-export` in the directory field
- **THEN** the value is stored as config key `backup_webdav_directory` with value_type `string`

### Requirement: Backup interval input
The section SHALL include a numeric input for the backup interval in hours.

#### Scenario: Set backup interval
- **WHEN** the admin enters `24` in the interval field
- **THEN** the value is stored as config key `backup_interval_hours` with value_type `int`

### Requirement: Backup enabled toggle
The section SHALL include a toggle or checkbox to enable/disable automatic backups.

#### Scenario: Enable backup
- **WHEN** the admin enables the backup toggle and saves
- **THEN** `backup_webdav_enabled` is stored with value `true` and value_type `bool`

#### Scenario: Disable backup
- **WHEN** the admin disables the backup toggle and saves
- **THEN** `backup_webdav_enabled` is stored with value `false` and value_type `bool`

### Requirement: Save button for backup settings
The section SHALL include a save button that persists all WebDAV backup settings. The button SHALL show a loading state during save and display success/error feedback.

#### Scenario: Save all settings
- **WHEN** the admin fills in URL, username, password, directory, interval, and enabled, then clicks save
- **THEN** all config entries are persisted via the existing config API endpoints and a success message is displayed

#### Scenario: Save with empty password
- **WHEN** the admin saves without entering a password (field empty) and a password was previously set
- **THEN** the existing password is not overwritten

### Requirement: Backup status display
The section SHALL display the last backup run timestamp and status below the configuration form, read from config entries `backup_last_run` and `backup_last_status`.

#### Scenario: Show last successful backup
- **WHEN** `backup_last_run` is `2026-04-12T03:00:00Z` and `backup_last_status` is `Erfolgreich: 6 CSVs, 142 Dokumente synchronisiert`
- **THEN** the section displays "Letztes Backup: 12.04.2026 03:00" and the status message in green

#### Scenario: Show last failed backup
- **WHEN** `backup_last_status` contains an error message
- **THEN** the status is displayed in red

#### Scenario: No backup run yet
- **WHEN** `backup_last_run` does not exist in config
- **THEN** the section displays "Noch kein Backup durchgeführt"

### Requirement: I18n support
All labels, placeholders, and messages in the WebDAV backup section SHALL use the i18n system with keys for German, English, and Czech translations.

#### Scenario: German labels displayed
- **WHEN** the user's locale is German
- **THEN** labels are displayed in German (e.g., "WebDAV-Sicherung", "URL", "Benutzername", "Passwort", "Zielverzeichnis", "Intervall (Stunden)", "Aktiviert")

#### Scenario: English labels displayed
- **WHEN** the user's locale is English
- **THEN** labels are displayed in English (e.g., "WebDAV Backup", "URL", "Username", "Password", "Target Directory", "Interval (Hours)", "Enabled")
