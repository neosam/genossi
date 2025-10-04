-- Create rack table
CREATE TABLE IF NOT EXISTS rack (
    id BLOB PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    description TEXT NOT NULL,
    created TEXT NOT NULL,
    deleted TEXT,
    version BLOB NOT NULL
);

-- Create index for soft delete queries
CREATE INDEX IF NOT EXISTS idx_rack_deleted ON rack(deleted);