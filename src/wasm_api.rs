//! WASM-facing API for browser integration.
//!
//! This module provides a small wrapper around `GameState` so JavaScript can:
//! - create/reset a game
//! - mutate a bit of state
//! - read a serializable snapshot

use std::collections::{HashMap, HashSet};

use rand::seq::SliceRandom;
use rand::{SeedableRng, rngs::StdRng};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

use crate::ability::{Ability, AbilityKind};
use crate::cards::{CardDefinition, CardRegistry};
use crate::combat_state::AttackTarget;
use crate::decision::{
    AttackerDeclaration, BlockerDeclaration, GameProgress, GameResult, LegalAction,
    SelectFirstDecisionMaker,
};
use crate::decisions::context::DecisionContext;
use crate::game_loop::{
    ActivationStage, CastStage, PriorityLoopState, PriorityResponse, add_saga_lore_counters,
    advance_priority, apply_priority_response_with_dm, generate_and_queue_step_triggers,
    get_declare_attackers_decision, get_declare_blockers_decision,
};
use crate::game_state::{GameState, Phase, Step, Target};
use crate::ids::{ObjectId, PlayerId};
use crate::mana::ManaSymbol;
use crate::target::ObjectFilter;
use crate::triggers::{TriggerQueue, check_triggers};
use crate::zone::Zone;

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
    hand_cards: Vec<HandCardSnapshot>,
    graveyard_cards: Vec<ZoneCardSnapshot>,
    exile_cards: Vec<ZoneCardSnapshot>,
    library_top: Option<String>,
    graveyard_top: Option<String>,
    battlefield: Vec<PermanentSnapshot>,
}

#[derive(Debug, Clone, Serialize)]
struct HandCardSnapshot {
    id: u64,
    name: String,
}

#[derive(Debug, Clone, Serialize)]
struct ZoneCardSnapshot {
    id: u64,
    name: String,
}

#[derive(Debug, Clone, Serialize)]
struct CounterSnapshot {
    kind: String,
    amount: u32,
}

