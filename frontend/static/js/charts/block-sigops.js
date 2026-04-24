const ANNOTATIONS = []
const MOVING_AVERAGE_DAYS = MOVING_AVERAGE_7D
const NAME = "sigops per block"
const PRECISION = 0
let START_DATE = new Date("2009");

const CSVs = [
  fetchCSV("/csv/date.csv"),
  fetchCSV("/csv/sigops_avg.csv"),
]

function preprocess(input) {
  let data = { date: [], y: [] }
  for (let i = 0; i < input[0].length; i++) {
    data.date.push(+(new Date(input[0][i].date)))
    data.y.push(parseFloat(input[1][i].sigops_avg))
  }
  return data
}

function chartDefinition(d, movingAverage) {
  return lineChart(d, NAME, movingAverage, PRECISION, START_DATE, ANNOTATIONS);
}
