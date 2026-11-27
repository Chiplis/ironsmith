use super::*;
use crate::perf::PerfTimer;
#[cfg(feature = "serialization")]
use serde::Serialize;
use std::cell::RefCell;

#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serialization", derive(Serialize))]
pub struct PriorityAdvancePerfMetrics {
    pub replacement_choice_ms: f64,
    pub state_based_actions_ms: f64,
    pub put_triggers_ms: f64,
    pub game_over_check_ms: f64,
    pub compute_legal_actions_ms: f64,
    pub compute_legal_actions_detail: Option<crate::decision::ComputeLegalActionsPerfMetrics>,
    pub compute_commander_actions_ms: f64,
    pub total_ms: f64,
    pub result_kind: String,
    pub priority_player: Option<u8>,
    pub action_count: usize,
    pub commander_action_count: usize,
}

thread_local! {
    static LAST_PRIORITY_ADVANCE_PERF: RefCell<Option<PriorityAdvancePerfMetrics>> = const { RefCell::new(None) };
}

fn store_priority_advance_perf(metrics: PriorityAdvancePerfMetrics) {
    LAST_PRIORITY_ADVANCE_PERF.with(|slot| {
        *slot.borrow_mut() = Some(metrics);
    });
}

pub fn last_priority_advance_perf() -> Option<PriorityAdvancePerfMetrics> {
    LAST_PRIORITY_ADVANCE_PERF.with(|slot| slot.borrow().clone())
}

fn terminal_progress_or_resume_subgame(
    game: &mut GameState,
    result: GameResult,
    decision_maker: &mut dyn DecisionMaker,
) -> Result<GameProgress, GameLoopError> {
    if game.is_subgame() {
        game.finish_subgame_with(result, decision_maker)
            .map_err(|error| GameLoopError::ResolutionFailed(error.to_string()))?;
        Ok(GameProgress::StackResolved)
    } else {
        game.reveal_face_down_objects_at_game_end();
        Ok(GameProgress::GameOver(result))
    }
}

pub(super) fn finish_mandatory_loop_draw(
    game: &mut GameState,
    decision_maker: &mut dyn DecisionMaker,
) -> Result<GameProgress, GameLoopError> {
    if !game.mandatory_loop_draw_pending() {
        game.mark_mandatory_loop_draw();
    }
    if game.resolve_mandatory_loop_draw()? {
        terminal_progress_or_resume_subgame(game, GameResult::Draw, decision_maker)
    } else {
        Ok(GameProgress::StackResolved)
    }
}

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
    use super::loop_shortcut::{CallSpec, RecordingDm, begin, finish};
    let scope = begin(game, false, CallSpec::Advance);
    if !scope.recording() {
        return advance_priority_unrecorded(game, trigger_queue, decision_maker);
    }
    let mut recorder = RecordingDm::new(decision_maker, &scope);
    let result = advance_priority_unrecorded(game, trigger_queue, &mut recorder);
    let awaiting = recorder.awaiting_choice();
    finish(game, trigger_queue, None, scope, result, awaiting)
}

