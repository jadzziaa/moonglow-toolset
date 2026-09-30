# Creature challenge rating (Aurora's calculation)

Aurora computes a creature's `ChallengeRating` when its Creature Properties
open (Basic › Challenge Rating) and stores it on OK; the stored value is
calculated + `CRAdjust`. The engine does not recompute it:
`GetChallengeRating` returns the stored value (a creature spawned with 42
stored reports 42), so Aurora is the only oracle.

Not yet reproduced. What is known, from `creature-probes.mod` (oracle
capture; `crates/mg-corpus-tests/examples/cr_probe_module.rs` makes the
module, each creature the bandit `nw_bandit001` with one thing changed:
base HP 10, Str 15, Dex 13, Con 12, 11 feats including Toughness):

| probe | change | MaxHP | CR |
|---|---|---|---|
| cra01 | Fighter 1 | 12 | 0.5 |
| cra02 | Fighter 2 | 14 | 1 |
| cra03 | Fighter 3 | 16 | 2 |
| cra04 | Fighter 5 | 20 | 3 |
| cra05 | Fighter 10 | 30 | 5 |
| cra06 | Fighter 20 | 50 | 9 |
| cra07 | Wizard 5 | 20 | 3 |
| cra08 | Commoner 1 | 12 | 1 |
| cra09 | Commoner 5 | 20 | 3 |
| cra10 | Animal 1 | 12 | 1 |
| cra11 | Animal 5 | 20 | 3 |
| cra12 | Dragon 10 | 30 | 5 |
| cra13 | Fighter 5 + Wizard 5 | 30 | 5 |
| cra14 | Fighter 5, Str 20 | 20 | 3 |
| cra15 | Fighter 5, Con 20 | 40 | 3 |
| cra16 | Fighter 5, Dex 20 | 20 | 3 |
| cra17 | Fighter 5, natural AC 10 | 20 | 4 |
| cra18 | Fighter 5, base HP 100 | 110 | 6 |
| cra19 | Fighter 5, 3 special abilities | 20 | 3 |
| cra20 | Fighter 5, no feats | 15 | 2 |
| cra21 | Fighter 1, Str/Dex/Con 3, HP 1, no feats | 1 | 0.25 |
| cra22 | Undead 8 | 26 | 4 |

Observations: the class's `EffCRLvl` columns do not explain it (Wizard 5 and
Fighter 5 both give 3); hit points count (base HP 100 gives +3, but Con 20's
+20 hit points gives nothing), natural AC counts (+1 for 10), feats count
(−1 without them), ability scores and the special abilities tried here do
not; low levels give fractions (fractionalcr.2da: 1/2, 1/3, 1/4, 1/6, 1/8).
The Creature Wizard's review page shows the same value (a level-1 human
Fighter: 1/2).

Next: probes that vary one quantity in finer steps (base HP 10–200 at one
level, natural AC 0–20, feat counts 0–20, levels 1–40 at HP scaled with
level) to fit each term.
