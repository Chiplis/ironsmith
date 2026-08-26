//! For each counter kind on target, choose put or remove one.

use crate::decisions::context::{
    SelectObjectsContext, SelectOptionsContext, SelectableObject, SelectableOption,
};
use crate::effect::EffectOutcome;
use crate::effects::helpers::resolve_objects_for_effect;
use crate::effects::{EffectExecutor, PutCountersEffect, RemoveCountersEffect};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::object::CounterType;
use crate::target::ChooseSpec;

/// For each distinct counter type on the target permanent, choose to either
/// put one counter of that type on it or remove one from it.
#[derive(Debug, Clone, PartialEq)]
pub struct ForEachCounterKindPutOrRemoveEffect {
    pub target: ChooseSpec,
    pub counter_source: Option<ChooseSpec>,
    pub all_kinds: bool,
    pub fixed_counter_type: Option<CounterType>,
    pub optional_action: bool,
    pub put_only: bool,
    pub choose_target_per_kind: bool,
}

impl ForEachCounterKindPutOrRemoveEffect {
    pub fn new(target: ChooseSpec) -> Self {
        Self {
            target,
            counter_source: None,
            all_kinds: true,
            fixed_counter_type: None,
            optional_action: false,
            put_only: false,
            choose_target_per_kind: false,
        }
    }

    pub fn one_kind(target: ChooseSpec) -> Self {
        Self {
            target,
            counter_source: None,
            all_kinds: false,
            fixed_counter_type: None,
            optional_action: false,
            put_only: false,
            choose_target_per_kind: false,
        }
    }

    pub fn fixed_counter_type(
        target: ChooseSpec,
        counter_type: CounterType,
        optional_action: bool,
    ) -> Self {
        Self {
            target,
            counter_source: None,
            all_kinds: false,
            fixed_counter_type: Some(counter_type),
            optional_action,
            put_only: false,
            choose_target_per_kind: false,
        }
    }

    pub fn put_each_kind_from(counter_source: ChooseSpec, target: ChooseSpec) -> Self {
        Self {
            target,
            counter_source: Some(counter_source),
            all_kinds: true,
            fixed_counter_type: None,
            optional_action: false,
            put_only: true,
            choose_target_per_kind: true,
        }
    }

    fn counter_label(counter_type: CounterType) -> String {
        format!("{counter_type:?}").to_ascii_lowercase()
    }

    fn choose_counter_kind(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        counter_kinds: &[CounterType],
    ) -> Option<CounterType> {
        let options = counter_kinds
            .iter()
            .enumerate()
            .map(|(idx, counter_type)| {
                SelectableOption::new(
                    idx,
                    format!("Choose {} counter", Self::counter_label(*counter_type)),
                )
            })
            .collect::<Vec<_>>();
        let choice_ctx = SelectOptionsContext::new(
            ctx.controller,
            Some(ctx.source),
            "Choose a counter kind".to_string(),
            options,
            1,
            1,
        );
        let choice = ctx
            .decision_maker
            .decide_options(game, &choice_ctx)
            .into_iter()
            .next();
        if ctx.decision_maker.awaiting_choice() {
            return None;
        }
        choice.and_then(|idx| counter_kinds.get(idx).copied())
    }
}

