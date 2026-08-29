use super::*;
use crate::lexer::lex_line;
use crate::static_abilities::StaticAbilityId;
use crate::target::ObjectCharacteristicRelationKind;

fn parse_filter(text: &str) -> ObjectFilter {
    let tokens = lex_line(text, 0).expect("filter text should lex");
    parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false)
        .expect("filter text should parse")
}

#[test]
fn spell_filter_preserves_authored_convoke_ability_requirement() {
    let filter = parse_filter("a spell that has convoke");

    assert_eq!(filter.static_abilities, [StaticAbilityId::Convoke]);
    assert!(filter.ability_markers.is_empty(), "{filter:#?}");
}

#[test]
fn qualified_spell_and_ability_set_keeps_the_complete_stack_domain() {
    let filter = parse_filter("each spell and ability your opponents control");

    assert_eq!(filter.zone, Some(Zone::Stack));
    assert_eq!(
        filter.stack_kind,
        Some(crate::filter::StackObjectKind::SpellOrAbility)
    );
    assert!(!filter.has_mana_cost, "{filter:#?}");
    assert!(filter.has_conjunctive_set_surface(), "{filter:#?}");
}

#[test]
fn suffix_conjunction_does_not_weaken_compound_subtype_identity() {
    let filter = parse_filter("Eldrazi Spawn creature you both own and control");

    assert!(filter.subtypes.is_empty(), "{filter:#?}");
    assert_eq!(filter.all_subtypes, vec![Subtype::Eldrazi, Subtype::Spawn]);
    assert_eq!(filter.owner, Some(PlayerFilter::You));
    assert_eq!(filter.controller, Some(PlayerFilter::You));
}

#[test]
fn foretold_owner_zone_filter_keeps_runtime_state_and_authored_scope() {
    let filter = parse_filter("foretold card you own in exile");

    assert!(filter.foretold);
    assert_eq!(filter.owner, Some(PlayerFilter::You));
    assert_eq!(filter.zone, Some(Zone::Exile));
    assert_eq!(filter.description(), "a foretold card you own in exile");
}

#[test]
fn graveyard_cards_with_different_mana_values_keep_selection_constraint() {
    let filter = parse_filter("cards with different mana values from your graveyard");

    assert!(filter.distinct_mana_values, "{filter:#?}");
    assert_eq!(filter.zone, Some(Zone::Graveyard));
    assert_eq!(filter.owner, Some(PlayerFilter::You));
    assert!(
        filter.description().contains("with different mana values"),
        "{}",
        filter.description()
    );
}

#[test]
fn set_quantifier_before_pt_literal_keeps_the_exact_characteristics() {
    let filter = parse_filter("each 1/1 creature you control");

    assert_eq!(filter.card_types, vec![CardType::Creature]);
    assert_eq!(filter.controller, Some(PlayerFilter::You));
    assert_eq!(filter.power, Some(crate::filter::Comparison::Equal(1)));
    assert_eq!(filter.toughness, Some(crate::filter::Comparison::Equal(1)));
}

#[test]
fn excluded_literal_name_keeps_original_case_apostrophe_and_comma_surface() {
    let filter = parse_filter(
        "target legendary permanent card not named Staff of Eden, Vault's Key from a graveyard",
    );

    assert_eq!(
        filter.excluded_name.as_deref(),
        Some("staff of eden vaults key")
    );
    assert_eq!(
        filter.excluded_name_surface(),
        Some("Staff of Eden, Vault's Key")
    );
}

#[test]
fn creature_type_relation_keeps_comparison_controller_out_of_candidate() {
    let filter =
        parse_filter("creature card that shares a creature type with a creature you control");

    assert_eq!(filter.card_types, vec![CardType::Creature]);
    assert_eq!(filter.controller, None);
    assert_eq!(filter.characteristic_relations.len(), 1);
    let relation = &filter.characteristic_relations[0];
    assert_eq!(relation.kind, ObjectCharacteristicRelationKind::SharesAny);
    assert_eq!(
        relation.characteristics,
        vec![ObjectCharacteristic::Subtype(SubtypeFamily::Creature)]
    );
    assert_eq!(relation.comparison.card_types, vec![CardType::Creature]);
    assert_eq!(relation.comparison.controller, Some(PlayerFilter::You));
    assert_eq!(relation.comparison.zone, Some(Zone::Battlefield));
    assert_eq!(
        filter.description(),
        "creature card that shares a creature type with a creature you control"
    );
}

