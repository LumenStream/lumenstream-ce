-- Multi-root library support: move library root paths into dedicated table.

CREATE TABLE IF NOT EXISTS library_paths (
    id UUID PRIMARY KEY,
    library_id UUID NOT NULL REFERENCES libraries(id) ON DELETE CASCADE,
    path TEXT NOT NULL,
    normalized_path TEXT NOT NULL,
    sort_order INTEGER NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX IF NOT EXISTS uq_library_paths_library_normalized
    ON library_paths(library_id, normalized_path);
CREATE UNIQUE INDEX IF NOT EXISTS uq_library_paths_library_sort_order
    ON library_paths(library_id, sort_order);
CREATE INDEX IF NOT EXISTS idx_library_paths_library_sort
    ON library_paths(library_id, sort_order);

WITH migrated AS (
    SELECT
        l.id AS library_id,
        CASE
            WHEN length(btrim(l.root_path)) > 1
                THEN regexp_replace(btrim(l.root_path), '/+$', '')
            ELSE btrim(l.root_path)
        END AS canonical_path
    FROM libraries l
)
INSERT INTO library_paths (id, library_id, path, normalized_path, sort_order)
SELECT
    migrated.library_id,
    migrated.library_id,
    migrated.canonical_path,
    lower(migrated.canonical_path),
    0
FROM migrated
WHERE migrated.canonical_path <> ''
ON CONFLICT (id) DO NOTHING;

ALTER TABLE libraries
    DROP COLUMN IF EXISTS root_path;
