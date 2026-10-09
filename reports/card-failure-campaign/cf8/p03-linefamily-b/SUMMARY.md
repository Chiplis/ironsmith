# p03-linefamily-b — summary

176 cards, all failing with `parser does not yet support line family`. Ledger:
`ledger.jsonl` (one row per card). Counts (after round 4): **54 source-proposed**, **2 already-on-main**,
**120 blocked**. Nothing was built or run. The prebuilt `compile_oracle_text` probe was used
until it disappeared mid-session; later clusters rely on reading the code only.

## Clusters fixed (source-proposed)

| Cluster | Cards | Root cause | General fix | Test |
|---|---|---|---|---|
| supertype-landwalk | Livonya Silone, Ayumi, Zombie Musher | `LandwalkKind` had no legendary/snow-land variants (CR 702.14c) | New `LandwalkKind::{LegendaryLand, SnowLand}` in core and engine. Added the keyword phrases, the blocking check (current supertypes) and every match site (grammar, lowering, engine builders, interpreter, text changes, game_state permission) | `supertype_landwalk_variants.rs` |
| shadow-block-blocker-side | Heartwood Dryad, Wall of Diffusion | The shadow-block permission accepted only the attacker-side wording ("as though they didn't have shadow") | Accepts "as though it had shadow" as the same CR 702.28b permission | `shadow_block_as_though_had_shadow.rs` |
| unreachable-filtered-etb-replacement | Gond Gate, Phyrexian Censor, Bard Class | The grammars existed, but `static_ability_rule_head_hints` listed only a fixed set of subject heads | The 3 filtered-ETB rules are now whole-line. They decline bare pronoun subjects, and the untapped form declines subjects it can't parse | `open_subject_filtered_entry_replacements.rs` |
| graveyard-cast-trailing-condition | Oathsworn Vampire, The Indomitable, Ebondeath, Undead Sprinter | Only the leading "As long as X, you may cast this card from your graveyard" form existed | New production reads a trailing `if`/`as long as` condition into the same `ConditionalStaticAbility(Grants PlayFrom)`. Added a general "<object filter> died this turn" gate → `TurnHistoryCount::Died(filter) >= 1` (CR 700.4). The "If you do, this creature enters with a counter" rider goes through the existing entry-counter grammar | `source_graveyard_cast_trailing_conditions.rs` |
| counted-keyword-actions | Ethereal Ambush, Step Right Up | Manifest and open-Attraction accepted only singular counts | "Manifest the top N cards" and "Open N Attractions" lower to `RepeatEffects(N, single action)` (CR 701.40c) | `counted_manifest_and_attraction_opening.rs` |
| qualified-equip | Team Pennant | The equip qualifier lacked "creature token" | Qualifier `token` → target filter `token = true` | `token_equip_qualifier.rs` |
| subtype-retrace-grant | Deeproot Historian | The retrace fact accepted only instant/sorcery subjects and only from those heads | Subtype-list subjects (a union through `ObjectFilter.subtypes`); mixed type+subtype lists are rejected; the rule is now whole-line | `subtype_retrace_grants.rs` |
| attached-attack-as-though-haste | Instill Energy | No attached production for the existing `CanAttackAsThoughHaste` | `AttachedStaticAbilityGrant` for enchanted/equipped creature | `attached_attack_as_though_haste.rs` |
| unreachable-skip-upkeep | Gibbering Descent | `parse_players_skip_upkeep_line` already reads labeled conditional "Skip your upkeep step if ...", but its derived head was only `players` | Made the rule whole-line | `conditional_skip_upkeep_reachability.rs` |
| skip-untap-step | Stasis | No static for skipping untap steps | New typed `PlayersSkipUntapStep{player}` (core id+payload, engine kind, interpreter, compiler_model, text-change rewrite, `player_skips_untap_step`, turn runner + `execute_untap_step_with` skip, CR 614.10); grammar for 'Players skip their untap steps' / 'Skip your untap step' | `skip_untap_steps.rs` |
| scoped-mana-spend | Oath of Nissa, Quicksilver Elemental | The any-color spend grammar lacked "to cast <filter>" and the one-color source-activation form | New shapes → `any_color_for_casting_matching` and `ActivationCostsOf(source)` + `any_color_mana_symbol` (CR 609.4b) | `scoped_mana_spend_permissions.rs` |

