-- What an audit entry needs to name a user who was deleted since: a few details, copied when the
-- user is deleted. Nothing references it, and `anonymized` rows have lost their personal data.
CREATE TABLE audit_users
(
    id                 VARCHAR NOT NULL
        CONSTRAINT audit_users_pk
            PRIMARY KEY,
    email              VARCHAR,
    given_name         VARCHAR,
    family_name        VARCHAR,
    preferred_username VARCHAR,
    phone              VARCHAR,
    deleted_at         BIGINT  NOT NULL,
    anonymized         BOOLEAN NOT NULL
);
