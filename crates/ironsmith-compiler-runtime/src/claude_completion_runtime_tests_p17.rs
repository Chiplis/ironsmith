//! Claude package 17 runtime checks for the compilation-completion task
//! (see reports/compilation-completion-20261016/COORDINATION.md).
#![allow(unused_imports)]
use super::compile_to_runtime_definition;
use ironsmith::effect::Effect;

fn p17_visit(effect: &Effect, f: &mut dyn FnMut(&Effect)) {
    f(effect);
    effect.visit_child_effects(&mut |child| p17_visit(child, f));
}

fn p17_triggered(definition: &ironsmith::cards::CardDefinition) -> Vec<&ironsmith::ability::TriggeredAbility> {
    definition.abilities.iter().filter_map(|ability| match &ability.kind {
        ironsmith::ability::AbilityKind::Triggered(trigger) => Some(trigger),
        _ => None,
    }).collect()
}

#[test]
fn rhinos_rampage_reflexive_trigger_reads_excess_damage_to_the_fought_creature() {
    use ironsmith::card::{CardBuilder, PowerToughness};
    use ironsmith::effect::EffectPredicateRuntimeExt as _;
    use ironsmith::effects::{EffectContext, ResolvedTarget, execute_effect};
    use ironsmith::ids::CardId;
    use ironsmith::types::CardType;
    use ironsmith::{PlayerId, Zone};
    let definition = compile_to_runtime_definition("Rhino's Rampage",
        "Mana cost: {R/G}\nType: Sorcery\nTarget creature you control gets +1/+0 until end of turn. It fights target creature an opponent controls. When excess damage is dealt to the creature an opponent controls this way, destroy up to one target noncreature artifact with mana value 3 or less.", false)
        .expect("Rhino's Rampage should compile");
    let mut fight_id = None;
    let mut reflexive = None;
    for effect in definition.spell_effect.as_ref().expect("spell effect").all_effects() {
        p17_visit(effect, &mut |effect| {
            if let Some(with_id) = effect.downcast_ref::<ironsmith::effects::WithIdEffect>()
                && format!("{:?}", with_id.effect).contains("FightEffect")
            {
                fight_id = Some(with_id.id);
            }
            if let Some(trigger) = effect.downcast_ref::<ironsmith::effects::ReflexiveTriggerEffect>() {
                reflexive = Some(trigger.clone());
            }
        });
    }
    let reflexive = reflexive.expect("reflexive excess-damage trigger");
    assert_eq!(Some(reflexive.condition), fight_id, "the reflexive trigger reads the fight result");
    assert_eq!(reflexive.predicate, ironsmith::effect::EffectPredicate::ExcessDamageDealt);
    assert!(format!("{:?}", reflexive.effects).contains("Destroy"), "{:?}", reflexive.effects);
    assert_eq!(reflexive.choices.len(), 1, "the up-to-one artifact target is chosen by the reflexive ability");

    // Only excess damage dealt to the fought (second) creature counts.
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for (mine, theirs, expected) in [((5, 5), (2, 2), true), ((1, 1), (5, 5), false), ((2, 2), (2, 2), false)] {
        let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let card = |name: &str, (power, toughness): (i32, i32)| CardBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(power, toughness))
            .build();
        let first = game.create_object_from_card(&card("Mine", mine), alice, Zone::Battlefield);
        let second = game.create_object_from_card(&card("Theirs", theirs), bob, Zone::Battlefield);
        game.refresh_continuous_state().unwrap();
        let fight = Effect::new(ironsmith::effects::FightEffect::new(
            ironsmith::target::ChooseSpec::creature(),
            ironsmith::target::ChooseSpec::creature(),
        ));
        let mut ctx = EffectContext::new_default(first, alice)
            .with_targets(vec![ResolvedTarget::Object(first), ResolvedTarget::Object(second)]);
        let outcome = execute_effect(&mut game, &fight, &mut ctx).unwrap();
        assert_eq!(ironsmith::effect::EffectPredicate::ExcessDamageDealt.evaluate_outcome(&outcome), expected,
            "mine {mine:?} vs theirs {theirs:?}: {:?}", outcome.execution_facts);
    }
}

