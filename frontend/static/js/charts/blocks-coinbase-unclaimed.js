const MOVING_AVERAGE_DAYS = MOVING_AVERAGE_1D

const CSVs = [
  fetchCSV("/csv/unclaimed-coinbase-blocks.csv"),
]

function preprocess(input) {
  let data = { header: [], rows: [] }
  data.header = Object.keys(input[0][0])
  for (let i = 0; i < input[0].length; i++) {
    let row = [];
    for (h of data.header) {
      if (h == "unclaimed_sat") {
        let value = input[0][i][h]
        if (value > 100_000) {
          row.push(`${value / 100_000_000} BTC`)
        } else {
          row.push(`${value} sat`)
        }
      } else {
        row.push(input[0][i][h])
      }
    }
    data.rows.push(row)
  }
  return data
}

function chartDefinition(data) {
  return table(data)
}
