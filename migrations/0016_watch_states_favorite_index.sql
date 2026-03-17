CREATE INDEX IF NOT EXISTS idx_watch_states_user_favorite_media_item
    ON watch_states(user_id, is_favorite, media_item_id);
