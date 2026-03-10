const ANNOTATIONS = []
const MOVING_AVERAGE_DAYS = MOVING_AVERAGE_7D
const NAME = "inputs per transaction"
const PRECISION = 2
let START_DATE = new Date();
START_DATE.setFullYear(new Date().getFullYear() - 3);

const CSVs = [
  fetchCSV("/csv/date.csv"),
  fetchCSV("/csv/inputs_sum.csv"),
  fetchCSV("/csv/transactions_sum.csv"),
]

function preprocess(input) {
  let data = { date: [], y: [] }
  for (let i = 0; i < input[0].length; i++) {
    data.date.push(+(new Date(input[0][i].date)))
    const y = parseFloat(input[1][i].inputs_sum) / parseFloat(input[2][i].transactions_sum) || 0
    data.y.push(y)
  }
  return data
}

function chartDefinition(d, movingAverage) {
  return lineChart(d, NAME, movingAverage, PRECISION, START_DATE, ANNOTATIONS);
}
