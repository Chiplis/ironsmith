use super::*;

#[test]
fn look_top_then_exile_one_uses_captured_count_owner_and_followup() {
    let tokens = crate::runtime_backend::lex_line(
        "Look at the top three cards of your library, then exile one of those cards.",
        0,
    )
    .expect("look-top exile-one text should lex");

    let effects = parse_look_at_top_then_exile_one_sentence(&tokens)
        .expect("look-top exile-one parser should not error")
        .expect("look-top exile-one parser should match");
    let debug = format!("{effects:#?}");

    assert!(debug.contains("LookAtTopCards"), "{debug}");
    assert!(debug.contains("Fixed") && debug.contains("3"), "{debug}");
    assert!(debug.contains("ChooseObjects"), "{debug}");
    assert!(debug.contains("Exile"), "{debug}");
}

#[test]
fn exile_then_return_same_object_uses_captured_clauses_and_counter_followup() {
    let tokens = crate::runtime_backend::lex_line(
        "You may exile target artifact or creature, then return it to the battlefield under its owner's control with a +1/+1 counter on it.",
        0,
    )
    .expect("exile-return text should lex");

    let effects = parse_exile_then_return_same_object_sentence(&tokens)
        .expect("exile-return parser should not error")
        .expect("exile-return parser should match");
    let debug = format!("{effects:#?}");

    assert!(debug.contains("Exile"), "{debug}");
    assert!(debug.contains("ReturnToBattlefield"), "{debug}");
    assert!(debug.contains("PutCounters"), "{debug}");
    assert!(debug.contains("PlusOnePlusOne"), "{debug}");
}

#[test]
fn exile_then_face_down_return_keeps_the_same_object_link() {
    let tokens = crate::runtime_backend::lex_line(
        "Exile this creature, then return it to the battlefield face down under its owner's control.",
        0,
    )
    .expect("face-down exile-return text should lex");

    let effects = parse_exile_then_return_same_object_sentence(&tokens)
        .expect("face-down exile-return parser should not error")
        .expect("face-down exile-return parser should match");
    assert_eq!(effects.len(), 2, "{effects:#?}");
    let debug = format!("{effects:#?}");

    assert!(debug.contains("Exile"), "{debug}");
    assert!(debug.contains("zone: Battlefield"), "{debug}");
    assert!(debug.contains("battlefield_face_down: true"), "{debug}");
    assert!(debug.contains("Tagged"), "{debug}");
}

#[test]
fn token_end_of_combat_recognizer_uses_captured_verb_object_and_timing() {
    let exile_tokens = crate::runtime_backend::lex_line("Exile those tokens at end of combat.", 0)
        .expect("exile token end-combat text should lex");
    assert!(is_exile_that_token_at_end_of_combat(&exile_tokens));
    assert!(!is_sacrifice_that_token_at_end_of_combat(&exile_tokens));

    let sacrifice_tokens =
        crate::runtime_backend::lex_line("Sacrifice it at the end of combat.", 0)
            .expect("sacrifice token end-combat text should lex");
    assert!(is_sacrifice_that_token_at_end_of_combat_lexed(
        &sacrifice_tokens
    ));
    assert!(!is_exile_that_token_at_end_of_combat_lexed(
        &sacrifice_tokens
    ));
}

#[test]
fn extra_turn_parser_uses_captured_subject_action_and_anchor() {
    let you_tokens = crate::runtime_backend::lex_line("Take an extra turn after this one.", 0)
        .expect("you extra-turn text should lex");
    let chosen_tokens = crate::runtime_backend::lex_line(
        "The chosen player takes an extra turn after this one.",
        0,
    )
    .expect("chosen-player extra-turn text should lex");
    let that_tokens =
        crate::runtime_backend::lex_line("After that turn, that player takes an extra turn.", 0)
            .expect("that-player referenced-turn text should lex");

    let you_effect = parse_take_extra_turn_sentence(&you_tokens)
        .expect("you extra-turn parser should not error")
        .expect("you extra-turn parser should match");
    let chosen_effect = parse_take_extra_turn_sentence(&chosen_tokens)
        .expect("chosen extra-turn parser should not error")
        .expect("chosen extra-turn parser should match");
    let that_effect = parse_take_extra_turn_sentence(&that_tokens)
        .expect("that extra-turn parser should not error")
        .expect("that extra-turn parser should match");
    let debug = format!("{you_effect:#?}\n{chosen_effect:#?}\n{that_effect:#?}");

    assert!(debug.contains("ExtraTurnAfterTurn"), "{debug}");
    assert!(debug.contains("You"), "{debug}");
    assert!(debug.contains("Chosen"), "{debug}");
    assert!(debug.contains("That"), "{debug}");
    assert!(debug.contains("CurrentTurn"), "{debug}");
    assert!(debug.contains("ReferencedTurn"), "{debug}");
}

