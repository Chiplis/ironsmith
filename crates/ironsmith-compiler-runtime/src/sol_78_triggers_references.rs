//! Source-only regressions. Authored under the explicit no-build restriction;
//! these are pending integration checks, not evidence that the source compiles.
use super::compile_to_runtime_definition;

#[test]
fn source_ready_trigger_reference_cards_compile_in_full() {
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../reports/compilation-completion-20261016/sol-78-source-pass/triggers_references-source-ready.fixture.json"
    )).unwrap();
    assert_eq!(rows.len(), 24, "source-ready cards, including both Arsonist faces");
    for row in rows {
        let name = row["name"].as_str().unwrap();
        let mut text = format!("Mana cost: {}\nType: {}\n", row["mana_cost"].as_str().unwrap(), row["type_line"].as_str().unwrap());
        if let (Some(power), Some(toughness)) = (row["power"].as_str(), row["toughness"].as_str()) {
            text.push_str(&format!("Power/Toughness: {power}/{toughness}\n"));
        }
        text.push_str(row["oracle_text"].as_str().unwrap());
        compile_to_runtime_definition(name, &text, false)
            .unwrap_or_else(|error| panic!("{name}: {error:?}"));
    }
}

#[test]
fn targeting_events_freeze_activated_versus_triggered_stack_kind() {
    use ironsmith::events::BecomesTargetedEvent;
    use ironsmith::game_state::{StackEntry, Target};
    use ironsmith::{ObjectId, PlayerId};
    use ironsmith_core::filter_model::StackObjectKind;
    let source = ObjectId::from_raw(101);
    let target = ObjectId::from_raw(102);
    let player = PlayerId::from_index(0);
    let mut entry = StackEntry::ability(source, player,
        ironsmith::resolution::ResolutionProgram::from_effects(vec![]));
    let activated = BecomesTargetedEvent::from_stack_entry(Target::Object(target), &entry);
    assert_eq!(activated.stack_kind, Some(StackObjectKind::ActivatedAbility));
    entry.triggering_event = Some(ironsmith::triggers::TriggerEvent::new_with_provenance(
        BecomesTargetedEvent::new(target, source, player, false), Default::default()));
    let triggered = BecomesTargetedEvent::from_stack_entry(Target::Object(target), &entry);
    assert_eq!(triggered.stack_kind, Some(StackObjectKind::TriggeredAbility));
    assert_eq!(activated.stack_kind, Some(StackObjectKind::ActivatedAbility), "later changes cannot rewrite the frozen event kind");
}

#[test]
fn damage_redirect_keeps_original_target_and_uses_its_controller_as_chooser() {
    use ironsmith::card::{CardBuilder, PowerToughness};
    use ironsmith::effects::{EffectContext, ResolvedTarget, execute_effect};
    use ironsmith::{CardId, PlayerId, Zone};
    struct Choice { accept: bool, players: Vec<PlayerId> }
    impl ironsmith::decision::DecisionMaker for Choice {
        fn decide_boolean(&mut self, _: &ironsmith::GameState, ctx: &ironsmith::decisions::context::BooleanContext) -> bool {
            self.players.push(ctx.player);
            self.accept
        }
        fn decide_objects(&mut self, _: &ironsmith::GameState, ctx: &ironsmith::decisions::context::SelectObjectsContext) -> Vec<ironsmith::ObjectId> {
            self.players.push(ctx.player);
            ctx.candidates.iter().filter(|item| item.legal).take(1).map(|item| item.id).collect()
        }
    }
    let definition = compile_to_runtime_definition("Redirect example",
        "Mana cost: {X}{R}\nType: Instant\nRedirect example deals X damage to target player or planeswalker. That player or that planeswalker's controller may choose a creature they control and have Redirect example deal that damage to it instead.", false).unwrap();
    let a = PlayerId::from_index(0);
    let b = PlayerId::from_index(1);
    let creature = CardBuilder::new(CardId::new(), "Recipient")
        .card_types(vec![ironsmith::types::CardType::Creature])
        .power_toughness(PowerToughness::fixed(5, 5)).build();
    for accept in [false, true] {
        let mut game = ironsmith::GameState::new(vec!["A".into(), "B".into()], 20);
        let theirs = game.create_object_from_card(&creature, b, Zone::Battlefield);
        let source = game.create_object_from_definition(&definition, a, Zone::Stack);
        let mut choice = Choice { accept, players: vec![] };
        let mut ctx = EffectContext::new(source, a, &mut choice)
            .with_x(3).with_targets(vec![ResolvedTarget::Player(b)]);
        for effect in definition.spell_effect.as_ref().unwrap().all_effects() {
            execute_effect(&mut game, effect, &mut ctx).unwrap();
        }
        drop(ctx);
        assert!(!choice.players.is_empty());
        assert!(choice.players.iter().all(|player| *player == b));
        assert_eq!(game.damage_on(theirs), if accept { 3 } else { 0 });
        assert_eq!(game.player(b).unwrap().life, if accept { 20 } else { 17 });
    }
}

