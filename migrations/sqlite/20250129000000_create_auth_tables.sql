-- Authentication system tables
-- Based on the AUTHENTICATION.md specification

-- Users table
CREATE TABLE user (
    name TEXT NOT NULL PRIMARY KEY,
    update_timestamp TEXT,
    update_process TEXT NOT NULL
);

-- Roles table  
CREATE TABLE role (
    name TEXT NOT NULL PRIMARY KEY,
    update_timestamp TEXT,
    update_process TEXT NOT NULL
);

-- Privileges table
CREATE TABLE privilege (
    name TEXT NOT NULL PRIMARY KEY,
    update_timestamp TEXT,
    update_process TEXT NOT NULL
);

-- User-Role relationship (many-to-many)
CREATE TABLE user_role (
    user_name TEXT NOT NULL,
    role_name TEXT NOT NULL,
    update_timestamp TEXT,
    update_process TEXT NOT NULL,
    FOREIGN KEY (user_name) REFERENCES user(name) ON DELETE CASCADE,
    FOREIGN KEY (role_name) REFERENCES role(name) ON DELETE CASCADE,
    UNIQUE (user_name, role_name)
);

-- Role-Privilege relationship (many-to-many)  
CREATE TABLE role_privilege (
    role_name TEXT NOT NULL,
    privilege_name TEXT NOT NULL,
    update_timestamp TEXT,
    update_process TEXT NOT NULL,
    FOREIGN KEY (role_name) REFERENCES role(name) ON DELETE CASCADE,
    FOREIGN KEY (privilege_name) REFERENCES privilege(name) ON DELETE CASCADE,
    UNIQUE (role_name, privilege_name)
);

-- Session management for authentication
CREATE TABLE session (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    expires INTEGER NOT NULL,
    created INTEGER NOT NULL,
    FOREIGN KEY (user_id) REFERENCES user(name) ON DELETE CASCADE
);

-- Create indexes for better performance
CREATE INDEX idx_user_role_user ON user_role(user_name);
CREATE INDEX idx_user_role_role ON user_role(role_name);
CREATE INDEX idx_role_privilege_role ON role_privilege(role_name);
CREATE INDEX idx_role_privilege_privilege ON role_privilege(privilege_name);
CREATE INDEX idx_session_user ON session(user_id);
CREATE INDEX idx_session_expires ON session(expires);