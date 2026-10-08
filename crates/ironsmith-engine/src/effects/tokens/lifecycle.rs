//! Shared token lifecycle helpers.

use crate::ability::Ability;
use crate::effect::{Effect, EffectOutcome};
use crate::effects::{
    EffectExecutor, EnterAttackingEffect, SacrificeTargetEffect, ScheduleDelayedTriggerEffect,
};
use crate::effects::{ExecutionContext, ExecutionError, ResolvedTarget};
#[cfg(test)]
use crate::events::EnterBattlefieldEvent;
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::static_abilities::StaticAbility;
use crate::target::{ChooseSpec, PlayerFilter};
use crate::triggers::{Trigger, TriggerEvent};
use crate::zone::Zone;

/// Token creation, entry choices and replacement payloads are one instruction.
/// Keep prompt/answer state but restore game and resolution memory on suspension.
pub(crate) fn execute_token_instruction_atomically<'a>(
    game: &mut GameState,
    ctx: &mut ExecutionContext<'a>,
    execute: impl FnOnce(
        &mut GameState,
        &mut ExecutionContext<'a>,
    ) -> Result<crate::effect::EffectOutcome, ExecutionError>,
) -> Result<crate::effect::EffectOutcome, ExecutionError> {
    execute_token_instruction_with_pending_value(
        game,
        ctx,
        || crate::effect::EffectOutcome::with_objects(Vec::new()),
        execute,
    )
}

/// One resource transaction and instruction permit serve either receipt shape.
pub(crate) fn execute_token_instruction_with_pending_value<'a, T>(
    game: &mut GameState,
    ctx: &mut ExecutionContext<'a>,
    pending_value: impl FnOnce() -> T,
    execute: impl FnOnce(&mut GameState, &mut ExecutionContext<'a>) -> Result<T, ExecutionError>,
) -> Result<T, ExecutionError> {
    execute_resource_transaction_with_pending_value(game, ctx, pending_value, |game, ctx| {
        let (_, meter) = game.begin_token_resource_scope();
        let _guard = super::resources::TokenInstructionGuard::enter(meter)?;
        execute(game, ctx)
    })
}

pub(crate) fn execute_resource_transaction_atomically<'a>(
    game: &mut GameState,
    ctx: &mut ExecutionContext<'a>,
    execute: impl FnOnce(
        &mut GameState,
        &mut ExecutionContext<'a>,
    ) -> Result<crate::effect::EffectOutcome, ExecutionError>,
) -> Result<crate::effect::EffectOutcome, ExecutionError> {
    execute_resource_transaction_with_pending_value(
        game,
        ctx,
        || crate::effect::EffectOutcome::with_objects(Vec::new()),
        execute,
    )
}

/// The resource rollback owner is independent of the completed receipt shape.
/// Callers supply their existing neutral result for a suspended operation.
pub(crate) fn execute_resource_transaction_with_pending_value<'a, T>(
    game: &mut GameState,
    ctx: &mut ExecutionContext<'a>,
    pending_value: impl FnOnce() -> T,
    execute: impl FnOnce(&mut GameState, &mut ExecutionContext<'a>) -> Result<T, ExecutionError>,
) -> Result<T, ExecutionError> {
    if ctx.decision_maker.awaiting_choice() {
        return Ok(pending_value());
    }
    game.clear_pending_decision_controllers();
    let (root, meter) = game.begin_token_resource_scope();
    let result =
        crate::effects::composition::execute_transaction(game, ctx, pending_value, |game, ctx| {
            let mut result = match game.token_resource_failure() {
                Some(error) => Err(error),
                None => execute(game, ctx),
            };
            if let Err(error) = &result {
                game.record_token_resource_failure(error);
            }
            if let Some(error) = game.token_resource_failure() {
                result = Err(error);
            }
            result
        });
    game.end_token_resource_scope(root, &meter);
    result
}

