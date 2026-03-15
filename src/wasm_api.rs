//! WASM-facing API for browser integration.
//!
//! This module provides a small wrapper around `GameState` so JavaScript can:
//! - create/reset a game
//! - mutate a bit of state
//! - read a serializable snapshot

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::OnceLock;

use rand::seq::SliceRandom;
use rand::{SeedableRng, rngs::StdRng};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

use crate::cards::{CardDefinition, CardRegistry};
use crate::combat_state::AttackTarget;
use crate::decision::{
    AttackerDeclaration, BlockerDeclaration, DecisionMaker, GameProgress, GameResult, LegalAction,
};
use crate::decisions::context::DecisionContext;
use crate::game_loop::{
    ActivationStage, CastStage, PendingPriorityContinuation, PriorityLoopState, PriorityResponse,
    advance_priority_with_dm, apply_decision_context_with_dm, apply_priority_response_with_dm,
};
use crate::game_state::{GameState, Target};
use crate::ids::{
    ObjectId, PlayerId, reset_runtime_id_counters, restore_id_counters, snapshot_id_counters,
};
use crate::mana::ManaSymbol;
use crate::targeting::{normalize_targets_for_requirements, validate_flat_target_assignment};
use crate::triggers::TriggerQueue;
use crate::types::{CardType, Subtype};
use crate::zone::Zone;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum BattlefieldLane {
    Lands,
    Creatures,
    Planeswalkers,
    Battles,
    Artifacts,
    Enchantments,
    Other,
}

impl BattlefieldLane {
    fn as_str(self) -> &'static str {
        match self {
            BattlefieldLane::Lands => "lands",
            BattlefieldLane::Creatures => "creatures",
            BattlefieldLane::Planeswalkers => "planeswalkers",
            BattlefieldLane::Battles => "battles",
            BattlefieldLane::Artifacts => "artifacts",
            BattlefieldLane::Enchantments => "enchantments",
            BattlefieldLane::Other => "other",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct BattlefieldGroupKey {
    lane: BattlefieldLane,
    name: String,
    tapped: bool,
    counter_signature: String,
    power_toughness_signature: String,
    token: bool,
    force_single_object: Option<u64>,
}

fn battlefield_lane_for_object(obj: &crate::object::Object) -> BattlefieldLane {
    if obj.has_card_type(CardType::Land) {
        return BattlefieldLane::Lands;
    }
    if obj.has_card_type(CardType::Creature) {
        return BattlefieldLane::Creatures;
    }
    if obj.has_card_type(CardType::Planeswalker) {
        return BattlefieldLane::Planeswalkers;
    }
    if obj.has_card_type(CardType::Battle) {
        return BattlefieldLane::Battles;
    }
    if obj.has_card_type(CardType::Artifact) {
        return BattlefieldLane::Artifacts;
    }
    if obj.has_card_type(CardType::Enchantment) {
        return BattlefieldLane::Enchantments;
    }
    BattlefieldLane::Other
}

fn counter_signature_for_group(obj: &crate::object::Object) -> String {
    let mut parts: Vec<(String, u32)> = obj
        .counters
        .iter()
        .map(|(counter_type, amount)| (counter_type.description().into_owned(), *amount))
        .collect();
    parts.sort_unstable_by(|left, right| left.0.cmp(&right.0));
    if parts.is_empty() {
        return "-".to_string();
    }
    parts
        .into_iter()
        .map(|(kind, amount)| format!("{kind}:{amount}"))
        .collect::<Vec<_>>()
        .join("|")
}

fn power_toughness_signature_for_group(obj: &crate::object::Object) -> String {
    match (obj.power(), obj.toughness()) {
        (Some(power), Some(toughness)) => format!("{power}/{toughness}"),
        _ => "-".to_string(),
    }
}

fn counter_snapshots_for_object(obj: &crate::object::Object) -> Vec<CounterSnapshot> {
    let mut counters: Vec<CounterSnapshot> = obj
        .counters
        .iter()
        .map(|(kind, amount)| CounterSnapshot {
            kind: kind.description().into_owned(),
            amount: *amount,
        })
        .collect();
    counters.sort_unstable_by(|left, right| left.kind.cmp(&right.kind));
    counters
}

fn protected_object_ids_for_decision(decision: Option<&DecisionContext>) -> HashSet<ObjectId> {
    let mut ids = HashSet::new();
    let Some(decision) = decision else {
        return ids;
    };

    match decision {
        // Priority: don't force-ungroup — identical permanents should stay stacked.
        // The UI picks actions by index, not by specific object ID.
        DecisionContext::Priority(_) => {}
        DecisionContext::Targets(targets) => {
            for requirement in &targets.requirements {
                for target in &requirement.legal_targets {
                    if let Target::Object(object_id) = target {
                        ids.insert(*object_id);
                    }
                }
            }
        }
        DecisionContext::SelectObjects(objects) => {
            for candidate in &objects.candidates {
                if candidate.legal {
                    ids.insert(candidate.id);
                }
            }
        }
        DecisionContext::Attackers(attackers) => {
            for option in &attackers.attacker_options {
                ids.insert(option.creature);
                for target in &option.valid_targets {
                    if let AttackTarget::Planeswalker(object_id) = target {
                        ids.insert(*object_id);
                    }
                }
            }
        }
        DecisionContext::Blockers(blockers) => {
            for option in &blockers.blocker_options {
                ids.insert(option.attacker);
                for (blocker, _) in &option.valid_blockers {
                    ids.insert(*blocker);
                }
            }
        }
        DecisionContext::Modes(_)
        | DecisionContext::HybridChoice(_)
        | DecisionContext::SelectOptions(_)
        | DecisionContext::Boolean(_)
        | DecisionContext::Number(_)
        | DecisionContext::Order(_)
        | DecisionContext::Distribute(_)
        | DecisionContext::Colors(_)
        | DecisionContext::Counters(_)
        | DecisionContext::Partition(_)
        | DecisionContext::Proliferate(_) => {}
    }

    ids
}

fn grouped_battlefield_for_player(
    game: &GameState,
    player: PlayerId,
    protected_ids: &HashSet<ObjectId>,
) -> (Vec<PermanentSnapshot>, usize) {
    let mut grouped: HashMap<BattlefieldGroupKey, Vec<&crate::object::Object>> = HashMap::new();
    let mut total = 0usize;

    for object_id in &game.battlefield {
        let Some(obj) = game.object(*object_id) else {
            continue;
        };
        if obj.controller != player {
            continue;
        }
        total += 1;

        let force_single = protected_ids.contains(&obj.id).then_some(obj.id.0);
        let key = BattlefieldGroupKey {
            lane: battlefield_lane_for_object(obj),
            name: obj.name.clone(),
            tapped: game.is_tapped(obj.id),
            counter_signature: counter_signature_for_group(obj),
            power_toughness_signature: power_toughness_signature_for_group(obj),
            token: matches!(obj.kind, crate::object::ObjectKind::Token),
            force_single_object: force_single,
        };
        grouped.entry(key).or_default().push(obj);
    }

    let mut groups: Vec<(BattlefieldGroupKey, Vec<&crate::object::Object>)> =
        grouped.into_iter().collect();
    groups.sort_unstable_by(|(left_key, left_members), (right_key, right_members)| {
        left_key
            .lane
            .cmp(&right_key.lane)
            .then_with(|| left_key.name.cmp(&right_key.name))
            .then_with(|| left_key.tapped.cmp(&right_key.tapped))
            .then_with(|| left_key.token.cmp(&right_key.token))
            .then_with(|| {
                left_members
                    .first()
                    .map(|obj| obj.id.0)
                    .cmp(&right_members.first().map(|obj| obj.id.0))
            })
    });

    let snapshots = groups
        .into_iter()
        .map(|(key, mut members)| {
            members.sort_unstable_by_key(|obj| obj.id.0);
            let representative = members.first().copied();
            let member_ids: Vec<u64> = members.iter().map(|obj| obj.id.0).collect();
            let member_stable_ids: Vec<u64> = members.iter().map(|obj| obj.stable_id.0.0).collect();
            let id = representative.map(|obj| obj.id.0).unwrap_or_default();
            let stable_id = representative
                .map(|obj| obj.stable_id.0.0)
                .unwrap_or_default();
            let name = representative
                .map(|obj| obj.name.clone())
                .unwrap_or_else(|| key.name.clone());
            let power_toughness = representative.and_then(|obj| {
                let p = game.calculated_power(obj.id).or_else(|| obj.power())?;
                let t = game
                    .calculated_toughness(obj.id)
                    .or_else(|| obj.toughness())?;
                Some(format!("{p}/{t}"))
            });
            let mana_cost =
                representative.and_then(|obj| obj.mana_cost.as_ref().map(|mc| mc.to_oracle()));
            let counters = representative
                .map(counter_snapshots_for_object)
                .unwrap_or_default();
            PermanentSnapshot {
                id,
                stable_id,
                name,
                token: key.token,
                tapped: key.tapped,
                count: member_ids.len().max(1),
                member_ids,
                member_stable_ids,
                lane: key.lane.as_str().to_string(),
                mana_cost,
                power_toughness,
                counter_signature: key.counter_signature.clone(),
                counters,
            }
        })
        .collect();

    (snapshots, total)
}

fn should_surface_zone_card_in_pseudo_hand(
    game: &GameState,
    perspective: PlayerId,
    object: &crate::object::Object,
    zone: Zone,
) -> bool {
    if object.zone != zone
        || matches!(
            zone,
            Zone::Hand | Zone::Library | Zone::Battlefield | Zone::Stack
        )
    {
        return false;
    }

    if zone == Zone::Command && object.owner == perspective && game.is_commander(object.id) {
        return true;
    }

    if !game
        .grant_registry
        .granted_play_from_for_card(game, object.id, zone, perspective)
        .is_empty()
    {
        return true;
    }

    if !game
        .grant_registry
        .granted_alternative_casts_for_card(game, object.id, zone, perspective)
        .is_empty()
    {
        return true;
    }

    object
        .alternative_casts
        .iter()
        .any(|method| method.cast_from_zone() == zone)
}

#[derive(Debug, Clone, Serialize)]
struct PermanentSnapshot {
    id: u64,
    stable_id: u64,
    name: String,
    token: bool,
    tapped: bool,
    count: usize,
    member_ids: Vec<u64>,
    member_stable_ids: Vec<u64>,
    lane: String,
    mana_cost: Option<String>,
    power_toughness: Option<String>,
    counter_signature: String,
    counters: Vec<CounterSnapshot>,
}

#[derive(Debug, Clone, Serialize)]
struct ManaPoolSnapshot {
    white: u32,
    blue: u32,
    black: u32,
    red: u32,
    green: u32,
    colorless: u32,
}

#[derive(Debug, Clone, Serialize)]
struct PlayerSnapshot {
    id: u8,
    name: String,
    life: i32,
    mana_pool: ManaPoolSnapshot,
    can_view_hand: bool,
    hand_size: usize,
    library_size: usize,
    graveyard_size: usize,
    command_size: usize,
    hand_cards: Vec<HandCardSnapshot>,
    graveyard_cards: Vec<ZoneCardSnapshot>,
    exile_cards: Vec<ZoneCardSnapshot>,
    command_cards: Vec<ZoneCardSnapshot>,
    library_top: Option<String>,
    graveyard_top: Option<String>,
    battlefield: Vec<PermanentSnapshot>,
    battlefield_total: usize,
}

#[derive(Debug, Clone, Serialize)]
struct ViewedCardsSnapshot {
    viewer: u8,
    subject: u8,
    zone: String,
    visibility: String,
    cards: Vec<ViewedCardSnapshot>,
    card_ids: Vec<u64>,
    source: Option<u64>,
    description: String,
}

#[derive(Debug, Clone, Serialize)]
struct ViewedCardSnapshot {
    id: u64,
    name: String,
}

#[derive(Debug, Clone, Serialize)]
struct HandCardSnapshot {
    id: u64,
    stable_id: u64,
    name: String,
    mana_cost: Option<String>,
    power_toughness: Option<String>,
    loyalty: Option<u32>,
    defense: Option<u32>,
    card_types: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct ZoneCardSnapshot {
    id: u64,
    stable_id: u64,
    name: String,
    show_in_pseudo_hand: bool,
}

#[derive(Debug, Clone, Serialize)]
struct StackObjectSnapshot {
    id: u64,
    inspect_object_id: Option<u64>,
    stable_id: Option<u64>,
    source_stable_id: Option<u64>,
    controller: u8,
    name: String,
    mana_cost: Option<String>,
    effect_text: Option<String>,
    /// "Triggered", "Activated", or null for spells.
    ability_kind: Option<String>,
    /// Compiled text of the specific ability effects (for inspector display).
    ability_text: Option<String>,
    targets: Vec<TargetChoiceView>,
}

fn build_stack_object_snapshot(
    game: &GameState,
    perspective: PlayerId,
    viewed_cards: Option<&ActiveViewedCards>,
    entry: &crate::game_state::StackEntry,
) -> StackObjectSnapshot {
    let obj = game.object(entry.object_id);
    let source_obj = entry
        .source_stable_id
        .and_then(|stable_id| game.find_object_by_stable_id(stable_id))
        .and_then(|id| game.object(id));
    let source_stable_id = entry.source_stable_id.map(|stable_id| stable_id.0.0);
    let inspect_object_id = if entry.is_ability {
        source_obj.or(obj).map(|object| object.id.0)
    } else {
        obj.or(source_obj).map(|object| object.id.0)
    };
    let stable_id = obj.or(source_obj).map(|o| o.stable_id.0.0);
    let name = obj
        .map(|o| o.name.clone())
        .or_else(|| source_obj.map(|o| o.name.clone()))
        .or_else(|| entry.source_name.clone())
        .unwrap_or_else(|| format!("Object#{}", entry.object_id.0));
    let targets = entry
        .targets
        .iter()
        .enumerate()
        .map(|(index, target)| target_choice_view(game, perspective, viewed_cards, index, target))
        .collect();

    if entry.is_ability {
        let ability_kind = if entry.triggering_event.is_some() {
            "Triggered"
        } else {
            "Activated"
        };
        let ability_text = stack_entry_ability_text(entry, obj);
        StackObjectSnapshot {
            id: entry.object_id.0,
            inspect_object_id,
            stable_id,
            source_stable_id,
            controller: entry.controller.0,
            name,
            mana_cost: None,
            effect_text: None,
            ability_kind: Some(ability_kind.to_string()),
            ability_text,
            targets,
        }
    } else {
        let effect_text = if let Some(o) = obj.or(source_obj) {
            let lines = crate::compiled_text::compiled_lines(&o.to_card_definition());
            if lines.is_empty() {
                None
            } else {
                Some(lines.join("; "))
            }
        } else {
            None
        };
        StackObjectSnapshot {
            id: entry.object_id.0,
            inspect_object_id,
            stable_id,
            source_stable_id,
            controller: entry.controller.0,
            name,
            mana_cost: obj
                .or(source_obj)
                .and_then(|o| o.mana_cost.as_ref().map(|mc| mc.to_oracle())),
            effect_text,
            ability_kind: None,
            ability_text: None,
            targets,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
struct CounterSnapshot {
    kind: String,
    amount: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
enum BattlefieldTransitionKindSnapshot {
    Damaged,
    Destroyed,
    Sacrificed,
    Exiled,
}

#[derive(Debug, Clone, Serialize)]
struct BattlefieldTransitionSnapshot {
    stable_id: u64,
    kind: BattlefieldTransitionKindSnapshot,
}

#[derive(Debug, Clone, Serialize)]
struct ObjectDetailsSnapshot {
    id: u64,
    stable_id: u64,
    name: String,
    kind: String,
    zone: String,
    owner: u8,
    controller: u8,
    type_line: String,
    mana_cost: Option<String>,
    oracle_text: String,
    power: Option<i32>,
    toughness: Option<i32>,
    loyalty: Option<u32>,
    tapped: bool,
    counters: Vec<CounterSnapshot>,
    abilities: Vec<String>,
    raw_compilation: String,
    semantic_score: Option<f32>,
}

#[derive(Debug, Clone, Serialize)]
struct GameSnapshot {
    snapshot_id: u64,
    perspective: u8,
    turn_number: u32,
    active_player: u8,
    priority_player: Option<u8>,
    phase: String,
    step: Option<String>,
    stack_size: usize,
    stack_preview: Vec<String>,
    stack_objects: Vec<StackObjectSnapshot>,
    resolving_stack_object: Option<StackObjectSnapshot>,
    battlefield_size: usize,
    exile_size: usize,
    players: Vec<PlayerSnapshot>,
    battlefield_transitions: Vec<BattlefieldTransitionSnapshot>,
    viewed_cards: Option<ViewedCardsSnapshot>,
    decision: Option<DecisionView>,
    mana_payment: Option<ManaPaymentView>,
    game_over: Option<GameOverView>,
    /// True when the current decision chain can be cancelled (user-initiated
    /// action like casting a spell, NOT triggered ability resolution).
    cancelable: bool,
    /// Stable id of the most recent reversible land-for-mana tap in the current
    /// priority epoch. Only surfaced while the perspective player is back on
    /// priority and the tap can still be undone.
    undo_land_stable_id: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
struct ManaPaymentView {
    source_name: String,
    pips: Vec<Vec<String>>,
    current_pip_index: usize,
}

#[derive(Debug, Clone)]
struct ActiveViewedCards {
    viewer: PlayerId,
    subject: PlayerId,
    zone: Zone,
    cards: Vec<ObjectId>,
    public: bool,
    source: Option<ObjectId>,
    description: String,
}

fn mana_symbol_display_code(symbol: &ManaSymbol) -> String {
    match symbol {
        ManaSymbol::White => "W".to_string(),
        ManaSymbol::Blue => "U".to_string(),
        ManaSymbol::Black => "B".to_string(),
        ManaSymbol::Red => "R".to_string(),
        ManaSymbol::Green => "G".to_string(),
        ManaSymbol::Colorless => "C".to_string(),
        ManaSymbol::Generic(n) => n.to_string(),
        ManaSymbol::Snow => "S".to_string(),
        ManaSymbol::Life(_) => "P".to_string(),
        ManaSymbol::X => "X".to_string(),
    }
}

fn mana_payment_view_from_pending_cast(
    game: &GameState,
    pending: &crate::game_loop::PendingCast,
) -> Option<ManaPaymentView> {
    if !matches!(pending.stage, CastStage::PayingMana) {
        return None;
    }

    let pips = if !pending.display_mana_pips.is_empty() {
        pending.display_mana_pips.clone()
    } else if let Some(cost) = pending.mana_cost_to_pay.as_ref() {
        crate::game_loop::expand_mana_cost_to_display_pips(
            cost,
            pending.x_value.unwrap_or(0) as usize,
        )
    } else {
        Vec::new()
    };

    if pips.is_empty() {
        return None;
    }

    let current_pip_index = pips.len().saturating_sub(pending.remaining_mana_pips.len());
    let source_name = game
        .object(pending.spell_id)
        .map(|obj| obj.name.clone())
        .unwrap_or_else(|| "spell".to_string());

    Some(ManaPaymentView {
        source_name,
        pips: pips
            .into_iter()
            .map(|pip| pip.iter().map(mana_symbol_display_code).collect())
            .collect(),
        current_pip_index,
    })
}

fn mana_payment_view_from_pending_activation(
    pending: &crate::game_loop::PendingActivation,
) -> Option<ManaPaymentView> {
    if !matches!(pending.stage, ActivationStage::PayingMana) {
        return None;
    }

    let pips = if !pending.display_mana_pips.is_empty() {
        pending.display_mana_pips.clone()
    } else if let Some(cost) = pending.mana_cost_to_pay.as_ref() {
        crate::game_loop::expand_mana_cost_to_display_pips(cost, pending.x_value.unwrap_or(0))
    } else {
        Vec::new()
    };

    if pips.is_empty() {
        return None;
    }

    let current_pip_index = pips.len().saturating_sub(pending.remaining_mana_pips.len());

    Some(ManaPaymentView {
        source_name: pending.source_name.clone(),
        pips: pips
            .into_iter()
            .map(|pip| pip.iter().map(mana_symbol_display_code).collect())
            .collect(),
        current_pip_index,
    })
}

fn merge_active_viewed_cards(
    current: &mut Option<ActiveViewedCards>,
    viewer: PlayerId,
    cards: &[ObjectId],
    ctx: &crate::decisions::context::ViewCardsContext,
) {
    let can_merge = current.as_ref().is_some_and(|existing| {
        existing.public == ctx.public
            && existing.zone == ctx.zone
            && existing.source == ctx.source
            && existing.description == ctx.description
            && if existing.zone == Zone::Hand {
                existing.subject == ctx.subject
            } else if ctx.public {
                true
            } else {
                existing.viewer == viewer && existing.subject == ctx.subject
            }
    });

    if can_merge {
        if let Some(existing) = current.as_mut() {
            for &card in cards {
                if !existing.cards.contains(&card) {
                    existing.cards.push(card);
                }
            }
        }
        return;
    }

    *current = Some(ActiveViewedCards {
        viewer,
        subject: ctx.subject,
        zone: ctx.zone,
        cards: cards.to_vec(),
        public: ctx.public,
        source: ctx.source,
        description: ctx.description.clone(),
    });
}

fn normalize_stack_display_text(text: &str) -> Option<String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn is_trigger_display_line(line: &str) -> bool {
    let normalized = line.trim().to_ascii_lowercase();
    normalized.starts_with("when ")
        || normalized.starts_with("whenever ")
        || normalized.starts_with("at the beginning ")
}

fn is_activated_display_line(line: &str) -> bool {
    line.contains(':')
}

fn first_matching_stack_line(lines: &[String], wants_triggered: bool) -> Option<String> {
    let matcher = if wants_triggered {
        is_trigger_display_line as fn(&str) -> bool
    } else {
        is_activated_display_line as fn(&str) -> bool
    };
    lines
        .iter()
        .find(|line| matcher(line))
        .and_then(|line| normalize_stack_display_text(line))
}

fn fallback_stack_entry_ability_text(
    entry: &crate::game_state::StackEntry,
    obj: Option<&crate::object::Object>,
) -> Option<String> {
    let wants_triggered = entry.triggering_event.is_some();

    if let Some(source_obj) = obj {
        let ability_texts: Vec<String> = source_obj
            .abilities
            .iter()
            .filter_map(|ability| match (&ability.kind, wants_triggered) {
                (crate::ability::AbilityKind::Triggered(_), true) => ability.text.clone(),
                (crate::ability::AbilityKind::Activated(_), false) => ability.text.clone(),
                _ => None,
            })
            .collect();
        if let Some(text) = first_matching_stack_line(&ability_texts, wants_triggered) {
            return Some(text);
        }

        let oracle_lines: Vec<String> = source_obj
            .oracle_text
            .lines()
            .filter_map(normalize_stack_display_text)
            .collect();
        if let Some(text) = first_matching_stack_line(&oracle_lines, wants_triggered) {
            return Some(text);
        }

        let compiled_lines = crate::compiled_text::compiled_lines(&source_obj.to_card_definition());
        if let Some(text) = first_matching_stack_line(&compiled_lines, wants_triggered) {
            return Some(text);
        }
    }

    let snapshot_ability_texts: Vec<String> = entry
        .source_snapshot
        .as_ref()
        .into_iter()
        .flat_map(|snapshot| snapshot.abilities.iter())
        .filter_map(|ability| match (&ability.kind, wants_triggered) {
            (crate::ability::AbilityKind::Triggered(_), true) => ability.text.clone(),
            (crate::ability::AbilityKind::Activated(_), false) => ability.text.clone(),
            _ => None,
        })
        .collect();
    first_matching_stack_line(&snapshot_ability_texts, wants_triggered)
}

fn stack_entry_ability_text(
    entry: &crate::game_state::StackEntry,
    obj: Option<&crate::object::Object>,
) -> Option<String> {
    entry
        .ability_effects
        .as_ref()
        .map(|effects| crate::compiled_text::compile_effect_list(effects))
        .and_then(|text| normalize_stack_display_text(&text))
        .or_else(|| fallback_stack_entry_ability_text(entry, obj))
}

impl GameSnapshot {
    fn from_game(
        game: &GameState,
        perspective: PlayerId,
        decision: Option<&DecisionContext>,
        mana_payment: Option<ManaPaymentView>,
        game_over: Option<&GameResult>,
        pending_cast_stack_id: Option<ObjectId>,
        resolving_stack_object: Option<StackObjectSnapshot>,
        battlefield_transitions: Vec<BattlefieldTransitionSnapshot>,
        viewed_cards: Option<&ActiveViewedCards>,
        cancelable: bool,
        undo_land_stable_id: Option<u64>,
        snapshot_id: u64,
    ) -> Self {
        let protected_ids = protected_object_ids_for_decision(decision);
        let players = game
            .players
            .iter()
            .map(|p| {
                let (battlefield, battlefield_total) =
                    grouped_battlefield_for_player(game, p.id, &protected_ids);
                let is_perspective_player = p.id == perspective;
                let visible_hand_view = viewed_cards.filter(|view| {
                    view.zone == Zone::Hand
                        && view.subject == p.id
                        && (view.public || view.viewer == perspective)
                });
                let visible_hand_ids = visible_hand_view
                    .map(|view| view.cards.iter().copied().collect::<HashSet<_>>());
                let can_view_hand = is_perspective_player || visible_hand_view.is_some();
                PlayerSnapshot {
                    can_view_hand,
                    hand_cards: if can_view_hand {
                        p.hand
                            .iter()
                            .rev()
                            .filter(|id| {
                                is_perspective_player
                                    || visible_hand_ids
                                        .as_ref()
                                        .is_some_and(|visible_ids| visible_ids.contains(id))
                            })
                            .filter_map(|id| game.object(*id))
                            .map(|o| {
                                let mana_cost = o.mana_cost.as_ref().map(|mc| mc.to_oracle());
                                let power_toughness = match (o.power(), o.toughness()) {
                                    (Some(p), Some(t)) => Some(format!("{p}/{t}")),
                                    _ => None,
                                };
                                HandCardSnapshot {
                                    id: o.id.0,
                                    stable_id: o.stable_id.0.0,
                                    name: o.name.clone(),
                                    mana_cost,
                                    power_toughness,
                                    loyalty: o.loyalty(),
                                    defense: o.defense(),
                                    card_types: o
                                        .card_types
                                        .iter()
                                        .map(|ct| ct.name().to_string())
                                        .collect(),
                                }
                            })
                            .collect()
                    } else {
                        Vec::new()
                    },
                    graveyard_cards: p
                        .graveyard
                        .iter()
                        .rev()
                        .filter_map(|id| game.object(*id))
                        .map(|o| ZoneCardSnapshot {
                            id: o.id.0,
                            stable_id: o.stable_id.0.0,
                            name: o.name.clone(),
                            show_in_pseudo_hand: is_perspective_player
                                && should_surface_zone_card_in_pseudo_hand(
                                    game,
                                    perspective,
                                    o,
                                    Zone::Graveyard,
                                ),
                        })
                        .collect(),
                    exile_cards: game
                        .exile
                        .iter()
                        .rev()
                        .filter_map(|id| game.object(*id))
                        .filter(|o| o.owner == p.id)
                        .map(|o| ZoneCardSnapshot {
                            id: o.id.0,
                            stable_id: o.stable_id.0.0,
                            name: o.name.clone(),
                            show_in_pseudo_hand: is_perspective_player
                                && should_surface_zone_card_in_pseudo_hand(
                                    game,
                                    perspective,
                                    o,
                                    Zone::Exile,
                                ),
                        })
                        .collect(),
                    command_cards: game
                        .command_zone
                        .iter()
                        .rev()
                        .filter_map(|id| game.object(*id))
                        .filter(|o| o.owner == p.id)
                        .map(|o| ZoneCardSnapshot {
                            id: o.id.0,
                            stable_id: o.stable_id.0.0,
                            name: o.name.clone(),
                            show_in_pseudo_hand: is_perspective_player
                                && should_surface_zone_card_in_pseudo_hand(
                                    game,
                                    perspective,
                                    o,
                                    Zone::Command,
                                ),
                        })
                        .collect(),
                    library_top: p
                        .library
                        .last()
                        .and_then(|id| game.object(*id))
                        .map(|o| o.name.clone()),
                    graveyard_top: p
                        .graveyard
                        .last()
                        .and_then(|id| game.object(*id))
                        .map(|o| o.name.clone()),
                    battlefield,
                    battlefield_total,
                    id: p.id.0,
                    name: p.name.clone(),
                    life: p.life,
                    mana_pool: ManaPoolSnapshot {
                        white: p.mana_pool.white,
                        blue: p.mana_pool.blue,
                        black: p.mana_pool.black,
                        red: p.mana_pool.red,
                        green: p.mana_pool.green,
                        colorless: p.mana_pool.colorless,
                    },
                    hand_size: p.hand.len(),
                    library_size: p.library.len(),
                    graveyard_size: p.graveyard.len(),
                    command_size: game
                        .command_zone
                        .iter()
                        .filter_map(|id| game.object(*id))
                        .filter(|o| o.owner == p.id)
                        .count(),
                }
            })
            .collect();

        let mut stack_preview: Vec<String> = game
            .stack
            .iter()
            .rev()
            .map(|entry| {
                game.object(entry.object_id)
                    .map(|obj| obj.name.clone())
                    .unwrap_or_else(|| format!("Object#{}", entry.object_id.0))
            })
            .collect();
        let mut stack_objects: Vec<StackObjectSnapshot> = game
            .stack
            .iter()
            .rev()
            .map(|entry| build_stack_object_snapshot(game, perspective, viewed_cards, entry))
            .collect();
        let mut stack_size = game.stack.len();

        // During casting (rule 601.2), the card can be moved to stack before finalization.
        // Surface that pending spell in UI so it doesn't look like it vanished.
        if let Some(stack_id) = pending_cast_stack_id
            && !game.stack.iter().any(|entry| entry.object_id == stack_id)
            && let Some(obj) = game.object(stack_id)
        {
            stack_preview.insert(0, obj.name.clone());
            let pending_effect_text = {
                let lines = crate::compiled_text::compiled_lines(&obj.to_card_definition());
                if lines.is_empty() {
                    None
                } else {
                    Some(lines.join("; "))
                }
            };
            stack_objects.insert(
                0,
                StackObjectSnapshot {
                    id: stack_id.0,
                    inspect_object_id: Some(stack_id.0),
                    stable_id: Some(obj.stable_id.0.0),
                    source_stable_id: None,
                    controller: obj.controller.0,
                    name: obj.name.clone(),
                    mana_cost: obj.mana_cost.as_ref().map(|mc| mc.to_oracle()),
                    effect_text: pending_effect_text,
                    ability_kind: None,
                    ability_text: None,
                    targets: Vec::new(),
                },
            );
            stack_size += 1;
        }
        Self {
            snapshot_id,
            perspective: perspective.0,
            turn_number: game.turn.turn_number,
            active_player: game.turn.active_player.0,
            priority_player: game.turn.priority_player.map(|p| p.0),
            phase: game.turn.phase.to_string(),
            step: game.turn.step.map(|step| step.to_string()),
            stack_size,
            stack_preview,
            stack_objects,
            resolving_stack_object,
            battlefield_size: game.battlefield.len(),
            exile_size: game.exile.len(),
            players,
            battlefield_transitions,
            viewed_cards: viewed_cards
                .filter(|view| view.public || view.viewer == perspective)
                .map(|view| ViewedCardsSnapshot {
                    viewer: view.viewer.0,
                    subject: view.subject.0,
                    zone: view.zone.to_string(),
                    visibility: if view.public {
                        "public".to_string()
                    } else {
                        "private".to_string()
                    },
                    cards: view
                        .cards
                        .iter()
                        .map(|id| ViewedCardSnapshot {
                            id: id.0,
                            name: game
                                .object(*id)
                                .map(|obj| obj.name.clone())
                                .unwrap_or_else(|| format!("Card #{}", id.0)),
                        })
                        .collect(),
                    card_ids: view.cards.iter().map(|id| id.0).collect(),
                    source: view.source.map(|id| id.0),
                    description: view.description.clone(),
                }),
            decision: decision
                .map(|ctx| DecisionView::from_context(game, ctx, perspective, viewed_cards)),
            mana_payment,
            game_over: game_over.map(|r| GameOverView::from_result(game, r)),
            cancelable,
            undo_land_stable_id,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
struct ActionView {
    index: usize,
    label: String,
    kind: String,
    object_id: Option<u64>,
    ability_index: Option<usize>,
    from_zone: Option<String>,
    to_zone: Option<String>,
    action_ref: PriorityActionRef,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum PriorityActionRef {
    PassPriority,
    KeepOpeningHand,
    TakeMulligan,
    SerumPowderMulligan {
        card_id: u64,
    },
    ContinuePregame,
    BeginGame,
    UsePregameAction {
        card_id: u64,
        ability_index: usize,
    },
    CastSpell {
        spell_id: u64,
        from_zone: String,
        casting_method: CastingMethodRef,
    },
    ActivateAbility {
        source: u64,
        ability_index: usize,
    },
    PlayLand {
        land_id: u64,
    },
    ActivateManaAbility {
        source: u64,
        ability_index: usize,
    },
    TurnFaceUp {
        creature_id: u64,
    },
    SpecialAction {
        action: SpecialActionRef,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum SpecialActionRef {
    PlayLand {
        card_id: u64,
    },
    TurnFaceUp {
        permanent_id: u64,
    },
    Suspend {
        card_id: u64,
    },
    Foretell {
        card_id: u64,
    },
    Plot {
        card_id: u64,
    },
    ActivateManaAbility {
        permanent_id: u64,
        ability_index: usize,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum CastingMethodRef {
    Normal,
    SplitOtherHalf,
    Fuse,
    Alternative {
        index: usize,
    },
    GrantedEscape {
        source: u64,
        exile_count: u32,
    },
    GrantedFlashback,
    PlayFrom {
        source: u64,
        zone: String,
        use_alternative: Option<usize>,
    },
}

#[derive(Debug, Clone, Serialize)]
struct OptionView {
    index: usize,
    description: String,
    legal: bool,
    repeatable: bool,
    max_count: Option<u32>,
    object_id: Option<u64>,
    object_controller: Option<u8>,
}

#[derive(Debug, Clone, Serialize)]
struct ObjectChoiceView {
    id: u64,
    name: String,
    legal: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum TargetChoiceView {
    Player { player: u8, name: String },
    Object { object: u64, name: String },
}

#[derive(Debug, Clone, Serialize)]
struct TargetRequirementView {
    description: String,
    min_targets: usize,
    max_targets: Option<usize>,
    legal_targets: Vec<TargetChoiceView>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum AttackTargetView {
    Player { player: u8, name: String },
    Planeswalker { object: u64, name: String },
}

#[derive(Debug, Clone, Serialize)]
struct AttackerOptionView {
    creature: u64,
    creature_name: String,
    valid_targets: Vec<AttackTargetView>,
    must_attack: bool,
}

#[derive(Debug, Clone, Serialize)]
struct BlockerChoiceView {
    id: u64,
    name: String,
}

#[derive(Debug, Clone, Serialize)]
struct BlockerOptionView {
    attacker: u64,
    attacker_name: String,
    valid_blockers: Vec<BlockerChoiceView>,
    min_blockers: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum DecisionView {
    Priority {
        player: u8,
        actions: Vec<ActionView>,
    },
    Number {
        player: u8,
        description: String,
        min: u32,
        max: u32,
        is_x_value: bool,
        source_id: Option<u64>,
        source_name: Option<String>,
        context_text: Option<String>,
        consequence_text: Option<String>,
        reason: Option<String>,
    },
    SelectOptions {
        player: u8,
        description: String,
        min: usize,
        max: usize,
        options: Vec<OptionView>,
        source_id: Option<u64>,
        source_name: Option<String>,
        context_text: Option<String>,
        consequence_text: Option<String>,
        reason: Option<String>,
    },
    SelectObjects {
        player: u8,
        description: String,
        min: usize,
        max: Option<usize>,
        candidates: Vec<ObjectChoiceView>,
        source_id: Option<u64>,
        source_name: Option<String>,
        context_text: Option<String>,
        consequence_text: Option<String>,
        reason: Option<String>,
    },
    Targets {
        player: u8,
        context: String,
        requirements: Vec<TargetRequirementView>,
        source_id: Option<u64>,
        source_name: Option<String>,
        context_text: Option<String>,
        consequence_text: Option<String>,
        reason: Option<String>,
    },
    Attackers {
        player: u8,
        attacker_options: Vec<AttackerOptionView>,
    },
    Blockers {
        player: u8,
        blocker_options: Vec<BlockerOptionView>,
    },
}

impl DecisionView {
    fn from_context(
        game: &GameState,
        ctx: &DecisionContext,
        perspective: PlayerId,
        viewed_cards: Option<&ActiveViewedCards>,
    ) -> Self {
        let enriched_ctx = crate::decisions::context::enrich_display_hints(game, ctx.clone());
        let ctx = &enriched_ctx;
        let resolve_source_name = |source: Option<ObjectId>| -> Option<String> {
            source
                .and_then(|id| game.object(id).map(|obj| (id, obj)))
                .map(|(id, obj)| {
                    if object_visible_to_perspective(game, perspective, viewed_cards, id) {
                        obj.name.clone()
                    } else {
                        hidden_object_label()
                    }
                })
        };
        let resolve_source_id = |source: Option<ObjectId>| -> Option<u64> {
            source.and_then(|id| {
                object_visible_to_perspective(game, perspective, viewed_cards, id).then_some(id.0)
            })
        };
        let context_text = || ctx.context_text().map(str::to_string);
        let consequence_text = || ctx.consequence_text().map(str::to_string);
        let reason = decision_reason(ctx);

        match ctx {
            DecisionContext::Boolean(boolean) => DecisionView::SelectOptions {
                player: boolean.player.0,
                description: boolean.description.clone(),
                min: 1,
                max: 1,
                options: vec![
                    OptionView {
                        index: 1,
                        description: "Yes".to_string(),
                        legal: true,
                        repeatable: false,
                        max_count: Some(1),
                        object_id: None,
                        object_controller: None,
                    },
                    OptionView {
                        index: 0,
                        description: "No".to_string(),
                        legal: true,
                        repeatable: false,
                        max_count: Some(1),
                        object_id: None,
                        object_controller: None,
                    },
                ],
                source_id: resolve_source_id(boolean.source),
                source_name: resolve_source_name(boolean.source),
                context_text: context_text(),
                consequence_text: consequence_text(),
                reason: reason.clone(),
            },
            DecisionContext::Priority(priority) => DecisionView::Priority {
                player: priority.player.0,
                actions: priority
                    .actions
                    .iter()
                    .enumerate()
                    .map(|(index, action)| {
                        build_action_view(game, perspective, viewed_cards, index, action)
                    })
                    .collect(),
            },
            DecisionContext::Number(number) => DecisionView::Number {
                player: number.player.0,
                description: number.description.clone(),
                min: number.min,
                max: number.max,
                is_x_value: number.is_x_value,
                source_id: resolve_source_id(number.source),
                source_name: resolve_source_name(number.source),
                context_text: context_text(),
                consequence_text: consequence_text(),
                reason: reason.clone(),
            },
            DecisionContext::SelectOptions(options) => DecisionView::SelectOptions {
                player: options.player.0,
                description: options.description.clone(),
                min: options.min,
                max: options.max,
                options: {
                    let is_optional_cost_choice = options
                        .description
                        .to_ascii_lowercase()
                        .contains("optional cost");
                    options
                        .options
                        .iter()
                        .map(|opt| {
                            let (repeatable, max_count) = if is_optional_cost_choice {
                                optional_cost_selection_metadata(game, options.source, opt.index)
                            } else {
                                (false, None)
                            };
                            let visible_object_id = opt.object_id.and_then(|id| {
                                object_visible_to_perspective(game, perspective, viewed_cards, id)
                                    .then_some(id)
                            });
                            OptionView {
                                index: opt.index,
                                description: if opt.object_id.is_some()
                                    && visible_object_id.is_none()
                                {
                                    hidden_object_label()
                                } else {
                                    opt.description.clone()
                                },
                                legal: opt.legal,
                                repeatable,
                                max_count,
                                object_id: visible_object_id.map(|id| id.0),
                                object_controller: visible_object_id
                                    .and_then(|id| game.object(id))
                                    .map(|obj| obj.controller.0),
                            }
                        })
                        .collect()
                },
                source_id: resolve_source_id(options.source),
                source_name: resolve_source_name(options.source),
                context_text: context_text(),
                consequence_text: consequence_text(),
                reason: reason.clone(),
            },
            DecisionContext::Modes(modes) => DecisionView::SelectOptions {
                player: modes.player.0,
                description: format!("Choose mode for {}", modes.spell_name),
                min: modes.spec.min_modes,
                max: modes.spec.max_modes,
                options: modes
                    .spec
                    .modes
                    .iter()
                    .map(|mode| OptionView {
                        index: mode.index,
                        description: mode.description.clone(),
                        legal: mode.legal,
                        repeatable: false,
                        max_count: Some(1),
                        object_id: None,
                        object_controller: None,
                    })
                    .collect(),
                source_id: resolve_source_id(modes.source),
                source_name: resolve_source_name(modes.source),
                context_text: context_text(),
                consequence_text: consequence_text(),
                reason: reason.clone(),
            },
            DecisionContext::HybridChoice(hybrid) => DecisionView::SelectOptions {
                player: hybrid.player.0,
                description: format!(
                    "Choose how to pay pip {} of {}",
                    hybrid.pip_number, hybrid.spell_name
                ),
                min: 1,
                max: 1,
                options: hybrid
                    .options
                    .iter()
                    .map(|opt| OptionView {
                        index: opt.index,
                        description: opt.label.clone(),
                        legal: true,
                        repeatable: false,
                        max_count: Some(1),
                        object_id: None,
                        object_controller: None,
                    })
                    .collect(),
                source_id: resolve_source_id(hybrid.source),
                source_name: resolve_source_name(hybrid.source),
                context_text: context_text(),
                consequence_text: consequence_text(),
                reason: reason.clone(),
            },
            DecisionContext::Order(order) => DecisionView::SelectOptions {
                player: order.player.0,
                description: order.description.clone(),
                min: order.items.len(),
                max: order.items.len(),
                options: order
                    .items
                    .iter()
                    .enumerate()
                    .map(|(index, (object_id, name))| OptionView {
                        index,
                        description: if object_visible_to_perspective(
                            game,
                            perspective,
                            viewed_cards,
                            *object_id,
                        ) {
                            name.clone()
                        } else {
                            hidden_object_label()
                        },
                        legal: true,
                        repeatable: false,
                        max_count: Some(1),
                        object_id: object_visible_to_perspective(
                            game,
                            perspective,
                            viewed_cards,
                            *object_id,
                        )
                        .then_some(object_id.0),
                        object_controller: object_visible_to_perspective(
                            game,
                            perspective,
                            viewed_cards,
                            *object_id,
                        )
                        .then(|| game.object(*object_id).map(|obj| obj.controller.0))
                        .flatten(),
                    })
                    .collect(),
                source_id: resolve_source_id(order.source),
                source_name: resolve_source_name(order.source),
                context_text: context_text(),
                consequence_text: consequence_text(),
                reason: reason.clone(),
            },
            DecisionContext::Distribute(distribute) => DecisionView::SelectOptions {
                player: distribute.player.0,
                description: format!(
                    "{} (assign exactly {} total)",
                    distribute.description, distribute.total
                ),
                min: 0,
                max: distribute.total as usize,
                options: distribute
                    .targets
                    .iter()
                    .enumerate()
                    .map(|(index, target)| {
                        let visible_object_id = match &target.target {
                            Target::Object(object_id) => object_visible_to_perspective(
                                game,
                                perspective,
                                viewed_cards,
                                *object_id,
                            )
                            .then_some(*object_id),
                            _ => None,
                        };
                        OptionView {
                            index,
                            description: if matches!(target.target, Target::Object(_))
                                && visible_object_id.is_none()
                            {
                                hidden_object_label()
                            } else {
                                target.name.clone()
                            },
                            legal: true,
                            repeatable: true,
                            max_count: Some(distribute.total),
                            object_id: visible_object_id.map(|object_id| object_id.0),
                            object_controller: visible_object_id
                                .and_then(|object_id| game.object(object_id))
                                .map(|obj| obj.controller.0),
                        }
                    })
                    .collect(),
                source_id: resolve_source_id(distribute.source),
                source_name: resolve_source_name(distribute.source),
                context_text: context_text(),
                consequence_text: consequence_text(),
                reason: reason.clone(),
            },
            DecisionContext::Colors(colors) => {
                let choices = colors_for_context(colors);
                let repeatable_colors = !colors.same_color && colors.count > 1;
                DecisionView::SelectOptions {
                    player: colors.player.0,
                    description: colors.description.clone(),
                    min: if colors.count == 0 { 0 } else { 1 },
                    max: if colors.same_color {
                        1
                    } else {
                        (colors.count as usize).max(1)
                    },
                    options: choices
                        .into_iter()
                        .enumerate()
                        .map(|(index, color)| OptionView {
                            index,
                            description: color_name(color).to_string(),
                            legal: true,
                            repeatable: repeatable_colors,
                            max_count: Some(if repeatable_colors { colors.count } else { 1 }),
                            object_id: None,
                            object_controller: None,
                        })
                        .collect(),
                    source_id: resolve_source_id(colors.source),
                    source_name: resolve_source_name(colors.source),
                    context_text: context_text(),
                    consequence_text: consequence_text(),
                    reason: reason.clone(),
                }
            }
            DecisionContext::Counters(counters) => DecisionView::SelectOptions {
                player: counters.player.0,
                description: format!(
                    "Choose up to {} counters to remove from {}",
                    counters.max_total, counters.target_name
                ),
                min: 0,
                max: counters.max_total as usize,
                options: counters
                    .available_counters
                    .iter()
                    .enumerate()
                    .map(|(index, (counter_type, available))| OptionView {
                        index,
                        description: format!(
                            "{} ({available} available)",
                            counter_type.description()
                        ),
                        legal: *available > 0,
                        repeatable: *available > 1,
                        max_count: Some(*available),
                        object_id: None,
                        object_controller: None,
                    })
                    .collect(),
                source_id: resolve_source_id(counters.source),
                source_name: resolve_source_name(counters.source),
                context_text: context_text(),
                consequence_text: consequence_text(),
                reason: reason.clone(),
            },
            DecisionContext::Partition(partition) => DecisionView::SelectObjects {
                player: partition.player.0,
                description: format!(
                    "{} \u{2014} select cards to put on {}",
                    partition.description, partition.secondary_label
                ),
                min: 0,
                max: Some(partition.cards.len()),
                candidates: partition
                    .cards
                    .iter()
                    .map(|(id, name)| ObjectChoiceView {
                        id: id.0,
                        name: name.clone(),
                        legal: true,
                    })
                    .collect(),
                source_id: resolve_source_id(partition.source),
                source_name: resolve_source_name(partition.source),
                context_text: context_text(),
                consequence_text: consequence_text(),
                reason: reason.clone(),
            },
            DecisionContext::Proliferate(proliferate) => DecisionView::SelectOptions {
                player: proliferate.player.0,
                description: "Choose permanents and/or players to proliferate".to_string(),
                min: 0,
                max: proliferate.eligible_permanents.len() + proliferate.eligible_players.len(),
                options: proliferate
                    .eligible_permanents
                    .iter()
                    .enumerate()
                    .map(|(index, (_, name))| OptionView {
                        index,
                        description: format!("Permanent: {name}"),
                        legal: true,
                        repeatable: false,
                        max_count: Some(1),
                        object_id: proliferate
                            .eligible_permanents
                            .get(index)
                            .map(|(id, _)| id.0),
                        object_controller: proliferate
                            .eligible_permanents
                            .get(index)
                            .and_then(|(id, _)| game.object(*id))
                            .map(|obj| obj.controller.0),
                    })
                    .chain(proliferate.eligible_players.iter().enumerate().map(
                        |(offset, (_, name))| OptionView {
                            index: proliferate.eligible_permanents.len() + offset,
                            description: format!("Player: {name}"),
                            legal: true,
                            repeatable: false,
                            max_count: Some(1),
                            object_id: None,
                            object_controller: None,
                        },
                    ))
                    .collect(),
                source_id: resolve_source_id(proliferate.source),
                source_name: resolve_source_name(proliferate.source),
                context_text: context_text(),
                consequence_text: consequence_text(),
                reason: reason.clone(),
            },
            DecisionContext::SelectObjects(objects) => DecisionView::SelectObjects {
                player: objects.player.0,
                description: objects.description.clone(),
                min: objects.min,
                max: objects.max,
                candidates: objects
                    .candidates
                    .iter()
                    .enumerate()
                    .map(|(index, obj)| {
                        let visible =
                            object_visible_to_perspective(game, perspective, viewed_cards, obj.id);
                        ObjectChoiceView {
                            id: if visible {
                                obj.id.0
                            } else {
                                redacted_choice_id(index)
                            },
                            name: if visible {
                                obj.name.clone()
                            } else {
                                hidden_object_label()
                            },
                            legal: obj.legal,
                        }
                    })
                    .collect(),
                source_id: resolve_source_id(objects.source),
                source_name: resolve_source_name(objects.source),
                context_text: context_text(),
                consequence_text: consequence_text(),
                reason: reason.clone(),
            },
            DecisionContext::Targets(targets) => DecisionView::Targets {
                player: targets.player.0,
                context: targets.context.clone(),
                requirements: targets
                    .requirements
                    .iter()
                    .map(|req| TargetRequirementView {
                        description: req.description.clone(),
                        min_targets: req.min_targets,
                        max_targets: req.max_targets,
                        legal_targets: req
                            .legal_targets
                            .iter()
                            .enumerate()
                            .map(|(index, target)| {
                                target_choice_view(game, perspective, viewed_cards, index, target)
                            })
                            .collect(),
                    })
                    .collect(),
                source_id: Some(targets.source.0),
                source_name: resolve_source_name(Some(targets.source)),
                context_text: context_text(),
                consequence_text: consequence_text(),
                reason,
            },
            DecisionContext::Attackers(attackers) => DecisionView::Attackers {
                player: attackers.player.0,
                attacker_options: attackers
                    .attacker_options
                    .iter()
                    .map(|option| AttackerOptionView {
                        creature: option.creature.0,
                        creature_name: option.creature_name.clone(),
                        valid_targets: option
                            .valid_targets
                            .iter()
                            .map(|target| attack_target_view(game, target))
                            .collect(),
                        must_attack: option.must_attack,
                    })
                    .collect(),
            },
            DecisionContext::Blockers(blockers) => DecisionView::Blockers {
                player: blockers.player.0,
                blocker_options: blockers
                    .blocker_options
                    .iter()
                    .map(|option| BlockerOptionView {
                        attacker: option.attacker.0,
                        attacker_name: option.attacker_name.clone(),
                        valid_blockers: option
                            .valid_blockers
                            .iter()
                            .map(|(id, name)| BlockerChoiceView {
                                id: id.0,
                                name: name.clone(),
                            })
                            .collect(),
                        min_blockers: option.min_blockers,
                    })
                    .collect(),
            },
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum GameOverView {
    Winner { player: u8, name: String },
    Draw,
    Remaining { players: Vec<u8> },
}

impl GameOverView {
    fn from_result(game: &GameState, result: &GameResult) -> Self {
        match result {
            GameResult::Winner(player) => GameOverView::Winner {
                player: player.0,
                name: game
                    .player(*player)
                    .map(|p| p.name.clone())
                    .unwrap_or_else(|| format!("Player {}", player.0 + 1)),
            },
            GameResult::Draw => GameOverView::Draw,
            GameResult::Remaining(players) => GameOverView::Remaining {
                players: players.iter().map(|p| p.0).collect(),
            },
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum UiCommand {
    PriorityAction {
        #[serde(default)]
        action_index: Option<usize>,
        #[serde(default)]
        action_ref: Option<PriorityActionRef>,
    },
    NumberChoice {
        value: u32,
    },
    SelectOptions {
        option_indices: Vec<usize>,
    },
    SelectObjects {
        object_ids: Vec<u64>,
    },
    SelectTargets {
        targets: Vec<TargetInput>,
    },
    DeclareAttackers {
        declarations: Vec<AttackerDeclarationInput>,
    },
    DeclareBlockers {
        declarations: Vec<BlockerDeclarationInput>,
    },
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum TargetInput {
    Player { player: u8 },
    Object { object: u64 },
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum AttackTargetInput {
    Player { player: u8 },
    Planeswalker { object: u64 },
}

#[derive(Debug, Clone, Deserialize)]
struct AttackerDeclarationInput {
    creature: u64,
    target: AttackTargetInput,
}

#[derive(Debug, Clone, Deserialize)]
struct BlockerDeclarationInput {
    blocker: u64,
    blocking: u64,
}

#[derive(Debug, Clone)]
enum ReplayDecisionAnswer {
    Boolean(bool),
    Number(u32),
    Options(Vec<usize>),
    Objects(Vec<ObjectId>),
    Order(Vec<ObjectId>),
    Distribute(Vec<(Target, u32)>),
    Colors(Vec<crate::color::Color>),
    Counters(Vec<(crate::object::CounterType, u32)>),
    Partition(Vec<ObjectId>),
    Proliferate(crate::decisions::specs::ProliferateResponse),
    Targets(Vec<Target>),
    Priority(LegalAction),
    Attackers(Vec<crate::decisions::spec::AttackerDeclaration>),
    Blockers(Vec<crate::decisions::spec::BlockerDeclaration>),
}

#[derive(Debug, Clone)]
struct ReplayCheckpoint {
    game: GameState,
    trigger_queue: TriggerQueue,
    priority_state: PriorityLoopState,
    game_over: Option<GameResult>,
    id_counters: crate::ids::IdCountersSnapshot,
    /// Diagnostic tag identifying where this checkpoint was captured.
    diag_tag: &'static str,
}

/// Distinguishes user-action replays from auto-advance replays.
#[derive(Debug, Clone)]
enum ReplayRoot {
    /// User chose a priority response (cast spell, activate ability, etc.)
    Response(PriorityResponse),
    /// The game loop is auto-advancing and hit a decision (e.g. triggered ability targeting).
    Advance,
}

#[derive(Debug, Clone)]
struct PendingReplayAction {
    checkpoint: ReplayCheckpoint,
    root: ReplayRoot,
    nested_answers: Vec<ReplayDecisionAnswer>,
}

#[derive(Debug, Clone)]
struct LivePriorityContinuation {
    checkpoint: ReplayCheckpoint,
    root: PendingPriorityContinuation,
    answers: Vec<ReplayDecisionAnswer>,
}

#[derive(Debug, Clone)]
enum ReplayOutcome {
    NeedsDecision(DecisionContext),
    Complete(GameProgress),
}

#[derive(Debug)]
struct WasmReplayDecisionMaker {
    answers: VecDeque<ReplayDecisionAnswer>,
    pending_context: Option<DecisionContext>,
    viewed_cards: Option<ActiveViewedCards>,
}

impl WasmReplayDecisionMaker {
    fn new(answers: &[ReplayDecisionAnswer]) -> Self {
        Self {
            answers: answers.iter().cloned().collect(),
            pending_context: None,
            viewed_cards: None,
        }
    }

    fn capture_once(&mut self, ctx: DecisionContext) {
        if self.pending_context.is_none() {
            self.pending_context = Some(ctx);
        }
    }

    fn capture_once_for_game(&mut self, game: &GameState, ctx: DecisionContext) {
        self.capture_once(crate::decisions::context::enrich_display_hints(game, ctx));
    }

    fn finish(self) -> (Option<DecisionContext>, Option<ActiveViewedCards>) {
        (self.pending_context, self.viewed_cards)
    }
}

impl DecisionMaker for WasmReplayDecisionMaker {
    fn awaiting_choice(&self) -> bool {
        self.pending_context.is_some()
    }

    fn decide_boolean(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::BooleanContext,
    ) -> bool {
        match self.answers.front() {
            Some(ReplayDecisionAnswer::Boolean(value)) => {
                let value = *value;
                self.answers.pop_front();
                value
            }
            _ => {
                self.capture_once_for_game(game, DecisionContext::Boolean(ctx.clone()));
                false
            }
        }
    }

    fn decide_number(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::NumberContext,
    ) -> u32 {
        match self.answers.front() {
            Some(ReplayDecisionAnswer::Number(value)) => {
                let value = *value;
                self.answers.pop_front();
                value
            }
            _ => {
                self.capture_once_for_game(game, DecisionContext::Number(ctx.clone()));
                ctx.min
            }
        }
    }

    fn decide_objects(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::SelectObjectsContext,
    ) -> Vec<ObjectId> {
        match self.answers.front() {
            Some(ReplayDecisionAnswer::Objects(ids)) => {
                let ids = ids.clone();
                self.answers.pop_front();
                ids
            }
            _ => {
                self.capture_once_for_game(game, DecisionContext::SelectObjects(ctx.clone()));
                ctx.candidates
                    .iter()
                    .filter(|candidate| candidate.legal)
                    .map(|candidate| candidate.id)
                    .take(ctx.min)
                    .collect()
            }
        }
    }

    fn decide_options(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::SelectOptionsContext,
    ) -> Vec<usize> {
        match self.answers.front() {
            Some(ReplayDecisionAnswer::Options(indices)) => {
                let indices = indices.clone();
                self.answers.pop_front();
                indices
            }
            _ => {
                self.capture_once_for_game(game, DecisionContext::SelectOptions(ctx.clone()));
                ctx.options
                    .iter()
                    .filter(|option| option.legal)
                    .map(|option| option.index)
                    .take(ctx.min)
                    .collect()
            }
        }
    }

    fn decide_priority(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::PriorityContext,
    ) -> LegalAction {
        match self.answers.front() {
            Some(ReplayDecisionAnswer::Priority(action)) => {
                let action = action.clone();
                self.answers.pop_front();
                action
            }
            _ => ctx
                .actions
                .iter()
                .find(|action| matches!(action, LegalAction::PassPriority))
                .cloned()
                .unwrap_or_else(|| {
                    ctx.actions
                        .first()
                        .cloned()
                        .unwrap_or(LegalAction::PassPriority)
                }),
        }
    }

    fn decide_targets(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::TargetsContext,
    ) -> Vec<Target> {
        match self.answers.front() {
            Some(ReplayDecisionAnswer::Targets(targets)) => {
                let targets = targets.clone();
                self.answers.pop_front();
                targets
            }
            _ => {
                self.capture_once_for_game(game, DecisionContext::Targets(ctx.clone()));
                normalize_targets_for_requirements(&ctx.requirements, Vec::new())
                    .unwrap_or_default()
            }
        }
    }

    fn decide_attackers(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::AttackersContext,
    ) -> Vec<crate::decisions::spec::AttackerDeclaration> {
        match self.answers.front() {
            Some(ReplayDecisionAnswer::Attackers(declarations)) => {
                let declarations = declarations.clone();
                self.answers.pop_front();
                declarations
            }
            _ => ctx
                .attacker_options
                .iter()
                .filter(|option| option.must_attack)
                .filter_map(|option| {
                    option.valid_targets.first().map(|target| {
                        crate::decisions::spec::AttackerDeclaration {
                            creature: option.creature,
                            target: target.clone(),
                        }
                    })
                })
                .collect(),
        }
    }

    fn decide_blockers(
        &mut self,
        _game: &GameState,
        _ctx: &crate::decisions::context::BlockersContext,
    ) -> Vec<crate::decisions::spec::BlockerDeclaration> {
        match self.answers.front() {
            Some(ReplayDecisionAnswer::Blockers(declarations)) => {
                let declarations = declarations.clone();
                self.answers.pop_front();
                declarations
            }
            _ => Vec::new(),
        }
    }

    fn decide_order(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::OrderContext,
    ) -> Vec<ObjectId> {
        match self.answers.front() {
            Some(ReplayDecisionAnswer::Order(order)) => {
                let order = order.clone();
                self.answers.pop_front();
                order
            }
            _ => {
                self.capture_once_for_game(game, DecisionContext::Order(ctx.clone()));
                ctx.items.iter().map(|(id, _)| *id).collect()
            }
        }
    }

    fn decide_distribute(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::DistributeContext,
    ) -> Vec<(Target, u32)> {
        match self.answers.front() {
            Some(ReplayDecisionAnswer::Distribute(distribution)) => {
                let distribution = distribution.clone();
                self.answers.pop_front();
                distribution
            }
            _ => {
                self.capture_once_for_game(game, DecisionContext::Distribute(ctx.clone()));
                Vec::new()
            }
        }
    }

    fn decide_colors(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::ColorsContext,
    ) -> Vec<crate::color::Color> {
        match self.answers.front() {
            Some(ReplayDecisionAnswer::Colors(colors)) => {
                let colors = colors.clone();
                self.answers.pop_front();
                colors
            }
            _ => {
                self.capture_once_for_game(game, DecisionContext::Colors(ctx.clone()));
                vec![crate::color::Color::Green; ctx.count as usize]
            }
        }
    }

    fn decide_counters(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::CountersContext,
    ) -> Vec<(crate::object::CounterType, u32)> {
        match self.answers.front() {
            Some(ReplayDecisionAnswer::Counters(counters)) => {
                let counters = counters.clone();
                self.answers.pop_front();
                counters
            }
            _ => {
                self.capture_once_for_game(game, DecisionContext::Counters(ctx.clone()));
                Vec::new()
            }
        }
    }

    fn decide_partition(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::PartitionContext,
    ) -> Vec<ObjectId> {
        match self.answers.front() {
            Some(ReplayDecisionAnswer::Partition(partition)) => {
                let partition = partition.clone();
                self.answers.pop_front();
                partition
            }
            _ => {
                self.capture_once_for_game(game, DecisionContext::Partition(ctx.clone()));
                Vec::new()
            }
        }
    }

    fn decide_proliferate(
        &mut self,
        game: &GameState,
        ctx: &crate::decisions::context::ProliferateContext,
    ) -> crate::decisions::specs::ProliferateResponse {
        match self.answers.front() {
            Some(ReplayDecisionAnswer::Proliferate(response)) => {
                let response = response.clone();
                self.answers.pop_front();
                response
            }
            _ => {
                self.capture_once_for_game(game, DecisionContext::Proliferate(ctx.clone()));
                crate::decisions::specs::ProliferateResponse::default()
            }
        }
    }

    fn view_cards(
        &mut self,
        _game: &GameState,
        viewer: PlayerId,
        cards: &[ObjectId],
        ctx: &crate::decisions::context::ViewCardsContext,
    ) {
        merge_active_viewed_cards(&mut self.viewed_cards, viewer, cards, ctx);
    }
}

/// Browser-exposed game handle.
#[wasm_bindgen]
pub struct WasmGame {
    game: GameState,
    registry: CardRegistry,
    trigger_queue: TriggerQueue,
    priority_state: PriorityLoopState,
    pregame: Option<PregameState>,
    match_format: MatchFormatInput,
    pending_decision: Option<DecisionContext>,
    pending_replay_action: Option<PendingReplayAction>,
    /// Checkpoint at the start of the current user-initiated spell/ability
    /// action chain. Unlike `pending_replay_action`, this survives nested
    /// prompts while the action is still being announced or paid. Once the
    /// spell or ability is committed and resolution produces a follow-up
    /// prompt, this checkpoint is cleared so Undo does not rewind a resolving
    /// action.
    pending_action_checkpoint: Option<ReplayCheckpoint>,
    /// Root priority response for the current live action chain.
    pending_live_action_root: Option<PriorityResponse>,
    /// Replayable suspended live priority computation plus any nested answers
    /// already provided for it.
    pending_live_continuation: Option<LivePriorityContinuation>,
    game_over: Option<GameResult>,
    perspective: PlayerId,
    /// The unified turn state machine. Created lazily on first advance.
    runner: Option<crate::turn_runner::TurnRunner>,
    /// True when the TurnRunner has yielded RunPriority and we're inside
    /// the priority loop waiting for it to complete.
    runner_awaiting_priority: bool,
    /// True when the pending_decision came from TurnRunner (attacker/blocker/discard
    /// decisions) rather than from the priority loop.
    runner_pending_decision: bool,
    /// When true, cleanup discard decisions are auto-resolved with random cards.
    auto_cleanup_discard: bool,
    /// Snapshot of game state when the player first got priority in the current
    /// priority round.  `cancelDecision` rolls back to this point so that
    /// mana-ability activations, partial casts, etc. are all undone.
    priority_epoch_checkpoint: Option<ReplayCheckpoint>,
    /// True once an undoable user action has successfully committed in the
    /// current priority epoch.
    priority_epoch_has_undoable_action: bool,
    /// Latched for the current priority epoch when an irreversible mana ability
    /// activation has occurred (for example sacrifice/counter/life side effects).
    priority_epoch_undo_locked_by_mana: bool,
    /// Stable id of the most recent reversible land-for-mana tap committed in
    /// the current priority epoch.
    priority_epoch_undo_land_stable_id: Option<u64>,
    /// User-configured minimum semantic threshold for card addition (0.0 = no filter).
    semantic_threshold: f32,
    /// Monotonic UI snapshot sequence so the frontend can process one-shot batches once.
    snapshot_serial: u64,
    /// Most recent transient card-view event visible to the current perspective.
    active_viewed_cards: Option<ActiveViewedCards>,
    /// UI-only top stack entry that is currently resolving while a prompt is open.
    active_resolving_stack_object: Option<StackObjectSnapshot>,
}

#[derive(Debug, Clone, Serialize)]
struct RegistryPreloadStatus {
    loaded: usize,
    cursor: usize,
    total: usize,
    done: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct DeckLoadResult {
    loaded: u32,
    failed: Vec<String>,
    failed_below_threshold: Vec<String>,
    failed_to_parse: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct CardLoadDiagnostics {
    query: String,
    canonical_name: Option<String>,
    error: Option<String>,
    parse_error: Option<String>,
    oracle_text: Option<String>,
    compiled_text: Vec<String>,
    compiled_abilities: Vec<String>,
    semantic_score: Option<f32>,
    threshold_percent: Option<f32>,
}

static AUTOCOMPLETE_CARD_NAMES: OnceLock<Vec<(String, String)>> = OnceLock::new();

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MatchSetupInput {
    player_names: Vec<String>,
    starting_life: i32,
    seed: u64,
    #[serde(default)]
    format: MatchFormatInput,
    #[serde(default)]
    decks: Option<Vec<Vec<String>>>,
    #[serde(default)]
    commanders: Option<Vec<Vec<String>>>,
    #[serde(default)]
    opening_hand_size: Option<usize>,
}

#[derive(Debug, Clone, Copy, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum MatchFormatInput {
    #[default]
    Normal,
    Commander,
}

#[derive(Debug, Clone)]
struct PregameState {
    opening_hand_size: usize,
    format: MatchFormatInput,
    mulligans_taken: HashMap<PlayerId, u32>,
    stage: PregameStage,
}

#[derive(Debug, Clone)]
struct PendingPregameHandExile {
    player: PlayerId,
    source: ObjectId,
    amount: usize,
}

#[derive(Debug, Clone)]
enum PregameStage {
    MulliganDecision {
        undecided_players: Vec<PlayerId>,
        round_mulliganers: Vec<PlayerId>,
    },
    BottomCards {
        queue: Vec<PlayerId>,
        pending_order: Option<(PlayerId, Vec<ObjectId>)>,
    },
    OpeningActions {
        current_index: usize,
        pending_hand_exile: Option<PendingPregameHandExile>,
    },
}

impl PregameState {
    fn new(turn_order: &[PlayerId], opening_hand_size: usize, format: MatchFormatInput) -> Self {
        Self {
            opening_hand_size,
            format,
            mulligans_taken: HashMap::new(),
            stage: PregameStage::MulliganDecision {
                undecided_players: turn_order.to_vec(),
                round_mulliganers: Vec::new(),
            },
        }
    }

    fn free_mulligan_count(&self) -> u32 {
        match self.format {
            MatchFormatInput::Commander => 1,
            MatchFormatInput::Normal => 0,
        }
    }

    fn cards_to_bottom(&self, player: PlayerId) -> usize {
        self.mulligans_taken
            .get(&player)
            .copied()
            .unwrap_or(0)
            .saturating_sub(self.free_mulligan_count()) as usize
    }
}

#[wasm_bindgen(start)]
pub fn wasm_start() {
    console_error_panic_hook::set_once();
}

#[wasm_bindgen]
impl WasmGame {
    fn current_mana_payment_view(&self) -> Option<ManaPaymentView> {
        if let Some(pending) = self.priority_state.pending_cast.as_ref()
            && let Some(view) = mana_payment_view_from_pending_cast(&self.game, pending)
        {
            return Some(view);
        }

        if let Some(pending) = self.priority_state.pending_activation.as_ref()
            && let Some(view) = mana_payment_view_from_pending_activation(pending)
        {
            return Some(view);
        }

        None
    }

    /// Construct a demo game with two players.
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        let mut priority_state = PriorityLoopState::new(2);
        priority_state.set_auto_choose_single_pip_payment(false);
        Self {
            game: GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20),
            registry: CardRegistry::new(),
            trigger_queue: TriggerQueue::new(),
            priority_state,
            pregame: None,
            match_format: MatchFormatInput::Normal,
            pending_decision: None,
            pending_replay_action: None,
            pending_action_checkpoint: None,
            pending_live_action_root: None,
            pending_live_continuation: None,
            game_over: None,
            perspective: PlayerId::from_index(0),
            runner: None,
            runner_awaiting_priority: false,
            runner_pending_decision: false,
            auto_cleanup_discard: true,
            priority_epoch_checkpoint: None,
            priority_epoch_has_undoable_action: false,
            priority_epoch_undo_locked_by_mana: false,
            priority_epoch_undo_land_stable_id: None,
            semantic_threshold: 0.0,
            snapshot_serial: 0,
            active_viewed_cards: None,
            active_resolving_stack_object: None,
        }
    }

    /// Reset game with custom player names and starting life.
    #[wasm_bindgen(js_name = reset)]
    pub fn reset_from_js(
        &mut self,
        player_names: JsValue,
        starting_life: i32,
    ) -> Result<(), JsValue> {
        let names: Vec<String> = serde_wasm_bindgen::from_value(player_names)
            .map_err(|e| JsValue::from_str(&format!("invalid player_names: {e}")))?;

        if names.is_empty() {
            return Err(JsValue::from_str("player_names cannot be empty"));
        }

        let seed = self.generate_match_seed();
        self.initialize_empty_match(names, starting_life, seed);
        self.populate_demo_libraries()?;
        self.finish_match_setup(7)
    }

    /// Start a fully specified match from a synchronized lobby payload.
    #[wasm_bindgen(js_name = startMatch)]
    pub fn start_match(&mut self, config: JsValue) -> Result<JsValue, JsValue> {
        let config: MatchSetupInput = serde_wasm_bindgen::from_value(config)
            .map_err(|e| JsValue::from_str(&format!("invalid match config: {e}")))?;

        if config.player_names.is_empty() {
            return Err(JsValue::from_str("player_names cannot be empty"));
        }

        let opening_hand_size = config.opening_hand_size.unwrap_or(7);
        self.initialize_empty_match(config.player_names, config.starting_life, config.seed);
        self.match_format = config.format;

        if let MatchFormatInput::Commander = config.format {
            let Some(decks) = config.decks.as_ref() else {
                return Err(JsValue::from_str(
                    "commander matches require explicit decklists",
                ));
            };
            let Some(commanders) = config.commanders.as_ref() else {
                return Err(JsValue::from_str(
                    "commander matches require commander lists",
                ));
            };
            self.validate_commander_setup(decks, commanders)?;
        }

        if let Some(decks) = config.decks {
            if decks.len() != self.game.players.len() {
                return Err(JsValue::from_str(
                    "deck count must match number of players in game",
                ));
            }
            self.populate_explicit_libraries(&decks)?;
        } else {
            self.populate_demo_libraries()?;
        }

        if let Some(commanders) = config.commanders {
            if commanders.len() != self.game.players.len() {
                return Err(JsValue::from_str(
                    "commander count must match number of players in game",
                ));
            }
            self.populate_explicit_commanders(&commanders)?;
        }

        self.finish_match_setup(opening_hand_size)?;
        self.snapshot()
    }

    /// Return a JS object snapshot of public game state.
    #[wasm_bindgen]
    pub fn snapshot(&mut self) -> Result<JsValue, JsValue> {
        let pending_cast_stack_id = self
            .priority_state
            .pending_cast
            .as_ref()
            .map(|p| p.stack_id);
        let cancelable = self.is_cancelable();
        let undo_land_stable_id = self.visible_undo_land_stable_id(cancelable);
        self.snapshot_serial = self.snapshot_serial.saturating_add(1);
        let snapshot_id = self.snapshot_serial;
        let battlefield_transitions = self
            .game
            .take_ui_battlefield_transitions()
            .into_iter()
            .map(|transition| BattlefieldTransitionSnapshot {
                stable_id: transition.stable_id.0.0,
                kind: match transition.kind {
                    crate::game_state::UiBattlefieldTransitionKind::Damaged => {
                        BattlefieldTransitionKindSnapshot::Damaged
                    }
                    crate::game_state::UiBattlefieldTransitionKind::Destroyed => {
                        BattlefieldTransitionKindSnapshot::Destroyed
                    }
                    crate::game_state::UiBattlefieldTransitionKind::Sacrificed => {
                        BattlefieldTransitionKindSnapshot::Sacrificed
                    }
                    crate::game_state::UiBattlefieldTransitionKind::Exiled => {
                        BattlefieldTransitionKindSnapshot::Exiled
                    }
                },
            })
            .collect();
        let snap = GameSnapshot::from_game(
            &self.game,
            self.perspective,
            self.pending_decision.as_ref(),
            self.current_mana_payment_view(),
            self.game_over.as_ref(),
            pending_cast_stack_id,
            self.active_resolving_stack_object.clone(),
            battlefield_transitions,
            self.active_viewed_cards.as_ref(),
            cancelable,
            undo_land_stable_id,
            snapshot_id,
        );
        serde_wasm_bindgen::to_value(&snap)
            .map_err(|e| JsValue::from_str(&format!("snapshot encode failed: {e}")))
    }

    /// Return the current UI state from the selected player perspective.
    #[wasm_bindgen(js_name = uiState)]
    pub fn ui_state(&mut self) -> Result<JsValue, JsValue> {
        self.snapshot()
    }

    /// Number of cards currently available in the registry.
    #[wasm_bindgen(js_name = registrySize)]
    pub fn registry_size(&self) -> usize {
        self.registry.len()
    }

    /// Incremental generated-registry preload status.
    #[wasm_bindgen(js_name = preloadRegistryStatus)]
    pub fn preload_registry_status(&self) -> Result<JsValue, JsValue> {
        // Fidelity coverage is precomputed during the build pipeline, so this is
        // effectively complete immediately.
        let total = CardRegistry::generated_parser_semantic_scored_count();
        let status = RegistryPreloadStatus {
            loaded: total,
            cursor: total,
            total,
            done: true,
        };
        serde_wasm_bindgen::to_value(&status)
            .map_err(|e| JsValue::from_str(&format!("preloadRegistryStatus encode failed: {e}")))
    }

    /// Parse/register the next batch of generated cards for startup warmup.
    #[wasm_bindgen(js_name = preloadRegistryChunk)]
    pub fn preload_registry_chunk(&mut self, _chunk_size: usize) -> Result<JsValue, JsValue> {
        self.preload_registry_status()
    }

    /// Return a detailed, human-readable object snapshot for inspector UI.
    #[wasm_bindgen(js_name = objectDetails)]
    pub fn object_details(&self, object_id: u64) -> Result<JsValue, JsValue> {
        let details = build_object_details_snapshot(&self.game, ObjectId::from_raw(object_id))
            .ok_or_else(|| JsValue::from_str(&format!("unknown object id: {object_id}")))?;
        serde_wasm_bindgen::to_value(&details)
            .map_err(|e| JsValue::from_str(&format!("objectDetails encode failed: {e}")))
    }

    /// Return game snapshot as pretty JSON.
    #[wasm_bindgen(js_name = snapshotJson)]
    pub fn snapshot_json(&mut self) -> Result<String, JsValue> {
        let pending_cast_stack_id = self
            .priority_state
            .pending_cast
            .as_ref()
            .map(|p| p.stack_id);
        let cancelable = self.is_cancelable();
        let undo_land_stable_id = self.visible_undo_land_stable_id(cancelable);
        self.snapshot_serial = self.snapshot_serial.saturating_add(1);
        let snapshot_id = self.snapshot_serial;
        let battlefield_transitions = self
            .game
            .take_ui_battlefield_transitions()
            .into_iter()
            .map(|transition| BattlefieldTransitionSnapshot {
                stable_id: transition.stable_id.0.0,
                kind: match transition.kind {
                    crate::game_state::UiBattlefieldTransitionKind::Damaged => {
                        BattlefieldTransitionKindSnapshot::Damaged
                    }
                    crate::game_state::UiBattlefieldTransitionKind::Destroyed => {
                        BattlefieldTransitionKindSnapshot::Destroyed
                    }
                    crate::game_state::UiBattlefieldTransitionKind::Sacrificed => {
                        BattlefieldTransitionKindSnapshot::Sacrificed
                    }
                    crate::game_state::UiBattlefieldTransitionKind::Exiled => {
                        BattlefieldTransitionKindSnapshot::Exiled
                    }
                },
            })
            .collect();
        let snap = GameSnapshot::from_game(
            &self.game,
            self.perspective,
            self.pending_decision.as_ref(),
            self.current_mana_payment_view(),
            self.game_over.as_ref(),
            pending_cast_stack_id,
            self.active_resolving_stack_object.clone(),
            battlefield_transitions,
            self.active_viewed_cards.as_ref(),
            cancelable,
            undo_land_stable_id,
            snapshot_id,
        );
        serde_json::to_string_pretty(&snap)
            .map_err(|e| JsValue::from_str(&format!("json encode failed: {e}")))
    }

    /// Return locally-known card name suggestions from the generated registry.
    #[wasm_bindgen(js_name = autocompleteCardNames)]
    pub fn autocomplete_card_names(
        &self,
        query: String,
        limit: Option<usize>,
    ) -> Result<JsValue, JsValue> {
        let trimmed = query.trim();
        if trimmed.is_empty() {
            return serde_wasm_bindgen::to_value(&Vec::<String>::new()).map_err(|e| {
                JsValue::from_str(&format!("autocompleteCardNames encode failed: {e}"))
            });
        }

        let query_lower = trimmed.to_lowercase();
        let capped_limit = limit.unwrap_or(5).clamp(1, 25);
        let threshold = self.semantic_threshold;
        let mut matches = Self::autocomplete_name_corpus()
            .iter()
            .filter_map(|(name, lower)| {
                let rank = if lower == &query_lower {
                    0u8
                } else if lower.starts_with(&query_lower) {
                    1
                } else if lower
                    .split_whitespace()
                    .any(|word| word.starts_with(&query_lower))
                {
                    2
                } else if lower.contains(&query_lower) {
                    3
                } else {
                    return None;
                };

                if threshold > 0.0
                    && let Some(score) = Self::semantic_score_for_name(name.as_str())
                    && score < threshold
                {
                    return None;
                }

                Some((rank, name.len(), name))
            })
            .collect::<Vec<_>>();
        matches.sort_unstable_by(|left, right| {
            left.0
                .cmp(&right.0)
                .then_with(|| left.1.cmp(&right.1))
                .then_with(|| left.2.cmp(right.2))
        });
        let suggestions: Vec<String> = matches
            .into_iter()
            .take(capped_limit)
            .map(|(_, _, name)| name.clone())
            .collect();

        serde_wasm_bindgen::to_value(&suggestions)
            .map_err(|e| JsValue::from_str(&format!("autocompleteCardNames encode failed: {e}")))
    }

    /// Set a player's life total.
    #[wasm_bindgen(js_name = setLife)]
    pub fn set_life(&mut self, player_index: u8, life: i32) -> Result<(), JsValue> {
        let player_id = PlayerId::from_index(player_index);
        let Some(player) = self.game.player_mut(player_id) else {
            return Err(JsValue::from_str("invalid player index"));
        };
        player.life = life;
        self.recompute_ui_decision()?;
        Ok(())
    }

    /// Add a signed life delta (negative = damage, positive = gain).
    #[wasm_bindgen(js_name = addLifeDelta)]
    pub fn add_life_delta(&mut self, player_index: u8, delta: i32) -> Result<(), JsValue> {
        let player_id = PlayerId::from_index(player_index);
        let Some(player) = self.game.player_mut(player_id) else {
            return Err(JsValue::from_str("invalid player index"));
        };
        player.life += delta;
        self.recompute_ui_decision()?;
        Ok(())
    }

    /// Draw one card for a player.
    #[wasm_bindgen(js_name = drawCard)]
    pub fn draw_card(&mut self, player_index: u8) -> Result<usize, JsValue> {
        let player_id = PlayerId::from_index(player_index);
        if self.game.player(player_id).is_none() {
            return Err(JsValue::from_str("invalid player index"));
        }
        let drawn = self.game.draw_cards(player_id, 1);
        self.recompute_ui_decision()?;
        Ok(drawn.len())
    }

    /// Add a specific card by name to a player's hand.
    #[wasm_bindgen(js_name = addCardToHand)]
    pub fn add_card_to_hand(
        &mut self,
        player_index: u8,
        card_name: String,
    ) -> Result<u64, JsValue> {
        let player_id = PlayerId::from_index(player_index);
        if self.game.player(player_id).is_none() {
            return Err(JsValue::from_str("invalid player index"));
        }

        let query = card_name.trim();
        if query.is_empty() {
            return Err(JsValue::from_str("card name cannot be empty"));
        }

        self.registry.ensure_cards_loaded([query]);
        let definition = self.load_compilable_card_definition(query)?;

        let object_id = self.game.create_object_from_definition(
            &definition,
            player_id,
            crate::zone::Zone::Hand,
        );
        self.recompute_ui_decision()?;
        Ok(object_id.0)
    }

    /// Add a specific card by name to a player's zone.
    ///
    /// When `skip_triggers` is true the card is placed directly without
    /// processing ETB or other zone-change triggers.
    #[wasm_bindgen(js_name = addCardToZone)]
    pub fn add_card_to_zone(
        &mut self,
        player_index: u8,
        card_name: String,
        zone_name: String,
        skip_triggers: bool,
    ) -> Result<u64, JsValue> {
        let player_id = PlayerId::from_index(player_index);
        if self.game.player(player_id).is_none() {
            return Err(JsValue::from_str("invalid player index"));
        }

        let query = card_name.trim();
        if query.is_empty() {
            return Err(JsValue::from_str("card name cannot be empty"));
        }

        let zone = match zone_name.trim().to_lowercase().as_str() {
            "hand" => crate::zone::Zone::Hand,
            "battlefield" => crate::zone::Zone::Battlefield,
            "graveyard" => crate::zone::Zone::Graveyard,
            "exile" => crate::zone::Zone::Exile,
            "library" => crate::zone::Zone::Library,
            "command" => crate::zone::Zone::Command,
            other => {
                return Err(JsValue::from_str(&format!("unknown zone: {other}")));
            }
        };

        self.registry.ensure_cards_loaded([query]);
        let definition = self.load_compilable_card_definition(query)?;

        if self.semantic_threshold > 0.0
            && let Some(score) = Self::semantic_score_for_name(definition.name())
            && score < self.semantic_threshold
        {
            return Err(JsValue::from_str(&format!(
                "Card '{}' has fidelity {:.0}%, below threshold {:.0}%",
                definition.name(),
                score * 100.0,
                self.semantic_threshold * 100.0,
            )));
        }

        if skip_triggers {
            let object_id = self
                .game
                .create_object_from_definition(&definition, player_id, zone);
            self.recompute_ui_decision()?;
            Ok(object_id.0)
        } else {
            // Create in Command zone first, then move to target zone so that
            // zone-change triggers (ETB, etc.) fire naturally.
            let temp_id = self.game.create_object_from_definition(
                &definition,
                player_id,
                crate::zone::Zone::Command,
            );
            let object_id = if zone == crate::zone::Zone::Battlefield {
                let mut dm = crate::decision::SelectFirstDecisionMaker;
                let Some(result) = self.game.move_object_with_etb_processing_with_dm(
                    temp_id,
                    crate::zone::Zone::Battlefield,
                    &mut dm,
                ) else {
                    self.game.remove_object(temp_id);
                    return Err(JsValue::from_str(
                        "battlefield entry was prevented by replacement effect",
                    ));
                };

                let entered_id = result.new_id;
                let entered_tapped = result.enters_tapped;
                let entered_battlefield = self
                    .game
                    .object(entered_id)
                    .is_some_and(|obj| obj.zone == crate::zone::Zone::Battlefield);
                if entered_battlefield {
                    let etb_event_provenance = self
                        .game
                        .provenance_graph
                        .alloc_root_event(crate::events::EventKind::EnterBattlefield);
                    let event = if entered_tapped {
                        crate::triggers::TriggerEvent::new_with_provenance(
                            crate::events::EnterBattlefieldEvent::tapped(
                                entered_id,
                                crate::zone::Zone::Command,
                            ),
                            etb_event_provenance,
                        )
                    } else {
                        crate::triggers::TriggerEvent::new_with_provenance(
                            crate::events::EnterBattlefieldEvent::new(
                                entered_id,
                                crate::zone::Zone::Command,
                            ),
                            etb_event_provenance,
                        )
                    };
                    self.game.queue_trigger_event(etb_event_provenance, event);

                    crate::game_loop::drain_pending_trigger_events(
                        &mut self.game,
                        &mut self.trigger_queue,
                    );

                    if self
                        .game
                        .object(entered_id)
                        .is_some_and(|obj| obj.subtypes.contains(&Subtype::Saga))
                    {
                        crate::game_loop::add_lore_counter_and_check_chapters(
                            &mut self.game,
                            entered_id,
                            &mut self.trigger_queue,
                        );
                    }
                }

                entered_id
            } else {
                self.game.move_object(temp_id, zone).unwrap_or(temp_id)
            };
            crate::game_loop::drain_pending_trigger_events(&mut self.game, &mut self.trigger_queue);
            self.recompute_ui_decision()?;
            Ok(object_id.0)
        }
    }

    /// Draw opening hands for all players.
    #[wasm_bindgen(js_name = drawOpeningHands)]
    pub fn draw_opening_hands(&mut self, cards_per_player: usize) -> Result<(), JsValue> {
        let player_ids: Vec<PlayerId> = self.game.players.iter().map(|p| p.id).collect();
        for player_id in player_ids {
            let _ = self.game.draw_cards(player_id, cards_per_player);
        }
        self.recompute_ui_decision()?;
        Ok(())
    }

    /// Replace game state with demo decks and no battlefield/stack state.
    #[wasm_bindgen(js_name = loadDemoDecks)]
    pub fn load_demo_decks(&mut self) -> Result<(), JsValue> {
        let names: Vec<String> = self.game.players.iter().map(|p| p.name.clone()).collect();
        let starting_life = self.game.players.first().map_or(20, |p| p.life);
        self.initialize_empty_match(names, starting_life, self.generate_match_seed());
        self.populate_demo_libraries()?;
        self.finish_match_setup(7)
    }

    /// Load explicit decks by card name. JS format: `string[][]`.
    ///
    /// Deck list index maps to player index.
    /// Returns a JSON object with total and categorized failures:
    /// `{ loaded, failed, failedBelowThreshold, failedToParse }`.
    /// Unknown cards are skipped rather than aborting the entire load.
    #[wasm_bindgen(js_name = loadDecks)]
    pub fn load_decks(&mut self, decks_js: JsValue) -> Result<JsValue, JsValue> {
        let decks: Vec<Vec<String>> = serde_wasm_bindgen::from_value(decks_js)
            .map_err(|e| JsValue::from_str(&format!("invalid decks payload: {e}")))?;

        if decks.len() != self.game.players.len() {
            return Err(JsValue::from_str(
                "deck count must match number of players in game",
            ));
        }

        let names: Vec<String> = self.game.players.iter().map(|p| p.name.clone()).collect();
        let starting_life = self.game.players.first().map_or(20, |p| p.life);
        self.initialize_empty_match(names, starting_life, self.generate_match_seed());

        let mut loaded: u32 = 0;
        let mut failed: Vec<String> = Vec::new();
        let mut failed_below_threshold: Vec<String> = Vec::new();
        let mut failed_to_parse: Vec<String> = Vec::new();

        let player_ids: Vec<PlayerId> = self.game.players.iter().map(|p| p.id).collect();
        for (&player_id, deck) in player_ids.iter().zip(decks.iter()) {
            self.registry
                .ensure_cards_loaded(deck.iter().map(|name| name.as_str()));

            for name in deck {
                if let Some(definition) = self.find_card_definition(name).cloned() {
                    if self.semantic_threshold > 0.0
                        && let Some(score) = Self::semantic_score_for_name(definition.name())
                        && score < self.semantic_threshold
                    {
                        failed.push(name.clone());
                        failed_below_threshold.push(name.clone());
                        continue;
                    }
                    self.game.create_object_from_definition(
                        &definition,
                        player_id,
                        crate::zone::Zone::Library,
                    );
                    loaded += 1;
                } else {
                    failed.push(name.clone());
                    failed_to_parse.push(name.clone());
                }
            }

            self.game.shuffle_player_library(player_id);
        }

        self.finish_match_setup(7)?;

        serde_wasm_bindgen::to_value(&DeckLoadResult {
            loaded,
            failed,
            failed_below_threshold,
            failed_to_parse,
        })
        .map_err(|e| JsValue::from_str(&format!("failed to serialize deck load result: {e}")))
    }

    #[wasm_bindgen(js_name = cardLoadDiagnostics)]
    pub fn card_load_diagnostics(
        &mut self,
        card_name: String,
        error_message: Option<String>,
    ) -> Result<JsValue, JsValue> {
        let diagnostics = self.build_card_load_diagnostics(&card_name, error_message.as_deref());
        serde_wasm_bindgen::to_value(&diagnostics)
            .map_err(|e| JsValue::from_str(&format!("failed to serialize card diagnostics: {e}")))
    }

    /// Advance to next phase (or next turn if ending phase).
    /// Resets the TurnRunner so it picks up from the new game state.
    #[wasm_bindgen(js_name = advancePhase)]
    pub fn advance_phase(&mut self) -> Result<(), JsValue> {
        crate::turn::advance_step(&mut self.game)
            .map_err(|e| JsValue::from_str(&format!("advance_step failed: {e:?}")))?;
        self.runner = None;
        self.runner_awaiting_priority = false;
        self.runner_pending_decision = false;
        self.recompute_ui_decision()?;
        Ok(())
    }

    /// Toggle automatic cleanup discard (random cards).
    #[wasm_bindgen(js_name = setAutoCleanupDiscard)]
    pub fn set_auto_cleanup_discard(&mut self, enabled: bool) {
        self.auto_cleanup_discard = enabled;
    }

    /// Set the semantic similarity threshold for card addition (0..100%, 0 = off).
    #[wasm_bindgen(js_name = setSemanticThreshold)]
    pub fn set_semantic_threshold(&mut self, threshold: f32) {
        self.semantic_threshold = (threshold / 100.0).clamp(0.0, 1.0);
    }

    /// Get the current semantic threshold as percentage points.
    #[wasm_bindgen(js_name = getSemanticThreshold)]
    pub fn get_semantic_threshold(&self) -> f32 {
        self.semantic_threshold * 100.0
    }

    /// Get the semantic score for a specific card. Returns -1.0 if score is unavailable.
    #[wasm_bindgen(js_name = getCardSemanticScore)]
    pub fn get_card_semantic_score(&self, card_name: &str) -> f32 {
        Self::semantic_score_for_name(card_name).unwrap_or(-1.0)
    }

    /// Get the count of scored cards meeting the current threshold.
    #[wasm_bindgen(js_name = cardsMeetingThreshold)]
    pub fn cards_meeting_threshold(&self) -> usize {
        if self.semantic_threshold <= 0.0 {
            return CardRegistry::generated_parser_semantic_scored_count();
        }
        let threshold_counts = CardRegistry::generated_parser_semantic_threshold_counts();
        let threshold_index = ((self.semantic_threshold * 100.0).ceil() as usize)
            .clamp(1, threshold_counts.len())
            - 1;
        threshold_counts[threshold_index]
    }

    /// Switch local perspective to the next player.
    #[wasm_bindgen(js_name = switchPerspective)]
    pub fn switch_perspective(&mut self) -> Result<u8, JsValue> {
        let current_index = self
            .game
            .players
            .iter()
            .position(|p| p.id == self.perspective)
            .unwrap_or(0);
        let next_index = (current_index + 1) % self.game.players.len().max(1);
        self.perspective = self.game.players[next_index].id;
        Ok(self.perspective.0)
    }

    /// Set local perspective explicitly.
    #[wasm_bindgen(js_name = setPerspective)]
    pub fn set_perspective(&mut self, player_index: u8) -> Result<(), JsValue> {
        let pid = PlayerId::from_index(player_index);
        if self.game.player(pid).is_none() {
            return Err(JsValue::from_str("invalid player index"));
        }
        self.perspective = pid;
        Ok(())
    }

    /// Cancel the current pending decision chain.
    ///
    /// Rollback preference:
    /// 1. The active user-action checkpoint (start of this spell/ability chain).
    /// 2. The active replay-action checkpoint (for speculative nested prompts).
    /// 3. The priority-epoch checkpoint (start of this priority round).
    ///
    /// This mirrors "take back this action chain" behavior first, while still
    /// preserving the broader epoch rollback as a fallback.
    #[wasm_bindgen(js_name = cancelDecision)]
    pub fn cancel_decision(&mut self) -> Result<JsValue, JsValue> {
        if let Some(checkpoint) = self.pending_action_checkpoint.as_ref().cloned() {
            self.restore_replay_checkpoint(&checkpoint);
        } else if let Some(checkpoint) = self
            .pending_replay_action
            .as_ref()
            .map(|replay| replay.checkpoint.clone())
        {
            self.restore_replay_checkpoint(&checkpoint);
        } else if let Some(epoch) = self.priority_epoch_checkpoint.as_ref().cloned() {
            self.restore_replay_checkpoint(&epoch);
        }
        self.pending_decision = None;
        self.pending_replay_action = None;
        self.pending_action_checkpoint = None;
        self.pending_live_action_root = None;
        self.pending_live_continuation = None;
        self.priority_epoch_has_undoable_action = false;
        self.priority_epoch_undo_locked_by_mana = false;
        self.priority_epoch_undo_land_stable_id = None;
        self.active_viewed_cards = None;
        self.clear_active_resolving_stack_object();
        self.recompute_ui_decision()?;
        self.snapshot()
    }

    /// Apply a player command for the currently pending decision.
    #[wasm_bindgen]
    pub fn dispatch(&mut self, command: JsValue) -> Result<JsValue, JsValue> {
        let command: UiCommand = serde_wasm_bindgen::from_value(command)
            .map_err(|e| JsValue::from_str(&format!("invalid command payload: {e}")))?;
        self.clear_active_resolving_stack_object();

        let pending_ctx = self
            .pending_decision
            .take()
            .ok_or_else(|| JsValue::from_str("no pending decision to dispatch"))?;

        if self.pregame.is_some() {
            return self.dispatch_pregame_decision(pending_ctx, command);
        }

        // If this decision came from the TurnRunner, route through runner.respond_*()
        if self.runner_pending_decision {
            self.runner_pending_decision = false;
            return self.dispatch_runner_decision(pending_ctx, command);
        }

        if self.pending_live_continuation.is_some() {
            return self.dispatch_live_priority_continuation(pending_ctx, command);
        }

        if self.pending_replay_action.is_none()
            && self.decision_uses_live_priority_response(&pending_ctx)
        {
            return self.dispatch_live_priority_response(pending_ctx, command);
        }

        if let Some(mut replay) = self.pending_replay_action.take() {
            let answer = match self.command_to_replay_answer(&pending_ctx, command.clone()) {
                Ok(answer) => answer,
                Err(err) => {
                    self.pending_decision = Some(pending_ctx);
                    self.pending_replay_action = Some(replay);
                    return Err(err);
                }
            };
            replay.nested_answers.push(answer);
            let should_track_action_checkpoint = self.pending_action_checkpoint.is_none()
                && Self::replay_answers_start_cancelable_action_chain(
                    &replay.root,
                    &replay.nested_answers,
                );
            let live_checkpoint = self.capture_replay_checkpoint();
            let progress = if self.decision_requires_root_reexecution(&pending_ctx) {
                match self.execute_with_replay(
                    &replay.checkpoint,
                    &replay.root,
                    &replay.nested_answers,
                ) {
                    Ok(ReplayOutcome::NeedsDecision(next_ctx)) => {
                        if should_track_action_checkpoint {
                            self.pending_action_checkpoint = Some(replay.checkpoint.clone());
                        }
                        self.pending_decision = Some(next_ctx);
                        self.pending_replay_action = Some(replay);
                        return self.snapshot();
                    }
                    Ok(ReplayOutcome::Complete(progress)) => progress,
                    Err(err) => {
                        self.restore_replay_checkpoint(&live_checkpoint);
                        self.pending_decision = Some(pending_ctx);
                        self.pending_replay_action = Some(replay);
                        return Err(err);
                    }
                }
            } else {
                let response = match self.command_to_response(&pending_ctx, command) {
                    Ok(response) => response,
                    Err(err) => {
                        self.pending_decision = Some(pending_ctx);
                        self.pending_replay_action = Some(replay);
                        return Err(err);
                    }
                };
                let mut live_dm = WasmReplayDecisionMaker::new(&[]);
                let result = apply_priority_response_with_dm(
                    &mut self.game,
                    &mut self.trigger_queue,
                    &mut self.priority_state,
                    &response,
                    &mut live_dm,
                );
                let (pending_context, viewed_cards) = live_dm.finish();
                self.active_viewed_cards = viewed_cards;

                if let Some(next_ctx) = pending_context {
                    self.sync_active_resolving_stack_object_for_prompt(Some(&live_checkpoint));
                    if should_track_action_checkpoint {
                        self.pending_action_checkpoint = Some(replay.checkpoint.clone());
                    }
                    self.pending_decision = Some(next_ctx);
                    self.pending_replay_action = Some(replay);
                    return self.snapshot();
                }

                match result {
                    Ok(progress) => progress,
                    Err(err) => {
                        self.restore_replay_checkpoint(&live_checkpoint);
                        self.pending_decision = Some(pending_ctx);
                        self.pending_replay_action = Some(replay);
                        return Err(JsValue::from_str(&format!("dispatch failed: {err}")));
                    }
                }
            };

            match progress {
                GameProgress::NeedsDecisionCtx(next_ctx) => {
                    self.sync_active_resolving_stack_object_for_prompt(Some(&live_checkpoint));
                    if self.priority_action_chain_still_pending() {
                        if should_track_action_checkpoint {
                            self.pending_action_checkpoint = Some(replay.checkpoint.clone());
                        }
                        self.pending_decision = Some(next_ctx);
                        self.pending_replay_action = Some(replay);
                        return self.snapshot();
                    }

                    // The spell/ability is now committed. Follow-up prompts
                    // produced during resolution must not preserve Undo for
                    // the action that just finished paying its costs.
                    self.pending_action_checkpoint = None;
                    self.pending_decision = Some(next_ctx);
                    self.pending_replay_action = Some(replay);
                    self.snapshot()
                }
                progress => {
                    self.clear_active_resolving_stack_object();
                    if Self::replay_root_starts_undoable_action(&replay.root) {
                        self.priority_epoch_has_undoable_action = true;
                    }
                    if self.replay_chain_has_irreversible_mana_activation(&replay)
                        || self.replay_root_mana_activation_added_to_stack(
                            &replay.checkpoint,
                            &replay.root,
                        )
                    {
                        self.priority_epoch_undo_locked_by_mana = true;
                    }
                    self.priority_epoch_undo_land_stable_id =
                        self.committed_undo_land_stable_id(&replay.checkpoint, &replay.root);
                    self.pending_action_checkpoint = None;
                    self.pending_replay_action = None;
                    self.apply_progress(progress)?;
                    self.snapshot()
                }
            }
        } else {
            let response = match self.command_to_response(&pending_ctx, command) {
                Ok(response) => response,
                Err(err) => {
                    self.pending_decision = Some(pending_ctx);
                    return Err(err);
                }
            };

            let checkpoint = self.capture_replay_checkpoint();
            let should_track_action_checkpoint = self.pending_action_checkpoint.is_none()
                && Self::response_starts_cancelable_action_chain(&response);
            let root = ReplayRoot::Response(response);
            let outcome = match self.execute_with_replay(&checkpoint, &root, &[]) {
                Ok(outcome) => outcome,
                Err(err) => {
                    self.pending_decision = Some(pending_ctx);
                    self.pending_replay_action = None;
                    return Err(err);
                }
            };
            match outcome {
                ReplayOutcome::NeedsDecision(next_ctx) => {
                    self.sync_active_resolving_stack_object_for_prompt(Some(&checkpoint));
                    if should_track_action_checkpoint {
                        self.pending_action_checkpoint = Some(checkpoint.clone());
                    }
                    self.pending_decision = Some(next_ctx);
                    self.pending_replay_action = Some(PendingReplayAction {
                        checkpoint,
                        root,
                        nested_answers: Vec::new(),
                    });
                    self.snapshot()
                }
                ReplayOutcome::Complete(progress) => {
                    match progress {
                        GameProgress::NeedsDecisionCtx(next_ctx) => {
                            self.sync_active_resolving_stack_object_for_prompt(Some(&checkpoint));
                            if self.priority_action_chain_still_pending() {
                                if should_track_action_checkpoint {
                                    self.pending_action_checkpoint = Some(checkpoint.clone());
                                }
                                self.pending_decision = Some(next_ctx);
                                self.pending_replay_action = Some(PendingReplayAction {
                                    checkpoint,
                                    root,
                                    nested_answers: Vec::new(),
                                });
                                return self.snapshot();
                            }

                            // The spell/ability is now committed. Follow-up prompts
                            // produced during resolution must not preserve Undo for
                            // the action that just finished paying its costs.
                            self.pending_action_checkpoint = None;
                            self.pending_decision = Some(next_ctx);
                            self.pending_replay_action = Some(PendingReplayAction {
                                checkpoint,
                                root,
                                nested_answers: Vec::new(),
                            });
                            self.snapshot()
                        }
                        progress => {
                            self.clear_active_resolving_stack_object();
                            if Self::replay_root_starts_undoable_action(&root) {
                                self.priority_epoch_has_undoable_action = true;
                            }
                            if Self::replay_root_has_irreversible_mana_activation(
                                &checkpoint.game,
                                &root,
                            ) || self
                                .replay_root_mana_activation_added_to_stack(&checkpoint, &root)
                            {
                                self.priority_epoch_undo_locked_by_mana = true;
                            }
                            self.priority_epoch_undo_land_stable_id =
                                self.committed_undo_land_stable_id(&checkpoint, &root);
                            self.pending_action_checkpoint = None;
                            self.pending_replay_action = None;
                            self.apply_progress(progress)?;
                            self.snapshot()
                        }
                    }
                }
            }
        }
    }
}

impl WasmGame {
    /// Whether the current decision can be cancelled.
    ///
    /// Cancel is intentionally conservative:
    /// - only user-initiated replay chains are cancelable;
    /// - priority pass commits the action chain and disables cancel;
    /// - playing a land commits the action chain and disables cancel;
    /// - hidden-information/library changes that cannot be safely reversed
    ///   (shuffle/reorder, draw/mill/exile-from-library, etc.) disable cancel.
    /// - mana-ability activations with non-mana game-state side effects lock undo
    ///   once they are committed.
    ///
    /// While a decision prompt is still open, we allow undo back to the replay
    /// checkpoint even if the in-progress chain includes an irreversible mana ability.
    fn is_cancelable(&self) -> bool {
        if let Some(replay) = self.pending_replay_action.as_ref() {
            return self.is_replay_chain_cancelable(replay);
        }

        if self.pending_action_checkpoint.is_none()
            && self
                .pending_decision
                .as_ref()
                .is_some_and(|ctx| !matches!(ctx, DecisionContext::Priority(_)))
        {
            return false;
        }

        if let Some(checkpoint) = self.pending_action_checkpoint.as_ref() {
            return !self.has_irreversible_mana_undo_lock()
                && !self.has_irreversible_library_change_since(checkpoint)
                && !self.has_irreversible_random_change_since(checkpoint);
        }

        let Some(epoch) = self.priority_epoch_checkpoint.as_ref() else {
            return false;
        };

        self.priority_epoch_has_undoable_action
            && !self.has_irreversible_mana_undo_lock()
            && !self.has_land_play_since(epoch)
            && !self.has_irreversible_library_change_since(epoch)
            && !self.has_irreversible_random_change_since(epoch)
    }

    fn response_starts_cancelable_action_chain(response: &PriorityResponse) -> bool {
        match response {
            PriorityResponse::PriorityAction(action) => {
                Self::priority_action_starts_cancelable_action_chain(action)
            }
            _ => false,
        }
    }

    fn priority_action_starts_cancelable_action_chain(action: &LegalAction) -> bool {
        !matches!(
            action,
            LegalAction::PassPriority
                | LegalAction::PlayLand { .. }
                | LegalAction::KeepOpeningHand
                | LegalAction::TakeMulligan
                | LegalAction::SerumPowderMulligan { .. }
                | LegalAction::ContinuePregame
                | LegalAction::BeginGame
                | LegalAction::UsePregameAction { .. }
        )
    }

    fn replay_answers_start_cancelable_action_chain(
        root: &ReplayRoot,
        nested_answers: &[ReplayDecisionAnswer],
    ) -> bool {
        match root {
            ReplayRoot::Response(response) => {
                Self::response_starts_cancelable_action_chain(response)
            }
            ReplayRoot::Advance => matches!(
                nested_answers.first(),
                Some(ReplayDecisionAnswer::Priority(action))
                    if Self::priority_action_starts_cancelable_action_chain(action)
            ),
        }
    }

    fn is_replay_chain_cancelable(&self, replay: &PendingReplayAction) -> bool {
        let ReplayRoot::Response(response) = &replay.root else {
            return false;
        };

        if matches!(
            response,
            PriorityResponse::PriorityAction(LegalAction::PassPriority)
        ) {
            return false;
        }

        if replay.nested_answers.iter().any(|answer| {
            matches!(
                answer,
                ReplayDecisionAnswer::Priority(LegalAction::PassPriority)
            )
        }) {
            return false;
        }

        if self.has_irreversible_mana_undo_lock() {
            return false;
        }

        if self.has_land_play_since(&replay.checkpoint) {
            return false;
        }

        if self.pending_decision.is_none()
            && self.replay_chain_has_irreversible_mana_activation(replay)
        {
            return false;
        }

        !self.has_irreversible_library_change_since(&replay.checkpoint)
            && !self.has_irreversible_random_change_since(&replay.checkpoint)
    }

    fn priority_action_chain_still_pending(&self) -> bool {
        self.priority_state.pending_cast.is_some()
            || self.priority_state.pending_activation.is_some()
            || self.priority_state.pending_mana_ability.is_some()
            || self.priority_state.pending_method_selection.is_some()
            || self.priority_state.pending_continuation.is_some()
    }

    fn select_objects_uses_live_priority_response(&self) -> bool {
        self.priority_state
            .pending_activation
            .as_ref()
            .is_some_and(|pending| {
                matches!(
                    pending.stage,
                    ActivationStage::ChoosingSacrifice | ActivationStage::ChoosingCardCost
                )
            })
            || self
                .priority_state
                .pending_cast
                .as_ref()
                .is_some_and(|pending| {
                    matches!(
                        pending.stage,
                        CastStage::ChoosingSacrifice | CastStage::ChoosingCardCost
                    )
                })
    }

    fn decision_requires_root_reexecution(&self, ctx: &DecisionContext) -> bool {
        replay_decision_requires_root_reexecution(ctx)
            || matches!(ctx, DecisionContext::SelectObjects(_))
                && !self.select_objects_uses_live_priority_response()
    }

    fn select_options_uses_live_priority_response(
        &self,
        _ctx: &crate::decisions::context::SelectOptionsContext,
    ) -> bool {
        self.game.pending_replacement_choice.is_some()
            || self.priority_state.pending_method_selection.is_some()
            || self.priority_state.pending_mana_ability.is_some()
            || self
                .priority_state
                .pending_cast
                .as_ref()
                .is_some_and(|pending| {
                    matches!(
                        pending.stage,
                        CastStage::ChoosingOptionalCosts
                            | CastStage::ChoosingNextCost
                            | CastStage::PayingMana
                    )
                })
            || self
                .priority_state
                .pending_activation
                .as_ref()
                .is_some_and(|pending| {
                    matches!(
                        pending.stage,
                        ActivationStage::ChoosingNextCost | ActivationStage::PayingMana
                    )
                })
    }

    fn decision_uses_live_priority_response(&self, ctx: &DecisionContext) -> bool {
        if self.priority_state.pending_continuation.is_some() {
            return true;
        }

        match ctx {
            DecisionContext::Priority(_)
            | DecisionContext::Number(_)
            | DecisionContext::Modes(_)
            | DecisionContext::HybridChoice(_)
            | DecisionContext::Targets(_) => true,
            DecisionContext::SelectOptions(ctx) => {
                self.select_options_uses_live_priority_response(ctx)
            }
            DecisionContext::SelectObjects(_) => self.select_objects_uses_live_priority_response(),
            _ => false,
        }
    }

    fn has_irreversible_mana_undo_lock(&self) -> bool {
        if self.priority_epoch_undo_locked_by_mana {
            return true;
        }

        // In-flight locks should not hide Undo while the user is still resolving
        // a prompt in the current action chain. The lock is latched at epoch level
        // when that chain commits.
        if self.pending_decision.is_some() {
            return false;
        }

        self.priority_state
            .pending_cast
            .as_ref()
            .is_some_and(|pending| pending.undo_locked_by_mana)
            || self
                .priority_state
                .pending_activation
                .as_ref()
                .is_some_and(|pending| pending.undo_locked_by_mana)
            || self
                .priority_state
                .pending_mana_ability
                .as_ref()
                .is_some_and(|pending| pending.undo_locked_by_mana)
    }

    fn visible_undo_land_stable_id(&self, cancelable: bool) -> Option<u64> {
        if !cancelable {
            return None;
        }

        let Some(DecisionContext::Priority(ctx)) = self.pending_decision.as_ref() else {
            return None;
        };
        if ctx.player != self.perspective {
            return None;
        }

        self.priority_epoch_undo_land_stable_id
    }

    fn replay_root_has_irreversible_mana_activation(game: &GameState, root: &ReplayRoot) -> bool {
        if let ReplayRoot::Response(PriorityResponse::PriorityAction(action)) = root {
            return Self::legal_action_has_irreversible_mana_ability(game, action);
        }
        false
    }

    fn replay_root_starts_undoable_action(root: &ReplayRoot) -> bool {
        match root {
            ReplayRoot::Response(PriorityResponse::PriorityAction(LegalAction::PassPriority)) => {
                false
            }
            ReplayRoot::Response(_) => true,
            ReplayRoot::Advance => false,
        }
    }

    fn replay_root_is_mana_activation(root: &ReplayRoot) -> bool {
        matches!(
            root,
            ReplayRoot::Response(PriorityResponse::PriorityAction(
                LegalAction::ActivateManaAbility { .. }
            ))
        )
    }

    fn replay_root_land_mana_source_stable_id(game: &GameState, root: &ReplayRoot) -> Option<u64> {
        let ReplayRoot::Response(PriorityResponse::PriorityAction(
            LegalAction::ActivateManaAbility { source, .. },
        )) = root
        else {
            return None;
        };

        let object = game.object(*source)?;
        object
            .has_card_type(CardType::Land)
            .then_some(object.stable_id.0.0)
    }

    fn stack_grew_since(&self, checkpoint: &ReplayCheckpoint) -> bool {
        self.game.stack.len() > checkpoint.game.stack.len()
    }

    fn replay_root_mana_activation_added_to_stack(
        &self,
        checkpoint: &ReplayCheckpoint,
        root: &ReplayRoot,
    ) -> bool {
        Self::replay_root_is_mana_activation(root) && self.stack_grew_since(checkpoint)
    }

    fn committed_undo_land_stable_id(
        &self,
        checkpoint: &ReplayCheckpoint,
        root: &ReplayRoot,
    ) -> Option<u64> {
        if Self::replay_root_has_irreversible_mana_activation(&checkpoint.game, root)
            || self.replay_root_mana_activation_added_to_stack(checkpoint, root)
        {
            return None;
        }

        Self::replay_root_land_mana_source_stable_id(&checkpoint.game, root)
    }

    fn replay_chain_has_irreversible_mana_activation(&self, replay: &PendingReplayAction) -> bool {
        if Self::replay_root_has_irreversible_mana_activation(&replay.checkpoint.game, &replay.root)
        {
            return true;
        }

        replay.nested_answers.iter().any(|answer| {
            if let ReplayDecisionAnswer::Priority(action) = answer {
                return Self::legal_action_has_irreversible_mana_ability(
                    &replay.checkpoint.game,
                    action,
                );
            }
            false
        })
    }

    fn has_land_play_since(&self, checkpoint: &ReplayCheckpoint) -> bool {
        for before_player in &checkpoint.game.players {
            let Some(after_player) = self.game.player(before_player.id) else {
                return true;
            };
            if after_player.lands_played_this_turn > before_player.lands_played_this_turn {
                return true;
            }
        }
        false
    }

    fn legal_action_has_irreversible_mana_ability(game: &GameState, action: &LegalAction) -> bool {
        let LegalAction::ActivateManaAbility {
            source,
            ability_index,
        } = action
        else {
            return false;
        };

        !crate::game_loop::mana_ability_is_undo_safe(game, *source, *ability_index)
    }

    /// Returns true when the current game diverged from `checkpoint` in a way
    /// that should not be silently rewound (hidden-information/library changes).
    ///
    /// Allowed library delta:
    /// - removing cards from library only when those cards are currently on stack
    /// - preserving relative order of remaining library cards
    ///
    /// Everything else is treated as irreversible for cancel purposes.
    fn has_irreversible_library_change_since(&self, checkpoint: &ReplayCheckpoint) -> bool {
        for before_player in &checkpoint.game.players {
            let Some(after_player) = self.game.player(before_player.id) else {
                return true;
            };

            let before_library = &before_player.library;
            let after_library = &after_player.library;

            if before_library == after_library {
                continue;
            }

            // Cards moving into library are not safely reversible (includes
            // "put into library" effects and many reorder/shuffle outcomes).
            if after_library.len() > before_library.len() {
                return true;
            }

            let after_set: HashSet<ObjectId> = after_library.iter().copied().collect();
            let removed: HashSet<ObjectId> = before_library
                .iter()
                .copied()
                .filter(|id| !after_set.contains(id))
                .collect();

            // Pure reorder/shuffle with no net removals.
            if removed.is_empty() {
                return true;
            }

            // Moving a card from library is only reversible when that card is
            // currently on stack.
            if removed
                .iter()
                .any(|id| !self.game.stack.iter().any(|entry| entry.object_id == *id))
            {
                return true;
            }

            let expected_after: Vec<ObjectId> = before_library
                .iter()
                .copied()
                .filter(|id| !removed.contains(id))
                .collect();

            if expected_after != *after_library {
                return true;
            }
        }

        false
    }

    fn has_irreversible_random_change_since(&self, checkpoint: &ReplayCheckpoint) -> bool {
        self.game.irreversible_random_count() != checkpoint.game.irreversible_random_count()
    }

    fn semantic_score_for_name(card_name: &str) -> Option<f32> {
        CardRegistry::generated_parser_semantic_score(card_name)
    }

    fn autocomplete_name_corpus() -> &'static [(String, String)] {
        AUTOCOMPLETE_CARD_NAMES.get_or_init(|| {
            let mut names = CardRegistry::generated_parser_card_names();
            names.sort_unstable();
            names.dedup();
            names
                .into_iter()
                .map(|name| {
                    let lower = name.to_lowercase();
                    (name, lower)
                })
                .collect()
        })
    }

    fn has_demo_supported_cost_symbols(cost: &crate::mana::ManaCost) -> bool {
        !cost.pips().iter().flatten().any(|symbol| {
            matches!(
                symbol,
                ManaSymbol::Colorless | ManaSymbol::Snow | ManaSymbol::Life(_) | ManaSymbol::X
            )
        })
    }

    fn is_strict_demo_spell_candidate(def: &CardDefinition) -> bool {
        if def.card.is_token || def.card.is_land() {
            return false;
        }
        // Keep startup-safe random decks to cards the current UI/decision loop
        // can consistently represent without hitting unsupported cast branches.
        if !def.alternative_casts.is_empty()
            || !def.optional_costs.is_empty()
            || def.additional_cost.has_non_mana_costs()
            || def.max_saga_chapter.is_some()
            || def.name().contains("//")
        {
            return false;
        }
        let Some(cost) = &def.card.mana_cost else {
            return false;
        };
        Self::has_demo_supported_cost_symbols(cost)
    }

    fn is_fallback_demo_spell_candidate(def: &CardDefinition) -> bool {
        if def.card.is_token || def.card.is_land() {
            return false;
        }
        match &def.card.mana_cost {
            Some(cost) => Self::has_demo_supported_cost_symbols(cost),
            None => true,
        }
    }

    fn build_random_demo_deck_names(
        &mut self,
        deck_size: usize,
        land_count: usize,
    ) -> Result<Vec<String>, JsValue> {
        if deck_size == 0 || land_count >= deck_size {
            return Err(JsValue::from_str(
                "invalid deck sizing (deck_size must be > 0 and land_count < deck_size)",
            ));
        }

        let spells_needed = deck_size - land_count;
        let mut rng = StdRng::seed_from_u64(self.next_deck_seed());

        // Keep demo setup JIT-friendly: only parse/register cards as they are sampled.
        let mut candidate_names = CardRegistry::generated_parser_card_names();
        candidate_names.shuffle(&mut rng);
        self.registry
            .ensure_cards_loaded(["Plains", "Island", "Swamp", "Mountain", "Forest"]);

        let mut strict_spell_pool: Vec<String> = Vec::new();
        let mut fallback_spell_pool: Vec<String> = Vec::new();
        let mut strict_seen: HashSet<String> = HashSet::new();
        let mut fallback_seen: HashSet<String> = HashSet::new();

        for candidate in candidate_names {
            // Skip cards below the semantic fidelity threshold before parsing.
            if self.semantic_threshold > 0.0
                && let Some(score) = Self::semantic_score_for_name(candidate.as_str())
                && score < self.semantic_threshold
            {
                continue;
            }
            self.registry.ensure_cards_loaded([candidate.as_str()]);
            let Some(def) = self.registry.get(candidate.as_str()) else {
                continue;
            };
            let canonical = def.name().to_string();
            let key = canonical.to_lowercase();
            if Self::is_strict_demo_spell_candidate(def) {
                if strict_seen.insert(key) {
                    strict_spell_pool.push(canonical);
                }
                if strict_spell_pool.len() >= spells_needed {
                    break;
                }
            } else if Self::is_fallback_demo_spell_candidate(def) && fallback_seen.insert(key) {
                fallback_spell_pool.push(canonical);
            }
        }

        let mut spell_pool: Vec<String> = if strict_spell_pool.is_empty() {
            fallback_spell_pool
        } else {
            strict_spell_pool
        };

        if spell_pool.is_empty() {
            return Err(JsValue::from_str(
                "registry has no nonland cards eligible for random deck generation",
            ));
        }

        spell_pool.shuffle(&mut rng);

        let mut spells: Vec<String> = Vec::with_capacity(spells_needed);
        if spell_pool.len() >= spells_needed {
            spells.extend(spell_pool.iter().take(spells_needed).cloned());
        } else {
            // If pool is smaller than requested spells, wrap and keep shuffling for variety.
            while spells.len() < spells_needed {
                spell_pool.shuffle(&mut rng);
                for card_name in &spell_pool {
                    spells.push(card_name.clone());
                    if spells.len() >= spells_needed {
                        break;
                    }
                }
            }
        }

        let mut symbol_counts: HashMap<ManaSymbol, u32> = HashMap::new();
        for card_name in &spells {
            if let Some(def) = self.registry.get(card_name)
                && let Some(cost) = &def.card.mana_cost
            {
                for pip in cost.pips() {
                    for symbol in pip {
                        match symbol {
                            ManaSymbol::White
                            | ManaSymbol::Blue
                            | ManaSymbol::Black
                            | ManaSymbol::Red
                            | ManaSymbol::Green => {
                                *symbol_counts.entry(*symbol).or_insert(0) += 1;
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        let mut deck = spells;
        let total_colored_symbols: u32 = symbol_counts.values().sum();
        let color_order = [
            ManaSymbol::White,
            ManaSymbol::Blue,
            ManaSymbol::Black,
            ManaSymbol::Red,
            ManaSymbol::Green,
        ];

        let mut assigned_lands = 0usize;
        for color in color_order {
            let count = symbol_counts.get(&color).copied().unwrap_or(0);
            if count == 0 || total_colored_symbols == 0 {
                continue;
            }
            let share = (count as f64 / total_colored_symbols as f64) * land_count as f64;
            let land_slots = share.round() as usize;
            let basic_name = Self::basic_land_name_for_symbol(color);
            if self.registry.get(basic_name).is_none() {
                continue;
            }
            for _ in 0..land_slots {
                deck.push(basic_name.to_string());
                assigned_lands += 1;
                if assigned_lands >= land_count {
                    break;
                }
            }
            if assigned_lands >= land_count {
                break;
            }
        }

        let fallback_color = symbol_counts
            .iter()
            .max_by_key(|(_, count)| *count)
            .map(|(color, _)| *color)
            .unwrap_or(ManaSymbol::Green);
        let mut fallback_land = Self::basic_land_name_for_symbol(fallback_color);
        if self.registry.get(fallback_land).is_none() {
            fallback_land = ["Plains", "Island", "Swamp", "Mountain", "Forest"]
                .into_iter()
                .find(|name| self.registry.get(name).is_some())
                .ok_or_else(|| {
                    JsValue::from_str("registry has no basic lands for demo manabase")
                })?;
        }
        while assigned_lands < land_count {
            deck.push(fallback_land.to_string());
            assigned_lands += 1;
        }

        deck.shuffle(&mut rng);
        Ok(deck)
    }

    fn next_deck_seed(&mut self) -> u64 {
        self.game.next_random_u64()
    }

    fn basic_land_name_for_symbol(symbol: ManaSymbol) -> &'static str {
        match symbol {
            ManaSymbol::White => "Plains",
            ManaSymbol::Blue => "Island",
            ManaSymbol::Black => "Swamp",
            ManaSymbol::Red => "Mountain",
            ManaSymbol::Green => "Forest",
            _ => "Forest",
        }
    }

    fn populate_player_library(
        &mut self,
        player_id: PlayerId,
        deck_names: &[String],
    ) -> Result<(), JsValue> {
        self.registry
            .ensure_cards_loaded(deck_names.iter().map(|name| name.as_str()));

        for name in deck_names {
            let Some(definition) = self.find_card_definition(name).cloned() else {
                return Err(JsValue::from_str(&format!("unknown card name: {name}")));
            };
            self.game.create_object_from_definition(
                &definition,
                player_id,
                crate::zone::Zone::Library,
            );
        }

        self.game.shuffle_player_library(player_id);
        Ok(())
    }

    fn find_card_definition(&self, query: &str) -> Option<&CardDefinition> {
        self.registry.get(query).or_else(|| {
            self.registry
                .all()
                .find(|def| def.name().eq_ignore_ascii_case(query))
        })
    }

    fn generated_parse_source_for_name(query: &str) -> Option<(String, String)> {
        CardRegistry::generated_parser_card_parse_source(query)
    }

    fn extract_oracle_text_from_parse_block(block: &str) -> Option<String> {
        let oracle_lines: Vec<&str> = block
            .lines()
            .filter(|line| {
                let trimmed = line.trim();
                !trimmed.starts_with("Mana cost:")
                    && !trimmed.starts_with("Type:")
                    && !trimmed.starts_with("Power/Toughness:")
                    && !trimmed.starts_with("Loyalty:")
                    && !trimmed.starts_with("Defense:")
            })
            .collect();
        let oracle_text = oracle_lines.join("\n").trim().to_string();
        if oracle_text.is_empty() {
            None
        } else {
            Some(oracle_text)
        }
    }

    fn compile_definition_from_parse_source(
        source_name: &str,
        parse_block: &str,
    ) -> Result<CardDefinition, String> {
        crate::cards::CardDefinitionBuilder::new(crate::ids::CardId::new(), source_name)
            .parse_text(parse_block.to_string())
            .map_err(|err| format!("{err:?}"))
    }

    fn compiled_ability_lines(definition: &CardDefinition) -> Vec<String> {
        definition
            .abilities
            .iter()
            .enumerate()
            .map(|(index, ability)| {
                let text = match &ability.kind {
                    crate::ability::AbilityKind::Static(static_ability) => static_ability.display(),
                    crate::ability::AbilityKind::Triggered(triggered) => {
                        let trigger = triggered.trigger.display();
                        let effects = if triggered.effects.is_empty() {
                            String::new()
                        } else {
                            crate::compiled_text::compile_effect_list(&triggered.effects)
                        };
                        if effects.trim().is_empty() {
                            trigger
                        } else {
                            format!("{trigger} -> {effects}")
                        }
                    }
                    crate::ability::AbilityKind::Activated(activated) => {
                        let cost = activated.mana_cost.display();
                        let resolution = if let Some(mana) = &activated.mana_output {
                            if mana.is_empty() {
                                crate::compiled_text::compile_effect_list(&activated.effects)
                            } else {
                                format!(
                                    "Add {}",
                                    crate::mana::ManaCost::from_symbols(mana.clone()).to_oracle()
                                )
                            }
                        } else {
                            crate::compiled_text::compile_effect_list(&activated.effects)
                        };

                        match (cost.trim().is_empty(), resolution.trim().is_empty()) {
                            (true, true) => "Activated ability".to_string(),
                            (false, true) => cost,
                            (true, false) => resolution,
                            (false, false) => format!("{cost} -> {resolution}"),
                        }
                    }
                };
                format!("Ability {}: {}", index + 1, text)
            })
            .collect()
    }

    fn build_card_load_diagnostics(
        &mut self,
        card_name: &str,
        explicit_error: Option<&str>,
    ) -> CardLoadDiagnostics {
        let query = card_name.trim();
        let parse_source = if query.is_empty() {
            None
        } else {
            Self::generated_parse_source_for_name(query)
        };
        let source_compile_result = parse_source.as_ref().map(|(source_name, parse_block)| {
            Self::compile_definition_from_parse_source(source_name, parse_block)
        });

        if !query.is_empty() {
            self.registry.ensure_cards_loaded([query]);
        }
        let registry_definition = self.find_card_definition(query).cloned();
        let compiled_definition = registry_definition.or_else(|| {
            source_compile_result
                .as_ref()
                .and_then(|result| result.as_ref().ok().cloned())
        });
        let canonical_name = compiled_definition
            .as_ref()
            .map(|definition| definition.name().to_string())
            .or_else(|| {
                parse_source
                    .as_ref()
                    .map(|(source_name, _)| source_name.clone())
            });
        let oracle_text = compiled_definition
            .as_ref()
            .map(|definition| definition.card.oracle_text.clone())
            .or_else(|| {
                parse_source.as_ref().and_then(|(_, parse_block)| {
                    Self::extract_oracle_text_from_parse_block(parse_block)
                })
            });
        let compiled_text = compiled_definition
            .as_ref()
            .map(crate::compiled_text::compiled_lines)
            .unwrap_or_default();
        let compiled_abilities = compiled_definition
            .as_ref()
            .map(Self::compiled_ability_lines)
            .unwrap_or_default();
        let semantic_score = canonical_name
            .as_deref()
            .and_then(Self::semantic_score_for_name)
            .or_else(|| Self::semantic_score_for_name(query));
        let threshold_percent =
            (self.semantic_threshold > 0.0).then_some(self.semantic_threshold * 100.0);
        let parse_error = if query.is_empty() {
            Some("card name cannot be empty".to_string())
        } else if let Some(result) = source_compile_result.as_ref() {
            result
                .clone()
                .err()
                .or_else(|| CardRegistry::try_compile_card(query).err())
        } else {
            CardRegistry::try_compile_card(query).err()
        };
        let error = explicit_error
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
            .or_else(|| parse_error.clone());

        CardLoadDiagnostics {
            query: query.to_string(),
            canonical_name,
            error,
            parse_error,
            oracle_text,
            compiled_text,
            compiled_abilities,
            semantic_score,
            threshold_percent,
        }
    }

    fn load_compilable_card_definition(
        &mut self,
        query: &str,
    ) -> Result<crate::cards::CardDefinition, JsValue> {
        if let Some(definition) = self.find_card_definition(query).cloned() {
            if let Some(error) = crate::cards::unsupported_generated_definition_error(&definition) {
                return Err(JsValue::from_str(&error));
            }
            return Ok(definition);
        }

        match crate::cards::CardRegistry::try_compile_card(query) {
            Ok(_) => Err(JsValue::from_str(&format!("unknown card name: {query}"))),
            Err(err) => Err(JsValue::from_str(&err)),
        }
    }

    fn initialize_empty_match(&mut self, player_names: Vec<String>, starting_life: i32, seed: u64) {
        reset_runtime_id_counters();
        self.game = GameState::new(player_names, starting_life);
        self.game.set_random_seed(seed);
        self.match_format = MatchFormatInput::Normal;
        self.pregame = None;
    }

    fn populate_demo_libraries(&mut self) -> Result<(), JsValue> {
        let player_ids: Vec<PlayerId> = self.game.players.iter().map(|p| p.id).collect();
        for player_id in player_ids {
            let deck = self.build_random_demo_deck_names(60, 24)?;
            self.populate_player_library(player_id, &deck)?;
        }
        Ok(())
    }

    fn populate_explicit_libraries(&mut self, decks: &[Vec<String>]) -> Result<(), JsValue> {
        let player_ids: Vec<PlayerId> = self.game.players.iter().map(|p| p.id).collect();
        for (&player_id, deck) in player_ids.iter().zip(decks.iter()) {
            self.populate_player_library(player_id, deck)?;
        }
        Ok(())
    }

    fn populate_explicit_commanders(&mut self, commanders: &[Vec<String>]) -> Result<(), JsValue> {
        let player_ids: Vec<PlayerId> = self.game.players.iter().map(|p| p.id).collect();
        for (&player_id, commander_names) in player_ids.iter().zip(commanders.iter()) {
            self.registry
                .ensure_cards_loaded(commander_names.iter().map(|name| name.as_str()));

            for name in commander_names {
                let Some(definition) = self.find_card_definition(name).cloned() else {
                    return Err(JsValue::from_str(&format!("unknown card name: {name}")));
                };
                let object_id = self.game.create_object_from_definition(
                    &definition,
                    player_id,
                    crate::zone::Zone::Command,
                );
                self.game.set_as_commander(object_id, player_id);
            }
        }
        Ok(())
    }

    fn validate_commander_setup(
        &self,
        decks: &[Vec<String>],
        commanders: &[Vec<String>],
    ) -> Result<(), JsValue> {
        if decks.len() != self.game.players.len() {
            return Err(JsValue::from_str(
                "deck count must match number of players in game",
            ));
        }
        if commanders.len() != self.game.players.len() {
            return Err(JsValue::from_str(
                "commander count must match number of players in game",
            ));
        }

        for (deck, commander_list) in decks.iter().zip(commanders.iter()) {
            if !(commander_list.len() == 1 || commander_list.len() == 2) {
                return Err(JsValue::from_str(
                    "commander matches require exactly 1 or 2 commanders per player",
                ));
            }

            let expected_deck_size = if commander_list.len() == 2 { 98 } else { 99 };
            if deck.len() != expected_deck_size {
                return Err(JsValue::from_str(&format!(
                    "commander main decks must contain {expected_deck_size} cards for {count} commander(s)",
                    count = commander_list.len()
                )));
            }
        }

        Ok(())
    }

    fn player_hand_ids(&self, player: PlayerId) -> Vec<ObjectId> {
        self.game
            .player(player)
            .map(|player| player.hand.clone())
            .unwrap_or_default()
    }

    fn build_hand_selectable_objects(
        &self,
        player: PlayerId,
    ) -> Vec<crate::decisions::context::SelectableObject> {
        self.player_hand_ids(player)
            .into_iter()
            .map(|id| {
                let name = self
                    .game
                    .object(id)
                    .map(|object| object.name.clone())
                    .unwrap_or_else(|| format!("Card {}", id.0));
                crate::decisions::context::SelectableObject::new(id, name)
            })
            .collect()
    }

    fn parsed_pregame_begin_on_battlefield_spec(
        &self,
        card_id: ObjectId,
        ability_index: usize,
    ) -> Option<crate::static_abilities::PregameBeginOnBattlefieldSpec> {
        let ability = self.game.object(card_id)?.abilities.get(ability_index)?;
        let crate::ability::AbilityKind::Static(static_ability) = &ability.kind else {
            return None;
        };
        match static_ability.pregame_action_kind()? {
            crate::static_abilities::PregameActionKind::BeginOnBattlefield(spec) => Some(spec),
        }
    }

    fn available_pregame_actions(&self, player: PlayerId) -> Vec<LegalAction> {
        let starting_player = self.game.turn_order.first().copied();
        let hand_ids = self.player_hand_ids(player);
        let other_cards_in_hand = hand_ids.len().saturating_sub(1);
        let mut actions = Vec::new();
        for card_id in hand_ids {
            let Some(object) = self.game.object(card_id) else {
                continue;
            };
            for (ability_index, ability) in object.abilities.iter().enumerate() {
                let crate::ability::AbilityKind::Static(static_ability) = &ability.kind else {
                    continue;
                };
                let Some(crate::static_abilities::PregameActionKind::BeginOnBattlefield(spec)) =
                    static_ability.pregame_action_kind()
                else {
                    continue;
                };
                if spec.require_not_starting_player && starting_player == Some(player) {
                    continue;
                }
                if other_cards_in_hand < spec.exile_cards_from_hand {
                    continue;
                }
                actions.push(LegalAction::UsePregameAction {
                    card_id,
                    ability_index,
                });
            }
        }
        actions
    }

    fn shuffle_hand_into_library_and_draw(&mut self, player: PlayerId, opening_hand_size: usize) {
        let hand_ids = self.player_hand_ids(player);
        for id in hand_ids {
            let _ = self.game.move_object(id, Zone::Library);
        }
        self.game.shuffle_player_library(player);
        let _ = self.game.draw_cards(player, opening_hand_size);
    }

    fn move_cards_to_library_bottom(&mut self, ordered_cards_bottom_first: &[ObjectId]) {
        for card_id in ordered_cards_bottom_first.iter().rev().copied() {
            let Some(owner) = self.game.object(card_id).map(|object| object.owner) else {
                continue;
            };
            let Some(new_id) = self.game.move_object(card_id, Zone::Library) else {
                continue;
            };
            let Some(player) = self.game.player_mut(owner) else {
                continue;
            };
            let Some(index) = player
                .library
                .iter()
                .rposition(|candidate| *candidate == new_id)
            else {
                continue;
            };
            let moved = player.library.remove(index);
            player.library.insert(0, moved);
        }
    }

    fn normalize_pregame_state(&mut self) -> Result<(), JsValue> {
        loop {
            let Some(pregame) = self.pregame.as_ref() else {
                return Ok(());
            };

            match &pregame.stage {
                PregameStage::MulliganDecision {
                    undecided_players,
                    round_mulliganers,
                } if undecided_players.is_empty() => {
                    if round_mulliganers.is_empty() {
                        let queue = self
                            .game
                            .turn_order
                            .iter()
                            .copied()
                            .filter(|player| pregame.cards_to_bottom(*player) > 0)
                            .collect();
                        if let Some(pregame) = self.pregame.as_mut() {
                            pregame.stage = PregameStage::BottomCards {
                                queue,
                                pending_order: None,
                            };
                        }
                        continue;
                    }

                    let opening_hand_size = pregame.opening_hand_size;
                    let mulliganers = round_mulliganers.clone();
                    if let Some(pregame) = self.pregame.as_mut() {
                        for player in &mulliganers {
                            *pregame.mulligans_taken.entry(*player).or_insert(0) += 1;
                        }
                    }
                    for player in mulliganers.iter().copied() {
                        self.shuffle_hand_into_library_and_draw(player, opening_hand_size);
                    }
                    if let Some(pregame) = self.pregame.as_mut() {
                        pregame.stage = PregameStage::MulliganDecision {
                            undecided_players: mulliganers,
                            round_mulliganers: Vec::new(),
                        };
                    }
                    continue;
                }
                PregameStage::BottomCards {
                    queue,
                    pending_order,
                } if queue.is_empty() && pending_order.is_none() => {
                    if let Some(pregame) = self.pregame.as_mut() {
                        pregame.stage = PregameStage::OpeningActions {
                            current_index: 0,
                            pending_hand_exile: None,
                        };
                    }
                    continue;
                }
                PregameStage::OpeningActions {
                    current_index,
                    pending_hand_exile,
                } if pending_hand_exile.is_none()
                    && *current_index >= self.game.turn_order.len() =>
                {
                    self.pregame = None;
                    continue;
                }
                _ => return Ok(()),
            }
        }
    }

    fn build_pregame_decision(&self) -> Result<Option<DecisionContext>, JsValue> {
        let Some(pregame) = self.pregame.as_ref() else {
            return Ok(None);
        };

        let ctx = match &pregame.stage {
            PregameStage::MulliganDecision {
                undecided_players, ..
            } => {
                let Some(player) = undecided_players.first().copied() else {
                    return Ok(None);
                };
                let mut actions = vec![LegalAction::KeepOpeningHand, LegalAction::TakeMulligan];
                actions.extend(
                    self.player_hand_ids(player)
                        .into_iter()
                        .filter_map(|card_id| {
                            let is_serum_powder = self
                                .game
                                .object(card_id)
                                .is_some_and(|object| object.name == "Serum Powder");
                            is_serum_powder.then_some(LegalAction::SerumPowderMulligan { card_id })
                        }),
                );
                DecisionContext::Priority(crate::decisions::context::PriorityContext::new(
                    player, actions,
                ))
            }
            PregameStage::BottomCards {
                queue,
                pending_order,
            } => {
                if let Some((player, selected_cards)) = pending_order {
                    let items = selected_cards
                        .iter()
                        .filter_map(|id| {
                            self.game
                                .object(*id)
                                .map(|object| (*id, object.name.clone()))
                        })
                        .collect();
                    DecisionContext::Order(crate::decisions::context::OrderContext::new(
                        *player,
                        None,
                        "Order the selected cards for the bottom of your library. The first option becomes the bottom-most card.",
                        items,
                    ))
                } else {
                    let Some(player) = queue.first().copied() else {
                        return Ok(None);
                    };
                    let amount = pregame.cards_to_bottom(player);
                    DecisionContext::SelectObjects(
                        crate::decisions::context::SelectObjectsContext::new(
                            player,
                            None,
                            format!("Choose {amount} card(s) to put on the bottom of your library"),
                            self.build_hand_selectable_objects(player),
                            amount,
                            Some(amount),
                        ),
                    )
                }
            }
            PregameStage::OpeningActions {
                current_index,
                pending_hand_exile,
            } => {
                let Some(player) = self.game.turn_order.get(*current_index).copied() else {
                    return Ok(None);
                };
                if let Some(pending_exile) = pending_hand_exile {
                    if pending_exile.player != player {
                        return Err(JsValue::from_str(
                            "pregame hand exile prompt is out of sync with turn order",
                        ));
                    }
                    let source_name = self
                        .game
                        .object(pending_exile.source)
                        .map(|object| object.name.as_str())
                        .unwrap_or("this card");
                    DecisionContext::SelectObjects(
                        crate::decisions::context::SelectObjectsContext::new(
                            player,
                            Some(pending_exile.source),
                            format!(
                                "Choose {} card(s) from your hand to exile for {}",
                                pending_exile.amount, source_name
                            ),
                            self.build_hand_selectable_objects(player),
                            pending_exile.amount,
                            Some(pending_exile.amount),
                        ),
                    )
                } else {
                    let is_last_player = *current_index + 1 >= self.game.turn_order.len();
                    let mut actions = vec![if is_last_player {
                        LegalAction::BeginGame
                    } else {
                        LegalAction::ContinuePregame
                    }];
                    actions.extend(self.available_pregame_actions(player));
                    DecisionContext::Priority(crate::decisions::context::PriorityContext::new(
                        player, actions,
                    ))
                }
            }
        };

        Ok(Some(ctx))
    }

    fn apply_pregame_priority_action(&mut self, action: LegalAction) -> Result<(), JsValue> {
        match action {
            LegalAction::KeepOpeningHand => {
                let Some(PregameState {
                    stage:
                        PregameStage::MulliganDecision {
                            undecided_players, ..
                        },
                    ..
                }) = self.pregame.as_mut()
                else {
                    return Err(JsValue::from_str(
                        "keep hand is only legal during mulligan decisions",
                    ));
                };
                if undecided_players.is_empty() {
                    return Err(JsValue::from_str(
                        "no player is waiting on a mulligan decision",
                    ));
                }
                undecided_players.remove(0);
            }
            LegalAction::TakeMulligan => {
                let Some(PregameState {
                    stage:
                        PregameStage::MulliganDecision {
                            undecided_players,
                            round_mulliganers,
                        },
                    ..
                }) = self.pregame.as_mut()
                else {
                    return Err(JsValue::from_str(
                        "mulligan is only legal during mulligan decisions",
                    ));
                };
                let Some(player) = undecided_players.first().copied() else {
                    return Err(JsValue::from_str(
                        "no player is waiting on a mulligan decision",
                    ));
                };
                undecided_players.remove(0);
                round_mulliganers.push(player);
            }
            LegalAction::SerumPowderMulligan { card_id } => {
                let player = match self.pregame.as_ref() {
                    Some(PregameState {
                        stage:
                            PregameStage::MulliganDecision {
                                undecided_players, ..
                            },
                        ..
                    }) => undecided_players.first().copied(),
                    _ => None,
                }
                .ok_or_else(|| {
                    JsValue::from_str("Serum Powder can only be used while mulliganing")
                })?;
                let hand_ids = self.player_hand_ids(player);
                if !hand_ids.contains(&card_id) {
                    return Err(JsValue::from_str(
                        "Serum Powder must be in the current player's hand",
                    ));
                }
                let is_serum_powder = self
                    .game
                    .object(card_id)
                    .is_some_and(|object| object.name == "Serum Powder");
                if !is_serum_powder {
                    return Err(JsValue::from_str("selected card is not Serum Powder"));
                }
                let draw_count = hand_ids.len();
                for id in hand_ids {
                    let _ = self.game.move_object(id, Zone::Exile);
                }
                let _ = self.game.draw_cards(player, draw_count);
            }
            LegalAction::ContinuePregame | LegalAction::BeginGame => {
                let Some(PregameState {
                    stage:
                        PregameStage::OpeningActions {
                            current_index,
                            pending_hand_exile,
                        },
                    ..
                }) = self.pregame.as_mut()
                else {
                    return Err(JsValue::from_str(
                        "continue is only legal during pregame opening actions",
                    ));
                };
                if pending_hand_exile.is_some() {
                    return Err(JsValue::from_str(
                        "a pregame action requires exiling cards before continuing",
                    ));
                }
                *current_index += 1;
            }
            LegalAction::UsePregameAction {
                card_id,
                ability_index,
            } => {
                let player = match self.pregame.as_ref() {
                    Some(PregameState {
                        stage:
                            PregameStage::OpeningActions {
                                current_index,
                                pending_hand_exile: None,
                            },
                        ..
                    }) => self.game.turn_order.get(*current_index).copied(),
                    _ => None,
                }
                .ok_or_else(|| {
                    JsValue::from_str(
                        "pregame actions can only be used during pregame opening actions",
                    )
                })?;
                let hand_ids = self.player_hand_ids(player);
                if !hand_ids.contains(&card_id) {
                    return Err(JsValue::from_str(
                        "pregame action source must be in the current player's hand",
                    ));
                }
                let Some(spec) =
                    self.parsed_pregame_begin_on_battlefield_spec(card_id, ability_index)
                else {
                    return Err(JsValue::from_str(
                        "selected ability is not a supported pregame action",
                    ));
                };
                if spec.require_not_starting_player
                    && self.game.turn_order.first().copied() == Some(player)
                {
                    return Err(JsValue::from_str(
                        "the starting player can't use that pregame action",
                    ));
                }
                if hand_ids.len().saturating_sub(1) < spec.exile_cards_from_hand {
                    return Err(JsValue::from_str(
                        "that pregame action requires more cards in hand to exile",
                    ));
                }
                let exile_cards_from_hand = spec.exile_cards_from_hand;
                let Some(new_id) = self.game.move_object(card_id, Zone::Battlefield) else {
                    return Err(JsValue::from_str(
                        "failed to move the pregame card to the battlefield",
                    ));
                };
                for (counter_type, count) in spec.counters.iter().cloned() {
                    let _ = self.game.add_counters(new_id, counter_type, count);
                }
                let Some(PregameState {
                    stage:
                        PregameStage::OpeningActions {
                            pending_hand_exile, ..
                        },
                    ..
                }) = self.pregame.as_mut()
                else {
                    return Err(JsValue::from_str(
                        "pregame opening actions disappeared while resolving a pregame action",
                    ));
                };
                *pending_hand_exile =
                    (exile_cards_from_hand > 0).then_some(PendingPregameHandExile {
                        player,
                        source: new_id,
                        amount: exile_cards_from_hand,
                    });
            }
            other => {
                return Err(JsValue::from_str(&format!(
                    "illegal pregame priority action: {other:?}"
                )));
            }
        }

        Ok(())
    }

    fn dispatch_pregame_decision(
        &mut self,
        pending_ctx: DecisionContext,
        command: UiCommand,
    ) -> Result<JsValue, JsValue> {
        let restore =
            |this: &mut Self, ctx: DecisionContext, err: JsValue| -> Result<JsValue, JsValue> {
                this.pending_decision = Some(ctx);
                Err(err)
            };

        match (&pending_ctx, command) {
            (
                DecisionContext::Priority(priority),
                UiCommand::PriorityAction {
                    action_index,
                    action_ref,
                },
            ) => {
                let action = resolve_priority_action(priority, action_index, action_ref.as_ref())
                    .ok_or_else(|| {
                        if let Some(action_ref) = action_ref.as_ref() {
                            JsValue::from_str(&format!(
                                "invalid priority action ref: {action_ref:?}"
                            ))
                        } else if let Some(action_index) = action_index {
                            JsValue::from_str(&format!(
                                "invalid priority action index: {action_index}"
                            ))
                        } else {
                            JsValue::from_str("missing priority action selector")
                        }
                    });
                let action = match action {
                    Ok(action) => action,
                    Err(err) => return restore(self, pending_ctx, err),
                };
                if let Err(err) = self.apply_pregame_priority_action(action) {
                    return restore(self, pending_ctx, err);
                }
            }
            (DecisionContext::SelectObjects(objects), UiCommand::SelectObjects { object_ids }) => {
                let legal_ids: Vec<u64> = objects
                    .candidates
                    .iter()
                    .filter(|candidate| candidate.legal)
                    .map(|candidate| candidate.id.0)
                    .collect();
                if let Err(err) =
                    validate_object_selection(objects.min, objects.max, &object_ids, &legal_ids)
                {
                    return restore(self, pending_ctx, err);
                }
                let selected: Vec<ObjectId> =
                    object_ids.into_iter().map(ObjectId::from_raw).collect();
                enum PregameSelectResolution {
                    BottomNow,
                    BottomNeedsOrdering(PlayerId),
                    HandExile(Vec<ObjectId>),
                }

                let resolution = match self.pregame.as_ref().map(|pregame| &pregame.stage) {
                    Some(PregameStage::BottomCards {
                        queue,
                        pending_order,
                    }) if pending_order.is_none() => {
                        let Some(player) = queue.first().copied() else {
                            return restore(
                                self,
                                pending_ctx,
                                JsValue::from_str("no player is waiting to bottom cards"),
                            );
                        };
                        if selected.len() <= 1 {
                            PregameSelectResolution::BottomNow
                        } else {
                            PregameSelectResolution::BottomNeedsOrdering(player)
                        }
                    }
                    Some(PregameStage::OpeningActions {
                        pending_hand_exile, ..
                    }) if pending_hand_exile.is_some() => {
                        if selected.is_empty() {
                            return restore(
                                self,
                                pending_ctx,
                                JsValue::from_str("expected at least one card to exile"),
                            );
                        }
                        PregameSelectResolution::HandExile(selected.clone())
                    }
                    _ => {
                        return restore(
                            self,
                            pending_ctx,
                            JsValue::from_str("unexpected select_objects command during pregame"),
                        );
                    }
                };

                match resolution {
                    PregameSelectResolution::BottomNow => {
                        self.move_cards_to_library_bottom(&selected);
                        let Some(PregameStage::BottomCards { queue, .. }) =
                            self.pregame.as_mut().map(|pregame| &mut pregame.stage)
                        else {
                            return restore(
                                self,
                                pending_ctx,
                                JsValue::from_str("pregame bottoming state disappeared"),
                            );
                        };
                        if !queue.is_empty() {
                            queue.remove(0);
                        }
                    }
                    PregameSelectResolution::BottomNeedsOrdering(player) => {
                        let Some(PregameStage::BottomCards { pending_order, .. }) =
                            self.pregame.as_mut().map(|pregame| &mut pregame.stage)
                        else {
                            return restore(
                                self,
                                pending_ctx,
                                JsValue::from_str("pregame bottoming state disappeared"),
                            );
                        };
                        *pending_order = Some((player, selected));
                    }
                    PregameSelectResolution::HandExile(card_ids) => {
                        for card_id in card_ids {
                            let _ = self.game.move_object(card_id, Zone::Exile);
                        }
                        let Some(PregameStage::OpeningActions {
                            pending_hand_exile, ..
                        }) = self.pregame.as_mut().map(|pregame| &mut pregame.stage)
                        else {
                            return restore(
                                self,
                                pending_ctx,
                                JsValue::from_str("pregame hand exile state disappeared"),
                            );
                        };
                        *pending_hand_exile = None;
                    }
                }
            }
            (DecisionContext::Order(order), UiCommand::SelectOptions { option_indices }) => {
                let legal: Vec<usize> = (0..order.items.len()).collect();
                if let Err(err) = validate_option_selection(
                    order.items.len(),
                    Some(order.items.len()),
                    &option_indices,
                    &legal,
                ) {
                    return restore(self, pending_ctx, err);
                }
                if unique_indices(&option_indices).len() != order.items.len() {
                    return restore(
                        self,
                        pending_ctx,
                        JsValue::from_str("ordering requires each option index exactly once"),
                    );
                }
                let selected_cards = match self.pregame.as_mut().map(|pregame| &mut pregame.stage) {
                    Some(PregameStage::BottomCards { pending_order, .. }) => {
                        let Some((_, selected_cards)) = pending_order.take() else {
                            return restore(
                                self,
                                pending_ctx,
                                JsValue::from_str("no selected cards are waiting to be ordered"),
                            );
                        };
                        selected_cards
                    }
                    _ => {
                        return restore(
                            self,
                            pending_ctx,
                            JsValue::from_str("unexpected ordering command during pregame"),
                        );
                    }
                };
                let ordered_cards: Vec<ObjectId> = option_indices
                    .into_iter()
                    .filter_map(|index| selected_cards.get(index).copied())
                    .collect();
                self.move_cards_to_library_bottom(&ordered_cards);
                if let Some(PregameStage::BottomCards { queue, .. }) =
                    self.pregame.as_mut().map(|pregame| &mut pregame.stage)
                    && !queue.is_empty()
                {
                    queue.remove(0);
                }
            }
            _ => {
                return restore(
                    self,
                    pending_ctx,
                    JsValue::from_str("command type does not match pregame decision"),
                );
            }
        }

        self.pending_decision = None;
        self.advance_until_decision()?;
        self.snapshot()
    }

    fn finish_match_setup(&mut self, opening_hand_size: usize) -> Result<(), JsValue> {
        self.reset_runtime_state();
        let player_ids: Vec<PlayerId> = self.game.players.iter().map(|p| p.id).collect();
        for player_id in player_ids {
            let _ = self.game.draw_cards(player_id, opening_hand_size);
        }
        self.pregame = Some(PregameState::new(
            &self.game.turn_order,
            opening_hand_size,
            self.match_format,
        ));
        self.recompute_ui_decision()
    }

    fn generate_match_seed(&self) -> u64 {
        let bits = (js_sys::Math::random() * (u64::MAX as f64)) as u64;
        if bits == 0 {
            0x9e37_79b9_7f4a_7c15
        } else {
            bits
        }
    }

    fn reset_runtime_state(&mut self) {
        self.trigger_queue = TriggerQueue::new();
        self.priority_state = PriorityLoopState::new(self.game.players.len());
        self.priority_state
            .set_auto_choose_single_pip_payment(false);
        self.pregame = None;
        self.pending_decision = None;
        self.pending_replay_action = None;
        self.pending_action_checkpoint = None;
        self.pending_live_action_root = None;
        self.priority_epoch_checkpoint = None;
        self.priority_epoch_has_undoable_action = false;
        self.priority_epoch_undo_locked_by_mana = false;
        self.priority_epoch_undo_land_stable_id = None;
        self.active_viewed_cards = None;
        self.clear_active_resolving_stack_object();
        self.game_over = None;
        self.runner = None;
        self.runner_awaiting_priority = false;
        self.runner_pending_decision = false;
        if self.game.player(self.perspective).is_none()
            && let Some(first) = self.game.players.first()
        {
            self.perspective = first.id;
        }
    }

    fn recompute_ui_decision(&mut self) -> Result<(), JsValue> {
        self.pending_decision = None;
        self.pending_replay_action = None;
        self.pending_action_checkpoint = None;
        self.pending_live_action_root = None;
        self.priority_epoch_checkpoint = None;
        self.priority_epoch_has_undoable_action = false;
        self.priority_epoch_undo_locked_by_mana = false;
        self.priority_epoch_undo_land_stable_id = None;
        self.active_viewed_cards = None;
        self.clear_active_resolving_stack_object();
        if self.game_over.is_some() {
            return Ok(());
        }
        self.advance_until_decision()
    }

    fn should_auto_resolve_cleanup_discard(&self, ctx: &DecisionContext) -> bool {
        if !self.auto_cleanup_discard {
            return false;
        }
        let DecisionContext::SelectObjects(obj) = ctx else {
            return false;
        };
        self.game.turn.step == Some(crate::game_state::Step::Cleanup)
            && obj.min > 0
            && obj.player != self.perspective
    }

    fn advance_until_decision(&mut self) -> Result<(), JsValue> {
        use crate::turn_runner::TurnAction;

        if self.pregame.is_some() {
            for _ in 0..64 {
                self.normalize_pregame_state()?;
                if let Some(ctx) = self.build_pregame_decision()? {
                    self.pending_decision = Some(ctx);
                    self.runner_pending_decision = false;
                    return Ok(());
                }
                if self.pregame.is_none() {
                    break;
                }
            }
        }

        // Lazily create the TurnRunner on first call.
        if self.runner.is_none() {
            self.runner = Some(crate::turn_runner::TurnRunner::new());
            self.runner_awaiting_priority = false;
        }

        for _ in 0..192 {
            // If we're NOT currently inside a priority loop, advance the TurnRunner
            if !self.runner_awaiting_priority {
                let action = {
                    let runner = self.runner.as_mut().unwrap();
                    runner
                        .advance(&mut self.game, &mut self.trigger_queue)
                        .map_err(|e| JsValue::from_str(&format!("{e}")))?
                };

                match action {
                    TurnAction::Continue => continue,

                    TurnAction::Decision(ctx) => {
                        self.clear_active_resolving_stack_object();
                        // Auto-resolve cleanup discards when the flag is set.
                        if self.should_auto_resolve_cleanup_discard(&ctx)
                            && let DecisionContext::SelectObjects(ref obj) = ctx
                        {
                            let mut ids: Vec<_> = obj
                                .candidates
                                .iter()
                                .filter(|c| c.legal)
                                .map(|c| c.id)
                                .collect();
                            self.game.shuffle_slice(&mut ids);
                            ids.truncate(obj.min);
                            self.runner.as_mut().unwrap().respond_discard(ids);
                            continue;
                        }
                        self.pending_decision = Some(ctx);
                        self.runner_pending_decision = true;
                        return Ok(());
                    }

                    TurnAction::RunPriority => {
                        self.runner_awaiting_priority = true;
                        // Fall through to the priority loop below
                    }

                    TurnAction::TurnComplete => {
                        // Check for game over before starting next turn
                        let remaining: Vec<_> = self
                            .game
                            .players
                            .iter()
                            .filter(|p| p.is_in_game())
                            .collect();
                        if remaining.len() <= 1 {
                            let result = if let Some(winner) = remaining.first() {
                                GameResult::Winner(winner.id)
                            } else {
                                GameResult::Draw
                            };
                            self.game_over = Some(result);
                            return Ok(());
                        }

                        // Advance to next turn
                        self.game.next_turn();
                        self.runner = Some(crate::turn_runner::TurnRunner::new());
                        self.runner_awaiting_priority = false;
                        continue;
                    }

                    TurnAction::GameOver(result) => {
                        self.game_over = Some(result);
                        return Ok(());
                    }
                }
            }

            // We're inside a priority loop - use existing priority mechanism
            if self.priority_epoch_checkpoint.is_none() {
                self.priority_epoch_checkpoint = Some(self.capture_replay_checkpoint());
                self.priority_epoch_has_undoable_action = false;
                self.priority_epoch_undo_locked_by_mana = false;
                self.priority_epoch_undo_land_stable_id = None;
            }
            let checkpoint = self.capture_replay_checkpoint();
            let outcome = self.execute_with_replay(&checkpoint, &ReplayRoot::Advance, &[])?;

            match outcome {
                ReplayOutcome::NeedsDecision(ctx) => {
                    self.pending_decision = Some(ctx);
                    self.runner_pending_decision = false;
                    self.pending_replay_action = Some(PendingReplayAction {
                        checkpoint,
                        root: ReplayRoot::Advance,
                        nested_answers: Vec::new(),
                    });
                    return Ok(());
                }
                ReplayOutcome::Complete(progress) => match progress {
                    GameProgress::NeedsDecisionCtx(ctx) => {
                        self.clear_active_resolving_stack_object();
                        self.pending_decision = Some(ctx);
                        self.runner_pending_decision = false;
                        return Ok(());
                    }
                    GameProgress::Continue => {
                        // Priority loop ended - notify runner
                        self.runner.as_mut().unwrap().priority_done();
                        self.runner_awaiting_priority = false;
                        self.pending_action_checkpoint = None;
                        self.priority_epoch_checkpoint = None;
                        self.priority_epoch_has_undoable_action = false;
                        self.priority_epoch_undo_locked_by_mana = false;
                        self.priority_epoch_undo_land_stable_id = None;
                        self.pending_decision = None;
                        self.clear_active_resolving_stack_object();
                        continue;
                    }
                    GameProgress::StackResolved => {
                        // New priority round after resolution — fresh epoch.
                        self.pending_action_checkpoint = None;
                        self.priority_epoch_checkpoint = None;
                        self.priority_epoch_has_undoable_action = false;
                        self.priority_epoch_undo_locked_by_mana = false;
                        self.priority_epoch_undo_land_stable_id = None;
                        self.clear_active_resolving_stack_object();
                        continue;
                    }
                    GameProgress::GameOver(result) => {
                        self.pending_action_checkpoint = None;
                        self.pending_decision = None;
                        self.clear_active_resolving_stack_object();
                        self.game_over = Some(result);
                        return Ok(());
                    }
                },
            }
        }

        Err(JsValue::from_str(
            "advance loop exceeded iteration budget (possible infinite loop)",
        ))
    }

    fn apply_progress(&mut self, progress: GameProgress) -> Result<(), JsValue> {
        match progress {
            GameProgress::NeedsDecisionCtx(ctx) => {
                self.clear_active_resolving_stack_object();
                self.pending_decision = Some(ctx);
                Ok(())
            }
            GameProgress::Continue => {
                // Priority loop ended - notify runner and continue
                if self.runner.is_some() {
                    self.runner.as_mut().unwrap().priority_done();
                    self.runner_awaiting_priority = false;
                }
                self.pending_action_checkpoint = None;
                self.priority_epoch_checkpoint = None;
                self.priority_epoch_has_undoable_action = false;
                self.priority_epoch_undo_locked_by_mana = false;
                self.priority_epoch_undo_land_stable_id = None;
                self.pending_decision = None;
                self.clear_active_resolving_stack_object();
                self.advance_until_decision()
            }
            GameProgress::GameOver(result) => {
                self.pending_action_checkpoint = None;
                self.pending_decision = None;
                self.clear_active_resolving_stack_object();
                self.game_over = Some(result);
                Ok(())
            }
            GameProgress::StackResolved => {
                self.pending_action_checkpoint = None;
                self.priority_epoch_checkpoint = None;
                self.priority_epoch_has_undoable_action = false;
                self.priority_epoch_undo_locked_by_mana = false;
                self.priority_epoch_undo_land_stable_id = None;
                self.pending_decision = None;
                self.clear_active_resolving_stack_object();
                self.advance_until_decision()
            }
        }
    }

    /// Handle a response to a TurnRunner-sourced decision (attackers/blockers/discard).
    fn dispatch_runner_decision(
        &mut self,
        pending_ctx: DecisionContext,
        command: UiCommand,
    ) -> Result<JsValue, JsValue> {
        let _runner = self.runner.as_mut().ok_or_else(|| {
            // Restore decision on structural error so UI can retry.
            self.pending_decision = Some(pending_ctx.clone());
            self.runner_pending_decision = true;
            JsValue::from_str("runner_pending_decision set but no runner present")
        })?;

        let restore_on_err = |this: &mut Self, ctx: DecisionContext, err: JsValue| -> JsValue {
            this.pending_decision = Some(ctx);
            this.runner_pending_decision = true;
            err
        };

        match (&pending_ctx, command) {
            (DecisionContext::Attackers(actx), UiCommand::DeclareAttackers { declarations }) => {
                let converted = validate_attacker_declarations(actx, &declarations)
                    .map_err(|e| restore_on_err(self, pending_ctx.clone(), e))?;
                self.runner.as_mut().unwrap().respond_attackers(converted);
            }
            (DecisionContext::Blockers(bctx), UiCommand::DeclareBlockers { declarations }) => {
                let player = bctx.player;
                let converted = validate_blocker_declarations(bctx, &declarations)
                    .map_err(|e| restore_on_err(self, pending_ctx.clone(), e))?;
                self.runner
                    .as_mut()
                    .unwrap()
                    .respond_blockers(converted, player);
            }
            (DecisionContext::SelectObjects(obj_ctx), UiCommand::SelectObjects { object_ids }) => {
                // Validate discard selection against the decision context.
                let legal_ids: Vec<u64> = obj_ctx
                    .candidates
                    .iter()
                    .filter(|c| c.legal)
                    .map(|c| c.id.0)
                    .collect();
                validate_object_selection(obj_ctx.min, obj_ctx.max, &object_ids, &legal_ids)
                    .map_err(|e| restore_on_err(self, pending_ctx.clone(), e))?;

                let cards: Vec<ObjectId> = object_ids
                    .iter()
                    .map(|&id| ObjectId::from_raw(id))
                    .collect();
                self.runner.as_mut().unwrap().respond_discard(cards);
            }
            (DecisionContext::Boolean(_), UiCommand::SelectOptions { option_indices }) => {
                validate_option_selection(1, Some(1), &option_indices, &[0usize, 1usize])?;
                let answer = option_indices.first().copied() == Some(1);
                self.runner.as_mut().unwrap().respond_boolean(answer);
            }
            _ => {
                self.pending_decision = Some(pending_ctx);
                self.runner_pending_decision = true;
                return Err(JsValue::from_str("unexpected command for runner decision"));
            }
        }

        // The runner is now in a state where advance() will apply the response.
        // We're no longer awaiting priority (runner will handle the next steps).
        self.runner_awaiting_priority = false;
        self.advance_until_decision()?;
        self.snapshot()
    }

    fn finish_live_priority_dispatch(
        &mut self,
        progress: GameProgress,
        action_checkpoint: Option<ReplayCheckpoint>,
        resolving_checkpoint: Option<ReplayCheckpoint>,
    ) -> Result<JsValue, JsValue> {
        match progress {
            GameProgress::NeedsDecisionCtx(next_ctx) => {
                let action_still_pending = self.priority_action_chain_still_pending();
                if action_still_pending {
                    self.clear_active_resolving_stack_object();
                } else {
                    self.sync_active_resolving_stack_object(resolving_checkpoint.as_ref());
                }
                if action_still_pending {
                    if let Some(checkpoint) = action_checkpoint {
                        self.pending_action_checkpoint.get_or_insert(checkpoint);
                    }
                } else {
                    self.pending_action_checkpoint = None;
                }

                if !action_still_pending {
                    self.priority_state.pending_continuation = None;
                    self.pending_live_action_root = None;
                    self.pending_replay_action = None;
                    self.pending_live_continuation = Some(LivePriorityContinuation {
                        checkpoint: self.capture_replay_checkpoint_tagged("finish_live_dispatch"),
                        root: PendingPriorityContinuation::ApplyDecisionContext(next_ctx.clone()),
                        answers: Vec::new(),
                    });
                } else if self.decision_uses_live_priority_response(&next_ctx) {
                    self.priority_state.pending_continuation = None;
                    self.pending_live_continuation = None;
                    self.pending_replay_action = None;
                } else {
                    self.priority_state.pending_continuation = None;
                    self.pending_live_continuation = Some(LivePriorityContinuation {
                        checkpoint: self.capture_replay_checkpoint_tagged("finish_live_dispatch"),
                        root: PendingPriorityContinuation::ApplyDecisionContext(next_ctx.clone()),
                        answers: Vec::new(),
                    });
                    self.pending_replay_action = None;
                }
                self.pending_decision = Some(next_ctx);
                self.snapshot()
            }
            progress => {
                self.active_viewed_cards = None;
                self.clear_active_resolving_stack_object();
                self.priority_state.pending_continuation = None;
                if let Some(root_response) = self.pending_live_action_root.take() {
                    self.priority_epoch_has_undoable_action |=
                        Self::response_starts_cancelable_action_chain(&root_response);

                    if let Some(checkpoint) = self
                        .pending_action_checkpoint
                        .as_ref()
                        .or(action_checkpoint.as_ref())
                    {
                        let root = ReplayRoot::Response(root_response);
                        if Self::replay_root_has_irreversible_mana_activation(
                            &checkpoint.game,
                            &root,
                        ) || self.replay_root_mana_activation_added_to_stack(checkpoint, &root)
                        {
                            self.priority_epoch_undo_locked_by_mana = true;
                        }
                        self.priority_epoch_undo_land_stable_id =
                            self.committed_undo_land_stable_id(checkpoint, &root);
                    }
                }

                self.pending_action_checkpoint = None;
                self.pending_live_continuation = None;
                self.pending_replay_action = None;
                self.apply_progress(progress)?;
                self.snapshot()
            }
        }
    }

    fn dispatch_live_priority_response(
        &mut self,
        pending_ctx: DecisionContext,
        command: UiCommand,
    ) -> Result<JsValue, JsValue> {
        let response = match self.command_to_response(&pending_ctx, command) {
            Ok(response) => response,
            Err(err) => {
                self.pending_decision = Some(pending_ctx);
                return Err(err);
            }
        };

        let should_track_action_checkpoint = self.pending_action_checkpoint.is_none()
            && self.pending_live_action_root.is_none()
            && Self::response_starts_cancelable_action_chain(&response);
        let action_checkpoint =
            should_track_action_checkpoint.then(|| self.capture_replay_checkpoint());
        if should_track_action_checkpoint {
            self.pending_live_action_root = Some(response.clone());
        }

        let step_checkpoint = self.capture_replay_checkpoint_tagged("live_response_dm_capture");
        let mut live_dm = WasmReplayDecisionMaker::new(&[]);
        let result = apply_priority_response_with_dm(
            &mut self.game,
            &mut self.trigger_queue,
            &mut self.priority_state,
            &response,
            &mut live_dm,
        );
        let (pending_context, viewed_cards) = live_dm.finish();
        self.active_viewed_cards = viewed_cards;

        if let Some(next_ctx) = pending_context {
            self.sync_active_resolving_stack_object_for_prompt(Some(&step_checkpoint));
            if self.priority_action_chain_still_pending() {
                if let Some(checkpoint) = action_checkpoint {
                    self.pending_action_checkpoint.get_or_insert(checkpoint);
                }
            } else {
                self.pending_action_checkpoint = None;
            }
            self.priority_state.pending_continuation = None;
            self.pending_live_continuation = Some(LivePriorityContinuation {
                checkpoint: step_checkpoint,
                root: PendingPriorityContinuation::ApplyResponse(response),
                answers: Vec::new(),
            });
            self.pending_decision = Some(next_ctx);
            return self.snapshot();
        }

        match result {
            Ok(progress) => self.finish_live_priority_dispatch(
                progress,
                action_checkpoint,
                Some(step_checkpoint),
            ),
            Err(err) => {
                self.restore_replay_checkpoint(&step_checkpoint);
                if should_track_action_checkpoint {
                    self.pending_live_action_root = None;
                }
                self.pending_decision = Some(pending_ctx);
                Err(JsValue::from_str(&format!("dispatch failed: {err}")))
            }
        }
    }

    fn dispatch_live_priority_continuation(
        &mut self,
        pending_ctx: DecisionContext,
        command: UiCommand,
    ) -> Result<JsValue, JsValue> {
        let mut continuation = self
            .pending_live_continuation
            .take()
            .ok_or_else(|| JsValue::from_str("no live continuation checkpoint to resume"))?;
        let answer = match self.command_to_replay_answer(&pending_ctx, command) {
            Ok(answer) => answer,
            Err(err) => {
                self.pending_decision = Some(pending_ctx);
                self.pending_live_continuation = Some(continuation);
                return Err(err);
            }
        };
        continuation.answers.push(answer);

        // Diagnostic: record whether checkpoint has pending_activation before restore
        let checkpoint_diag_tag = continuation.checkpoint.diag_tag;
        let checkpoint_has_pa = continuation
            .checkpoint
            .priority_state
            .pending_activation
            .is_some();
        let checkpoint_pa_debug = continuation
            .checkpoint
            .priority_state
            .pending_activation
            .as_ref()
            .map(|p| {
                format!(
                    "stage={}, staged_remove={}, remaining_costs={}",
                    p.stage,
                    p.pending_remove_counters_among.is_some(),
                    p.remaining_cost_steps.len()
                )
            });
        let live_pa_before = self.priority_state.pending_activation.is_some();

        self.restore_replay_checkpoint(&continuation.checkpoint);
        self.priority_state.pending_continuation = None;

        let live_pa_after = self.priority_state.pending_activation.is_some();
        let mut live_dm = WasmReplayDecisionMaker::new(&continuation.answers);
        let result = match &continuation.root {
            PendingPriorityContinuation::ApplyResponse(response) => {
                apply_priority_response_with_dm(
                    &mut self.game,
                    &mut self.trigger_queue,
                    &mut self.priority_state,
                    response,
                    &mut live_dm,
                )
            }
            PendingPriorityContinuation::ApplyDecisionContext(ctx) => {
                apply_decision_context_with_dm(
                    &mut self.game,
                    &mut self.trigger_queue,
                    &mut self.priority_state,
                    ctx,
                    &mut live_dm,
                )
            }
        };
        let (pending_context, viewed_cards) = live_dm.finish();
        self.active_viewed_cards = viewed_cards;

        if let Some(next_ctx) = pending_context {
            self.sync_active_resolving_stack_object_for_prompt(Some(&continuation.checkpoint));
            self.priority_state.pending_continuation = None;
            continuation.checkpoint.diag_tag = "continuation_dm_capture";
            self.pending_live_continuation = Some(continuation);
            self.pending_decision = Some(next_ctx);
            return self.snapshot();
        }

        match result {
            Ok(progress) => self.finish_live_priority_dispatch(
                progress,
                None,
                Some(continuation.checkpoint.clone()),
            ),
            Err(err) => {
                self.restore_replay_checkpoint(&continuation.checkpoint);
                self.priority_state.pending_continuation = None;
                self.pending_live_continuation = Some(continuation);
                self.pending_decision = Some(pending_ctx);
                Err(JsValue::from_str(&format!(
                    "dispatch failed: {err} [diag: tag={checkpoint_diag_tag}, checkpoint_has_pa={checkpoint_has_pa}, \
                     checkpoint_pa={checkpoint_pa_debug:?}, \
                     live_pa_before={live_pa_before}, live_pa_after={live_pa_after}]"
                )))
            }
        }
    }

    fn capture_replay_checkpoint_tagged(&self, tag: &'static str) -> ReplayCheckpoint {
        ReplayCheckpoint {
            game: self.game.clone(),
            trigger_queue: self.trigger_queue.clone(),
            priority_state: self.priority_state.clone(),
            game_over: self.game_over.clone(),
            id_counters: snapshot_id_counters(),
            diag_tag: tag,
        }
    }

    fn capture_replay_checkpoint(&self) -> ReplayCheckpoint {
        self.capture_replay_checkpoint_tagged("untagged")
    }

    fn restore_replay_checkpoint(&mut self, checkpoint: &ReplayCheckpoint) {
        restore_id_counters(checkpoint.id_counters);
        self.game = checkpoint.game.clone();
        self.trigger_queue = checkpoint.trigger_queue.clone();
        self.priority_state = checkpoint.priority_state.clone();
        self.game_over = checkpoint.game_over.clone();
    }

    fn clear_active_resolving_stack_object(&mut self) {
        self.active_resolving_stack_object = None;
    }

    fn sync_active_resolving_stack_object(&mut self, checkpoint: Option<&ReplayCheckpoint>) {
        if let Some(checkpoint) = checkpoint {
            self.update_active_resolving_stack_object_from_checkpoint(checkpoint);
        } else {
            self.clear_active_resolving_stack_object();
        }
    }

    fn sync_active_resolving_stack_object_for_prompt(
        &mut self,
        checkpoint: Option<&ReplayCheckpoint>,
    ) {
        if self.priority_action_chain_still_pending() {
            self.clear_active_resolving_stack_object();
        } else {
            self.sync_active_resolving_stack_object(checkpoint);
        }
    }

    fn resolving_stack_object_from_checkpoint(
        &self,
        checkpoint: &ReplayCheckpoint,
    ) -> Option<StackObjectSnapshot> {
        let entry = checkpoint.game.stack.last()?;
        if checkpoint.game.stack.len() != self.game.stack.len() + 1 {
            return None;
        }
        if self
            .game
            .stack
            .iter()
            .any(|current| current.object_id == entry.object_id)
        {
            return None;
        }
        Some(build_stack_object_snapshot(
            &self.game,
            self.perspective,
            self.active_viewed_cards.as_ref(),
            entry,
        ))
    }

    fn update_active_resolving_stack_object_from_checkpoint(
        &mut self,
        checkpoint: &ReplayCheckpoint,
    ) {
        self.active_resolving_stack_object =
            self.resolving_stack_object_from_checkpoint(checkpoint);
    }

    fn execute_with_replay(
        &mut self,
        checkpoint: &ReplayCheckpoint,
        root: &ReplayRoot,
        nested_answers: &[ReplayDecisionAnswer],
    ) -> Result<ReplayOutcome, JsValue> {
        self.restore_replay_checkpoint(checkpoint);
        self.active_viewed_cards = None;
        self.clear_active_resolving_stack_object();

        let mut replay_dm = WasmReplayDecisionMaker::new(nested_answers);

        let result = match root {
            ReplayRoot::Response(response) => apply_priority_response_with_dm(
                &mut self.game,
                &mut self.trigger_queue,
                &mut self.priority_state,
                response,
                &mut replay_dm,
            )
            .map_err(|e| format!("{e}")),
            ReplayRoot::Advance => {
                advance_priority_with_dm(&mut self.game, &mut self.trigger_queue, &mut replay_dm)
                    .map_err(|e| format!("{e}"))
            }
        };

        let (pending_context, viewed_cards) = replay_dm.finish();
        self.active_viewed_cards = viewed_cards;

        if let Some(next_ctx) = pending_context {
            self.sync_active_resolving_stack_object_for_prompt(Some(checkpoint));
            return Ok(ReplayOutcome::NeedsDecision(next_ctx));
        }

        match result {
            Ok(progress) => {
                if matches!(progress, GameProgress::NeedsDecisionCtx(_)) {
                    self.sync_active_resolving_stack_object_for_prompt(Some(checkpoint));
                } else {
                    self.clear_active_resolving_stack_object();
                }
                Ok(ReplayOutcome::Complete(progress))
            }
            Err(e) => {
                self.active_viewed_cards = None;
                self.clear_active_resolving_stack_object();
                self.restore_replay_checkpoint(checkpoint);
                Err(JsValue::from_str(&format!("dispatch failed: {e}")))
            }
        }
    }

    fn command_to_replay_answer(
        &self,
        ctx: &DecisionContext,
        command: UiCommand,
    ) -> Result<ReplayDecisionAnswer, JsValue> {
        match (ctx, command) {
            (DecisionContext::Boolean(_), UiCommand::SelectOptions { option_indices }) => {
                validate_option_selection(1, Some(1), &option_indices, &[0usize, 1usize])?;
                let choice = option_indices
                    .first()
                    .copied()
                    .ok_or_else(|| JsValue::from_str("boolean choice requires one option"))?;
                Ok(ReplayDecisionAnswer::Boolean(choice == 1))
            }
            (DecisionContext::Number(number), UiCommand::NumberChoice { value }) => {
                if value < number.min || value > number.max {
                    return Err(JsValue::from_str(&format!(
                        "number out of range: expected {}..={}, got {}",
                        number.min, number.max, value
                    )));
                }
                Ok(ReplayDecisionAnswer::Number(value))
            }
            (
                DecisionContext::SelectOptions(options),
                UiCommand::SelectOptions { option_indices },
            ) => {
                let legal_indices: Vec<usize> = options
                    .options
                    .iter()
                    .filter(|o| o.legal)
                    .map(|o| o.index)
                    .collect();
                validate_option_selection(
                    options.min,
                    Some(options.max),
                    &option_indices,
                    &legal_indices,
                )?;
                Ok(ReplayDecisionAnswer::Options(option_indices))
            }
            (
                DecisionContext::Priority(priority),
                UiCommand::PriorityAction {
                    action_index,
                    action_ref,
                },
            ) => {
                let action = resolve_priority_action(priority, action_index, action_ref.as_ref())
                    .ok_or_else(|| {
                    if let Some(action_ref) = action_ref.as_ref() {
                        JsValue::from_str(&format!("invalid priority action ref: {action_ref:?}"))
                    } else if let Some(action_index) = action_index {
                        JsValue::from_str(&format!("invalid priority action index: {action_index}"))
                    } else {
                        JsValue::from_str("missing priority action selector")
                    }
                })?;
                Ok(ReplayDecisionAnswer::Priority(action))
            }
            (DecisionContext::SelectObjects(objects), UiCommand::SelectObjects { object_ids }) => {
                let legal_ids: Vec<u64> = objects
                    .candidates
                    .iter()
                    .filter(|obj| obj.legal)
                    .map(|obj| obj.id.0)
                    .collect();
                validate_object_selection(objects.min, objects.max, &object_ids, &legal_ids)?;
                Ok(ReplayDecisionAnswer::Objects(
                    object_ids
                        .into_iter()
                        .map(ObjectId::from_raw)
                        .collect::<Vec<_>>(),
                ))
            }
            (DecisionContext::Order(order), UiCommand::SelectOptions { option_indices }) => {
                let legal: Vec<usize> = (0..order.items.len()).collect();
                validate_option_selection(
                    order.items.len(),
                    Some(order.items.len()),
                    &option_indices,
                    &legal,
                )?;
                if unique_indices(&option_indices).len() != order.items.len() {
                    return Err(JsValue::from_str(
                        "ordering requires each option index exactly once",
                    ));
                }
                Ok(ReplayDecisionAnswer::Order(
                    option_indices
                        .into_iter()
                        .filter_map(|index| order.items.get(index).map(|(id, _)| *id))
                        .collect(),
                ))
            }
            (
                DecisionContext::Distribute(distribute),
                UiCommand::SelectOptions { option_indices },
            ) => {
                let legal: Vec<usize> = (0..distribute.targets.len()).collect();
                validate_option_selection(
                    0,
                    Some(distribute.total as usize),
                    &option_indices,
                    &legal,
                )?;

                if distribute.targets.is_empty() || distribute.total == 0 {
                    return Ok(ReplayDecisionAnswer::Distribute(Vec::new()));
                }

                let mut counts: HashMap<usize, u32> = HashMap::new();
                for index in option_indices {
                    *counts.entry(index).or_insert(0) += 1;
                }

                let total_assigned: u32 = counts.values().sum();
                if total_assigned != distribute.total {
                    return Err(JsValue::from_str(&format!(
                        "distribution must assign exactly {} total (got {})",
                        distribute.total, total_assigned
                    )));
                }

                if distribute.min_per_target > 0
                    && counts
                        .values()
                        .any(|amount| *amount > 0 && *amount < distribute.min_per_target)
                {
                    return Err(JsValue::from_str(&format!(
                        "each selected target must receive at least {}",
                        distribute.min_per_target
                    )));
                }

                let mut allocations: Vec<(Target, u32)> = Vec::new();
                for index in 0..distribute.targets.len() {
                    let Some(amount) = counts.get(&index).copied() else {
                        continue;
                    };
                    if amount == 0 {
                        continue;
                    }
                    allocations.push((distribute.targets[index].target, amount));
                }
                Ok(ReplayDecisionAnswer::Distribute(allocations))
            }
            (DecisionContext::Colors(colors), UiCommand::SelectOptions { option_indices }) => {
                if colors.count == 0 {
                    validate_option_selection(0, Some(0), &option_indices, &[])?;
                    return Ok(ReplayDecisionAnswer::Colors(Vec::new()));
                }

                let choices = colors_for_context(colors);
                if choices.is_empty() {
                    return Err(JsValue::from_str("no legal colors in colors decision"));
                }
                let legal: Vec<usize> = (0..choices.len()).collect();
                let max = if colors.same_color {
                    1
                } else {
                    colors.count as usize
                };
                validate_option_selection(1, Some(max), &option_indices, &legal)?;

                if colors.same_color {
                    let choice = option_indices.first().copied().ok_or_else(|| {
                        JsValue::from_str("color choice requires selecting one option")
                    })?;
                    let color = choices.get(choice).copied().ok_or_else(|| {
                        JsValue::from_str("selected color option is out of range")
                    })?;
                    return Ok(ReplayDecisionAnswer::Colors(vec![
                        color;
                        colors.count as usize
                    ]));
                }

                let mut selected: Vec<crate::color::Color> = option_indices
                    .iter()
                    .copied()
                    .into_iter()
                    .filter_map(|index| choices.get(index).copied())
                    .collect();
                if selected.is_empty() {
                    return Err(JsValue::from_str("choose at least one color"));
                }
                let desired = colors.count as usize;
                if selected.len() > desired {
                    selected.truncate(desired);
                }
                if selected.len() < desired {
                    let pad = selected[0];
                    selected.resize(desired, pad);
                }
                Ok(ReplayDecisionAnswer::Colors(selected))
            }
            (DecisionContext::Counters(counters), UiCommand::SelectOptions { option_indices }) => {
                let legal: Vec<usize> = counters
                    .available_counters
                    .iter()
                    .enumerate()
                    .filter(|(_, (_, available))| *available > 0)
                    .map(|(index, _)| index)
                    .collect();
                validate_option_selection(
                    0,
                    Some(counters.max_total as usize),
                    &option_indices,
                    &legal,
                )?;

                let mut counts: HashMap<usize, u32> = HashMap::new();
                for index in option_indices {
                    *counts.entry(index).or_insert(0) += 1;
                }

                let mut selected: Vec<(crate::object::CounterType, u32)> = Vec::new();
                for index in 0..counters.available_counters.len() {
                    let Some(chosen) = counts.get(&index).copied() else {
                        continue;
                    };
                    let Some((counter_type, available)) =
                        counters.available_counters.get(index).copied()
                    else {
                        continue;
                    };
                    if chosen > available {
                        return Err(JsValue::from_str(&format!(
                            "cannot remove {} of counter {} (only {} available)",
                            chosen,
                            counter_type.description(),
                            available
                        )));
                    }
                    if chosen > 0 {
                        selected.push((counter_type, chosen));
                    }
                }

                Ok(ReplayDecisionAnswer::Counters(selected))
            }
            (DecisionContext::Partition(partition), UiCommand::SelectObjects { object_ids }) => {
                let legal_ids: Vec<u64> = partition.cards.iter().map(|(id, _)| id.0).collect();
                validate_object_selection(0, Some(legal_ids.len()), &object_ids, &legal_ids)?;
                Ok(ReplayDecisionAnswer::Partition(
                    unique_object_ids(&object_ids)
                        .into_iter()
                        .map(ObjectId::from_raw)
                        .collect(),
                ))
            }
            (
                DecisionContext::Proliferate(proliferate),
                UiCommand::SelectOptions { option_indices },
            ) => {
                let permanent_count = proliferate.eligible_permanents.len();
                let total_options = permanent_count + proliferate.eligible_players.len();
                let legal: Vec<usize> = (0..total_options).collect();
                validate_option_selection(0, Some(total_options), &option_indices, &legal)?;

                let mut response = crate::decisions::specs::ProliferateResponse::default();
                for index in unique_indices(&option_indices) {
                    if index < permanent_count {
                        if let Some((permanent, _)) = proliferate.eligible_permanents.get(index) {
                            response.permanents.push(*permanent);
                        }
                        continue;
                    }
                    let player_index = index - permanent_count;
                    if let Some((player, _)) = proliferate.eligible_players.get(player_index) {
                        response.players.push(*player);
                    }
                }
                Ok(ReplayDecisionAnswer::Proliferate(response))
            }
            (DecisionContext::Targets(targets_ctx), UiCommand::SelectTargets { targets }) => {
                let converted = convert_and_validate_targets(targets_ctx, targets)?;
                Ok(ReplayDecisionAnswer::Targets(converted))
            }
            (
                DecisionContext::Attackers(attackers),
                UiCommand::DeclareAttackers { declarations },
            ) => {
                let converted = validate_attacker_declarations(attackers, &declarations)?
                    .into_iter()
                    .map(|declaration| crate::decisions::spec::AttackerDeclaration {
                        creature: declaration.creature,
                        target: declaration.target,
                    })
                    .collect();
                Ok(ReplayDecisionAnswer::Attackers(converted))
            }
            (DecisionContext::Blockers(blockers), UiCommand::DeclareBlockers { declarations }) => {
                let converted = validate_blocker_declarations(blockers, &declarations)?
                    .into_iter()
                    .map(|declaration| crate::decisions::spec::BlockerDeclaration {
                        blocker: declaration.blocker,
                        blocking: declaration.blocking,
                    })
                    .collect();
                Ok(ReplayDecisionAnswer::Blockers(converted))
            }
            (DecisionContext::Modes(modes), UiCommand::SelectOptions { option_indices }) => {
                let legal: Vec<usize> = modes
                    .spec
                    .modes
                    .iter()
                    .filter(|mode| mode.legal)
                    .map(|mode| mode.index)
                    .collect();
                validate_option_selection(
                    modes.spec.min_modes,
                    Some(modes.spec.max_modes),
                    &option_indices,
                    &legal,
                )?;
                Ok(ReplayDecisionAnswer::Options(option_indices))
            }
            (
                DecisionContext::HybridChoice(hybrid),
                UiCommand::SelectOptions { option_indices },
            ) => {
                let legal: Vec<usize> = hybrid.options.iter().map(|opt| opt.index).collect();
                validate_option_selection(1, Some(1), &option_indices, &legal)?;
                Ok(ReplayDecisionAnswer::Options(option_indices))
            }
            (ctx, _) => Err(JsValue::from_str(&format!(
                "command type does not match pending replay decision: {}",
                decision_context_kind(ctx)
            ))),
        }
    }

    fn command_to_response(
        &self,
        ctx: &DecisionContext,
        command: UiCommand,
    ) -> Result<PriorityResponse, JsValue> {
        match (ctx, command) {
            (
                DecisionContext::Priority(priority),
                UiCommand::PriorityAction {
                    action_index,
                    action_ref,
                },
            ) => {
                let action = resolve_priority_action(priority, action_index, action_ref.as_ref())
                    .ok_or_else(|| {
                    if let Some(action_ref) = action_ref.as_ref() {
                        JsValue::from_str(&format!("invalid priority action ref: {action_ref:?}"))
                    } else if let Some(action_index) = action_index {
                        JsValue::from_str(&format!("invalid priority action index: {action_index}"))
                    } else {
                        JsValue::from_str("missing priority action selector")
                    }
                })?;
                Ok(PriorityResponse::PriorityAction(action))
            }
            (DecisionContext::Number(number), UiCommand::NumberChoice { value }) => {
                if value < number.min || value > number.max {
                    return Err(JsValue::from_str(&format!(
                        "number out of range: expected {}..={}, got {}",
                        number.min, number.max, value
                    )));
                }
                if number.is_x_value {
                    Ok(PriorityResponse::XValue(value))
                } else {
                    Ok(PriorityResponse::NumberChoice(value))
                }
            }
            (
                DecisionContext::SelectOptions(options),
                UiCommand::SelectOptions { option_indices },
            ) => {
                let legal_indices: Vec<usize> = options
                    .options
                    .iter()
                    .filter(|o| o.legal)
                    .map(|o| o.index)
                    .collect();
                validate_option_selection(
                    options.min,
                    Some(options.max),
                    &option_indices,
                    &legal_indices,
                )?;
                self.map_select_options_response(option_indices)
            }
            (DecisionContext::Modes(modes), UiCommand::SelectOptions { option_indices }) => {
                let legal: Vec<usize> = modes
                    .spec
                    .modes
                    .iter()
                    .filter(|mode| mode.legal)
                    .map(|mode| mode.index)
                    .collect();
                validate_option_selection(
                    modes.spec.min_modes,
                    Some(modes.spec.max_modes),
                    &option_indices,
                    &legal,
                )?;
                Ok(PriorityResponse::Modes(option_indices))
            }
            (
                DecisionContext::HybridChoice(hybrid),
                UiCommand::SelectOptions { option_indices },
            ) => {
                let legal: Vec<usize> = hybrid.options.iter().map(|opt| opt.index).collect();
                validate_option_selection(1, Some(1), &option_indices, &legal)?;
                let choice = option_indices.first().copied().ok_or_else(|| {
                    JsValue::from_str("hybrid choice requires selecting one option")
                })?;
                Ok(PriorityResponse::HybridChoice(choice))
            }
            (DecisionContext::SelectObjects(objects), UiCommand::SelectObjects { object_ids }) => {
                let legal_ids: Vec<u64> = objects
                    .candidates
                    .iter()
                    .filter(|obj| obj.legal)
                    .map(|obj| obj.id.0)
                    .collect();
                validate_object_selection(objects.min, objects.max, &object_ids, &legal_ids)?;

                let chosen = object_ids.first().copied().ok_or_else(|| {
                    JsValue::from_str("select_objects requires one chosen object")
                })?;
                if let Some(pending) = self.priority_state.pending_activation.as_ref() {
                    match pending.stage {
                        ActivationStage::ChoosingSacrifice => Ok(
                            PriorityResponse::SacrificeTarget(ObjectId::from_raw(chosen)),
                        ),
                        ActivationStage::ChoosingCardCost => {
                            Ok(PriorityResponse::CardCostChoice(ObjectId::from_raw(chosen)))
                        }
                        _ => Err(JsValue::from_str(
                            "SelectObjects received while activation is not in an object-cost stage",
                        )),
                    }
                } else if self
                    .priority_state
                    .pending_cast
                    .as_ref()
                    .is_some_and(|pending| {
                        matches!(
                            pending.stage,
                            CastStage::ChoosingSacrifice | CastStage::ChoosingCardCost
                        )
                    })
                {
                    Ok(PriorityResponse::CardCostChoice(ObjectId::from_raw(chosen)))
                } else {
                    let cast_stage = self
                        .priority_state
                        .pending_cast
                        .as_ref()
                        .map(|p| p.stage.to_string());
                    let act_stage = self
                        .priority_state
                        .pending_activation
                        .as_ref()
                        .map(|p| p.stage.to_string());
                    Err(JsValue::from_str(&format!(
                        "unsupported SelectObjects context in priority flow \
                         (pending_cast={}, pending_activation={})",
                        cast_stage.as_deref().unwrap_or("none"),
                        act_stage.as_deref().unwrap_or("none"),
                    )))
                }
            }
            (DecisionContext::Targets(targets_ctx), UiCommand::SelectTargets { targets }) => {
                let converted = convert_and_validate_targets(targets_ctx, targets)?;
                Ok(PriorityResponse::Targets(converted))
            }
            (
                DecisionContext::Attackers(attackers),
                UiCommand::DeclareAttackers { declarations },
            ) => {
                let converted = validate_attacker_declarations(attackers, &declarations)?;
                Ok(PriorityResponse::Attackers(converted))
            }
            (DecisionContext::Blockers(blockers), UiCommand::DeclareBlockers { declarations }) => {
                let converted = validate_blocker_declarations(blockers, &declarations)?;
                Ok(PriorityResponse::Blockers {
                    defending_player: blockers.player,
                    declarations: converted,
                })
            }
            (DecisionContext::Modes(_), UiCommand::NumberChoice { .. })
            | (DecisionContext::Modes(_), UiCommand::SelectObjects { .. })
            | (DecisionContext::Modes(_), UiCommand::SelectTargets { .. })
            | (DecisionContext::Modes(_), UiCommand::DeclareAttackers { .. })
            | (DecisionContext::Modes(_), UiCommand::DeclareBlockers { .. })
            | (DecisionContext::HybridChoice(_), UiCommand::PriorityAction { .. })
            | (DecisionContext::HybridChoice(_), UiCommand::NumberChoice { .. })
            | (DecisionContext::HybridChoice(_), UiCommand::SelectObjects { .. })
            | (DecisionContext::HybridChoice(_), UiCommand::SelectTargets { .. })
            | (DecisionContext::HybridChoice(_), UiCommand::DeclareAttackers { .. })
            | (DecisionContext::HybridChoice(_), UiCommand::DeclareBlockers { .. })
            | (DecisionContext::SelectOptions(_), UiCommand::PriorityAction { .. })
            | (DecisionContext::SelectOptions(_), UiCommand::NumberChoice { .. })
            | (DecisionContext::SelectOptions(_), UiCommand::SelectObjects { .. })
            | (DecisionContext::SelectOptions(_), UiCommand::SelectTargets { .. })
            | (DecisionContext::SelectOptions(_), UiCommand::DeclareAttackers { .. })
            | (DecisionContext::SelectOptions(_), UiCommand::DeclareBlockers { .. })
            | (DecisionContext::SelectObjects(_), UiCommand::PriorityAction { .. })
            | (DecisionContext::SelectObjects(_), UiCommand::NumberChoice { .. })
            | (DecisionContext::SelectObjects(_), UiCommand::SelectOptions { .. })
            | (DecisionContext::SelectObjects(_), UiCommand::SelectTargets { .. })
            | (DecisionContext::SelectObjects(_), UiCommand::DeclareAttackers { .. })
            | (DecisionContext::SelectObjects(_), UiCommand::DeclareBlockers { .. })
            | (DecisionContext::Targets(_), UiCommand::PriorityAction { .. })
            | (DecisionContext::Targets(_), UiCommand::NumberChoice { .. })
            | (DecisionContext::Targets(_), UiCommand::SelectObjects { .. })
            | (DecisionContext::Targets(_), UiCommand::SelectOptions { .. })
            | (DecisionContext::Targets(_), UiCommand::DeclareAttackers { .. })
            | (DecisionContext::Targets(_), UiCommand::DeclareBlockers { .. })
            | (DecisionContext::Number(_), UiCommand::PriorityAction { .. })
            | (DecisionContext::Number(_), UiCommand::SelectOptions { .. })
            | (DecisionContext::Number(_), UiCommand::SelectObjects { .. })
            | (DecisionContext::Number(_), UiCommand::SelectTargets { .. })
            | (DecisionContext::Number(_), UiCommand::DeclareAttackers { .. })
            | (DecisionContext::Number(_), UiCommand::DeclareBlockers { .. })
            | (DecisionContext::Priority(_), UiCommand::NumberChoice { .. })
            | (DecisionContext::Priority(_), UiCommand::SelectOptions { .. })
            | (DecisionContext::Priority(_), UiCommand::SelectObjects { .. })
            | (DecisionContext::Priority(_), UiCommand::SelectTargets { .. })
            | (DecisionContext::Priority(_), UiCommand::DeclareAttackers { .. })
            | (DecisionContext::Priority(_), UiCommand::DeclareBlockers { .. })
            | (DecisionContext::Attackers(_), UiCommand::PriorityAction { .. })
            | (DecisionContext::Attackers(_), UiCommand::NumberChoice { .. })
            | (DecisionContext::Attackers(_), UiCommand::SelectOptions { .. })
            | (DecisionContext::Attackers(_), UiCommand::SelectObjects { .. })
            | (DecisionContext::Attackers(_), UiCommand::SelectTargets { .. })
            | (DecisionContext::Attackers(_), UiCommand::DeclareBlockers { .. })
            | (DecisionContext::Blockers(_), UiCommand::PriorityAction { .. })
            | (DecisionContext::Blockers(_), UiCommand::NumberChoice { .. })
            | (DecisionContext::Blockers(_), UiCommand::SelectOptions { .. })
            | (DecisionContext::Blockers(_), UiCommand::SelectObjects { .. })
            | (DecisionContext::Blockers(_), UiCommand::SelectTargets { .. })
            | (DecisionContext::Blockers(_), UiCommand::DeclareAttackers { .. }) => Err(
                JsValue::from_str("command type does not match pending decision"),
            ),
            (_, _) => Err(JsValue::from_str(&format!(
                "pending decision type is not yet supported in WASM dispatch: {}",
                decision_context_kind(ctx)
            ))),
        }
    }

    fn map_select_options_response(
        &self,
        option_indices: Vec<usize>,
    ) -> Result<PriorityResponse, JsValue> {
        if self.game.pending_replacement_choice.is_some() {
            let choice = option_indices.first().copied().unwrap_or(0);
            return Ok(PriorityResponse::ReplacementChoice(choice));
        }
        if self.priority_state.pending_method_selection.is_some() {
            let choice = option_indices.first().copied().ok_or_else(|| {
                JsValue::from_str("casting method choice requires one selected option")
            })?;
            return Ok(PriorityResponse::CastingMethodChoice(choice));
        }
        if self
            .priority_state
            .pending_cast
            .as_ref()
            .is_some_and(|pending| matches!(pending.stage, CastStage::ChoosingOptionalCosts))
        {
            let mut counts: HashMap<usize, u32> = HashMap::new();
            let mut order: Vec<usize> = Vec::new();
            for index in option_indices {
                if !counts.contains_key(&index) {
                    order.push(index);
                }
                *counts.entry(index).or_insert(0) += 1;
            }
            let choices: Vec<(usize, u32)> = order
                .into_iter()
                .filter_map(|index| counts.get(&index).copied().map(|count| (index, count)))
                .collect();
            return Ok(PriorityResponse::OptionalCosts(choices));
        }
        if self.priority_state.pending_mana_ability.is_some() {
            let choice = option_indices
                .first()
                .copied()
                .ok_or_else(|| JsValue::from_str("mana payment choice requires one option"))?;
            return Ok(PriorityResponse::ManaPayment(choice));
        }
        if self
            .priority_state
            .pending_activation
            .as_ref()
            .is_some_and(|pending| matches!(pending.stage, ActivationStage::ChoosingNextCost))
            || self
                .priority_state
                .pending_cast
                .as_ref()
                .is_some_and(|pending| matches!(pending.stage, CastStage::ChoosingNextCost))
        {
            let choice = option_indices
                .first()
                .copied()
                .ok_or_else(|| JsValue::from_str("next-cost choice requires one option"))?;
            return Ok(PriorityResponse::NextCostChoice(choice));
        }
        if self
            .priority_state
            .pending_activation
            .as_ref()
            .is_some_and(|pending| matches!(pending.stage, ActivationStage::PayingMana))
            || self
                .priority_state
                .pending_cast
                .as_ref()
                .is_some_and(|pending| matches!(pending.stage, CastStage::PayingMana))
        {
            let choice = option_indices
                .first()
                .copied()
                .ok_or_else(|| JsValue::from_str("mana pip payment requires one option"))?;
            return Ok(PriorityResponse::ManaPipPayment(choice));
        }

        // Build diagnostic info about current priority state.
        let cast_stage = self
            .priority_state
            .pending_cast
            .as_ref()
            .map(|p| p.stage.to_string());
        let act_stage = self
            .priority_state
            .pending_activation
            .as_ref()
            .map(|p| p.stage.to_string());
        Err(JsValue::from_str(&format!(
            "unsupported SelectOptions context in priority flow \
             (pending_cast={}, pending_activation={}, \
             pending_mana_ability={}, pending_method={}, replacement={})",
            cast_stage.as_deref().unwrap_or("none"),
            act_stage.as_deref().unwrap_or("none"),
            self.priority_state.pending_mana_ability.is_some(),
            self.priority_state.pending_method_selection.is_some(),
            self.game.pending_replacement_choice.is_some(),
        )))
    }
}

impl Default for WasmGame {
    fn default() -> Self {
        Self::new()
    }
}

fn build_object_details_snapshot(game: &GameState, id: ObjectId) -> Option<ObjectDetailsSnapshot> {
    let obj = game.object(id)?;
    let (power, toughness) = if obj.zone == Zone::Battlefield {
        (
            game.calculated_power(id).or_else(|| obj.power()),
            game.calculated_toughness(id).or_else(|| obj.toughness()),
        )
    } else {
        (obj.power(), obj.toughness())
    };
    let counters = counter_snapshots_for_object(obj);

    Some(ObjectDetailsSnapshot {
        id: obj.id.0,
        stable_id: obj.stable_id.0.0,
        name: obj.name.clone(),
        kind: obj.kind.to_string(),
        zone: zone_name(obj.zone),
        owner: obj.owner.0,
        controller: obj.controller.0,
        type_line: format_type_line(obj),
        mana_cost: obj.mana_cost.as_ref().map(|cost| cost.to_oracle()),
        oracle_text: obj.oracle_text.clone(),
        power,
        toughness,
        loyalty: obj.loyalty(),
        tapped: game.is_tapped(obj.id),
        counters,
        abilities: {
            let def = obj.to_card_definition();
            let mut lines = crate::compiled_text::compiled_lines(&def);
            for granted in obj.level_granted_abilities() {
                lines.push(format!("Level bonus: {}", granted.display()));
            }
            lines
        },
        raw_compilation: format!("{:#?}", obj.to_card_definition()),
        semantic_score: WasmGame::semantic_score_for_name(obj.name.as_str()),
    })
}

fn format_type_line(obj: &crate::object::Object) -> String {
    let mut left = Vec::new();
    left.extend(obj.supertypes.iter().map(|value| format!("{value:?}")));
    left.extend(obj.card_types.iter().map(|value| format!("{value:?}")));

    let mut type_line = left.join(" ");
    if !obj.subtypes.is_empty() {
        let subtypes = obj
            .subtypes
            .iter()
            .map(|value| format!("{value:?}"))
            .collect::<Vec<_>>()
            .join(" ");
        if type_line.is_empty() {
            type_line = subtypes;
        } else {
            type_line.push_str(" - ");
            type_line.push_str(&subtypes);
        }
    }

    if type_line.is_empty() {
        "Object".to_string()
    } else {
        type_line
    }
}

fn build_action_view(
    game: &GameState,
    perspective: PlayerId,
    viewed_cards: Option<&ActiveViewedCards>,
    index: usize,
    action: &LegalAction,
) -> ActionView {
    let (kind, object_id, ability_index, from_zone, to_zone) = action_drag_metadata(action);
    let source_visible = object_id
        .map(ObjectId::from_raw)
        .is_none_or(|id| object_visible_to_perspective(game, perspective, viewed_cards, id));
    ActionView {
        index,
        label: if source_visible {
            describe_action(game, action)
        } else {
            redacted_action_label(action)
        },
        kind: kind.to_string(),
        object_id: source_visible.then_some(object_id).flatten(),
        ability_index,
        from_zone: source_visible.then_some(from_zone).flatten(),
        to_zone: source_visible.then_some(to_zone).flatten(),
        action_ref: priority_action_ref(action),
    }
}

fn action_drag_metadata(
    action: &LegalAction,
) -> (
    &'static str,
    Option<u64>,
    Option<usize>,
    Option<String>,
    Option<String>,
) {
    match action {
        LegalAction::PassPriority => ("pass_priority", None, None, None, None),
        LegalAction::KeepOpeningHand => ("pass_priority", None, None, None, None),
        LegalAction::TakeMulligan => ("take_mulligan", None, None, None, None),
        LegalAction::SerumPowderMulligan { card_id } => (
            "serum_powder_mulligan",
            Some(card_id.0),
            None,
            Some(zone_name(Zone::Hand)),
            None,
        ),
        LegalAction::ContinuePregame => ("pass_priority", None, None, None, None),
        LegalAction::BeginGame => ("pass_priority", None, None, None, None),
        LegalAction::UsePregameAction { card_id, .. } => (
            "use_pregame_action",
            Some(card_id.0),
            None,
            Some(zone_name(Zone::Hand)),
            Some(zone_name(Zone::Battlefield)),
        ),
        LegalAction::PlayLand { land_id } => (
            "play_land",
            Some(land_id.0),
            None,
            Some(zone_name(Zone::Hand)),
            Some(zone_name(Zone::Battlefield)),
        ),
        LegalAction::CastSpell {
            spell_id,
            from_zone,
            ..
        } => (
            "cast_spell",
            Some(spell_id.0),
            None,
            Some(zone_name(*from_zone)),
            Some(zone_name(Zone::Stack)),
        ),
        LegalAction::ActivateAbility {
            source,
            ability_index,
        } => (
            "activate_ability",
            Some(source.0),
            Some(*ability_index),
            Some(zone_name(Zone::Battlefield)),
            Some(zone_name(Zone::Stack)),
        ),
        LegalAction::ActivateManaAbility {
            source,
            ability_index,
        } => (
            "activate_mana_ability",
            Some(source.0),
            Some(*ability_index),
            Some(zone_name(Zone::Battlefield)),
            None,
        ),
        LegalAction::TurnFaceUp { creature_id } => (
            "turn_face_up",
            Some(creature_id.0),
            None,
            Some(zone_name(Zone::Battlefield)),
            Some(zone_name(Zone::Battlefield)),
        ),
        LegalAction::SpecialAction(_) => ("special_action", None, None, None, None),
    }
}

fn zone_name(zone: Zone) -> String {
    match zone {
        Zone::Library => "library",
        Zone::Hand => "hand",
        Zone::Battlefield => "battlefield",
        Zone::Graveyard => "graveyard",
        Zone::Exile => "exile",
        Zone::Stack => "stack",
        Zone::Command => "command",
    }
    .to_string()
}

fn describe_action(game: &GameState, action: &LegalAction) -> String {
    match action {
        LegalAction::PassPriority => "Pass priority".to_string(),
        LegalAction::KeepOpeningHand => "Keep hand".to_string(),
        LegalAction::TakeMulligan => "Mulligan".to_string(),
        LegalAction::SerumPowderMulligan { card_id } => {
            format!("Use {}", object_name(game, *card_id))
        }
        LegalAction::ContinuePregame => "Continue".to_string(),
        LegalAction::BeginGame => "Begin game".to_string(),
        LegalAction::UsePregameAction { card_id, .. } => {
            format!("Begin with {}", object_name(game, *card_id))
        }
        LegalAction::PlayLand { land_id } => {
            format!("Play {}", object_name(game, *land_id))
        }
        LegalAction::CastSpell {
            spell_id,
            from_zone,
            casting_method,
        } => {
            let name = object_name(game, *spell_id);
            let mut qualifiers = Vec::new();

            match casting_method {
                crate::alternative_cast::CastingMethod::Normal => {
                    if *from_zone != Zone::Hand {
                        qualifiers.push(format!("from {}", zone_display_name(*from_zone)));
                    }
                }
                crate::alternative_cast::CastingMethod::SplitOtherHalf => {
                    qualifiers.push("other half".to_string());
                }
                crate::alternative_cast::CastingMethod::Fuse => {
                    qualifiers.push("fuse".to_string());
                }
                crate::alternative_cast::CastingMethod::Alternative(index) => {
                    let method_name = game
                        .object(*spell_id)
                        .and_then(|obj| obj.alternative_casts.get(*index))
                        .map(|m| m.name().to_ascii_lowercase())
                        .unwrap_or_else(|| format!("alternative #{index}"));
                    qualifiers.push(method_name);
                }
                crate::alternative_cast::CastingMethod::GrantedEscape { .. } => {
                    qualifiers.push("escape".to_string());
                }
                crate::alternative_cast::CastingMethod::GrantedFlashback => {
                    qualifiers.push("flashback".to_string());
                }
                crate::alternative_cast::CastingMethod::PlayFrom {
                    zone,
                    use_alternative,
                    ..
                } => {
                    if let Some(index) = use_alternative {
                        let alt = game
                            .object(*spell_id)
                            .and_then(|obj| {
                                crate::decision::resolve_play_from_alternative_method(
                                    game,
                                    game.turn.priority_player.unwrap_or(obj.owner),
                                    obj,
                                    *zone,
                                    *index,
                                )
                            })
                            .map(|m| m.name().to_ascii_lowercase())
                            .unwrap_or_else(|| format!("alternative #{index}"));
                        qualifiers.push(alt);
                    }
                    qualifiers.push(format!("from {}", zone_display_name(*zone)));
                }
            }

            if qualifiers.is_empty() {
                format!("Cast {}", name)
            } else {
                format!("Cast {} ({})", name, qualifiers.join(", "))
            }
        }
        LegalAction::ActivateAbility {
            source,
            ability_index,
        } => {
            let name = object_name(game, *source);
            let ability_text = game
                .current_ability(*source, *ability_index)
                .and_then(|ability| ability.text)
                .map(|text| normalize_action_text(&text));
            match ability_text {
                Some(text) => format!("Activate {}: {}", name, text),
                None => format!("Activate {} ability #{}", name, ability_index + 1),
            }
        }
        LegalAction::ActivateManaAbility {
            source,
            ability_index,
        } => {
            let name = object_name(game, *source);
            let ability_text = game
                .current_ability(*source, *ability_index)
                .and_then(|ability| ability.text)
                .map(|text| normalize_action_text(&text));
            match ability_text {
                Some(text) => format!("Activate {}: {}", name, text),
                None => format!(
                    "Activate mana ability on {} (# {})",
                    name,
                    ability_index + 1
                ),
            }
        }
        LegalAction::TurnFaceUp { creature_id } => {
            format!("Turn face up {}", object_name(game, *creature_id))
        }
        LegalAction::SpecialAction(action) => match action {
            crate::special_actions::SpecialAction::PlayLand { card_id } => {
                format!("Play {}", object_name(game, *card_id))
            }
            crate::special_actions::SpecialAction::TurnFaceUp { permanent_id } => {
                format!("Turn face up {}", object_name(game, *permanent_id))
            }
            crate::special_actions::SpecialAction::Suspend { card_id } => {
                format!("Suspend {}", object_name(game, *card_id))
            }
            crate::special_actions::SpecialAction::Foretell { card_id } => {
                format!("Foretell {}", object_name(game, *card_id))
            }
            crate::special_actions::SpecialAction::Plot { card_id } => {
                format!("Plot {}", object_name(game, *card_id))
            }
            crate::special_actions::SpecialAction::ActivateManaAbility { permanent_id, .. } => {
                format!(
                    "Activate mana ability on {}",
                    object_name(game, *permanent_id)
                )
            }
        },
    }
}

fn zone_display_name(zone: Zone) -> &'static str {
    match zone {
        Zone::Library => "library",
        Zone::Hand => "hand",
        Zone::Battlefield => "battlefield",
        Zone::Graveyard => "graveyard",
        Zone::Exile => "exile",
        Zone::Stack => "stack",
        Zone::Command => "command zone",
    }
}

fn object_name(game: &GameState, id: ObjectId) -> String {
    game.object(id)
        .map(|o| o.name.clone())
        .unwrap_or_else(|| format!("Object#{}", id.0))
}

fn hidden_object_label() -> String {
    "Hidden card".to_string()
}

const JS_SAFE_INTEGER_MAX: u64 = 9_007_199_254_740_991;

fn redacted_choice_id(index: usize) -> u64 {
    JS_SAFE_INTEGER_MAX.saturating_sub(index as u64)
}

fn object_visible_to_perspective(
    game: &GameState,
    perspective: PlayerId,
    viewed_cards: Option<&ActiveViewedCards>,
    id: ObjectId,
) -> bool {
    let Some(obj) = game.object(id) else {
        return false;
    };

    if !obj.zone.is_hidden() || obj.owner == perspective {
        return true;
    }

    viewed_cards
        .is_some_and(|view| (view.public || view.viewer == perspective) && view.cards.contains(&id))
}

fn redacted_action_label(action: &LegalAction) -> String {
    match action {
        LegalAction::CastSpell { .. } => "Cast hidden spell".to_string(),
        LegalAction::PlayLand { .. } => "Play hidden land".to_string(),
        LegalAction::UsePregameAction { .. } => "Use hidden pregame action".to_string(),
        LegalAction::SerumPowderMulligan { .. } => "Use hidden mulligan action".to_string(),
        _ => "Hidden action".to_string(),
    }
}

fn optional_cost_selection_metadata(
    game: &GameState,
    source: Option<ObjectId>,
    option_index: usize,
) -> (bool, Option<u32>) {
    let Some(source_id) = source else {
        return (false, None);
    };
    let Some(obj) = game.object(source_id) else {
        return (false, None);
    };
    let Some(optional_cost) = obj.optional_costs.get(option_index) else {
        return (false, None);
    };
    if optional_cost.repeatable {
        // Keep a practical cap for UI count inputs. Engine legality remains authoritative.
        (true, Some(32))
    } else {
        (false, Some(1))
    }
}

fn priority_action_ref(action: &LegalAction) -> PriorityActionRef {
    match action {
        LegalAction::PassPriority => PriorityActionRef::PassPriority,
        LegalAction::KeepOpeningHand => PriorityActionRef::KeepOpeningHand,
        LegalAction::TakeMulligan => PriorityActionRef::TakeMulligan,
        LegalAction::SerumPowderMulligan { card_id } => {
            PriorityActionRef::SerumPowderMulligan { card_id: card_id.0 }
        }
        LegalAction::ContinuePregame => PriorityActionRef::ContinuePregame,
        LegalAction::BeginGame => PriorityActionRef::BeginGame,
        LegalAction::UsePregameAction {
            card_id,
            ability_index,
        } => PriorityActionRef::UsePregameAction {
            card_id: card_id.0,
            ability_index: *ability_index,
        },
        LegalAction::CastSpell {
            spell_id,
            from_zone,
            casting_method,
        } => PriorityActionRef::CastSpell {
            spell_id: spell_id.0,
            from_zone: zone_name(*from_zone),
            casting_method: casting_method_ref(casting_method),
        },
        LegalAction::ActivateAbility {
            source,
            ability_index,
        } => PriorityActionRef::ActivateAbility {
            source: source.0,
            ability_index: *ability_index,
        },
        LegalAction::PlayLand { land_id } => PriorityActionRef::PlayLand { land_id: land_id.0 },
        LegalAction::ActivateManaAbility {
            source,
            ability_index,
        } => PriorityActionRef::ActivateManaAbility {
            source: source.0,
            ability_index: *ability_index,
        },
        LegalAction::TurnFaceUp { creature_id } => PriorityActionRef::TurnFaceUp {
            creature_id: creature_id.0,
        },
        LegalAction::SpecialAction(action) => PriorityActionRef::SpecialAction {
            action: special_action_ref(action),
        },
    }
}

fn special_action_ref(action: &crate::special_actions::SpecialAction) -> SpecialActionRef {
    match action {
        crate::special_actions::SpecialAction::PlayLand { card_id } => {
            SpecialActionRef::PlayLand { card_id: card_id.0 }
        }
        crate::special_actions::SpecialAction::TurnFaceUp { permanent_id } => {
            SpecialActionRef::TurnFaceUp {
                permanent_id: permanent_id.0,
            }
        }
        crate::special_actions::SpecialAction::Suspend { card_id } => {
            SpecialActionRef::Suspend { card_id: card_id.0 }
        }
        crate::special_actions::SpecialAction::Foretell { card_id } => {
            SpecialActionRef::Foretell { card_id: card_id.0 }
        }
        crate::special_actions::SpecialAction::Plot { card_id } => {
            SpecialActionRef::Plot { card_id: card_id.0 }
        }
        crate::special_actions::SpecialAction::ActivateManaAbility {
            permanent_id,
            ability_index,
        } => SpecialActionRef::ActivateManaAbility {
            permanent_id: permanent_id.0,
            ability_index: *ability_index,
        },
    }
}

fn casting_method_ref(method: &crate::alternative_cast::CastingMethod) -> CastingMethodRef {
    match method {
        crate::alternative_cast::CastingMethod::Normal => CastingMethodRef::Normal,
        crate::alternative_cast::CastingMethod::SplitOtherHalf => CastingMethodRef::SplitOtherHalf,
        crate::alternative_cast::CastingMethod::Fuse => CastingMethodRef::Fuse,
        crate::alternative_cast::CastingMethod::Alternative(index) => {
            CastingMethodRef::Alternative { index: *index }
        }
        crate::alternative_cast::CastingMethod::GrantedEscape {
            source,
            exile_count,
        } => CastingMethodRef::GrantedEscape {
            source: source.0,
            exile_count: *exile_count,
        },
        crate::alternative_cast::CastingMethod::GrantedFlashback => {
            CastingMethodRef::GrantedFlashback
        }
        crate::alternative_cast::CastingMethod::PlayFrom {
            source,
            zone,
            use_alternative,
        } => CastingMethodRef::PlayFrom {
            source: source.0,
            zone: zone_name(*zone),
            use_alternative: *use_alternative,
        },
    }
}

fn resolve_priority_action(
    priority: &crate::decisions::context::PriorityContext,
    action_index: Option<usize>,
    action_ref: Option<&PriorityActionRef>,
) -> Option<LegalAction> {
    if let Some(action_ref) = action_ref {
        return priority
            .actions
            .iter()
            .find(|action| priority_action_ref(action) == *action_ref)
            .cloned();
    }
    action_index.and_then(|index| priority.actions.get(index).cloned())
}

/// Derive a short structured reason label from a DecisionContext.
fn decision_reason(ctx: &DecisionContext) -> Option<String> {
    match ctx {
        DecisionContext::Boolean(b) => {
            let d = b.description.to_lowercase();
            if d.contains("ward") {
                Some("Ward".into())
            } else if d.contains("miracle") {
                Some("Miracle".into())
            } else if d.contains("madness") {
                Some("Madness".into())
            } else if d.contains("new targets") {
                Some("Retarget".into())
            } else if d.starts_with("you may") || d.starts_with("may ") {
                Some("May ability".into())
            } else {
                None
            }
        }
        DecisionContext::Number(n) => {
            if n.is_x_value {
                Some("X value".into())
            } else {
                Some("Choose number".into())
            }
        }
        DecisionContext::SelectOptions(o) => {
            let d = o.description.to_lowercase();
            if d.contains("replacement") {
                Some("Replacement effect".into())
            } else if d.contains("choose the next cost to pay") {
                Some("Next cost".into())
            } else if d.contains("optional cost") {
                Some("Additional costs".into())
            } else {
                None
            }
        }
        DecisionContext::Modes(_) => Some("Modal choice".into()),
        DecisionContext::HybridChoice(_) => Some("Mana payment".into()),
        DecisionContext::Order(o) => {
            let d = o.description.to_lowercase();
            if d.contains("blocker") {
                Some("Order blockers".into())
            } else if d.contains("attacker") {
                Some("Order attackers".into())
            } else if d.contains("trigger") {
                Some("Order triggers".into())
            } else {
                Some("Ordering".into())
            }
        }
        DecisionContext::Distribute(_) => Some("Distribute".into()),
        DecisionContext::Colors(_) => Some("Choose color".into()),
        DecisionContext::Counters(_) => Some("Remove counters".into()),
        DecisionContext::Partition(p) => {
            let d = p.description.to_lowercase();
            if d.starts_with("surveil") {
                Some("Surveil".into())
            } else {
                Some("Scry".into())
            }
        }
        DecisionContext::Proliferate(_) => Some("Proliferate".into()),
        DecisionContext::SelectObjects(o) => {
            let d = o.description.to_lowercase();
            if d.contains("sacrifice") {
                Some("Sacrifice".into())
            } else if d.contains("discard") {
                Some("Discard".into())
            } else if d.contains("exile") {
                Some("Exile".into())
            } else if d.contains("search") {
                Some("Search library".into())
            } else if d.contains("legend rule") {
                Some("Legend rule".into())
            } else if d.contains("destroy") {
                Some("Destroy".into())
            } else if d.contains("return") {
                Some("Return".into())
            } else {
                None
            }
        }
        DecisionContext::Targets(_) => Some("Choose targets".into()),
        DecisionContext::Priority(_)
        | DecisionContext::Attackers(_)
        | DecisionContext::Blockers(_) => None,
    }
}

fn normalize_action_text(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn target_choice_view(
    game: &GameState,
    perspective: PlayerId,
    viewed_cards: Option<&ActiveViewedCards>,
    index: usize,
    target: &Target,
) -> TargetChoiceView {
    match target {
        Target::Player(pid) => TargetChoiceView::Player {
            player: pid.0,
            name: game
                .player(*pid)
                .map(|p| p.name.clone())
                .unwrap_or_else(|| format!("Player {}", pid.0 + 1)),
        },
        Target::Object(id) => {
            let visible = object_visible_to_perspective(game, perspective, viewed_cards, *id);
            TargetChoiceView::Object {
                object: if visible {
                    id.0
                } else {
                    redacted_choice_id(index)
                },
                name: if visible {
                    object_name(game, *id)
                } else {
                    hidden_object_label()
                },
            }
        }
    }
}

fn attack_target_view(game: &GameState, target: &AttackTarget) -> AttackTargetView {
    match target {
        AttackTarget::Player(pid) => AttackTargetView::Player {
            player: pid.0,
            name: game
                .player(*pid)
                .map(|p| p.name.clone())
                .unwrap_or_else(|| format!("Player {}", pid.0 + 1)),
        },
        AttackTarget::Planeswalker(id) => AttackTargetView::Planeswalker {
            object: id.0,
            name: object_name(game, *id),
        },
    }
}

fn attack_target_from_input(input: &AttackTargetInput) -> AttackTarget {
    match input {
        AttackTargetInput::Player { player } => AttackTarget::Player(PlayerId::from_index(*player)),
        AttackTargetInput::Planeswalker { object } => {
            AttackTarget::Planeswalker(ObjectId::from_raw(*object))
        }
    }
}

fn colors_for_context(ctx: &crate::decisions::context::ColorsContext) -> Vec<crate::color::Color> {
    if let Some(available) = &ctx.available_colors {
        if !available.is_empty() {
            return available.clone();
        }
    }
    crate::color::Color::ALL.to_vec()
}

fn color_name(color: crate::color::Color) -> &'static str {
    match color {
        crate::color::Color::White => "White",
        crate::color::Color::Blue => "Blue",
        crate::color::Color::Black => "Black",
        crate::color::Color::Red => "Red",
        crate::color::Color::Green => "Green",
    }
}

fn unique_indices(indices: &[usize]) -> Vec<usize> {
    let mut unique = Vec::new();
    let mut seen = HashSet::new();
    for &index in indices {
        if seen.insert(index) {
            unique.push(index);
        }
    }
    unique
}

fn unique_object_ids(ids: &[u64]) -> Vec<u64> {
    let mut unique = Vec::new();
    let mut seen = HashSet::new();
    for &id in ids {
        if seen.insert(id) {
            unique.push(id);
        }
    }
    unique
}

fn decision_context_kind(ctx: &DecisionContext) -> &'static str {
    match ctx {
        DecisionContext::Boolean(_) => "boolean",
        DecisionContext::Number(_) => "number",
        DecisionContext::SelectObjects(_) => "select_objects",
        DecisionContext::SelectOptions(_) => "select_options",
        DecisionContext::Modes(_) => "modes",
        DecisionContext::HybridChoice(_) => "hybrid_choice",
        DecisionContext::Order(_) => "order",
        DecisionContext::Attackers(_) => "attackers",
        DecisionContext::Blockers(_) => "blockers",
        DecisionContext::Distribute(_) => "distribute",
        DecisionContext::Colors(_) => "colors",
        DecisionContext::Counters(_) => "counters",
        DecisionContext::Partition(_) => "partition",
        DecisionContext::Proliferate(_) => "proliferate",
        DecisionContext::Priority(_) => "priority",
        DecisionContext::Targets(_) => "targets",
    }
}

fn replay_decision_requires_root_reexecution(ctx: &DecisionContext) -> bool {
    matches!(
        ctx,
        DecisionContext::Boolean(_)
            | DecisionContext::Order(_)
            | DecisionContext::Distribute(_)
            | DecisionContext::Colors(_)
            | DecisionContext::Counters(_)
            | DecisionContext::Partition(_)
            | DecisionContext::Proliferate(_)
    )
}

fn validate_attacker_declarations(
    attackers: &crate::decisions::context::AttackersContext,
    declarations: &[AttackerDeclarationInput],
) -> Result<Vec<AttackerDeclaration>, JsValue> {
    let options: HashMap<u64, &crate::decisions::context::AttackerOptionContext> = attackers
        .attacker_options
        .iter()
        .map(|option| (option.creature.0, option))
        .collect();
    let mut declared_creatures = HashSet::new();
    let mut converted = Vec::new();

    for declaration in declarations {
        let Some(option) = options.get(&declaration.creature) else {
            return Err(JsValue::from_str(&format!(
                "invalid attacker creature id: {}",
                declaration.creature
            )));
        };
        if !declared_creatures.insert(declaration.creature) {
            return Err(JsValue::from_str(&format!(
                "attacker declared twice: {}",
                declaration.creature
            )));
        }

        let target = attack_target_from_input(&declaration.target);
        if !option.valid_targets.contains(&target) {
            return Err(JsValue::from_str(&format!(
                "invalid attack target for creature {}",
                declaration.creature
            )));
        }

        converted.push(AttackerDeclaration {
            creature: ObjectId::from_raw(declaration.creature),
            target,
        });
    }

    for option in &attackers.attacker_options {
        if option.must_attack && !declared_creatures.contains(&option.creature.0) {
            return Err(JsValue::from_str(&format!(
                "{} must attack if able",
                option.creature_name
            )));
        }
    }

    Ok(converted)
}

fn validate_blocker_declarations(
    blockers: &crate::decisions::context::BlockersContext,
    declarations: &[BlockerDeclarationInput],
) -> Result<Vec<BlockerDeclaration>, JsValue> {
    let options: HashMap<u64, &crate::decisions::context::BlockerOptionContext> = blockers
        .blocker_options
        .iter()
        .map(|option| (option.attacker.0, option))
        .collect();

    // Compute per-blocker max assignments: the number of distinct attacker options
    // that list this blocker as valid (i.e. how many attackers it can block).
    let mut blocker_max_assignments: HashMap<u64, usize> = HashMap::new();
    for option in &blockers.blocker_options {
        for (blocker_id, _) in &option.valid_blockers {
            *blocker_max_assignments.entry(blocker_id.0).or_insert(0) += 1;
        }
    }

    let mut blocker_assignment_count: HashMap<u64, usize> = HashMap::new();
    let mut blocker_attacker_pairs: HashSet<(u64, u64)> = HashSet::new();
    let mut counts_by_attacker: HashMap<u64, usize> = HashMap::new();
    let mut converted = Vec::new();

    for declaration in declarations {
        let Some(option) = options.get(&declaration.blocking) else {
            return Err(JsValue::from_str(&format!(
                "invalid blocking attacker id: {}",
                declaration.blocking
            )));
        };
        if !option
            .valid_blockers
            .iter()
            .any(|(id, _)| id.0 == declaration.blocker)
        {
            return Err(JsValue::from_str(&format!(
                "invalid blocker {} for attacker {}",
                declaration.blocker, declaration.blocking
            )));
        }
        // Reject duplicate (blocker, attacker) pairs.
        if !blocker_attacker_pairs.insert((declaration.blocker, declaration.blocking)) {
            return Err(JsValue::from_str(&format!(
                "blocker {} already assigned to attacker {}",
                declaration.blocker, declaration.blocking
            )));
        }
        // Check per-blocker assignment limit.
        let count = blocker_assignment_count
            .entry(declaration.blocker)
            .or_insert(0);
        *count += 1;
        let max = blocker_max_assignments
            .get(&declaration.blocker)
            .copied()
            .unwrap_or(1);
        if *count > max {
            return Err(JsValue::from_str(&format!(
                "blocker {} cannot block more than {} attacker(s)",
                declaration.blocker, max
            )));
        }
        *counts_by_attacker.entry(declaration.blocking).or_insert(0) += 1;
        converted.push(BlockerDeclaration {
            blocker: ObjectId::from_raw(declaration.blocker),
            blocking: ObjectId::from_raw(declaration.blocking),
        });
    }

    for option in &blockers.blocker_options {
        let assigned = counts_by_attacker
            .get(&option.attacker.0)
            .copied()
            .unwrap_or(0);
        // "Minimum blockers" applies only when the attacker is blocked at all.
        // Example: menace means if blocked, it must be by 2+, but not blocked is legal.
        if assigned > 0 && assigned < option.min_blockers {
            return Err(JsValue::from_str(&format!(
                "{} requires at least {} blocker(s)",
                option.attacker_name, option.min_blockers
            )));
        }
    }

    Ok(converted)
}

fn validate_option_selection(
    min: usize,
    max: Option<usize>,
    selected: &[usize],
    legal_indices: &[usize],
) -> Result<(), JsValue> {
    if selected.len() < min {
        return Err(JsValue::from_str(&format!(
            "must select at least {min} option(s)"
        )));
    }
    if let Some(max) = max
        && selected.len() > max
    {
        return Err(JsValue::from_str(&format!(
            "must select at most {max} option(s)"
        )));
    }
    for selected_index in selected {
        if !legal_indices.contains(selected_index) {
            return Err(JsValue::from_str(&format!(
                "option index {selected_index} is not legal"
            )));
        }
    }
    Ok(())
}

fn validate_object_selection(
    min: usize,
    max: Option<usize>,
    selected: &[u64],
    legal_ids: &[u64],
) -> Result<(), JsValue> {
    if selected.len() < min {
        return Err(JsValue::from_str(&format!(
            "must select at least {min} object(s)"
        )));
    }
    if let Some(max) = max
        && selected.len() > max
    {
        return Err(JsValue::from_str(&format!(
            "must select at most {max} object(s)"
        )));
    }
    for object_id in selected {
        if !legal_ids.contains(object_id) {
            return Err(JsValue::from_str(&format!(
                "object id {object_id} is not legal"
            )));
        }
    }
    Ok(())
}

/// Convert and validate target inputs against the requirements in a TargetsContext.
///
/// Validates that:
/// - Each selected target is legal in at least one requirement
/// - The flattened target list can be assigned to the requirements in order
fn convert_and_validate_targets(
    ctx: &crate::decisions::context::TargetsContext,
    inputs: Vec<TargetInput>,
) -> Result<Vec<Target>, JsValue> {
    let converted: Vec<Target> = inputs
        .into_iter()
        .map(|target| match target {
            TargetInput::Player { player } => Target::Player(PlayerId::from_index(player)),
            TargetInput::Object { object } => Target::Object(ObjectId::from_raw(object)),
        })
        .collect();

    // Build the set of all legal targets across all requirements.
    let all_legal: HashSet<Target> = ctx
        .requirements
        .iter()
        .flat_map(|req| req.legal_targets.iter().copied())
        .collect();

    // Validate every chosen target is legal somewhere.
    for target in &converted {
        if !all_legal.contains(target) {
            return Err(JsValue::from_str(&format!(
                "target {} is not a legal choice",
                match target {
                    Target::Player(p) => format!("player {}", p.0),
                    Target::Object(o) => format!("object {}", o.0),
                }
            )));
        }
    }

    if !validate_flat_target_assignment(&ctx.requirements, &converted) {
        return Err(JsValue::from_str(
            "targets do not satisfy the targeting requirements in order",
        ));
    }

    Ok(converted)
}

#[cfg(test)]
mod tests {
    use super::{
        GameSnapshot, MatchFormatInput, PendingReplayAction, PregameState, ReplayOutcome,
        ReplayRoot, TargetChoiceView, TargetInput, WasmGame, build_object_details_snapshot,
        build_stack_object_snapshot, convert_and_validate_targets,
    };
    use crate::ability::Ability;
    use crate::alternative_cast::CastingMethod;
    use crate::card::CardBuilder;
    use crate::cards::CardRegistry;
    use crate::cards::definitions::{
        basic_island, basic_mountain, emrakul_the_promised_end, gemstone_caverns, grizzly_bears,
        lightning_bolt, ornithopter, polluted_delta, serum_powder, urzas_saga,
        yawgmoth_thran_physician,
    };
    use crate::continuous::ContinuousEffect;
    use crate::cost::OptionalCostsPaid;
    use crate::decision::compute_legal_actions;
    use crate::decision::{GameProgress, LegalAction};
    use crate::decisions::context::{
        BooleanContext, DecisionContext, PriorityContext, SelectObjectsContext, SelectableObject,
        SelectableOption, TargetRequirementContext, TargetsContext,
    };
    use crate::effect::{Effect, Until};
    use crate::events::spells::SpellCastEvent;
    use crate::game_loop::{CastStage, PendingCast, PendingManaAbility, PriorityResponse};
    use crate::game_state::{GameState, Phase, StackEntry, Step, Target};
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::object::CounterType;
    use crate::provenance::ProvNodeId;
    use crate::triggers::{Trigger, TriggerEvent, check_triggers};
    use crate::types::CardType;
    use crate::wasm_api::colors_for_context;
    use crate::zone::Zone;
    use serde::Deserialize;
    use serde_json::json;

    fn setup_pregame_match(format: MatchFormatInput) -> WasmGame {
        let mut wasm = WasmGame::new();
        wasm.initialize_empty_match(vec!["Alice".to_string(), "Bob".to_string()], 20, 1);
        wasm.match_format = format;
        wasm
    }

    fn seed_filler_cards(
        wasm: &mut WasmGame,
        player: PlayerId,
        zone: Zone,
        count: usize,
    ) -> Vec<ObjectId> {
        (0..count)
            .map(|_| {
                wasm.game
                    .create_object_from_definition(&ornithopter(), player, zone)
            })
            .collect()
    }

    fn start_pregame(wasm: &mut WasmGame, opening_hand_size: usize, format: MatchFormatInput) {
        wasm.pregame = Some(PregameState::new(
            &wasm.game.turn_order,
            opening_hand_size,
            format,
        ));
        wasm.advance_until_decision()
            .expect("pregame should produce a decision");
    }

    fn dispatch_matching_priority_action<F>(wasm: &mut WasmGame, predicate: F)
    where
        F: FnMut(&LegalAction) -> bool,
    {
        let index = match wasm.pending_decision.as_ref() {
            Some(DecisionContext::Priority(ctx)) => ctx
                .actions
                .iter()
                .position(predicate)
                .expect("expected matching priority action"),
            other => panic!("expected priority decision, got {other:?}"),
        };
        wasm.dispatch(
            serde_wasm_bindgen::to_value(&json!({
                "type": "priority_action",
                "action_index": index,
            }))
            .expect("priority action should serialize"),
        )
        .expect("priority action should succeed");
    }

    fn dispatch_select_objects(wasm: &mut WasmGame, object_ids: &[u64]) {
        wasm.dispatch(
            serde_wasm_bindgen::to_value(&json!({
                "type": "select_objects",
                "object_ids": object_ids,
            }))
            .expect("select_objects should serialize"),
        )
        .expect("select_objects should succeed");
    }

    #[test]
    fn convert_and_validate_targets_rejects_wrong_requirement_order() {
        let first = Target::Object(ObjectId::from_raw(1));
        let second = Target::Object(ObjectId::from_raw(2));
        let ctx = TargetsContext::new(
            PlayerId::from_index(0),
            ObjectId::from_raw(99),
            "test spell",
            vec![
                TargetRequirementContext {
                    description: "first target".to_string(),
                    legal_targets: vec![first],
                    min_targets: 1,
                    max_targets: Some(1),
                },
                TargetRequirementContext {
                    description: "second target".to_string(),
                    legal_targets: vec![second],
                    min_targets: 1,
                    max_targets: Some(1),
                },
            ],
        );

        let err = convert_and_validate_targets(
            &ctx,
            vec![
                TargetInput::Object { object: 2 },
                TargetInput::Object { object: 1 },
            ],
        )
        .expect_err("reversed targets should be rejected");

        assert_eq!(
            err.as_string().as_deref(),
            Some("targets do not satisfy the targeting requirements in order")
        );
    }

    #[test]
    fn convert_and_validate_targets_accepts_unbounded_then_fixed_sequence() {
        let a = Target::Object(ObjectId::from_raw(1));
        let b = Target::Object(ObjectId::from_raw(2));
        let c = Target::Object(ObjectId::from_raw(3));
        let ctx = TargetsContext::new(
            PlayerId::from_index(0),
            ObjectId::from_raw(99),
            "test spell",
            vec![
                TargetRequirementContext {
                    description: "any number".to_string(),
                    legal_targets: vec![a, b],
                    min_targets: 0,
                    max_targets: None,
                },
                TargetRequirementContext {
                    description: "last target".to_string(),
                    legal_targets: vec![c],
                    min_targets: 1,
                    max_targets: Some(1),
                },
            ],
        );

        let converted = convert_and_validate_targets(
            &ctx,
            vec![
                TargetInput::Object { object: 1 },
                TargetInput::Object { object: 2 },
                TargetInput::Object { object: 3 },
            ],
        )
        .expect("valid unbounded assignment");

        assert_eq!(converted, vec![a, b, c]);
    }

    #[test]
    fn object_details_reports_calculated_battlefield_power_toughness() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);

        let bears_def = grizzly_bears();
        let bears_id = game.create_object_from_definition(&bears_def, alice, Zone::Battlefield);

        // Apply +3/+0 until end of turn to the bears.
        game.continuous_effects.add_effect(ContinuousEffect::pump(
            bears_id,
            alice,
            bears_id,
            3,
            0,
            Until::EndOfTurn,
        ));

        let details =
            build_object_details_snapshot(&game, bears_id).expect("expected object details");
        assert_eq!(details.power, Some(5));
        assert_eq!(details.toughness, Some(2));
    }

    #[test]
    fn resolving_spell_snapshot_uses_current_source_object_name_after_stack_exit() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let aura = CardBuilder::new(CardId::from_raw(70_001), "Tall as a Beanstalk")
            .card_types(vec![CardType::Enchantment])
            .build();

        let stack_id = game.create_object_from_card(&aura, alice, Zone::Stack);
        let stack_obj = game.object(stack_id).expect("spell should exist on stack");
        let entry = StackEntry::new(stack_id, alice)
            .with_source_info(stack_obj.stable_id, stack_obj.name.clone());

        let battlefield_id = game
            .move_object(stack_id, Zone::Battlefield)
            .expect("spell should resolve to battlefield");
        let snapshot = build_stack_object_snapshot(&game, alice, None, &entry);

        assert_eq!(snapshot.name, "Tall as a Beanstalk");
        assert_eq!(snapshot.inspect_object_id, Some(battlefield_id.0));
        assert_eq!(
            snapshot.source_stable_id,
            game.object(battlefield_id).map(|obj| obj.stable_id.0.0)
        );
    }

    #[test]
    fn delayed_trigger_snapshot_keeps_source_name_after_source_changes_zones() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let source = CardBuilder::new(CardId::from_raw(70_002), "Flickerwisp")
            .card_types(vec![CardType::Creature])
            .build();

        let source_id = game.create_object_from_card(&source, alice, Zone::Battlefield);
        let source_stable_id = game
            .object(source_id)
            .expect("source should exist")
            .stable_id;

        crate::effects::delayed::trigger_queue::queue_delayed_trigger(
            &mut game,
            crate::effects::delayed::trigger_queue::DelayedTriggerConfig::new(
                Trigger::beginning_of_end_step(crate::target::PlayerFilter::Specific(alice)),
                Vec::new(),
                true,
                Vec::new(),
                alice,
            )
            .with_ability_source(Some(source_id)),
        );

        let moved_source_id = game
            .move_object(source_id, Zone::Exile)
            .expect("source should move to exile");
        assert_ne!(moved_source_id, source_id);

        let event = TriggerEvent::new_with_provenance(
            crate::events::phase::BeginningOfEndStepEvent::new(alice),
            crate::provenance::ProvNodeId::default(),
        );
        let triggered = crate::triggers::check_delayed_triggers(&mut game, &event);
        assert_eq!(triggered.len(), 1, "delayed trigger should fire");

        let mut entry = StackEntry::ability(
            triggered[0].source,
            triggered[0].controller,
            triggered[0].ability.effects.clone(),
        )
        .with_source_info(
            triggered[0].source_stable_id,
            triggered[0].source_name.clone(),
        )
        .with_triggering_event(triggered[0].triggering_event.clone());
        if let Some(snapshot) = triggered[0].source_snapshot.clone() {
            entry = entry.with_source_snapshot(snapshot);
        }
        let snapshot = build_stack_object_snapshot(&game, alice, None, &entry);

        assert_eq!(snapshot.name, "Flickerwisp");
        assert_eq!(snapshot.source_stable_id, Some(source_stable_id.0.0));
        assert_eq!(snapshot.ability_kind.as_deref(), Some("Triggered"));
    }

    #[test]
    fn card_load_diagnostics_include_compilation_context_for_builtin_cards() {
        let mut wasm = WasmGame::new();
        let diagnostics =
            wasm.build_card_load_diagnostics("Urza's Saga", Some("synthetic failure"));

        assert_eq!(diagnostics.query, "Urza's Saga");
        assert_eq!(diagnostics.canonical_name.as_deref(), Some("Urza's Saga"));
        assert_eq!(diagnostics.error.as_deref(), Some("synthetic failure"));
        assert!(
            diagnostics
                .oracle_text
                .as_deref()
                .is_some_and(|oracle| oracle.contains("chapter")),
            "expected oracle text in diagnostics"
        );
        assert!(
            !diagnostics.compiled_text.is_empty(),
            "expected compiled text lines in diagnostics"
        );
        assert!(
            !diagnostics.compiled_abilities.is_empty(),
            "expected compiled abilities in diagnostics"
        );
    }

    #[cfg(feature = "generated-registry")]
    #[test]
    fn card_load_diagnostics_report_parse_error_for_unsupported_generated_cards() {
        let mut wasm = WasmGame::new();
        let diagnostics = wasm.build_card_load_diagnostics("Sicarian Infiltrator", None);

        assert_eq!(diagnostics.query, "Sicarian Infiltrator");
        assert!(
            diagnostics
                .parse_error
                .as_deref()
                .is_some_and(|error| error.to_ascii_lowercase().contains("unsupported")),
            "expected unsupported parse error in diagnostics, got {:?}",
            diagnostics.parse_error
        );
    }

    #[cfg(feature = "generated-registry")]
    #[test]
    fn load_decks_reports_threshold_and_parse_failures_separately() {
        #[derive(Debug, Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct DeckLoadResultView {
            loaded: u32,
            failed: Vec<String>,
            failed_below_threshold: Vec<String>,
            failed_to_parse: Vec<String>,
        }

        let mut wasm = WasmGame::new();
        let (below_threshold_name, below_threshold_score) =
            CardRegistry::generated_parser_card_names()
                .into_iter()
                .find_map(|name| {
                    let score = WasmGame::semantic_score_for_name(name.as_str())?;
                    if score >= 1.0 || CardRegistry::try_compile_card(name.as_str()).is_err() {
                        return None;
                    }
                    Some((name, score))
                })
                .expect("expected a compilable generated card below 100% fidelity");
        let threshold_percent = ((below_threshold_score * 100.0) + 0.5).clamp(1.0, 100.0);
        wasm.set_semantic_threshold(threshold_percent);

        let threshold = threshold_percent / 100.0;
        let loaded_name = CardRegistry::generated_parser_card_names()
            .into_iter()
            .find(|name| {
                WasmGame::semantic_score_for_name(name.as_str())
                    .is_some_and(|score| score >= threshold)
                    && CardRegistry::try_compile_card(name.as_str()).is_ok()
            })
            .expect("expected a compilable generated card that meets the chosen threshold");

        let decks_js = serde_wasm_bindgen::to_value(&vec![
            vec![
                loaded_name.clone(),
                below_threshold_name.clone(),
                "Sicarian Infiltrator".to_string(),
            ],
            Vec::<String>::new(),
        ])
        .expect("should encode test deck lists");
        let result = wasm
            .load_decks(decks_js)
            .expect("deck load should return categorized failures");
        let result: DeckLoadResultView =
            serde_wasm_bindgen::from_value(result).expect("should decode deck load result");

        assert_eq!(result.loaded, 1);
        assert_eq!(
            result.failed,
            vec![
                below_threshold_name.clone(),
                "Sicarian Infiltrator".to_string(),
            ]
        );
        assert_eq!(result.failed_below_threshold, vec![below_threshold_name]);
        assert_eq!(
            result.failed_to_parse,
            vec!["Sicarian Infiltrator".to_string()]
        );
    }

    #[cfg(feature = "generated-registry")]
    #[test]
    fn load_decks_accepts_alternative_card_names() {
        #[derive(Debug, Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct DeckLoadResultView {
            loaded: u32,
            failed: Vec<String>,
            failed_below_threshold: Vec<String>,
            failed_to_parse: Vec<String>,
        }

        let mut wasm = WasmGame::new();
        let decks_js = serde_wasm_bindgen::to_value(&vec![
            vec![
                "T-60 Power Armor".to_string(),
                "Sunset Sarsaparilla Machine".to_string(),
            ],
            Vec::<String>::new(),
        ])
        .expect("should encode deck lists");
        let result = wasm.load_decks(decks_js).expect("deck load should succeed");
        let result: DeckLoadResultView =
            serde_wasm_bindgen::from_value(result).expect("should decode deck load result");

        assert_eq!(result.loaded, 2);
        assert!(result.failed.is_empty());
        assert!(result.failed_below_threshold.is_empty());
        assert!(result.failed_to_parse.is_empty());

        let alice = wasm
            .game
            .player(PlayerId::from_index(0))
            .expect("alice should exist");
        let library_names: Vec<String> = alice
            .library
            .iter()
            .filter_map(|&id| wasm.game.object(id).map(|object| object.name.clone()))
            .collect();

        assert!(
            library_names.iter().any(|name| name == "T-45 Power Armor"),
            "expected canonical T-45 Power Armor in library, got {library_names:?}"
        );
        assert!(
            library_names
                .iter()
                .any(|name| name == "Nuka-Cola Vending Machine"),
            "expected canonical Nuka-Cola Vending Machine in library, got {library_names:?}"
        );
    }

    #[cfg(feature = "generated-registry")]
    #[test]
    fn add_card_to_hand_accepts_alternative_card_names() {
        let mut wasm = WasmGame::new();

        let flavor_id = wasm
            .add_card_to_hand(0, "T-60 Power Armor".to_string())
            .expect("should add flavor-name alias to hand");
        let printed_id = wasm
            .add_card_to_hand(0, "Sunset Sarsaparilla Machine".to_string())
            .expect("should add flavor-name alias to hand");

        let flavor_card = wasm
            .game
            .object(ObjectId::from_raw(flavor_id))
            .expect("flavor-name object should exist");
        let printed_card = wasm
            .game
            .object(ObjectId::from_raw(printed_id))
            .expect("printed-name object should exist");

        assert_eq!(flavor_card.name, "T-45 Power Armor");
        assert_eq!(printed_card.name, "Nuka-Cola Vending Machine");
    }

    #[test]
    fn add_card_to_zone_battlefield_applies_etb_replacement_effects() {
        let mut wasm = WasmGame::new();

        let _tayam_id = wasm
            .add_card_to_zone(
                0,
                "Tayam, Luminous Enigma".to_string(),
                "battlefield".to_string(),
                true,
            )
            .expect("should add Tayam to battlefield");
        wasm.game.refresh_continuous_state();

        let entered_id = wasm
            .add_card_to_zone(
                0,
                "Grizzly Bears".to_string(),
                "battlefield".to_string(),
                false,
            )
            .expect("should add Grizzly Bears to battlefield with ETB processing");

        let entered = wasm
            .game
            .object(ObjectId::from_raw(entered_id))
            .expect("entered permanent should exist");
        assert_eq!(
            entered.counters.get(&CounterType::Vigilance).copied(),
            Some(1),
            "addCardToZone battlefield path should apply Tayam ETB replacement counter"
        );
    }

    #[test]
    fn add_card_to_zone_battlefield_adds_initial_saga_lore_counter() {
        let mut wasm = WasmGame::new();

        let entered_id = wasm
            .add_card_to_zone(
                0,
                "Urza's Saga".to_string(),
                "battlefield".to_string(),
                false,
            )
            .expect("should add Urza's Saga to battlefield with ETB processing");

        let entered = wasm
            .game
            .object(ObjectId::from_raw(entered_id))
            .expect("entered saga should exist");
        assert_eq!(
            entered.counters.get(&CounterType::Lore).copied(),
            Some(1),
            "battlefield ETB path should give a Saga its initial lore counter"
        );
    }

    #[test]
    fn playing_urzas_saga_from_hand_adds_initial_lore_counter_and_surfaces_snapshot_counters() {
        let mut wasm = WasmGame::new();
        let alice = PlayerId::from_index(0);

        wasm.game.turn.active_player = alice;
        wasm.game.turn.priority_player = Some(alice);
        wasm.game.turn.phase = Phase::FirstMain;
        wasm.game.turn.step = None;

        let saga_id = wasm
            .game
            .create_object_from_definition(&urzas_saga(), alice, Zone::Hand);

        wasm.priority_epoch_checkpoint = Some(wasm.capture_replay_checkpoint());
        wasm.pending_decision = Some(DecisionContext::Priority(PriorityContext::new(
            alice,
            compute_legal_actions(&wasm.game, alice),
        )));

        let priority_ctx = match wasm.pending_decision.as_ref() {
            Some(DecisionContext::Priority(ctx)) => ctx,
            other => panic!("expected priority decision, got {other:?}"),
        };
        let play_saga_index = priority_ctx
            .actions
            .iter()
            .position(
                |action| matches!(action, LegalAction::PlayLand { land_id } if *land_id == saga_id),
            )
            .expect("expected play Urza's Saga action");

        wasm.dispatch(
            serde_wasm_bindgen::to_value(&json!({
                "type": "priority_action",
                "action_index": play_saga_index,
            }))
            .expect("priority action should serialize"),
        )
        .expect("playing Urza's Saga should succeed");

        let entered_id = wasm
            .game
            .battlefield
            .iter()
            .copied()
            .find(|&id| {
                wasm.game
                    .object(id)
                    .is_some_and(|object| object.name == "Urza's Saga")
            })
            .expect("Urza's Saga should be on battlefield");

        let entered = wasm
            .game
            .object(entered_id)
            .expect("played saga should still exist");
        assert_eq!(
            entered.counters.get(&CounterType::Lore).copied(),
            Some(1),
            "playing Urza's Saga as a land should give it its initial lore counter"
        );

        let snapshot = GameSnapshot::from_game(
            &wasm.game,
            alice,
            None,
            None,
            None,
            None,
            None,
            Vec::new(),
            None,
            false,
            None,
            0,
        );
        let me = snapshot
            .players
            .iter()
            .find(|player| player.id == alice.0)
            .expect("perspective player should exist");
        let saga = me
            .battlefield
            .iter()
            .find(|perm| perm.name == "Urza's Saga")
            .expect("snapshot should include Urza's Saga");

        assert_eq!(
            saga.counters.len(),
            1,
            "snapshot should surface Saga counters"
        );
        assert_eq!(saga.counters[0].kind, "Lore");
        assert_eq!(saga.counters[0].amount, 1);
    }

    #[test]
    fn cancelability_allows_locked_pending_mana_ability_while_decision_open() {
        let mut wasm = WasmGame::new();
        wasm.priority_epoch_checkpoint = Some(wasm.capture_replay_checkpoint());
        wasm.priority_epoch_has_undoable_action = true;
        wasm.priority_state.pending_mana_ability = Some(PendingManaAbility {
            source: ObjectId::from_raw(1),
            ability_index: 0,
            activator: PlayerId::from_index(0),
            provenance: crate::provenance::ProvNodeId::default(),
            mana_cost: ManaCost::new(),
            other_costs: Vec::new(),
            mana_to_add: vec![ManaSymbol::Green],
            effects: Vec::new(),
            undo_locked_by_mana: true,
        });
        wasm.pending_decision = Some(DecisionContext::Boolean(BooleanContext::new(
            PlayerId::from_index(0),
            None,
            "choose a color",
        )));

        assert!(
            wasm.is_cancelable(),
            "cancel should stay enabled while a decision prompt is open"
        );
    }

    #[test]
    fn cancelability_allows_mana_undo_when_not_locked() {
        let mut wasm = WasmGame::new();
        wasm.priority_epoch_checkpoint = Some(wasm.capture_replay_checkpoint());
        wasm.priority_epoch_has_undoable_action = true;
        wasm.priority_state.pending_mana_ability = Some(PendingManaAbility {
            source: ObjectId::from_raw(1),
            ability_index: 0,
            activator: PlayerId::from_index(0),
            provenance: crate::provenance::ProvNodeId::default(),
            mana_cost: ManaCost::new(),
            other_costs: Vec::new(),
            mana_to_add: vec![ManaSymbol::Green],
            effects: Vec::new(),
            undo_locked_by_mana: false,
        });

        assert!(
            wasm.is_cancelable(),
            "cancel should stay enabled for undo-safe mana activation chains"
        );
    }

    #[test]
    fn cancelability_allows_epoch_undo_without_pending_chain() {
        let mut wasm = WasmGame::new();
        wasm.priority_epoch_checkpoint = Some(wasm.capture_replay_checkpoint());
        wasm.priority_epoch_has_undoable_action = true;

        assert!(
            wasm.is_cancelable(),
            "cancel should stay available during a reversible priority epoch"
        );
    }

    #[test]
    fn cancelability_blocks_epoch_undo_without_user_action() {
        let mut wasm = WasmGame::new();
        wasm.priority_epoch_checkpoint = Some(wasm.capture_replay_checkpoint());
        wasm.priority_epoch_has_undoable_action = false;

        assert!(
            !wasm.is_cancelable(),
            "cancel should be disabled when no undoable action happened in this epoch"
        );
    }

    #[test]
    fn cancelability_allows_irreversible_mana_replay_while_decision_open() {
        let mut wasm = WasmGame::new();
        wasm.priority_epoch_checkpoint = Some(wasm.capture_replay_checkpoint());
        wasm.pending_decision = Some(DecisionContext::Boolean(BooleanContext::new(
            PlayerId::from_index(0),
            None,
            "choose a color",
        )));
        let checkpoint = wasm.capture_replay_checkpoint();
        wasm.pending_replay_action = Some(PendingReplayAction {
            checkpoint,
            root: ReplayRoot::Response(PriorityResponse::PriorityAction(
                LegalAction::ActivateManaAbility {
                    source: ObjectId::from_raw(999),
                    ability_index: 0,
                },
            )),
            nested_answers: Vec::new(),
        });

        assert!(
            wasm.is_cancelable(),
            "cancel should stay enabled while replay is waiting on a decision"
        );
    }

    #[test]
    fn cancelability_blocks_irreversible_mana_replay_without_open_decision() {
        let mut wasm = WasmGame::new();
        wasm.priority_epoch_checkpoint = Some(wasm.capture_replay_checkpoint());
        let checkpoint = wasm.capture_replay_checkpoint();
        wasm.pending_replay_action = Some(PendingReplayAction {
            checkpoint,
            root: ReplayRoot::Response(PriorityResponse::PriorityAction(
                LegalAction::ActivateManaAbility {
                    source: ObjectId::from_raw(999),
                    ability_index: 0,
                },
            )),
            nested_answers: Vec::new(),
        });

        assert!(
            !wasm.is_cancelable(),
            "cancel should be disabled once irreversible mana replay is committed"
        );
    }

    #[test]
    fn cancelability_blocks_when_land_played_in_epoch() {
        let mut wasm = WasmGame::new();
        let player = PlayerId::from_index(0);
        wasm.priority_epoch_checkpoint = Some(wasm.capture_replay_checkpoint());
        wasm.game
            .player_mut(player)
            .expect("player should exist")
            .record_land_play();

        assert!(
            !wasm.is_cancelable(),
            "cancel should be disabled after a land play in the current epoch"
        );
    }

    #[test]
    fn cancelability_blocks_land_play_replay_even_with_open_decision() {
        let mut wasm = WasmGame::new();
        let player = PlayerId::from_index(0);
        wasm.priority_epoch_checkpoint = Some(wasm.capture_replay_checkpoint());
        wasm.pending_decision = Some(DecisionContext::Boolean(BooleanContext::new(
            player,
            None,
            "resolve trigger",
        )));
        let checkpoint = wasm.capture_replay_checkpoint();
        wasm.pending_replay_action = Some(PendingReplayAction {
            checkpoint,
            root: ReplayRoot::Response(PriorityResponse::PriorityAction(LegalAction::PlayLand {
                land_id: ObjectId::from_raw(777),
            })),
            nested_answers: Vec::new(),
        });
        wasm.game
            .player_mut(player)
            .expect("player should exist")
            .record_land_play();

        assert!(
            !wasm.is_cancelable(),
            "cancel should stay disabled once a replay chain includes a land play"
        );
    }

    #[test]
    fn cancelability_blocks_when_epoch_is_mana_locked() {
        let mut wasm = WasmGame::new();
        wasm.priority_epoch_checkpoint = Some(wasm.capture_replay_checkpoint());
        wasm.priority_epoch_undo_locked_by_mana = true;

        assert!(
            !wasm.is_cancelable(),
            "cancel should be disabled once epoch is locked by irreversible mana activation"
        );
    }

    #[test]
    fn dispatch_disables_cancel_when_mana_tap_trigger_adds_stack_object() {
        let mut wasm = WasmGame::new();
        let alice = PlayerId::from_index(0);

        wasm.game.turn.active_player = alice;
        wasm.game.turn.priority_player = Some(alice);
        wasm.game.turn.phase = Phase::FirstMain;
        wasm.game.turn.step = None;

        let swamp_card = CardBuilder::new(CardId::new(), "Undo Probe Swamp")
            .card_types(vec![CardType::Land])
            .build();
        let swamp_id = wasm
            .game
            .create_object_from_card(&swamp_card, alice, Zone::Battlefield);
        if let Some(swamp) = wasm.game.object_mut(swamp_id) {
            swamp.abilities.push(Ability::mana(
                crate::cost::TotalCost::free(),
                vec![ManaSymbol::Black],
            ));
        }

        let trigger_source = CardBuilder::new(CardId::new(), "Undo Probe Trigger")
            .card_types(vec![CardType::Enchantment])
            .build();
        let trigger_source_id =
            wasm.game
                .create_object_from_card(&trigger_source, alice, Zone::Battlefield);
        if let Some(source) = wasm.game.object_mut(trigger_source_id) {
            source.abilities.push(Ability::triggered(
                Trigger::player_taps_for_mana(
                    crate::target::PlayerFilter::Any,
                    crate::filter::ObjectFilter::land(),
                ),
                vec![Effect::lose_life_player(
                    1,
                    crate::target::PlayerFilter::Specific(alice),
                )],
            ));
        }

        wasm.priority_epoch_checkpoint = Some(wasm.capture_replay_checkpoint());
        wasm.pending_decision = Some(DecisionContext::Priority(PriorityContext::new(
            alice,
            compute_legal_actions(&wasm.game, alice),
        )));

        let priority_ctx = match wasm.pending_decision.as_ref() {
            Some(DecisionContext::Priority(ctx)) => ctx,
            other => panic!("expected priority decision, got {other:?}"),
        };
        let action_index = priority_ctx
            .actions
            .iter()
            .position(|action| {
                matches!(
                    action,
                    LegalAction::ActivateManaAbility { source, .. } if *source == swamp_id
                )
            })
            .expect("expected tap-for-mana action");

        wasm.dispatch(
            serde_wasm_bindgen::to_value(&json!({
                "type": "priority_action",
                "action_index": action_index,
            }))
            .expect("priority action should serialize"),
        )
        .expect("tapping swamp for mana should succeed");

        assert_eq!(
            wasm.game.stack.len(),
            1,
            "non-mana tap-for-mana trigger should add an object to the stack"
        );
        assert!(
            !wasm.is_cancelable(),
            "undo should be disabled once tapping for mana creates a stack object"
        );
    }

    #[test]
    fn snapshot_surfaces_undo_land_stable_id_for_reversible_land_tap() {
        let mut wasm = WasmGame::new();
        let alice = PlayerId::from_index(0);

        wasm.game.turn.active_player = alice;
        wasm.game.turn.priority_player = Some(alice);
        wasm.game.turn.phase = Phase::FirstMain;
        wasm.game.turn.step = None;

        let swamp_card = CardBuilder::new(CardId::new(), "Undo Probe Swamp")
            .card_types(vec![CardType::Land])
            .build();
        let swamp_id = wasm
            .game
            .create_object_from_card(&swamp_card, alice, Zone::Battlefield);
        if let Some(swamp) = wasm.game.object_mut(swamp_id) {
            swamp.abilities.push(Ability::mana(
                crate::cost::TotalCost::free(),
                vec![ManaSymbol::Black],
            ));
        }
        let swamp_stable_id = wasm
            .game
            .object(swamp_id)
            .expect("swamp should exist")
            .stable_id
            .0
            .0;

        wasm.priority_epoch_checkpoint = Some(wasm.capture_replay_checkpoint());
        wasm.pending_decision = Some(DecisionContext::Priority(PriorityContext::new(
            alice,
            compute_legal_actions(&wasm.game, alice),
        )));

        let priority_ctx = match wasm.pending_decision.as_ref() {
            Some(DecisionContext::Priority(ctx)) => ctx,
            other => panic!("expected priority decision, got {other:?}"),
        };
        let action_index = priority_ctx
            .actions
            .iter()
            .position(|action| {
                matches!(
                    action,
                    LegalAction::ActivateManaAbility { source, .. } if *source == swamp_id
                )
            })
            .expect("expected tap-for-mana action");

        wasm.dispatch(
            serde_wasm_bindgen::to_value(&json!({
                "type": "priority_action",
                "action_index": action_index,
            }))
            .expect("priority action should serialize"),
        )
        .expect("tapping swamp for mana should succeed");

        assert!(
            wasm.is_cancelable(),
            "plain land tap should remain undoable"
        );
        assert_eq!(
            wasm.priority_epoch_undo_land_stable_id,
            Some(swamp_stable_id),
            "the current undoable land tap should be tracked by stable id"
        );

        let pending_cast_stack_id = wasm
            .priority_state
            .pending_cast
            .as_ref()
            .map(|p| p.stack_id);
        let cancelable = wasm.is_cancelable();
        let snapshot = GameSnapshot::from_game(
            &wasm.game,
            wasm.perspective,
            wasm.pending_decision.as_ref(),
            None,
            wasm.game_over.as_ref(),
            pending_cast_stack_id,
            wasm.active_resolving_stack_object.clone(),
            Vec::new(),
            None,
            cancelable,
            wasm.visible_undo_land_stable_id(cancelable),
            0,
        );
        assert_eq!(
            snapshot.undo_land_stable_id,
            Some(swamp_stable_id),
            "snapshot should expose the reversible tapped land for the UI"
        );
        let me = snapshot
            .players
            .iter()
            .find(|player| player.id == alice.0)
            .expect("perspective player should exist");
        let swamp = me
            .battlefield
            .iter()
            .find(|perm| perm.stable_id == swamp_stable_id)
            .expect("snapshot should still include the tapped swamp");
        assert!(
            swamp.tapped,
            "tracked undo land should be tapped in the snapshot"
        );
    }

    #[test]
    fn cleanup_auto_discard_only_applies_for_non_perspective_player() {
        let mut wasm = WasmGame::new();
        wasm.game.turn.step = Some(Step::Cleanup);

        let perspective_ctx = DecisionContext::SelectObjects(SelectObjectsContext::new(
            wasm.perspective,
            None,
            "Discard cards",
            vec![
                SelectableObject::new(ObjectId::from_raw(1), "Card A"),
                SelectableObject::new(ObjectId::from_raw(2), "Card B"),
            ],
            1,
            Some(1),
        ));
        assert!(
            !wasm.should_auto_resolve_cleanup_discard(&perspective_ctx),
            "cleanup discard should not auto-resolve for the perspective player"
        );

        let opponent =
            PlayerId::from_index((wasm.perspective.0 + 1) % wasm.game.players.len() as u8);
        let opponent_ctx = DecisionContext::SelectObjects(SelectObjectsContext::new(
            opponent,
            None,
            "Discard cards",
            vec![
                SelectableObject::new(ObjectId::from_raw(3), "Card C"),
                SelectableObject::new(ObjectId::from_raw(4), "Card D"),
            ],
            1,
            Some(1),
        ));
        assert!(
            wasm.should_auto_resolve_cleanup_discard(&opponent_ctx),
            "cleanup discard should auto-resolve for non-perspective players"
        );
    }

    #[test]
    fn cleanup_auto_discard_respects_toggle_and_cleanup_step() {
        let mut wasm = WasmGame::new();
        let opponent =
            PlayerId::from_index((wasm.perspective.0 + 1) % wasm.game.players.len() as u8);
        let opponent_ctx = DecisionContext::SelectObjects(SelectObjectsContext::new(
            opponent,
            None,
            "Discard cards",
            vec![SelectableObject::new(ObjectId::from_raw(5), "Card E")],
            1,
            Some(1),
        ));

        wasm.game.turn.step = Some(Step::Cleanup);
        wasm.auto_cleanup_discard = false;
        assert!(
            !wasm.should_auto_resolve_cleanup_discard(&opponent_ctx),
            "toggle should disable cleanup auto-discard"
        );

        wasm.auto_cleanup_discard = true;
        wasm.game.turn.step = Some(Step::End);
        assert!(
            !wasm.should_auto_resolve_cleanup_discard(&opponent_ctx),
            "auto-discard should only happen during cleanup step"
        );
    }

    #[test]
    fn snapshot_perspective_hand_cards_are_not_truncated() {
        let mut wasm = WasmGame::new();
        for _ in 0..20 {
            wasm.add_card_to_zone(0, "Ornithopter".to_string(), "hand".to_string(), true)
                .expect("adding card to hand should succeed");
        }

        let pending_cast_stack_id = wasm
            .priority_state
            .pending_cast
            .as_ref()
            .map(|p| p.stack_id);
        let snapshot = GameSnapshot::from_game(
            &wasm.game,
            wasm.perspective,
            wasm.pending_decision.as_ref(),
            None,
            wasm.game_over.as_ref(),
            pending_cast_stack_id,
            wasm.active_resolving_stack_object.clone(),
            Vec::new(),
            None,
            wasm.is_cancelable(),
            None,
            0,
        );
        let me = snapshot
            .players
            .iter()
            .find(|p| p.id == wasm.perspective.0)
            .expect("perspective player should exist in snapshot");

        assert_eq!(
            me.hand_cards.len(),
            me.hand_size,
            "perspective hand_cards must stay in sync with hand_size"
        );
        assert!(
            me.hand_cards.len() >= 20,
            "expected all 20 hand cards to be present in snapshot"
        );
    }

    #[test]
    fn snapshot_redacts_hidden_opponent_select_object_candidates() {
        let mut wasm = WasmGame::new();
        let bob = PlayerId::from_index(1);
        let card_a = wasm
            .add_card_to_zone(1, "Primeval Titan".to_string(), "hand".to_string(), true)
            .expect("adding first hidden card should succeed");
        let card_b = wasm
            .add_card_to_zone(1, "Forest".to_string(), "hand".to_string(), true)
            .expect("adding second hidden card should succeed");

        let decision = DecisionContext::SelectObjects(SelectObjectsContext::new(
            bob,
            None,
            "Choose cards to discard",
            vec![
                SelectableObject::new(ObjectId::from_raw(card_a), "Primeval Titan"),
                SelectableObject::new(ObjectId::from_raw(card_b), "Forest"),
            ],
            1,
            Some(1),
        ));

        let pending_cast_stack_id = wasm
            .priority_state
            .pending_cast
            .as_ref()
            .map(|p| p.stack_id);
        let snapshot = GameSnapshot::from_game(
            &wasm.game,
            wasm.perspective,
            Some(&decision),
            None,
            wasm.game_over.as_ref(),
            pending_cast_stack_id,
            wasm.active_resolving_stack_object.clone(),
            Vec::new(),
            None,
            wasm.is_cancelable(),
            None,
            0,
        );

        let redacted_candidates = match snapshot
            .decision
            .as_ref()
            .expect("snapshot should include the pending select-objects decision")
        {
            super::DecisionView::SelectObjects { candidates, .. } => candidates,
            other => panic!("expected select_objects view, got {other:?}"),
        };

        assert_eq!(redacted_candidates.len(), 2);
        assert!(
            redacted_candidates
                .iter()
                .all(|candidate| candidate.name == "Hidden card"),
            "opponent hand choices should be redacted for other perspectives"
        );
        assert!(
            redacted_candidates
                .iter()
                .all(|candidate| candidate.id != card_a && candidate.id != card_b),
            "redacted candidates should not expose the real hidden object ids"
        );
    }

    #[test]
    fn snapshot_redacts_hidden_opponent_priority_hand_actions() {
        let mut wasm = WasmGame::new();
        let bob = PlayerId::from_index(1);
        let spell_id = wasm
            .add_card_to_zone(1, "Lightning Bolt".to_string(), "hand".to_string(), true)
            .expect("adding hidden spell should succeed");
        let priority = DecisionContext::Priority(PriorityContext::new(
            bob,
            vec![
                LegalAction::PassPriority,
                LegalAction::CastSpell {
                    spell_id: ObjectId::from_raw(spell_id),
                    from_zone: Zone::Hand,
                    casting_method: crate::alternative_cast::CastingMethod::Normal,
                },
            ],
        ));

        let pending_cast_stack_id = wasm
            .priority_state
            .pending_cast
            .as_ref()
            .map(|p| p.stack_id);
        let snapshot = GameSnapshot::from_game(
            &wasm.game,
            wasm.perspective,
            Some(&priority),
            None,
            wasm.game_over.as_ref(),
            pending_cast_stack_id,
            wasm.active_resolving_stack_object.clone(),
            Vec::new(),
            None,
            wasm.is_cancelable(),
            None,
            0,
        );

        let actions = match snapshot
            .decision
            .as_ref()
            .expect("snapshot should include the priority decision")
        {
            super::DecisionView::Priority { actions, .. } => actions,
            other => panic!("expected priority view, got {other:?}"),
        };
        let hidden_cast = actions
            .iter()
            .find(|action| action.kind == "cast_spell")
            .expect("snapshot should include the redacted cast action");

        assert_eq!(hidden_cast.label, "Cast hidden spell");
        assert_eq!(hidden_cast.object_id, None);
        assert_eq!(hidden_cast.from_zone, None);
        assert_eq!(hidden_cast.to_zone, None);
    }

    #[test]
    fn snapshot_grouped_battlefield_count_matches_total() {
        let mut wasm = WasmGame::new();
        for _ in 0..3 {
            wasm.add_card_to_zone(
                0,
                "Black Lotus".to_string(),
                "battlefield".to_string(),
                true,
            )
            .expect("adding lotus to battlefield should succeed");
        }
        wasm.add_card_to_zone(0, "Mountain".to_string(), "battlefield".to_string(), true)
            .expect("adding mountain to battlefield should succeed");

        let pending_cast_stack_id = wasm
            .priority_state
            .pending_cast
            .as_ref()
            .map(|p| p.stack_id);
        let snapshot = GameSnapshot::from_game(
            &wasm.game,
            wasm.perspective,
            wasm.pending_decision.as_ref(),
            None,
            wasm.game_over.as_ref(),
            pending_cast_stack_id,
            wasm.active_resolving_stack_object.clone(),
            Vec::new(),
            None,
            wasm.is_cancelable(),
            None,
            0,
        );
        let me = snapshot
            .players
            .iter()
            .find(|p| p.id == wasm.perspective.0)
            .expect("perspective player should exist in snapshot");

        let grouped_total: usize = me.battlefield.iter().map(|perm| perm.count).sum();
        assert_eq!(
            grouped_total, me.battlefield_total,
            "battlefield_total must equal sum of grouped permanent counts"
        );
    }

    #[test]
    fn snapshot_grouped_battlefield_includes_mana_cost() {
        let mut wasm = WasmGame::new();
        wasm.add_card_to_zone(
            0,
            "Ornithopter".to_string(),
            "battlefield".to_string(),
            true,
        )
        .expect("adding ornithopter to battlefield should succeed");

        let pending_cast_stack_id = wasm
            .priority_state
            .pending_cast
            .as_ref()
            .map(|p| p.stack_id);
        let snapshot = GameSnapshot::from_game(
            &wasm.game,
            wasm.perspective,
            wasm.pending_decision.as_ref(),
            None,
            wasm.game_over.as_ref(),
            pending_cast_stack_id,
            wasm.active_resolving_stack_object.clone(),
            Vec::new(),
            None,
            wasm.is_cancelable(),
            None,
            0,
        );
        let me = snapshot
            .players
            .iter()
            .find(|p| p.id == wasm.perspective.0)
            .expect("perspective player should exist in snapshot");
        let ornithopter = me
            .battlefield
            .iter()
            .find(|perm| perm.name == "Ornithopter")
            .expect("expected ornithopter on battlefield");

        assert_eq!(ornithopter.mana_cost.as_deref(), Some("{0}"));
    }

    #[test]
    fn canceling_spell_chain_after_land_play_keeps_land_on_battlefield() {
        let mut wasm = WasmGame::new();
        let alice = PlayerId::from_index(0);

        wasm.game.turn.active_player = alice;
        wasm.game.turn.priority_player = Some(alice);
        wasm.game.turn.phase = Phase::FirstMain;
        wasm.game.turn.step = None;

        let mountain_id =
            wasm.game
                .create_object_from_definition(&basic_mountain(), alice, Zone::Hand);
        let bolt_id = wasm
            .game
            .create_object_from_definition(&lightning_bolt(), alice, Zone::Hand);

        wasm.priority_epoch_checkpoint = Some(wasm.capture_replay_checkpoint());
        wasm.pending_decision = Some(DecisionContext::Priority(PriorityContext::new(
            alice,
            compute_legal_actions(&wasm.game, alice),
        )));

        let priority_ctx = match wasm.pending_decision.as_ref() {
            Some(DecisionContext::Priority(ctx)) => ctx,
            other => panic!("expected priority decision, got {other:?}"),
        };
        let play_mountain_index = priority_ctx
            .actions
            .iter()
            .position(|action| matches!(action, LegalAction::PlayLand { land_id } if *land_id == mountain_id))
            .expect("expected play mountain action");

        wasm.dispatch(
            serde_wasm_bindgen::to_value(&json!({
                "type": "priority_action",
                "action_index": play_mountain_index,
            }))
            .expect("priority action command should serialize"),
        )
        .expect("playing mountain should succeed");

        let priority_ctx = match wasm.pending_decision.as_ref() {
            Some(DecisionContext::Priority(ctx)) => ctx,
            other => panic!("expected priority decision after land play, got {other:?}"),
        };
        let cast_bolt_index = priority_ctx
            .actions
            .iter()
            .position(|action| {
                matches!(
                    action,
                    LegalAction::CastSpell { spell_id, .. } if *spell_id == bolt_id
                )
            })
            .expect("expected cast lightning bolt action");

        wasm.dispatch(
            serde_wasm_bindgen::to_value(&json!({
                "type": "priority_action",
                "action_index": cast_bolt_index,
            }))
            .expect("cast spell command should serialize"),
        )
        .expect("casting lightning bolt should enter its decision chain");

        assert!(
            matches!(wasm.pending_decision, Some(DecisionContext::Targets(_))),
            "lightning bolt cast should be waiting on targets"
        );

        wasm.cancel_decision()
            .expect("canceling the in-progress spell should succeed");

        let alice_player = wasm.game.player(alice).expect("alice should exist");
        let mountains_on_battlefield = wasm
            .game
            .battlefield
            .iter()
            .filter(|&&id| {
                wasm.game
                    .object(id)
                    .is_some_and(|object| object.name == "Mountain")
            })
            .count();
        let bolts_in_hand = alice_player
            .hand
            .iter()
            .filter(|&&id| {
                wasm.game
                    .object(id)
                    .is_some_and(|object| object.name == "Lightning Bolt")
            })
            .count();

        assert_eq!(
            mountains_on_battlefield, 1,
            "canceling the spell should keep the played land on the battlefield"
        );
        assert_eq!(
            bolts_in_hand, 1,
            "canceling the spell should return the spell to hand"
        );
        assert!(
            wasm.game.stack.is_empty(),
            "canceling the spell should remove it from the stack"
        );
    }

    #[test]
    fn yawgmoth_activation_stays_cancelable_through_target_and_cost_prompts() {
        let mut wasm = WasmGame::new();
        let alice = PlayerId::from_index(0);

        wasm.game.turn.active_player = alice;
        wasm.game.turn.priority_player = Some(alice);
        wasm.game.turn.phase = Phase::FirstMain;
        wasm.game.turn.step = None;

        let yawgmoth_id = wasm.game.create_object_from_definition(
            &yawgmoth_thran_physician(),
            alice,
            Zone::Battlefield,
        );
        let target_id =
            wasm.game
                .create_object_from_definition(&grizzly_bears(), alice, Zone::Battlefield);
        wasm.game
            .create_object_from_definition(&ornithopter(), alice, Zone::Battlefield);

        wasm.priority_epoch_checkpoint = Some(wasm.capture_replay_checkpoint());
        wasm.pending_decision = Some(DecisionContext::Priority(PriorityContext::new(
            alice,
            compute_legal_actions(&wasm.game, alice),
        )));

        let priority_ctx = match wasm.pending_decision.as_ref() {
            Some(DecisionContext::Priority(ctx)) => ctx,
            other => panic!("expected priority decision, got {other:?}"),
        };
        let activate_index = priority_ctx
            .actions
            .iter()
            .position(|action| {
                matches!(
                    action,
                    LegalAction::ActivateAbility { source, .. } if *source == yawgmoth_id
                )
            })
            .expect("expected Yawgmoth activation action");

        wasm.dispatch(
            serde_wasm_bindgen::to_value(&json!({
                "type": "priority_action",
                "action_index": activate_index,
            }))
            .expect("priority action command should serialize"),
        )
        .expect("activating Yawgmoth should enter target selection");

        let targets_ctx = match wasm.pending_decision.as_ref() {
            Some(DecisionContext::Targets(ctx)) => ctx,
            other => panic!("expected target prompt after Yawgmoth activation, got {other:?}"),
        };
        assert_eq!(
            targets_ctx.player, alice,
            "Yawgmoth target prompt should belong to the activating player"
        );
        assert!(
            wasm.pending_replay_action.is_some(),
            "Yawgmoth activation should keep replay state open while choosing targets"
        );
        assert!(
            wasm.is_cancelable(),
            "Yawgmoth activation should remain cancelable during target selection"
        );

        let pending_cast_stack_id = wasm
            .priority_state
            .pending_cast
            .as_ref()
            .map(|p| p.stack_id);
        let cancelable = wasm.is_cancelable();
        let snapshot = GameSnapshot::from_game(
            &wasm.game,
            wasm.perspective,
            wasm.pending_decision.as_ref(),
            None,
            wasm.game_over.as_ref(),
            pending_cast_stack_id,
            wasm.active_resolving_stack_object.clone(),
            Vec::new(),
            None,
            cancelable,
            wasm.visible_undo_land_stable_id(cancelable),
            0,
        );
        assert!(
            snapshot.cancelable,
            "snapshot should expose Yawgmoth target prompt as cancelable"
        );
        assert!(
            snapshot.resolving_stack_object.is_none(),
            "activation-time target prompts should not pin a resolving stack entry"
        );
        let decision = snapshot
            .decision
            .expect("snapshot should still include the target decision");
        let player = match decision {
            super::DecisionView::Targets { player, .. } => player,
            other => panic!("expected target decision snapshot, got {other:?}"),
        };
        assert_eq!(
            player, alice.0,
            "snapshot target decision should belong to the perspective player"
        );

        wasm.dispatch(
            serde_wasm_bindgen::to_value(&json!({
                "type": "select_targets",
                "targets": [
                    { "kind": "object", "object": target_id.0 }
                ],
            }))
            .expect("target selection command should serialize"),
        )
        .expect("choosing Yawgmoth's target should continue activation");

        let next_cost_ctx = match wasm.pending_decision.as_ref() {
            Some(DecisionContext::SelectOptions(ctx)) => ctx,
            other => panic!("expected next-cost prompt after Yawgmoth target, got {other:?}"),
        };
        assert_eq!(
            next_cost_ctx.player, alice,
            "Yawgmoth next-cost prompt should belong to the activating player"
        );
        assert!(
            wasm.pending_replay_action.is_some(),
            "Yawgmoth activation should keep replay state open while choosing costs"
        );
        assert!(
            wasm.is_cancelable(),
            "Yawgmoth activation should remain cancelable after choosing targets"
        );

        let pending_cast_stack_id = wasm
            .priority_state
            .pending_cast
            .as_ref()
            .map(|p| p.stack_id);
        let cancelable = wasm.is_cancelable();
        let snapshot = GameSnapshot::from_game(
            &wasm.game,
            wasm.perspective,
            wasm.pending_decision.as_ref(),
            None,
            wasm.game_over.as_ref(),
            pending_cast_stack_id,
            wasm.active_resolving_stack_object.clone(),
            Vec::new(),
            None,
            cancelable,
            wasm.visible_undo_land_stable_id(cancelable),
            0,
        );
        assert!(
            snapshot.cancelable,
            "snapshot should expose Yawgmoth next-cost prompt as cancelable"
        );
        assert!(
            snapshot.resolving_stack_object.is_none(),
            "cost-payment prompts should not pin a resolving stack entry before the ability is committed"
        );
        let decision = snapshot
            .decision
            .expect("snapshot should still include the next-cost decision");
        match decision {
            super::DecisionView::SelectOptions { player, reason, .. } => {
                assert_eq!(player, alice.0);
                assert_eq!(reason.as_deref(), Some("Next cost"));
            }
            other => panic!("expected next-cost decision snapshot, got {other:?}"),
        }
    }

    #[test]
    fn yawgmoth_proliferate_next_cost_choices_advance_in_replay_chain() {
        fn setup_proliferate_prompt() -> WasmGame {
            let mut wasm = WasmGame::new();
            let alice = PlayerId::from_index(0);

            wasm.game.turn.active_player = alice;
            wasm.game.turn.priority_player = Some(alice);
            wasm.game.turn.phase = Phase::FirstMain;
            wasm.game.turn.step = None;

            let yawgmoth_id = wasm.game.create_object_from_definition(
                &yawgmoth_thran_physician(),
                alice,
                Zone::Battlefield,
            );
            wasm.add_card_to_zone(
                alice.0,
                "Black Lotus".to_string(),
                "battlefield".to_string(),
                true,
            )
            .expect("should add Black Lotus to battlefield");
            wasm.game
                .create_object_from_definition(&grizzly_bears(), alice, Zone::Hand);
            wasm.game
                .create_object_from_definition(&ornithopter(), alice, Zone::Hand);

            let proliferate_ability_index = wasm
                .game
                .object(yawgmoth_id)
                .and_then(|object| {
                    object.abilities.iter().position(|ability| {
                        matches!(
                            &ability.kind,
                            crate::ability::AbilityKind::Activated(activated)
                                if activated.mana_cost.mana_cost().is_some()
                                    && activated
                                        .mana_cost
                                        .costs()
                                        .iter()
                                        .any(|cost| cost.is_discard())
                        )
                    })
                })
                .expect("Yawgmoth should have proliferate ability");

            wasm.priority_epoch_checkpoint = Some(wasm.capture_replay_checkpoint());
            wasm.pending_decision = Some(DecisionContext::Priority(PriorityContext::new(
                alice,
                compute_legal_actions(&wasm.game, alice),
            )));

            let priority_ctx = match wasm.pending_decision.as_ref() {
                Some(DecisionContext::Priority(ctx)) => ctx,
                other => panic!("expected priority decision, got {other:?}"),
            };
            let activate_index = priority_ctx
                .actions
                .iter()
                .position(|action| {
                    matches!(
                        action,
                        LegalAction::ActivateAbility { source, ability_index }
                            if *source == yawgmoth_id && *ability_index == proliferate_ability_index
                    )
                })
                .expect("expected Yawgmoth proliferate activation action");

            wasm.dispatch(
                serde_wasm_bindgen::to_value(&json!({
                    "type": "priority_action",
                    "action_index": activate_index,
                }))
                .expect("priority action command should serialize"),
            )
            .expect("activating Yawgmoth proliferate should open next-cost chooser");

            assert!(
                matches!(
                    wasm.pending_decision,
                    Some(DecisionContext::SelectOptions(_))
                ),
                "Yawgmoth proliferate should begin on a next-cost chooser"
            );

            wasm
        }

        let mut mana_wasm = setup_proliferate_prompt();
        mana_wasm
            .dispatch(
                serde_wasm_bindgen::to_value(&json!({
                    "type": "select_options",
                    "option_indices": [0],
                }))
                .expect("next-cost mana choice should serialize"),
            )
            .expect("choosing Yawgmoth's mana cost should advance to mana payment");

        let mana_ctx = match mana_wasm.pending_decision.as_ref() {
            Some(DecisionContext::SelectOptions(ctx)) => ctx,
            other => panic!("expected mana payment prompt after choosing mana, got {other:?}"),
        };
        assert!(
            mana_ctx.description.to_lowercase().contains("pay mana pip"),
            "mana choice should advance to mana pip payment, got description: {}",
            mana_ctx.description
        );
        assert!(
            mana_ctx
                .options
                .iter()
                .any(|option| option.legal && option.description.contains("Black Lotus")),
            "mana payment prompt should offer Black Lotus"
        );

        let mut discard_wasm = setup_proliferate_prompt();
        discard_wasm
            .dispatch(
                serde_wasm_bindgen::to_value(&json!({
                    "type": "select_options",
                    "option_indices": [1],
                }))
                .expect("next-cost discard choice should serialize"),
            )
            .expect("choosing Yawgmoth's discard cost should advance to discard selection");

        let discard_ctx = match discard_wasm.pending_decision.as_ref() {
            Some(DecisionContext::SelectObjects(ctx)) => ctx,
            other => {
                panic!("expected discard selection prompt after choosing discard, got {other:?}")
            }
        };
        assert!(
            discard_ctx.description.to_lowercase().contains("discard"),
            "discard choice should advance to discard selection, got description: {}",
            discard_ctx.description
        );
        assert_eq!(discard_ctx.min, 1);
        assert_eq!(discard_ctx.max, Some(1));
    }

    #[test]
    fn stack_snapshot_includes_controller_and_targets() {
        let mut wasm = WasmGame::new();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        wasm.game.turn.active_player = alice;
        wasm.game.turn.priority_player = Some(alice);
        wasm.game.turn.phase = Phase::FirstMain;
        wasm.game.turn.step = None;

        let mountain_id =
            wasm.game
                .create_object_from_definition(&basic_mountain(), alice, Zone::Hand);
        let bolt_id = wasm
            .game
            .create_object_from_definition(&lightning_bolt(), alice, Zone::Hand);

        wasm.priority_epoch_checkpoint = Some(wasm.capture_replay_checkpoint());
        wasm.pending_decision = Some(DecisionContext::Priority(PriorityContext::new(
            alice,
            compute_legal_actions(&wasm.game, alice),
        )));

        let priority_ctx = match wasm.pending_decision.as_ref() {
            Some(DecisionContext::Priority(ctx)) => ctx,
            other => panic!("expected priority decision, got {other:?}"),
        };
        let play_mountain_index = priority_ctx
            .actions
            .iter()
            .position(
                |action| matches!(action, LegalAction::PlayLand { land_id } if *land_id == mountain_id),
            )
            .expect("expected play mountain action");

        wasm.dispatch(
            serde_wasm_bindgen::to_value(&json!({
                "type": "priority_action",
                "action_index": play_mountain_index,
            }))
            .expect("priority action command should serialize"),
        )
        .expect("playing mountain should succeed");

        let priority_ctx = match wasm.pending_decision.as_ref() {
            Some(DecisionContext::Priority(ctx)) => ctx,
            other => panic!("expected priority decision after land play, got {other:?}"),
        };
        let cast_bolt_index = priority_ctx
            .actions
            .iter()
            .position(|action| {
                matches!(
                    action,
                    LegalAction::CastSpell { spell_id, .. } if *spell_id == bolt_id
                )
            })
            .expect("expected cast lightning bolt action");

        wasm.dispatch(
            serde_wasm_bindgen::to_value(&json!({
                "type": "priority_action",
                "action_index": cast_bolt_index,
            }))
            .expect("cast spell command should serialize"),
        )
        .expect("casting lightning bolt should enter its decision chain");

        assert!(
            matches!(wasm.pending_decision, Some(DecisionContext::Targets(_))),
            "lightning bolt cast should be waiting on targets"
        );

        wasm.dispatch(
            serde_wasm_bindgen::to_value(&json!({
                "type": "select_targets",
                "targets": [
                    { "kind": "player", "player": bob.0 }
                ],
            }))
            .expect("target selection command should serialize"),
        )
        .expect("choosing the lightning bolt target should succeed");

        let pending_cast_stack_id = wasm
            .priority_state
            .pending_cast
            .as_ref()
            .map(|p| p.stack_id);
        let snapshot = GameSnapshot::from_game(
            &wasm.game,
            wasm.perspective,
            wasm.pending_decision.as_ref(),
            None,
            wasm.game_over.as_ref(),
            pending_cast_stack_id,
            wasm.active_resolving_stack_object.clone(),
            Vec::new(),
            None,
            wasm.is_cancelable(),
            None,
            0,
        );
        let stack_entry = snapshot
            .stack_objects
            .first()
            .expect("snapshot should include the cast lightning bolt on the stack");

        assert_eq!(stack_entry.name, "Lightning Bolt");
        assert_eq!(stack_entry.controller, alice.0);
        assert_eq!(stack_entry.targets.len(), 1);
        match &stack_entry.targets[0] {
            TargetChoiceView::Player { player, name } => {
                assert_eq!(*player, bob.0);
                assert_eq!(name, "Bob");
            }
            other => panic!("expected player target on stack snapshot, got {other:?}"),
        }
    }

    #[test]
    fn duress_snapshot_keeps_revealed_hand_visible_during_discard_choice() {
        let mut wasm = WasmGame::new();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        wasm.game.turn.active_player = alice;
        wasm.game.turn.priority_player = Some(alice);
        wasm.game.turn.phase = Phase::FirstMain;
        wasm.game.turn.step = None;

        let duress_id = wasm
            .add_card_to_zone(0, "Duress".to_string(), "hand".to_string(), true)
            .expect("should add Duress to hand");
        wasm.add_card_to_zone(
            0,
            "Black Lotus".to_string(),
            "battlefield".to_string(),
            true,
        )
        .expect("should add Black Lotus to battlefield");

        let hydra_id = wasm
            .add_card_to_zone(1, "Ulvenwald Hydra".to_string(), "hand".to_string(), true)
            .expect("should add Ulvenwald Hydra to hand");
        let peek_id = wasm
            .add_card_to_zone(1, "Peek".to_string(), "hand".to_string(), true)
            .expect("should add Peek to hand");
        let keyrune_id = wasm
            .add_card_to_zone(1, "Dimir Keyrune".to_string(), "hand".to_string(), true)
            .expect("should add Dimir Keyrune to hand");
        let forest_id = wasm
            .add_card_to_zone(1, "Forest".to_string(), "hand".to_string(), true)
            .expect("should add Forest to hand");

        wasm.priority_epoch_checkpoint = Some(wasm.capture_replay_checkpoint());
        wasm.pending_decision = Some(DecisionContext::Priority(PriorityContext::new(
            alice,
            compute_legal_actions(&wasm.game, alice),
        )));

        let priority_ctx = match wasm.pending_decision.as_ref() {
            Some(DecisionContext::Priority(ctx)) => ctx,
            other => panic!("expected priority decision, got {other:?}"),
        };
        let cast_duress_index = priority_ctx
            .actions
            .iter()
            .position(|action| {
                matches!(
                    action,
                    LegalAction::CastSpell { spell_id, .. } if *spell_id == ObjectId::from_raw(duress_id)
                )
            })
            .expect("expected cast Duress action");

        wasm.dispatch(
            serde_wasm_bindgen::to_value(&json!({
                "type": "priority_action",
                "action_index": cast_duress_index,
            }))
            .expect("cast spell command should serialize"),
        )
        .expect("casting Duress should enter its decision chain");

        assert!(
            matches!(wasm.pending_decision, Some(DecisionContext::Targets(_))),
            "Duress should be waiting on targets after cast"
        );

        wasm.dispatch(
            serde_wasm_bindgen::to_value(&json!({
                "type": "select_targets",
                "targets": [
                    { "kind": "player", "player": bob.0 }
                ],
            }))
            .expect("target selection command should serialize"),
        )
        .expect("choosing the Duress target should succeed");

        loop {
            match wasm.pending_decision.as_ref() {
                Some(DecisionContext::SelectOptions(options)) => {
                    let option_index = options
                        .options
                        .iter()
                        .find(|option| option.legal && option.description.contains("Black Lotus"))
                        .or_else(|| options.options.iter().find(|option| option.legal))
                        .map(|option| option.index)
                        .unwrap_or_else(|| {
                            panic!(
                                "expected a legal mana-payment option, got {:?}",
                                options
                                    .options
                                    .iter()
                                    .map(|option| option.description.clone())
                                    .collect::<Vec<_>>()
                            )
                        });
                    wasm.dispatch(
                        serde_wasm_bindgen::to_value(&json!({
                            "type": "select_options",
                            "option_indices": [option_index],
                        }))
                        .expect("option choice command should serialize"),
                    )
                    .expect("payment choice should succeed");
                }
                Some(DecisionContext::SelectObjects(_)) => break,
                Some(other) => panic!("unexpected Duress follow-up decision: {other:?}"),
                None => panic!("Duress resolved without presenting the discard decision"),
            }
        }

        let pending_cast_stack_id = wasm
            .priority_state
            .pending_cast
            .as_ref()
            .map(|p| p.stack_id);
        let snapshot = GameSnapshot::from_game(
            &wasm.game,
            wasm.perspective,
            wasm.pending_decision.as_ref(),
            None,
            wasm.game_over.as_ref(),
            pending_cast_stack_id,
            wasm.active_resolving_stack_object.clone(),
            Vec::new(),
            wasm.active_viewed_cards.as_ref(),
            wasm.is_cancelable(),
            None,
            0,
        );

        let viewed_cards = snapshot
            .viewed_cards
            .as_ref()
            .expect("Duress discard prompt should keep revealed cards in snapshot");
        assert_eq!(viewed_cards.visibility, "public");
        assert_eq!(viewed_cards.subject, bob.0);
        assert_eq!(
            viewed_cards.card_ids,
            vec![hydra_id, peek_id, keyrune_id, forest_id],
            "snapshot should surface every revealed hand card, not only legal discard choices"
        );

        let decision = match snapshot
            .decision
            .as_ref()
            .expect("snapshot should include the pending discard choice")
        {
            super::DecisionView::SelectObjects { candidates, .. } => candidates,
            other => panic!("expected select_objects decision, got {other:?}"),
        };
        let candidate_ids: Vec<u64> = decision.iter().map(|candidate| candidate.id).collect();
        assert_eq!(
            candidate_ids,
            vec![peek_id, keyrune_id],
            "discard decision should only offer the legal noncreature nonland cards"
        );
    }

    #[test]
    fn tayam_black_lotus_color_choice_keeps_paid_mana_state() {
        let mut wasm = WasmGame::new();
        let alice = PlayerId::from_index(0);

        wasm.game.turn.active_player = alice;
        wasm.game.turn.priority_player = Some(alice);
        wasm.game.turn.phase = Phase::FirstMain;
        wasm.game.turn.step = None;

        let tayam_id = ObjectId::from_raw(
            wasm.add_card_to_zone(
                alice.0,
                "Tayam, Luminous Enigma".to_string(),
                "battlefield".to_string(),
                true,
            )
            .expect("should add Tayam to battlefield"),
        );
        let ornithopter_ids: Vec<ObjectId> = (0..3)
            .map(|_| {
                ObjectId::from_raw(
                    wasm.add_card_to_zone(
                        alice.0,
                        "Ornithopter".to_string(),
                        "battlefield".to_string(),
                        false,
                    )
                    .expect("should add Ornithopter to battlefield"),
                )
            })
            .collect();
        let lotus_id = ObjectId::from_raw(
            wasm.add_card_to_zone(
                alice.0,
                "Black Lotus".to_string(),
                "battlefield".to_string(),
                true,
            )
            .expect("should add Black Lotus to battlefield"),
        );

        for ornithopter_id in &ornithopter_ids {
            let ornithopter = wasm
                .game
                .object(*ornithopter_id)
                .expect("ornithopter should exist");
            assert_eq!(
                ornithopter
                    .counters
                    .get(&crate::object::CounterType::Vigilance)
                    .copied(),
                Some(1),
                "Tayam should grant each Ornithopter a vigilance counter"
            );
        }

        wasm.priority_epoch_checkpoint = Some(wasm.capture_replay_checkpoint());
        wasm.pending_decision = Some(DecisionContext::Priority(PriorityContext::new(
            alice,
            compute_legal_actions(&wasm.game, alice),
        )));

        let priority_ctx = match wasm.pending_decision.as_ref() {
            Some(DecisionContext::Priority(ctx)) => ctx,
            other => panic!("expected priority decision, got {other:?}"),
        };
        let activate_index = priority_ctx
            .actions
            .iter()
            .position(|action| matches!(action, LegalAction::ActivateAbility { source, .. } if *source == tayam_id))
            .expect("expected Tayam activation action");

        wasm.dispatch(
            serde_wasm_bindgen::to_value(&json!({
                "type": "priority_action",
                "action_index": activate_index,
            }))
            .expect("priority action command should serialize"),
        )
        .expect("activating Tayam should begin its cost-payment chain");

        let next_cost_ctx = match wasm.pending_decision.as_ref() {
            Some(DecisionContext::SelectOptions(ctx)) => ctx,
            other => panic!("expected next-cost chooser after activating Tayam, got {other:?}"),
        };
        let mana_choice = next_cost_ctx
            .options
            .iter()
            .find(|option| option.legal && option.description.contains("Pay {3}"))
            .map(|option| option.index)
            .unwrap_or(0);

        wasm.dispatch(
            serde_wasm_bindgen::to_value(&json!({
                "type": "select_options",
                "option_indices": [mana_choice],
            }))
            .expect("next-cost choice command should serialize"),
        )
        .expect("choosing Tayam's mana cost should advance to mana payment");

        let mana_ctx = match wasm.pending_decision.as_ref() {
            Some(DecisionContext::SelectOptions(ctx)) => ctx,
            other => panic!("expected mana payment prompt after choosing mana, got {other:?}"),
        };
        let lotus_option = mana_ctx
            .options
            .iter()
            .find(|option| option.legal && option.description.contains("Black Lotus"))
            .map(|option| option.index)
            .expect("mana payment prompt should offer Black Lotus");

        wasm.dispatch(
            serde_wasm_bindgen::to_value(&json!({
                "type": "select_options",
                "option_indices": [lotus_option],
            }))
            .expect("Black Lotus mana payment command should serialize"),
        )
        .expect("activating Black Lotus during Tayam payment should succeed");

        assert!(
            !wasm.game.battlefield.contains(&lotus_id),
            "Black Lotus should be sacrificed immediately once selected"
        );
        assert!(
            matches!(wasm.pending_decision, Some(DecisionContext::Colors(_))),
            "Black Lotus should surface a color-choice prompt"
        );

        let colors_ctx = match wasm.pending_decision.as_ref() {
            Some(DecisionContext::Colors(ctx)) => ctx,
            other => panic!("expected color-choice decision, got {other:?}"),
        };
        let green_option = colors_for_context(colors_ctx)
            .iter()
            .position(|color| *color == crate::color::Color::Green)
            .expect("green should be a legal Black Lotus color choice");

        wasm.dispatch(
            serde_wasm_bindgen::to_value(&json!({
                "type": "select_options",
                "option_indices": [green_option],
            }))
            .expect("color choice command should serialize"),
        )
        .expect("choosing a Black Lotus color should replay the payment chain");

        assert!(
            !wasm.game.battlefield.contains(&lotus_id),
            "Black Lotus should remain sacrificed after the replayed color choice resolves"
        );
        let pool = &wasm
            .game
            .player(alice)
            .expect("alice should exist")
            .mana_pool;
        assert_eq!(
            pool.green, 2,
            "one of the three chosen mana should pay the current generic pip and two should remain"
        );

        let pending_activation = wasm
            .priority_state
            .pending_activation
            .as_ref()
            .expect("Tayam activation should still be in progress");
        assert_eq!(
            pending_activation.remaining_mana_pips.len(),
            2,
            "paying Black Lotus into Tayam should consume exactly one generic pip"
        );
        assert!(
            matches!(
                wasm.pending_decision,
                Some(DecisionContext::SelectOptions(_))
            ),
            "after choosing the color, the UI should advance to the next payment prompt"
        );
    }

    #[test]
    fn tayam_counter_choice_keeps_removed_counters_state() {
        let mut wasm = WasmGame::new();
        let alice = PlayerId::from_index(0);

        wasm.game.turn.active_player = alice;
        wasm.game.turn.priority_player = Some(alice);
        wasm.game.turn.phase = Phase::FirstMain;
        wasm.game.turn.step = None;

        let tayam_id = ObjectId::from_raw(
            wasm.add_card_to_zone(
                alice.0,
                "Tayam, Luminous Enigma".to_string(),
                "battlefield".to_string(),
                true,
            )
            .expect("should add Tayam to battlefield"),
        );
        let ornithopter_ids: Vec<ObjectId> = (0..3)
            .map(|_| {
                ObjectId::from_raw(
                    wasm.add_card_to_zone(
                        alice.0,
                        "Ornithopter".to_string(),
                        "battlefield".to_string(),
                        false,
                    )
                    .expect("should add Ornithopter to battlefield"),
                )
            })
            .collect();

        wasm.priority_epoch_checkpoint = Some(wasm.capture_replay_checkpoint());
        wasm.pending_decision = Some(DecisionContext::Priority(PriorityContext::new(
            alice,
            compute_legal_actions(&wasm.game, alice),
        )));

        let priority_ctx = match wasm.pending_decision.as_ref() {
            Some(DecisionContext::Priority(ctx)) => ctx,
            other => panic!("expected priority decision, got {other:?}"),
        };
        let activate_index = priority_ctx
            .actions
            .iter()
            .position(|action| matches!(action, LegalAction::ActivateAbility { source, .. } if *source == tayam_id))
            .expect("expected Tayam activation action");

        wasm.dispatch(
            serde_wasm_bindgen::to_value(&json!({
                "type": "priority_action",
                "action_index": activate_index,
            }))
            .expect("priority action command should serialize"),
        )
        .expect("activating Tayam should begin its cost-payment chain");

        let next_cost_ctx = match wasm.pending_decision.as_ref() {
            Some(DecisionContext::SelectOptions(ctx)) => ctx,
            other => panic!("expected next-cost chooser after activating Tayam, got {other:?}"),
        };
        let counter_choice = next_cost_ctx
            .options
            .iter()
            .find(|option| option.legal && option.description.contains("Remove three counters"))
            .map(|option| option.index)
            .unwrap_or(1);

        wasm.dispatch(
            serde_wasm_bindgen::to_value(&json!({
                "type": "select_options",
                "option_indices": [counter_choice],
            }))
            .expect("counter-cost choice command should serialize"),
        )
        .expect("choosing Tayam's counter cost should open distribution");

        let distribute_ctx = match wasm.pending_decision.as_ref() {
            Some(DecisionContext::Distribute(ctx)) => ctx,
            other => panic!("expected counter distribution prompt, got {other:?}"),
        };
        let distribution_indices: Vec<usize> = ornithopter_ids
            .iter()
            .map(|ornithopter_id| {
                distribute_ctx
                    .targets
                    .iter()
                    .position(|target| target.target == Target::Object(*ornithopter_id))
                    .expect("each Ornithopter should be a legal distribution target")
            })
            .collect();

        wasm.dispatch(
            serde_wasm_bindgen::to_value(&json!({
                "type": "select_options",
                "option_indices": distribution_indices,
            }))
            .expect("distribution command should serialize"),
        )
        .expect("distributing Tayam's counters across the Ornithopters should succeed");

        for ornithopter_id in &ornithopter_ids {
            let counters_ctx = match wasm.pending_decision.as_ref() {
                Some(DecisionContext::Counters(ctx)) => ctx,
                other => panic!("expected counter-removal prompt, got {other:?}"),
            };
            assert_eq!(
                counters_ctx.target, *ornithopter_id,
                "counter-removal replay should advance through the distributed targets in order"
            );

            wasm.dispatch(
                serde_wasm_bindgen::to_value(&json!({
                    "type": "select_options",
                    "option_indices": [0],
                }))
                .expect("counter selection command should serialize"),
            )
            .expect("removing the selected vigilance counter should succeed");

            let ornithopter = wasm
                .game
                .object(*ornithopter_id)
                .expect("ornithopter should still exist");
            assert_eq!(
                ornithopter
                    .counters
                    .get(&crate::object::CounterType::Vigilance)
                    .copied()
                    .unwrap_or(0),
                0,
                "selected Ornithopter should keep its counter removed after replay"
            );
        }

        let pending_activation = wasm
            .priority_state
            .pending_activation
            .as_ref()
            .expect("Tayam activation should still be in progress");
        assert!(
            pending_activation.remaining_cost_steps.is_empty(),
            "after removing all three counters, the counter-payment step should be complete"
        );
        assert!(
            matches!(
                wasm.pending_decision,
                Some(DecisionContext::SelectOptions(_))
            ),
            "after paying the counter cost, the UI should advance to the remaining mana payment"
        );
    }

    #[test]
    fn tayam_activation_can_resolve_and_choose_graveyard_return_target() {
        let mut wasm = WasmGame::new();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        wasm.game.turn.active_player = alice;
        wasm.game.turn.priority_player = Some(alice);
        wasm.game.turn.phase = Phase::FirstMain;
        wasm.game.turn.step = None;

        let tayam_id = ObjectId::from_raw(
            wasm.add_card_to_zone(
                alice.0,
                "Tayam, Luminous Enigma".to_string(),
                "battlefield".to_string(),
                true,
            )
            .expect("should add Tayam to battlefield"),
        );
        let wall_id = ObjectId::from_raw(
            wasm.add_card_to_zone(
                alice.0,
                "Wall of Roots".to_string(),
                "battlefield".to_string(),
                false,
            )
            .expect("should add Wall of Roots to battlefield"),
        );
        let ornithopter_id = ObjectId::from_raw(
            wasm.add_card_to_zone(
                alice.0,
                "Ornithopter".to_string(),
                "battlefield".to_string(),
                false,
            )
            .expect("should add Ornithopter to battlefield"),
        );
        let forest_a = ObjectId::from_raw(
            wasm.add_card_to_zone(
                alice.0,
                "Forest".to_string(),
                "battlefield".to_string(),
                false,
            )
            .expect("should add first Forest to battlefield"),
        );
        let forest_b = ObjectId::from_raw(
            wasm.add_card_to_zone(
                alice.0,
                "Forest".to_string(),
                "battlefield".to_string(),
                false,
            )
            .expect("should add second Forest to battlefield"),
        );
        let return_target = ObjectId::from_raw(
            wasm.add_card_to_zone(
                alice.0,
                "Forest".to_string(),
                "graveyard".to_string(),
                false,
            )
            .expect("should add return target to graveyard"),
        );

        assert!(
            wasm.game.player(bob).is_some(),
            "second player should exist for priority passing"
        );

        wasm.priority_epoch_checkpoint = Some(wasm.capture_replay_checkpoint());
        wasm.pending_decision = Some(DecisionContext::Priority(PriorityContext::new(
            alice,
            compute_legal_actions(&wasm.game, alice),
        )));

        let priority_ctx = match wasm.pending_decision.as_ref() {
            Some(DecisionContext::Priority(ctx)) => ctx,
            other => panic!("expected priority decision, got {other:?}"),
        };
        let activate_index = priority_ctx
            .actions
            .iter()
            .position(|action| matches!(action, LegalAction::ActivateAbility { source, .. } if *source == tayam_id))
            .expect("expected Tayam activation action");

        wasm.dispatch(
            serde_wasm_bindgen::to_value(&json!({
                "type": "priority_action",
                "action_index": activate_index,
            }))
            .expect("priority action command should serialize"),
        )
        .expect("activating Tayam should begin its cost-payment chain");

        loop {
            let pending = wasm
                .pending_decision
                .clone()
                .expect("Tayam activation should still have a pending decision");
            match pending {
                DecisionContext::SelectOptions(ctx) => {
                    let choice = if ctx.description.contains("Choose next cost") {
                        ctx.options
                            .iter()
                            .find(|option| option.legal && option.description.contains("Pay {3}"))
                            .map(|option| option.index)
                            .expect("next-cost chooser should offer the mana payment")
                    } else if ctx.description.contains("Pay mana") {
                        if let Some(option) = ctx.options.iter().find(|option| {
                            option.legal && option.description.contains("Wall of Roots")
                        }) {
                            option.index
                        } else {
                            ctx.options
                                .iter()
                                .find(|option| {
                                    option.legal && option.description.contains("Forest")
                                })
                                .map(|option| option.index)
                                .expect("mana payment prompt should offer a legal mana source")
                        }
                    } else if ctx.description.contains("Choose next cost") {
                        unreachable!("handled above")
                    } else {
                        ctx.options
                            .iter()
                            .find(|option| {
                                option.legal && option.description.contains("Remove three counters")
                            })
                            .map(|option| option.index)
                            .or_else(|| {
                                ctx.options
                                    .iter()
                                    .find(|option| {
                                        option.legal && option.description.contains("Pass")
                                    })
                                    .map(|option| option.index)
                            })
                            .unwrap_or_else(|| {
                                ctx.options
                                    .iter()
                                    .find(|option| option.legal)
                                    .map(|option| option.index)
                                    .expect("select-options prompt should offer a legal choice")
                            })
                    };

                    wasm.dispatch(
                        serde_wasm_bindgen::to_value(&json!({
                            "type": "select_options",
                            "option_indices": [choice],
                        }))
                        .expect("select-options command should serialize"),
                    )
                    .expect("dispatching Tayam select-options step should succeed");
                }
                DecisionContext::Distribute(ctx) => {
                    let wall_index = ctx
                        .targets
                        .iter()
                        .position(|target| target.target == Target::Object(wall_id))
                        .expect("Wall of Roots should be a legal distribution target");
                    let ornithopter_index = ctx
                        .targets
                        .iter()
                        .position(|target| target.target == Target::Object(ornithopter_id))
                        .expect("Ornithopter should be a legal distribution target");
                    let indices = vec![wall_index, wall_index, ornithopter_index];
                    wasm.dispatch(
                        serde_wasm_bindgen::to_value(&json!({
                            "type": "select_options",
                            "option_indices": indices,
                        }))
                        .expect("distribute command should serialize"),
                    )
                    .expect("counter distribution should succeed");
                }
                DecisionContext::Counters(ctx) => {
                    let counter_index = ctx
                        .available_counters
                        .iter()
                        .position(|(_, available)| *available > 0)
                        .expect("counter prompt should offer at least one removable counter");
                    wasm.dispatch(
                        serde_wasm_bindgen::to_value(&json!({
                            "type": "select_options",
                            "option_indices": [counter_index],
                        }))
                        .expect("counter selection command should serialize"),
                    )
                    .expect("counter removal should succeed");
                }
                DecisionContext::Priority(ctx) => {
                    let pass_index = ctx
                        .actions
                        .iter()
                        .position(|action| matches!(action, LegalAction::PassPriority))
                        .expect("priority prompt should include pass");
                    wasm.dispatch(
                        serde_wasm_bindgen::to_value(&json!({
                            "type": "priority_action",
                            "action_index": pass_index,
                        }))
                        .expect("priority pass command should serialize"),
                    )
                    .expect("priority pass during Tayam line should succeed");
                }
                DecisionContext::SelectObjects(ctx) => {
                    let target_id = ctx
                        .candidates
                        .iter()
                        .find(|candidate| candidate.legal && candidate.id == return_target)
                        .map(|candidate| candidate.id.0)
                        .expect("graveyard return target should be legal");
                    wasm.dispatch(
                        serde_wasm_bindgen::to_value(&json!({
                            "type": "select_objects",
                            "object_ids": [target_id],
                        }))
                        .expect("graveyard target command should serialize"),
                    )
                    .expect("selecting Tayam's graveyard return target should succeed");
                    break;
                }
                other => panic!("unexpected Tayam resolution decision: {other:?}"),
            }
        }

        assert!(
            !wasm.game.battlefield.contains(&forest_a)
                || !wasm.game.battlefield.contains(&forest_b),
            "at least one Forest should remain tapped after paying Tayam's mana cost"
        );
        assert!(
            wasm.game.battlefield.iter().any(|id| {
                wasm.game
                    .object(*id)
                    .is_some_and(|obj| obj.name == "Forest" && obj.owner == alice)
            }),
            "a Forest should still exist on the battlefield after Tayam resolves"
        );
    }

    #[test]
    fn polluted_delta_resolution_choice_keeps_paid_costs_and_resolved_land() {
        let mut wasm = WasmGame::new();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        wasm.game.turn.active_player = alice;
        wasm.game.turn.priority_player = Some(alice);
        wasm.game.turn.phase = Phase::FirstMain;
        wasm.game.turn.step = None;

        let delta_id =
            wasm.game
                .create_object_from_definition(&polluted_delta(), alice, Zone::Battlefield);
        let island_id =
            wasm.game
                .create_object_from_definition(&basic_island(), alice, Zone::Library);
        wasm.game
            .create_object_from_definition(&basic_mountain(), alice, Zone::Library);

        wasm.priority_epoch_checkpoint = Some(wasm.capture_replay_checkpoint());
        wasm.pending_decision = Some(DecisionContext::Priority(PriorityContext::new(
            alice,
            compute_legal_actions(&wasm.game, alice),
        )));

        let priority_ctx = match wasm.pending_decision.as_ref() {
            Some(DecisionContext::Priority(ctx)) => ctx,
            other => panic!("expected priority decision, got {other:?}"),
        };
        let activate_index = priority_ctx
            .actions
            .iter()
            .position(|action| {
                matches!(
                    action,
                    LegalAction::ActivateAbility { source, .. } if *source == delta_id
                )
            })
            .expect("expected Polluted Delta activation action");

        wasm.dispatch(
            serde_wasm_bindgen::to_value(&json!({
                "type": "priority_action",
                "action_index": activate_index,
            }))
            .expect("priority action command should serialize"),
        )
        .expect("activating Polluted Delta should succeed");

        assert!(
            wasm.game.player(bob).is_some(),
            "second player should exist for the pass-priority sequence"
        );
        assert!(
            !wasm.game.battlefield.contains(&delta_id),
            "Polluted Delta should be sacrificed during activation"
        );
        assert!(
            wasm.game
                .player(alice)
                .expect("alice should exist")
                .graveyard
                .contains(&delta_id),
            "Polluted Delta should be in the graveyard after activation"
        );
        assert_eq!(
            wasm.game.player(alice).expect("alice should exist").life,
            19,
            "Polluted Delta activation should pay 1 life immediately"
        );

        loop {
            let pending = wasm
                .pending_decision
                .clone()
                .expect("fetchland line should keep producing prompts until the search resolves");
            match pending {
                DecisionContext::Priority(ctx) => {
                    let pass_index = ctx
                        .actions
                        .iter()
                        .position(|action| matches!(action, LegalAction::PassPriority))
                        .expect("priority prompt should include pass");
                    wasm.dispatch(
                        serde_wasm_bindgen::to_value(&json!({
                            "type": "priority_action",
                            "action_index": pass_index,
                        }))
                        .expect("priority pass command should serialize"),
                    )
                    .expect("passing priority during fetchland line should succeed");
                }
                DecisionContext::SelectObjects(ctx) => {
                    let choice = ctx
                        .candidates
                        .iter()
                        .find(|candidate| candidate.legal && candidate.id == island_id)
                        .map(|candidate| candidate.id.0)
                        .expect("basic Island should be a legal fetchland search result");
                    wasm.dispatch(
                        serde_wasm_bindgen::to_value(&json!({
                            "type": "select_objects",
                            "object_ids": [choice],
                        }))
                        .expect("fetchland selection command should serialize"),
                    )
                    .expect("choosing the searched land should succeed");
                    break;
                }
                other => panic!("unexpected Polluted Delta follow-up decision: {other:?}"),
            }
        }

        assert_eq!(
            wasm.game.player(alice).expect("alice should exist").life,
            19,
            "resolving the fetchland search should not rewind the paid life cost"
        );
        assert!(
            !wasm.game.battlefield.contains(&delta_id),
            "resolving the fetchland search should not put Polluted Delta back onto the battlefield"
        );
        assert!(
            wasm.game
                .player(alice)
                .expect("alice should exist")
                .graveyard
                .contains(&delta_id),
            "Polluted Delta should remain in the graveyard after the search completes"
        );
        assert!(
            wasm.game.battlefield.contains(&island_id),
            "the chosen Island should enter the battlefield"
        );
        assert!(
            !wasm
                .game
                .player(alice)
                .expect("alice should exist")
                .library
                .contains(&island_id),
            "the chosen Island should leave the library after resolution"
        );
        assert!(
            matches!(wasm.pending_decision, Some(DecisionContext::Priority(_))),
            "after the search resolves, the game should return to priority"
        );
    }

    #[test]
    fn committed_resolution_prompt_is_not_cancelable() {
        let mut wasm = WasmGame::new();
        let alice = PlayerId::from_index(0);
        wasm.priority_epoch_checkpoint = Some(wasm.capture_replay_checkpoint());
        wasm.priority_epoch_has_undoable_action = true;
        wasm.pending_decision = Some(DecisionContext::SelectObjects(SelectObjectsContext::new(
            alice,
            None,
            "Resolve effect",
            vec![SelectableObject::new(ObjectId::from_raw(1), "Choice")],
            1,
            Some(1),
        )));
        assert!(
            wasm.pending_action_checkpoint.is_none(),
            "committed follow-up prompts should not retain the action-chain undo checkpoint"
        );
        assert!(
            !wasm.is_cancelable(),
            "once the spell has resolved into its imprint prompt, undo should be disabled"
        );
    }

    #[test]
    fn emrakul_cast_trigger_needs_targets_in_four_player_game() {
        let mut wasm = WasmGame::new();
        wasm.initialize_empty_match(
            vec![
                "Alice".to_string(),
                "Bob".to_string(),
                "Charlie".to_string(),
                "Dana".to_string(),
            ],
            20,
            1,
        );

        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let charlie = PlayerId::from_index(2);
        let dana = PlayerId::from_index(3);

        wasm.game.turn.active_player = alice;
        wasm.game.turn.priority_player = Some(alice);
        wasm.game.turn.phase = Phase::FirstMain;
        wasm.game.turn.step = None;

        let emrakul_id = wasm.game.create_object_from_definition(
            &emrakul_the_promised_end(),
            alice,
            Zone::Stack,
        );
        let (emrakul_stable_id, emrakul_name) = wasm
            .game
            .object(emrakul_id)
            .map(|object| (object.stable_id, object.name.clone()))
            .expect("Emrakul spell object should exist");
        wasm.game.push_to_stack(
            StackEntry::new(emrakul_id, alice).with_source_info(emrakul_stable_id, emrakul_name),
        );

        let event = TriggerEvent::new_with_provenance(
            SpellCastEvent::new(emrakul_id, alice, Zone::Hand),
            crate::provenance::ProvNodeId::default(),
        );
        for trigger in check_triggers(&wasm.game, &event) {
            wasm.trigger_queue.add(trigger);
        }

        assert_eq!(
            wasm.trigger_queue.entries.len(),
            1,
            "Emrakul should queue its cast trigger from the stack"
        );

        let checkpoint = wasm.capture_replay_checkpoint();
        let outcome = wasm
            .execute_with_replay(&checkpoint, &ReplayRoot::Advance, &[])
            .expect("auto-advance should reach Emrakul's trigger decision");

        let targets_ctx = match outcome {
            ReplayOutcome::NeedsDecision(DecisionContext::Targets(ctx)) => ctx,
            other => panic!("expected Emrakul cast trigger target prompt, got {other:?}"),
        };

        assert_eq!(
            targets_ctx.player, alice,
            "the caster should choose Emrakul's target opponent"
        );
        assert_eq!(
            targets_ctx.requirements.len(),
            1,
            "Emrakul should ask for exactly one target requirement"
        );

        let legal_targets = &targets_ctx.requirements[0].legal_targets;
        let legal_players: Vec<PlayerId> = legal_targets
            .iter()
            .filter_map(|target| match target {
                crate::game_state::Target::Player(player) => Some(*player),
                crate::game_state::Target::Object(_) => None,
            })
            .collect();
        assert_eq!(
            legal_players,
            vec![bob, charlie, dana],
            "all opponents should be legal Emrakul targets"
        );

        assert_eq!(
            wasm.game.stack.len(),
            1,
            "replay should leave the live game advanced to the pending target decision"
        );
    }

    #[test]
    fn priority_decision_routing_uses_replay_for_generic_modal_choices() {
        let boolean = DecisionContext::Boolean(BooleanContext::new(
            PlayerId::from_index(0),
            None,
            "play an additional land this turn",
        ));
        let select_objects = DecisionContext::SelectObjects(SelectObjectsContext::new(
            PlayerId::from_index(0),
            None,
            "choose a land",
            vec![SelectableObject::new(ObjectId::from_raw(1), "Forest")],
            1,
            Some(1),
        ));
        let select_options =
            DecisionContext::SelectOptions(crate::decisions::context::SelectOptionsContext::new(
                PlayerId::from_index(0),
                None,
                "choose a mode",
                vec![SelectableOption::new(0, "Only option")],
                1,
                1,
            ));
        let wasm = WasmGame::new();

        assert!(
            wasm.decision_requires_root_reexecution(&boolean),
            "boolean prompts should replay from the original root response"
        );
        assert!(
            wasm.decision_requires_root_reexecution(&select_objects),
            "resolution-time object prompts should replay from the original root response"
        );
        assert!(
            !wasm.decision_requires_root_reexecution(&select_options),
            "generic select-options prompts do not require full root reexecution"
        );
        assert!(
            !wasm.decision_uses_live_priority_response(&select_options),
            "generic select-options prompts should route through replay continuations, not the live priority responder"
        );
    }

    #[test]
    fn priority_decision_routing_keeps_cost_option_prompts_on_live_responder() {
        let mut wasm = WasmGame::new();
        wasm.priority_state.pending_cast = Some(PendingCast::new(
            ObjectId::from_raw(1),
            Zone::Hand,
            PlayerId::from_index(0),
            ProvNodeId::default(),
            CastStage::ChoosingOptionalCosts,
            None,
            Vec::new(),
            CastingMethod::Normal,
            OptionalCostsPaid::new(1),
            None,
            ObjectId::from_raw(1),
        ));

        let select_options =
            DecisionContext::SelectOptions(crate::decisions::context::SelectOptionsContext::new(
                PlayerId::from_index(0),
                Some(ObjectId::from_raw(1)),
                "Choose optional costs",
                vec![SelectableOption::new(0, "Kicker")],
                0,
                1,
            ));

        assert!(
            wasm.decision_uses_live_priority_response(&select_options),
            "cost-selection select-options prompts should stay on the live priority responder"
        );
    }

    #[test]
    fn cultivator_colossus_etb_does_not_repeat_may_prompt_before_next_land_choice() {
        let mut wasm = WasmGame::new();

        let forest_a = wasm
            .add_card_to_zone(0, "Forest".to_string(), "hand".to_string(), true)
            .expect("first Forest should be added to hand");
        let forest_b = wasm
            .add_card_to_zone(0, "Forest".to_string(), "hand".to_string(), true)
            .expect("second Forest should be added to hand");
        wasm.add_card_to_zone(0, "Grizzly Bears".to_string(), "library".to_string(), true)
            .expect("first library filler should be added");
        wasm.add_card_to_zone(0, "Grizzly Bears".to_string(), "library".to_string(), true)
            .expect("second library filler should be added");

        wasm.add_card_to_zone(
            0,
            "Cultivator Colossus".to_string(),
            "battlefield".to_string(),
            false,
        )
        .expect("Cultivator Colossus should enter with ETB processing");

        let first_may = match wasm.pending_decision.as_ref() {
            Some(DecisionContext::Boolean(ctx)) => ctx,
            other => panic!("expected Cultivator Colossus may prompt, got {other:?}"),
        };
        assert!(
            first_may
                .description
                .to_ascii_lowercase()
                .contains("put a land card from your hand onto the battlefield tapped"),
            "expected Cultivator Colossus may text, got {:?}",
            first_may.description
        );

        wasm.dispatch(
            serde_wasm_bindgen::to_value(&json!({
                "type": "select_options",
                "option_indices": [1],
            }))
            .expect("yes choice should serialize"),
        )
        .expect("accepting the first Cultivator iteration should succeed");

        let first_land_choice = match wasm.pending_decision.as_ref() {
            Some(DecisionContext::SelectObjects(ctx)) => ctx,
            other => panic!("expected first land selection prompt, got {other:?}"),
        };
        let mut first_candidates: Vec<u64> = first_land_choice
            .candidates
            .iter()
            .filter(|candidate| candidate.legal)
            .map(|candidate| candidate.id.0)
            .collect();
        first_candidates.sort_unstable();
        assert_eq!(
            first_candidates,
            vec![forest_a, forest_b],
            "first land selection should offer both lands in hand"
        );

        wasm.dispatch(
            serde_wasm_bindgen::to_value(&json!({
                "type": "select_objects",
                "object_ids": [forest_a],
            }))
            .expect("first land selection should serialize"),
        )
        .expect("choosing the first land should succeed");

        assert_eq!(
            wasm.game
                .player(PlayerId::from_index(0))
                .expect("player should exist")
                .hand
                .len(),
            1,
            "after choosing a land, the live game state should keep that land out of hand"
        );
        let lands_on_battlefield = wasm
            .game
            .battlefield
            .iter()
            .filter(|&&id| {
                wasm.game.object(id).is_some_and(|object| {
                    object.is_land() && object.owner == PlayerId::from_index(0)
                })
            })
            .count();
        assert_eq!(
            lands_on_battlefield, 1,
            "the chosen land should already be on the battlefield before the next repeat decision"
        );

        let second_may = match wasm.pending_decision.as_ref() {
            Some(DecisionContext::Boolean(ctx)) => ctx,
            other => panic!("expected second Cultivator may prompt, got {other:?}"),
        };
        assert!(
            second_may
                .description
                .to_ascii_lowercase()
                .contains("put a land card from your hand onto the battlefield tapped"),
            "expected repeated Cultivator may text, got {:?}",
            second_may.description
        );

        wasm.dispatch(
            serde_wasm_bindgen::to_value(&json!({
                "type": "select_options",
                "option_indices": [1],
            }))
            .expect("second yes choice should serialize"),
        )
        .expect("accepting the second Cultivator iteration should succeed");

        let second_land_choice = match wasm.pending_decision.as_ref() {
            Some(DecisionContext::SelectObjects(ctx)) => ctx,
            other => panic!("expected second land selection prompt, got {other:?}"),
        };
        let second_candidates: Vec<u64> = second_land_choice
            .candidates
            .iter()
            .filter(|candidate| candidate.legal)
            .map(|candidate| candidate.id.0)
            .collect();
        assert_eq!(
            second_candidates,
            vec![forest_b],
            "after one land is chosen, the next prompt should go straight to the remaining land"
        );
    }

    #[test]
    fn live_resolution_follow_up_prompts_restore_resolving_stack_object() {
        let mut wasm = WasmGame::new();

        wasm.add_card_to_zone(0, "Forest".to_string(), "hand".to_string(), true)
            .expect("first Forest should be added to hand");
        wasm.add_card_to_zone(0, "Grizzly Bears".to_string(), "library".to_string(), true)
            .expect("library filler should be added");

        wasm.add_card_to_zone(
            0,
            "Cultivator Colossus".to_string(),
            "battlefield".to_string(),
            false,
        )
        .expect("Cultivator Colossus should enter with ETB processing");

        let resolving_checkpoint = wasm
            .pending_live_continuation
            .as_ref()
            .map(|continuation| continuation.checkpoint.clone())
            .expect("Cultivator ETB prompt should retain the committed resolution checkpoint");
        let next_ctx = wasm
            .pending_decision
            .clone()
            .expect("Cultivator ETB prompt should be pending");
        let expected_resolving_id = wasm
            .active_resolving_stack_object
            .as_ref()
            .map(|entry| entry.id)
            .expect("Cultivator ETB prompt should expose the resolving stack entry");

        wasm.clear_active_resolving_stack_object();
        assert!(
            wasm.active_resolving_stack_object.is_none(),
            "test setup should clear the resolving entry before simulating live dispatch"
        );

        wasm.finish_live_priority_dispatch(
            GameProgress::NeedsDecisionCtx(next_ctx),
            None,
            Some(resolving_checkpoint),
        )
        .expect("live follow-up prompt should snapshot cleanly");

        assert_eq!(
            wasm.active_resolving_stack_object
                .as_ref()
                .map(|entry| entry.id),
            Some(expected_resolving_id),
            "live follow-up prompts should restore the resolving stack entry from the committed resolution checkpoint"
        );
    }

    #[test]
    fn cultivator_colossus_snapshot_tracks_repeat_iteration_state() {
        let mut wasm = WasmGame::new();
        let alice = PlayerId::from_index(0);

        let forest_a = wasm
            .add_card_to_zone(0, "Forest".to_string(), "hand".to_string(), true)
            .expect("first Forest should be added to hand");
        let forest_b = wasm
            .add_card_to_zone(0, "Forest".to_string(), "hand".to_string(), true)
            .expect("second Forest should be added to hand");
        wasm.add_card_to_zone(0, "Grizzly Bears".to_string(), "library".to_string(), true)
            .expect("first library filler should be added");
        wasm.add_card_to_zone(0, "Grizzly Bears".to_string(), "library".to_string(), true)
            .expect("second library filler should be added");

        wasm.add_card_to_zone(
            0,
            "Cultivator Colossus".to_string(),
            "battlefield".to_string(),
            false,
        )
        .expect("Cultivator Colossus should enter with ETB processing");

        let pending_cast_stack_id = wasm
            .priority_state
            .pending_cast
            .as_ref()
            .map(|p| p.stack_id);
        let snapshot = GameSnapshot::from_game(
            &wasm.game,
            wasm.perspective,
            wasm.pending_decision.as_ref(),
            None,
            wasm.game_over.as_ref(),
            pending_cast_stack_id,
            wasm.active_resolving_stack_object.clone(),
            Vec::new(),
            wasm.active_viewed_cards.as_ref(),
            wasm.is_cancelable(),
            None,
            0,
        );
        let resolving_stack_object = snapshot
            .resolving_stack_object
            .as_ref()
            .expect("Cultivator ETB prompt should expose the resolving trigger in the snapshot");
        assert_eq!(resolving_stack_object.name, "Cultivator Colossus");
        assert_eq!(
            resolving_stack_object.ability_kind.as_deref(),
            Some("Triggered"),
            "the pinned resolving entry should surface Cultivator's ETB as a triggered ability"
        );
        assert!(
            snapshot.stack_objects.is_empty(),
            "the real stack should stay empty while the UI-only resolving entry is shown separately"
        );

        wasm.dispatch(
            serde_wasm_bindgen::to_value(&json!({
                "type": "select_options",
                "option_indices": [1],
            }))
            .expect("yes choice should serialize"),
        )
        .expect("accepting the first Cultivator iteration should succeed");

        let pending_cast_stack_id = wasm
            .priority_state
            .pending_cast
            .as_ref()
            .map(|p| p.stack_id);
        let snapshot = GameSnapshot::from_game(
            &wasm.game,
            wasm.perspective,
            wasm.pending_decision.as_ref(),
            None,
            wasm.game_over.as_ref(),
            pending_cast_stack_id,
            wasm.active_resolving_stack_object.clone(),
            Vec::new(),
            wasm.active_viewed_cards.as_ref(),
            wasm.is_cancelable(),
            None,
            0,
        );
        let me = snapshot
            .players
            .iter()
            .find(|player| player.id == alice.0)
            .expect("perspective player should exist in snapshot");
        let mut hand_ids: Vec<u64> = me.hand_cards.iter().map(|card| card.id).collect();
        hand_ids.sort_unstable();
        assert_eq!(
            hand_ids,
            vec![forest_a, forest_b],
            "first land-choice snapshot should still show both lands in hand"
        );
        let first_choice = match snapshot
            .decision
            .as_ref()
            .expect("snapshot should include first land-choice decision")
        {
            super::DecisionView::SelectObjects { candidates, .. } => candidates,
            other => panic!("expected select_objects snapshot, got {other:?}"),
        };
        let mut first_candidates: Vec<u64> = first_choice
            .iter()
            .filter(|candidate| candidate.legal)
            .map(|candidate| candidate.id)
            .collect();
        first_candidates.sort_unstable();
        assert_eq!(
            first_candidates,
            vec![forest_a, forest_b],
            "first land-choice snapshot should offer both lands"
        );
        assert!(
            snapshot.resolving_stack_object.is_some(),
            "the resolving Cultivator trigger should stay visible during the land-choice step"
        );

        wasm.dispatch(
            serde_wasm_bindgen::to_value(&json!({
                "type": "select_objects",
                "object_ids": [forest_a],
            }))
            .expect("first land selection should serialize"),
        )
        .expect("choosing the first land should succeed");

        let pending_cast_stack_id = wasm
            .priority_state
            .pending_cast
            .as_ref()
            .map(|p| p.stack_id);
        let snapshot = GameSnapshot::from_game(
            &wasm.game,
            wasm.perspective,
            wasm.pending_decision.as_ref(),
            None,
            wasm.game_over.as_ref(),
            pending_cast_stack_id,
            wasm.active_resolving_stack_object.clone(),
            Vec::new(),
            wasm.active_viewed_cards.as_ref(),
            wasm.is_cancelable(),
            None,
            1,
        );
        let me = snapshot
            .players
            .iter()
            .find(|player| player.id == alice.0)
            .expect("perspective player should exist in snapshot");
        let hand_ids: Vec<u64> = me.hand_cards.iter().map(|card| card.id).collect();
        assert_eq!(
            hand_ids,
            vec![forest_b],
            "after the first land move, the snapshot hand should only show the remaining land"
        );
        let forest_count = me
            .battlefield
            .iter()
            .filter(|permanent| permanent.name == "Forest")
            .map(|permanent| permanent.count)
            .sum::<usize>();
        assert_eq!(
            forest_count, 1,
            "after the first land move, the snapshot battlefield should already show one Forest"
        );
        match snapshot
            .decision
            .as_ref()
            .expect("snapshot should include the repeated may decision")
        {
            super::DecisionView::SelectOptions { options, .. } => {
                let option_text: Vec<&str> = options
                    .iter()
                    .map(|option| option.description.as_str())
                    .collect();
                assert_eq!(option_text, vec!["Yes", "No"]);
            }
            other => panic!("expected repeat yes/no snapshot, got {other:?}"),
        }
        assert!(
            snapshot.resolving_stack_object.is_some(),
            "the resolving Cultivator trigger should stay visible across repeat iterations"
        );

        wasm.dispatch(
            serde_wasm_bindgen::to_value(&json!({
                "type": "select_options",
                "option_indices": [1],
            }))
            .expect("second yes choice should serialize"),
        )
        .expect("accepting the second Cultivator iteration should succeed");

        let pending_cast_stack_id = wasm
            .priority_state
            .pending_cast
            .as_ref()
            .map(|p| p.stack_id);
        let snapshot = GameSnapshot::from_game(
            &wasm.game,
            wasm.perspective,
            wasm.pending_decision.as_ref(),
            None,
            wasm.game_over.as_ref(),
            pending_cast_stack_id,
            wasm.active_resolving_stack_object.clone(),
            Vec::new(),
            wasm.active_viewed_cards.as_ref(),
            wasm.is_cancelable(),
            None,
            2,
        );
        let me = snapshot
            .players
            .iter()
            .find(|player| player.id == alice.0)
            .expect("perspective player should exist in snapshot");
        let hand_ids: Vec<u64> = me.hand_cards.iter().map(|card| card.id).collect();
        assert_eq!(
            hand_ids,
            vec![forest_b],
            "before the second land is chosen, the snapshot hand should still show only the remaining land"
        );
        let second_choice = match snapshot
            .decision
            .as_ref()
            .expect("snapshot should include second land-choice decision")
        {
            super::DecisionView::SelectObjects { candidates, .. } => candidates,
            other => panic!("expected second select_objects snapshot, got {other:?}"),
        };
        let second_candidates: Vec<u64> = second_choice
            .iter()
            .filter(|candidate| candidate.legal)
            .map(|candidate| candidate.id)
            .collect();
        assert_eq!(
            second_candidates,
            vec![forest_b],
            "second land-choice snapshot should only offer the remaining land"
        );
    }

    #[test]
    fn pregame_mulligan_prompt_offers_keep_and_mulligan() {
        let mut wasm = setup_pregame_match(MatchFormatInput::Normal);
        start_pregame(&mut wasm, 7, MatchFormatInput::Normal);

        let actions = match wasm.pending_decision.as_ref() {
            Some(DecisionContext::Priority(ctx)) => &ctx.actions,
            other => panic!("expected pregame priority decision, got {other:?}"),
        };
        assert!(
            actions
                .iter()
                .any(|action| matches!(action, LegalAction::KeepOpeningHand))
        );
        assert!(
            actions
                .iter()
                .any(|action| matches!(action, LegalAction::TakeMulligan))
        );
    }

    #[test]
    fn commander_first_mulligan_is_free() {
        let mut wasm = setup_pregame_match(MatchFormatInput::Commander);
        let alice = PlayerId::from_index(0);

        seed_filler_cards(&mut wasm, alice, Zone::Hand, 7);
        seed_filler_cards(&mut wasm, alice, Zone::Library, 7);
        start_pregame(&mut wasm, 7, MatchFormatInput::Commander);

        dispatch_matching_priority_action(&mut wasm, |action| {
            matches!(action, LegalAction::TakeMulligan)
        });
        dispatch_matching_priority_action(&mut wasm, |action| {
            matches!(action, LegalAction::KeepOpeningHand)
        });
        dispatch_matching_priority_action(&mut wasm, |action| {
            matches!(action, LegalAction::KeepOpeningHand)
        });

        match wasm.pending_decision.as_ref() {
            Some(DecisionContext::Priority(ctx)) => {
                assert_eq!(ctx.player, alice, "pregame should move to opening actions");
                assert!(
                    ctx.actions
                        .iter()
                        .any(|action| matches!(action, LegalAction::ContinuePregame))
                );
            }
            other => panic!("expected opening-actions priority prompt, got {other:?}"),
        }
    }

    #[test]
    fn commander_second_mulligan_bottoms_one_card() {
        let mut wasm = setup_pregame_match(MatchFormatInput::Commander);
        let alice = PlayerId::from_index(0);

        seed_filler_cards(&mut wasm, alice, Zone::Hand, 7);
        seed_filler_cards(&mut wasm, alice, Zone::Library, 7);
        start_pregame(&mut wasm, 7, MatchFormatInput::Commander);

        dispatch_matching_priority_action(&mut wasm, |action| {
            matches!(action, LegalAction::TakeMulligan)
        });
        dispatch_matching_priority_action(&mut wasm, |action| {
            matches!(action, LegalAction::KeepOpeningHand)
        });
        dispatch_matching_priority_action(&mut wasm, |action| {
            matches!(action, LegalAction::TakeMulligan)
        });
        dispatch_matching_priority_action(&mut wasm, |action| {
            matches!(action, LegalAction::KeepOpeningHand)
        });

        match wasm.pending_decision.as_ref() {
            Some(DecisionContext::SelectObjects(ctx)) => {
                assert_eq!(ctx.player, alice);
                assert_eq!(ctx.min, 1);
                assert_eq!(ctx.max, Some(1));
            }
            other => panic!("expected one-card bottoming prompt, got {other:?}"),
        }
    }

    #[test]
    fn serum_powder_redraws_without_counting_as_a_mulligan() {
        let mut wasm = setup_pregame_match(MatchFormatInput::Normal);
        let alice = PlayerId::from_index(0);

        let serum_id = wasm
            .game
            .create_object_from_definition(&serum_powder(), alice, Zone::Hand);
        seed_filler_cards(&mut wasm, alice, Zone::Hand, 6);
        seed_filler_cards(&mut wasm, alice, Zone::Library, 7);
        start_pregame(&mut wasm, 7, MatchFormatInput::Normal);

        dispatch_matching_priority_action(&mut wasm, |action| {
            matches!(
                action,
                LegalAction::SerumPowderMulligan { card_id } if *card_id == serum_id
            )
        });

        assert_eq!(
            wasm.game
                .player(alice)
                .expect("alice should exist")
                .hand
                .len(),
            7,
            "Serum Powder should redraw the same hand size"
        );
        assert_eq!(
            wasm.game
                .player(alice)
                .expect("alice should exist")
                .exile
                .len(),
            7,
            "Serum Powder should exile the original opening hand"
        );
        assert_eq!(
            wasm.pregame
                .as_ref()
                .and_then(|pregame| pregame.mulligans_taken.get(&alice).copied())
                .unwrap_or(0),
            0,
            "Serum Powder should not increment the mulligan count"
        );
        assert!(
            matches!(wasm.pending_decision, Some(DecisionContext::Priority(_))),
            "the same player should remain on the mulligan prompt"
        );
    }

    #[test]
    fn gemstone_caverns_appears_for_non_starting_player_in_opening_actions() {
        let mut wasm = setup_pregame_match(MatchFormatInput::Normal);
        let bob = PlayerId::from_index(1);

        seed_filler_cards(&mut wasm, PlayerId::from_index(0), Zone::Hand, 7);
        let gemstone_id =
            wasm.game
                .create_object_from_definition(&gemstone_caverns(), bob, Zone::Hand);
        seed_filler_cards(&mut wasm, bob, Zone::Hand, 1);
        start_pregame(&mut wasm, 7, MatchFormatInput::Normal);

        dispatch_matching_priority_action(&mut wasm, |action| {
            matches!(action, LegalAction::KeepOpeningHand)
        });
        dispatch_matching_priority_action(&mut wasm, |action| {
            matches!(action, LegalAction::KeepOpeningHand)
        });
        dispatch_matching_priority_action(&mut wasm, |action| {
            matches!(action, LegalAction::ContinuePregame)
        });

        match wasm.pending_decision.as_ref() {
            Some(DecisionContext::Priority(ctx)) => {
                assert_eq!(ctx.player, bob);
                assert!(ctx.actions.iter().any(|action| {
                    matches!(
                        action,
                        LegalAction::UsePregameAction { card_id, .. }
                            if *card_id == gemstone_id
                    )
                }));
                assert!(
                    ctx.actions
                        .iter()
                        .any(|action| matches!(action, LegalAction::BeginGame))
                );
            }
            other => panic!("expected Bob's opening-actions prompt, got {other:?}"),
        }
    }

    #[test]
    fn gemstone_caverns_moves_to_battlefield_and_prompts_for_exile() {
        let mut wasm = setup_pregame_match(MatchFormatInput::Normal);
        let bob = PlayerId::from_index(1);

        seed_filler_cards(&mut wasm, PlayerId::from_index(0), Zone::Hand, 7);
        let _gemstone_id =
            wasm.game
                .create_object_from_definition(&gemstone_caverns(), bob, Zone::Hand);
        let exile_card = seed_filler_cards(&mut wasm, bob, Zone::Hand, 1)
            .into_iter()
            .next()
            .expect("expected filler card in hand");
        start_pregame(&mut wasm, 7, MatchFormatInput::Normal);

        dispatch_matching_priority_action(&mut wasm, |action| {
            matches!(action, LegalAction::KeepOpeningHand)
        });
        dispatch_matching_priority_action(&mut wasm, |action| {
            matches!(action, LegalAction::KeepOpeningHand)
        });
        dispatch_matching_priority_action(&mut wasm, |action| {
            matches!(action, LegalAction::ContinuePregame)
        });
        dispatch_matching_priority_action(&mut wasm, |action| {
            matches!(action, LegalAction::UsePregameAction { .. })
        });

        let gemstone_on_battlefield = wasm.game.battlefield.iter().copied().find(|id| {
            wasm.game
                .object(*id)
                .is_some_and(|object| object.name == "Gemstone Caverns")
        });
        let gemstone_on_battlefield =
            gemstone_on_battlefield.expect("Gemstone Caverns should move to the battlefield");
        assert_eq!(
            wasm.game
                .object(gemstone_on_battlefield)
                .and_then(|object| object.counters.get(&CounterType::Luck).copied()),
            Some(1),
            "Gemstone Caverns should enter with a luck counter"
        );
        match wasm.pending_decision.as_ref() {
            Some(DecisionContext::SelectObjects(ctx)) => {
                assert_eq!(ctx.player, bob);
                assert_eq!(ctx.min, 1);
                assert_eq!(ctx.max, Some(1));
            }
            other => panic!("expected Gemstone exile prompt, got {other:?}"),
        }

        dispatch_select_objects(&mut wasm, &[exile_card.0]);

        assert!(
            wasm.game
                .player(bob)
                .expect("bob should exist")
                .exile
                .iter()
                .any(|id| wasm
                    .game
                    .object(*id)
                    .is_some_and(|object| object.name == "Ornithopter")),
            "the chosen card should be exiled"
        );
        match wasm.pending_decision.as_ref() {
            Some(DecisionContext::Priority(ctx)) => {
                assert_eq!(ctx.player, bob);
                assert!(
                    ctx.actions
                        .iter()
                        .any(|action| matches!(action, LegalAction::BeginGame))
                );
            }
            other => panic!("expected Bob to resume opening actions, got {other:?}"),
        }
    }
}
