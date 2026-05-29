CREATE TABLE IF NOT EXISTS repayment_phase (
    id BLOB PRIMARY KEY NOT NULL,
    fiscal_year INTEGER NOT NULL,
    share_value INTEGER NOT NULL,
    status TEXT NOT NULL DEFAULT 'Preparation',
    opened_at TEXT,
    closed_at TEXT,
    created TEXT NOT NULL,
    deleted TEXT,
    version BLOB NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_repayment_phase_status ON repayment_phase(status);
CREATE INDEX IF NOT EXISTS idx_repayment_phase_deleted ON repayment_phase(deleted);
