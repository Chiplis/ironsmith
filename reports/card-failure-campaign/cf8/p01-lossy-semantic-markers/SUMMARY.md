# cf8 p01 — lossy compilations and dropped semantic markers

Branch `cf8/p01-lossy-semantic-markers` (worktree `ironsmith-cf8-p01`), off origin/main 84ea8b41c.
Source-only: nothing was built or run. The prebuilt `compile_oracle_text` was used for
diagnostics until it disappeared mid-session (main rebuild); later work is reviewed by reading.

Status (round 3): 90 already-on-main, 44 source-proposed, 36 blocked, 3 semantic-fix-collateral (170 package cards + 3 collateral).
Ledger: `ledger.jsonl`. Fixture with full typed bodies: `fixtures/p01_lossy_semantic_markers.json.fixture`.

## Clusters, root causes and fixes

### speculative-probe-loss (93: 90 already-on-main, 3 source-proposed)
Root cause: `parse_best_object_filter_suffix` records `suffix_object_filter_recovery`
immediately, and the new conditional copular reader ran the legacy sized-animation reader as an
uncaptured ownership probe; that probe read a subject up to any is/are/it's/its copula and then
declined, leaking the loss into an unrelated committed reading (every recovered span ends before a
copula). Fixed on main by cba78f342 (PR #873, captured probe). This branch adds a general backstop:
`parse_static_ability_ast_line_lexed` now captures its whole run and replays losses only when it
returns static abilities (same ownership rule the static registry already used for candidates), so
an abandoned whole-line static probe can no longer taint the trigger/activated/effect reading that
owns the line. That is what the three pre-Oct7 losses (Prydwen, Brenard, Hofri — spans start at
"whenever ...") need. Mathemagics is separate (blocked).
Files: `keyword_static/mod.rs`. Test: `speculative_probe_loss_ownership.rs` (all 93 bodies, both routes, loss-free).

### waterbend-additional-cost-render (4, source-proposed)
Mandatory "As an additional cost ..., waterbend {N}/{X}" is a waterbend-scoped `ManaPaymentCost`
in `additional_cost`; the renderer printed only non-mana components, dropping the line. Now renders
`ManaCost::payment_surface()`. Files: `text/.../ast_render.rs` (small additive hunk).

### fixed-mana-alternative-price-ambiguity (5, source-proposed)
"You may pay <mana> rather than pay the mana cost for <spells> you cast" matched both the fixed
alternative-mana-cost grant and `parse_independent_alternative_price_line` (its "pay rather"
deferral never fired because mana groups become word pieces). Different ASTs ⇒ registry ambiguity ⇒
the effect parser silently kept "You may pay {..}". The independent reader now defers pure-mana
prices. Files: `keyword_static/alternative_prices.rs`. Gap noted: the grant is hand(+command)-scoped.

### draw-replacement-ambiguity (2, source-proposed)
`parse_if_you_would_draw_instead_effects_line` did not defer to `DrawReplacementDouble` or
`DrawReplacementExileTopFaceDown`; ambiguity sent Thought Reflection / Asmodeus to a resolving
"If custom condition you_would_draw_card" effect. Added both exclusions and a flavor-label strip in
the face-down reader. Files: `keyword_static/costs_replacements_and_permissions.rs` (2 small hunks).

### tagged-reference-render (9: 6 source-proposed, 3 blocked)
Renderer-only: `chosen_exiled` helper tag ⇒ "that card" (play-until-next-turn); hand-zone
`PlayerTaggedObjectMatches` ⇒ "you put <card> into your hand this way"; ForEachTaggedPlayer over
`coin_opponents_lost` ⇒ "For each flip you lose", and over any this-way action tag ⇒ "For each
player <action> this way". Blocked: Aurelia's Fury (tap filter lacks Creature), Hog-Monkey Rampage
and Stolen Uniform (conditions bound to the shared `__chosen_objects__` set / wrong object).
Files: `effect_impl/late.rs`, `normalize_common/condition_rendering.rs`.

### stale-fail-loud-guards (5: 3 source-proposed, 2 blocked)
Exact-text "unsupported" rules in `grammar/document_shapes/unsupported.rs` blocked lines whose owners
now exist: Tetsuko (power-or-toughness disjunction filter; also removed the static-line guard),
Hellraiser Goblin (granted keyword + must-attack split already tested), Leviathan (new
`keyword_static/enters_tapped_untap_conjunction.rs` splits the source "enters tapped and doesn't
untap" line into its two owned statics). Death Cloud / Rebuild the City rules kept (not verifiable
without a build).

### each-basic-land-type (4, source-proposed)
Coalition Victory was misread as "a land with a basic land type and a creature". New predicate
reading `predicate_phrases/each_quality_control.rs` lowers "you control a <noun> of each basic land
type / color" to five `PlayerControls` leaves per quality (dual lands satisfy several types), and
the condition renderer folds them back. Planar Overlay / Sundering Titan already lowered five
per-type choices; new `effect_list/basic_land_type_choices.rs` renders the run. Tromp the Domains:
`ModifyPowerToughnessAll` reuses `describe_basic_land_type_pt_for_each`.

## Second pass (coordinator follow-up)

### Silent miscompiles fixed (semantic bugs)
- Aurelia's Fury: "Tap each creature dealt damage this way" tapped every remembered damage
  recipient; now TapAll(creature ∧ tagged damaged_0) (`subject_verb_followups.rs`), rendered
  without tags (`effect_impl/early.rs`).
- Hog-Monkey Rampage: a trailing-if "it" over a remembered set now inherits the consequence
  object's qualifiers (new `compile_support/trailing_if_antecedent.rs`, wired in the TrailingIf
  lowering) — "it" means the creature you control, not either chosen creature.
