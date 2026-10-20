//! Claude package 12 runtime checks for the compilation-completion task
//! (see reports/compilation-completion-20261016/COORDINATION.md).
#![allow(unused_imports)]
use super::compile_to_runtime_definition;
use ironsmith::effect::Effect;
use ironsmith::{PlayerId, Zone};

fn p12_all_effects(definition: &ironsmith::cards::CardDefinition) -> Vec<Effect> {
    fn walk(effect: &Effect, out: &mut Vec<Effect>) {
        out.push(effect.clone());
        effect.visit_child_effects(&mut |child| walk(child, out));
    }
    let mut out = Vec::new();
    if let Some(program) = &definition.spell_effect {
        for effect in program.all_effects() {
            walk(effect, &mut out);
        }
    }
    for ability in &definition.abilities {
        match &ability.kind {
            ironsmith::ability::AbilityKind::Activated(activated) => {
                for effect in activated.effects.all_effects() {
                    walk(effect, &mut out);
                }
            }
            ironsmith::ability::AbilityKind::Triggered(triggered) => {
                for effect in triggered.effects.all_effects() {
                    walk(effect, &mut out);
                }
            }
            _ => {}
        }
    }
    out
}

#[test]
fn open_the_omenpaths_adds_two_of_one_color_and_two_of_another() {
    let definition = compile_to_runtime_definition(
        "Open the Omenpaths",
        "Mana cost: {2}{R}\nType: Instant\nChoose one —\n• Add two mana of any one color and two mana of any other color. Spend this mana only to cast creature or enchantment spells.\n• Creatures you control get +1/+0 until end of turn.",
        false,
    )
    .expect("Open the Omenpaths should compile");
    let effects = p12_all_effects(&definition);
    assert!(effects.iter().any(|effect| effect.downcast_ref::<ironsmith::effects::ManaRestrictedEffect>().is_some()));
    let choice = effects
        .iter()
        .filter_map(|effect| effect.downcast_ref::<ironsmith::effects::ChooseModeEffect>())
        .find(|choice| choice.modes.len() == 10)
        .expect("ten color-pair outputs");
    let mut pairs = Vec::new();
    for mode in &choice.modes {
        let add = mode
            .effects
            .iter()
            .find_map(|effect| effect.downcast_ref::<ironsmith::effects::AddManaEffect>())
            .expect("fixed add");
        assert_eq!(add.mana.len(), 4);
        let mut colors = add.mana.clone();
        colors.dedup();
        assert_eq!(colors.len(), 2, "two of each of two different colors: {:?}", add.mana);
        pairs.push(colors);
    }
    pairs.sort_by_key(|pair| format!("{pair:?}"));
    pairs.dedup();
    assert_eq!(pairs.len(), 10);
}

#[test]
fn paragon_of_modernity_counts_colors_spent_to_activate() {
    let definition = compile_to_runtime_definition(
        "Paragon of Modernity",
        "Mana cost: {4}\nType: Artifact Creature — Angel Warrior\nPower/Toughness: 2/2\nFlying\n{3}: This creature gets +1/+1 until end of turn. If exactly three colors of mana were spent to activate this ability, put a +1/+1 counter on it instead.",
        false,
    )
    .expect("Paragon of Modernity should compile");
    let debug = format!("{definition:?}");
    assert!(
        debug.contains("ColorsOfManaSpentToActivateThisAbility { amount: 3, exactly: true }"),
        "{debug}"
    );
    let activated = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            ironsmith::ability::AbilityKind::Activated(activated) if !activated.effects.all_effects().is_empty() => Some(activated.clone()),
            _ => None,
        })
        .expect("pump ability");
    for (payment, expect_counter) in [
        (ironsmith::player::ManaPool { white: 1, blue: 1, black: 1, ..Default::default() }, true),
        (ironsmith::player::ManaPool { white: 2, blue: 1, ..Default::default() }, false),
    ] {
        let mut game = ironsmith::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = PlayerId::from_index(0);
        let paragon = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        game.push_to_stack(ironsmith::game_state::StackEntry::ability(
            paragon, alice, activated.effects.clone(),
        ).with_mana_spent_on_activation(payment));
        ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
        assert_eq!(
            game.counter_count(paragon, ironsmith::object::CounterType::PlusOnePlusOne),
            u32::from(expect_counter)
        );
    }
}

