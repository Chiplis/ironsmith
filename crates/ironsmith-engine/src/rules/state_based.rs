//! State-based actions for MTG.
//!
//! State-based actions are checked whenever a player would receive priority.
//! They don't use the stack and happen simultaneously.

use crate::effects::permanents::attachment_can_attach_to_target;
use crate::filter::ObjectFilterExt as _;
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
use std::collections::{HashMap, HashSet};

fn controlled_existing_attachment_is_preserved_by_protection_grant(
    game: &GameState,
    view: &crate::derived_view::DerivedGameView<'_>,
    protected: ObjectId,
    attachment: ObjectId,
) -> bool {
    let attachment_subtypes = view.calculated_subtypes(attachment);
    let attachment_is_aura_or_equipment = (view
        .object_has_card_type(attachment, CardType::Enchantment)
        && attachment_subtypes.contains(&Subtype::Aura))
        || (view.object_has_card_type(attachment, CardType::Artifact)
            && attachment_subtypes.contains(&Subtype::Equipment));
    if !attachment_is_aura_or_equipment {
        return false;
    }
    let Some(protected_object) = game.object(protected) else {
        return false;
    };

    protected_object
        .attachments
        .iter()
        .copied()
        .any(|grant_source| {
            if game.controller_of_id(grant_source) != game.controller_of_id(attachment) {
                return false;
            }
            let Some(source) = game.object(grant_source) else {
                return false;
            };
            source.abilities.iter().any(|ability| {
                let crate::ability::AbilityKind::Static(static_ability) = &ability.kind else {
                    return false;
                };
                let Some(model) = static_ability.compiled_model() else {
                    return false;
                };
                let ironsmith_core::StaticAbilityPayload::AttachedAbilityGrant(grant) =
                    &model.payload
                else {
                    return false;
                };
                if !grant.protection_does_not_remove_controlled_attachments {
                    return false;
                }
                let ironsmith_core::AbilityKind::Static(granted) = &grant.ability.kind else {
                    return false;
                };
                if !matches!(
                    &granted.payload,
                    ironsmith_core::StaticAbilityPayload::Protection(
                        ironsmith_core::ProtectionFrom::ChosenColor
                    )
                ) {
                    return false;
                }
                game.chosen_color(grant_source)
                    .is_some_and(|color| view.object_colors(attachment).contains(color))
            })
        })
}

/// A state-based action that needs to be performed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StateBasedAction {
    /// An object goes from battlefield to graveyard.
    ObjectDies(ObjectId),

    /// A planeswalker has 0 or less loyalty and is put into graveyard.
    PlaneswalkerDies(ObjectId),

    /// A battle has defense 0, or has no legal protector, and is put into its owner's graveyard.
    BattleDies(ObjectId),

    /// A battle's controller must choose a legal protector.
    BattleProtectorChoice(ObjectId),

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

    /// The world rule puts this already-determined simultaneous group into
    /// their owners' graveyards. Unlike the legend rule, no player choice is
    /// involved: the most recently acquired unique World supertype survives,
    /// and a tie for most recent removes every World permanent.
    WorldRuleViolation { permanents: Vec<ObjectId> },

    /// An Aura is not attached to anything or is attached to an illegal permanent.
    AuraFallsOff(ObjectId),

    /// A bestowed Aura is no longer legally attached and reverts to creature form.
    BestowBecomesCreature(ObjectId),

    /// A non-Aura attachment becomes unattached from an illegal or nonexistent target.
    AttachmentBecomesUnattached(ObjectId),

    /// +1/+1 and -1/-1 counters on a permanent annihilate (remove pairs).
    CountersAnnihilate { permanent: ObjectId, count: u32 },

    /// Remove counters above the smallest active static cap (CR 704.5r).
    CountersExceedMaximum {
        permanent: ObjectId,
        counter_type: CounterType,
        count: u32,
    },

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

    /// All existing sector designations end because no space sculptor remains.
    ClearSectorDesignations,

    /// Controllers choose sectors for all currently undesignated creatures.
    ///
    /// The vector is already in the two CR 704.5u choice partitions, with
    /// APNAP order inside each partition. Every choice is collected before any
    /// designation is committed.
    SectorDesignationChoices {
        source: ObjectId,
        creatures: Vec<(PlayerId, ObjectId)>,
    },

    /// A soulbond pair no longer satisfies the pairing requirements.
    SoulbondUnpairs(ObjectId),

    /// A face-up phenomenon's encounter trigger has left the stack, so the
    /// planar controller planeswalks (CR 704.6f / 312.7).
    PlaneswalkFromPhenomenon(ObjectId),

    /// A face-up nonongoing scheme has no triggered ability pending or on the
    /// stack, so its owner turns it face down on the bottom of their scheme deck.
    RecycleScheme(ObjectId),
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
    pending_battle_defeat_sources: HashSet<ObjectId>,
    pending_ability_sources: HashSet<ObjectId>,
}

impl StateBasedActionContext {
    pub(crate) fn from_trigger_queue(trigger_queue: &TriggerQueue) -> Self {
        let pending_chapter_ability_sources = trigger_queue
            .entries
            .iter()
            .filter(|entry| entry.ability.trigger.saga_chapters().is_some())
            .map(|entry| entry.source)
            .collect();
        let pending_battle_defeat_sources = trigger_queue
            .entries
            .iter()
            .filter(|entry| crate::triggers::check::is_intrinsic_siege_defeat_trigger(entry))
            .map(|entry| entry.source)
            .collect();
        let pending_ability_sources = trigger_queue
            .entries
            .iter()
            .map(|entry| entry.source)
            .collect();
        Self {
            pending_chapter_ability_sources,
            pending_battle_defeat_sources,
            pending_ability_sources,
        }
    }

    fn has_pending_chapter_ability_from(&self, source: ObjectId) -> bool {
        self.pending_chapter_ability_sources.contains(&source)
    }

    fn has_pending_battle_defeat_from(&self, source: ObjectId) -> bool {
        self.pending_battle_defeat_sources.contains(&source)
    }

    fn has_pending_ability_from(&self, source: ObjectId) -> bool {
        self.pending_ability_sources.contains(&source)
    }
}

/// Check state-based actions and return a list of actions that need to be performed.
///
/// This should be called whenever a player would receive priority.
/// State-based actions happen simultaneously.
pub fn check_state_based_actions(game: &GameState) -> Vec<StateBasedAction> {
    let view = crate::derived_view::DerivedGameView::new(game);
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
    game.count_sba_scan_objects(game.battlefield.len());
    view.prewarm_characteristics(&game.battlefield);
    let mut actions = Vec::new();

    // Check player state-based actions
    check_player_sbas(game, &mut actions);
    check_commander_zone_sbas(game, &mut actions);
    check_start_engines_sbas_with_view(game, view, &mut actions);
    check_phenomenon_sba(game, context, &mut actions);
    check_scheme_sba(game, context, &mut actions);

    // Check permanent state-based actions
    check_permanent_sbas_with_view(game, view, context, &mut actions);

    // Check Role Aura uniqueness (one Role Aura per controller per permanent)
    check_role_sbas_with_view(game, view, &mut actions);

    // Check token/copy cleanup
    check_token_cleanup(game, &mut actions);

    // Check counter annihilation
    check_counter_annihilation(game, &mut actions);
    check_counter_limits_with_view(game, view, &mut actions);

    // Check soulbond pair validity
    check_soulbond_pair_sbas_with_view(game, view, &mut actions);

    // Check legend rule
    check_legend_rule_with_view(game, view, &mut actions);

    // Check world rule
    check_world_rule_with_view(game, view, &mut actions);

    // Space sculptor designation assignment/expiry (CR 704.5u, 702.158b-c).
    check_space_sculptor_sbas_with_view(game, view, &mut actions);

    actions
}

fn check_phenomenon_sba(
    game: &GameState,
    context: &StateBasedActionContext,
    actions: &mut Vec<StateBasedAction>,
) {
    use crate::events::{KeywordActionEvent, KeywordActionKind};
    use crate::game_state::PlanarCardKind;

    for &object in game.face_up_planar_objects() {
        if game.planar_card_kind(object) != Some(PlanarCardKind::Phenomenon) {
            continue;
        }
        let encounter_event_pending =
            game.effect_store
                .pending_trigger_events
                .iter()
                .any(|event| {
                    event.downcast::<KeywordActionEvent>().is_some_and(|event| {
                        event.action == KeywordActionKind::EncounterPhenomenon
                            && event.source == object
                    })
                });
        let ability_pending = context.has_pending_ability_from(object)
            || game
                .effect_store
                .pending_trigger_entries
                .iter()
                .any(|entry| entry.source == object)
            || game
                .stack
                .iter()
                .any(|entry| entry.is_ability && entry.object_id == object);
        if !encounter_event_pending && !ability_pending {
            actions.push(StateBasedAction::PlaneswalkFromPhenomenon(object));
        }
    }
}

fn check_scheme_sba(
    game: &GameState,
    context: &StateBasedActionContext,
    actions: &mut Vec<StateBasedAction>,
) {
    use crate::events::{KeywordActionEvent, KeywordActionKind};

    let face_up = game
        .face_up_schemes()
        .iter()
        .copied()
        .collect::<HashSet<_>>();
    if face_up.is_empty() {
        return;
    }
    // CR 704.6e considers triggered abilities of every scheme, rather than
    // only the nonongoing scheme that would be turned face down.
    let set_event_pending = game
        .effect_store
        .pending_trigger_events
        .iter()
        .any(|event| {
            event.downcast::<KeywordActionEvent>().is_some_and(|event| {
                event.action == KeywordActionKind::SetSchemeInMotion
                    && face_up.contains(&event.source)
            })
        });
    let scheme_ability_pending = face_up.iter().any(|source| {
        context.has_pending_ability_from(*source)
            || game
                .effect_store
                .pending_trigger_entries
                .iter()
                .any(|entry| entry.source == *source)
            || game
                .stack
                .iter()
                .any(|entry| entry.is_ability && entry.object_id == *source)
    });
    if set_event_pending || scheme_ability_pending {
        return;
    }
    actions.extend(
        face_up
            .into_iter()
            .filter(|object| !game.scheme_is_ongoing(*object))
            .map(StateBasedAction::RecycleScheme),
    );
}

fn players_in_apnap_order(game: &GameState) -> Vec<PlayerId> {
    game.team_apnap_player_order()
}

