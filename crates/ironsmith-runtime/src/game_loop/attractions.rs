use super::*;

use crate::ability::TriggeredAbility;
use crate::events::other::DieRolledEvent;
use crate::triggers::{TriggeredAbilitySourceKind, compute_trigger_identity};

/// Perform the CR 505.5 / 717.5 precombat-main Attraction turn-based action.
///
/// The roll is made only if the active player controls at least one face-up
/// Attraction. Every controlled Attraction whose physical printing has the
/// result lit is visited simultaneously, and its stored Visit program becomes
/// a normal triggered ability waiting to be put on the stack.
pub fn roll_to_visit_attractions(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
) -> Result<Option<u32>, GameLoopError> {
    let player = game.turn.active_player;
    if !game.face_up_attractions().iter().any(|object| {
        game.object(*object).is_some_and(|candidate| {
            candidate.zone == Zone::Battlefield && game.current_controller(*object) == Some(player)
        })
    }) {
        return Ok(None);
    }

    // The TurnRunner currently has no interactive effect-resolution decision
    // channel at a turn-based action. Reuse the ordinary die transaction so
    // mandatory/static modifiers and deterministic forced rolls stay aligned;
    // AutoPass declines genuinely optional modifier payments.
    let rule_source = ObjectId::from_raw(0);
    let mut decision_maker = crate::decision::AutoPassDecisionMaker;
    let mut context = ExecutionContext::new(rule_source, player, &mut decision_maker);
    let Some(mut rolls) = crate::effects::player::die_roll_transaction::roll_dice_with_modifiers(
        game,
        &mut context,
        player,
        1,
        6,
    )
    .map_err(|error| GameLoopError::ResolutionFailed(error.to_string()))?
    else {
        return Ok(None);
    };
    let roll = rolls.remove(0);

    game.turn_store
        .turn_history
        .record_die_roll(player, roll.result);
    game.mark_continuous_state_dirty();
    game.record_ui_effect_event(
        "attraction_visit_roll",
        Some(player),
        None,
        Vec::new(),
        Some(i64::from(roll.result)),
        Some("d6".to_string()),
    );
    let provenance = crate::provenance::ProvNodeId::default();
    queue_triggers_from_event(
        game,
        trigger_queue,
        TriggerEvent::new_with_provenance(
            DieRolledEvent::new_with_natural_result(
                player,
                rule_source,
                roll.natural_result,
                roll.result,
                6,
            ),
            provenance,
        ),
        true,
    );

    let visits = game.attraction_visit_profiles(player, roll.result);
    let visit_events = visits
        .iter()
        .map(|visit| {
            TriggerEvent::new_with_provenance(
                KeywordActionEvent::new(
                    KeywordActionKind::VisitAttraction,
                    player,
                    visit.object,
                    1,
                ),
                provenance,
            )
        })
        .collect::<Vec<_>>();

    // Record the whole simultaneous visit batch before checking observers, so
    // turn-history conditions (for example Soul Swindler) see the completed
    // turn-based action while any resulting triggers are being created.
    queue_triggers_for_simultaneous_events(game, trigger_queue, visit_events.clone());

    for (visit, event) in visits.into_iter().zip(visit_events) {
        let source_snapshot = game.object(visit.object).map(|object| {
            ObjectSnapshot::from_object_with_calculated_characteristics(object, game)
        });
        let ability = TriggeredAbility {
            trigger: crate::triggers::Trigger::keyword_action(
                KeywordActionKind::VisitAttraction,
                crate::target::PlayerFilter::Any,
            ),
            effects: visit.program,
            choices: Vec::new(),
            intervening_if: None,
            presentation_label: Some(ironsmith_core::PresentationLabel::AbilityWord(
                "Visit".to_string(),
            )),
        };
        let trigger_identity = compute_trigger_identity(&ability);
        trigger_queue.add(TriggeredAbilityEntry {
            source: visit.object,
            controller: visit.controller,
            x_value: None,
            event_value_amount: None,
            ability,
            triggering_event: event,
            source_stable_id: visit.stable_id,
            source_name: visit.name,
            source_snapshot,
            tagged_objects: std::collections::HashMap::new(),
            source_kind: TriggeredAbilitySourceKind::Object,
            trigger_identity,
        });
    }

    Ok(Some(roll.result))
}
