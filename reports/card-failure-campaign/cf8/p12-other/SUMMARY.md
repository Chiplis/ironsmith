# p12-other — fixer summary (cf8, branch cf8/p12-other)

123 frozen cards. Result (round 2): **35 source-proposed, 2 already-on-main, 86 blocked**, plus 2 out-of-package `semantic-fix-collateral` ledger rows (Mishra, Eminent One; Astral Drift).
Nothing was built or run (campaign policy). The prebuilt `compile_oracle_text` was used read-only
for triage until it disappeared mid-session; none of the fixes below were validated by it.
Tests: `crates/ironsmith-compiler-runtime/tests/p12_*.rs` (shared helper `tests/p12_other/support.rs`,
fixture `fixtures/p12_other_card_bodies.json.fixture` with the complete frozen bodies + cost/type).

## Clusters fixed (general grammar/lowering changes)

| Cluster | Cards | Root cause → general fix |
|---|---|---|
| trigger-subject-relative-clauses | Goro-Goro and Satoru, Whirlwind Killer Cyclone, Killian Decisive Mentor, Circle of Affliction | Any relative pronoun in a trigger subject was rejected. Now: `… that entered (the battlefield) this turn` → `entered_battlefield_this_turn`; `… that are/is enchanted by <Aura filter>` → general object-filter reading; `a source of the chosen color` → `chosen_color` filter. (`trigger_subject_filters.rs`, `grammar/trigger_subjects.rs`) |
| negated-copy-source-name | Dutiful Replicator | `not named X` after a copy source was captured as the token's *name*; named-token clause now ignores a preceding `not`; strict controller tail admits `not named` only when `excluded_name` was captured. |
| virtuous-role-token | Ellivere of the Wild Court | New predefined `BuiltinTokenShape::VirtuousRole` (CR 111.10): Aura Role, +1/+1 for each enchantment you control (live `AnthemCountExpression`). |
| coordinated-arms | Besmirch, Sinister Gnarlbark, Inspired Ultimatum | Bare `untap`/`tap` arm shares the next arm's object reference; `blight` is a non-verb effect head; `, this deals N damage` after a comma starts a sibling action. |
| function-word-short-names | And They Shall Know No Fear | First-word short-name alias `And` rewrote every “and” into a source reference. Closed-class words (and, into, up, no, all, …) never become short-name aliases. |
| attached-anthem-negated-restriction | Spectral Shield | `Enchanted creature gets P/T and can't …` = attached anthem + the generic negated object restriction. |
| event-triggers | Drownyard Lurker, Warped Tusker, Saproling Infestation, Jokulmorder, Rose Cutthroat Raider | `you cast or cycle <this>` → Either(cast-this, cycle-this); **Either triggers now function from the union of both arms' zones** (both derive functions); `<player> kicks a spell` = cast of a kicked spell (CR 702.33d); play-land trigger accepts a land-subtype noun; `end of combat on your turn` = EndOfCombat qualified by YourTurn. |
| count-values | Benediction of Moons, Breathe Your Last, Wanderwine Farewell, Anowon | `for each player` → CountPlayers(Any); `for each of its colors` → ColorsOf(It); `returned to its owner's hand this way` → PriorEffect(Returned); mill `a card for each 1 damage dealt to them` → EventValue(Amount). |
| discard-qualifiers | Tsabo's Decree, Void | Trailing `of that type` adds `chosen_creature_type`; **silent-drop fix**: a trailing relation used to *replace* the leading card qualifier (`nonland` lost) — now the full card phrase is read or the clause fails closed. |
| unless-payment-life | Soul Charmer | `gain N life unless <player> pays <cost>` → UnlessPays (rejects an implicit payer). |
| already-on-main | Assassin's Ink, Geistlight Snare | `parse_double_conditional_this_spell_cost_reduction_line` from merged cba78f342. |

Residual (unverified) risks, all fail-closed if wrong: Rose's “Junk token for each opponent you
attacked”, Anowon's “if the player mills at least one creature card this way”, Tsabo's
“destroy all creatures of that type that player controls”, Soul Charmer's payer binding, Killian's
“that are enchanted by” plural form.

