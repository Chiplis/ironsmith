# p02-linefamily-a — summary

Package: 149 cards failing with "parser does not yet support line family" (rule-path
`unsupported-line-family`). Branch `cf8/p02-linefamily-a`. Nothing was built or run (campaign
policy); the hints below come from reading code plus read-only probes with a prebuilt
`compile_oracle_text` (sf-b4 tree, Oct 8 00:24, run with `--cards`), which does not contain
these changes.

`unsupported-line-family` is only the final fallback: every line family declined the line. The
real root causes were varied (missing head hints, missing grammar surfaces, missing typed
mechanics), so the package splits into many small clusters rather than one.

## Risk notes
- `leading_condition_wrapper.rs` is a new last-resort registry rule ("During your turn, <static>"
  / "As long as <cond>, <static>"). It only claims a line when the full static parse returns
  `Ok(None)` (thread-local reentrancy guard), requires a single sentence (periods inside quotes
  ignored) and refuses pronoun remainders unless the condition is about "this" object.
  Lines that previously failed as unsupported may now compile through it; lines that compiled
  before are unaffected by construction.
- `leaf::recognize_target_head` (grammar-common) now commits on singular card-type nouns
  (artifact/enchantment/land/planeswalker/battle) like it already did for "creature".
- `DevourEffect` gained serde-defaulted fields (`quality`, `multiplier_is_devoured_count`);
  existing artifacts decode unchanged.
- New core payload `StaticAbilityPayload::SetBaseToughness` (additive; try_map arm, engine
  interpreter and text-change arms added). Other exhaustive matches over the payload use
  wildcards (checked by grep).
- New `ThisSpellCastTiming` variants (exhaustive match in `decision/mana.rs` updated).
- `KeywordAction::Provoke` now `lowers_to_static_ability` and is an executable grant (like exalted).
- Commit history: one early commit accidentally included `.cargo/config.toml`; it was removed by
  rewriting the two branch commits before the no-rewrite rule was announced. No later rewrites.
- Cross-package: mechanisms owned by other packages are marked "needs ... (owned by pNN)" in the
  ledger (p05 attack requirements / exile play permissions, p06 would-instead replacements,
  p09 choices/votes, p10 restrictions and library exile-until, p12 ability copying).

## Round 3 (on cf8/integration)
- Shared casting-keyword grant path (`keyword_static/granted_casting_keywords.rs`, replacing the
  warp-only rule): warp, prowl, freerunning, miracle granted in hand (spell subjects map to those
  cards in hand, so no new zone permission); jump-start granted in the graveyard.
- Granted encore with derived costs (`KeywordAction::EncoreFromSourceCost`, graveyard grant).
- Commander ninjutsu (`KeywordAction::CommanderNinjutsu`, Hand+Command functional zones).
- Retarget "The new target must be ..." (`StackActionAst::RetargetStackObject.new_target_restriction`).
- Paid-method cost modifiers: the existing rule was unreachable except for flashback; head hints
  added, and a trailing "for each ..." is now owned instead of silently dropped.
