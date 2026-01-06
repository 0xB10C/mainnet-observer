ALTER TABLE output_stats
  ADD COLUMN outputs_opreturn_bytes BIGINT NOT NULL DEFAULT (0);
