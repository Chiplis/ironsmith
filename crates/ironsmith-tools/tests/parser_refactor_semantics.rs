use ironsmith::{AbilityKind, CardDefinition, CardType, GameState, PlayerId, PowerToughness, Zone};
use ironsmith::ids::CardId;
use ironsmith::effects::EffectContext as ExecutionContext;
use ironsmith::effects::execute_effect;
use ironsmith::game_state::{StackEntry, Target};

fn compile(text: &str, kind: CardType) -> CardDefinition {
    let mut builder = ironsmith_compiler::CardDefinitionBuilder::new(CardId::new(), "Semantic Probe")
        .card_types(vec![kind]);
    if kind == CardType::Creature { builder = builder.power_toughness(PowerToughness::fixed(2, 2)); }
    ironsmith_registry::compile_builder_to_runtime_definition(builder, text.to_owned(), false).unwrap()
}
fn permanent(name: &str, types: Vec<CardType>) -> CardDefinition {
    ironsmith::cards::builders::CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(types).power_toughness(PowerToughness::fixed(2, 2)).build()
}
fn game() -> GameState { GameState::new(vec!["Alice".into(), "Bob".into()], 20) }

#[test]
fn counter_removal_trigger_checks_that_the_card_is_still_exiled() {
    let definition = compile("When the last time counter is removed from this card, if it's exiled, you may cast it without paying its mana cost.", CardType::Creature);
    let AbilityKind::Triggered(ability) = &definition.abilities[0].kind else { panic!("trigger"); };
    assert!(format!("{:?}", ability.intervening_if).contains("zone: Some(Exile)"), "{ability:#?}");
}

#[test]
fn permanent_quoted_grants_remain_static_abilities() {
    for text in ["Tokens you control have \"{T}: Add {G}.\"", "Enchanted creature has vigilance and \"At the beginning of your end step, draw a card.\""] {
        let definition = compile(text, CardType::Enchantment);
        assert!(definition.spell_effect.is_none(), "{definition:#?}");
        assert!(definition.abilities.iter().any(|ability| matches!(ability.kind, AbilityKind::Static(_))));
    }
}

#[test]
fn reciprocal_damage_keeps_each_objects_own_power_and_damage_source() {
    let definition = compile("When this creature enters, if it's on the battlefield, it deals damage equal to its power to target creature an opponent controls and that creature deals damage equal to its power to this creature.", CardType::Creature);
    let AbilityKind::Triggered(ability) = &definition.abilities[0].kind else { panic!("trigger"); };
    let mut game = game();
    let alice = PlayerId::from_index(0);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let victim_definition = ironsmith::cards::builders::CardDefinitionBuilder::new(CardId::new(), "Opponent")
        .card_types(vec![CardType::Creature]).power_toughness(PowerToughness::fixed(3, 6)).build();
    let victim = game.create_object_from_definition(&victim_definition, PlayerId::from_index(1), Zone::Battlefield);
    let provenance = game.provenance_graph_mut().alloc_root_event(ironsmith::events::EventKind::EnterBattlefield);
    let event = ironsmith::triggers::TriggerEvent::new_with_provenance(
        ironsmith::events::EnterBattlefieldEvent::new(source, Zone::Hand), provenance,
    );
    let mut ctx = ExecutionContext::new_default(source, alice).with_triggering_event(event);
    ctx.targets = vec![ironsmith::effects::ResolvedTarget::Object(victim)];
    let mut emitted = Vec::new();
    for effect in &ability.effects { emitted.extend(execute_effect(&mut game, effect, &mut ctx).unwrap().events); }
    assert_eq!(game.damage_on(victim), 2);
    assert_eq!(game.damage_on(source), 3);
    // Direct effect execution leaves event dispatch to its caller. Record
    // those actual emitted events as the stack resolver normally would.
    for event in emitted {
        let snapshot = event.downcast::<ironsmith::events::DamageEvent>()
            .and_then(|damage| game.object(damage.source))
            .map(|object| ironsmith::snapshot::ObjectSnapshot::from_object(object, &game));
        game.turn_store.turn_history.record_event(&event, snapshot, None);
    }
    let damage_total = ironsmith::effect::Value::TurnHistoryCount(ironsmith::effect::TurnHistoryCount::DamageDealtBySource);
    assert_eq!(ironsmith::effects::helpers::resolve_value(&game, &damage_total, &ctx).unwrap(), 2);
    let other_ctx = ExecutionContext::new_default(victim, PlayerId::from_index(1));
    assert_eq!(ironsmith::effects::helpers::resolve_value(&game, &damage_total, &other_ctx).unwrap(), 3);
    let condition = compile("If this creature has dealt 3 or more damage this turn, draw a card.", CardType::Sorcery);
    assert!(format!("{condition:#?}").contains("DamageDealtBySource"));
}

