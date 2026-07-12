use super::*;
use crate::card::{CardBuilder, PowerToughness};
use crate::cards::builders::CardDefinitionBuilder;
use crate::color::Color;
use crate::filter::StackObjectKind;
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::object::AttachmentTarget;
use crate::target::PlayerFilter;
use crate::types::{Subtype, Supertype};
use crate::zone::Zone;

#[test]
fn test_anthem() {
    let anthem = Anthem::creatures_you_control(1, 1);
    assert_eq!(anthem.id(), StaticAbilityId::Anthem);
    assert!(anthem.is_anthem());
    assert_eq!(anthem.display(), "creatures you control get +1/+1");
}

#[test]
fn test_remove_supertypes_display_mentions_scope_and_supertype() {
    let remove = RemoveSupertypesForFilter::new(ObjectFilter::land(), vec![Supertype::Snow]);
    assert_eq!(remove.display(), "All lands are no longer snow");
}

#[test]
fn test_add_card_types_display_pluralizes_compound_spell_subjects() {
    let filter = ObjectFilter {
        zone: Some(Zone::Stack),
        controller: Some(PlayerFilter::You),
        stack_kind: Some(StackObjectKind::Spell),
        has_mana_cost: true,
        card_types: vec![
            CardType::Artifact,
            CardType::Creature,
            CardType::Enchantment,
            CardType::Land,
            CardType::Planeswalker,
            CardType::Battle,
        ],
        ..Default::default()
    };

    let add = AddCardTypesForFilter::new(filter, vec![CardType::Artifact]);
    assert_eq!(
        add.display(),
        "permanent spells you control are artifacts in addition to their other types"
    );
}

#[test]
fn describe_static_condition_displays_half_starting_life_total() {
    assert_eq!(
        describe_static_condition(
            &crate::ConditionExpr::PlayerLifeAtMostHalfStartingLifeTotal {
                player: PlayerFilter::You,
            }
        ),
        "as long as your life total is less than or equal to half your starting life total"
    );
}

#[test]
fn describe_static_condition_displays_devotion_value_comparison() {
    assert_eq!(
        describe_static_condition(&crate::ConditionExpr::ValueComparison {
            left: Value::Add(
                Box::new(Value::Devotion {
                    player: PlayerFilter::You,
                    color: Color::Black,
                }),
                Box::new(Value::Devotion {
                    player: PlayerFilter::You,
                    color: Color::Red,
                }),
            ),
            operator: crate::effect::ValueComparisonOperator::LessThan,
            right: Value::Fixed(7),
        }),
        "as long as your devotion to black and red is less than seven"
    );
}

#[test]
fn describe_static_condition_displays_opponent_life_threshold() {
    assert_eq!(
        describe_static_condition(&crate::ConditionExpr::ValueComparison {
            left: Value::LifeTotal(PlayerFilter::Opponent),
            operator: crate::effect::ValueComparisonOperator::LessThanOrEqual,
            right: Value::Fixed(10),
        }),
        "as long as an opponent has 10 or less life"
    );
}

#[test]
fn describe_static_condition_displays_equipped_subtype_match() {
    assert_eq!(
        describe_static_condition(&crate::ConditionExpr::TaggedObjectMatches(
            crate::TagKey::from("equipped"),
            ObjectFilter::default().with_subtype(Subtype::Human),
        )),
        "as long as equipped creature is a human"
    );
}

#[test]
fn remove_card_types_display_keeps_source_creature_subject() {
    let remove = RemoveCardTypesForFilter::new(
        ObjectFilter::source().with_type(CardType::Creature),
        vec![CardType::Creature],
    )
    .with_condition(crate::ConditionExpr::ValueComparison {
        left: Value::Add(
            Box::new(Value::Devotion {
                player: PlayerFilter::You,
                color: Color::Black,
            }),
            Box::new(Value::Devotion {
                player: PlayerFilter::You,
                color: Color::Red,
            }),
        ),
        operator: crate::effect::ValueComparisonOperator::LessThan,
        right: Value::Fixed(7),
    });
    assert_eq!(
        remove.display(),
        "this card isn't a creature as long as your devotion to black and red is less than seven"
    );
}

#[test]
fn test_add_all_subtypes_display_pluralizes_compound_card_subjects() {
    let filter = ObjectFilter {
        zone: Some(Zone::Hand),
        owner: Some(PlayerFilter::You),
        card_types: vec![CardType::Creature],
        ..Default::default()
    };

    let add = AddAllSubtypesOfFamilyForFilter::new(filter, SubtypeFamily::Creature);
    assert_eq!(
        add.display(),
        "creature cards in your hand are every creature type"
    );
}

