/* Shared code to load data and render charts */

const movingAverageSelect = document.getElementById('maSelector')

var chart
var preProcessedData
var currentMovingAverage = 0
var showAnnotations = true

const annotationBitcoinQTv0_6 = {'text': 'Bitcoin-QT v0.6 release', 'date': '2012-03-30'} // https://bitcoin.org/en/release/v0.6.0
const annotationBitcoinQTv0_7 = {'text': 'Bitcoin-QT v0.7 release', 'date': '2012-09-17'} // https://bitcoin.org/en/release/v0.7.0
const annotationBitcoinQTv0_8 = {'text': 'Bitcoin-QT v0.8 release', 'date': '2013-02-19'}
const annotationBitcoinCorev0_9 = {'text': 'Bitcoin Core v0.9.0 release', 'date': '2014-03-19'}
const annotationBitcoinCorev0_10 = {'text': 'Bitcoin Core v0.10.0 release', 'date': '2015-02-16'}
const annotationBIP66Activated = {'text': 'BIP66 Activation', 'date': '2015-07-04'}
const annotationBitcoinCorev0_11_1 = {'text': 'Bitcoin Core v0.11.1 release', 'date': '2015-10-15'}
const annotationBitcoinCoreSegWitWalletReleased = {'text': 'Bitcoin Core SegWit wallet released', 'date': '2018-02-26'}
const annotationSegWitActivated = {'text': 'SegWit Activation', 'date': '2017-08-24'}
const annotationBitcoinCorev0_17 = {'text': 'Bitcoin Core v0.17.0 release', 'date': '2018-10-03'}
const annotationBitcoinCorev0_19 = {'text': 'Bitcoin Core v0.19.0.1 release', 'date': '2019-11-24'}
const annotationBlockchainComSegwit = {'text': 'Blockchain.com wallet supports SegWit ', 'date': '2021-06-02'}
const annotationTaprootLockedIn = {'text': 'Taproot soft-fork locked-in', 'date': '2021-06-12'}
const annotationTaprootActivated = {'text': 'Taproot soft-fork activated', 'date': '2021-11-14'}
const annotationBitcoinCore23 = {'text': 'Bitcoin Core v23 released', 'date': '2022-04-25'}
const annotationBitcoinCore28 = {'text': 'Bitcoin Core v28 released (P2A support)', 'date': '2024-10-02'}
const annotationChinaMiningBan = {'text': 'China mining ban', 'date': '2021-05-21'}
const annotationASICsAvaliable = {'text': 'First ASIC miners avaliable', 'date': '2013-08-01'}
const annotationGPUMinerAvaliable = {'text': 'First GPU miners avaliable', 'date': '2010-07-01'}
const annotationP2SHActivation = {'text': 'P2SH Activation', 'date': '2012-04-01'}
const annotationInscriptionsHype = {'text': 'Inscriptions hype', 'date': '2023-04-15'}
const annotationRunestones = {'text': 'Runestones launch', 'date': '2024-04-21'}
const annotationVeriBlock = {'text': 'VeriBlock launch', 'date': '2018-09-14'}

const MOVING_AVERAGE_1D = 1
const MOVING_AVERAGE_7D = 7
const MOVING_AVERAGE_30D = 30
const MOVING_AVERAGE_90D = 90

const zip = (a, b) => a.map((k, i) => [k, b[i]]);

