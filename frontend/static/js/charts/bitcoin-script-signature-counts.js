const ANNOTATIONS = []
const MOVING_AVERAGE_DAYS = MOVING_AVERAGE_7D
const NAMES = ["ECDSA", "Schnorr"]
const PRECISION = 0
let START_DATE =  new Date();
START_DATE.setFullYear(new Date().getFullYear() - 3);

const CSVs = [
  fetchCSV("/csv/date.csv"),
  fetchCSV("/csv/sigs_ecdsa_sum.csv"),
  fetchCSV("/csv/sigs_schnorr_sum.csv"),
]

function preprocess(input) {
  let data = { date: [], y1: [], y2: [] }
  for (let i = 0; i < input[0].length; i++) {
    data.date.push(+(new Date(input[0][i].date)))
    const y1 = parseFloat(input[1][i].sigs_ecdsa_sum)
    const y2 = parseFloat(input[2][i].sigs_schnorr_sum)
    data.y1.push(y1)
    data.y2.push(y2)
  }
  return data
}

function chartDefinition(d, movingAverage) {
  return doubleLineChart(d, NAMES, movingAverage, PRECISION, START_DATE, ANNOTATIONS);
}