#[test]
fn rally_the_horde_repeats_until_the_last_exiled_card_is_a_land() {
    use ironsmith::card::CardBuilder;
    use ironsmith::ids::CardId;
    use ironsmith::types::CardType;
    let definition = compile_to_runtime_definition(
        "Rally the Horde",
        "Mana cost: {5}{R}\nType: Sorcery\nExile the top card of your library. Exile the top card of your library. Exile the top card of your library. If the last card exiled isn't a land card, repeat this process. Create a 1/1 red Warrior creature token for each nonland card exiled this way.",
        false,
    )
    .expect("Rally the Horde should compile");
    let effects = p12_all_effects(&definition);
    let repeat = effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<ironsmith::effects::RepeatProcessEffect>())
        .expect("repeat-process loop");
    let exiles = repeat
        .effects
        .iter()
        .filter(|effect| effect.downcast_ref::<ironsmith::effects::ExileTopOfLibraryEffect>().is_some())
        .count();
    assert_eq!(exiles, 3, "{repeat:?}");

    let mut game = ironsmith::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = PlayerId::from_index(0);
    // Library from bottom to top; the top is the last card created.
    for is_land in [false, false, true, false, false, false, false, false] {
        let card = CardBuilder::new(CardId::new(), if is_land { "Land" } else { "Spell" })
            .card_types(vec![if is_land { CardType::Land } else { CardType::Sorcery }])
            .build();
        game.create_object_from_card(&card, alice, Zone::Library);
    }
    let rally = game.create_object_from_definition(&definition, alice, Zone::Stack);
    let before = game.battlefield.len();
    let mut ctx = ironsmith::effects::EffectContext::new_default(rally, alice);
    for effect in definition.spell_effect.as_ref().unwrap().all_effects() {
        ironsmith::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
    }
    drop(ctx);
    assert_eq!(game.player(alice).unwrap().library.len(), 2, "two rounds of three exiles");
    assert_eq!(game.battlefield.len() - before, 5, "one Warrior per nonland card exiled");
}

#[test]
fn outmaneuver_grants_mandatory_unblocked_assignment_to_x_targets() {
    let definition = compile_to_runtime_definition(
        "Outmaneuver",
        "Mana cost: {X}{R}\nType: Instant\nX target blocked creatures assign their combat damage this turn as though they weren't blocked.",
        false,
    )
    .expect("Outmaneuver should compile");
    let debug = format!("{definition:?}");
    assert!(debug.contains("AssignsDamageAsUnblocked"), "{debug}");
    assert!(!debug.contains("MayAssignDamageAsUnblocked"), "the assignment is mandatory: {debug}");
    assert!(debug.contains("EndOfTurn"), "{debug}");
    assert!(debug.contains("blocked: true") || debug.contains("Blocked"), "targets must be blocked creatures: {debug}");
}

#[test]
fn quakebringer_upkeep_trigger_functions_from_battlefield_or_graveyard_with_a_giant() {
    let definition = compile_to_runtime_definition(
        "Quakebringer",
        "Mana cost: {3}{R}{R}\nType: Creature — Giant Berserker\nPower/Toughness: 5/4\nYour opponents can't gain life.\nAt the beginning of your upkeep, Quakebringer deals 2 damage to each opponent. This ability triggers only if Quakebringer is on the battlefield or if Quakebringer is in your graveyard and you control a Giant.\nForetell {2}{R}{R}",
        false,
    )
    .expect("Quakebringer should compile");
    let ability = definition
        .abilities
        .iter()
        .find(|ability| matches!(ability.kind, ironsmith::ability::AbilityKind::Triggered(_)))
        .expect("upkeep trigger");
    assert!(ability.functional_zones.contains(&Zone::Battlefield), "{:?}", ability.functional_zones);
    assert!(ability.functional_zones.contains(&Zone::Graveyard), "{:?}", ability.functional_zones);
    let ironsmith::ability::AbilityKind::Triggered(triggered) = &ability.kind else { unreachable!() };
    let condition = format!("{:?}", triggered.intervening_if);
    assert!(
        condition.starts_with("Some(TriggerEventOnly(Or(SourceIsInZone(Battlefield), And(SourceIsInZone(Graveyard)"),
        "the graveyard arm alone requires a Giant: {condition}"
    );
}

