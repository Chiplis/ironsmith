use crate::ability::AbilityKind;
use crate::cards::builders::{
    CardDefinitionBuilder, CarryContext, ChooseOneModeAst, EffectAst, PlayerAst, PredicateAst,
    SubjectVerbActionAst, SubjectVerbEffectAst, SubjectVerbSubjectAst, TargetAst,
};
use crate::effect::Value;
use crate::ids::CardId;
use crate::types::{CardType, Subtype};
use crate::zone::Zone;

use super::super::super::lexer::lex_line;
use super::super::parse_effect_sentence_inner_lexed;
use super::{
    maybe_apply_carried_player_with_clause_lexed, parse_effect_chain_lexed,
    parse_effect_chain_with_subject_verb_primitives_lexed,
    parse_effect_clause_with_trailing_if_lexed, parse_effect_sentence_lexed,
    parse_leading_player_may_lexed, preserve_coordinated_effect_chain_surface,
    starts_like_create_fragment_lexed,
};
use crate::runtime_backend::front_end::shared::util::with_source_reference_context;

#[test]
fn leading_duration_scaled_target_then_pronoun_grant_keeps_both_actions() {
    let tokens = lex_line(
        "Until end of turn, double target creature's power and it gains first strike.",
        0,
    )
    .expect("scaled target/grant chain should lex");
    let effects =
        parse_effect_sentence_lexed(&tokens).expect("scaled target/grant chain should parse");
    let [
        EffectAst::Coordinated {
            effects: coordinated,
            leading_duration: true,
            ..
        },
    ] = effects.as_slice()
    else {
        panic!("expected one leading-duration coordinated chain, got {effects:#?}");
    };
    assert!(
        matches!(
            coordinated.as_slice(),
            [
                EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action: SubjectVerbActionAst::Pump { .. },
                    ..
                }),
                EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action: SubjectVerbActionAst::GrantAbilitiesToTarget { .. },
                    ..
                }),
            ]
        ),
        "{coordinated:#?}"
    );
}

#[test]
fn generic_comma_then_chain_keeps_a_distinct_typed_boundary() {
    let comma_then = lex_line(
        "Target player draws cards equal to the number of cards in their hand, then discards that many cards.",
        0,
    )
    .expect("comma-then chain should lex");
    let effects =
        parse_effect_chain_lexed(&comma_then).expect("comma-then chain should parse completely");
    let [EffectAst::CommaThen { effects: nested }] = effects.as_slice() else {
        panic!("expected typed comma-then wrapper, got {effects:#?}");
    };
    let debug = format!("{nested:#?}");
    assert!(debug.contains("Draw"), "{debug}");
    assert!(debug.contains("Discard"), "{debug}");
    assert!(
        matches!(
            preserve_coordinated_effect_chain_surface(&comma_then, nested.clone()).as_slice(),
            [EffectAst::CommaThen { .. }]
        ),
        "whole-line surface preservation should retain the same typed comma-then boundary"
    );

    let coordinated = lex_line(
        "Target player draws cards equal to the number of cards in their hand and discards that many cards.",
        0,
    )
    .expect("coordinated chain should lex");
    let effects =
        parse_effect_chain_lexed(&coordinated).expect("coordinated chain should still parse");
    assert!(
        effects
            .iter()
            .all(|effect| !matches!(effect, EffectAst::CommaThen { .. })),
        "{effects:#?}"
    );
}

#[test]
fn sentence_dispatch_keeps_create_then_copy_as_two_typed_actions() {
    let tokens = lex_line(
        "Create a 1/1 red Soldier creature token with haste, then copy that spell.",
        0,
    )
    .expect("create/copy chain should lex");
    let effects =
        parse_effect_sentence_lexed(&tokens).expect("sentence dispatch should keep both actions");
    let [EffectAst::CommaThen { effects: nested }] = effects.as_slice() else {
        panic!("expected typed comma-then create/copy chain, got {effects:#?}");
    };
    assert!(
        matches!(
            nested.as_slice(),
            [
                EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action: SubjectVerbActionAst::CreateTokenWithMods { .. },
                    ..
                }),
                EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action: SubjectVerbActionAst::CopySpell { .. },
                    ..
                }),
            ]
        ),
        "both authored actions must survive sentence dispatch: {nested:#?}"
    );
}

#[test]
fn specialist_pronoun_chain_keeps_its_authored_comma_then_surface() {
    let tokens =
        lex_line("It explores, then it explores again.", 0).expect("explore chain should lex");
    let effects =
        parse_effect_sentence_lexed(&tokens).expect("explore chain should parse completely");
    let [EffectAst::CommaThen { effects: nested }] = effects.as_slice() else {
        panic!("expected a typed comma-then explore chain, got {effects:#?}");
    };
    assert_eq!(nested.len(), 2, "{nested:#?}");

    let coordinated =
        lex_line("It explores and it explores again.", 0).expect("coordinated chain should lex");
    let effects = parse_effect_sentence_lexed(&coordinated)
        .expect("coordinated explore chain should parse completely");
    assert!(
        effects
            .iter()
            .all(|effect| !matches!(effect, EffectAst::CommaThen { .. })),
        "{effects:#?}"
    );
}

#[test]
fn leading_may_chain_keeps_nested_comma_then_surface() {
    let tokens = lex_line(
        "You may exile this creature, then return it to the battlefield transformed under its owner's control.",
        0,
    )
    .expect("leading-may chain should lex");
    let effects =
        parse_effect_sentence_lexed(&tokens).expect("leading-may chain should parse completely");
    let debug = format!("{effects:#?}");
    assert!(
        debug.contains("CommaThen"),
        "the authored boundary must survive inside the optional program: {debug}"
    );
}

#[test]
fn triggered_optional_chain_resurfaces_nested_comma_then_after_trigger_parsing() {
    let definition = CardDefinitionBuilder::new(CardId::new(), "Ajani, Nacatl Pariah")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Whenever one or more other Cats you control die, you may exile Ajani, then return him to the battlefield transformed under his owner's control.",
        )
        .expect("triggered optional sequence should compile");
    let debug = format!("{definition:#?}");
    assert!(
        debug.contains("surface: CommaThen"),
        "trigger parsing must retain nested optional sequence provenance: {debug}"
    );
}

#[test]
fn conditional_instead_chain_keeps_every_replacement_action() {
    let branch_tokens = lex_line("exile it, then return that card to its owner's hand.", 0)
        .expect("replacement action chain should lex");
    let branch_effects =
        parse_effect_chain_lexed(&branch_tokens).expect("replacement action chain should parse");
    assert!(
        matches!(
            branch_effects.as_slice(),
            [EffectAst::CommaThen { effects }] if effects.len() == 2
        ),
        "the standalone replacement chain must retain both actions: {branch_effects:#?}"
    );

    let tokens = lex_line(
        "If it has unearth, instead exile it, then return that card to its owner's hand.",
        0,
    )
    .expect("conditional replacement sequence should lex");
    let effects = parse_effect_sentence_lexed(&tokens)
        .expect("conditional replacement sequence should parse");
    assert!(
        matches!(
            effects.as_slice(),
            [EffectAst::Conditional {
                if_true,
                ..
            }] if matches!(
                if_true.as_slice(),
                [EffectAst::CommaThen { effects }] if matches!(
                    effects.as_slice(),
                    [
                        EffectAst::SubjectVerb(SubjectVerbEffectAst {
                            action: SubjectVerbActionAst::Exile { .. },
                            ..
                        }),
                        EffectAst::SubjectVerb(SubjectVerbEffectAst {
                            action: SubjectVerbActionAst::ReturnToHand { .. },
                            ..
                        }),
                    ]
                )
            )
        ),
        "the replacement branch must retain both ordered actions: {effects:#?}"
    );
}

#[test]
fn activated_conditional_instead_chain_lowers_every_replacement_action() {
    let definition = CardDefinitionBuilder::new(CardId::new(), "Meticulous Excavation")
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "{2}{W}: Return target permanent you control to its owner's hand. If it has unearth, instead exile it, then return that card to its owner's hand. Activate only during your turn.",
        )
        .expect("conditional activated replacement should compile");
    let debug = format!("{definition:#?}");
    assert!(
        debug.contains("ability_markers: [\n")
            && debug.contains("\"unearth\"")
            && debug.contains("MoveToZoneEffect")
            && debug.contains("zone: Exile")
            && debug.contains("ReturnToHandEffect")
            && debug.contains("condition: TaggedObjectMatches")
            && !debug.contains("condition: SourceMatches"),
        "lowering must retain both actions from the replacement branch: {debug}"
    );
}

#[test]
fn comma_then_optional_energy_payment_keeps_gain_and_payment() {
    let gain = lex_line("you get {e}{e}{e}{e}.", 0).expect("energy gain should lex");
    parse_effect_sentence_lexed(&gain).expect("fixed energy gain should parse independently");
    let payment = lex_line(
        "you may pay an amount of {e} equal to that permanent's mana value.",
        0,
    )
    .expect("optional energy payment should lex");
    parse_effect_sentence_lexed(&payment)
        .expect("optional dynamic energy payment should parse independently");

    let tokens = lex_line(
        "you get {e}{e}{e}{e}, then you may pay an amount of {e} equal to that permanent's mana value.",
        0,
    )
    .expect("energy chain should lex");
    let effects =
        parse_effect_sentence_lexed(&tokens).expect("energy gain/payment chain should parse");
    let debug = format!("{effects:#?}");
    assert!(
        debug.contains("EnergyCounters") && debug.contains("PayEnergy"),
        "the fixed energy gain and optional dynamic payment must both survive: {debug}"
    );
}

#[test]
fn draw_then_optional_free_cast_keeps_both_actions() {
    let tokens = lex_line(
        "draw a card, then you may cast a spell from your hand with mana value less than or equal to that damage without paying its mana cost.",
        0,
    )
    .expect("draw/free-cast chain should lex");
    let effects = parse_effect_sentence_lexed(&tokens).expect("draw/free-cast chain should parse");
    let debug = format!("{effects:#?}");

    assert!(debug.contains("Draw"), "{debug}");
    assert!(
        debug.contains("MayCastMatchingSpellWithoutPayingManaCost")
            || debug.contains("GrantPlayTagged"),
        "the optional free-cast action must survive after the draw: {debug}"
    );
}

#[test]
fn optional_payment_any_number_of_times_lowers_to_counted_repeat() {
    let tokens = lex_line("You may pay {1}{U} any number of times.", 0)
        .expect("repeated payment should lex");
    let effects =
        parse_effect_sentence_lexed(&tokens).expect("repeated payment should parse completely");
    let debug = format!("{effects:#?}");

    assert!(debug.contains("RepeatProcess"), "{debug}");
    assert!(debug.contains("PayMana"), "{debug}");
    assert!(debug.contains("continue_effect_index: 0"), "{debug}");
}

#[test]
fn paid_cost_count_reflexive_keeps_counter_and_dynamic_targets() {
    let tokens = lex_line(
        "When you pay this cost one or more times, put that many +1/+1 counters on this creature, then up to that many other target artifacts, creatures, and/or enchantments phase out.",
        0,
    )
    .expect("paid-cost reflexive should lex");
    let effects =
        parse_effect_sentence_lexed(&tokens).expect("paid-cost reflexive should parse completely");
    let debug = format!("{effects:#?}");

    assert!(
        matches!(
            effects.as_slice(),
            [EffectAst::WhenResult {
                predicate: crate::cards::builders::IfResultPredicate::Value(
                    crate::effect::Comparison::GreaterThan(0)
                ),
                ..
            }]
        ),
        "{debug}"
    );
    assert!(debug.contains("PutCounters"), "{debug}");
    assert!(debug.contains("PhaseOut"), "{debug}");
    assert!(debug.contains("WithCountValue"), "{debug}");
    assert_eq!(
        debug.matches("PendingEffectMetric").count(),
        2,
        "both `that many` values must bind to the repeated-payment outcome: {debug}"
    );
}