fn advance_priority_unrecorded(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    decision_maker: &mut dyn DecisionMaker,
) -> Result<GameProgress, GameLoopError> {
    let total_started_at = PerfTimer::start();
    let mut perf = PriorityAdvancePerfMetrics::default();
    if game.subgame_starting_procedure_pending() {
        return Err(GameLoopError::InvalidState(
            "the subgame starting procedure must finish before priority can advance".to_string(),
        ));
    }
    if game.mandatory_loop_draw_pending() {
        perf.total_ms = total_started_at.elapsed_ms();
        perf.result_kind = "mandatory_loop_draw".to_string();
        store_priority_advance_perf(perf);
        return finish_mandatory_loop_draw(game, decision_maker);
    }
    // Check for pending replacement effect choice first
    // This takes priority over normal game flow
    let replacement_started_at = PerfTimer::start();
    if game.effect_store.pending_replacement_choice.is_some() {
        return Err(GameLoopError::InvalidState(
            "legacy replacement choice has no originating operation continuation; resume replacement choices through their operation owner".into(),
        ));
    }
    perf.replacement_choice_ms = replacement_started_at.elapsed_ms();

    // CR 104.1/104.2a: once all but one player (or team) have left, the game
    // is already over; no state-based action or trigger can change that.
    let decided_before_sbas = game.players.len() > 1
        && (game.players.iter().filter(|p| p.is_in_game()).count() == 1
            || game.sole_surviving_team_winners().is_some());
    if !decided_before_sbas {
        // Check and apply state-based actions
        let sba_started_at = PerfTimer::start();
        if let Err(error) = check_and_apply_sbas_with(game, trigger_queue, decision_maker) {
            if matches!(error, GameLoopError::MandatoryLoopDraw) {
                perf.state_based_actions_ms = sba_started_at.elapsed_ms();
                perf.total_ms = total_started_at.elapsed_ms();
                perf.result_kind = "mandatory_loop_draw".to_string();
                store_priority_advance_perf(perf);
                return finish_mandatory_loop_draw(game, decision_maker);
            }
            return Err(error);
        }
        perf.state_based_actions_ms = sba_started_at.elapsed_ms();

        // Put triggered abilities on the stack with target selection
        let triggers_started_at = PerfTimer::start();
        if let Err(error) = put_triggers_on_stack_with_dm(game, trigger_queue, decision_maker) {
            if matches!(error, GameLoopError::MandatoryLoopDraw) {
                perf.put_triggers_ms = triggers_started_at.elapsed_ms();
                perf.total_ms = total_started_at.elapsed_ms();
                perf.result_kind = "mandatory_loop_draw".to_string();
                store_priority_advance_perf(perf);
                return finish_mandatory_loop_draw(game, decision_maker);
            }
            return Err(error);
        }
        // CR 610.3c: an exile-until duration that ended where no player could be
        // asked (a cost payment, the monarch changing, a default-chooser drain)
        // returns its cards now, before anyone receives priority, with the
        // players answering the entry choices. The returns can cause further
        // state-based actions and triggers.
        while game.has_pending_duration_end_returns() && !decision_maker.awaiting_choice() {
            game.process_pending_duration_end_returns(decision_maker)?;
            if decision_maker.awaiting_choice() {
                break;
            }
            drain_pending_trigger_events_with_dm(game, trigger_queue, decision_maker)?;
            check_and_apply_sbas_with(game, trigger_queue, decision_maker)?;
            put_triggers_on_stack_with_dm(game, trigger_queue, decision_maker)?;
        }
        perf.put_triggers_ms = triggers_started_at.elapsed_ms();
    }

    // Check if game is over
    let game_over_started_at = PerfTimer::start();
    let remaining: Vec<_> = game
        .players
        .iter()
        .filter(|p| p.is_in_game())
        .map(|p| p.id)
        .collect();

    if remaining.is_empty() {
        perf.game_over_check_ms = game_over_started_at.elapsed_ms();
        perf.total_ms = total_started_at.elapsed_ms();
        perf.result_kind = "game_over_draw".to_string();
        store_priority_advance_perf(perf);
        return terminal_progress_or_resume_subgame(game, GameResult::Draw, decision_maker);
    }
    if let Some(winners) = game.sole_surviving_team_winners() {
        perf.game_over_check_ms = game_over_started_at.elapsed_ms();
        perf.total_ms = total_started_at.elapsed_ms();
        perf.result_kind = "game_over_team".to_string();
        store_priority_advance_perf(perf);
        for winner in &winners {
            game.mark_team_winner(*winner);
        }
        let result = if winners.len() == 1 {
            GameResult::Winner(winners[0])
        } else {
            GameResult::Remaining(winners)
        };
        return terminal_progress_or_resume_subgame(game, result, decision_maker);
    }
    if remaining.len() == 1 {
        perf.game_over_check_ms = game_over_started_at.elapsed_ms();
        perf.total_ms = total_started_at.elapsed_ms();
        perf.result_kind = "game_over_winner".to_string();
        store_priority_advance_perf(perf);
        let winner = remaining[0];
        if game.is_subgame() {
            return terminal_progress_or_resume_subgame(
                game,
                GameResult::Winner(winner),
                decision_maker,
            );
        }
        if let Some(player) = game.player_mut(winner) {
            player.has_won = true;
        }
        game.finalize_ante_ownership(winner);
        game.reveal_face_down_objects_at_game_end();
        return Ok(GameProgress::GameOver(GameResult::Winner(winner)));
    }
    perf.game_over_check_ms = game_over_started_at.elapsed_ms();

    // Get current priority player
    let Some(priority_player) = game.turn.priority_player else {
        // No one has priority, phase should end
        perf.total_ms = total_started_at.elapsed_ms();
        perf.result_kind = "continue_no_priority_player".to_string();
        store_priority_advance_perf(perf);
        return Ok(GameProgress::Continue);
    };
    perf.priority_player = Some(priority_player.index() as u8);

    // Enumeration reads calculated characteristics for every object, and that
    // read is served from cache only while the continuous state is clean.
    // Putting triggers on the stack above dirties it, so without this the whole
    // menu is computed against a dirty state and every characteristic lookup
    // regenerates all continuous effects from scratch.
    game.refresh_continuous_state().map_err(crate::effects::ExecutionError::ContinuousDiscovery)?;
    let analysis_started_at = PerfTimer::start();
    let ctx = priority_context(game, priority_player)?;
    perf.compute_legal_actions_ms = analysis_started_at.elapsed_ms();
    perf.action_count = ctx.actions.len();
    if ctx.analysis_complete {
        perf.compute_legal_actions_detail = crate::decision::last_compute_legal_actions_perf();
    }
    perf.total_ms = total_started_at.elapsed_ms();
    perf.result_kind = "needs_priority_decision".to_string();
    store_priority_advance_perf(perf);
    Ok(GameProgress::NeedsDecisionCtx(
        crate::decisions::context::DecisionContext::Priority(ctx),
    ))
}

