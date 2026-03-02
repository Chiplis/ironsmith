//! State-based actions for MTG.
//!
//! State-based actions are checked whenever a player would receive priority.
//! They don't use the stack and happen simultaneously.

use crate::effects::helpers::validate_target;
use crate::executor::{ExecutionContext, ResolvedTarget};
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::object::CounterType;
use crate::snapshot::ObjectSnapshot;
use crate::static_abilities::StaticAbilityId;
use crate::targeting::has_protection_from_source;
use crate::types::{CardType, Subtype, Supertype};
use crate::zone::Zone;

/// A state-based action that needs to be performed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StateBasedAction {
    /// An object goes from battlefield to graveyard.
    ObjectDies(ObjectId),

    /// A planeswalker has 0 or less loyalty and is put into graveyard.
    PlaneswalkerDies(ObjectId),

    /// A player loses the game (life <= 0, poison >= 10, or tried to draw from empty library).
    PlayerLoses {
        player: PlayerId,
        reason: LoseReason,
    },

    /// Two or more legendary permanents with the same name are controlled by the same player.
    /// The player must choose which to keep; the others are put into graveyard.
    LegendRuleViolation {
        player: PlayerId,
        name: String,
        permanents: Vec<ObjectId>,
    },

    /// An Aura is not attached to anything or is attached to an illegal permanent.
    AuraFallsOff(ObjectId),

    /// A bestowed Aura is no longer legally attached and reverts to creature form.
    BestowBecomesCreature(ObjectId),

    /// An Equipment or Fortification is attached to an illegal permanent.
    EquipmentFallsOff(ObjectId),

    /// +1/+1 and -1/-1 counters on a permanent annihilate (remove pairs).
    CountersAnnihilate { permanent: ObjectId, count: u32 },

    // Note: Undying and Persist are handled as triggered abilities, not SBAs.
    // See triggers.rs for the implementation.
    /// A token not on the battlefield ceases to exist.
    TokenCeasesToExist(ObjectId),

    /// A copy of a spell not on the stack ceases to exist.
    CopyCeasesToExist(ObjectId),

    /// A saga's final chapter ability has resolved; sacrifice it.
    SagaSacrifice(ObjectId),
}

/// Reason why a player loses the game.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoseReason {
    /// Life total is 0 or less.
    ZeroLife,
    /// Has 10 or more poison counters.
    Poison,
    /// Attempted to draw from an empty library.
    DrewFromEmptyLibrary,
    /// 21 or more combat damage from a single commander (Commander format).
    CommanderDamage,
}

/// Check state-based actions and return a list of actions that need to be performed.
///
/// This should be called whenever a player would receive priority.
/// State-based actions happen simultaneously.
pub fn check_state_based_actions(game: &GameState) -> Vec<StateBasedAction> {
    let all_effects = game.all_continuous_effects();
    check_state_based_actions_with_effects(game, &all_effects)
}

fn check_state_based_actions_with_effects(
    game: &GameState,
    all_effects: &[crate::continuous::ContinuousEffect],
) -> Vec<StateBasedAction> {
    let mut actions = Vec::new();

    // Check player state-based actions
    check_player_sbas(game, &mut actions);

    // Check permanent state-based actions
    check_permanent_sbas(game, &all_effects, &mut actions);

    // Check Role Aura uniqueness (one Role Aura per controller per permanent)
    check_role_sbas(game, &all_effects, &mut actions);

    // Check token/copy cleanup
    check_token_cleanup(game, &mut actions);

    // Check counter annihilation
    check_counter_annihilation(game, &mut actions);

    // Check legend rule
    check_legend_rule(game, &mut actions);

    actions
}

