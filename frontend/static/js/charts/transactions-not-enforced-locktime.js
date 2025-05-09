const ANNOTATIONS = []
const MOVING_AVERAGE_DAYS = MOVING_AVERAGE_7D
const NAME = "unenforced locktime"
const PRECISION = 2
let START_DATE = new Date("2014");


const CSVs = [
  fetchCSV("/csv/date.csv"),
  fetchCSV("/csv/tx_timelock_not_enforced_sum.csv"),
  fetchCSV("/csv/transactions_sum.csv"),
]

function preprocess(input) {
  let data = { date: [], y: [] }
  for (let i = 0; i < input[0].length; i++) {
    data.date.push(+(new Date(input[0][i].date)))
    const not_enforced = parseFloat(input[1][i].tx_timelock_not_enforced_sum)
    const all_tx = parseFloat(input[2][i].transactions_sum)
    const y = (not_enforced/all_tx)
    data.y.push(y * 100)
  }
  return data
}

function chartDefinition(d, movingAverage) {
  return areaPercentageChart(d, NAME, movingAverage, PRECISION, START_DATE, ANNOTATIONS);
}
