CREATE TABLE IF NOT EXISTS mail_job_static_attachments (
    mail_job_id BLOB NOT NULL REFERENCES mail_jobs(id),
    static_document_id BLOB NOT NULL REFERENCES static_documents(id),
    PRIMARY KEY (mail_job_id, static_document_id)
);
CREATE INDEX IF NOT EXISTS idx_mail_job_static_attachments_job
    ON mail_job_static_attachments(mail_job_id);
CREATE INDEX IF NOT EXISTS idx_mail_job_static_attachments_doc
    ON mail_job_static_attachments(static_document_id);
