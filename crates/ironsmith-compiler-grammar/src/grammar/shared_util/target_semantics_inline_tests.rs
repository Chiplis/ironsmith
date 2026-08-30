use super::*;
use crate::color::ColorSet;
use crate::events::KeywordActionKind;
use crate::lexer::lex_line;

fn parse(raw: &str) -> TargetAst {
    let tokens = lex_line(raw, 0).expect("lex target");
    parse_target_phrase_inner(&tokens).expect(raw)
}

fn parse_with_source(raw: &str, source_name: &str) -> TargetAst {
    let context =
        crate::parse_context::ParseContext::for_fragment(source_name, Vec::new(), Vec::new(), raw);
    let tokens = lex_line(raw, 0).expect("lex contextual target");
    crate::util::parse_target_phrase_with_context(context.view(), &tokens).expect(raw)
}

#[test]
fn total_mana_value_target_restriction_is_lifted_to_the_selected_set() {
    let TargetAst::WithCount(inner, count) = parse(
        "any number of target creature cards with total mana value 6 or less from your graveyard",
    ) else {
        panic!("expected a counted target");
    };
    assert_eq!(count, ChoiceCount::any_number());
    let TargetAst::Object(filter, explicit_target, _) = *inner else {
        panic!("expected an object target");
    };
    assert!(explicit_target.is_some());
    assert_eq!(filter.zone, Some(Zone::Graveyard));
    assert!(filter.mana_value.is_none());
    let constraint = filter
        .target_set_aggregate_constraint
        .as_deref()
        .expect("aggregate target-set constraint");
    assert_eq!(
        constraint,
        &crate::effect::ChoiceAggregateConstraint::total_mana_value_at_most(6)
    );
}

#[test]
fn ordinary_mana_value_target_restriction_remains_per_object() {
    let TargetAst::WithCount(inner, _) =
        parse("any number of target creature cards with mana value 6 or less from your graveyard")
    else {
        panic!("expected a counted target");
    };
    let TargetAst::Object(filter, _, _) = *inner else {
        panic!("expected an object target");
    };
    assert!(filter.mana_value.is_some());
    assert!(filter.target_set_aggregate_constraint.is_none());
}

#[test]
fn qualified_any_target_exclusion_keeps_players_in_the_target_domain() {
    let TargetAst::ObjectOrPlayer(filter, player, explicit_target) =
        parse("any target that isn't a Dinosaur")
    else {
        panic!("expected a qualified object-or-player target");
    };
    assert_eq!(player, PlayerFilter::Any);
    assert!(explicit_target.is_some());
    assert_eq!(filter.excluded_subtypes, [crate::types::Subtype::Dinosaur]);
}

#[test]
fn any_target_other_than_damaged_permanent_keeps_both_domains_and_identity() {
    let TargetAst::ObjectOrPlayer(filter, player, explicit_target) =
        parse("any target other than that permanent")
    else {
        panic!("expected an object-or-player target");
    };
    assert_eq!(player, PlayerFilter::Any);
    assert!(explicit_target.is_some());
    assert_eq!(filter.zone, Some(Zone::Battlefield));
    assert!(filter.tagged_constraints.iter().any(|constraint| {
        constraint.tag.as_str() == "damaged"
            && constraint.relation == TaggedOpbjectRelation::IsNotTaggedObject
    }));
}

#[test]
fn target_other_than_source_remains_an_explicit_object_target() {
    let TargetAst::Object(filter, explicit_target, _) =
        parse("target creature other than this creature")
    else {
        panic!("expected object target");
    };
    assert!(explicit_target.is_some());
    assert!(filter.other);
    assert!(!filter.source);
    assert_eq!(filter.card_types, [CardType::Creature]);
    assert!(filter.source_surface.is_some());
}

#[test]
fn definite_combat_role_targets_keep_the_block_pair_identity() {
    for (text, expected_tag, expected_role) in [
        ("the blocking creature", "blocking", "blocking"),
        ("the attacking creature", "blocked", "attacking"),
    ] {
        let TargetAst::Object(filter, explicit_target, _) = parse(text) else {
            panic!("expected object target for {text}");
        };
        assert!(explicit_target.is_none(), "{filter:#?}");
        assert!(
            filter.tagged_constraints.iter().any(|constraint| {
                constraint.tag.as_str() == expected_tag
                    && constraint.relation == TaggedOpbjectRelation::IsTaggedObject
            }),
            "{filter:#?}"
        );
        assert_eq!(filter.blocking, expected_role == "blocking", "{filter:#?}");
        assert_eq!(
            filter.attacking,
            expected_role == "attacking",
            "{filter:#?}"
        );
    }
}

