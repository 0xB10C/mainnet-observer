CREATE TABLE block_stats (
	height                            BIGINT    PRIMARY KEY   NOT NULL,
	date                              DATE      NOT NULL,

	version                           INTEGER   NOT NULL,
	nonce                             INTEGER   NOT NULL,
	bits                              INTEGER   NOT NULL,

	size                              BIGINT    NOT NULL,
	stripped_size                     BIGINT    NOT NULL,
	vsize                             BIGINT    NOT NULL,
	weight                            BIGINT    NOT NULL,
	empty                             BOOLEAN   NOT NULL,

	coinbase_output_amount            BIGINT    NOT NULL,
	coinbase_weight                   BIGINT    NOT NULL,

	transactions                      INTEGER   NOT NULL,
	payments                          INTEGER   NOT NULL,
	payments_segwit_spending_tx       INTEGER   NOT NULL,
	payments_taproot_spending_tx      INTEGER   NOT NULL,
	payments_signaling_explicit_rbf   INTEGER   NOT NULL,

	inputs                            INTEGER   NOT NULL,
	outputs                           INTEGER   NOT NULL
);

CREATE TABLE tx_stats (
	height                            BIGINT    PRIMARY KEY   NOT NULL,
	date                              DATE      NOT NULL,

	tx_version_1                      INTEGER   NOT NULL,
	tx_version_2                      INTEGER   NOT NULL,
	tx_version_3                      INTEGER   NOT NULL,
	tx_version_unknown                INTEGER   NOT NULL,

	tx_output_amount                  BIGINT    NOT NULL,

	tx_spending_segwit                INTEGER   NOT NULL,
	tx_spending_only_segwit           INTEGER   NOT NULL,
	tx_spending_only_legacy           INTEGER   NOT NULL,
	tx_spending_only_taproot          INTEGER   NOT NULL,
	tx_spending_segwit_and_legacy     INTEGER   NOT NULL,
	tx_spending_nested_segwit         INTEGER   NOT NULL,
	tx_spending_native_segwit         INTEGER   NOT NULL,
	tx_spending_taproot               INTEGER   NOT NULL,

	tx_bip69_compliant                INTEGER   NOT NULL,
	tx_signaling_explicit_rbf         INTEGER   NOT NULL,

	tx_1_input                        INTEGER   NOT NULL,
	tx_1_output                       INTEGER   NOT NULL,
	tx_1_input_1_output               INTEGER   NOT NULL,
	tx_1_input_2_output               INTEGER   NOT NULL,
	tx_spending_newly_created_utxos   INTEGER   NOT NULL,

	tx_timelock_height                INTEGER   NOT NULL,
	tx_timelock_timestamp             INTEGER   NOT NULL,
	tx_timelock_not_enforced          INTEGER   NOT NULL,
	tx_timelock_too_high              INTEGER   NOT NULL
);


