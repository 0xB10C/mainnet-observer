---
title: "BIP-54 Coinbase Locktime set"
draft: false
author: "0xb10c"
categories: Transactions
categories_weight: 0
tags: [coinbase, locktime bip54]
thumbnail: transactions-coinbase-locktime-bip54.png
chartJS: transactions-coinbase-locktime-bip54.js
images:
  - /img/chart-thumbnails/transactions-coinbase-locktime-bip54.png
---

Shows the percentage of coinbase transactions that have their locktime set according to BIP-54.

<!--more-->

[BIP-54 (Consensus Cleanup)](https://github.com/bitcoin/bips/blob/master/bip-0054.md) specifies: The coinbase transaction's `nLockTime` field must be set to the height of the block minus 1 and its `nSequence` field must not be equal to `0xffffffff`.

As of early 2026, no coinbase transactions had their locktime set according to BIP-54.
