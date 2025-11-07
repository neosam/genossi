-- Add privileges and role for inventory counting (Inventur) feature
-- This migration adds privileges for managing inventory counting sessions and recording measurements

-- Insert new inventur privileges (ignore if already exist)
INSERT OR IGNORE INTO privilege (name, update_timestamp, update_process) VALUES
    ('view_inventur', datetime('now'), 'migration-20251107165143'),
    ('manage_inventur', datetime('now'), 'migration-20251107165143'),
    ('perform_inventur', datetime('now'), 'migration-20251107165143');

-- Create inventur_counter role (ignore if already exists)
-- This role is for staff who count inventory but cannot modify master data
INSERT OR IGNORE INTO role (name, update_timestamp, update_process) VALUES
    ('inventur_counter', datetime('now'), 'migration-20251107165143');

-- Grant privileges to existing roles

-- Admin role gets all inventur privileges
INSERT OR IGNORE INTO role_privilege (role_name, privilege_name, update_timestamp, update_process) VALUES
    ('admin', 'view_inventur', datetime('now'), 'migration-20251107165143'),
    ('admin', 'manage_inventur', datetime('now'), 'migration-20251107165143'),
    ('admin', 'perform_inventur', datetime('now'), 'migration-20251107165143');

-- User role gets full inventur access (view, manage, and perform)
INSERT OR IGNORE INTO role_privilege (role_name, privilege_name, update_timestamp, update_process) VALUES
    ('user', 'view_inventur', datetime('now'), 'migration-20251107165143'),
    ('user', 'manage_inventur', datetime('now'), 'migration-20251107165143'),
    ('user', 'perform_inventur', datetime('now'), 'migration-20251107165143');

-- Readonly role gets only view access to inventur data
INSERT OR IGNORE INTO role_privilege (role_name, privilege_name, update_timestamp, update_process) VALUES
    ('readonly', 'view_inventur', datetime('now'), 'migration-20251107165143');

-- Inventur counter role: can view inventory and inventur data, and perform measurements
-- This role CANNOT manage inventur sessions or modify master data (products, racks, etc.)
INSERT OR IGNORE INTO role_privilege (role_name, privilege_name, update_timestamp, update_process) VALUES
    ('inventur_counter', 'view_inventory', datetime('now'), 'migration-20251107165143'),
    ('inventur_counter', 'view_inventur', datetime('now'), 'migration-20251107165143'),
    ('inventur_counter', 'perform_inventur', datetime('now'), 'migration-20251107165143');