#[test]
fn revealed_type_phases_move_overlapping_types_once_and_enchantments_last() {
    let definition = compile("Each player shuffles all permanents they own into their library, then reveals that many cards from the top of their library. Each player puts all artifact, creature, and land cards revealed this way onto the battlefield, then does the same for enchantment cards, then puts all cards revealed this way that weren't put onto the battlefield on the bottom of their library.", CardType::Sorcery);
    let mut game = game();
    let alice = PlayerId::from_index(0);
    let enchantment = game.create_object_from_definition(&permanent("Last", vec![CardType::Enchantment]), alice, Zone::Battlefield);
    let last_stable = game.object(enchantment).unwrap().stable_id;
    let hybrid = game.create_object_from_definition(&permanent("Hybrid", vec![CardType::Enchantment, CardType::Creature]), alice, Zone::Battlefield);
    let hybrid_stable = game.object(hybrid).unwrap().stable_id;
    game.create_object_from_definition(&permanent("Creature", vec![CardType::Creature]), alice, Zone::Battlefield);
    let source = game.new_object_id();
    let mut ctx = ExecutionContext::new_default(source, alice);
    for effect in definition.spell_effect.as_ref().unwrap() { execute_effect(&mut game, effect, &mut ctx).unwrap(); }
    assert_eq!(game.battlefield.len(), 3);
    assert_eq!(game.object(*game.battlefield.last().unwrap()).unwrap().stable_id, last_stable);
    assert_eq!(game.battlefield.iter().filter(|id| game.object(**id).unwrap().stable_id == hybrid_stable).count(), 1);
    assert!(game.player(alice).unwrap().library.is_empty());
}

#[test]
fn intervening_sacrifice_does_not_replace_the_exiled_collection() {
    let definition = compile("Each player exiles all creature cards from their graveyard, then sacrifices all creatures they control, then puts all cards they exiled this way onto the battlefield.", CardType::Sorcery);
    let mut game = game();
    let alice = PlayerId::from_index(0);
    let mut tracked = Vec::new();
    for player in [alice, PlayerId::from_index(1)] {
        for (name, zone) in [("Returns", Zone::Graveyard), ("Sacrificed", Zone::Battlefield)] {
            let id = game.create_object_from_definition(&permanent(name, vec![CardType::Creature]), player, zone);
            tracked.push((game.object(id).unwrap().stable_id, player, zone));
        }
    }
    let source = game.new_object_id();
    let mut ctx = ExecutionContext::new_default(source, alice);
    for effect in definition.spell_effect.as_ref().unwrap() { execute_effect(&mut game, effect, &mut ctx).unwrap(); }
    for (stable, owner, initial) in tracked {
        let object = game.object(game.find_object_by_stable_id(stable).unwrap()).unwrap();
        assert_eq!(object.zone, if initial == Zone::Graveyard { Zone::Battlefield } else { Zone::Graveyard });
        assert_eq!(object.owner, owner);
    }
}

#[test]
fn return_waits_for_source_to_leave_and_keeps_the_exiled_card_reference() {
    let definition = compile("{T}: Exile target creature. Return that card to the battlefield under its owner's control when this artifact leaves the battlefield.", CardType::Artifact);
    let AbilityKind::Activated(ability) = &definition.abilities[0].kind else { panic!("activation"); };
    let mut game = game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let victim = game.create_object_from_definition(&permanent("Returns", vec![CardType::Creature]), bob, Zone::Battlefield);
    let stable = game.object(victim).unwrap().stable_id;
    game.push_to_stack(StackEntry::ability(source, alice, ability.effects.clone()).with_targets(vec![Target::Object(victim)]));
    ironsmith::resolve_stack_entry(&mut game).unwrap();
    assert_eq!(game.object(game.find_object_by_stable_id(stable).unwrap()).unwrap().zone, Zone::Exile);
    assert_eq!(game.effect_store.delayed_triggers.len(), 1);
    game.take_pending_trigger_events();
    game.move_object_by_effect(source, Zone::Graveyard).unwrap();
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    for event in game.take_pending_trigger_events() {
        queue.entries.extend(ironsmith::triggers::check_delayed_triggers(&mut game, &event));
    }
    assert_eq!(queue.entries.len(), 1);
    ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
    ironsmith::resolve_stack_entry(&mut game).unwrap();
    let object = game.object(game.find_object_by_stable_id(stable).unwrap()).unwrap();
    assert_eq!(object.zone, Zone::Battlefield);
    assert_eq!(game.controller_of(object), bob);
}

#[test]
fn counted_creature_collection_supplies_both_draw_and_later_grant() {
    let definition = compile("Draw a card for each creature you control with a +1/+1 counter on it. Those creatures gain indestructible until end of turn.", CardType::Sorcery);
    let mut game = game();
    let alice = PlayerId::from_index(0);
    let mut counted = Vec::new();
    for _ in 0..2 {
        let id = game.create_object_from_definition(&permanent("Counted", vec![CardType::Creature]), alice, Zone::Battlefield);
        game.add_counters(id, ironsmith::object::CounterType::PlusOnePlusOne, 1);
        counted.push(id);
        game.create_object_from_definition(&permanent("Drawn", vec![CardType::Creature]), alice, Zone::Library);
    }
    let other = game.create_object_from_definition(&permanent("Excluded", vec![CardType::Creature]), alice, Zone::Battlefield);
    let source = game.new_object_id();
    let mut ctx = ExecutionContext::new_default(source, alice);
    for effect in definition.spell_effect.as_ref().unwrap() { execute_effect(&mut game, effect, &mut ctx).unwrap(); }
    assert_eq!(game.player(alice).unwrap().hand.len(), 2);
    for id in counted { assert!(game.current_has_static_ability_id(id, ironsmith::static_abilities::StaticAbilityId::Indestructible)); }
    assert!(!game.current_has_static_ability_id(other, ironsmith::static_abilities::StaticAbilityId::Indestructible));
}

