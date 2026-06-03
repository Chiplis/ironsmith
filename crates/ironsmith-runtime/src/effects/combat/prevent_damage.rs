//! Prevent damage effect implementation.

use super::prevention_helpers::{register_prevention_shield, resolve_prevention_target_from_spec};
use crate::effect::{Effect, EffectOutcome, Until, Value};
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_value;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::prevention::DamageFilter;
use crate::target::ChooseSpec;

/// Effect that prevents N damage to a target.
///
/// Creates a prevention shield that prevents the next N damage to the target
/// for the specified duration.
/// Per Rule 615.6, this creates a shield that tracks remaining prevention.
///
/// # Fields
///
/// * `amount` - Amount of damage to prevent
/// * `target` - What to protect
///
/// # Example
///
/// ```ignore
/// // Prevent the next 3 damage to target creature or player
/// let effect = PreventDamageEffect::new(3, ChooseSpec::AnyTarget, Until::EndOfTurn);
///
/// // Prevent the next 2 damage to you
/// let effect = PreventDamageEffect::to_you(2, Until::EndOfTurn);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct PreventDamageEffect {
    /// Amount of damage to prevent.
    pub amount: Value,
    /// What to protect.
    pub target: ChooseSpec,
    /// Duration for the prevention shield.
    pub duration: Until,
    /// Filter for what damage this shield applies to.
    pub damage_filter: DamageFilter,
    /// Effects to run using the amount this shield actually prevented.
    pub follow_up_effects: Vec<Effect>,
    /// Whether the source is chosen as the effect resolves.
    pub source_of_your_choice: bool,
    /// Protect the controller and permanents they control with one shared shield.
    pub protect_you_and_permanents_you_control: bool,
}

impl PreventDamageEffect {
    /// Create a new prevent damage effect with explicit duration.
    pub fn new(amount: impl Into<Value>, target: ChooseSpec, duration: Until) -> Self {
        Self {
            amount: amount.into(),
            target,
            duration,
            damage_filter: DamageFilter::all(),
            follow_up_effects: Vec::new(),
            source_of_your_choice: false,
            protect_you_and_permanents_you_control: false,
        }
    }

    /// Prevent damage to yourself with explicit duration.
    pub fn to_you(amount: impl Into<Value>, duration: Until) -> Self {
        Self::new(amount, ChooseSpec::SourceController, duration)
    }

    /// Prevent damage to target creature or player with explicit duration.
    pub fn any_target(amount: impl Into<Value>, duration: Until) -> Self {
        Self::new(amount, ChooseSpec::AnyTarget, duration)
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

    /// Protect the controller and permanents they control with one prevention pool.
    pub fn protecting_you_and_permanents_you_control(mut self) -> Self {
        self.protect_you_and_permanents_you_control = true;
        self
    }
}

impl EffectExecutor for PreventDamageEffect {
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
        let amount = resolve_value(game, &self.amount, ctx)?.max(0) as u32;
        let mut damage_filter = self.damage_filter.clone();

        if self.source_of_your_choice {
            let mut candidates = Vec::new();
            candidates.extend(game.stack.iter().map(|entry| entry.object_id));
            candidates.extend(game.battlefield.iter().copied());
            candidates.sort_by_key(|id| id.0);
            candidates.dedup();

            if candidates.is_empty() {
                return Ok(EffectOutcome::resolved());
            }

            let selectable = candidates
                .iter()
                .copied()
                .map(|id| {
                    let name = game
                        .object(id)
                        .map(|object| object.name.clone())
                        .unwrap_or_else(|| format!("object {}", id.0));
                    crate::decisions::context::SelectableObject::new(id, name)
                })
                .collect::<Vec<_>>();
            let select_ctx = crate::decisions::context::SelectObjectsContext::new(
                ctx.controller,
                Some(ctx.source),
                "Choose a source",
                selectable,
                1,
                Some(1),
            );
            let chosen_source = ctx
                .decision_maker
                .decide_objects(game, &select_ctx)
                .into_iter()
                .find(|id| candidates.contains(id));
            if ctx.decision_maker.awaiting_choice() {
                return Ok(EffectOutcome::count(0));
            }
            let Some(chosen_source) = chosen_source else {
                return Ok(EffectOutcome::count(0));
            };
            damage_filter.from_specific_source = Some(chosen_source);
        }

        let protected = if self.protect_you_and_permanents_you_control {
            crate::prevention::PreventionTarget::YouAndPermanentsYouControl
        } else {
            resolve_prevention_target_from_spec(game, &self.target, ctx)?
        };
        register_prevention_shield(
            game,
            ctx,
            protected,
            Some(amount),
            self.duration.clone(),
            damage_filter,
            self.follow_up_effects.clone(),
            ctx.targets.clone(),
        );

        Ok(EffectOutcome::resolved())
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        Some(&self.target)
    }

    fn target_description(&self) -> &'static str {
        "target to protect"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::effects::ResolvedTarget;
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::object::Object;
    use crate::types::CardType;
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    fn make_creature_card(card_id: u32, name: &str) -> crate::card::Card {
        CardBuilder::new(CardId::from_raw(card_id), name)
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(2)]]))
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build()
    }

    fn create_creature(game: &mut GameState, name: &str, controller: PlayerId) -> ObjectId {
        let id = game.new_object_id();
        let card = make_creature_card(id.0 as u32, name);
        let obj = Object::from_card(id, &card, controller, Zone::Battlefield);
        game.add_object(obj);
        id
    }

    #[test]
    fn test_prevent_damage_to_you() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = PreventDamageEffect::to_you(3, Until::EndOfTurn);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.status, crate::effect::OutcomeStatus::Succeeded);
        assert_eq!(game.effect_store.prevention_effects.shields().len(), 1);
    }

    #[test]
    fn test_prevent_damage_to_creature() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let creature = create_creature(&mut game, "Bear", alice);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature)]);

        let effect = PreventDamageEffect::any_target(2, Until::EndOfTurn);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.status, crate::effect::OutcomeStatus::Succeeded);
        assert_eq!(game.effect_store.prevention_effects.shields().len(), 1);
    }

    #[test]
    fn test_prevent_damage_to_player() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Player(bob)]);

        let effect = PreventDamageEffect::any_target(5, Until::EndOfTurn);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.status, crate::effect::OutcomeStatus::Succeeded);
        assert_eq!(game.effect_store.prevention_effects.shields().len(), 1);
    }

    #[test]
    fn test_prevent_damage_variable_amount() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice).with_x(7);
        let effect =
            PreventDamageEffect::new(Value::X, ChooseSpec::SourceController, Until::EndOfTurn);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.status, crate::effect::OutcomeStatus::Succeeded);
    }

    #[test]
    fn test_prevent_damage_source() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let creature = create_creature(&mut game, "Self-protecting", alice);

        let mut ctx = ExecutionContext::new_default(creature, alice);
        let effect = PreventDamageEffect::new(2, ChooseSpec::Source, Until::EndOfTurn);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.status, crate::effect::OutcomeStatus::Succeeded);
    }

    #[test]
    fn test_prevent_damage_clone_box() {
        let effect = PreventDamageEffect::to_you(3, Until::EndOfTurn);
        let cloned = effect.clone_box();
        assert!(format!("{:?}", cloned).contains("PreventDamageEffect"));
    }
}
