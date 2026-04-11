CREATE INDEX IF NOT EXISTS idx_mail_recipients_member_id ON mail_recipients(member_id);
CREATE INDEX IF NOT EXISTS idx_inbound_mails_assigned_member_id ON inbound_mails(assigned_member_id);
