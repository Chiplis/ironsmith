///
/// This is the main entry point for the decision-based game loop.
/// Call this repeatedly, handling decisions as they come, until
/// it returns `GameProgress::Continue` (phase ends) or `GameProgress::GameOver`.
pub fn advance_priority(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
) -> Result<GameProgress, GameLoopError> {
    let mut dm = crate::decision::AutoPassDecisionMaker;
    advance_priority_with_dm(game, trigger_queue, &mut dm)
}

/// Advance priority with a decision maker for triggered ability targeting.
///
/// This version allows proper target selection for triggered abilities.
pub fn advance_priority_with_dm(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    decision_maker: &mut dyn DecisionMaker,
) -> Result<GameProgress, GameLoopError> {
    // Check for pending replacement effect choice first
    // This takes priority over normal game flow
    if let Some(pending) = &game.pending_replacement_choice {
        let options: Vec<ReplacementOption> = pending
            .applicable_effects
            .iter()
            .enumerate()
            .filter_map(|(i, id)| {
                game.replacement_effects
                    .get_effect(*id)
                    .map(|e| ReplacementOption {
                        index: i,
                        source: e.source,
                        description: format!("{:?}", e.replacement),
                    })
            })
            .collect();

        // Convert to SelectOptionsContext for replacement effect choice
        let selectable_options: Vec<crate::decisions::context::SelectableOption> = options
            .iter()
            .map(|opt| {
                crate::decisions::context::SelectableOption::new(opt.index, &opt.description)
            })
            .collect();
        let ctx = crate::decisions::context::SelectOptionsContext::new(
            pending.player,
            None,
            "Choose replacement effect to apply",
            selectable_options,
            1,
            1,
        );
        return Ok(GameProgress::NeedsDecisionCtx(
            crate::decisions::context::DecisionContext::SelectOptions(ctx),
        ));
    }

    // Check and apply state-based actions
    check_and_apply_sbas(game, trigger_queue)?;

    // Put triggered abilities on the stack with target selection
    put_triggers_on_stack_with_dm(game, trigger_queue, decision_maker)?;

    // Check if game is over
    let remaining: Vec<_> = game
        .players
        .iter()
        .filter(|p| p.is_in_game())
        .map(|p| p.id)
        .collect();

    if remaining.is_empty() {
        return Ok(GameProgress::GameOver(GameResult::Draw));
    }
    if remaining.len() == 1 {
        return Ok(GameProgress::GameOver(GameResult::Winner(remaining[0])));
    }

    // Get current priority player
    let Some(priority_player) = game.turn.priority_player else {
        // No one has priority, phase should end
        return Ok(GameProgress::Continue);
    };

    // Compute legal actions for the priority player
    let mut actions = compute_legal_actions(game, priority_player);
    actions.extend(compute_commander_actions(game, priority_player));

    // Return decision for the player using the new context-based system
    let ctx = crate::decisions::context::PriorityContext::new(priority_player, actions);
    Ok(GameProgress::NeedsDecisionCtx(
        crate::decisions::context::DecisionContext::Priority(ctx),
    ))
}

/// Apply a player's response to a decision during the priority loop.
///
/// This handles both `PriorityAction` responses (for normal priority decisions)
/// and `Targets` responses (when a spell is being cast and needs targets).
pub fn apply_priority_response(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    state: &mut PriorityLoopState,
    response: &PriorityResponse,
) -> Result<GameProgress, GameLoopError> {
    let mut auto_dm = crate::decision::CliDecisionMaker;
    apply_priority_response_with_dm(game, trigger_queue, state, response, &mut auto_dm)
}
