ALTER TABLE user_attr_config
    ADD regex VARCHAR;

ALTER TABLE user_attr_config
    ADD error_msg VARCHAR;

-- JSON array of strings, used by the `Enum` type
ALTER TABLE user_attr_config
    ADD options VARCHAR;