#[test]
fn object_targeting_batch_excludes_triggered_abilities_and_player_targets() {
    use ironsmith::card::{CardBuilder, PowerToughness};
    use ironsmith::events::BecomesTargetedEvent;
    use ironsmith::target::{ObjectFilter, PlayerFilter};
    use ironsmith::triggers::{Trigger, TriggerContext, TriggerEvent};
    use ironsmith::{CardId, PlayerId, Zone};
    use ironsmith_core::filter_model::StackObjectKind;
    let a = PlayerId::from_index(0);
    let mut game = ironsmith::GameState::new(vec!["A".into(), "B".into()], 20);
    let card = CardBuilder::new(CardId::new(), "Creature")
        .card_types(vec![ironsmith::types::CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1)).build();
    let source = game.create_object_from_card(&card, a, Zone::Battlefield);
    let target = game.create_object_from_card(&card, a, Zone::Battlefield);
    let trigger = Trigger::player_or_object_becomes_targeted_by_source_controller(
        PlayerFilter::excluding(PlayerFilter::Any, PlayerFilter::Any),
        ObjectFilter::creature().you_control(), PlayerFilter::Any,
        StackObjectKind::ActivatedAbility, true);
    for (kind, expected) in [(StackObjectKind::ActivatedAbility, true), (StackObjectKind::TriggeredAbility, false)] {
        let mut event = BecomesTargetedEvent::new(target, source, a, true);
        event.stack_kind = Some(kind);
        let event = TriggerEvent::new_with_provenance(event, Default::default());
        assert_eq!(trigger.matches(&event, &TriggerContext::for_source(source, a, &game)), expected);
    }
    let mut player_event = BecomesTargetedEvent::new_player(a, source, a, true);
    player_event.stack_kind = Some(StackObjectKind::ActivatedAbility);
    assert!(!trigger.matches(&TriggerEvent::new_with_provenance(player_event, Default::default()),
        &TriggerContext::for_source(source, a, &game)));
}

