-- Rules a user must meet to log in to a client, like `country in UG,KE,TZ` or `age >= 18`.
-- `rules` is a JSON array, see `entity/client_access_rules.rs`.
CREATE TABLE client_access_rules
(
    client_id VARCHAR NOT NULL
        CONSTRAINT client_access_rules_pk
            PRIMARY KEY
        CONSTRAINT client_access_rules_client_fk
            REFERENCES clients (id)
            ON UPDATE CASCADE ON DELETE CASCADE,
    rules     VARCHAR NOT NULL
);

-- Which user got an inactivity reminder, when, and how many in a row.
CREATE TABLE inactivity_reminders
(
    user_id    VARCHAR NOT NULL
        CONSTRAINT inactivity_reminders_pk
            PRIMARY KEY
        CONSTRAINT inactivity_reminders_user_fk
            REFERENCES users (id)
            ON UPDATE CASCADE ON DELETE CASCADE,
    last_sent  BIGINT  NOT NULL,
    sent_count BIGINT  NOT NULL
);
