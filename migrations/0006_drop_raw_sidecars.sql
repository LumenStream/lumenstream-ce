-- Drop unused raw_sidecars table.
-- Data was write-only; all useful fields already live in media_items.
DROP TABLE IF EXISTS raw_sidecars;