#[test]
fn overclocked_electromancer_gains_energy_equal_to_the_triggering_excess_damage() {
    let definition = compile_to_runtime_definition(
        "Overclocked Electromancer",
        "Mana cost: {2}{R}\nType: Creature — Lizard Wizard\nPower/Toughness: 1/3\nAt the beginning of combat on your turn, you may pay {E}{E}{E}. If you do, put a +1/+1 counter on this creature.\nWhenever this creature attacks, double its power until end of turn.\nWhenever this creature deals combat damage to a creature, if that creature was dealt excess damage this turn, you get X {E}, where X is that excess damage.",
        false,
    )
    .expect("Overclocked Electromancer should compile");
    let triggered = definition
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            ironsmith::ability::AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .find(|triggered| format!("{:?}", triggered.trigger).contains("ThisDealsDamageToTrigger"))
        .expect("combat damage trigger");
    let condition = format!("{:?}", triggered.intervening_if);
    assert!(condition.contains("EventValue(ExcessDamage)"), "{condition}");
    let energy = triggered
        .effects
        .all_effects()
        .into_iter()
        .find_map(|effect| effect.downcast_ref::<ironsmith::effects::EnergyCountersEffect>())
        .expect("energy gain");
    assert!(matches!(energy.count.unhinted(), ironsmith::effect::Value::EventValue(ironsmith::effect::EventValueSpec::ExcessDamage)), "{:?}", energy.count);
}

#[test]
fn phyrexian_splicer_chooses_the_keyword_on_activation_as_a_mode() {
    let definition = compile_to_runtime_definition(
        "Phyrexian Splicer",
        "Mana cost: {2}\nType: Artifact\n{2}, {T}, Choose flying, first strike, trample, or shadow: Until end of turn, target creature with the chosen ability loses it and another target creature gains it.",
        false,
    )
    .expect("Phyrexian Splicer should compile");
    let activated = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            ironsmith::ability::AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .expect("activated ability");
    let choice = activated
        .effects
        .all_effects()
        .into_iter()
        .find_map(|effect| effect.downcast_ref::<ironsmith::effects::ChooseModeEffect>())
        .expect("activation-time keyword choice");
    assert_eq!(choice.modes.len(), 4);
    for (mode, keyword) in choice.modes.iter().zip(["flying", "first strike", "trample", "shadow"]) {
        assert!(mode.source_text.to_ascii_lowercase().contains(keyword), "{}", mode.source_text);
        assert!(!mode.source_text.contains("chosen"), "{}", mode.source_text);
    }
    let debug = format!("{activated:?}");
    assert!(debug.contains("Tap") || debug.contains("tap"), "the {{T}} cost stays: {debug}");
}

