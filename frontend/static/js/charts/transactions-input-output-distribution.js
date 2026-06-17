const ANNOTATIONS = []
const MOVING_AVERAGE_DAYS = MOVING_AVERAGE_7D
const NAMES = [
  "1in-1out",
  "1in-2out",
  "1in-Nout",
  "Nin-1out",
  "others",
]
const COLORS = [
  "#33b1ff",
  "#1677d2",
  "#0f4fa8",
  "#6fdc8c",
  "#fa4d56",
]
const DATA_KEYS = [
  "tx_1_input_1_output_pct",
  "tx_1_input_2_output_pct",
  "tx_1_input_n_output_pct",
  "tx_n_input_1_output_pct",
  "tx_other_pct",
]
const PRECISION = 2
let START_DATE =  new Date();
START_DATE.setFullYear(new Date().getFullYear() - 3);

const CSVs = [
  fetchCSV("/csv/date.csv"),
  fetchCSV("/csv/tx_1_input_sum.csv"),
  fetchCSV("/csv/tx_1_output_sum.csv"),
  fetchCSV("/csv/tx_1_input_1_output_sum.csv"),
  fetchCSV("/csv/tx_1_input_2_output_sum.csv"),
  fetchCSV("/csv/transactions_sum.csv"),
]

function preprocess(input) {
  let data = {
    date: [],
    tx_1_input_1_output_pct: [],
    tx_1_input_n_output_pct: [],
    tx_1_input_2_output_pct: [],
    tx_n_input_1_output_pct: [],
    tx_other_pct: [],
  }

  for (let i = 0; i < input[0].length; i++) {
    const date = new Date(input[0][i].date)
    const total = parseFloat(input[5][i].transactions_sum)
    const oneInput = parseFloat(input[1][i].tx_1_input_sum)
    const oneOutput = parseFloat(input[2][i].tx_1_output_sum)
    const oneInputOneOutput = parseFloat(input[3][i].tx_1_input_1_output_sum)
    const oneInputTwoOutputs = parseFloat(input[4][i].tx_1_input_2_output_sum)

    const oneInputManyOutputs = oneInput - oneInputOneOutput - oneInputTwoOutputs
    const manyInputsOneOutput = oneOutput - oneInputOneOutput
    const others = total - oneInputOneOutput - oneInputManyOutputs - oneInputTwoOutputs - manyInputsOneOutput

    data.date.push(+(date))
    data.tx_1_input_1_output_pct.push((oneInputOneOutput / total) * 100)
    data.tx_1_input_n_output_pct.push((oneInputManyOutputs / total) * 100)
    data.tx_1_input_2_output_pct.push((oneInputTwoOutputs / total) * 100)
    data.tx_n_input_1_output_pct.push((manyInputsOneOutput / total) * 100)
    data.tx_other_pct.push((others / total) * 100)
  }

  return data
}

function chartDefinition(d, movingAverage) {
  const option = stackedAreaPercentageChart(d, DATA_KEYS, NAMES, movingAverage, PRECISION, START_DATE, ANNOTATIONS)
  option.color = COLORS
  option.legend = {
    ...option.legend,
    data: NAMES,
  }
  return option
}
