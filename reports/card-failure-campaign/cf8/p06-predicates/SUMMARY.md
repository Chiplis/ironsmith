# cf8 / p06-predicates — summary

204 cards. Status: 56 source-proposed, 2 already-on-main, 146 blocked, 0 untriaged
(see `ledger.jsonl`). Nothing was built or run; tests in
`crates/ironsmith-compiler-runtime/tests/predicate_fallback_readings.rs` and
`crates/ironsmith-compiler-runtime/tests/event_instead_and_this_way_readings.rs` (fixture
`fixtures/p06_predicate_fallbacks.json.fixture`) are authored and unrun.

## Root cause
The package is a long tail: almost every card has its own predicate surface. The
shared predicate registry (`parse_predicate` → `predicate_readings`) has no reading
for these surfaces, so both "unsupported predicate" and "unsupported intervening-if
predicate in triggered line" are the same failure seen from two entry points. Most
fixes are new readings in `predicate_readings/fallback.rs`, which only runs when no
ranked reading claims the input, so they cannot change cards that already compile.

## Clusters and general fixes
| Cluster | Cards | Fix |
|---|---|---|
| negated-copula | Twinned Vision, Sphinx of Lost Truths, Court of Vantress, Dose of Dawnglow, Luminarch Ascension | `negated_copula`: `<simple subject> isn't/wasn't/aren't/weren't/doesn't have …`, `you're/they're/it's not …`, `<subject> didn't <verb> …` → re-read the positive clause with the shared grammar, wrap `Not`. Subjects are limited to it / you / this, that, the, target, enchanted, equipped + ≤2 words with no relative or prepositional words, so a negation inside a noun phrase never matches |
| cast-from-your-hand | Apex of Power, Transpose (+ Twinned Vision) | `parse_this_spell_was_cast_from_shape` accepts `your <zone>` for "this spell" (advanced.rs) |
| zone-quantity | Visions of Beyond, Jace the Perfected Mind, Nightmares and Daydreams, Sanguine Spy, Tainted Indulgence, Negative Zone Portal, Profane Procession | graveyard-size threshold, distinct mana values in your graveyard, count of source-linked exiled cards (SourceExiled tag, CR 607.2a) |
| player-turn-facts | Timely Reinforcements, Servant of the Stinger, Oko the Ringleader, The Raven Man, River of Tears, Kiora of Salt and Sand, Lunar Convocation | readings that map to existing conditions: less life than an opponent, committed a crime (CR 700.13), a player discarded, played a land, activated a loyalty ability, gained and lost life |
| object-state | Polis Crusher, Arachnus Web, Domestication, Anax, Burn the Impure, Hotshot Investigators, Unyielding Gatekeeper, Gleeful Demolition | monstrous source state (CR 701.37b); possessive "X's power is N" rewritten to "X has power N"; "the creature had …" read as "that creature had …"; "that creature has <keyword>"; "you controlled it/that <type>" → ItMatchedLastKnown(controlled by you) |
| prepared-designation | Paradox Shaper, Stingerquill Voxmancer, Woodwork Prodigy | new `Condition::SourceIsPrepared` + `SourcePredicateAst::SourceIsPrepared` (core, semantic, resolve, engine condition_eval/dependency/text_change_predicates, text rendering); gameplay test on the condition |
| die-result | Dissatisfied Customer, Non-Human Cannonball | `the result is/was N or less` (and `or greater`) on the completed-roll metric |
| would-die-replacement | Void Maw | `If another creature would die, exile it instead` → `SimpleCreature { other }` + `filter.other()` |
| already-on-main | Dragonfly Swarm, Walltop Sentries | merged intervening-predicate cohort |

## Files touched
- grammar: `predicate_phrases.rs`, `predicate_phrases/advanced.rs`, `predicate_phrases/advanced/predicate_readings/fallback.rs`, `keyword_static_lines/exile_replacement_shapes.rs`, `keyword_static/costs_replacements_and_permissions.rs` (one small hunk)
- semantic `ast/predicates/source.rs`; resolve `predicate_conditions.rs`; core `value_model.rs` (one variant)
- engine `condition_eval.rs`, `dependency.rs`, `continuous/text_change_predicates.rs`; text `condition_rendering.rs`

