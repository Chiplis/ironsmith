//! Integration test framework for simulating gameplay.
//!
//! This module provides a way to write tests that simulate actual gameplay
//! through a series of user actions, similar to `cargo run` but automated.
//!
//! # Example
//!
//! ```ignore
//! let result = GameScript::new()
//!     .player("Alice", &["Forest", "Llanowar Elves"])
//!     .player("Bob", &["Mountain", "Lightning Bolt"])
//!     .action(Action::PlayLand("Forest"))
//!     .action(Action::Pass)
//!     .action(Action::Pass) // Bob passes too
//!     .run();
//! ```

#![allow(dead_code)]

use crate::cards::CardRegistry;
use crate::combat_state::{AttackTarget, CombatState};
use crate::decision::{
    AttackerDeclaration, BlockerDeclaration, DecisionMaker, GameProgress, LegalAction,
};
use crate::game_loop::{
    apply_attacker_declarations, apply_blocker_declarations, check_and_apply_sbas,
    get_declare_attackers_decision, get_declare_blockers_decision, put_triggers_on_stack,
    run_priority_loop_with,
};
use crate::game_state::GameState;
use crate::game_state::{Phase, Step};
use crate::ids::{ObjectId, PlayerId};
use crate::triggers::TriggerQueue;
use crate::turn::{execute_draw_step, execute_untap_step};
use crate::zone::Zone;

/// A high-level action that can be scripted in tests.
#[derive(Debug, Clone)]
pub enum Action {
    /// Pass priority.
    Pass,

