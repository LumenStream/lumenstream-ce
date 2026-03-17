-- Add playlist_type to distinguish playlists from collections (BoxSets)
ALTER TABLE playlists ADD COLUMN IF NOT EXISTS playlist_type TEXT NOT NULL DEFAULT 'playlist';

CREATE INDEX IF NOT EXISTS idx_playlists_type ON playlists(playlist_type);

-- Auto-create item_id_aliases for playlists (so collections get compat numeric IDs)
INSERT INTO item_id_aliases (entity_id)
SELECT id FROM playlists
ON CONFLICT(entity_id) DO NOTHING;

DROP TRIGGER IF EXISTS trg_playlists_item_id_alias_insert ON playlists;
CREATE TRIGGER trg_playlists_item_id_alias_insert
AFTER INSERT ON playlists
FOR EACH ROW
EXECUTE FUNCTION ensure_item_id_alias_on_insert();

DROP TRIGGER IF EXISTS trg_playlists_item_id_alias_delete ON playlists;
CREATE TRIGGER trg_playlists_item_id_alias_delete
AFTER DELETE ON playlists
FOR EACH ROW
EXECUTE FUNCTION cleanup_item_id_alias_on_delete();
