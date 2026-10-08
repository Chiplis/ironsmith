# cf8 p05-noverb-b — "could not find verb in effect clause" (153 cards)

Status (round 3): 47 source-proposed, 5 already-on-main, 101 blocked, 0 untriaged.
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
