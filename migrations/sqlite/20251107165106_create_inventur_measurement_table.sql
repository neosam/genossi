CREATE TABLE IF NOT EXISTS inventur_measurement (
    id BLOB PRIMARY KEY NOT NULL,
    inventur_id BLOB NOT NULL,
    product_id BLOB NOT NULL,
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
    FOREIGN KEY (product_id) REFERENCES product(id),
    FOREIGN KEY (rack_id) REFERENCES rack(id),
    FOREIGN KEY (container_id) REFERENCES container(id)
);

-- Create indexes for efficient queries
CREATE INDEX idx_inventur_measurement_inventur_id ON inventur_measurement(inventur_id);
CREATE INDEX idx_inventur_measurement_product_id ON inventur_measurement(product_id);
CREATE INDEX idx_inventur_measurement_rack_id ON inventur_measurement(rack_id);
CREATE INDEX idx_inventur_measurement_measured_at ON inventur_measurement(measured_at);
CREATE INDEX idx_inventur_measurement_deleted ON inventur_measurement(deleted);

-- Create composite index for common queries (inventur + product + rack)
CREATE INDEX idx_inventur_measurement_lookup ON inventur_measurement(inventur_id, product_id, rack_id) WHERE deleted IS NULL;
