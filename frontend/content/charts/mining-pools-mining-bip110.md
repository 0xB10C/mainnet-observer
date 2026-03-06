---
title: "BIP-110 Signaling Blocks by Pool"
draft: false
author: "0xb10c"
categories: mining-pools
categories_weight: 0
tags: [bip110, signaling, pools]
thumbnail: mining-pools-mining-bip110.png
chartJS: mining-pools-mining-bip110.js
images:
  - /img/chart-thumbnails/mining-pools-mining-bip110.png
---

Shows when a pool first mined a block signaling [BIP-110](https://github.com/bitcoin/bips/blob/master/bip-0110.mediawiki) and the total number of such blocks mined.

<!--more-->

[BIP-110](https://github.com/bitcoin/bips/blob/master/bip-0110.mediawiki) is a proposal for a temporary soft fork that restricts data storage in transactions. Miners signal support by setting version bit 4 (`1 << 4`) in the block version field, as defined by [BIP-9](https://github.com/bitcoin/bips/blob/master/bip-0009.mediawiki). Only blocks in the height range 926000–970000 are counted to exclude the earlier [BIP-91](https://github.com/bitcoin/bips/blob/master/bip-0091.mediawiki) signaling from July 2017, which reused the same bit.
