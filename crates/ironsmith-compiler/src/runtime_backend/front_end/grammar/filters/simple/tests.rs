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

    let chosen = parse_simple_object_filter_words(
        &["tapped", "lands", "the", "chosen", "player", "controls"],
        false,
    )
    .expect("chosen-player controller suffix");
    assert_eq!(chosen.controller, Some(PlayerFilter::ChosenPlayer));
    assert!(chosen.tapped);

    let chosen_hand = parse_simple_object_filter_words(
        &["card", "in", "the", "chosen", "players", "hand"],
        false,
    )
    .expect("chosen-player hand suffix");
    assert_eq!(chosen_hand.owner, Some(PlayerFilter::ChosenPlayer));
    assert_eq!(chosen_hand.zone, Some(Zone::Hand));
}

#[test]
fn player_or_planeswalker_backref_is_one_typed_controller_suffix() {
    let referenced = parse("creatures that opponent or that planeswalker's controller controls");
    assert_eq!(referenced.card_types, vec![CardType::Creature]);
    assert_eq!(
        referenced.controller,
        Some(PlayerFilter::TargetPlayerOrControllerOfTarget)
    );
    assert_eq!(referenced.zone, Some(Zone::Battlefield));

    let ordinary_union = parse("creatures or planeswalkers you control");
    assert_eq!(
        ordinary_union.card_types,
        vec![CardType::Creature, CardType::Planeswalker]
    );
    assert_eq!(ordinary_union.controller, Some(PlayerFilter::You));
}

#[test]
fn your_team_controller_suffix_preserves_team_scope() {
    let team = parse("Warrior your team controls");
    assert!(
        team.controller
            .as_ref()
            .is_some_and(PlayerFilter::is_your_team)
    );
    assert_eq!(team.description(), "a Warrior your team controls");
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
    assert!(!type_or_subtype.has_subtype_before_card_type_union_surface());

    let controller_qualified_subtype_first = parse("vehicles and/or creatures you control");
    assert_eq!(
        controller_qualified_subtype_first.controller,
        Some(PlayerFilter::You)
    );
    assert_eq!(
        controller_qualified_subtype_first.card_types,
        vec![CardType::Creature]
    );
    assert_eq!(
        controller_qualified_subtype_first.subtypes,
        vec![Subtype::Vehicle]
    );
    assert!(controller_qualified_subtype_first.type_or_subtype_union);
    assert!(controller_qualified_subtype_first.has_subtype_before_card_type_union_surface());
    assert_eq!(
        controller_qualified_subtype_first.union_connective(),
        ObjectFilterUnionConnective::AndOr
    );
    assert_eq!(
        controller_qualified_subtype_first.description(),
        "a Vehicle and/or creature you control"
    );

    let terminal_spell_noun = parse("creature or Aura spell");
    assert!(terminal_spell_noun.has_terminal_noun_after_type_subtype_union_surface());
    assert_eq!(terminal_spell_noun.description(), "creature or Aura spell");
    let nonterminal_spell_noun = parse("instant spell or Aura");
    assert!(!nonterminal_spell_noun.has_terminal_noun_after_type_subtype_union_surface());
    assert_eq!(
        nonterminal_spell_noun.description(),
        "instant spell or Aura"
    );

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
fn adjacent_creature_subtypes_require_every_subtype_but_lists_remain_inclusive() {
    let compound = parse("Eldrazi Spawn creatures you control");
    assert!(compound.subtypes.is_empty(), "{compound:#?}");
    assert_eq!(
        compound.all_subtypes,
        vec![Subtype::Eldrazi, Subtype::Spawn]
    );
    assert_eq!(
        compound.description(),
        "an Eldrazi Spawn creature you control"
    );

    let listed = parse("Eldrazi or Spawn creatures you control");
    assert!(listed.all_subtypes.is_empty(), "{listed:#?}");
    assert_eq!(listed.subtypes, vec![Subtype::Eldrazi, Subtype::Spawn]);
    assert_eq!(
        listed.description(),
        "an Eldrazi or Spawn creature you control"
    );

    let outlaw = parse("outlaw creature you control");
    assert!(outlaw.all_subtypes.is_empty(), "{outlaw:#?}");
    assert_eq!(outlaw.subtypes.len(), 5, "{outlaw:#?}");
}

#[test]
fn explicit_land_noun_is_preserved_separately_from_subtype_semantics() {
    let explicit = parse("Urza's land you control");
    let mut canonical = explicit.clone();
    canonical.set_explicit_card_type_noun(None);

    assert_eq!(explicit.explicit_card_type_noun(), Some(CardType::Land));
    assert_eq!(canonical.explicit_card_type_noun(), None);
    assert_eq!(explicit, canonical);
    assert_eq!(explicit.description(), "an Urza's land you control");
    assert_eq!(canonical.description(), "an Urza's you control");
}

#[test]
fn abuelos_awakening_preserves_non_aura_on_only_the_enchantment_arm() {
    let filter = parse("artifact or non-Aura enchantment card from your graveyard");

    assert_eq!(filter.zone, Some(Zone::Graveyard));
    assert_eq!(filter.owner, Some(PlayerFilter::You));
    assert!(filter.card_types.is_empty());
    assert!(filter.excluded_subtypes.is_empty());
    assert_eq!(filter.any_of.len(), 2);
    assert_eq!(filter.any_of[0].card_types, [CardType::Artifact]);
    assert!(filter.any_of[0].excluded_subtypes.is_empty());
    assert_eq!(filter.any_of[1].card_types, [CardType::Enchantment]);
    assert_eq!(filter.any_of[1].excluded_subtypes, [Subtype::Aura]);
}

#[test]
fn absorbing_man_preserves_branch_local_exclusion_in_a_three_type_union() {
    let filter = parse("artifact, non-Aura enchantment, or land");

    assert_eq!(filter.zone, Some(Zone::Battlefield));
    assert!(filter.card_types.is_empty());
    assert!(filter.excluded_subtypes.is_empty());
    assert_eq!(filter.any_of.len(), 3);
    assert_eq!(filter.any_of[0].card_types, [CardType::Artifact]);
    assert_eq!(filter.any_of[1].card_types, [CardType::Enchantment]);
    assert_eq!(filter.any_of[1].excluded_subtypes, [Subtype::Aura]);
    assert_eq!(filter.any_of[2].card_types, [CardType::Land]);
}

#[test]
fn wave_of_vitriol_preserves_nonbasic_on_only_the_land_arm() {
    let tokens = lex_line(
        "artifacts, enchantments, and nonbasic lands they control",
        0,
    )
    .unwrap();
    let filter = crate::runtime_backend::grammar::filters::
        parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false)
        .expect("branch-scoped card-type list should parse");

    assert_eq!(filter.zone, Some(Zone::Battlefield));
    assert_eq!(filter.controller, Some(PlayerFilter::IteratedPlayer));
    assert!(filter.card_types.is_empty());
    assert!(filter.excluded_supertypes.is_empty());
    assert_eq!(filter.any_of.len(), 3);
    assert_eq!(filter.any_of[0].card_types, [CardType::Artifact]);
    assert!(filter.any_of[0].excluded_supertypes.is_empty());
    assert_eq!(filter.any_of[1].card_types, [CardType::Enchantment]);
    assert!(filter.any_of[1].excluded_supertypes.is_empty());
    assert_eq!(filter.any_of[2].card_types, [CardType::Land]);
    assert_eq!(filter.any_of[2].excluded_supertypes, [Supertype::Basic]);
}