/// Check the sector-assignment state-based action (CR 704.5u).
fn check_space_sculptor_sbas_with_view(
    game: &GameState,
    view: &crate::derived_view::DerivedGameView<'_>,
    actions: &mut Vec<StateBasedAction>,
) {
    let mut sculptors = game
        .battlefield
        .iter()
        .copied()
        .filter(|&object| !game.is_phased_out(object))
        .filter(|&object| view.object_has_static_ability_id(object, StaticAbilityId::SpaceSculptor))
        .collect::<Vec<_>>();
    sculptors.sort_by_key(|object| object.0);

    if sculptors.is_empty() {
        // CR 702.158b also retains designations while a player controls an
        // ability whose source has space sculptor. The source snapshot is the
        // correct LKI surface after that permanent leaves the battlefield.
        let sculptor_source_ability_on_stack = game.stack.iter().any(|entry| {
            entry.is_ability
                && entry.source_snapshot.as_ref().is_some_and(|source| {
                    source.has_static_ability_id(StaticAbilityId::SpaceSculptor)
                })
        });
        if !sculptor_source_ability_on_stack && game.has_sector_designations() {
            actions.push(StateBasedAction::ClearSectorDesignations);
        }
        return;
    }

    let sculptor_controllers = sculptors
        .iter()
        .filter_map(|&object| game.current_controller(object))
        .collect::<HashSet<_>>();
    let mut creatures_by_controller = HashMap::<PlayerId, Vec<ObjectId>>::new();
    for &object in &game.battlefield {
        if game.is_phased_out(object)
            || game.sector_designation(object).is_some()
            || !view.object_has_card_type(object, CardType::Creature)
        {
            continue;
        }
        if let Some(controller) = game.current_controller(object) {
            creatures_by_controller
                .entry(controller)
                .or_default()
                .push(object);
        }
    }
    if creatures_by_controller.is_empty() {
        return;
    }

    let apnap = players_in_apnap_order(game);
    let mut creatures = Vec::new();
    // CR 704.5u's explicit first partition: players without a sculptor source.
    for controls_sculptor in [false, true] {
        for &player in &apnap {
            if sculptor_controllers.contains(&player) != controls_sculptor {
                continue;
            }
            if let Some(player_creatures) = creatures_by_controller.get(&player) {
                creatures.extend(player_creatures.iter().map(|&object| (player, object)));
            }
        }
    }

    if !creatures.is_empty() {
        actions.push(StateBasedAction::SectorDesignationChoices {
            source: sculptors[0],
            creatures,
        });
    }
}