#[test]
fn psychotic_episode_chooses_from_the_targets_hand_and_library_top() {
    use ironsmith::card::CardBuilder;
    use ironsmith::ids::CardId;
    use ironsmith::types::CardType;
    let definition = compile_to_runtime_definition(
        "Psychotic Episode",
        "Mana cost: {1}{B}{B}\nType: Sorcery\nTarget player reveals their hand and the top card of their library. You choose a card revealed this way. That player puts the chosen card on the bottom of their library.\nMadness {1}{B} (If you discard this card, discard it into exile. When you do, cast it for its madness cost or put it into your graveyard.)",
        false,
    )
    .expect("Psychotic Episode should compile");
    let effects = p12_all_effects(&definition);
    let choice = effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<ironsmith::effects::ChooseObjectsEffect>())
        .expect("a choice over the revealed pool");
    assert_eq!(choice.chooser, ironsmith::target::PlayerFilter::You);
    assert_ne!(choice.filter.controller, Some(ironsmith::target::PlayerFilter::You), "the pool is the target's cards: {choice:?}");
    let zones: Vec<_> = choice.zone.into_iter().chain(choice.additional_zones.iter().copied()).collect();
    assert!(zones.contains(&Zone::Hand) && zones.contains(&Zone::Library), "{zones:?}");

    let mut game = ironsmith::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let card = |name: &str| CardBuilder::new(CardId::new(), name).card_types(vec![CardType::Sorcery]).build();
    game.create_object_from_card(&card("Bottom"), bob, Zone::Library);
    game.create_object_from_card(&card("Top"), bob, Zone::Library);
    game.create_object_from_card(&card("Bob Hand"), bob, Zone::Hand);
    game.create_object_from_card(&card("Alice Hand"), alice, Zone::Hand);
    let spell = game.create_object_from_definition(&definition, alice, Zone::Stack);
    let mut ctx = ironsmith::effects::EffectContext::new_default(spell, alice)
        .with_targets(vec![ironsmith::effects::ResolvedTarget::Player(bob)]);
    for effect in definition.spell_effect.as_ref().unwrap().all_effects() {
        ironsmith::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
    }
    drop(ctx);
    let name = |id: &ironsmith::ids::ObjectId| game.object(*id).unwrap().name.to_string();
    assert_eq!(game.player(alice).unwrap().hand.len(), 1, "your own hand is not the pool");
    let bob_library: Vec<String> = game.player(bob).unwrap().library.iter().map(name).collect();
    assert_eq!(bob_library.len() + game.player(bob).unwrap().hand.len(), 3);
    assert!(
        bob_library.first().is_some_and(|bottom| bottom == "Top" || bottom == "Bob Hand"),
        "the chosen revealed card goes to the bottom: {bob_library:?}"
    );
}

#[test]
fn preston_garvey_settlement_token_grants_its_nested_mana_ability() {
    let definition = compile_to_runtime_definition(
        "Preston Garvey, Minuteman",
        "Mana cost: {2}{R}{G}{W}\nType: Legendary Creature — Human Soldier\nPower/Toughness: 4/4\nAt the beginning of combat on your turn, create a green Aura enchantment token named Settlement attached to up to one target land you control. It has enchant land and \"Enchanted land has '{T}: Add one mana of any color.'\"\nWhenever Preston Garvey attacks, untap each enchanted permanent you control.",
        false,
    )
    .expect("Preston Garvey should compile");
    let effects = p12_all_effects(&definition);
    let create = effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<ironsmith::effects::CreateTokenEffect>())
        .expect("Settlement token");
    let debug = format!("{create:?}");
    assert!(debug.contains("Settlement"), "{debug}");
    assert!(debug.contains("AddManaOfAnyColorEffect"), "the nested '{{T}}: Add one mana of any color.' must survive: {debug}");
    assert!(debug.contains("Enchant"), "{debug}");
}

#[test]
fn primitive_justice_targets_one_artifact_plus_one_per_additional_payment() {
    let definition = compile_to_runtime_definition(
        "Primitive Justice",
        "Mana cost: {1}{R}\nType: Sorcery\nAs an additional cost to cast this spell, you may pay {1}{R} and/or {1}{G} any number of times.\nDestroy target artifact. For each additional {1}{R} you paid, destroy another target artifact. For each additional {1}{G} you paid, destroy another target artifact, and you gain 1 life.",
        false,
    )
    .expect("Primitive Justice should compile");
    assert_eq!(definition.optional_costs.len(), 2, "{:?}", definition.optional_costs);
    assert!(definition.optional_costs.iter().all(|cost| cost.repeatable));
    let effects = p12_all_effects(&definition);
    let destroys: Vec<_> = effects
        .iter()
        .filter_map(|effect| effect.downcast_ref::<ironsmith::effects::DestroyEffect>())
        .collect();
    assert_eq!(destroys.len(), 1, "one destroy over every chosen artifact");
    let destroy = format!("{:?}", destroys[0]);
    assert!(destroy.contains("WithCountValue"), "{destroy}");
    assert_eq!(destroy.matches("TimesPaidLabel").count(), 2, "{destroy}");
    let repeat = effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<ironsmith::effects::RepeatEffectsEffect>())
        .expect("life per {1}{G} payment");
    assert!(format!("{:?}", repeat.count).contains("TimesPaidLabel"));
    assert!(format!("{repeat:?}").contains("GainLife"), "{repeat:?}");
}