#[test]
fn test_anthem_generates_effects() {
    let anthem = Anthem::creatures_you_control(2, 2);
    let game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let source = ObjectId::from_raw(1);
    let controller = PlayerId::from_index(0);

    let effects = anthem.generate_effects(source, controller, &game);
    assert_eq!(effects.len(), 1);
    assert!(matches!(
        effects[0].modification,
        Modification::ModifyPowerToughness {
            power: 2,
            toughness: 2
        }
    ));
}

#[test]
fn test_attached_anthem_uses_attached_target() {
    let mut filter = ObjectFilter::creature();
    filter
        .tagged_constraints
        .push(crate::filter::TaggedObjectConstraint {
            tag: crate::tag::TagKey::from("enchanted"),
            relation: crate::filter::TaggedOpbjectRelation::IsTaggedObject,
        });
    let anthem = Anthem::new(filter, 1, 1);
    assert_eq!(anthem.display(), "enchanted creature gets +1/+1");

    let game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let source = ObjectId::from_raw(1);
    let controller = PlayerId::from_index(0);
    let effects = anthem.generate_effects(source, controller, &game);
    assert_eq!(effects.len(), 1);
    assert!(matches!(
        effects[0].applies_to,
        EffectTarget::AttachedTo(id) if id == source
    ));
}

