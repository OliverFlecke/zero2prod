CREATE TABLE users (
    user_id uuid PRIMARY KEY,
    username text NOT NULL UNIQUE,
    password_hash text NOT NULL
);
