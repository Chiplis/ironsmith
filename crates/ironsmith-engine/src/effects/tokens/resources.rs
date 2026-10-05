//! Computation limits are incomplete-execution boundaries, never Magic rules.
use crate::effects::ExecutionError;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TokenCreationLimits {
    pub max_created_tokens: u32,
    pub max_live_objects: usize,
    pub max_instructions: u32,
    pub max_nesting: u32,
}
impl Default for TokenCreationLimits {
    fn default() -> Self {
        Self {
            max_created_tokens: 16_384,
            max_live_objects: 65_536,
            max_instructions: 4_096,
            max_nesting: 64,
        }
    }
}
#[derive(Debug)]
pub(crate) struct TokenCreationMeter {
    limits: TokenCreationLimits,
    active: bool,
    failure: Option<ExecutionError>,
    created: u32,
    outstanding: u32,
    instructions: u32,
    nesting: u32,
}
pub(crate) type SharedTokenCreationMeter = Arc<Mutex<TokenCreationMeter>>;
pub(crate) fn new_meter(limits: TokenCreationLimits) -> SharedTokenCreationMeter {
    Arc::new(Mutex::new(TokenCreationMeter {
        limits,
        active: true,
        failure: None,
        created: 0,
        outstanding: 0,
        instructions: 0,
        nesting: 0,
    }))
}
pub(crate) fn meter_is_active(meter: &SharedTokenCreationMeter) -> bool {
    // Poison must reach the typed execution error, not open a fresh allowance.
    meter.lock().map_or(true, |state| state.active)
}
pub(crate) fn close_meter(meter: &SharedTokenCreationMeter) {
    if let Ok(mut state) = meter.lock() {
        state.active = false;
    }
}
pub(crate) fn record_failure(meter: &SharedTokenCreationMeter, error: &ExecutionError) {
    if error.is_incomplete_execution()
        && let Ok(mut state) = meter.lock()
        && state.failure.is_none()
    {
        state.failure = Some(error.clone());
    }
}
pub(crate) fn failure(meter: &SharedTokenCreationMeter) -> Option<ExecutionError> {
    match meter.lock() {
        Ok(state) => state.failure.clone(),
        Err(_) => Some(ExecutionError::InternalError(
            "token resource meter poisoned".into(),
        )),
    }
}

/// A simulation query owns an independent work budget. Its branch clones share
/// this allowance across slices, but do not consume actual-game creation slots.
#[derive(Debug)]
pub(crate) struct TokenQueryScope(SharedTokenCreationMeter);
impl TokenQueryScope {
    pub(crate) fn new(limits: TokenCreationLimits) -> Self {
        Self(new_meter(limits))
    }
    pub(crate) fn meter(&self) -> SharedTokenCreationMeter {
        self.0.clone()
    }
}
impl Drop for TokenQueryScope {
    fn drop(&mut self) {
        close_meter(&self.0);
    }
}

pub(crate) fn limit(resource: &'static str, requested: u128, maximum: u128) -> ExecutionError {
    ExecutionError::ResourceLimitExceeded {
        resource,
        requested,
        maximum,
    }
}
pub(crate) fn checked_token_count(count: u128) -> Result<u32, ExecutionError> {
    u32::try_from(count).map_err(|_| limit("token count representation", count, u32::MAX as u128))
}
/// One charged instruction split across simultaneous preparation/completion.
/// Sibling permits share work accounting, but nesting is active only while a
/// participant is executing a phase, never while it waits for its siblings.
pub(crate) struct TokenInstructionPermit(SharedTokenCreationMeter);
impl TokenInstructionPermit {
    pub(crate) fn charge(meter: SharedTokenCreationMeter) -> Result<Self, ExecutionError> {
        {
            let mut state = meter.lock().map_err(|_| {
                ExecutionError::InternalError("token resource meter poisoned".into())
            })?;
            let instructions = u64::from(state.instructions) + 1;
            if instructions > u64::from(state.limits.max_instructions) {
                return Err(limit(
                    "token instruction work",
                    instructions as u128,
                    state.limits.max_instructions as u128,
                ));
            }
            state.instructions = instructions as u32;
        }
        Ok(Self(meter))
    }
    pub(crate) fn enter_phase(&self) -> Result<TokenInstructionGuard, ExecutionError> {
        {
            let mut state = self.0.lock().map_err(|_| {
                ExecutionError::InternalError("token resource meter poisoned".into())
            })?;
            let nesting = u64::from(state.nesting) + 1;
            if nesting > u64::from(state.limits.max_nesting) {
                return Err(limit(
                    "nested token instructions",
                    nesting as u128,
                    state.limits.max_nesting as u128,
                ));
            }
            state.nesting = nesting as u32;
        }
        Ok(TokenInstructionGuard(self.0.clone()))
    }
}

