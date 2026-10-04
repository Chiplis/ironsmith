//! UNVALIDATED: one original occurrence survives split/reconverged replacements.
//! Consumes the passive damage receipt and idempotent ingestion prerequisites.
use ironsmith::decision::SelectFirstDecisionMaker;
use ironsmith::effect::Effect;
use ironsmith::effects::{EffectContext as ExecutionContext, execute_effect};
use ironsmith::events::damage::matchers::DamageToPlayerMatcher;
use ironsmith::events::{DamageEvent, DamageTarget};
use ironsmith::game_loop::{put_triggers_on_stack_with_dm, resolve_stack_entry_with};
use ironsmith::replacement::{RedirectTarget, RedirectWhich, ReplacementAction, ReplacementEffect};
use ironsmith::target::{ChooseSpec, PlayerFilter};
use ironsmith::triggers::TriggerQueue;
use ironsmith::{GameState, PlayerId, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
#[test]
fn split_reconverged_damage_keeps_one_threshold_trigger_and_one_history_count() {
    let (artifact,direct)=compile_to_artifact("Pain Magnification","Mana cost: {1}{B}{R}\nType: Enchantment\nWhenever an opponent is dealt 3 or more damage by a single source, that player discards a card.",false).unwrap();
    let decoded = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    let materialized =
        ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&decoded).unwrap();
    for definition in [direct, materialized] {
        for mode in 0..3 {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Charlie".into()], 20);
            let [a, b, c] = [
                PlayerId::from_index(0),
                PlayerId::from_index(1),
                PlayerId::from_index(2),
            ];
            let observer = game.create_object_from_definition(&definition, a, Zone::Battlefield);
            let source = game.create_object_from_definition(
                &compile_to_runtime_definition(
                    "Source",
                    "Type: Creature — Human\nPower/Toughness: 2/4",
                    false,
                )
                .unwrap(),
                a,
                Zone::Battlefield,
            );
            for n in 0..3 {
                game.create_object_from_definition(
                    &compile_to_runtime_definition(&format!("Hand {n}"), "Type: Land", false)
                        .unwrap(),
                    b,
                    Zone::Hand,
                );
            }
            if mode > 0 {
                game.effect_store.replacement_effects.add_resolution_effect(
                    ReplacementEffect::with_matcher(
                        observer,
                        a,
                        DamageToPlayerMatcher::new(PlayerFilter::Specific(b)),
                        ReplacementAction::RedirectDamageAmount {
                            target: RedirectTarget::ToPlayer(c),
                            which: RedirectWhich::First,
                            amount: 2,
                        },
                    ),
                );
            }
            if mode == 2 {
                game.effect_store.replacement_effects.add_resolution_effect(
                    ReplacementEffect::with_matcher(
                        observer,
                        a,
                        DamageToPlayerMatcher::new(PlayerFilter::Specific(c)),
                        ReplacementAction::Redirect {
                            target: RedirectTarget::ToPlayer(b),
                            which: RedirectWhich::First,
                        },
                    ),
                );
            }
            game.take_pending_trigger_events();
            let outcome = execute_effect(
                &mut game,
                &Effect::deal_damage(4, ChooseSpec::SpecificPlayer(b)),
                &mut ExecutionContext::new_default(source, a),
            )
            .unwrap();
            let damage = outcome
                .events
                .iter()
                .filter(|event| event.downcast::<DamageEvent>().is_some())
                .collect::<Vec<_>>();
            assert_eq!(damage.len(), if mode == 0 { 1 } else { 2 });
            assert_eq!(damage[0].simultaneous_batch().is_some(), mode > 0);
            if mode > 0 {
                assert_eq!(
                    damage[0].simultaneous_batch(),
                    damage[1].simultaneous_batch()
                );
            }
            if mode == 2 {
                assert!(
                    damage
                        .iter()
                        .all(|event| event.downcast::<DamageEvent>().unwrap().target
                            == DamageTarget::Player(b))
                );
            }
            for event in outcome.events {
                game.queue_trigger_event(Default::default(), event);
            }
            let mut dm = SelectFirstDecisionMaker;
            put_triggers_on_stack_with_dm(&mut game, &mut TriggerQueue::new(), &mut dm).unwrap();
            let expected = usize::from(mode != 1);
            assert_eq!(
                game.stack.len(),
                expected,
                "mode {mode}: threshold applies per recipient and one source, once per occurrence"
            );
            while !game.stack_is_empty() {
                resolve_stack_entry_with(&mut game, &mut dm).unwrap();
                put_triggers_on_stack_with_dm(&mut game, &mut TriggerQueue::new(), &mut dm)
                    .unwrap();
            }
            assert_eq!(game.player(b).unwrap().hand.len(), 3 - expected);
            let history = game
                .turn_store
                .turn_history
                .event_records
                .iter()
                .chain(game.turn_store.turn_history.staged_event_records.iter())
                .filter_map(|record| record.event.downcast::<DamageEvent>())
                .filter(|event| event.source == source)
                .collect::<Vec<_>>();
            assert_eq!(
                history.len(),
                if mode == 0 { 1 } else { 2 },
                "captured receipts must not be committed/staged again"
            );
            assert_eq!(history.iter().map(|event| event.amount).sum::<u32>(), 4);
            let maximum = ironsmith_core::Value::DamageHistory(Box::new(
                ironsmith_core::DamageHistoryQuery {
                    sources: ironsmith_core::DamageHistorySources::Any,
                    recipients: ironsmith_core::DamageHistoryRecipients::Any,
                    combat: None,
                    reduction:
                        ironsmith_core::DamageHistoryReduction::LargestSourceRecipientOccurrence,
                },
            ));
            assert_eq!(
                ironsmith::effects::helpers::resolve_value(
                    &game,
                    &maximum,
                    &ExecutionContext::new_default(source, a)
                )
                .unwrap(),
                if mode == 1 { 2 } else { 4 },
                "the occurrence maximum coalesces only fragments reaching the same recipient"
            );
        }
    }
}
