const ANNOTATIONS = [annotationSegWitActivated]
const MOVING_AVERAGE_DAYS = MOVING_AVERAGE_7D
const NAME = "Outputs"
const PRECISION = 2
const START_DATE = new Date("2011")

const CSVs = [
  fetchCSV("/csv/date.csv"),
  fetchCSV("/csv/outputs_coinbase_sum.csv"),
  fetchCSV("/csv/block_count_sum.csv"),
]

function preprocess(input) {
  let data = { date: [], y: [] }
  for (let i = 0; i < input[0].length; i++) {
    data.date.push(+(new Date(input[0][i].date)))
    const y = input[1][i].outputs_coinbase_sum / input[2][i].block_count_sum
    data.y.push(y)
  }
  return data
}

function chartDefinition(d, movingAverage) {
  return lineChart(d, NAME, movingAverage, PRECISION, START_DATE, ANNOTATIONS);
}