CREATE TABLE script_stats (
  	height                            BIGINT    PRIMARY KEY   NOT NULL,
  	date                              DATE      NOT NULL,

  	pubkeys                           INTEGER   NOT NULL,
	pubkeys_compressed                INTEGER   NOT NULL,
	pubkeys_uncompressed              INTEGER   NOT NULL,
	pubkeys_compressed_inputs         INTEGER   NOT NULL,
	pubkeys_uncompressed_inputs       INTEGER   NOT NULL,
	pubkeys_compressed_outputs        INTEGER   NOT NULL,
	pubkeys_uncompressed_outputs      INTEGER   NOT NULL,

	sigs_schnorr                      INTEGER   NOT NULL,
	sigs_ecdsa                        INTEGER   NOT NULL,
	sigs_ecdsa_not_strict_der         INTEGER   NOT NULL,
	sigs_ecdsa_strict_der             INTEGER   NOT NULL,

	sigs_ecdsa_length_less_70byte     INTEGER   NOT NULL,
	sigs_ecdsa_length_70byte          INTEGER   NOT NULL,
	sigs_ecdsa_length_71byte          INTEGER   NOT NULL,
	sigs_ecdsa_length_72byte          INTEGER   NOT NULL,
	sigs_ecdsa_length_73byte          INTEGER   NOT NULL,
	sigs_ecdsa_length_74byte          INTEGER   NOT NULL,
	sigs_ecdsa_length_75byte_or_more  INTEGER   NOT NULL,

	sigs_ecdsa_low_r                  INTEGER   NOT NULL,
	sigs_ecdsa_high_r                 INTEGER   NOT NULL,
	sigs_ecdsa_low_s                  INTEGER   NOT NULL,
	sigs_ecdsa_high_s                 INTEGER   NOT NULL,
	sigs_ecdsa_high_rs                INTEGER   NOT NULL,
	sigs_ecdsa_low_rs                 INTEGER   NOT NULL,
	sigs_ecdsa_low_r_high_s           INTEGER   NOT NULL,
	sigs_ecdsa_high_r_low_s           INTEGER   NOT NULL,

	sigs_sighashes                    INTEGER   NOT NULL,
	sigs_sighash_all                  INTEGER   NOT NULL,
	sigs_sighash_none                 INTEGER   NOT NULL,
	sigs_sighash_single               INTEGER   NOT NULL,
	sigs_sighash_all_acp              INTEGER   NOT NULL,
	sigs_sighash_none_acp             INTEGER   NOT NULL,
	sigs_sighash_single_acp           INTEGER   NOT NULL
);


CREATE TABLE input_stats (
  	height                            		BIGINT    PRIMARY KEY   NOT NULL,
  	date                              		DATE      NOT NULL,

	inputs_spending_legacy                  INTEGER   NOT NULL,
	inputs_spending_segwit                  INTEGER   NOT NULL,
	inputs_spending_taproot                 INTEGER   NOT NULL,
	inputs_spending_nested_segwit           INTEGER   NOT NULL,
	inputs_spending_native_segwit           INTEGER   NOT NULL,
	inputs_spending_multisig                INTEGER   NOT NULL,
	inputs_spending_p2ms_multisig           INTEGER   NOT NULL,
	inputs_spending_p2sh_multisig           INTEGER   NOT NULL,
	inputs_spending_nested_p2wsh_multisig   INTEGER   NOT NULL,
	inputs_spending_p2wsh_multisig          INTEGER   NOT NULL,

	inputs_p2pk                             INTEGER   NOT NULL,
	inputs_p2pkh                            INTEGER   NOT NULL,
	inputs_nested_p2wpkh                    INTEGER   NOT NULL,
	inputs_p2wpkh                           INTEGER   NOT NULL,
	inputs_p2ms                             INTEGER   NOT NULL,
	inputs_p2sh                             INTEGER   NOT NULL,
	inputs_nested_p2wsh                     INTEGER   NOT NULL,
	inputs_p2wsh                            INTEGER   NOT NULL,
	inputs_coinbase                         INTEGER   NOT NULL,
	inputs_witness_coinbase                 INTEGER   NOT NULL,
	inputs_p2tr_keypath                     INTEGER   NOT NULL,
	inputs_p2tr_scriptpath                  INTEGER   NOT NULL,
	inputs_unknown                          INTEGER   NOT NULL,

  	inputs_spend_in_same_block              INTEGER   NOT NULL
);


