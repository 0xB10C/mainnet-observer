---
title: "First Ephemeral Dust by Pool"
draft: false
author: "0xb10c"
categories: mining-pools
categories_weight: 0
tags: [ephemeral, dust]
thumbnail: mining-pools-mining-ephemeral-dust.png
chartJS: mining-pools-mining-ephemeral-dust.js
images:
  - /img/chart-thumbnails/mining-pools-mining-ephemeral-dust.png
---

Shows when a pool first mined a transaction spending ephemeral dust.

<!--more-->

Here, **height** and **date** are the first time the pool mined a transaction spending ephemeral dust.
The **total** column shows who many times the pool mined transactions spending ephemeral dust.

Since Ephemeral Dust is only supported starting with Bitcoin Core v29.0, this data can be used to get an
overview of which pools have upgraded to v29.0 or newer. However, since pools like Ocean allow pool
participants to supply their own block templates, it doesn’t mean that the pool will consistently mine
Ephemeral Dust spending transactions. Additionally, a pool might run a mix of old and new node software,
which causes it to sometimes mine Ephemeral Dust transactions and sometimes not.
