---
title: "Sigops per block"
draft: false
author: "0xb10c"
categories: Block
categories_weight: 0
tags: ["sigops"]
thumbnail: block-sigops.png
chartJS: block-sigops.js
images:
  - /img/chart-thumbnails/block-sigops.png
---

Shows the daily average legacy sigop count per block.

<!--more-->

Bitcoin consensus limits the number of signature operations (sigops) a block may
contain to 80'000. This is the "legacy" sigop count: `OP_CHECKSIG`/`OP_CHECKSIGVERIFY`
count as one sigop and `OP_CHECKMULTISIG`/`OP_CHECKMULTISIGVERIFY` count as up to
twenty. Sigops outside of a witness (SegWit v0) context, i.e. legacy scriptSigs,
non-P2SH-wrapped scriptPubkeys, and P2SH redeem scripts, are weighted 4x so that
they're comparable to the block weight. Sigops inside a witness (P2WPKH/P2WSH,
optionally P2SH-wrapped) are counted at their unweighted 1x value. Pay-to-Taproot
inputs and outputs don't count towards this limit at all.

In the past, there have been blocks that came close to the sigops limit. As this chart
shows the daily average, these don't appear here. A case where a pool mined two invalid
blocks with too many sigops is described in [Invalid F2Pool blocks 783426 and 784121].


[Invalid F2Pool blocks 783426 and 784121]: https://b10c.me/observations/11-invalid-blocks-783426-and-784121/
