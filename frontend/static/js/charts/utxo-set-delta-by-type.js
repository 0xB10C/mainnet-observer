const ANNOTATIONS = []
const MOVING_AVERAGE_DAYS = MOVING_AVERAGE_7D
const NAMES = ["P2PK", "P2PKH", "P2WPKH", "P2MS", "P2SH", "P2WSH", "P2TR", "P2A", "Unknown"]
const PRECISION = 0
let START_DATE = new Date();
START_DATE.setFullYear(new Date().getFullYear() - 3);

const CSVs = [
  fetchCSV("/csv/date.csv"),
  // Input CSVs
  fetchCSV("/csv/inputs_p2pk_sum.csv"),
  fetchCSV("/csv/inputs_p2pkh_sum.csv"),
  fetchCSV("/csv/inputs_p2wpkh_sum.csv"),
  fetchCSV("/csv/inputs_p2ms_sum.csv"),
  fetchCSV("/csv/inputs_p2sh_sum.csv"),
  fetchCSV("/csv/inputs_p2wsh_sum.csv"),
  fetchCSV("/csv/inputs_p2tr_keypath_sum.csv"),
  fetchCSV("/csv/inputs_p2tr_scriptpath_sum.csv"),
  fetchCSV("/csv/inputs_p2a_sum.csv"),
  fetchCSV("/csv/inputs_unknown_sum.csv"),
  fetchCSV("/csv/inputs_nested_p2wpkh_sum.csv"),
  fetchCSV("/csv/inputs_nested_p2wsh_sum.csv"),
  // Output CSVs
  fetchCSV("/csv/outputs_p2pk_sum.csv"),
  fetchCSV("/csv/outputs_p2pkh_sum.csv"),
  fetchCSV("/csv/outputs_p2wpkh_sum.csv"),
  fetchCSV("/csv/outputs_p2ms_sum.csv"),
  fetchCSV("/csv/outputs_p2sh_sum.csv"),
  fetchCSV("/csv/outputs_p2wsh_sum.csv"),
  fetchCSV("/csv/outputs_p2tr_sum.csv"),
  fetchCSV("/csv/outputs_p2a_sum.csv"),
  fetchCSV("/csv/outputs_unknown_sum.csv"),
]

function preprocess(input) {
  let data = { 
    date: [],
    p2pk: [],
    p2pkh: [],
    p2wpkh: [],
    p2ms: [],
    p2sh: [],
    p2wsh: [],
    p2tr: [],
    p2a: [],
    unknown: []
  }
  
  // CSV indices for input data
  const [dates,
    inputs_p2pk, inputs_p2pkh, inputs_p2wpkh, inputs_p2ms, inputs_p2sh, inputs_p2wsh,
    inputs_p2tr_keypath, inputs_p2tr_scriptpath, inputs_p2a, inputs_unknown,
    inputs_nested_p2wpkh, inputs_nested_p2wsh,
    outputs_p2pk, outputs_p2pkh, outputs_p2wpkh, outputs_p2ms, outputs_p2sh, outputs_p2wsh,
    outputs_p2tr, outputs_p2a, outputs_unknown] = input;

  for (let i = 0; i < dates.length; i++) {
    data.date.push(+(new Date(dates[i].date)))

    // Calculate deltas: outputs - inputs for each script type
    // Use exact CSV column names as specified in the backend
    data.p2pk.push((parseFloat(outputs_p2pk[i]["outputs_p2pk_sum"]) || 0) - (parseFloat(inputs_p2pk[i]["inputs_p2pk_sum"]) || 0))
    data.p2pkh.push((parseFloat(outputs_p2pkh[i]["outputs_p2pkh_sum"]) || 0) - (parseFloat(inputs_p2pkh[i]["inputs_p2pkh_sum"]) || 0))
    data.p2wpkh.push((parseFloat(outputs_p2wpkh[i]["outputs_p2wpkh_sum"]) || 0) - (parseFloat(inputs_p2wpkh[i]["inputs_p2wpkh_sum"]) || 0))
    data.p2ms.push((parseFloat(outputs_p2ms[i]["outputs_p2ms_sum"]) || 0) - (parseFloat(inputs_p2ms[i]["inputs_p2ms_sum"]) || 0))

    // Special handling for P2SH: outputs_p2sh - (inputs_p2sh + inputs_nested_p2wpkh + inputs_nested_p2wsh)
    // This prevents "data inflation" from nested SegWit inputs
    const p2sh_inputs = (parseFloat(inputs_p2sh[i]["inputs_p2sh_sum"]) || 0) +
                       (parseFloat(inputs_nested_p2wpkh[i]["inputs_nested_p2wpkh_sum"]) || 0) +
                       (parseFloat(inputs_nested_p2wsh[i]["inputs_nested_p2wsh_sum"]) || 0);
    data.p2sh.push((parseFloat(outputs_p2sh[i]["outputs_p2sh_sum"]) || 0) - p2sh_inputs)

    data.p2wsh.push((parseFloat(outputs_p2wsh[i]["outputs_p2wsh_sum"]) || 0) - (parseFloat(inputs_p2wsh[i]["inputs_p2wsh_sum"]) || 0))

    // Special handling for P2TR: outputs_p2tr - (inputs_p2tr_keypath + inputs_p2tr_scriptpath)
    // Handle key-path vs script-path spending separately
    const p2tr_inputs = (parseFloat(inputs_p2tr_keypath[i]["inputs_p2tr_keypath_sum"]) || 0) +
                       (parseFloat(inputs_p2tr_scriptpath[i]["inputs_p2tr_scriptpath_sum"]) || 0);
    data.p2tr.push((parseFloat(outputs_p2tr[i]["outputs_p2tr_sum"]) || 0) - p2tr_inputs)
    
    data.p2a.push((parseFloat(outputs_p2a[i]["outputs_p2a_sum"]) || 0) - (parseFloat(inputs_p2a[i]["inputs_p2a_sum"]) || 0))
    data.unknown.push((parseFloat(outputs_unknown[i]["outputs_unknown_sum"]) || 0) - (parseFloat(inputs_unknown[i]["inputs_unknown_sum"]) || 0))
  }
  
  return data
}

const DATA_KEYS = ['p2pk', 'p2pkh', 'p2wpkh', 'p2ms', 'p2sh', 'p2wsh', 'p2tr', 'p2a', 'unknown']

function chartDefinition(d, movingAverage) {
  let option = multiLineChart(d, DATA_KEYS, NAMES, movingAverage, PRECISION, START_DATE, ANNOTATIONS);
  option.tooltip["order"] = "seriesAsc";
  option.tooltip["valueFormatter"] = (v) => formatWithSIPrefix(v, "");
  return option;
}