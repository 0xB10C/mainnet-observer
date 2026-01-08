ALTER TABLE block_stats ADD COLUMN coinbase_locktime_set        BOOLEAN NOT NULL DEFAULT (FALSE);
ALTER TABLE block_stats ADD COLUMN coinbase_locktime_set_bip54  BOOLEAN NOT NULL DEFAULT (FALSE);
