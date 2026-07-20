use super::*;
use crate::card::{CardBuilder, PowerToughness};
use crate::cards::builders::CardDefinitionBuilder;
use crate::color::Color;
use crate::filter::StackObjectKind;
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::object::AttachmentTarget;
use crate::static_abilities::LandwalkKind;
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
fn anthem_scales_per_affected_objects_controller_hand() {
    let count = AnthemCountExpression::MatchingFilter(ObjectFilter {
        zone: Some(Zone::Hand),
        owner: Some(PlayerFilter::ControllerOf(ObjectRef::Target)),
        ..Default::default()
    });
    let anthem = Anthem::for_source(0, 0).with_values(
        AnthemValue::scaled(1, count.clone()),
        AnthemValue::scaled(1, count),
    );

    assert_eq!(
        anthem.display(),
        "this creature gets +1/+1 for each card in its controller's hand"
    );
}

#[test]
fn anthem_scales_per_card_in_your_hand() {
    let count = AnthemCountExpression::MatchingFilter(ObjectFilter {
        zone: Some(Zone::Hand),
        owner: Some(PlayerFilter::You),
        ..Default::default()
    });
    let anthem = Anthem::for_source(0, 0).with_values(
        AnthemValue::scaled(1, count.clone()),
        AnthemValue::scaled(1, count),
    );

    assert_eq!(
        anthem.display(),
        "this creature gets +1/+1 for each card in your hand"
    );
}

#[test]
fn equal_per_count_anthem_keeps_trailing_graveyard_condition() {
    let count = AnthemCountExpression::MatchingFilter(
        ObjectFilter::permanent()
            .controlled_by(PlayerFilter::Opponent)
            .with_colors(crate::color::ColorSet::BLACK),
    );
    let anthem = Anthem::for_source(0, 0)
        .with_values(
            AnthemValue::scaled(1, count.clone()),
            AnthemValue::scaled(1, count),
        )
        .with_condition(crate::ConditionExpr::ValueComparison {
            left: Value::CardsInGraveyard(PlayerFilter::You),
            operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
            right: Value::Fixed(7),
        });

    let display = anthem.display();
    assert!(display.contains("for each black permanent"), "{display}");
    assert!(
        display.ends_with(" as long as there are seven or more cards in your graveyard"),
        "{display}"
    );
}

#[test]
fn source_only_conditions_use_same_source_pronouns() {
    assert_eq!(
        Anthem::for_source(1, 0)
            .with_condition(crate::ConditionExpr::SourceIsAttacking)
            .display(),
        "this creature gets +1/+0 as long as it's attacking"
    );

    assert_eq!(
        GrantAbility::source(StaticAbility::first_strike())
            .with_condition(crate::ConditionExpr::SourceIsEquipped)
            .display(),
        "this creature has first strike as long as it's equipped"
    );

    let mut defender = ObjectFilter::default();
    defender.static_abilities.push(StaticAbilityId::Defender);
    let condition = crate::ConditionExpr::SourceMatches(defender);
    let grant =
        GrantAbility::source(StaticAbility::indestructible()).with_condition(condition.clone());
    assert_eq!(
        grant.display(),
        "this creature has indestructible as long as it has defender"
    );

    let game = GameState::new(vec!["Alice".to_string()], 20);
    let effects = grant.generate_effects(ObjectId::from_raw(1), PlayerId::from_index(0), &game);
    assert_eq!(effects.len(), 1);
    assert_eq!(effects[0].condition, Some(condition));
}

#[test]
fn attacking_alone_condition_counts_every_attacking_creature() {
    let mut attacking_creatures = ObjectFilter::creature();
    attacking_creatures.attacking = true;
    let condition = crate::ConditionExpr::And(
        Box::new(crate::ConditionExpr::SourceIsAttacking),
        Box::new(crate::ConditionExpr::CountComparison {
            count: AnthemCountExpression::MatchingFilter(attacking_creatures),
            comparison: Comparison::Equal(1),
            display: Some("no other creatures are attacking".to_string()),
        }),
    );

    assert_eq!(
        describe_static_condition(&condition),
        "as long as this creature is attacking alone"
    );
    assert_eq!(
        describe_same_source_static_condition(&condition),
        "as long as it's attacking alone"
    );
    assert_eq!(
        GrantObjectAbilityForFilter::new(
            ObjectFilter::source(),
            Ability::static_ability(StaticAbility::unblockable()),
            "This creature can't be blocked".to_string(),
        )
        .with_condition(condition.clone())
        .display(),
        "this creature can't be blocked as long as it's attacking alone"
    );

    let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let source = game.create_object_from_card(
        &CardBuilder::new(CardId::new(), "Solo Attacker")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build(),
        alice,
        Zone::Battlefield,
    );
    let other = game.create_object_from_card(
        &CardBuilder::new(CardId::new(), "Other Attacker")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build(),
        alice,
        Zone::Battlefield,
    );

    game.combat = Some(crate::combat_state::CombatState {
        attackers: vec![crate::combat_state::AttackerInfo {
            creature: source,
            target: crate::combat_state::AttackTarget::Player(bob),
        }],
        ..Default::default()
    });
    assert!(static_condition_is_active(&condition, &game, source, alice));

    game.combat
        .as_mut()
        .expect("combat should exist")
        .attackers
        .push(crate::combat_state::AttackerInfo {
            creature: other,
            target: crate::combat_state::AttackTarget::Player(bob),
        });
    assert!(!static_condition_is_active(
        &condition, &game, source, alice
    ));

    game.combat
        .as_mut()
        .expect("combat should exist")
        .attackers
        .remove(0);
    assert!(!static_condition_is_active(
        &condition, &game, source, alice
    ));
}

#[test]
fn non_source_grants_keep_explicit_condition_source() {
    assert_eq!(
        GrantAbility::new(
            ObjectFilter::creature().you_control(),
            StaticAbility::flying()
        )
        .with_condition(crate::ConditionExpr::SourceIsEnchanted)
        .display(),
        "as long as this creature is enchanted, creatures you control have flying"
    );
}

