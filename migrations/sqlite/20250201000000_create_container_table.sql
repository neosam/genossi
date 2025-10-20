CREATE TABLE IF NOT EXISTS container (
    id BLOB PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    weight_grams INTEGER NOT NULL,
    description TEXT,
    created TEXT NOT NULL,
    deleted TEXT,
    version BLOB NOT NULL
);

-- Create index on name for fast lookups and sorting
CREATE INDEX idx_container_name ON container(name);

-- Create index on deleted for filtering active containers
CREATE INDEX idx_container_deleted ON container(deleted);

-- Create index on weight_grams for filtering by weight ranges
CREATE INDEX idx_container_weight ON container(weight_grams);