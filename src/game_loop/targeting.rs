// ============================================================================
// Target Extraction
// ============================================================================

/// Check if a ChooseSpec requires player selection.
/// Check if a target spec requires the player to select a target.
pub fn requires_target_selection(spec: &ChooseSpec) -> bool {
    match spec {
        // Target wrapper - check the inner spec
        ChooseSpec::Target(inner) | ChooseSpec::WithCount(inner, _) => {
            requires_target_selection(inner)
        }
        // These require target selection during casting
        ChooseSpec::AnyTarget
        | ChooseSpec::PlayerOrPlaneswalker(_)
        | ChooseSpec::Player(_)
        | ChooseSpec::Object(_) => true,
        ChooseSpec::AttackedPlayerOrPlaneswalker => false,
        // These don't require selection - they're resolved at execution time
        _ => false,
    }
}

/// Queue trigger matches for all triggered abilities that see this event.
fn queue_triggers_for_event(
    game: &GameState,
    trigger_queue: &mut TriggerQueue,
    event: TriggerEvent,
) {
    let triggers = check_triggers(game, &event);
    for trigger in triggers {
        trigger_queue.add(trigger);
    }
}

/// Ingest an event into trigger system with optional delayed-trigger checks.
fn queue_triggers_from_event(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    event: TriggerEvent,
    include_delayed: bool,
) {
    if let Some(damage_event) = event.downcast::<DamageEvent>()
        && let EventDamageTarget::Player(player_id) = damage_event.target
    {
        *game
            .damage_to_players_this_turn
            .entry(player_id)
            .or_insert(0) += damage_event.amount;
        if !damage_event.is_combat {
            *game
                .noncombat_damage_to_players_this_turn
                .entry(player_id)
                .or_insert(0) += damage_event.amount;
        }

        if game
            .object(damage_event.source)
            .map(|o| o.is_creature())
            .unwrap_or(false)
        {
            *game
                .creature_damage_to_players_this_turn
                .entry(player_id)
                .or_insert(0) += damage_event.amount;
        }
    }
    if let Some(life_gain_event) = event.downcast::<LifeGainEvent>() {
        *game
            .life_gained_this_turn
            .entry(life_gain_event.player)
            .or_insert(0) += life_gain_event.amount;
    }
    if let Some(life_loss_event) = event.downcast::<LifeLossEvent>() {
        *game
            .life_lost_this_turn
            .entry(life_loss_event.player)
            .or_insert(0) += life_loss_event.amount;
    }
    if let Some(keyword_action_event) = event.downcast::<KeywordActionEvent>()
        && keyword_action_event.action == KeywordActionKind::CommitCrime
    {
        let committed = keyword_action_event.amount.max(1);
        *game
            .crimes_committed_this_turn
            .entry(keyword_action_event.player)
            .or_insert(0) += committed;
    }
    if let Some(sacrifice_event) = event.downcast::<SacrificeEvent>() {
        let sacrificing_player = sacrifice_event
            .sacrificing_player
            .or_else(|| sacrifice_event.snapshot.as_ref().map(|snapshot| snapshot.controller))
            .or_else(|| game.object(sacrifice_event.permanent).map(|obj| obj.controller));
        let sacrificed_artifact = sacrifice_event
            .snapshot
            .as_ref()
            .is_some_and(|snapshot| snapshot.card_types.contains(&CardType::Artifact))
            || game
                .object(sacrifice_event.permanent)
                .is_some_and(|obj| obj.card_types.contains(&CardType::Artifact));
        if sacrificed_artifact && let Some(player) = sacrificing_player {
            *game.artifacts_sacrificed_this_turn.entry(player).or_insert(0) += 1;
        }
    }

    game.record_trigger_event_kind(event.kind());
    queue_triggers_for_event(game, trigger_queue, event.clone());

    if include_delayed {
        let delayed = crate::triggers::check_delayed_triggers(game, &event);
        for trigger in delayed {
            trigger_queue.add(trigger);
        }
    }
}

/// Queue trigger matches for each event in this list.
fn queue_triggers_for_events(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    events: Vec<TriggerEvent>,
) {
    for event in events {
        queue_triggers_from_event(game, trigger_queue, event, false);
    }
}

