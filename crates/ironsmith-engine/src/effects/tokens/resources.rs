//! Token creation policy and execution resource accounting.
use crate::effects::ExecutionError;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TokenCreationLimits {
    pub max_created_tokens: u32,
    pub max_live_objects: usize,
    pub max_instructions: u32,
    pub max_nesting: u32,
    /// Optional simulation cap. Normal games use the full rule-derived batch quantity.
    pub max_tokens_per_name_per_batch: u32,
}
impl Default for TokenCreationLimits {
    fn default() -> Self {
        Self {
            max_created_tokens: 16_384,
            max_live_objects: 65_536,
            max_instructions: 4_096,
            max_nesting: 64,
            max_tokens_per_name_per_batch: u32::MAX,
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
/// Token counts are game quantities: a proposal past `u32` saturates, and the
/// creation budget (see [`reserve_creation_up_to`]) bounds what is created.
pub(crate) fn token_count(count: u128) -> u32 {
    u32::try_from(count).unwrap_or(u32::MAX)
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
            let instructions = u64::from(state.instructions).saturating_add(1);
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
            let nesting = u64::from(state.nesting).saturating_add(1);
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
            let instructions = u64::from(state.instructions).saturating_add(1);
            let nesting = u64::from(state.nesting).saturating_add(1);
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
            state.nesting = state.nesting.saturating_sub(1);
        }
    }
}
/// Reserve as many of `requested` tokens as the remaining created-token and
/// live-object budgets allow, returning the granted count. Creation past the
/// budget is not an error: the instruction creates what fits and stops.
pub(crate) fn reserve_creation_up_to(
    meter: &SharedTokenCreationMeter,
    requested: u128,
    live_objects: usize,
) -> Result<u128, ExecutionError> {
    let mut state = meter
        .lock()
        .map_err(|_| ExecutionError::InternalError("token resource meter poisoned".into()))?;
    let created_room =
        u128::from(state.limits.max_created_tokens).saturating_sub(u128::from(state.created));
    let live_room = (state.limits.max_live_objects as u128)
        .saturating_sub(live_objects as u128)
        .saturating_sub(u128::from(state.outstanding));
    let granted = requested.min(created_room).min(live_room);
    // `granted` is bounded by `max_created_tokens - created`, so it fits u32.
    let granted_u32 = token_count(granted);
    state.outstanding = state.outstanding.saturating_add(granted_u32);
    state.created = state.created.saturating_add(granted_u32);
    Ok(granted)
}
/// [`reserve_creation_up_to`] through the game's active resource scope. The
/// game-state wrapper reports the exact shortfall of a refused reservation,
/// which bounds the grant; a retry reserves what remains.
pub(crate) fn reserve_token_creation_up_to(
    game: &mut crate::game_state::GameState,
    requested: u128,
) -> Result<u128, ExecutionError> {
    let mut granted = requested;
    loop {
        if granted == 0 {
            return Ok(0);
        }
        match game.reserve_token_creation(granted) {
            Ok(()) => return Ok(granted),
            Err(ExecutionError::ResourceLimitExceeded {
                resource: "token creation work" | "live game objects",
                requested: total,
                maximum,
            }) if total > maximum => {
                granted = granted.saturating_sub(total.saturating_sub(maximum));
            }
            Err(error) => return Err(error),
        }
    }
}
/// Apply the per-name cap to the final replacement event before reserving its objects.
pub(crate) fn reserve_token_batch(
    game: &mut crate::game_state::GameState,
    event: &mut crate::events::CreateTokensEvent,
) -> Result<(), ExecutionError> {
    let cap = game.token_creation_limits().max_tokens_per_name_per_batch;
    let mut count = event.total_count();
    if count > u128::from(cap) {
        event.cap_per_name(cap);
        count = event.total_count();
    }
    game.reserve_token_creation(count)
}
/// Consume one reserved slot for an inserted token. `Ok(false)` means other
/// allocations since the reservation used the remaining live-object budget:
/// the caller stops creating tokens instead of failing the instruction.
pub(crate) fn commit_token_slot_within_budget(
    game: &mut crate::game_state::GameState,
) -> Result<bool, ExecutionError> {
    match game.commit_token_resource_slot() {
        Ok(()) => Ok(true),
        Err(ExecutionError::ResourceLimitExceeded {
            resource: "live game objects",
            ..
        }) => Ok(false),
        Err(error) => Err(error),
    }
}
/// Exact reservation: refuses rather than granting part of `requested`.
pub(crate) fn reserve_creation(
    meter: &SharedTokenCreationMeter,
    requested: u128,
    live_objects: usize,
) -> Result<(), ExecutionError> {
    let mut state = meter
        .lock()
        .map_err(|_| ExecutionError::InternalError("token resource meter poisoned".into()))?;
    let total = u128::from(state.created).saturating_add(requested);
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
    let live = (live_objects as u128)
        .saturating_add(u128::from(state.outstanding))
        .saturating_add(requested);
    if live > state.limits.max_live_objects as u128 {
        return Err(limit(
            "live game objects",
            live,
            state.limits.max_live_objects as u128,
        ));
    }
    // Both checks passed, so `requested` and `total` fit the u32 budgets.
    state.outstanding = state.outstanding.saturating_add(token_count(requested));
    state.created = token_count(total);
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
    let live = (live_objects as u128).saturating_add(u128::from(state.outstanding));
    if live > state.limits.max_live_objects as u128 {
        return Err(limit(
            "live game objects",
            live,
            state.limits.max_live_objects as u128,
        ));
    }
    state.outstanding = state.outstanding.saturating_sub(1);
    Ok(())
}
pub(crate) fn reserve_repetition_work(
    meter: &SharedTokenCreationMeter,
    repetitions: usize,
) -> Result<(), ExecutionError> {
    let mut state = meter
        .lock()
        .map_err(|_| ExecutionError::InternalError("token resource meter poisoned".into()))?;
    let work = u128::from(state.instructions).saturating_add(repetitions as u128);
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
    fn inner_branch_resource_failure_rolls_back_the_parent() {
        let mut game = game();
        let source = game.new_object_id();
        game.set_token_creation_limits(limits(1));
        let mut ctx = ExecutionContext::new_default(source, alice());
        exhausted(crate::effects::execute_effect(&mut game, &Effect::new(AbsorbedChildError), &mut ctx));
        assert_eq!(game.player(alice()).unwrap().life, 20);
        assert!(game.battlefield.is_empty());
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
        // The clamped reservation grants only the one unreserved slot.
        assert_eq!(reserve_creation_up_to(&meter, 2, 2).unwrap(), 1);
        assert_eq!(reserve_creation_up_to(&meter, 1, 2).unwrap(), 0);
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
        assert_eq!(reserve_creation_up_to(&meter, 1, 4).unwrap(), 0);
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
    fn proposal_arithmetic_saturates_instead_of_failing() {
        use crate::events::{CreateTokensEvent, cause::EventCause};
        let event = CreateTokensEvent::with_cause(alice(), u32::MAX, EventCause::effect());
        assert_eq!(event.doubled().unwrap().count, u32::MAX);
        let food = event
            .with_additional_tokens(ironsmith_core::AdditionalTokenKind::Food, 1)
            .unwrap();
        assert_eq!(food.total_count(), u128::from(u32::MAX) + 1);
        assert_eq!(event.with_template(token(), 1).unwrap().total_count(), u128::from(u32::MAX) + 1);
        assert_eq!(food.with_count(u32::MAX).unwrap().total_count(), u128::from(u32::MAX) + 1);
        // "None instead" removes every group, including what saturation hid.
        assert_eq!(food.adjusted_token_total(|_| true, |_| 0).unwrap().total_count(), 0);
        let mut capped = food;
        capped.cap_per_name(512);
        assert_eq!(capped.count, 512);
        assert_eq!(capped.total_count(), 513); // The additional Food has its own allowance.
    }

    #[test]
    fn token_cap_is_shared_by_same_name_groups_only() {
        use crate::events::{CreateTokensEvent, cause::EventCause};
        use ironsmith_core::AdditionalTokenKind::{Food, Treasure};

        let mut event = CreateTokensEvent::with_cause(alice(), 2, EventCause::effect())
            .with_additional_tokens(Food, 2).unwrap()
            .with_additional_tokens(Treasure, 3).unwrap()
            .with_template(crate::cards::tokens::food_token_definition(), 2).unwrap()
            .with_template(crate::cards::tokens::food_token_definition(), 1).unwrap();
        event.cap_per_name(3);

        assert_eq!(event.count, 2);
        assert_eq!(event.additional_tokens, vec![(Food, 2), (Treasure, 3)]);
        assert_eq!(event.additional_templates.len(), 1);
        assert_eq!(event.additional_templates[0].count, 1);
        assert_eq!(event.total_count(), 8);
    }

    #[test]
    fn incubate_past_the_budget_rolls_back_all_repetitions() {
        let mut game = game();
        game.set_token_creation_limits(limits(3));
        let source = game.new_object_id();
        let next = game.next_object_id_counter();
        let mut ctx = ExecutionContext::new_default(source, alice());
        ctx.x_value = Some(73);
        exhausted(IncubateEffect::you(2, 4).execute(&mut game, &mut ctx));
        assert_eq!(game.next_object_id_counter(), next);
        assert!(game.battlefield.is_empty());
        assert_eq!(ctx.x_value, Some(73));
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
    fn nested_entry_payload_requires_the_whole_reservation() {
        use crate::replacement::{ReplacementAction, ReplacementEffect};
        for budget in [2, 3] {
            let mut game = game();
            let source = game.new_object_id();
            game.set_token_creation_limits(limits(budget));
            game.effect_store.replacement_effects.add_one_shot_effect(
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
            let effect = CreateTokenEffect::you(token(), 2)
                .tapped()
                .exile_at_next_end_step();
            let mut ctx = ExecutionContext::new_default(source, alice());
            let result = effect.execute(&mut game, &mut ctx);
            if budget == 2 {
                exhausted(result);
                assert!(game.battlefield.is_empty());
                assert_eq!(game.player(alice()).unwrap().life, 20);
                assert!(game.effect_store.delayed_triggers.is_empty());
                continue;
            }
            result.unwrap();
            assert_eq!(game.battlefield.len(), 3);
            assert_eq!(game.player(alice()).unwrap().life, 23);
            assert_eq!(
                game.effect_store.delayed_triggers.len(),
                1,
                "the outer instruction owns one batch cleanup"
            );
            assert_eq!(game.effect_store.delayed_triggers[0].target_objects.len(), 2,
                "the nested replacement's token must not inherit outer cleanup");
        }
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
            let _ = next;
            let result = if populate {
                crate::effects::PopulateEffect::new(3).execute(&mut game, &mut ctx)
            } else {
                crate::effects::InvestigateEffect::you(3).execute(&mut game, &mut ctx)
            };
            exhausted(result);
            assert_eq!(game.battlefield.len(), before);
        }
    }

    #[test]
    fn dispatcher_creation_past_the_budget_restores_the_prefix() {
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
    }

    #[test]
    fn stack_resolution_shares_budget_across_instructions() {
        use crate::game_loop::resolve_stack_entry;
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
        let result = resolve_stack_entry(&mut game);
        assert!(result.is_err());
        assert!(game.battlefield.is_empty());
        assert_eq!(game.player(alice()).unwrap().life, 20);
        assert_eq!(game.stack.len(), 1);
    }
}
