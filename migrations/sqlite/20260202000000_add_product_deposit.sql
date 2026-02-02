-- Add deposit column to product table for storing deposit/Pfand prices
ALTER TABLE product ADD COLUMN deposit INTEGER NOT NULL DEFAULT 0;