thread_local! {
    static DEFER_PRIORITY_ANALYSIS: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Presentation policy for this engine thread; never changes action validation.
/// WASM owns one engine thread. Native callers default to synchronous decisions.
pub fn set_priority_analysis_deferred(deferred: bool) {
    DEFER_PRIORITY_ANALYSIS.with(|value| value.set(deferred));
}

pub fn priority_analysis_deferred() -> bool {
    DEFER_PRIORITY_ANALYSIS.with(|value| value.get())
}

thread_local! {
    static BOUND_PRIORITY_ANALYSIS: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Menu policy for synchronous priority decisions on this engine thread.
///
/// Off (the default): every offered action carries an exact payment proof.
/// On: affordability is the negative-only bound the browser's deferred menu
/// uses (`payment_may_be_possible`). The menu leaves out only what certainly
/// cannot be paid and runs no payment search; the payment step that follows an
/// announcement stays exact, so an offered action whose payment turns out
/// impossible is cancelled there instead of never being offered. The
/// context's `presentation_actions` list is left empty in this mode. Intended for
/// callers that enumerate far more menus than they take actions from (search,
/// self-play). Never changes which actions the engine accepts.
pub fn set_priority_analysis_bounded(bounded: bool) {
    BOUND_PRIORITY_ANALYSIS.with(|value| value.set(bounded));
}

pub fn priority_analysis_bounded() -> bool {
    BOUND_PRIORITY_ANALYSIS.with(|value| value.get())
}

thread_local! {
    static BOUNDED_PROOF_BUDGET: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// Payment-search units a bounded menu may spend proving its offers.
///
/// Zero (the default) is the plain bound: no search at all. With a budget,
/// every source the bound offers gets that many units of the resumable payment
/// search the browser's deferred menu runs, and keeps an unproven offer only
/// when its search does not finish. An interface then offers unaffordable
/// casts only where proving them is the expensive case. Has no effect unless
/// bounded menus are on.
pub fn set_priority_analysis_bounded_proof_budget(units: usize) {
    BOUNDED_PROOF_BUDGET.with(|value| value.set(units));
}

pub fn priority_analysis_bounded_proof_budget() -> usize {
    BOUNDED_PROOF_BUDGET.with(|value| value.get())
}

thread_local! {
    static PROVEN_OFFERS_ONLY: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

/// Players (one bit per player index) whose budgeted bounded menu holds only
/// proven offers. A person can cancel a payment prompt that opened unfunded;
/// an automated player that answers the same prompt the same way cannot get
/// out of it, so it is not shown an offer whose payment was not found.
pub fn set_priority_analysis_proven_offers_only(players: u64) {
    PROVEN_OFFERS_ONLY.with(|value| value.set(players));
}

pub fn priority_analysis_proven_offers_only() -> u64 {
    PROVEN_OFFERS_ONLY.with(|value| value.get())
}

/// Whether announcing `action` shows other players a card they could not see:
/// an ability activated from hand (cycling), a face-down permanent turned up,
/// a card opened from blind exile.
fn announcement_reveals_hidden_information(game: &GameState, action: &LegalAction) -> bool {
    match action {
        LegalAction::ActivateAbility { source, .. }
        | LegalAction::ActivateManaAbility { source, .. } => game
            .object(*source)
            .is_some_and(|object| object.zone == Zone::Hand),
        LegalAction::TurnFaceUp { .. }
        | LegalAction::OpenExiledCardForPlay { .. }
        | LegalAction::SpecialAction(crate::special_actions::SpecialAction::TurnFaceUp {
            ..
        }) => true,
        _ => false,
    }
}

/// The bounded menu with its offers proven where that is cheap.
///
/// The bound gives every action that is not certainly unaffordable. Each
/// source among them that asks for a payment then gets its own resumable
/// search, as a candidate does in the browser's deferred menu, limited to
/// `budget` units. A source whose search finishes keeps exactly the actions it
/// proved. One whose search does not finish keeps its unproven offers: their
/// payment prompt opens unfunded and can be cancelled. The exceptions are an
/// offer that reveals hidden information as it is announced, which cannot be
/// taken back, and any offer to a player marked proven-only: those are made
/// only with their proof.
fn proven_bounded_menu(
    game: &GameState,
    player: PlayerId,
    budget: usize,
) -> Result<Vec<LegalAction>, GameLoopError> {
    use crate::decision::legal_action_source;
    let mut actions = crate::decision::with_bounded_payment_recheck(true, || {
        compute_legal_actions(game, player)
    })?;
    // Nothing to prove for an action that asks for no mana.
    let needs_proof = |action: &LegalAction| match action {
        LegalAction::PassPriority
        | LegalAction::PlayLand { .. }
        | LegalAction::PlayLandBackFace { .. } => false,
        LegalAction::ActivateManaAbility {
            source,
            ability_index,
        } => game
            .current_ability(*source, *ability_index)
            .is_none_or(|ability| match &ability.kind {
                AbilityKind::Activated(activated) => activated.mana_cost.mana_cost().is_some(),
                _ => true,
            }),
        _ => true,
    };
    let mut sources = Vec::new();
    for action in actions.iter().filter(|action| needs_proof(action)) {
        let source = legal_action_source(action);
        if !sources.contains(&source) {
            sources.push(source);
        }
    }
    let mut proven = Vec::new();
    let mut settled = Vec::new();
    for source in sources {
        let mut session = crate::decision::ManaAnalysisSession::menu_bound();
        let (found, complete) = session.run_for_game(game, budget, || match source {
            Some(source) => crate::decision::compute_actions_for_source(game, player, Some(source)),
            None => crate::decision::compute_global_actions(game, player),
        });
        // A search that failed to calculate proved nothing about its source.
        let Ok(found) = found else { continue };
        if session.failure().is_some() {
            continue;
        }
        proven.extend(found);
        if complete {
            settled.push(source);
        }
    }
    let proven_only = offers_are_proven_only(player);
    actions.retain(|action| {
        !needs_proof(action)
            || proven.contains(action)
            || (!proven_only
                && !settled.contains(&legal_action_source(action))
                && !announcement_reveals_hidden_information(game, action))
    });
    Ok(actions)
}

/// Whether `player` is only ever offered actions whose payment was proven.
fn offers_are_proven_only(player: PlayerId) -> bool {
    u32::try_from(player.index())
        .ok()
        .and_then(|index| 1u64.checked_shl(index))
        .is_some_and(|bit| priority_analysis_proven_offers_only() & bit != 0)
}

/// Commander casts under the bounded menu policy.
///
/// A commander in the command zone is asked about at every priority, and an
/// exact answer for one that cannot be paid for has to exhaust the payment
/// search, which grows exponentially with the mana sources in play. As for the
/// rest of the bounded menu: the negative-only bound rules out what certainly
/// cannot be paid, a search limited to `budget` units proves what it can, and
/// an unfinished search keeps the unproven offer (its payment prompt opens
/// unfunded and can be cancelled) unless the player is marked proven-only.
fn bounded_commander_actions(
    game: &GameState,
    player: PlayerId,
    budget: usize,
) -> Result<Vec<LegalAction>, GameLoopError> {
    let mut actions = crate::decision::with_bounded_payment_recheck(true, || {
        compute_commander_actions(game, player)
    })?;
    if actions.is_empty() || budget == 0 {
        return Ok(actions);
    }
    let mut session = crate::decision::ManaAnalysisSession::menu_bound();
    let (found, complete) =
        session.run_for_game(game, budget, || compute_commander_actions(game, player));
    // A search that failed to calculate proved nothing.
    let (proven, settled) = match found {
        Ok(found) if session.failure().is_none() => (found, complete),
        _ => (Vec::new(), false),
    };
    let proven_only = offers_are_proven_only(player);
    actions.retain(|action| proven.contains(action) || (!proven_only && !settled));
    Ok(actions)
}

/// Legal actions under this thread's menu policy.
fn priority_menu_actions(
    game: &GameState,
    player: PlayerId,
) -> Result<Vec<LegalAction>, GameLoopError> {
    crate::perf::work(crate::perf::Work::Menu);
    if priority_analysis_bounded() {
        let proof_budget = priority_analysis_bounded_proof_budget();
        if proof_budget > 0 {
            return proven_bounded_menu(game, player, proof_budget);
        }
        // One menu asks about the same proposed spells and snapshot facts
        // through several payment routes. An analysis session bound to this
        // game keeps those for the length of the computation, as the
        // browser's menu analysis does; the bound itself never searches.
        let mut session = crate::decision::ManaAnalysisSession::default();
        let (actions, complete) = session.run_for_game(game, usize::MAX, || {
            crate::decision::with_bounded_payment_recheck(true, || {
                compute_legal_actions(game, player)
            })
        });
        if complete && session.failure().is_none() {
            return Ok(actions?);
        }
        // An incomplete or failed session proved nothing: compute without it.
        return Ok(crate::decision::with_bounded_payment_recheck(true, || {
            compute_legal_actions(game, player)
        })?);
    }
    Ok(compute_legal_actions(game, player)?)
}

pub fn priority_context(
    game: &GameState,
    player: PlayerId,
) -> Result<crate::decisions::context::PriorityContext, GameLoopError> {
    game.try_all_continuous_effects().map_err(crate::effects::ExecutionError::ContinuousDiscovery)?;
    if priority_analysis_deferred() {
        let mut ctx = crate::decisions::context::PriorityContext::new(
            game, player,
            vec![LegalAction::PassPriority],
        ).map_err(crate::effects::ExecutionError::ContinuousDiscovery)?;
        ctx.analysis_complete = false;
        Ok(ctx)
    } else {
        analyze_priority_context(game, player)
    }
}

/// Exact enumeration, also used by background analysis against an owned snapshot.
pub fn analyze_priority_context(
    game: &GameState,
    priority_player: PlayerId,
) -> Result<crate::decisions::context::PriorityContext, GameLoopError> {
    let priority_players = game.priority_team_players();
    let mut actions = Vec::new();
    for player in priority_players.iter().copied() {
        for action in priority_menu_actions(game, player)? {
            if !actions.contains(&action) {
                actions.push(action);
            }
        }
    }
    let mut commander_actions = Vec::new();
    for player in priority_players {
        let offered = if priority_analysis_bounded() {
            bounded_commander_actions(game, player, priority_analysis_bounded_proof_budget())?
        } else {
            compute_commander_actions(game, player)?
        };
        for action in offered {
            if !commander_actions.contains(&action) && !actions.contains(&action) {
                commander_actions.push(action);
            }
        }
    }
    actions.extend(commander_actions);
    // CR 732.2: the loop this player just completed may be repeated.
    if game.loop_shortcut_offered_to(priority_player) {
        actions.push(LegalAction::ShortcutLoop);
    }

    let mut ctx = crate::decisions::context::PriorityContext::new(game, priority_player, actions)
        .map_err(crate::effects::ExecutionError::ContinuousDiscovery)?;
    // The presentation list exists so an interface can show announcements
    // whose payment is unproven. A bounded menu already offers every action
    // that is not ruled out, and its callers never render that list.
    if !priority_analysis_bounded() {
        for player in game.priority_team_players() {
            ctx.presentation_actions.extend(crate::decision::compute_actions_assuming_mana_for_presentation(
                game, player, None,
            )?);
        }
    }
    Ok(ctx)
}

pub(super) fn priority_actor_for_action(
    game: &GameState,
    action: &LegalAction,
) -> Result<Option<PlayerId>, GameLoopError> {
    if matches!(action, LegalAction::PassPriority) { return Ok(game.turn.priority_player); }
    for player in game.priority_team_players() {
        if let LegalAction::SpecialAction(action @ (crate::special_actions::SpecialAction::Foretell { card_id } | crate::special_actions::SpecialAction::PrepareExile { card_id, .. })) = action
            && game.is_hidden_card_placeholder(*card_id)
        {
            match crate::special_actions::can_perform_check(action, game, player) {
                Ok(()) => return Ok(Some(player)),
                Err(crate::special_actions::ActionError::ExecutionFailure { error, .. }) => return Err(error.into()),
                Err(_) => continue,
            }
        }
        let candidates = if matches!(action, LegalAction::CastSpell { .. }
            | LegalAction::ActivateAbility { .. } | LegalAction::ActivateManaAbility { .. }) {
            crate::decision::compute_actions_assuming_mana_for_presentation(game, player,
                crate::decision::legal_action_source(action))?
        } else {
            crate::decision::with_bounded_payment_recheck(
                priority_analysis_deferred() || priority_analysis_bounded(),
                || crate::decision::compute_actions_for_source(
                    game, player, crate::decision::legal_action_source(action),
                ),
            )?
        };
        if candidates.contains(action) { return Ok(Some(player)); }
    }
    Ok(None)
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

#[cfg(test)]
mod deferred_analysis_tests {
    use super::*;

    #[test]
    fn pending_priority_exposes_pass_without_auto_passing_or_changing_the_game() {
        struct Restore(bool);
        impl Drop for Restore {
            fn drop(&mut self) {
                set_priority_analysis_deferred(self.0);
            }
        }
        let _restore = Restore(priority_analysis_deferred());
        set_priority_analysis_deferred(true);
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = PlayerId::from_index(0);
        game.turn.priority_player = Some(alice);
        let ctx = priority_context(&game, alice).expect("fixture has complete replacement state");
        assert!(!ctx.analysis_complete);
        assert_eq!(ctx.actions.to_vec(), vec![LegalAction::PassPriority]);
        assert!(!super::super::priority_mana::should_auto_pass_ctx(
            &crate::decisions::context::DecisionContext::Priority(ctx)
        ));
        let full = analyze_priority_context(&game, alice).expect("fixture has complete replacement state");
        assert!(full.analysis_complete);
        assert_eq!(game.turn.priority_player, Some(alice));
    }
}

#[cfg(test)]
mod bounded_special_action_tests {
    use super::*;
    use crate::{ManaCost, ManaSymbol, Phase};

    #[test]
    fn special_action_actor_recheck_does_not_restart_payment_search() {
        let player = PlayerId::from_index(0);
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.active_player = player;
        game.turn.priority_player = Some(player);
        let spell = crate::cards::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Suspend probe")
            .card_types(vec![CardType::Creature])
            .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Blue; 3]))
            .suspend(2, ManaCost::new().add_generic(1))
            .build();
        let card = game.create_object_from_definition(&spell, player, Zone::Hand);
        let land = crate::cards::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Life source")
            .card_types(vec![CardType::Land])
            .with_ability(crate::Ability::mana_with_effects(
                crate::cost::TotalCost::from_cost(crate::costs::Cost::life(1)),
                vec![crate::effect::Effect::add_mana_of_any_color_restricted(
                    1, vec![crate::color::Color::White, crate::color::Color::Blue],
                )],
            ))
            .build();
        game.create_object_from_definition(&land, player, Zone::Battlefield);
        let action = LegalAction::SpecialAction(crate::special_actions::SpecialAction::Suspend { card_id: card });
        for (bounded, deferred) in [(true, false), (false, true)] {
            let previous = (priority_analysis_bounded(), priority_analysis_deferred());
            set_priority_analysis_bounded(bounded);
            set_priority_analysis_deferred(deferred);
            let mut session = crate::decision::ManaAnalysisSession::default();
            let (actor, complete) = session.run_for_game(&game, 1, || {
                priority_actor_for_action(&game, &action)
            });
            set_priority_analysis_bounded(previous.0);
            set_priority_analysis_deferred(previous.1);
            assert!(complete, "actor admission restarted payment search");
            assert_eq!(actor.unwrap(), Some(player));
            assert_eq!(session.last_slice_nodes(), 0);
            assert!(session.failure().is_none());
        }
        game.turn.priority_player = Some(PlayerId::from_index(1));
        assert_eq!(priority_actor_for_action(&game, &action).unwrap(), None);
        assert_eq!(game.object(card).unwrap().zone, Zone::Hand);
        assert_eq!(game.player(player).unwrap().life, 20);
    }
}