/// Check player-related state-based actions.
fn check_player_sbas(game: &GameState, actions: &mut Vec<StateBasedAction>) {
    for player in &game.players {
        if !player.is_in_game() {
            continue;
        }

        // Check if player can actually lose the game (Platinum Angel effect)
        if !game.can_lose_game(player.id) {
            continue;
        }

        // Life total 0 or less
        if player.has_lethal_life() {
            actions.push(StateBasedAction::PlayerLoses {
                player: player.id,
                reason: LoseReason::ZeroLife,
            });
        }

        // 10 or more poison counters
        if player.has_lethal_poison() {
            actions.push(StateBasedAction::PlayerLoses {
                player: player.id,
                reason: LoseReason::Poison,
            });
        }

        // Note: "drew from empty library" is tracked separately when draw happens
    }
}

/// Check permanent-related state-based actions.
fn check_permanent_sbas(
    game: &GameState,
    all_effects: &[crate::continuous::ContinuousEffect],
    actions: &mut Vec<StateBasedAction>,
) {
    for &obj_id in &game.battlefield {
        let Some(obj) = game.object(obj_id) else {
            continue;
        };
        let calculated_subtypes = game.calculated_subtypes_with_effects(obj_id, all_effects);

        // Creature with 0 or less toughness dies (unless indestructible)
        // Check both:
        // 1. CantEffectTracker (catches indestructibility from external sources)
        // 2. Direct ability check (in case tracker hasn't been refreshed)
        // IMPORTANT: Use calculated_toughness to account for counters and effects!
        if game.object_has_card_type_with_effects(obj_id, CardType::Creature, all_effects) {
            let is_indestructible = !game.can_be_destroyed(obj_id)
                || game
                    .calculated_characteristics_with_effects(obj.id, all_effects)
                    .is_some_and(|c| {
                        c.static_abilities
                            .iter()
                            .any(|ability| ability.id() == StaticAbilityId::Indestructible)
                    })
                || game.object_has_static_ability_id(obj.id, StaticAbilityId::Indestructible);

            // Use calculated toughness to include -1/-1 counters, pump effects, etc.
            if let Some(toughness) = game.calculated_toughness_with_effects(obj_id, all_effects)
                && toughness <= 0
                && !is_indestructible
            {
                actions.push(StateBasedAction::ObjectDies(obj_id));
                continue;
            }

            // Creature with lethal damage dies (unless indestructible)
            let damage_marked = game.damage_on(obj_id);
            let toughness_for_lethal = game
                .calculated_toughness_with_effects(obj_id, all_effects)
                .or_else(|| obj.toughness());
            if toughness_for_lethal
                .is_some_and(|toughness| toughness > 0 && damage_marked >= toughness as u32)
                && !is_indestructible
            {
                actions.push(StateBasedAction::ObjectDies(obj_id));
                continue;
            }
        }

        // Planeswalker with 0 or less loyalty
        if game.object_has_card_type_with_effects(obj_id, CardType::Planeswalker, all_effects) {
            let loyalty_counters = obj
                .counters
                .get(&CounterType::Loyalty)
                .copied()
                .unwrap_or(0);
            if loyalty_counters == 0 {
                actions.push(StateBasedAction::PlaneswalkerDies(obj_id));
                continue;
            }
        }

        // Aura not attached to anything or attached to illegal permanent
        if game.object_has_card_type_with_effects(obj_id, CardType::Enchantment, all_effects)
            && calculated_subtypes.contains(&Subtype::Aura)
        {
            if obj.attached_to.is_none() {
                if obj.is_bestow_overlay_active() {
                    actions.push(StateBasedAction::BestowBecomesCreature(obj_id));
                } else {
                    actions.push(StateBasedAction::AuraFallsOff(obj_id));
                }
            } else if let Some(attached_id) = obj.attached_to {
                // Check if attached permanent still exists
                if game.object(attached_id).is_none() {
                    if obj.is_bestow_overlay_active() {
                        actions.push(StateBasedAction::BestowBecomesCreature(obj_id));
                    } else {
                        actions.push(StateBasedAction::AuraFallsOff(obj_id));
                    }
                } else if let Some(effects) = obj.spell_effect.as_ref()
                    && let Some(spec) = effects.iter().filter_map(|e| e.0.get_target_spec()).next()
                {
                    let ctx = ExecutionContext::new_default(obj_id, obj.controller);
                    let resolved = ResolvedTarget::Object(attached_id);
                    if !validate_target(game, &resolved, spec, &ctx)
                        || has_protection_from_source(game, attached_id, obj_id)
                    {
                        if obj.is_bestow_overlay_active() {
                            actions.push(StateBasedAction::BestowBecomesCreature(obj_id));
                        } else {
                            actions.push(StateBasedAction::AuraFallsOff(obj_id));
                        }
                    }
                }
            }
        }

        // Equipment not attached to a creature
        if game.object_has_card_type_with_effects(obj_id, CardType::Artifact, all_effects)
            && calculated_subtypes.contains(&Subtype::Equipment)
            && let Some(attached_id) = obj.attached_to
        {
            if game.object(attached_id).is_some() {
                if !game.object_has_card_type_with_effects(
                    attached_id,
                    CardType::Creature,
                    all_effects,
                ) {
                    actions.push(StateBasedAction::EquipmentFallsOff(obj_id));
                }
            } else {
                actions.push(StateBasedAction::EquipmentFallsOff(obj_id));
            }
        }

        // Saga with final chapter resolved AND still at max lore counters
        // (If lore counters are removed after final chapter triggers, the saga survives)
        if calculated_subtypes.contains(&Subtype::Saga)
            && game.is_saga_final_chapter_resolved(obj_id)
        {
            let max_chapter = obj.max_saga_chapter.unwrap_or(0);
            let lore_count = obj
                .counters
                .get(&crate::object::CounterType::Lore)
                .copied()
                .unwrap_or(0);
            if lore_count >= max_chapter {
                actions.push(StateBasedAction::SagaSacrifice(obj_id));
            }
        }
    }
}

