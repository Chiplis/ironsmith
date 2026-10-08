//! If effect implementation.

use crate::effect::{EffectOutcome, EffectPredicate, EffectPredicateRuntimeExt, ExecutionFact};
use crate::effects::EffectExecutor;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::filter::{ObjectFilterExt, player_filter_matches_game};
use crate::game_state::GameState;
use crate::target::ChooseSpec;
pub type IfEffect = ironsmith_core::IfEffect<crate::effect::Effect>;

fn object_filter_mentions_iterated_player(filter: &crate::target::ObjectFilter) -> bool {
    filter
        .controller
        .as_ref()
        .is_some_and(crate::target::PlayerFilter::mentions_iterated_player)
        || filter
            .owner
            .as_ref()
            .is_some_and(crate::target::PlayerFilter::mentions_iterated_player)
        || filter
            .targets_player
            .as_ref()
            .is_some_and(crate::target::PlayerFilter::mentions_iterated_player)
        || filter
            .targets_only_player
            .as_ref()
            .is_some_and(crate::target::PlayerFilter::mentions_iterated_player)
        || filter
            .attacking_player_or_planeswalker_controlled_by
            .as_ref()
            .is_some_and(crate::target::PlayerFilter::mentions_iterated_player)
        || filter
            .protected_by
            .as_ref()
            .is_some_and(crate::target::PlayerFilter::mentions_iterated_player)
        || filter
            .attached_to_player
            .as_ref()
            .is_some_and(crate::target::PlayerFilter::mentions_iterated_player)
        || filter
            .entered_battlefield_controller
            .as_ref()
            .is_some_and(crate::target::PlayerFilter::mentions_iterated_player)
        || filter
            .counters_put_on_this_turn
            .as_ref()
            .is_some_and(|constraint| constraint.source_controller.mentions_iterated_player())
        || filter
            .attached_to_object
            .as_deref()
            .is_some_and(object_filter_mentions_iterated_player)
        || filter
            .no_shared_creature_types_with
            .iter()
            .any(object_filter_mentions_iterated_player)
        || filter
            .characteristic_relations
            .iter()
            .any(|relation| object_filter_mentions_iterated_player(&relation.comparison))
        || filter
            .any_of
            .iter()
            .any(object_filter_mentions_iterated_player)
}

fn restriction_mentions_iterated_player(restriction: &crate::effect::Restriction) -> bool {
    match restriction {
        crate::effect::Restriction::PreventDamageFrom { sources, .. }
        | crate::effect::Restriction::ActivateLoyaltyAbilitiesOf(sources)
        | crate::effect::Restriction::MustAttack(sources)
        | crate::effect::Restriction::MustBlock(sources)
        | crate::effect::Restriction::MaximumBlockers { filter: sources, .. } => {
            object_filter_mentions_iterated_player(sources)
        }
        crate::effect::Restriction::PlayLandsMatching(player, filter) => {
            player.mentions_iterated_player() || object_filter_mentions_iterated_player(filter)
        }
        crate::effect::Restriction::AttackPlayerOrPlaneswalkersControlledBy {
            attackers,
            player,
        } => object_filter_mentions_iterated_player(attackers) || player.mentions_iterated_player(),
        _ => false,
    }
}

fn effect_mentions_iterated_player(effect: &crate::effect::Effect) -> bool {
    if let Some(cant) = effect.downcast_ref::<crate::effects::CantEffect>()
        && restriction_mentions_iterated_player(&cant.restriction)
    {
        return true;
    }

    let mut found = false;
    effect.visit_child_effects(&mut |child| {
        if effect_mentions_iterated_player(child) {
            found = true;
        }
    });
    found
}

fn effect_list_mentions_iterated_player(effects: &[crate::effect::Effect]) -> bool {
    effects.iter().any(effect_mentions_iterated_player)
}

fn result_memories_share_characteristic(
    memories: &[&crate::effect::ObjectSnapshot],
    required_count: usize,
    characteristic: crate::ObjectCharacteristic,
) -> bool {
    match characteristic {
        crate::ObjectCharacteristic::CardType => memories.iter().any(|candidate| {
            candidate.card_types.iter().any(|card_type| {
                memories
                    .iter()
                    .filter(|memory| memory.card_types.contains(card_type))
                    .count()
                    >= required_count
            })
        }),
        crate::ObjectCharacteristic::PermanentType => memories.iter().any(|candidate| {
            candidate
                .card_types
                .iter()
                .filter(|card_type| {
                    matches!(
                        card_type,
                        crate::types::CardType::Land
                            | crate::types::CardType::Creature
                            | crate::types::CardType::Artifact
                            | crate::types::CardType::Enchantment
                            | crate::types::CardType::Planeswalker
                            | crate::types::CardType::Battle
                    )
                })
                .any(|card_type| {
                    memories
                        .iter()
                        .filter(|memory| memory.card_types.contains(card_type))
                        .count()
                        >= required_count
                })
        }),
        crate::ObjectCharacteristic::Subtype(family) => memories.iter().any(|candidate| {
            candidate
                .subtypes
                .iter()
                .filter(|subtype| subtype.belongs_to_family(family))
                .any(|subtype| {
                    memories
                        .iter()
                        .filter(|memory| memory.subtypes.contains(subtype))
                        .count()
                        >= required_count
                })
        }),
        crate::ObjectCharacteristic::Color => crate::color::Color::ALL.into_iter().any(|color| {
            memories
                .iter()
                .filter(|memory| memory.colors.contains(color))
                .count()
                >= required_count
        }),
        crate::ObjectCharacteristic::ManaValue => memories.iter().any(|candidate| {
            memories
                .iter()
                .filter(|memory| memory.mana_value() == candidate.mana_value())
                .count()
                >= required_count
        }),
        crate::ObjectCharacteristic::Name => memories.iter().any(|candidate| {
            candidate
                .name
                .split(" // ")
                .chain(candidate.split_other_half_name())
                .filter(|name| !crate::filter::name_is_nameless(name))
                .any(|name| {
                    memories
                        .iter()
                        .filter(|memory| {
                            crate::filter::names_match(name, &memory.name)
                                || memory
                                    .split_other_half_name()
                                    .is_some_and(|other| crate::filter::names_match(name, other))
                        })
                        .count()
                        >= required_count
                })
        }),
    }
}

