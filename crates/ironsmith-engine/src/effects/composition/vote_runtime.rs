//! Runtime orchestration for `VoteEffect`.

use crate::filter::ObjectFilterExt as _;
use std::collections::HashMap;

use crate::decision::FallbackStrategy;
use crate::decisions::spec::DisplayOption;
use crate::decisions::specs::{ChoiceSpec, ChooseObjectsSpec};
use crate::decisions::{make_boolean_decision, make_decision};
use crate::effect::EffectOutcome;
use crate::effects::helpers::resolve_player_filter_to_list;
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError};
use crate::events::{
    KeywordActionEvent, KeywordActionKind, PlayerVote, PlayersFinishedVotingEvent,
};
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::snapshot::ObjectSnapshot;
use crate::tag::TagKey;
use crate::triggers::TriggerEvent;
use crate::zone::Zone;

use super::vote::{VOTE_WINNERS_TAG, VOTED_OBJECTS_TAG, VoteChoice, VoteEffect, VoteResult};

fn option_vote_tag(option_name: &str) -> TagKey {
    let slug = option_name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect::<String>();
    TagKey::new(format!("voted_for:{}", slug))
}

/// Who chooses how `voter` votes: the player an effect such as Illusion of
/// Choice ("You choose how each player votes this turn") names, else the
/// voter. The voter still casts the vote and decides whether to vote an
/// additional time.
fn vote_chooser(game: &GameState, voter: PlayerId) -> PlayerId {
    game.vote_controller_this_turn().unwrap_or(voter)
}

fn active_players_in_vote_order(game: &GameState, controller: PlayerId) -> Vec<PlayerId> {
    let mut players: Vec<PlayerId> = game
        .players
        .iter()
        .filter(|player| player.is_in_game())
        .map(|player| player.id)
        .collect();

    if let Some(controller_pos) = players
        .iter()
        .position(|&player_id| player_id == controller)
    {
        players.rotate_left(controller_pos);
    }

    players
}

fn build_display_options(effect: &VoteEffect) -> Vec<DisplayOption> {
    let VoteChoice::NamedOptions(options) = &effect.choice else {
        return Vec::new();
    };
    options
        .iter()
        .enumerate()
        .map(|(index, option)| DisplayOption::new(index, &option.name))
        .collect()
}

fn additional_vote_modifiers_from_static_abilities(
    game: &GameState,
    player_id: PlayerId,
) -> (u32, u32) {
    game.battlefield
        .iter()
        .filter_map(|&id| game.object(id))
        .filter(|obj| game.controller_of(obj) == player_id)
        // Current characteristics: an ability lost to Humility or gained
        // from an effect counts as it is now (CR 613.1f).
        .flat_map(|obj| {
            game.calculated_characteristics_arc(obj.id)
                .map(|calc| calc.static_abilities.to_vec())
                .unwrap_or_default()
        })
        .fold((0u32, 0u32), |(mandatory, optional), ability| {
            (
                mandatory.saturating_add(ability.additional_votes_while_voting()),
                optional.saturating_add(ability.optional_additional_votes_while_voting()),
            )
        })
}

fn candidate_object_ids_for_vote(
    game: &GameState,
    filter: &crate::filter::ObjectFilter,
    ctx: &ExecutionContext,
) -> Vec<ObjectId> {
    let filter_ctx = ctx.filter_context(game);
    let zone = filter.zone.unwrap_or(Zone::Battlefield);
    game.zone_ids(zone)
        .filter_map(|id| game.object(id).map(|obj| (id, obj)))
        .filter(|(_, obj)| filter.matches(obj, &filter_ctx, game))
        .map(|(id, _)| id)
        .collect()
}

fn vote_instances_for_player(
    effect: &VoteEffect,
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    player_id: PlayerId,
) -> usize {
    let mut num_votes = 1usize;
    if player_id == ctx.controller {
        num_votes += effect.controller_extra_votes as usize;
        for _ in 0..effect.controller_optional_extra_votes {
            let wants_extra = make_boolean_decision(
                game,
                &mut ctx.decision_maker,
                player_id,
                ctx.source,
                "vote an additional time",
                FallbackStrategy::Decline,
            );
            if wants_extra {
                num_votes += 1;
            }
        }
    }

    let (battlefield_mandatory, battlefield_optional) =
        additional_vote_modifiers_from_static_abilities(game, player_id);
    num_votes += battlefield_mandatory as usize;
    for _ in 0..battlefield_optional {
        let wants_extra = make_boolean_decision(
            game,
            &mut ctx.decision_maker,
            player_id,
            ctx.source,
            "vote an additional time",
            FallbackStrategy::Decline,
        );
        if wants_extra {
            num_votes += 1;
        }
    }

    num_votes
}

