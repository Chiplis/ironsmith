use crate::game_state::GameState;

pub(crate) fn setup_two_player_game() -> GameState {
    GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
}

/// Complete a successful fixture's original movement and every deferred
/// replacement instruction before returning the original zone-change result.
pub(crate) fn finish_fixture_zone_change(
    game: &mut GameState,
    source: crate::ids::ObjectId,
    controller: crate::ids::PlayerId,
    receipt: crate::events::processing::PreparedEventOutcome<crate::effects::zones::AppliedZoneChange>,
    decision_maker: &mut dyn crate::decision::DecisionMaker,
) -> crate::events::processing::EventOutcome<crate::effects::zones::AppliedZoneChange> {
    let original = receipt.original.clone();
    let mut ctx = crate::effects::ExecutionContext::new(source, controller, decision_maker);
    let mut outcome = crate::effects::zones::finish_zone_change_receipts(
        game, &mut ctx, crate::effect::EffectOutcome::count(1), vec![(source, receipt)],
    ).expect("fixture replacement instructions must complete successfully");
    assert!(!ctx.decision_maker.awaiting_choice(), "fixture movement must not remain pending");
    crate::effects::retain_unmatched_outcome_events(game, &mut outcome.events);
    for event in outcome.events { game.queue_trigger_event(event.provenance(), event); }
    original
}

/// Fixture adapter for a known successful battlefield entry. Unlike the old
/// Option adapter, this completes deferred replacement instructions explicitly.
pub(crate) fn enter_fixture_with_dm(
    game: &mut GameState,
    object: crate::ids::ObjectId,
    decision_maker: &mut dyn crate::decision::DecisionMaker,
    expected: &str,
) -> crate::game_state::EntersResult {
    use crate::events::processing::{EventOutcome, PreparedEventOutcome};
    let controller = game.controller_of(game.object(object).expect("fixture source must exist"));
    let receipt = game.move_object_with_etb_processing_with_dm(
        object, crate::zone::Zone::Battlefield, decision_maker,
    ).expect("replacement operation must execute successfully in this scenario");
    assert!(!receipt.pending, "fixture entry must not remain pending");
    let original = receipt.original.clone();
    let mapped = match receipt.original {
        EventOutcome::Proceed(entry) => EventOutcome::Proceed(crate::effects::zones::AppliedZoneChange {
            final_zone: crate::zone::Zone::Battlefield,
            new_object_id: Some(entry.new_id), new_object_ids: vec![entry.new_id],
        }),
        EventOutcome::Prevented => EventOutcome::Prevented,
        EventOutcome::Replaced => EventOutcome::Replaced,
        EventOutcome::NotApplicable => EventOutcome::NotApplicable,
    };
    let completed = finish_fixture_zone_change(game, object, controller,
        PreparedEventOutcome { original: mapped, programs: receipt.programs }, decision_maker);
    assert!(matches!(completed, EventOutcome::Proceed(_)), "{expected}: {completed:?}");
    let EventOutcome::Proceed(entry) = original else { panic!("{expected}: {original:?}"); };
    entry
}

pub(crate) fn enter_fixture(
    game: &mut GameState,
    object: crate::ids::ObjectId,
    expected: &str,
) -> crate::game_state::EntersResult {
    enter_fixture_with_dm(game, object, &mut crate::decision::SelectFirstDecisionMaker, expected)
}
