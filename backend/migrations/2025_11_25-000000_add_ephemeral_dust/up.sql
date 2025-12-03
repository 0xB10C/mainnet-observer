ALTER TABLE tx_stats
ADD COLUMN tx_spending_ephemeral_dust INTEGER NOT NULL DEFAULT (0);
