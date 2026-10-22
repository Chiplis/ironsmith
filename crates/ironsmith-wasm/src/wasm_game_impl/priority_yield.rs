// Pass intentions are gameplay protocol state: only the pending decision's
// authenticated actor may grant one, and every peer derives the same passes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum PassMode {
    Stack,
    Phases,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct PassIntent {
    mode: PassMode,
    turn: u32,
    stops: Vec<String>,
    starting_phase: Option<String>,
    starting_step: Option<String>,
    previous_step: String,
    // Runtime incarnations detect new entries locally. Public consensus keeps
    // the identities captured at authorization, even after those spells leave.
    #[serde(skip)]
    stack: Vec<u64>,
    stack_identity: Vec<(bool, u64)>,
}

impl WasmGame {
    fn pass_window(&self) -> (String, String) {
        let step = match self.game.turn.step {
            Some(Step::CombatDamage)
                if combat_damage_step_for_runner(self.runner.as_ref()) == Some("first_strike") =>
            {
                "FirstStrikeDamage".into()
            }
            Some(step) => format!("{step:?}"),
            None => String::new(),
        };
        let phase = match self.game.turn.phase {
            Phase::FirstMain => "Main".into(),
            Phase::NextMain => "Main2".into(),
            Phase::Combat => "Combat".into(),
            _ => step.clone(),
        };
        (phase, step)
    }

    fn pass_stack_ids(&self) -> Vec<u64> {
        self.game
            .stack
            .iter()
            .map(|entry| entry.ability_id.unwrap_or(entry.object_id).0)
            .collect()
    }

    fn dispatch_priority_yield(
        &mut self,
        mode: PassMode,
        stops: Vec<String>,
    ) -> Result<JsValue, JsValue> {
        let Some(DecisionContext::Priority(priority)) = self.pending_decision.as_ref() else {
            return Err(payment_disclosure_error(
                "passing requires a priority decision",
            ));
        };
        if self.pending_priority_decision_is_stale()
            || !priority
                .actions
                .iter()
                .any(|a| matches!(a, LegalAction::PassPriority))
            || (mode == PassMode::Stack && self.game.stack.is_empty())
        {
            return Err(payment_disclosure_error("passing is no longer available"));
        }
        const STOPS: &[&str] = &[
            "phase:Untap",
            "phase:Upkeep",
            "phase:Draw",
            "phase:Main",
            "phase:Combat",
            "phase:Main2",
            "phase:End",
            "phase:Cleanup",
            "step:Untap",
            "step:Upkeep",
            "step:Draw",
            "step:BeginCombat",
            "step:DeclareAttackers",
            "step:DeclareBlockers",
            "step:FirstStrikeDamage",
            "step:CombatDamage",
            "step:EndCombat",
            "step:End",
            "step:Cleanup",
        ];
        if stops.len() > STOPS.len() || stops.iter().any(|s| !STOPS.contains(&s.as_str())) {
            return Err(payment_disclosure_error("invalid priority stops"));
        }
        let actor = priority.player.0;
        let before = RuntimeSavepoint::capture(self);
        let (phase, step) = self.pass_window();
        self.pass_intents.insert(
            actor,
            PassIntent {
                mode,
                turn: self.game.turn.turn_number,
                stops,
                starting_phase: Some(phase),
                previous_step: step.clone(),
                starting_step: Some(step),
                stack: self.pass_stack_ids(),
                stack_identity: self
                    .game
                    .stack
                    .iter()
                    .map(|entry| match entry.ability_id {
                        Some(id) => (true, id.0),
                        None => (
                            false,
                            self.game
                                .object(entry.object_id)
                                .map(|object| object.stable_id.0.0)
                                .unwrap_or(entry.object_id.0),
                        ),
                    })
                    .collect(),
            },
        );
        // Reuse the rules/crypto snapshot boundary without constructing or
        // encoding any intermediate presentation. Stop at the first disclosure
        // so its material remains attributable to this signed command.
        let output_policy = std::mem::replace(&mut self.previewing_crypto_requirements, true);
        let cleanup_policy = std::mem::replace(&mut self.auto_cleanup_discard, false);
        let result = self.drain_pass_intents();
        self.previewing_crypto_requirements = output_policy;
        self.auto_cleanup_discard = cleanup_policy;
        if let Err(error) = result {
            before.restore(self);
            return Err(error);
        }
        self.cached_snapshot = None;
        let result = self.snapshot();
        if result.is_err() {
            before.restore(self);
        }
        result
    }

