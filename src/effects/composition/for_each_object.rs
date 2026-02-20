//! ForEach effect implementation.

use std::collections::HashSet;

use crate::effect::{Effect, EffectOutcome};
use crate::effects::EffectExecutor;
use crate::executor::{ExecutionContext, ExecutionError, execute_effect};
use crate::game_state::GameState;
use crate::target::{ObjectFilter, TaggedOpbjectRelation};

/// Effect that applies effects once for each object matching a filter.
///
/// # Fields
///
/// * `filter` - Filter for which objects to iterate over
/// * `effects` - Effects to execute for each matching object
///
/// # Example
///
/// ```ignore
/// // For each creature you control, gain 1 life
/// let effect = ForEachEffect::new(
///     ObjectFilter::creature().you_control(),
///     vec![Effect::gain_life(1)],
/// );
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct ForEachObject {
    /// Filter for which objects to iterate over.
    pub filter: ObjectFilter,
    /// Effects to execute for each matching object.
    pub effects: Vec<Effect>,
}

impl ForEachObject {
    /// Create a new ForEach effect.
    pub fn new(filter: ObjectFilter, effects: Vec<Effect>) -> Self {
        Self { filter, effects }
    }
}

impl EffectExecutor for ForEachObject {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let filter_ctx = ctx.filter_context(game);

        // For "for each ... revealed/exiled/... this way" patterns, the filter can
        // reference tagged cards outside the battlefield.
        let has_only_is_tagged_constraints = !self.filter.tagged_constraints.is_empty()
            && self
                .filter
                .tagged_constraints
                .iter()
                .all(|constraint| {
                    constraint.relation == TaggedOpbjectRelation::IsTaggedObject
                });

        let candidate_ids: Vec<_> = if has_only_is_tagged_constraints {
            let mut seen = HashSet::new();
            let mut ids = Vec::new();
            for constraint in &self.filter.tagged_constraints {
                let Some(snapshots) = ctx.get_tagged_all(&constraint.tag) else {
                    continue;
                };
                for snapshot in snapshots {
                    if seen.insert(snapshot.object_id) {
                        ids.push(snapshot.object_id);
                    }
                }
            }
            ids
        } else {
            game.battlefield.clone()
        };

        let matching: Vec<_> = candidate_ids
            .into_iter()
            .filter_map(|id| game.object(id).map(|obj| (id, obj)))
            .filter(|(_, obj)| self.filter.matches(obj, &filter_ctx, game))
            .map(|(id, _)| id)
            .collect();

        let mut outcomes = Vec::new();

        // Execute the effects once for each matching object and expose that object via
        // ctx.iterated_object for inner effects using ChooseSpec::Iterated.
        for object_id in &matching {
            ctx.with_temp_iterated_object(Some(*object_id), |ctx| {
                for effect in &self.effects {
                    outcomes.push(execute_effect(game, effect, ctx)?);
                }
                Ok::<(), ExecutionError>(())
            })?;
        }

