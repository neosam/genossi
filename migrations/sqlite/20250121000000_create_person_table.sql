-- Create person table
CREATE TABLE IF NOT EXISTS person (
    id BLOB PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    age INTEGER NOT NULL,
    created TEXT NOT NULL,
    deleted TEXT,
    version BLOB NOT NULL
);

-- Create index for soft delete queries
CREATE INDEX IF NOT EXISTS idx_person_deleted ON person(deleted);