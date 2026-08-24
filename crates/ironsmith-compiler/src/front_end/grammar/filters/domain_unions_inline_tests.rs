use super::*;
use crate::lexer::lex_line;
use crate::{CardType, ColorSet, Subtype};

#[test]
fn shared_terminal_subtype_and_color_card_is_a_typed_union() {
    let tokens = lex_line("Forest and green card", 0).unwrap();
    let filter = parse_subtype_color_shared_card_union_lexed(&tokens, false)
        .expect("shared-terminal subtype/color union");

    assert!(filter.has_explicit_card_noun(), "{filter:#?}");
    assert!(filter.has_conjunctive_set_surface(), "{filter:#?}");
    assert_eq!(filter.any_of.len(), 2, "{filter:#?}");
    assert_eq!(filter.any_of[0].subtypes, [Subtype::Forest]);
    assert_eq!(filter.any_of[1].colors, Some(ColorSet::GREEN));
    assert!(filter.any_of.iter().all(|branch| branch.zone.is_none()));
}

#[test]
fn shared_terminal_color_and_subtype_card_keeps_authored_order() {
    let tokens = lex_line("red and Mountain cards", 0).unwrap();
    let filter = parse_subtype_color_shared_card_union_lexed(&tokens, false)
        .expect("reversed shared-terminal subtype/color union");

    assert_eq!(filter.any_of[0].colors, Some(ColorSet::RED));
    assert_eq!(filter.any_of[1].subtypes, [Subtype::Mountain]);
}

fn assert_branch_local_another(filter: &ObjectFilter) {
    assert_eq!(filter.zone, Some(Zone::Battlefield));
    assert_eq!(filter.controller, Some(PlayerFilter::You));
    assert!(!filter.other);
    assert_eq!(filter.any_of.len(), 2, "{filter:#?}");

    let creature = filter
        .any_of
        .iter()
        .find(|branch| branch.card_types == [CardType::Creature])
        .expect("creature arm");
    assert!(creature.other, "{filter:#?}");

    let land = filter
        .any_of
        .iter()
        .find(|branch| branch.card_types == [CardType::Land])
        .expect("land arm");
    assert!(!land.other, "{filter:#?}");
}

#[test]
fn preserves_another_on_only_the_authored_disjunction_arm() {
    let tokens = lex_line("another creature you control or a land you control", 0).unwrap();
    let filter = parse_branch_scoped_object_filter_union_lexed(&tokens, false)
        .expect("independently nouned arms with branch-local another");

    assert_branch_local_another(&filter);
}

#[test]
fn preserves_consumed_another_on_only_the_first_disjunction_arm() {
    let tokens = lex_line("creature you control or a land you control", 0).unwrap();
    let filter = parse_branch_scoped_object_filter_union_lexed(&tokens, true)
        .expect("trigger caller's consumed another modifier");

    assert_branch_local_another(&filter);
}

#[test]
fn consumed_another_and_leading_nontoken_scope_shared_union_subject() {
    let tokens = lex_line("nontoken artifact creature or Vehicle you control", 0).unwrap();
    let filter = parse_branch_scoped_object_filter_union_lexed(&tokens, true)
        .expect("shared-suffix mixed union should parse");

    assert_eq!(filter.controller, Some(PlayerFilter::You), "{filter:#?}");
    assert!(filter.other, "{filter:#?}");
    assert!(filter.nontoken, "{filter:#?}");
    assert_eq!(filter.any_of.len(), 2, "{filter:#?}");
    assert!(filter.any_of.iter().all(|branch| {
        !branch.other && !branch.nontoken && branch.controller.is_none() && branch.zone.is_none()
    }));
    assert!(
        filter
            .any_of
            .iter()
            .any(|branch| { branch.all_card_types == [CardType::Artifact, CardType::Creature] })
    );
    assert!(
        filter
            .any_of
            .iter()
            .any(|branch| branch.subtypes == [Subtype::Vehicle])
    );
    assert_eq!(
        filter.description(),
        "another nontoken artifact creature or Vehicle you control"
    );
}

