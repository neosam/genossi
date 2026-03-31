CREATE TABLE IF NOT EXISTS member_action (
    id BLOB PRIMARY KEY NOT NULL,
    member_id BLOB NOT NULL,
    action_type TEXT NOT NULL,
    date TEXT NOT NULL,
    shares_change INTEGER NOT NULL,
    transfer_member_id BLOB,
    effective_date TEXT,
    comment TEXT,
    created TEXT NOT NULL,
    deleted TEXT,
    version BLOB NOT NULL,
    FOREIGN KEY (member_id) REFERENCES member(id),
    FOREIGN KEY (transfer_member_id) REFERENCES member(id)
);

CREATE INDEX IF NOT EXISTS idx_member_action_member_id ON member_action(member_id);
CREATE INDEX IF NOT EXISTS idx_member_action_deleted ON member_action(deleted);
CREATE INDEX IF NOT EXISTS idx_member_action_date ON member_action(date);
