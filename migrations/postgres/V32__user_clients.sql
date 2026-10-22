CREATE TABLE user_clients
(
    user_id   VARCHAR NOT NULL
        CONSTRAINT user_clients_users_id_fk
            REFERENCES users
            ON UPDATE CASCADE ON DELETE CASCADE,
    client_id VARCHAR NOT NULL
        CONSTRAINT user_clients_clients_id_fk
            REFERENCES clients
            ON UPDATE CASCADE ON DELETE CASCADE,
    first_at  BIGINT  NOT NULL,
    last_at   BIGINT  NOT NULL,
    scope     VARCHAR,
    logins    BIGINT  NOT NULL DEFAULT 1,
    CONSTRAINT user_clients_pk
        PRIMARY KEY (user_id, client_id)
);

CREATE INDEX user_clients_last_at_index
    ON user_clients (last_at);
