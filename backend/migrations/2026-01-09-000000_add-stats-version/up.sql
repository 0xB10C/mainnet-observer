-- A version of the stats in the database. This is used to resync all stats
-- for blocks where the stats_version < wanted_version. The wanted_version
-- is defined in the backend.
ALTER TABLE block_stats ADD COLUMN stats_version INT NOT NULL DEFAULT (0);
