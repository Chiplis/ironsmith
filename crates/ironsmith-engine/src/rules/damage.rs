//! Damage calculation rules for MTG.
//!
//! This module handles damage-related rules including:
//! - Deathtouch (any damage is lethal)
//! - Lifelink (controller gains life)
//! - Infect (damage as -1/-1 counters to creatures, poison to players)
//! - Wither (damage as -1/-1 counters to creatures)
//! - Trample (excess damage goes to defending player)

use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::object::Object;
use crate::static_abilities::StaticAbilityId;
use crate::types::CardType;

/// The target of damage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DamageTarget {
    /// Damage to a creature, planeswalker, or battle.
    Permanent,
    /// Damage to a player.
    Player(PlayerId),
}

/// Result of calculating damage, including side effects from keywords.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DamageResult {
    /// Amount of normal damage dealt to the target.
    pub damage_dealt: u32,

    /// Life gained by the controller (from lifelink).
    pub life_gained: u32,

    /// Poison counters given to a player (from infect).
    pub poison_counters: u32,

    /// -1/-1 counters to place on a creature (from wither/infect).
    pub minus_counters: u32,

    /// Excess damage (CR 120.10) this combat damage event dealt to a
    /// permanent, carried onto its Damage trigger event.
    pub excess_damage: u32,

    /// Whether the damage source has deathtouch.
    pub has_deathtouch: bool,

    /// Whether the damage source has infect.
    pub has_infect: bool,

    /// Whether the damage source has wither.
    pub has_wither: bool,

    /// Whether the damage source has lifelink.
    pub has_lifelink: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct SourceDamageKeywords {
    pub has_deathtouch: bool,
    pub has_infect: bool,
    pub has_wither: bool,
    pub has_lifelink: bool,
}

pub(crate) fn source_damage_keywords(
    game: &GameState,
    source: ObjectId,
    source_snapshot: Option<&crate::snapshot::ObjectSnapshot>,
) -> SourceDamageKeywords {
    // Damage from a still-present source uses its current characteristics
    // (CR 608.2h), even when its ability was put on the stack earlier.
    if game.object(source).is_some() && !game.is_phased_out(source) {
        return SourceDamageKeywords {
            has_deathtouch: game.object_has_static_ability_id(source, StaticAbilityId::Deathtouch),
            has_infect: game.object_has_static_ability_id(source, StaticAbilityId::Infect),
            has_wither: game.object_has_static_ability_id(source, StaticAbilityId::Wither),
            has_lifelink: game.object_has_static_ability_id(source, StaticAbilityId::Lifelink),
        };
    }

    if let Some(snapshot) = source_snapshot {
        return SourceDamageKeywords {
            has_deathtouch: snapshot.has_static_ability_id(StaticAbilityId::Deathtouch),
            has_infect: snapshot.has_static_ability_id(StaticAbilityId::Infect),
            has_wither: snapshot.has_static_ability_id(StaticAbilityId::Wither),
            has_lifelink: snapshot.has_static_ability_id(StaticAbilityId::Lifelink),
        };
    }

    SourceDamageKeywords::default()
}

#[derive(Debug, Clone)]
pub(crate) struct AppliedDamageAssignment<Output = crate::effect::EffectOutcome> {
    pub applied: bool,
    pub life_lost: u32,
    pub consequence_outcome: Option<Output>,
}

impl<Output> Default for AppliedDamageAssignment<Output> {
    fn default() -> Self {
        Self {
            applied: false,
            life_lost: 0,
            consequence_outcome: None,
        }
    }
}
impl AppliedDamageAssignment<crate::effects::CompletedEffectOutputs> {
    fn into_aggregate(self) -> AppliedDamageAssignment {
        AppliedDamageAssignment {
            applied: self.applied,
            life_lost: self.life_lost,
            consequence_outcome: self
                .consequence_outcome
                .map(crate::effects::CompletedEffectOutputs::into_outcome),
        }
    }
}

pub(crate) fn apply_processed_damage_assignment(
    game: &mut GameState,
    source: ObjectId,
    target: crate::events::DamageTarget,
    amount: u32,
    keywords: SourceDamageKeywords,
    cause: crate::events::cause::EventCause,
) -> AppliedDamageAssignment {
    let mut dm = crate::decision::SelectFirstDecisionMaker;
    apply_processed_damage_assignment_with_dm(
        game, source, target, amount, keywords, cause, &mut dm,
    )
    .expect("damage consequences failed")
}

