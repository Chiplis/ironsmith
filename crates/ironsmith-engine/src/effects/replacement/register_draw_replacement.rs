//! Timed draw replacements retain their resolving ability's announced bindings.
use crate::effect::{Effect, EffectOutcome};
use crate::effects::helpers::{resolve_player_filter, resolve_player_from_spec};
use crate::effects::{
    ApplyReplacementEffect, EffectExecutor, ExecutionContext, ExecutionError, ResolvedTarget,
};
use crate::events::cards::matchers::WouldDrawCardMatcher;
use crate::game_state::{GameState, TargetAssignment};
use crate::ids::PlayerId;
use crate::replacement::{ReplacementAction, ReplacementEffect};
use crate::snapshot::ObjectSnapshot;
use crate::tag::TagKey;
use crate::target::{ChooseSpec, PlayerFilter};
use std::collections::HashMap;

pub type RegisterDrawReplacementEffect = ironsmith_core::RegisterDrawReplacementEffect<Effect>;

/// A runtime-only closure. The artifact contains the typed registration and
/// program; resolving it captures native object/player identities. Active
/// registrations require native savepoints or accepted-transcript replay.
#[derive(Debug, Clone)]
struct RegisteredDrawProgram {
    effects: Vec<Effect>,
    targets: Vec<ResolvedTarget>,
    target_assignments: Vec<TargetAssignment>,
    x_value: Option<u32>,
    tagged_objects: HashMap<TagKey, Vec<ObjectSnapshot>>,
    tagged_players: HashMap<TagKey, Vec<PlayerId>>,
    source_snapshot: Option<ObjectSnapshot>,
}
impl EffectExecutor for RegisteredDrawProgram {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        // Current characteristics prevail while the source exists. Otherwise
        // use its actual departure/phase-out LKI, not an older activation copy.
        let source_snapshot = game
            .object(ctx.source)
            .filter(|_| !game.is_phased_out(ctx.source))
            .map(|object| ObjectSnapshot::from_object_with_calculated_characteristics(object, game))
            .or_else(|| game.source_last_known_snapshot(ctx.source).cloned())
            .or_else(|| self.source_snapshot.clone());
        let mut child = ExecutionContext::new(ctx.source, ctx.controller, &mut *ctx.decision_maker);
        child.replacement = ctx.replacement.clone();
        // The event owner supplies the actual future drawer. Do not substitute
        // the registering spell's unrelated interrupted event or target slot.
        child.iteration.iterated_player = ctx.iteration.iterated_player;
        child.event_value_amount = ctx.event_value_amount;
        child.triggering_event = ctx.triggering_event.clone();
        child.provenance = ctx.provenance;
        child.cause = ctx.cause.clone();
        child.source_snapshot = source_snapshot;
        child.targets = self.targets.clone();
        child.target_assignments = self.target_assignments.clone();
        child.x_value = self.x_value;
        child.tagged_objects = self.tagged_objects.clone();
        child.tagged_players = self.tagged_players.clone();
        // The closure is only a binding owner. Keep the standard instruction
        // receipts so a later child cannot erase an earlier event's observer.
        // DrawCards still owns rollback of pending choices and errors.
        super::execute_payload::execute_replacement_program(game, &mut child, &self.effects)
    }
    fn visit_child_effects(&self, visit: &mut dyn FnMut(&Effect)) {
        for effect in &self.effects {
            visit(effect);
        }
    }
}

impl EffectExecutor for RegisterDrawReplacementEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let player = if let Some(target) = &self.player_target {
            PlayerFilter::Specific(resolve_player_from_spec(game, target, ctx)?)
        } else if matches!(
            &self.player,
            PlayerFilter::You
                | PlayerFilter::IteratedPlayer
                | PlayerFilter::TargetPlayerOrControllerOfTarget
                | PlayerFilter::Target(_)
                | PlayerFilter::AliasedTarget(_)
                | PlayerFilter::TaggedPlayer(_)
                | PlayerFilter::ChosenPlayer
        ) {
            PlayerFilter::Specific(resolve_player_filter(game, &self.player, ctx)?)
        } else {
            self.player.clone()
        };
        let mut tagged_objects = ctx.tagged_objects.clone();
        crate::effects::helpers::pin_tagged_objects_to_current(game, ctx, &mut tagged_objects);
        let program = RegisteredDrawProgram {
            effects: self.replacement_effects.clone(),
            targets: ctx.targets.clone(),
            target_assignments: ctx.target_assignments.clone(),
            x_value: ctx.x_value,
            tagged_objects,
            tagged_players: ctx.tagged_players.clone(),
            source_snapshot: game
                .object(ctx.source)
                .map(|object| {
                    ObjectSnapshot::from_object_with_calculated_characteristics(object, game)
                })
                .or_else(|| ctx.source_snapshot.clone()),
        };
        let replacement = ReplacementEffect::with_matcher(
            ctx.source,
            ctx.controller,
            WouldDrawCardMatcher::new(player),
            ReplacementAction::Instead(vec![Effect::new(program)]),
        );
        ApplyReplacementEffect {
            effect: replacement,
            mode: self.mode,
        }
        .execute_child(game, ctx)
    }
    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        self.player_target.as_ref()
    }
    fn visit_child_effects(&self, visit: &mut dyn FnMut(&Effect)) {
        for effect in &self.replacement_effects {
            visit(effect);
        }
    }
    fn primary_execution_category(&self) -> crate::effects::EffectExecutionCategory {
        crate::effects::EffectExecutionCategory::ReplacementRegistration
    }
}
