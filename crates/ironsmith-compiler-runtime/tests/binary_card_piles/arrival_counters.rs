use super::*;
use ironsmith::effect::{Effect, EffectId, Value};
use ironsmith::target::{ChooseSpec, ObjectFilter, PlayerFilter};
use ironsmith::replacement::{ReplacementAction, ReplacementEffect};

fn grouped_move(effect: &Effect) -> bool {
    if effect.downcast_ref::<ironsmith::effects::MoveToZoneEffect>()
        .is_some_and(|effect| effect.tagged_destinations.len() == 2) { return true; }
    let mut found = false;
    effect.0.visit_child_effects(&mut |child| found |= grouped_move(child));
    found
}
fn result_bindings(effect: &Effect, producers: &mut Vec<EffectId>, consumers: &mut Vec<EffectId>) {
    if let Some(with_id) = effect.downcast_ref::<ironsmith::effects::WithIdEffect>()
        && grouped_move(&with_id.effect) { producers.push(with_id.id); }
    fn value(quantity: &Value, consumers: &mut Vec<EffectId>) {
        match quantity.unhinted() {
            Value::PriorEffectMetric { effect_id, query } => {
                assert_eq!(query.original_destination, Some(Zone::Graveyard));
                assert_eq!(query.action, Some(ironsmith::effect::PriorEffectAction::PutIntoGraveyard));
                consumers.push(*effect_id);
            }
            Value::Scaled(inner, _) => value(inner, consumers),
            Value::Fixed(1) => {},
            other => panic!("counter quantity must retain the actual movement result: {other:?}"),
        }
    }
    if let Some(counters) = effect.downcast_ref::<ironsmith::effects::PutCountersEffect>() {
        value(&counters.amount, consumers);
    }
    if let Some(repeat) = effect.downcast_ref::<ironsmith::effects::RepeatEffectsEffect>() {
        value(&repeat.count, consumers);
    }
    effect.0.visit_child_effects(&mut |child| result_bindings(child, producers, consumers));
}
fn counters(game: &GameState, expected: u32) {
    let thopters: Vec<_> = game.battlefield.iter().filter(|id| game.object(**id).unwrap().kind == ironsmith::object::ObjectKind::Token
        && game.object(**id).unwrap().subtypes.contains(&ironsmith::types::Subtype::Thopter)).copied().collect();
    if thopters.is_empty() { assert_eq!(expected, 0, "a positive Thopter must survive"); return; }
    assert_eq!(thopters.len(), 1);
    let thopter = thopters[0];
    assert_eq!(game.counter_count(thopter, ironsmith::object::CounterType::PlusOnePlusOne), expected);
    assert_eq!(game.current_power(thopter), Some(expected as i32));
    assert_eq!(game.current_toughness(thopter), Some(expected as i32));
    assert!(game.object(thopter).unwrap().card_types.contains(&ironsmith::types::CardType::Artifact));
    assert!(format!("{:?}", game.object(thopter).unwrap().abilities).contains("Flying"));
}

#[test]
fn intrude_counter_queries_bind_to_the_captured_move_not_token_creation() {
    for definition in definitions("Intrude on the Mind") {
        let mut producers = vec![];
        let mut consumers = vec![];
        for effect in definition.spell_effect.as_ref().unwrap().all_effects() {
            result_bindings(effect, &mut producers, &mut consumers);
        }
        assert!(!consumers.is_empty());
        assert!(consumers.iter().all(|id| producers.contains(id)));
    }
}

#[test]
fn intrude_uses_only_original_actual_graveyard_arrivals() {
    for definition in definitions("Intrude on the Mind") {
        for treatment in 0..6 {
            let mut game = game();
            let pool = library(&mut game, A, 5);
            let original_ids = pool.clone();
            let mut dm = Choices::new(pool, false, 2, 0);
            let source = cast(&mut game, &definition, &mut dm);
            let independent = compile_to_runtime_definition("Independent replacement card", "Type: Artifact", false).unwrap();
            let independent = game.create_object_from_definition(&independent, A, Zone::Exile);
            let grave = ObjectFilter::default().in_zone(Zone::Graveyard).owned_by(PlayerFilter::You);
            let action = match treatment {
                0 => None,
                1 => Some(ReplacementAction::Prevent),
                2 => Some(ReplacementAction::ChangeDestination(Zone::Exile)),
                3 => Some(ReplacementAction::Instead(vec![Effect::move_to_zone(ChooseSpec::SpecificObject(independent), Zone::Graveyard, false)])),
                4 => Some(ReplacementAction::Additionally(vec![Effect::move_to_zone(ChooseSpec::All(grave), Zone::Exile, false)])),
                _ => Some(ReplacementAction::Additionally(vec![Effect::move_to_zone(ChooseSpec::SpecificObject(independent), Zone::Graveyard, false)])),
            };
            if let Some(action) = action {
                game.effect_store.replacement_effects.add_resolution_effect(ReplacementEffect::with_matcher(source, A,
                    ironsmith::events::zones::matchers::WouldChangeZoneMatcher::new(
                        ObjectFilter::default(), Some(Zone::Library), Some(Zone::Graveyard)), action));
            }
            resolve(&mut game, &mut dm);
            assert_eq!(game.player(A).unwrap().hand.len(), 2);
            counters(&game, if matches!(treatment, 1..=3) { 0 } else { 3 });
            if treatment == 1 || treatment == 3 {
                assert_eq!(game.player(A).unwrap().library.len(), 3);
                assert!(game.player(A).unwrap().library.iter().all(|id| original_ids.contains(id)));
            }
        }
    }
}

#[test]
fn intrude_short_empty_piles_and_pending_added_program_keep_the_count_boundary() {
    for definition in definitions("Intrude on the Mind") {
        for available in [0, 1, 5] {
            for take in [0, available] {
                for mode in 0..2 {
                    let mut game = game();
                    let pool = library(&mut game, A, available);
                    let mut dm = Choices::new(pool, false, take, mode);
                    cast(&mut game, &definition, &mut dm);
                    resolve(&mut game, &mut dm);
                    counters(&game, if mode == 0 { (available - take) as u32 } else { take as u32 });
                }
            }
        }
        let mut game = game();
        let all = library(&mut game, A, 5);
        let mut dm = Choices::new(all.clone(), false, 2, 0);
        let source = cast(&mut game, &definition, &mut dm);
        game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(source, A,
            ironsmith::events::zones::matchers::WouldChangeZoneMatcher::new(
                ObjectFilter::default(), Some(Zone::Library), Some(Zone::Graveyard)),
            ReplacementAction::Additionally(vec![Effect::may(vec![Effect::gain_life(1)])])));
        dm.pause_replacement = true;
        resolve(&mut game, &mut dm);
        assert!(dm.pending);
        assert_eq!(game.stack.len(), 1);
        assert_eq!(game.player(A).unwrap().library, all);
        assert!(game.battlefield.is_empty());
        dm.pending = false;
        dm.pause_replacement = false;
        resolve(&mut game, &mut dm);
        counters(&game, 3);
        assert_eq!(game.player(A).unwrap().life, 21);
    }
}