Already on main through merged PR source: Summer Bloom (`temporary_additional_land_caps.rs`) and Rukarumel, Biologist (`chosen_type_domain_regressions.rs`).

## Files touched
- core: `static_ability_model.rs` (+2 constructors, +2 keyword strings), `static_ability_model/grants.rs` (+2 variants)
- engine: `static_abilities/{combat,mod,model_interpreter}.rs`, `rules/combat.rs`, `cards/builders.rs`, `game_state.rs`, `continuous/text_changes.rs` (landwalk only)
- grammar: `keyword_static/{mod,etb_static_lines,anthem_grant_conditionals,costs_replacements_and_permissions}.rs`, `grammar/keyword_action_costs.rs`, `activation_and_restrictions/{keyword_action_costs,keyword_activated_lines}.rs`, `grammar/shared_util/reference_shapes/reference.rs`, `grammar/filters/predicate_phrases/advanced/phase_step_gates.rs`, `grammar/keyword_static_lines/{permission_counter_shapes,grants_and_permissions}.rs`, `grammar/effects/clause_pattern_shapes/keywords.rs`, `effect_sentences/clause_pattern_helpers.rs`, `grammar/keyword_activated_lines/equip.rs`, `grammar/static_keyword_facts/late.rs`, `static_ability_helpers.rs`
- lowering: `lowering_impl/{runtime_static_ability_helpers,lowering_support}.rs` (landwalk arms)

## Risk notes
- **Whole-line rules.** Five rules are now whole-line: the 3 filtered-ETB rules, retrace, and skip-upkeep. They now run on every static line. Each grammar is anchored, but the registry treats different readings of the same line as ambiguous. Watch for new ambiguity or error diagnostics on lines that start with other heads, especially "It/They enter tapped", which the tapped rule now explicitly declines.
- **Died gate.** The "died this turn" gate only fires when the line ends in exactly `died this turn` and is not the bare `a creature`. It returns a `ValueComparison` predicate. Check that `parse_static_condition_clause` reaches the phase-step gate registry for static conditions.
- **Trailing-condition production.** It runs early in `parse_static_ability_ast_line_lexed_single`, and only for "you may cast this card from your graveyard if/as long as ...". Gravecrawler-style lines are still claimed earlier by the `graveyard-cast-control-condition` line family.
- **Card text unchecked after the probe binary vanished.** Quicksilver Elemental's line 1 and the other lines of the scoped-mana-spend, retrace, haste and skip-upkeep cards were never checked against the binary.
- **Merge conflicts.** Additive edits to shared hot spots (`costs_replacements_and_permissions.rs`, `model_interpreter.rs`, `static_ability_model.rs`, the `keyword_static/mod.rs` head-hint table) may conflict with sibling packages. The hunks are small and appended.

## Blocked, grouped by missing mechanic
- **Friend-or-foe partition (5).** Khorvath's Fury, Pir's Whim, Regna's Sanction, Virtus's Maneuver, Zndrsplt's Judgment.
- **Keyed target groups, per color or per player (5).** All Suns' Dawn, Rogues' Gallery, Windgrace's Judgment, Guff Rewrites History, Face Yourself.
- **Prevention follow-ups, filters and divided shields (14).**
  - Follow-ups and filters: Channel Harm, Comeuppance, Judgment of Alexander, Samite Ministration, Refraction Trap, Inspire Awe, Undergrowth, Well-Laid Plans, Pollen Lullaby, Revealing Wind, Pay No Heed.
  - Divided shields: Embolden, Pollen Remedy, Remedy.
- **Chosen-source redirection (4).** Eye for an Eye, Harm's Way, Reflect Damage, Shining Shoal.
- **Once-per-turn cast permissions with new filters (5).** Vision, Zaffai, Arcade Gannon, Banon, Maralen.
- **Cast-permission additional costs (4).** Falco Spara, Into the Pit, Noctis, Quilled Greatwurm.
- **Conditional self-flash tied to casting choices (5).** Molten Exhale, Quantum Reduction, Silver Scrutiny, Tegwyll's Scouring, The Blue Spirit.
- **Combat rule variants.**
  - Block while tapped: Masako.
  - Attack only alone: Master of Cruelties.
  - Exact block-count requirements: Nacatl War-Pride, Gorm.
  - Attack limits: Eternal Wanderer, Tomik.
  - Assign damage as though unblocked: Ruxa, Outmaneuver.
  - Divided combat damage: Butcher Orgg.
  - Block control: Invasion Plans.
  - Remove from combat and reblock: False Orders.
