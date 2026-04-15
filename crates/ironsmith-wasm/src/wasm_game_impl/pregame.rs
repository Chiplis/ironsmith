
impl WasmGame {
    pub(super) fn initialize_empty_match(&mut self, player_names: Vec<String>, starting_life: i32, seed: u64) {
        self.game = GameState::new_with_runtime_id_reset(player_names, starting_life);
        self.registry = CardRegistry::new();
        self.game.set_random_seed(seed);
        self.match_format = MatchFormatInput::Normal;
        self.pregame = None;
        self.loaded_decks = Vec::new();
    }

    fn populate_demo_libraries(&mut self) -> Result<(), JsValue> {
        let player_ids: Vec<PlayerId> = self.game.players.iter().map(|p| p.id).collect();
        let mut generated_decks = Vec::with_capacity(player_ids.len());
        for player_id in player_ids {
            let deck = self.build_random_demo_deck_names(60, 24)?;
            self.populate_player_library(player_id, &deck)?;
            generated_decks.push(deck);
        }
        self.loaded_decks = generated_decks;
        Ok(())
    }

    fn populate_explicit_libraries(&mut self, decks: &[Vec<String>]) -> Result<(), JsValue> {
        let player_ids: Vec<PlayerId> = self.game.players.iter().map(|p| p.id).collect();
        for (&player_id, deck) in player_ids.iter().zip(decks.iter()) {
            self.populate_player_library(player_id, deck)?;
        }
        self.loaded_decks = decks.to_vec();
        Ok(())
    }

    fn populate_explicit_commanders(&mut self, commanders: &[Vec<String>]) -> Result<(), JsValue> {
        let player_ids: Vec<PlayerId> = self.game.players.iter().map(|p| p.id).collect();
        for (&player_id, commander_names) in player_ids.iter().zip(commanders.iter()) {
            self.registry
                .ensure_cards_loaded(commander_names.iter().map(|name| name.as_str()));

            for name in commander_names {
                let Some(definition) = self.find_card_definition(name).cloned() else {
                    return Err(JsValue::from_str(&format!("unknown card name: {name}")));
                };
                let object_id = self.game.create_object_from_catalog_definition(
                    &definition,
                    &self.registry,
                    player_id,
                    ironsmith::zone::Zone::Command,
                );
                self.game.set_as_commander(object_id, player_id);
            }
        }
        Ok(())
    }

    fn validate_commander_setup(
        &self,
        decks: &[Vec<String>],
        commanders: &[Vec<String>],
    ) -> Result<(), JsValue> {
        if decks.len() != self.game.players.len() {
            return Err(JsValue::from_str(
                "deck count must match number of players in game",
            ));
        }
        if commanders.len() != self.game.players.len() {
            return Err(JsValue::from_str(
                "commander count must match number of players in game",
            ));
        }

        for (deck, commander_list) in decks.iter().zip(commanders.iter()) {
            if !(commander_list.len() == 1 || commander_list.len() == 2) {
                return Err(JsValue::from_str(
                    "commander matches require exactly 1 or 2 commanders per player",
                ));
            }

            let expected_deck_size = if commander_list.len() == 2 { 98 } else { 99 };
            if deck.len() != expected_deck_size {
                return Err(JsValue::from_str(&format!(
                    "commander main decks must contain {expected_deck_size} cards for {count} commander(s)",
                    count = commander_list.len()
                )));
            }
        }

        Ok(())
    }

    fn player_hand_ids(&self, player: PlayerId) -> Vec<ObjectId> {
        self.game
            .player(player)
            .map(|player| player.hand.clone())
            .unwrap_or_default()
    }

    fn build_hand_selectable_objects(
        &self,
        player: PlayerId,
    ) -> Vec<ironsmith::decisions::context::SelectableObject> {
        self.player_hand_ids(player)
            .into_iter()
            .map(|id| {
                let name = self
                    .game
                    .object(id)
                    .map(|object| object.name.clone())
                    .unwrap_or_else(|| format!("Card {}", id.0));
                ironsmith::decisions::context::SelectableObject::new(id, name)
            })
            .collect()
    }

