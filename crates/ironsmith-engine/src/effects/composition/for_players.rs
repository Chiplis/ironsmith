//! ForPlayers effect implementation.

use crate::effect::{Effect, EffectOutcome};
#[cfg(test)]
use crate::effects::execute_effect;
use crate::effects::{EffectExecutor, SimultaneousEffectProposal};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::filter::player_filter_matches_game;
use crate::game_state::GameState;
use crate::ids::PlayerId;
use crate::target::PlayerFilter;

/// Effect that applies effects once for each player matching a filter.
///
/// Sets `ctx.iterated_player` for each iteration, allowing inner effects
/// to reference the current player via `PlayerFilter::IteratedPlayer`.
///
/// # Fields
///
/// * `filter` - Filter for which players to iterate over
/// * `effects` - Effects to execute for each matching player
///
/// # Example
///
/// ```ignore
/// // Deal 3 damage to each opponent
/// let effect = ForPlayersEffect::new(
///     PlayerFilter::Opponent,
///     vec![Effect::deal_damage(3, ChooseSpec::Player(PlayerFilter::IteratedPlayer))],
/// );
///
/// // Each player draws a card
/// let effect = ForPlayersEffect::new(
///     PlayerFilter::Any,
///     vec![Effect::target_draws(1, PlayerFilter::IteratedPlayer)],
/// );
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct ForPlayersEffect {
    /// Filter for which players to iterate over.
    pub filter: PlayerFilter,
    /// Effects to execute for each matching player.
    pub effects: Vec<Effect>,
    /// Whether iteration should begin with the effect controller and proceed in turn order.
    pub starting_with_controller: bool,
    /// Complete the body for one player before proceeding to the next.
    pub sequential: bool,
    /// Whether iteration should stop after the first player whose effects happened.
    pub stop_after_first_happened: bool,
}

impl ForPlayersEffect {
    /// Create a new ForPlayers effect.
    pub fn new(filter: PlayerFilter, effects: Vec<Effect>) -> Self {
        Self {
            filter,
            effects,
            starting_with_controller: false,
            sequential: false,
            stop_after_first_happened: false,
        }
    }

    pub fn new_starting_with_controller(filter: PlayerFilter, effects: Vec<Effect>) -> Self {
        Self {
            filter,
            effects,
            starting_with_controller: true,
            sequential: false,
            stop_after_first_happened: false,
        }
    }

    pub fn stop_after_first_happened(mut self) -> Self {
        self.stop_after_first_happened = true;
        self
    }
}

fn rotate_players_to_start(players: &mut Vec<PlayerId>, start: PlayerId) {
    if let Some(start_pos) = players.iter().position(|&player_id| player_id == start) {
        players.rotate_left(start_pos);
    }
}

fn order_selected_players_from(
    game: &GameState,
    selected_players: Vec<PlayerId>,
    start: PlayerId,
) -> Vec<PlayerId> {
    let mut turn_order = game.turn_store.turn_order.clone();
    rotate_players_to_start(&mut turn_order, start);

    let mut ordered_players = turn_order
        .into_iter()
        .filter(|player_id| selected_players.contains(player_id))
        .collect::<Vec<_>>();
    for player_id in selected_players {
        if !ordered_players.contains(&player_id) {
            ordered_players.push(player_id);
        }
    }
    ordered_players
}

/// In Two-Headed Giant, a shared-life action like a "set life total" effect
/// applies once per team: the team's primary player picks which head performs
/// it at the team's first position in APNAP order. Returns, per shared
/// effect, the acting players in that order; other effects keep ordinary
/// per-player iteration.
fn twohg_shared_action_players(
    game: &GameState,
    ctx: &mut ExecutionContext,
    effects: &[Effect],
    players: &[PlayerId],
) -> Result<std::collections::HashMap<usize, Vec<PlayerId>>, ExecutionError> {
    let mut shared: std::collections::HashMap<usize, Vec<PlayerId>> =
        std::collections::HashMap::new();
    if game.two_headed_giant().is_none() {
        return Ok(shared);
    }
    for (effect_index, effect) in effects.iter().enumerate() {
        if effect
            .downcast_ref::<crate::effects::SetLifeTotalEffect>()
            .is_none()
        {
            continue;
        }
        let mut seen_teams = std::collections::HashSet::new();
        for player in players.iter().copied() {
            let Some(team) = game.team_index_for(player) else {
                continue;
            };
            if !seen_teams.insert(team) {
                continue;
            }
            let candidates = game
                .team_players_for(player)
                .into_iter()
                .filter(|member| players.contains(member))
                .collect::<Vec<_>>();
            if candidates.len() == 1 {
                shared.entry(effect_index).or_default().push(candidates[0]);
                continue;
            }
            let options = candidates
                .iter()
                .filter_map(|member| {
                    game.player(*member)
                        .map(|candidate| (candidate.name.to_string(), *member))
                })
                .collect::<Vec<_>>();
            let chooser = game.primary_player_for_team(team).unwrap_or(player);
            let chosen = crate::decisions::ask_choose_one(
                game,
                &mut ctx.decision_maker,
                chooser,
                ctx.source,
                &options,
            )
            .unwrap_or(player);
            if ctx.decision_maker.awaiting_choice() {
                return Ok(shared);
            }
            shared.entry(effect_index).or_default().push(chosen);
        }
    }
    Ok(shared)
}

/// Flatten coordinated `SequenceEffect` wrappers into the printed actions they
/// contain.
///
/// A simultaneous each-player action is analysed as action units — runs of
/// read-only chooser effects plus the one mutating effect they feed (CR 608.2e).
/// Lowering groups a chooser and its mutator into a single coordinated
/// `SequenceEffect` when Oracle prints them as one clause ("Each player chooses
/// ... , then sacrifices the rest"), and that wrapper implements neither
/// `is_read_only_simultaneous_player_action` nor
/// `supports_simultaneous_player_action`. Left wrapped it is opaque to the unit
/// grouping and the whole action is rejected, so unwrap it here: inside
/// `ForPlayers` a sequence is exactly an ordered list of that player's actions.
fn flatten_sequences_for_simultaneous_units(
    effects: &[Effect],
    has_target_assignments: bool,
) -> Vec<Effect> {
    let mut flattened = Vec::with_capacity(effects.len());
    for effect in effects {
        match effect.downcast_ref::<crate::effects::SequenceEffect>() {
            // A multi-child sequence scopes announced target assignments to
            // its children; unwrapping it would change which target each
            // child reads. Without announced targets the wrapper is pure
            // ordering, so unwrap it (recursively) and let each printed
            // action finish for every player before the next (CR 608.2e).
            Some(sequence)
                if !sequence.effects.is_empty()
                    && (!has_target_assignments
                        || sequence.effects.len() == 1
                        || sequence.effects.iter().all(|child| {
                            child.0.is_read_only_simultaneous_player_action()
                                || child.0.supports_simultaneous_player_action()
                        })) =>
            {
                flattened.extend(flatten_sequences_for_simultaneous_units(
                    &sequence.effects,
                    has_target_assignments,
                ));
            }
            _ => flattened.push(effect.clone()),
        }
    }
    flattened
}

/// Optional programs retain one acceptance per player, but each child printed
/// action still uses the same APNAP preparation/commit phases as an unwrapped
/// action. Markers are scheduler instructions, never executable placeholders.
#[derive(Clone)]
enum ProgramWrapper {
    ResultId(crate::effect::EffectId),
    Tagged(crate::effects::TaggedEffect),
    Source(crate::effects::ExecuteWithSourceEffect),
    Rewrite(crate::effects::LocalRewriteEffect),
}

#[derive(Clone)]
enum CapturedProgramScope {
    Tagged(
        crate::effects::TaggedEffect,
        super::tagging_runtime::TaggedRuntimeState,
    ),
    Source(
        (
            crate::ids::ObjectId,
            Option<crate::snapshot::ObjectSnapshot>,
        ),
    ),
    Rewrite(Vec<crate::replacement::ReplacementEffect>),
}

#[derive(Clone)]
struct CapturedProgramGroup {
    scopes: Vec<CapturedProgramScope>,
    valid: bool,
}

fn with_program_scope<T>(
    ctx: &mut ExecutionContext,
    scopes: &[CapturedProgramScope],
    f: impl FnOnce(&mut ExecutionContext) -> T,
) -> T {
    let Some((scope, rest)) = scopes.split_first() else {
        return f(ctx);
    };
    match scope {
        CapturedProgramScope::Source(binding) => {
            super::execute_with_source::with_source_binding(ctx, binding, |ctx| {
                with_program_scope(ctx, rest, f)
            })
        }
        CapturedProgramScope::Rewrite(replacements) => ctx
            .with_temp_additional_replacement_effects(replacements.clone(), |ctx| {
                with_program_scope(ctx, rest, f)
            }),
        CapturedProgramScope::Tagged(_, _) => with_program_scope(ctx, rest, f),
    }
}

fn capture_program_group(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    wrappers: &[ProgramWrapper],
) -> Result<CapturedProgramGroup, ExecutionError> {
    let Some((wrapper, rest)) = wrappers.split_first() else {
        return Ok(CapturedProgramGroup {
            scopes: vec![],
            valid: true,
        });
    };
    let scope = match wrapper {
        ProgramWrapper::ResultId(id) => {
            ctx.effect_outcomes.remove(id);
            return capture_program_group(game, ctx, rest);
        }
        ProgramWrapper::Tagged(effect) => CapturedProgramScope::Tagged(
            effect.clone(),
            super::tagging_runtime::capture_tagged_runtime_state(game, &effect.effect, ctx),
        ),
        ProgramWrapper::Source(effect) => {
            let Some(binding) =
                super::execute_with_source::resolve_source_binding(effect, game, ctx)
            else {
                return Ok(CapturedProgramGroup {
                    scopes: vec![],
                    valid: false,
                });
            };
            CapturedProgramScope::Source(binding)
        }
        ProgramWrapper::Rewrite(effect) => CapturedProgramScope::Rewrite(
            super::local_rewrite::prepare_local_replacements(effect, game, ctx)?,
        ),
    };
    if ctx.decision_maker.awaiting_choice() {
        return Ok(CapturedProgramGroup {
            scopes: vec![scope],
            valid: false,
        });
    }
    let mut children = with_program_scope(ctx, std::slice::from_ref(&scope), |ctx| {
        capture_program_group(game, ctx, rest)
    })?;
    children.scopes.insert(0, scope);
    Ok(children)
}

fn program_path_scopes(
    path: &[usize],
    groups: &[Vec<Option<CapturedProgramGroup>>],
    player_index: usize,
) -> Vec<CapturedProgramScope> {
    path.iter()
        .flat_map(|&group| {
            groups[group][player_index]
                .as_ref()
                .into_iter()
                .flat_map(|captured| captured.scopes.iter().cloned())
        })
        .collect()
}

fn finish_program_scope(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    scopes: &[CapturedProgramScope],
    outcome: &EffectOutcome,
) {
    for (index, scope) in scopes.iter().enumerate().rev() {
        if let CapturedProgramScope::Tagged(effect, runtime) = scope {
            with_program_scope(ctx, &scopes[..index], |ctx| {
                super::tagged::apply_outcome_tags(effect, game, ctx, outcome, runtime.clone())
            });
        }
    }
}

struct OptionalActionProgram {
    effects: Vec<Effect>,
    markers: Vec<Option<(usize, bool)>>,
    paths: Vec<Vec<usize>>,
    offers: Vec<crate::effects::MayEffect>,
    outcome_ids: Vec<Vec<crate::effect::EffectId>>,
    wrappers: Vec<Vec<ProgramWrapper>>,
    is_optional: Vec<bool>,
}

impl OptionalActionProgram {
    fn new(effects: &[Effect], has_targets: bool) -> Self {
        fn append(
            program: &mut OptionalActionProgram,
            effects: &[Effect],
            path: &[usize],
            has_targets: bool,
        ) {
            for effect in flatten_sequences_for_simultaneous_units(effects, has_targets) {
                let mut unwrapped = &effect;
                let mut outcome_ids = Vec::new();
                let mut wrappers = Vec::new();
                loop {
                    if let Some(annotation) =
                        unwrapped.downcast_ref::<crate::effects::WithIdEffect>()
                    {
                        outcome_ids.push(annotation.id);
                        wrappers.push(ProgramWrapper::ResultId(annotation.id));
                        unwrapped = &annotation.effect;
                    } else if let Some(tagged) =
                        unwrapped.downcast_ref::<crate::effects::TaggedEffect>()
                    {
                        wrappers.push(ProgramWrapper::Tagged(tagged.clone()));
                        unwrapped = &tagged.effect;
                    } else if let Some(source) =
                        unwrapped.downcast_ref::<crate::effects::ExecuteWithSourceEffect>()
                    {
                        wrappers.push(ProgramWrapper::Source(source.clone()));
                        unwrapped = &source.effect;
                    } else if let Some(rewrite) =
                        unwrapped.downcast_ref::<crate::effects::LocalRewriteEffect>()
                    {
                        wrappers.push(ProgramWrapper::Rewrite(rewrite.clone()));
                        unwrapped = &rewrite.effect;
                    } else {
                        break;
                    }
                }
                let optional = unwrapped.downcast_ref::<crate::effects::MayEffect>();
                // A compound payment is one action owner. Splitting its
                // children would bypass the TotalCost transaction.
                if optional.is_some_and(|optional| optional.pay_as_cost) {
                    program.effects.push(effect);
                    program.markers.push(None);
                    program.paths.push(path.to_vec());
                    continue;
                }
                if optional.is_some() || !wrappers.is_empty() {
                    let offer = optional
                        .cloned()
                        .unwrap_or_else(|| crate::effects::MayEffect::new(vec![unwrapped.clone()]));
                    let group = program.offers.len();
                    program.offers.push(offer.clone());
                    program.wrappers.push(wrappers);
                    program.is_optional.push(optional.is_some());
                    program.outcome_ids.push(outcome_ids);
                    program.effects.push(effect.clone());
                    program.markers.push(Some((group, true)));
                    program.paths.push(path.to_vec());
                    let mut children_path = path.to_vec();
                    children_path.push(group);
                    append(program, &offer.effects, &children_path, has_targets);
                    program.effects.push(effect.clone());
                    program.markers.push(Some((group, false)));
                    program.paths.push(path.to_vec());
                } else {
                    program.effects.push(effect);
                    program.markers.push(None);
                    program.paths.push(path.to_vec());
                }
            }
        }
        let mut program = Self {
            effects: vec![],
            markers: vec![],
            paths: vec![],
            offers: vec![],
            outcome_ids: vec![],
            wrappers: vec![],
            is_optional: vec![],
        };
        append(&mut program, effects, &[], has_targets);
        program
    }
    fn path_is_optional(&self, path: &[usize]) -> bool {
        path.iter().any(|&group| self.is_optional[group])
    }
}

/// Acceptance belongs to the first child's preparation, so all choices for
/// this participant precede the next participant's choices (101.4c). Later
/// child units consult the retained answer and never offer the same action again.
fn prepare_optional_instruction(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    player_index: usize,
    effect_index: usize,
    program: &OptionalActionProgram,
    acceptance: &mut [Vec<bool>],
    initialized: &mut [bool],
    limits: &mut [Option<crate::effects::DoThisLimit>],
    reached: &mut [bool],
    groups: &mut [Vec<Option<CapturedProgramGroup>>],
) -> Result<bool, ExecutionError> {
    if !program.paths[effect_index]
        .iter()
        .all(|&group| acceptance[group][player_index])
    {
        return Ok(false);
    }
    if let Some((group, true)) = program.markers[effect_index] {
        if !initialized[group] {
            if program.is_optional[group] {
                limits[group] = ctx.do_this_limit.take();
                reached[group] = limits[group].is_some_and(|limit| limit.reached(game));
            }
            initialized[group] = true;
        }
        let parent_scopes = program_path_scopes(&program.paths[effect_index], groups, player_index);
        let captured = with_program_scope(ctx, &parent_scopes, |ctx| {
            capture_program_group(game, ctx, &program.wrappers[group])
        })?;
        let accepted = if captured.valid && !reached[group] && !ctx.decision_maker.awaiting_choice()
        {
            let mut scopes = parent_scopes;
            scopes.extend(captured.scopes.iter().cloned());
            with_program_scope(ctx, &scopes, |ctx| {
                if program.is_optional[group] {
                    program.offers[group].prepare_optional_choice(game, ctx)
                } else {
                    Ok(true)
                }
            })?
        } else {
            false
        };
        groups[group][player_index] = Some(captured);
        acceptance[group][player_index] = accepted;
        return Ok(false);
    }
    Ok(true)
}

fn finish_optional_preparation(
    game: &mut GameState,
    unit: &[usize],
    program: &OptionalActionProgram,
    acceptance: &[Vec<bool>],
    limits: &mut [Option<crate::effects::DoThisLimit>],
) {
    for &index in unit {
        if let Some((group, true)) = program.markers[index]
            && let Some(limit) = limits[group].take()
            && acceptance[group].iter().any(|accepted| *accepted)
        {
            super::may::record_optional_limit(game, limit);
        }
    }
}

fn in_optional_action<T>(
    ctx: &mut ExecutionContext,
    optional: bool,
    f: impl FnOnce(&mut ExecutionContext) -> Result<T, ExecutionError>,
) -> Result<T, ExecutionError> {
    let previous = ctx.optional_action;
    ctx.optional_action |= optional;
    let result = f(ctx);
    ctx.optional_action = previous;
    result
}

/// A player binding can project an action owned by a wider cohort. Its view
/// never owns that cohort's chronological history. Optional closing markers
/// similarly bind completed children without publishing their events again.
struct PlayerActionOutcome {
    binding: EffectOutcome,
    owner: Option<EffectOutcome>,
    own_facts: Vec<crate::effect::ExecutionFact>,
}

type PreparedPlayerAction = (
    usize,
    std::collections::HashMap<crate::tag::TagKey, Vec<crate::snapshot::ObjectSnapshot>>,
    Box<dyn SimultaneousEffectProposal>,
    bool,
    Vec<usize>,
);

enum PlayerOriginalContext {
    Instruction(crate::effects::ExecutionContextCheckpoint, bool, Vec<usize>),
    DamageCohort(Vec<PreparedPlayerAction>),
}

/// A physical completion may supply several player bindings. Keep the owner
/// independent of the rows so no arbitrary player inherits the whole action.
struct CompletedPlayerActionGroup {
    owner: Option<EffectOutcome>,
    bindings: Vec<(usize, PlayerActionOutcome)>,
}
impl PlayerActionOutcome {
    fn owned(owner: EffectOutcome) -> Self {
        Self {
            binding: owner.clone(),
            owner: Some(owner),
            own_facts: Vec::new(),
        }
    }

    fn binding_only(binding: EffectOutcome, own_facts: Vec<crate::effect::ExecutionFact>) -> Self {
        Self {
            binding,
            owner: None,
            own_facts,
        }
    }
}

fn retain_optional_outcome(
    receipt: PlayerActionOutcome,
    player_index: usize,
    path: &[usize],
    selection: bool,
    program: &OptionalActionProgram,
    optional_outcomes: &mut [Vec<Vec<EffectOutcome>>],
    outcomes_by_player: &mut [Vec<EffectOutcome>],
    outcomes: &mut Vec<EffectOutcome>,
    actual_events: &mut Vec<crate::events::RawEvent>,
    owner_facts: &mut Vec<crate::effect::ExecutionFact>,
) {
    let PlayerActionOutcome {
        mut binding,
        owner,
        own_facts,
    } = receipt;
    if let Some(owner) = owner {
        actual_events.extend(owner.events);
        owner_facts.extend(owner.execution_facts);
    }
    owner_facts.extend(own_facts);
    if let Some(&group) = path.last() {
        if selection
            && program.is_optional[group]
            && program.offers[group]
                .effects
                .iter()
                .any(|effect| !super::may::is_object_selection(effect))
        {
            binding.set_value(crate::effect::OutcomeValue::None);
        }
        optional_outcomes[group][player_index].push(binding);
    } else {
        outcomes_by_player[player_index].push(binding.clone());
        outcomes.push(binding);
    }
}

/// Attach the completed action's per-player counts to every player's copy of
/// each result id the unit produced (collective metrics such as "the greatest
/// number"), keeping each player's scalar result local ("that many").
fn attach_unit_player_counts(
    unit: &[usize],
    simultaneous_effects: &[Effect],
    players: &[PlayerId],
    effect_outcomes_by_player: &mut [std::collections::HashMap<
        crate::effect::EffectId,
        EffectOutcome,
    >],
) {
    let mut result_ids = Vec::new();
    for &effect_index in unit {
        collect_result_ids(&simultaneous_effects[effect_index], &mut result_ids);
    }
    attach_result_player_counts(&result_ids, players, effect_outcomes_by_player);
}

fn attach_result_player_counts(
    result_ids: &[crate::effect::EffectId],
    players: &[PlayerId],
    effect_outcomes_by_player: &mut [std::collections::HashMap<
        crate::effect::EffectId,
        EffectOutcome,
    >],
) {
    for &id in result_ids {
        let counts = players
            .iter()
            .zip(effect_outcomes_by_player.iter())
            .filter_map(|(&player, results)| {
                results
                    .get(&id)
                    .map(|outcome| (player, outcome.count_or_zero()))
            })
            .collect::<Vec<_>>();
        for results in effect_outcomes_by_player.iter_mut() {
            if let Some(outcome) = results.get_mut(&id) {
                *outcome = outcome.clone().with_player_counts(counts.clone());
            }
        }
    }
}

/// Merge each player's local tagged-player bindings back into one context
/// map after an each-player loop. Players are visited in APNAP order so the
/// merged lists are deterministic regardless of map iteration order.
fn merge_tagged_players_by_player(
    incoming: &std::collections::HashMap<crate::tag::TagKey, Vec<PlayerId>>,
    by_player: &[std::collections::HashMap<crate::tag::TagKey, Vec<PlayerId>>],
) -> std::collections::HashMap<crate::tag::TagKey, Vec<PlayerId>> {
    let mut merged = incoming.clone();
    let mut changed: std::collections::HashMap<crate::tag::TagKey, Vec<PlayerId>> =
        std::collections::HashMap::new();
    for player_tags in by_player {
        for (tag, tagged) in player_tags {
            if incoming.get(tag) == Some(tagged) {
                continue;
            }
            let collected = changed.entry(tag.clone()).or_default();
            for player in tagged {
                if !collected.contains(player) {
                    collected.push(*player);
                }
            }
        }
    }
    merged.extend(changed);
    merged
}

fn collect_result_ids(effect: &Effect, ids: &mut Vec<crate::effect::EffectId>) {
    if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>()
        && !ids.contains(&with_id.id)
    {
        ids.push(with_id.id);
    }
    effect
        .0
        .visit_child_effects(&mut |child| collect_result_ids(child, ids));
}

fn merge_tagged_object_sets(
    aggregate: &mut std::collections::HashMap<
        crate::tag::TagKey,
        Vec<crate::snapshot::ObjectSnapshot>,
    >,
    current: &std::collections::HashMap<crate::tag::TagKey, Vec<crate::snapshot::ObjectSnapshot>>,
) {
    for (tag, snapshots) in current {
        let collected = aggregate.entry(tag.clone()).or_default();
        for snapshot in snapshots {
            if !collected
                .iter()
                .any(|existing| existing.stable_id == snapshot.stable_id)
            {
                collected.push(snapshot.clone());
            }
        }
    }
}

fn capture_player_tagged_object_deltas(
    baseline: &std::collections::HashMap<crate::tag::TagKey, Vec<crate::snapshot::ObjectSnapshot>>,
    current: &std::collections::HashMap<crate::tag::TagKey, Vec<crate::snapshot::ObjectSnapshot>>,
    player_tags: &mut std::collections::HashMap<
        crate::tag::TagKey,
        Vec<crate::snapshot::ObjectSnapshot>,
    >,
    loop_local_tags: &mut std::collections::HashSet<crate::tag::TagKey>,
) {
    for (tag, snapshots) in current {
        let prior = baseline.get(tag);
        let additions = snapshots.iter().filter(|snapshot| {
            !prior.is_some_and(|prior| {
                prior
                    .iter()
                    .any(|existing| existing.stable_id == snapshot.stable_id)
            })
        });
        let destination = player_tags.entry(tag.clone()).or_default();
        let mut changed = false;
        for snapshot in additions {
            if !destination
                .iter()
                .any(|existing| existing.stable_id == snapshot.stable_id)
            {
                destination.push(snapshot.clone());
                changed = true;
            }
        }
        if changed {
            loop_local_tags.insert(tag.clone());
        }
    }
}