#[test]
fn rite_of_ruin_chooses_a_type_order_for_positional_sacrifices() {
    use ironsmith::card::CardBuilder;
    use ironsmith::effects::{EffectContext, execute_effect};
    use ironsmith::ids::CardId;
    use ironsmith::types::CardType;
    use ironsmith::{PlayerId, Zone};
    let definition = compile_to_runtime_definition("Rite of Ruin",
        "Mana cost: {5}{R}{R}\nType: Sorcery\nChoose an order for artifacts, creatures, and lands. Each player sacrifices one permanent of their choice of the first type, sacrifices two of their choice of the second type, then sacrifices three of their choice of the third type.", false)
        .expect("Rite of Ruin should compile");
    let spell = definition.spell_effect.as_ref().expect("spell effect");
    let mut modal = None;
    for effect in spell.all_effects() {
        p17_visit(effect, &mut |effect| {
            if let Some(choice) = effect.downcast_ref::<ironsmith::effects::ChooseModeEffect>() {
                modal = Some(choice.clone());
            }
        });
    }
    let modal = modal.expect("type-order choice");
    assert_eq!(modal.modes.len(), 6, "one mode per ordering of three types");

    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let kinds = [CardType::Artifact, CardType::Creature, CardType::Land];
    for player in [alice, bob] {
        for kind in kinds {
            for idx in 0..3 {
                let card = CardBuilder::new(CardId::new(), &format!("{kind:?} {idx}")).card_types(vec![kind]).build();
                game.create_object_from_card(&card, player, Zone::Battlefield);
            }
        }
    }
    let source_card = CardBuilder::new(CardId::new(), "Rite source").card_types(vec![CardType::Sorcery]).build();
    let source = game.create_object_from_card(&source_card, alice, Zone::Stack);
    game.refresh_continuous_state().unwrap();
    let mut ctx = EffectContext::new_default(source, alice);
    for effect in spell.all_effects() {
        execute_effect(&mut game, effect, &mut ctx).unwrap();
    }
    drop(ctx);
    let remaining = |player: PlayerId| kinds.map(|kind| game.battlefield.iter().filter_map(|id| game.object(*id))
        .filter(|object| game.controller_of(object) == player && object.card_types.contains(&kind)).count());
    let alice_left = remaining(alice);
    let mut sorted = alice_left;
    sorted.sort_unstable();
    assert_eq!(sorted, [0, 1, 2], "one, two and three permanents of the three ordered types: {alice_left:?}");
    assert_eq!(remaining(bob), alice_left, "every player follows the same chosen order");
}

