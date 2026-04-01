//! Runtime orchestration for `VoteEffect`.

use std::collections::{BTreeMap, HashMap};

use crate::decision::FallbackStrategy;
use crate::decisions::spec::DisplayOption;
use crate::decisions::specs::{ChoiceSpec, ChooseObjectsSpec};
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
use crate::snapshot::ObjectSnapshot;
use crate::tag::TagKey;
use crate::triggers::TriggerEvent;
use crate::zone::Zone;

use super::vote::{VOTE_WINNERS_TAG, VOTED_OBJECTS_TAG, VoteChoice, VoteEffect, VoteResult};

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
        .filter(|obj| obj.controller == player_id)
        .flat_map(|obj| obj.abilities.iter())
        .filter_map(|ability| match &ability.kind {
            crate::ability::AbilityKind::Static(static_ability) => Some(static_ability),
            _ => None,
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
    let candidate_ids: Vec<ObjectId> = match filter.zone {
        Some(Zone::Battlefield) => game.battlefield.clone(),
        Some(Zone::Graveyard) => game
            .players
            .iter()
            .flat_map(|player| player.graveyard.iter().copied())
            .collect(),
        Some(Zone::Hand) => game
            .players
            .iter()
            .flat_map(|player| player.hand.iter().copied())
            .collect(),
        Some(Zone::Library) => game
            .players
            .iter()
            .flat_map(|player| player.library.iter().copied())
            .collect(),
        Some(Zone::Stack) => game.stack.iter().map(|entry| entry.object_id).collect(),
        Some(Zone::Exile) => game.exile.clone(),
        Some(Zone::Command) => game.command_zone.clone(),
        None => game.battlefield.clone(),
    };

    candidate_ids
        .iter()
        .filter_map(|&id| game.object(id).map(|obj| (id, obj)))
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
            let chosen = make_decision(
                game,
                &mut ctx.decision_maker,
                player_id,
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
                    option_name: options[vote_index].name.clone(),
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
            let chosen = make_decision(
                game,
                &mut ctx.decision_maker,
                player_id,
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
                    option_name: object.name.clone(),
                    object_vote: Some(object_id),
                });
            }
        }
    }

    Some((votes, vote_counts))
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

fn queue_vote_events(
    effect: &VoteEffect,
    game: &mut GameState,
    ctx: &ExecutionContext,
    votes: &[PlayerVote],
    vote_counts: HashMap<usize, usize>,
) {
    let option_names: Vec<String> = match &effect.choice {
        VoteChoice::NamedOptions(options) => {
            options.iter().map(|option| option.name.clone()).collect()
        }
        VoteChoice::Objects { .. } => votes.iter().map(|vote| vote.option_name.clone()).collect(),
    };
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
    let VoteChoice::NamedOptions(options) = &effect.choice else {
        return Ok(EffectOutcome::resolved());
    };
    let mut outcomes = Vec::new();
    let mut token_batches: Vec<TokenBatchByController> = vec![BTreeMap::new(); options.len()];

    for vote in votes {
        if let Some(option) = options.get(vote.option_index) {
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
    let players = active_players_in_vote_order(game, ctx.controller);
    match &effect.choice {
        VoteChoice::NamedOptions(options) => {
            if options.is_empty() {
                return Ok(EffectOutcome::resolved());
            }
            let display_options = build_display_options(effect);
            let Some((votes, vote_counts)) =
                collect_votes(effect, game, ctx, &players, &display_options)
            else {
                return Ok(EffectOutcome::count(0));
            };
            let vote_counts_map = build_vote_counts_map(&vote_counts);
            let mut result = VoteResult::default();
            result.total_votes = votes.len();
            for (idx, count) in &vote_counts_map {
                if let Some(option) = options.get(*idx) {
                    result.option_counts.insert(option.name.clone(), *count);
                }
            }
            ctx.vote_results.insert(ctx.source, result);
            ctx.clear_object_tag(VOTE_WINNERS_TAG);
            ctx.clear_object_tag(VOTED_OBJECTS_TAG);
            queue_vote_events(effect, game, ctx, &votes, vote_counts_map);
            execute_vote_payloads(effect, &votes, game, ctx)
        }
        VoteChoice::Objects { .. } => {
            let Some((votes, object_vote_counts)) =
                collect_object_votes(effect, game, ctx, &players)
            else {
                return Ok(EffectOutcome::count(0));
            };

            let max_votes = object_vote_counts.values().copied().max().unwrap_or(0);
            let winning_objects: Vec<ObjectId> = object_vote_counts
                .iter()
                .filter_map(|(object_id, count)| {
                    (*count == max_votes && *count > 0).then_some(*object_id)
                })
                .collect();
            let voted_objects: Vec<ObjectId> = object_vote_counts.keys().copied().collect();
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
            queue_vote_events(effect, game, ctx, &votes, vote_counts_map);
            Ok(EffectOutcome::resolved())
        }
    }
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
            .abilities
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