pub(super) fn predicate_matches_with_context(
    predicate: &EffectPredicate,
    outcome: &EffectOutcome,
    game: &GameState,
    ctx: &ExecutionContext,
) -> bool {
    if let EffectPredicate::AffectedObjectsShare { required_count, characteristic } = predicate {
        let Some(memory) = outcome.affected_object_memory() else { return false; };
        let mut seen = std::collections::HashSet::new();
        let objects = memory.iter().filter(|object| seen.insert(object.stable_id)).collect::<Vec<_>>();
        return *required_count >= 2 && objects.len() >= *required_count as usize
            && result_memories_share_characteristic(&objects, *required_count as usize, *characteristic);
    }
    if let EffectPredicate::PlayerAffectedObjectHasGreatestManaValue { player } = predicate {
        let Some(all_memory) = outcome.affected_object_memory() else {
            return false;
        };
        let Some(greatest) = all_memory.iter().map(|memory| memory.mana_value()).max() else {
            return false;
        };
        let Some(partitions) = outcome.player_affected_object_memory() else {
            return false;
        };
        let filter_ctx = ctx.filter_context(game);
        return partitions.iter().any(|(affected_player, memory)| {
            player_filter_matches_game(player, *affected_player, game, &filter_ctx)
                && memory.iter().any(|object| object.mana_value() == greatest)
        });
    }

    let EffectPredicate::PriorEffectResult(surface) = predicate else {
        return predicate.evaluate_outcome(outcome);
    };
    if surface.negated {
        let mut positive = surface.clone();
        positive.negated = false;
        return !predicate_matches_with_context(
            &EffectPredicate::PriorEffectResult(positive),
            outcome,
            game,
            ctx,
        );
    }
    if matches!(surface.action, crate::effect::PriorEffectAction::CountersMoved(_)) {
        return outcome.count_or_zero() > 0;
    }
    if surface.action == crate::effect::PriorEffectAction::Died {
        let filter_ctx = ctx.filter_context(game);
        let recorded =
            crate::effects::outcome_recording::action_objects(outcome, surface.action, None);
        let known = recorded.is_some();
        let matching = recorded.as_deref().or_else(|| outcome.affected_object_memory()).unwrap_or_default().iter()
            .filter(|memory| memory.card_types.contains(&crate::types::CardType::Creature)
                && surface.filter.matches_snapshot(memory, &filter_ctx, game)
                && (known || outcome.execution_facts.iter().any(|fact|
                                matches!(fact, crate::effect::ExecutionFact::ObjectsDied(ids) if ids.contains(&memory.object_id)))
                            || outcome.events_of_type::<crate::events::ZoneChangeEvent>().any(|event|
                    event.from == crate::zone::Zone::Battlefield
                        && event.to == crate::zone::Zone::Graveyard
                        && event.objects.contains(&memory.object_id))))
            .collect::<Vec<_>>();
        if matching.len() < surface.required_count.unwrap_or(1) as usize {
            return false;
        }
        return surface.shared_characteristic.is_none_or(|characteristic| {
            result_memories_share_characteristic(
                &matching,
                surface.required_count.unwrap_or(2) as usize,
                characteristic,
            )
        });
    }
    if surface.action == crate::effect::PriorEffectAction::PutIntoHand {
        let player = match surface.actor {
            crate::effect::PriorEffectResultActor::You => Some(ctx.controller),
            crate::effect::PriorEffectResultActor::ThatPlayer => {
                match ctx.iteration.iterated_player {
                    Some(player) => Some(player),
                    None => return false,
                }
            }
            crate::effect::PriorEffectResultActor::Passive => None,
            crate::effect::PriorEffectResultActor::It => return false,
        };
        let filter_ctx = ctx.filter_context(game);
        let cards = outcome
            .instruction_result()
            .execution_facts
            .iter()
            .filter_map(|fact| {
                let ExecutionFact::CardsPutIntoHand {
                    player: recipient,
                    cards,
                } = fact
                else {
                    return None;
                };
                player
                    .is_none_or(|player| player == *recipient)
                    .then_some(cards)
            })
            .flatten()
            .filter(|card| surface.filter.matches_snapshot(card, &filter_ctx, game))
            .collect::<Vec<_>>();
        if cards.len() < surface.required_count.unwrap_or(1) as usize {
            return false;
        }
        return surface.shared_characteristic.is_none_or(|characteristic| {
            result_memories_share_characteristic(
                &cards,
                surface.required_count.unwrap_or(2) as usize,
                characteristic,
            )
        });
    }
    if surface.action == crate::effect::PriorEffectAction::Drawn
        && surface.filter == crate::target::ObjectFilter::default()
        && surface.shared_characteristic.is_none()
    {
        let player = match surface.actor {
            crate::effect::PriorEffectResultActor::You => Some(ctx.controller),
            crate::effect::PriorEffectResultActor::ThatPlayer => {
                match ctx.iteration.iterated_player {
                    Some(player) => Some(player),
                    None => return false,
                }
            }
            crate::effect::PriorEffectResultActor::Passive => None,
            crate::effect::PriorEffectResultActor::It => return false,
        };
        if let Some(objects) = crate::effects::outcome_recording::action_objects(
            outcome,
            surface.action,
            player.as_ref().map(std::slice::from_ref),
        ) {
            return objects.len() >= surface.required_count.unwrap_or(1) as usize;
        }
        let drawn: u32 = outcome
            .instruction_result()
            .events_of_type::<crate::events::CardsDrawnEvent>()
            .filter(|event| player.is_none_or(|player| event.player == player))
            .map(|event| event.amount())
            .sum();
        return drawn >= surface.required_count.unwrap_or(1);
    }
    if surface.filter == crate::target::ObjectFilter::default()
        && surface.required_count.is_none()
        && surface.shared_characteristic.is_none()
    {
        let player = match surface.actor {
            crate::effect::PriorEffectResultActor::You => Some(ctx.controller),
            crate::effect::PriorEffectResultActor::ThatPlayer => {
                let Some(player) = ctx.iteration.iterated_player else {
                    return false;
                };
                Some(player)
            }
            crate::effect::PriorEffectResultActor::Passive => None,
            crate::effect::PriorEffectResultActor::It => return false,
        };
        if let Some(objects) = crate::effects::outcome_recording::action_objects(
            outcome,
            surface.action,
            player.as_ref().map(std::slice::from_ref),
        ) {
            return !objects.is_empty();
        }
        return predicate.evaluate_outcome(outcome.instruction_result());
    }

    let actor = match surface.actor {
        crate::effect::PriorEffectResultActor::You => Some(ctx.controller),
        crate::effect::PriorEffectResultActor::ThatPlayer => {
            let Some(player) = ctx.iteration.iterated_player else {
                return false;
            };
            Some(player)
        }
        crate::effect::PriorEffectResultActor::Passive => None,
        crate::effect::PriorEffectResultActor::It => return false,
    };
    let players = actor.map(|actor| vec![actor]);
    let recorded = crate::effects::outcome_recording::action_objects(
        outcome,
        surface.action,
        players.as_deref(),
    );
    let Some(memories) = recorded
        .as_deref()
        .or_else(|| outcome.affected_object_memory())
    else {
        return false;
    };
    let filter_ctx = ctx.filter_context(game);
    let matching = memories
        .iter()
        // One producer can record several actions ("create an Insect token,
        // then mill a card"); "an Insect card was milled this way" reads only
        // the milled cards, whose memory is captured in the library.
        .filter(|memory| {
            surface.action != crate::effect::PriorEffectAction::Milled
                || (memory.zone == crate::zone::Zone::Library && !memory.is_token)
        })
        .filter(|memory| surface.filter.matches_snapshot(memory, &filter_ctx, game))
        .collect::<Vec<_>>();
    if surface
        .required_count
        .is_some_and(|required| matching.len() < required as usize)
    {
        return false;
    }
    if let Some(characteristic) = surface.shared_characteristic {
        return result_memories_share_characteristic(
            &matching,
            surface.required_count.unwrap_or(2) as usize,
            characteristic,
        );
    }
    !matching.is_empty()
}