#[test]
fn parses_controlled_creature_and_owned_graveyard_card_as_domain_union() {
    let tokens = lex_line(
        "creatures you control and creature cards in your graveyard",
        0,
    )
    .unwrap();
    let filter = parse_object_filter(&tokens, false).unwrap();

    assert_eq!(filter.any_of.len(), 2);
    assert_eq!(filter.any_of[0].card_types, vec![CardType::Creature]);
    assert_eq!(filter.any_of[0].zone, Some(Zone::Battlefield));
    assert_eq!(filter.any_of[0].controller, Some(PlayerFilter::You));
    assert_eq!(filter.any_of[1].card_types, vec![CardType::Creature]);
    assert_eq!(filter.any_of[1].zone, Some(Zone::Graveyard));
    assert_eq!(filter.any_of[1].owner, Some(PlayerFilter::You));
}

#[test]
fn flattens_owned_nonbattlefield_zone_set_beside_a_controlled_battlefield_set() {
    let tokens = lex_line(
        "lands you control and land cards you own that aren't on the battlefield",
        0,
    )
    .unwrap();
    let filter = parse_branch_scoped_object_filter_union_lexed(&tokens, false)
        .expect("the two authored domains should remain independently scoped");

    assert_eq!(filter.any_of.len(), 6, "{filter:#?}");
    assert!(filter.any_of.iter().any(|branch| {
        branch.zone == Some(Zone::Battlefield)
            && branch.controller == Some(PlayerFilter::You)
            && branch.owner.is_none()
            && branch.card_types == [CardType::Land]
    }));
    for zone in [
        Zone::Hand,
        Zone::Library,
        Zone::Graveyard,
        Zone::Exile,
        Zone::Command,
    ] {
        assert!(filter.any_of.iter().any(|branch| {
            branch.zone == Some(zone)
                && branch.owner == Some(PlayerFilter::You)
                && branch.controller.is_none()
                && branch.card_types == [CardType::Land]
        }));
    }
}

#[test]
fn parses_shared_controller_type_subtype_conjunction_as_scoped_union() {
    let tokens = lex_line("Creatures and Vehicles you control", 0).unwrap();
    let filter = parse_branch_scoped_object_filter_union_lexed(&tokens, false).unwrap();

    assert_eq!(filter.zone, Some(Zone::Battlefield));
    assert_eq!(filter.controller, Some(PlayerFilter::You));
    assert!(filter.has_conjunctive_set_surface());
    assert_eq!(filter.any_of.len(), 2, "{filter:#?}");
    assert!(
        filter
            .any_of
            .iter()
            .any(|branch| branch.card_types == [CardType::Creature]),
        "{filter:#?}"
    );
    assert!(
        filter
            .any_of
            .iter()
            .any(|branch| branch.subtypes == [Subtype::Vehicle]),
        "{filter:#?}"
    );
    assert_eq!(filter.description(), "a creature and Vehicle you control");
}

#[test]
fn trailing_attachment_provenance_scopes_every_coordinated_noun() {
    for (text, relation) in [
        (
            "Auras and Equipment that were attached to it",
            TaggedOpbjectRelation::WasAttachedToTaggedObject,
        ),
        (
            "Auras and Equipment attached to it",
            TaggedOpbjectRelation::AttachedToTaggedObject,
        ),
    ] {
        let tokens = lex_line(text, 0).unwrap();
        let filter = parse_branch_scoped_object_filter_union_lexed(&tokens, false).unwrap();

        assert_eq!(filter.any_of.len(), 2, "{text}: {filter:#?}");
        assert!(
            filter.any_of.iter().all(|branch| {
                matches!(
                    branch.tagged_constraints.as_slice(),
                    [constraint]
                        if constraint.tag == crate::TagKey::from(crate::cards::builders::IT_TAG)
                            && constraint.relation == relation
                )
            }),
            "{text}: {filter:#?}"
        );
    }
}

#[test]
fn relative_characteristic_union_is_not_split_from_its_common_domain() {
    let tokens = lex_line("other creature you control that's a token or a Rabbit", 0).unwrap();

    assert!(
        contains_relative_characteristic_union(&tokens),
        "the relative selector list should be recognized lexically"
    );
    assert!(
        parse_branch_scoped_object_filter_union_lexed(&tokens, false).is_none(),
        "the shared creature/controller domain belongs outside both selector arms"
    );
}

