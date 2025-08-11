ALTER TABLE feerate_stats
  ADD COLUMN feerate_1_2_sat_vbyte INTEGER NOT NULL DEFAULT (0);

ALTER TABLE feerate_stats
  ADD COLUMN feerate_2_5_sat_vbyte INTEGER NOT NULL DEFAULT (0);

ALTER TABLE feerate_stats
  ADD COLUMN feerate_5_10_sat_vbyte INTEGER NOT NULL DEFAULT (0);

ALTER TABLE feerate_stats
  ADD COLUMN feerate_10_25_sat_vbyte INTEGER NOT NULL DEFAULT (0);

ALTER TABLE feerate_stats
  ADD COLUMN feerate_25_50_sat_vbyte INTEGER NOT NULL DEFAULT (0);

ALTER TABLE feerate_stats
  ADD COLUMN feerate_50_100_sat_vbyte INTEGER NOT NULL DEFAULT (0);

ALTER TABLE feerate_stats
  ADD COLUMN feerate_100_250_sat_vbyte INTEGER NOT NULL DEFAULT (0);

ALTER TABLE feerate_stats
  ADD COLUMN feerate_250_500_sat_vbyte INTEGER NOT NULL DEFAULT (0);

ALTER TABLE feerate_stats
  ADD COLUMN feerate_500_1000_sat_vbyte INTEGER NOT NULL DEFAULT (0);

ALTER TABLE feerate_stats
  ADD COLUMN feerate_1000_plus_sat_vbyte INTEGER NOT NULL DEFAULT (0);
