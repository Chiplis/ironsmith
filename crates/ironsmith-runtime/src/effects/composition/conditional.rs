//! Conditional effect implementation.

use crate::effect::{Condition, EffectOutcome};
use crate::effects::{EffectExecutor, ModalSpec};
use crate::effects::{ExecutionContext, ExecutionError, execute_effect};
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::target::ChooseSpec;
pub type ConditionalEffect = ironsmith_core::ConditionalEffect<crate::effect::Effect>;

/// Effect that branches based on game state conditions.
///
/// Unlike `If` which checks the result of a prior effect, `Conditional`
/// evaluates game state conditions like "if you control a creature" or
/// "if your life total is 10 or less".
///
/// # Fields
///
/// * `condition` - The game state condition to check
/// * `if_true` - Effects to execute if condition is true
/// * `if_false` - Effects to execute if condition is false
///
/// # Example
///
/// ```ignore
/// // If you control a creature, draw a card. Otherwise, gain 2 life.
/// let effect = ConditionalEffect::new(
///     Condition::YouControl(ObjectFilter::creature()),
///     vec![Effect::draw(1)],
///     vec![Effect::gain_life(2)],
/// );
/// ```
impl EffectExecutor for ConditionalEffect {
    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let result = evaluate_condition(game, &self.condition, ctx)?;

        let effects_to_execute = if result {
            &self.if_true
        } else {
            &self.if_false
        };

        let mut outcomes = Vec::new();
        for effect in effects_to_execute {
            outcomes.push(execute_effect(game, effect, ctx)?);
        }

        Ok(EffectOutcome::aggregate(outcomes))
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        super::target_metadata::first_target_spec(&[&self.if_true, &self.if_false])
    }

    fn decision_related_object_specs(&self) -> Vec<ChooseSpec> {
        super::target_metadata::related_object_specs(&[&self.if_true, &self.if_false])
    }

    fn target_description(&self) -> &'static str {
        super::target_metadata::first_target_description(&[&self.if_true, &self.if_false], "target")
    }

    fn get_target_count(&self) -> Option<crate::effect::ChoiceCount> {
        super::target_metadata::first_target_count(&[&self.if_true, &self.if_false])
    }

    fn get_modal_spec_with_context(
        &self,
        game: &GameState,
        controller: PlayerId,
        source: ObjectId,
    ) -> Option<ModalSpec> {
        // Evaluate the condition at cast time to determine which branch to use
        let condition_result = evaluate_condition_simple(game, &self.condition, controller, source);

        // Search the appropriate branch for modal specs
        let effects_to_search = if condition_result {
            &self.if_true
        } else {
            &self.if_false
        };

        // Recursively search through the effects in this branch
        for effect in effects_to_search {
            // First try the context-aware version
            if let Some(spec) = effect
                .0
                .get_modal_spec_with_context(game, controller, source)
            {
                return Some(spec);
            }
            // Fall back to the simple version
            if let Some(spec) = effect.0.get_modal_spec() {
                return Some(spec);
            }
        }

        None
    }
}

fn evaluate_condition_simple(
    game: &GameState,
    condition: &Condition,
    controller: PlayerId,
    source: ObjectId,
) -> bool {
    crate::condition_eval::evaluate_condition_cast_time(game, condition, controller, source)
}

