---
title: "Blocks with Unclaimed Coins"
draft: false
author: "0xb10c"
categories: Block
categories_weight: 0
tags: [Coinbase, Subsidy]
thumbnail: blocks-coinbase-unclaimed.png
chartJS: blocks-coinbase-unclaimed.js
images:
  - /img/chart-thumbnails/blocks-coinbase-unclaimed.png
---

Table of all blocks where the miner did not claim the full allowed coinbase reward.
<!--more-->

Miners are allowed to claim up to `block subsidy + total transaction fees` in the coinbase transaction.
If a miner claims less than that maximum, the difference is permanently destroyed — no one can ever spend those coins.

The **unclaimed** column shows how many coins were left unclaimed in that block.
