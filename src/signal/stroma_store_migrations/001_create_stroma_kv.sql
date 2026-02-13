-- Poll state key-value storage for Stroma
-- Used for ephemeral poll state (vote aggregates, HMAC'd voter dedup maps)
-- Per security-constraints.bead: MUST be zeroized when poll outcome determined
CREATE TABLE IF NOT EXISTS stroma_kv (
    key TEXT PRIMARY KEY NOT NULL,
    value BLOB NOT NULL,
    updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);