## Blocked, grouped by missing mechanic
- **Attack requirement toward a specific player** (needs an effect writing `attack_player_requirements` for a targeted creature): Alluring Siren, Dulcet Sirens, Ravener; delayed next-combat variant: Trench Behemoth.
- **Rooms lock/unlock choice**: Ghostly Keybearer, Keys to the House, Marina Vendrell, Smoky Lounge.
- **Transform-return attached to player/permanent**: Accursed Witch, Radiant Grace, Vengeful Strangler.
- **Ability-copy triggers** (activated/triggered ability of a creature): Battlemage's Bracers, Illusionist's Bracers, Firebender Ascension, Pit Automaton.
- **Granted flashback with cost = mana cost**: Iroh Grand Lotus, Lier.
- **Combat restrictions without engine model** (attack alone, must be blocked by X, unblockable-unless): Errantry, Slayer's Cleaver, Become the Pilot; conditional anthem by most-common color: Heroic Defiance.
- **“…damage instead if that target is …”**: Light Up the Night, Lithomantic Barrage.
- **Delayed / next-main-phase mana and mana-type families**: Conduit of Storms, Mana Sculpt, Open the Omenpaths, Squandered Resources, Paliano.
- **Token definitions**: land token (Overlord of the Hauntwoods), counter-sized LKI token (Phantasmal Sphere), copy with name override (The Eleventh Hour), named equipment token (Icingdeath), token-creation replacement (Moonlit Meditation), die-roll replacement (Mr. House).
- **Enter replacements “if it wasn't cast”**: Hallowed Moonlight, Mistcaller; discard replacement: Dodecapod; mill replacement: The Water Crystal.
- **Where-X / history values**: Dimir Strandcatcher, Erestor, Glyph of Delusion, Kotis, Pair o' Dice Lost, Toph, Reverse Polarity, Avenge (last-turn attack), Blaster Hulk, Aether Spike, Search for Glory, Phosphorescent Feast, Taste of Paradise, Expand the Sphere, Explosive Singularity.
- **Miscellaneous trigger families**: Agent Maria Hill, Bomb Squad, Bronze Bombshell, C.A.M.P., Case File Auditor, Dream Devourer, Historian's Boon, Motion Sickness, Selfless Squire, Seraphic Greatsword, Shipwreck Sifters, Temporal Anchor, Watcher of Hours, Cyclopean Tomb, Explosion of Riches, Flash Conscription, Pygmy Hippo, Redemptor Dreadnought, Tale of Katara and Toph, Tyvar, Reyhan, Desperate Measures, Ertai's Meddling, Spellweaver Helix, Eriette, Imodane, Kaust, Millicent, Darksteel Garrison (no Fortified reference), Pick Up the Pace (static linked-exile play permission), Erdwal Illuminator.
- **Other**: Djeru and Hazoret, Iron Mastiff, Rakdos Augermage, Charitable Levy, Missy, Barbarian Bully, Nakaya Shade, Skulking Killer, Tolsimir (fight is not a chain-split verb), Riverfall Mimic, Tariff, Triple Triad, Bludgeon Brawl, Pharika's Spawn.

## Pre-existing silent drops observed (not fixed, flag for owners)
- `Fortified land has indestructible.` compiles to “All lands have indestructible.”
- `Create a token that's a copy of target creature, except its name is X.` drops the name exception.
- `Until end of turn, if a creature would enter, exile it instead.` compiles to “Exile it.”
- `Destroy target creature that was turned face up this turn` reduces to “face-up creature”.