/// Check Role Aura uniqueness.
///
/// Per MTG rule 704.5y: if a permanent has multiple Role Auras attached that are
/// controlled by the same player, the one with the most recent timestamp stays
/// and the others are put into their owners' graveyards.
fn check_role_sbas(
    game: &GameState,
    all_effects: &[crate::continuous::ContinuousEffect],
    actions: &mut Vec<StateBasedAction>,
) {
    use std::collections::HashMap;

    let mut roles_by_target_and_controller: HashMap<(ObjectId, PlayerId), Vec<ObjectId>> =
        HashMap::new();

    for &obj_id in &game.battlefield {
        let Some(obj) = game.object(obj_id) else {
            continue;
        };
        if !game.object_has_card_type_with_effects(obj_id, CardType::Enchantment, all_effects) {
            continue;
        }
        let calculated_subtypes = game.calculated_subtypes_with_effects(obj_id, all_effects);
        if !calculated_subtypes.contains(&Subtype::Aura)
            || !calculated_subtypes.contains(&Subtype::Role)
        {
            continue;
        }
        let Some(attached_id) = obj.attached_to else {
            continue;
        };
        if game
            .object(attached_id)
            .is_none_or(|attached| attached.zone != Zone::Battlefield)
        {
            continue;
        }
        roles_by_target_and_controller
            .entry((attached_id, obj.controller))
            .or_default()
            .push(obj_id);
    }

    for (_group, mut roles) in roles_by_target_and_controller {
        if roles.len() < 2 {
            continue;
        }

        roles.sort_by_key(|role_id| {
            let timestamp = game
                .continuous_effects
                .get_attachment_timestamp(*role_id)
                .or_else(|| game.continuous_effects.get_entry_timestamp(*role_id))
                .unwrap_or(0);
            (timestamp, role_id.0)
        });
        let keep_role = roles.last().copied();

        for role_id in roles {
            if Some(role_id) == keep_role {
                continue;
            }
            if !actions.iter().any(
                |action| matches!(action, StateBasedAction::AuraFallsOff(id) if *id == role_id),
            ) {
                actions.push(StateBasedAction::AuraFallsOff(role_id));
            }
        }
    }
}