CREATE TABLE output_stats (
	height                            		BIGINT    PRIMARY KEY   NOT NULL,
	date                              		DATE      NOT NULL,

	outputs_p2pk                        	INTEGER   NOT NULL,
	outputs_p2pkh                       	INTEGER   NOT NULL,
	outputs_p2wpkh                      	INTEGER   NOT NULL,
	outputs_p2ms                        	INTEGER   NOT NULL,
	outputs_p2sh                        	INTEGER   NOT NULL,
	outputs_p2wsh                       	INTEGER   NOT NULL,
	outputs_opreturn                    	INTEGER   NOT NULL,
	outputs_p2tr                        	INTEGER   NOT NULL,
	outputs_unknown                     	INTEGER   NOT NULL,

	outputs_p2pk_amount                 	BIGINT    NOT NULL,
	outputs_p2pkh_amount                	BIGINT    NOT NULL,
	outputs_p2wpkh_amount               	BIGINT    NOT NULL,
	outputs_p2ms_amount                 	BIGINT    NOT NULL,
	outputs_p2sh_amount                 	BIGINT    NOT NULL,
	outputs_p2wsh_amount                	BIGINT    NOT NULL,
	outputs_p2tr_amount                 	BIGINT    NOT NULL,
	outputs_opreturn_amount             	BIGINT    NOT NULL,
	outputs_unknown_amount              	BIGINT    NOT NULL
);


CREATE TABLE  feerate_stats (
	height                            		BIGINT    PRIMARY KEY   NOT NULL,
	date                              		DATE      NOT NULL,

	fee_min                           		BIGINT    NOT NULL,
	fee_5th_percentile                		BIGINT    NOT NULL,
	fee_10th_percentile               		BIGINT    NOT NULL,
	fee_25th_percentile               		BIGINT    NOT NULL,
	fee_35th_percentile               		BIGINT    NOT NULL,
	fee_50th_percentile               		BIGINT    NOT NULL,
	fee_65th_percentile               		BIGINT    NOT NULL,
	fee_75th_percentile               		BIGINT    NOT NULL,
	fee_90th_percentile               		BIGINT    NOT NULL,
	fee_95th_percentile               		BIGINT    NOT NULL,
	fee_max                           		BIGINT    NOT NULL,
	fee_sum                           		BIGINT    NOT NULL,
	fee_avg                           		REAL      NOT NULL,

	size_min                          		INTEGER   NOT NULL,
	size_5th_percentile               		INTEGER   NOT NULL,
	size_10th_percentile              		INTEGER   NOT NULL,
	size_25th_percentile              		INTEGER   NOT NULL,
	size_35th_percentile              		INTEGER   NOT NULL,
	size_50th_percentile              		INTEGER   NOT NULL,
	size_65th_percentile              		INTEGER   NOT NULL,
	size_75th_percentile              		INTEGER   NOT NULL,
	size_90th_percentile              		INTEGER   NOT NULL,
	size_95th_percentile              		INTEGER   NOT NULL,
	size_max                          		INTEGER   NOT NULL,
	size_avg                          		REAL      NOT NULL,
	size_sum                          		BIGINT    NOT NULL,

	feerate_min                       		REAL      NOT NULL,
	feerate_5th_percentile            		REAL      NOT NULL,
	feerate_10th_percentile           		REAL      NOT NULL,
	feerate_25th_percentile           		REAL      NOT NULL,
	feerate_35th_percentile           		REAL      NOT NULL,
	feerate_50th_percentile           		REAL      NOT NULL,
	feerate_65th_percentile           		REAL      NOT NULL,
	feerate_75th_percentile           		REAL      NOT NULL,
	feerate_90th_percentile           		REAL      NOT NULL,
	feerate_95th_percentile           		REAL      NOT NULL,
	feerate_max                       		REAL      NOT NULL,
	feerate_avg                       		REAL      NOT NULL,

	feerate_package_min               		REAL      NOT NULL,
	feerate_package_5th_percentile    		REAL      NOT NULL,
	feerate_package_10th_percentile   		REAL      NOT NULL,
	feerate_package_25th_percentile   		REAL      NOT NULL,
	feerate_package_35th_percentile   		REAL      NOT NULL,
	feerate_package_50th_percentile   		REAL      NOT NULL,
	feerate_package_65th_percentile   		REAL      NOT NULL,
	feerate_package_75th_percentile   		REAL      NOT NULL,
	feerate_package_90th_percentile   		REAL      NOT NULL,
	feerate_package_95th_percentile   		REAL      NOT NULL,
	feerate_package_max               		REAL      NOT NULL,
	feerate_package_avg               		REAL      NOT NULL
);
