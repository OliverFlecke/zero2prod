CREATE TYPE header_pair AS (
    name TEXT,
    value BYTEA
);

CREATE TABLE idempotency (
    user_id uuid NOT NULL REFERENCES users (user_id),
    idempotency_key text NOT NULL,
    response_status_code smallint,
    response_headers header_pair [],
    response_body bytea,
    created_at timestamptz NOT NULL,
    PRIMARY KEY (user_id, idempotency_key)
);