    fn parsed_pregame_begin_on_battlefield_spec(
        &self,
        card_id: ObjectId,
        ability_index: usize,
    ) -> Option<ironsmith::static_abilities::PregameBeginOnBattlefieldSpec> {
        let ability = self.game.object(card_id)?.abilities.get(ability_index)?;
        let ironsmith::ability::AbilityKind::Static(static_ability) = &ability.kind else {
            return None;
        };
        match static_ability.pregame_action_kind()? {
            ironsmith::static_abilities::PregameActionKind::BeginOnBattlefield(spec) => Some(spec),
            ironsmith::static_abilities::PregameActionKind::ChooseColor => None,
        }
    }

    fn available_pregame_actions(&self, player: PlayerId) -> Vec<LegalAction> {
        let starting_player = self.game.turn_store.turn_order.first().copied();
        let hand_ids = self.player_hand_ids(player);
        let other_cards_in_hand = hand_ids.len().saturating_sub(1);
        let mut actions = Vec::new();
        for card_id in hand_ids {
            let Some(object) = self.game.object(card_id) else {
                continue;
            };
            for (ability_index, ability) in object.abilities.iter().enumerate() {
                let ironsmith::ability::AbilityKind::Static(static_ability) = &ability.kind else {
                    continue;
                };
                let Some(ironsmith::static_abilities::PregameActionKind::BeginOnBattlefield(spec)) =
                    static_ability.pregame_action_kind()
                else {
                    continue;
                };
                if spec.require_not_starting_player && starting_player == Some(player) {
                    continue;
                }
                if other_cards_in_hand < spec.exile_cards_from_hand {
                    continue;
                }
                actions.push(LegalAction::UsePregameAction {
                    card_id,
                    ability_index,
                });
            }
        }
        actions
    }

    fn shuffle_hand_into_library_and_draw(&mut self, player: PlayerId, opening_hand_size: usize) {
        let hand_ids = self.player_hand_ids(player);
        for id in hand_ids {
            let _ = self.game.move_object_by_effect(id, Zone::Library);
        }
        self.game.shuffle_player_library(player);
        let _ = self.game.draw_cards(player, opening_hand_size);
    }

    fn move_cards_to_library_bottom(&mut self, ordered_cards_bottom_first: &[ObjectId]) {
        for card_id in ordered_cards_bottom_first.iter().rev().copied() {
            let Some(owner) = self.game.object(card_id).map(|object| object.owner) else {
                continue;
            };
            let Some(new_id) = self.game.move_object_by_effect(card_id, Zone::Library) else {
                continue;
            };
            let Some(player) = self.game.player_mut(owner) else {
                continue;
            };
            let Some(index) = player
                .library
                .iter()
                .rposition(|candidate| *candidate == new_id)
            else {
                continue;
            };
            let moved = player.library.remove(index);
            player.library.insert(0, moved);
        }
    }

    fn normalize_pregame_state(&mut self) -> Result<(), JsValue> {
        loop {
            let Some(pregame) = self.pregame.as_ref() else {
                return Ok(());
            };

            match &pregame.stage {
                PregameStage::MulliganDecision {
                    undecided_players,
                    round_mulliganers,
                } if undecided_players.is_empty() => {
                    if round_mulliganers.is_empty() {
                        let queue = self
                            .game
                            .turn_store
                            .turn_order
                            .iter()
                            .copied()
                            .filter(|player| pregame.cards_to_bottom(*player) > 0)
                            .collect();
                        if let Some(pregame) = self.pregame.as_mut() {
                            pregame.stage = PregameStage::BottomCards {
                                queue,
                                pending_order: None,
                            };
                        }
                        continue;
                    }

                    let opening_hand_size = pregame.opening_hand_size;
                    let mulliganers = round_mulliganers.clone();
                    if let Some(pregame) = self.pregame.as_mut() {
                        for player in &mulliganers {
                            *pregame.mulligans_taken.entry(*player).or_insert(0) += 1;
                        }
                    }
                    for player in mulliganers.iter().copied() {
                        self.shuffle_hand_into_library_and_draw(player, opening_hand_size);
                    }
                    if let Some(pregame) = self.pregame.as_mut() {
                        pregame.stage = PregameStage::MulliganDecision {
                            undecided_players: mulliganers,
                            round_mulliganers: Vec::new(),
                        };
                    }
                    continue;
                }
                PregameStage::BottomCards {
                    queue,
                    pending_order,
                } if queue.is_empty() && pending_order.is_none() => {
                    if let Some(pregame) = self.pregame.as_mut() {
                        pregame.stage = PregameStage::OpeningActions {
                            current_index: 0,
                            pending_hand_exile: None,
                        };
                    }
                    continue;
                }
                PregameStage::OpeningActions {
                    current_index,
                    pending_hand_exile,
                } if pending_hand_exile.is_none()
                    && *current_index >= self.game.turn_store.turn_order.len() =>
                {
                    self.pregame = None;
                    continue;
                }
                _ => return Ok(()),
            }
        }
    }