#[test]
fn test_remove_supertypes_display_mentions_scope_and_supertype() {
    let remove = RemoveSupertypesForFilter::new(ObjectFilter::land(), vec![Supertype::Snow]);
    assert_eq!(remove.display(), "All lands are no longer snow");
}

#[test]
fn conditioned_set_card_types_reaches_the_generated_continuous_effect() {
    let condition = crate::ConditionExpr::YourTurn;
    let ability = SetCardTypesForFilter::new(
        ObjectFilter::source(),
        vec![CardType::Artifact, CardType::Creature],
    )
    .with_condition(condition.clone());

    let display = ability.display();
    assert!(display.starts_with("During your turn, "), "{display}");
    assert!(display.ends_with("is an artifact creature"), "{display}");

    let game = GameState::new(vec!["Alice".to_string()], 20);
    let effects = ability.generate_effects(ObjectId::from_raw(1), PlayerId::from_index(0), &game);
    assert_eq!(effects.len(), 1);
    assert_eq!(effects[0].condition, Some(condition));
    assert_eq!(
        effects[0].modification,
        Modification::SetCardTypes(vec![CardType::Artifact, CardType::Creature])
    );
}

#[test]
fn test_single_global_land_subtype_addition_uses_each_land_surface() {
    let swamp = AddSubtypesForFilter::new(ObjectFilter::land(), vec![Subtype::Swamp]);
    assert_eq!(
        swamp.display(),
        "Each land is a Swamp in addition to its other land types"
    );

    let forest = AddSubtypesForFilter::new(ObjectFilter::land(), vec![Subtype::Forest]);
    assert_eq!(
        forest.display(),
        "Each land is a Forest in addition to its other land types"
    );
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
fn describe_static_condition_displays_player_counter_threshold() {
    assert_eq!(
        describe_static_condition(&crate::ConditionExpr::ValueComparison {
            left: Value::PlayerCounters(PlayerFilter::Opponent, CounterType::Poison),
            operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
            right: Value::Fixed(3),
        }),
        "as long as an opponent has three or more poison counters"
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
fn attached_anthem_describes_positive_and_negative_attachment_state_conditions() {
    let mut filter = ObjectFilter::creature();
    filter
        .tagged_constraints
        .push(crate::filter::TaggedObjectConstraint {
            tag: crate::tag::TagKey::from("enchanted"),
            relation: crate::filter::TaggedOpbjectRelation::IsTaggedObject,
        });
    let pirate = ObjectFilter::default().with_subtype(Subtype::Pirate);

    assert_eq!(
        Anthem::new(filter.clone(), 0, 2)
            .with_condition(crate::ConditionExpr::AttachedToSourceMatches(
                pirate.clone()
            ))
            .display(),
        "enchanted creature gets +0/+2 as long as enchanted creature is a pirate"
    );
    assert_eq!(
        Anthem::new(filter, -2, 0)
            .with_condition(crate::ConditionExpr::Not(Box::new(
                crate::ConditionExpr::AttachedToSourceMatches(pirate)
            )))
            .display(),
        "enchanted creature gets -2/-0 as long as enchanted creature isn't a pirate"
    );
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
        Some(2),
        "the anthem should start at +1/+1 before commander casts"
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
fn exact_one_matching_filter_displays_that_creature() {
    let filter = ObjectFilter::creature().you_control();
    let condition = crate::ConditionExpr::CountComparison {
        count: AnthemCountExpression::MatchingFilter(filter.clone()),
        comparison: Comparison::Equal(1),
        display: Some("you control exactly one creature".to_string()),
    };

    assert_eq!(
        Anthem::new(filter.clone(), 3, 1)
            .with_condition(condition.clone())
            .display(),
        "that creature gets +3/+1 as long as you control exactly one creature"
    );
    assert_eq!(
        GrantAbility::new(filter.clone(), StaticAbility::lifelink())
            .with_condition(condition.clone())
            .display(),
        "as long as you control exactly one creature, that creature has lifelink"
    );
    assert_eq!(
        GrantObjectAbilityForFilter::new(
            filter,
            Ability::static_ability(StaticAbility::lifelink()),
            "Lifelink".to_string(),
        )
        .with_condition(condition)
        .display(),
        "as long as you control exactly one creature, that creature has lifelink"
    );
}

#[test]
fn object_unblockable_grant_renders_as_a_restriction() {
    assert_eq!(
        GrantObjectAbilityForFilter::new(
            ObjectFilter::source(),
            Ability::static_ability(StaticAbility::unblockable()),
            "This can't be blocked".to_string(),
        )
        .with_condition(crate::ConditionExpr::SourceIsEnchanted)
        .display(),
        "this creature can't be blocked as long as it's enchanted"
    );
}

#[test]
fn citys_blessing_condition_has_oracle_surface() {
    assert_eq!(
        describe_static_condition(&crate::ConditionExpr::PlayerHasCitysBlessing {
            player: crate::target::PlayerFilter::You,
        }),
        "as long as you have the city's blessing"
    );
}

#[test]
fn multi_object_ability_grants_are_executable_copyable_removable_and_expire() {
    let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);

    let attack_trigger = Ability::triggered(
        crate::triggers::Trigger::this_attacks(),
        vec![crate::effect::Effect::draw(1)],
    );
    let activated = Ability::activated(
        crate::cost::TotalCost::free(),
        vec![crate::effect::Effect::put_counters(
            crate::object::CounterType::PlusOnePlusOne,
            1,
            crate::target::ChooseSpec::Source,
        )],
    );

    let grant_source = CardDefinitionBuilder::new(CardId::new(), "Grant Source")
        .card_types(vec![CardType::Enchantment])
        .with_ability(Ability::static_ability(StaticAbility::new(
            GrantObjectAbilityForFilter::new(
                ObjectFilter::creature().named("Grant Target"),
                attack_trigger.clone(),
                "test keyword".to_string(),
            )
            .with_additional_abilities(vec![activated.clone()]),
        )))
        .build();
    let grant_source_id =
        game.create_object_from_definition(&grant_source, alice, Zone::Battlefield);
    let target_card = CardBuilder::new(CardId::new(), "Grant Target")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let target = game.create_object_from_card(&target_card, alice, Zone::Battlefield);

    let granted = game
        .calculated_characteristics(target)
        .expect("grant target should have calculated characteristics")
        .abilities;
    let has_ability = |abilities: &[Ability], expected: &Ability| {
        abilities
            .iter()
            .any(|candidate| format!("{candidate:?}") == format!("{expected:?}"))
    };
    assert!(has_ability(&granted, &attack_trigger));
    assert!(has_ability(&granted, &activated));

    let copy_source = CardDefinitionBuilder::new(CardId::new(), "Triggered Copy Probe")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .with_ability(Ability::static_ability(
            StaticAbility::copy_triggered_abilities(CopyTriggeredAbilities::new(
                ObjectFilter::creature().named("Grant Target"),
            )),
        ))
        .build();
    let copy = game.create_object_from_definition(&copy_source, alice, Zone::Battlefield);
    let copied = game
        .calculated_characteristics(copy)
        .expect("copy probe should have calculated characteristics")
        .abilities;
    assert!(
        has_ability(&copied, &attack_trigger),
        "a copied triggered ability must include a dynamically granted keyword trigger"
    );

    let remover = CardDefinitionBuilder::new(CardId::new(), "Loss Source")
        .card_types(vec![CardType::Enchantment])
        .with_ability(Ability::static_ability(
            StaticAbility::remove_object_abilities(
                ObjectFilter::creature().named("Grant Target"),
                vec![attack_trigger.clone(), activated.clone()],
                "test keyword",
            ),
        ))
        .build();
    let remover_id = game.create_object_from_definition(&remover, alice, Zone::Battlefield);
    let after_loss = game
        .calculated_characteristics(target)
        .expect("grant target should remain calculable")
        .abilities;
    assert!(!has_ability(&after_loss, &attack_trigger));
    assert!(!has_ability(&after_loss, &activated));

    game.remove_object(remover_id);
    let restored = game
        .calculated_characteristics(target)
        .expect("grant target should remain calculable")
        .abilities;
    assert!(has_ability(&restored, &attack_trigger));
    assert!(has_ability(&restored, &activated));

    game.remove_object(grant_source_id);
    game.effect_store.continuous_effects.add_effect(
        crate::continuous::ContinuousEffect::new(
            grant_source_id,
            alice,
            crate::continuous::EffectTarget::Specific(target),
            crate::continuous::Modification::AddAbilityGeneric(attack_trigger.clone()),
        )
        .until(crate::effect::Until::EndOfTurn),
    );
    let temporary = game
        .calculated_characteristics(target)
        .expect("temporary grant target should be calculable")
        .abilities;
    assert!(has_ability(&temporary, &attack_trigger));
    game.effect_store.continuous_effects.cleanup_end_of_turn();
    let expired = game
        .calculated_characteristics(target)
        .expect("expired grant target should be calculable")
        .abilities;
    assert!(
        !has_ability(&expired, &attack_trigger),
        "the granted keyword ability must disappear when its duration expires"
    );
}

#[test]
fn level_static_carriers_expose_triggered_and_activated_keyword_abilities() {
    let mut game = GameState::new(vec!["Alice".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let attack_trigger = Ability::triggered(
        crate::triggers::Trigger::this_attacks(),
        vec![crate::effect::Effect::draw(1)],
    );
    let activated = Ability::activated(
        crate::cost::TotalCost::free(),
        vec![crate::effect::Effect::put_counters(
            crate::object::CounterType::PlusOnePlusOne,
            1,
            crate::target::ChooseSpec::Source,
        )],
    );
    let carrier = StaticAbility::new(
        GrantObjectAbilityForFilter::new(
            ObjectFilter::source(),
            attack_trigger.clone(),
            "test level keyword".to_string(),
        )
        .with_additional_abilities(vec![activated.clone()]),
    );
    let definition = CardDefinitionBuilder::new(CardId::new(), "Level Grant Probe")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .with_ability(Ability::static_ability(
            StaticAbility::with_level_abilities(vec![
                crate::ability::LevelAbility::new(0, None).with_ability(carrier),
            ]),
        ))
        .build();
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let abilities = game
        .calculated_characteristics(source)
        .expect("level grant probe should be calculable")
        .abilities;
    let has_ability = |expected: &Ability| {
        abilities
            .iter()
            .any(|candidate| format!("{candidate:?}") == format!("{expected:?}"))
    };
    assert!(has_ability(&attack_trigger));
    assert!(has_ability(&activated));
}

#[test]
fn homicidal_seclusion_affects_only_the_unique_controlled_creature() {
    let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let filter = ObjectFilter::creature().you_control();
    let condition = crate::ConditionExpr::CountComparison {
        count: AnthemCountExpression::MatchingFilter(filter.clone()),
        comparison: Comparison::Equal(1),
        display: Some("you control exactly one creature".to_string()),
    };
    let source = CardDefinitionBuilder::new(CardId::new(), "Homicidal Seclusion")
        .card_types(vec![CardType::Enchantment])
        .with_ability(crate::ability::Ability::static_ability(StaticAbility::new(
            Anthem::new(filter.clone(), 3, 1).with_condition(condition.clone()),
        )))
        .with_ability(crate::ability::Ability::static_ability(StaticAbility::new(
            GrantAbility::new(filter, StaticAbility::lifelink()).with_condition(condition),
        )))
        .build();
    game.create_object_from_definition(&source, alice, Zone::Battlefield);

    let creature = CardBuilder::new(CardId::new(), "Test Creature")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let alice_creature = game.create_object_from_card(&creature, alice, Zone::Battlefield);
    let bob_creature = game.create_object_from_card(&creature, bob, Zone::Battlefield);

    assert_eq!(game.calculated_power(alice_creature), Some(5));
    assert_eq!(game.calculated_toughness(alice_creature), Some(3));
    assert!(game.current_has_static_ability_id(alice_creature, StaticAbilityId::Lifelink));
    assert_eq!(game.calculated_power(bob_creature), Some(2));
    assert_eq!(game.calculated_toughness(bob_creature), Some(2));
    assert!(!game.current_has_static_ability_id(bob_creature, StaticAbilityId::Lifelink));

    let second_alice_creature = game.create_object_from_card(&creature, alice, Zone::Battlefield);
    for creature_id in [alice_creature, second_alice_creature] {
        assert_eq!(game.calculated_power(creature_id), Some(2));
        assert_eq!(game.calculated_toughness(creature_id), Some(2));
        assert!(!game.current_has_static_ability_id(creature_id, StaticAbilityId::Lifelink));
    }
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
fn quantified_grant_preserves_each_and_direct_restriction_surface() {
    let grant = GrantAbility::new(
        ObjectFilter::creature().you_control(),
        StaticAbility::cant_be_blocked_by_more_than(1),
    )
    .with_set_quantifier_surface(Some(ironsmith_core::SetQuantifierSurface::Each));

    assert_eq!(
        grant.display(),
        "Each creature you control can't be blocked by more than 1 creature"
    );
}

#[test]
fn verb_phrase_grants_conjugate_without_has_or_have() {
    let creatures_you_control = ObjectFilter::creature().you_control();
    assert_eq!(
        GrantAbility::new(creatures_you_control.clone(), StaticAbility::must_attack(),).display(),
        "creatures you control attack each combat if able"
    );
    assert_eq!(
        GrantAbility::new(creatures_you_control, StaticAbility::must_block()).display(),
        "creatures you control block each combat if able"
    );
    assert_eq!(
        GrantAbility::source(StaticAbility::must_attack()).display(),
        "this creature attacks each combat if able"
    );
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
fn dynamic_base_power_toughness_uses_plural_possessive_for_plural_filter() {
    let value = Value::ManaValueOf(Box::new(ChooseSpec::Iterated));
    let ability =
        SetBasePowerToughnessValueForFilter::new(ObjectFilter::creature(), value.clone(), value);

    assert_eq!(
        ability.display(),
        "creatures have base power and base toughness each equal to their mana value"
    );
}

#[test]
fn each_surface_uses_singular_type_and_iterated_value_grammar() {
    let mut filter = ObjectFilter::enchantment();
    filter.other = true;
    filter.excluded_subtypes.push(Subtype::Aura);
    filter.set_set_quantifier_surface(Some(ironsmith_core::SetQuantifierSurface::Each));

    let add_type = AddCardTypesForFilter::new(filter.clone(), vec![CardType::Creature]);
    assert_eq!(
        add_type.display(),
        "Each other non-Aura enchantment is a creature in addition to its other types"
    );

    let value = Value::ManaValueOf(Box::new(ChooseSpec::Iterated));
    let set_pt = SetBasePowerToughnessValueForFilter::new(filter, value.clone(), value);
    assert_eq!(
        set_pt.display(),
        "Each other non-Aura enchantment has base power and base toughness each equal to its mana value"
    );
}

#[test]
fn filtered_animation_values_are_evaluated_per_affected_object() {
    let mut march_game = GameState::new(vec!["Alice".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let march_filter = ObjectFilter::artifact().without_type(CardType::Creature);
    let mana_value = Value::ManaValueOf(Box::new(ChooseSpec::Iterated));
    let march = CardDefinitionBuilder::new(CardId::new(), "March of the Machines")
        .card_types(vec![CardType::Enchantment])
        .with_ability(Ability::static_ability(StaticAbility::set_card_types(
            march_filter.clone(),
            vec![CardType::Artifact, CardType::Creature],
        )))
        .with_ability(Ability::static_ability(
            StaticAbility::set_base_power_toughness_value(
                march_filter,
                mana_value.clone(),
                mana_value,
            ),
        ))
        .build();
    march_game.create_object_from_definition(&march, alice, Zone::Battlefield);

    let artifact_two = CardBuilder::new(CardId::new(), "Two-Mana Artifact")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(2)]]))
        .card_types(vec![CardType::Artifact])
        .build();
    let artifact_five = CardBuilder::new(CardId::new(), "Five-Mana Artifact")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(5)]]))
        .card_types(vec![CardType::Artifact])
        .build();
    let existing_creature = CardBuilder::new(CardId::new(), "Existing Artifact Creature")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(3)]]))
        .card_types(vec![CardType::Artifact, CardType::Creature])
        .power_toughness(PowerToughness::fixed(7, 7))
        .build();
    let artifact_two = march_game.create_object_from_card(&artifact_two, alice, Zone::Battlefield);
    let artifact_five =
        march_game.create_object_from_card(&artifact_five, alice, Zone::Battlefield);
    let existing_creature =
        march_game.create_object_from_card(&existing_creature, alice, Zone::Battlefield);

    for (object, expected) in [(artifact_two, 2), (artifact_five, 5)] {
        let chars = march_game
            .calculated_characteristics(object)
            .expect("animated artifact should have calculated characteristics");
        assert!(chars.card_types.contains(&CardType::Artifact));
        assert!(chars.card_types.contains(&CardType::Creature));
        assert_eq!(chars.power, Some(expected));
        assert_eq!(chars.toughness, Some(expected));
    }
    assert_eq!(march_game.calculated_power(existing_creature), Some(7));
    assert_eq!(march_game.calculated_toughness(existing_creature), Some(7));

    let mut spark_game = GameState::new(vec!["Alice".to_string()], 20);
    let walker_filter = ObjectFilter::planeswalker().with_counter_type(CounterType::Loyalty);
    let loyalty = Value::CountersOn(Box::new(ChooseSpec::Iterated), Some(CounterType::Loyalty));
    let spark = CardDefinitionBuilder::new(CardId::new(), "Spark Rupture")
        .card_types(vec![CardType::Enchantment])
        .with_ability(Ability::static_ability(
            StaticAbility::remove_all_abilities(walker_filter.clone()),
        ))
        .with_ability(Ability::static_ability(StaticAbility::set_card_types(
            walker_filter.clone(),
            vec![CardType::Creature],
        )))
        .with_ability(Ability::static_ability(
            StaticAbility::set_base_power_toughness_value(walker_filter, loyalty.clone(), loyalty),
        ))
        .build();
    spark_game.create_object_from_definition(&spark, alice, Zone::Battlefield);

    let walker = CardDefinitionBuilder::new(CardId::new(), "Ability-Bearing Walker")
        .card_types(vec![CardType::Planeswalker])
        .loyalty(0)
        .with_ability(Ability::static_ability(StaticAbility::indestructible()))
        .build();
    let walker_three = spark_game.create_object_from_definition(&walker, alice, Zone::Battlefield);
    let walker_seven = spark_game.create_object_from_definition(&walker, alice, Zone::Battlefield);
    let walker_zero = spark_game.create_object_from_definition(&walker, alice, Zone::Battlefield);
    spark_game
        .object_mut(walker_three)
        .expect("three-loyalty walker")
        .counters
        .insert(CounterType::Loyalty, 3);
    spark_game
        .object_mut(walker_seven)
        .expect("seven-loyalty walker")
        .counters
        .insert(CounterType::Loyalty, 7);

    for (object, expected) in [(walker_three, 3), (walker_seven, 7)] {
        let chars = spark_game
            .calculated_characteristics(object)
            .expect("animated planeswalker should have calculated characteristics");
        assert_eq!(chars.card_types.len(), 1);
        assert!(chars.card_types.contains(&CardType::Creature));
        assert_eq!(chars.power, Some(expected));
        assert_eq!(chars.toughness, Some(expected));
        assert!(
            !chars
                .static_abilities
                .iter()
                .any(|ability| ability.id() == StaticAbilityId::Indestructible),
            "Spark Rupture should remove the animated planeswalker's abilities"
        );
    }
    let zero_chars = spark_game
        .calculated_characteristics(walker_zero)
        .expect("zero-loyalty walker should have calculated characteristics");
    assert!(zero_chars.card_types.contains(&CardType::Planeswalker));
    assert!(!zero_chars.card_types.contains(&CardType::Creature));
    assert!(
        zero_chars
            .static_abilities
            .iter()
            .any(|ability| ability.id() == StaticAbilityId::Indestructible),
        "a planeswalker without loyalty counters should remain untouched"
    );
}