#[test]
fn consulted_equipment_enters_attached_to_the_existing_source() {
    let definition = compile("{T}: Reveal cards from the top of your library until you reveal an Equipment card. Put that card onto the battlefield attached to this creature, then put the rest on the bottom of your library in a random order.", CardType::Creature);
    let AbilityKind::Activated(ability) = &definition.abilities[0].kind else { panic!("activation"); };
    let mut game = game();
    let alice = PlayerId::from_index(0);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let equipment = ironsmith::cards::builders::CardDefinitionBuilder::new(CardId::new(), "Equipment")
        .card_types(vec![CardType::Artifact]).subtypes(vec![ironsmith::types::Subtype::Equipment]).build();
    let equipment_id = game.create_object_from_definition(&equipment, alice, Zone::Library);
    let stable = game.object(equipment_id).unwrap().stable_id;
    game.push_to_stack(StackEntry::ability(source, alice, ability.effects.clone()));
    ironsmith::resolve_stack_entry(&mut game).unwrap();
    let object = game.object(game.find_object_by_stable_id(stable).unwrap()).unwrap();
    assert_eq!(object.zone, Zone::Battlefield);
    assert_eq!(object.attached_to, Some(ironsmith::object::AttachmentTarget::Object(source)));
}

#[test]
fn counted_consult_moves_only_the_matching_cards_and_keeps_the_remainder() {
    let definition = compile("Reveal cards from the top of your library until you reveal X creature cards. Put all creature cards revealed this way into your graveyard, then put the rest on the bottom of your library in a random order.", CardType::Sorcery);
    let mut game = game();
    let alice = PlayerId::from_index(0);
    let creature = game.create_object_from_definition(&permanent("Match", vec![CardType::Creature]), alice, Zone::Library);
    let creature_stable = game.object(creature).unwrap().stable_id;
    let other = game.create_object_from_definition(&permanent("Rest", vec![CardType::Instant]), alice, Zone::Library);
    let other_stable = game.object(other).unwrap().stable_id;
    let source = game.new_object_id();
    let mut ctx = ExecutionContext::new_default(source, alice);
    ctx.x_value = Some(1);
    for effect in definition.spell_effect.as_ref().unwrap() { execute_effect(&mut game, effect, &mut ctx).unwrap(); }
    assert_eq!(game.object(game.find_object_by_stable_id(creature_stable).unwrap()).unwrap().zone, Zone::Graveyard);
    assert_eq!(game.object(game.find_object_by_stable_id(other_stable).unwrap()).unwrap().zone, Zone::Library);
}

#[test]
fn opponent_selects_the_returned_card_from_the_controllers_graveyard() {
    struct ChooseLast { player: Option<PlayerId> }
    impl ironsmith::decision::DecisionMaker for ChooseLast {
        fn decide_objects(&mut self, _game: &GameState, ctx: &ironsmith::decisions::context::SelectObjectsContext) -> Vec<ironsmith::ids::ObjectId> {
            self.player = Some(ctx.player);
            ctx.candidates.iter().filter(|candidate| candidate.legal).last().map(|candidate| vec![candidate.id]).unwrap_or_default()
        }
    }
    let definition = compile("Return a creature card of an opponent's choice from your graveyard to your hand.", CardType::Sorcery);
    let mut game = game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let first = game.create_object_from_definition(&permanent("First", vec![CardType::Creature]), alice, Zone::Graveyard);
    let last = game.create_object_from_definition(&permanent("Last", vec![CardType::Creature]), alice, Zone::Graveyard);
    let last_stable = game.object(last).unwrap().stable_id;
    let source = game.new_object_id();
    let mut chooser = ChooseLast { player: None };
    let mut ctx = ExecutionContext::new_default(source, alice).with_decision_maker(&mut chooser);
    for effect in definition.spell_effect.as_ref().unwrap() { execute_effect(&mut game, effect, &mut ctx).unwrap(); }
    drop(ctx);
    assert_eq!(chooser.player, Some(bob));
    assert_eq!(game.object(first).unwrap().zone, Zone::Graveyard);
    assert_eq!(game.object(game.find_object_by_stable_id(last_stable).unwrap()).unwrap().zone, Zone::Hand);
}

#[test]
fn opponent_choice_of_a_target_happens_during_target_selection() {
    let definition = compile("Return target creature card of an opponent's choice from your graveyard to your hand.", CardType::Sorcery);
    let game = game();
    let requirements = ironsmith::game_loop::extract_target_requirements_from_program_with_modes(
        &game, definition.spell_effect.as_ref().unwrap(), PlayerId::from_index(0), None, None);
    assert_eq!(requirements.len(), 1);
    assert_eq!(requirements[0].chooser, Some(ironsmith::target::PlayerFilter::Opponent));
    assert!(requirements[0].spec.is_target());
}