## Risks
- Fallback readings rebuild the positive clause from synthetic word tokens, so card-name surfaces and spans are lost there. That matches the existing `read_side` idiom.
- `Condition::SourceIsPrepared` is a new core variant. Every match site found by grepping the sibling `SourceIsHarnessed` was updated. A sibling package that adds a Condition variant will conflict textually at the same lines (trivial merge).
- Prepared changes do not mark static-condition dependents dirty. Only intervening-if uses the condition today.
- Profane Procession is tested with the front face only; transform needs the back face in the corpus build.
- Some test assertions use Debug substrings (`Fixed(4)`, `MatchedLastKnown`, `Rolled`). If the lowering wraps values differently they may need adjusting after the first run.

## Blocked, grouped by missing mechanic
- **Replacement with a non-exile instead-action** (die → hand / library top / bottom, damage → counters / mill / sacrifice / exile cards, life gain → loss / draw, draw / mill / scry / copy multipliers, regenerate-on-destroy, untap replacement): Gravebane Zombie, Nissa's Chosen, Firestorm Phoenix, Necromancer's Magemark, Ugin's Nexus, Darigaaz, Ravenloft Adventurer, Ravenous Slime, the Holy Nimbus pair, Mossbridge Troll, Lichenthrope, Delaying Shield, Force Bubble, Nefarious Lich, Dralnu, Sekki, Panther Habit, Gloom Surgeon, Crumbling Sanctuary, Plague Drone, Tainted Remedy, Rain of Gore, Lich, Alms Collector, Bruvac, Eligeth, Kenessos, Twinning Staff, Ashiok, Freyalise's Winds, Land Equilibrium, Equal Treatment, Divine Presence, Forethought Amulet, Nine Lives, Szadek, Undead Alchemist, … (ledger `cluster=replacement-or-outcome`)
- **"… this way" outcome queries**: Long Rest, Mysterious Stranger, Demonic Covenant, Game Preserve, Atemsis, Rulik Mons, Break Out, Mr. Foxglove, Nashi, Flood of Tears, Vengeful Rebirth, Chandra Chill of Compliance, Enlightened Confidant, Transcendent Archaic, Blitzwing, Mishra's War Machine, Minion of Leshrac, …
- **Leading-if target player not declared as a target**: Hidetsugu's Second Rite, Vraska Betrayal's Sting
- **Renowned on a referenced object**: Consul's Lieutenant, Enshrouding Mist
- **Spell-sequence die roll result not exported**: Boing!, Clowning Around; Sword of Hours (result vs damage)
- **Others examined**: Kaito Shizuki (short-name source surface), Cut Propulsion ("twice that much"), Gandalf (suspend in exile), Court of Locthwain (duration-led permission body), Discordant Spirit (opponent's-turn condition), Urza's Miter ("it was sacrificed"), Henry Wu (exploited-creature referent), River Song's Diary ("them" antecedent unverified)
- **Untriaged (93)**: singleton predicates not reached in this pass; the ledger notes each one's failing predicate.

## Second pass (after the coordinator's follow-up)

### Generic "instead" replacement (CR 614.1a) — 8 cards
Tainted Remedy, Plague Drone, Crumbling Sanctuary, Force Bubble, Dralnu, Lichenthrope, Szadek, Undead Alchemist.

The engine already had everything except a static ability to install it. `ReplacementAction::Instead(effects)`
runs through `effects/replacement/execute_payload.rs::with_replacement_child`. That gives the program:
- the replaced event as its triggering event, so "that much"/"that many" read `EventValue(Amount)`;
- the affected player as its iterated player, so "that player" works;
- the damage target as its resolved target;
- the event's applied-replacement history, so the replacement can't reapply to what its own program does (CR 614.5).

New pieces (additive):
- core `replaced_event_model.rs::ReplacedEventSpec`, which is DamageToPlayer, DamageToObject or LifeGain, each with source / combat filters;
- an appended payload `StaticAbilityPayload::EventReplacementWithEffects` with its constructor and `try_map` arm, plus `StaticAbilityId::EventReplacementWithEffects`;
- engine `static_abilities/misc/event_replacement_with_effects.rs`: the kind plus `ReplacedEventMatcher`. Unlike the prevention matchers, it also matches unpreventable damage. It is wired with a one-arm hunk in `model_interpreter.rs`. `text_change_statics.rs` holds the payload (no text-change rewriting);
- a lowering arm that binds the iterated player and keeps event amounts;
- grammar `keyword_static/event_instead_replacements.rs`. It reads "If <damage to X | X would be dealt damage | X would deal [combat] damage to <player> | <player> would gain life>, <program> instead", with "instead" either leading or closing the first sentence. It declines bodies naming prevention, damage, doubling, "plus", gain, may, or +1/+1 on the source, which belong to the specialized readers. It also declines any body it cannot parse, so it never adds a diagnostic to lines other readers own.

### Prevention follow-up programs — 2 cards
Gloom Surgeon, Nine Lives. `parse_prevention_proposed_amount_follow_up_line` now accepts any complete follow-up program after "prevent that damage and" or "prevent that damage,". It declines pronoun, choice, reflexive, damage, remove and +1/+1 tails, which are owned by the put-counter and remove-counter readers.

### "This way" results — 8 cards
Long Rest, Flood of Tears, Vengeful Rebirth, Transcendent Archaic, Mr. Foxglove, Blitzwing, Rulik Mons, Break Out.
Counts use the existing `parse_prior_effect_aggregate_metric_value` grammar, which produces `PendingPriorEffectMetric{AffectedObjects, Count, action, filter}`. Reference resolution binds that to the producing instruction by its action. "No life is lost this way" reads the `Outcome, LifeLost` metric. "You didn't put a card onto the battlefield this way" is `Not(PlayerTaggedObjectMatches(It on battlefield))`.

### Other additions — 3 cards
- Case of the Gateway Express: creatures attacked this turn, via `TurnHistoryCount::CreaturesAttackedWith`.
- Smirking Spelljacker: "a card is exiled with it".
- Archangel of Wrath: "kicked twice" is `KickCount >= 2`.

### Triage of the former 93 untriaged
Each one now has a precise gap in the ledger. They are mostly singletons that need one of:
- **Combat or turn history the engine doesn't keep:** this-combat attacks, last-turn damage, excess-damage history, damage dealt by a source to an object.
- **Results of an earlier optional or choice step:** "if a player does", "if you pay", "if you can't", any-player payments.
- **Leading-condition targets:** Blood Lust, Guiding Spirit.
- **Facts recorded about a spell or ability at cast or activation time:** colors spent to activate, the sacrificed-cost record, life paid, the loyalty cost paid, warp / web-slinging / bargain / gift.
- **Elliptical conditions:** "If it doesn't", "If it is".
- **Repeat-process loops:** Sin, Rally the Horde.
- **Graveyard order adjacency:** "directly above".
- **Mechanics outside constructed play:** sticker kind, draft guessing.

### Additional risks
- A new core payload variant and a new `StaticAbilityId` were appended at the enum ends to keep ordinals stable. Sibling packages that append too will conflict textually at the enum tail, the `try_map` arm list and the id classification guard. These are trivial merges.
- `event_instead_replacements` overlaps the "If ... would ..." lines of existing readers. Overlaps are avoided by declining their vocabulary. Because the static registry runs every rule and reports differing results as ambiguous, the first corpus run must check that no previously compiling "would ... instead" card changed.
- Lich, Delaying Shield and Nefarious Lich now have their replacement lines, but they stay blocked on other lines. The ledger names each one's remaining gap.
