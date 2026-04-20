use std::collections::{HashMap, HashSet};

use serde::Serialize;

use ironsmith::combat_state::AttackTarget;
use ironsmith::decision::GameResult;
use ironsmith::decisions::context::DecisionContext;
use ironsmith::game_state::{
    GameState, Target, UiBattlefieldTransition, UiBattlefieldTransitionKind,
};
use ironsmith::ids::{ObjectId, PlayerId};
use ironsmith::static_abilities::StaticAbilityId;
use ironsmith::types::CardType;
use ironsmith::zone::Zone;

use super::{
    ActiveViewedCards, DecisionView, GameOverView, ManaPaymentView, WasmGame, hidden_object_label,
    object_visible_to_perspective,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(super) enum BattlefieldLane {
    Artifacts,
    Lands,
    Creatures,
    Enchantments,
    Planeswalkers,
    Battles,
    Other,
}

impl BattlefieldLane {
    fn as_str(self) -> &'static str {
        match self {
            BattlefieldLane::Artifacts => "artifacts",
            BattlefieldLane::Lands => "lands",
            BattlefieldLane::Creatures => "creatures",
            BattlefieldLane::Enchantments => "enchantments",
            BattlefieldLane::Planeswalkers => "planeswalkers",
            BattlefieldLane::Battles => "battles",
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

pub(super) fn battlefield_lane_for_object(obj: &ironsmith::object::Object) -> BattlefieldLane {
    if obj.has_card_type(CardType::Enchantment) {
        return BattlefieldLane::Enchantments;
    }
    if obj.has_card_type(CardType::Creature) {
        return BattlefieldLane::Creatures;
    }
    if obj.has_card_type(CardType::Artifact) {
        return BattlefieldLane::Artifacts;
    }
    if obj.has_card_type(CardType::Land) {
        return BattlefieldLane::Lands;
    }
    if obj.has_card_type(CardType::Planeswalker) {
        return BattlefieldLane::Planeswalkers;
    }
    if obj.has_card_type(CardType::Battle) {
        return BattlefieldLane::Battles;
    }
    BattlefieldLane::Other
}

fn counter_signature_for_group(obj: &ironsmith::object::Object) -> String {
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

fn power_toughness_signature_for_group(obj: &ironsmith::object::Object) -> String {
    match (obj.power(), obj.toughness()) {
        (Some(power), Some(toughness)) => format!("{power}/{toughness}"),
        _ => "-".to_string(),
    }
}

pub(super) fn counter_snapshots_for_object(
    obj: &ironsmith::object::Object,
) -> Vec<CounterSnapshot> {
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

pub(super) fn protected_object_ids_for_decision(
    decision: Option<&DecisionContext>,
) -> HashSet<ObjectId> {
    let mut ids = HashSet::new();
    let Some(decision) = decision else {
        return ids;
    };

    match decision {
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
        DecisionContext::SelectOptions(options) => {
            for option in &options.options {
                if let Some(object_id) = option.object_id {
                    ids.insert(object_id);
                }
                if let Some(related_object_ids) = &option.related_object_ids {
                    ids.extend(related_object_ids.iter().copied());
                }
            }
        }
        DecisionContext::Modes(_)
        | DecisionContext::HybridChoice(_)
        | DecisionContext::TextInput(_)
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

pub(super) fn grouped_battlefield_for_player(
    game: &GameState,
    player: PlayerId,
    protected_ids: &HashSet<ObjectId>,
) -> (Vec<PermanentSnapshot>, usize) {
    let mut grouped: HashMap<BattlefieldGroupKey, Vec<&ironsmith::object::Object>> = HashMap::new();
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
            token: matches!(obj.kind, ironsmith::object::ObjectKind::Token),
            force_single_object: force_single,
        };
        grouped.entry(key).or_default().push(obj);
    }

    let mut groups: Vec<(BattlefieldGroupKey, Vec<&ironsmith::object::Object>)> =
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
            let oracle_text = representative
                .map(|obj| obj.oracle_text.clone())
                .unwrap_or_default();
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
                oracle_text,
                power_toughness,
                counter_signature: key.counter_signature.clone(),
                counters,
            }
        })
        .collect();

    (snapshots, total)
}

fn pseudo_hand_glow_kind_for_zone_card(
    game: &GameState,
    perspective: PlayerId,
    object: &ironsmith::object::Object,
    zone: Zone,
) -> Option<&'static str> {
    if object.zone != zone
        || matches!(
            zone,
            Zone::Hand | Zone::Library | Zone::Battlefield | Zone::Stack
        )
    {
        return None;
    }

    if zone == Zone::Command && object.owner == perspective && game.is_commander(object.id) {
        return Some("extra");
    }

    if !game
        .effect_store
        .grant_registry
        .granted_play_from_for_card(game, object.id, zone, perspective)
        .is_empty()
    {
        return Some("play-from");
    }

    if !game
        .effect_store
        .grant_registry
        .granted_alternative_casts_for_card(game, object.id, zone, perspective)
        .is_empty()
    {
        return Some("extra");
    }

    object
        .alternative_casts
        .iter()
        .any(|method| method.cast_from_zone() == zone)
        .then_some("extra")
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
            object.controller == player
                && game.object_has_static_ability_id(*id, StaticAbilityId::LookAtTopCardOfLibrary)
        })
    })
}

