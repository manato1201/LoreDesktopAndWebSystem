-- Brings the pre-existing ad hoc `kv_store` table (previously created by a
-- one-off `CREATE TABLE IF NOT EXISTS` inside `db::connect`) under formal
-- `sqlx::migrate!()` management, so the migration history is complete from
-- the very first table onward. Schema is unchanged from what `db::connect`
-- already created — this is purely a bookkeeping move, not a schema change.
CREATE TABLE IF NOT EXISTS kv_store (key TEXT PRIMARY KEY, value TEXT NOT NULL);
