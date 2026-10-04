# Typed activation-cost modifiers

Status: fourteen source-complete proposals, **UNVALIDATED**. No compilation, build or test execution. Exact frozen identities and full metadata programs are in `fixtures/typed_activation_modifiers.json.fixture`.

## Cohort

Agatha of the Vile Cauldron; Baru, Wurmspeaker; Belt of Giant Strength; Crown of Gondor; Esquire of the King; Ghostfire Blade; Hylda's Crown of Winter; Loreseeker's Stone; Power Artifact; Razorlash Transmogrant; Sewer Crocodile; Starport Security; Survey Mechan; Thaumaton Torpedo.

The common root is an activation cost modifier whose amount, state predicate, exact preceding ability or attached/target object must survive parsing and pricing. Other clauses use existing executable anthems, equip/attachment, entry triggers, tapping, return-with-counter, damage/draw/life, and token creation. The fixture and full-card artifact tests deliberately retain every clause. No card name selects implementation behavior.

## Typed boundaries

- The extended reader emits existing typed `Value` expressions and `PredicateAst` conditions. It consumes complete generic-mana modifier clauses; colored-pip reductions, ability-subtype taxes, first-activation usage history and grantor-name aliases are outside this cohort.
- `ActivatedAbilityCostCondition::All` is appended after existing variants. A target condition can coexist with the exact preceding-ability identity. Adding one gate no longer replaces another.
- Lowering descends through conditional payloads and all condition operands, binding each newly unbound `ThisAbility` to its preceding activation. It carries that activation's functional zones, including Razorlash's graveyard activation. Unrelated sibling abilities keep their own costs.
- Dynamic reduction values are evaluated relative to the modifier source and its controller. The activation's announced targets remain explicit. Agatha's power must not be replaced by the power of the creature whose ability is priced.
- Generic self-surcharges retain an independent activation-identity gate and authored surface in serde-defaulted optional fields. Loreseeker's hand-count price resolves before combining it with base mana and applying reductions. Existing field/variant ordinals are retained.
- State predicates continue using the established static-condition evaluator. The matching-attacker predicate uses `TurnHistoryCount::CreaturesAttackedWith` snapshots, including the attacking controller and event-time characteristics, so a departed Spacecraft still counts during that turn.
- Each distinct pricing operation starts from the captured raw cost. Existing announcement code re-prices with actual targets and locks the result before payment; this change does not use an already-modified total as a new base. Public-reference announcement and checked resource-query owners remain intact.

Minimum-total-mana-one remains the established per-reducer rule. Generic reductions do not pay or erase colored pips. The existing conservative analytic mana fast path treats compound/unbound modifiers as potentially relevant and falls back to the authoritative planner.

## Authored regressions

`crates/ironsmith-compiler-runtime/tests/typed_activation_modifiers.rs` exercises all fourteen exact full programs directly and through typed-artifact JSON. Gameplay tests use legal activation announcement, actual mana/tap/sacrifice payment and stack resolution. They cover:

- Modifier source versus activated source and opponent ownership/control; multiple reductions, minimum one and mandatory colored mana.
- Target-specific equip pricing, actual attachment and changed-target repricing on a later activation.
- Conditional source-ability identity, sibling exclusion and graveyard operation.
- Monarch, controlled legendary/counter-bearing objects, distinct graveyard mana values and historical attack predicates.
- Real Wurm creation/anthem, Survey's complete damage/draw/life body, tap, return-with-counter and unblockability bodies.
- Repeated Loreseeker hand-count surcharges, canceled payment rollback and read-only repeated legality queries.
- Old artifact payloads omitting the new optional increase fields.

Grammar tests cover every shared predicate/value shape, conjunction retention, complete-consumption negatives, and replace the previous expected rejection of a now-supported conditional reduction.

Deferred command:

`cargo test -p ironsmith-compiler-runtime --test typed_activation_modifiers -- --nocapture`

These are source proposals, not measured recoveries. The campaign's shared build/test/replay remains deferred by user instruction.
