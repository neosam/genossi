-- Remove quantity column from product_rack table
-- SQLite doesn't support dropping columns directly, so we need to recreate the table

-- Create new table without quantity column
CREATE TABLE product_rack_new (
    product_id BLOB NOT NULL,
    rack_id BLOB NOT NULL,
    created TEXT NOT NULL,
    deleted TEXT NULL,
    version BLOB NOT NULL,
    PRIMARY KEY (product_id, rack_id),
    FOREIGN KEY (product_id) REFERENCES product(id),
    FOREIGN KEY (rack_id) REFERENCES rack(id)
);

-- Copy data from old table to new table (excluding quantity)
INSERT INTO product_rack_new (product_id, rack_id, created, deleted, version)
SELECT product_id, rack_id, created, deleted, version FROM product_rack;

-- Drop old table
DROP TABLE product_rack;

-- Rename new table to original name
ALTER TABLE product_rack_new RENAME TO product_rack;