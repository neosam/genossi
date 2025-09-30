-- Default authentication data
-- Creates core privileges, roles, and their relationships

-- Insert default privileges
INSERT INTO privilege (name, update_timestamp, update_process) VALUES 
    ('admin', datetime('now'), 'migration-20250129000001'),
    ('user', datetime('now'), 'migration-20250129000001'),
    ('readonly', datetime('now'), 'migration-20250129000001');

-- Insert default roles  
INSERT INTO role (name, update_timestamp, update_process) VALUES
    ('admin', datetime('now'), 'migration-20250129000001'),
    ('user', datetime('now'), 'migration-20250129000001'),
    ('readonly', datetime('now'), 'migration-20250129000001');

-- Link roles to privileges
INSERT INTO role_privilege (role_name, privilege_name, update_timestamp, update_process) VALUES
    -- Admin role gets admin privilege (full access)
    ('admin', 'admin', datetime('now'), 'migration-20250129000001'),
    -- User role gets user privilege (read-write access)  
    ('user', 'user', datetime('now'), 'migration-20250129000001'),
    -- Readonly role gets readonly privilege (read-only access)
    ('readonly', 'readonly', datetime('now'), 'migration-20250129000001');

-- Create default admin user for development
INSERT INTO user (name, update_timestamp, update_process) VALUES
    ('admin', datetime('now'), 'migration-20250129000001'),
    ('DEVUSER', datetime('now'), 'migration-20250129000001');

-- Assign admin role to default admin users
INSERT INTO user_role (user_name, role_name, update_timestamp, update_process) VALUES
    ('admin', 'admin', datetime('now'), 'migration-20250129000001'),
    ('DEVUSER', 'admin', datetime('now'), 'migration-20250129000001');