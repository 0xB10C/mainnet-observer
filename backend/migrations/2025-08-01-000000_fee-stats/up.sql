ALTER TABLE feerate_stats
  ADD COLUMN zero_fee_tx INTEGER NOT NULL DEFAULT (0);

ALTER TABLE feerate_stats
  ADD COLUMN below_1_sat_vbyte INTEGER NOT NULL DEFAULT (0);
