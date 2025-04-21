const ANNOTATIONS = [annotationP2SHActivation, annotationSegWitActivated, annotationTaprootActivated]
const MOVING_AVERAGE_DAYS = 31
const NAMES = ["P2PK", "P2PKH", "P2WPKH", "P2MS", "P2SH", "P2WSH", "P2TR", "OP_RETURN", "P2A"]
const PRECISION = 1
let START_DATE =  new Date("2016");

const CSVs = [
  fetchCSV("/csv/date.csv"),
  fetchCSV("/csv/outputs_p2pk_amount_sum.csv"),
  fetchCSV("/csv/outputs_p2pkh_amount_sum.csv"),
  fetchCSV("/csv/outputs_p2wpkh_amount_sum.csv"),
  fetchCSV("/csv/outputs_p2ms_amount_sum.csv"),
  fetchCSV("/csv/outputs_p2sh_amount_sum.csv"),
  fetchCSV("/csv/outputs_p2wsh_amount_sum.csv"),
  fetchCSV("/csv/outputs_p2tr_amount_sum.csv"),
  fetchCSV("/csv/outputs_opreturn_amount_sum.csv"),
  fetchCSV("/csv/outputs_p2a_amount_sum.csv"),
]

function preprocess(input) {
  let data = { date: [], y1: [], y2: [], y3: [], y4: [], y5: [], y6: [], y7: [], y8: [], y9: [], y10: []}
  for (let i = 0; i < input[0].length; i++) {
    data.date.push(+(new Date(input[0][i].date)))
    const outs_P2PK = parseFloat(input[1][i].outputs_p2pk_amount_sum)
    const outs_P2PKH = parseFloat(input[2][i].outputs_p2pkh_amount_sum)
    const outs_P2WPKH = parseFloat(input[3][i].outputs_p2wpkh_amount_sum)
    const outs_P2MS = parseFloat(input[4][i].outputs_p2ms_amount_sum)
    const outs_P2SH = parseFloat(input[5][i].outputs_p2sh_amount_sum)
    const outs_P2WSH = parseFloat(input[6][i].outputs_p2wsh_amount_sum)
    const outs_P2TR = parseFloat(input[7][i].outputs_p2tr_amount_sum)
    const outs_OPRETURN = parseFloat(input[8][i].outputs_opreturn_amount_sum)
    const outs_P2A = parseFloat(input[9][i].outputs_p2a_amount_sum)
    
    const total = outs_P2PK + outs_P2PKH + outs_P2WPKH + outs_P2MS + outs_P2SH + outs_OPRETURN + outs_P2WSH + outs_P2TR + outs_P2A

    const outs_P2PK_percentage = outs_P2PK / total || 0
    const outs_P2PKH_percentage = outs_P2PKH / total || 0
    const outs_P2WPKH_percentage = outs_P2WPKH / total || 0
    const outs_P2MS_percentage = outs_P2MS / total || 0
    const outs_P2SH_percentage = outs_P2SH / total || 0
    const outs_P2WSH_percentage = outs_P2WSH / total || 0
    const outs_P2TR_percentage = outs_P2TR / total || 0
    const outs_OPRETURN_percentage = outs_OPRETURN / total || 0
    const outs_P2A_percentage = outs_P2A / total || 0
    
    data.y1.push(outs_P2PK_percentage * 100)
    data.y2.push(outs_P2PKH_percentage * 100)
    data.y3.push(outs_P2WPKH_percentage * 100)
    data.y4.push(outs_P2MS_percentage * 100)
    data.y5.push(outs_P2SH_percentage * 100)
    data.y6.push(outs_P2WSH_percentage * 100)
    data.y7.push(outs_P2TR_percentage * 100)
    data.y8.push(outs_OPRETURN_percentage * 100)
    data.y9.push(outs_P2A_percentage * 100)
  }
  return data
}

function chartDefinition(d) {
  const DATA_KEYS = ["y1", "y2", "y3", "y4", "y5", "y6", "y7", "y8", "y9"]
  return stackedAreaPercentageChart(d, DATA_KEYS, NAMES, MOVING_AVERAGE_DAYS, PRECISION, START_DATE, ANNOTATIONS);
}