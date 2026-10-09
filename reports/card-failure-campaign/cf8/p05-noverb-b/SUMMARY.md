# cf8 p05-noverb-b — "could not find verb in effect clause" (153 cards)

Status (round 5): 61 source-proposed, 5 already-on-main, 87 blocked, 0 untriaged (own package); plus 50 other-package rows: 3 dependant-proposed, 47 dependant-blocked.
(Round 4: 60 / 5 / 88.)
(Round 3: 47 / 5 / 101.)
(Round 2: 42 / 5 / 106.)
(Round 1 was 18 / 4 / 131. The prebuilt probe binary was removed mid-session, so all round-2
work is source-only reasoning.)
Nothing was built or run. Every claim is from reading the source plus prebuilt-binary probes of
equivalent texts (for example "It becomes ..." in place of "It's ...").

The error text names the clause the parser reached, not the underlying cause. This package is
almost all singletons, so most cards need an engine mechanic that doesn't exist yet.

## Clusters fixed (general grammar/runtime fixes)

| Cluster | Cards | Root cause | Fix |
|---|---|---|---|
| copular_contraction_animation | Brilliance Unleashed, Fang, Princess Yue, Sauron, The Master, Yedora, Eluge, Cavernous Maw, Chainer, Quicksilver Fountain, Grimoire of the Dead (11) | The copular-animation shape only accepted a few descriptors after "it's". "He's" and "his other types", "they're", "is A and is B", and a counter-linked duration were not read. The counter-linked land followup only accepted the subject "That land is ... in addition to its other types". "It's still a Cave land" was not read as retention. | `parse_contracted_pronoun_copula_shape` reads a contracted pronoun copula (it's/he's/she's/they're) as "becomes" at both the sentence and clause level. It falls through if the become grammar cannot read the descriptor. `parse_copular_predicate_pair_shape` adds pairs. `parse_affected_object_counter_duration_suffix` adds the become duration `ForAsLongAs(ObjectHasCounter(AffectedObject))` (CR 611.2b). The counter-linked land shape now accepts "it's/it is" and the set-subtype form (CR 305.7). `is_still_land_followup` accepts land subtypes (CR 205.1b). |
| contracted_suspicion_clear | Airtight Alibi, Eliminate the Impossible (2) | `parse_clear_suspected_clause` compared raw slices ("it's") with apostrophe-less words | The contraction is matched with `is_any_word` |
| its_controller_manifest_dread | Fear of Impostors, Unwanted Remake (2) | Manifest dread accepted no player actor | Optional "its controller" actor → `PlayerAst::ItsController` (lowering already resolves the actor) |
| opponent_only_activation | Detention Vortex (1; also unblocks the first ability of Oft-Nabbed Goat and Soul Ransom) | There was no activator-relative "only your opponents" permission | New `ActivationTiming::{AnyTimeByOpponents, SorcerySpeedByOpponents}`. Engine legality: the activator must be an opponent of the source's current controller, plus sorcery timing (CR 602.5d). Also added: parse, routing and text |
| dungeon_game_noun_alias | Dungeon Descent, Dungeon Map (2) | The short-name alias "Dungeon" rewrote "venture into the dungeon" | "dungeon" is now a reserved alias word (CR 309) |

Tests (unrun): `crates/ironsmith-compiler-runtime/tests/{copular_contraction_animation,contracted_suspicion_clear,its_controller_manifest_dread,opponent_only_activation,dungeon_game_noun_alias}.rs`.
They share the loader `tests/cf8_p05_support/mod.rs` and the fixture `fixtures/cf8_p05_noverb_b.json.fixture`, which holds the frozen oracle text plus mana cost, type and P/T.

## Already on main (merged PRs #873–876)
Anzrag, Glorfindel and Loathsome Catoblepas are covered by source_must_be_blocked. Journey of Discovery is covered by temporary_additional_land_cap.
Spelljack is not on main: those PRs kept it as a HOLD, so it is listed as blocked.