        Ok(EffectOutcome::aggregate(outcomes))
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::effect::EffectResult;
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::object::CounterType;
    use crate::object::Object;
    use crate::snapshot::ObjectSnapshot;
    use crate::tag::TagKey;
    use crate::target::{ChooseSpec, TaggedObjectConstraint};
    use crate::types::CardType;
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    fn create_creature(game: &mut GameState, name: &str, controller: PlayerId) -> ObjectId {
        let card = CardBuilder::new(CardId::new(), name)
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(2)]]))
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        let id = game.new_object_id();
        let obj = Object::from_card(id, &card, controller, Zone::Battlefield);
        game.add_object(obj);
        id
    }

    fn create_library_card(
        game: &mut GameState,
        name: &str,
        controller: PlayerId,
        card_types: Vec<CardType>,
    ) -> ObjectId {
        let card = CardBuilder::new(CardId::new(), name)
            .card_types(card_types)
            .build();
        let id = game.new_object_id();
        let obj = Object::from_card(id, &card, controller, Zone::Library);
        game.add_object(obj);
        id
    }

    #[test]
    fn test_for_each_no_matches() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let initial_life = game.player(alice).unwrap().life;

        // No creatures on battlefield
        let effect = ForEachObject::new(ObjectFilter::creature(), vec![Effect::gain_life(1)]);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        // Empty aggregate returns Resolved (no effects executed)
        assert_eq!(result.result, EffectResult::Resolved);
        assert_eq!(game.player(alice).unwrap().life, initial_life);
    }

    #[test]
    fn test_for_each_multiple_matches() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create 3 creatures
        create_creature(&mut game, "Bear 1", alice);
        create_creature(&mut game, "Bear 2", alice);
        create_creature(&mut game, "Bear 3", alice);

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let initial_life = game.player(alice).unwrap().life;

        let effect = ForEachObject::new(ObjectFilter::creature(), vec![Effect::gain_life(1)]);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Count(3));
        // Gained 1 life for each creature (3 total)
        assert_eq!(game.player(alice).unwrap().life, initial_life + 3);
    }

    #[test]
    fn test_for_each_filtered() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Create 2 creatures for Alice, 1 for Bob
        create_creature(&mut game, "Alice Bear 1", alice);
        create_creature(&mut game, "Alice Bear 2", alice);
        create_creature(&mut game, "Bob Bear", bob);

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let initial_life = game.player(alice).unwrap().life;

        // Only count creatures Alice controls
        let effect = ForEachObject::new(
            ObjectFilter::creature().you_control(),
            vec![Effect::gain_life(1)],
        );
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Count(2));
        assert_eq!(game.player(alice).unwrap().life, initial_life + 2);
    }

    #[test]
    fn test_for_each_clone_box() {
        let effect = ForEachObject::new(ObjectFilter::creature(), vec![Effect::gain_life(1)]);
        let cloned = effect.clone_box();
        assert!(format!("{:?}", cloned).contains("ForEachObject"));
    }

    #[test]
    fn test_for_each_sets_iterated_object_for_inner_effects() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let c1 = create_creature(&mut game, "Bear 1", alice);
        let c2 = create_creature(&mut game, "Bear 2", alice);

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = ForEachObject::new(
            ObjectFilter::creature().you_control(),
            vec![Effect::put_counters(
                CounterType::PlusOnePlusOne,
                1,
                ChooseSpec::Iterated,
            )],
        );
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Count(2));
        let c1_obj = game.object(c1).expect("c1 should exist");
        let c2_obj = game.object(c2).expect("c2 should exist");
        assert_eq!(c1_obj.counters.get(&CounterType::PlusOnePlusOne), Some(&1));
        assert_eq!(c2_obj.counters.get(&CounterType::PlusOnePlusOne), Some(&1));
    }

    #[test]
    fn test_for_each_uses_tagged_nonbattlefield_candidates_for_is_tagged_filters() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let revealed_creature = create_library_card(
            &mut game,
            "Revealed Creature",
            alice,
            vec![CardType::Creature],
        );
        let revealed_land = create_library_card(&mut game, "Revealed Land", alice, vec![CardType::Land]);

        ctx.tag_object(
            "revealed_0",
            ObjectSnapshot::from_object(game.object(revealed_creature).unwrap(), &game),
        );
        ctx.tag_object(
            "revealed_0",
            ObjectSnapshot::from_object(game.object(revealed_land).unwrap(), &game),
        );

        let mut filter = ObjectFilter::default();
        filter.excluded_card_types.push(CardType::Land);
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: TagKey::from("revealed_0"),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });

        let initial_life = game.player(alice).unwrap().life;
        let effect = ForEachObject::new(filter, vec![Effect::gain_life(1)]);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Count(1));
        assert_eq!(game.player(alice).unwrap().life, initial_life + 1);
    }
}
