use super::*;
const TEXT: &str = "When this creature dies, target land becomes a Swamp. Exile this card.";
#[test]
fn death_source_land_type_changes_only_the_land_and_exiles_the_dead_source() {
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Death land fixture")
        .card_types(vec![CardType::Creature]).parse_text(TEXT).unwrap();
    let trigger = definition.abilities.iter().find_map(|a| match &a.kind {
        AbilityKind::Triggered(t) => Some(t), _ => None,
    }).unwrap();
    for moved_again in [false, true] {
        let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = game.players[0].id; let bob = game.players[1].id;
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let snapshot = crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(game.object(source).unwrap(), &game);
        let graveyard = game.move_object_by_effect(source, Zone::Graveyard).unwrap();
        let mut event = crate::events::ZoneChangeEvent::with_cause(source, Zone::Battlefield, Zone::Graveyard,
            crate::events::cause::EventCause::effect(), Some(snapshot.clone()));
        event.result_objects = vec![graveyard];
        let returned = if moved_again { Some(game.move_object_by_effect(graveyard, Zone::Battlefield).unwrap()) } else { None };
        let land = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Forest fixture")
            .card_types(vec![CardType::Land]).subtypes(vec![Subtype::Forest]).parse_text("{T}: Add {C}{C}.").unwrap();
        let target = game.create_object_from_definition(&land, bob, Zone::Battlefield);
        let other = game.create_object_from_definition(&land, alice, Zone::Battlefield);
        let event = crate::triggers::TriggerEvent::new_with_provenance(event, crate::provenance::ProvNodeId::default());
        let mut ctx = crate::effects::EffectContext::new_default(source, alice)
            .with_triggering_event(event).with_targets(vec![crate::effects::ResolvedTarget::Object(target)]);
        ctx.source_snapshot = Some(snapshot);
        ctx.snapshot_targets(&game);
        for effect in &trigger.effects { crate::effects::execute_effect(&mut game, effect, &mut ctx)
            .unwrap_or_else(|e| panic!("moved_again={moved_again}: {e:?}")); }
        game.refresh_continuous_state();
        assert_eq!(game.calculated_subtypes(target), vec![Subtype::Swamp]);
        assert_eq!(game.calculated_subtypes(other), vec![Subtype::Forest]);
        let characteristics = game.calculated_characteristics(target).unwrap();
        let mana: Vec<_> = characteristics.abilities.iter().filter_map(|ability| match &ability.kind {
            AbilityKind::Activated(a) if a.is_mana_ability() => Some(a.mana_symbols().to_vec()), _ => None,
        }).collect();
        assert!(mana.contains(&vec![crate::mana::ManaSymbol::Black]), "{mana:?}");
        assert!(!mana.contains(&vec![crate::mana::ManaSymbol::Colorless, crate::mana::ManaSymbol::Colorless]), "{mana:?}");

        assert_eq!(game.object(target).unwrap().zone, Zone::Battlefield);
        if let Some(returned) = returned { assert_eq!(game.object(returned).unwrap().zone, Zone::Battlefield); }
        else { assert_eq!(game.objects_in_zone(Zone::Exile).len(), 1); assert!(game.object(graveyard).is_none()); }
        game.effect_store.continuous_effects.cleanup_end_of_turn();
        game.refresh_continuous_state();
        assert_eq!(game.calculated_subtypes(target), vec![Subtype::Swamp]);
    }
}
#[test]
fn death_source_land_type_renders_permanent_change_and_source_card() {
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Death land fixture")
        .card_types(vec![CardType::Creature]).parse_text(TEXT).unwrap();
    assert_eq!(crate::compiled_text::compiled_text_lines(&definition), [TEXT]);
}

#[test]
fn death_source_return_obeys_total_power_budget() {
    struct ChooseAll;
    impl crate::decision::DecisionMaker for ChooseAll {
        fn decide_objects(&mut self, _game: &crate::game_state::GameState, choice: &crate::decisions::context::SelectObjectsContext) -> Vec<crate::ids::ObjectId> {
            choice.candidates.iter().filter(|c| c.legal).map(|c| c.id).collect()
        }
    }
    for (aggregate, counters, powers, expected) in [(true, 0, [2, 3], 2), (true, 2, [3, 3], 6), (true, 0, [-1, 5], 4), (false, 0, [2, 3], 5)] {
    let text = "When this creature dies, return any number of other creature cards with total power X or less from your graveyard to the battlefield, where X is this creature's power. Exile this card.";
    let text = if aggregate { text.to_string() } else { text.replace("total power", "power") };
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Return budget fixture")
        .card_types(vec![CardType::Creature]).power_toughness(crate::card::PowerToughness::fixed(4, 4))
        .parse_text(&text).unwrap();
    let trigger = definition.abilities.iter().find_map(|a| match &a.kind { AbilityKind::Triggered(t) => Some(t), _ => None }).unwrap();
    let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = game.players[0].id;
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    if counters > 0 { game.add_counters(source, crate::CounterType::PlusOnePlusOne, counters); }
    let snapshot = crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(game.object(source).unwrap(), &game);
    let graveyard = game.move_object_by_effect(source, Zone::Graveyard).unwrap();
    let mut event = crate::events::ZoneChangeEvent::with_cause(source, Zone::Battlefield, Zone::Graveyard,
        crate::events::cause::EventCause::effect(), Some(snapshot.clone()));
    event.result_objects = vec![graveyard];
    for power in powers {
        let card = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Return candidate")
            .card_types(vec![CardType::Creature]).power_toughness(crate::card::PowerToughness::fixed(power, 4)).build();
        game.create_object_from_card(&card, alice, Zone::Graveyard);
    }
    let event = crate::triggers::TriggerEvent::new_with_provenance(event, crate::provenance::ProvNodeId::default());
    let mut dm = ChooseAll;
    let mut ctx = crate::effects::EffectContext::new(source, alice, &mut dm).with_triggering_event(event);
    ctx.source_snapshot = Some(snapshot);
    for effect in &trigger.effects { crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap(); }
    let total: i32 = game.battlefield.iter().map(|id| game.calculated_power(*id).unwrap_or(0)).sum();
    assert!(total > 0, "at least one offered creature must be returned by ChooseAll");
    assert_eq!(total, expected, "counters={counters}, powers={powers:?}");
    assert_eq!(game.objects_in_zone(Zone::Exile).len(), 1, "the source must be excluded and exiled");
    }
}

#[test]
fn death_source_return_renders_total_power_and_its_bound_x() {
    let text = "When this creature dies, return any number of other creature cards with total power X or less from your graveyard to the battlefield, where X is this creature's power. Exile this card.";
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Return budget fixture")
        .card_types(vec![CardType::Creature]).parse_text(text).unwrap();
    assert_eq!(crate::compiled_text::compiled_text_lines(&definition), [text]);
}
