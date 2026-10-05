use ironsmith::ability::AbilityKind;
use ironsmith::cards::{CardDefinition, builders::CardDefinitionBuilder};
use ironsmith::effects::{EffectContext, ResolvedTarget, ReturnFromGraveyardToBattlefieldEffect};
use ironsmith::game_loop::extract_target_requirements_from_program_with_modes;
use ironsmith::mana::{ManaCost, ManaSymbol};
use ironsmith::object::CounterType;
use ironsmith::resolution::ResolutionProgram;
use ironsmith::static_abilities::{StaticAbility, StaticAbilityId};
use ironsmith::target::{ChooseSpec, PlayerFilter};
use ironsmith::{CardId, CardType, GameState, PlayerId, Subtype, Target, Zone};
use ironsmith_compiler_runtime::compile_to_runtime_definition;

const AWAKENING: &str = "Mana cost: {X}{3}{W}\nType: Sorcery\nReturn target artifact or non-Aura enchantment card from your graveyard to the battlefield with X additional +1/+1 counters on it. It's a 1/1 Spirit creature with flying in addition to its other types.";
const EXCAVA: &str = "Mana cost: {2}{R}{W}\nType: Legendary Creature — Spirit Horse\nPower/Toughness: 3/3\nFlying, haste\nWhenever Excava attacks, return up to one target artifact, creature, or non-Aura enchantment card with mana value 3 or less from your graveyard to the battlefield with a finality counter on it. It's a 1/1 Spirit creature with flying in addition to its other types. (If a creature with a finality counter on it would die, exile it instead.)";

fn return_program(definition: &CardDefinition) -> &ResolutionProgram {
    definition.spell_effect.as_ref().unwrap_or_else(|| {
        definition
            .abilities
            .iter()
            .find_map(|ability| match &ability.kind {
                AbilityKind::Triggered(triggered) => Some(&triggered.effects),
                _ => None,
            })
            .expect("Excava has an attack trigger")
    })
}

fn find_return(
    effect: &ironsmith::effect::Effect,
) -> Option<ReturnFromGraveyardToBattlefieldEffect> {
    if let Some(returned) = effect.downcast_ref::<ReturnFromGraveyardToBattlefieldEffect>() {
        return Some(returned.clone());
    }
    let mut found = None;
    effect.visit_child_effects(&mut |child| {
        if found.is_none() {
            found = find_return(child);
        }
    });
    found
}

fn fixture(types: Vec<CardType>, subtypes: Vec<Subtype>, mana_value: u8) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Return target fixture")
        .card_types(types)
        .subtypes(subtypes)
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(
            mana_value,
        )]]))
        .build()
}

#[test]
fn qualified_union_cards_preserve_return_filter_and_entry_counters() {
    for (name, text, serial) in [
        ("Abuelo's Awakening", AWAKENING, false),
        ("Excava, the Risen Past", EXCAVA, true),
    ] {
        let definition = compile_to_runtime_definition(name, text, false).unwrap_or_else(|error| {
            panic!("{name} must compile through the public pipeline: {error}")
        });
        let returned = return_program(&definition)
            .flattened_default_effects()
            .iter()
            .find_map(find_return)
            .expect("one graveyard return remains executable");
        let ChooseSpec::Object(filter) = returned.target.base() else {
            panic!("{name}: expected an object union: {returned:#?}");
        };
        assert_eq!(filter.zone, Some(Zone::Graveyard), "{name}");
        assert_eq!(filter.owner, Some(PlayerFilter::You), "{name}");
        assert_eq!(filter.mana_value.is_some(), serial, "{name}");
        assert!(
            filter.excluded_subtypes.is_empty(),
            "the Aura exclusion is branch-local"
        );
        assert_eq!(
            filter.any_of.len(),
            if serial { 3 } else { 2 },
            "{name}: {filter:#?}"
        );
        for branch in &filter.any_of {
            if branch.card_types == [CardType::Enchantment] {
                assert_eq!(branch.excluded_subtypes, [Subtype::Aura]);
            } else {
                assert!(branch.excluded_subtypes.is_empty());
            }
        }
        let [counter] = returned.enters_with_counters.as_slice() else {
            panic!("entry counters must be fused into the return: {returned:#?}");
        };
        assert_eq!(
            counter.counter_type,
            if serial {
                CounterType::Finality
            } else {
                CounterType::PlusOnePlusOne
            }
        );
        assert_eq!(
            counter.amount.unhinted(),
            if serial {
                &ironsmith::effect::Value::Fixed(1)
            } else {
                &ironsmith::effect::Value::X
            }
        );
        assert_eq!(returned.target.count().min, if serial { 0 } else { 1 });
        assert_eq!(returned.target.count().max, Some(1));
    }
}