fn snapshots_for_objects(game: &GameState, object_ids: &[ObjectId]) -> Vec<ObjectSnapshot> {
    object_ids
        .iter()
        .filter_map(|&id| {
            game.object(id)
                .map(|obj| ObjectSnapshot::from_object(obj, game))
        })
        .collect()
}

fn collect_votes(
    effect: &VoteEffect,
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    players: &[PlayerId],
    display_options: &[DisplayOption],
) -> Option<(Vec<PlayerVote>, Vec<usize>)> {
    let VoteChoice::NamedOptions(options) = &effect.choice else {
        return None;
    };
    let mut votes: Vec<PlayerVote> = Vec::new();
    let mut vote_counts: Vec<usize> = vec![0; options.len()];

    for &player_id in players {
        let num_votes = vote_instances_for_player(effect, game, ctx, player_id);
        for _ in 0..num_votes {
            let spec = ChoiceSpec::single(ctx.source, display_options.to_vec());
            let chooser = vote_chooser(game, player_id);
            let chosen = make_decision(
                game,
                &mut ctx.decision_maker,
                chooser,
                Some(ctx.source),
                spec,
            );
            if ctx.decision_maker.awaiting_choice() {
                return None;
            }

            if let Some(&vote_index) = chosen.first()
                && vote_index < vote_counts.len()
            {
                vote_counts[vote_index] += 1;
                votes.push(PlayerVote {
                    player: player_id,
                    option_index: vote_index,
                    option_name: options[vote_index].name.to_string(),
                    object_vote: None,
                });
            }
        }
    }

    Some((votes, vote_counts))
}

fn collect_object_votes(
    effect: &VoteEffect,
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    players: &[PlayerId],
) -> Option<(Vec<PlayerVote>, HashMap<ObjectId, usize>)> {
    let VoteChoice::Objects { filter, count } = &effect.choice else {
        return None;
    };

    let candidates = candidate_object_ids_for_vote(game, filter, ctx);

    let min = count.min;
    let max = count.max;
    let mut votes: Vec<PlayerVote> = Vec::new();
    let mut vote_counts: HashMap<ObjectId, usize> = HashMap::new();

    for &player_id in players {
        let num_votes = vote_instances_for_player(effect, game, ctx, player_id);
        for _ in 0..num_votes {
            let spec = ChooseObjectsSpec::new(
                ctx.source,
                "Choose an object to vote for",
                candidates.clone(),
                min,
                max,
            )
            .allow_partial_completion();
            let chooser = vote_chooser(game, player_id);
            let chosen = make_decision(
                game,
                &mut ctx.decision_maker,
                chooser,
                Some(ctx.source),
                spec,
            );
            if ctx.decision_maker.awaiting_choice() {
                return None;
            }

            for object_id in chosen {
                let Some(object) = game.object(object_id) else {
                    continue;
                };
                *vote_counts.entry(object_id).or_default() += 1;
                votes.push(PlayerVote {
                    player: player_id,
                    option_index: object_id.0 as usize,
                    option_name: object.name.to_string(),
                    object_vote: Some(object_id),
                });
            }
        }
    }

    Some((votes, vote_counts))
}

fn candidate_player_ids_for_vote(
    effect: &VoteEffect,
    game: &GameState,
    ctx: &mut ExecutionContext,
    voter: PlayerId,
) -> Result<Vec<PlayerId>, ExecutionError> {
    let VoteChoice::Players {
        filter,
        exclude_voter,
    } = &effect.choice
    else {
        return Ok(Vec::new());
    };

    let mut candidates = ctx.with_temp_iterated_player(Some(voter), |ctx| {
        let filter_ctx = ctx.filter_context(game);
        resolve_player_filter_to_list(game, filter, &filter_ctx, ctx)
    })?;
    if *exclude_voter {
        candidates.retain(|player| *player != voter);
    }
    candidates.sort_by_key(|player| player.0);
    candidates.dedup();
    Ok(candidates)
}