- Temporary damage multipliers: duration before recipient, and no-recipient form.
- Rampage possession keyword (Rapid Fire); Sands of Time is handled by p03's skip-untap static.
- Still blocked: granted replicate/offspring/demonstrate/sneak/madness (need a typed granted
  optional-cost / trigger mechanism; madness is p06's replacement family), and the remaining
  cast-timing spell bodies (Berserker's Frenzy, Camouflage, Illusionist's Gambit, Siren's Call).

## Round 4
- Typed granted spell keywords (`StaticAbilityPayload::GrantSpellKeyword`, `StaticAbilityId::GrantSpellKeyword`,
  core `granted_spell_keyword_model.rs`): replicate (fixed or "equal to its mana cost"), offspring, conspire,
  demonstrate granted to spells. Engine `granted_spell_keywords.rs` discovers every applicable grant (battlefield
  statics and grants attached to the spell) at cast time (CR 601.2b): costs become optional costs with a per-grant
  discriminator; native Replicate/GrantedConspire copy triggers fire from the paid costs; offspring attaches its
  CR 702.175a ETB trigger to the spell's incarnation; demonstrate triggers are synthesized on SpellCast.
  The conspire display-text match (`granted_conspire_count`) and marker grant are gone; Wort / Raiding Schemes /
  Rassilon now compile to the typed grant (collateral, not package cards). Grammar: `granted_spell_keywords.rs`
  plus the anthem-grant conspire branch (now also demonstrate).
- Granted sneak: shared casting-keyword path grants a Composed "Sneak" cost in hand or graveyard; new
  `AlternativeCastFromZoneForFilter` permission ("cast creature spells from your graveyard using their sneak
  abilities", `AlternativeCastKeyword::Sneak`) admits printed or granted sneak methods in legal-action enumeration;
  SneakCostEffect accepts a graveyard source.
- Granted madness: `DerivedAlternativeCast::MadnessFromCardManaCost` granted in hand/exile/graveyard/library; the
  engine-native DiscardWithMadness replacement (not p06's EventReplacementWithEffects) now also applies when madness
  is granted, and MayCastForMadnessCostEffect casts via the granted method from exile. Granted madness can't be an
  ordinary cast from exile (authorization guard in legal actions).
- Cast-timing bodies: referenced-creature combat sentences (`attacks that combat if able`, `can't attack you or
  planeswalkers you control that combat` → RestrictionStart::LastAddedCombatPhase; `They block this turn if able`
  → MustBlock); "Roll two d20 and ignore the lower roll" (`RollDiceChooseResultEffect.ignore_lower`); player-relation
  subject "the active player"; block-declaration control already existed (ControlCombatChoicesThisTurn).
- Taunt: next-turn requirement accepts "attack you"; engine binds the targeted controller in MustAttackPlayer.

### Round 4 risk notes
- New appended enum variants: StaticAbilityPayload::{GrantSpellKeyword, AlternativeCastFromZoneForFilter},
  StaticAbilityId::{GrantSpellKeyword, AlternativeCastFromZoneForFilter}, AlternativeCastKeyword::Sneak,
  DerivedAlternativeCast::MadnessFromCardManaCost, SentencePreludeShape::RollDiceIgnoreLower; new field
  RandomActionAst::RollDiceChooseResult.ignore_lower and RollDiceChooseResultEffect.ignore_lower (core serde default,
  engine struct). Arms added in core try_map, text_change_statics, compiler-runtime convert, artifact materializer,
  engine grant materialize, sentence registry.
- `AlternativeCastingMethod::keyword()` now maps a Composed method named "Sneak" to `AlternativeCastKeyword::Sneak`.
- Granted offspring: a printed offspring trigger's discriminator-less paid query also sees a granted payment.
- Shard_06 engine test now attaches the typed conspire grant instead of a KeywordMarker.
- Referenced-creature readers rely on the It tag binding to "those creatures"/"they" (unverified without a build).
- Still blocked here: Siren's Call (exception sentence + delayed "that player" binding), Camouflage (pile-based
  random block assignment).

## Round 5
- Offspring instances are separate (CR 702.175b): printed offspring now carries its own `printed-N` discriminator
  (lowering `materialize_optional_cost`), so its ETB trigger checks only its own payment; a granted instance
  already had its own. Test: printed offspring + Zinnia paying 0, 1 or 2 costs creates 0, 1 or 2 tokens.
- Verified by reading: referenced-creature restrictions resolve `It` through `resolve_restriction_it_tag`
  (MustAttack, MustBlock, AttackPlayerOrPlaneswalkersControlledBy) against the lowering reference env, whose last
  object tag comes from RemoveFromCombat/untap (Illusionist's Gambit) or ChooseObjects (Berserker's Frenzy); the
  engine collapses the tagged filter to exactly those creatures.
- Siren's Call: new line family `continuous-control-exception` folds "Ignore this effect for each creature the
  player didn't control continuously since the beginning of the turn" into the preceding instruction's filter via
  the existing continuous-control predicate and binds "that player" to the active player when the card's previous
  player-naming line names the active player (claimed only then).
- Final-Word Phantom: new `Condition::OpponentsEndStep` (appended) and a "During each opponent's end step," leading
  condition over a complete static.
- Nahiri, Storm of Stone: the leading-condition wrapper reads two complete statics joined by "and" (only when the
  whole remainder isn't one static, exactly one split reads, and the right half isn't a bare keyword list).
- Magnigoth Treefolk: domain landwalk as five land-type-conditioned landwalk statics.
- Taunt (round 4 tail): next-turn requirement "attack you" plus targeted-controller binding for MustAttackPlayer.

### Round 5 risk notes
- `LINE_FAMILY_RULES` grew to 33 entries; the new continuous-control rule runs first and re-dispatches a rewritten
  line (synthetic tokens "the active player has controlled continuously since the beginning of the turn").
- `Condition::OpponentsEndStep` arms: condition_eval, dependency, text_change_predicates, condition_rendering.
- Engine-side dead (cfg ironsmith_runtime_parser_tests) offspring tests still expect the plain "Offspring" label.

## Source-proposed clusters
### absorb-keyword (1): Lymph Sliver
- Fix: Absorb had no grammar. New registry rule lowers 'Absorb N' and '<subject> have absorb N' to the existing PreventMatchingDamage self-prevention (amount N, target = this object) that the spelled-out CR 702.64a sentence already compiles to (probe), granted via GrantStaticAbility.
- Files: crates/ironsmith-compiler-grammar/src/keyword_static/absorb_keyword.rs, crates/ironsmith-compiler-grammar/src/keyword_static/mod.rs
- Test: crates/ironsmith-compiler-runtime/tests/absorb_keyword.rs::lymph_sliver_grants_absorb_one_to_slivers
### adjective-led-entry-counter-subject (1): Curator Beastie
- Fix: parse_enters_with_additional_counter_for_filter_line (and enters tapped/untapped for filter) existed but its head-hint list had no color/supertype adjectives, so 'Colorless creatures you control enter with ...' was unreachable; added colorless/multicolored/monocolored/colors/legendary/nonlegendary/noncreature/nonland heads.
- Files: crates/ironsmith-compiler-grammar/src/keyword_static/mod.rs
- Test: crates/ironsmith-compiler-runtime/tests/adjective_led_entry_counter_subjects.rs::colorless_creatures_enter_with_two_additional_counters
### as-turned-face-up-replacement (3): Bubble Smuggler, Hooded Hydra, Gift of Doom
- Fix: Statement-sentence normalization only stripped 'As this X enters/transforms into,' intros, so 'As this X is turned face up, <instruction>' never reached the effect body. Line facts already set turns_face_up_only and line lowering already emits the face-up-only AsEntersEffectProgram (engine runs it at turn-face-up, CR 702.37/708.8); the strip now recognises the face-up intro.
- Files: crates/ironsmith-compiler-grammar/src/document_parser/statement_recognition.rs
- Test: crates/ironsmith-compiler-runtime/tests/turned_face_up_replacement_programs.rs::counters_arrive_only_when_turned_face_up
### bare-card-type-selection-head (1): Phylactery Lich
- Fix: Probe (sf-b4 binary): 'put a +1/+1 counter on a creature you control' parses but '... on an artifact you control' fails 'unrecognized target or selection phrase' because leaf::recognize_target_head committed on the noun 'creature' but not other singular card-type nouns. Added artifact/enchantment/land/planeswalker/battle heads; as-enters body then lowers through the existing AsEntersEffectProgram path.
- Files: crates/ironsmith-grammar-common/src/grammar/leaf/outcomes.rs
- Test: crates/ironsmith-compiler-runtime/tests/bare_card_type_selection_heads.rs::phylactery_lich_marks_a_chosen_artifact_as_it_enters
### base-pt-keyword-list (1): Timber Paladin
- Fix: Probe: the aura-count conditions and 'has base power and toughness N/M and has <kw>' compile; only the elided second 'has' ('5/5 and vigilance', '10/10, vigilance, and trample') failed. The base-P/T grant shape now treats the second verb as optional, feeding the existing heterogeneous granted tail.
- Files: crates/ironsmith-compiler-grammar/src/grammar/anthem_grants/tail_static_shapes.rs
- Test: crates/ironsmith-compiler-runtime/tests/base_pt_keyword_lists.rs::timber_paladin_tiers_compile_with_their_keywords
### base-toughness-only (1): Maha, Its Feathers Night
- Fix: No toughness-only base setting existed (only SetBasePower). Added additive core payload SetBaseToughness{filter,toughness} (try_map arm, constructor), engine SetBaseToughnessForFilter (layer 7b Modification::SetToughness, sublayer Setting, CR 613.4b) in a new continuous submodule, model-interpreter + text-change arms, and a registry rule for '<subject> have base toughness N'.
- Files: crates/ironsmith-compiler-grammar/src/keyword_static/base_toughness_line.rs, crates/ironsmith-compiler-grammar/src/keyword_static/mod.rs, crates/ironsmith-core/src/static_ability_model.rs, crates/ironsmith-engine/src/continuous/text_change_statics.rs, crates/ironsmith-engine/src/static_abilities/continuous.rs, crates/ironsmith-engine/src/static_abilities/continuous/base_toughness.rs, crates/ironsmith-engine/src/static_abilities/mod.rs, crates/ironsmith-engine/src/static_abilities/model_interpreter.rs
- Test: crates/ironsmith-compiler-runtime/tests/base_toughness_only.rs::maha_sets_only_opposing_base_toughness
### bounded-named-deck-limit (2): Nazgûl, Seven Dwarves
- Fix: is_named_deck_construction now also accepts 'a deck can have up to <N> cards named X' -> DeckConstructionRuleText; wasm pregame deck_construction_copy_limit already parses 'a deck can have up to N'. Other lines already compile.
- Files: crates/ironsmith-compiler-grammar/src/grammar/semantic_lowering/static_shapes.rs
- Test: crates/ironsmith-compiler-runtime/tests/bounded_named_deck_limits.rs::bounded_named_deck_rule_is_a_deck_construction_rule_on_both_routes
### cast-this-spell-only-timing (1): Rapid Fire
- Fix: Timing: new ThisSpellCastTiming::BeforeBlockersAreDeclared. Body: probe showed 'If it doesn't have flying, ...' compiles but 'rampage' was not a possession keyword; added Marker('rampage'), which matches the printed/granted 'rampage N' keyword marker (CR 702.23).
- Files: crates/ironsmith-compiler-grammar/src/grammar/shared_util/cast_restriction_lines.rs, crates/ironsmith-compiler-grammar/src/grammar/shared_util/reference_shapes/reference.rs, crates/ironsmith-core/src/spell_timing_model.rs, crates/ironsmith-engine/src/decision/mana.rs
- Test: crates/ironsmith-compiler-runtime/tests/cast_timing_windows.rs::rapid_fire_compiles_with_its_window_and_conditional_rampage
### commander-ninjutsu (1): Yuriko, the Tiger's Shadow
- Fix: New KeywordAction::CommanderNinjutsu(cost) (grammar rule emits it; lowering builds the ninjutsu ability with functional zones Hand+Command, CR 702.49d). NinjutsuEffect/NinjutsuCostEffect source-zone checks accept the command zone; the ability's functional zones still gate plain ninjutsu to the hand. Command-zone abilities are already enumerated by collect_non_battlefield_source_ids.
- Files: crates/ironsmith-compiler-grammar/src/keyword_static/commander_ninjutsu.rs, crates/ironsmith-compiler-grammar/src/keyword_static/mod.rs, crates/ironsmith-compiler-lowering/src/card_builders.rs, crates/ironsmith-compiler-lowering/src/keyword_actions.rs, crates/ironsmith-compiler-semantic/src/payload.rs, crates/ironsmith-engine/src/effects/permanents/ninjutsu.rs
- Test: crates/ironsmith-compiler-runtime/tests/commander_ninjutsu.rs::yuriko_ninjutsu_functions_from_hand_and_command_zone
### counted-number-sentence (1): Invincible Hymn
- Fix: 'Count the number of X.' has no verb the effect grammar knows; statement sentence normalization now inlines 'the number of X' into the following sentence's single 'that number' anaphor (probe: 'Your life total becomes the number of cards in your library.' compiles).
- Files: crates/ironsmith-compiler-grammar/src/document_parser/statement_recognition.rs
- Test: crates/ironsmith-compiler-runtime/tests/counted_number_sentences.rs::invincible_hymn_resolution_uses_current_library_count
### devour-quality-variants (4): Caprichrome, Feasting Hobbit, Famished Worldsire, Thromok the Insatiable
- Fix: CR 702.82c: DevourEffect gains serde-defaulted quality: Option<ObjectFilter> (sacrifice candidates) and multiplier_is_devoured_count (Thromok: count^2 counters). New static registry rule parse_devour_quality_line (head 'devour') lowers 'Devour <quality> N' / 'Devour X, where X is the number of creatures devoured this way' to an as-enters DevourEffect program with the Devour presentation label; plain 'Devour N' keyword path untouched.
- Files: crates/ironsmith-compiler-grammar/src/keyword_static/devour_quality.rs, crates/ironsmith-compiler-grammar/src/keyword_static/mod.rs, crates/ironsmith-core/src/effect.rs, crates/ironsmith-engine/src/effects/composition/mechanic_actions.rs, crates/ironsmith-text/src/compiled_text/render_effects/effect_impl/late.rs, crates/ironsmith-text/src/compiled_text/render_effects/single_effects_late.rs
- Test: crates/ironsmith-compiler-runtime/tests/devour_quality_variants.rs::devour_artifact_sacrifices_only_artifacts_and_counts_them
### each-player-additional-land-plays (3): Ghirapur Orrery, Rites of Flourishing, Storm Cauldron
- Fix: New registry rule parse_each_player_additional_land_play_line (head each/each player) lowers to RuleRestriction AdditionalLandPlays(PlayerFilter::Any, n); engine restriction refresh already raises every matching player's land_plays_per_turn (CR 305.2). Other lines already compile per prebuilt probe.
- Files: crates/ironsmith-compiler-grammar/src/grammar/static_keyword_facts/late.rs, crates/ironsmith-compiler-grammar/src/keyword_static/each_player_land_plays.rs, crates/ironsmith-compiler-grammar/src/keyword_static/mod.rs
- Test: crates/ironsmith-compiler-runtime/tests/each_player_additional_land_plays.rs::each_player_land_allowance_raises_every_players_land_plays
### every-subtype-family-in-addition (1): Omo, Queen of Vesuva
- Fix: 'is every <family> type' (add_all_subtypes_of_family, additive) did not accept the explicit 'in addition to its/their other types' tail; the family fact now consumes it (same additive meaning). Sibling creature-type line already compiles (probe).
- Files: crates/ironsmith-compiler-grammar/src/grammar/anthem_grants/static_grant_facts.rs
- Test: crates/ironsmith-compiler-runtime/tests/every_land_type_additions.rs::omo_adds_every_land_type_and_every_creature_type
### filtered-lure-requirement (2): Talruum Piper, Marble Priest
- Fix: Only the unfiltered 'All creatures able to block this creature do so' was supported. New registry rule (head 'all') parses 'All <blocker filter> able to block this creature do so' into Restriction::MustBlockSpecificAttacker(filter+Creature, source) (CR 509.1c); engine requirement maximisation already generic.
- Files: crates/ironsmith-compiler-grammar/src/keyword_static/filtered_lure.rs, crates/ironsmith-compiler-grammar/src/keyword_static/mod.rs
- Test: crates/ironsmith-compiler-runtime/tests/filtered_lure_requirements.rs::only_matching_blockers_are_required_to_block
### granted-casting-keyword (5): Hunting Velociraptor, Lorehold, the Historian, Niv-Mizzet, Supreme, Tannuk, Steadfast Second, Ezio Auditore da Firenze
- Fix: Shared casting-keyword grant path: '<subject> have|has <warp|prowl|freerunning|miracle> <cost>' grants the typed alternative cast in the hand (spell subjects 'X spells you cast' map to those cards in hand; no new zone permission), 'has jump-start' grants JumpStart in the graveyard. Engine resolves granted casts through resolve_play_from_alternative_method, so method-keyed behaviour (warp exile, jump-start exile, miracle draw trigger reads granted Miracle casts, prowl/freerunning conditions) applies.
- Files: crates/ironsmith-compiler-grammar/src/keyword_static/granted_casting_keywords.rs, crates/ironsmith-compiler-grammar/src/keyword_static/mod.rs
- Test: crates/ironsmith-compiler-runtime/tests/granted_casting_keywords.rs::granted_casting_keywords_compile_to_zone_scoped_alternative_cast_grants
### granted-encore (3): Wire Surgeons, Graywater's Fixer, Sliver Gravemother
- Fix: New KeywordAction::EncoreFromSourceCost{mana_value_generic} lowered by the existing encore builder with a DynamicManaCost (the card's mana cost, or generic equal to ManaValueOf(Source)); grammar rule emits GrantKeywordAction over a graveyard-scoped card filter, and the keyword grant expands through executable_object_abilities_for_keyword_action like scavenge's graveyard grant (CR 702.141).
- Files: crates/ironsmith-compiler-grammar/src/keyword_static/granted_encore.rs, crates/ironsmith-compiler-grammar/src/keyword_static/mod.rs, crates/ironsmith-compiler-lowering/src/card_builders.rs, crates/ironsmith-compiler-lowering/src/keyword_actions.rs, crates/ironsmith-compiler-lowering/src/lowering_impl/runtime_static_ability_helpers.rs, crates/ironsmith-compiler-semantic/src/payload.rs
- Test: crates/ironsmith-compiler-runtime/tests/granted_encore.rs::granted_encore_grants_a_graveyard_activated_ability_with_a_derived_cost
### granted-provoke (1): Hunter Sliver
- Fix: Provoke parsed as an intrinsic keyword (probe) but was absent from KeywordAction::lowers_to_static_ability and executable_object_abilities_for_keyword_action, so grant lines rejected it. Added alongside exalted: the grant expands the printed provoke attack trigger onto each Sliver (CR 702.39).
- Files: crates/ironsmith-compiler-lowering/src/lowering_impl/runtime_static_ability_helpers.rs, crates/ironsmith-compiler-semantic/src/payload.rs
- Test: crates/ironsmith-compiler-runtime/tests/granted_provoke.rs::hunter_sliver_grants_provoke_to_all_slivers
### halved-player-counter-count (1): Contaminated Drink
- Fix: Two gaps (probe): the rad-counter clause rejected 'half X ..., rounded up', and the player-gets-counters force surface only accepted fixed counts, so 'you get X/half X rad counters' was misread as a persistent 'gets' anthem and dropped by statement recognition. Rad clause now builds HalfRoundedDown(X[+1]); surface accepts X / half X with a rounding tail.
- Files: crates/ironsmith-compiler-grammar/src/effect_sentences/misc_actions.rs, crates/ironsmith-compiler-grammar/src/grammar/statement_player_counters.rs
- Test: crates/ironsmith-compiler-runtime/tests/halved_rad_counters.rs::contaminated_drink_gives_half_x_rounded_up_rad_counters
### labeled-trigger-body-references (2): Viv Vision, Teen Synthezoid, Cleopatra, Exiled Pharaoh
- Fix: Trailing 'if her power is 4 or greater' failed: the source-possessive power-threshold predicate accepted 'this creature's'/'<name>'s' but not gendered possessives; Oracle uses his/her only for the named card (players are 'their'), so her/his now map to SourcePowerAtLeast.
- Files: crates/ironsmith-compiler-grammar/src/grammar/effects/zone_move_shapes/draw.rs, crates/ironsmith-compiler-grammar/src/grammar/filters/predicate_phrases.rs
- Test: crates/ironsmith-compiler-runtime/tests/labeled_trigger_body_references.rs::viv_vision_draws_only_while_her_power_is_at_least_four
### leading-condition-wrapped-static (3): Personal Sanctuary, Multiclass Baldric, Flaring Flame-Kin
- Fix: No general 'During your turn, <static>' / 'As long as <cond>, <static>' composition existed; each condition-aware static rule handled its own surface. New last-resort registry rule (heads during your / as long) splits the leading condition, requires that no other rule reads the whole line (thread-local reentrancy guard), parses the single-sentence remainder with the full static registry and wraps each result in ConditionalStaticAbility (CR 604.2/611.3a). Not build-verified: relies on remainder rules parse_prevent_all_damage_to_you_line / parse_attached_prevent_all_damage_dealt_to_attached_line and the full-party condition.
- Files: crates/ironsmith-compiler-grammar/src/keyword_static/leading_condition_wrapper.rs, crates/ironsmith-compiler-grammar/src/keyword_static/mod.rs
- Test: crates/ironsmith-compiler-runtime/tests/leading_condition_wrapped_statics.rs::personal_sanctuary_prevention_is_gated_on_your_turn
### paid-method-cost-modifier (2): Henzie "Toolbox" Torre, Warbringer
- Fix: parse_flashback_cost_modifier_line already read every AlternativeCastKind ('<kind> costs you pay cost {N} less') and the engine's spell_matches_cost_modifier_filter already requires the casting method to be that kind (casting_method_matches_alternative_kind), but the rule's head hints were derived from its name ('flashback' only). Added dash/blitz/escape/madness/miracle/suspend/foretell/jump-start heads. Also: words after less/more were silently ignored; they must now parse as a 'for each' count (scaling the amount) or the line is declined. Henzie's tail maps to Value::CommanderCastCount(You).
- Files: crates/ironsmith-compiler-grammar/src/keyword_static/costs_replacements_and_permissions.rs, crates/ironsmith-compiler-grammar/src/keyword_static/mod.rs
- Test: crates/ironsmith-compiler-runtime/tests/paid_method_cost_modifiers.rs::henzie_scales_blitz_reduction_by_commander_casts
### players-skip-untap-step (1): Sands of Time
- Fix: Handled by p03's merged parse_skip_untap_steps_line ('Each player skips their untap step.' -> PlayersSkipUntapStep static, turn_runner consults player_skips_untap_step); the upkeep untap/tap line already compiled.
- Files: 
- Test: None
### plural-hand-discard (1): Wheel and Deal
- Fix: Probe: 'Any number of target opponents each discard their hand, then draw seven cards.' compiles; only the plural 'their hands' failed. Added 'their hands' to the discard hand references (each player's own hand).
- Files: crates/ironsmith-compiler-grammar/src/grammar/effects/sacrifice_discard_shapes/discard.rs
- Test: crates/ironsmith-compiler-runtime/tests/plural_hand_discards.rs::wheel_and_deal_wheels_each_targeted_opponent_then_cantrips
### relative-clause-damage-doubler (1): Raphael, the Muscle
- Fix: Imperative 'Double all damage that <X> would deal' required an explicit source-noun shape after 'that'; when that fails it now falls back to the named-dealer filter (same as the no-'that' Mjolnir form), producing the existing multiply_damage_amount_replacement (CR 614.1a). Probe: equivalent 'If a creature you control with a counter on it would deal damage, it deals double that damage instead' already compiles.
- Files: crates/ironsmith-compiler-grammar/src/grammar/keyword_static_lines/damage_combat.rs
- Test: crates/ironsmith-compiler-runtime/tests/relative_clause_damage_doublers.rs::raphael_doubles_damage_from_countered_creatures_you_control
### retarget-new-target-restriction (1): Rebound
- Fix: StackActionAst::RetargetStackObject gained new_target_restriction (core NewTargetRestriction); a trailing 'The new target must be <player|object filter>.' sentence is split off in parse_effect_sentences_lexed and attached to the last retarget instruction (error if none); lowering applies RetargetStackObjectEffect::with_restriction, which the engine already enforces (CR 115.7). Probe: the first sentence alone already compiled.
- Files: crates/ironsmith-compiler-grammar/src/effect_sentences/dispatch_entry.rs, crates/ironsmith-compiler-grammar/src/effect_sentences/mod.rs, crates/ironsmith-compiler-grammar/src/effect_sentences/new_target_restriction.rs, crates/ironsmith-compiler-lowering/src/lowering_impl/compile_support/effect_dispatch/subject_verb_middle.rs, crates/ironsmith-compiler-semantic/src/model_impl/ast/actions.rs, crates/ironsmith-compiler-semantic/src/model_impl/ast/actions/stack.rs, crates/ironsmith-compiler-semantic/src/model_impl/ast/effects.rs
- Test: crates/ironsmith-compiler-runtime/tests/retarget_new_target_restrictions.rs::rebound_retargets_only_to_a_player
### scaled-for-each-mill (1): Urborg Lhurgoyf
- Fix: Mill's trailing 'for each X' was only accepted with a count of one ('mill a card for each'); a fixed count N>1 now becomes Value::Scaled(each, N). Kick count comes from the existing 'time it was kicked' count shape; as-enters program path already compiles plain mill (probe).
- Files: crates/ironsmith-compiler-grammar/src/grammar/effects/misc_action_shapes.rs
- Test: crates/ironsmith-compiler-runtime/tests/scaled_for_each_mill.rs::urborg_lhurgoyf_mills_three_per_kick_as_it_enters
### shared-object-verb-pair (1): Fell Beast's Shriek
- Fix: Probe: 'Each opponent chooses a creature they control. Tap the chosen creatures. Goad the chosen creatures.' compiles; 'Tap and goad ...' errored ('tap clause missing target'). parse_effect_sentences_lexed now splits an untargeted '(un)tap and goad <object>' sentence into two ordered sentences on the same object.
- Files: crates/ironsmith-compiler-grammar/src/effect_sentences/dispatch_entry.rs, crates/ironsmith-compiler-grammar/src/effect_sentences/mod.rs, crates/ironsmith-compiler-grammar/src/effect_sentences/shared_object_verb_pairs.rs
- Test: crates/ironsmith-compiler-runtime/tests/shared_object_verb_pairs.rs::fell_beasts_shriek_taps_then_goads_the_chosen_creatures
### temporary-damage-multiplier (2): Insult // Injury, Isengard Unleashed
- Fix: Reused the existing temporary damage-multiplier registration (RegisterDamageMultiplier, mode UntilEndOfTurn). Gaps: the shape required the recipient before 'this turn' (Isengard has 'this turn to ...'), and the temporary reader's recipient whitelist lacked 'no recipient' (Insult) and 'an opponent or a permanent an opponent controls'. Probe: 'Damage can't be prevented this turn.' compiles.
- Files: crates/ironsmith-compiler-grammar/src/effect_sentences/dispatch_entry/temporary_damage_multiplier.rs, crates/ironsmith-compiler-grammar/src/grammar/keyword_static_lines/damage_combat.rs
- Test: crates/ironsmith-compiler-runtime/tests/temporary_source_damage_multipliers.rs::insult_doubles_all_damage_from_your_sources_this_turn
### type-qualified-typecycling (1): Sojourner's Companion
- Fix: Keyword dispatch only recognised '<x>cycling' or 'basic landcycling' heads, so 'Artifact landcycling {2}' never reached the cycling parser (whose filter grammar already accepts prefix atoms). Dispatch now admits <card type> <x>cycling; a multi-card-type typecycling quality is conjunctive (all_card_types), CR 702.29e.
- Files: crates/ironsmith-compiler-grammar/src/activation_and_restrictions/keyword_activated_lines.rs, crates/ironsmith-compiler-grammar/src/grammar/keyword_dispatch.rs
- Test: crates/ironsmith-compiler-runtime/tests/type_qualified_typecycling.rs::artifact_landcycling_searches_for_artifact_lands_from_hand

## Blocked (grouped by missing mechanic)
- **activated-ability-target-tax** (1): Kopala, Warden of Waves — Probe: the spell half ('Spells your opponents cast that target a Merfolk you control cost {2} more') compiles; activated-ability costs keyed on chosen targets have no grammar/runtime.
- **additional-phases-after-second-main** (1): World at War — Needs an additional combat+main phase inserted after the second main phase with a linked 'at the beginning of that combat' delayed trigger; additional-phase support only covers 'after this phase'.
- **as-becomes-attached-name-and-type-choice** (1): Psychic Paper — Needs an 'as this Equipment becomes attached' replacement choosing a creature card name and a creature type.
- **as-enters-discard-keyword-counters** (1): Indominus Rex, Alpha — Needs as-enters discard of any number of creature cards with keyword-counter placement per discarded keyword.
- **as-enters-exile-x-with-fallback** (1): Frankenstein's Monster — Needs as-enters exile X creature cards with a graveyard fallback and per-card counter-kind choice.
- **as-enters-reveal-or-control-counter** (1): Dragon's Disciple — Probe: '... you may reveal a Dragon card from your hand. If you do, this creature enters with a +1/+1 counter' compiles; the disjunctive 'If you do or if you control a Dragon' result-or-state condition is unsupported.
- **as-enters-roll-twice-base-pt** (1): Vedalken Squirrel-Whacker — Needs two die results assigned to base power and base toughness.
- **as-enters-roll-x-d6** (1): Neverwinter Hydra — Probe: 'Roll two d6.' / 'Roll X d6.' are unsupported; needs a multi-die roll effect whose summed results feed 'the total of those results'.
- **as-enters-sacrifice-total-pt** (1): Dracoplasm — Needs as-enters sacrifice of any number of creatures and setting P/T to their totals.
- **attack-as-though-haste-scoped-target** (1): Frenzied Saddlebrute — needs attack requirements toward a player / play-from-exile-graveyard variants / spend-as-any-color (owned by p05). Needs a global 'can attack as though they had haste' permission restricted to attacks against your opponents/their planeswalkers (permission scoped by attack target).
- **attacking-alone-conditional-unblockable** (1): Security Bypass — Leading condition 'enchanted creature is attacking alone' with pronoun 'it' bound to the enchanted creature; condition/pronoun binding unverified.
- **auras-equipment-modified-grant** (1): Silkguard — Probe: 'Auras, Equipment, and modified creatures you control have hexproof.' and the two-item 'gain ... until end of turn' compile; only the three-item serial subject with 'gain ... until end of turn' is rejected by the effect ability-grant dispatcher. Not fixed without a build to trace the gate.
- **banding-desert-prevention** (1): Camel — Banding is not implemented; 'creatures banded with this creature' cannot be expressed.
- **behold-entry-condition** (1): Theorist's Sanctum — Probe: 'As this land enters, you may reveal a Jace card from your hand. If you don't, ...' and 'Behold a Jace.' compile; behold (choose a Jace you control OR reveal one) is not an option of the reveal-or-enters-tapped payload, and as-enters programs cannot express 'enters tapped'.
- **cast-this-spell-only-timing** (4): Berserker's Frenzy — Timing rider now parses (new ThisSpellCastTiming::BeforeBlockersAreDeclared, CR 506-509 windows); the d20 body remains unsupported.; Camouflage — Timing rider now parses (DuringYourDeclareAttackersStep); body unsupported.; Illusionist's Gambit — Timing rider now parses (DuringDeclareBlockersStepOnOpponentsTurn); body unsupported.; Siren's Call — Timing rider parses (DuringOpponentsTurnBeforeAttackersAreDeclared). Probe: the body's delayed 'destroy all non-Wall creatures that player controls that didn't attack this turn' compiles but 'that player' is not bound to 'the active player' of the first sentence (IteratedPlayer invariant), and 'Ignore this effect for each creature the player didn't control continuously since the beginning of the turn' has no grammar.
- **chosen-type-outside-battlefield** (3): Arcane Adaptation — Needs the chosen creature type applied to creature spells and owned cards in every zone (CR 205 type change outside the battlefield).; Conspiracy — Same as Arcane Adaptation (setting rather than adding the type).; Leyline of Transformation — Same as Arcane Adaptation.
- **coin-flip-entry-characteristics** (1): Molten Sentry — Needs coin-flip-dependent entry characteristics (P/T and keyword) as an entry replacement.
- **colorless-damage-sources** (1): Ghostly Flame — Needs a continuous rule making matching permanents and spells colorless sources of damage.
- **comparative-player-restrictions** (1): Ward of Bones — needs cast/player restrictions and library look/exile-until moves (owned by p10). Needs per-type 'controls more X than you' cast and play restrictions.
- **compound-your-turn-static** (1): Nahiri, Storm of Stone — 'creatures you control have first strike and equip abilities you activate cost {1} less' under 'during your turn' – compound static with an equip cost modifier.
- **conditional-anthem-otherwise** (1): Mishra's Domination — Needs 'As long as you control enchanted creature, it gets +2/+2. Otherwise, it can't block.' – a two-branch static condition.
- **conditional-linked-exile-play** (2): Evendo Brushrazer — needs attack requirements toward a player / play-from-exile-graveyard variants / spend-as-any-color (owned by p05). Double leading condition over a linked-exile play permission; the inner sacrificed-this-turn condition is unverified.; Theater of Horrors — needs attack requirements toward a player / play-from-exile-graveyard variants / spend-as-any-color (owned by p05). Needs an 'if an opponent lost life this turn' gated linked-exile play permission.
- **conditional-pay-to-cast-alternative** (1): Asmoranomardicadaistinaculdacar — 'As long as you've discarded a card this turn, you may pay {B/R} to cast this spell' needs a conditional alternative cost surface (not 'rather than'); no grammar.
- **control-scoped-goad** (1): Vislor Turlough — Needs 'goaded for as long as they control it' after donating control.
- **counter-removal-cast-cost** (1): Dawnhand Dissident — needs attack requirements toward a player / play-from-exile-graveyard variants / spend-as-any-color (owned by p05). Needs casting from linked exile by removing counters from among creatures as an additional cost.
- **damage-as-though-infect-to-you** (1): Phyrexian Unlife — Probe: no runtime for 'damage is dealt to you as though its source had infect' (CR 702.90b poison conversion for a player).
- **damage-source-history-condition** (1): Suffocation — needs cast/player restrictions and library look/exile-until moves (owned by p10). Needs a cast restriction on damage dealt to you this turn by a red instant or sorcery spell and the 'controller of the last such spell' reference.
- **deck-construction-color-circling** (1): Cryptic Spires — Un-card deck-construction colour circling not modelled.
- **die-roll-loyalty-table** (1): Comet, Stellar Pup — Comet's [0] ability rolls a d6 into a result table whose rows use loyalty-symbol shorthand ('1 or 2 — [+2], then ...' = put loyalty counters); no grammar for loyalty-shorthand die-table rows.
- **die-roll-result-adjustment-with-cost** (1): Xenosquirrels — Die-roll adjustment exists only as typed specs; 'after you roll a die, you may remove a +1/+1 counter ... if you do, increase or decrease the result by 1' needs an optional cost-gated +/-1 adjustment choice.
- **direction-attack-restriction** (2): Mystic Barrier — Seat direction ('nearest opponent in the chosen direction') not modelled.; Pramikon, Sky Rampart — Seat direction not modelled.
- **discover-difference-tokens** (1): Hit the Mother Lode — Needs the discovered card's mana value as a result value.
- **distributed-counters-delayed-removal** (1): Bounty of the Hunt — Needs distributed counters with per-counter delayed removal at cleanup.
- **domain-landwalk** (1): Magnigoth Treefolk — Needs landwalk for each basic land type among your lands.
- **draft-pregame-reveal** (3): Arcane Savant — Conspiracy draft mechanic (cards drafted that aren't in your deck).; Caller of the Untamed — Conspiracy draft mechanic.; Volatile Chimera — Conspiracy draft mechanic.
- **dual-card-name-choice** (1): Null Chamber — needs cast/player restrictions and library look/exile-until moves (owned by p10). Needs 'you and an opponent each choose a card name' and a cast prohibition for the chosen names.
- **each-player-named-choice** (1): Archangel of Strife — needs multi-choice designations / votes (owned by p09). 'As this creature enters, each player chooses war or peace' needs per-player named-option choices plus per-chooser anthems.
- **energy-paid-scaled-wheel** (1): Wheel of Potential — Needs energy-paid-this-way value and conditional play permission.
- **entry-counter-kind-choice** (1): Denry Klin, Editor in Chief — Needs a choice among counter kinds for an entry counter.
- **exchange-life-lost-this-way** (1): Mister Negative — Needs the life-exchange effect to report life lost as a result value for 'If you lost life this way, draw that many cards'.
- **exile-until-nonland-free-cast** (1): Fevered Suspicion — needs cast/player restrictions and library look/exile-until moves (owned by p10). Needs per-opponent exile-until-nonland and free casting from the revealed set.
- **exile-until-total-mana-value** (2): Dream Harvest — needs cast/player restrictions and library look/exile-until moves (owned by p10). Needs exile-until with a cumulative mana value threshold and free-cast permission.; Tasha's Hideous Laughter — needs cast/player restrictions and library look/exile-until moves (owned by p10). Needs exile-until with a cumulative mana value threshold.
- **fame-or-fortune-vote** (1): Seize the Spotlight — needs multi-choice designations / votes (owned by p09). Needs per-opponent named choice with per-choice effects.
- **gain-activated-abilities-of-target** (1): Grell Philosopher — needs attack requirements toward a player / play-from-exile-graveyard variants / spend-as-any-color (owned by p05). Needs 'each Horror you control gains all activated abilities of target artifact until end of turn' plus a scoped 'spend blue mana as though any color to activate those abilities' rider and a compound 'when this enters and at the beginning of your upkeep' trigger; no temporary ability-copy-from-target effect with linked mana-spending permission.
- **global-damage-as-though-wither** (1): Everlasting Torment — Probe: no grammar or runtime for 'All damage is dealt as though its source had wither' (global damage-result overlay, CR 702.80).
- **granted-demonstrate** (1): Silverquill Lecturer — Granted demonstrate is a cast trigger on spells; needs a granted triggered keyword on stack objects (the casting-keyword grant path covers alternative casts only).
- **granted-draw-replacement-to-commanders** (1): Scion of Halaster — needs 'would X ... instead' replacement family (owned by p06). Needs a granted quoted first-draw-each-turn replacement on commander creatures.
- **granted-madness** (1): Falkenrath Gorger — needs 'would X ... instead' replacement family (owned by p06): madness is a discard-to-exile replacement plus trigger built on the card; a grant to cards 'you own that aren't on the battlefield' needs that replacement to read granted madness.
- **granted-offspring** (1): Zinnia, Valley's Voice — Granted offspring is an optional additional cost plus a linked ETB token-copy trigger; needs the typed granted optional cost described for replicate plus a cast-paid ETB trigger grant.
- **granted-quoted-replacement** (1): Pulmonic Sliver — needs 'would X ... instead' replacement family (owned by p06). Needs a granted quoted optional self-replacement ('If this permanent would be put into a graveyard, you may put it on top of its owner's library instead').
- **granted-replicate** (3): Djinn Illuminatus — Granted replicate needs a typed granted optional cost: the engine only discovers granted optional costs per keyword (ensure_granted_conspire_optional_costs / ensure_granted_casualty_optional_costs, conspire matched by marker display text). Needs a typed 'granted optional cost' static (kind Replicate, fixed or source-mana-cost price) consulted at cast; the Replicate copy trigger in triggers/check.rs already keys on OptionalCostKind::Replicate. Not built (would add a stringly path).; Hatchery Sliver — Granted replicate needs a typed granted optional cost: the engine only discovers granted optional costs per keyword (ensure_granted_conspire_optional_costs / ensure_granted_casualty_optional_costs, conspire matched by marker display text). Needs a typed 'granted optional cost' static (kind Replicate, fixed or source-mana-cost price) consulted at cast; the Replicate copy trigger in triggers/check.rs already keys on OptionalCostKind::Replicate. Not built (would add a stringly path).; Threefold Signal — Granted replicate needs a typed granted optional cost: the engine only discovers granted optional costs per keyword (ensure_granted_conspire_optional_costs / ensure_granted_casualty_optional_costs, conspire matched by marker display text). Needs a typed 'granted optional cost' static (kind Replicate, fixed or source-mana-cost price) consulted at cast; the Replicate copy trigger in triggers/check.rs already keys on OptionalCostKind::Replicate. Not built (would add a stringly path).
- **granted-sneak** (1): Ninja Teen — Granted sneak (return-unblocked-attacker alternative cast, CR 702.190) from the graveyard plus 'You may cast creature spells from your graveyard using their sneak abilities' (a zone permission keyed on one method); sneak is a special-form keyword with no AlternativeCastingMethod, so the casting-keyword grant path cannot express it.
- **graveyard-cast-life-cost-your-turn** (1): Festival of Embers — needs attack requirements toward a player / play-from-exile-graveyard variants / spend-as-any-color (owned by p05). Needs a during-your-turn graveyard cast permission with a life additional cost for instants/sorceries.
- **hexproof-from-own-colors** (1): Tam, Mindful First-Year — Needs 'hexproof from each of its colors' (self-colour-relative protection).
- **hidden-items** (1): Goblin Game — Hidden object game not implemented.
- **life-floor-replacement** (1): Elderscale Wurm — needs 'would X ... instead' replacement family (owned by p06). Needs a damage-to-life-total floor replacement at 7 (only 'below 1' style life restrictions exist).
- **linked-exile-cast-with-flash** (1): Azula, Cunning Usurper — needs attack requirements toward a player / play-from-exile-graveyard variants / spend-as-any-color (owned by p05). Needs a during-your-turn linked-exile cast permission with flash and any-type mana spending.
- **loyalty-removal-replacement** (1): Deification — needs 'would X ... instead' replacement family (owned by p06). Needs a replacement that leaves one loyalty counter when damage would remove all (chosen planeswalker type).
- **morph-cost-modification** (1): Exiled Doomsayer — Morph costs are special-action (turn face up) costs, not spell/ability costs; no cost-modifier hook exists for them (CR 702.37).
- **next-turn-attack-requirement** (1): Taunt — needs attack requirements toward a player / play-from-exile-graveyard variants / spend-as-any-color (owned by p05). Needs 'during target player's next turn, creatures that player controls attack you if able'.
- **optional-draw-up-to-with-shortfall** (2): Temporary Truce — Needs per-player 'may draw up to two' with a life gain per card not drawn.; Truce — Same as Temporary Truce.
- **per-permanent-type-graveyard-play** (1): Muldrotha, the Gravetide — needs attack requirements toward a player / play-from-exile-graveyard variants / spend-as-any-color (owned by p05). Needs once-per-permanent-type graveyard play permissions.
- **per-player-linked-exile-this-turn** (1): Uba Mask — needs attack requirements toward a player / play-from-exile-graveyard variants / spend-as-any-color (owned by p05). Needs per-player 'cards they exiled with this this turn' play permission.
- **player-conditional-restrictions** (1): Angelic Arbiter — needs cast/player restrictions and library look/exile-until moves (owned by p10). Needs restrictions on each opponent who cast a spell this turn (player-scoped conditional restriction).
- **player-level-attack-requirement** (2): Seeker of Slaanesh — needs attack requirements toward a player / play-from-exile-graveyard variants / spend-as-any-color (owned by p05). Needs a player-scoped requirement 'must attack with at least one creature each combat if able' in the CR 508.1d requirement maximisation (current scoring is per-creature additive).; Trove of Temptation — needs attack requirements toward a player / play-from-exile-graveyard variants / spend-as-any-color (owned by p05). Same as Seeker of Slaanesh, additionally restricted to attacking you or your planeswalkers.
- **power-up-additional-activation** (1): Wonder Man, Hollywood Hero — Needs an extra activation allowance for power-up abilities.
- **prevention-by-targeting-spell** (1): Bronze Horse — Probe: 'Prevent all damage that would be dealt to this creature by spells that target it.' has no grammar (source filter 'spell that targets this object').
- **secret-choices** (1): Call to the Void — needs multi-choice designations / votes (owned by p09). Secret simultaneous choices not implemented.
- **secret-number-choice** (1): Wheel of Misfortune — needs multi-choice designations / votes (owned by p09). Secret number choice not implemented.
- **secret-opponent-choice** (2): Emissary of Grudges — Secretly choosing an opponent (hidden choice revealed later) is not implemented.; Guardian Archon — Secretly choosing an opponent (hidden choice) is not implemented.
- **share-loyalty-abilities** (1): Kasmina, Enigma Sage — needs copy activated/loyalty abilities (owned by p12). Needs granting the source's loyalty abilities to other planeswalkers.
- **shared-exile-play-permission** (2): Share the Spoils — needs attack requirements toward a player / play-from-exile-graveyard variants / spend-as-any-color (owned by p05). Needs a per-player shared exile pool play permission with linked replenish trigger.; Shared Fate — needs attack requirements toward a player / play-from-exile-graveyard variants / spend-as-any-color (owned by p05). Needs per-player linked exile look/play permissions.
- **source-choice-damage-redirection** (2): Kor Chant — needs 'would X ... instead' replacement family (owned by p06). Needs a one-shot redirection shield from a chosen source to a second target creature (CR 615 redirection keyed on a source chosen on resolution); not implemented.; Kor Dirge — needs 'would X ... instead' replacement family (owned by p06). Same as Kor Chant: redirect damage from a chosen source dealt to target creature to another target creature this turn.
- **station-using-toughness** (1): Tapestry Warden — Needs station using toughness rather than power.
- **step-scoped-flash-permission** (1): Final-Word Phantom — Probe: 'You may cast spells as though they had flash.' compiles; needs a step-scoped condition (each opponent's end step) for the flash permission; ActivationTiming/PredicateAst have no end-step window.
- **stickers** (2): Clandestine Chameleon — Ability stickers are not implemented.; Wicker Picker — Sticker kicker / {TK} tickets not implemented.
- **target-permanent-damage-doubling** (1): Overblaze — needs 'would X ... instead' replacement family (owned by p06). Needs a one-shot doubling replacement for damage dealt by a target permanent this turn.
- **target-player-subject-attack-requirement** (1): Imaginary Threats — needs attack requirements toward a player / play-from-exile-graveyard variants / spend-as-any-color (owned by p05). 'Creatures target opponent controls attack this turn if able' (must-attack filter excludes target subjects) plus 'that player's next untap step' antecedent binding; not verified.
- **token-creation-copy-replacement** (1): Mirrormind Crown — needs 'would X ... instead' replacement family (owned by p06). Needs a first-time-each-turn token-creation replacement that creates copies of the equipped creature instead.
- **two-chosen-basic-land-types** (1): Illusionary Terrain — Needs two ordered chosen basic land types and a type-changing effect between them.
- **untap-step-type-choice** (1): Storage Matrix — Needs each player choosing a permanent type during their untap step and a type-limited untap restriction.
- **variable-additional-cost-entry-counters** (1): Chorus of the Conclave — Needs an optional pay-any-amount additional cost on other creature spells with a linked entry-counter replacement.
