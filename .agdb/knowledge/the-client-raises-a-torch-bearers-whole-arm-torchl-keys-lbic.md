---
type: Fact
title: "The client raises a torch-bearer's whole arm (torchl keys lbicep_g); 'only the forearm bends' is a steep camera's reading"
description: "Settles a report that the previews raise a torch-bearer's whole arm where the client bends only the forearm: the client does the same, and nothing was to fix."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-26T23:06:55Z }
touches: [crates/mg-corpus-tests/tests/client_render/torch_arm.rs, "crates/mg-render/src/anim.rs:Playing", crates/mg-preview/src/compose.rs]
asks:
  - Does the client raise the whole arm or only the forearm for a torch?
  - "Is torchl layered additively or does each key replace the idle's?"
  - Why does a torch-bearer look as if only his forearm is bent in the game?
  - "How do I compare a held item's place in Moonglow with the client's?"
fingerprint: v2-73928249f1d4072e
created: 2026-11-26
author: augustaugur
---
## Found (2026-11-26, EE 89.8193.37)
- Report: previews (toolset and viewer stage) raise the whole arm above the head; the client bends only the forearm (nw_bandit001 + nw_it_torch001, pictures at a steep player camera).
- Client run close and from four sides (torch_arm_look, torch_arm_held_as_the_client_holds_it in client_render/torch_arm.rs): the client raises the whole arm exactly as Moonglow. torchl (a_ba_non_combat, animroot lbicep_g) keys orientation of lbicep_g, lforearm_g, lforearm, lhand_g, lhand (+ lshould_g and a shoulder pad bone the bandit's c_ynpc_h_m03 lacks); each key replaces the idle's on its node, the upper arm included; the other nodes idle. The wiki's Animations page: animroot is the highest bone the animation controls.
- The upper arm points ahead about level, the forearm up. Seen steeply from ahead (SetCameraFacing(90, 10, 40)) the arm ahead is drawn below the head and reads as 'upper arm hanging, forearm bent'. Moonglow from that camera reads the same. A forced PlayAnimation(ANIMATION_LOOPING_PAUSE) changes nothing.
- Measure: the torch's model overridden by a magenta stick along the item's +y (the torch's own length; +z points away from a camera ahead and is seen end on). Its middle and direction in the client's picture against Moonglow's: 0.7 to 4.5 px, up to 1 degree; the pose from the keys projected through shoot_close's camera: 5 to 8 px. Forearm-only layering (torchl without lbicep_g): 160 to 185 px away.
- anim.rs and mg-preview are the same at the viewer's tag (v5.4.0) and develop; the viewer built on develop's crates (cargo --config patch lines, in a clone) draws the same pose. It needed one line for Scene's new boxes field, which its next tag bump needs anyway.
## Tried and ruled out
- Judging the arm from a front or steep picture: cannot tell a forward upper arm from a hanging one. Use a side view.
- The installed mgv 0.3.1 as the 'before' picture: it plays no overlay at all (arm down).