#[test]
fn attached_conditional_animation_is_scoped_to_each_aura() {
    let mut game = GameState::new(vec!["Alice".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let animation_filter =
        ObjectFilter::artifact().match_tagged("enchanted", TaggedOpbjectRelation::IsTaggedObject);
    let condition =
        crate::ConditionExpr::Not(Box::new(crate::ConditionExpr::AttachedToSourceMatches(
            ObjectFilter::default().with_type(CardType::Creature),
        )));
    let mana_value = Value::ManaValueOf(Box::new(ChooseSpec::Iterated));
    let animate_artifact = CardDefinitionBuilder::new(CardId::new(), "Animate Artifact")
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![Subtype::Aura])
        .with_ability(Ability::static_ability(
            StaticAbility::set_card_types(
                animation_filter.clone(),
                vec![CardType::Artifact, CardType::Creature],
            )
            .with_condition(condition.clone())
            .expect("set-card-types animation supports a condition"),
        ))
        .with_ability(Ability::static_ability(
            StaticAbility::set_base_power_toughness_value(
                animation_filter,
                mana_value.clone(),
                mana_value,
            )
            .with_condition(condition)
            .expect("dynamic base P/T animation supports a condition"),
        ))
        .build();

    let noncreature_artifact = CardBuilder::new(CardId::new(), "Animated Relic")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(4)]]))
        .card_types(vec![CardType::Artifact])
        .build();
    let existing_creature = CardBuilder::new(CardId::new(), "Already Animated")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(2)]]))
        .card_types(vec![CardType::Artifact, CardType::Creature])
        .power_toughness(PowerToughness::fixed(6, 6))
        .build();
    let noncreature_artifact =
        game.create_object_from_card(&noncreature_artifact, alice, Zone::Battlefield);
    let existing_creature =
        game.create_object_from_card(&existing_creature, alice, Zone::Battlefield);
    let active_aura =
        game.create_object_from_definition(&animate_artifact, alice, Zone::Battlefield);
    let inactive_aura =
        game.create_object_from_definition(&animate_artifact, alice, Zone::Battlefield);
    attach_equipment(&mut game, active_aura, noncreature_artifact);
    attach_equipment(&mut game, inactive_aura, existing_creature);

    let animated = game
        .calculated_characteristics(noncreature_artifact)
        .expect("enchanted noncreature artifact should be calculable");
    assert!(animated.card_types.contains(&CardType::Artifact));
    assert!(animated.card_types.contains(&CardType::Creature));
    assert_eq!(animated.power, Some(4));
    assert_eq!(animated.toughness, Some(4));

    let unchanged = game
        .calculated_characteristics(existing_creature)
        .expect("already-creature artifact should be calculable");
    assert_eq!(unchanged.power, Some(6));
    assert_eq!(unchanged.toughness, Some(6));
}