#[test]
fn named_animation_preserves_every_characteristic_change() {
    let definition = compile("Target nontoken creature becomes a 6/6 legendary Horror creature named Omen and loses all abilities.", CardType::Sorcery);
    let mut game = game();
    let alice = PlayerId::from_index(0);
    let target = game.create_object_from_definition(&compile("Flying", CardType::Creature), alice, Zone::Battlefield);
    let mut context = ExecutionContext::new_default(game.new_object_id(), alice);
    context.targets = vec![ironsmith::effects::ResolvedTarget::Object(target)];
    for effect in definition.spell_effect.as_ref().unwrap() { execute_effect(&mut game, effect, &mut context).unwrap(); }
    let characteristics = game.calculated_characteristics(target).unwrap();
    assert!(characteristics.name.eq_ignore_ascii_case("Omen"));
    assert_eq!((characteristics.power, characteristics.toughness), (Some(6), Some(6)));
    assert!(characteristics.supertypes.contains(&ironsmith::types::Supertype::Legendary));
    assert!(characteristics.subtypes.contains(&ironsmith::types::Subtype::Horror));
    assert!(characteristics.abilities.is_empty(), "{characteristics:#?}");
    assert!(characteristics.static_abilities.is_empty(), "{characteristics:#?}");
}

#[test]
fn counters_on_each_of_them_use_the_preceding_population() {
    let definition = compile("Artifact creatures you control gain deathtouch until end of turn. Put a +1/+1 counter on each of them.", CardType::Sorcery);
    let mut game = game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let selected = game.create_object_from_definition(&permanent("Selected", vec![CardType::Artifact, CardType::Creature]), alice, Zone::Battlefield);
    let excluded = [
        game.create_object_from_definition(&permanent("Nonartifact", vec![CardType::Creature]), alice, Zone::Battlefield),
        game.create_object_from_definition(&permanent("Noncreature", vec![CardType::Artifact]), alice, Zone::Battlefield),
        game.create_object_from_definition(&permanent("Opponent", vec![CardType::Artifact, CardType::Creature]), bob, Zone::Battlefield),
    ];
    let source = game.new_object_id();
    let mut ctx = ExecutionContext::new_default(source, alice);
    for effect in definition.spell_effect.as_ref().unwrap() { execute_effect(&mut game, effect, &mut ctx).unwrap(); }
    assert_eq!(game.calculated_characteristics(selected).unwrap().power, Some(3));
    for id in excluded { assert_eq!(game.calculated_characteristics(id).unwrap().power, Some(2)); }
}

#[test]
fn entry_counter_does_not_swallow_the_zone_move() {
    let definition = compile("Put a creature card from your hand onto the battlefield under your control with a finality counter on it. It gains haste.", CardType::Sorcery);
    let mut game = game();
    let alice = PlayerId::from_index(0);
    let creature = game.create_object_from_definition(&permanent("Enters", vec![CardType::Creature]), alice, Zone::Hand);
    let stable = game.object(creature).unwrap().stable_id;
    let source = game.new_object_id();
    let mut ctx = ExecutionContext::new_default(source, alice);
    for effect in definition.spell_effect.as_ref().unwrap() { execute_effect(&mut game, effect, &mut ctx).unwrap(); }
    let entered = game.find_object_by_stable_id(stable).unwrap();
    assert_eq!(game.object(entered).unwrap().zone, Zone::Battlefield);
    assert!(game.current_has_static_ability_id(entered, ironsmith::static_abilities::StaticAbilityId::Haste));
    assert_eq!(game.object(entered).unwrap().counters.get(&ironsmith::object::CounterType::Finality), Some(&1));
}

#[test]
fn conditional_copy_choice_excludes_the_original_targets_controller() {
    let definition = compile("Whenever you cast an instant or sorcery spell that targets only a single nonland permanent an opponent controls, if another opponent controls one or more nonland permanents that spell could target, choose one of those permanents. Copy that spell. The copy targets the chosen permanent.", CardType::Creature);
    let AbilityKind::Triggered(trigger) = &definition.abilities[0].kind else { panic!("trigger"); };
    let choice = trigger.effects.iter().find_map(|effect| effect.downcast_ref::<ironsmith::effects::ChooseObjectsEffect>()).expect("object choice");
    let mut filter = choice.filter.clone();
    let targetability = filter.could_be_targeted_by.as_mut().expect("the original spell's targeting rules");
    assert!(targetability.exclude_current_target_controllers);
    assert!(matches!(&targetability.stack_object, ironsmith::filter::ObjectRef::Tagged(tag) if tag.as_str() == "triggering"));
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Carol".into()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let carol = PlayerId::from_index(2);
    let original = game.create_object_from_definition(&permanent("Original", vec![CardType::Creature]), bob, Zone::Battlefield);
    game.create_object_from_definition(&permanent("Same controller", vec![CardType::Creature]), bob, Zone::Battlefield);
    let legal = game.create_object_from_definition(&permanent("Other opponent", vec![CardType::Creature]), carol, Zone::Battlefield);
    game.create_object_from_definition(&permanent("Wrong type", vec![CardType::Artifact]), carol, Zone::Battlefield);
    let untargetable = compile("Hexproof", CardType::Creature);
    game.create_object_from_definition(&untargetable, carol, Zone::Battlefield);
    let spell = compile("Destroy target creature an opponent controls.", CardType::Instant);
    let spell_id = game.create_object_from_definition(&spell, alice, Zone::Stack);
    game.push_to_stack(StackEntry::new(spell_id, alice).with_targets(vec![Target::Object(original)]));
    targetability.stack_object = ironsmith::filter::ObjectRef::Specific(spell_id);
    let spec = ironsmith::target::ChooseSpec::target(ironsmith::target::ChooseSpec::Object(filter));
    let candidates = ironsmith::targeting::compute_legal_targets(&game, &spec, alice, None);
    assert_eq!(candidates, vec![Target::Object(legal)]);
}

