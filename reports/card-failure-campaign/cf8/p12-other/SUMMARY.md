# p12-other — fixer summary (cf8, branch cf8/p12-other)

123 frozen cards. Result: **23 source-proposed, 2 already-on-main, 98 blocked**.
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
