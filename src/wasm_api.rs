//! WASM-facing API for browser integration.
//!
//! This module provides a small wrapper around `GameState` so JavaScript can:
//! - create/reset a game
//! - mutate a bit of state
//! - read a serializable snapshot

use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

use crate::cards::CardRegistry;
use crate::decision::{GameProgress, GameResult, LegalAction};
use crate::decisions::context::DecisionContext;
use crate::game_loop::{
    ActivationStage, CastStage, PriorityLoopState, PriorityResponse, advance_priority,
    apply_priority_response,
};
use crate::game_state::GameState;
use crate::game_state::Target;
use crate::ids::{ObjectId, PlayerId};
use crate::triggers::TriggerQueue;

#[derive(Debug, Clone, Serialize)]
struct PermanentSnapshot {
    id: u64,
    name: String,
    tapped: bool,
}

#[derive(Debug, Clone, Serialize)]
struct PlayerSnapshot {
    id: u8,
    name: String,
    life: i32,
    can_view_hand: bool,
    hand_size: usize,
    library_size: usize,
    graveyard_size: usize,
    hand_cards: Vec<String>,
    library_top: Option<String>,
    graveyard_top: Option<String>,
    battlefield: Vec<PermanentSnapshot>,
}

#[derive(Debug, Clone, Serialize)]
struct GameSnapshot {
    perspective: u8,
    turn_number: u32,
    active_player: u8,
    priority_player: Option<u8>,
    phase: String,
    step: Option<String>,
    stack_size: usize,
    stack_preview: Vec<String>,
    battlefield_size: usize,
    exile_size: usize,
    players: Vec<PlayerSnapshot>,
    decision: Option<DecisionView>,
    game_over: Option<GameOverView>,
}