fn apply_player_tagged_object_partition(
    tagged_objects: &mut std::collections::HashMap<
        crate::tag::TagKey,
        Vec<crate::snapshot::ObjectSnapshot>,
    >,
    player_tags: &std::collections::HashMap<
        crate::tag::TagKey,
        Vec<crate::snapshot::ObjectSnapshot>,
    >,
    loop_local_tags: &std::collections::HashSet<crate::tag::TagKey>,
) {
    for tag in loop_local_tags {
        tagged_objects.remove(tag);
        if let Some(snapshots) = player_tags.get(tag) {
            tagged_objects.insert(tag.clone(), snapshots.clone());
        }
    }
}

/// "you may have each other player gain 5 life rather than pay this spell's
/// mana cost": a per-player body made only of cost-executable effects is
/// itself payable. Each participant's part is checked when it is executed,
/// because the iterated player is bound only inside the loop.
impl crate::effects::CostExecutableEffect for ForPlayersEffect {
    fn execute_payment_with_outputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        execute_player_program_with_outputs(
            self,
            game,
            ctx,
            crate::effects::EffectExecutionPurpose::Payment,
        )
    }

    fn payment_bindings_are_owned_by_children(&self) -> bool {
        true
    }

    fn can_execute_as_cost(
        &self,
        _game: &GameState,
        _source: crate::ids::ObjectId,
        _controller: PlayerId,
    ) -> Result<(), crate::effects::CostValidationError> {
        Ok(())
    }
}

impl ForPlayersEffect {
    pub(crate) fn selected_players(
        &self,
        game: &GameState,
        ctx: &ExecutionContext,
    ) -> Result<Vec<PlayerId>, ExecutionError> {
        if self.filter.is_opponents_attacking_event_defender()
            && !ctx.triggering_event.as_ref().is_some_and(|event| {
                event
                    .downcast::<crate::events::PlayerAttackDeclarationEvent>()
                    .is_some()
            })
        {
            return Err(ExecutionError::UnresolvableValue(
                "attacking-player reward has no captured attacked player".into(),
            ));
        }
        let filter_ctx = ctx.filter_context(game);

        // Iterate over all players that match the filter
        let mut players: Vec<PlayerId> = game
            .players
            .iter()
            .filter(|p| p.is_in_game())
            .filter(|p| player_filter_matches_game(&self.filter, p.id, game, &filter_ctx))
            .map(|p| p.id)
            .collect();

        let first_player = if self.starting_with_controller {
            ctx.controller
        } else {
            game.turn.active_player
        };
        Ok(order_selected_players_from(game, players, first_player))
    }

    pub(crate) fn prepare_draw_continuation(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<ForPlayersDrawProgress, ExecutionError> {
        let players = self.selected_players(game, ctx)?;
        if players.is_empty() {
            return Ok(ForPlayersDrawProgress {
                prefix: crate::effects::CompletedEffectOutputs::aggregate_only(
                    EffectOutcome::count(0),
                ),
                resume: None,
            });
        }
        let (progress, prefix_outputs) =
            if self.sequential || self.starting_with_controller || self.stop_after_first_happened {
                ForPlayersSequentialState::new(self, players, ctx)
                    .run_with_outputs(game, ctx, true)?
            } else {
                let Some(state) = ForPlayersActionState::new(self, players, game, ctx)? else {
                    return Ok(ForPlayersDrawProgress {
                        prefix: crate::effects::CompletedEffectOutputs::aggregate_only(
                            EffectOutcome::count(0),
                        ),
                        resume: None,
                    });
                };
                state.run_with_outputs(game, ctx, true)?
            };
        match progress {
            ActionRun::Complete(prefix) => {
                let mut outputs = crate::effects::CompletedEffectOutputs::aggregate_only(prefix);
                outputs.retain_batch_children(prefix_outputs);
                Ok(ForPlayersDrawProgress {
                    prefix: outputs,
                    resume: None,
                })
            }
            ActionRun::Paused { prefix, state } => Ok(ForPlayersDrawProgress {
                prefix: crate::effects::CompletedEffectOutputs::aggregate_only(prefix),
                resume: Some(ForPlayersDrawContinuation {
                    state,
                    prefix_outputs,
                    context: crate::effects::ExecutionContextCheckpoint::capture(ctx),
                }),
            }),
        }
    }

    fn execute_players_for_purpose(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        purpose: crate::effects::EffectExecutionPurpose,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        let players = self.selected_players(game, ctx)?;
        self.execute_selected_players_for_purpose(players, game, ctx, purpose)
    }

    fn execute_selected_players_with_outputs(
        &self,
        players: Vec<PlayerId>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        self.execute_selected_players_for_purpose(
            players,
            game,
            ctx,
            crate::effects::EffectExecutionPurpose::Action,
        )
    }

    fn execute_selected_players_for_purpose(
        &self,
        players: Vec<PlayerId>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        purpose: crate::effects::EffectExecutionPurpose,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        if players.is_empty() {
            return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                EffectOutcome::count(0),
            ));
        }

        let (progress, retained_outputs) =
            if self.sequential || self.starting_with_controller || self.stop_after_first_happened {
                ForPlayersSequentialState::new(self, players, ctx)
                    .with_purpose(purpose)
                    .run_with_outputs(game, ctx, false)?
            } else {
                let Some(state) = ForPlayersActionState::new(self, players, game, ctx)? else {
                    return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::count(0),
                    ));
                };
                state
                    .with_purpose(purpose)
                    .run_with_outputs(game, ctx, false)?
            };
        match progress {
            ActionRun::Complete(outcome) => {
                // Counts, optional markers and stored results still have
                // aggregate-only owners. Retain known child projections
                // without claiming that these cover the whole player action.
                let mut outputs = crate::effects::CompletedEffectOutputs::aggregate_only(outcome);
                for child in retained_outputs {
                    outputs = outputs.append_owned_child(child);
                }
                Ok(outputs)
            }
            ActionRun::Paused { .. } => Err(ExecutionError::InternalError(
                "native player program unexpectedly paused at a draw".into(),
            )),
        }
    }
}

/// An explicit list of participant occurrences composes the same action
/// iterator as a player-filter instruction. Repeated participants retain
/// separate choice/result frames rather than being collapsed into a set.
#[derive(Debug, Clone)]
struct PlayerOccurrences {
    effects: Vec<Effect>,
    players: Vec<PlayerId>,
}

impl EffectExecutor for PlayerOccurrences {
    fn visit_child_effects(&self, visitor: &mut dyn FnMut(&Effect)) {
        for effect in &self.effects {
            visitor(effect);
        }
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        self.execute_with_outputs(game, ctx)
            .map(crate::effects::CompletedEffectOutputs::into_outcome)
    }

    fn execute_with_outputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        crate::effects::tokens::execute_resource_transaction_with_pending_value(
            game,
            ctx,
            || {
                crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::with_objects(
                    Vec::new(),
                ))
            },
            |game, ctx| {
                let order = order_selected_players_from(
                    game,
                    self.players.clone(),
                    game.turn.active_player,
                );
                let mut players = self
                    .players
                    .iter()
                    .copied()
                    .filter(|player| {
                        game.player(*player)
                            .is_some_and(|player| player.is_in_game())
                    })
                    .collect::<Vec<_>>();
                // Stable sorting preserves the order of a player's occurrences.
                players
                    .sort_by_key(|player| order.iter().position(|candidate| candidate == player));
                ForPlayersEffect::new(PlayerFilter::Any, self.effects.clone())
                    .execute_selected_players_with_outputs(players, game, ctx)
            },
        )
    }
}

pub(crate) fn execute_player_occurrences_with_outputs(
    effects: &[Effect],
    players: Vec<PlayerId>,
    game: &mut GameState,
    ctx: &mut ExecutionContext,
) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
    PlayerOccurrences {
        effects: effects.to_vec(),
        players,
    }
    .execute_child_with_outputs(game, ctx)
}

type PlayerObjectTags =
    std::collections::HashMap<crate::tag::TagKey, Vec<crate::snapshot::ObjectSnapshot>>;
type PlayerResultFrame = std::collections::HashMap<crate::effect::EffectId, EffectOutcome>;
type PlayerTagBindings = std::collections::HashMap<crate::tag::TagKey, Vec<PlayerId>>;

pub(crate) struct ForPlayersDrawProgress {
    pub(crate) prefix: crate::effects::CompletedEffectOutputs,
    pub(crate) resume: Option<ForPlayersDrawContinuation>,
}
impl ForPlayersDrawProgress {
    pub(crate) fn into_commit(
        self,
    ) -> crate::effects::SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs> {
        crate::effects::SimultaneousEffectCommit {
            outcome: self.prefix,
            completion: self.resume.map(|resume| {
                Box::new(PlayerProgramCompletion(resume))
                    as Box<dyn crate::effects::SimultaneousEffectCompletion>
            }),
        }
    }
}
struct PlayerProgramCompletion(ForPlayersDrawContinuation);
impl crate::effects::SimultaneousEffectCompletion for PlayerProgramCompletion {
    // This owner retains one reached player instruction. Enclosing replacement
    // additions are retained by the caller, not by the player cursor.
    fn original_phase_status(&self) -> crate::effects::OriginalPhaseStatus {
        crate::effects::OriginalPhaseStatus::Retained
    }

    fn complete_original_phase_with_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: EffectOutcome,
    ) -> Result<
        crate::effects::SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>,
        ExecutionError,
    > {
        self.complete_original_phase_from_outputs(
            game,
            ctx,
            crate::effects::CompletedEffectOutputs::aggregate_only(original),
        )
    }

    fn complete_original_phase_from_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: crate::effects::CompletedEffectOutputs,
    ) -> Result<
        crate::effects::SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>,
        ExecutionError,
    > {
        super::complete_authored_original_subtree_with_outputs(game, ctx, self, original)
    }

    fn observe_original(
        &mut self,
        _game: &mut GameState,
        _ctx: &mut ExecutionContext,
        original: &mut EffectOutcome,
    ) -> Result<(), ExecutionError> {
        self.0.observe_prefix(&original.events);
        Ok(())
    }

    fn freeze(&mut self, game: &mut GameState) -> Result<(), ExecutionError> {
        self.0.freeze(game)
    }
    fn prepare_draw_boundary_with_outputs(
        self: Box<Self>,
        _game: &mut GameState,
        _ctx: &mut ExecutionContext,
        original: EffectOutcome,
    ) -> Result<
        crate::effects::SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>,
        ExecutionError,
    > {
        Ok(crate::effects::SimultaneousEffectCommit {
            outcome: crate::effects::CompletedEffectOutputs::aggregate_only(original),
            completion: Some(self),
        })
    }
    fn complete(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: EffectOutcome,
    ) -> Result<EffectOutcome, ExecutionError> {
        self.complete_with_outputs(game, ctx, original)
            .map(crate::effects::CompletedEffectOutputs::into_outcome)
    }
    fn complete_with_outputs(
        mut self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: EffectOutcome,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        self.0.observe_prefix(&original.events);
        self.0.resume_outputs(game, ctx)
    }
}

pub(crate) struct ForPlayersDrawContinuation {
    state: ForPlayersContinuationState,
    context: crate::effects::ExecutionContextCheckpoint,
    prefix_outputs: Vec<crate::effects::CompletedEffectOutputs>,
}
impl ForPlayersDrawContinuation {
    pub(crate) fn freeze(&mut self, game: &mut GameState) -> Result<(), ExecutionError> {
        if let ForPlayersContinuationState::Action(state) = &mut self.state
            && let Some(pending) = &mut state.pending_program_draw
        {
            pending.continuation.freeze(game)?;
        }
        if let ForPlayersContinuationState::Action(state) = &mut self.state
            && let Some(batch) = &mut state.pending_batch_draw
        {
            game.freeze_completed_entry_events(
                batch
                    .outcomes
                    .iter_mut()
                    .flat_map(|(_, receipt, _)| receipt.outcome.outcome.events.iter_mut()),
            )?;
            for (_, receipt, _) in &mut batch.outcomes {
                if let Some(completion) = &mut receipt.completion {
                    completion.freeze(game)?;
                }
            }
        }
        let (outcomes, player_outcomes, child) = match &mut self.state {
            ForPlayersContinuationState::Action(state) => (
                &mut state.outcomes,
                &mut state.outcomes_by_player,
                state
                    .pending_unit_draw
                    .as_mut()
                    .and_then(|pending| pending.child.as_mut()),
            ),
            ForPlayersContinuationState::Sequential(state) => (
                &mut state.outcomes,
                &mut state.outcomes_by_player,
                state.pending_child.as_mut(),
            ),
        };
        for outcome in outcomes
            .iter_mut()
            .chain(player_outcomes.iter_mut().flatten())
        {
            crate::effects::outcome_recording::complete_outcome(
                game,
                None,
                None,
                outcome,
                Vec::new(),
            );
        }
        if let Some(child) = child {
            child.freeze(game)?;
        }
        Ok(())
    }

    pub(crate) fn observe_prefix(&mut self, observed: &[crate::triggers::TriggerEvent]) {
        let outcomes = match &mut self.state {
            ForPlayersContinuationState::Action(state) => {
                crate::effects::composition::inherit_observed_events(
                    &mut state.actual_events,
                    observed,
                );
                for outcomes in &mut state.outcomes_by_player {
                    for outcome in outcomes {
                        crate::effects::composition::inherit_original_observations(
                            outcome, observed,
                        );
                    }
                }
                if let Some(child) = state
                    .pending_unit_draw
                    .as_mut()
                    .and_then(|pending| pending.child.as_mut())
                {
                    child.observe_prefix(observed);
                }
                if let Some(pending) = &mut state.pending_program_draw {
                    pending.continuation.observe_prefix(observed);
                }
                if let Some(batch) = &mut state.pending_batch_draw {
                    for (_, receipt, _) in &mut batch.outcomes {
                        crate::effects::composition::inherit_original_observations(
                            &mut receipt.outcome.outcome,
                            observed,
                        );
                        receipt.outcome.synchronize_observations();
                    }
                }
                &mut state.outcomes
            }
            ForPlayersContinuationState::Sequential(state) => {
                for outcomes in &mut state.outcomes_by_player {
                    for outcome in outcomes {
                        crate::effects::composition::inherit_original_observations(
                            outcome, observed,
                        );
                    }
                }
                if let Some(child) = &mut state.pending_child {
                    child.observe_prefix(observed);
                }
                &mut state.outcomes
            }
        };
        for outcome in outcomes {
            crate::effects::composition::inherit_original_observations(outcome, observed);
        }
    }

    pub(crate) fn resume(
        self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        self.resume_outputs(game, ctx)
            .map(crate::effects::CompletedEffectOutputs::into_outcome)
    }
    pub(crate) fn resume_outputs(
        self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        self.context.restore_preserving_resolution_control(ctx);
        let (progress, children) = match self.state {
            ForPlayersContinuationState::Action(state) => {
                state.run_with_outputs(game, ctx, false)?
            }
            ForPlayersContinuationState::Sequential(state) => {
                state.run_with_outputs(game, ctx, false)?
            }
        };
        match progress {
            ActionRun::Complete(outcome) => {
                if ctx.decision_maker.awaiting_choice() {
                    return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::count(0),
                    ));
                }
                // The root already contains its prefix and every child once.
                // Keep prior packets in the genuine continuation until its completion.
                let mut outputs = crate::effects::CompletedEffectOutputs::aggregate_only(outcome);
                outputs.retain_batch_children(self.prefix_outputs.into_iter().chain(children));
                Ok(outputs)
            }
            ActionRun::Paused { .. } => Err(ExecutionError::InternalError(
                "resumed player program paused twice".into(),
            )),
        }
    }
}

struct PlayerUnitDraw {
    unit_index: usize,
    player_position: usize,
    effect_position: usize,
    participant_indices: Vec<usize>,
    unit_tags: PlayerObjectTags,
    accumulated_tags: PlayerObjectTags,
    child: Option<Box<dyn crate::effects::replacement::ReplacementResume>>,
}
type OriginalBatchOutcome = (
    Option<usize>,
    crate::effects::SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>,
    Option<PlayerOriginalContext>,
);
struct PendingBatchDraw {
    unit_index: usize,
    outcomes: Vec<OriginalBatchOutcome>,
    tags: PlayerObjectTags,
}
struct PendingProgramDraw {
    unit_index: usize,
    metadata: Vec<(usize, PlayerObjectTags)>,
    parent: crate::effects::ExecutionContextCheckpoint,
    tags: PlayerObjectTags,
    continuation: super::action_program::ActionProgramsContinuation,
}
enum ForPlayersContinuationState {
    Action(Box<ForPlayersActionState>),
    Sequential(Box<ForPlayersSequentialState>),
}
enum ActionRun {
    Complete(EffectOutcome),
    Paused {
        prefix: EffectOutcome,
        state: ForPlayersContinuationState,
    },
}

/// The authored player-major variant has its own cursor. It shares the same
/// child continuation contract but preserves its existing tag-reset and
/// stop-after-first-success semantics instead of becoming action-major.
struct ForPlayersSequentialState {
    effect: ForPlayersEffect,
    purpose: crate::effects::EffectExecutionPurpose,
    inherited_x: Option<u32>,
    players: Vec<PlayerId>,
    outcomes: Vec<EffectOutcome>,
    outcomes_by_player: Vec<Vec<EffectOutcome>>,
    incoming_tags: PlayerObjectTags,
    completed_tags: PlayerObjectTags,
    player_index: usize,
    effect_index: usize,
    player_started: bool,
    pending_child: Option<Box<dyn crate::effects::replacement::ReplacementResume>>,
}
impl ForPlayersSequentialState {
    fn new(effect: &ForPlayersEffect, players: Vec<PlayerId>, ctx: &ExecutionContext) -> Self {
        let outcomes_by_player = vec![Vec::new(); players.len()];
        Self {
            effect: effect.clone(),
            purpose: crate::effects::EffectExecutionPurpose::Action,
            inherited_x: ctx.x_value,
            players,
            outcomes: Vec::new(),
            outcomes_by_player,
            incoming_tags: ctx.tagged_objects.clone(),
            completed_tags: Default::default(),
            player_index: 0,
            effect_index: 0,
            player_started: false,
            pending_child: None,
        }
    }
    fn with_purpose(mut self, purpose: crate::effects::EffectExecutionPurpose) -> Self {
        self.purpose = purpose;
        self
    }

    fn run_with_outputs(
        mut self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        defer_draws: bool,
    ) -> Result<(ActionRun, Vec<crate::effects::CompletedEffectOutputs>), ExecutionError> {
        let mut retained_outputs = Vec::new();
        while self.player_index < self.players.len() && !ctx.resolution_stopped() {
            let player = self.players[self.player_index];
            if !self.player_started {
                if matches!(
                    self.purpose,
                    crate::effects::EffectExecutionPurpose::Payment
                ) {
                    ctx.x_value = self.inherited_x;
                }
                if self.effect.sequential {
                    ctx.tagged_objects = self.incoming_tags.clone();
                }
                self.player_started = true;
            }
            while self.effect_index < self.effect.effects.len() && !ctx.resolution_stopped() {
                let effect = self.effect.effects[self.effect_index].clone();
                if defer_draws || game.effect_store.per_event_trigger_matching {
                    crate::effects::capture_triggers_before_added_program(
                        game,
                        ctx,
                        Some(&effect),
                        self.outcomes
                            .iter_mut()
                            .flat_map(|outcome| outcome.events.iter_mut()),
                    )?;
                }
                let pending = self.pending_child.take();
                let prepared = ctx.with_temp_iterated_player(Some(player), |ctx| {
                    if let Some(pending) = pending {
                        let outputs =
                            crate::effects::replacement::resume_replacement_child_with_outputs(
                                game, ctx, pending,
                            )?;
                        Ok(crate::effects::replacement::PreparedReplacementChild {
                            prefix: outputs,
                            resume: None,
                        })
                    } else if defer_draws {
                        crate::effects::replacement::prepare_replacement_child(game, ctx, &effect)
                    } else {
                        let outputs = self.purpose.execute(game, &effect, ctx)?;
                        Ok(crate::effects::replacement::PreparedReplacementChild {
                            prefix: outputs,
                            resume: None,
                        })
                    }
                })?;
                if let Some(pending) = prepared.resume {
                    let mut partial = prepared.prefix.outcome;
                    crate::effects::capture_triggers_before_added_program(
                        game,
                        ctx,
                        None,
                        self.outcomes
                            .iter_mut()
                            .flat_map(|outcome| outcome.events.iter_mut())
                            .chain(partial.events.iter_mut()),
                    )?;
                    let complete_prefix = finish_players_outcome(
                        &self.effect,
                        self.players.clone(),
                        self.outcomes.clone(),
                        self.outcomes_by_player.clone(),
                        Vec::new(),
                        None,
                    )?;
                    self.pending_child = Some(pending);
                    return Ok((
                        ActionRun::Paused {
                            prefix: EffectOutcome::aggregate([complete_prefix, partial]),
                            state: ForPlayersContinuationState::Sequential(Box::new(self)),
                        },
                        retained_outputs,
                    ));
                }
                self.outcomes_by_player[self.player_index].push(prepared.prefix.outcome.clone());
                self.outcomes.push(prepared.prefix.outcome.clone());
                retained_outputs.push(prepared.prefix);
                self.effect_index += 1;
                if ctx.decision_maker.awaiting_choice() {
                    return Ok((
                        ActionRun::Complete(EffectOutcome::count(0)),
                        retained_outputs,
                    ));
                }
            }
            let iteration = EffectOutcome::aggregate_summing_counts(
                self.outcomes_by_player[self.player_index].iter().cloned(),
            );
            if self.effect.sequential {
                merge_tagged_object_sets(&mut self.completed_tags, &ctx.tagged_objects);
            }
            if self.effect.stop_after_first_happened && iteration.something_happened() {
                break;
            }
            self.player_index += 1;
            self.effect_index = 0;
            self.player_started = false;
        }
        if self.effect.sequential {
            ctx.tagged_objects = self.completed_tags;
        }
        finish_players_outcome(
            &self.effect,
            self.players,
            self.outcomes,
            self.outcomes_by_player,
            Vec::new(),
            None,
        )
        .map(|outcome| (ActionRun::Complete(outcome), retained_outputs))
    }
}