    fn build_pregame_decision(&self) -> Result<Option<DecisionContext>, JsValue> {
        let Some(pregame) = self.pregame.as_ref() else {
            return Ok(None);
        };

        let ctx = match &pregame.stage {
            PregameStage::MulliganDecision {
                undecided_players, ..
            } => {
                let Some(player) = undecided_players.first().copied() else {
                    return Ok(None);
                };
                let mut actions = vec![LegalAction::KeepOpeningHand, LegalAction::TakeMulligan];
                actions.extend(
                    self.player_hand_ids(player)
                        .into_iter()
                        .filter_map(|card_id| {
                            let is_serum_powder = self
                                .game
                                .object(card_id)
                                .is_some_and(|object| object.name == "Serum Powder");
                            is_serum_powder.then_some(LegalAction::SerumPowderMulligan { card_id })
                        }),
                );
                DecisionContext::Priority(ironsmith::decisions::context::PriorityContext::new(
                    player, actions,
                ))
            }
            PregameStage::BottomCards {
                queue,
                pending_order,
            } => {
                if let Some((player, selected_cards)) = pending_order {
                    let items = selected_cards
                        .iter()
                        .filter_map(|id| {
                            self.game
                                .object(*id)
                                .map(|object| (*id, object.name.clone()))
                        })
                        .collect();
                    DecisionContext::Order(ironsmith::decisions::context::OrderContext::new(
                        *player,
                        None,
                        "Order the selected cards for the bottom of your library. The first option becomes the bottom-most card.",
                        items,
                    ))
                } else {
                    let Some(player) = queue.first().copied() else {
                        return Ok(None);
                    };
                    let amount = pregame.cards_to_bottom(player);
                    DecisionContext::SelectObjects(
                        ironsmith::decisions::context::SelectObjectsContext::new(
                            player,
                            None,
                            format!("Choose {amount} card(s) to put on the bottom of your library"),
                            self.build_hand_selectable_objects(player),
                            amount,
                            Some(amount),
                        ),
                    )
                }
            }
            PregameStage::OpeningActions {
                current_index,
                pending_hand_exile,
            } => {
                let Some(player) = self.game.turn_store.turn_order.get(*current_index).copied()
                else {
                    return Ok(None);
                };
                if let Some(pending_exile) = pending_hand_exile {
                    if pending_exile.player != player {
                        return Err(JsValue::from_str(
                            "pregame hand exile prompt is out of sync with turn order",
                        ));
                    }
                    let source_name = self
                        .game
                        .object(pending_exile.source)
                        .map(|object| object.name.as_str())
                        .unwrap_or("this card");
                    DecisionContext::SelectObjects(
                        ironsmith::decisions::context::SelectObjectsContext::new(
                            player,
                            Some(pending_exile.source),
                            format!(
                                "Choose {} card(s) from your hand to exile for {}",
                                pending_exile.amount, source_name
                            ),
                            self.build_hand_selectable_objects(player),
                            pending_exile.amount,
                            Some(pending_exile.amount),
                        ),
                    )
                } else {
                    let is_last_player =
                        *current_index + 1 >= self.game.turn_store.turn_order.len();
                    let mut actions = vec![if is_last_player {
                        LegalAction::BeginGame
                    } else {
                        LegalAction::ContinuePregame
                    }];
                    actions.extend(self.available_pregame_actions(player));
                    DecisionContext::Priority(ironsmith::decisions::context::PriorityContext::new(
                        player, actions,
                    ))
                }
            }
        };

        Ok(Some(ctx))
    }

