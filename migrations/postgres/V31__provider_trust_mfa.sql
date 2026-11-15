ALTER TABLE auth_providers
    ADD trust_mfa bool DEFAULT false NOT NULL;
