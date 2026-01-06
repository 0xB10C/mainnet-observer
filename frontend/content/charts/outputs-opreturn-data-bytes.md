---
title: "OP_RETURN Data Bytes"
draft: false
author: "deadmanoz"
categories: "Outputs"
position: 6
tags: [op_return]
thumbnail: outputs-opreturn-data-bytes.png
chartJS: outputs-opreturn-data-bytes.js
images:
  - /img/chart-thumbnails/outputs-opreturn-data-bytes.png
---

Shows the total data bytes in OP_RETURN outputs per day.
<!--more-->

OP_RETURN outputs (aka datacarrier outputs or nulldata outputs) allow for the inclusion of arbitrary data in an output script (following the OP_RETURN opcode).
OP_RETURN outputs are provably unspendable, so can be safely dropped from the UTXO set by a Bitcoin node.
This chart tracks the total volume of data bytes stored via OP_RETURN outputs daily.
