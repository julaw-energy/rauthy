-- The channels that can verify a verifiable custom attribute, in order of preference,
-- like `sms,whatsapp_user`.
CREATE TABLE attr_verify_channels
(
    attr_name TEXT NOT NULL
        CONSTRAINT attr_verify_channels_pk
            PRIMARY KEY
        CONSTRAINT attr_verify_channels_attr_fk
            REFERENCES user_attr_config (name)
            ON UPDATE CASCADE ON DELETE CASCADE,
    channels  TEXT NOT NULL
) STRICT;
