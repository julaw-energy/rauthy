-- The user that owns what the system does on its own (background tasks). The time is fixed, Hiqlite
-- refuses `unixepoch()` in writes. It cannot log in: it is
-- disabled, has no password and no passkey, and the server refuses every change to it.
INSERT INTO users (id, email, given_name, family_name, roles, groups, enabled, email_verified, created_at, language)
VALUES ('system', 'system@rauthy.invalid', 'System', 'Background tasks', '', NULL, 0, 1, 1793174400, 'en')
ON CONFLICT DO NOTHING;

-- Who did what. `actor` is the id of a user, `system` for the background tasks. Rows are only added,
-- never changed.
CREATE TABLE audit_log
(
    id          TEXT    NOT NULL
        CONSTRAINT audit_log_pk
            PRIMARY KEY,
    at          INTEGER NOT NULL,
    actor       TEXT    NOT NULL,
    action      TEXT    NOT NULL,
    target_type TEXT    NOT NULL,
    target_id   TEXT,
    details     TEXT
) STRICT;

-- the newest entries, and the history of one user, client, ...
CREATE INDEX audit_log_at_idx ON audit_log (at);
CREATE INDEX audit_log_target_idx ON audit_log (target_id, at);