#[test]
fn resourceful_defense_places_departed_counters_and_moves_any_kinds_between_targets() {
    use ironsmith::card::CardBuilder;
    use ironsmith::effects::{EffectContext, ResolvedTarget, execute_effect};
    use ironsmith::ids::CardId;
    use ironsmith::object::CounterType;
    use ironsmith::snapshot::ObjectSnapshot;
    use ironsmith::types::CardType;
    use ironsmith::{PlayerId, Zone};
    let definition = compile_to_runtime_definition("Resourceful Defense",
        "Mana cost: {2}{W}\nType: Enchantment\nWhenever a permanent you control leaves the battlefield, if it had counters on it, put those counters on target permanent you control.\n{4}{W}: Move any number of counters from target permanent you control onto a second target permanent you control.", false)
        .expect("Resourceful Defense should compile");
    let trigger = p17_triggered(&definition).into_iter().next().expect("leaves trigger");
    let mut placement = None;
    for effect in trigger.effects.all_effects() {
        p17_visit(effect, &mut |effect| {
            if let Some(model) = effect.downcast_ref::<ironsmith::effects::MoveAllCountersEffect>() {
                placement = Some(model.clone());
            }
        });
    }
    let placement = placement.expect("those counters are placed as a collection");
    assert!(!placement.remove_from_source, "placing a departed permanent's counters is not a move: {placement:?}");
    assert!(placement.to.is_target(), "{placement:?}");
    let gated = trigger.intervening_if.is_some() || format!("{:?}", trigger.effects).contains("CountersOn");
    assert!(gated, "the trigger requires the permanent to have had counters: {:?}", trigger);

    let mut transfer = None;
    for ability in &definition.abilities {
        if let ironsmith::ability::AbilityKind::Activated(activated) = &ability.kind {
            for effect in activated.effects.all_effects() {
                p17_visit(effect, &mut |effect| {
                    if let Some(model) = effect.downcast_ref::<ironsmith::effects::MoveCountersEffect>() {
                        transfer = Some(model.clone());
                    }
                });
            }
        }
    }
    let transfer = transfer.expect("activated counter transfer");
    assert_eq!(transfer.count, ironsmith_core::effect::CounterMoveAmount::AnyNumberOfAnyKind, "{transfer:?}");
    assert!(transfer.from.is_target() && transfer.to.is_target(), "{transfer:?}");

    // The departed permanent's last-known counters land on the target.
    let alice = PlayerId::from_index(0);
    let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let card = CardBuilder::new(CardId::new(), "Counter holder").card_types(vec![CardType::Artifact]).build();
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let departed = game.create_object_from_card(&card, alice, Zone::Battlefield);
    let recipient = game.create_object_from_card(&card, alice, Zone::Battlefield);
    game.add_counters(departed, CounterType::PlusOnePlusOne, 2);
    game.add_counters(departed, CounterType::Charge, 1);
    let snapshot = ObjectSnapshot::from_object_id(&game, departed).unwrap();
    game.move_object(departed, Zone::Graveyard, ironsmith::events::EventCause::effect());
    let departure = ironsmith::triggers::TriggerEvent::new(
        ironsmith::events::ZoneChangeEvent::with_cause(
            departed, Zone::Battlefield, Zone::Graveyard,
            ironsmith::events::EventCause::effect(), Some(snapshot.clone()),
        ), Default::default(),
    );
    let mut ctx = EffectContext::new_default(source, alice)
        .with_targets(vec![ResolvedTarget::Object(recipient)])
        .with_triggering_event(departure);
    ctx.set_tagged_objects("triggering", vec![snapshot.clone()]);
    ctx.set_tagged_objects("it", vec![snapshot]);
    for effect in trigger.effects.all_effects() {
        execute_effect(&mut game, effect, &mut ctx).unwrap();
    }
    drop(ctx);
    assert_eq!(game.counter_count(recipient, CounterType::PlusOnePlusOne), 2);
    assert_eq!(game.counter_count(recipient, CounterType::Charge), 1);
    let destination = game.create_object_from_card(&card, alice, Zone::Battlefield);
    let activated = definition.abilities.iter().find_map(|ability| match &ability.kind {
        ironsmith::ability::AbilityKind::Activated(activated) => Some(activated), _ => None,
    }).unwrap();
    let mut ctx = EffectContext::new_default(source, alice)
        .with_targets(vec![ResolvedTarget::Object(recipient), ResolvedTarget::Object(destination)]);
    for effect in activated.effects.all_effects() {
        execute_effect(&mut game, effect, &mut ctx).unwrap();
    }
    drop(ctx);
    assert_eq!(game.counter_count(recipient, CounterType::PlusOnePlusOne), 0);
    assert_eq!(game.counter_count(recipient, CounterType::Charge), 0);
    assert_eq!(game.counter_count(destination, CounterType::PlusOnePlusOne), 2);
    assert_eq!(game.counter_count(destination, CounterType::Charge), 1);
}

#[test]
fn sculpted_sunburst_opponents_choose_by_relative_power_and_unchosen_are_exiled() {
    use ironsmith::card::{CardBuilder, PowerToughness};
    use ironsmith::effects::{EffectContext, execute_effect};
    use ironsmith::ids::CardId;
    use ironsmith::types::CardType;
    use ironsmith::{PlayerId, Zone};
    let definition = compile_to_runtime_definition("Sculpted Sunburst",
        "Mana cost: {3}{W}{W}\nType: Sorcery\nChoose a creature you control, then each opponent chooses a creature they control with equal or lesser power. If you chose a creature this way, exile each creature not chosen by any player this way.", false)
        .expect("Sculpted Sunburst should compile");
    let spell = definition.spell_effect.as_ref().expect("spell effect");
    let mut choices = 0;
    for effect in spell.all_effects() {
        p17_visit(effect, &mut |effect| {
            if effect.downcast_ref::<ironsmith::effects::ChooseObjectsEffect>().is_some() {
                choices += 1;
            }
        });
    }
    assert_eq!(choices, 2, "your choice and each opponent's choice");
    let debug = format!("{spell:?}");
    assert!(debug.contains("LessThanOrEqualExpr") && debug.contains("PowerOf"), "{debug}");
    assert_eq!(debug.matches("IsNotTaggedObject").count(), 2, "both chosen sets are spared: {debug}");

    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let creature = |name: &str, power: i32| CardBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(power, power))
        .build();
    let anchor = game.create_object_from_card(&creature("Anchor", 3), alice, Zone::Battlefield);
    let small = game.create_object_from_card(&creature("Small", 2), bob, Zone::Battlefield);
    let big = game.create_object_from_card(&creature("Big", 4), bob, Zone::Battlefield);
    let source_card = CardBuilder::new(CardId::new(), "Sunburst source").card_types(vec![CardType::Sorcery]).build();
    let source = game.create_object_from_card(&source_card, alice, Zone::Stack);
    game.refresh_continuous_state().unwrap();
    let mut ctx = EffectContext::new_default(source, alice);
    for effect in spell.all_effects() {
        execute_effect(&mut game, effect, &mut ctx).unwrap();
    }
    drop(ctx);
    let on_battlefield = |id| game.battlefield.iter().any(|object| *object == id);
    assert!(on_battlefield(anchor), "your chosen creature stays");
    assert!(on_battlefield(small), "the opponent could only choose the creature with lesser power");
    assert!(!on_battlefield(big), "an unchosen creature is exiled");
}