#[test]
fn qualified_union_target_legality_keeps_common_domain_and_local_exclusion() {
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for (name, text, serial) in [
        ("Abuelo's Awakening", AWAKENING, false),
        ("Excava, the Risen Past", EXCAVA, true),
    ] {
        let definition = compile_to_runtime_definition(name, text, false).unwrap();
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = game.create_object_from_definition(
            &definition,
            alice,
            if serial {
                Zone::Battlefield
            } else {
                Zone::Stack
            },
        );
        let mut cases = Vec::new();
        for (types, subtypes, owner, zone, mana_value, legal) in [
            (
                vec![CardType::Artifact],
                vec![],
                alice,
                Zone::Graveyard,
                3,
                true,
            ),
            (
                vec![CardType::Enchantment],
                vec![],
                alice,
                Zone::Graveyard,
                3,
                true,
            ),
            (
                vec![CardType::Creature],
                vec![],
                alice,
                Zone::Graveyard,
                3,
                serial,
            ),
            (
                vec![CardType::Enchantment],
                vec![Subtype::Aura],
                alice,
                Zone::Graveyard,
                3,
                false,
            ),
            (
                vec![CardType::Artifact, CardType::Enchantment],
                vec![Subtype::Aura],
                alice,
                Zone::Graveyard,
                3,
                true,
            ),
            (
                vec![CardType::Artifact],
                vec![],
                bob,
                Zone::Graveyard,
                3,
                false,
            ),
            (
                vec![CardType::Enchantment],
                vec![],
                bob,
                Zone::Graveyard,
                3,
                false,
            ),
            (
                vec![CardType::Artifact],
                vec![],
                alice,
                Zone::Battlefield,
                3,
                false,
            ),
            (
                vec![CardType::Enchantment],
                vec![],
                alice,
                Zone::Hand,
                3,
                false,
            ),
            (
                vec![CardType::Artifact],
                vec![],
                alice,
                Zone::Graveyard,
                4,
                !serial,
            ),
            (
                vec![CardType::Enchantment],
                vec![],
                alice,
                Zone::Graveyard,
                4,
                !serial,
            ),
        ] {
            let card = fixture(types, subtypes, mana_value);
            cases.push((
                game.create_object_from_definition(&card, owner, zone),
                legal,
            ));
        }
        let requirements = extract_target_requirements_from_program_with_modes(
            &game,
            return_program(&definition),
            alice,
            Some(source),
            None,
        );
        assert_eq!(
            requirements.len(),
            1,
            "a union must not invent modal choices: {name}: {requirements:#?}"
        );
        assert_eq!(requirements[0].min_targets, if serial { 0 } else { 1 });
        assert_eq!(requirements[0].max_targets, Some(1));
        for (object, legal) in cases {
            assert_eq!(
                requirements[0]
                    .legal_targets
                    .contains(&Target::Object(object)),
                legal,
                "{name}: {:?}",
                game.object(object)
            );
        }
    }
}

#[test]
fn qualified_union_returns_selected_card_with_entry_counters_and_spirit_animation() {
    let alice = PlayerId::from_index(0);
    for (name, text, serial) in [
        ("Abuelo's Awakening", AWAKENING, false),
        ("Excava, the Risen Past", EXCAVA, true),
    ] {
        let definition = compile_to_runtime_definition(name, text, false).unwrap();
        for card_type in [CardType::Artifact, CardType::Enchantment] {
            for x in [0, 2] {
                let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
                let source = game.create_object_from_definition(
                    &definition,
                    alice,
                    if serial {
                        Zone::Battlefield
                    } else {
                        Zone::Stack
                    },
                );
                let card =
                    CardDefinitionBuilder::new(CardId::new(), "Intrinsic entry counter target")
                        .card_types(vec![card_type])
                        .with_ability(ironsmith::Ability::static_ability(
                            StaticAbility::enters_with_counters(CounterType::PlusOnePlusOne, 1),
                        ))
                        .build();
                let target = game.create_object_from_definition(&card, alice, Zone::Graveyard);
                let stable = game.object(target).unwrap().stable_id;
                let untouched = game.create_object_from_definition(&card, alice, Zone::Graveyard);
                let mut context = EffectContext::new_default(source, alice)
                    .with_x(x)
                    .with_targets(vec![ResolvedTarget::Object(target)]);
                for effect in return_program(&definition).flattened_default_effects() {
                    ironsmith::effects::execute_effect(&mut game, effect, &mut context).unwrap();
                }
                let returned = game.find_object_by_stable_id(stable).unwrap();
                let object = game.object(returned).unwrap();
                assert_eq!(object.zone, Zone::Battlefield, "{name}");
                let plus_counters = if serial { 1 } else { x + 1 };
                assert_eq!(
                    object.counters.get(&CounterType::PlusOnePlusOne).copied(),
                    Some(plus_counters)
                );
                assert_eq!(
                    object
                        .counters
                        .get(&CounterType::Finality)
                        .copied()
                        .unwrap_or(0),
                    u32::from(serial)
                );
                assert_eq!(
                    game.calculated_power(returned),
                    Some((1 + plus_counters) as i32)
                );
                assert_eq!(
                    game.calculated_toughness(returned),
                    Some((1 + plus_counters) as i32)
                );
                let types = game.current_card_types(returned).unwrap();
                assert!(types.contains(&card_type) && types.contains(&CardType::Creature));
                assert!(
                    game.current_subtypes(returned)
                        .unwrap()
                        .contains(&Subtype::Spirit)
                );
                assert!(game.object_has_static_ability_id(returned, StaticAbilityId::Flying));
                assert_eq!(game.object(untouched).unwrap().zone, Zone::Graveyard);
            }
        }
    }
}
