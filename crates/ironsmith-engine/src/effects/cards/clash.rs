//! Clash effect implementation.
//!
//! Clash (701.30): You and an opponent each reveal the top card of your library,
//! then each may put that card on the bottom of their library. A player wins if
//! their revealed card has greater mana value.

use crate::decisions::{
    ChoiceSpec, DisplayOption, context::ViewCardsContext, make_decision,
};
use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::{ExecutionContext, ExecutionError, ResolvedTarget};
use crate::events::{CardRevealedEvent, KeywordActionEvent, KeywordActionKind};
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::snapshot::ObjectSnapshot;
use crate::tag::TagKey;
use crate::zone::Zone;
use std::collections::HashMap;

pub use ironsmith_core::ClashOpponentMode;

/// Effect that performs a clash with an opponent.
pub type ClashEffect = ironsmith_core::ClashEffect;

fn in_game_opponents(game: &GameState, controller: PlayerId) -> Vec<PlayerId> {
    game.players
        .iter()
        .filter(|player| {
            player.is_in_game()
                && game.are_opponents(controller, player.id)
                && game.player_is_within_range(controller, player.id)
        })
        .map(|player| player.id)
        .collect()
}

fn targeted_opponent(ctx: &ExecutionContext, opponents: &[PlayerId]) -> Option<PlayerId> {
    ctx.targets.iter().find_map(|target| match target {
        ResolvedTarget::Player(player) if opponents.contains(player) => Some(*player),
        _ => None,
    })
}

fn choose_opponent(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    opponents: &[PlayerId],
    mode: ClashOpponentMode,
) -> Option<PlayerId> {
    match mode {
        ClashOpponentMode::TargetOpponent => {
            return targeted_opponent(ctx, opponents);
        }
        ClashOpponentMode::DefendingPlayer => {
            return ctx
                .combat
                .defending_player
                .filter(|player_id| opponents.contains(player_id));
        }
        ClashOpponentMode::AnyOpponent => {}
    }

    if opponents.is_empty() {
        return None;
    }
    if opponents.len() == 1 {
        return opponents.first().copied();
    }

    let options: Vec<DisplayOption> = opponents
        .iter()
        .enumerate()
        .map(|(index, player_id)| {
            let name = game
                .player(*player_id)
                .map(|player| player.name.to_string())
                .unwrap_or_else(|| format!("Player {}", player_id.0));
            DisplayOption::new(index, name)
        })
        .collect();

    let spec = ChoiceSpec::single(ctx.source, options);
    let chosen = make_decision(
        game,
        &mut ctx.decision_maker,
        ctx.controller,
        Some(ctx.source),
        spec,
    );
    if ctx.decision_maker.awaiting_choice() {
        return None;
    }

    chosen
        .first()
        .copied()
        .and_then(|index| opponents.get(index).copied())
}

fn top_card(game: &GameState, player: PlayerId) -> Option<ObjectId> {
    game.player(player)
        .and_then(|entry| entry.library.last().copied())
}

fn clashing_players_in_apnap_order(
    game: &GameState,
    controller: PlayerId,
    opponent: PlayerId,
) -> Vec<PlayerId> {
    if game.turn_store.turn_order.is_empty() {
        return vec![controller, opponent];
    }

    let participants = [controller, opponent];
    let mut ordered = Vec::new();
    for player_id in game.team_apnap_player_order() {
        if participants.contains(&player_id) && !ordered.contains(&player_id) {
            ordered.push(player_id);
        }
    }
    if ordered.len() < participants.len() {
        for player_id in participants {
            if !ordered.contains(&player_id) {
                ordered.push(player_id);
            }
        }
    }
    ordered
}

fn card_mana_value(game: &GameState, card: Option<ObjectId>) -> Option<u32> {
    card.and_then(|card_id| {
        // CR 709.4b: a split card outside the stack has the combined mana
        // value of its halves, so use the shared filter helper.
        game.object(card_id)
            .map(|object| crate::filter::object_mana_value_for_filter(object).max(0) as u32)
    })
}

