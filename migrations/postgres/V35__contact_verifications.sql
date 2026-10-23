-- Verification state of a user's contact values (`phone`, and pending `email` codes).
-- `value` is what was verified / what a pending code was sent to, so changing the phone number
-- invalidates an older verification without any extra bookkeeping.
CREATE TABLE contact_verifications
(
    user_id     VARCHAR NOT NULL
        CONSTRAINT contact_verifications_users_id_fk
            REFERENCES users
            ON UPDATE CASCADE ON DELETE CASCADE,
    kind        VARCHAR NOT NULL,
    value       VARCHAR NOT NULL,
    verified_at BIGINT,
    code_hash   VARCHAR,
    code_exp    BIGINT,
    attempts    BIGINT  NOT NULL DEFAULT 0,
    last_sent   BIGINT  NOT NULL DEFAULT 0,
    CONSTRAINT contact_verifications_pk
        PRIMARY KEY (user_id, kind)
);