- **Cost modifiers.**
  - Colored this-ability reductions: Flying Drone, Kami.
  - First-each-turn reductions: Hojo, Tezzeret, Ranar.
  - Plot and unlock costs: Doc Aurlock, Inquisitive Glimmer.
  - Commander tax: Myth Unbound.
  - Loyalty cost: Carth.
  - Mana-ability life cost: Thran Portal.
  - Additional cost per mana symbol: Drought.
- **Casting restrictions.**
  - Own-turn casting: City of Solitude, Dosan, Fires of Invention.
  - Shared-color restriction: Mana Maze.
  - Died-this-turn cast restriction: Grim Wanderer.
  - Land-play cross restriction: Rock Jockey.
  - Zone-only casting: Haakon.
- **Granting keywords to spells or cards.** Ashling (evoke), Molecule Man (miracle), Ian Chesterton (replicate), The Twelfth Doctor (demonstrate), Weftwalking (first spell free).
- **Chosen-ability, copy and name mechanics.** Greymond, Koh, Metamorphic Alteration, Spy Kit, Pin Collection (stickers).
- **Attached composites.** Bewitching Leechcraft, Bonds of Faith, Snowblind, Street Savvy, Nim Deathmantle, Eidolon of Countless Battles.
- **Doctor Who / Warhammer 40,000 / Final Fantasy labeled bodies.**
  - Villainous choice: The Master, Midnight Crusader Shuttle.
  - Additional upkeep step: Ninth Doctor.
  - Grant suspend: Eleventh Doctor.
  - Per-player "who does": Second Doctor.
  - Capped pay-X: Mortarion.
  - Secret vote: Círdan.
  - Random opponent: Knight Rampager.
  - Attack requirement: Galactus.
  - Once-per-turn copy for another player: Lucy MacLean.
  - Pay life to cast: Anrakyr.
  - Additional combat with untap: Swinging Ship.
  - Reveal until X nonland cards: Sanar.
  - Optional counter-removal cost: Hierophant Bio-Titan.
- **Turn structure.**
  - Skip untap steps: Stasis.
  - Extra-turn riders: Alchemist's Gambit, Savor the Moment.
  - Two-turn restrictions: Peace Talks.
  - Lose-game replacement: Stunning Reversal.
  - Turn-order choices: Sadistic Shell Game.
- **Miscellaneous.**
  - Draft: Cogwork Tracker, Agent of Acquisitions.
  - Loyalty twice per turn: Oath of Teferi, Urza.
  - Coin-flip loops: Game of Chaos, Odds // Ends.
  - Earthbend riders: Earthshape, Rockalanche.
  - All basic land types: Energybending.
  - Per-kind counters: Blue, Loyal Raptor.
  - Escape riders: Skyway Robber, Polukranos.
  - Mana spent as a value: Verazol.
  - Dynamic echo: Volcano Hellion.
  - Tiered: Vincent's Limit Break.
  - Exchange of control: Juxtapose.
  - Controlling a player: Secret of Bloodbending.
  - Mutate from graveyard: Brokkos.
  - Special-action discard: Circling Vultures.
  - Wishes: Death Wish, Extrapolate the Impossible.
  - Surveil replacement: Enhanced Surveillance.
  - Free-cast sets: Finale of Promise, Invoke Calamity.
  - Opponents' face-down look: Found Footage.
  - Source-exiled land play: Hedonist's Trove.
  - Retarget: Sideswipe.
  - Mana-spent conditions: Moonhold.
  - Reveal draws: Booby Trap.
  - Token replacement: Esix.
  - Draw replacement: Reed Richards.
  - Monarch control: Fealty to the Realm.
  - Owner chooses top or bottom: Endless Detour.
  - Discover by owner: Zoyowa's Justice.
  - Random graveyard: Search for Survivors.
  - Remove any counters: Eventide's Shadow.
  - Redistribute life: Reverse the Sands.
  - Face-down entry counters: Veiled Ascension.
  - Mana-spent devotion: Altar of the Pantheon.
  - Cast-time X definition: Spoils of War.
  - Exiled-card permission: Null Summoner.
  - Support X: The Crowd Goes Wild.
  - Equip planeswalker: Luxior.
  - Specific-color mana: Sunglasses of Urza.
  - Ownership-scoped mana: Nathan Drake.
  - Ability copy with exclusion: Sharkey.