#[test]
fn attacked_planeswalker_clause_does_not_expand_or_control_the_attacker() {
    let filter = parse_filter("creature that's attacking you or a planeswalker you control");

    assert_eq!(filter.card_types, vec![CardType::Creature]);
    assert_eq!(filter.controller, None);
    assert!(filter.attacking);
    assert_eq!(
        filter.attacking_player_or_planeswalker_controlled_by,
        Some(PlayerFilter::You)
    );
    assert!(!filter.attacking_player_only);
}

#[test]
fn attacking_your_opponents_keeps_the_opponent_destination_union() {
    let filter =
        parse_filter("creatures attacking your opponents and/or planeswalkers they control");

    assert_eq!(filter.card_types, vec![CardType::Creature]);
    assert_eq!(filter.controller, None);
    assert!(filter.attacking);
    assert_eq!(
        filter.attacking_player_or_planeswalker_controlled_by,
        Some(PlayerFilter::Opponent)
    );
    assert!(!filter.attacking_player_only);
    assert_eq!(
        filter.description(),
        "creatures attacking your opponents and/or planeswalkers they control"
    );
}

#[test]
fn attacking_alone_is_an_executable_post_noun_state() {
    let filter = parse_filter("creature that's attacking alone");

    assert_eq!(filter.card_types, vec![CardType::Creature]);
    assert!(filter.attacking);
    assert!(filter.attacking_alone);
    assert_eq!(filter.description(), "creature that's attacking alone");

    let ordinary = parse_filter("attacking creature");
    assert!(ordinary.attacking);
    assert!(!ordinary.attacking_alone);
}

#[test]
fn attacking_last_chosen_player_keeps_persistent_player_relation() {
    let filter = parse_filter("creature attacking the last chosen player");

    assert_eq!(filter.card_types, vec![CardType::Creature]);
    assert!(filter.attacking);
    assert_eq!(
        filter.attacking_player_or_planeswalker_controlled_by,
        Some(PlayerFilter::ChosenPlayer)
    );
    assert!(filter.attacking_player_only);
}

#[test]
fn source_and_chosen_object_exclusions_keep_both_identities() {
    let filter = parse_filter("creatures other than this creature and the chosen creature");

    assert_eq!(filter.card_types, vec![CardType::Creature]);
    assert!(filter.other, "the source exclusion must remain executable");
    assert!(matches!(
        filter.source_surface.as_ref(),
        Some(crate::target::SourceReferenceSurface::ThisPermanentType(noun))
            if noun == "this creature"
    ));
    assert_eq!(filter.tagged_constraints.len(), 1, "{filter:#?}");
    assert_eq!(
        filter.tagged_constraints[0],
        TaggedObjectConstraint {
            tag: TagKey::from(crate::cards::builders::CHOSEN_OBJECTS_TAG),
            relation: TaggedOpbjectRelation::IsNotTaggedObject,
        }
    );
}

#[test]
fn targets_only_relation_keeps_source_exclusion_on_nested_filter() {
    let filter =
        parse_filter("a spell that targets only a single creature other than this creature");
    let target = filter
        .targets_only_object
        .as_deref()
        .expect("spell filter should retain its sole object target");

    assert!(target.other, "{target:#?}");
    assert_eq!(target.card_types, [CardType::Creature]);
    assert!(matches!(
        target.source_surface.as_ref(),
        Some(crate::target::SourceReferenceSurface::ThisPermanentType(noun))
            if noun == "this creature"
    ));
}

#[test]
fn compound_ambiguous_subtype_phrase_keeps_both_subtypes() {
    let filter = parse_filter("all Sand Warriors");

    assert!(filter.subtypes.is_empty(), "{filter:#?}");
    assert_eq!(filter.all_subtypes, vec![Subtype::Sand, Subtype::Warrior]);
}