#[test]
fn created_tokens_then_source_deals_where_x_keeps_both_actions() {
    let stripped_tokens = lex_line(
        "create three 1/1 red Hamster creature tokens, then it deals X damage to any target.",
        0,
    )
    .expect("token/damage chain without binding should lex");
    let stripped_effects = parse_effect_chain_lexed(&stripped_tokens)
        .expect("token/damage chain without binding should parse");
    let stripped_debug = format!("{stripped_effects:#?}");
    assert!(stripped_debug.contains("CreateToken"), "{stripped_debug}");
    assert!(stripped_debug.contains("DealDamage"), "{stripped_debug}");

    let tokens = lex_line(
        "create three 1/1 red Hamster creature tokens, then it deals X damage to any target, where X is the number of Hamsters you control.",
        0,
    )
    .expect("token/damage chain should lex");
    let effects = parse_effect_sentence_lexed(&tokens)
        .expect("token/damage chain should parse as one ordered program");
    let debug = format!("{effects:#?}");

    assert!(debug.contains("CreateToken"), "{debug}");
    assert!(debug.contains("DealDamage"), "{debug}");
    assert!(
        debug.contains("subtypes: [\n") && debug.contains("Hamster"),
        "the damage amount must retain its Hamster count basis: {debug}"
    );
    assert!(
        matches!(
            effects.as_slice(),
            [EffectAst::CommaThen { effects }] if matches!(
                effects.as_slice(),
                [
                    EffectAst::SubjectVerb(SubjectVerbEffectAst {
                        action: SubjectVerbActionAst::CreateTokenWithMods { .. },
                        ..
                    }),
                    EffectAst::SubjectVerb(SubjectVerbEffectAst {
                        action: SubjectVerbActionAst::DealDamageEqualToPower {
                            source: crate::cards::builders::TargetAst::Source(_),
                            ..
                        },
                        ..
                    }),
                ]
            )
        ),
        "the singular damage pronoun after a plural token result must bind to the source: {debug}"
    );
}

#[test]
fn comma_then_choose_player_keeps_both_ordered_actions() {
    let tokens = lex_line(
        "Return target card from your graveyard to your hand, then choose an opponent.",
        0,
    )
    .expect("return/choice chain should lex");
    let effects = parse_effect_sentence_lexed(&tokens).expect("return/choice chain should parse");
    let debug = format!("{effects:#?}");
    assert!(
        debug.contains("Return") && debug.contains("ChoosePlayer"),
        "the ordered opponent choice must not be swallowed by the return parser: {debug}"
    );
}

#[test]
fn unique_nested_program_keeps_its_authored_comma_then_surface() {
    let tokens = lex_line("You may solve this case, then solve this case.", 0)
        .expect("nested comma-then surface should lex");
    let effects = preserve_coordinated_effect_chain_surface(
        &tokens,
        vec![EffectAst::May {
            effects: vec![EffectAst::SolveCase, EffectAst::SolveCase],
        }],
    );
    let [EffectAst::May { effects: nested }] = effects.as_slice() else {
        panic!("expected one optional nested program, got {effects:#?}");
    };
    assert!(
        matches!(
            nested.as_slice(),
            [EffectAst::CommaThen { effects }] if effects.len() == 2
        ),
        "the unique nested program should own the typed comma-then surface: {nested:#?}"
    );
}

#[test]
fn ambiguous_conditional_branches_do_not_guess_a_comma_then_owner() {
    let tokens = lex_line("If it is night, solve this case, then solve this case.", 0)
        .expect("ambiguous conditional surface should lex");
    let effects = preserve_coordinated_effect_chain_surface(
        &tokens,
        vec![EffectAst::Conditional {
            predicate: PredicateAst::ItIsNight,
            if_true: vec![EffectAst::SolveCase, EffectAst::SolveCase],
            if_false: vec![EffectAst::SolveCase, EffectAst::SolveCase],
        }],
    );
    let debug = format!("{effects:#?}");
    assert!(
        !debug.contains("CommaThen"),
        "two eligible branches are ambiguous and must remain unmarked: {debug}"
    );
}

#[test]
fn multi_mode_choice_does_not_guess_a_comma_then_owner() {
    let tokens = lex_line("Choose one, solve this case, then solve this case.", 0)
        .expect("multi-mode surface should lex");
    let effects = preserve_coordinated_effect_chain_surface(
        &tokens,
        vec![EffectAst::ChooseOneOf {
            modes: vec![
                ChooseOneModeAst {
                    description: "First".to_string(),
                    effects: vec![EffectAst::SolveCase, EffectAst::SolveCase],
                },
                ChooseOneModeAst {
                    description: "Second".to_string(),
                    effects: vec![EffectAst::SolveCase, EffectAst::SolveCase],
                },
            ],
        }],
    );
    let debug = format!("{effects:#?}");
    assert!(
        !debug.contains("CommaThen"),
        "multiple eligible modes are ambiguous and must remain unmarked: {debug}"
    );
}

#[test]
fn for_each_opponent_imperative_create_keeps_controller_as_actor() {
    let tokens = lex_line(
        "For each opponent, create two 3/3 blue and red Elemental creature tokens with flying.",
        0,
    )
    .expect("quantified imperative create should lex");
    let effects = parse_effect_sentence_lexed(&tokens)
        .expect("quantified imperative create should parse completely");
    let [EffectAst::ForEachOpponent { effects: nested }] = effects.as_slice() else {
        panic!("expected one quantified opponent loop, got {effects:#?}");
    };
    assert!(
        matches!(
            nested.as_slice(),
            [EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::CreateTokenWithMods {
                    player: PlayerAst::You,
                    ..
                },
                ..
            })]
        ),
        "the loop count must not make each opponent the token controller: {nested:#?}"
    );

    let explicit = lex_line(
        "For each opponent, that player creates a 1/1 red Goblin creature token.",
        0,
    )
    .expect("explicit participant create should lex");
    let effects = parse_effect_sentence_lexed(&explicit)
        .expect("explicit participant create should parse completely");
    let [EffectAst::ForEachOpponent { effects: nested }] = effects.as_slice() else {
        panic!("expected one explicit opponent loop, got {effects:#?}");
    };
    assert!(
        matches!(
            nested.as_slice(),
            [EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::CreateTokenWithMods {
                    player: PlayerAst::That,
                    ..
                },
                ..
            })]
        ),
        "an explicitly authored participant actor must remain iterated: {nested:#?}"
    );
}

#[test]
fn repeated_explicit_may_clauses_keep_independent_choice_scopes() {
    let tokens = lex_line(
        "You may have Allies you control gain lifelink until end of turn, and you may put a +1/+1 counter on this creature.",
        0,
    )
    .expect("independent optional clauses should lex");
    let effects =
        parse_effect_chain_lexed(&tokens).expect("independent optional clauses should parse");
    let [
        EffectAst::Coordinated {
            effects,
            leading_duration: false,
            result_conjunction: false,
        },
    ] = effects.as_slice()
    else {
        panic!("expected one typed coordinated choice bundle: {effects:#?}");
    };
    let [first, second] = effects.as_slice() else {
        panic!("expected two independent choices: {effects:#?}");
    };
    assert!(
        matches!(
            first,
            EffectAst::MayByPlayer {
                player: PlayerAst::You,
                ..
            }
        ),
        "{first:#?}"
    );
    assert!(
        matches!(
            second,
            EffectAst::MayByPlayer {
                player: PlayerAst::You,
                ..
            }
        ),
        "{second:#?}"
    );
    let debug = format!("{effects:#?}");
    assert!(debug.contains("GrantAbilitiesAll"), "{debug}");
    assert!(debug.contains("PutCounters"), "{debug}");
}

#[test]
fn convert_then_adapt_keeps_both_executable_keyword_actions() {
    let tokens = lex_line("Convert this creature, then adapt 3.", 0)
        .expect("convert/adapt chain should lex");
    let effects = parse_effect_chain_lexed(&tokens).expect("convert/adapt chain should parse");
    let debug = format!("{effects:#?}");

    assert!(debug.contains("Convert"), "{debug}");
    assert!(debug.contains("Adapt"), "{debug}");
    assert!(!debug.contains("KeywordFallback"), "{debug}");
}

#[test]
fn result_prefixed_inline_consult_keeps_its_complete_disposition() {
    let tokens = lex_line(
        "If you do, reveal cards from the top of your library until you reveal a nonlegendary creature card with lesser mana value, put it onto the battlefield, then put the rest on the bottom of your library in a random order.",
        0,
    )
    .expect("result-prefixed consult should lex");
    let effects = parse_effect_sentence_lexed(&tokens).expect("complete consult should parse");
    let debug = format!("{effects:#?}");

    assert!(debug.contains("IfResult"), "{debug}");
    assert!(debug.contains("ConsultTopOfLibrary"), "{debug}");
    assert!(debug.contains("MoveToZone"), "{debug}");
    assert!(
        debug.contains("PutTaggedRemainderOnBottomOfLibrary"),
        "{debug}"
    );
}

#[test]
fn duration_scoped_combat_damage_trigger_reaches_the_trigger_parser_intact() {
    for (text, expected_effect, expected_either_surface) in [
        (
            "Until your next turn, whenever either of those creatures deals combat damage, you draw a card.",
            "Draw",
            true,
        ),
        (
            "Until your next turn, whenever a creature deals combat damage to this, destroy that creature.",
            "Destroy",
            false,
        ),
    ] {
        let tokens = lex_line(text, 0).expect("duration-scoped damage trigger should lex");
        let effects = parse_effect_chain_lexed(&tokens)
            .expect("the duration prefix must not expose combat damage as an effect clause");
        let debug = format!("{effects:#?}");
        assert!(debug.contains("DelayedTriggerForDuration"), "{debug}");
        assert!(debug.contains("DealsCombatDamage"), "{debug}");
        assert!(debug.contains("YourNextTurn"), "{debug}");
        assert!(debug.contains(expected_effect), "{debug}");
        assert_eq!(
            debug.contains("either_of_watched_objects: true"),
            expected_either_surface,
            "{debug}"
        );
    }
}

#[test]
fn duration_scoped_becomes_tapped_trigger_reaches_the_trigger_parser_intact() {
    let tokens = lex_line(
        "Until your next turn, whenever a creature becomes tapped, destroy it.",
        0,
    )
    .expect("duration-scoped becomes-tapped trigger should lex");
    let effects = parse_effect_chain_lexed(&tokens)
        .expect("the duration prefix must not expose 'becomes tapped' as an effect clause");
    let debug = format!("{effects:#?}");

    assert!(debug.contains("DelayedTriggerForDuration"), "{debug}");
    assert!(debug.contains("PermanentBecomesTapped"), "{debug}");
    assert!(debug.contains("YourNextTurn"), "{debug}");
    assert!(debug.contains("Destroy"), "{debug}");
}

#[test]
fn duration_scoped_delayed_triggers_compile_for_the_three_real_cards() {
    let dont_move = CardDefinitionBuilder::new(CardId::new(), "Don't Move")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Destroy all tapped creatures. Until your next turn, whenever a creature becomes tapped, destroy it.",
        )
        .expect("Don't Move should compile through the delayed-trigger runtime effect");
    let dont_move_debug = format!("{dont_move:#?}");
    assert!(
        dont_move_debug.contains("ScheduleDelayedTriggerEffect")
            && dont_move_debug.contains("PermanentBecomesTapped")
            && dont_move_debug.contains("UntilControllerNextTurn")
            && dont_move_debug.contains("DestroyEffect"),
        "{dont_move_debug}"
    );

    let tamiyo = CardDefinitionBuilder::new(CardId::new(), "Tamiyo, Field Researcher")
        .card_types(vec![CardType::Planeswalker])
        .parse_text(
            "+1: Choose up to two target creatures. Until your next turn, whenever either of those creatures deals combat damage, you draw a card.\n−2: Tap up to two target nonland permanents. They don't untap during their controller's next untap step.\n−7: Draw three cards. You get an emblem with \"You may cast spells from your hand without paying their mana costs.\"",
        )
        .expect("Tamiyo should compile through the delayed-trigger runtime effect");
    let tamiyo_debug = format!("{tamiyo:#?}");
    assert!(
        tamiyo_debug.contains("ScheduleDelayedTriggerEffect")
            && tamiyo_debug.contains("DealsCombatDamage")
            && tamiyo_debug.contains("UntilControllerNextTurn")
            && tamiyo_debug.contains("either_of_watched_objects: true")
            && tamiyo_debug.contains("watch_all_object_targets: true")
            && tamiyo_debug.contains("DrawCardsEffect"),
        "{tamiyo_debug}"
    );

    let vraska = CardDefinitionBuilder::new(CardId::new(), "Vraska the Unseen")
        .card_types(vec![CardType::Planeswalker])
        .parse_text(
            "+1: Until your next turn, whenever a creature deals combat damage to Vraska, destroy that creature.\n−3: Destroy target nonland permanent.\n−7: Create three 1/1 black Assassin creature tokens with \"Whenever this token deals combat damage to a player, that player loses the game.\"",
        )
        .expect("Vraska should compile through the delayed-trigger runtime effect");
    let vraska_debug = format!("{vraska:#?}");
    assert!(
        vraska_debug.contains("ScheduleDelayedTriggerEffect")
            && vraska_debug.contains("DealsCombatDamageTo")
            && vraska_debug.contains("UntilControllerNextTurn")
            && vraska_debug.contains("watch_ability_source: true")
            && vraska_debug.contains("DestroyEffect"),
        "{vraska_debug}"
    );
}

