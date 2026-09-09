use super::*;

const TEXT: &str = "If you would get one or more poison counters, instead you get one poison counter and you can't get additional poison counters this turn.\nExile this creature: Choose another target creature or artifact. When it's put into a graveyard this turn, return that card to the battlefield under its owner's control.";

fn definition() -> crate::cards::CardDefinition {
    crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Melira, the Living Cure")
        .card_types(vec![CardType::Creature])
        .power_toughness(crate::card::PowerToughness::fixed(3, 3))
        .parse_text(TEXT)
        .unwrap()
}

#[test]
fn watched_return_handles_noncreature_artifacts_and_only_the_chosen_object() {
    let definition = definition();
    let activated = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .unwrap();
    for kind in [CardType::Artifact, CardType::Creature] {
        for destination in [Zone::Graveyard, Zone::Exile] {
            let mut game =
                crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let alice = game.players[0].id;
            let bob = game.players[1].id;
            let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
            let card =
                crate::card::CardBuilder::new(crate::ids::CardId::new(), "Watched Permanent")
                    .card_types(vec![kind])
                    .power_toughness(crate::card::PowerToughness::fixed(4, 4))
                    .build();
            let watched = game.create_object_from_card(&card, bob, Zone::Battlefield);
            let other = game.create_object_from_card(&card, bob, Zone::Battlefield);
            let stable = game.object(watched).unwrap().stable_id;
            let mut ctx = crate::effects::EffectContext::new_default(source, alice)
                .with_targets(vec![crate::effects::ResolvedTarget::Object(watched)]);
            ctx.snapshot_targets(&game);
            game.move_object_by_effect(source, Zone::Exile).unwrap();
            for effect in &activated.effects {
                crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap();
            }
            assert_eq!(game.effect_store.delayed_triggers.len(), 1);
            for target in [other, watched] {
                let snapshot = crate::snapshot::ObjectSnapshot::from_object(
                    game.object(target).unwrap(),
                    &game,
                );
                let moved = game.move_object_by_effect(target, destination).unwrap();
                let event = crate::triggers::TriggerEvent::new_with_provenance(
                    crate::events::ZoneChangeEvent::with_results(
                        target,
                        vec![moved],
                        Zone::Battlefield,
                        destination,
                        crate::events::cause::EventCause::effect(),
                        Some(snapshot),
                    ),
                    crate::provenance::ProvNodeId::default(),
                );
                let entries = crate::triggers::check_delayed_triggers(&mut game, &event);
                let returns = target == watched && destination == Zone::Graveyard;
                assert_eq!(
                    entries.len(),
                    usize::from(returns),
                    "kind={kind:?}, destination={destination:?}, watched={}",
                    target == watched
                );
                let mut queue = crate::triggers::TriggerQueue::new();
                for entry in entries {
                    queue.add(entry);
                }
                crate::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
                if returns {
                    crate::game_loop::resolve_stack_entry(&mut game).unwrap();
                }
            }
            let current = game.find_object_by_stable_id(stable).unwrap();
            assert_eq!(
                game.object(current).unwrap().zone,
                if destination == Zone::Graveyard {
                    Zone::Battlefield
                } else {
                    Zone::Exile
                }
            );
            assert_eq!(game.controller_of(game.object(current).unwrap()), bob);
            if destination == Zone::Exile {
                // The exiled card is a new object; putting it into a graveyard
                // later must not revive the original watched permanent.
                let snapshot = crate::snapshot::ObjectSnapshot::from_object(
                    game.object(current).unwrap(),
                    &game,
                );
                let moved = game
                    .move_object_by_effect(current, Zone::Graveyard)
                    .unwrap();
                let event = crate::triggers::TriggerEvent::new_with_provenance(
                    crate::events::ZoneChangeEvent::with_results(
                        current,
                        vec![moved],
                        Zone::Exile,
                        Zone::Graveyard,
                        crate::events::cause::EventCause::effect(),
                        Some(snapshot),
                    ),
                    crate::provenance::ProvNodeId::default(),
                );
                assert!(crate::triggers::check_delayed_triggers(&mut game, &event).is_empty());
            }
        }
    }
}

#[test]
fn watched_graveyard_return_renders_as_a_reference_to_the_chosen_permanent() {
    let text = crate::compiled_text::compiled_text_lines(&definition()).join("\n");
    assert_eq!(text, TEXT);
}
