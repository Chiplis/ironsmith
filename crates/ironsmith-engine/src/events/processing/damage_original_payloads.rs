//! Explicit compatibility completion for ordinary damage-processing wrappers.
//! Prepared damage retains these intents for its original commit owner instead.
use super::*;

pub(super) fn complete_legacy(
    game: &mut GameState,
    dm: &mut dyn DecisionMaker,
    scope: &crate::effects::ReplacementExecutionContext,
    processed: &mut ProcessedDamageResult,
) -> Result<(), crate::effects::ExecutionError> {
    for program in std::mem::take(&mut processed.original_payloads) {
        let raw = program.context.event.clone().into_raw();
        let damage = crate::events::downcast_event::<crate::events::DamageEvent>(raw.inner())
            .ok_or_else(|| crate::effects::ExecutionError::InternalError(
                "retained original damage replacement lost its exact event".into(),
            ))?;
        let source_snapshot = raw.source_snapshot().cloned();
        let controller = damage.cause.source_controller
            .or_else(|| source_snapshot.as_ref().map(|snapshot| snapshot.controller))
            .unwrap_or(program.context.affected_player);
        let targets = vec![match damage.target {
            DamageTarget::Player(player) => crate::effects::ResolvedTarget::Player(player),
            DamageTarget::Object(object) => crate::effects::ResolvedTarget::Object(object),
        }];
        let mut ctx = crate::effects::ExecutionContext::new(damage.source, controller, &mut *dm)
            .with_cause(damage.cause.clone()).with_provenance(raw.provenance());
        ctx.source_snapshot = source_snapshot;
        ctx.replacement = scope.clone();
        let output = crate::effects::replacement::execute_replacement_payload_with_outputs(
            game, &mut ctx, &program.effects, program.source, program.controller,
            &program.context, Some(targets), program.source_snapshot, Vec::new(),
        )?;
        if ctx.decision_maker.awaiting_choice() { return Ok(()); }
        let mut original = crate::effect::EffectOutcome::replaced();
        original.set_value(crate::effect::OutcomeValue::Count(0));
        let completed = crate::effect::EffectOutcome::aggregate_replacement_outcomes(
            original, [output.into_outcome()],
        );
        processed.payload_outcome = Some(crate::effect::EffectOutcome::aggregate(
            processed.payload_outcome.take().into_iter().chain(std::iter::once(completed)),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effect::{Effect, EffectOutcome};
    use crate::effects::{CompletedEffectOutputs, ExecutionContext, SimultaneousEffectCommit};

    fn setup(effects: Vec<Effect>) -> (GameState, ObjectId, PlayerId, SimultaneousDamageEvent) {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let player = PlayerId::from_index(0);
        let card = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Instead source")
            .card_types(vec![crate::types::CardType::Artifact]).build();
        let source = game.create_object_from_card(&card, player, Zone::Battlefield);
        game.create_object_from_card(&card, player, Zone::Library);
        game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(
            source, player, crate::events::damage::matchers::DamageToPlayerMatcher::to_any_player(),
            ReplacementAction::Instead(effects),
        ));
        let event = SimultaneousDamageEvent {
            source, target: DamageTarget::Player(player), amount: 3, is_combat: false,
            unpreventable: false, cause: crate::events::cause::EventCause::from_effect(source, player),
            source_snapshot: None,
        };
        (game, source, player, event)
    }

    fn prepare_original(game: &mut GameState, ctx: &mut ExecutionContext, program: PreparedReplacementProgram)
        -> Result<SimultaneousEffectCommit<CompletedEffectOutputs>, crate::effects::ExecutionError> {
        crate::effects::replacement::prepare_draw_continuation_with_bindings_and_outputs(
            game, ctx, &program.effects, program.source, program.controller, &program.context,
            program.source_snapshot, crate::effects::replacement::ReplacementProgramBindings {
                targets: Some(vec![crate::effects::ResolvedTarget::Player(program.context.affected_player)]),
                object_tags: Vec::new(),
            },
        )?.ok_or_else(|| crate::effects::ExecutionError::InternalError("fixture must retain its authored draw".into()))
    }

    // Authored only: replacement selection must not execute even the non-draw prefix.
    #[test]
    fn damage_instead_is_sealed_without_actions_then_retained_at_its_first_draw() {
        let (mut game, source, player, event) = setup(vec![Effect::gain_life(2), Effect::draw(1), Effect::gain_life(4)]);
        let mut dm = crate::decision::SelectFirstDecisionMaker;
        let scope = crate::effects::ReplacementExecutionContext::default();
        let (mut selected, followups) = prepare_simultaneous_damage_assignments_with_scopes(
            &mut game, &[event], &mut dm, &[&scope],
        ).unwrap().into_parts();
        assert!(followups.iter().all(CapturedPreventionFollowUps::is_empty));
        assert_eq!(game.player(player).unwrap().life, 20);
        assert!(game.player(player).unwrap().hand.is_empty());
        assert!(selected[0].assignments.is_empty());
        assert!(selected[0].payload_outcome.is_none());
        assert_eq!(selected[0].original_payloads.len(), 1);
        let program = selected[0].original_payloads.remove(0);
        assert_eq!(program.source_snapshot.as_ref().unwrap().object_id, source);
        let mut ctx = ExecutionContext::new(source, player, &mut dm);
        let receipt = prepare_original(&mut game, &mut ctx, program).unwrap();
        assert!(receipt.completion.is_some());
        assert_eq!(game.player(player).unwrap().life, 22);
        assert!(game.player(player).unwrap().hand.is_empty());
        crate::effects::composition::complete_standalone_original_with_outputs(&mut game, &mut ctx, receipt).unwrap();
        assert_eq!(game.player(player).unwrap().life, 26, "the non-draw prefix executes once");
        assert_eq!(game.player(player).unwrap().hand.len(), 1);
    }

    #[derive(Default)]
    struct PendingChoice { pending: bool }
    impl DecisionMaker for PendingChoice {
        fn decide_boolean(&mut self, _: &GameState, _: &crate::decisions::context::BooleanContext) -> bool {
            self.pending = true;
            false
        }
        fn awaiting_choice(&self) -> bool { self.pending }
    }

    #[test]
    fn pending_original_payload_restores_prefix_without_starting_draw_or_suffix() {
        let (mut game, source, player, event) = setup(vec![
            Effect::gain_life(2), Effect::may(vec![Effect::draw(1)]), Effect::gain_life(4),
        ]);
        let mut dm = PendingChoice::default();
        let scope = crate::effects::ReplacementExecutionContext::default();
        let (mut selected, _) = prepare_simultaneous_damage_assignments_with_scopes(
            &mut game, &[event], &mut dm, &[&scope],
        ).unwrap().into_parts();
        assert!(!dm.awaiting_choice(), "sealing cannot ask a payload's decision");
        let program = selected[0].original_payloads.remove(0);
        let mut ctx = ExecutionContext::new(source, player, &mut dm);
        let receipt = crate::effects::composition::execute_transaction(&mut game, &mut ctx,
            || SimultaneousEffectCommit::finished(CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0))),
            |game, ctx| prepare_original(game, ctx, program),
        ).unwrap();
        assert!(ctx.decision_maker.awaiting_choice());
        assert!(receipt.completion.is_none());
        assert_eq!(game.player(player).unwrap().life, 20);
        assert!(game.player(player).unwrap().hand.is_empty());
        assert_eq!(game.player(player).unwrap().library.len(), 1);
    }

    #[test]
    fn ordinary_damage_wrapper_completes_its_selected_payload_once() {
        let (mut game, source, player, event) = setup(vec![Effect::gain_life(2), Effect::draw(1), Effect::gain_life(4)]);
        let processed = process_damage_assignments_with_event(
            &mut game, source, event.target, event.amount, event.is_combat, event.cause,
        ).unwrap();
        assert!(processed.original_payloads.is_empty());
        assert!(processed.payload_outcome.is_some());
        assert!(processed.assignments.is_empty());
        assert_eq!(game.player(player).unwrap().life, 26);
        assert_eq!(game.player(player).unwrap().hand.len(), 1);
    }
}
