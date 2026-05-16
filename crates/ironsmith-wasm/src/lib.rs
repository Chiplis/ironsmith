//! WASM-facing API for browser integration.
//!
//! This module provides a small wrapper around `GameState` so JavaScript can:
//! - create/reset a game
//! - mutate a bit of state
//! - read a serializable snapshot

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::OnceLock;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;

use rand::seq::SliceRandom;
use rand::{SeedableRng, rngs::StdRng};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

use ironsmith::cards::{CardDefinition, CardRegistry};
use ironsmith::color::{Color, ColorSet};
use ironsmith::combat_state::AttackTarget;
use ironsmith::decision::{
    AttackerDeclaration, BlockerDeclaration, DecisionMaker, GameProgress, GameResult, LegalAction,
};
use ironsmith::decisions::context::{
    DecisionContext, DecisionHiddenCardVisibility, ViewCardsContext,
};
use ironsmith::game_loop::{
    ActivationStage, CastStage, PendingPriorityContinuation, PriorityActionPerfMetrics,
    PriorityAdvancePerfMetrics, PriorityLoopState, PriorityResponse, advance_priority_with_dm,
    apply_decision_context_with_dm, apply_priority_response_with_dm, last_priority_action_perf,
    last_priority_advance_perf,
};
use ironsmith::game_state::{GameState, HiddenInfoOperation, StackEntry, Target};
use ironsmith::ids::{
    CardId, ObjectId, PlayerId, StableId, restore_id_counters, snapshot_id_counters,
};
use ironsmith::mana::{ManaCost, ManaSymbol};
use ironsmith::static_abilities::StaticAbilityId;
use ironsmith::targeting::{normalize_targets_for_requirements, validate_flat_target_assignment};
use ironsmith::triggers::TriggerQueue;
use ironsmith::types::CardType;
use ironsmith::zone::Zone;

mod ui_snapshot;
use ui_snapshot::{GameSnapshot, battlefield_transition_snapshots, build_object_details_snapshot};

#[cfg(test)]
static TEST_ID_COUNTER_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[cfg(test)]
pub(crate) fn test_id_counter_guard() -> std::sync::MutexGuard<'static, ()> {
    TEST_ID_COUNTER_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

struct PerfTimer {
    #[cfg(target_arch = "wasm32")]
    started_at_ms: f64,
    #[cfg(not(target_arch = "wasm32"))]
    started_at: Instant,
}

impl PerfTimer {
    fn start() -> Self {
        Self {
            #[cfg(target_arch = "wasm32")]
            started_at_ms: js_sys::Date::now(),
            #[cfg(not(target_arch = "wasm32"))]
            started_at: Instant::now(),
        }
    }

    fn elapsed_ms(&self) -> f64 {
        #[cfg(target_arch = "wasm32")]
        {
            js_sys::Date::now() - self.started_at_ms
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            self.started_at.elapsed().as_secs_f64() * 1000.0
        }
    }
}

const DETERMINISTIC_MATCH_SEED_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const DETERMINISTIC_MATCH_SEED_PRIME: u64 = 0x0000_0100_0000_01b3;

fn mix_match_seed_bytes(seed: &mut u64, bytes: &[u8]) {
    for &byte in bytes {
        *seed ^= byte as u64;
        *seed = seed.wrapping_mul(DETERMINISTIC_MATCH_SEED_PRIME);
    }
    *seed ^= 0xff;
    *seed = seed.wrapping_mul(DETERMINISTIC_MATCH_SEED_PRIME);
}

fn mix_match_seed_str(seed: &mut u64, value: &str) {
    mix_match_seed_bytes(seed, value.as_bytes());
}

fn mix_match_seed_u64(seed: &mut u64, value: u64) {
    mix_match_seed_bytes(seed, &value.to_le_bytes());
}

