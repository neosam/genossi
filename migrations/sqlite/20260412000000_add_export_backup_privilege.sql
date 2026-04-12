-- Add export_backup privilege
INSERT OR IGNORE INTO privilege (name, update_timestamp, update_process) VALUES
    ('export_backup', datetime('now'), 'migration-20260412000000');

-- Assign to admin role
INSERT OR IGNORE INTO role_privilege (role_name, privilege_name, update_timestamp, update_process) VALUES
    ('admin', 'export_backup', datetime('now'), 'migration-20260412000000');
