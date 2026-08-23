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
    fn supports_simultaneous_player_action(&self) -> bool {
        true
    }

    fn prepare_simultaneous_player_action(
        &self,
        _game: &GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<Box<dyn crate::effects::SimultaneousEffectProposal>, ExecutionError> {
        Ok(Box::new(crate::effects::DeferredPlayerActionProposal {
            effect: crate::effect::Effect::new(self.clone()),
            iterated_player: ctx.iteration.iterated_player,
        }))
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn visit_child_effects(&self, visitor: &mut dyn FnMut(&crate::effect::Effect)) {
        for effect in &self.if_true {
            visitor(effect);
        }
        for effect in &self.if_false {
            visitor(effect);
        }
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

        if outcomes.is_empty() {
            // A false conditional with no else branch did not perform the
            // guarded action. Preserve that distinction for trailing
            // "otherwise" effects and other result predicates.
            return Ok(EffectOutcome::count(0));
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
            if let Some(spec) = effect
                .0
                .get_modal_spec_with_context(game, controller, source)
            {
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
    use crate::effect::{ChoiceCount, Condition};
    use crate::effects::ResolvedTarget;
    use crate::ids::{CardId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::snapshot::ObjectSnapshot;
    use crate::tag::TagKey;
    use crate::target::ObjectFilter;
    use crate::test_prelude::*;
    use crate::types::CardType;
    use crate::zone::Zone;
    use std::collections::HashMap;

    fn make_creature_card(card_id: u32, name: &str, symbol: ManaSymbol) -> crate::card::Card {
        make_creature_card_with_symbols(card_id, name, &[symbol])
    }

    fn make_creature_card_with_symbols(
        card_id: u32,
        name: &str,
        symbols: &[ManaSymbol],
    ) -> crate::card::Card {
        CardBuilder::new(CardId::from_raw(card_id), name)
            .mana_cost(ManaCost::from_pips(
                symbols.iter().copied().map(|symbol| vec![symbol]).collect(),
            ))
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

    fn create_creature_with_symbols(
        game: &mut crate::game_state::GameState,
        name: &str,
        controller: PlayerId,
        symbols: &[ManaSymbol],
    ) -> crate::ids::ObjectId {
        let id = game.new_object_id();
        let card = make_creature_card_with_symbols(id.0 as u32, name, symbols);
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
        let mut game =
            crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let tagged_permanent = create_creature(&mut game, "Guard Marker", alice, ManaSymbol::Red);
        let matching_target =
            create_creature(&mut game, "Matching Attacker", alice, ManaSymbol::Red);
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

        let matching_source_colors = game
            .object(matching_target)
            .expect("matching target")
            .colors();
        let matching_source_types = game
            .object(matching_target)
            .expect("matching target")
            .card_types
            .clone();
        let prevented = game
            .effect_store
            .prevention_effects
            .apply_prevention_to_player(
                alice,
                3,
                true,
                matching_target,
                &matching_source_colors,
                &matching_source_types,
                true,
            );
        assert_eq!(prevented, 0);

        let mut nonmatching_game =
            crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice2 = PlayerId::from_index(0);
        let source2 = nonmatching_game.new_object_id();
        let tagged_permanent2 = create_creature(
            &mut nonmatching_game,
            "Guard Marker",
            alice2,
            ManaSymbol::Red,
        );
        let _matching_target2 = create_creature(
            &mut nonmatching_game,
            "Matching Attacker",
            alice2,
            ManaSymbol::Red,
        );
        let nonmatching_target2 = create_creature(
            &mut nonmatching_game,
            "Nonmatching Attacker",
            alice2,
            ManaSymbol::Blue,
        );
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

    #[test]
    fn conditional_target_color_sets_destroy_only_when_sets_are_equal() {
        let mut same_game =
            crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let same_first = create_creature_with_symbols(
            &mut same_game,
            "First Blue-Red Creature",
            bob,
            &[ManaSymbol::Blue, ManaSymbol::Red],
        );
        let same_second = create_creature_with_symbols(
            &mut same_game,
            "Second Blue-Red Creature",
            bob,
            &[ManaSymbol::Red, ManaSymbol::Blue],
        );
        let same_spec =
            ChooseSpec::target(ChooseSpec::creature()).with_count(ChoiceCount::exactly(2));
        let same_effect = Effect::new(ConditionalEffect::if_only(
            Condition::Not(Box::new(Condition::TargetObjectsHaveDifferentColorSets)),
            vec![Effect::new(crate::effects::DestroyEffect::with_spec(
                same_spec.clone(),
            ))],
        ));
        let mut same_ctx = ExecutionContext::new_default(same_game.new_object_id(), alice)
            .with_targets(vec![
                ResolvedTarget::Object(same_first),
                ResolvedTarget::Object(same_second),
            ])
            .with_target_assignments(vec![crate::game_state::TargetAssignment {
                spec: same_spec,
                range: 0..2,
            }]);

        execute_effect(&mut same_game, &same_effect, &mut same_ctx)
            .expect("equal target color sets should resolve");
        assert!(
            [same_first, same_second].into_iter().all(|id| same_game
                .object(id)
                .is_none_or(|object| object.zone != Zone::Battlefield)),
            "both equal-color-set targets should be destroyed"
        );

        let mut different_game =
            crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let different_first = create_creature_with_symbols(
            &mut different_game,
            "Blue-Red Creature",
            bob,
            &[ManaSymbol::Blue, ManaSymbol::Red],
        );
        let different_second = create_creature_with_symbols(
            &mut different_game,
            "Red Creature",
            bob,
            &[ManaSymbol::Red],
        );
        let different_spec =
            ChooseSpec::target(ChooseSpec::creature()).with_count(ChoiceCount::exactly(2));
        let different_effect = Effect::new(ConditionalEffect::if_only(
            Condition::Not(Box::new(Condition::TargetObjectsHaveDifferentColorSets)),
            vec![Effect::new(crate::effects::DestroyEffect::with_spec(
                different_spec.clone(),
            ))],
        ));
        let mut different_ctx =
            ExecutionContext::new_default(different_game.new_object_id(), alice)
                .with_targets(vec![
                    ResolvedTarget::Object(different_first),
                    ResolvedTarget::Object(different_second),
                ])
                .with_target_assignments(vec![crate::game_state::TargetAssignment {
                    spec: different_spec,
                    range: 0..2,
                }]);

        execute_effect(&mut different_game, &different_effect, &mut different_ctx)
            .expect("different target color sets should resolve");
        assert!(
            [different_first, different_second]
                .into_iter()
                .all(|id| different_game
                    .object(id)
                    .is_some_and(|object| object.zone == Zone::Battlefield)),
            "overlapping but unequal color sets must prevent destruction"
        );
    }
}