/// Retain a child receipt at its execution position relative to authored
/// entry/group events. Neither its observations nor its event identity are
/// flattened away by the token adapter.
pub(crate) fn retain_token_child(
    events: &mut Vec<TriggerEvent>,
    children: &mut Vec<crate::effects::CompletedEffectOutputs>,
    child: crate::effects::CompletedEffectOutputs,
) {
    if !events.is_empty() {
        children.push(crate::effects::CompletedEffectOutputs::aggregate_only(
            EffectOutcome::resolved().with_events(std::mem::take(events)),
        ));
    }
    children.push(child);
}

pub(crate) fn compose_token_original(
    original: EffectOutcome,
    children: Vec<crate::effects::CompletedEffectOutputs>,
) -> crate::effects::CompletedEffectOutputs {
    let primary = original.summary_projection();
    crate::effects::CompletedEffectOutputs::from_children(
        children
            .into_iter()
            .chain([crate::effects::CompletedEffectOutputs::aggregate_only(
                original,
            )]),
        |children| EffectOutcome::aggregate_with_primary_result(primary, children),
    )
}

/// Entry-processing options for newly created tokens.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct TokenEntryOptions {
    pub enters_attacking: bool,
}

impl TokenEntryOptions {
    pub fn new(enters_attacking: bool) -> Self {
        Self { enters_attacking }
    }
}

/// Keep the complete entry receipt while exposing only its original arrival
/// for the owner's authored token links, combat setup and cleanup registration.
/// The owner must finish these receipts after every original token is complete.
pub(crate) fn retain_token_entry_receipt(
    game: &mut GameState,
    provisional: ObjectId,
    entry: crate::game_state::EntryCommitResult,
    published: &mut Vec<crate::effects::PublishedEffectOutputs>,
    receipts: &mut Vec<(
        ObjectId,
        crate::events::processing::PreparedEventOutcome<crate::effects::zones::AppliedZoneChange>,
    )>,
) -> Result<Option<crate::game_state::EntersResult>, ExecutionError> {
    use crate::events::processing::{EventOutcome, PreparedEventOutcome};
    if entry.pending {
        return Err(ExecutionError::InternalError(
            "pending token entry reached original commit owner".into(),
        ));
    }
    crate::effects::PublishedEffectOutputs::append_distinct(published, entry.published_outputs);
    let (original, arrival) = match entry.original {
        EventOutcome::Proceed(result) => {
            let final_zone = game
                .object(result.new_id)
                .map(|object| object.zone)
                .ok_or_else(|| {
                    ExecutionError::InternalError(
                        "token entry arrival missing before authored work".into(),
                    )
                })?;
            let mut ids = game.take_zone_change_results(provisional);
            if ids.is_empty() {
                ids.push(result.new_id);
            }
            game.record_zone_change_results(provisional, ids.clone());
            (
                EventOutcome::Proceed(crate::effects::zones::AppliedZoneChange {
                    final_zone,
                    new_object_id: Some(result.new_id),
                    new_object_ids: ids,
                }),
                Some(result),
            )
        }
        EventOutcome::Prevented => (EventOutcome::Prevented, None),
        EventOutcome::Replaced => (EventOutcome::Replaced, None),
        EventOutcome::NotApplicable => (EventOutcome::NotApplicable, None),
    };
    receipts.push((
        provisional,
        PreparedEventOutcome {
            original,
            programs: entry.programs,
        },
    ));
    Ok(arrival)
}

#[derive(Debug, Clone, Default)]
pub(crate) struct AdditionalTokenInstructions {
    pub enters_tapped: bool,
    pub suppress_aura_attachment_choice: bool,
    pub entry: TokenEntryOptions,
    pub attack_player: Option<PlayerId>,
    pub attack_player_only: bool,
    /// One preselected destination per attempted added token, in group order.
    /// `Some` disables all destination prompts during original commitment.
    pub prepared_attack_targets: Option<Vec<Option<crate::combat_state::AttackTarget>>>,
    pub blocking_attacker: Option<ObjectId>,
    pub initial_counters: Vec<(crate::object::CounterType, u32)>,
    pub cleanup: Option<TokenCleanupOptions>,
    pub linked_exiles: Vec<ObjectId>,
}