fn deterministic_match_seed(
    player_names: &[String],
    starting_life: i32,
    format: MatchFormatInput,
    decks: Option<&[Vec<String>]>,
    commanders: Option<&[Vec<String>]>,
    opening_hand_size: usize,
) -> u64 {
    let mut seed = DETERMINISTIC_MATCH_SEED_OFFSET;
    mix_match_seed_str(&mut seed, "ironsmith-match-seed-v1");
    mix_match_seed_str(
        &mut seed,
        match format {
            MatchFormatInput::Normal => "normal",
            MatchFormatInput::Commander => "commander",
        },
    );
    mix_match_seed_u64(&mut seed, starting_life as i64 as u64);
    mix_match_seed_u64(&mut seed, opening_hand_size as u64);
    mix_match_seed_u64(&mut seed, player_names.len() as u64);
    for name in player_names {
        mix_match_seed_str(&mut seed, name);
    }

    if let Some(decks) = decks {
        mix_match_seed_u64(&mut seed, decks.len() as u64);
        for deck in decks {
            mix_match_seed_u64(&mut seed, deck.len() as u64);
            for card_name in deck {
                mix_match_seed_str(&mut seed, card_name);
            }
        }
    }

    if let Some(commanders) = commanders {
        mix_match_seed_u64(&mut seed, commanders.len() as u64);
        for commander_list in commanders {
            mix_match_seed_u64(&mut seed, commander_list.len() as u64);
            for commander_name in commander_list {
                mix_match_seed_str(&mut seed, commander_name);
            }
        }
    }

    if seed == 0 {
        0x9e37_79b9_7f4a_7c15
    } else {
        seed
    }
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
    entry: &ironsmith::game_state::StackEntry,
) -> StackObjectSnapshot {
    let obj = game.object(entry.object_id);
    let source_obj = entry
        .source_stable_id
        .and_then(|stable_id| game.find_object_by_stable_id(stable_id))
        .and_then(|id| game.object(id));
    let id = if entry.is_ability {
        let provenance_id = entry.provenance.raw();
        if provenance_id != 0 {
            provenance_id.saturating_mul(2).saturating_add(1)
        } else {
            entry.object_id.0.saturating_mul(2).saturating_add(1)
        }
    } else {
        entry.object_id.0.saturating_mul(2)
    };
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
        .map(|(index, target)| {
            target_choice_view(game, perspective, viewed_cards, None, index, target)
        })
        .collect();

    if entry.is_ability {
        let ability_kind = if entry.triggering_event.is_some() {
            "Triggered"
        } else {
            "Activated"
        };
        let ability_text = stack_entry_ability_text(entry, obj);
        StackObjectSnapshot {
            id,
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
            let lines: Vec<_> = o
                .compiled_card_text
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty())
                .collect();
            if lines.is_empty() {
                None
            } else {
                Some(lines.join("; "))
            }
        } else {
            None
        };
        StackObjectSnapshot {
            id,
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

fn pending_stack_preview_id(index: usize) -> u64 {
    JS_SAFE_INTEGER_MAX
        .saturating_sub(100_000)
        .saturating_sub(index as u64)
}

fn insert_pending_stack_object_snapshots(
    snapshot: &mut GameSnapshot,
    stack_objects: Vec<StackObjectSnapshot>,
) {
    if stack_objects.is_empty() {
        return;
    }

    let preview_names =
        stack_objects
            .iter()
            .map(|stack_object| match stack_object.ability_kind.as_deref() {
                Some(kind) => format!("{} ({kind})", stack_object.name),
                None => stack_object.name.clone(),
            });

    snapshot.stack_preview.splice(0..0, preview_names);
    let count = stack_objects.len();
    snapshot.stack_objects.splice(0..0, stack_objects);
    snapshot.stack_size += count;
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
struct SnapshotPerfMetrics {
    snapshot_id: u64,
    battlefield_transition_ms: f64,
    snapshot_build_ms: f64,
    pending_stack_insert_ms: f64,
    snapshot_encode_ms: f64,
    total_snapshot_ms: f64,
    player_count: usize,
    battlefield_size: usize,
    stack_size: usize,
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
struct ReplayExecutionPerfMetrics {
    root_kind: String,
    restore_checkpoint_ms: f64,
    root_execution_ms: f64,
    decision_maker_finish_ms: f64,
    total_ms: f64,
    outcome_kind: String,
    progress_kind: Option<String>,
    priority_action: Option<PriorityActionPerfMetrics>,
    priority_advance: Option<PriorityAdvancePerfMetrics>,
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
struct AdvanceUntilDecisionPerfMetrics {
    total_ms: f64,
    iterations: usize,
    pregame_normalize_ms: f64,
    pregame_decision_build_ms: f64,
    runner_advance_ms: f64,
    auto_cleanup_discard_ms: f64,
    replay_advance_ms: f64,
    final_outcome: String,
    replay_execution: Option<ReplayExecutionPerfMetrics>,
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
struct DispatchPerfMetrics {
    command_kind: String,
    pending_decision_kind: String,
    route_kind: String,
    command_decode_ms: f64,
    command_to_response_ms: f64,
    checkpoint_capture_ms: f64,
    execute_with_replay_ms: f64,
    apply_progress_ms: f64,
    total_dispatch_ms: f64,
    outcome_kind: String,
    replay_execution: Option<ReplayExecutionPerfMetrics>,
    advance_until_decision: Option<AdvanceUntilDecisionPerfMetrics>,
    snapshot: Option<SnapshotPerfMetrics>,
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
    card_stable_ids: Vec<StableId>,
    public: bool,
    source: Option<ObjectId>,
    description: String,
}

fn stable_id_for_viewed_card(game: &GameState, card: ObjectId) -> StableId {
    game.object(card)
        .map(|object| object.stable_id)
        .unwrap_or_else(|| StableId::from_raw(card.0))
}

fn stable_ids_for_viewed_cards(game: &GameState, cards: &[ObjectId]) -> Vec<StableId> {
    cards
        .iter()
        .map(|card| stable_id_for_viewed_card(game, *card))
        .collect()
}

impl ActiveViewedCards {
    fn push_unique_card(&mut self, game: &GameState, card: ObjectId) {
        self.push_unique_card_with_stable_id(card, stable_id_for_viewed_card(game, card));
    }

    fn push_unique_card_with_stable_id(&mut self, card: ObjectId, stable_id: StableId) {
        if let Some(index) = self
            .card_stable_ids
            .iter()
            .position(|existing| *existing == stable_id)
        {
            self.cards[index] = card;
            return;
        }
        if let Some(index) = self.cards.iter().position(|existing| *existing == card) {
            if let Some(existing_stable_id) = self.card_stable_ids.get_mut(index) {
                *existing_stable_id = stable_id;
            } else {
                self.card_stable_ids.push(stable_id);
            }
            return;
        }
        if !self.cards.contains(&card) {
            self.cards.push(card);
            self.card_stable_ids.push(stable_id);
        }
    }

    fn stable_id_at(&self, index: usize, card: ObjectId) -> StableId {
        self.card_stable_ids
            .get(index)
            .copied()
            .unwrap_or_else(|| StableId::from_raw(card.0))
    }

    fn resolved_object_id(&self, game: &GameState, index: usize, card: ObjectId) -> ObjectId {
        if game.object(card).is_some() {
            return card;
        }
        game.find_object_by_stable_id(self.stable_id_at(index, card))
            .unwrap_or(card)
    }

    fn contains_object(&self, game: &GameState, card: ObjectId) -> bool {
        if self.cards.contains(&card) {
            return true;
        }
        let Some(object) = game.object(card) else {
            return false;
        };
        self.card_stable_ids.contains(&object.stable_id)
    }
}

fn active_viewed_cards_can_carry_over(
    existing: &ActiveViewedCards,
    next: &ActiveViewedCards,
) -> bool {
    existing.public
        && next.public
        && existing.zone == next.zone
        && existing.subject == next.subject
        && existing.source == next.source
}

fn merge_carried_active_viewed_cards(
    carry: Option<ActiveViewedCards>,
    next: Option<ActiveViewedCards>,
) -> Option<ActiveViewedCards> {
    match (carry, next) {
        (Some(mut carry), Some(next)) if active_viewed_cards_can_carry_over(&carry, &next) => {
            for (index, card) in next.cards.iter().copied().enumerate() {
                carry.push_unique_card_with_stable_id(card, next.stable_id_at(index, card));
            }
            carry.viewer = next.viewer;
            carry.description = next.description;
            Some(carry)
        }
        (_, Some(next)) => Some(next),
        (Some(carry), None) => Some(carry),
        (None, None) => None,
    }
}

#[derive(Debug, Clone, Default)]
struct CryptoAuditState {
    hidden_by_id: HashMap<ObjectId, HiddenAuditCard>,
    hidden_by_key: HashMap<(u8, u16, String), HiddenAuditCard>,
    stable_by_id: HashMap<ObjectId, StableId>,
    id_by_stable: HashMap<StableId, ObjectId>,
    libraries: HashMap<PlayerId, Vec<ObjectId>>,
    hands: HashMap<PlayerId, Vec<ObjectId>>,
    random_count: u64,
    operation_checkpoint: usize,
}

#[derive(Debug, Clone)]
struct HiddenAuditCard {
    object_id: ObjectId,
    owner: PlayerId,
    zone: Zone,
    slot: u16,
    commitment: String,
    card: Option<String>,
    face_down: bool,
    foretold: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CryptoRequirementView {
    id: String,
    #[serde(rename = "type")]
    requirement_type: String,
    owner: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    viewer: Option<u8>,
    zone: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    slot: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    object_id: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    commitment: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    card: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    visibility: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    count: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    from: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    to: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    before_order: Option<Vec<u64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    after_order: Option<Vec<u64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    random_count_before: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    random_count_after: Option<u64>,
}

impl CryptoRequirementView {
    fn hidden_open(
        requirement_type: &str,
        card: &HiddenAuditCard,
        viewer: Option<PlayerId>,
        visibility: &str,
        reason: &str,
    ) -> Self {
        Self {
            id: format!(
                "{}:{}:{}:{}:{}",
                requirement_type,
                card.owner.index(),
                zone_crypto_kind(card.zone),
                card.slot,
                card.object_id.0
            ),
            requirement_type: requirement_type.to_string(),
            owner: card.owner.index() as u8,
            viewer: viewer.map(|viewer| viewer.index() as u8),
            zone: zone_crypto_kind(card.zone).to_string(),
            slot: Some(card.slot),
            object_id: Some(card.object_id.0),
            commitment: (!card.commitment.is_empty()).then(|| card.commitment.clone()),
            card: card.card.clone(),
            visibility: Some(visibility.to_string()),
            reason: Some(reason.to_string()),
            count: None,
            from: None,
            to: None,
            before_order: None,
            after_order: None,
            random_count_before: None,
            random_count_after: None,
        }
    }
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
    pending: &ironsmith::game_loop::PendingCast,
) -> Option<ManaPaymentView> {
    if !matches!(pending.stage, CastStage::PayingMana) {
        return None;
    }

    let pips = if !pending.display_mana_pips.is_empty() {
        pending.display_mana_pips.clone()
    } else if let Some(cost) = pending.mana_cost_to_pay.as_ref() {
        ironsmith::game_loop::expand_mana_cost_to_display_pips(
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
    pending: &ironsmith::game_loop::PendingActivation,
) -> Option<ManaPaymentView> {
    if !matches!(pending.stage, ActivationStage::PayingMana) {
        return None;
    }

    let pips = if !pending.display_mana_pips.is_empty() {
        pending.display_mana_pips.clone()
    } else if let Some(cost) = pending.mana_cost_to_pay.as_ref() {
        ironsmith::game_loop::expand_mana_cost_to_display_pips(cost, pending.x_value.unwrap_or(0))
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
    game: &GameState,
    current: &mut Option<ActiveViewedCards>,
    viewer: PlayerId,
    cards: &[ObjectId],
    ctx: &ironsmith::decisions::context::ViewCardsContext,
) {
    let can_merge = current
        .as_ref()
        .is_some_and(|existing| viewed_cards_can_merge(existing, viewer, ctx));

    if can_merge {
        if let Some(existing) = current.as_mut() {
            for &card in cards {
                existing.push_unique_card(game, card);
            }
        }
        return;
    }

    *current = Some(ActiveViewedCards {
        viewer,
        subject: ctx.subject,
        zone: ctx.zone,
        cards: cards.to_vec(),
        card_stable_ids: stable_ids_for_viewed_cards(game, cards),
        public: ctx.public,
        source: ctx.source,
        description: ctx.description.clone(),
    });
}

fn viewed_cards_can_merge(
    existing: &ActiveViewedCards,
    viewer: PlayerId,
    ctx: &ironsmith::decisions::context::ViewCardsContext,
) -> bool {
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
}

fn audit_viewed_cards_can_merge(
    existing: &ActiveViewedCards,
    viewer: PlayerId,
    ctx: &ironsmith::decisions::context::ViewCardsContext,
) -> bool {
    existing.public == ctx.public
        && existing.zone == ctx.zone
        && existing.source == ctx.source
        && existing.description == ctx.description
        && existing.subject == ctx.subject
        && if ctx.public {
            true
        } else {
            existing.viewer == viewer
        }
}

fn merge_audit_viewed_cards(
    game: &GameState,
    current: &mut Vec<ActiveViewedCards>,
    viewer: PlayerId,
    cards: &[ObjectId],
    ctx: &ironsmith::decisions::context::ViewCardsContext,
) {
    if let Some(existing) = current
        .iter_mut()
        .find(|existing| audit_viewed_cards_can_merge(existing, viewer, ctx))
    {
        for &card in cards {
            existing.push_unique_card(game, card);
        }
        return;
    }

    current.push(ActiveViewedCards {
        viewer,
        subject: ctx.subject,
        zone: ctx.zone,
        cards: cards.to_vec(),
        card_stable_ids: stable_ids_for_viewed_cards(game, cards),
        public: ctx.public,
        source: ctx.source,
        description: ctx.description.clone(),
    });
}

fn merge_hidden_decision_views(
    game: &GameState,
    current: &mut Option<ActiveViewedCards>,
    audit: &mut Vec<ActiveViewedCards>,
    ctx: &DecisionContext,
) {
    for view in ctx.hidden_card_views() {
        if view.visibility == DecisionHiddenCardVisibility::None || view.object_ids.is_empty() {
            continue;
        }

        let mut grouped: HashMap<(PlayerId, Zone), Vec<ObjectId>> = HashMap::new();
        for &id in &view.object_ids {
            let Some(object) = game.object(id) else {
                continue;
            };
            if !object.zone.is_hidden() && game.hidden_card_info(id).is_none() {
                continue;
            }
            grouped
                .entry((object.owner, object.zone))
                .or_default()
                .push(id);
        }

        for ((subject, zone), cards) in grouped {
            if cards.is_empty() {
                continue;
            }
            match view.visibility {
                DecisionHiddenCardVisibility::PrivateToDecisionPlayer => {
                    let viewer = game.controlling_player_for(ctx.player());
                    let view_ctx = ViewCardsContext::new(
                        viewer,
                        subject,
                        ctx.source(),
                        zone,
                        view.description.clone(),
                    );
                    merge_active_viewed_cards(game, current, viewer, &cards, &view_ctx);
                    merge_audit_viewed_cards(game, audit, viewer, &cards, &view_ctx);
                }
                DecisionHiddenCardVisibility::Public => {
                    for viewer_idx in 0..game.players.len() {
                        let viewer = PlayerId::from_index(viewer_idx as u8);
                        let view_ctx = ViewCardsContext::new(
                            viewer,
                            subject,
                            ctx.source(),
                            zone,
                            view.description.clone(),
                        )
                        .with_public(true);
                        merge_active_viewed_cards(game, current, viewer, &cards, &view_ctx);
                        merge_audit_viewed_cards(game, audit, viewer, &cards, &view_ctx);
                    }
                }
                DecisionHiddenCardVisibility::None => {}
            }
        }
    }
}

fn stack_revealed_view(game: &GameState) -> Option<ActiveViewedCards> {
    for entry in game.stack.iter().rev() {
        if let Some(source_snapshot) = entry
            .source_snapshot
            .as_ref()
            .filter(|snapshot| snapshot.zone.is_hidden())
        {
            return Some(ActiveViewedCards {
                viewer: entry.controller,
                subject: source_snapshot.owner,
                zone: source_snapshot.zone,
                cards: vec![source_snapshot.object_id],
                card_stable_ids: vec![source_snapshot.stable_id],
                public: true,
                source: Some(entry.object_id),
                description: "Revealed while on the stack".to_string(),
            });
        }

        let Some(revealed) = entry.tagged_objects.get(&ironsmith::tag::TagKey::from(
            ironsmith::effects::PUBLIC_REVEALED_TAG,
        )) else {
            continue;
        };
        let hidden: Vec<_> = revealed
            .iter()
            .filter(|snapshot| snapshot.zone.is_hidden())
            .cloned()
            .collect();
        let Some(first) = hidden.first() else {
            continue;
        };
        let first_owner = first.owner;
        let first_zone = first.zone;

        let mut cards = Vec::new();
        let mut card_stable_ids = Vec::new();
        for snapshot in hidden {
            if snapshot.owner == first_owner
                && snapshot.zone == first_zone
                && !cards.contains(&snapshot.object_id)
            {
                cards.push(snapshot.object_id);
                card_stable_ids.push(snapshot.stable_id);
            }
        }

        if !cards.is_empty() {
            return Some(ActiveViewedCards {
                viewer: entry.controller,
                subject: first_owner,
                zone: first_zone,
                cards,
                card_stable_ids,
                public: true,
                source: Some(entry.object_id),
                description: "Revealed while on the stack".to_string(),
            });
        }
    }

    None
}

fn zone_crypto_kind(zone: Zone) -> &'static str {
    match zone {
        Zone::Library => "library",
        Zone::Hand => "hand",
        Zone::Exile => "face_down_exile",
        Zone::Battlefield => "face_down_permanent",
        _ => "hidden_zone",
    }
}

fn battlefield_has_static_ability(game: &GameState, ability_id: StaticAbilityId) -> bool {
    game.object_store.battlefield.iter().any(|id| {
        game.object(*id)
            .is_some_and(|_| game.object_has_static_ability_id(*id, ability_id))
    })
}

fn can_view_own_library_top(game: &GameState, player: PlayerId) -> bool {
    game.object_store.battlefield.iter().any(|id| {
        game.object(*id).is_some_and(|object| {
            game.current_controller(*id).unwrap_or(object.owner) == player
                && game.object_has_static_ability_id(*id, StaticAbilityId::LookAtTopCardOfLibrary)
        })
    })
}

fn library_top_revealed_by_static_ability(game: &GameState, player: PlayerId) -> bool {
    battlefield_has_static_ability(game, StaticAbilityId::AllPlayersLookAtTopCardsOfLibraries)
        || game.object_store.battlefield.iter().any(|id| {
            game.object(*id).is_some_and(|object| {
                game.current_controller(*id).unwrap_or(object.owner) == player
                    && game.object_has_static_ability_id(
                        *id,
                        StaticAbilityId::AllPlayersLookAtYourTopLibraryCard,
                    )
            })
        })
}

fn hand_revealed_by_static_ability(game: &GameState, player: PlayerId) -> bool {
    game.object_store.battlefield.iter().any(|id| {
        game.object(*id).is_some_and(|object| {
            game.current_controller(*id).unwrap_or(object.owner) != player
                && game.object_has_static_ability_id(
                    *id,
                    StaticAbilityId::OpponentsPlayWithHandsRevealed,
                )
        })
    })
}

fn append_static_visibility_views(game: &GameState, views: &mut Vec<ActiveViewedCards>) {
    let public_viewer = PlayerId::from_index(0);
    for player in &game.players {
        if let Some(&top_card) = player.library.last() {
            if library_top_revealed_by_static_ability(game, player.id) {
                views.push(ActiveViewedCards {
                    viewer: public_viewer,
                    subject: player.id,
                    zone: Zone::Library,
                    cards: vec![top_card],
                    card_stable_ids: stable_ids_for_viewed_cards(game, &[top_card]),
                    public: true,
                    source: None,
                    description: "Static ability reveals the top card of a library".to_string(),
                });
            } else if can_view_own_library_top(game, player.id) {
                views.push(ActiveViewedCards {
                    viewer: player.id,
                    subject: player.id,
                    zone: Zone::Library,
                    cards: vec![top_card],
                    card_stable_ids: stable_ids_for_viewed_cards(game, &[top_card]),
                    public: false,
                    source: None,
                    description: "Static ability allows viewing the top card of a library"
                        .to_string(),
                });
            }
        }

        if hand_revealed_by_static_ability(game, player.id) && !player.hand.is_empty() {
            views.push(ActiveViewedCards {
                viewer: public_viewer,
                subject: player.id,
                zone: Zone::Hand,
                cards: player.hand.clone(),
                card_stable_ids: stable_ids_for_viewed_cards(game, &player.hand),
                public: true,
                source: None,
                description: "Static ability reveals a player's hand".to_string(),
            });
        }
    }
}

fn hidden_audit_key(card: &HiddenAuditCard) -> (u8, u16, String) {
    (card.owner.index() as u8, card.slot, card.commitment.clone())
}

fn push_requirement_unique(
    requirements: &mut Vec<CryptoRequirementView>,
    seen: &mut HashSet<String>,
    requirement: CryptoRequirementView,
) {
    if seen.insert(requirement.id.clone()) {
        requirements.push(requirement);
    }
}

fn push_hidden_move_requirements(
    requirements: &mut Vec<CryptoRequirementView>,
    seen: &mut HashSet<String>,
    before_card: &HiddenAuditCard,
    after_card: &HiddenAuditCard,
    reason: &str,
) {
    let moved = CryptoRequirementView {
        id: format!(
            "hidden_move:{}:{}:{}:{}:{}",
            before_card.owner.index(),
            zone_crypto_kind(before_card.zone),
            zone_crypto_kind(after_card.zone),
            before_card.slot,
            after_card.object_id.0
        ),
        requirement_type: "hidden_move".to_string(),
        owner: before_card.owner.index() as u8,
        viewer: None,
        zone: zone_crypto_kind(after_card.zone).to_string(),
        slot: Some(before_card.slot),
        object_id: Some(after_card.object_id.0),
        commitment: (!after_card.commitment.is_empty()).then(|| after_card.commitment.clone()),
        card: after_card.card.clone(),
        visibility: None,
        reason: Some(reason.to_string()),
        count: None,
        from: Some(zone_crypto_kind(before_card.zone).to_string()),
        to: Some(zone_crypto_kind(after_card.zone).to_string()),
        before_order: None,
        after_order: None,
        random_count_before: None,
        random_count_after: None,
    };
    push_requirement_unique(requirements, seen, moved);

    if before_card.zone == Zone::Library && after_card.zone == Zone::Hand {
        push_requirement_unique(
            requirements,
            seen,
            CryptoRequirementView::hidden_open(
                "private_open",
                after_card,
                Some(after_card.owner),
                "owner_only",
                "hidden library card moved to hand",
            ),
        );
    }

    if after_card.card.is_some()
        && !matches!(after_card.zone, Zone::Library | Zone::Hand)
        && !after_card.face_down
        && !after_card.foretold
    {
        push_requirement_unique(
            requirements,
            seen,
            CryptoRequirementView::hidden_open(
                "public_open",
                after_card,
                None,
                "public",
                "hidden card moved to a public zone",
            ),
        );
    }
}

fn push_hidden_order_update_requirement(
    requirements: &mut Vec<CryptoRequirementView>,
    seen: &mut HashSet<String>,
    player: PlayerId,
    before_order: &[ObjectId],
    after_order: &[ObjectId],
    reason: &str,
) {
    if before_order == after_order || before_order.len() != after_order.len() {
        return;
    }
    let before_ids: Vec<u64> = before_order.iter().map(|id| id.0).collect();
    let after_ids: Vec<u64> = after_order.iter().map(|id| id.0).collect();
    let id = format!(
        "hidden_order_update:{}:library:{}:{}",
        player.index(),
        before_ids
            .iter()
            .map(|id| id.to_string())
            .collect::<Vec<_>>()
            .join("-"),
        after_ids
            .iter()
            .map(|id| id.to_string())
            .collect::<Vec<_>>()
            .join("-")
    );
    push_requirement_unique(
        requirements,
        seen,
        CryptoRequirementView {
            id,
            requirement_type: "hidden_order_update".to_string(),
            owner: player.index() as u8,
            viewer: None,
            zone: "library".to_string(),
            slot: None,
            object_id: None,
            commitment: None,
            card: None,
            visibility: None,
            reason: Some(reason.to_string()),
            count: Some(after_order.len().min(u16::MAX as usize) as u16),
            from: None,
            to: None,
            before_order: Some(before_ids),
            after_order: Some(after_ids),
            random_count_before: None,
            random_count_after: None,
        },
    );
}

fn library_relative_order_changed(before: &[ObjectId], after: &[ObjectId]) -> bool {
    let after_set: HashSet<_> = after.iter().copied().collect();
    let before_common: Vec<_> = before
        .iter()
        .copied()
        .filter(|id| after_set.contains(id))
        .collect();
    let before_set: HashSet<_> = before.iter().copied().collect();
    let after_common: Vec<_> = after
        .iter()
        .copied()
        .filter(|id| before_set.contains(id))
        .collect();
    before_common.len() > 1 && before_common != after_common
}

fn hidden_keys_for_order(
    state: &CryptoAuditState,
    order: &[ObjectId],
) -> HashSet<(u8, u16, String)> {
    order
        .iter()
        .filter_map(|id| state.hidden_by_id.get(id).map(hidden_audit_key))
        .collect()
}

fn changed_hidden_hand_cards_after_shuffle(
    player: PlayerId,
    before: &CryptoAuditState,
    after: &CryptoAuditState,
) -> Vec<ObjectId> {
    let mut drawn = Vec::new();
    let Some(after_hand) = after.hands.get(&player) else {
        return drawn;
    };
    for &object_id in after_hand {
        let Some(after_card) = after.hidden_by_id.get(&object_id) else {
            continue;
        };
        if after_card.owner != player || after_card.zone != Zone::Hand {
            continue;
        }
        if before
            .hidden_by_id
            .get(&object_id)
            .is_some_and(|card| card.owner == player && card.zone == Zone::Hand)
        {
            continue;
        }
        let Some(before_card) = before.hidden_by_key.get(&hidden_audit_key(after_card)) else {
            continue;
        };
        if before_card.owner == player
            && (before_card.zone == Zone::Library || before_card.object_id != after_card.object_id)
        {
            drawn.push(object_id);
        }
    }
    drawn.reverse();
    drawn
}

fn effective_after_shuffle_order(
    player: PlayerId,
    before: &CryptoAuditState,
    after: &CryptoAuditState,
) -> Vec<ObjectId> {
    let mut order = after.libraries.get(&player).cloned().unwrap_or_default();
    order.extend(changed_hidden_hand_cards_after_shuffle(
        player, before, after,
    ));
    order
}

fn object_order_has_unique_ids(order: &[ObjectId]) -> bool {
    let mut seen = HashSet::new();
    order.iter().copied().all(|id| seen.insert(id))
}

fn remap_journaled_after_shuffle_order(
    before: &CryptoAuditState,
    after: &CryptoAuditState,
    after_order: &[ObjectId],
) -> Vec<ObjectId> {
    after_order
        .iter()
        .copied()
        .map(|id| {
            let key = after
                .hidden_by_id
                .get(&id)
                .or_else(|| before.hidden_by_id.get(&id))
                .map(hidden_audit_key);
            key.and_then(|key| after.hidden_by_key.get(&key).map(|card| card.object_id))
                .or_else(|| {
                    before
                        .stable_by_id
                        .get(&id)
                        .and_then(|stable_id| after.id_by_stable.get(stable_id).copied())
                })
                .unwrap_or(id)
        })
        .collect()
}

fn normalized_after_shuffle_order(
    player: PlayerId,
    before: &CryptoAuditState,
    after: &CryptoAuditState,
    after_order: &[ObjectId],
) -> Vec<ObjectId> {
    let remapped = remap_journaled_after_shuffle_order(before, after, after_order);
    let effective = effective_after_shuffle_order(player, before, after);
    if remapped.is_empty() || !object_order_has_unique_ids(&remapped) {
        return effective;
    }
    remapped
}

fn effective_before_shuffle_order(
    player: PlayerId,
    before: &CryptoAuditState,
    after: &CryptoAuditState,
    after_order: &[ObjectId],
) -> Vec<ObjectId> {
    let after_keys = hidden_keys_for_order(after, after_order);
    let mut order = Vec::new();
    let mut seen = HashSet::new();
    let mut append_matching = |ids: Option<&Vec<ObjectId>>, order: &mut Vec<ObjectId>| {
        let Some(ids) = ids else {
            return;
        };
        for &id in ids {
            let Some(card) = before.hidden_by_id.get(&id) else {
                continue;
            };
            if card.owner == player
                && after_keys.contains(&hidden_audit_key(card))
                && seen.insert(id)
            {
                order.push(id);
            }
        }
    };
    append_matching(before.libraries.get(&player), &mut order);
    append_matching(before.hands.get(&player), &mut order);
    let mut remaining: Vec<_> = before
        .hidden_by_id
        .iter()
        .filter_map(|(&id, card)| {
            (card.owner == player && after_keys.contains(&hidden_audit_key(card))).then_some(id)
        })
        .collect();
    remaining.sort_by_key(|id| id.0);
    append_matching(Some(&remaining), &mut order);
    order
}

impl WasmGame {
    fn capture_crypto_audit_state(&self) -> CryptoAuditState {
        let mut state = CryptoAuditState {
            random_count: self.game.irreversible_random_count(),
            operation_checkpoint: self.game.crypto_audit_checkpoint(),
            ..CryptoAuditState::default()
        };
        for (&object_id, info) in &self.game.hidden_cards {
            let Some(object) = self.game.object(object_id) else {
                continue;
            };
            let card = HiddenAuditCard {
                object_id,
                owner: info.owner,
                zone: object.zone,
                slot: info.slot,
                commitment: info.commitment.clone(),
                card: object.card.as_ref().map(|_| object.name.clone()),
                face_down: self.game.is_face_down(object_id),
                foretold: self.game.is_foretold(object_id),
            };
            state
                .hidden_by_key
                .insert(hidden_audit_key(&card), card.clone());
            state.hidden_by_id.insert(object_id, card);
        }
        for player in &self.game.players {
            state.libraries.insert(player.id, player.library.clone());
            state.hands.insert(player.id, player.hand.clone());
            for &object_id in player.library.iter().chain(player.hand.iter()) {
                if let Some(object) = self.game.object(object_id) {
                    state.stable_by_id.insert(object_id, object.stable_id);
                    state.id_by_stable.insert(object.stable_id, object_id);
                }
            }
        }
        state
    }

    fn update_crypto_requirements_from(&mut self, before: CryptoAuditState) {
        let after = self.capture_crypto_audit_state();
        let mut requirements = Vec::new();
        let mut seen = HashSet::new();
        let operations = self
            .game
            .crypto_audit_operations_since(before.operation_checkpoint);
        let mut journaled_shuffle_players = HashSet::new();
        let mut journaled_random_delta = 0u64;

        for operation in operations {
            match operation {
                HiddenInfoOperation::HiddenMove {
                    owner,
                    old_object_id,
                    new_object_id,
                    slot,
                    commitment,
                    ..
                } => {
                    let key = (owner.index() as u8, slot, commitment.clone());
                    let before_card = before
                        .hidden_by_id
                        .get(&old_object_id)
                        .or_else(|| before.hidden_by_key.get(&key));
                    let after_card = after
                        .hidden_by_id
                        .get(&new_object_id)
                        .or_else(|| after.hidden_by_key.get(&key));
                    if let (Some(before_card), Some(after_card)) = (before_card, after_card) {
                        push_hidden_move_requirements(
                            &mut requirements,
                            &mut seen,
                            before_card,
                            after_card,
                            "hidden object moved between zones",
                        );
                    }
                }
                HiddenInfoOperation::LibraryShuffle {
                    player,
                    before_order,
                    after_order,
                    random_count_before,
                    random_count_after,
                } => {
                    journaled_shuffle_players.insert(player);
                    journaled_random_delta = journaled_random_delta
                        .saturating_add(random_count_after.saturating_sub(random_count_before));
                    let mut after_shuffle_order =
                        normalized_after_shuffle_order(player, &before, &after, &after_order);
                    if after_shuffle_order.is_empty() {
                        after_shuffle_order = after_order;
                    }
                    if !object_order_has_unique_ids(&after_shuffle_order) {
                        continue;
                    }
                    let mut before_shuffle_order = effective_before_shuffle_order(
                        player,
                        &before,
                        &after,
                        &after_shuffle_order,
                    );
                    if before_shuffle_order.len() != after_shuffle_order.len() {
                        before_shuffle_order = before_order;
                    }
                    if before_shuffle_order.len() <= 1
                        || before_shuffle_order.len() != after_shuffle_order.len()
                    {
                        continue;
                    }
                    let library_prefix_count = after
                        .libraries
                        .get(&player)
                        .map(|order| order.len())
                        .unwrap_or(after_shuffle_order.len())
                        .min(after_shuffle_order.len());
                    push_requirement_unique(
                        &mut requirements,
                        &mut seen,
                        CryptoRequirementView {
                            id: format!(
                                "verifiable_shuffle:{}:library:{}:{}",
                                player.index(),
                                random_count_before,
                                random_count_after
                            ),
                            requirement_type: "verifiable_shuffle".to_string(),
                            owner: player.index() as u8,
                            viewer: None,
                            zone: "library".to_string(),
                            slot: None,
                            object_id: None,
                            commitment: None,
                            card: None,
                            visibility: None,
                            reason: Some("library shuffled".to_string()),
                            count: Some(library_prefix_count.min(u16::MAX as usize) as u16),
                            from: None,
                            to: None,
                            before_order: Some(
                                before_shuffle_order.iter().map(|id| id.0).collect(),
                            ),
                            after_order: Some(after_shuffle_order.iter().map(|id| id.0).collect()),
                            random_count_before: Some(random_count_before),
                            random_count_after: Some(random_count_after),
                        },
                    );
                }
                HiddenInfoOperation::LibraryReorder {
                    player,
                    before_order,
                    after_order,
                    reason,
                } => {
                    push_hidden_order_update_requirement(
                        &mut requirements,
                        &mut seen,
                        player,
                        &before_order,
                        &after_order,
                        &reason,
                    );
                }
                HiddenInfoOperation::FairRandom {
                    random_count_before,
                    random_count_after,
                    reason,
                } => {
                    let delta = random_count_after.saturating_sub(random_count_before);
                    journaled_random_delta = journaled_random_delta.saturating_add(delta);
                    if delta == 0 {
                        continue;
                    }
                    let owner = self
                        .game
                        .turn
                        .priority_player
                        .unwrap_or(self.game.turn.active_player);
                    push_requirement_unique(
                        &mut requirements,
                        &mut seen,
                        CryptoRequirementView {
                            id: format!(
                                "fair_random:{}:{}:{}",
                                owner.index(),
                                random_count_before,
                                random_count_after
                            ),
                            requirement_type: "fair_random".to_string(),
                            owner: owner.index() as u8,
                            viewer: None,
                            zone: "game".to_string(),
                            slot: None,
                            object_id: None,
                            commitment: None,
                            card: None,
                            visibility: None,
                            reason: Some(reason),
                            count: Some(delta.min(u16::MAX as u64) as u16),
                            from: None,
                            to: None,
                            before_order: None,
                            after_order: None,
                            random_count_before: Some(random_count_before),
                            random_count_after: Some(random_count_after),
                        },
                    );
                }
            }
        }

        for before_card in before.hidden_by_id.values() {
            let after_card = after
                .hidden_by_id
                .get(&before_card.object_id)
                .or_else(|| after.hidden_by_key.get(&hidden_audit_key(before_card)));

            if let Some(after_card) = after_card {
                if before_card.zone != after_card.zone {
                    push_hidden_move_requirements(
                        &mut requirements,
                        &mut seen,
                        before_card,
                        after_card,
                        "hidden object moved between zones (snapshot fallback)",
                    );
                }
                continue;
            }

            if let Some(object) = self.game.object(before_card.object_id)
                && object.card.is_some()
            {
                let mut opened = before_card.clone();
                opened.card = Some(object.name.clone());
                let is_public = !object.zone.is_hidden();
                push_requirement_unique(
                    &mut requirements,
                    &mut seen,
                    CryptoRequirementView::hidden_open(
                        if is_public {
                            "public_open"
                        } else {
                            "private_open"
                        },
                        &opened,
                        (!is_public).then_some(opened.owner),
                        if is_public { "public" } else { "owner_only" },
                        "hidden card identity became known",
                    ),
                );
            }
        }

        let mut audit_views = self.active_audit_viewed_cards.clone();
        if audit_views.is_empty()
            && let Some(view) = self.active_viewed_cards.as_ref()
        {
            audit_views.push(view.clone());
        }
        append_static_visibility_views(&self.game, &mut audit_views);

        for view in audit_views {
            let count = view.cards.len().min(u16::MAX as usize) as u16;
            let view_requirement = CryptoRequirementView {
                id: format!(
                    "{}_view:{}:{}:{}:{}",
                    if view.public { "public" } else { "private" },
                    view.viewer.index(),
                    view.subject.index(),
                    zone_crypto_kind(view.zone),
                    count
                ),
                requirement_type: if view.public {
                    "public_view_window".to_string()
                } else {
                    "private_view_window".to_string()
                },
                owner: view.subject.index() as u8,
                viewer: Some(view.viewer.index() as u8),
                zone: zone_crypto_kind(view.zone).to_string(),
                slot: None,
                object_id: None,
                commitment: None,
                card: None,
                visibility: Some(if view.public { "public" } else { "viewer" }.to_string()),
                reason: Some(view.description.clone()),
                count: Some(count),
                from: None,
                to: None,
                before_order: None,
                after_order: None,
                random_count_before: None,
                random_count_after: None,
            };
            push_requirement_unique(&mut requirements, &mut seen, view_requirement);

            for (index, &object_id) in view.cards.iter().enumerate() {
                let resolved_object_id = view.resolved_object_id(&self.game, index, object_id);
                let Some(card) = before
                    .hidden_by_id
                    .get(&object_id)
                    .or_else(|| after.hidden_by_id.get(&object_id))
                    .or_else(|| before.hidden_by_id.get(&resolved_object_id))
                    .or_else(|| after.hidden_by_id.get(&resolved_object_id))
                else {
                    continue;
                };
                push_requirement_unique(
                    &mut requirements,
                    &mut seen,
                    CryptoRequirementView::hidden_open(
                        if view.public {
                            "public_open"
                        } else {
                            "private_open"
                        },
                        card,
                        (!view.public).then_some(view.viewer),
                        if view.public { "public" } else { "viewer" },
                        &view.description,
                    ),
                );
            }
        }

        let mut shuffle_requirements = 0u64;
        for (player, before_order) in &before.libraries {
            if journaled_shuffle_players.contains(player) {
                continue;
            }
            let Some(after_library_order) = after.libraries.get(player) else {
                continue;
            };
            if after.random_count <= before.random_count {
                continue;
            }
            if !library_relative_order_changed(before_order, after_library_order) {
                continue;
            }
            let after_shuffle_order = effective_after_shuffle_order(*player, &before, &after);
            let before_shuffle_order =
                effective_before_shuffle_order(*player, &before, &after, &after_shuffle_order);
            shuffle_requirements = shuffle_requirements.saturating_add(1);
            push_requirement_unique(
                &mut requirements,
                &mut seen,
                CryptoRequirementView {
                    id: format!("verifiable_shuffle:{}:library", player.index()),
                    requirement_type: "verifiable_shuffle".to_string(),
                    owner: player.index() as u8,
                    viewer: None,
                    zone: "library".to_string(),
                    slot: None,
                    object_id: None,
                    commitment: None,
                    card: None,
                    visibility: None,
                    reason: Some("library order changed (snapshot fallback)".to_string()),
                    count: Some(after_library_order.len().min(u16::MAX as usize) as u16),
                    from: None,
                    to: None,
                    before_order: Some(before_shuffle_order.iter().map(|id| id.0).collect()),
                    after_order: Some(after_shuffle_order.iter().map(|id| id.0).collect()),
                    random_count_before: Some(before.random_count),
                    random_count_after: Some(after.random_count),
                },
            );
        }

        let random_delta = after
            .random_count
            .saturating_sub(before.random_count)
            .saturating_sub(journaled_random_delta);
        if random_delta > shuffle_requirements {
            let owner = self
                .game
                .turn
                .priority_player
                .unwrap_or(self.game.turn.active_player);
            push_requirement_unique(
                &mut requirements,
                &mut seen,
                CryptoRequirementView {
                    id: format!("fair_random:{}:{}", owner.index(), after.random_count),
                    requirement_type: "fair_random".to_string(),
                    owner: owner.index() as u8,
                    viewer: None,
                    zone: "game".to_string(),
                    slot: None,
                    object_id: None,
                    commitment: None,
                    card: None,
                    visibility: None,
                    reason: Some("runtime consumed irreversible random output".to_string()),
                    count: Some((random_delta - shuffle_requirements).min(u16::MAX as u64) as u16),
                    from: None,
                    to: None,
                    before_order: None,
                    after_order: None,
                    random_count_before: Some(before.random_count),
                    random_count_after: Some(after.random_count),
                },
            );
        }

        self.last_crypto_requirements = requirements;
    }
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

fn stack_display_lines_from_abilities(
    abilities: &[ironsmith::ability::Ability],
    wants_triggered: bool,
) -> Vec<String> {
    abilities
        .iter()
        .filter_map(|ability| match (&ability.kind, wants_triggered) {
            (ironsmith::ability::AbilityKind::Triggered(triggered), true) => {
                let trigger = triggered.trigger.display();
                let effects = ironsmith::compiled_text::compile_effect_list(&triggered.effects);
                if effects.trim().is_empty() {
                    Some(trigger)
                } else {
                    Some(format!("{trigger}: {effects}"))
                }
            }
            (ironsmith::ability::AbilityKind::Activated(_), false) => {
                let text = ironsmith::compiled_text::ability_surface_text(ability);
                if text.trim().is_empty() {
                    Some("Activated ability".to_string())
                } else {
                    Some(text)
                }
            }
            _ => None,
        })
        .collect()
}

fn fallback_stack_entry_ability_text(
    entry: &ironsmith::game_state::StackEntry,
    obj: Option<&ironsmith::object::Object>,
) -> Option<String> {
    let wants_triggered = entry.triggering_event.is_some();

    if let Some(source_obj) = obj {
        let compiled_lines =
            ironsmith::compiled_text::compiled_text_lines(&source_obj.to_card_definition());
        if let Some(text) = first_matching_stack_line(&compiled_lines, wants_triggered) {
            return Some(text);
        }

        let oracle_lines: Vec<String> = source_obj
            .compiled_card_text
            .lines()
            .filter_map(normalize_stack_display_text)
            .collect();
        if let Some(text) = first_matching_stack_line(&oracle_lines, wants_triggered) {
            return Some(text);
        }
    }

    let snapshot_ability_texts: Vec<String> = entry
        .source_snapshot
        .as_ref()
        .into_iter()
        .flat_map(|snapshot| {
            stack_display_lines_from_abilities(&snapshot.abilities, wants_triggered)
        })
        .collect();
    first_matching_stack_line(&snapshot_ability_texts, wants_triggered)
}

fn stack_entry_ability_text(
    entry: &ironsmith::game_state::StackEntry,
    obj: Option<&ironsmith::object::Object>,
) -> Option<String> {
    entry
        .ability_effects
        .as_ref()
        .map(|effects| ironsmith::compiled_text::compile_effect_list(effects))
        .and_then(|text| normalize_stack_display_text(&text))
        .or_else(|| fallback_stack_entry_ability_text(entry, obj))
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
    UntapLand {
        stable_id: u64,
    },
    TurnFaceUp {
        creature_id: u64,
        method: String,
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
        method: String,
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
    FaceDown,
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
    SplitOtherHalfPlayFrom {
        source: u64,
        zone: String,
        use_alternative: usize,
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
    related_object_ids: Option<Vec<u64>>,
}

#[derive(Debug, Clone, Serialize)]
struct ObjectChoiceView {
    id: u64,
    name: String,
    legal: bool,
    object_controller: Option<u8>,
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
    TextInput {
        player: u8,
        description: String,
        placeholder: Option<String>,
        value: Option<String>,
        require_known_value: bool,
        source_id: Option<u64>,
        source_name: Option<String>,
        context_text: Option<String>,
        consequence_text: Option<String>,
        reason: Option<String>,
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
        allow_partial_completion: bool,
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
    fn text_with_visible_hidden_cards<F>(
        game: &GameState,
        ctx: &DecisionContext,
        text: &str,
        object_visible: &F,
    ) -> String
    where
        F: Fn(ObjectId) -> bool,
    {
        if !text.contains("Hidden Card") && !text.contains("Hidden card") {
            return text.to_string();
        }

        let mut names = Vec::new();
        let mut seen = HashSet::new();
        for view in ctx.hidden_card_views() {
            for id in &view.object_ids {
                if !object_visible(*id) {
                    continue;
                }
                let Some(object) = game.object(*id) else {
                    continue;
                };
                if object.name.eq_ignore_ascii_case("hidden card") {
                    continue;
                }
                if seen.insert(object.name.clone()) {
                    names.push(object.name.clone());
                }
            }
        }

        match names.as_slice() {
            [] => text.to_string(),
            [name] => text
                .replace("Hidden Card", name)
                .replace("Hidden card", name),
            many => {
                let mut replaced = text.to_string();
                for name in many {
                    if replaced.contains("Hidden Card") {
                        replaced = replaced.replacen("Hidden Card", name, 1);
                    } else if replaced.contains("Hidden card") {
                        replaced = replaced.replacen("Hidden card", name, 1);
                    } else {
                        break;
                    }
                }
                replaced
            }
        }
    }

    fn from_context(
        game: &GameState,
        ctx: &DecisionContext,
        perspective: PlayerId,
        viewed_cards: Option<&ActiveViewedCards>,
        undo_land_stable_id: Option<u64>,
    ) -> Self {
        let enriched_ctx = ironsmith::decisions::context::enrich_display_hints(game, ctx.clone());
        let ctx = &enriched_ctx;
        let decision_player_for = |player: PlayerId| game.controlling_player_for(player);
        let decision_object_visible = |id| {
            object_visible_to_perspective(game, perspective, viewed_cards, id)
                || decision_exposes_object_to_perspective(game, Some(ctx), perspective, id)
        };
        let resolve_source_name = |source: Option<ObjectId>| -> Option<String> {
            source
                .and_then(|id| game.object(id).map(|obj| (id, obj)))
                .map(|(id, obj)| {
                    if decision_object_visible(id) {
                        obj.name.clone()
                    } else {
                        hidden_object_label()
                    }
                })
        };
        let resolve_source_id = |source: Option<ObjectId>| -> Option<u64> {
            source.and_then(|id| decision_object_visible(id).then_some(id.0))
        };
        let context_text = || ctx.context_text().map(str::to_string);
        let consequence_text = || ctx.consequence_text().map(str::to_string);
        let reason = decision_reason(ctx);

        match ctx {
            DecisionContext::Boolean(boolean) => DecisionView::SelectOptions {
                player: decision_player_for(boolean.player).0,
                description: Self::text_with_visible_hidden_cards(
                    game,
                    ctx,
                    &boolean.description,
                    &decision_object_visible,
                ),
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
                        related_object_ids: None,
                    },
                    OptionView {
                        index: 0,
                        description: "No".to_string(),
                        legal: true,
                        repeatable: false,
                        max_count: Some(1),
                        object_id: None,
                        object_controller: None,
                        related_object_ids: None,
                    },
                ],
                source_id: resolve_source_id(boolean.source),
                source_name: resolve_source_name(boolean.source),
                context_text: context_text(),
                consequence_text: consequence_text(),
                reason: reason.clone(),
            },
            DecisionContext::Priority(priority) => {
                let decision_player = decision_player_for(priority.player);
                let mut actions: Vec<ActionView> = priority
                    .actions
                    .iter()
                    .enumerate()
                    .map(|(index, action)| {
                        build_action_view(game, perspective, viewed_cards, index, action)
                    })
                    .collect();
                if decision_player == perspective
                    && let Some(stable_id) = undo_land_stable_id
                    && let Some(action) = build_untap_land_action_view(
                        game,
                        perspective,
                        viewed_cards,
                        actions.len(),
                        stable_id,
                    )
                {
                    actions.push(action);
                }
                DecisionView::Priority {
                    player: decision_player.0,
                    actions,
                }
            }
            DecisionContext::TextInput(text) => DecisionView::TextInput {
                player: decision_player_for(text.player).0,
                description: Self::text_with_visible_hidden_cards(
                    game,
                    ctx,
                    &text.description,
                    &decision_object_visible,
                ),
                placeholder: text.placeholder.clone(),
                value: text.initial_value.clone(),
                require_known_value: text.require_known_value,
                source_id: resolve_source_id(text.source),
                source_name: resolve_source_name(text.source),
                context_text: context_text(),
                consequence_text: consequence_text(),
                reason: reason.clone(),
            },
            DecisionContext::Number(number) => DecisionView::Number {
                player: decision_player_for(number.player).0,
                description: Self::text_with_visible_hidden_cards(
                    game,
                    ctx,
                    &number.description,
                    &decision_object_visible,
                ),
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
                player: decision_player_for(options.player).0,
                description: Self::text_with_visible_hidden_cards(
                    game,
                    ctx,
                    &options.description,
                    &decision_object_visible,
                ),
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
                                (opt.repeatable, opt.max_count)
                            };
                            let visible_object_id = opt
                                .object_id
                                .and_then(|id| decision_object_visible(id).then_some(id));
                            let visible_related_object_ids =
                                opt.related_object_ids.as_ref().map(|object_ids| {
                                    object_ids
                                        .iter()
                                        .filter(|id| decision_object_visible(**id))
                                        .map(|id| id.0)
                                        .collect::<Vec<_>>()
                                });
                            let description =
                                if opt.object_id.is_some() && visible_object_id.is_none() {
                                    hidden_object_label()
                                } else {
                                    opt.description.clone()
                                };
                            OptionView {
                                index: opt.index,
                                description: Self::text_with_visible_hidden_cards(
                                    game,
                                    ctx,
                                    &description,
                                    &decision_object_visible,
                                ),
                                legal: opt.legal,
                                repeatable,
                                max_count,
                                object_id: visible_object_id.map(|id| id.0),
                                object_controller: visible_object_id
                                    .and_then(|id| {
                                        game.current_controller(id)
                                            .or_else(|| game.object(id).map(|obj| obj.owner))
                                    })
                                    .map(|controller| controller.0),
                                related_object_ids: visible_related_object_ids,
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
                player: decision_player_for(modes.player).0,
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
                        repeatable: modes.spec.allow_repeated_modes,
                        max_count: Some(modes.spec.max_modes.min(u32::MAX as usize) as u32),
                        object_id: None,
                        object_controller: None,
                        related_object_ids: mode
                            .related_object_ids
                            .as_ref()
                            .map(|object_ids| object_ids.iter().map(|id| id.0).collect()),
                    })
                    .collect(),
                source_id: resolve_source_id(modes.source),
                source_name: resolve_source_name(modes.source),
                context_text: context_text(),
                consequence_text: consequence_text(),
                reason: reason.clone(),
            },
            DecisionContext::HybridChoice(hybrid) => DecisionView::SelectOptions {
                player: decision_player_for(hybrid.player).0,
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
                        related_object_ids: None,
                    })
                    .collect(),
                source_id: resolve_source_id(hybrid.source),
                source_name: resolve_source_name(hybrid.source),
                context_text: context_text(),
                consequence_text: consequence_text(),
                reason: reason.clone(),
            },
            DecisionContext::Order(order) => DecisionView::SelectOptions {
                player: decision_player_for(order.player).0,
                description: order.description.clone(),
                min: order.items.len(),
                max: order.items.len(),
                options: order
                    .items
                    .iter()
                    .enumerate()
                    .map(|(index, (object_id, name))| {
                        let is_real_object = game.object(*object_id).is_some();
                        let visible = decision_object_visible(*object_id);
                        OptionView {
                            index,
                            description: if visible || !is_real_object {
                                name.clone()
                            } else {
                                hidden_object_label()
                            },
                            legal: true,
                            repeatable: false,
                            max_count: Some(1),
                            object_id: visible.then_some(object_id.0),
                            object_controller: visible
                                .then(|| {
                                    game.current_controller(*object_id)
                                        .or_else(|| game.object(*object_id).map(|obj| obj.owner))
                                        .map(|controller| controller.0)
                                })
                                .flatten(),
                            related_object_ids: None,
                        }
                    })
                    .collect(),
                source_id: resolve_source_id(order.source),
                source_name: resolve_source_name(order.source),
                context_text: context_text(),
                consequence_text: consequence_text(),
                reason: reason.clone(),
            },
            DecisionContext::Distribute(distribute) => DecisionView::SelectOptions {
                player: decision_player_for(distribute.player).0,
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
                            Target::Object(object_id) => {
                                decision_object_visible(*object_id).then_some(*object_id)
                            }
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
                                .and_then(|object_id| {
                                    game.current_controller(object_id)
                                        .or_else(|| game.object(object_id).map(|obj| obj.owner))
                                })
                                .map(|controller| controller.0),
                            related_object_ids: None,
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
                    player: decision_player_for(colors.player).0,
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
                            related_object_ids: None,
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
                player: decision_player_for(counters.player).0,
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
                        related_object_ids: None,
                    })
                    .collect(),
                source_id: resolve_source_id(counters.source),
                source_name: resolve_source_name(counters.source),
                context_text: context_text(),
                consequence_text: consequence_text(),
                reason: reason.clone(),
            },
            DecisionContext::Partition(partition) => DecisionView::SelectObjects {
                player: decision_player_for(partition.player).0,
                description: format!(
                    "{} \u{2014} select cards to put on {}",
                    partition.description, partition.secondary_label
                ),
                min: 0,
                max: Some(partition.cards.len()),
                allow_partial_completion: false,
                candidates: partition
                    .cards
                    .iter()
                    .map(|(id, name)| ObjectChoiceView {
                        id: id.0,
                        name: name.clone(),
                        legal: true,
                        object_controller: game
                            .current_controller(*id)
                            .or_else(|| game.object(*id).map(|obj| obj.owner))
                            .map(|controller| controller.0),
                    })
                    .collect(),
                source_id: resolve_source_id(partition.source),
                source_name: resolve_source_name(partition.source),
                context_text: context_text(),
                consequence_text: consequence_text(),
                reason: reason.clone(),
            },
            DecisionContext::Proliferate(proliferate) => DecisionView::SelectOptions {
                player: decision_player_for(proliferate.player).0,
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
                            .and_then(|(id, _)| {
                                game.current_controller(*id)
                                    .or_else(|| game.object(*id).map(|obj| obj.owner))
                            })
                            .map(|controller| controller.0),
                        related_object_ids: None,
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
                            related_object_ids: None,
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
                player: decision_player_for(objects.player).0,
                description: objects.description.clone(),
                min: objects.min,
                max: objects.max,
                allow_partial_completion: objects.allow_partial_completion,
                candidates: objects
                    .candidates
                    .iter()
                    .enumerate()
                    .map(|(index, obj)| {
                        let visible = decision_object_visible(obj.id);
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
                            object_controller: visible
                                .then(|| {
                                    game.current_controller(obj.id)
                                        .or_else(|| game.object(obj.id).map(|object| object.owner))
                                })
                                .flatten()
                                .map(|controller| controller.0),
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
                player: decision_player_for(targets.player).0,
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
                                target_choice_view(
                                    game,
                                    perspective,
                                    viewed_cards,
                                    Some(ctx),
                                    index,
                                    target,
                                )
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
                player: decision_player_for(attackers.player).0,
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
                player: decision_player_for(blockers.player).0,
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
    TextChoice {
        value: String,
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
    Text(String),
    Options(Vec<usize>),
    Objects(Vec<ObjectId>),
    Order(Vec<ObjectId>),
    Distribute(Vec<(Target, u32)>),
    Colors(Vec<ironsmith::color::Color>),
    Counters(Vec<(ironsmith::object::CounterType, u32)>),
    Partition(Vec<ObjectId>),
    Proliferate(ironsmith::decisions::specs::ProliferateResponse),
    Targets(Vec<Target>),
    Priority(LegalAction),
    Attackers(Vec<ironsmith::decisions::spec::AttackerDeclaration>),
    Blockers(Vec<ironsmith::decisions::spec::BlockerDeclaration>),
}

#[derive(Debug, Clone)]
struct ReplayCheckpoint {
    game: GameState,
    trigger_queue: TriggerQueue,
    priority_state: PriorityLoopState,
    game_over: Option<GameResult>,
    id_counters: ironsmith::ids::IdCountersSnapshot,
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
    /// A card was injected directly into a zone and needs replay to resolve nested prompts.
    AddCardToZone {
        player: PlayerId,
        card_name: String,
        zone: Zone,
        skip_triggers: bool,
    },
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
    speculative_progress: Option<GameProgress>,
}

#[derive(Debug, Clone)]
enum ReplayOutcome {
    NeedsDecision(DecisionContext),
    Complete(GameProgress),
}

fn ui_command_kind(command: &UiCommand) -> &'static str {
    match command {
        UiCommand::PriorityAction { .. } => "priority_action",
        UiCommand::SelectTargets { .. } => "select_targets",
        UiCommand::SelectOptions { .. } => "select_options",
        UiCommand::SelectObjects { .. } => "select_objects",
        UiCommand::NumberChoice { .. } => "number_choice",
        UiCommand::TextChoice { .. } => "text_choice",
        UiCommand::DeclareAttackers { .. } => "declare_attackers",
        UiCommand::DeclareBlockers { .. } => "declare_blockers",
    }
}

fn replay_root_kind(root: &ReplayRoot) -> &'static str {
    match root {
        ReplayRoot::Response(_) => "response",
        ReplayRoot::Advance => "advance",
        ReplayRoot::AddCardToZone { .. } => "add_card_to_zone",
    }
}

fn game_progress_kind(progress: &GameProgress) -> &'static str {
    match progress {
        GameProgress::NeedsDecisionCtx(_) => "needs_decision",
        GameProgress::Continue => "continue",
        GameProgress::GameOver(_) => "game_over",
        GameProgress::StackResolved => "stack_resolved",
    }
}

fn replay_outcome_kind(outcome: &ReplayOutcome) -> &'static str {
    match outcome {
        ReplayOutcome::NeedsDecision(_) => "needs_decision",
        ReplayOutcome::Complete(_) => "complete",
    }
}

#[derive(Debug)]
struct WasmReplayDecisionMaker {
    answers: VecDeque<ReplayDecisionAnswer>,
    pending_context: Option<DecisionContext>,
    viewed_cards: Option<ActiveViewedCards>,
    audit_viewed_cards: Vec<ActiveViewedCards>,
}

impl WasmReplayDecisionMaker {
    fn new(answers: &[ReplayDecisionAnswer]) -> Self {
        Self {
            answers: answers.iter().cloned().collect(),
            pending_context: None,
            viewed_cards: None,
            audit_viewed_cards: Vec::new(),
        }
    }

    fn capture_once(&mut self, ctx: DecisionContext) {
        if self.pending_context.is_none() {
            self.pending_context = Some(ctx);
        }
    }

    fn capture_once_for_game(&mut self, game: &GameState, ctx: DecisionContext) {
        let enriched = ironsmith::decisions::context::enrich_display_hints(game, ctx);
        merge_hidden_decision_views(
            game,
            &mut self.viewed_cards,
            &mut self.audit_viewed_cards,
            &enriched,
        );
        self.capture_once(enriched);
    }

    fn finish(
        self,
    ) -> (
        Option<DecisionContext>,
        Option<ActiveViewedCards>,
        Vec<ActiveViewedCards>,
    ) {
        (
            self.pending_context,
            self.viewed_cards,
            self.audit_viewed_cards,
        )
    }
}

impl DecisionMaker for WasmReplayDecisionMaker {
    fn awaiting_choice(&self) -> bool {
        self.pending_context.is_some()
    }

    fn decide_boolean(
        &mut self,
        game: &GameState,
        ctx: &ironsmith::decisions::context::BooleanContext,
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
        ctx: &ironsmith::decisions::context::NumberContext,
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

    fn decide_text(
        &mut self,
        game: &GameState,
        ctx: &ironsmith::decisions::context::TextInputContext,
    ) -> String {
        match self.answers.front() {
            Some(ReplayDecisionAnswer::Text(value)) => {
                let value = value.clone();
                self.answers.pop_front();
                value
            }
            _ => {
                self.capture_once_for_game(game, DecisionContext::TextInput(ctx.clone()));
                ctx.initial_value.clone().unwrap_or_default()
            }
        }
    }

    fn decide_objects(
        &mut self,
        game: &GameState,
        ctx: &ironsmith::decisions::context::SelectObjectsContext,
    ) -> Vec<ObjectId> {
        match self.answers.front() {
            Some(ReplayDecisionAnswer::Objects(ids)) => {
                let ids = ids.clone();
                self.answers.pop_front();
                ids
            }
            _ => {
                self.capture_once_for_game(game, DecisionContext::SelectObjects(ctx.clone()));
                if ctx.allow_partial_completion {
                    Vec::new()
                } else {
                    ctx.candidates
                        .iter()
                        .filter(|candidate| candidate.legal)
                        .map(|candidate| candidate.id)
                        .take(ctx.min)
                        .collect()
                }
            }
        }
    }

    fn decide_options(
        &mut self,
        game: &GameState,
        ctx: &ironsmith::decisions::context::SelectOptionsContext,
    ) -> Vec<usize> {
        match self.answers.front() {
            Some(ReplayDecisionAnswer::Options(indices)) => {
                let indices = indices.clone();
                self.answers.pop_front();
                indices
            }
            _ => {
                self.capture_once_for_game(game, DecisionContext::SelectOptions(ctx.clone()));
                let mut selected = Vec::new();
                let mut counts: std::collections::HashMap<usize, usize> =
                    std::collections::HashMap::new();
                let legal: Vec<_> = ctx.options.iter().filter(|option| option.legal).collect();
                while selected.len() < ctx.min {
                    let mut added = false;
                    for option in &legal {
                        let current = counts.get(&option.index).copied().unwrap_or(0);
                        let limit = if option.repeatable {
                            option
                                .max_count
                                .map(|count| count as usize)
                                .unwrap_or(usize::MAX)
                        } else {
                            1
                        };
                        if current >= limit {
                            continue;
                        }
                        selected.push(option.index);
                        counts.insert(option.index, current + 1);
                        added = true;
                        break;
                    }
                    if !added {
                        break;
                    }
                }
                selected
            }
        }
    }

    fn decide_priority(
        &mut self,
        _game: &GameState,
        ctx: &ironsmith::decisions::context::PriorityContext,
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
        ctx: &ironsmith::decisions::context::TargetsContext,
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
        ctx: &ironsmith::decisions::context::AttackersContext,
    ) -> Vec<ironsmith::decisions::spec::AttackerDeclaration> {
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
                        ironsmith::decisions::spec::AttackerDeclaration {
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
        _ctx: &ironsmith::decisions::context::BlockersContext,
    ) -> Vec<ironsmith::decisions::spec::BlockerDeclaration> {
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
        ctx: &ironsmith::decisions::context::OrderContext,
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
        ctx: &ironsmith::decisions::context::DistributeContext,
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
        ctx: &ironsmith::decisions::context::ColorsContext,
    ) -> Vec<ironsmith::color::Color> {
        match self.answers.front() {
            Some(ReplayDecisionAnswer::Colors(colors)) => {
                let colors = colors.clone();
                self.answers.pop_front();
                colors
            }
            _ => {
                self.capture_once_for_game(game, DecisionContext::Colors(ctx.clone()));
                vec![ironsmith::color::Color::Green; ctx.count as usize]
            }
        }
    }

    fn decide_counters(
        &mut self,
        game: &GameState,
        ctx: &ironsmith::decisions::context::CountersContext,
    ) -> Vec<(ironsmith::object::CounterType, u32)> {
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
        ctx: &ironsmith::decisions::context::PartitionContext,
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
        ctx: &ironsmith::decisions::context::ProliferateContext,
    ) -> ironsmith::decisions::specs::ProliferateResponse {
        match self.answers.front() {
            Some(ReplayDecisionAnswer::Proliferate(response)) => {
                let response = response.clone();
                self.answers.pop_front();
                response
            }
            _ => {
                self.capture_once_for_game(game, DecisionContext::Proliferate(ctx.clone()));
                ironsmith::decisions::specs::ProliferateResponse::default()
            }
        }
    }

    fn view_cards(
        &mut self,
        game: &GameState,
        viewer: PlayerId,
        cards: &[ObjectId],
        ctx: &ironsmith::decisions::context::ViewCardsContext,
    ) {
        merge_active_viewed_cards(game, &mut self.viewed_cards, viewer, cards, ctx);
        merge_audit_viewed_cards(game, &mut self.audit_viewed_cards, viewer, cards, ctx);
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
    runner: Option<ironsmith::turn_runner::TurnRunner>,
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
    /// All hidden-card view events emitted by the most recent resolved command.
    active_audit_viewed_cards: Vec<ActiveViewedCards>,
    /// Crypto material required by the most recent resolved command.
    last_crypto_requirements: Vec<CryptoRequirementView>,
    /// Hidden/random snapshot captured immediately before the command currently
    /// being resolved. Consumed by the next snapshot after dispatch.
    pending_crypto_audit_before: Option<CryptoAuditState>,
    /// UI-only top stack entry that is currently resolving while a prompt is open.
    active_resolving_stack_object: Option<StackObjectSnapshot>,
    /// Last decklists loaded into the current session, indexed by player.
    loaded_decks: Vec<Vec<String>>,
    /// Timing breakdown for the most recent snapshot build/encode pass.
    last_snapshot_perf: Option<SnapshotPerfMetrics>,
    /// Timing breakdown for the most recent replay execution inside dispatch.
    last_replay_execution_perf: Option<ReplayExecutionPerfMetrics>,
    /// Timing breakdown for the most recent `advance_until_decision` pass.
    last_advance_until_decision_perf: Option<AdvanceUntilDecisionPerfMetrics>,
    /// Timing breakdown for the most recent dispatch-like engine call.
    last_dispatch_perf: Option<DispatchPerfMetrics>,
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

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum CustomCardLayoutInput {
    #[default]
    Single,
    TransformLike,
    Split,
}

impl CustomCardLayoutInput {
    fn face_count(self) -> usize {
        match self {
            CustomCardLayoutInput::Single => 1,
            CustomCardLayoutInput::TransformLike | CustomCardLayoutInput::Split => 2,
        }
    }

    fn linked_face_layout(self) -> ironsmith::card::LinkedFaceLayout {
        match self {
            CustomCardLayoutInput::Single => ironsmith::card::LinkedFaceLayout::None,
            CustomCardLayoutInput::TransformLike => {
                ironsmith::card::LinkedFaceLayout::TransformLike
            }
            CustomCardLayoutInput::Split => ironsmith::card::LinkedFaceLayout::Split,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct CustomCardFaceInput {
    name: String,
    #[serde(default)]
    mana_cost: Option<String>,
    #[serde(default)]
    color_indicator: Vec<String>,
    #[serde(default)]
    supertypes: Vec<String>,
    #[serde(default)]
    card_types: Vec<String>,
    #[serde(default)]
    subtypes: Vec<String>,
    #[serde(default)]
    oracle_text: String,
    #[serde(default)]
    power: Option<String>,
    #[serde(default)]
    toughness: Option<String>,
    #[serde(default)]
    loyalty: Option<u32>,
    #[serde(default)]
    defense: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CustomCardInput {
    #[serde(default)]
    layout: CustomCardLayoutInput,
    #[serde(default)]
    has_fuse: bool,
    #[serde(default)]
    faces: Vec<CustomCardFaceInput>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateCustomCardInput {
    draft: CustomCardInput,
    player_index: u8,
    zone_name: String,
    #[serde(default)]
    skip_triggers: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct CustomCardPreviewFace {
    name: String,
    mana_cost: Option<String>,
    color_indicator: Vec<String>,
    type_line: String,
    oracle_text: String,
    power: Option<String>,
    toughness: Option<String>,
    loyalty: Option<u32>,
    defense: Option<u32>,
    compiled_text: Vec<String>,
    compiled_abilities: Vec<String>,
    raw_compilation: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct CustomCardPreviewResult {
    layout: CustomCardLayoutInput,
    has_fuse: bool,
    faces: Vec<CustomCardPreviewFace>,
    can_create: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct CustomCardSeedResult {
    layout: CustomCardLayoutInput,
    has_fuse: bool,
    faces: Vec<CustomCardFaceInput>,
}

static AUTOCOMPLETE_CARD_NAMES: OnceLock<Vec<(String, String)>> = OnceLock::new();

#[derive(Debug, Clone, Deserialize, Serialize)]
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
    sideboards: Option<Vec<Vec<String>>>,
    #[serde(default)]
    commanders: Option<Vec<Vec<String>>>,
    #[serde(default)]
    opening_hand_size: Option<usize>,
    #[serde(default)]
    hidden_deck_manifests: Option<Vec<HiddenDeckManifestInput>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct HiddenDeckManifestInput {
    owner: u8,
    #[serde(default)]
    deck_count: usize,
    #[serde(default)]
    sideboard_count: usize,
    #[serde(default)]
    commander_count: usize,
    #[serde(default)]
    decklist_hash: String,
    #[serde(default)]
    commitment_root: String,
    #[serde(default)]
    slot_commitments: Vec<HiddenDeckSlotInput>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct HiddenDeckSlotInput {
    slot: u16,
    commitment: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RevealHiddenObjectInput {
    object_id: u64,
    card_name: String,
    #[serde(default)]
    slot: Option<u16>,
    #[serde(default)]
    commitment: Option<String>,
    #[serde(default)]
    recompute_decision: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RevealHiddenSlotInput {
    owner: u8,
    slot: u16,
    card_name: String,
    #[serde(default)]
    commitment: Option<String>,
    #[serde(default)]
    recompute_decision: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RevealHiddenPositionInput {
    owner: u8,
    position: u16,
    original_slot: u16,
    card_name: String,
    #[serde(default)]
    position_commitment: Option<String>,
    #[serde(default)]
    commitment: Option<String>,
    #[serde(default)]
    recompute_decision: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TranscriptRandomSeedsInput {
    #[serde(default)]
    seeds: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApplyHiddenLibraryShuffleInput {
    owner: u8,
    deck_hash: String,
    #[serde(default)]
    after_order: Vec<u64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct HiddenCardOpeningExport {
    object_id: u64,
    owner: u8,
    slot: u16,
    card: String,
    commitment: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ZiffleEntropyInput {
    deck_count: usize,
    context: String,
    entropy_hex: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ZiffleKeygenOutput {
    deck_count: usize,
    public_key_hex: String,
    secret_key_hex: String,
    ownership_proof_hex: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ZifflePublicKeyInput {
    player: u8,
    public_key_hex: String,
    ownership_proof_hex: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ZiffleShuffleStepInput {
    shuffler: u8,
    deck_hex: String,
    proof_hex: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ZiffleShuffleStepOutput {
    shuffler: u8,
    deck_hex: String,
    proof_hex: String,
    deck_hash: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ZiffleBuildShuffleStepInput {
    deck_count: usize,
    context: String,
    #[serde(default)]
    key_context: String,
    keys: Vec<ZifflePublicKeyInput>,
    steps: Vec<ZiffleShuffleStepInput>,
    shuffler: u8,
    entropy_hex: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ZiffleVerifyShuffleInput {
    deck_count: usize,
    context: String,
    #[serde(default)]
    key_context: String,
    keys: Vec<ZifflePublicKeyInput>,
    steps: Vec<ZiffleShuffleStepInput>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ZiffleVerifyShuffleOutput {
    deck_count: usize,
    deck_hex: String,
    deck_hash: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ZiffleBuildRevealTokenInput {
    deck_count: usize,
    context: String,
    #[serde(default)]
    key_context: String,
    keys: Vec<ZifflePublicKeyInput>,
    steps: Vec<ZiffleShuffleStepInput>,
    card_position: usize,
    public_key_hex: String,
    secret_key_hex: String,
    entropy_hex: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ZiffleBuildRevealTokensInput {
    deck_count: usize,
    context: String,
    #[serde(default)]
    key_context: String,
    keys: Vec<ZifflePublicKeyInput>,
    steps: Vec<ZiffleShuffleStepInput>,
    card_positions: Vec<usize>,
    public_key_hex: String,
    secret_key_hex: String,
    entropy_hex: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ZiffleRevealTokenInput {
    player: u8,
    public_key_hex: String,
    token_hex: String,
    proof_hex: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ZiffleRevealTokenBatchInput {
    card_position: usize,
    player: u8,
    public_key_hex: String,
    token_hex: String,
    proof_hex: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ZiffleRevealTokenOutput {
    player: u8,
    public_key_hex: String,
    token_hex: String,
    proof_hex: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ZiffleRevealTokenBatchOutput {
    card_position: usize,
    player: u8,
    public_key_hex: String,
    token_hex: String,
    proof_hex: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ZiffleRevealCardInput {
    deck_count: usize,
    context: String,
    #[serde(default)]
    key_context: String,
    keys: Vec<ZifflePublicKeyInput>,
    steps: Vec<ZiffleShuffleStepInput>,
    card_position: usize,
    tokens: Vec<ZiffleRevealTokenInput>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ZiffleRevealCardsInput {
    deck_count: usize,
    context: String,
    #[serde(default)]
    key_context: String,
    keys: Vec<ZifflePublicKeyInput>,
    steps: Vec<ZiffleShuffleStepInput>,
    card_positions: Vec<usize>,
    tokens: Vec<ZiffleRevealTokenBatchInput>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ZiffleRevealCardOutput {
    card_position: usize,
    original_slot: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct MatchValidationIssue {
    player_index: usize,
    player_name: String,
    section: String,
    card_name: String,
    error: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct MatchValidationResult {
    valid: bool,
    issues: Vec<MatchValidationIssue>,
}

#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize, PartialEq, Eq)]
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

mod wasm_game_impl;
use wasm_game_impl::*;

#[cfg(all(test, not(target_arch = "wasm32")))]
mod native_tests {
    use super::*;

    #[test]
    fn phyrexian_tower_action_surface_uses_compiled_cost_text() {
        let definition = ironsmith_registry::cards::definitions::phyrexian_tower();
        let lines = stack_display_lines_from_abilities(&definition.abilities, false);

        assert!(
            lines
                .iter()
                .any(|line| line.contains("Sacrifice a creature") && line.contains("Add {B}{B}")),
            "Phyrexian Tower sacrifice mana line should use compiled oracle text: {lines:?}"
        );
        assert!(
            lines.iter().all(|line| {
                !line.contains("Exile a creature") && !line.contains("Sacrifice a permanent")
            }),
            "Phyrexian Tower action lines should not expose raw tagged costs: {lines:?}"
        );
    }

    #[test]
    fn pregame_pass_actions_surface_decision_labels() {
        let game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);

        assert_eq!(
            describe_action(&game, &LegalAction::KeepOpeningHand),
            "Keep hand"
        );
        assert_eq!(
            describe_action(&game, &LegalAction::ContinuePregame),
            "Pregame"
        );
        assert_eq!(describe_action(&game, &LegalAction::BeginGame), "Pregame");
    }

    #[test]
    fn crypto_requirements_include_static_public_top_library_opening() {
        let mut wasm = WasmGame::new();
        let alice = PlayerId::from_index(0);

        let lantern = ironsmith::cards::builders::CardDefinitionBuilder::new(
            CardId::new(),
            "Lantern of Insight Variant",
        )
        .card_types(vec![CardType::Artifact])
        .with_ability(ironsmith::ability::Ability::static_ability(
            ironsmith::static_abilities::StaticAbility::all_players_look_at_top_cards_of_libraries(
            ),
        ))
        .build();
        wasm.game
            .create_object_from_definition(&lantern, alice, Zone::Battlefield);
        let hidden_top = wasm.game.create_hidden_card_placeholder(
            alice,
            Zone::Library,
            0,
            "alice-library-top-commitment".to_string(),
        );

        let before = wasm.capture_crypto_audit_state();
        wasm.update_crypto_requirements_from(before);

        assert!(wasm.last_crypto_requirements.iter().any(|requirement| {
            requirement.requirement_type == "public_view_window"
                && requirement.owner == alice.index() as u8
                && requirement.zone == "library"
                && requirement.count == Some(1)
        }));
        assert!(wasm.last_crypto_requirements.iter().any(|requirement| {
            requirement.requirement_type == "public_open"
                && requirement.owner == alice.index() as u8
                && requirement.zone == "library"
                && requirement.object_id == Some(hidden_top.0)
                && requirement.commitment.as_deref() == Some("alice-library-top-commitment")
        }));
    }

    #[test]
    fn crypto_requirements_include_static_private_own_top_library_opening() {
        let mut wasm = WasmGame::new();
        let alice = PlayerId::from_index(0);

        let future_sight = ironsmith::cards::builders::CardDefinitionBuilder::new(
            CardId::new(),
            "Future Sight Variant",
        )
        .card_types(vec![CardType::Enchantment])
        .with_ability(ironsmith::ability::Ability::static_ability(
            ironsmith::static_abilities::StaticAbility::look_at_top_card_of_library(),
        ))
        .build();
        wasm.game
            .create_object_from_definition(&future_sight, alice, Zone::Battlefield);
        let hidden_top = wasm.game.create_hidden_card_placeholder(
            alice,
            Zone::Library,
            0,
            "alice-private-top-commitment".to_string(),
        );

        let before = wasm.capture_crypto_audit_state();
        wasm.update_crypto_requirements_from(before);

        assert!(wasm.last_crypto_requirements.iter().any(|requirement| {
            requirement.requirement_type == "private_view_window"
                && requirement.owner == alice.index() as u8
                && requirement.viewer == Some(alice.index() as u8)
                && requirement.zone == "library"
                && requirement.count == Some(1)
        }));
        assert!(wasm.last_crypto_requirements.iter().any(|requirement| {
            requirement.requirement_type == "private_open"
                && requirement.owner == alice.index() as u8
                && requirement.viewer == Some(alice.index() as u8)
                && requirement.zone == "library"
                && requirement.object_id == Some(hidden_top.0)
                && requirement.commitment.as_deref() == Some("alice-private-top-commitment")
        }));
    }

    #[test]
    fn crypto_requirements_include_static_public_hand_openings() {
        let mut wasm = WasmGame::new();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let telepathy = ironsmith::cards::builders::CardDefinitionBuilder::new(
            CardId::new(),
            "Telepathy Variant",
        )
        .card_types(vec![CardType::Enchantment])
        .with_ability(ironsmith::ability::Ability::static_ability(
            ironsmith::static_abilities::StaticAbility::opponents_play_with_hands_revealed(),
        ))
        .build();
        wasm.game
            .create_object_from_definition(&telepathy, alice, Zone::Battlefield);
        let hidden_hand = wasm.game.create_hidden_card_placeholder(
            bob,
            Zone::Hand,
            4,
            "bob-hand-commitment".to_string(),
        );

        let before = wasm.capture_crypto_audit_state();
        wasm.update_crypto_requirements_from(before);

        assert!(wasm.last_crypto_requirements.iter().any(|requirement| {
            requirement.requirement_type == "public_view_window"
                && requirement.owner == bob.index() as u8
                && requirement.zone == "hand"
                && requirement.count == Some(1)
        }));
        assert!(wasm.last_crypto_requirements.iter().any(|requirement| {
            requirement.requirement_type == "public_open"
                && requirement.owner == bob.index() as u8
                && requirement.zone == "hand"
                && requirement.object_id == Some(hidden_hand.0)
                && requirement.commitment.as_deref() == Some("bob-hand-commitment")
        }));
    }

    #[test]
    fn crypto_requirements_public_open_revealed_hidden_card_moved_to_exile() {
        let mut wasm = WasmGame::new();
        wasm.initialize_empty_match(vec!["Alice".to_string(), "Bob".to_string()], 20, 1);
        let alice = PlayerId::from_index(0);
        let hidden_hand_card = wasm.game.create_hidden_card_placeholder(
            alice,
            Zone::Hand,
            3,
            "alice-hidden-hand-commitment".to_string(),
        );
        let definition = ironsmith_registry::cards::definitions::ornithopter();
        wasm.game
            .reveal_hidden_card_with_definition(hidden_hand_card, &definition)
            .expect("hidden hand card should reveal locally");

        let before = wasm.capture_crypto_audit_state();
        let exiled_id = wasm
            .game
            .move_object_by_effect(hidden_hand_card, Zone::Exile)
            .expect("hidden hand card should move to exile");
        wasm.update_crypto_requirements_from(before);

        assert!(wasm.last_crypto_requirements.iter().any(|requirement| {
            requirement.requirement_type == "public_open"
                && requirement.owner == alice.index() as u8
                && requirement.zone == "face_down_exile"
                && requirement.object_id == Some(exiled_id.0)
                && requirement.commitment.as_deref() == Some("alice-hidden-hand-commitment")
                && requirement.card.as_deref() == Some("Ornithopter")
        }));
    }

    #[test]
    fn crypto_requirements_include_decision_hidden_card_public_openings() {
        let mut wasm = WasmGame::new();
        let alice = PlayerId::from_index(0);
        let hidden_exiled = wasm.game.create_hidden_card_placeholder(
            alice,
            Zone::Exile,
            0,
            "alice-exile-decision-commitment".to_string(),
        );
        let source = ObjectId::from_raw(77);
        let ctx = DecisionContext::Boolean(
            ironsmith::decisions::context::BooleanContext::new(
                alice,
                Some(source),
                "Put Hidden Card into your hand?",
            )
            .with_hidden_card_view(
                vec![hidden_exiled],
                DecisionHiddenCardVisibility::Public,
                "Inspect hidden card for decision",
            ),
        );
        let before = wasm.capture_crypto_audit_state();
        let mut replay = WasmReplayDecisionMaker::new(&[]);
        replay.capture_once_for_game(&wasm.game, ctx);
        let (_pending, viewed_cards, audit_views) = replay.finish();
        wasm.active_viewed_cards = viewed_cards;
        wasm.active_audit_viewed_cards = audit_views;

        wasm.update_crypto_requirements_from(before);

        assert!(wasm.last_crypto_requirements.iter().any(|requirement| {
            requirement.requirement_type == "public_view_window"
                && requirement.owner == alice.index() as u8
                && requirement.zone == "face_down_exile"
                && requirement.count == Some(1)
        }));
        assert!(wasm.last_crypto_requirements.iter().any(|requirement| {
            requirement.requirement_type == "public_open"
                && requirement.owner == alice.index() as u8
                && requirement.zone == "face_down_exile"
                && requirement.object_id == Some(hidden_exiled.0)
                && requirement.commitment.as_deref() == Some("alice-exile-decision-commitment")
        }));
    }

    #[test]
    fn decision_description_names_visible_hidden_card_views() {
        let mut game = GameState::new(vec!["Alice".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let hidden_exiled = game.create_hidden_card_placeholder(
            alice,
            Zone::Exile,
            0,
            "alice-exile-decision-commitment".to_string(),
        );
        let definition =
            ironsmith::cards::builders::CardDefinitionBuilder::new(CardId::new(), "Tainted Pact")
                .card_types(vec![CardType::Instant])
                .build();
        game.reveal_hidden_card_with_definition(hidden_exiled, &definition)
            .expect("hidden card should reveal in test state");
        let ctx = DecisionContext::Boolean(
            ironsmith::decisions::context::BooleanContext::new(
                alice,
                None,
                "Put Hidden Card into your hand?",
            )
            .with_hidden_card_view(
                vec![hidden_exiled],
                DecisionHiddenCardVisibility::PrivateToDecisionPlayer,
                "Inspect hidden card for decision",
            ),
        );
        let viewed_cards = ActiveViewedCards {
            viewer: alice,
            subject: alice,
            zone: Zone::Exile,
            cards: vec![hidden_exiled],
            card_stable_ids: stable_ids_for_viewed_cards(&game, &[hidden_exiled]),
            public: false,
            source: None,
            description: "Inspect hidden card for decision".to_string(),
        };

        let view = DecisionView::from_context(&game, &ctx, alice, Some(&viewed_cards), None);

        match view {
            DecisionView::SelectOptions { description, .. } => {
                assert_eq!(description, "Put Tainted Pact into your hand?");
            }
            other => panic!("expected select options view, got {other:?}"),
        }
    }

    #[test]
    fn viewed_card_snapshots_follow_stable_identity_when_object_id_is_stale() {
        let mut wasm = WasmGame::new();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let revealed_card =
            ironsmith::card::CardBuilder::new(CardId::from_raw(90_202), "Bob's Stable Secret")
                .card_types(vec![CardType::Instant])
                .build();
        let revealed_id = wasm
            .game
            .create_object_from_card(&revealed_card, bob, Zone::Hand);
        let stale_unrelated_id = ObjectId::from_raw(revealed_id.0.saturating_add(10_000));
        wasm.active_viewed_cards = Some(ActiveViewedCards {
            viewer: alice,
            subject: bob,
            zone: Zone::Hand,
            cards: vec![stale_unrelated_id],
            card_stable_ids: stable_ids_for_viewed_cards(&wasm.game, &[revealed_id]),
            public: false,
            source: None,
            description: "Inspect hidden card for decision".to_string(),
        });

        let snapshot = GameSnapshot::from_game(
            &wasm.game,
            alice,
            wasm.pending_decision.as_ref(),
            None,
            wasm.game_over.as_ref(),
            None,
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
            .expect("view should resolve through the card's stable id");
        assert_eq!(viewed_cards.card_ids, vec![revealed_id.0]);
        assert_eq!(viewed_cards.cards[0].name, "Bob's Stable Secret");
        assert_eq!(
            snapshot.players[bob.index()].hand_cards[0].name,
            "Bob's Stable Secret"
        );
    }

    #[test]
    fn crypto_requirements_include_journaled_library_shuffle() {
        let mut wasm = WasmGame::new();
        let alice = PlayerId::from_index(0);
        wasm.game
            .create_hidden_card_placeholder(alice, Zone::Library, 0, "slot-0".to_string());
        wasm.game
            .create_hidden_card_placeholder(alice, Zone::Library, 1, "slot-1".to_string());
        wasm.game
            .create_hidden_card_placeholder(alice, Zone::Library, 2, "slot-2".to_string());

        let before = wasm.capture_crypto_audit_state();
        wasm.game.shuffle_player_library(alice);
        wasm.update_crypto_requirements_from(before);

        assert!(wasm.last_crypto_requirements.iter().any(|requirement| {
            requirement.requirement_type == "verifiable_shuffle"
                && requirement.owner == alice.index() as u8
                && requirement.zone == "library"
                && requirement
                    .before_order
                    .as_ref()
                    .is_some_and(|order| order.len() == 3)
                && requirement
                    .after_order
                    .as_ref()
                    .is_some_and(|order| order.len() == 3)
        }));
    }

    #[test]
    fn crypto_requirements_include_hidden_order_update_for_nonrandom_library_reorder() {
        let mut wasm = WasmGame::new();
        let alice = PlayerId::from_index(0);
        let bottom =
            wasm.game
                .create_hidden_card_placeholder(alice, Zone::Library, 0, "slot-0".to_string());
        let top =
            wasm.game
                .create_hidden_card_placeholder(alice, Zone::Library, 1, "slot-1".to_string());

        let before = wasm.capture_crypto_audit_state();
        wasm.game.set_player_library_order_with_audit(
            alice,
            vec![top, bottom],
            "test nonrandom reorder",
        );
        wasm.update_crypto_requirements_from(before);

        assert!(wasm.last_crypto_requirements.iter().any(|requirement| {
            requirement.requirement_type == "hidden_order_update"
                && requirement.owner == alice.index() as u8
                && requirement.zone == "library"
                && requirement.before_order == Some(vec![bottom.0, top.0])
                && requirement.after_order == Some(vec![top.0, bottom.0])
        }));
        assert!(!wasm.last_crypto_requirements.iter().any(|requirement| {
            requirement.requirement_type == "verifiable_shuffle"
                && requirement.owner == alice.index() as u8
        }));
    }

    #[test]
    fn crypto_requirements_for_search_shuffle_exclude_card_put_on_top() {
        let mut wasm = WasmGame::new();
        let alice = PlayerId::from_index(0);
        let card_a =
            wasm.game
                .create_hidden_card_placeholder(alice, Zone::Library, 0, "slot-0".to_string());
        let card_b =
            wasm.game
                .create_hidden_card_placeholder(alice, Zone::Library, 1, "slot-1".to_string());
        let card_c =
            wasm.game
                .create_hidden_card_placeholder(alice, Zone::Library, 2, "slot-2".to_string());
        let tutored =
            wasm.game
                .create_hidden_card_placeholder(alice, Zone::Library, 3, "slot-3".to_string());

        let before = wasm.capture_crypto_audit_state();
        assert!(wasm.game.shuffle_library_except_then_put_on_top(
            alice,
            &[tutored],
            "searched card put on top after library shuffle",
        ));
        wasm.update_crypto_requirements_from(before);

        let shuffle = wasm
            .last_crypto_requirements
            .iter()
            .find(|requirement| requirement.requirement_type == "verifiable_shuffle")
            .expect("search shuffle should require ziffle proof");
        assert_eq!(shuffle.count, Some(3));
        assert_eq!(
            shuffle.before_order.as_ref().map(Vec::len),
            Some(3),
            "the searched card is not part of the randomized library subset"
        );
        assert_eq!(shuffle.after_order.as_ref().map(Vec::len), Some(3));
        let before_ids = shuffle.before_order.as_ref().expect("before order");
        assert!(before_ids.contains(&card_a.0));
        assert!(before_ids.contains(&card_b.0));
        assert!(before_ids.contains(&card_c.0));
        assert!(!before_ids.contains(&tutored.0));
        assert!(wasm.last_crypto_requirements.iter().any(|requirement| {
            requirement.requirement_type == "hidden_order_update"
                && requirement
                    .before_order
                    .as_ref()
                    .is_some_and(|order| order.len() == 4)
                && requirement
                    .after_order
                    .as_ref()
                    .is_some_and(|order| order.len() == 4)
        }));
    }

    #[test]
    fn crypto_requirements_for_search_shuffle_support_card_third_from_top() {
        let mut wasm = WasmGame::new();
        let alice = PlayerId::from_index(0);
        let _card_a =
            wasm.game
                .create_hidden_card_placeholder(alice, Zone::Library, 0, "slot-0".to_string());
        let _card_b =
            wasm.game
                .create_hidden_card_placeholder(alice, Zone::Library, 1, "slot-1".to_string());
        let _card_c =
            wasm.game
                .create_hidden_card_placeholder(alice, Zone::Library, 2, "slot-2".to_string());
        let tutored =
            wasm.game
                .create_hidden_card_placeholder(alice, Zone::Library, 3, "slot-3".to_string());

        let before = wasm.capture_crypto_audit_state();
        assert!(wasm.game.shuffle_library_except_then_insert_from_top(
            alice,
            &[tutored],
            3,
            "searched card put third from top after library shuffle",
        ));
        wasm.update_crypto_requirements_from(before);

        let shuffle = wasm
            .last_crypto_requirements
            .iter()
            .find(|requirement| requirement.requirement_type == "verifiable_shuffle")
            .expect("search shuffle should require ziffle proof");
        assert_eq!(shuffle.count, Some(3));
        assert!(
            !shuffle
                .before_order
                .as_ref()
                .expect("before order")
                .contains(&tutored.0)
        );

        let order_update = wasm
            .last_crypto_requirements
            .iter()
            .find(|requirement| requirement.requirement_type == "hidden_order_update")
            .expect("searched-card placement should require commitment remapping");
        let after_order = order_update.after_order.as_ref().expect("after order");
        assert_eq!(after_order[after_order.len() - 3], tutored.0);
    }

    #[test]
    fn public_view_requirements_stay_scoped_to_each_revealing_player() {
        let mut wasm = WasmGame::new();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let alice_top = wasm.game.create_hidden_card_placeholder(
            alice,
            Zone::Library,
            3,
            "alice-parley-top".to_string(),
        );
        let bob_top = wasm.game.create_hidden_card_placeholder(
            bob,
            Zone::Library,
            9,
            "bob-parley-top".to_string(),
        );
        let source = ObjectId::from_raw(77);
        let description = "Parley reveal".to_string();
        let alice_view = ironsmith::decisions::context::ViewCardsContext::new(
            alice,
            alice,
            Some(source),
            Zone::Library,
            description.clone(),
        )
        .with_public(true);
        let bob_view = ironsmith::decisions::context::ViewCardsContext::new(
            alice,
            bob,
            Some(source),
            Zone::Library,
            description,
        )
        .with_public(true);

        merge_active_viewed_cards(
            &wasm.game,
            &mut wasm.active_viewed_cards,
            alice,
            &[alice_top],
            &alice_view,
        );
        merge_active_viewed_cards(
            &wasm.game,
            &mut wasm.active_viewed_cards,
            alice,
            &[bob_top],
            &bob_view,
        );
        merge_audit_viewed_cards(
            &wasm.game,
            &mut wasm.active_audit_viewed_cards,
            alice,
            &[alice_top],
            &alice_view,
        );
        merge_audit_viewed_cards(
            &wasm.game,
            &mut wasm.active_audit_viewed_cards,
            alice,
            &[bob_top],
            &bob_view,
        );

        assert_eq!(
            wasm.active_viewed_cards
                .as_ref()
                .map(|view| view.cards.len()),
            Some(2),
            "the UI should still get one combined revealed-card strip"
        );
        assert_eq!(
            wasm.active_audit_viewed_cards.len(),
            2,
            "audit view windows must stay owner-scoped"
        );

        let before = wasm.capture_crypto_audit_state();
        wasm.update_crypto_requirements_from(before);

        assert!(wasm.last_crypto_requirements.iter().any(|requirement| {
            requirement.requirement_type == "public_view_window"
                && requirement.owner == alice.index() as u8
                && requirement.zone == "library"
                && requirement.count == Some(1)
        }));
        assert!(wasm.last_crypto_requirements.iter().any(|requirement| {
            requirement.requirement_type == "public_view_window"
                && requirement.owner == bob.index() as u8
                && requirement.zone == "library"
                && requirement.count == Some(1)
        }));
        assert!(wasm.last_crypto_requirements.iter().any(|requirement| {
            requirement.requirement_type == "public_open"
                && requirement.owner == alice.index() as u8
                && requirement.object_id == Some(alice_top.0)
                && requirement.commitment.as_deref() == Some("alice-parley-top")
        }));
        assert!(wasm.last_crypto_requirements.iter().any(|requirement| {
            requirement.requirement_type == "public_open"
                && requirement.owner == bob.index() as u8
                && requirement.object_id == Some(bob_top.0)
                && requirement.commitment.as_deref() == Some("bob-parley-top")
        }));
    }
}

#[cfg(all(test, target_arch = "wasm32"))]
mod tests;
