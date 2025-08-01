
ALTER TABLE feerate_stats
  DROP COLUMN zero_fee_tx;

ALTER TABLE feerate_stats
  DROP COLUMN below_1_sat_vbyte;
