use std::collections::{HashMap, HashSet};

use serde::Serialize;

use ironsmith::cards::CardRegistry;
use ironsmith::combat_state::AttackTarget;
use ironsmith::decision::GameResult;
use ironsmith::decisions::context::DecisionContext;
use ironsmith::game_state::{
    GameState, Target, UiBattlefieldTransition, UiBattlefieldTransitionKind,
};
use ironsmith::ids::{ObjectId, PlayerId, StableId};
use ironsmith::object::AttachmentTarget;
use ironsmith::static_abilities::StaticAbilityId;
use ironsmith::types::{CardType, Subtype};
use ironsmith::zone::Zone;

use super::{
    ActiveViewedCards, CryptoRequirementView, DecisionView, GameOverView, ManaPaymentView,
    WasmGame, hidden_object_label, object_visible_to_perspective,
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
    characteristic_signature: String,
    counter_signature: String,
    token: bool,
    force_single_object: Option<u64>,
}

fn battlefield_lane_for_card_types(card_types: &[CardType]) -> BattlefieldLane {
    if card_types.contains(&CardType::Enchantment) {
        return BattlefieldLane::Enchantments;
    }
    if card_types.contains(&CardType::Creature) {
        return BattlefieldLane::Creatures;
    }
    if card_types.contains(&CardType::Artifact) {
        return BattlefieldLane::Artifacts;
    }
    if card_types.contains(&CardType::Land) {
        return BattlefieldLane::Lands;
    }
    if card_types.contains(&CardType::Planeswalker) {
        return BattlefieldLane::Planeswalkers;
    }
    if card_types.contains(&CardType::Battle) {
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

fn sorted_name_signature<T, F>(items: &[T], mut name: F) -> String
where
    F: FnMut(&T) -> String,
{
    let mut parts = items.iter().map(&mut name).collect::<Vec<_>>();
    parts.sort_unstable();
    parts.join(",")
}

fn color_signature(colors: ironsmith::color::ColorSet) -> String {
    let mut parts = Vec::new();
    for color in ironsmith::color::Color::ALL {
        if colors.contains(color) {
            parts.push(color.name());
        }
    }
    if parts.is_empty() {
        "-".to_string()
    } else {
        parts.join(",")
    }
}

fn ability_signature(abilities: &[ironsmith::ability::Ability]) -> String {
    let mut parts = abilities
        .iter()
        .map(|ability| format!("{:?}", ability.kind))
        .collect::<Vec<_>>();
    parts.sort_unstable();
    if parts.is_empty() {
        "-".to_string()
    } else {
        parts.join("|")
    }
}

fn static_ability_signature(
    static_abilities: &[ironsmith::static_abilities::StaticAbility],
) -> String {
    let mut parts = static_abilities
        .iter()
        .map(|ability| format!("{ability:?}"))
        .collect::<Vec<_>>();
    parts.sort_unstable();
    if parts.is_empty() {
        "-".to_string()
    } else {
        parts.join("|")
    }
}

fn attached_to_signature(game: &GameState, obj: &ironsmith::object::Object) -> String {
    match obj.attached_to {
        Some(AttachmentTarget::Object(id)) => game
            .object(id)
            .map(|target| object_characteristic_signature(game, target, false))
            .unwrap_or_else(|| "object:missing".to_string()),
        Some(AttachmentTarget::Player(id)) => format!("player:{}", id.0),
        None => "-".to_string(),
    }
}

fn attachment_signature(game: &GameState, obj: &ironsmith::object::Object) -> String {
    let mut parts = obj
        .attachments
        .iter()
        .filter_map(|attachment_id| game.object(*attachment_id))
        .map(|attachment| object_characteristic_signature(game, attachment, false))
        .collect::<Vec<_>>();
    parts.sort_unstable();
    if parts.is_empty() {
        "-".to_string()
    } else {
        parts.join("||")
    }
}

fn object_characteristic_signature(
    game: &GameState,
    obj: &ironsmith::object::Object,
    include_attachments: bool,
) -> String {
    let current = game.current_characteristics(obj.id);
    let (
        name,
        compiled_card_text,
        power,
        toughness,
        card_types,
        subtypes,
        supertypes,
        colors,
        abilities,
        static_abilities,
        controller,
    ) = if let Some(chars) = current {
        (
            chars.name,
            chars.compiled_card_text,
            chars.power,
            chars.toughness,
            chars.card_types,
            chars.subtypes,
            chars.supertypes,
            chars.colors,
            chars.abilities,
            chars.static_abilities,
            chars.controller,
        )
    } else {
        (
            obj.name.clone(),
            obj.compiled_card_text.clone(),
            obj.power(),
            obj.toughness(),
            obj.card_types.clone(),
            obj.subtypes.clone(),
            obj.supertypes.clone(),
            obj.colors(),
            obj.abilities.clone(),
            obj.abilities
                .iter()
                .filter_map(|ability| match &ability.kind {
                    ironsmith::ability::AbilityKind::Static(static_ability) => {
                        Some(static_ability.clone())
                    }
                    _ => None,
                })
                .collect(),
            obj.owner,
        )
    };

    let card_type_signature =
        sorted_name_signature(&card_types, |card_type| card_type.name().to_string());
    let subtype_signature = sorted_name_signature(&subtypes, |subtype| subtype.display_name());
    let supertype_signature =
        sorted_name_signature(&supertypes, |supertype| supertype.name().to_string());
    let attachment_part = include_attachments
        .then(|| attachment_signature(game, obj))
        .unwrap_or_else(|| "-".to_string());

    [
        format!("owner:{}", obj.owner.0),
        format!("controller:{}", controller.0),
        format!("kind:{}", obj.kind.name()),
        format!("name:{name}"),
        format!(
            "mana:{}",
            obj.mana_cost
                .as_ref()
                .map(|mana_cost| mana_cost.to_oracle())
                .unwrap_or_else(|| "-".to_string())
        ),
        format!("colors:{}", color_signature(colors)),
        format!("supertypes:{supertype_signature}"),
        format!("types:{card_type_signature}"),
        format!("subtypes:{subtype_signature}"),
        format!("oracle:{compiled_card_text}"),
        format!(
            "pt:{}/{}",
            power.map_or("-".to_string(), |p| p.to_string()),
            toughness.map_or("-".to_string(), |t| t.to_string())
        ),
        format!(
            "loyalty:{}",
            obj.loyalty()
                .map_or_else(|| "-".to_string(), |loyalty| loyalty.to_string())
        ),
        format!(
            "defense:{}",
            obj.defense()
                .map_or_else(|| "-".to_string(), |defense| defense.to_string())
        ),
        format!("counters:{}", counter_signature_for_group(obj)),
        format!("abilities:{}", ability_signature(&abilities)),
        format!("static:{}", static_ability_signature(&static_abilities)),
        format!("attached_to:{}", attached_to_signature(game, obj)),
        format!("attachments:{attachment_part}"),
    ]
    .join("\n")
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
        if game.current_controller(obj.id).unwrap_or(obj.owner) != player {
            continue;
        }
        total += 1;

        let force_single = protected_ids.contains(&obj.id).then_some(obj.id.0);
        let current = game.current_characteristics(obj.id);
        let current_card_types = current
            .as_ref()
            .map(|chars| chars.card_types.as_slice())
            .unwrap_or(&obj.card_types);
        let current_name = current
            .as_ref()
            .map(|chars| chars.name.clone())
            .unwrap_or_else(|| obj.name.clone());
        let key = BattlefieldGroupKey {
            lane: battlefield_lane_for_card_types(current_card_types),
            name: current_name,
            tapped: game.is_tapped(obj.id),
            characteristic_signature: object_characteristic_signature(game, obj, true),
            counter_signature: counter_signature_for_group(obj),
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
                .map(|obj| {
                    game.current_characteristics(obj.id)
                        .map(|chars| chars.name)
                        .unwrap_or_else(|| obj.name.clone())
                })
                .unwrap_or_else(|| key.name.clone());
            let power_toughness = representative.and_then(|obj| {
                let current = game.current_characteristics(obj.id);
                let p = current
                    .as_ref()
                    .and_then(|chars| chars.power)
                    .or_else(|| obj.power())?;
                let t = current
                    .as_ref()
                    .and_then(|chars| chars.toughness)
                    .or_else(|| obj.toughness())?;
                Some(format!("{p}/{t}"))
            });
            let mana_cost =
                representative.and_then(|obj| obj.mana_cost.as_ref().map(|mc| mc.to_oracle()));
            let compiled_card_text = representative
                .map(|obj| {
                    game.current_characteristics(obj.id)
                        .map(|chars| chars.compiled_card_text)
                        .unwrap_or_else(|| obj.compiled_card_text.clone())
                })
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
                oracle_text: compiled_card_text,
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

    if object.owner != perspective {
        return None;
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
            game.current_controller(*id).unwrap_or(object.owner) == player
                && game.object_has_static_ability_id(*id, StaticAbilityId::LookAtTopCardOfLibrary)
        })
    })
}