/// Effect that branches based on a prior effect's result.
///
/// Looks up the result of an effect executed with `WithId`, evaluates the predicate,
/// and executes either `then` or `else_` effects.
///
/// # Fields
///
/// * `condition` - The EffectId to check
/// * `predicate` - How to evaluate success
/// * `then` - Effects to execute if predicate is true
/// * `else_` - Effects to execute if predicate is false
///
/// # Example
///
/// ```ignore
/// // "Sacrifice a creature. If you do, draw two cards."
/// let effects = vec![
///     Effect::with_id(EffectId(0), Effect::sacrifice(ObjectFilter::creature(), 1)),
///     Effect::if_then(
///         EffectId(0),
///         EffectPredicate::Happened,
///         vec![Effect::draw(2)],
///     ),
/// ];
/// ```
#[derive(Clone)]
pub(crate) struct PreparedIfBranch {
    pub(crate) player: Option<crate::ids::PlayerId>,
    pub(crate) effects: Vec<crate::effect::Effect>,
    pub(crate) repetitions: usize,
    pub(crate) branch: usize,
}

/// Freeze contextual result predicates and participant partitions once.
pub(crate) fn prepare_if_branches(
    effect: &IfEffect,
    game: &GameState,
    ctx: &ExecutionContext,
) -> Vec<PreparedIfBranch> {
    let outcome = ctx
        .get_outcome(effect.condition)
        .cloned()
        .unwrap_or_else(EffectOutcome::impossible);
    let outcome = outcome.instruction_result();

    if matches!(
        effect.predicate,
        EffectPredicate::Happened
            | EffectPredicate::DidNotHappen
            | EffectPredicate::SearchedLibrary
    ) && (effect.per_player_result
        || effect_list_mentions_iterated_player(&effect.then)
        || effect_list_mentions_iterated_player(&effect.else_))
        && let Some(player_counts) = outcome.execution_facts.iter().find_map(|fact| match fact {
            ExecutionFact::PlayerCounts(counts) => Some(counts.clone()),
            _ => None,
        })
    {
        let mut branches = Vec::new();
        let searched_players = outcome
            .events
            .iter()
            .filter_map(|event| {
                event
                    .downcast::<crate::events::SearchLibraryEvent>()
                    .map(|event| event.player)
            })
            .collect::<Vec<_>>();
        // Inside a player loop the condition belongs to the iterated
        // player alone ("each opponent who didn't ..."): evaluate only
        // that player's result, treating a player who took no part in
        // the earlier action as one for whom it did not happen.
        let player_counts = match ctx.iteration.iterated_player {
            Some(iterated) => vec![(
                iterated,
                player_counts
                    .iter()
                    .find(|(player, _)| *player == iterated)
                    .map(|(_, count)| *count)
                    .unwrap_or(0),
            )],
            None => player_counts,
        };
        for (player_id, count) in player_counts {
            let predicate_matches = match effect.predicate {
                EffectPredicate::Happened => count > 0,
                EffectPredicate::DidNotHappen => count <= 0,
                EffectPredicate::SearchedLibrary => searched_players.contains(&player_id),
                _ => false,
            };
            let branch = if predicate_matches {
                &effect.then
            } else {
                &effect.else_
            };
            branches.push(PreparedIfBranch {
                player: Some(player_id),
                effects: branch.clone(),
                repetitions: 1,
                branch: if predicate_matches { 0 } else { 1 },
            });
        }
        return branches;
    }

    let match_repetitions = if let EffectPredicate::Value(cmp) = &effect.predicate {
        let chosen_numbers = outcome
            .execution_facts
            .iter()
            .filter_map(|fact| match fact {
                ExecutionFact::ChosenNumber(n) => Some(i64::from(*n)),
                _ => None,
            })
            .collect::<Vec<_>>();
        if chosen_numbers.is_empty() {
            None
        } else {
            let matches = chosen_numbers
                .into_iter()
                .filter(|value| cmp.evaluate_wide(*value))
                .count();
            Some(matches)
        }
    } else {
        None
    };

    let (branch, repetitions, branch_identity) = if let Some(matches) = match_repetitions {
        if matches > 0 {
            (&effect.then, matches, 0)
        } else {
            (&effect.else_, 1, 1)
        }
    } else if predicate_matches_with_context(&effect.predicate, outcome, game, ctx) {
        (&effect.then, 1, 0)
    } else {
        (&effect.else_, 1, 1)
    };

    if branch.is_empty() {
        Vec::new()
    } else {
        vec![PreparedIfBranch {
            player: None,
            effects: branch.clone(),
            repetitions,
            branch: branch_identity,
        }]
    }
}