#[test]
fn pyramids_replaces_the_lands_next_destruction_with_damage_removal() {
    use ironsmith::card::CardBuilder;
    use ironsmith::ids::CardId;
    use ironsmith::types::CardType;
    let definition = compile_to_runtime_definition(
        "Pyramids",
        "Mana cost: {6}\nType: Artifact\n{2}: Choose one —\n• Destroy target Aura attached to a land.\n• The next time target land would be destroyed this turn, remove all damage marked on it instead.",
        false,
    )
    .expect("Pyramids should compile");
    let effects = p12_all_effects(&definition);
    let shield = effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<ironsmith::effects::RegenerateEffect>())
        .expect("one-shot destruction shield");
    assert!(shield.remove_damage_only, "{shield:?}");

    let mut game = ironsmith::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = PlayerId::from_index(0);
    let land = CardBuilder::new(CardId::new(), "Plain Land").card_types(vec![CardType::Land]).build();
    let land = game.create_object_from_card(&land, alice, Zone::Battlefield);
    let pyramids = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    game.mark_damage(land, 2);
    let mut ctx = ironsmith::effects::EffectContext::new_default(pyramids, alice)
        .with_targets(vec![ironsmith::effects::ResolvedTarget::Object(land)]);
    ironsmith::effects::execute_effect(&mut game, &Effect::new(shield.clone()), &mut ctx).unwrap();
    assert!(!game.is_tapped(land), "not regeneration: the land is not tapped yet");
    let destroy = Effect::destroy(ironsmith::target::ChooseSpec::SpecificObject(land));
    ironsmith::effects::execute_effect(&mut game, &destroy, &mut ctx).unwrap();
    assert!(game.battlefield.contains(&land), "the destruction was replaced");
    assert_eq!(game.damage_on(land), 0, "its damage was removed instead");
    assert!(!game.is_tapped(land), "a damage-removal shield does not tap");
    ironsmith::effects::execute_effect(&mut game, &destroy, &mut ctx).unwrap();
    assert!(!game.battlefield.contains(&land), "only the next destruction is replaced");
}

#[test]
fn opal_palace_commander_mana_carries_cast_count_entry_counters() {
    let definition = compile_to_runtime_definition(
        "Opal Palace",
        "Type: Land\n{T}: Add {C}.\n{1}, {T}: Add one mana of any color in your commander's color identity. If you spend this mana to cast your commander, it enters with a number of additional +1/+1 counters on it equal to the number of times it's been cast from the command zone this game.",
        false,
    )
    .expect("Opal Palace should compile");
    let bonus = definition
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            ironsmith::ability::AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .flat_map(|activated| activated.mana_usage_restrictions.iter())
        .find_map(|restriction| match restriction {
            ironsmith::ability::ManaUsageRestriction::CastSpellWithManaBonus {
                filter,
                enters_with_counter_values,
                ..
            } => Some((filter.clone(), enters_with_counter_values.clone())),
            _ => None,
        })
        .expect("commander-cast mana bonus");
    assert!(bonus.0.is_commander, "{:?}", bonus.0);
    assert_eq!(bonus.1.len(), 1);
    assert_eq!(bonus.1[0].0, ironsmith::object::CounterType::PlusOnePlusOne);
    assert!(matches!(bonus.1[0].1, ironsmith::effect::Value::CommanderCastCount(_)), "{:?}", bonus.1);
}