const watermark = (text) => { return { type: 'text', left: 'center', top: 'center', style: { text: text, font: 'bold 48px sans-serif', fill: 'rgba(0, 0, 0, 0.1)' }, z: 100, silent: true } };
const thumbnailTool = {
  show: isDevBuild,
  title: "save Thumbnail",
  icon: "path://M432.45,595.444c0,2.177-4.661,6.82-11.305,6.82c-6.475,0-11.306-4.567-11.306-6.82s4.852-6.812,11.306-6.812C427.841,588.632,432.452,593.191,432.45,595.444L432.45,595.444z M421.155,589.876c-3.009,0-5.448,2.495-5.448,5.572s2.439,5.572,5.448,5.572c3.01,0,5.449-2.495,5.449-5.572C426.604,592.371,424.165,589.876,421.155,589.876L421.155,589.876z M421.146,591.891c-1.916,0-3.47,1.589-3.47,3.549c0,1.959,1.554,3.548,3.47,3.548s3.469-1.589,3.469-3.548C424.614,593.479,423.062,591.891,421.146,591.891L421.146,591.891zM421.146,591.891",
  onclick: saveThumbnail,
}
const annotationToggleTool = {
  show: true,
  title: "toggle annotations",
  icon: 'path://M16,6m-2,0a2,2 0 1,0 4,0a2,2 0 1,0 -4,0M16,12m-2,0a2,2 0 1,0 4,0a2,2 0 1,0 -4,0M16,18m-2,0a2,2 0 1,0 4,0a2,2 0 1,0 -4,0M16,24m-2,0a2,2 0 1,0 4,0a2,2 0 1,0 -4,0M16,30m-2,0a2,2 0 1,0 4,0a2,2 0 1,0 -4,0',
  onclick: function (){
    showAnnotations = !showAnnotations;
    let option = chartDefinition(processedData, currentMovingAverage)
    chart.setOption(option);
  }
}
const toolbox = { show: true, feature: { myThumbnailTool: thumbnailTool, myAnnotationTool: annotationToggleTool, dataZoom: { yAxisIndex: 'none' }, restore: {}, saveAsImage: { name: chartPNGFileName }, dataView: {}}};
const tooltipAxisPointer = {type: "cross"}
const tooltipPosition = function (pos, _params, _el, _elRect, size) {
  var obj = { };
  obj[['left', 'right'][+(pos[0] < size.viewSize[0] / 2)]] = 100;
  obj[['top', 'bottom'][+(pos[1] < size.viewSize[1] / 2)]] = 100;
  return obj;
}

const BASE_CHART_OPTION = (START_DATE) => {
  return {
    backgroundColor: 'white',
    grid: {
      left: 0,
      right: 0,
      containLabel: true,
    },
    graphic: watermark(watermarkText),
    legend: { },
    animation: false,
    toolbox: toolbox,
    tooltip: { trigger: 'axis' },
    dataZoom: [ { type: 'inside', startValue: START_DATE.toISOString().slice(0, 10) }, { type: 'slider', brushSelect: false, showDetails: false, handleSize: 30 }],
  }
}

async function fetchCSV(url) {
  const res = await fetch(url);
  const text = await res.text();
  return parseCSV(text);
}

function parseCSV(text) {
  const lines = text.trim().split('\n');
  const headers = lines[0].split(',').map(h => h.trim());
  return lines.slice(1).map(line => {
    const values = line.split(',').map(v => v.trim());
    return Object.fromEntries(headers.map((h, i) => [h, values[i]]));
  });
}

function calcMovingAverage(data, windowSize, precision = 0) {
  const result = [];
  for (let i = 0; i < data.length; i++) {
    if (i < windowSize - 1) {
      result.push(null); // not enough data yet
      continue;
    }
    const slice = data.slice(i - windowSize + 1, i + 1);
    const sum = slice.reduce((a, b) => a + b, 0);
    result.push((sum / windowSize).toFixed(precision));
  }
  return result;
}

async function draw(option) {
  chart = echarts.init(document.getElementById("chart"), 'kelly');
  chart.setOption(option);
}

function formatWithSIPrefix(value, unit="", long = false) {
  const si = [
    { value: 1e30, symbol: "Q", name: "quetta" },
    { value: 1e27, symbol: "R", name: "ronna" },
    { value: 1e24, symbol: "Y", name: "yotta" },
    { value: 1e21, symbol: "Z", name: "zetta" },
    { value: 1e18, symbol: "E", name: "exa" },
    { value: 1e15, symbol: "P", name: "peta" },
    { value: 1e12, symbol: "T", name: "tera" },
    { value: 1e9,  symbol: "G", name: "giga" },
    { value: 1e6,  symbol: "M", name: "mega" },
    { value: 1e3,  symbol: "k", name: "kilo" },
  ];
  for (let i = 0; i < si.length; i++) {
    if (Math.abs(value) >= si[i].value) {
      return (value / si[i].value).toFixed(2).replace(/\.00$/, '').replace(/0$/, '') + " " + (long ? si[i].name : si[i].symbol) + unit;
    }
  }
  return `${value} ${unit}`
}

