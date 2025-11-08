CREATE TABLE IF NOT EXISTS inventur_custom_entry (
    id BLOB PRIMARY KEY NOT NULL,
    inventur_id BLOB NOT NULL,
    custom_product_name TEXT NOT NULL,
    rack_id BLOB,
    container_id BLOB,
    count INTEGER,
    weight_grams INTEGER,
    measured_by TEXT NOT NULL,
    measured_at TEXT NOT NULL,
    notes TEXT,
    created TEXT NOT NULL,
    deleted TEXT,
    version BLOB NOT NULL,
    FOREIGN KEY (inventur_id) REFERENCES inventur(id),
    FOREIGN KEY (rack_id) REFERENCES rack(id),
    FOREIGN KEY (container_id) REFERENCES container(id)
);

-- Create indexes for efficient queries
CREATE INDEX idx_inventur_custom_entry_inventur_id ON inventur_custom_entry(inventur_id);
CREATE INDEX idx_inventur_custom_entry_rack_id ON inventur_custom_entry(rack_id);
CREATE INDEX idx_inventur_custom_entry_measured_at ON inventur_custom_entry(measured_at);
CREATE INDEX idx_inventur_custom_entry_deleted ON inventur_custom_entry(deleted);

-- Create composite index for common queries (inventur + rack)
CREATE INDEX idx_inventur_custom_entry_lookup ON inventur_custom_entry(inventur_id, rack_id) WHERE deleted IS NULL;
