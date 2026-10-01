//! ForPlayers effect implementation.

use crate::effect::{Effect, EffectOutcome};
use crate::effects::{EffectExecutor, SimultaneousEffectProposal};
use crate::effects::{ExecutionContext, ExecutionError, execute_effect};
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

/// True when this effect (or a nested child) selects objects through the
/// given context tag, i.e. it consumes what an earlier tagged effect binds.
fn effect_consumes_tag(effect: &Effect, tag: &crate::tag::TagKey) -> bool {
    fn spec_consumes(spec: &crate::target::ChooseSpec, tag: &crate::tag::TagKey) -> bool {
        match spec.base() {
            crate::target::ChooseSpec::Tagged(spec_tag) => spec_tag == tag,
            crate::target::ChooseSpec::Object(filter) => filter
                .tagged_constraints
                .iter()
                .any(|constraint| &constraint.tag == tag),
            _ => false,
        }
    }
    if effect
        .0
        .get_target_spec()
        .is_some_and(|spec| spec_consumes(spec, tag))
    {
        return true;
    }
    if effect
        .0
        .decision_related_object_specs()
        .iter()
        .any(|spec| spec_consumes(spec, tag))
    {
        return true;
    }
    let mut found = false;
    effect.0.visit_child_effects(&mut |child| {
        found |= effect_consumes_tag(child, tag);
    });
    found
}

