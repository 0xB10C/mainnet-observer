const ANNOTATIONS = [
  { 'text': '1st halving', 'date': '2012-11-28' },
  { 'text': '2nd halving', 'date': '2016-07-09' },
  { 'text': '3rd halving', 'date': '2020-05-11' },
  { 'text': '4th halving', 'date': '2024-04-20' }
]
const MOVING_AVERAGE_DAYS = MOVING_AVERAGE_7D
const NAMES = ["block subsidy", "transaction fees"]
const PRECISION = 8
let START_DATE =  new Date("2009");
const UNIT = "BTC"

const CSVs = [
  fetchCSV("/csv/date.csv"),
  fetchCSV("/csv/coinbase_subsidy_avg.csv"),
  fetchCSV("/csv/coinbase_fees_avg.csv"),
]

function preprocess(input) {
  let data = { date: [], subsidy: [], fees: [] }
  for (let i = 0; i < input[0].length; i++) {
    data.date.push(+(new Date(input[0][i].date)))
    data.subsidy.push(parseFloat(input[1][i].coinbase_subsidy_avg) / 100_000_000)
    data.fees.push(parseFloat(input[2][i].coinbase_fees_avg) / 100_000_000)
  }
  return data
}

function chartDefinition(d, movingAverage) {
  const DATA_KEYS = ["subsidy", "fees"]
  const EXTRA = {
    tooltip: { trigger: 'axis', valueFormatter: (v) => formatWithSIPrefix(v, UNIT)},
    yAxis: { axisLabel: {formatter: (v) => formatWithSIPrefix(v, UNIT) } },
  }
  let option = stackedAreaChart(d, DATA_KEYS, NAMES, movingAverage, PRECISION, START_DATE, ANNOTATIONS);
  return {...option, ...EXTRA};
}