    /// Play a land by name from hand.
    PlayLand(&'static str),

    /// Cast a spell by name from hand.
    CastSpell(&'static str),

    /// Cast a spell with specific targets.
    CastSpellTargeting {
        spell: &'static str,
        targets: Vec<TargetChoice>,
    },

    /// Activate a mana ability on a permanent by name.
    TapForMana(&'static str),

    /// Activate an ability on a permanent.
    ActivateAbility {
        source: &'static str,
        ability_index: usize,
    },

    /// Declare attackers (creature names attacking the opponent).
    DeclareAttackers(Vec<&'static str>),

    /// Declare blockers (blocker name, attacker name pairs).
    DeclareBlockers(Vec<(&'static str, &'static str)>),

    /// No attackers this combat.
    DeclareNoAttackers,

    /// No blockers this combat.
    DeclareNoBlockers,
}

/// A target choice for spells/abilities.
#[derive(Debug, Clone)]
pub enum TargetChoice {
    /// Target a player by name ("Alice", "Bob") or "Opponent".
    Player(&'static str),
    /// Target a permanent by name.
    Permanent(&'static str),
}

/// Result of running a scripted turn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScriptResult {
    /// Script is exhausted, stop running.
    Exhausted,
    /// Turn completed normally.
    TurnComplete,
}

/// Run a single turn with a scripted decision maker.
/// Returns Some(ScriptResult::Exhausted) if the script runs out during the turn.
fn run_scripted_turn(
    game: &mut GameState,
    combat: &mut CombatState,
    trigger_queue: &mut TriggerQueue,
    dm: &mut ScriptedGameDecisionMaker,
) -> Result<Option<ScriptResult>, ScriptError> {
    // Helper macro to run priority and check exhaustion
    macro_rules! run_priority {
        ($game:expr, $trigger_queue:expr, $dm:expr) => {{
            if $dm.is_exhausted() {
                return Ok(Some(ScriptResult::Exhausted));
            }
            match run_priority_loop_with($game, $trigger_queue, $dm) {
                Ok(GameProgress::GameOver(_)) => return Ok(Some(ScriptResult::TurnComplete)),
                Ok(_) => {}
                Err(e) => {
                    if $dm.is_exhausted() {
                        return Ok(Some(ScriptResult::Exhausted));
                    }
                    return Err(ScriptError::GameLoopError(format!("{:?}", e)));
                }
            }
            if $dm.is_exhausted() {
                return Ok(Some(ScriptResult::Exhausted));
            }
        }};
    }

    // === Beginning Phase ===
    game.turn.phase = Phase::Beginning;
    game.turn.step = Some(Step::Untap);
    execute_untap_step(game);
    game.empty_mana_pools();

    game.turn.step = Some(Step::Upkeep);
    game.turn.priority_player = Some(game.turn.active_player);
    run_priority!(game, trigger_queue, dm);
    game.empty_mana_pools();

    game.turn.step = Some(Step::Draw);
    let _ = execute_draw_step(game);
    game.turn.priority_player = Some(game.turn.active_player);
    run_priority!(game, trigger_queue, dm);
    game.empty_mana_pools();

    // === Precombat Main Phase ===
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.priority_player = Some(game.turn.active_player);
    run_priority!(game, trigger_queue, dm);
    game.empty_mana_pools();

    // === Combat Phase ===
    game.turn.phase = Phase::Combat;
    game.turn.step = Some(Step::BeginCombat);
    game.turn.priority_player = Some(game.turn.active_player);
    run_priority!(game, trigger_queue, dm);
    game.empty_mana_pools();

    // Declare attackers
    game.turn.step = Some(Step::DeclareAttackers);
    let attacker_ctx = get_declare_attackers_decision(game, combat);

    if dm.is_exhausted() {
        return Ok(Some(ScriptResult::Exhausted));
    }

    if let crate::decisions::context::DecisionContext::Attackers(ctx) = attacker_ctx {
        let result = dm.decide_attackers(game, &ctx);
        let declarations: Vec<_> = result
            .into_iter()
            .map(|d| AttackerDeclaration {
                creature: d.creature,
                target: d.target,
            })
            .collect();
        if let Err(e) = apply_attacker_declarations(game, combat, trigger_queue, &declarations) {
            return Err(ScriptError::GameLoopError(format!("{:?}", e)));
        }
    }

    let _ = put_triggers_on_stack(game, trigger_queue);
    run_priority!(game, trigger_queue, dm);
    game.empty_mana_pools();

    // Declare blockers (if there are attackers)
    if !combat.attackers.is_empty() {
        game.turn.step = Some(Step::DeclareBlockers);

        let defending_player = game
            .players
            .iter()
            .find(|p| p.id != game.turn.active_player && p.is_in_game())
            .map(|p| p.id)
            .unwrap_or(game.turn.active_player);

        game.turn.priority_player = Some(defending_player);

        let blocker_ctx = get_declare_blockers_decision(game, combat, defending_player);

        if dm.is_exhausted() {
            return Ok(Some(ScriptResult::Exhausted));
        }

        if let crate::decisions::context::DecisionContext::Blockers(ctx) = blocker_ctx {
            let result = dm.decide_blockers(game, &ctx);
            let declarations: Vec<_> = result
                .into_iter()
                .map(|d| BlockerDeclaration {
                    blocker: d.blocker,
                    blocking: d.blocking,
                })
                .collect();
            if let Err(e) = apply_blocker_declarations(
                game,
                combat,
                trigger_queue,
                &declarations,
                defending_player,
            ) {
                return Err(ScriptError::GameLoopError(format!("{:?}", e)));
            }
        }

        let _ = put_triggers_on_stack(game, trigger_queue);
        run_priority!(game, trigger_queue, dm);
        game.empty_mana_pools();

        // Combat damage
        game.turn.step = Some(Step::CombatDamage);
        crate::execute_combat_damage_step(game, combat, false);
        let _ = check_and_apply_sbas(game, trigger_queue);
        run_priority!(game, trigger_queue, dm);
        game.empty_mana_pools();
    }

    // End combat
    game.turn.step = Some(Step::EndCombat);
    *combat = CombatState::default();
    game.turn.priority_player = Some(game.turn.active_player);
    run_priority!(game, trigger_queue, dm);
    game.empty_mana_pools();

    // === Postcombat Main Phase ===
    game.turn.phase = Phase::NextMain;
    game.turn.step = None;
    game.turn.priority_player = Some(game.turn.active_player);
    run_priority!(game, trigger_queue, dm);
    game.empty_mana_pools();

    // === Ending Phase ===
    game.turn.phase = Phase::Ending;
    game.turn.step = Some(Step::End);
    game.turn.priority_player = Some(game.turn.active_player);
    run_priority!(game, trigger_queue, dm);
    game.empty_mana_pools();

    // Cleanup (no priority normally)
    game.turn.step = Some(Step::Cleanup);
    game.turn.priority_player = None;

    Ok(Some(ScriptResult::TurnComplete))
}

/// A scripted game setup and action sequence.
pub struct GameScript {
    /// Starting hands for each player (player name -> card names).
    hands: Vec<(&'static str, Vec<&'static str>)>,
    /// Actions to execute in order.
    actions: Vec<Action>,
    /// Starting life total.
    starting_life: i32,
}

impl GameScript {
    /// Create a new empty game script.
    pub fn new() -> Self {
        Self {
            hands: Vec::new(),
            actions: Vec::new(),
            starting_life: 20,
        }
    }

    /// Set a player's starting hand.
    pub fn player(mut self, name: &'static str, hand: &[&'static str]) -> Self {
        self.hands.push((name, hand.to_vec()));
        self
    }

    /// Add an action to the script.
    pub fn action(mut self, action: Action) -> Self {
        self.actions.push(action);
        self
    }

    /// Add multiple actions at once.
    pub fn actions(mut self, actions: impl IntoIterator<Item = Action>) -> Self {
        self.actions.extend(actions);
        self
    }

    /// Set starting life total.
    pub fn starting_life(mut self, life: i32) -> Self {
        self.starting_life = life;
        self
    }

    /// Run the game script and return the final game state.
    pub fn run(self) -> Result<GameState, ScriptError> {
        // Create the card registry
        let registry = CardRegistry::with_builtin_cards();

        // Create player names
        let player_names: Vec<String> = if self.hands.is_empty() {
            vec!["Alice".to_string(), "Bob".to_string()]
        } else {
            self.hands
                .iter()
                .map(|(name, _)| name.to_string())
                .collect()
        };

        // Create game state
        let mut game = GameState::new(player_names.clone(), self.starting_life);

        // Set up hands
        for (i, (_, hand)) in self.hands.iter().enumerate() {
            let player_id = PlayerId::from_index(i as u8);
            for card_name in hand {
                let def = registry
                    .get(card_name)
                    .ok_or_else(|| ScriptError::CardNotFound(card_name.to_string()))?;
                game.create_object_from_definition(def, player_id, Zone::Hand);
            }
        }

        // Create the scripted decision maker
        let mut dm = ScriptedGameDecisionMaker::new(&game, self.actions);

        // Run turns until the script is exhausted
        let mut trigger_queue = TriggerQueue::new();
        let mut combat = CombatState::default();
        let max_turns = 10; // Safety limit for tests

        for _turn in 0..max_turns {
            if dm.is_exhausted() {
                break;
            }

            // Run a full turn
            if let Some(result) =
                run_scripted_turn(&mut game, &mut combat, &mut trigger_queue, &mut dm)?
            {
                if result == ScriptResult::Exhausted {
                    break;
                }
            }

            // Switch active player for next turn
            let next_player = PlayerId::from_index(
                ((game.turn.active_player.index() + 1) % game.players.len()) as u8,
            );
            game.turn.active_player = next_player;
            game.turn.priority_player = Some(next_player);
            game.turn.turn_number += 1;

            if let Some(player) = game.player_mut(next_player) {
                player.begin_turn();
            }
        }

        Ok(game)
    }
}

impl Default for GameScript {
    fn default() -> Self {
        Self::new()
    }
}

/// Error type for script execution.
#[derive(Debug, Clone)]
pub enum ScriptError {
    /// A card name was not found in the registry.
    CardNotFound(String),
    /// A permanent was not found on the battlefield.
    PermanentNotFound(String),
    /// The action doesn't match the current decision.
    ActionMismatch { expected: String, action: String },
    /// Game loop error.
    GameLoopError(String),
}

/// A decision maker that executes scripted actions.
struct ScriptedGameDecisionMaker {
    actions: Vec<Action>,
    index: usize,
    player_names: Vec<String>,
}

impl ScriptedGameDecisionMaker {
    fn new(game: &GameState, actions: Vec<Action>) -> Self {
        let player_names = game.players.iter().map(|p| p.name.clone()).collect();
        Self {
            actions,
            index: 0,
            player_names,
        }
    }

    fn is_exhausted(&self) -> bool {
        self.index >= self.actions.len()
    }

    fn next_action(&mut self) -> Option<Action> {
        if self.index < self.actions.len() {
            let action = self.actions[self.index].clone();
            self.index += 1;
            Some(action)
        } else {
            None
        }
    }

    /// Peek at the next action without consuming it.
    fn peek_action(&self) -> Option<&Action> {
        self.actions.get(self.index)
    }

    /// Find an object on the battlefield by name.
    fn find_permanent(&self, game: &GameState, name: &str) -> Option<ObjectId> {
        game.battlefield
            .iter()
            .find(|&&id| game.object(id).map(|obj| obj.name == name).unwrap_or(false))
            .copied()
    }

    /// Find an object in a player's hand by name.
    fn find_in_hand(&self, game: &GameState, player: PlayerId, name: &str) -> Option<ObjectId> {
        game.player(player).and_then(|p| {
            p.hand
                .iter()
                .find(|&&id| game.object(id).map(|obj| obj.name == name).unwrap_or(false))
                .copied()
        })
    }

    /// Convert a scripted action to a LegalAction.
    fn action_to_legal_action(
        &self,
        game: &GameState,
        player: PlayerId,
        action: &Action,
        legal_actions: &[LegalAction],
    ) -> Option<LegalAction> {
        match action {
            Action::Pass => Some(LegalAction::PassPriority),

            Action::PlayLand(name) => {
                let land_id = self.find_in_hand(game, player, name)?;
                legal_actions
                    .iter()
                    .find(
                        |la| matches!(la, LegalAction::PlayLand { land_id: id } if *id == land_id),
                    )
                    .cloned()
            }

            Action::CastSpell(name) | Action::CastSpellTargeting { spell: name, .. } => {
                let spell_id = self.find_in_hand(game, player, name)?;
                legal_actions
                    .iter()
                    .find(|la| {
                        matches!(la, LegalAction::CastSpell { spell_id: id, .. } if *id == spell_id)
                    })
                    .cloned()
            }

            Action::TapForMana(name) => {
                let source = self.find_permanent(game, name)?;
                legal_actions
                    .iter()
                    .find(|la| {
                        matches!(la, LegalAction::ActivateManaAbility { source: id, .. } if *id == source)
                    })
                    .cloned()
            }

            Action::ActivateAbility {
                source: name,
                ability_index,
            } => {
                let source = self.find_permanent(game, name)?;
                legal_actions
                    .iter()
                    .find(|la| {
                        matches!(
                            la,
                            LegalAction::ActivateAbility { source: id, ability_index: idx }
                            if *id == source && *idx == *ability_index
                        )
                    })
                    .cloned()
            }

            _ => None, // Non-priority actions handled elsewhere
        }
    }
}

impl DecisionMaker for ScriptedGameDecisionMaker {
    fn decide_priority(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::PriorityContext,
    ) -> LegalAction {
        if let Some(action) = self.next_action() {
            if let Some(legal_action) =
                self.action_to_legal_action(game, ctx.player, &action, &ctx.legal_actions)
            {
                return legal_action;
            }
            // If action doesn't match, log and pass
            eprintln!(
                "Warning: Action {:?} not valid for {:?}, passing priority",
                action, ctx.player
            );
        }
        LegalAction::PassPriority
    }

    fn decide_attackers(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::AttackersContext,
    ) -> Vec<crate::decisions::spec::AttackerDeclaration> {
        if let Some(action) = self.next_action() {
            match action {
                Action::DeclareNoAttackers => {
                    return vec![];
                }
                Action::DeclareAttackers(names) => {
                    let mut declarations = Vec::new();
                    for name in names {
                        if let Some(creature_id) = self.find_permanent(game, name) {
                            // Default to attacking the opponent
                            let opponent =
                                PlayerId::from_index(
                                    if ctx.player.index() == 0 { 1 } else { 0 } as u8
                                );
                            declarations.push(crate::decisions::spec::AttackerDeclaration {
                                creature: creature_id,
                                target: AttackTarget::Player(opponent),
                            });
                        }
                    }
                    return declarations;
                }
                _ => {}
            }
        }
        // Default: no attackers
        vec![]
    }

    fn decide_blockers(
        &mut self,
        game: &GameState,
        _ctx: &crate::decisions::context::BlockersContext,
    ) -> Vec<crate::decisions::spec::BlockerDeclaration> {
        if let Some(action) = self.next_action() {
            match action {
                Action::DeclareNoBlockers => {
                    return vec![];
                }
                Action::DeclareBlockers(pairs) => {
                    let mut declarations = Vec::new();
                    for (blocker_name, attacker_name) in pairs {
                        if let (Some(blocker_id), Some(attacker_id)) = (
                            self.find_permanent(game, blocker_name),
                            self.find_permanent(game, attacker_name),
                        ) {
                            declarations.push(crate::decisions::spec::BlockerDeclaration {
                                blocker: blocker_id,
                                blocking: attacker_id,
                            });
                        }
                    }
                    return declarations;
                }
                _ => {}
            }
        }
        // Default: no blockers
        vec![]
    }

    fn decide_boolean(
        &mut self,
        _game: &GameState,
        _ctx: &crate::decisions::context::BooleanContext,
    ) -> bool {
        // Default: decline may abilities
        false
    }

    fn decide_number(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::NumberContext,
    ) -> u32 {
        // Default to max value
        ctx.max
    }

    fn decide_objects(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::SelectObjectsContext,
    ) -> Vec<ObjectId> {
        // Default: select first legal candidates up to min
        ctx.candidates
            .iter()
            .filter(|c| c.legal)
            .map(|c| c.id)
            .take(ctx.min)
            .collect()
    }

    fn decide_options(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::SelectOptionsContext,
    ) -> Vec<usize> {
        // Default: choose first N options up to min
        (0..ctx.min).collect()
    }

    fn decide_order(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::OrderContext,
    ) -> Vec<ObjectId> {
        // Default: keep original order
        ctx.items.iter().map(|(id, _)| *id).collect()
    }
}

// ============================================================================
// Helper methods for GameState to make assertions easier
// ============================================================================

impl GameState {
    /// Check if a permanent with the given name is on the battlefield.
    pub fn battlefield_has(&self, name: &str) -> bool {
        self.battlefield
            .iter()
            .any(|&id| self.object(id).map(|obj| obj.name == name).unwrap_or(false))
    }

    /// Count permanents with the given name on the battlefield.
    pub fn battlefield_count(&self, name: &str) -> usize {
        self.battlefield
            .iter()
            .filter(|&&id| self.object(id).map(|obj| obj.name == name).unwrap_or(false))
            .count()
    }

    /// Check if a player has a card in hand by name.
    pub fn hand_has(&self, player: PlayerId, name: &str) -> bool {
        self.player(player)
            .map(|p| {
                p.hand
                    .iter()
                    .any(|&id| self.object(id).map(|obj| obj.name == name).unwrap_or(false))
            })
            .unwrap_or(false)
    }

    /// Get a player's life total by index.
    pub fn life_total(&self, player: PlayerId) -> i32 {
        self.player(player).map(|p| p.life).unwrap_or(0)
    }
}

// ============================================================================
// Replay Test Runner - runs games from recorded input files
// ============================================================================

/// Configuration for a replay test.
pub struct ReplayTestConfig {
    /// Cards in each player's starting hand (by name).
    pub hands: Vec<Vec<&'static str>>,
    /// Cards on each player's starting battlefield (by name).
    pub battlefields: Vec<Vec<&'static str>>,
    /// Cards in each player's starting graveyard (by name).
    pub graveyards: Vec<Vec<&'static str>>,
    /// Cards in each player's starting deck (by name).
    pub decks: Vec<Vec<&'static str>>,
    /// Commander(s) in each player's command zone (by name).
    pub commanders: Vec<Vec<&'static str>>,
    /// Commander(s) starting on the battlefield (by name).
    /// These are created on the battlefield AND registered as commanders.
    pub commanders_on_battlefield: Vec<Vec<&'static str>>,
    /// Starting life total (default 20).
    pub starting_life: i32,
    /// Maximum turns to run before stopping.
    pub max_turns: u32,
}

impl Default for ReplayTestConfig {
    fn default() -> Self {
        Self {
            hands: vec![vec![], vec![]],
            battlefields: vec![vec![], vec![]],
            graveyards: vec![vec![], vec![]],
            decks: vec![vec![], vec![]],
            commanders: vec![vec![], vec![]],
            commanders_on_battlefield: vec![vec![], vec![]],
            starting_life: 20,
            max_turns: 100,
        }
    }
}

impl ReplayTestConfig {
    pub fn new() -> Self {
        Self::default()
    }

    fn ensure_len(vec: &mut Vec<Vec<&'static str>>, len: usize) {
        while vec.len() < len {
            vec.push(vec![]);
        }
    }

    fn ensure_player_slots(&mut self, count: usize) {
        Self::ensure_len(&mut self.hands, count);
        Self::ensure_len(&mut self.battlefields, count);
        Self::ensure_len(&mut self.graveyards, count);
        Self::ensure_len(&mut self.decks, count);
        Self::ensure_len(&mut self.commanders, count);
        Self::ensure_len(&mut self.commanders_on_battlefield, count);
    }

    fn player_count(&self) -> usize {
        let mut count = 2;
        count = count.max(self.hands.len());
        count = count.max(self.battlefields.len());
        count = count.max(self.graveyards.len());
        count = count.max(self.decks.len());
        count = count.max(self.commanders.len());
        count = count.max(self.commanders_on_battlefield.len());
        count
    }

    /// Set player 1's starting hand.
    pub fn p1_hand(mut self, cards: Vec<&'static str>) -> Self {
        Self::ensure_len(&mut self.hands, 1);
        self.hands[0] = cards;
        self
    }

    /// Set player 2's starting hand.
    pub fn p2_hand(mut self, cards: Vec<&'static str>) -> Self {
        Self::ensure_len(&mut self.hands, 2);
        self.hands[1] = cards;
        self
    }

    /// Set player 3's starting hand.
    pub fn p3_hand(mut self, cards: Vec<&'static str>) -> Self {
        Self::ensure_len(&mut self.hands, 3);
        self.hands[2] = cards;
        self
    }

    /// Set player 1's starting battlefield.
    pub fn p1_battlefield(mut self, cards: Vec<&'static str>) -> Self {
        Self::ensure_len(&mut self.battlefields, 1);
        self.battlefields[0] = cards;
        self
    }

    /// Set player 2's starting battlefield.
    pub fn p2_battlefield(mut self, cards: Vec<&'static str>) -> Self {
        Self::ensure_len(&mut self.battlefields, 2);
        self.battlefields[1] = cards;
        self
    }

    /// Set player 3's starting battlefield.
    pub fn p3_battlefield(mut self, cards: Vec<&'static str>) -> Self {
        Self::ensure_len(&mut self.battlefields, 3);
        self.battlefields[2] = cards;
        self
    }

    /// Set player 1's starting graveyard.
    pub fn p1_graveyard(mut self, cards: Vec<&'static str>) -> Self {
        Self::ensure_len(&mut self.graveyards, 1);
        self.graveyards[0] = cards;
        self
    }

    /// Set player 2's starting graveyard.
    pub fn p2_graveyard(mut self, cards: Vec<&'static str>) -> Self {
        Self::ensure_len(&mut self.graveyards, 2);
        self.graveyards[1] = cards;
        self
    }

    /// Set player 3's starting graveyard.
    pub fn p3_graveyard(mut self, cards: Vec<&'static str>) -> Self {
        Self::ensure_len(&mut self.graveyards, 3);
        self.graveyards[2] = cards;
        self
    }

    /// Set player 1's starting deck.
    pub fn p1_deck(mut self, cards: Vec<&'static str>) -> Self {
        Self::ensure_len(&mut self.decks, 1);
        self.decks[0] = cards;
        self
    }

    /// Set player 2's starting deck.
    pub fn p2_deck(mut self, cards: Vec<&'static str>) -> Self {
        Self::ensure_len(&mut self.decks, 2);
        self.decks[1] = cards;
        self
    }

    /// Set player 3's starting deck.
    pub fn p3_deck(mut self, cards: Vec<&'static str>) -> Self {
        Self::ensure_len(&mut self.decks, 3);
        self.decks[2] = cards;
        self
    }

    /// Set player 1's commander(s).
    pub fn p1_commander(mut self, cards: Vec<&'static str>) -> Self {
        Self::ensure_len(&mut self.commanders, 1);
        self.commanders[0] = cards;
        self
    }

    /// Set player 2's commander(s).
    pub fn p2_commander(mut self, cards: Vec<&'static str>) -> Self {
        Self::ensure_len(&mut self.commanders, 2);
        self.commanders[1] = cards;
        self
    }

    /// Set player 3's commander(s).
    pub fn p3_commander(mut self, cards: Vec<&'static str>) -> Self {
        Self::ensure_len(&mut self.commanders, 3);
        self.commanders[2] = cards;
        self
    }

    /// Set player 1's commander(s) starting on the battlefield.
    /// These cards are placed on the battlefield AND registered as commanders.
    /// Use this when you need to test commander-related effects without casting.
    pub fn p1_commander_on_battlefield(mut self, cards: Vec<&'static str>) -> Self {
        Self::ensure_len(&mut self.commanders_on_battlefield, 1);
        self.commanders_on_battlefield[0] = cards;
        self
    }

    /// Set player 2's commander(s) starting on the battlefield.
    pub fn p2_commander_on_battlefield(mut self, cards: Vec<&'static str>) -> Self {
        Self::ensure_len(&mut self.commanders_on_battlefield, 2);
        self.commanders_on_battlefield[1] = cards;
        self
    }

    /// Set player 3's commander(s) starting on the battlefield.
    pub fn p3_commander_on_battlefield(mut self, cards: Vec<&'static str>) -> Self {
        Self::ensure_len(&mut self.commanders_on_battlefield, 3);
        self.commanders_on_battlefield[2] = cards;
        self
    }
}

/// Input source for replay tests - either a file path or inline inputs.
#[derive(Debug, Clone)]
pub enum ReplayInput {
    /// Read inputs from a file (each line is an input, # comments are filtered).
    File(&'static str),
    /// Use inline inputs directly (each string is one input).
    Inline(Vec<&'static str>),
}

impl From<&'static str> for ReplayInput {
    fn from(path: &'static str) -> Self {
        ReplayInput::File(path)
    }
}

impl From<Vec<&'static str>> for ReplayInput {
    fn from(inputs: Vec<&'static str>) -> Self {
        ReplayInput::Inline(inputs)
    }
}

/// Reads inputs from a file, filtering out comments.
fn read_inputs_from_file(path: &str) -> Vec<String> {
    use std::fs::File;
    use std::io::{BufRead, BufReader};

    let file = File::open(path).expect(&format!("Failed to open input file: {}", path));
    let reader = BufReader::new(file);

    reader
        .lines()
        .filter_map(|l| l.ok())
        .filter(|l| !l.trim().starts_with('#'))
        .collect()
}

/// Resolves replay inputs to a vector of strings.
fn resolve_inputs(input: ReplayInput) -> Vec<String> {
    match input {
        ReplayInput::File(path) => read_inputs_from_file(path),
        ReplayInput::Inline(inputs) => inputs.into_iter().map(|s| s.to_string()).collect(),
    }
}

/// Runs a game with inputs and returns the final game state.
/// This is a simplified version that starts in the main phase (skipping upkeep/draw)
/// to allow direct testing of spell casting and combat.
///
/// # Arguments
/// * `input` - Either a file path or inline inputs. Use `ReplayInput::File("path")` or
///             `ReplayInput::Inline(vec!["1", "0", ""])` or pass a `&'static str` / `Vec<&'static str>` directly.
/// * `config` - The test configuration (hands, battlefields, etc.)
///
/// # Example with inline inputs
/// ```ignore
/// let game = run_replay_test(
///     vec!["1", "1", "1", "0", "", ""],  // Play land, cast spell, target, pay mana, pass, pass
///     ReplayTestConfig::new()
///         .p1_hand(vec!["Mountain", "Lightning Bolt"])
/// );
/// ```
pub fn run_replay_test(input: impl Into<ReplayInput>, config: ReplayTestConfig) -> GameState {
    use crate::cards::CardRegistry;
    use crate::decision::NumericInputDecisionMaker;
    use crate::game_loop::run_priority_loop_with;
    use crate::triggers::TriggerQueue;

    let registry = CardRegistry::with_builtin_cards();

    let mut config = config;
    let player_count = config.player_count();
    config.ensure_player_slots(player_count);

    let player_names = (1..=player_count)
        .map(|i| format!("Player {}", i))
        .collect::<Vec<_>>();

    // Create game
    let mut game = GameState::new(player_names, config.starting_life);

    // Helper to find card definition by name
    let find_card =
        |name: &str| -> Option<crate::cards::CardDefinition> { registry.get(name).cloned() };

    // Set up starting hands
    for (player_idx, hand_cards) in config.hands.iter().enumerate() {
        let player_id = PlayerId::from_index(player_idx as u8);
        for card_name in hand_cards {
            if let Some(def) = find_card(card_name) {
                game.create_object_from_definition(&def, player_id, Zone::Hand);
            } else {
                panic!("Card not found: {}", card_name);
            }
        }
    }

    // Set up starting battlefields
    for (player_idx, bf_cards) in config.battlefields.iter().enumerate() {
        let player_id = PlayerId::from_index(player_idx as u8);
        for card_name in bf_cards {
            if let Some(def) = find_card(card_name) {
                let obj_id = game.create_object_from_definition(&def, player_id, Zone::Battlefield);
                // Remove summoning sickness for creatures that start on battlefield
                game.remove_summoning_sickness(obj_id);
            } else {
                panic!("Card not found: {}", card_name);
            }
        }
    }

    // Set up starting graveyards
    for (player_idx, gy_cards) in config.graveyards.iter().enumerate() {
        let player_id = PlayerId::from_index(player_idx as u8);
        for card_name in gy_cards {
            if let Some(def) = find_card(card_name) {
                game.create_object_from_definition(&def, player_id, Zone::Graveyard);
            } else {
                panic!("Card not found: {}", card_name);
            }
        }
    }

    // Set up starting decks
    for (player_idx, deck_cards) in config.decks.iter().enumerate() {
        let player_id = PlayerId::from_index(player_idx as u8);
        for card_name in deck_cards {
            if let Some(def) = find_card(card_name) {
                game.create_object_from_definition(&def, player_id, Zone::Library);
            } else {
                panic!("Card not found: {}", card_name);
            }
        }
    }

    // Set up commanders (in command zone)
    for (player_idx, commander_cards) in config.commanders.iter().enumerate() {
        let player_id = PlayerId::from_index(player_idx as u8);
        for card_name in commander_cards {
            if let Some(def) = find_card(card_name) {
                let obj_id = game.create_object_from_definition(&def, player_id, Zone::Command);
                game.set_as_commander(obj_id, player_id);
            } else {
                panic!("Card not found: {}", card_name);
            }
        }
    }

    // Set up commanders on battlefield (already in play, registered as commanders)
    for (player_idx, commander_cards) in config.commanders_on_battlefield.iter().enumerate() {
        let player_id = PlayerId::from_index(player_idx as u8);
        for card_name in commander_cards {
            if let Some(def) = find_card(card_name) {
                let obj_id = game.create_object_from_definition(&def, player_id, Zone::Battlefield);
                game.set_as_commander(obj_id, player_id);
                // Remove summoning sickness for commanders that start on battlefield
                game.remove_summoning_sickness(obj_id);
            } else {
                panic!("Card not found: {}", card_name);
            }
        }
    }

    // Start directly in main phase (skip upkeep/draw for simpler testing)
    let active_player = PlayerId::from_index(0);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = active_player;
    game.turn.priority_player = Some(active_player);

    // Resolve inputs (from file or inline)
    let inputs = resolve_inputs(input.into());

    let mut dm = NumericInputDecisionMaker::new(inputs).with_debug(true);
    let mut trigger_queue = TriggerQueue::new();

    // Run priority loop - this handles casting spells, stack resolution, etc.
    let _ = run_priority_loop_with(&mut game, &mut trigger_queue, &mut dm);

    game
}

/// Runs a game with inputs through a full turn including combat.
/// Unlike `run_replay_test`, this function executes a complete game turn with
/// all phases (upkeep, draw, main, combat, second main, end).
///
/// # Arguments
/// * `input` - Either a file path or inline inputs
/// * `config` - The test configuration (hands, battlefields, etc.)
///
/// # Example
/// ```ignore
/// let game = run_replay_test_full_turn(
///     vec![
///         "",         // Pass upkeep (no mana abilities to tap)
///         "",         // Pass draw step
///         "1",        // Cast Akroma's Will in main phase
///         "0",        // Tap Plains 1
///         "0",        // Tap Plains 2
///         "0",        // Tap Plains 3
///         "0",        // Choose mode 1
///         "",         // Pass main phase (go to combat)
///         "0",        // Declare creature 0 as attacker
///         "",         // Pass declare attackers
///         "",         // Opponent declines to block
///         "",         // Pass blockers step
///         // Combat damage happens automatically
///     ],
///     ReplayTestConfig::new()
///         .p1_hand(vec!["Akroma's Will"])
///         .p1_battlefield(vec!["Plains", "Plains", "Plains", "Plains", "Grizzly Bears"])
/// );
/// ```
pub fn run_replay_test_full_turn(
    input: impl Into<ReplayInput>,
    config: ReplayTestConfig,
) -> GameState {
    use crate::cards::CardRegistry;
    use crate::combat_state::CombatState;
    use crate::decision::NumericInputDecisionMaker;
    use crate::game_loop::execute_turn_with;
    use crate::triggers::TriggerQueue;

    let registry = CardRegistry::with_builtin_cards();

    let mut config = config;
    let player_count = config.player_count();
    config.ensure_player_slots(player_count);

    let player_names = (1..=player_count)
        .map(|i| format!("Player {}", i))
        .collect::<Vec<_>>();

    // Create game
    let mut game = GameState::new(player_names, config.starting_life);

    // Helper to find card definition by name
    let find_card =
        |name: &str| -> Option<crate::cards::CardDefinition> { registry.get(name).cloned() };

    // Set up starting hands
    for (player_idx, hand_cards) in config.hands.iter().enumerate() {
        let player_id = PlayerId::from_index(player_idx as u8);
        for card_name in hand_cards {
            if let Some(def) = find_card(card_name) {
                game.create_object_from_definition(&def, player_id, Zone::Hand);
            } else {
                panic!("Card not found: {}", card_name);
            }
        }
    }

    // Set up starting battlefields
    for (player_idx, bf_cards) in config.battlefields.iter().enumerate() {
        let player_id = PlayerId::from_index(player_idx as u8);
        for card_name in bf_cards {
            if let Some(def) = find_card(card_name) {
                let obj_id = game.create_object_from_definition(&def, player_id, Zone::Battlefield);
                // Remove summoning sickness for creatures that start on battlefield
                game.remove_summoning_sickness(obj_id);
            } else {
                panic!("Card not found: {}", card_name);
            }
        }
    }

    // Set up starting graveyards
    for (player_idx, gy_cards) in config.graveyards.iter().enumerate() {
        let player_id = PlayerId::from_index(player_idx as u8);
        for card_name in gy_cards {
            if let Some(def) = find_card(card_name) {
                game.create_object_from_definition(&def, player_id, Zone::Graveyard);
            } else {
                panic!("Card not found: {}", card_name);
            }
        }
    }

    // Set up starting decks
    for (player_idx, deck_cards) in config.decks.iter().enumerate() {
        let player_id = PlayerId::from_index(player_idx as u8);
        for card_name in deck_cards {
            if let Some(def) = find_card(card_name) {
                game.create_object_from_definition(&def, player_id, Zone::Library);
            } else {
                panic!("Card not found: {}", card_name);
            }
        }
    }

    // Set up commanders (in command zone)
    for (player_idx, commander_cards) in config.commanders.iter().enumerate() {
        let player_id = PlayerId::from_index(player_idx as u8);
        for card_name in commander_cards {
            if let Some(def) = find_card(card_name) {
                let obj_id = game.create_object_from_definition(&def, player_id, Zone::Command);
                game.set_as_commander(obj_id, player_id);
            } else {
                panic!("Card not found: {}", card_name);
            }
        }
    }

    // Set up commanders on battlefield (already in play, registered as commanders)
    for (player_idx, commander_cards) in config.commanders_on_battlefield.iter().enumerate() {
        let player_id = PlayerId::from_index(player_idx as u8);
        for card_name in commander_cards {
            if let Some(def) = find_card(card_name) {
                let obj_id = game.create_object_from_definition(&def, player_id, Zone::Battlefield);
                game.set_as_commander(obj_id, player_id);
                // Remove summoning sickness for commanders that start on battlefield
                game.remove_summoning_sickness(obj_id);
            } else {
                panic!("Card not found: {}", card_name);
            }
        }
    }

    // Set up for turn execution
    let active_player = PlayerId::from_index(0);
    game.turn.active_player = active_player;
    game.turn.turn_number = 1;

    // Resolve inputs (from file or inline)
    let inputs = resolve_inputs(input.into());

    let mut dm = NumericInputDecisionMaker::new(inputs).with_debug(true);
    let mut trigger_queue = TriggerQueue::new();
    let mut combat = CombatState::default();

    // Run a full turn including combat
    let _ = execute_turn_with(&mut game, &mut combat, &mut trigger_queue, &mut dm);

    game
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_play_land() {
        let result = GameScript::new()
            .player("Alice", &["Forest"])
            .player("Bob", &[])
            .action(Action::PlayLand("Forest"))
            .action(Action::Pass)
            .action(Action::Pass) // Bob passes, ending main phase
            .run();

        let game = result.expect("Game should run successfully");

        // Forest should be on the battlefield
        assert!(
            game.battlefield_has("Forest"),
            "Forest should be on battlefield"
        );

        // Alice's hand should be empty
        assert!(
            !game.hand_has(PlayerId::from_index(0), "Forest"),
            "Forest should not be in hand"
        );
    }

    #[test]
    fn test_cast_creature_spell() {
        let result = GameScript::new()
            .player("Alice", &["Forest", "Llanowar Elves"])
            .player("Bob", &[])
            // Main phase 1
            .action(Action::PlayLand("Forest"))
            .action(Action::TapForMana("Forest"))
            .action(Action::CastSpell("Llanowar Elves"))
            .action(Action::Pass)
            .action(Action::Pass) // Spell resolves
            .run();

        let game = result.expect("Game should run successfully");

        // Both Forest and Llanowar Elves should be on battlefield
        assert!(
            game.battlefield_has("Forest"),
            "Forest should be on battlefield"
        );
        assert!(
            game.battlefield_has("Llanowar Elves"),
            "Llanowar Elves should be on battlefield"
        );
    }

    /// Tests Lightning Bolt dealing damage using a replay input file.
    ///
    /// This test reads inputs from tests/scenarios/play_land_bolt.txt which contains:
    /// - Play Mountain
    /// - Cast Lightning Bolt targeting Player 2
    /// - Pay mana by tapping Mountain
    /// - Pass priority to let spell resolve
    #[test]
    fn test_replay_lightning_bolt() {
        let config = ReplayTestConfig::new()
            .p1_hand(vec!["Mountain", "Lightning Bolt"])
            .p1_deck(vec!["Mountain"])
            .p2_deck(vec!["Forest"]);

        let game = run_replay_test("tests/scenarios/test_lightning_bolt.txt", config);

        let bob = PlayerId::from_index(1);

        // Player 2 should have taken 3 damage from Lightning Bolt
        assert_eq!(
            game.life_total(bob),
            17,
            "Player 2 should be at 17 life after Lightning Bolt"
        );

        // Mountain should be on the battlefield
        assert!(
            game.battlefield_has("Mountain"),
            "Mountain should be on battlefield"
        );

        // Lightning Bolt should be in graveyard
        let p1 = PlayerId::from_index(0);
        let bolt_in_gy = game
            .player(p1)
            .map(|p| {
                p.graveyard.iter().any(|&id| {
                    game.object(id)
                        .map(|o| o.name == "Lightning Bolt")
                        .unwrap_or(false)
                })
            })
            .unwrap_or(false);
        assert!(
            bolt_in_gy,
            "Lightning Bolt should be in graveyard after resolving"
        );
    }

    /// Tests simple priority passing using a replay input file.
    ///
    /// This test reads inputs from tests/scenarios/simple_pass.txt which just
    /// passes priority repeatedly.
    #[test]
    fn test_replay_simple_pass() {
        let config = ReplayTestConfig::new()
            .p1_hand(vec!["Mountain"])
            .p1_deck(vec!["Mountain"])
            .p2_deck(vec!["Forest"]);

        let game = run_replay_test("tests/scenarios/simple_pass.txt", config);

        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Both players should still be at 20 life
        assert_eq!(game.life_total(alice), 20, "Player 1 should be at 20 life");
        assert_eq!(game.life_total(bob), 20, "Player 2 should be at 20 life");
    }

    /// Tests Lightning Bolt using inline inputs (no file needed).
    ///
    /// This demonstrates the inline input capability for simpler test setup.
    #[test]
    fn test_replay_inline_lightning_bolt() {
        let game = run_replay_test(
            // Inline inputs: play land, cast bolt, target player 2, tap for mana, pass x2
            vec!["1", "1", "1", "0", "", ""],
            ReplayTestConfig::new()
                .p1_hand(vec!["Mountain", "Lightning Bolt"])
                .p1_deck(vec!["Mountain"])
                .p2_deck(vec!["Forest"]),
        );

        let bob = PlayerId::from_index(1);

        // Player 2 should have taken 3 damage from Lightning Bolt
        assert_eq!(
            game.life_total(bob),
            17,
            "Player 2 should be at 17 life after Lightning Bolt"
        );

        // Mountain should be on the battlefield
        assert!(
            game.battlefield_has("Mountain"),
            "Mountain should be on battlefield"
        );
    }

    #[test]
    fn test_commander_casting_via_c_input() {
        // Test that commanders in the command zone can be cast via 'C' input
        // Grizzly Bears is a simple 2-mana creature to test basic functionality
        let game = run_replay_test(
            vec![
                "c", // Cast commander (Grizzly Bears)
                "0", // Tap first Forest
                "0", // Tap second Forest
                "",  // Pass priority
                "",  // Opponent passes (commander resolves)
            ],
            ReplayTestConfig::new()
                .p1_battlefield(vec!["Forest", "Forest"])
                .p1_commander(vec!["Grizzly Bears"]),
        );

        // Grizzly Bears should now be on the battlefield
        assert!(
            game.battlefield_has("Grizzly Bears"),
            "Commander should be on battlefield after casting"
        );

        // Command zone should be empty (commander moved to battlefield)
        let command_zone_ids = game.objects_in_zone(Zone::Command);
        assert!(
            command_zone_ids.is_empty(),
            "Command zone should be empty after casting commander"
        );
    }

    // Card-specific replay tests have been moved to their respective card definition files.
    // See each card's tests module (e.g., src/cards/definitions/ancient_tomb.rs) for the tests.
    //
    // The following tests remain here as infrastructure/example tests:
    // - test_simple_play_land: Tests the GameScript infrastructure
    // - test_cast_creature_spell: Tests casting through GameScript
    // - test_replay_lightning_bolt: Tests file-based input
    // - test_replay_simple_pass: Tests basic priority passing
    // - test_replay_inline_lightning_bolt: Example of inline inputs
    // - test_commander_casting_via_c_input: Tests commander casting via 'C' input
}
