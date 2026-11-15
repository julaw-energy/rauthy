ALTER TABLE auth_providers
    ADD trust_mfa INTEGER DEFAULT 0 NOT NULL;
