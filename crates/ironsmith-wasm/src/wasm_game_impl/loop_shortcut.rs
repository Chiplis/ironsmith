// CR 732.2 player-proposed shortcuts. The player with priority proposes
// repeating a recorded sequence of their own choices a number of times; each
// other player, in turn order, accepts or shortens it; then the proposer's
// client replays the steps as ordinary commands while the others pass.
//
// The steps are opaque to the engine: the proposer's client recorded them by
// stable object identity and re-resolves each one against the live game, so
// every replayed step is validated like any other command. The engine only
// keeps the agreement every peer must share.

/// Longest proposal and largest repeat count accepted from a client.
const MAX_LOOP_SHORTCUT_STEPS: usize = 512;
const MAX_LOOP_SHORTCUT_STEPS_BYTES: usize = 128 * 1024;
const MAX_LOOP_SHORTCUT_ITERATIONS: u32 = 10_000;

#[derive(Debug, Clone, Serialize)]
pub(super) struct LoopShortcut {
    proposer: u8,
    steps: serde_json::Value,
    /// The count the proposer suggested.
    proposed: u32,
    /// The count after every response so far; it only ever shrinks (CR 732.2b).
    agreed: u32,
    /// Players still to respond, in turn order after the proposer.
    awaiting: Vec<u8>,
    /// First iteration with manual opponent priority windows (one-based).
    interrupt_at: Option<u32>,
    interrupt_player: Option<u8>,
    /// Earlier iterations have finished; opponents now pass manually.
    stepping: bool,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct LoopShortcutView {
    proposer: u8,
    steps: serde_json::Value,
    proposed: u32,
    agreed: u32,
    awaiting_player: Option<u8>,
    /// Every other player accepted; the proposer's client is replaying it.
    running: bool,
    interrupt_at: Option<u32>,
    interrupt_player: Option<u8>,
    stepping: bool,
}

impl WasmGame {
    fn loop_shortcut_view(&self) -> Option<LoopShortcutView> {
        self.loop_shortcut
            .as_ref()
            .map(|shortcut| LoopShortcutView {
                proposer: shortcut.proposer,
                steps: shortcut.steps.clone(),
                proposed: shortcut.proposed,
                agreed: shortcut.agreed,
                awaiting_player: shortcut.awaiting.first().copied(),
                running: shortcut.awaiting.is_empty(),
                interrupt_at: shortcut.interrupt_at,
                interrupt_player: shortcut.interrupt_player,
                stepping: shortcut.stepping,
            })
    }

    /// CR 732.2a: only the player with priority may suggest a shortcut.
    fn propose_loop_shortcut(
        &mut self,
        steps: serde_json::Value,
        iterations: u32,
    ) -> Result<(), String> {
        let Some(DecisionContext::Priority(priority)) = self.pending_decision.as_ref() else {
            return Err("a loop can be proposed only by the player with priority".to_string());
        };
        if self.pending_priority_decision_is_stale() {
            return Err(
                "pending priority decision no longer matches the game priority holder".to_string(),
            );
        }
        if self.loop_shortcut.is_some() {
            return Err("another loop shortcut is already in progress".to_string());
        }
        let step_count = steps.as_array().map(Vec::len).unwrap_or(0);
        if step_count == 0 || step_count > MAX_LOOP_SHORTCUT_STEPS {
            return Err(format!(
                "a loop needs between 1 and {MAX_LOOP_SHORTCUT_STEPS} recorded steps"
            ));
        }
        if serde_json::to_vec(&steps)
            .map(|bytes| bytes.len())
            .unwrap_or(usize::MAX)
            > MAX_LOOP_SHORTCUT_STEPS_BYTES
        {
            return Err("the recorded loop is too large".to_string());
        }
        if iterations == 0 || iterations > MAX_LOOP_SHORTCUT_ITERATIONS {
            return Err(format!(
                "a loop repeats between 1 and {MAX_LOOP_SHORTCUT_ITERATIONS} times"
            ));
        }
        let proposer = priority.player;
        let mut awaiting = Vec::new();
        let mut cursor = proposer;
        while let Some(next) = self.game.next_player_in_game_after(cursor) {
            if next == proposer || awaiting.contains(&next.0) {
                break;
            }
            awaiting.push(next.0);
            cursor = next;
        }
        self.loop_shortcut = Some(LoopShortcut {
            proposer: proposer.0,
            steps,
            proposed: iterations,
            agreed: iterations,
            awaiting,
            interrupt_at: None,
            interrupt_player: None,
            stepping: false,
        });
        if self
            .loop_shortcut
            .as_ref()
            .is_some_and(|shortcut| shortcut.awaiting.is_empty())
        {
            self.pass_intents.clear();
        }
        Ok(())
    }

