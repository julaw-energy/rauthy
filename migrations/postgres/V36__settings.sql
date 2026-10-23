-- Runtime settings that can be changed from the admin UI instead of the config file / env.
-- `key` is e.g. `smtp` or `channel.sms`, `value` a JSON document. Secrets inside of it are
-- encrypted with the active encryption key.
CREATE TABLE settings
(
    key     VARCHAR NOT NULL
        CONSTRAINT settings_pk
            PRIMARY KEY,
    value   VARCHAR NOT NULL,
    updated BIGINT  NOT NULL
);