#[test]
fn dance_of_the_manse_preserves_branch_local_exclusion_for_and_or() {
    let filter = parse("artifact and/or non-Aura enchantment cards from your graveyard");

    assert_eq!(
        filter.union_connective(),
        ObjectFilterUnionConnective::AndOr
    );
    assert_eq!(filter.any_of.len(), 2);
    assert!(filter.any_of[0].excluded_subtypes.is_empty());
    assert_eq!(filter.any_of[1].excluded_subtypes, [Subtype::Aura]);
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
fn suspected_is_a_typed_object_filter_flag() {
    let filter = parse("suspected creature you control");

    assert!(filter.suspected);
    assert_eq!(filter.card_types, vec![CardType::Creature]);
    assert_eq!(filter.controller, Some(PlayerFilter::You));
    assert_eq!(filter.zone, Some(Zone::Battlefield));
    assert_eq!(filter.description(), "a suspected creature you control");
}

#[test]
fn foretold_is_a_distinct_typed_exile_card_state() {
    let filter = parse("foretold card in exile");

    assert!(filter.foretold);
    assert_eq!(filter.owner, None);
    assert_eq!(filter.zone, Some(Zone::Exile));
    assert_eq!(filter.description(), "foretold card in exile");
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

    let bare_spell = parse("spell you control");
    assert_eq!(bare_spell.zone, Some(Zone::Stack));
    assert_eq!(bare_spell.controller, Some(PlayerFilter::You));
    assert_eq!(bare_spell.stack_kind, Some(StackObjectKind::Spell));
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