#[test]
fn parker_luck_players_lose_life_for_each_others_revealed_card() {
    use ironsmith::card::CardBuilder;
    use ironsmith::ids::CardId;
    use ironsmith::mana::{ManaCost, ManaSymbol};
    use ironsmith::types::CardType;
    let definition = compile_to_runtime_definition(
        "Parker Luck",
        "Mana cost: {2}{B}\nType: Enchantment\nAt the beginning of your end step, two target players each reveal the top card of their library. They each lose life equal to the mana value of the card revealed by the other player. Then they each put the card they revealed into their hand.",
        false,
    )
    .expect("Parker Luck should compile");
    let triggered = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            ironsmith::ability::AbilityKind::Triggered(triggered) => Some(triggered.clone()),
            _ => None,
        })
        .expect("end step trigger");
    let mut game = ironsmith::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let card = |name: &str, generic: u8| {
        CardBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Sorcery])
            .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Generic(generic)]))
            .build()
    };
    game.create_object_from_card(&card("Alice Top", 2), alice, Zone::Library);
    game.create_object_from_card(&card("Bob Top", 5), bob, Zone::Library);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let mut ctx = ironsmith::effects::EffectContext::new_default(source, alice).with_targets(vec![
        ironsmith::effects::ResolvedTarget::Player(alice),
        ironsmith::effects::ResolvedTarget::Player(bob),
    ]);
    for effect in triggered.effects.all_effects() {
        ironsmith::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
    }
    drop(ctx);
    assert_eq!(game.player(alice).unwrap().life, 15, "Alice loses the mana value of Bob's card");
    assert_eq!(game.player(bob).unwrap().life, 18, "Bob loses the mana value of Alice's card");
    assert_eq!(game.player(alice).unwrap().hand.len(), 1);
    assert_eq!(game.player(bob).unwrap().hand.len(), 1);
    assert_eq!(game.object(game.player(alice).unwrap().hand[0]).unwrap().name, "Alice Top");
    assert_eq!(game.object(game.player(bob).unwrap().hand[0]).unwrap().name, "Bob Top");
    assert!(game.object(source).is_some(), "unrelated source remains on the battlefield");
}

#[test]
fn professor_hojo_discounts_the_first_activation_targeting_your_creature() {
    let definition = compile_to_runtime_definition(
        "Professor Hojo",
        "Mana cost: {1}{G}\nType: Legendary Creature — Human Scientist\nPower/Toughness: 2/2\nThe first activated ability you activate during your turn that targets a creature you control costs {2} less to activate.\nWhenever one or more creatures you control become the target of an activated ability, draw a card. This ability triggers only once each turn.",
        false,
    )
    .expect("Professor Hojo should compile");
    let debug = format!("{definition:?}");
    assert!(debug.contains("FirstTargetingActivationDuringYourTurn"), "{debug}");
    assert!(debug.contains("ActivatedAbilityCostReduction"), "{debug}");
}

#[test]
fn prowl_lets_the_exiled_cards_owner_play_it_while_exiled() {
    let definition = compile_to_runtime_definition(
        "Prowl, Stoic Strategist // Prowl, Pursuit Vehicle",
        "Mana cost: {3}{W}\nType: Legendary Artifact Creature — Robot\nPower/Toughness: 3/3\nMore Than Meets the Eye {2}{W} (You may cast this card converted for {2}{W}.)\nWhenever Prowl attacks, exile up to one other target tapped creature or Vehicle. For as long as that card remains exiled, its owner may play it.\nWhenever a player plays a card exiled with Prowl, you draw a card and convert Prowl.",
        false,
    )
    .expect("Prowl's front face should compile");
    let debug = format!("{definition:?}");
    assert!(debug.contains("ExileEffect") || debug.contains("Exile"), "{debug}");
    assert!(debug.contains("OwnerOf") || debug.contains("Owner"), "the permission belongs to the card's owner: {debug}");
    assert!(debug.contains("ConvertEffect"), "{debug}");
}
