-- Create product_rack junction table for many-to-many relationship
CREATE TABLE IF NOT EXISTS product_rack (
    product_id BLOB NOT NULL,
    rack_id BLOB NOT NULL,
    quantity INTEGER NOT NULL DEFAULT 1,
    created TEXT NOT NULL,
    deleted TEXT,
    version BLOB NOT NULL,
    PRIMARY KEY (product_id, rack_id),
    FOREIGN KEY (product_id) REFERENCES product(id),
    FOREIGN KEY (rack_id) REFERENCES rack(id)
);

-- Create indexes for efficient queries
CREATE INDEX IF NOT EXISTS idx_product_rack_product_id ON product_rack(product_id);
CREATE INDEX IF NOT EXISTS idx_product_rack_rack_id ON product_rack(rack_id);
CREATE INDEX IF NOT EXISTS idx_product_rack_deleted ON product_rack(deleted);

-- Create index for active relationships (where deleted IS NULL)
CREATE INDEX IF NOT EXISTS idx_product_rack_active ON product_rack(product_id, rack_id) WHERE deleted IS NULL;