    /// CR 732.2b: the next player in turn order accepts the current count or
    /// shortens it. Shortening to zero declines the shortcut.
    #[cfg(test)]
    fn respond_loop_shortcut(&mut self, player: u8, iterations: u32) -> Result<(), String> {
        self.respond_loop_shortcut_at(player, iterations, None)
    }

    fn respond_loop_shortcut_at(
        &mut self,
        player: u8,
        iterations: u32,
        interrupt_at: Option<u32>,
    ) -> Result<(), String> {
        let Some(shortcut) = self.loop_shortcut.as_mut() else {
            return Err("no loop shortcut is waiting for a response".to_string());
        };
        if shortcut.awaiting.first() != Some(&player) {
            return Err("it is not this player's turn to respond to the loop".to_string());
        }
        if iterations > shortcut.agreed {
            return Err("a response can only accept or shorten the loop".to_string());
        }
        if interrupt_at.is_some_and(|iteration| iteration == 0 || iteration > iterations) {
            return Err(
                "the interruption must name an iteration within the accepted loop".to_string(),
            );
        }
        if let Some(iteration) = interrupt_at {
            if shortcut
                .interrupt_at
                .is_none_or(|previous| iteration < previous)
            {
                shortcut.interrupt_at = Some(iteration);
                shortcut.interrupt_player = Some(player);
            }
        }
        shortcut.awaiting.remove(0);
        shortcut.agreed = iterations.min(shortcut.interrupt_at.unwrap_or(iterations));
        if shortcut
            .interrupt_at
            .is_some_and(|iteration| iteration > shortcut.agreed)
        {
            shortcut.interrupt_at = None;
            shortcut.interrupt_player = None;
        }
        if shortcut.agreed == 0 {
            self.loop_shortcut = None;
        }
        // Accepted shortcut execution owns priority; old yielding intentions
        // must not skip a response window during a later manual iteration.
        if self
            .loop_shortcut
            .as_ref()
            .is_some_and(|shortcut| shortcut.awaiting.is_empty())
        {
            self.pass_intents.clear();
        }
        Ok(())
    }

    fn step_loop_shortcut(&mut self, player: u8) -> Result<(), String> {
        let Some(shortcut) = self.loop_shortcut.as_mut() else {
            return Err("no loop shortcut is running".to_string());
        };
        if shortcut.proposer != player
            || !shortcut.awaiting.is_empty()
            || shortcut.interrupt_at.is_none()
        {
            return Err(
                "only the proposer can begin the agreed interruption iteration".to_string(),
            );
        }
        shortcut.stepping = true;
        self.pass_intents.clear();
        Ok(())
    }

    /// The proposer ends the shortcut: it finished, or a recorded step is no
    /// longer available, or they changed their mind.
    fn cancel_loop_shortcut(&mut self, player: u8) -> Result<(), String> {
        match self.loop_shortcut.as_ref() {
            Some(shortcut) if shortcut.proposer == player => {
                self.loop_shortcut = None;
                self.pass_intents.clear();
                Ok(())
            }
            Some(_) => Err("only the proposing player can end the loop".to_string()),
            None => Ok(()),
        }
    }

