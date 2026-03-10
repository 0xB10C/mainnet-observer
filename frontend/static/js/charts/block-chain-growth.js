const ANNOTATIONS = [
  annotationSegWitActivated,
  annotationTaprootActivated,
  annotationInscriptionsHype,
  annotationRunestones,
];
const MOVING_AVERAGE_DAYS = MOVING_AVERAGE_30D;
const NAME = "chain growth rate";
const PRECISION = 0;
let START_DATE = new Date();
START_DATE.setFullYear(new Date().getFullYear() - 8);
const UNIT = "B / day";

const CSVs = [fetchCSV("/csv/date.csv"), fetchCSV("/csv/size_sum.csv")];

function preprocess(input) {
  let data = { date: [], y: [] };
  for (let i = 0; i < input[0].length; i++) {
    data.date.push(+new Date(input[0][i].date));
    data.y.push(parseFloat(input[1][i].size_sum));
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

