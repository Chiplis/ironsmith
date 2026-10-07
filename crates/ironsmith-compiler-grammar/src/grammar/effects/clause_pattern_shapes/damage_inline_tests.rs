use super::*;
use crate::lexer::lex_line;

fn lex(text: &str) -> Vec<OwnedLexToken> {
    lex_line(text, 0).unwrap()
}

#[test]
fn parses_prevent_next_damage_shape() {
    let tokens = lex(
        "Prevent the next 3 damage that would be dealt to you and permanents you control this turn by a source of your choice.",
    );
    let shape = parse_prevent_next_damage_tokens(&tokens).expect("shape");
    assert!(shape.source_of_your_choice);
    assert!(shape.protects_you_and_permanents_you_control);
}

// Authored for the source-only campaign; execution is deferred.
#[test]
fn finite_prevention_retains_combat_kind_and_both_turn_positions() {
    for text in [
        "Prevent the next 1 combat damage that would be dealt to you this turn.",
        "Prevent the next 4 combat damage that would be dealt this turn to target creature you control.",
    ] {
        let tokens = lex(text);
        let shape = parse_prevent_next_damage_tokens(&tokens).expect("complete finite shield");
        assert!(shape.combat_only);
        assert!(!shape.source_of_your_choice);
    }
    let tokens = lex("Prevent the next 4 damage that would be dealt this turn to target creature you control.");
    let shape = parse_prevent_next_damage_tokens(&tokens).unwrap();
    assert!(!shape.combat_only);
    assert_eq!(crate::lexer::token_word_refs(shape.target_tokens),
        ["target", "creature", "you", "control"]);
}

#[test]
fn finite_shield_shape_rejects_unowned_durations_and_trailing_instructions() {
    for text in [
        "Prevent the next 1 combat damage that would be dealt to you until your next turn.",
        "Prevent the next 1 combat damage that would be dealt to you this turn unless you pay 1 life.",
        "Prevent the next 1 combat damage that would be dealt to you this turn. Draw a card.",
        "Prevent the next 1 combat damage that would be dealt by this artifact this turn.",
    ] {
        assert!(parse_prevent_next_damage_tokens(&lex(text)).is_none(), "{text}");
    }
}

#[test]
fn parses_passive_next_damage_destroy_replacement() {
    let tokens = lex(
        "The next time damage would be dealt to target creature this turn, destroy that creature instead.",
    );
    let shape = parse_replace_next_damage_with_destroy_tokens(&tokens).expect("shape");
    assert_eq!(
        shape.destroyed_reference,
        DestroyDamageTargetReference::Creature
    );
    assert_eq!(
        crate::lexer::token_word_refs(shape.target_tokens),
        ["target", "creature"]
    );
}

#[test]
fn parses_source_controller_redirect_without_raw_text() {
    let tokens = lex(
        "All damage that would be dealt this turn by target spell is dealt to that spell's controller instead.",
    );
    assert!(matches!(
        parse_redirect_next_damage_tokens(&tokens),
        Some(RedirectNextDamageShape::AllBySourceToSourceController { .. })
    ));
}

#[test]
fn parses_next_time_redirect_target() {
    let tokens = lex(
        "The next time a red source would deal damage to target creature this turn, that damage is dealt to target player instead.",
    );
    assert!(matches!(
        parse_redirect_next_damage_tokens(&tokens),
        Some(RedirectNextDamageShape::NextTime {
            destination: RedirectDamageDestinationShape::Target(_),
            ..
        })
    ));
}

