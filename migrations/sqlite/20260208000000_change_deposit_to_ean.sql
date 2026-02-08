-- Change deposit from INTEGER (cents) to TEXT (EAN reference)
-- Add new column for deposit EAN reference
ALTER TABLE product ADD COLUMN deposit_ean TEXT;

-- Drop old deposit column (SQLite 3.35.0+ supports DROP COLUMN)
ALTER TABLE product DROP COLUMN deposit;
