const ANNOTATIONS = []
const MOVING_AVERAGE_DAYS = MOVING_AVERAGE_1D
const NAMES = ["inputs", "outputs"]
const PRECISION = 0
let START_DATE =  new Date();
START_DATE.setFullYear(2025);

const CSVs = [
  fetchCSV("/csv/date.csv"),
  fetchCSV("/csv/tx_spending_ephemeral_dust_sum.csv"),
  fetchCSV("/csv/outputs_p2a_sum.csv"),
]

function preprocess(input) {
  let data = { date: [], y: [] }
  for (let i = 0; i < input[0].length; i++) {
    data.date.push(+(new Date(input[0][i].date)))
    const y = parseFloat(input[1][i].tx_spending_ephemeral_dust_sum)
    data.y.push(y)
  }
  return data
}

function chartDefinition(d, movingAverage) {
  return lineChart(d, NAMES, movingAverage, PRECISION, START_DATE, ANNOTATIONS);
}