/// Apply the consequences of damage with the caller's decision maker. Life
/// loss and counters can have replacement choices of their own.
pub(crate) fn apply_processed_damage_assignment_with_dm(
    game: &mut GameState,
    source: ObjectId,
    target: crate::events::DamageTarget,
    amount: u32,
    keywords: SourceDamageKeywords,
    cause: crate::events::cause::EventCause,
    dm: &mut dyn crate::decision::DecisionMaker,
) -> Result<AppliedDamageAssignment, crate::effects::ExecutionError> {
    apply_processed_damage_assignment_with_scope(
        game,
        source,
        target,
        amount,
        keywords,
        cause,
        dm,
        &crate::effects::ReplacementExecutionContext::default(),
        None,
        crate::provenance::ProvNodeId::default(),
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn apply_processed_damage_assignment_with_scope(
    game: &mut GameState,
    source: ObjectId,
    target: crate::events::DamageTarget,
    amount: u32,
    keywords: SourceDamageKeywords,
    cause: crate::events::cause::EventCause,
    dm: &mut dyn crate::decision::DecisionMaker,
    replacement_scope: &crate::effects::ReplacementExecutionContext,
    source_snapshot: Option<&crate::snapshot::ObjectSnapshot>,
    provenance: crate::provenance::ProvNodeId,
) -> Result<AppliedDamageAssignment, crate::effects::ExecutionError> {
    let checkpoint=game.clone();
    let controller=source_snapshot.map(|snapshot|snapshot.controller)
        .or(cause.source_controller).or_else(||game.current_controller(source))
        .unwrap_or_else(|| match target {crate::events::DamageTarget::Player(player)=>player,crate::events::DamageTarget::Object(id)=>game.current_controller(id).unwrap_or(game.turn.active_player)});
    let mut ctx=crate::effects::ExecutionContext::new(source,controller,dm).with_cause(cause);
    ctx.replacement=replacement_scope.clone();ctx.source_snapshot=source_snapshot.cloned();ctx.provenance=provenance;
    let result=(||{
        let prepared=prepare_processed_damage_assignment(game,&mut ctx,target,amount,keywords)?;
        if ctx.decision_maker.awaiting_choice(){return Ok(AppliedDamageAssignment::default());}
        let mut receipt=commit_prepared_damage_original(game,&mut ctx,prepared)?;
        if ctx.decision_maker.awaiting_choice(){return Ok(AppliedDamageAssignment::default());}
        if receipt.completion.is_some(){
            crate::effects::capture_triggers_before_added_program(game,&ctx,None,
                receipt.original.consequence_outcome.iter_mut().flat_map(|outcome|outcome.events.iter_mut()))?;
        }
        complete_damage_original(game,&mut ctx,receipt)
    })();
    if result.is_err()||ctx.decision_maker.awaiting_choice(){
        game.restore_execution_checkpoint(checkpoint,result.is_ok()&&ctx.decision_maker.awaiting_choice());
        if ctx.decision_maker.awaiting_choice(){return Ok(AppliedDamageAssignment::default());}
    }
    result
}

#[path = "damage_assignment.rs"]
mod damage_assignment;
pub(crate) use damage_assignment::{PreparedDamageAssignment,DamageAssignmentReceipt,prepare_processed_damage_assignment,commit_prepared_damage_original,commit_prepared_damage_original_with_outputs,freeze_damage_original,observe_damage_original,complete_observed_damage_original_with_outputs,complete_damage_original};

fn build_damage_result(
    target: DamageTarget,
    amount: u32,
    has_deathtouch: bool,
    has_infect: bool,
    has_wither: bool,
    has_lifelink: bool,
) -> DamageResult {
    let mut result = DamageResult {
        has_deathtouch,
        has_infect,
        has_wither,
        has_lifelink,
        ..Default::default()
    };

    match target {
        DamageTarget::Permanent => {
            if has_infect || has_wither {
                result.minus_counters = amount;
            } else {
                result.damage_dealt = amount;
            }
        }
        DamageTarget::Player(_) => {
            if has_infect {
                result.poison_counters = amount;
            } else {
                result.damage_dealt = amount;
            }
        }
    }

    if has_lifelink {
        result.life_gained = amount;
    }

    result
}

/// Calculate the result of dealing damage.
///
/// # Arguments
/// * `source` - The object dealing damage
/// * `target` - Whether the target is a permanent or player
/// * `amount` - The amount of damage being dealt
/// * `is_combat` - Whether this is combat damage
///
/// # Returns
/// A `DamageResult` describing all effects of the damage.
pub fn calculate_damage(
    source: &Object,
    target: DamageTarget,
    amount: u32,
    _is_combat: bool,
) -> DamageResult {
    if amount == 0 {
        return DamageResult::default();
    }

    let has_deathtouch = source.has_static_ability_id(StaticAbilityId::Deathtouch);
    let has_infect = source.has_static_ability_id(StaticAbilityId::Infect);
    let has_wither = source.has_static_ability_id(StaticAbilityId::Wither);
    let has_lifelink = source.has_static_ability_id(StaticAbilityId::Lifelink);

    build_damage_result(
        target,
        amount,
        has_deathtouch,
        has_infect,
        has_wither,
        has_lifelink,
    )
}

/// Calculate damage using calculated characteristics (continuous effects included).
pub fn calculate_damage_with_game(
    game: &GameState,
    source: &Object,
    target: DamageTarget,
    amount: u32,
    _is_combat: bool,
) -> DamageResult {
    if amount == 0 {
        return DamageResult::default();
    }

    let has_deathtouch = game.object_has_static_ability_id(source.id, StaticAbilityId::Deathtouch);
    let has_infect = game.object_has_static_ability_id(source.id, StaticAbilityId::Infect);
    let has_wither = game.object_has_static_ability_id(source.id, StaticAbilityId::Wither);
    let has_lifelink = game.object_has_static_ability_id(source.id, StaticAbilityId::Lifelink);

    build_damage_result(
        target,
        amount,
        has_deathtouch,
        has_infect,
        has_wither,
        has_lifelink,
    )
}

/// Check if damage is lethal to a creature.
///
/// Damage is lethal if:
/// - The source has deathtouch and dealt any damage (> 0), OR
/// - The total damage marked (including this damage) >= toughness
///
/// # Arguments
/// * `source` - The object dealing damage
/// * `creature` - The creature receiving damage
/// * `damage` - The amount of damage being dealt
/// * `game` - The game state (for accessing damage_marked)
///
/// # Returns
/// `true` if this damage would be lethal to the creature.
pub fn is_lethal(
    source: &Object,
    creature: &Object,
    damage: u32,
    game: &crate::game_state::GameState,
) -> bool {
    if damage == 0 {
        return false;
    }

    // Deathtouch: any damage is lethal
    if game.object_has_static_ability_id(source.id, StaticAbilityId::Deathtouch) {
        return true;
    }

    // Normal lethal check: damage >= toughness - existing damage
    let Some(toughness) = game
        .calculated_toughness(creature.id)
        .or_else(|| creature.toughness())
    else {
        return false;
    };

    let existing_damage = game.damage_on(creature.id);
    let effective_toughness = (i64::from(toughness) - i64::from(existing_damage)).max(0) as u32;
    damage >= effective_toughness
}

/// Calculate excess damage for trample.
///
/// Trample allows attackers to deal excess damage to the defending player
/// after dealing lethal damage to blockers.
///
/// # Arguments
/// * `attacker` - The attacking creature with trample
/// * `blockers` - The creatures blocking the attacker
/// * `total_damage` - The attacker's power (total damage available)
/// * `game` - The game state (for accessing damage_marked)
///
/// # Returns
/// The amount of excess damage that can trample through to the defending player.
/// Returns 0 if the attacker doesn't have trample.
pub fn calculate_trample_excess(
    attacker: &Object,
    blockers: &[&Object],
    total_damage: u32,
    game: &crate::game_state::GameState,
) -> u32 {
    // Must have trample
    if !game.object_has_static_ability_id(attacker.id, StaticAbilityId::Trample) {
        return 0;
    }

    let has_deathtouch =
        game.object_has_static_ability_id(attacker.id, StaticAbilityId::Deathtouch);

    // Calculate minimum damage needed to kill each blocker
    let mut damage_needed: u32 = 0;

    for blocker in blockers {
        if has_deathtouch {
            // With deathtouch, only need 1 damage to each blocker
            damage_needed += 1;
        } else {
            // Need to deal lethal damage (toughness - existing damage)
            if let Some(toughness) = game
                .calculated_toughness(blocker.id)
                .or_else(|| blocker.toughness())
            {
                let existing_damage = game.damage_on(blocker.id);
                let remaining = (i64::from(toughness) - i64::from(existing_damage)).max(0) as u32;
                damage_needed += remaining;
            }
        }
    }

    // Excess damage tramples through
    total_damage.saturating_sub(damage_needed)
}

/// Calculate damage distribution for trample with multiple blockers.
///
/// Returns a vector of (damage, is_lethal) tuples for each blocker,
/// plus the excess damage that goes to the defending player.
pub fn distribute_trample_damage(
    attacker: &Object,
    blockers: &[&Object],
    total_damage: u32,
    game: &crate::game_state::GameState,
) -> (Vec<(u32, bool)>, u32) {
    if blockers.is_empty() {
        // No blockers, all damage to player
        return (vec![], total_damage);
    }

    let has_deathtouch =
        game.object_has_static_ability_id(attacker.id, StaticAbilityId::Deathtouch);
    let has_trample = game.object_has_static_ability_id(attacker.id, StaticAbilityId::Trample);

    let mut distribution = Vec::with_capacity(blockers.len());
    let mut remaining_damage = total_damage;

    for blocker in blockers {
        let existing_damage = game.damage_on(blocker.id);
        let lethal = if has_deathtouch {
            1
        } else if let Some(threshold) = lethal_damage_threshold_for_creature(game, blocker) {
            (i64::from(threshold) - i64::from(existing_damage)).max(0) as u32
        } else {
            0
        };

        let damage_to_blocker = remaining_damage.min(lethal);
        let is_lethal = damage_to_blocker >= lethal && lethal > 0;

        distribution.push((damage_to_blocker, is_lethal));
        remaining_damage = remaining_damage.saturating_sub(damage_to_blocker);
    }

    if has_trample {
        (distribution, remaining_damage)
    } else {
        (distribution, 0)
    }
}

/// Calculate damage distribution among multiple creature recipients when excess can't trample through.
///
/// This is used for cases like a creature blocking multiple attackers, where the creature assigns
/// all of its combat damage among the creatures it's blocking.
pub fn distribute_combat_damage_to_creatures(
    _source: &Object,
    recipients: &[&Object],
    total_damage: u32,
    _game: &crate::game_state::GameState,
) -> Vec<(u32, bool)> {
    if recipients.is_empty() {
        return vec![];
    }

    recipients
        .iter()
        .enumerate()
        .map(|(index, _)| (if index == 0 { total_damage } else { 0 }, false))
        .collect()
}

pub(crate) fn lethal_damage_threshold_for_creature(
    game: &crate::game_state::GameState,
    creature: &Object,
) -> Option<i32> {
    let creature_controller = game.controller_of(creature);
    let uses_power = game.battlefield.iter().any(|&source_id| {
        game.controller_of_id(source_id) == Some(creature_controller)
            && game.object_has_static_ability_id(
                source_id,
                StaticAbilityId::LethalDamageToCreaturesYouControlUsesPower,
            )
    });

    if uses_power {
        game.calculated_power(creature.id)
            .or_else(|| creature.power())
            .map(|power| power.max(1))
    } else {
        game.calculated_toughness(creature.id)
            .or_else(|| creature.toughness())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::Ability;
    use crate::card::{CardBuilder, PtValue};
    use crate::cost::OptionalCostsPaid;
    use crate::events::EventKind;
    use crate::game_state::GameState;
    use crate::ids::{CardId, ObjectId, StableId};
    use crate::object::CounterType;
    use crate::static_abilities::StaticAbility;
    use crate::types::CardType;
    use crate::zone::Zone;
    use std::collections::HashMap;

    fn test_game_state() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    static NEXT_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

    fn make_creature(name: &str, power: i32, toughness: i32) -> Object {
        let id = ObjectId::from_raw(NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst));
        Object {
            id,
            stable_id: StableId::from(id),
            last_modified: 0,
            kind: crate::object::ObjectKind::Card,
            card: None,
            zone: Zone::Battlefield,
            owner: PlayerId::from_index(0),
            initial_controller: PlayerId::from_index(0),
            name: name.to_string().into(),
            first_printed_set_name: None,
            mana_cost: None,
            color_override: None,
            supertypes: vec![].into(),
            card_types: vec![CardType::Creature].into(),
            subtypes: vec![].into(),
            compiled_card_text: String::new().into(),
            ability_labels: Default::default(),
            rules_text_color_identity: crate::color::ColorSet::COLORLESS,
            other_face: None,
            other_face_name: None,
            linked_face_layout: crate::card::LinkedFaceLayout::None,
            linked_face_mana_cost: None,
            split_combined: None,
            base_power: Some(PtValue::Fixed(power)),
            base_toughness: Some(PtValue::Fixed(toughness)),
            base_loyalty: None,
            base_defense: None,
            hand_modifier: 0,
            life_modifier: 0,
            abilities: std::sync::Arc::new(vec![]),
            counters: crate::object::ObjectCounters::default(),
            attached_to: None,
            attachments: vec![],
            spell_effect: None,
            splice_cast_state: None,
            aura_attach_filter: None,
            alternative_casts: vec![].into(),
            cast_alternative_method: None,
            cast_play_from_constraints: None,
            cast_grant_usage_identity: None,
            cast_price: None,
            cast_play_permission: None,
            has_fuse: false,
            optional_costs: vec![].into(),
            optional_costs_paid: OptionalCostsPaid::default(),
            mana_spent_to_cast: crate::player::ManaPool::default(),
            caster_mana_spent_to_cast: None,
            mana_spent_on_x: None,
            snow_mana_spent_to_cast: crate::player::ManaPool::default(),
            temporary_static_ability_grants: crate::object::TemporaryStaticAbilityGrants::new(id),
            x_value: None,
            keyword_payment_contributions_to_cast: vec![],
            cast_tagged_objects: HashMap::new(),
            additional_cost: crate::cost::TotalCost::free().into(),
            bestow_cast_state: None,
            face_down_cast_state: None,
            prototype_cast_state: None,
            enters_as_copy_restore_state: None,
        }
    }

    fn add_ability(obj: &mut Object, static_ability: StaticAbility) {
        obj.abilities_mut()
            .push(Ability::static_ability(static_ability));
    }

    #[test]
    fn test_normal_damage_to_creature() {
        let source = make_creature("Attacker", 3, 3);
        let result = calculate_damage(&source, DamageTarget::Permanent, 3, true);

        assert_eq!(result.damage_dealt, 3);
        assert_eq!(result.minus_counters, 0);
        assert_eq!(result.poison_counters, 0);
        assert_eq!(result.life_gained, 0);
    }

    #[test]
    fn test_damage_with_lifelink() {
        let mut source = make_creature("Lifelinker", 3, 3);
        add_ability(&mut source, StaticAbility::lifelink());

        let result = calculate_damage(&source, DamageTarget::Permanent, 3, true);

        assert_eq!(result.damage_dealt, 3);
        assert_eq!(result.life_gained, 3);
    }

    #[test]
    fn test_infect_damage_to_creature() {
        let mut source = make_creature("Infector", 3, 3);
        add_ability(&mut source, StaticAbility::infect());

        let result = calculate_damage(&source, DamageTarget::Permanent, 3, true);

        assert_eq!(result.damage_dealt, 0);
        assert_eq!(result.minus_counters, 3);
    }

    #[test]
    fn test_infect_damage_to_player() {
        let mut source = make_creature("Infector", 3, 3);
        add_ability(&mut source, StaticAbility::infect());

        let result = calculate_damage(
            &source,
            DamageTarget::Player(PlayerId::from_index(0)),
            3,
            true,
        );

        assert_eq!(result.damage_dealt, 0);
        assert_eq!(result.poison_counters, 3);
    }

    #[test]
    fn test_apply_infect_damage_to_player_retains_markers_changed_event() {
        let mut game = test_game_state();
        let mut source = make_creature("Infector", 3, 3);
        add_ability(&mut source, StaticAbility::infect());
        let source_id = source.id;
        game.add_object(source);

        let result = apply_processed_damage_assignment(
            &mut game,
            source_id,
            crate::events::DamageTarget::Player(PlayerId::from_index(1)),
            3,
            SourceDamageKeywords {
                has_infect: true,
                ..SourceDamageKeywords::default()
            },
            crate::events::cause::EventCause::from_effect(source_id, PlayerId::from_index(0)),
        );

        assert!(result.applied);
        assert_eq!(result.life_lost, 0);
        assert_eq!(
            game.player(PlayerId::from_index(1))
                .expect("player exists")
                .poison_counters,
            3
        );
        assert!(
            result.consequence_outcome.as_ref().unwrap().events
                .iter()
                .any(|event| event.kind() == EventKind::MarkersChanged),
            "infect damage must retain MarkersChangedEvent for its caller"
        );
        assert!(game.take_pending_trigger_events().is_empty());
    }

    #[test]
    fn damage_to_battle_removes_defense_counters_and_queues_event() {
        let mut game = test_game_state();
        let alice = PlayerId::from_index(0);
        let source_id = game.create_object_from_card(
            &CardBuilder::new(CardId::new(), "Attacker")
                .card_types(vec![CardType::Creature])
                .power_toughness(crate::card::PowerToughness::fixed(2, 2))
                .build(),
            alice,
            Zone::Battlefield,
        );
        let battle_id = game.create_object_from_card(
            &CardBuilder::new(CardId::new(), "Battle")
                .card_types(vec![CardType::Battle])
                .defense(5)
                .build(),
            alice,
            Zone::Battlefield,
        );

        let result = apply_processed_damage_assignment(
            &mut game,
            source_id,
            crate::events::DamageTarget::Object(battle_id),
            2,
            SourceDamageKeywords::default(),
            crate::events::cause::EventCause::from_effect(source_id, alice),
        );

        assert!(result.applied);
        assert_eq!(game.counter_count(battle_id, CounterType::Defense), 3);
        assert!(game.take_pending_trigger_events().iter().any(|event| {
            event
                .downcast::<crate::events::MarkersChangedEvent>()
                .is_some_and(|changed| {
                    changed.change_type == crate::events::MarkerChangeType::Removed
                        && changed.marker.as_counter() == Some(CounterType::Defense)
                        && changed.location.as_object() == Some(battle_id)
                        && changed.amount == 2
                })
        }));
    }

    #[test]
    fn test_damage_applies_to_land_made_creature_by_continuous_effect() {
        let mut game = test_game_state();
        let source = make_creature("Pinger", 1, 1);
        let source_id = source.id;
        game.add_object(source);

        let mut land = make_creature("Animated Land", 0, 3);
        land.card_types = vec![CardType::Land].into();
        let land_id = land.id;
        game.add_object(land);
        game.effect_store.continuous_effects.add_effect(
            crate::continuous::ContinuousEffect::new(
                land_id,
                PlayerId::from_index(0),
                crate::continuous::EffectTarget::Specific(land_id),
                crate::continuous::Modification::AddCardTypes(vec![CardType::Creature]),
            )
            .until(crate::effect::Until::EndOfTurn),
        );

        let result = apply_processed_damage_assignment(
            &mut game,
            source_id,
            crate::events::DamageTarget::Object(land_id),
            2,
            SourceDamageKeywords::default(),
            crate::events::cause::EventCause::from_effect(source_id, PlayerId::from_index(0)),
        );

        assert!(result.applied);
        assert_eq!(game.damage_on(land_id), 2);
    }

    #[test]
    fn test_damage_to_planeswalker_removes_loyalty_counters() {
        let mut game = test_game_state();
        let source = make_creature("Pinger", 1, 1);
        let source_id = source.id;
        game.add_object(source);

        let mut planeswalker = make_creature("Test Planeswalker", 0, 0);
        planeswalker.card_types = vec![CardType::Planeswalker].into();
        planeswalker.base_power = None;
        planeswalker.base_toughness = None;
        planeswalker.base_loyalty = Some(5);
        planeswalker.counters.insert(crate::CounterType::Loyalty, 5);
        let planeswalker_id = planeswalker.id;
        game.add_object(planeswalker);

        let result = apply_processed_damage_assignment(
            &mut game,
            source_id,
            crate::events::DamageTarget::Object(planeswalker_id),
            3,
            SourceDamageKeywords::default(),
            crate::events::cause::EventCause::from_effect(source_id, PlayerId::from_index(0)),
        );

        assert!(result.applied);
        assert_eq!(
            game.counter_count(planeswalker_id, crate::CounterType::Loyalty),
            2
        );
        assert_eq!(
            game.damage_on(planeswalker_id),
            0,
            "planeswalker damage should not be marked as creature damage"
        );
    }

    #[test]
    fn test_infect_damage_to_land_made_creature_uses_minus_one_minus_one_counters() {
        let mut game = test_game_state();
        let source = make_creature("Infector", 1, 1);
        let source_id = source.id;
        game.add_object(source);

        let mut land = make_creature("Animated Land", 0, 3);
        land.card_types = vec![CardType::Land].into();
        let land_id = land.id;
        game.add_object(land);
        game.effect_store.continuous_effects.add_effect(
            crate::continuous::ContinuousEffect::new(
                land_id,
                PlayerId::from_index(0),
                crate::continuous::EffectTarget::Specific(land_id),
                crate::continuous::Modification::AddCardTypes(vec![CardType::Creature]),
            )
            .until(crate::effect::Until::EndOfTurn),
        );

        let result = apply_processed_damage_assignment(
            &mut game,
            source_id,
            crate::events::DamageTarget::Object(land_id),
            2,
            SourceDamageKeywords {
                has_infect: true,
                ..SourceDamageKeywords::default()
            },
            crate::events::cause::EventCause::from_effect(source_id, PlayerId::from_index(0)),
        );

        assert!(result.applied);
        assert_eq!(game.damage_on(land_id), 0);
        assert_eq!(
            game.counter_count(land_id, crate::object::CounterType::MinusOneMinusOne),
            2
        );
    }

    #[test]
    fn test_wither_damage_to_creature() {
        let mut source = make_creature("Witherer", 3, 3);
        add_ability(&mut source, StaticAbility::wither());

        let result = calculate_damage(&source, DamageTarget::Permanent, 3, true);

        assert_eq!(result.damage_dealt, 0);
        assert_eq!(result.minus_counters, 3);
    }

    #[test]
    fn test_wither_damage_to_player() {
        let mut source = make_creature("Witherer", 3, 3);
        add_ability(&mut source, StaticAbility::wither());

        // Wither only affects creatures, normal damage to players
        let result = calculate_damage(
            &source,
            DamageTarget::Player(PlayerId::from_index(0)),
            3,
            true,
        );

        assert_eq!(result.damage_dealt, 3);
        assert_eq!(result.poison_counters, 0);
    }

    #[test]
    fn test_infect_with_lifelink() {
        let mut source = make_creature("Infect Lifelink", 3, 3);
        add_ability(&mut source, StaticAbility::infect());
        add_ability(&mut source, StaticAbility::lifelink());

        // Infect to player
        let result = calculate_damage(
            &source,
            DamageTarget::Player(PlayerId::from_index(0)),
            3,
            true,
        );
        assert_eq!(result.poison_counters, 3);
        assert_eq!(result.life_gained, 3); // Lifelink still works

        // Infect to creature
        let result = calculate_damage(&source, DamageTarget::Permanent, 3, true);
        assert_eq!(result.minus_counters, 3);
        assert_eq!(result.life_gained, 3);
    }

    #[test]
    fn test_deathtouch_is_lethal() {
        let mut game = test_game_state();
        let mut source = make_creature("Deathtoucher", 1, 1);
        add_ability(&mut source, StaticAbility::deathtouch());

        let creature = make_creature("Big", 5, 5);

        // Add objects to game so ability lookups work
        game.add_object(source.clone());
        game.add_object(creature.clone());

        // 1 damage with deathtouch is lethal
        assert!(is_lethal(&source, &creature, 1, &game));
    }

    #[test]
    fn test_normal_lethal_damage() {
        let mut game = test_game_state();
        let source = make_creature("Normal", 3, 3);

        let creature = make_creature("Target", 4, 4);

        // 3 damage to 4 toughness is not lethal
        assert!(!is_lethal(&source, &creature, 3, &game));

        // 4 damage to 4 toughness is lethal
        assert!(is_lethal(&source, &creature, 4, &game));

        // With existing damage, less is needed
        game.mark_damage(creature.id, 2);
        assert!(is_lethal(&source, &creature, 2, &game));
    }

    #[test]
    fn test_zero_damage_not_lethal() {
        let game = test_game_state();
        let mut source = make_creature("Deathtoucher", 1, 1);
        add_ability(&mut source, StaticAbility::deathtouch());

        let creature = make_creature("Target", 4, 4);

        // 0 damage is never lethal, even with deathtouch
        assert!(!is_lethal(&source, &creature, 0, &game));
    }

    #[test]
    fn test_trample_excess_damage() {
        let mut game = test_game_state();
        let mut attacker = make_creature("Trampler", 6, 6);
        add_ability(&mut attacker, StaticAbility::trample());

        let blocker = make_creature("Small", 2, 2);

        game.add_object(attacker.clone());
        game.add_object(blocker.clone());

        // 6 power - 2 toughness = 4 excess
        let excess = calculate_trample_excess(&attacker, &[&blocker], 6, &game);
        assert_eq!(excess, 4);
    }

    #[test]
    fn test_trample_multiple_blockers() {
        let mut game = test_game_state();
        let mut attacker = make_creature("Trampler", 7, 7);
        add_ability(&mut attacker, StaticAbility::trample());

        let blocker1 = make_creature("Small1", 2, 2);
        let blocker2 = make_creature("Small2", 3, 3);

        game.add_object(attacker.clone());
        game.add_object(blocker1.clone());
        game.add_object(blocker2.clone());

        // 7 power - (2 + 3) toughness = 2 excess
        let excess = calculate_trample_excess(&attacker, &[&blocker1, &blocker2], 7, &game);
        assert_eq!(excess, 2);
    }

    #[test]
    fn test_trample_with_deathtouch() {
        let mut game = test_game_state();
        let mut attacker = make_creature("Deathtouch Trampler", 6, 6);
        add_ability(&mut attacker, StaticAbility::trample());
        add_ability(&mut attacker, StaticAbility::deathtouch());

        let blocker1 = make_creature("Big1", 5, 5);
        let blocker2 = make_creature("Big2", 5, 5);

        game.add_object(attacker.clone());
        game.add_object(blocker1.clone());
        game.add_object(blocker2.clone());

        // With deathtouch, only need 1 damage to each blocker
        // 6 power - (1 + 1) = 4 excess
        let excess = calculate_trample_excess(&attacker, &[&blocker1, &blocker2], 6, &game);
        assert_eq!(excess, 4);
    }

    #[test]
    fn test_no_trample_no_excess() {
        let game = test_game_state();
        let attacker = make_creature("Normal", 6, 6);
        let blocker = make_creature("Small", 2, 2);

        // Without trample, no excess damage
        let excess = calculate_trample_excess(&attacker, &[&blocker], 6, &game);
        assert_eq!(excess, 0);
    }

    #[test]
    fn test_distribute_trample_damage() {
        let mut game = test_game_state();
        let mut attacker = make_creature("Trampler", 5, 5);
        add_ability(&mut attacker, StaticAbility::trample());

        let blocker1 = make_creature("Small1", 2, 2);
        let blocker2 = make_creature("Small2", 2, 2);

        game.add_object(attacker.clone());
        game.add_object(blocker1.clone());
        game.add_object(blocker2.clone());

        let (distribution, excess) =
            distribute_trample_damage(&attacker, &[&blocker1, &blocker2], 5, &game);

        assert_eq!(distribution.len(), 2);
        assert_eq!(distribution[0], (2, true)); // 2 damage to blocker1, lethal
        assert_eq!(distribution[1], (2, true)); // 2 damage to blocker2, lethal
        assert_eq!(excess, 1); // 1 damage tramples through
    }

    #[test]
    fn test_distribute_damage_no_trample() {
        let game = test_game_state();
        let attacker = make_creature("Normal", 5, 5);
        let blocker = make_creature("Small", 2, 2);

        let (distribution, excess) = distribute_trample_damage(&attacker, &[&blocker], 5, &game);

        assert_eq!(distribution.len(), 1);
        assert_eq!(distribution[0], (2, true)); // Only lethal damage assigned
        assert_eq!(excess, 0); // No trample, no excess
    }

    #[test]
    fn test_existing_damage_affects_lethal() {
        let mut game = test_game_state();
        let source = make_creature("Attacker", 2, 2);

        let creature = make_creature("Damaged", 4, 4);
        game.mark_damage(creature.id, 2);

        // 2 damage to a 4 toughness creature with 2 damage already is lethal
        assert!(is_lethal(&source, &creature, 2, &game));

        // 1 damage is not quite lethal
        assert!(!is_lethal(&source, &creature, 1, &game));
    }
}