fn if_branch_cursor(branches: &[PreparedIfBranch]) -> Box<dyn crate::effects::ActionProgramCursor> {
    super::branch_program::selected_branch_cursor(
        branches
            .iter()
            .enumerate()
            .map(
                |(index, branch)| super::branch_program::SelectedProgramBranch {
                    effects: branch.effects.clone(),
                    identity: vec![branch.branch, index],
                    repetitions: branch.repetitions,
                    scope: crate::effects::ProgramActionScope {
                        iterated_player: branch.player.map(Some),
                        ..Default::default()
                    },
                    child_scope: None,
                    first_scope: None,
                    match_before_first: true,
                },
            )
            .collect(),
    )
}

pub(crate) fn execute_if_branches_with_outputs(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    branches: &[PreparedIfBranch],
) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
    super::action_program::execute_action_program_with_outputs(
        if_branch_cursor(branches),
        game,
        ctx,
        crate::effects::EffectExecutionPurpose::Action,
    )
}

/// A shared antecedent selects an exact roster for one simultaneous authored
/// action. Delegate it to the same player-action owner as ordinary ForPlayers.
fn correlated_branch(effect: &IfEffect, branches: &[PreparedIfBranch])
    -> Option<(Vec<crate::ids::PlayerId>, Vec<crate::effect::Effect>)> {
    if !effect.per_player_result || !effect.else_.is_empty() { return None; }
    let first = branches.iter().find(|branch| !branch.effects.is_empty())?;
    if !first.effects.iter().all(|effect| effect.0.supports_simultaneous_player_action()
        || effect.0.is_read_only_simultaneous_player_action()
        || effect.0.supports_prepared_action_program()) { return None; }
    if !branches.iter().filter(|branch| !branch.effects.is_empty()).all(|branch|
        branch.player.is_some() && branch.repetitions == 1 && branch.effects == first.effects) { return None; }
    let participants = branches.iter().filter(|branch| !branch.effects.is_empty())
        .filter_map(|branch| branch.player).collect::<Vec<_>>();
    (participants.len() > 1).then(|| (participants, first.effects.clone()))
}

