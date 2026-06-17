---
title: "BIP-69 compliant Transactions"
draft: false
author: "0xb10c"
categories: Transactions
position: 4
tags: [BIP-69, Privacy]
thumbnail: transactions-bip69-compliant.png
chartJS: transactions-bip69-compliant.js
images:
  - /img/chart-thumbnails/transactions-bip69-compliant.png
---

Shows the percentage of transactions compliant with BIP-69 ordering of inputs and outputs.
<!--more-->

[BIP-69](https://github.com/bitcoin/bips/blob/master/bip-0069.mediawiki) specifies the lexicographical ordering of inputs and outputs.
This makes it harder to fingerprint certain wallets and increases privacy.
The metric reports transactions being BIP-69 compliant, but this does not mean that the inputs and outputs are purposefully ordered to be BIP-69 compliant.
Transactions with only a few inputs and outputs, for example transactions with [one input and two outputs](/charts/transactions-input-output-distribution/), are common.
But these transactions have a high probability of being BIP-69 compliant when the inputs and outputs are ordered randomly.
All transactions with [one input and one output](/charts/transactions-input-output-distribution/) are automatically considered to be BIP-69 compliant.
The percentage reported here can be considered as an upper bound.
