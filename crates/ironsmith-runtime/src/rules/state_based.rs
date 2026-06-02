//! State-based actions for MTG.
//!
//! State-based actions are checked whenever a player would receive priority.
//! They don't use the stack and happen simultaneously.

use crate::effects::permanents::attachment_can_attach_to_target;
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::object::AttachmentTarget;
use crate::object::CounterType;
use crate::snapshot::ObjectSnapshot;
use crate::static_abilities::StaticAbilityId;
use crate::targeting::has_protection_from_source;
use crate::triggers::TriggerQueue;
use crate::types::{CardType, Subtype, Supertype};
use crate::zone::Zone;
use std::collections::HashSet;

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

    /// A non-Aura attachment becomes unattached from an illegal or nonexistent target.
    AttachmentBecomesUnattached(ObjectId),

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

    /// A commander in graveyard or exile returns to the command zone.
    CommanderReturnsToCommandZone(ObjectId),

    /// A player controlling Start your engines gets speed 1.
    StartEngines { player: PlayerId },

    /// A soulbond pair no longer satisfies the pairing requirements.
    SoulbondUnpairs(ObjectId),
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

#[derive(Debug, Clone, Default)]
pub(crate) struct StateBasedActionContext {
    pending_chapter_ability_sources: HashSet<ObjectId>,
}

impl StateBasedActionContext {
    pub(crate) fn from_trigger_queue(trigger_queue: &TriggerQueue) -> Self {
        let pending_chapter_ability_sources = trigger_queue
            .entries
            .iter()
            .filter(|entry| entry.ability.trigger.saga_chapters().is_some())
            .map(|entry| entry.source)
            .collect();
        Self {
            pending_chapter_ability_sources,
        }
    }

    fn has_pending_chapter_ability_from(&self, source: ObjectId) -> bool {
        self.pending_chapter_ability_sources.contains(&source)
    }
}

/// Check state-based actions and return a list of actions that need to be performed.
///
/// This should be called whenever a player would receive priority.
/// State-based actions happen simultaneously.
pub fn check_state_based_actions(game: &GameState) -> Vec<StateBasedAction> {
    let view = crate::derived_view::DerivedGameView::from_effects(
        game,
        crate::static_ability_processor::get_all_continuous_effects(game),
    );
    check_state_based_actions_with_view(game, &view)
}

pub(crate) fn check_state_based_actions_with_effects(
    game: &GameState,
    all_effects: &[crate::continuous::ContinuousEffect],
) -> Vec<StateBasedAction> {
    let view = crate::derived_view::DerivedGameView::from_effects(game, all_effects.to_vec());
    check_state_based_actions_with_view(game, &view)
}

pub(crate) fn check_state_based_actions_with_view(
    game: &GameState,
    view: &crate::derived_view::DerivedGameView<'_>,
) -> Vec<StateBasedAction> {
    check_state_based_actions_with_context(game, view, &StateBasedActionContext::default())
}

pub(crate) fn check_state_based_actions_with_context(
    game: &GameState,
    view: &crate::derived_view::DerivedGameView<'_>,
    context: &StateBasedActionContext,
) -> Vec<StateBasedAction> {
    let mut actions = Vec::new();

    // Check player state-based actions
    check_player_sbas(game, &mut actions);
    check_commander_zone_sbas(game, &mut actions);
    check_start_engines_sbas_with_view(game, view, &mut actions);

    // Check permanent state-based actions
    check_permanent_sbas_with_view(game, view, context, &mut actions);

    // Check Role Aura uniqueness (one Role Aura per controller per permanent)
    check_role_sbas_with_view(game, view, &mut actions);

    // Check token/copy cleanup
    check_token_cleanup(game, &mut actions);

    // Check counter annihilation
    check_counter_annihilation(game, &mut actions);

    // Check soulbond pair validity
    check_soulbond_pair_sbas_with_view(game, view, &mut actions);

    // Check legend rule
    check_legend_rule_with_view(game, view, &mut actions);

    actions
}