fn evaluate_condition(
    game: &GameState,
    condition: &Condition,
    ctx: &ExecutionContext,
) -> Result<bool, ExecutionError> {
    crate::condition_eval::evaluate_condition_resolution(game, condition, ctx)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::effect::Condition;
    use crate::effects::ResolvedTarget;
    use crate::ids::{CardId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::snapshot::ObjectSnapshot;
    use crate::tag::TagKey;
    use crate::target::ObjectFilter;
    use crate::types::CardType;
    use crate::zone::Zone;
    use crate::test_prelude::*;
    use std::collections::HashMap;

    fn make_creature_card(card_id: u32, name: &str, symbol: ManaSymbol) -> crate::card::Card {
        CardBuilder::new(CardId::from_raw(card_id), name)
            .mana_cost(ManaCost::from_pips(vec![vec![symbol]]))
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build()
    }

    fn create_creature(
        game: &mut crate::game_state::GameState,
        name: &str,
        controller: PlayerId,
        symbol: ManaSymbol,
    ) -> crate::ids::ObjectId {
        let id = game.new_object_id();
        let card = make_creature_card(id.0 as u32, name, symbol);
        let obj = crate::object::Object::from_card(id, &card, controller, Zone::Battlefield);
        game.add_object(obj);
        id
    }

    #[test]
    fn conditional_forwards_inner_target_spec_from_if_true() {
        let effect = ConditionalEffect::if_only(
            Condition::YourTurn,
            vec![Effect::counter(ChooseSpec::target_spell())],
        );

        assert!(effect.get_target_spec().is_some());
        assert_eq!(effect.target_description(), "spell to counter");
    }

    #[test]
    fn conditional_forwards_inner_target_spec_from_if_false() {
        let effect = ConditionalEffect::new(
            Condition::YourTurn,
            vec![Effect::draw(1)],
            vec![Effect::counter(ChooseSpec::target_spell())],
        );

        assert!(effect.get_target_spec().is_some());
        assert_eq!(effect.target_description(), "spell to counter");
    }

    #[test]
    fn conditional_shares_color_with_tagged_target_gates_combat_prevention() {
        let mut game = crate::game_state::GameState::new(
            vec!["Alice".to_string(), "Bob".to_string()],
            20,
        );
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let tagged_permanent = create_creature(&mut game, "Guard Marker", alice, ManaSymbol::Red);
        let matching_target = create_creature(&mut game, "Matching Attacker", alice, ManaSymbol::Red);
        let tagged_snapshot = ObjectSnapshot::from_object(
            game.object(tagged_permanent).expect("tagged permanent"),
            &game,
        );
        let matching_tags: HashMap<TagKey, Vec<ObjectSnapshot>> =
            HashMap::from([(TagKey::from("it"), vec![tagged_snapshot.clone()])]);
        let mut matching_ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(matching_target)])
            .with_tagged_objects(matching_tags);

        let effect = Effect::new(ConditionalEffect::if_only(
            Condition::TargetMatches(
                ObjectFilter::creature().shares_color_with_tagged(TagKey::from("it")),
            ),
            vec![Effect::prevent_all_combat_damage_from(
                ChooseSpec::target_creature(),
                crate::effect::Until::EndOfTurn,
            )],
        ));

        execute_effect(&mut game, &effect, &mut matching_ctx)
            .expect("matching target should resolve");
        assert_eq!(game.effect_store.prevention_effects.shields().len(), 1);

        let matching_source_colors = game.object(matching_target).expect("matching target").colors();
        let matching_source_types = game
            .object(matching_target)
            .expect("matching target")
            .card_types
            .clone();
        let prevented = game.effect_store.prevention_effects.apply_prevention_to_player(
            alice,
            3,
            true,
            matching_target,
            &matching_source_colors,
            &matching_source_types,
            true,
        );
        assert_eq!(prevented, 0);

        let mut nonmatching_game = crate::game_state::GameState::new(
            vec!["Alice".to_string(), "Bob".to_string()],
            20,
        );
        let alice2 = PlayerId::from_index(0);
        let source2 = nonmatching_game.new_object_id();
        let tagged_permanent2 =
            create_creature(&mut nonmatching_game, "Guard Marker", alice2, ManaSymbol::Red);
        let _matching_target2 =
            create_creature(&mut nonmatching_game, "Matching Attacker", alice2, ManaSymbol::Red);
        let nonmatching_target2 =
            create_creature(&mut nonmatching_game, "Nonmatching Attacker", alice2, ManaSymbol::Blue);
        let tagged_snapshot2 = ObjectSnapshot::from_object(
            nonmatching_game
                .object(tagged_permanent2)
                .expect("tagged permanent"),
            &nonmatching_game,
        );
        let nonmatching_tags2: HashMap<TagKey, Vec<ObjectSnapshot>> =
            HashMap::from([(TagKey::from("it"), vec![tagged_snapshot2])]);
        let mut nonmatching_ctx = ExecutionContext::new_default(source2, alice2)
            .with_targets(vec![ResolvedTarget::Object(nonmatching_target2)])
            .with_tagged_objects(nonmatching_tags2);

        execute_effect(&mut nonmatching_game, &effect, &mut nonmatching_ctx)
            .expect("nonmatching target should resolve");
        assert!(
            nonmatching_game
                .effect_store
                .prevention_effects
                .shields()
                .is_empty(),
            "expected no shield for nonmatching target"
        );
    }
}
