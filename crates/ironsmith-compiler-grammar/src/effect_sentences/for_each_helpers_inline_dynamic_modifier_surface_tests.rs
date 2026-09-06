use crate::cards::builders::ForEachEffectAst;
use crate::cards::builders::CharacteristicActionAst;
use super::*;
use crate::lexer::lex_line;

#[test]
fn resolving_gets_for_each_count_keeps_authored_surface() {
    let tokens = lex_line("for each permanent card in your graveyard", 0)
        .expect("for-each count should lex");
    let count = parse_get_for_each_count_value(&tokens)
        .expect("for-each count should parse")
        .expect("for-each count should match");

    assert!(count.has_surface_hint(ValueSurfaceHint::ForEach));
    assert!(matches!(count.unhinted(), Value::Count(_)));
}

#[test]
fn each_player_choice_keeps_comma_separated_negative_modifiers_in_one_filter() {
    let tokens = lex_line(
        "Each player chooses two nontoken, non-Vehicle creatures they control.",
        0,
    )
    .expect("participant choice should lex");
    let effect = parse_for_each_player_clause(&tokens)
        .expect("participant choice should parse")
        .expect("participant choice should match");
    let EffectAst::ForEach(ForEachEffectAst::ForEachPlayer { effects }) = effect else {
        panic!("expected a player loop, got {effect:#?}");
    };
    let [EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects { filter, count, .. })] = effects.as_slice() else {
        panic!("expected one object choice, got {effects:#?}");
    };

    assert_eq!(*count, ChoiceCount::exactly(2));
    assert!(filter.nontoken, "{filter:#?}");
    assert_eq!(filter.card_types, [crate::types::CardType::Creature]);
    assert_eq!(filter.excluded_subtypes, [crate::types::Subtype::Vehicle]);
    assert_eq!(filter.controller, Some(PlayerFilter::IteratedPlayer));
}

#[test]
fn resolving_gets_for_each_count_keeps_serial_relative_subtype_surface() {
    let tokens = lex_line(
        "for each creature you control that's an Insect, Rat, Spider, or Squirrel",
        0,
    )
    .expect("serial for-each count should lex");
    let count = parse_get_for_each_count_value(&tokens)
        .expect("serial for-each count should parse")
        .expect("serial for-each count should match");
    let Value::Count(filter) = count.unhinted() else {
        panic!("expected an object count, got {count:#?}");
    };

    assert!(filter.has_serial_or_list_surface(), "{filter:#?}");
    assert_eq!(
        filter.description(),
        "a creature you control that's an Insect, Rat, Spider, or Squirrel"
    );
}

#[test]
fn exact_surface_reparse_preserves_specialized_cast_time_count_tag() {
    let tokens = lex_line(
        "for each modified creature you controlled as you cast this spell",
        0,
    )
    .expect("cast-time for-each count should lex");
    let count = parse_get_for_each_count_value(&tokens)
        .expect("cast-time for-each count should parse")
        .expect("cast-time for-each count should match");
    let Value::Count(filter) = count.unhinted() else {
        panic!("expected an object count, got {count:#?}");
    };

    assert!(matches!(
        filter.tagged_constraints.as_slice(),
        [constraint]
            if constraint.tag.as_str() == crate::tag::CompilerReferenceTag::CastModifiedCreatures.as_str()
                && constraint.relation
                    == crate::target::TaggedOpbjectRelation::IsTaggedObject
    ));
    assert!(filter.card_types.is_empty(), "{filter:#?}");
}

#[test]
fn targeted_discard_for_each_suffix_does_not_claim_target_player_iteration() {
    for text in [
        "Target player discards a card for each Swamp you control.",
        "Target player discards a card for each charge counter on this artifact.",
        "Target player discards a card for each Swamp returned this way.",
    ] {
        let tokens = lex_line(text, 0).expect("targeted discard clause should lex");
        let parsed = parse_for_each_target_players_clause(&tokens)
            .expect("ambiguous target-player shape should yield cleanly");

        assert!(
            parsed.is_none(),
            "discard count suffix must not become a target-player iterator: {parsed:#?}"
        );
    }
}

#[test]
fn full_sentence_dispatch_preserves_bounded_target_player_iteration() {
    let tokens = lex_line("Two target players each draw a card.", 0)
        .expect("bounded target-player fanout should lex");
    let effects = crate::effect_sentences::parse_effect_sentence_lexed(&tokens)
        .expect("bounded target-player fanout should parse");

    assert!(
        matches!(
            effects.as_slice(),
            [EffectAst::ForEach(ForEachEffectAst::ForEachTargetPlayers { count, .. })]
                if *count == ChoiceCount::exactly(2)
        ),
        "{effects:#?}"
    );
}

