# cf8 p09-other — summary

154 cards: 18 source-proposed, 136 blocked, 0 untriaged (first pass). Unvalidated: no builds or test
runs; probing stopped when the prebuilt binary disappeared.

## Fixed clusters
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
- Throne slots: the activation-availability check may count one multi-subtype card toward two slots.
- Gabriel: rampage option and "until your next upkeep" duration unverified.
- Fastbond currently miscompiles "if it wasn't the first land you played this turn" as "if it wasn't a land" (pre-existing, strict-compiled).
- "Put one of them into your hand and the other into your graveyard" drops the second destination (Fork in the Road family).

## Blocked, by missing mechanic (per-card detail in ledger.jsonl)
- New engine effects/designations: day/night set/toggle (Into the Night, Unnatural Moonrise, Tovolar, The Celestus); unprepare (Biblioplex Tomekeeper, Infinite Coursework); foretell from an effect (Ethereal Valkyrie, The Foretold Soldier); unblock/re-block (Balduvian Warlord, Ydwen Efreet); turn control (The Dominion Bracelet); gain suspend (Sinister Concierge); N untap steps (Telekinesis).
- Choice designations: two colors (Seal of the Guildpact, Tablet of the Guilds); two players (Bitter Feud, Sower of Discord); counter kind (Aven Courier, Contractual Safeguard, Dramatist's Puppet, Quarry Hauler); non-controller choices (Choice of Damnations, Master of Ceremonies, Noxious Vapors, Selective Obliteration); votes (Custodi Squire, Vault 11, Illusion of Choice); others (Rite of Ruin, Phyrexian Splicer, Swirl the Mists).
- Missing predicates/values: has an activated ability (Enigma Jewel); shares a card type (Eye of Ojer Taq); player damaged by the source this turn (Wicked Akuba); players who lost the game (Rampant Frogantua); chroma/bushido on affected creature (Light from Within, Takeno); double-faced (Invasion of Pyrulea); same name (Winnow); greatest power (Getaway Glamer); different/same controllers (Cloud's Limit Break, Simic Guildmage); ability source characteristics (Abstruse Archaic, The Peregrine Dynamo); suspended card (Amy Pond).
- Composite static readers: Possessed ×4; assign-damage-as-unblocked grants (Predatory Focus, Siege Behemoth); conditioned grants (Cloud, Kosei); granted ability naming the granter's controller (Hold for Ransom).
- Animation/become: still-a-land (Hunting Wilds, Primal Adversary, Restless Prairie, Rude Awakening); color/type become (Puca's Eye, Foraging Wickermaw, Ageless Sentinels, Mistform Sliver, Traitor's Clutch, Soul Sculptor); dynamic base P/T (Amplifire, Arni Brokenbrow, Sworn Defender, Captain Rex Nebula).
- Other: token copies attached to creatures (Arna Kennerüd, Three Dog); clone exception tails (Sakashima, Superior Spider-Man); split destinations (Eye of Yawgmoth, Fork in the Road, Jarad's Orders, Memories Returning); singletons in the ledger.

## Cross-package conflicts
`procedures.rs`/`mod.rs` (additive), `anthem_grant_lines.rs`, `preprocess.rs` + `line_shapes.rs` (struct field added), `remove_destroy.rs`, `reference.rs`, core `anthem_model.rs`.
