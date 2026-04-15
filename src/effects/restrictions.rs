//! Effects that apply rule restrictions ("can't" effects).

use std::collections::HashSet;

use crate::effect::{EffectOutcome, Restriction, Until};
use crate::effects::EffectExecutor;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::ObjectFilter;

fn collapse_tagged_filter_to_specific_objects(
    filter: &ObjectFilter,
    ctx: &ExecutionContext,
) -> ObjectFilter {
    if filter.source || filter.tagged_constraints.is_empty() {
        return filter.clone();
    }

    if !filter.tagged_constraints.iter().all(|constraint| {
        constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
    }) {
        return filter.clone();
    }

    let mut seen = HashSet::new();
    let mut object_ids = filter
        .tagged_constraints
        .iter()
        .filter_map(|constraint| ctx.get_tagged_all(&constraint.tag))
        .flat_map(|snapshots| snapshots.iter())
        .filter_map(|snapshot| {
            seen.insert(snapshot.object_id)
                .then_some(snapshot.object_id)
        })
        .collect::<Vec<_>>();

    if object_ids.is_empty() {
        let mut fallback_seen = HashSet::new();
        object_ids = ctx
            .tagged_objects
            .values()
            .flat_map(|snapshots| snapshots.iter())
            .filter_map(|snapshot| {
                fallback_seen
                    .insert(snapshot.object_id)
                    .then_some(snapshot.object_id)
            })
            .collect();
    }

    match object_ids.as_slice() {
        [] => filter.clone(),
        [object_id] => ObjectFilter::specific(*object_id),
        _ => ObjectFilter {
            any_of: object_ids.into_iter().map(ObjectFilter::specific).collect(),
            ..Default::default()
        },
    }
}

fn normalize_restriction_for_resolution(
    restriction: &Restriction,
    ctx: &ExecutionContext,
) -> Restriction {
    match restriction {
        Restriction::BeBlocked(filter) => {
            Restriction::be_blocked(collapse_tagged_filter_to_specific_objects(filter, ctx))
        }
        _ => restriction.clone(),
    }
}

/// Effect that applies a restriction for a duration.
#[derive(Debug, Clone, PartialEq)]
pub struct CantEffect {
    pub restriction: Restriction,
    pub duration: Until,
}

impl CantEffect {
    pub fn new(restriction: Restriction, duration: Until) -> Self {
        Self {
            restriction,
            duration,
        }
    }

    pub fn until_end_of_turn(restriction: Restriction) -> Self {
        Self::new(restriction, Until::EndOfTurn)
    }
}

