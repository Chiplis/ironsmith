//! For each counter kind on target, choose put or remove one.

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
        super::kind_selection::counter_label(counter_type)
    }

    fn put_another_of_each_kind_on_players(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        players: &[crate::ids::PlayerId],
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        let mut requests = Vec::new();
        for player_id in players {
            let Some(player) = game.player(*player_id) else {
                continue;
            };
            for counter_type in player.counter_types_with_counters() {
                requests.push(
                    crate::events::Event::put_player_counters(
                        *player_id,
                        counter_type,
                        1,
                        ctx.cause.clone(),
                    )
                    .with_provenance(ctx.provenance),
                );
            }
        }
        if requests.is_empty() {
            return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                EffectOutcome::resolved(),
            ));
        }
        let children = super::execute_counter_batch_with_outputs(game, ctx, requests)?;
        if ctx.decision_maker.awaiting_choice() {
            return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                EffectOutcome::count(0),
            ));
        }
        Ok(crate::effects::CompletedEffectOutputs::from_children(
            children,
            EffectOutcome::aggregate_summing_counts,
        ))
    }
}

impl EffectExecutor for ForEachCounterKindPutOrRemoveEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        self.execute_with_outputs(game, ctx)
            .map(crate::effects::CompletedEffectOutputs::into_outcome)
    }
    fn execute_with_outputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        crate::effects::composition::execute_transaction(
            game,
            ctx,
            || crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
            |game, ctx| {
                // CR 122.1: players can have counters too. "For each kind of counter
                // on target permanent or player, give that permanent or player
                // another counter of that kind" puts one more of every kind the
                // chosen player has.
                if self.put_only
                    && self.all_kinds
                    && self.counter_source.is_none()
                    && self.fixed_counter_type.is_none()
                    && self.target.is_target()
                    && matches!(
                        self.target.base(),
                        ChooseSpec::ObjectOrPlayer(..) | ChooseSpec::Player(_)
                    )
                {
                    let players = crate::effects::helpers::matching_player_targets_for_spec(
                        game,
                        &self.target,
                        ctx,
                    );
                    if !players.is_empty() {
                        return self.put_another_of_each_kind_on_players(game, ctx, &players);
                    }
                }

                let target_ids = resolve_objects_for_effect(game, ctx, &self.target)?;
                if target_ids.is_empty() {
                    return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::resolved(),
                    ));
                }

                let source_ids = if let Some(counter_source) = &self.counter_source {
                    resolve_objects_for_effect(game, ctx, counter_source)?
                } else {
                    target_ids.clone()
                };
                let shared_counter_kinds = super::kind_selection::kinds_on(game, &source_ids);

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
                        super::kind_selection::kinds_on(game, &target_group)
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
                        let Some(counter_type) =
                            super::kind_selection::choose_kind(game, ctx, &counter_kinds)
                        else {
                            return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                                EffectOutcome::count(0),
                            ));
                        };
                        vec![counter_type]
                    };

                    for counter_type in selected_kinds {
                        let target_id = if self.choose_target_per_kind {
                            let candidates = target_group
                                .iter()
                                .copied()
                                .filter(|id| game.object(*id).is_some())
                                .collect::<Vec<_>>();
                            let selected = crate::effects::composition::captured_selection::choose_captured_object(game, ctx, ctx.controller, &candidates,
                                &format!("Choose an object to receive a {} counter", Self::counter_label(counter_type)),
                                crate::effects::composition::captured_selection::CapturedChoicePolicy { prompt_empty: true, ..crate::effects::composition::captured_selection::CapturedChoicePolicy::EXPLICIT_FIRST })?;
                            if ctx.decision_maker.awaiting_choice() {
                                return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                                    EffectOutcome::count(0),
                                ));
                            }
                            let Some(target_id) = selected else {
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
                            outcomes.push(super::kind_selection::put_kind_on_recipients(
                                game,
                                ctx,
                                counter_type,
                                &[target_id],
                                None,
                                false,
                            )?);
                            if ctx.decision_maker.awaiting_choice() {
                                return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                                    EffectOutcome::count(0),
                                ));
                            }
                            continue;
                        }

                        let label = Self::counter_label(counter_type);
                        let mut options = vec![
                            (0, format!("Put one {label} counter on it")),
                            (1, format!("Remove one {label} counter from it")),
                        ];
                        if self.optional_action {
                            options.push((2, format!("Don't add or remove a {label} counter")));
                        }
                        let Some(choice) = super::kind_selection::choose_binding(
                            game,
                            ctx,
                            &format!("Choose for {label} counter"),
                            &options,
                            false,
                        ) else {
                            return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                                EffectOutcome::count(0),
                            ));
                        };

                        if choice == 2 {
                            continue;
                        }

                        let spec = ChooseSpec::SpecificObject(target_id);
                        let outcome = if choice == 1 {
                            RemoveCountersEffect::new(counter_type, 1, spec)
                                .execute_child_with_outputs(game, ctx)?
                        } else {
                            PutCountersEffect::new(counter_type, 1, spec)
                                .execute_child_with_outputs(game, ctx)?
                        };
                        outcomes.push(outcome);
                        if ctx.decision_maker.awaiting_choice() {
                            return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                                EffectOutcome::count(0),
                            ));
                        }
                    }
                }

                Ok(crate::effects::CompletedEffectOutputs::from_children(
                    outcomes,
                    EffectOutcome::aggregate_summing_counts,
                ))
            },
        )
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