    fn dispatch_loop_shortcut_command(&mut self, command: UiCommand) -> Result<JsValue, JsValue> {
        let before = RuntimeSavepoint::capture(self);
        let applied = match command {
            UiCommand::ProposeLoopShortcut { steps, iterations } => {
                self.propose_loop_shortcut(steps, iterations)
            }
            UiCommand::RespondLoopShortcut {
                player,
                iterations,
                interrupt_at,
            } => self.respond_loop_shortcut_at(player, iterations, interrupt_at),
            UiCommand::StepLoopShortcut { player } => self.step_loop_shortcut(player),
            UiCommand::CancelLoopShortcut { player } => self.cancel_loop_shortcut(player),
            _ => Err("not a loop shortcut command".to_string()),
        };
        if let Err(error) = applied {
            before.restore(self);
            return Err(payment_disclosure_error(&error));
        }
        self.cached_snapshot = None;
        let result = self.snapshot();
        if result.is_err() {
            before.restore(self);
        }
        result
    }
}

#[cfg(test)]
mod loop_shortcut_tests {
    use super::*;

    fn fixture(players: usize) -> WasmGame {
        let mut wasm = WasmGame::new_with_registry(CardRegistry::new());
        let names = (0..players).map(|index| format!("P{index}")).collect();
        wasm.initialize_empty_match(names, 20, 1);
        wasm.game.turn.active_player = PlayerId(1);
        wasm.game.turn.priority_player = Some(PlayerId(1));
        wasm.pending_decision = Some(DecisionContext::Priority(
            ironsmith::decisions::context::PriorityContext::new(
                &wasm.game,
                PlayerId(1),
                vec![LegalAction::PassPriority],
            )
            .unwrap(),
        ));
        wasm
    }

    fn steps() -> serde_json::Value {
        serde_json::json!([{ "type": "priority_action", "action_ref": { "kind": "pass_priority" } }])
    }

    #[test]
    fn opponents_respond_in_turn_order_and_can_only_shorten() {
        let _ids = crate::test_id_counter_guard();
        let mut wasm = fixture(4);
        wasm.propose_loop_shortcut(steps(), 20)
            .expect("priority holder proposes");
        assert!(
            wasm.propose_loop_shortcut(steps(), 5).is_err(),
            "one shortcut at a time"
        );
        let view = wasm.loop_shortcut_view().unwrap();
        assert_eq!(
            (view.proposer, view.awaiting_player, view.running),
            (1, Some(2), false)
        );
        assert!(
            wasm.respond_loop_shortcut(3, 20).is_err(),
            "player 3 responds after player 2"
        );
        wasm.respond_loop_shortcut(2, 20).expect("player 2 accepts");
        assert!(
            wasm.respond_loop_shortcut(3, 21).is_err(),
            "a response cannot lengthen the loop"
        );
        wasm.respond_loop_shortcut(3, 12)
            .expect("player 3 shortens");
        wasm.respond_loop_shortcut(0, 12)
            .expect("player 0 accepts the shortened loop");
        let view = wasm.loop_shortcut_view().unwrap();
        assert_eq!(
            (
                view.proposed,
                view.agreed,
                view.awaiting_player,
                view.running
            ),
            (20, 12, None, true)
        );
        assert!(
            wasm.cancel_loop_shortcut(2).is_err(),
            "only the proposer ends the loop"
        );
        wasm.cancel_loop_shortcut(1)
            .expect("proposer ends the loop");
        assert!(wasm.loop_shortcut.is_none());
    }