#[test]
#[ignore = "manual perf probe for the classic anthem-pile board"]
fn probe_big_board_dependency_hotspots() {
    let piles: &[(&str, &str)] = &[
        (
            "Mycosynth Lattice",
            "All permanents are artifacts in addition to their other types.\nAll cards that aren't on the battlefield, spells, and permanents are colorless.\nPlayers may spend mana as though it were mana of any color.",
        ),
        (
            "Akroma's Memorial",
            "Creatures you control have flying, first strike, vigilance, trample, haste, and protection from black and from red.",
        ),
        (
            "Always Watching",
            "Nontoken creatures you control get +1/+1 and have vigilance.",
        ),
        ("Fervor", "Creatures you control have haste."),
        ("Glorious Anthem", "Creatures you control get +1/+1."),
        (
            "Honor of the Pure",
            "White creatures you control get +1/+1.",
        ),
        ("Bad Moon", "Black creatures get +1/+1."),
        (
            "Favorable Winds",
            "Creatures you control with flying get +1/+1.",
        ),
        ("Gaea's Anthem", "Creatures you control get +1/+1."),
        (
            "Intangible Virtue",
            "Creature tokens you control get +1/+1 and have vigilance.",
        ),
        (
            "Spidersilk Armor",
            "Creatures you control get +0/+1 and have reach.",
        ),
        (
            "Dictate of Heliod",
            "Flash\nCreatures you control get +2/+2.",
        ),
    ];

    let creature_count = 200usize;
    let run = |label: &str, pile: &[(&str, &str)]| {
        let mut game = GameState::new(vec!["Alice".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let creature = CardBuilder::new(CardId::new(), "Grizzly Bears")
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(2)]]))
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        for _ in 0..creature_count {
            game.create_object_from_card(&creature, alice, Zone::Battlefield);
        }
        for &(name, text) in pile {
            let definition = CardDefinitionBuilder::new(CardId::new(), name)
                .card_types(vec![CardType::Enchantment])
                .parse_text(text)
                .unwrap_or_else(|err| panic!("{name} should parse: {err:?}"));
            for _ in 0..6 {
                game.create_object_from_definition(&definition, alice, Zone::Battlefield);
            }
        }
        let started = std::time::Instant::now();
        let battlefield = game.battlefield.clone();
        for id in &battlefield {
            let _ = game.calculated_characteristics(*id);
        }
        let counters = game.work_counters();
        eprintln!(
            "{label}: {}ms sorts={} pairs={} recomputes={}",
            started.elapsed().as_millis(),
            counters.dependency_sorts,
            counters.dependency_pairs_probed,
            counters.characteristics_full_recomputes,
        );
        if counters.dependency_sorts > 0 && std::env::var_os("PROBE_DUMP_GROUPS").is_some() {
            let effects = game.all_continuous_effects();
            let mut by_layer: std::collections::BTreeMap<u8, Vec<&crate::ContinuousEffect>> =
                std::collections::BTreeMap::new();
            for effect in effects.iter() {
                by_layer
                    .entry(effect.modification.layer() as u8)
                    .or_default()
                    .push(effect);
            }
            for (layer, group) in &by_layer {
                if group.len() <= 1 {
                    continue;
                }
                let needs =
                    crate::dependency::needs_baseline_dependency_sort(group.as_slice(), &game);
                eprintln!(
                    "  layer {layer}: {} effects, needs_baseline_sort={needs}",
                    group.len()
                );
                if needs {
                    let sample = group[0];
                    eprintln!(
                        "    sample condition={:?}",
                        sample.condition.as_ref().map(|_| "some")
                    );
                    eprintln!(
                        "    sample originating_static={:?}",
                        sample.originating_static_ability.as_ref().map(|_| "some")
                    );
                    eprintln!("    sample applies_to={:.300?}", sample.applies_to);
                    eprintln!("    sample modification={:.300?}", sample.modification);
                }
            }
        }
    };

    for &(name, text) in piles {
        run(name, &[(name, text)]);
    }
    run("ALL PILES", piles);
}