#[derive(Debug, Clone, Serialize)]
struct ObjectDetailsSnapshot {
    id: u64,
    stable_id: u64,
    name: String,
    kind: String,
    zone: String,
    owner: u8,
    controller: u8,
    type_line: String,
    mana_cost: Option<String>,
    oracle_text: String,
    power: Option<i32>,
    toughness: Option<i32>,
    loyalty: Option<u32>,
    tapped: bool,
    counters: Vec<CounterSnapshot>,
    abilities: Vec<String>,
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
        pending_cast_stack_id: Option<ObjectId>,
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
                        .map(|o| HandCardSnapshot {
                            id: o.id.0,
                            name: o.name.clone(),
                        })
                        .collect()
                } else {
                    Vec::new()
                },
                graveyard_cards: p
                    .graveyard
                    .iter()
                    .rev()
                    .filter_map(|id| game.object(*id))
                    .map(|o| ZoneCardSnapshot {
                        id: o.id.0,
                        name: o.name.clone(),
                    })
                    .collect(),
                exile_cards: game
                    .exile
                    .iter()
                    .rev()
                    .filter_map(|id| game.object(*id))
                    .filter(|o| o.owner == p.id)
                    .map(|o| ZoneCardSnapshot {
                        id: o.id.0,
                        name: o.name.clone(),
                    })
                    .collect(),
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

        let mut stack_preview: Vec<String> = game
            .stack
            .iter()
            .rev()
            .map(|entry| {
                game.object(entry.object_id)
                    .map(|obj| obj.name.clone())
                    .unwrap_or_else(|| format!("Object#{}", entry.object_id.0))
            })
            .collect();
        let mut stack_size = game.stack.len();

        // During casting (rule 601.2), the card can be moved to stack before finalization.
        // Surface that pending spell in UI so it doesn't look like it vanished.
        if let Some(stack_id) = pending_cast_stack_id
            && !game.stack.iter().any(|entry| entry.object_id == stack_id)
            && let Some(obj) = game.object(stack_id)
        {
            stack_preview.insert(0, obj.name.clone());
            stack_size += 1;
        }
        stack_preview.truncate(4);

        Self {
            perspective: perspective.0,
            turn_number: game.turn.turn_number,
            active_player: game.turn.active_player.0,
            priority_player: game.turn.priority_player.map(|p| p.0),
            phase: format!("{:?}", game.turn.phase),
            step: game.turn.step.map(|step| format!("{:?}", step)),
            stack_size,
            stack_preview,
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
    kind: String,
    object_id: Option<u64>,
    from_zone: Option<String>,
    to_zone: Option<String>,
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
enum AttackTargetView {
    Player { player: u8, name: String },
    Planeswalker { object: u64, name: String },
}

#[derive(Debug, Clone, Serialize)]
struct AttackerOptionView {
    creature: u64,
    creature_name: String,
    valid_targets: Vec<AttackTargetView>,
    must_attack: bool,
}

#[derive(Debug, Clone, Serialize)]
struct BlockerChoiceView {
    id: u64,
    name: String,
}

#[derive(Debug, Clone, Serialize)]
struct BlockerOptionView {
    attacker: u64,
    attacker_name: String,
    valid_blockers: Vec<BlockerChoiceView>,
    min_blockers: usize,
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
    Attackers {
        player: u8,
        attacker_options: Vec<AttackerOptionView>,
    },
    Blockers {
        player: u8,
        blocker_options: Vec<BlockerOptionView>,
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
                    .map(|(index, action)| build_action_view(game, index, action))
                    .collect(),
                commander_actions: priority
                    .commander_actions
                    .iter()
                    .enumerate()
                    .map(|(index, action)| build_action_view(game, index, action))
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
            DecisionContext::Attackers(attackers) => DecisionView::Attackers {
                player: attackers.player.0,
                attacker_options: attackers
                    .attacker_options
                    .iter()
                    .map(|option| AttackerOptionView {
                        creature: option.creature.0,
                        creature_name: option.creature_name.clone(),
                        valid_targets: option
                            .valid_targets
                            .iter()
                            .map(|target| attack_target_view(game, target))
                            .collect(),
                        must_attack: option.must_attack,
                    })
                    .collect(),
            },
            DecisionContext::Blockers(blockers) => DecisionView::Blockers {
                player: blockers.player.0,
                blocker_options: blockers
                    .blocker_options
                    .iter()
                    .map(|option| BlockerOptionView {
                        attacker: option.attacker.0,
                        attacker_name: option.attacker_name.clone(),
                        valid_blockers: option
                            .valid_blockers
                            .iter()
                            .map(|(id, name)| BlockerChoiceView {
                                id: id.0,
                                name: name.clone(),
                            })
                            .collect(),
                        min_blockers: option.min_blockers,
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
    PriorityAction {
        action_index: usize,
    },
    NumberChoice {
        value: u32,
    },
    SelectOptions {
        option_indices: Vec<usize>,
    },
    SelectObjects {
        object_ids: Vec<u64>,
    },
    SelectTargets {
        targets: Vec<TargetInput>,
    },
    DeclareAttackers {
        declarations: Vec<AttackerDeclarationInput>,
    },
    DeclareBlockers {
        declarations: Vec<BlockerDeclarationInput>,
    },
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum TargetInput {
    Player { player: u8 },
    Object { object: u64 },
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum AttackTargetInput {
    Player { player: u8 },
    Planeswalker { object: u64 },
}

#[derive(Debug, Clone, Deserialize)]
struct AttackerDeclarationInput {
    creature: u64,
    target: AttackTargetInput,
}

#[derive(Debug, Clone, Deserialize)]
struct BlockerDeclarationInput {
    blocker: u64,
    blocking: u64,
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
    attackers_declared_this_step: bool,
    blockers_declared_this_step: bool,
    deck_generation_nonce: u64,
    last_step_actions_applied: Option<(u32, Phase, Option<Step>)>,
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
        let mut priority_state = PriorityLoopState::new(2);
        priority_state.set_auto_choose_single_pip_payment(false);
        Self {
            game: GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20),
            registry: CardRegistry::with_builtin_cards(),
            trigger_queue: TriggerQueue::new(),
            priority_state,
            pending_decision: None,
            game_over: None,
            perspective: PlayerId::from_index(0),
            attackers_declared_this_step: false,
            blockers_declared_this_step: false,
            deck_generation_nonce: 0,
            last_step_actions_applied: None,
        }
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
        let pending_cast_stack_id = self
            .priority_state
            .pending_cast
            .as_ref()
            .map(|p| p.stack_id);
        let snap = GameSnapshot::from_game(
            &self.game,
            self.perspective,
            self.pending_decision.as_ref(),
            self.game_over.as_ref(),
            pending_cast_stack_id,
        );
        serde_wasm_bindgen::to_value(&snap)
            .map_err(|e| JsValue::from_str(&format!("snapshot encode failed: {e}")))
    }

    /// Return the current UI state from the selected player perspective.
    #[wasm_bindgen(js_name = uiState)]
    pub fn ui_state(&self) -> Result<JsValue, JsValue> {
        self.snapshot()
    }

    /// Number of cards currently available in the registry.
    #[wasm_bindgen(js_name = registrySize)]
    pub fn registry_size(&self) -> usize {
        self.registry.len()
    }

    /// Return a detailed, human-readable object snapshot for inspector UI.
    #[wasm_bindgen(js_name = objectDetails)]
    pub fn object_details(&self, object_id: u64) -> Result<JsValue, JsValue> {
        let details = build_object_details_snapshot(&self.game, ObjectId::from_raw(object_id))
            .ok_or_else(|| JsValue::from_str(&format!("unknown object id: {object_id}")))?;
        serde_wasm_bindgen::to_value(&details)
            .map_err(|e| JsValue::from_str(&format!("objectDetails encode failed: {e}")))
    }

    /// Return game snapshot as pretty JSON.
    #[wasm_bindgen(js_name = snapshotJson)]
    pub fn snapshot_json(&self) -> Result<String, JsValue> {
        let pending_cast_stack_id = self
            .priority_state
            .pending_cast
            .as_ref()
            .map(|p| p.stack_id);
        let snap = GameSnapshot::from_game(
            &self.game,
            self.perspective,
            self.pending_decision.as_ref(),
            self.game_over.as_ref(),
            pending_cast_stack_id,
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

    /// Add a specific card by name to a player's hand.
    #[wasm_bindgen(js_name = addCardToHand)]
    pub fn add_card_to_hand(
        &mut self,
        player_index: u8,
        card_name: String,
    ) -> Result<u64, JsValue> {
        let player_id = PlayerId::from_index(player_index);
        if self.game.player(player_id).is_none() {
            return Err(JsValue::from_str("invalid player index"));
        }

        let query = card_name.trim();
        if query.is_empty() {
            return Err(JsValue::from_str("card name cannot be empty"));
        }

        let definition = self
            .registry
            .get(query)
            .cloned()
            .or_else(|| {
                self.registry
                    .all()
                    .find(|def| def.name().eq_ignore_ascii_case(query))
                    .cloned()
            })
            .ok_or_else(|| JsValue::from_str(&format!("unknown card name: {query}")))?;

        let object_id = self.game.create_object_from_definition(
            &definition,
            player_id,
            crate::zone::Zone::Hand,
        );
        self.recompute_ui_decision()?;
        Ok(object_id.0)
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

        for player_id in player_ids {
            // 60-card random deck with 24-land color-aware basic manabase.
            let deck = self.build_random_demo_deck_names(60, 24)?;
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
        self.advance_step_with_actions()?;
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
        let response = match self.command_to_response(&ctx, command) {
            Ok(response) => response,
            Err(err) => {
                // Validation/shape errors should not consume the current decision.
                self.pending_decision = Some(ctx);
                return Err(err);
            }
        };
        let ctx_for_restore = ctx.clone();
        let prev_attackers_declared = self.attackers_declared_this_step;
        let prev_blockers_declared = self.blockers_declared_this_step;
        match &response {
            PriorityResponse::Attackers(_) => {
                self.attackers_declared_this_step = true;
            }
            PriorityResponse::Blockers { .. } => {
                self.blockers_declared_this_step = true;
            }
            _ => {}
        }

        // Always route through the DM-aware path so nested decisions during
        // resolution (ETB choices, mana-color picks, etc.) never fall back to CLI stdin.
        let mut wasm_dm = SelectFirstDecisionMaker;
        let progress = match apply_priority_response_with_dm(
            &mut self.game,
            &mut self.trigger_queue,
            &mut self.priority_state,
            &response,
            &mut wasm_dm,
        ) {
            Ok(progress) => progress,
            Err(e) => {
                // Mirror CLI rollback behavior for partially-executed action chains.
                if let Some(checkpoint) = self.priority_state.checkpoint.take() {
                    self.game = checkpoint;
                    self.priority_state.pending_cast = None;
                    self.priority_state.pending_activation = None;
                    self.priority_state.pending_method_selection = None;
                    self.priority_state.pending_mana_ability = None;
                    self.pending_decision = None;
                    self.game_over = None;
                    let _ = self.recompute_ui_decision();
                } else {
                    // No action-chain checkpoint means this was a pure validation failure;
                    // keep the current decision visible in UI.
                    self.pending_decision = Some(ctx_for_restore);
                }
                self.attackers_declared_this_step = prev_attackers_declared;
                self.blockers_declared_this_step = prev_blockers_declared;
                return Err(JsValue::from_str(&format!("dispatch failed: {e}")));
            }
        };

        self.apply_progress(progress)?;
        self.snapshot()
    }
}

impl WasmGame {
    fn has_demo_supported_cost_symbols(cost: &crate::mana::ManaCost) -> bool {
        !cost.pips().iter().flatten().any(|symbol| {
            matches!(
                symbol,
                ManaSymbol::Colorless | ManaSymbol::Snow | ManaSymbol::Life(_) | ManaSymbol::X
            )
        })
    }

    fn is_strict_demo_spell_candidate(def: &CardDefinition) -> bool {
        if def.card.is_token || def.card.is_land() {
            return false;
        }
        // Keep startup-safe random decks to cards the current UI/decision loop
        // can consistently represent without hitting unsupported cast branches.
        if !def.alternative_casts.is_empty()
            || !def.optional_costs.is_empty()
            || !def.cost_effects.is_empty()
            || def.max_saga_chapter.is_some()
            || def.name().contains("//")
        {
            return false;
        }
        let Some(cost) = &def.card.mana_cost else {
            return false;
        };
        Self::has_demo_supported_cost_symbols(cost)
    }

    fn is_fallback_demo_spell_candidate(def: &CardDefinition) -> bool {
        if def.card.is_token || def.card.is_land() {
            return false;
        }
        match &def.card.mana_cost {
            Some(cost) => Self::has_demo_supported_cost_symbols(cost),
            None => true,
        }
    }

    fn build_random_demo_deck_names(
        &mut self,
        deck_size: usize,
        land_count: usize,
    ) -> Result<Vec<String>, JsValue> {
        if deck_size == 0 || land_count >= deck_size {
            return Err(JsValue::from_str(
                "invalid deck sizing (deck_size must be > 0 and land_count < deck_size)",
            ));
        }

        let spells_needed = deck_size - land_count;
        let mut rng = StdRng::seed_from_u64(self.next_deck_seed());

        let mut strict_spell_pool: Vec<_> = self
            .registry
            .all()
            .filter(|def| Self::is_strict_demo_spell_candidate(def))
            .collect();

        let mut spell_pool: Vec<_> = if strict_spell_pool.is_empty() {
            self.registry
                .all()
                .filter(|def| Self::is_fallback_demo_spell_candidate(def))
                .collect()
        } else {
            std::mem::take(&mut strict_spell_pool)
        };

        if spell_pool.is_empty() {
            return Err(JsValue::from_str(
                "registry has no nonland cards eligible for random deck generation",
            ));
        }

        spell_pool.shuffle(&mut rng);

        let mut spells: Vec<String> = Vec::with_capacity(spells_needed);
        if spell_pool.len() >= spells_needed {
            spells.extend(
                spell_pool
                    .iter()
                    .take(spells_needed)
                    .map(|def| def.name().to_string()),
            );
        } else {
            // If pool is smaller than requested spells, wrap and keep shuffling for variety.
            while spells.len() < spells_needed {
                spell_pool.shuffle(&mut rng);
                for def in &spell_pool {
                    spells.push(def.name().to_string());
                    if spells.len() >= spells_needed {
                        break;
                    }
                }
            }
        }

        let mut symbol_counts: HashMap<ManaSymbol, u32> = HashMap::new();
        for card_name in &spells {
            if let Some(def) = self.registry.get(card_name)
                && let Some(cost) = &def.card.mana_cost
            {
                for pip in cost.pips() {
                    for symbol in pip {
                        match symbol {
                            ManaSymbol::White
                            | ManaSymbol::Blue
                            | ManaSymbol::Black
                            | ManaSymbol::Red
                            | ManaSymbol::Green => {
                                *symbol_counts.entry(*symbol).or_insert(0) += 1;
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        let mut deck = spells;
        let total_colored_symbols: u32 = symbol_counts.values().sum();
        let color_order = [
            ManaSymbol::White,
            ManaSymbol::Blue,
            ManaSymbol::Black,
            ManaSymbol::Red,
            ManaSymbol::Green,
        ];

        let mut assigned_lands = 0usize;
        for color in color_order {
            let count = symbol_counts.get(&color).copied().unwrap_or(0);
            if count == 0 || total_colored_symbols == 0 {
                continue;
            }
            let share = (count as f64 / total_colored_symbols as f64) * land_count as f64;
            let land_slots = share.round() as usize;
            let basic_name = Self::basic_land_name_for_symbol(color);
            if self.registry.get(basic_name).is_none() {
                continue;
            }
            for _ in 0..land_slots {
                deck.push(basic_name.to_string());
                assigned_lands += 1;
                if assigned_lands >= land_count {
                    break;
                }
            }
            if assigned_lands >= land_count {
                break;
            }
        }

        let fallback_color = symbol_counts
            .iter()
            .max_by_key(|(_, count)| *count)
            .map(|(color, _)| *color)
            .unwrap_or(ManaSymbol::Green);
        let mut fallback_land = Self::basic_land_name_for_symbol(fallback_color);
        if self.registry.get(fallback_land).is_none() {
            fallback_land = ["Plains", "Island", "Swamp", "Mountain", "Forest"]
                .into_iter()
                .find(|name| self.registry.get(name).is_some())
                .ok_or_else(|| {
                    JsValue::from_str("registry has no basic lands for demo manabase")
                })?;
        }
        while assigned_lands < land_count {
            deck.push(fallback_land.to_string());
            assigned_lands += 1;
        }

        deck.shuffle(&mut rng);
        Ok(deck)
    }

    fn next_deck_seed(&mut self) -> u64 {
        self.deck_generation_nonce = self.deck_generation_nonce.wrapping_add(1);
        let random_bits = (js_sys::Math::random() * (u64::MAX as f64)) as u64;
        random_bits ^ self.deck_generation_nonce.rotate_left(17)
    }

    fn basic_land_name_for_symbol(symbol: ManaSymbol) -> &'static str {
        match symbol {
            ManaSymbol::White => "Plains",
            ManaSymbol::Blue => "Island",
            ManaSymbol::Black => "Swamp",
            ManaSymbol::Red => "Mountain",
            ManaSymbol::Green => "Forest",
            _ => "Forest",
        }
    }

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
        self.priority_state
            .set_auto_choose_single_pip_payment(false);
        self.pending_decision = None;
        self.game_over = None;
        self.attackers_declared_this_step = false;
        self.blockers_declared_this_step = false;
        self.last_step_actions_applied = None;
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
        for _ in 0..192 {
            self.apply_current_step_actions_once();
            self.sync_combat_step_flags();

            if let Some(combat_ctx) = self.pending_combat_declare_decision()? {
                self.pending_decision = Some(combat_ctx);
                return Ok(());
            }

            let progress = advance_priority(&mut self.game, &mut self.trigger_queue)
                .map_err(|e| JsValue::from_str(&format!("advance_priority failed: {e}")))?;
            match progress {
                GameProgress::NeedsDecisionCtx(ctx) => {
                    self.pending_decision = Some(ctx);
                    return Ok(());
                }
                GameProgress::Continue => {
                    self.advance_step_with_actions()?;
                    self.pending_decision = None;
                    continue;
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
                self.advance_step_with_actions()?;
                self.pending_decision = None;
                self.advance_until_decision()
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

    fn sync_combat_step_flags(&mut self) {
        if self.game.turn.step != Some(Step::DeclareAttackers) {
            self.attackers_declared_this_step = false;
        }
        if self.game.turn.step != Some(Step::DeclareBlockers) {
            self.blockers_declared_this_step = false;
        }
    }

    fn pending_combat_declare_decision(&mut self) -> Result<Option<DecisionContext>, JsValue> {
        if self.game.turn.step == Some(Step::DeclareAttackers) && !self.attackers_declared_this_step
        {
            if self.game.combat.is_none() {
                self.game.combat = Some(Default::default());
            }
            let combat = self.game.combat.clone().unwrap_or_default();
            return Ok(Some(get_declare_attackers_decision(&self.game, &combat)));
        }

        if self.game.turn.step == Some(Step::DeclareBlockers) && !self.blockers_declared_this_step {
            let combat = self.game.combat.clone().unwrap_or_default();
            if combat.attackers.is_empty() {
                self.blockers_declared_this_step = true;
                self.advance_step_with_actions()?;
                return Ok(None);
            }
            let defending_player = self.blocking_player_for_step(&combat);
            return Ok(Some(get_declare_blockers_decision(
                &self.game,
                &combat,
                defending_player,
            )));
        }

        Ok(None)
    }

    fn advance_step_with_actions(&mut self) -> Result<(), JsValue> {
        crate::turn::advance_step(&mut self.game)
            .map_err(|e| JsValue::from_str(&format!("advance_step failed: {e:?}")))?;
        self.apply_current_step_actions_once();
        Ok(())
    }

    fn apply_current_step_actions_once(&mut self) {
        let marker = (
            self.game.turn.turn_number,
            self.game.turn.phase,
            self.game.turn.step,
        );
        if self.last_step_actions_applied == Some(marker) {
            return;
        }

        match (self.game.turn.phase, self.game.turn.step) {
            (Phase::Beginning, Some(Step::Untap)) => {
                crate::turn::execute_untap_step(&mut self.game);
            }
            (Phase::Beginning, Some(Step::Upkeep)) => {
                generate_and_queue_step_triggers(&mut self.game, &mut self.trigger_queue);
            }
            (Phase::Beginning, Some(Step::Draw)) => {
                let draw_events = crate::turn::execute_draw_step(&mut self.game);
                generate_and_queue_step_triggers(&mut self.game, &mut self.trigger_queue);
                for draw_event in draw_events {
                    for entry in check_triggers(&self.game, &draw_event) {
                        self.trigger_queue.add(entry);
                    }
                }
            }
            (Phase::FirstMain, None) => {
                generate_and_queue_step_triggers(&mut self.game, &mut self.trigger_queue);
                add_saga_lore_counters(&mut self.game, &mut self.trigger_queue);
            }
            (Phase::Combat, Some(Step::BeginCombat))
            | (Phase::Combat, Some(Step::EndCombat))
            | (Phase::NextMain, None)
            | (Phase::Ending, Some(Step::End)) => {
                generate_and_queue_step_triggers(&mut self.game, &mut self.trigger_queue);
            }
            (Phase::Ending, Some(Step::Cleanup)) => {
                crate::turn::execute_cleanup_step(&mut self.game);
            }
            _ => {}
        }

        self.last_step_actions_applied = Some(marker);
    }

    fn blocking_player_for_step(&self, combat: &crate::combat_state::CombatState) -> PlayerId {
        if let Some(priority_player) = self.game.turn.priority_player
            && priority_player != self.game.turn.active_player
        {
            return priority_player;
        }
        if let Some(attacker) = combat.attackers.first()
            && let AttackTarget::Player(player) = attacker.target
        {
            return player;
        }
        self.game
            .players
            .iter()
            .find(|p| p.id != self.game.turn.active_player && p.is_in_game())
            .map(|p| p.id)
            .unwrap_or(self.game.turn.active_player)
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
            (
                DecisionContext::Attackers(attackers),
                UiCommand::DeclareAttackers { declarations },
            ) => {
                let converted = validate_attacker_declarations(attackers, &declarations)?;
                Ok(PriorityResponse::Attackers(converted))
            }
            (DecisionContext::Blockers(blockers), UiCommand::DeclareBlockers { declarations }) => {
                let converted = validate_blocker_declarations(blockers, &declarations)?;
                Ok(PriorityResponse::Blockers {
                    defending_player: blockers.player,
                    declarations: converted,
                })
            }
            (DecisionContext::Modes(_), UiCommand::NumberChoice { .. })
            | (DecisionContext::Modes(_), UiCommand::SelectObjects { .. })
            | (DecisionContext::Modes(_), UiCommand::SelectTargets { .. })
            | (DecisionContext::Modes(_), UiCommand::DeclareAttackers { .. })
            | (DecisionContext::Modes(_), UiCommand::DeclareBlockers { .. })
            | (DecisionContext::HybridChoice(_), UiCommand::PriorityAction { .. })
            | (DecisionContext::HybridChoice(_), UiCommand::NumberChoice { .. })
            | (DecisionContext::HybridChoice(_), UiCommand::SelectObjects { .. })
            | (DecisionContext::HybridChoice(_), UiCommand::SelectTargets { .. })
            | (DecisionContext::HybridChoice(_), UiCommand::DeclareAttackers { .. })
            | (DecisionContext::HybridChoice(_), UiCommand::DeclareBlockers { .. })
            | (DecisionContext::SelectOptions(_), UiCommand::PriorityAction { .. })
            | (DecisionContext::SelectOptions(_), UiCommand::NumberChoice { .. })
            | (DecisionContext::SelectOptions(_), UiCommand::SelectObjects { .. })
            | (DecisionContext::SelectOptions(_), UiCommand::SelectTargets { .. })
            | (DecisionContext::SelectOptions(_), UiCommand::DeclareAttackers { .. })
            | (DecisionContext::SelectOptions(_), UiCommand::DeclareBlockers { .. })
            | (DecisionContext::SelectObjects(_), UiCommand::PriorityAction { .. })
            | (DecisionContext::SelectObjects(_), UiCommand::NumberChoice { .. })
            | (DecisionContext::SelectObjects(_), UiCommand::SelectOptions { .. })
            | (DecisionContext::SelectObjects(_), UiCommand::SelectTargets { .. })
            | (DecisionContext::SelectObjects(_), UiCommand::DeclareAttackers { .. })
            | (DecisionContext::SelectObjects(_), UiCommand::DeclareBlockers { .. })
            | (DecisionContext::Targets(_), UiCommand::PriorityAction { .. })
            | (DecisionContext::Targets(_), UiCommand::NumberChoice { .. })
            | (DecisionContext::Targets(_), UiCommand::SelectObjects { .. })
            | (DecisionContext::Targets(_), UiCommand::SelectOptions { .. })
            | (DecisionContext::Targets(_), UiCommand::DeclareAttackers { .. })
            | (DecisionContext::Targets(_), UiCommand::DeclareBlockers { .. })
            | (DecisionContext::Number(_), UiCommand::PriorityAction { .. })
            | (DecisionContext::Number(_), UiCommand::SelectOptions { .. })
            | (DecisionContext::Number(_), UiCommand::SelectObjects { .. })
            | (DecisionContext::Number(_), UiCommand::SelectTargets { .. })
            | (DecisionContext::Number(_), UiCommand::DeclareAttackers { .. })
            | (DecisionContext::Number(_), UiCommand::DeclareBlockers { .. })
            | (DecisionContext::Priority(_), UiCommand::NumberChoice { .. })
            | (DecisionContext::Priority(_), UiCommand::SelectOptions { .. })
            | (DecisionContext::Priority(_), UiCommand::SelectObjects { .. })
            | (DecisionContext::Priority(_), UiCommand::SelectTargets { .. })
            | (DecisionContext::Priority(_), UiCommand::DeclareAttackers { .. })
            | (DecisionContext::Priority(_), UiCommand::DeclareBlockers { .. })
            | (DecisionContext::Attackers(_), UiCommand::PriorityAction { .. })
            | (DecisionContext::Attackers(_), UiCommand::NumberChoice { .. })
            | (DecisionContext::Attackers(_), UiCommand::SelectOptions { .. })
            | (DecisionContext::Attackers(_), UiCommand::SelectObjects { .. })
            | (DecisionContext::Attackers(_), UiCommand::SelectTargets { .. })
            | (DecisionContext::Attackers(_), UiCommand::DeclareBlockers { .. })
            | (DecisionContext::Blockers(_), UiCommand::PriorityAction { .. })
            | (DecisionContext::Blockers(_), UiCommand::NumberChoice { .. })
            | (DecisionContext::Blockers(_), UiCommand::SelectOptions { .. })
            | (DecisionContext::Blockers(_), UiCommand::SelectObjects { .. })
            | (DecisionContext::Blockers(_), UiCommand::SelectTargets { .. })
            | (DecisionContext::Blockers(_), UiCommand::DeclareAttackers { .. }) => Err(
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

fn build_object_details_snapshot(game: &GameState, id: ObjectId) -> Option<ObjectDetailsSnapshot> {
    let obj = game.object(id)?;
    let counters = obj
        .counters
        .iter()
        .map(|(kind, amount)| CounterSnapshot {
            kind: split_camel_case(&format!("{kind:?}")),
            amount: *amount,
        })
        .collect();

    Some(ObjectDetailsSnapshot {
        id: obj.id.0,
        stable_id: obj.stable_id.0.0,
        name: obj.name.clone(),
        kind: format!("{:?}", obj.kind),
        zone: zone_name(obj.zone),
        owner: obj.owner.0,
        controller: obj.controller.0,
        type_line: format_type_line(obj),
        mana_cost: obj.mana_cost.as_ref().map(|cost| cost.to_oracle()),
        oracle_text: obj.oracle_text.clone(),
        power: obj.power(),
        toughness: obj.toughness(),
        loyalty: obj.loyalty(),
        tapped: game.is_tapped(obj.id),
        counters,
        abilities: collect_object_abilities(obj),
    })
}

fn collect_object_abilities(obj: &crate::object::Object) -> Vec<String> {
    let mut abilities: Vec<String> = Vec::new();
    let has_attach_only_spell_effect = obj.spell_effect.as_ref().is_some_and(|effects| {
        effects.len() == 1
            && effects[0]
                .downcast_ref::<crate::effects::AttachToEffect>()
                .is_some()
    });

    for (idx, ability) in obj.abilities.iter().enumerate() {
        abilities.extend(describe_compiled_ability(idx + 1, ability));
    }

    if let Some(filter) = &obj.aura_attach_filter {
        abilities.push(format!("Enchant {}", describe_enchant_filter(filter)));
    }

    if !obj.cost_effects.is_empty() {
        abilities.extend(describe_effect_list(
            "As an additional cost to cast this spell",
            &obj.cost_effects,
        ));
    }
    if let Some(effects) = &obj.spell_effect {
        if !(obj.aura_attach_filter.is_some() && has_attach_only_spell_effect) {
            abilities.extend(describe_effect_list("Spell effect", effects));
        }
    }

    for granted in obj.level_granted_abilities() {
        abilities.push(format!("Level bonus: {}", granted.display()));
    }

    abilities
        .into_iter()
        .map(|line| line.trim().to_string())
        .filter(|line| !line.is_empty())
        .collect()
}

fn describe_enchant_filter(filter: &ObjectFilter) -> String {
    let desc = filter.description();
    if let Some(stripped) = desc.strip_prefix("a ") {
        stripped.to_string()
    } else if let Some(stripped) = desc.strip_prefix("an ") {
        stripped.to_string()
    } else {
        desc
    }
}

fn describe_keyword_ability(ability: &Ability) -> Option<String> {
    let raw_text = ability.text.as_deref()?.trim();
    let text = raw_text.to_ascii_lowercase();
    let words = text.split_whitespace().collect::<Vec<_>>();
    if words.first().copied() == Some("equip") {
        return Some("Equip".to_string());
    }
    if words.len() >= 2 && words[0] == "level" && words[1] == "up" {
        return Some("Level up".to_string());
    }
    let cycling_words = words
        .iter()
        .copied()
        .filter(|word| word.ends_with("cycling"))
        .collect::<Vec<_>>();
    if !cycling_words.is_empty() {
        let rendered = cycling_words
            .into_iter()
            .map(|word| {
                let mut chars = word.chars();
                match chars.next() {
                    Some(first) => {
                        format!("{}{}", first.to_ascii_uppercase(), chars.as_str())
                    }
                    None => "Cycling".to_string(),
                }
            })
            .collect::<Vec<_>>();
        return Some(rendered.join(", "));
    }
    if text == "prowess" {
        return Some("Prowess".to_string());
    }
    if text == "exalted" {
        return Some("Exalted".to_string());
    }
    if text == "persist" {
        return Some("Persist".to_string());
    }
    if text == "undying" {
        return Some("Undying".to_string());
    }
    if text.starts_with("bushido ") {
        return Some(raw_text.to_string());
    }
    None
}

fn describe_compiled_ability(index: usize, ability: &Ability) -> Vec<String> {
    if let Some(keyword) = describe_keyword_ability(ability) {
        return vec![format!("Keyword ability {index}: {keyword}")];
    }
    match &ability.kind {
        AbilityKind::Static(static_ability) => {
            vec![format!(
                "Static ability {index}: {}",
                static_ability.display()
            )]
        }
        AbilityKind::Triggered(triggered) => describe_triggered_compiled(index, triggered),
        AbilityKind::Activated(activated) => describe_activated_compiled(index, activated),
        AbilityKind::Mana(mana_ability) => describe_mana_compiled(index, mana_ability),
    }
}

fn describe_triggered_compiled(
    index: usize,
    triggered: &crate::ability::TriggeredAbility,
) -> Vec<String> {
    let mut intro = format!("Triggered ability {index}: {}", triggered.trigger.display());
    if let Some(condition) = &triggered.intervening_if {
        intro.push_str(&format!(", {}", describe_intervening_if(condition)));
    }

    let mut clauses = Vec::new();
    if !triggered.choices.is_empty() {
        let choices = triggered
            .choices
            .iter()
            .map(|choice| describe_choose_spec(choice, &HashMap::new()))
            .collect::<Vec<_>>()
            .join(", ");
        clauses.push(format!("choose {choices}"));
    }
    if !triggered.effects.is_empty() {
        let descriptions = collect_effect_descriptions(&triggered.effects);
        clauses.push(join_effect_descriptions(&descriptions));
    }

    if clauses.is_empty() {
        vec![intro]
    } else {
        vec![format!("{intro}: {}", clauses.join(": "))]
    }
}

fn describe_activated_compiled(
    index: usize,
    activated: &crate::ability::ActivatedAbility,
) -> Vec<String> {
    let mut intro = format!("Activated ability {index}");
    if !matches!(activated.timing, crate::ability::ActivationTiming::AnyTime) {
        intro.push_str(&format!(
            " (timing {})",
            describe_activation_timing(&activated.timing)
        ));
    }

    let mut pre_effect_parts = Vec::new();
    if !activated.mana_cost.costs().is_empty() {
        let costs = describe_cost_list(activated.mana_cost.costs(), &HashMap::new());
        pre_effect_parts.push(costs);
    }
    if !activated.choices.is_empty() {
        let choices = activated
            .choices
            .iter()
            .map(|choice| describe_choose_spec(choice, &HashMap::new()))
            .collect::<Vec<_>>()
            .join(", ");
        pre_effect_parts.push(format!("choose {choices}"));
    }

    let effects_text = if activated.effects.is_empty() {
        String::new()
    } else {
        let descriptions = collect_effect_descriptions(&activated.effects);
        join_effect_descriptions(&descriptions)
    };

    let mut line = intro;
    if !pre_effect_parts.is_empty() {
        line.push_str(": ");
        line.push_str(&pre_effect_parts.join(", "));
    }
    if !effects_text.is_empty() {
        line.push_str(": ");
        line.push_str(&effects_text);
    }

    vec![line]
}

fn describe_mana_compiled(index: usize, mana_ability: &crate::ability::ManaAbility) -> Vec<String> {
    let mut intro = format!("Mana ability {index}");
    if let Some(condition) = &mana_ability.activation_condition {
        intro.push_str(&format!(" ({})", describe_mana_condition(condition)));
    }

    let mut pre_effect_parts = Vec::new();
    if !mana_ability.mana_cost.costs().is_empty() {
        let costs = describe_cost_list(mana_ability.mana_cost.costs(), &HashMap::new());
        pre_effect_parts.push(costs);
    }
    if !mana_ability.mana.is_empty() {
        let produced = mana_ability
            .mana
            .iter()
            .map(|symbol| mana_symbol_to_oracle(*symbol))
            .collect::<Vec<_>>()
            .join("");
        pre_effect_parts.push(format!("Add {produced}"));
    }

    let effects_text = mana_ability
        .effects
        .as_ref()
        .map(|effects| join_effect_descriptions(&collect_effect_descriptions(effects)))
        .unwrap_or_default();

    let mut line = intro;
    if !pre_effect_parts.is_empty() {
        line.push_str(": ");
        line.push_str(&pre_effect_parts.join(", "));
    }
    if !effects_text.is_empty() {
        line.push_str(": ");
        line.push_str(&effects_text);
    }

    vec![line]
}

fn describe_effect_list(prefix: &str, effects: &[crate::effect::Effect]) -> Vec<String> {
    if effects.is_empty() {
        return Vec::new();
    }
    let descriptions = collect_effect_descriptions(effects);
    vec![format!(
        "{prefix}s: {}",
        join_effect_descriptions(&descriptions)
    )]
}

fn collect_effect_descriptions(effects: &[crate::effect::Effect]) -> Vec<String> {
    let mut descriptions = Vec::new();
    let mut tagged_subjects: HashMap<String, String> = HashMap::new();
    let has_non_target_only = effects.iter().any(|effect| {
        effect
            .downcast_ref::<crate::effects::TargetOnlyEffect>()
            .is_none()
    });
    let mut idx = 0usize;

    while idx < effects.len() {
        if has_non_target_only
            && effects[idx]
                .downcast_ref::<crate::effects::TargetOnlyEffect>()
                .is_some()
        {
            idx += 1;
            continue;
        }

        if idx + 1 < effects.len()
            && let Some(choose) = effects[idx].downcast_ref::<crate::effects::ChooseObjectsEffect>()
            && let Some(for_each) =
                effects[idx + 1].downcast_ref::<crate::effects::ForEachTaggedEffect>()
        {
            let shuffle = effects
                .get(idx + 2)
                .and_then(|effect| effect.downcast_ref::<crate::effects::ShuffleLibraryEffect>());
            if let Some(line) =
                describe_search_choose_for_each(choose, for_each, shuffle, tagged_subjects)
            {
                descriptions.push(line);
                idx += if shuffle.is_some() { 3 } else { 2 };
                continue;
            }
        }

        if idx + 1 < effects.len()
            && let Some(line) = describe_exile_then_gain_life_combo(
                &effects[idx],
                &effects[idx + 1],
                &mut tagged_subjects,
            )
        {
            descriptions.push(line);
            idx += 2;
            continue;
        }

        descriptions.push(describe_single_effect_line(
            &effects[idx],
            &mut tagged_subjects,
        ));
        idx += 1;
    }

    descriptions
}

fn ensure_sentence(text: &str) -> String {
    let cleaned = cleanup_decompiled_text(text);
    let trimmed = cleaned.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    if trimmed.ends_with('.') || trimmed.ends_with('!') || trimmed.ends_with('?') {
        trimmed.to_string()
    } else {
        format!("{trimmed}.")
    }
}

fn cleanup_decompiled_text(text: &str) -> String {
    let mut out = text.to_string();
    while out.contains("target target") {
        out = out.replace("target target", "target");
    }
    while out.contains("Target target") {
        out = out.replace("Target target", "Target");
    }
    out
}

fn join_effect_descriptions(descriptions: &[String]) -> String {
    descriptions
        .iter()
        .map(|line| ensure_sentence(line))
        .collect::<Vec<_>>()
        .join(" ")
}

fn describe_exile_then_gain_life_combo(
    first: &crate::effect::Effect,
    second: &crate::effect::Effect,
    tagged_subjects: &mut HashMap<String, String>,
) -> Option<String> {
    let tagged = first.downcast_ref::<crate::effects::TaggedEffect>()?;
    let move_to_zone = tagged
        .effect
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if move_to_zone.zone != Zone::Exile {
        return None;
    }

    let gain = second.downcast_ref::<crate::effects::GainLifeEffect>()?;
    let tag = tagged.tag.as_str();

    if !matches!(
        gain.player,
        crate::target::ChooseSpec::Player(crate::target::PlayerFilter::ControllerOf(
            crate::target::ObjectRef::Tagged(ref t)
        )) if t.as_str() == tag
    ) {
        return None;
    }

    if !matches!(
        gain.amount,
        crate::effect::Value::PowerOf(ref spec)
            if matches!(spec.base(), crate::target::ChooseSpec::Tagged(t) if t.as_str() == tag)
    ) {
        return None;
    }

    let target_text = describe_choose_spec(&move_to_zone.target, tagged_subjects);
    let noun_phrase = strip_leading_article(&target_text);
    tagged_subjects.insert(tag.to_string(), noun_phrase.to_string());
    Some(format!(
        "Exile {target}. Its controller gains life equal to its power.",
        target = target_text
    ))
}

fn describe_single_effect_line(
    effect: &crate::effect::Effect,
    tagged_subjects: &mut HashMap<String, String>,
) -> String {
    let body = describe_single_effect_body(effect, tagged_subjects);
    body
}

fn describe_single_effect_body(
    effect: &crate::effect::Effect,
    tagged_subjects: &mut HashMap<String, String>,
) -> String {
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        if let Some(spec) = tagged.effect.0.get_target_spec() {
            tagged_subjects.insert(
                tagged.tag.as_str().to_string(),
                strip_leading_article(&describe_choose_spec(spec, tagged_subjects)).to_string(),
            );
        }
        let inner = describe_effect_core(&tagged.effect, tagged_subjects);
        return inner;
    }

    describe_effect_core(effect, tagged_subjects)
}

fn describe_effects_inline(
    effects: &[crate::effect::Effect],
    tagged_subjects: &HashMap<String, String>,
) -> String {
    if effects.is_empty() {
        return "no effect".to_string();
    }

    let mut local_tags = tagged_subjects.clone();
    let mut parts = Vec::new();
    let mut idx = 0usize;
    while idx < effects.len() {
        if idx + 1 < effects.len()
            && let Some(tagged) = effects[idx].downcast_ref::<crate::effects::TaggedEffect>()
            && let Some(move_back) =
                effects[idx + 1].downcast_ref::<crate::effects::MoveToZoneEffect>()
            && let Some(compact) = describe_exile_then_return(tagged, move_back, &local_tags)
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        if idx + 1 < effects.len()
            && let Some(choose) =
                effects[idx].downcast_ref::<crate::effects::ChooseObjectsEffect>()
            && let Some(for_each) =
                effects[idx + 1].downcast_ref::<crate::effects::ForEachTaggedEffect>()
        {
            let shuffle = effects
                .get(idx + 2)
                .and_then(|effect| effect.downcast_ref::<crate::effects::ShuffleLibraryEffect>());
            if let Some(compact) =
                describe_search_choose_for_each(choose, for_each, shuffle, &local_tags)
            {
                parts.push(compact.trim_end_matches('.').to_string());
                idx += if shuffle.is_some() { 3 } else { 2 };
                continue;
            }
        }
        if idx + 1 < effects.len()
            && let Some(choose) =
                effects[idx].downcast_ref::<crate::effects::ChooseObjectsEffect>()
            && let Some(sacrifice) =
                effects[idx + 1].downcast_ref::<crate::effects::SacrificeEffect>()
            && let Some(compact) =
                describe_choose_then_sacrifice(choose, sacrifice, &local_tags)
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        if idx + 1 < effects.len()
            && let Some(draw) = effects[idx].downcast_ref::<crate::effects::DrawCardsEffect>()
            && let Some(discard) = effects[idx + 1].downcast_ref::<crate::effects::DiscardEffect>()
            && let Some(compact) = describe_draw_then_discard(draw, discard, &local_tags)
        {
            parts.push(compact);
            idx += 2;
            continue;
        }
        parts.push(describe_single_effect_body(&effects[idx], &mut local_tags));
        idx += 1;
    }

    parts.join(" Then ")
}

fn describe_cost_component(cost: &crate::costs::Cost) -> String {
    if let Some(mana_cost) = cost.mana_cost_ref() {
        return format!("Pay {}", mana_cost.to_oracle());
    }
    if let Some(effect) = cost.effect_ref() {
        return describe_effect_core(effect, &HashMap::new());
    }
    if cost.requires_tap() {
        return "{T}".to_string();
    }
    if cost.requires_untap() {
        return "{Q}".to_string();
    }
    if let Some(amount) = cost.life_amount() {
        return if amount == 1 {
            "Pay 1 life".to_string()
        } else {
            format!("Pay {amount} life")
        };
    }
    if cost.is_sacrifice_self() {
        return "Sacrifice this source".to_string();
    }
    let display = cost.display().trim().to_string();
    if display.is_empty() {
        format!("{cost:?}")
    } else {
        display
    }
}

fn describe_cost_list(
    costs: &[crate::costs::Cost],
    tagged_subjects: &HashMap<String, String>,
) -> String {
    let mut parts = Vec::new();
    let mut idx = 0usize;
    while idx < costs.len() {
        if idx + 1 < costs.len()
            && let Some(choose) = costs[idx]
                .effect_ref()
                .and_then(|effect| effect.downcast_ref::<crate::effects::ChooseObjectsEffect>())
            && let Some(sacrifice) = costs[idx + 1]
                .effect_ref()
                .and_then(|effect| effect.downcast_ref::<crate::effects::SacrificeEffect>())
            && let Some(compact) = describe_choose_then_sacrifice(choose, sacrifice, tagged_subjects)
        {
            parts.push(compact.trim_end_matches('.').to_string());
            idx += 2;
            continue;
        }
        parts.push(describe_cost_component(&costs[idx]));
        idx += 1;
    }
    parts.join(", ")
}

fn describe_activation_timing(timing: &crate::ability::ActivationTiming) -> &'static str {
    match timing {
        crate::ability::ActivationTiming::AnyTime => "any time",
        crate::ability::ActivationTiming::SorcerySpeed => "sorcery speed",
        crate::ability::ActivationTiming::DuringCombat => "during combat",
        crate::ability::ActivationTiming::OncePerTurn => "once per turn",
        crate::ability::ActivationTiming::DuringYourTurn => "during your turn",
        crate::ability::ActivationTiming::DuringOpponentsTurn => "during opponents' turns",
    }
}

fn describe_intervening_if(condition: &crate::ability::InterveningIfCondition) -> String {
    match condition {
        crate::ability::InterveningIfCondition::YouControl(filter) => {
            format!("you control {}", filter.description())
        }
        crate::ability::InterveningIfCondition::OpponentControls(filter) => {
            format!(
                "an opponent controls {}",
                strip_leading_article(&filter.description())
            )
        }
        crate::ability::InterveningIfCondition::LifeTotalAtLeast(value) => {
            format!("your life total is at least {value}")
        }
        crate::ability::InterveningIfCondition::LifeTotalAtMost(value) => {
            format!("your life total is at most {value}")
        }
        crate::ability::InterveningIfCondition::NoCreaturesDiedThisTurn => {
            "no creature died this turn".to_string()
        }
        crate::ability::InterveningIfCondition::CreatureDiedThisTurn => {
            "a creature died this turn".to_string()
        }
        crate::ability::InterveningIfCondition::FirstTimeThisTurn => {
            "this is the first time this turn".to_string()
        }
        crate::ability::InterveningIfCondition::WasEnchanted => {
            "the source was enchanted".to_string()
        }
        crate::ability::InterveningIfCondition::HadCounters(counter_type, amount) => {
            format!("the source had at least {amount} {counter_type:?} counter(s)")
        }
    }
}

fn describe_mana_condition(condition: &crate::ability::ManaAbilityCondition) -> String {
    match condition {
        crate::ability::ManaAbilityCondition::ControlLandWithSubtype(subtypes) => {
            if subtypes.is_empty() {
                "you control a land with required subtype".to_string()
            } else if subtypes.len() == 1 {
                format!(
                    "you control a {}",
                    split_camel_case(&format!("{:?}", subtypes[0]))
                )
            } else {
                let names = subtypes
                    .iter()
                    .map(|subtype| split_camel_case(&format!("{subtype:?}")))
                    .collect::<Vec<_>>()
                    .join(" or ");
                format!("you control a land with subtype {names}")
            }
        }
    }
}

fn describe_until(
    until: &crate::effect::Until,
    tagged_subjects: &HashMap<String, String>,
) -> String {
    match until {
        crate::effect::Until::Forever => "forever".to_string(),
        crate::effect::Until::EndOfTurn => "until end of turn".to_string(),
        crate::effect::Until::YourNextTurn => "until your next turn".to_string(),
        crate::effect::Until::EndOfCombat => "until end of combat".to_string(),
        crate::effect::Until::ThisLeavesTheBattlefield => {
            "while this remains on the battlefield".to_string()
        }
        crate::effect::Until::YouStopControllingThis => "while you control this".to_string(),
        crate::effect::Until::TurnsPass(value) => {
            format!("for {}", describe_value(value, tagged_subjects))
        }
    }
}

fn describe_condition(
    condition: &crate::effect::Condition,
    tagged_subjects: &HashMap<String, String>,
) -> String {
    match condition {
        crate::effect::Condition::YouControl(filter) => {
            format!("you control {}", filter.description())
        }
        crate::effect::Condition::OpponentControls(filter) => {
            format!(
                "an opponent controls {}",
                strip_leading_article(&filter.description())
            )
        }
        crate::effect::Condition::LifeTotalOrLess(value) => {
            format!("your life total is {value} or less")
        }
        crate::effect::Condition::LifeTotalOrGreater(value) => {
            format!("your life total is {value} or greater")
        }
        crate::effect::Condition::CardsInHandOrMore(value) => {
            format!("you have {value} or more cards in hand")
        }
        crate::effect::Condition::YourTurn => "it is your turn".to_string(),
        crate::effect::Condition::CreatureDiedThisTurn => "a creature died this turn".to_string(),
        crate::effect::Condition::CastSpellThisTurn => "you cast a spell this turn".to_string(),
        crate::effect::Condition::TargetIsTapped => "the target is tapped".to_string(),
        crate::effect::Condition::SourceIsTapped => "this source is tapped".to_string(),
        crate::effect::Condition::TargetIsAttacking => "the target is attacking".to_string(),
        crate::effect::Condition::ManaSpentToCastThisSpellAtLeast { amount, symbol } => {
            if let Some(symbol) = symbol {
                format!(
                    "at least {amount} {} mana was spent to cast this spell",
                    mana_symbol_to_oracle(*symbol)
                )
            } else {
                format!("at least {amount} mana was spent to cast this spell")
            }
        }
        crate::effect::Condition::YouControlCommander => "you control your commander".to_string(),
        crate::effect::Condition::TaggedObjectMatches(tag, filter) => {
            format!(
                "the tagged object '{}' matches {}",
                tag.as_str(),
                filter.description()
            )
        }
        crate::effect::Condition::Not(inner) => {
            format!("not ({})", describe_condition(inner, tagged_subjects))
        }
        crate::effect::Condition::And(left, right) => format!(
            "({}) and ({})",
            describe_condition(left, tagged_subjects),
            describe_condition(right, tagged_subjects)
        ),
        crate::effect::Condition::Or(left, right) => format!(
            "({}) or ({})",
            describe_condition(left, tagged_subjects),
            describe_condition(right, tagged_subjects)
        ),
    }
}

fn describe_effect_predicate(predicate: &crate::effect::EffectPredicate) -> String {
    match predicate {
        crate::effect::EffectPredicate::Succeeded => "it succeeded".to_string(),
        crate::effect::EffectPredicate::Failed => "it failed".to_string(),
        crate::effect::EffectPredicate::Happened => "it happened".to_string(),
        crate::effect::EffectPredicate::DidNotHappen => "it did not happen".to_string(),
        crate::effect::EffectPredicate::HappenedNotReplaced => {
            "it happened and was not replaced".to_string()
        }
        crate::effect::EffectPredicate::Value(comparison) => {
            format!("its value {}", describe_comparison(comparison))
        }
        crate::effect::EffectPredicate::Chosen => "it was chosen".to_string(),
        crate::effect::EffectPredicate::WasDeclined => "it was declined".to_string(),
    }
}

fn describe_comparison(comparison: &crate::effect::Comparison) -> String {
    match comparison {
        crate::effect::Comparison::GreaterThan(n) => format!("is greater than {n}"),
        crate::effect::Comparison::GreaterThanOrEqual(n) => format!("is at least {n}"),
        crate::effect::Comparison::Equal(n) => format!("equals {n}"),
        crate::effect::Comparison::LessThan(n) => format!("is less than {n}"),
        crate::effect::Comparison::LessThanOrEqual(n) => format!("is at most {n}"),
        crate::effect::Comparison::NotEqual(n) => format!("is not equal to {n}"),
    }
}

fn describe_restriction(
    restriction: &crate::effect::Restriction,
    tagged_subjects: &HashMap<String, String>,
) -> String {
    match restriction {
        crate::effect::Restriction::GainLife(filter) => {
            format!(
                "{} can't gain life",
                describe_player_filter(filter, tagged_subjects)
            )
        }
        crate::effect::Restriction::SearchLibraries(filter) => format!(
            "{} can't search libraries",
            describe_player_filter(filter, tagged_subjects)
        ),
        crate::effect::Restriction::CastSpells(filter) => {
            format!(
                "{} can't cast spells",
                describe_player_filter(filter, tagged_subjects)
            )
        }
        crate::effect::Restriction::DrawCards(filter) => {
            format!(
                "{} can't draw cards",
                describe_player_filter(filter, tagged_subjects)
            )
        }
        crate::effect::Restriction::DrawExtraCards(filter) => format!(
            "{} can't draw extra cards",
            describe_player_filter(filter, tagged_subjects)
        ),
        crate::effect::Restriction::ChangeLifeTotal(filter) => format!(
            "{} can't have life total changed",
            describe_player_filter(filter, tagged_subjects)
        ),
        crate::effect::Restriction::LoseGame(filter) => {
            format!(
                "{} can't lose the game",
                describe_player_filter(filter, tagged_subjects)
            )
        }
        crate::effect::Restriction::WinGame(filter) => {
            format!(
                "{} can't win the game",
                describe_player_filter(filter, tagged_subjects)
            )
        }
        crate::effect::Restriction::PreventDamage => "damage can't be prevented".to_string(),
        crate::effect::Restriction::Attack(filter) => {
            format!("{} can't attack", filter.description())
        }
        crate::effect::Restriction::Block(filter) => {
            format!("{} can't block", filter.description())
        }
        crate::effect::Restriction::Untap(filter) => {
            format!("{} can't untap", filter.description())
        }
        crate::effect::Restriction::BeBlocked(filter) => {
            format!("{} can't be blocked", filter.description())
        }
        crate::effect::Restriction::BeDestroyed(filter) => {
            format!("{} can't be destroyed", filter.description())
        }
        crate::effect::Restriction::BeSacrificed(filter) => {
            format!("{} can't be sacrificed", filter.description())
        }
        crate::effect::Restriction::HaveCountersPlaced(filter) => {
            format!("counters can't be placed on {}", filter.description())
        }
        crate::effect::Restriction::BeTargeted(filter) => {
            format!("{} can't be targeted", filter.description())
        }
        crate::effect::Restriction::BeCountered(filter) => {
            format!("{} can't be countered", filter.description())
        }
    }
}

fn describe_choice_count(count: &crate::effect::ChoiceCount) -> String {
    match count.max {
        Some(max) if max == count.min => {
            if max == 1 {
                "exactly one".to_string()
            } else {
                format!("exactly {max}")
            }
        }
        Some(max) if count.min == 0 => format!("up to {max}"),
        Some(max) => format!("between {} and {max}", count.min),
        None if count.min == 0 => "any number of".to_string(),
        None => format!("at least {}", count.min),
    }
}

fn describe_damage_filter(filter: &crate::prevention::DamageFilter) -> String {
    let mut parts = Vec::new();
    if filter.combat_only {
        parts.push("combat damage".to_string());
    } else if filter.noncombat_only {
        parts.push("noncombat damage".to_string());
    }
    if let Some(source_filter) = &filter.from_source {
        parts.push(format!("damage from {}", source_filter.description()));
    }
    if let Some(colors) = &filter.from_colors
        && !colors.is_empty()
    {
        let colors_text = colors
            .iter()
            .map(|color| split_camel_case(&format!("{color:?}")))
            .collect::<Vec<_>>()
            .join(" or ");
        parts.push(format!("damage from {colors_text} sources"));
    }
    if let Some(types) = &filter.from_card_types
        && !types.is_empty()
    {
        let types_text = types
            .iter()
            .map(|card_type| split_camel_case(&format!("{card_type:?}")))
            .collect::<Vec<_>>()
            .join(" or ");
        parts.push(format!("damage from {types_text} sources"));
    }
    if filter.from_specific_source.is_some() {
        parts.push("damage from a specific source".to_string());
    }
    if parts.is_empty() {
        "all damage".to_string()
    } else {
        parts.join(", ")
    }
}

fn describe_prevention_target(
    target: &crate::prevention::PreventionTarget,
    _tagged_subjects: &HashMap<String, String>,
) -> String {
    match target {
        crate::prevention::PreventionTarget::Player(player) => {
            format!("player {}", player.0 + 1)
        }
        crate::prevention::PreventionTarget::Permanent(_) => "that permanent".to_string(),
        crate::prevention::PreventionTarget::PermanentsMatching(filter) => {
            format!("permanents matching {}", filter.description())
        }
        crate::prevention::PreventionTarget::You => "you".to_string(),
        crate::prevention::PreventionTarget::YouAndPermanentsYouControl => {
            "you and permanents you control".to_string()
        }
        crate::prevention::PreventionTarget::All => "all players and permanents".to_string(),
    }
}

fn describe_static_abilities(abilities: &[crate::static_abilities::StaticAbility]) -> String {
    if abilities.is_empty() {
        return "no abilities".to_string();
    }
    abilities
        .iter()
        .map(|ability| ability.display())
        .collect::<Vec<_>>()
        .join(", ")
}

fn describe_effect_core(
    effect: &crate::effect::Effect,
    tagged_subjects: &HashMap<String, String>,
) -> String {
    if let Some(text) = describe_effect_core_expanded(effect, tagged_subjects) {
        return text;
    }
    format!("{effect:?}")
}

fn with_indefinite_article(noun: &str) -> String {
    let trimmed = noun.trim();
    if trimmed.is_empty() {
        return "a permanent".to_string();
    }
    if trimmed.starts_with("a ")
        || trimmed.starts_with("an ")
        || trimmed.starts_with("another ")
        || trimmed.starts_with("target ")
        || trimmed.starts_with("each ")
        || trimmed.starts_with("all ")
        || trimmed
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_digit())
    {
        return trimmed.to_string();
    }
    let first = trimmed.chars().next().unwrap_or('a').to_ascii_lowercase();
    let article = if matches!(first, 'a' | 'e' | 'i' | 'o' | 'u') {
        "an"
    } else {
        "a"
    };
    format!("{article} {trimmed}")
}

fn sacrifice_uses_chosen_tag(filter: &crate::target::ObjectFilter, tag: &str) -> bool {
    filter.tagged_constraints.iter().any(|constraint| {
        constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
            && constraint.tag.as_str() == tag
    })
}

fn describe_choose_then_sacrifice(
    choose: &crate::effects::ChooseObjectsEffect,
    sacrifice: &crate::effects::SacrificeEffect,
    tagged_subjects: &HashMap<String, String>,
) -> Option<String> {
    if choose.zone != crate::zone::Zone::Battlefield
        || choose.is_search
        || !choose.count.is_single()
        || !matches!(sacrifice.count, crate::effect::Value::Fixed(1))
        || sacrifice.player != choose.chooser
        || !sacrifice_uses_chosen_tag(&sacrifice.filter, choose.tag.as_str())
    {
        return None;
    }

    let player = describe_player_filter(&choose.chooser, tagged_subjects);
    let verb = if player == "You" {
        "sacrifice"
    } else {
        "sacrifices"
    };
    let chosen = with_indefinite_article(&choose.filter.description());
    Some(format!("{player} {verb} {chosen}."))
}

fn describe_draw_then_discard(
    draw: &crate::effects::DrawCardsEffect,
    discard: &crate::effects::DiscardEffect,
    tagged_subjects: &HashMap<String, String>,
) -> Option<String> {
    if draw.player != discard.player {
        return None;
    }
    let player = describe_player_filter(&draw.player, tagged_subjects);
    let draw_verb = if player == "You" { "draw" } else { "draws" };
    let discard_verb = if player == "You" {
        "discard"
    } else {
        "discards"
    };
    let mut text = format!(
        "{player} {draw_verb} {} card(s), then {discard_verb} {} card(s)",
        describe_value(&draw.count, tagged_subjects),
        describe_value(&discard.count, tagged_subjects)
    );
    if discard.random {
        text.push_str(" at random");
    }
    text.push('.');
    Some(text)
}

fn describe_exile_then_return(
    tagged: &crate::effects::TaggedEffect,
    move_back: &crate::effects::MoveToZoneEffect,
    tagged_subjects: &HashMap<String, String>,
) -> Option<String> {
    if move_back.zone != Zone::Battlefield {
        return None;
    }
    let crate::target::ChooseSpec::Tagged(return_tag) = &move_back.target else {
        return None;
    };
    if !return_tag.as_str().starts_with("exiled_") || return_tag != &tagged.tag {
        return None;
    }
    let exile_move = tagged
        .effect
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if exile_move.zone != Zone::Exile {
        return None;
    }

    let target = describe_choose_spec(&exile_move.target, tagged_subjects);
    Some(format!("Exile {target}, then return it to the battlefield"))
}

enum SearchDestination {
    Battlefield { tapped: bool },
    Hand,
    LibraryTop,
}

fn describe_search_choose_for_each(
    choose: &crate::effects::ChooseObjectsEffect,
    for_each: &crate::effects::ForEachTaggedEffect,
    shuffle: Option<&crate::effects::ShuffleLibraryEffect>,
    tagged_subjects: &HashMap<String, String>,
) -> Option<String> {
    if !choose.is_search || choose.zone != crate::zone::Zone::Library {
        return None;
    }
    if for_each.tag != choose.tag || for_each.effects.len() != 1 {
        return None;
    }

    let destination = if let Some(put) =
        for_each.effects[0].downcast_ref::<crate::effects::PutOntoBattlefieldEffect>()
    {
        if !matches!(put.target, crate::target::ChooseSpec::Iterated) {
            return None;
        }
        SearchDestination::Battlefield { tapped: put.tapped }
    } else if let Some(return_to_hand) =
        for_each.effects[0].downcast_ref::<crate::effects::ReturnToHandEffect>()
    {
        if !matches!(return_to_hand.spec, crate::target::ChooseSpec::Iterated) {
            return None;
        }
        SearchDestination::Hand
    } else if let Some(move_to_zone) =
        for_each.effects[0].downcast_ref::<crate::effects::MoveToZoneEffect>()
    {
        if !matches!(move_to_zone.target, crate::target::ChooseSpec::Iterated) {
            return None;
        }
        if move_to_zone.zone == crate::zone::Zone::Hand {
            SearchDestination::Hand
        } else if move_to_zone.zone == crate::zone::Zone::Library && move_to_zone.to_top {
            SearchDestination::LibraryTop
        } else {
            return None;
        }
    } else {
        return None;
    };

    if let Some(shuffle) = shuffle
        && shuffle.player != choose.chooser
    {
        return None;
    }

    let chooser = describe_player_filter(&choose.chooser, tagged_subjects);
    let library_owner = if chooser == "You" {
        "your".to_string()
    } else {
        format!(
            "{}'s",
            strip_leading_article(&chooser).to_ascii_lowercase()
        )
    };
    let filter_text = choose.filter.description();
    let selection_text = if choose.count.is_single() {
        with_indefinite_article(&filter_text)
    } else {
        format!("{} {}", describe_choice_count(&choose.count), filter_text)
    };
    let pronoun = if choose.count.max == Some(1) {
        "it"
    } else {
        "them"
    };
    let mut text = match destination {
        SearchDestination::Battlefield { tapped } => {
            let mut text = format!(
                "Search {library_owner} library for {selection_text}, put {pronoun} onto the battlefield"
            );
            if tapped {
                text.push_str(" tapped");
            }
            text
        }
        SearchDestination::Hand => {
            format!("Search {library_owner} library for {selection_text}, put {pronoun} into hand")
        }
        SearchDestination::LibraryTop => format!(
            "Search {library_owner} library for {selection_text}, put {pronoun} on top of library"
        ),
    };
    if shuffle.is_some() {
        text.push_str(", then shuffle");
    }
    text.push('.');
    Some(text)
}

fn describe_search_sequence(
    sequence: &crate::effects::SequenceEffect,
    tagged_subjects: &HashMap<String, String>,
) -> Option<String> {
    if sequence.effects.len() < 2 || sequence.effects.len() > 3 {
        return None;
    }
    let choose = sequence.effects[0].downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let for_each = sequence.effects[1].downcast_ref::<crate::effects::ForEachTaggedEffect>()?;
    let shuffle = if sequence.effects.len() == 3 {
        Some(sequence.effects[2].downcast_ref::<crate::effects::ShuffleLibraryEffect>()?)
    } else {
        None
    };
    describe_search_choose_for_each(choose, for_each, shuffle, tagged_subjects)
}

fn describe_for_players_choose_then_sacrifice(
    for_players: &crate::effects::ForPlayersEffect,
) -> Option<String> {
    if for_players.effects.len() != 2 {
        return None;
    }
    let choose = for_players.effects[0].downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let sacrifice = for_players.effects[1].downcast_ref::<crate::effects::SacrificeEffect>()?;
    if choose.zone != crate::zone::Zone::Battlefield
        || choose.is_search
        || !choose.count.is_single()
        || choose.chooser != crate::target::PlayerFilter::IteratedPlayer
        || !matches!(sacrifice.count, crate::effect::Value::Fixed(1))
        || sacrifice.player != crate::target::PlayerFilter::IteratedPlayer
        || !sacrifice_uses_chosen_tag(&sacrifice.filter, choose.tag.as_str())
    {
        return None;
    }

    let (subject, verb, possessive) = match for_players.filter {
        crate::target::PlayerFilter::Any => ("Each player", "sacrifices", "their"),
        crate::target::PlayerFilter::Opponent => ("Each opponent", "sacrifices", "their"),
        crate::target::PlayerFilter::You => ("You", "sacrifice", "your"),
        _ => return None,
    };
    let chosen = with_indefinite_article(&choose.filter.description());
    Some(format!(
        "{subject} {verb} {chosen} of {possessive} choice."
    ))
}

fn describe_effect_core_expanded(
    effect: &crate::effect::Effect,
    tagged_subjects: &HashMap<String, String>,
) -> Option<String> {
    if let Some(conditional) = effect.downcast_ref::<crate::effects::ConditionalEffect>() {
        let then_text = describe_effects_inline(&conditional.if_true, tagged_subjects);
        if conditional.if_false.is_empty() {
            return Some(format!(
                "If {}, then {}.",
                describe_condition(&conditional.condition, tagged_subjects),
                then_text
            ));
        }
        let else_text = describe_effects_inline(&conditional.if_false, tagged_subjects);
        return Some(format!(
            "If {}, then {}. Otherwise {}.",
            describe_condition(&conditional.condition, tagged_subjects),
            then_text,
            else_text
        ));
    }
    if let Some(if_effect) = effect.downcast_ref::<crate::effects::IfEffect>() {
        let then_text = describe_effects_inline(&if_effect.then, tagged_subjects);
        if if_effect.else_.is_empty() {
            return Some(format!(
                "If effect #{} satisfies '{}', then {}.",
                if_effect.condition.0,
                describe_effect_predicate(&if_effect.predicate),
                then_text
            ));
        }
        let else_text = describe_effects_inline(&if_effect.else_, tagged_subjects);
        return Some(format!(
            "If effect #{} satisfies '{}', then {}. Otherwise {}.",
            if_effect.condition.0,
            describe_effect_predicate(&if_effect.predicate),
            then_text,
            else_text
        ));
    }
    if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
        return Some(format!(
            "Execute and store result as effect #{}: {}.",
            with_id.id.0,
            describe_effect_core(&with_id.effect, tagged_subjects)
        ));
    }
    if let Some(may) = effect.downcast_ref::<crate::effects::MayEffect>() {
        return Some(format!(
            "You may {}.",
            describe_effects_inline(&may.effects, tagged_subjects)
        ));
    }
    if let Some(sequence) = effect.downcast_ref::<crate::effects::SequenceEffect>() {
        if let Some(compact) = describe_search_sequence(sequence, tagged_subjects) {
            return Some(compact);
        }
        return Some(format!(
            "Sequence: {}.",
            describe_effects_inline(&sequence.effects, tagged_subjects)
        ));
    }
    if let Some(choose_mode) = effect.downcast_ref::<crate::effects::ChooseModeEffect>() {
        let max = describe_value(&choose_mode.choose_count, tagged_subjects);
        let min = choose_mode
            .min_choose_count
            .as_ref()
            .map(|value| describe_value(value, tagged_subjects))
            .unwrap_or_else(|| max.clone());
        let modes = choose_mode
            .modes
            .iter()
            .map(|mode| {
                format!(
                    "{} -> {}",
                    mode.description,
                    describe_effects_inline(&mode.effects, tagged_subjects)
                )
            })
            .collect::<Vec<_>>()
            .join(" | ");
        return Some(format!("Choose between {min} and {max} mode(s): {modes}."));
    }
    if let Some(choose_objects) = effect.downcast_ref::<crate::effects::ChooseObjectsEffect>() {
        return Some(format!(
            "{} {} {} {} in {} and tag as '{}'.",
            describe_player_filter(&choose_objects.chooser, tagged_subjects),
            if choose_objects.is_search {
                "searches for"
            } else {
                "chooses"
            },
            describe_choice_count(&choose_objects.count),
            pluralize_noun_phrase(&choose_objects.filter.description()),
            zone_name(choose_objects.zone),
            choose_objects.tag.as_str()
        ));
    }
    if let Some(for_each_object) = effect.downcast_ref::<crate::effects::ForEachObject>() {
        return Some(format!(
            "For each {}, {}.",
            for_each_object.filter.description(),
            describe_effects_inline(&for_each_object.effects, tagged_subjects)
        ));
    }
    if let Some(for_each_opponent) = effect.downcast_ref::<crate::effects::ForEachOpponentEffect>()
    {
        return Some(format!(
            "For each opponent, {}.",
            describe_effects_inline(for_each_opponent.effects(), tagged_subjects)
        ));
    }
    if let Some(for_players) = effect.downcast_ref::<crate::effects::ForPlayersEffect>() {
        if let Some(compact) = describe_for_players_choose_then_sacrifice(for_players) {
            return Some(compact);
        }
        return Some(format!(
            "For each {}, {}.",
            pluralize_noun_phrase(&describe_player_filter(
                &for_players.filter,
                tagged_subjects
            )),
            describe_effects_inline(&for_players.effects, tagged_subjects)
        ));
    }
    if let Some(for_each_tagged) = effect.downcast_ref::<crate::effects::ForEachTaggedEffect>() {
        return Some(format!(
            "For each tagged '{}' object, {}.",
            for_each_tagged.tag.as_str(),
            describe_effects_inline(&for_each_tagged.effects, tagged_subjects)
        ));
    }
    if let Some(for_each_controller) =
        effect.downcast_ref::<crate::effects::ForEachControllerOfTaggedEffect>()
    {
        return Some(format!(
            "For each controller of tagged '{}' objects, {}.",
            for_each_controller.tag.as_str(),
            describe_effects_inline(&for_each_controller.effects, tagged_subjects)
        ));
    }
    if let Some(for_each_tagged_player) =
        effect.downcast_ref::<crate::effects::ForEachTaggedPlayerEffect>()
    {
        return Some(format!(
            "For each tagged '{}' player, {}.",
            for_each_tagged_player.tag.as_str(),
            describe_effects_inline(&for_each_tagged_player.effects, tagged_subjects)
        ));
    }
    if let Some(tag_all) = effect.downcast_ref::<crate::effects::TagAllEffect>() {
        return Some(format!(
            "Tag all targets as '{}' and {}.",
            tag_all.tag.as_str(),
            describe_effect_core(&tag_all.effect, tagged_subjects)
        ));
    }
    if let Some(target_only) = effect.downcast_ref::<crate::effects::TargetOnlyEffect>() {
        return Some(format!(
            "Choose {}.",
            describe_choose_spec(&target_only.target, tagged_subjects)
        ));
    }
    if let Some(tag_trigger) = effect.downcast_ref::<crate::effects::TagTriggeringObjectEffect>() {
        return Some(format!(
            "Tag the triggering object as '{}'.",
            tag_trigger.tag.as_str()
        ));
    }
    if let Some(tag_attached) = effect.downcast_ref::<crate::effects::TagAttachedToSourceEffect>() {
        return Some(format!(
            "Tag the object attached to this source as '{}'.",
            tag_attached.tag.as_str()
        ));
    }
    if let Some(vote) = effect.downcast_ref::<crate::effects::VoteEffect>() {
        let options = vote
            .options
            .iter()
            .map(|option| {
                format!(
                    "{} -> {}",
                    option.name,
                    describe_effects_inline(&option.effects_per_vote, tagged_subjects)
                )
            })
            .collect::<Vec<_>>()
            .join(" | ");
        return Some(format!(
            "Each player votes. Options: {options}. Controller extra votes: {}. Optional extra votes: {}.",
            vote.controller_extra_votes, vote.controller_optional_extra_votes
        ));
    }

    if let Some(move_to_zone) = effect.downcast_ref::<crate::effects::MoveToZoneEffect>() {
        return Some(describe_move_to_zone(move_to_zone, tagged_subjects));
    }
    if let Some(put_onto_battlefield) =
        effect.downcast_ref::<crate::effects::PutOntoBattlefieldEffect>()
    {
        let mut text = format!(
            "Put {} onto the battlefield",
            describe_choose_spec(&put_onto_battlefield.target, tagged_subjects)
        );
        if put_onto_battlefield.tapped {
            text.push_str(" tapped");
        }
        if !matches!(
            put_onto_battlefield.controller,
            crate::target::PlayerFilter::You
        ) {
            text.push_str(&format!(
                " under {} control",
                describe_player_filter(&put_onto_battlefield.controller, tagged_subjects)
            ));
        }
        text.push('.');
        return Some(text);
    }
    if let Some(return_to_battlefield) =
        effect.downcast_ref::<crate::effects::ReturnFromGraveyardToBattlefieldEffect>()
    {
        let mut text = format!(
            "Return {} from graveyard to the battlefield",
            describe_choose_spec(&return_to_battlefield.target, tagged_subjects)
        );
        if return_to_battlefield.tapped {
            text.push_str(" tapped");
        }
        text.push('.');
        return Some(text);
    }
    if let Some(return_triggered) =
        effect.downcast_ref::<crate::effects::ReturnFromGraveyardOrExileToBattlefieldEffect>()
    {
        let mut text =
            "Return the linked card from graveyard or exile to the battlefield".to_string();
        if return_triggered.tapped {
            text.push_str(" tapped");
        }
        text.push('.');
        return Some(text);
    }
    if let Some(return_from_graveyard) =
        effect.downcast_ref::<crate::effects::ReturnFromGraveyardToHandEffect>()
    {
        return Some(format!(
            "Return {} from graveyard to hand.",
            describe_choose_spec(&return_from_graveyard.target, tagged_subjects)
        ));
    }
    if let Some(gain_life) = effect.downcast_ref::<crate::effects::GainLifeEffect>() {
        return Some(describe_gain_life(gain_life, tagged_subjects));
    }
    if let Some(lose_life) = effect.downcast_ref::<crate::effects::LoseLifeEffect>() {
        return Some(format!(
            "{} loses {} life.",
            describe_gain_life_player(&lose_life.player, tagged_subjects),
            describe_value(&lose_life.amount, tagged_subjects)
        ));
    }
    if let Some(set_life_total) = effect.downcast_ref::<crate::effects::SetLifeTotalEffect>() {
        return Some(format!(
            "Set {} life total to {}.",
            describe_player_filter(&set_life_total.player, tagged_subjects),
            describe_value(&set_life_total.amount, tagged_subjects)
        ));
    }
    if let Some(exchange_life_totals) =
        effect.downcast_ref::<crate::effects::ExchangeLifeTotalsEffect>()
    {
        return Some(format!(
            "Exchange life totals between {} and {}.",
            describe_player_filter(&exchange_life_totals.player1, tagged_subjects),
            describe_player_filter(&exchange_life_totals.player2, tagged_subjects)
        ));
    }
    if let Some(draw) = effect.downcast_ref::<crate::effects::DrawCardsEffect>() {
        let player = describe_player_filter(&draw.player, tagged_subjects);
        let verb = if player == "You" { "draw" } else { "draws" };
        return Some(format!(
            "{} {} {} card(s).",
            player,
            verb,
            describe_value(&draw.count, tagged_subjects)
        ));
    }
    if let Some(mill) = effect.downcast_ref::<crate::effects::MillEffect>() {
        let player = describe_player_filter(&mill.player, tagged_subjects);
        let verb = if player == "You" { "mill" } else { "mills" };
        return Some(format!(
            "{} {} {} card(s).",
            player,
            verb,
            describe_value(&mill.count, tagged_subjects)
        ));
    }
    if let Some(scry) = effect.downcast_ref::<crate::effects::ScryEffect>() {
        let player = describe_player_filter(&scry.player, tagged_subjects);
        let verb = if player == "You" { "scry" } else { "scries" };
        return Some(format!(
            "{} {} {}.",
            player,
            verb,
            describe_value(&scry.count, tagged_subjects)
        ));
    }
    if let Some(surveil) = effect.downcast_ref::<crate::effects::SurveilEffect>() {
        let player = describe_player_filter(&surveil.player, tagged_subjects);
        let verb = if player == "You" {
            "surveil"
        } else {
            "surveils"
        };
        return Some(format!(
            "{} {} {}.",
            player,
            verb,
            describe_value(&surveil.count, tagged_subjects)
        ));
    }
    if let Some(discard) = effect.downcast_ref::<crate::effects::DiscardEffect>() {
        let player = describe_player_filter(&discard.player, tagged_subjects);
        let verb = if player == "You" {
            "discard"
        } else {
            "discards"
        };
        let mut text = format!(
            "{} {} {} card(s)",
            player,
            verb,
            describe_value(&discard.count, tagged_subjects)
        );
        if discard.random {
            text.push_str(" at random");
        }
        text.push('.');
        return Some(text);
    }
    if let Some(discard_hand) = effect.downcast_ref::<crate::effects::DiscardHandEffect>() {
        let player = describe_player_filter(&discard_hand.player, tagged_subjects);
        let verb = if player == "You" {
            "discard your hand"
        } else {
            "discards their hand"
        };
        return Some(format!("{} {}.", player, verb));
    }
    if let Some(search_library) = effect.downcast_ref::<crate::effects::SearchLibraryEffect>() {
        let destination = match search_library.destination {
            Zone::Hand => "hand".to_string(),
            Zone::Battlefield => "battlefield".to_string(),
            Zone::Library => "top of library".to_string(),
            zone => format!("{zone:?}"),
        };
        return Some(format!(
            "{} searches their library for {} and puts it into {}{}.",
            describe_player_filter(&search_library.player, tagged_subjects),
            search_library.filter.description(),
            destination,
            if search_library.reveal {
                ", then reveals it"
            } else {
                ""
            }
        ));
    }
    if let Some(shuffle_library) = effect.downcast_ref::<crate::effects::ShuffleLibraryEffect>() {
        return Some(format!(
            "{} shuffles their library.",
            describe_player_filter(&shuffle_library.player, tagged_subjects)
        ));
    }
    if let Some(look_at_hand) = effect.downcast_ref::<crate::effects::LookAtHandEffect>() {
        return Some(format!(
            "Look at {} hand.",
            describe_choose_spec(&look_at_hand.target, tagged_subjects)
        ));
    }
    if let Some(reveal_top) = effect.downcast_ref::<crate::effects::RevealTopEffect>() {
        let mut text = format!(
            "Reveal the top card of {} library",
            describe_player_filter(&reveal_top.player, tagged_subjects)
        );
        if let Some(tag) = &reveal_top.tag {
            text.push_str(&format!(" and tag it as '{}'", tag.as_str()));
        }
        text.push('.');
        return Some(text);
    }
    if let Some(imprint_from_hand) =
        effect.downcast_ref::<crate::effects::cards::ImprintFromHandEffect>()
    {
        return Some(format!(
            "Imprint a card from hand matching {}.",
            imprint_from_hand.filter.description()
        ));
    }
    if let Some(damage) = effect.downcast_ref::<crate::effects::DealDamageEffect>() {
        let mut text = format!(
            "Deal {} damage to {}",
            describe_value(&damage.amount, tagged_subjects),
            describe_choose_spec(&damage.target, tagged_subjects)
        );
        if damage.source_is_combat {
            text.push_str(" as combat damage");
        }
        text.push('.');
        return Some(text);
    }
    if let Some(prevent_damage) = effect.downcast_ref::<crate::effects::PreventDamageEffect>() {
        return Some(format!(
            "Prevent the next {} {} to {} {}.",
            describe_value(&prevent_damage.amount, tagged_subjects),
            describe_damage_filter(&prevent_damage.damage_filter),
            describe_choose_spec(&prevent_damage.target, tagged_subjects),
            describe_until(&prevent_damage.duration, tagged_subjects)
        ));
    }
    if let Some(prevent_all) = effect.downcast_ref::<crate::effects::PreventAllDamageEffect>() {
        return Some(format!(
            "Prevent {} to {} {}.",
            describe_damage_filter(&prevent_all.damage_filter),
            describe_prevention_target(&prevent_all.target, tagged_subjects),
            describe_until(&prevent_all.until, tagged_subjects)
        ));
    }
    if let Some(clear_damage) = effect.downcast_ref::<crate::effects::ClearDamageEffect>() {
        return Some(format!(
            "Clear damage from {}.",
            describe_choose_spec(&clear_damage.target, tagged_subjects)
        ));
    }
    if let Some(fight) = effect.downcast_ref::<crate::effects::FightEffect>() {
        return Some(format!(
            "{} fights {}.",
            describe_choose_spec(&fight.creature1, tagged_subjects),
            describe_choose_spec(&fight.creature2, tagged_subjects)
        ));
    }
    if let Some(counter) = effect.downcast_ref::<crate::effects::CounterEffect>() {
        return Some(format!(
            "Counter {}.",
            describe_choose_spec(&counter.target, tagged_subjects)
        ));
    }
    if let Some(counter_unless) = effect.downcast_ref::<crate::effects::CounterUnlessPaysEffect>() {
        let mana = counter_unless
            .mana
            .iter()
            .map(|symbol| mana_symbol_to_oracle(*symbol))
            .collect::<Vec<_>>()
            .join("");
        return Some(format!(
            "Counter {} unless its controller pays {}.",
            describe_choose_spec(&counter_unless.target, tagged_subjects),
            if mana.is_empty() {
                "{0}".to_string()
            } else {
                mana
            }
        ));
    }
    if let Some(copy_spell) = effect.downcast_ref::<crate::effects::CopySpellEffect>() {
        return Some(format!(
            "Copy {} {} time(s).",
            describe_choose_spec(&copy_spell.target, tagged_subjects),
            describe_value(&copy_spell.count, tagged_subjects)
        ));
    }
    if let Some(choose_new_targets) =
        effect.downcast_ref::<crate::effects::ChooseNewTargetsEffect>()
    {
        return Some(format!(
            "{} choose new targets for copied effect #{}.",
            if choose_new_targets.may {
                "You may"
            } else {
                "Choose"
            },
            choose_new_targets.from_effect.0
        ));
    }
    if let Some(return_to_hand) = effect.downcast_ref::<crate::effects::ReturnToHandEffect>() {
        return Some(format!(
            "Return {} to its owner's hand.",
            describe_choose_spec(&return_to_hand.spec, tagged_subjects)
        ));
    }
    if let Some(exile) = effect.downcast_ref::<crate::effects::ExileEffect>() {
        return Some(format!(
            "Exile {}.",
            describe_choose_spec(&exile.spec, tagged_subjects)
        ));
    }
    if let Some(destroy) = effect.downcast_ref::<crate::effects::DestroyEffect>() {
        return Some(format!(
            "Destroy {}.",
            describe_choose_spec(&destroy.spec, tagged_subjects)
        ));
    }
    if let Some(sacrifice) = effect.downcast_ref::<crate::effects::SacrificeEffect>() {
        return Some(format!(
            "{} sacrifices {} {}.",
            describe_player_filter(&sacrifice.player, tagged_subjects),
            describe_value(&sacrifice.count, tagged_subjects),
            pluralize_noun_phrase(&sacrifice.filter.description())
        ));
    }
    if let Some(sacrifice_target) = effect.downcast_ref::<crate::effects::SacrificeTargetEffect>() {
        return Some(format!(
            "Sacrifice {}.",
            describe_choose_spec(&sacrifice_target.target, tagged_subjects)
        ));
    }
    if let Some(exile_from_hand) =
        effect.downcast_ref::<crate::effects::ExileFromHandAsCostEffect>()
    {
        let mut text = format!("Exile {} card(s) from hand", exile_from_hand.count);
        if let Some(colors) = exile_from_hand.color_filter {
            text.push_str(&format!(" with colors {colors:?}"));
        }
        text.push('.');
        return Some(text);
    }
    if let Some(add_mana) = effect.downcast_ref::<crate::effects::AddManaEffect>() {
        let mana = add_mana
            .mana
            .iter()
            .map(|symbol| mana_symbol_to_oracle(*symbol))
            .collect::<Vec<_>>()
            .join("");
        return Some(format!(
            "Add {} to {} mana pool.",
            if mana.is_empty() {
                "{0}".to_string()
            } else {
                mana
            },
            describe_player_filter(&add_mana.player, tagged_subjects)
        ));
    }
    if let Some(add_scaled) = effect.downcast_ref::<crate::effects::AddScaledManaEffect>() {
        let mana = add_scaled
            .mana
            .iter()
            .map(|symbol| mana_symbol_to_oracle(*symbol))
            .collect::<Vec<_>>()
            .join("");
        let mana_text = if mana.is_empty() {
            "{0}"
        } else {
            mana.as_str()
        };
        let player = describe_player_filter(&add_scaled.player, tagged_subjects);
        let mana_pool = if player == "You" {
            "your mana pool".to_string()
        } else {
            format!("{player}'s mana pool")
        };
        if let crate::effect::Value::Count(filter) = &add_scaled.amount {
            return Some(format!(
                "Add {} for each {} to {}.",
                mana_text,
                filter.description(),
                mana_pool
            ));
        }
        return Some(format!(
            "Add {} {} time(s) to {}.",
            mana_text,
            describe_value(&add_scaled.amount, tagged_subjects),
            mana_pool
        ));
    }
    if let Some(add_colorless) = effect.downcast_ref::<crate::effects::AddColorlessManaEffect>() {
        return Some(format!(
            "Add {} colorless mana to {} mana pool.",
            describe_value(&add_colorless.amount, tagged_subjects),
            describe_player_filter(&add_colorless.player, tagged_subjects)
        ));
    }
    if let Some(add_any_color) = effect.downcast_ref::<crate::effects::AddManaOfAnyColorEffect>() {
        return Some(format!(
            "Add {} mana of any color to {} mana pool.",
            describe_value(&add_any_color.amount, tagged_subjects),
            describe_player_filter(&add_any_color.player, tagged_subjects)
        ));
    }
    if let Some(add_any_one_color) =
        effect.downcast_ref::<crate::effects::AddManaOfAnyOneColorEffect>()
    {
        return Some(format!(
            "Add {} mana of any one color to {} mana pool.",
            describe_value(&add_any_one_color.amount, tagged_subjects),
            describe_player_filter(&add_any_one_color.player, tagged_subjects)
        ));
    }
    if let Some(add_land_produced) =
        effect.downcast_ref::<crate::effects::AddManaOfLandProducedTypesEffect>()
    {
        let any_word = if add_land_produced.allow_colorless {
            "type"
        } else {
            "color"
        };
        let one_word = if add_land_produced.same_type {
            " one"
        } else {
            ""
        };
        return Some(format!(
            "Add {} mana of any{} {} to {} mana pool that {} could produce.",
            describe_value(&add_land_produced.amount, tagged_subjects),
            one_word,
            any_word,
            describe_player_filter(&add_land_produced.player, tagged_subjects),
            add_land_produced.land_filter.description()
        ));
    }
    if let Some(add_commander_color) =
        effect.downcast_ref::<crate::effects::AddManaFromCommanderColorIdentityEffect>()
    {
        return Some(format!(
            "Add {} mana of any commander-identity color to {} mana pool.",
            describe_value(&add_commander_color.amount, tagged_subjects),
            describe_player_filter(&add_commander_color.player, tagged_subjects)
        ));
    }
    if effect
        .downcast_ref::<crate::effects::mana::AddManaOfImprintedColorsEffect>()
        .is_some()
    {
        return Some("Add one mana of any imprinted card color.".to_string());
    }
    if let Some(modify_pt) = effect.downcast_ref::<crate::effects::ModifyPowerToughnessEffect>() {
        return Some(format!(
            "{} gets {}/{} {}.",
            describe_choose_spec(&modify_pt.target, tagged_subjects),
            describe_signed_value(&modify_pt.power, tagged_subjects),
            describe_signed_value(&modify_pt.toughness, tagged_subjects),
            describe_until(&modify_pt.duration, tagged_subjects)
        ));
    }
    if let Some(set_base_pt) = effect.downcast_ref::<crate::effects::SetBasePowerToughnessEffect>()
    {
        return Some(format!(
            "{} has base power and toughness {}/{} {}.",
            describe_choose_spec(&set_base_pt.target, tagged_subjects),
            describe_value(&set_base_pt.power, tagged_subjects),
            describe_value(&set_base_pt.toughness, tagged_subjects),
            describe_until(&set_base_pt.duration, tagged_subjects)
        ));
    }
    if let Some(modify_pt_all) =
        effect.downcast_ref::<crate::effects::ModifyPowerToughnessAllEffect>()
    {
        return Some(format!(
            "{} get {}/{} {}.",
            pluralize_noun_phrase(&modify_pt_all.filter.description()),
            describe_signed_value(&modify_pt_all.power, tagged_subjects),
            describe_signed_value(&modify_pt_all.toughness, tagged_subjects),
            describe_until(&modify_pt_all.duration, tagged_subjects)
        ));
    }
    if let Some(modify_pt_each) =
        effect.downcast_ref::<crate::effects::ModifyPowerToughnessForEachEffect>()
    {
        return Some(format!(
            "{} gets +{} / +{} for each {} {}.",
            describe_choose_spec(&modify_pt_each.target, tagged_subjects),
            modify_pt_each.power_per,
            modify_pt_each.toughness_per,
            describe_value(&modify_pt_each.count, tagged_subjects),
            describe_until(&modify_pt_each.duration, tagged_subjects)
        ));
    }
    if let Some(grant_all) = effect.downcast_ref::<crate::effects::GrantAbilitiesAllEffect>() {
        return Some(format!(
            "Grant {} to {} {}.",
            describe_static_abilities(&grant_all.abilities),
            grant_all.filter.description(),
            describe_until(&grant_all.duration, tagged_subjects)
        ));
    }
    if let Some(grant_target) = effect.downcast_ref::<crate::effects::GrantAbilitiesTargetEffect>()
    {
        return Some(format!(
            "Grant {} to {} {}.",
            describe_static_abilities(&grant_target.abilities),
            describe_choose_spec(&grant_target.target, tagged_subjects),
            describe_until(&grant_target.duration, tagged_subjects)
        ));
    }
    if let Some(enter_attacking) = effect.downcast_ref::<crate::effects::EnterAttackingEffect>() {
        return Some(format!(
            "{} enters the battlefield attacking.",
            describe_choose_spec(&enter_attacking.target, tagged_subjects)
        ));
    }
    if let Some(tap) = effect.downcast_ref::<crate::effects::TapEffect>() {
        return Some(format!(
            "Tap {}.",
            describe_choose_spec(&tap.spec, tagged_subjects)
        ));
    }
    if let Some(untap) = effect.downcast_ref::<crate::effects::UntapEffect>() {
        return Some(format!(
            "Untap {}.",
            describe_choose_spec(&untap.spec, tagged_subjects)
        ));
    }
    if let Some(regenerate) = effect.downcast_ref::<crate::effects::RegenerateEffect>() {
        return Some(format!(
            "Regenerate {} {}.",
            describe_choose_spec(&regenerate.target, tagged_subjects),
            describe_until(&regenerate.duration, tagged_subjects)
        ));
    }
    if let Some(transform) = effect.downcast_ref::<crate::effects::TransformEffect>() {
        return Some(format!(
            "Transform {}.",
            describe_choose_spec(&transform.target, tagged_subjects)
        ));
    }
    if let Some(grant_object_ability) =
        effect.downcast_ref::<crate::effects::GrantObjectAbilityEffect>()
    {
        return Some(format!(
            "Grant ability '{}' to {}{}.",
            grant_object_ability
                .ability
                .text
                .clone()
                .unwrap_or_else(|| format!("{:?}", grant_object_ability.ability.kind)),
            describe_choose_spec(&grant_object_ability.target, tagged_subjects),
            if grant_object_ability.allow_duplicates {
                " (duplicates allowed)"
            } else {
                ""
            }
        ));
    }
    if let Some(attach_to) = effect.downcast_ref::<crate::effects::AttachToEffect>() {
        return Some(format!(
            "Attach this source to {}.",
            describe_choose_spec(&attach_to.target, tagged_subjects)
        ));
    }
    if let Some(earthbend) = effect.downcast_ref::<crate::effects::EarthbendEffect>() {
        return Some(format!(
            "Put {} charge counter(s) on {}.",
            earthbend.counters,
            describe_choose_spec(&earthbend.target, tagged_subjects)
        ));
    }
    if let Some(monstrosity) = effect.downcast_ref::<crate::effects::MonstrosityEffect>() {
        return Some(format!(
            "This source becomes monstrous with {} +1/+1 counter(s).",
            describe_value(&monstrosity.n, tagged_subjects)
        ));
    }
    if let Some(create_token) = effect.downcast_ref::<crate::effects::CreateTokenEffect>() {
        let token_blueprint = describe_token_blueprint(&create_token.token);
        let mut text = format!(
            "Create {} {} under {} control",
            describe_value(&create_token.count, tagged_subjects),
            token_blueprint,
            describe_player_filter(&create_token.controller, tagged_subjects)
        );
        if create_token.enters_tapped {
            text.push_str(", tapped");
        }
        if create_token.enters_attacking {
            text.push_str(", attacking");
        }
        if create_token.exile_at_end_of_combat {
            text.push_str(", and exile them at end of combat");
        }
        text.push('.');
        return Some(text);
    }
    if let Some(create_copy) = effect.downcast_ref::<crate::effects::CreateTokenCopyEffect>() {
        let mut text = format!(
            "Create {} token copy/copies of {} under {} control",
            describe_value(&create_copy.count, tagged_subjects),
            describe_choose_spec(&create_copy.target, tagged_subjects),
            describe_player_filter(&create_copy.controller, tagged_subjects)
        );
        if create_copy.enters_tapped {
            text.push_str(", tapped");
        }
        if create_copy.enters_attacking {
            text.push_str(", attacking");
        }
        if create_copy.has_haste {
            text.push_str(", with haste");
        }
        if create_copy.exile_at_end_of_combat {
            text.push_str(", and exile them at end of combat");
        }
        if create_copy.sacrifice_at_next_end_step {
            text.push_str(", and sacrifice it at the beginning of the next end step");
        }
        if let Some(adjustment) = &create_copy.pt_adjustment {
            text.push_str(&format!(", with P/T adjustment {adjustment:?}"));
        }
        text.push('.');
        return Some(text);
    }
    if let Some(investigate) = effect.downcast_ref::<crate::effects::InvestigateEffect>() {
        return Some(format!(
            "Investigate {} time(s).",
            describe_value(&investigate.count, tagged_subjects)
        ));
    }
    if let Some(gain_control) = effect.downcast_ref::<crate::effects::GainControlEffect>() {
        return Some(format!(
            "Gain control of {} {}.",
            describe_choose_spec(&gain_control.target, tagged_subjects),
            describe_until(&gain_control.duration, tagged_subjects)
        ));
    }
    if let Some(exchange_control) = effect.downcast_ref::<crate::effects::ExchangeControlEffect>() {
        return Some(format!(
            "Exchange control of {} and {}.",
            describe_choose_spec(&exchange_control.permanent1, tagged_subjects),
            describe_choose_spec(&exchange_control.permanent2, tagged_subjects)
        ));
    }
    if let Some(put_counters) = effect.downcast_ref::<crate::effects::PutCountersEffect>() {
        return Some(format!(
            "Put {} {:?} counter(s) on {}.",
            describe_value(&put_counters.count, tagged_subjects),
            put_counters.counter_type,
            describe_choose_spec(&put_counters.target, tagged_subjects)
        ));
    }
    if let Some(remove_counters) = effect.downcast_ref::<crate::effects::RemoveCountersEffect>() {
        return Some(format!(
            "Remove {} {:?} counter(s) from {}.",
            describe_value(&remove_counters.count, tagged_subjects),
            remove_counters.counter_type,
            describe_choose_spec(&remove_counters.target, tagged_subjects)
        ));
    }
    if let Some(remove_up_to) = effect.downcast_ref::<crate::effects::RemoveUpToCountersEffect>() {
        return Some(format!(
            "Remove up to {} {:?} counter(s) from {}.",
            describe_value(&remove_up_to.max_count, tagged_subjects),
            remove_up_to.counter_type,
            describe_choose_spec(&remove_up_to.target, tagged_subjects)
        ));
    }
    if let Some(remove_up_to_any) =
        effect.downcast_ref::<crate::effects::RemoveUpToAnyCountersEffect>()
    {
        return Some(format!(
            "Remove up to {} counters from {}.",
            describe_value(&remove_up_to_any.max_count, tagged_subjects),
            describe_choose_spec(&remove_up_to_any.target, tagged_subjects)
        ));
    }
    if let Some(move_counters) = effect.downcast_ref::<crate::effects::MoveCountersEffect>() {
        return Some(format!(
            "Move {} {:?} counter(s) from {} to {}.",
            describe_value(&move_counters.count, tagged_subjects),
            move_counters.counter_type,
            describe_choose_spec(&move_counters.from, tagged_subjects),
            describe_choose_spec(&move_counters.to, tagged_subjects)
        ));
    }
    if let Some(move_all_counters) = effect.downcast_ref::<crate::effects::MoveAllCountersEffect>()
    {
        return Some(format!(
            "Move all counters from {} to {}.",
            describe_choose_spec(&move_all_counters.from, tagged_subjects),
            describe_choose_spec(&move_all_counters.to, tagged_subjects)
        ));
    }
    if effect
        .downcast_ref::<crate::effects::ProliferateEffect>()
        .is_some()
    {
        return Some("Proliferate.".to_string());
    }
    if let Some(apply_continuous) = effect.downcast_ref::<crate::effects::ApplyContinuousEffect>() {
        return Some(format!(
            "Apply continuous effect to {:?}: {:?} {}.",
            apply_continuous.target,
            apply_continuous.modification,
            describe_until(&apply_continuous.until, tagged_subjects)
        ));
    }
    if let Some(apply_replacement) = effect.downcast_ref::<crate::effects::ApplyReplacementEffect>()
    {
        return Some(format!(
            "Register replacement effect ({:?}) as {:?}.",
            apply_replacement.effect, apply_replacement.mode
        ));
    }
    if let Some(cant) = effect.downcast_ref::<crate::effects::CantEffect>() {
        return Some(format!(
            "{} {}.",
            describe_restriction(&cant.restriction, tagged_subjects),
            describe_until(&cant.duration, tagged_subjects)
        ));
    }
    if let Some(control_player) = effect.downcast_ref::<crate::effects::ControlPlayerEffect>() {
        return Some(format!(
            "{} controls {} (start: {:?}, duration: {:?}).",
            describe_player_filter(&crate::target::PlayerFilter::You, tagged_subjects),
            describe_player_filter(&control_player.player, tagged_subjects),
            control_player.start,
            control_player.duration
        ));
    }
    if let Some(grant_effect) = effect.downcast_ref::<crate::effects::GrantEffect>() {
        return Some(format!(
            "Grant {} to {} ({:?}).",
            grant_effect.grantable.display(),
            describe_choose_spec(&grant_effect.target, tagged_subjects),
            grant_effect.duration
        ));
    }
    if let Some(grant_play) = effect.downcast_ref::<crate::effects::GrantPlayFromGraveyardEffect>()
    {
        return Some(format!(
            "Allow {} to play cards from graveyard.",
            describe_player_filter(&grant_play.player, tagged_subjects)
        ));
    }
    if let Some(create_emblem) = effect.downcast_ref::<crate::effects::CreateEmblemEffect>() {
        return Some(format!("Create emblem '{}'.", create_emblem.emblem.name));
    }
    if let Some(lose_game) = effect.downcast_ref::<crate::effects::LoseTheGameEffect>() {
        return Some(format!(
            "{} loses the game.",
            describe_player_filter(&lose_game.player, tagged_subjects)
        ));
    }
    if let Some(win_game) = effect.downcast_ref::<crate::effects::WinTheGameEffect>() {
        return Some(format!(
            "{} wins the game.",
            describe_player_filter(&win_game.player, tagged_subjects)
        ));
    }
    if let Some(poison) = effect.downcast_ref::<crate::effects::PoisonCountersEffect>() {
        return Some(format!(
            "Give {} {} poison counter(s).",
            describe_player_filter(&poison.player, tagged_subjects),
            describe_value(&poison.count, tagged_subjects)
        ));
    }
    if let Some(energy) = effect.downcast_ref::<crate::effects::EnergyCountersEffect>() {
        return Some(format!(
            "Give {} {} energy counter(s).",
            describe_player_filter(&energy.player, tagged_subjects),
            describe_value(&energy.count, tagged_subjects)
        ));
    }
    if let Some(experience) = effect.downcast_ref::<crate::effects::ExperienceCountersEffect>() {
        return Some(format!(
            "Give {} {} experience counter(s).",
            describe_player_filter(&experience.player, tagged_subjects),
            describe_value(&experience.count, tagged_subjects)
        ));
    }
    if let Some(skip_turn) = effect.downcast_ref::<crate::effects::SkipTurnEffect>() {
        return Some(format!(
            "{} skips their next turn.",
            describe_player_filter(&skip_turn.player, tagged_subjects)
        ));
    }
    if let Some(skip_draw) = effect.downcast_ref::<crate::effects::SkipDrawStepEffect>() {
        return Some(format!(
            "{} skips their next draw step.",
            describe_player_filter(&skip_draw.player, tagged_subjects)
        ));
    }
    if let Some(extra_turn) = effect.downcast_ref::<crate::effects::ExtraTurnEffect>() {
        return Some(format!(
            "{} takes an extra turn after this one.",
            describe_player_filter(&extra_turn.player, tagged_subjects)
        ));
    }
    if let Some(exile_instead) =
        effect.downcast_ref::<crate::effects::ExileInsteadOfGraveyardEffect>()
    {
        return Some(format!(
            "If {} cards would go to graveyard this turn, exile them instead.",
            describe_player_filter(&exile_instead.player, tagged_subjects)
        ));
    }
    if let Some(schedule_delayed) =
        effect.downcast_ref::<crate::effects::ScheduleDelayedTriggerEffect>()
    {
        return Some(format!(
            "Schedule delayed trigger {:?} for {} with effects: {}.",
            schedule_delayed.trigger,
            describe_player_filter(&schedule_delayed.controller, tagged_subjects),
            describe_effects_inline(&schedule_delayed.effects, tagged_subjects)
        ));
    }

    None
}

fn describe_move_to_zone(
    effect: &crate::effects::MoveToZoneEffect,
    tagged_subjects: &HashMap<String, String>,
) -> String {
    let target = describe_choose_spec(&effect.target, tagged_subjects);
    match effect.zone {
        Zone::Exile => format!("Exile {target}."),
        Zone::Graveyard => format!("Put {target} into its owner's graveyard."),
        Zone::Hand => format!("Return {target} to its owner's hand."),
        Zone::Library => {
            if effect.to_top {
                format!("Put {target} on top of its owner's library.")
            } else {
                format!("Put {target} on the bottom of its owner's library.")
            }
        }
        Zone::Battlefield => {
            if let crate::target::ChooseSpec::Tagged(tag) = &effect.target
                && tag.as_str().starts_with("exiled_")
            {
                format!("Return {target} to the battlefield.")
            } else {
                format!("Put {target} onto the battlefield.")
            }
        }
        Zone::Stack => format!("Put {target} on the stack."),
        Zone::Command => format!("Move {target} to the command zone."),
    }
}

fn describe_gain_life(
    effect: &crate::effects::GainLifeEffect,
    tagged_subjects: &HashMap<String, String>,
) -> String {
    let player = describe_gain_life_player(&effect.player, tagged_subjects);
    let verb = if player == "You" { "gain" } else { "gains" };
    if let Some(equal_clause) = describe_value_equal_clause(&effect.amount, tagged_subjects) {
        format!("{player} {verb} life {equal_clause}.")
    } else {
        format!(
            "{player} {verb} {} life.",
            describe_value(&effect.amount, tagged_subjects)
        )
    }
}

fn describe_gain_life_player(
    spec: &crate::target::ChooseSpec,
    tagged_subjects: &HashMap<String, String>,
) -> String {
    match spec.base() {
        crate::target::ChooseSpec::Player(filter) => {
            describe_player_filter(filter, tagged_subjects)
        }
        _ => describe_choose_spec(spec, tagged_subjects),
    }
}

fn describe_player_filter(
    filter: &crate::target::PlayerFilter,
    tagged_subjects: &HashMap<String, String>,
) -> String {
    match filter {
        crate::target::PlayerFilter::You => "You".to_string(),
        crate::target::PlayerFilter::Opponent => "An opponent".to_string(),
        crate::target::PlayerFilter::Any => "A player".to_string(),
        crate::target::PlayerFilter::Target(inner) => {
            format!(
                "Target {}",
                strip_leading_article(&describe_player_filter(inner, tagged_subjects))
            )
        }
        crate::target::PlayerFilter::ControllerOf(crate::target::ObjectRef::Tagged(tag)) => {
            if tagged_subjects.contains_key(tag.as_str()) {
                "Its controller".to_string()
            } else {
                "That object's controller".to_string()
            }
        }
        crate::target::PlayerFilter::OwnerOf(crate::target::ObjectRef::Tagged(tag)) => {
            if tagged_subjects.contains_key(tag.as_str()) {
                "Its owner".to_string()
            } else {
                "That object's owner".to_string()
            }
        }
        crate::target::PlayerFilter::Specific(_) => "That player".to_string(),
        crate::target::PlayerFilter::Active => "The active player".to_string(),
        crate::target::PlayerFilter::Defending => "The defending player".to_string(),
        crate::target::PlayerFilter::Attacking => "The attacking player".to_string(),
        crate::target::PlayerFilter::DamagedPlayer => "That damaged player".to_string(),
        crate::target::PlayerFilter::Teammate => "A teammate".to_string(),
        crate::target::PlayerFilter::IteratedPlayer => "That player".to_string(),
        crate::target::PlayerFilter::ControllerOf(_) => "That object's controller".to_string(),
        crate::target::PlayerFilter::OwnerOf(_) => "That object's owner".to_string(),
    }
}

fn join_words_with_and(parts: &[String]) -> String {
    match parts.len() {
        0 => String::new(),
        1 => parts[0].clone(),
        2 => format!("{} and {}", parts[0], parts[1]),
        _ => {
            let mut text = parts[..parts.len() - 1].join(", ");
            text.push_str(", and ");
            text.push_str(parts.last().map(String::as_str).unwrap_or_default());
            text
        }
    }
}

fn describe_token_pt_value(value: crate::card::PtValue) -> String {
    match value {
        crate::card::PtValue::Fixed(n) => n.to_string(),
        crate::card::PtValue::Star => "*".to_string(),
        crate::card::PtValue::StarPlus(n) => format!("*+{n}"),
    }
}

fn describe_token_color_words(colors: crate::color::ColorSet, include_colorless: bool) -> String {
    if colors.is_empty() {
        return if include_colorless {
            "colorless".to_string()
        } else {
            String::new()
        };
    }

    let mut names = Vec::new();
    if colors.contains(crate::color::Color::White) {
        names.push("white".to_string());
    }
    if colors.contains(crate::color::Color::Blue) {
        names.push("blue".to_string());
    }
    if colors.contains(crate::color::Color::Black) {
        names.push("black".to_string());
    }
    if colors.contains(crate::color::Color::Red) {
        names.push("red".to_string());
    }
    if colors.contains(crate::color::Color::Green) {
        names.push("green".to_string());
    }
    join_words_with_and(&names)
}

fn describe_token_blueprint(token: &CardDefinition) -> String {
    let card = &token.card;
    let mut parts = Vec::new();

    if let Some(pt) = card.power_toughness {
        parts.push(format!(
            "{}/{}",
            describe_token_pt_value(pt.power),
            describe_token_pt_value(pt.toughness)
        ));
    }

    let colors = describe_token_color_words(card.colors(), card.is_creature());
    if !colors.is_empty() {
        parts.push(colors);
    }

    if !card.subtypes.is_empty() {
        parts.push(
            card.subtypes
                .iter()
                .map(|subtype| format!("{subtype:?}"))
                .collect::<Vec<_>>()
                .join(" "),
        );
    }

    if !card.card_types.is_empty() {
        parts.push(
            card.card_types
                .iter()
                .map(|card_type| format!("{card_type:?}").to_ascii_lowercase())
                .collect::<Vec<_>>()
                .join(" "),
        );
    }

    parts.push("token".to_string());

    let mut text = parts.join(" ");
    let mut keyword_texts = Vec::new();
    for ability in &token.abilities {
        if let AbilityKind::Static(static_ability) = &ability.kind
            && static_ability.is_keyword()
        {
            keyword_texts.push(static_ability.display().to_ascii_lowercase());
        }
    }
    keyword_texts.sort();
    keyword_texts.dedup();
    if !keyword_texts.is_empty() {
        text.push_str(" with ");
        text.push_str(&join_words_with_and(&keyword_texts));
    }

    text
}

fn describe_choose_spec(
    spec: &crate::target::ChooseSpec,
    tagged_subjects: &HashMap<String, String>,
) -> String {
    match spec {
        crate::target::ChooseSpec::Target(inner) => {
            let inner_text =
                strip_leading_article(&describe_choose_spec(inner, tagged_subjects)).to_string();
            if inner_text.starts_with("target ") {
                inner_text
            } else {
                format!("target {inner_text}")
            }
        }
        crate::target::ChooseSpec::Object(filter) => filter.description(),
        crate::target::ChooseSpec::Player(filter) => {
            strip_leading_article(&describe_player_filter(filter, tagged_subjects)).to_string()
        }
        crate::target::ChooseSpec::AnyTarget => "any target".to_string(),
        crate::target::ChooseSpec::Source => "this source".to_string(),
        crate::target::ChooseSpec::SourceController => "you".to_string(),
        crate::target::ChooseSpec::SourceOwner => "this source's owner".to_string(),
        crate::target::ChooseSpec::Tagged(tag) => tagged_subjects
            .get(tag.as_str())
            .map(|subject| format!("the tagged {subject}"))
            .unwrap_or_else(|| "the tagged object".to_string()),
        crate::target::ChooseSpec::All(filter) => {
            format!("all {}", pluralize_noun_phrase(&filter.description()))
        }
        crate::target::ChooseSpec::EachPlayer(filter) => {
            format!(
                "each {}",
                pluralize_noun_phrase(&describe_player_filter(filter, tagged_subjects))
            )
        }
        crate::target::ChooseSpec::SpecificObject(_) => "that object".to_string(),
        crate::target::ChooseSpec::SpecificPlayer(_) => "that player".to_string(),
        crate::target::ChooseSpec::Iterated => "that object".to_string(),
        crate::target::ChooseSpec::WithCount(inner, count) => {
            let inner_text = describe_choose_spec(inner, tagged_subjects);
            if count.is_single() {
                inner_text
            } else {
                let count_text = describe_choice_count(count);
                format!("{count_text} {inner_text}")
            }
        }
    }
}

fn describe_value_equal_clause(
    value: &crate::effect::Value,
    _tagged_subjects: &HashMap<String, String>,
) -> Option<String> {
    match value {
        crate::effect::Value::PowerOf(spec) => {
            if matches!(spec.base(), crate::target::ChooseSpec::Tagged(_)) {
                Some("equal to its power".to_string())
            } else {
                Some("equal to that creature's power".to_string())
            }
        }
        crate::effect::Value::ToughnessOf(spec) => {
            if matches!(spec.base(), crate::target::ChooseSpec::Tagged(_)) {
                Some("equal to its toughness".to_string())
            } else {
                Some("equal to that creature's toughness".to_string())
            }
        }
        _ => None,
    }
}

fn describe_value(
    value: &crate::effect::Value,
    tagged_subjects: &HashMap<String, String>,
) -> String {
    match value {
        crate::effect::Value::Fixed(n) => n.to_string(),
        crate::effect::Value::X => "X".to_string(),
        crate::effect::Value::XTimes(n) => format!("{n}*X"),
        crate::effect::Value::Count(filter) => {
            format!(
                "the number of {}",
                pluralize_noun_phrase(&filter.description())
            )
        }
        crate::effect::Value::CountPlayers(filter) => format!(
            "the number of {}",
            pluralize_noun_phrase(&describe_player_filter(filter, tagged_subjects))
        ),
        crate::effect::Value::SourcePower => "this source's power".to_string(),
        crate::effect::Value::SourceToughness => "this source's toughness".to_string(),
        crate::effect::Value::PowerOf(spec) => {
            format!(
                "the power of {}",
                describe_choose_spec(spec, tagged_subjects)
            )
        }
        crate::effect::Value::ToughnessOf(spec) => {
            format!(
                "the toughness of {}",
                describe_choose_spec(spec, tagged_subjects)
            )
        }
        crate::effect::Value::LifeTotal(_) => "that player's life total".to_string(),
        crate::effect::Value::CardsInHand(_) => "the number of cards in hand".to_string(),
        crate::effect::Value::CardsInGraveyard(_) => "the number of cards in graveyard".to_string(),
        crate::effect::Value::SpellsCastThisTurn(_) => "spells cast this turn".to_string(),
        crate::effect::Value::SpellsCastBeforeThisTurn(_) => "spells cast before this".to_string(),
        crate::effect::Value::CardTypesInGraveyard(_) => {
            "the number of card types in graveyard".to_string()
        }
        crate::effect::Value::EffectValue(_) => "a prior effect value".to_string(),
        crate::effect::Value::WasKicked => "1 if kicked, else 0".to_string(),
        crate::effect::Value::WasBoughtBack => "1 if buyback was paid, else 0".to_string(),
        crate::effect::Value::WasEntwined => "1 if entwined, else 0".to_string(),
        crate::effect::Value::WasPaid(idx) => format!("1 if optional cost #{idx} was paid, else 0"),
        crate::effect::Value::WasPaidLabel(label) => {
            format!("1 if optional cost '{label}' was paid, else 0")
        }
        crate::effect::Value::TimesPaid(idx) => format!("times optional cost #{idx} was paid"),
        crate::effect::Value::TimesPaidLabel(label) => {
            format!("times optional cost '{label}' was paid")
        }
        crate::effect::Value::KickCount => "times kicked".to_string(),
        crate::effect::Value::CountersOnSource(kind) => {
            format!("{kind:?} counters on source")
        }
        crate::effect::Value::CountersOn(spec, Some(kind)) => {
            format!(
                "the number of {kind:?} counters on {}",
                describe_choose_spec(spec, tagged_subjects)
            )
        }
        crate::effect::Value::CountersOn(spec, None) => {
            format!(
                "the number of counters on {}",
                describe_choose_spec(spec, tagged_subjects)
            )
        }
        crate::effect::Value::TaggedCount => "the tagged count".to_string(),
    }
}

fn describe_signed_value(
    value: &crate::effect::Value,
    tagged_subjects: &HashMap<String, String>,
) -> String {
    match value {
        crate::effect::Value::Fixed(n) if *n > 0 => format!("+{n}"),
        crate::effect::Value::Fixed(n) => n.to_string(),
        _ => describe_value(value, tagged_subjects),
    }
}

fn strip_leading_article(text: &str) -> &str {
    text.strip_prefix("a ")
        .or_else(|| text.strip_prefix("an "))
        .or_else(|| text.strip_prefix("the "))
        .unwrap_or(text)
}

fn pluralize_noun_phrase(text: &str) -> String {
    let normalized = strip_leading_article(text).trim();
    if normalized.is_empty() {
        return "objects".to_string();
    }
    if normalized.ends_with('s') {
        normalized.to_string()
    } else {
        format!("{normalized}s")
    }
}

fn mana_symbol_to_oracle(symbol: ManaSymbol) -> String {
    match symbol {
        ManaSymbol::White => "{W}".to_string(),
        ManaSymbol::Blue => "{U}".to_string(),
        ManaSymbol::Black => "{B}".to_string(),
        ManaSymbol::Red => "{R}".to_string(),
        ManaSymbol::Green => "{G}".to_string(),
        ManaSymbol::Colorless => "{C}".to_string(),
        ManaSymbol::Generic(value) => format!("{{{value}}}"),
        ManaSymbol::Snow => "{S}".to_string(),
        ManaSymbol::Life(_) => "{P}".to_string(),
        ManaSymbol::X => "{X}".to_string(),
    }
}

fn format_type_line(obj: &crate::object::Object) -> String {
    let mut left = Vec::new();
    left.extend(obj.supertypes.iter().map(|value| format!("{value:?}")));
    left.extend(obj.card_types.iter().map(|value| format!("{value:?}")));

    let mut type_line = left.join(" ");
    if !obj.subtypes.is_empty() {
        let subtypes = obj
            .subtypes
            .iter()
            .map(|value| format!("{value:?}"))
            .collect::<Vec<_>>()
            .join(" ");
        if type_line.is_empty() {
            type_line = subtypes;
        } else {
            type_line.push_str(" - ");
            type_line.push_str(&subtypes);
        }
    }

    if type_line.is_empty() {
        "Object".to_string()
    } else {
        type_line
    }
}

fn split_camel_case(value: &str) -> String {
    let mut output = String::with_capacity(value.len() + 8);
    let mut previous_lower = false;
    for ch in value.chars() {
        if ch.is_ascii_uppercase() && previous_lower {
            output.push(' ');
        }
        previous_lower = ch.is_ascii_lowercase() || ch.is_ascii_digit();
        output.push(ch);
    }
    output
}

fn build_action_view(game: &GameState, index: usize, action: &LegalAction) -> ActionView {
    let (kind, object_id, from_zone, to_zone) = action_drag_metadata(action);
    ActionView {
        index,
        label: describe_action(game, action),
        kind: kind.to_string(),
        object_id,
        from_zone,
        to_zone,
    }
}

fn action_drag_metadata(
    action: &LegalAction,
) -> (&'static str, Option<u64>, Option<String>, Option<String>) {
    match action {
        LegalAction::PassPriority => ("pass_priority", None, None, None),
        LegalAction::PlayLand { land_id } => (
            "play_land",
            Some(land_id.0),
            Some(zone_name(Zone::Hand)),
            Some(zone_name(Zone::Battlefield)),
        ),
        LegalAction::CastSpell {
            spell_id,
            from_zone,
            ..
        } => (
            "cast_spell",
            Some(spell_id.0),
            Some(zone_name(*from_zone)),
            Some(zone_name(Zone::Stack)),
        ),
        LegalAction::ActivateAbility { source, .. } => (
            "activate_ability",
            Some(source.0),
            Some(zone_name(Zone::Battlefield)),
            Some(zone_name(Zone::Stack)),
        ),
        LegalAction::ActivateManaAbility { source, .. } => (
            "activate_mana_ability",
            Some(source.0),
            Some(zone_name(Zone::Battlefield)),
            None,
        ),
        LegalAction::TurnFaceUp { creature_id } => (
            "turn_face_up",
            Some(creature_id.0),
            Some(zone_name(Zone::Battlefield)),
            Some(zone_name(Zone::Battlefield)),
        ),
        LegalAction::SpecialAction(_) => ("special_action", None, None, None),
    }
}

fn zone_name(zone: Zone) -> String {
    match zone {
        Zone::Library => "library",
        Zone::Hand => "hand",
        Zone::Battlefield => "battlefield",
        Zone::Graveyard => "graveyard",
        Zone::Exile => "exile",
        Zone::Stack => "stack",
        Zone::Command => "command",
    }
    .to_string()
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

fn attack_target_view(game: &GameState, target: &AttackTarget) -> AttackTargetView {
    match target {
        AttackTarget::Player(pid) => AttackTargetView::Player {
            player: pid.0,
            name: game
                .player(*pid)
                .map(|p| p.name.clone())
                .unwrap_or_else(|| format!("Player {}", pid.0 + 1)),
        },
        AttackTarget::Planeswalker(id) => AttackTargetView::Planeswalker {
            object: id.0,
            name: object_name(game, *id),
        },
    }
}

fn attack_target_from_input(input: &AttackTargetInput) -> AttackTarget {
    match input {
        AttackTargetInput::Player { player } => AttackTarget::Player(PlayerId::from_index(*player)),
        AttackTargetInput::Planeswalker { object } => {
            AttackTarget::Planeswalker(ObjectId::from_raw(*object))
        }
    }
}

fn validate_attacker_declarations(
    attackers: &crate::decisions::context::AttackersContext,
    declarations: &[AttackerDeclarationInput],
) -> Result<Vec<AttackerDeclaration>, JsValue> {
    let options: HashMap<u64, &crate::decisions::context::AttackerOptionContext> = attackers
        .attacker_options
        .iter()
        .map(|option| (option.creature.0, option))
        .collect();
    let mut declared_creatures = HashSet::new();
    let mut converted = Vec::new();

    for declaration in declarations {
        let Some(option) = options.get(&declaration.creature) else {
            return Err(JsValue::from_str(&format!(
                "invalid attacker creature id: {}",
                declaration.creature
            )));
        };
        if !declared_creatures.insert(declaration.creature) {
            return Err(JsValue::from_str(&format!(
                "attacker declared twice: {}",
                declaration.creature
            )));
        }

        let target = attack_target_from_input(&declaration.target);
        if !option.valid_targets.contains(&target) {
            return Err(JsValue::from_str(&format!(
                "invalid attack target for creature {}",
                declaration.creature
            )));
        }

        converted.push(AttackerDeclaration {
            creature: ObjectId::from_raw(declaration.creature),
            target,
        });
    }

    for option in &attackers.attacker_options {
        if option.must_attack && !declared_creatures.contains(&option.creature.0) {
            return Err(JsValue::from_str(&format!(
                "{} must attack if able",
                option.creature_name
            )));
        }
    }

    Ok(converted)
}

fn validate_blocker_declarations(
    blockers: &crate::decisions::context::BlockersContext,
    declarations: &[BlockerDeclarationInput],
) -> Result<Vec<BlockerDeclaration>, JsValue> {
    let options: HashMap<u64, &crate::decisions::context::BlockerOptionContext> = blockers
        .blocker_options
        .iter()
        .map(|option| (option.attacker.0, option))
        .collect();
    let mut blockers_used = HashSet::new();
    let mut counts_by_attacker: HashMap<u64, usize> = HashMap::new();
    let mut converted = Vec::new();

    for declaration in declarations {
        let Some(option) = options.get(&declaration.blocking) else {
            return Err(JsValue::from_str(&format!(
                "invalid blocking attacker id: {}",
                declaration.blocking
            )));
        };
        if !option
            .valid_blockers
            .iter()
            .any(|(id, _)| id.0 == declaration.blocker)
        {
            return Err(JsValue::from_str(&format!(
                "invalid blocker {} for attacker {}",
                declaration.blocker, declaration.blocking
            )));
        }
        if !blockers_used.insert(declaration.blocker) {
            return Err(JsValue::from_str(&format!(
                "blocker {} assigned more than once",
                declaration.blocker
            )));
        }
        *counts_by_attacker.entry(declaration.blocking).or_insert(0) += 1;
        converted.push(BlockerDeclaration {
            blocker: ObjectId::from_raw(declaration.blocker),
            blocking: ObjectId::from_raw(declaration.blocking),
        });
    }

    for option in &blockers.blocker_options {
        let assigned = counts_by_attacker
            .get(&option.attacker.0)
            .copied()
            .unwrap_or(0);
        // "Minimum blockers" applies only when the attacker is blocked at all.
        // Example: menace means if blocked, it must be by 2+, but not blocked is legal.
        if assigned > 0 && assigned < option.min_blockers {
            return Err(JsValue::from_str(&format!(
                "{} requires at least {} blocker(s)",
                option.attacker_name, option.min_blockers
            )));
        }
    }

    Ok(converted)
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