#[test]
fn historical_block_partner_relation_is_not_split_as_an_or_union() {
    let tokens = lex_line(
        "creature that blocked or was blocked by a Zombie this turn",
        0,
    )
    .unwrap();

    assert!(contains_historical_block_partner_relation(&tokens));
    assert!(
        parse_branch_scoped_object_filter_union_lexed(&tokens, false).is_none(),
        "the Zombie is the nested combat partner, not a second object-domain arm"
    );
}

#[test]
fn current_block_partner_relation_is_not_split_as_an_or_union() {
    let tokens = lex_line("creature blocking or blocked by this creature", 0).unwrap();

    assert!(contains_current_block_partner_relation(&tokens));
    assert!(
        parse_branch_scoped_object_filter_union_lexed(&tokens, false).is_none(),
        "blocking and blocked describe the two directions of one source-relative relation"
    );
    let filter = parse_object_filter(&tokens, false).unwrap();
    assert!(filter.any_of.is_empty(), "{filter:#?}");
    assert!(filter.in_combat_with_source, "{filter:#?}");
    assert_eq!(
        filter.description(),
        "creature blocking or blocked by this creature"
    );
}

#[test]
fn shared_characteristic_comparison_or_is_not_split_as_a_domain_union() {
    let tokens = lex_line(
            "creature spell that doesn't share a creature type with a creature you control or a creature card in your graveyard",
            0,
        )
        .unwrap();

    assert!(contains_shared_characteristic_comparison_union(&tokens));
    assert!(
        parse_branch_scoped_object_filter_union_lexed(&tokens, false).is_none(),
        "the inner `or` joins comparison domains, not spell-filter branches"
    );
    assert!(
        parse_domain_union_object_filter_lexed(&tokens, false).is_none(),
        "the graveyard comparison must not become the spell's origin domain"
    );
}

#[test]
fn heterogeneous_stack_battlefield_graveyard_list_keeps_three_domains() {
    for branch_text in ["spell", "nonland permanent", "card in a graveyard"] {
        let branch_tokens = lex_line(branch_text, 0).unwrap();
        let branch = parse_object_filter(&branch_tokens, false).unwrap();
        assert!(
            branch_has_explicit_object_selector(&branch),
            "{branch_text}: {branch:#?}"
        );
    }
    let tokens = lex_line("spell, nonland permanent, or card in a graveyard", 0).unwrap();
    let filter = parse_branch_scoped_object_filter_union_lexed(&tokens, false)
        .expect("each independently zoned target arm should remain in the union");

    assert_eq!(filter.zone, None);
    assert_eq!(filter.any_of.len(), 3, "{filter:#?}");
    assert!(filter.any_of.iter().any(|branch| {
        branch.zone == Some(Zone::Stack)
            && branch.stack_kind == Some(crate::filter::StackObjectKind::Spell)
    }));
    assert!(filter.any_of.iter().any(|branch| {
        branch.zone == Some(Zone::Battlefield) && branch.excluded_card_types == [CardType::Land]
    }));
    assert!(
        filter
            .any_of
            .iter()
            .any(|branch| branch.zone == Some(Zone::Graveyard))
    );
}

#[test]
fn shared_controller_card_type_list_defers_to_flat_filter_grammar() {
    let tokens = lex_line(
        "an artifact, creature, land, or planeswalker you control",
        0,
    )
    .unwrap();

    assert!(
        parse_branch_scoped_object_filter_union_lexed(&tokens, false).is_none(),
        "a common controller does not make otherwise bare card types branch-local"
    );
    let filter = parse_object_filter(&tokens, false).unwrap();
    assert!(filter.any_of.is_empty(), "{filter:#?}");
    assert_eq!(
        filter.card_types,
        [
            CardType::Artifact,
            CardType::Creature,
            CardType::Land,
            CardType::Planeswalker,
        ]
    );
    assert_eq!(filter.controller, Some(PlayerFilter::You));
    assert_eq!(
        filter.description(),
        "an artifact, creature, land, or planeswalker you control"
    );
}

#[test]
fn mirrored_owner_or_controller_scope_keeps_one_shared_object_noun() {
    let tokens = lex_line("permanent you own or control", 0).unwrap();
    let filter =
        crate::grammar::filters::parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false)
            .unwrap();

    assert_eq!(filter.any_of.len(), 2, "{filter:#?}");
    assert_eq!(filter.description(), "a permanent you own or control");
}