#[test]
fn effect_sentence_inner_preserves_coordinated_if_result_body() {
    let tokens = lex_line(
        "If you do, put a +1/+1 counter on it and tap up to one target creature defending player controls.",
        0,
    )
    .expect("Aetherstorm Roc result clause should lex");

    let effects = parse_effect_sentence_inner_lexed(&tokens)
        .expect("Aetherstorm Roc result clause should parse through sentence dispatch");
    let [
        EffectAst::IfResult {
            predicate: crate::cards::builders::IfResultPredicate::Did,
            effects: conditional_effects,
        },
    ] = effects.as_slice()
    else {
        panic!("expected one if-result wrapper, got {effects:#?}");
    };
    let [
        EffectAst::Coordinated {
            effects: coordinated_effects,
            leading_duration: false,
            result_conjunction: true,
        },
    ] = conditional_effects.as_slice()
    else {
        panic!("expected one coordinated result body, got {conditional_effects:#?}");
    };
    assert_eq!(coordinated_effects.len(), 2, "{coordinated_effects:#?}");
}

#[test]
fn hollow_specter_dependent_result_arms_stay_flat() {
    let tokens = lex_line(
        "If you do, that player reveals X cards from their hand and you choose one of them.",
        0,
    )
    .expect("Hollow Specter result clause should lex");

    let effects = parse_effect_sentence_inner_lexed(&tokens)
        .expect("Hollow Specter result clause should parse through sentence dispatch");
    let [
        EffectAst::IfResult {
            predicate: crate::cards::builders::IfResultPredicate::Did,
            effects: conditional_effects,
        },
    ] = effects.as_slice()
    else {
        panic!("expected one if-result wrapper, got {effects:#?}");
    };
    assert_eq!(conditional_effects.len(), 2, "{conditional_effects:#?}");
    assert!(
        conditional_effects
            .iter()
            .all(|effect| !matches!(effect, EffectAst::Coordinated { .. })),
        "the revealed-card dependency must remain visible to the flat specialist: {conditional_effects:#?}"
    );

    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action: SubjectVerbActionAst::RevealCardsFromHand { tag, .. },
        ..
    }) = &conditional_effects[0]
    else {
        panic!("expected the first arm to reveal tagged hand cards: {conditional_effects:#?}");
    };
    let EffectAst::ChooseObjects { filter, .. } = &conditional_effects[1] else {
        panic!("expected the second arm to choose from those cards: {conditional_effects:#?}");
    };
    assert!(
        filter
            .tagged_constraints
            .iter()
            .any(|constraint| constraint.tag.as_str() == tag.as_str()),
        "the second arm must consume the first arm's reveal tag: {conditional_effects:#?}"
    );
}

#[test]
fn moku_safe_existing_coordination_becomes_a_result_conjunction() {
    let tokens = lex_line(
        "If you do, this gets +2/+1 and creatures you control gain haste until end of turn.",
        0,
    )
    .expect("Moku result clause should lex");

    let effects = parse_effect_sentence_inner_lexed(&tokens)
        .expect("Moku result clause should parse through sentence dispatch");
    let [
        EffectAst::IfResult {
            predicate: crate::cards::builders::IfResultPredicate::Did,
            effects: conditional_effects,
        },
    ] = effects.as_slice()
    else {
        panic!("expected one if-result wrapper, got {effects:#?}");
    };
    let [
        EffectAst::Coordinated {
            effects: coordinated_effects,
            leading_duration: false,
            result_conjunction: true,
        },
    ] = conditional_effects.as_slice()
    else {
        panic!(
            "expected Moku's safe authored 'and' body to be a result conjunction: {conditional_effects:#?}"
        );
    };
    assert_eq!(coordinated_effects.len(), 2, "{coordinated_effects:#?}");
    let debug = format!("{coordinated_effects:#?}");
    assert!(debug.contains("Pump"), "{debug}");
    assert!(debug.contains("GrantAbilitiesAll"), "{debug}");
}

#[test]
fn ulalek_then_body_stays_an_ordinary_coordinated_specialist() {
    let tokens = lex_line(
        "If you do, copy all spells you control, then copy all other activated and triggered abilities you control.",
        0,
    )
    .expect("Ulalek result clause should lex");

    let effects = parse_effect_sentence_inner_lexed(&tokens)
        .expect("Ulalek result clause should parse through sentence dispatch");
    let [
        EffectAst::IfResult {
            predicate: crate::cards::builders::IfResultPredicate::Did,
            effects: conditional_effects,
        },
    ] = effects.as_slice()
    else {
        panic!("expected one if-result wrapper, got {effects:#?}");
    };
    let [
        EffectAst::Coordinated {
            effects: coordinated,
            result_conjunction: false,
            ..
        },
    ] = conditional_effects.as_slice()
    else {
        panic!(
            "expected Ulalek's specialist 'then' body to remain ordinary: {conditional_effects:#?}"
        );
    };
    let [
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::CopySpell {
                    target: TargetAst::Object(spells, ..),
                    ..
                },
            ..
        }),
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::CopySpell {
                    target: TargetAst::Object(abilities, ..),
                    ..
                },
            ..
        }),
    ] = coordinated.as_slice()
    else {
        panic!("expected two typed copy-all actions: {coordinated:#?}");
    };
    assert_eq!(
        spells.stack_kind,
        Some(crate::filter::StackObjectKind::Spell)
    );
    assert_eq!(
        abilities.stack_kind,
        Some(crate::filter::StackObjectKind::Ability)
    );
}

#[test]
fn leading_if_result_scopes_every_coordinated_effect_arm() {
    let tokens = lex_line("If you do, draw a card and gain 2 life.", 0)
        .expect("coordinated if-result clause should lex");

    let effects =
        parse_effect_chain_lexed(&tokens).expect("coordinated if-result clause should parse");
    let [
        EffectAst::IfResult {
            predicate: crate::cards::builders::IfResultPredicate::Did,
            effects: conditional_effects,
        },
    ] = effects.as_slice()
    else {
        panic!("expected one if-result wrapper, got {effects:#?}");
    };
    let [
        EffectAst::Coordinated {
            effects: coordinated_effects,
            leading_duration: false,
            result_conjunction: true,
        },
    ] = conditional_effects.as_slice()
    else {
        panic!("expected coordinated result body, got {conditional_effects:#?}");
    };
    assert_eq!(coordinated_effects.len(), 2, "{coordinated_effects:#?}");
}

#[test]
fn leading_when_result_scopes_every_coordinated_effect_arm() {
    let tokens = lex_line("When you do, draw a card and gain 2 life.", 0)
        .expect("coordinated when-result clause should lex");

    let effects = parse_effect_chain_with_subject_verb_primitives_lexed(&tokens)
        .expect("coordinated when-result clause should parse through subject/verb dispatch");
    let [
        EffectAst::WhenResult {
            predicate: crate::cards::builders::IfResultPredicate::Did,
            effects: conditional_effects,
        },
    ] = effects.as_slice()
    else {
        panic!("expected one when-result wrapper, got {effects:#?}");
    };
    let [
        EffectAst::Coordinated {
            effects: coordinated_effects,
            leading_duration: false,
            result_conjunction: true,
        },
    ] = conditional_effects.as_slice()
    else {
        panic!("expected coordinated result body, got {conditional_effects:#?}");
    };
    assert_eq!(coordinated_effects.len(), 2, "{coordinated_effects:#?}");
}

#[test]
fn effect_sentence_inner_preserves_overseer_counter_grant_coordination() {
    let tokens = lex_line(
        "When you do, put a +1/+1 counter on each creature you control and they gain vigilance until end of turn.",
        0,
    )
    .expect("Overseer-style result clause should lex");

    let effects = parse_effect_sentence_inner_lexed(&tokens)
        .expect("Overseer-style result clause should parse through sentence dispatch");
    let [
        EffectAst::WhenResult {
            predicate: crate::cards::builders::IfResultPredicate::Did,
            effects: conditional_effects,
        },
    ] = effects.as_slice()
    else {
        panic!("expected one when-result wrapper, got {effects:#?}");
    };
    let [
        EffectAst::Coordinated {
            effects: coordinated_effects,
            leading_duration: false,
            result_conjunction: true,
        },
    ] = conditional_effects.as_slice()
    else {
        panic!("expected one coordinated result body, got {conditional_effects:#?}");
    };
    assert_eq!(coordinated_effects.len(), 2, "{coordinated_effects:#?}");
    let debug = format!("{coordinated_effects:#?}");
    assert!(debug.contains("PutCounters"), "{debug}");
    assert!(debug.contains("GrantAbilitiesToTarget"), "{debug}");
}

#[test]
fn direct_counter_followup_preserves_its_authored_conjunction() {
    let tokens = lex_line(
        "Put a +1/+1 counter on it and it gains haste until end of turn.",
        0,
    )
    .expect("counter-followup conjunction should lex");

    let effects = parse_effect_sentence_lexed(&tokens)
        .expect("counter-followup conjunction should parse through sentence dispatch");
    let [
        EffectAst::Coordinated {
            effects: coordinated,
            leading_duration: false,
            result_conjunction: false,
        },
    ] = effects.as_slice()
    else {
        panic!("expected one coordinated counter-followup clause, got {effects:#?}");
    };
    assert_eq!(coordinated.len(), 2, "{coordinated:#?}");
    let debug = format!("{coordinated:#?}");
    assert!(debug.contains("PutCounters"), "{debug}");
    assert!(debug.contains("GrantAbilitiesToTarget"), "{debug}");
}

#[test]
fn absolving_lammasu_one_clause_actions_keep_coordinated_surface() {
    let tokens = lex_line(
        "You gain 3 life and suspect up to one target creature an opponent controls.",
        0,
    )
    .expect("Absolving Lammasu effect should lex");

    let effects =
        parse_effect_sentence_lexed(&tokens).expect("Absolving Lammasu effect should parse");
    let [
        EffectAst::Coordinated {
            effects: coordinated,
            leading_duration: false,
            result_conjunction: false,
        },
    ] = effects.as_slice()
    else {
        panic!("expected one coordinated Lammasu clause, got {effects:#?}");
    };
    assert_eq!(coordinated.len(), 2, "{coordinated:#?}");
    assert!(
        coordinated.iter().any(|effect| matches!(
            effect,
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::GainLife { .. },
                ..
            })
        )),
        "{coordinated:#?}"
    );
    assert!(
        coordinated.iter().any(|effect| matches!(
            effect,
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::Suspect { .. },
                ..
            })
        )),
        "{coordinated:#?}"
    );
}

#[test]
fn aatchik_separate_sentence_actions_do_not_become_coordinated() {
    let tokens = lex_line(
        "Put a +1/+1 counter on this. Each opponent loses 1 life.",
        0,
    )
    .expect("Aatchik effect sentences should lex");

    let effects = super::super::parse_effect_sentences_lexed(&tokens)
        .expect("Aatchik effect sentences should parse");
    assert_eq!(effects.len(), 2, "{effects:#?}");
    assert!(
        effects
            .iter()
            .all(|effect| !matches!(effect, EffectAst::Coordinated { .. })),
        "separate Oracle sentences must remain ordinary siblings: {effects:#?}"
    );
}

#[test]
fn peregrination_search_partition_stays_a_specialist_bundle() {
    let tokens = lex_line(
        "Search your library for up to two basic land cards, reveal those cards, and put one onto the battlefield tapped and the other into your hand.",
        0,
    )
    .expect("Peregrination search sentence should lex");

    let effects =
        parse_effect_sentence_lexed(&tokens).expect("Peregrination search sentence should parse");
    let debug = format!("{effects:#?}");
    assert!(debug.contains("ChooseObjectsAcrossZones"), "{debug}");
    assert!(debug.contains("PutTaggedRemainderInZone"), "{debug}");
}

#[test]
fn extortion_hand_choice_stays_a_specialist_bundle() {
    let tokens = lex_line(
        "Look at target player's hand and choose up to two cards from it.",
        0,
    )
    .expect("Extortion hand-choice sentence should lex");

    let effects =
        parse_effect_sentence_lexed(&tokens).expect("Extortion hand-choice sentence should parse");
    assert!(format!("{effects:#?}").contains("ChooseObjects"));
}

