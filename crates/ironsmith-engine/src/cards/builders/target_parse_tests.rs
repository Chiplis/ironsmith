use super::*;

#[test]
fn parse_target_creature() {
    let tokens = tokenize_line("target creature", 0);
    let target = parse_target_phrase(&tokens).expect("parse target creature");
    match target {
        TargetAst::Object(filter, _, _) => {
            assert_eq!(filter, ObjectFilter::creature());
        }
        _ => panic!("expected object target"),
    }
}

#[test]
fn parse_target_artifact_or_enchantment() {
    let tokens = tokenize_line("target artifact or enchantment", 0);
    let target = parse_target_phrase(&tokens).expect("parse target artifact or enchantment");
    match target {
        TargetAst::Object(filter, _, _) => {
            let expected = ObjectFilter::any_of_types(&[CardType::Artifact, CardType::Enchantment]);
            assert_eq!(filter, expected);
        }
        _ => panic!("expected object target"),
    }
}

#[test]
fn parse_target_battle() {
    let tokens = tokenize_line("target battle", 0);
    let target = parse_target_phrase(&tokens).expect("parse target battle");
    match target {
        TargetAst::Object(filter, _, _) => {
            let expected = ObjectFilter::default()
                .in_zone(Zone::Battlefield)
                .with_type(CardType::Battle);
            assert_eq!(filter, expected);
        }
        _ => panic!("expected object target"),
    }
}

#[test]
fn parse_target_creature_you_control() {
    let tokens = tokenize_line("target creature you control", 0);
    let target = parse_target_phrase(&tokens).expect("parse target creature you control");
    match target {
        TargetAst::Object(filter, _, _) => {
            assert_eq!(filter, ObjectFilter::creature().you_control());
        }
        _ => panic!("expected object target"),
    }
}

#[test]
fn parse_another_target_creature_you_control() {
    let tokens = tokenize_line("another target creature you control", 0);
    let target = parse_target_phrase(&tokens).expect("parse another target creature");
    match target {
        TargetAst::Object(filter, _, _) => {
            assert_eq!(filter, ObjectFilter::creature().you_control().other());
        }
        _ => panic!("expected object target"),
    }
}

#[test]
fn parse_target_nonblack_creature() {
    let tokens = tokenize_line("target nonblack creature", 0);
    let target = parse_target_phrase(&tokens).expect("parse target nonblack creature");
    match target {
        TargetAst::Object(filter, _, _) => {
            let expected = ObjectFilter::creature().without_colors(ColorSet::BLACK);
            assert_eq!(filter, expected);
        }
        _ => panic!("expected object target"),
    }
}

#[test]
fn parse_target_on_it() {
    let tokens = tokenize_line("on it", 0);
    let target = parse_target_phrase(&tokens).expect("parse on it");
    match target {
        TargetAst::Tagged(tag, _) => {
            assert_eq!(tag.as_str(), IT_TAG);
        }
        TargetAst::Object(filter, _, _) => {
            assert_eq!(filter.tagged_constraints.len(), 1);
            let constraint = &filter.tagged_constraints[0];
            assert_eq!(constraint.tag.as_str(), IT_TAG);
            assert_eq!(
                constraint.relation,
                crate::TaggedOpbjectRelation::IsTaggedObject
            );
        }
        _ => panic!("expected object target"),
    }
}

#[test]
fn parse_target_it_as_tagged_reference() {
    let tokens = tokenize_line("it", 0);
    let target = parse_target_phrase(&tokens).expect("parse it");
    match target {
        TargetAst::Tagged(tag, _) => assert_eq!(tag.as_str(), IT_TAG),
        other => panic!("expected tagged it reference, got {other:?}"),
    }
}

#[test]
fn parse_target_this_as_source() {
    let tokens = tokenize_line("this", 0);
    let target = parse_target_phrase(&tokens).expect("parse this");
    assert!(matches!(target, TargetAst::Source(_)));
}

#[test]
fn parse_target_this_creature_as_source() {
    let tokens = tokenize_line("this creature", 0);
    let target = parse_target_phrase(&tokens).expect("parse this creature");
    assert!(matches!(target, TargetAst::Source(_)));
}

#[test]
fn parse_target_this_card_from_your_graveyard_as_source() {
    let tokens = tokenize_line("this card from your graveyard", 0);
    let target = parse_target_phrase(&tokens).expect("parse this card from your graveyard");
    match target {
        TargetAst::Object(filter, _, _) => {
            assert!(filter.source, "expected source filter");
            assert_eq!(filter.zone, Some(Zone::Graveyard));
            assert_eq!(filter.owner, Some(PlayerFilter::You));
        }
        _ => panic!("expected source-object graveyard target"),
    }
}

#[test]
fn parse_permanent_shares_card_type_with_it() {
    let tokens = tokenize_line("a permanent that shares a card type with it", 0);
    let filter = parse_object_filter(&tokens, false).expect("parse shared card type filter");
    assert_eq!(filter.tagged_constraints.len(), 1);
    let constraint = &filter.tagged_constraints[0];
    assert_eq!(constraint.tag.as_str(), IT_TAG);
    assert_eq!(
        constraint.relation,
        crate::TaggedOpbjectRelation::SharesCardType
    );
}

#[test]
fn parse_object_filter_enchanted_creature_adds_attachment_tag() {
    let tokens = tokenize_line("enchanted creature", 0);
    let filter = parse_object_filter(&tokens, false).expect("parse enchanted creature filter");
    assert!(
        filter.card_types.contains(&CardType::Creature),
        "expected creature type in filter"
    );
    assert!(
        filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag.as_str() == "enchanted"
                && constraint.relation == crate::TaggedOpbjectRelation::IsTaggedObject
        }),
        "expected enchanted attachment constraint, got {:?}",
        filter.tagged_constraints
    );
}

