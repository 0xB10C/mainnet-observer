ALTER TABLE block_stats ADD COLUMN coinbase_unclaimed_sat BIGINT NOT NULL DEFAULT (0);
