ALTER TABLE media_items
    ADD COLUMN IF NOT EXISTS sort_name TEXT;

UPDATE media_items
SET sort_name = NULLIF(BTRIM(metadata->>'sort_name'), '')
WHERE sort_name IS DISTINCT FROM NULLIF(BTRIM(metadata->>'sort_name'), '');

CREATE OR REPLACE FUNCTION media_items_sync_sort_name()
RETURNS TRIGGER AS
$$
BEGIN
    NEW.sort_name := NULLIF(BTRIM(NEW.metadata->>'sort_name'), '');
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_media_items_sync_sort_name ON media_items;

CREATE TRIGGER trg_media_items_sync_sort_name
BEFORE INSERT OR UPDATE OF metadata ON media_items
FOR EACH ROW
EXECUTE FUNCTION media_items_sync_sort_name();

CREATE INDEX IF NOT EXISTS idx_media_items_v0_library_type_name_created_sort_name
    ON media_items(
        library_id,
        item_type,
        name DESC,
        created_at DESC,
        (COALESCE(sort_name, name)) DESC
    )
    WHERE version_rank = 0;
