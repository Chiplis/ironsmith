use super::*;
use crate::effect::{Effect, Value};
use crate::replacement::{EventModification, ReplacementAction, ReplacementEffect};

struct Answers {
    pause: bool,
    calls: usize,
    pending: bool,
}
impl crate::decision::DecisionMaker for Answers {
    fn decide_boolean(
        &mut self,
        _: &GameState,
        _: &crate::decisions::context::BooleanContext,
    ) -> bool {
        assert!(!self.pending);
        self.calls += 1;
        self.pending = self.pause && self.calls == 2;
        !self.pending
    }
    fn awaiting_choice(&self) -> bool {
        self.pending
    }
}

fn setup() -> (
    GameState,
    ObjectId,
    PlayerId,
    crate::replacement::ReplacementEffectId,
) {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = PlayerId::from_index(0);
    let card = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Program source")
        .card_types(vec![crate::types::CardType::Creature])
        .power_toughness(crate::card::PowerToughness::fixed(2, 2))
        .build();
    let source = game.create_object_from_card(&card, alice, Zone::Battlefield);
    game.object_mut(source)
        .unwrap()
        .abilities_mut()
        .push(crate::ability::Ability::triggered(
            crate::triggers::Trigger::you_gain_life(),
            vec![Effect::gain_life(1)],
        ));
    let one_shot =
        game.effect_store
            .replacement_effects
            .add_one_shot_effect(ReplacementEffect::with_matcher(
                source,
                alice,
                crate::events::WouldGainLifeMatcher::new(crate::target::PlayerFilter::Specific(
                    alice,
                )),
                ReplacementAction::Modify(EventModification::Multiply(2)),
            ));
    game.take_pending_trigger_events();
    (game, source, alice, one_shot)
}

#[test]
fn program_errors_keep_the_execution_error_and_restore_prior_replacements() {
    for match_triggers in [false, true] {
        let (mut game, source, alice, one_shot) = setup();
        let program = crate::resolution::ResolutionProgram::from_effects(vec![
            Effect::gain_life(2),
            Effect::lose_life(Value::X),
        ]);
        let mut ctx = ExecutionContext::new_default(source, alice);
        let result = execute_resolution_program_with_trigger_matching_typed(
            &mut game,
            &mut ctx,
            alice,
            source,
            &program,
            None,
            &[],
            match_triggers,
        );
        assert!(matches!(
            result,
            Err(crate::effects::ExecutionError::UnresolvableValue(_))
        ));
        assert_eq!(game.player(alice).unwrap().life, 20);
        assert!(
            game.effect_store
                .replacement_effects
                .get_effect(one_shot)
                .is_some()
        );
        assert!(game.take_pending_trigger_events().is_empty());
        assert!(ctx.replacement.suppressed_replacement_effects.is_empty());
        assert!(
            ctx.replacement
                .suppressed_replacement_effect_keys
                .is_empty()
        );
    }
}

#[test]
fn program_pause_restores_the_whole_program_and_replay_consumes_once() {
    for match_triggers in [false, true] {
        let (mut game, source, alice, one_shot) = setup();
        let program = crate::resolution::ResolutionProgram::from_effects(vec![
            Effect::gain_life(2),
            Effect::may(vec![Effect::gain_life(1)]),
            Effect::may(vec![Effect::gain_life(3)]),
        ]);
        let mut dm = Answers {
            pause: true,
            calls: 0,
            pending: false,
        };
        let mut ctx = ExecutionContext::new(source, alice, &mut dm);
        let events = execute_resolution_program_with_trigger_matching_typed(
            &mut game,
            &mut ctx,
            alice,
            source,
            &program,
            None,
            &[],
            match_triggers,
        )
        .unwrap();
        assert!(ctx.decision_maker.awaiting_choice());
        assert!(events.is_empty());
        assert_eq!(game.player(alice).unwrap().life, 20);
        assert!(
            game.effect_store
                .replacement_effects
                .get_effect(one_shot)
                .is_some()
        );
        assert!(game.take_pending_trigger_events().is_empty());
        assert!(game.take_pending_trigger_entries().is_empty());
        drop(ctx);
        assert_eq!(dm.calls, 2);
        let mut replay = Answers {
            pause: false,
            calls: 0,
            pending: false,
        };
        let mut ctx = ExecutionContext::new(source, alice, &mut replay);
        let mut events = execute_resolution_program_with_trigger_matching_typed(
            &mut game,
            &mut ctx,
            alice,
            source,
            &program,
            None,
            &[],
            match_triggers,
        )
        .unwrap();
        assert!(!ctx.decision_maker.awaiting_choice());
        assert_eq!(game.player(alice).unwrap().life, 28);
        assert!(
            game.effect_store
                .replacement_effects
                .get_effect(one_shot)
                .is_none()
        );
        events.extend(game.take_pending_trigger_events());
        let entries = game.take_pending_trigger_entries();
        if match_triggers {
            assert!(
                events.is_empty(),
                "already matched events must not be reported again"
            );
            assert_eq!(entries.len(), 3);
            assert!(
                entries
                    .iter()
                    .all(|entry| entry.source == source && entry.controller == alice)
            );
            events.extend(entries.into_iter().map(|entry| entry.triggering_event));
        } else {
            assert!(
                entries.is_empty(),
                "unmatched mode must leave trigger matching to the caller"
            );
        }
        let gains = events
            .iter()
            .filter_map(|event| event.downcast::<crate::events::LifeGainEvent>())
            .collect::<Vec<_>>();
        assert_eq!(gains.len(), 3);
        assert_eq!(gains.iter().map(|event| event.amount).sum::<u32>(), 8);
    }
}

#[test]
fn legacy_program_boundary_reports_failure_without_partial_consequences() {
    let (mut game, source, alice, one_shot) = setup();
    let program = crate::resolution::ResolutionProgram::from_effects(vec![
        Effect::gain_life(2),
        Effect::lose_life(Value::X),
    ]);
    let mut ctx = ExecutionContext::new_default(source, alice);
    assert!(matches!(
        execute_resolution_program(&mut game, &mut ctx, alice, source, &program, None, &[]),
        Err(GameLoopError::ExecutionFailed(
            crate::effects::ExecutionError::UnresolvableValue(_)
        ))
    ));
    assert_eq!(game.player(alice).unwrap().life, 20);
    assert!(
        game.effect_store
            .replacement_effects
            .get_effect(one_shot)
            .is_some()
    );
    assert!(game.take_pending_trigger_events().is_empty());
}