    fn continue_priority_yield(&mut self, command: UiCommand) -> Result<JsValue, JsValue> {
        let before = RuntimeSavepoint::capture(self);
        let output = std::mem::replace(&mut self.previewing_crypto_requirements, true);
        let cleanup = std::mem::replace(&mut self.auto_cleanup_discard, false);
        let result = self.dispatch_routed_command(command, 0.0).and_then(|_| {
            if !self.last_crypto_requirements.is_empty()
                || self.active_viewed_cards.is_some()
                || !self.active_audit_viewed_cards.is_empty()
            {
                self.pass_intents_reached_stop();
                if self.active_viewed_cards.is_some() || !self.active_audit_viewed_cards.is_empty()
                {
                    self.pass_intents.clear();
                }
                Ok(())
            } else {
                self.drain_pass_intents()
            }
        });
        self.previewing_crypto_requirements = output;
        self.auto_cleanup_discard = cleanup;
        if let Err(error) = result {
            before.restore(self);
            return Err(error);
        }
        self.cached_snapshot = None;
        let result = self.snapshot();
        if result.is_err() {
            before.restore(self);
        }
        result
    }

    // End turn grants phase passing with no stops. Only that attacker's own
    // consent may answer combat, and mandatory attacks still require a choice.
    fn pass_intent_skips_attackers(&self) -> bool {
        let Some(DecisionContext::Attackers(attackers)) = self.pending_decision.as_ref() else {
            return false;
        };
        self.pass_intents
            .get(&attackers.player.0)
            .is_some_and(|intent| {
                intent.mode == PassMode::Phases
                    && intent.stops.is_empty()
                    && intent.turn == self.game.turn.turn_number
                    && !attackers
                        .attacker_options
                        .iter()
                        .any(|option| option.must_attack)
            })
    }

    fn pass_intents_reached_stop(&mut self) -> bool {
        let (phase, step) = self.pass_window();
        let stack = self.pass_stack_ids();
        let turn = self.game.turn.turn_number;
        let mut stop = self.game_over.is_some()
            || (!matches!(self.pending_decision, Some(DecisionContext::Priority(_)))
                && !self.pass_intent_skips_attackers());
        for intent in self.pass_intents.values_mut() {
            let new_combat =
                phase == "Combat" && step == "BeginCombat" && intent.previous_step != "BeginCombat";
            intent.previous_step = step.clone();
            if intent.starting_phase.as_ref() != Some(&phase) || new_combat {
                intent.starting_phase = None;
            }
            if intent.starting_step.as_ref() != Some(&step) {
                intent.starting_step = None;
            }
            stop |= intent.turn != turn
                || (intent.starting_phase.is_none()
                    && intent.stops.contains(&format!("phase:{phase}")))
                || (intent.starting_step.is_none()
                    && intent.stops.contains(&format!("step:{step}")))
                || (intent.mode == PassMode::Stack
                    && (stack.is_empty() || stack.iter().any(|id| !intent.stack.contains(id))));
        }
        if stop {
            self.pass_intents.clear();
        }
        stop
    }

