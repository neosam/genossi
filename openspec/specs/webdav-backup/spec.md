## MODIFIED Requirements

### Requirement: WebDAV backup worker
The backup worker is NOT modified by this change. The qualified timestamp system runs as an independent worker. The backup worker continues to perform only its regular backup cycle (members, actions, documents) without any timestamp-related steps.

Note: The timestamp worker independently uploads .tsr files to the `audit-timestamps/` subdirectory on WebDAV when configured, but this does not affect the backup worker's behavior.