fn target_events_from_targets(
    targets: &[Target],
    source: ObjectId,
    source_controller: PlayerId,
    by_ability: bool,
) -> Vec<TriggerEvent> {
    targets
        .iter()
        .filter_map(|target| {
            let Target::Object(target_id) = target else {
                return None;
            };
            Some(TriggerEvent::new(BecomesTargetedEvent::new(
                *target_id,
                source,
                source_controller,
                by_ability,
            )))
        })
        .collect()
}

fn is_crime_target(game: &GameState, committer: PlayerId, target: &Target) -> bool {
    match target {
        Target::Player(player) => *player != committer,
        Target::Object(object_id) => {
            let Some(obj) = game.object(*object_id) else {
                return false;
            };
            if obj.zone == Zone::Graveyard {
                obj.owner != committer
            } else {
                obj.controller != committer
            }
        }
    }
}

fn targets_commit_crime(game: &GameState, committer: PlayerId, targets: &[Target]) -> bool {
    targets
        .iter()
        .any(|target| is_crime_target(game, committer, target))
}

fn queue_becomes_targeted_events(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    targets: &[Target],
    source: ObjectId,
    source_controller: PlayerId,
    by_ability: bool,
) {
    for event in target_events_from_targets(targets, source, source_controller, by_ability) {
        queue_triggers_from_event(game, trigger_queue, event, true);
    }

    if !targets.is_empty() && targets_commit_crime(game, source_controller, targets) {
        queue_triggers_from_event(
            game,
            trigger_queue,
            TriggerEvent::new(KeywordActionEvent::new(
                KeywordActionKind::CommitCrime,
                source_controller,
                source,
                1,
            )),
            true,
        );
    }
}

fn queue_ability_activated_event(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    source: ObjectId,
    activator: PlayerId,
    is_mana_ability: bool,
    source_stable_id: Option<StableId>,
) {
    let snapshot = if let Some(obj) = game.object(source) {
        Some(ObjectSnapshot::from_object(obj, game))
    } else if let Some(stable_id) = source_stable_id {
        game.find_object_by_stable_id(stable_id)
            .and_then(|id| game.object(id))
            .map(|obj| ObjectSnapshot::from_object(obj, game))
    } else {
        None
    };
    if is_mana_ability {
        let is_land_source = game
            .object(source)
            .map(|obj| obj.is_land())
            .or_else(|| snapshot.as_ref().map(|snap| snap.is_land()))
            .unwrap_or(false);
        if is_land_source {
            game.players_tapped_land_for_mana_this_turn
                .insert(activator);
        }
    }
    let event = TriggerEvent::new(
        AbilityActivatedEvent::new(source, activator, is_mana_ability).with_snapshot(snapshot),
    );
    queue_triggers_from_event(game, trigger_queue, event, true);
}

fn queue_mana_ability_event_for_action(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    action: &ManaPipPaymentAction,
    activator: PlayerId,
) {
    if let ManaPipPaymentAction::ActivateManaAbility { source_id, .. } = action {
        queue_ability_activated_event(game, trigger_queue, *source_id, activator, true, None);
    }
}

fn tap_permanent_with_trigger(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    permanent: ObjectId,
) {
    if game.object(permanent).is_some() && !game.is_tapped(permanent) {
        game.tap(permanent);
        queue_triggers_from_event(
            game,
            trigger_queue,
            TriggerEvent::new(crate::events::PermanentTappedEvent::new(permanent)),
            true,
        );
    }
}

fn keyword_action_from_alternative_effect(effect: AlternativePaymentEffect) -> KeywordActionKind {
    match effect {
        AlternativePaymentEffect::Convoke => KeywordActionKind::Convoke,
        AlternativePaymentEffect::Improvise => KeywordActionKind::Improvise,
    }
}

fn payment_contribution_tag(effect: AlternativePaymentEffect) -> &'static str {
    match effect {
        AlternativePaymentEffect::Convoke => "convoked_this_spell",
        AlternativePaymentEffect::Improvise => "improvised_this_spell",
    }
}