    fn drain_pass_intents(&mut self) -> Result<(), JsValue> {
        // Bound each worker batch so cancellation and crypto transport can run
        // between chunks. Retained consent lets the current seat continue.
        for _ in 0..4096 {
            if self.pass_intents_reached_stop() {
                return Ok(());
            }
            let command = if self.pass_intent_skips_attackers() {
                UiCommand::DeclareAttackers {
                    declarations: vec![],
                    bands: vec![],
                }
            } else {
                let Some(DecisionContext::Priority(priority)) = self.pending_decision.as_ref()
                else {
                    unreachable!()
                };
                if !self.pass_intents.contains_key(&priority.player.0) {
                    return Ok(());
                }
                if !priority
                    .actions
                    .iter()
                    .any(|a| matches!(a, LegalAction::PassPriority))
                {
                    self.pass_intents.clear();
                    return Ok(());
                }
                UiCommand::PriorityAction {
                    action_index: None,
                    action_ref: Some(PriorityActionRef::PassPriority),
                }
            };
            self.dispatch_routed_command(command, 0.0)?;
            if !self.last_crypto_requirements.is_empty()
                || self.active_viewed_cards.is_some()
                || !self.active_audit_viewed_cards.is_empty()
            {
                self.pass_intents_reached_stop();
                if self.active_viewed_cards.is_some() || !self.active_audit_viewed_cards.is_empty()
                {
                    self.pass_intents.clear();
                }
                return Ok(());
            }
        }
        self.pass_intents_reached_stop();
        Ok(())
    }
}

#[cfg(test)]
mod priority_yield_tests {
    use super::*;
    use ironsmith::turn_runner::{TurnRunner, TurnState};

    fn fixture(stack_count: usize) -> WasmGame {
        let mut wasm = WasmGame::new_with_registry(CardRegistry::new());
        wasm.initialize_empty_match(vec!["A".into(), "B".into()], 20, 1);
        wasm.game.turn.phase = Phase::FirstMain;
        wasm.game.turn.step = None;
        wasm.game.turn.active_player = PlayerId(0);
        wasm.game.turn.priority_player = Some(PlayerId(0));
        wasm.runner = Some(TurnRunner::from_state_for_sync(
            TurnState::FirstMainPriority,
        ));
        wasm.runner_awaiting_priority = true;
        for _ in 0..stack_count {
            wasm.game
                .push_to_stack(ironsmith::game_state::StackEntry::ability(
                    ObjectId(100),
                    PlayerId(0),
                    vec![ironsmith::effect::Effect::gain_life(1)],
                ));
        }
        wasm.pending_decision = Some(DecisionContext::Priority(
            ironsmith::decisions::context::PriorityContext::new(
                &wasm.game,
                PlayerId(0),
                vec![LegalAction::PassPriority],
            )
            .unwrap(),
        ));
        wasm
    }

    fn authorize(wasm: &mut WasmGame, mode: PassMode, stops: &[&str]) {
        wasm.dispatch_typed_command(
            UiCommand::PriorityYield {
                mode,
                stops: stops.iter().map(|s| s.to_string()).collect(),
            },
            0.0,
        )
        .unwrap();
    }

    #[test]
    fn priority_yield_drains_large_stack_only_after_both_seats_authorize() {
        let _ids = crate::test_id_counter_guard();
        let mut wasm = fixture(100);
        authorize(&mut wasm, PassMode::Stack, &[]);
        assert_eq!(wasm.game.stack.len(), 100);
        assert_eq!(wasm.game.turn.priority_player, Some(PlayerId(1)));
        assert_eq!(wasm.pass_intents.len(), 1);
        let serial = wasm.snapshot_serial;
        authorize(&mut wasm, PassMode::Stack, &[]);
        assert!(wasm.game.stack.is_empty());
        assert_eq!(wasm.game.player(PlayerId(0)).unwrap().life, 120);
        assert_eq!(wasm.game.turn.phase, Phase::FirstMain);
        assert!(wasm.pass_intents.is_empty());
        assert_eq!(
            wasm.snapshot_serial,
            serial + 1,
            "one final render, not one per pass"
        );
    }

    #[test]
    fn priority_yield_phases_stop_at_other_seats_earlier_pause() {
        let _ids = crate::test_id_counter_guard();
        let mut wasm = fixture(0);
        authorize(&mut wasm, PassMode::Phases, &["step:BeginCombat"]);
        authorize(&mut wasm, PassMode::Phases, &["phase:Main2"]);
        assert_eq!(wasm.game.turn.step, Some(Step::BeginCombat));
        assert!(wasm.pass_intents.is_empty());
        assert!(matches!(
            wasm.pending_decision,
            Some(DecisionContext::Priority(_))
        ));
    }