#[test]
fn shambling_swarm_schedules_one_removal_per_counter_put_on_each_creature() {
    use ironsmith::card::{CardBuilder, PowerToughness};
    use ironsmith::effects::{EffectContext, ResolvedTarget, execute_effect};
    use ironsmith::ids::CardId;
    use ironsmith::object::CounterType;
    use ironsmith::types::CardType;
    use ironsmith::{PlayerId, Zone};
    let definition = compile_to_runtime_definition("Shambling Swarm",
        "Mana cost: {1}{B}{B}{B}\nType: Creature — Horror\nPower/Toughness: 3/3\nWhen this creature dies, distribute three -1/-1 counters among one, two, or three target creatures. For each -1/-1 counter you put on a creature this way, remove a -1/-1 counter from that creature at the beginning of the next end step.", false)
        .expect("Shambling Swarm should compile");
    let trigger = p17_triggered(&definition).into_iter().next().expect("dies trigger");
    let mut repeat = None;
    for effect in trigger.effects.all_effects() {
        p17_visit(effect, &mut |effect| {
            if let Some(model) = effect.downcast_ref::<ironsmith::effects::RepeatEffectsEffect>() {
                repeat = Some(model.clone());
            }
        });
    }
    let repeat = repeat.expect("per-counter repetition");
    let ironsmith::effect::Value::PriorEffectMetric { query, .. } = &repeat.count else {
        panic!("the repetition counts the placement's own counters: {:?}", repeat.count);
    };
    assert_eq!(query.metric, ironsmith::effect::EffectMetric::CountersPut);
    assert_eq!(query.counter_type, Some(CounterType::MinusOneMinusOne));
    assert!(format!("{:?}", repeat.effects).contains("ScheduleDelayedTrigger"), "{:?}", repeat.effects);

    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let creature = |name: &str| CardBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(5, 5))
        .build();
    let first = game.create_object_from_card(&creature("First"), bob, Zone::Battlefield);
    let second = game.create_object_from_card(&creature("Second"), bob, Zone::Battlefield);
    let bystander = game.create_object_from_card(&creature("Bystander"), bob, Zone::Battlefield);
    // A pre-existing counter is not one put this way.
    game.add_counters(first, CounterType::MinusOneMinusOne, 1);
    let source = game.create_object_from_definition(&definition, alice, Zone::Graveyard);
    game.refresh_continuous_state().unwrap();
    let before = game.effect_store.delayed_triggers.len();
    let target_spec = trigger.effects.all_effects().into_iter()
        .find_map(|effect| effect.0.get_target_spec()).unwrap().clone();
    let mut ctx = EffectContext::new_default(source, alice)
        .with_targets(vec![ResolvedTarget::Object(first), ResolvedTarget::Object(second)])
        .with_target_assignments(vec![ironsmith::game_state::TargetAssignment {
            spec: target_spec.clone(), range: 0..2,
        }])
        .with_target_distributions(vec![ironsmith::game_state::TargetDistribution {
            spec: target_spec, range: 0..2,
            allocations: vec![(ironsmith::game_state::Target::Object(first), 2),
                (ironsmith::game_state::Target::Object(second), 1)],
        }]);
    for effect in trigger.effects.all_effects() {
        execute_effect(&mut game, effect, &mut ctx).unwrap();
    }
    drop(ctx);
    let put_on = |id, existing: u32| game.counter_count(id, CounterType::MinusOneMinusOne) - existing;
    let (first_put, second_put) = (put_on(first, 1), put_on(second, 0));
    assert_eq!(first_put + second_put, 3, "all three counters are distributed");
    let delayed = &game.effect_store.delayed_triggers[before..];
    assert_eq!(delayed.len(), 3, "one delayed removal per counter put");
    // Each registration captures the iterated creature it removes from.
    let it = ironsmith::tag::TagKey::from("__it__");
    let watching = |id| delayed.iter().filter(|trigger| trigger.tagged_objects.get(&it)
        .is_some_and(|snapshots| snapshots.iter().any(|snapshot| snapshot.object_id == id))).count() as u32;
    assert_eq!(watching(first), first_put);
    assert_eq!(watching(second), second_put);
    assert_eq!(watching(bystander), 0);
    let event = ironsmith::triggers::TriggerEvent::new(
        ironsmith::events::BeginningOfEndStepEvent::new(bob), Default::default());
    let entries = ironsmith::triggers::check_delayed_triggers(&mut game, &event);
    assert_eq!(entries.len(), 3);
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    for entry in entries { queue.add(entry); }
    ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
    while !game.stack.is_empty() { ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap(); }
    assert_eq!(game.counter_count(first, CounterType::MinusOneMinusOne), 1);
    assert_eq!(game.counter_count(second, CounterType::MinusOneMinusOne), 0);
    assert!(ironsmith::triggers::check_delayed_triggers(&mut game, &event).is_empty());
}

