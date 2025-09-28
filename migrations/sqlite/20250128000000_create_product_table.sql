CREATE TABLE IF NOT EXISTS product (
    id BLOB PRIMARY KEY NOT NULL,
    ean TEXT NOT NULL,
    name TEXT NOT NULL,
    short_name TEXT NOT NULL,
    sales_unit TEXT NOT NULL,
    requires_weighing INTEGER NOT NULL,
    price INTEGER NOT NULL,
    created TEXT NOT NULL,
    deleted TEXT,
    version BLOB NOT NULL
);

-- Create unique index on EAN for fast lookups
CREATE UNIQUE INDEX idx_product_ean ON product(ean);

-- Create index on deleted for filtering active products
CREATE INDEX idx_product_deleted ON product(deleted);

-- Create index on name for sorting
CREATE INDEX idx_product_name ON product(name);