#[test]
fn attachment_host_noun_does_not_narrow_the_attachment_filter() {
    let filter = parse_filter("Equipment attached to that creature");

    assert!(filter.card_types.is_empty(), "{filter:#?}");
    assert_eq!(filter.subtypes, vec![Subtype::Equipment], "{filter:#?}");
    assert!(filter.tagged_constraints.iter().any(|constraint| {
        constraint.tag.as_str() == IT_TAG
            && constraint.relation == TaggedOpbjectRelation::AttachedToTaggedObject
    }));
}

#[test]
fn explicit_non_subtype_with_numeric_suffix_is_never_readded_as_positive() {
    for (text, card_type, excluded_subtype) in [
        (
            "non-Equipment artifact you control with mana value 4 or greater",
            CardType::Artifact,
            Subtype::Equipment,
        ),
        (
            "non-Aura enchantment you control with mana value 4 or greater",
            CardType::Enchantment,
            Subtype::Aura,
        ),
    ] {
        let filter = parse_filter(text);

        assert_eq!(filter.card_types, vec![card_type], "{filter:#?}");
        assert_eq!(
            filter.excluded_subtypes,
            vec![excluded_subtype],
            "{filter:#?}"
        );
        assert!(!filter.subtypes.contains(&excluded_subtype), "{filter:#?}");
        assert_eq!(filter.controller, Some(PlayerFilter::You), "{filter:#?}");
        assert!(filter.mana_value.is_some(), "{filter:#?}");
    }
}

#[test]
fn historical_block_relation_keeps_partner_characteristics_nested() {
    let filter =
        parse_filter("creature that blocked or was blocked by a Zombie you control this turn");

    assert_eq!(filter.card_types, vec![CardType::Creature]);
    assert!(filter.subtypes.is_empty());
    assert!(!filter.blocked);
    assert!(!filter.blocking);
    let partner = filter
        .blocked_or_was_blocked_by_this_turn
        .as_deref()
        .expect("typed historical combat partner");
    assert_eq!(partner.subtypes, vec![Subtype::Zombie]);
    assert_eq!(partner.controller, Some(PlayerFilter::You));
    assert_eq!(partner.zone, Some(Zone::Battlefield));
    assert_eq!(
        filter.description(),
        "creature that blocked or was blocked by a Zombie you control this turn"
    );
}

#[test]
fn active_voice_blocked_this_turn_is_history_not_current_combat_state() {
    let filter = parse_filter("target creature that blocked this turn");

    assert_eq!(filter.card_types, vec![CardType::Creature]);
    assert!(filter.blocked_this_turn, "{filter:#?}");
    assert!(!filter.blocked, "{filter:#?}");
    assert!(!filter.blocking, "{filter:#?}");
    assert_eq!(filter.description(), "creature that blocked this turn");
}

#[test]
fn color_relation_keeps_legendary_comparison_identity_nested() {
    let filter = parse_filter("card that shares a color with a legendary creature you control");

    assert!(filter.has_explicit_card_noun());
    assert!(filter.supertypes.is_empty());
    assert_eq!(filter.controller, None);
    let relation = &filter.characteristic_relations[0];
    assert_eq!(relation.characteristics, vec![ObjectCharacteristic::Color]);
    assert_eq!(relation.comparison.supertypes, vec![Supertype::Legendary]);
    assert_eq!(relation.comparison.card_types, vec![CardType::Creature]);
    assert_eq!(relation.comparison.controller, Some(PlayerFilter::You));
    assert_eq!(
        filter.description(),
        "card that shares a color with a legendary creature you control"
    );
}

#[test]
fn negated_land_type_relation_keeps_basic_on_candidate_only() {
    let filter =
        parse_filter("basic land card that doesn't share a land type with a land you control");

    assert_eq!(filter.supertypes, vec![Supertype::Basic]);
    assert_eq!(filter.card_types, vec![CardType::Land]);
    assert_eq!(filter.controller, None);
    let relation = &filter.characteristic_relations[0];
    assert_eq!(relation.kind, ObjectCharacteristicRelationKind::SharesNone);
    assert_eq!(
        relation.characteristics,
        vec![ObjectCharacteristic::Subtype(SubtypeFamily::Land)]
    );
    assert!(relation.comparison.supertypes.is_empty());
    assert_eq!(relation.comparison.card_types, vec![CardType::Land]);
    assert_eq!(relation.comparison.controller, Some(PlayerFilter::You));
    assert_eq!(
        filter.description(),
        "basic land card that doesn't share a land type with a land you control"
    );
}