#[test]
fn each_players_kept_permanent_survives_the_shared_complement_return() {
    let definition = compile("Each player chooses a nonland permanent they control. Return all nonland permanents not chosen this way to their owners' hands.", CardType::Sorcery);
    let mut game = game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for player in [alice, bob] {
        for name in ["First", "Second"] {
            game.create_object_from_definition(&permanent(name, vec![CardType::Creature]), player, Zone::Battlefield);
        }
        game.create_object_from_definition(&permanent("Land", vec![CardType::Land]), player, Zone::Battlefield);
    }
    let source = game.new_object_id();
    let mut ctx = ExecutionContext::new_default(source, alice);
    for effect in definition.spell_effect.as_ref().unwrap() { execute_effect(&mut game, effect, &mut ctx).unwrap(); }
    for player in [alice, bob] {
        let remaining = game.battlefield.iter().filter_map(|id| game.object(*id)).filter(|object| game.controller_of(object) == player).collect::<Vec<_>>();
        assert_eq!(remaining.len(), 2, "one land and the chosen nonland must remain for each player");
        assert_eq!(game.player(player).unwrap().hand.len(), 1);
    }
}

#[test]
fn bare_attached_subtype_setting_is_continuous_and_token_entry_sentence_is_retained() {
    let definition = compile("Enchant creature\nEnchanted creature is a Demon Spirit.", CardType::Enchantment);
    // Aura casting still attaches the Aura; the subtype change itself is static.
    assert!(!format!("{:?}", definition.spell_effect).contains("SetSubtypes"));
    assert!(!format!("{:?}", definition.spell_effect).contains("ApplyContinuousEffect"));
    assert!(format!("{definition:#?}").contains("SetCreatureSubtypes"));
    let token = compile("Create a token that's a copy of another target attacking creature. The token enters tapped and attacking.", CardType::Sorcery);
    let debug = format!("{token:#?}");
    assert!(debug.contains("entry_tapped_attacking_followup: true"), "{debug}");
    assert!(debug.contains("enters_tapped: true") && debug.contains("enters_attacking: true"));
}

#[test]
fn linked_exiled_card_supplies_token_owner_and_size_in_a_later_ability() {
    let definition = compile("When this creature leaves the battlefield, the exiled card's owner creates an X/X white Spirit creature token, where X is the mana value of the exiled card.", CardType::Creature);
    let AbilityKind::Triggered(ability) = &definition.abilities[0].kind else { panic!("trigger"); };
    let mut game = game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let mut exiled_definition = permanent("Linked", vec![CardType::Sorcery]);
    exiled_definition.card.mana_cost = Some(ironsmith::mana::ManaCost::from_pips(vec![vec![ironsmith::mana::ManaSymbol::Generic(5)]]));
    let exiled = game.create_object_from_definition(&exiled_definition, bob, Zone::Exile);
    game.add_exiled_with_source_link(source, exiled);
    let snapshot = ironsmith::snapshot::ObjectSnapshot::from_object(game.object(source).unwrap(), &game);
    game.move_object_by_effect(source, Zone::Graveyard).unwrap();
    let provenance = game.provenance_graph_mut().alloc_root_event(ironsmith::events::EventKind::ZoneChange);
    let event = ironsmith::triggers::TriggerEvent::new_with_provenance(
        ironsmith::events::zones::ZoneChangeEvent::with_cause(source, Zone::Battlefield, Zone::Graveyard,
            ironsmith::events::cause::EventCause::effect(), Some(snapshot.clone())), provenance);
    let mut ctx = ExecutionContext::new_default(source, alice).with_source_snapshot(snapshot).with_triggering_event(event);
    // Stack resolution seeds persistent linked-exile snapshots before executing
    // an ability; reproduce that setup when exercising the effects directly.
    let linked = game.get_exiled_with_source_links(source).iter().map(|id|
        ironsmith::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(game.object(*id).unwrap(), &game)
    ).collect::<Vec<_>>();
    assert_eq!(linked.len(), 1, "the source's old identity retains the linked card after leaving");
    ctx.set_tagged_objects(ironsmith::tag::SOURCE_EXILED_TAG, linked);
    for effect in &ability.effects { execute_effect(&mut game, effect, &mut ctx).unwrap(); }
    let token = *game.battlefield.iter().find(|id| **id != source).expect("linked owner receives a token");
    assert_eq!(game.controller_of_id(token), Some(bob));
    assert_eq!(game.calculated_characteristics(token).unwrap().power, Some(5));
    assert_eq!(game.calculated_characteristics(token).unwrap().toughness, Some(5));
}