#[test]
fn second_animation_aura_priority_loop_resolves_without_hanging() {
    use crate::decision::{AutoPassDecisionMaker, GameProgress, LegalAction};
    use crate::game_loop::{
        PriorityLoopState, PriorityResponse, advance_priority_with_dm,
        apply_priority_response_with_dm,
    };
    use crate::game_state::{StackEntry, Target};
    use crate::triggers::TriggerQueue;

    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);

    let animate_artifact = CardDefinitionBuilder::new(CardId::new(), "Animate Artifact")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(1)],
            vec![ManaSymbol::Blue],
        ]))
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![Subtype::Aura])
        .parse_text(
            "Enchant artifact\nAs long as enchanted artifact isn't a creature, it's an artifact creature with power and toughness each equal to its mana value.",
        )
        .expect("Animate Artifact should parse");

    let mine = CardBuilder::new(CardId::new(), "Howling Mine")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(2)]]))
        .card_types(vec![CardType::Artifact])
        .build();
    let mine_one = game.create_object_from_card(&mine, alice, Zone::Battlefield);
    let mine_two = game.create_object_from_card(&mine, alice, Zone::Battlefield);

    let aura_one = game.create_object_from_definition(&animate_artifact, alice, Zone::Battlefield);
    if let Some(aura) = game.object_mut(aura_one) {
        aura.attached_to = Some(AttachmentTarget::Object(mine_one));
    }
    if let Some(host) = game.object_mut(mine_one) {
        host.attachments.push(aura_one);
    }

    let aura_two = game.create_object_from_definition(&animate_artifact, alice, Zone::Stack);
    game.push_to_stack(
        StackEntry::new(aura_two, alice).with_targets(vec![Target::Object(mine_two)]),
    );

    let mut trigger_queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut dm = AutoPassDecisionMaker;
    let mut progress = advance_priority_with_dm(&mut game, &mut trigger_queue, &mut dm)
        .expect("priority loop should begin");

    for _ in 0..64 {
        if game.stack.is_empty() {
            break;
        }
        progress = match progress {
            GameProgress::NeedsDecisionCtx(
                crate::decisions::context::DecisionContext::Priority(_),
            ) => apply_priority_response_with_dm(
                &mut game,
                &mut trigger_queue,
                &mut state,
                &PriorityResponse::PriorityAction(LegalAction::PassPriority),
                &mut dm,
            )
            .expect("passing priority should succeed"),
            GameProgress::StackResolved | GameProgress::Continue => {
                advance_priority_with_dm(&mut game, &mut trigger_queue, &mut dm)
                    .expect("advancing after resolution should succeed")
            }
            other => panic!("unexpected progress: {other:?}"),
        };
    }

    assert!(game.stack.is_empty(), "aura should have resolved");
    for mine_id in [mine_one, mine_two] {
        let animated = game
            .calculated_characteristics(mine_id)
            .expect("each enchanted artifact should be calculable");
        assert!(
            animated.card_types.contains(&CardType::Creature),
            "mine {mine_id:?} should be animated"
        );
    }
}

