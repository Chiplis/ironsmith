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
use ironsmith::decisions::context::DecisionContext;
use ironsmith::game_loop::{
    ActivationStage, CastStage, PendingPriorityContinuation, PriorityActionPerfMetrics,
    PriorityAdvancePerfMetrics, PriorityLoopState, PriorityResponse, advance_priority_with_dm,
    apply_decision_context_with_dm, apply_priority_response_with_dm, last_priority_action_perf,
    last_priority_advance_perf,
};
use ironsmith::game_state::{GameState, StackEntry, Target};
use ironsmith::ids::{CardId, ObjectId, PlayerId, restore_id_counters, snapshot_id_counters};
use ironsmith::mana::{ManaCost, ManaSymbol};
use ironsmith::targeting::{normalize_targets_for_requirements, validate_flat_target_assignment};
use ironsmith::triggers::TriggerQueue;
use ironsmith::types::{CardType, Subtype};
use ironsmith::zone::Zone;

mod ui_snapshot;
use ui_snapshot::{GameSnapshot, battlefield_transition_snapshots, build_object_details_snapshot};

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
            let lines = ironsmith::compiled_text::compiled_lines(&o.to_card_definition());
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
    current: &mut Option<ActiveViewedCards>,
    viewer: PlayerId,
    cards: &[ObjectId],
    ctx: &ironsmith::decisions::context::ViewCardsContext,
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
        for snapshot in hidden {
            if snapshot.owner == first_owner
                && snapshot.zone == first_zone
                && !cards.contains(&snapshot.object_id)
            {
                cards.push(snapshot.object_id);
            }
        }

        if !cards.is_empty() {
            return Some(ActiveViewedCards {
                viewer: entry.controller,
                subject: first_owner,
                zone: first_zone,
                cards,
                public: true,
                source: Some(entry.object_id),
                description: "Revealed while on the stack".to_string(),
            });
        }
    }

    None
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
    entry: &ironsmith::game_state::StackEntry,
    obj: Option<&ironsmith::object::Object>,
) -> Option<String> {
    let wants_triggered = entry.triggering_event.is_some();

    if let Some(source_obj) = obj {
        let ability_texts: Vec<String> = source_obj
            .abilities
            .iter()
            .filter_map(|ability| match (&ability.kind, wants_triggered) {
                (ironsmith::ability::AbilityKind::Triggered(_), true) => ability.text.clone(),
                (ironsmith::ability::AbilityKind::Activated(_), false) => ability.text.clone(),
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

        let compiled_lines =
            ironsmith::compiled_text::compiled_lines(&source_obj.to_card_definition());
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
            (ironsmith::ability::AbilityKind::Triggered(_), true) => ability.text.clone(),
            (ironsmith::ability::AbilityKind::Activated(_), false) => ability.text.clone(),
            _ => None,
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
    fn from_context(
        game: &GameState,
        ctx: &DecisionContext,
        perspective: PlayerId,
        viewed_cards: Option<&ActiveViewedCards>,
    ) -> Self {
        let enriched_ctx = ironsmith::decisions::context::enrich_display_hints(game, ctx.clone());
        let ctx = &enriched_ctx;
        let decision_object_visible = |id| {
            object_visible_to_perspective(game, perspective, viewed_cards, id)
                || decision_exposes_object_to_perspective(Some(ctx), perspective, id)
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
            DecisionContext::TextInput(text) => DecisionView::TextInput {
                player: text.player.0,
                description: text.description.clone(),
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
                                (opt.repeatable, opt.max_count)
                            };
                            let visible_object_id = opt
                                .object_id
                                .and_then(|id| decision_object_visible(id).then_some(id));
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
                        repeatable: modes.spec.allow_repeated_modes,
                        max_count: Some(modes.spec.max_modes.min(u32::MAX as usize) as u32),
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
                        description: if decision_object_visible(*object_id) {
                            name.clone()
                        } else {
                            hidden_object_label()
                        },
                        legal: true,
                        repeatable: false,
                        max_count: Some(1),
                        object_id: decision_object_visible(*object_id).then_some(object_id.0),
                        object_controller: decision_object_visible(*object_id)
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
                allow_partial_completion: false,
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
        self.capture_once(ironsmith::decisions::context::enrich_display_hints(
            game, ctx,
        ));
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
        _game: &GameState,
        viewer: PlayerId,
        cards: &[ObjectId],
        ctx: &ironsmith::decisions::context::ViewCardsContext,
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

mod wasm_game_impl;
use wasm_game_impl::*;

#[cfg(all(test, target_arch = "wasm32"))]
mod tests;