#[test]
fn activation_mana_source_condition_uses_the_current_payment_snapshots() {
    let definition = compile("If mana from a Treasure was spent to activate this ability, draw a card.", CardType::Sorcery);
    for (subtype, expected_draws) in [(ironsmith::Subtype::Treasure, 1), (ironsmith::Subtype::Clue, 0)] {
        let mut game = game();
        let alice = PlayerId::from_index(0);
        let source = game.create_object_from_definition(&permanent("Source", vec![CardType::Creature]), alice, Zone::Battlefield);
        let mana_source_definition = ironsmith::cards::builders::CardDefinitionBuilder::new(CardId::new(), "Payment source")
            .card_types(vec![CardType::Artifact]).subtypes(vec![subtype]).build();
        let mana_source = game.create_object_from_definition(&mana_source_definition, alice, Zone::Battlefield);
        let payment = ironsmith::snapshot::ObjectSnapshot::from_object(game.object(mana_source).unwrap(), &game);
        game.move_object_by_effect(mana_source, Zone::Graveyard).unwrap();
        game.create_object_from_definition(&permanent("Drawn", vec![CardType::Creature]), alice, Zone::Library);
        let mut ctx = ExecutionContext::new_default(source, alice);
        ctx.set_tagged_objects(ironsmith::tag::MANA_SOURCES_SPENT_TO_CAST_TAG, vec![payment]);
        for effect in definition.spell_effect.as_ref().unwrap() { execute_effect(&mut game, effect, &mut ctx).unwrap(); }
        assert_eq!(game.player(alice).unwrap().hand.len(), expected_draws);
    }
}

#[test]
fn combat_attack_count_keeps_last_known_attackers_and_resets_each_combat() {
    let mut game = game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let source = game.create_object_from_definition(&permanent("Attacker", vec![CardType::Creature]), alice, Zone::Battlefield);
    let snapshot = ironsmith::snapshot::ObjectSnapshot::from_object(game.object(source).unwrap(), &game);
    let begin = |game: &mut GameState| {
        let provenance = game.provenance_graph_mut().alloc_root_event(ironsmith::events::EventKind::BeginningOfCombat);
        let event = ironsmith::triggers::TriggerEvent::new_with_provenance(ironsmith::events::BeginningOfCombatEvent::new(alice), provenance);
        game.turn_store.turn_history.record_event(&event, None, None);
    };
    begin(&mut game);
    for _ in 0..2 {
        let provenance = game.provenance_graph_mut().alloc_root_event(ironsmith::events::EventKind::CreatureAttacked);
        let event = ironsmith::triggers::TriggerEvent::new_with_provenance(
            ironsmith::events::CreatureAttackedEvent::new(source, ironsmith::triggers::event::AttackEventTarget::Player(bob)), provenance);
        game.turn_store.turn_history.record_event(&event, Some(snapshot.clone()), None);
    }
    let value = ironsmith::effect::Value::TurnHistoryCount(ironsmith::effect::TurnHistoryCount::PlayersAttackedThisCombat(ironsmith::PlayerFilter::You));
    let ctx = ExecutionContext::new_default(source, alice);
    assert_eq!(ironsmith::effects::helpers::resolve_value(&game, &value, &ctx).unwrap(), 1);
    game.move_object_by_effect(source, Zone::Graveyard).unwrap();
    assert_eq!(ironsmith::effects::helpers::resolve_value(&game, &value, &ctx).unwrap(), 1);
    begin(&mut game);
    assert_eq!(ironsmith::effects::helpers::resolve_value(&game, &value, &ctx).unwrap(), 0);
}

#[test]
fn sacrifice_combat_history_requires_one_matching_damage_event() {
    let definition = compile("Each opponent sacrifices a creature of their choice that dealt combat damage to you this turn.", CardType::Sorcery);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Carol".into()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let carol = PlayerId::from_index(2);
    let spared = game.create_object_from_definition(&permanent("Spared", vec![CardType::Creature]), bob, Zone::Battlefield);
    let sacrificed = game.create_object_from_definition(&permanent("Sacrificed", vec![CardType::Creature]), bob, Zone::Battlefield);
    for (source, target, combat) in [(spared, alice, false), (spared, carol, true), (sacrificed, alice, true)] {
        let snapshot = ironsmith::snapshot::ObjectSnapshot::from_object(game.object(source).unwrap(), &game);
        let provenance = game.provenance_graph_mut().alloc_root_event(ironsmith::events::EventKind::Damage);
        let event = ironsmith::triggers::TriggerEvent::new_with_provenance(
            ironsmith::events::DamageEvent::with_cause(source, ironsmith::events::DamageTarget::Player(target), 1, combat, ironsmith::events::cause::EventCause::effect()), provenance);
        game.turn_store.turn_history.record_event(&event, Some(snapshot), None);
    }
    let mut ctx = ExecutionContext::new_default(game.new_object_id(), alice);
    for effect in definition.spell_effect.as_ref().unwrap() { execute_effect(&mut game, effect, &mut ctx).unwrap(); }
    assert!(game.battlefield.contains(&spared), "combat damage to a different player must not combine with noncombat damage to you");
    assert!(!game.battlefield.contains(&sacrificed));
}

