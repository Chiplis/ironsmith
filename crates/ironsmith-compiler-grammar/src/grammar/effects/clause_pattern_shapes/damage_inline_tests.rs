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
