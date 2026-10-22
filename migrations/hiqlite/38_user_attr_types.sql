ALTER TABLE user_attr_config
    ADD regex TEXT;

ALTER TABLE user_attr_config
    ADD error_msg TEXT;

-- JSON array of strings, used by the `Enum` type
ALTER TABLE user_attr_config
    ADD options TEXT;
