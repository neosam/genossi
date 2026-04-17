-- Add last_used_at column for session inactivity tracking
ALTER TABLE session ADD COLUMN last_used_at INTEGER NOT NULL DEFAULT 0;

-- Initialize existing sessions with their creation timestamp
UPDATE session SET last_used_at = created WHERE last_used_at = 0;

-- Index for efficient cleanup queries
CREATE INDEX idx_session_last_used ON session(last_used_at);
