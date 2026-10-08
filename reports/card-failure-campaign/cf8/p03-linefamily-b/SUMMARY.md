# p03-linefamily-b — summary

176 cards, all failing with `parser does not yet support line family`. Ledger:
`ledger.jsonl` (one row per card). Counts: **21 source-proposed**, **2 already-on-main**,
**153 blocked**. Nothing was built or run. The prebuilt `compile_oracle_text` probe was used
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
