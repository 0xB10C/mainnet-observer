const ANNOTATIONS = []
const MOVING_AVERAGE_DAYS = MOVING_AVERAGE_7D
const NAME = "spending newly created UTXOs"
const PRECISION = 2
let START_DATE =  new Date();
START_DATE.setFullYear(new Date().getFullYear() - 5);

const CSVs = [
  fetchCSV("/csv/date.csv"),
  fetchCSV("/csv/transactions_sum.csv"),
  fetchCSV("/csv/tx_spending_newly_created_utxos_sum.csv"),
]

function preprocess(input) {
  let data = { date: [], y: [] }
  for (let i = 0; i < input[0].length; i++) {
    data.date.push(+(new Date(input[0][i].date)))
    const y = parseFloat(input[2][i].tx_spending_newly_created_utxos_sum) / parseFloat(input[1][i].transactions_sum)
    data.y.push(y * 100)
  }
  return data
}

function chartDefinition(d, movingAverage) {
  return areaPercentageChart(d, NAME, movingAverage, PRECISION, START_DATE, ANNOTATIONS);
}
