---
title: "BIP-110 Signaling Blocks"
draft: false
author: "0xb10c"
categories: Block
categories_weight: 0
tags: [bip110, signaling]
thumbnail: blocks-bip110-signaling.png
chartJS: blocks-bip110-signaling.js
images:
  - /img/chart-thumbnails/blocks-bip110-signaling.png
---

Shows the number of blocks per day where the BIP-110 version bit signaling flag is set.

<!--more-->

[BIP-110](https://github.com/bitcoin/bips/blob/master/bip-0110.mediawiki) is a proposal for a temporary soft fork that restricts data storage in transactions. Miners signal support for a soft fork by setting a specific bit in the block version field, as defined by [BIP-9 (Version bits with timeout and delay)](https://github.com/bitcoin/bips/blob/master/bip-0009.mediawiki). BIP-110 uses version bit 4. A block is counted as signaling when bit 4 is set in the block version, i.e. `(version & (1 << 4)) != 0`.

Note: version bit 4 was previously used by [BIP-91](https://github.com/bitcoin/bips/blob/master/bip-0091.mediawiki) (Reduced threshold Segwit MASF) during the SegWit activation in July 2017, which explains the cluster of signaling blocks visible around that time.