impl EffectExecutor for IfEffect {
    fn supports_replacement_draw_continuation(&self) -> bool {
        self.then.iter().chain(&self.else_).all(crate::effects::replacement::replacement_effect_supported)
    }
    fn prepare_replacement_draw_continuation_with_outputs(
        &self, game: &mut GameState, ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>, ExecutionError> {
        let branches = prepare_if_branches(self, game, ctx);
        if let Some((participants, effects)) = correlated_branch(self, &branches) {
            let excluded = participants.into_iter().fold(crate::target::PlayerFilter::Any, |remaining, player|
                crate::target::PlayerFilter::excluding(remaining, crate::target::PlayerFilter::Specific(player)));
            let filter = crate::target::PlayerFilter::excluding(crate::target::PlayerFilter::Any, excluded);
            return super::ForPlayersEffect::new(filter, effects).prepare_draw_continuation(game, ctx)
                .map(super::for_players::ForPlayersDrawProgress::into_commit);
        }
        let cursor = self.select_prepared_action_program(game, ctx)?;
        super::object_iteration::prepare_iteration_continuation(cursor, game, ctx)
    }

    fn supports_prepared_action_program(&self) -> bool {
        // A per-player antecedent owns its own simultaneous roster boundary.
        !self.per_player_result && self.then
            .iter()
            .chain(&self.else_)
            .all(super::action_program::action_program_child_is_prepared)
    }
    fn select_prepared_action_program(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<Option<Box<dyn crate::effects::ActionProgramCursor>>, ExecutionError> {
        if ctx.decision_maker.awaiting_choice() {
            return Ok(None);
        }
        let branches = prepare_if_branches(self, game, ctx);
        Ok(Some(if_branch_cursor(&branches)))
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn visit_child_effects(&self, visitor: &mut dyn FnMut(&crate::effect::Effect)) {
        for effect in &self.then {
            visitor(effect);
        }
        for effect in &self.else_ {
            visitor(effect);
        }
    }

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
        // A prior instruction that never ran (a declined optional, an
        // untaken branch, an antecedent skipped because its object is gone)
        // left no result: it didn't happen (CR 608.2c).
        let branches = prepare_if_branches(self, game, ctx);
        if let Some((participants, effects)) = correlated_branch(self, &branches) {
            return super::execute_player_occurrences_with_outputs(&effects, participants, game, ctx);
        }
        execute_if_branches_with_outputs(game, ctx, &branches)
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        super::target_metadata::first_target_spec(&[&self.then, &self.else_])
    }

    fn decision_related_object_specs(&self) -> Vec<ChooseSpec> {
        super::target_metadata::related_object_specs(&[&self.then, &self.else_])
    }

    fn target_description(&self) -> &'static str {
        super::target_metadata::first_target_description(&[&self.then, &self.else_], "target")
    }

    fn get_target_count(&self) -> Option<crate::effect::ChoiceCount> {
        super::target_metadata::first_target_count(&[&self.then, &self.else_])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::PlayerId;
    use crate::test_prelude::*;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    #[test]
    fn test_if_then_branch_taken() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        // Simulate a prior effect that "happened"
        ctx.store_outcome(EffectId(0), EffectOutcome::count(1));

        let initial_life = game.player(alice).unwrap().life;

        let effect = IfEffect::if_then(
            EffectId(0),
            EffectPredicate::Happened,
            vec![Effect::gain_life(5)],
        );
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        // Then branch should execute
        assert_eq!(result.value, crate::effect::OutcomeValue::Count(5));
        assert_eq!(game.player(alice).unwrap().life, initial_life + 5);
    }

    #[test]
    fn test_if_then_branch_not_taken() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        // Simulate a prior effect that didn't happen
        ctx.store_outcome(EffectId(0), EffectOutcome::count(0));

        let initial_life = game.player(alice).unwrap().life;

        let effect = IfEffect::if_then(
            EffectId(0),
            EffectPredicate::Happened,
            vec![Effect::gain_life(5)],
        );
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        // Then branch should NOT execute (no else branch, so Resolved)
        assert_eq!(result.status, crate::effect::OutcomeStatus::Succeeded);
        assert_eq!(game.player(alice).unwrap().life, initial_life);
    }