## Ownership notes (after the shared-mechanism map)
- scoped-mana-spend (Oath of Nissa, Quicksilver Elemental) was written before the ownership map
  assigned spend-as-any-color to **p05**; reconcile at merge and keep one implementation.
- Blocked rows that depend on other packages' mechanisms say so in `gameplay_gap`
  ("needs mechanism owned by pNN"): p06 replacements (redirection, energy/draw/token/surveil
  replacements, lose-game), p10 cast/player restrictions, p05 play-from-exile/cast permissions and
  attack requirements, p09 friend-or-foe/votes/villainous choice/chosen abilities, p12 ability
  copying, p01 random choices.
- New engine static `PlayersSkipUntapStep` changes the engine schema hash; artifact fixtures that pin
  ENGINE_SCHEMA_HASH will need regeneration.

## Owned shared mechanisms (assigned after triage) — work done

| Mechanism | Cards | Change | Test |
|---|---|---|---|
| Chosen-source all-damage prevention | Pay No Heed (+ Auriok Replica, Prahv, Rith's Charm outside package) | Active-voice "a source of your choice would deal [to you] this turn" -> PreventAllDamageEffect target All/You + source choice | `chosen_source_all_damage_prevention.rs` |
| Chosen-source finite shield + "prevented this way" rider (CR 615.5/615.7) | Refraction Trap | Active-voice "prevent the next N damage that a source of your choice would deal to ..." in the finite-shield grammar; existing reflect rider attaches | `chosen_source_shields_and_redirects.rs` |
| Damage redirection (CR 614.9) | Reflect Damage, Harm's Way, Shining Shoal | "that source's controller" destination; bounded redirection **extended** (not forked) with serde-default `source_of_your_choice` + `protect_you_and_permanents` (core+engine+interpreter+renderer), chosen-source replacement matcher | `chosen_source_shields_and_redirects.rs` |
| Divided prevention (CR 601.2d / 615.7) | Embolden, Remedy | PreventDamageEffect `divided` flag drives the existing cast-time distribution announcement; one shield per target share | `divided_damage_prevention.rs` |
| Per-color target groups (CR 115.3) | All Suns' Dawn, Rogues' Gallery | "For each color, ... target X of that color ..." expands to five color-qualified target instances | `per_color_target_groups.rs` |
| Once-per-turn cast permissions | Zaffai, Vision | Usage-limited free hand-cast grant with spell filter; one-shot decline exempts usage-limited grants | `once_per_turn_hand_free_casts.rs` |
| Casting restrictions | Grim Wanderer, Dosan | New cast-restriction label/condition "if a creature died this turn"; non-active-player casting restriction | `died_this_turn_cast_restriction.rs`, `own_turn_casting_restriction.rs` |
| Whole-line rule regression guard | — | Lines with other heads keep their readings | `whole_line_static_rule_regressions.rs` |

Still queued in owned mechanisms (blocked, with gaps in the ledger): prevention follow-ups that
declare fresh targets or branch on source type (Channel Harm, Comeuppance, Judgment of Alexander,
Samite Ministration), filtered/excepted combat prevention (Inspire Awe, Undergrowth, Well-Laid
Plans), kicker-amount override of a divided shield (Pollen Remedy), Eye for an Eye (non-preventing
mirror), per-player target groups (Windgrace's Judgment, Guff Rewrites History, Face Yourself — need a
runtime target group keyed by player, CR 601.2c), friend-or-foe (5), remaining once-per-turn
permissions (Arcade Gannon, Banon, Maralen), cost-modifier variants (colored this-ability, first
each turn, plot/unlock, commander tax, loyalty, mana-ability life, Drought), keyword grants to spells,
City of Solitude (needs a player-scoped all-abilities restriction incl. mana abilities and
off-battlefield abilities), Fires of Invention (two-spell cap), then the labelled Doctor Who /
Warhammer bodies.

Risk notes for the new work: engine schema changes (`RedirectNextDamageToTargetEffect`,
`PreventDamageEffect`, `ThisSpellCastCondition`) are serde-default/additive; the per-color expansion
synthesizes color word tokens before the ordinary target grammar; `prevention_helpers` became
`pub(crate)` so the redirection executor can reuse the source chooser.

## Round 3 (on cf8/integration)

| Mechanism | Cards | Change | Test |
|---|---|---|---|
| Per-opponent target groups (CR 601.2c) | Windgrace's Judgment | "For any number of opponents, <verb> target X that player controls" -> any number of targets "controlled by different players" (existing set constraint) | `per_player_target_groups.rs` |
| Conditional Fog exception | Undergrowth | "If <pred>, this effect doesn't affect combat damage that would be dealt by <color> creatures" -> Conditional(pred){combat prevention from non-<color> creatures}{Fog} | `conditional_fog_exceptions.rs` |
| Friend-or-foe (Battlebond) | Virtus's Maneuver, Pir's Whim, Zndrsplt's Judgment | New `ChooseFriendsOrFoesEffect` (core, engine executor, decoder family+payload+card-graph, materializer, interpreter direct list, text, effect-registry.tsv, runtime-audit contracts) + `EffectAst::ChooseFriendsOrFoes` (visit/resolve/lowering arms) + tags `Friends`/`Foes`; "Each friend/foe <verb>" = the "Each player <verb>" reading as `ForEachTaggedPlayer` | `friend_or_foe.rs` |
| Fog with excepted sets | Inspire Awe | "... except combat damage that would be dealt by enchanted creatures and enchantment creatures" -> complement filter (without Aura attached, not Enchantment) | `conditional_fog_exceptions.rs` |

Mana-spend dedupe: on the merged tree the static spend-as-any-color shapes exist only in
`grants_and_permissions.rs` (this package). p04/p05 added resolving/tagged-play spend riders
(`mana_replacement.rs` temporary symbol permission, `tagged_surface.rs`), which are different scopes,
so there is one implementation per scope and nothing to remove.

"needs X (pNN)" recheck: Knight Rampager and Galactus re-pointed at p05's merged MustAttackPlayer
(remaining gaps: chosen-player binding; most-life selector + named-creature gate). Other owner
dependencies unchanged.

Next in the owned queue (not started): Khorvath's Fury / Regna's Sanction group bodies, Guff per-player mandatory groups,
prevention riders that pick targets/branch on source type (Channel Harm, Comeuppance, Judgment of
Alexander, Samite Ministration), Pollen Remedy, Eye for an Eye (p06 replacement), cost-modifier
variants, keyword grants to spells, City of Solitude, Fires of Invention, labelled Doctor Who /
Warhammer bodies.

## Round 4

| Mechanism | Cards | Change | Test |
|---|---|---|---|
| Friend-or-foe bodies | Khorvath's Fury, Regna's Sanction | Recipient reading: "... to each foe/friend ..." is read as its "each opponent"/"each player" form and iterates the tagged group (per-player hand-size damage keeps IteratedPlayer). Choice-complement program gained a tap disposition ("chooses one untapped creature they control, then taps the rest") | `friend_or_foe.rs` |
| Prevention riders on the prevented source (CR 615.5) | Channel Harm, Comeuppance, Judgment of Alexander, Samite Ministration | New `effect_sentences/prevention_source_riders.rs`: "If/Whenever damage [from a <quality> source] is prevented this way[ this turn], <body>" with bodies "[you may have] ~ deal that much damage to <target / that creature / the source's controller>", "you gain that much life", "each <filter> deals damage equal to its power to that creature". Quality gate = TaggedMatches(triggering). `PreventAllDamageToTargetFromSourceFilter` gained `follow_up_effects`; lowering prepends `TagTriggeringSource` and returns the rider's target choices so the target is announced with the spell (CR 601.2c). Chosen-source shield protecting you now accepts follow-ups. Damage-source sets accept a controller clause after the noun ("sources you don't control / your opponents control"); "you and planeswalkers you control" recipient | `prevention_source_riders.rs` |
| Kicked divided shield | Pollen Remedy | "If this spell was kicked, prevent the next N damage this way instead" -> amount = base + (N-base)*WasKicked; the cast-time distribution announcement now receives the pending cast's optional costs (CR 601.2b before 601.2d) | `prevention_source_riders.rs` |
| Shared-color pair shield | Well-Laid Plans | Persistent shield + "if they share a color": recipient filter carries a SharesColorWithTagged constraint on the damage-source tag, resolved by the damage matcher (source live or LKI, distinct objects) | `prevention_source_riders.rs` |
| First-spell flash + combat entry trigger | The Blue Spirit | Static "You may cast the first <spell> you cast each turn as though it had flash" -> flash timing grant with the first-spell filter; "enters during combat" -> generic ZoneChangeTrigger with DuringCombat timing | `first_spell_flash_and_combat_entry.rs` |
| Gain all basic land types | Energybending | "<lands> gain all basic land types until end of turn" -> AddSubtypes(five basics) | `gain_all_basic_land_types.rs` |
| Per-planeswalker attacker cap | The Eternal Wanderer, Tomik | New static `MaxCreaturesCanAttackSourceEachCombat` (id appended), enforced in both attack-declaration validators (CR 508.1c) | `planeswalker_attack_caps.rs` |
| Block while tapped | Masako the Humorless | New static `CanBlockAsThoughUntapped` (id appended) granted to the filter; the three tapped-blocker gates honor it (CR 509.1a) | `block_as_though_untapped.rs` |
| Blocker-side landwalk permission | Street Savvy | New static `CanBlockAsThoughNoLandwalk` (id appended) recognized in the heterogeneous granted tail; `can_block_with_view` skips landwalk evasion for that blocker | `blocker_landwalk_permission.rs` |

Round 4 risk notes:
- **Delayed prevention triggers (Samite Ministration, Judgment of Alexander):** "Whenever ... is prevented this way" is now a delayed triggered ability (CR 603.7). It is registered right after the shield and linked to that shield's id through `DelayedTriggerSpec::DamagePreventedThisWay` and `with_prior_prevention_event_value`. Each matching `DamagePreventedEvent` puts it on the stack with that event's prevented amount. Simultaneous damage merged into one prevention event triggers once.
- **Channel Harm's target** is announced with the spell through the missing-target prelude. The rider then reads it from the shield's stored targets and target assignments.
- **Engine schema hash:** three new `StaticAbilityId`/payload variants (appended) and `PreventAllDamageToTargetFromSourceFilter.follow_up_effects` change the schema. `append_target_distribution_requirements` gained an `optional_costs_paid` parameter (3 callers updated).
- **Apostrophes:** new phrases accept both "didnt"/"didn't" and "source's"/"sources".
- **Unverified lines:** The Eternal Wanderer's loyalty abilities and Tomik's activated ability were not checked against a build.
- **Guff Rewrites History** stays blocked. The one-target-per-player announcement already exists (Vaevictis); the blocker is three correlated result-set sentences (see ledger).
- **Shared hot spots touched with small additive hunks:** `chain_carry.rs` (bind_prevention_followup), `keyword_static/mod.rs` (rule table + head hints), `generic_subject_verb.rs` (complement program), core `static_ability_model.rs`/`static_ability_id.rs`, engine `static_abilities/{mod,combat,model_interpreter}.rs`.

Still blocked, main remaining queues: cost-modifier variants (colored or conditional this-ability reductions, first-each-turn activation/foretell, plot/unlock, commander tax, loyalty, mana-ability life cost, Drought), keyword grants to spells (evoke, miracle, replicate, demonstrate, Weftwalking free first spell), combat variants (block-count requirements, attack-alone-only, divided combat damage, reblock), Doctor Who / Warhammer labelled bodies, and the miscellaneous singletons listed above. Not touched: City of Solitude / Fires of Invention (p10) and Eye for an Eye (p06).