#[test]
fn another_stickered_target_does_not_turn_reflexive_it_into_a_reference() {
    let TargetAst::Object(filter, explicit_target, it_span) =
        parse("another target creature with an art sticker on it")
    else {
        panic!("expected object target");
    };
    assert!(explicit_target.is_some());
    assert!(filter.other);
    assert!(!filter.source);
    assert_eq!(filter.sticker, Some(KeywordActionKind::ArtSticker));
    assert!(filter.tagged_constraints.is_empty());
    assert!(it_span.is_none());
}

#[test]
fn each_other_object_preserves_source_exclusion() {
    let TargetAst::Object(filter, _, _) = parse("each other creature") else {
        panic!("expected object filter");
    };
    assert!(filter.other);
    assert_eq!(filter.card_types, vec![CardType::Creature]);
}

#[test]
fn counters_put_on_it_this_way_keeps_exact_producer_set_reference() {
    let TargetAst::Object(filter, _, _) =
        parse("each creature that had counters put on it this way")
    else {
        panic!("expected object filter");
    };
    assert_eq!(filter.card_types, vec![CardType::Creature]);
    assert_eq!(
        filter.union_surface.prior_effect_action(),
        Some(ironsmith_core::PriorEffectAction::CountersPut)
    );
    assert!(filter.tagged_constraints.iter().any(|constraint| {
        constraint.tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str()
            && constraint.relation == TaggedOpbjectRelation::IsTaggedObject
    }));
}

#[test]
fn became_a_creature_this_way_keeps_exact_animation_result_reference() {
    let TargetAst::Object(filter, _, _) = parse("each artifact that became a creature this way")
    else {
        panic!("expected object filter");
    };
    assert!(filter.card_types.contains(&CardType::Artifact));
    assert!(filter.tagged_constraints.iter().any(|constraint| {
        constraint.tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str()
            && constraint.relation == TaggedOpbjectRelation::IsTaggedObject
    }));
}

#[test]
fn battle_or_opponent_preserves_both_target_domains_and_source_exclusion() {
    let TargetAst::ObjectOrPlayer(filter, player, explicit_target) =
        parse("another target battle or opponent")
    else {
        panic!("expected object/player union target");
    };
    assert!(explicit_target.is_some());
    assert!(filter.other);
    assert_eq!(filter.zone, Some(Zone::Battlefield));
    assert_eq!(filter.card_types, vec![CardType::Battle]);
    assert_eq!(player, PlayerFilter::Opponent);
}

#[test]
fn attacking_or_blocking_target_preserves_both_combat_roles() {
    let TargetAst::Object(filter, explicit_target, _) =
        parse("target attacking or blocking creature")
    else {
        panic!("expected object target");
    };
    assert!(explicit_target.is_some());
    assert_eq!(filter.any_of.len(), 2, "{filter:#?}");
    assert!(
        filter.any_of.iter().any(|branch| {
            branch.attacking && !branch.blocking && branch.card_types == [CardType::Creature]
        }),
        "{filter:#?}"
    );
    assert!(
        filter.any_of.iter().any(|branch| {
            branch.blocking && !branch.attacking && branch.card_types == [CardType::Creature]
        }),
        "{filter:#?}"
    );
}

#[test]
fn opponent_target_unions_preserve_every_legal_recipient_domain() {
    assert!(matches!(
        parse("target opponent or planeswalker"),
        TargetAst::PlayerOrPlaneswalker(PlayerFilter::Opponent, Some(_))
    ));

    let TargetAst::ObjectOrPlayer(filter, player, explicit_target) =
        parse("target opponent or battle")
    else {
        panic!("expected opponent/battle union target");
    };
    assert!(explicit_target.is_some());
    assert_eq!(player, PlayerFilter::Opponent);
    assert_eq!(filter.card_types, [CardType::Battle]);
}

#[test]
fn object_type_list_or_opponent_preserves_every_target_domain() {
    let TargetAst::ObjectOrPlayer(filter, player, explicit_target) =
        parse("target artifact, creature, planeswalker, or opponent")
    else {
        panic!("expected object/player union target");
    };
    assert!(explicit_target.is_some());
    assert_eq!(filter.zone, Some(Zone::Battlefield));
    assert_eq!(
        filter.card_types,
        vec![
            CardType::Artifact,
            CardType::Creature,
            CardType::Planeswalker
        ]
    );
    assert_eq!(player, PlayerFilter::Opponent);
}

