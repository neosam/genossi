-- Make email, street, house_number, postal_code, city nullable for admin-created applications
CREATE TABLE IF NOT EXISTS application_new (
    id BLOB PRIMARY KEY NOT NULL,
    first_name TEXT NOT NULL,
    last_name TEXT NOT NULL,
    salutation TEXT,
    email TEXT,
    street TEXT,
    house_number TEXT,
    postal_code TEXT,
    city TEXT,
    shares INTEGER NOT NULL,
    status TEXT NOT NULL DEFAULT 'Offen',
    created TEXT NOT NULL,
    deleted TEXT,
    version BLOB NOT NULL
);

INSERT INTO application_new SELECT * FROM application;

DROP TABLE application;

ALTER TABLE application_new RENAME TO application;

CREATE INDEX IF NOT EXISTS idx_application_status ON application(status);
CREATE INDEX IF NOT EXISTS idx_application_deleted ON application(deleted);