## Merge risks
- `trigger_subject_filters.rs`, `trigger_clause_core.rs::parse_trigger_clause_lexed`, `chain_splitting/recognition.rs`, `chain_carry.rs` and `count_shapes_core.rs` are shared hot spots; hunks are additive and local.
- Either-trigger zone union changes behaviour for any Either trigger with a non-battlefield arm (e.g. Astral Drift on the lowering path now also functions from graveyard/exile) — intended per CR 113.6.
- The discard qualifier merge can turn previously *lossy-compiling* cards with “<qualifier> cards <trailing relation>” into failures if the full phrase does not parse — intended (fail closed).
- `BuiltinTokenShape` gained a variant; all exhaustive matches in grammar/semantic/lowering were updated (engine's separate `role_token.rs` not touched).

## Round 2 (coordinator follow-up)

### Corrections
- **Union-trigger zones (CR 113.6, 603.6)** — replaces the round-1 "widen to the union" change. A new
  `TriggerKind::ZoneGated { trigger, zones }` (core, appended variant) + engine `ZoneGatedTrigger`
  gates *each arm* of an `Either` trigger to its own functional zones; the ability's zones are the
  union. Applied in `compile_trigger_spec` only when the arms' zones differ. Astral Drift's
  "cycle another card" arm now never fires from the graveyard/exile (negative test
  `p12_union_trigger_zones.rs`); the self-cycle arm keeps the engine's existing cycled-card zones
  (graveyard/exile — where the card is when the cycle event is observed). Text-change rewrite,
  model interpreter and runtime-audit contract walk the new kind.

### Silent miscompilations fixed (with collateral)
| Bug | Fix | Cards |
|---|---|---|
| "Fortified land has X" → all lands | `CompilerReferenceTag::Fortified` binds the Fortification's host like Equipped (CR 301.6) in grammar, filter matching, statics (`AttachedTo`), descriptions; new **Fortify** keyword (CR 702.67a: sorcery-speed attach to target land you control) | Darksteel Garrison (now source-proposed), C.A.M.P. (host fixed; mana-tapped trigger still blocked) |
| "copy …, except its name is X / named X" dropped | CR 707.9b `set_name` + `added_supertypes` on `CreateTokenCopyEffect` (core, serde-default), AST, grammar (name split out before characteristic words, cased from tokens), lowering, engine; a name in a copy exception is never a token name; anaphoric copies with a name exception fail closed | The Eleventh Hour, Mishra, Eminent One (collateral) |
| "Until end of turn, if a creature would enter, exile it instead" → "Exile it" | new `turn_scoped_enter_replacement` recognizer → until-end-of-turn `RegisterFutureZoneReplacement` (any zone → battlefield ⇒ exile, CR 614.12); "wasn't cast" = not entering from the stack; "nontoken"; the lossy coordinated-leading-duration reader no longer admits the shape | Hallowed Moonlight, Mistcaller |
| "turned face up this turn" → "face-up" | `ObjectFilter.turned_face_up_this_turn` (serde-default) read from `TurnedFaceUpEvent` turn history (CR 708.8); object-filter grammar and trigger-subject relative clause | Kaust, Eyes of the Glade (only printed card) |

Other wording-family cards checked and left unchanged: Don't Blink, Gather Specimens (different
replacement shapes); the become-copy "except its name is" family (Gogo, Kimahri, Lazav ×2, Ludevic,
Sakashima, Sarkhan, Sunfrill) already keeps the name through its own grammar.

### Blocked queue worked
- **Targeted "attacks <player> this turn if able"** — new `MustAttackPlayerThisTurnEffect` (core +
  engine + decoder + materializer + text + audit) writing `effect_store.attack_player_requirements`
  (CR 508.1d scoring, current turn only); new `KeywordActionAst::MustAttackPlayerThisTurn`.
  Alluring Siren, Dulcet Sirens, Ravener → source-proposed. Trench Behemoth stays blocked (next
  combat phase, possibly another turn).
- **Copy an activated ability (CR 707.10)** — passive trigger "an ability of <object> is activated".
  Battlemage's Bracers, Illusionist's Bracers → source-proposed. Firebender Ascension, Pit Automaton
  blocked.
- **Granted flashback = mana cost** — "The flashback cost is equal to that card's mana cost".
  Lier, Iroh → source-proposed.
- **"deals N instead if that target is …"** — predicate "that target is <filter>" added
  (`TargetMatches` after rebinding); Light Up the Night and Lithomantic Barrage remain blocked
  because the anaphoric damage-instead composition is unverified (and Light Up the Night's
  loyalty-removal flashback cost is unsupported).
- **Room doors** — blocked: no engine "lock a door" operation (Keys to the House, Marina Vendrell);
  targeted unlock has no target slot (Ghostly Keybearer); unlock-door mana spend restriction
  (Smoky Lounge).