#[test]
fn second_animation_aura_resolves_from_stack_without_hanging() {
    use crate::game_loop::resolve_stack_entry;
    use crate::game_state::{StackEntry, Target};

    let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);

    let animate_artifact = CardDefinitionBuilder::new(CardId::new(), "Animate Artifact")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(1)],
            vec![ManaSymbol::Blue],
        ]))
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![Subtype::Aura])
        .parse_text(
            "Enchant artifact\nAs long as enchanted artifact isn't a creature, it's an artifact creature with power and toughness each equal to its mana value.",
        )
        .expect("Animate Artifact should parse");

    let mine = CardBuilder::new(CardId::new(), "Howling Mine")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(2)]]))
        .card_types(vec![CardType::Artifact])
        .build();
    let mine_one = game.create_object_from_card(&mine, alice, Zone::Battlefield);
    let mine_two = game.create_object_from_card(&mine, alice, Zone::Battlefield);

    let aura_one = game.create_object_from_definition(&animate_artifact, alice, Zone::Battlefield);
    if let Some(aura) = game.object_mut(aura_one) {
        aura.attached_to = Some(AttachmentTarget::Object(mine_one));
    }
    if let Some(host) = game.object_mut(mine_one) {
        host.attachments.push(aura_one);
    }

    let aura_two = game.create_object_from_definition(&animate_artifact, alice, Zone::Stack);
    game.push_to_stack(
        StackEntry::new(aura_two, alice).with_targets(vec![Target::Object(mine_two)]),
    );

    resolve_stack_entry(&mut game).expect("second Animate Artifact should resolve");

    assert!(
        game.stack.is_empty(),
        "stack should be empty after resolution"
    );
    for mine_id in [mine_one, mine_two] {
        let animated = game
            .calculated_characteristics(mine_id)
            .expect("each enchanted artifact should be calculable");
        assert!(
            animated.card_types.contains(&CardType::Creature),
            "mine {mine_id:?} should be animated"
        );
        assert_eq!(animated.power, Some(2));
        assert_eq!(animated.toughness, Some(2));
    }
}

