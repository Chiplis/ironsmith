//! WASM-facing API for browser integration.
//!
//! This module provides a small wrapper around `GameState` so JavaScript can:
//! - create/reset a game
//! - mutate a bit of state
//! - read a serializable snapshot

use std::collections::{HashMap, HashSet, VecDeque};

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
    ActivationStage, CastStage, PriorityLoopState, PriorityResponse, advance_priority_with_dm,
    apply_priority_response_with_dm,
};
use crate::game_state::{GameState, Target};
use crate::ids::{ObjectId, PlayerId, restore_id_counters, snapshot_id_counters};
use crate::mana::ManaSymbol;
use crate::triggers::TriggerQueue;
use crate::types::CardType;
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
        .map(|(counter_type, amount)| (format!("{counter_type:?}"), *amount))
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
            let id = representative.map(|obj| obj.id.0).unwrap_or_default();
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
            PermanentSnapshot {
                id,
                name,
                tapped: key.tapped,
                count: member_ids.len().max(1),
                member_ids,
                lane: key.lane.as_str().to_string(),
                power_toughness,
            }
        })
        .collect();

    (snapshots, total)
}

#[derive(Debug, Clone, Serialize)]
struct PermanentSnapshot {
    id: u64,
    name: String,
    tapped: bool,
    count: usize,
    member_ids: Vec<u64>,
    lane: String,
    power_toughness: Option<String>,
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
    hand_cards: Vec<HandCardSnapshot>,
    graveyard_cards: Vec<ZoneCardSnapshot>,
    exile_cards: Vec<ZoneCardSnapshot>,
    library_top: Option<String>,
    graveyard_top: Option<String>,
    battlefield: Vec<PermanentSnapshot>,
    battlefield_total: usize,
}

#[derive(Debug, Clone, Serialize)]
struct HandCardSnapshot {
    id: u64,
    name: String,
}

#[derive(Debug, Clone, Serialize)]
struct ZoneCardSnapshot {
    id: u64,
    name: String,
}