- **Token shapes** — blocked with precise gaps (land token spec, where-X after keyword list,
  named Equipment token, creation replacements).
- **Return transformed attached** — blocked: MoveToZone with attachment has no "enters
  transformed" option (three Curse/Aura DFCs).

### New tests (unrun)
`p12_union_trigger_zones.rs`, `p12_fortified_host.rs`, `p12_copy_name_exception.rs`,
`p12_turn_scoped_enter_replacement.rs`, `p12_turned_face_up_this_turn.rs`,
`p12_attack_player_requirement.rs`, `p12_equipped_ability_copy.rs`,
`p12_granted_flashback_mana_cost.rs`.

### Additional merge risks
- Core additions: `TriggerKind::ZoneGated` (appended), `MustAttackPlayerThisTurnEffect`,
  `CreateTokenCopyEffect.{set_name, added_supertypes}`, `ObjectFilter.turned_face_up_this_turn`
  (all serde-default/appended). `CompilerReferenceTag::Fortified` and
  `KeywordActionAst::MustAttackPlayerThisTurn` / `CreateTokenCopyFromSource.{set_name,
  added_supertypes}` touch shared enums — every exhaustive match found was updated.
- `"enchanted" | "equipped"` tag lists across engine/text were widened with `"fortified"`.
- The round-1 history rewrite removed an accidentally committed `.cargo/config.toml` hunk; all
  later commits stage explicit paths.

## Round 3 (on top of cf8/integration)

Package counts: **42 source-proposed / 2 already-on-main / 79 blocked** (123), plus 2
`semantic-fix-collateral` rows (Mishra, Eminent One; Astral Drift). Round 3 moved 7 cards to
source-proposed. Cycle-self trigger zones were left alone, as instructed.

### Mechanisms
- **Effect registry** — `MustAttackPlayerThisTurnEffect` row added to `effect-registry.tsv`
  (combat family).
- **Return transformed and attached** (CR 712.14, CR 303.4f) — the attached-return branch of
  `return_exchange_zone.rs` composes the transformed move (`enters_transformed`) with the existing
  `AttachObjectsEffect`. Accursed Witch, Radiant Grace, Vengeful Strangler → source-proposed.
- **Targeted Room unlock** (CR 709.5f) — new `KeywordActionAst::UnlockTargetRoomDoor { target }`
  (arms added in resolve reference_resolution/tag_support, grammar dispatch_entry/modal_support,
  lowering handles_action) lowers to TargetOnly(up to one Room you control) +
  `UnlockRoomDoorEffect` with `room_filter.is_target_object`. The engine executor now builds the
  filter context with the resolving targets. Ghostly Keybearer → source-proposed.
- **Next exhaust activation copy** (CR 702.177a, CR 707.10) — the Dynaheir next-activation delayed
  shape accepts an `exhaust` ability marker instead of the mana requirement; the engine marker
  matcher recognizes exhaust abilities through `is_exhaust_ability()`. Pit Automaton →
  source-proposed.
- **"deals N instead if that target is …"** — carry-over confirmed: the SelfReplacement
  composition reuses the default target, and the round-2 predicate makes the condition
  `TargetMatches`. Lithomantic Barrage → source-proposed. Light Up the Night stays blocked (its
  remove-X-loyalty flashback cost is unsupported).
- **Token keyword list + where-X** — an unquoted `where X is` after `with <keywords>` ends the
  keyword list. Before this, it leaked into the token definition through the `this` rules-text
  start. Phantasmal Sphere → source-proposed.

### Still blocked (precise gaps)
- Keys to the House, Marina Vendrell — locking a door (CR 709.5c) needs a reversible door state.
  Unlocking currently applies a fused split overlay with no inverse.
- Smoky Lounge — "spend this mana only to unlock doors" needs the unlock special action
  (CR 709.5e) to be a typed mana-spend context.
- Firebender Ascension — `AbilityTriggeredEvent` has no attack-declaration cause.
- Trench Behemoth — no "until its controller's next combat" duration;
  `attack_player_requirements` is current-turn only.