/// Commit every added/substituted group as part of its original creation.
/// Update the same event with actual counts; the owner publishes it once.
pub(crate) fn create_replacement_additional_tokens(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    controller_id: PlayerId,
    creation: &mut crate::events::CreateTokensEvent,
    instructions: &AdditionalTokenInstructions,
    events: &mut Vec<TriggerEvent>,
    receipts: &mut Vec<(
        ObjectId,
        crate::events::processing::PreparedEventOutcome<crate::effects::zones::AppliedZoneChange>,
    )>,
    children: &mut Vec<crate::effects::CompletedEffectOutputs>,
    published: &mut Vec<crate::effects::PublishedEffectOutputs>,
) -> Result<Vec<ObjectId>, ExecutionError> {
    use super::create_token_copy::{attack_targets_for_player, choose_attack_target};
    use crate::events::tokens::TokenGroupKey;
    let mut created_ids = Vec::new();
    let mut attack_index = 0usize;
    for key in creation.group_keys() {
        let definition = match key {
            TokenGroupKey::Original => continue,
            TokenGroupKey::Named(index) => crate::events::tokens::additional_token_definition(
                creation.additional_tokens[index].0,
            ),
            TokenGroupKey::Template(index) => {
                creation.additional_templates[index].definition.clone()
            }
        };
        let count = creation.group_count(key) as usize;
        let mut actual = 0u32;
        for _ in 0..count {
            let prepared_attack = instructions
                .prepared_attack_targets
                .as_ref()
                .map(|targets| {
                    targets.get(attack_index).cloned().ok_or_else(|| {
                        ExecutionError::InternalError(
                            "added token lost its prepared attack destination".into(),
                        )
                    })
                })
                .transpose()?;
            attack_index += 1;
            let id = game.new_object_id();
            let mut token = game.object_from_token_definition(id, &definition, controller_id);
            token.zone = Zone::Command;
            let is_creature = token.is_creature();
            game.commit_token_resource_slot()?;
            game.add_object(token);
            let entry = game.move_created_token_with_entry_instructions(
                id,
                ctx.cause.clone(),
                &mut ctx.decision_maker,
                instructions.enters_tapped,
                !instructions.suppress_aura_attachment_choice,
                instructions.initial_counters.clone(),
            )?;
            if ctx.decision_maker.awaiting_choice() {
                return Ok(Vec::new());
            }
            let Some(entry) = retain_token_entry_receipt(game, id, entry, published, receipts)?
            else {
                game.remove_object(id);
                continue;
            };
            let entered = entry.new_id;
            actual += 1;
            created_ids.push(entered);
            for &exiled in &instructions.linked_exiles {
                game.add_exiled_with_source_link(entered, exiled);
            }
            if game
                .object(entered)
                .is_some_and(|object| object.zone == Zone::Battlefield)
            {
                let entry_observation = apply_token_battlefield_entry_with_outputs(
                    game,
                    ctx,
                    entered,
                    controller_id,
                    is_creature,
                    if instructions.prepared_attack_targets.is_some() {
                        TokenEntryOptions::default()
                    } else {
                        instructions.entry
                    },
                    Zone::Command,
                    entry.enters_tapped,
                    events,
                )?;
                retain_token_child(events, children, entry_observation);
                if ctx.decision_maker.awaiting_choice() {
                    return Ok(Vec::new());
                }
                if let Some(target) = prepared_attack {
                    if let Some(target) = target
                        && crate::effects::combat::can_enter_attacking(game, entered)
                    {
                        game.add_entering_attacker(entered, target);
                    }
                } else if let Some(player) = instructions.attack_player
                    && crate::effects::combat::can_enter_attacking(game, entered)
                {
                    let target = if instructions.attack_player_only {
                        game.player(player)
                            .is_some_and(|player| player.is_in_game())
                            .then_some(crate::combat_state::AttackTarget::Player(player))
                    } else {
                        let targets = attack_targets_for_player(game, player);
                        (!targets.is_empty())
                            .then(|| choose_attack_target(game, ctx, player, &targets))
                            .flatten()
                    };
                    if ctx.decision_maker.awaiting_choice() {
                        return Ok(Vec::new());
                    }
                    if let Some(target) = target {
                        game.add_entering_attacker(entered, target);
                    }
                }
                if let Some(attacker) = instructions.blocking_attacker {
                    crate::effects::combat::put_onto_battlefield_blocking(game, entered, attacker);
                }
                if let Some(cleanup) = &instructions.cleanup {
                    let cleanup = schedule_token_cleanup_with_outputs(
                        game,
                        ctx,
                        entered,
                        controller_id,
                        cleanup.clone(),
                    )?;
                    retain_token_child(events, children, cleanup);
                }
                if ctx.decision_maker.awaiting_choice() {
                    return Ok(Vec::new());
                }
            }
        }
        *creation.group_count_mut(key) = actual;
    }
    Ok(created_ids)
}

