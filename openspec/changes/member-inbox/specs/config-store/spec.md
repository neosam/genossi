## ADDED Requirements

### Requirement: IMAP configuration keys

The config store SHALL recognize the following IMAP configuration keys used by the inbox worker:

- `imap_host` (`string`)
- `imap_port` (`int`)
- `imap_user` (`string`)
- `imap_pass` (`secret`)
- `imap_tls` (`bool`)
- `imap_mailbox` (`string`, default `INBOX` when unset)
- `imap_archive_mailbox` (`string`)
- `imap_poll_interval_seconds` (`int`, default `300` when unset)

These keys SHALL follow the same validation and storage rules as existing SMTP keys.

#### Scenario: Set IMAP credentials

- **WHEN** config entries are set for `imap_host`, `imap_port`, `imap_user`, `imap_pass`, `imap_tls`
- **THEN** the entries are stored with the declared value types and can be read back by the inbox worker

#### Scenario: Default poll interval

- **WHEN** `imap_poll_interval_seconds` is not set
- **THEN** the inbox worker uses 300 seconds as the polling interval

#### Scenario: Default mailbox

- **WHEN** `imap_mailbox` is not set
- **THEN** the inbox worker polls the `INBOX` folder