#[test]
fn upkeep_payment_receipt_counts_actual_matching_mana_across_installments() {
    use ironsmith::card::CardBuilder;
    use ironsmith::effect::{Effect, EventValueSpec, Value};
    use ironsmith::effects::{EffectContext, EffectExecutor};
    use ironsmith::mana::{ManaCost, ManaSymbol};
    use ironsmith::target::{ChooseSpec, PlayerFilter};
    use ironsmith::{CardId, PlayerId, Zone};
    let a = PlayerId::from_index(0);
    let mut game = ironsmith::GameState::new(vec!["A".into(), "B".into()], 20);
    let card = CardBuilder::new(CardId::new(), "Upkeep permanent").build();
    let source = game.create_object_from_card(&card, a, Zone::Battlefield);
    game.object_mut(source).unwrap().add_counters(ironsmith::object::CounterType::Age, 2);
    game.player_mut(a).unwrap().mana_pool.add(ManaSymbol::Black, 2);
    game.player_mut(a).unwrap().mana_pool.add(ManaSymbol::Red, 2);
    let payment = Effect::new(ironsmith::effects::PayManaEffect::new(
        ManaCost::from_symbols(vec![ManaSymbol::Black, ManaSymbol::Red]), ChooseSpec::SourceController));
    let upkeep = ironsmith::effects::CumulativeUpkeepEffect::new(PlayerFilter::You, vec![payment], vec![]);
    let mut ctx = EffectContext::new_default(source, a);
    let outcome = upkeep.execute(&mut game, &mut ctx).unwrap();
    let event = outcome.events.iter().find(|event| event.downcast::<ironsmith::events::KeywordActionEvent>()
        .is_some_and(|event| event.action == ironsmith::events::KeywordActionKind::CumulativeUpkeepPaid))
        .expect("one completed cost receipt").clone();
    let receipt = event.downcast::<ironsmith::events::KeywordActionEvent>().unwrap();
    assert_eq!((receipt.mana_spent.black, receipt.mana_spent.red), (2, 2));
    let ctx = EffectContext::new_default(source, a).with_triggering_event(event);
    let quantity = Value::EventValue(EventValueSpec::ManaSpentOnKeyword {
        colors: ironsmith::color::ColorSet::BLACK.union(ironsmith::color::ColorSet::RED),
    });
    assert_eq!(ironsmith::effects::helpers::resolve_value(&game, &quantity, &ctx).unwrap(), 4);
}

#[test]
fn cumulative_paid_colored_mana_multiplier_compiles() {
    let definition = compile_to_runtime_definition("Upkeep example",
        "Mana cost: {3}{B}\nType: Creature — Zombie\nPower/Toughness: 3/5\nCumulative upkeep {1}\nWhenever this creature's cumulative upkeep is paid, it gets +1/+0 until end of turn for each {B} or {R} spent this way.", false).unwrap();
    let debug = format!("{definition:?}");
    assert!(debug.contains("ManaSpentOnKeyword"), "{debug}");
    assert!(debug.contains("CumulativeUpkeepPaid"), "{debug}");
}

#[test]
fn stack_insert_receipt_names_independent_ability_and_is_queued_on_insertion() {
    use ironsmith::card::CardBuilder;
    use ironsmith::game_state::{StackEntry, Target};
    use ironsmith::{CardId, PlayerId, Zone};
    let a = PlayerId::from_index(0);
    let mut game = ironsmith::GameState::new(vec!["A".into(), "B".into()], 20);
    let card = CardBuilder::new(CardId::new(), "Ability source").build();
    let source = game.create_object_from_card(&card, a, Zone::Battlefield);
    let entry = StackEntry::ability(source, a,
        ironsmith::resolution::ResolutionProgram::from_effects(vec![]))
        .with_targets(vec![Target::Player(a)]);
    game.push_to_stack(entry);
    let stack_id = game.stack.last().unwrap().target_id();
    assert_ne!(source, stack_id);
    let events = game.take_pending_trigger_events();
    let receipt = events.iter().find_map(|event|
        event.downcast::<ironsmith::events::spells::StackObjectAddedEvent>()).unwrap();
    assert_eq!(receipt.stack_id, stack_id);
    assert_eq!(receipt.source, source);
    assert_eq!(receipt.snapshot.as_ref().unwrap().object_id, stack_id);
}

#[test]
fn stack_insertion_single_target_random_reselection_compiles() {
    let definition = compile_to_runtime_definition("Retargeting example",
        "Mana cost: {5}{R}\nType: Enchantment\nWhenever a spell or ability is put onto the stack, if it has a single target, reselect its target at random. (Select from among all legal targets.)", false).unwrap();
    let debug = format!("{definition:?}");
    assert!(debug.contains("StackObjectAdded"), "{debug}");
    assert!(debug.contains("RetargetStackObjectEffect"), "{debug}");
}