function formatPercentage(v) {
  return v + "%";
}

function formatCount(value) {
  const abs = Math.abs(value);
  if (abs >= 1e12) return (value / 1e12).toFixed(1).replace(/\.0$/, '') + 'T';
  if (abs >= 1e9) return (value / 1e9).toFixed(1).replace(/\.0$/, '') + 'B';
  if (abs >= 1e6) return (value / 1e6).toFixed(1).replace(/\.0$/, '') + 'M';
  if (abs >= 1e3) return (value / 1e3).toFixed(1).replace(/\.0$/, '') + 'K';
  return value.toString();
}

function saveThumbnail() {
  draw({
    grid: {
      top: 0,
      bottom: 0,
      left: 0,
      right: 0,
      containLabel: false
    },
    xAxis: { show: false },
    yAxis: { show: false },
    tooltip: { show: false },
    legend: { show: false },
    toolbox: { show: false },
    dataZoom: [{show: false}, {show: false}],
    graphic: {show: false},
    animation: false,
    animationDuration: 0,
    series: chart.getOption().series.map(s => {
      s.markLine = { label: { show: false }, lineStyle: {width: 0}};
      return s
    })
  });
  const canvas = chart.getRenderedCanvas({
    // If the chart had a background color, use it. Otherwise, use white.
    backgroundColor: chart.getOption().backgroundColor ? chart.getOption().backgroundColor : 'white',
    pixelRatio: 1
  });

  const link = document.createElement('a');
  link.href = canvas.toDataURL('image/png');
  link.download = chartPNGFileName;
  link.click();
}

// Single line (area) chart
// expects date and y
function lineChart(d, NAME, movingAverage, PRECISION, START_DATE, ANNOTATIONS = []) {
  y = zip(d.date, calcMovingAverage(d.y, movingAverage, PRECISION))
  return {
    ...BASE_CHART_OPTION(START_DATE),
    xAxis: { type: "time", data: d.date },
    yAxis: { type: 'value' },
    series: [
      { name: NAME, smooth: true, type: 'line', areaStyle: {}, data: y, symbol: "none", barCategoryGap: '0%', barGap: '0%', barWidth: '100%', itemStyle: { borderWidth: 0 } },
      // Annotations:
      { type: "line", markLine: { symbol: "none", label:{show: showAnnotations, position:"insideEndTop"}, lineStyle: { color: showAnnotations ? "gray": "transparent", type: "dotted" }, data: ANNOTATIONS.map(a => { return { xAxis: a.date, label: { formatter: a.text }} } ) } }
    ]
  }
}

// Area chart showing a precentage
// expects date and y (between 0 and 100)
function areaPercentageChart(d, NAME, movingAverage, PRECISION, START_DATE, ANNOTATIONS = []) {
  y = zip(d.date, calcMovingAverage(d.y, movingAverage, PRECISION))
  return {
    ...BASE_CHART_OPTION(START_DATE),
    xAxis: { type: "time", data: d.date },
    yAxis: { type: 'value', min: 0, max: 100, axisLabel: { formatter: formatPercentage } },
    tooltip: { trigger: 'axis', valueFormatter: formatPercentage},
    series: [
      { name: NAME, smooth: true, type: 'line', areaStyle: {}, data: y, symbol: "none", barCategoryGap: '0%', barGap: '0%', barWidth: '100%', itemStyle: { borderWidth: 0 } },
      // Annotations:
      { type: "line", markLine: { symbol: "none", label:{show: showAnnotations, position:"insideEndTop"}, lineStyle: { color: showAnnotations ? "gray": "transparent", type: "dotted" }, data: ANNOTATIONS.map(a => { return { xAxis: a.date, label: { formatter: a.text }} } ) } }
    ]
  }
}