#[test]
fn test_source_dynamic_anthem_scales_from_filter_count() {
    let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);

    let source_card = CardBuilder::new(CardId::new(), "Nim Lasher")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();
    let source = game.create_object_from_card(&source_card, alice, Zone::Battlefield);

    let artifact_card = CardBuilder::new(CardId::new(), "Myr Token")
        .card_types(vec![CardType::Artifact])
        .build();
    game.create_object_from_card(&artifact_card, alice, Zone::Battlefield);
    game.create_object_from_card(&artifact_card, alice, Zone::Battlefield);

    let anthem = Anthem::for_source(0, 0).with_values(
        AnthemValue::scaled(
            1,
            AnthemCountExpression::MatchingFilter(ObjectFilter::artifact().you_control()),
        ),
        AnthemValue::Fixed(0),
    );

    let effects = anthem.generate_effects(source, alice, &game);
    assert_eq!(effects.len(), 1);
    assert!(matches!(
        effects[0].modification,
        Modification::ModifyPowerToughness {
            power: 2,
            toughness: 0
        }
    ));
    assert!(matches!(effects[0].applies_to, EffectTarget::Source));
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn test_commander_cast_count_anthem_scales_from_player_commander_casts() {
    let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);

    let insignia = crate::cards::builders::CardDefinitionBuilder::new(
            CardId::new(),
            "Commander's Insignia Variant",
        )
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "Creatures you control get +1/+1 for each time you've cast your commander from the command zone this game.",
        )
        .expect("Commander's Insignia text should parse");
    game.create_object_from_definition(&insignia, alice, Zone::Battlefield);

    let commander_card = CardBuilder::new(CardId::new(), "Commander")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let commander_id = game.create_object_from_card(&commander_card, alice, Zone::Command);
    game.set_as_commander(commander_id, alice);

    let creature_card = CardBuilder::new(CardId::new(), "Insignia Bearer")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();
    let creature_id = game.create_object_from_card(&creature_card, alice, Zone::Battlefield);

    assert_eq!(
        game.calculated_power(creature_id),
        Some(1),
        "the anthem should start at +0/+0 before commander casts"
    );
    assert_eq!(
        game.commander_cast_count_for_player(alice),
        0,
        "fresh commanders should start with zero command-zone casts"
    );

    game.record_commander_cast_from_command_zone(commander_id);
    game.record_commander_cast_from_command_zone(commander_id);

    assert_eq!(
        game.commander_cast_count_for_player(alice),
        2,
        "the player's commander cast count should include repeated command-zone casts"
    );
    assert_eq!(
        game.calculated_power(creature_id),
        Some(3),
        "Commander's Insignia should grant +1/+1 per commander cast from the command zone"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn test_bonehoard_counts_creature_cards_in_all_graveyards() {
    use crate::cards::builders::CardDefinitionBuilder;
    use crate::ids::CardId;
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::object::AttachmentTarget;

    let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let bonehoard = CardDefinitionBuilder::new(CardId::new(), "Bonehoard Variant")
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(4)]]))
            .card_types(vec![CardType::Artifact])
            .subtypes(vec![Subtype::Equipment])
            .parse_text(
                "Living weapon (When this Equipment enters, create a 0/0 black Phyrexian Germ creature token, then attach this to it.)\n\
                 Equipped creature gets +X/+X, where X is the number of creature cards in all graveyards.\n\
                 Equip {2}",
            )
            .expect("Bonehoard text should parse");

    let equipment_id = game.create_object_from_definition(&bonehoard, alice, Zone::Battlefield);

    let creature_card = CardBuilder::new(CardId::new(), "Carried Creature")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();
    let creature_id = game.create_object_from_card(&creature_card, alice, Zone::Battlefield);
    game.object_mut(equipment_id).unwrap().attached_to =
        Some(AttachmentTarget::Object(creature_id));
    game.object_mut(creature_id)
        .unwrap()
        .attachments
        .push(equipment_id);

    let graveyard_card_a = CardBuilder::new(CardId::new(), "Graveyard Creature A")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();
    game.create_object_from_card(&graveyard_card_a, alice, Zone::Graveyard);

    let graveyard_card_b = CardBuilder::new(CardId::new(), "Graveyard Creature B")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    game.create_object_from_card(&graveyard_card_b, bob, Zone::Graveyard);

    assert_eq!(
        game.calculated_power(creature_id),
        Some(3),
        "Bonehoard should count creature cards in both graveyards"
    );
    assert_eq!(
        game.calculated_toughness(creature_id),
        Some(3),
        "Bonehoard should count creature cards in both graveyards"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn test_kembas_banner_counts_creatures_you_control_for_each() {
    use crate::cards::builders::CardDefinitionBuilder;
    use crate::ids::CardId;
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::object::AttachmentTarget;

    let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);

    let kembas_banner = CardDefinitionBuilder::new(CardId::new(), "Kemba's Banner Variant")
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(3)],
                vec![ManaSymbol::White],
            ]))
            .card_types(vec![CardType::Artifact])
            .subtypes(vec![Subtype::Equipment])
            .parse_text(
                "For Mirrodin! (When this Equipment enters, create a 2/2 red Rebel creature token, then attach this to it.)\n\
                 Equipped creature gets +1/+1 for each creature you control.\n\
                 Equip {2}{W}",
            )
            .expect("Kemba's Banner text should parse");

    let compiled = crate::compiled_text::unprocessed_compiled_lines(&kembas_banner)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        compiled.contains("equipped creature gets +1/+1 for each creature you control"),
        "expected Kemba's Banner to render the for-each anthem wording, got {compiled}"
    );

    let equipment_id = game.create_object_from_definition(&kembas_banner, alice, Zone::Battlefield);

    let bearer = CardBuilder::new(CardId::new(), "Bearer")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();
    let bearer_id = game.create_object_from_card(&bearer, alice, Zone::Battlefield);

    let ally = CardBuilder::new(CardId::new(), "Ally")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();
    game.create_object_from_card(&ally, alice, Zone::Battlefield);

    game.object_mut(equipment_id).unwrap().attached_to = Some(AttachmentTarget::Object(bearer_id));
    game.object_mut(bearer_id)
        .unwrap()
        .attachments
        .push(equipment_id);

    assert_eq!(
        game.calculated_power(bearer_id),
        Some(3),
        "Kemba's Banner should count both creatures you control while attached"
    );
    assert_eq!(
        game.calculated_toughness(bearer_id),
        Some(3),
        "Kemba's Banner should apply the same bonus to toughness"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn test_cranial_ram_counts_artifacts_for_x_only_bonus() {
    let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);

    let cranial_ram = CardDefinitionBuilder::new(CardId::new(), "Cranial Ram Variant")
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Black],
                vec![ManaSymbol::Red],
            ]))
            .card_types(vec![CardType::Artifact])
            .subtypes(vec![Subtype::Equipment])
            .parse_text(
                "Living weapon (When this Equipment enters, create a 0/0 black Phyrexian Germ creature token, then attach this to it.)\n\
                 Equipped creature gets +X/+1, where X is the number of artifacts you control.\n\
                 Equip {2}",
            )
            .expect("Cranial Ram text should parse");

    let equipment_id = game.create_object_from_definition(&cranial_ram, alice, Zone::Battlefield);

    let bearer = CardBuilder::new(CardId::new(), "Bearer")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();
    let bearer_id = game.create_object_from_card(&bearer, alice, Zone::Battlefield);

    let extra_artifact = CardBuilder::new(CardId::new(), "Artifact")
        .card_types(vec![CardType::Artifact])
        .build();
    game.create_object_from_card(&extra_artifact, alice, Zone::Battlefield);

    game.object_mut(equipment_id).unwrap().attached_to = Some(AttachmentTarget::Object(bearer_id));
    game.object_mut(bearer_id)
        .unwrap()
        .attachments
        .push(equipment_id);

    assert_eq!(
        game.calculated_power(bearer_id),
        Some(3),
        "Cranial Ram should count artifacts you control for power"
    );
    assert_eq!(
        game.calculated_toughness(bearer_id),
        Some(2),
        "Cranial Ram should keep toughness at +1"
    );
}