#[test]
fn coordinated_destruction_preserves_the_second_targets_opponent_chooser() {
    let definition = compile("Destroy target nonbasic land you don't control and target nonbasic land of an opponent's choice you don't control. This spell deals 7 damage to target creature you don't control and 7 damage to target creature of an opponent's choice you don't control.", CardType::Sorcery);
    let mut game = game();
    let bob = PlayerId::from_index(1);
    for kind in [CardType::Land, CardType::Creature] {
        game.create_object_from_definition(&permanent("First", vec![kind]), bob, Zone::Battlefield);
        game.create_object_from_definition(&permanent("Second", vec![kind]), bob, Zone::Battlefield);
    }
    let requirements = ironsmith::game_loop::extract_target_requirements_from_program_with_modes(
        &game, definition.spell_effect.as_ref().unwrap(), PlayerId::from_index(0), None, None);
    assert_eq!(requirements.len(), 4, "{definition:#?}");
    assert_eq!(requirements[0].chooser, None);
    assert_eq!(requirements[1].chooser, Some(ironsmith::target::PlayerFilter::Opponent));
    assert_eq!(requirements[2].chooser, None);
    assert_eq!(requirements[3].chooser, Some(ironsmith::target::PlayerFilter::Opponent));
}

#[test]
fn ordered_graveyard_choice_only_moves_the_top_pool_and_its_complement() {
    let definition = compile("Target opponent chooses one of the top two cards of your graveyard. Exile that card and put the other one into your hand.", CardType::Sorcery);
    let mut game = game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let oldest = game.create_object_from_definition(&permanent("Oldest", vec![CardType::Creature]), alice, Zone::Graveyard);
    game.create_object_from_definition(&permanent("Middle", vec![CardType::Creature]), alice, Zone::Graveyard);
    game.create_object_from_definition(&permanent("Newest", vec![CardType::Creature]), alice, Zone::Graveyard);
    let source = game.new_object_id();
    let mut ctx = ExecutionContext::new_default(source, alice);
    ctx.targets = vec![ironsmith::effects::ResolvedTarget::Player(bob)];
    for effect in definition.spell_effect.as_ref().unwrap() { execute_effect(&mut game, effect, &mut ctx).unwrap(); }
    assert_eq!(game.object(oldest).unwrap().zone, Zone::Graveyard);
    assert_eq!(game.player(alice).unwrap().graveyard.len(), 1);
    assert_eq!(game.player(alice).unwrap().hand.len(), 1);
    assert_eq!(game.exile.len(), 1);
}

#[test]
fn delayed_death_watches_the_created_token_not_an_unrelated_token() {
    let definition = compile("{T}: Create a 1/1 blue Spirit creature token. Return this artifact to the battlefield under its owner's control when that token dies.", CardType::Artifact);
    let AbilityKind::Activated(ability) = &definition.abilities[0].kind else { panic!("activation"); };
    let mut game = game();
    let alice = PlayerId::from_index(0);
    let source = game.create_object_from_definition(&definition, alice, Zone::Exile);
    let stable = game.object(source).unwrap().stable_id;
    game.push_to_stack(StackEntry::ability(source, alice, ability.effects.clone()));
    ironsmith::resolve_stack_entry(&mut game).unwrap();
    let watched = game.battlefield[0];
    assert_eq!(game.effect_store.delayed_triggers.len(), 1);
    let extra = compile("Create a 1/1 blue Spirit creature token.", CardType::Sorcery);
    let mut ctx = ExecutionContext::new_default(source, alice);
    for effect in extra.spell_effect.as_ref().unwrap() { execute_effect(&mut game, effect, &mut ctx).unwrap(); }
    let unrelated = *game.battlefield.last().unwrap();
    assert_ne!(unrelated, watched);
    game.take_pending_trigger_events();
    game.move_object_by_effect(unrelated, Zone::Graveyard).unwrap();
    for event in game.take_pending_trigger_events() {
        assert!(ironsmith::triggers::check_delayed_triggers(&mut game, &event).is_empty());
    }
    game.move_object_by_effect(watched, Zone::Graveyard).unwrap();
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    for event in game.take_pending_trigger_events() {
        queue.entries.extend(ironsmith::triggers::check_delayed_triggers(&mut game, &event));
    }
    assert_eq!(queue.entries.len(), 1);
    ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
    ironsmith::resolve_stack_entry(&mut game).unwrap();
    assert_eq!(game.object(game.find_object_by_stable_id(stable).unwrap()).unwrap().zone, Zone::Battlefield);
}

