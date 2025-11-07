CREATE TABLE IF NOT EXISTS inventur (
    id BLOB PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    description TEXT NOT NULL,
    start_date TEXT NOT NULL,
    end_date TEXT,
    status TEXT NOT NULL,
    created_by TEXT NOT NULL,
    created TEXT NOT NULL,
    deleted TEXT,
    version BLOB NOT NULL
);

-- Create index on status for filtering by status
CREATE INDEX idx_inventur_status ON inventur(status);

-- Create index on deleted for filtering active inventurs
CREATE INDEX idx_inventur_deleted ON inventur(deleted);

-- Create index on start_date for sorting
CREATE INDEX idx_inventur_start_date ON inventur(start_date);

-- Create index on created_by for user filtering
CREATE INDEX idx_inventur_created_by ON inventur(created_by);