fn collect_player_votes(
    effect: &VoteEffect,
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    players: &[PlayerId],
) -> Result<Option<(Vec<PlayerVote>, HashMap<PlayerId, usize>)>, ExecutionError> {
    if !matches!(effect.choice, VoteChoice::Players { .. }) {
        return Ok(None);
    }

    let mut votes: Vec<PlayerVote> = Vec::new();
    let mut vote_counts: HashMap<PlayerId, usize> = HashMap::new();

    for &player_id in players {
        let num_votes = vote_instances_for_player(effect, game, ctx, player_id);
        for _ in 0..num_votes {
            let candidates = candidate_player_ids_for_vote(effect, game, ctx, player_id)?;
            let options = candidates
                .iter()
                .filter_map(|candidate| {
                    game.player(*candidate)
                        .map(|player| (player.name.to_string(), *candidate))
                })
                .collect::<Vec<_>>();
            let chooser = vote_chooser(game, player_id);
            let Some(chosen) = (!options.is_empty())
                .then(|| {
                    crate::decisions::ask_choose_one(
                        game,
                        &mut ctx.decision_maker,
                        chooser,
                        ctx.source,
                        &options,
                    )
                })
                .flatten()
            else {
                continue;
            };
            if ctx.decision_maker.awaiting_choice() {
                return Ok(None);
            }

            let option_name = game
                .player(chosen)
                .map(|player| player.name.to_string())
                .unwrap_or_else(|| "player".to_string());
            *vote_counts.entry(chosen).or_default() += 1;
            votes.push(PlayerVote {
                player: player_id,
                option_index: chosen.0 as usize,
                option_name,
                object_vote: None,
            });
        }
    }

    Ok(Some((votes, vote_counts)))
}

fn build_vote_counts_map(vote_counts: &[usize]) -> HashMap<usize, usize> {
    vote_counts
        .iter()
        .enumerate()
        .filter(|(_, count)| **count > 0)
        .map(|(idx, count)| (idx, *count))
        .collect()
}

fn build_option_voter_tags(
    effect: &VoteEffect,
    votes: &[PlayerVote],
) -> HashMap<TagKey, Vec<PlayerId>> {
    let VoteChoice::NamedOptions(options) = &effect.choice else {
        return HashMap::new();
    };
    let mut option_tags: HashMap<TagKey, Vec<PlayerId>> = HashMap::new();

    for (option_index, option) in options.iter().enumerate() {
        let mut voters: Vec<PlayerId> = votes
            .iter()
            .filter(|vote| vote.option_index == option_index)
            .map(|vote| vote.player)
            .collect();

        if voters.is_empty() {
            continue;
        }

        voters.sort_by_key(|player| player.0);
        voters.dedup();
        option_tags.insert(option_vote_tag(&option.name), voters);
    }

    option_tags
}

#[derive(Debug, Clone)]
struct PublishVotingFacts(TriggerEvent);

impl crate::effects::EffectExecutor for PublishVotingFacts {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        if ctx.decision_maker.awaiting_choice() {
            return Ok(EffectOutcome::count(0));
        }
        let mut event = self.0.clone();
        let provenance = game.alloc_child_event_provenance(event.provenance(), event.kind());
        event.set_provenance(provenance);
        if let Some(batch) = game.simultaneous_action_batch() {
            event = event.with_simultaneous_batch(batch);
        }
        Ok(EffectOutcome::resolved().with_event(event))
    }
}