#[test]
fn delayed_monarch_attack_requires_the_watched_creature_and_actual_monarch_defender() {
    use ironsmith::combat_state::{AttackTarget, AttackerInfo, CombatState};
    use ironsmith::triggers::AttackEventTarget;
    let definition = compile("{T}: Put a +1/+1 counter on target creature. Whenever that creature attacks the monarch this turn, draw a card.", CardType::Artifact);
    let AbilityKind::Activated(ability) = &definition.abilities[0].kind else { panic!("activation"); };
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let carol = PlayerId::from_index(2);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Carol".into()], 20);
    let watched = game.create_object_from_definition(&permanent("Watched", vec![CardType::Creature]), alice, Zone::Battlefield);
    let other = game.create_object_from_definition(&permanent("Other", vec![CardType::Creature]), alice, Zone::Battlefield);
    let planeswalker = game.create_object_from_definition(&permanent("Walker", vec![CardType::Planeswalker]), bob, Zone::Battlefield);
    let source = game.new_object_id();
    let mut ctx = ExecutionContext::new_default(source, alice);
    ctx.targets = vec![ironsmith::effects::ResolvedTarget::Object(watched)];
    for effect in &ability.effects { execute_effect(&mut game, effect, &mut ctx).unwrap(); }
    assert_eq!(game.effect_store.delayed_triggers.len(), 1);
    game.monarch = Some(bob);
    game.take_pending_trigger_events();
    for (attacker, target, event_target, expected) in [
        (other, AttackTarget::Player(bob), AttackEventTarget::Player(bob), 0),
        (watched, AttackTarget::Player(carol), AttackEventTarget::Player(carol), 0),
        (watched, AttackTarget::Planeswalker(planeswalker), AttackEventTarget::Planeswalker(planeswalker), 0),
        (watched, AttackTarget::Player(bob), AttackEventTarget::Player(bob), 1),
    ] {
        let mut combat = CombatState::default();
        combat.attackers.push(AttackerInfo { creature: attacker, target });
        game.combat = Some(combat);
        let provenance = game.provenance_graph_mut().alloc_root_event(ironsmith::events::EventKind::CreatureAttacked);
        let event = ironsmith::triggers::TriggerEvent::new_with_provenance(
            ironsmith::events::combat::CreatureAttackedEvent::new(attacker, event_target), provenance);
        assert_eq!(ironsmith::triggers::check_delayed_triggers(&mut game, &event).len(), expected,
            "attacker={attacker:?}, watched={watched:?}, event={event:?}");
    }
}

#[test]
fn revealing_a_hand_does_not_replace_the_prior_objects_name_antecedent() {
    let definition = compile("Exile target creature. If that creature was an Elf, its controller reveals their hand and exiles all cards from it with the same name as that creature.", CardType::Sorcery);
    for qualifies in [false, true] {
        let mut game = game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let target_definition = ironsmith::cards::builders::CardDefinitionBuilder::new(CardId::new(), "Namesake")
            .card_types(vec![CardType::Creature])
            .subtypes(if qualifies { vec![ironsmith::Subtype::Elf] } else { vec![] })
            .power_toughness(PowerToughness::fixed(2, 2)).build();
        let target = game.create_object_from_definition(&target_definition, bob, Zone::Battlefield);
        let matching = game.create_object_from_definition(&target_definition, bob, Zone::Hand);
        let matching_stable = game.object(matching).unwrap().stable_id;
        let distinct_definition = ironsmith::cards::builders::CardDefinitionBuilder::new(CardId::new(), "Distinct")
            .card_types(vec![CardType::Creature]).subtypes(vec![ironsmith::Subtype::Elf])
            .power_toughness(PowerToughness::fixed(2, 2)).build();
        let distinct = game.create_object_from_definition(&distinct_definition, bob, Zone::Hand);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);
        ctx.targets = vec![ironsmith::effects::ResolvedTarget::Object(target)];
        for effect in definition.spell_effect.as_ref().unwrap() { execute_effect(&mut game, effect, &mut ctx).unwrap(); }
        assert_eq!(game.object(distinct).unwrap().zone, Zone::Hand);
        assert_eq!(game.object(game.find_object_by_stable_id(matching_stable).unwrap()).unwrap().zone,
            if qualifies { Zone::Exile } else { Zone::Hand });
    }
}

#[test]
fn each_opponents_exile_and_life_gain_share_the_same_iteration() {
    let definition = compile("For each opponent, exile up to one target creature that player controls and that player gains life equal to its power.", CardType::Sorcery);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Carol".into()], 20);
    let alice = PlayerId::from_index(0);
    let mut targets = Vec::new();
    let mut stable_ids = Vec::new();
    for (index, power) in [(1, 3), (2, 7)] {
        let creature = ironsmith::cards::builders::CardDefinitionBuilder::new(CardId::new(), "Iteration Probe")
            .card_types(vec![CardType::Creature]).power_toughness(PowerToughness::fixed(power, 8)).build();
        let id = game.create_object_from_definition(&creature, PlayerId::from_index(index), Zone::Battlefield);
        stable_ids.push(game.object(id).unwrap().stable_id);
        targets.push(ironsmith::effects::ResolvedTarget::Object(id));
    }
    let mut ctx = ExecutionContext::new_default(game.new_object_id(), alice);
    ctx.targets = targets;
    for effect in definition.spell_effect.as_ref().unwrap() {
        execute_effect(&mut game, effect, &mut ctx).unwrap();
    }
    assert_eq!(game.player(alice).unwrap().life, 20);
    assert_eq!(game.player(PlayerId::from_index(1)).unwrap().life, 23);
    assert_eq!(game.player(PlayerId::from_index(2)).unwrap().life, 27);
    for stable in stable_ids {
        assert_eq!(game.object(game.find_object_by_stable_id(stable).unwrap()).unwrap().zone, Zone::Exile);
    }
}
