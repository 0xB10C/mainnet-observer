const ANNOTATIONS = []
const MOVING_AVERAGE_DAYS = MOVING_AVERAGE_7D
const NAMES = ["GB of ECDSA Signatures", "GB of Schnorr Signatures"]
const PRECISION = 2
let START_DATE =  new Date("2009-01-01");

const CSVs = [
  fetchCSV("/csv/date.csv"),
  fetchCSV("/csv/sigs_ecdsa_length_70byte_sum.csv"),
  fetchCSV("/csv/sigs_ecdsa_length_71byte_sum.csv"),
  fetchCSV("/csv/sigs_ecdsa_length_72byte_sum.csv"),
  fetchCSV("/csv/sigs_ecdsa_length_73byte_sum.csv"),
  fetchCSV("/csv/sigs_ecdsa_length_74byte_sum.csv"),
  fetchCSV("/csv/sigs_ecdsa_length_75byte_or_more_sum.csv"),
  fetchCSV("l/csv/sigs_schnorr_sum.csv"),
]

function preprocess(input) {
  let data = { date: [], y1: [], y2: [] }
  let total_ecdsa = 0;
  let total_schnorr = 0;
  for (let i = 0; i < input[0].length; i++) {
    data.date.push(+(new Date(input[0][i].date)))
    const len70 = parseFloat(input[1][i].sigs_ecdsa_length_70byte_sum)
    const len71 = parseFloat(input[2][i].sigs_ecdsa_length_71byte_sum)
    const len72 = parseFloat(input[3][i].sigs_ecdsa_length_72byte_sum)
    const len73 = parseFloat(input[4][i].sigs_ecdsa_length_73byte_sum)
    const len74 = parseFloat(input[5][i].sigs_ecdsa_length_74byte_sum)
    const len75 = parseFloat(input[6][i].sigs_ecdsa_length_75byte_or_more_sum)
    total_ecdsa += ((len70*70) + (len71*71) + (len72*72) + (len73*73) + (len74*74) + (len75*75)) / 1000000000.0
    data.y1.push(total_ecdsa)
    total_schnorr += parseFloat(input[7][i].sigs_schnorr_sum * 64) / 1000000000.0
    data.y2.push(total_schnorr)
  }
  return data
}

function chartDefinition(d, movingAverage) {
  return doubleLineChart(d, NAMES, movingAverage, PRECISION, START_DATE, ANNOTATIONS);
}