CREATE TABLE IF NOT EXISTS user_preferences (
    id BLOB PRIMARY KEY NOT NULL,
    user_id TEXT NOT NULL,
    key TEXT NOT NULL,
    value TEXT NOT NULL,
    created TEXT NOT NULL,
    deleted TEXT,
    version BLOB NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_user_preferences_user_key ON user_preferences(user_id, key);
CREATE INDEX IF NOT EXISTS idx_user_preferences_deleted ON user_preferences(deleted);