#[test]
fn ranar_first_foretell_each_turn_is_free_and_exile_batches_make_spirits() {
    let definition = compile_to_runtime_definition("Ranar the Ever-Watchful",
        "Mana cost: {2}{W}{U}\nType: Legendary Creature — Spirit Warrior\nPower/Toughness: 2/3\nFlying, vigilance\nThe first card you foretell each turn costs {0} to foretell.\nWhenever one or more cards are put into exile from your hand or a spell or ability you control exiles one or more permanents from the battlefield, create a 1/1 white Spirit creature token with flying.", false)
        .expect("Ranar should compile");
    let statics = definition.abilities.iter().filter_map(|ability| match &ability.kind {
        ironsmith::ability::AbilityKind::Static(ability) => Some(ability),
        _ => None,
    }).collect::<Vec<_>>();
    let foretell = statics.iter().find(|ability| ability.foretell_special_action_modifier().is_some())
        .expect("foretell special-action modifier");
    assert_eq!(foretell.foretell_special_action_modifier(), Some((2, false)),
        "the {{2}} foretell action costs {{0}}, only during the usual timing");
    assert!(foretell.foretell_special_action_first_each_turn(), "only the first foretell each turn is free");
    let trigger = p17_triggered(&definition).into_iter().next().expect("exile batch trigger");
    let debug = format!("{:?}", trigger.trigger);
    assert!(debug.contains("AnyOf") || debug.contains("Hand"), "{debug}");
    assert!(format!("{:?}", trigger.effects).contains("Spirit"), "{:?}", trigger.effects);
}

#[test]
fn rocco_grants_each_player_their_exiled_card_and_watches_plays_from_exile() {
    let definition = compile_to_runtime_definition("Rocco, Street Chef",
        "Mana cost: {R}{G}{W}\nType: Legendary Creature — Elf Druid\nPower/Toughness: 2/4\nAt the beginning of your end step, each player exiles the top card of their library. Until your next end step, each player may play the card they exiled this way.\nWhenever a player plays a land from exile or casts a spell from exile, you put a +1/+1 counter on target creature and create a Food token. (It's an artifact with \"{2}, {T}, Sacrifice this token: You gain 3 life.\")", false)
        .expect("Rocco should compile");
    let triggers = p17_triggered(&definition);
    assert_eq!(triggers.len(), 2);
    let mut grant = None;
    for effect in triggers[0].effects.all_effects() {
        p17_visit(effect, &mut |effect| {
            if let Some(model) = effect.downcast_ref::<ironsmith::effects::GrantPlayTaggedEffect>() {
                grant = Some(model.clone());
            }
        });
    }
    let grant = grant.expect("per-player play permission");
    assert_eq!(grant.duration, ironsmith::effects::GrantPlayTaggedDuration::UntilYourNextEndStep);
    assert_eq!(grant.player, ironsmith::target::PlayerFilter::IteratedPlayer,
        "each player may play only the card they exiled");
    let watcher = format!("{:?}", triggers[1].trigger);
    assert!(watcher.contains("PlayerPlaysLandTrigger") && watcher.contains("SpellCastTrigger")
        && watcher.matches("Exile").count() >= 2, "{watcher}");
    let body = format!("{:?}", triggers[1].effects);
    assert!(body.contains("PutCounters") && body.contains("Food"), "{body}");
}

