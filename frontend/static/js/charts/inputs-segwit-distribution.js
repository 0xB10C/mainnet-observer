const ANNOTATIONS = [annotationSegWitActivated, annotationBitcoinCoreSegWitWalletReleased, annotationBitcoinCore23, annotationTaprootActivated]
const MOVING_AVERAGE_DAYS = MOVING_AVERAGE_30D
const NAMES = ["nested P2WPKH", "nested P2WSH", "P2WPKH", "P2WSH", "P2TR key-path", "P2TR script-path"]
const PRECISION = 1
let START_DATE =  new Date("2017-08");

const CSVs = [
  fetchCSV("/csv/date.csv"),
  fetchCSV("/csv/inputs_p2wsh_sum.csv"),
  fetchCSV("/csv/inputs_p2wpkh_sum.csv"),
  fetchCSV("/csv/inputs_nested_p2wpkh_sum.csv"),
  fetchCSV("/csv/inputs_nested_p2wsh_sum.csv"),
  fetchCSV("/csv/inputs_p2tr_keypath_sum.csv"),
  fetchCSV("/csv/inputs_p2tr_scriptpath_sum.csv"),
]

function preprocess(input) {
  let data = { date: [], y1: [], y2: [], y3: [], y4: [], y5: [], y6: []}
  for (let i = 0; i < input[0].length; i++) {
    data.date.push(+(new Date(input[0][i].date)))
    const nativeSH = parseFloat(input[1][i].inputs_p2wsh_sum)
    const nativePKH = parseFloat(input[2][i].inputs_p2wpkh_sum)
    const nestedPKH = parseFloat(input[3][i].inputs_nested_p2wpkh_sum)
    const nestedSH = parseFloat(input[4][i].inputs_nested_p2wsh_sum)
    const p2trKeypath = parseFloat(input[5][i].inputs_p2tr_keypath_sum)
    const p2trScriptpath = parseFloat(input[6][i].inputs_p2tr_scriptpath_sum)

    const total = nativeSH + nativePKH + nestedPKH + nestedSH + p2trKeypath + p2trScriptpath

    const y1 = nestedPKH / total || 0
    const y2 = nestedSH / total || 0
    const y3 = nativePKH / total || 0
    const y4 = nativeSH / total || 0
    const y5 = p2trKeypath / total || 0
    const y6 = p2trScriptpath / total || 0
    
    data.y1.push(y1 * 100)
    data.y2.push(y2 * 100)
    data.y3.push(y3 * 100)
    data.y4.push(y4 * 100)
    data.y5.push(y5 * 100)
    data.y6.push(y6 * 100)
  }
  return data
}

function chartDefinition(d, movingAverage) {
  const DATA_KEYS = ["y1", "y2", "y3", "y4", "y5", "y6"]
  return stackedAreaPercentageChart(d, DATA_KEYS, NAMES, movingAverage, PRECISION, START_DATE, ANNOTATIONS);
}