-- Add token column to inventur table for token-based authentication
ALTER TABLE inventur ADD COLUMN token TEXT;
CREATE UNIQUE INDEX idx_inventur_token ON inventur(token) WHERE token IS NOT NULL;