pub(crate) fn publish_created_token_groups(
    _game: &mut GameState,
    ctx: &ExecutionContext,
    creation: crate::events::CreateTokensEvent,
    reported: &mut Vec<TriggerEvent>,
) {
    if creation.total_count() > 0 {
        let event = TriggerEvent::new_with_provenance(creation, ctx.provenance);
        reported.push(event);
    }
}

/// Apply common post-create entry processing for a token that entered the battlefield.
pub(crate) fn apply_token_battlefield_entry(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    token_id: ObjectId,
    _controller_id: PlayerId,
    _token_is_creature: bool,
    options: TokenEntryOptions,
    from_zone: Zone,
    enters_tapped: bool,
    events: &mut Vec<TriggerEvent>,
) -> Result<EffectOutcome, ExecutionError> {
    apply_token_battlefield_entry_with_outputs(
        game,
        ctx,
        token_id,
        _controller_id,
        _token_is_creature,
        options,
        from_zone,
        enters_tapped,
        events,
    )
    .map(crate::effects::CompletedEffectOutputs::into_outcome)
}

pub(crate) fn apply_token_battlefield_entry_with_outputs(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    token_id: ObjectId,
    _controller_id: PlayerId,
    _token_is_creature: bool,
    options: TokenEntryOptions,
    from_zone: Zone,
    enters_tapped: bool,
    events: &mut Vec<TriggerEvent>,
) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
    let source_stable_id = game
        .object(ctx.source)
        .map(|source| source.stable_id)
        .or_else(|| ctx.source_snapshot.as_ref().map(|source| source.stable_id));
    let token_stable_id = game.object(token_id).map(|token| token.stable_id);
    if let (Some(source_stable_id), Some(token_stable_id)) = (source_stable_id, token_stable_id) {
        game.add_token_created_with_source_link(source_stable_id, token_stable_id);
    }

    // Tapped state was committed from the resolved entry event.
    // Tokens always have summoning sickness.
    game.set_summoning_sick(token_id);

    // Zone-change events are queued by `move_object_with_etb_processing_with_dm`.
    // Emit the explicit ETB event here so enters-tapped/untapped triggers can match.
    events.push(crate::effects::zones::battlefield_entry_observation(
        game,
        token_id,
        from_zone,
        enters_tapped,
        ctx.provenance,
        Vec::new(),
    )?);

    if options.enters_attacking {
        return ctx.with_temp_targets(vec![ResolvedTarget::Object(token_id)], |ctx| {
            let enter_attacking = EnterAttackingEffect::new(ChooseSpec::AnyTarget);
            enter_attacking.execute_child_with_outputs(game, ctx)
        });
    }

    Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
        EffectOutcome::resolved(),
    ))
}

/// Grant a sequence of static abilities to a created token.
pub(crate) fn grant_token_static_abilities(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    token_id: ObjectId,
    static_abilities: &[StaticAbility],
) -> Result<EffectOutcome, ExecutionError> {
    grant_token_static_abilities_with_outputs(game, ctx, token_id, static_abilities)
        .map(crate::effects::CompletedEffectOutputs::into_outcome)
}

