#[path = "coin_flip_transaction.rs"]
mod coin_flip_transaction;
use crate::effect::{EffectOutcome, ExecutionFact};
use crate::effects::{EffectExecutor, helpers::resolve_player_filter};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::events::CoinFlippedEvent;
use crate::game_state::GameState;
use crate::target::PlayerFilter;

/// Flip a coin for a player using the game's deterministic RNG.
#[derive(Debug, Clone, PartialEq)]
pub struct FlipCoinEffect {
    /// Each retained flip is a fresh batch until an actual loss ends the process.
    pub repeat_until_loss: bool,
    pub opponent_results: Option<ironsmith_core::CoinFlipOpponentTags>,
    pub count_value: Option<ironsmith_core::Value>,
    pub count: u32,
    pub player: PlayerFilter,
    pub kind: ironsmith_core::CoinFlipKind,
    pub forced_face: Option<ironsmith_core::CoinFace>,
    pub forced_winner: Option<PlayerFilter>,
    pub forced_loser: Option<PlayerFilter>,
    pub stop_condition: Option<ironsmith_core::CoinFlipStopCondition>,
    pub loss_action: Option<ironsmith_core::CoinFlipLossAction>,
}

impl FlipCoinEffect {
    pub fn new(player: PlayerFilter) -> Self {
        Self {
            repeat_until_loss: false,
            stop_condition: None,
            loss_action: None,
            opponent_results: None,
            count_value: None,
            count: 1,
            player,
            kind: ironsmith_core::CoinFlipKind::Called,
            forced_face: None,
            forced_winner: None,
            forced_loser: None,
        }
    }

    pub fn face_only(player: PlayerFilter) -> Self {
        Self {
            repeat_until_loss: false,
            stop_condition: None,
            loss_action: None,
            opponent_results: None,
            count_value: None,
            count: 1,
            player,
            kind: ironsmith_core::CoinFlipKind::FaceOnly,
            forced_face: None,
            forced_winner: None,
            forced_loser: None,
        }
    }

    pub fn with_forced_face(mut self, face: ironsmith_core::CoinFace) -> Self {
        self.forced_face = Some(face);
        self
    }

    pub fn with_forced_winner(mut self, winner: PlayerFilter) -> Self {
        self.forced_winner = Some(winner);
        self
    }

    pub fn with_forced_loser(mut self, loser: PlayerFilter) -> Self {
        self.forced_loser = Some(loser);
        self
    }
}

