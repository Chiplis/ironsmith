//! Subroutines and programs (custom CR 303.9, 505.6, 701.81).
//!
//! A Subroutine (an enchantment subtype) has a program designation chosen as
//! it enters (CR 303.9a). In their precombat main phase, a player who controls
//! one or more Subroutines runs their next program (CR 505.6, a turn-based
//! action that doesn't use the stack): the current program advances
//! A -> B -> C -> A (starting at A), then each Subroutine they control with
//! that designation runs (CR 701.81a, 303.9b). Running has no intrinsic
//! effect; it is an event abilities can trigger on, and it is recorded in the
//! controller's turn history ("a Subroutine ran under your control").
use crate::ability::Ability;
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::types::Subtype;

/// The program designations, in program order.
pub const PROGRAM_DESIGNATIONS: [&str; 3] = ["Program A", "Program B", "Program C"];

pub fn is_subroutine_subtype(subtype: &Subtype) -> bool {
    matches!(subtype, Subtype::Custom(custom) if custom.name().eq_ignore_ascii_case("subroutine"))
}

/// The inherent "as this Subroutine enters, choose its program designation"
/// choice (CR 303.9a).
pub fn program_designation_choice() -> Ability {
    Ability::static_ability(crate::static_abilities::StaticAbility::new(
        crate::static_abilities::ChooseNamedOptionAsEnters::new(
            PROGRAM_DESIGNATIONS.iter().map(|name| name.to_string()).collect(),
            "As this Subroutine enters, choose Program A, Program B, or Program C".to_string(),
        ),
    ))
}

fn controlled_subroutines(game: &GameState, player: PlayerId) -> Vec<ObjectId> {
    game.battlefield
        .iter()
        .copied()
        .filter(|id| {
            game.object(*id).is_some_and(|object| game.controller_of(object) == player)
                && game.current_subtypes(*id).is_some_and(|subtypes| subtypes.iter().any(is_subroutine_subtype))
        })
        .collect()
}

/// CR 505.6 + 701.81a: the active player runs their next program if they
/// control one or more Subroutines.
pub fn run_next_program_turn_based_action(game: &mut GameState) -> Result<(), crate::effects::ExecutionError> {
    let player = game.turn.active_player;
    let subroutines = controlled_subroutines(game, player);
    if subroutines.is_empty() {
        return Ok(());
    }
    let next = match game.current_program(player) {
        None => 0,
        Some(current) => (current + 1) % 3,
    };
    game.set_current_program(player, next);
    let designation = PROGRAM_DESIGNATIONS[usize::from(next)];
    for subroutine in subroutines {
        if game.chosen_named_option(subroutine) != Some(designation) {
            continue;
        }
        let Some(snapshot) = crate::snapshot::ObjectSnapshot::from_object_id(game, subroutine) else { continue };
        game.record_player_named_action(player, ironsmith_core::SUBROUTINE_RUN_ACTION)?;
        let event = crate::events::NamedActionEvent::new(
            "run", player, subroutine, snapshot.clone(), vec![snapshot], 1,
            crate::events::EventCause::from_game_rule(),
        );
        let provenance = game.alloc_child_event_provenance(Default::default(), crate::events::EventKind::KeywordAction);
        game.queue_trigger_event(Default::default(), crate::triggers::TriggerEvent::new_with_provenance(event, provenance));
    }
    Ok(())
}