#[test]
fn two_active_conditional_animation_auras_terminate() {
    let mut game = GameState::new(vec!["Alice".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let animation_filter =
        ObjectFilter::artifact().match_tagged("enchanted", TaggedOpbjectRelation::IsTaggedObject);
    let condition =
        crate::ConditionExpr::Not(Box::new(crate::ConditionExpr::AttachedToSourceMatches(
            ObjectFilter::default().with_type(CardType::Creature),
        )));
    let mana_value = Value::ManaValueOf(Box::new(ChooseSpec::Iterated));
    let animate_artifact = CardDefinitionBuilder::new(CardId::new(), "Animate Artifact")
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![Subtype::Aura])
        .with_ability(Ability::static_ability(
            StaticAbility::set_card_types(
                animation_filter.clone(),
                vec![CardType::Artifact, CardType::Creature],
            )
            .with_condition(condition.clone())
            .expect("set-card-types animation supports a condition"),
        ))
        .with_ability(Ability::static_ability(
            StaticAbility::set_base_power_toughness_value(
                animation_filter,
                mana_value.clone(),
                mana_value,
            )
            .with_condition(condition)
            .expect("dynamic base P/T animation supports a condition"),
        ))
        .build();

    let mine = CardBuilder::new(CardId::new(), "Howling Mine")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(2)]]))
        .card_types(vec![CardType::Artifact])
        .build();
    let mine_one = game.create_object_from_card(&mine, alice, Zone::Battlefield);
    let mine_two = game.create_object_from_card(&mine, alice, Zone::Battlefield);
    let aura_one = game.create_object_from_definition(&animate_artifact, alice, Zone::Battlefield);
    let aura_two = game.create_object_from_definition(&animate_artifact, alice, Zone::Battlefield);
    attach_equipment(&mut game, aura_one, mine_one);
    attach_equipment(&mut game, aura_two, mine_two);

    for mine_id in [mine_one, mine_two] {
        let animated = game
            .calculated_characteristics(mine_id)
            .expect("each enchanted artifact should be calculable");
        assert!(animated.card_types.contains(&CardType::Creature));
        assert_eq!(animated.power, Some(2));
        assert_eq!(animated.toughness, Some(2));
    }
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

#[test]
fn party_scaled_anthem_display_uses_for_each_party_surface() {
    let party = Value::PartySize(PlayerFilter::You)
        .with_surface_hint(ironsmith_core::ValueSurfaceHint::ForEach);
    let anthem =
        Anthem::for_source(0, 0).with_values(AnthemValue::Dynamic(party), AnthemValue::Fixed(0));

    assert_eq!(
        anthem.display(),
        "this creature gets +1/+0 for each creature in your party"
    );
}

#[test]
fn dynamic_party_anthem_uses_rules_legal_maximum_party_size() {
    let mut game = GameState::new(vec!["Alice".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let party = Value::PartySize(PlayerFilter::You)
        .with_surface_hint(ironsmith_core::ValueSurfaceHint::ForEach);
    let anthem =
        Anthem::for_source(0, 0).with_values(AnthemValue::Dynamic(party), AnthemValue::Fixed(0));
    let source = CardDefinitionBuilder::new(CardId::new(), "Party Leader")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .with_ability(crate::ability::Ability::static_ability(StaticAbility::new(
            anthem,
        )))
        .build();
    let source_id = game.create_object_from_definition(&source, alice, Zone::Battlefield);

    for (name, subtypes) in [
        ("Flexible Member", vec![Subtype::Cleric, Subtype::Rogue]),
        ("Cleric Member", vec![Subtype::Cleric]),
    ] {
        let member = CardBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Creature])
            .subtypes(subtypes)
            .power_toughness(PowerToughness::fixed(1, 1))
            .build();
        game.create_object_from_card(&member, alice, Zone::Battlefield);
    }

    assert_eq!(game.calculated_power(source_id), Some(3));
}

