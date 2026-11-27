---
type: Proposal
title: 'Animated objects in the area scene: a proposal'
description: How mg-area's AreaScene could play each object's own animation for a game client (Moonglow Client's request) - the cut between crates (a GPU-free player and the visual effects' rows in mg-preview, a per-object pose with transitions on Composed, the run-time state held by the caller beside the scene and keyed by a key of the caller's), one section for each of the eight capabilities asked with its API sketch and cost, how the run-time state relates to the saved AnimationState, the viewer's part, the migration of today's idle animation, the tests and client comparisons, risks, open questions and milestones after 5.5.0.
tags: [proposal, animation, mg-area, mg-preview, mg-render, moonglow-client, moonglow-viewer, visual-effects]
status: draft
generated: { by: claude-code/claude-opus-5-5, at: 2026-11-27T07:42:24Z }
sources:
  - id: request
    resource: '~/Projects/moonglow-client, branch main at b6d7bbb: docs/research/stage-for-a-client.md, the section "Draft request for Moonglow Toolset" (a draft Finding, read at the client''s pin v5.4.0)'
    title: 'Moonglow Client: the viewer''s stage for a client driven by server updates'
  - id: area-scene
    resource: crates/mg-area/src/scene.rs, lib.rs, values.rs and walk.rs at develop 321a1ef2
    title: The area scene, its objects and the values API
  - id: compose
    resource: crates/mg-preview/src/compose.rs, object.rs and lib.rs at develop 321a1ef2
    title: Previews composed and posed (Composed), and the placeable's and door's idle from AnimationState
  - id: render
    resource: crates/mg-render/src/anim.rs, scene.rs (Instance), renderer.rs (bones from Instance::pose), particles.rs, dangly.rs, reach.rs (Frustum) at develop 321a1ef2
    title: The renderer's animation primitives, instances, skinning and particles
  - id: mgv-stage
    resource: '~/Projects/moonglow-viewer at b81b03e (on toolset tag v5.4.0): crates/mgv-stage/src/lib.rs, actor.rs, vfx.rs, subject.rs; crates/mgv-library/src/lib.rs'
    title: Moonglow Viewer's stage
  - id: area-view
    resource: crates/mg-ui/src/area_view.rs at develop 321a1ef2 (View built per frame, view.time += dt)
    title: How the area view animates objects now
  - id: frames
    resource: research/notes_frames.md
    title: 'Frames at 144 a second: the budget (6.94 ms) and the area scene''s recorded cost'
  - id: values-note
    resource: .agdb/knowledge/an-areas-model-from-values-the-scenes-objects-followed-witho.md
    title: 'agdb note: an area''s model from values; the scene''s objects followed without a rebuild'
  - id: torch-note
    resource: .agdb/knowledge/an-items-light-is-the-creatures-effect-at-headconjure-0-0-75.md
    title: 'agdb note: the torch arm (torchl) layered over the base and the worn parts'
  - id: emitters-note
    resource: .agdb/knowledge/parts-emitters-tails-wings-held-items-chunk-size.md and heeds-armourer-against-the-client-tiles-emitters-run-ahead.md
    title: 'agdb notes: parts'' emitter keys; emitters run ahead when new'
  - id: door-model
    resource: mgv info t_door01 (genericdoors.2da row 0), the installed game, 2026-11-27
    title: 'A stock door''s animations: opening1, closing1, opening2 and others'
---

# Animated objects in the area scene: a proposal

Moonglow Client asked for creatures, doors and placeables in an
`AreaScene` that animate as a server tells them.[^request] It asked for
eight capabilities. The cut between crates is ours to decide. This
proposal answers each of the eight, says where each part lives, and gives
an order to build it in after 5.5.0. Nothing here has been built.

**Marks.** *Read* means read in the code at develop `321a1ef2` (the
viewer at `b81b03e`). *Inferred* means reasoned from the code but not run.
*Unmeasured* means a cost nobody has timed yet. The client's finding was
read at its pin, `v5.4.0`. Between `v5.4.0` and `321a1ef2`, mg-area gained
the Player's light (`player_light.rs`, and `View::player` in scene.rs);
mg-preview and mg-render's `anim.rs` did not change
(`git diff --stat v5.4.0 321a1ef2`).

## Summary: the cut

- **The player** (what plays, in which mode, time, speed, queue, the
  one-shot's end and the model's events) is GPU-free plain state in
  **`mg_preview::play`**. It is the piece the viewer and the area share.
- **Posing with a blend** goes on **`Composed`** in `mg_preview::compose`:
  a per-object `Pose` (locals of the base and of each animated part),
  blended over the new animation's `transtime`, and drawn by the same
  rules as today's `instances`. Worn parts, the torch overlay and held
  items' emitters come along unchanged.
- **Visual effects' rows, hooks and sequences** move from the viewer into
  **`mg_preview::vfx`**, read through `GameData`.
- **The run-time state** of an area's objects is a **`mg_area::Live`**.
  The caller holds it beside the scene, as it already holds the particle
  simulations, and it is keyed by a new **`ObjectKey`** of the caller's.
  `AreaModel`, the values, stays what is saved. `AreaScene` stays the
  shared cache of looks.
- An object with no entry in `Live` is drawn exactly as today: its idle
  animation, at `View::time`.
- No new crate is needed. All of it fits the layering of
  [PLAN §4](PLAN.md#4-architecture) (mg-render, then mg-preview, then
  mg-area), and nothing goes behind mg-area's `edit` feature.

This is the client's shape (b), with one change: the state sits beside the
area, not on `AreaModel`'s objects.

## What is there today (read)

- **Every object stands in its idle animation at the view's one clock.**
  In `scene_hiding`, each shown object is drawn with
  `c.instances(c.idle, object_time(kind, is_static, view), transform)`.
  `object_time` is `View::time`, or 0 for a static placeable or with
  Animations off. The module comment says so too: "Objects stand in their
  previews' idle animation." The area view sets `View::time` from its own
  frame clock (`view.time += dt`, `area_view.rs`).[^area-scene][^area-view]
- **`Composed` is stateless.** It poses from a name and a time:
  `instances(animation, t, transform)`, `emitting(animation, t)` and
  `point_lights_at(animation, t, transform)`. It has no last pose, no
  transition and no queue. It is shared by `Arc` among every object with
  the same look (`Shown`, the `Looks` cache).[^compose]
- **A static idle is posed once** (`idle_static`, `idle_pose`). Any other
  object is posed again every frame, whether it is in sight or not: the
  object loop has no frustum test. Grass's `Sight` carries one
  (`grass_view.rs`). The renderer culls meshes only after posing, while
  it makes bone matrices on the CPU from `Instance::pose`
  (`renderer.rs`).[^render]
- **Particles** are the caller's `HashMap<(usize, usize), Particles>`,
  keyed by `object_place` (kind and `AreaObject::index`, under 2^24)
  and by the shown look's address (`source_key`). `carry_over` moves a
  sim to a neighboring place of the same look. A new sim runs 20 s ahead
  (`PARTICLES_AHEAD`). Sims not seen in a frame are dropped.[^area-scene]
- **The pieces for a blend already exist** in `mg_render::anim`:
  `locals_layers`, `blend`, `compose`, `pose_worn_layers`, and `Binding`
  and `Playing` for animations bound once. `mg_mdl::Animation` carries
  `length`, `transtime` and `events: Vec<(f32, String)>`.[^render]

## 1. Per-object animation set by the caller

**Answer.** `Live::play(key, Play)` takes a name, a mode (loop; once then
a named loop; once then hold the last frame; once then back to idle), a
speed and a start time. `Composed` poses it, blending from the last pose
over the new animation's `transtime`. Supermodel chains and the
standing-idle fallback (`playing`: `pause1` and `cpause1` stand in for
each other) work as now. Doors and placeables play the same way: a stock
door has `opening1`, `closing1`, `opening2` and others.[^door-model]
Blends of two loops by weight come later, in the same `Pose`.

```rust
// mg_preview::play (no GPU, no feature)
pub enum Then { Loop, Hold, Next(String), Idle }
pub enum Mode { Loop, Once(Then) }
pub struct Play { pub animation: String, pub mode: Mode, pub speed: f32, pub start: f32 }
pub struct Player { /* playing, time, speed, queue */ }
impl Player {
    pub fn play(&mut self, play: Play);
    pub fn queue(&mut self, play: Play);
    pub fn set_speed(&mut self, speed: f32);
    pub fn restart(&mut self);
    pub fn pause(&mut self, paused: bool);
    pub fn now(&self) -> Option<(&str, f32)>;
    /// `find` looks a name up in the model and its supermodels.
    pub fn step<'a>(&mut self, dt: f32, find: &dyn Fn(&str) -> Option<&'a Animation>,
                    heard: &mut Vec<Heard>);
}

// mg_preview::compose (feature render)
pub struct Pose { /* locals: the base's, then each animated part's */ }
impl Composed {
    pub fn sample(&self, animation: Option<&str>, t: f32) -> Pose;
    pub fn blend(&self, from: &Pose, to: &Pose, f: f32) -> Pose;
    pub fn instances_posed(&self, pose: &Pose, animation: Option<&str>, t: f32,
                           transform: Mat4) -> Vec<Instance>;
}
```

`instances(animation, t, transform)` becomes
`instances_posed(&self.sample(animation, t), …)`: one rule for both.

Two things change in `Composed`:

- Today the mesh state (alpha, self-illumination and vertex keys) is
  played only when `animation == self.idle` (`idle_keys_meshes`). Under a
  playback it must follow whatever plays, or a door or placeable whose
  `open` keys its meshes would play its nodes alone. *Read; how much
  stock content keys meshes in non-idle animations is unmeasured.*
- The transition from the first pose has nothing to blend from. As in the
  viewer, it starts unblended.

**Cost.** Sampling is what happens today. A blend adds one `blend` over
the locals for the length of `transtime`, plus `compose`. That is about
one more pass over the nodes per object while it blends. *Unmeasured.*

## 2. Time stepped by the caller

**Answer.** `AreaScene::step(&self, area, &mut Live, dt) -> Vec<Notice>`
moves every playback by `dt` times its speed. Nothing reads a clock. The
same steps give the same poses, and so the same picture. Particles
already step by the caller's `dt`. `View::time` stays the clock of tiles,
water, grass sway and the objects with no playback.

```rust
impl AreaScene {
    pub fn step(&self, area: &AreaModel, live: &mut Live, dt: f32) -> Vec<Notice>;
    pub fn scene_live(&self, area: &AreaModel, view: &View, live: &Live) -> Scene;
}
```

```rust
impl Live {
    pub fn play(&mut self, key: ObjectKey, play: Play);
    pub fn queue(&mut self, key: ObjectKey, play: Play);
    pub fn set_speed(&mut self, key: ObjectKey, speed: f32);
    pub fn pause(&mut self, key: ObjectKey, paused: bool);
    /// Back to the object's idle, blended over its transtime.
    pub fn stop(&mut self, key: ObjectKey);
}
```

`step` takes `&self` because it only reads the looks (an animation's
length and events). `scene(area, view)` stays and draws as
`scene_live(area, view, &Live::default())`.

**Cost.** One addition per playback per step. Steps of `0.5 + 0.5` and of
`1.0` give the same poses. They do not give the same particles, which are
simulated in steps, as they are today.

## 3. Stable object keys

**Answer.** Add an optional key of the caller's on objects:
`values::Object::key` and `AreaObject::key`, both `Option<ObjectKey>`.
Every piece of run-time state is keyed by it: the playback, the effects
and the particle sims. The objects' vector may then shift as it does
today, and nothing that plays is lost. Moves (`position`, `rotation`),
re-dresses (`update_objects(at)`) and removals (`remove_object`) keep
their current calls. Objects without a key (every area read from files,
so the editor) keep today's `object_place` and `carry_over`.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ObjectKey(pub u64);
impl AreaModel { pub fn position_of(&self, key: ObjectKey) -> Option<usize>; }
impl Live { pub fn forget(&mut self, key: ObjectKey); }
// The particle sims' key, opaque, in place of (usize, usize):
pub struct SimKey(/* tile | object place | keyed object, source | effect, source */);
```

**What the client does without it (its result, 2026-11-27).** At
`v5.4.0`, adding, moving and removing objects at run time works with no
toolset change. This is Moonglow Client's test
`objects_are_added_moved_and_removed_at_run_time` (`mc-scene/tests/shard.rs`,
on a builder's area):
- A move of 1.5 m and 45° redraws all 23 of the object's instances at the
  new place in the next scene, with nothing reloaded.
- `update_objects` adds a creature's parts exactly.
- `remove_object` takes out only that object, and the later indices shift
  correctly.

The client keeps its own map from server id to index, and a counter per
kind for the particle keys. So for the client, `ObjectKey` only saves that
map. What it adds beyond that is a re-dress that keeps the particles (below)
and keys that the playback and effects can share.

**Why a new key rather than `(kind, index)`.** `index` is already the
particles' key, and the client sets it. But it is also the object's place
in its GIT list: `AreaModel::object` finds objects by it, and the editor's
edits address objects through it. It is also capped at 2^24 per kind. The
server's object ids are 32-bit (*not checked against the client's
protocol notes*). Who else reads `index`, besides the scene and
`AreaModel::object`, *was not fully surveyed*. A separate key keeps the
two meanings apart.

**Found (read):** today a re-dress restarts an object's particles. The
sim key's second half is the shown look's address, `carry_over` only
matches the same look, and so a new weapon gives the base a new sim,
which runs 20 s ahead. With keys, the sim of a keyed object's base is
kept across a re-dress when the base model is the same.

**Re-dress and the playback.** The playback survives. If the new look
lacks what plays, it falls back as `playing` does. The base's last pose
is kept for the blend when its node count is the same. Animated parts
that changed start unblended.

**Cost.** One hash lookup per keyed object per frame (`Live` is a
`HashMap<ObjectKey, _>`). `position_of` is a linear search, which a
client may replace with its own map. It costs O(n) per call: tens of
microseconds at most for a few hundred objects *(inferred)*.

## 4. Worn and held parts follow the body

**Answer.** They already do in `Composed`, and they keep doing so,
because `instances_posed` keeps today's part rules:

- parts bound to the skeleton follow the base's pose;
- animated parts (robes, cloaks, wings, tails) play the animation on
  their own nodes at the wearer's scale (`worn_scale`), or their own of
  the name;
- held items' emitters are keyed by their `equip` (`held_at`);
- the torch overlay `torchl` is layered over whatever the base plays and
  over the worn parts' bones, never at rest.[^torch-note][^emitters-note]

What is new is that `Pose` holds the animated parts' locals too, so they
blend with the base and do not snap. A re-dress brings its parts with the
new look (capability 3).

**Cost.** As today, plus the blend of the animated parts' locals while a
transition runs. A part-based creature has few animated parts (a robe or
cloak, wings, a tail). Its body parts follow the base's pose for nothing
extra. *Inferred from `part_pose`.*

## 5. Visual effects on hooks, by the caller's key

**Answer.** Move the viewer's rows, hooks and sequences into
`mg_preview::vfx`, GPU-free and reading `visualeffects.2da` through
`GameData`. An applied effect is a small set of `Composed` effect models
(loaded and shared by `AreaScene`'s look cache) hung on the object's
hooks:

- the head (`Imp_HeadCon_Node`), the impact (`Imp_Impact_Node`) and the
  root by the target's size (`Imp_Root_{S,M,L,H}_Node`);
- the hook nodes are chosen by model kind (a door's `hhit`, `impc`,
  `grnd`; others' `head`, `impact`, `root` and their suffixed forms);
- turning with the hook where `OrientWithObject` says, the root kept
  level.

Each model has its own `Player` in `Live`, playing impact then duration.
A `D` effect loops its last animation until it is removed. Any other
plays once and is gone. Removal plays `cessation` where the model has one
and drops the rest.[^mgv-stage]

```rust
// mg_preview::vfx (no GPU)
pub struct VisualEffect { /* row, kind F/D/P/B, models by hook, OrientWithObject, sounds, progfx */ }
pub fn row(game: &GameData, row: u32) -> Option<VisualEffect>;
pub enum Hook { Head, Impact, Root }
pub fn hook_node(model: &Model, hook: Hook) -> Option<usize>;
pub fn sequence(model: &Model, kind: char) -> Vec<Play>;

// mg_area
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EffectKey(pub u64);
impl AreaScene {
    pub fn apply_effect(&mut self, gpu: &Gpu, game: &GameData, area: &AreaModel,
                        live: &mut Live, object: ObjectKey, effect: EffectKey,
                        row: u32, size: u32) -> Result<(), Vec<String> /* models missing */>;
}
impl Live { pub fn remove_effect(&mut self, object: ObjectKey, effect: EffectKey); }
```

`size` is the target's `appearance.2da` `SIZECATEGORY`, as the viewer
takes it (3 for doors and placeables). The caller gives it, because a
client's creature look (`DressedLook`) may not carry its appearance row
(*not checked*). Taking it from the look instead is open question 8.

Effects' particles are keyed by object, effect and source. They must
**not run ahead** when new: `PARTICLES_AHEAD` would spend an impact's
burst before the first frame (*inferred from the code*). Effects' light
nodes join the area's lights. Whether effect models cast the sun's
shadows in the client is *unread*: a reading decides the flag. Beams,
projectiles and `progfx` are not asked and not proposed.

**Cost.** Loading happens once per effect model, at application. Each
frame, each live effect model costs one sample (it is small) and its
particles. A heavy effect is particles, which already cost what they
cost. *Unmeasured.*

## 6. Notice of a one-shot's end and of events

**Answer.** `step` returns notices in the order they happened. An event
is reported each time the time crosses it, in `(t0, t1]`, including
across a loop's wrap and several wraps in one long step. A one-shot's end
is reported once, when `Then` takes over.

```rust
pub enum Heard { Ended { animation: String }, Event { animation: String, name: String } } // mg_preview::play
pub enum Notice {                                                                     // mg_area
    Ended { object: ObjectKey, animation: String },
    Event { object: ObjectKey, animation: String, name: String },
    EffectEnded { object: ObjectKey, effect: EffectKey },
}
```

Events come from the model's own `events` (the base's animation, found
through its supermodels). A stock creature's walk-type animations carry
events: `walk`, `run`, `sitdown` and `taunt` four each, `walkinj` eight,
`bow` two. A stock door's have none (`mgv info a_ba`,
`t_door01`).[^door-model] The events' names were not listed. What an
event *means* (a sound, a hit) stays the caller's.

**Cost.** A scan of the playing animation's events per step: a handful.

## 7. Cheap with hundreds animated; still objects not posed again

**Answer.** Three measures, cheapest first:

1. **Still objects are posed once.** A playback that holds its last frame
   (or is paused) keeps its posed instances in `Live` (`Arc`) and reuses
   them until it plays again or its object moves. Static idles already
   share one pose (`idle_static`).
2. **Out of sight, not posed.** `View` gains `sight: Option<Sight>`, the
   frustum grass already uses. An object whose box at rest (`Shown`'s
   bounds, widened by its animated reach) is out of sight is skipped. Its
   playback still steps, because time is not pose.
3. **Posing in parallel, if the measure asks for it.** Poses are
   independent per object. `rayon` is already a workspace dependency.

```rust
pub struct View { /* … */ pub sight: Option<Sight> }
pub struct Usage { /* … */ pub posed: usize }   // objects posed last frame, for tests and timing
```

**Recorded cost.** The budget is 6.94 ms a frame (144 fps). On a
784-tile area, the scene (tiles and objects posed) took 0.5 ms after
October's work, down from 1.2. The area view redraws for animation every
40 ms (`ANIMATION_STEP`), not every frame.[^frames] A client draws every
frame, so its whole scene cost counts each time.

**Unmeasured.** How long one part-based creature takes to pose, and so
how many fit. This is the first thing to measure (M0 below), before 2
and 3 are built.

Sharing one pose among creatures with the same look, animation and time
(as tiles share theirs) works only while they play in step. A client
whose creatures each have their own phase (open question 3) gets little
from it.

## 8. Resources through GameData

**Answer.** Everything loads through `GameData` and its `resman`, as
`AreaScene` already does (`Models::load`):

- 2DA rows (`visualeffects.2da`) through `GameData::table`;
- models through the `load: &dyn Fn(&str) -> Option<Arc<Model>>` that
  `Composed::new` already takes.

Nothing depends on mgv-library. The viewer passes its `Library`'s
`game()` and `|n| lib.model(n)`.

**Cost.** None beyond what is loaded.

## AnimationState and the run-time state

**Found (read).**

- **scene.rs never reads `AreaObject::state`.** (`posed.state` at
  scene.rs:1837 is a tile's mesh state, not this field.)
- **The picture still follows the field, through the look.** When a look
  is read from a GIT or blueprint struct,
  `mg_preview::object::placeable` maps `AnimationState` to
  `Preview::idle`: 1 `open`, 2 `close`, 3 `dead`, 4 `on`, 5 `off`, else
  `default`. `object::door` maps it to `opened1`, `opened2`, else
  `closed`. `Composed::new` copies that into `Composed::idle`, and the
  scene draws `c.instances(c.idle, …)`.[^compose]
- **`AreaObject::state` is read only by `walk.rs`.** It chooses the
  door's walkmesh surfaces (`DoorState::Open1`, `Open2`, `Closed`). A
  workspace-wide search (`grep -rnE '\.state\b'` over `crates` and
  `apps`) found no other reader. mg-ui and mg-module write the GFF field
  directly.
- **The two copies are not kept in step.** `values::Object::read` reads
  `state` from the field. `Look::read` reads the same field into the
  preview's idle. `AreaObject::from_values` copies both as given. A client
  that builds a door with `state: 1` and a look whose idle is `closed`
  gets an open walkmesh and a closed picture.
- **Correcting the request's premise:** `values.rs` does not write
  `AnimationState` into the area's values. `Look::placeable(game,
  appearance, state, is_static)` sets it only on a scratch struct, to
  read a look from. There is no `Look::door`.

**Proposed relation.**

- `AnimationState` stays the **saved rest state**: what the object stands
  in when nothing plays (its idle) and what its walkmesh is.
- The **playback is transient**. It never writes `state`, and `Live` is
  never saved.
- Changing a door's or placeable's state goes through one call that keeps
  both copies in step, with the mapping lifted from `object.rs` into one
  shared function:

  ```rust
  // mg_preview::object
  pub fn state_idle(kind: ObjectKind, state: u8) -> &'static str;
  // mg_area
  impl AreaObject { pub fn set_state(&mut self, state: u8); } // state and preview.idle
  ```

  The caller then calls `update_objects(at)`, which hits the look cache.
  A preview whose idle was replaced (`AnimationReplace`) needs care
  (open question 4).
- A door opening then plays, for example:

  ```rust
  live.play(k, Play { animation: "opening1", mode: Once(Then::Hold), .. })
  ```

  On `Notice::Ended`, the client calls `set_state(1)` (the idle is now
  `opened1`) and `live.stop(k)`. The blend carries the door from the
  held last frame into its idle. With `Then::Idle` before `set_state`, the
  door would blend back to `closed`.

## The crate cut, weighed

| Where | For | Against |
| --- | --- | --- |
| **mg-preview: `play`, `vfx`, and `Pose` on `Composed` (proposed)** | Parts' posing stays private to `compose.rs`, so the client-read part rules stay one rule. `GameData` is already there for the 2DAs. The viewer and mg-area already depend on it. `play` and `vfx` work without the `render` feature. | mg-preview grows from "what a blueprint looks like" to "and how it moves". |
| A new crate `mg-stage` between mg-preview and mg-area | A clean name and dependency for the viewer. | It must reach into `Composed`'s parts, so `Placed`, `part_pose` and the bindings become public API: a wider surface to keep stable. One more crate to version. |
| mg-render's `anim` | The player is pure and sits next to its primitives. | No `GameData` for effects, and no knowledge of previews: posing a preview would stay elsewhere anyway. |
| mg-area | It is where the client works. | The viewer would have to take tiles, grass and walkmeshes to get a player. |
| The state on `AreaModel`'s objects (the client's wording of (b)) | One structure for the client. | `AreaModel` is the values. `area_values.rs` hashes its `Debug` against the files, and play state is never saved. |
| The state inside `AreaScene` | The fewest calls for a client. | `AreaScene::update` rebuilds would have to keep it. The editor's scenes would carry it. The scene would stop being a function of (area, view). |

The caller holds `Live` as it holds the particle sims: the scene stays a
cache of looks, and the editor passes nothing.

## The viewer's part

mgv-stage has no `anim.rs` or `compose.rs`. Its player is in `actor.rs`,
its step in `lib.rs` (`Stage::step`), and its effects in `vfx.rs` and
`subject.rs`.[^mgv-stage]

**What moves out, into mg-preview:**

- `Player`, `PlayMode`, `queue` and `sequence` (to `play`);
- `Actor::advance`, the one-shot and queue logic (to `Player::step`);
- `Actor::transition` and `Transition` (to `Composed::blend` and the
  playback);
- `sample_worn` (to `Composed`'s worn parts);
- `emitter_keys` (already `Composed`'s `held_at`);
- `vfx::{VisualEffect, row, table, Hook, hook_node, progfx}` and
  `subject::default_sequence`'s effect branch (to `vfx`).

**What it keeps:**

- `Stage` and its actors for single models (a model opened alone,
  `replace_model`, `reload` on edits), on the shared `Player`;
- the camera, lighting rigs, ground and its shadow, the casters view,
  overlays, `posed`, framing, and headless stills and turntables;
- dangly meshes, per actor (the area scene has none: not asked);
- its texture fallbacks, if `Composed` does not already do the same
  (*not compared*).

**The ruling the owner needs (open question 1).** Posing is the part that
is not neutral. The viewer poses one `Actor` per model, and a part
without the animation falls back to `creadyl`, `cpause1`, `pause1`. The
toolset's `Composed` follows the client-read rule instead: a part without
the animation plays none of its own (`reading_wings_without_bones`).
Two options:

- the viewer draws previews through `Composed` and a playback, taking the
  toolset's rule and dropping its own;
- it keeps its actors and shares only `play` and `vfx`.

The first gives one look everywhere. The second keeps the viewer's
per-part inspection.

## Migration of today's idle animation

- `scene(area, view)` keeps its signature and draws through
  `scene_live(area, view, &Live::default())`. With an empty `Live`, every
  object takes today's path: `c.idle`, `object_time`, `idle_static`'s
  shared pose, and particles keyed by `object_place`.
- **Test:** over a sample of the install's areas (`mg_testkit::sweep`),
  the instances' poses and transforms from `scene` before the change and
  from `scene_live` with an empty `Live` hash the same. This is the
  `AREA_DUMP` method of the values note.[^values-note] The `screens` and
  `area_harness` pictures stay unchanged.
- The area view passes no `Live` and looks the same. A builder's "play
  this door" in the area view would be possible later. It is not
  proposed.

## Tests and client comparisons by part

| Part | Toolset tests | Client comparison |
| --- | --- | --- |
| 1 Play, modes, blend | `play` unit tests on made-up animations (loop wraps, once then loop, hold, back to idle, queue, speed, restart); `Composed::blend` halfway at `transtime / 2` on the synthetic skeleton of `compose.rs`'s tests; the mesh state under a non-idle animation | A door scripted open (`ActionOpenDoor`) and a creature in a held animation (a death), screenshot at rest after the one-shot: the held frame matches. Mid-blend timing is not compared, because the client cannot be stepped. |
| 2 Stepped time | The same steps twice give the same poses' hash; split steps give the same poses | Not needed |
| 3 Keys | `tests/scene_updates.rs` extended: add, re-dress, move and remove in mixed orders; the playback, effects and particle sims follow the key; the editor's unkeyed path unchanged (`particles_stay_with_their_objects_when_places_move` kept) | Not needed |
| 4 Parts | A bound robe and an animated cloak blend with the base (synthetic); a held torch's emitters keyed by `equip` under a playback; `torchl` over a playback | A cloaked creature in a held animation against the client's picture |
| 5 Effects | Synthetic effect models on each hook of a creature, a door and a placeable; corpus sweep: every `visualeffects.2da` row's models load and their hooks resolve on a human, a stock door and a placeable (and with `MOONGLOW_HAKS`); instant effects gone after their play; no run-ahead | One duration effect on a creature (impact, then duration) and its hook's place; whether effects cast shadows (a reading) |
| 6 Notices | An end exactly at the length; an event per crossing over a wrap, at speed 2 and over several loops in one step; events found through supermodels | Not needed |
| 7 Cost | A timing test (beside `world_perf`): 300 stock creatures playing, frame time against the budget; `Usage::posed` is 0 for a frame of still objects; objects out of sight not posed | The client's own frame times are its side |
| 8 Resources | mg-area built with `default-features = false` reaches every new call (CI already runs clippy on `mg-area --no-default-features` and checks its dependency tree); a hak's `visualeffects.2da` row read through the load order | Not needed |

## API changes at the client's pin

These are breaking for code written against `v5.4.0`:

- **New public fields** on `values::Object` (`key`) and `AreaObject`
  (`key`). Struct literals without `..` must add them. Objects made with
  `Object::new` or `facing` are not affected. How the client builds its
  objects was *not checked*.
- **New public field** on `View` (`sight`). `View::of` fills it with
  `None`. `View` already gained `player` after `v5.4.0`, so a literal
  without `..` breaks at 5.5.0 anyway.
- **The particle sims' key type** changes from `(usize, usize)` to
  `SimKey`, in `particles` and `particles_and_chunks`. The alternative
  keeps the tuple and gives keyed objects compact slots as a tenth
  "kind". That is uglier, but not a break.
- **New public field** on `Usage` (`posed`).

Nothing is removed, and nothing depends on the `edit` feature.

## Risks

- **Cost is the unknown.** If posing a few hundred part-based creatures
  every frame is over the budget even in sight only, the client needs a
  level of detail (posing far ones less often). That changes what is
  drawn and needs a client reading of what the game does.
- **Two rules for parts** if the viewer keeps its own posing
  (open question 1).
- **Effects in the light budget.** Effects' lights join the area's
  lights and its existing cap. A spell storm could push the area's own
  lamps out *(inferred; the cap's rule was not re-read for this)*.
- **Picking** stays on the box at rest (`Shown::stance`). A creature
  mid-animation is picked by its standing box. That is fine for an
  editor. For a client it is open question 6.

## Open questions needing a ruling

1. **The viewer's posing:** Composed and playback (one rule, the
   client's), or its own actors on the shared player? (See
   [The viewer's part](#the-viewers-part).)
2. **Where `play` and `vfx` live:** in mg-preview as proposed, or in a
   new `mg-stage` crate (a wider `Composed` API)?
3. **Each creature's idle phase:** do idles start at each object's own
   time, as a client with `Live` can give them, or does the client keep
   all creatures in step as the editor does? A client reading settles
   what the game does.
4. **`set_state` and a replaced idle:** when an object's
   `AnimationReplace` replaced its idle, does a state change keep the
   replacement? It needs an engine or client reading.
5. **The particle key:** an opaque `SimKey` (a break) or compact slots
   (no break)?
6. **Picking a playing creature:** by its pose (cost: a stance per pick)
   or by its box at rest?
7. **Do effect models cast the sun's shadows** in the client? This is a
   reading, not a ruling, but it is needed before M4.
8. **An effect's target size:** given by the caller with each effect (as
   proposed), or kept on the object's look?

## Build order (after 5.5.0)

| Milestone | What | Size |
| --- | --- | --- |
| **M0 Measure** | Time posing 50, 150 and 300 stock creatures through today's `Composed::instances` in a `world_perf`-style test. Read in the client: idle phases (Q3), door states and a replaced idle (Q4), effects' shadows (Q7). | S |
| **M1 Player and Pose** | `mg_preview::play` (`Player`, `Heard`). `Composed::sample`, `blend` and `instances_posed`, with `instances` rewritten over them (pictures unchanged). The mesh state follows what plays. The viewer can start on `play`. | M |
| **M2 Live in the area** | `ObjectKey`, `SimKey`, `Live`, `step`, `scene_live`, `particles` by key, notices (ends and events), `set_state` and `state_idle`. Migration test with an empty `Live`. Doors and placeables. | M–L |
| **M3 Cost** | Still playbacks posed once, `View::sight` culling, `Usage::posed`, the 300-creature timing test, and parallel posing only if M0 and this test ask for it. | M |
| **M4 Effects** | `mg_preview::vfx`, `apply_effect` and `remove_effect`, effect particles without run-ahead, effect lights, the row sweep and the client comparison. | M |
| **M5 Viewer** | In the viewer's repository, after a toolset tag: mgv-stage on `play` and `vfx`, and on `Composed` per open question 1. | M (viewer) |
| Later | Blends of two loops by weight, once the client has measured the need (`Pose` takes it). Beams, projectiles and `progfx`, if asked. | — |

M1 to M4 each land in a toolset release. The client can move objects and
draw idles on today's API while M0 and M1 run.[^request]

[^request]: Moonglow Client, `docs/research/stage-for-a-client.md` on `main` (b6d7bbb), "Draft request for Moonglow Toolset".
[^area-scene]: `crates/mg-area/src/scene.rs` (`scene_hiding`, `object_time`, `particles_and_chunks`, `object_place`, `source_key`, `carry_over`, `update_objects`, `remove_object`), `lib.rs` (`AreaObject`), `walk.rs` (`ObjectWalkmeshes::faces`), at develop 321a1ef2.
[^area-view]: `crates/mg-ui/src/area_view.rs`: the `View` built for each frame, `animate: !still_objects`, `view.time += dt`.
[^compose]: `crates/mg-preview/src/compose.rs` (`Composed`, `base_posed`, `part_pose`, `posed`, `emitting`, `playing`) and `object.rs` (`placeable`, `door`), at develop 321a1ef2.
[^render]: `crates/mg-render/src/anim.rs`, `scene.rs` (`Instance`), `renderer.rs` (bones from `Instance::pose`, meshes culled after), `crates/mg-mdl/src/lib.rs` (`Animation`), at develop 321a1ef2.
[^mgv-stage]: Moonglow Viewer at b81b03e: `crates/mgv-stage/src/actor.rs` (`Player`, `advance`, `transition`, `sample_worn`), `lib.rs` (`Stage::step`, `add_preview`), `vfx.rs` (`row`, `hook_node`, `apply`, `remove`), `subject.rs` (`default_sequence`).
[^door-model]: `mgv info t_door01` and `mgv info a_ba` on the installed game, 2026-11-27.
[^torch-note]: agdb note "An item's light is the creature's effect at headconjure + (0, 0.75, 0.6); the torch raises the arm by torchl".
[^emitters-note]: agdb notes "Parts' emitters: tails, wings, held items, chunk size" and "Heed's armourer against the client, tiles' emitters run ahead".
[^values-note]: agdb note "An area's model from values; the scene's objects followed without a rebuild".
[^frames]: [Frames at 144 a second](research/notes_frames.md): the budget and the 784-tile area's scene cost.