#[test]
fn test_conditional_anthem_is_active_only_when_condition_matches() {
    let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);

    let source_card = CardBuilder::new(CardId::new(), "Ardent Recruit")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();
    let source = game.create_object_from_card(&source_card, alice, Zone::Battlefield);

    let artifact_card = CardBuilder::new(CardId::new(), "Myr Token")
        .card_types(vec![CardType::Artifact])
        .build();
    game.create_object_from_card(&artifact_card, alice, Zone::Battlefield);
    game.create_object_from_card(&artifact_card, alice, Zone::Battlefield);

    let anthem = Anthem::for_source(2, 2).with_condition(crate::ConditionExpr::CountComparison {
        count: AnthemCountExpression::MatchingFilter(ObjectFilter::artifact().you_control()),
        comparison: Comparison::GreaterThanOrEqual(3),
        display: Some("you control three or more artifacts".to_string()),
    });

    assert!(
        !anthem.is_active(&game, source),
        "condition should be false with only two artifacts"
    );

    game.create_object_from_card(&artifact_card, alice, Zone::Battlefield);
    assert!(
        anthem.is_active(&game, source),
        "condition should be true with three artifacts"
    );
}

#[test]
fn test_domain_count_expression_counts_distinct_basic_land_types() {
    let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);

    let source_card = CardBuilder::new(CardId::new(), "Kavu Scout")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(0, 2))
        .build();
    let source = game.create_object_from_card(&source_card, alice, Zone::Battlefield);

    let plains = CardBuilder::new(CardId::new(), "Plains")
        .card_types(vec![CardType::Land])
        .subtypes(vec![Subtype::Plains])
        .build();
    let forest = CardBuilder::new(CardId::new(), "Forest")
        .card_types(vec![CardType::Land])
        .subtypes(vec![Subtype::Forest])
        .build();
    let second_plains = CardBuilder::new(CardId::new(), "Snow Plains")
        .card_types(vec![CardType::Land])
        .subtypes(vec![Subtype::Plains])
        .build();

    game.create_object_from_card(&plains, alice, Zone::Battlefield);
    game.create_object_from_card(&forest, alice, Zone::Battlefield);
    game.create_object_from_card(&second_plains, alice, Zone::Battlefield);

    let anthem = Anthem::for_source(0, 0).with_values(
        AnthemValue::scaled(
            1,
            AnthemCountExpression::BasicLandTypesAmong(ObjectFilter::land().you_control()),
        ),
        AnthemValue::Fixed(0),
    );
    let effects = anthem.generate_effects(source, alice, &game);
    assert_eq!(effects.len(), 1);
    assert!(matches!(
        effects[0].modification,
        Modification::ModifyPowerToughness {
            power: 2,
            toughness: 0
        }
    ));
}

#[test]
fn test_anthem_count_expression_counts_distinct_creature_types() {
    let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let source_card = CardBuilder::new(CardId::new(), "Kindred Scout")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Elf])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();
    let source = game.create_object_from_card(&source_card, alice, Zone::Battlefield);

    let warrior = CardBuilder::new(CardId::new(), "Warrior Ally")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Warrior])
        .build();
    let second_elf = CardBuilder::new(CardId::new(), "Second Elf")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Elf])
        .build();
    let goblin = CardBuilder::new(CardId::new(), "Opponent Goblin")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Goblin])
        .build();

    game.create_object_from_card(&warrior, alice, Zone::Battlefield);
    game.create_object_from_card(&second_elf, alice, Zone::Battlefield);
    game.create_object_from_card(&goblin, bob, Zone::Battlefield);

    let anthem = Anthem::for_source(0, 0).with_values(
        AnthemValue::scaled(
            1,
            AnthemCountExpression::CreatureTypesAmong(ObjectFilter::creature().you_control()),
        ),
        AnthemValue::Fixed(0),
    );
    let effects = anthem.generate_effects(source, alice, &game);
    assert_eq!(effects.len(), 1);
    assert!(matches!(
        effects[0].modification,
        Modification::ModifyPowerToughness {
            power: 2,
            toughness: 0
        }
    ));
}

