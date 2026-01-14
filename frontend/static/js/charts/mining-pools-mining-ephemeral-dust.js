MOVING_AVERAGE_DAYS = 1

const CSVs = [
  fetchCSV("/csv/miningpools-mining-ephemeral-dust.csv"),
]

function preprocess(input) {
  let data = { header: [], rows: [] }
  data.header = Object.keys(input[0][0])
  for (let i = 0; i < input[0].length; i++) {
    let row = [];
    for (h of data.header) {
      row.push(input[0][i][h])
    }
    data.rows.push(row)
  }
  return data
}

function chartDefinition(data) {
  return table(data)
}