pub(crate) struct TokenInstructionGuard(SharedTokenCreationMeter);
impl TokenInstructionGuard {
    pub(crate) fn enter(meter: SharedTokenCreationMeter) -> Result<Self, ExecutionError> {
        {
            let mut state = meter.lock().map_err(|_| {
                ExecutionError::InternalError("token resource meter poisoned".into())
            })?;
            let instructions = u64::from(state.instructions) + 1;
            let nesting = u64::from(state.nesting) + 1;
            if instructions > u64::from(state.limits.max_instructions) {
                return Err(limit(
                    "token instruction work",
                    instructions as u128,
                    state.limits.max_instructions as u128,
                ));
            }
            if nesting > u64::from(state.limits.max_nesting) {
                return Err(limit(
                    "nested token instructions",
                    nesting as u128,
                    state.limits.max_nesting as u128,
                ));
            }
            state.instructions = instructions as u32;
            state.nesting = nesting as u32;
        }
        Ok(Self(meter))
    }
}
impl Drop for TokenInstructionGuard {
    fn drop(&mut self) {
        if let Ok(mut state) = self.0.lock() {
            state.nesting -= 1;
        }
    }
}
pub(crate) fn reserve_creation(
    meter: &SharedTokenCreationMeter,
    requested: u128,
    live_objects: usize,
) -> Result<(), ExecutionError> {
    let mut state = meter
        .lock()
        .map_err(|_| ExecutionError::InternalError("token resource meter poisoned".into()))?;
    let total = state.created as u128 + requested;
    if total > state.limits.max_created_tokens as u128 {
        return Err(limit(
            "token creation work",
            total,
            state.limits.max_created_tokens as u128,
        ));
    }
    // The current object store includes completed siblings, while outstanding
    // reserves only tokens not yet inserted. Non-token allocations in an entry
    // program are therefore visible without hiding reserved future siblings.
    let live = live_objects as u128 + state.outstanding as u128 + requested;
    if live > state.limits.max_live_objects as u128 {
        return Err(limit(
            "live game objects",
            live,
            state.limits.max_live_objects as u128,
        ));
    }
    state.outstanding += requested as u32;
    state.created = total as u32;
    Ok(())
}
pub(crate) fn commit_materialization(
    meter: &SharedTokenCreationMeter,
    live_objects: usize,
) -> Result<(), ExecutionError> {
    let mut state = meter
        .lock()
        .map_err(|_| ExecutionError::InternalError("token resource meter poisoned".into()))?;
    if state.outstanding == 0 {
        return Err(ExecutionError::InternalError(
            "token insertion has no reserved slot".into(),
        ));
    }
    let live = live_objects as u128 + state.outstanding as u128;
    if live > state.limits.max_live_objects as u128 {
        return Err(limit(
            "live game objects",
            live,
            state.limits.max_live_objects as u128,
        ));
    }
    state.outstanding -= 1;
    Ok(())
}
pub(crate) fn reserve_repetition_work(
    meter: &SharedTokenCreationMeter,
    repetitions: usize,
) -> Result<(), ExecutionError> {
    let mut state = meter
        .lock()
        .map_err(|_| ExecutionError::InternalError("token resource meter poisoned".into()))?;
    let work = state.instructions as u128 + repetitions as u128;
    if work > state.limits.max_instructions as u128 {
        return Err(limit(
            "repeated token instructions",
            work,
            state.limits.max_instructions as u128,
        ));
    }
    state.instructions = work as u32;
    Ok(())
}

