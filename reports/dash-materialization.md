# Dash paid-cost identity and materialization

Baseline: `e8740178a7f7367ffa3147e7642607042079237c`, authoritative frozen sharded snapshot. There are exactly **21** entries whose unsupported-content diagnostic points into the conditional `Dash haste` grant. Their complete source card metadata, Oracle text, Oracle IDs, and Scryfall links are retained in `fixtures/dash_materialization.json.fixture` from the same frozen `cards.json` source.

## Root cause and correction

The existing engine already implements Dash's alternative mana cost, cast-time paid fact, conditional haste, and one-shot return at the next end step. Compiler lowering and artifact materialization already carry those structures. However, the shared `OptionalCostKind::from_label("Dash")` fell through to `CustomUnsupported("Dash")`: Dash was missing from the typed payment identities beside Blitz, Evoke, and Escape. That unsupported identity was retained in `Condition::ThisSpellPaidLabel` and correctly rejected by the support validator.

Add `OptionalCostKind::Dash`, its recognized label, and its canonical label. Both compiler conditions and runtime payment facts now use the same typed identity through their existing shared conversion path. No card-name rule, new broad fallback, unsupported-validator change, or behavior inferred from a display label is added. Unknown costs remain unsupported. This is a missing typed identity, not a newly implemented Dash mechanic.

## Frozen affected member list

- Alesha's Vanguard
- Ambuscade Shaman
- Death-Greeter's Champion
- Flamerush Rider
- Goblin Heelcutter
- Kolaghan Forerunners
- Kolaghan Skirmisher
- Kolaghan, the Storm's Fury
- Lightning Berserker
- Mardu Scout
- Mardu Shadowspear
- Mardu Strike Leader
- Pitiless Horde
- Ragavan, Nimble Pilferer
- Reckless Imp
- Riders of Rohan
- Screamreach Brawler
- Sprinting Warbrute
- Treetop Ambusher
- Vaultbreaker
- Zurgo Bellstriker

This is the baseline family size, not a claimed verified post-change support improvement until the coordinator runs the tests and corpus probe.

## Regression coverage

- Typed identity recognition, case/whitespace normalization, exact payment-query matching, and unknown-cost negative control.
- All 21 full frozen cards through direct runtime conversion and JSON-serialized, validated, registry-materialized artifacts, checking typed payment conditions and unchanged support validation.
- Normal casting versus Dash: different actual mana spent, paid-cost facts, haste/summoning sickness, and conditional delayed return.
- Dash does not permit instant-speed casting; a hand with only Dash mana cannot pay the printed cost.
- No return at upkeep; one-shot return at the next end step.
- A Flash creature dashed after the current end step began survives cleanup, retains haste after changing controller, and returns to its owner's hand at the next player's end step.
- A blinked creature has a new object identity, loses Dash haste, and is not moved by the old delayed trigger.
- An intentionally unknown paid condition remains rejected by the unchanged unsupported-content validator.

## Validation

Run serially with the campaign environment:

```sh
source /workspace/shared/ironsmith-card-env.sh
cargo test -p ironsmith-core dash_cost_identity
cargo test -p ironsmith-compiler-runtime --test dash_materialization
```

Worker validation: rustfmt parsed the modified Rust sources; `git diff --check` passed; all 21 fixture names and Oracle texts were verified against the frozen authoritative snapshot/source. No Rust builds or test executions were run in this worktree, per coordinator instruction.