// this is not a chart, but a table
function table(data) {
  const table = document.createElement('table');
  table.classList.add("table")
  const thead = document.createElement('thead');
  const trHead = document.createElement('tr');

  data.header.forEach(h => {
    const th = document.createElement('th');
    th.textContent = h;
    trHead.appendChild(th);
  });
  thead.appendChild(trHead);
  table.appendChild(thead);

  const tbody = document.createElement('tbody');
  data.rows.forEach(row => {
    const tr = document.createElement('tr');
    Object.values(row).forEach(val => {
      const td = document.createElement('td');
      td.textContent = val;
      tr.appendChild(td);
    });
    tbody.appendChild(tr);
  });
  table.appendChild(tbody);

  const container = document.getElementById('chart');
  container.appendChild(table);

  // HACK: drop the movingAverage selector, as we don't need it for tables
  document.getElementById('maSelector').parentNode.remove();

  // HACK: Don't use a pre-defined height for tables
  document.getElementById('chart').style.height = null;

  // inidcates to the caller that this isn't a chart
  return null;
}

// double line chart
// expects date, y1 and y2
function doubleLineChart(d, NAMES, movingAverage, PRECISION, START_DATE, ANNOTATIONS = []) {
  y1 = zip(d.date, calcMovingAverage(d.y1, movingAverage, PRECISION))
  y2 = zip(d.date, calcMovingAverage(d.y2, movingAverage, PRECISION))
  return {
    ...BASE_CHART_OPTION(START_DATE),
    xAxis: { type: "time", data: d.date },
    yAxis: { type: 'value' },
    series: [
      { name: NAMES[0], smooth: false, type: 'line', data: y1, symbol: "none"},
      { name: NAMES[1], smooth: false, type: 'line', data: y2, symbol: "none"},
      // Annotations:
      { type: "line", markLine: { symbol: "none", label:{show: showAnnotations, position:"insideEndTop"}, lineStyle: { color: showAnnotations ? "gray": "transparent", type: "dotted" }, data: ANNOTATIONS.map(a => { return { xAxis: a.date, label: { formatter: a.text }} } ) } }
    ]
  }
}

// multi line chart
// expects date, and multiple DATA_KEYS entries
function multiLineChart(d, DATA_KEYS, NAMES, movingAverage, PRECISION, START_DATE, ANNOTATIONS = []) {
  if (DATA_KEYS.length != NAMES.length) {
    alert("DATA_KEYS length does not match NAMES length!");
    return
  }
  return {
    ...BASE_CHART_OPTION(START_DATE),
    tooltip: { trigger: 'axis', axisPointer: tooltipAxisPointer, position: tooltipPosition},
    xAxis: { type: "time" },
    yAxis: { type: 'value' },
    series: zip(NAMES, DATA_KEYS).map(([name, key]) => {
      return {
        name: name,
        smooth: false,
        type: 'line',
        data: zip(d.date, calcMovingAverage(d[key], movingAverage, PRECISION)),
        symbol: "none",
        lineStyle: { width: 2 },
        emphasis: { focus: 'series' }
      }
    }).concat(
    [
      // Annotations:
      { type: "line", markLine: { symbol: "none", label:{show: showAnnotations, position:"insideEndTop", backgroundColor: "transparent"}, lineStyle: { color: showAnnotations ? "gray": "transparent", type: "dotted" }, data: ANNOTATIONS.map(a => { return { xAxis: a.date, label: { formatter: a.text }} } ) } }
    ]),
  }
}