#[test]
fn vraskas_fall_choice_and_consequences_do_not_become_coordinated() {
    let tokens = lex_line(
        "Each opponent sacrifices a creature or planeswalker of their choice and gets a poison counter.",
        0,
    )
    .expect("Vraska's Fall sentence should lex");

    let effects =
        parse_effect_sentence_lexed(&tokens).expect("Vraska's Fall sentence should parse");
    assert!(
        effects
            .iter()
            .all(|effect| !matches!(effect, EffectAst::Coordinated { .. })),
        "choice and its consequences must remain on the specialist path: {effects:#?}"
    );
}

#[test]
fn malboro_three_action_opponent_chain_does_not_become_coordinated() {
    let tokens = lex_line(
        "Each opponent discards a card, loses 2 life, and exiles the top three cards of their library.",
        0,
    )
    .expect("Malboro sentence should lex");

    let effects = parse_effect_sentence_lexed(&tokens).expect("Malboro sentence should parse");
    assert!(
        effects
            .iter()
            .all(|effect| !matches!(effect, EffectAst::Coordinated { .. })),
        "multi-action opponent chains must retain their existing specialist rendering: {effects:#?}"
    );
}

#[test]
fn triggered_lowering_keeps_sentences_separate_and_one_clause_coordinated() {
    let aatchik = CardDefinitionBuilder::new(CardId::from_raw(1), "Aatchik Boundary Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Whenever another Insect you control dies, put a +1/+1 counter on this creature. Each opponent loses 1 life.",
        )
        .expect("Aatchik-style trigger should lower");
    let aatchik_triggered = aatchik
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Aatchik-style fixture should produce a triggered ability");
    assert_eq!(
        aatchik_triggered.effects.segments.len(),
        2,
        "separate Oracle sentences must lower as separate resolution segments: {aatchik_triggered:#?}"
    );

    let lammasu = CardDefinitionBuilder::new(CardId::from_raw(2), "Lammasu Boundary Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "When this creature dies, you gain 3 life and suspect up to one target creature an opponent controls.",
        )
        .expect("Lammasu-style trigger should lower");
    let lammasu_triggered = lammasu
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Lammasu-style fixture should produce a triggered ability");
    assert_eq!(
        lammasu_triggered.effects.segments.len(),
        1,
        "one coordinated Oracle clause must stay in one resolution segment: {lammasu_triggered:#?}"
    );
    let [coordinated] = lammasu_triggered.effects.flattened_default_effects() else {
        panic!("expected one typed coordinated effect: {lammasu_triggered:#?}");
    };
    let sequence = coordinated
        .downcast_ref::<crate::effects::SequenceEffect>()
        .expect("Lammasu actions should retain a typed sequence");
    assert_eq!(
        sequence.surface,
        ironsmith_core::SequenceSurface::Coordinated,
        "Lammasu's single-clause conjunction must not become a sentence break"
    );
}

#[test]
fn leading_may_land_play_permission_does_not_lower_to_may_effect() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Explore")
        .parse_text("You may play an additional land this turn.\nDraw a card.")
        .expect("explore-style text should parse");

    let spell_debug = format!("{:?}", def.spell_effect.as_ref().expect("spell effects"));
    assert!(
        super::string_contains(&spell_debug, "AdditionalLandPlaysEffect")
            || super::string_contains(&spell_debug, "additional_land_plays"),
        "expected Explore-style permission text to lower to additional land plays, got {spell_debug}"
    );
}

#[test]
fn create_fragment_probe_accepts_capitalized_pt_token_clauses() {
    let tokens = lex_line("Two 1/1 white Soldier creature tokens", 0)
        .expect("rewrite lexer should classify create-fragment text");

    assert!(starts_like_create_fragment_lexed(&tokens));
}

#[test]
fn implicit_draw_then_discard_keeps_discard_on_ability_controller() {
    let tokens = lex_line("Draw an additional card, then discard a card.", 0)
        .expect("draw-discard fixture should lex");
    let effects = parse_effect_chain_lexed(&tokens).expect("draw-discard fixture should parse");

    let [EffectAst::CommaThen { effects: sequence }] = effects.as_slice() else {
        panic!("expected a typed comma-then draw/discard sequence, got {effects:#?}");
    };
    let [
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::Draw { .. },
            ..
        }),
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            subject,
            action: SubjectVerbActionAst::Discard { .. },
        }),
    ] = sequence.as_slice()
    else {
        panic!("expected adjacent draw and discard effects, got {sequence:#?}");
    };
    assert_eq!(subject.player, PlayerAst::You);
}

#[test]
fn imperative_then_draw_does_not_inherit_a_quantified_player_subject() {
    let tokens = lex_line("Then draw a card.", 0).expect("imperative draw should lex");
    let [mut effect] = parse_effect_chain_lexed(&tokens)
        .expect("imperative draw should parse")
        .try_into()
        .expect("imperative draw should be one effect");

    maybe_apply_carried_player_with_clause_lexed(&mut effect, CarryContext::ForEachPlayer, &tokens);

    assert!(
        matches!(
            effect,
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                subject: SubjectVerbSubjectAst {
                    player: PlayerAst::Implicit,
                    ..
                },
                action: SubjectVerbActionAst::Draw { .. },
            })
        ),
        "an imperative draw remains the ability controller's action: {effect:#?}"
    );
}

#[test]
fn source_damage_then_keyword_grant_keeps_coordinated_surface() {
    let tokens = lex_line(
        "This creature deals 2 damage to target player and gains indestructible until end of turn.",
        0,
    )
    .expect("source damage-and-grant fixture should lex");
    let effects =
        parse_effect_chain_lexed(&tokens).expect("source damage-and-grant fixture should parse");

    let [
        EffectAst::Coordinated {
            effects: coordinated,
            leading_duration: false,
            result_conjunction: false,
        },
    ] = effects.as_slice()
    else {
        panic!("expected coordinated source damage-and-grant clause, got {effects:#?}");
    };
    let debug = format!("{coordinated:#?}");
    assert!(debug.contains("DealDamage"), "{debug}");
    assert!(debug.contains("GrantAbilitiesToTarget"), "{debug}");
    assert!(debug.contains("Indestructible"), "{debug}");
}

#[test]
fn source_damage_then_tagged_keyword_loss_keeps_both_coordinated_actions() {
    let tokens = lex_line(
        "This creature deals 2 damage to target creature with flying and that creature loses flying until end of turn.",
        0,
    )
    .expect("source damage-and-loss fixture should lex");
    let effects =
        parse_effect_chain_lexed(&tokens).expect("source damage-and-loss fixture should parse");

    let [
        EffectAst::Coordinated {
            effects: coordinated,
            leading_duration: false,
            result_conjunction: false,
        },
    ] = effects.as_slice()
    else {
        panic!("expected coordinated source damage-and-loss clause, got {effects:#?}");
    };
    assert_eq!(coordinated.len(), 2, "{coordinated:#?}");
    let debug = format!("{coordinated:#?}");
    assert!(debug.contains("DealDamage"), "{debug}");
    assert!(debug.contains("RemoveAbilitiesFromTarget"), "{debug}");
    assert!(debug.contains("Flying"), "{debug}");
}

#[test]
fn trailing_duration_applies_to_both_gain_and_loss_arms() {
    let tokens = lex_line(
        "This creature gains flying and loses trample until end of turn.",
        0,
    )
    .expect("gain-and-loss duration fixture should lex");
    let effects =
        parse_effect_chain_lexed(&tokens).expect("gain-and-loss duration fixture should parse");

    let [
        EffectAst::Coordinated {
            effects: coordinated,
            leading_duration: false,
            result_conjunction: false,
        },
    ] = effects.as_slice()
    else {
        panic!("expected coordinated gain-and-loss clause, got {effects:#?}");
    };
    let [
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::GrantAbilitiesToTarget {
                    duration: first_duration,
                    ..
                },
            ..
        }),
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::RemoveAbilitiesFromTarget {
                    duration: second_duration,
                    ..
                },
            ..
        }),
    ] = coordinated.as_slice()
    else {
        panic!("expected source gain followed by source loss, got {coordinated:#?}");
    };
    assert_eq!(first_duration, &crate::effect::Until::EndOfTurn);
    assert_eq!(second_duration, &crate::effect::Until::EndOfTurn);
}

#[test]
fn next_turn_duration_scopes_life_lock_and_player_protection() {
    let tokens = lex_line(
        "Until your next turn, your life total can't change and you gain protection from everything.",
        0,
    )
    .expect("Teferi's Protection opening sentence should lex");
    let effects = parse_effect_sentence_lexed(&tokens)
        .expect("the sentence entrypoint should retain the shared duration");
    let debug = format!("{effects:#?}");

    assert!(
        debug.contains("ChangeLifeTotal")
            && debug.contains("BeTargetedPlayer")
            && debug.contains("PreventAllDamageToTarget")
            && debug.matches("YourNextTurn").count() >= 3,
        "expected one life-total lock and the two protection components through your next turn, got {debug}"
    );
}

#[test]
fn coordinated_player_restrictions_route_before_broad_subject_parsing() {
    let tokens = lex_line(
        "Players can't lose life this turn and players can't lose the game or win the game this turn.",
        0,
    )
    .expect("Everybody Lives restriction sentence should lex");
    let effects = parse_effect_sentence_lexed(&tokens)
        .expect("the sentence entrypoint should route the complete restriction conjunction");
    let debug = format!("{effects:#?}");

    assert!(
        debug.contains("LoseLife")
            && debug.contains("LoseGame")
            && debug.contains("WinGame")
            && debug.matches("EndOfTurn").count() >= 3,
        "expected all three player restrictions through end of turn, got {debug}"
    );
}

#[test]
fn trailing_duration_applies_to_ability_loss_before_type_change() {
    let tokens = lex_line(
        "This creature loses defender and becomes a Human until end of turn.",
        0,
    )
    .expect("loss-and-type duration fixture should lex");
    let effects =
        parse_effect_chain_lexed(&tokens).expect("loss-and-type duration fixture should parse");

    let [
        EffectAst::Coordinated {
            effects: coordinated,
            leading_duration: false,
            result_conjunction: false,
        },
    ] = effects.as_slice()
    else {
        panic!("expected coordinated loss-and-type clause, got {effects:#?}");
    };
    let [
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::RemoveAbilitiesFromTarget {
                    duration: first_duration,
                    ..
                },
            ..
        }),
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::SetCreatureSubtypes {
                    duration: second_duration,
                    ..
                }
                | SubjectVerbActionAst::AddSubtypes {
                    duration: second_duration,
                    ..
                },
            ..
        }),
    ] = coordinated.as_slice()
    else {
        panic!("expected source loss followed by source type change, got {coordinated:#?}");
    };
    assert_eq!(first_duration, &crate::effect::Until::EndOfTurn);
    assert_eq!(second_duration, &crate::effect::Until::EndOfTurn);
}

#[test]
fn tap_then_next_untap_conjunction_keeps_coordinated_surface() {
    let tokens = lex_line(
        "Tap target creature and it doesn't untap during its controller's next untap step.",
        0,
    )
    .expect("freeze conjunction fixture should lex");
    let effects =
        parse_effect_chain_lexed(&tokens).expect("freeze conjunction fixture should parse");

    let [
        EffectAst::Coordinated {
            effects: coordinated,
            leading_duration: false,
            result_conjunction: false,
        },
    ] = effects.as_slice()
    else {
        panic!("expected coordinated tap/freeze clause, got {effects:#?}");
    };
    assert_eq!(coordinated.len(), 2, "{coordinated:#?}");
    assert!(
        matches!(
            coordinated.first(),
            Some(EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::Tap { .. },
                ..
            }))
        ),
        "{coordinated:#?}"
    );
    assert!(
        matches!(
            coordinated.get(1),
            Some(EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::Cant {
                    restriction: crate::effect::Restriction::Untap(_),
                    duration: crate::effect::Until::ControllersNextUntapStep,
                    condition: None,
                    ..
                },
                ..
            }))
        ),
        "{coordinated:#?}"
    );
}

#[test]
fn create_fragment_probe_accepts_named_token_appositive_clauses() {
    let tokens = lex_line(
        "a legendary 2/1 black Skeleton creature token with \"Jumblebones can't block\"",
        0,
    )
    .expect("rewrite lexer should classify named-token appositive text");

    assert!(starts_like_create_fragment_lexed(&tokens));
}

#[test]
fn parses_named_token_appositive_with_quoted_trigger_rules() {
    let tokens = lex_line(
        "Create Jumblebones, a legendary 2/1 black Skeleton creature token with \"Jumblebones can't block\" and \"When Jumblebones leaves the battlefield, return target card named Ozox, the Clattering King from your graveyard to your hand.\"",
        0,
    )
    .expect("named-token appositive should lex");

    parse_effect_chain_lexed(&tokens)
        .expect("named-token appositive with nested token trigger should parse");
}