fn record_keyword_payment_contribution(
    contributions: &mut Vec<KeywordPaymentContribution>,
    action: &ManaPipPaymentAction,
) {
    let ManaPipPaymentAction::PayViaAlternative {
        permanent_id,
        effect,
    } = action
    else {
        return;
    };

    let contribution = KeywordPaymentContribution {
        permanent_id: *permanent_id,
        effect: *effect,
    };
    if !contributions.contains(&contribution) {
        contributions.push(contribution);
    }
}

fn apply_keyword_payment_tags_for_resolution(
    game: &GameState,
    entry: &StackEntry,
    ctx: &mut ExecutionContext,
) {
    for contribution in &entry.keyword_payment_contributions {
        if let Some(obj) = game.object(contribution.permanent_id) {
            let snapshot = ObjectSnapshot::from_object(obj, game);
            ctx.tag_object(payment_contribution_tag(contribution.effect), snapshot);
        }
    }

    for crew_id in &entry.crew_contributors {
        if let Some(obj) = game.object(*crew_id) {
            let snapshot = ObjectSnapshot::from_object(obj, game);
            ctx.tag_object("crewed_it_this_turn", snapshot);
        }
    }

    for saddle_id in &entry.saddle_contributors {
        if let Some(obj) = game.object(*saddle_id) {
            let snapshot = ObjectSnapshot::from_object(obj, game);
            ctx.tag_object("saddled_it_this_turn", snapshot);
        }
    }
}

/// Drain pending death and custom trigger events and enqueue all matches.
pub(crate) fn drain_pending_trigger_events(game: &mut GameState, trigger_queue: &mut TriggerQueue) {
    let pending_events = game.take_pending_trigger_events();
    for event in pending_events {
        queue_triggers_from_event(game, trigger_queue, event, true);
    }
}

/// Extracted target information from an effect.
pub struct ExtractedTarget<'a> {
    pub spec: &'a ChooseSpec,
    pub description: &'static str,
    pub min_targets: usize,
    pub max_targets: Option<usize>,
}

/// Extract a ChooseSpec from an Effect, if it has one that requires selection.
pub fn extract_target_spec(effect: &Effect) -> Option<ExtractedTarget<'_>> {
    // Use the EffectExecutor trait methods on the wrapped executor
    effect.0.get_target_spec().map(|spec| {
        // Check if the effect has a target count override
        let (min_targets, max_targets) = if let Some(target_count) = effect.0.get_target_count() {
            (target_count.min, target_count.max)
        } else {
            // Default: exactly 1 target
            (1, Some(1))
        };

        ExtractedTarget {
            spec,
            description: effect.0.target_description(),
            min_targets,
            max_targets,
        }
    })
}

fn resolve_modal_mode_counts(choose_mode: &crate::effects::ChooseModeEffect) -> (usize, usize) {
    let max_modes = match choose_mode.choose_count {
        crate::effect::Value::Fixed(n) => n.max(0) as usize,
        _ => 1,
    };
    let min_modes = match choose_mode.min_choose_count.as_ref() {
        Some(crate::effect::Value::Fixed(n)) => (*n).max(0) as usize,
        Some(_) => max_modes,
        None => max_modes,
    };
    (min_modes, max_modes)
}

fn effect_mode_has_legal_targets(
    game: &GameState,
    mode: &crate::effect::EffectMode,
    caster: PlayerId,
    source_id: Option<ObjectId>,
) -> bool {
    mode.effects
        .iter()
        .all(|effect| spell_effect_has_legal_targets(game, effect, caster, source_id, None))
}