/// Check for tokens not on battlefield and spell copies not on stack.
fn check_token_cleanup(game: &GameState, actions: &mut Vec<StateBasedAction>) {
    // Check all zones except battlefield for tokens
    for player in &game.players {
        for &obj_id in &player.graveyard {
            if let Some(obj) = game.object(obj_id)
                && obj.kind == crate::object::ObjectKind::Token
            {
                actions.push(StateBasedAction::TokenCeasesToExist(obj_id));
            }
        }
        for &obj_id in &player.hand {
            if let Some(obj) = game.object(obj_id)
                && obj.kind == crate::object::ObjectKind::Token
            {
                actions.push(StateBasedAction::TokenCeasesToExist(obj_id));
            }
        }
        for &obj_id in &player.library {
            if let Some(obj) = game.object(obj_id)
                && obj.kind == crate::object::ObjectKind::Token
            {
                actions.push(StateBasedAction::TokenCeasesToExist(obj_id));
            }
        }
    }

    for &obj_id in &game.exile {
        if let Some(obj) = game.object(obj_id)
            && obj.kind == crate::object::ObjectKind::Token
        {
            actions.push(StateBasedAction::TokenCeasesToExist(obj_id));
        }
    }
}

/// Check for +1/+1 and -1/-1 counter annihilation.
fn check_counter_annihilation(game: &GameState, actions: &mut Vec<StateBasedAction>) {
    for &obj_id in &game.battlefield {
        let Some(obj) = game.object(obj_id) else {
            continue;
        };

        let plus_counters = obj
            .counters
            .get(&CounterType::PlusOnePlusOne)
            .copied()
            .unwrap_or(0);
        let minus_counters = obj
            .counters
            .get(&CounterType::MinusOneMinusOne)
            .copied()
            .unwrap_or(0);

        if plus_counters > 0 && minus_counters > 0 {
            let count = plus_counters.min(minus_counters);
            actions.push(StateBasedAction::CountersAnnihilate {
                permanent: obj_id,
                count,
            });
        }
    }
}

/// Check the legend rule (no player can control two legendary permanents with the same name).
fn check_legend_rule(game: &GameState, actions: &mut Vec<StateBasedAction>) {
    use std::collections::HashMap;

    // Group legendary permanents by controller and name
    let mut legends: HashMap<(PlayerId, String), Vec<ObjectId>> = HashMap::new();

    for &obj_id in &game.battlefield {
        let Some(obj) = game.object(obj_id) else {
            continue;
        };

        if obj.has_supertype(Supertype::Legendary) {
            let key = (obj.controller, obj.name.clone());
            legends.entry(key).or_default().push(obj_id);
        }
    }

    // Find violations (more than one legendary with same name under same controller)
    for ((player, name), permanents) in legends {
        if permanents.len() > 1 {
            actions.push(StateBasedAction::LegendRuleViolation {
                player,
                name,
                permanents,
            });
        }
    }
}

/// Apply state-based actions to the game state.
///
/// Returns true if any state-based actions were applied.
/// Should be called repeatedly until it returns false.
///
/// Per MTG Rule 704.7: "If a state-based action results in a permanent leaving the
/// battlefield at the same time other state-based actions were performed, that
/// permanent's last known information is derived from the game state before any
/// of those state-based actions were performed."
///
/// To implement this correctly, we pre-capture snapshots for all dying creatures
/// BEFORE any of them are moved to the graveyard. This ensures that if creature A
/// gives +1/+1 to creature B, and both die simultaneously, B's snapshot correctly
/// includes A's buff.
///
/// Note: Legend rule violations are skipped by this function. Use
/// `get_legend_rule_decisions()` and `apply_legend_rule_choice()` to handle
/// those interactively.
///
/// Note: This version auto-passes all replacement effect choices. Use
/// `apply_state_based_actions_with` to provide a decision maker for interactive
/// replacement effect choices (e.g., when Yawgmoth's Will and another effect
/// both want to replace a zone change).
pub fn apply_state_based_actions(game: &mut GameState) -> bool {
    let mut auto_dm = crate::decision::CliDecisionMaker;
    apply_state_based_actions_with(game, &mut auto_dm)
}