#[test]
fn additional_phase_parser_uses_captured_count_and_tail() {
    let one_combat_tokens = crate::runtime_backend::lex_line(
        "After this phase, there is an additional combat phase.",
        0,
    )
    .expect("single additional combat text should lex");
    let two_combat_tokens = crate::runtime_backend::lex_line(
        "After this main phase, there are two additional combat phases.",
        0,
    )
    .expect("two additional combats text should lex");
    let combat_main_tokens = crate::runtime_backend::lex_line(
        "After this main phase, there is an additional combat phase followed by an additional main phase.",
        0,
    )
    .expect("combat-then-main text should lex");

    let one_combat = parse_additional_phase_sentence(&one_combat_tokens)
        .expect("single additional combat parser should match");
    let two_combat = parse_additional_phase_sentence(&two_combat_tokens)
        .expect("two additional combats parser should match");
    let combat_main = parse_additional_phase_sentence(&combat_main_tokens)
        .expect("combat-then-main parser should match");
    let debug = format!("{one_combat:#?}\n{two_combat:#?}\n{combat_main:#?}");

    assert!(debug.contains("AdditionalPhases"), "{debug}");
    assert!(debug.contains("Combat"), "{debug}");
    assert!(debug.contains("Main"), "{debug}");
    assert_eq!(debug.matches("Combat").count(), 4, "{debug}");
    assert_eq!(debug.matches("Main").count(), 1, "{debug}");
}

#[test]
fn look_at_hand_parser_uses_captured_player_and_followup() {
    let target_player_tokens = crate::runtime_backend::lex_line("Look at target player's hand.", 0)
        .expect("target-player hand text should lex");
    let opponent_choose_tokens = crate::runtime_backend::lex_line(
        "Look at an opponent's hand, then choose any card name.",
        0,
    )
    .expect("opponent choose-name hand text should lex");
    let iterated_tokens = crate::runtime_backend::lex_line("Look at that player's hand.", 0)
        .expect("iterated-player hand text should lex");

    let target_player = parse_look_at_hand_sentence(&target_player_tokens)
        .expect("target-player hand parser should not error")
        .expect("target-player hand parser should match");
    let opponent_choose = parse_look_at_hand_sentence(&opponent_choose_tokens)
        .expect("opponent choose-name hand parser should not error")
        .expect("opponent choose-name hand parser should match");
    let iterated = parse_look_at_hand_sentence(&iterated_tokens)
        .expect("iterated-player hand parser should not error")
        .expect("iterated-player hand parser should match");
    let debug = format!("{target_player:#?}\n{opponent_choose:#?}\n{iterated:#?}");

    assert!(debug.contains("LookAtHand"), "{debug}");
    assert!(debug.contains("Target") && debug.contains("Any"), "{debug}");
    assert!(debug.contains("Opponent"), "{debug}");
    assert!(debug.contains("ChooseCardName"), "{debug}");
    assert!(debug.contains("IteratedPlayer"), "{debug}");
}

#[test]
fn voted_with_you_scry_parser_uses_captured_count() {
    let tokens = crate::runtime_backend::lex_line(
        "You and each opponent who voted for a choice you voted for may scry 2.",
        0,
    )
    .expect("voted-with-you scry text should lex");

    let effects = parse_you_and_each_opponent_voted_with_you_sentence(&tokens)
        .expect("voted-with-you scry parser should not error")
        .expect("voted-with-you scry parser should match");
    let debug = format!("{effects:#?}");

    assert!(debug.contains("May"), "{debug}");
    assert!(debug.contains("Scry"), "{debug}");
    assert!(debug.contains("Fixed") && debug.contains("2"), "{debug}");
    assert!(debug.contains("ForEachTaggedPlayer"), "{debug}");
    assert!(debug.contains("voted_with_you"), "{debug}");
}

#[test]
fn for_each_counter_removed_uses_captured_subject_action_and_modifier() {
    let tokens = crate::runtime_backend::lex_line(
        "For each counter removed this way, this creature gets +1/+0 until end of turn.",
        0,
    )
    .expect("counter-removed pump text should lex");

    let effect = parse_for_each_counter_removed_sentence(&tokens)
        .expect("counter-removed parser should not error")
        .expect("counter-removed parser should match");
    let debug = format!("{effect:#?}");

    assert!(debug.contains("PumpByLastEffect"), "{debug}");
    assert!(debug.contains("power: 1"), "{debug}");
    assert!(debug.contains("toughness: 0"), "{debug}");
    assert!(debug.contains("Source"), "{debug}");
}