pub(crate) fn grant_token_static_abilities_with_outputs(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    token_id: ObjectId,
    static_abilities: &[StaticAbility],
) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
    let mut children = Vec::new();
    for static_ability in static_abilities {
        let outcome = ctx.with_temp_targets(vec![ResolvedTarget::Object(token_id)], |ctx| {
            let grant_effect = crate::effects::GrantObjectAbilityEffect::new(
                Ability::static_ability(static_ability.clone()),
                ChooseSpec::AnyTarget,
            );
            grant_effect.execute_child_with_outputs(game, ctx)
        })?;
        children.push(outcome);
        if ctx.decision_maker.awaiting_choice() {
            return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                EffectOutcome::count(0),
            ));
        }
    }

    Ok(crate::effects::CompletedEffectOutputs::from_children(
        children,
        |children| {
            EffectOutcome::aggregate_with_primary_result(EffectOutcome::resolved(), children)
        },
    ))
}

/// Delayed-cleanup scheduling options for newly created tokens.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct TokenCleanupOptions {
    pub exile_at_end_of_combat: bool,
    pub sacrifice_at_end_of_combat: bool,
    pub sacrifice_at_next_end_step: bool,
    pub exile_at_next_end_step: bool,
    pub next_end_step_player: PlayerFilter,
}

impl TokenCleanupOptions {
    pub fn new(
        exile_at_end_of_combat: bool,
        sacrifice_at_end_of_combat: bool,
        sacrifice_at_next_end_step: bool,
        exile_at_next_end_step: bool,
        next_end_step_player: PlayerFilter,
    ) -> Self {
        Self {
            exile_at_end_of_combat,
            sacrifice_at_end_of_combat,
            sacrifice_at_next_end_step,
            exile_at_next_end_step,
            next_end_step_player,
        }
    }
}

/// Schedule configured delayed cleanup for a token.
pub(crate) fn schedule_token_cleanup(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    token_id: ObjectId,
    controller_id: PlayerId,
    options: TokenCleanupOptions,
) -> Result<EffectOutcome, ExecutionError> {
    schedule_token_cleanup_with_outputs(game, ctx, token_id, controller_id, options)
        .map(crate::effects::CompletedEffectOutputs::into_outcome)
}

pub(crate) fn schedule_token_cleanup_with_outputs(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    token_id: ObjectId,
    controller_id: PlayerId,
    options: TokenCleanupOptions,
) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
    let mut children = Vec::new();
    if options.exile_at_end_of_combat {
        children.push(schedule_token_delayed_effect_with_outputs(
            game,
            ctx,
            token_id,
            controller_id,
            Trigger::end_of_combat(),
            vec![Effect::exile(ChooseSpec::SpecificObject(token_id))],
        )?);
        if ctx.decision_maker.awaiting_choice() {
            return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                EffectOutcome::count(0),
            ));
        }
    }

    if options.sacrifice_at_end_of_combat {
        children.push(schedule_token_delayed_effect_with_outputs(
            game,
            ctx,
            token_id,
            controller_id,
            Trigger::end_of_combat(),
            vec![Effect::new(SacrificeTargetEffect::new(ChooseSpec::All(
                crate::target::ObjectFilter::specific(token_id).you_control(),
            )))],
        )?);
        if ctx.decision_maker.awaiting_choice() {
            return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                EffectOutcome::count(0),
            ));
        }
    }

    if options.sacrifice_at_next_end_step {
        children.push(schedule_token_delayed_effect_with_outputs(
            game,
            ctx,
            token_id,
            controller_id,
            Trigger::beginning_of_end_step(options.next_end_step_player.clone()),
            vec![Effect::new(SacrificeTargetEffect::new(ChooseSpec::All(
                crate::target::ObjectFilter::specific(token_id).you_control(),
            )))],
        )?);
        if ctx.decision_maker.awaiting_choice() {
            return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                EffectOutcome::count(0),
            ));
        }
    }

    if options.exile_at_next_end_step {
        children.push(schedule_token_delayed_effect_with_outputs(
            game,
            ctx,
            token_id,
            controller_id,
            Trigger::beginning_of_end_step(options.next_end_step_player.clone()),
            vec![Effect::exile(ChooseSpec::SpecificObject(token_id))],
        )?);
        if ctx.decision_maker.awaiting_choice() {
            return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                EffectOutcome::count(0),
            ));
        }
    }

    Ok(crate::effects::CompletedEffectOutputs::from_children(
        children,
        |children| {
            EffectOutcome::aggregate_with_primary_result(EffectOutcome::resolved(), children)
        },
    ))
}

