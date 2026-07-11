use super::*;
use crate::runtime_backend::lexer::lex_line;

fn parse(raw: &str) -> ObjectFilter {
    let tokens = lex_line(raw, 0).expect("lex simple object-filter fixture");
    parse_simple_object_filter_lexed(&tokens, false)
        .unwrap_or_else(|| panic!("simple object filter should parse: {raw}"))
}

#[test]
fn suffixes_preserve_owner_controller_and_zone_semantics() {
    let owned = parse("artifact card from your graveyard");
    assert_eq!(owned.owner, Some(PlayerFilter::You));
    assert_eq!(owned.zone, Some(Zone::Graveyard));
    assert_eq!(owned.card_types, vec![CardType::Artifact]);

    let controlled = parse("land you control but don't own");
    assert_eq!(controlled.controller, Some(PlayerFilter::You));
    assert_eq!(controlled.owner, Some(PlayerFilter::NotYou));
    assert_eq!(controlled.zone, Some(Zone::Battlefield));
}

#[test]
fn controller_suffixes_preserve_target_and_iterated_players() {
    let controller_only = parse("you control");
    assert_eq!(controller_only.controller, Some(PlayerFilter::You));
    assert_eq!(controller_only.zone, Some(Zone::Battlefield));

    let target =
        parse_simple_object_filter_words(&["artifact", "target", "player", "controls"], false)
            .expect("target-player controller suffix");
    assert_eq!(target.controller, Some(PlayerFilter::target_player()));

    let iterated =
        parse_simple_object_filter_words(&["creature", "that", "player", "controls"], false)
            .expect("iterated-player controller suffix");
    assert_eq!(iterated.controller, Some(PlayerFilter::IteratedPlayer));
}

#[test]
fn adjacency_and_explicit_lists_keep_distinct_type_semantics() {
    let adjacent = parse("artifact creature");
    assert!(adjacent.card_types.is_empty());
    assert_eq!(
        adjacent.all_card_types,
        vec![CardType::Artifact, CardType::Creature]
    );

    let listed = parse("artifact, creature, or land");
    assert!(listed.all_card_types.is_empty());
    assert_eq!(
        listed.card_types,
        vec![CardType::Artifact, CardType::Creature, CardType::Land]
    );

    let type_or_subtype = parse("creature or vehicle");
    assert_eq!(type_or_subtype.card_types, vec![CardType::Creature]);
    assert_eq!(type_or_subtype.subtypes, vec![Subtype::Vehicle]);
    assert!(type_or_subtype.type_or_subtype_union);

    let subtype_list_with_trailing_type = parse("cleric rogue warrior or wizard creature card");
    assert_eq!(
        subtype_list_with_trailing_type.card_types,
        vec![CardType::Creature]
    );
    assert_eq!(
        subtype_list_with_trailing_type.subtypes,
        vec![
            Subtype::Cleric,
            Subtype::Rogue,
            Subtype::Warrior,
            Subtype::Wizard,
        ]
    );
    assert!(!subtype_list_with_trailing_type.type_or_subtype_union);
}

#[test]
fn face_state_named_atoms_and_split_non_are_typed() {
    let filter = parse("face down non artifact creature of chosen type");
    assert_eq!(filter.face_down, Some(true));
    assert_eq!(filter.card_types, vec![CardType::Creature]);
    assert_eq!(filter.excluded_card_types, vec![CardType::Artifact]);
    assert!(filter.chosen_creature_type);
}

#[test]
fn other_than_exclusions_preserve_base_filter_and_excluded_types() {
    let filter = parse("creature other than artifact or outlaw");
    assert_eq!(filter.card_types, vec![CardType::Creature]);
    assert_eq!(filter.excluded_card_types, vec![CardType::Artifact]);
    assert!(!filter.excluded_subtypes.is_empty());
}

#[test]
fn spell_markers_preserve_stack_inference() {
    let filter = parse("creature spell you control");
    assert_eq!(filter.zone, Some(Zone::Stack));
    assert_eq!(filter.controller, Some(PlayerFilter::You));
    assert_eq!(filter.stack_kind, Some(StackObjectKind::Spell));
    assert!(filter.has_mana_cost);
}

#[test]
fn permanent_marker_preserves_the_full_permanent_type_union() {
    let filter = parse("permanent card from your graveyard");
    assert_eq!(filter.zone, Some(Zone::Graveyard));
    assert_eq!(filter.card_types, ObjectFilter::permanent_card().card_types);
}

#[test]
fn simple_parser_keeps_complex_that_type_phrase_out_of_its_language() {
    assert!(parse_simple_object_filter_words(&["that", "type"], false).is_none());
}
