const ANNOTATIONS = []
const MOVING_AVERAGE_DAYS = MOVING_AVERAGE_1D
const NAME = "unclaimed coins (BTC)"
const PRECISION = 8
let START_DATE = new Date("2009");

const CSVs = [
  fetchCSV("/csv/date.csv"),
  fetchCSV("/csv/coinbase_unclaimed_sat_sum.csv"),
]

function preprocess(input) {
  let data = { date: [], y: [] }
  let cumulative = 0;
  for (let i = 0; i < input[0].length; i++) {
    data.date.push(+(new Date(input[0][i].date)))
    const y = parseFloat(input[1][i].coinbase_unclaimed_sat_sum)
    cumulative += y;
    data.y.push(cumulative / 100_000_000)
  }
  return data
}

function chartDefinition(d, movingAverage) {
  return lineChart(d, NAME, movingAverage, PRECISION, START_DATE, ANNOTATIONS);
}