#[test]
fn tagged_comparison_surfaces_remain_nested_in_generic_relations() {
    let equipped = parse_filter("creature that shares a color with equipped creature");
    let equipped_relation = &equipped.characteristic_relations[0];
    assert_eq!(
        equipped_relation.characteristics,
        vec![ObjectCharacteristic::Color]
    );
    assert!(
        equipped_relation
            .comparison
            .tagged_constraints
            .iter()
            .any(|constraint| {
                constraint.tag.as_str() == "equipped"
                    && constraint.relation == TaggedOpbjectRelation::IsTaggedObject
            })
    );
    assert_eq!(
        equipped_relation.comparison_description(),
        "equipped creature"
    );

    let exiled = parse_filter("spell that shares a color or mana value with the exiled card");
    let exiled_relation = &exiled.characteristic_relations[0];
    assert_eq!(
        exiled_relation.characteristics,
        vec![ObjectCharacteristic::Color, ObjectCharacteristic::ManaValue]
    );
    assert!(
        exiled_relation
            .comparison
            .tagged_constraints
            .iter()
            .any(|constraint| {
                constraint.tag.as_str() == crate::tag::SOURCE_EXILED_TAG
                    && constraint.relation == TaggedOpbjectRelation::IsTaggedObject
            })
    );
    assert_eq!(exiled_relation.comparison_description(), "the exiled card");
}

#[test]
fn convoked_comparison_keeps_its_tag_and_candidate_identity_separate() {
    let filter = parse_filter(
        "creature that shares a creature type with a creature that convoked this spell",
    );

    assert_eq!(filter.card_types, vec![CardType::Creature]);
    assert_eq!(filter.tagged_constraints.len(), 0);
    let comparison = &filter.characteristic_relations[0].comparison;
    assert_eq!(comparison.card_types, vec![CardType::Creature]);
    assert!(comparison.tagged_constraints.iter().any(|constraint| {
        constraint.tag.as_str() == "convoked_this_spell"
            && constraint.relation == TaggedOpbjectRelation::IsTaggedObject
    }));
    assert_eq!(
        filter.description(),
        "creature that shares a creature type with a creature that convoked this spell"
    );
}

#[test]
fn enchanted_by_relation_keeps_host_and_aura_constraints_separate() {
    let filter =
        parse_filter("creature your opponents control that's enchanted by an Aura you control");

    assert_eq!(filter.card_types, vec![CardType::Creature]);
    assert_eq!(filter.controller, Some(PlayerFilter::Opponent));
    assert!(filter.subtypes.is_empty());
    assert!(filter.has_relative_attachment_state_surface());
    let aura = filter
        .with_attached_object
        .as_deref()
        .expect("enchanted-by clause should create a nested attachment filter");
    assert_eq!(aura.subtypes, vec![Subtype::Aura]);
    assert_eq!(aura.controller, Some(PlayerFilter::You));
}

#[test]
fn activated_this_turn_is_a_branch_local_executable_object_history_fact() {
    let filter = parse_filter("planeswalker that was activated this turn or tapped creature");

    assert_eq!(filter.any_of.len(), 2, "{filter:#?}");
    assert!(filter.any_of.iter().any(|branch| {
        branch.card_types == [CardType::Planeswalker] && branch.ability_activated_this_turn
    }));
    assert!(filter.any_of.iter().any(|branch| {
        branch.card_types == [CardType::Creature]
            && branch.tapped
            && !branch.ability_activated_this_turn
    }));
    assert_eq!(
        filter.description(),
        "planeswalker that was activated this turn or tapped creature"
    );
}

#[test]
fn not_enchanted_is_the_negative_aura_attachment_predicate() {
    let filter = parse_filter("creatures that aren't enchanted");

    assert_eq!(filter.card_types, [CardType::Creature], "{filter:#?}");
    let aura = filter
        .without_attached_object
        .as_deref()
        .expect("negative enchanted state should retain a typed attachment filter");
    assert_eq!(aura.card_types, [CardType::Enchantment]);
    assert_eq!(aura.subtypes, [Subtype::Aura]);
    assert_eq!(filter.description(), "creature that isn't enchanted");
}
