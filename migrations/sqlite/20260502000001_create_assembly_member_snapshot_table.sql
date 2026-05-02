CREATE TABLE IF NOT EXISTS assembly_member_snapshot (
    assembly_id BLOB NOT NULL,
    member_id BLOB NOT NULL,
    captured_at TEXT NOT NULL,
    PRIMARY KEY (assembly_id, member_id),
    FOREIGN KEY (assembly_id) REFERENCES assembly(id),
    FOREIGN KEY (member_id) REFERENCES member(id)
);

CREATE INDEX IF NOT EXISTS idx_assembly_member_snapshot_assembly_id
    ON assembly_member_snapshot(assembly_id);
