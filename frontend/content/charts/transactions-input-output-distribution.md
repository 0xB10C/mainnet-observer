---
title: "Transaction Input/Output Distribution"
draft: false
author: "maflcko"
aliases:
  - /charts/transactions-1in-1out/
  - /charts/transactions-1in-2out/
  - /charts/transactions-1in/
  - /charts/transactions-1out/
categories: Transactions
position: 7
tags: [input, output, distribution]
thumbnail: transactions-input-output-distribution.png
chartJS: transactions-input-output-distribution.js
images:
  - /img/chart-thumbnails/transactions-input-output-distribution.png
---

Shows how Bitcoin transactions are distributed across common input and output count patterns.
<!--more-->

The stacked chart separates one-input-one-output, one-input-two-output, one-input-many-output,
many-input-one-output, and all other transactions.

Transactions with one input and one output are [likely self-transfers](https://en.bitcoin.it/wiki/Privacy#Exact_payment_amounts_.28no_change.29).

A lot of transactions that pay someone have one input and two outputs.
The input comes from the payer, one output goes to the payee and the other one (likely) goes back to the payer as a change output.

Transactions with one input and many outputs are usually fan-out transactions,
such as batches or distribution payments.

Transactions with many inputs and one output are usually consolidation transactions.