#[test]
fn counter_removed_result_shapes_route_before_generic_object_iteration() {
    let pump_tokens = crate::runtime_backend::lex_line(
        "For each counter removed this way, this creature gets +1/+0 until end of turn.",
        0,
    )
    .expect("counter-removed pump text should lex");
    let pump = parse_effect_sentence_lexed(&pump_tokens)
        .expect("counter-removed pump should dispatch through its typed shape");
    let pump_debug = format!("{pump:#?}");
    assert!(pump_debug.contains("PumpByLastEffect"), "{pump_debug}");

    let grouped_tokens = crate::runtime_backend::lex_line(
        "For each five counters removed this way, take an extra turn after this one.",
        0,
    )
    .expect("counter-group extra-turn text should lex");
    let grouped = parse_effect_sentence_lexed(&grouped_tokens)
        .expect("counter-group extra turn should dispatch through its typed shape");
    let grouped_debug = format!("{grouped:#?}");
    assert!(grouped_debug.contains("RepeatEffects"), "{grouped_debug}");
    assert!(
        grouped_debug.contains("DividedRoundedDown"),
        "{grouped_debug}"
    );
    assert!(grouped_debug.contains("ExtraTurn"), "{grouped_debug}");
}

#[test]
fn typed_for_each_result_shapes_route_before_generic_object_iteration() {
    let prevention_tokens = crate::runtime_backend::lex_line(
        "For each attacking red creature, prevent all combat damage that would be dealt by that creature this turn unless its controller pays {2}{R}.",
        0,
    )
    .expect("per-attacker prevention text should lex");
    let prevention = parse_effect_sentence_lexed(&prevention_tokens)
        .expect("per-attacker prevention should use its typed for-each shape");
    let prevention_debug = format!("{prevention:#?}");
    assert!(
        prevention_debug.contains("ForEachObject"),
        "{prevention_debug}"
    );
    assert!(
        prevention_debug.contains("attacking: true"),
        "{prevention_debug}"
    );
    assert!(
        prevention_debug.contains("UnlessPays"),
        "{prevention_debug}"
    );
    assert!(
        prevention_debug.contains("ItsController"),
        "{prevention_debug}"
    );

    let graveyard_tokens = crate::runtime_backend::lex_line(
        "For each permanent put into a graveyard this way, its controller creates a 3/3 green Elephant creature token.",
        0,
    )
    .expect("graveyard-result text should lex");
    let graveyard = parse_effect_sentence_lexed(&graveyard_tokens)
        .expect("graveyard result should use its typed tagged-result shape");
    let graveyard_debug = format!("{graveyard:#?}");
    assert!(
        graveyard_debug.contains("ForEachTagged"),
        "{graveyard_debug}"
    );
    assert!(graveyard_debug.contains("CreateToken"), "{graveyard_debug}");
}

#[test]
fn destroy_all_split_uses_captured_verb_and_object_tail() {
    let tokens = crate::runtime_backend::lex_line("Destroy all artifacts and enchantments.", 0)
        .expect("destroy-all split text should lex");

    let effects = parse_destroy_or_exile_all_split_sentence(&tokens)
        .expect("destroy-all split parser should not error")
        .expect("destroy-all split parser should match");
    let debug = format!("{effects:#?}");

    assert_eq!(effects.len(), 1, "{debug}");
    assert!(debug.contains("DestroyAll"), "{debug}");
    assert!(debug.contains("any_of"), "{debug}");
    assert!(debug.contains("Artifact"), "{debug}");
    assert!(debug.contains("Enchantment"), "{debug}");
}

#[test]
fn repeated_all_or_branches_remain_a_resolution_choice() {
    let tokens = crate::runtime_backend::lex_line(
        "Destroy all lands or all creatures.",
        0,
    )
    .expect("destroy-all alternative text should lex");

    let effects = parse_destroy_or_exile_all_split_sentence(&tokens)
        .expect("destroy-all alternative parser should not error")
        .expect("destroy-all alternative parser should match");
    let [EffectAst::ChooseOneOf { modes }] = effects.as_slice() else {
        panic!("expected one typed choice, got {effects:#?}");
    };
    assert_eq!(modes.len(), 2, "{modes:#?}");

    let expected = [CardType::Land, CardType::Creature];
    for (mode, expected_type) in modes.iter().zip(expected) {
        let [
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::DestroyAll { filter, .. },
                ..
            }),
        ] = mode.effects.as_slice()
        else {
            panic!("expected one destroy-all effect per mode, got {mode:#?}");
        };
        assert_eq!(filter.card_types, vec![expected_type], "{filter:#?}");
    }
}