fn check_soulbond_pair_sbas_with_view(
    game: &GameState,
    view: &crate::derived_view::DerivedGameView<'_>,
    actions: &mut Vec<StateBasedAction>,
) {
    let mut seen = HashSet::new();
    for (&left, &right) in &game.soulbond_pairs {
        if !seen.insert(left) {
            continue;
        }
        seen.insert(right);

        let valid = match (game.object(left), game.object(right)) {
            (Some(left_obj), Some(right_obj)) => {
                left_obj.zone == Zone::Battlefield
                    && right_obj.zone == Zone::Battlefield
                    && game.controller_of(left_obj) == game.controller_of(right_obj)
                    && view.object_has_card_type(left, CardType::Creature)
                    && view.object_has_card_type(right, CardType::Creature)
            }
            _ => false,
        };
        if !valid {
            actions.push(StateBasedAction::SoulbondUnpairs(left));
        }
    }
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

        if player.commander_damage.values().any(|&damage| damage >= 21) {
            actions.push(StateBasedAction::PlayerLoses {
                player: player.id,
                reason: LoseReason::CommanderDamage,
            });
        }

        // Note: "drew from empty library" is tracked separately when draw happens
    }
}

fn check_start_engines_sbas_with_view(
    game: &GameState,
    view: &crate::derived_view::DerivedGameView<'_>,
    actions: &mut Vec<StateBasedAction>,
) {
    for player in &game.players {
        if !player.is_in_game() || player.speed.is_some() {
            continue;
        }

        let controls_start_your_engines = game.battlefield.iter().copied().any(|obj_id| {
            game.current_controller(obj_id) == Some(player.id)
                && view.object_has_static_ability_id(obj_id, StaticAbilityId::StartYourEngines)
        });

        if controls_start_your_engines {
            actions.push(StateBasedAction::StartEngines { player: player.id });
        }
    }
}

fn check_commander_zone_sbas(game: &GameState, actions: &mut Vec<StateBasedAction>) {
    for player in &game.players {
        for &obj_id in &player.graveyard {
            if game.is_commander(obj_id) && !game.commander_command_zone_move_declined(obj_id) {
                actions.push(StateBasedAction::CommanderReturnsToCommandZone(obj_id));
            }
        }
    }

    for &obj_id in &game.exile {
        if game.is_commander(obj_id) && !game.commander_command_zone_move_declined(obj_id) {
            actions.push(StateBasedAction::CommanderReturnsToCommandZone(obj_id));
        }
    }
}