fn queue_vote_events(
    effect: &VoteEffect,
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    votes: &[PlayerVote],
    vote_counts: HashMap<usize, usize>,
) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
    let option_names: Vec<String> = match &effect.choice {
        VoteChoice::NamedOptions(options) => options
            .iter()
            .map(|option| option.name.to_string())
            .collect(),
        VoteChoice::Objects { .. } | VoteChoice::Players { .. } => {
            votes.iter().map(|vote| vote.option_name.clone()).collect()
        }
    };
    let voting_event = PlayersFinishedVotingEvent::new(
        ctx.source,
        ctx.controller,
        votes.to_vec(),
        vote_counts,
        option_names,
    )
    .with_player_tags(build_option_voter_tags(effect, votes));
    let voter_teams: Vec<(PlayerId, usize)> = game
        .players
        .iter()
        .filter_map(|player| game.team_index_for(player.id).map(|team| (player.id, team)))
        .collect();
    let voting_event = voting_event.with_voter_teams(voter_teams.clone());

    let vote_action_event = KeywordActionEvent::new(
        KeywordActionKind::Vote,
        ctx.controller,
        ctx.source,
        votes.len() as u32,
    )
    .with_votes(votes.to_vec())
    .with_voter_teams(voter_teams)
    .with_player_tags(
        voting_event
            .player_tags
            .iter()
            .filter_map(|(tag, players)| {
                if tag.as_str() == "voted_with_you" || tag.as_str() == "voted_against_you" {
                    None
                } else {
                    Some((tag.clone(), players.clone()))
                }
            })
            .collect(),
    );

    let source_snapshot =
        ObjectSnapshot::from_object_id(game, ctx.source).or_else(|| ctx.source_snapshot.clone());
    let keyword = super::publish_keyword_action_completion_receipt(
        game,
        ctx,
        TriggerEvent::new_with_provenance(
            vote_action_event.with_snapshot(source_snapshot.clone()),
            ctx.provenance,
        ),
    )?;
    if ctx.decision_maker.awaiting_choice() {
        return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
            EffectOutcome::count(0),
        ));
    }
    let mut voting_event = TriggerEvent::new_with_provenance(voting_event, ctx.provenance);
    if let Some(snapshot) = source_snapshot {
        voting_event = voting_event.with_source_snapshot(snapshot);
    }
    let facts = PublishVotingFacts(voting_event).execute_child_with_outputs(game, ctx)?;
    if ctx.decision_maker.awaiting_choice() {
        return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
            EffectOutcome::count(0),
        ));
    }
    let mut completion = crate::effects::CompletedEffectOutputs::from_children(
        [keyword, facts],
        EffectOutcome::aggregate,
    );
    crate::effects::capture_triggers_before_added_program(
        game,
        ctx,
        None,
        completion.outcome.events.iter_mut(),
    )?;
    // Retain the existing queued-notification protocol; returned aliases carry
    // the same occurrence and matching receipt, so publication cannot duplicate
    // the actual observation when the enclosing instruction reports its result.
    completion.synchronize_observations();
    for event in &completion.outcome.events {
        game.queue_trigger_event(event.provenance(), event.clone());
    }
    Ok(completion)
}

fn execute_vote_payloads(
    effect: &VoteEffect,
    votes: &[PlayerVote],
    game: &mut GameState,
    ctx: &mut ExecutionContext,
) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
    if !effect.payloads.is_empty() {
        let mut outcomes = Vec::new();
        for payload in &effect.payloads {
            let outcome = match payload {
                ironsmith_core::VotePayload::Effects(effects) => {
                    crate::effects::SequenceEffect::new(effects.clone())
                        .execute_child_with_outputs(game, ctx)?
                }
                ironsmith_core::VotePayload::ForEachVote { option, effects } => {
                    // Keep every occurrence and its voter binding. Set-valued
                    // voter tags cannot represent a player's additional votes.
                    let players = votes
                        .iter()
                        .filter(|vote| vote.option_name.eq_ignore_ascii_case(option))
                        .map(|vote| vote.player)
                        .collect();
                    super::execute_player_occurrences_with_outputs(effects, players, game, ctx)?
                }
            };
            if ctx.decision_maker.awaiting_choice() {
                return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                    EffectOutcome::count(0),
                ));
            }
            outcomes.push(outcome);
        }
        return Ok(crate::effects::CompletedEffectOutputs::from_children(
            outcomes,
            EffectOutcome::aggregate,
        ));
    }
    let VoteChoice::NamedOptions(options) = &effect.choice else {
        return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
            EffectOutcome::resolved(),
        ));
    };
    let mut outcomes = Vec::new();
    // Older option-local programs have no separate clause schedule. Preserve
    // option order while delegating their actual participant action execution.
    for (option_index, option) in options.iter().enumerate() {
        let players = votes
            .iter()
            .filter(|vote| vote.option_index == option_index)
            .map(|vote| vote.player)
            .collect();
        outcomes.push(super::execute_player_occurrences_with_outputs(
            &option.effects_per_vote,
            players,
            game,
            ctx,
        )?);
        if ctx.decision_maker.awaiting_choice() {
            return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                EffectOutcome::count(0),
            ));
        }
    }
    Ok(crate::effects::CompletedEffectOutputs::from_children(
        outcomes,
        EffectOutcome::aggregate,
    ))
}

