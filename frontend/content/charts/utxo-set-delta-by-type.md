---
title: "UTXO Set Delta by Script Type"
draft: false
author: "0xb10c"
categories: "UTXO set"
categories_weight: 10
tags: [UTXO, "Script Types", "Set Analysis", P2PKH, P2SH, P2WPKH, P2WSH, P2TR, "Taproot", "SegWit"]
thumbnail: utxo-set-delta-by-type.png
chartJS: utxo-set-delta-by-type.js
images:
  - /img/chart-thumbnails/utxo-set-delta-by-type.png
---

Shows the net change in UTXO count by script type per day (outputs created - inputs spent).
<!--more-->

This chart displays how the Bitcoin UTXO set composition evolves over time by showing the daily net change (delta) for each script type. Positive values indicate UTXO creation (more outputs than inputs), while negative values indicate UTXO destruction (more inputs than outputs).