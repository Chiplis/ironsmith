//! Runtime orchestration for `VoteEffect`.

use std::collections::{BTreeMap, HashMap};

use crate::decision::FallbackStrategy;
use crate::decisions::spec::DisplayOption;
use crate::decisions::specs::ChoiceSpec;
use crate::decisions::{make_boolean_decision, make_decision};
use crate::effect::EffectOutcome;
use crate::effects::InvestigateEffect;
use crate::events::{
    EventCause, EventKind, KeywordActionEvent, KeywordActionKind, PlayerVote,
    PlayersFinishedVotingEvent, ZoneChangeEvent,
};
use crate::executor::{ExecutionContext, ExecutionError, execute_effect};
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::object::ObjectKind;
use crate::tag::TagKey;
use crate::triggers::TriggerEvent;
use crate::zone::Zone;

use super::vote::VoteEffect;

type TokenBatchByController = BTreeMap<PlayerId, Vec<ObjectId>>;

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
    effect
        .options
        .iter()
        .enumerate()
        .map(|(index, option)| DisplayOption::new(index, &option.name))
        .collect()
}

fn collect_votes(
    effect: &VoteEffect,
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    players: &[PlayerId],
    display_options: &[DisplayOption],
) -> (Vec<PlayerVote>, Vec<usize>) {
    let controller = ctx.controller;
    let mut votes: Vec<PlayerVote> = Vec::new();
    let mut vote_counts: Vec<usize> = vec![0; effect.options.len()];

    for player_id in players {
        let mut num_votes = 1usize;
        if *player_id == controller {
            num_votes += effect.controller_extra_votes as usize;
            for _ in 0..effect.controller_optional_extra_votes {
                let wants_extra = make_boolean_decision(
                    game,
                    &mut ctx.decision_maker,
                    *player_id,
                    ctx.source,
                    "vote an additional time",
                    FallbackStrategy::Decline,
                );
                if wants_extra {
                    num_votes += 1;
                }
            }
        }

        for _ in 0..num_votes {
            let spec = ChoiceSpec::single(ctx.source, display_options.to_vec());
            let chosen = make_decision(
                game,
                &mut ctx.decision_maker,
                *player_id,
                Some(ctx.source),
                spec,
            );

            if let Some(&vote_index) = chosen.first()
                && vote_index < vote_counts.len()
            {
                vote_counts[vote_index] += 1;
                votes.push(PlayerVote {
                    player: *player_id,
                    option_index: vote_index,
                    option_name: effect.options[vote_index].name.clone(),
                });
            }
        }
    }

    (votes, vote_counts)
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
    let mut option_tags: HashMap<TagKey, Vec<PlayerId>> = HashMap::new();

    for (option_index, option) in effect.options.iter().enumerate() {
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

fn queue_vote_events(
    effect: &VoteEffect,
    game: &mut GameState,
    ctx: &ExecutionContext,
    votes: &[PlayerVote],
    vote_counts: HashMap<usize, usize>,
) {
    let option_names: Vec<String> = effect
        .options
        .iter()
        .map(|option| option.name.clone())
        .collect();
    let voting_event = PlayersFinishedVotingEvent::new(
        ctx.source,
        ctx.controller,
        votes.to_vec(),
        vote_counts,
        option_names,
    )
    .with_player_tags(build_option_voter_tags(effect, votes));

    let vote_action_event = KeywordActionEvent::new(
        KeywordActionKind::Vote,
        ctx.controller,
        ctx.source,
        votes.len() as u32,
    )
    .with_votes(votes.to_vec())
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

    game.queue_trigger_event(
        ctx.provenance,
        TriggerEvent::new_with_provenance(vote_action_event, ctx.provenance),
    );
    game.queue_trigger_event(
        ctx.provenance,
        TriggerEvent::new_with_provenance(voting_event, ctx.provenance),
    );
}

fn collect_token_batch(
    game: &GameState,
    outcome: &mut EffectOutcome,
    by_controller: &mut TokenBatchByController,
) {
    if outcome.events.is_empty() {
        return;
    }

    let mut filtered_events = Vec::with_capacity(outcome.events.len());

    for event in outcome.events.drain(..) {
        if event.kind() == EventKind::ZoneChange
            && let Some(zone_change) = event.downcast::<ZoneChangeEvent>()
            && zone_change.to == Zone::Battlefield
            && zone_change.objects.iter().all(|&object_id| {
                game.object(object_id)
                    .map(|object| matches!(object.kind, ObjectKind::Token))
                    .unwrap_or(false)
            })
        {
            for &object_id in &zone_change.objects {
                if let Some(object) = game.object(object_id) {
                    by_controller
                        .entry(object.controller)
                        .or_default()
                        .push(object_id);
                }
            }
            continue;
        }

        filtered_events.push(event);
    }

    outcome.events = filtered_events;
}

fn append_batched_token_events(
    outcome: &mut EffectOutcome,
    cause: EventCause,
    token_batches: Vec<TokenBatchByController>,
    provenance: crate::provenance::ProvNodeId,
) {
    for by_controller in token_batches {
        for (_controller, mut object_ids) in by_controller {
            if object_ids.is_empty() {
                continue;
            }

            object_ids.sort();
            object_ids.dedup();
            outcome.events.push(TriggerEvent::new_with_provenance(
                ZoneChangeEvent::batch(object_ids, Zone::Stack, Zone::Battlefield, cause.clone()),
                provenance,
            ));
        }
    }
}

fn execute_vote_payloads(
    effect: &VoteEffect,
    votes: &[PlayerVote],
    game: &mut GameState,
    ctx: &mut ExecutionContext,
) -> Result<EffectOutcome, ExecutionError> {
    let mut outcomes = Vec::new();
    let mut token_batches: Vec<TokenBatchByController> =
        vec![BTreeMap::new(); effect.options.len()];

    for vote in votes {
        if let Some(option) = effect.options.get(vote.option_index) {
            ctx.with_temp_iterated_player(Some(vote.player), |ctx| {
                for vote_effect in &option.effects_per_vote {
                    let is_investigate = vote_effect.downcast_ref::<InvestigateEffect>().is_some();
                    let mut outcome = execute_effect(game, vote_effect, ctx)?;

                    if !is_investigate {
                        let batch = token_batches
                            .get_mut(vote.option_index)
                            .expect("vote option index should be valid");
                        collect_token_batch(game, &mut outcome, batch);
                    }

                    outcomes.push(outcome);
                }
                Ok::<(), ExecutionError>(())
            })?;
        }
    }

    let mut aggregate = EffectOutcome::aggregate(outcomes);
    let cause = EventCause::from_effect(ctx.source, ctx.controller);
    append_batched_token_events(&mut aggregate, cause, token_batches, ctx.provenance);
    Ok(aggregate)
}

pub(crate) fn run_vote(
    effect: &VoteEffect,
    game: &mut GameState,
    ctx: &mut ExecutionContext,
) -> Result<EffectOutcome, ExecutionError> {
    if effect.options.is_empty() {
        return Ok(EffectOutcome::resolved());
    }

    let players = active_players_in_vote_order(game, ctx.controller);
    let display_options = build_display_options(effect);
    let (votes, vote_counts) = collect_votes(effect, game, ctx, &players, &display_options);
    let vote_counts = build_vote_counts_map(&vote_counts);

    queue_vote_events(effect, game, ctx, &votes, vote_counts);
    execute_vote_payloads(effect, &votes, game, ctx)
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