#[test]
fn test_set_land_subtypes() {
    let nonbasic_land_filter = ObjectFilter {
        zone: Some(Zone::Battlefield),
        card_types: vec![CardType::Land],
        excluded_supertypes: vec![Supertype::Basic],
        ..Default::default()
    };
    let ability = SetLandSubtypesForFilter::new(nonbasic_land_filter, vec![Subtype::Mountain]);
    assert_eq!(ability.id(), StaticAbilityId::SetLandSubtypes);
    assert_eq!(ability.display(), "nonbasic lands are mountains");
}

#[test]
fn test_set_land_subtypes_generates_type_and_ability_effects() {
    let nonbasic_land_filter = ObjectFilter {
        zone: Some(Zone::Battlefield),
        card_types: vec![CardType::Land],
        excluded_supertypes: vec![Supertype::Basic],
        ..Default::default()
    };
    let ability = SetLandSubtypesForFilter::new(nonbasic_land_filter, vec![Subtype::Mountain]);
    let game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let source = ObjectId::from_raw(1);
    let controller = PlayerId::from_index(0);

    let effects = ability.generate_effects(source, controller, &game);
    assert_eq!(effects.len(), 2);
}

#[test]
fn test_grant_ability() {
    let grant = GrantAbility::new(
        ObjectFilter::creature().you_control(),
        StaticAbility::flying(),
    );
    assert_eq!(grant.id(), StaticAbilityId::GrantAbility);
    assert!(grant.grants_abilities());
    assert_eq!(grant.display(), "creatures you control have flying");
}

#[test]
fn soulbond_shared_bonus_generation_does_not_reenter_characteristics() {
    let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);

    let soulbond_card = CardBuilder::new(CardId::new(), "Trusted Forcemage")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let partner_card = CardBuilder::new(CardId::new(), "Paired Creature")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();

    let source = game.create_object_from_card(&soulbond_card, alice, Zone::Battlefield);
    game.object_mut(source)
        .expect("source should exist")
        .abilities_mut()
        .push(Ability::static_ability(
            StaticAbility::soulbond_shared_power_toughness(1, 1),
        ));
    let partner = game.create_object_from_card(&partner_card, alice, Zone::Battlefield);
    game.set_soulbond_pair(source, partner);

    assert_eq!(game.soulbond_partner(source), Some(partner));
    assert_eq!(game.calculated_power(source), Some(3));
    assert_eq!(game.calculated_power(partner), Some(3));
}

#[test]
fn test_grant_ability_displays_spell_subjects_with_cast_and_origin() {
    let mut filter = ObjectFilter::default();
    filter.has_mana_cost = true;
    filter.zone = Some(Zone::Hand);
    filter.cast_by = Some(crate::target::PlayerFilter::You);
    filter.card_types.push(CardType::Enchantment);
    let grant = GrantAbility::new(filter, StaticAbility::cascade());

    assert_eq!(
        grant.display(),
        "enchantment spells you cast from your hand have cascade"
    );
}

#[test]
fn test_attached_grant_ability_uses_attached_target() {
    let mut filter = ObjectFilter::creature();
    filter
        .tagged_constraints
        .push(crate::filter::TaggedObjectConstraint {
            tag: crate::tag::TagKey::from("equipped"),
            relation: crate::filter::TaggedOpbjectRelation::IsTaggedObject,
        });
    let grant = GrantAbility::new(filter, StaticAbility::trample());
    assert_eq!(grant.display(), "equipped creature has trample");

    let game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let source = ObjectId::from_raw(1);
    let controller = PlayerId::from_index(0);
    let effects = grant.generate_effects(source, controller, &game);
    assert_eq!(effects.len(), 1);
    assert!(matches!(
        effects[0].applies_to,
        EffectTarget::AttachedTo(id) if id == source
    ));
}

#[test]
fn test_control_attached_permanent_changes_controller() {
    let ability = ControlAttachedPermanent::new("You control enchanted creature.".to_string());
    assert_eq!(ability.id(), StaticAbilityId::ControlAttachedPermanent);
    assert_eq!(ability.display(), "You control enchanted creature.");

    let game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let source = ObjectId::from_raw(1);
    let controller = PlayerId::from_index(0);
    let effects = ability.generate_effects(source, controller, &game);
    assert_eq!(effects.len(), 1);
    assert!(matches!(
        effects[0].applies_to,
        EffectTarget::AttachedTo(id) if id == source
    ));
    assert!(matches!(
        effects[0].modification,
        Modification::ChangeController(player) if player == controller
    ));
}