impl EffectExecutor for FlipCoinEffect {
    fn is_read_only_simultaneous_player_action(&self) -> bool {
        // A flip consumes randomness and records presentation events, but it
        // does not change cards, life totals, counters, or any other game
        // object inspected by another player's action. ForPlayers may
        // therefore collect every player's result in APNAP order before a
        // later result-correlated action mutates the game.
        true
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        crate::effects::composition::execute_transaction_from_body(
            game,
            ctx,
            || EffectOutcome::count(0),
            |game, ctx| {
                if (self.repeat_until_loss || self.loss_action.is_some())
                    && self.kind != ironsmith_core::CoinFlipKind::Called
                {
                    return Err(ExecutionError::UnresolvableValue(
                        "repeat until loss requires called flips".into(),
                    ));
                }
                use ironsmith_core::CoinFlipStopCondition;
                if self.stop_condition.is_some() && !self.repeat_until_loss {
                    return Err(ExecutionError::UnresolvableValue(
                        "a coin stop condition requires successive called flips".into(),
                    ));
                }
                let player = resolve_player_filter(game, &self.player, ctx)?;
                if self.repeat_until_loss && self.opponent_results.is_some() {
                    return Err(ExecutionError::UnresolvableValue(
                        "a repeated flip cannot also be an opponent batch".into(),
                    ));
                }
                let associated = if self.opponent_results.is_some() {
                    let players_in_range = ctx.filter_context(game).players_in_range;
                    let mut opponents = Vec::new();
                    opponents.try_reserve(game.players.len()).map_err(|_| {
                        ExecutionError::ResourceAllocationFailed {
                            resource: "opponent coin associations",
                            requested: game.players.len(),
                        }
                    })?;
                    for participant in &game.players {
                        if participant.is_in_game()
                            && game.are_opponents(player, participant.id)
                            && players_in_range
                                .as_ref()
                                .is_none_or(|range| range.contains(&participant.id))
                        {
                            opponents.push(participant.id);
                        }
                    }
                    Some(opponents)
                } else {
                    None
                };
                if self.count_value.is_some()
                    && (self.opponent_results.is_some()
                        || (self.repeat_until_loss
                            && self.stop_condition != Some(CoinFlipStopCondition::CountReached)))
                {
                    return Err(ExecutionError::UnresolvableValue(
                        "a chosen coin count cannot also be a repeat or opponent count".into(),
                    ));
                }
                let authored_count = if let Some(value) = &self.count_value {
                    crate::effects::helpers::resolve_nonnegative_u32(game, value, ctx)?
                } else if let Some(players) = &associated {
                    u32::try_from(players.len()).map_err(|_| {
                        ExecutionError::ResourceLimitExceeded {
                            resource: "opponent coin count",
                            requested: players.len() as u128,
                            maximum: u32::MAX as u128,
                        }
                    })?
                } else {
                    self.count
                };
                let mut results = Vec::new();
                let mut events = Vec::new();
                let mut facts = Vec::new();
                loop {
                    if self.stop_condition == Some(CoinFlipStopCondition::CountReached)
                        && results.len() as u128 >= u128::from(authored_count)
                    {
                        break;
                    }
                    let count = if self.repeat_until_loss {
                        1
                    } else {
                        authored_count
                    };
                    let start = u32::try_from(results.len()).map_err(|_| {
                        ExecutionError::ResourceLimitExceeded {
                            resource: "instruction coin-flip ordinal",
                            requested: results.len() as u128,
                            maximum: i32::MAX as u128,
                        }
                    })? + 1;
                    let Some(mut batch) =
                        coin_flip_transaction::flip_batch(game, ctx, self, player, count, start)?
                    else {
                        return Ok(EffectOutcome::count(0));
                    };
                    events.try_reserve(batch.len()).map_err(|_| {
                        ExecutionError::ResourceAllocationFailed {
                            resource: "coin-flip event receipts",
                            requested: batch.len(),
                        }
                    })?;
                    results.try_reserve(batch.len()).map_err(|_| {
                        ExecutionError::ResourceAllocationFailed {
                            resource: "coin-flip grouped receipt",
                            requested: batch.len(),
                        }
                    })?;
                    facts.try_reserve(batch.len()).map_err(|_| {
                        ExecutionError::ResourceAllocationFailed {
                            resource: "coin-flip result facts",
                            requested: batch.len(),
                        }
                    })?;
                    if let Some(players) = &associated {
                        for (flip, opponent) in batch.iter_mut().zip(players) {
                            flip.associated_player = Some(*opponent);
                        }
                    }
                    let lost = batch.iter().any(|flip| flip.loser == Some(player));
                    let batch_provenance = game.alloc_child_event_provenance(
                        ctx.provenance,
                        crate::events::EventKind::CoinFlipped,
                    );
                    for flip in &batch {
                        let result = match self.kind {
                            ironsmith_core::CoinFlipKind::Called => flip.winner == Some(player),
                            ironsmith_core::CoinFlipKind::FaceOnly => {
                                flip.face == ironsmith_core::CoinFace::Heads
                            }
                        };
                        game.record_ui_effect_event(
                            "coin_flip",
                            Some(player),
                            None,
                            Vec::new(),
                            Some(i64::from(result)),
                            Some(
                                if flip.face == ironsmith_core::CoinFace::Heads {
                                    "heads"
                                } else {
                                    "tails"
                                }
                                .into(),
                            ),
                        );
                        let provenance = game.alloc_child_event_provenance(
                            ctx.provenance,
                            crate::events::EventKind::CoinFlipped,
                        );
                        events.push(
                            crate::triggers::TriggerEvent::new_with_provenance(
                                CoinFlippedEvent {
                                    player,
                                    source: ctx.source,
                                    face: flip.face,
                                    call: flip.call,
                                    winner: flip.winner,
                                    loser: flip.loser,
                                    turn_ordinal: flip.turn_ordinal,
                                    instruction_ordinal: flip.instruction_ordinal,
                                },
                                provenance,
                            )
                            .with_simultaneous_batch(batch_provenance),
                        );
                        facts.push(ExecutionFact::CoinFlip {
                            face: flip.face,
                            call: flip.call,
                            winner: flip.winner,
                            loser: flip.loser,
                        });
                    }
                    results.extend(batch);
                    if !self.repeat_until_loss || lost {
                        break;
                    }
                    if self.stop_condition == Some(CoinFlipStopCondition::ChooseToStop) {
                        let again = crate::decisions::ask_may_choice(
                            game,
                            &mut ctx.decision_maker,
                            player,
                            ctx.source,
                            "Flip again",
                            crate::decision::FallbackStrategy::Decline,
                        );
                        if ctx.decision_maker.awaiting_choice() {
                            return Ok(EffectOutcome::count(0));
                        }
                        if !again {
                            break;
                        }
                    }
                }
                if let Some(tags) = &self.opponent_results {
                    let mut won = Vec::new();
                    let mut lost = Vec::new();
                    for players in [&mut won, &mut lost] {
                        players.try_reserve(results.len()).map_err(|_| {
                            ExecutionError::ResourceAllocationFailed {
                                resource: "opponent coin-result roster",
                                requested: results.len(),
                            }
                        })?;
                    }
                    for flip in &results {
                        if let Some(opponent) = flip.associated_player {
                            if flip.winner == Some(player) {
                                won.push(opponent);
                            }
                            if flip.loser == Some(player) {
                                lost.push(opponent);
                            }
                        }
                    }
                    ctx.set_tagged_players(tags.won.clone(), won);
                    ctx.set_tagged_players(tags.lost.clone(), lost);
                }
                let positive = results
                    .iter()
                    .filter(|flip| match self.kind {
                        ironsmith_core::CoinFlipKind::Called => flip.winner == Some(player),
                        ironsmith_core::CoinFlipKind::FaceOnly => {
                            flip.face == ironsmith_core::CoinFace::Heads
                        }
                    })
                    .count() as i64;
                facts
                    .try_reserve(1)
                    .map_err(|_| ExecutionError::ResourceAllocationFailed {
                        resource: "coin-flip grouped fact",
                        requested: 1,
                    })?;
                if self.loss_action == Some(ironsmith_core::CoinFlipLossAction::StopResolution)
                    && results.iter().any(|flip| flip.loser == Some(player))
                {
                    ctx.stop_resolution();
                }
                facts.push(ExecutionFact::CoinFlips(results));
                Ok(EffectOutcome::with_details(
                    crate::effect::OutcomeStatus::Succeeded,
                    crate::effect::OutcomeValue::Count(positive),
                    events,
                    facts,
                ))
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decision::DecisionMaker;
    use crate::decisions::SelectOptionsContext;
    use crate::effect::{Effect, EffectId, EffectPredicate};
    use crate::effects::{ExecutionContext, execute_effect};
    use crate::ids::PlayerId;

    struct Call(usize);

    impl DecisionMaker for Call {
        fn decide_options(&mut self, _game: &GameState, _ctx: &SelectOptionsContext) -> Vec<usize> {
            vec![self.0]
        }
    }

    struct NoCallAllowed;

    impl DecisionMaker for NoCallAllowed {
        fn decide_options(&mut self, _game: &GameState, _ctx: &SelectOptionsContext) -> Vec<usize> {
            panic!("a face-only flip must not ask for a call")
        }
    }

    #[test]
    fn flip_coin_is_deterministic_for_a_seed_and_marks_random_usage() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        game.set_random_seed(7);

        let before = game.irreversible_random_count();
        let mut ctx = ExecutionContext::new_default(source, alice);
        let outcome = execute_effect(&mut game, &Effect::flip_coin(PlayerFilter::You), &mut ctx)
            .expect("coin flip should resolve");

        assert_eq!(
            game.irreversible_random_count(),
            before + 1,
            "coin flips should consume irreversible randomness"
        );
        assert_eq!(
            outcome.as_count(),
            Some(0),
            "seeded coin flip should stay deterministic"
        );
    }

    #[test]
    fn flip_coin_outcome_drives_if_result_branches() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        game.set_random_seed(2);

        let mut ctx = ExecutionContext::new_default(source, alice);
        execute_effect(
            &mut game,
            &Effect::with_id(0, Effect::flip_coin(PlayerFilter::You)),
            &mut ctx,
        )
        .expect("coin flip should resolve");

        execute_effect(
            &mut game,
            &Effect::if_then(
                EffectId(0),
                EffectPredicate::Happened,
                vec![Effect::gain_life(3)],
            ),
            &mut ctx,
        )
        .expect("if-result branch should resolve");

        assert_eq!(
            game.player(alice).unwrap().life,
            23,
            "winning the seeded coin flip should take the happened branch"
        );
    }

    #[test]
    fn called_flip_keeps_face_call_and_win_as_distinct_facts() {
        fn flip_with_call(call: usize) -> (EffectOutcome, PlayerId) {
            let mut game = crate::tests::test_helpers::setup_two_player_game();
            let alice = PlayerId::from_index(0);
            let source = game.new_object_id();
            game.set_random_seed(41);
            let mut decisions = Call(call);
            let mut ctx =
                ExecutionContext::new_default(source, alice).with_decision_maker(&mut decisions);
            let outcome =
                execute_effect(&mut game, &Effect::flip_coin(PlayerFilter::You), &mut ctx)
                    .expect("coin flip resolves");
            (outcome, alice)
        }

        let (heads_call, alice) = flip_with_call(0);
        let (tails_call, _) = flip_with_call(1);
        let heads_event = heads_call.events[0]
            .downcast::<CoinFlippedEvent>()
            .expect("coin event");
        let tails_event = tails_call.events[0]
            .downcast::<CoinFlippedEvent>()
            .expect("coin event");

        assert_eq!(heads_event.face, tails_event.face);
        assert_eq!(heads_event.call, Some(ironsmith_core::CoinFace::Heads));
        assert_eq!(tails_event.call, Some(ironsmith_core::CoinFace::Tails));
        assert_ne!(heads_event.flipper_won(), tails_event.flipper_won());
        assert!(
            [heads_event.winner, tails_event.winner]
                .into_iter()
                .all(|winner| winner.is_none() || winner == Some(alice))
        );
    }

    #[test]
    fn face_only_flip_has_no_call_winner_or_loser() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        game.set_random_seed(7);
        let mut decisions = NoCallAllowed;
        let mut ctx =
            ExecutionContext::new_default(source, alice).with_decision_maker(&mut decisions);

        let outcome = execute_effect(
            &mut game,
            &Effect::flip_coin_for_face(PlayerFilter::You),
            &mut ctx,
        )
        .expect("face-only flip resolves");
        let event = outcome.events[0]
            .downcast::<CoinFlippedEvent>()
            .expect("coin event");

        assert_eq!(event.call, None);
        assert_eq!(event.winner, None);
        assert_eq!(event.loser, None);
        assert!(!event.flipper_won());
        assert!(!event.flipper_lost());
        assert_eq!(
            outcome.as_count(),
            Some(i64::from(event.face == ironsmith_core::CoinFace::Heads))
        );
    }