    #[test]
    fn end_turn_declares_no_attackers_unless_an_attack_is_required() {
        let _ids = crate::test_id_counter_guard();
        for (has_creature, mandatory, selected_stop) in [
            (false, false, false),
            (true, false, false),
            (true, true, false),
            (true, false, true),
        ] {
            let mut wasm = fixture(0);
            if has_creature {
                let definition = ironsmith::cards::builders::CardDefinitionBuilder::new(
                    CardId::new(),
                    "Attack consent probe",
                )
                .card_types(vec![CardType::Creature])
                .power_toughness(ironsmith::PowerToughness::fixed(2, 2))
                .build();
                let id = wasm.game.create_object_from_definition(
                    &definition,
                    PlayerId(0),
                    Zone::Battlefield,
                );
                wasm.game.remove_summoning_sickness(id);
                if mandatory {
                    wasm.game.object_mut(id).unwrap().abilities =
                        std::sync::Arc::new(vec![ironsmith::ability::Ability::static_ability(
                            ironsmith::static_abilities::StaticAbility::must_attack(),
                            "This creature attacks each combat if able".into(),
                        )]);
                }
            }
            authorize(
                &mut wasm,
                PassMode::Phases,
                if selected_stop {
                    &["step:DeclareAttackers"]
                } else {
                    &[]
                },
            );
            authorize(&mut wasm, PassMode::Phases, &["phase:Main2"]);
            assert!(wasm.pass_intents.is_empty());
            if mandatory || selected_stop {
                assert_eq!(wasm.game.turn.step, Some(Step::DeclareAttackers));
                assert!(matches!(
                    wasm.pending_decision,
                    Some(DecisionContext::Attackers(_))
                ));
            } else {
                assert_eq!(wasm.game.turn.phase, Phase::NextMain);
                assert!(matches!(
                    wasm.pending_decision,
                    Some(DecisionContext::Priority(_))
                ));
                assert_eq!(wasm.game.player(PlayerId(1)).unwrap().life, 20);
            }
        }
    }

    #[test]
    fn priority_yield_stops_for_required_choices_without_answering() {
        let _ids = crate::test_id_counter_guard();
        for choice in [
            DecisionContext::Attackers(ironsmith::decisions::context::AttackersContext::new(
                PlayerId(1),
                vec![],
            )),
            DecisionContext::Number(ironsmith::decisions::context::NumberContext::new(
                PlayerId(1),
                None,
                0,
                5,
                "Choose",
            )),
        ] {
            let mut wasm = fixture(0);
            authorize(&mut wasm, PassMode::Phases, &[]);
            wasm.pending_decision = Some(choice);
            let decision = hash_debug_value(&wasm.pending_decision);
            let saved = hash_debug_value(&wasm.game);
            wasm.drain_pass_intents().unwrap();
            assert!(wasm.pass_intents.is_empty());
            assert_eq!(hash_debug_value(&wasm.pending_decision), decision);
            assert_eq!(hash_debug_value(&wasm.game), saved);
        }
    }

    #[test]
    fn priority_yield_detects_same_size_replacement_from_same_source() {
        let _ids = crate::test_id_counter_guard();
        let mut wasm = fixture(2);
        authorize(&mut wasm, PassMode::Stack, &[]);
        wasm.game.stack.pop();
        wasm.game
            .push_to_stack(ironsmith::game_state::StackEntry::ability(
                ObjectId(100),
                PlayerId(0),
                vec![ironsmith::effect::Effect::gain_life(1)],
            ));
        authorize(&mut wasm, PassMode::Stack, &[]);
        assert_eq!(wasm.game.stack.len(), 2);
        assert_eq!(wasm.game.player(PlayerId(0)).unwrap().life, 20);
        assert!(wasm.pass_intents.is_empty());
    }

