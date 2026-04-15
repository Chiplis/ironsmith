use crate::game_state::GameState;

pub(crate) fn setup_two_player_game() -> GameState {
    GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
}