#[derive(Debug, Clone, Serialize)]
struct StackObjectSnapshot {
    id: u64,
    name: String,
    effect_text: Option<String>,
    /// "Triggered", "Activated", or null for spells.
    ability_kind: Option<String>,
    /// Compiled text of the specific ability effects (for inspector display).
    ability_text: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct CounterSnapshot {
    kind: String,
    amount: u32,
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
}

#[derive(Debug, Clone, Serialize)]
struct GameSnapshot {
    perspective: u8,
    turn_number: u32,
    active_player: u8,
    priority_player: Option<u8>,
    phase: String,
    step: Option<String>,
    stack_size: usize,
    stack_preview: Vec<String>,
    stack_objects: Vec<StackObjectSnapshot>,
    battlefield_size: usize,
    exile_size: usize,
    players: Vec<PlayerSnapshot>,
    decision: Option<DecisionView>,
    game_over: Option<GameOverView>,
    /// True when the current decision chain can be cancelled (user-initiated
    /// action like casting a spell, NOT triggered ability resolution).
    cancelable: bool,
}

impl GameSnapshot {
    fn from_game(
        game: &GameState,
        perspective: PlayerId,
        decision: Option<&DecisionContext>,
        game_over: Option<&GameResult>,
        pending_cast_stack_id: Option<ObjectId>,
        cancelable: bool,
    ) -> Self {
        let protected_ids = protected_object_ids_for_decision(decision);
        let players = game
            .players
            .iter()
            .map(|p| {
                let (battlefield, battlefield_total) =
                    grouped_battlefield_for_player(game, p.id, &protected_ids);
                PlayerSnapshot {
                    can_view_hand: p.id == perspective,
                    hand_cards: if p.id == perspective {
                        p.hand
                            .iter()
                            .rev()
                            .take(12)
                            .filter_map(|id| game.object(*id))
                            .map(|o| HandCardSnapshot {
                                id: o.id.0,
                                name: o.name.clone(),
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
                            name: o.name.clone(),
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
                            name: o.name.clone(),
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
            .map(|entry| {
                let obj = game.object(entry.object_id);
                let name = obj
                    .map(|o| o.name.clone())
                    .or_else(|| entry.source_name.clone())
                    .unwrap_or_else(|| format!("Object#{}", entry.object_id.0));
                if entry.is_ability {
                    let ability_kind = if entry.triggering_event.is_some() {
                        "Triggered"
                    } else {
                        "Activated"
                    };
                    let ability_text = entry
                        .ability_effects
                        .as_ref()
                        .map(|effects| crate::compiled_text::compile_effect_list(effects))
                        .filter(|s| !s.is_empty());
                    StackObjectSnapshot {
                        id: entry.object_id.0,
                        name,
                        effect_text: None,
                        ability_kind: Some(ability_kind.to_string()),
                        ability_text,
                    }
                } else {
                    let effect_text = if let Some(o) = obj {
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
                        name,
                        effect_text,
                        ability_kind: None,
                        ability_text: None,
                    }
                }
            })
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
                    name: obj.name.clone(),
                    effect_text: pending_effect_text,
                    ability_kind: None,
                    ability_text: None,
                },
            );
            stack_size += 1;
        }
        Self {
            perspective: perspective.0,
            turn_number: game.turn.turn_number,
            active_player: game.turn.active_player.0,
            priority_player: game.turn.priority_player.map(|p| p.0),
            phase: format!("{:?}", game.turn.phase),
            step: game.turn.step.map(|step| format!("{:?}", step)),
            stack_size,
            stack_preview,
            stack_objects,
            battlefield_size: game.battlefield.len(),
            exile_size: game.exile.len(),
            players,
            decision: decision.map(|ctx| DecisionView::from_context(game, ctx)),
            game_over: game_over.map(|r| GameOverView::from_result(game, r)),
            cancelable,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
struct ActionView {
    index: usize,
    label: String,
    kind: String,
    object_id: Option<u64>,
    from_zone: Option<String>,
    to_zone: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct OptionView {
    index: usize,
    description: String,
    legal: bool,
    repeatable: bool,
    max_count: Option<u32>,
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
        source_name: Option<String>,
        reason: Option<String>,
    },
    SelectOptions {
        player: u8,
        description: String,
        min: usize,
        max: usize,
        options: Vec<OptionView>,
        source_name: Option<String>,
        reason: Option<String>,
    },
    SelectObjects {
        player: u8,
        description: String,
        min: usize,
        max: Option<usize>,
        candidates: Vec<ObjectChoiceView>,
        source_name: Option<String>,
        reason: Option<String>,
    },
    Targets {
        player: u8,
        context: String,
        requirements: Vec<TargetRequirementView>,
        source_name: Option<String>,
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
    fn from_context(game: &GameState, ctx: &DecisionContext) -> Self {
        let resolve_source_name = |source: Option<ObjectId>| -> Option<String> {
            source
                .and_then(|id| game.object(id))
                .map(|o| o.name.clone())
        };
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
                    },
                    OptionView {
                        index: 0,
                        description: "No".to_string(),
                        legal: true,
                        repeatable: false,
                        max_count: Some(1),
                    },
                ],
                source_name: resolve_source_name(boolean.source),
                reason: reason.clone(),
            },
            DecisionContext::Priority(priority) => DecisionView::Priority {
                player: priority.player.0,
                actions: priority
                    .actions
                    .iter()
                    .enumerate()
                    .map(|(index, action)| build_action_view(game, index, action))
                    .collect(),
            },
            DecisionContext::Number(number) => DecisionView::Number {
                player: number.player.0,
                description: number.description.clone(),
                min: number.min,
                max: number.max,
                is_x_value: number.is_x_value,
                source_name: resolve_source_name(number.source),
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
                            OptionView {
                                index: opt.index,
                                description: opt.description.clone(),
                                legal: opt.legal,
                                repeatable,
                                max_count,
                            }
                        })
                        .collect()
                },
                source_name: resolve_source_name(options.source),
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
                    })
                    .collect(),
                source_name: resolve_source_name(modes.source),
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
                    })
                    .collect(),
                source_name: resolve_source_name(hybrid.source),
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
                    .map(|(index, (_, name))| OptionView {
                        index,
                        description: name.clone(),
                        legal: true,
                        repeatable: false,
                        max_count: Some(1),
                    })
                    .collect(),
                source_name: resolve_source_name(order.source),
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
                    .map(|(index, target)| OptionView {
                        index,
                        description: target.name.clone(),
                        legal: true,
                        repeatable: true,
                        max_count: Some(distribute.total),
                    })
                    .collect(),
                source_name: resolve_source_name(distribute.source),
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
                        })
                        .collect(),
                    source_name: resolve_source_name(colors.source),
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
                            split_camel_case(&format!("{counter_type:?}"))
                        ),
                        legal: *available > 0,
                        repeatable: *available > 1,
                        max_count: Some(*available),
                    })
                    .collect(),
                source_name: resolve_source_name(counters.source),
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
                source_name: resolve_source_name(partition.source),
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
                    })
                    .chain(proliferate.eligible_players.iter().enumerate().map(
                        |(offset, (_, name))| OptionView {
                            index: proliferate.eligible_permanents.len() + offset,
                            description: format!("Player: {name}"),
                            legal: true,
                            repeatable: false,
                            max_count: Some(1),
                        },
                    ))
                    .collect(),
                source_name: resolve_source_name(proliferate.source),
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
                    .map(|obj| ObjectChoiceView {
                        id: obj.id.0,
                        name: obj.name.clone(),
                        legal: obj.legal,
                    })
                    .collect(),
                source_name: resolve_source_name(objects.source),
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
                            .map(|target| target_choice_view(game, target))
                            .collect(),
                    })
                    .collect(),
                source_name: resolve_source_name(Some(targets.source)),
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
        action_index: usize,
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
enum ReplayOutcome {
    NeedsDecision(DecisionContext),
    Complete(GameProgress),
}

#[derive(Debug)]
struct WasmReplayDecisionMaker {
    answers: VecDeque<ReplayDecisionAnswer>,
    pending_context: Option<DecisionContext>,
}

impl WasmReplayDecisionMaker {
    fn new(answers: &[ReplayDecisionAnswer]) -> Self {
        Self {
            answers: answers.iter().cloned().collect(),
            pending_context: None,
        }
    }

    fn capture_once(&mut self, ctx: DecisionContext) {
        if self.pending_context.is_none() {
            self.pending_context = Some(ctx);
        }
    }