fn check_soulbond_pair_sbas_with_view(
    game: &GameState,
    view: &crate::derived_view::DerivedGameView<'_>,
    actions: &mut Vec<StateBasedAction>,
) {
    let mut seen = HashSet::new();
    for (&left, &right) in game.soulbond_pairs() {
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

/// Check permanent-specific counter caps (CR 704.5r).
fn check_counter_limits_with_view(
    game: &GameState,
    view: &crate::derived_view::DerivedGameView<'_>,
    actions: &mut Vec<StateBasedAction>,
) {
    for &permanent in &game.battlefield {
        if game.is_phased_out(permanent) {
            continue;
        }
        let Some(object) = game.object(permanent) else {
            continue;
        };
        let Some(chars) = view.calculated_characteristics(permanent) else {
            continue;
        };

        let mut limits = Vec::<(CounterType, u32)>::new();
        for (counter_type, maximum) in chars
            .static_abilities
            .iter()
            .filter_map(|ability| ability.counter_limit())
        {
            if let Some((_, existing)) = limits
                .iter_mut()
                .find(|(existing_type, _)| *existing_type == counter_type)
            {
                *existing = (*existing).min(maximum);
            } else {
                limits.push((counter_type, maximum));
            }
        }
        limits.sort_by_key(|(counter_type, _)| counter_type.description());

        for (counter_type, maximum) in limits {
            let current = object.counters.get(&counter_type).copied().unwrap_or(0);
            if current > maximum {
                actions.push(StateBasedAction::CountersExceedMaximum {
                    permanent,
                    counter_type,
                    count: current - maximum,
                });
            }
        }
    }
}

/// Check player-related state-based actions.
fn check_player_sbas(game: &GameState, actions: &mut Vec<StateBasedAction>) {
    let mut checked_two_headed_teams = std::collections::HashSet::new();
    for player in &game.players {
        if !player.is_in_game() {
            continue;
        }

        // Check if player can actually lose the game (Platinum Angel effect)
        if !game.can_lose_game(player.id) {
            continue;
        }

        if let Some(team) = game
            .two_headed_giant()
            .and_then(|state| state.team_index(player.id))
        {
            if checked_two_headed_teams.insert(team) {
                if player.has_lethal_life() {
                    actions.push(StateBasedAction::PlayerLoses {
                        player: player.id,
                        reason: LoseReason::ZeroLife,
                    });
                }
                if player.poison_counters
                    >= game
                        .two_headed_giant_poison_threshold(player.id)
                        .expect("Two-Headed Giant team has a poison threshold")
                {
                    actions.push(StateBasedAction::PlayerLoses {
                        player: player.id,
                        reason: LoseReason::Poison,
                    });
                }
            }
        } else {
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
        }

        if game.commander_damage_loss_enabled()
            && player.commander_damage.values().any(|&damage| damage >= 21)
        {
            actions.push(StateBasedAction::PlayerLoses {
                player: player.id,
                reason: LoseReason::CommanderDamage,
            });
        }

        if player.attempted_draw_from_empty_library {
            actions.push(StateBasedAction::PlayerLoses {
                player: player.id,
                reason: LoseReason::DrewFromEmptyLibrary,
            });
        }
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
            !game.is_phased_out(obj_id)
                && game.current_controller(obj_id) == Some(player.id)
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
        if game.is_phased_out(obj_id) {
            continue;
        }
        let Some(obj) = game.object(obj_id) else {
            continue;
        };
        let calculated_subtypes = view.calculated_subtypes(obj_id);

        // Creature with 0 or less toughness dies. This is not destruction,
        // so indestructible and regeneration do not stop it.
        // IMPORTANT: Use calculated_toughness to account for counters and effects!
        if view.object_has_card_type(obj_id, CardType::Creature) {
            let is_indestructible = !game.can_be_destroyed(obj_id)
                || view.object_has_static_ability_id(obj.id, StaticAbilityId::Indestructible);

            // Use calculated toughness to include -1/-1 counters, pump effects, etc.
            if let Some(toughness) = view.calculated_toughness(obj_id)
                && toughness <= 0
            {
                actions.push(StateBasedAction::ObjectDies(obj_id));
                continue;
            }

            // Creature with lethal damage dies (unless indestructible)
            let damage_marked = game.damage_on(obj_id);
            if damage_marked > 0 {
                let lethal_damage_threshold =
                    lethal_damage_threshold_for_creature(game, view, obj_id);
                if lethal_damage_threshold
                    .is_some_and(|threshold| threshold > 0 && damage_marked >= threshold as u32)
                    && !is_indestructible
                {
                    actions.push(StateBasedAction::ObjectDies(obj_id));
                    continue;
                }
            }

            if game.has_deathtouch_damage_since_sba(obj_id) && !is_indestructible {
                let toughness_for_deathtouch = view
                    .calculated_toughness(obj_id)
                    .or_else(|| obj.toughness());
                if toughness_for_deathtouch.is_some_and(|toughness| toughness > 0) {
                    actions.push(StateBasedAction::ObjectDies(obj_id));
                    continue;
                }
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

        if view.object_has_card_type(obj_id, CardType::Battle) {
            let defense_counters = obj
                .counters
                .get(&CounterType::Defense)
                .copied()
                .unwrap_or(0);
            let defeat_ability_pending_or_stacked = context.has_pending_battle_defeat_from(obj_id)
                || game
                    .stack
                    .iter()
                    .any(|entry| entry.battle_defeat_source == Some(obj_id));
            if defense_counters == 0 && !defeat_ability_pending_or_stacked {
                actions.push(StateBasedAction::BattleDies(obj_id));
                continue;
            }

            let is_being_attacked = game.combat.as_ref().is_some_and(|combat| {
                !crate::combat_state::attackers_targeting_battle(combat, obj_id).is_empty()
            });
            let protector_is_legal = game
                .battle_protector(obj_id)
                .is_some_and(|protector| game.legal_battle_protectors(obj_id).contains(&protector));
            if !is_being_attacked && !protector_is_legal {
                if game.legal_battle_protectors(obj_id).is_empty() {
                    actions.push(StateBasedAction::BattleDies(obj_id));
                } else {
                    actions.push(StateBasedAction::BattleProtectorChoice(obj_id));
                }
                continue;
            }
        }

        // Aura not attached to anything or attached to an illegal object or player
        if view.object_has_card_type(obj_id, CardType::Enchantment)
            && calculated_subtypes.contains(&Subtype::Aura)
            && obj.attached_to.is_none()
        {
            if obj.is_bestow_overlay_active() {
                actions.push(StateBasedAction::BestowBecomesCreature(obj_id));
            } else {
                actions.push(StateBasedAction::AuraFallsOff(obj_id));
            }
        }

        if let Some(attached_target) = obj.attached_to {
            let is_aura = view.object_has_card_type(obj_id, CardType::Enchantment)
                && calculated_subtypes.contains(&Subtype::Aura);
            if view.object_has_card_type(obj_id, CardType::Battle)
                || view.object_has_card_type(obj_id, CardType::Creature)
            {
                actions.push(StateBasedAction::AttachmentBecomesUnattached(obj_id));
            } else if is_aura {
                if !attachment_can_attach_to_target(game, obj_id, attached_target)
                    || matches!(
                        attached_target,
                        AttachmentTarget::Object(attached_id)
                            if has_protection_from_source(game, attached_id, obj_id)
                                && !controlled_existing_attachment_is_preserved_by_protection_grant(
                                    game,
                                    view,
                                    attached_id,
                                    obj_id,
                                )
                    )
                {
                    if obj.is_bestow_overlay_active() {
                        actions.push(StateBasedAction::BestowBecomesCreature(obj_id));
                    } else {
                        actions.push(StateBasedAction::AuraFallsOff(obj_id));
                    }
                }
            } else {
                let is_equipment = calculated_subtypes.contains(&Subtype::Equipment);
                let protection_makes_attachment_illegal = is_equipment
                    && matches!(
                        attached_target,
                        AttachmentTarget::Object(attached_id)
                            if has_protection_from_source(game, attached_id, obj_id)
                                && !controlled_existing_attachment_is_preserved_by_protection_grant(
                                    game,
                                    view,
                                    attached_id,
                                    obj_id,
                                )
                    );
                if !attachment_can_attach_to_target(game, obj_id, attached_target)
                    || protection_makes_attachment_illegal
                {
                    actions.push(StateBasedAction::AttachmentBecomesUnattached(obj_id));
                }
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
            if lore_count >= max_chapter
                && !chapter_ability_pending_or_stacked
                && game.can_be_sacrificed(obj_id)
            {
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
        !game.is_phased_out(source_id)
            && game.controller_of_id(source_id) == Some(creature_controller)
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
    if toughness.is_none_or(|toughness| toughness <= 0) {
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
        if game.is_phased_out(obj_id) {
            continue;
        }
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

    // CR 704.5e applies to a spell copy in every zone other than the stack,
    // including destinations selected by a countering replacement effect.
    for object in game.objects_in_deterministic_order() {
        if object.kind == crate::object::ObjectKind::SpellCopy && object.zone != Zone::Stack {
            actions.push(StateBasedAction::CopyCeasesToExist(object.id));
        }
    }
}

/// Check for +1/+1 and -1/-1 counter annihilation.
fn check_counter_annihilation(game: &GameState, actions: &mut Vec<StateBasedAction>) {
    for &obj_id in &game.battlefield {
        if game.is_phased_out(obj_id) {
            continue;
        }
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
    let mut controller_exemptions = Vec::new();
    for &obj_id in &game.battlefield {
        if game.is_phased_out(obj_id) {
            continue;
        }
        if view.object_has_static_ability_id(obj_id, StaticAbilityId::LegendRuleDoesntApply) {
            return;
        }
        let Some(object) = game.object(obj_id) else {
            continue;
        };
        let controller = game.controller_of(object);
        if let Some(abilities) = view.static_abilities_rc(obj_id) {
            for ability in abilities.iter().filter(|ability| {
                ability.is_active(game, obj_id)
                    && matches!(
                        ability.id(),
                        StaticAbilityId::LegendRuleDoesntApplyToController
                            | StaticAbilityId::LegendRuleDoesntApplyToControllerTokens
                    )
            }) {
                let fallback =
                    if ability.id() == StaticAbilityId::LegendRuleDoesntApplyToControllerTokens {
                        let mut filter = crate::target::ObjectFilter::permanent();
                        filter.token = true;
                        filter
                    } else {
                        crate::target::ObjectFilter::permanent()
                    };
                controller_exemptions.push((
                    controller,
                    obj_id,
                    ability
                        .legend_rule_exemption_filter()
                        .cloned()
                        .unwrap_or(fallback),
                ));
            }
        }
    }

    // Group legendary permanents by current controller and current name. Copy effects and
    // other continuous effects can make an object legendary or change its name.
    // Groups preserve battlefield order: violation order feeds the decision-prompt
    // order, which must be identical on every peer for multiplayer replay.
    let mut legends: Vec<((PlayerId, String), Vec<ObjectId>)> = Vec::new();
    let mut group_indexes: crate::FxMap<(PlayerId, String), usize> = crate::FxMap::default();

    for &obj_id in &game.battlefield {
        if game.is_phased_out(obj_id) {
            continue;
        }
        let Some(chars) = view.calculated_characteristics(obj_id) else {
            continue;
        };
        if controller_exemptions
            .iter()
            .any(|(controller, source, filter)| {
                *controller == chars.controller
                    && game.object(obj_id).is_some_and(|candidate| {
                        filter.matches(
                            candidate,
                            &game.filter_context_for(*controller, Some(*source)),
                            game,
                        )
                    })
            })
        {
            continue;
        }

        if chars.supertypes.contains(&Supertype::Legendary) {
            let key = (chars.controller, chars.name.to_owned_string());
            if let Some(&index) = group_indexes.get(&key) {
                legends[index].1.push(obj_id);
            } else {
                group_indexes.insert(key.clone(), legends.len());
                legends.push((key, vec![obj_id]));
            }
        }
    }

    // Simultaneous choices by different players happen in APNAP order (rule 101.4).
    let apnap = game.team_apnap_player_order();
    let apnap_position = |player: PlayerId| {
        apnap
            .iter()
            .position(|candidate| *candidate == player)
            .unwrap_or(usize::MAX)
    };
    legends.sort_by_key(|&((player, _), _)| apnap_position(player));

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

/// Check the world rule (CR 704.5k).
///
/// `world_supertype_since` is calculated through layers. That makes a later
/// copy/type-changing effect newer than a printed World permanent and gives
/// every object affected by one simultaneous grant the same timestamp.
fn check_world_rule_with_view(
    game: &GameState,
    view: &crate::derived_view::DerivedGameView<'_>,
    actions: &mut Vec<StateBasedAction>,
) {
    let mut worlds = game
        .battlefield
        .iter()
        .copied()
        .filter(|&id| !game.is_phased_out(id))
        .filter_map(|id| {
            let chars = view.calculated_characteristics(id)?;
            chars.supertypes.contains(&Supertype::World).then_some((
                id,
                chars.world_supertype_since.unwrap_or(0),
                chars.controller,
            ))
        })
        .collect::<Vec<_>>();
    if worlds.len() < 2 {
        return;
    }

    worlds.sort_by_key(|&(id, timestamp, _)| (timestamp, id.0));
    let mut permanents: Vec<ObjectId> = if game.limited_range_of_influence().is_none() {
        let newest_timestamp = worlds
            .last()
            .map(|(_, timestamp, _)| *timestamp)
            .unwrap_or(0);
        let newest_count = worlds
            .iter()
            .filter(|(_, timestamp, _)| *timestamp == newest_timestamp)
            .count();
        if newest_count == 1 {
            worlds
                .iter()
                .filter_map(|(id, timestamp, _)| (*timestamp != newest_timestamp).then_some(*id))
                .collect()
        } else {
            worlds.iter().map(|(id, _, _)| *id).collect()
        }
    } else {
        // CR 801.12 applies the world rule to each permanent only when another
        // World is in its controller's (potentially asymmetric) range.
        worlds
            .iter()
            .filter_map(|&(world, timestamp, controller)| {
                let local = worlds
                    .iter()
                    .filter(|&&(candidate, _, _)| {
                        candidate == world
                            || game.object_is_within_range(controller, candidate, None)
                    })
                    .collect::<Vec<_>>();
                if local.len() < 2 {
                    return None;
                }
                let newest_timestamp = local
                    .iter()
                    .map(|(_, timestamp, _)| *timestamp)
                    .max()
                    .unwrap_or(timestamp);
                let newest_count = local
                    .iter()
                    .filter(|(_, candidate_timestamp, _)| *candidate_timestamp == newest_timestamp)
                    .count();
                (timestamp != newest_timestamp || newest_count > 1).then_some(world)
            })
            .collect()
    };
    permanents.sort_by_key(|id| id.0);
    permanents.dedup();
    if permanents.is_empty() {
        return;
    }
    actions.push(StateBasedAction::WorldRuleViolation { permanents });
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
    game.clear_empty_library_draw_attempts_since_sba();
    if actions.is_empty() {
        return false;
    }

    let mut simultaneous_zone_changes: HashMap<ObjectId, Zone> = HashMap::new();
    for action in &actions {
        match action {
            StateBasedAction::ObjectDies(object)
            | StateBasedAction::PlaneswalkerDies(object)
            | StateBasedAction::BattleDies(object)
            | StateBasedAction::AuraFallsOff(object)
            | StateBasedAction::SagaSacrifice(object) => {
                simultaneous_zone_changes.insert(*object, Zone::Graveyard);
            }
            StateBasedAction::WorldRuleViolation { permanents } => {
                simultaneous_zone_changes.extend(
                    permanents
                        .iter()
                        .copied()
                        .map(|object| (object, Zone::Graveyard)),
                );
            }
            _ => {}
        }
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
                        all_effects,
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
    let mut processed_player_losses = HashSet::new();
    for action in actions {
        // Skip legend rule - it requires player choice
        if matches!(action, StateBasedAction::LegendRuleViolation { .. }) {
            continue;
        }
        // CR 704.7: one replacement effect replaces every simultaneous SBA
        // that would make the same player lose. Collapse all loss reasons for
        // that player into one replaceable game-loss event.
        if let StateBasedAction::PlayerLoses { player, .. } = &action
            && !processed_player_losses.insert(*player)
        {
            continue;
        }
        apply_single_sba_with_snapshots(
            game,
            action,
            &pre_captured_snapshots,
            &damage_destroyed_object_ids,
            &simultaneous_zone_changes,
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

    // Preserve the canonical API for callers that only retained the chosen
    // object. Decision-driven callers should pass the already-computed group
    // to `apply_legend_rule_choice_from_group` and avoid rescanning the board.
    let candidates: Vec<ObjectId> = game
        .battlefield
        .iter()
        .filter_map(|&id| {
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

    apply_legend_rule_choice_from_group(game, keep, &candidates);
}

/// Apply one already-identified legend-rule violation.
///
/// The candidate order comes from the SBA scan and is kept stable for replay.
/// Every candidate is revalidated against one pre-move derived view so a stale
/// decision cannot move an unrelated permanent.
pub fn apply_legend_rule_choice_from_group(
    game: &mut GameState,
    keep: ObjectId,
    candidates: &[ObjectId],
) {
    if !candidates.contains(&keep) {
        return;
    }

    let view = crate::derived_view::DerivedGameView::new(game);
    let Some(keep_chars) = view.calculated_characteristics(keep) else {
        return;
    };
    if !keep_chars.supertypes.contains(&Supertype::Legendary) {
        return;
    }
    let name = keep_chars.name;
    let controller = keep_chars.controller;

    let mut seen = HashSet::new();
    let to_remove: Vec<(ObjectId, ObjectSnapshot)> = candidates
        .iter()
        .copied()
        .filter(|&id| id != keep && seen.insert(id))
        .filter_map(|id| {
            let chars = view.calculated_characteristics(id)?;
            if chars.controller != controller
                || chars.name != name
                || !chars.supertypes.contains(&Supertype::Legendary)
            {
                return None;
            }
            let object = game.object(id)?;
            Some((
                id,
                ObjectSnapshot::from_object_with_known_characteristics(object, game, Some(&chars)),
            ))
        })
        .collect();
    if to_remove.is_empty() {
        return;
    }

    // Legend-rule moves are simultaneous. Capture trigger-source LKI once
    // before any source leaves, then attach the same pre-event lookback set to
    // every member of the batch.
    let pre_event_lookback_source_snapshots =
        if game.may_have_triggered_abilities_for_event_kind(crate::events::EventKind::ZoneChange) {
            game.trigger_source_lookback_snapshots()
        } else {
            Vec::new()
        };
    drop(view);

    for (id, snapshot) in to_remove {
        game.move_object_with_snapshot_and_pre_event_lookback(
            id,
            Zone::Graveyard,
            crate::events::cause::EventCause::from_legend_rule(controller),
            Some(snapshot),
            &pre_event_lookback_source_snapshots,
        );
    }
}

/// Commit one already-collected CR 704.5u assignment batch atomically.
///
/// Revalidating the whole candidate vector before the first write prevents a
/// stale asynchronous answer from partially designating a changed battlefield.
pub(crate) fn apply_sector_designation_choices_from_group(
    game: &mut GameState,
    source: ObjectId,
    creatures: &[(PlayerId, ObjectId)],
    choices: &[crate::marker::SectorDesignation],
) -> bool {
    if creatures.is_empty() || creatures.len() != choices.len() {
        return false;
    }
    let current = check_state_based_actions(game);
    let still_current = current.iter().any(|action| {
        matches!(
            action,
            StateBasedAction::SectorDesignationChoices {
                source: current_source,
                creatures: current_creatures,
            } if *current_source == source && current_creatures == creatures
        )
    });
    if !still_current {
        return false;
    }

    for (&(_, creature), &sector) in creatures.iter().zip(choices) {
        game.set_sector_designation(creature, sector);
    }
    true
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
    simultaneous_zone_changes: &HashMap<ObjectId, Zone>,
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

        StateBasedAction::BattleDies(obj_id) => {
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

        StateBasedAction::PlaneswalkFromPhenomenon(source) => {
            if let Some(controller) = game
                .planar_controller_of_face(source)
                .or_else(|| game.planar_controller())
            {
                let mut ctx =
                    crate::effects::ExecutionContext::new(source, controller, decision_maker);
                let effect = crate::effect::Effect::emit_keyword_action(
                    crate::events::KeywordActionKind::Planeswalk,
                    1,
                );
                let _ = crate::effects::execute_effect(game, &effect, &mut ctx);
            }
        }

        StateBasedAction::RecycleScheme(source) => {
            let _ = game.turn_face_up_scheme_down(source);
        }

        StateBasedAction::BattleProtectorChoice(obj_id) => {
            game.choose_battle_protector(obj_id, decision_maker);
        }

        StateBasedAction::PlayerLoses { player, reason: _ } => {
            crate::events::processing::process_player_loss_with_simultaneous_zone_changes(
                game,
                player,
                decision_maker,
                simultaneous_zone_changes,
            );
        }

        StateBasedAction::StartEngines { player } => {
            game.start_engines(player);
        }

        StateBasedAction::ClearSectorDesignations => {
            game.clear_sector_designations();
        }

        StateBasedAction::SectorDesignationChoices { source, creatures } => {
            let options = crate::marker::SectorDesignation::ALL
                .into_iter()
                .enumerate()
                .map(|(index, sector)| {
                    crate::decisions::context::SelectableOption::new(index, sector.description())
                })
                .collect::<Vec<_>>();
            let mut choices = Vec::with_capacity(creatures.len());
            for &(player, creature) in &creatures {
                let name = game
                    .object(creature)
                    .map(|object| object.name.to_string())
                    .unwrap_or_else(|| "this creature".to_string());
                let context = crate::decisions::context::SelectOptionsContext::new(
                    player,
                    Some(source),
                    format!("Choose a sector for {name}"),
                    options.clone(),
                    1,
                    1,
                );
                let index = decision_maker
                    .decide_options(game, &context)
                    .first()
                    .copied()
                    .unwrap_or(0);
                if decision_maker.awaiting_choice() {
                    return;
                }
                choices.push(
                    crate::marker::SectorDesignation::from_option_index(index)
                        .unwrap_or(crate::marker::SectorDesignation::Alpha),
                );
            }
            apply_sector_designation_choices_from_group(game, source, &creatures, &choices);
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

        StateBasedAction::WorldRuleViolation { permanents } => {
            use crate::events::processing::{ZoneChangeOutcome, process_zone_change_with_snapshot};

            // Determine every replacement result while all World permanents
            // still exist, then commit the zone changes with one shared
            // pre-event lookback set. This preserves simultaneous LKI for
            // leaves/dies observers even though storage commits one object at
            // a time.
            let mut prepared = Vec::new();
            for obj_id in permanents {
                let snapshot = pre_captured_snapshots.get(&obj_id).cloned().or_else(|| {
                    game.object(obj_id).map(|object| {
                        ObjectSnapshot::from_object_with_calculated_characteristics(object, game)
                    })
                });
                let outcome = process_zone_change_with_snapshot(
                    game,
                    obj_id,
                    Zone::Battlefield,
                    Zone::Graveyard,
                    crate::events::cause::EventCause::from_sba(),
                    decision_maker,
                    snapshot.clone(),
                );
                if let ZoneChangeOutcome::Proceed(final_zone) = outcome {
                    prepared.push((obj_id, final_zone, snapshot));
                }
            }
            let lookback = if game
                .may_have_triggered_abilities_for_event_kind(crate::events::EventKind::ZoneChange)
            {
                game.trigger_source_lookback_snapshots()
            } else {
                Vec::new()
            };
            for (obj_id, final_zone, snapshot) in prepared {
                game.move_object_with_snapshot_and_pre_event_lookback(
                    obj_id,
                    final_zone,
                    crate::events::cause::EventCause::from_sba(),
                    snapshot,
                    &lookback,
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
            for counter_type in [CounterType::PlusOnePlusOne, CounterType::MinusOneMinusOne] {
                if let Some((_, event)) =
                    game.remove_counters(permanent, counter_type, count, None, None)
                {
                    game.queue_trigger_event(event.provenance(), event);
                }
            }
        }

        StateBasedAction::CountersExceedMaximum {
            permanent,
            counter_type,
            count,
        } => {
            if let Some((_, event)) =
                game.remove_counters(permanent, counter_type, count, None, None)
            {
                game.queue_trigger_event(event.provenance(), event);
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
            let Some(controller) = game
                .object(obj_id)
                .map(|object| game.current_controller(obj_id).unwrap_or(object.owner))
            else {
                return;
            };
            let mut ctx = crate::effects::ExecutionContext::new(obj_id, controller, decision_maker)
                .with_cause(crate::events::cause::EventCause::from_sba());
            if let Ok(outcome) = crate::effects::execute_effect(
                game,
                &crate::effect::Effect::sacrifice_source(),
                &mut ctx,
            ) {
                for event in outcome.events {
                    game.queue_trigger_event(event.provenance(), event);
                }
            }
        }

        StateBasedAction::CommanderReturnsToCommandZone(obj_id) => {
            let Some(obj) = game.object(obj_id) else {
                return;
            };
            let owner = obj.owner;
            let name = obj.name.to_string();
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

    fn legendary_creature_definition(card_id: u32, name: &str) -> crate::cards::CardDefinition {
        crate::cards::builders::CardDefinitionBuilder::new(CardId::from_raw(card_id), name)
            .supertypes(vec![crate::types::Supertype::Legendary])
            .card_types(vec![CardType::Creature])
            .power_toughness(crate::card::PowerToughness::fixed(2, 2))
            .build()
    }

    fn controller_legend_rule_exemption_definition(card_id: u32) -> crate::cards::CardDefinition {
        crate::cards::builders::CardDefinitionBuilder::new(
            CardId::from_raw(card_id),
            "Scoped Legend Exemption",
        )
        .card_types(vec![CardType::Artifact])
        .with_ability(Ability::static_ability(
            StaticAbility::legend_rule_doesnt_apply_to_controller(),
        ))
        .build()
    }

    fn creature_legend_rule_exemption_definition(card_id: u32) -> crate::cards::CardDefinition {
        crate::cards::builders::CardDefinitionBuilder::new(
            CardId::from_raw(card_id),
            "Creature Legend Exemption",
        )
        .card_types(vec![CardType::Artifact])
        .with_ability(Ability::static_ability(
            StaticAbility::legend_rule_doesnt_apply_to_controller_matching(
                crate::target::ObjectFilter::creature(),
            ),
        ))
        .build()
    }

    fn legendary_artifact_definition(card_id: u32, name: &str) -> crate::cards::CardDefinition {
        crate::cards::builders::CardDefinitionBuilder::new(CardId::from_raw(card_id), name)
            .supertypes(vec![crate::types::Supertype::Legendary])
            .card_types(vec![CardType::Artifact])
            .build()
    }

    fn controller_token_legend_rule_exemption_definition(
        card_id: u32,
    ) -> crate::cards::CardDefinition {
        crate::cards::builders::CardDefinitionBuilder::new(
            CardId::from_raw(card_id),
            "Token Legend Exemption",
        )
        .card_types(vec![CardType::Artifact])
        .with_ability(Ability::static_ability(
            StaticAbility::legend_rule_doesnt_apply_to_tokens_you_control(),
        ))
        .build()
    }

    fn create_final_chapter_saga(game: &mut GameState, owner: PlayerId, name: &str) -> ObjectId {
        let saga = CardBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Enchantment])
            .subtypes(vec![Subtype::Saga])
            .build();
        let saga_id = game.create_object_from_card(&saga, owner, Zone::Battlefield);
        game.object_mut(saga_id)
            .expect("Saga should exist")
            .abilities_mut()
            .push(Ability::triggered(
                crate::triggers::Trigger::saga_chapter(vec![1]),
                Vec::<crate::effect::Effect>::new(),
            ));
        game.object_mut(saga_id)
            .expect("Saga should exist")
            .add_counters(CounterType::Lore, 1);
        saga_id
    }

    #[test]
    fn legend_rule_violations_use_stable_apnap_order() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let alice_legend = legendary_creature_definition(401, "Alice Twin");
        let bob_legend = legendary_creature_definition(402, "Bob Twin");
        game.create_object_from_definition(&alice_legend, alice, Zone::Battlefield);
        game.create_object_from_definition(&alice_legend, alice, Zone::Battlefield);
        game.create_object_from_definition(&bob_legend, bob, Zone::Battlefield);
        game.create_object_from_definition(&bob_legend, bob, Zone::Battlefield);

        // Bob is the active player, so APNAP puts his violation first.
        game.turn.active_player = bob;

        let expected = vec![
            (bob, "Bob Twin".to_string()),
            (alice, "Alice Twin".to_string()),
        ];
        // Violation order feeds decision-prompt order, which multiplayer replay
        // consumes positionally — it must be identical on every check.
        for _ in 0..50 {
            let order: Vec<(PlayerId, String)> = check_state_based_actions(&game)
                .into_iter()
                .filter_map(|action| match action {
                    StateBasedAction::LegendRuleViolation { player, name, .. } => {
                        Some((player, name))
                    }
                    _ => None,
                })
                .collect();
            assert_eq!(order, expected);
        }
    }

    #[test]
    fn controller_scoped_legend_rule_exemption_does_not_protect_opponents() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let alice_legend = legendary_creature_definition(411, "Alice Twin");
        let bob_legend = legendary_creature_definition(412, "Bob Twin");
        game.create_object_from_definition(&alice_legend, alice, Zone::Battlefield);
        game.create_object_from_definition(&alice_legend, alice, Zone::Battlefield);
        game.create_object_from_definition(&bob_legend, bob, Zone::Battlefield);
        game.create_object_from_definition(&bob_legend, bob, Zone::Battlefield);
        let exemption = controller_legend_rule_exemption_definition(413);
        game.create_object_from_definition(&exemption, alice, Zone::Battlefield);

        let violations = check_state_based_actions(&game)
            .into_iter()
            .filter_map(|action| match action {
                StateBasedAction::LegendRuleViolation { player, name, .. } => Some((player, name)),
                _ => None,
            })
            .collect::<Vec<_>>();

        assert_eq!(violations, vec![(bob, "Bob Twin".to_string())]);
    }

    #[test]
    fn creature_filtered_legend_rule_exemption_leaves_noncreatures_subject_to_the_rule() {
        let mut game = GameState::new(vec!["Alice".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let creature = legendary_creature_definition(417, "Creature Twin");
        let artifact = legendary_artifact_definition(418, "Relic Twin");
        for _ in 0..2 {
            game.create_object_from_definition(&creature, alice, Zone::Battlefield);
            game.create_object_from_definition(&artifact, alice, Zone::Battlefield);
        }
        let exemption = creature_legend_rule_exemption_definition(419);
        game.create_object_from_definition(&exemption, alice, Zone::Battlefield);

        let violations = check_state_based_actions(&game)
            .into_iter()
            .filter_map(|action| match action {
                StateBasedAction::LegendRuleViolation { name, .. } => Some(name),
                _ => None,
            })
            .collect::<Vec<_>>();

        assert_eq!(violations, vec!["Relic Twin".to_string()]);
    }

    #[test]
    fn token_scoped_legend_rule_exemption_leaves_nontoken_duplicates_subject_to_rule() {
        let mut game = GameState::new(vec!["Alice".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let token_legend = legendary_creature_definition(414, "Token Twin");
        let nontoken_legend = legendary_creature_definition(415, "Nontoken Twin");

        for _ in 0..2 {
            let token = game.create_object_from_definition(&token_legend, alice, Zone::Battlefield);
            game.object_mut(token)
                .expect("token legend should exist")
                .kind = crate::object::ObjectKind::Token;
        }
        game.create_object_from_definition(&nontoken_legend, alice, Zone::Battlefield);
        game.create_object_from_definition(&nontoken_legend, alice, Zone::Battlefield);
        let exemption = controller_token_legend_rule_exemption_definition(416);
        game.create_object_from_definition(&exemption, alice, Zone::Battlefield);

        let violations = check_state_based_actions(&game)
            .into_iter()
            .filter_map(|action| match action {
                StateBasedAction::LegendRuleViolation { name, .. } => Some(name),
                _ => None,
            })
            .collect::<Vec<_>>();

        assert_eq!(violations, vec!["Nontoken Twin".to_string()]);
    }

    #[test]
    fn legend_rule_batch_reuses_precomputed_characteristics_for_lki() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let legend = legendary_creature_definition(403, "Many Memorials");
        let legends: Vec<_> = (0..6)
            .map(|_| game.create_object_from_definition(&legend, alice, Zone::Battlefield))
            .collect();

        game.refresh_continuous_state();
        game.prewarm_calculated_characteristics(&game.battlefield.clone());
        let before = game.work_counters();

        apply_legend_rule_choice_from_group(&mut game, legends[0], &legends);

        let after = game.work_counters();
        assert_eq!(
            after.characteristics_full_recomputes, before.characteristics_full_recomputes,
            "all legend-rule LKI snapshots should reuse the pre-mutation characteristic batch"
        );
        assert_eq!(
            game.battlefield
                .iter()
                .filter(|&&id| game
                    .object(id)
                    .is_some_and(|object| object.name == "Many Memorials"))
                .count(),
            1
        );
    }

    #[test]
    fn known_legend_group_does_not_recalculate_unrelated_battlefield_objects() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let creature = creature_card(404, "Unrelated Creature", 1, 1);
        let unrelated: Vec<_> = (0..128)
            .map(|_| game.create_object_from_card(&creature, alice, Zone::Battlefield))
            .collect();
        let legend = legendary_creature_definition(405, "Scoped Legends");
        let legends: Vec<_> = (0..6)
            .map(|_| game.create_object_from_definition(&legend, alice, Zone::Battlefield))
            .collect();
        let other_legend = game.create_object_from_definition(
            &legendary_creature_definition(406, "Different Legend"),
            alice,
            Zone::Battlefield,
        );
        game.effect_store
            .continuous_effects
            .add_effect(ContinuousEffect::new(
                unrelated[0],
                alice,
                EffectTarget::AllCreatures,
                Modification::ModifyPowerToughness {
                    power: 1,
                    toughness: 1,
                },
            ));
        game.refresh_continuous_state();
        let before = game.work_counters();
        let mut supplied_group = legends.clone();
        supplied_group.push(other_legend);

        apply_legend_rule_choice_from_group(&mut game, legends[0], &supplied_group);

        let after = game.work_counters();
        assert!(
            after
                .characteristics_full_recomputes
                .saturating_sub(before.characteristics_full_recomputes)
                <= (legends.len() * 2) as u64,
            "only the supplied same-name legend group should need layered characteristics"
        );
        assert!(game.battlefield.contains(&other_legend));
        assert_eq!(
            legends
                .iter()
                .filter(|id| game.battlefield.contains(id))
                .count(),
            1
        );
    }

    #[test]
    fn sba_scan_reuses_supplied_view_for_indestructible_checks() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let creature = creature_card(408, "SBA Creature", 2, 2);
        let creatures: Vec<_> = (0..128)
            .map(|_| game.create_object_from_card(&creature, alice, Zone::Battlefield))
            .collect();
        game.effect_store
            .continuous_effects
            .add_effect(ContinuousEffect::new(
                creatures[0],
                alice,
                EffectTarget::AllCreatures,
                Modification::ModifyPowerToughness {
                    power: 1,
                    toughness: 1,
                },
            ));
        let effects = game.all_continuous_effects();
        let view = crate::derived_view::DerivedGameView::from_effects(&game, effects);
        view.prewarm_characteristics(&game.battlefield);
        let before = game.work_counters();

        let actions = check_state_based_actions_with_view(&game, &view);

        let after = game.work_counters();
        assert!(actions.is_empty());
        assert_eq!(
            after.characteristics_full_recomputes, before.characteristics_full_recomputes,
            "the indestructible check should reuse the SBA view instead of recalculating through GameState"
        );
    }

    #[test]
    fn legend_rule_leavers_share_pre_event_lki_and_batch_trigger_event() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let legend = legendary_creature_definition(407, "Doomed Legends");
        let legends: Vec<_> = (0..3)
            .map(|_| game.create_object_from_definition(&legend, alice, Zone::Battlefield))
            .collect();
        for &legend_id in &legends {
            game.object_mut(legend_id)
                .expect("legend should exist")
                .abilities_mut()
                .push(crate::ability::dies_trigger(vec![
                    crate::effect::Effect::gain_life(1),
                ]));
        }
        game.refresh_continuous_state();

        apply_legend_rule_choice_from_group(&mut game, legends[0], &legends);
        let mut trigger_queue = TriggerQueue::new();
        crate::game_loop::drain_pending_trigger_events(&mut game, &mut trigger_queue);

        assert_eq!(trigger_queue.entries.len(), 2);
        for entry in &trigger_queue.entries {
            let zone_change = entry
                .triggering_event
                .downcast::<crate::events::zones::ZoneChangeEvent>()
                .expect("dies trigger should retain its zone-change event");
            assert_eq!(
                zone_change.snapshots().len(),
                2,
                "each departing legend should see the full simultaneous legend-rule batch"
            );
        }
    }

    #[test]
    fn zero_toughness_sba_ignores_indestructible() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);

        let card = creature_card(399, "Indestructible Zero", 1, 0);
        let creature_id = game.create_object_from_card(&card, alice, Zone::Battlefield);
        game.object_mut(creature_id)
            .expect("indestructible zero should exist")
            .abilities_mut()
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
    fn counter_annihilation_queues_both_counter_removed_events() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let card = creature_card(408, "Counter Collision", 2, 2);
        let permanent = game.create_object_from_card(&card, alice, Zone::Battlefield);
        game.object_mut(permanent)
            .expect("permanent should exist")
            .add_counters(CounterType::PlusOnePlusOne, 3);
        game.object_mut(permanent)
            .expect("permanent should exist")
            .add_counters(CounterType::MinusOneMinusOne, 2);

        assert!(apply_state_based_actions(&mut game));
        assert_eq!(
            game.counter_count(permanent, CounterType::PlusOnePlusOne),
            1
        );
        assert_eq!(
            game.counter_count(permanent, CounterType::MinusOneMinusOne),
            0
        );

        let marker_events = game
            .take_pending_trigger_events()
            .into_iter()
            .filter_map(|event| {
                event
                    .downcast::<crate::events::MarkersChangedEvent>()
                    .cloned()
            })
            .collect::<Vec<_>>();
        assert_eq!(marker_events.len(), 2);
        for counter_type in [CounterType::PlusOnePlusOne, CounterType::MinusOneMinusOne] {
            assert!(marker_events.iter().any(|event| {
                event.is_removed()
                    && event.marker.as_counter() == Some(counter_type)
                    && event.object() == Some(permanent)
                    && event.amount == 2
                    && event.source.is_none()
                    && event.source_controller.is_none()
            }));
        }
    }

    #[test]
    fn final_chapter_saga_sba_uses_the_sacrifice_event_pipeline() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let saga_id = create_final_chapter_saga(&mut game, alice, "Final Chapter Probe");

        assert!(
            check_state_based_actions(&game).contains(&StateBasedAction::SagaSacrifice(saga_id))
        );
        assert!(apply_state_based_actions(&mut game));

        let moved_saga = game
            .current_object_id_after_zone_change(saga_id)
            .expect("the Saga should move to its owner's graveyard");
        assert!(game.object(moved_saga).is_some_and(|object| {
            object.zone == Zone::Graveyard && object.name == "Final Chapter Probe"
        }));
        assert!(game.take_pending_trigger_events().iter().any(|event| {
            event
                .downcast::<crate::events::permanents::SacrificeEvent>()
                .is_some_and(|sacrifice| sacrifice.permanent == saga_id)
        }));
    }

    #[test]
    fn final_chapter_saga_sacrifice_honors_zone_change_replacement() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let saga_id = create_final_chapter_saga(&mut game, alice, "Replaced Saga");
        let register =
            crate::effect::Effect::new(crate::effects::RegisterZoneReplacementEffect::new(
                crate::target::ChooseSpec::SpecificObject(saga_id),
                Some(Zone::Battlefield),
                Some(Zone::Graveyard),
                Zone::Exile,
                crate::effects::ReplacementApplyMode::OneShot,
            ));
        let mut dm = crate::decision::SelectFirstDecisionMaker;
        {
            let mut ctx = crate::effects::ExecutionContext::new(saga_id, alice, &mut dm);
            crate::effects::execute_effect(&mut game, &register, &mut ctx)
                .expect("replacement should register");
        }

        assert!(apply_state_based_actions_with(&mut game, &mut dm));
        let moved_saga = game
            .current_object_id_after_zone_change(saga_id)
            .expect("the replacement should retain the Saga in exile");
        assert_eq!(
            game.object(moved_saga)
                .expect("moved Saga should exist")
                .zone,
            Zone::Exile
        );
        assert!(game.take_pending_trigger_events().iter().any(|event| {
            event
                .downcast::<crate::events::permanents::SacrificeEvent>()
                .is_some_and(|sacrifice| sacrifice.permanent == saga_id)
        }));
    }

    #[test]
    fn simultaneous_sba_death_lki_uses_pre_sba_continuous_effects() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);

        let anthem_card = creature_card(400, "Doomed Marshal", 1, 1);
        let anthem_id = game.create_object_from_card(&anthem_card, alice, Zone::Battlefield);
        game.object_mut(anthem_id)
            .expect("anthem creature should exist")
            .abilities_mut()
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
    fn simultaneous_loss_reasons_are_one_replaceable_event() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let source = game.create_object_from_card(
            &CardBuilder::new(CardId::new(), "Loss Replacement")
                .card_types(vec![CardType::Artifact])
                .build(),
            alice,
            Zone::Battlefield,
        );
        game.effect_store.replacement_effects.add_resolution_effect(
            crate::replacement::ReplacementEffect::with_matcher(
                source,
                alice,
                crate::events::other::WouldLoseGameMatcher,
                crate::replacement::ReplacementAction::Instead(vec![
                    crate::effect::Effect::energy_counters(1),
                ]),
            ),
        );
        {
            let player = game.player_mut(alice).expect("alice");
            player.life = 0;
            player.poison_counters = 10;
        }

        let actions = check_state_based_actions(&game);
        assert_eq!(
            actions
                .iter()
                .filter(|action| matches!(action, StateBasedAction::PlayerLoses { .. }))
                .count(),
            2
        );
        let all_effects = game.all_continuous_effects();
        let mut dm = crate::decision::SelectFirstDecisionMaker;
        assert!(apply_state_based_actions_from_actions_with(
            &mut game,
            actions,
            &all_effects,
            &mut dm,
        ));

        let player = game.player(alice).expect("alice");
        assert!(player.is_in_game());
        assert_eq!(
            player.energy_counters, 1,
            "CR 704.7 requires one replacement to cover both simultaneous loss reasons"
        );
    }

    #[test]
    fn loss_replacement_source_controller_chooses_simultaneous_death_destination() {
        struct ChooseZone {
            player: PlayerId,
            option: usize,
        }

        impl DecisionMaker for ChooseZone {
            fn decide_options(
                &mut self,
                _game: &GameState,
                ctx: &crate::decisions::context::SelectOptionsContext,
            ) -> Vec<usize> {
                assert_eq!(ctx.player, self.player);
                assert_eq!(ctx.options.len(), 2);
                vec![self.option]
            }
        }

        for (option, expected_zone) in [(0, Zone::Exile), (1, Zone::Graveyard)] {
            let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
            let alice = PlayerId::from_index(0);
            let angel = game.create_object_from_card(
                &CardBuilder::new(CardId::new(), "Loss-Replacement Angel")
                    .card_types(vec![CardType::Creature])
                    .power_toughness(PowerToughness::fixed(5, 5))
                    .build(),
                alice,
                Zone::Battlefield,
            );
            game.effect_store.replacement_effects.add_resolution_effect(
                crate::replacement::ReplacementEffect::with_matcher(
                    angel,
                    alice,
                    crate::events::other::WouldLoseGameMatcher,
                    crate::replacement::ReplacementAction::Instead(vec![
                        crate::effect::Effect::exile(crate::target::ChooseSpec::Source),
                        crate::effect::Effect::set_life_total(20),
                    ]),
                ),
            );
            game.player_mut(alice).expect("alice").life = 0;
            game.mark_damage(angel, 5);

            let actions = check_state_based_actions(&game);
            assert!(actions.contains(&StateBasedAction::ObjectDies(angel)));
            assert!(actions.iter().any(|action| matches!(
                action,
                StateBasedAction::PlayerLoses { player, .. } if *player == alice
            )));
            let all_effects = game.all_continuous_effects();
            let mut dm = ChooseZone {
                player: alice,
                option,
            };
            assert!(apply_state_based_actions_from_actions_with(
                &mut game,
                actions,
                &all_effects,
                &mut dm,
            ));

            let player = game.player(alice).expect("alice");
            assert!(player.is_in_game());
            assert_eq!(player.life, 20);
            assert!(game.objects_in_zone(expected_zone).iter().any(|object_id| {
                game.object(*object_id)
                    .is_some_and(|object| object.name == "Loss-Replacement Angel")
            }));
            let other_zone = if expected_zone == Zone::Exile {
                Zone::Graveyard
            } else {
                Zone::Exile
            };
            assert!(!game.objects_in_zone(other_zone).iter().any(|object_id| {
                game.object(*object_id)
                    .is_some_and(|object| object.name == "Loss-Replacement Angel")
            }));
        }
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
    fn brawl_profile_disables_commander_damage_loss() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 25);
        let bob = PlayerId::from_index(1);
        game.player_mut(bob)
            .expect("bob should exist")
            .record_commander_damage(ObjectId::from_raw(100), 21);
        game.set_commander_damage_loss_enabled(false);

        assert!(!check_state_based_actions(&game).iter().any(|action| {
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
    fn empty_library_draw_attempt_becomes_loss_at_next_sba_pass() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let card = creature_card(299, "Only Card", 1, 1);
        game.create_object_from_card(&card, alice, Zone::Library);

        let drawn = game.draw_cards(alice, 3);

        assert_eq!(drawn.len(), 1, "the remaining card is drawn first");
        assert!(
            game.player(alice)
                .expect("Alice exists")
                .attempted_draw_from_empty_library
        );
        assert!(check_state_based_actions(&game).iter().any(|action| {
            matches!(
                action,
                StateBasedAction::PlayerLoses {
                    player,
                    reason: LoseReason::DrewFromEmptyLibrary,
                } if *player == alice
            )
        }));

        assert!(apply_state_based_actions(&mut game));
        assert!(game.player(alice).expect("Alice exists").has_lost);
        assert!(
            !game
                .player(alice)
                .expect("Alice exists")
                .attempted_draw_from_empty_library
        );
    }

    #[test]
    fn empty_draw_attempt_expires_when_player_cannot_lose() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        game.draw_cards(alice, 1);
        game.effect_store.cant_effects.cant_lose_game.insert(alice);

        assert!(!apply_state_based_actions(&mut game));
        assert!(!game.player(alice).expect("Alice exists").has_lost);
        assert!(
            !game
                .player(alice)
                .expect("Alice exists")
                .attempted_draw_from_empty_library,
            "the attempt expires when SBAs are checked even if losing is prohibited"
        );

        game.effect_store.cant_effects.cant_lose_game.remove(&alice);
        assert!(
            !check_state_based_actions(&game)
                .iter()
                .any(|action| matches!(
                    action,
                    StateBasedAction::PlayerLoses {
                        player,
                        reason: LoseReason::DrewFromEmptyLibrary,
                    } if *player == alice
                )),
            "removing the prohibition later must not revive an expired draw attempt"
        );
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

    fn siege_card(name: &str, defense: u32) -> crate::card::Card {
        CardBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Battle])
            .subtypes(vec![Subtype::Siege])
            .defense(defense)
            .build()
    }

    #[test]
    fn battle_intrinsics_seed_defense_and_siege_protector() {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let battle =
            game.create_object_from_card(&siege_card("Test Siege", 4), alice, Zone::Battlefield);

        assert_eq!(game.counter_count(battle, CounterType::Defense), 4);
        assert_eq!(game.battle_protector(battle), Some(bob));
    }

    #[test]
    fn zero_defense_battle_waits_for_defeat_ability_on_stack() {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = PlayerId::from_index(0);
        let battle = game.create_object_from_card(
            &siege_card("Defeated Siege", 1),
            alice,
            Zone::Battlefield,
        );
        game.object_mut(battle)
            .expect("battle")
            .counters
            .remove(&CounterType::Defense);
        game.stack.push(
            crate::game_state::StackEntry::new(battle, alice).with_battle_defeat_source(battle),
        );

        assert!(!check_state_based_actions(&game).contains(&StateBasedAction::BattleDies(battle)));
        game.stack.clear();
        assert!(check_state_based_actions(&game).contains(&StateBasedAction::BattleDies(battle)));
    }

    #[test]
    fn removing_last_siege_defense_counter_queues_intrinsic_defeat_ability() {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = PlayerId::from_index(0);
        let battle = game.create_object_from_card(
            &siege_card("Triggered Siege", 1),
            alice,
            Zone::Battlefield,
        );
        let (_, markers_event) = game
            .remove_counters(battle, CounterType::Defense, 1, None, Some(alice))
            .expect("the defense counter should be removed");
        let trigger_event = markers_event;
        let entries = crate::triggers::check_triggers(&game, &trigger_event);
        assert_eq!(entries.len(), 1);
        assert!(crate::triggers::check::is_intrinsic_siege_defeat_trigger(
            &entries[0]
        ));

        let mut queue = TriggerQueue::new();
        queue.add(entries[0].clone());
        let context = StateBasedActionContext::from_trigger_queue(&queue);
        let view = crate::derived_view::DerivedGameView::new(&game);
        assert!(
            !check_state_based_actions_with_context(&game, &view, &context)
                .contains(&StateBasedAction::BattleDies(battle)),
            "the zero-defense SBA must wait while the intrinsic trigger is pending"
        );
    }

    #[test]
    fn siege_defeat_trigger_uses_the_event_time_defense_count() {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = PlayerId::from_index(0);
        let battle = game.create_object_from_card(
            &siege_card("Event-Time Siege", 1),
            alice,
            Zone::Battlefield,
        );
        let (_, removal_event) = game
            .remove_counters(battle, CounterType::Defense, 1, None, Some(alice))
            .expect("the last defense counter should be removed");

        game.add_counters(battle, CounterType::Defense, 1)
            .expect("the later counter should be added");
        let entries = crate::triggers::check_triggers(&game, &removal_event);

        assert_eq!(entries.len(), 1);
        assert!(crate::triggers::check::is_intrinsic_siege_defeat_trigger(
            &entries[0]
        ));
    }

    #[test]
    fn intrinsic_siege_defeat_exiles_and_casts_the_linked_face() {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = PlayerId::from_index(0);
        let front_id = CardId::from_raw(991_001);
        let back_id = CardId::from_raw(991_002);
        let front = crate::cards::CardDefinitionBuilder::new(front_id, "Resolving Siege")
            .card_types(vec![CardType::Battle])
            .subtypes(vec![Subtype::Siege])
            .defense(1)
            .other_face(back_id)
            .other_face_name("Resolving Victory")
            .linked_face_layout(crate::card::LinkedFaceLayout::TransformLike)
            .build();
        let back = crate::cards::CardDefinitionBuilder::new(back_id, "Resolving Victory")
            .card_types(vec![CardType::Sorcery])
            .other_face(front_id)
            .other_face_name("Resolving Siege")
            .linked_face_layout(crate::card::LinkedFaceLayout::TransformLike)
            .build();
        game.register_linked_face_definition(&back);
        let battle = game.create_object_from_definition(&front, alice, Zone::Battlefield);
        let (_, markers_event) = game
            .remove_counters(battle, CounterType::Defense, 1, None, Some(alice))
            .expect("last defense counter");
        let mut queue = TriggerQueue::new();
        for entry in crate::triggers::check_triggers(&game, &markers_event) {
            queue.add(entry);
        }
        crate::game_loop::put_triggers_on_stack(&mut game, &mut queue)
            .expect("intrinsic defeat trigger should stack");
        assert_eq!(game.stack.len(), 1);
        assert_eq!(game.stack[0].battle_defeat_source, Some(battle));

        let mut dm = crate::decision::SelectFirstDecisionMaker;
        crate::game_loop::resolve_stack_entry_with(&mut game, &mut dm)
            .expect("intrinsic defeat trigger should resolve");

        assert_eq!(game.stack.len(), 1, "the linked face should be cast");
        let spell = game
            .object(game.stack[0].object_id)
            .expect("linked-face spell on stack");
        assert_eq!(spell.name, "Resolving Victory");
        assert!(spell.has_card_type(CardType::Sorcery));
    }

    #[test]
    fn battle_becomes_unattached_even_if_attachment_would_otherwise_be_legal() {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = PlayerId::from_index(0);
        let battle = game.create_object_from_card(
            &siege_card("Attached Siege", 3),
            alice,
            Zone::Battlefield,
        );
        let target = game.create_object_from_card(
            &CardBuilder::new(CardId::new(), "Target")
                .card_types(vec![CardType::Artifact])
                .build(),
            alice,
            Zone::Battlefield,
        );
        game.object_mut(battle).expect("battle").attached_to =
            Some(AttachmentTarget::Object(target));
        game.object_mut(target)
            .expect("target")
            .attachments
            .push(battle);

        assert!(
            check_state_based_actions(&game)
                .contains(&StateBasedAction::AttachmentBecomesUnattached(battle))
        );
        assert!(apply_state_based_actions(&mut game));
        assert_eq!(game.object(battle).expect("battle").attached_to, None);
    }

    #[test]
    fn siege_controller_chooses_a_new_protector_when_the_old_one_leaves() {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Charlie".into()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let charlie = PlayerId::from_index(2);
        let battle = game.create_object_from_card(
            &siege_card("Multiplayer Siege", 3),
            alice,
            Zone::Battlefield,
        );
        assert_eq!(game.battle_protector(battle), Some(bob));
        game.player_mut(bob).expect("Bob").has_left_game = true;

        let actions = check_state_based_actions(&game);
        assert!(actions.contains(&StateBasedAction::BattleProtectorChoice(battle)));
        let all_effects = game.all_continuous_effects();
        let mut dm = crate::decision::SelectFirstDecisionMaker;
        assert!(apply_state_based_actions_from_actions_with(
            &mut game,
            actions,
            &all_effects,
            &mut dm,
        ));
        assert_eq!(game.battle_protector(battle), Some(charlie));
    }

    #[test]
    fn siege_with_no_legal_protector_goes_to_its_owners_graveyard() {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let battle = game.create_object_from_card(
            &siege_card("Unprotected Siege", 3),
            alice,
            Zone::Battlefield,
        );
        game.player_mut(bob).expect("Bob").has_left_game = true;

        assert!(check_state_based_actions(&game).contains(&StateBasedAction::BattleDies(battle)));
        assert!(apply_state_based_actions(&mut game));
        assert!(game.object(battle).is_none());
        assert!(
            game.player(alice)
                .expect("Alice")
                .graveyard
                .iter()
                .any(|id| {
                    game.object(*id)
                        .is_some_and(|object| object.name == "Unprotected Siege")
                })
        );
    }

    #[test]
    fn battle_protector_designation_persists_through_type_changes() {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let battle = game.create_object_from_card(
            &siege_card("Changing Siege", 3),
            alice,
            Zone::Battlefield,
        );
        game.object_mut(battle)
            .expect("battle")
            .card_types
            .retain(|card_type| *card_type != CardType::Battle);
        assert_eq!(game.battle_protector(battle), Some(bob));
        game.object_mut(battle)
            .expect("battle")
            .card_types
            .push(CardType::Battle);
        assert_eq!(game.battle_protector(battle), Some(bob));
    }

    fn world_permanent(game: &mut GameState, owner: PlayerId, name: &str) -> ObjectId {
        let card = CardBuilder::new(CardId::new(), name)
            .supertypes(vec![Supertype::World])
            .card_types(vec![CardType::Enchantment])
            .build();
        game.create_object_from_card(&card, owner, Zone::Battlefield)
    }

    fn ordinary_enchantment(game: &mut GameState, owner: PlayerId, name: &str) -> ObjectId {
        let card = CardBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Enchantment])
            .build();
        game.create_object_from_card(&card, owner, Zone::Battlefield)
    }

    fn grant_world_at(game: &mut GameState, object: ObjectId, timestamp: u64) {
        let controller = game.current_controller(object).expect("controller");
        let mut effect = ContinuousEffect::new(
            object,
            controller,
            EffectTarget::Specific(object),
            Modification::AddSupertypes(vec![Supertype::World]),
        );
        effect.timestamp = timestamp;
        game.effect_store.continuous_effects.add_effect(effect);
        game.mark_continuous_state_dirty();
    }

    fn counter_limited_permanent(
        game: &mut GameState,
        owner: PlayerId,
        limits: &[(CounterType, u32)],
    ) -> ObjectId {
        let mut builder = crate::cards::builders::CardDefinitionBuilder::new(
            CardId::new(),
            "Counter-Limited Permanent",
        )
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2));
        for &(counter_type, maximum) in limits {
            builder =
                builder.with_ability(Ability::static_ability(StaticAbility::counter_limit_rule(
                    counter_type,
                    maximum,
                    format!(
                        "This permanent can't have more than {maximum} {} counters on it",
                        counter_type.description()
                    ),
                )));
        }
        let definition = builder.build();
        game.create_object_from_definition(&definition, owner, Zone::Battlefield)
    }

    #[test]
    fn u034_world_rule_keeps_only_the_unique_newest_world_permanent() {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = PlayerId::from_index(0);
        let old_world = world_permanent(&mut game, alice, "Old World");
        let new_world = world_permanent(&mut game, alice, "New World");

        assert_eq!(
            check_state_based_actions(&game)
                .into_iter()
                .find(|action| matches!(action, StateBasedAction::WorldRuleViolation { .. })),
            Some(StateBasedAction::WorldRuleViolation {
                permanents: vec![old_world]
            })
        );

        assert!(apply_state_based_actions(&mut game));
        assert!(
            game.object(new_world)
                .is_some_and(|object| object.zone == Zone::Battlefield)
        );
        assert!(
            game.object(old_world).is_none(),
            "zone changes use a new object id"
        );
        assert!(
            game.player(alice)
                .expect("Alice")
                .graveyard
                .iter()
                .any(|id| {
                    game.object(*id)
                        .is_some_and(|object| object.name == "Old World")
                })
        );
    }

    #[test]
    fn u034_simultaneous_world_grants_tie_and_remove_every_world_permanent() {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = PlayerId::from_index(0);
        let printed_world = world_permanent(&mut game, alice, "Printed World");
        let first = ordinary_enchantment(&mut game, alice, "Granted World One");
        let second = ordinary_enchantment(&mut game, alice, "Granted World Two");
        let printed_world_timestamp = crate::derived_view::DerivedGameView::new(&game)
            .calculated_characteristics(printed_world)
            .and_then(|chars| chars.world_supertype_since)
            .expect("printed World timestamp");
        let simultaneous_grant_timestamp =
            game.effect_store.continuous_effects.current_timestamp() + 1;
        grant_world_at(&mut game, first, simultaneous_grant_timestamp);
        grant_world_at(&mut game, second, simultaneous_grant_timestamp);

        let effects = crate::static_ability_processor::get_all_continuous_effects(&game);
        let view = crate::derived_view::DerivedGameView::from_effects(&game, effects);
        let printed_chars = view
            .calculated_characteristics(printed_world)
            .expect("printed World characteristics");
        assert!(printed_chars.supertypes.contains(&Supertype::World));
        assert_eq!(
            printed_chars.world_supertype_since,
            Some(printed_world_timestamp)
        );
        assert_eq!(
            view.calculated_characteristics(first)
                .and_then(|chars| chars.world_supertype_since),
            Some(simultaneous_grant_timestamp)
        );
        assert_eq!(
            view.calculated_characteristics(second)
                .and_then(|chars| chars.world_supertype_since),
            Some(simultaneous_grant_timestamp)
        );

        let action = check_state_based_actions(&game)
            .into_iter()
            .find(|action| matches!(action, StateBasedAction::WorldRuleViolation { .. }))
            .expect("world rule violation");
        let StateBasedAction::WorldRuleViolation { permanents } = action else {
            unreachable!()
        };
        assert_eq!(permanents, vec![printed_world, first, second]);

        assert!(apply_state_based_actions(&mut game));
        assert!(game.battlefield.is_empty());
        assert_eq!(game.player(alice).expect("Alice").graveyard.len(), 3);
    }

    #[test]
    fn u034_later_world_grant_uses_the_grant_timestamp_not_entry_timestamp() {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = PlayerId::from_index(0);
        let older_object = ordinary_enchantment(&mut game, alice, "Older Object");
        let later_printed_world = world_permanent(&mut game, alice, "Later Printed World");
        let later_grant_timestamp = game.effect_store.continuous_effects.current_timestamp() + 1;
        grant_world_at(&mut game, older_object, later_grant_timestamp);

        assert_eq!(
            check_state_based_actions(&game)
                .into_iter()
                .find(|action| matches!(action, StateBasedAction::WorldRuleViolation { .. })),
            Some(StateBasedAction::WorldRuleViolation {
                permanents: vec![later_printed_world]
            })
        );
    }

    #[test]
    fn u034_new_permanent_under_older_world_grant_uses_its_entry_timestamp() {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = PlayerId::from_index(0);
        let printed_world = world_permanent(&mut game, alice, "Printed World");
        let mut grant = ContinuousEffect::new(
            printed_world,
            alice,
            EffectTarget::AllPermanents,
            Modification::AddSupertypes(vec![Supertype::World]),
        );
        grant.timestamp = game.effect_store.continuous_effects.current_timestamp() + 1;
        game.effect_store.continuous_effects.add_effect(grant);
        game.mark_continuous_state_dirty();

        let newcomer = ordinary_enchantment(&mut game, alice, "Newly Affected World");
        let newcomer_entry = game
            .effect_store
            .continuous_effects
            .get_entry_timestamp(newcomer)
            .expect("newcomer entry timestamp");
        let newcomer_world_since = crate::derived_view::DerivedGameView::new(&game)
            .calculated_characteristics(newcomer)
            .and_then(|chars| chars.world_supertype_since);
        assert_eq!(newcomer_world_since, Some(newcomer_entry));
        assert_eq!(
            check_state_based_actions(&game)
                .into_iter()
                .find(|action| matches!(action, StateBasedAction::WorldRuleViolation { .. })),
            Some(StateBasedAction::WorldRuleViolation {
                permanents: vec![printed_world]
            })
        );
    }

    #[test]
    fn u035_counter_limit_removes_only_excess_and_queues_removal_event() {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = PlayerId::from_index(0);
        let permanent = counter_limited_permanent(&mut game, alice, &[(CounterType::Dream, 7)]);
        game.add_counters(permanent, CounterType::Dream, 10);

        assert!(check_state_based_actions(&game).contains(
            &StateBasedAction::CountersExceedMaximum {
                permanent,
                counter_type: CounterType::Dream,
                count: 3,
            }
        ));
        assert!(apply_state_based_actions(&mut game));
        assert_eq!(
            game.object(permanent)
                .and_then(|object| object.counters.get(&CounterType::Dream).copied()),
            Some(7)
        );
        assert_eq!(game.effect_store.pending_trigger_events.len(), 1);
    }

    #[test]
    fn u035_smallest_active_limit_wins_without_touching_other_counter_kinds() {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = PlayerId::from_index(0);
        let permanent = counter_limited_permanent(
            &mut game,
            alice,
            &[(CounterType::Dream, 7), (CounterType::Dream, 5)],
        );
        game.add_counters(permanent, CounterType::Dream, 8);
        game.add_counters(permanent, CounterType::Time, 9);

        assert!(apply_state_based_actions(&mut game));
        let object = game.object(permanent).expect("limited permanent");
        assert_eq!(object.counters.get(&CounterType::Dream), Some(&5));
        assert_eq!(object.counters.get(&CounterType::Time), Some(&9));
    }

    #[test]
    fn u035_lost_counter_limit_does_not_generate_a_state_based_action() {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = PlayerId::from_index(0);
        let permanent = counter_limited_permanent(&mut game, alice, &[(CounterType::Dream, 7)]);
        game.add_counters(permanent, CounterType::Dream, 10);
        game.effect_store
            .continuous_effects
            .add_effect(ContinuousEffect::from_resolution(
                permanent,
                alice,
                vec![permanent],
                Modification::RemoveAllAbilities,
            ));

        assert!(!check_state_based_actions(&game).iter().any(|action| {
            matches!(
                action,
                StateBasedAction::CountersExceedMaximum {
                    permanent: candidate,
                    ..
                } if *candidate == permanent
            )
        }));
    }

    fn u036_space_sculptor_source(
        game: &mut GameState,
        controller: PlayerId,
        name: &str,
    ) -> ObjectId {
        let definition = crate::cards::CardDefinitionBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Artifact])
            .with_ability(Ability::static_ability(StaticAbility::space_sculptor()))
            .build();
        game.create_object_from_definition(&definition, controller, Zone::Battlefield)
    }

    fn u036_creature(game: &mut GameState, controller: PlayerId, name: &str) -> ObjectId {
        let card = CardBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        game.create_object_from_card(&card, controller, Zone::Battlefield)
    }

    #[test]
    fn u036_sector_sba_uses_opponent_first_partitions_and_apnap_within_them() {
        let mut game = GameState::new(
            vec![
                "Alice".into(),
                "Bob".into(),
                "Charlie".into(),
                "Dana".into(),
            ],
            20,
        );
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let charlie = PlayerId::from_index(2);
        let dana = PlayerId::from_index(3);
        game.turn.active_player = alice;
        let source = u036_space_sculptor_source(&mut game, alice, "Alice Sculptor");
        u036_space_sculptor_source(&mut game, charlie, "Charlie Sculptor");
        let alice_creature = u036_creature(&mut game, alice, "Alice Creature");
        let bob_creature = u036_creature(&mut game, bob, "Bob Creature");
        let charlie_creature = u036_creature(&mut game, charlie, "Charlie Creature");
        let dana_creature = u036_creature(&mut game, dana, "Dana Creature");

        let action = check_state_based_actions(&game)
            .into_iter()
            .find(|action| matches!(action, StateBasedAction::SectorDesignationChoices { .. }))
            .expect("sector assignment SBA");
        assert_eq!(
            action,
            StateBasedAction::SectorDesignationChoices {
                source,
                creatures: vec![
                    (bob, bob_creature),
                    (dana, dana_creature),
                    (alice, alice_creature),
                    (charlie, charlie_creature),
                ],
            }
        );
    }

    struct SectorDecisionMaker {
        choices: std::collections::VecDeque<usize>,
    }

    impl DecisionMaker for SectorDecisionMaker {
        fn decide_options(
            &mut self,
            _game: &GameState,
            _ctx: &crate::decisions::context::SelectOptionsContext,
        ) -> Vec<usize> {
            vec![self.choices.pop_front().unwrap_or(0)]
        }
    }

    #[test]
    fn u036_designations_are_noncopying_zone_scoped_and_expire_without_sculptor() {
        use crate::marker::SectorDesignation::{Alpha, Beta};

        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = PlayerId::from_index(0);
        let source = u036_space_sculptor_source(&mut game, alice, "Space Sculptor");
        let copied_card = CardBuilder::new(CardId::new(), "Original")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        let creature = game.create_object_from_card(&copied_card, alice, Zone::Battlefield);
        assert!(game.set_sector_designation(creature, Alpha));
        let independent_copy = game.create_object_from_card(&copied_card, alice, Zone::Battlefield);
        assert_eq!(game.sector_designation(creature), Some(Alpha));
        assert_eq!(game.sector_designation(independent_copy), None);
        let mut dm = SectorDecisionMaker {
            choices: [1].into_iter().collect(),
        };

        assert!(apply_state_based_actions_with(&mut game, &mut dm));
        assert_eq!(game.sector_designation(creature), Some(Alpha));
        assert_eq!(game.sector_designation(independent_copy), Some(Beta));
        assert!(!game.permanents_are_in_same_sector(creature, independent_copy));

        let new_id = game
            .move_object_by_effect(creature, Zone::Exile)
            .expect("zone change creates a new object");
        assert_eq!(game.sector_designation(creature), None);
        assert_eq!(game.sector_designation(new_id), None);

        let source_snapshot =
            crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(
                game.object(source).expect("sculptor source"),
                &game,
            );
        game.stack.push(
            crate::game_state::StackEntry::ability(
                source,
                alice,
                crate::resolution::ResolutionProgram::default(),
            )
            .with_source_snapshot(source_snapshot),
        );
        game.move_object_by_effect(source, Zone::Graveyard);
        assert!(
            !check_state_based_actions(&game).contains(&StateBasedAction::ClearSectorDesignations),
            "a controlled ability whose source had space sculptor retains designations"
        );
        game.stack.pop();
        assert!(
            check_state_based_actions(&game).contains(&StateBasedAction::ClearSectorDesignations)
        );
        assert!(apply_state_based_actions(&mut game));
        assert_eq!(game.sector_designation(independent_copy), None);
    }
}
