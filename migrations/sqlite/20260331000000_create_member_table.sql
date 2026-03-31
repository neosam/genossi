CREATE TABLE IF NOT EXISTS member (
    id BLOB PRIMARY KEY NOT NULL,
    member_number INTEGER NOT NULL,
    first_name TEXT NOT NULL,
    last_name TEXT NOT NULL,
    email TEXT,
    company TEXT,
    comment TEXT,
    street TEXT,
    house_number TEXT,
    postal_code TEXT,
    city TEXT,
    join_date TEXT NOT NULL,
    shares_at_joining INTEGER NOT NULL,
    current_shares INTEGER NOT NULL,
    current_balance INTEGER NOT NULL,
    exit_date TEXT,
    bank_account TEXT,
    created TEXT NOT NULL,
    deleted TEXT,
    version BLOB NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_member_member_number ON member(member_number);
CREATE INDEX IF NOT EXISTS idx_member_deleted ON member(deleted);
CREATE INDEX IF NOT EXISTS idx_member_last_name ON member(last_name);
