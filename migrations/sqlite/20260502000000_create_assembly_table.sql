CREATE TABLE IF NOT EXISTS assembly (
    id BLOB PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    date TEXT NOT NULL,
    location TEXT,
    status TEXT NOT NULL DEFAULT 'Preparation',
    opened_at TEXT,
    closed_at TEXT,
    created TEXT NOT NULL,
    deleted TEXT,
    version BLOB NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_assembly_status ON assembly(status);
CREATE INDEX IF NOT EXISTS idx_assembly_deleted ON assembly(deleted);
CREATE INDEX IF NOT EXISTS idx_assembly_date ON assembly(date);
