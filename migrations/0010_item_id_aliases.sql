CREATE TABLE IF NOT EXISTS item_id_aliases (
    entity_id UUID PRIMARY KEY,
    compat_id BIGSERIAL NOT NULL UNIQUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_item_id_aliases_compat_id ON item_id_aliases(compat_id);

INSERT INTO item_id_aliases (entity_id)
SELECT id
FROM media_items
ON CONFLICT(entity_id) DO NOTHING;

INSERT INTO item_id_aliases (entity_id)
SELECT id
FROM libraries
ON CONFLICT(entity_id) DO NOTHING;

CREATE OR REPLACE FUNCTION ensure_item_id_alias_on_insert()
RETURNS trigger AS $$
BEGIN
    INSERT INTO item_id_aliases (entity_id)
    VALUES (NEW.id)
    ON CONFLICT(entity_id) DO NOTHING;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION cleanup_item_id_alias_on_delete()
RETURNS trigger AS $$
BEGIN
    DELETE FROM item_id_aliases WHERE entity_id = OLD.id;
    RETURN OLD;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_media_items_item_id_alias_insert ON media_items;
CREATE TRIGGER trg_media_items_item_id_alias_insert
AFTER INSERT ON media_items
FOR EACH ROW
EXECUTE FUNCTION ensure_item_id_alias_on_insert();

DROP TRIGGER IF EXISTS trg_media_items_item_id_alias_delete ON media_items;
CREATE TRIGGER trg_media_items_item_id_alias_delete
AFTER DELETE ON media_items
FOR EACH ROW
EXECUTE FUNCTION cleanup_item_id_alias_on_delete();

DROP TRIGGER IF EXISTS trg_libraries_item_id_alias_insert ON libraries;
CREATE TRIGGER trg_libraries_item_id_alias_insert
AFTER INSERT ON libraries
FOR EACH ROW
EXECUTE FUNCTION ensure_item_id_alias_on_insert();

DROP TRIGGER IF EXISTS trg_libraries_item_id_alias_delete ON libraries;
CREATE TRIGGER trg_libraries_item_id_alias_delete
AFTER DELETE ON libraries
FOR EACH ROW
EXECUTE FUNCTION cleanup_item_id_alias_on_delete();