    #[test]
    fn player_partition_did_not_branch_applies_only_to_zero_count_player() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);
        let condition = EffectId(19);
        ctx.store_outcome(
            condition,
            EffectOutcome::count(1).with_player_counts(vec![(alice, 1), (bob, 0)]),
        );

        IfEffect::if_then(
            condition,
            EffectPredicate::DidNotHappen,
            vec![Effect::lose_life_player(
                crate::effect::Value::Fixed(1),
                PlayerFilter::IteratedPlayer,
            )],
        )
        .with_per_player_result(true)
        .execute(&mut game, &mut ctx)
        .expect("per-player result branch should resolve");

        assert_eq!(game.player(alice).expect("alice").life, 20);
        assert_eq!(game.player(bob).expect("bob").life, 19);
    }

    #[test]
    fn unmarked_player_counts_keep_one_global_result_evaluation() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);
        let condition = EffectId(20);
        ctx.store_outcome(
            condition,
            EffectOutcome::count(1).with_player_counts(vec![(alice, 1), (bob, 1)]),
        );

        IfEffect::if_then(
            condition,
            EffectPredicate::Happened,
            vec![Effect::gain_life(1)],
        )
        .execute(&mut game, &mut ctx)
        .expect("an ordinary result branch should resolve once");

        assert_eq!(game.player(alice).expect("alice").life, 21);
        assert_eq!(game.player(bob).expect("bob").life, 20);
    }

    #[test]
    fn test_if_else_branch() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        // Simulate a prior effect that didn't happen
        ctx.store_outcome(EffectId(0), EffectOutcome::count(0));

        let initial_life = game.player(alice).unwrap().life;

        let effect = IfEffect::new(
            EffectId(0),
            EffectPredicate::Happened,
            vec![Effect::gain_life(5)],
            vec![Effect::gain_life(2)], // else branch
        );
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        // Else branch should execute
        assert_eq!(result.value, crate::effect::OutcomeValue::Count(2));
        assert_eq!(game.player(alice).unwrap().life, initial_life + 2);
    }

    #[test]
    fn test_if_uses_full_outcome_not_only_summary_result() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        ctx.store_outcome(
            EffectId(0),
            EffectOutcome::with_details(
                crate::effect::OutcomeStatus::Succeeded,
                crate::effect::OutcomeValue::Count(0),
                vec![crate::events::RawEvent::new_with_provenance(
                    crate::events::TapEvent { permanent: source },
                    crate::provenance::ProvNodeId::default(),
                )],
                Vec::new(),
            ),
        );

        let initial_life = game.player(alice).unwrap().life;
        let effect = IfEffect::if_then(
            EffectId(0),
            EffectPredicate::Happened,
            vec![Effect::gain_life(5)],
        );
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(5));
        assert_eq!(game.player(alice).unwrap().life, initial_life + 5);
    }

    #[test]
    fn negated_prior_result_means_no_matching_objects_not_any_nonmatch() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);
        let mut surface = crate::effect::PriorEffectResultSurface::new(
            crate::effect::PriorEffectAction::Revealed,
            crate::target::ObjectFilter::default().with_type(crate::types::CardType::Creature),
            crate::effect::PriorEffectResultActor::Passive,
            crate::effect::PriorEffectResultQuantifier::One,
        );
        surface.negated = true;
        let predicate = EffectPredicate::PriorEffectResult(surface);
        for (types, expected) in [
            (vec![], true),
            (vec![crate::types::CardType::Land], true),
            (vec![crate::types::CardType::Creature], false),
            (
                vec![
                    crate::types::CardType::Land,
                    crate::types::CardType::Creature,
                ],
                false,
            ),
        ] {
            let memories = types
                .iter()
                .enumerate()
                .map(|(i, ty)| {
                    let id = crate::ids::ObjectId::from_raw(1000 + i as u64);
                    {
                        let mut snapshot = crate::snapshot::ObjectSnapshot::public_placeholder(
                            id,
                            crate::ids::StableId::from(id),
                            alice,
                            alice,
                            crate::zone::Zone::Library,
                        );
                        snapshot.name = "Revealed Probe".into();
                        snapshot.power = None;
                        snapshot.toughness = None;
                        snapshot.linked_face_mana_value = Some((1) as u32);
                        snapshot.card_types = vec![*ty];
                        snapshot.colors = crate::color::ColorSet::default();
                        snapshot.subtypes = vec![];
                        snapshot.is_token = false;
                        snapshot
                    }
                })
                .collect();
            let outcome =
                EffectOutcome::count(types.len() as i32).with_affected_object_memory(memories);
            assert_eq!(predicate.evaluate_outcome(&outcome), expected);
            assert_eq!(
                predicate_matches_with_context(&predicate, &outcome, &game, &ctx),
                expected
            );
            ctx.store_outcome(EffectId(0), outcome);
            let life = game.player(alice).unwrap().life;
            IfEffect::if_then(EffectId(0), predicate.clone(), vec![Effect::gain_life(1)])
                .execute(&mut game, &mut ctx)
                .unwrap();
            assert_eq!(game.player(alice).unwrap().life, life + i32::from(expected));
        }
    }

    #[test]
    fn prior_result_counted_shared_color_uses_filtered_affected_object_memory() {
        fn memory(
            id: u64,
            card_type: crate::types::CardType,
            colors: crate::color::ColorSet,
        ) -> crate::effect::ObjectSnapshot {
            let object_id = crate::ids::ObjectId::from_raw(id);
            {
                let mut snapshot = crate::snapshot::ObjectSnapshot::public_placeholder(
                    object_id,
                    crate::ids::StableId::from(object_id),
                    PlayerId::from_index(0),
                    PlayerId::from_index(0),
                    crate::zone::Zone::Library,
                );
                snapshot.name = format!("Card {id}");
                snapshot.power = None;
                snapshot.toughness = None;
                snapshot.linked_face_mana_value = Some((id as i32) as u32);
                snapshot.card_types = vec![card_type];
                snapshot.colors = colors;
                snapshot.subtypes = Vec::new();
                snapshot.is_token = false;
                snapshot
            }
        }

        let game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = crate::ids::ObjectId::from_raw(100);
        let ctx = ExecutionContext::new_default(source, alice);
        let mut nonland = crate::target::ObjectFilter::default();
        nonland
            .excluded_card_types
            .push(crate::types::CardType::Land);
        let predicate = EffectPredicate::PriorEffectResult(
            crate::effect::PriorEffectResultSurface::new(
                crate::effect::PriorEffectAction::Milled,
                nonland,
                crate::effect::PriorEffectResultActor::Passive,
                crate::effect::PriorEffectResultQuantifier::One,
            )
            .with_count_sharing(2, crate::ObjectCharacteristic::Color),
        );

        let shared_blue = EffectOutcome::count(2).with_affected_object_memory(vec![
            memory(
                1,
                crate::types::CardType::Creature,
                crate::color::ColorSet::BLUE,
            ),
            memory(
                2,
                crate::types::CardType::Instant,
                crate::color::ColorSet::BLUE,
            ),
        ]);
        assert!(predicate_matches_with_context(
            &predicate,
            &shared_blue,
            &game,
            &ctx
        ));

        let different_colors = EffectOutcome::count(2).with_affected_object_memory(vec![
            memory(
                3,
                crate::types::CardType::Creature,
                crate::color::ColorSet::BLUE,
            ),
            memory(
                4,
                crate::types::CardType::Instant,
                crate::color::ColorSet::RED,
            ),
        ]);
        assert!(!predicate_matches_with_context(
            &predicate,
            &different_colors,
            &game,
            &ctx
        ));

        let matching_land_is_excluded = EffectOutcome::count(2).with_affected_object_memory(vec![
            memory(
                5,
                crate::types::CardType::Creature,
                crate::color::ColorSet::BLUE,
            ),
            memory(
                6,
                crate::types::CardType::Land,
                crate::color::ColorSet::BLUE,
            ),
        ]);
        assert!(!predicate_matches_with_context(
            &predicate,
            &matching_land_is_excluded,
            &game,
            &ctx
        ));

        let three_share_predicate = EffectPredicate::PriorEffectResult(
            crate::effect::PriorEffectResultSurface::new(
                crate::effect::PriorEffectAction::Milled,
                crate::target::ObjectFilter::default(),
                crate::effect::PriorEffectResultActor::Passive,
                crate::effect::PriorEffectResultQuantifier::One,
            )
            .with_count_sharing(3, crate::ObjectCharacteristic::Color),
        );
        let only_a_pair_shares_blue = EffectOutcome::count(3).with_affected_object_memory(vec![
            memory(
                7,
                crate::types::CardType::Creature,
                crate::color::ColorSet::BLUE,
            ),
            memory(
                8,
                crate::types::CardType::Instant,
                crate::color::ColorSet::BLUE,
            ),
            memory(
                9,
                crate::types::CardType::Sorcery,
                crate::color::ColorSet::RED,
            ),
        ]);
        assert!(!predicate_matches_with_context(
            &three_share_predicate,
            &only_a_pair_shares_blue,
            &game,
            &ctx
        ));
    }

    #[test]
    fn participant_discard_extremum_executes_on_a_tie_but_not_a_strict_loss() {
        fn memory(id: u64, player: PlayerId, mana_value: i32) -> crate::effect::ObjectSnapshot {
            let object_id = crate::ids::ObjectId::from_raw(id);
            let mut snapshot = crate::snapshot::ObjectSnapshot::public_placeholder(
                object_id,
                crate::ids::StableId::from(object_id),
                player,
                player,
                crate::zone::Zone::Hand,
            );
            snapshot.name = format!("Discarded {id}");
            snapshot.linked_face_mana_value = Some(mana_value as u32);
            snapshot.card_types = vec![crate::types::CardType::Sorcery];
            snapshot
        }

        fn run_case(your_mana_value: i32, their_mana_value: i32) -> u32 {
            let mut game = setup_game();
            let alice = PlayerId::from_index(0);
            let bob = PlayerId::from_index(1);
            let source_card =
                crate::card::CardBuilder::new(crate::ids::CardId::new(), "Cage Brawler")
                    .card_types(vec![crate::types::CardType::Creature])
                    .build();
            let source =
                game.create_object_from_card(&source_card, alice, crate::zone::Zone::Battlefield);
            let mut ctx = ExecutionContext::new_default(source, alice);
            let condition = EffectId(7);
            let yours = memory(7001, alice, your_mana_value);
            let theirs = memory(7002, bob, their_mana_value);
            ctx.store_outcome(
                condition,
                EffectOutcome::count(2)
                    .with_affected_object_memory(vec![yours.clone(), theirs.clone()])
                    .with_player_affected_object_memory(vec![
                        (alice, vec![yours]),
                        (bob, vec![theirs]),
                    ]),
            );

            IfEffect::if_then(
                condition,
                EffectPredicate::PlayerAffectedObjectHasGreatestManaValue {
                    player: crate::target::PlayerFilter::You,
                },
                vec![Effect::put_counters_on_source(
                    crate::object::CounterType::PlusOnePlusOne,
                    2,
                )],
            )
            .execute(&mut game, &mut ctx)
            .expect("participant-result branch should resolve");

            game.counter_count(source, crate::object::CounterType::PlusOnePlusOne)
        }

        assert_eq!(run_case(5, 5), 2, "a tied maximum satisfies the branch");
        assert_eq!(
            run_case(5, 6),
            0,
            "another participant's strict maximum rejects the branch"
        );
    }

    #[test]
    fn test_if_skipped_condition_does_not_happen() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = IfEffect::if_then(
            EffectId(0),
            EffectPredicate::Happened,
            vec![Effect::gain_life(5)],
        );
        let outcome = effect
            .execute(&mut game, &mut ctx)
            .expect("a skipped antecedent did not happen");
        assert_eq!(game.player(alice).unwrap().life, 20);
        assert!(outcome.events.is_empty());
        assert!(ctx.get_outcome(EffectId(0)).is_none());
        let negative = IfEffect::if_then(
            EffectId(0),
            EffectPredicate::DidNotHappen,
            vec![Effect::gain_life(2)],
        );
        negative
            .execute(&mut game, &mut ctx)
            .expect("did-not-happen branch handles a skipped antecedent");
        assert_eq!(game.player(alice).unwrap().life, 22);
    }

    #[test]
    fn test_if_clone_box() {
        let effect = IfEffect::if_then(
            EffectId(0),
            EffectPredicate::Happened,
            vec![Effect::gain_life(1)],
        );
        let cloned = effect.clone_box();
        assert!(format!("{:?}", cloned).contains("IfEffect"));
    }

    #[test]
    fn if_effect_forwards_inner_target_spec_from_then_branch() {
        let effect = IfEffect::if_then(
            EffectId(0),
            EffectPredicate::Happened,
            vec![Effect::counter(ChooseSpec::target_spell())],
        );

        assert!(effect.get_target_spec().is_some());
        assert_eq!(effect.target_description(), "spell to counter");
    }

    #[test]
    fn if_effect_forwards_inner_target_spec_from_else_branch() {
        let effect = IfEffect::new(
            EffectId(0),
            EffectPredicate::Happened,
            vec![Effect::draw(1)],
            vec![Effect::counter(ChooseSpec::target_spell())],
        );

        assert!(effect.get_target_spec().is_some());
        assert_eq!(effect.target_description(), "spell to counter");
    }
}