/// The tag a wrapper effect binds for later effects, if any.
fn effect_bound_tag(effect: &Effect) -> Option<crate::tag::TagKey> {
    effect
        .downcast_ref::<crate::effects::TaggedEffect>()
        .map(|tagged| tagged.tag.clone())
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
            game.record_do_this_action(limit.source, limit.trigger_identity);
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

fn retain_optional_outcome(
    mut outcome: EffectOutcome,
    player_index: usize,
    path: &[usize],
    selection: bool,
    program: &OptionalActionProgram,
    optional_outcomes: &mut [Vec<Vec<EffectOutcome>>],
    outcomes_by_player: &mut [Vec<EffectOutcome>],
    outcomes: &mut Vec<EffectOutcome>,
    actual_events: &mut Vec<crate::events::RawEvent>,
    publish_events: bool,
) {
    if publish_events {
        actual_events.extend(outcome.events.iter().cloned());
    }
    if let Some(&group) = path.last() {
        if selection
            && program.is_optional[group]
            && program.offers[group]
                .effects
                .iter()
                .any(|effect| !super::may::is_object_selection(effect))
        {
            outcome.set_value(crate::effect::OutcomeValue::None);
        }
        optional_outcomes[group][player_index].push(outcome);
    } else {
        outcomes_by_player[player_index].push(outcome.clone());
        outcomes.push(outcome);
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
    fn execute_players(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
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
        players = order_selected_players_from(game, players, first_player);

        if players.is_empty() {
            return Ok(EffectOutcome::count(0));
        }

        let mut outcomes = Vec::new();
        let mut outcomes_by_player = vec![Vec::new(); players.len()];

        // The simultaneous protocol works on printed actions, so coordinated
        // sequence wrappers are unwrapped first. The sequential branch below
        // keeps `self.effects` as authored: nesting there is already executed in
        // order and carries no unit grouping.
        let optional_program =
            OptionalActionProgram::new(&self.effects, !ctx.target_assignments.is_empty());
        let simultaneous_effects = &optional_program.effects;
        let mut optional_acceptance =
            vec![vec![false; players.len()]; optional_program.offers.len()];
        let mut optional_outcomes =
            vec![vec![Vec::<EffectOutcome>::new(); players.len()]; optional_program.offers.len()];
        let mut program_groups: Vec<Vec<Option<CapturedProgramGroup>>> = vec![vec![None; players.len()]; optional_program.offers.len()];
        let mut optional_initialized = vec![false; optional_program.offers.len()];
        let mut optional_limits = vec![None; optional_program.offers.len()];
        let mut optional_limit_reached = vec![false; optional_program.offers.len()];
        let mut actual_events = Vec::new();

        if self.sequential || self.starting_with_controller || self.stop_after_first_happened {
            // An explicit starting player describes a sequential instruction
            // ("starting with ..."), as does stopping after the first player
            // whose action happened. Preserve player-major execution there.
            let incoming_tags = ctx.tagged_objects.clone();
            let mut completed_tags = std::collections::HashMap::new();
            for (player_index, &player_id) in players.iter().enumerate() {
                if self.sequential {
                    // Each body sees the outer scope, never another player's
                    // local result. Its complete results remain available to
                    // plural references after the loop finishes.
                    ctx.tagged_objects = incoming_tags.clone();
                }
                let mut stop = false;
                ctx.with_temp_iterated_player(Some(player_id), |ctx| {
                    for effect in &self.effects {
                        let outcome = execute_effect(game, effect, ctx)?;
                        outcomes_by_player[player_index].push(outcome.clone());
                        outcomes.push(outcome);
                        if ctx.decision_maker.awaiting_choice() {
                            return Ok(());
                        }
                    }
                    let iteration_outcome = EffectOutcome::aggregate_summing_counts(
                        outcomes_by_player[player_index].iter().cloned(),
                    );
                    stop = self.stop_after_first_happened && iteration_outcome.something_happened();
                    Ok::<(), ExecutionError>(())
                })?;
                if ctx.decision_maker.awaiting_choice() {
                    return Ok(EffectOutcome::count(0));
                }
                if self.sequential {
                    merge_tagged_object_sets(&mut completed_tags, &ctx.tagged_objects);
                }
                if stop {
                    break;
                }
            }
            if self.sequential {
                ctx.tagged_objects = completed_tags;
            }
        } else {
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
                return Ok(EffectOutcome::count(0));
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
            let mut units: Vec<Vec<usize>> = Vec::new();
            let mut current: Vec<usize> = Vec::new();
            for (effect_index, effect) in simultaneous_effects.iter().enumerate() {
                if let Some((_, begin)) = optional_program.markers[effect_index] {
                    if begin {
                        current.push(effect_index);
                    } else {
                        if !current.is_empty() {
                            units.push(std::mem::take(&mut current));
                        }
                        units.push(vec![effect_index]);
                    }
                    continue;
                }
                if current.last().is_some_and(|previous| {
                    optional_program.markers[*previous].is_none()
                        && optional_program.paths[*previous] != optional_program.paths[effect_index]
                }) {
                    units.push(std::mem::take(&mut current));
                }
                current.push(effect_index);
                if effect.0.is_read_only_simultaneous_player_action() {
                    continue;
                }
                // A mutating effect that binds a tag consumed by the next
                // effect is half of one printed action ("return it ... with a
                // counter on it") — keep the consumer in the same unit so the
                // per-player commit interleaving preserves the tag handoff.
                if let Some(tag) = effect_bound_tag(effect)
                    && simultaneous_effects
                        .get(effect_index + 1)
                        .is_some_and(|next| effect_consumes_tag(next, &tag))
                {
                    continue;
                }
                units.push(std::mem::take(&mut current));
            }
            if !current.is_empty() {
                units.push(current);
            }

            let mut tagged_objects_by_player = vec![
                std::collections::HashMap::<
                    crate::tag::TagKey,
                    Vec<crate::snapshot::ObjectSnapshot>,
                >::new();
                players.len()
            ];
            let mut loop_local_tags = std::collections::HashSet::<crate::tag::TagKey>::new();
            // A later printed action can reference the outcome of an earlier
            // action for this same player ("reveal that many"). Keep those
            // bindings per player just like the affected-object collections.
            let mut effect_outcomes_by_player = vec![ctx.effect_outcomes.clone(); players.len()];
            // Player bindings ("the chosen opponent", "that player") made by
            // one player's action belong to that player's later actions only.
            let incoming_tagged_players = ctx.tagged_players.clone();
            let mut tagged_players_by_player = vec![incoming_tagged_players.clone(); players.len()];

            for unit in units {
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
                        let captured = program_groups[group][player_index].as_ref();
                        let outcome = if captured.is_some_and(|group| !group.valid) {
                            EffectOutcome::target_invalid()
                        } else if optional_acceptance[group][player_index] {
                            let outcome = EffectOutcome::aggregate(std::mem::take(
                                &mut optional_outcomes[group][player_index],
                            ));
                            if optional_program.is_optional[group] {
                                outcome.with_execution_fact(crate::effect::ExecutionFact::Accepted)
                            } else {
                                outcome
                            }
                        } else {
                            EffectOutcome::declined()
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
                            false,
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
                    }
                    ctx.tagged_objects = completed_tags;
                    continue;
                }
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
                // Read-only choices bind tags in the shared execution context.
                // Each player's proposal must see the same pre-unit context,
                // not tags left behind by an earlier player's choice. The
                // proposal owns the frozen result it needs; restore the base
                // tags again before committing so commit-time result tags can
                // accumulate normally across players.
                let pre_unit_tagged_objects = ctx.tagged_objects.clone();
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
                let unit_players: Vec<PlayerId> = match unit_shared_order {
                    Some(acting) => acting.clone(),
                    None => players.clone(),
                }
                .into_iter()
                .filter(|player| {
                    let index = players
                        .iter()
                        .position(|candidate| candidate == player)
                        .expect("participant");
                    path.iter().all(|&group| optional_acceptance[group][index])
                })
                .collect();

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
                        && !effect.0.supports_simultaneous_player_action()
                        && !effect.0.is_read_only_simultaneous_player_action()
                });
                if unit_runs_player_by_player {
                    let pinned_lookback =
                        crate::effects::helpers::begin_simultaneous_zone_change_lookback(game);
                    let opened_batch = game.open_simultaneous_action();
                    let mut accumulated_unit_tags = pre_unit_tagged_objects.clone();
                    let mut unit_error = None;
                    let mut unit_waiting = false;
                    for &player_id in &unit_players {
                        let player_index = players
                            .iter()
                            .position(|candidate| *candidate == player_id)
                            .expect("acting player is in the iteration set");
                        ctx.tagged_objects = pre_unit_tagged_objects.clone();
                        apply_player_tagged_object_partition(
                            &mut ctx.tagged_objects,
                            &tagged_objects_by_player[player_index],
                            &loop_local_tags,
                        );
                        ctx.effect_outcomes = effect_outcomes_by_player[player_index].clone();
                        ctx.tagged_players = tagged_players_by_player[player_index].clone();
                        let pre_player_tagged_objects = ctx.tagged_objects.clone();
                        let result = ctx.with_temp_iterated_player(Some(player_id), |ctx| {
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
                                            execute_effect(game, effect, ctx)
                                        })
                                    },
                                )?;
                                retain_optional_outcome(
                                    outcome,
                                    player_index,
                                    path,
                                    super::may::is_object_selection(effect),
                                    &optional_program,
                                    &mut optional_outcomes,
                                    &mut outcomes_by_player,
                                    &mut outcomes,
                                    &mut actual_events,
                                    true,
                                );
                                if ctx.decision_maker.awaiting_choice() {
                                    break;
                                }
                            }
                            Ok::<(), ExecutionError>(())
                        });
                        effect_outcomes_by_player[player_index] = ctx.effect_outcomes.clone();
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
                        return Ok(EffectOutcome::count(0));
                    }
                    finish_optional_preparation(
                        game,
                        &unit,
                        &optional_program,
                        &optional_acceptance,
                        &mut optional_limits,
                    );
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
                for &player_id in &unit_players {
                    let player_index = players
                        .iter()
                        .position(|candidate| *candidate == player_id)
                        .expect("acting player is in the iteration set");
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
                                            execute_effect(game, effect, ctx)
                                        })
                                    },
                                )?;
                                retain_optional_outcome(
                                    outcome,
                                    player_index,
                                    path,
                                    super::may::is_object_selection(effect),
                                    &optional_program,
                                    &mut optional_outcomes,
                                    &mut outcomes_by_player,
                                    &mut outcomes,
                                    &mut actual_events,
                                    true,
                                );
                            } else if effect.0.supports_simultaneous_player_action() {
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
                                            effect.0.prepare_simultaneous_player_action(game, ctx)
                                        })
                                    },
                                )?;
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
                        return Ok(EffectOutcome::count(0));
                    }
                    effect_outcomes_by_player[player_index] = ctx.effect_outcomes.clone();
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
                    return Ok(EffectOutcome::count(0));
                }

                finish_optional_preparation(
                    game,
                    &unit,
                    &optional_program,
                    &optional_acceptance,
                    &mut optional_limits,
                );
                let game_checkpoint = game.clone();
                // CR 101.4 / 603.2c / 603.10a: the players' prepared actions
                // happen at the same time, as one event that looks back at
                // the same trigger sources.
                let pinned_lookback =
                    crate::effects::helpers::begin_simultaneous_zone_change_lookback(game);
                let opened_batch = game.open_simultaneous_action();
                let mut batch_outcomes = Vec::with_capacity(prepared.len());
                let mut accumulated_unit_tags = ctx.tagged_objects.clone();
                let mut active_commit_player = None;
                for (player_index, prepared_tagged_objects, proposal, optional, path) in prepared {
                    if active_commit_player != Some(player_index) {
                        if active_commit_player.is_some() {
                            merge_tagged_object_sets(
                                &mut accumulated_unit_tags,
                                &ctx.tagged_objects,
                            );
                        }
                        ctx.tagged_objects = prepared_tagged_objects.clone();
                        ctx.effect_outcomes = effect_outcomes_by_player[player_index].clone();
                        ctx.tagged_players = tagged_players_by_player[player_index].clone();
                        active_commit_player = Some(player_index);
                    }
                    let proposal_baseline = prepared_tagged_objects.clone();
                    match ctx.with_temp_iterated_player(Some(players[player_index]), |ctx| {
                        in_optional_action(ctx, optional, |ctx| {
                            let scopes = program_path_scopes(&path, &program_groups, player_index);
                            with_program_scope(ctx, &scopes, |ctx| proposal.commit(game, ctx))
                        })
                    }) {
                        Ok(outcome) => {
                            effect_outcomes_by_player[player_index] = ctx.effect_outcomes.clone();
                            tagged_players_by_player[player_index] = ctx.tagged_players.clone();
                            capture_player_tagged_object_deltas(
                                &proposal_baseline,
                                &ctx.tagged_objects,
                                &mut tagged_objects_by_player[player_index],
                                &mut loop_local_tags,
                            );
                            batch_outcomes.push((player_index, outcome));
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
                        return Ok(EffectOutcome::count(0));
                    }
                }
                game.close_simultaneous_action(opened_batch);
                crate::effects::helpers::end_simultaneous_zone_change_lookback(
                    game,
                    pinned_lookback,
                );
                merge_tagged_object_sets(&mut accumulated_unit_tags, &ctx.tagged_objects);
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
                for (player_index, outcome) in batch_outcomes {
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
                        true,
                    );
                }
            }
            ctx.tagged_players =
                merge_tagged_players_by_player(&incoming_tagged_players, &tagged_players_by_player);
        }

        let mut player_counts = Vec::new();
        let mut player_affected_memory = Vec::new();
        for (&player_id, player_outcomes) in players.iter().zip(&outcomes_by_player) {
            if player_outcomes.is_empty() {
                continue;
            }
            let iteration_outcome =
                EffectOutcome::aggregate_summing_counts(player_outcomes.iter().cloned());
            let count = if self.stop_after_first_happened {
                i32::from(iteration_outcome.something_happened())
            } else {
                iteration_outcome
                    .as_count()
                    .unwrap_or_else(|| i32::from(iteration_outcome.something_happened()))
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
        let mut outcome = if self.stop_after_first_happened {
            outcomes_by_player
                .iter()
                .filter(|iteration| !iteration.is_empty())
                .map(|iteration| EffectOutcome::aggregate_summing_counts(iteration.iter().cloned()))
                .find(EffectOutcome::something_happened)
                .unwrap_or_else(|| EffectOutcome::aggregate_summing_counts(outcomes))
        } else {
            EffectOutcome::aggregate_summing_counts(outcomes)
        };
        if !(self.sequential || self.starting_with_controller || self.stop_after_first_happened) {
            outcome.events = actual_events;
        }
        Ok(outcome
            .with_player_counts(player_counts)
            .with_player_affected_object_memory(player_affected_memory))
    }
}