#[test]
fn runesword_gives_the_pumped_attacker_damage_riders_until_end_of_turn() {
    let definition = compile_to_runtime_definition("Runesword",
        "Mana cost: {6}\nType: Artifact\n{3}, {T}: Target attacking creature gets +2/+0 until end of turn. When that creature leaves the battlefield this turn, sacrifice this artifact. If the creature deals damage to a creature this turn, the creature dealt damage can't be regenerated this turn. If a creature dealt damage by the targeted creature would die this turn, exile that creature instead.", false)
        .expect("Runesword should compile");
    let activated = definition.abilities.iter().find_map(|ability| match &ability.kind {
        ironsmith::ability::AbilityKind::Activated(activated) => Some(activated),
        _ => None,
    }).expect("activated ability");
    let debug = format!("{:?}", activated.effects);
    assert!(debug.contains("ScheduleDelayedTriggerEffect") && debug.contains("SacrificeTargetEffect"),
        "the artifact is sacrificed when the creature leaves this turn: {debug}");
    assert!(debug.contains("ExileWouldDieInstead") && debug.contains("damaged_by: Some(ThisCreature)"),
        "creatures it damaged are exiled instead of dying: {debug}");
    assert!(debug.contains("BeRegenerated") && debug.contains("dealt_damage_by_source_this_turn: Some(ThisCreature)"),
        "creatures it damaged can't be regenerated: {debug}");
    assert_eq!(debug.matches("\"pumped_0\"").count() >= 3, true,
        "the pump, the leave watcher and the riders all name the same target: {debug}");
    assert!(debug.contains("EndOfTurn"), "{debug}");
}

#[test]
fn sasaya_reveals_hand_to_flip_and_essence_repeats_land_mana_per_same_name_land() {
    let front = compile_to_runtime_definition("Sasaya, Orochi Ascendant",
        "Mana cost: {1}{G}{G}\nType: Legendary Creature — Snake Monk\nPower/Toughness: 2/3\nReveal your hand: If you have seven or more land cards in your hand, flip Sasaya.", false)
        .expect("Sasaya front face should compile");
    let activated = front.abilities.iter().find_map(|ability| match &ability.kind {
        ironsmith::ability::AbilityKind::Activated(activated) => Some(activated),
        _ => None,
    }).expect("reveal-hand activated ability");
    let debug = format!("{activated:?}");
    assert!(debug.contains("RevealFromHand") && debug.contains("CardsInHand(You)"),
        "revealing the whole hand is the cost: {debug}");
    assert!(debug.contains("Flip"), "{debug}");
    assert!(debug.contains("GreaterThanOrEqual") && debug.contains("Fixed(7)") && debug.contains("Land"),
        "seven or more land cards in hand gate the flip: {debug}");

    let back = compile_to_runtime_definition("Sasaya's Essence",
        "Type: Legendary Enchantment\nWhenever a land you control is tapped for mana, add an additional one mana of any type that land produced for each other land you control with the same name as it.", false)
        .expect("Sasaya's Essence should compile");
    let trigger = p17_triggered(&back).into_iter().next().expect("mana trigger");
    let mut add = None;
    for effect in trigger.effects.all_effects() {
        p17_visit(effect, &mut |effect| {
            if let Some(model) = effect.downcast_ref::<ironsmith::effects::AddManaOfLandProducedTypesEffect>() {
                add = Some(model.clone());
            }
        });
    }
    let add = add.expect("produced-type mana");
    assert_eq!(add.mana_type_source, ironsmith::effects::ManaTypeSource::TriggeringEventProduced);
    let ironsmith::effect::Value::Count(per) = &add.amount else {
        panic!("one mana per other same-name land: {:?}", add.amount);
    };
    assert!(!per.other, "'other' is relative to the tapped land, not the enchantment: {per:?}");
    assert!(per.tagged_constraints.iter().any(|constraint|
        constraint.relation == ironsmith::target::TaggedOpbjectRelation::IsNotTaggedObject), "{per:?}");
}