    fn apply_pregame_priority_action(&mut self, action: LegalAction) -> Result<(), JsValue> {
        match action {
            LegalAction::KeepOpeningHand => {
                let Some(PregameState {
                    stage:
                        PregameStage::MulliganDecision {
                            undecided_players, ..
                        },
                    ..
                }) = self.pregame.as_mut()
                else {
                    return Err(JsValue::from_str(
                        "keep hand is only legal during mulligan decisions",
                    ));
                };
                if undecided_players.is_empty() {
                    return Err(JsValue::from_str(
                        "no player is waiting on a mulligan decision",
                    ));
                }
                undecided_players.remove(0);
            }
            LegalAction::TakeMulligan => {
                let Some(PregameState {
                    stage:
                        PregameStage::MulliganDecision {
                            undecided_players,
                            round_mulliganers,
                        },
                    ..
                }) = self.pregame.as_mut()
                else {
                    return Err(JsValue::from_str(
                        "mulligan is only legal during mulligan decisions",
                    ));
                };
                let Some(player) = undecided_players.first().copied() else {
                    return Err(JsValue::from_str(
                        "no player is waiting on a mulligan decision",
                    ));
                };
                undecided_players.remove(0);
                round_mulliganers.push(player);
            }
            LegalAction::SerumPowderMulligan { card_id } => {
                let player = match self.pregame.as_ref() {
                    Some(PregameState {
                        stage:
                            PregameStage::MulliganDecision {
                                undecided_players, ..
                            },
                        ..
                    }) => undecided_players.first().copied(),
                    _ => None,
                }
                .ok_or_else(|| {
                    JsValue::from_str("Serum Powder can only be used while mulliganing")
                })?;
                let hand_ids = self.player_hand_ids(player);
                if !hand_ids.contains(&card_id) {
                    return Err(JsValue::from_str(
                        "Serum Powder must be in the current player's hand",
                    ));
                }
                let is_serum_powder = self
                    .game
                    .object(card_id)
                    .is_some_and(|object| object.name == "Serum Powder");
                if !is_serum_powder {
                    return Err(JsValue::from_str("selected card is not Serum Powder"));
                }
                let draw_count = hand_ids.len();
                for id in hand_ids {
                    let _ = self.game.move_object_by_effect(id, Zone::Exile);
                }
                let _ = self.game.draw_cards(player, draw_count);
            }
            LegalAction::ContinuePregame | LegalAction::BeginGame => {
                let Some(PregameState {
                    stage:
                        PregameStage::OpeningActions {
                            current_index,
                            pending_hand_exile,
                        },
                    ..
                }) = self.pregame.as_mut()
                else {
                    return Err(JsValue::from_str(
                        "continue is only legal during pregame opening actions",
                    ));
                };
                if pending_hand_exile.is_some() {
                    return Err(JsValue::from_str(
                        "a pregame action requires exiling cards before continuing",
                    ));
                }
                *current_index += 1;
            }
            LegalAction::UsePregameAction {
                card_id,
                ability_index,
            } => {
                let player = match self.pregame.as_ref() {
                    Some(PregameState {
                        stage:
                            PregameStage::OpeningActions {
                                current_index,
                                pending_hand_exile: None,
                            },
                        ..
                    }) => self.game.turn_store.turn_order.get(*current_index).copied(),
                    _ => None,
                }
                .ok_or_else(|| {
                    JsValue::from_str(
                        "pregame actions can only be used during pregame opening actions",
                    )
                })?;
                let hand_ids = self.player_hand_ids(player);
                if !hand_ids.contains(&card_id) {
                    return Err(JsValue::from_str(
                        "pregame action source must be in the current player's hand",
                    ));
                }
                let Some(spec) =
                    self.parsed_pregame_begin_on_battlefield_spec(card_id, ability_index)
                else {
                    return Err(JsValue::from_str(
                        "selected ability is not a supported pregame action",
                    ));
                };
                if spec.require_not_starting_player
                    && self.game.turn_store.turn_order.first().copied() == Some(player)
                {
                    return Err(JsValue::from_str(
                        "the starting player can't use that pregame action",
                    ));
                }
                if hand_ids.len().saturating_sub(1) < spec.exile_cards_from_hand {
                    return Err(JsValue::from_str(
                        "that pregame action requires more cards in hand to exile",
                    ));
                }
                let exile_cards_from_hand = spec.exile_cards_from_hand;
                let Some(new_id) = self.game.move_object_by_effect(card_id, Zone::Battlefield)
                else {
                    return Err(JsValue::from_str(
                        "failed to move the pregame card to the battlefield",
                    ));
                };
                for (counter_type, count) in spec.counters.iter().cloned() {
                    let _ = self.game.add_counters(new_id, counter_type, count);
                }
                let Some(PregameState {
                    stage:
                        PregameStage::OpeningActions {
                            pending_hand_exile, ..
                        },
                    ..
                }) = self.pregame.as_mut()
                else {
                    return Err(JsValue::from_str(
                        "pregame opening actions disappeared while resolving a pregame action",
                    ));
                };
                *pending_hand_exile =
                    (exile_cards_from_hand > 0).then_some(PendingPregameHandExile {
                        player,
                        source: new_id,
                        amount: exile_cards_from_hand,
                    });
            }
            other => {
                return Err(JsValue::from_str(&format!(
                    "illegal pregame priority action: {other:?}"
                )));
            }
        }

        Ok(())
    }

