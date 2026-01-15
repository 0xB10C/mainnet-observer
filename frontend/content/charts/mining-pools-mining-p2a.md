---
title: "First P2A by Pool"
draft: false
author: "0xb10c"
categories: mining-pools
categories_weight: 0
tags: [p2a, table]
thumbnail: mining-pools-mining-p2a.png
chartJS: mining-pools-mining-p2a.js
images:
  - /img/chart-thumbnails/mining-pools-mining-p2a.png
---

Shows when a pool first mined a transaction spending or creating P2A.

<!--more-->

Here, **first spend** is the height and date when a transaction included by the
pool first spend an Pay-To-Anchor (P2A) output while **first creation** shows the
height and date when a transaction included by the pool first created a P2A
output. The table is sorted by **first spend**.


Since P2A spending is only standard starting with Bitcoin Core v28.0, this data can be used to get an
overview of which pools have upgraded to v28.0 or newer. However, since pools like Ocean allow pool
participants to supply their own block templates, it doesn’t mean that the pool will consistently mine
P2A spending transactions. Additionally, a pool might run a mix of old and new node software,
which causes it to sometimes mine P2A spending transactions and sometimes not.