fn library_top_revealed_by_static_ability(game: &GameState, player: PlayerId) -> bool {
    game.object_store.battlefield.iter().any(|id| {
        game.object(*id).is_some_and(|object| {
            game.current_controller(*id).unwrap_or(object.owner) == player
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
            game.current_controller(*id).unwrap_or(object.owner) != player
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
        oracle_text: visible
            .then(|| object.compiled_card_text.clone())
            .unwrap_or_default(),
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

#[derive(Debug, Clone, Serialize)]
pub(super) struct ZoneTransitionSnapshot {
    pub(super) id: u64,
    pub(super) old_object_id: u64,
    pub(super) new_object_id: u64,
    pub(super) stable_id: u64,
    pub(super) owner: u8,
    pub(super) controller: u8,
    pub(super) from_zone: String,
    pub(super) to_zone: String,
    pub(super) card: ZoneCardSnapshot,
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

fn zone_transition_snapshots(
    game: &GameState,
    perspective: PlayerId,
    viewed_cards: Option<&ActiveViewedCards>,
) -> Vec<ZoneTransitionSnapshot> {
    let hidden_label = hidden_object_label().to_ascii_lowercase();
    game.ui_zone_transitions()
        .iter()
        .filter_map(|transition| {
            let object = game.object(transition.new_object_id)?;
            if !object_visible_to_perspective(game, perspective, viewed_cards, object.id) {
                return None;
            }
            let card =
                build_zone_card_snapshot(game, perspective, viewed_cards, object, transition.to);
            if card.name.trim().to_ascii_lowercase() == hidden_label {
                return None;
            }
            Some(ZoneTransitionSnapshot {
                id: transition.id,
                old_object_id: transition.old_object_id.0,
                new_object_id: transition.new_object_id.0,
                stable_id: transition.stable_id.0.0,
                owner: transition.owner.0,
                controller: transition.controller.0,
                from_zone: transition.from.to_string(),
                to_zone: transition.to.to_string(),
                card,
            })
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
    pub(super) type_line_display: String,
    pub(super) type_line_badges: Vec<String>,
    pub(super) mana_cost: Option<String>,
    pub(super) oracle_text: String,
    pub(super) power: Option<i32>,
    pub(super) toughness: Option<i32>,
    pub(super) loyalty: Option<u32>,
    pub(super) tapped: bool,
    pub(super) counters: Vec<CounterSnapshot>,
    pub(super) compiled_text: Vec<String>,
    pub(super) abilities: Vec<String>,
    pub(super) chosen_color: Option<String>,
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
    pub(super) zone_transitions: Vec<ZoneTransitionSnapshot>,
    pub(super) crypto_requirements: Vec<CryptoRequirementView>,
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
    pub(super) sideboard_cards: Vec<ZoneCardSnapshot>,
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
    pub(super) stable_id: u64,
    pub(super) name: String,
    pub(super) oracle_text: String,
}

fn resolve_viewed_card(
    game: &GameState,
    id: ObjectId,
    stable_id: StableId,
) -> (ObjectId, u64, String, String) {
    if let Some(obj) = game.object(id) {
        return (
            id,
            obj.stable_id.0.0,
            obj.name.clone(),
            obj.compiled_card_text.clone(),
        );
    }

    if let Some(current_id) = game.find_object_by_stable_id(stable_id)
        && let Some(obj) = game.object(current_id)
    {
        return (
            current_id,
            obj.stable_id.0.0,
            obj.name.clone(),
            obj.compiled_card_text.clone(),
        );
    }

    let id_as_stable_id = StableId::from_raw(id.0);
    if id_as_stable_id != stable_id
        && let Some(current_id) = game.find_object_by_stable_id(id_as_stable_id)
        && let Some(obj) = game.object(current_id)
    {
        return (
            current_id,
            obj.stable_id.0.0,
            obj.name.clone(),
            obj.compiled_card_text.clone(),
        );
    }

    (id, stable_id.0.0, format!("Card #{}", id.0), String::new())
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct HandCardSnapshot {
    pub(super) id: u64,
    pub(super) stable_id: u64,
    pub(super) name: String,
    pub(super) mana_cost: Option<String>,
    pub(super) oracle_text: String,
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
    pub(super) oracle_text: String,
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
        let mut characteristic_ids = game.battlefield.clone();
        characteristic_ids.extend(game.stack.iter().map(|entry| entry.object_id));
        if let Some(stack_id) = pending_cast_stack_id {
            characteristic_ids.push(stack_id);
        }
        characteristic_ids.sort_unstable();
        characteristic_ids.dedup();
        game.prewarm_calculated_characteristics(&characteristic_ids);
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
                                    || visible_hand_view
                                        .is_some_and(|view| view.contains_object(game, **id))
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
                                    oracle_text: o.compiled_card_text.clone(),
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
                    sideboard_cards: if is_perspective_player {
                        p.sideboard
                            .iter()
                            .rev()
                            .filter_map(|id| game.object(*id))
                            .map(|o| {
                                build_zone_card_snapshot(
                                    game,
                                    perspective,
                                    viewed_cards,
                                    o,
                                    Zone::OutsideGame,
                                )
                            })
                            .collect()
                    } else {
                        Vec::new()
                    },
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
        let zone_transitions = zone_transition_snapshots(game, perspective, viewed_cards);

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
                let lines: Vec<_> = obj
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
            };
            stack_objects.insert(
                0,
                super::StackObjectSnapshot {
                    id: stack_id.0,
                    inspect_object_id: Some(stack_id.0),
                    stable_id: Some(obj.stable_id.0.0),
                    source_stable_id: None,
                    controller: game.current_controller(stack_id).unwrap_or(obj.owner).0,
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
            zone_transitions,
            crypto_requirements: Vec::new(),
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
                        .enumerate()
                        .map(|(index, id)| {
                            let (current_id, stable_id, name, oracle_text) =
                                resolve_viewed_card(game, *id, view.stable_id_at(index, *id));
                            ViewedCardSnapshot {
                                id: current_id.0,
                                stable_id,
                                name,
                                oracle_text,
                            }
                        })
                        .collect(),
                    card_ids: view
                        .cards
                        .iter()
                        .enumerate()
                        .map(|(index, id)| {
                            resolve_viewed_card(game, *id, view.stable_id_at(index, *id))
                                .0
                                .0
                        })
                        .collect(),
                    source: view.source.map(|id| id.0),
                    description: view.description.clone(),
                }),
            decision: decision.map(|ctx| {
                DecisionView::from_context(
                    game,
                    ctx,
                    perspective,
                    viewed_cards,
                    undo_land_stable_id,
                )
            }),
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
    let current_controller = game.current_controller(id).unwrap_or(obj.owner);
    let current_supertypes = game
        .current_supertypes(id)
        .unwrap_or_else(|| obj.supertypes.clone());
    let current_card_types = game
        .current_card_types(id)
        .unwrap_or_else(|| obj.card_types.clone());
    let current_subtypes = game
        .current_subtypes(id)
        .unwrap_or_else(|| obj.subtypes.clone());
    let (power, toughness) = if obj.zone == Zone::Battlefield {
        (
            game.calculated_power(id).or_else(|| obj.power()),
            game.calculated_toughness(id).or_else(|| obj.toughness()),
        )
    } else {
        (obj.power(), obj.toughness())
    };
    let counters = counter_snapshots_for_object(obj);
    let compiled_text = ironsmith::compiled_text::compiled_text_lines(&obj.to_card_definition());

    let type_line =
        format_type_line_parts(&current_supertypes, &current_card_types, &current_subtypes);
    let (type_line_display, type_line_badges) = format_type_line_display_parts(
        &current_supertypes,
        &current_card_types,
        &current_subtypes,
        &obj.subtypes,
    );

    let abilities = game
        .current_abilities(id)
        .unwrap_or_else(|| obj.abilities.clone())
        .iter()
        .map(ironsmith::compiled_text::ability_surface_text)
        .collect();

    Some(ObjectDetailsSnapshot {
        id: obj.id.0,
        stable_id: obj.stable_id.0.0,
        name: current_name,
        kind: obj.kind.to_string(),
        zone: zone_label(obj.zone),
        owner: obj.owner.0,
        controller: current_controller.0,
        type_line,
        type_line_display,
        type_line_badges,
        mana_cost: obj.mana_cost.as_ref().map(|cost| cost.to_oracle()),
        oracle_text: obj.compiled_card_text.clone(),
        power,
        toughness,
        loyalty: obj.loyalty(),
        tapped: game.is_tapped(obj.id),
        counters,
        compiled_text,
        abilities,
        chosen_color: game
            .chosen_color(obj.id)
            .map(|color| color.name().to_string()),
        raw_compilation: format!("{:#?}", obj.to_card_definition()),
        semantic_score: CardRegistry::generated_parser_semantic_score(obj.name.as_str()),
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

fn format_type_line_display_parts(
    supertypes: &[ironsmith::types::Supertype],
    card_types: &[ironsmith::types::CardType],
    subtypes: &[ironsmith::types::Subtype],
    printed_subtypes: &[ironsmith::types::Subtype],
) -> (String, Vec<String>) {
    if object_has_all_creature_types(card_types, subtypes) {
        let display_subtypes =
            compact_all_creature_type_display_subtypes(printed_subtypes, subtypes);
        return (
            format_type_line_parts(supertypes, card_types, &display_subtypes),
            vec!["All creature types".to_string()],
        );
    }

    (
        format_type_line_parts(supertypes, card_types, subtypes),
        Vec::new(),
    )
}

fn object_has_all_creature_types(
    card_types: &[ironsmith::types::CardType],
    subtypes: &[ironsmith::types::Subtype],
) -> bool {
    let can_have_creature_types = card_types
        .iter()
        .any(|card_type| matches!(card_type, CardType::Creature | CardType::Kindred));
    can_have_creature_types
        && Subtype::all_creature_types()
            .iter()
            .all(|subtype| subtypes.contains(subtype))
}

fn compact_all_creature_type_display_subtypes(
    printed_subtypes: &[ironsmith::types::Subtype],
    current_subtypes: &[ironsmith::types::Subtype],
) -> Vec<ironsmith::types::Subtype> {
    let mut display_subtypes = Vec::new();
    for subtype in printed_subtypes.iter().chain(
        current_subtypes
            .iter()
            .filter(|subtype| !subtype.is_creature_type()),
    ) {
        if current_subtypes.contains(subtype) && !display_subtypes.contains(subtype) {
            display_subtypes.push(*subtype);
        }
    }
    display_subtypes
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
        Zone::OutsideGame => "outside_game",
    }
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ironsmith::alternative_cast::AlternativeCastingMethod;
    use ironsmith::card::{Card, CardBuilder, PowerToughness};
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::cards::tokens::cursed_role_token_definition;
    use ironsmith::decisions::context::{DecisionContext, SelectObjectsContext, SelectableObject};
    use ironsmith::game_state::{GameState, PlayerControlDuration, PlayerControlStart};
    use ironsmith::ids::{CardId, PlayerId};
    use ironsmith::mana::{ManaCost, ManaSymbol};
    use ironsmith::object::AttachmentTarget;
    use ironsmith::types::Subtype;

    fn test_bears_card() -> Card {
        CardBuilder::new(CardId::from_raw(90_001), "Grizzly Bears")
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Bear])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build()
    }

    fn add_native_escape_to_object(game: &mut GameState, id: ironsmith::ids::ObjectId) {
        game.object_mut(id)
            .expect("escape object should exist")
            .alternative_casts
            .push(AlternativeCastingMethod::Escape {
                cost: Some(ManaCost::from_pips(vec![
                    vec![ManaSymbol::Red],
                    vec![ManaSymbol::Red],
                ])),
                exile_count: 2,
            });
    }

    #[test]
    fn visible_hand_snapshots_include_oracle_text() {
        let _id_counter_guard = crate::test_id_counter_guard();
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let definition = CardDefinitionBuilder::new(CardId::from_raw(90_012), "Hand Flying Probe")
            .card_types(vec![CardType::Creature])
            .flying()
            .build();
        let object_id = game.create_object_from_definition(&definition, alice, Zone::Hand);

        let snapshot = GameSnapshot::from_game(
            &game,
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
        let alice_snapshot = snapshot
            .players
            .iter()
            .find(|player| player.id == alice.0)
            .expect("Alice snapshot should exist");
        let card = alice_snapshot
            .hand_cards
            .iter()
            .find(|card| card.id == object_id.0)
            .expect("visible hand card should be in snapshot");

        assert!(
            card.oracle_text.contains("Flying"),
            "hand snapshots should carry oracle text for inspector fallback"
        );
    }

    #[test]
    fn visible_zone_snapshots_include_oracle_text() {
        let _id_counter_guard = crate::test_id_counter_guard();
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let definition = CardDefinitionBuilder::new(CardId::from_raw(90_013), "Zone Flying Probe")
            .card_types(vec![CardType::Creature])
            .flying()
            .build();
        let object_id = game.create_object_from_definition(&definition, bob, Zone::Graveyard);

        let snapshot = GameSnapshot::from_game(
            &game,
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
        let bob_snapshot = snapshot
            .players
            .iter()
            .find(|player| player.id == bob.0)
            .expect("Bob snapshot should exist");
        let card = bob_snapshot
            .graveyard_cards
            .iter()
            .find(|card| card.id == object_id.0)
            .expect("visible graveyard card should be in snapshot");

        assert!(
            card.oracle_text.contains("Flying"),
            "zone snapshots should carry oracle text for inspector fallback"
        );
    }

    #[test]
    fn pseudo_hand_hides_opponent_native_escape_card() {
        let _id_counter_guard = crate::test_id_counter_guard();
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let escape_card = CardBuilder::new(CardId::from_raw(90_010), "Escape Probe")
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(1)],
                vec![ManaSymbol::Red],
            ]))
            .card_types(vec![CardType::Creature])
            .build();
        let escape_id = game.create_object_from_card(&escape_card, bob, Zone::Graveyard);
        add_native_escape_to_object(&mut game, escape_id);

        let snapshot = GameSnapshot::from_game(
            &game,
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
        let bob_snapshot = snapshot
            .players
            .iter()
            .find(|player| player.id == bob.0)
            .expect("opponent snapshot should exist");
        let graveyard_card = bob_snapshot
            .graveyard_cards
            .iter()
            .find(|card| card.id == escape_id.0)
            .expect("opponent graveyard card should be visible");

        assert!(
            !graveyard_card.show_in_pseudo_hand,
            "an opponent-owned native escape card should not be surfaced in Alice's pseudo-hand"
        );
        assert_eq!(graveyard_card.pseudo_hand_glow_kind, None);
    }

    #[test]
    fn pseudo_hand_keeps_own_native_escape_card() {
        let _id_counter_guard = crate::test_id_counter_guard();
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let escape_card = CardBuilder::new(CardId::from_raw(90_011), "Own Escape Probe")
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(1)],
                vec![ManaSymbol::Red],
            ]))
            .card_types(vec![CardType::Creature])
            .build();
        let escape_id = game.create_object_from_card(&escape_card, alice, Zone::Graveyard);
        add_native_escape_to_object(&mut game, escape_id);

        let snapshot = GameSnapshot::from_game(
            &game,
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
        let alice_snapshot = snapshot
            .players
            .iter()
            .find(|player| player.id == alice.0)
            .expect("Alice snapshot should exist");
        let graveyard_card = alice_snapshot
            .graveyard_cards
            .iter()
            .find(|card| card.id == escape_id.0)
            .expect("own graveyard card should be visible");

        assert!(
            graveyard_card.show_in_pseudo_hand,
            "an owned native escape card should still be surfaced in Alice's pseudo-hand"
        );
        assert_eq!(
            graveyard_card.pseudo_hand_glow_kind.as_deref(),
            Some("extra")
        );
    }

    #[test]
    fn decision_snapshot_routes_controlled_player_prompt_to_controller() {
        let _id_counter_guard = crate::test_id_counter_guard();
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let card = CardBuilder::new(CardId::from_raw(90_012), "Primeval Titan")
            .card_types(vec![CardType::Creature])
            .build();
        let card_id = game.create_object_from_card(&card, alice, Zone::Hand);

        game.add_player_control(
            bob,
            alice,
            PlayerControlStart::Immediate,
            PlayerControlDuration::UntilEndOfTurn,
            None,
        );

        let decision = DecisionContext::SelectObjects(SelectObjectsContext::new(
            alice,
            None,
            "Choose a card",
            vec![SelectableObject::new(card_id, "Primeval Titan")],
            1,
            Some(1),
        ));
        let snapshot = GameSnapshot::from_game(
            &game,
            bob,
            Some(&decision),
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

        match snapshot
            .decision
            .as_ref()
            .expect("decision should be present")
        {
            DecisionView::SelectObjects {
                player, candidates, ..
            } => {
                assert_eq!(
                    *player, bob.0,
                    "the browser prompt should belong to the controlling player"
                );
                assert_eq!(candidates[0].id, card_id.0);
                assert_eq!(candidates[0].name, "Primeval Titan");
            }
            other => panic!("expected select-object decision, got {other:?}"),
        }
    }

    #[test]
    fn battlefield_grouping_uses_calculated_characteristics_and_matching_attachments() {
        let _id_counter_guard = crate::test_id_counter_guard();
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let protected_ids = std::collections::HashSet::new();
        let bears_card = test_bears_card();
        let role_def = cursed_role_token_definition();

        let unenchanted_bear = game.create_object_from_card(&bears_card, alice, Zone::Battlefield);
        let enchanted_bears: Vec<_> = (0..3)
            .map(|_| {
                let bear = game.create_object_from_card(&bears_card, alice, Zone::Battlefield);
                let role = game.create_object_from_definition(&role_def, alice, Zone::Battlefield);
                assert!(
                    game.attach_object_to_target(role, AttachmentTarget::Object(bear)),
                    "role should attach to bear"
                );
                bear
            })
            .collect();

        assert_eq!(
            game.current_power(unenchanted_bear),
            Some(2),
            "control bear should keep printed power"
        );
        assert_eq!(
            battlefield_lane_for_card_types(
                &game
                    .object(unenchanted_bear)
                    .expect("bear should exist")
                    .card_types
            ),
            BattlefieldLane::Creatures
        );
        assert!(
            enchanted_bears
                .iter()
                .all(|bear| game.current_power(*bear) == Some(1)),
            "cursed roles should set each attached bear to 1/1"
        );

        let (battlefield, _) = grouped_battlefield_for_player(&game, alice, &protected_ids);
        let bear_groups: Vec<_> = battlefield
            .iter()
            .filter(|permanent| permanent.name == "Grizzly Bears")
            .collect();

        assert_eq!(
            bear_groups.len(),
            2,
            "unenchanted and cursed bears should not collapse together"
        );
        assert!(
            bear_groups
                .iter()
                .any(|group| group.count == 1 && group.power_toughness.as_deref() == Some("2/2")),
            "single unenchanted bear should stay in its own group: {bear_groups:?}"
        );
        assert!(
            bear_groups
                .iter()
                .any(|group| group.count == 3 && group.power_toughness.as_deref() == Some("1/1")),
            "matching cursed bears should group together: {bear_groups:?}"
        );
    }

    #[test]
    fn battlefield_grouping_splits_each_protected_legal_target() {
        let _id_counter_guard = crate::test_id_counter_guard();
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bears_card = test_bears_card();
        let role_def = cursed_role_token_definition();
        let mut protected_ids = std::collections::HashSet::new();

        for _ in 0..3 {
            let bear = game.create_object_from_card(&bears_card, alice, Zone::Battlefield);
            let role = game.create_object_from_definition(&role_def, alice, Zone::Battlefield);
            assert!(
                game.attach_object_to_target(role, AttachmentTarget::Object(bear)),
                "role should attach to bear"
            );
            protected_ids.insert(bear);
        }

        let (battlefield, _) = grouped_battlefield_for_player(&game, alice, &protected_ids);
        let bear_groups: Vec<_> = battlefield
            .iter()
            .filter(|permanent| permanent.name == "Grizzly Bears")
            .collect();

        assert_eq!(
            bear_groups.len(),
            3,
            "every legal target should stay individually clickable"
        );
        assert!(
            bear_groups.iter().all(|group| group.count == 1),
            "protected legal targets should not be grouped: {bear_groups:?}"
        );
    }

    #[test]
    fn battlefield_grouping_uses_current_controller_for_temporary_control_effects() {
        let _id_counter_guard = crate::test_id_counter_guard();
        use ironsmith::continuous::{ContinuousEffect, EffectTarget, Modification};
        use ironsmith::effect::Until;

        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let protected_ids = std::collections::HashSet::new();
        let bears_card = test_bears_card();
        let bear = game.create_object_from_card(&bears_card, bob, Zone::Battlefield);

        game.effect_store.continuous_effects.add_effect(
            ContinuousEffect::new(
                bear,
                alice,
                EffectTarget::Specific(bear),
                Modification::ChangeController(alice),
            )
            .until(Until::EndOfTurn)
            .with_expires_end_of_turn(game.turn.turn_number),
        );
        game.refresh_continuous_state();

        assert_eq!(
            game.current_controller(bear),
            Some(alice),
            "continuous control effect should make Alice the current controller"
        );

        let (alice_battlefield, alice_total) =
            grouped_battlefield_for_player(&game, alice, &protected_ids);
        let (bob_battlefield, bob_total) =
            grouped_battlefield_for_player(&game, bob, &protected_ids);

        assert_eq!(alice_total, 1, "Alice should see the stolen bear");
        assert!(
            alice_battlefield
                .iter()
                .any(|permanent| permanent.member_ids.contains(&bear.0)),
            "Alice's battlefield snapshot should contain the stolen bear: {alice_battlefield:?}"
        );
        assert_eq!(bob_total, 0, "Bob should no longer see the stolen bear");
        assert!(
            bob_battlefield.is_empty(),
            "Bob's battlefield snapshot should not contain the stolen bear: {bob_battlefield:?}"
        );
    }

    #[test]
    fn type_line_display_compacts_all_creature_types() {
        let mut subtypes = vec![Subtype::Shapeshifter];
        for subtype in Subtype::all_creature_types() {
            if !subtypes.contains(subtype) {
                subtypes.push(*subtype);
            }
        }

        let (display, badges) = format_type_line_display_parts(
            &[],
            &[CardType::Creature],
            &subtypes,
            &[Subtype::Shapeshifter],
        );

        assert_eq!(display, "Creature - Shapeshifter");
        assert_eq!(badges, vec!["All creature types"]);
    }

    #[test]
    fn type_line_display_keeps_normal_subtypes_inline() {
        let (display, badges) = format_type_line_display_parts(
            &[],
            &[CardType::Creature],
            &[Subtype::Angel, Subtype::Advisor],
            &[Subtype::Angel],
        );

        assert_eq!(display, "Creature - Angel Advisor");
        assert!(badges.is_empty());
    }
}