/// Apply all pending state-based actions with a decision maker for replacement effects.
///
/// This version allows the decision maker to choose between multiple applicable
/// replacement effects during zone changes (e.g., choosing between Yawgmoth's Will
/// and another effect that wants to replace going to graveyard).
///
/// Note: Legend rule violations are skipped by this function. Use
/// `get_legend_rule_decisions()` and `apply_legend_rule_choice()` to handle
/// those interactively.
pub fn apply_state_based_actions_with(
    game: &mut GameState,
    decision_maker: &mut impl crate::decision::DecisionMaker,
) -> bool {
    let all_effects = game.all_continuous_effects();
    let actions = check_state_based_actions_with_effects(game, &all_effects);

    if actions.is_empty() {
        return false;
    }

    // Per Rule 704.7, pre-capture snapshots for all dying creatures BEFORE
    // any state-based actions are applied. This ensures LKI is derived from
    // the game state before any SBAs were performed.
    let pre_captured_snapshots: std::collections::HashMap<ObjectId, ObjectSnapshot> = actions
        .iter()
        .filter_map(|action| {
            if let StateBasedAction::ObjectDies(obj_id) = action {
                game.object(*obj_id).map(|obj| {
                    (
                        *obj_id,
                        ObjectSnapshot::from_object_with_calculated_characteristics_and_effects(
                            obj,
                            game,
                            &all_effects,
                        ),
                    )
                })
            } else {
                None
            }
        })
        .collect();

    let mut any_applied = false;
    for action in actions {
        // Skip legend rule - it requires player choice
        if matches!(action, StateBasedAction::LegendRuleViolation { .. }) {
            continue;
        }
        apply_single_sba_with_snapshots(game, action, &pre_captured_snapshots, decision_maker);
        any_applied = true;
    }

    any_applied
}

/// Get legend rule violations that require player decisions.
///
/// Returns a list of (player, spec) tuples for legend rule violations.
pub fn get_legend_rule_specs(
    game: &GameState,
) -> Vec<(
    crate::ids::PlayerId,
    crate::decisions::specs::LegendRuleSpec,
)> {
    use crate::decisions::specs::LegendRuleSpec;

    let actions = check_state_based_actions(game);
    let mut specs = Vec::new();

    for action in actions {
        if let StateBasedAction::LegendRuleViolation {
            player,
            name,
            permanents,
        } = action
        {
            specs.push((player, LegendRuleSpec::new(name, permanents)));
        }
    }

    specs
}

/// Apply the legend rule with a specific choice of which permanent to keep.
///
/// All other legends with the same name controlled by the same player
/// are put into the graveyard.
pub fn apply_legend_rule_choice(game: &mut GameState, keep: ObjectId) {
    // Find the name and controller of the kept permanent
    let (name, controller) = if let Some(obj) = game.object(keep) {
        (obj.name.clone(), obj.controller)
    } else {
        return;
    };

    // Find all other legends with the same name controlled by the same player
    let to_remove: Vec<ObjectId> = game
        .battlefield
        .iter()
        .filter_map(|&id| {
            if id == keep {
                return None;
            }
            let obj = game.object(id)?;
            if obj.controller == controller
                && obj.name == name
                && obj.has_supertype(Supertype::Legendary)
            {
                Some(id)
            } else {
                None
            }
        })
        .collect();

    // Move all others to graveyard
    for id in to_remove {
        game.move_object(id, Zone::Graveyard);
    }
}

