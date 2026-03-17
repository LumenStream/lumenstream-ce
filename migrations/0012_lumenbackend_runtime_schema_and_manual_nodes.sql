-- LumenBackend runtime schema registry + manual node onboarding hard switch.

CREATE TABLE IF NOT EXISTS lumenbackend_node_runtime_schemas (
    id UUID PRIMARY KEY,
    node_id TEXT NOT NULL REFERENCES lumenbackend_nodes(node_id) ON DELETE CASCADE,
    schema_version TEXT NOT NULL,
    schema_hash TEXT,
    schema JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(node_id, schema_version)
);

CREATE INDEX IF NOT EXISTS idx_lumenbackend_node_runtime_schemas_latest
    ON lumenbackend_node_runtime_schemas(node_id, updated_at DESC);

-- One-time migration policy: drop legacy node runtime payloads.
DELETE FROM lumenbackend_runtime_configs
WHERE scope = 'node';