#[test]
fn test_equipment_grant() {
    let grant = EquipmentGrant::new(vec![StaticAbility::haste(), StaticAbility::shroud()]);
    assert_eq!(grant.id(), StaticAbilityId::EquipmentGrant);
    assert!(grant.grants_abilities());
    assert!(grant.display().contains("Haste"));
    assert!(grant.display().contains("Shroud"));
}

#[test]
fn test_remove_all_abilities_for_filter() {
    let ability = RemoveAllAbilitiesForFilter::new(ObjectFilter::creature());
    assert_eq!(ability.id(), StaticAbilityId::RemoveAllAbilitiesForFilter);

    let game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let effects = ability.generate_effects(ObjectId::from_raw(1), PlayerId::from_index(0), &game);
    assert_eq!(effects.len(), 1);
    assert!(matches!(
        effects[0].modification,
        Modification::RemoveAllAbilities
    ));
}

#[test]
fn test_remove_all_abilities_except_mana_for_filter() {
    let ability = RemoveAllAbilitiesExceptManaForFilter::new(ObjectFilter::land());
    assert_eq!(
        ability.id(),
        StaticAbilityId::RemoveAllAbilitiesExceptManaForFilter
    );

    let game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let effects = ability.generate_effects(ObjectId::from_raw(1), PlayerId::from_index(0), &game);
    assert_eq!(effects.len(), 1);
    assert!(matches!(
        effects[0].modification,
        Modification::RemoveAllAbilitiesExceptMana
    ));
}

#[test]
fn test_set_base_power_toughness_for_filter() {
    let ability = SetBasePowerToughnessForFilter::new(ObjectFilter::creature(), 1, 1);
    assert_eq!(
        ability.id(),
        StaticAbilityId::SetBasePowerToughnessForFilter
    );

    let game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let effects = ability.generate_effects(ObjectId::from_raw(1), PlayerId::from_index(0), &game);
    assert_eq!(effects.len(), 1);
    assert!(matches!(
        effects[0].modification,
        Modification::SetPowerToughness {
            power: Value::Fixed(1),
            toughness: Value::Fixed(1),
            sublayer: PtSublayer::Setting,
        }
    ));
}

#[test]
fn test_add_all_subtypes_of_family_for_filter() {
    let ability = AddAllSubtypesOfFamilyForFilter::new(
        ObjectFilter::creature().you_control(),
        SubtypeFamily::Creature,
    );
    assert_eq!(ability.id(), StaticAbilityId::AddAllSubtypesOfFamily);
    assert_eq!(
        ability.display(),
        "creatures you control are every creature type"
    );

    let game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let effects = ability.generate_effects(ObjectId::from_raw(1), PlayerId::from_index(0), &game);
    assert_eq!(effects.len(), 1);
    assert!(matches!(
        effects[0].modification,
        Modification::AddAllSubtypesOfFamily(SubtypeFamily::Creature)
    ));
}

// ---------------------------------------------------------------
// Bruenor Battlehammer: "Each creature you control gets +2/+0
// for each Equipment attached to it."
// ---------------------------------------------------------------

/// Helper: attach an equipment object to a creature in the game state.
fn attach_equipment(game: &mut GameState, equipment_id: ObjectId, creature_id: ObjectId) {
    if let Some(equipment) = game.object_mut(equipment_id) {
        equipment.attached_to = Some(AttachmentTarget::Object(creature_id));
    }
    if let Some(creature) = game.object_mut(creature_id) {
        creature.attachments.push(equipment_id);
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn bruenor_anthem_parses_with_attached_to_affected_and_renders_correctly() {
    // Structure / text test: verify the anthem is parsed with the
    // AttachedToAffected count expression and rendered as oracle text.
    let def = CardDefinitionBuilder::new(CardId::new(), "Bruenor Anthem Test")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Dwarf, Subtype::Warrior])
        .power_toughness(PowerToughness::fixed(5, 3))
        .parse_text("Each creature you control gets +2/+0 for each Equipment attached to it.")
        .expect("Bruenor anthem text should parse");

    let abilities_debug = format!("{:?}", def.abilities);
    assert!(
        abilities_debug.contains("AttachedToAffected"),
        "expected anthem to use AttachedToAffected count expression, got {abilities_debug}"
    );
    assert!(
        !abilities_debug.contains("AttachedToSource"),
        "AttachedToSource should have been promoted to AttachedToAffected, got {abilities_debug}"
    );

    let lines = crate::compiled_text::unprocessed_compiled_lines(&def);
    let joined = lines.join(" ").to_ascii_lowercase();
    assert!(
        joined.contains("creature you control gets +2/+0 for each equipment attached to it")
            || joined.contains("creatures you control get +2/+0 for each equipment attached to it"),
        "expected oracle-like anthem wording with 'attached to it', got {joined}"
    );
}