#[test]
fn parse_object_filter_equipped_creature_adds_attachment_tag() {
    let tokens = tokenize_line("equipped creature", 0);
    let filter = parse_object_filter(&tokens, false).expect("parse equipped creature filter");
    assert!(
        filter.card_types.contains(&CardType::Creature),
        "expected creature type in filter"
    );
    assert!(
        filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag.as_str() == "equipped"
                && constraint.relation == crate::TaggedOpbjectRelation::IsTaggedObject
        }),
        "expected equipped attachment constraint, got {:?}",
        filter.tagged_constraints
    );
}

#[test]
fn parse_object_filter_cards_with_cycling_from_your_graveyard() {
    let tokens = tokenize_line("cards with cycling from your graveyard", 0);
    let filter =
        parse_object_filter(&tokens, false).expect("parse cycling graveyard object filter");
    assert_eq!(filter.zone, Some(Zone::Graveyard));
    assert_eq!(filter.owner, Some(PlayerFilter::You));
    assert!(
        filter
            .ability_markers
            .iter()
            .any(|marker| marker.eq_ignore_ascii_case("cycling")),
        "expected cycling marker in filter, got {:?}",
        filter.ability_markers
    );
}

#[test]
fn parse_object_filter_exiled_with_this_artifact_keeps_target_type() {
    let tokens = tokenize_line("target creature card exiled with this artifact", 0);
    let target = parse_target_phrase(&tokens).expect("parse exiled-with-source object filter");
    let TargetAst::Object(filter, _, _) = target else {
        panic!("expected object target");
    };
    assert!(
        filter.card_types.contains(&CardType::Creature),
        "expected creature type"
    );
    assert!(
        !filter.card_types.contains(&CardType::Artifact),
        "source artifact reference should not become a target type"
    );
    assert!(
        !filter.all_card_types.contains(&CardType::Artifact),
        "source artifact reference should not become an all-card-types selector"
    );
    assert_eq!(filter.zone, Some(Zone::Exile));
    assert!(
        filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag.as_str() == crate::tag::SOURCE_EXILED_TAG
                && constraint.relation == crate::TaggedOpbjectRelation::IsTaggedObject
        }),
        "expected source-linked exile tag, got {:?}",
        filter.tagged_constraints
    );
}

#[test]
fn parse_object_filter_commanders_you_own_sets_commander_and_owner() {
    let tokens = tokenize_line("commander creatures you own", 0);
    let filter =
        parse_object_filter(&tokens, false).expect("parse commander creatures you own filter");
    assert!(filter.is_commander, "expected commander marker");
    assert_eq!(filter.owner, Some(PlayerFilter::You));
    assert!(filter.card_types.contains(&CardType::Creature));
}

#[test]
fn parse_target_djinn_or_efreet_includes_both_subtypes() {
    let tokens = tokenize_line("target Djinn or Efreet", 0);
    let target = parse_target_phrase(&tokens).expect("parse subtype-or target phrase");
    let TargetAst::Object(filter, _, _) = target else {
        panic!("expected object target");
    };
    assert!(
        filter.subtypes.contains(&Subtype::Djinn),
        "expected Djinn subtype in filter"
    );
    assert!(
        filter.subtypes.contains(&Subtype::Efreet),
        "expected Efreet subtype in filter"
    );
}

#[test]
fn parse_target_non_subtypes_populates_excluded_subtypes() {
    let tokens = tokenize_line("target non-Vampire, non-Werewolf, non-Zombie creature", 0);
    let target = parse_target_phrase(&tokens).expect("parse excluded subtype target");
    let TargetAst::Object(filter, _, _) = target else {
        panic!("expected object target");
    };
    assert!(
        filter.card_types.contains(&CardType::Creature),
        "expected creature type in filter"
    );
    assert!(
        filter.excluded_subtypes.contains(&Subtype::Vampire),
        "expected excluded Vampire subtype"
    );
    assert!(
        filter.excluded_subtypes.contains(&Subtype::Werewolf),
        "expected excluded Werewolf subtype"
    );
    assert!(
        filter.excluded_subtypes.contains(&Subtype::Zombie),
        "expected excluded Zombie subtype"
    );
}

#[test]
fn parse_target_non_army_creature_populates_excluded_army_subtype() {
    let tokens = tokenize_line("target non-Army creature", 0);
    let target = parse_target_phrase(&tokens).expect("parse non-Army creature target");
    let TargetAst::Object(filter, _, _) = target else {
        panic!("expected object target");
    };
    assert!(
        filter.card_types.contains(&CardType::Creature),
        "expected creature type in filter"
    );
    assert!(
        filter.excluded_subtypes.contains(&Subtype::Army),
        "expected excluded Army subtype"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_for_each_land_unless_any_player_pays_life_uses_non_target_destroy() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Cleansing Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text("For each land, destroy that land unless any player pays 1 life.")
        .expect("for-each land unless-any-player-pay-life should parse");

    let spell_debug = format!("{:?}", def.spell_effect);
    assert!(
        spell_debug.contains("ForEachObject"),
        "expected for-each lowering, got {spell_debug}"
    );
    assert!(
        spell_debug.contains("UnlessActionEffect") && spell_debug.contains("LoseLifeEffect"),
        "expected unless-action life-payment lowering, got {spell_debug}"
    );
    assert!(
        !spell_debug.contains("DestroyEffect { spec: Target("),
        "expected non-target destroy for 'destroy that land', got {spell_debug}"
    );
}