#[cfg(test)]
mod replacement_original_if_adapter_contract_tests {
    use super::*;
    use crate::effect::{Effect, EffectId};
    #[test]
    fn auxiliary_player_counts_do_not_create_original_per_player_followups() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = crate::ids::PlayerId::from_index(0);
        let bob = crate::ids::PlayerId::from_index(1);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);
        ctx.store_outcome(
            EffectId(921),
            EffectOutcome::aggregate_replacement_outcomes(
                EffectOutcome::count(0),
                [EffectOutcome::count(1).with_player_counts(vec![(bob, 1)])],
            ),
        );
        IfEffect::if_then(
            EffectId(921),
            EffectPredicate::Happened,
            vec![Effect::lose_life_player(
                crate::effect::Value::Fixed(1),
                crate::target::PlayerFilter::IteratedPlayer,
            )],
        )
        .with_per_player_result(true)
        .execute(&mut game, &mut ctx)
        .unwrap();
        assert_eq!(game.player(alice).unwrap().life, 20);
        assert_eq!(game.player(bob).unwrap().life, 20);
    }
    #[test]
    fn auxiliary_chosen_number_does_not_repeat_original_value_followup() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = crate::ids::PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);
        ctx.store_outcome(
            EffectId(922),
            EffectOutcome::aggregate_replacement_outcomes(
                EffectOutcome::count(0),
                [EffectOutcome::count(1).with_execution_fact(ExecutionFact::ChosenNumber(1))],
            ),
        );
        IfEffect::if_then(
            EffectId(922),
            EffectPredicate::Value(crate::effect::Comparison::Equal(1)),
            vec![Effect::gain_life(5)],
        )
        .execute(&mut game, &mut ctx)
        .unwrap();
        assert_eq!(game.player(alice).unwrap().life, 20);
    }
    #[test]
    fn auxiliary_search_event_does_not_satisfy_original_player_search_followup() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = crate::ids::PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);
        let auxiliary =
            EffectOutcome::count(0).with_event(crate::events::RawEvent::new_with_provenance(
                crate::events::SearchLibraryEvent::new(alice, Some(alice)),
                crate::provenance::ProvNodeId::default(),
            ));
        ctx.store_outcome(
            EffectId(923),
            EffectOutcome::aggregate_replacement_outcomes(
                EffectOutcome::count(0).with_player_counts(vec![(alice, 0)]),
                [auxiliary],
            ),
        );
        IfEffect::if_then(
            EffectId(923),
            EffectPredicate::SearchedLibrary,
            vec![Effect::gain_life(5)],
        )
        .with_per_player_result(true)
        .execute(&mut game, &mut ctx)
        .unwrap();
        assert_eq!(
            ctx.get_outcome(EffectId(923)).unwrap().events.len(),
            1,
            "actual auxiliary search remains observable"
        );
        assert_eq!(game.player(alice).unwrap().life, 20);
    }
    #[test]
    fn result_name_relation_requires_one_common_name_and_retains_split_names() {
        use crate::{ObjectCharacteristic, PlayerId, Zone};
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let mut memory = |left: &str, right: Option<&str>| {
            let id = game.new_object_id();
            let mut snapshot = crate::snapshot::ObjectSnapshot::public_placeholder(
                id,
                id.into(),
                PlayerId(0),
                PlayerId(0),
                Zone::Hand,
            );
            snapshot.name = left.into();
            if let Some(right) = right {
                snapshot.linked_face_layout = crate::card::LinkedFaceLayout::Split;
                snapshot.other_face_name = Some(right.into());
            }
            Clone::clone(&snapshot)
        };
        let ab = memory("Alpha", Some("Beta"));
        let bc = memory("Beta", Some("Gamma"));
        let ca = memory("Gamma", Some("Alpha"));
        let beta = memory("Beta", None);
        let nameless = memory("", None);
        assert!(result_memories_share_characteristic(
            &[&ab, &bc, &beta],
            3,
            ObjectCharacteristic::Name
        ));
        assert!(!result_memories_share_characteristic(
            &[&ab, &bc, &ca],
            3,
            ObjectCharacteristic::Name
        ));
        assert!(!result_memories_share_characteristic(
            &[&nameless],
            1,
            ObjectCharacteristic::Name
        ));
    }
    #[test]
    fn affected_name_subset_uses_distinct_latest_objects_and_rejects_short_or_empty_sets() {
        use crate::ids::PlayerId;
        let game = crate::tests::test_helpers::setup_two_player_game();
        let ctx = ExecutionContext::new_default(crate::ids::ObjectId::from_raw(500), PlayerId(0));
        let memory = |id: u64, name: &str| {
            let object = crate::ids::ObjectId::from_raw(id);
            let mut snapshot = crate::snapshot::ObjectSnapshot::public_placeholder(
                object, object.into(), PlayerId(0), PlayerId(0), crate::zone::Zone::Library,
            );
            snapshot.name = name.into();
            snapshot
        };
        let pair = vec![memory(1, "Same"), memory(2, "Same"), memory(3, "Other"), memory(4, "Different")];
        let predicate = EffectPredicate::AffectedObjectsShare {
            required_count: 2, characteristic: crate::ObjectCharacteristic::Name,
        };
        for (objects, expected) in [
            (pair.clone(), true), (pair[..2].to_vec(), true),
            (vec![pair[0].clone()], false), (Vec::new(), false),
            (vec![pair[0].clone(), pair[0].clone()], false),
            (vec![pair[1].clone(), pair[2].clone()], false),
            (vec![memory(5, ""), memory(6, "")], false),
        ] {
            let result = EffectOutcome::count(objects.len() as i32).with_affected_object_memory(objects);
            assert_eq!(predicate_matches_with_context(&predicate, &result, &game, &ctx), expected);
        }
    }

}