fn choose_mode_has_legal_targets(
    game: &GameState,
    choose_mode: &crate::effects::ChooseModeEffect,
    caster: PlayerId,
    source_id: Option<ObjectId>,
    chosen_modes: Option<&[usize]>,
) -> bool {
    let (min_modes, max_modes) = resolve_modal_mode_counts(choose_mode);
    if min_modes > max_modes {
        return false;
    }
    if choose_mode.modes.is_empty() || max_modes == 0 {
        return min_modes == 0;
    }

    if let Some(chosen_modes) = chosen_modes {
        let mut selected_count = 0usize;
        let mut seen_modes = std::collections::HashSet::new();

        for mode_idx in chosen_modes {
            let Some(mode) = choose_mode.modes.get(*mode_idx) else {
                return false;
            };

            if !choose_mode.allow_repeated_modes && !seen_modes.insert(*mode_idx) {
                return false;
            }

            if !effect_mode_has_legal_targets(game, mode, caster, source_id) {
                return false;
            }
            selected_count += 1;
        }

        return selected_count >= min_modes && selected_count <= max_modes;
    }

    let legal_mode_count = choose_mode
        .modes
        .iter()
        .filter(|mode| effect_mode_has_legal_targets(game, mode, caster, source_id))
        .count();

    if min_modes == 0 {
        return true;
    }

    if choose_mode.allow_repeated_modes {
        legal_mode_count > 0
    } else {
        legal_mode_count >= min_modes
    }
}

fn spell_effect_has_legal_targets(
    game: &GameState,
    effect: &Effect,
    caster: PlayerId,
    source_id: Option<ObjectId>,
    chosen_modes: Option<&[usize]>,
) -> bool {
    let mut consumed_modal_selection = false;
    spell_effect_has_legal_targets_internal(
        game,
        effect,
        caster,
        source_id,
        chosen_modes,
        &mut consumed_modal_selection,
    )
}

fn spell_effect_has_legal_targets_internal(
    game: &GameState,
    effect: &Effect,
    caster: PlayerId,
    source_id: Option<ObjectId>,
    chosen_modes: Option<&[usize]>,
    consumed_modal_selection: &mut bool,
) -> bool {
    if let Some(choose_mode) = effect.downcast_ref::<crate::effects::ChooseModeEffect>() {
        let modes_for_this_choose_mode = if !*consumed_modal_selection {
            *consumed_modal_selection = true;
            chosen_modes
        } else {
            None
        };
        return choose_mode_has_legal_targets(
            game,
            choose_mode,
            caster,
            source_id,
            modes_for_this_choose_mode,
        );
    }

    if let Some(extracted) = extract_target_spec(effect)
        && requires_target_selection(extracted.spec)
    {
        // For "any number" effects, we can cast even with no legal targets.
        if extracted.min_targets == 0 {
            return true;
        }
        let legal_targets = compute_legal_targets(game, extracted.spec, caster, source_id);
        return legal_targets.len() >= extracted.min_targets;
    }

    true
}

fn extract_target_requirements_from_effect_internal(
    game: &GameState,
    effect: &Effect,
    caster: PlayerId,
    source_id: Option<ObjectId>,
    chosen_modes: Option<&[usize]>,
    consumed_modal_selection: &mut bool,
    requirements: &mut Vec<TargetRequirement>,
) {
    if let Some(choose_mode) = effect.downcast_ref::<crate::effects::ChooseModeEffect>() {
        let modes_for_this_choose_mode = if !*consumed_modal_selection {
            *consumed_modal_selection = true;
            chosen_modes
        } else {
            None
        };
        if let Some(chosen_modes) = modes_for_this_choose_mode {
            for mode_idx in chosen_modes {
                if let Some(mode) = choose_mode.modes.get(*mode_idx) {
                    for inner in &mode.effects {
                        extract_target_requirements_from_effect_internal(
                            game,
                            inner,
                            caster,
                            source_id,
                            None,
                            consumed_modal_selection,
                            requirements,
                        );
                    }
                }
            }
        }
        return;
    }

    if let Some(extracted) = extract_target_spec(effect)
        && requires_target_selection(extracted.spec)
    {
        let legal_targets = compute_legal_targets(game, extracted.spec, caster, source_id);
        // For "any number" effects (min_targets == 0), we can cast even with no legal targets.
        // For required targets (min_targets > 0), we need at least min_targets legal targets.
        let has_enough_targets = extracted.min_targets == 0 || legal_targets.len() >= extracted.min_targets;
        if has_enough_targets {
            requirements.push(TargetRequirement {
                spec: extracted.spec.clone(),
                legal_targets,
                description: extracted.description.to_string(),
                min_targets: extracted.min_targets,
                max_targets: extracted.max_targets,
            });
        }
    }
}

