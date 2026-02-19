CREATE TABLE sync_state (
    id           INTEGER PRIMARY KEY DEFAULT 1,
    last_sync_at TIMESTAMP WITH TIME ZONE,
    CONSTRAINT single_row CHECK (id = 1)
);

INSERT INTO sync_state (id, last_sync_at) VALUES (1, NULL);
