use crate::decisions::ask_choose_one;
use crate::effect::{EffectOutcome, ExecutionFact};
use crate::effects::{EffectExecutor, helpers::resolve_player_filter};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::events::CoinFlippedEvent;
use crate::game_state::GameState;
use crate::target::PlayerFilter;

/// Flip a coin for a player using the game's deterministic RNG.
#[derive(Debug, Clone, PartialEq)]
pub struct FlipCoinEffect {
    pub player: PlayerFilter,
    pub kind: ironsmith_core::CoinFlipKind,
    pub forced_face: Option<ironsmith_core::CoinFace>,
    pub forced_winner: Option<PlayerFilter>,
    pub forced_loser: Option<PlayerFilter>,
}

impl FlipCoinEffect {
    pub fn new(player: PlayerFilter) -> Self {
        Self {
            player,
            kind: ironsmith_core::CoinFlipKind::Called,
            forced_face: None,
            forced_winner: None,
            forced_loser: None,
        }
    }

    pub fn face_only(player: PlayerFilter) -> Self {
        Self {
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
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let player = resolve_player_filter(game, &self.player, ctx)?;
        let call = if self.kind == ironsmith_core::CoinFlipKind::Called {
            let options = [
                ("Heads".to_string(), ironsmith_core::CoinFace::Heads),
                ("Tails".to_string(), ironsmith_core::CoinFace::Tails),
            ];
            let selected =
                ask_choose_one(game, &mut ctx.decision_maker, player, ctx.source, &options);
            if ctx.decision_maker.awaiting_choice() {
                return Ok(EffectOutcome::count(0));
            }
            Some(selected.unwrap_or(ironsmith_core::CoinFace::Heads))
        } else {
            None
        };

        let mut faces = [
            ironsmith_core::CoinFace::Heads,
            ironsmith_core::CoinFace::Tails,
        ];
        game.shuffle_slice(&mut faces);
        let face = self.forced_face.unwrap_or(faces[0]);
        let result_is_overridden = self.forced_winner.is_some() || self.forced_loser.is_some();
        let winner = if let Some(winner) = &self.forced_winner {
            Some(resolve_player_filter(game, winner, ctx)?)
        } else if !result_is_overridden && call == Some(face) {
            Some(player)
        } else {
            None
        };
        let loser = if let Some(loser) = &self.forced_loser {
            Some(resolve_player_filter(game, loser, ctx)?)
        } else if !result_is_overridden && call.is_some() && call != Some(face) {
            Some(player)
        } else {
            None
        };
        let result = match self.kind {
            ironsmith_core::CoinFlipKind::Called => u32::from(winner == Some(player)),
            ironsmith_core::CoinFlipKind::FaceOnly => {
                u32::from(face == ironsmith_core::CoinFace::Heads)
            }
        };
        let face_text = match face {
            ironsmith_core::CoinFace::Heads => "heads",
            ironsmith_core::CoinFace::Tails => "tails",
        };
        game.record_ui_effect_event(
            "coin_flip",
            Some(player),
            None,
            Vec::new(),
            Some(i64::from(result)),
            Some(face_text.to_string()),
        );
        Ok(EffectOutcome::count(result as i32)
            .with_event(crate::triggers::TriggerEvent::new_with_provenance(
                CoinFlippedEvent {
                    player,
                    source: ctx.source,
                    face,
                    call,
                    winner,
                    loser,
                },
                ctx.provenance,
            ))
            .with_execution_fact(ExecutionFact::CoinFlip {
                face,
                call,
                winner,
                loser,
            }))
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
            Some(i32::from(event.face == ironsmith_core::CoinFace::Heads))
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