/// Extract target requirements from a list of effects with optional mode choices.
fn extract_target_requirements_with_modes(
    game: &GameState,
    effects: &[Effect],
    caster: PlayerId,
    source_id: Option<ObjectId>,
    chosen_modes: Option<&[usize]>,
) -> Vec<TargetRequirement> {
    let mut requirements = Vec::new();
    let mut consumed_modal_selection = false;

    for effect in effects {
        extract_target_requirements_from_effect_internal(
            game,
            effect,
            caster,
            source_id,
            chosen_modes,
            &mut consumed_modal_selection,
            &mut requirements,
        );
    }

    requirements
}

/// Extract target requirements from a list of effects.
fn extract_target_requirements(
    game: &GameState,
    effects: &[Effect],
    caster: PlayerId,
    source_id: Option<ObjectId>,
) -> Vec<TargetRequirement> {
    extract_target_requirements_with_modes(game, effects, caster, source_id, None)
}

pub(crate) fn spell_has_legal_targets_with_modes(
    game: &GameState,
    effects: &[Effect],
    caster: PlayerId,
    source_id: Option<ObjectId>,
    chosen_modes: Option<&[usize]>,
) -> bool {
    let mut consumed_modal_selection = false;
    for effect in effects {
        if !spell_effect_has_legal_targets_internal(
            game,
            effect,
            caster,
            source_id,
            chosen_modes,
            &mut consumed_modal_selection,
        ) {
            return false;
        }
    }
    true
}

/// Check if a spell has all required legal targets.
/// Returns true if all targeting requirements have enough legal targets,
/// or if the spell has no targeting requirements.
/// For "any number" effects (min_targets == 0), no legal targets are required.
pub fn spell_has_legal_targets(
    game: &GameState,
    effects: &[Effect],
    caster: PlayerId,
    source_id: Option<ObjectId>,
) -> bool {
    spell_has_legal_targets_with_modes(game, effects, caster, source_id, None)
}

/// Compute legal targets for a given ChooseSpec.
///
/// The `caster` parameter is used for resolving "you control" and similar filters.
/// The `source_id` is used for "other" filters (exclude the source itself).
pub fn compute_legal_targets(
    game: &GameState,
    spec: &ChooseSpec,
    caster: PlayerId,
    source_id: Option<ObjectId>,
) -> Vec<Target> {
    crate::targeting::compute_legal_targets(game, spec, caster, source_id)
}

/// Compute legal targets for a given ChooseSpec with additional tagged-object context.
///
/// This is used for cases where a target filter references tagged constraints like
/// "that crewed it this turn" or "that saddled it this turn" during target selection.
pub fn compute_legal_targets_with_tagged_objects(
    game: &GameState,
    spec: &ChooseSpec,
    caster: PlayerId,
    source_id: Option<ObjectId>,
    tagged_objects: Option<
        &std::collections::HashMap<crate::tag::TagKey, Vec<crate::snapshot::ObjectSnapshot>>,
    >,
) -> Vec<Target> {
    crate::targeting::compute_legal_targets_with_tagged_objects(
        game,
        spec,
        caster,
        source_id,
        tagged_objects,
    )
}

/// Check if a player matches a PlayerFilter with explicit combat context.
pub fn player_matches_filter_with_combat(
    player_id: PlayerId,
    filter: &crate::target::PlayerFilter,
    game: &GameState,
    controller: PlayerId,
    combat: Option<&CombatState>,
) -> bool {
    use crate::combat_state::{get_attacking_player, is_defending_player};
    use crate::target::PlayerFilter;

    match filter {
        PlayerFilter::Any => true,
        PlayerFilter::You => player_id == controller,
        PlayerFilter::NotYou => player_id != controller,
        PlayerFilter::Opponent => player_id != controller,
        PlayerFilter::Active => game.turn.active_player == player_id,
        PlayerFilter::Teammate => false, // In 2-player games, no teammates
        PlayerFilter::Defending => combat
            .map(|c| is_defending_player(c, player_id))
            .unwrap_or(false),
        PlayerFilter::Attacking => combat
            .map(|c| get_attacking_player(c, game) == Some(player_id))
            .unwrap_or(false),
        PlayerFilter::DamagedPlayer => false,
        PlayerFilter::Specific(id) => player_id == *id,
        PlayerFilter::IteratedPlayer => {
            // IteratedPlayer is resolved at runtime during iteration, not here
            false
        }
        PlayerFilter::Target(_) => {
            // Target filters are resolved through targeting, not direct matching
            true
        }
        PlayerFilter::Excluding { base, excluded } => {
            player_matches_filter_with_combat(player_id, base, game, controller, combat)
                && !player_matches_filter_with_combat(player_id, excluded, game, controller, combat)
        }
        PlayerFilter::ControllerOf(_) | PlayerFilter::OwnerOf(_) => {
            // These require object resolution, not applicable for simple player matching
            false
        }
    }
}