/// Owned action-iterator state. Player-local results, tags, optional groups and
/// APNAP preparation survive without reconstructing them from merged counts.
struct ForPlayersActionState {
    effect: ForPlayersEffect,
    purpose: crate::effects::EffectExecutionPurpose,
    payment_x_by_player: Vec<Option<u32>>,
    players: Vec<PlayerId>,
    outcomes: Vec<EffectOutcome>,
    outcomes_by_player: Vec<Vec<EffectOutcome>>,
    optional_program: OptionalActionProgram,
    optional_acceptance: Vec<Vec<bool>>,
    optional_outcomes: Vec<Vec<Vec<EffectOutcome>>>,
    program_groups: Vec<Vec<Option<CapturedProgramGroup>>>,
    optional_initialized: Vec<bool>,
    optional_limits: Vec<Option<crate::effects::DoThisLimit>>,
    optional_limit_reached: Vec<bool>,
    actual_events: Vec<crate::events::RawEvent>,
    owner_facts: Vec<crate::effect::ExecutionFact>,
    shared_action_players: std::collections::HashMap<usize, Vec<PlayerId>>,
    units: Vec<Vec<usize>>,
    next_unit: usize,
    pending_unit_draw: Option<PlayerUnitDraw>,
    pending_batch_draw: Option<PendingBatchDraw>,
    pending_program_draw: Option<PendingProgramDraw>,
    tagged_objects_by_player: Vec<PlayerObjectTags>,
    loop_local_tags: std::collections::HashSet<crate::tag::TagKey>,
    effect_outcomes_by_player: Vec<PlayerResultFrame>,
    incoming_tagged_players: PlayerTagBindings,
    tagged_players_by_player: Vec<PlayerTagBindings>,
}
impl ForPlayersActionState {
    fn new(
        effect: &ForPlayersEffect,
        players: Vec<PlayerId>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<Option<Self>, ExecutionError> {
        let outcomes = Vec::new();
        let outcomes_by_player = vec![Vec::new(); players.len()];
        let payment_x_by_player = vec![ctx.x_value; players.len()];

        // The simultaneous protocol works on printed actions, so coordinated
        // sequence wrappers are unwrapped first. The sequential branch below
        // keeps `effect.effects` as authored: nesting there is already executed in
        // order and carries no unit grouping.
        let optional_program =
            OptionalActionProgram::new(&effect.effects, !ctx.target_assignments.is_empty());
        let simultaneous_effects = &optional_program.effects;
        let optional_acceptance = vec![vec![false; players.len()]; optional_program.offers.len()];
        let optional_outcomes =
            vec![vec![Vec::<EffectOutcome>::new(); players.len()]; optional_program.offers.len()];
        let program_groups: Vec<Vec<Option<CapturedProgramGroup>>> =
            vec![vec![None; players.len()]; optional_program.offers.len()];
        let optional_initialized = vec![false; optional_program.offers.len()];
        let optional_limits = vec![None; optional_program.offers.len()];
        let optional_limit_reached = vec![false; optional_program.offers.len()];
        let actual_events = Vec::new();
        let owner_facts = Vec::new();

        // CR 608.2f: choices for a simultaneous each-player action are
        // made in APNAP order against the pre-action game state, then the
        // whole action commits as one transaction. Decisions (including
        // read-only chooser effects that tag the execution context) run
        // player-major so one player's tags feed that player's own
        // proposal without leaking into the next player's pass; game
        // mutations are deferred to the batched commit below.
        let shared_action_players =
            twohg_shared_action_players(game, ctx, &simultaneous_effects, &players)?;
        if ctx.decision_maker.awaiting_choice() {
            return Ok(None);
        }

        // CR 608.2e: finish each printed action for every player before
        // beginning the next. A read-only chooser effect is not a printed
        // action of its own — it feeds the effect that follows it — so
        // effects are grouped into action units: any run of read-only
        // effects plus the next mutating effect. Within one unit, choices
        // and proposal preparation happen player-major in APNAP order
        // against the pre-action state (CR 608.2f, 101.4), keeping each
        // player's context tags scoped to their own proposal; the unit
        // then commits as one batch before the next unit begins.
        let units = super::action_units::partition_action_units(
            simultaneous_effects,
            |index| optional_program.markers[index].map(|(_, begin)| begin),
            |previous, next| optional_program.paths[previous] == optional_program.paths[next],
        );

        let tagged_objects_by_player = vec![
            std::collections::HashMap::<
                crate::tag::TagKey,
                Vec<crate::snapshot::ObjectSnapshot>,
            >::new();
            players.len()
        ];
        let loop_local_tags = std::collections::HashSet::<crate::tag::TagKey>::new();
        // A later printed action can reference the outcome of an earlier
        // action for this same player ("reveal that many"). Keep those
        // bindings per player just like the affected-object collections.
        let effect_outcomes_by_player = vec![ctx.effect_outcomes.clone(); players.len()];
        // Player bindings ("the chosen opponent", "that player") made by
        // one player's action belong to that player's later actions only.
        let incoming_tagged_players = ctx.tagged_players.clone();
        let tagged_players_by_player = vec![incoming_tagged_players.clone(); players.len()];

        Ok(Some(Self {
            effect: effect.clone(),
            purpose: crate::effects::EffectExecutionPurpose::Action,
            payment_x_by_player,
            next_unit: 0,
            pending_unit_draw: None,
            pending_batch_draw: None,
            pending_program_draw: None,
            players,
            outcomes,
            outcomes_by_player,
            optional_program,
            optional_acceptance,
            optional_outcomes,
            program_groups,
            optional_initialized,
            optional_limits,
            optional_limit_reached,
            actual_events,
            owner_facts,
            shared_action_players,
            units,
            tagged_objects_by_player,
            loop_local_tags,
            effect_outcomes_by_player,
            incoming_tagged_players,
            tagged_players_by_player,
        }))
    }

    fn with_purpose(mut self, purpose: crate::effects::EffectExecutionPurpose) -> Self {
        self.purpose = purpose;
        self
    }

    fn run_with_outputs(
        self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        defer_draws: bool,
    ) -> Result<(ActionRun, Vec<crate::effects::CompletedEffectOutputs>), ExecutionError> {
        let mut retained_outputs = Vec::new();
        let Self {
            effect,
            purpose,
            mut payment_x_by_player,
            players,
            mut outcomes,
            mut outcomes_by_player,
            optional_program,
            mut optional_acceptance,
            mut optional_outcomes,
            mut program_groups,
            mut optional_initialized,
            mut optional_limits,
            mut optional_limit_reached,
            mut actual_events,
            mut owner_facts,
            shared_action_players,
            units,
            mut next_unit,
            mut pending_unit_draw,
            mut pending_batch_draw,
            mut pending_program_draw,
            mut tagged_objects_by_player,
            mut loop_local_tags,
            mut effect_outcomes_by_player,
            incoming_tagged_players,
            mut tagged_players_by_player,
        } = self;
        let simultaneous_effects = &optional_program.effects;
        while next_unit < units.len() && !ctx.resolution_stopped() {
            let unit_index = next_unit;
            let unit = units[next_unit].clone();
            next_unit += 1;
            if !actual_events.is_empty()
                && (defer_draws || game.effect_store.per_event_trigger_matching)
            {
                crate::effects::capture_triggers_before_added_program(
                    game,
                    ctx,
                    unit.first().map(|index| &simultaneous_effects[*index]),
                    actual_events.iter_mut(),
                )?;
            }
            if let Some(pending) = pending_program_draw.take() {
                if pending.unit_index != unit_index {
                    return Err(ExecutionError::InternalError(
                        "selected program lost its player action unit".into(),
                    ));
                }
                let PendingProgramDraw {
                    metadata,
                    parent,
                    tags: pre_unit_tagged_objects,
                    continuation,
                    ..
                } = pending;
                parent.restore_ref_preserving_resolution_control(ctx);
                let Some(completed) = continuation.resume(game, ctx)? else {
                    return Ok((
                        ActionRun::Complete(EffectOutcome::count(0)),
                        retained_outputs,
                    ));
                };
                actual_events.extend(completed.events);
                owner_facts.extend(completed.facts);
                retained_outputs.extend(completed.shared);
                let mut accumulated_tags = pre_unit_tagged_objects.clone();
                let mut last_results = None;
                for ((player_index, baseline), participant) in
                    metadata.into_iter().zip(completed.participants)
                {
                    participant
                        .context
                        .restore_ref_preserving_resolution_control(ctx);
                    effect_outcomes_by_player[player_index] = ctx.effect_outcomes.clone();
                    tagged_players_by_player[player_index] = ctx.tagged_players.clone();
                    capture_player_tagged_object_deltas(
                        &baseline,
                        &ctx.tagged_objects,
                        &mut tagged_objects_by_player[player_index],
                        &mut loop_local_tags,
                    );
                    merge_tagged_object_sets(&mut accumulated_tags, &ctx.tagged_objects);
                    last_results = Some(ctx.effect_outcomes.clone());
                    let outcome = participant.outputs.outcome.clone();
                    retained_outputs.push(participant.outputs);
                    let path = &optional_program.paths[*unit.last().expect("action unit")];
                    retain_optional_outcome(
                        PlayerActionOutcome::binding_only(outcome, Vec::new()),
                        player_index,
                        path,
                        false,
                        &optional_program,
                        &mut optional_outcomes,
                        &mut outcomes_by_player,
                        &mut outcomes,
                        &mut actual_events,
                        &mut owner_facts,
                    );
                }
                parent.restore_ref_preserving_resolution_control(ctx);
                ctx.tagged_objects = accumulated_tags;
                if let Some(results) = last_results {
                    ctx.effect_outcomes = results;
                }
                attach_unit_player_counts(
                    &unit,
                    &simultaneous_effects,
                    &players,
                    &mut effect_outcomes_by_player,
                );
                continue;
            }
            let mut resuming = pending_unit_draw.take();
            if resuming
                .as_ref()
                .is_some_and(|resume| resume.unit_index != unit_index)
            {
                return Err(ExecutionError::InternalError(
                    "player draw continuation lost its action unit".into(),
                ));
            }
            let path = &optional_program.paths[unit[0]];
            if let Some((group, false)) = optional_program.markers[unit[0]] {
                let marker_tags = ctx.tagged_objects.clone();
                let mut completed_tags = marker_tags.clone();
                for (player_index, _) in players.iter().enumerate() {
                    if !path
                        .iter()
                        .all(|&parent| optional_acceptance[parent][player_index])
                    {
                        continue;
                    }
                    ctx.tagged_objects = marker_tags.clone();
                    apply_player_tagged_object_partition(
                        &mut ctx.tagged_objects,
                        &tagged_objects_by_player[player_index],
                        &loop_local_tags,
                    );
                    ctx.tagged_players = tagged_players_by_player[player_index].clone();
                    let tag_baseline = ctx.tagged_objects.clone();
                    ctx.effect_outcomes = effect_outcomes_by_player[player_index].clone();
                    if matches!(purpose, crate::effects::EffectExecutionPurpose::Payment) {
                        ctx.x_value = payment_x_by_player[player_index];
                    }
                    let captured = program_groups[group][player_index].as_ref();
                    let (outcome, own_facts) = if captured.is_some_and(|group| !group.valid) {
                        (
                            EffectOutcome::target_invalid(),
                            vec![crate::effect::ExecutionFact::TargetInvalid],
                        )
                    } else if optional_acceptance[group][player_index] {
                        let outcome = EffectOutcome::aggregate(std::mem::take(
                            &mut optional_outcomes[group][player_index],
                        ));
                        if optional_program.is_optional[group] {
                            (
                                outcome.with_execution_fact(crate::effect::ExecutionFact::Accepted),
                                vec![crate::effect::ExecutionFact::Accepted],
                            )
                        } else {
                            (outcome, Vec::new())
                        }
                    } else {
                        (
                            EffectOutcome::declined(),
                            vec![crate::effect::ExecutionFact::Declined],
                        )
                    };
                    if let Some(captured) = captured {
                        let parents = program_path_scopes(path, &program_groups, player_index);
                        with_program_scope(ctx, &parents, |ctx| {
                            finish_program_scope(game, ctx, &captured.scopes, &outcome)
                        });
                    }
                    capture_player_tagged_object_deltas(
                        &tag_baseline,
                        &ctx.tagged_objects,
                        &mut tagged_objects_by_player[player_index],
                        &mut loop_local_tags,
                    );
                    merge_tagged_object_sets(&mut completed_tags, &ctx.tagged_objects);
                    tagged_players_by_player[player_index] = ctx.tagged_players.clone();
                    // Match WithId's nested same-id precedence: remove the
                    // previous result at begin, preserve a child-produced
                    // result, otherwise store this completed optional result.
                    for id in optional_program.outcome_ids[group].iter().rev() {
                        ctx.effect_outcomes
                            .entry(*id)
                            .or_insert_with(|| outcome.clone());
                    }
                    effect_outcomes_by_player[player_index] = ctx.effect_outcomes.clone();
                    if matches!(purpose, crate::effects::EffectExecutionPurpose::Payment) {
                        payment_x_by_player[player_index] = ctx.x_value;
                    }
                    retain_optional_outcome(
                        PlayerActionOutcome::binding_only(outcome, own_facts),
                        player_index,
                        path,
                        false,
                        &optional_program,
                        &mut optional_outcomes,
                        &mut outcomes_by_player,
                        &mut outcomes,
                        &mut actual_events,
                        &mut owner_facts,
                    );
                }
                // Only the completed optional wrapper's result ids belong
                // to this boundary. Reattaching all descendant ids would
                // overwrite metrics from earlier child action phases.
                attach_result_player_counts(
                    &optional_program.outcome_ids[group],
                    &players,
                    &mut effect_outcomes_by_player,
                );
                if let Some((last, _)) = players.iter().enumerate().rev().find(|(index, _)| {
                    path.iter()
                        .all(|&parent| optional_acceptance[parent][*index])
                }) {
                    ctx.effect_outcomes = effect_outcomes_by_player[last].clone();
                    if matches!(purpose, crate::effects::EffectExecutionPurpose::Payment) {
                        ctx.x_value = payment_x_by_player[last];
                    }
                }
                ctx.tagged_objects = completed_tags;
                continue;
            }
            let batch_resumed = pending_batch_draw.is_some();
            let (mut batch_outcomes, mut accumulated_unit_tags) = if let Some(pending) =
                pending_batch_draw.take()
            {
                if pending.unit_index != unit_index {
                    return Err(ExecutionError::InternalError(
                        "player continuation lost its original action unit".into(),
                    ));
                }
                (pending.outcomes, pending.tags)
            } else {
                let mut prepared: Vec<(
                    usize,
                    std::collections::HashMap<
                        crate::tag::TagKey,
                        Vec<crate::snapshot::ObjectSnapshot>,
                    >,
                    Box<dyn SimultaneousEffectProposal>,
                    bool,
                    Vec<usize>,
                )> = Vec::new();
                let mut prepared_programs: Vec<(
                    usize,
                    PlayerObjectTags,
                    super::action_program::ProgramParticipant,
                )> = Vec::new();
                // Read-only choices bind tags in the shared execution context.
                // Each player's proposal must see the same pre-unit context,
                // not tags left behind by an earlier player's choice. The
                // proposal owns the frozen result it needs; restore the base
                // tags again before committing so commit-time result tags can
                // accumulate normally across players.
                let pre_unit_tagged_objects = resuming
                    .as_ref()
                    .map(|resume| resume.unit_tags.clone())
                    .unwrap_or_else(|| ctx.tagged_objects.clone());
                let unit_has_mutating_effect = unit.iter().any(|effect_index| {
                    optional_program.markers[*effect_index].is_none()
                        && !simultaneous_effects[*effect_index]
                            .0
                            .is_read_only_simultaneous_player_action()
                });
                // A shared (once-per-team) effect prepares for its chosen
                // acting players in team-first APNAP order instead of every
                // seat; the whole unit follows that ordering so commit order
                // matches the pre-unit behavior.
                let unit_shared_order: Option<&Vec<PlayerId>> = unit
                    .iter()
                    .find_map(|effect_index| shared_action_players.get(effect_index));
                let unit_participants: Vec<usize> = match unit_shared_order {
                    Some(acting) => acting
                        .iter()
                        .filter_map(|player| {
                            players.iter().position(|candidate| candidate == player)
                        })
                        .collect::<Vec<usize>>(),
                    None => (0..players.len()).collect::<Vec<usize>>(),
                }
                .into_iter()
                .filter(|&index| path.iter().all(|&group| optional_acceptance[group][index]))
                .collect();

                let unit_participants = resuming
                    .as_ref()
                    .map(|resume| resume.participant_indices.clone())
                    .unwrap_or(unit_participants);

                // A printed action whose effect cannot pre-build an immutable
                // proposal (a search, a choose-then-act body, a conditional
                // follow-up, a nested choice) is performed by each player in
                // APNAP order (CR 101.4): every player finishes this action
                // before any player begins the next one (CR 608.2e), and the
                // resulting events still form one simultaneous action
                // (CR 603.2c). Each player keeps their own tag, player and
                // outcome bindings.
                let unit_runs_player_by_player = unit.iter().any(|effect_index| {
                    let effect = &simultaneous_effects[*effect_index];
                    optional_program.markers[*effect_index].is_none()
                        && !super::prepared_branch::supports_action_preparation(effect, purpose)
                        && !effect.0.is_read_only_simultaneous_player_action()
                        && !(matches!(purpose, crate::effects::EffectExecutionPurpose::Action)
                            && effect.0.supports_prepared_action_program())
                });
                let unit_has_draw = unit.iter().any(|index| {
                    optional_program.markers[*index].is_none()
                        && crate::effects::replacement::replacement_effect_contains_draw(
                            &simultaneous_effects[*index],
                        )
                }) || resuming
                    .as_ref()
                    .is_some_and(|resume| resume.child.is_some());
                let unit_is_sequential_action = unit_has_draw
                    || unit.iter().any(|index| {
                        simultaneous_effects[*index]
                            .0
                            .requires_sequential_player_actions()
                    });
                if unit_runs_player_by_player || unit_is_sequential_action || resuming.is_some() {
                    // CR 121.2c/d: each player's draws (and their replacement
                    // programs) complete before the next player's draws.
                    // Do not stamp them with a fictitious simultaneous batch.
                    let pinned_lookback = !unit_is_sequential_action
                        && crate::effects::helpers::begin_simultaneous_zone_change_lookback(game);
                    let opened_batch =
                        !unit_is_sequential_action && game.open_simultaneous_action();
                    let mut accumulated_unit_tags = resuming
                        .as_ref()
                        .map(|resume| resume.accumulated_tags.clone())
                        .unwrap_or_else(|| pre_unit_tagged_objects.clone());
                    let mut unit_error = None;
                    let mut unit_waiting = false;
                    let start_player = resuming.as_ref().map_or(0, |resume| resume.player_position);
                    for (player_position, &player_index) in
                        unit_participants.iter().enumerate().skip(start_player)
                    {
                        let player_id = players[player_index];
                        ctx.tagged_objects = pre_unit_tagged_objects.clone();
                        apply_player_tagged_object_partition(
                            &mut ctx.tagged_objects,
                            &tagged_objects_by_player[player_index],
                            &loop_local_tags,
                        );
                        ctx.effect_outcomes = effect_outcomes_by_player[player_index].clone();
                        if matches!(purpose, crate::effects::EffectExecutionPurpose::Payment) {
                            ctx.x_value = payment_x_by_player[player_index];
                        }
                        ctx.tagged_players = tagged_players_by_player[player_index].clone();
                        let pre_player_tagged_objects = ctx.tagged_objects.clone();
                        let start_effect = resuming
                            .as_ref()
                            .filter(|resume| resume.player_position == player_position)
                            .map_or(0, |resume| resume.effect_position);
                        let mut paused_child = None;
                        let mut paused_prefix = None;
                        let mut paused_effect_position = 0;
                        let result = ctx.with_temp_iterated_player(Some(player_id), |ctx| {
                            for (effect_position, &effect_index) in unit.iter().enumerate().skip(start_effect) {
                                if ctx.resolution_stopped() { break; }
                                if unit_has_draw {
                                    crate::effects::capture_triggers_before_added_program(game, ctx,
                                        Some(&simultaneous_effects[effect_index]), actual_events.iter_mut())?;
                                }
                                let path = &optional_program.paths[effect_index];
                                let resume_child = resuming.as_mut().filter(|resume| resume.player_position == player_position && resume.effect_position == effect_position).and_then(|resume| resume.child.take());
                                let execute_child = if resume_child.is_some() { true } else { prepare_optional_instruction(
                                    game,
                                    ctx,
                                    player_index,
                                    effect_index,
                                    &optional_program,
                                    &mut optional_acceptance,
                                    &mut optional_initialized,
                                    &mut optional_limits,
                                    &mut optional_limit_reached,
                                    &mut program_groups,
                                )? };
                                if ctx.decision_maker.awaiting_choice() {
                                    return Ok(());
                                }
                                if !execute_child {
                                    continue;
                                }
                                let effect = &simultaneous_effects[effect_index];
                                let outcome = in_optional_action(
                                    ctx,
                                    optional_program.path_is_optional(path),
                                    |ctx| {
                                        let scopes = program_path_scopes(
                                            path,
                                            &program_groups,
                                            player_index,
                                        );
                                        with_program_scope(ctx, &scopes, |ctx| {
                                            if let Some(resume) = resume_child {
                                                {
                    let outputs = crate::effects::replacement::resume_replacement_child_with_outputs(game, ctx, resume)?;
                    let outcome = outputs.outcome.clone();
                    retained_outputs.push(outputs);
                    Ok(outcome)
                }
                                            } else if defer_draws {
                                                let prepared = crate::effects::replacement::prepare_replacement_child(game, ctx, effect)?;
                                                if let Some(resume) = prepared.resume {
                                                    paused_effect_position = effect_position;
                                                    paused_prefix = Some(prepared.prefix.outcome.clone());
                                                    paused_child = Some(resume);
                                                }
                                                let outcome = prepared.prefix.outcome.clone();
                                                if paused_child.is_none() { retained_outputs.push(prepared.prefix); }
                                                Ok(outcome)
                                            } else {
                                                let outputs = purpose.execute(game, effect, ctx)?;
                                                let outcome = outputs.outcome.clone();
                                                retained_outputs.push(outputs);
                                                Ok(outcome)
                                            }
                                        })
                                    },
                                )?;
                                if paused_child.is_some() { return Ok(()); }
                                retain_optional_outcome(
                                    PlayerActionOutcome::owned(outcome),
                                    player_index,
                                    path,
                                    super::may::is_object_selection(effect),
                                    &optional_program,
                                    &mut optional_outcomes,
                                    &mut outcomes_by_player,
                                    &mut outcomes,
                                    &mut actual_events,
                                    &mut owner_facts,
                                );
                                if ctx.decision_maker.awaiting_choice() {
                                    break;
                                }
                            }
                            Ok::<(), ExecutionError>(())
                        });
                        if let Some(child) = paused_child {
                            result?;
                            game.close_simultaneous_action(opened_batch);
                            crate::effects::helpers::end_simultaneous_zone_change_lookback(
                                game,
                                pinned_lookback,
                            );
                            let mut partial =
                                paused_prefix.expect("paused child has a prefix receipt");
                            crate::effects::capture_triggers_before_added_program(
                                game,
                                ctx,
                                None,
                                actual_events.iter_mut().chain(partial.events.iter_mut()),
                            )?;
                            let mut prefix = finish_players_outcome(
                                &effect,
                                players.clone(),
                                outcomes.clone(),
                                outcomes_by_player.clone(),
                                actual_events.clone(),
                                Some(owner_facts.clone()),
                            )?;
                            prefix = EffectOutcome::aggregate([prefix, partial]);
                            let pending_unit_draw = Some(PlayerUnitDraw {
                                unit_index,
                                player_position,
                                effect_position: paused_effect_position,
                                participant_indices: unit_participants.clone(),
                                unit_tags: pre_unit_tagged_objects,
                                accumulated_tags: accumulated_unit_tags,
                                child: Some(child),
                            });
                            return Ok((
                                ActionRun::Paused {
                                    prefix,
                                    state: ForPlayersContinuationState::Action(Box::new(Self {
                                        next_unit: unit_index,
                                        pending_unit_draw,
                                        pending_batch_draw,
                                        pending_program_draw,
                                        effect,
                                        purpose,
                                        payment_x_by_player,
                                        players,
                                        outcomes,
                                        outcomes_by_player,
                                        optional_program,
                                        optional_acceptance,
                                        optional_outcomes,
                                        program_groups,
                                        optional_initialized,
                                        optional_limits,
                                        optional_limit_reached,
                                        actual_events,
                                        owner_facts,
                                        shared_action_players,
                                        units,
                                        tagged_objects_by_player,
                                        loop_local_tags,
                                        effect_outcomes_by_player,
                                        incoming_tagged_players,
                                        tagged_players_by_player,
                                    })),
                                },
                                retained_outputs,
                            ));
                        }
                        effect_outcomes_by_player[player_index] = ctx.effect_outcomes.clone();
                        if matches!(purpose, crate::effects::EffectExecutionPurpose::Payment) {
                            payment_x_by_player[player_index] = ctx.x_value;
                        }
                        tagged_players_by_player[player_index] = ctx.tagged_players.clone();
                        if let Err(error) = result {
                            unit_error = Some(error);
                            break;
                        }
                        capture_player_tagged_object_deltas(
                            &pre_player_tagged_objects,
                            &ctx.tagged_objects,
                            &mut tagged_objects_by_player[player_index],
                            &mut loop_local_tags,
                        );
                        merge_tagged_object_sets(&mut accumulated_unit_tags, &ctx.tagged_objects);
                        if ctx.decision_maker.awaiting_choice() {
                            unit_waiting = true;
                            break;
                        }
                    }
                    game.close_simultaneous_action(opened_batch);
                    crate::effects::helpers::end_simultaneous_zone_change_lookback(
                        game,
                        pinned_lookback,
                    );
                    if let Some(error) = unit_error {
                        ctx.tagged_objects = pre_unit_tagged_objects;
                        ctx.tagged_players = incoming_tagged_players;
                        return Err(error);
                    }
                    if unit_waiting {
                        ctx.tagged_objects = pre_unit_tagged_objects;
                        ctx.tagged_players = incoming_tagged_players;
                        return Ok((
                            ActionRun::Complete(EffectOutcome::count(0)),
                            retained_outputs,
                        ));
                    }
                    finish_optional_preparation(
                        game,
                        &unit,
                        &optional_program,
                        &optional_acceptance,
                        &mut optional_limits,
                    );
                    game.freeze_completed_entry_events(actual_events.iter_mut())?;
                    if unit_is_sequential_action {
                        crate::effects::capture_triggers_before_added_program(
                            game,
                            ctx,
                            None,
                            actual_events.iter_mut(),
                        )?;
                    }
                    ctx.tagged_objects = accumulated_unit_tags;
                    attach_unit_player_counts(
                        &unit,
                        &simultaneous_effects,
                        &players,
                        &mut effect_outcomes_by_player,
                    );
                    continue;
                }

                // Read-only producers can replace a named result tag for
                // each participant (for example, reveal the top card). Retain
                // the complete collection independently of the last player's
                // local bindings, including when that player finds no card.
                let mut readonly_unit_tags = std::collections::HashMap::new();
                for &player_index in &unit_participants {
                    let player_id = players[player_index];
                    // Read-only selections also produce player-local tags.
                    // A later participant must not reveal or otherwise consume
                    // the earlier participant's selection a second time.
                    ctx.tagged_objects = pre_unit_tagged_objects.clone();
                    apply_player_tagged_object_partition(
                        &mut ctx.tagged_objects,
                        &tagged_objects_by_player[player_index],
                        &loop_local_tags,
                    );
                    ctx.effect_outcomes = effect_outcomes_by_player[player_index].clone();
                    if matches!(purpose, crate::effects::EffectExecutionPurpose::Payment) {
                        ctx.x_value = payment_x_by_player[player_index];
                    }
                    ctx.tagged_players = tagged_players_by_player[player_index].clone();
                    let pre_player_tagged_objects = ctx.tagged_objects.clone();
                    ctx.with_temp_iterated_player(Some(player_id), |ctx| {
                        for &effect_index in &unit {
                            let path = &optional_program.paths[effect_index];
                            let execute_child = prepare_optional_instruction(
                                game,
                                ctx,
                                player_index,
                                effect_index,
                                &optional_program,
                                &mut optional_acceptance,
                                &mut optional_initialized,
                                &mut optional_limits,
                                &mut optional_limit_reached,
                                &mut program_groups,
                            )?;
                            if ctx.decision_maker.awaiting_choice() {
                                return Ok(());
                            }
                            if !execute_child {
                                continue;
                            }
                            let effect = &simultaneous_effects[effect_index];
                            if effect.0.is_read_only_simultaneous_player_action() {
                                let outcome = in_optional_action(
                                    ctx,
                                    optional_program.path_is_optional(path),
                                    |ctx| {
                                        let scopes = program_path_scopes(
                                            path,
                                            &program_groups,
                                            player_index,
                                        );
                                        with_program_scope(ctx, &scopes, |ctx| {
                                            let outputs = purpose.execute(game, effect, ctx)?;
                                            let outcome = outputs.outcome.clone();
                                            retained_outputs.push(outputs);
                                            Ok(outcome)
                                        })
                                    },
                                )?;
                                retain_optional_outcome(
                                    PlayerActionOutcome::owned(outcome),
                                    player_index,
                                    path,
                                    super::may::is_object_selection(effect),
                                    &optional_program,
                                    &mut optional_outcomes,
                                    &mut outcomes_by_player,
                                    &mut outcomes,
                                    &mut actual_events,
                                    &mut owner_facts,
                                );
                            } else if matches!(
                                purpose,
                                crate::effects::EffectExecutionPurpose::Action
                            ) && effect.0.supports_prepared_action_program()
                            {
                                let selected = in_optional_action(
                                    ctx,
                                    optional_program.path_is_optional(path),
                                    |ctx| {
                                        let scopes = program_path_scopes(
                                            path,
                                            &program_groups,
                                            player_index,
                                        );
                                        with_program_scope(ctx, &scopes, |ctx| {
                                            let cursor =
                                                effect.select_prepared_action_program(game, ctx)?;
                                            Ok(cursor.map(|cursor| {
                                                super::action_program::ProgramParticipant::new(
                                                    cursor, ctx,
                                                )
                                            }))
                                        })
                                    },
                                )?;
                                let Some(selected) = selected else {
                                    if ctx.decision_maker.awaiting_choice() {
                                        return Ok(());
                                    }
                                    return Err(ExecutionError::InternalError(
                                        "completed program selection has no cursor".into(),
                                    ));
                                };
                                prepared_programs.push((
                                    player_index,
                                    ctx.tagged_objects.clone(),
                                    selected,
                                ));
                            } else if super::prepared_branch::supports_action_preparation(
                                effect, purpose,
                            ) {
                                let proposal = in_optional_action(
                                    ctx,
                                    optional_program.path_is_optional(path),
                                    |ctx| {
                                        let scopes = program_path_scopes(
                                            path,
                                            &program_groups,
                                            player_index,
                                        );
                                        with_program_scope(ctx, &scopes, |ctx| {
                                            super::prepared_branch::prepare_action_for_purpose(
                                                effect, purpose, game, ctx,
                                            )
                                        })
                                    },
                                )?;
                                let Some(proposal) = proposal else {
                                    // A pending payment retains its decision; it
                                    // must not become a fallback or a decline.
                                    return Ok::<(), ExecutionError>(());
                                };
                                // Some deferred proposals (notably a tagged
                                // MoveToZone) resolve their tagged target at
                                // commit time. Freeze this player's chooser
                                // context beside the proposal so the reset for
                                // the next APNAP player cannot erase it.
                                prepared.push((
                                    player_index,
                                    ctx.tagged_objects.clone(),
                                    proposal,
                                    optional_program.path_is_optional(path),
                                    path.clone(),
                                ));
                            } else {
                                return Err(ExecutionError::Impossible(
                                "generic each-player action lacks simultaneous proposal support"
                                    .to_string(),
                            ));
                            }
                            // Preserve the first unresolved choice. Later
                            // effects or APNAP players cannot prepare another
                            // prompt until this player's answer is available.
                            if ctx.decision_maker.awaiting_choice() {
                                return Ok::<(), ExecutionError>(());
                            }
                        }
                        Ok::<(), ExecutionError>(())
                    })?;
                    if ctx.decision_maker.awaiting_choice() {
                        return Ok((
                            ActionRun::Complete(EffectOutcome::count(0)),
                            retained_outputs,
                        ));
                    }
                    effect_outcomes_by_player[player_index] = ctx.effect_outcomes.clone();
                    if matches!(purpose, crate::effects::EffectExecutionPurpose::Payment) {
                        payment_x_by_player[player_index] = ctx.x_value;
                    }
                    tagged_players_by_player[player_index] = ctx.tagged_players.clone();
                    capture_player_tagged_object_deltas(
                        &pre_player_tagged_objects,
                        &ctx.tagged_objects,
                        &mut tagged_objects_by_player[player_index],
                        &mut loop_local_tags,
                    );
                    if !unit_has_mutating_effect {
                        merge_tagged_object_sets(&mut readonly_unit_tags, &ctx.tagged_objects);
                    }
                }
                ctx.tagged_objects = if unit_has_mutating_effect {
                    pre_unit_tagged_objects.clone()
                } else {
                    readonly_unit_tags
                };
                // A proposal prompt is still unanswered: unwind before
                // committing any fallback choice.
                if ctx.decision_maker.awaiting_choice() {
                    return Ok((
                        ActionRun::Complete(EffectOutcome::count(0)),
                        retained_outputs,
                    ));
                }

                finish_optional_preparation(
                    game,
                    &unit,
                    &optional_program,
                    &optional_acceptance,
                    &mut optional_limits,
                );
                if !unit_has_mutating_effect {
                    // A read-only unit has already completed every player's
                    // action. Do not reset its collected tags for an empty
                    // mutation phase.
                    attach_unit_player_counts(
                        &unit,
                        &simultaneous_effects,
                        &players,
                        &mut effect_outcomes_by_player,
                    );
                    continue;
                }
                if !prepared_programs.is_empty() {
                    if !prepared.is_empty() {
                        return Err(ExecutionError::InternalError(
                            "action unit mixes selected programs and leaf proposals".into(),
                        ));
                    }
                    let parent = crate::effects::ExecutionContextCheckpoint::capture(ctx);
                    let (metadata, selected): (Vec<_>, Vec<_>) = prepared_programs
                        .into_iter()
                        .map(|(index, tags, program)| ((index, tags), program))
                        .unzip();
                    let completed = super::action_program::execute_action_programs(
                        game,
                        ctx,
                        selected,
                        defer_draws,
                    )?;
                    let Some(progress) = completed else {
                        return Ok((
                            ActionRun::Complete(EffectOutcome::count(0)),
                            retained_outputs,
                        ));
                    };
                    let completed = match progress {
                        super::action_program::ActionProgramsProgress::Complete(completed) => {
                            completed
                        }
                        super::action_program::ActionProgramsProgress::Paused {
                            prefix,
                            continuation,
                        } => {
                            let completed_prefix = finish_players_outcome(
                                &effect,
                                players.clone(),
                                outcomes.clone(),
                                outcomes_by_player.clone(),
                                actual_events.clone(),
                                Some(owner_facts.clone()),
                            )?;
                            let combined_prefix = EffectOutcome::aggregate([
                                completed_prefix,
                                prefix.outcome.clone(),
                            ]);
                            retained_outputs.push(prefix);
                            let pending_program_draw = Some(PendingProgramDraw {
                                unit_index,
                                metadata,
                                parent,
                                tags: pre_unit_tagged_objects,
                                continuation,
                            });
                            return Ok((
                                ActionRun::Paused {
                                    prefix: combined_prefix,
                                    state: ForPlayersContinuationState::Action(Box::new(Self {
                                        next_unit: unit_index,
                                        pending_unit_draw,
                                        pending_batch_draw,
                                        pending_program_draw,
                                        effect,
                                        purpose,
                                        payment_x_by_player,
                                        players,
                                        outcomes,
                                        outcomes_by_player,
                                        optional_program,
                                        optional_acceptance,
                                        optional_outcomes,
                                        program_groups,
                                        optional_initialized,
                                        optional_limits,
                                        optional_limit_reached,
                                        actual_events,
                                        owner_facts,
                                        shared_action_players,
                                        units,
                                        tagged_objects_by_player,
                                        loop_local_tags,
                                        effect_outcomes_by_player,
                                        incoming_tagged_players,
                                        tagged_players_by_player,
                                    })),
                                },
                                retained_outputs,
                            ));
                        }
                    };
                    actual_events.extend(completed.events);
                    owner_facts.extend(completed.facts);
                    retained_outputs.extend(completed.shared);
                    let mut accumulated_tags = pre_unit_tagged_objects.clone();
                    let mut last_results = None;
                    for ((player_index, baseline), participant) in
                        metadata.into_iter().zip(completed.participants)
                    {
                        participant
                            .context
                            .restore_ref_preserving_resolution_control(ctx);
                        effect_outcomes_by_player[player_index] = ctx.effect_outcomes.clone();
                        tagged_players_by_player[player_index] = ctx.tagged_players.clone();
                        capture_player_tagged_object_deltas(
                            &baseline,
                            &ctx.tagged_objects,
                            &mut tagged_objects_by_player[player_index],
                            &mut loop_local_tags,
                        );
                        merge_tagged_object_sets(&mut accumulated_tags, &ctx.tagged_objects);
                        last_results = Some(ctx.effect_outcomes.clone());
                        let outcome = participant.outputs.outcome.clone();
                        retained_outputs.push(participant.outputs);
                        let path = &optional_program.paths[*unit.last().expect("action unit")];
                        retain_optional_outcome(
                            PlayerActionOutcome::binding_only(outcome, Vec::new()),
                            player_index,
                            path,
                            false,
                            &optional_program,
                            &mut optional_outcomes,
                            &mut outcomes_by_player,
                            &mut outcomes,
                            &mut actual_events,
                            &mut owner_facts,
                        );
                    }
                    parent.restore_ref_preserving_resolution_control(ctx);
                    ctx.tagged_objects = accumulated_tags;
                    if let Some(results) = last_results {
                        ctx.effect_outcomes = results;
                    }
                    attach_unit_player_counts(
                        &unit,
                        &simultaneous_effects,
                        &players,
                        &mut effect_outcomes_by_player,
                    );
                    continue;
                }
                let game_checkpoint = game.clone();
                // All hidden identities and participant selections are settled
                // before any replacement prefix may change another player's set.
                for (index, tags, proposal, optional, path) in &mut prepared {
                    ctx.tagged_objects = tags.clone();
                    ctx.effect_outcomes = effect_outcomes_by_player[*index].clone();
                    ctx.tagged_players = tagged_players_by_player[*index].clone();
                    ctx.with_temp_iterated_player(Some(players[*index]), |ctx| {
                        in_optional_action(ctx, *optional, |ctx| {
                            let scopes = program_path_scopes(path, &program_groups, *index);
                            with_program_scope(ctx, &scopes, |ctx| {
                                proposal.prepare_selection(game, ctx)
                            })
                        })
                    })?;
                    if ctx.decision_maker.awaiting_choice() {
                        return Ok((
                            ActionRun::Complete(EffectOutcome::count(0)),
                            retained_outputs,
                        ));
                    }
                }
                let declared_payments = prepared
                    .iter()
                    .flat_map(|(_, _, proposal, _, _)| proposal.declared_payment_resources())
                    .collect::<Vec<_>>();
                if !crate::effects::can_pay_declared_resources(game, &declared_payments) {
                    return Err(ExecutionError::Impossible(
                        "simultaneous payments exceed available shared resources".into(),
                    ));
                }
                // Replacement eligibility observes one pre-mutation world.
                // One-shot consumption and APNAP choices remain ordered, but
                // no player's original life change has committed yet.
                for (index, tags, proposal, optional, path) in &mut prepared {
                    ctx.tagged_objects = tags.clone();
                    ctx.effect_outcomes = effect_outcomes_by_player[*index].clone();
                    if matches!(purpose, crate::effects::EffectExecutionPurpose::Payment) {
                        ctx.x_value = payment_x_by_player[*index];
                    }
                    ctx.tagged_players = tagged_players_by_player[*index].clone();
                    ctx.with_temp_iterated_player(Some(players[*index]), |ctx| {
                        in_optional_action(ctx, *optional, |ctx| {
                            let scopes = program_path_scopes(path, &program_groups, *index);
                            with_program_scope(ctx, &scopes, |ctx| {
                                proposal.prepare_original(game, ctx)
                            })
                        })
                    })?;
                    if ctx.decision_maker.awaiting_choice() {
                        return Ok((
                            ActionRun::Complete(EffectOutcome::count(0)),
                            retained_outputs,
                        ));
                    }
                }
                let prepared_payments = prepared
                    .iter()
                    .flat_map(|(_, _, proposal, _, _)| proposal.declared_payment_resources())
                    .collect::<Vec<_>>();
                if !crate::effects::can_pay_declared_resources(game, &prepared_payments) {
                    return Err(ExecutionError::Impossible(
                        "simultaneous payments exceed available shared resources".into(),
                    ));
                }
                ctx.tagged_objects = pre_unit_tagged_objects.clone();
                // CR 101.4 / 603.2c / 603.10a: the players' prepared actions
                // happen at the same time, as one event that looks back at
                // the same trigger sources.
                let pinned_lookback =
                    crate::effects::helpers::begin_simultaneous_zone_change_lookback(game);
                let opened_batch = game.open_simultaneous_action();
                let mut damage_bindings = Vec::new();
                let mut prepared = prepared
                    .into_iter()
                    .map(|action| {
                        if action.2.damage_action_inputs().is_some() {
                            damage_bindings.push(action);
                            None
                        } else {
                            Some(action)
                        }
                    })
                    .collect::<Vec<_>>();
                // Preserve commitment order while sealing every uncollected
                // instruction before any original mutates the world. The cohort
                // claims its combined replacements at its first original marker.
                let first_damage = prepared
                    .iter()
                    .position(Option::is_none)
                    .unwrap_or(prepared.len());
                let seal_range = |game: &mut GameState,
                                  ctx: &mut ExecutionContext,
                                  actions: &mut [Option<PreparedPlayerAction>]|
                 -> Result<(), ExecutionError> {
                    for (index, tags, proposal, optional, path) in actions.iter_mut().flatten() {
                        let parent = crate::effects::ExecutionContextCheckpoint::capture(ctx);
                        ctx.tagged_objects = tags.clone();
                        ctx.effect_outcomes = effect_outcomes_by_player[*index].clone();
                        if matches!(purpose, crate::effects::EffectExecutionPurpose::Payment) {
                            ctx.x_value = payment_x_by_player[*index];
                        }
                        ctx.tagged_players = tagged_players_by_player[*index].clone();
                        let result = ctx.with_temp_iterated_player(Some(players[*index]), |ctx| {
                            in_optional_action(ctx, *optional, |ctx| {
                                let scopes = program_path_scopes(path, &program_groups, *index);
                                with_program_scope(ctx, &scopes, |ctx| {
                                    proposal.seal_original(game, ctx)
                                })
                            })
                        });
                        if result.is_ok() && !ctx.decision_maker.awaiting_choice() {
                            parent.restore_preserving_resolution_control(ctx);
                        } else {
                            parent.restore(ctx);
                        }
                        result?;
                        if ctx.decision_maker.awaiting_choice() {
                            break;
                        }
                    }
                    Ok(())
                };
                if let Err(error) = seal_range(game, ctx, &mut prepared[..first_damage]) {
                    *game = game_checkpoint;
                    ctx.tagged_objects = pre_unit_tagged_objects;
                    return Err(error);
                }
                if ctx.decision_maker.awaiting_choice() {
                    game.close_simultaneous_action(opened_batch);
                    crate::effects::helpers::end_simultaneous_zone_change_lookback(
                        game,
                        pinned_lookback,
                    );
                    ctx.tagged_objects = pre_unit_tagged_objects;
                    return Ok((
                        ActionRun::Complete(EffectOutcome::count(0)),
                        retained_outputs,
                    ));
                }
                // Seal the entire damage cohort against the pre-mutation world,
                // including shared prevention and per-source lifelink proposals.
                let mut damage_cohort = if damage_bindings.is_empty() {
                    None
                } else {
                    let inputs = crate::effects::damage::DamageActionInputs::collect(
                        damage_bindings
                            .iter()
                            .map(|(_, _, proposal, _, _)| proposal.damage_action_inputs()),
                    )
                    .ok_or_else(|| {
                        ExecutionError::InternalError(
                            "player damage cohort lost its frozen assignments".into(),
                        )
                    })?;
                    match inputs.seal(game, ctx) {
                        Ok(proposal) => Some(proposal),
                        Err(error) => {
                            *game = game_checkpoint;
                            ctx.tagged_objects = pre_unit_tagged_objects;
                            return Err(error);
                        }
                    }
                };
                if ctx.decision_maker.awaiting_choice() {
                    game.close_simultaneous_action(opened_batch);
                    crate::effects::helpers::end_simultaneous_zone_change_lookback(
                        game,
                        pinned_lookback,
                    );
                    ctx.tagged_objects = pre_unit_tagged_objects;
                    return Ok((
                        ActionRun::Complete(EffectOutcome::count(0)),
                        retained_outputs,
                    ));
                }
                if let Err(error) = seal_range(game, ctx, &mut prepared[first_damage..]) {
                    *game = game_checkpoint;
                    ctx.tagged_objects = pre_unit_tagged_objects;
                    return Err(error);
                }
                if ctx.decision_maker.awaiting_choice() {
                    game.close_simultaneous_action(opened_batch);
                    crate::effects::helpers::end_simultaneous_zone_change_lookback(
                        game,
                        pinned_lookback,
                    );
                    ctx.tagged_objects = pre_unit_tagged_objects;
                    return Ok((
                        ActionRun::Complete(EffectOutcome::count(0)),
                        retained_outputs,
                    ));
                }
                let sealed_payments = prepared
                    .iter()
                    .flatten()
                    .flat_map(|(_, _, proposal, _, _)| proposal.declared_payment_resources())
                    .collect::<Vec<_>>();
                if !crate::effects::can_pay_declared_resources(game, &sealed_payments) {
                    *game = game_checkpoint;
                    ctx.tagged_objects = pre_unit_tagged_objects;
                    return Err(ExecutionError::Impossible(
                        "simultaneous payments exceed available shared resources".into(),
                    ));
                }
                let mut batch_outcomes = Vec::with_capacity(prepared.len());
                let mut original_observations = Vec::new();
                let mut accumulated_unit_tags = ctx.tagged_objects.clone();
                let mut active_commit_player = None;
                for action in prepared {
                    let Some((player_index, prepared_tagged_objects, proposal, optional, path)) =
                        action
                    else {
                        if let Some(proposal) = damage_cohort.take() {
                            let (committed, observations) =
                                match crate::effects::with_action_observations(game, |game| {
                                    proposal.commit_original_with_outputs(game, ctx)
                                }) {
                                    Ok(committed) => committed,
                                    Err(error) => {
                                        *game = game_checkpoint;
                                        ctx.tagged_objects = pre_unit_tagged_objects;
                                        return Err(error);
                                    }
                                };
                            original_observations.extend(observations);
                            batch_outcomes.push((
                                None,
                                committed,
                                Some(PlayerOriginalContext::DamageCohort(std::mem::take(
                                    &mut damage_bindings,
                                ))),
                            ));
                            if ctx.decision_maker.awaiting_choice() {
                                game.close_simultaneous_action(opened_batch);
                                crate::effects::helpers::end_simultaneous_zone_change_lookback(
                                    game,
                                    pinned_lookback,
                                );
                                ctx.tagged_objects = pre_unit_tagged_objects;
                                return Ok((
                                    ActionRun::Complete(EffectOutcome::count(0)),
                                    retained_outputs,
                                ));
                            }
                        }
                        continue;
                    };
                    if active_commit_player != Some(player_index) {
                        if active_commit_player.is_some() {
                            merge_tagged_object_sets(
                                &mut accumulated_unit_tags,
                                &ctx.tagged_objects,
                            );
                        }
                        ctx.tagged_objects = prepared_tagged_objects.clone();
                        ctx.effect_outcomes = effect_outcomes_by_player[player_index].clone();
                        if matches!(purpose, crate::effects::EffectExecutionPurpose::Payment) {
                            ctx.x_value = payment_x_by_player[player_index];
                        }
                        ctx.tagged_players = tagged_players_by_player[player_index].clone();
                        active_commit_player = Some(player_index);
                    }
                    let proposal_baseline = prepared_tagged_objects.clone();
                    match crate::effects::with_action_observations(game, |game| {
                        ctx.with_temp_iterated_player(Some(players[player_index]), |ctx| {
                            in_optional_action(ctx, optional, |ctx| {
                                let scopes =
                                    program_path_scopes(&path, &program_groups, player_index);
                                with_program_scope(ctx, &scopes, |ctx| {
                                    proposal.commit_original_with_outputs(game, ctx)
                                })
                            })
                        })
                    }) {
                        Ok((committed, observations)) => {
                            original_observations.extend(observations);
                            let completion_context = committed.completion.as_ref().map(|_| {
                                PlayerOriginalContext::Instruction(
                                    crate::effects::ExecutionContextCheckpoint::capture(ctx),
                                    optional,
                                    path.clone(),
                                )
                            });
                            effect_outcomes_by_player[player_index] = ctx.effect_outcomes.clone();
                            if matches!(purpose, crate::effects::EffectExecutionPurpose::Payment) {
                                payment_x_by_player[player_index] = ctx.x_value;
                            }
                            tagged_players_by_player[player_index] = ctx.tagged_players.clone();
                            capture_player_tagged_object_deltas(
                                &proposal_baseline,
                                &ctx.tagged_objects,
                                &mut tagged_objects_by_player[player_index],
                                &mut loop_local_tags,
                            );
                            batch_outcomes.push((
                                Some(player_index),
                                committed,
                                completion_context,
                            ));
                        }
                        Err(error) => {
                            *game = game_checkpoint;
                            ctx.tagged_objects = pre_unit_tagged_objects;
                            return Err(error);
                        }
                    }
                    if ctx.decision_maker.awaiting_choice() {
                        game.close_simultaneous_action(opened_batch);
                        crate::effects::helpers::end_simultaneous_zone_change_lookback(
                            game,
                            pinned_lookback,
                        );
                        ctx.tagged_objects = pre_unit_tagged_objects;
                        return Ok((
                            ActionRun::Complete(EffectOutcome::count(0)),
                            retained_outputs,
                        ));
                    }
                }

                super::original_observations::retain_original_observations(
                    batch_outcomes.iter_mut().map(|(_, receipt, _)| receipt),
                    original_observations,
                );
                game.close_simultaneous_action(opened_batch);
                crate::effects::helpers::end_simultaneous_zone_change_lookback(
                    game,
                    pinned_lookback,
                );
                merge_tagged_object_sets(&mut accumulated_unit_tags, &ctx.tagged_objects);
                (batch_outcomes, accumulated_unit_tags)
            };
            if !batch_resumed {
                batch_outcomes =
                    crate::effects::composition::prepare_simultaneous_originals_with_participants(
                        game,
                        ctx,
                        batch_outcomes,
                        |(_, receipt, _)| receipt,
                        |game, ctx, participants| {
                            game.observe_prepared_life_payment_originals(
                                ctx,
                                participants.iter_mut().flat_map(|(_, receipt, _)| {
                                    receipt.outcome.outcome.events.iter_mut()
                                }),
                            )
                        },
                    )?;
            }
            if defer_draws {
                let mut boundaries = Vec::with_capacity(batch_outcomes.len());
                let mut paused = false;
                for (player_index, mut receipt, mut context) in batch_outcomes {
                    if !paused
                        && !ctx.resolution_stopped()
                        && let Some(completion) = receipt.completion.take()
                    {
                        let original = receipt.outcome;
                        receipt = match (&player_index, &context) {
                            (
                                Some(index),
                                Some(PlayerOriginalContext::Instruction(captured, optional, path)),
                            ) => {
                                captured.restore_ref_preserving_resolution_control(ctx);
                                let baseline = ctx.tagged_objects.clone();
                                let prepared =
                                    ctx.with_temp_iterated_player(Some(players[*index]), |ctx| {
                                        in_optional_action(ctx, *optional, |ctx| {
                                            let scopes =
                                                program_path_scopes(path, &program_groups, *index);
                                            with_program_scope(ctx, &scopes, |ctx| {
                                                completion.prepare_draw_boundary_from_outputs(
                                                    game, ctx, original,
                                                )
                                            })
                                        })
                                    })?;
                                effect_outcomes_by_player[*index] = ctx.effect_outcomes.clone();
                                if matches!(
                                    purpose,
                                    crate::effects::EffectExecutionPurpose::Payment
                                ) {
                                    payment_x_by_player[*index] = ctx.x_value;
                                }
                                tagged_players_by_player[*index] = ctx.tagged_players.clone();
                                capture_player_tagged_object_deltas(
                                    &baseline,
                                    &ctx.tagged_objects,
                                    &mut tagged_objects_by_player[*index],
                                    &mut loop_local_tags,
                                );
                                merge_tagged_object_sets(
                                    &mut accumulated_unit_tags,
                                    &ctx.tagged_objects,
                                );
                                context = Some(PlayerOriginalContext::Instruction(
                                    crate::effects::ExecutionContextCheckpoint::capture(ctx),
                                    *optional,
                                    path.clone(),
                                ));
                                prepared
                            }
                            _ => completion
                                .prepare_draw_boundary_from_outputs(game, ctx, original)?,
                        };
                    }
                    paused |= receipt.completion.is_some();
                    boundaries.push((player_index, receipt, context));
                    if ctx.decision_maker.awaiting_choice() {
                        return Ok((
                            ActionRun::Complete(EffectOutcome::count(0)),
                            retained_outputs,
                        ));
                    }
                }
                batch_outcomes = boundaries;
                if ctx.resolution_stopped() {
                    for (_, receipt, _) in &mut batch_outcomes {
                        receipt.completion = None;
                    }
                    paused = false;
                }
                if paused {
                    let completed_prefix = finish_players_outcome(
                        &effect,
                        players.clone(),
                        outcomes.clone(),
                        outcomes_by_player.clone(),
                        actual_events.clone(),
                        Some(owner_facts.clone()),
                    )?;
                    let prefix = EffectOutcome::aggregate(
                        std::iter::once(completed_prefix).chain(
                            batch_outcomes
                                .iter()
                                .map(|(_, receipt, _)| receipt.outcome.outcome.clone()),
                        ),
                    );
                    let pending_batch_draw = Some(PendingBatchDraw {
                        unit_index,
                        outcomes: batch_outcomes,
                        tags: accumulated_unit_tags,
                    });
                    return Ok((
                        ActionRun::Paused {
                            prefix,
                            state: ForPlayersContinuationState::Action(Box::new(Self {
                                next_unit: unit_index,
                                pending_unit_draw,
                                pending_batch_draw,
                                pending_program_draw,
                                effect,
                                purpose,
                                payment_x_by_player,
                                players,
                                outcomes,
                                outcomes_by_player,
                                optional_program,
                                optional_acceptance,
                                optional_outcomes,
                                program_groups,
                                optional_initialized,
                                optional_limits,
                                optional_limit_reached,
                                actual_events,
                                owner_facts,
                                shared_action_players,
                                units,
                                tagged_objects_by_player,
                                loop_local_tags,
                                effect_outcomes_by_player,
                                incoming_tagged_players,
                                tagged_players_by_player,
                            })),
                        },
                        retained_outputs,
                    ));
                }
            }
            if super::simultaneous::original_cohort_phase_status_with_participants(
                &mut batch_outcomes,
                |(_, receipt, _)| receipt,
            ) != crate::effects::OriginalPhaseStatus::Combined
            {
                let Some(phased) = super::simultaneous::complete_original_cohort_phase_with_participants(
                    game,
                    ctx,
                    batch_outcomes,
                    |(_, receipt, _)| receipt,
                    |game, ctx, (player_index, mut receipt, completion_context): OriginalBatchOutcome| {
                        let Some(index) = player_index else {
                            let Some(PlayerOriginalContext::DamageCohort(bindings)) = completion_context else {
                                return Err(ExecutionError::InternalError(
                                    "shared player damage lost its binding contexts".into(),
                                ));
                            };
                            receipt = super::simultaneous::complete_retained_original_phase_with_outputs(
                                game, ctx, receipt,
                            )?;
                            return Ok((None, receipt, Some(PlayerOriginalContext::DamageCohort(bindings))));
                        };
                        let PlayerOriginalContext::Instruction(captured, optional, path) =
                            completion_context.ok_or_else(|| ExecutionError::InternalError(
                                "player original lost its completion context".into(),
                            ))?
                        else {
                            return Err(ExecutionError::InternalError(
                                "player instruction received a cohort binding context".into(),
                            ));
                        };
                        captured.restore_preserving_resolution_control(ctx);
                        let baseline = ctx.tagged_objects.clone();
                        if ctx.resolution_stopped() {
                            receipt.completion = None;
                        } else {
                            receipt = ctx.with_temp_iterated_player(Some(players[index]), |ctx| {
                                in_optional_action(ctx, optional, |ctx| {
                                    let scopes = program_path_scopes(&path, &program_groups, index);
                                    with_program_scope(ctx, &scopes, |ctx| {
                                        super::simultaneous::complete_retained_original_phase_with_outputs(
                                            game, ctx, receipt,
                                        )
                                    })
                                })
                            })?;
                        }
                        let captured = crate::effects::ExecutionContextCheckpoint::capture(ctx);
                        if !ctx.decision_maker.awaiting_choice() {
                            effect_outcomes_by_player[index] = ctx.effect_outcomes.clone();
                            if matches!(purpose, crate::effects::EffectExecutionPurpose::Payment) {
                                payment_x_by_player[index] = ctx.x_value;
                            }
                            tagged_players_by_player[index] = ctx.tagged_players.clone();
                            capture_player_tagged_object_deltas(
                                &baseline,
                                &ctx.tagged_objects,
                                &mut tagged_objects_by_player[index],
                                &mut loop_local_tags,
                            );
                            merge_tagged_object_sets(&mut accumulated_unit_tags, &ctx.tagged_objects);
                        }
                        Ok((Some(index), receipt, Some(PlayerOriginalContext::Instruction(captured, optional, path))))
                    },
                )? else {
                    return Ok((
                        ActionRun::Complete(EffectOutcome::count(0)),
                        retained_outputs,
                    ));
                };
                batch_outcomes = phased;
            }
            let completed_outcomes = {
                let mut completed = Vec::new();
                let mut complete =
                    |game: &mut GameState,
                     ctx: &mut ExecutionContext,
                     (player_index, committed, completion_context): OriginalBatchOutcome|
                     -> Result<CompletedPlayerActionGroup, ExecutionError> {
                        let Some(player_index) = player_index else {
                            let Some(PlayerOriginalContext::DamageCohort(bindings)) =
                                completion_context
                            else {
                                return Err(ExecutionError::InternalError(
                                    "shared player damage lost its binding contexts".into(),
                                ));
                            };
                            let mut outputs =
                                crate::effects::composition::complete_committed_original_with_outputs(
                                    game, ctx, committed,
                                )?;
                            if ctx.decision_maker.awaiting_choice() {
                                return Ok(CompletedPlayerActionGroup {
                                    owner: None,
                                    bindings: Vec::new(),
                                });
                            }
                            let mut rows = Vec::with_capacity(bindings.len());
                            let mut owned_bindings = Vec::with_capacity(bindings.len());
                            let mut active_binding_player = None;
                            for (index, tags, proposal, optional, path) in bindings {
                                if active_binding_player != Some(index) {
                                    ctx.tagged_objects = tags.clone();
                                    ctx.effect_outcomes = effect_outcomes_by_player[index].clone();
                                    if matches!(
                                        purpose,
                                        crate::effects::EffectExecutionPurpose::Payment
                                    ) {
                                        ctx.x_value = payment_x_by_player[index];
                                    }
                                    ctx.tagged_players = tagged_players_by_player[index].clone();
                                    active_binding_player = Some(index);
                                }
                                let binding =
                                    ctx.with_temp_iterated_player(Some(players[index]), |ctx| {
                                        in_optional_action(ctx, optional, |ctx| {
                                            let scopes =
                                                program_path_scopes(&path, &program_groups, index);
                                            with_program_scope(ctx, &scopes, |ctx| {
                                                proposal.bind_damage_action(game, ctx, &outputs)
                                            })
                                        })
                                    })?;
                                if ctx.decision_maker.awaiting_choice() {
                                    return Ok(CompletedPlayerActionGroup {
                                        owner: None,
                                        bindings: Vec::new(),
                                    });
                                }
                                let outcome = binding.outcome.clone();
                                owned_bindings.push(binding);
                                effect_outcomes_by_player[index] = ctx.effect_outcomes.clone();
                                if matches!(
                                    purpose,
                                    crate::effects::EffectExecutionPurpose::Payment
                                ) {
                                    payment_x_by_player[index] = ctx.x_value;
                                }
                                tagged_players_by_player[index] = ctx.tagged_players.clone();
                                capture_player_tagged_object_deltas(
                                    &tags,
                                    &ctx.tagged_objects,
                                    &mut tagged_objects_by_player[index],
                                    &mut loop_local_tags,
                                );
                                merge_tagged_object_sets(
                                    &mut accumulated_unit_tags,
                                    &ctx.tagged_objects,
                                );
                                rows.push((
                                    index,
                                    PlayerActionOutcome::binding_only(outcome, Vec::new()),
                                ));
                            }
                            crate::effects::DamageActionBinding::from_bindings(
                                owned_bindings,
                                |_| EffectOutcome::count(0),
                            )
                            .transfer_owned_outputs(&mut outputs);
                            let owner = outputs.outcome.clone();
                            retained_outputs.push(outputs);
                            return Ok(CompletedPlayerActionGroup {
                                owner: Some(owner),
                                bindings: rows,
                            });
                        };
                        let mut original_outputs = committed.outcome;
                        let mut outcome = original_outputs.outcome.clone();
                        if let Some(completion) = committed.completion {
                            let PlayerOriginalContext::Instruction(captured, optional, path) =
                                completion_context.ok_or_else(|| {
                                    ExecutionError::InternalError(
                                        "player original lost its completion context".into(),
                                    )
                                })?
                            else {
                                return Err(ExecutionError::InternalError(
                                    "player instruction received a cohort binding context".into(),
                                ));
                            };
                            captured.restore_preserving_resolution_control(ctx);
                            let baseline = ctx.tagged_objects.clone();
                            outcome = ctx.with_temp_iterated_player(
                                        Some(players[player_index]),
                                        |ctx| {
                                            in_optional_action(ctx, optional, |ctx| {
                                                let scopes = program_path_scopes(
                                                    &path,
                                                    &program_groups,
                                                    player_index,
                                                );
                                                with_program_scope(ctx, &scopes, |ctx| {
                                                    let outputs = crate::effects::composition::complete_committed_original_with_outputs(
                                                        game, ctx, crate::effects::SimultaneousEffectCommit {
                                                            outcome: original_outputs,
                                                            completion: Some(completion),
                                                        },
                                                    )?;
                                                    let outcome = outputs.outcome.clone();
                                                    retained_outputs.push(outputs);
                                                    Ok(outcome)
                                                })
                                            })
                                        },
                                    )?;
                            if !ctx.decision_maker.awaiting_choice() {
                                effect_outcomes_by_player[player_index] =
                                    ctx.effect_outcomes.clone();
                                if matches!(
                                    purpose,
                                    crate::effects::EffectExecutionPurpose::Payment
                                ) {
                                    payment_x_by_player[player_index] = ctx.x_value;
                                }
                                tagged_players_by_player[player_index] = ctx.tagged_players.clone();
                                capture_player_tagged_object_deltas(
                                    &baseline,
                                    &ctx.tagged_objects,
                                    &mut tagged_objects_by_player[player_index],
                                    &mut loop_local_tags,
                                );
                                merge_tagged_object_sets(
                                    &mut accumulated_unit_tags,
                                    &ctx.tagged_objects,
                                );
                            }
                        } else {
                            original_outputs.synchronize_observations();
                            retained_outputs.push(original_outputs);
                        }
                        Ok(CompletedPlayerActionGroup {
                            owner: None,
                            bindings: vec![(player_index, PlayerActionOutcome::owned(outcome))],
                        })
                    };
                for participant in batch_outcomes {
                    completed.push(complete(game, ctx, participant)?);
                    if ctx.decision_maker.awaiting_choice() {
                        break;
                    }
                }
                completed
            };
            if ctx.decision_maker.awaiting_choice() {
                return Ok((
                    ActionRun::Complete(EffectOutcome::count(0)),
                    retained_outputs,
                ));
            }
            ctx.tagged_objects = accumulated_unit_tags;
            // Keep each player's scalar result local ("that many"), while
            // attaching the completed action's per-player counts for
            // collective metrics such as the greatest count. No following
            // action may read a partial result before every player commits.
            attach_unit_player_counts(
                &unit,
                &simultaneous_effects,
                &players,
                &mut effect_outcomes_by_player,
            );
            for group in completed_outcomes {
                if let Some(owner) = group.owner {
                    actual_events.extend(owner.events);
                    owner_facts.extend(owner.execution_facts);
                }
                for (player_index, outcome) in group.bindings {
                    let path = &optional_program.paths[*unit.last().expect("action unit")];
                    retain_optional_outcome(
                        outcome,
                        player_index,
                        path,
                        false,
                        &optional_program,
                        &mut optional_outcomes,
                        &mut outcomes_by_player,
                        &mut outcomes,
                        &mut actual_events,
                        &mut owner_facts,
                    );
                }
            }
        }
        ctx.tagged_players =
            merge_tagged_players_by_player(&incoming_tagged_players, &tagged_players_by_player);
        finish_players_outcome(
            &effect,
            players,
            outcomes,
            outcomes_by_player,
            actual_events,
            Some(owner_facts),
        )
        .map(|outcome| (ActionRun::Complete(outcome), retained_outputs))
    }
}

fn finish_players_outcome(
    effect: &ForPlayersEffect,
    players: Vec<PlayerId>,
    outcomes: Vec<EffectOutcome>,
    outcomes_by_player: Vec<Vec<EffectOutcome>>,
    actual_events: Vec<crate::events::RawEvent>,
    owner_facts: Option<Vec<crate::effect::ExecutionFact>>,
) -> Result<EffectOutcome, ExecutionError> {
    let mut player_counts = Vec::new();
    let mut player_affected_memory = Vec::new();
    for (&player_id, player_outcomes) in players.iter().zip(&outcomes_by_player) {
        if player_outcomes.is_empty() {
            continue;
        }
        let iteration_outcome =
            EffectOutcome::aggregate_summing_counts(player_outcomes.iter().cloned());
        let count = if effect.stop_after_first_happened {
            i64::from(iteration_outcome.something_happened())
        } else {
            iteration_outcome
                .as_count()
                .unwrap_or_else(|| i64::from(iteration_outcome.something_happened()))
        };
        player_counts.push((player_id, count));
        if let Some(memory) = iteration_outcome.affected_object_memory()
            && !memory.is_empty()
        {
            player_affected_memory.push((player_id, memory.to_vec()));
        }
    }

    // An offer's collective result is the accepted action, or a declined
    // result when nobody acts. Earlier declines must not negate a later
    // acceptance; all participants remain available through PlayerCounts.
    let mut outcome = if effect.stop_after_first_happened {
        outcomes_by_player
            .iter()
            .filter(|iteration| !iteration.is_empty())
            .map(|iteration| EffectOutcome::aggregate_summing_counts(iteration.iter().cloned()))
            .find(EffectOutcome::something_happened)
            .unwrap_or_else(|| EffectOutcome::aggregate_summing_counts(outcomes))
    } else {
        EffectOutcome::aggregate_summing_counts(outcomes)
    };
    if !(effect.sequential || effect.starting_with_controller || effect.stop_after_first_happened) {
        outcome.events = actual_events;
    }
    let observations = owner_facts.map(|facts| {
        EffectOutcome::resolved()
            .with_events(outcome.events.clone())
            .with_execution_facts(EffectOutcome::merge_execution_facts(facts))
            .with_player_counts(player_counts.clone())
            .with_player_affected_object_memory(player_affected_memory.clone())
    });
    let outcome = outcome
        .with_player_counts(player_counts)
        .with_player_affected_object_memory(player_affected_memory);
    Ok(match observations {
        Some(observations) => outcome.with_authoritative_observations(observations),
        None => outcome,
    })
}

fn execute_player_program_with_outputs(
    effect: &ForPlayersEffect,
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    purpose: crate::effects::EffectExecutionPurpose,
) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
    let inherited_x = ctx.x_value;
    let result = crate::effects::tokens::execute_resource_transaction_with_pending_value(
        game,
        ctx,
        || {
            crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::with_objects(
                Vec::new(),
            ))
        },
        |game, ctx| {
            crate::effects::runtime::with_per_event_trigger_matching(game, true, |game| {
                effect.execute_players_for_purpose(game, ctx, purpose)
            })
        },
    );
    if matches!(purpose, crate::effects::EffectExecutionPurpose::Payment) {
        ctx.x_value = inherited_x;
    }
    if ctx.decision_maker.awaiting_choice() {
        result.map(|_| {
            crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0))
        })
    } else {
        result
    }
}