    fn dispatch_pregame_decision(
        &mut self,
        pending_ctx: DecisionContext,
        command: UiCommand,
    ) -> Result<JsValue, JsValue> {
        let restore =
            |this: &mut Self, ctx: DecisionContext, err: JsValue| -> Result<JsValue, JsValue> {
                this.pending_decision = Some(ctx);
                Err(err)
            };

        match (&pending_ctx, command) {
            (
                DecisionContext::Priority(priority),
                UiCommand::PriorityAction {
                    action_index,
                    action_ref,
                },
            ) => {
                let action = resolve_priority_action(priority, action_index, action_ref.as_ref())
                    .ok_or_else(|| {
                        if let Some(action_ref) = action_ref.as_ref() {
                            JsValue::from_str(&format!(
                                "invalid priority action ref: {action_ref:?}"
                            ))
                        } else if let Some(action_index) = action_index {
                            JsValue::from_str(&format!(
                                "invalid priority action index: {action_index}"
                            ))
                        } else {
                            JsValue::from_str("missing priority action selector")
                        }
                    });
                let action = match action {
                    Ok(action) => action,
                    Err(err) => return restore(self, pending_ctx, err),
                };
                if let Err(err) = self.apply_pregame_priority_action(action) {
                    return restore(self, pending_ctx, err);
                }
            }
            (DecisionContext::SelectObjects(objects), UiCommand::SelectObjects { object_ids }) => {
                let legal_ids: Vec<u64> = objects
                    .candidates
                    .iter()
                    .filter(|candidate| candidate.legal)
                    .map(|candidate| candidate.id.0)
                    .collect();
                if let Err(err) = validate_object_selection(
                    objects.min,
                    objects.max,
                    objects.allow_partial_completion,
                    &object_ids,
                    &legal_ids,
                ) {
                    return restore(self, pending_ctx, err);
                }
                let selected: Vec<ObjectId> =
                    object_ids.into_iter().map(ObjectId::from_raw).collect();
                enum PregameSelectResolution {
                    BottomNow,
                    BottomNeedsOrdering(PlayerId),
                    HandExile(Vec<ObjectId>),
                }

                let resolution = match self.pregame.as_ref().map(|pregame| &pregame.stage) {
                    Some(PregameStage::BottomCards {
                        queue,
                        pending_order,
                    }) if pending_order.is_none() => {
                        let Some(player) = queue.first().copied() else {
                            return restore(
                                self,
                                pending_ctx,
                                JsValue::from_str("no player is waiting to bottom cards"),
                            );
                        };
                        if selected.len() <= 1 {
                            PregameSelectResolution::BottomNow
                        } else {
                            PregameSelectResolution::BottomNeedsOrdering(player)
                        }
                    }
                    Some(PregameStage::OpeningActions {
                        pending_hand_exile, ..
                    }) if pending_hand_exile.is_some() => {
                        if selected.is_empty() {
                            return restore(
                                self,
                                pending_ctx,
                                JsValue::from_str("expected at least one card to exile"),
                            );
                        }
                        PregameSelectResolution::HandExile(selected.clone())
                    }
                    _ => {
                        return restore(
                            self,
                            pending_ctx,
                            JsValue::from_str("unexpected select_objects command during pregame"),
                        );
                    }
                };

                match resolution {
                    PregameSelectResolution::BottomNow => {
                        self.move_cards_to_library_bottom(&selected);
                        let Some(PregameStage::BottomCards { queue, .. }) =
                            self.pregame.as_mut().map(|pregame| &mut pregame.stage)
                        else {
                            return restore(
                                self,
                                pending_ctx,
                                JsValue::from_str("pregame bottoming state disappeared"),
                            );
                        };
                        if !queue.is_empty() {
                            queue.remove(0);
                        }
                    }
                    PregameSelectResolution::BottomNeedsOrdering(player) => {
                        let Some(PregameStage::BottomCards { pending_order, .. }) =
                            self.pregame.as_mut().map(|pregame| &mut pregame.stage)
                        else {
                            return restore(
                                self,
                                pending_ctx,
                                JsValue::from_str("pregame bottoming state disappeared"),
                            );
                        };
                        *pending_order = Some((player, selected));
                    }
                    PregameSelectResolution::HandExile(card_ids) => {
                        for card_id in card_ids {
                            let _ = self.game.move_object_by_effect(card_id, Zone::Exile);
                        }
                        let Some(PregameStage::OpeningActions {
                            pending_hand_exile, ..
                        }) = self.pregame.as_mut().map(|pregame| &mut pregame.stage)
                        else {
                            return restore(
                                self,
                                pending_ctx,
                                JsValue::from_str("pregame hand exile state disappeared"),
                            );
                        };
                        *pending_hand_exile = None;
                    }
                }
            }
            (DecisionContext::Order(order), UiCommand::SelectOptions { option_indices }) => {
                let legal: Vec<usize> = (0..order.items.len()).collect();
                if let Err(err) = validate_option_selection(
                    order.items.len(),
                    Some(order.items.len()),
                    &option_indices,
                    &legal,
                ) {
                    return restore(self, pending_ctx, err);
                }
                if unique_indices(&option_indices).len() != order.items.len() {
                    return restore(
                        self,
                        pending_ctx,
                        JsValue::from_str("ordering requires each option index exactly once"),
                    );
                }
                let selected_cards = match self.pregame.as_mut().map(|pregame| &mut pregame.stage) {
                    Some(PregameStage::BottomCards { pending_order, .. }) => {
                        let Some((_, selected_cards)) = pending_order.take() else {
                            return restore(
                                self,
                                pending_ctx,
                                JsValue::from_str("no selected cards are waiting to be ordered"),
                            );
                        };
                        selected_cards
                    }
                    _ => {
                        return restore(
                            self,
                            pending_ctx,
                            JsValue::from_str("unexpected ordering command during pregame"),
                        );
                    }
                };
                let ordered_cards: Vec<ObjectId> = option_indices
                    .into_iter()
                    .filter_map(|index| selected_cards.get(index).copied())
                    .collect();
                self.move_cards_to_library_bottom(&ordered_cards);
                if let Some(PregameStage::BottomCards { queue, .. }) =
                    self.pregame.as_mut().map(|pregame| &mut pregame.stage)
                    && !queue.is_empty()
                {
                    queue.remove(0);
                }
            }
            _ => {
                return restore(
                    self,
                    pending_ctx,
                    JsValue::from_str("command type does not match pregame decision"),
                );
            }
        }

        self.pending_decision = None;
        self.advance_until_decision()?;
        self.snapshot()
    }