/// Check if an object has protection from a source that would prevent targeting.
///
/// Protection from a quality prevents:
/// - Damage from sources with that quality
/// - Enchanting/Equipping by permanents with that quality
/// - Blocking by creatures with that quality
/// - Targeting by spells/abilities from sources with that quality
fn has_protection_from_source(game: &GameState, target_id: ObjectId, source_id: ObjectId) -> bool {
    crate::targeting::has_protection_from_source(game, target_id, source_id)
}

/// Validate targets for a stack entry that's about to resolve.
///
/// Per MTG Rule 608.2b:
/// - If a spell/ability has targets and ALL targets are now illegal, it fizzles
/// - If SOME targets are still legal, the spell/ability resolves and does as much as possible
///
/// Returns (valid_targets, all_targets_invalid)
fn validate_stack_entry_targets(
    game: &GameState,
    entry: &StackEntry,
) -> (Vec<ResolvedTarget>, bool) {
    if entry.targets.is_empty() {
        return (Vec::new(), false);
    }

    let aura_target_spec = if let Some(obj) = game.object(entry.object_id)
        && obj.has_card_type(CardType::Enchantment)
        && obj.subtypes.contains(&crate::types::Subtype::Aura)
    {
        let effects = get_effects_for_stack_entry(game, entry, obj);
        effects
            .iter()
            .filter_map(extract_target_spec)
            .map(|extracted| extracted.spec.clone())
            .next()
    } else {
        None
    };

    let aura_ctx =
        crate::executor::ExecutionContext::new_default(entry.object_id, entry.controller);

    let mut valid_targets = Vec::new();
    let mut invalid_count = 0;

    for target in &entry.targets {
        let is_valid = match target {
            Target::Object(obj_id) => {
                // Check if object still exists
                if let Some(obj) = game.object(*obj_id) {
                    // Check if object is still on battlefield or stack (as appropriate)
                    let in_valid_zone = obj.zone == Zone::Battlefield || obj.zone == Zone::Stack;

                    // Check if object now has protection from the source
                    let has_protection = has_protection_from_source(game, *obj_id, entry.object_id);

                    // Check hexproof/shroud
                    let is_untargetable = game.is_untargetable(*obj_id);
                    let source_controller = game.object(entry.object_id).map(|s| s.controller);
                    let target_controller = obj.controller;
                    // Hexproof only prevents opponents from targeting
                    let blocked_by_hexproof = is_untargetable
                        && source_controller.is_some_and(|sc| sc != target_controller);

                    let mut valid = in_valid_zone && !has_protection && !blocked_by_hexproof;

                    if valid && let Some(ref spec) = aura_target_spec {
                        let resolved = crate::executor::ResolvedTarget::Object(*obj_id);
                        valid = crate::effects::helpers::validate_target(
                            game, &resolved, spec, &aura_ctx,
                        );
                    }

                    valid
                } else {
                    // Object no longer exists
                    false
                }
            }
            Target::Player(player_id) => {
                // Check if player is still in the game
                game.player(*player_id)
                    .map(|p| p.is_in_game())
                    .unwrap_or(false)
            }
        };

        if is_valid {
            valid_targets.push(match target {
                Target::Object(id) => ResolvedTarget::Object(*id),
                Target::Player(id) => ResolvedTarget::Player(*id),
            });
        } else {
            invalid_count += 1;
        }
    }

    let all_invalid = invalid_count == entry.targets.len();
    (valid_targets, all_invalid)
}
