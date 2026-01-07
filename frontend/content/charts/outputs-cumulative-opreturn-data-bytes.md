---
title: "Cumulative OP_RETURN Data Bytes"
draft: false
author: "0xb10c"
categories: "Outputs"
position: 6
tags: [op_return, cumulative]
thumbnail: outputs-cumulative-opreturn-data-bytes.png
chartJS: outputs-cumulative-opreturn-data-bytes.js
images:
  - /img/chart-thumbnails/outputs-cumulative-opreturn-data-bytes.png
---

Shows the cumulative data bytes in OP_RETURN outputs.
<!--more-->

OP_RETURN outputs (aka datacarrier outputs or nulldata outputs) allow for the inclusion of arbitrary data in an output script (following the OP_RETURN opcode).
OP_RETURN outputs are provably unspendable, so can be safely dropped from the UTXO set by a Bitcoin node.
This chart tracks the cumulative bytes stored via OP_RETURN outputs since the start of the chain.