#[test]
fn bruenor_anthem_generates_per_creature_effects_based_on_equipment_count() {
    // Scenario test: two creatures with different numbers of Equipment
    // should receive different power bonuses.
    let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);

    // Create the source permanent (Bruenor).
    let bruenor_card = CardBuilder::new(CardId::new(), "Bruenor Battlehammer")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(5, 3))
        .build();
    let bruenor_id = game.create_object_from_card(&bruenor_card, alice, Zone::Battlefield);

    // Create creature A (will have 2 equipment).
    let creature_a_card = CardBuilder::new(CardId::new(), "Warrior A")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();
    let creature_a = game.create_object_from_card(&creature_a_card, alice, Zone::Battlefield);

    // Create creature B (will have 0 equipment).
    let creature_b_card = CardBuilder::new(CardId::new(), "Warrior B")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let creature_b = game.create_object_from_card(&creature_b_card, alice, Zone::Battlefield);

    // Create two Equipment and attach both to creature A.
    let equipment_card = CardBuilder::new(CardId::new(), "Sword")
        .card_types(vec![CardType::Artifact])
        .subtypes(vec![Subtype::Equipment])
        .build();
    let eq1 = game.create_object_from_card(&equipment_card, alice, Zone::Battlefield);
    let eq2 = game.create_object_from_card(&equipment_card, alice, Zone::Battlefield);
    attach_equipment(&mut game, eq1, creature_a);
    attach_equipment(&mut game, eq2, creature_a);

    // Build the anthem: "Each creature you control gets +2/+0 for each
    // Equipment attached to it".
    let equipment_filter = ObjectFilter::default().with_subtype(Subtype::Equipment);
    let anthem = Anthem::creatures_you_control(0, 0).with_values(
        AnthemValue::scaled(
            2,
            AnthemCountExpression::AttachedToAffected(equipment_filter),
        ),
        AnthemValue::Fixed(0),
    );

    let effects = anthem.generate_effects(bruenor_id, alice, &game);

    // Should produce one effect per creature (A, B, and Bruenor itself = 3).
    assert_eq!(
        effects.len(),
        3,
        "expected one effect per creature on the battlefield, got {}",
        effects.len()
    );

    // Find effects for creature A (2 equipment -> +4/+0) and creature B (0 equipment -> +0/+0).
    let effect_for = |target_id: ObjectId| -> Option<&ContinuousEffect> {
        effects
            .iter()
            .find(|e| matches!(e.applies_to, EffectTarget::Specific(id) if id == target_id))
    };

    let a_effect = effect_for(creature_a).expect("creature A should have an effect");
    assert!(
        matches!(
            a_effect.modification,
            Modification::ModifyPowerToughness {
                power: 4,
                toughness: 0,
            }
        ),
        "creature A with 2 Equipment should get +4/+0, got {:?}",
        a_effect.modification,
    );

    let b_effect = effect_for(creature_b).expect("creature B should have an effect");
    assert!(
        matches!(
            b_effect.modification,
            Modification::ModifyPowerToughness {
                power: 0,
                toughness: 0,
            }
        ),
        "creature B with 0 Equipment should get +0/+0, got {:?}",
        b_effect.modification,
    );
}