- Stolen Uniform: delayed "When you lose control of that Equipment this turn" now watches the
  remembered Equipment (`ScheduleDelayedTriggerEffect::from_tag` + ControlChanged, CR 603.10d) and
  binds body pronouns to it; new complete-shape predicate reading "it's attached to X"
  (`predicate_phrases/pronoun_attached_to.rs`). The predicate registry now lets complete-shape
  readings (`COMPLETE_SHAPE_READINGS`) own their input over partial readings.
- Mathemagics (2ˣ read as 2), April O'Neil (card types among spells read as spell count), Winter
  (set-wide "card types among them" read per card): new `Value::PowerOfTwo` and
  `Value::CardTypesAmongSpellsCastThisTurn` (4 exhaustive engine matches updated), 'ˣ' kept as a
  word piece, lexer accepts superscript digits and '=' (exponent reminder), and
  "with N or more card types among them" becomes a `DistinctCardTypes` set constraint.
- Collateral grep of cards.json found no other card with these exact wordings (Nethergoyf's
  escape cost uses its own parser; Ogre Geargrabber's undated lose-control trigger is untouched).

### Mechanisms implemented
- Triggered abilities inside level-up ranges (CR 711.2a): new
  `ParsedLevelAbilityItemAst::TriggeredAbility`, lowered with an event-time `ConditionQualified`
  level gate (not an intervening if) and rendered under its LEVEL header.
- Death Cloud fail-loud rule retired (the each-player chain already owns Pox's shape).
  Rebuild the City's rule kept: it depends on token-copy exceptions owned by p12.
- Dead `cfg(ironsmith_runtime_parser_tests)` expectations (engine shard_07/09/10) updated.

## Round 3 (on cf8/integration)

- Resolving-spell destination replacement (owned mechanism): "exile that card/spell [with N
  <counter> counters on it] instead of putting it into your graveyard as it resolves" now registers
  a one-shot RegisterZoneReplacement on the triggering spell (Stack→Graveyard ⇒ Exile, CR 608.2n /
  614.1a) instead of exiling it on cast — fixes the Goliath Daydreamer and Lilah silent miscompiles.
  Lilah's "If you do, it becomes plotted" is a new `LinkedExileFollowUp::BecomePlotted`, executed
  only when the replacement exiles the card. Collateral: Gandalf of the Secret Fire's first sentence
  (its suspend follow-up still needs the same follow-up treatment). Quintorius stays blocked
  (future replacement without a library-bottom destination).
- Random targets (owned): "<target> chosen at random" keeps `ChoiceCount.random`; casting,
  activation and trigger target announcement narrow the requirement to a uniform pick from the
  replayable random stream (`targeting/random_targets.rs`). Collateral: Scab-Clan Giant, Power Pack.
- As-you-activate snapshot (owned): `ValueSurfaceHint::AsYouActivateThisAbility` on where-X
  bindings (Bobbleheads, Lukka). Keeper of the Beasts still needs a player-filter reading.
- Piles (owned): binary-pile program gains a graveyard-pool producer and an exile/battlefield
  destination (Death or Glory). Ecological Appreciation / Abstract Performance not done.
- Level-up triggers were done in round 2. Echo / first-each-turn cycling and power-up
  alternatives: not done — no machinery exists (note: FirstEquipCostAlternative is display-only,
  a likely silent miscompile for Bruenor-style cards outside this package).
- Re-check of "needs pNN" dependants against the merged tree: p06's generic instead replacement
  covers damage/life-gain events only (not destruction or cross-paragraph self-replacement); p09's
  shares-a-card-type predicate covers "with that permanent / the exiled card / that spell" but not
  "the card exiled this way" / "the card you discarded" / reveal-until; same-name forms still
  missing; p12 copy exceptions not verified for Rebuild the City. All stay blocked.

## Blocked, grouped by missing mechanic
- Owned by other packages: generalized "instead" replacements (p06): Epicenter, Orim's Touch,
  Archmage's Newt, Crackling/Harmonious Emergence, Gideon's Triumph (+ attacked-or-blocked filter);
  shares-a-card-type / same-name (p09): Creeping Dread, Holistic Wisdom, Reality Scramble, Wild
  Magic Surge, Locket of Yesterdays, The Apprentice's Folly, Yenna; copy abilities (p12): Vesuvan
  Doppelganger, Rebuild the City.
- Random targets (engine has no random target announcement; hook points in priority_cast.rs and
  sba_triggers.rs recorded in the ledger): Goblin Test Pilot, Witch Hunt; random hand reveal bound
  to a value: Singe-Mind Ogre.
- Resolving-spell destination replacement with counters / follow-ups / library bottom (extend
  RegisterFutureZoneReplacementEffect): Goliath Daydreamer, Lilah, Quintorius (currently a silent
  miscompile for Goliath/Lilah — the spell is exiled on cast).
- Counter replacement: Guile. Piles/divvy: Death or Glory, Ecological Appreciation, Abstract
  Performance. Echo-cost / first-each-turn cycling and power-up alternatives: Thick-Skinned Goblin,
  Gavi, Advancing the Spirit.
- Activation-time value surface ("as you activate this ability"): Agility/Endurance Bobblehead,
  Lukka; Keeper of the Beasts (target-player filter missing).
- Command zone: Liesa, Next of Kin, Stinging Study, The Ur-Dragon. Draft-matters: Paliano
  Vanguard, Smuggler Captain, Volo. Misc: Atomic Microsizer (conjoined can't-be-blocked dropped),
  Rekindling Phoenix (token quoted ability unverified), Trial of Agony, Aluren, The Ruinous
  Wrecking Crew (self-name rendering vs "crew" marker).

## Risk notes
- `parse_static_ability_ast_line_lexed` loss ownership: losses from a static probe that returns
  None/Err are now dropped. A caller that relied on a declined static reading to report loss would
  lose that report — but such a reading committed nothing, so the loss was never the committed
  body's. The memoized `broad_static` cache was already non-replaying.
- Removed fail-loud rules: dead `cfg(ironsmith_runtime_parser_tests)` engine tests (shard_07,
  shard_09, shard_10) still assert the old errors.
- Cross-package: `unsupported.rs` RULES table, `alternative_prices.rs`, `condition_rendering.rs`,
  `effect_impl/late.rs` and `early.rs` are shared; hunks are small and additive. `keyword_static/mod.rs`
  gained a wrapper around the static line entry plus one reader call — likely merge touch point.
- History note: an early commit accidentally included `.cargo/config.toml`; before the no-rewrite
  rule was announced the branch was rewritten locally (never pushed) to drop it.
- New `Value` variants and `ParsedLevelAbilityItemAst::TriggeredAbility` touch shared enums; the
  exhaustive matches found by scanning (dependency.rs x2, text_change_predicates.rs, value_eval.rs)
  were updated, but a build must confirm no other exhaustive match exists. The lexer regex change
  (superscripts, '=') affects every card's lexing.
