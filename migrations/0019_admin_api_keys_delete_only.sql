DELETE FROM admin_api_keys
WHERE is_active = false;

ALTER TABLE admin_api_keys
DROP COLUMN IF EXISTS is_active;
