use super::*;
use crate::lexer::lex_line;

#[test]
fn parses_counted_activated_or_triggered_ability_target() {
    let tokens = lex_line(
        "up to two target activated or triggered abilities you don't control",
        0,
    )
    .unwrap();
    let shape = parse_counter_ability_target_tokens(&tokens).expect("shape");
    assert!(shape.explicit_target);
    assert!(shape.target_count.is_some());
    assert_eq!(shape.target_filter.any_of.len(), 2);
    assert!(
        shape
            .target_filter
            .any_of
            .iter()
            .all(|filter| filter.controller == Some(PlayerFilter::NotYou))
    );
}

#[test]
fn parses_ability_source_type_restriction() {
    let tokens = lex_line("target activated ability from an artifact source", 0).unwrap();
    let shape = parse_counter_ability_target_tokens(&tokens).expect("shape");
    assert_eq!(shape.target_filter.card_types, vec![CardType::Artifact]);
}

#[test]
fn ordinary_targets_relation_requires_any_matching_target() {
    let tokens = lex_line(
        "target spell or ability that targets a creature you control",
        0,
    )
    .unwrap();
    let shape = parse_counter_ability_target_tokens(&tokens).expect("shape");
    assert_eq!(shape.target_filter.any_of.len(), 2);
    assert!(
        shape.target_filter.any_of.iter().all(|filter| {
            filter.targets_object.is_some() && filter.targets_only_object.is_none()
        })
    );
}

#[test]
fn explicit_targets_only_relation_requires_every_target_to_match() {
    let tokens = lex_line(
        "target spell or ability that targets only a creature you control",
        0,
    )
    .unwrap();
    let shape = parse_counter_ability_target_tokens(&tokens).expect("shape");
    assert_eq!(shape.target_filter.any_of.len(), 2);
    assert!(
        shape.target_filter.any_of.iter().all(|filter| {
            filter.targets_object.is_none() && filter.targets_only_object.is_some()
        })
    );
}

#[test]
fn targets_relation_preserves_player_or_controlled_creature_union() {
    let tokens = lex_line(
        "target spell or ability that targets you or a creature you control",
        0,
    )
    .unwrap();
    let shape = parse_counter_ability_target_tokens(&tokens).expect("shape");
    assert_eq!(shape.target_filter.any_of.len(), 2);
    assert!(shape.target_filter.any_of.iter().all(|filter| {
        filter.targets_player == Some(PlayerFilter::You)
            && filter.targets_object.is_some()
            && filter.targets_any_of
            && filter.targets_only_player.is_none()
            && filter.targets_only_object.is_none()
    }));
}
