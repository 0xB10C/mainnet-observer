ALTER TABLE input_stats
ADD COLUMN inputs_p2a INTEGER NOT NULL DEFAULT (0);

ALTER TABLE output_stats
ADD COLUMN outputs_p2a INTEGER NOT NULL DEFAULT (0);

ALTER TABLE output_stats
ADD COLUMN outputs_p2a_amount BIGINT NOT NULL DEFAULT (0);
