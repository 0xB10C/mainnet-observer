---
title: "First BIP-54 Coinbase Transactions by Pool"
draft: false
author: "0xb10c"
categories: mining-pools
categories_weight: 0
tags: [bip54, coinbase, pools]
thumbnail: mining-pools-mining-bip54-coinbase.png
chartJS: mining-pools-mining-bip54-coinbase.js
images:
  - /img/chart-thumbnails/mining-pools-mining-bip54-coinbase.png
---

Shows when a pool first mined block with a BIP-54 valid coinbase.

<!--more-->

[BIP-54 (Consensus Cleanup)](https://github.com/bitcoin/bips/blob/master/bip-0054.md) specifies: The coinbase transaction's `nLockTime` field must be set to the height of the block minus 1 and its `nSequence` field must not be equal to `0xffffffff`.