    fn finish_match_setup(&mut self, opening_hand_size: usize) -> Result<(), JsValue> {
        self.reset_runtime_state();
        let player_ids: Vec<PlayerId> = self.game.players.iter().map(|p| p.id).collect();
        for player_id in player_ids {
            let _ = self.game.draw_cards(player_id, opening_hand_size);
        }
        self.pregame = Some(PregameState::new(
            &self.game.turn_store.turn_order,
            opening_hand_size,
            self.match_format,
        ));
        self.recompute_ui_decision()
    }

    fn reset_runtime_state(&mut self) {
        self.trigger_queue = TriggerQueue::new();
        self.priority_state = PriorityLoopState::new(self.game.players.len());
        self.priority_state
            .set_auto_choose_single_pip_payment(false);
        self.pregame = None;
        self.pending_decision = None;
        self.pending_replay_action = None;
        self.pending_action_checkpoint = None;
        self.pending_live_action_root = None;
        self.priority_epoch_checkpoint = None;
        self.priority_epoch_has_undoable_action = false;
        self.priority_epoch_undo_locked_by_mana = false;
        self.priority_epoch_undo_land_stable_id = None;
        self.active_viewed_cards = None;
        self.clear_active_resolving_stack_object();
        self.game_over = None;
        self.last_snapshot_perf = None;
        self.last_replay_execution_perf = None;
        self.last_advance_until_decision_perf = None;
        self.last_dispatch_perf = None;
        self.runner = None;
        self.runner_awaiting_priority = false;
        self.runner_pending_decision = false;
        if self.game.player(self.perspective).is_none()
            && let Some(first) = self.game.players.first()
        {
            self.perspective = first.id;
        }
    }

}