impl EffectExecutor for ForPlayersEffect {
    fn supports_replacement_draw_continuation(&self) -> bool {
        self.effects
            .iter()
            .all(crate::effects::replacement::replacement_effect_supported)
    }

    fn prepare_replacement_draw_continuation_with_outputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<
        crate::effects::SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>,
        ExecutionError,
    > {
        self.prepare_draw_continuation(game, ctx)
            .map(ForPlayersDrawProgress::into_commit)
    }

    fn directly_mentions_player_filter(&self, needle: &crate::target::PlayerFilter) -> bool {
        self.filter.mentions_player_filter(needle)
    }

    fn shares_iterated_damage_action(&self) -> bool {
        // Sequential and early-stop instructions own separate boundaries.
        !self.sequential
            && !self.stop_after_first_happened
            && !self.effects.is_empty()
            && self
                .effects
                .iter()
                .all(|effect| effect.0.shares_iterated_damage_action())
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn as_cost_executable(&self) -> Option<&dyn crate::effects::CostExecutableEffect> {
        (!self.effects.is_empty()
            && self
                .effects
                .iter()
                .all(|effect| effect.0.as_cost_executable().is_some()))
        .then_some(self as &dyn crate::effects::CostExecutableEffect)
    }

    fn visit_child_effects(&self, visitor: &mut dyn FnMut(&Effect)) {
        for effect in &self.effects {
            visitor(effect);
        }
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        self.execute_with_outputs(game, ctx)
            .map(crate::effects::CompletedEffectOutputs::into_outcome)
    }

    fn execute_with_outputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        execute_player_program_with_outputs(
            self,
            game,
            ctx,
            crate::effects::EffectExecutionPurpose::Action,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone)]
    struct RecordIteratedPlayerChoice(&'static str);

    #[derive(Debug)]
    struct ReadOnlyChoiceProposal;

    impl crate::effects::SimultaneousEffectProposal for ReadOnlyChoiceProposal {
        fn commit(
            self: Box<Self>,
            _game: &mut GameState,
            _ctx: &mut ExecutionContext,
        ) -> Result<EffectOutcome, ExecutionError> {
            Ok(EffectOutcome::count(0))
        }
    }

    impl EffectExecutor for RecordIteratedPlayerChoice {
        fn execute(
            &self,
            game: &mut GameState,
            ctx: &mut ExecutionContext,
        ) -> Result<EffectOutcome, ExecutionError> {
            let player = ctx
                .iteration
                .iterated_player
                .expect("ForPlayers must set the iterated player");
            let prompt =
                crate::decisions::context::BooleanContext::new(player, Some(ctx.source), self.0);
            ctx.decision_maker.decide_boolean(game, &prompt);
            Ok(EffectOutcome::count(0))
        }

        fn supports_simultaneous_player_action(&self) -> bool {
            true
        }

        fn prepare_simultaneous_player_action(
            &self,
            game: &GameState,
            ctx: &mut ExecutionContext,
        ) -> Result<Box<dyn crate::effects::SimultaneousEffectProposal>, ExecutionError> {
            let player = ctx
                .iteration
                .iterated_player
                .expect("ForPlayers must set the iterated player");
            let prompt =
                crate::decisions::context::BooleanContext::new(player, Some(ctx.source), self.0);
            ctx.decision_maker.decide_boolean(game, &prompt);
            Ok(Box::new(ReadOnlyChoiceProposal))
        }
    }

    #[derive(Default)]
    struct RecordChoiceOrder {
        prompts: Vec<(PlayerId, String)>,
    }

    impl crate::decision::DecisionMaker for RecordChoiceOrder {
        fn decide_boolean(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::BooleanContext,
        ) -> bool {
            self.prompts.push((ctx.player, ctx.description.clone()));
            false
        }
    }

    #[derive(Default)]
    struct FirstPlayerPays {
        prompted: Vec<PlayerId>,
    }

    impl crate::decision::DecisionMaker for FirstPlayerPays {
        fn decide_boolean(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::BooleanContext,
        ) -> bool {
            self.prompted.push(ctx.player);
            ctx.player == PlayerId::from_index(0)
        }
    }

    #[derive(Debug, Clone)]
    struct AtomicBatchProbe;

    #[derive(Debug)]
    struct AtomicBatchProposal {
        player: PlayerId,
        fail: bool,
    }

    impl crate::effects::SimultaneousEffectProposal for AtomicBatchProposal {
        fn commit(
            self: Box<Self>,
            game: &mut GameState,
            _ctx: &mut ExecutionContext,
        ) -> Result<EffectOutcome, ExecutionError> {
            if self.fail {
                return Err(ExecutionError::Impossible("probe failure".to_string()));
            }
            game.player_mut(self.player)
                .expect("probe player")
                .lose_life(1);
            Ok(EffectOutcome::count(1))
        }
    }

    impl EffectExecutor for AtomicBatchProbe {
        fn execute(
            &self,
            _game: &mut GameState,
            _ctx: &mut ExecutionContext,
        ) -> Result<EffectOutcome, ExecutionError> {
            unreachable!("generic each-player execution must use the proposal hook")
        }

        fn supports_simultaneous_player_action(&self) -> bool {
            true
        }

        fn prepare_simultaneous_player_action(
            &self,
            _game: &GameState,
            ctx: &mut ExecutionContext,
        ) -> Result<Box<dyn crate::effects::SimultaneousEffectProposal>, ExecutionError> {
            let player = ctx.iteration.iterated_player.expect("iterated player");
            Ok(Box::new(AtomicBatchProposal {
                player,
                fail: player == PlayerId::from_index(1),
            }))
        }
    }

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    #[test]
    fn pending_simultaneous_optional_action_stops_first_apnap_prompt() {
        use std::{
            cell::{Cell, RefCell},
            rc::Rc,
        };
        struct Answers {
            ready: Rc<Cell<bool>>,
            pending: bool,
            calls: Rc<RefCell<Vec<PlayerId>>>,
        }
        impl crate::decision::DecisionMaker for Answers {
            fn decide_boolean(
                &mut self,
                game: &GameState,
                choice: &crate::decisions::context::BooleanContext,
            ) -> bool {
                assert!(
                    game.players.iter().all(|player| player.life == 20),
                    "all choices precede every player's action"
                );
                self.calls.borrow_mut().push(choice.player);
                self.pending = !self.ready.get();
                !self.pending
            }
            fn awaiting_choice(&self) -> bool {
                self.pending && !self.ready.get()
            }
        }
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Cara".into()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let cara = PlayerId::from_index(2);
        game.turn.active_player = bob;
        game.turn_store.turn_order = vec![alice, bob, cara];
        let source = game.new_object_id();
        let before_ids = game.next_object_id_counter();
        let before_random = game.irreversible_random_count();
        let ready = Rc::new(Cell::new(false));
        let calls = Rc::new(RefCell::new(vec![]));
        let mut answers = Answers {
            ready: ready.clone(),
            pending: false,
            calls: calls.clone(),
        };
        let mut ctx = ExecutionContext::new(source, alice, &mut answers);
        ctx.tag_player("retained", alice);
        let effect = Effect::new(ForPlayersEffect::new(
            PlayerFilter::Any,
            vec![Effect::may(vec![Effect::new(
                crate::effects::GainLifeEffect::new(
                    2,
                    crate::target::ChooseSpec::Player(PlayerFilter::IteratedPlayer),
                ),
            )])],
        ));
        let pending = execute_effect(&mut game, &effect, &mut ctx).unwrap();
        assert!(ctx.decision_maker.awaiting_choice());
        assert_eq!(
            *calls.borrow(),
            vec![bob],
            "later APNAP prompts cannot overwrite an unanswered first choice"
        );
        assert!(pending.events.is_empty());
        assert!(pending.execution_facts.is_empty());
        assert!(game.players.iter().all(|player| player.life == 20));
        assert_eq!(game.next_object_id_counter(), before_ids);
        assert_eq!(game.irreversible_random_count(), before_random);
        assert!(game.take_pending_trigger_events().is_empty());
        assert!(ctx.iteration.iterated_player.is_none());
        assert_eq!(
            ctx.tagged_players
                .get(&crate::tag::TagKey::from("retained"))
                .unwrap(),
            &vec![alice]
        );
        ready.set(true);
        calls.borrow_mut().clear();
        let resolved = execute_effect(&mut game, &effect, &mut ctx).unwrap();
        assert!(!ctx.decision_maker.awaiting_choice());
        assert_eq!(*calls.borrow(), vec![bob, cara, alice]);
        assert!(game.players.iter().all(|player| player.life == 22));
        let gains = resolved
            .events
            .iter()
            .filter_map(|event| event.downcast::<crate::events::LifeGainEvent>())
            .map(|event| (event.player, event.amount))
            .collect::<Vec<_>>();
        assert_eq!(gains, vec![(bob, 2), (cara, 2), (alice, 2)]);
        assert!(ctx.iteration.iterated_player.is_none());
        assert_eq!(
            ctx.tagged_players
                .get(&crate::tag::TagKey::from("retained"))
                .unwrap(),
            &vec![alice]
        );
    }

    #[test]
    fn pending_simultaneous_readonly_selection_does_not_reveal_or_ask_later_players() {
        use std::{
            cell::{Cell, RefCell},
            rc::Rc,
        };
        struct Answers {
            ready: Rc<Cell<bool>>,
            pending: bool,
            calls: Rc<RefCell<Vec<PlayerId>>>,
            views: Rc<RefCell<Vec<(PlayerId, Vec<crate::ids::ObjectId>)>>>,
        }
        impl crate::decision::DecisionMaker for Answers {
            fn decide_objects(
                &mut self,
                _: &GameState,
                choices: &crate::decisions::context::SelectObjectsContext,
            ) -> Vec<crate::ids::ObjectId> {
                self.calls.borrow_mut().push(choices.player);
                assert_eq!(choices.candidates.len(), 2);
                assert!(choices.candidates[0].legal);
                self.pending = !self.ready.get();
                if self.pending {
                    vec![]
                } else {
                    vec![choices.candidates[0].id]
                }
            }
            fn view_cards(
                &mut self,
                _: &GameState,
                viewer: PlayerId,
                cards: &[crate::ids::ObjectId],
                _: &crate::decisions::context::ViewCardsContext,
            ) {
                self.views.borrow_mut().push((viewer, cards.to_vec()));
            }
            fn awaiting_choice(&self) -> bool {
                self.pending && !self.ready.get()
            }
        }
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Cara".into()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let cara = PlayerId::from_index(2);
        game.turn.active_player = bob;
        game.turn_store.turn_order = vec![alice, bob, cara];
        let card =
            crate::card::CardBuilder::new(crate::ids::CardId::new(), "Readonly selection card")
                .card_types(vec![crate::types::CardType::Creature])
                .build();
        let cards = [alice, bob, cara].map(|owner| {
            [
                game.create_object_from_card(&card, owner, crate::zone::Zone::Hand),
                game.create_object_from_card(&card, owner, crate::zone::Zone::Hand),
            ]
        });
        let source = game.new_object_id();
        game.take_pending_trigger_events();
        let ready = Rc::new(Cell::new(false));
        let calls = Rc::new(RefCell::new(vec![]));
        let views = Rc::new(RefCell::new(vec![]));
        let mut answers = Answers {
            ready: ready.clone(),
            pending: false,
            calls: calls.clone(),
            views: views.clone(),
        };
        let mut ctx = ExecutionContext::new(source, alice, &mut answers);
        ctx.tag_player("retained", alice);
        let tag = crate::tag::TagKey::from("readonly-selected");
        let filter = crate::filter::ObjectFilter {
            zone: Some(crate::zone::Zone::Hand),
            owner: Some(PlayerFilter::IteratedPlayer),
            ..Default::default()
        };
        let choose = crate::effects::ChooseObjectsEffect::new(
            filter,
            crate::effect::ChoiceCount::exactly(1),
            PlayerFilter::IteratedPlayer,
            tag.clone(),
        )
        .in_zone(crate::zone::Zone::Hand);
        let effect = Effect::new(ForPlayersEffect::new(
            PlayerFilter::Any,
            vec![
                Effect::new(choose),
                Effect::new(crate::effects::RevealTaggedEffect::new(tag.clone())),
            ],
        ));
        let pending = execute_effect(&mut game, &effect, &mut ctx).unwrap();
        assert!(ctx.decision_maker.awaiting_choice());
        assert_eq!(*calls.borrow(), vec![bob]);
        assert!(pending.events.is_empty());
        assert!(pending.execution_facts.is_empty());
        assert!(ctx.get_tagged_all(tag.clone()).is_none());
        assert!(
            views.borrow().is_empty(),
            "no public reveal callback while first selection is pending"
        );
        assert!(
            ctx.get_tagged_all(crate::effects::PUBLIC_REVEALED_TAG)
                .is_none()
        );
        assert!(game.take_pending_trigger_events().is_empty());
        ready.set(true);
        calls.borrow_mut().clear();
        let resolved = execute_effect(&mut game, &effect, &mut ctx).unwrap();
        assert!(!ctx.decision_maker.awaiting_choice());
        assert_eq!(*calls.borrow(), vec![bob, cara, alice]);
        assert_eq!(
            views.borrow().len(),
            9,
            "one reveal to each of three viewers for each player"
        );
        assert_eq!(
            resolved
                .events
                .iter()
                .filter(|event| event.kind() == crate::events::EventKind::CardRevealed)
                .count(),
            3
        );
        assert_eq!(
            ctx.get_tagged_all(tag)
                .unwrap()
                .iter()
                .map(|snapshot| snapshot.object_id)
                .collect::<std::collections::HashSet<_>>(),
            cards.into_iter().map(|pair| pair[0]).collect()
        );
        assert_eq!(
            ctx.get_tagged_all(crate::effects::PUBLIC_REVEALED_TAG)
                .unwrap()
                .len(),
            3
        );
        assert!(
            cards
                .iter()
                .flatten()
                .all(|id| game.object(*id).unwrap().zone == crate::zone::Zone::Hand)
        );
        assert!(ctx.iteration.iterated_player.is_none());
        assert_eq!(
            ctx.tagged_players
                .get(&crate::tag::TagKey::from("retained"))
                .unwrap(),
            &vec![alice]
        );
    }

    #[test]
    fn i004_generic_each_player_choices_use_apnap_order() {
        let mut game = GameState::new(
            vec!["Alice".to_string(), "Bob".to_string(), "Cara".to_string()],
            20,
        );
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let cara = PlayerId::from_index(2);
        game.turn.active_player = cara;
        game.turn_store.turn_order = vec![alice, bob, cara];

        let source = game.new_object_id();
        let mut decisions = RecordChoiceOrder::default();
        let mut ctx = ExecutionContext::new(source, alice, &mut decisions);
        ForPlayersEffect::new(
            PlayerFilter::Any,
            vec![Effect::new(RecordIteratedPlayerChoice("choose"))],
        )
        .execute(&mut game, &mut ctx)
        .expect("each-player effect should resolve");

        assert_eq!(
            decisions.prompts,
            vec![
                (cara, "choose".to_string()),
                (alice, "choose".to_string()),
                (bob, "choose".to_string()),
            ]
        );
    }

    #[test]
    fn i004_generic_each_player_clauses_are_action_major() {
        let mut game = GameState::new(
            vec!["Alice".to_string(), "Bob".to_string(), "Cara".to_string()],
            20,
        );
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let cara = PlayerId::from_index(2);
        game.turn.active_player = bob;
        game.turn_store.turn_order = vec![alice, bob, cara];

        let source = game.new_object_id();
        let mut decisions = RecordChoiceOrder::default();
        let mut ctx = ExecutionContext::new(source, alice, &mut decisions);
        ForPlayersEffect::new(
            PlayerFilter::Any,
            vec![
                Effect::new(RecordIteratedPlayerChoice("first action")),
                Effect::new(RecordIteratedPlayerChoice("second action")),
            ],
        )
        .execute(&mut game, &mut ctx)
        .expect("each-player effect should resolve");

        assert_eq!(
            decisions.prompts,
            vec![
                (bob, "first action".to_string()),
                (cara, "first action".to_string()),
                (alice, "first action".to_string()),
                (bob, "second action".to_string()),
                (cara, "second action".to_string()),
                (alice, "second action".to_string()),
            ]
        );
    }

    #[test]
    fn sequential_player_loop_completes_each_body_in_turn_order() {
        let mut game = GameState::new(
            vec!["Alice".to_string(), "Bob".to_string(), "Cara".to_string()],
            20,
        );
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let cara = PlayerId::from_index(2);
        game.turn.active_player = bob;
        game.turn_store.turn_order = vec![alice, bob, cara];

        let source = game.new_object_id();
        let mut decisions = RecordChoiceOrder::default();
        let mut ctx = ExecutionContext::new(source, alice, &mut decisions);
        let mut effect = ForPlayersEffect::new(
            PlayerFilter::Any,
            vec![
                Effect::new(RecordIteratedPlayerChoice("first action")),
                Effect::new(RecordIteratedPlayerChoice("second action")),
            ],
        );
        effect.sequential = true;
        effect
            .execute(&mut game, &mut ctx)
            .expect("each-player effect should resolve");

        assert_eq!(
            decisions.prompts,
            vec![
                (bob, "first action".to_string()),
                (bob, "second action".to_string()),
                (cara, "first action".to_string()),
                (cara, "second action".to_string()),
                (alice, "first action".to_string()),
                (alice, "second action".to_string()),
            ]
        );
    }

    #[test]
    fn simultaneous_optional_child_choices_precede_any_commit() {
        use std::{cell::RefCell, rc::Rc};
        struct Answers {
            states: Rc<RefCell<Vec<(PlayerId, usize)>>>,
        }
        impl crate::decision::DecisionMaker for Answers {
            fn decide_boolean(
                &mut self,
                _: &GameState,
                _: &crate::decisions::context::BooleanContext,
            ) -> bool {
                true
            }
            fn decide_objects(
                &mut self,
                game: &GameState,
                choices: &crate::decisions::context::SelectObjectsContext,
            ) -> Vec<crate::ids::ObjectId> {
                self.states
                    .borrow_mut()
                    .push((choices.player, game.battlefield.len()));
                assert_eq!(choices.candidates.len(), 2);
                vec![choices.candidates[0].id]
            }
        }
        for optional in [false, true] {
            let mut game = setup_game();
            let alice = PlayerId::from_index(0);
            let bob = PlayerId::from_index(1);
            game.turn.active_player = alice;
            game.turn_store.turn_order = vec![alice, bob];
            let card = crate::card::CardBuilder::new(
                crate::ids::CardId::new(),
                "Simultaneous sacrifice candidate",
            )
            .card_types(vec![crate::types::CardType::Creature])
            .build();
            for owner in [alice, bob] {
                for _ in 0..2 {
                    game.create_object_from_card(&card, owner, crate::zone::Zone::Battlefield);
                }
            }
            let source = game.new_object_id();
            let states = Rc::new(RefCell::new(Vec::new()));
            let mut answers = Answers {
                states: states.clone(),
            };
            let mut ctx = ExecutionContext::new(source, alice, &mut answers);
            let child = Effect::new(crate::effects::SacrificeEffect::player(
                crate::filter::ObjectFilter::creature(),
                1,
                PlayerFilter::IteratedPlayer,
            ));
            let action = if optional {
                Effect::may(vec![child])
            } else {
                child
            };
            ForPlayersEffect::new(PlayerFilter::Any, vec![action])
                .execute(&mut game, &mut ctx)
                .unwrap();
            assert_eq!(game.battlefield.len(), 2);
            assert_eq!(
                *states.borrow(),
                vec![(alice, 4), (bob, 4)],
                "optional={optional}: child choices see the complete pre-action battlefield"
            );
        }
    }

    #[test]
    fn simultaneous_optional_children_preserve_instruction_boundaries() {
        struct Accept;
        impl crate::decision::DecisionMaker for Accept {
            fn decide_boolean(
                &mut self,
                _: &GameState,
                _: &crate::decisions::context::BooleanContext,
            ) -> bool {
                true
            }
        }
        for optional in [false, true] {
            let mut game = setup_game();
            let alice = PlayerId::from_index(0);
            let bob = PlayerId::from_index(1);
            game.turn.active_player = alice;
            game.turn_store.turn_order = vec![alice, bob];
            let source = game.new_object_id();
            let mut answers = Accept;
            let mut ctx = ExecutionContext::new(source, alice, &mut answers);
            let child = Effect::new(crate::effects::GainLifeEffect::new(
                crate::effect::Value::LifeTotal(PlayerFilter::You),
                crate::target::ChooseSpec::Player(PlayerFilter::IteratedPlayer),
            ));
            let children = vec![child.clone(), child];
            let actions = if optional {
                vec![Effect::may(children)]
            } else {
                children
            };
            let result = ForPlayersEffect::new(PlayerFilter::Any, actions)
                .execute(&mut game, &mut ctx)
                .unwrap();
            assert_eq!(game.player(alice).unwrap().life, 80);
            assert_eq!(
                game.player(bob).unwrap().life,
                80,
                "optional={optional}: complete first instruction for all players before preparing the second"
            );
            let gains = result
                .events
                .iter()
                .filter_map(|event| event.downcast::<crate::events::LifeGainEvent>())
                .map(|event| (event.player, event.amount))
                .collect::<Vec<_>>();
            assert_eq!(gains, vec![(alice, 20), (bob, 20), (alice, 40), (bob, 40)]);
        }
    }

    #[test]
    fn simultaneous_optional_wrapped_result_preserves_child_proposals() {
        struct Accept;
        impl crate::decision::DecisionMaker for Accept {
            fn decide_boolean(
                &mut self,
                _: &GameState,
                _: &crate::decisions::context::BooleanContext,
            ) -> bool {
                true
            }
        }
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        game.turn.active_player = alice;
        game.turn_store.turn_order = vec![alice, bob];
        let source = game.new_object_id();
        let mut answers = Accept;
        let mut ctx = ExecutionContext::new(source, alice, &mut answers);
        let action = Effect::with_id(
            41,
            Effect::may(vec![Effect::new(crate::effects::GainLifeEffect::new(
                crate::effect::Value::LifeTotal(PlayerFilter::You),
                crate::target::ChooseSpec::Player(PlayerFilter::IteratedPlayer),
            ))]),
        );
        ForPlayersEffect::new(PlayerFilter::Any, vec![action])
            .execute(&mut game, &mut ctx)
            .unwrap();
        assert_eq!(game.player(alice).unwrap().life, 40);
        assert_eq!(
            game.player(bob).unwrap().life,
            40,
            "outcome annotation cannot hide an optional program from phased scheduling"
        );
        let outcome = ctx
            .effect_outcomes
            .get(&crate::effect::EffectId(41))
            .expect("annotated optional outcome retained");
        assert_eq!(
            outcome.player_counts(),
            Some([(alice, 20), (bob, 20)].as_slice()),
            "completed optional result retains the collective metrics for later instructions"
        );
        assert!(
            outcome
                .execution_facts
                .contains(&crate::effect::ExecutionFact::Accepted)
        );
        let followup = Effect::new(crate::effects::GainLifeEffect::new(
            crate::effect::Value::EffectMetric {
                effect_id: crate::effect::EffectId(41),
                source: ironsmith_core::EffectMetricSource::Outcome,
                metric: ironsmith_core::EffectMetric::GreatestPlayerCount,
            },
            crate::target::ChooseSpec::Player(PlayerFilter::You),
        ));
        execute_effect(&mut game, &followup, &mut ctx).unwrap();
        assert_eq!(
            game.player(alice).unwrap().life,
            60,
            "a later action reads the completed optional collection's greatest count"
        );
    }

    #[test]
    fn simultaneous_optional_first_action_choices_are_player_major() {
        use std::{cell::RefCell, rc::Rc};
        struct Answers {
            calls: Rc<RefCell<Vec<(PlayerId, &'static str)>>>,
        }
        impl crate::decision::DecisionMaker for Answers {
            fn decide_boolean(
                &mut self,
                game: &GameState,
                choice: &crate::decisions::context::BooleanContext,
            ) -> bool {
                assert_eq!(game.battlefield.len(), 4);
                self.calls.borrow_mut().push((choice.player, "accept"));
                true
            }
            fn decide_objects(
                &mut self,
                game: &GameState,
                choice: &crate::decisions::context::SelectObjectsContext,
            ) -> Vec<crate::ids::ObjectId> {
                assert_eq!(game.battlefield.len(), 4);
                self.calls.borrow_mut().push((choice.player, "sacrifice"));
                vec![choice.candidates[0].id]
            }
        }
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        game.turn.active_player = alice;
        game.turn_store.turn_order = vec![alice, bob];
        let card =
            crate::card::CardBuilder::new(crate::ids::CardId::new(), "Optional APNAP candidate")
                .card_types(vec![crate::types::CardType::Creature])
                .build();
        for owner in [alice, bob] {
            for _ in 0..2 {
                game.create_object_from_card(&card, owner, crate::zone::Zone::Battlefield);
            }
        }
        let source = game.new_object_id();
        let calls = Rc::new(RefCell::new(vec![]));
        let mut answers = Answers {
            calls: calls.clone(),
        };
        let mut ctx = ExecutionContext::new(source, alice, &mut answers);
        ForPlayersEffect::new(
            PlayerFilter::Any,
            vec![Effect::may(vec![Effect::new(
                crate::effects::SacrificeEffect::player(
                    crate::filter::ObjectFilter::creature(),
                    1,
                    PlayerFilter::IteratedPlayer,
                ),
            )])],
        )
        .execute(&mut game, &mut ctx)
        .unwrap();
        assert_eq!(
            *calls.borrow(),
            vec![
                (alice, "accept"),
                (alice, "sacrifice"),
                (bob, "accept"),
                (bob, "sacrifice")
            ],
            "101.4/101.4c: first player's choices precede the next player's choices for this action"
        );
        assert_eq!(game.battlefield.len(), 2);
    }

    #[test]
    fn nested_optional_phases_retain_only_accepting_players() {
        struct Answers {
            calls: Vec<PlayerId>,
        }
        impl crate::decision::DecisionMaker for Answers {
            fn decide_boolean(
                &mut self,
                _: &GameState,
                choice: &crate::decisions::context::BooleanContext,
            ) -> bool {
                self.calls.push(choice.player);
                choice.player == PlayerId::from_index(0)
            }
        }
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        game.turn.active_player = alice;
        game.turn_store.turn_order = vec![alice, bob];
        let source = game.new_object_id();
        let mut answers = Answers { calls: vec![] };
        let mut ctx = ExecutionContext::new(source, alice, &mut answers);
        let gain = Effect::new(crate::effects::GainLifeEffect::new(
            crate::effect::Value::LifeTotal(PlayerFilter::You),
            crate::target::ChooseSpec::Player(PlayerFilter::IteratedPlayer),
        ));
        let action = Effect::with_id(
            42,
            Effect::may(vec![
                gain.clone(),
                Effect::with_id(43, Effect::may(vec![gain])),
            ]),
        );
        let result = ForPlayersEffect::new(PlayerFilter::Any, vec![action])
            .execute(&mut game, &mut ctx)
            .unwrap();
        assert_eq!(game.player(alice).unwrap().life, 80);
        assert_eq!(game.player(bob).unwrap().life, 20);
        assert_eq!(result.events.len(), 2);
        assert!(
            ctx.effect_outcomes
                .get(&crate::effect::EffectId(42))
                .unwrap()
                .execution_facts
                .contains(&crate::effect::ExecutionFact::Declined)
        );
        assert!(
            !ctx.effect_outcomes
                .contains_key(&crate::effect::EffectId(43)),
            "declining outer action never offers the nested action"
        );
        drop(ctx);
        assert_eq!(answers.calls, vec![alice, bob, alice]);
    }

    #[test]
    fn pending_optional_later_phase_restores_prior_commits_and_action_limit() {
        use std::{
            cell::{Cell, RefCell},
            rc::Rc,
        };
        struct Answers {
            ready: Rc<Cell<bool>>,
            pending: bool,
            calls: Rc<RefCell<Vec<(PlayerId, &'static str)>>>,
        }
        impl crate::decision::DecisionMaker for Answers {
            fn decide_boolean(
                &mut self,
                _: &GameState,
                choice: &crate::decisions::context::BooleanContext,
            ) -> bool {
                self.calls.borrow_mut().push((choice.player, "accept"));
                true
            }
            fn decide_objects(
                &mut self,
                game: &GameState,
                choice: &crate::decisions::context::SelectObjectsContext,
            ) -> Vec<crate::ids::ObjectId> {
                assert_eq!(game.battlefield.len(), 4);
                assert!(game.players.iter().all(|player| player.life == 22));
                self.calls.borrow_mut().push((choice.player, "sacrifice"));
                self.pending = !self.ready.get();
                if self.pending {
                    vec![]
                } else {
                    vec![choice.candidates[0].id]
                }
            }
            fn awaiting_choice(&self) -> bool {
                self.pending && !self.ready.get()
            }
        }
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        game.turn.active_player = alice;
        game.turn_store.turn_order = vec![alice, bob];
        let card =
            crate::card::CardBuilder::new(crate::ids::CardId::new(), "Pending optional candidate")
                .card_types(vec![crate::types::CardType::Creature])
                .build();
        for owner in [alice, bob] {
            for _ in 0..2 {
                game.create_object_from_card(&card, owner, crate::zone::Zone::Battlefield);
            }
        }
        let source = game.new_object_id();
        game.take_pending_trigger_events();
        let identity = crate::triggers::TriggerIdentity(1167);
        let limit = crate::effects::DoThisLimit {
            source,
            trigger_identity: identity,
            limit: 1,
        };
        let ready = Rc::new(Cell::new(false));
        let calls = Rc::new(RefCell::new(vec![]));
        let mut answers = Answers {
            ready: ready.clone(),
            pending: false,
            calls: calls.clone(),
        };
        let mut ctx = ExecutionContext::new(source, alice, &mut answers);
        ctx.do_this_limit = Some(limit);
        let action = Effect::with_id(
            44,
            Effect::may(vec![
                Effect::new(crate::effects::GainLifeEffect::new(
                    2,
                    crate::target::ChooseSpec::Player(PlayerFilter::IteratedPlayer),
                )),
                Effect::new(crate::effects::SacrificeEffect::player(
                    crate::filter::ObjectFilter::creature(),
                    1,
                    PlayerFilter::IteratedPlayer,
                )),
            ]),
        );
        let effect = ForPlayersEffect::new(PlayerFilter::Any, vec![action]);
        let pending = effect.execute(&mut game, &mut ctx).unwrap();
        assert!(ctx.decision_maker.awaiting_choice());
        assert!(pending.events.is_empty());
        assert!(pending.execution_facts.is_empty());
        assert_eq!(
            *calls.borrow(),
            vec![(alice, "accept"), (bob, "accept"), (alice, "sacrifice")]
        );
        assert!(game.players.iter().all(|player| player.life == 20));
        assert_eq!(game.battlefield.len(), 4);
        assert_eq!(game.do_this_action_count_this_turn(source, identity), 0);
        assert_eq!(ctx.do_this_limit, Some(limit));
        assert!(
            !ctx.effect_outcomes
                .contains_key(&crate::effect::EffectId(44))
        );
        assert!(game.take_pending_trigger_events().is_empty());
        ready.set(true);
        calls.borrow_mut().clear();
        let resolved = effect.execute(&mut game, &mut ctx).unwrap();
        assert_eq!(
            *calls.borrow(),
            vec![
                (alice, "accept"),
                (bob, "accept"),
                (alice, "sacrifice"),
                (bob, "sacrifice")
            ]
        );
        assert!(game.players.iter().all(|player| player.life == 22));
        assert_eq!(game.battlefield.len(), 2);
        assert_eq!(game.do_this_action_count_this_turn(source, identity), 1);
        assert_eq!(
            resolved
                .events
                .iter()
                .filter(|event| event.downcast::<crate::events::LifeGainEvent>().is_some())
                .count(),
            2
        );
        assert!(
            ctx.effect_outcomes
                .get(&crate::effect::EffectId(44))
                .unwrap()
                .execution_facts
                .contains(&crate::effect::ExecutionFact::Accepted)
        );
    }

    #[test]
    fn nested_same_id_optional_result_keeps_child_value_for_followup() {
        struct Accept;
        impl crate::decision::DecisionMaker for Accept {
            fn decide_boolean(
                &mut self,
                _: &GameState,
                _: &crate::decisions::context::BooleanContext,
            ) -> bool {
                true
            }
        }
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        game.turn.active_player = alice;
        game.turn_store.turn_order = vec![alice, bob];
        let source = game.new_object_id();
        let mut answers = Accept;
        let mut ctx = ExecutionContext::new(source, alice, &mut answers);
        let id = crate::effect::EffectId(45);
        ctx.store_outcome(id, EffectOutcome::count(99));
        let gain = |amount| {
            Effect::new(crate::effects::GainLifeEffect::new(
                amount,
                crate::target::ChooseSpec::Player(PlayerFilter::IteratedPlayer),
            ))
        };
        let action = Effect::with_id(45, Effect::may(vec![Effect::with_id(45, gain(2)), gain(3)]));
        let followup = Effect::if_then(
            id,
            crate::effect::EffectPredicate::Value(crate::effect::Comparison::GreaterThan(1)),
            vec![gain(5)],
        );
        ForPlayersEffect::new(PlayerFilter::Any, vec![action, followup])
            .execute(&mut game, &mut ctx)
            .unwrap();
        assert!(
            game.players.iter().all(|player| player.life == 30),
            "if-you-do followup uses the child count, not stale99 or outer heterogeneous aggregate"
        );
        let result = ctx.effect_outcomes.get(&id).unwrap();
        assert_eq!(result.count_or_zero(), 2);
        assert_eq!(
            result.player_counts(),
            Some([(alice, 2), (bob, 2)].as_slice())
        );
        assert!(
            !result
                .execution_facts
                .contains(&crate::effect::ExecutionFact::Accepted),
            "outer optional aggregate cannot overwrite independently annotated same-ID child"
        );
    }

    fn assert_wrapped_player_action_preserves_proposal(kind: &str, optional: bool) {
        struct Accept;
        impl crate::decision::DecisionMaker for Accept {
            fn decide_boolean(
                &mut self,
                _: &GameState,
                _: &crate::decisions::context::BooleanContext,
            ) -> bool {
                true
            }
        }
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        game.turn.active_player = alice;
        game.turn_store.turn_order = vec![alice, bob];
        let card = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Wrapper source")
            .card_types(vec![crate::types::CardType::Artifact])
            .build();
        let source = game.create_object_from_card(&card, alice, crate::zone::Zone::Battlefield);
        let mut answers = Accept;
        let mut ctx = ExecutionContext::new(source, alice, &mut answers);
        let gain = Effect::new(crate::effects::GainLifeEffect::new(
            crate::effect::Value::LifeTotal(PlayerFilter::You),
            crate::target::ChooseSpec::Player(PlayerFilter::IteratedPlayer),
        ));
        let child = if optional {
            Effect::may(vec![gain])
        } else {
            gain
        };
        let wrapped = match kind {
            "tagged" => child.tag("wrapper-result"),
            "source" => Effect::new(crate::effects::ExecuteWithSourceEffect::new(
                crate::target::ChooseSpec::Source,
                child,
            )),
            "rewrite" => Effect::new(crate::effects::LocalRewriteEffect::new(child, vec![])),
            _ => unreachable!(),
        };
        let result = ForPlayersEffect::new(PlayerFilter::Any, vec![wrapped])
            .execute(&mut game, &mut ctx)
            .unwrap();
        assert_eq!(game.player(alice).unwrap().life, 40);
        assert_eq!(
            game.player(bob).unwrap().life,
            40,
            "{kind}, optional={optional}: wrapper retains immutable child amount"
        );
        let gains = result
            .events
            .iter()
            .filter_map(|event| event.downcast::<crate::events::LifeGainEvent>())
            .map(|event| (event.player, event.amount))
            .collect::<Vec<_>>();
        assert_eq!(gains, vec![(alice, 20), (bob, 20)]);
        assert_eq!(ctx.source, source);
        assert!(ctx.source_snapshot.is_none());
        assert!(ctx.additional_replacement_effects().is_empty());
    }

    #[test]
    fn simultaneous_tagged_wrapper_preserves_mandatory_proposal() {
        assert_wrapped_player_action_preserves_proposal("tagged", false);
    }
    #[test]
    fn simultaneous_tagged_wrapper_preserves_optional_proposal() {
        assert_wrapped_player_action_preserves_proposal("tagged", true);
    }
    #[test]
    fn simultaneous_source_wrapper_preserves_mandatory_proposal() {
        assert_wrapped_player_action_preserves_proposal("source", false);
    }
    #[test]
    fn simultaneous_source_wrapper_preserves_optional_proposal() {
        assert_wrapped_player_action_preserves_proposal("source", true);
    }
    #[test]
    fn simultaneous_rewrite_wrapper_preserves_mandatory_proposal() {
        assert_wrapped_player_action_preserves_proposal("rewrite", false);
    }
    #[test]
    fn simultaneous_rewrite_wrapper_preserves_optional_proposal() {
        assert_wrapped_player_action_preserves_proposal("rewrite", true);
    }

    #[test]
    fn scoped_optional_multiple_phases_keep_source_and_annotation_across_wrapper_orders() {
        struct Answers {
            accepted: [bool; 2],
            rebound: crate::ids::ObjectId,
            calls: Vec<PlayerId>,
        }
        impl crate::decision::DecisionMaker for Answers {
            fn decide_boolean(
                &mut self,
                game: &GameState,
                choice: &crate::decisions::context::BooleanContext,
            ) -> bool {
                assert!(game.players.iter().all(|player| player.life == 20));
                assert_eq!(choice.source, Some(self.rebound));
                self.calls.push(choice.player);
                self.accepted[game
                    .players
                    .iter()
                    .position(|p| p.id == choice.player)
                    .unwrap()]
            }
        }
        let mut checked = 0;
        for a in 0..4 {
            for b in 0..4 {
                for c in 0..4 {
                    for d in 0..4 {
                        let order = [a, b, c, d];
                        if (0..4).any(|i| (i + 1..4).any(|j| order[i] == order[j])) {
                            continue;
                        }
                        for mask in 0..4 {
                            let accepted = [mask & 1 != 0, mask & 2 != 0];
                            let mut game = setup_game();
                            let alice = PlayerId::from_index(0);
                            let bob = PlayerId::from_index(1);
                            game.turn.active_player = bob;
                            game.turn_store.turn_order = vec![alice, bob];
                            let card = crate::card::CardBuilder::new(
                                crate::ids::CardId::new(),
                                "Scoped action source",
                            )
                            .card_types(vec![crate::types::CardType::Artifact])
                            .build();
                            let source = game.create_object_from_card(
                                &card,
                                alice,
                                crate::zone::Zone::Battlefield,
                            );
                            let rebound = game.create_object_from_card(
                                &card,
                                bob,
                                crate::zone::Zone::Battlefield,
                            );
                            let mut answers = Answers {
                                accepted,
                                rebound,
                                calls: vec![],
                            };
                            let mut ctx = ExecutionContext::new(source, alice, &mut answers);
                            ctx.store_outcome(
                                crate::effect::EffectId(46),
                                EffectOutcome::count(999),
                            );
                            let gain = Effect::new(crate::effects::GainLifeEffect::new(
                                crate::effect::Value::LifeTotal(PlayerFilter::You),
                                crate::target::ChooseSpec::Player(PlayerFilter::IteratedPlayer),
                            ));
                            let mut action = Effect::may(vec![gain.clone(), gain]);
                            for kind in order {
                                action = match kind {
                                    0 => action.tag("scoped-phase-result"),
                                    1 => Effect::with_id(46, action),
                                    2 => Effect::new(crate::effects::ExecuteWithSourceEffect::new(
                                        crate::target::ChooseSpec::SpecificObject(rebound),
                                        action,
                                    )),
                                    3 => Effect::new(crate::effects::LocalRewriteEffect::new(
                                        action,
                                        vec![],
                                    )),
                                    _ => unreachable!(),
                                };
                            }
                            let result = ForPlayersEffect::new(PlayerFilter::Any, vec![action])
                                .execute(&mut game, &mut ctx)
                                .unwrap();
                            let second_amount = if accepted[0] { 40 } else { 20 };
                            assert_eq!(
                                game.player(alice).unwrap().life,
                                if accepted[0] { 80 } else { 20 },
                                "order={order:?}, mask={mask}"
                            );
                            assert_eq!(
                                game.player(bob).unwrap().life,
                                if accepted[1] { 40 + second_amount } else { 20 },
                                "order={order:?}, mask={mask}"
                            );
                            let expected = [20, second_amount]
                                .into_iter()
                                .flat_map(|amount| {
                                    [bob, alice]
                                        .into_iter()
                                        .filter(move |player| accepted[usize::from(*player == bob)])
                                        .map(move |player| (player, amount as u32, Some(rebound)))
                                })
                                .collect::<Vec<_>>();
                            let actual = result
                                .events
                                .iter()
                                .filter_map(|event| {
                                    event.downcast::<crate::events::LifeGainEvent>()
                                })
                                .map(|event| (event.player, event.amount, event.source))
                                .collect::<Vec<_>>();
                            assert_eq!(actual, expected);
                            let outcome = ctx
                                .effect_outcomes
                                .get(&crate::effect::EffectId(46))
                                .unwrap();
                            assert!(outcome.execution_facts.contains(&if accepted[0] {
                                crate::effect::ExecutionFact::Accepted
                            } else {
                                crate::effect::ExecutionFact::Declined
                            }));
                            assert_eq!(ctx.source, source);
                            assert!(ctx.source_snapshot.is_none());
                            assert!(ctx.iteration.iterated_player.is_none());
                            assert!(ctx.additional_replacement_effects().is_empty());
                            drop(ctx);
                            assert_eq!(answers.calls, vec![bob, alice]);
                            checked += 1;
                        }
                    }
                }
            }
        }
        assert_eq!(checked, 96);
    }

    #[test]
    fn scoped_optional_sacrifice_redirects_and_retains_actual_tagged_collection() {
        struct Answers {
            selected: Vec<crate::ids::ObjectId>,
            calls: Vec<(PlayerId, &'static str)>,
            rebound: crate::ids::ObjectId,
        }
        impl crate::decision::DecisionMaker for Answers {
            fn decide_boolean(
                &mut self,
                game: &GameState,
                choice: &crate::decisions::context::BooleanContext,
            ) -> bool {
                assert_eq!(game.battlefield.len(), 6);
                assert_eq!(choice.source, Some(self.rebound));
                self.calls.push((choice.player, "accept"));
                true
            }
            fn decide_objects(
                &mut self,
                game: &GameState,
                choice: &crate::decisions::context::SelectObjectsContext,
            ) -> Vec<crate::ids::ObjectId> {
                assert_eq!(game.battlefield.len(), 6);
                assert_eq!(choice.candidates.len(), 2);
                self.calls.push((choice.player, "sacrifice"));
                self.selected.push(choice.candidates[0].id);
                vec![choice.candidates[0].id]
            }
        }
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        game.turn.active_player = bob;
        game.turn_store.turn_order = vec![alice, bob];
        let artifact =
            crate::card::CardBuilder::new(crate::ids::CardId::new(), "Scoped redirect source")
                .card_types(vec![crate::types::CardType::Artifact])
                .build();
        let source = game.create_object_from_card(&artifact, alice, crate::zone::Zone::Battlefield);
        let rebound = game.create_object_from_card(&artifact, bob, crate::zone::Zone::Battlefield);
        let creature =
            crate::card::CardBuilder::new(crate::ids::CardId::new(), "Scoped sacrifice candidate")
                .card_types(vec![crate::types::CardType::Creature])
                .build();
        for owner in [alice, bob] {
            for _ in 0..2 {
                game.create_object_from_card(&creature, owner, crate::zone::Zone::Battlefield);
            }
        }
        game.take_pending_trigger_events();
        let mut answers = Answers {
            selected: vec![],
            calls: vec![],
            rebound,
        };
        let mut ctx = ExecutionContext::new(source, alice, &mut answers);
        ctx.tag_object(
            "retained",
            crate::snapshot::ObjectSnapshot::from_object(game.object(source).unwrap(), &game),
        );
        let replacement = ironsmith_core::RegisterZoneReplacementEffect::new(
            crate::target::ChooseSpec::All(crate::filter::ObjectFilter::creature()),
            Some(crate::zone::Zone::Battlefield),
            Some(crate::zone::Zone::Graveyard),
            crate::zone::Zone::Exile,
            crate::effects::ReplacementApplyMode::OneShot,
        );
        let action = Effect::new(crate::effects::ExecuteWithSourceEffect::new(
            crate::target::ChooseSpec::SpecificObject(rebound),
            Effect::new(crate::effects::LocalRewriteEffect::new(
                Effect::may(vec![Effect::new(crate::effects::SacrificeEffect::player(
                    crate::filter::ObjectFilter::creature(),
                    1,
                    PlayerFilter::IteratedPlayer,
                ))]),
                vec![replacement],
            )),
        ))
        .tag("scoped-sacrificed");
        ForPlayersEffect::new(PlayerFilter::Any, vec![action])
            .execute(&mut game, &mut ctx)
            .unwrap();
        assert_eq!(game.battlefield.len(), 4);
        assert!(
            game.players
                .iter()
                .all(|player| player.graveyard.is_empty())
        );
        assert_eq!(game.exile.len(), 2);
        let tagged = ctx
            .get_tagged_all("scoped-sacrificed")
            .unwrap()
            .iter()
            .map(|snapshot| snapshot.object_id)
            .collect::<Vec<_>>();
        assert_eq!(tagged.len(), 2);
        assert!(
            ctx.get_tagged_all("retained")
                .unwrap()
                .iter()
                .any(|snapshot| snapshot.object_id == source)
        );
        assert_eq!(ctx.source, source);
        assert!(ctx.source_snapshot.is_none());
        assert!(ctx.additional_replacement_effects().is_empty());
        let events = game
            .take_pending_trigger_events()
            .into_iter()
            .filter_map(|event| event.downcast::<crate::events::ZoneChangeEvent>().cloned())
            .collect::<Vec<_>>();
        assert_eq!(events.len(), 2);
        assert!(
            events
                .iter()
                .all(|event| event.from == crate::zone::Zone::Battlefield
                    && event.to == crate::zone::Zone::Exile)
        );
        drop(ctx);
        assert_eq!(
            answers.calls,
            vec![
                (bob, "accept"),
                (bob, "sacrifice"),
                (alice, "accept"),
                (alice, "sacrifice")
            ]
        );
        assert!(
            answers
                .selected
                .iter()
                .all(|object| tagged.contains(object))
        );
    }

    #[test]
    fn simultaneous_optional_child_uses_pre_action_amount() {
        struct Accept;
        impl crate::decision::DecisionMaker for Accept {
            fn decide_boolean(
                &mut self,
                game: &GameState,
                _: &crate::decisions::context::BooleanContext,
            ) -> bool {
                assert!(game.players.iter().all(|player| player.life == 20));
                true
            }
        }
        for optional in [false, true] {
            let mut game = setup_game();
            let alice = PlayerId::from_index(0);
            let bob = PlayerId::from_index(1);
            game.turn.active_player = alice;
            game.turn_store.turn_order = vec![alice, bob];
            let source = game.new_object_id();
            let mut answers = Accept;
            let mut ctx = ExecutionContext::new(source, alice, &mut answers);
            let child = Effect::new(crate::effects::GainLifeEffect::new(
                crate::effect::Value::LifeTotal(PlayerFilter::You),
                crate::target::ChooseSpec::Player(PlayerFilter::IteratedPlayer),
            ));
            let action = if optional {
                Effect::may(vec![child])
            } else {
                child
            };
            let result = ForPlayersEffect::new(PlayerFilter::Any, vec![action])
                .execute(&mut game, &mut ctx)
                .expect("simultaneous action resolves");
            assert_eq!(game.player(alice).unwrap().life, 40);
            assert_eq!(
                game.player(bob).unwrap().life,
                40,
                "optional={optional}: accepted optional action retains the child's pre-action proposal semantics"
            );
            let gains = result
                .events
                .iter()
                .filter_map(|event| event.downcast::<crate::events::LifeGainEvent>())
                .map(|event| (event.player, event.amount))
                .collect::<Vec<_>>();
            assert_eq!(gains, vec![(alice, 20), (bob, 20)]);
        }
    }

    #[test]
    fn i004_generic_each_player_action_uses_one_immutable_proposal_state() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.new_object_id();
        let provenance = game
            .provenance_graph_mut()
            .alloc_root_event(crate::events::EventKind::LifeLoss);
        let mut ctx = ExecutionContext::new_default(source, alice).with_provenance(provenance);
        assert_ne!(provenance, crate::provenance::ProvNodeId::default());

        let result = ForPlayersEffect::new(
            PlayerFilter::Any,
            vec![Effect::lose_life_player(
                crate::effect::Value::LifeTotal(PlayerFilter::You),
                PlayerFilter::IteratedPlayer,
            )],
        )
        .execute(&mut game, &mut ctx)
        .expect("simultaneous each-player life loss should resolve");

        assert_eq!(game.player(alice).expect("alice").life, 0);
        assert_eq!(
            game.player(bob).expect("bob").life,
            0,
            "Bob's proposal must use Alice's pre-action life total"
        );
        assert_eq!(
            result.player_counts(),
            Some([(alice, 20), (bob, 20)].as_slice())
        );
        assert_eq!(result.events.len(), 2);
        let physical_ids = result
            .events
            .iter()
            .map(|event| event.provenance())
            .collect::<std::collections::HashSet<_>>();
        assert_eq!(
            physical_ids.len(),
            2,
            "each physical loss has its own identity"
        );
        assert!(
            result.events.iter().all(|event| {
                game.provenance_graph()
                    .node(event.provenance())
                    .is_some_and(|node| {
                        node.parent == Some(ctx.provenance)
                            && node.kind
                                == crate::provenance::ProvenanceNodeKind::DerivedEvent {
                                    kind: crate::events::EventKind::LifeLoss,
                                }
                    })
            }),
            "physical losses retain their shared immutable proposal ancestry"
        );
        assert!(
            game.player(alice).expect("alice").is_in_game()
                && game.player(bob).expect("bob").is_in_game(),
            "state-based actions are checked only after the whole batch resolves"
        );
    }

    #[test]
    fn quantified_damage_uses_each_iterated_players_own_life_total() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        game.player_mut(bob).expect("bob").life = 7;
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);
        let amount = crate::effect::Value::HalfRoundedDown(Box::new(
            crate::effect::Value::LifeTotal(PlayerFilter::IteratedPlayer),
        ));

        ForPlayersEffect::new(
            PlayerFilter::Any,
            vec![Effect::deal_damage(
                amount,
                crate::target::ChooseSpec::Player(PlayerFilter::IteratedPlayer),
            )],
        )
        .execute(&mut game, &mut ctx)
        .expect("each player's own life total should resolve inside the loop");

        assert_eq!(game.player(alice).expect("alice").life, 10);
        assert_eq!(game.player(bob).expect("bob").life, 4);
    }

    #[test]
    fn i004_simultaneous_proposal_commit_is_atomic_on_error() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let error = ForPlayersEffect::new(PlayerFilter::Any, vec![Effect::new(AtomicBatchProbe)])
            .execute(&mut game, &mut ctx)
            .expect_err("second proposal should fail");

        assert_eq!(
            error,
            ExecutionError::Impossible("probe failure".to_string())
        );
        assert_eq!(game.player(alice).expect("alice").life, 20);
        assert_eq!(game.player(bob).expect("bob").life, 20);
    }