/// Check permanent-related state-based actions.
fn check_permanent_sbas_with_view(
    game: &GameState,
    view: &crate::derived_view::DerivedGameView<'_>,
    context: &StateBasedActionContext,
    actions: &mut Vec<StateBasedAction>,
) {
    for &obj_id in &game.battlefield {
        let Some(obj) = game.object(obj_id) else {
            continue;
        };
        let calculated_subtypes = view.calculated_subtypes(obj_id);

        // Creature with 0 or less toughness dies. This is not destruction,
        // so indestructible and regeneration do not stop it.
        // IMPORTANT: Use calculated_toughness to account for counters and effects!
        if view.object_has_card_type(obj_id, CardType::Creature) {
            let is_indestructible = !game.can_be_destroyed(obj_id)
                || view.object_has_static_ability_id(obj.id, StaticAbilityId::Indestructible)
                || game.object_has_static_ability_id(obj.id, StaticAbilityId::Indestructible);

            // Use calculated toughness to include -1/-1 counters, pump effects, etc.
            if let Some(toughness) = view.calculated_toughness(obj_id)
                && toughness <= 0
            {
                actions.push(StateBasedAction::ObjectDies(obj_id));
                continue;
            }

            // Creature with lethal damage dies (unless indestructible)
            let damage_marked = game.damage_on(obj_id);
            let lethal_damage_threshold = lethal_damage_threshold_for_creature(game, view, obj_id);
            if lethal_damage_threshold
                .is_some_and(|threshold| threshold > 0 && damage_marked >= threshold as u32)
                && !is_indestructible
            {
                actions.push(StateBasedAction::ObjectDies(obj_id));
                continue;
            }

            let toughness_for_deathtouch = view
                .calculated_toughness(obj_id)
                .or_else(|| obj.toughness());
            if toughness_for_deathtouch.is_some_and(|toughness| toughness > 0)
                && game.has_deathtouch_damage_since_sba(obj_id)
                && !is_indestructible
            {
                actions.push(StateBasedAction::ObjectDies(obj_id));
                continue;
            }
        }

        // Planeswalker with 0 or less loyalty
        if view.object_has_card_type(obj_id, CardType::Planeswalker) {
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

        // Aura not attached to anything or attached to an illegal object or player
        if view.object_has_card_type(obj_id, CardType::Enchantment)
            && calculated_subtypes.contains(&Subtype::Aura)
        {
            if obj.attached_to.is_none() {
                if obj.is_bestow_overlay_active() {
                    actions.push(StateBasedAction::BestowBecomesCreature(obj_id));
                } else {
                    actions.push(StateBasedAction::AuraFallsOff(obj_id));
                }
            }
        }

        if let Some(attached_target) = obj.attached_to {
            let is_aura = view.object_has_card_type(obj_id, CardType::Enchantment)
                && calculated_subtypes.contains(&Subtype::Aura);
            if is_aura {
                if !attachment_can_attach_to_target(game, obj_id, attached_target)
                    || matches!(attached_target, AttachmentTarget::Object(attached_id) if has_protection_from_source(game, attached_id, obj_id))
                {
                    if obj.is_bestow_overlay_active() {
                        actions.push(StateBasedAction::BestowBecomesCreature(obj_id));
                    } else {
                        actions.push(StateBasedAction::AuraFallsOff(obj_id));
                    }
                }
            } else if !attachment_can_attach_to_target(game, obj_id, attached_target) {
                actions.push(StateBasedAction::AttachmentBecomesUnattached(obj_id));
            }
        }

        if calculated_subtypes.contains(&Subtype::Saga)
            && let Some(max_chapter) =
                crate::game_loop::final_chapter_number_with_view(view, obj_id)
        {
            let lore_count = obj
                .counters
                .get(&crate::object::CounterType::Lore)
                .copied()
                .unwrap_or(0);
            let chapter_ability_pending_or_stacked = context
                .has_pending_chapter_ability_from(obj_id)
                || game
                    .stack
                    .iter()
                    .any(|entry| entry.chapter_ability_source == Some(obj_id));
            if lore_count >= max_chapter && !chapter_ability_pending_or_stacked {
                actions.push(StateBasedAction::SagaSacrifice(obj_id));
            }
        }
    }
}

fn lethal_damage_threshold_for_creature(
    game: &GameState,
    view: &crate::derived_view::DerivedGameView<'_>,
    creature_id: ObjectId,
) -> Option<i32> {
    let creature = game.object(creature_id)?;
    let creature_controller = game.controller_of(creature);
    let uses_power = game.battlefield.iter().any(|&source_id| {
        game.controller_of_id(source_id) == Some(creature_controller)
            && view.object_has_static_ability_id(
                source_id,
                StaticAbilityId::LethalDamageToCreaturesYouControlUsesPower,
            )
    });

    if uses_power {
        view.calculated_characteristics(creature_id)
            .and_then(|chars| chars.power)
            .or_else(|| creature.power())
            .map(|power| power.max(1))
    } else {
        view.calculated_toughness(creature_id)
            .or_else(|| creature.toughness())
    }
}

fn is_damage_based_creature_death_sba(
    game: &GameState,
    view: &crate::derived_view::DerivedGameView<'_>,
    creature_id: ObjectId,
) -> bool {
    if game.object(creature_id).is_none() {
        return false;
    }
    if !view.object_has_card_type(creature_id, CardType::Creature) {
        return false;
    }
    let toughness = view.calculated_toughness(creature_id).or_else(|| {
        game.object(creature_id)
            .and_then(|object| object.toughness())
    });
    if !toughness.is_some_and(|toughness| toughness > 0) {
        return false;
    }

    let Some(threshold) = lethal_damage_threshold_for_creature(game, view, creature_id) else {
        return false;
    };
    threshold > 0
        && (game.damage_on(creature_id) >= threshold as u32
            || game.has_deathtouch_damage_since_sba(creature_id))
}

fn check_role_sbas_with_view(
    game: &GameState,
    view: &crate::derived_view::DerivedGameView<'_>,
    actions: &mut Vec<StateBasedAction>,
) {
    use std::collections::HashMap;

    let mut roles_by_target_and_controller: HashMap<(ObjectId, PlayerId), Vec<ObjectId>> =
        HashMap::new();

    for &obj_id in &game.battlefield {
        let Some(obj) = game.object(obj_id) else {
            continue;
        };
        if !view.object_has_card_type(obj_id, CardType::Enchantment) {
            continue;
        }
        let calculated_subtypes = view.calculated_subtypes(obj_id);
        if !calculated_subtypes.contains(&Subtype::Aura)
            || !calculated_subtypes.contains(&Subtype::Role)
        {
            continue;
        }
        let Some(AttachmentTarget::Object(attached_id)) = obj.attached_to else {
            continue;
        };
        if game
            .object(attached_id)
            .is_none_or(|attached| attached.zone != Zone::Battlefield)
        {
            continue;
        }
        roles_by_target_and_controller
            .entry((
                attached_id,
                game.current_controller(obj_id)
                    .unwrap_or_else(|| game.controller_of(obj)),
            ))
            .or_default()
            .push(obj_id);
    }

    for (_group, mut roles) in roles_by_target_and_controller {
        if roles.len() < 2 {
            continue;
        }

        roles.sort_by_key(|role_id| {
            let timestamp = game
                .effect_store
                .continuous_effects
                .get_attachment_timestamp(*role_id)
                .or_else(|| {
                    game.effect_store
                        .continuous_effects
                        .get_entry_timestamp(*role_id)
                })
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
fn check_legend_rule_with_view(
    game: &GameState,
    view: &crate::derived_view::DerivedGameView<'_>,
    actions: &mut Vec<StateBasedAction>,
) {
    use std::collections::HashMap;

    if game.battlefield.iter().copied().any(|obj_id| {
        view.object_has_static_ability_id(obj_id, StaticAbilityId::LegendRuleDoesntApply)
    }) {
        return;
    }

    // Group legendary permanents by current controller and current name. Copy effects and
    // other continuous effects can make an object legendary or change its name.
    let mut legends: HashMap<(PlayerId, String), Vec<ObjectId>> = HashMap::new();

    for &obj_id in &game.battlefield {
        let Some(chars) = view.calculated_characteristics(obj_id) else {
            continue;
        };

        if chars.supertypes.contains(&Supertype::Legendary) {
            let key = (chars.controller, chars.name);
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
/// Per MTG Rule 704.8: "If a state-based action results in a permanent leaving the
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
/// Note: This version uses the CLI decision maker for any interactive choices
/// that arise while applying SBAs.
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
    decision_maker: &mut dyn crate::decision::DecisionMaker,
) -> bool {
    let all_effects = crate::static_ability_processor::get_all_continuous_effects(game);
    let actions = check_state_based_actions_with_effects(game, &all_effects);
    apply_state_based_actions_from_actions_with(game, actions, &all_effects, decision_maker)
}

pub(crate) fn apply_state_based_actions_from_actions_with(
    game: &mut GameState,
    actions: Vec<StateBasedAction>,
    all_effects: &[crate::continuous::ContinuousEffect],
    decision_maker: &mut dyn crate::decision::DecisionMaker,
) -> bool {
    if actions.is_empty() {
        return false;
    }

    // Per Rule 704.8, pre-capture snapshots for all dying creatures BEFORE
    // any state-based actions are applied. This ensures LKI is derived from
    // the game state before any SBAs were performed.
    let pre_captured_snapshots: std::collections::HashMap<ObjectId, ObjectSnapshot> = actions
        .iter()
        .filter_map(|action| {
            let obj_id = match action {
                StateBasedAction::ObjectDies(obj_id) | StateBasedAction::AuraFallsOff(obj_id) => {
                    *obj_id
                }
                _ => return None,
            };
            game.object(obj_id).map(|obj| {
                (
                    obj_id,
                    ObjectSnapshot::from_object_with_calculated_characteristics_and_effects(
                        obj,
                        game,
                        &all_effects,
                    ),
                )
            })
        })
        .collect();
    let damage_destroyed_object_ids: HashSet<ObjectId> = {
        let view = crate::derived_view::DerivedGameView::from_effects(game, all_effects.to_vec());
        actions
            .iter()
            .filter_map(|action| match action {
                StateBasedAction::ObjectDies(obj_id) => Some(*obj_id),
                _ => None,
            })
            .filter(|&obj_id| is_damage_based_creature_death_sba(game, &view, obj_id))
            .collect()
    };

    let mut any_applied = false;
    for action in actions {
        // Skip legend rule - it requires player choice
        if matches!(action, StateBasedAction::LegendRuleViolation { .. }) {
            continue;
        }
        apply_single_sba_with_snapshots(
            game,
            action,
            &pre_captured_snapshots,
            &damage_destroyed_object_ids,
            decision_maker,
        );
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
    let actions = check_state_based_actions(game);
    legend_rule_specs_from_actions(&actions)
}

pub(crate) fn legend_rule_specs_from_actions(
    actions: &[StateBasedAction],
) -> Vec<(
    crate::ids::PlayerId,
    crate::decisions::specs::LegendRuleSpec,
)> {
    use crate::decisions::specs::LegendRuleSpec;

    let mut specs = Vec::new();

    for action in actions {
        if let StateBasedAction::LegendRuleViolation {
            player,
            name,
            permanents,
        } = action
        {
            specs.push((
                *player,
                LegendRuleSpec::new(name.clone(), permanents.clone()),
            ));
        }
    }

    specs
}

/// Apply the legend rule with a specific choice of which permanent to keep.
///
/// All other legends with the same name controlled by the same player
/// are put into the graveyard.
pub fn apply_legend_rule_choice(game: &mut GameState, keep: ObjectId) {
    let view = crate::derived_view::DerivedGameView::new(game);

    // Find the current name and controller of the kept permanent.
    let (name, controller) = if let Some(chars) = view.calculated_characteristics(keep) {
        (chars.name, chars.controller)
    } else {
        return;
    };

    // Find all other current legends with the same name controlled by the same player.
    let to_remove: Vec<ObjectId> = game
        .battlefield
        .iter()
        .filter_map(|&id| {
            if id == keep {
                return None;
            }
            let chars = view.calculated_characteristics(id)?;
            if chars.controller == controller
                && chars.name == name
                && chars.supertypes.contains(&Supertype::Legendary)
            {
                Some(id)
            } else {
                None
            }
        })
        .collect();
    drop(view);

    // Move all others to graveyard
    for id in to_remove {
        game.move_object(
            id,
            Zone::Graveyard,
            crate::events::cause::EventCause::from_legend_rule(controller),
        );
    }
}

/// Apply a single state-based action with pre-captured snapshots.
///
/// Per Rule 704.8, creature death snapshots must be captured BEFORE any SBAs are applied.
/// The `pre_captured_snapshots` map contains these pre-captured snapshots.
fn apply_single_sba_with_snapshots(
    game: &mut GameState,
    action: StateBasedAction,
    pre_captured_snapshots: &std::collections::HashMap<ObjectId, ObjectSnapshot>,
    damage_destroyed_object_ids: &HashSet<ObjectId>,
    decision_maker: &mut dyn crate::decision::DecisionMaker,
) {
    match action {
        StateBasedAction::ObjectDies(obj_id) => {
            // Determine if this is from destruction (lethal damage or deathtouch)
            // or from 0 toughness.
            // Per MTG rules:
            // - Rule 704.5f: 0 toughness -> put into graveyard directly, regeneration can't help
            // - Rules 704.5g-h: lethal damage or deathtouch damage -> destroyed,
            //   regeneration CAN replace this
            let is_destroyed_by_damage_sba = damage_destroyed_object_ids.contains(&obj_id);

            if is_destroyed_by_damage_sba {
                // Damage-based SBAs are destruction, so process through the event
                // system to allow replacement effects like regeneration.
                use crate::events::processing::process_destroy_with_snapshot;
                let pre_snapshot = pre_captured_snapshots.get(&obj_id).cloned();
                let _ =
                    process_destroy_with_snapshot(game, obj_id, None, decision_maker, pre_snapshot);
            } else {
                // 0 toughness or object not found - goes directly to graveyard
                // Regeneration cannot replace this (Rule 704.5f), but other
                // replacement effects like Yawgmoth's Will can still apply
                use crate::events::processing::{
                    ZoneChangeOutcome, process_zone_change_with_snapshot,
                };
                let pre_snapshot = pre_captured_snapshots.get(&obj_id).cloned();
                let outcome = process_zone_change_with_snapshot(
                    game,
                    obj_id,
                    Zone::Battlefield,
                    Zone::Graveyard,
                    crate::events::cause::EventCause::from_sba(),
                    decision_maker,
                    pre_snapshot.clone(),
                );
                if let ZoneChangeOutcome::Proceed(final_zone) = outcome {
                    game.move_object_by_sba_with_snapshot(obj_id, final_zone, pre_snapshot);
                }
            }
        }

        StateBasedAction::PlaneswalkerDies(obj_id) => {
            // Process through replacement effects (e.g., Yawgmoth's Will)
            use crate::events::processing::{ZoneChangeOutcome, process_zone_change};
            let outcome = process_zone_change(
                game,
                obj_id,
                Zone::Battlefield,
                Zone::Graveyard,
                crate::events::cause::EventCause::from_sba(),
                decision_maker,
            );
            if let ZoneChangeOutcome::Proceed(final_zone) = outcome {
                game.move_object_by_sba(obj_id, final_zone);
            }
        }

        StateBasedAction::PlayerLoses { player, reason: _ } => {
            game.mark_player_lost(player);
        }

        StateBasedAction::StartEngines { player } => {
            game.start_engines(player);
        }

        StateBasedAction::SoulbondUnpairs(obj_id) => {
            game.clear_soulbond_pair(obj_id);
        }

        StateBasedAction::LegendRuleViolation {
            player,
            name: _,
            permanents,
        } => {
            // In a full implementation, the player would choose which to keep
            // For now, keep the first one, sacrifice the rest
            for &obj_id in permanents.iter().skip(1) {
                game.move_object(
                    obj_id,
                    Zone::Graveyard,
                    crate::events::cause::EventCause::from_legend_rule(player),
                );
            }
        }

        StateBasedAction::AuraFallsOff(obj_id) => {
            use crate::events::processing::{ZoneChangeOutcome, process_zone_change_with_snapshot};
            let pre_snapshot = pre_captured_snapshots.get(&obj_id).cloned();
            let outcome = process_zone_change_with_snapshot(
                game,
                obj_id,
                Zone::Battlefield,
                Zone::Graveyard,
                crate::events::cause::EventCause::from_sba(),
                decision_maker,
                pre_snapshot.clone(),
            );
            if let ZoneChangeOutcome::Proceed(final_zone) = outcome {
                game.move_object_by_sba_with_snapshot(obj_id, final_zone, pre_snapshot);
            }
        }

        StateBasedAction::AttachmentBecomesUnattached(obj_id) => {
            game.detach_object_from_current_target(obj_id);
        }

        StateBasedAction::BestowBecomesCreature(obj_id) => {
            game.detach_object_from_current_target(obj_id);
            if let Some(obj) = game.object_mut(obj_id) {
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
            game.move_object_by_sba(obj_id, Zone::Graveyard);
        }

        StateBasedAction::CommanderReturnsToCommandZone(obj_id) => {
            let Some(obj) = game.object(obj_id) else {
                return;
            };
            let owner = obj.owner;
            let name = obj.name.clone();
            let choice_ctx = crate::decisions::context::BooleanContext::new(
                owner,
                Some(obj_id),
                "move it to the command zone",
            )
            .with_source_name(name);

            if decision_maker.decide_boolean(game, &choice_ctx) {
                game.move_object_by_sba(obj_id, Zone::Command);
            } else {
                game.decline_commander_command_zone_move(obj_id);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::Ability;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::continuous::{ContinuousEffect, EffectTarget, Modification};
    use crate::decision::DecisionMaker;
    use crate::effect::Until;
    use crate::ids::CardId;
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::static_abilities::{Anthem, StaticAbility};
    use crate::types::CardType;

    #[derive(Default)]
    struct AlwaysYesDecisionMaker;

    impl DecisionMaker for AlwaysYesDecisionMaker {
        fn decide_boolean(
            &mut self,
            _game: &GameState,
            _ctx: &crate::decisions::context::BooleanContext,
        ) -> bool {
            true
        }
    }

    struct SequenceDecisionMaker {
        answers: std::collections::VecDeque<bool>,
        calls: usize,
    }

    impl SequenceDecisionMaker {
        fn new(answers: impl IntoIterator<Item = bool>) -> Self {
            Self {
                answers: answers.into_iter().collect(),
                calls: 0,
            }
        }
    }

    impl DecisionMaker for SequenceDecisionMaker {
        fn decide_boolean(
            &mut self,
            _game: &GameState,
            _ctx: &crate::decisions::context::BooleanContext,
        ) -> bool {
            self.calls += 1;
            self.answers.pop_front().unwrap_or(false)
        }
    }

    fn creature_card(card_id: u32, name: &str, power: i32, toughness: i32) -> crate::card::Card {
        CardBuilder::new(CardId::from_raw(card_id), name)
            .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Green]))
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(power, toughness))
            .build()
    }

    #[test]
    fn zero_toughness_sba_ignores_indestructible() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);

        let card = creature_card(399, "Indestructible Zero", 1, 0);
        let creature_id = game.create_object_from_card(&card, alice, Zone::Battlefield);
        game.object_mut(creature_id)
            .expect("indestructible zero should exist")
            .abilities
            .push(Ability::static_ability(StaticAbility::indestructible()));

        let actions = check_state_based_actions(&game);
        assert!(actions.contains(&StateBasedAction::ObjectDies(creature_id)));
        assert!(apply_state_based_actions(&mut game));

        assert!(
            game.current_object_id_after_zone_change(creature_id)
                .and_then(|id| game.object(id))
                .is_some_and(|object| object.zone == Zone::Graveyard),
            "0-toughness creature should go to the graveyard even with indestructible"
        );
    }

    #[test]
    fn simultaneous_sba_death_lki_uses_pre_sba_continuous_effects() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);

        let anthem_card = creature_card(400, "Doomed Marshal", 1, 1);
        let anthem_id = game.create_object_from_card(&anthem_card, alice, Zone::Battlefield);
        game.object_mut(anthem_id)
            .expect("anthem creature should exist")
            .abilities
            .push(Ability::static_ability(StaticAbility::new(
                Anthem::creatures_you_control(1, 1),
            )));

        let bear_card = creature_card(401, "Doomed Bear", 1, 1);
        let bear_id = game.create_object_from_card(&bear_card, alice, Zone::Battlefield);

        assert_eq!(game.calculated_toughness(anthem_id), Some(2));
        assert_eq!(game.calculated_toughness(bear_id), Some(2));
        game.mark_damage(anthem_id, 2);
        game.mark_damage(bear_id, 2);

        let actions = check_state_based_actions(&game);
        assert!(actions.contains(&StateBasedAction::ObjectDies(anthem_id)));
        assert!(actions.contains(&StateBasedAction::ObjectDies(bear_id)));

        let mut dm = AlwaysYesDecisionMaker;
        let all_effects = game.all_continuous_effects();
        assert!(apply_state_based_actions_from_actions_with(
            &mut game,
            actions,
            &all_effects,
            &mut dm,
        ));

        let pending = game.take_pending_trigger_events();
        let bear_death = pending
            .iter()
            .filter_map(|event| event.downcast::<crate::events::zones::ZoneChangeEvent>())
            .find(|event| event.objects.first().copied() == Some(bear_id))
            .expect("bear death should queue a zone-change event");
        let snapshot = bear_death
            .snapshot
            .as_ref()
            .expect("bear death should carry LKI");

        assert_eq!(
            snapshot.toughness,
            Some(2),
            "704.8 requires LKI from before any simultaneous SBAs, while the anthem still applied"
        );
    }

    #[test]
    fn commander_damage_loss_requires_twenty_one_from_one_commander() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 40);
        let bob = PlayerId::from_index(1);

        {
            let player = game.player_mut(bob).expect("bob should exist");
            player.record_commander_damage(ObjectId::from_raw(100), 11);
            player.record_commander_damage(ObjectId::from_raw(200), 10);
        }

        let actions = check_state_based_actions(&game);
        assert!(
            !actions.iter().any(|action| {
                matches!(
                    action,
                    StateBasedAction::PlayerLoses {
                        player,
                        reason: LoseReason::CommanderDamage,
                    } if *player == bob
                )
            }),
            "combined damage from different commanders should not be lethal"
        );

        game.player_mut(bob)
            .expect("bob should exist")
            .record_commander_damage(ObjectId::from_raw(100), 10);

        let actions = check_state_based_actions(&game);
        assert!(actions.iter().any(|action| {
            matches!(
                action,
                StateBasedAction::PlayerLoses {
                    player,
                    reason: LoseReason::CommanderDamage,
                } if *player == bob
            )
        }));
    }

    #[test]
    fn commander_in_graveyard_returns_to_command_zone() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 40);
        let alice = PlayerId::from_index(0);

        let commander = CardBuilder::new(CardId::from_raw(300), "Returned Commander")
            .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Green]))
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        let commander_id = game.create_object_from_card(&commander, alice, Zone::Graveyard);
        game.set_as_commander(commander_id, alice);

        let mut dm = AlwaysYesDecisionMaker;
        assert!(apply_state_based_actions_with(&mut game, &mut dm));

        let command_zone_ids = game.objects_in_zone(Zone::Command);
        assert_eq!(command_zone_ids.len(), 1);
        assert!(game.is_commander(command_zone_ids[0]));
        assert_eq!(
            game.object(command_zone_ids[0])
                .map(|obj| obj.name.as_str()),
            Some("Returned Commander")
        );
    }

    #[test]
    fn commander_decline_is_sticky_until_that_object_changes_zones() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 40);
        let alice = PlayerId::from_index(0);

        let commander = CardBuilder::new(CardId::from_raw(301), "Sticky Commander")
            .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Green]))
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        let commander_id = game.create_object_from_card(&commander, alice, Zone::Graveyard);
        game.set_as_commander(commander_id, alice);

        let mut dm = SequenceDecisionMaker::new([false, false]);
        assert!(apply_state_based_actions_with(&mut game, &mut dm));
        assert_eq!(dm.calls, 1, "first graveyard SBA should ask once");
        assert_eq!(game.objects_in_zone(Zone::Graveyard), vec![commander_id]);

        assert!(!apply_state_based_actions_with(&mut game, &mut dm));
        assert_eq!(
            dm.calls, 1,
            "declined commander should not reprompt while it stays put"
        );

        let exile_id = game
            .move_object_by_effect(commander_id, Zone::Exile)
            .expect("commander should move to exile");
        assert!(apply_state_based_actions_with(&mut game, &mut dm));
        assert_eq!(dm.calls, 2, "new object in exile should prompt again");
        assert_eq!(game.objects_in_zone(Zone::Exile), vec![exile_id]);
    }

    #[test]
    fn soulbond_pair_stays_valid_while_land_is_animated() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);

        let creature = CardBuilder::new(CardId::new(), "Soulbond Creature")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        let land = CardBuilder::new(CardId::new(), "Animated Land")
            .card_types(vec![CardType::Land])
            .build();
        let creature_id = game.create_object_from_card(&creature, alice, Zone::Battlefield);
        let land_id = game.create_object_from_card(&land, alice, Zone::Battlefield);

        game.effect_store.continuous_effects.add_effect(
            ContinuousEffect::new(
                land_id,
                alice,
                EffectTarget::Specific(land_id),
                Modification::AddCardTypes(vec![CardType::Creature]),
            )
            .until(Until::EndOfTurn),
        );
        game.set_soulbond_pair(creature_id, land_id);

        let actions = check_state_based_actions(&game);
        assert!(
            !actions
                .iter()
                .any(|action| matches!(action, StateBasedAction::SoulbondUnpairs(_))),
            "animated land should still satisfy soulbond creature requirement"
        );
    }

    #[test]
    fn soulbond_pair_unpairs_when_land_stops_being_creature() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);

        let creature = CardBuilder::new(CardId::new(), "Soulbond Creature")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        let land = CardBuilder::new(CardId::new(), "Former Creature Land")
            .card_types(vec![CardType::Land])
            .build();
        let creature_id = game.create_object_from_card(&creature, alice, Zone::Battlefield);
        let land_id = game.create_object_from_card(&land, alice, Zone::Battlefield);

        game.effect_store.continuous_effects.add_effect(
            ContinuousEffect::new(
                land_id,
                alice,
                EffectTarget::Specific(land_id),
                Modification::AddCardTypes(vec![CardType::Creature]),
            )
            .until(Until::EndOfTurn),
        );
        game.set_soulbond_pair(creature_id, land_id);
        game.effect_store.continuous_effects.cleanup_end_of_turn();

        let actions = check_state_based_actions(&game);
        assert!(
            actions.iter().any(|action| {
                matches!(
                    action,
                    StateBasedAction::SoulbondUnpairs(id) if *id == creature_id || *id == land_id
                )
            }),
            "noncreature land should no longer satisfy soulbond creature requirement"
        );

        let mut dm = AlwaysYesDecisionMaker;
        assert!(apply_state_based_actions_with(&mut game, &mut dm));
        assert_eq!(game.soulbond_partner(creature_id), None);
        assert_eq!(game.soulbond_partner(land_id), None);
    }
}