fn schedule_token_delayed_effect_with_outputs(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    token_id: ObjectId,
    controller_id: PlayerId,
    trigger: Trigger,
    effects: Vec<Effect>,
) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
    let schedule = ScheduleDelayedTriggerEffect::new(
        trigger,
        effects,
        true,
        vec![token_id],
        PlayerFilter::Specific(controller_id),
    );
    schedule.execute_child_with_outputs(game, ctx)
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::AbilityKind;
    use crate::color::ColorSet;
    use crate::combat_state::{AttackTarget, AttackerInfo, CombatState};
    use crate::effects::ExecutionContext;
    use crate::events::EventKind;
    use crate::ids::PlayerId;
    use crate::object::Object;
    use crate::types::CardType;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    #[test]
    fn test_schedule_token_cleanup_no_flags_is_noop() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let token_id = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        schedule_token_cleanup(
            &mut game,
            &mut ctx,
            token_id,
            alice,
            TokenCleanupOptions::default(),
        )
        .unwrap();

        assert_eq!(game.effect_store.delayed_triggers.len(), 0);
    }

    #[test]
    fn test_schedule_token_cleanup_exile_at_end_of_combat() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.new_object_id();
        let token_id = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        schedule_token_cleanup(
            &mut game,
            &mut ctx,
            token_id,
            bob,
            TokenCleanupOptions::new(true, false, false, false, PlayerFilter::Any),
        )
        .unwrap();

        assert_eq!(game.effect_store.delayed_triggers.len(), 1);
        let delayed = &game.effect_store.delayed_triggers[0];
        assert_eq!(
            delayed.trigger.display(),
            Trigger::end_of_combat().display()
        );
        assert!(delayed.one_shot);
        assert_eq!(delayed.target_objects, vec![token_id]);
        assert_eq!(delayed.controller, bob);
    }

    #[test]
    fn test_schedule_token_cleanup_all_flags() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let token_id = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        schedule_token_cleanup(
            &mut game,
            &mut ctx,
            token_id,
            alice,
            TokenCleanupOptions::new(true, true, true, true, PlayerFilter::Any),
        )
        .unwrap();

        assert_eq!(game.effect_store.delayed_triggers.len(), 4);
        let end_of_combat_display = Trigger::end_of_combat().display();
        let end_step_display = Trigger::beginning_of_end_step(PlayerFilter::Any).display();
        let end_of_combat_count = game
            .effect_store
            .delayed_triggers
            .iter()
            .filter(|delayed| delayed.trigger.display() == end_of_combat_display)
            .count();
        let end_step_count = game
            .effect_store
            .delayed_triggers
            .iter()
            .filter(|delayed| delayed.trigger.display() == end_step_display)
            .count();
        assert_eq!(end_of_combat_count, 2);
        assert_eq!(end_step_count, 2);
        for delayed in &game.effect_store.delayed_triggers {
            assert!(delayed.one_shot);
            assert_eq!(delayed.target_objects, vec![token_id]);
            assert_eq!(delayed.controller, alice);
        }
    }

    #[test]
    fn test_schedule_token_cleanup_uses_your_next_end_step_filter() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let token_id = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        schedule_token_cleanup(
            &mut game,
            &mut ctx,
            token_id,
            alice,
            TokenCleanupOptions::new(false, false, true, false, PlayerFilter::You),
        )
        .unwrap();

        assert_eq!(game.effect_store.delayed_triggers.len(), 1);
        let delayed = &game.effect_store.delayed_triggers[0];
        assert_eq!(
            delayed.trigger,
            Trigger::beginning_of_end_step(PlayerFilter::You)
        );
    }

    #[test]
    fn test_apply_token_battlefield_entry_preserves_committed_state_and_events() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let token_id = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);
        let mut events = Vec::new();

        game.tap(token_id); // The entry commit has already applied the final state.

        apply_token_battlefield_entry(
            &mut game,
            &mut ctx,
            token_id,
            alice,
            true,
            TokenEntryOptions::new(false),
            Zone::Command,
            true,
            &mut events,
        )
        .unwrap();

        assert!(game.is_tapped(token_id));
        assert!(game.is_summoning_sick(token_id));
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].kind(), EventKind::EnterBattlefield);
        let etb = events[0]
            .downcast::<EnterBattlefieldEvent>()
            .expect("expected EnterBattlefieldEvent");
        assert!(etb.enters_tapped);
    }

    #[test]
    fn test_apply_token_battlefield_entry_enters_attacking() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.new_object_id();
        let token_id = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);
        let mut events = Vec::new();
        game.add_object(Object::new_token(
            token_id,
            alice,
            "Token".to_string(),
            vec![CardType::Creature],
            Vec::new(),
            Some(1),
            Some(1),
            ColorSet::default(),
        ));

        game.combat = Some(CombatState {
            attackers: vec![AttackerInfo {
                creature: source,
                target: AttackTarget::Player(bob),
            }],
            ..CombatState::default()
        });
        game.turn.phase = crate::game_state::Phase::Combat;

        apply_token_battlefield_entry(
            &mut game,
            &mut ctx,
            token_id,
            alice,
            true,
            TokenEntryOptions::new(true),
            Zone::Command,
            false,
            &mut events,
        )
        .unwrap();

        let combat = game.combat.as_ref().expect("combat should exist");
        assert!(
            combat.attackers.iter().any(|attacker| {
                attacker.creature == token_id && attacker.target == AttackTarget::Player(bob)
            }),
            "token should enter attacking same target as source"
        );
    }

    #[test]
    fn test_grant_token_static_abilities() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let token_id = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);
        let token = Object::new_token(
            token_id,
            alice,
            "Token".to_string(),
            vec![CardType::Creature],
            Vec::new(),
            Some(1),
            Some(1),
            ColorSet::default(),
        );
        game.add_object(token);

        grant_token_static_abilities(
            &mut game,
            &mut ctx,
            token_id,
            &[StaticAbility::haste(), StaticAbility::flying()],
        )
        .unwrap();

        let token = game.object(token_id).expect("token should exist");
        let has_haste = token.abilities.iter().any(|ability| {
            if let AbilityKind::Static(static_ability) = &ability.kind {
                static_ability.has_haste()
            } else {
                false
            }
        });
        let has_flying = token.abilities.iter().any(|ability| {
            if let AbilityKind::Static(static_ability) = &ability.kind {
                static_ability.has_flying()
            } else {
                false
            }
        });
        assert!(has_haste, "token should gain haste");
        assert!(has_flying, "token should gain flying");
    }
}

