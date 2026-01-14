---
title: "Coinbase Output Types by Count"
draft: false
author: "Vojtěch Strnad"
categories: "Outputs"
categories_weight: 0
tags: [Coinbase, P2WSH, P2SH, P2MS, P2WPKH, P2PK, P2PKH, "OP_RETURN", "Type-Distribution"]
thumbnail: output-coinbase-type-distribution-count.png
chartJS: output-coinbase-type-distribution-count.js
images:
  - /img/chart-thumbnails/output-coinbase-type-distribution-count.png
---

Shows the distribution of coinbase output types by output count per day.
<!--more-->

Since the activation of SegWit, every block containing SegWit spending transactions
(which is almost all blocks) must include a witness commitment in the form of an
OP_RETURN output in its coinbase transaction.

Additional OP_RETURN outputs are often used for merge mining. Between 2018 and 2019,
some miners of the Rootstock sidechain embedded its tags in coinbase transactions
without using OP_RETURN, resulting in an unusally large number of non-standard outputs.