#[test]
fn destroy_all_split_preserves_branch_scoped_collection_surface() {
    let tokens = crate::runtime_backend::lex_line(
        "Destroy all other enchantments you control, all other Auras attached to permanents you control, and all other Auras attached to attacking creatures your opponents control.",
        0,
    )
    .expect("branch-scoped destroy-all text should lex");

    let effects = parse_destroy_or_exile_all_split_sentence(&tokens)
        .expect("branch-scoped destroy-all parser should not error")
        .expect("branch-scoped destroy-all parser should match");
    let [
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::DestroyAll { filter, .. },
            ..
        }),
    ] = effects.as_slice()
    else {
        panic!("expected one destroy-all effect, got {effects:#?}");
    };

    assert_eq!(filter.any_of.len(), 3, "{filter:#?}");
    assert!(filter.has_conjunctive_set_surface(), "{filter:#?}");
    assert!(
        filter.any_of.iter().all(|branch| branch.other),
        "{filter:#?}"
    );

    let then_tokens = crate::runtime_backend::lex_line(
        "Then destroy all other enchantments you control, all other Auras attached to permanents you control, and all other Auras attached to attacking creatures your opponents control.",
        0,
    )
    .expect("leading-then branch-scoped destroy-all text should lex");
    let full_effects = parse_effect_sentence_lexed(&then_tokens)
        .expect("full branch-scoped destroy-all sentence should parse");
    let [
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::DestroyAll {
                    filter: full_filter,
                    ..
                },
            ..
        }),
    ] = full_effects.as_slice()
    else {
        panic!("expected one full-sentence destroy-all effect, got {full_effects:#?}");
    };
    assert_eq!(full_filter.any_of.len(), 3, "{full_filter:#?}");
    assert!(
        full_filter.has_conjunctive_set_surface(),
        "{full_filter:#?}"
    );
}

#[test]
fn destroy_all_split_exclusions_use_clause_and_zone_captures() {
    let except_tokens =
        crate::runtime_backend::lex_line("Destroy all creatures except Elves and Goblins.", 0)
            .expect("except split text should lex");
    let temporary_exile_tokens = crate::runtime_backend::lex_line(
        "Exile all creatures and planeswalkers until this enchantment leaves the battlefield.",
        0,
    )
    .expect("temporary exile split text should lex");
    let multi_zone_tokens = crate::runtime_backend::lex_line(
        "Exile all cards from target player's graveyard and hand.",
        0,
    )
    .expect("multi-zone exile text should lex");

    assert!(
        parse_destroy_or_exile_all_split_sentence(&except_tokens)
            .expect("except split parser should not error")
            .is_none()
    );
    assert!(
        parse_destroy_or_exile_all_split_sentence(&temporary_exile_tokens)
            .expect("temporary exile parser should not error")
            .is_none()
    );
    assert!(
        parse_destroy_or_exile_all_split_sentence(&multi_zone_tokens)
            .expect("multi-zone exile parser should not error")
            .is_none()
    );
}

#[test]
fn monstrosity_uses_captured_amount() {
    let tokens =
        crate::runtime_backend::lex_line("Monstrosity 3.", 0).expect("monstrosity text should lex");

    let effect = parse_monstrosity_sentence(&tokens)
        .expect("monstrosity parser should not error")
        .expect("monstrosity parser should match");
    let debug = format!("{effect:#?}");

    assert!(debug.contains("Monstrosity"), "{debug}");
    assert!(debug.contains("Fixed") && debug.contains("3"), "{debug}");
}

#[test]
fn exile_up_to_one_each_target_type_uses_captured_target_clauses() {
    let tokens = crate::runtime_backend::lex_line(
        "Exile up to one target artifact, up to one target creature, and up to one target enchantment.",
        0,
    )
    .expect("exile repeated target type text should lex");

    let effects = parse_exile_up_to_one_each_target_type_sentence(&tokens)
        .expect("exile repeated target type parser should not error")
        .expect("exile repeated target type parser should match");
    let debug = format!("{effects:#?}");

    assert_eq!(effects.len(), 4, "{debug}");
    assert_eq!(debug.matches("ChooseObjects").count(), 3, "{debug}");
    assert!(debug.contains("Artifact"), "{debug}");
    assert!(debug.contains("Creature"), "{debug}");
    assert!(debug.contains("Enchantment"), "{debug}");
    assert!(debug.contains("Exile"), "{debug}");
}
