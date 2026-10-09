# cf8 / p06-predicates — summary

204 cards. Status: 60 source-proposed, 2 already-on-main, 142 blocked, 0 untriaged
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

## Round 3

### Instead replacement extended to more events
`ReplacedEventSpec` now also covers:
- **LifeLoss**, through the existing `WouldLoseLifeMatcher`;
- **Destroy**, through `WouldBeDestroyedMatcher`. The destruction owner binds the permanent as `__it__` and as the program's target;
- **ZoneChange**, for "would die" and "would be put into a graveyard [from the battlefield / from anywhere]", through `WouldChangeZoneMatcher`.

Two supporting changes:
- **Engine fix (`events/processing/mod.rs`):** an instead-program on a zone change outside a draw continuation now binds the object that would have moved as `it` / `__it__`, as the draw branch already did. Without this, "put it on top of its owner's library instead" had no object.
- **Lowering:** sets the program's antecedent to that tag for Destroy and ZoneChange events.

Grammar ownership:
- The reader defers to the exile-instead readers and the "reveal it and shuffle it into its owner's library" reader.
- It declines regeneration programs, because regeneration is itself the destruction replacement (CR 701.19).

New source-proposed cards: Gravebane Zombie, Nissa's Chosen, Necromancer's Magemark. Ugin's Nexus, Darigaaz and Firestorm Phoenix now have their would-die lines but stay blocked on other lines.

Not done: would draw, mill and scry. Draw already has its own owner (`DrawReplacementWithEffects`), and the mill/scry/surveil cases in this package and its dependants are count modifications, not instead programs.

### Dependants in other packages
Recorded in the Tainted Remedy ledger note:
- **Now expressible:** Crackling Emergence and Harmonious Emergence (p01), via the Destroy instead program.
- **Need an owner that isn't built yet:**
  - p04's spell-cast-this-way graveyard replacements need a one-shot replacement on one specific spell.
  - Pulmonic Sliver needs a granted, optional zone-change replacement.
  - Many p02/p03/p07 cards are damage, counter, energy or draw count modifications or redirections, not instead programs.
- **Wrongly attributed to p06:** Gideon's Triumph, Epicenter, Orim's Touch and Archmage's Newt are resolution-time "X instead if Y" spell text, not replacements.

