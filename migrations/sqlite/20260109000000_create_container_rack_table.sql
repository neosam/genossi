-- Create container_rack junction table for many-to-many relationship between containers and racks
CREATE TABLE IF NOT EXISTS container_rack (
    container_id BLOB NOT NULL,
    rack_id BLOB NOT NULL,
    sort_order INTEGER NOT NULL DEFAULT 0,
    created TEXT NOT NULL,
    deleted TEXT,
    version BLOB NOT NULL,
    PRIMARY KEY (container_id, rack_id),
    FOREIGN KEY (container_id) REFERENCES container(id),
    FOREIGN KEY (rack_id) REFERENCES rack(id)
);

-- Create indexes for efficient queries
CREATE INDEX IF NOT EXISTS idx_container_rack_container_id ON container_rack(container_id);
CREATE INDEX IF NOT EXISTS idx_container_rack_rack_id ON container_rack(rack_id);
CREATE INDEX IF NOT EXISTS idx_container_rack_deleted ON container_rack(deleted);
CREATE INDEX IF NOT EXISTS idx_container_rack_active ON container_rack(container_id, rack_id) WHERE deleted IS NULL;
CREATE INDEX IF NOT EXISTS idx_container_rack_rack_sort ON container_rack(rack_id, sort_order) WHERE deleted IS NULL;
