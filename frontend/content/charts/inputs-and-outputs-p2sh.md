---
title: "P2SH Inputs and Outputs (including nested SegWit)"
draft: false
author: "0xb10c"
categories: "Inputs and Outputs"
categories_weight: 10
tags: [P2SH, "Pay-to-Script-Hash"]
thumbnail: inputs-and-outputs-p2sh.png
chartJS: inputs-and-outputs-p2sh.js
images:
  - /img/chart-thumbnails/inputs-and-outputs-p2sh.png
---

Shows the number of P2SH inputs and outputs per day, including a combined view of P2SH + nested SegWit input spending.
<!--more-->

P2SH (Pay-to-Script-Hash) templates were introduced with [BIP-16](https://github.com/bitcoin/bips/blob/master/bip-0016.mediawiki).
BIP-16 was activated on the Bitcoin network in April 2012.

Since the activation of SegWit, the number of P2SH inputs reported is significantly lower than the number of P2SH outputs.
This is due to Nested P2WPKH and Nested P2WSH outputs being indistinguishable from P2SH outputs.
However, Nested SegWit inputs can be distinguished and are tracked separately - see [Nested P2WPKH](/charts/inputs-nested-p2wpkh) and [Nested P2WSH](/charts/inputs-nested-p2wsh).

To provide a complete view of P2SH-related transaction activity, this chart shows three lines: pure P2SH inputs, combined P2SH + nested SegWit inputs, and P2SH outputs.

More information about P2SH can be found on [learnmeabitcoin.com](https://learnmeabitcoin.com/guide/p2sh) and the [Bitcoin Wiki](https://en.bitcoin.it/wiki/Pay_to_script_hash).
