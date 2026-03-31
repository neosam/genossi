-- Member management privileges
INSERT OR IGNORE INTO privilege (name, update_timestamp, update_process) VALUES
    ('view_members', datetime('now'), 'migration-20260331000001'),
    ('manage_members', datetime('now'), 'migration-20260331000001');

-- Grant both privileges to admin role
INSERT OR IGNORE INTO role_privilege (role_name, privilege_name, update_timestamp, update_process) VALUES
    ('admin', 'view_members', datetime('now'), 'migration-20260331000001'),
    ('admin', 'manage_members', datetime('now'), 'migration-20260331000001');
