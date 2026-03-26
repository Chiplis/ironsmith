//! Exchange control effect implementation.

use crate::continuous::{EffectTarget, Modification};
use crate::effect::{Effect, EffectOutcome, Until};
use crate::effects::helpers::{resolve_objects_for_effect, resolve_single_object_for_effect};
use crate::effects::{ApplyContinuousEffect, EffectExecutor};
use crate::executor::{ExecutionContext, ExecutionError, execute_effect};
use crate::game_state::GameState;
use crate::snapshot::ObjectSnapshot;
use crate::tag::TagKey;
use crate::target::ChooseSpec;
use crate::types::CardType;
use std::collections::HashSet;

const TEMP_IT_TAG: &str = "__it__";

/// Effect that exchanges control of two permanents.
///
/// Creates continuous effects that swap the controllers of two permanents.
///
/// # Fields
///
/// * `permanent1` - First permanent
/// * `permanent2` - Second permanent
///
/// # Example
///
/// ```ignore
/// // Exchange control of two target creatures
/// let effect = ExchangeControlEffect::new(
///     ChooseSpec::creature(),
///     ChooseSpec::creature(),
/// );
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct ExchangeControlEffect {
    /// First permanent to exchange.
    pub permanent1: ChooseSpec,
    /// Second permanent to exchange.
    pub permanent2: ChooseSpec,
    /// Optional targeting constraint that requires the two permanents to share a type.
    pub shared_type: Option<SharedTypeConstraint>,
    /// Optional temporary tag used to resolve the second permanent relative to the first.
    pub permanent1_reference_tag: Option<TagKey>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SharedTypeConstraint {
    CardType,
    PermanentType,
}

impl ExchangeControlEffect {
    /// Create a new exchange control effect.
    pub fn new(permanent1: ChooseSpec, permanent2: ChooseSpec) -> Self {
        Self {
            permanent1,
            permanent2,
            shared_type: None,
            permanent1_reference_tag: None,
        }
    }

    pub fn with_shared_type(mut self, constraint: SharedTypeConstraint) -> Self {
        self.shared_type = Some(constraint);
        self
    }

    pub fn with_permanent1_reference_tag(mut self, tag: impl Into<TagKey>) -> Self {
        self.permanent1_reference_tag = Some(tag.into());
        self
    }

    /// Exchange control of two creatures.
    pub fn creatures() -> Self {
        Self::new(ChooseSpec::creature(), ChooseSpec::creature())
    }

    /// Exchange control of two permanents.
    pub fn permanents() -> Self {
        Self::new(ChooseSpec::permanent(), ChooseSpec::permanent())
    }
}

impl EffectExecutor for ExchangeControlEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let (perm1_id, perm2_id) = if self.permanent1 == self.permanent2 {
            let resolved = match resolve_objects_for_effect(game, ctx, &self.permanent1) {
                Ok(resolved) => resolved,
                Err(ExecutionError::InvalidTarget) => return Ok(EffectOutcome::target_invalid()),
                Err(err) => return Err(err),
            };
            let Some(first) = resolved.first().copied() else {
                return Ok(EffectOutcome::target_invalid());
            };
            let Some(second) = resolved.get(1).copied() else {
                return Ok(EffectOutcome::target_invalid());
            };
            (first, second)
        } else {
            let perm1_id = match resolve_single_object_for_effect(game, ctx, &self.permanent1) {
                Ok(object_id) => object_id,
                Err(ExecutionError::InvalidTarget) => return Ok(EffectOutcome::target_invalid()),
                Err(err) => return Err(err),
            };

            let original_it = ctx.get_tagged_all(TEMP_IT_TAG).cloned();
            let original_reference = self
                .permanent1_reference_tag
                .as_ref()
                .and_then(|tag| ctx.get_tagged_all(tag).cloned());
            if let Some(obj) = game.object(perm1_id) {
                let snapshot = ObjectSnapshot::from_object(obj, game);
                ctx.set_tagged_objects(TEMP_IT_TAG, vec![snapshot.clone()]);
                if let Some(tag) = &self.permanent1_reference_tag {
                    ctx.set_tagged_objects(tag.clone(), vec![snapshot]);
                }
            }

            let perm2_result = resolve_single_object_for_effect(game, ctx, &self.permanent2);

            if let Some(snapshots) = original_it {
                ctx.set_tagged_objects(TEMP_IT_TAG, snapshots);
            } else {
                ctx.clear_object_tag(TEMP_IT_TAG);
            }
            if let Some(tag) = &self.permanent1_reference_tag {
                if let Some(snapshots) = original_reference {
                    ctx.set_tagged_objects(tag.clone(), snapshots);
                } else {
                    ctx.clear_object_tag(tag.as_str());
                }
            }

            let perm2_id = match perm2_result {
                Ok(object_id) => object_id,
                Err(ExecutionError::InvalidTarget) => return Ok(EffectOutcome::target_invalid()),
                Err(err) => return Err(err),
            };
            (perm1_id, perm2_id)
        };

        if let Some(constraint) = self.shared_type {
            let Some(obj1) = game.object(perm1_id) else {
                return Ok(EffectOutcome::target_invalid());
            };
            let Some(obj2) = game.object(perm2_id) else {
                return Ok(EffectOutcome::target_invalid());
            };

            let relevant = |ty: CardType| -> bool {
                match constraint {
                    SharedTypeConstraint::CardType => true,
                    SharedTypeConstraint::PermanentType => matches!(
                        ty,
                        CardType::Artifact
                            | CardType::Creature
                            | CardType::Enchantment
                            | CardType::Land
                            | CardType::Planeswalker
                            | CardType::Battle
                    ),
                }
            };

            let types1: HashSet<CardType> = obj1
                .card_types
                .iter()
                .copied()
                .filter(|ty| relevant(*ty))
                .collect();
            let shares_type = obj2
                .card_types
                .iter()
                .copied()
                .filter(|ty| relevant(*ty))
                .any(|ty| types1.contains(&ty));

            if !shares_type {
                return Ok(EffectOutcome::target_invalid());
            }
        }

        // Get current controllers
        let controller1 = game.object(perm1_id).map(|o| o.controller);
        let controller2 = game.object(perm2_id).map(|o| o.controller);

        if let (Some(c1), Some(c2)) = (controller1, controller2) {
            if c1 == c2 {
                return Ok(EffectOutcome::resolved());
            }

            let effect1 = ApplyContinuousEffect::new(
                EffectTarget::Specific(perm1_id),
                Modification::ChangeController(c2),
                Until::Forever,
            );

            let effect2 = ApplyContinuousEffect::new(
                EffectTarget::Specific(perm2_id),
                Modification::ChangeController(c1),
                Until::Forever,
            );

            let outcomes = vec![
                execute_effect(game, &Effect::new(effect1), ctx)?,
                execute_effect(game, &Effect::new(effect2), ctx)?,
            ];

            Ok(EffectOutcome::aggregate(outcomes))
        } else {
            Ok(EffectOutcome::target_invalid())
        }
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        if self.permanent1.is_target() {
            Some(&self.permanent1)
        } else if self.permanent2.is_target() {
            Some(&self.permanent2)
        } else {
            None
        }
    }

    fn get_target_count(&self) -> Option<crate::effect::ChoiceCount> {
        self.get_target_spec().map(|spec| spec.count())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::executor::ResolvedTarget;
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::object::Object;
    use crate::tag::TagKey;
    use crate::target::{ObjectFilter, TaggedOpbjectRelation};
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

    fn create_artifact(game: &mut GameState, name: &str, controller: PlayerId) -> ObjectId {
        let id = game.new_object_id();
        let card = CardBuilder::new(CardId::from_raw(id.0 as u32), name)
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(2)]]))
            .card_types(vec![CardType::Artifact])
            .build();
        let obj = Object::from_card(id, &card, controller, Zone::Battlefield);
        game.add_object(obj);
        id
    }

    #[test]
    fn test_exchange_control() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let creature1 = create_creature(&mut game, "Alice's Creature", alice);
        let creature2 = create_creature(&mut game, "Bob's Creature", bob);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice).with_targets(vec![
            ResolvedTarget::Object(creature1),
            ResolvedTarget::Object(creature2),
        ]);

        let effect = ExchangeControlEffect::creatures();
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.status, crate::effect::OutcomeStatus::Succeeded);
        // Two continuous effects should be created
        assert_eq!(game.continuous_effects.effects_sorted().len(), 2);
    }

    #[test]
    fn test_exchange_control_insufficient_targets() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let creature1 = create_creature(&mut game, "Creature", alice);
        let source = game.new_object_id();

        // Only one target
        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature1)]);

        let effect = ExchangeControlEffect::creatures();
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.status, crate::effect::OutcomeStatus::TargetInvalid);
    }

    #[test]
    fn test_exchange_control_invalid_first_target() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let creature2 = create_creature(&mut game, "Creature", bob);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice).with_targets(vec![
            ResolvedTarget::Player(alice), // Invalid - should be object
            ResolvedTarget::Object(creature2),
        ]);

        let effect = ExchangeControlEffect::creatures();
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.status, crate::effect::OutcomeStatus::TargetInvalid);
    }

    #[test]
    fn test_exchange_control_invalid_second_target() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let creature1 = create_creature(&mut game, "Creature", alice);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice).with_targets(vec![
            ResolvedTarget::Object(creature1),
            ResolvedTarget::Player(bob), // Invalid - should be object
        ]);

        let effect = ExchangeControlEffect::creatures();
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.status, crate::effect::OutcomeStatus::TargetInvalid);
    }

    #[test]
    fn test_exchange_control_permanents() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let creature1 = create_creature(&mut game, "Permanent 1", alice);
        let creature2 = create_creature(&mut game, "Permanent 2", bob);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice).with_targets(vec![
            ResolvedTarget::Object(creature1),
            ResolvedTarget::Object(creature2),
        ]);

        let effect = ExchangeControlEffect::permanents();
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.status, crate::effect::OutcomeStatus::Succeeded);
    }

    #[test]
    fn test_exchange_control_clone_box() {
        let effect = ExchangeControlEffect::creatures();
        let cloned = effect.clone_box();
        assert!(format!("{:?}", cloned).contains("ExchangeControlEffect"));
    }

    #[test]
    fn test_exchange_control_same_controller_is_no_op() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let creature1 = create_creature(&mut game, "Creature A", alice);
        let creature2 = create_creature(&mut game, "Creature B", alice);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice).with_targets(vec![
            ResolvedTarget::Object(creature1),
            ResolvedTarget::Object(creature2),
        ]);

        let effect = ExchangeControlEffect::creatures();
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.status, crate::effect::OutcomeStatus::Succeeded);
        assert!(
            game.continuous_effects.effects_sorted().is_empty(),
            "same-controller exchange should create no controller-changing effects"
        );
    }

    #[test]
    fn test_exchange_control_source_and_target() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let source = create_creature(&mut game, "Source Permanent", alice);
        let target = create_creature(&mut game, "Target Permanent", bob);

        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(target)]);

        let effect = ExchangeControlEffect::new(
            ChooseSpec::Source,
            ChooseSpec::target(ChooseSpec::creature()),
        );
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.status, crate::effect::OutcomeStatus::Succeeded);
        assert_eq!(game.continuous_effects.effects_sorted().len(), 2);
    }

    #[test]
    fn test_exchange_control_relative_shared_type_uses_first_target_context() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let your_artifact = create_artifact(&mut game, "Your Artifact", alice);
        let opponents_artifact = create_artifact(&mut game, "Opponent Artifact", bob);
        create_creature(&mut game, "Unrelated Creature", bob);

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice).with_targets(vec![
            ResolvedTarget::Object(your_artifact),
            ResolvedTarget::Object(opponents_artifact),
        ]);

        let reference_tag = TagKey::from("exchange_first");
        let first = ChooseSpec::target(ChooseSpec::Object(ObjectFilter::nonland().you_control()));
        let second = ChooseSpec::target(ChooseSpec::Object(
            ObjectFilter::permanent()
                .opponent_controls()
                .match_tagged(reference_tag.clone(), TaggedOpbjectRelation::SharesCardType),
        ));
        let effect = ExchangeControlEffect::new(first, second)
            .with_permanent1_reference_tag(reference_tag)
            .with_shared_type(SharedTypeConstraint::CardType);

        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.status, crate::effect::OutcomeStatus::Succeeded);
        assert_eq!(game.continuous_effects.effects_sorted().len(), 2);
    }

    #[test]
    fn test_exchange_control_relative_shared_type_rejects_mismatched_second_target() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let your_artifact = create_artifact(&mut game, "Your Artifact", alice);
        let opponents_creature = create_creature(&mut game, "Opponent Creature", bob);

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice).with_targets(vec![
            ResolvedTarget::Object(your_artifact),
            ResolvedTarget::Object(opponents_creature),
        ]);

        let reference_tag = TagKey::from("exchange_first");
        let first = ChooseSpec::target(ChooseSpec::Object(ObjectFilter::nonland().you_control()));
        let second = ChooseSpec::target(ChooseSpec::Object(
            ObjectFilter::permanent()
                .opponent_controls()
                .match_tagged(reference_tag.clone(), TaggedOpbjectRelation::SharesCardType),
        ));
        let effect = ExchangeControlEffect::new(first, second)
            .with_permanent1_reference_tag(reference_tag)
            .with_shared_type(SharedTypeConstraint::CardType);

        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.status, crate::effect::OutcomeStatus::TargetInvalid);
        assert!(
            game.continuous_effects.effects_sorted().is_empty(),
            "invalid relative exchange should not create controller-changing effects"
        );
    }
}