/// Fallible owner buffers complement the work limits. Allocator refusal remains
/// an incomplete operation and is rolled back by the instruction owner.
pub(crate) fn buffer<T>(capacity: usize) -> Result<Vec<T>, ExecutionError> {
    let mut buffer = Vec::new();
    buffer
        .try_reserve_exact(capacity)
        .map_err(|_| ExecutionError::ResourceAllocationFailed {
            resource: "token result buffer",
            requested: capacity,
        })?;
    Ok(buffer)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cards::{CardDefinition, CardDefinitionBuilder};
    use crate::effect::Effect;
    use crate::effects::{CreateTokenEffect, EffectExecutor, ExecutionContext, IncubateEffect};
    use crate::game_state::GameState;
    use crate::ids::{CardId, PlayerId};
    use crate::zone::Zone;

    fn alice() -> PlayerId {
        PlayerId::from_index(0)
    }
    fn token() -> CardDefinition {
        CardDefinitionBuilder::new(CardId::new(), "Resource boundary token")
            .token()
            .card_types(vec![crate::types::CardType::Creature])
            .power_toughness(crate::card::PowerToughness::fixed(1, 1))
            .build()
    }
    fn game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }
    fn limits(tokens: u32) -> TokenCreationLimits {
        TokenCreationLimits {
            max_created_tokens: tokens,
            ..Default::default()
        }
    }
    fn exhausted<T>(result: Result<T, ExecutionError>) {
        assert!(matches!(
            result,
            Err(ExecutionError::ResourceLimitExceeded { .. })
        ));
    }

    #[derive(Debug, Clone)]
    struct AbsorbedChildError;
    impl EffectExecutor for AbsorbedChildError {
        fn execute(
            &self,
            game: &mut GameState,
            ctx: &mut ExecutionContext,
        ) -> Result<crate::effect::EffectOutcome, ExecutionError> {
            let _ignored = CreateTokenEffect::you(token(), 2).execute(game, ctx);
            game.player_mut(alice()).unwrap().life += 7;
            Ok(crate::effect::EffectOutcome::resolved())
        }
    }
    #[test]
    fn outer_owner_rejects_a_resource_error_absorbed_by_an_inner_branch() {
        let mut game = game();
        let source = game.new_object_id();
        game.set_token_creation_limits(limits(1));
        let mut ctx = ExecutionContext::new_default(source, alice());
        exhausted(crate::effects::execute_effect(
            &mut game,
            &Effect::new(AbsorbedChildError),
            &mut ctx,
        ));
        assert_eq!(game.player(alice()).unwrap().life, 20);
        assert!(game.battlefield.is_empty());
        assert!(game.take_pending_trigger_events().is_empty());
    }

    #[test]
    fn capacity_overflow_is_a_typed_allocator_boundary() {
        assert!(matches!(
            buffer::<u128>(usize::MAX),
            Err(ExecutionError::ResourceAllocationFailed { .. })
        ));
    }

    #[test]
    fn nested_creation_cannot_spend_reserved_outer_object_capacity() {
        let meter = new_meter(TokenCreationLimits {
            max_live_objects: 4,
            ..Default::default()
        });
        reserve_creation(&meter, 2, 1).unwrap();
        commit_materialization(&meter, 1).unwrap();
        // One outer token exists; the second is reserved but not materialized.
        exhausted(reserve_creation(&meter, 2, 2));
        reserve_creation(&meter, 1, 2).unwrap();
    }

    #[test]
    fn non_token_allocations_cannot_hide_unmaterialized_siblings() {
        let meter = new_meter(TokenCreationLimits {
            max_live_objects: 5,
            ..Default::default()
        });
        reserve_creation(&meter, 2, 1).unwrap();
        commit_materialization(&meter, 1).unwrap();
        // The first token plus two copied spells now exist alongside the source.
        exhausted(reserve_creation(&meter, 1, 4));
        // Further external growth also fails at the remaining token insertion.
        exhausted(commit_materialization(&meter, 5));
    }

    #[test]
    fn saved_continuation_starts_a_fresh_budget_after_the_attempt_closes() {
        let mut game = game();
        game.set_token_creation_limits(limits(2));
        let (root, meter) = game.begin_token_resource_scope();
        game.reserve_token_creation(2).unwrap();
        let mut saved = game.clone();
        let (nested, shared) = saved.begin_token_resource_scope();
        assert!(!nested);
        assert!(Arc::ptr_eq(&meter, &shared));
        game.end_token_resource_scope(root, &meter);
        let (replay, fresh) = saved.begin_token_resource_scope();
        assert!(replay);
        assert!(!Arc::ptr_eq(&meter, &fresh));
        saved.reserve_token_creation(2).unwrap();
        saved.end_token_resource_scope(replay, &fresh);
    }

    #[test]
    fn guards_bound_work_and_depth_without_converting_exhaustion_to_success() {
        let meter = new_meter(TokenCreationLimits {
            max_nesting: 1,
            max_instructions: 2,
            ..Default::default()
        });
        let first = TokenInstructionGuard::enter(meter.clone()).unwrap();
        exhausted(TokenInstructionGuard::enter(meter.clone()));
        drop(first);
        drop(TokenInstructionGuard::enter(meter.clone()).unwrap());
        exhausted(TokenInstructionGuard::enter(meter));
    }

    #[test]
    fn proposal_arithmetic_does_not_saturate_or_wrap_before_later_replacements() {
        use crate::events::{CreateTokensEvent, cause::EventCause};
        let event = CreateTokensEvent::with_cause(alice(), u32::MAX, EventCause::effect());
        exhausted(event.doubled());
        exhausted(event.with_additional_tokens(ironsmith_core::AdditionalTokenKind::Food, 1));
        exhausted(event.with_template(token(), 1));
        let mut external = event;
        external
            .additional_tokens
            .push((ironsmith_core::AdditionalTokenKind::Food, 1));
        assert_eq!(external.total_count(), u128::from(u32::MAX) + 1);
        exhausted(external.with_count(u32::MAX));
        // An invalid covered input cannot become a plausible zero by saturating
        // first and then applying a subsequent replacement.
        exhausted(external.adjusted_token_total(|_| true, |_| 0));
    }

    #[test]
    fn incubate_failure_after_earlier_iterations_restores_every_owned_receipt() {
        let mut game = game();
        game.set_token_creation_limits(limits(3));
        let source = game.new_object_id();
        let next = game.next_object_id_counter();
        let mut ctx = ExecutionContext::new_default(source, alice());
        ctx.x_value = Some(73);
        exhausted(IncubateEffect::you(2, 4).execute(&mut game, &mut ctx));
        assert!(game.battlefield.is_empty());
        assert!(game.command_zone.is_empty());
        assert!(game.objects_in_deterministic_order().is_empty());
        assert_eq!(game.next_object_id_counter(), next);
        assert_eq!(ctx.x_value, Some(73));
        assert!(game.take_pending_trigger_events().is_empty());
        assert!(game.effect_store.delayed_triggers.is_empty());
        // A new attempt has a fresh computation allowance, not a rules counter.
        let out = IncubateEffect::you(2, 3)
            .execute(&mut game, &mut ctx)
            .unwrap();
        assert_eq!(out.result_objects().unwrap().len(), 3);
    }

    #[test]
    fn repeated_incubate_is_checked_before_allocating_or_registering_faces() {
        let mut game = game();
        let source = game.new_object_id();
        game.set_token_creation_limits(TokenCreationLimits {
            max_instructions: 2,
            ..Default::default()
        });
        let next = game.next_object_id_counter();
        exhausted(IncubateEffect::you(1, 3).execute(
            &mut game,
            &mut ExecutionContext::new_default(source, alice()),
        ));
        assert_eq!(game.next_object_id_counter(), next);
        assert!(game.objects_in_deterministic_order().is_empty());
        assert!(game.take_pending_trigger_events().is_empty());
    }

    #[test]
    fn nested_entry_payload_shares_meter_and_rolls_back_tokens_life_and_one_shot() {
        use crate::replacement::{ReplacementAction, ReplacementEffect};
        let mut game = game();
        let source = game.new_object_id();
        game.set_token_creation_limits(limits(2));
        let shield = game.effect_store.replacement_effects.add_one_shot_effect(
            ReplacementEffect::with_matcher(
                source,
                alice(),
                crate::events::zones::matchers::WouldEnterBattlefieldMatcher::new(
                    crate::target::ObjectFilter::default(),
                ),
                ReplacementAction::Additionally(vec![
                    Effect::gain_life(3),
                    Effect::new(CreateTokenEffect::one(token())),
                ]),
            ),
        );
        let next = game.next_object_id_counter();
        let effect = CreateTokenEffect::you(token(), 2)
            .tapped()
            .exile_at_next_end_step();
        let mut ctx = ExecutionContext::new_default(source, alice());
        exhausted(effect.execute(&mut game, &mut ctx));
        assert_eq!(game.player(alice()).unwrap().life, 20);
        assert_eq!(game.next_object_id_counter(), next);
        assert!(game.battlefield.is_empty());
        assert!(game.command_zone.is_empty());
        assert!(game.effect_store.delayed_triggers.is_empty());
        assert!(game.take_pending_trigger_events().is_empty());
        assert!(
            game.effect_store
                .replacement_effects
                .get_effect(shield)
                .is_some()
        );
        game.set_token_creation_limits(limits(3));
        effect.execute(&mut game, &mut ctx).unwrap();
        assert_eq!(game.battlefield.len(), 3);
        assert_eq!(game.player(alice()).unwrap().life, 23);
        assert_eq!(
            game.effect_store.delayed_triggers.len(),
            2,
            "only outer instruction owns cleanup"
        );
    }

    #[test]
    fn repeated_keyword_owners_cannot_reset_budget_between_direct_children() {
        for populate in [false, true] {
            let mut game = game();
            let source = game.new_object_id();
            if populate {
                CreateTokenEffect::one(token())
                    .execute(
                        &mut game,
                        &mut ExecutionContext::new_default(source, alice()),
                    )
                    .unwrap();
            }
            let before = game.battlefield.len();
            let next = game.next_object_id_counter();
            game.take_pending_trigger_events();
            game.set_token_creation_limits(limits(2));
            let mut ctx = ExecutionContext::new_default(source, alice());
            let result = if populate {
                crate::effects::PopulateEffect::new(3).execute(&mut game, &mut ctx)
            } else {
                crate::effects::InvestigateEffect::you(3).execute(&mut game, &mut ctx)
            };
            exhausted(result);
            assert_eq!(game.battlefield.len(), before);
            assert_eq!(game.next_object_id_counter(), next);
            assert!(game.take_pending_trigger_events().is_empty());
        }
    }

    #[test]
    fn dispatcher_resource_error_restores_prefix_and_resolution_context() {
        let mut game = game();
        let source = game.new_object_id();
        game.set_token_creation_limits(limits(1));
        let effect = Effect::new(crate::effects::SequenceEffect::new(vec![
            Effect::gain_life(4),
            Effect::new(CreateTokenEffect::you(token(), 2)),
        ]));
        let mut ctx = ExecutionContext::new_default(source, alice());
        ctx.x_value = Some(29);
        exhausted(crate::effects::execute_effect(&mut game, &effect, &mut ctx));
        assert_eq!(game.player(alice()).unwrap().life, 20);
        assert_eq!(ctx.x_value, Some(29));
        assert!(game.battlefield.is_empty());
        assert!(game.take_pending_trigger_events().is_empty());
    }

    #[test]
    fn stack_resolution_shares_budget_across_instructions_and_keeps_spell_on_error() {
        use crate::game_loop::{GameLoopError, resolve_stack_entry};
        let mut game = game();
        game.set_token_creation_limits(limits(3));
        let definition = CardDefinitionBuilder::new(CardId::new(), "Resource budget spell")
            .card_types(vec![crate::types::CardType::Sorcery])
            .build();
        let source = game.create_object_from_definition(&definition, alice(), Zone::Stack);
        let program = crate::resolution::ResolutionProgram::from_effects(vec![
            Effect::gain_life(4),
            Effect::new(CreateTokenEffect::you(token(), 2).exile_at_next_end_step()),
            Effect::new(CreateTokenEffect::you(token(), 2)),
        ]);
        game.object_mut(source).unwrap().spell_effect = Some(program.into());
        game.push_to_stack(crate::game_state::StackEntry::new(source, alice()));
        game.take_pending_trigger_events();
        let next = game.next_object_id_counter();
        let result = resolve_stack_entry(&mut game);
        assert!(
            matches!(
                result,
                Err(GameLoopError::ExecutionFailed(
                    ExecutionError::ResourceLimitExceeded {
                        resource: "token creation work",
                        requested: 4,
                        maximum: 3
                    }
                ))
            ),
            "{result:?}"
        );
        assert_eq!(game.stack.len(), 1);
        assert_eq!(game.object(source).unwrap().zone, Zone::Stack);
        assert_eq!(game.player(alice()).unwrap().life, 20);
        assert_eq!(game.next_object_id_counter(), next);
        assert!(game.battlefield.is_empty());
        assert!(game.effect_store.delayed_triggers.is_empty());
        assert!(game.take_pending_trigger_events().is_empty());
    }
}
