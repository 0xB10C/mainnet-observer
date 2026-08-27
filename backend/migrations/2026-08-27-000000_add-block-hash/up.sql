-- The block hash. Existing rows get an empty placeholder; the stats_version
-- bump that comes with this column forces a resync that backfills it.
ALTER TABLE block_stats ADD COLUMN hash TEXT NOT NULL DEFAULT ('');