#[test]
fn nontoken_comma_modifier_is_not_a_standalone_union_arm() {
    let tokens = lex_line("a nontoken, non-Angel creature you control", 0).unwrap();

    assert!(
        parse_branch_scoped_object_filter_union_lexed(&tokens, false).is_none(),
        "`nontoken` is an adjective modifying the shared creature noun"
    );
    let filter = parse_object_filter(&tokens, false).unwrap();
    assert!(filter.any_of.is_empty(), "{filter:#?}");
    assert!(filter.nontoken);
    assert_eq!(filter.excluded_subtypes, [Subtype::Angel]);
    assert_eq!(filter.card_types, [CardType::Creature]);
    assert_eq!(filter.controller, Some(PlayerFilter::You));
    assert_eq!(
        filter.description(),
        "a nontoken non-angel creature you control"
    );
}

#[test]
fn preserves_equipped_state_on_only_its_conjunctive_union_arm() {
    let tokens = lex_line("equipped creatures and Equipment you control", 0).unwrap();
    let filter = parse_branch_scoped_object_filter_union_lexed(&tokens, false).unwrap();

    assert_eq!(filter.controller, Some(PlayerFilter::You));
    assert!(filter.has_conjunctive_set_surface());
    let equipped_creature = filter
        .any_of
        .iter()
        .find(|branch| branch.card_types == [CardType::Creature])
        .expect("equipped creature branch");
    assert!(
        equipped_creature
            .tagged_constraints
            .iter()
            .any(|constraint| {
                constraint.tag.as_str() == "equipped"
                    && constraint.relation == crate::TaggedOpbjectRelation::IsTaggedObject
            })
    );
    let equipment = filter
        .any_of
        .iter()
        .find(|branch| branch.subtypes == [Subtype::Equipment])
        .expect("Equipment branch");
    assert!(equipment.tagged_constraints.is_empty(), "{equipment:#?}");
    assert_eq!(
        filter.description(),
        "an equipped creature and Equipment you control"
    );
}

#[test]
fn preserves_branch_local_combat_state_and_controllers_in_or_union() {
    let tokens = lex_line(
        "an attacking creature you control or a blocking creature an opponent controls",
        0,
    )
    .unwrap();
    let filter = parse_branch_scoped_object_filter_union_lexed(&tokens, false).unwrap();

    assert_eq!(filter.zone, Some(Zone::Battlefield));
    assert_eq!(filter.controller, None);
    assert_eq!(filter.any_of.len(), 2, "{filter:#?}");
    assert!(filter.any_of.iter().any(|branch| {
        branch.attacking && branch.controller == Some(PlayerFilter::You) && !branch.blocking
    }));
    assert!(filter.any_of.iter().any(|branch| {
        branch.blocking && branch.controller == Some(PlayerFilter::Opponent) && !branch.attacking
    }));
    assert_eq!(
        filter.description(),
        "an attacking creature you control or a blocking creature an opponent controls"
    );
}

#[test]
fn comma_list_preserves_internal_ownership_conjunction_and_attachment_scopes() {
    let tokens = lex_line(
            "enchantments you both own and control, all Auras you own attached to permanents you control, and all Auras you own attached to attacking creatures your opponents control",
            0,
        )
        .unwrap();
    let filter = parse_branch_scoped_object_filter_union_lexed(&tokens, false).unwrap();

    assert_eq!(filter.zone, Some(Zone::Battlefield));
    assert_eq!(filter.owner, Some(PlayerFilter::You));
    assert_eq!(filter.controller, None);
    assert!(filter.has_conjunctive_set_surface());
    assert_eq!(filter.any_of.len(), 3, "{filter:#?}");

    let enchantments = filter
        .any_of
        .iter()
        .find(|branch| branch.card_types == [CardType::Enchantment])
        .expect("enchantment branch");
    assert_eq!(enchantments.controller, Some(PlayerFilter::You));
    assert_eq!(enchantments.owner, None);

    let aura_branches = filter
        .any_of
        .iter()
        .filter(|branch| branch.subtypes == [Subtype::Aura])
        .collect::<Vec<_>>();
    assert_eq!(aura_branches.len(), 2, "{filter:#?}");
    let controlled_host = aura_branches
        .iter()
        .find(|branch| {
            branch
                .attached_to_object
                .as_deref()
                .is_some_and(|host| !host.attacking && host.controller == Some(PlayerFilter::You))
        })
        .expect("Aura attached to a controlled permanent");
    assert_eq!(controlled_host.owner, None);
    assert!(
        controlled_host
            .attached_to_object
            .as_deref()
            .is_some_and(ObjectFilter::has_plural_object_noun_surface)
    );

    let opposing_attacker = aura_branches
        .iter()
        .filter_map(|branch| branch.attached_to_object.as_deref())
        .find(|host| host.attacking && host.controller == Some(PlayerFilter::Opponent));
    assert!(opposing_attacker.is_some(), "{filter:#?}");
}

