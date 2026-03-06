const ANNOTATIONS = [
  { text: "BIP91 / SegWit signaling (bit 4)", date: "2017-07-20" },
];
const MOVING_AVERAGE_DAYS = MOVING_AVERAGE_1D;
const NAME = "blocks signaling BIP110";
const PRECISION = 0;
let START_DATE = new Date("2025-12-03"); // BIP-110 was assigned

const CSVs = [fetchCSV("/csv/date.csv"), fetchCSV("/csv/bip110-signaling.csv")];

function preprocess(input) {
  let data = { date: [], y: [] };
  for (let i = 0; i < input[0].length; i++) {
    data.date.push(+new Date(input[0][i].date));
    data.y.push(parseFloat(input[1][i].bip110_signaling_count));
  }
  return data;
}

function chartDefinition(d, movingAverage) {
  return lineChart(d, NAME, movingAverage, PRECISION, START_DATE, ANNOTATIONS);
}