#[test]
fn parses_target_card_type_list_with_lte_mana_value_reference() {
    let tokens = lex_line(
        "Exile target enchantment, instant, or sorcery card with equal or lesser mana value than that spell from an opponent's graveyard",
        0,
    )
    .expect("target list clause should lex");

    parse_effect_chain_lexed(&tokens).expect("target list clause should parse");
}

#[test]
fn coordinated_tap_set_stays_one_antecedent_for_then_them() {
    let tokens = lex_line(
        "Tap this creature and all creatures named Kobolds of Kher Keep, then an opponent gains control of them.",
        0,
    )
    .expect("coordinated tap chain should lex");

    let effects = parse_effect_chain_lexed(&tokens).expect("coordinated tap chain should parse");
    let [EffectAst::CommaThen { effects: sequence }] = effects.as_slice() else {
        panic!("expected a typed comma-then tap/control sequence, got {effects:#?}");
    };
    let [
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::TapAll { filter },
            ..
        }),
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::GainControl { .. },
            ..
        }),
    ] = sequence.as_slice()
    else {
        panic!("expected tap-union then gain-control effects, got {sequence:#?}");
    };
    assert_eq!(filter.any_of.len(), 2, "{filter:#?}");
    assert!(filter.any_of[0].source, "{filter:#?}");
    assert_eq!(
        filter.any_of[1].name.as_deref(),
        Some("kobolds of kher keep")
    );
}

#[test]
fn coordinated_tap_named_source_set_stays_one_antecedent() {
    let tokens = lex_line(
        "tap Rohgahh and all creatures named Kobolds of Kher Keep, then an opponent gains control of them.",
        0,
    )
    .expect("coordinated named-source tap chain should lex");
    let effects =
        with_source_reference_context("Rohgahh of Kher Keep", || parse_effect_chain_lexed(&tokens))
            .expect("coordinated named-source tap chain should parse");
    let [EffectAst::CommaThen { effects: sequence }] = effects.as_slice() else {
        panic!("expected a typed comma-then tap/control sequence, got {effects:#?}");
    };
    assert_eq!(sequence.len(), 2, "{sequence:#?}");
}

#[test]
fn conditional_named_source_tap_set_stays_one_antecedent() {
    let tokens = lex_line(
        "If you don't, tap Rohgahh and all creatures named Kobolds of Kher Keep, then an opponent gains control of them.",
        0,
    )
    .expect("conditional named-source tap chain should lex");
    let effects = with_source_reference_context("Rohgahh of Kher Keep", || {
        parse_effect_sentence_lexed(&tokens)
    })
    .expect("conditional named-source tap chain should parse");
    assert!(
        matches!(
            effects.as_slice(),
            [EffectAst::IfResult {
                predicate: crate::cards::builders::IfResultPredicate::DidNot,
                effects,
            }] if matches!(
                effects.as_slice(),
                [EffectAst::CommaThen { effects }] if effects.len() == 2
            )
        ),
        "{effects:#?}"
    );
}

#[test]
fn discard_up_to_two_then_draw_binds_the_actual_discard_outcome() {
    let tokens = lex_line("Discard up to two cards, then draw that many cards.", 0)
        .expect("discard/draw chain should lex");

    let effects = parse_effect_chain_lexed(&tokens).expect("discard/draw chain should parse");
    let [EffectAst::CommaThen { effects: sequence }] = effects.as_slice() else {
        panic!("expected a typed comma-then discard/draw sequence, got {effects:#?}");
    };
    let [
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::Discard {
                    count: discard_count,
                    any_number,
                    ..
                },
            ..
        }),
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::Draw { count: draw_count },
            ..
        }),
    ] = sequence.as_slice()
    else {
        panic!("expected adjacent discard and draw effects, got {sequence:#?}");
    };

    assert_eq!(discard_count, &Value::Fixed(2));
    assert!(*any_number, "up to two must allow choosing fewer than two");
    assert!(matches!(
        draw_count.unhinted(),
        Value::PendingEffectMetric {
            source: ironsmith_core::EffectMetricSource::Outcome,
            metric: ironsmith_core::EffectMetric::Count,
        }
    ));
}

#[test]
fn targeted_discard_for_each_clauses_keep_dynamic_counts() {
    for (card, text) in [
        (
            "Mind Sludge",
            "Target player discards a card for each Swamp you control.",
        ),
        (
            "Shrine of Limitless Power",
            "Target player discards a card for each charge counter on this artifact.",
        ),
        (
            "Sink into Takenuma",
            "Target player discards a card for each Swamp returned this way.",
        ),
    ] {
        let tokens = lex_line(text, 0).expect("targeted discard clause should lex");
        let effects = parse_effect_chain_lexed(&tokens)
            .unwrap_or_else(|error| panic!("{card} discard clause should parse: {error}"));
        let [
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                subject:
                    SubjectVerbSubjectAst {
                        player: PlayerAst::Target,
                        ..
                    },
                action: SubjectVerbActionAst::Discard { count, .. },
            }),
        ] = effects.as_slice()
        else {
            panic!("{card} should lower to one targeted discard effect: {effects:#?}");
        };

        assert_ne!(
            count,
            &Value::Fixed(1),
            "{card} must keep its dynamic for-each discard count"
        );
    }
}

#[test]
fn congregate_targeted_gain_for_each_keeps_dynamic_amount() {
    let tokens = lex_line(
        "Target player gains 2 life for each creature on the battlefield.",
        0,
    )
    .expect("Congregate life-gain clause should lex");
    let effects = parse_effect_chain_lexed(&tokens)
        .expect("Congregate life-gain clause should parse through the gain-life family");
    let [
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            subject:
                SubjectVerbSubjectAst {
                    player: PlayerAst::Target,
                    ..
                },
            action: SubjectVerbActionAst::GainLife { amount },
        }),
    ] = effects.as_slice()
    else {
        panic!("Congregate should lower to one targeted life-gain effect: {effects:#?}");
    };
    let Value::CountScaled(filter, 2) = amount.unhinted() else {
        panic!("Congregate must scale a creature count by two: {amount:#?}");
    };

    assert!(
        filter.card_types.contains(&CardType::Creature),
        "Congregate must count creatures: {filter:#?}"
    );
    assert_eq!(filter.zone, Some(Zone::Battlefield), "{filter:#?}");
}

#[test]
fn devouring_greed_targeted_loss_keeps_base_plus_scaled_sacrifice_count() {
    let tokens = lex_line(
        "Target player loses 2 life plus 2 life for each Spirit sacrificed this way.",
        0,
    )
    .expect("Devouring Greed life-loss clause should lex");
    let effects = parse_effect_chain_lexed(&tokens)
        .expect("Devouring Greed life-loss clause should parse through the lose-life family");
    let [
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            subject:
                SubjectVerbSubjectAst {
                    player: PlayerAst::Target,
                    ..
                },
            action: SubjectVerbActionAst::LoseLife { amount },
        }),
    ] = effects.as_slice()
    else {
        panic!("Devouring Greed should lower to one targeted life-loss effect: {effects:#?}");
    };
    let Value::Add(base, addend) = amount.unhinted() else {
        panic!("Devouring Greed must retain its base and dynamic terms: {amount:#?}");
    };
    assert_eq!(base.unhinted(), &Value::Fixed(2), "{amount:#?}");
    let Value::Scaled(metric, 2) = addend.unhinted() else {
        panic!("Devouring Greed dynamic term must be doubled: {amount:#?}");
    };
    let Value::PendingPriorEffectMetric(query) = metric.unhinted() else {
        panic!("Devouring Greed must count the prior sacrifice result: {amount:#?}");
    };

    assert_eq!(
        query.action,
        Some(ironsmith_core::PriorEffectAction::Sacrificed),
        "{query:#?}"
    );
    assert!(
        query
            .filter
            .as_ref()
            .is_some_and(|filter| filter.subtypes.contains(&Subtype::Spirit)),
        "Devouring Greed must count sacrificed Spirits: {query:#?}"
    );
}

#[test]
fn gain_toughness_lose_power_then_put_keeps_all_three_actions() {
    let tokens = lex_line(
        "You gain life equal to that card's toughness, lose life equal to its power, then put it into your hand.",
        0,
    )
    .expect("life-stat chain should lex");

    let effects = parse_effect_chain_lexed(&tokens).expect("life-stat chain should parse");
    let [EffectAst::CommaThen { effects: sequence }] = effects.as_slice() else {
        panic!("expected a typed comma-then life-stat sequence, got {effects:#?}");
    };
    let [
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::GainLife { amount: _ },
            ..
        }),
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::LoseLife { amount: _ },
            ..
        }),
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::MoveToZone {
                    zone: Zone::Hand, ..
                },
            ..
        }),
    ] = sequence.as_slice()
    else {
        panic!("expected gain-toughness, lose-power, then put-into-hand, got {sequence:#?}");
    };

    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action: SubjectVerbActionAst::GainLife {
            amount: gain_amount,
        },
        ..
    }) = &sequence[0]
    else {
        unreachable!();
    };
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action: SubjectVerbActionAst::LoseLife {
            amount: lose_amount,
        },
        ..
    }) = &sequence[1]
    else {
        unreachable!();
    };
    let Value::ToughnessOf(gain_spec) = gain_amount.unhinted() else {
        unreachable!();
    };
    let Value::PowerOf(lose_spec) = lose_amount.unhinted() else {
        unreachable!();
    };
    assert_eq!(gain_spec.unhinted(), lose_spec.unhinted());
    assert!(matches!(
        lose_spec.unhinted(),
        crate::target::ChooseSpec::Tagged(tag) if tag.as_str() == crate::cards::builders::IT_TAG
    ));
}

#[test]
fn conditional_reveal_moves_preserve_explicit_contextual_destinations() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Reveal Destination Variant")
        .parse_text(
            "Reveal the top card of your library. If it's a creature card, put it onto the battlefield. Otherwise, put it into your graveyard.",
        )
        .expect("conditional reveal destination should parse");
    let debug = format!("{:#?}", def.spell_effect);

    assert!(debug.contains("zone: Graveyard"), "{debug}");
    assert!(
        debug.contains("destination_player_surface: Some(\n") && debug.contains("You"),
        "explicit your-graveyard surface was lost: {debug}"
    );
}

#[test]
fn return_to_hand_preserves_explicit_contextual_destination() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(2), "Return Destination Variant")
        .parse_text("Return target permanent card from your graveyard to your hand.")
        .expect("contextual return destination should parse");
    let debug = format!("{:#?}", def.spell_effect);

    assert!(debug.contains("ReturnFromGraveyardToHandEffect"), "{debug}");
    assert!(
        debug.contains("destination_player_surface: Some(") && debug.contains("You"),
        "explicit your-hand surface was lost: {debug}"
    );
}

#[test]
fn source_card_return_preserves_identity_and_explicit_graveyard() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(3), "Chandra's Phoenix")
        .parse_text("Return this card from your graveyard to your hand.")
        .expect("source-card return should parse");
    let debug = format!("{:#?}", def.spell_effect);

    assert!(debug.contains("ReturnFromGraveyardToHandEffect"), "{debug}");
    assert!(
        debug.contains("target: Source")
            && debug.contains("graveyard_player_surface: Some(\n")
            && debug.contains("You"),
        "{debug}"
    );
    assert!(
        debug.contains("destination_player_surface: Some("),
        "{debug}"
    );
}

#[test]
fn chain_entrypoint_accepts_nonverb_additional_phase_clause() {
    let tokens = lex_line("There's an additional combat phase after this phase.", 0)
        .expect("additional phase clause should lex");

    let effects = parse_effect_chain_lexed(&tokens).expect("additional phase should parse");
    assert!(
        matches!(
            effects.as_slice(),
            [EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::AdditionalPhases { .. },
                ..
            })]
        ),
        "{effects:#?}"
    );
}

#[test]
fn copy_then_gain_clause_keeps_the_explicit_gain_duration() {
    let tokens = lex_line(
        "Each land you control of that type becomes a copy of target creature you control until end of turn and gains haste until end of turn.",
        0,
    )
    .expect("copy-and-gain clause should lex");

    let effects = parse_effect_chain_lexed(&tokens).expect("copy-and-gain clause should parse");
    let gain_effects = match effects.as_slice() {
        [EffectAst::Coordinated { effects, .. }] => effects.as_slice(),
        _ => effects.as_slice(),
    };
    let gain = gain_effects
        .iter()
        .find_map(|effect| match effect {
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::GrantAbilitiesAll { duration, .. },
                ..
            }) => Some(duration),
            _ => None,
        })
        .unwrap_or_else(|| panic!("expected an all-lands haste grant, got {effects:#?}"));
    assert_eq!(*gain, crate::effect::Until::EndOfTurn, "{effects:#?}");
}