impl EffectExecutor for ForPlayersEffect {
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
        if ctx.decision_maker.awaiting_choice() {
            return Ok(EffectOutcome::count(0));
        }
        let checkpoint = game.clone();
        let context_checkpoint = crate::effects::ExecutionContextCheckpoint::capture(ctx);
        let result = self.execute_players(game, ctx);
        let pending = ctx.decision_maker.awaiting_choice();
        if pending || result.is_err() {
            *game = checkpoint;
            context_checkpoint.restore(ctx);
        }
        if pending {
            return Ok(EffectOutcome::count(0));
        }
        result
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
        struct Answers {accepted: [bool; 2], rebound: crate::ids::ObjectId, calls: Vec<PlayerId>}
        impl crate::decision::DecisionMaker for Answers {
            fn decide_boolean(&mut self, game: &GameState, choice: &crate::decisions::context::BooleanContext) -> bool {
                assert!(game.players.iter().all(|player| player.life == 20));
                assert_eq!(choice.source, Some(self.rebound));self.calls.push(choice.player);
                self.accepted[game.players.iter().position(|p| p.id == choice.player).unwrap()]
            }
        }
        let mut checked = 0;
        for a in 0..4 {for b in 0..4 {for c in 0..4 {for d in 0..4 {
            let order = [a,b,c,d];if (0..4).any(|i| (i+1..4).any(|j| order[i] == order[j])) {continue;}
            for mask in 0..4 {
                let accepted = [mask & 1 != 0, mask & 2 != 0];
                let mut game = setup_game();let alice = PlayerId::from_index(0);let bob = PlayerId::from_index(1);
                game.turn.active_player = bob;game.turn_store.turn_order = vec![alice,bob];
                let card = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Scoped action source")
                    .card_types(vec![crate::types::CardType::Artifact]).build();
                let source = game.create_object_from_card(&card, alice, crate::zone::Zone::Battlefield);
                let rebound = game.create_object_from_card(&card, bob, crate::zone::Zone::Battlefield);
                let mut answers = Answers {accepted, rebound, calls: vec![]};
                let mut ctx = ExecutionContext::new(source, alice, &mut answers);
                ctx.store_outcome(crate::effect::EffectId(46), EffectOutcome::count(999));
                let gain = Effect::new(crate::effects::GainLifeEffect::new(
                    crate::effect::Value::LifeTotal(PlayerFilter::You), crate::target::ChooseSpec::Player(PlayerFilter::IteratedPlayer)));
                let mut action = Effect::may(vec![gain.clone(), gain]);
                for kind in order {
                    action = match kind {
                        0 => action.tag("scoped-phase-result"),
                        1 => Effect::with_id(46, action),
                        2 => Effect::new(crate::effects::ExecuteWithSourceEffect::new(crate::target::ChooseSpec::SpecificObject(rebound), action)),
                        3 => Effect::new(crate::effects::LocalRewriteEffect::new(action, vec![])),
                        _ => unreachable!(),
                    };
                }
                let result = ForPlayersEffect::new(PlayerFilter::Any, vec![action]).execute(&mut game, &mut ctx).unwrap();
                let second_amount = if accepted[0] {40} else {20};
                assert_eq!(game.player(alice).unwrap().life, if accepted[0] {80} else {20}, "order={order:?}, mask={mask}");
                assert_eq!(game.player(bob).unwrap().life, if accepted[1] {40 + second_amount} else {20}, "order={order:?}, mask={mask}");
                let expected = [20,second_amount].into_iter().flat_map(|amount| [bob,alice].into_iter()
                    .filter(move |player| accepted[usize::from(*player == bob)])
                    .map(move |player| (player, amount as u32, Some(rebound)))).collect::<Vec<_>>();
                let actual = result.events.iter().filter_map(|event| event.downcast::<crate::events::LifeGainEvent>())
                    .map(|event| (event.player,event.amount,event.source)).collect::<Vec<_>>();assert_eq!(actual,expected);
                let outcome = ctx.effect_outcomes.get(&crate::effect::EffectId(46)).unwrap();
                assert!(outcome.execution_facts.contains(&if accepted[0] {crate::effect::ExecutionFact::Accepted} else {crate::effect::ExecutionFact::Declined}));
                assert_eq!(ctx.source,source);assert!(ctx.source_snapshot.is_none());assert!(ctx.iteration.iterated_player.is_none());
                assert!(ctx.additional_replacement_effects().is_empty());drop(ctx);assert_eq!(answers.calls,vec![bob,alice]);checked += 1;
            }
        }}}}
        assert_eq!(checked,96);
    }

    #[test]
    fn scoped_optional_sacrifice_redirects_and_retains_actual_tagged_collection() {
        struct Answers {selected: Vec<crate::ids::ObjectId>, calls: Vec<(PlayerId, &'static str)>, rebound: crate::ids::ObjectId}
        impl crate::decision::DecisionMaker for Answers {
            fn decide_boolean(&mut self, game: &GameState, choice: &crate::decisions::context::BooleanContext) -> bool {
                assert_eq!(game.battlefield.len(),6);assert_eq!(choice.source,Some(self.rebound));self.calls.push((choice.player,"accept"));true
            }
            fn decide_objects(&mut self, game: &GameState, choice: &crate::decisions::context::SelectObjectsContext) -> Vec<crate::ids::ObjectId> {
                assert_eq!(game.battlefield.len(),6);assert_eq!(choice.candidates.len(),2);
                self.calls.push((choice.player,"sacrifice"));self.selected.push(choice.candidates[0].id);vec![choice.candidates[0].id]
            }
        }
        let mut game = setup_game();let alice = PlayerId::from_index(0);let bob = PlayerId::from_index(1);
        game.turn.active_player = bob;game.turn_store.turn_order = vec![alice,bob];
        let artifact = crate::card::CardBuilder::new(crate::ids::CardId::new(),"Scoped redirect source").card_types(vec![crate::types::CardType::Artifact]).build();
        let source = game.create_object_from_card(&artifact,alice,crate::zone::Zone::Battlefield);
        let rebound = game.create_object_from_card(&artifact,bob,crate::zone::Zone::Battlefield);
        let creature = crate::card::CardBuilder::new(crate::ids::CardId::new(),"Scoped sacrifice candidate").card_types(vec![crate::types::CardType::Creature]).build();
        for owner in [alice,bob] {for _ in 0..2 {game.create_object_from_card(&creature,owner,crate::zone::Zone::Battlefield);}}
        game.take_pending_trigger_events();
        let mut answers = Answers {selected: vec![], calls: vec![], rebound};let mut ctx = ExecutionContext::new(source,alice,&mut answers);
        ctx.tag_object("retained",crate::snapshot::ObjectSnapshot::from_object(game.object(source).unwrap(),&game));
        let replacement = ironsmith_core::RegisterZoneReplacementEffect::new(
            crate::target::ChooseSpec::All(crate::filter::ObjectFilter::creature()),Some(crate::zone::Zone::Battlefield),
            Some(crate::zone::Zone::Graveyard),crate::zone::Zone::Exile,crate::effects::ReplacementApplyMode::OneShot);
        let action = Effect::new(crate::effects::ExecuteWithSourceEffect::new(crate::target::ChooseSpec::SpecificObject(rebound),
            Effect::new(crate::effects::LocalRewriteEffect::new(Effect::may(vec![Effect::new(crate::effects::SacrificeEffect::player(
                crate::filter::ObjectFilter::creature(),1,PlayerFilter::IteratedPlayer))]),vec![replacement])))).tag("scoped-sacrificed");
        ForPlayersEffect::new(PlayerFilter::Any,vec![action]).execute(&mut game,&mut ctx).unwrap();
        assert_eq!(game.battlefield.len(),4);assert!(game.players.iter().all(|player| player.graveyard.is_empty()));assert_eq!(game.exile.len(),2);
        let tagged = ctx.get_tagged_all("scoped-sacrificed").unwrap().iter().map(|snapshot| snapshot.object_id).collect::<Vec<_>>();
        assert_eq!(tagged.len(),2);assert!(ctx.get_tagged_all("retained").unwrap().iter().any(|snapshot| snapshot.object_id == source));
        assert_eq!(ctx.source,source);assert!(ctx.source_snapshot.is_none());assert!(ctx.additional_replacement_effects().is_empty());
        let events = game.take_pending_trigger_events().into_iter().filter_map(|event| event.downcast::<crate::events::ZoneChangeEvent>().cloned()).collect::<Vec<_>>();
        assert_eq!(events.len(),2);assert!(events.iter().all(|event| event.from == crate::zone::Zone::Battlefield && event.to == crate::zone::Zone::Exile));
        drop(ctx);assert_eq!(answers.calls,vec![(bob,"accept"),(bob,"sacrifice"),(alice,"accept"),(alice,"sacrifice")]);
        assert!(answers.selected.iter().all(|object| tagged.contains(object)));
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
        let alice_memory = crate::effect::OutcomeObjectMemory {
            object_id: alice_card,
            stable_id: crate::ids::StableId::from(alice_card),
            name: "Alice Card".to_string(),
            controller: alice,
            owner: alice,
            zone: crate::zone::Zone::Library,
            power: None,
            toughness: None,
            mana_value: 1,
            card_types: vec![crate::types::CardType::Creature],
            colors: crate::color::ColorSet::COLORLESS,
            subtypes: Vec::new(),
            is_token: false,
        };
        let bob_memory = crate::effect::OutcomeObjectMemory {
            object_id: bob_card,
            stable_id: crate::ids::StableId::from(bob_card),
            name: "Bob Card".to_string(),
            controller: bob,
            owner: bob,
            zone: crate::zone::Zone::Library,
            power: None,
            toughness: None,
            mana_value: 2,
            card_types: vec![crate::types::CardType::Instant],
            colors: crate::color::ColorSet::COLORLESS,
            subtypes: Vec::new(),
            is_token: false,
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
        assert_eq!(result.count_or_zero(), expected.len() as i32);
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