### Own remaining clusters
- **Choice results:** "If no player does" now reads as did-not and "If a player does either" as did, in the if-result grammar. Distant Memories and Worms of the Earth stay blocked on their any-player choice bodies. "If you pay" after "unless you pay" is not mapped: whether that result records the payment or the punished action is ambiguous.
- **Cast records:** "{C} wasn't spent to cast it" now reads as the negated mana-spent check (Wumpus Aberration). Bargain (Rowan's Grim Search) is blocked on its comma split, not on the predicate.
- **Targets inside a leading "if"** (Blood Lust, Hidetsugu's Second Rite, Vraska, Guiding Spirit) and **elliptical conditions** ("If it doesn't", "If it is"): still blocked. Both need sentence-level work — declaring targets from a condition, or carrying the previous conditional's predicate to the next sentence — that I couldn't verify without a build.

## Round 4

Status now: 71 source-proposed, 2 already-on-main, 131 blocked (204 cards). Nothing
was built or run. Tests are authored and unrun. Their fixture is
`fixtures/p06_round4.json.fixture`.

### Mechanisms built

1. **Amount-modifying replacements (CR 614.1a, 616.1).**
   - New core payload `StaticAbilityPayload::EventAmountReplacement { event: AmountEventSpec, modifier: AmountModifierSpec, optional, display }`, in `core/src/amount_replacement_model.rs`. It is appended, along with a new `StaticAbilityId`.
     - Events: `Damage { source, player, object, combat_only, minimum }` and `KeywordAction { action, performer }`.
     - Modifiers: `Multiply`, `Add`, `SetTo` and `Half { round_up }`. `Half` maps to the new `EventModification::Halve`, which has arms in all three exhaustive sites.
   - The engine kind (`static_abilities/misc/event_amount_replacement.rs`) emits `ReplacementAction::Modify`. Several modifiers therefore compose in the affected player's chosen order. The damage matcher reuses `DamageAmountReplacementMatcher` and adds a minimum-amount gate.
   - Scry and surveil now propose their number through the shared keyword-action envelope (`execute_keyword_action_with_outputs`), as heal, connive and earthbend already do. This means "would scry" instead programs (`KeywordActionReplacement`) also apply now.
   - Mill gets `KeywordActionKind::Mill`, appended. It is proposed only for replacement: the mill freezes its cards first, then the proposed number may shrink the frozen set or extend it further down the library. A cheap pre-check skips the pass when no replacement could watch a keyword action.
   - Energy plus-N reuses `AddCountersPlacementReplacement` through a new shape in `replacement_facts.rs`.
   - Grammar is in `keyword_static/event_amount_replacements.rs`. It handles:
     - mill/scry/surveil "twice / N times / that many plus N";
     - "draw that many cards instead" (Eligeth);
     - "You may look at an additional N cards each time you surveil";
     - damage "N or more ... deals M instead";
     - "half that damage, rounded down".
   - Resolving damage multipliers:
     - "Until your next turn, if a source/that creature would deal [combat] damage to that player or a permanent that player controls / one of your opponents, it deals double/triple that damage instead."
     - The registration fixes "that player" / "that creature" as it resolves (`register_damage_multiplier.rs`). Lowering binds "that player" to the ability's player antecedent.
     - "this turn" multipliers accept "a source you control" and the recipient-after-duration order.
2. **A spell cast this way, then into a graveyard (CR 614.1a).** "If a spell cast this way would be put into your graveyard, exile it instead" binds to the tag of the statement's this-turn cast permission. It produces a future zone replacement (stack → graveyard ⇒ exile) lasting until end of turn. This is a rider binder in `effect_sentences/cast_spell_graveyard_rider.rs`.
   - "If that spell would ..." already read through the anaphoric spell path and is left alone.
   - The other cards in this family (Sorcerous Squall, Kylox, Gale, Mavinda, Bösium Strip) fail on their *cast* clause (play-from-graveyard/exile variants, owned by p05/p04), not on the rider.
3. **Granted optional replacement.** `EventReplacementWithEffects` gains `optional` (serde default). For a source's own zone change, "you may <program> instead" becomes `ReplacementEffect::optional()`; declining lets the event happen. Pulmonic Sliver's quoted grant reads through the existing quoted-static grant path.
4. **Targets inside a leading "if" (CR 601.2c, 608.2b).**
   - "If target <object> has <quality>, ..." declares the target first, then tests `TargetMatches(<noun> with <quality>)`. This is in `grammar/effects/leading_condition_targets.rs`.
   - "target player/opponent has <cmp> life" reads as `LifeTotal(Target(..))`, which the existing life-condition prelude declares.
5. **Elliptical conditions.** "If it doesn't, ..." / "If it isn't, ..." becomes the false arm of the immediately preceding conditional. The fallback's "it" is bound to the condition's object (`effect_sentences/elliptical_conditions.rs`). Positive "If it does" is left to the result readers.
6. **"If you pay" after "unless you pay" (CR 118.12).** `UnlessPaysEffect` reports a paid cost as a `Declined` outcome (punishment prevented) and an unpaid one as the punishment's own outcome. So the follow-up is `IfResult(WasDeclined)`; `Did` would read the opposite. This applies only when the immediately preceding effect is `UnlessPays` by you (`effect_sentences/unless_payment_results.rs`).
7. **"Do X instead if Y" inside one resolution.** The instead classifier treated any "would" before "instead" as a future replacement. Orim's Touch's "prevent the next 4 damage that would be dealt ... instead" was therefore read as a replacement, and the marker was dropped. Now a "would" counts only inside the conditional head (before its comma) when a leading "if" exists (`grammar/effects/instead.rs`).
   - Gideon's Triumph, Epicenter and Archmage's Newt were not changed. Their dropped "instead" sits in the line-level self-replacement assembly (`semantic_line_parsing/lines.rs`, `dispatch_entry.rs` flat-statement guards), and without a probe I could not locate which reader loses it.
8. **Turn-scoped control-changing entry.** "If a creature would enter under an opponent's control this turn, it enters under your control instead" (Gather Specimens), and the token-creation form (Crafty Cutpurse), now read as an until-end-of-turn `RegisterEnterUnderControlReplacement`. The old reader required the word "opponent", not the possessive.

### Dependants in other packages now expressible (their ledgers are untouched)
Izzet Generatorium (energy plus one), Ghosts of the Innocent (half, rounded down),
Enhanced Surveillance (optional surveil +2), Pulmonic Sliver, Orim's Touch, Lightning,
Army of One and Jeska, Thrice Reborn (multiplier line only; their other lines are
unverified), Isengard Unleashed, Insult // Injury (Insult half), Crafty Cutpurse,
Gather Specimens. Still blocked:
- Sorcerous Squall, Kylox's Voltstrider, Gale, Mavinda, Bösium Strip: blocked by their cast clauses.
- Zabaz: needs a cause filter (modular ability).
- Worship and Elderscale Wurm: need a damage-result life-floor that keeps lifelink at full damage.
- Alms Collector and Reed Richards: draws are per-card events.
- Twinning Staff: copy count.
- Equal Treatment: needs a resolution-registered amount effect.
- Overblaze and Impulsive Maneuvers: need a target-scoped or next-time multiplier.
- Kor Chant, Kor Dirge, Eye for an Eye: redirection, owned by p03.

### Risks
- **Scry and surveil now go through the keyword-action envelope.** It processes the proposed event (one game clone, as for heal and connive) and can suspend for a replacement-order choice before the cards are looked at. Existing "would scry" `KeywordActionReplacement`s, which were inert before because the event was completion-only, now apply.
- **Mill runs a replacement pass when any keyword-action replacement might exist.** The pre-check scans live replacements and ability-generated ones. Instead programs on Mill fail closed with an internal error; no reader produces them.
- **New variants:** `KeywordActionKind::Mill`, `StaticAbilityId::EventAmountReplacement`, `StaticAbilityPayload::EventAmountReplacement` and `EventModification::Halve`, all appended. There are exhaustive arms in `event_model.rs` (2), `static_ability_id.rs`, `static_ability_model.rs` (`try_map`), `text_change_statics.rs`, `model_interpreter.rs`, `application.rs` (2) and `damage_result_modification.rs`.
- **`EventReplacementWithEffects` gained a field.** Its explicit struct patterns are updated in core `try_map`, the lowering arm and the engine interpreter.
- **The instead-classifier change affects every follow-up sentence with a leading "if" and a later "would".** The first corpus run should diff "instead" cards.
- **The rider binder, elliptical merge and unless-payment binder run in the sentence loop** before ordinary parsing. They claim only exact shapes after the right antecedent effect.
- **`register_damage_multiplier.rs` now fixes `IteratedPlayer`, `Target`, `TaggedPlayer` and single-tag object filters as it resolves.** Existing registrations used only class filters (`You`, `Opponent`, `Any`), which pass through unchanged.