struct TokenInstructionCompletion {
    instruction: Option<super::resources::TokenInstructionPermit>,
    entries: Option<
        Vec<(
            ObjectId,
            crate::events::processing::PreparedEventOutcome<
                crate::effects::zones::AppliedZoneChange,
            >,
        )>,
    >,
    frozen: Option<crate::effects::zones::FrozenZoneChangeReceipts>,
    programs: Vec<crate::events::processing::PreparedReplacementProgram>,
    haste_recipients: Vec<ObjectId>,
}
impl crate::effects::SimultaneousEffectCompletion for TokenInstructionCompletion {
    fn original_phase_status(&self) -> crate::effects::OriginalPhaseStatus {
        // Every producer commits entries, initial grants and cleanup before
        // constructing this owner. Frozen entry programs and creation programs
        // are additions; no physical token original is retained here.
        crate::effects::OriginalPhaseStatus::Complete
    }

    fn freeze(&mut self, game: &mut GameState) -> Result<(), ExecutionError> {
        let entries = self
            .entries
            .take()
            .ok_or_else(|| ExecutionError::InternalError("token entries already frozen".into()))?;
        self.frozen = Some(crate::effects::zones::freeze_zone_change_receipts(
            game, entries,
        ));
        Ok(())
    }
    fn complete(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: EffectOutcome,
    ) -> Result<EffectOutcome, ExecutionError> {
        self.complete_with_outputs(game, ctx, original)
            .map(crate::effects::CompletedEffectOutputs::into_outcome)
    }
    fn complete_with_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: EffectOutcome,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        let _phase = self
            .instruction
            .as_ref()
            .map(|permit| permit.enter_phase())
            .transpose()?;
        let frozen = self.frozen.ok_or_else(|| {
            ExecutionError::InternalError("token completion requires the original batch".into())
        })?;
        let outputs = crate::effects::zones::finish_zone_change_receipts_frozen_with_outputs(
            game, ctx, original, frozen,
        )?;
        if ctx.decision_maker.awaiting_choice() {
            return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                EffectOutcome::count(0),
            ));
        }
        let programs = self.programs;
        let mut outputs =
            crate::effects::replacement::complete_replacement_programs_with_original_outputs(
                game,
                ctx,
                outputs,
                |game, ctx, original| {
                    crate::effects::replacement::complete_deferred_replacement_programs(
                        game, ctx, original, programs,
                    )
                },
            )?;
        if ctx.decision_maker.awaiting_choice() {
            return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                EffectOutcome::count(0),
            ));
        }

        for id in self.haste_recipients {
            if game
                .object(id)
                .is_some_and(|object| object.zone == Zone::Battlefield)
                && !game.is_phased_out(id)
            {
                let child = crate::effects::ApplyContinuousEffect::new(
                    crate::continuous::EffectTarget::Specific(id),
                    crate::continuous::Modification::AddAbility(StaticAbility::haste()),
                    crate::effect::Until::Forever,
                )
                .execute_child_with_outputs(game, ctx)?;
                if ctx.decision_maker.awaiting_choice() {
                    return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::count(0),
                    ));
                }
                let aggregate = EffectOutcome::aggregate_with_primary_result(
                    outputs.outcome.clone(),
                    [child.outcome.clone()],
                );
                outputs.retain_owned_child(child);
                outputs = outputs.project_aggregate(aggregate);
            }
        }
        Ok(outputs)
    }
}

/// Creation and copy-template adapters share original-entry freezing,
/// resource permits, and deferred creation-program completion.
pub(crate) fn token_instruction_completion(
    instruction: Option<super::resources::TokenInstructionPermit>,
    entries: Vec<(
        ObjectId,
        crate::events::processing::PreparedEventOutcome<crate::effects::zones::AppliedZoneChange>,
    )>,
    programs: Vec<crate::events::processing::PreparedReplacementProgram>,
) -> Box<dyn crate::effects::SimultaneousEffectCompletion> {
    token_instruction_completion_with_haste(instruction, entries, programs, Vec::new())
}

pub(crate) fn token_instruction_completion_with_haste(
    instruction: Option<super::resources::TokenInstructionPermit>,
    entries: Vec<(
        ObjectId,
        crate::events::processing::PreparedEventOutcome<crate::effects::zones::AppliedZoneChange>,
    )>,
    programs: Vec<crate::events::processing::PreparedReplacementProgram>,
    haste_recipients: Vec<ObjectId>,
) -> Box<dyn crate::effects::SimultaneousEffectCompletion> {
    Box::new(TokenInstructionCompletion {
        instruction,
        entries: Some(entries),
        frozen: None,
        programs,
        haste_recipients,
    })
}