    #[test]
    fn shortening_to_zero_declines_and_bad_proposals_are_rejected() {
        let _ids = crate::test_id_counter_guard();
        let mut wasm = fixture(2);
        assert!(
            wasm.propose_loop_shortcut(serde_json::json!([]), 3)
                .is_err()
        );
        assert!(wasm.propose_loop_shortcut(steps(), 0).is_err());
        assert!(
            wasm.propose_loop_shortcut(steps(), MAX_LOOP_SHORTCUT_ITERATIONS + 1)
                .is_err()
        );
        wasm.propose_loop_shortcut(steps(), 3).unwrap();
        wasm.respond_loop_shortcut(0, 0)
            .expect("the opponent declines");
        assert!(wasm.loop_shortcut.is_none());
        wasm.pending_decision = None;
        assert!(
            wasm.propose_loop_shortcut(steps(), 3).is_err(),
            "no priority, no proposal"
        );
    }

    #[test]
    fn earliest_interruption_runs_previous_iterations_then_enters_manual_responses() {
        let _ids = crate::test_id_counter_guard();
        let mut wasm = fixture(4);
        wasm.propose_loop_shortcut(steps(), 20).unwrap();
        assert!(wasm.respond_loop_shortcut_at(2, 20, Some(0)).is_err());
        assert!(wasm.respond_loop_shortcut_at(2, 20, Some(21)).is_err());
        wasm.respond_loop_shortcut_at(2, 20, Some(8)).unwrap();
        assert!(
            wasm.step_loop_shortcut(1).is_err(),
            "everyone must respond first"
        );
        wasm.respond_loop_shortcut_at(3, 8, Some(3)).unwrap();
        wasm.respond_loop_shortcut(0, 3).unwrap();
        let view = wasm.loop_shortcut_view().unwrap();
        assert_eq!(
            (view.agreed, view.interrupt_at, view.interrupt_player),
            (3, Some(3), Some(3))
        );
        assert!(view.running);
        assert!(!view.stepping);
        assert!(
            wasm.step_loop_shortcut(3).is_err(),
            "only the proposer advances replay"
        );
        wasm.step_loop_shortcut(1).unwrap();
        assert!(wasm.loop_shortcut_view().unwrap().stepping);
        wasm.cancel_loop_shortcut(1).unwrap();
        assert!(wasm.loop_shortcut_view().is_none());
    }

    #[test]
    fn loop_commands_dispatch_through_the_typed_command_path() {
        let _ids = crate::test_id_counter_guard();
        let mut wasm = fixture(2);
        wasm.dispatch_typed_command(
            UiCommand::ProposeLoopShortcut {
                steps: steps(),
                iterations: 4,
            },
            0.0,
        )
        .expect("proposal accepted");
        assert!(
            wasm.dispatch_typed_command(
                UiCommand::PriorityAction {
                    action_ref: Some(PriorityActionRef::PassPriority),
                    action_index: None,
                },
                0.0
            )
            .is_err(),
            "gameplay waits for the agreement"
        );
        wasm.dispatch_typed_command(
            UiCommand::RespondLoopShortcut {
                player: 0,
                iterations: 4,
                interrupt_at: Some(2),
            },
            0.0,
        )
        .expect("response accepted");
        assert!(wasm.loop_shortcut_view().unwrap().running);
        wasm.dispatch_typed_command(UiCommand::StepLoopShortcut { player: 1 }, 0.0)
            .unwrap();
        assert!(wasm.loop_shortcut_view().unwrap().stepping);
        assert!(
            wasm.dispatch_typed_command(
                UiCommand::PriorityYield {
                    mode: PassMode::Stack,
                    stops: vec![],
                },
                0.0
            )
            .is_err(),
            "Resolve all cannot bypass the manual iteration"
        );
        assert!(
            matches!(wasm.pending_decision, Some(DecisionContext::Priority(_))),
            "the decision is untouched"
        );
        wasm.dispatch_typed_command(UiCommand::CancelLoopShortcut { player: 1 }, 0.0)
            .expect("proposer ends the loop");
        assert!(wasm.loop_shortcut.is_none());
    }
}
