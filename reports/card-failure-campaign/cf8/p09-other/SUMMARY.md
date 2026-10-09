# cf8 p09-other — summary

154 cards: 43 source-proposed, 111 blocked, 0 untriaged (round 3). Unvalidated: no builds or test
runs; the prebuilt probe binary was unavailable for the second pass, so proposals are checked only by
reading. One full-card regression test file per mechanism (unrun).

## Round 3 (on cf8/integration)
- Counts: 43 source-proposed, 111 blocked, 0 untriaged.
- Ported new engine executors to the transaction idiom: `SetDayNightEffect` and the unprepare branch of `PrepareEffect` run inside `execute_world_checkpoint_transaction`; added `SetDayNightEffect` to `effect-registry.tsv`.
- **counter-kind** (Dramatist's Puppet, Quarry Hauler): root cause was the generic for-each-object readers (sentence reading part_4 and chain_carry) claiming "for each kind of counter on target permanent" as an object iteration and erroring; they now decline that shape so the existing `ForEachCounterKindPutOrRemove` primitive owns it. Test `counter_kind_put_or_remove.rs`.
- p09 dependants in other ledgers (35 cards, almost all p12): villainous choice is already implemented on main (the p12 blocks are body-specific: "you gain control ... it's attacking", "a token that's a copy of that card", Dalek life loss); same-name filter grammar (`SameNameAsTagged`, "with the same name as") already exists — the blocked bodies need spell/splice/graveyard-relative antecedents; shares-a-card-type for reveal-until/per-opponent discard comparisons needs `ObjectCharacteristicRelation` wiring, not the selection relation added here; friend-or-foe, secret votes and per-player named choices remain open (multi-choice designation design not built this round).
- Still open owned mechanisms: two-color/two-player designations (engine stores one chosen color per object), counter-kind choice referenced by a later put (Aven Courier, Contractual Safeguard), N untap steps (needs a counted `Until`), damaged-by-source-this-turn player filter, lost-the-game count, chroma/bushido per affected object, double-faced predicate, stack-ability source filters, Eye of Yawgmoth/Memories Returning partitions, Tovolar chosen-set transform, foretell from an effect, unblock/re-block, gain suspend (main holds Sinister Concierge deliberately), turn control.

## Second pass mechanisms
- **land-play-ordinal** (Fastbond; fixes a silent miscompile): new `TurnHistoryCount::LandsPlayed(PlayerFilter)` (engine sums `Player::lands_played_this_turn`, CR 305.2); "if it wasn't/was the Nth land you played this turn" -> `LandsPlayed(You) !=/== N` (was "if it wasn't a land"); "any number of lands on each of your turns" -> `AdditionalLandPlays(u32::MAX)`. Collateral: none (corpus grep finds only Fastbond).
- **search-partition** (Fork in the Road, Jarad's Orders; fixes a lossy reading): procedure `search_partition_procedure.rs` — search+reveal group, choose one -> hand, rest -> graveyard/library bottom, optional "Then shuffle". Collateral: none beyond these two (look/reveal-top form already uses `LookedCardDisposition::HandAndGraveyard`).
- **craft slot matching** (Throne): cost preflight requires a distinct-object matching across exile-chosen slots (reuses the discard bipartite matcher, `special_actions.rs`).
- **filter predicates** (The Enigma Jewel, Eye of Ojer Taq): `ObjectFilter.has_activated_ability` (+ "with activated abilities" phrase) and `ObjectFilter.shares_card_type` whole-selection relation; Craft general-filter and "<n> that share a card type" materials.
- **day/night** (Into the Night, Unnatural Moonrise, The Celestus): core `SetDayNightEffect{Day|Night}` end to end + `EffectAst::SetDayNight`; Celestus toggle via existing `ItIsNight` conditional (CR 731).
- **unprepare** (Biblioplex Tomekeeper, Infinite Coursework): `PrepareEffect.unprepare` + `KeywordActionAst::Prepare{unprepare}`; engine `clear_prepared`.
- **animation** (Rude Awakening, Hunting Wilds, Restless Prairie, Primal Adversary): plural "that are still lands" tail; `Subtype::Llama` appended.
- **possessed-threshold** (Possessed ×4): static rule `parse_anthem_color_and_quoted_activated_grant_line`.
- **unblocked-assignment grants** (Predatory Focus, Siege Behemoth): resolving and static grants of `MayAssignDamageAsUnblocked`.
- **conditional destroy** (Aggression, Getaway Glamer): "it didn't attack this turn", "no other creature has greater power".
- **counter type** (Jabari's Influence): `CounterType::MinusOneMinusZero` appended; counter-unless payer "that ability's controller" (Ayesha still blocked on its target filter).
- New tests: craft_filtered_materials, day_night_effects, land_play_ordinals, search_partition_destinations, unprepare_effects, animation_still_lands, possessed_threshold_grants, granted_unblocked_assignment, conditional_destroy_predicates, minus_one_minus_zero_counters (+ craft_material_slots::slot_matching).
- Second-pass risks: new appended enum/field surface needs a build (SetDayNightEffect/DayNightDesignation, LandsPlayed, has_activated_ability, shares_card_type, PrepareEffect.unprepare, EffectAst::SetDayNight, Subtype::Llama, CounterType::MinusOneMinusZero); engine executors use the local pre-refactor idiom (direct GameState calls), not main's in-flight transaction wrappers; Possessed rule could be reported ambiguous if another static rule also matches; unprobed neighbouring lines per ledger `gameplay_gap`.
- Still blocked notes updated in ledger: Tovolar (transform any number chosen), Tizerus (conditional counter choice), two colors (`chosen_colors` holds one color per object), Eye of Yawgmoth ("exile the rest" destination), Winnow (same name relative to target), Ayesha/Abstruse/Peregrine (ability-source filters).

## First pass clusters
- **craft-materials** (Altar of the Wretched, Paleontologist's Pick-Axe, Saheeli's Lattice, Throne of the Grim Captain): Craft reads open-ended "<n> or more <type/subtype>" and per-subtype slot lists, one exile payment per slot (CR 702.167a). Files: grammar `keyword_activated_lines/craft.rs`, `activation_and_restrictions/keyword_activated_lines.rs`, text `single_effects_late.rs`.
- **defending-player-recipient** (Electryte, Latulla's Orders): combat damage recipient → `PlayerFilter::Defending` (CR 506.2). `semantic/semantic_trigger.rs`.
- **anthem-also-adverb** (Jetmir ×2): `parse_anthem_subject` strips "also". `anthem_grant_lines.rs`.
- **assign-damage-pronouns** (Wolverine): personal pronouns accepted. `grammar/abilities.rs`.
- **cost-reduction-restriction-tail** (Radha's Firebrand, The Lonely Mountain): preprocess returns a trailing "Activate only …" to the activated ability (CR 602.5b). `preprocess/line_shapes.rs`, `preprocess.rs`.
- **anthem-player-count** (Blazing Sunsteel): `Dynamic(CountPlayers(Opponent))`; core `anthem_model.rs` admits CountPlayers.
- **day-night-enters** (Vadrik): starts-day recognizers accept "as this (artifact) enters". `semantic_facts.rs`, `statement_shapes.rs`, `semantic_lowering/statement_shapes.rs`.
- **keyword-choice-grant** (Angelic Skirmisher, Linvala, Gabriel Angelfire): new procedure `effect_sentences/keyword_choice_procedure.rs`, registered in `procedures.rs`/`mod.rs`.
- **combat-history-target-count** (Case of the Gorgon's Kiss): `remove_destroy.rs` looks through WithCount/WithCountValue.
- **mixed-target-union** (Coalborn Entity): `target_semantics/reference.rs`.

Tests: `craft_material_slots.rs`, `combat_damage_defending_player.rs`, `anthem_also_adverb.rs`, `assign_damage_pronouns.rs`, `activated_cost_reduction_restriction_tail.rs`, `anthem_player_count.rs`, `day_night_starts_day_named_source.rs`, `keyword_choice_grants.rs`, `combat_history_target_counts.rs`, `mixed_token_player_planeswalker_targets.rs`; helper `p09_common/mod.rs`; fixtures `fixtures/<cluster>.json.fixture`.

## Risks
- Throne slots: fixed in the second pass (distinct matching).
- Gabriel: rampage option and "until your next upkeep" duration unverified.
- Fastbond miscompile: fixed in the second pass.
- Search partition destinations: fixed in the second pass for the search+reveal form.

## Blocked, by missing mechanic (first-pass list; second-pass fixes above supersede it — per-card status in ledger.jsonl)
- New engine effects/designations: day/night set/toggle (Into the Night, Unnatural Moonrise, Tovolar, The Celestus); unprepare (Biblioplex Tomekeeper, Infinite Coursework); foretell from an effect (Ethereal Valkyrie, The Foretold Soldier); unblock/re-block (Balduvian Warlord, Ydwen Efreet); turn control (The Dominion Bracelet); gain suspend (Sinister Concierge); N untap steps (Telekinesis).
- Choice designations: two colors (Seal of the Guildpact, Tablet of the Guilds); two players (Bitter Feud, Sower of Discord); counter kind (Aven Courier, Contractual Safeguard, Dramatist's Puppet, Quarry Hauler); non-controller choices (Choice of Damnations, Master of Ceremonies, Noxious Vapors, Selective Obliteration); votes (Custodi Squire, Vault 11, Illusion of Choice); others (Rite of Ruin, Phyrexian Splicer, Swirl the Mists).
- Missing predicates/values: has an activated ability (Enigma Jewel); shares a card type (Eye of Ojer Taq); player damaged by the source this turn (Wicked Akuba); players who lost the game (Rampant Frogantua); chroma/bushido on affected creature (Light from Within, Takeno); double-faced (Invasion of Pyrulea); same name (Winnow); greatest power (Getaway Glamer); different/same controllers (Cloud's Limit Break, Simic Guildmage); ability source characteristics (Abstruse Archaic, The Peregrine Dynamo); suspended card (Amy Pond).
- Composite static readers: Possessed ×4; assign-damage-as-unblocked grants (Predatory Focus, Siege Behemoth); conditioned grants (Cloud, Kosei); granted ability naming the granter's controller (Hold for Ransom).
- Animation/become: still-a-land (Hunting Wilds, Primal Adversary, Restless Prairie, Rude Awakening); color/type become (Puca's Eye, Foraging Wickermaw, Ageless Sentinels, Mistform Sliver, Traitor's Clutch, Soul Sculptor); dynamic base P/T (Amplifire, Arni Brokenbrow, Sworn Defender, Captain Rex Nebula).
- Other: token copies attached to creatures (Arna Kennerüd, Three Dog); clone exception tails (Sakashima, Superior Spider-Man); split destinations (Eye of Yawgmoth, Fork in the Road, Jarad's Orders, Memories Returning); singletons in the ledger.

## Cross-package conflicts
`procedures.rs`/`mod.rs` (additive), `anthem_grant_lines.rs`, `preprocess.rs` + `line_shapes.rs` (struct field added), `remove_destroy.rs`, `reference.rs`, core `anthem_model.rs`.
