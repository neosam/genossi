-- Add granular privileges for role-based access control
-- This migration adds the specific privileges needed for the frontend RBAC system

-- Insert new granular privileges (ignore if already exist)
INSERT OR IGNORE INTO privilege (name, update_timestamp, update_process) VALUES 
    ('view_inventory', datetime('now'), 'migration-20250210000000'),
    ('manage_inventory', datetime('now'), 'migration-20250210000000'),
    ('detect_duplicates', datetime('now'), 'migration-20250210000000'),
    ('manage_users', datetime('now'), 'migration-20250210000000');

-- Create inventory_steward role (ignore if already exists)
INSERT OR IGNORE INTO role (name, update_timestamp, update_process) VALUES
    ('inventory_steward', datetime('now'), 'migration-20250210000000');

-- Grant privileges to existing roles

-- Admin role gets all privileges (maintains backward compatibility + new granular access)
INSERT OR IGNORE INTO role_privilege (role_name, privilege_name, update_timestamp, update_process) VALUES
    ('admin', 'view_inventory', datetime('now'), 'migration-20250210000000'),
    ('admin', 'manage_inventory', datetime('now'), 'migration-20250210000000'),
    ('admin', 'detect_duplicates', datetime('now'), 'migration-20250210000000'),
    ('admin', 'manage_users', datetime('now'), 'migration-20250210000000');

-- User role gets inventory management and duplicate detection
INSERT OR IGNORE INTO role_privilege (role_name, privilege_name, update_timestamp, update_process) VALUES
    ('user', 'view_inventory', datetime('now'), 'migration-20250210000000'),
    ('user', 'manage_inventory', datetime('now'), 'migration-20250210000000'),
    ('user', 'detect_duplicates', datetime('now'), 'migration-20250210000000');

-- Readonly role gets only view access
INSERT OR IGNORE INTO role_privilege (role_name, privilege_name, update_timestamp, update_process) VALUES
    ('readonly', 'view_inventory', datetime('now'), 'migration-20250210000000');

-- Inventory steward role gets inventory management and duplicate detection (perfect for inventory preparation)
INSERT OR IGNORE INTO role_privilege (role_name, privilege_name, update_timestamp, update_process) VALUES
    ('inventory_steward', 'view_inventory', datetime('now'), 'migration-20250210000000'),
    ('inventory_steward', 'manage_inventory', datetime('now'), 'migration-20250210000000'),
    ('inventory_steward', 'detect_duplicates', datetime('now'), 'migration-20250210000000');