#[test]
fn comma_list_keeps_other_and_controller_scope_on_each_destroy_branch() {
    let tokens = lex_line(
            "other enchantments you control, all other Auras attached to permanents you control, and all other Auras attached to attacking creatures your opponents control",
            0,
        )
        .unwrap();
    let filter = parse_branch_scoped_object_filter_union_lexed(&tokens, false).unwrap();

    assert_eq!(filter.zone, Some(Zone::Battlefield));
    assert_eq!(filter.owner, None);
    assert_eq!(filter.controller, None);
    assert!(filter.has_conjunctive_set_surface());
    assert_eq!(filter.any_of.len(), 3, "{filter:#?}");
    assert!(filter.any_of.iter().all(|branch| branch.other));

    let enchantments = filter
        .any_of
        .iter()
        .find(|branch| branch.card_types == [CardType::Enchantment])
        .expect("other enchantment branch");
    assert_eq!(enchantments.controller, Some(PlayerFilter::You));

    assert!(filter.any_of.iter().any(|branch| {
        branch.subtypes == [Subtype::Aura]
            && branch
                .attached_to_object
                .as_deref()
                .is_some_and(|host| !host.attacking && host.controller == Some(PlayerFilter::You))
    }));
    assert!(filter.any_of.iter().any(|branch| {
        branch.subtypes == [Subtype::Aura]
            && branch.attached_to_object.as_deref().is_some_and(|host| {
                host.attacking && host.controller == Some(PlayerFilter::Opponent)
            })
    }));
}

#[test]
fn preserves_arm_local_other_qualifier_for_subtype_domain_union() {
    let tokens = lex_line(
        "other Dragons you control and Dragon cards in your graveyard",
        0,
    )
    .unwrap();
    let filter = parse_domain_union_object_filter_lexed(&tokens, false).unwrap();

    assert_eq!(filter.any_of.len(), 2);
    assert_eq!(filter.any_of[0].subtypes, vec![Subtype::Dragon]);
    assert!(filter.any_of[0].other);
    assert_eq!(filter.any_of[0].controller, Some(PlayerFilter::You));
    assert_eq!(filter.any_of[1].subtypes, vec![Subtype::Dragon]);
    assert!(!filter.any_of[1].other);
    assert_eq!(filter.any_of[1].zone, Some(Zone::Graveyard));
    assert_eq!(filter.any_of[1].owner, Some(PlayerFilter::You));
}

#[test]
fn parses_and_or_scoped_domains_as_one_semantic_union() {
    let tokens = lex_line(
        "creatures you control and/or creature cards in your graveyard",
        0,
    )
    .unwrap();
    let filter = parse_domain_union_object_filter_lexed(&tokens, false).unwrap();

    assert_eq!(filter.any_of.len(), 2);
    assert_eq!(
        filter.union_connective(),
        ObjectFilterUnionConnective::AndOr
    );
    assert_eq!(filter.any_of[0].zone, Some(Zone::Battlefield));
    assert_eq!(filter.any_of[1].zone, Some(Zone::Graveyard));
}

#[test]
fn parses_repeated_each_quantifiers_across_scoped_domains() {
    let tokens = lex_line("Caves you control and each Cave card in your graveyard", 0).unwrap();
    let filter = parse_domain_union_object_filter_lexed(&tokens, false).unwrap();

    assert_eq!(filter.any_of.len(), 2);
    assert_eq!(filter.any_of[0].subtypes, vec![Subtype::Cave]);
    assert_eq!(filter.any_of[0].zone, Some(Zone::Battlefield));
    assert_eq!(filter.any_of[0].controller, Some(PlayerFilter::You));
    assert_eq!(filter.any_of[1].subtypes, vec![Subtype::Cave]);
    assert_eq!(filter.any_of[1].zone, Some(Zone::Graveyard));
    assert_eq!(filter.any_of[1].owner, Some(PlayerFilter::You));
}