    #[test]
    fn each_player_face_only_flips_keep_per_player_heads_and_tails_results() {
        fn run(face: ironsmith_core::CoinFace) -> Vec<(PlayerId, i64)> {
            let mut game = crate::tests::test_helpers::setup_two_player_game();
            let alice = PlayerId::from_index(0);
            let source = game.new_object_id();
            let mut decisions = NoCallAllowed;
            let mut ctx =
                ExecutionContext::new_default(source, alice).with_decision_maker(&mut decisions);
            let flip = Effect::new(
                FlipCoinEffect::face_only(PlayerFilter::IteratedPlayer).with_forced_face(face),
            );
            let each_player = Effect::new(crate::effects::ForPlayersEffect::new(
                PlayerFilter::Any,
                vec![flip],
            ));

            let outcome = execute_effect(&mut game, &each_player, &mut ctx)
                .expect("face-only flips should execute for every player");
            assert_eq!(outcome.events.len(), 2);
            outcome
                .player_counts()
                .expect("each-player flips retain one result per player")
                .to_vec()
        }

        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        assert_eq!(
            run(ironsmith_core::CoinFace::Heads),
            vec![(alice, 1), (bob, 1)]
        );
        assert_eq!(
            run(ironsmith_core::CoinFace::Tails),
            vec![(alice, 0), (bob, 0)]
        );
    }

    #[test]
    fn stated_face_and_winner_override_actual_called_result() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.new_object_id();
        game.set_random_seed(2);
        let before = game.irreversible_random_count();
        let mut decisions = Call(0);
        let mut ctx =
            ExecutionContext::new_default(source, alice).with_decision_maker(&mut decisions);

        let outcome = FlipCoinEffect::new(PlayerFilter::You)
            .with_forced_face(ironsmith_core::CoinFace::Tails)
            .with_forced_winner(PlayerFilter::Specific(bob))
            .execute(&mut game, &mut ctx)
            .expect("overridden flip resolves");
        let event = outcome.events[0]
            .downcast::<CoinFlippedEvent>()
            .expect("coin event");

        assert_eq!(event.face, ironsmith_core::CoinFace::Tails);
        assert_eq!(event.call, Some(ironsmith_core::CoinFace::Heads));
        assert_eq!(event.winner, Some(bob));
        assert_eq!(event.loser, None);
        assert!(!event.flipper_lost());
        assert_eq!(outcome.as_count(), Some(0));
        assert_eq!(game.irreversible_random_count(), before + 1);
    }
}