pub(crate) fn run_vote(
    effect: &VoteEffect,
    game: &mut GameState,
    ctx: &mut ExecutionContext,
) -> Result<EffectOutcome, ExecutionError> {
    run_vote_with_outputs(effect, game, ctx)
        .map(crate::effects::CompletedEffectOutputs::into_outcome)
}

pub(crate) fn run_vote_with_outputs(
    effect: &VoteEffect,
    game: &mut GameState,
    ctx: &mut ExecutionContext,
) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
    super::execute_transaction(
        game,
        ctx,
        || crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
        |game, ctx| {
            let starting_player = if effect.starting_with_controller {
                ctx.controller
            } else {
                game.turn.active_player
            };
            let players = active_players_in_vote_order(game, starting_player);
            match &effect.choice {
                VoteChoice::NamedOptions(options) => {
                    if options.is_empty() {
                        return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                            EffectOutcome::resolved(),
                        ));
                    }
                    let display_options = build_display_options(effect);
                    let Some((votes, vote_counts)) =
                        collect_votes(effect, game, ctx, &players, &display_options)
                    else {
                        return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                            EffectOutcome::count(0),
                        ));
                    };
                    let vote_counts_map = build_vote_counts_map(&vote_counts);
                    let mut result = VoteResult::default();
                    result.total_votes = votes.len();
                    for (idx, count) in &vote_counts_map {
                        if let Some(option) = options.get(*idx) {
                            result.option_counts.insert(option.name.to_string(), *count);
                        }
                    }
                    ctx.vote_results.insert(ctx.source, result);
                    ctx.clear_object_tag(VOTE_WINNERS_TAG);
                    ctx.clear_object_tag(VOTED_OBJECTS_TAG);
                    let completion = queue_vote_events(effect, game, ctx, &votes, vote_counts_map)?;
                    if ctx.decision_maker.awaiting_choice() {
                        return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                            EffectOutcome::count(0),
                        ));
                    }
                    let payload = execute_vote_payloads(effect, &votes, game, ctx)?;
                    let primary = payload.outcome.summary_projection();
                    Ok(crate::effects::CompletedEffectOutputs::with_primary_result(
                        primary,
                        [completion, payload],
                    ))
                }
                VoteChoice::Objects { .. } => {
                    let Some((votes, object_vote_counts)) =
                        collect_object_votes(effect, game, ctx, &players)
                    else {
                        return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                            EffectOutcome::count(0),
                        ));
                    };

                    let max_votes = object_vote_counts.values().copied().max().unwrap_or(0);
                    // Sorted so the tagged-object order is identical on every peer
                    // (the counts map is a std HashMap with per-instance iteration order).
                    let mut winning_objects: Vec<ObjectId> = object_vote_counts
                        .iter()
                        .filter_map(|(object_id, count)| {
                            (*count == max_votes && *count > 0).then_some(*object_id)
                        })
                        .collect();
                    winning_objects.sort_unstable();
                    let mut voted_objects: Vec<ObjectId> =
                        object_vote_counts.keys().copied().collect();
                    voted_objects.sort_unstable();
                    ctx.set_tagged_objects(
                        VOTED_OBJECTS_TAG,
                        snapshots_for_objects(game, &voted_objects),
                    );
                    ctx.set_tagged_objects(
                        VOTE_WINNERS_TAG,
                        snapshots_for_objects(game, &winning_objects),
                    );

                    let mut result = VoteResult::default();
                    result.total_votes = votes.len();
                    result.object_counts = object_vote_counts.clone();
                    ctx.vote_results.insert(ctx.source, result);

                    let mut vote_counts_map: HashMap<usize, usize> = HashMap::new();
                    for (object_id, count) in object_vote_counts {
                        vote_counts_map.insert(object_id.0 as usize, count);
                    }
                    queue_vote_events(effect, game, ctx, &votes, vote_counts_map)
                }
                VoteChoice::Players { .. } => {
                    let Some((votes, player_vote_counts)) =
                        collect_player_votes(effect, game, ctx, &players)?
                    else {
                        return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                            EffectOutcome::count(0),
                        ));
                    };

                    let mut result = VoteResult::default();
                    result.total_votes = votes.len();
                    result.player_counts = player_vote_counts.clone();
                    ctx.vote_results.insert(ctx.source, result);

                    let mut vote_counts_map: HashMap<usize, usize> = HashMap::new();
                    for (player_id, count) in player_vote_counts {
                        vote_counts_map.insert(player_id.0 as usize, count);
                    }
                    queue_vote_events(effect, game, ctx, &votes, vote_counts_map)
                }
            }
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::Ability;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::decision::SelectFirstDecisionMaker;
    use crate::effect::ChoiceCount;
    use crate::effects::VoteOption;
    use crate::filter::ObjectFilter;
    use crate::ids::CardId;
    use crate::static_abilities::StaticAbility;
    use crate::types::CardType;
    use crate::zone::Zone;

    fn creature_card(id: u32, name: &str) -> crate::card::Card {
        CardBuilder::new(CardId::from_raw(id), name)
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build()
    }

    #[test]
    fn test_active_players_in_vote_order_starts_with_controller() {
        let game = GameState::new(
            vec![
                "Alice".to_string(),
                "Bob".to_string(),
                "Charlie".to_string(),
            ],
            20,
        );
        let controller = PlayerId::from_index(1);

        let order = active_players_in_vote_order(&game, controller);
        assert_eq!(
            order,
            vec![
                PlayerId::from_index(1),
                PlayerId::from_index(2),
                PlayerId::from_index(0),
            ]
        );
    }

    #[test]
    fn test_build_vote_counts_map_drops_zero_counts() {
        let vote_counts = vec![2, 0, 3];
        let map = build_vote_counts_map(&vote_counts);
        assert_eq!(map.len(), 2);
        assert_eq!(map.get(&0), Some(&2usize));
        assert_eq!(map.get(&2), Some(&3usize));
        assert_eq!(map.get(&1), None);
    }

    #[test]
    fn test_option_vote_tag_slugifies_name() {
        let tag = option_vote_tag("Evidence / Bribery!");
        assert_eq!(tag.as_str(), "voted_for:evidence___bribery_");
    }

    #[test]
    fn vote_runtime_records_object_vote_winners_and_voted_objects() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let first = game.create_object_from_card(
            &creature_card(91_001, "First Candidate"),
            alice,
            Zone::Battlefield,
        );
        let _second = game.create_object_from_card(
            &creature_card(91_002, "Second Candidate"),
            bob,
            Zone::Battlefield,
        );

        let vote = VoteEffect::vote_objects(ObjectFilter::creature(), ChoiceCount::exactly(1), 0);
        let source = game.new_object_id();
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new_default(source, alice).with_decision_maker(&mut dm);

        let outcome = run_vote(&vote, &mut game, &mut ctx).expect("object vote should resolve");
        assert!(outcome.status.is_success());

        let result = ctx
            .vote_results
            .get(&source)
            .expect("vote result should be stored");
        assert_eq!(result.total_votes, 2);
        assert_eq!(result.object_counts.get(&first), Some(&2usize));

        let winners = ctx
            .get_tagged_all(VOTE_WINNERS_TAG)
            .expect("winning objects should be tagged");
        assert_eq!(winners.len(), 1);
        assert_eq!(winners[0].object_id, first);

        let voted = ctx
            .get_tagged_all(VOTED_OBJECTS_TAG)
            .expect("voted objects should be tagged");
        assert_eq!(voted.len(), 1);
        assert_eq!(voted[0].object_id, first);
    }

    #[test]
    fn vote_runtime_counts_battlefield_additional_votes() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bonus_source = game.create_object_from_card(
            &creature_card(91_010, "Brago Proxy"),
            alice,
            Zone::Battlefield,
        );
        game.object_mut(bonus_source)
            .expect("bonus permanent should exist")
            .abilities_mut()
            .push(Ability::static_ability(
                StaticAbility::vote_additional_vote_while_voting(),
            ));

        let vote = VoteEffect::basic(vec![
            VoteOption::new("evidence", vec![]),
            VoteOption::new("bribery", vec![]),
        ]);
        let source = game.new_object_id();
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new_default(source, alice).with_decision_maker(&mut dm);

        let outcome = run_vote(&vote, &mut game, &mut ctx).expect("vote should resolve");
        assert!(outcome.status.is_success());

        let result = ctx
            .vote_results
            .get(&source)
            .expect("vote result should be stored");
        assert_eq!(result.total_votes, 3);
        assert_eq!(result.count_for_option("evidence"), 3);
        assert!(result.option_gets_more_votes("evidence"));
    }
}
