//! Prevent all damage to a specific target effect implementation.

use super::prevention_helpers::register_prevention_shield;
use crate::effect::{Effect, EffectOutcome, Until};
use crate::effects::EffectExecutor;
use crate::effects::helpers::{resolve_objects_from_spec, resolve_players_from_spec};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::prevention::DamageFilter;
use crate::target::ChooseSpec;

/// Effect that prevents all damage to a chosen target for a duration.
#[derive(Debug, Clone, PartialEq)]
pub struct PreventAllDamageToTargetEffect {
    /// What to protect.
    pub target: ChooseSpec,
    /// Duration for the prevention shield.
    pub duration: Until,
    /// Filter for what damage this shield applies to.
    pub damage_filter: DamageFilter,
    /// Effects to run using the amount this shield actually prevented.
    pub follow_up_effects: Vec<Effect>,
    pub source_color_of_your_choice: bool,
}

impl PreventAllDamageToTargetEffect {
    /// Create a new "prevent all damage to target" effect.
    pub fn new(target: ChooseSpec, duration: Until) -> Self {
        Self {
            target,
            duration,
            damage_filter: DamageFilter::all(),
            follow_up_effects: Vec::new(),
            source_color_of_your_choice: false,
        }
    }

    /// Set a damage filter for this prevention effect.
    pub fn with_filter(mut self, filter: DamageFilter) -> Self {
        self.damage_filter = filter;
        self
    }

    /// Execute these effects using the amount this shield prevented.
    pub fn with_follow_up_effects(mut self, effects: Vec<Effect>) -> Self {
        self.follow_up_effects = effects;
        self
    }

    fn execute_bound(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        game.establish_control_transition_boundary()
            .map_err(ExecutionError::ContinuousDiscovery)?;
        let can_protect_player = matches!(self.target.base(),
            ChooseSpec::Player(_) | ChooseSpec::SpecificPlayer(_) | ChooseSpec::SourceController
                | ChooseSpec::SourceOwner | ChooseSpec::EachPlayer(_) | ChooseSpec::AnyTarget
                | ChooseSpec::AnyOtherTarget | ChooseSpec::ObjectOrPlayer(_, _)
                | ChooseSpec::PlayerOrPlaneswalker(_) | ChooseSpec::AttackedPlayerOrPlaneswalker
                | ChooseSpec::Iterated)
            // An inner object iteration shadows the enclosing player binding.
            // Resolving both would either fail or protect an unrelated player.
            && !(matches!(self.target.base(), ChooseSpec::Iterated)
                && ctx.iteration.iterated_object.is_some());
        let objects = match resolve_objects_from_spec(game, &self.target, ctx) {
            Ok(objects) => objects,
            Err(ExecutionError::InvalidTarget)
                if self.target.count().min == 0 && ctx.targets.is_empty() => Vec::new(),
            Err(ExecutionError::InvalidTarget | ExecutionError::UnresolvableValue(_))
                if can_protect_player => Vec::new(),
            Err(error) => return Err(error),
        };
        let players = if can_protect_player
            && !(matches!(self.target.base(), ChooseSpec::AttackedPlayerOrPlaneswalker)
                && !objects.is_empty())
        {
            match resolve_players_from_spec(game, &self.target, ctx) {
                Ok(players) => players,
                Err(ExecutionError::InvalidTarget) if !objects.is_empty() => Vec::new(),
                Err(error) => return Err(error),
            }
        } else { Vec::new() };
        if objects.is_empty() && players.is_empty() {
            return if self.target.count().min == 0 { Ok(EffectOutcome::count(0)) }
                else { Err(ExecutionError::InvalidTarget) };
        }
        let mut filter = self.damage_filter.clone();
        if self.source_color_of_your_choice {
            // This choice belongs to this shield, not a stored choice on the
            // ability's source. Source properties remain live at damage time.
            if filter.from_colors.is_some() {
                return Err(ExecutionError::UnresolvableValue(
                    "chosen-color prevention cannot overwrite a fixed color restriction".into(),
                ));
            }
            use crate::decisions::context::{SelectOptionsContext, SelectableOption};
            let options = crate::color::Color::ALL.iter().enumerate()
                .map(|(index, color)| SelectableOption::new(index, color.name()))
                .collect();
            let choice = SelectOptionsContext::new(
                ctx.controller, Some(ctx.source), "Choose a color", options, 1, 1,
            );
            let selected = ctx.decision_maker.decide_options(game, &choice);
            if ctx.decision_maker.awaiting_choice() {
                return Ok(EffectOutcome::count(0));
            }
            let [index] = selected.as_slice() else {
                return Err(ExecutionError::UnresolvableValue("prevention requires one color".into()));
            };
            let color = crate::color::Color::ALL.get(*index).copied().ok_or_else(|| {
                ExecutionError::UnresolvableValue("invalid prevention color choice".into())
            })?;
            filter.from_colors = Some(vec![color]);
        }
        for protected in objects.into_iter().map(crate::prevention::PreventionTarget::Permanent)
            .chain(players.into_iter().map(crate::prevention::PreventionTarget::Player))
        {
            register_prevention_shield(game, ctx, protected, None, self.duration.clone(),
                filter.clone(), self.follow_up_effects.clone(), ctx.targets.clone(),
                ctx.target_assignments.clone());
        }
        Ok(EffectOutcome::resolved())
    }
}

impl EffectExecutor for PreventAllDamageToTargetEffect {
    fn visit_child_effects(&self, visitor: &mut dyn FnMut(&Effect)) {
        for effect in &self.follow_up_effects {
            visitor(effect);
        }
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        crate::effects::tokens::execute_resource_transaction_atomically(
            game, ctx, |game, ctx| self.execute_bound(game, ctx),
        )
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        Some(&self.target)
    }

    fn target_description(&self) -> &'static str {
        "target to protect from all damage"
    }
}