#[test]
fn parses_source_object_and_chosen_destination_redirect_shapes() {
    let next_time = lex(
        "The next time a source of your choice would deal damage to target creature this turn, that damage is dealt to this creature instead.",
    );
    assert!(matches!(
        parse_redirect_next_damage_tokens(&next_time),
        Some(RedirectNextDamageShape::NextTime {
            destination: RedirectDamageDestinationShape::SourceObject,
            ..
        })
    ));
    let all_damage = lex(
        "All damage that would be dealt to target creature this turn by a source of your choice is dealt to this creature instead.",
    );
    assert!(matches!(
        parse_redirect_next_damage_tokens(&all_damage),
        Some(RedirectNextDamageShape::AllToTargetByChosenSource {
            destination: RedirectDamageDestinationShape::SourceObject,
            ..
        })
    ));
    let chosen_destination = lex(
        "The next time a source of your choice would deal damage to you this turn, that damage is dealt to target creature of an opponent's choice instead.",
    );
    assert!(matches!(
        parse_redirect_next_damage_tokens(&chosen_destination),
        Some(RedirectNextDamageShape::NextTime {
            destination: RedirectDamageDestinationShape::TargetOfChoice(_),
            ..
        })
    ));
}

#[test]
fn scoped_all_damage_retains_recipient_source_combat_and_exact_duration() {
    for (text, combat, next_turn, source) in [
        ("All combat damage that would be dealt to you this turn by target unblocked creature is dealt to its controller instead.", true, false, true),
        ("All damage that would be dealt to target creature you control this turn is dealt to you instead.", false, false, false),
        ("Until your next turn, all damage that would be dealt to creatures you control is dealt to that creature instead.", false, true, false),
        ("All damage that would be dealt to you this turn by target attacking creature is dealt to this creature instead.", false, false, true),
    ] {
        let tokens = crate::lexer::lex_line(text, 0).unwrap();
        let Some(RedirectNextDamageShape::ScopedAll(shape)) = parse_redirect_next_damage_tokens(&tokens) else { panic!("{text}"); };
        assert_eq!(shape.combat_only, combat); assert_eq!(shape.source.is_some(), source);
        assert_eq!(shape.mode == ironsmith_core::ReplacementApplyMode::UntilYourNextTurn, next_turn);
    }
}

#[test]
fn all_damage_reader_does_not_eat_a_shared_next_amount_or_trailing_instruction() {
    for text in [
        "All damage that would be dealt to you this turn is dealt to target creature instead. Draw a card.",
        "Until your next turn, all damage that would be dealt to you this turn is dealt to target creature instead.",
        "All damage that would be dealt to you is dealt to target creature instead.",
    ] {
        let tokens = crate::lexer::lex_line(text, 0).unwrap();
        assert!(parse_redirect_next_damage_tokens(&tokens).is_none(), "{text}");
    }
}

#[test]
fn bounded_redirection_keeps_passive_forms_combat_pronouns_and_duration_position() {
    for (text, combat) in [
        ("The next time damage would be dealt to this creature and/or you this turn, that damage is dealt to any target instead.", false),
        ("The next time it would deal combat damage this turn, it deals that damage to you instead.", true),
        ("The next time target attacking creature would deal combat damage to this creature this turn, that creature deals that damage to itself instead.", true),
        ("The next time this creature would deal combat damage to an opponent this turn, it deals that damage to target creature instead.", true),
        ("The next time an instant or sorcery spell would deal damage to you this turn, that spell deals that damage to its controller instead.", false),
    ] {
        let tokens = lex(text);
        let Some(RedirectNextDamageShape::NextTime { combat_only, .. }) = parse_redirect_next_damage_tokens(&tokens) else { panic!("{text}") };
        assert_eq!(combat_only, combat);
    }
    for text in [
        "The next X damage that would be dealt this turn to target white creature you control is dealt to this creature instead.",
        "The next X damage that would be dealt to target white creature you control this turn is dealt to this creature instead.",
    ] {
        assert!(matches!(parse_redirect_next_damage_tokens(&lex(text)), Some(RedirectNextDamageShape::NextAmount { protected_tokens: Some(_), destination: RedirectDamageDestinationShape::SourceObject, .. })));
    }
    for text in [
        "The next time damage would be dealt to this creature this turn, that damage is dealt to any target instead. Draw a card.",
        "The next time damage would be dealt to this creature, that damage is dealt to any target instead.",
        "The next time this creature would deal damage to you this turn, you may deal that damage to target creature instead.",
    ] { assert!(parse_redirect_next_damage_tokens(&lex(text)).is_none(), "{text}"); }
}