- Overlord of the Hauntwoods (land token spec that has every basic land type), Icingdeath (named
  legendary Equipment token with quoted abilities and equip), Moonlit Meditation (first-time
  token-creation replacement into copies), Mr. House (created-set replacement inside a die roll).

### Dependants in other packages (grep of all cf8 ledgers for "p12")
All are p01 rows that wait on copy mechanisms owned here. None were unblocked this round:
- Copy-cards-then-cast (Chandra, Pyromaster; Mnemonic Deluge; Reversal of Fortune; Arcane
  Bombardment; Zethi; Spellweaver Volute; Bloodthirsty Adversary; The Tale of Tamiyo; Baron Helmut
  Zemo; Myra the Magnificent; Mizzix's Mastery; Garth One-Eye).
- Has-all-abilities / granted copies of abilities (Koh, the Face Stealer; Sharkey; Kasmina).
- Copy exceptions on become-copy or token copies (Vesuvan Doppelganger, Aurora Shifter, Rebuild
  the City, Ob Nixilis the Adversary, Esoteric Duplicator, Calamity, Shredder).
- Bulleted characteristic templates (Wild Shape, Genku, Outlaws' Merriment).

Rows already unblocked by p12 work and recorded in p01's ledger: Alluring Siren, Dulcet Sirens,
Ravener (attack-player requirement), Darksteel Garrison (Fortified), Kaust (turned face up this
turn), and The Eleventh Hour (copy name exception).

### New tests (unrun)
`p12_exhaust_next_activation_copy.rs`, `p12_return_transformed_attached.rs`,
`p12_targeted_room_unlock.rs`, `p12_damage_instead_if_target.rs`, `p12_token_keyword_where_x.rs`.

### Round 3 merge risks
- `KeywordActionAst::UnlockTargetRoomDoor` is a new shared-enum variant. Every match that names
  `MustAttackPlayerThisTurn` was given a sibling arm.
- `unlock_room_door.rs` now passes the resolving targets into its filter context. A non-target
  `room_filter` is unaffected.
- The create-token `with` slice now stops at `where X`. This affects every "token with <keywords>,
  where X is …" create; before the change those failed to parse.

## Round 4 (coordinator: build the remaining owned mechanisms)

Package counts: **48 source-proposed / 2 already-on-main / 73 blocked** (123), plus 2
`semantic-fix-collateral` rows. There are also **8 `dependant-proposed` rows** for other packages'
cards that the new mechanisms should unblock (status and `owner_package` are set so the orchestrator
can move them; they are not counted in the 123).

### Mechanisms (one commit each)
- **Room door locking (CR 709.5c)** — `UnlockRoomDoorEffect.allow_lock` (serde default);
  `special_actions::{unlocked_room_doors, apply_room_door_lock}` with
  `GameState::{lock_room_only_unlocked_door, lock_door_of_fully_unlocked_room}`. Locking a fully
  unlocked Room re-applies the current half's definition, which drops the fused overlay. Locking
  the current door then shows the linked half. The grammar reads "lock or unlock a door of target
  Room you control" (`UnlockTargetRoomDoor { allow_lock }`). Keys to the House, Marina Vendrell →
  source-proposed.
- **"Spend this mana only to cast <spells> and unlock doors" (CR 709.5e)** — new
  `ManaUsageRestriction::CastSpellOrUnlockDoor`. Smoky Lounge → source-proposed.
- **Next-combat attack requirement (CR 508.1d)** —
  `MustAttackPlayerThisTurnEffect.controllers_next_combat` writes
  `effect_store.next_combat_attack_requirements`. The requirement is read by `must_attack_with_view`
  and attack scoring on that player's turn, and is spent at the end of that combat. Trench Behemoth
  → source-proposed.
- **Ability triggered by its source attacking + copy that triggered ability (CR 707.10)** —
  `TriggerKind/TriggerSpec::AbilityTriggered.caused_by_source_attacking`. The match requires
  `cause_kind == CreatureAttacked` and `cause_object == source`. `copy_spell` resolves an
  `AbilityTriggeredEvent` source to the triggered ability's stack entry by trigger identity.
  Firebender Ascension → source-proposed.
- **Land token** — `TokenDefinitionSpec::Land(LandTokenShape)`. "That is every basic land type"
  sets the five basic types, which grant their intrinsic mana abilities (CR 305.6). The clause fails
  closed on any other kind of token. Overlord of the Hauntwoods → source-proposed.
- **Copy cards, then cast the copies (CR 707.12)** — `copy_cast_procedure.rs` gains:
  - "Copy that card N times" → `RepeatEffects(N, may cast copy)`.
  - "Copy them / those cards" → `ForEachTagged(may cast copy of it)`.
  - A plural cast statement: "You may cast [any number of] the copies [without paying their mana
    costs]".
  - "Choose a <filter> card exiled this way and copy it N times".
- **Characteristic-list modes** — `document_parser/characteristic_modes.rs`. Under "Create a
  [colors] creature token with those characteristics", each bullet (`P/T types with abilities`) is
  read as "create a P/T <colors> <types> creature token with <abilities>". Quoted abilities keep
  their authored tokens.
- **Copy exceptions grant keywords (CR 707.9a)** — the keyword list after the last has/have/gain
  (vigilance, menace, haste, reach, lifelink, deathtouch, hexproof, defender, first/double strike,
  flying, trample).
- **"For each other opponent"** — the opponents other than the defending (attacked) player.
- **Copy activated abilities except mana abilities (CR 613.1f)** —
  `CopyActivatedAbilities.exclude_mana_abilities` (core, serde default) → engine
  `include_mana = false`.

### Dependants proposed for other packages (`dependant-proposed` rows)
| Card | Owner | Mechanism | Remaining risk |
|---|---|---|---|
| Mnemonic Deluge | p04 | copy that card three times | — |
| Chandra, Pyromaster | p04 | choose exiled card, copy three times | +1 line not re-verified |
| The Tale of Tamiyo | p04 | copy them, cast any number | chapters I–III need p11's repeat process |
| Outlaws' Merriment | p04 | characteristic-list modes | — |
| Genku, Future Shaper | p04 | characteristic-list modes | — |
| Shredder, Shadow Master | p04 | for each other opponent | "attacking that player" rebinding in the filtered loop unverified |
| Rebuild the City | p01 | copy exception keywords | p01 must retire its ChooseLeadingSpell guard |
| Sharkey, Tyrant of the Shire | p03 | copy except mana abilities | "Mana of any type …" line is p05's |

These dependants stay blocked, with the gaps named:
- **Wild Shape** — "has that base power and toughness, becomes that creature type, and gains that
  ability" template.
- **Reversal of Fortune** — copy a card in a revealed hand.
- **Arcane Bombardment** — copy each card exiled with this enchantment (linked-exile set).
- **Zethi** — copy exiled cards with a kick counter.
- **Spellweaver Volute** — copy the enchanted card, then re-attach.
- **Bloodthirsty Adversary** — copy inside a reflexive "when you pay" trigger.
- **Baron Helmut Zemo** — "up to three of the copies".
- **Myra the Magnificent** — attraction-visit copy.
- **Koh** — the last chosen card's abilities.
- **Kasmina** — loyalty abilities granted to other planeswalkers, with exclude-source semantics.
- **Vesuvan Doppelganger / Aurora Shifter** — "except it has this ability" self-reference.
- **Ob Nixilis** — casualty copy with starting loyalty X.
- **Mizzix's Mastery** — overload "each" exile collection.
- **Calamity** — copy of the creature that saddled it.
- **Garth One-Eye** — copy of a named card from outside the game.
- **Esoteric Duplicator** — "If you do, at the beginning of the next end step, create a copy of
  that artifact" (LKI copy in a delayed result branch).

### Still blocked in this package (precise gaps in the ledger)
- **Light Up the Night** — an X minimum scoped to the flashback method.
- **Icingdeath** — the quoted-ability list split on the comma inside a quote.
- **Moonlit Meditation** — a dynamic copy template plus a first-time-each-turn token replacement.
- **Mr. House** — an in-effect alternative ("instead create that token and a Treasure") plus dice
  counted from Treasure mana spent on an activation.

### New tests (unrun)
- `p12_room_lock_and_door_mana.rs` (plus engine unit tests in `unlock_room_door.rs`)
- `p12_next_combat_attack_requirement.rs`
- `p12_attack_caused_trigger_copy.rs` (plus a matcher unit test in `ability_triggered.rs`)
- `p12_land_token.rs`
- `p12_copy_cards_then_cast.rs`
- `p12_characteristic_token_modes.rs`
- `p12_copy_exception_keywords.rs`
- `p12_each_other_opponent_token_copies.rs`
- `p12_copy_activated_except_mana.rs`

### Round 4 merge risks
- **New fields / variants touching other crates:**
  - `UnlockRoomDoorEffect.allow_lock`
  - `MustAttackPlayerThisTurnEffect.controllers_next_combat`
  - `TriggerKind::AbilityTriggered.caused_by_source_attacking`
  - `CopyActivatedAbilities.exclude_mana_abilities`
  - `ManaUsageRestriction::CastSpellOrUnlockDoor`
  - `TokenDefinitionSpec::Land`
  - AST fields: `UnlockTargetRoomDoor.allow_lock`, `MustAttackPlayerThisTurn.controllers_next_combat`,
    `TriggerSpec::AbilityTriggered.caused_by_source_attacking`

  Every match site found by grep was updated, including grammar tests and
  `crates/ironsmith-compiler/tests/u013_ability_triggered.rs`.
- **Minimize-list files:** `effect.rs` and the `static_ability_model/grants.rs` submodule got small
  additive hunks. `keyword_static/costs_replacements_and_permissions.rs` changed only inside
  `parse_copy_activated_abilities_line`.
- **`EffectStore.next_combat_attack_requirements`** is a new field. Any checkpoint or serialization
  code that lists EffectStore fields explicitly needs it; none was found by grep.

## Round 5 (coordinator: finish the remaining blocked owned items and the copy dependants)

**Package counts (123 cards):** 52 source-proposed / 2 already-on-main / 69 blocked.

**Other rows:**
- 2 `semantic-fix-collateral` rows.
- 16 `dependant-proposed` rows for other packages' cards: 8 from round 4 and 8 from round 5.

### Owned items (one commit each)
- **Light Up the Night:**
  - Mechanism: a new field, `AlternativeCastingMethod::Flashback.x_minimum` (serde default). Every
    construction and match site across all crates was updated (patterns use `{ total_cost, .. }`;
    constructions use `x_minimum: 0`).
  - Grammar: "If you cast this spell this way, X can't be 0" sets the field.
  - Engine: `compute_spell_cast_x_bounds_with_reduction` applies it only for that alternative
    method.
  - Text renders the sentence.
- **Icingdeath:**
  - Root cause: `parse_granted_abilities_for_gain_clause` split the ability list with
    `split_lexed_slices_on_comma`, which ignores quotes. That cut `"Whenever equipped creature
    attacks, tap ...,"` at its inner comma.
  - Fix: list items now split only at top-level commas and between adjacent quoted rules.
  - Unit tests cover that list and a keyword + quoted-trigger + quoted-activation list.
- **Moonlit Meditation:**
  - "The first time you would create one or more tokens each turn" wraps `TokenCreationTemplates`
    in a ValueComparison condition: TokensCreated(You) this turn == 0 (CR 614.1).
  - The "tokens that are copies of enchanted permanent" template is a `CreateTokenCopy`. The
    replacement application resolves it from the replacement source: the attached host for
    enchanted/equipped/fortified, otherwise a unique battlefield match. It captures that object's
    copiable values when the replacement applies.
- **Mr. House:**
  - "If <cond>, instead create that token and <more>" becomes a SelfReplacement of the prior
    creation.
  - "a six-sided die plus an additional six-sided die for each mana from Treasures spent to
    activate this ability" becomes one roll plus
    `RepeatEffects(ManaFromSourceSpentToCastThisSpell{ThisAbility})`.

### Dependant mechanisms, biggest shared mechanism first

**1. Copy-cards-then-cast procedure** — `copy_cast_procedure.rs`:
- "Copy each <filter>" / "Copy the <filter>" opens a group: `TagMatchingObjects`, then
  `ForEachTagged` with a may-cast of a copy, or a single cast.
- "For each card exiled this way, copy it, and you may cast the copy" iterates the exiled cards.
- This covers Arcane Bombardment (p04), Zethi (p04) and Mizzix's Mastery (p07).

**2. Characteristic-list modes:**
- New Wild Shape template (p04): "Until end of turn, <subject> has that base power and toughness,
  becomes that creature type, and gains that ability".
- Each bullet is read as "becomes a <Type> with base power and toughness P/T and gains <ability>".

**3. Become-copy exception "it has this ability and "...""** — Aurora Shifter (p07). The copy
preserves the source ability and gains the quoted one.

**4. "tapped and attacking"** — chain splitting no longer cuts this token entry at its "and".
Calamity (p07); its "Repeat this process once" depends on p11.

**5. Casualty X copy** — Ob Nixilis (p07). The existing engine effect lacked a text renderer, so it
compiled to an "Unsupported effect" marker. It is now rendered.

**6. "has the loyalty abilities of <this>"** — Kasmina (p02). A granted `CopyActivatedAbilities`
with the source as donor and `only_loyalty`. The engine excludes only the recipient's own id, so the
donor is kept.

### Still blocked dependants (gaps)
- **Koh:** needs a persistent "last chosen card" designation per source, plus copying triggered
  abilities from an exiled card.
- **Vesuvan Doppelganger:**
  - needs the color-exclusion copy exception ("doesn't copy that creature's color");
  - needs a granted quoted ability whose own "this ability" refers to itself (recursive grant)
    inside an enter-as-copy replacement.
- **Garth One-Eye:** needs runtime card-registry lookup to create a copy of a named card from
  outside the game.
- **Esoteric Duplicator:** "If you do, at the beginning of the next end step, create a token that's
  a copy of that artifact" fails the if-clause split. The delayed body needs an LKI copy of a
  sacrificed object; not isolated without a probe.
- **Spellweaver Volute:**
  - "copy the enchanted instant card": an Aura on a graveyard card has no zone in the filter, so
    the filtered copy opener declines it;
  - re-attaching the Aura to a graveyard card is unverified.
- **Bloodthirsty Adversary:** "exile ... and copy them" sits inside a reflexive "when you pay"
  trigger body. The document-level copy procedure does not run there.
- **Baron Helmut Zemo:**
  - "up to three of the copies" needs a choose-up-to-N step;
  - the activation cost "with fifteen or more black mana symbols among their mana costs" is an
    aggregate exile-cost constraint.
- **Myra the Magnificent:** the visit-attraction copy is tied to a chosen Attraction.
- **Reversal of Fortune:** "You may copy an instant or sorcery card in it", where "it" is the
  revealed hand of the target opponent. Binding the owner to the announced target opponent is
  unverified.

### New tests (unrun)
- `p12_flashback_x_minimum.rs`
- `p12_multi_quote_token.rs`, plus unit tests in `gain_ability.rs`
- `p12_first_time_token_copy_replacement.rs`
- `p12_instead_that_token_and_dice_per_mana.rs`
- `p12_copy_filtered_cards_then_cast.rs`
- `p12_become_copy_keeps_this_ability.rs`
- `p12_tapped_and_attacking_copy.rs`
- `p12_casualty_x_planeswalker_copy.rs`
- `p12_shared_loyalty_abilities.rs`
- extended `p12_characteristic_token_modes.rs` (Wild Shape)

### Round 5 merge risks
- **`Flashback.x_minimum`** touches 18 files, including test literals in engine, text,
  artifact-baker and compiler-runtime. Any branch adding a `Flashback { total_cost }` pattern or
  construction needs `..` or `x_minimum`.
- **The granted-ability item split** changes how any quoted list with inner commas is read. Lists
  without quotes behave as before.
- **`TokenTemplateReplacement`** gains `first_time_each_turn`. Its header now also accepts "the
  first time ... each turn".
- **`copy_cast_procedure::open`** now tries a filter-named copy opener before parsing the sentence.
  It only fires when the next sentence is a matching cast statement.
