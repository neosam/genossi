ALTER TABLE inventur_custom_entry ADD COLUMN ean TEXT;
CREATE INDEX idx_inventur_custom_entry_ean ON inventur_custom_entry(ean) WHERE ean IS NOT NULL;
