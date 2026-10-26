-- Where an admin has read the events up to (a timestamp in milliseconds, like the events have). One
-- row per user, so the unread count of the bell is the same on every device.
CREATE TABLE event_read_state
(
    user_id    VARCHAR NOT NULL
        CONSTRAINT event_read_state_pk
            PRIMARY KEY
        CONSTRAINT event_read_state_user_fk
            REFERENCES users (id)
            ON UPDATE CASCADE ON DELETE CASCADE,
    read_until BIGINT  NOT NULL
);
