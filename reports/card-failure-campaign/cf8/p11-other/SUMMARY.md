# p11-other — card-failure campaign cf8 summary

158 cards: **33 source-proposed**, **125 blocked**, 0 already-on-main, 0 untriaged.

All work is source-only and UNBUILT (per brief). The prebuilt `compile_oracle_text` was used for probing until it disappeared mid-session (main checkout rebuilding); clusters fixed after that point (blocker-count, block-alone, without-either-keyword, possessive switch) are verified by code reading only. A probe run at session start showed none of the 158 cards compiled on the binary, so none are already-on-main.

## Fixed clusters (general fixes)

| Cluster | Cards | Root cause | Fix | Files |
|---|---|---|---|---|
| discarded-this-turn-count | Misty Knight, Living Laser, Jiang Yanggu, Change of Fortune, Astonishing Spider-Man, Green Goblin | singular 'card you've discarded this turn' not a turn-history count (only plural 'cards ... discarded' reader existed) | `parse_turn_history_count_value` reads it as `Value::CardsDiscardedThisTurn(You)` (shared by draw-for-each, for-each counts, counters, token copies) | grammar `value_semantics/value_semantics_core.rs` |
| source-of-your-choice-prevention | Burrenton Forge-Tender, Auriok Replica, Prahv, Rith's Charm | 'Prevent all damage a [red] source of your choice would deal [to you] this turn' had no reading (only 'dealt to you this turn by a source of your choice') | new `source_of_your_choice` flag on `PreventAllDamageToTargetFromSourceFilter`; lowering adds `with_source_of_your_choice()` with descriptor as `from_source`; engine clears the descriptor after the choice (CR 609.7a) | grammar `clause_pattern_helpers.rs`; semantic `damage_prevention.rs`, `effects.rs`, `actions.rs`; lowering `subject_verb_early.rs`; engine `prevent_all_damage.rs` |
| rest-of-those-cards | Winota, Armored Skyhunter, Call to the Kindred | 'the rest of the cards / of those cards' not in the Rest surface when the look-at sequence is split | added phrases to `TargetSurface::Rest` and rest reference phrases | `target_surfaces.rs`, `reference_tag_stage.rs` |
| players-being-attacked-count | Apothecary White, Amber Gristle O'Maul | 'for each player being attacked' unread | `parse_for_each_count_value_words` -> `Value::PlayersBeingAttacked` (existing engine value: directly attacked players); draw route accepts it | `count_shapes_core.rs`, `zone_move_verbs.rs` |
| damage-for-each-multiplier | Black Market Tycoon, Lotleth Giant (also fixes Niko Aris outside package) | trailing 'for each X' after one damage recipient reached the target parser, which read it as a dynamic number of targets (`WithCountValue`) or rejected 'you' | `split_trailing_damage_for_each_multiplier` scales the fixed amount (`Count`/`Scaled`) before target parsing; sets/conditional tails excluded | `combat_verbs_combat.rs` |
| reinforce-x | Wren's Run Hydra, Swell of Courage | Reinforce only accepted fixed N | accept X when the reinforce cost has {X} (CR 702.77a, 107.3) | grammar `util.rs` |
| blocker-count-restrictions | Hexmark Destroyer, Sonorous Howlbonder, Rocksteady | (a) blocker-count evasion only on the source / 'each creature'; (b) labeled static lines returned the builder-aware view's error instead of trying the normalized body | new `parse_filtered_blocker_count_restriction_line` (grant of CantBeBlockedByMoreThan / CantBeBlockedExceptByNOrMore to a filtered set) + guard in `parse_cant_clauses_unbound`; labeled-static fallback | `anthem_grant_lines.rs`, `keyword_static/mod.rs`, `activation_costs.rs`, `document_parser/mod.rs` |
| source-cant-block-alone | Craven Hulk | 'can't block alone' read as 'can't block <attacker "alone">' | `DirectCantFact::SourceCantBlockAlone` -> `Restriction::block_alone(source)` claimed before the generic action | `cant_shapes/direct.rs`, `activation_costs.rs` |
| without-either-keyword | Stormtide Leviathan | 'without flying or islandwalk' unsupported | `FilterTailDecoration::WithoutEitherKeyword` excludes both | `filters/decorations.rs` |
| possessive-switch-pt | Valakut Fireboar | 'switch its power and toughness' | 'its' maps to the It tag like 'it' | `misc_action_shapes.rs` |