#[test]
fn leading_action_keeps_following_get_and_gain_on_one_shared_object_set() {
    let tokens = lex_line(
        "You draw X cards and the chosen creatures get +X/+X and gain trample until end of turn, where X is the difference between the chosen creatures' powers.",
        0,
    )
    .expect("draw followed by shared-subject pump and grant should lex");

    let effects = parse_effect_chain_lexed(&tokens)
        .expect("draw followed by shared-subject pump and grant should parse");
    let [
        EffectAst::Coordinated {
            effects: shared_effects,
            leading_duration: false,
            result_conjunction: false,
        },
    ] = effects.as_slice()
    else {
        panic!("expected one coordinated draw/pump/grant clause, got {effects:#?}");
    };
    let [
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::Draw { .. },
            ..
        }),
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::PumpAll {
                    filter: pump_filter,
                    ..
                },
            ..
        }),
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::GrantAbilitiesAll {
                    filter: grant_filter,
                    abilities,
                    ..
                },
            ..
        }),
    ] = shared_effects.as_slice()
    else {
        panic!("expected one shared-subject pump and ability grant, got {shared_effects:#?}");
    };

    assert_eq!(pump_filter, grant_filter);
    assert!(pump_filter.tagged_constraints.iter().any(|constraint| {
        constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
    }));
    assert!(abilities.iter().any(|ability| match ability {
        crate::cards::builders::GrantedAbilityAst::KeywordAction(
            crate::cards::builders::KeywordAction::Trample,
        ) => true,
        crate::cards::builders::GrantedAbilityAst::StaticAbility(ability) => {
            ability.id() == crate::static_abilities::StaticAbilityId::Trample
        }
        _ => false,
    }));
}

#[test]
fn trailing_if_keeps_relative_target_spell_controller_predicate() {
    let tokens = lex_line(
        "Counter target spell if you control more creatures than that spell's controller.",
        0,
    )
    .expect("relative counter condition should lex");

    let effect = parse_effect_clause_with_trailing_if_lexed(&tokens)
        .expect("relative counter condition should parse");
    assert!(
        matches!(
            effect,
            EffectAst::TrailingIf {
                predicate: crate::cards::builders::PredicateAst::YouControlMoreCreaturesThanTargetSpellController,
                ..
            }
        ),
        "{effect:#?}"
    );
}

#[test]
fn trailing_if_binds_its_mana_value_to_the_declared_object_target() {
    let tokens = lex_line(
        "Return target artifact card from your graveyard to the battlefield if its mana value is less than or equal to their total power.",
        0,
    )
    .expect("targeted conditional return should lex");

    let effect = parse_effect_clause_with_trailing_if_lexed(&tokens)
        .expect("targeted conditional return should parse");
    let EffectAst::TrailingIf {
        predicate:
            crate::cards::builders::PredicateAst::ValueComparison {
                left: crate::effect::Value::ManaValueOf(spec),
                ..
            },
        ..
    } = effect
    else {
        panic!("expected a mana-value-gated return, got {effect:#?}");
    };
    assert!(
        matches!(
            spec.unhinted(),
            crate::target::ChooseSpec::Target(inner)
                if matches!(inner.base(), crate::target::ChooseSpec::Object(filter)
                    if filter.card_types.contains(&CardType::Artifact))
        ),
        "the condition must inspect the declared artifact target, got {spec:#?}"
    );
    assert_eq!(
        spec.source_reference_surface(),
        Some(&crate::target::SourceReferenceSurface::ThisPermanentType(
            "it".to_string(),
        ),),
        "binding the semantic target must retain the authored possessive pronoun"
    );
}

#[test]
fn trailing_if_dispatch_preserves_face_down_return_then_turn_procedure() {
    let tokens = lex_line(
        "Return it to the battlefield face down under its owner's control if it's a permanent card, then turn it face up.",
        0,
    )
    .expect("face-down return procedure should lex");

    let effect = parse_effect_clause_with_trailing_if_lexed(&tokens)
        .expect("face-down return procedure should parse");
    let EffectAst::TrailingIf { predicate, effects } = effect else {
        panic!("expected a resolution-time condition, got {effect:#?}");
    };
    assert_eq!(effects.len(), 2, "{effects:#?}");
    assert!(matches!(
        &effects[0],
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::MoveToZone {
                zone: Zone::Battlefield,
                battlefield_face_down: true,
                ..
            },
            ..
        })
    ));
    assert!(matches!(
        &effects[1],
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::TurnFaceUp { .. },
            ..
        })
    ));
    assert!(
        !format!("{predicate:#?}").contains("face_down: Some(false)"),
        "the turn-face-up followup must remain outside the permanent-card predicate"
    );
}

#[test]
fn conditional_transform_keeps_the_control_threshold_as_a_resolution_gate() {
    let tokens = lex_line(
        "Transform this artifact if you control four or more artifacts.",
        0,
    )
    .expect("conditional transform should lex");
    let effects =
        parse_effect_chain_lexed(&tokens).expect("conditional transform should parse as a chain");
    let [
        EffectAst::TrailingIf {
            predicate:
                crate::cards::builders::PredicateAst::PlayerHasAtLeast {
                    player,
                    filter,
                    count,
                },
            effects: gated_effects,
        },
    ] = effects.as_slice()
    else {
        panic!("expected a threshold-gated transform effect, got {effects:#?}");
    };

    assert_eq!(*player, PlayerAst::You);
    assert_eq!(*count, 4);
    assert!(filter.card_types.contains(&CardType::Artifact));
    assert!(
        matches!(
            gated_effects.as_slice(),
            [EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::Transform { .. },
                ..
            })]
        ),
        "{gated_effects:#?}"
    );
}

#[test]
fn untap_then_transform_keeps_both_executable_actions() {
    let tokens = lex_line("Untap this creature, then transform it.", 0)
        .expect("untap-transform sequence should lex");
    let effects = parse_effect_chain_lexed(&tokens).expect("untap-transform sequence should parse");
    let debug = format!("{effects:#?}");

    assert!(debug.contains("Untap"), "{debug}");
    assert!(debug.contains("Transform"), "{debug}");
}

#[test]
fn harness_chain_segment_uses_typed_keyword_action() {
    let tokens = lex_line("Harness this", 0).expect("harness chain segment should lex");
    let effects = parse_effect_sentence_lexed(&tokens).expect("harness should parse");
    assert!(matches!(
        effects.as_slice(),
        [crate::cards::builders::EffectAst::SubjectVerb(
            SubjectVerbEffectAst {
                action: SubjectVerbActionAst::EmitKeywordAction {
                    action: crate::events::KeywordActionKind::Harness,
                    amount: 1,
                },
                ..
            }
        )]
    ));
}

#[test]
fn source_linked_exile_reveal_keeps_nonpermanents_face_up_and_moves_only_permanents() {
    let tokens = lex_line(
        "Each player turns face up all cards they own exiled with this artifact, then puts all permanent cards among them onto the battlefield.",
        0,
    )
    .expect("source-linked exile sequence should lex");

    let effects = parse_effect_chain_lexed(&tokens).expect("sequence should parse");
    let sentence_effects =
        parse_effect_sentence_lexed(&tokens).expect("sentence entrypoint should parse");
    assert_eq!(sentence_effects, effects);
    let [EffectAst::ForEachPlayer { effects: nested }] = effects.as_slice() else {
        panic!("expected per-player source-linked sequence, got {effects:#?}");
    };
    let nested = match nested.as_slice() {
        [EffectAst::CommaThen { effects }] => effects.as_slice(),
        effects => effects,
    };
    let [
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::TurnFaceUp { target },
            ..
        }),
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::ReturnAllToBattlefield { filter, .. },
            ..
        }),
    ] = nested
    else {
        panic!("expected reveal then permanent-return effects, got {nested:#?}");
    };
    let crate::cards::builders::TargetAst::Object(reveal_filter, None, None) = target else {
        panic!("expected non-target reveal filter, got {target:#?}");
    };
    for candidate in [reveal_filter, filter] {
        assert_eq!(candidate.zone, Some(Zone::Exile));
        assert_eq!(
            candidate.owner,
            Some(crate::target::PlayerFilter::IteratedPlayer)
        );
        assert_eq!(
            candidate.source_surface,
            Some(crate::target::SourceReferenceSurface::ThisPermanentType(
                "this artifact".to_string()
            ))
        );
        assert!(
            candidate
                .tagged_constraints
                .iter()
                .any(|constraint| { constraint.tag.as_str() == crate::tag::SOURCE_EXILED_TAG })
        );
    }
    assert!(reveal_filter.card_types.is_empty(), "{reveal_filter:#?}");
    assert_eq!(filter.card_types.len(), 6, "{filter:#?}");
}

#[test]
fn leading_player_may_probe_accepts_capitalized_opponent_clauses() {
    let tokens = lex_line("An opponent may cast it", 0)
        .expect("rewrite lexer should classify player-may text");

    assert_eq!(
        parse_leading_player_may_lexed(&tokens),
        Some(PlayerAst::Opponent)
    );
}

#[test]
fn leading_player_may_probe_accepts_then_target_player_clauses() {
    let tokens = lex_line("Then target player may draw a card", 0)
        .expect("rewrite lexer should classify target-player may text");

    assert_eq!(
        parse_leading_player_may_lexed(&tokens),
        Some(PlayerAst::Target)
    );
}

#[test]
fn leading_player_may_probe_accepts_possessive_controller_clauses() {
    let tokens = lex_line("That creature's controller may cast it", 0)
        .expect("rewrite lexer should classify possessive controller text");

    assert_eq!(
        parse_leading_player_may_lexed(&tokens),
        Some(PlayerAst::ItsController)
    );
}

#[test]
fn leading_player_may_probe_accepts_that_attacking_player_clauses() {
    let tokens = lex_line("That attacking player may create a tapped Zombie token", 0)
        .expect("rewrite lexer should classify attacking-player may text");

    assert_eq!(
        parse_leading_player_may_lexed(&tokens),
        Some(PlayerAst::Attacking)
    );
}

#[test]
fn leading_player_may_probe_accepts_that_player_or_target_controller_clauses() {
    let tokens = lex_line(
        "That player or that permanent's controller may draw a card",
        0,
    )
    .expect("rewrite lexer should classify split controller text");

    assert_eq!(
        parse_leading_player_may_lexed(&tokens),
        Some(PlayerAst::ThatPlayerOrTargetController)
    );
}

#[test]
fn effect_sentence_keeps_split_target_actor_and_optional_payment() {
    let tokens = lex_line(
        "Then that player or that permanent's controller may pay {R}{R}.",
        0,
    )
    .expect("chain payment sentence should lex");

    let effects = parse_effect_sentence_inner_lexed(&tokens)
        .expect("chain payment sentence should preserve its actor and optionality");
    let [
        EffectAst::MayByPlayer {
            player: PlayerAst::ThatPlayerOrTargetController,
            effects: payment,
        },
    ] = effects.as_slice()
    else {
        panic!("expected a split-actor optional payment, got {effects:#?}");
    };
    assert!(
        matches!(
            payment.as_slice(),
            [EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::PayMana { .. },
                ..
            })]
        ),
        "expected one typed mana payment, got {payment:#?}"
    );
}

#[test]
fn top_cards_then_put_counted_into_hand_rest_graveyard_chain_parses() {
    let tokens = lex_line(
        "Look at the top three cards of your library, then put one of them into your hand and the rest into your graveyard",
        0,
    )
    .expect("looked-cards split clause should lex");

    let effects =
        parse_effect_chain_lexed(&tokens).expect("looked-cards split clause should parse");

    let [EffectAst::CommaThen { effects: sequence }] = effects.as_slice() else {
        panic!("expected a typed comma-then looked-card sequence, got {effects:#?}");
    };
    match sequence.as_slice() {
        [
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::LookAtTopCards { .. },
                ..
            }),
            EffectAst::ChooseTaggedObjectsInZone {
                player,
                count,
                tag: hand_tag,
                zone: Zone::Library,
                ..
            },
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::TagMatchingObjects {
                        filter,
                        tag: remainder_tag,
                        ..
                    },
                ..
            }),
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::MoveToZone {
                        target: crate::cards::builders::TargetAst::Tagged(moved_hand_tag, _),
                        zone: Zone::Hand,
                        ..
                    },
                ..
            }),
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::MoveToZone {
                        target: crate::cards::builders::TargetAst::Tagged(moved_remainder_tag, _),
                        zone: Zone::Graveyard,
                        ..
                    },
                ..
            }),
        ] => {
            assert_eq!(*player, PlayerAst::You);
            assert_eq!(*count, crate::effect::ChoiceCount::exactly(1));
            assert_eq!(moved_hand_tag, hand_tag);
            assert_eq!(moved_remainder_tag, remainder_tag);
            assert!(
                filter.tagged_constraints.iter().any(|constraint| {
                    &constraint.tag == hand_tag
                        && matches!(
                            constraint.relation,
                            crate::filter::TaggedOpbjectRelation::IsNotTaggedObject
                        )
                }),
                "the remainder must exclude the chosen hand card: {filter:#?}"
            );
        }
        other => panic!("expected composed looked-cards split effects, got {other:?}"),
    }
}

