ALTER TABLE media_items
    ADD COLUMN IF NOT EXISTS version_group_id UUID,
    ADD COLUMN IF NOT EXISTS version_rank INT NOT NULL DEFAULT 0;

CREATE INDEX IF NOT EXISTS idx_media_items_version_group_id
    ON media_items(version_group_id);

CREATE INDEX IF NOT EXISTS idx_media_items_version_group_item
    ON media_items(version_group_id, item_type, id);