    fn take_pending_context(self) -> Option<DecisionContext> {
        self.pending_context
    }
}

impl DecisionMaker for WasmReplayDecisionMaker {
    fn decide_boolean(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::BooleanContext,
    ) -> bool {
        match self.answers.front() {
            Some(ReplayDecisionAnswer::Boolean(value)) => {
                let value = *value;
                self.answers.pop_front();
                value
            }
            _ => {
                self.capture_once(DecisionContext::Boolean(ctx.clone()));
                false
            }
        }
    }

    fn decide_number(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::NumberContext,
    ) -> u32 {
        match self.answers.front() {
            Some(ReplayDecisionAnswer::Number(value)) => {
                let value = *value;
                self.answers.pop_front();
                value
            }
            _ => {
                self.capture_once(DecisionContext::Number(ctx.clone()));
                ctx.min
            }
        }
    }

    fn decide_objects(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::SelectObjectsContext,
    ) -> Vec<ObjectId> {
        match self.answers.front() {
            Some(ReplayDecisionAnswer::Objects(ids)) => {
                let ids = ids.clone();
                self.answers.pop_front();
                ids
            }
            _ => {
                self.capture_once(DecisionContext::SelectObjects(ctx.clone()));
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
        _game: &GameState,
        ctx: &crate::decisions::context::SelectOptionsContext,
    ) -> Vec<usize> {
        match self.answers.front() {
            Some(ReplayDecisionAnswer::Options(indices)) => {
                let indices = indices.clone();
                self.answers.pop_front();
                indices
            }
            _ => {
                self.capture_once(DecisionContext::SelectOptions(ctx.clone()));
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
        _game: &GameState,
        ctx: &crate::decisions::context::TargetsContext,
    ) -> Vec<Target> {
        match self.answers.front() {
            Some(ReplayDecisionAnswer::Targets(targets)) => {
                let targets = targets.clone();
                self.answers.pop_front();
                targets
            }
            _ => {
                self.capture_once(DecisionContext::Targets(ctx.clone()));
                ctx.requirements
                    .iter()
                    .filter(|requirement| requirement.min_targets > 0)
                    .filter_map(|requirement| requirement.legal_targets.first().cloned())
                    .collect()
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
        _game: &GameState,
        ctx: &crate::decisions::context::OrderContext,
    ) -> Vec<ObjectId> {
        match self.answers.front() {
            Some(ReplayDecisionAnswer::Order(order)) => {
                let order = order.clone();
                self.answers.pop_front();
                order
            }
            _ => {
                self.capture_once(DecisionContext::Order(ctx.clone()));
                ctx.items.iter().map(|(id, _)| *id).collect()
            }
        }
    }

    fn decide_distribute(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::DistributeContext,
    ) -> Vec<(Target, u32)> {
        match self.answers.front() {
            Some(ReplayDecisionAnswer::Distribute(distribution)) => {
                let distribution = distribution.clone();
                self.answers.pop_front();
                distribution
            }
            _ => {
                self.capture_once(DecisionContext::Distribute(ctx.clone()));
                Vec::new()
            }
        }
    }

    fn decide_colors(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::ColorsContext,
    ) -> Vec<crate::color::Color> {
        match self.answers.front() {
            Some(ReplayDecisionAnswer::Colors(colors)) => {
                let colors = colors.clone();
                self.answers.pop_front();
                colors
            }
            _ => {
                self.capture_once(DecisionContext::Colors(ctx.clone()));
                vec![crate::color::Color::Green; ctx.count as usize]
            }
        }
    }

    fn decide_counters(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::CountersContext,
    ) -> Vec<(crate::object::CounterType, u32)> {
        match self.answers.front() {
            Some(ReplayDecisionAnswer::Counters(counters)) => {
                let counters = counters.clone();
                self.answers.pop_front();
                counters
            }
            _ => {
                self.capture_once(DecisionContext::Counters(ctx.clone()));
                Vec::new()
            }
        }
    }

    fn decide_partition(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::PartitionContext,
    ) -> Vec<ObjectId> {
        match self.answers.front() {
            Some(ReplayDecisionAnswer::Partition(partition)) => {
                let partition = partition.clone();
                self.answers.pop_front();
                partition
            }
            _ => {
                self.capture_once(DecisionContext::Partition(ctx.clone()));
                Vec::new()
            }
        }
    }

    fn decide_proliferate(
        &mut self,
        _game: &GameState,
        ctx: &crate::decisions::context::ProliferateContext,
    ) -> crate::decisions::specs::ProliferateResponse {
        match self.answers.front() {
            Some(ReplayDecisionAnswer::Proliferate(response)) => {
                let response = response.clone();
                self.answers.pop_front();
                response
            }
            _ => {
                self.capture_once(DecisionContext::Proliferate(ctx.clone()));
                crate::decisions::specs::ProliferateResponse::default()
            }
        }
    }
}

/// Browser-exposed game handle.
#[wasm_bindgen]
pub struct WasmGame {
    game: GameState,
    registry: CardRegistry,
    registry_preload_cursor: usize,
    trigger_queue: TriggerQueue,
    priority_state: PriorityLoopState,
    pending_decision: Option<DecisionContext>,
    pending_replay_action: Option<PendingReplayAction>,
    game_over: Option<GameResult>,
    perspective: PlayerId,
    deck_generation_nonce: u64,
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
    /// Semantic similarity scores for each registered card (card name → score 0.0–1.0).
    semantic_scores: HashMap<String, f32>,
    /// User-configured minimum semantic threshold for card addition (0.0 = no filter).
    semantic_threshold: f32,
}

#[derive(Debug, Clone, Serialize)]
struct RegistryPreloadStatus {
    loaded: usize,
    cursor: usize,
    total: usize,
    done: bool,
}

#[wasm_bindgen(start)]
pub fn wasm_start() {
    console_error_panic_hook::set_once();
}

#[wasm_bindgen]
impl WasmGame {
    /// Construct a demo game with two players.
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        let mut priority_state = PriorityLoopState::new(2);
        priority_state.set_auto_choose_single_pip_payment(false);
        Self {
            game: GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20),
            registry: CardRegistry::new(),
            registry_preload_cursor: 0,
            trigger_queue: TriggerQueue::new(),
            priority_state,
            pending_decision: None,
            pending_replay_action: None,
            game_over: None,
            perspective: PlayerId::from_index(0),
            deck_generation_nonce: 0,
            runner: None,
            runner_awaiting_priority: false,
            runner_pending_decision: false,
            auto_cleanup_discard: true,
            priority_epoch_checkpoint: None,
            semantic_scores: HashMap::new(),
            semantic_threshold: 0.0,
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

        self.game = GameState::new(names, starting_life);
        self.load_demo_decks()?;
        self.draw_opening_hands(7)?;
        self.reset_runtime_state();
        self.recompute_ui_decision()?;
        Ok(())
    }

    /// Return a JS object snapshot of public game state.
    #[wasm_bindgen]
    pub fn snapshot(&self) -> Result<JsValue, JsValue> {
        let pending_cast_stack_id = self
            .priority_state
            .pending_cast
            .as_ref()
            .map(|p| p.stack_id);
        let cancelable = self.is_cancelable();
        let snap = GameSnapshot::from_game(
            &self.game,
            self.perspective,
            self.pending_decision.as_ref(),
            self.game_over.as_ref(),
            pending_cast_stack_id,
            cancelable,
        );
        serde_wasm_bindgen::to_value(&snap)
            .map_err(|e| JsValue::from_str(&format!("snapshot encode failed: {e}")))
    }

    /// Return the current UI state from the selected player perspective.
    #[wasm_bindgen(js_name = uiState)]
    pub fn ui_state(&self) -> Result<JsValue, JsValue> {
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
        let total = CardRegistry::generated_parser_entry_count();
        let cursor = self.registry_preload_cursor.min(total);
        let status = RegistryPreloadStatus {
            loaded: self.registry.len(),
            cursor,
            total,
            done: cursor >= total,
        };
        serde_wasm_bindgen::to_value(&status)
            .map_err(|e| JsValue::from_str(&format!("preloadRegistryStatus encode failed: {e}")))
    }

    /// Parse/register the next batch of generated cards for startup warmup.
    #[wasm_bindgen(js_name = preloadRegistryChunk)]
    pub fn preload_registry_chunk(&mut self, chunk_size: usize) -> Result<JsValue, JsValue> {
        let total = CardRegistry::generated_parser_entry_count();
        if self.registry_preload_cursor < total {
            self.registry_preload_cursor = self
                .registry
                .preload_generated_cards_chunk(self.registry_preload_cursor, chunk_size);

            // Score any newly registered cards.
            for def in self.registry.all() {
                let name = def.name();
                if self.semantic_scores.contains_key(name) {
                    continue;
                }
                let score = Self::compute_card_score(def);
                self.semantic_scores.insert(name.to_string(), score);
            }
        }
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
    pub fn snapshot_json(&self) -> Result<String, JsValue> {
        let pending_cast_stack_id = self
            .priority_state
            .pending_cast
            .as_ref()
            .map(|p| p.stack_id);
        let cancelable = self.is_cancelable();
        let snap = GameSnapshot::from_game(
            &self.game,
            self.perspective,
            self.pending_decision.as_ref(),
            self.game_over.as_ref(),
            pending_cast_stack_id,
            cancelable,
        );
        serde_json::to_string_pretty(&snap)
            .map_err(|e| JsValue::from_str(&format!("json encode failed: {e}")))
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
        let definition = self
            .find_card_definition(query)
            .cloned()
            .ok_or_else(|| {
                match crate::cards::CardRegistry::try_compile_card(query) {
                    Ok(_) => JsValue::from_str(&format!("unknown card name: {query}")),
                    Err(err) => JsValue::from_str(&err),
                }
            })?;

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
        let definition = self
            .find_card_definition(query)
            .cloned()
            .ok_or_else(|| {
                match crate::cards::CardRegistry::try_compile_card(query) {
                    Ok(_) => JsValue::from_str(&format!("unknown card name: {query}")),
                    Err(err) => JsValue::from_str(&err),
                }
            })?;

        if self.semantic_threshold > 0.0 {
            let score = self
                .semantic_scores
                .get(definition.name())
                .copied()
                .unwrap_or_else(|| Self::compute_card_score(&definition));
            if score < self.semantic_threshold {
                return Err(JsValue::from_str(&format!(
                    "Card '{}' has fidelity {:.0}%, below threshold {:.0}%",
                    definition.name(),
                    score * 100.0,
                    self.semantic_threshold * 100.0,
                )));
            }
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
            let object_id = self.game.move_object(temp_id, zone).unwrap_or(temp_id);
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
        self.game = GameState::new(names, starting_life);

        let player_ids: Vec<PlayerId> = self.game.players.iter().map(|p| p.id).collect();

        for player_id in player_ids {
            // 60-card random deck with 24-land color-aware basic manabase.
            let deck = self.build_random_demo_deck_names(60, 24)?;
            self.populate_player_library(player_id, &deck)?;
        }

        self.reset_runtime_state();
        self.recompute_ui_decision()?;
        Ok(())
    }

    /// Load explicit decks by card name. JS format: `string[][]`.
    ///
    /// Deck list index maps to player index.
    /// Returns a JSON object: `{ loaded: number, failed: string[] }`.
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
        self.game = GameState::new(names, starting_life);

        let mut loaded: u32 = 0;
        let mut failed: Vec<String> = Vec::new();

        let player_ids: Vec<PlayerId> = self.game.players.iter().map(|p| p.id).collect();
        for (player_id, deck) in player_ids.into_iter().zip(decks.iter()) {
            self.registry
                .ensure_cards_loaded(deck.iter().map(|name| name.as_str()));

            for name in deck {
                if let Some(definition) = self.find_card_definition(name).cloned() {
                    self.game.create_object_from_definition(
                        &definition,
                        player_id,
                        crate::zone::Zone::Library,
                    );
                    loaded += 1;
                } else {
                    failed.push(name.clone());
                }
            }

            if let Some(player) = self.game.player_mut(player_id) {
                player.shuffle_library();
            }
        }

        self.reset_runtime_state();
        self.recompute_ui_decision()?;

        // Return { loaded, failed } to JS
        let result = js_sys::Object::new();
        js_sys::Reflect::set(
            &result,
            &JsValue::from_str("loaded"),
            &JsValue::from(loaded),
        )
        .ok();
        let failed_arr = js_sys::Array::new();
        for name in &failed {
            failed_arr.push(&JsValue::from_str(name));
        }
        js_sys::Reflect::set(&result, &JsValue::from_str("failed"), &failed_arr).ok();
        Ok(result.into())
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

    /// Set the semantic similarity threshold for card addition (0.0 = off).
    #[wasm_bindgen(js_name = setSemanticThreshold)]
    pub fn set_semantic_threshold(&mut self, threshold: f32) {
        self.semantic_threshold = threshold.clamp(0.0, 1.0);
    }

    /// Get the current semantic threshold.
    #[wasm_bindgen(js_name = getSemanticThreshold)]
    pub fn get_semantic_threshold(&self) -> f32 {
        self.semantic_threshold
    }

    /// Get the semantic score for a specific card. Returns -1.0 if not scored yet.
    #[wasm_bindgen(js_name = getCardSemanticScore)]
    pub fn get_card_semantic_score(&self, card_name: &str) -> f32 {
        self.semantic_scores
            .get(card_name)
            .or_else(|| {
                let lower = card_name.trim().to_lowercase();
                self.semantic_scores
                    .iter()
                    .find(|(k, _)| k.to_lowercase() == lower)
                    .map(|(_, v)| v)
            })
            .copied()
            .unwrap_or(-1.0)
    }

    /// Get the count of registered cards meeting the current threshold.
    #[wasm_bindgen(js_name = cardsMeetingThreshold)]
    pub fn cards_meeting_threshold(&self) -> usize {
        if self.semantic_threshold <= 0.0 {
            return self.registry.len();
        }
        let t = self.semantic_threshold;
        self.semantic_scores.values().filter(|&&s| s >= t).count()
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
    /// 1. The active replay-action checkpoint (start of this user action chain).
    /// 2. The priority-epoch checkpoint (start of this priority round).
    ///
    /// This mirrors "take back this action chain" behavior first, while still
    /// preserving the broader epoch rollback as a fallback.
    #[wasm_bindgen(js_name = cancelDecision)]
    pub fn cancel_decision(&mut self) -> Result<JsValue, JsValue> {
        if let Some(checkpoint) = self
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
        self.recompute_ui_decision()?;
        self.snapshot()
    }

    /// Apply a player command for the currently pending decision.
    #[wasm_bindgen]
    pub fn dispatch(&mut self, command: JsValue) -> Result<JsValue, JsValue> {
        let command: UiCommand = serde_wasm_bindgen::from_value(command)
            .map_err(|e| JsValue::from_str(&format!("invalid command payload: {e}")))?;

        let pending_ctx = self
            .pending_decision
            .take()
            .ok_or_else(|| JsValue::from_str("no pending decision to dispatch"))?;

        // If this decision came from the TurnRunner, route through runner.respond_*()
        if self.runner_pending_decision {
            self.runner_pending_decision = false;
            return self.dispatch_runner_decision(pending_ctx, command);
        }

        if let Some(mut replay) = self.pending_replay_action.take() {
            let answer = match self.command_to_replay_answer(&pending_ctx, command) {
                Ok(answer) => answer,
                Err(err) => {
                    self.pending_decision = Some(pending_ctx);
                    self.pending_replay_action = Some(replay);
                    return Err(err);
                }
            };
            replay.nested_answers.push(answer);

            let outcome = match self.execute_with_replay(
                &replay.checkpoint,
                &replay.root,
                &replay.nested_answers,
            ) {
                Ok(outcome) => outcome,
                Err(err) => {
                    self.pending_decision = Some(pending_ctx);
                    self.pending_replay_action = Some(replay);
                    return Err(err);
                }
            };

            match outcome {
                ReplayOutcome::NeedsDecision(next_ctx) => {
                    self.pending_decision = Some(next_ctx);
                    self.pending_replay_action = Some(replay);
                    self.snapshot()
                }
                ReplayOutcome::Complete(progress) => {
                    if let GameProgress::NeedsDecisionCtx(next_ctx) = progress {
                        // Follow-up contexts produced directly by GameProgress
                        // must be handled by normal dispatch, not replay.
                        self.pending_decision = Some(next_ctx);
                        self.pending_replay_action = None;
                        self.snapshot()
                    } else {
                        self.pending_replay_action = None;
                        self.apply_progress(progress)?;
                        self.snapshot()
                    }
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
                    self.pending_decision = Some(next_ctx);
                    self.pending_replay_action = Some(PendingReplayAction {
                        checkpoint,
                        root,
                        nested_answers: Vec::new(),
                    });
                    self.snapshot()
                }
                ReplayOutcome::Complete(progress) => {
                    if let GameProgress::NeedsDecisionCtx(next_ctx) = progress {
                        // Follow-up contexts produced directly by GameProgress
                        // must be handled by normal dispatch, not replay.
                        self.pending_decision = Some(next_ctx);
                        self.pending_replay_action = None;
                        self.snapshot()
                    } else {
                        self.pending_replay_action = None;
                        self.apply_progress(progress)?;
                        self.snapshot()
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
    /// - hidden-information/library changes that cannot be safely reversed
    ///   (shuffle/reorder, draw/mill/exile-from-library, etc.) disable cancel.
    fn is_cancelable(&self) -> bool {
        if let Some(replay) = self.pending_replay_action.as_ref() {
            return self.is_replay_chain_cancelable(replay);
        }

        let Some(epoch) = self.priority_epoch_checkpoint.as_ref() else {
            return false;
        };

        let has_pending_action_chain = self.priority_state.pending_cast.is_some()
            || self.priority_state.pending_activation.is_some()
            || self.priority_state.pending_method_selection.is_some()
            || self.priority_state.pending_mana_ability.is_some();

        has_pending_action_chain && !self.has_irreversible_library_change_since(epoch)
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

        !self.has_irreversible_library_change_since(&replay.checkpoint)
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

    /// Compute the semantic similarity score for a card definition.
    fn compute_card_score(def: &CardDefinition) -> f32 {
        let oracle = &def.card.oracle_text;
        if oracle.is_empty() {
            return 1.0;
        }
        let compiled = crate::compiled_text::compiled_lines(def);
        let (_oracle_cov, _compiled_cov, similarity, _delta, _mismatch) =
            crate::semantic_compare::compare_semantics_scored(oracle, &compiled, None);
        similarity
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
            || !def.additional_cost_effects().is_empty()
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
            self.registry.ensure_cards_loaded([candidate.as_str()]);
            let Some(def) = self.registry.get(candidate.as_str()) else {
                continue;
            };
            // Skip cards below the semantic fidelity threshold.
            if self.semantic_threshold > 0.0 {
                let score = self
                    .semantic_scores
                    .get(def.name())
                    .copied()
                    .unwrap_or_else(|| Self::compute_card_score(def));
                if score < self.semantic_threshold {
                    continue;
                }
            }
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
            spells.extend(
                spell_pool
                    .iter()
                    .take(spells_needed)
                    .cloned(),
            );
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
        self.deck_generation_nonce = self.deck_generation_nonce.wrapping_add(1);
        let random_bits = (js_sys::Math::random() * (u64::MAX as f64)) as u64;
        random_bits ^ self.deck_generation_nonce.rotate_left(17)
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

        if let Some(player) = self.game.player_mut(player_id) {
            player.shuffle_library();
        }
        Ok(())
    }

    fn find_card_definition(&self, query: &str) -> Option<&CardDefinition> {
        self.registry.get(query).or_else(|| {
            self.registry
                .all()
                .find(|def| def.name().eq_ignore_ascii_case(query))
        })
    }

    fn reset_runtime_state(&mut self) {
        self.trigger_queue = TriggerQueue::new();
        self.priority_state = PriorityLoopState::new(self.game.players.len());
        self.priority_state
            .set_auto_choose_single_pip_payment(false);
        self.pending_decision = None;
        self.pending_replay_action = None;
        self.priority_epoch_checkpoint = None;
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
        if self.game_over.is_some() {
            return Ok(());
        }
        self.advance_until_decision()
    }

    fn advance_until_decision(&mut self) -> Result<(), JsValue> {
        use crate::game_state::Step;
        use crate::turn_runner::TurnAction;

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
                        // Auto-resolve cleanup discards when the flag is set.
                        if self.auto_cleanup_discard {
                            if let DecisionContext::SelectObjects(ref obj) = ctx {
                                if self.game.turn.step == Some(Step::Cleanup) && obj.min > 0 {
                                    use rand::seq::SliceRandom;
                                    let mut ids: Vec<_> = obj
                                        .candidates
                                        .iter()
                                        .filter(|c| c.legal)
                                        .map(|c| c.id)
                                        .collect();
                                    ids.shuffle(&mut rand::rng());
                                    ids.truncate(obj.min);
                                    self.runner
                                        .as_mut()
                                        .unwrap()
                                        .respond_discard(ids);
                                    continue;
                                }
                            }
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
                        self.pending_decision = Some(ctx);
                        self.runner_pending_decision = false;
                        return Ok(());
                    }
                    GameProgress::Continue => {
                        // Priority loop ended - notify runner
                        self.runner.as_mut().unwrap().priority_done();
                        self.runner_awaiting_priority = false;
                        self.priority_epoch_checkpoint = None;
                        self.pending_decision = None;
                        continue;
                    }
                    GameProgress::StackResolved => {
                        // New priority round after resolution — fresh epoch.
                        self.priority_epoch_checkpoint = None;
                        continue;
                    }
                    GameProgress::GameOver(result) => {
                        self.pending_decision = None;
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
                self.pending_decision = Some(ctx);
                Ok(())
            }
            GameProgress::Continue => {
                // Priority loop ended - notify runner and continue
                if self.runner.is_some() {
                    self.runner.as_mut().unwrap().priority_done();
                    self.runner_awaiting_priority = false;
                }
                self.priority_epoch_checkpoint = None;
                self.pending_decision = None;
                self.advance_until_decision()
            }
            GameProgress::GameOver(result) => {
                self.pending_decision = None;
                self.game_over = Some(result);
                Ok(())
            }
            GameProgress::StackResolved => {
                self.priority_epoch_checkpoint = None;
                self.pending_decision = None;
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
                self.runner
                    .as_mut()
                    .unwrap()
                    .respond_attackers(converted);
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
                self.runner
                    .as_mut()
                    .unwrap()
                    .respond_discard(cards);
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

    fn capture_replay_checkpoint(&self) -> ReplayCheckpoint {
        ReplayCheckpoint {
            game: self.game.clone(),
            trigger_queue: self.trigger_queue.clone(),
            priority_state: self.priority_state.clone(),
            game_over: self.game_over.clone(),
            id_counters: snapshot_id_counters(),
        }
    }

    fn restore_replay_checkpoint(&mut self, checkpoint: &ReplayCheckpoint) {
        restore_id_counters(checkpoint.id_counters);
        self.game = checkpoint.game.clone();
        self.trigger_queue = checkpoint.trigger_queue.clone();
        self.priority_state = checkpoint.priority_state.clone();
        self.game_over = checkpoint.game_over.clone();
    }

    fn execute_with_replay(
        &mut self,
        checkpoint: &ReplayCheckpoint,
        root: &ReplayRoot,
        nested_answers: &[ReplayDecisionAnswer],
    ) -> Result<ReplayOutcome, JsValue> {
        self.restore_replay_checkpoint(checkpoint);

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

        if let Some(next_ctx) = replay_dm.take_pending_context() {
            self.restore_replay_checkpoint(checkpoint);
            return Ok(ReplayOutcome::NeedsDecision(next_ctx));
        }

        match result {
            Ok(progress) => Ok(ReplayOutcome::Complete(progress)),
            Err(e) => {
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
            (DecisionContext::Priority(priority), UiCommand::PriorityAction { action_index }) => {
                let action =
                    resolve_priority_action_by_index(priority, action_index).ok_or_else(|| {
                        JsValue::from_str(&format!("invalid priority action index: {action_index}"))
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
                            split_camel_case(&format!("{counter_type:?}")),
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
            (DecisionContext::Priority(priority), UiCommand::PriorityAction { action_index }) => {
                let action =
                    resolve_priority_action_by_index(priority, action_index).ok_or_else(|| {
                        JsValue::from_str(&format!("invalid priority action index: {action_index}"))
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
                    .is_some_and(|pending| matches!(pending.stage, CastStage::ChoosingCardCost))
                {
                    Ok(PriorityResponse::CardCostChoice(ObjectId::from_raw(chosen)))
                } else {
                    let cast_stage = self
                        .priority_state
                        .pending_cast
                        .as_ref()
                        .map(|p| format!("{:?}", p.stage));
                    let act_stage = self
                        .priority_state
                        .pending_activation
                        .as_ref()
                        .map(|p| format!("{:?}", p.stage));
                    Err(JsValue::from_str(&format!(
                        "unsupported SelectObjects context in priority flow \
                         (pending_cast={cast_stage:?}, pending_activation={act_stage:?})"
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
            (_, _) => Err(JsValue::from_str(
                "pending decision type is not yet supported in WASM dispatch",
            )),
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
            .map(|p| format!("{:?}", p.stage));
        let act_stage = self
            .priority_state
            .pending_activation
            .as_ref()
            .map(|p| format!("{:?}", p.stage));
        Err(JsValue::from_str(&format!(
            "unsupported SelectOptions context in priority flow \
             (pending_cast={cast_stage:?}, pending_activation={act_stage:?}, \
             pending_mana_ability={}, pending_method={}, replacement={})",
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
    let counters = obj
        .counters
        .iter()
        .map(|(kind, amount)| CounterSnapshot {
            kind: split_camel_case(&format!("{kind:?}")),
            amount: *amount,
        })
        .collect();

    Some(ObjectDetailsSnapshot {
        id: obj.id.0,
        stable_id: obj.stable_id.0.0,
        name: obj.name.clone(),
        kind: format!("{:?}", obj.kind),
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

fn split_camel_case(value: &str) -> String {
    let mut output = String::with_capacity(value.len() + 8);
    let mut previous_lower = false;
    for ch in value.chars() {
        if ch.is_ascii_uppercase() && previous_lower {
            output.push(' ');
        }
        previous_lower = ch.is_ascii_lowercase() || ch.is_ascii_digit();
        output.push(ch);
    }
    output
}

fn build_action_view(game: &GameState, index: usize, action: &LegalAction) -> ActionView {
    let (kind, object_id, from_zone, to_zone) = action_drag_metadata(action);
    ActionView {
        index,
        label: describe_action(game, action),
        kind: kind.to_string(),
        object_id,
        from_zone,
        to_zone,
    }
}

fn action_drag_metadata(
    action: &LegalAction,
) -> (&'static str, Option<u64>, Option<String>, Option<String>) {
    match action {
        LegalAction::PassPriority => ("pass_priority", None, None, None),
        LegalAction::PlayLand { land_id } => (
            "play_land",
            Some(land_id.0),
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
            Some(zone_name(*from_zone)),
            Some(zone_name(Zone::Stack)),
        ),
        LegalAction::ActivateAbility { source, .. } => (
            "activate_ability",
            Some(source.0),
            Some(zone_name(Zone::Battlefield)),
            Some(zone_name(Zone::Stack)),
        ),
        LegalAction::ActivateManaAbility { source, .. } => (
            "activate_mana_ability",
            Some(source.0),
            Some(zone_name(Zone::Battlefield)),
            None,
        ),
        LegalAction::TurnFaceUp { creature_id } => (
            "turn_face_up",
            Some(creature_id.0),
            Some(zone_name(Zone::Battlefield)),
            Some(zone_name(Zone::Battlefield)),
        ),
        LegalAction::SpecialAction(_) => ("special_action", None, None, None),
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
                            .and_then(|obj| obj.alternative_casts.get(*index))
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
                .object(*source)
                .and_then(|obj| obj.abilities.get(*ability_index))
                .and_then(|ability| ability.text.as_deref())
                .map(|text| truncate_text(text, 80));
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
                .object(*source)
                .and_then(|obj| obj.abilities.get(*ability_index))
                .and_then(|ability| ability.text.as_deref())
                .map(|text| truncate_text(text, 80));
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
            crate::special_actions::SpecialAction::Suspend { card_id } => {
                format!("Suspend {}", object_name(game, *card_id))
            }
            crate::special_actions::SpecialAction::Foretell { card_id } => {
                format!("Foretell {}", object_name(game, *card_id))
            }
            _ => format!("Special action: {:?}", action),
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

fn resolve_priority_action_by_index(
    priority: &crate::decisions::context::PriorityContext,
    action_index: usize,
) -> Option<LegalAction> {
    priority.actions.get(action_index).cloned()
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

fn truncate_text(text: &str, max_len: usize) -> String {
    if text.len() <= max_len {
        text.to_string()
    } else {
        let mut end = max_len.saturating_sub(3).min(text.len());
        while end > 0 && !text.is_char_boundary(end) {
            end -= 1;
        }
        format!("{}...", &text[..end])
    }
}

fn target_choice_view(game: &GameState, target: &Target) -> TargetChoiceView {
    match target {
        Target::Player(pid) => TargetChoiceView::Player {
            player: pid.0,
            name: game
                .player(*pid)
                .map(|p| p.name.clone())
                .unwrap_or_else(|| format!("Player {}", pid.0 + 1)),
        },
        Target::Object(id) => TargetChoiceView::Object {
            object: id.0,
            name: object_name(game, *id),
        },
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
/// - Total count meets the aggregate min across all requirements
/// - Total count does not exceed the aggregate max
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

    // Aggregate min/max across requirements.
    let total_min: usize = ctx.requirements.iter().map(|r| r.min_targets).sum();
    let total_max: Option<usize> = {
        let mut sum: usize = 0;
        let mut unbounded = false;
        for req in &ctx.requirements {
            match req.max_targets {
                Some(max) => sum = sum.saturating_add(max),
                None => {
                    unbounded = true;
                    break;
                }
            }
        }
        if unbounded { None } else { Some(sum) }
    };

    if converted.len() < total_min {
        return Err(JsValue::from_str(&format!(
            "must select at least {total_min} target(s), got {}",
            converted.len()
        )));
    }
    if let Some(max) = total_max {
        if converted.len() > max {
            return Err(JsValue::from_str(&format!(
                "must select at most {max} target(s), got {}",
                converted.len()
            )));
        }
    }

    Ok(converted)
}

#[cfg(test)]
mod tests {
    use super::build_object_details_snapshot;
    use crate::cards::definitions::grizzly_bears;
    use crate::continuous::ContinuousEffect;
    use crate::effect::Until;
    use crate::game_state::GameState;
    use crate::ids::PlayerId;
    use crate::zone::Zone;

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
}