    #[test]
    fn priority_yield_preview_and_savepoint_preserve_consent_and_audit() {
        let _ids = crate::test_id_counter_guard();
        let mut wasm = fixture(3);
        authorize(&mut wasm, PassMode::Stack, &[]);
        let before = serde_json::to_value(wasm.build_public_audit_checkpoint()).unwrap();
        let serial = wasm.snapshot_serial;
        let requirements = wasm
            .preview_typed_crypto_requirements(UiCommand::PriorityYield {
                mode: PassMode::Stack,
                stops: vec![],
            })
            .unwrap();
        assert!(requirements.is_empty());
        assert_eq!(wasm.game.stack.len(), 3);
        assert_eq!(wasm.snapshot_serial, serial);
        assert_eq!(
            serde_json::to_value(wasm.build_public_audit_checkpoint()).unwrap(),
            before
        );
        let saved = RuntimeSavepoint::capture(&wasm);
        wasm.dispatch_typed_command(UiCommand::CancelPriorityYield { player: 0 }, 0.0)
            .unwrap();
        assert!(wasm.pass_intents.is_empty());
        saved.restore(&mut wasm);
        assert_eq!(
            serde_json::to_value(wasm.build_public_audit_checkpoint()).unwrap(),
            before
        );
    }
    #[test]
    fn priority_yield_replays_identically_on_peers_and_cancellation_revokes_only_signers_consent() {
        let _ids = crate::test_id_counter_guard();
        let mut a = fixture(5);
        let mut b = fixture(5);
        for wasm in [&mut a, &mut b] {
            authorize(wasm, PassMode::Stack, &[]);
        }
        assert_eq!(
            serde_json::to_value(a.build_public_audit_checkpoint()).unwrap(),
            serde_json::to_value(b.build_public_audit_checkpoint()).unwrap()
        );
        for wasm in [&mut a, &mut b] {
            wasm.dispatch_typed_command(UiCommand::CancelPriorityYield { player: 0 }, 0.0)
                .unwrap();
            authorize(wasm, PassMode::Stack, &[]);
            assert_eq!(
                wasm.game.stack.len(),
                4,
                "one ordinary priority round can resolve, but the revoked seat must stop the batch"
            );
            assert!(!wasm.pass_intents.contains_key(&0));
        }
        assert_eq!(
            serde_json::to_value(a.build_public_audit_checkpoint()).unwrap(),
            serde_json::to_value(b.build_public_audit_checkpoint()).unwrap()
        );
    }
    #[test]
    fn priority_yield_returns_crypto_requirements_before_resolving_more_objects() {
        let _ids = crate::test_id_counter_guard();
        let mut wasm = fixture(2);
        wasm.game.create_hidden_card_placeholder(
            PlayerId(0),
            Zone::Library,
            0,
            "yield-draw".into(),
        );
        wasm.game
            .push_to_stack(ironsmith::game_state::StackEntry::ability(
                ObjectId(100),
                PlayerId(0),
                vec![ironsmith::effect::Effect::draw(1)],
            ));
        authorize(&mut wasm, PassMode::Stack, &[]);
        authorize(&mut wasm, PassMode::Stack, &[]);
        assert_eq!(wasm.game.stack.len(), 2);
        assert_eq!(wasm.game.player(PlayerId(0)).unwrap().life, 20);
        assert!(!wasm.last_crypto_requirements.is_empty());
    }
    #[test]
    fn priority_yield_rearms_phase_pause_for_an_inserted_combat() {
        let _ids = crate::test_id_counter_guard();
        let mut wasm = fixture(0);
        wasm.game.turn.phase = Phase::Combat;
        wasm.game.turn.step = Some(Step::EndCombat);
        authorize(&mut wasm, PassMode::Phases, &["phase:Combat"]);
        wasm.game.turn.step = Some(Step::BeginCombat);
        authorize(&mut wasm, PassMode::Phases, &[]);
        assert!(wasm.pass_intents.is_empty());
        assert_eq!(wasm.game.turn.step, Some(Step::BeginCombat));
    }
}