#[test]
fn bruenor_anthem_calculated_power_reflects_per_creature_equipment() {
    // End-to-end behavioral test: verify that calculated_power returns
    // the correct values after static abilities are applied.
    let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);

    let equipment_filter = ObjectFilter::default().with_subtype(Subtype::Equipment);
    let anthem = Anthem::creatures_you_control(0, 0).with_values(
        AnthemValue::scaled(
            2,
            AnthemCountExpression::AttachedToAffected(equipment_filter),
        ),
        AnthemValue::Fixed(0),
    );

    // Create Bruenor (the source of the anthem) with the static ability.
    let bruenor_def = CardDefinitionBuilder::new(CardId::new(), "Bruenor Battlehammer")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(5, 3))
        .with_ability(crate::ability::Ability::static_ability(StaticAbility::new(
            anthem,
        )))
        .build();
    let bruenor_id = game.create_object_from_definition(&bruenor_def, alice, Zone::Battlefield);

    // Create a 1/1 creature.
    let soldier_card = CardBuilder::new(CardId::new(), "Soldier")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();
    let soldier_id = game.create_object_from_card(&soldier_card, alice, Zone::Battlefield);

    // Without equipment, both should have base power.
    assert_eq!(
        game.calculated_power(bruenor_id),
        Some(5),
        "Bruenor with 0 Equipment should have base power 5"
    );
    assert_eq!(
        game.calculated_power(soldier_id),
        Some(1),
        "Soldier with 0 Equipment should have base power 1"
    );

    // Attach one Equipment to the soldier.
    let eq_card = CardBuilder::new(CardId::new(), "Longsword")
        .card_types(vec![CardType::Artifact])
        .subtypes(vec![Subtype::Equipment])
        .build();
    let eq_id = game.create_object_from_card(&eq_card, alice, Zone::Battlefield);
    attach_equipment(&mut game, eq_id, soldier_id);

    assert_eq!(
        game.calculated_power(soldier_id),
        Some(3),
        "Soldier with 1 Equipment should have power 1 + 2 = 3"
    );
    assert_eq!(
        game.calculated_power(bruenor_id),
        Some(5),
        "Bruenor with 0 Equipment should still have base power 5"
    );

    // Attach a second Equipment to the soldier.
    let eq2_id = game.create_object_from_card(&eq_card, alice, Zone::Battlefield);
    attach_equipment(&mut game, eq2_id, soldier_id);

    assert_eq!(
        game.calculated_power(soldier_id),
        Some(5),
        "Soldier with 2 Equipment should have power 1 + 4 = 5"
    );

    // Attach one Equipment to Bruenor.
    let eq3_id = game.create_object_from_card(&eq_card, alice, Zone::Battlefield);
    attach_equipment(&mut game, eq3_id, bruenor_id);

    assert_eq!(
        game.calculated_power(bruenor_id),
        Some(7),
        "Bruenor with 1 Equipment should have power 5 + 2 = 7"
    );
    // Soldier should be unaffected by Bruenor's own Equipment.
    assert_eq!(
        game.calculated_power(soldier_id),
        Some(5),
        "Soldier with 2 Equipment should still have power 1 + 4 = 5"
    );
}

#[test]
fn equipped_creature_anthem_counts_attachments_on_equipped_creature() {
    let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);

    let equipment_filter = ObjectFilter::default().with_subtype(Subtype::Equipment);
    let anthem_filter =
        ObjectFilter::creature().match_tagged("equipped", TaggedOpbjectRelation::IsTaggedObject);
    let anthem = Anthem::new(anthem_filter, 0, 0).with_values(
        AnthemValue::scaled(
            1,
            AnthemCountExpression::AttachedToAffected(equipment_filter),
        ),
        AnthemValue::Fixed(0),
    );

    let gauntlets_def = CardDefinitionBuilder::new(CardId::new(), "Golem-Skin Gauntlets")
        .card_types(vec![CardType::Artifact])
        .subtypes(vec![Subtype::Equipment])
        .with_ability(crate::ability::Ability::static_ability(StaticAbility::new(
            anthem,
        )))
        .build();
    let gauntlets_id = game.create_object_from_definition(&gauntlets_def, alice, Zone::Battlefield);

    let creature_card = CardBuilder::new(CardId::new(), "Elite Vanguard")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 1))
        .build();
    let creature_id = game.create_object_from_card(&creature_card, alice, Zone::Battlefield);

    let other_equipment_card = CardBuilder::new(CardId::new(), "Other Equipment")
        .card_types(vec![CardType::Artifact])
        .subtypes(vec![Subtype::Equipment])
        .build();
    let other_equipment_id =
        game.create_object_from_card(&other_equipment_card, alice, Zone::Battlefield);

    attach_equipment(&mut game, gauntlets_id, creature_id);
    attach_equipment(&mut game, other_equipment_id, creature_id);

    assert_eq!(
        game.calculated_power(creature_id),
        Some(4),
        "equipped creature should get +1/+0 for each Equipment attached to it"
    );
}

#[test]
fn bruenor_anthem_display_shows_for_each_equipment_attached_to_it() {
    let equipment_filter = ObjectFilter::default().with_subtype(Subtype::Equipment);
    let anthem = Anthem::creatures_you_control(0, 0).with_values(
        AnthemValue::scaled(
            2,
            AnthemCountExpression::AttachedToAffected(equipment_filter),
        ),
        AnthemValue::Fixed(0),
    );
    let display = anthem.display();
    assert!(
        display
            .to_ascii_lowercase()
            .contains("for each equipment attached to it"),
        "expected display to mention 'for each Equipment attached to it', got {display}"
    );
    assert!(
        display.to_ascii_lowercase().contains("+2/+0"),
        "expected display to show +2/+0 modifier, got {display}"
    );
}
