ALTER TABLE output_stats
    ADD COLUMN outputs_coinbase INTEGER NOT NULL DEFAULT (0);

ALTER TABLE output_stats
    ADD COLUMN outputs_coinbase_p2pk INTEGER NOT NULL DEFAULT (0);

ALTER TABLE output_stats
    ADD COLUMN outputs_coinbase_p2pkh INTEGER NOT NULL DEFAULT (0);

ALTER TABLE output_stats
    ADD COLUMN outputs_coinbase_p2wpkh INTEGER NOT NULL DEFAULT (0);

ALTER TABLE output_stats
    ADD COLUMN outputs_coinbase_p2ms INTEGER NOT NULL DEFAULT (0);

ALTER TABLE output_stats
    ADD COLUMN outputs_coinbase_p2sh INTEGER NOT NULL DEFAULT (0);

ALTER TABLE output_stats
    ADD COLUMN outputs_coinbase_p2wsh INTEGER NOT NULL DEFAULT (0);

ALTER TABLE output_stats
    ADD COLUMN outputs_coinbase_p2tr INTEGER NOT NULL DEFAULT (0);

ALTER TABLE output_stats
    ADD COLUMN outputs_coinbase_opreturn INTEGER NOT NULL DEFAULT (0);

ALTER TABLE output_stats
    ADD COLUMN outputs_coinbase_unknown INTEGER NOT NULL DEFAULT (0);