impl GameSnapshot {
    fn from_game(
        game: &GameState,
        perspective: PlayerId,
        decision: Option<&DecisionContext>,
        game_over: Option<&GameResult>,
    ) -> Self {
        let players = game
            .players
            .iter()
            .map(|p| PlayerSnapshot {
                can_view_hand: p.id == perspective,
                hand_cards: if p.id == perspective {
                    p.hand
                        .iter()
                        .rev()
                        .take(12)
                        .filter_map(|id| game.object(*id))
                        .map(|o| o.name.clone())
                        .collect()
                } else {
                    Vec::new()
                },
                library_top: p
                    .library
                    .last()
                    .and_then(|id| game.object(*id))
                    .map(|o| o.name.clone()),
                graveyard_top: p
                    .graveyard
                    .last()
                    .and_then(|id| game.object(*id))
                    .map(|o| o.name.clone()),
                battlefield: game
                    .battlefield
                    .iter()
                    .filter_map(|id| game.object(*id))
                    .filter(|obj| obj.controller == p.id)
                    .map(|obj| PermanentSnapshot {
                        id: obj.id.0,
                        name: obj.name.clone(),
                        tapped: game.is_tapped(obj.id),
                    })
                    .collect(),
                id: p.id.0,
                name: p.name.clone(),
                life: p.life,
                hand_size: p.hand.len(),
                library_size: p.library.len(),
                graveyard_size: p.graveyard.len(),
            })
            .collect();

        Self {
            perspective: perspective.0,
            turn_number: game.turn.turn_number,
            active_player: game.turn.active_player.0,
            priority_player: game.turn.priority_player.map(|p| p.0),
            phase: format!("{:?}", game.turn.phase),
            step: game.turn.step.map(|step| format!("{:?}", step)),
            stack_size: game.stack.len(),
            stack_preview: game
                .stack
                .iter()
                .rev()
                .take(4)
                .map(|entry| {
                    game.object(entry.object_id)
                        .map(|obj| obj.name.clone())
                        .unwrap_or_else(|| format!("Object#{}", entry.object_id.0))
                })
                .collect(),
            battlefield_size: game.battlefield.len(),
            exile_size: game.exile.len(),
            players,
            decision: decision.map(|ctx| DecisionView::from_context(game, ctx)),
            game_over: game_over.map(|r| GameOverView::from_result(game, r)),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
struct ActionView {
    index: usize,
    label: String,
}

#[derive(Debug, Clone, Serialize)]
struct OptionView {
    index: usize,
    description: String,
    legal: bool,
}

#[derive(Debug, Clone, Serialize)]
struct ObjectChoiceView {
    id: u64,
    name: String,
    legal: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum TargetChoiceView {
    Player { player: u8, name: String },
    Object { object: u64, name: String },
}

#[derive(Debug, Clone, Serialize)]
struct TargetRequirementView {
    description: String,
    min_targets: usize,
    max_targets: Option<usize>,
    legal_targets: Vec<TargetChoiceView>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum DecisionView {
    Priority {
        player: u8,
        actions: Vec<ActionView>,
        commander_actions: Vec<ActionView>,
    },
    Number {
        player: u8,
        description: String,
        min: u32,
        max: u32,
        is_x_value: bool,
    },
    SelectOptions {
        player: u8,
        description: String,
        min: usize,
        max: usize,
        options: Vec<OptionView>,
    },
    SelectObjects {
        player: u8,
        description: String,
        min: usize,
        max: Option<usize>,
        candidates: Vec<ObjectChoiceView>,
    },
    Targets {
        player: u8,
        context: String,
        requirements: Vec<TargetRequirementView>,
    },
}

impl DecisionView {
    fn from_context(game: &GameState, ctx: &DecisionContext) -> Self {
        match ctx {
            DecisionContext::Priority(priority) => DecisionView::Priority {
                player: priority.player.0,
                actions: priority
                    .legal_actions
                    .iter()
                    .enumerate()
                    .map(|(index, action)| ActionView {
                        index,
                        label: describe_action(game, action),
                    })
                    .collect(),
                commander_actions: priority
                    .commander_actions
                    .iter()
                    .enumerate()
                    .map(|(index, action)| ActionView {
                        index,
                        label: describe_action(game, action),
                    })
                    .collect(),
            },
            DecisionContext::Number(number) => DecisionView::Number {
                player: number.player.0,
                description: number.description.clone(),
                min: number.min,
                max: number.max,
                is_x_value: number.is_x_value,
            },
            DecisionContext::SelectOptions(options) => DecisionView::SelectOptions {
                player: options.player.0,
                description: options.description.clone(),
                min: options.min,
                max: options.max,
                options: options
                    .options
                    .iter()
                    .map(|opt| OptionView {
                        index: opt.index,
                        description: opt.description.clone(),
                        legal: opt.legal,
                    })
                    .collect(),
            },
            DecisionContext::Modes(modes) => DecisionView::SelectOptions {
                player: modes.player.0,
                description: format!("Choose mode for {}", modes.spell_name),
                min: modes.spec.min_modes,
                max: modes.spec.max_modes,
                options: modes
                    .spec
                    .modes
                    .iter()
                    .map(|mode| OptionView {
                        index: mode.index,
                        description: mode.description.clone(),
                        legal: mode.legal,
                    })
                    .collect(),
            },
            DecisionContext::HybridChoice(hybrid) => DecisionView::SelectOptions {
                player: hybrid.player.0,
                description: format!(
                    "Choose how to pay pip {} of {}",
                    hybrid.pip_number, hybrid.spell_name
                ),
                min: 1,
                max: 1,
                options: hybrid
                    .options
                    .iter()
                    .map(|opt| OptionView {
                        index: opt.index,
                        description: opt.label.clone(),
                        legal: true,
                    })
                    .collect(),
            },
            DecisionContext::SelectObjects(objects) => DecisionView::SelectObjects {
                player: objects.player.0,
                description: objects.description.clone(),
                min: objects.min,
                max: objects.max,
                candidates: objects
                    .candidates
                    .iter()
                    .map(|obj| ObjectChoiceView {
                        id: obj.id.0,
                        name: obj.name.clone(),
                        legal: obj.legal,
                    })
                    .collect(),
            },
            DecisionContext::Targets(targets) => DecisionView::Targets {
                player: targets.player.0,
                context: targets.context.clone(),
                requirements: targets
                    .requirements
                    .iter()
                    .map(|req| TargetRequirementView {
                        description: req.description.clone(),
                        min_targets: req.min_targets,
                        max_targets: req.max_targets,
                        legal_targets: req
                            .legal_targets
                            .iter()
                            .map(|target| target_choice_view(game, target))
                            .collect(),
                    })
                    .collect(),
            },
            _ => DecisionView::SelectOptions {
                player: game.turn.active_player.0,
                description: "Unsupported decision context for web UI".to_string(),
                min: 0,
                max: 0,
                options: Vec::new(),
            },
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum GameOverView {
    Winner { player: u8, name: String },
    Draw,
    Remaining { players: Vec<u8> },
}

impl GameOverView {
    fn from_result(game: &GameState, result: &GameResult) -> Self {
        match result {
            GameResult::Winner(player) => GameOverView::Winner {
                player: player.0,
                name: game
                    .player(*player)
                    .map(|p| p.name.clone())
                    .unwrap_or_else(|| format!("Player {}", player.0 + 1)),
            },
            GameResult::Draw => GameOverView::Draw,
            GameResult::Remaining(players) => GameOverView::Remaining {
                players: players.iter().map(|p| p.0).collect(),
            },
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum UiCommand {
    PriorityAction { action_index: usize },
    NumberChoice { value: u32 },
    SelectOptions { option_indices: Vec<usize> },
    SelectObjects { object_ids: Vec<u64> },
    SelectTargets { targets: Vec<TargetInput> },
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum TargetInput {
    Player { player: u8 },
    Object { object: u64 },
}

/// Browser-exposed game handle.
#[wasm_bindgen]
pub struct WasmGame {
    game: GameState,
    registry: CardRegistry,
    trigger_queue: TriggerQueue,
    priority_state: PriorityLoopState,
    pending_decision: Option<DecisionContext>,
    game_over: Option<GameResult>,
    perspective: PlayerId,
}

#[wasm_bindgen(start)]
pub fn wasm_start() {
    console_error_panic_hook::set_once();
}

#[wasm_bindgen]
impl WasmGame {
    /// Construct a demo game with two players.
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        let mut instance = Self {
            game: GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20),
            registry: CardRegistry::with_builtin_cards(),
            trigger_queue: TriggerQueue::new(),
            priority_state: PriorityLoopState::new(2),
            pending_decision: None,
            game_over: None,
            perspective: PlayerId::from_index(0),
        };
        // Start demo with real decks so UI interactions can immediately draw real cards.
        let _ = instance.load_demo_decks();
        let _ = instance.draw_opening_hands(7);
        let _ = instance.recompute_ui_decision();
        instance
    }

    /// Reset game with custom player names and starting life.
    #[wasm_bindgen(js_name = reset)]
    pub fn reset_from_js(
        &mut self,
        player_names: JsValue,
        starting_life: i32,
    ) -> Result<(), JsValue> {
        let names: Vec<String> = serde_wasm_bindgen::from_value(player_names)
            .map_err(|e| JsValue::from_str(&format!("invalid player_names: {e}")))?;

        if names.is_empty() {
            return Err(JsValue::from_str("player_names cannot be empty"));
        }

        self.game = GameState::new(names, starting_life);
        self.load_demo_decks()?;
        self.draw_opening_hands(7)?;
        self.reset_runtime_state();
        self.recompute_ui_decision()?;
        Ok(())
    }

    /// Return a JS object snapshot of public game state.
    #[wasm_bindgen]
    pub fn snapshot(&self) -> Result<JsValue, JsValue> {
        let snap = GameSnapshot::from_game(
            &self.game,
            self.perspective,
            self.pending_decision.as_ref(),
            self.game_over.as_ref(),
        );
        serde_wasm_bindgen::to_value(&snap)
            .map_err(|e| JsValue::from_str(&format!("snapshot encode failed: {e}")))
    }

    /// Return the current UI state from the selected player perspective.
    #[wasm_bindgen(js_name = uiState)]
    pub fn ui_state(&self) -> Result<JsValue, JsValue> {
        self.snapshot()
    }

    /// Return game snapshot as pretty JSON.
    #[wasm_bindgen(js_name = snapshotJson)]
    pub fn snapshot_json(&self) -> Result<String, JsValue> {
        let snap = GameSnapshot::from_game(
            &self.game,
            self.perspective,
            self.pending_decision.as_ref(),
            self.game_over.as_ref(),
        );
        serde_json::to_string_pretty(&snap)
            .map_err(|e| JsValue::from_str(&format!("json encode failed: {e}")))
    }

    /// Set a player's life total.
    #[wasm_bindgen(js_name = setLife)]
    pub fn set_life(&mut self, player_index: u8, life: i32) -> Result<(), JsValue> {
        let player_id = PlayerId::from_index(player_index);
        let Some(player) = self.game.player_mut(player_id) else {
            return Err(JsValue::from_str("invalid player index"));
        };
        player.life = life;
        self.recompute_ui_decision()?;
        Ok(())
    }

    /// Add a signed life delta (negative = damage, positive = gain).
    #[wasm_bindgen(js_name = addLifeDelta)]
    pub fn add_life_delta(&mut self, player_index: u8, delta: i32) -> Result<(), JsValue> {
        let player_id = PlayerId::from_index(player_index);
        let Some(player) = self.game.player_mut(player_id) else {
            return Err(JsValue::from_str("invalid player index"));
        };
        player.life += delta;
        self.recompute_ui_decision()?;
        Ok(())
    }

    /// Draw one card for a player.
    #[wasm_bindgen(js_name = drawCard)]
    pub fn draw_card(&mut self, player_index: u8) -> Result<usize, JsValue> {
        let player_id = PlayerId::from_index(player_index);
        if self.game.player(player_id).is_none() {
            return Err(JsValue::from_str("invalid player index"));
        }
        let drawn = self.game.draw_cards(player_id, 1);
        self.recompute_ui_decision()?;
        Ok(drawn.len())
    }

    /// Draw opening hands for all players.
    #[wasm_bindgen(js_name = drawOpeningHands)]
    pub fn draw_opening_hands(&mut self, cards_per_player: usize) -> Result<(), JsValue> {
        let player_ids: Vec<PlayerId> = self.game.players.iter().map(|p| p.id).collect();
        for player_id in player_ids {
            let _ = self.game.draw_cards(player_id, cards_per_player);
        }
        self.recompute_ui_decision()?;
        Ok(())
    }

    /// Replace game state with demo decks and no battlefield/stack state.
    #[wasm_bindgen(js_name = loadDemoDecks)]
    pub fn load_demo_decks(&mut self) -> Result<(), JsValue> {
        let names: Vec<String> = self.game.players.iter().map(|p| p.name.clone()).collect();
        let starting_life = self.game.players.first().map_or(20, |p| p.life);
        self.game = GameState::new(names, starting_life);

        let player_ids: Vec<PlayerId> = self.game.players.iter().map(|p| p.id).collect();

        for (idx, player_id) in player_ids.into_iter().enumerate() {
            let land = if idx % 2 == 0 { "Forest" } else { "Island" };
            let deck: Vec<String> = std::iter::repeat_n(land.to_string(), 24)
                .chain(std::iter::repeat_n("Grizzly Bears".to_string(), 16))
                .chain(std::iter::repeat_n("Llanowar Elves".to_string(), 10))
                .chain(std::iter::repeat_n("Lightning Bolt".to_string(), 10))
                .collect();

            self.populate_player_library(player_id, &deck)?;
        }

        self.reset_runtime_state();
        self.recompute_ui_decision()?;
        Ok(())
    }

    /// Load explicit decks by card name. JS format: `string[][]`.
    ///
    /// Deck list index maps to player index.
    #[wasm_bindgen(js_name = loadDecks)]
    pub fn load_decks(&mut self, decks_js: JsValue) -> Result<(), JsValue> {
        let decks: Vec<Vec<String>> = serde_wasm_bindgen::from_value(decks_js)
            .map_err(|e| JsValue::from_str(&format!("invalid decks payload: {e}")))?;

        if decks.len() != self.game.players.len() {
            return Err(JsValue::from_str(
                "deck count must match number of players in game",
            ));
        }

        let names: Vec<String> = self.game.players.iter().map(|p| p.name.clone()).collect();
        let starting_life = self.game.players.first().map_or(20, |p| p.life);
        self.game = GameState::new(names, starting_life);

        let player_ids: Vec<PlayerId> = self.game.players.iter().map(|p| p.id).collect();
        for (player_id, deck) in player_ids.into_iter().zip(decks.iter()) {
            self.populate_player_library(player_id, deck)?;
        }
        self.reset_runtime_state();
        self.recompute_ui_decision()?;
        Ok(())
    }

    /// Advance to next phase (or next turn if ending phase).
    #[wasm_bindgen(js_name = advancePhase)]
    pub fn advance_phase(&mut self) -> Result<(), JsValue> {
        crate::turn::advance_phase(&mut self.game)
            .map_err(|e| JsValue::from_str(&format!("advance_phase failed: {e:?}")))?;
        self.recompute_ui_decision()?;
        Ok(())
    }

    /// Switch local perspective to the next player.
    #[wasm_bindgen(js_name = switchPerspective)]
    pub fn switch_perspective(&mut self) -> Result<u8, JsValue> {
        let current_index = self
            .game
            .players
            .iter()
            .position(|p| p.id == self.perspective)
            .unwrap_or(0);
        let next_index = (current_index + 1) % self.game.players.len().max(1);
        self.perspective = self.game.players[next_index].id;
        Ok(self.perspective.0)
    }

    /// Set local perspective explicitly.
    #[wasm_bindgen(js_name = setPerspective)]
    pub fn set_perspective(&mut self, player_index: u8) -> Result<(), JsValue> {
        let pid = PlayerId::from_index(player_index);
        if self.game.player(pid).is_none() {
            return Err(JsValue::from_str("invalid player index"));
        }
        self.perspective = pid;
        Ok(())
    }

    /// Apply a player command for the currently pending decision.
    #[wasm_bindgen]
    pub fn dispatch(&mut self, command: JsValue) -> Result<JsValue, JsValue> {
        let command: UiCommand = serde_wasm_bindgen::from_value(command)
            .map_err(|e| JsValue::from_str(&format!("invalid command payload: {e}")))?;

        let ctx = self
            .pending_decision
            .take()
            .ok_or_else(|| JsValue::from_str("no pending decision to dispatch"))?;
        let response = self.command_to_response(&ctx, command)?;

        let progress = apply_priority_response(
            &mut self.game,
            &mut self.trigger_queue,
            &mut self.priority_state,
            &response,
        )
        .map_err(|e| JsValue::from_str(&format!("dispatch failed: {e}")))?;

        self.apply_progress(progress)?;
        self.snapshot()
    }
}

impl WasmGame {
    fn populate_player_library(
        &mut self,
        player_id: PlayerId,
        deck_names: &[String],
    ) -> Result<(), JsValue> {
        for name in deck_names {
            let Some(def) = self.registry.get(name) else {
                return Err(JsValue::from_str(&format!("unknown card name: {name}")));
            };
            self.game
                .create_object_from_definition(def, player_id, crate::zone::Zone::Library);
        }

        if let Some(player) = self.game.player_mut(player_id) {
            player.shuffle_library();
        }
        Ok(())
    }

    fn reset_runtime_state(&mut self) {
        self.trigger_queue = TriggerQueue::new();
        self.priority_state = PriorityLoopState::new(self.game.players.len());
        self.pending_decision = None;
        self.game_over = None;
        if self.game.player(self.perspective).is_none()
            && let Some(first) = self.game.players.first()
        {
            self.perspective = first.id;
        }
    }

    fn recompute_ui_decision(&mut self) -> Result<(), JsValue> {
        self.pending_decision = None;
        if self.game_over.is_some() {
            return Ok(());
        }
        self.advance_until_decision()
    }

    fn advance_until_decision(&mut self) -> Result<(), JsValue> {
        for _ in 0..128 {
            let progress = advance_priority(&mut self.game, &mut self.trigger_queue)
                .map_err(|e| JsValue::from_str(&format!("advance_priority failed: {e}")))?;
            match progress {
                GameProgress::NeedsDecisionCtx(ctx) => {
                    self.pending_decision = Some(ctx);
                    return Ok(());
                }
                GameProgress::Continue => {
                    self.pending_decision = None;
                    return Ok(());
                }
                GameProgress::StackResolved => {
                    continue;
                }
                GameProgress::GameOver(result) => {
                    self.pending_decision = None;
                    self.game_over = Some(result);
                    return Ok(());
                }
            }
        }

        Err(JsValue::from_str(
            "advance loop exceeded iteration budget (possible infinite loop)",
        ))
    }

    fn apply_progress(&mut self, progress: GameProgress) -> Result<(), JsValue> {
        match progress {
            GameProgress::NeedsDecisionCtx(ctx) => {
                self.pending_decision = Some(ctx);
                Ok(())
            }
            GameProgress::Continue => {
                self.pending_decision = None;
                Ok(())
            }
            GameProgress::GameOver(result) => {
                self.pending_decision = None;
                self.game_over = Some(result);
                Ok(())
            }
            GameProgress::StackResolved => {
                self.pending_decision = None;
                self.advance_until_decision()
            }
        }
    }

    fn command_to_response(
        &self,
        ctx: &DecisionContext,
        command: UiCommand,
    ) -> Result<PriorityResponse, JsValue> {
        match (ctx, command) {
            (DecisionContext::Priority(priority), UiCommand::PriorityAction { action_index }) => {
                let action = priority.legal_actions.get(action_index).ok_or_else(|| {
                    JsValue::from_str(&format!("invalid priority action index: {action_index}"))
                })?;
                Ok(PriorityResponse::PriorityAction(action.clone()))
            }
            (DecisionContext::Number(number), UiCommand::NumberChoice { value }) => {
                if value < number.min || value > number.max {
                    return Err(JsValue::from_str(&format!(
                        "number out of range: expected {}..={}, got {}",
                        number.min, number.max, value
                    )));
                }
                if number.is_x_value {
                    Ok(PriorityResponse::XValue(value))
                } else {
                    Ok(PriorityResponse::NumberChoice(value))
                }
            }
            (
                DecisionContext::SelectOptions(options),
                UiCommand::SelectOptions { option_indices },
            ) => {
                let legal_indices: Vec<usize> = options
                    .options
                    .iter()
                    .filter(|o| o.legal)
                    .map(|o| o.index)
                    .collect();
                validate_option_selection(
                    options.min,
                    Some(options.max),
                    &option_indices,
                    &legal_indices,
                )?;
                self.map_select_options_response(option_indices)
            }
            (DecisionContext::Modes(modes), UiCommand::SelectOptions { option_indices }) => {
                let legal: Vec<usize> = modes
                    .spec
                    .modes
                    .iter()
                    .filter(|mode| mode.legal)
                    .map(|mode| mode.index)
                    .collect();
                validate_option_selection(
                    modes.spec.min_modes,
                    Some(modes.spec.max_modes),
                    &option_indices,
                    &legal,
                )?;
                Ok(PriorityResponse::Modes(option_indices))
            }
            (
                DecisionContext::HybridChoice(hybrid),
                UiCommand::SelectOptions { option_indices },
            ) => {
                let legal: Vec<usize> = hybrid.options.iter().map(|opt| opt.index).collect();
                validate_option_selection(1, Some(1), &option_indices, &legal)?;
                let choice = option_indices.first().copied().ok_or_else(|| {
                    JsValue::from_str("hybrid choice requires selecting one option")
                })?;
                Ok(PriorityResponse::HybridChoice(choice))
            }
            (DecisionContext::SelectObjects(objects), UiCommand::SelectObjects { object_ids }) => {
                let legal_ids: Vec<u64> = objects
                    .candidates
                    .iter()
                    .filter(|obj| obj.legal)
                    .map(|obj| obj.id.0)
                    .collect();
                validate_object_selection(objects.min, objects.max, &object_ids, &legal_ids)?;

                let chosen = object_ids.first().copied().ok_or_else(|| {
                    JsValue::from_str("select_objects requires one chosen object")
                })?;
                if self.priority_state.pending_activation.is_some() {
                    Ok(PriorityResponse::SacrificeTarget(ObjectId::from_raw(
                        chosen,
                    )))
                } else if self
                    .priority_state
                    .pending_cast
                    .as_ref()
                    .is_some_and(|pending| {
                        matches!(pending.stage, CastStage::ChoosingExileFromHand)
                    })
                {
                    Ok(PriorityResponse::CardToExile(ObjectId::from_raw(chosen)))
                } else {
                    Err(JsValue::from_str(
                        "unsupported SelectObjects context in priority flow",
                    ))
                }
            }
            (DecisionContext::Targets(_), UiCommand::SelectTargets { targets }) => {
                let converted: Vec<Target> = targets
                    .into_iter()
                    .map(|target| match target {
                        TargetInput::Player { player } => {
                            Target::Player(PlayerId::from_index(player))
                        }
                        TargetInput::Object { object } => {
                            Target::Object(ObjectId::from_raw(object))
                        }
                    })
                    .collect();
                Ok(PriorityResponse::Targets(converted))
            }
            (DecisionContext::Modes(_), UiCommand::NumberChoice { .. })
            | (DecisionContext::Modes(_), UiCommand::SelectObjects { .. })
            | (DecisionContext::Modes(_), UiCommand::SelectTargets { .. })
            | (DecisionContext::HybridChoice(_), UiCommand::PriorityAction { .. })
            | (DecisionContext::HybridChoice(_), UiCommand::NumberChoice { .. })
            | (DecisionContext::HybridChoice(_), UiCommand::SelectObjects { .. })
            | (DecisionContext::HybridChoice(_), UiCommand::SelectTargets { .. })
            | (DecisionContext::SelectOptions(_), UiCommand::PriorityAction { .. })
            | (DecisionContext::SelectOptions(_), UiCommand::NumberChoice { .. })
            | (DecisionContext::SelectOptions(_), UiCommand::SelectObjects { .. })
            | (DecisionContext::SelectOptions(_), UiCommand::SelectTargets { .. })
            | (DecisionContext::SelectObjects(_), UiCommand::PriorityAction { .. })
            | (DecisionContext::SelectObjects(_), UiCommand::NumberChoice { .. })
            | (DecisionContext::SelectObjects(_), UiCommand::SelectOptions { .. })
            | (DecisionContext::SelectObjects(_), UiCommand::SelectTargets { .. })
            | (DecisionContext::Targets(_), UiCommand::PriorityAction { .. })
            | (DecisionContext::Targets(_), UiCommand::NumberChoice { .. })
            | (DecisionContext::Targets(_), UiCommand::SelectObjects { .. })
            | (DecisionContext::Targets(_), UiCommand::SelectOptions { .. })
            | (DecisionContext::Number(_), UiCommand::PriorityAction { .. })
            | (DecisionContext::Number(_), UiCommand::SelectOptions { .. })
            | (DecisionContext::Number(_), UiCommand::SelectObjects { .. })
            | (DecisionContext::Number(_), UiCommand::SelectTargets { .. })
            | (DecisionContext::Priority(_), UiCommand::NumberChoice { .. })
            | (DecisionContext::Priority(_), UiCommand::SelectOptions { .. })
            | (DecisionContext::Priority(_), UiCommand::SelectObjects { .. })
            | (DecisionContext::Priority(_), UiCommand::SelectTargets { .. }) => Err(
                JsValue::from_str("command type does not match pending decision"),
            ),
            (_, _) => Err(JsValue::from_str(
                "pending decision type is not yet supported in WASM dispatch",
            )),
        }
    }

    fn map_select_options_response(
        &self,
        option_indices: Vec<usize>,
    ) -> Result<PriorityResponse, JsValue> {
        if self.game.pending_replacement_choice.is_some() {
            let choice = option_indices.first().copied().unwrap_or(0);
            return Ok(PriorityResponse::ReplacementChoice(choice));
        }
        if self.priority_state.pending_method_selection.is_some() {
            let choice = option_indices.first().copied().ok_or_else(|| {
                JsValue::from_str("casting method choice requires one selected option")
            })?;
            return Ok(PriorityResponse::CastingMethodChoice(choice));
        }
        if self
            .priority_state
            .pending_cast
            .as_ref()
            .is_some_and(|pending| matches!(pending.stage, CastStage::ChoosingOptionalCosts))
        {
            let choices: Vec<(usize, u32)> =
                option_indices.into_iter().map(|index| (index, 1)).collect();
            return Ok(PriorityResponse::OptionalCosts(choices));
        }
        if self.priority_state.pending_mana_ability.is_some() {
            let choice = option_indices
                .first()
                .copied()
                .ok_or_else(|| JsValue::from_str("mana payment choice requires one option"))?;
            return Ok(PriorityResponse::ManaPayment(choice));
        }
        if self
            .priority_state
            .pending_activation
            .as_ref()
            .is_some_and(|pending| matches!(pending.stage, ActivationStage::PayingMana))
            || self
                .priority_state
                .pending_cast
                .as_ref()
                .is_some_and(|pending| matches!(pending.stage, CastStage::PayingMana))
        {
            let choice = option_indices
                .first()
                .copied()
                .ok_or_else(|| JsValue::from_str("mana pip payment requires one option"))?;
            return Ok(PriorityResponse::ManaPipPayment(choice));
        }

        Err(JsValue::from_str(
            "unsupported SelectOptions context in priority flow",
        ))
    }
}

impl Default for WasmGame {
    fn default() -> Self {
        Self::new()
    }
}

fn describe_action(game: &GameState, action: &LegalAction) -> String {
    match action {
        LegalAction::PassPriority => "Pass priority".to_string(),
        LegalAction::PlayLand { land_id } => {
            format!("Play {}", object_name(game, *land_id))
        }
        LegalAction::CastSpell {
            spell_id,
            from_zone,
            casting_method,
        } => {
            let method = match casting_method {
                crate::alternative_cast::CastingMethod::Normal => "normal".to_string(),
                crate::alternative_cast::CastingMethod::Alternative(index) => {
                    format!("alternative #{index}")
                }
                crate::alternative_cast::CastingMethod::GrantedEscape { .. } => {
                    "granted escape".to_string()
                }
                crate::alternative_cast::CastingMethod::GrantedFlashback => {
                    "granted flashback".to_string()
                }
                crate::alternative_cast::CastingMethod::PlayFrom { zone, .. } => {
                    format!("play from {:?}", zone)
                }
            };
            format!(
                "Cast {} ({:?}, {})",
                object_name(game, *spell_id),
                from_zone,
                method
            )
        }
        LegalAction::ActivateAbility {
            source,
            ability_index,
        } => {
            format!(
                "Activate {} ability #{}",
                object_name(game, *source),
                ability_index + 1
            )
        }
        LegalAction::ActivateManaAbility {
            source,
            ability_index,
        } => {
            format!(
                "Activate mana ability on {} (# {})",
                object_name(game, *source),
                ability_index + 1
            )
        }
        LegalAction::TurnFaceUp { creature_id } => {
            format!("Turn face up {}", object_name(game, *creature_id))
        }
        LegalAction::SpecialAction(action) => format!("Special action: {:?}", action),
    }
}

fn object_name(game: &GameState, id: ObjectId) -> String {
    game.object(id)
        .map(|o| o.name.clone())
        .unwrap_or_else(|| format!("Object#{}", id.0))
}

fn target_choice_view(game: &GameState, target: &Target) -> TargetChoiceView {
    match target {
        Target::Player(pid) => TargetChoiceView::Player {
            player: pid.0,
            name: game
                .player(*pid)
                .map(|p| p.name.clone())
                .unwrap_or_else(|| format!("Player {}", pid.0 + 1)),
        },
        Target::Object(id) => TargetChoiceView::Object {
            object: id.0,
            name: object_name(game, *id),
        },
    }
}

fn validate_option_selection(
    min: usize,
    max: Option<usize>,
    selected: &[usize],
    legal_indices: &[usize],
) -> Result<(), JsValue> {
    if selected.len() < min {
        return Err(JsValue::from_str(&format!(
            "must select at least {min} option(s)"
        )));
    }
    if let Some(max) = max
        && selected.len() > max
    {
        return Err(JsValue::from_str(&format!(
            "must select at most {max} option(s)"
        )));
    }
    for selected_index in selected {
        if !legal_indices.contains(selected_index) {
            return Err(JsValue::from_str(&format!(
                "option index {selected_index} is not legal"
            )));
        }
    }
    Ok(())
}

fn validate_object_selection(
    min: usize,
    max: Option<usize>,
    selected: &[u64],
    legal_ids: &[u64],
) -> Result<(), JsValue> {
    if selected.len() < min {
        return Err(JsValue::from_str(&format!(
            "must select at least {min} object(s)"
        )));
    }
    if let Some(max) = max
        && selected.len() > max
    {
        return Err(JsValue::from_str(&format!(
            "must select at most {max} object(s)"
        )));
    }
    for object_id in selected {
        if !legal_ids.contains(object_id) {
            return Err(JsValue::from_str(&format!(
                "object id {object_id} is not legal"
            )));
        }
    }
    Ok(())
}
