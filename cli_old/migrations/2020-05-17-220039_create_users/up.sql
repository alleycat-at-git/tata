CREATE TABLE IF NOT EXISTS users (
    id INTEGER PRIMARY KEY NOT NULL,
    first_name VARCHAR(255) NOT NULL,
    last_name VARCHAR(255),
    peer_id VARCHAR(255) NOT NULL,
    secret BLOB NOT NULL
);