// stacked area chart with values from 0 to 100
// expects date, and multiple DATA_KEYS entries
function stackedAreaPercentageChart(d, DATA_KEYS, NAMES, movingAverage, PRECISION, START_DATE, ANNOTATIONS = []) {
  if (DATA_KEYS.length != NAMES.length) {
    alert("DATA_KEYS length does not match NAMES length!");
    return
  }
  return {
    ...BASE_CHART_OPTION(START_DATE),
    tooltip: { trigger: 'axis', valueFormatter: formatPercentage, axisPointer: tooltipAxisPointer, position: tooltipPosition},
    xAxis: { type: "time", data: d.date },
    yAxis: { type: 'value', min: 0, max: 100, axisLabel: { formatter: formatPercentage } },
    series: zip(NAMES, DATA_KEYS).map(([name, key]) => {
      return { name: name, smooth: true, areaStyle: {}, lineStyle: {width: 0}, stack: "Total", type: 'line', data: zip(d.date, calcMovingAverage(d[key], movingAverage, PRECISION)), symbol: "none"}
    }).concat(
    [
      // Annotations:
      { type: "line", markLine: { symbol: "none", label:{show: showAnnotations, position:"insideEndTop", backgroundColor: "transparent"}, lineStyle: { color: showAnnotations ? "gray": "transparent", type: "dotted" }, data: ANNOTATIONS.map(a => { return { xAxis: a.date, label: { formatter: a.text }} } ) } }
    ]),
  }
}

// stacked area chart
// expects date, and multiple DATA_KEYS entries
function stackedAreaChart(d, DATA_KEYS, NAMES, movingAverage, PRECISION, START_DATE, ANNOTATIONS = []) {
  if (DATA_KEYS.length != NAMES.length) {
    alert("DATA_KEYS length does not match NAMES length!");
    return
  }
  return {
    ...BASE_CHART_OPTION(START_DATE),
    tooltip: { trigger: 'axis' },
    xAxis: { type: "time", data: d.date },
    yAxis: { type: 'value', axisLabel: { formatter: (v) => formatWithSIPrefix(v, "") } },
    series: zip(NAMES, DATA_KEYS).map(([name, key]) => {
      return { name: name, smooth: true, areaStyle: {}, lineStyle: {width: 0}, stack: "Total", type: 'line', data: zip(d.date, calcMovingAverage(d[key], movingAverage, PRECISION)), symbol: "none"}
    }).concat(
    [
      // Annotations:
      { type: "line", markLine: { symbol: "none", label:{show: showAnnotations, position:"insideEndTop", backgroundColor: "transparent"}, lineStyle: { color: showAnnotations ? "gray": "transparent", type: "dotted" }, data: ANNOTATIONS.map(a => { return { xAxis: a.date, label: { formatter: a.text }} } ) } }
    ]),
  }
}

// on windown resize, resize the chart
window.addEventListener("resize", (event) => {
  if (chart) {
    chart.resize()
  }
})

window.onload = function () {
  currentMovingAverage = MOVING_AVERAGE_DAYS
  // set the default moving average value in the HTML select
  movingAverageSelect.value = currentMovingAverage

  document.getElementById('maSelector').addEventListener('change', (e) => {
    const selected = Number(e.target.value);
    currentMovingAverage = selected
    let option = chartDefinition(processedData, currentMovingAverage)

    // keep the start date the same as the current chart when the moving average is changed
    let xAxisModel = chart.getModel().getComponent('xAxis', 0);
    // might not be set on e.g. Pie charts
    if (xAxisModel) {
      let xExtent = xAxisModel.axis.scale.getExtent();
      let currentStartDate = xExtent[0];
      option.dataZoom[0].startValue = currentStartDate
    }
    chart.setOption(option);
  });

  Promise.all(CSVs).then(function(data) {
    processedData = preprocess(data)
    let option = chartDefinition(processedData, currentMovingAverage)
    // For e.g. tables, as we don't want to draw them as charts
    if (option === null) {
      return
    }
    draw(option)

    const paramsString = window.location.search;
    const searchParams = new URLSearchParams(paramsString);
    if (searchParams.has("thumbnail")) {
      saveThumbnail()
      window.close()
    }
  });
}
