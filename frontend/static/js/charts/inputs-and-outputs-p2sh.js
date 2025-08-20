const ANNOTATIONS = []
const MOVING_AVERAGE_DAYS = MOVING_AVERAGE_7D
const NAMES = ["P2SH inputs", "P2SH, nested P2WPKH, nested P2WSH inputs", "P2SH outputs"]
const DATA_KEYS = ["y1", "y2", "y3"]
const PRECISION = 0
let START_DATE =  new Date();
START_DATE.setFullYear(new Date().getFullYear() - 3);

const CSVs = [
  fetchCSV("/csv/date.csv"),
  fetchCSV("/csv/inputs_p2sh_sum.csv"),
  fetchCSV("/csv/outputs_p2sh_sum.csv"),
  fetchCSV("/csv/inputs_nested_p2wpkh_sum.csv"),
  fetchCSV("/csv/inputs_nested_p2wsh_sum.csv"),
]

function preprocess(input) {
  let data = { date: [], y1: [], y2: [], y3: [] }
  for (let i = 0; i < input[0].length; i++) {
    data.date.push(+(new Date(input[0][i].date)))
    const p2sh_inputs = parseFloat(input[1][i].inputs_p2sh_sum)
    const p2sh_outputs = parseFloat(input[2][i].outputs_p2sh_sum)
    const nested_p2wpkh = parseFloat(input[3][i].inputs_nested_p2wpkh_sum)
    const nested_p2wsh = parseFloat(input[4][i].inputs_nested_p2wsh_sum)
    
    // P2SH inputs (excludes nested SegWit)
    const y1 = p2sh_inputs
    
    // P2SH inputs (includes nested SegWit) = P2SH inputs + nested SegWit inputs
    const y2 = p2sh_inputs + nested_p2wpkh + nested_p2wsh
    
    // P2SH outputs (unchanged)
    const y3 = p2sh_outputs
    
    data.y1.push(y1)
    data.y2.push(y2)
    data.y3.push(y3)
  }
  return data
}

function chartDefinition(d, movingAverage) {
  return multiLineChart(d, DATA_KEYS, NAMES, movingAverage, PRECISION, START_DATE, ANNOTATIONS);
}