---
title: "Sigops per Block"
draft: false
author: "0xb10c"
categories: Block
tags: [Sigops]
thumbnail: block-sigops.png
chartJS: block-sigops.js
images:
  - /img/chart-thumbnails/block-sigops.png
---

Shows the average number of signature operations (sigops) per block per day.

<!--more-->

Each Bitcoin block has a sigops limit of 80000. Sigops are counted with a weight of 4 for legacy and 1 for SegWit inputs. Taproot key-path and script-path spends do not count towards the sigops limit.
