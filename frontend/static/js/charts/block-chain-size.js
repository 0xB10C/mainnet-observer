const ANNOTATIONS = [annotationInscriptionsHype];
const MOVING_AVERAGE_DAYS = MOVING_AVERAGE_1D;
const NAME = "chain size";
const PRECISION = 0;
let START_DATE = new Date("2009-01-03");
const UNIT = "B";

const CSVs = [fetchCSV("/csv/date.csv"), fetchCSV("/csv/size_sum.csv")];

function preprocess(input) {
  let data = { date: [], y: [] };
  cumulative = 0;
  for (let i = 0; i < input[0].length; i++) {
    data.date.push(+new Date(input[0][i].date));
    cumulative += parseFloat(input[1][i].size_sum);
    data.y.push(cumulative);
  }
  return data;
}

function chartDefinition(d, movingAverage) {
  let option = lineChart(
    d,
    NAME,
    movingAverage,
    PRECISION,
    START_DATE,
    ANNOTATIONS,
  );
  option.tooltip["valueFormatter"] = (v) => formatWithSIPrefix(v, UNIT, false);
  option.yAxis.axisLabel = { formatter: (v) => formatWithSIPrefix(v, UNIT) };
  return option;
}