#[test]
fn state_trigger_tracks_each_matching_creature_and_suppresses_only_its_pending_instance() {
    use ironsmith::card::{CardBuilder, PowerToughness};
    use ironsmith::{CardId, PlayerId, Zone};
    let a = PlayerId::from_index(0);
    let definition = compile_to_runtime_definition("Counter watcher",
        "Mana cost: {3}{R}\nType: Creature — Dwarf\nPower/Toughness: 1/1\nWhenever a creature has four or more fuse counters on it, remove all fuse counters from it and destroy it. That creature deals 4 damage to its controller.", false).unwrap();
    let mut game = ironsmith::GameState::new(vec!["A".into(), "B".into()], 20);
    let watcher = game.create_object_from_definition(&definition, a, Zone::Battlefield);
    let card = CardBuilder::new(CardId::new(), "Watched creature")
        .card_types(vec![ironsmith::types::CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1)).build();
    let first = game.create_object_from_card(&card, a, Zone::Battlefield);
    let second = game.create_object_from_card(&card, a, Zone::Battlefield);
    for id in [first, second] {
        game.object_mut(id).unwrap().add_counters(ironsmith::object::CounterType::Fuse, 4);
    }
    let (pending, _) = ironsmith::triggers::check_state_triggers(&game, &[]);
    assert_eq!(pending.len(), 2);
    let watched: Vec<_> = pending.iter().map(|entry| entry.triggering_event
        .downcast::<ironsmith::events::StateTriggerEvent>().unwrap().watched_object.unwrap()).collect();
    assert!(watched.contains(&first) && watched.contains(&second));
    assert!(pending.iter().all(|entry| entry.source == watcher));
    let (new, _) = ironsmith::triggers::check_state_triggers(&game, &pending[..1]);
    assert_eq!(new.len(), 1, "one object's pending trigger cannot suppress the other");
    let entry = &pending[0];
    let doomed = entry.triggering_event.downcast::<ironsmith::events::StateTriggerEvent>()
        .unwrap().watched_object.unwrap();
    let mut ctx = ironsmith::effects::EffectContext::new_default(watcher, a)
        .with_triggering_event(entry.triggering_event.clone());
    for effect in entry.ability.effects.all_effects() {
        ironsmith::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
    }
    assert!(!game.battlefield.contains(&doomed));
    assert_eq!(game.player(a).unwrap().life, 16, "the destroyed creature deals damage using its last known source");
}

#[test]
fn craft_cost_trigger_uses_selected_activation_keyword_not_other_source_abilities() {
    use ironsmith::ability::AbilityKind;
    use ironsmith::costs::{CostContext, PaymentReason};
    use ironsmith::decision::SelectFirstDecisionMaker;
    use ironsmith::events::ZoneChangeEvent;
    use ironsmith::triggers::{TriggerContext, TriggerEvent};
    use ironsmith::{PlayerId, Zone};
    let a = PlayerId::from_index(0);
    let definition = compile_to_runtime_definition("Exile watcher",
        "Mana cost: {1}\nType: Artifact Creature — Gnome\nPower/Toughness: 0/3\nWhen this creature is exiled from the battlefield while you're activating a craft ability, you gain 1 life and draw a card.", false).unwrap();
    let mut game = ironsmith::GameState::new(vec!["A".into(), "B".into()], 20);
    let source = game.create_object_from_definition(&definition, a, Zone::Battlefield);
    let ability = definition.abilities.iter().find_map(|ability|
        if let AbilityKind::Triggered(trigger) = &ability.kind { Some(trigger) } else { None }).unwrap();
    let snapshot = ironsmith::snapshot::ObjectSnapshot::from_object(game.object(source).unwrap(), &game);
    for (keyword, expected) in [(Some(ironsmith_core::ActivatedAbilityKeyword::Craft), true),
        (Some(ironsmith_core::ActivatedAbilityKeyword::Equip), false), (None, false)] {
        let mut dm = SelectFirstDecisionMaker;
        let cause = CostContext::new(source, a, &mut dm)
            .with_reason(PaymentReason::activation(keyword, false)).event_cause();
        let event = TriggerEvent::new_with_provenance(ZoneChangeEvent::with_cause(
            source, Zone::Battlefield, Zone::Exile, cause, Some(snapshot.clone())), Default::default());
        assert_eq!(ability.trigger.matches(&event, &TriggerContext::for_source(source, a, &game)), expected);
    }
}