#[test]
fn targeted_life_gain_for_each_suffix_does_not_claim_target_player_iteration() {
    let tokens = lex_line(
        "Target player gains 2 life for each creature on the battlefield.",
        0,
    )
    .expect("Congregate life-gain clause should lex");
    let parsed = parse_for_each_target_players_clause(&tokens)
        .expect("ambiguous target-player life-gain shape should yield cleanly");

    assert!(
        parsed.is_none(),
        "life-gain count suffix must not become a target-player iterator: {parsed:#?}"
    );
}

#[test]
fn targeted_life_loss_for_each_suffix_does_not_claim_target_player_iteration() {
    let tokens = lex_line(
        "Target player loses 2 life plus 2 life for each Spirit sacrificed this way.",
        0,
    )
    .expect("Devouring Greed life-loss clause should lex");
    let parsed = parse_for_each_target_players_clause(&tokens)
        .expect("ambiguous target-player life-loss shape should yield cleanly");

    assert!(
        parsed.is_none(),
        "life-loss count suffix must not become a target-player iterator: {parsed:#?}"
    );
}

#[test]
fn candlekeep_shape_binds_where_x_to_an_owned_multi_zone_count() {
    let tokens = lex_line(
            "Until end of turn, creatures you control have base power and toughness X/X, where X is the number of cards you own in exile and in your graveyard that are instant cards, are sorcery cards, and/or have an Adventure.",
            0,
        )
        .expect("Candlekeep-style base-P/T clause should lex");
    let effect = parse_has_base_power_toughness_clause(&tokens)
        .expect("Candlekeep-style base-P/T clause should parse")
        .expect("Candlekeep-style base-P/T clause should be recognized");
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action:
            SubjectVerbActionAst::Characteristics(CharacteristicActionAst::SetBasePowerToughness {
                power,
                toughness,
                target,
                duration,
                ..
            }),
        ..
    }) = effect
    else {
        panic!("expected typed base-P/T effect, got {effect:#?}");
    };

    assert_eq!(duration, Until::EndOfTurn);
    assert_eq!(power, toughness);
    assert!(power.has_surface_hint(ValueSurfaceHint::WhereXIs));
    let Value::Count(filter) = power.unhinted() else {
        panic!("expected a counted object-filter basis, got {power:#?}");
    };
    assert_eq!(filter.owner, Some(PlayerFilter::You));
    assert_eq!(
        filter.card_types,
        vec![
            crate::types::CardType::Instant,
            crate::types::CardType::Sorcery
        ]
    );
    assert_eq!(filter.subtypes, vec![crate::types::Subtype::Adventure]);
    assert!(filter.type_or_subtype_union);
    assert_eq!(filter.any_of.len(), 2);
    assert!(
        filter
            .any_of
            .iter()
            .any(|branch| branch.zone == Some(crate::zone::Zone::Exile))
    );
    assert!(
        filter
            .any_of
            .iter()
            .any(|branch| { branch.zone == Some(crate::zone::Zone::Graveyard) })
    );
    assert!(matches!(target, TargetAst::Object(_, _, _)));
}

#[test]
fn jolrael_shape_binds_where_x_to_cards_in_hand() {
    let tokens = lex_line(
            "Until end of turn, creatures you control have base power and toughness X/X, where X is the number of cards in your hand.",
            0,
        )
        .expect("Jolrael-style base-P/T clause should lex");
    let effect = parse_has_base_power_toughness_clause(&tokens)
        .expect("Jolrael-style base-P/T clause should parse")
        .expect("Jolrael-style base-P/T clause should be recognized");
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action:
            SubjectVerbActionAst::Characteristics(CharacteristicActionAst::SetBasePowerToughness {
                power,
                toughness,
                target,
                duration,
                ..
            }),
        ..
    }) = effect
    else {
        panic!("expected typed base-P/T effect, got {effect:#?}");
    };

    assert_eq!(duration, Until::EndOfTurn);
    assert_eq!(power, toughness);
    assert!(power.has_surface_hint(ValueSurfaceHint::WhereXIs));
    assert!(matches!(
        power.unhinted(),
        Value::CardsInHand(PlayerFilter::You)
    ));
    let TargetAst::Object(filter, _, _) = target else {
        panic!("expected a creature-filter target, got {target:#?}");
    };
    assert_eq!(filter.card_types, vec![crate::types::CardType::Creature]);
    assert_eq!(filter.controller, Some(PlayerFilter::You));
}
