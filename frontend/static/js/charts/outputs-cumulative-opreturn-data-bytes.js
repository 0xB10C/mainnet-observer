const ANNOTATIONS = [annotationVeriBlock, annotationRunestones]
const MOVING_AVERAGE_DAYS = MOVING_AVERAGE_1D
const NAME = "OP_RETURN data bytes"
const PRECISION = 0
const START_DATE =  new Date("2014");
const UNIT = "B" // bytes

const CSVs = [
  fetchCSV("/csv/date.csv"),
  fetchCSV("/csv/outputs_opreturn_bytes_sum.csv"),
]

function preprocess(input) {
  let data = { date: [], y: [] }
  let cumulative = 0;
  for (let i = 0; i < input[0].length; i++) {
    data.date.push(+(new Date(input[0][i].date)))
    cumulative += parseFloat(input[1][i].outputs_opreturn_bytes_sum)
    data.y.push(cumulative)
  }
  return data
}

function chartDefinition(d, movingAverage) {
  const EXTRA = {
    tooltip: { trigger: 'axis', valueFormatter: (v) => formatWithSIPrefix(v, UNIT)},
    yAxis: { axisLabel: {formatter: (v) => formatWithSIPrefix(v, UNIT) } },
  }
  const option = lineChart(d, NAME, movingAverage, PRECISION, START_DATE, ANNOTATIONS);
  return {...option, ...EXTRA};
}