/// Apply a single state-based action with pre-captured snapshots.
///
/// Per Rule 704.7, creature death snapshots must be captured BEFORE any SBAs are applied.
/// The `pre_captured_snapshots` map contains these pre-captured snapshots.
fn apply_single_sba_with_snapshots(
    game: &mut GameState,
    action: StateBasedAction,
    _pre_captured_snapshots: &std::collections::HashMap<ObjectId, ObjectSnapshot>,
    decision_maker: &mut impl crate::decision::DecisionMaker,
) {
    match action {
        StateBasedAction::ObjectDies(obj_id) => {
            // Determine if this is from lethal damage (destruction) or 0 toughness.
            // Per MTG rules:
            // - Rule 704.5f: 0 toughness -> put into graveyard directly, regeneration can't help
            // - Rule 704.5g: Lethal damage -> destroyed, regeneration CAN replace this
            let is_lethal_damage = game
                .object(obj_id)
                .map(|obj| {
                    let toughness = game.calculated_toughness(obj_id).unwrap_or(0);
                    let damage = game.damage_on(obj_id);
                    // It's lethal damage if toughness > 0 and damage >= toughness
                    toughness > 0 && obj.has_lethal_damage(damage)
                })
                .unwrap_or(false);

            if is_lethal_damage {
                // Lethal damage is destruction - process through event system
                // to allow replacement effects like regeneration
                use crate::event_processor::process_destroy;
                let _ = process_destroy(game, obj_id, None, decision_maker);
            } else {
                // 0 toughness or object not found - goes directly to graveyard
                // Regeneration cannot replace this (Rule 704.5f), but other
                // replacement effects like Yawgmoth's Will can still apply
                use crate::event_processor::{ZoneChangeOutcome, process_zone_change};
                let outcome = process_zone_change(
                    game,
                    obj_id,
                    Zone::Battlefield,
                    Zone::Graveyard,
                    decision_maker,
                );
                if let ZoneChangeOutcome::Proceed(final_zone) = outcome {
                    game.move_object(obj_id, final_zone);
                }
            }
        }

        StateBasedAction::PlaneswalkerDies(obj_id) => {
            // Process through replacement effects (e.g., Yawgmoth's Will)
            use crate::event_processor::{ZoneChangeOutcome, process_zone_change};
            let outcome = process_zone_change(
                game,
                obj_id,
                Zone::Battlefield,
                Zone::Graveyard,
                decision_maker,
            );
            if let ZoneChangeOutcome::Proceed(final_zone) = outcome {
                game.move_object(obj_id, final_zone);
            }
        }

        StateBasedAction::PlayerLoses { player, reason: _ } => {
            if let Some(p) = game.player_mut(player) {
                p.has_lost = true;
            }
        }

        StateBasedAction::LegendRuleViolation {
            player: _,
            name: _,
            permanents,
        } => {
            // In a full implementation, the player would choose which to keep
            // For now, keep the first one, sacrifice the rest
            for &obj_id in permanents.iter().skip(1) {
                game.move_object(obj_id, Zone::Graveyard);
            }
        }

        StateBasedAction::AuraFallsOff(obj_id) | StateBasedAction::EquipmentFallsOff(obj_id) => {
            game.move_object(obj_id, Zone::Graveyard);
        }

        StateBasedAction::BestowBecomesCreature(obj_id) => {
            let attached_to = game.object(obj_id).and_then(|obj| obj.attached_to);
            if let Some(parent_id) = attached_to
                && let Some(parent) = game.object_mut(parent_id)
            {
                parent.attachments.retain(|id| *id != obj_id);
            }
            if let Some(obj) = game.object_mut(obj_id) {
                obj.attached_to = None;
                obj.end_bestow_cast_overlay();
            }
        }

        StateBasedAction::CountersAnnihilate { permanent, count } => {
            if let Some(obj) = game.object_mut(permanent) {
                obj.remove_counters(CounterType::PlusOnePlusOne, count);
                obj.remove_counters(CounterType::MinusOneMinusOne, count);
            }
        }

        // Note: Undying/Persist are handled as triggered abilities,
        // not through SBAs. See triggers.rs.
        StateBasedAction::TokenCeasesToExist(token_id)
        | StateBasedAction::CopyCeasesToExist(token_id) => {
            // Remove from the game entirely (not to any zone)
            game.remove_object(token_id);
        }

        StateBasedAction::SagaSacrifice(obj_id) => {
            // Saga is sacrificed (put into graveyard) after final chapter resolves
            game.move_object(obj_id, Zone::Graveyard);
        }
    }
}