#[test]
fn non_target_permanent_or_player_union_remains_non_targeting() {
    let TargetAst::ObjectOrPlayer(filter, player, explicit_target) = parse("a permanent or player")
    else {
        panic!("expected object/player union reference");
    };
    assert!(explicit_target.is_none());
    assert_eq!(filter.zone, Some(Zone::Battlefield));
    assert_eq!(player, PlayerFilter::Any);
}

#[test]
fn attacked_player_or_planeswalker_remains_a_combat_reference() {
    assert!(matches!(
        parse("the player or planeswalker it's attacking"),
        TargetAst::AttackedPlayerOrPlaneswalker(_)
    ));
}

#[test]
fn both_color_target_records_an_all_of_color_constraint() {
    let TargetAst::Object(filter, explicit_target, _) =
        parse("target spell thats both blue and black")
    else {
        panic!("expected spell object target");
    };
    assert!(explicit_target.is_some());
    assert_eq!(
        filter.required_colors,
        Some(ColorSet::BLUE.union(ColorSet::BLACK))
    );
    assert_eq!(filter.colors, None);
}

#[test]
fn typed_demonstrative_target_records_its_exact_surface() {
    let tokens = lex_line("that creature", 0).expect("lex target");
    let target = parse_target_phrase_inner(&tokens).expect("parse target");
    let TargetAst::Object(filter, _, Some(_span)) = target else {
        panic!("expected typed demonstrative object target with reference span");
    };
    assert_eq!(
        filter.source_surface,
        Some(SourceReferenceSurface::ThisPermanentType(
            "that creature".to_string()
        ))
    );
}

#[test]
fn qualified_typed_demonstrative_target_keeps_prior_object_identity() {
    let tokens = lex_line("that non-Wall creature", 0).expect("lex target");
    let target = parse_target_phrase_inner(&tokens).expect("parse target");
    let TargetAst::Object(filter, _, Some(_span)) = target else {
        panic!("expected qualified demonstrative object target with reference span");
    };
    assert!(filter.tagged_constraints.iter().any(|constraint| {
        constraint.tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str()
            && constraint.relation == TaggedOpbjectRelation::IsTaggedObject
    }));
    assert_eq!(
        filter.source_surface,
        Some(SourceReferenceSurface::ThisPermanentType(
            "that non-wall creature".to_string()
        ))
    );
}

#[test]
fn definite_card_target_is_a_tagged_reference() {
    let TargetAst::Tagged(tag, Some(_span)) = parse("the card") else {
        panic!("expected definite card reference");
    };
    assert_eq!(tag.as_str(), crate::tag::CompilerReferenceTag::It.as_str());
}

#[test]
fn bare_anaphoric_and_attachment_targets_reach_typed_semantics() {
    let TargetAst::Tagged(it, _) = parse("it") else {
        panic!("expected bare it to resolve as a tagged reference");
    };
    assert_eq!(it.as_str(), crate::tag::CompilerReferenceTag::It.as_str());

    let TargetAst::Object(enchanted, _, _) = parse("enchanted creature") else {
        panic!("expected enchanted creature to resolve as an attachment reference");
    };
    assert!(enchanted.tagged_constraints.iter().any(|constraint| {
        constraint.tag.as_str() == "enchanted"
            && constraint.relation == TaggedOpbjectRelation::IsTaggedObject
    }));

    assert!(matches!(
        parse("you"),
        TargetAst::Player(PlayerFilter::You, _)
    ));
}

#[test]
fn sacrificed_object_target_is_a_typed_tagged_reference() {
    let TargetAst::Tagged(tag, Some(_span)) = parse("the sacrificed creature") else {
        panic!("expected tagged sacrificed-object reference");
    };
    assert_eq!(tag.as_str(), crate::tag::CompilerReferenceTag::It.as_str());
}

#[test]
fn named_possessive_source_target_preserves_short_name_surface() {
    let parsed = parse_with_source("Casey Jones's", "Casey Jones, Asphalt Hooligan");
    assert!(
        matches!(parsed, TargetAst::Source(Some(_span))),
        "expected named possessive to resolve to the source: {parsed:#?}"
    );
}

#[test]
fn full_name_possessive_source_target_preserves_full_name_surface() {
    let parsed = parse_with_source("Tifa Lockhart's", "Tifa Lockhart");
    assert!(
        matches!(parsed, TargetAst::Source(Some(_span))),
        "expected named possessive to resolve to the source: {parsed:#?}"
    );
}