## Blocked, grouped by missing mechanic (see ledger.jsonl for each card's gap)
- **Attack requirement toward a specific player or in a later combat** (Ruhan, Raving Dead, Ursine Monstrosity, Territorial Hellkite, Sizzling Soloist, Maddening Imp, Arcum's Whistle, Ekundu Cyclops, Nahiri). The only engine support is the turn-scoped `attack_player_requirements`, and only token copies feed it. It needs a new effect that is scoped to one combat (CR 508.1d).
- **Changing which player a creature attacks** (Portal Manipulator, Capricopian, Misleading Signpost, Portal Mage, Windshaper Planetar; CR 506.4).
- **Prevention/redirection shapes** (Immortal Coil, Phyrexian Vindicator, Silhouette, Barbed Wire, Elvish Healer, Battletide Alchemist, Cover of Winter, Blood of the Martyr, Wolverine).
- **Play-from-exile permission variants** (Raphael, Ziatora's Envoy, Ignite the Future, Kayla's Music Box, Spelljack, Gix, Magus of the Mind, Howltooth Hollow, Shelldock Isle, Extract Power, Memory Vessel, Brazen Cannonade, Elkin Bottle, Grinning Totem) and spend-as-any-color permissions (Abstruse Appropriation, Curse of Hospitality, Klaw).
- **Manifest N / from a set / by others** (Omarthis, Write into Being, Kozilek, Jeskai Infiltrator). **Turning a permanent face up as an effect** (Ugin's Mastery, Zimone, Grimoire Thief, Etrata).
- **Ordered "starting with" multiplayer choices** (Grenzo's Rebuttal, Manifold Insights, Rejoin the Fight, The Horus Heresy, The Legend of Yangchen, Thieves' Auction, Whims of the Fates). **"Repeat this process"** (Protection Racket, Firemind's Foresight, Kathril, Timesifter).
- **Time travel** (3), **secret choices/guesses** (5), **controlling a player** (2), **constrained retarget** (2), **additional beginning phase** (2), **cast if able** (2), **follow-up entry counters on a created token** (Ochre Jelly, Printlifter Ooze, Torgal), **cleave sentences inside brackets** (Lantern Flare, Inspired Idea), plus single cards for which the ledger records the exact gap.
- **Comma subtype list + "you control" split** (Vaan, Oakhollow Village, Mirkwood): "put counters on each X, Y, or Z you control" gets split into clauses at the commas. "Destroy/Tap target X, Y, or Z you control" works, so the splitter is specific to put-counter/untap. I could not find it without a build or a fine-grained trace. It is worth one targeted session.

## Risk notes
- The contracted-copula fallback runs only when nothing earlier claimed the clause. An "it's" or "they're" clause that the become grammar can't read still falls through to the old paths, so no clause that previously parsed changes. Clauses starting with "it's still / no / not" are excluded.
- "It's" now reads exactly like "becomes" (same duration defaults, Forever). That is right for one-shot effect sentences. Static abilities are parsed elsewhere.
- The counter-linked-land shape now also accepts the subject "it's" and the form without "in addition". The form without "in addition" is limited to basic land types and lowers to the fixed `BecomeBasicLandType` (SetSubtypes + RemoveLandRulesTextAbilities).
- Two new `ActivationTiming` variants were added to ironsmith-core/src/ability_model.rs. The exhaustive matches are in text rendering, `activation_timing_allows` and condition_eval; the remaining matches use a catch-all. The artifact uses serde for this enum. Any merge that adds other `ActivationTiming` variants or touches `allows_any_player_to_activate` will conflict here.
- The shared hot spots I edited are `clause_dispatch_core.rs` (one block before `find_verb`) and `top_level_readings.rs` (`read_copular_animation`). Both edits are small and additive.

## Round 2 — blocked mechanics implemented as general features

| Mechanic | Cards | Change |
|---|---|---|
| Play-from-exile permission variants (existing GrantPlayTagged / CastTagged machinery) | Raphael, Kayla's Music Box, Gix, Magus of the Mind, Howltooth Hollow, Extract Power, Elkin Bottle, Klaw (8) | Permission tail: free price + exile lifetime in either order. Target "lands and cast spells from among cards exiled this way" (= play those cards, CR 305.1/601.1). "a card exiled with <source>" (source pool, `max_plays` 1) + preprocess short-name replacement after "exiled with". "cards you own exiled with <source>" (owner-narrowed pool). Lifetime "until the beginning of your next upkeep" == until next turn start (no priority in untap step, CR 502.4). Singular any-type rider. Conditional tagged free play takes a general predicate; new fallback predicate "each player has no cards in hand". |
| Attack requirement toward a specific player | Ruhan, Raving Dead, Ursine Monstrosity, Nahiri (4) | New `Restriction::MustAttackPlayer { attackers, player }` (appended variant). Restriction tracker fills `CantEffectTracker::must_attack_players`; `required_attack_players_this_turn` chains it, so attack scoring/preview honour it (CR 508.1d). Duration EndOfCombat/EndOfTurn/leading duration; the named player is bound at resolution. |
| Reselect what an attacking creature attacks | Misleading Signpost, Portal Mage, Windshaper Planetar (3) | New core `ReselectAttackTargetEffect` + engine executor (inside `execute_result_transaction`), AST `PermanentStateActionAst::ReselectAttackTarget`, lowering, decoder/materializer/interpreter registration, text. The effect's controller chooses among the players/planeswalkers/battles the creature could attack. |
| Serial subtype object list + "you control" | Vaan, Oakhollow Village, Mirkwood (3) | `is_subtype_object_list_boundary` in coordination and and-split preservation: the controller relative clause is part of the filter. |
| Turn a chosen permanent face up | Ugin's Mastery, Zimone (2) | Turn-face-up shape accepts "a/an/all/each <filter>". |
| Manifest N from the top | Omarthis (1) | "manifest the top N cards" / "a number of cards ... equal to X" -> repeated single manifest (CR 701.40c). |
| Misc coordination/value | Sphinx of Forgotten Lore, Willowdusk, Lightwielder Paladin (3) | Flashback cost "that card's mana cost". "A or B, whichever is greater" is one amount. An adjacent color list ("black or red permanent") is one qualifier. |
| "Starting with you, each player ..." (infrastructure only) | — | A sentence led by "starting with you" whose body reads as a for-each-player loop is wrapped in `SourceSentence { starting_with_controller }`, making the loop sequential and controller-first. No card is claimed yet: each of the 7 cards still needs its specific choice-pool grammar. |
| Spelljack | already on main | Covered by the merged exact counter/exile permission PR. |

Still blocked after round 2 (see ledger): Abstruse Appropriation (needs a colorless-as-any-color ManaSpendMode), Curse of Hospitality (a permission for a player other than "you" plus a "they may spend" rider), Ignite the Future / Memory Vessel / Ziatora's Envoy / Brazen Cannonade (end-of-combat-next-turn lifetime) / Grinning Totem (upkeep cleanup) / Shelldock Isle (needs a min-library value).
Also still blocked: Sizzling Soloist / Maddening Imp / Arcum's Whistle / Ekundu Cyclops / Territorial Hellkite (further attack-requirement forms), Capricopian (needs an attacked-player activator), Portal Manipulator (forced reassignment), the prevention shapes, repeat-process loops, the other manifest variants (Write into Being needs a manifest flag on PutOntoBattlefield; there are 18 pattern sites), Grimoire Thief and Etrata.

### Round-2 risk notes
- New enum variants change the artifact model: `ActivationTiming::{AnyTimeByOpponents, SorcerySpeedByOpponents}`, `Restriction::MustAttackPlayer` and the new effect type `ReselectAttackTargetEffect`. The orchestrator must bump the artifact schema descriptor and regenerate caches.
- The `PermanentStateActionAst::ReselectAttackTarget` variant was added next to every `RemoveFromCombat` or-pattern (13 sites). Exhaustive Debug/lowering arms were added by hand. A merge that adds other PermanentState variants will conflict there.
- `required_attack_players_this_turn` now also yields restriction-based requirements. `create_token_copy` and the attack preview consume it unchanged.
- The coordination recognizer has three new non-boundary rules (whichever-is-greater, adjacent colors, subtype object lists). They are narrow, but other packages that edit `classify_boundary` will conflict textually.
- `parse_effect_sentence_lexed_uncached_inner` gains a leading "starting with you" reader that falls through on failure.


## Round 3 (on cf8/integration)

| Change | Cards |
|---|---|
| **Time travel as a clause primitive.** The existing sentence-only lowering (time-counter put/remove choice, CR 701.55) now reads anywhere, with "twice", "N times" and ", then time travel" repeats. "time travel" is also a chain-split effect head. | The Parting of the Ways, The Tenth Doctor, The Girl in the Fireplace |
| **New `ActivationTiming::DeclareAttackersStepByAttackedPlayer`.** The activator must be the player the source is attacking, during the declare attackers step. The activator then picks via `ReselectAttackTargetEffect` (players only). | Capricopian |
| **p06's prevention follow-up, reused.** "for each 1 damage prevented this way" now repeats any single follow-up action, not only token creation. | Immortal Coil |
| **Integration hygiene.** `ReselectAttackTargetEffect` moved to the combat decoder family and added to `effect-registry.tsv`. My "attacks <player> if able" reader no longer accepts "attacks you this turn", so it can't be read two ways against p12's `MustAttackPlayerThisTurn`. It keeps "that player" / "a player" / "this combat" / "each combat". | — |

Not done this round, with exact gaps recorded in the ledger:
- **Abstruse Appropriation:** the engine already has per-symbol any-color spending. What's missing is a field to carry it through the GrantPlayTagged AST/effect.
- **Curse of Hospitality:** needs a play permission for a player other than "you".
- **Portal Manipulator:** needs a forced reassignment to a target player.
- **Write into Being / Jeskai Infiltrator:** need a manifest flag on the put-face-down AST.
- **Prevention/redirection shapes:** listed as owned by p03/p06.
- **Parameter-substituting repeat (Firemind's Foresight, Kathril, Protection Racket, Timesifter):** p11's `RepeatProcessEffect` is a condition loop, not this.
- **The 7 "starting with you" cards:** each still needs its choice-pool grammar. The ordering infrastructure from round 2 is in place.


## Round 4 — permission, manifest, attack, repeat and ordered-choice mechanisms

| Mechanism | Cards | Change |
|---|---|---|
| Colorless-as-any-color exile permission | Abstruse Appropriation | New appended `ManaSpendMode::ColorlessAsAnyColor`. `ManaSpendPolicy::allow_mode` turns it into a per-symbol `{C}` conversion (CR 609.4b). The grant registers a stable-id permission with `any_color_mana_symbol: Colorless`. The suffix grammar reads "and you/they may spend [colorless] mana as though ...". |
| Permission for another player | Curse of Hospitality | Permission actor "that creature's controller" maps to `TriggeringSourceController`, the source of the trigger event. The until-end-of-turn tagged grant keeps that grantee. |
| Manifest flag on put-face-down | Write into Being, Jeskai Infiltrator | `ZoneMoveActionAst::PutOntoBattlefield.manifest`, lowering to `ManifestObjectsEffect` without cloak (CR 701.40a). Face-down piles read as a single sentence and accept "the top card" and "then manifest those cards". A new looked-procedure statement handles "Manifest/Cloak N of those cards, then put the other on the top or bottom / the rest on the bottom". |
| Forced attack onto a target player | Portal Manipulator | `ReselectAttackTargetEffect.attacked_player` (CR 506.4). A creature whose controller can't attack that player keeps its attack. New clause "<creatures> are now attacking <that player/you>". In choose-target preludes, "their/they" binds to the earlier target player, and the engine links `controller: OpponentOf(Target)` creature targets to the prior player target through a target pair constraint. |
| Repeat with new values | Firemind's Foresight (Kathril: partial) | "[Then] repeat this process for <numbers / keyword list>" re-reads the previous instruction once per value. It substitutes the one number, or every occurrence of the keyword (pair shape, head Any). |
| Following process per opponent | Protection Racket | "Repeat the following process for each opponent [in turn order]" wraps the rest of the ability. Shapes consume 2–6 sentences, up to the end of the ability. The result is a ForEachOpponent ordered from the controller. |
| Tie-break repeat | Timesifter | New `TagPlayersEffect` and `KeepGreatestManaValuePlayersEffect` in the player family, plus the AST `GreatestManaValueTieBreakExile`. The lowering is a seed followed by `RepeatProcessEffect{contenders exile top card; keep greatest}` while more than one player is tied. The winner's action runs through ForEachTaggedPlayer. |
| Ordered per-player choices | The Horus Heresy, The Legend of Yangchen, Grenzo's Rebuttal, Rejoin the Fight, Manifold Insights | The ordering adds "starting with the next opponent in turn order" and a leading "then". New choice pools: "from among them", "from among permanents <relation>" (merged into the filter), "a different", and "that hasn't been chosen". New appended `PlayerFilter::PlayerToLeftOf(base)` for "the player to their left" (CR 101.4a). Type-slot choices may name the slots before the pool. A new normalization binds "those permanents" / "each card chosen this way" after a quantified choice to the accumulating chosen set. A 3-sentence program covers reveal, ordered choice, and the chosen-to-hand/rest-to-bottom split. |

Tests (unrun): `crates/ironsmith-compiler-runtime/tests/{exile_play_mana_spend_permissions,manifest_face_down_selection,forced_attack_reassignment,repeat_process_variants,ordered_player_choices}.rs`. Engine unit tests were added in grant_play_tagged.rs, reselect_attack_target.rs, filter.rs (player_to_left_of_tests) and player/tie_break.rs.

Still blocked after round 4:
- **Kathril:** the aggregate "for each counter put on a creature this way" across the repetitions.
- **Thieves' Auction:** needs a per-pick tag plus a durable chosen set to guarantee the loop terminates.
- **Whims of the Fates:** three-pile separation (p01).
- **Other packages' p05-tagged permission dependants** (p02/p03/p04/p07 ledgers) are not addressed. Each needs its own permission shape: once-per-turn filtered casts, additional-cost casts, color-to-color conversion, ownership mana scopes.

### Round-4 risk notes
- **New serialized variants**, all appended to keep ordinals: `ManaSpendMode::ColorlessAsAnyColor` and `PlayerFilter::PlayerToLeftOf`. New fields: `ReselectAttackTargetEffect.attacked_player` (serde default). New effects: `TagPlayersEffect` and `KeepGreatestManaValuePlayersEffect`, registered in the decoder, materializer, interpreter and effect-registry.tsv. The artifact schema descriptor needs a bump.
- **`ManaSpendMode::combine` is now an explicit lattice instead of `max`.** `allows_any_color()` is false for the colorless mode.
- **`PlayerToLeftOf` arms** were added next to every `OpponentOf` site in core, engine, resolve, lowering, text and grammar. Any exhaustive `PlayerFilter` match I missed will show as a compile error.
- **AST changes:**
  - `PutOntoBattlefield.manifest` and `ReselectAttackTarget.attacked_player`: the fully destructuring sites were updated.
  - `EffectAst::GreatestManaValueTieBreakExile`: added in visit.rs coverage, reference_resolution and lowering.
- **Shared hot spots touched:**
  - `permission_helpers.rs`: an until-end-of-turn guard.
  - `pair_procedure.rs`: new shapes at the head of `PAIR_SHAPES`.
  - `effect_ast_normalization.rs`: one new binder in the pipeline.
  - `choices.rs`: suffixes, "different", and the pool merge.
  - `targeting.rs`: the opponent-of-target pair link.
  - `dispatch_entry.rs`: `parse_effect_sentences_from_sentence_inputs` is now `pub(super)`.
- **Behaviour change in the pronoun binder.** A plural "it/those …" consumer of exile, move or return right after a choice made by each player now takes the union of the choices instead of the last one.
- **Behaviour change in the suffix grammar.** "and they may spend mana as though ..." is now a permission rider. Before, it was a split clause.


## Round 5 — other packages' p05 dependants, Thieves' Auction

I searched every package ledger for blocked rows that need a p05-owned mechanism and found 50. Each one is now recorded in this ledger with `owner_package` set. Shapes built:
- **3 dependant-proposed**, built as source extensions of the exile-pool permission (`tagged_surface.rs`, `exiled_top_procedure.rs`), with tests in `tests/p05_dependant_exile_pool_permissions.rs` and fixture `fixtures/cf8_p05_dependants.json.fixture`:
  - **King Narfi's Betrayal (p04):** a "spells from among cards exiled with this Saga" target over the source-linked pool.
  - **Chandra, Flameshaper (p07):** the exiled-top statement "Choose one [of them]." picks one exiled card, then "You may play that card this turn" grants permission over it.
  - **Nahiri, Forged in Fury (p04):** "You may cast <filter> spells this way without paying their mana costs" tags the matching exiled card and grants the free cast over it.
- **Thieves' Auction (own package):** now source-proposed. The program tags the exiled pool, then repeats a round-robin ordered from the controller while an unchosen pool card remains in exile:
  1. Reset this player's pick tag.
  2. The player chooses one unchosen card from the pool.
  3. They put it onto the battlefield tapped under their control.
  4. The pick joins the shared chosen set.

  A pick that can't enter (CR 303.4g) stays chosen, so the loop always terminates.
- **Kathril:** still blocked; the ledger records the exact gap. I rejected counting how many keywords are present as lossy, because it miscounts under counter-doubling replacements.

The 47 dependant-blocked rows, grouped by the shape each still needs:
- **Static zone permissions** (graveyard/library/linked exile, plus additional costs, during-your-turn windows, once-per-turn or once-per-type budgets, conditions; 20 cards): Muldrotha, Share the Spoils, Azula, Dawnhand Dissident, Evendo Brushrazer, Festival of Embers, Theater of Horrors, Shared Fate, Uba Mask, Arcade Gannon, Maralen, Banon, Null Summoner, Falco Spara, Hedonist's Trove, Into the Pit, Noctis, Quilled Greatwurm, Qasali Ambusher, Fblthp.
  - All of these are unsupported-line-family statics that would need routing in `static_ability_rule_head_hints`.
  - The engine already has `DerivedAlternativeCast::GraveyardCastFromCardManaCost{additional_costs, usage_limit, condition}` for the graveyard additional-cost subset.
- **Attack requirements and permissions** (9 cards):
  - Player-scoped requirements: Seeker, Trove.
  - Planeswalker target: Gideon Jura.
  - Per-target "other chosen player": The Brothers' War.
  - Directional: Teyo.
  - Most-life selector: Galactus.
  - Attack-target-scoped haste: Frenzied Saddlebrute.
  - Filter subject: Imaginary Threats.
  - Token copies attacking each other opponent: Shredder (p12 token shape).
- **Mana-spend scopes** (4 cards):
  - Single color to single color: Sunglasses of Urza.
  - Owner-relative scopes: Nathan Drake.
  - One-spell grant: North Star.
  - Ability-scoped rider: Grell Philosopher.
- **Exile-pool permissions still open** (6 cards):
  - "During your next turn" window: Galvanic Relay.
  - Delayed "if you haven't cast it": Planeswalker's Mischief.
  - Consult-result "exiled nonland card" inside "if you don't": Black Widow.
  - Exactly-three-colors filter: Meeting of the Five.
  - Pool spans two exiles: Kotose.
  - Waterbend price: Hama.
- **Cast from another player's zone** (4 cards): Whispersteel Dagger, Tinybones; Sen Triplets and Xanathar also need p10's restrictions.
- **"As many times as you choose"** (2 cards): Dance with Calamity, Lim-Dûl's Vault (owned by p11).
- **Granted quoted permissions** (2 cards): Lukka, Monk Class.

### Round-5 risk notes
- `exiled_top_procedure.rs` has a new `Statements::Chosen` state and two new statement arms. The procedure now opens on "choose one" + play permission, or on play permission + free-cast rider.
- The Thieves' Auction program resets the pick tag by taking the union of a helper tag that is never written. If a reference-ledger validator rejects consumed-but-unproduced tags, it will flag it.
