# p11-other — card-failure campaign cf8 summary

158 cards: **25 source-proposed**, **133 blocked**, 0 already-on-main, 0 untriaged.

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
- **damage-history-condition**: Karakyk Guardian, Palladia-Mors, the Ruiner, Ruric Thar, Magecrusher
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
- **full-text-copy**: Volrath's Shapeshifter
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
- **outside-game**: Mastermind's Acquisition, North Wind Avatar, Research // Development, Ring of Ma'rûf, The Raven's Warning
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
- **spell-trigger-filter**: Codie, Ravenous Codex, Riku of Many Paths, Virtual Assistant
- **surveil-count**: Starving Revenant
- **target-change-contest**: Psychic Battle
- **token-followups**: Phantom Steed, Preston Garvey, Minuteman
- **top-of-library-static**: Conspicuous Snoop, Crown of Convergence, Mul Daya Channelers, Skill Borrower, Vampire Nocturnus
- **trigger-suppression**: Hushbringer
- **triggered-ability-counter**: Strict Proctor
- **turn-history-zone-move-count**: Anzrag's Rampage, Structural Assault
- **x-in-cost-filter**: Rosheen, Roaring Prophet

Per-card precise gaps are in `ledger.jsonl` (`gameplay_gap`). Most promising next step: a `Condition::TopCardOfLibraryMatches{player, filter}` (engine already marks continuous state dirty on library-top changes) would unlock Mul Daya Channelers and Vampire Nocturnus.