#[test]
fn repeated_card_nouns_share_the_trailing_graveyard_domain() {
    let tokens = lex_line(
        "Assassin card or card with freerunning from your graveyard",
        0,
    )
    .unwrap();
    let filter = parse_branch_scoped_object_filter_union_lexed(&tokens, false).unwrap();

    assert_eq!(filter.zone, Some(Zone::Graveyard), "{filter:#?}");
    assert_eq!(filter.owner, Some(PlayerFilter::You), "{filter:#?}");
    assert_eq!(filter.any_of.len(), 2, "{filter:#?}");
    assert!(
        filter
            .any_of
            .iter()
            .any(|branch| branch.subtypes == [Subtype::Assassin]),
        "{filter:#?}"
    );
    assert!(
        filter.any_of.iter().any(|branch| branch
            .ability_markers
            .iter()
            .any(|marker| marker == "freerunning")),
        "{filter:#?}"
    );
}

#[test]
fn permanent_arm_does_not_inherit_a_trailing_graveyard_card_domain() {
    let tokens = lex_line("a permanent you control or a card from your graveyard", 0).unwrap();
    let filter = parse_branch_scoped_object_filter_union_lexed(&tokens, false).unwrap();

    assert_eq!(filter.zone, None, "{filter:#?}");
    assert!(filter.any_of.iter().any(|branch| {
        branch.zone == Some(Zone::Battlefield) && branch.controller == Some(PlayerFilter::You)
    }));
    assert!(filter.any_of.iter().any(|branch| {
        branch.zone == Some(Zone::Graveyard) && branch.owner == Some(PlayerFilter::You)
    }));
}

#[test]
fn does_not_reinterpret_different_object_selectors_as_domain_union() {
    let tokens = lex_line("artifacts and creatures you control", 0).unwrap();
    assert!(parse_domain_union_object_filter_lexed(&tokens, false).is_none());
}

#[test]
fn parses_elided_owned_selector_across_exile_and_graveyard() {
    let tokens = lex_line(
            "cards you own in exile and in your graveyard that are instant cards, are sorcery cards, and/or have an Adventure",
            0,
        )
        .unwrap();
    let filter = parse_domain_union_object_filter_lexed(&tokens, false).unwrap();

    assert_eq!(filter.owner, Some(PlayerFilter::You));
    assert_eq!(
        filter.card_types,
        vec![crate::CardType::Instant, crate::CardType::Sorcery]
    );
    assert_eq!(filter.subtypes, vec![Subtype::Adventure]);
    assert!(filter.type_or_subtype_union);
    assert_eq!(filter.any_of.len(), 2);
    assert_eq!(filter.any_of[0].zone, Some(Zone::Exile));
    assert_eq!(filter.any_of[1].zone, Some(Zone::Graveyard));
    assert_eq!(
        domain_selector_signature(&filter.any_of[0]),
        Some(ObjectFilter::default())
    );
    assert_eq!(
        domain_selector_signature(&filter.any_of[1]),
        Some(ObjectFilter::default())
    );
}

#[test]
fn parses_elided_owned_selector_with_repeated_card_alternatives() {
    let tokens = lex_line(
            "card you own in exile and in your graveyard that's an instant card, a sorcery card, or a card that has an Adventure",
            0,
        )
        .unwrap();
    let filter = crate::object_filters::parse_object_filter_lexed(&tokens, false).unwrap();

    assert_eq!(filter.owner, Some(PlayerFilter::You));
    assert_eq!(
        filter.card_types,
        vec![crate::CardType::Instant, crate::CardType::Sorcery]
    );
    assert_eq!(filter.subtypes, vec![Subtype::Adventure]);
    assert!(filter.type_or_subtype_union);
    assert_eq!(filter.any_of.len(), 2);
    assert_eq!(filter.any_of[0].zone, Some(Zone::Exile));
    assert_eq!(filter.any_of[1].zone, Some(Zone::Graveyard));
    assert!(
        filter.any_of.iter().all(|branch| branch.owner.is_none()),
        "{filter:#?}"
    );
}