impl EffectExecutor for CantEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let restriction = normalize_restriction_for_resolution(&self.restriction, ctx);
        if matches!(self.duration, Until::ControllersNextUntapStep)
            && let Restriction::Untap(filter) = &restriction
        {
            let filter_ctx = ctx.filter_context(game);
            let targets: Vec<_> = game
                .battlefield
                .iter()
                .filter_map(|object_id| {
                    let obj = game.object(*object_id)?;
                    if filter.matches(obj, &filter_ctx, game) {
                        Some((*object_id, obj.controller))
                    } else {
                        None
                    }
                })
                .collect();

            if !targets.is_empty() {
                for (object_id, controller) in targets {
                    game.add_restriction_effect(
                        Restriction::untap(crate::target::ObjectFilter::specific(object_id)),
                        self.duration.clone(),
                        ctx.source,
                        controller,
                    );
                }
            } else {
                game.add_restriction_effect(
                    self.restriction.clone(),
                    self.duration.clone(),
                    ctx.source,
                    ctx.controller,
                );
            }
        } else {
            game.add_restriction_effect(
                restriction,
                self.duration.clone(),
                ctx.source,
                ctx.controller,
            );
        }
        game.update_cant_effects();
        Ok(EffectOutcome::resolved())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PowerToughness;
    use crate::card::CardBuilder;
    use crate::effects::RegenerateEffect;
    use crate::executor::ExecutionContext;
    use crate::game_state::GameState;
    use crate::ids::CardId;
    use crate::ids::PlayerId;
    use crate::snapshot::ObjectSnapshot;
    use crate::target::{ObjectFilter, PlayerFilter};
    use crate::types::CardType;
    use crate::zone::Zone;

    #[test]
    fn cant_effect_blocks_life_gain() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = CantEffect::until_end_of_turn(Restriction::gain_life(PlayerFilter::Any));
        effect.execute(&mut game, &mut ctx).expect("execute cant");

        game.update_cant_effects();

        assert!(!game.can_gain_life(PlayerId::from_index(0)));
        assert!(!game.can_gain_life(PlayerId::from_index(1)));
    }

    #[test]
    fn cant_be_regenerated_clears_existing_regeneration_shields() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);

        let creature_card = CardBuilder::new(CardId::from_raw(1), "Shielded Bear")
            .card_types(vec![CardType::Creature])
            .build();
        let creature_id = game.create_object_from_card(&creature_card, alice, Zone::Battlefield);

        let mut regen_ctx = ExecutionContext::new_default(creature_id, alice);
        RegenerateEffect::source(Until::EndOfTurn)
            .execute(&mut game, &mut regen_ctx)
            .expect("apply regeneration shield");
        assert!(
            game.effect_store
                .replacement_effects
                .count_one_shot_effects_from_source(creature_id)
                > 0
        );

        let source = game.new_object_id();
        let mut cant_ctx = ExecutionContext::new_default(source, alice);
        CantEffect::until_end_of_turn(Restriction::be_regenerated(ObjectFilter::specific(
            creature_id,
        )))
        .execute(&mut game, &mut cant_ctx)
        .expect("apply cant be regenerated");

        assert!(!game.can_be_regenerated(creature_id));
        assert_eq!(
            game.effect_store
                .replacement_effects
                .count_one_shot_effects_from_source(creature_id),
            0
        );
    }

    #[test]
    fn cant_effect_normalizes_source_tagged_be_blocked_filter() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);

        let creature_card = CardBuilder::new(CardId::from_raw(1), "Tagged Bear")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        let creature_id = game.create_object_from_card(&creature_card, alice, Zone::Battlefield);

        let source_snapshot = ObjectSnapshot::from_object(
            game.object(creature_id).expect("source creature exists"),
            &game,
        );
        let mut ctx = ExecutionContext::new_default(creature_id, alice);
        ctx.tag_object("carry", source_snapshot);

        CantEffect::until_end_of_turn(Restriction::be_blocked(ObjectFilter::tagged("carry")))
            .execute(&mut game, &mut ctx)
            .expect("execute be blocked cant effect");

        assert!(
            !game.can_be_blocked(creature_id),
            "tagged source be-blocked restriction should normalize to the source object"
        );
    }

    #[test]
    fn cant_effect_normalizes_tagged_be_blocked_filter_even_when_source_is_stack_object() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);

        let creature_card = CardBuilder::new(CardId::from_raw(1), "Tagged Octopus")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 1))
            .build();
        let creature_id = game.create_object_from_card(&creature_card, alice, Zone::Battlefield);
        let stack_object_id = game.new_object_id();

        let creature_snapshot = ObjectSnapshot::from_object(
            game.object(creature_id).expect("tagged creature exists"),
            &game,
        );
        let mut ctx = ExecutionContext::new_default(stack_object_id, alice);
        ctx.tag_object("carry", creature_snapshot);

        CantEffect::until_end_of_turn(Restriction::be_blocked(ObjectFilter::tagged("carry")))
            .execute(&mut game, &mut ctx)
            .expect("execute be blocked cant effect from stack source");

        assert!(
            !game.can_be_blocked(creature_id),
            "tagged be-blocked restriction should stay attached to the resolved creature, not the stack object source"
        );
    }

    #[test]
    fn cant_effect_normalizes_tagged_be_blocked_filter_when_runtime_tag_aliases_drift() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);

        let creature_card = CardBuilder::new(CardId::from_raw(1), "Alias Drift Octopus")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 1))
            .build();
        let creature_id = game.create_object_from_card(&creature_card, alice, Zone::Battlefield);

        let creature_snapshot = ObjectSnapshot::from_object(
            game.object(creature_id).expect("tagged creature exists"),
            &game,
        );
        let mut ctx = ExecutionContext::new_default(creature_id, alice);
        ctx.tag_object("granted_0", creature_snapshot.clone());
        ctx.tag_object("__it__", creature_snapshot);

        CantEffect::until_end_of_turn(Restriction::be_blocked(ObjectFilter::tagged("targeted_0")))
            .execute(&mut game, &mut ctx)
            .expect("execute be blocked cant effect with drifted runtime tag alias");

        assert!(
            !game.can_be_blocked(creature_id),
            "tagged be-blocked restriction should still resolve when the runtime context only retains equivalent aliases for the same object"
        );
    }
}
