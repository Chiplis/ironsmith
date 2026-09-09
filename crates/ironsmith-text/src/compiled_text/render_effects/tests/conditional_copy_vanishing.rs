use super::*;

#[derive(Default)]
struct ChooseLastCopyOption;
impl crate::decision::DecisionMaker for ChooseLastCopyOption {
    fn decide_options(&mut self, _game: &crate::game_state::GameState, ctx: &crate::decisions::context::SelectOptionsContext) -> Vec<usize> {
        ctx.options.iter().rev().find(|option| option.legal).map(|option| vec![option.index]).unwrap_or_default()
    }
}

#[test]
fn entering_copy_adds_vanishing_only_when_chosen_source_lacks_it() {
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Conditional copy fixture")
        .card_types(vec![CardType::Creature])
        .parse_text("You may have this creature enter as a copy of any creature on the battlefield, except it has vanishing 3 if that creature doesn't have vanishing.").unwrap();
    assert!(definition.abilities.iter().any(|ability| matches!(&ability.kind,
        crate::ability::AbilityKind::Static(ability) if ability.enter_as_copy_as_enters().is_some())));
    for (existing, partial) in [(None, false), (Some(0), false), (Some(2), false), (Some(5), false), (Some(0), true)] {
        let expected = if partial { 3 } else { existing.unwrap_or(3) };
        let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = game.players[0].id;
        let source = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Copy source fixture")
            .card_types(vec![CardType::Creature]);
        let mut source = match existing {
            None => source.build(),
            Some(0) => source.parse_text("Vanishing").unwrap(),
            Some(amount) => source.parse_text(format!("Vanishing {amount}")).unwrap(),
        };
        if partial { source.abilities.remove(0); }
        game.create_object_from_definition(&source, alice, Zone::Battlefield);
        let object = game.create_object_from_definition(&definition, alice, Zone::Hand);
        let mut decisions = ChooseLastCopyOption;
        let entered = game.move_object_with_etb_processing_with_dm(object, Zone::Battlefield, &mut decisions).unwrap();
        assert_eq!(game.current_name(entered.new_id).as_deref(), Some("Copy source fixture"));
        let second = game.create_object_from_definition(&definition, alice, Zone::Hand);
        let second = game.move_object_with_etb_processing_with_dm(second, Zone::Battlefield, &mut ChooseLastCopyOption).unwrap().new_id;
        assert_eq!(game.object(second).unwrap().counters.get(&crate::object::CounterType::Time).copied().unwrap_or(0), expected, "a second copy retains the first copy's vanishing without adding it twice");
        assert_eq!(game.object(entered.new_id).unwrap().counters.get(&crate::object::CounterType::Time).copied().unwrap_or(0), expected, "existing={existing:?}; source={:#?}; condition={:#?}", source.abilities, definition.abilities.iter().find_map(|ability| match &ability.kind { crate::ability::AbilityKind::Static(ability) => ability.enter_as_copy_as_enters().map(|spec| &spec.added_abilities_source_filter), _ => None }));
    }
}

#[test]
fn conditional_copy_additions_match_the_chosen_source() {
    let mut definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Copy addition fixture")
        .card_types(vec![CardType::Creature])
        .parse_text("You may have this creature enter as a copy of any creature on the battlefield, except it has flying.").unwrap();
    let ability = definition.abilities.iter_mut().find(|ability| matches!(&ability.kind,
        crate::ability::AbilityKind::Static(ability) if ability.enter_as_copy_as_enters().is_some())).unwrap();
    let crate::ability::AbilityKind::Static(original) = &ability.kind else { unreachable!() };
    let mut spec = original.enter_as_copy_as_enters().unwrap().clone();
    let mut condition = ObjectFilter::default();
    condition.name = Some("Eligible source".into());
    spec.added_abilities_source_filter = Some(condition);
    ability.kind = crate::ability::AbilityKind::Static(crate::static_abilities::StaticAbility::with_enter_as_copy_as_enters(spec, "Conditional copy addition".to_string()));
    for (name, should_fly) in [("Eligible source", true), ("Other source", false)] {
        let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = game.players[0].id;
        let source = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), name).card_types(vec![CardType::Creature]).build();
        game.create_object_from_definition(&source, alice, Zone::Battlefield);
        let object = game.create_object_from_definition(&definition, alice, Zone::Hand);
        let entered = game.move_object_with_etb_processing_with_dm(object, Zone::Battlefield, &mut ChooseLastCopyOption).unwrap();
        assert_eq!(game.current_name(entered.new_id).as_deref(), Some(name));
        assert_eq!(game.current_has_static_ability_id(entered.new_id, crate::static_abilities::StaticAbilityId::Flying), should_fly);
    }
}

