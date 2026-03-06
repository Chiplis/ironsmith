//! Return from graveyard to hand effect implementation.

use crate::effect::{EffectOutcome, EffectResult};
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_objects_from_spec;
use crate::event_processor::EventOutcome;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::ids::ObjectId;
use crate::target::ChooseSpec;
use crate::zone::Zone;

use super::apply_zone_change;

/// Effect that returns a target card from a graveyard to its owner's hand.
///
/// This is used for recursion spells like Regrowth, Raise Dead, etc.
///
/// # Fields
///
/// * `target` - Which card to return (resolved from `ChooseSpec`)
///
/// # Example
///
/// ```ignore
/// // Return target creature card from your graveyard to your hand
/// let effect = ReturnFromGraveyardToHandEffect::new(ChooseSpec::creature_card_in_graveyard());
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct ReturnFromGraveyardToHandEffect {
    /// The targeting specification (for UI/validation purposes).
    pub target: ChooseSpec,
    /// Whether the cards are selected at random (text-level semantics).
    pub random: bool,
}

impl ReturnFromGraveyardToHandEffect {
    /// Create a new return from graveyard to hand effect.
    pub fn new(target: ChooseSpec, random: bool) -> Self {
        Self { target, random }
    }

    /// Create an effect targeting any card in a graveyard.
    pub fn any_card() -> Self {
        Self::new(ChooseSpec::card_in_zone(Zone::Graveyard), false)
    }

    fn return_object(
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        object_id: ObjectId,
    ) -> Option<ObjectId> {
        let Some(obj) = game.object(object_id) else {
            return None;
        };
        if obj.zone != Zone::Graveyard {
            return None;
        }

        match apply_zone_change(
            game,
            object_id,
            Zone::Graveyard,
            Zone::Hand,
            &mut ctx.decision_maker,
        ) {
            EventOutcome::Proceed(result) => result.new_object_id,
            EventOutcome::Prevented | EventOutcome::Replaced | EventOutcome::NotApplicable => None,
        }
    }
}

impl EffectExecutor for ReturnFromGraveyardToHandEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let mut returned = Vec::new();

        if self.random {
            let base = self.target.base();
            let ChooseSpec::Object(filter) = base else {
                return Ok(EffectOutcome::from_result(EffectResult::Impossible));
            };
            if filter.zone != Some(Zone::Graveyard) {
                return Ok(EffectOutcome::from_result(EffectResult::Impossible));
            }

            let count = self.target.count();
            let requested = if count.is_dynamic_x() {
                0
            } else {
                count.max.unwrap_or(count.min)
            };
            if requested == 0 {
                return Ok(EffectOutcome::from_result(
                    EffectResult::Objects(Vec::new()),
                ));
            }

            let filter_ctx = ctx.filter_context(game);
            let mut candidates: Vec<_> = game
                .players
                .iter()
                .flat_map(|p| p.graveyard.iter().copied())
                .filter(|id| {
                    game.object(*id)
                        .is_some_and(|obj| filter.matches(obj, &filter_ctx, game))
                })
                .collect();

            game.shuffle_slice(&mut candidates);
            for id in candidates.into_iter().take(requested) {
                if let Some(new_id) = Self::return_object(game, ctx, id) {
                    returned.push(new_id);
                }
            }

            return Ok(EffectOutcome::from_result(EffectResult::Objects(returned)));
        }

        // Non-random: return all resolved object targets that are still in a graveyard.
        let resolved_targets = match resolve_objects_from_spec(game, &self.target, ctx) {
            Ok(targets) => targets,
            Err(_) => return Ok(EffectOutcome::from_result(EffectResult::TargetInvalid)),
        };
        for target_id in resolved_targets {
            if let Some(new_id) = Self::return_object(game, ctx, target_id) {
                returned.push(new_id);
            }
        }

        if returned.is_empty() {
            Ok(EffectOutcome::from_result(EffectResult::TargetInvalid))
        } else {
            Ok(EffectOutcome::from_result(EffectResult::Objects(returned)))
        }
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        if self.random {
            None
        } else if self.target.is_target() {
            Some(&self.target)
        } else {
            None
        }
    }

    fn get_target_count(&self) -> Option<crate::effect::ChoiceCount> {
        if self.random {
            None
        } else if self.target.is_target() {
            Some(self.target.count())
        } else {
            None
        }
    }

    fn target_description(&self) -> &'static str {
        "card in graveyard to return"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::events::zones::matchers::WouldGoToHandMatcher;
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::object::Object;
    use crate::replacement::{ReplacementAction, ReplacementEffect};
    use crate::snapshot::ObjectSnapshot;
    use crate::types::CardType;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    fn make_creature_card(card_id: u32, name: &str) -> crate::card::Card {
        CardBuilder::new(CardId::from_raw(card_id), name)
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(1)],
                vec![ManaSymbol::Green],
            ]))
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build()
    }

    fn create_creature_in_graveyard(game: &mut GameState, name: &str, owner: PlayerId) -> ObjectId {
        let id = game.new_object_id();
        let card = make_creature_card(id.0 as u32, name);
        let obj = Object::from_card(id, &card, owner, Zone::Graveyard);
        game.add_object(obj);
        id
    }

    #[test]
    fn test_return_tagged_target_without_ctx_targets() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let creature_id = create_creature_in_graveyard(&mut game, "Eternal Witness", alice);
        let snapshot = ObjectSnapshot::from_object(game.object(creature_id).unwrap(), &game);

        let mut ctx = ExecutionContext::new_default(source, alice);
        ctx.tag_object("return_target", snapshot);

        let effect =
            ReturnFromGraveyardToHandEffect::new(ChooseSpec::Tagged("return_target".into()), false);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        let EffectResult::Objects(ids) = result.result else {
            panic!("Expected Objects result");
        };
        assert_eq!(ids.len(), 1);
        assert!(game.players[0].hand.contains(&ids[0]));
        assert!(game.players[0].graveyard.is_empty());
    }

    #[test]
    fn test_return_from_graveyard_to_hand_respects_replacement_effects() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let creature_id = create_creature_in_graveyard(&mut game, "Reassembling Skeleton", alice);

        game.replacement_effects
            .add_resolution_effect(ReplacementEffect::with_matcher(
                source,
                alice,
                WouldGoToHandMatcher::you(),
                ReplacementAction::ChangeDestination(Zone::Exile),
            ));

        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect =
            ReturnFromGraveyardToHandEffect::new(ChooseSpec::SpecificObject(creature_id), false);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        let EffectResult::Objects(ids) = result.result else {
            panic!("Expected Objects result");
        };
        assert_eq!(ids.len(), 1);
        assert!(game.players[0].hand.is_empty());
        assert!(game.players[0].graveyard.is_empty());
        assert!(game.exile.contains(&ids[0]));
    }
}