    #[test]
    fn for_players_sums_count_results_across_players() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = ForPlayersEffect::new(
            PlayerFilter::Any,
            vec![Effect::lose_life_player(1, PlayerFilter::IteratedPlayer)],
        );
        let result = effect
            .execute(&mut game, &mut ctx)
            .expect("effect should resolve");

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(2));
        assert_eq!(game.player(alice).expect("alice").life, 19);
        assert_eq!(game.player(PlayerId::from_index(1)).expect("bob").life, 19);
    }

    #[test]
    fn each_player_unless_pays_asks_and_resolves_for_each_iterated_player() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.new_object_id();
        let mut decisions = FirstPlayerPays::default();
        let effect = ForPlayersEffect::new(
            PlayerFilter::Any,
            vec![Effect::new(
                crate::effects::UnlessPaysEffect::new_total_cost(
                    vec![Effect::lose_life_player(5, PlayerFilter::IteratedPlayer)],
                    PlayerFilter::IteratedPlayer,
                    crate::cost::TotalCost::from_cost(crate::costs::Cost::life(1)),
                ),
            )],
        );

        {
            let mut ctx = ExecutionContext::new(source, alice, &mut decisions);
            effect
                .execute(&mut game, &mut ctx)
                .expect("each-player unless-payment should resolve");
        }

        assert_eq!(decisions.prompted, [alice, bob]);
        assert_eq!(
            game.player(alice).expect("alice").life,
            19,
            "Alice pays 1 life and prevents her consequence"
        );
        assert_eq!(
            game.player(bob).expect("bob").life,
            15,
            "Bob declines and receives only his own consequence"
        );
    }

    #[test]
    fn for_players_records_per_player_count_partitions() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = ForPlayersEffect::new(
            PlayerFilter::Any,
            vec![Effect::lose_life_player(
                crate::effect::Value::Fixed(1),
                PlayerFilter::IteratedPlayer,
            )],
        );
        let result = effect
            .execute(&mut game, &mut ctx)
            .expect("effect should resolve");

        assert_eq!(
            result.player_counts(),
            Some([(alice, 1), (bob, 1)].as_slice())
        );
    }

    #[test]
    fn for_players_records_per_player_affected_object_memory_partitions() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let alice_card = game.new_object_id();
        let bob_card = game.new_object_id();
        let alice_memory = {
            let mut snapshot = crate::snapshot::ObjectSnapshot::public_placeholder(
                alice_card,
                crate::ids::StableId::from(alice_card),
                alice,
                alice,
                crate::zone::Zone::Library,
            );
            snapshot.name = "Alice Card".to_string();
            snapshot.power = None;
            snapshot.toughness = None;
            snapshot.linked_face_mana_value = Some((1) as u32);
            snapshot.card_types = vec![crate::types::CardType::Creature];
            snapshot.colors = crate::color::ColorSet::COLORLESS;
            snapshot.subtypes = Vec::new();
            snapshot.is_token = false;
            snapshot
        };
        let bob_memory = {
            let mut snapshot = crate::snapshot::ObjectSnapshot::public_placeholder(
                bob_card,
                crate::ids::StableId::from(bob_card),
                bob,
                bob,
                crate::zone::Zone::Library,
            );
            snapshot.name = "Bob Card".to_string();
            snapshot.power = None;
            snapshot.toughness = None;
            snapshot.linked_face_mana_value = Some((2) as u32);
            snapshot.card_types = vec![crate::types::CardType::Instant];
            snapshot.colors = crate::color::ColorSet::COLORLESS;
            snapshot.subtypes = Vec::new();
            snapshot.is_token = false;
            snapshot
        };

        let result = EffectOutcome::aggregate_summing_counts(vec![
            EffectOutcome::count(1)
                .with_affected_object_memory(vec![alice_memory.clone()])
                .with_player_affected_object_memory(vec![(alice, vec![alice_memory])]),
            EffectOutcome::count(1)
                .with_affected_object_memory(vec![bob_memory.clone()])
                .with_player_affected_object_memory(vec![(bob, vec![bob_memory])]),
        ]);

        let partitions = result
            .player_affected_object_memory()
            .expect("per-player affected memory");
        assert_eq!(partitions.len(), 2);
        assert_eq!(partitions[0].0, alice);
        assert_eq!(partitions[0].1[0].controller, alice);
        assert_eq!(partitions[1].0, bob);
        assert_eq!(partitions[1].1[0].controller, bob);

        let effect = ForPlayersEffect::new(PlayerFilter::Any, Vec::new());
        let empty_result = effect
            .execute(&mut game, &mut ctx)
            .expect("empty per-player effect should resolve");
        assert!(empty_result.player_affected_object_memory().is_none());
    }

    #[test]
    fn for_each_opponent_reveal_keeps_each_opponents_revealed_card_partitioned() {
        let mut game = GameState::new(
            vec!["Alice".to_string(), "Bob".to_string(), "Cara".to_string()],
            20,
        );
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let cara = PlayerId::from_index(2);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let bob_card = crate::card::CardBuilder::new(crate::ids::CardId::from_raw(1001), "Bob Top")
            .card_types(vec![crate::types::CardType::Creature])
            .build();
        let cara_card =
            crate::card::CardBuilder::new(crate::ids::CardId::from_raw(1002), "Cara Top")
                .card_types(vec![crate::types::CardType::Instant])
                .build();
        let bob_id = game.create_object_from_card(&bob_card, bob, crate::zone::Zone::Library);
        let cara_id = game.create_object_from_card(&cara_card, cara, crate::zone::Zone::Library);

        let effect = ForPlayersEffect::new(
            PlayerFilter::Opponent,
            vec![Effect::reveal_top_cards(
                PlayerFilter::IteratedPlayer,
                crate::effect::Value::Fixed(1),
                crate::tag::TagKey::from("revealed"),
            )],
        );
        let result = effect
            .execute(&mut game, &mut ctx)
            .expect("for each opponent reveal should resolve");

        assert_eq!(result.events.len(), 2);
        assert_eq!(
            result.affected_object_memory().map(|memory| memory.len()),
            Some(2)
        );
        let partitions = result
            .player_affected_object_memory()
            .expect("per-player reveal partitions");
        assert_eq!(partitions.len(), 2);
        assert_eq!(partitions[0].0, bob);
        assert_eq!(partitions[0].1.len(), 1);
        assert_eq!(partitions[0].1[0].object_id, bob_id);
        assert_eq!(partitions[1].0, cara);
        assert_eq!(partitions[1].1.len(), 1);
        assert_eq!(partitions[1].1[0].object_id, cara_id);
    }

    #[test]
    fn per_player_graveyard_choices_shuffle_only_each_players_chosen_set() {
        fn create_graveyard_card(game: &mut GameState, owner: PlayerId, raw_id: u32, name: &str) {
            let card = crate::card::CardBuilder::new(crate::ids::CardId::from_raw(raw_id), name)
                .card_types(vec![crate::types::CardType::Creature])
                .build();
            game.create_object_from_card(&card, owner, crate::zone::Zone::Graveyard);
        }

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        for (raw_id, name) in [
            (2001, "Alice One"),
            (2002, "Alice Two"),
            (2003, "Alice Three"),
            (2004, "Alice Four"),
        ] {
            create_graveyard_card(&mut game, alice, raw_id, name);
        }
        for (raw_id, name) in [(3001, "Bob One"), (3002, "Bob Two")] {
            create_graveyard_card(&mut game, bob, raw_id, name);
        }

        let chosen_tag = crate::tag::TagKey::from("__each_graveyard_chosen");
        let mut graveyard_filter = crate::filter::ObjectFilter::default();
        graveyard_filter.zone = Some(crate::zone::Zone::Graveyard);
        graveyard_filter.owner = Some(PlayerFilter::IteratedPlayer);
        let choose = crate::effects::ChooseObjectsEffect::new(
            graveyard_filter,
            crate::effect::ChoiceCount::exactly(3),
            PlayerFilter::You,
            chosen_tag.clone(),
        )
        .in_zone(crate::zone::Zone::Graveyard);
        let shuffle = crate::effects::ShuffleObjectsIntoLibraryEffect::new(
            crate::target::ChooseSpec::Tagged(chosen_tag.clone()),
            PlayerFilter::OwnerOf(crate::target::ObjectRef::Tagged(chosen_tag)),
        );
        let effect = ForPlayersEffect::new(
            PlayerFilter::Any,
            vec![Effect::new(choose), Effect::new(shuffle)],
        );

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);
        let outcome = effect
            .execute(&mut game, &mut ctx)
            .expect("each graveyard choice and owner shuffle should resolve");

        assert_eq!(
            game.player(alice).expect("Alice").graveyard.len(),
            1,
            "exactly three of Alice's four cards should move"
        );
        assert_eq!(
            game.player(bob).expect("Bob").graveyard.len(),
            0,
            "an undersized graveyard should contribute every available card"
        );
        assert_eq!(game.player(alice).expect("Alice").library.len(), 3);
        assert_eq!(game.player(bob).expect("Bob").library.len(), 2);

        let shuffled_players = outcome
            .events
            .iter()
            .filter_map(|event| {
                event
                    .downcast::<crate::events::ShuffleLibraryEvent>()
                    .map(|shuffle| shuffle.player)
            })
            .collect::<Vec<_>>();
        assert_eq!(shuffled_players.len(), 2);
        assert!(shuffled_players.contains(&alice));
        assert!(shuffled_players.contains(&bob));
    }

    #[test]
    fn per_player_choice_tags_survive_into_deferred_zone_move_commits() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        for (owner, raw_id, name) in [
            (alice, 3501, "Alice Hand Choice"),
            (bob, 3502, "Bob Hand Choice"),
        ] {
            let card = crate::card::CardBuilder::new(crate::ids::CardId::from_raw(raw_id), name)
                .card_types(vec![crate::types::CardType::Creature])
                .build();
            game.create_object_from_card(&card, owner, crate::zone::Zone::Hand);
        }

        let chosen_tag = crate::tag::TagKey::from("__each_player_hand_choice");
        let choose = crate::effects::ChooseObjectsEffect::new(
            crate::filter::ObjectFilter::default()
                .in_zone(crate::zone::Zone::Hand)
                .owned_by(PlayerFilter::IteratedPlayer),
            crate::effect::ChoiceCount::exactly(1),
            PlayerFilter::IteratedPlayer,
            chosen_tag.clone(),
        )
        .in_zone(crate::zone::Zone::Hand);
        let move_to_exile = crate::effects::MoveToZoneEffect::new(
            crate::target::ChooseSpec::Tagged(chosen_tag),
            crate::zone::Zone::Exile,
            false,
        );
        let effect = ForPlayersEffect::new(
            PlayerFilter::Any,
            vec![Effect::new(choose), Effect::new(move_to_exile)],
        );

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);
        effect
            .execute(&mut game, &mut ctx)
            .expect("each player's tagged choice should move during the deferred commit");

        for name in ["Alice Hand Choice", "Bob Hand Choice"] {
            assert!(
                game.objects_in_zone(crate::zone::Zone::Exile)
                    .into_iter()
                    .any(|object_id| game
                        .object(object_id)
                        .is_some_and(|object| object.name == name)),
                "{name} should remain bound to its player's deferred zone-move proposal"
            );
        }
    }

    #[test]
    fn tagged_results_from_an_earlier_action_stay_partitioned_for_later_player_actions() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        for (owner, raw_id, name) in [
            (alice, 3601, "Alice Returning Creature"),
            (bob, 3602, "Bob Returning Creature"),
        ] {
            let card = crate::card::CardBuilder::new(crate::ids::CardId::from_raw(raw_id), name)
                .card_types(vec![crate::types::CardType::Creature])
                .build();
            game.create_object_from_card(&card, owner, crate::zone::Zone::Graveyard);
        }

        let exiled_tag = crate::tag::TagKey::from("__each_player_exiled");
        let graveyard_creatures = crate::filter::ObjectFilter::creature()
            .in_zone(crate::zone::Zone::Graveyard)
            .owned_by(PlayerFilter::IteratedPlayer);
        let exile = Effect::exile_all(graveyard_creatures).tag(exiled_tag.clone());
        let return_own = Effect::put_onto_battlefield(
            crate::target::ChooseSpec::Tagged(exiled_tag),
            false,
            PlayerFilter::IteratedPlayer,
        );
        let sequence = Effect::new(crate::effects::SequenceEffect::new(vec![exile, return_own]));
        let effect = ForPlayersEffect::new(PlayerFilter::Any, vec![sequence]);

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);
        effect
            .execute(&mut game, &mut ctx)
            .expect("each player's tagged set should return under that player's control");

        for (name, expected_controller) in [
            ("Alice Returning Creature", alice),
            ("Bob Returning Creature", bob),
        ] {
            let object_id = game
                .objects_in_zone(crate::zone::Zone::Battlefield)
                .into_iter()
                .find(|object_id| {
                    game.object(*object_id)
                        .is_some_and(|object| object.name == name)
                })
                .unwrap_or_else(|| panic!("{name} should return"));
            assert_eq!(game.controller_of_id(object_id), Some(expected_controller));
        }
    }

    #[test]
    fn trailing_per_player_choices_still_accumulate_for_a_later_consumer() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        for (owner, raw_id, name) in [(alice, 4001, "Alice Choice"), (bob, 4002, "Bob Choice")] {
            let card = crate::card::CardBuilder::new(crate::ids::CardId::from_raw(raw_id), name)
                .card_types(vec![crate::types::CardType::Creature])
                .build();
            game.create_object_from_card(&card, owner, crate::zone::Zone::Graveyard);
        }

        let chosen_tag = crate::tag::TagKey::from("__later_each_player_choice");
        let choose = crate::effects::ChooseObjectsEffect::new(
            crate::filter::ObjectFilter::default()
                .in_zone(crate::zone::Zone::Graveyard)
                .owned_by(PlayerFilter::IteratedPlayer),
            crate::effect::ChoiceCount::exactly(1),
            PlayerFilter::You,
            chosen_tag.clone(),
        )
        .in_zone(crate::zone::Zone::Graveyard);
        let effect = ForPlayersEffect::new(PlayerFilter::Any, vec![Effect::new(choose)]);

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);
        effect
            .execute(&mut game, &mut ctx)
            .expect("each-player choices should resolve");

        let chosen = ctx
            .get_tagged_all(&chosen_tag)
            .expect("the accumulated choices should remain available");
        assert_eq!(chosen.len(), 2);
        assert!(chosen.iter().any(|snapshot| snapshot.owner == alice));
        assert!(chosen.iter().any(|snapshot| snapshot.owner == bob));
    }

    #[test]
    fn tagged_mutating_results_accumulate_across_players_for_a_plural_followup() {
        let mut game = GameState::new(
            vec!["Alice".to_string(), "Bob".to_string(), "Cara".to_string()],
            20,
        );
        let alice = PlayerId::from_index(0);
        let created_tag = crate::tag::TagKey::from("created_for_each_opponent");
        let create = Effect::new(crate::effects::CreateTokenEffect::new(
            crate::cards::tokens::treasure_token_definition(),
            2,
            PlayerFilter::You,
        ))
        .tag(created_tag.clone());
        let effect = ForPlayersEffect::new(PlayerFilter::Opponent, vec![create]);

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);
        effect
            .execute(&mut game, &mut ctx)
            .expect("per-opponent token creation should resolve");

        let created = ctx
            .get_tagged_all(&created_tag)
            .expect("the complete created result set should remain tagged");
        assert_eq!(
            created.len(),
            4,
            "two tokens for each of two opponents must feed the plural follow-up"
        );
    }
    #[test]
    fn sequential_results_accumulate_for_plural_followup() {
        let mut game = GameState::new(
            vec!["Alice".to_string(), "Bob".to_string(), "Cara".to_string()],
            20,
        );
        let alice = PlayerId::from_index(0);
        let created_tag = crate::tag::TagKey::from("created_for_each_opponent");
        let create = Effect::new(crate::effects::CreateTokenEffect::new(
            crate::cards::tokens::treasure_token_definition(),
            2,
            PlayerFilter::You,
        ))
        .tag(created_tag.clone());
        let mut effect = ForPlayersEffect::new(PlayerFilter::Opponent, vec![create]);
        effect.sequential = true;

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);
        effect
            .execute(&mut game, &mut ctx)
            .expect("per-opponent token creation should resolve");

        let created = ctx
            .get_tagged_all(&created_tag)
            .expect("the complete created result set should remain tagged");
        assert_eq!(
            created.len(),
            4,
            "two tokens for each of two opponents must feed the plural follow-up"
        );
    }
    #[test]
    fn later_player_token_payload_pause_or_error_restores_prior_units_and_players() {
        struct Answers {
            pending: bool,
            pause: bool,
        }
        impl crate::decision::DecisionMaker for Answers {
            fn decide_boolean(
                &mut self,
                _: &GameState,
                _: &crate::decisions::context::BooleanContext,
            ) -> bool {
                self.pending = self.pause;
                !self.pause
            }
            fn awaiting_choice(&self) -> bool {
                self.pending
            }
        }
        for sequential in [false, true] {
            for error in [false, true] {
                let mut game = setup_game();
                let alice = PlayerId::from_index(0);
                let bob = PlayerId::from_index(1);
                let token = crate::cards::CardDefinitionBuilder::new(
                    crate::ids::CardId::new(),
                    "Quantified token",
                )
                .token()
                .card_types(vec![crate::types::CardType::Creature])
                .power_toughness(crate::card::PowerToughness::fixed(1, 1))
                .build();
                let source = game.create_object_from_definition(
                    &token,
                    alice,
                    crate::zone::Zone::Battlefield,
                );
                let final_payload = if error {
                    Effect::gain_life(crate::effect::Value::X)
                } else {
                    Effect::may(vec![Effect::gain_life(1)])
                };
                let shield =
                    game.effect_store
                        .replacement_effects
                        .add_one_shot_effect(crate::replacement::ReplacementEffect::with_matcher(
                        source,
                        bob,
                        crate::events::tokens::matchers::WouldCreateTokensUnderControlMatcher::new(
                            PlayerFilter::You,
                        ),
                        crate::replacement::ReplacementAction::Instead(vec![
                            Effect::gain_life(3),
                            final_payload,
                        ]),
                    ));
                game.take_pending_trigger_events();
                let allocation_start = game.next_object_id_counter();
                let mut dm = Answers {
                    pending: false,
                    pause: true,
                };
                let mut ctx = ExecutionContext::new(source, alice, &mut dm);
                ctx.tag_player("retained", alice);
                let mut effect = ForPlayersEffect::new(
                    PlayerFilter::Any,
                    vec![
                        Effect::new(crate::effects::GainLifeEffect::new(
                            2,
                            crate::target::ChooseSpec::Player(PlayerFilter::IteratedPlayer),
                        )),
                        Effect::new(crate::effects::CreateTokenEffect::new(
                            token,
                            1,
                            PlayerFilter::IteratedPlayer,
                        )),
                    ],
                );
                effect.sequential = sequential;
                let outcome = effect.execute(&mut game, &mut ctx);
                if error {
                    assert!(matches!(outcome, Err(ExecutionError::UnresolvableValue(_))));
                } else {
                    assert!(ctx.decision_maker.awaiting_choice());
                    assert!(outcome.unwrap().events.is_empty());
                }
                assert_eq!(
                    game.player(alice).unwrap().life,
                    20,
                    "earlier action/player must be restored; sequential={sequential}, error={error}"
                );
                assert_eq!(game.player(bob).unwrap().life, 20);
                assert_eq!(game.battlefield.len(), 1);
                assert_eq!(game.next_object_id_counter(), allocation_start);
                assert!(
                    game.effect_store
                        .replacement_effects
                        .get_effect(shield)
                        .is_some()
                );
                assert!(game.take_pending_trigger_events().is_empty());
                assert!(ctx.effect_outcomes.is_empty());
                assert!(ctx.iteration.iterated_player.is_none());
                assert_eq!(
                    ctx.tagged_players
                        .get(&crate::tag::TagKey::from("retained"))
                        .unwrap(),
                    &vec![alice]
                );
                if !error {
                    drop(ctx);
                    dm.pause = false;
                    dm.pending = false;
                    let mut ctx = ExecutionContext::new(source, alice, &mut dm);
                    let outcome = effect.execute(&mut game, &mut ctx).unwrap();
                    assert!(!ctx.decision_maker.awaiting_choice());
                    assert_eq!(game.player(alice).unwrap().life, 22);
                    assert_eq!(game.player(bob).unwrap().life, 26);
                    assert_eq!(game.battlefield.len(), 2);
                    assert!(
                        game.effect_store
                            .replacement_effects
                            .get_effect(shield)
                            .is_none()
                    );
                    assert_eq!(
                        outcome
                            .events
                            .iter()
                            .filter(|event| event
                                .downcast::<crate::events::LifeGainEvent>()
                                .is_some())
                            .count(),
                        4
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod replacement_simultaneous_life_batch_contract_tests {
    #[test]
    fn each_player_life_loss_preserves_one_batch_across_distinct_observations() {
        use crate::effect::Effect;
        use crate::effects::{EffectExecutor, ExecutionContext, ForPlayersEffect};
        use crate::target::PlayerFilter;
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = crate::ids::PlayerId::from_index(0);
        let bob = crate::ids::PlayerId::from_index(1);
        let source = game.new_object_id();
        let proposal = game
            .provenance_graph_mut()
            .alloc_root_event(crate::events::EventKind::LifeLoss);
        let mut ctx = ExecutionContext::new_default(source, alice).with_provenance(proposal);
        let outcome = ForPlayersEffect::new(
            PlayerFilter::Any,
            vec![Effect::lose_life_player(3, PlayerFilter::IteratedPlayer)],
        )
        .execute(&mut game, &mut ctx)
        .expect("one simultaneous player action completes");
        assert_eq!(game.player(alice).unwrap().life, 17);
        assert_eq!(game.player(bob).unwrap().life, 17);
        assert_eq!(outcome.events.len(), 2);
        assert!(
            outcome
                .events
                .iter()
                .all(|event| event.kind() == crate::events::EventKind::LifeLoss)
        );
        let batch = outcome.events[0]
            .simultaneous_batch()
            .expect("simultaneous life losses retain their batch identity");
        assert!(
            outcome
                .events
                .iter()
                .all(|event| event.simultaneous_batch() == Some(batch))
        );
        assert_ne!(
            outcome.events[0].provenance(),
            outcome.events[1].provenance()
        );
        assert!(
            game.simultaneous_action_batch().is_none(),
            "the owner's batch scope closes"
        );
    }
}

#[cfg(test)]
mod readonly_player_result_tag_contract_tests {
    use super::*;
    use crate::types::CardType;
    use crate::zone::Zone;
    use crate::{CardDefinitionBuilder, CardId};

    fn reveal_collection(last_empty: bool) {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Carol".into()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let carol = PlayerId::from_index(2);
        let source = CardDefinitionBuilder::new(CardId::new(), "Reveal source")
            .card_types(vec![CardType::Artifact])
            .build();
        let source = game.create_object_from_definition(&source, alice, Zone::Battlefield);
        let top = CardDefinitionBuilder::new(CardId::new(), "Earlier matching card")
            .card_types(vec![CardType::Sorcery])
            .build();
        let first = game.create_object_from_definition(&top, bob, Zone::Library);
        let mut expected = vec![first];
        if !last_empty {
            let top = CardDefinitionBuilder::new(CardId::new(), "Later nonmatching card")
                .card_types(vec![CardType::Land])
                .build();
            expected.push(game.create_object_from_definition(&top, carol, Zone::Library));
        }
        let sentinel =
            crate::snapshot::ObjectSnapshot::from_object(game.object(source).unwrap(), &game);
        let mut ctx = ExecutionContext::new_default(source, alice);
        ctx.tag_object("outer", sentinel);
        let effect = ForPlayersEffect::new(
            PlayerFilter::Opponent,
            vec![Effect::new(crate::effects::RevealTopEffect::new(
                PlayerFilter::IteratedPlayer,
                Some("revealed".into()),
            ))],
        );
        let result = effect.execute(&mut game, &mut ctx).unwrap();
        assert_eq!(result.count_or_zero(), expected.len() as i64);
        assert_eq!(
            result.affected_object_memory().unwrap().len(),
            expected.len()
        );
        assert_eq!(
            result
                .events
                .iter()
                .filter(|event| event.kind() == crate::events::EventKind::CardRevealed)
                .count(),
            expected.len()
        );
        assert_eq!(game.player(bob).unwrap().library, vec![first]);
        assert_eq!(
            game.player(carol).unwrap().library.len(),
            usize::from(!last_empty)
        );
        assert_eq!(ctx.get_tagged_all("outer").unwrap()[0].object_id, source);
        let actual = ctx
            .get_tagged_all("revealed")
            .unwrap()
            .iter()
            .map(|s| s.object_id)
            .collect::<Vec<_>>();
        assert_eq!(
            actual, expected,
            "read-only result tags must agree with the complete multi-player outcome"
        );
        let conditional = crate::effects::ConditionalEffect::if_only(
            crate::effect::Condition::TaggedObjectMatches(
                "revealed".into(),
                crate::target::ObjectFilter::default().with_type(CardType::Sorcery),
            ),
            vec![Effect::gain_life(3)],
        );
        conditional.execute(&mut game, &mut ctx).unwrap();
        assert_eq!(
            game.player(alice).unwrap().life,
            23,
            "the earlier participant's matching card controls the follow-up"
        );
    }
    #[test]
    fn read_only_results_feed_only_their_players_later_conditional() {
        for later_empty in [false, true] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Carol".into()], 20);
            let alice = PlayerId::from_index(0);
            let bob = PlayerId::from_index(1);
            let carol = PlayerId::from_index(2);
            let source_def = CardDefinitionBuilder::new(CardId::new(), "Source")
                .card_types(vec![CardType::Artifact])
                .build();
            let source = game.create_object_from_definition(&source_def, alice, Zone::Battlefield);
            let first = CardDefinitionBuilder::new(CardId::new(), "Matching top")
                .card_types(vec![CardType::Sorcery])
                .build();
            let first = game.create_object_from_definition(&first, bob, Zone::Library);
            let mut expected = vec![first];
            if !later_empty {
                let second = CardDefinitionBuilder::new(CardId::new(), "Other top")
                    .card_types(vec![CardType::Land])
                    .build();
                expected.push(game.create_object_from_definition(&second, carol, Zone::Library));
            }
            let mut ctx = ExecutionContext::new_default(source, alice);
            let conditional = crate::effects::ConditionalEffect::if_only(
                crate::effect::Condition::TaggedObjectMatches(
                    "revealed".into(),
                    crate::target::ObjectFilter::default().with_type(CardType::Sorcery),
                ),
                vec![Effect::new(crate::effects::GainLifeEffect::new(
                    3,
                    crate::target::ChooseSpec::Player(PlayerFilter::IteratedPlayer),
                ))],
            );
            let effect = ForPlayersEffect::new(
                PlayerFilter::Opponent,
                vec![
                    Effect::new(crate::effects::RevealTopEffect::new(
                        PlayerFilter::IteratedPlayer,
                        Some("revealed".into()),
                    )),
                    Effect::new(conditional),
                ],
            );
            let outcome = effect.execute(&mut game, &mut ctx).unwrap();
            assert_eq!(game.player(alice).unwrap().life, 20);
            assert_eq!(
                game.player(bob).unwrap().life,
                23,
                "Bob's sorcery enables Bob's action"
            );
            assert_eq!(
                game.player(carol).unwrap().life,
                20,
                "another participant's card must not enable Carol's action; empty={later_empty}"
            );
            let tags = ctx
                .get_tagged_all("revealed")
                .unwrap()
                .iter()
                .map(|s| s.object_id)
                .collect::<Vec<_>>();
            assert_eq!(
                tags, expected,
                "the outer follow-up retains the complete collection"
            );
            assert_eq!(
                outcome
                    .events
                    .iter()
                    .filter(|event| event.kind() == crate::events::EventKind::CardRevealed)
                    .count(),
                expected.len()
            );
            assert_eq!(game.player(bob).unwrap().library, vec![first]);
            assert_eq!(
                game.player(carol).unwrap().library.len(),
                usize::from(!later_empty)
            );
        }
    }

    #[test]
    fn read_only_player_results_keep_both_participants_under_the_named_tag() {
        reveal_collection(false);
    }
    #[test]
    fn empty_later_library_does_not_erase_an_earlier_participants_result() {
        reveal_collection(true);
    }
}