#[test]
fn copied_vanishing_decays_on_its_controllers_upkeep_and_only_last_time_counter() {
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Copy decay fixture")
        .card_types(vec![CardType::Creature])
        .parse_text("You may have this creature enter as a copy of any creature on the battlefield, except it has vanishing 3 if that creature doesn't have vanishing.").unwrap();
    let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = game.players[0].id; let bob = game.players[1].id;
    let original = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Plain creature")
        .card_types(vec![CardType::Creature]).build();
    let original = game.create_object_from_definition(&original, bob, Zone::Battlefield);
    let source = game.create_object_from_definition(&definition, alice, Zone::Hand);
    let source = game.move_object_with_etb_processing_with_dm(source, Zone::Battlefield, &mut ChooseLastCopyOption).unwrap().new_id;
    let upkeep = |player| crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::BeginningOfUpkeepEvent::new(player), crate::provenance::ProvNodeId::default());
    assert!(crate::triggers::check_triggers(&game, &upkeep(bob)).is_empty());
    let triggers = crate::triggers::check_triggers(&game, &upkeep(alice));
    assert_eq!(triggers.len(), 1);
    for trigger in triggers {
        assert_eq!(trigger.source, source);
        let mut ctx = crate::effects::EffectContext::new_default(source, alice);
        ctx.triggering_event = Some(trigger.triggering_event);
        for segment in &trigger.ability.effects.segments {
            for effect in &segment.default_effects { crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap(); }
        }
    }
    assert_eq!(game.object(source).unwrap().counters.get(&crate::object::CounterType::Time), Some(&2));
    game.add_counters(source, crate::object::CounterType::PlusOnePlusOne, 1);
    let (_, other_counter) = game.remove_counters(source, crate::object::CounterType::PlusOnePlusOne, 1, None, None).unwrap();
    assert!(crate::triggers::check_triggers(&game, &other_counter).is_empty());
    game.add_counters(original, crate::object::CounterType::Time, 1);
    let (_, other_object) = game.remove_counters(original, crate::object::CounterType::Time, 1, None, None).unwrap();
    assert!(crate::triggers::check_triggers(&game, &other_object).is_empty());
    let (_, last_counters) = game.remove_counters(source, crate::object::CounterType::Time, 2, None, None).unwrap();
    assert!(crate::triggers::check_triggers(&game, &upkeep(alice)).is_empty(), "vanishing cannot trigger upkeep with no time counters");
    let triggers = crate::triggers::check_triggers(&game, &last_counters);
    assert_eq!(triggers.len(), 1, "removing the last two counters together triggers once");
    for trigger in triggers {
        let mut ctx = crate::effects::EffectContext::new_default(source, alice);
        ctx.triggering_event = Some(trigger.triggering_event);
        for segment in &trigger.ability.effects.segments {
            for effect in &segment.default_effects { crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap(); }
        }
    }
    assert!(!game.battlefield.contains(&source));
    assert!(game.battlefield.contains(&original));
}


struct DeclineCopyOption;
impl crate::decision::DecisionMaker for DeclineCopyOption {
    fn decide_options(&mut self, _game: &crate::game_state::GameState, ctx: &crate::decisions::context::SelectOptionsContext) -> Vec<usize> {
        ctx.options.iter().find(|option| option.legal).map(|option| vec![option.index]).unwrap_or_default()
    }
}

#[test]
fn declining_conditional_copy_does_not_add_vanishing() {
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Copy decline fixture")
        .card_types(vec![CardType::Creature])
        .parse_text("You may have this creature enter as a copy of any creature on the battlefield, except it has vanishing 3 if that creature doesn't have vanishing.").unwrap();
    let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = game.players[0].id;
    let original = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Available source").card_types(vec![CardType::Creature]).build();
    game.create_object_from_definition(&original, alice, Zone::Battlefield);
    let source = game.create_object_from_definition(&definition, alice, Zone::Hand);
    let source = game.move_object_with_etb_processing_with_dm(source, Zone::Battlefield, &mut DeclineCopyOption).unwrap().new_id;
    assert_eq!(game.current_name(source).as_deref(), Some("Copy decline fixture"));
    assert_eq!(game.object(source).unwrap().counters.get(&crate::object::CounterType::Time), None);
    let upkeep = crate::triggers::TriggerEvent::new_with_provenance(crate::events::BeginningOfUpkeepEvent::new(alice), crate::provenance::ProvNodeId::default());
    assert!(crate::triggers::check_triggers(&game, &upkeep).is_empty());
}
