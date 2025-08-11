const ANNOTATIONS = []
const MOVING_AVERAGE_DAYS = MOVING_AVERAGE_90D
const NAME = "fee rate distribution by bands"
const PRECISION = 1
let START_DATE = new Date();
START_DATE.setFullYear(new Date().getFullYear() - 5); // Show last 5 years for historical context

const CSVs = [
  fetchCSV("/csv/date.csv"),
  fetchCSV("/csv/zero_fee_tx_sum.csv"),
  fetchCSV("/csv/below_1_sat_vbyte_sum.csv"),
  fetchCSV("/csv/feerate_1_2_sat_vbyte_sum.csv"),
  fetchCSV("/csv/feerate_2_5_sat_vbyte_sum.csv"),
  fetchCSV("/csv/feerate_5_10_sat_vbyte_sum.csv"),
  fetchCSV("/csv/feerate_10_25_sat_vbyte_sum.csv"),
  fetchCSV("/csv/feerate_25_50_sat_vbyte_sum.csv"),
  fetchCSV("/csv/feerate_50_100_sat_vbyte_sum.csv"),
  fetchCSV("/csv/feerate_100_250_sat_vbyte_sum.csv"),
  fetchCSV("/csv/feerate_250_500_sat_vbyte_sum.csv"),
  fetchCSV("/csv/feerate_500_1000_sat_vbyte_sum.csv"),
  fetchCSV("/csv/feerate_1000_plus_sat_vbyte_sum.csv"),
]

function preprocess(input) {
  let data = { 
    date: [],
    zero_fee: [],
    below_1: [],
    fee_1_2: [],
    fee_2_5: [],
    fee_5_10: [],
    fee_10_25: [],
    fee_25_50: [],
    fee_50_100: [],
    fee_100_250: [],
    fee_250_500: [],
    fee_500_1000: [],
    fee_1000_plus: []
  }
  
  for (let i = 0; i < input[0].length; i++) {
    data.date.push(+(new Date(input[0][i].date)))
    
    // Get raw counts
    const zeroFee = parseFloat(input[1][i].zero_fee_tx_sum)
    const below1 = parseFloat(input[2][i].below_1_sat_vbyte_sum)
    const fee12 = parseFloat(input[3][i].feerate_1_2_sat_vbyte_sum)
    const fee25 = parseFloat(input[4][i].feerate_2_5_sat_vbyte_sum)
    const fee510 = parseFloat(input[5][i].feerate_5_10_sat_vbyte_sum)
    const fee1025 = parseFloat(input[6][i].feerate_10_25_sat_vbyte_sum)
    const fee2550 = parseFloat(input[7][i].feerate_25_50_sat_vbyte_sum)
    const fee50100 = parseFloat(input[8][i].feerate_50_100_sat_vbyte_sum)
    const fee100250 = parseFloat(input[9][i].feerate_100_250_sat_vbyte_sum)
    const fee250500 = parseFloat(input[10][i].feerate_250_500_sat_vbyte_sum)
    const fee5001000 = parseFloat(input[11][i].feerate_500_1000_sat_vbyte_sum)
    const fee1000plus = parseFloat(input[12][i].feerate_1000_plus_sat_vbyte_sum)
    
    // Calculate total transactions
    const total = zeroFee + below1 + fee12 + fee25 + fee510 + fee1025 + fee2550 + fee50100 + fee100250 + fee250500 + fee5001000 + fee1000plus
    
    // Convert to percentages (avoid division by zero)
    if (total > 0) {
      data.zero_fee.push((zeroFee / total) * 100)
      data.below_1.push((below1 / total) * 100)
      data.fee_1_2.push((fee12 / total) * 100)
      data.fee_2_5.push((fee25 / total) * 100)
      data.fee_5_10.push((fee510 / total) * 100)
      data.fee_10_25.push((fee1025 / total) * 100)
      data.fee_25_50.push((fee2550 / total) * 100)
      data.fee_50_100.push((fee50100 / total) * 100)
      data.fee_100_250.push((fee100250 / total) * 100)
      data.fee_250_500.push((fee250500 / total) * 100)
      data.fee_500_1000.push((fee5001000 / total) * 100)
      data.fee_1000_plus.push((fee1000plus / total) * 100)
    } else {
      // If no transactions, set all percentages to 0
      data.zero_fee.push(0)
      data.below_1.push(0)
      data.fee_1_2.push(0)
      data.fee_2_5.push(0)
      data.fee_5_10.push(0)
      data.fee_10_25.push(0)
      data.fee_25_50.push(0)
      data.fee_50_100.push(0)
      data.fee_100_250.push(0)
      data.fee_250_500.push(0)
      data.fee_500_1000.push(0)
      data.fee_1000_plus.push(0)
    }
  }
  return data
}

const DATA_KEYS = [
  'zero_fee',
  'below_1', 
  'fee_1_2',
  'fee_2_5',
  'fee_5_10',
  'fee_10_25',
  'fee_25_50',
  'fee_50_100',
  'fee_100_250',
  'fee_250_500',
  'fee_500_1000',
  'fee_1000_plus'
]

const NAMES = [
  '0 sat/vB',
  '< 1 sat/vB',
  '1-2 sat/vB',
  '2-5 sat/vB',
  '5-10 sat/vB',
  '10-25 sat/vB',
  '25-50 sat/vB',
  '50-100 sat/vB',
  '100-250 sat/vB',
  '250-500 sat/vB',
  '500-1000 sat/vB',
  '1000+ sat/vB'
]

function chartDefinition(d, movingAverage) {
  let option = stackedAreaPercentageChart(d, DATA_KEYS, NAMES, movingAverage, PRECISION, START_DATE, ANNOTATIONS);
  option.tooltip["order"] = "seriesDesc";

  // Reduce legend item spacing to prevent overlapping
  option.legend = {
    ...option.legend,
    width: '95%',
    itemGap: 2,
    textStyle: {
      fontSize: 9.5
    }
  };

  return option;
}