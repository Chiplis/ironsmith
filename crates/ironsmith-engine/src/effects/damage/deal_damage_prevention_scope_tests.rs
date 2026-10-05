use super::*;
use crate::effect::{Effect, Value};
use crate::events::damage::matchers::DamageFromSourceMatcher;
use crate::ids::{CardId, PlayerId};
use crate::replacement::{EventModification, ReplacementAction, ReplacementEffect};
use crate::target::ObjectFilter;
use crate::zone::Zone;

#[test]
fn deferred_prevention_payloads_retain_parent_scope_and_source_lki() {
    for simultaneous in [false, true] {
        for suppressed in [false, true] {
            for departed in [false, true] {
                let alice = PlayerId::from_index(0);
                let bob = PlayerId::from_index(1);
                let carol = PlayerId::from_index(2);
                let mut game =
                    GameState::new(vec!["Alice".into(), "Bob".into(), "Carol".into()], 20);
                let definition =
                    crate::cards::CardDefinitionBuilder::new(CardId::new(), "Prevention source")
                        .card_types(vec![CardType::Creature])
                        .power_toughness(crate::card::PowerToughness::fixed(2, 2))
                        .build();
                let source =
                    game.create_object_from_definition(&definition, alice, Zone::Battlefield);
                let prevention = game.effect_store.replacement_effects.add_resolution_effect(
                    ReplacementEffect::with_matcher(
                        source,
                        alice,
                        DamageFromSourceMatcher::new(ObjectFilter::specific(source)),
                        ReplacementAction::PreventDamageThen(vec![Effect::gain_life(
                            Value::SourcePower,
                        )]),
                    ),
                );
                let snapshot =
                    crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(
                        game.object(source).unwrap(),
                        &game,
                    );
                if departed {
                    game.move_object(
                        source,
                        Zone::Exile,
                        crate::events::cause::EventCause::effect(),
                    )
                    .unwrap();
                }
                game.take_pending_trigger_events();
                let multiplier = ReplacementEffect::with_matcher(
                    source,
                    alice,
                    crate::events::WouldGainLifeMatcher::new(PlayerFilter::Specific(alice)),
                    ReplacementAction::Modify(EventModification::Multiply(2)),
                );
                let key = multiplier.application_key();
                let mut dm = crate::decision::SelectFirstDecisionMaker;
                let mut ctx = ExecutionContext::new(source, alice, &mut dm);
                ctx.source_snapshot = Some(snapshot);
                ctx.replacement
                    .additional_replacement_effects
                    .push(multiplier);
                if suppressed {
                    ctx.replacement
                        .suppressed_replacement_effect_keys
                        .insert(key.clone());
                }
                let spec = if simultaneous {
                    ChooseSpec::EachPlayer(PlayerFilter::Opponent)
                } else {
                    ChooseSpec::SpecificPlayer(bob)
                };
                let outcome = DealDamageEffect::new(5, spec)
                    .execute(&mut game, &mut ctx)
                    .unwrap();
                assert!(!ctx.decision_maker.awaiting_choice());
                let recipients = if simultaneous { 2 } else { 1 };
                let gain = if suppressed { 2 } else { 4 };
                assert_eq!(game.player(alice).unwrap().life, 20 + recipients * gain);
                assert_eq!(game.player(bob).unwrap().life, 20);
                assert_eq!(game.player(carol).unwrap().life, 20);
                assert_eq!(outcome.count_or_zero(), 0);
                assert_eq!(ctx.replacement.additional_replacement_effects.len(), 1);
                assert_eq!(
                    ctx.replacement
                        .suppressed_replacement_effect_keys
                        .contains(&key),
                    suppressed
                );
                assert!(ctx.replacement.suppressed_replacement_effects.is_empty());
                assert!(
                    game.effect_store
                        .replacement_effects
                        .get_effect(prevention)
                        .is_some()
                );
                let mut events = outcome.events;
                events.extend(game.take_pending_trigger_events());
                let mut occurrences = std::collections::HashSet::new();
                events.retain(|event| occurrences.insert(event.occurrence_key()));
                assert!(
                    !events
                        .iter()
                        .any(|event| event.downcast::<DamageEvent>().is_some())
                );
                let gains = events
                    .iter()
                    .filter_map(|event| event.downcast::<LifeGainEvent>())
                    .collect::<Vec<_>>();
                assert_eq!(gains.len(), recipients as usize);
                assert!(gains.iter().all(|event| event.player == alice
                    && event.amount == gain as u32
                    && event.source == Some(source)));
                assert!(
                    !game
                        .effect_store
                        .prevention_effects
                        .has_pending_follow_ups()
                );
                assert!(
                    !game
                        .effect_store
                        .prevention_effects
                        .follow_ups_are_deferred()
                );
                if departed {
                    let retained = events
                        .iter()
                        .filter(|event| event.downcast::<LifeGainEvent>().is_some())
                        .map(|event| {
                            event
                                .source_snapshot()
                                .expect("departed prevention source snapshot")
                        })
                        .collect::<Vec<_>>();
                    assert!(retained.iter().all(
                        |snapshot| snapshot.object_id == source && snapshot.controller == alice
                    ));
                }
                let next_snapshot = ctx.source_snapshot.clone();
                drop(ctx);
                let mut next = ExecutionContext::new(source, alice, &mut dm);
                next.source_snapshot = next_snapshot;
                DealDamageEffect::new(1, ChooseSpec::SpecificPlayer(bob))
                    .execute(&mut game, &mut next)
                    .unwrap();
                assert_eq!(
                    game.player(alice).unwrap().life,
                    22 + recipients * gain,
                    "a later independent event must not inherit the earlier temporary multiplier"
                );
            }
        }
    }
}