#[test]
fn exile_then_shuffle_graveyard_chain_keeps_both_effects() {
    let tokens = lex_line(
        "Exile all cards from your library face down, then shuffle all cards from your graveyard into your library.",
        0,
    )
    .expect("rewrite lexer should classify exile-then-shuffle text");
    let effects = parse_effect_chain_lexed(&tokens).expect("chain should parse");
    let debug = format!("{effects:?}");

    assert!(
        debug.contains("ExileAll")
            && debug.contains("face_down: true")
            && debug.contains("ShuffleGraveyardIntoLibrary"),
        "expected exile-all face-down and graveyard shuffle effects, got {debug}"
    );
    let [EffectAst::CommaThen { effects: sequence }] = effects.as_slice() else {
        panic!("expected a typed comma-then exile/shuffle sequence, got {effects:#?}");
    };
    assert!(
        sequence.iter().any(|effect| matches!(
            effect,
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::ExileAll {
                    face_down: true,
                    ..
                },
                ..
            })
        )),
        "expected a face-down exile-all effect in the parsed chain: {debug}"
    );
    assert!(
        sequence.iter().any(|effect| {
            matches!(
                effect,
                EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action: SubjectVerbActionAst::ShuffleGraveyardIntoLibrary { .. },
                    ..
                })
            )
        }),
        "expected a graveyard shuffle effect in the parsed chain: {debug}"
    );
}

#[test]
fn or_action_clause_preserves_secondary_or_inside_sacrifice_filter() {
    let tokens = lex_line(
        "Discard two cards or sacrifice a creature or planeswalker of your choice",
        0,
    )
    .expect("or-action text should lex");

    let parsed = super::parse_or_action_clause_lexed(&tokens)
        .expect("or-action parse should succeed")
        .expect("or-action clause should be recognized");

    let debug = format!("{parsed:?}");
    assert!(
        debug.contains("UnlessAction"),
        "expected or-action lowering to use unless-action AST, got {debug}"
    );
    assert!(
        debug.contains("Discard"),
        "expected discard branch in or-action AST, got {debug}"
    );
    assert!(
        debug.contains("Sacrifice"),
        "expected sacrifice branch in or-action AST, got {debug}"
    );
    assert!(
        debug.contains("Planeswalker"),
        "expected sacrifice filter to keep planeswalker branch, got {debug}"
    );
}

#[test]
fn or_action_clause_accepts_an_explicit_source_gain_choice_branch() {
    let tokens = lex_line(
        "Put a +1/+1 counter on this creature or this creature gains flying, first strike, or trample.",
        0,
    )
    .expect("counter-or-ability-choice text should lex");
    let action_splits = super::chain_grammar::parse_or_action_splits_tokens(&tokens);
    let action_split = action_splits
        .iter()
        .find(|split| {
            crate::runtime_backend::token_word_refs(split.second_tokens)
                .starts_with(&["this", "creature", "gains"])
        })
        .expect("outer action `or` should remain distinct from the nested ability choices");
    assert!(
        !super::parse_effect_chain_with_subject_verb_primitives_lexed(action_split.first_tokens)
            .expect("counter branch should parse")
            .is_empty()
    );
    let branch_tokens = lex_line("this creature gains flying, first strike, or trample.", 0)
        .expect("ability-choice branch should lex");
    let branch = super::parse_simple_gain_ability_clause_lexed(&branch_tokens)
        .expect("ability-choice branch parse should succeed")
        .expect("ability-choice branch should be recognized");
    assert!(
        format!("{branch:#?}").contains("GrantAbilitiesChoiceToTarget"),
        "{branch:#?}"
    );
    assert!(
        format!("{branch:#?}").contains("source: true"),
        "the explicit `this creature` subject must remain source-relative: {branch:#?}"
    );

    let parsed = super::parse_or_action_clause_lexed(&tokens)
        .expect("or-action parse should succeed")
        .expect("the explicit source gain branch should be recognized");
    let debug = format!("{parsed:#?}");

    assert!(debug.contains("UnlessAction"), "{debug}");
    assert!(debug.contains("PutCounters"), "{debug}");
    assert!(
        debug.contains("GrantAbilitiesChoiceToTarget")
            && debug.contains("Flying")
            && debug.contains("FirstStrike")
            && debug.contains("Trample"),
        "the second branch must retain all three mutually exclusive abilities: {debug}"
    );

    let routed = parse_effect_sentence_lexed(&tokens)
        .expect("whole-sentence dispatch should preserve both choice branches");
    assert!(
        matches!(routed.as_slice(), [EffectAst::UnlessAction { .. }]),
        "the broad gain parser must not consume the leading counter action: {routed:#?}"
    );

    let near_miss = lex_line(
        "Put a +1/+1 counter on this creature or flying, first strike, or trample.",
        0,
    )
    .expect("near-miss text should lex");
    assert!(
        super::parse_or_action_clause_lexed(&near_miss)
            .expect("near miss must not error")
            .is_none(),
        "an orphaned ability list must not be promoted into an action branch"
    );
}

#[test]
fn or_action_clause_reuses_the_primary_explicit_target_for_a_demonstrative_branch() {
    let tokens = lex_line(
        "Put a +1/+1 counter on target creature or that creature gains banding, first strike, or trample.",
        0,
    )
    .expect("shared-target action choice should lex");

    let parsed = super::parse_or_action_clause_lexed(&tokens)
        .expect("shared-target action choice should parse")
        .expect("outer action choice should be recognized");
    let EffectAst::UnlessAction {
        effects,
        alternative,
        ..
    } = parsed
    else {
        panic!("expected a typed outer action choice");
    };

    let primary_target = effects
        .iter()
        .find_map(super::super::primary_target_from_effect)
        .expect("counter branch should declare the target");
    let alternative_target = alternative
        .iter()
        .find_map(super::super::primary_target_from_effect)
        .expect("keyword branch should reuse the declared target");
    assert_eq!(
        alternative_target, primary_target,
        "both alternatives must share one legal target slot"
    );
    let alternative_debug = format!("{alternative:#?}");
    assert!(
        !alternative_debug.contains("__it__") && !alternative_debug.contains("discarded_cost"),
        "the demonstrative must not remain ambient or bind to a cost object: {alternative_debug}"
    );
}

#[test]
fn quantified_opponent_subject_uses_typed_fanout() {
    let tokens = lex_line("Each opponent draws a card.", 0).expect("fanout should lex");
    let effects = parse_effect_chain_lexed(&tokens).expect("fanout should parse");
    let [EffectAst::ForEachOpponent { effects: nested }] = effects.as_slice() else {
        panic!("expected opponent fanout, got {effects:#?}");
    };
    assert!(matches!(
        nested.as_slice(),
        [EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::Draw { .. },
            ..
        })]
    ));
}

#[test]
fn quantified_player_subject_uses_typed_fanout() {
    let tokens = lex_line("Each player gains 1 life.", 0).expect("fanout should lex");
    let effects = parse_effect_chain_lexed(&tokens).expect("fanout should parse");
    let [EffectAst::ForEachPlayer { effects: nested }] = effects.as_slice() else {
        panic!("expected player fanout, got {effects:#?}");
    };
    assert!(matches!(
        nested.as_slice(),
        [EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::GainLife { .. },
            ..
        })]
    ));
}

#[test]
fn qualified_player_search_binds_both_library_owner_and_chooser() {
    let tokens = lex_line(
        "Each player who controls fewer lands than the player who controls the most lands searches their library for a number of basic land cards less than or equal to the difference, puts those cards onto the battlefield tapped, then shuffles.",
        0,
    )
    .expect("qualified player search should lex");
    let mut effects =
        parse_effect_chain_lexed(&tokens).expect("qualified player search should parse");
    let [effect] = effects.as_mut_slice() else {
        panic!("expected one typed search action, got {effects:#?}");
    };
    super::bind_implicit_player_context(effect, PlayerAst::That);
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action: SubjectVerbActionAst::SearchLibrary {
            chooser, player, ..
        },
        ..
    }) = effect
    else {
        panic!("expected typed search action, got {effect:#?}");
    };

    assert_eq!((*chooser, *player), (PlayerAst::That, PlayerAst::That));
}

#[test]
fn coordinated_that_player_search_carries_the_player_into_the_search() {
    let tokens = lex_line(
        "That player loses 3 life, searches their library for a card, puts it into their hand, then shuffles.",
        0,
    )
    .expect("coordinated player search should lex");
    let effects =
        parse_effect_chain_lexed(&tokens).expect("coordinated player search should parse");
    let [EffectAst::CommaThen { effects: nested }] = effects.as_slice() else {
        panic!("expected one typed comma-then sequence, got {effects:#?}");
    };
    let [
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            subject:
                SubjectVerbSubjectAst {
                    player: life_player,
                    ..
                },
            action: SubjectVerbActionAst::LoseLife { .. },
        }),
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::SearchLibrary {
                    chooser,
                    player: library_player,
                    ..
                },
            ..
        }),
    ] = nested.as_slice()
    else {
        panic!("expected life loss followed by one search procedure, got {nested:#?}");
    };

    assert_eq!(
        (*life_player, *chooser, *library_player),
        (PlayerAst::That, PlayerAst::That, PlayerAst::That)
    );
}

#[test]
fn imperative_search_does_not_inherit_a_leading_player() {
    let tokens = lex_line(
        "That player loses 3 life, then search your library for a card, put it into your hand, then shuffle.",
        0,
    )
    .expect("imperative search should lex");
    let effects = parse_effect_chain_lexed(&tokens).expect("imperative search should parse");
    let [EffectAst::CommaThen { effects: nested }] = effects.as_slice() else {
        panic!("expected one typed comma-then sequence, got {effects:#?}");
    };
    let Some(EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action:
            SubjectVerbActionAst::SearchLibrary {
                chooser,
                player: library_player,
                ..
            },
        ..
    })) = nested.last()
    else {
        panic!("expected a terminal search procedure, got {nested:#?}");
    };

    assert_eq!(
        (*chooser, *library_player),
        (PlayerAst::Implicit, PlayerAst::Implicit),
        "an imperative search remains an action by the ability controller"
    );
}

#[test]
fn quantified_player_across_zone_choice_stays_a_union() {
    let tokens = lex_line(
        "Each player exiles X permanents they control and/or cards from their hand.",
        0,
    )
    .expect("across-zone fanout should lex");
    let effects = parse_effect_chain_lexed(&tokens).expect("across-zone fanout should parse");
    let [EffectAst::ForEachPlayer { effects: nested }] = effects.as_slice() else {
        panic!("expected player fanout, got {effects:#?}");
    };
    let [
        EffectAst::ChooseObjectsAcrossZones {
            filter,
            count_value,
            zones,
            ..
        },
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::Exile { .. },
            ..
        }),
    ] = nested.as_slice()
    else {
        panic!("expected one coordinated across-zone exile choice, got {nested:#?}");
    };

    assert_eq!(count_value, &Some(Value::X));
    assert_eq!(
        zones,
        &[crate::zone::Zone::Hand, crate::zone::Zone::Battlefield]
    );
    assert_eq!(filter.any_of.len(), 2, "{filter:#?}");
    assert!(filter.any_of.iter().any(|arm| {
        arm.zone == Some(crate::zone::Zone::Hand)
            && arm.owner == Some(crate::target::PlayerFilter::IteratedPlayer)
            && arm.controller.is_none()
    }));
    assert!(filter.any_of.iter().any(|arm| {
        arm.zone == Some(crate::zone::Zone::Battlefield)
            && arm.controller == Some(crate::target::PlayerFilter::IteratedPlayer)
            && arm.owner.is_none()
    }));
}

