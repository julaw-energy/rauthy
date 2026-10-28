-- What an audit entry needs to name a user who was deleted since: a few details, copied when the
-- user is deleted. Nothing references it, and `anonymized` rows have lost their personal data.
CREATE TABLE audit_users
(
    id                 TEXT    NOT NULL
        CONSTRAINT audit_users_pk
            PRIMARY KEY,
    email              TEXT,
    given_name         TEXT,
    family_name        TEXT,
    preferred_username TEXT,
    phone              TEXT,
    deleted_at         INTEGER NOT NULL,
    anonymized         INTEGER NOT NULL
) STRICT;
