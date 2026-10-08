# cf8 / p06-predicates — summary

204 cards. Status: 35 source-proposed, 2 already-on-main, 74 blocked, 93 untriaged
(see `ledger.jsonl`). Nothing was built or run; tests in
`crates/ironsmith-compiler-runtime/tests/predicate_fallback_readings.rs` (fixture
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
