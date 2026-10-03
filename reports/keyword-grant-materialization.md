# Supported triggered-keyword grant materialization

Baseline: origin/main e8740178a7f7367ffa3147e7642607042079237c. Probes used the frozen `compile_oracle_text` binary and the Scryfall `cards.json` snapshot, without rebuilding the baseline.

## Inventory and scope

Searching non-reminder Oracle clauses for grants of melee, myriad, and afflict found 21 cards. Twelve failed strict baseline compilation; nine already compiled. Ten failures enter the generic static/equipment/temporary keyword-grant paths fixed here:

- Adriana, Captain of the Guard
- Blade of Selves
- Cybermen Squadron
- Dagger of the Worthy
- Depthshaker Titan
- Drogskol Reinforcements
- Fang of the Pack
- Legion Loyalty
- Skyhunter Strike Force
- Titania, Proud Pummeler

The exact ten source texts and public Scryfall links are retained in `fixtures/keyword_grant_materialization.json.fixture`. These are baseline-confirmed failures, not a claimed post-change improvement count until the regression suite and corpus probe pass.

Two other baseline failures remain outside this fix: Auton Soldier's copy-entry front-end conversion requires a compiler-owned object-ability representation for Myriad; Vivien's Stampede has an independent “for each player who was dealt combat damage this turn” draw-predicate error. Existing successful candidates are Corporeal Projection, Cyberman Patrol, Duke Ulder Ravengard, Firbolg Flutist, Ironwill Forger, Lazotep Sliver, Lost Monarch of Ifnir, Mass of Mysteries, and Muddle, the Ever-Changing.

Aeon Chronicler is a separate runtime gap, not a marker-materialization omission. Its `Suspend X—{X}{3}{U}. X can't be 0.` requires a chosen time-counter amount and a minimum-X constraint; the existing Suspend action and alternative-cast model store a fixed `u32` time. This patch does not suppress that unsupported diagnostic.

## Implementation

The semantic keyword grant eligibility now includes Melee, Myriad, and Afflict. Static AST retains typed keyword actions; runtime lowering expands them through the same printed-card keyword builders as temporary grants. Duplicate temporary-only Afflict/Myriad constructions were removed. No card names, marker suppression, or new runtime primitive are needed.

## Validation

Targeted commands (one shared build job):

```
cargo test -p ironsmith-compiler-grammar keyword_grant_materialization
cargo test -p ironsmith-compiler-lowering keyword_grant_materialization
cargo test -p ironsmith-compiler-runtime keyword_grant_materialization
```

Tests cover typed anthem/equipment AST and filters, alternative-cost counterexamples, printed/static/temporary lowering parity, strict compilation of ten exact cards, and direct plus artifact-decoded runtime scenarios: independent multiple melee instances, controller/other scoping and source removal; myriad copying the recipient only for nondefending opponents; equipment afflict following attachment, triggering once for multiple blockers, and affecting only the defending player.

The worker ran rustfmt parsing and `git diff --check`. Rust compilation/tests are reserved for the coordinator's serialized build and have not been run in this worktree.

## Coordinator validation follow-up

The coordinator's serialized run passed all three grammar tests, the lowering parity test, the ten-card strict/artifact regression, and the equipment-afflict runtime scenario. The initial melee and myriad runtime scenarios exposed fixture errors: `TriggerIdentity` is a structural hash rather than an occurrence ID; token entry requires the turn's actual phase to be Combat, not merely a populated CombatState. The corrected scenarios use the public trigger queue and stack resolution APIs. Melee checks two separate stack entries and the intermediate/final +2/+2 increments; Myriad explicitly establishes Alice's combat phase. No engine behavior changed in this correction. The corrected tests await the coordinator's rerun.

The subsequent coordinator rerun passed Myriad, Afflict, and the ten-card fixture test. The remaining melee setup populated only the legacy attacked-player summary map, whereas compiled `TurnHistoryCount::PlayersAttackedThisCombat` reads recorded CreatureAttackedEvent snapshots. Its fixture now declares the recipient attacking Bob and a support creature attacking Cara through `apply_attacker_declarations`, which records the real combat state and event history together, then resolves the recipient's two occurrences from that actual trigger queue. This is also test-only; rerun pending.
