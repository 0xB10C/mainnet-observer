---
title: "Inputs per Outputs"
draft: false
author: "0xb10c"
aliases:
    - /charts/payments/inputsPerOutputs
categories: "Inputs and Outputs"
position: 2
tags: [Inputs, Outputs]
thumbnail: inputs-per-outputs.png
chartJS: inputs-per-outputs.js
images:
  - /img/chart-thumbnails/inputs-per-outputs.png
---

Shows the number of inputs per output per day.
<!--more-->

An input is an unspent-transaction-output (UTXO) being spent.
A rate of 1.00 input per output means that exactly the same amount of UTXOs are being spend as are being created.
Rates higher than 1.00 mean that more inputs are spent than UTXOs created.
The net UTXO-set size decreases.
This happens for example when an exchange consolidates a lot of UTXOs into one.
If the rate is lower than 1.00, then the net UTXO-set size increases.
More new UTXOs are created than are spent.
This happens for example when a [transaction spends one input and creates two outputs](/charts/transactions-input-output-distribution/), where one pays a recipient and the other one is a change output.
