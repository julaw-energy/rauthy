-- User fields / attributes a client needs before a login may complete, e.g. `phone,city,attr:nin`.
CREATE TABLE client_required_fields
(
    client_id VARCHAR NOT NULL
        CONSTRAINT client_required_fields_pk
            PRIMARY KEY
        CONSTRAINT client_required_fields_clients_id_fk
            REFERENCES clients
            ON UPDATE CASCADE ON DELETE CASCADE,
    fields    VARCHAR NOT NULL
);