#[test]
fn optional_token_copy_assigns_creator_draws_for_opponent_and_counts_only_acceptance() {
    use ironsmith::ability::AbilityKind;
    use ironsmith::card::{CardBuilder, PowerToughness};
    use ironsmith::effects::{DoThisLimit, EffectContext, ResolvedTarget, execute_effect};
    use ironsmith::triggers::{TriggerEvent, TriggerIdentity};
    use ironsmith::{CardId, PlayerId, Zone};
    struct Accept(bool);
    impl ironsmith::decision::DecisionMaker for Accept {
        fn decide_boolean(&mut self, _: &ironsmith::GameState, _: &ironsmith::decisions::context::BooleanContext) -> bool { self.0 }
    }
    let definition = compile_to_runtime_definition("Copy watcher",
        "Mana cost: {2}{W}{U}\nType: Legendary Creature — Human\nPower/Toughness: 2/3\nGolden Rule — Whenever a token enters, you may have target player other than its controller create a token that's a copy of it, then you draw a card if an opponent created a token this way. Do this only once each turn.", false).unwrap();
    let trigger = definition.abilities.iter().find_map(|ability|
        if let AbilityKind::Triggered(trigger) = &ability.kind { Some(trigger) } else { None }).unwrap();
    assert!(format!("{:?}", trigger.intervening_if).contains("DoThisMaxTimesEachTurn"));
    let a = PlayerId::from_index(0);
    let b = PlayerId::from_index(1);
    let c = PlayerId::from_index(2);
    let mut game = ironsmith::GameState::new(vec!["A".into(), "B".into(), "C".into()], 20);
    let watcher = game.create_object_from_definition(&definition, a, Zone::Battlefield);
    let token = CardBuilder::new(CardId::new(), "Original token")
        .card_types(vec![ironsmith::types::CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1)).token().build();
    let entering = game.create_object_from_card(&token, c, Zone::Battlefield);
    let filler = CardBuilder::new(CardId::new(), "Drawn card").build();
    game.create_object_from_card(&filler, a, Zone::Library);
    let mut event = ironsmith::events::EnterBattlefieldEvent::new(entering, Zone::Command);
    event.completed_snapshot = Some(ironsmith::snapshot::ObjectSnapshot::from_object(game.object(entering).unwrap(), &game));
    let event = TriggerEvent::new_with_provenance(event, Default::default());
    let initial = game.battlefield.len();
    for (accept, expected_extra, expected_hand) in [(false, 0, 0), (true, 1, 1), (true, 1, 1)] {
        let mut dm = Accept(accept);
        let mut ctx = EffectContext::new(watcher, a, &mut dm)
            .with_targets(vec![ResolvedTarget::Player(b)]).with_triggering_event(event.clone());
        ctx.do_this_limit = Some(DoThisLimit { source: watcher, trigger_identity: TriggerIdentity(777), limit: 1 });
        for effect in trigger.effects.all_effects() { execute_effect(&mut game, effect, &mut ctx).unwrap(); }
        assert_eq!(game.battlefield.len(), initial + expected_extra);
        assert_eq!(game.player(a).unwrap().hand.len(), expected_hand);
    }
    let copies: Vec<_> = game.battlefield.iter().filter_map(|id| game.object(*id))
        .filter(|object| object.id != entering && object.name == "Original token").collect();
    assert_eq!(copies.len(), 1);
    assert_eq!(copies[0].owner, b);
}