fn library_top_revealed_by_static_ability(game: &GameState, player: PlayerId) -> bool {
    game.object_store.battlefield.iter().any(|id| {
        game.object(*id).is_some_and(|object| {
            object.controller == player
                && game.object_has_static_ability_id(
                    *id,
                    StaticAbilityId::AllPlayersLookAtYourTopLibraryCard,
                )
        })
    })
}

fn can_view_library_top(game: &GameState, perspective: PlayerId, player: PlayerId) -> bool {
    if perspective == player && can_view_own_library_top(game, player) {
        return true;
    }

    if library_top_revealed_by_static_ability(game, player) {
        return true;
    }

    battlefield_has_static_ability(game, StaticAbilityId::AllPlayersLookAtTopCardsOfLibraries)
}

fn hand_revealed_by_static_ability(game: &GameState, player: PlayerId) -> bool {
    game.object_store.battlefield.iter().any(|id| {
        game.object(*id).is_some_and(|object| {
            object.controller != player
                && game.object_has_static_ability_id(
                    *id,
                    StaticAbilityId::OpponentsPlayWithHandsRevealed,
                )
        })
    })
}

fn build_zone_card_snapshot(
    game: &GameState,
    perspective: PlayerId,
    viewed_cards: Option<&ActiveViewedCards>,
    object: &ironsmith::object::Object,
    zone: Zone,
) -> ZoneCardSnapshot {
    let visible = object_visible_to_perspective(game, perspective, viewed_cards, object.id);
    let pseudo_hand_glow_kind = visible
        .then(|| pseudo_hand_glow_kind_for_zone_card(game, perspective, object, zone))
        .flatten()
        .map(str::to_string);
    let power_toughness = visible
        .then(|| match (object.power(), object.toughness()) {
            (Some(power), Some(toughness)) => Some(format!("{power}/{toughness}")),
            _ => None,
        })
        .flatten();

    ZoneCardSnapshot {
        id: object.id.0,
        stable_id: object.stable_id.0.0,
        name: if visible {
            object.name.clone()
        } else {
            hidden_object_label()
        },
        mana_cost: visible
            .then(|| object.mana_cost.as_ref().map(|mc| mc.to_oracle()))
            .flatten(),
        power_toughness,
        loyalty: visible.then(|| object.loyalty()).flatten(),
        defense: visible.then(|| object.defense()).flatten(),
        card_types: if visible {
            object
                .card_types
                .iter()
                .map(|ct| ct.name().to_string())
                .collect()
        } else {
            Vec::new()
        },
        show_in_pseudo_hand: visible && pseudo_hand_glow_kind.is_some(),
        pseudo_hand_glow_kind,
    }
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct PermanentSnapshot {
    pub(super) id: u64,
    pub(super) stable_id: u64,
    pub(super) name: String,
    pub(super) token: bool,
    pub(super) tapped: bool,
    pub(super) count: usize,
    pub(super) member_ids: Vec<u64>,
    pub(super) member_stable_ids: Vec<u64>,
    pub(super) lane: String,
    pub(super) mana_cost: Option<String>,
    pub(super) oracle_text: String,
    pub(super) power_toughness: Option<String>,
    pub(super) counter_signature: String,
    pub(super) counters: Vec<CounterSnapshot>,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct CounterSnapshot {
    pub(super) kind: String,
    pub(super) amount: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum BattlefieldTransitionKindSnapshot {
    Damaged,
    Destroyed,
    Sacrificed,
    Exiled,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct BattlefieldTransitionSnapshot {
    pub(super) stable_id: u64,
    pub(super) kind: BattlefieldTransitionKindSnapshot,
}

pub(super) fn battlefield_transition_snapshots(
    transitions: impl IntoIterator<Item = UiBattlefieldTransition>,
) -> Vec<BattlefieldTransitionSnapshot> {
    transitions
        .into_iter()
        .map(|transition| BattlefieldTransitionSnapshot {
            stable_id: transition.stable_id.0.0,
            kind: match transition.kind {
                UiBattlefieldTransitionKind::Damaged => BattlefieldTransitionKindSnapshot::Damaged,
                UiBattlefieldTransitionKind::Destroyed => {
                    BattlefieldTransitionKindSnapshot::Destroyed
                }
                UiBattlefieldTransitionKind::Sacrificed => {
                    BattlefieldTransitionKindSnapshot::Sacrificed
                }
                UiBattlefieldTransitionKind::Exiled => BattlefieldTransitionKindSnapshot::Exiled,
            },
        })
        .collect()
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct ObjectDetailsSnapshot {
    pub(super) id: u64,
    pub(super) stable_id: u64,
    pub(super) name: String,
    pub(super) kind: String,
    pub(super) zone: String,
    pub(super) owner: u8,
    pub(super) controller: u8,
    pub(super) type_line: String,
    pub(super) mana_cost: Option<String>,
    pub(super) oracle_text: String,
    pub(super) power: Option<i32>,
    pub(super) toughness: Option<i32>,
    pub(super) loyalty: Option<u32>,
    pub(super) tapped: bool,
    pub(super) counters: Vec<CounterSnapshot>,
    pub(super) compiled_text: Vec<String>,
    pub(super) abilities: Vec<String>,
    pub(super) raw_compilation: String,
    pub(super) semantic_score: Option<f32>,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct GameSnapshot {
    pub(super) snapshot_id: u64,
    pub(super) perspective: u8,
    pub(super) turn_number: u32,
    pub(super) active_player: u8,
    pub(super) priority_player: Option<u8>,
    pub(super) phase: String,
    pub(super) step: Option<String>,
    pub(super) stack_size: usize,
    pub(super) stack_preview: Vec<String>,
    pub(super) stack_objects: Vec<super::StackObjectSnapshot>,
    pub(super) resolving_stack_object: Option<super::StackObjectSnapshot>,
    pub(super) battlefield_size: usize,
    pub(super) exile_size: usize,
    pub(super) players: Vec<PlayerSnapshot>,
    pub(super) battlefield_transitions: Vec<BattlefieldTransitionSnapshot>,
    pub(super) viewed_cards: Option<ViewedCardsSnapshot>,
    pub(super) decision: Option<DecisionView>,
    pub(super) mana_payment: Option<ManaPaymentView>,
    pub(super) game_over: Option<GameOverView>,
    pub(super) cancelable: bool,
    pub(super) undo_land_stable_id: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct ManaPoolSnapshot {
    pub(super) white: u32,
    pub(super) blue: u32,
    pub(super) black: u32,
    pub(super) red: u32,
    pub(super) green: u32,
    pub(super) colorless: u32,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct PlayerSnapshot {
    pub(super) id: u8,
    pub(super) name: String,
    pub(super) life: i32,
    pub(super) mana_pool: ManaPoolSnapshot,
    pub(super) can_view_hand: bool,
    pub(super) can_view_library_top: bool,
    pub(super) hand_size: usize,
    pub(super) library_size: usize,
    pub(super) graveyard_size: usize,
    pub(super) command_size: usize,
    pub(super) hand_cards: Vec<HandCardSnapshot>,
    pub(super) graveyard_cards: Vec<ZoneCardSnapshot>,
    pub(super) exile_cards: Vec<ZoneCardSnapshot>,
    pub(super) command_cards: Vec<ZoneCardSnapshot>,
    pub(super) library_top: Option<String>,
    pub(super) graveyard_top: Option<String>,
    pub(super) battlefield: Vec<PermanentSnapshot>,
    pub(super) battlefield_total: usize,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct ViewedCardsSnapshot {
    pub(super) viewer: u8,
    pub(super) subject: u8,
    pub(super) zone: String,
    pub(super) visibility: String,
    pub(super) cards: Vec<ViewedCardSnapshot>,
    pub(super) card_ids: Vec<u64>,
    pub(super) source: Option<u64>,
    pub(super) description: String,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct ViewedCardSnapshot {
    pub(super) id: u64,
    pub(super) name: String,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct HandCardSnapshot {
    pub(super) id: u64,
    pub(super) stable_id: u64,
    pub(super) name: String,
    pub(super) mana_cost: Option<String>,
    pub(super) power_toughness: Option<String>,
    pub(super) loyalty: Option<u32>,
    pub(super) defense: Option<u32>,
    pub(super) card_types: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct ZoneCardSnapshot {
    pub(super) id: u64,
    pub(super) stable_id: u64,
    pub(super) name: String,
    pub(super) mana_cost: Option<String>,
    pub(super) power_toughness: Option<String>,
    pub(super) loyalty: Option<u32>,
    pub(super) defense: Option<u32>,
    pub(super) card_types: Vec<String>,
    pub(super) show_in_pseudo_hand: bool,
    pub(super) pseudo_hand_glow_kind: Option<String>,
}

impl GameSnapshot {
    pub(super) fn from_game(
        game: &GameState,
        perspective: PlayerId,
        decision: Option<&DecisionContext>,
        mana_payment: Option<ManaPaymentView>,
        game_over: Option<&GameResult>,
        pending_cast_stack_id: Option<ObjectId>,
        resolving_stack_object: Option<super::StackObjectSnapshot>,
        battlefield_transitions: Vec<BattlefieldTransitionSnapshot>,
        viewed_cards: Option<&ActiveViewedCards>,
        cancelable: bool,
        undo_land_stable_id: Option<u64>,
        snapshot_id: u64,
    ) -> Self {
        let stack_viewed_cards = super::stack_revealed_view(game);
        let viewed_cards = viewed_cards.or(stack_viewed_cards.as_ref());
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
                let hand_revealed_by_static = hand_revealed_by_static_ability(game, p.id);
                let can_view_hand =
                    is_perspective_player || visible_hand_view.is_some() || hand_revealed_by_static;
                let can_view_library_top = can_view_library_top(game, perspective, p.id);
                PlayerSnapshot {
                    can_view_hand,
                    can_view_library_top,
                    hand_cards: if can_view_hand {
                        p.hand
                            .iter()
                            .rev()
                            .filter(|id| {
                                is_perspective_player
                                    || hand_revealed_by_static
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
                        .map(|o| {
                            build_zone_card_snapshot(
                                game,
                                perspective,
                                viewed_cards,
                                o,
                                Zone::Graveyard,
                            )
                        })
                        .collect(),
                    exile_cards: game
                        .exile
                        .iter()
                        .rev()
                        .filter_map(|id| game.object(*id))
                        .filter(|o| o.owner == p.id)
                        .map(|o| {
                            build_zone_card_snapshot(
                                game,
                                perspective,
                                viewed_cards,
                                o,
                                Zone::Exile,
                            )
                        })
                        .collect(),
                    command_cards: game
                        .command_zone
                        .iter()
                        .rev()
                        .filter_map(|id| game.object(*id))
                        .filter(|o| o.owner == p.id)
                        .map(|o| {
                            build_zone_card_snapshot(
                                game,
                                perspective,
                                viewed_cards,
                                o,
                                Zone::Command,
                            )
                        })
                        .collect(),
                    library_top: can_view_library_top
                        .then(|| {
                            p.library
                                .last()
                                .and_then(|id| game.object(*id))
                                .map(|o| o.name.clone())
                        })
                        .flatten(),
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
        let mut stack_objects: Vec<super::StackObjectSnapshot> = game
            .stack
            .iter()
            .rev()
            .map(|entry| super::build_stack_object_snapshot(game, perspective, viewed_cards, entry))
            .collect();
        let mut stack_size = game.stack.len();

        if let Some(stack_id) = pending_cast_stack_id
            && !game.stack.iter().any(|entry| entry.object_id == stack_id)
            && let Some(obj) = game.object(stack_id)
        {
            stack_preview.insert(0, obj.name.clone());
            let pending_effect_text = {
                let lines =
                    ironsmith::compiled_text::debug_compiled_lines(&obj.to_card_definition());
                if lines.is_empty() {
                    None
                } else {
                    Some(lines.join("; "))
                }
            };
            stack_objects.insert(
                0,
                super::StackObjectSnapshot {
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

pub(super) fn build_object_details_snapshot(
    game: &GameState,
    id: ObjectId,
) -> Option<ObjectDetailsSnapshot> {
    let obj = game.object(id)?;
    let current_name = game.current_name(id).unwrap_or_else(|| obj.name.clone());
    let current_controller = game.current_controller(id).unwrap_or(obj.controller);
    let current_supertypes = game
        .current_supertypes(id)
        .unwrap_or_else(|| obj.supertypes.clone());
    let current_card_types = game
        .current_card_types(id)
        .unwrap_or_else(|| obj.card_types.clone());
    let current_subtypes = game
        .current_subtypes(id)
        .unwrap_or_else(|| obj.subtypes.clone());
    let current_abilities = game
        .current_abilities(id)
        .unwrap_or_else(|| obj.abilities.clone());
    let (power, toughness) = if obj.zone == Zone::Battlefield {
        (
            game.calculated_power(id).or_else(|| obj.power()),
            game.calculated_toughness(id).or_else(|| obj.toughness()),
        )
    } else {
        (obj.power(), obj.toughness())
    };
    let counters = counter_snapshots_for_object(obj);
    let compiled_text = ironsmith::compiled_text::debug_compiled_lines(&obj.to_card_definition());

    Some(ObjectDetailsSnapshot {
        id: obj.id.0,
        stable_id: obj.stable_id.0.0,
        name: current_name,
        kind: obj.kind.to_string(),
        zone: zone_label(obj.zone),
        owner: obj.owner.0,
        controller: current_controller.0,
        type_line: format_type_line_parts(
            &current_supertypes,
            &current_card_types,
            &current_subtypes,
        ),
        mana_cost: obj.mana_cost.as_ref().map(|cost| cost.to_oracle()),
        oracle_text: obj.oracle_text.clone(),
        power,
        toughness,
        loyalty: obj.loyalty(),
        tapped: game.is_tapped(obj.id),
        counters,
        compiled_text,
        abilities: current_abilities
            .iter()
            .filter_map(|ability| ability.text.clone())
            .collect(),
        raw_compilation: format!("{:#?}", obj.to_card_definition()),
        semantic_score: WasmGame::semantic_score_for_name(obj.name.as_str()),
    })
}

fn format_type_line_parts(
    supertypes: &[ironsmith::types::Supertype],
    card_types: &[ironsmith::types::CardType],
    subtypes: &[ironsmith::types::Subtype],
) -> String {
    let mut left = Vec::new();
    left.extend(supertypes.iter().map(|value| format!("{value:?}")));
    left.extend(card_types.iter().map(|value| format!("{value:?}")));

    let mut type_line = left.join(" ");
    if !subtypes.is_empty() {
        let subtypes = subtypes
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

fn zone_label(zone: Zone) -> String {
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
