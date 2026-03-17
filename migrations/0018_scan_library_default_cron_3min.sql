-- Tune scan_library default schedule to every 3 minutes.
-- Preserve user-customized cron expressions by only migrating the previous default value.

UPDATE task_definitions
SET cron_expr = '0 */3 * * * *',
    updated_at = now()
WHERE task_key = 'scan_library'
  AND cron_expr = '0 */30 * * * *';