#[test]
fn co_target_blocking_history_restricts_pair_and_counts_the_blocked_creatures_power() {
    use ironsmith::card::{CardBuilder, PowerToughness};
    use ironsmith::effects::{EffectContext, ResolvedTarget, execute_effect};
    use ironsmith::game_state::{Target, TargetAssignment};
    use ironsmith::triggers::TriggerEvent;
    use ironsmith::{CardId, PlayerId, Zone};
    let definition = compile_to_runtime_definition("Counter delusion",
        "Mana cost: {U}\nType: Instant\nPut X glyph counters on target creature that target Wall blocked this turn, where X is the power of that blocked creature. The creature gains \"This creature doesn't untap during your untap step if it has a glyph counter on it\" and \"At the beginning of your upkeep, remove a glyph counter from this creature.\"", false).unwrap();
    let a = PlayerId::from_index(0);
    let b = PlayerId::from_index(1);
    let mut game = ironsmith::GameState::new(vec!["A".into(), "B".into()], 20);
    let wall_card = CardBuilder::new(CardId::new(), "Wall")
        .card_types(vec![ironsmith::types::CardType::Creature])
        .subtypes(vec![ironsmith::types::Subtype::Wall])
        .power_toughness(PowerToughness::fixed(0, 5)).build();
    let creature = CardBuilder::new(CardId::new(), "Attacker")
        .card_types(vec![ironsmith::types::CardType::Creature])
        .power_toughness(PowerToughness::fixed(3, 4)).build();
    let wall = game.create_object_from_card(&wall_card, a, Zone::Battlefield);
    let attacker = game.create_object_from_card(&creature, b, Zone::Battlefield);
    let other = game.create_object_from_card(&creature, b, Zone::Battlefield);
    let event = TriggerEvent::new_with_provenance(
        ironsmith::events::combat::CreatureBlockedEvent::new(wall, attacker), Default::default());
    game.turn_store.turn_history.record_event(&event, None, None);
    let source = game.create_object_from_definition(&definition, a, Zone::Stack);
    let program = definition.spell_effect.as_ref().unwrap();
    let requirements = ironsmith::game_loop::extract_target_requirements_from_program_with_modes(
        &game, program, a, Some(source), None);
    assert_eq!(requirements.len(), 2);
    let pairs = &requirements[1].shared_player_group.as_ref().unwrap().pair_constraint.as_ref().unwrap().allowed_pairs;
    assert!(pairs.contains(&(Target::Object(wall), Target::Object(attacker))));
    assert!(!pairs.contains(&(Target::Object(wall), Target::Object(other))));
    let mut ctx = EffectContext::new_default(source, a).with_targets(vec![
        ResolvedTarget::Object(wall), ResolvedTarget::Object(attacker)]);
    ctx.target_assignments = vec![TargetAssignment { spec: requirements[0].spec.clone(), range: 0..1 },
        TargetAssignment { spec: requirements[1].spec.clone(), range: 1..2 }];
    for effect in program.all_effects() { execute_effect(&mut game, effect, &mut ctx).unwrap(); }
    assert_eq!(game.object(attacker).unwrap().counters.get(&ironsmith::object::CounterType::Glyph).copied().unwrap_or(0), 3);
    assert_eq!(game.object(other).unwrap().counters.get(&ironsmith::object::CounterType::Glyph).copied().unwrap_or(0), 0);
    // The targeted blocker can leave before resolution. Its historical
    // identity still constrains the surviving creature target; an empty
    // blocker assignment must not abort the spell or borrow another slot.
    game.move_object_by_effect(wall, Zone::Graveyard).unwrap();
    let mut ctx = EffectContext::new_default(source, a)
        .with_targets(vec![ResolvedTarget::Object(attacker)]);
    ctx.target_assignments = vec![TargetAssignment { spec: requirements[0].spec.clone(), range: 0..0 },
        TargetAssignment { spec: requirements[1].spec.clone(), range: 0..1 }];
    for effect in program.all_effects() { execute_effect(&mut game, effect, &mut ctx).unwrap(); }
    assert_eq!(game.object(attacker).unwrap().counters.get(&ironsmith::object::CounterType::Glyph).copied().unwrap_or(0), 6);
}
