CREATE TABLE IF NOT EXISTS application (
    id BLOB PRIMARY KEY NOT NULL,
    first_name TEXT NOT NULL,
    last_name TEXT NOT NULL,
    salutation TEXT,
    email TEXT NOT NULL,
    street TEXT NOT NULL,
    house_number TEXT NOT NULL,
    postal_code TEXT NOT NULL,
    city TEXT NOT NULL,
    shares INTEGER NOT NULL,
    status TEXT NOT NULL DEFAULT 'Offen',
    created TEXT NOT NULL,
    deleted TEXT,
    version BLOB NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_application_status ON application(status);
CREATE INDEX IF NOT EXISTS idx_application_deleted ON application(deleted);
