-- Add review_state to inventur_measurement
ALTER TABLE inventur_measurement ADD COLUMN review_state TEXT NOT NULL DEFAULT 'unreviewed';
CREATE INDEX idx_inventur_measurement_review_state ON inventur_measurement(review_state);

-- Add review_state to inventur_custom_entry
ALTER TABLE inventur_custom_entry ADD COLUMN review_state TEXT NOT NULL DEFAULT 'unreviewed';
CREATE INDEX idx_inventur_custom_entry_review_state ON inventur_custom_entry(review_state);