Tests (unrun): `crates/ironsmith-compiler-runtime/tests/{discarded_this_turn_counts,source_of_your_choice_prevention,rest_of_those_cards_remainder,players_being_attacked_counts,damage_for_each_multiplier,reinforce_x_amount,blocker_count_restrictions,source_cant_block_alone,without_either_keyword,possessive_switch_pt}.rs` with fixtures in `fixtures/*.json.fixture`.

## Risk notes

- Engine `PreventAllDamageEffect`: once a source of your choice is chosen, `damage_filter.from_source` is cleared (shield follows the chosen object, CR 609.7a). Affects existing 'creature of your choice' shields too (behaviour change only if the chosen source later stops matching).
- New AST field `source_of_your_choice` on `DamagePreventionActionAst::PreventAllDamageToTargetFromSourceFilter`; all other sites use `..`; the only struct literal is the constructor in `semantic/.../effects.rs`.
- Damage multiplier split applies only to Fixed amounts with a single plain recipient; 'each ...' recipients (Baki's Curse, Dragonhawk) are untouched.
- `parse_filtered_blocker_count_restriction_line` is registered WholeLine (no head hints); it declines source subjects, conditions, durations and defers 'each creature ... by more than' to the existing rule.
- Labeled-static fallback changes only lines whose builder-aware parse errored.
- New enum variants: `DirectCantFact::SourceCantBlockAlone`, `FilterTailDecoration::WithoutEitherKeyword` (all matches updated; grammar-crate local).
- Possible conflicts with siblings: `activation_costs.rs` (parse_cant_clauses_unbound guard list), `keyword_static/mod.rs` rule list, `count_shapes_core.rs`, `value_semantics_core.rs`.

## Second pass (coordinator queue)

| Mechanism | Cards | Implementation | Files |
|---|---|---|---|
| top-of-library static condition | Mul Daya Channelers, Vampire Nocturnus | `PredicateAst`/`Condition::TopCardOfYourLibraryMatches(filter)`; `parse_predicate` reads 'the top card of your library is <a card noun / color>'; engine checks `library.last()` (CR 401.1); library-top changes already bump revision and dirty continuous state | grammar `predicate_phrases/advanced/library_top.rs`; semantic `predicates.rs`; resolve `predicate_conditions.rs`; core `value_model.rs`; engine `condition_eval.rs`, `dependency.rs`, `text_change_predicates.rs`; lowering `iterated_player_validation.rs`; text `condition_rendering.rs` |
| lifetime damage history | Karakyk Guardian, Palladia-Mors, the Ruiner | `BattlefieldFlags.dealt_damage_since_entered` set in `commit_prepared_damage_original_with_outputs` (non-prevented damage, combat and noncombat; source = ctx.source), cleared with battlefield state (new object, CR 400.7); `Condition::SourceHasDealtDamageSinceEntered` invalidated through `mark_source_designation_changed`; grammar 'it/this creature hasn't dealt damage yet' | engine `game_state.rs`, `object_state_and_events.rs`, `turns_and_tracking.rs`, `rules/damage_assignment.rs` + condition plumbing above; grammar `advanced/source_damage_history.rs` |
| wishes (outside the game) | Mastermind's Acquisition, North Wind Avatar, Ring of Ma'rûf, The Raven's Warning | outside-the-game = existing `Zone::OutsideGame` backed by `Player.sideboard` (ChooseObjects already enumerates owned sideboard cards; Burning-Wish-style reveal readings already used it). New `parse_outside_game_put_shape` + `parse_put_from_outside_game`: '[you may] put a(n) <card> you own from outside the game into your hand / on top of your library' | grammar `effects/effect_composition.rs`, `effect_sentences/effect_composition.rs`, `dispatch_entry.rs` |

Second-pass tests (unrun): `top_of_library_conditions.rs`, `source_damage_history.rs`, `outside_game_put.rs`.

### Findings on the queued items

- **'Otherwise, that creature …' (Stolen Vitality): not a miscompile.** `rewrite_otherwise_referential_subject` deliberately turns it into 'target creature'; lowering's `push_choice` dedups equal target specs, so it binds the same declared target. This is required when the target is declared inside a gated branch (Insatiable Appetite, Pippin's Bravery), because the reference resolver hides gated-branch objects from the fallback's 'it'. The only other 'Otherwise, that …' cards in cards.json (Captivating Glance, Pulling Teeth, Zur's Weirding) are player subjects, not this rewrite. No collateral listed. Stolen Vitality itself ('Otherwise, it gains …' after an 'If it's your turn' conditional) stays blocked.
- **Repeat-this-process loop: one already exists.** Core `RepeatProcessEffect<E>` (`ironsmith-core/src/effect/mana_damage_and_control.rs`) + `ForEachEffectAst::RepeatProcess` + `rewrite_repeat_process` in `ironsmith-compiler-resolve/src/effect_ast_normalization.rs`. I did not add a `repeat_process.rs`; p05 should reuse this type instead of adding a second loop. The 4 loop cards fail on body shapes, not on a missing loop (details in the ledger).
- **Regeneration trigger:** design recorded in the ledger (Matopi Golem). Not implemented because `process_destroy_owned` runs the replacement program in the destroyer's context, so a reflexive trigger made there would get the wrong controller.
- **Player-scoped restrictions, search filters, dice/Attractions:** each card needs a different missing piece (listed per card). Shared blockers: player-subject attack restrictions with a 'their next turn' duration; a keyword list longer than two in `FilterTailDecoration`; and the English-noun subtype exclusions (Plan, Sphere). Dice/Attractions need Attraction-visit rolls outside the turn structure, planar-die actions and die-roll replacements.

Risk notes, second pass: (1) `Condition` gains two variants; every exhaustive match (condition_eval, text_change_predicates, condition_rendering::describe_condition) was updated, and the other sites use wildcards. (2) `BattlefieldFlags` gains a `HashSet` field (Default; the struct is not serialized). (3) Every first damage dealt by a permanent now runs `mark_source_designation_changed` (once per object lifetime).

## Blocked, grouped by missing mechanic

- **ability-copy**: Gogo, Master of Mimicry
- **additional-cost-count**: Primitive Justice
- **anaphoric-library**: Creative Technique
- **attached-control-condition**: Dog Umbra
- **block-restriction**: Ironclaw Curse
- **bundle-tokens**: Stangg, Echo Warrior
- **but-one**: Aladdin's Lamp
- **card-type-choice**: Portent of Calamity
- **chosen-card-reveal**: Stronghold Gambit
- **chosen-color-filter**: Chromatic Armor
- **chosen-set-each**: Sigardian Zealot
- **color-identity-note**: Fallaji Wayfarer
- **compound-grants**: Legion Loyalist
- **compound-self-statics**: Alexios, Deimos of Kosmos, Xantcha, Sleeper Agent
- **conditional-damage-unless**: Erg Raiders
- **conditional-dont-untap**: Icy Blast, Send to Sleep
- **conditional-keyword-grants**: Scion of Draco
- **conditional-prevention**: Sanwell, Avenger Ace
- **cost-tap-substitution**: Heirloom Epic
- **curse-attach-player**: Lynde, Cheerful Tormentor
- **damage-history-restriction**: Runesword
- **delayed-zone-move**: Three Wishes
- **dice-attractions**: Bamboozling Beeble, Centaur of Attention, Command Performance, Delina, Wild Mage, Ferris Wheel, Fractured Powerstone, Ichor Elixir, Line Cutter, Six-Sided Die
- **draft-matters**: Archdemon of Paliano
- **each-of-them**: The War in Heaven
- **each-of-x-targets**: Batroc the Leaper
- **energy-payment**: Vault 112: Sadistic Simulation
- **equipment-names**: Helm of Kaldra
- **exile-play-permission**: Evelyn, the Covetous
- **experience-count**: Otharri, Suns' Glory
- **face-down-piles**: Expose the Culprit, Ghastly Conscription
- **face-down-reveal**: Hauntwoods Shrieker
- **flip**: Sasaya, Orochi Ascendant // Sasaya's Essence
- **goad-permanent**: Jon Irenicus, Shattered One
- **granted-damage-split**: Psionic Sliver
- **granted-quoted-restriction**: Stilt-Man, Towering Terror
- **hand-exile-loop**: Struggle for Sanity
- **history-player-count**: Malcolm, Keen-Eyed Navigator, Reaper's Scythe
- **history-target**: Diseased Vermin, Fire and Brimstone, Needle Drop
- **legend-rule-effect**: Hall of Echoes
- **library-count-condition**: Isleback Spawn
- **life-total-set**: Torgaar, Famine Incarnate
- **look-put-back**: Dimir Charm
- **loyalty-copy**: Jaya's Phoenix
- **mana-spend-restriction**: Thran Turbine
- **mana-symbol-iteration**: Charmed Pendant
- **monarch-instead**: Court of Cunning
- **must-be-blocked**: Ace's Baseball Bat, The Masamune
- **named-card-count**: Mindblaze
- **on-stack-static**: Kaervek's Torch, Torrent of Lava
- **opponent-chosen-target**: Karplusan Minotaur
- **opponent-who-didnt**: Hollow Marauder, Zoyowa Lava-Tongue
- **opponents-discard**: Everything Pizza
- **otherwise-branch**: Stolen Vitality
- **outside-game-wish**: Research // Development
- **player-chooses-name**: Petra Sphinx, Vexing Arcanix
- **player-restriction**: Sen Triplets, Willie Lumpkin, Postman, Xanathar, Guild Kingpin, Keen-Eared Sentry, Mirri, Weatherlight Duelist
- **player-restriction-unless**: Antagonism
- **power-parity-condition**: Kianne, Corrupted Memory
- **rad-counters**: Vexing Radgull
- **redirect-damage**: Nova Pentacle
- **regeneration-trigger**: Matopi Golem, Skeleton Scavengers, Soldevi Sentry
- **repeat-process**: Crooked Scales, Forgotten Lore, Shrouded Lore, Demonlord Belzenlok
- **reveal-conditional**: Omnath, Locus of All
- **reveal-hand-and-top**: Psychotic Episode
- **reveal-hand-count**: Blood Oath, Thought Hemorrhage
- **reveal-reference**: Keen Duelist, Parker Luck
- **same-action**: The Wedding of River Song
- **same-name-play-trigger**: Search the City
- **search-filter**: Light-Paws, Emperor's Voice, Mimeofacture, Monument to Perfection, Mwonvuli Beast Tracker, The Masters of Evil
- **shared-color-condition**: Common Cause
- **shuffle-zones**: Sway of the Stars, The Great Aurora
- **skip-replacement**: Fasting, Island Sanctuary
- **source-damage-history**: Ruric Thar, Magecrusher
- **spell-trigger-filter**: Codie, Ravenous Codex, Riku of Many Paths, Virtual Assistant
- **surveil-count**: Starving Revenant
- **target-change-contest**: Psychic Battle
- **token-followups**: Phantom Steed, Preston Garvey, Minuteman
- **top-of-library-static**: Conspicuous Snoop, Crown of Convergence, Skill Borrower, Volrath's Shapeshifter
- **trigger-suppression**: Hushbringer
- **triggered-ability-counter**: Strict Proctor
- **turn-history-zone-move-count**: Anzrag's Rampage, Structural Assault
- **x-in-cost-filter**: Rosheen, Roaring Prophet

Per-card precise gaps are in `ledger.jsonl` (`gameplay_gap`).
