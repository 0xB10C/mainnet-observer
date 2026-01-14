ALTER TABLE output_stats
    DROP COLUMN outputs_coinbase;

ALTER TABLE output_stats
    DROP COLUMN outputs_coinbase_p2pk;

ALTER TABLE output_stats
    DROP COLUMN outputs_coinbase_p2pkh;

ALTER TABLE output_stats
    DROP COLUMN outputs_coinbase_p2wpkh;

ALTER TABLE output_stats
    DROP COLUMN outputs_coinbase_p2ms;

ALTER TABLE output_stats
    DROP COLUMN outputs_coinbase_p2sh;

ALTER TABLE output_stats
    DROP COLUMN outputs_coinbase_p2wsh;

ALTER TABLE output_stats
    DROP COLUMN outputs_coinbase_p2tr;

ALTER TABLE output_stats
    DROP COLUMN outputs_coinbase_opreturn;

ALTER TABLE output_stats
    DROP COLUMN outputs_coinbase_unknown;
