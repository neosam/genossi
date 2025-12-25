-- Add sort_order column to product_rack table
-- SQLite doesn't support adding columns with complex defaults, so we need to recreate the table

-- Create new table with sort_order column
CREATE TABLE product_rack_new (
    product_id BLOB NOT NULL,
    rack_id BLOB NOT NULL,
    sort_order INTEGER NOT NULL DEFAULT 0,
    created TEXT NOT NULL,
    deleted TEXT NULL,
    version BLOB NOT NULL,
    PRIMARY KEY (product_id, rack_id),
    FOREIGN KEY (product_id) REFERENCES product(id),
    FOREIGN KEY (rack_id) REFERENCES rack(id)
);

-- Copy data from old table to new table, using ROW_NUMBER to preserve insertion order per rack
INSERT INTO product_rack_new (product_id, rack_id, sort_order, created, deleted, version)
SELECT
    product_id,
    rack_id,
    ROW_NUMBER() OVER (PARTITION BY rack_id ORDER BY created, rowid) as sort_order,
    created,
    deleted,
    version
FROM product_rack;

-- Drop old table
DROP TABLE product_rack;

-- Rename new table to original name
ALTER TABLE product_rack_new RENAME TO product_rack;

-- Recreate indexes
CREATE INDEX IF NOT EXISTS idx_product_rack_product_id ON product_rack(product_id);
CREATE INDEX IF NOT EXISTS idx_product_rack_rack_id ON product_rack(rack_id);
CREATE INDEX IF NOT EXISTS idx_product_rack_deleted ON product_rack(deleted);
CREATE INDEX IF NOT EXISTS idx_product_rack_active ON product_rack(product_id, rack_id) WHERE deleted IS NULL;
CREATE INDEX IF NOT EXISTS idx_product_rack_rack_sort ON product_rack(rack_id, sort_order) WHERE deleted IS NULL;
