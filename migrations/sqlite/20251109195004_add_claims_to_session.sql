-- Add claims column to session table for storing generic session metadata
ALTER TABLE session ADD COLUMN claims TEXT;
CREATE INDEX idx_session_claims ON session(claims);
