-- Delivery channel of a bulk message job: `email` (default), `sms` or `whatsapp`
ALTER TABLE email_jobs
    ADD COLUMN channel VARCHAR NOT NULL DEFAULT 'email';
