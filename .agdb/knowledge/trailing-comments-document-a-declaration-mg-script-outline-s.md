---
type: Gotcha
title: Trailing comments document a declaration (mg-script outline/spec)
description: "How a comment after a declaration's ; becomes its doc in spec.rs, outline.rs and analysis.rs locals, and why it is consumed."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-23T12:02:06Z }
asks:
  - Why does a constant lose its trailing comment?
  - "Why did a constant get the previous line's comment as its doc?"
  - Where is the one place that finds trailing comments?
created: 2026-11-23
author: augustaugur
---
outline::trailing_comment(text,toks,semi) finds a comment after a ; with nothing else after it on the line; outline::doc_with_trailing joins above+trailing (above first). Spec::parse must CONSUME the trailing comment token, or the loop pushes it as the next constant's comment above (15 nwscript.nss constants had that). Heading with a blank line under it is no doc; directly above the first constant it still is. Definitions with bodies take none. Test with game file: ovr/nwscript.nss under the install (mg-script cannot use mg-testkit).