#[test]
fn descent_into_madness_exact_body_does_not_collapse_the_zone_arms() {
    let tokens = lex_line(
        "Put a despair counter on this enchantment, then each player exiles X permanents they control and/or cards from their hand, where X is the number of despair counters on this enchantment.",
        0,
    )
    .expect("Descent into Madness body should lex");
    let effects = parse_effect_sentence_lexed(&tokens)
        .expect("Descent into Madness body should parse through sentence dispatch");
    let [EffectAst::CommaThen { effects: sequence }] = effects.as_slice() else {
        panic!("expected a typed comma-then Descent sequence, got {effects:#?}");
    };
    let [_, EffectAst::ForEachPlayer { effects: nested }] = sequence.as_slice() else {
        panic!("expected counter placement followed by player fanout, got {effects:#?}");
    };
    let [
        EffectAst::ChooseObjectsAcrossZones {
            filter,
            count_value: Some(count_value),
            zones,
            ..
        },
        _,
    ] = nested.as_slice()
    else {
        panic!("expected Descent's coordinated across-zone choice, got {nested:#?}");
    };

    assert_eq!(
        zones,
        &[crate::zone::Zone::Hand, crate::zone::Zone::Battlefield]
    );
    assert_eq!(filter.any_of.len(), 2, "{filter:#?}");
    assert!(matches!(
        count_value.unhinted(),
        Value::CountersOnSource(crate::object::CounterType::Named(name))
            | Value::CountersOn(_, Some(crate::object::CounterType::Named(name)))
            if *name == "despair"
    ));
    assert!(
        !filter.any_of.iter().any(|arm| {
            arm.zone == Some(crate::zone::Zone::Hand)
                && arm.controller == Some(crate::target::PlayerFilter::IteratedPlayer)
                && !arm.card_types.is_empty()
        }),
        "the hand and battlefield arms were intersected: {filter:#?}"
    );
}

#[test]
fn quantified_other_player_subject_uses_not_you_filter() {
    let tokens =
        lex_line("Each other player draws a card.", 0).expect("filtered fanout should lex");
    let effects = parse_effect_chain_lexed(&tokens).expect("filtered fanout should parse");
    let [
        EffectAst::ForEachPlayersFiltered {
            filter,
            effects: nested,
        },
    ] = effects.as_slice()
    else {
        panic!("expected filtered player fanout, got {effects:#?}");
    };
    assert_eq!(filter, &crate::target::PlayerFilter::NotYou);
    assert!(matches!(
        nested.as_slice(),
        [EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::Draw { .. },
            ..
        })]
    ));
}

#[test]
fn quantified_other_player_may_stays_inside_filtered_fanout() {
    let tokens = lex_line("Each other player may draw three cards.", 0)
        .expect("optional filtered fanout should lex");
    let effects = parse_effect_chain_lexed(&tokens).expect("optional filtered fanout should parse");
    let [
        EffectAst::ForEachPlayersFiltered {
            filter,
            effects: nested,
        },
    ] = effects.as_slice()
    else {
        panic!("expected filtered player fanout, got {effects:#?}");
    };
    assert_eq!(filter, &crate::target::PlayerFilter::NotYou);
    assert!(matches!(nested.as_slice(), [EffectAst::May { .. }]));
}

#[test]
fn quantified_shared_subject_chain_stays_in_one_fanout() {
    let tokens = lex_line("Each opponent draws a card and gains 2 life.", 0)
        .expect("shared-subject fanout should lex");
    let effects = parse_effect_chain_lexed(&tokens).expect("shared-subject fanout should parse");
    let [EffectAst::ForEachOpponent { effects: nested }] = effects.as_slice() else {
        panic!("expected one opponent fanout, got {effects:#?}");
    };
    let nested = match nested.as_slice() {
        [EffectAst::Coordinated { effects, .. }] => effects.as_slice(),
        effects => effects,
    };
    assert!(matches!(
        nested,
        [
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::Draw { .. },
                ..
            }),
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::GainLife { .. },
                ..
            })
        ]
    ));
}

#[test]
fn quantified_subject_tail_stays_nested_after_prior_actions() {
    let tokens = lex_line(
        "Copy that spell, you may choose new targets for the copy, and each opponent draws a card.",
        0,
    )
    .expect("nested fanout tail should lex");
    let effects = parse_effect_chain_lexed(&tokens).expect("nested fanout tail should parse");
    let action_effects = match effects.as_slice() {
        [EffectAst::Coordinated { effects, .. }] => effects.as_slice(),
        effects => effects,
    };
    assert!(
        action_effects.iter().any(
            |effect| matches!(effect, EffectAst::ForEachOpponent { effects } if matches!(
                effects.as_slice(),
                [EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action: SubjectVerbActionAst::Draw { .. },
                    ..
                })]
            ))
        ),
        "expected a typed opponent fanout after the copy actions, got {effects:#?}"
    );
}

#[test]
fn opportunistic_dragon_keeps_source_lifetime_target_effects_in_its_trigger() {
    let clause = lex_line(
        "For as long as this creature remains on the battlefield, gain control of that permanent, it loses all abilities, and it can't attack or block.",
        0,
    )
    .expect("source-lifetime clause should lex");
    let clause_effects = parse_effect_chain_lexed(&clause)
        .expect("source-lifetime clause should parse as a resolution chain");
    assert!(
        format!("{clause_effects:#?}").contains("ThisLeavesTheBattlefield"),
        "{clause_effects:#?}"
    );

    let def = CardDefinitionBuilder::new(CardId::new(), "Opportunistic Dragon")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Flying\nWhen this creature enters, choose target Human or artifact an opponent controls. For as long as this creature remains on the battlefield, gain control of that permanent, it loses all abilities, and it can't attack or block.",
        )
        .expect("Opportunistic Dragon source-lifetime trigger should parse");
    let debug = format!("{def:#?}");
    assert!(
        debug.contains("ChangeControllerToEffectController"),
        "{debug}"
    );
    assert!(debug.contains("ThisLeavesTheBattlefield"), "{debug}");
    assert!(debug.contains("RemoveAllAbilities"), "{debug}");
    assert!(
        debug.contains("BeBlocked") || debug.contains("Block"),
        "{debug}"
    );
    assert!(debug.contains("Attack"), "{debug}");
    assert!(
        !debug.contains("RemoveAllAbilitiesForFilter"),
        "targeted loss must not become a global static ability: {debug}"
    );
}

#[test]
fn wondrous_wasp_keeps_source_lifetime_ability_loss_on_the_tapped_target() {
    let def = CardDefinitionBuilder::new(CardId::new(), "The Wondrous Wasp")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Flash\nFlying\nWasp's Sting — When The Wondrous Wasp enters, tap up to one target creature. It loses all abilities for as long as The Wondrous Wasp remains on the battlefield.",
        )
        .expect("The Wondrous Wasp source-lifetime trigger should parse");
    let debug = format!("{def:#?}");
    assert!(debug.contains("TapEffect"), "{debug}");
    assert!(debug.contains("ThisLeavesTheBattlefield"), "{debug}");
    assert!(debug.contains("RemoveAllAbilities"), "{debug}");
    assert!(
        !debug.contains("RemoveAllAbilitiesForFilter"),
        "targeted loss must not become a global static ability: {debug}"
    );
}

#[test]
fn base_pt_where_x_full_chains_keep_duration_and_binding_together() {
    for (card, text, expected_value) in [
        (
            "Candlekeep Inspiration",
            "Until end of turn, creatures you control have base power and toughness X/X, where X is the number of cards you own in exile and in your graveyard that are instant cards, are sorcery cards, and/or have an Adventure.",
            "Adventure",
        ),
        (
            "Jolrael, Mwonvuli Recluse",
            "Until end of turn, creatures you control have base power and toughness X/X, where X is the number of cards in your hand.",
            "CardsInHand",
        ),
    ] {
        let tokens = lex_line(text, 0).expect("base-P/T where-X chain should lex");
        let effects = parse_effect_chain_lexed(&tokens)
            .unwrap_or_else(|error| panic!("{card} full effect chain should parse: {error}"));
        let [
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::SetBasePowerToughness {
                        power,
                        toughness,
                        duration,
                        ..
                    },
                ..
            }),
        ] = effects.as_slice()
        else {
            panic!("{card} should lower to one typed base-P/T effect: {effects:#?}");
        };

        assert_eq!(duration, &crate::effect::Until::EndOfTurn, "{effects:#?}");
        assert_eq!(power, toughness, "{effects:#?}");
        assert!(!matches!(power.unhinted(), Value::X), "{effects:#?}");
        assert!(
            format!("{power:#?}").contains(expected_value),
            "{card} should retain its authored where-X value: {effects:#?}"
        );
    }
}

#[test]
fn base_pt_where_x_full_cards_compile_candlekeep_and_jolrael() {
    let candlekeep = CardDefinitionBuilder::new(CardId::new(), "Candlekeep Inspiration")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Until end of turn, creatures you control have base power and toughness X/X, where X is the number of cards you own in exile and in your graveyard that are instant cards, are sorcery cards, and/or have an Adventure.",
        )
        .expect("Candlekeep Inspiration's complete rules text should compile");
    let candlekeep_debug = format!("{candlekeep:#?}");
    assert!(
        candlekeep_debug.contains("ApplyContinuousEffect")
            && candlekeep_debug.contains("resolve_set_pt_values_at_resolution: true")
            && candlekeep_debug.contains("Adventure")
            && candlekeep_debug.contains("Graveyard")
            && candlekeep_debug.contains("Exile"),
        "Candlekeep should keep its typed multi-zone where-X value: {candlekeep_debug}"
    );

    let jolrael = CardDefinitionBuilder::new(CardId::new(), "Jolrael, Mwonvuli Recluse")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Whenever you draw your second card each turn, create a 2/2 green Cat creature token.\n{4}{G}{G}: Until end of turn, creatures you control have base power and toughness X/X, where X is the number of cards in your hand.",
        )
        .expect("Jolrael, Mwonvuli Recluse's complete rules text should compile");
    let jolrael_debug = format!("{jolrael:#?}");
    assert!(
        jolrael_debug.contains("ApplyContinuousEffect")
            && jolrael_debug.contains("resolve_set_pt_values_at_resolution: true")
            && jolrael_debug.contains("Hand")
            && jolrael_debug.contains("WhereXIs"),
        "Jolrael should keep its typed cards-in-hand where-X value: {jolrael_debug}"
    );
}

#[test]
fn sacrifice_source_then_source_deals_damage_keeps_both_actions() {
    let tokens = lex_line(
        "Sacrifice this creature and it deals damage equal to the number of +1/+1 counters on it to each creature without flying and each player.",
        0,
    )
    .expect("source sacrifice and damage chain should lex");
    let segments = super::split_effect_chain_on_and_lexed(&tokens);
    assert_eq!(
        segments.len(),
        3,
        "the top-level action and paired damage operands should remain distinct: {segments:#?}"
    );
    assert!(
        segments[..2]
            .iter()
            .all(|segment| super::segment_has_effect_head_lexed(segment)),
        "the sacrifice and damage arms must be recognized as executable effects: {segments:#?}"
    );
    let sacrifice_arm =
        parse_effect_chain_lexed(segments[0]).expect("standalone sacrifice arm should parse");
    assert!(
        format!("{sacrifice_arm:#?}").contains("Sacrifice"),
        "the standalone first arm must remain a sacrifice: {sacrifice_arm:#?}"
    );
    let effects =
        parse_effect_chain_lexed(&tokens).expect("source sacrifice and damage chain should parse");
    let debug = format!("{effects:#?}");

    assert!(
        debug.contains("Sacrifice") && debug.contains("DealDamage"),
        "both coordinated actions must survive chain parsing: {debug}"
    );
}

#[test]
fn exile_source_and_anaphoric_objects_keeps_both_operands() {
    let tokens = lex_line("Exile this artifact and those creature cards.", 0)
        .expect("shared-verb exile operands should lex");
    let effects =
        parse_effect_chain_lexed(&tokens).expect("shared-verb exile operands should parse");
    let debug = format!("{effects:#?}");
    assert_eq!(
        debug.matches("Exile").count(),
        2,
        "both coordinated exile operands must survive chain parsing: {debug}"
    );
}

#[test]
fn comma_then_exile_source_and_anaphoric_objects_keeps_both_operands() {
    let tokens = lex_line(
        "This artifact deals 1 damage to any target, then exile this artifact and those creature cards.",
        0,
    )
    .expect("comma-then shared-verb exile operands should lex");
    let effects = parse_effect_chain_lexed(&tokens)
        .expect("comma-then shared-verb exile operands should parse");
    let debug = format!("{effects:#?}");

    assert!(
        debug.contains("CommaThen"),
        "the printed comma-then boundary must survive parsing: {debug}"
    );
    assert_eq!(
        debug.matches("Exile").count(),
        2,
        "both coordinated exile operands must survive the comma-then chain: {debug}"
    );
    assert!(
        debug.contains("plural_object_noun: true"),
        "the authored `those creature cards` result set must remain plural: {debug}"
    );
}