fn move_revealed_cards_to_bottom(game: &mut GameState, cards_to_bottom: &[(PlayerId, ObjectId)]) {
    for (player, card) in cards_to_bottom {
        super::arrange_library_cards(game, *player, &[], &[*card], "clash card put on bottom");
    }
}

fn controller_wins_clash(controller_mv: Option<u32>, opponent_mv: Option<u32>) -> bool {
    match (controller_mv, opponent_mv) {
        (Some(left), Some(right)) => left > right,
        (Some(_), None) => true,
        _ => false,
    }
}

fn winner_tags(winner: Option<PlayerId>) -> HashMap<TagKey, Vec<PlayerId>> {
    let mut tags = HashMap::new();
    if let Some(winner) = winner {
        tags.insert(TagKey::from("winner"), vec![winner]);
    }
    tags
}

impl EffectExecutor for ClashEffect {
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
                let mut retained_children = Vec::new();
                let opponents = in_game_opponents(game, ctx.controller);
                let Some(opponent) = choose_opponent(game, ctx, &opponents, self.opponent_mode)
                else {
                    return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::count(0),
                    ));
                };
                if ctx.decision_maker.awaiting_choice() {
                    return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::count(0),
                    ));
                }
                // Later clauses ("Otherwise, that player ...") refer back to the
                // opponent this clash was performed with.
                ctx.set_tagged_players(ironsmith_core::CLASH_OPPONENT_TAG, vec![opponent]);

                let controller_card = top_card(game, ctx.controller);
                let opponent_card = top_card(game, opponent);

                let mut reveals = Vec::new();
                for (player, card) in [(ctx.controller, controller_card), (opponent, opponent_card)]
                {
                    if let Some(card) = card {
                        let cards = ObjectSnapshot::from_object_id(game, card)
                            .into_iter()
                            .collect();
                        let revealed = super::reveal_objects_with_outputs(
                            game,
                            ctx,
                            cards,
                            Some(player),
                            "Reveal the top card of a library",
                            None,
                        )?;
                        if ctx.decision_maker.awaiting_choice() {
                            return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                                EffectOutcome::count(0),
                            ));
                        }
                        reveals.push(revealed.outcome.clone());
                        retained_children.push(revealed);
                    }
                }

                // Public opening must finish before comparing identity-dependent values.
                let clash_winner = controller_wins_clash(
                    card_mana_value(game, controller_card),
                    card_mana_value(game, opponent_card),
                )
                .then_some(ctx.controller)
                .or_else(|| {
                    let controller_mv = card_mana_value(game, controller_card);
                    let opponent_mv = card_mana_value(game, opponent_card);
                    match (controller_mv, opponent_mv) {
                        (Some(left), Some(right)) if right > left => Some(opponent),
                        (None, Some(_)) => Some(opponent),
                        _ => None,
                    }
                });

                let participants = clashing_players_in_apnap_order(game, ctx.controller, opponent)
                    .into_iter().filter_map(|player| {
                        let card = if player == ctx.controller { controller_card } else { opponent_card };
                        card.map(|card| (player, card))
                    }).collect::<Vec<_>>();
                let Some(bottom_choices) = crate::effects::composition::scoped_choice::collect_choices_before_commit(
                    game, ctx, participants, |game, ctx, (player, card)| {
                        let chosen = super::choose_library_bottom_partition(game, ctx, player, &[card]);
                        Ok(chosen.filter(|cards| cards.contains(&card)).map(|_| (player, card)))
                    },
                )? else {
                    return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)));
                };
                let cards_to_bottom = bottom_choices.into_iter().flatten().collect::<Vec<_>>();

                move_revealed_cards_to_bottom(game, &cards_to_bottom);

                let player_tags = winner_tags(clash_winner);
                for player in [ctx.controller, opponent] {
                    let keyword =
                        crate::effects::composition::publish_keyword_action_completion_receipt(
                            game,
                            ctx,
                            crate::triggers::TriggerEvent::new_with_provenance(
                                KeywordActionEvent::new(
                                    KeywordActionKind::Clash,
                                    player,
                                    ctx.source,
                                    1,
                                )
                                .with_player_tags(player_tags.clone()),
                                ctx.provenance,
                            ),
                        )?;
                    if ctx.decision_maker.awaiting_choice() {
                        return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                            EffectOutcome::count(0),
                        ));
                    }
                    reveals.push(keyword.outcome.clone());
                    retained_children.push(keyword);
                }

                Ok(crate::effects::CompletedEffectOutputs::from_children(
                    retained_children,
                    |_| {
                        EffectOutcome::aggregate_with_primary_result(
                            EffectOutcome::count(if clash_winner == Some(ctx.controller) {
                                1
                            } else {
                                0
                            }),
                            reveals,
                        )
                    },
                ))
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::decision::DecisionMaker;
    use crate::effect::{Comparison, EffectPredicate, EffectPredicateRuntimeExt};
    use crate::effects::ExecutionContext;
    use crate::ids::CardId;
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::types::CardType;

    fn add_library_card(
        game: &mut GameState,
        owner: PlayerId,
        name: &str,
        mana_value: u8,
    ) -> ObjectId {
        let id = game.new_object_id();
        let card = CardBuilder::new(CardId::from_raw(id.0 as u32), name)
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(
                mana_value,
            )]]))
            .card_types(vec![CardType::Instant])
            .build();
        game.create_object_from_card(&card, owner, Zone::Library)
    }

    fn library_names_bottom_to_top(game: &GameState, player: PlayerId) -> Vec<String> {
        game.player(player)
            .expect("player should exist")
            .library
            .iter()
            .filter_map(|id| game.object(*id).map(|object| object.name.to_string()))
            .collect()
    }

    #[derive(Default)]
    struct ClashDecisionMaker {
        option_choices: HashMap<PlayerId, Vec<usize>>,
        partitions: HashMap<PlayerId, Vec<ObjectId>>,
        partition_calls: Vec<PlayerId>,
        top_observations: Vec<(PlayerId, Vec<String>)>,
        view_calls: Vec<(PlayerId, PlayerId, Zone, bool, Vec<ObjectId>)>,
    }

    impl DecisionMaker for ClashDecisionMaker {
        fn decide_options(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::SelectOptionsContext,
        ) -> Vec<usize> {
            self.option_choices
                .get(&ctx.player)
                .cloned()
                .unwrap_or_else(|| {
                    ctx.options
                        .iter()
                        .filter(|option| option.legal)
                        .map(|option| option.index)
                        .take(ctx.min)
                        .collect()
                })
        }

        fn decide_partition(
            &mut self,
            game: &GameState,
            ctx: &crate::decisions::context::PartitionContext,
        ) -> Vec<ObjectId> {
            self.partition_calls.push(ctx.player);
            let tops = game
                .players
                .iter()
                .filter_map(|player| {
                    game.player(player.id)
                        .and_then(|entry| entry.library.last().copied())
                        .and_then(|card| game.object(card).map(|object| object.name.to_string()))
                })
                .collect::<Vec<_>>();
            self.top_observations.push((ctx.player, tops));
            self.partitions
                .get(&ctx.player)
                .cloned()
                .unwrap_or_default()
        }

        fn view_cards(
            &mut self,
            _game: &GameState,
            viewer: PlayerId,
            cards: &[ObjectId],
            ctx: &crate::decisions::context::ViewCardsContext,
        ) {
            self.view_calls
                .push((viewer, ctx.subject, ctx.zone, ctx.public, cards.to_vec()));
        }
    }

    #[test]
    fn clash_multiplayer_chooses_opponent_and_emits_reveal_and_clash_events() {
        let mut game = GameState::new(
            vec![
                "Alice".to_string(),
                "Bob".to_string(),
                "Charlie".to_string(),
            ],
            20,
        );
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let charlie = PlayerId::from_index(2);
        let _a1 = add_library_card(&mut game, alice, "A1", 1);
        let a2 = add_library_card(&mut game, alice, "A2", 5);
        let _b1 = add_library_card(&mut game, bob, "B1", 1);
        let b2 = add_library_card(&mut game, bob, "B2", 9);
        let c1 = add_library_card(&mut game, charlie, "C1", 1);
        let c2 = add_library_card(&mut game, charlie, "C2", 2);

        let source = game.new_object_id();
        let mut dm = ClashDecisionMaker::default();
        dm.option_choices.insert(alice, vec![1]);
        dm.partitions.insert(charlie, vec![c2]);

        let outcome = {
            let mut ctx = ExecutionContext::new(source, alice, &mut dm);
            ClashEffect::against_any_opponent()
                .execute(&mut game, &mut ctx)
                .expect("clash should resolve")
        };

        assert_eq!(outcome.value, crate::effect::OutcomeValue::Count(1));
        assert_eq!(
            library_names_bottom_to_top(&game, bob),
            vec!["B1".to_string(), "B2".to_string()]
        );
        assert_eq!(
            library_names_bottom_to_top(&game, charlie),
            vec!["C2".to_string(), "C1".to_string()]
        );
        assert_eq!(dm.partition_calls, vec![alice, charlie]);
        assert_eq!(dm.view_calls.len(), 6);
        assert!(
            dm.view_calls
                .iter()
                .all(|(_, _, zone, public, _)| { *zone == Zone::Library && *public })
        );

        let reveal_events = outcome
            .events
            .iter()
            .filter(|event| event.downcast::<CardRevealedEvent>().is_some())
            .count();
        assert_eq!(reveal_events, 2);

        let clash_events = outcome
            .events
            .iter()
            .filter_map(|event| event.downcast::<KeywordActionEvent>())
            .collect::<Vec<_>>();
        assert_eq!(clash_events.len(), 2);
        assert!(
            clash_events
                .iter()
                .all(|event| event.action == KeywordActionKind::Clash)
        );
        assert!(clash_events.iter().any(|event| {
            event.player == alice
                && event
                    .player_tags
                    .get(&TagKey::from("winner"))
                    .is_some_and(|players| players == &vec![alice])
        }));
        assert_eq!(top_card(&game, alice), Some(a2));
        assert_eq!(top_card(&game, bob), Some(b2));
        assert_eq!(top_card(&game, charlie), Some(c1));
    }

    #[test]
    fn clash_uses_apnap_order_and_moves_revealed_cards_after_all_choices() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        game.turn.active_player = bob;

        add_library_card(&mut game, alice, "A1", 1);
        let _a2 = add_library_card(&mut game, alice, "A2", 4);
        add_library_card(&mut game, bob, "B1", 1);
        let b2 = add_library_card(&mut game, bob, "B2", 2);

        let source = game.new_object_id();
        let mut dm = ClashDecisionMaker::default();
        dm.partitions.insert(bob, vec![b2]);

        ClashEffect::against_any_opponent()
            .execute(
                &mut game,
                &mut ExecutionContext::new(source, alice, &mut dm),
            )
            .expect("clash should resolve");

        assert_eq!(dm.partition_calls, vec![bob, alice]);
        assert_eq!(
            dm.top_observations,
            vec![
                (bob, vec!["A2".to_string(), "B2".to_string()]),
                (alice, vec!["A2".to_string(), "B2".to_string()]),
            ]
        );
        assert_eq!(
            library_names_bottom_to_top(&game, bob),
            vec!["B2".to_string(), "B1".to_string()]
        );
    }

    #[test]
    fn clash_with_defending_player_uses_the_defender() {
        let mut game = GameState::new(
            vec![
                "Alice".to_string(),
                "Bob".to_string(),
                "Charlie".to_string(),
            ],
            20,
        );
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let charlie = PlayerId::from_index(2);

        add_library_card(&mut game, alice, "A1", 5);
        add_library_card(&mut game, bob, "B1", 9);
        let c1 = add_library_card(&mut game, charlie, "C1", 1);
        let source = game.new_object_id();
        let mut dm = ClashDecisionMaker::default();

        let outcome = ClashEffect::against_defending_player()
            .execute(
                &mut game,
                &mut ExecutionContext::new(source, alice, &mut dm).with_defending_player(charlie),
            )
            .expect("clash with defending player should resolve");

        assert_eq!(outcome.value, crate::effect::OutcomeValue::Count(1));
        let clash_players = outcome
            .events
            .iter()
            .filter_map(|event| event.downcast::<KeywordActionEvent>())
            .map(|event| event.player)
            .collect::<Vec<_>>();
        assert_eq!(clash_players, vec![alice, charlie]);
        assert_eq!(top_card(&game, charlie), Some(c1));
        assert_eq!(
            library_names_bottom_to_top(&game, bob),
            vec!["B1".to_string()]
        );
    }

    #[test]
    fn clash_win_predicate_uses_the_result_count_not_the_emitted_events() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        add_library_card(&mut game, alice, "Alice Low", 1);
        add_library_card(&mut game, bob, "Bob High", 5);

        let source = game.new_object_id();
        let mut dm = ClashDecisionMaker::default();
        let outcome = ClashEffect::against_any_opponent()
            .execute(
                &mut game,
                &mut ExecutionContext::new(source, alice, &mut dm),
            )
            .expect("losing clash should resolve");

        assert!(
            !outcome.events.is_empty(),
            "the clash still emits its events"
        );
        assert_eq!(outcome.value, crate::effect::OutcomeValue::Count(0));
        assert!(
            EffectPredicate::Happened.evaluate_outcome(&outcome),
            "generic happened semantics see the clash events and cannot represent winning"
        );
        assert!(
            !EffectPredicate::Value(Comparison::GreaterThan(0)).evaluate_outcome(&outcome),
            "the typed clash-win predicate must be false when the opponent wins"
        );
    }
    struct PausePartition {
        calls: usize,
        pause_at: usize,
        pending: bool,
    }
    impl DecisionMaker for PausePartition {
        fn decide_partition(
            &mut self,
            _: &GameState,
            context: &crate::decisions::context::PartitionContext,
        ) -> Vec<ObjectId> {
            self.calls += 1;
            self.pending = self.calls == self.pause_at;
            context.cards.iter().map(|(id, _)| *id).collect()
        }
        fn awaiting_choice(&self) -> bool {
            self.pending
        }
    }

    #[test]
    fn suspended_multiplayer_library_choices_commit_only_after_all_players_and_replay_once() {
        for (scry, pause_at) in [(false, 1), (false, 2), (true, 1), (true, 2)] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let alice = PlayerId(0);
            let bob = PlayerId(1);
            add_library_card(&mut game, alice, "A bottom", 1);
            let a = add_library_card(&mut game, alice, "A top", 5);
            add_library_card(&mut game, bob, "B bottom", 1);
            let b = add_library_card(&mut game, bob, "B top", 2);
            let before = [
                game.player(alice).unwrap().library.clone(),
                game.player(bob).unwrap().library.clone(),
            ];
            let source = game.new_object_id();
            let effect = if scry {
                crate::effect::Effect::new(crate::effects::EachPlayerScryEffect::new(
                    1,
                    crate::target::PlayerFilter::Any,
                ))
            } else {
                crate::effect::Effect::new(ClashEffect::against_any_opponent())
            };
            let mut dm = PausePartition {
                calls: 0,
                pause_at,
                pending: false,
            };
            {
                let mut ctx = ExecutionContext::new(source, alice, &mut dm);
                let output = effect.0.execute_with_outputs(&mut game, &mut ctx).unwrap();
                assert_eq!(output.outcome.value, crate::effect::OutcomeValue::Count(0));
                assert!(output.outcome.events.is_empty());
            }
            assert_eq!(game.player(alice).unwrap().library, before[0]);
            assert_eq!(game.player(bob).unwrap().library, before[1]);
            dm.pause_at = usize::MAX;
            dm.pending = false;
            let mut ctx = ExecutionContext::new(source, alice, &mut dm);
            let output = effect.0.execute(&mut game, &mut ctx).unwrap();
            assert_eq!(
                output.value,
                crate::effect::OutcomeValue::Count(if scry { 2 } else { 1 })
            );
            assert_eq!(game.player(alice).unwrap().library.first(), Some(&a));
            assert_eq!(game.player(bob).unwrap().library.first(), Some(&b));
            assert_eq!(
                output
                    .events
                    .iter()
                    .filter(|event| event.downcast::<KeywordActionEvent>().is_some())
                    .count(),
                2
            );
        }
    }
}