impl EffectExecutor for ForEachCounterKindPutOrRemoveEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let target_ids = resolve_objects_for_effect(game, ctx, &self.target)?;
        if target_ids.is_empty() {
            return Ok(EffectOutcome::resolved());
        }

        let source_ids = if let Some(counter_source) = &self.counter_source {
            resolve_objects_for_effect(game, ctx, counter_source)?
        } else {
            target_ids.clone()
        };
        let mut shared_counter_kinds = source_ids
            .iter()
            .filter_map(|source_id| game.object(*source_id))
            .flat_map(|object| {
                object
                    .counters
                    .iter()
                    .filter_map(|(counter_type, count)| (*count > 0).then_some(*counter_type))
            })
            .collect::<Vec<_>>();
        shared_counter_kinds.sort_by_key(|counter_type| format!("{counter_type:?}"));
        shared_counter_kinds.dedup();

        let mut outcomes = Vec::new();
        let target_groups = if self.counter_source.is_some() {
            vec![target_ids]
        } else {
            target_ids
                .into_iter()
                .map(|target_id| vec![target_id])
                .collect()
        };
        for target_group in target_groups {
            let mut counter_kinds = if self.counter_source.is_some() {
                shared_counter_kinds.clone()
            } else {
                target_group
                    .first()
                    .and_then(|target_id| game.object(*target_id))
                    .map(|object| {
                        object
                            .counters
                            .iter()
                            .filter_map(|(counter_type, count)| {
                                (*count > 0).then_some(*counter_type)
                            })
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default()
            };
            if counter_kinds.is_empty() {
                continue;
            }
            counter_kinds.sort_by_key(|counter_type| format!("{counter_type:?}"));

            let selected_kinds = if let Some(counter_type) = self.fixed_counter_type {
                vec![counter_type]
            } else if self.all_kinds {
                counter_kinds
            } else {
                let Some(counter_type) = self.choose_counter_kind(game, ctx, &counter_kinds) else {
                    return Ok(EffectOutcome::count(0));
                };
                vec![counter_type]
            };

            for counter_type in selected_kinds {
                let target_id = if self.choose_target_per_kind {
                    let candidates = target_group
                        .iter()
                        .filter_map(|target_id| {
                            game.object(*target_id).map(|object| {
                                SelectableObject::new(*target_id, object.name.clone())
                            })
                        })
                        .collect::<Vec<_>>();
                    let choice = SelectObjectsContext::new(
                        ctx.controller,
                        Some(ctx.source),
                        format!(
                            "Choose an object to receive a {} counter",
                            Self::counter_label(counter_type)
                        ),
                        candidates,
                        1,
                        Some(1),
                    )
                    .require_explicit_choice();
                    let selected = ctx
                        .decision_maker
                        .decide_objects(game, &choice)
                        .into_iter()
                        .next();
                    if ctx.decision_maker.awaiting_choice() {
                        return Ok(EffectOutcome::count(0));
                    }
                    let Some(target_id) = selected.filter(|selected| {
                        choice
                            .candidates
                            .iter()
                            .any(|candidate| candidate.legal && candidate.id == *selected)
                    }) else {
                        continue;
                    };
                    target_id
                } else {
                    let Some(target_id) = target_group.first().copied() else {
                        continue;
                    };
                    target_id
                };

                if self.put_only {
                    outcomes.push(
                        PutCountersEffect::new(
                            counter_type,
                            1,
                            ChooseSpec::SpecificObject(target_id),
                        )
                        .execute(game, ctx)?,
                    );
                    continue;
                }

                let label = Self::counter_label(counter_type);
                let mut options = vec![
                    SelectableOption::new(0, format!("Put one {label} counter on it")),
                    SelectableOption::new(1, format!("Remove one {label} counter from it")),
                ];
                if self.optional_action {
                    options.push(SelectableOption::new(
                        2,
                        format!("Don't add or remove a {label} counter"),
                    ));
                }
                let choice_ctx = SelectOptionsContext::new(
                    ctx.controller,
                    Some(ctx.source),
                    format!("Choose for {label} counter"),
                    options,
                    1,
                    1,
                );
                let choice = ctx
                    .decision_maker
                    .decide_options(game, &choice_ctx)
                    .into_iter()
                    .next();
                if ctx.decision_maker.awaiting_choice() {
                    return Ok(EffectOutcome::count(0));
                }
                let max_choice = if self.optional_action { 2 } else { 1 };
                let Some(choice) = choice.filter(|idx| *idx <= max_choice) else {
                    return Ok(EffectOutcome::count(0));
                };

                if choice == 2 {
                    continue;
                }

                let spec = ChooseSpec::SpecificObject(target_id);
                let outcome = if choice == 1 {
                    RemoveCountersEffect::new(counter_type, 1, spec).execute(game, ctx)?
                } else {
                    PutCountersEffect::new(counter_type, 1, spec).execute(game, ctx)?
                };
                outcomes.push(outcome);
            }
        }

        Ok(EffectOutcome::aggregate_summing_counts(outcomes))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::object::Object;
    use crate::snapshot::ObjectSnapshot;
    use crate::types::CardType;
    use crate::zone::Zone;
    use std::collections::HashMap;

    fn creature(game: &mut GameState, name: &str, controller: PlayerId) -> ObjectId {
        let id = game.new_object_id();
        let card = CardBuilder::new(CardId::from_raw(id.0 as u32), name)
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(1, 1))
            .build();
        game.add_object(Object::from_card(id, &card, controller, Zone::Battlefield));
        id
    }

    #[test]
    fn put_each_kind_from_set_chooses_one_destination_for_every_distinct_kind() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let first_counter_source = creature(&mut game, "First source", alice);
        let second_counter_source = creature(&mut game, "Second source", alice);
        let first_destination = creature(&mut game, "First Fish", alice);
        let second_destination = creature(&mut game, "Second Fish", alice);
        game.add_counters(first_counter_source, CounterType::Charge, 1)
            .expect("source should accept a charge counter");
        game.add_counters(second_counter_source, CounterType::Flying, 1)
            .expect("source should accept a flying counter");

        let snapshots = [first_destination, second_destination]
            .into_iter()
            .map(|object_id| {
                ObjectSnapshot::from_object(game.object(object_id).expect("destination"), &game)
            })
            .collect::<Vec<_>>();
        let target_tag = crate::tag::TagKey::from("created");
        let mut decisions = crate::decision::SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new(source, alice, &mut decisions)
            .with_tagged_objects(HashMap::from([(target_tag.clone(), snapshots)]));

        let effect = ForEachCounterKindPutOrRemoveEffect::put_each_kind_from(
            ChooseSpec::All(
                crate::filter::ObjectFilter::creature()
                    .you_control()
                    .in_zone(Zone::Battlefield),
            ),
            ChooseSpec::Tagged(target_tag),
        );
        effect
            .execute(&mut game, &mut ctx)
            .expect("counter-kind distribution should resolve");

        assert_eq!(
            game.counter_count(first_destination, CounterType::Charge),
            1
        );
        assert_eq!(
            game.counter_count(first_destination, CounterType::Flying),
            1
        );
        assert_eq!(
            game.counter_count(second_destination, CounterType::Charge),
            0
        );
        assert_eq!(
            game.counter_count(second_destination, CounterType::Flying),
            0
        );
    }
}