#[test]
fn copied_static_variants_preserve_payloads_scope_and_rules_behavior() {
    use ironsmith_core::StaticAbilityVariantSelector::{Any, ProtectionFromColor};

    let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let red_filter = ObjectFilter::default().with_colors(crate::color::ColorSet::RED);
    let blue_filter = ObjectFilter::default().with_colors(crate::color::ColorSet::BLUE);

    let donor = CardDefinitionBuilder::new(CardId::new(), "Variant Donor")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .with_ability(Ability::static_ability(StaticAbility::protection(
            crate::ability::ProtectionFrom::Color(crate::color::ColorSet::RED),
        )))
        .with_ability(Ability::static_ability(StaticAbility::protection(
            crate::ability::ProtectionFrom::Creatures,
        )))
        .with_ability(Ability::static_ability(StaticAbility::landwalk(
            Subtype::Plains,
        )))
        .with_ability(Ability::static_ability(StaticAbility::hexproof_from(
            red_filter.clone(),
        )))
        .with_ability(Ability::static_ability(StaticAbility::hexproof_from(
            blue_filter.clone(),
        )))
        .build();
    let donor_id = game.create_object_from_definition(&donor, alice, Zone::Battlefield);

    let donor_filter = ObjectFilter::creature().named("Variant Donor");
    let general_copy = CardDefinitionBuilder::new(CardId::new(), "General Copy")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .with_ability(Ability::static_ability(
            StaticAbility::copy_static_ability_variants(CopyStaticAbilityVariants::new(
                donor_filter.clone(),
                vec![
                    Any(StaticAbilityId::Protection),
                    Any(StaticAbilityId::Landwalk),
                    Any(StaticAbilityId::HexproofFrom),
                ],
                "copy selected static abilities".to_string(),
            )),
        ))
        .build();
    let general_id = game.create_object_from_definition(&general_copy, alice, Zone::Battlefield);

    let color_copy = CardDefinitionBuilder::new(CardId::new(), "Color Copy")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .with_ability(Ability::static_ability(
            StaticAbility::copy_static_ability_variants(CopyStaticAbilityVariants::new(
                donor_filter,
                vec![ProtectionFromColor],
                "copy protection from colors".to_string(),
            )),
        ))
        .build();
    let color_id = game.create_object_from_definition(&color_copy, alice, Zone::Battlefield);

    let landwalk_copy = CardDefinitionBuilder::new(CardId::new(), "Landwalk Copy")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .with_ability(Ability::static_ability(
            StaticAbility::copy_static_ability_variants(CopyStaticAbilityVariants::new(
                ObjectFilter::creature().named("Variant Donor"),
                vec![Any(StaticAbilityId::Landwalk)],
                "copy landwalk".to_string(),
            )),
        ))
        .build();
    let landwalk_id = game.create_object_from_definition(&landwalk_copy, alice, Zone::Battlefield);

    let inherited = game
        .calculated_characteristics(general_id)
        .expect("general copy should be calculable")
        .static_abilities;
    assert!(inherited.iter().any(|ability| {
        ability.protection_from()
            == Some(&crate::ability::ProtectionFrom::Color(
                crate::color::ColorSet::RED,
            ))
    }));
    assert!(inherited.iter().any(|ability| {
        ability.protection_from() == Some(&crate::ability::ProtectionFrom::Creatures)
    }));
    assert!(inherited.iter().any(|ability| {
        ability.landwalk_kind()
            == Some(LandwalkKind::Subtype {
                subtype: Subtype::Plains,
                snow: false,
            })
    }));
    assert!(
        inherited
            .iter()
            .any(|ability| { ability.hexproof_from_filter() == Some(&red_filter) })
    );
    assert!(
        inherited
            .iter()
            .any(|ability| { ability.hexproof_from_filter() == Some(&blue_filter) })
    );

    let color_scoped = game
        .calculated_characteristics(color_id)
        .expect("color copy should be calculable")
        .static_abilities;
    assert!(color_scoped.iter().any(|ability| {
        ability.protection_from()
            == Some(&crate::ability::ProtectionFrom::Color(
                crate::color::ColorSet::RED,
            ))
    }));
    assert!(!color_scoped.iter().any(|ability| {
        ability.protection_from() == Some(&crate::ability::ProtectionFrom::Creatures)
    }));

    let red_blocker = game.create_object_from_card(
        &CardBuilder::new(CardId::new(), "Red Blocker")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .color_indicator(crate::color::ColorSet::RED)
            .build(),
        bob,
        Zone::Battlefield,
    );
    let blue_blocker = game.create_object_from_card(
        &CardBuilder::new(CardId::new(), "Blue Blocker")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .color_indicator(crate::color::ColorSet::BLUE)
            .build(),
        bob,
        Zone::Battlefield,
    );
    let color_attacker = game
        .object(color_id)
        .expect("color copy should exist")
        .clone();
    assert!(!crate::rules::can_block(
        &color_attacker,
        game.object(red_blocker).expect("red blocker should exist"),
        &game,
    ));
    assert!(crate::rules::can_block(
        &color_attacker,
        game.object(blue_blocker)
            .expect("blue blocker should exist"),
        &game,
    ));

    let landwalk_attacker = game
        .object(landwalk_id)
        .expect("landwalk copy should exist")
        .clone();
    assert!(crate::rules::can_block(
        &landwalk_attacker,
        game.object(blue_blocker)
            .expect("blue blocker should exist"),
        &game,
    ));

    game.create_object_from_card(
        &CardBuilder::new(CardId::new(), "Plains")
            .card_types(vec![CardType::Land])
            .subtypes(vec![Subtype::Plains])
            .build(),
        bob,
        Zone::Battlefield,
    );
    assert!(!crate::rules::can_block(
        &landwalk_attacker,
        game.object(blue_blocker)
            .expect("blue blocker should exist"),
        &game,
    ));

    game.remove_object(donor_id);
    let after_removal = game
        .calculated_characteristics(general_id)
        .expect("general copy should remain calculable")
        .static_abilities;
    assert!(!after_removal.iter().any(|ability| {
        matches!(
            ability.id(),
            StaticAbilityId::Protection | StaticAbilityId::Landwalk | StaticAbilityId::HexproofFrom
        )
    }));
    assert!(crate::rules::can_block(
        &landwalk_attacker,
        game.object(blue_blocker)
            .expect("blue blocker should exist"),
        &game,
    ));
}
