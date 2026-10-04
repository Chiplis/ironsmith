use ironsmith::game_state::{
    ArchenemyState, ArchenemyVariant, ConspiracyState, HiddenCardInfo, Phase, PlanarCardKind,
    PlanechaseState, Step, TurnState, VanguardState,
};
use ironsmith::ids::{IdCountersSnapshot, StableId};
use ironsmith::object::{AttachmentTarget, Object};
use ironsmith::player::ManaPool;
use ironsmith::turn_runner::{TurnRunner, TurnState as RunnerTurnState};
use ironsmith::types::Subtype;
use sha2::{Digest, Sha256};

// Version 1 conflated effective control with an initial-controller assignment.
// That information cannot be recovered losslessly from its object snapshots.
// Version 3 distinguishes full executable checkpoints from perspective metadata
// carriers and requires a full payload rather than silently losing programs.
const SYNC_CHECKPOINT_VERSION: u32 = 4;
const PUBLIC_AUDIT_CHECKPOINT_VERSION: u32 = 2;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SyncIdCounters {
    player: u8,
    object: u64,
    card: u32,
    /// Next reserved stack-ability target id; absent in older checkpoints.
    #[serde(default)]
    stack_ability: u64,
}

impl From<IdCountersSnapshot> for SyncIdCounters {
    fn from(value: IdCountersSnapshot) -> Self {
        Self {
            player: value.player,
            object: value.object,
            card: value.card,
            stack_ability: 0,
        }
    }
}

impl SyncIdCounters {
    fn from_game(game: &ironsmith::game_state::GameState) -> Self {
        let mut counters = Self::from(ironsmith::ids::snapshot_id_counters());
        counters.object = game.next_object_id_counter();
        counters.stack_ability = game.next_stack_ability_id_counter();
        counters
    }
}

impl From<SyncIdCounters> for IdCountersSnapshot {
    fn from(value: SyncIdCounters) -> Self {
        Self {
            player: value.player,
            object: value.object,
            card: value.card,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SyncManaPool {
    white: u32,
    blue: u32,
    black: u32,
    red: u32,
    green: u32,
    colorless: u32,
}

impl From<&ManaPool> for SyncManaPool {
    fn from(value: &ManaPool) -> Self {
        Self {
            white: value.white,
            blue: value.blue,
            black: value.black,
            red: value.red,
            green: value.green,
            colorless: value.colorless,
        }
    }
}

impl From<SyncManaPool> for ManaPool {
    fn from(value: SyncManaPool) -> Self {
        Self {
            white: value.white,
            blue: value.blue,
            black: value.black,
            red: value.red,
            green: value.green,
            colorless: value.colorless,
        }
    }
}

// Mana restrictions without executable spend payloads use the shared value
// model directly. Never turn a restriction/bonus into unrestricted mana when
// its executable payload cannot be represented by this checkpoint.
type SyncRestrictedManaUnit = ironsmith_core::RestrictedManaUnit<()>;
fn sync_restricted_mana(
    units: &[ironsmith::ability::RestrictedManaUnit],
) -> Result<Vec<SyncRestrictedManaUnit>, String> {
    units
        .iter()
        .map(|unit| {
            Ok(SyncRestrictedManaUnit {
                symbol: unit.symbol,
                source: unit.source,
                source_chosen_creature_type: unit.source_chosen_creature_type,
                restrictions: unit
                    .restrictions
                    .iter()
                    .cloned()
                    .map(|restriction| {
                        restriction.try_map_effects(&mut |_| {
                            Err(
                                "mana spend payload requires an approved executable identity graph"
                                    .to_string(),
                            )
                        })
                    })
                    .collect::<Result<_, String>>()?,
            })
        })
        .collect()
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SyncPlayer {
    id: u8,
    name: String,
    starting_life: i32,
    life: i32,
    mana_pool: SyncManaPool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    restricted_mana: Vec<SyncRestrictedManaUnit>,
    poison_counters: u32,
    energy_counters: u32,
    experience_counters: u32,
    ring_temptations: u32,
    lands_played_this_turn: u32,
    land_plays_per_turn: u32,
    max_hand_size: i32,
    has_lost: bool,
    has_won: bool,
    has_left_game: bool,
    library: Vec<u64>,
    hand: Vec<u64>,
    graveyard: Vec<u64>,
    sideboard: Vec<u64>,
    commanders: Vec<u64>,
    /// Commander color identities fixed at designation (CR 903.4a), in
    /// commander-id order.
    #[serde(default)]
    commander_color_identities: Vec<(u64, ironsmith::color::ColorSet)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SyncTurn {
    active_player: u8,
    priority_player: Option<u8>,
    turn_number: u32,
    phase: String,
    step: Option<String>,
    /// Seating order rotated onto the seat that took the first turn, so a
    /// randomly chosen starting player survives a checkpoint round trip.
    #[serde(default)]
    turn_order: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
enum SyncAttachmentTarget {
    Object { object: u64 },
    Player { player: u8 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SyncCounter {
    kind: String,
    amount: u32,
    /// Exact identity; display names cannot distinguish named and built-in kinds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    counter_type: Option<ironsmith::CounterType>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SyncCounterAbilityOrigin {
    kind: String,
    serial: Vec<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    counter_type: Option<ironsmith::CounterType>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SyncCounterAbilityState {
    next_serial: Vec<u32>,
    origins: Vec<SyncCounterAbilityOrigin>,
}
impl SyncCounterAbilityState {
    fn from_object(object: &Object) -> Self {
        let state = object.counters.ability_state();
        Self {
            next_serial: state.next_serial,
            origins: state.origins.into_iter().map(|origin| SyncCounterAbilityOrigin {
                kind: sync_counter_kind(origin.counter_type), serial: origin.serial,
                counter_type: Some(origin.counter_type),
            }).collect(),
        }
    }
    fn into_runtime(self) -> Result<ironsmith::object::CounterAbilityState, String> {
        Ok(ironsmith::object::CounterAbilityState {
            next_serial: self.next_serial,
            origins: self.origins.into_iter().map(|origin| {
                Ok(ironsmith::object::CounterAbilityOrigin {
                    counter_type: sync_counter_from_wire(&origin.kind, origin.counter_type)?,
                    serial: origin.serial,
                })
            }).collect::<Result<Vec<_>, String>>()?,
        })
    }
}

// Both metadata and executable imports validate the same typed counter facts.
// This does not replace retained executable ability occurrences with rebuilt ones.
fn sync_counters_from_checkpoint(object: &SyncObject) -> Result<ironsmith::object::ObjectCounters, String> {
    let mut counts = std::collections::BTreeMap::new();
    for counter in &object.counters {
        let kind = sync_counter_from_wire(&counter.kind, counter.counter_type)?;
        if counts.insert(kind, counter.amount).is_some() {
            return Err("duplicate counter kind in checkpoint".into());
        }
    }
    let registrations = object.counter_ability_state.clone()
        .map(SyncCounterAbilityState::into_runtime).transpose()?;
    ironsmith::object::ObjectCounters::from_checkpoint(counts, registrations)
        .map_err(|error| format!("invalid counter registrations: {error}"))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SyncObject {
    id: u64,
    stable_id: u64,
    owner: u8,
    initial_controller: u8,
    controller: u8,
    zone: String,
    name: String,
    /// Physical card definition, independent of the face or copied name now shown.
    /// Omitted for unrevealed placeholders and removed by perspective redaction.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    original_card_name: Option<String>,
    token: bool,
    card_types: Vec<String>,
    subtypes: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    chosen_subtype: Option<Subtype>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    chosen_subtypes: Vec<Subtype>,
    power: Option<i32>,
    toughness: Option<i32>,
    loyalty: Option<u32>,
    defense: Option<u32>,
    #[serde(default)]
    hand_modifier: i32,
    #[serde(default)]
    life_modifier: i32,
    oracle_text: String,
    counters: Vec<SyncCounter>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    counter_ability_state: Option<SyncCounterAbilityState>,
    attached_to: Option<SyncAttachmentTarget>,
    attachments: Vec<u64>,
    tapped: bool,
    summoning_sick: bool,
    monstrous: bool,
    renowned: bool,
    #[serde(default)]
    saga_entry_lore_processed: bool,
    saddled: bool,
    flipped: bool,
    face_down: bool,
    manifested: bool,
    phased_out: bool,
    madness_exiled: bool,
    foretold: bool,
    /// Turn the card became foretold (CR 702.143a: castable only on a later turn).
    #[serde(default)]
    foretold_turn: Option<u32>,
    #[serde(default)]
    suspected: bool,
    #[serde(default)]
    prepared: bool,
    /// Set on a prepare spell copy in exile: the prepared permanent it belongs
    /// to. The link is restored rather than recreated, because the copy itself
    /// is already part of the restored exile zone.
    #[serde(default)]
    prepared_spell_source: Option<u64>,
    /// Class level designation (CR 716.2b); 0/1 means level 1.
    #[serde(default)]
    class_level: u32,
    /// Room with neither door unlocked (CR 709.5d).
    #[serde(default)]
    room_no_unlocked_door: bool,
    /// Room with both doors unlocked (CR 709.5e).
    #[serde(default)]
    room_fully_unlocked: bool,
    /// Solved Case designation (CR 719.3).
    #[serde(default)]
    case_solved: bool,
    plotted_by: Option<u8>,
    plotted_turn: Option<u32>,
    damage_marked: u32,
    commander: bool,
    #[serde(default)]
    hidden_card: Option<SyncHiddenCard>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SyncHiddenCard {
    owner: u8,
    slot: u16,
    commitment: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    origin_slot: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    origin_commitment: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    public_slot: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    public_commitment: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PublicAuditPlayer {
    id: u8,
    name: String,
    starting_life: i32,
    life: i32,
    mana_pool: SyncManaPool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    restricted_mana: Vec<SyncRestrictedManaUnit>,
    poison_counters: u32,
    energy_counters: u32,
    experience_counters: u32,
    ring_temptations: u32,
    lands_played_this_turn: u32,
    land_plays_per_turn: u32,
    max_hand_size: i32,
    has_lost: bool,
    has_won: bool,
    has_left_game: bool,
    library_count: usize,
    hand_count: usize,
    sideboard_count: usize,
    graveyard: Vec<u64>,
    commanders: Vec<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PublicAuditObjectIdentity {
    name: String,
    card_types: Vec<String>,
    subtypes: Vec<String>,
    oracle_text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PublicAuditObject {
    id: u64,
    stable_id: u64,
    owner: u8,
    initial_controller: u8,
    controller: u8,
    zone: String,
    identity: Option<PublicAuditObjectIdentity>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    chosen_subtype: Option<Subtype>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    chosen_subtypes: Vec<Subtype>,
    token: bool,
    power: Option<i32>,
    toughness: Option<i32>,
    loyalty: Option<u32>,
    defense: Option<u32>,
    counters: Vec<SyncCounter>,
    attached_to: Option<SyncAttachmentTarget>,
    attachments: Vec<u64>,
    tapped: bool,
    summoning_sick: bool,
    monstrous: bool,
    renowned: bool,
    #[serde(default)]
    saga_entry_lore_processed: bool,
    saddled: bool,
    flipped: bool,
    face_down: bool,
    manifested: bool,
    phased_out: bool,
    madness_exiled: bool,
    foretold: bool,
    /// Turn the card became foretold (CR 702.143a: castable only on a later turn).
    #[serde(default)]
    foretold_turn: Option<u32>,
    #[serde(default)]
    suspected: bool,
    #[serde(default)]
    prepared: bool,
    plotted_by: Option<u8>,
    plotted_turn: Option<u32>,
    damage_marked: u32,
    commander: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PublicAuditHiddenZone {
    owner: u8,
    zone: String,
    count: usize,
    protocol: String,
    commitment_root: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PublicAuditCheckpoint {
    version: u32,
    format: MatchFormatInput,
    perspective: u8,
    snapshot_serial: u64,
    turn: SyncTurn,
    priority_runtime: SyncPriorityRuntime,
    players: Vec<PublicAuditPlayer>,
    objects: Vec<PublicAuditObject>,
    battlefield: Vec<u64>,
    public_exile: Vec<u64>,
    command: Vec<u64>,
    ante: Vec<u64>,
    #[serde(default)]
    planechase: Option<PublicAuditPlanechase>,
    #[serde(default)]
    vanguard: Option<SyncVanguard>,
    #[serde(default)]
    archenemy: Option<PublicAuditArchenemy>,
    #[serde(default)]
    conspiracy: Option<PublicAuditConspiracy>,
    #[serde(default)]
    free_for_all: Option<SyncFreeForAll>,
    #[serde(default)]
    team_vs_team: Option<SyncTeamVsTeam>,
    #[serde(default)]
    emperor: Option<SyncEmperor>,
    #[serde(default)]
    two_headed_giant: Option<SyncTwoHeadedGiant>,
    #[serde(default)]
    alternating_teams: Option<SyncAlternatingTeams>,
    #[serde(default)]
    grand_melee: Option<SyncGrandMelee>,
    stack: Vec<SyncStackEntry>,
    hidden_zones: Vec<PublicAuditHiddenZone>,
    /// SHA-256 (hex) of the canonical JSON of the shared hidden-claim ledger
    /// (obligations, face-down cast claims, claim subjects, library anchor
    /// keys; see `PublicHiddenClaimLedger`). Identical on every peer, so the
    /// checkpoint hash every signed action carries commits to the ledger.
    /// Omitted while the ledger is empty.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    hidden_claim_ledger_digest: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PublicAuditPlanechase {
    decks: Vec<(u8, usize)>,
    communal_deck_size: Option<usize>,
    face_up: Vec<u64>,
    planar_controller: u8,
    planar_controllers: Vec<u8>,
    face_up_controllers: Vec<(u64, u8)>,
    voluntary_rolls_this_turn: Vec<(u8, u32)>,
    planeswalk_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PublicAuditArchenemy {
    variant: String,
    archenemies: Vec<u8>,
    decks: Vec<(u8, usize)>,
    face_up: Vec<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PublicAuditConspiracy {
    cards: Vec<(u8, Vec<u64>)>,
    face_down: Vec<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SyncStackEntry {
    object_id: u64,
    /// The ability's own target id; absent in older checkpoints.
    #[serde(default)]
    ability_id: Option<u64>,
    #[serde(default)]
    ninjutsu_attack_target: Option<SyncGrandMeleeAttackTarget>,
    controller: u8,
    targets: Vec<SyncTarget>,
    is_ability: bool,
    x_value: Option<u32>,
    source_stable_id: Option<u64>,
    source_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
enum SyncTarget {
    Player { player: u8 },
    Object { object: u64 },
}


#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SyncContinuousTimestamps {
    current_timestamp: u64,
    object_entries: Vec<(u64, u64)>,
    counters: Vec<(u64, String, u64)>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    typed_counters: Option<Vec<(u64, ironsmith::CounterType, u64)>>,
    attachments: Vec<(u64, u64)>,
}
impl SyncContinuousTimestamps {
    fn from_game(game: &GameState) -> Self {
        let state = game.effect_store.continuous_effects.timestamp_state();
        Self {
            current_timestamp: state.current_timestamp,
            object_entries: state.object_entries.into_iter().map(|(object, time)| (object.0, time)).collect(),
            counters: state.counters.iter().map(|((object, kind), time)|
                (object.0, sync_counter_kind(*kind), *time)).collect(),
            typed_counters: Some(state.counters.into_iter().map(|((object, kind), time)|
                (object.0, kind, time)).collect()),
            attachments: state.attachments.into_iter().map(|(object, time)| (object.0, time)).collect(),
        }
    }
    fn into_runtime(self) -> Result<ironsmith::continuous::ContinuousTimestampState, String> {
        let counters = match self.typed_counters {
            Some(typed) => {
                let descriptive: Vec<_> = typed.iter().map(|(object, kind, time)|
                    (*object, sync_counter_kind(*kind), *time)).collect();
                if descriptive != self.counters {
                    return Err("typed counter chronology disagrees with display records".into());
                }
                typed.into_iter().map(|(object, kind, time)|
                    ((ObjectId::from_raw(object), kind), time)).collect()
            }
            None => self.counters.into_iter().map(|(object, kind, time)|
                ((ObjectId::from_raw(object), sync_counter_from_name(&kind)), time)).collect(),
        };
        Ok(ironsmith::continuous::ContinuousTimestampState {
            current_timestamp: self.current_timestamp,
            object_entries: self.object_entries.into_iter().map(|(object, time)|
                (ObjectId::from_raw(object), time)).collect(),
            counters,
            attachments: self.attachments.into_iter().map(|(object, time)|
                (ObjectId::from_raw(object), time)).collect(),
        })
    }
}

/// Complete native executable roots for the owning checkpoint. This carrier is
/// staged separately from publication until perspective reachability is wired.
/// Graph slots are sorted by source CardId and receiver IDs must preserve that
/// order: current linked-face semantics still give allocation order meaning.

type SyncStackHistory = ironsmith_runtime_catalog::artifact_materializer::RetainedOccurrenceObjectSnapshot<u32>;
type SyncRetainedEvent = ironsmith::events::raw_event::RetainedRawEvent<u32, SyncStackHistory>;
type SyncRetainedStack = ironsmith::game_state::RetainedStackEntry<
    ironsmith_runtime_catalog::artifact_materializer::RetainedOccurrenceProgram,
    SyncStackHistory, SyncRetainedEvent,
    ironsmith::effect::RetainedEffectOutcome<SyncRetainedEvent>,
    ironsmith_core::ManaUsageRestriction<ironsmith_runtime_catalog::artifact_materializer::RetainedOccurrenceEffect>,
>;
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", deny_unknown_fields)]
enum SyncEventBody {
    AbilityActivatedEvent {
        source: ObjectId,
        activator: PlayerId,
        is_mana_ability: bool,
        is_loyalty_ability: bool,
        activation_cost_has_x: bool,
        activation_cost_has_tap: bool,
        #[serde(deserialize_with="sync_event_body_required_option")]
        x_value: Option<u32>,
        #[serde(deserialize_with="sync_event_body_required_option")]
        stack_entry_provenance: Option<ironsmith::provenance::ProvNodeId>,
        #[serde(deserialize_with="sync_event_body_required_option")]
        snapshot: Option<SyncStackHistory>,
        #[serde(deserialize_with="sync_event_body_required_option")]
        activated_ability: Option<ironsmith_runtime_catalog::artifact_materializer::RetainedOccurrenceAbility>,
        mana_sources_spent: Vec<SyncStackHistory>,
        mana_spent_total: u32,
    },
    CreateTokensEvent {
        controller: PlayerId,
        count: u32,
        cause: ironsmith::events::cause::EventCause,
        #[serde(deserialize_with="sync_event_body_required_option")]
        token: Option<ironsmith_runtime_catalog::artifact_materializer::RetainedOccurrenceLiveObject<u32>>,
        additional_tokens: Vec<(ironsmith_core::AdditionalTokenKind, u32)>,
    },
    EnterBattlefieldEvent { value: ironsmith_runtime_catalog::artifact_materializer::RetainedOccurrenceEntryEvent<u32> },
    DamagePreventedEvent {
        damage_source: ObjectId,
        target: SyncTarget,
        amount: u32,
        prevention_source: ObjectId,
        prevention_controller: PlayerId,
        is_combat: bool,
        #[serde(deserialize_with="sync_event_body_required_option")]
        target_snapshot: Option<SyncStackHistory>,
        applications: Vec<SyncPreventedDamage>,
        #[serde(deserialize_with="sync_event_body_required_option")]
        prevention_shield: Option<ironsmith::prevention::PreventionShieldId>,
    },
    KeywordActionEvent {
        action: ironsmith_core::KeywordActionKind,
        player: PlayerId,
        source: ObjectId,
        amount: u32,
        #[serde(deserialize_with="sync_event_body_required_option")]
        votes: Option<Vec<SyncPlayerVote>>,
        #[serde(deserialize_with="sync_event_body_required_option")]
        snapshot: Option<SyncStackHistory>,
        player_tags: std::collections::BTreeMap<ironsmith::tag::TagKey, Vec<PlayerId>>,
        object_tags: std::collections::BTreeMap<ironsmith::tag::TagKey, Vec<SyncStackHistory>>,
        #[serde(deserialize_with="sync_event_body_required_option")]
        combat_phase: Option<u32>,
        #[serde(deserialize_with="sync_event_body_required_option")]
        unlocked_door_triggers: Option<Vec<u64>>,
        #[serde(deserialize_with="sync_event_body_required_option")]
        unlocked_door_ability_range: Option<std::ops::Range<usize>>,
        #[serde(deserialize_with="sync_event_body_required_option")]
        x_value: Option<u32>,
        voter_teams: Vec<(PlayerId, usize)>,
    },
    MarkersChangedEvent {
        change_type: SyncMarkerChange,
        marker: ironsmith::CounterType,
        location: SyncTarget,
        amount: u32,
        #[serde(deserialize_with="sync_event_body_required_option")]
        count_after: Option<u32>,
        #[serde(deserialize_with="sync_event_body_required_option")]
        source: Option<ObjectId>,
        #[serde(deserialize_with="sync_event_body_required_option")]
        source_controller: Option<PlayerId>,
    },
    PlayersFinishedVotingEvent {
        source: ObjectId,
        controller: PlayerId,
        votes: Vec<SyncPlayerVote>,
        vote_counts: Vec<(usize, usize)>,
        option_names: Vec<String>,
        player_tags: std::collections::BTreeMap<ironsmith::tag::TagKey, Vec<PlayerId>>,
        voter_teams: Vec<(PlayerId, usize)>,
    },
    AbilityTriggeredEvent {
        source: ObjectId,
        source_stable_id: StableId,
        controller: PlayerId,
        trigger_identity: u64,
        #[serde(deserialize_with="sync_event_body_required_option")]
        source_snapshot: Option<SyncStackHistory>,
        #[serde(deserialize_with="sync_event_body_required_option")]
        cause_kind: Option<ironsmith::events::EventKind>,
        #[serde(deserialize_with="sync_event_body_required_option")]
        cause_object: Option<ObjectId>,
        #[serde(deserialize_with="sync_event_body_required_option")]
        zone_change_cause: Option<SyncTriggerZoneCause>,
    },
    CreatureAttackedEvent {
        attacker: ObjectId,
        target: SyncAttackEventTarget,
        total_attackers: usize,
        #[serde(deserialize_with="sync_event_body_required_option")]
        declared_attackers: Option<Vec<SyncDeclaredAttacker>>,
    },
    CreatureAttackedAndUnblockedEvent {
        attacker: ObjectId,
        target: SyncAttackEventTarget,
    },
    CreatureBecameBlockedEvent {
        attacker: ObjectId,
        blocker_count: u32,
        blockers: Vec<ObjectId>,
        #[serde(deserialize_with="sync_event_body_required_option")]
        attack_target: Option<SyncAttackEventTarget>,
        #[serde(deserialize_with="sync_event_body_required_option")]
        attacker_snapshot: Option<SyncStackHistory>,
        blocker_snapshots: Vec<SyncStackHistory>,
    },
    ManaAddedEvent {
        source: ObjectId,
        controller: PlayerId,
        player: PlayerId,
        mana: Vec<ironsmith::mana::ManaSymbol>,
        #[serde(deserialize_with="sync_event_body_required_option")]
        snapshot: Option<SyncStackHistory>,
        provenance: ironsmith::events::mana::ManaProductionProvenance,
    },
    ManaUnitSpentEvent {
        player: PlayerId,
        mana_source: ObjectId,
        #[serde(deserialize_with="sync_event_body_required_option")]
        payment_source: Option<ObjectId>,
        symbol: ironsmith::mana::ManaSymbol,
        purpose: ironsmith::ability::ManaPaymentPurpose,
        #[serde(deserialize_with="sync_event_body_required_option")]
        source_snapshot: Option<SyncStackHistory>,
    },
    ObjectBecameUnattachedEvent {
        object: ObjectId,
        previous_target: ironsmith::object::AttachmentTarget,
        controller: PlayerId,
        #[serde(deserialize_with="sync_event_body_required_option")]
        snapshot: Option<SyncStackHistory>,
    },
    CreatureBlockedEvent {
        blocker: ObjectId,
        attacker: ObjectId,
        #[serde(deserialize_with = "sync_event_body_required_option")]
        blocker_snapshot: Option<SyncStackHistory>,
        #[serde(deserialize_with = "sync_event_body_required_option")]
        attacker_snapshot: Option<SyncStackHistory>,
    },
    CardDiscardedEvent {
        player: PlayerId,
        card: ObjectId,
        #[serde(deserialize_with = "sync_event_body_required_option")]
        cause: Option<ironsmith::events::cause::EventCause>,
        #[serde(deserialize_with = "sync_event_body_required_option")]
        snapshot: Option<SyncStackHistory>,
        batch_cards: Vec<ObjectId>,
        batch_snapshots: Vec<SyncStackHistory>,
        #[serde(deserialize_with = "sync_event_body_required_option")]
        batch_index: Option<usize>,
    },
    CardRevealedEvent {
        player: PlayerId,
        card: ObjectId,
        zone: Zone,
        #[serde(deserialize_with = "sync_event_body_required_option")]
        source: Option<ObjectId>,
        #[serde(deserialize_with = "sync_event_body_required_option")]
        snapshot: Option<SyncStackHistory>,
        #[serde(deserialize_with = "sync_event_body_required_option")]
        reveal_context_amount: Option<i32>,
    },
    PermanentPhasedOutEvent {
        permanent: ObjectId,
        controller: PlayerId,
        #[serde(deserialize_with = "sync_event_body_required_option")]
        snapshot: Option<SyncStackHistory>,
    },
    SpellCounteredEvent {
        spell: ObjectId,
        controller: PlayerId,
        #[serde(deserialize_with = "sync_event_body_required_option")]
        snapshot: Option<SyncStackHistory>,
    },
    DestroyEvent {
        permanent: ObjectId,
        #[serde(deserialize_with = "sync_event_body_required_option")]
        source: Option<ObjectId>,
        #[serde(deserialize_with = "sync_event_body_required_option")]
        snapshot: Option<SyncStackHistory>,
        #[serde(deserialize_with = "sync_event_body_required_option")]
        final_zone: Option<Zone>,
    },
    SacrificeEvent {
        permanent: ObjectId,
        #[serde(deserialize_with = "sync_event_body_required_option")]
        source: Option<ObjectId>,
        #[serde(deserialize_with = "sync_event_body_required_option")]
        snapshot: Option<SyncStackHistory>,
        #[serde(deserialize_with = "sync_event_body_required_option")]
        sacrificing_player: Option<PlayerId>,
    },
    SpellCastEvent {
        spell: ObjectId,
        caster: PlayerId,
        from_zone: Zone,
        #[serde(deserialize_with = "sync_event_body_required_option")]
        snapshot: Option<SyncStackHistory>,
    },
    ObjectLeavesGameEvent {
        object: ObjectId,
        snapshot: SyncStackHistory,
        cause: ironsmith::events::cause::EventCause,
    },
    ZoneChangeEvent {
        objects: Vec<ObjectId>,
        result_objects: Vec<ObjectId>,
        from: Zone,
        to: Zone,
        cause: ironsmith::events::cause::EventCause,
        #[serde(deserialize_with = "sync_event_body_required_option")]
        snapshot: Option<SyncStackHistory>,
        snapshots: Vec<SyncStackHistory>,
        object_tags: std::collections::BTreeMap<ironsmith::tag::TagKey, Vec<SyncStackHistory>>,
    },
    DiscardEvent {
        card: ObjectId,
        player: PlayerId,
        destination: Zone,
        cause: ironsmith::events::cause::EventCause,
        requires_type_verification: bool,
        madness_applied: bool,
    },
    DrawEvent {
        player: PlayerId,
        count: u32,
        is_first_this_turn: bool,
        first_of_instruction: bool,
        first_of_draw_step: bool,
    },
    MoveCountersEvent {
        from: ObjectId,
        to: ObjectId,
        #[serde(deserialize_with = "sync_event_body_required_option")]
        counter_type: Option<ironsmith::CounterType>,
        #[serde(deserialize_with = "sync_event_body_required_option")]
        count: Option<u32>,
    },
    PutCountersEvent {
        target: ironsmith::game_state::Target,
        counter_type: ironsmith::CounterType,
        count: u32,
        #[serde(deserialize_with = "sync_event_body_required_option")]
        maximum_count: Option<u32>,
        cause: ironsmith::events::cause::EventCause,
    },
    RemoveCountersEvent {
        target: ObjectId,
        counter_type: ironsmith::CounterType,
        count: u32,
    },
    BecameMonstrousEvent {
        creature: ObjectId,
        controller: PlayerId,
        n: u32,
    },
    CardsDrawnEvent {
        player: PlayerId,
        cards: Vec<ObjectId>,
        is_first_this_turn: bool,
        is_during_players_draw_step: bool,
        cards_previously_drawn_this_draw_step: u32,
    },
    ChapterAbilityResolvedEvent {
        saga: ObjectId,
        controller: PlayerId,
        final_chapter: bool,
    },
    CoinFlippedEvent {
        player: PlayerId,
        source: ObjectId,
        face: ironsmith_core::CoinFace,
        #[serde(deserialize_with = "sync_event_body_required_option")]
        call: Option<ironsmith_core::CoinFace>,
        #[serde(deserialize_with = "sync_event_body_required_option")]
        winner: Option<PlayerId>,
        #[serde(deserialize_with = "sync_event_body_required_option")]
        loser: Option<PlayerId>,
    },
    ControlChangedEvent {
        permanent: ObjectId,
        previous_controller: PlayerId,
        new_controller: PlayerId,
    },
    ConvertedEvent {
        permanent: ObjectId,
    },
    CounterPlacedEvent {
        permanent: ObjectId,
        counter_type: ironsmith::CounterType,
        amount: u32,
        #[serde(deserialize_with = "sync_event_body_required_option")]
        previous_count: Option<u32>,
    },
    DayNightChangedEvent {
        is_daytime: bool,
    },
    DieRolledEvent {
        player: PlayerId,
        source: ObjectId,
        natural_result: u32,
        result: u32,
        sides: u32,
        is_planar: bool,
        is_attraction_visit: bool,
    },
    GiftGivenEvent {
        player: PlayerId,
        recipient: PlayerId,
        source: ObjectId,
    },
    LandPlayedEvent {
        land: ObjectId,
        player: PlayerId,
        from_zone: Zone,
    },
    MutatedEvent {
        permanent: ObjectId,
        controller: PlayerId,
    },
    PermanentTappedEvent {
        permanent: ObjectId,
    },
    PermanentUntappedEvent {
        permanent: ObjectId,
    },
    PlayerLosesGameEvent {
        player: PlayerId,
    },
    SearchLibraryEvent {
        player: PlayerId,
        #[serde(deserialize_with = "sync_event_body_required_option")]
        library_owner: Option<PlayerId>,
    },
    ShuffleLibraryEvent {
        player: PlayerId,
        cause: ironsmith::events::cause::EventCause,
    },
    StateTriggerEvent {
        source: ObjectId,
    },
    TransformedEvent {
        permanent: ObjectId,
    },
    TurnedFaceUpEvent {
        permanent: ObjectId,
        player: PlayerId,
    },
    TapEvent {
        permanent: ObjectId,
    },
    UntapEvent {
        permanent: ObjectId,
    },
    BeginningOfCleanupStepEvent {
        player: PlayerId,
    },
    BeginningOfDrawStepEvent {
        player: PlayerId,
    },
    BeginningOfPrecombatMainPhaseEvent {
        player: PlayerId,
    },
    BeginningOfPostcombatMainPhaseEvent {
        player: PlayerId,
        #[serde(deserialize_with = "sync_event_body_required_option")]
        main_phase_ordinal: Option<u32>,
    },
    EndOfCombatEvent,
    PermanentsUntapStepEvent {
        player: PlayerId,
    },
    BecomesTargetedEvent {
        target: ironsmith::game_state::Target,
        source: ObjectId,
        source_controller: PlayerId,
        by_ability: bool,
        #[serde(deserialize_with = "sync_event_body_required_option")]
        stack_ability: Option<ObjectId>,
    },
    SpellCopiedEvent {
        spell: ObjectId,
        copier: PlayerId,
    },
    BeginningOfEndStep { player: PlayerId },
    BeginningOfUpkeep { player: PlayerId },
    BeginningOfCombat { player: PlayerId },
    LifeGain { player: PlayerId, amount: u32,
        #[serde(deserialize_with = "sync_event_body_required_option")]
        source: Option<ObjectId> },
    LifeLoss { player: PlayerId, amount: u32, from_damage: bool, from_radiation: bool },
    Damage { source: ObjectId, target: SyncTarget, amount: u32, excess_damage: u32,
        is_combat: bool, is_unpreventable: bool, cause: ironsmith::events::cause::EventCause,
        #[serde(deserialize_with = "sync_event_body_required_option")]
        remainder: Option<(SyncTarget, u32)>,
        #[serde(deserialize_with = "sync_event_body_required_option")]
        target_snapshot: Option<SyncStackHistory> },
}
fn sync_event_body_required_option<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where D: serde::Deserializer<'de>, T: serde::Deserialize<'de> {
    <Option<T> as serde::Deserialize>::deserialize(deserializer)
}
fn validate_sync_event_body_actors(body: &SyncEventBody, player_count: usize) -> Result<(), String> {
    let check = |player: PlayerId| if usize::from(player.0) < player_count { Ok(()) } else { Err("stack event body has invalid actor".to_string()) };
    match body {
        SyncEventBody::DiscardEvent { card, player, destination, cause, requires_type_verification, madness_applied } => { check(*player)?; if let Some(actor) = cause.source_controller { check(actor)?; } Ok(()) },
        SyncEventBody::DrawEvent { player, count, is_first_this_turn, first_of_instruction, first_of_draw_step } => { check(*player)?; Ok(()) },
        SyncEventBody::PutCountersEvent { target, counter_type, count, maximum_count, cause } => { if let ironsmith::game_state::Target::Player(actor) = target { check(*actor)?; } if let Some(actor) = cause.source_controller { check(actor)?; } Ok(()) },
        SyncEventBody::BecameMonstrousEvent { creature, controller, n } => { check(*controller)?; Ok(()) },
        SyncEventBody::CardsDrawnEvent { player, cards, is_first_this_turn, is_during_players_draw_step, cards_previously_drawn_this_draw_step } => { check(*player)?; Ok(()) },
        SyncEventBody::ChapterAbilityResolvedEvent { saga, controller, final_chapter } => { check(*controller)?; Ok(()) },
        SyncEventBody::CoinFlippedEvent { player, source, face, call, winner, loser } => { check(*player)?; if let Some(actor) = winner { check(*actor)?; } if let Some(actor) = loser { check(*actor)?; } Ok(()) },
        SyncEventBody::ControlChangedEvent { permanent, previous_controller, new_controller } => { check(*previous_controller)?; check(*new_controller)?; Ok(()) },
        SyncEventBody::DieRolledEvent { player, source, natural_result, result, sides, is_planar, is_attraction_visit } => { check(*player)?; Ok(()) },
        SyncEventBody::GiftGivenEvent { player, recipient, source } => { check(*player)?; check(*recipient)?; Ok(()) },
        SyncEventBody::LandPlayedEvent { land, player, from_zone } => { check(*player)?; Ok(()) },
        SyncEventBody::MutatedEvent { permanent, controller } => { check(*controller)?; Ok(()) },
        SyncEventBody::PlayerLosesGameEvent { player } => { check(*player)?; Ok(()) },
        SyncEventBody::SearchLibraryEvent { player, library_owner } => { check(*player)?; if let Some(actor) = library_owner { check(*actor)?; } Ok(()) },
        SyncEventBody::ShuffleLibraryEvent { player, cause } => { check(*player)?; if let Some(actor) = cause.source_controller { check(actor)?; } Ok(()) },
        SyncEventBody::TurnedFaceUpEvent { permanent, player } => { check(*player)?; Ok(()) },
        SyncEventBody::BeginningOfCleanupStepEvent { player } => { check(*player)?; Ok(()) },
        SyncEventBody::BeginningOfDrawStepEvent { player } => { check(*player)?; Ok(()) },
        SyncEventBody::BeginningOfPrecombatMainPhaseEvent { player } => { check(*player)?; Ok(()) },
        SyncEventBody::BeginningOfPostcombatMainPhaseEvent { player, main_phase_ordinal } => { check(*player)?; Ok(()) },
        SyncEventBody::PermanentsUntapStepEvent { player } => { check(*player)?; Ok(()) },
        SyncEventBody::BecomesTargetedEvent { target, source, source_controller, by_ability, stack_ability } => { if let ironsmith::game_state::Target::Player(actor) = target { check(*actor)?; } check(*source_controller)?; Ok(()) },
        SyncEventBody::SpellCopiedEvent { spell, copier } => { check(*copier)?; Ok(()) },
        SyncEventBody::CardDiscardedEvent { player, cause, .. } => { check(*player)?; if let Some(actor) = cause.as_ref().and_then(|c| c.source_controller) { check(actor)?; } Ok(()) },
        SyncEventBody::CardRevealedEvent { player, .. } => { check(*player)?; Ok(()) },
        SyncEventBody::PermanentPhasedOutEvent { controller, .. } => { check(*controller)?; Ok(()) },
        SyncEventBody::SpellCounteredEvent { controller, .. } => { check(*controller)?; Ok(()) },
        SyncEventBody::SacrificeEvent { sacrificing_player, .. } => { if let Some(actor) = sacrificing_player { check(*actor)?; } Ok(()) },
        SyncEventBody::SpellCastEvent { caster, .. } => { check(*caster)?; Ok(()) },
        SyncEventBody::ObjectLeavesGameEvent { cause, .. } => { if let Some(actor) = cause.source_controller { check(actor)?; } Ok(()) },
        SyncEventBody::ZoneChangeEvent { cause, .. } => { if let Some(actor) = cause.source_controller { check(actor)?; } Ok(()) },
        _ => Ok(()),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag="kind", deny_unknown_fields)]
enum SyncAttackEventTarget { Player { player: PlayerId }, Planeswalker { object: ObjectId }, Battle { object: ObjectId }, Nothing }
impl SyncAttackEventTarget {
    fn retain(target: ironsmith::triggers::event::AttackEventTarget) -> Self {
        use ironsmith::triggers::event::AttackEventTarget as T;
        match target { T::Player(player) => Self::Player { player }, T::Planeswalker(object) => Self::Planeswalker { object }, T::Battle(object) => Self::Battle { object }, T::Nothing => Self::Nothing }
    }
    fn restore(self) -> ironsmith::triggers::event::AttackEventTarget {
        use ironsmith::triggers::event::AttackEventTarget as T;
        match self { Self::Player { player } => T::Player(player), Self::Planeswalker { object } => T::Planeswalker(object), Self::Battle { object } => T::Battle(object), Self::Nothing => T::Nothing }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag="kind", deny_unknown_fields)]
enum SyncDeclaredAttackTarget { Player { player: PlayerId }, Planeswalker { object: ObjectId }, Battle { object: ObjectId }, Nothing {
    #[serde(deserialize_with="sync_event_body_required_option")]
    defending_player: Option<PlayerId>, was_planeswalker: bool } }
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SyncDeclaredAttacker { creature: ObjectId, target: SyncDeclaredAttackTarget }
impl SyncDeclaredAttacker {
    fn retain(info: ironsmith::combat_state::AttackerInfo) -> Self {
        use ironsmith::combat_state::AttackTarget as T;
        let ironsmith::combat_state::AttackerInfo { creature, target } = info;
        let target = match target { T::Player(player) => SyncDeclaredAttackTarget::Player { player }, T::Planeswalker(object) => SyncDeclaredAttackTarget::Planeswalker { object }, T::Battle(object) => SyncDeclaredAttackTarget::Battle { object }, T::Nothing { defending_player, was_planeswalker } => SyncDeclaredAttackTarget::Nothing { defending_player, was_planeswalker } };
        Self { creature, target }
    }
    fn restore(self) -> ironsmith::combat_state::AttackerInfo {
        use ironsmith::combat_state::AttackTarget as T;
        let Self { creature, target } = self;
        let target = match target { SyncDeclaredAttackTarget::Player { player } => T::Player(player), SyncDeclaredAttackTarget::Planeswalker { object } => T::Planeswalker(object), SyncDeclaredAttackTarget::Battle { object } => T::Battle(object), SyncDeclaredAttackTarget::Nothing { defending_player, was_planeswalker } => T::Nothing { defending_player, was_planeswalker } };
        ironsmith::combat_state::AttackerInfo { creature, target }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SyncPlayerVote { player: PlayerId, option_index: usize, option_name: String,
    #[serde(deserialize_with="sync_event_body_required_option")] object_vote: Option<ObjectId> }
impl SyncPlayerVote {
 fn retain(value: ironsmith::events::other::PlayerVote) -> Self { let ironsmith::events::other::PlayerVote { player, option_index, option_name, object_vote } = value; Self { player, option_index, option_name, object_vote } }
 fn restore(self) -> ironsmith::events::other::PlayerVote { let Self { player, option_index, option_name, object_vote } = self; ironsmith::events::other::PlayerVote { player, option_index, option_name, object_vote } }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SyncTriggerZoneCause { from: Zone, to: Zone, destination_objects: Vec<ObjectId> }
impl SyncTriggerZoneCause {
 fn retain(value: ironsmith::events::spells::AbilityTriggerZoneChangeCause) -> Self { let ironsmith::events::spells::AbilityTriggerZoneChangeCause { from, to, destination_objects } = value; Self { from, to, destination_objects } }
 fn restore(self) -> ironsmith::events::spells::AbilityTriggerZoneChangeCause { let Self { from, to, destination_objects } = self; ironsmith::events::spells::AbilityTriggerZoneChangeCause { from, to, destination_objects } }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
enum SyncMarkerChange { Added, Removed }
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SyncPreventedDamage { damage_source: ObjectId, target: SyncTarget, amount: u32, is_combat: bool,
    #[serde(deserialize_with="sync_event_body_required_option")] target_snapshot: Option<SyncStackHistory> }
#[derive(Default)]
struct SyncEventArena {
    keys: std::collections::HashMap<usize, u32>,
    bodies: Vec<SyncEventBody>,
}
type StackBindingError = ironsmith_runtime_catalog::artifact_materializer::OccurrenceBindingError;
type SyncStackEncoder<'a, 'b, 'c> = (&'a mut ironsmith_runtime_catalog::artifact_materializer::StaticAbilityOccurrenceEncoder,
    &'b mut dyn FnMut(CardId) -> Result<u32, StackBindingError>, &'c mut SyncEventArena);
type SyncStackDecoder<'a, 'b, 'c> = (&'a ironsmith_runtime_catalog::artifact_materializer::StaticAbilityOccurrenceDecoder,
    &'b mut dyn FnMut(u32) -> Result<CardId, StackBindingError>, &'c [std::sync::Arc<dyn ironsmith::events::GameEventType>]);
fn stack_codec_error(detail: &str) -> StackBindingError {
    StackBindingError::InvalidModel { detail: detail.into() }
}
fn sync_damage_target(target: ironsmith::events::DamageTarget) -> SyncTarget {
    match target { ironsmith::events::DamageTarget::Player(player) => SyncTarget::Player { player: player.0 },
        ironsmith::events::DamageTarget::Object(object) => SyncTarget::Object { object: object.0 } }
}
fn damage_target_from_sync(target: SyncTarget) -> ironsmith::events::DamageTarget {
    match target { SyncTarget::Player { player } => ironsmith::events::DamageTarget::Player(PlayerId::from_index(player)),
        SyncTarget::Object { object } => ironsmith::events::DamageTarget::Object(ObjectId::from_raw(object)) }
}
fn retain_sync_event_body(context: &mut SyncStackEncoder<'_, '_, '_>, body: &dyn ironsmith::events::GameEventType) -> Result<SyncEventBody, StackBindingError> {
    use ironsmith::events::{phase, life, damage};
    if let Some(value) = body.as_any().downcast_ref::<phase::BeginningOfEndStepEvent>() {
        let phase::BeginningOfEndStepEvent { player } = value.clone(); return Ok(SyncEventBody::BeginningOfEndStep { player });
    }
    if let Some(value) = body.as_any().downcast_ref::<phase::BeginningOfUpkeepEvent>() {
        let phase::BeginningOfUpkeepEvent { player } = value.clone(); return Ok(SyncEventBody::BeginningOfUpkeep { player });
    }
    if let Some(value) = body.as_any().downcast_ref::<phase::BeginningOfCombatEvent>() {
        let phase::BeginningOfCombatEvent { player } = value.clone(); return Ok(SyncEventBody::BeginningOfCombat { player });
    }
    if let Some(value) = body.as_any().downcast_ref::<life::LifeGainEvent>() {
        let life::LifeGainEvent { player, amount, source } = value.clone(); return Ok(SyncEventBody::LifeGain { player, amount, source });
    }
    if let Some(value) = body.as_any().downcast_ref::<life::LifeLossEvent>() {
        let life::LifeLossEvent { player, amount, from_damage, from_radiation } = value.clone(); return Ok(SyncEventBody::LifeLoss { player, amount, from_damage, from_radiation });
    }
    if let Some(value) = body.as_any().downcast_ref::<damage::DamageEvent>() {
        let damage::DamageEvent { source, target, amount, excess_damage, is_combat, is_unpreventable, cause, remainder, target_snapshot } = value.clone();
        let target_snapshot = target_snapshot.map(|history| context.0.encode_snapshot(history, &mut *context.1)).transpose()?;
        return Ok(SyncEventBody::Damage { source, target: sync_damage_target(target), amount, excess_damage, is_combat, is_unpreventable, cause,
            remainder: remainder.map(|(target, amount)| (sync_damage_target(target), amount)), target_snapshot });
    }
    if let Some(value) = body.as_any().downcast_ref::<ironsmith::events::cards::DiscardEvent>() {
        let ironsmith::events::cards::DiscardEvent { card, player, destination, cause, requires_type_verification, madness_applied } = value.clone(); return Ok(SyncEventBody::DiscardEvent { card, player, destination, cause, requires_type_verification, madness_applied });
    }
    if let Some(value) = body.as_any().downcast_ref::<ironsmith::events::cards::DrawEvent>() {
        let ironsmith::events::cards::DrawEvent { player, count, is_first_this_turn, first_of_instruction, first_of_draw_step } = value.clone(); return Ok(SyncEventBody::DrawEvent { player, count, is_first_this_turn, first_of_instruction, first_of_draw_step });
    }
    if let Some(value) = body.as_any().downcast_ref::<ironsmith::events::counters::MoveCountersEvent>() {
        let ironsmith::events::counters::MoveCountersEvent { from, to, counter_type, count } = value.clone(); return Ok(SyncEventBody::MoveCountersEvent { from, to, counter_type, count });
    }
    if let Some(value) = body.as_any().downcast_ref::<ironsmith::events::counters::PutCountersEvent>() {
        let ironsmith::events::counters::PutCountersEvent { target, counter_type, count, maximum_count, cause } = value.clone(); return Ok(SyncEventBody::PutCountersEvent { target, counter_type, count, maximum_count, cause });
    }
    if let Some(value) = body.as_any().downcast_ref::<ironsmith::events::counters::RemoveCountersEvent>() {
        let ironsmith::events::counters::RemoveCountersEvent { target, counter_type, count } = value.clone(); return Ok(SyncEventBody::RemoveCountersEvent { target, counter_type, count });
    }
    if let Some(value) = body.as_any().downcast_ref::<ironsmith::events::other::BecameMonstrousEvent>() {
        let ironsmith::events::other::BecameMonstrousEvent { creature, controller, n } = value.clone(); return Ok(SyncEventBody::BecameMonstrousEvent { creature, controller, n });
    }
    if let Some(value) = body.as_any().downcast_ref::<ironsmith::events::other::CardsDrawnEvent>() {
        let ironsmith::events::other::CardsDrawnEvent { player, cards, is_first_this_turn, is_during_players_draw_step, cards_previously_drawn_this_draw_step } = value.clone(); return Ok(SyncEventBody::CardsDrawnEvent { player, cards, is_first_this_turn, is_during_players_draw_step, cards_previously_drawn_this_draw_step });
    }
    if let Some(value) = body.as_any().downcast_ref::<ironsmith::events::other::ChapterAbilityResolvedEvent>() {
        let ironsmith::events::other::ChapterAbilityResolvedEvent { saga, controller, final_chapter } = value.clone(); return Ok(SyncEventBody::ChapterAbilityResolvedEvent { saga, controller, final_chapter });
    }
    if let Some(value) = body.as_any().downcast_ref::<ironsmith::events::other::CoinFlippedEvent>() {
        let ironsmith::events::other::CoinFlippedEvent { player, source, face, call, winner, loser } = value.clone(); return Ok(SyncEventBody::CoinFlippedEvent { player, source, face, call, winner, loser });
    }
    if let Some(value) = body.as_any().downcast_ref::<ironsmith::events::other::ControlChangedEvent>() {
        let ironsmith::events::other::ControlChangedEvent { permanent, previous_controller, new_controller } = value.clone(); return Ok(SyncEventBody::ControlChangedEvent { permanent, previous_controller, new_controller });
    }
    if let Some(value) = body.as_any().downcast_ref::<ironsmith::events::other::ConvertedEvent>() {
        let ironsmith::events::other::ConvertedEvent { permanent } = value.clone(); return Ok(SyncEventBody::ConvertedEvent { permanent });
    }
    if let Some(value) = body.as_any().downcast_ref::<ironsmith::events::other::CounterPlacedEvent>() {
        let ironsmith::events::other::CounterPlacedEvent { permanent, counter_type, amount, previous_count } = value.clone(); return Ok(SyncEventBody::CounterPlacedEvent { permanent, counter_type, amount, previous_count });
    }
    if let Some(value) = body.as_any().downcast_ref::<ironsmith::events::other::DayNightChangedEvent>() {
        let ironsmith::events::other::DayNightChangedEvent { is_daytime } = value.clone(); return Ok(SyncEventBody::DayNightChangedEvent { is_daytime });
    }
    if let Some(value) = body.as_any().downcast_ref::<ironsmith::events::other::DieRolledEvent>() {
        let ironsmith::events::other::DieRolledEvent { player, source, natural_result, result, sides, is_planar, is_attraction_visit } = value.clone(); return Ok(SyncEventBody::DieRolledEvent { player, source, natural_result, result, sides, is_planar, is_attraction_visit });
    }
    if let Some(value) = body.as_any().downcast_ref::<ironsmith::events::other::GiftGivenEvent>() {
        let ironsmith::events::other::GiftGivenEvent { player, recipient, source } = value.clone(); return Ok(SyncEventBody::GiftGivenEvent { player, recipient, source });
    }
    if let Some(value) = body.as_any().downcast_ref::<ironsmith::events::other::LandPlayedEvent>() {
        let ironsmith::events::other::LandPlayedEvent { land, player, from_zone } = value.clone(); return Ok(SyncEventBody::LandPlayedEvent { land, player, from_zone });
    }
    if let Some(value) = body.as_any().downcast_ref::<ironsmith::events::other::MutatedEvent>() {
        let ironsmith::events::other::MutatedEvent { permanent, controller } = value.clone(); return Ok(SyncEventBody::MutatedEvent { permanent, controller });
    }
    if let Some(value) = body.as_any().downcast_ref::<ironsmith::events::other::PermanentTappedEvent>() {
        let ironsmith::events::other::PermanentTappedEvent { permanent } = value.clone(); return Ok(SyncEventBody::PermanentTappedEvent { permanent });
    }
    if let Some(value) = body.as_any().downcast_ref::<ironsmith::events::other::PermanentUntappedEvent>() {
        let ironsmith::events::other::PermanentUntappedEvent { permanent } = value.clone(); return Ok(SyncEventBody::PermanentUntappedEvent { permanent });
    }
    if let Some(value) = body.as_any().downcast_ref::<ironsmith::events::other::PlayerLosesGameEvent>() {
        let ironsmith::events::other::PlayerLosesGameEvent { player } = value.clone(); return Ok(SyncEventBody::PlayerLosesGameEvent { player });
    }
    if let Some(value) = body.as_any().downcast_ref::<ironsmith::events::other::SearchLibraryEvent>() {
        let ironsmith::events::other::SearchLibraryEvent { player, library_owner } = value.clone(); return Ok(SyncEventBody::SearchLibraryEvent { player, library_owner });
    }
    if let Some(value) = body.as_any().downcast_ref::<ironsmith::events::other::ShuffleLibraryEvent>() {
        let ironsmith::events::other::ShuffleLibraryEvent { player, cause } = value.clone(); return Ok(SyncEventBody::ShuffleLibraryEvent { player, cause });
    }
    if let Some(value) = body.as_any().downcast_ref::<ironsmith::events::other::StateTriggerEvent>() {
        let ironsmith::events::other::StateTriggerEvent { source } = value.clone(); return Ok(SyncEventBody::StateTriggerEvent { source });
    }
    if let Some(value) = body.as_any().downcast_ref::<ironsmith::events::other::TransformedEvent>() {
        let ironsmith::events::other::TransformedEvent { permanent } = value.clone(); return Ok(SyncEventBody::TransformedEvent { permanent });
    }
    if let Some(value) = body.as_any().downcast_ref::<ironsmith::events::other::TurnedFaceUpEvent>() {
        let ironsmith::events::other::TurnedFaceUpEvent { permanent, player } = value.clone(); return Ok(SyncEventBody::TurnedFaceUpEvent { permanent, player });
    }
    if let Some(value) = body.as_any().downcast_ref::<ironsmith::events::permanents::TapEvent>() {
        let ironsmith::events::permanents::TapEvent { permanent } = value.clone(); return Ok(SyncEventBody::TapEvent { permanent });
    }
    if let Some(value) = body.as_any().downcast_ref::<ironsmith::events::permanents::UntapEvent>() {
        let ironsmith::events::permanents::UntapEvent { permanent } = value.clone(); return Ok(SyncEventBody::UntapEvent { permanent });
    }
    if let Some(value) = body.as_any().downcast_ref::<ironsmith::events::phase::BeginningOfCleanupStepEvent>() {
        let ironsmith::events::phase::BeginningOfCleanupStepEvent { player } = value.clone(); return Ok(SyncEventBody::BeginningOfCleanupStepEvent { player });
    }
    if let Some(value) = body.as_any().downcast_ref::<ironsmith::events::phase::BeginningOfDrawStepEvent>() {
        let ironsmith::events::phase::BeginningOfDrawStepEvent { player } = value.clone(); return Ok(SyncEventBody::BeginningOfDrawStepEvent { player });
    }
    if let Some(value) = body.as_any().downcast_ref::<ironsmith::events::phase::BeginningOfPrecombatMainPhaseEvent>() {
        let ironsmith::events::phase::BeginningOfPrecombatMainPhaseEvent { player } = value.clone(); return Ok(SyncEventBody::BeginningOfPrecombatMainPhaseEvent { player });
    }
    if let Some(value) = body.as_any().downcast_ref::<ironsmith::events::phase::BeginningOfPostcombatMainPhaseEvent>() {
        let ironsmith::events::phase::BeginningOfPostcombatMainPhaseEvent { player, main_phase_ordinal } = value.clone(); return Ok(SyncEventBody::BeginningOfPostcombatMainPhaseEvent { player, main_phase_ordinal });
    }
    if body.as_any().is::<ironsmith::events::phase::EndOfCombatEvent>() { return Ok(SyncEventBody::EndOfCombatEvent); }
    if let Some(value) = body.as_any().downcast_ref::<ironsmith::events::phase::PermanentsUntapStepEvent>() {
        let ironsmith::events::phase::PermanentsUntapStepEvent { player } = value.clone(); return Ok(SyncEventBody::PermanentsUntapStepEvent { player });
    }
    if let Some(value) = body.as_any().downcast_ref::<ironsmith::events::spells::BecomesTargetedEvent>() {
        let ironsmith::events::spells::BecomesTargetedEvent { target, source, source_controller, by_ability, stack_ability } = value.clone(); return Ok(SyncEventBody::BecomesTargetedEvent { target, source, source_controller, by_ability, stack_ability });
    }
    if let Some(value) = body.as_any().downcast_ref::<ironsmith::events::spells::SpellCopiedEvent>() {
        let ironsmith::events::spells::SpellCopiedEvent { spell, copier } = value.clone(); return Ok(SyncEventBody::SpellCopiedEvent { spell, copier });
    }
    if let Some(value) = body.as_any().downcast_ref::<ironsmith::events::combat::CreatureBlockedEvent>() { let ironsmith::events::combat::CreatureBlockedEvent { blocker, attacker, blocker_snapshot, attacker_snapshot } = value.clone(); let blocker_snapshot = blocker_snapshot.map(|h| context.0.encode_snapshot(h, &mut *context.1)).transpose()?; let attacker_snapshot = attacker_snapshot.map(|h| context.0.encode_snapshot(h, &mut *context.1)).transpose()?; return Ok(SyncEventBody::CreatureBlockedEvent { blocker, attacker, blocker_snapshot, attacker_snapshot }); }
    if let Some(value) = body.as_any().downcast_ref::<ironsmith::events::other::CardDiscardedEvent>() { let ironsmith::events::other::CardDiscardedEvent { player, card, cause, snapshot, batch_cards, batch_snapshots, batch_index } = value.clone(); let snapshot = snapshot.map(|h| context.0.encode_snapshot(h, &mut *context.1)).transpose()?; let batch_snapshots = batch_snapshots.into_iter().map(|h| context.0.encode_snapshot(h, &mut *context.1)).collect::<Result<Vec<_>, _>>()?; return Ok(SyncEventBody::CardDiscardedEvent { player, card, cause, snapshot, batch_cards, batch_snapshots, batch_index }); }
    if let Some(value) = body.as_any().downcast_ref::<ironsmith::events::other::CardRevealedEvent>() { let ironsmith::events::other::CardRevealedEvent { player, card, zone, source, snapshot, reveal_context_amount } = value.clone(); let snapshot = snapshot.map(|h| context.0.encode_snapshot(h, &mut *context.1)).transpose()?; return Ok(SyncEventBody::CardRevealedEvent { player, card, zone, source, snapshot, reveal_context_amount }); }
    if let Some(value) = body.as_any().downcast_ref::<ironsmith::events::other::PermanentPhasedOutEvent>() { let ironsmith::events::other::PermanentPhasedOutEvent { permanent, controller, snapshot } = value.clone(); let snapshot = snapshot.map(|h| context.0.encode_snapshot(h, &mut *context.1)).transpose()?; return Ok(SyncEventBody::PermanentPhasedOutEvent { permanent, controller, snapshot }); }
    if let Some(value) = body.as_any().downcast_ref::<ironsmith::events::other::SpellCounteredEvent>() { let ironsmith::events::other::SpellCounteredEvent { spell, controller, snapshot } = value.clone(); let snapshot = snapshot.map(|h| context.0.encode_snapshot(h, &mut *context.1)).transpose()?; return Ok(SyncEventBody::SpellCounteredEvent { spell, controller, snapshot }); }
    if let Some(value) = body.as_any().downcast_ref::<ironsmith::events::permanents::DestroyEvent>() { let ironsmith::events::permanents::DestroyEvent { permanent, source, snapshot, final_zone } = value.clone(); let snapshot = snapshot.map(|h| context.0.encode_snapshot(h, &mut *context.1)).transpose()?; return Ok(SyncEventBody::DestroyEvent { permanent, source, snapshot, final_zone }); }
    if let Some(value) = body.as_any().downcast_ref::<ironsmith::events::permanents::SacrificeEvent>() { let ironsmith::events::permanents::SacrificeEvent { permanent, source, snapshot, sacrificing_player } = value.clone(); let snapshot = snapshot.map(|h| context.0.encode_snapshot(h, &mut *context.1)).transpose()?; return Ok(SyncEventBody::SacrificeEvent { permanent, source, snapshot, sacrificing_player }); }
    if let Some(value) = body.as_any().downcast_ref::<ironsmith::events::spells::SpellCastEvent>() { let ironsmith::events::spells::SpellCastEvent { spell, caster, from_zone, snapshot } = value.clone(); let snapshot = snapshot.map(|h| context.0.encode_snapshot(h, &mut *context.1)).transpose()?; return Ok(SyncEventBody::SpellCastEvent { spell, caster, from_zone, snapshot }); }
    if let Some(value) = body.as_any().downcast_ref::<ironsmith::events::zones::ObjectLeavesGameEvent>() { let ironsmith::events::zones::ObjectLeavesGameEvent { object, snapshot, cause } = value.clone(); let snapshot = context.0.encode_snapshot(snapshot, &mut *context.1)?; return Ok(SyncEventBody::ObjectLeavesGameEvent { object, snapshot, cause }); }
    if let Some(value) = body.as_any().downcast_ref::<ironsmith::events::zones::ZoneChangeEvent>() { let ironsmith::events::zones::ZoneChangeEvent { objects, result_objects, from, to, cause, snapshot, snapshots, object_tags } = value.clone(); let snapshot = snapshot.map(|h| context.0.encode_snapshot(h, &mut *context.1)).transpose()?; let snapshots = snapshots.into_iter().map(|h| context.0.encode_snapshot(h, &mut *context.1)).collect::<Result<Vec<_>, _>>()?; let ordered: std::collections::BTreeMap<_, _> = object_tags.into_iter().collect(); let object_tags = ordered.into_iter().map(|(key, hs)| Ok((key, hs.into_iter().map(|h| context.0.encode_snapshot(h, &mut *context.1)).collect::<Result<Vec<_>, StackBindingError>>()?))).collect::<Result<_, StackBindingError>>()?; return Ok(SyncEventBody::ZoneChangeEvent { objects, result_objects, from, to, cause, snapshot, snapshots, object_tags }); }
    if let Some(value) = body.as_any().downcast_ref::<ironsmith::events::combat::CreatureAttackedEvent>() { let ironsmith::events::combat::CreatureAttackedEvent { attacker, target, total_attackers, declared_attackers } = value.clone(); let target = SyncAttackEventTarget::retain(target); let declared_attackers = declared_attackers.map(|rows| rows.iter().cloned().map(SyncDeclaredAttacker::retain).collect()); return Ok(SyncEventBody::CreatureAttackedEvent { attacker, target, total_attackers, declared_attackers }); }
    if let Some(value) = body.as_any().downcast_ref::<ironsmith::events::combat::CreatureAttackedAndUnblockedEvent>() { let ironsmith::events::combat::CreatureAttackedAndUnblockedEvent { attacker, target } = value.clone(); let target = SyncAttackEventTarget::retain(target); return Ok(SyncEventBody::CreatureAttackedAndUnblockedEvent { attacker, target }); }
    if let Some(value) = body.as_any().downcast_ref::<ironsmith::events::combat::CreatureBecameBlockedEvent>() { let ironsmith::events::combat::CreatureBecameBlockedEvent { attacker, blocker_count, blockers, attack_target, attacker_snapshot, blocker_snapshots } = value.clone(); let attack_target = attack_target.map(SyncAttackEventTarget::retain); let attacker_snapshot = attacker_snapshot.map(|h| context.0.encode_snapshot(h, &mut *context.1)).transpose()?; let blocker_snapshots = blocker_snapshots.into_iter().map(|h| context.0.encode_snapshot(h, &mut *context.1)).collect::<Result<Vec<_>, _>>()?; return Ok(SyncEventBody::CreatureBecameBlockedEvent { attacker, blocker_count, blockers, attack_target, attacker_snapshot, blocker_snapshots }); }
    if let Some(value) = body.as_any().downcast_ref::<ironsmith::events::mana::ManaAddedEvent>() { let ironsmith::events::mana::ManaAddedEvent { source, controller, player, mana, snapshot, provenance } = value.clone(); let snapshot = snapshot.map(|h| context.0.encode_snapshot(h, &mut *context.1)).transpose()?; return Ok(SyncEventBody::ManaAddedEvent { source, controller, player, mana, snapshot, provenance }); }
    if let Some(value) = body.as_any().downcast_ref::<ironsmith::events::mana::ManaUnitSpentEvent>() { let ironsmith::events::mana::ManaUnitSpentEvent { player, mana_source, payment_source, symbol, purpose, source_snapshot } = value.clone(); let source_snapshot = source_snapshot.map(|h| context.0.encode_snapshot(h, &mut *context.1)).transpose()?; return Ok(SyncEventBody::ManaUnitSpentEvent { player, mana_source, payment_source, symbol, purpose, source_snapshot }); }
    if let Some(value) = body.as_any().downcast_ref::<ironsmith::events::other::ObjectBecameUnattachedEvent>() { let ironsmith::events::other::ObjectBecameUnattachedEvent { object, previous_target, controller, snapshot } = value.clone(); let snapshot = snapshot.map(|h| context.0.encode_snapshot(h, &mut *context.1)).transpose()?; return Ok(SyncEventBody::ObjectBecameUnattachedEvent { object, previous_target, controller, snapshot }); }
    if let Some(value) = body.as_any().downcast_ref::<ironsmith::events::damage::DamagePreventedEvent>() { let ironsmith::events::damage::DamagePreventedEvent { damage_source, target, amount, prevention_source, prevention_controller, is_combat, target_snapshot, applications, prevention_shield } = value.clone(); let target = sync_damage_target(target); let target_snapshot = target_snapshot.map(|h| context.0.encode_snapshot(h, &mut *context.1)).transpose()?; let applications = applications.into_iter().map(|row| { let ironsmith::events::damage::PreventedDamage { damage_source, target, amount, is_combat, target_snapshot } = row; Ok(SyncPreventedDamage { damage_source, target: sync_damage_target(target), amount, is_combat, target_snapshot: target_snapshot.map(|h| context.0.encode_snapshot(h, &mut *context.1)).transpose()? }) }).collect::<Result<_, StackBindingError>>()?; return Ok(SyncEventBody::DamagePreventedEvent { damage_source, target, amount, prevention_source, prevention_controller, is_combat, target_snapshot, applications, prevention_shield }); }
    if let Some(value) = body.as_any().downcast_ref::<ironsmith::events::other::KeywordActionEvent>() { let ironsmith::events::other::KeywordActionEvent { action, player, source, amount, votes, snapshot, player_tags, object_tags, combat_phase, unlocked_door_triggers, unlocked_door_ability_range, x_value, voter_teams } = value.clone(); let votes = votes.map(|rows| rows.into_iter().map(SyncPlayerVote::retain).collect()); let snapshot = snapshot.map(|h| context.0.encode_snapshot(h, &mut *context.1)).transpose()?; let player_tags = player_tags.into_iter().collect(); let ordered: std::collections::BTreeMap<_, _> = object_tags.into_iter().collect(); let object_tags = ordered.into_iter().map(|(key, hs)| Ok((key, hs.into_iter().map(|h| context.0.encode_snapshot(h, &mut *context.1)).collect::<Result<Vec<_>, StackBindingError>>()?))).collect::<Result<_, StackBindingError>>()?; let unlocked_door_triggers = unlocked_door_triggers.map(|rows| rows.into_iter().map(|id| id.0).collect()); return Ok(SyncEventBody::KeywordActionEvent { action, player, source, amount, votes, snapshot, player_tags, object_tags, combat_phase, unlocked_door_triggers, unlocked_door_ability_range, x_value, voter_teams }); }
    if let Some(value) = body.as_any().downcast_ref::<ironsmith::events::other::MarkersChangedEvent>() { let ironsmith::events::other::MarkersChangedEvent { change_type, marker, location, amount, count_after, source, source_controller } = value.clone(); let change_type = match change_type { ironsmith::events::other::MarkerChangeType::Added => SyncMarkerChange::Added, ironsmith::events::other::MarkerChangeType::Removed => SyncMarkerChange::Removed }; let ironsmith::marker::Marker::Counter(marker) = marker; let location = match location { ironsmith::marker::MarkerLocation::Player(player) => SyncTarget::Player { player: player.0 }, ironsmith::marker::MarkerLocation::Object(object) => SyncTarget::Object { object: object.0 } }; return Ok(SyncEventBody::MarkersChangedEvent { change_type, marker, location, amount, count_after, source, source_controller }); }
    if let Some(value) = body.as_any().downcast_ref::<ironsmith::events::other::PlayersFinishedVotingEvent>() { let ironsmith::events::other::PlayersFinishedVotingEvent { source, controller, votes, vote_counts, option_names, player_tags, voter_teams } = value.clone(); let votes = votes.into_iter().map(SyncPlayerVote::retain).collect(); let ordered: std::collections::BTreeMap<_, _> = vote_counts.into_iter().collect(); let vote_counts = ordered.into_iter().collect(); let player_tags = player_tags.into_iter().collect(); return Ok(SyncEventBody::PlayersFinishedVotingEvent { source, controller, votes, vote_counts, option_names, player_tags, voter_teams }); }
    if let Some(value) = body.as_any().downcast_ref::<ironsmith::events::spells::AbilityTriggeredEvent>() { let ironsmith::events::spells::AbilityTriggeredEvent { source, source_stable_id, controller, trigger_identity, source_snapshot, cause_kind, cause_object, zone_change_cause } = value.clone(); let trigger_identity = trigger_identity.0; let source_snapshot = source_snapshot.map(|h| context.0.encode_snapshot(h, &mut *context.1)).transpose()?; let zone_change_cause = zone_change_cause.map(SyncTriggerZoneCause::retain); return Ok(SyncEventBody::AbilityTriggeredEvent { source, source_stable_id, controller, trigger_identity, source_snapshot, cause_kind, cause_object, zone_change_cause }); }
    if let Some(value) = body.as_any().downcast_ref::<ironsmith::events::spells::AbilityActivatedEvent>() { let ironsmith::events::spells::AbilityActivatedEvent { source, activator, is_mana_ability, is_loyalty_ability, activation_cost_has_x, activation_cost_has_tap, x_value, stack_entry_provenance, snapshot, activated_ability, mana_sources_spent, mana_spent_total } = value.clone(); let snapshot = snapshot.map(|h| context.0.encode_snapshot(h, &mut *context.1)).transpose()?; let activated_ability = activated_ability.map(|a| context.0.encode_ability_with_card_graph(a, &mut *context.1)).transpose()?; let mana_sources_spent = mana_sources_spent.into_iter().map(|h| context.0.encode_snapshot(h, &mut *context.1)).collect::<Result<Vec<_>, _>>()?; return Ok(SyncEventBody::AbilityActivatedEvent { source, activator, is_mana_ability, is_loyalty_ability, activation_cost_has_x, activation_cost_has_tap, x_value, stack_entry_provenance, snapshot, activated_ability, mana_sources_spent, mana_spent_total }); }
    if let Some(value) = body.as_any().downcast_ref::<ironsmith::events::tokens::CreateTokensEvent>() { let ironsmith::events::tokens::CreateTokensEvent { controller, count, cause, token, additional_tokens } = value.clone(); let token = token.map(|o| context.0.encode_live_object(o, &mut *context.1)).transpose()?; return Ok(SyncEventBody::CreateTokensEvent { controller, count, cause, token, additional_tokens }); }
    if let Some(value) = body.as_any().downcast_ref::<ironsmith::events::zones::EnterBattlefieldEvent>() { return Ok(SyncEventBody::EnterBattlefieldEvent { value: context.0.encode_entry_event(value.clone(), &mut *context.1)? }); }
    Err(stack_codec_error("stack event body has no approved checkpoint codec"))
}
fn retain_sync_event(context: &mut SyncStackEncoder<'_, '_, '_>, event: ironsmith::triggers::TriggerEvent) -> Result<SyncRetainedEvent, StackBindingError> {
    event.try_retain(context, |context, body| {
        let key = std::sync::Arc::as_ptr(&body) as *const () as usize;
        if let Some(index) = context.2.keys.get(&key) { return Ok(*index); }
        let index = u32::try_from(context.2.bodies.len()).map_err(|_| stack_codec_error("too many stack event occurrences"))?;
        let retained = retain_sync_event_body(context, body.as_ref())?;
        context.2.keys.insert(key, index); context.2.bodies.push(retained); Ok(index)
    }, |context, history| context.0.encode_snapshot(history, &mut *context.1))
}
fn retain_sync_stack(encoder: &mut ironsmith_runtime_catalog::artifact_materializer::StaticAbilityOccurrenceEncoder,
    entry: StackEntry, card: &mut dyn FnMut(CardId) -> Result<u32, StackBindingError>, arena: &mut SyncEventArena) -> Result<SyncRetainedStack, StackBindingError> {
    entry.try_retain(&mut (encoder, card, arena),
        |context, program| context.0.encode_program_with_card_graph(program, &mut *context.1),
        |context, history| context.0.encode_snapshot(history, &mut *context.1),
        retain_sync_event,
        |context, outcome| outcome.try_retain(context, retain_sync_event),
        |context, restriction| restriction.try_map_effects(&mut |effect| context.0.encode_effect_with_card_graph(effect, &mut *context.1)))
}

type SyncStackProjector<'a, 'b> = (&'a mut dyn FnMut(ironsmith::snapshot::ObjectSnapshot) -> Result<ironsmith::snapshot::ObjectSnapshot, String>,
    &'b mut std::collections::HashMap<usize, std::sync::Arc<dyn ironsmith::events::GameEventType>>);
fn project_sync_stack_event(context: &mut SyncStackProjector<'_, '_>, event: ironsmith::triggers::TriggerEvent) -> Result<ironsmith::triggers::TriggerEvent, String> {
    let retained = event.try_retain(context, |context, body| {
        let key = std::sync::Arc::as_ptr(&body) as *const () as usize;
        if let Some(projected) = context.1.get(&key) { return Ok(projected.clone()); }
        let projected: std::sync::Arc<dyn ironsmith::events::GameEventType> = if let Some(damage) = body.as_any().downcast_ref::<ironsmith::events::damage::DamageEvent>() {
            let mut damage = damage.clone();
            damage.target_snapshot = damage.target_snapshot.map(|history| context.0(history)).transpose()?;
            std::sync::Arc::new(damage)
        } else if let Some(value) = body.as_any().downcast_ref::<ironsmith::events::spells::AbilityActivatedEvent>() { let mut value = value.clone(); value.snapshot = value.snapshot.map(|h| context.0(h)).transpose()?; value.mana_sources_spent = value.mana_sources_spent.into_iter().map(|h| context.0(h)).collect::<Result<Vec<_>, String>>()?; std::sync::Arc::new(value) } else if let Some(value) = body.as_any().downcast_ref::<ironsmith::events::tokens::CreateTokensEvent>() { let mut value = value.clone(); if let Some(o) = &mut value.token { let capture = ironsmith_runtime_catalog::artifact_materializer::StaticAbilityOccurrenceEncoder::project_cast_history(ironsmith::object::NativeCastPaymentState::from(&*o), &mut |h| context.0(h)).map_err(|e|e.to_string())?; capture.apply_to(o)?; } std::sync::Arc::new(value) } else if let Some(value) = body.as_any().downcast_ref::<ironsmith::events::zones::EnterBattlefieldEvent>() { let retained = ironsmith::replacement_entry_capture::RetainedEntryEvent::capture(value.clone()).try_map_payloads(Ok::<_, String>, Ok, Ok, |h| h.try_project_tree(&mut *context.0).map_err(|e|e.to_string()), Ok, |e|e)?; std::sync::Arc::new(retained.into_native()?) } else if let Some(value) = body.as_any().downcast_ref::<ironsmith::events::damage::DamagePreventedEvent>() { let mut value = value.clone(); value.target_snapshot = value.target_snapshot.map(|h| context.0(h)).transpose()?; for row in &mut value.applications { row.target_snapshot = row.target_snapshot.take().map(|h| context.0(h)).transpose()?; } std::sync::Arc::new(value) } else if let Some(value) = body.as_any().downcast_ref::<ironsmith::events::other::KeywordActionEvent>() { let mut value = value.clone(); value.snapshot = value.snapshot.map(|h| context.0(h)).transpose()?; let ordered: std::collections::BTreeMap<_, _> = value.object_tags.into_iter().collect(); value.object_tags = ordered.into_iter().map(|(key, hs)| Ok((key, hs.into_iter().map(|h| context.0(h)).collect::<Result<Vec<_>, String>>()?))).collect::<Result<_, String>>()?; std::sync::Arc::new(value) } else if let Some(value) = body.as_any().downcast_ref::<ironsmith::events::other::MarkersChangedEvent>() { let mut value = value.clone();  std::sync::Arc::new(value) } else if let Some(value) = body.as_any().downcast_ref::<ironsmith::events::other::PlayersFinishedVotingEvent>() { let mut value = value.clone();  std::sync::Arc::new(value) } else if let Some(value) = body.as_any().downcast_ref::<ironsmith::events::spells::AbilityTriggeredEvent>() { let mut value = value.clone(); value.source_snapshot = value.source_snapshot.map(|h| context.0(h)).transpose()?; std::sync::Arc::new(value) } else if let Some(value) = body.as_any().downcast_ref::<ironsmith::events::combat::CreatureAttackedEvent>() { let mut value = value.clone();  std::sync::Arc::new(value) } else if let Some(value) = body.as_any().downcast_ref::<ironsmith::events::combat::CreatureAttackedAndUnblockedEvent>() { let mut value = value.clone();  std::sync::Arc::new(value) } else if let Some(value) = body.as_any().downcast_ref::<ironsmith::events::combat::CreatureBecameBlockedEvent>() { let mut value = value.clone(); value.attacker_snapshot = value.attacker_snapshot.map(|h| context.0(h)).transpose()?; value.blocker_snapshots = value.blocker_snapshots.into_iter().map(|h| context.0(h)).collect::<Result<Vec<_>, String>>()?; std::sync::Arc::new(value) } else if let Some(value) = body.as_any().downcast_ref::<ironsmith::events::mana::ManaAddedEvent>() { let mut value = value.clone(); value.snapshot = value.snapshot.map(|h| context.0(h)).transpose()?; std::sync::Arc::new(value) } else if let Some(value) = body.as_any().downcast_ref::<ironsmith::events::mana::ManaUnitSpentEvent>() { let mut value = value.clone(); value.source_snapshot = value.source_snapshot.map(|h| context.0(h)).transpose()?; std::sync::Arc::new(value) } else if let Some(value) = body.as_any().downcast_ref::<ironsmith::events::other::ObjectBecameUnattachedEvent>() { let mut value = value.clone(); value.snapshot = value.snapshot.map(|h| context.0(h)).transpose()?; std::sync::Arc::new(value) } else if let Some(value) = body.as_any().downcast_ref::<ironsmith::events::combat::CreatureBlockedEvent>() { let mut value = value.clone(); value.blocker_snapshot = value.blocker_snapshot.map(|h| context.0(h)).transpose()?; value.attacker_snapshot = value.attacker_snapshot.map(|h| context.0(h)).transpose()?; std::sync::Arc::new(value) } else if let Some(value) = body.as_any().downcast_ref::<ironsmith::events::other::CardDiscardedEvent>() { let mut value = value.clone(); value.snapshot = value.snapshot.map(|h| context.0(h)).transpose()?; value.batch_snapshots = value.batch_snapshots.into_iter().map(|h| context.0(h)).collect::<Result<Vec<_>, _>>()?; std::sync::Arc::new(value) } else if let Some(value) = body.as_any().downcast_ref::<ironsmith::events::other::CardRevealedEvent>() { let mut value = value.clone(); value.snapshot = value.snapshot.map(|h| context.0(h)).transpose()?; std::sync::Arc::new(value) } else if let Some(value) = body.as_any().downcast_ref::<ironsmith::events::other::PermanentPhasedOutEvent>() { let mut value = value.clone(); value.snapshot = value.snapshot.map(|h| context.0(h)).transpose()?; std::sync::Arc::new(value) } else if let Some(value) = body.as_any().downcast_ref::<ironsmith::events::other::SpellCounteredEvent>() { let mut value = value.clone(); value.snapshot = value.snapshot.map(|h| context.0(h)).transpose()?; std::sync::Arc::new(value) } else if let Some(value) = body.as_any().downcast_ref::<ironsmith::events::permanents::DestroyEvent>() { let mut value = value.clone(); value.snapshot = value.snapshot.map(|h| context.0(h)).transpose()?; std::sync::Arc::new(value) } else if let Some(value) = body.as_any().downcast_ref::<ironsmith::events::permanents::SacrificeEvent>() { let mut value = value.clone(); value.snapshot = value.snapshot.map(|h| context.0(h)).transpose()?; std::sync::Arc::new(value) } else if let Some(value) = body.as_any().downcast_ref::<ironsmith::events::spells::SpellCastEvent>() { let mut value = value.clone(); value.snapshot = value.snapshot.map(|h| context.0(h)).transpose()?; std::sync::Arc::new(value) } else if let Some(value) = body.as_any().downcast_ref::<ironsmith::events::zones::ObjectLeavesGameEvent>() { let mut value = value.clone(); value.snapshot = context.0(value.snapshot)?; std::sync::Arc::new(value) } else if let Some(value) = body.as_any().downcast_ref::<ironsmith::events::zones::ZoneChangeEvent>() { let mut value = value.clone(); value.snapshot = value.snapshot.map(|h| context.0(h)).transpose()?; value.snapshots = value.snapshots.into_iter().map(|h| context.0(h)).collect::<Result<Vec<_>, _>>()?; let ordered: std::collections::BTreeMap<_, _> = value.object_tags.into_iter().collect(); value.object_tags = ordered.into_iter().map(|(key, hs)| Ok((key, hs.into_iter().map(|h| context.0(h)).collect::<Result<Vec<_>, String>>()?))).collect::<Result<_, String>>()?; std::sync::Arc::new(value) } else if body.as_any().is::<ironsmith::events::phase::BeginningOfEndStepEvent>()
            || body.as_any().is::<ironsmith::events::phase::BeginningOfUpkeepEvent>()
            || body.as_any().is::<ironsmith::events::phase::BeginningOfCombatEvent>()
            || body.as_any().is::<ironsmith::events::life::LifeGainEvent>()
            || body.as_any().is::<ironsmith::events::life::LifeLossEvent>()
            || body.as_any().is::<ironsmith::events::cards::DiscardEvent>()
            || body.as_any().is::<ironsmith::events::cards::DrawEvent>()
            || body.as_any().is::<ironsmith::events::counters::MoveCountersEvent>()
            || body.as_any().is::<ironsmith::events::counters::PutCountersEvent>()
            || body.as_any().is::<ironsmith::events::counters::RemoveCountersEvent>()
            || body.as_any().is::<ironsmith::events::other::BecameMonstrousEvent>()
            || body.as_any().is::<ironsmith::events::other::CardsDrawnEvent>()
            || body.as_any().is::<ironsmith::events::other::ChapterAbilityResolvedEvent>()
            || body.as_any().is::<ironsmith::events::other::CoinFlippedEvent>()
            || body.as_any().is::<ironsmith::events::other::ControlChangedEvent>()
            || body.as_any().is::<ironsmith::events::other::ConvertedEvent>()
            || body.as_any().is::<ironsmith::events::other::CounterPlacedEvent>()
            || body.as_any().is::<ironsmith::events::other::DayNightChangedEvent>()
            || body.as_any().is::<ironsmith::events::other::DieRolledEvent>()
            || body.as_any().is::<ironsmith::events::other::GiftGivenEvent>()
            || body.as_any().is::<ironsmith::events::other::LandPlayedEvent>()
            || body.as_any().is::<ironsmith::events::other::MutatedEvent>()
            || body.as_any().is::<ironsmith::events::other::PermanentTappedEvent>()
            || body.as_any().is::<ironsmith::events::other::PermanentUntappedEvent>()
            || body.as_any().is::<ironsmith::events::other::PlayerLosesGameEvent>()
            || body.as_any().is::<ironsmith::events::other::SearchLibraryEvent>()
            || body.as_any().is::<ironsmith::events::other::ShuffleLibraryEvent>()
            || body.as_any().is::<ironsmith::events::other::StateTriggerEvent>()
            || body.as_any().is::<ironsmith::events::other::TransformedEvent>()
            || body.as_any().is::<ironsmith::events::other::TurnedFaceUpEvent>()
            || body.as_any().is::<ironsmith::events::permanents::TapEvent>()
            || body.as_any().is::<ironsmith::events::permanents::UntapEvent>()
            || body.as_any().is::<ironsmith::events::phase::BeginningOfCleanupStepEvent>()
            || body.as_any().is::<ironsmith::events::phase::BeginningOfDrawStepEvent>()
            || body.as_any().is::<ironsmith::events::phase::BeginningOfPrecombatMainPhaseEvent>()
            || body.as_any().is::<ironsmith::events::phase::BeginningOfPostcombatMainPhaseEvent>()
            || body.as_any().is::<ironsmith::events::phase::EndOfCombatEvent>()
            || body.as_any().is::<ironsmith::events::phase::PermanentsUntapStepEvent>()
            || body.as_any().is::<ironsmith::events::spells::BecomesTargetedEvent>()
            || body.as_any().is::<ironsmith::events::spells::SpellCopiedEvent>() {
            body
        } else { return Err("stack event body lacks historical projection codec".into()); };
        context.1.insert(key, projected.clone()); Ok(projected)
    }, |context, history| context.0(history))?;
    retained.try_restore(&mut (), |_, body| Ok::<_, String>(body), |_, history| Ok(history))
}
fn project_sync_stack_history(stack: &[StackEntry], project: &mut dyn FnMut(ironsmith::snapshot::ObjectSnapshot) -> Result<ironsmith::snapshot::ObjectSnapshot, String>) -> Result<Vec<StackEntry>, String> {
    let mut bodies = std::collections::HashMap::new();
    let mut context = (project, &mut bodies);
    stack.iter().cloned().map(|entry| {
        let retained = entry.try_retain(&mut context,
            |_, program| Ok::<_, String>(program), |context, history| context.0(history),
            project_sync_stack_event,
            |context, outcome| outcome.try_retain(context, project_sync_stack_event),
            |_, restriction| Ok(restriction))?;
        retained.try_restore(&mut (), |_, program| Ok::<_, String>(program), |_, history| Ok(history),
            |_, event| Ok(event), |_, outcome| outcome.try_restore(&mut (), |_, event| Ok::<_, String>(event)), |_, restriction| Ok(restriction))
    }).collect()
}

fn sync_restore_vote_counts(rows: Vec<(usize, usize)>) -> Result<std::collections::HashMap<usize, usize>, StackBindingError> {
    let mut previous = None; let mut counts = std::collections::HashMap::new();
    for (key, value) in rows {
        if previous.is_some_and(|last| key <= last) { return Err(stack_codec_error("vote count keys must be unique and sorted")); }
        previous = Some(key); counts.insert(key, value);
    }
    Ok(counts)
}
fn restore_sync_event_body(decoder: &ironsmith_runtime_catalog::artifact_materializer::StaticAbilityOccurrenceDecoder,
    body: SyncEventBody, face_binding: &mut dyn FnMut(u32) -> Result<CardId, StackBindingError>) -> Result<std::sync::Arc<dyn ironsmith::events::GameEventType>, StackBindingError> {
    use ironsmith::events::{phase, life, damage};
    Ok(match body {
        SyncEventBody::AbilityActivatedEvent { source, activator, is_mana_ability, is_loyalty_ability, activation_cost_has_x, activation_cost_has_tap, x_value, stack_entry_provenance, snapshot, activated_ability, mana_sources_spent, mana_spent_total } => { let snapshot = snapshot.map(|h| decoder.restore_snapshot(h, &mut *face_binding)).transpose()?; let activated_ability = activated_ability.map(|a| decoder.restore_ability(a)).transpose()?; let mana_sources_spent = mana_sources_spent.into_iter().map(|h| decoder.restore_snapshot(h, &mut *face_binding)).collect::<Result<Vec<_>, _>>()?; std::sync::Arc::new(ironsmith::events::spells::AbilityActivatedEvent { source, activator, is_mana_ability, is_loyalty_ability, activation_cost_has_x, activation_cost_has_tap, x_value, stack_entry_provenance, snapshot, activated_ability, mana_sources_spent, mana_spent_total }) },
        SyncEventBody::CreateTokensEvent { controller, count, cause, token, additional_tokens } => { let token = token.map(|o| decoder.restore_live_object(o, &mut *face_binding)).transpose()?; std::sync::Arc::new(ironsmith::events::tokens::CreateTokensEvent { controller, count, cause, token, additional_tokens }) },
        SyncEventBody::EnterBattlefieldEvent { value } => std::sync::Arc::new(decoder.restore_entry_event(value, &mut *face_binding)?),
        SyncEventBody::DamagePreventedEvent { damage_source, target, amount, prevention_source, prevention_controller, is_combat, target_snapshot, applications, prevention_shield } => { let target = damage_target_from_sync(target); let target_snapshot = target_snapshot.map(|h| decoder.restore_snapshot(h, &mut *face_binding)).transpose()?; let applications = applications.into_iter().map(|row| { let SyncPreventedDamage { damage_source, target, amount, is_combat, target_snapshot } = row; Ok(ironsmith::events::damage::PreventedDamage { damage_source, target: damage_target_from_sync(target), amount, is_combat, target_snapshot: target_snapshot.map(|h| decoder.restore_snapshot(h, &mut *face_binding)).transpose()? }) }).collect::<Result<_, StackBindingError>>()?; std::sync::Arc::new(ironsmith::events::damage::DamagePreventedEvent { damage_source, target, amount, prevention_source, prevention_controller, is_combat, target_snapshot, applications, prevention_shield }) },
        SyncEventBody::KeywordActionEvent { action, player, source, amount, votes, snapshot, player_tags, object_tags, combat_phase, unlocked_door_triggers, unlocked_door_ability_range, x_value, voter_teams } => { let votes = votes.map(|rows| rows.into_iter().map(SyncPlayerVote::restore).collect()); let snapshot = snapshot.map(|h| decoder.restore_snapshot(h, &mut *face_binding)).transpose()?; let player_tags = player_tags.into_iter().collect(); let object_tags = object_tags.into_iter().map(|(key, hs)| Ok((key, hs.into_iter().map(|h| decoder.restore_snapshot(h, &mut *face_binding)).collect::<Result<Vec<_>, StackBindingError>>()?))).collect::<Result<_, StackBindingError>>()?; let unlocked_door_triggers = unlocked_door_triggers.map(|rows| rows.into_iter().map(ironsmith::triggers::TriggerIdentity).collect()); std::sync::Arc::new(ironsmith::events::other::KeywordActionEvent { action, player, source, amount, votes, snapshot, player_tags, object_tags, combat_phase, unlocked_door_triggers, unlocked_door_ability_range, x_value, voter_teams }) },
        SyncEventBody::MarkersChangedEvent { change_type, marker, location, amount, count_after, source, source_controller } => { let change_type = match change_type { SyncMarkerChange::Added => ironsmith::events::other::MarkerChangeType::Added, SyncMarkerChange::Removed => ironsmith::events::other::MarkerChangeType::Removed }; let marker = ironsmith::marker::Marker::Counter(marker); let location = match location { SyncTarget::Player { player } => ironsmith::marker::MarkerLocation::Player(PlayerId::from_index(player)), SyncTarget::Object { object } => ironsmith::marker::MarkerLocation::Object(ObjectId::from_raw(object)) }; std::sync::Arc::new(ironsmith::events::other::MarkersChangedEvent { change_type, marker, location, amount, count_after, source, source_controller }) },
        SyncEventBody::PlayersFinishedVotingEvent { source, controller, votes, vote_counts, option_names, player_tags, voter_teams } => { let votes = votes.into_iter().map(SyncPlayerVote::restore).collect(); let vote_counts = sync_restore_vote_counts(vote_counts)?; let player_tags = player_tags.into_iter().collect(); std::sync::Arc::new(ironsmith::events::other::PlayersFinishedVotingEvent { source, controller, votes, vote_counts, option_names, player_tags, voter_teams }) },
        SyncEventBody::AbilityTriggeredEvent { source, source_stable_id, controller, trigger_identity, source_snapshot, cause_kind, cause_object, zone_change_cause } => { let trigger_identity = ironsmith::triggers::TriggerIdentity(trigger_identity); let source_snapshot = source_snapshot.map(|h| decoder.restore_snapshot(h, &mut *face_binding)).transpose()?; let zone_change_cause = zone_change_cause.map(SyncTriggerZoneCause::restore); std::sync::Arc::new(ironsmith::events::spells::AbilityTriggeredEvent { source, source_stable_id, controller, trigger_identity, source_snapshot, cause_kind, cause_object, zone_change_cause }) },
        SyncEventBody::CreatureAttackedEvent { attacker, target, total_attackers, declared_attackers } => { let target = target.restore(); let declared_attackers = declared_attackers.map(|rows| rows.into_iter().map(SyncDeclaredAttacker::restore).collect::<Vec<_>>().into()); std::sync::Arc::new(ironsmith::events::combat::CreatureAttackedEvent { attacker, target, total_attackers, declared_attackers }) },
        SyncEventBody::CreatureAttackedAndUnblockedEvent { attacker, target } => { let target = target.restore(); std::sync::Arc::new(ironsmith::events::combat::CreatureAttackedAndUnblockedEvent { attacker, target }) },
        SyncEventBody::CreatureBecameBlockedEvent { attacker, blocker_count, blockers, attack_target, attacker_snapshot, blocker_snapshots } => { let attack_target = attack_target.map(SyncAttackEventTarget::restore); let attacker_snapshot = attacker_snapshot.map(|h| decoder.restore_snapshot(h, &mut *face_binding)).transpose()?; let blocker_snapshots = blocker_snapshots.into_iter().map(|h| decoder.restore_snapshot(h, &mut *face_binding)).collect::<Result<Vec<_>, _>>()?; std::sync::Arc::new(ironsmith::events::combat::CreatureBecameBlockedEvent { attacker, blocker_count, blockers, attack_target, attacker_snapshot, blocker_snapshots }) },
        SyncEventBody::ManaAddedEvent { source, controller, player, mana, snapshot, provenance } => { let snapshot = snapshot.map(|h| decoder.restore_snapshot(h, &mut *face_binding)).transpose()?; std::sync::Arc::new(ironsmith::events::mana::ManaAddedEvent { source, controller, player, mana, snapshot, provenance }) },
        SyncEventBody::ManaUnitSpentEvent { player, mana_source, payment_source, symbol, purpose, source_snapshot } => { let source_snapshot = source_snapshot.map(|h| decoder.restore_snapshot(h, &mut *face_binding)).transpose()?; std::sync::Arc::new(ironsmith::events::mana::ManaUnitSpentEvent { player, mana_source, payment_source, symbol, purpose, source_snapshot }) },
        SyncEventBody::ObjectBecameUnattachedEvent { object, previous_target, controller, snapshot } => { let snapshot = snapshot.map(|h| decoder.restore_snapshot(h, &mut *face_binding)).transpose()?; std::sync::Arc::new(ironsmith::events::other::ObjectBecameUnattachedEvent { object, previous_target, controller, snapshot }) },
        SyncEventBody::CreatureBlockedEvent { blocker, attacker, blocker_snapshot, attacker_snapshot } => { let blocker_snapshot = blocker_snapshot.map(|h| decoder.restore_snapshot(h, &mut *face_binding)).transpose()?; let attacker_snapshot = attacker_snapshot.map(|h| decoder.restore_snapshot(h, &mut *face_binding)).transpose()?; std::sync::Arc::new(ironsmith::events::combat::CreatureBlockedEvent { blocker, attacker, blocker_snapshot, attacker_snapshot }) },
        SyncEventBody::CardDiscardedEvent { player, card, cause, snapshot, batch_cards, batch_snapshots, batch_index } => { let snapshot = snapshot.map(|h| decoder.restore_snapshot(h, &mut *face_binding)).transpose()?; let batch_snapshots = batch_snapshots.into_iter().map(|h| decoder.restore_snapshot(h, &mut *face_binding)).collect::<Result<Vec<_>, _>>()?; std::sync::Arc::new(ironsmith::events::other::CardDiscardedEvent { player, card, cause, snapshot, batch_cards, batch_snapshots, batch_index }) },
        SyncEventBody::CardRevealedEvent { player, card, zone, source, snapshot, reveal_context_amount } => { let snapshot = snapshot.map(|h| decoder.restore_snapshot(h, &mut *face_binding)).transpose()?; std::sync::Arc::new(ironsmith::events::other::CardRevealedEvent { player, card, zone, source, snapshot, reveal_context_amount }) },
        SyncEventBody::PermanentPhasedOutEvent { permanent, controller, snapshot } => { let snapshot = snapshot.map(|h| decoder.restore_snapshot(h, &mut *face_binding)).transpose()?; std::sync::Arc::new(ironsmith::events::other::PermanentPhasedOutEvent { permanent, controller, snapshot }) },
        SyncEventBody::SpellCounteredEvent { spell, controller, snapshot } => { let snapshot = snapshot.map(|h| decoder.restore_snapshot(h, &mut *face_binding)).transpose()?; std::sync::Arc::new(ironsmith::events::other::SpellCounteredEvent { spell, controller, snapshot }) },
        SyncEventBody::DestroyEvent { permanent, source, snapshot, final_zone } => { let snapshot = snapshot.map(|h| decoder.restore_snapshot(h, &mut *face_binding)).transpose()?; std::sync::Arc::new(ironsmith::events::permanents::DestroyEvent { permanent, source, snapshot, final_zone }) },
        SyncEventBody::SacrificeEvent { permanent, source, snapshot, sacrificing_player } => { let snapshot = snapshot.map(|h| decoder.restore_snapshot(h, &mut *face_binding)).transpose()?; std::sync::Arc::new(ironsmith::events::permanents::SacrificeEvent { permanent, source, snapshot, sacrificing_player }) },
        SyncEventBody::SpellCastEvent { spell, caster, from_zone, snapshot } => { let snapshot = snapshot.map(|h| decoder.restore_snapshot(h, &mut *face_binding)).transpose()?; std::sync::Arc::new(ironsmith::events::spells::SpellCastEvent { spell, caster, from_zone, snapshot }) },
        SyncEventBody::ObjectLeavesGameEvent { object, snapshot, cause } => { let snapshot = decoder.restore_snapshot(snapshot, &mut *face_binding)?; std::sync::Arc::new(ironsmith::events::zones::ObjectLeavesGameEvent { object, snapshot, cause }) },
        SyncEventBody::ZoneChangeEvent { objects, result_objects, from, to, cause, snapshot, snapshots, object_tags } => { let snapshot = snapshot.map(|h| decoder.restore_snapshot(h, &mut *face_binding)).transpose()?; let snapshots = snapshots.into_iter().map(|h| decoder.restore_snapshot(h, &mut *face_binding)).collect::<Result<Vec<_>, _>>()?; let object_tags = object_tags.into_iter().map(|(key, hs)| Ok((key, hs.into_iter().map(|h| decoder.restore_snapshot(h, &mut *face_binding)).collect::<Result<Vec<_>, StackBindingError>>()?))).collect::<Result<_, StackBindingError>>()?; std::sync::Arc::new(ironsmith::events::zones::ZoneChangeEvent { objects, result_objects, from, to, cause, snapshot, snapshots, object_tags }) },
        SyncEventBody::DiscardEvent { card, player, destination, cause, requires_type_verification, madness_applied } => std::sync::Arc::new(ironsmith::events::cards::DiscardEvent { card, player, destination, cause, requires_type_verification, madness_applied }),
        SyncEventBody::DrawEvent { player, count, is_first_this_turn, first_of_instruction, first_of_draw_step } => std::sync::Arc::new(ironsmith::events::cards::DrawEvent { player, count, is_first_this_turn, first_of_instruction, first_of_draw_step }),
        SyncEventBody::MoveCountersEvent { from, to, counter_type, count } => std::sync::Arc::new(ironsmith::events::counters::MoveCountersEvent { from, to, counter_type, count }),
        SyncEventBody::PutCountersEvent { target, counter_type, count, maximum_count, cause } => std::sync::Arc::new(ironsmith::events::counters::PutCountersEvent { target, counter_type, count, maximum_count, cause }),
        SyncEventBody::RemoveCountersEvent { target, counter_type, count } => std::sync::Arc::new(ironsmith::events::counters::RemoveCountersEvent { target, counter_type, count }),
        SyncEventBody::BecameMonstrousEvent { creature, controller, n } => std::sync::Arc::new(ironsmith::events::other::BecameMonstrousEvent { creature, controller, n }),
        SyncEventBody::CardsDrawnEvent { player, cards, is_first_this_turn, is_during_players_draw_step, cards_previously_drawn_this_draw_step } => std::sync::Arc::new(ironsmith::events::other::CardsDrawnEvent { player, cards, is_first_this_turn, is_during_players_draw_step, cards_previously_drawn_this_draw_step }),
        SyncEventBody::ChapterAbilityResolvedEvent { saga, controller, final_chapter } => std::sync::Arc::new(ironsmith::events::other::ChapterAbilityResolvedEvent { saga, controller, final_chapter }),
        SyncEventBody::CoinFlippedEvent { player, source, face, call, winner, loser } => std::sync::Arc::new(ironsmith::events::other::CoinFlippedEvent { player, source, face, call, winner, loser }),
        SyncEventBody::ControlChangedEvent { permanent, previous_controller, new_controller } => std::sync::Arc::new(ironsmith::events::other::ControlChangedEvent { permanent, previous_controller, new_controller }),
        SyncEventBody::ConvertedEvent { permanent } => std::sync::Arc::new(ironsmith::events::other::ConvertedEvent { permanent }),
        SyncEventBody::CounterPlacedEvent { permanent, counter_type, amount, previous_count } => std::sync::Arc::new(ironsmith::events::other::CounterPlacedEvent { permanent, counter_type, amount, previous_count }),
        SyncEventBody::DayNightChangedEvent { is_daytime } => std::sync::Arc::new(ironsmith::events::other::DayNightChangedEvent { is_daytime }),
        SyncEventBody::DieRolledEvent { player, source, natural_result, result, sides, is_planar, is_attraction_visit } => std::sync::Arc::new(ironsmith::events::other::DieRolledEvent { player, source, natural_result, result, sides, is_planar, is_attraction_visit }),
        SyncEventBody::GiftGivenEvent { player, recipient, source } => std::sync::Arc::new(ironsmith::events::other::GiftGivenEvent { player, recipient, source }),
        SyncEventBody::LandPlayedEvent { land, player, from_zone } => std::sync::Arc::new(ironsmith::events::other::LandPlayedEvent { land, player, from_zone }),
        SyncEventBody::MutatedEvent { permanent, controller } => std::sync::Arc::new(ironsmith::events::other::MutatedEvent { permanent, controller }),
        SyncEventBody::PermanentTappedEvent { permanent } => std::sync::Arc::new(ironsmith::events::other::PermanentTappedEvent { permanent }),
        SyncEventBody::PermanentUntappedEvent { permanent } => std::sync::Arc::new(ironsmith::events::other::PermanentUntappedEvent { permanent }),
        SyncEventBody::PlayerLosesGameEvent { player } => std::sync::Arc::new(ironsmith::events::other::PlayerLosesGameEvent { player }),
        SyncEventBody::SearchLibraryEvent { player, library_owner } => std::sync::Arc::new(ironsmith::events::other::SearchLibraryEvent { player, library_owner }),
        SyncEventBody::ShuffleLibraryEvent { player, cause } => std::sync::Arc::new(ironsmith::events::other::ShuffleLibraryEvent { player, cause }),
        SyncEventBody::StateTriggerEvent { source } => std::sync::Arc::new(ironsmith::events::other::StateTriggerEvent { source }),
        SyncEventBody::TransformedEvent { permanent } => std::sync::Arc::new(ironsmith::events::other::TransformedEvent { permanent }),
        SyncEventBody::TurnedFaceUpEvent { permanent, player } => std::sync::Arc::new(ironsmith::events::other::TurnedFaceUpEvent { permanent, player }),
        SyncEventBody::TapEvent { permanent } => std::sync::Arc::new(ironsmith::events::permanents::TapEvent { permanent }),
        SyncEventBody::UntapEvent { permanent } => std::sync::Arc::new(ironsmith::events::permanents::UntapEvent { permanent }),
        SyncEventBody::BeginningOfCleanupStepEvent { player } => std::sync::Arc::new(ironsmith::events::phase::BeginningOfCleanupStepEvent { player }),
        SyncEventBody::BeginningOfDrawStepEvent { player } => std::sync::Arc::new(ironsmith::events::phase::BeginningOfDrawStepEvent { player }),
        SyncEventBody::BeginningOfPrecombatMainPhaseEvent { player } => std::sync::Arc::new(ironsmith::events::phase::BeginningOfPrecombatMainPhaseEvent { player }),
        SyncEventBody::BeginningOfPostcombatMainPhaseEvent { player, main_phase_ordinal } => std::sync::Arc::new(ironsmith::events::phase::BeginningOfPostcombatMainPhaseEvent { player, main_phase_ordinal }),
        SyncEventBody::EndOfCombatEvent => std::sync::Arc::new(ironsmith::events::phase::EndOfCombatEvent),
        SyncEventBody::PermanentsUntapStepEvent { player } => std::sync::Arc::new(ironsmith::events::phase::PermanentsUntapStepEvent { player }),
        SyncEventBody::BecomesTargetedEvent { target, source, source_controller, by_ability, stack_ability } => std::sync::Arc::new(ironsmith::events::spells::BecomesTargetedEvent { target, source, source_controller, by_ability, stack_ability }),
        SyncEventBody::SpellCopiedEvent { spell, copier } => std::sync::Arc::new(ironsmith::events::spells::SpellCopiedEvent { spell, copier }),
        SyncEventBody::BeginningOfEndStep { player } => std::sync::Arc::new(phase::BeginningOfEndStepEvent { player }),
        SyncEventBody::BeginningOfUpkeep { player } => std::sync::Arc::new(phase::BeginningOfUpkeepEvent { player }),
        SyncEventBody::BeginningOfCombat { player } => std::sync::Arc::new(phase::BeginningOfCombatEvent { player }),
        SyncEventBody::LifeGain { player, amount, source } => std::sync::Arc::new(life::LifeGainEvent { player, amount, source }),
        SyncEventBody::LifeLoss { player, amount, from_damage, from_radiation } => std::sync::Arc::new(life::LifeLossEvent { player, amount, from_damage, from_radiation }),
        SyncEventBody::Damage { source, target, amount, excess_damage, is_combat, is_unpreventable, cause, remainder, target_snapshot } =>
            std::sync::Arc::new(damage::DamageEvent { source, target: damage_target_from_sync(target), amount, excess_damage,
                is_combat, is_unpreventable, cause, remainder: remainder.map(|(target, amount)| (damage_target_from_sync(target), amount)),
                target_snapshot: target_snapshot.map(|history| decoder.restore_snapshot(history, &mut *face_binding)).transpose()? }),
    })
}
fn restore_sync_event(context: &mut SyncStackDecoder<'_, '_, '_>, event: SyncRetainedEvent) -> Result<ironsmith::triggers::TriggerEvent, StackBindingError> {
    event.try_restore(context, |context, index| context.2.get(index as usize).cloned().ok_or_else(|| stack_codec_error("unknown stack event occurrence")),
        |context, history| context.0.restore_snapshot(history, &mut *context.1))
}
fn restore_sync_stack(decoder: &ironsmith_runtime_catalog::artifact_materializer::StaticAbilityOccurrenceDecoder,
    entry: SyncRetainedStack, card: &mut dyn FnMut(u32) -> Result<CardId, StackBindingError>, bodies: &[std::sync::Arc<dyn ironsmith::events::GameEventType>]) -> Result<StackEntry, StackBindingError> {
    entry.try_restore(&mut (decoder, card, bodies),
        |context, program| context.0.restore_program(program),
        |context, history| context.0.restore_snapshot(history, &mut *context.1),
        restore_sync_event,
        |context, outcome| outcome.try_restore(context, restore_sync_event),
        |context, restriction| restriction.try_map_effects(&mut |effect| context.0.restore_effect(effect)))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SyncExecutableState {
    provenance_graph: ironsmith::provenance::RetainedProvenanceGraph,
    graph_card_count: u32,
    occurrences: ironsmith_runtime_catalog::artifact_materializer::RetainedStaticAbilityTable,
    definitions: Vec<(
        u32,
        ironsmith_runtime_catalog::artifact_materializer::RetainedOccurrenceCardDefinition<u32>,
    )>,
    objects:
        Vec<ironsmith_runtime_catalog::artifact_materializer::RetainedOccurrenceLiveObject<u32>>,
    continuous: ironsmith_runtime_catalog::artifact_materializer::RetainedGraphRegisteredState<u32>,
    replacement: ironsmith_runtime_catalog::artifact_materializer::RetainedRegisteredReplacementState<u32>,
    prevention: ironsmith_runtime_catalog::artifact_materializer::RetainedOccurrencePreventionState<u32>,
    grants: ironsmith_runtime_catalog::artifact_materializer::RetainedOccurrenceGrantRegistry<u32>,
    used_grant_permissions: Vec<(PlayerId, ironsmith_runtime_catalog::artifact_materializer::RetainedOccurrenceGrantPermission<u32>)>,
    delayed_triggers: Vec<ironsmith_runtime_catalog::artifact_materializer::RetainedDelayedTrigger<u32>>,
    stack: Vec<SyncRetainedStack>,
    event_bodies: Vec<SyncEventBody>,
}

struct RestoredSyncExecutableState {
    provenance_graph: ironsmith::provenance::ProvenanceGraph,
    definitions: Vec<CardDefinition>,
    objects: Vec<Object>,
    continuous: ironsmith::continuous::RegisteredContinuousEffectState,
    replacement: ironsmith::replacement::RegisteredReplacementEffectState,
    prevention: ironsmith::prevention::PreventionEffectState,
    grants: ironsmith::grant_registry::RegisteredGrantState,
    used_grant_permissions: Vec<(PlayerId, ironsmith::grant_registry::GrantPermissionIdentity)>,
    delayed_triggers: Vec<ironsmith::triggers::DelayedTrigger>,
    stack: Vec<StackEntry>,
}

/// Executable payloads require authorization even when they carry no card ID
/// or historical ObjectSnapshot. For example copy values retain native ability
/// programs after printed text/labels and the copied face identity are removed.
enum SyncContinuousExecutablePayload<'a> {
    Static(&'a ironsmith::static_abilities::StaticAbility),
    Ability(&'a ironsmith::Ability),
    Copy(&'a ironsmith::snapshot::CopiableValues),
    Text(&'a ironsmith::continuous::TextBoxOverlay),
    Restriction(&'a ironsmith::continuous::RegisteredRestriction),
    Attachment(&'a ironsmith::object::AuraAttachmentMetadata),
    Origin(&'a ironsmith::continuous::ContinuousAbilityOrigin),
}

fn approve_sync_continuous_payloads<E>(
    state: &ironsmith::continuous::RegisteredContinuousEffectState,
    mut approve: impl FnMut(&ironsmith::continuous::ContinuousEffect, SyncContinuousExecutablePayload<'_>) -> Result<(), E>,
) -> Result<(), E> {
    let approve = std::cell::RefCell::new(&mut approve);
    for effect in &state.effects {
        // These exhaustive native converters include copy/text/attachment and
        // restriction models, plus static and generating occurrence origins.
        // This pass neither encodes a program nor discovers a graph reference.
        let _ = effect.clone().try_map_payloads(
            |modification| modification.try_map_payloads(
                |value| { (*approve.borrow_mut())(effect, SyncContinuousExecutablePayload::Static(&value))?; Ok(()) },
                |value| { (*approve.borrow_mut())(effect, SyncContinuousExecutablePayload::Ability(&value))?; Ok(()) },
                |value| { (*approve.borrow_mut())(effect, SyncContinuousExecutablePayload::Copy(&value))?; Ok(()) },
                |value| { (*approve.borrow_mut())(effect, SyncContinuousExecutablePayload::Text(&value))?; Ok(()) },
                |value| { (*approve.borrow_mut())(effect, SyncContinuousExecutablePayload::Restriction(&value))?; Ok(()) },
                |value| { (*approve.borrow_mut())(effect, SyncContinuousExecutablePayload::Attachment(&value))?; Ok(()) },
            ),
            |value| { (*approve.borrow_mut())(effect, SyncContinuousExecutablePayload::Static(&value))?; Ok(()) },
            |value| { (*approve.borrow_mut())(effect, SyncContinuousExecutablePayload::Origin(&value))?; Ok(()) },
        )?;
    }
    Ok(())
}


/// Capabilities currently admitted to perspective execution. Other executable
/// families must acquire their own content/association policy, never disappear
/// from a successful checkpoint. Full owning checkpoints remain lossless.
struct SyncPerspectiveExecutionPolicy {
    visible: std::collections::BTreeSet<ObjectId>,
    opaque: std::collections::BTreeSet<ObjectId>,
    perspective: PlayerId,
}
impl SyncPerspectiveExecutionPolicy {
    fn source(&self, source: ObjectId) -> Result<(), String> {
        if self.visible.contains(&source) { Ok(()) }
        else { Err("perspective executable source lacks disclosed identity".into()) }
    }
    fn history(&self, snapshot: ironsmith::snapshot::ObjectSnapshot) -> Result<ironsmith::snapshot::ObjectSnapshot, String> {
        let hidden = snapshot.zone == Zone::Library
            || (matches!(snapshot.zone, Zone::Hand | Zone::OutsideGame) && snapshot.owner != self.perspective)
            || self.opaque.contains(&snapshot.object_id);
        if hidden || !self.visible.contains(&snapshot.object_id) {
            return Err("perspective executable history requires private facts or historical disclosure approval".into());
        }
        Ok(snapshot)
    }
    fn mana_ability(&self, ability: &ironsmith::Ability) -> Result<(), String> {
        let ironsmith::ability::AbilityKind::Activated(activated) = &ability.kind else {
            return Err("perspective ability payload requires content approval".into());
        };
        if !ability.is_mana_ability() || !activated.choices.is_empty()
            || !activated.additional_restrictions.is_empty() || !activated.activation_restrictions.is_empty()
            || activated.activation_condition.is_some() || !activated.mana_usage_restrictions.is_empty() {
            return Err("perspective ability payload requires content approval".into());
        }
        let _ = activated.mana_cost.clone().try_map(|cost| {
            match cost.compiled_model() {
                Some(ironsmith_core::cost_model::Cost::Mana(_) | ironsmith_core::cost_model::Cost::Tap) => Ok(cost),
                Some(ironsmith_core::cost_model::Cost::Effect(effect)) if effect.downcast_ref::<ironsmith::effects::TapEffect>()
                    .is_some_and(|tap| matches!(tap.target, ironsmith::target::ChooseSpec::Source)) => Ok(cost),
                _ => Err("perspective mana cost requires content approval".to_string()),
            }
        })?;
        for effect in activated.effects.all_effects() {
            if !matches!(effect.mana_production(), Some(ironsmith::mana_payment::program::ManaProduction::Fixed { player: ironsmith::target::PlayerFilter::You, .. })) {
                return Err("perspective mana program requires content approval".into());
            }
        }
        Ok(())
    }
    fn continuous_filter(&self, filter: &ironsmith::target::ObjectFilter) -> Result<(), String> {
        use ironsmith::target::PlayerFilter;
        // Admit a closed structural profile. Equality against the complete
        // native filter prevents nested captures or newly added fields from
        // bypassing approval. The original filter is retained unchanged.
        let player = |value: &Option<PlayerFilter>| -> Result<(), String> {
            match value {
                None | Some(PlayerFilter::Any | PlayerFilter::You | PlayerFilter::NotYou
                    | PlayerFilter::Opponent | PlayerFilter::Teammate | PlayerFilter::Active
                    | PlayerFilter::EffectController) => Ok(()),
                _ => Err("perspective continuous filter player requires content approval".into()),
            }
        };
        player(&filter.controller)?;
        player(&filter.owner)?;
        if matches!(filter.zone, Some(Zone::Library | Zone::Hand | Zone::OutsideGame)) {
            return Err("perspective continuous filter zone requires disclosure approval".into());
        }
        for child in &filter.any_of { self.continuous_filter(child)?; }
        if let Some(target) = filter.specific { self.source(target)?; }
        let approved = ironsmith::target::ObjectFilter {
            zone: filter.zone,
            controller: filter.controller.clone(),
            owner: filter.owner.clone(),
            card_types: filter.card_types.clone(),
            all_card_types: filter.all_card_types.clone(),
            excluded_card_types: filter.excluded_card_types.clone(),
            colors: filter.colors,
            required_colors: filter.required_colors,
            excluded_colors: filter.excluded_colors,
            colorless: filter.colorless,
            multicolored: filter.multicolored,
            monocolored: filter.monocolored,
            token: filter.token,
            nontoken: filter.nontoken,
            specific: filter.specific,
            any_of: filter.any_of.clone(),
            ..Default::default()
        };
        if filter != &approved {
            return Err("perspective continuous filter requires content approval".into());
        }
        Ok(())
    }
    fn continuous_value(&self, value: &ironsmith::effect::Value) -> Result<(), String> {
        use ironsmith::effect::Value;
        match value {
            Value::Fixed(_) => Ok(()),
            Value::Add(left, right) => { self.continuous_value(left)?; self.continuous_value(right) },
            Value::Scaled(value, _) | Value::DividedRoundedDown(value, _) | Value::HalfRoundedDown(value) =>
                self.continuous_value(value),
            Value::Count(filter) | Value::CountScaled(filter, _) | Value::GreatestCount(filter)
                | Value::GreatestSharedCreatureTypeCount(filter) | Value::GreatestSharedNameCount(filter)
                | Value::TotalPower(filter) => self.continuous_filter(filter),
            _ => Err("perspective continuous value requires content approval".into()),
        }
    }
    fn continuous_condition(&self, condition: &ironsmith::ConditionExpr) -> Result<(), String> {
        use ironsmith::ConditionExpr as Condition;
        match condition {
            Condition::YouControl(filter) | Condition::OpponentControls(filter) => self.continuous_filter(filter),
            Condition::Not(value) => self.continuous_condition(value),
            Condition::And(left, right) | Condition::Or(left, right) => {
                self.continuous_condition(left)?;
                self.continuous_condition(right)
            },
            Condition::SourceIsUntapped | Condition::SourceIsAttacking | Condition::SourceIsBlocking
                | Condition::SourceIsEquipped | Condition::SourceIsEnchanted => Ok(()),
            _ => Err("perspective continuous condition requires content approval".into()),
        }
    }
    fn continuous_duration_object(&self, object: &ironsmith_core::effect::ContinuousDurationObject) -> Result<(), String> {
        use ironsmith_core::effect::ContinuousDurationObject as Object;
        match object {
            Object::Source | Object::AffectedObject => Ok(()),
            Object::Specific(target) => self.source(*target),
            Object::Tagged(_) => Err("perspective continuous duration capture requires content approval".into()),
        }
    }
    fn continuous_duration_player(&self, player: &ironsmith_core::effect::ContinuousDurationPlayer) -> Result<(), String> {
        use ironsmith_core::effect::ContinuousDurationPlayer as Player;
        match player {
            Player::EffectController | Player::Specific(_) => Ok(()),
            Player::ControllerOf(object) => self.continuous_duration_object(object),
            Player::Tagged(_) => Err("perspective continuous duration player requires content approval".into()),
        }
    }
    fn continuous_duration_predicate(&self, predicate: &ironsmith_core::effect::ContinuousDurationPredicate) -> Result<(), String> {
        use ironsmith_core::effect::ContinuousDurationPredicate as Predicate;
        match predicate {
            Predicate::All(values) => { for value in values { self.continuous_duration_predicate(value)?; } Ok(()) },
            Predicate::ObjectOnBattlefield(object) | Predicate::ObjectTapped(object)
                | Predicate::ObjectIsEnchanted(object) => self.continuous_duration_object(object),
            Predicate::ObjectInZone { object, zone } => {
                if matches!(zone, Zone::Library | Zone::Hand | Zone::OutsideGame) {
                    return Err("perspective continuous duration zone requires disclosure approval".into());
                }
                self.continuous_duration_object(object)
            },
            Predicate::ObjectControlledBy { object, player } => {
                self.continuous_duration_object(object)?;
                self.continuous_duration_player(player)
            },
            Predicate::ObjectHasCounter { object, counter_type, .. } => {
                if matches!(counter_type, ironsmith::CounterType::Named(_)) {
                    return Err("perspective continuous duration counter requires content approval".into());
                }
                self.continuous_duration_object(object)
            },
            Predicate::ObjectAttachedTo { attachment, attached_to } => {
                self.continuous_duration_object(attachment)?;
                self.continuous_duration_object(attached_to)
            },
            Predicate::PlayerIsMonarch(player) => self.continuous_duration_player(player),
            Predicate::ObjectPowerAtMostObject { lesser, greater } => {
                self.continuous_duration_object(lesser)?;
                self.continuous_duration_object(greater)
            },
        }
    }
    fn continuous_duration(&self, duration: &ironsmith::effect::Until) -> Result<(), String> {
        use ironsmith::effect::Until;
        match duration {
            Until::Forever | Until::EndOfTurn | Until::EndOfTurnOrAnyPlayerRolls { .. }
                | Until::YourNextTurn | Until::YourNextTurnEnd | Until::YourNextUpkeep
                | Until::ControllersNextUntapStep | Until::NextEndStep | Until::EndOfCombat
                | Until::ThisLeavesTheBattlefield | Until::SourceUntaps | Until::YouStopControllingThis => Ok(()),
            Until::ForAsLongAs(predicate) => self.continuous_duration_predicate(predicate),
            Until::TurnsPass(value) => self.continuous_value(value),
        }
    }
    fn continuous_modification(&self, modification: &ironsmith::continuous::Modification) -> Result<(), String> {
        use ironsmith::continuous::Modification;
        match modification {
            // Payload contents are approved separately by the native exhaustive
            // payload walk. Here approve every non-payload captured operand.
            Modification::CopyOf { target_id, name_override, name_override_surface, .. } => {
                self.source(*target_id)?;
                if name_override.is_some() || name_override_surface.is_some() {
                    return Err("perspective continuous copy name requires content approval".into());
                }
                Ok(())
            },
            Modification::ChangeText { .. } | Modification::SetName(_) | Modification::InsertNameWords { .. } =>
                Err("perspective continuous text requires content approval".into()),
            Modification::CopyActivatedAbilities { filter, counter, .. } => {
                if matches!(counter, Some(ironsmith::CounterType::Named(_))) {
                    return Err("perspective continuous copy counter requires content approval".into());
                }
                self.continuous_filter(filter)
            },
            Modification::CopyStaticAbilityVariants { filter, .. } | Modification::CopyTriggeredAbilities { filter, .. } =>
                self.continuous_filter(filter),
            Modification::SetPower { value, .. } | Modification::SetToughness { value, .. } => self.continuous_value(value),
            Modification::SetPowerToughness { power, toughness, .. }
                | Modification::ModifyPowerToughnessValue { power, toughness } => {
                self.continuous_value(power)?;
                self.continuous_value(toughness)
            },
            Modification::ChangeController(_) | Modification::ChangeControllerToEffectController
                | Modification::SetTextBox(_) | Modification::AddCardTypes(_) | Modification::RemoveCardTypes(_)
                | Modification::SetCardTypes(_) | Modification::AddSubtypes(_) | Modification::AddAllSubtypesOfFamily(_)
                | Modification::RemoveSubtypes(_) | Modification::RemoveAllSubtypesOfFamily(_) | Modification::SetSubtypes(_)
                | Modification::SetAuraAttachmentFilter(_) | Modification::AddSupertypes(_) | Modification::RemoveSupertypes(_)
                | Modification::RemoveAllCreatureTypes | Modification::AddColors(_) | Modification::RemoveColors(_)
                | Modification::SetColors(_) | Modification::MakeColorless | Modification::AddAbility(_)
                | Modification::AddAbilityGeneric(_) | Modification::SetAbilities(_) | Modification::AddCombatDamageDrawAbility
                | Modification::RemoveAbility(_) | Modification::RemoveAbilityGeneric { .. }
                | Modification::RemoveStaticAbilityFamily(_) | Modification::RemoveAllAbilities
                | Modification::RemoveAllAbilitiesExceptMana | Modification::Restriction(_)
                | Modification::ModifyPower(_) | Modification::ModifyToughness(_)
                | Modification::ModifyPowerToughness { .. } | Modification::ModifyPowerToughnessByColorCount { .. }
                | Modification::SwitchPowerToughness => Ok(()),
        }
    }
    fn continuous_origin(&self, origin: &ironsmith::continuous::ContinuousAbilityOrigin, objects: &[Object]) -> Result<(), String> {
        use ironsmith::continuous::ContinuousOriginMetadata as Metadata;
        origin.try_visit_metadata(&mut |metadata| {
            match metadata {
                Metadata::Object(id) => self.source(id),
                Metadata::Printed { host, slot } => {
                    self.source(host)?;
                    if objects.iter().find(|object| object.id == host).is_some_and(|object| slot < object.abilities.len()) {
                        Ok(())
                    } else { Err("perspective continuous printed origin requires association approval".into()) }
                },
                Metadata::Counter { host, occurrence, slot } => {
                    self.source(host)?;
                    if objects.iter().find(|object| object.id == host).is_some_and(|object|
                        object.counters.contains_ability_origin(occurrence, slot)) {
                        Ok(())
                    } else { Err("perspective continuous counter origin requires association approval".into()) }
                },
                Metadata::Temporary { host, registration } => {
                    self.source(host)?;
                    if objects.iter().find(|object| object.id == host).is_some_and(|object|
                        object.temporary_static_ability_grants.contains_origin(registration)) {
                        Ok(())
                    } else { Err("perspective continuous temporary origin requires association approval".into()) }
                },
            }
        })?;
        let faces: std::collections::BTreeSet<_> = objects.iter()
            .filter(|object| self.visible.contains(&object.id)).filter_map(|object| object.card.map(|face| face.0)).collect();
        // The native recursive converter visits faces in levels, borrowed
        // origins and generating-effect ancestry as well as this outer face.
        // An origin reference cannot disclose an opaque object's definition.
        let _ = origin.clone().try_map_card_ids(&mut |face| {
            if faces.contains(&face.0) { Ok(face) }
            else { Err("perspective continuous origin face requires disclosure approval".to_string()) }
        })?;
        Ok(())
    }
    fn continuous_target(&self, effect: &ironsmith::continuous::ContinuousEffect) -> Result<(), String> {
        use ironsmith::continuous::{EffectSourceType, EffectTarget};
        self.source(effect.source)?;
        match &effect.applies_to {
            EffectTarget::Specific(target) | EffectTarget::AttachedTo(target) => self.source(*target)?,
            EffectTarget::Filter(filter) => self.continuous_filter(filter)?,
            EffectTarget::Source | EffectTarget::AllPermanents | EffectTarget::AllCreatures => {},
        }
        if let EffectSourceType::Resolution { locked_targets } = &effect.source_type {
            for target in locked_targets { self.source(*target)?; }
        }
        Ok(())
    }
    fn roots(&self, roots: SyncExecutableRootView<'_>) -> Result<(), String> {
        for object in roots.objects {
            if self.opaque.contains(&object.id) { validate_opaque_sync_executable_object(object)?; }
            else { self.source(object.id)?; }
        }
        for effect in &roots.continuous.effects {
            self.continuous_target(effect)?;
            if let Some(condition) = &effect.condition { self.continuous_condition(condition)?; }
            self.continuous_duration(&effect.duration)?;
            self.continuous_modification(&effect.modification)?;
        }
        approve_sync_continuous_payloads(roots.continuous, |effect, payload| {
            match payload {
                SyncContinuousExecutablePayload::Ability(ability) => {
                    match &effect.source_type {
                        ironsmith::continuous::EffectSourceType::Resolution { locked_targets } if !locked_targets.is_empty() => {
                            for target in locked_targets { self.source(*target)?; }
                        },
                        _ => {
                            let target = match effect.applies_to {
                                ironsmith::continuous::EffectTarget::Specific(target) => target,
                                ironsmith::continuous::EffectTarget::Source => effect.source,
                                _ => return Err("perspective ability target requires association approval".into()),
                            };
                            self.source(target)?;
                        },
                    }
                    self.mana_ability(ability)
                },
                SyncContinuousExecutablePayload::Origin(origin) => self.continuous_origin(origin, roots.objects),
                _ => Err("perspective continuous payload requires content approval".into()),
            }
        })?;
        for effect in &roots.replacement.effects {
            self.source(effect.source)?;
            let _ = effect.replacement.clone().try_map_payloads(
                |_: ironsmith::Effect| Err::<ironsmith::Effect, String>("perspective replacement program requires content approval".into()),
                |_: ironsmith::Ability| Err::<ironsmith::Ability, String>("perspective replacement ability requires content approval".into()),
                |_: ironsmith::resolution::ResolutionProgram| Err::<ironsmith::resolution::ResolutionProgram, String>("perspective replacement continuation requires content approval".into()),
                Ok::<_, String>,
            )?;
        }
        for shield in &roots.prevention.shields {
            self.source(shield.source)?;
            if !shield.follow_up_effects.is_empty() { return Err("perspective prevention program requires content approval".into()); }
        }
        if !roots.prevention.pending_follow_ups.is_empty() || !roots.prevention.follow_up_replacement_scopes.is_empty() {
            return Err("perspective prevention continuation requires content approval".into());
        }
        for grant in roots.grant_registry.registered_state().grants {
            self.source(grant.source.source_id())?;
            // A disclosed source does not disclose the card selected by a
            // permission, or literals captured when that permission was made.
            // Keep the whole permission or reject it; removing a constraint
            // would change which cards the recipient can legally play.
            let target = grant.target_id.ok_or_else(||
                "perspective grant requires recipient association approval".to_string())?;
            self.source(target)?;
            if grant.required_face_name.is_some() || grant.filter.is_some()
                || grant.cast_this_way_filter.is_some() {
                return Err("perspective grant constraints require content approval".into());
            }
            if !matches!(grant.grantable, ironsmith::grant::Grantable::PlayFrom)
                || !grant.cast_this_way_grants.is_empty() {
                return Err("perspective grant payload requires content approval".into());
            }
        }
        if !roots.delayed_triggers.is_empty() { return Err("perspective delayed program requires content approval".into()); }
        if !roots.stack.is_empty() { return Err("perspective stack program and captured context require content approval".into()); }
        Ok(())
    }
}

/// Native opaque object roots retain only public physical state. Comparing
/// the complete retained native carrier (rather than a list of secret fields)
/// makes admission fail if any cost, ability, overlay or permission survives.
/// The perspective owner must separately approve executable roots and histories;
/// this projection alone does not authorize pending hidden spell execution.
fn opaque_sync_executable_object(mut object: Object) -> Object {
    let last_modified = object.last_modified;
    let counters = object.counters.clone();
    let attached_to = object.attached_to;
    let attachments = object.attachments.clone();
    object.redact_to_hidden_card();
    object.last_modified = last_modified;
    object.counters = counters;
    object.attached_to = attached_to;
    object.attachments = attachments;
    object
}

fn validate_opaque_sync_executable_object(object: &Object) -> Result<(), String> {
    let expected = opaque_sync_executable_object(object.clone());
    if ironsmith::object::NativeRetainedLiveObject::from(expected)
        != ironsmith::object::NativeRetainedLiveObject::from(object.clone())
    {
        return Err(format!("opaque executable object {} contains private identity or executable state", object.id.0));
    }
    Ok(())
}

/// The perspective owner must authorize every executable root family, not
/// merely redact historical snapshot display fields. This view deliberately
/// exposes native roots before the graph codec discovers nested definitions.
struct SyncExecutableRootView<'a> {
    objects: &'a [Object],
    continuous: &'a ironsmith::continuous::RegisteredContinuousEffectState,
    replacement: &'a ironsmith::replacement::RegisteredReplacementEffectState,
    prevention: &'a ironsmith::prevention::PreventionEffectState,
    provenance: &'a ironsmith::provenance::ProvenanceGraph,
    grant_registry: &'a ironsmith::grant_registry::GrantRegistry,
    used_grant_permissions: &'a std::collections::HashSet<(PlayerId, ironsmith::grant_registry::GrantPermissionIdentity)>,
    player_count: usize,
    delayed_triggers: &'a [ironsmith::triggers::DelayedTrigger],
    stack: &'a [StackEntry],
}

impl SyncExecutableState {
    fn validate_queued_provenance(graph: &ironsmith::provenance::ProvenanceGraph, prevention: &ironsmith::prevention::PreventionEffectState) -> Result<(), String> {
        for pending in &prevention.pending_follow_ups {
            graph.validate_reference(pending.provenance)?;
        }
        Ok(())
    }
    fn validate_delayed_actors(delayed: &[ironsmith::triggers::DelayedTrigger], player_count: usize) -> Result<(), String> {
        for trigger in delayed {
            if usize::from(trigger.controller.0) >= player_count {
                return Err("delayed trigger has invalid controller".into());
            }
            if trigger.prepayment.as_ref().is_some_and(|payment| usize::from(payment.player.0) >= player_count) {
                return Err("delayed payment has invalid player".into());
            }
            if trigger.tagged_players.values().flatten().any(|player| usize::from(player.0) >= player_count) {
                return Err("delayed player capture has invalid player".into());
            }
        }
        Ok(())
    }
    fn validate_stack_roots(graph: &ironsmith::provenance::ProvenanceGraph, stack: &[StackEntry], player_count: usize) -> Result<(), String> {
        let player = |player: PlayerId| if usize::from(player.0) < player_count { Ok(()) } else { Err("stack capture has invalid player".to_string()) };
        let history = |snapshot: &ironsmith::snapshot::ObjectSnapshot| -> Result<(), String> { player(snapshot.owner)?; player(snapshot.controller) };
        let event = |event: &ironsmith::triggers::TriggerEvent| -> Result<(), String> {
            graph.validate_reference(event.provenance())?;
            if let Some(batch) = event.simultaneous_batch() { graph.validate_reference(batch)?; }
            if let Some(actor) = event.player() { player(actor)?; }
            for actor in event.player_tags().values().flatten() { player(*actor)?; }
            if let Some(snapshot) = event.source_snapshot() { history(snapshot)?; }
            for snapshot in event.lookback_source_snapshots() { history(snapshot)?; }
            if let Some(value) = event.downcast::<ironsmith::events::combat::CreatureBlockedEvent>() { if let Some(h) = &value.blocker_snapshot { history(h)?; } if let Some(h) = &value.attacker_snapshot { history(h)?; } }
            if let Some(value) = event.downcast::<ironsmith::events::other::CardDiscardedEvent>() { player(value.player)?; if let Some(actor) = value.cause.as_ref().and_then(|c| c.source_controller) { player(actor)?; } if let Some(h) = &value.snapshot { history(h)?; } for h in &value.batch_snapshots { history(h)?; } }
            if let Some(value) = event.downcast::<ironsmith::events::other::CardRevealedEvent>() { player(value.player)?; if let Some(h) = &value.snapshot { history(h)?; } }
            if let Some(value) = event.downcast::<ironsmith::events::other::PermanentPhasedOutEvent>() { player(value.controller)?; if let Some(h) = &value.snapshot { history(h)?; } }
            if let Some(value) = event.downcast::<ironsmith::events::other::SpellCounteredEvent>() { player(value.controller)?; if let Some(h) = &value.snapshot { history(h)?; } }
            if let Some(value) = event.downcast::<ironsmith::events::permanents::DestroyEvent>() { if let Some(h) = &value.snapshot { history(h)?; } }
            if let Some(value) = event.downcast::<ironsmith::events::permanents::SacrificeEvent>() { if let Some(h) = &value.snapshot { history(h)?; } if let Some(actor) = value.sacrificing_player { player(actor)?; } }
            if let Some(value) = event.downcast::<ironsmith::events::spells::SpellCastEvent>() { player(value.caster)?; if let Some(h) = &value.snapshot { history(h)?; } }
            if let Some(value) = event.downcast::<ironsmith::events::zones::ObjectLeavesGameEvent>() { history(&value.snapshot)?; if let Some(actor) = value.cause.source_controller { player(actor)?; } }
            if let Some(value) = event.downcast::<ironsmith::events::zones::ZoneChangeEvent>() { if let Some(actor) = value.cause.source_controller { player(actor)?; } if let Some(h) = &value.snapshot { history(h)?; } for h in &value.snapshots { history(h)?; } for h in value.object_tags.values().flatten() { history(h)?; } }
            if let Some(value) = event.downcast::<ironsmith::events::combat::CreatureAttackedEvent>() { if let ironsmith::triggers::event::AttackEventTarget::Player(actor) = value.target { player(actor)?; } if let Some(rows) = &value.declared_attackers { for row in rows.iter() { match row.target { ironsmith::combat_state::AttackTarget::Player(actor) | ironsmith::combat_state::AttackTarget::Nothing { defending_player: Some(actor), .. } => player(actor)?, _ => {} } } } }
            if let Some(value) = event.downcast::<ironsmith::events::combat::CreatureAttackedAndUnblockedEvent>() { if let ironsmith::triggers::event::AttackEventTarget::Player(actor) = value.target { player(actor)?; } }
            if let Some(value) = event.downcast::<ironsmith::events::combat::CreatureBecameBlockedEvent>() { if let Some(ironsmith::triggers::event::AttackEventTarget::Player(actor)) = value.attack_target { player(actor)?; } if let Some(h) = &value.attacker_snapshot { history(h)?; } for h in &value.blocker_snapshots { history(h)?; } }
            if let Some(value) = event.downcast::<ironsmith::events::mana::ManaAddedEvent>() { player(value.controller)?; player(value.player)?; if let Some(h) = &value.snapshot { history(h)?; } }
            if let Some(value) = event.downcast::<ironsmith::events::mana::ManaUnitSpentEvent>() { player(value.player)?; if let Some(h) = &value.source_snapshot { history(h)?; } }
            if let Some(value) = event.downcast::<ironsmith::events::other::ObjectBecameUnattachedEvent>() { if let ironsmith::object::AttachmentTarget::Player(actor) = value.previous_target { player(actor)?; } player(value.controller)?; if let Some(h) = &value.snapshot { history(h)?; } }
            if let Some(value) = event.downcast::<ironsmith::events::damage::DamagePreventedEvent>() { if let ironsmith::events::DamageTarget::Player(actor) = value.target { player(actor)?; } player(value.prevention_controller)?; if let Some(h) = &value.target_snapshot { history(h)?; } for row in &value.applications { if let ironsmith::events::DamageTarget::Player(actor) = row.target { player(actor)?; } if let Some(h) = &row.target_snapshot { history(h)?; } } }
            if let Some(value) = event.downcast::<ironsmith::events::other::KeywordActionEvent>() { player(value.player)?; for vote in value.votes.iter().flatten() { player(vote.player)?; } if let Some(h) = &value.snapshot { history(h)?; } for actor in value.player_tags.values().flatten() { player(*actor)?; } for h in value.object_tags.values().flatten() { history(h)?; } for (actor, _) in &value.voter_teams { player(*actor)?; } }
            if let Some(value) = event.downcast::<ironsmith::events::other::MarkersChangedEvent>() { if let ironsmith::marker::MarkerLocation::Player(actor) = value.location { player(actor)?; } if let Some(actor) = value.source_controller { player(actor)?; } }
            if let Some(value) = event.downcast::<ironsmith::events::other::PlayersFinishedVotingEvent>() { player(value.controller)?; for vote in &value.votes { player(vote.player)?; } for actor in value.player_tags.values().flatten() { player(*actor)?; } for (actor, _) in &value.voter_teams { player(*actor)?; } }
            if let Some(value) = event.downcast::<ironsmith::events::spells::AbilityTriggeredEvent>() { player(value.controller)?; if let Some(h) = &value.source_snapshot { history(h)?; } }
            if let Some(value) = event.downcast::<ironsmith::events::spells::AbilityActivatedEvent>() { player(value.activator)?; if let Some(id) = value.stack_entry_provenance { graph.validate_reference(id)?; } if let Some(h) = &value.snapshot { history(h)?; } for h in &value.mana_sources_spent { history(h)?; } }
            if let Some(value) = event.downcast::<ironsmith::events::tokens::CreateTokensEvent>() { player(value.controller)?; if let Some(actor) = value.cause.source_controller { player(actor)?; } if let Some(o) = &value.token { player(o.owner)?; player(o.initial_controller)?; } }
            if let Some(value) = event.downcast::<ironsmith::events::zones::EnterBattlefieldEvent>() { let capture = ironsmith::replacement_entry_capture::RetainedEntryEvent::capture(value.clone()); capture.validate()?; if let Some(actor) = capture.controller_override { player(actor)?; } if let Some((_, actor)) = &capture.pending_program { player(*actor)?; } for choices in std::iter::once(&capture.program_choices).chain(capture.prepared_choices.iter()) { if let Some(actor) = choices.chosen_player { player(actor)?; } if let Some(actor) = choices.battle_protector { player(actor)?; } for (_, snapshots) in &choices.as_enters_tagged_objects { for snapshot in snapshots { history(snapshot)?; } } } }
            if let Some(damage) = event.downcast::<ironsmith::events::damage::DamageEvent>() {
                if let Some(actor) = damage.cause.source_controller { player(actor)?; }
                if let ironsmith::events::DamageTarget::Player(actor) = damage.target { player(actor)?; }
                if let Some((ironsmith::events::DamageTarget::Player(actor), _)) = damage.remainder { player(actor)?; }
                if let Some(snapshot) = &damage.target_snapshot { history(snapshot)?; }
            }
            Ok(())
        };
        for entry in stack {
            player(entry.controller)?;
            if let Some(actor) = entry.defending_player { player(actor)?; }
            if let Some(actor) = entry.chosen_player { player(actor)?; }
            graph.validate_reference(entry.provenance)?;
            for target in &entry.targets { if let ironsmith::game_state::Target::Player(actor) = target { player(*actor)?; } }
            if let Some(snapshot) = &entry.source_snapshot { history(snapshot)?; }
            for snapshot in entry.tagged_objects.values().flatten() { history(snapshot)?; }
            if let Some(trigger) = &entry.triggering_event { event(trigger)?; }
            let mut outcomes: Vec<_> = entry.effect_outcomes.values().collect();
            while let Some(outcome) = outcomes.pop() {
                for trigger in &outcome.events { event(trigger)?; }
                for fact in &outcome.execution_facts {
                    if let ironsmith::effect::ExecutionFact::PlayerCounts(rows) = fact { for (actor, _) in rows { player(*actor)?; } }
                }
                if let Some(authored) = &outcome.instruction_result { outcomes.push(authored); }
            }
            for assignment in &entry.target_assignments {
                if assignment.range.start > assignment.range.end || assignment.range.end > entry.targets.len() {
                    return Err("stack capture has invalid target assignment".into());
                }
            }
            for distribution in &entry.target_distributions {
                if distribution.range.start > distribution.range.end || distribution.range.end > entry.targets.len() {
                    return Err("stack capture has invalid target distribution".into());
                }
                for (target, _) in &distribution.allocations { if let ironsmith::game_state::Target::Player(actor) = target { player(*actor)?; } }
            }
        }
        Ok(())
    }
    fn validate_native_roots(roots: &SyncExecutableRootView<'_>) -> Result<(), String> {
        Self::validate_queued_provenance(roots.provenance, roots.prevention)?;
        Self::validate_delayed_actors(roots.delayed_triggers, roots.player_count)?;
        Self::validate_stack_roots(roots.provenance, roots.stack, roots.player_count)?;
        let mut identities = std::collections::BTreeSet::new();
        for object in roots.objects {
            if !identities.insert(object.id) { return Err("duplicate executable object root".into()); }
        }
        let mut continuous = ironsmith::continuous::ContinuousEffectManager::new();
        continuous.restore_registered_state(roots.continuous.clone())?;
        let mut replacement = ironsmith::replacement::ReplacementEffectManager::new();
        replacement.restore_registered_state(roots.replacement.clone())?;
        let mut prevention = ironsmith::prevention::PreventionEffectManager::new();
        prevention.restore_retained_state(roots.prevention.clone())?;
        let mut grants = ironsmith::grant_registry::GrantRegistry::new();
        let state = roots.grant_registry.registered_state();
        grants.restore_registered_state(state.clone())?;
        for (player, permission) in roots.used_grant_permissions {
            if usize::from(player.0) >= roots.player_count {
                return Err("used grant permission has invalid player".into());
            }
            if matches!(permission, ironsmith::grant_registry::GrantPermissionIdentity::Stored(id)
                if *id >= state.next_permission_identity) {
                return Err("used grant permission exceeds its allocator".into());
            }
        }
        Ok(())
    }

    /// Mandatory whole-root authorization seam for perspective publication.
    /// The owner can inspect costs, actions, templates, origins, registrations
    /// and provenance as well as live abilities. Rejecting a root never drops
    /// that root, binds a graph node, or calls the history disclosure policy.
    /// Structural validation precedes authorization; approved histories are
    /// then projected before graph discovery through the existing typed codecs.
    fn retain_with_root_approval_and_history_policy<E: std::fmt::Display>(
        game: &GameState, registry: &ironsmith::cards::CardRegistry,
        objects: Vec<Object>, continuous: ironsmith::continuous::RegisteredContinuousEffectState,
        replacement: ironsmith::replacement::RegisteredReplacementEffectState,
        prevention: ironsmith::prevention::PreventionEffectState,
        approve: impl FnOnce(SyncExecutableRootView<'_>) -> Result<(), E>,
        project: impl FnMut(ironsmith::snapshot::ObjectSnapshot) -> Result<ironsmith::snapshot::ObjectSnapshot, E>,
    ) -> Result<Self, String> {
        Self::retain_with_root_history_and_reference_policy(game, registry, objects,
            continuous, replacement, prevention, approve, project, |_| Ok(()))
    }

    /// Joint approval path: validate native roots, authorize their executable
    /// payloads, project approved historical captures, then authorize each face
    /// before graph discovery. No publication caller needs to bypass a policy
    /// layer or reconstruct a second graph with different occurrence aliases.
    fn retain_with_root_history_and_reference_policy<E: std::fmt::Display>(
        game: &GameState, registry: &ironsmith::cards::CardRegistry,
        objects: Vec<Object>, continuous: ironsmith::continuous::RegisteredContinuousEffectState,
        replacement: ironsmith::replacement::RegisteredReplacementEffectState,
        prevention: ironsmith::prevention::PreventionEffectState,
        approve: impl FnOnce(SyncExecutableRootView<'_>) -> Result<(), E>,
        project: impl FnMut(ironsmith::snapshot::ObjectSnapshot) -> Result<ironsmith::snapshot::ObjectSnapshot, E>,
        approve_face: impl FnMut(CardId) -> Result<(), String>,
    ) -> Result<Self, String> {
        let roots = SyncExecutableRootView { objects: &objects, continuous: &continuous,
            replacement: &replacement, prevention: &prevention, provenance: game.provenance_graph(), grant_registry: &game.effect_store.grant_registry, used_grant_permissions: &game.turn_store.grant_cast_uses_this_turn, player_count: game.players.len(), delayed_triggers: &game.effect_store.delayed_triggers, stack: &game.stack };
        Self::validate_native_roots(&roots)?;
        approve(roots).map_err(|error| format!("executable root authorization failed: {error}"))?;
        Self::retain_with_history_selection(game, registry, objects,
            continuous, replacement, prevention, true, project, approve_face)
    }
    /// Owner-approved executable roots receive one historical policy pass before
    /// graph discovery. Projection preserves native occurrence/application IDs;
    /// discovery and encoding then consume the same projected roots without
    /// rerunning authorization callbacks. This is not an automatic perspective
    /// policy or approval of live abilities/actions/costs/templates/private facts.
    /// Owner-approved roots with dependency selection for the registered
    /// replacement manager. Other captured histories still receive the full
    /// policy pass; this is not automatic perspective root authorization.
    fn retain_with_registered_predicate_history_policy<E:std::fmt::Display>(
        game:&GameState,registry:&ironsmith::cards::CardRegistry,
        objects:Vec<Object>,continuous:ironsmith::continuous::RegisteredContinuousEffectState,
        replacement:ironsmith::replacement::RegisteredReplacementEffectState,
        prevention:ironsmith::prevention::PreventionEffectState,
        project:impl FnMut(ironsmith::snapshot::ObjectSnapshot)->Result<ironsmith::snapshot::ObjectSnapshot,E>,
    )->Result<Self,String>{
        Self::retain_with_history_selection(game,registry,objects,continuous,replacement,prevention,true,project, |_| Ok(()))
    }
    fn retain_with_history_policy<E:std::fmt::Display>(
        game:&GameState,registry:&ironsmith::cards::CardRegistry,
        objects:Vec<Object>,continuous:ironsmith::continuous::RegisteredContinuousEffectState,
        replacement:ironsmith::replacement::RegisteredReplacementEffectState,
        prevention:ironsmith::prevention::PreventionEffectState,
        project:impl FnMut(ironsmith::snapshot::ObjectSnapshot)->Result<ironsmith::snapshot::ObjectSnapshot,E>,
    )->Result<Self,String>{
        Self::retain_with_history_selection(game,registry,objects,continuous,replacement,prevention,false,project, |_| Ok(()))
    }
    fn retain_with_history_selection<E:std::fmt::Display>(
        game:&GameState,registry:&ironsmith::cards::CardRegistry,
        objects:Vec<Object>,continuous:ironsmith::continuous::RegisteredContinuousEffectState,
        replacement:ironsmith::replacement::RegisteredReplacementEffectState,
        prevention:ironsmith::prevention::PreventionEffectState,
        project_registered_predicates:bool,
        mut project:impl FnMut(ironsmith::snapshot::ObjectSnapshot)->Result<ironsmith::snapshot::ObjectSnapshot,E>,
        approve_face: impl FnMut(CardId) -> Result<(), String>,
    )->Result<Self,String>{
        use ironsmith_runtime_catalog::artifact_materializer::StaticAbilityOccurrenceEncoder;
        Self::validate_native_roots(&SyncExecutableRootView { objects: &objects, continuous: &continuous,
            replacement: &replacement, prevention: &prevention, provenance: game.provenance_graph(), grant_registry: &game.effect_store.grant_registry, used_grant_permissions: &game.turn_store.grant_cast_uses_this_turn, player_count: game.players.len(), delayed_triggers: &game.effect_store.delayed_triggers, stack: &game.stack })?;
        let objects=objects.into_iter().map(|mut object|{
            let capture=StaticAbilityOccurrenceEncoder::project_cast_history(ironsmith::object::NativeCastPaymentState::from(&object),&mut project).map_err(|error|error.to_string())?;
            capture.apply_to(&mut object)?;Ok::<_,String>(object)
        }).collect::<Result<Vec<_>,_>>()?;
        let replacement=replacement.try_map_effects(|effect| {
            if project_registered_predicates {
                effect.try_project_predicate_history(&mut project)
            } else {
                effect.try_project_matcher_history(&mut project)
            }
        }).map_err(|error|error.to_string())?;
        let prevention=StaticAbilityOccurrenceEncoder::project_prevention_history(prevention,&mut project).map_err(|error|error.to_string())?;
        let delayed_triggers = game.effect_store.delayed_triggers.iter().cloned().map(|trigger|
            StaticAbilityOccurrenceEncoder::project_delayed_trigger_history(trigger, &mut project)
                .map_err(|error| error.to_string())).collect::<Result<Vec<_>, _>>()?;
        let stack = project_sync_stack_history(&game.stack, &mut |history| project(history).map_err(|error| error.to_string()))?;
        Self::retain_with_card_reference_policy_and_delayed_triggers(game,registry,objects,continuous,replacement,prevention,delayed_triggers,stack,approve_face)
    }
    /// The caller supplies already approved native object and manager roots.
    /// Manager payloads may contain hidden historical/cost/template captures;
    /// perspective authorization must happen before discovery of graph closure.
    /// Identity-only
    /// graph nodes (for embedded templates and departed origins) carry no
    /// catalog text. Full linked families are retained for physical/live faces.
    fn retain(
        game: &GameState,
        registry: &ironsmith::cards::CardRegistry,
        objects: Vec<Object>,
        continuous: ironsmith::continuous::RegisteredContinuousEffectState,
        replacement: ironsmith::replacement::RegisteredReplacementEffectState,
        prevention: ironsmith::prevention::PreventionEffectState,
    ) -> Result<Self, String> {
        Self::retain_with_card_reference_policy(game, registry, objects, continuous, replacement,
            prevention, |_| Ok(()))
    }

    /// Approve each distinct printed/embedded face before visiting its catalog
    /// definition or admitting it into graph closure. References in all native
    /// payload families use the same exhaustive codec callback. Approval runs
    /// once per face; encoding reuses the approved bindings without callbacks.
    fn retain_with_card_reference_policy(
        game: &GameState,
        registry: &ironsmith::cards::CardRegistry,
        objects: Vec<Object>,
        continuous: ironsmith::continuous::RegisteredContinuousEffectState,
        replacement: ironsmith::replacement::RegisteredReplacementEffectState,
        prevention: ironsmith::prevention::PreventionEffectState,
        approve: impl FnMut(CardId) -> Result<(), String>,
    ) -> Result<Self, String> {
        Self::retain_with_card_reference_policy_and_delayed_triggers(game, registry, objects, continuous, replacement, prevention,
            game.effect_store.delayed_triggers.clone(), game.stack.iter().cloned().collect(), approve)
    }
    fn retain_with_card_reference_policy_and_delayed_triggers(
        game: &GameState,
        registry: &ironsmith::cards::CardRegistry,
        objects: Vec<Object>,
        continuous: ironsmith::continuous::RegisteredContinuousEffectState,
        replacement: ironsmith::replacement::RegisteredReplacementEffectState,
        prevention: ironsmith::prevention::PreventionEffectState,
        delayed_triggers: Vec<ironsmith::triggers::DelayedTrigger>,
        stack: Vec<StackEntry>,
        mut approve: impl FnMut(CardId) -> Result<(), String>,
    ) -> Result<Self, String> {
        use ironsmith_runtime_catalog::artifact_materializer::{
            OccurrenceBindingError, StaticAbilityOccurrenceEncoder,
        };
        Self::validate_native_roots(&SyncExecutableRootView { objects: &objects, continuous: &continuous,
            replacement: &replacement, prevention: &prevention, provenance: game.provenance_graph(), grant_registry: &game.effect_store.grant_registry, used_grant_permissions: &game.turn_store.grant_cast_uses_this_turn, player_count: game.players.len(), delayed_triggers: &delayed_triggers, stack: &stack })?;
        let grants = game.effect_store.grant_registry.registered_state();
        let mut approved = std::collections::BTreeSet::new();
        let mut approve_once = |face: CardId| -> Result<(), String> {
            if approved.insert(face.0) { approve(face)?; }
            Ok(())
        };
        let mut object_ids = std::collections::BTreeSet::new();
        let mut pending = std::collections::BTreeSet::new();
        for object in &objects {
            if !object_ids.insert(object.id) {
                return Err("duplicate executable object root".into());
            }
            pending.extend(object.card.map(|id| id.0));
            pending.extend(object.other_face.map(|id| id.0));
        }
        let mut definitions = std::collections::BTreeMap::new();
        while let Some(raw_id) = pending.pop_first() {
            let id = ironsmith::CardId::from_raw(raw_id);
            if definitions.contains_key(&raw_id) {
                continue;
            }
            approve_once(id)?;
            let definition = game
                .retained_card_definition(id).cloned()
                .or_else(|| registry.get_by_id(id).cloned())
                .ok_or_else(|| format!("missing executable definition for live face {}", id.0))?;
            if definition.card.id != id {
                return Err("catalog returned wrong executable face identity".into());
            }
            pending.extend(definition.card.other_face.map(|id| id.0));
            definitions.insert(raw_id, definition);
        }
        // Discover every typed CardId through the same exhaustive codec that
        // will publish it. Probe bindings are local and never leave this method.
        let mut graph = std::collections::BTreeSet::new();
        let mut probe = StaticAbilityOccurrenceEncoder::default();
        let mut discover = |id: ironsmith::CardId| {
            approve_once(id).map_err(|detail| OccurrenceBindingError::InvalidModel { detail })?;
            graph.insert(id.0);
            Ok::<_, OccurrenceBindingError>(id.0)
        };
        for definition in definitions.values() {
            probe
                .encode_card_definition(definition.clone(), &mut discover)
                .map_err(|e| e.to_string())?;
        }
        for object in &objects {
            probe
                .encode_live_object(object.clone(), &mut discover)
                .map_err(|e| e.to_string())?;
        }
        probe
            .encode_registered_state_with_card_graph(continuous.clone(), &mut discover)
            .map_err(|e| e.to_string())?;
        probe.encode_registered_replacement_state(replacement.clone(), &mut discover)
            .map_err(|e| e.to_string())?;
        probe.encode_prevention_state(prevention.clone(), &mut discover)
            .map_err(|e| e.to_string())?;
        probe.encode_grant_registry(grants.clone(), &mut discover)
            .map_err(|e| e.to_string())?;
        for trigger in &delayed_triggers {
            probe.encode_delayed_trigger(trigger.clone(), &mut discover).map_err(|e| e.to_string())?;
        }
        let mut probe_events = SyncEventArena::default();
        for entry in &stack {
            retain_sync_stack(&mut probe, entry.clone(), &mut discover, &mut probe_events).map_err(|e| e.to_string())?;
        }
        for (_, permission) in &game.turn_store.grant_cast_uses_this_turn {
            probe.encode_permission_identity(permission.clone(), &mut discover).map_err(|e| e.to_string())?;
        }
        drop(discover);
        let graph: Vec<_> = graph.into_iter().collect();
        let graph_card_count =
            u32::try_from(graph.len()).map_err(|_| "too many executable graph nodes")?;
        let bind = |id: ironsmith::CardId| {
            graph
                .binary_search(&id.0)
                .map(|index| index as u32)
                .map_err(|_| OccurrenceBindingError::InvalidModel {
                    detail: "undiscovered executable graph node".into(),
                })
        };
        let mut encoder = StaticAbilityOccurrenceEncoder::default();
        let definitions = definitions
            .into_iter()
            .map(|(id, definition)| {
                let index = bind(ironsmith::CardId::from_raw(id)).map_err(|e| e.to_string())?;
                let definition = encoder
                    .encode_card_definition(definition, bind)
                    .map_err(|e| e.to_string())?;
                Ok::<_, String>((index, definition))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let objects = objects
            .into_iter()
            .map(|object| {
                encoder
                    .encode_live_object(object, bind)
                    .map_err(|e| e.to_string())
            })
            .collect::<Result<Vec<_>, _>>()?;
        let continuous = encoder
            .encode_registered_state_with_card_graph(continuous, bind)
            .map_err(|e| e.to_string())?;
        let replacement=encoder.encode_registered_replacement_state(replacement,bind)
            .map_err(|e|e.to_string())?;
        let prevention=encoder.encode_prevention_state(prevention,bind)
            .map_err(|e|e.to_string())?;
        let grants = encoder.encode_grant_registry(grants, bind).map_err(|e| e.to_string())?;
        let delayed_triggers = delayed_triggers.into_iter().map(|trigger|
            encoder.encode_delayed_trigger(trigger, bind).map_err(|e| e.to_string())).collect::<Result<Vec<_>, _>>()?;
        let mut events = SyncEventArena::default();
        let mut stack_bind = bind;
        let stack = stack.into_iter().map(|entry|
            retain_sync_stack(&mut encoder, entry, &mut stack_bind, &mut events).map_err(|e| e.to_string())).collect::<Result<Vec<_>, _>>()?;
        let event_bodies = events.bodies;
        // References use the shared occurrence table; rows are sorted after
        // rebinding so HashSet iteration cannot change checkpoint bytes.
        let mut used_grant_permissions = game.turn_store.grant_cast_uses_this_turn.iter().map(|(player, permission)| {
            let retained = encoder.encode_permission_identity(permission.clone(), bind).map_err(|e| e.to_string())?;
            let key = serde_json::to_string(&(*player, &retained)).map_err(|e| e.to_string())?;
            Ok::<_, String>((key, (*player, retained)))
        }).collect::<Result<Vec<_>, _>>()?;
        used_grant_permissions.sort_by(|a, b| a.0.cmp(&b.0));
        let used_grant_permissions = used_grant_permissions.into_iter().map(|(_, value)| value).collect();
        Ok(Self {
            provenance_graph: game.provenance_graph().retained_state(),
            graph_card_count,
            occurrences: encoder.into_table(),
            definitions,
            objects,
            continuous,
            replacement,
            prevention,
            grants,
            used_grant_permissions,
            delayed_triggers,
            stack,
            event_bodies,
        })
    }

    /// All roots decode before the caller publishes any object or manager.
    /// Allocate receiver graph IDs in the surrounding runtime transaction.
    fn restore(&self, graph: &[ironsmith::CardId]) -> Result<RestoredSyncExecutableState, String> {
        use ironsmith_runtime_catalog::artifact_materializer::{
            OccurrenceBindingError, StaticAbilityOccurrenceDecoder,
        };
        if graph.len() != self.graph_card_count as usize
            || graph.windows(2).any(|pair| pair[0].0 >= pair[1].0)
        {
            return Err(
                "executable graph requires exact ordered injective receiver bindings".into(),
            );
        }
        let provenance_graph = ironsmith::provenance::ProvenanceGraph::from_retained_state(self.provenance_graph.clone())?;
        let bind = |index: u32| {
            graph
                .get(index as usize)
                .copied()
                .ok_or_else(|| OccurrenceBindingError::InvalidModel {
                    detail: "unknown executable graph reference".into(),
                })
        };
        let decoder = StaticAbilityOccurrenceDecoder::restore_with_card_graph::<u32>(
            self.occurrences.clone(),
            bind,
        )
        .map_err(|e| e.to_string())?;
        let mut definition_slots = std::collections::BTreeSet::new();
        let definitions = self
            .definitions
            .iter()
            .map(|(index, definition)| {
                if !definition_slots.insert(*index) {
                    return Err("duplicate executable definition slot".into());
                }
                let expected = bind(*index).map_err(|e| e.to_string())?;
                let definition = decoder
                    .restore_card_definition(definition.clone(), bind)
                    .map_err(|e| e.to_string())?;
                if definition.card.id != expected {
                    return Err("executable definition contradicts its graph slot".into());
                }
                Ok(definition)
            })
            .collect::<Result<Vec<_>, String>>()?;
        let mut object_ids = std::collections::BTreeSet::new();
        let objects = self
            .objects
            .iter()
            .map(|object| {
                let object = decoder
                    .restore_live_object(object.clone(), bind)
                    .map_err(|e| e.to_string())?;
                if !object_ids.insert(object.id) {
                    return Err("duplicate executable object root".into());
                }
                Ok(object)
            })
            .collect::<Result<Vec<_>, String>>()?;
        let continuous = decoder
            .restore_registered_state_with_card_graph(self.continuous.clone(), bind)
            .map_err(|e| e.to_string())?;
        // Validate descriptor identity, allocators, chronology and latches now,
        // rather than leaving a partially decoded world for the owner to reject.
        let mut manager = ironsmith::continuous::ContinuousEffectManager::new();
        manager.restore_registered_state(continuous.clone())?;
        let replacement=decoder.restore_registered_replacement_state(self.replacement.clone(),bind)
            .map_err(|e|e.to_string())?;
        let prevention=decoder.restore_prevention_state(self.prevention.clone(),bind)
            .map_err(|e|e.to_string())?;
        Self::validate_queued_provenance(&provenance_graph, &prevention)?;
        let grants = decoder.restore_grant_registry(self.grants.clone(), bind).map_err(|e| e.to_string())?;
        let mut used = std::collections::HashSet::new();
        let used_grant_permissions = self.used_grant_permissions.iter().map(|(player, permission)| {
            let permission = decoder.restore_permission_identity(permission.clone(), bind).map_err(|e| e.to_string())?;
            if matches!(&permission, ironsmith::grant_registry::GrantPermissionIdentity::Stored(id)
                if *id >= grants.next_permission_identity) {
                return Err("used grant permission exceeds its allocator".into());
            }
            if !used.insert((*player, permission.clone())) {
                return Err("duplicate used grant permission root".into());
            }
            Ok((*player, permission))
        }).collect::<Result<Vec<_>, String>>()?;
        let delayed_triggers = self.delayed_triggers.iter().cloned().map(|trigger|
            decoder.restore_delayed_trigger(trigger, bind).map_err(|e| e.to_string())).collect::<Result<Vec<_>, _>>()?;
        let mut stack_bind = bind;
        let bodies = self.event_bodies.iter().cloned().map(|body|
            restore_sync_event_body(&decoder, body, &mut stack_bind).map_err(|e| e.to_string())).collect::<Result<Vec<_>, _>>()?;
        let stack = self.stack.iter().cloned().map(|entry|
            restore_sync_stack(&decoder, entry, &mut stack_bind, &bodies).map_err(|e| e.to_string())).collect::<Result<Vec<_>, _>>()?;
        Ok(RestoredSyncExecutableState {
            provenance_graph,
            definitions,
            objects,
            continuous,
            replacement,
            prevention,
            grants,
            used_grant_permissions,
            delayed_triggers,
            stack,
        })
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
enum SyncCheckpointExecutionKind {
    FullExecutable,
    PerspectiveMetadata,
    PerspectiveExecutable,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SyncCheckpoint {
    version: u32,
    execution_kind: SyncCheckpointExecutionKind,
    /// Full checkpoints retain native executable roots. Perspective checkpoints
    /// still use the metadata carrier until root authorization is integrated.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    executable_state: Option<SyncExecutableState>,
    /// Absent only in legacy checkpoints that did not preserve chronology.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    continuous_timestamps: Option<SyncContinuousTimestamps>,
    format: MatchFormatInput,
    perspective: u8,
    snapshot_serial: u64,
    auto_cleanup_discard: bool,
    #[serde(default = "default_auto_choose_single_object_decisions")]
    auto_choose_single_object_decisions: bool,
    semantic_threshold: f32,
    turn: SyncTurn,
    #[serde(default)]
    priority_runtime: SyncPriorityRuntime,
    players: Vec<SyncPlayer>,
    objects: Vec<SyncObject>,
    battlefield: Vec<u64>,
    exile: Vec<u64>,
    command: Vec<u64>,
    #[serde(default)]
    ante: Vec<u64>,
    #[serde(default)]
    planechase: Option<SyncPlanechase>,
    #[serde(default)]
    vanguard: Option<SyncVanguard>,
    #[serde(default)]
    archenemy: Option<SyncArchenemy>,
    #[serde(default)]
    conspiracy: Option<SyncConspiracy>,
    #[serde(default)]
    free_for_all: Option<SyncFreeForAll>,
    #[serde(default)]
    team_vs_team: Option<SyncTeamVsTeam>,
    #[serde(default)]
    emperor: Option<SyncEmperor>,
    #[serde(default)]
    two_headed_giant: Option<SyncTwoHeadedGiant>,
    #[serde(default)]
    alternating_teams: Option<SyncAlternatingTeams>,
    #[serde(default)]
    grand_melee: Option<SyncGrandMelee>,
    #[serde(default)]
    limited_range_of_influence: Option<SyncLimitedRangeOfInfluence>,
    #[serde(default)]
    attack_direction: Option<SyncAttackDirection>,
    #[serde(default)]
    teams: Option<Vec<Vec<u8>>>,
    #[serde(default)]
    deploy_creatures: bool,
    #[serde(default)]
    shared_team_turns: bool,
    #[serde(default)]
    shared_team_member_orders: Vec<Vec<u8>>,
    stack: Vec<SyncStackEntry>,
    #[serde(default)]
    exiled_with_source: Vec<(u64, Vec<u64>)>,
    #[serde(default)]
    return_exiled_when_source_leaves: Vec<u64>,
    /// Plain-data public rules state beyond objects and zones; absent in older
    /// checkpoints. Not part of the public audit checkpoint.
    #[serde(default)]
    rules: SyncRulesState,
    id_counters: SyncIdCounters,
}

/// Public, plain-data rules state that a checkpoint can carry losslessly.
///
/// Every field is public information (designations, combat declarations and
/// the extra-turn queue), so the same value is exported to every perspective.
/// State built from runtime programs (continuous effects, delayed triggers,
/// replacement/prevention shields, pending triggers) has no wire encoding; a
/// same-engine rollback must use a runtime savepoint instead of a checkpoint.
/// One deferred restart battlefield entry (plain card ids of the new game).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SyncRestartBattlefieldEntry {
    cards: Vec<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    controller: Option<u8>,
    #[serde(default)]
    enters_tapped: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SyncRulesState {
    /// Public regeneration state retained by main, keyed by incarnation.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    regeneration_shields: Vec<(u64, u32)>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    regenerated_this_turn: Vec<(u64, u32)>,
    /// Main-game combat. Grand Melee lanes carry their own combat instead.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    combat: Option<SyncGrandMeleeCombat>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    monarch: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    initiative: Option<u8>,
    #[serde(default)]
    has_day_night: bool,
    #[serde(default)]
    is_night: bool,
    #[serde(default)]
    extra_turns: Vec<u8>,
    /// CR 726.4: battlefield entries a restart effect still owes the new game.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pending_restart_battlefield_entries: Vec<SyncRestartBattlefieldEntry>,
    /// Extra turns scheduled after a player's next turn: (player, creation turn).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    extra_turns_after_next_turn: Vec<(u8, u32)>,
    /// Turns each player has taken this game, as `(player, count)`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    turns_taken: Vec<(u8, u32)>,
    #[serde(default)]
    current_turn_is_extra: bool,
    #[serde(default)]
    normal_turn_anchor: Option<u8>,
    #[serde(default)]
    combat_phases_started_this_turn: u32,
    /// CR 505.1b main-phase ordinal of the current turn.
    #[serde(default)]
    main_phases_started_this_turn: u32,
    /// Permanents that came under their controller's control since that
    /// player's last upkeep began (echo, CR 702.30a), sorted.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    came_under_control_since_last_upkeep: Vec<u64>,
    /// Echo's interval captured for the currently resolving upkeep.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    echo_eligible_this_upkeep: Vec<u64>,
    /// Seats whose hidden draws open an owner reveal window (Miracle); fixed
    /// at match setup from public inputs.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    hidden_draw_reveal_players: Vec<u8>,
    /// Seats that may hold a splice card in hand; fixed at match setup from
    /// public inputs.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    hidden_splice_players: Vec<u8>,
    /// Hidden-tracked cards every peer opened through an owner-answered
    /// public reveal.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    publicly_revealed_hidden_cards: Vec<u64>,
    /// Unanswered draw reveal windows as `(player, card)`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pending_hidden_draw_reveals: Vec<(u8, u64)>,
    /// Deferred "reveal the first card you draw" reveals of private cards as
    /// `(player, card, source, optional)`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pending_hidden_automatic_draw_reveals: Vec<(u8, u64, u64, bool)>,
    /// Combat damage each player was dealt by each commander (CR 903.10a),
    /// as `(player, [(commander, damage)])`, sorted for a stable encoding.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    commander_damage: Vec<(u8, Vec<(u64, u32)>)>,
    /// The perspective whose engine exported the ledger (informational: the
    /// ledger is shared, so every perspective exports the same one).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    hidden_obligation_ledger_perspective: Option<u8>,
    /// The shared claim ledger (see `hidden_hand_choices`), in recording
    /// order, identical on every peer. Public facts: every claim was made in
    /// the public decision stream about a card every peer tracks, and its
    /// filter context is in public claim form.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    hidden_identity_obligations: Vec<SyncHiddenIdentityObligation>,
    /// Public face-down cast kinds of hidden hand cards being cast.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    hidden_face_down_cast_claims: Vec<SyncFaceDownCastClaim>,
    /// Stable ids of hidden cards that are subjects of a pending public
    /// claim (symmetric).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    hidden_claim_subjects: Vec<u64>,
    /// Durable ziffle ciphertexts of claim subjects that entered a library,
    /// opened at the end-of-match disclosure (symmetric).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    hidden_library_anchors: Vec<SyncHiddenLibraryAnchor>,
    /// Hidden cards snapshotted as their owner left the game (CR 800.4a).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    departed_hidden_cards: Vec<SyncDepartedHiddenCard>,
    /// Effect permissions to cast cards face down (public).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    face_down_cast_permissions: Vec<SyncFaceDownCastPermission>,
}

/// A face-down cast claim `(object, kind)` of a hidden hand card.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SyncFaceDownCastClaim {
    object: u64,
    kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    permission_source: Option<u64>,
}

/// An obligation's filter context, lossless for the public claim form the
/// shared ledger records (see `GameState::public_claim_filter_context`).
///
/// Plain fields travel directly. The object-bearing fields (source, target
/// and tagged snapshots, tagged players, prior effect outcomes) travel as
/// one JSON text in `contextObjects` (like the filter: a stable,
/// self-describing encoding that keeps 64-bit ids exact across the JS
/// boundary), with map entries sorted by key so every peer encodes the same
/// bytes.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct SyncObligationFilterContext {
    you: Option<u8>,
    source: Option<u64>,
    caster: Option<u8>,
    prospective_cast: Option<u64>,
    active_player: Option<u8>,
    opponents: Vec<u8>,
    teammates: Vec<u8>,
    players_in_range: Option<Vec<u8>>,
    defending_player: Option<u8>,
    defending_players: Vec<u8>,
    attacking_player: Option<u8>,
    attacking_players: Vec<u8>,
    your_commanders: Vec<u64>,
    iterated_player: Option<u8>,
    x_value: Option<u32>,
    chosen_player: Option<u8>,
    target_players: Vec<u8>,
    stack_entry: Option<u64>,
    /// JSON of [`SyncClaimContextObjects`]; absent when every object-bearing
    /// field is empty.
    #[serde(skip_serializing_if = "Option::is_none")]
    context_objects: Option<String>,
}

/// The object-bearing part of a claim's filter context, in public claim form.
/// Maps are sorted vectors (the engine keeps them in `HashMap`s, whose
/// iteration order differs between wasm instances).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct SyncClaimContextObjects {
    #[serde(skip_serializing_if = "Option::is_none")]
    source_snapshot: Option<ironsmith::snapshot::ObjectSnapshot>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    target_objects: Vec<ironsmith::snapshot::ObjectSnapshot>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tagged_objects: Vec<(String, Vec<ironsmith::snapshot::ObjectSnapshot>)>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tagged_players: Vec<(String, Vec<u8>)>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    effect_outcomes: Vec<(u32, ironsmith::effect::EffectOutcome)>,
}

fn player_indices(players: &[PlayerId]) -> Vec<u8> {
    players.iter().map(|player| player.0).collect()
}

fn players_from_indices(players: &[u8]) -> Vec<PlayerId> {
    players.iter().copied().map(PlayerId::from_index).collect()
}

impl SyncObligationFilterContext {
    /// Encode `ctx`. Fails when the context is not in public claim form (a
    /// snapshot still carrying compiled abilities or a secret choice, or an
    /// outcome carrying events): those have no lossless encoding.
    fn from_context(ctx: &ironsmith::filter::FilterContext) -> Result<Self, String> {
        let snapshot_ok = |snapshot: &ironsmith::snapshot::ObjectSnapshot| {
            if snapshot.is_public_claim_form() {
                Ok(snapshot.clone())
            } else {
                Err(format!(
                    "claim context snapshot of object {} is not in public claim form",
                    snapshot.object_id.0
                ))
            }
        };
        let mut objects = SyncClaimContextObjects {
            source_snapshot: ctx.source_snapshot.as_ref().map(snapshot_ok).transpose()?,
            target_objects: ctx
                .target_objects
                .iter()
                .map(snapshot_ok)
                .collect::<Result<_, _>>()?,
            tagged_objects: ctx
                .tagged_objects
                .iter()
                .map(|(tag, snapshots)| {
                    Ok((
                        tag.as_str().to_string(),
                        snapshots.iter().map(snapshot_ok).collect::<Result<_, String>>()?,
                    ))
                })
                .collect::<Result<_, String>>()?,
            tagged_players: ctx
                .tagged_players
                .iter()
                .map(|(tag, players)| (tag.as_str().to_string(), player_indices(players)))
                .collect(),
            effect_outcomes: ctx
                .effect_outcomes
                .iter()
                .map(|(id, outcome)| {
                    if outcome.events.is_empty() {
                        Ok((id.0, outcome.clone()))
                    } else {
                        Err(format!(
                            "claim context outcome of effect {} carries events",
                            id.0
                        ))
                    }
                })
                .collect::<Result<_, String>>()?,
        };
        objects.tagged_objects.sort_by(|left, right| left.0.cmp(&right.0));
        objects.tagged_players.sort_by(|left, right| left.0.cmp(&right.0));
        objects.effect_outcomes.sort_by_key(|(id, _)| *id);
        let empty = objects.source_snapshot.is_none()
            && objects.target_objects.is_empty()
            && objects.tagged_objects.is_empty()
            && objects.tagged_players.is_empty()
            && objects.effect_outcomes.is_empty();
        let context_objects = if empty {
            None
        } else {
            Some(
                serde_json::to_string(&objects)
                    .map_err(|error| format!("claim context objects do not encode: {error}"))?,
            )
        };
        Ok(Self {
            you: ctx.you.map(|player| player.0),
            source: ctx.source.map(|id| id.0),
            caster: ctx.caster.map(|player| player.0),
            prospective_cast: ctx.prospective_cast.map(|id| id.0),
            active_player: ctx.active_player.map(|player| player.0),
            opponents: player_indices(&ctx.opponents),
            teammates: player_indices(&ctx.teammates),
            players_in_range: ctx.players_in_range.as_deref().map(player_indices),
            defending_player: ctx.defending_player.map(|player| player.0),
            defending_players: player_indices(&ctx.defending_players),
            attacking_player: ctx.attacking_player.map(|player| player.0),
            attacking_players: player_indices(&ctx.attacking_players),
            your_commanders: raw_ids(&ctx.your_commanders),
            iterated_player: ctx.iterated_player.map(|player| player.0),
            x_value: ctx.x_value,
            chosen_player: ctx.chosen_player.map(|player| player.0),
            target_players: player_indices(&ctx.target_players),
            stack_entry: ctx.stack_entry.map(|id| id.0),
            context_objects,
        })
    }

    fn to_context(&self) -> Result<ironsmith::filter::FilterContext, String> {
        let objects: SyncClaimContextObjects = match self.context_objects.as_deref() {
            Some(json) => serde_json::from_str(json)
                .map_err(|error| format!("claim context objects do not decode: {error}"))?,
            None => SyncClaimContextObjects::default(),
        };
        Ok(ironsmith::filter::FilterContext {
            you: self.you.map(PlayerId::from_index),
            source: self.source.map(ObjectId::from_raw),
            source_snapshot: objects.source_snapshot,
            caster: self.caster.map(PlayerId::from_index),
            prospective_cast: self.prospective_cast.map(ObjectId::from_raw),
            active_player: self.active_player.map(PlayerId::from_index),
            opponents: players_from_indices(&self.opponents),
            teammates: players_from_indices(&self.teammates),
            players_in_range: self.players_in_range.as_deref().map(players_from_indices),
            defending_player: self.defending_player.map(PlayerId::from_index),
            defending_players: players_from_indices(&self.defending_players),
            attacking_player: self.attacking_player.map(PlayerId::from_index),
            attacking_players: players_from_indices(&self.attacking_players),
            your_commanders: object_ids(self.your_commanders.clone()),
            iterated_player: self.iterated_player.map(PlayerId::from_index),
            x_value: self.x_value,
            chosen_player: self.chosen_player.map(PlayerId::from_index),
            target_players: players_from_indices(&self.target_players),
            target_objects: objects.target_objects,
            tagged_objects: objects
                .tagged_objects
                .into_iter()
                .map(|(tag, snapshots)| (ironsmith::tag::TagKey::from(tag), snapshots))
                .collect(),
            tagged_players: objects
                .tagged_players
                .into_iter()
                .map(|(tag, players)| {
                    (ironsmith::tag::TagKey::from(tag), players_from_indices(&players))
                })
                .collect(),
            effect_outcomes: objects
                .effect_outcomes
                .into_iter()
                .map(|(id, outcome)| (ironsmith::effect::EffectId(id), outcome))
                .collect(),
            stack_entry: self.stack_entry.map(ObjectId::from_raw),
            // Transient: only bound while a filter compares a candidate.
            filter_candidate_players: None,
            departed_battlefield_lookback: None,
        })
    }
}

/// One pending claim of the obligation ledger. The filter travels as JSON
/// text (a stable, self-describing encoding of the compiled filter).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SyncHiddenIdentityObligation {
    stable_id: u64,
    owner: u8,
    zone: String,
    filter: String,
    #[serde(default)]
    filter_context: SyncObligationFilterContext,
    description: String,
    /// "matches", "does_not_match", "cast_face_down", or "foretell".
    check: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    face_down_kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    permission_source: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    library_anchor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SyncHiddenLibraryAnchor {
    owner: u8,
    object_id: u64,
    slot: u16,
    commitment: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    origin_slot: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    origin_commitment: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    public_slot: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    public_commitment: Option<String>,
    /// Only exported to the owner's own perspective.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    known_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SyncDepartedHiddenCard {
    id: u64,
    stable_id: u64,
    owner: u8,
    zone: String,
    face_down: bool,
    /// The printed name; only exported to the owner's own perspective.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    hidden: SyncHiddenCard,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SyncFaceDownCastPermission {
    source: u64,
    player: u8,
    zone: String,
    filter: String,
    description: String,
    #[serde(default)]
    requires_source_on_battlefield: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    expires_after_turn: Option<u32>,
    #[serde(default)]
    single_use: bool,
}

fn sync_hidden_card(info: &HiddenCardInfo) -> SyncHiddenCard {
    SyncHiddenCard {
        owner: info.owner.0,
        slot: info.slot,
        commitment: info.commitment.clone(),
        origin_slot: info.origin_slot,
        origin_commitment: info.origin_commitment.clone(),
        public_slot: info.public_slot,
        public_commitment: info.public_commitment.clone(),
    }
}

/// Hide a deck-manifest slot from a perspective that does not own the card:
/// once a card has a public ziffle position, that position is all other
/// peers may know (the manifest slot would link it across shuffles).
fn redact_hidden_slot_for_other_perspective(
    slot: &mut u16,
    commitment: &mut String,
    public_slot: Option<u16>,
    public_commitment: Option<&str>,
) {
    if let (Some(public_slot), Some(public_commitment)) = (public_slot, public_commitment)
        && !public_commitment.is_empty()
    {
        *slot = public_slot;
        *commitment = public_commitment.to_string();
    }
}

fn sync_face_down_kind_fields(
    kind: ironsmith::game_state::FaceDownCastKind,
) -> (String, Option<u64>) {
    (
        kind.as_str().to_string(),
        kind.permission_source().map(|source| source.0),
    )
}

fn face_down_kind_from_sync(
    kind: &str,
    permission_source: Option<u64>,
) -> Option<ironsmith::game_state::FaceDownCastKind> {
    ironsmith::game_state::FaceDownCastKind::from_wire(
        kind,
        permission_source.map(ObjectId::from_raw),
    )
}

/// Encode one ledger entry. A claim that cannot be encoded losslessly is an
/// error, never dropped: the ledger is shared and hashed, so dropping an
/// entry would fork this peer's ledger from the others'.
fn sync_hidden_identity_obligation(
    obligation: &ironsmith::game_state::HiddenIdentityObligation,
) -> Result<SyncHiddenIdentityObligation, String> {
    use ironsmith::game_state::HiddenIdentityCheck;
    let (check, face_down_kind, permission_source) = match obligation.check {
        HiddenIdentityCheck::Matches => ("matches".to_string(), None, None),
        HiddenIdentityCheck::DoesNotMatch => ("does_not_match".to_string(), None, None),
        HiddenIdentityCheck::Foretell => ("foretell".to_string(), None, None),
        HiddenIdentityCheck::CastFaceDown(kind) => {
            let (kind, source) = sync_face_down_kind_fields(kind);
            ("cast_face_down".to_string(), Some(kind), source)
        }
    };
    let describe = |error: String| {
        format!(
            "hidden claim \"{}\" about card {} cannot be encoded: {error}",
            obligation.description, obligation.stable_id.0.0
        )
    };
    let filter = serde_json::to_string(&obligation.filter)
        .map_err(|error| describe(format!("filter: {error}")))?;
    Ok(SyncHiddenIdentityObligation {
        stable_id: obligation.stable_id.0.0,
        owner: obligation.owner.0,
        zone: sync_zone_name(obligation.zone).to_string(),
        filter,
        filter_context: SyncObligationFilterContext::from_context(&obligation.filter_ctx)
            .map_err(describe)?,
        description: obligation.description.clone(),
        check,
        face_down_kind,
        permission_source,
        library_anchor: obligation.library_anchor.clone(),
    })
}

fn hidden_identity_obligation_from_sync(
    sync: &SyncHiddenIdentityObligation,
) -> Result<ironsmith::game_state::HiddenIdentityObligation, String> {
    use ironsmith::game_state::HiddenIdentityCheck;
    let describe = |error: &str| {
        format!(
            "hidden claim \"{}\" about card {} cannot be decoded: {error}",
            sync.description, sync.stable_id
        )
    };
    let check = match sync.check.as_str() {
        "matches" => HiddenIdentityCheck::Matches,
        "does_not_match" => HiddenIdentityCheck::DoesNotMatch,
        "foretell" => HiddenIdentityCheck::Foretell,
        "cast_face_down" => HiddenIdentityCheck::CastFaceDown(
            sync.face_down_kind
                .as_deref()
                .and_then(|kind| face_down_kind_from_sync(kind, sync.permission_source))
                .ok_or_else(|| describe("unknown face-down kind"))?,
        ),
        other => return Err(describe(&format!("unknown check {other}"))),
    };
    Ok(ironsmith::game_state::HiddenIdentityObligation {
        stable_id: StableId::from_raw(sync.stable_id),
        owner: PlayerId::from_index(sync.owner),
        zone: sync_zone_from_name(&sync.zone).map_err(|_| describe("unknown zone"))?,
        filter: serde_json::from_str(&sync.filter)
            .map_err(|error| describe(&format!("filter: {error}")))?,
        filter_ctx: sync.filter_context.to_context().map_err(|error| describe(&error))?,
        description: sync.description.clone(),
        check,
        library_anchor: sync.library_anchor.clone(),
    })
}

/// The shared hidden-claim ledger in its canonical, public form: what every
/// peer holds identically at every sequence. Its digest is committed to by
/// the public audit checkpoint (`hiddenClaimLedgerDigest`).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct PublicHiddenClaimLedger<'a> {
    obligations: &'a [SyncHiddenIdentityObligation],
    face_down_cast_claims: &'a [SyncFaceDownCastClaim],
    claim_subjects: &'a [u64],
    /// `(owner, object, anchor key)`: the anchor fields every perspective
    /// holds identically (deck-manifest slots and known names are private).
    library_anchors: Vec<(u8, u64, String)>,
}

impl PublicHiddenClaimLedger<'_> {
    fn is_empty(&self) -> bool {
        self.obligations.is_empty()
            && self.face_down_cast_claims.is_empty()
            && self.claim_subjects.is_empty()
            && self.library_anchors.is_empty()
    }
}

fn public_hidden_claim_ledger(rules: &SyncRulesState) -> PublicHiddenClaimLedger<'_> {
    PublicHiddenClaimLedger {
        obligations: &rules.hidden_identity_obligations,
        face_down_cast_claims: &rules.hidden_face_down_cast_claims,
        claim_subjects: &rules.hidden_claim_subjects,
        library_anchors: rules
            .hidden_library_anchors
            .iter()
            .map(|anchor| {
                let key = ironsmith::game_state::HiddenLibraryAnchor {
                    owner: PlayerId::from_index(anchor.owner),
                    object_id: ObjectId::from_raw(anchor.object_id),
                    slot: anchor.slot,
                    commitment: anchor.commitment.clone(),
                    origin_slot: anchor.origin_slot,
                    origin_commitment: anchor.origin_commitment.clone(),
                    public_slot: anchor.public_slot,
                    public_commitment: anchor.public_commitment.clone(),
                    known_name: None,
                }
                .key();
                (anchor.owner, anchor.object_id, key)
            })
            .collect(),
    }
}

/// SHA-256 (hex) of the canonical JSON of the shared hidden-claim ledger, or
/// `None` when the ledger is empty (matches without hidden claims keep their
/// previous public checkpoint encoding).
fn hidden_claim_ledger_digest(rules: &SyncRulesState) -> Result<Option<String>, String> {
    let ledger = public_hidden_claim_ledger(rules);
    if ledger.is_empty() {
        return Ok(None);
    }
    let bytes = serde_json::to_vec(&ledger)
        .map_err(|error| format!("hidden claim ledger does not encode: {error}"))?;
    Ok(Some(
        Sha256::digest(&bytes)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect(),
    ))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SyncLimitedRangeOfInfluence {
    seats: Vec<u8>,
    ranges: Vec<u8>,
    turn_snapshot: Vec<(u8, Vec<u8>)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SyncFreeForAll {
    seats: Vec<u8>,
    attack: FreeForAllAttackInput,
    range_of_influence: Option<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SyncTeamVsTeam {
    teams: Vec<Vec<u8>>,
    seats: Vec<u8>,
    starting_team: usize,
    starting_player: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SyncEmperor {
    teams: Vec<Vec<u8>>,
    seats: Vec<u8>,
    ranges: Vec<u8>,
    starting_team: usize,
    starting_emperor: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SyncTwoHeadedGiant {
    teams: Vec<Vec<u8>>,
    seats: Vec<u8>,
    starting_team: usize,
    starting_player: u8,
    starting_life: i32,
    poison_threshold: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SyncAlternatingTeams {
    teams: Vec<Vec<u8>>,
    seats: Vec<u8>,
    starting_player: u8,
    attack: FreeForAllAttackInput,
    range_of_influence: Option<u8>,
    deploy_creatures: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SyncGrandMelee {
    seats: Vec<u8>,
    starting_player_count: usize,
    focused_marker: u32,
    markers: Vec<SyncGrandMeleeMarker>,
    deferred_extra_turns: Vec<(u8, usize)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SyncGrandMeleeMarker {
    number: u32,
    holder: u8,
    status: String,
    removal_designations: usize,
    normal_turn_pending: bool,
    #[serde(default)]
    retained_extra_turn_waiting: bool,
    turn: SyncTurn,
    #[serde(default)]
    extra_turns: Vec<u8>,
    stack: Vec<SyncStackEntry>,
    #[serde(default)]
    combat: Option<SyncGrandMeleeCombat>,
    #[serde(default)]
    range_turn_snapshot: Vec<(u8, Vec<u8>)>,
    #[serde(default)]
    runner_state: Option<String>,
    #[serde(default)]
    runner_awaiting_priority: bool,
    #[serde(default)]
    consecutive_priority_passes: usize,
    #[serde(default)]
    priority_players_in_game: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SyncGrandMeleeCombat {
    attackers: Vec<(u64, SyncGrandMeleeAttackTarget)>,
    blockers: Vec<(u64, Vec<u64>)>,
    #[serde(default)]
    blocked_attackers: Vec<u64>,
    damage_assignment_order: Vec<(u64, Vec<u64>)>,
    attacking_bands: Vec<Vec<u64>>,
    had_to_attack_this_combat: Vec<u64>,
    /// CR 506.4e: (permanent, was a planeswalker, was a battle) when it began
    /// being attacked. Older checkpoints omit it; types are then recorded
    /// again from the current state.
    #[serde(default)]
    attacked_permanent_types: Vec<(u64, bool, bool)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
enum SyncGrandMeleeAttackTarget {
    Player { player: u8 },
    Planeswalker { object: u64 },
    Battle { object: u64 },
    /// CR 506.4c: attacking nothing after its planeswalker or battle was
    /// removed from combat; keeps the declaration-time defending player.
    Nothing {
        #[serde(default)]
        defending_player: Option<u8>,
        #[serde(default)]
        was_planeswalker: bool,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum SyncAttackDirection {
    Left,
    Right,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SyncPlanechase {
    decks: Vec<(u8, Vec<u64>)>,
    communal_deck: Option<Vec<u64>>,
    deck_owners: Vec<(u64, u8)>,
    card_kinds: Vec<(u64, String)>,
    face_up: Vec<u64>,
    planar_controller: u8,
    #[serde(default)]
    planar_controllers: Vec<u8>,
    #[serde(default)]
    face_up_controllers: Vec<(u64, u8)>,
    voluntary_rolls_this_turn: Vec<(u8, u32)>,
    planeswalk_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SyncVanguard {
    cards: Vec<(u8, u64)>,
    hand_modifiers: Vec<(u8, i32)>,
    life_modifiers: Vec<(u8, i32)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SyncArchenemy {
    variant: String,
    archenemies: Vec<u8>,
    decks: Vec<(u8, Vec<u64>)>,
    face_up: Vec<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SyncConspiracy {
    cards: Vec<(u8, Vec<u64>)>,
    face_down: Vec<u64>,
    agenda_names: Vec<(u64, Vec<String>)>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SyncPriorityRuntime {
    #[serde(default)]
    runner_awaiting_priority: bool,
    #[serde(default)]
    runner_pending_decision: bool,
    #[serde(default)]
    turn_runner_state: Option<String>,
    #[serde(default)]
    consecutive_priority_passes: usize,
    #[serde(default)]
    priority_players_in_game: usize,
}

fn default_auto_choose_single_object_decisions() -> bool {
    true
}

fn sync_zone_name(zone: Zone) -> &'static str {
    match zone {
        Zone::Library => "library",
        Zone::Hand => "hand",
        Zone::Battlefield => "battlefield",
        Zone::Graveyard => "graveyard",
        Zone::Exile => "exile",
        Zone::Stack => "stack",
        Zone::Command => "command",
        Zone::Ante => "ante",
        Zone::OutsideGame => "outside_game",
    }
}

fn sync_zone_from_name(raw: &str) -> Result<Zone, String> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "library" => Ok(Zone::Library),
        "hand" => Ok(Zone::Hand),
        "battlefield" => Ok(Zone::Battlefield),
        "graveyard" => Ok(Zone::Graveyard),
        "exile" => Ok(Zone::Exile),
        "stack" => Ok(Zone::Stack),
        "command" => Ok(Zone::Command),
        "ante" => Ok(Zone::Ante),
        "sideboard" | "outside_game" | "outside game" | "outside the game" => Ok(Zone::OutsideGame),
        other => Err(format!(
            "unknown checkpoint zone: {other}"
        )),
    }
}

fn sync_phase_name(phase: Phase) -> &'static str {
    match phase {
        Phase::Beginning => "beginning",
        Phase::FirstMain => "first_main",
        Phase::Combat => "combat",
        Phase::NextMain => "next_main",
        Phase::Ending => "ending",
    }
}

fn sync_phase_from_name(raw: &str) -> Result<Phase, String> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "beginning" | "beginning_phase" => Ok(Phase::Beginning),
        "first_main" | "first main" | "precombat_main" => Ok(Phase::FirstMain),
        "combat" | "combat_phase" => Ok(Phase::Combat),
        "next_main" | "second_main" | "postcombat_main" => Ok(Phase::NextMain),
        "ending" | "ending_phase" => Ok(Phase::Ending),
        other => Err(format!(
            "unknown checkpoint phase: {other}"
        )),
    }
}

fn sync_step_name(step: Step) -> &'static str {
    match step {
        Step::Untap => "untap",
        Step::Upkeep => "upkeep",
        Step::Draw => "draw",
        Step::BeginCombat => "begin_combat",
        Step::DeclareAttackers => "declare_attackers",
        Step::DeclareBlockers => "declare_blockers",
        Step::CombatDamage => "combat_damage",
        Step::EndCombat => "end_combat",
        Step::End => "end",
        Step::Cleanup => "cleanup",
    }
}

fn sync_step_from_name(raw: &str) -> Result<Step, String> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "untap" | "untap_step" => Ok(Step::Untap),
        "upkeep" | "upkeep_step" => Ok(Step::Upkeep),
        "draw" | "draw_step" => Ok(Step::Draw),
        "begin_combat" | "beginning_of_combat" => Ok(Step::BeginCombat),
        "declare_attackers" => Ok(Step::DeclareAttackers),
        "declare_blockers" => Ok(Step::DeclareBlockers),
        "combat_damage" => Ok(Step::CombatDamage),
        "end_combat" | "end_of_combat" => Ok(Step::EndCombat),
        "end" | "end_step" => Ok(Step::End),
        "cleanup" | "cleanup_step" => Ok(Step::Cleanup),
        other => Err(format!(
            "unknown checkpoint step: {other}"
        )),
    }
}

fn sync_card_type_from_name(raw: &str) -> Option<CardType> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "land" => Some(CardType::Land),
        "creature" => Some(CardType::Creature),
        "artifact" => Some(CardType::Artifact),
        "enchantment" => Some(CardType::Enchantment),
        "planeswalker" => Some(CardType::Planeswalker),
        "instant" => Some(CardType::Instant),
        "sorcery" => Some(CardType::Sorcery),
        "battle" => Some(CardType::Battle),
        "plane" => Some(CardType::Plane),
        "phenomenon" => Some(CardType::Phenomenon),
        "vanguard" => Some(CardType::Vanguard),
        "scheme" => Some(CardType::Scheme),
        "conspiracy" => Some(CardType::Conspiracy),
        "kindred" | "tribal" => Some(CardType::Kindred),
        _ => None,
    }
}

fn sync_subtype_from_name(raw: &str) -> Option<Subtype> {
    let normalized = raw.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        return None;
    }
    [
        Subtype::all_land_types(),
        Subtype::all_creature_types(),
        Subtype::all_artifact_types(),
        Subtype::all_enchantment_types(),
        Subtype::all_spell_types(),
        Subtype::all_planeswalker_types(),
        Subtype::all_battle_types(),
    ]
    .into_iter()
    .flatten()
    .copied()
    .find(|subtype| subtype.display_name().to_ascii_lowercase() == normalized)
}

fn sync_counter_kind(counter: ironsmith::object::CounterType) -> String {
    counter.description().to_string()
}

fn sync_counter_from_wire(
    display: &str,
    identity: Option<ironsmith::CounterType>,
) -> Result<ironsmith::CounterType, String> {
    match identity {
        Some(kind) if sync_counter_kind(kind) == display => Ok(kind),
        Some(_) => Err("counter identity disagrees with its display name".into()),
        // Legacy snapshots carry only names. Preserve their existing decoding
        // contract; new exports always include exact typed identity.
        None => Ok(sync_counter_from_name(display)),
    }
}

fn sync_counter_from_name(raw: &str) -> ironsmith::object::CounterType {
    use ironsmith::object::CounterType;

    let normalized = raw.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "+1/+1" => CounterType::PlusOnePlusOne,
        "-1/-1" => CounterType::MinusOneMinusOne,
        "+1/+0" => CounterType::PlusOnePlusZero,
        "+0/+1" => CounterType::PlusZeroPlusOne,
        "+1/+2" => CounterType::PlusOnePlusTwo,
        "+2/+2" => CounterType::PlusTwoPlusTwo,
        "-0/-1" => CounterType::MinusZeroMinusOne,
        "-0/-2" => CounterType::MinusZeroMinusTwo,
        "-2/-1" => CounterType::MinusTwoMinusOne,
        "-2/-2" => CounterType::MinusTwoMinusTwo,
        "deathtouch" => CounterType::Deathtouch,
        "decayed" => CounterType::Decayed,
        "defense" => CounterType::Defense,
        "double strike" => CounterType::DoubleStrike,
        "first strike" => CounterType::FirstStrike,
        "flying" => CounterType::Flying,
        "haste" => CounterType::Haste,
        "hexproof" => CounterType::Hexproof,
        "indestructible" => CounterType::Indestructible,
        "lifelink" => CounterType::Lifelink,
        "menace" => CounterType::Menace,
        "reach" => CounterType::Reach,
        "trample" => CounterType::Trample,
        "vigilance" => CounterType::Vigilance,
        "loyalty" => CounterType::Loyalty,
        "charge" => CounterType::Charge,
        "age" => CounterType::Age,
        "aim" => CounterType::Aim,
        "arrow" => CounterType::Arrow,
        "awakening" => CounterType::Awakening,
        "blood" => CounterType::Blood,
        "brain" => CounterType::Brain,
        "bounty" => CounterType::Bounty,
        "brick" => CounterType::Brick,
        "corpse" => CounterType::Corpse,
        "credit" => CounterType::Credit,
        "crystal" => CounterType::Crystal,
        "cube" => CounterType::Cube,
        "currency" => CounterType::Currency,
        "death" => CounterType::Death,
        "depletion" => CounterType::Depletion,
        "despair" => CounterType::Despair,
        "devotion" => CounterType::Devotion,
        "divinity" => CounterType::Divinity,
        "doom" => CounterType::Doom,
        "dream" => CounterType::Dream,
        "echo" => CounterType::Echo,
        "egg" => CounterType::Egg,
        "energy" => CounterType::Energy,
        "enlightened" => CounterType::Enlightened,
        "eon" => CounterType::Eon,
        "experience" => CounterType::Experience,
        "eyeball" => CounterType::Eyeball,
        "fade" => CounterType::Fade,
        "fate" => CounterType::Fate,
        "feather" => CounterType::Feather,
        "filibuster" => CounterType::Filibuster,
        "finality" => CounterType::Finality,
        "flame" => CounterType::Flame,
        "flood" => CounterType::Flood,
        "foreshadow" => CounterType::Foreshadow,
        "fungus" => CounterType::Fungus,
        "fuse" => CounterType::Fuse,
        "gem" => CounterType::Gem,
        "glyph" => CounterType::Glyph,
        "gold" => CounterType::Gold,
        "growth" => CounterType::Growth,
        "hatchling" => CounterType::Hatchling,
        "healing" => CounterType::Healing,
        "hit" => CounterType::Hit,
        "hoofprint" => CounterType::Hoofprint,
        "hour" => CounterType::Hour,
        "hunger" => CounterType::Hunger,
        "ice" => CounterType::Ice,
        "incarnation" => CounterType::Incarnation,
        "infection" => CounterType::Infection,
        "intervention" => CounterType::Intervention,
        "isolation" => CounterType::Isolation,
        "javelin" => CounterType::Javelin,
        "ki" => CounterType::Ki,
        "keyword" => CounterType::Keyword,
        "knowledge" => CounterType::Knowledge,
        "level" => CounterType::Level,
        "lore" => CounterType::Lore,
        "luck" => CounterType::Luck,
        "magnet" => CounterType::Magnet,
        "manifestation" => CounterType::Manifestation,
        "mannequin" => CounterType::Mannequin,
        "matrix" => CounterType::Matrix,
        "mine" => CounterType::Mine,
        "mining" => CounterType::Mining,
        "mire" => CounterType::Mire,
        "music" => CounterType::Music,
        "muster" => CounterType::Muster,
        "net" => CounterType::Net,
        "night" => CounterType::Night,
        "oil" => CounterType::Oil,
        "omen" => CounterType::Omen,
        "ore" => CounterType::Ore,
        "page" => CounterType::Page,
        "pain" => CounterType::Pain,
        "paralyzation" => CounterType::Paralyzation,
        "petal" => CounterType::Petal,
        "petrification" => CounterType::Petrification,
        "phylactery" => CounterType::Phylactery,
        "pin" => CounterType::Pin,
        "plague" => CounterType::Plague,
        "plot" => CounterType::Plot,
        "polyp" => CounterType::Polyp,
        "poison" => CounterType::Poison,
        "pressure" => CounterType::Pressure,
        "prey" => CounterType::Prey,
        "pupa" => CounterType::Pupa,
        "quest" => CounterType::Quest,
        "rad" => CounterType::Rad,
        "scream" => CounterType::Scream,
        "shield" => CounterType::Shield,
        "silver" => CounterType::Silver,
        "sleep" => CounterType::Sleep,
        "slime" => CounterType::Slime,
        "slumber" => CounterType::Slumber,
        "soot" => CounterType::Soot,
        "soul" => CounterType::Soul,
        "spore" => CounterType::Spore,
        "storage" => CounterType::Storage,
        "strife" => CounterType::Strife,
        "study" => CounterType::Study,
        "stun" => CounterType::Stun,
        "void" => CounterType::Void,
        "task" => CounterType::Task,
        "theft" => CounterType::Theft,
        "tide" => CounterType::Tide,
        "time" => CounterType::Time,
        "tower" => CounterType::Tower,
        "training" => CounterType::Training,
        "trap" => CounterType::Trap,
        "treasure" => CounterType::Treasure,
        "unity" => CounterType::Unity,
        "velocity" => CounterType::Velocity,
        "verse" => CounterType::Verse,
        "vitality" => CounterType::Vitality,
        "volatile" => CounterType::Volatile,
        "voyage" => CounterType::Voyage,
        "wage" => CounterType::Wage,
        "winch" => CounterType::Winch,
        "wind" => CounterType::Wind,
        "wish" => CounterType::Wish,
        _ => CounterType::Named(normalized.into()),
    }
}

fn sync_attachment_target(target: AttachmentTarget) -> SyncAttachmentTarget {
    match target {
        AttachmentTarget::Object(object) => SyncAttachmentTarget::Object { object: object.0 },
        AttachmentTarget::Player(player) => SyncAttachmentTarget::Player { player: player.0 },
    }
}

fn attachment_target_from_sync(target: SyncAttachmentTarget) -> AttachmentTarget {
    match target {
        SyncAttachmentTarget::Object { object } => {
            AttachmentTarget::Object(ObjectId::from_raw(object))
        }
        SyncAttachmentTarget::Player { player } => {
            AttachmentTarget::Player(PlayerId::from_index(player))
        }
    }
}

fn sync_target_input(target: Target) -> SyncTarget {
    match target {
        Target::Player(player) => SyncTarget::Player { player: player.0 },
        Target::Object(object) => SyncTarget::Object { object: object.0 },
    }
}

fn target_from_sync_input(input: SyncTarget) -> Target {
    match input {
        SyncTarget::Player { player } => Target::Player(PlayerId::from_index(player)),
        SyncTarget::Object { object } => Target::Object(ObjectId::from_raw(object)),
    }
}

fn sync_attack_target(target: &AttackTarget) -> SyncGrandMeleeAttackTarget {
    match target {
        AttackTarget::Player(player) => SyncGrandMeleeAttackTarget::Player { player: player.0 },
        AttackTarget::Planeswalker(object) => {
            SyncGrandMeleeAttackTarget::Planeswalker { object: object.0 }
        }
        AttackTarget::Battle(object) => SyncGrandMeleeAttackTarget::Battle { object: object.0 },
        AttackTarget::Nothing {
            defending_player,
            was_planeswalker,
        } => SyncGrandMeleeAttackTarget::Nothing {
            defending_player: defending_player.map(|player| player.0),
            was_planeswalker: *was_planeswalker,
        },
    }
}

fn attack_target_from_sync(target: &SyncGrandMeleeAttackTarget) -> AttackTarget {
    match target {
        SyncGrandMeleeAttackTarget::Player { player } => {
            AttackTarget::Player(PlayerId::from_index(*player))
        }
        SyncGrandMeleeAttackTarget::Planeswalker { object } => {
            AttackTarget::Planeswalker(ObjectId::from_raw(*object))
        }
        SyncGrandMeleeAttackTarget::Battle { object } => {
            AttackTarget::Battle(ObjectId::from_raw(*object))
        }
        SyncGrandMeleeAttackTarget::Nothing {
            defending_player,
            was_planeswalker,
        } => AttackTarget::Nothing {
            defending_player: defending_player.map(PlayerId::from_index),
            was_planeswalker: *was_planeswalker,
        },
    }
}

fn sync_stack_entry(entry: &StackEntry) -> SyncStackEntry {
    SyncStackEntry {
        object_id: entry.object_id.0,
        ability_id: entry.ability_id.map(|id| id.0),
        ninjutsu_attack_target: entry
            .ninjutsu_attack_target
            .as_ref()
            .map(sync_attack_target),
        controller: entry.controller.0,
        targets: entry
            .targets
            .iter()
            .copied()
            .map(sync_target_input)
            .collect(),
        is_ability: entry.is_ability,
        x_value: entry.x_value,
        source_stable_id: entry.source_stable_id.map(|id| id.0.0),
        source_name: entry.source_name.clone(),
    }
}

fn stack_entry_from_sync(entry: &SyncStackEntry) -> StackEntry {
    let mut restored = StackEntry::new(
        ObjectId::from_raw(entry.object_id),
        PlayerId::from_index(entry.controller),
    );
    restored.targets = entry
        .targets
        .iter()
        .cloned()
        .map(target_from_sync_input)
        .collect();
    restored.is_ability = entry.is_ability;
    restored.ability_id = entry.ability_id.map(ObjectId::from_raw);
    restored.ninjutsu_attack_target = entry
        .ninjutsu_attack_target
        .as_ref()
        .map(attack_target_from_sync);
    restored.x_value = entry.x_value;
    restored.source_stable_id = entry.source_stable_id.map(StableId::from_raw);
    restored.source_name = entry.source_name.clone();
    restored
}

fn sync_turn_state(turn: &TurnState) -> SyncTurn {
    SyncTurn {
        active_player: turn.active_player.0,
        priority_player: turn.priority_player.map(|player| player.0),
        turn_number: turn.turn_number,
        phase: sync_phase_name(turn.phase).to_string(),
        step: turn.step.map(sync_step_name).map(str::to_string),
        // Grand Melee lanes take their seating from the profile's own seats,
        // not from the per-lane turn state.
        turn_order: Vec::new(),
    }
}

fn sync_grand_melee_combat(combat: &ironsmith::combat_state::CombatState) -> SyncGrandMeleeCombat {
    let mut blockers = combat
        .blockers
        .iter()
        .map(|(attacker, blockers)| (attacker.0, raw_ids(blockers)))
        .collect::<Vec<_>>();
    blockers.sort_by_key(|(attacker, _)| *attacker);
    let mut blocked_attackers = combat.blocked_attackers.iter().map(|id| id.0).collect::<Vec<_>>();
    blocked_attackers.sort_unstable();
    let mut damage_assignment_order = combat
        .damage_assignment_order
        .iter()
        .map(|(attacker, blockers)| (attacker.0, raw_ids(blockers)))
        .collect::<Vec<_>>();
    damage_assignment_order.sort_by_key(|(attacker, _)| *attacker);
    let mut had_to_attack_this_combat = combat
        .had_to_attack_this_combat
        .iter()
        .map(|object| object.0)
        .collect::<Vec<_>>();
    had_to_attack_this_combat.sort_unstable();
    SyncGrandMeleeCombat {
        attackers: combat
            .attackers
            .iter()
            .map(|attacker| {
                let target = match attacker.target {
                    AttackTarget::Player(player) => {
                        SyncGrandMeleeAttackTarget::Player { player: player.0 }
                    }
                    AttackTarget::Planeswalker(object) => {
                        SyncGrandMeleeAttackTarget::Planeswalker { object: object.0 }
                    }
                    AttackTarget::Battle(object) => {
                        SyncGrandMeleeAttackTarget::Battle { object: object.0 }
                    }
                    AttackTarget::Nothing {
                        defending_player,
                        was_planeswalker,
                    } => {
                        SyncGrandMeleeAttackTarget::Nothing {
                            defending_player: defending_player.map(|player| player.0),
                            was_planeswalker,
                        }
                    }
                };
                (attacker.creature.0, target)
            })
            .collect(),
        blockers,
        blocked_attackers,
        damage_assignment_order,
        attacking_bands: combat
            .attacking_bands
            .iter()
            .map(|band| raw_ids(band))
            .collect(),
        had_to_attack_this_combat,
        attacked_permanent_types: {
            let mut types = combat
                .attacked_permanent_types
                .iter()
                .map(|(object, types)| (object.0, types.planeswalker, types.battle))
                .collect::<Vec<_>>();
            types.sort_unstable();
            types
        },
    }
}

fn grand_melee_combat_from_sync(
    combat: &SyncGrandMeleeCombat,
) -> ironsmith::combat_state::CombatState {
    ironsmith::combat_state::CombatState {
        attacked_permanent_types: combat
            .attacked_permanent_types
            .iter()
            .map(|(object, planeswalker, battle)| {
                (
                    ObjectId::from_raw(*object),
                    ironsmith::combat_state::AttackedPermanentTypes {
                        planeswalker: *planeswalker,
                        battle: *battle,
                    },
                )
            })
            .collect(),
        blocked_attackers: combat.blocked_attackers.iter().map(|id| ObjectId::from_raw(*id)).collect(),
        attackers: combat
            .attackers
            .iter()
            .map(|(creature, target)| ironsmith::combat_state::AttackerInfo {
                creature: ObjectId::from_raw(*creature),
                target: match target {
                    SyncGrandMeleeAttackTarget::Player { player } => {
                        AttackTarget::Player(PlayerId::from_index(*player))
                    }
                    SyncGrandMeleeAttackTarget::Planeswalker { object } => {
                        AttackTarget::Planeswalker(ObjectId::from_raw(*object))
                    }
                    SyncGrandMeleeAttackTarget::Battle { object } => {
                        AttackTarget::Battle(ObjectId::from_raw(*object))
                    }
                    SyncGrandMeleeAttackTarget::Nothing {
                        defending_player,
                        was_planeswalker,
                    } => {
                        AttackTarget::Nothing {
                            defending_player: defending_player.map(PlayerId::from_index),
                            was_planeswalker: *was_planeswalker,
                        }
                    }
                },
            })
            .collect(),
        blockers: combat
            .blockers
            .iter()
            .map(|(attacker, blockers)| {
                (ObjectId::from_raw(*attacker), object_ids(blockers.clone()))
            })
            .collect(),
        damage_assignment_order: combat
            .damage_assignment_order
            .iter()
            .map(|(attacker, blockers)| {
                (ObjectId::from_raw(*attacker), object_ids(blockers.clone()))
            })
            .collect(),
        attacking_bands: combat
            .attacking_bands
            .iter()
            .cloned()
            .map(object_ids)
            .collect(),
        had_to_attack_this_combat: combat
            .had_to_attack_this_combat
            .iter()
            .copied()
            .map(ObjectId::from_raw)
            .collect(),
    }
}

fn sync_grand_melee_state(host: &WasmGame) -> Option<SyncGrandMelee> {
    let snapshot = host.game.grand_melee_restore_snapshot()?;
    Some(SyncGrandMelee {
        seats: snapshot.seats.iter().map(|player| player.0).collect(),
        starting_player_count: snapshot.starting_player_count,
        focused_marker: snapshot.focused_marker,
        markers: snapshot
            .markers
            .iter()
            .map(|marker| {
                let focused = marker.number == snapshot.focused_marker;
                let lane = host.grand_melee_host_lanes.get(&marker.number);
                let runner = if focused {
                    host.runner.as_ref()
                } else {
                    lane.and_then(|lane| lane.runner.as_ref())
                };
                let (consecutive_priority_passes, priority_players_in_game) = if focused {
                    host.priority_state.priority_tracker_snapshot()
                } else {
                    lane.map(|lane| lane.priority_state.priority_tracker_snapshot())
                        .unwrap_or_default()
                };
                SyncGrandMeleeMarker {
                    number: marker.number,
                    holder: marker.holder.0,
                    status: match marker.status {
                        ironsmith::GrandMeleeMarkerStatus::Active => "active",
                        ironsmith::GrandMeleeMarkerStatus::Waiting => "waiting",
                    }
                    .to_string(),
                    removal_designations: marker.removal_designations,
                    normal_turn_pending: marker.normal_turn_pending,
                    retained_extra_turn_waiting: marker.retained_extra_turn_waiting,
                    turn: sync_turn_state(&marker.turn),
                    extra_turns: marker
                        .turn_store
                        .extra_turns
                        .iter()
                        .map(|player| player.0)
                        .collect(),
                    stack: marker.stack.iter().map(sync_stack_entry).collect(),
                    combat: marker.combat.as_ref().map(sync_grand_melee_combat),
                    range_turn_snapshot: marker
                        .range_of_influence
                        .as_ref()
                        .map(|range| {
                            range
                                .seats()
                                .iter()
                                .copied()
                                .map(|observer| {
                                    (
                                        observer.0,
                                        range
                                            .players_in_turn_snapshot(observer)
                                            .iter()
                                            .map(|player| player.0)
                                            .collect(),
                                    )
                                })
                                .collect()
                        })
                        .unwrap_or_default(),
                    runner_state: runner.map(|runner| runner.state().sync_name().to_string()),
                    runner_awaiting_priority: if focused {
                        host.runner_awaiting_priority
                    } else {
                        lane.is_some_and(|lane| lane.runner_awaiting_priority)
                    },
                    consecutive_priority_passes,
                    priority_players_in_game,
                }
            })
            .collect(),
        deferred_extra_turns: snapshot
            .deferred_extra_turns
            .iter()
            .map(|(player, count)| (player.0, *count))
            .collect(),
    })
}

fn grand_melee_restore_from_sync(
    sync: &SyncGrandMelee,
) -> Result<ironsmith::GrandMeleeRestore, String> {
    Ok(ironsmith::GrandMeleeRestore {
        seats: sync
            .seats
            .iter()
            .copied()
            .map(PlayerId::from_index)
            .collect(),
        starting_player_count: sync.starting_player_count,
        focused_marker: sync.focused_marker,
        markers: sync
            .markers
            .iter()
            .map(|marker| {
                let status = match marker.status.as_str() {
                    "active" => ironsmith::GrandMeleeMarkerStatus::Active,
                    "waiting" => ironsmith::GrandMeleeMarkerStatus::Waiting,
                    other => {
                        return Err(format!(
                            "unknown Grand Melee marker status: {other}"
                        ));
                    }
                };
                let mut turn_store = ironsmith::game_state::TurnStore::default();
                turn_store.turn_order = sync
                    .seats
                    .iter()
                    .copied()
                    .map(PlayerId::from_index)
                    .collect();
                turn_store.extra_turns = marker
                    .extra_turns
                    .iter()
                    .copied()
                    .map(PlayerId::from_index)
                    .collect();
                Ok(ironsmith::GrandMeleeMarkerRestore {
                    number: marker.number,
                    holder: PlayerId::from_index(marker.holder),
                    status,
                    removal_designations: marker.removal_designations,
                    normal_turn_pending: marker.normal_turn_pending,
                    retained_extra_turn_waiting: marker.retained_extra_turn_waiting,
                    turn: TurnState {
                        active_player: PlayerId::from_index(marker.turn.active_player),
                        priority_player: marker
                            .turn
                            .priority_player
                            .map(PlayerId::from_index),
                        turn_number: marker.turn.turn_number,
                        phase: sync_phase_from_name(&marker.turn.phase)?,
                        step: marker
                            .turn
                            .step
                            .as_deref()
                            .map(sync_step_from_name)
                            .transpose()?,
                    },
                    turn_store,
                    stack: marker.stack.iter().map(stack_entry_from_sync).collect(),
                    combat: marker.combat.as_ref().map(grand_melee_combat_from_sync),
                    range_of_influence: if marker.range_turn_snapshot.is_empty() {
                        None
                    } else {
                        Some(ironsmith::game_state::LimitedRangeOfInfluenceState::from_restore_snapshot(
                            sync.seats
                                .iter()
                                .copied()
                                .map(PlayerId::from_index)
                                .collect(),
                            vec![1; sync.seats.len()],
                            marker
                                .range_turn_snapshot
                                .iter()
                                .map(|(observer, players)| {
                                    (
                                        PlayerId::from_index(*observer),
                                        players
                                            .iter()
                                            .copied()
                                            .map(PlayerId::from_index)
                                            .collect(),
                                    )
                                })
                                .collect(),
                        )?)
                    },
                })
            })
            .collect::<Result<Vec<_>, String>>()?,
        deferred_extra_turns: sync
            .deferred_extra_turns
            .iter()
            .map(|(player, count)| (PlayerId::from_index(*player), *count))
            .collect(),
    })
}

fn raw_ids(ids: &[ObjectId]) -> Vec<u64> {
    ids.iter().map(|id| id.0).collect()
}

fn object_ids(ids: Vec<u64>) -> Vec<ObjectId> {
    ids.into_iter().map(ObjectId::from_raw).collect()
}

fn sync_planechase_state(game: &GameState) -> Option<SyncPlanechase> {
    let state = game.planechase.as_ref()?;
    let mut decks = state
        .decks
        .iter()
        .map(|(owner, deck)| (owner.0, raw_ids(deck)))
        .collect::<Vec<_>>();
    decks.sort_by_key(|(owner, _)| *owner);
    let mut deck_owners = state
        .deck_owners
        .iter()
        .map(|(object, owner)| (object.0, owner.0))
        .collect::<Vec<_>>();
    deck_owners.sort_unstable();
    let mut card_kinds = state
        .card_kinds
        .iter()
        .map(|(object, kind)| {
            (
                object.0,
                match kind {
                    PlanarCardKind::Plane => "plane",
                    PlanarCardKind::Phenomenon => "phenomenon",
                }
                .to_string(),
            )
        })
        .collect::<Vec<_>>();
    card_kinds.sort_by_key(|(object, _)| *object);
    let mut voluntary_rolls_this_turn = state
        .voluntary_rolls_this_turn
        .iter()
        .map(|(player, count)| (player.0, *count))
        .collect::<Vec<_>>();
    voluntary_rolls_this_turn.sort_unstable();
    let mut planar_controllers = state
        .planar_controllers
        .iter()
        .map(|player| player.0)
        .collect::<Vec<_>>();
    planar_controllers.sort_unstable();
    let mut face_up_controllers = state
        .face_up_controllers
        .iter()
        .map(|(object, player)| (object.0, player.0))
        .collect::<Vec<_>>();
    face_up_controllers.sort_unstable();
    Some(SyncPlanechase {
        decks,
        communal_deck: state.communal_deck.as_deref().map(raw_ids),
        deck_owners,
        card_kinds,
        face_up: raw_ids(&state.face_up),
        planar_controller: state.planar_controller.0,
        planar_controllers,
        face_up_controllers,
        voluntary_rolls_this_turn,
        planeswalk_count: state.planeswalk_count,
    })
}

fn public_audit_planechase_state(game: &GameState) -> Option<PublicAuditPlanechase> {
    let state = game.planechase.as_ref()?;
    let mut decks = state
        .decks
        .iter()
        .map(|(owner, deck)| (owner.0, deck.len()))
        .collect::<Vec<_>>();
    decks.sort_unstable();
    let mut voluntary_rolls_this_turn = state
        .voluntary_rolls_this_turn
        .iter()
        .map(|(player, count)| (player.0, *count))
        .collect::<Vec<_>>();
    voluntary_rolls_this_turn.sort_unstable();
    let mut planar_controllers = state
        .planar_controllers
        .iter()
        .map(|player| player.0)
        .collect::<Vec<_>>();
    planar_controllers.sort_unstable();
    let mut face_up_controllers = state
        .face_up_controllers
        .iter()
        .map(|(object, player)| (object.0, player.0))
        .collect::<Vec<_>>();
    face_up_controllers.sort_unstable();
    Some(PublicAuditPlanechase {
        decks,
        communal_deck_size: state.communal_deck.as_ref().map(Vec::len),
        face_up: raw_ids(&state.face_up),
        planar_controller: state.planar_controller.0,
        planar_controllers,
        face_up_controllers,
        voluntary_rolls_this_turn,
        planeswalk_count: state.planeswalk_count,
    })
}

fn sync_vanguard_state(game: &GameState) -> Option<SyncVanguard> {
    let state = game.vanguard.as_ref()?;
    let mut cards = state
        .cards
        .iter()
        .map(|(owner, object)| (owner.0, object.0))
        .collect::<Vec<_>>();
    let mut hand_modifiers = state
        .hand_modifiers
        .iter()
        .map(|(owner, modifier)| (owner.0, *modifier))
        .collect::<Vec<_>>();
    let mut life_modifiers = state
        .life_modifiers
        .iter()
        .map(|(owner, modifier)| (owner.0, *modifier))
        .collect::<Vec<_>>();
    cards.sort_unstable();
    hand_modifiers.sort_unstable();
    life_modifiers.sort_unstable();
    Some(SyncVanguard {
        cards,
        hand_modifiers,
        life_modifiers,
    })
}

fn archenemy_variant_name(variant: ArchenemyVariant) -> &'static str {
    match variant {
        ArchenemyVariant::Default => "default",
        ArchenemyVariant::SupervillainRumble => "supervillain_rumble",
        ArchenemyVariant::Commander => "commander",
    }
}

fn sync_archenemy_state(game: &GameState) -> Option<SyncArchenemy> {
    let state = game.archenemy.as_ref()?;
    let mut archenemies = state
        .archenemies
        .iter()
        .map(|player| player.0)
        .collect::<Vec<_>>();
    archenemies.sort_unstable();
    let mut decks = state
        .scheme_decks
        .iter()
        .map(|(owner, deck)| (owner.0, raw_ids(deck)))
        .collect::<Vec<_>>();
    decks.sort_by_key(|(owner, _)| *owner);
    Some(SyncArchenemy {
        variant: archenemy_variant_name(state.variant).to_string(),
        archenemies,
        decks,
        face_up: raw_ids(&state.face_up),
    })
}

fn public_audit_archenemy_state(game: &GameState) -> Option<PublicAuditArchenemy> {
    let state = game.archenemy.as_ref()?;
    let mut archenemies = state
        .archenemies
        .iter()
        .map(|player| player.0)
        .collect::<Vec<_>>();
    archenemies.sort_unstable();
    let mut decks = state
        .scheme_decks
        .iter()
        .map(|(owner, deck)| (owner.0, deck.len()))
        .collect::<Vec<_>>();
    decks.sort_unstable();
    Some(PublicAuditArchenemy {
        variant: archenemy_variant_name(state.variant).to_string(),
        archenemies,
        decks,
        face_up: raw_ids(&state.face_up),
    })
}

fn sync_conspiracy_state(game: &GameState) -> Option<SyncConspiracy> {
    let state = game.conspiracy.as_ref()?;
    let mut cards = state
        .cards
        .iter()
        .map(|(owner, cards)| (owner.0, raw_ids(cards)))
        .collect::<Vec<_>>();
    cards.sort_by_key(|(owner, _)| *owner);
    let mut face_down = raw_ids(&state.face_down.iter().copied().collect::<Vec<_>>());
    face_down.sort_unstable();
    let mut agenda_names = state
        .agenda_names
        .iter()
        .map(|(object, names)| (object.0, names.clone()))
        .collect::<Vec<_>>();
    agenda_names.sort_by_key(|(object, _)| *object);
    Some(SyncConspiracy {
        cards,
        face_down,
        agenda_names,
    })
}

fn public_audit_conspiracy_state(game: &GameState) -> Option<PublicAuditConspiracy> {
    let state = game.conspiracy.as_ref()?;
    let mut cards = state
        .cards
        .iter()
        .map(|(owner, cards)| (owner.0, raw_ids(cards)))
        .collect::<Vec<_>>();
    cards.sort_by_key(|(owner, _)| *owner);
    let mut face_down = state
        .face_down
        .iter()
        .map(|object| object.0)
        .collect::<Vec<_>>();
    face_down.sort_unstable();
    Some(PublicAuditConspiracy { cards, face_down })
}

fn vanguard_state_from_sync(sync: &SyncVanguard) -> VanguardState {
    VanguardState {
        cards: sync
            .cards
            .iter()
            .map(|(owner, object)| (PlayerId::from_index(*owner), ObjectId::from_raw(*object)))
            .collect(),
        hand_modifiers: sync
            .hand_modifiers
            .iter()
            .map(|(owner, modifier)| (PlayerId::from_index(*owner), *modifier))
            .collect(),
        life_modifiers: sync
            .life_modifiers
            .iter()
            .map(|(owner, modifier)| (PlayerId::from_index(*owner), *modifier))
            .collect(),
    }
}

fn archenemy_state_from_sync(sync: &SyncArchenemy) -> Result<ArchenemyState, String> {
    let variant = match sync.variant.as_str() {
        "default" => ArchenemyVariant::Default,
        "supervillain_rumble" => ArchenemyVariant::SupervillainRumble,
        "commander" => ArchenemyVariant::Commander,
        other => {
            return Err(format!(
                "unknown Archenemy variant in checkpoint: {other}"
            ));
        }
    };
    Ok(ArchenemyState {
        variant,
        archenemies: sync
            .archenemies
            .iter()
            .map(|player| PlayerId::from_index(*player))
            .collect(),
        scheme_decks: sync
            .decks
            .iter()
            .map(|(owner, deck)| (PlayerId::from_index(*owner), object_ids(deck.clone())))
            .collect(),
        face_up: object_ids(sync.face_up.clone()),
    })
}

fn conspiracy_state_from_sync(sync: &SyncConspiracy) -> ConspiracyState {
    ConspiracyState {
        cards: sync
            .cards
            .iter()
            .map(|(owner, cards)| (PlayerId::from_index(*owner), object_ids(cards.clone())))
            .collect(),
        face_down: sync
            .face_down
            .iter()
            .map(|object| ObjectId::from_raw(*object))
            .collect(),
        agenda_names: sync
            .agenda_names
            .iter()
            .map(|(object, names)| (ObjectId::from_raw(*object), names.clone()))
            .collect(),
    }
}

fn planechase_state_from_sync(sync: &SyncPlanechase) -> Result<PlanechaseState, String> {
    let mut card_kinds = std::collections::BTreeMap::new();
    for (object, kind) in &sync.card_kinds {
        let kind = match kind.as_str() {
            "plane" => PlanarCardKind::Plane,
            "phenomenon" => PlanarCardKind::Phenomenon,
            other => {
                return Err(format!(
                    "unknown planar card kind in checkpoint: {other}"
                ));
            }
        };
        card_kinds.insert(ObjectId::from_raw(*object), kind);
    }
    let planar_controller = PlayerId::from_index(sync.planar_controller);
    let face_up = object_ids(sync.face_up.clone());
    Ok(PlanechaseState {
        decks: sync
            .decks
            .iter()
            .map(|(owner, deck)| (PlayerId::from_index(*owner), object_ids(deck.clone())))
            .collect(),
        communal_deck: sync.communal_deck.clone().map(object_ids),
        deck_owners: sync
            .deck_owners
            .iter()
            .map(|(object, owner)| (ObjectId::from_raw(*object), PlayerId::from_index(*owner)))
            .collect(),
        card_kinds,
        face_up: face_up.clone(),
        planar_controller,
        planar_controllers: if sync.planar_controllers.is_empty() {
            std::collections::BTreeSet::from([planar_controller])
        } else {
            sync.planar_controllers
                .iter()
                .map(|player| PlayerId::from_index(*player))
                .collect()
        },
        face_up_controllers: if sync.face_up_controllers.is_empty() {
            face_up
                .into_iter()
                .map(|object| (object, planar_controller))
                .collect()
        } else {
            sync.face_up_controllers
                .iter()
                .map(|(object, player)| {
                    (ObjectId::from_raw(*object), PlayerId::from_index(*player))
                })
                .collect()
        },
        voluntary_rolls_this_turn: sync
            .voluntary_rolls_this_turn
            .iter()
            .map(|(player, count)| (PlayerId::from_index(*player), *count))
            .collect(),
        planeswalk_count: sync.planeswalk_count,
    })
}

fn public_audit_protocol_name() -> String {
    "mental_poker_bayer_groth_v1".to_string()
}

#[wasm_bindgen]
impl WasmGame {
    fn public_audit_known_object_identity(object: &Object) -> PublicAuditObjectIdentity {
        PublicAuditObjectIdentity {
            name: object.name.to_string(),
            card_types: object
                .card_types
                .iter()
                .map(|card_type| card_type.name().to_string())
                .collect(),
            subtypes: object
                .subtypes
                .iter()
                .map(|subtype| subtype.display_name())
                .collect(),
            oracle_text: object.compiled_card_text.to_string(),
        }
    }

    fn public_audit_hidden_zone_entry(&self, position: usize, id: ObjectId) -> serde_json::Value {
        if let Some(info) = self.game.hidden_card_info(id) {
            let public_slot = info.public_slot.unwrap_or(info.slot);
            let public_commitment = info
                .public_commitment
                .as_deref()
                .unwrap_or(info.commitment.as_str());
            return serde_json::json!({
                "position": position,
                "owner": info.owner.0,
                "slot": public_slot,
                "commitment": public_commitment,
                "originSlot": info.origin_slot,
                "originCommitment": info.origin_commitment,
            });
        }

        let Some(object) = self.game.object(id) else {
            return serde_json::json!({
                "position": position,
                "kind": "missing_object",
                "object": id.0,
            });
        };

        serde_json::json!({
            "position": position,
            "kind": "known_object",
            "stableId": object.stable_id.0.0,
            "owner": object.owner.0,
            "controller": self.game.controller_of(object).0,
            "zone": sync_zone_name(object.zone),
            "identity": Self::public_audit_known_object_identity(object),
            "objectKind": object.kind.name(),
            "token": matches!(object.kind, ironsmith::object::ObjectKind::Token),
            "power": object.power(),
            "toughness": object.toughness(),
            "loyalty": object.loyalty(),
            "defense": object.defense(),
            "counters": object
                .counters
                .iter()
                .map(|(kind, amount)| SyncCounter {
                    kind: sync_counter_kind(*kind),
                    counter_type: Some(*kind),
                    amount: *amount,
                })
                .collect::<Vec<_>>(),
            "faceDown": self.game.is_face_down(id),
            "manifested": self.game.is_manifested(id),
            "foretold": self.game.is_foretold(id),
            "foretoldTurn": self.game.foretold_turn(id),
            "suspected": self.game.is_suspected(id),
            "plottedBy": self.game.plotted_by(id).map(|player| player.0),
            "plottedTurn": self.game.plotted_turn(id),
            "commander": self.game.is_commander_object(id),
        })
    }

    fn public_audit_commitment_root(
        &self,
        owner: PlayerId,
        zone_name: &str,
        ids: &[ObjectId],
    ) -> Option<String> {
        let entries = ids
            .iter()
            .enumerate()
            .map(|(position, id)| self.public_audit_hidden_zone_entry(position, *id))
            .collect::<Vec<_>>();
        let bytes = serde_json::to_vec(&serde_json::json!({
            "domain": "ironsmith-public-hidden-zone-root-v1",
            "owner": owner.0,
            "zone": zone_name,
            "entries": entries,
        }))
        .ok()?;
        Some(
            Sha256::digest(&bytes)
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect(),
        )
    }

    fn sync_checkpoint_object_ids(&self) -> Vec<ObjectId> {
        let mut ids = Vec::new();
        for player in &self.game.players {
            ids.extend(player.library.iter().copied());
            ids.extend(player.hand.iter().copied());
            ids.extend(player.graveyard.iter().copied());
            ids.extend(player.sideboard.iter().copied());
            ids.extend(player.attachments.iter().copied());
            ids.extend(player.commanders.iter().copied());
        }
        ids.extend(self.game.battlefield.iter().copied());
        ids.extend(self.game.exile.iter().copied());
        ids.extend(self.game.command_zone.iter().copied());
        ids.extend(self.game.ante.iter().copied());
        ids.extend(self.game.stack.iter().filter_map(|entry| self.game.object(entry.object_id).map(|_| entry.object_id)));
        // Proposed spells join game.stack only after costs are paid, and
        // resolving spells leave it before interactive effects finish. Both
        // still exist in Zone::Stack and must retain their trusted identity.
        ids.extend(
            self.game
                .objects_in_deterministic_order()
                .into_iter()
                .filter(|object| object.zone == Zone::Stack)
                .map(|object| object.id),
        );
        ids.sort_unstable();
        ids.dedup();
        ids
    }

    /// Test shorthand for [`WasmGame::try_build_sync_checkpoint`].
    #[cfg(test)]
    pub(crate) fn build_sync_checkpoint(&self) -> SyncCheckpoint {
        self.try_build_full_sync_checkpoint()
            .expect("sync checkpoint should encode")
    }

    /// Build this engine's full checkpoint. Fails when the shared hidden-claim
    /// ledger holds an entry without a lossless encoding (it is never
    /// silently dropped).
    pub(crate) fn try_build_sync_checkpoint(&self) -> Result<SyncCheckpoint, JsValue> {
        self.try_build_full_sync_checkpoint().map_err(|error| JsValue::from_str(&error))
    }

    fn try_build_full_sync_checkpoint(&self) -> Result<SyncCheckpoint, String> {
        let mut checkpoint = self.try_build_sync_checkpoint_metadata()
            .map_err(|error| format!("checkpoint metadata failed: {error:?}"))?;
        let objects = self.sync_checkpoint_object_ids().into_iter()
            .map(|id| self.game.object(id).cloned()
                .ok_or_else(|| format!("missing full checkpoint object {}", id.0)))
            .collect::<Result<Vec<_>, _>>()?;
        checkpoint.executable_state = Some(SyncExecutableState::retain(
            &self.game, &self.registry, objects,
            self.game.effect_store.continuous_effects.registered_state(),
            self.game.effect_store.replacement_effects.registered_state()
                ?,
            self.game.effect_store.prevention_effects.retained_state()
                ?,
        )?);
        checkpoint.execution_kind = SyncCheckpointExecutionKind::FullExecutable;
        Ok(checkpoint)
    }

    /// Metadata preparation does not discover executable graph closure. A
    /// perspective exporter must authorize roots before asking a codec to visit
    /// hidden histories, costs, actions or embedded card definitions.
    fn try_build_sync_checkpoint_metadata(&self) -> Result<SyncCheckpoint, JsValue> {
        let players = self
            .game
            .players
            .iter()
            .map(|player| Ok(SyncPlayer {
                id: player.id.0,
                name: player.name.clone(),
                starting_life: player.starting_life,
                life: player.life,
                mana_pool: SyncManaPool::from(&player.mana_pool),
                restricted_mana: sync_restricted_mana(&player.restricted_mana)
                    .map_err(|error| JsValue::from_str(&error))?,
                poison_counters: player.poison_counters,
                energy_counters: player.energy_counters,
                experience_counters: player.experience_counters,
                ring_temptations: player.ring_temptations,
                lands_played_this_turn: player.lands_played_this_turn,
                land_plays_per_turn: player.land_plays_per_turn,
                max_hand_size: player.max_hand_size,
                has_lost: player.has_lost,
                has_won: player.has_won,
                has_left_game: player.has_left_game,
                library: raw_ids(&player.library),
                hand: raw_ids(&player.hand),
                graveyard: raw_ids(&player.graveyard),
                sideboard: raw_ids(&player.sideboard),
                commanders: raw_ids(&player.commanders),
                commander_color_identities: player
                    .commander_color_identities
                    .iter()
                    .map(|(id, identity)| (id.0, *identity))
                    .collect(),
            }))
            .collect::<Result<Vec<_>, JsValue>>()?;

        let objects = self
            .sync_checkpoint_object_ids()
            .into_iter()
            .filter_map(|id| {
                let object = self.game.object(id)?;
                Some(SyncObject {
                    id: object.id.0,
                    stable_id: object.stable_id.0.0,
                    owner: object.owner.0,
                    initial_controller: object.initial_controller.0,
                    controller: self.game.controller_of(object).0,
                    zone: sync_zone_name(object.zone).to_string(),
                    name: object.name.to_string(),
                    original_card_name: object.card
                        .and_then(|card_id| self.registry.get_by_id(card_id))
                        .map(|definition| definition.card.name.clone()),
                    token: matches!(object.kind, ironsmith::object::ObjectKind::Token),
                    card_types: object
                        .card_types
                        .iter()
                        .map(|card_type| card_type.name().to_string())
                        .collect(),
                    subtypes: object
                        .subtypes
                        .iter()
                        .map(|subtype| subtype.display_name())
                        .collect(),
                    chosen_subtype: self.game.chosen_subtype(id),
                    chosen_subtypes: {
                        let mut types: Vec<_> = self.game.chosen_subtypes(id)
                            .into_iter().flatten().copied().collect();
                        types.sort_by_key(|subtype| subtype.display_name());
                        types
                    },
                    power: object.power(),
                    toughness: object.toughness(),
                    loyalty: object.loyalty(),
                    defense: object.defense(),
                    hand_modifier: object.hand_modifier,
                    life_modifier: object.life_modifier,
                    oracle_text: object.compiled_card_text.to_string(),
                    counters: object
                        .counters
                        .iter()
                        .map(|(kind, amount)| SyncCounter {
                            kind: sync_counter_kind(*kind),
                            counter_type: Some(*kind),
                            amount: *amount,
                        })
                        .collect(),
                    counter_ability_state: Some(SyncCounterAbilityState::from_object(object)),
                    attached_to: object.attached_to.map(sync_attachment_target),
                    attachments: raw_ids(&object.attachments),
                    tapped: self.game.is_tapped(id),
                    summoning_sick: self.game.is_summoning_sick(id),
                    monstrous: self.game.is_monstrous(id),
                    renowned: self.game.is_renowned(id),
                    saga_entry_lore_processed: self.game.has_processed_saga_entry_lore(id),
                    saddled: self.game.is_saddled(id),
                    flipped: self.game.is_flipped(id),
                    face_down: self.game.is_face_down(id),
                    manifested: self.game.is_manifested(id),
                    phased_out: self.game.is_phased_out(id),
                    madness_exiled: self.game.is_madness_exiled(id),
                    foretold: self.game.is_foretold(id),
                    foretold_turn: self.game.foretold_turn(id),
                    suspected: self.game.is_suspected(id),
                    prepared: self.game.is_prepared(id),
                    prepared_spell_source: self
                        .game
                        .prepared_spell_source(id)
                        .map(|source| source.0),
                    class_level: self.game.class_level(id),
                    room_no_unlocked_door: self.game.room_has_no_unlocked_door(id),
                    room_fully_unlocked: self.game.is_room_fully_unlocked(id),
                    case_solved: self.game.is_case_solved(id),
                    plotted_by: self.game.plotted_by(id).map(|player| player.0),
                    plotted_turn: self.game.plotted_turn(id),
                    damage_marked: self.game.damage_on(id),
                    commander: self.game.is_commander_object(id),
                    hidden_card: self.game.hidden_card_info(id).map(|info| SyncHiddenCard {
                        owner: info.owner.0,
                        slot: info.slot,
                        commitment: info.commitment.clone(),
                        origin_slot: info.origin_slot,
                        origin_commitment: info.origin_commitment.clone(),
                        public_slot: info.public_slot,
                        public_commitment: info.public_commitment.clone(),
                    }),
                })
            })
            .collect();

        let (consecutive_priority_passes, priority_players_in_game) =
            self.priority_state.priority_tracker_snapshot();

        Ok(SyncCheckpoint {
            version: SYNC_CHECKPOINT_VERSION,
            execution_kind: SyncCheckpointExecutionKind::PerspectiveMetadata,
            executable_state: None,
            continuous_timestamps: Some(SyncContinuousTimestamps::from_game(&self.game)),
            format: self.match_format,
            perspective: self.perspective.0,
            snapshot_serial: self.snapshot_serial,
            auto_cleanup_discard: self.auto_cleanup_discard,
            auto_choose_single_object_decisions: self.game.auto_choose_single_object_decisions(),
            semantic_threshold: self.semantic_threshold,
            turn: SyncTurn {
                active_player: self.game.turn.active_player.0,
                priority_player: self.game.turn.priority_player.map(|player| player.0),
                turn_number: self.game.turn.turn_number,
                phase: sync_phase_name(self.game.turn.phase).to_string(),
                step: self.game.turn.step.map(sync_step_name).map(str::to_string),
                turn_order: self
                    .game
                    .turn_store
                    .turn_order
                    .iter()
                    .map(|player| player.0)
                    .collect(),
            },
            priority_runtime: SyncPriorityRuntime {
                runner_awaiting_priority: self.runner_awaiting_priority,
                runner_pending_decision: self.runner_pending_decision,
                turn_runner_state: self
                    .runner
                    .as_ref()
                    .map(|runner| runner.state().sync_name().to_string()),
                consecutive_priority_passes,
                priority_players_in_game,
            },
            players,
            objects,
            battlefield: raw_ids(&self.game.battlefield),
            exile: raw_ids(&self.game.exile),
            command: raw_ids(&self.game.command_zone),
            ante: raw_ids(&self.game.ante),
            planechase: sync_planechase_state(&self.game),
            vanguard: sync_vanguard_state(&self.game),
            archenemy: sync_archenemy_state(&self.game),
            conspiracy: sync_conspiracy_state(&self.game),
            free_for_all: self.game.free_for_all().map(|state| SyncFreeForAll {
                seats: state.seats().iter().map(|player| player.0).collect(),
                attack: match state.attack_option() {
                    ironsmith::FreeForAllAttackOption::Left => FreeForAllAttackInput::Left,
                    ironsmith::FreeForAllAttackOption::Right => FreeForAllAttackInput::Right,
                    ironsmith::FreeForAllAttackOption::MultiplePlayers => {
                        FreeForAllAttackInput::MultiplePlayers
                    }
                },
                range_of_influence: state.range_of_influence(),
            }),
            team_vs_team: self.game.team_vs_team().map(|state| SyncTeamVsTeam {
                teams: state
                    .teams()
                    .iter()
                    .map(|team| team.iter().map(|player| player.0).collect())
                    .collect(),
                seats: state.seats().iter().map(|player| player.0).collect(),
                starting_team: state.starting_team(),
                starting_player: state.starting_player().0,
            }),
            emperor: self.game.emperor().map(|state| SyncEmperor {
                teams: state
                    .teams()
                    .iter()
                    .map(|team| team.iter().map(|player| player.0).collect())
                    .collect(),
                seats: state.seats().iter().map(|player| player.0).collect(),
                ranges: state.ranges().to_vec(),
                starting_team: state.starting_team(),
                starting_emperor: state.starting_emperor().0,
            }),
            two_headed_giant: self
                .game
                .two_headed_giant()
                .map(|state| SyncTwoHeadedGiant {
                    teams: state
                        .teams()
                        .iter()
                        .map(|team| team.iter().map(|player| player.0).collect())
                        .collect(),
                    seats: state.seats().iter().map(|player| player.0).collect(),
                    starting_team: state.starting_team(),
                    starting_player: state.starting_player().0,
                    starting_life: state.starting_life(),
                    poison_threshold: state.poison_threshold(),
                }),
            alternating_teams: self
                .game
                .alternating_teams()
                .map(|state| SyncAlternatingTeams {
                    teams: state
                        .teams()
                        .iter()
                        .map(|team| team.iter().map(|player| player.0).collect())
                        .collect(),
                    seats: state.seats().iter().map(|player| player.0).collect(),
                    starting_player: state.starting_player().0,
                    attack: match state.attack_option() {
                        ironsmith::FreeForAllAttackOption::Left => FreeForAllAttackInput::Left,
                        ironsmith::FreeForAllAttackOption::Right => FreeForAllAttackInput::Right,
                        ironsmith::FreeForAllAttackOption::MultiplePlayers => {
                            FreeForAllAttackInput::MultiplePlayers
                        }
                    },
                    range_of_influence: state.range_of_influence(),
                    deploy_creatures: state.deploy_creatures(),
                }),
            grand_melee: sync_grand_melee_state(self),
            limited_range_of_influence: self.game.limited_range_of_influence().map(|state| {
                SyncLimitedRangeOfInfluence {
                    seats: state.seats().iter().map(|player| player.0).collect(),
                    ranges: state
                        .seats()
                        .iter()
                        .map(|player| state.configured_range(*player).unwrap_or(0))
                        .collect(),
                    turn_snapshot: state
                        .seats()
                        .iter()
                        .map(|player| {
                            (
                                player.0,
                                state
                                    .players_in_turn_snapshot(*player)
                                    .into_iter()
                                    .map(|candidate| candidate.0)
                                    .collect(),
                            )
                        })
                        .collect(),
                }
            }),
            attack_direction: self
                .game
                .attack_direction()
                .map(|direction| match direction {
                    ironsmith::game_state::AttackDirection::Left => SyncAttackDirection::Left,
                    ironsmith::game_state::AttackDirection::Right => SyncAttackDirection::Right,
                }),
            teams: self.game.team_state().map(|state| {
                state
                    .teams()
                    .iter()
                    .map(|team| team.iter().map(|player| player.0).collect())
                    .collect()
            }),
            deploy_creatures: self.game.deploy_creatures_enabled(),
            shared_team_turns: self.game.shared_team_turns_enabled(),
            shared_team_member_orders: self
                .game
                .shared_team_turns()
                .map(|state| {
                    state
                        .member_orders()
                        .iter()
                        .map(|order| order.iter().map(|player| player.0).collect())
                        .collect()
                })
                .unwrap_or_default(),
            stack: self
                .game
                .stack
                .iter()
                .map(|entry| SyncStackEntry {
                    object_id: entry.object_id.0,
                    ability_id: entry.ability_id.map(|id| id.0),
                    ninjutsu_attack_target: entry.ninjutsu_attack_target.as_ref().map(sync_attack_target),
                    controller: entry.controller.0,
                    targets: entry
                        .targets
                        .iter()
                        .copied()
                        .map(sync_target_input)
                        .collect(),
                    is_ability: entry.is_ability,
                    x_value: entry.x_value,
                    source_stable_id: entry.source_stable_id.map(|id| id.0.0),
                    source_name: entry.source_name.clone(),
                })
                .collect(),
            // Sorted: both come from engine hash containers.
            exiled_with_source: {
                let mut entries = self
                    .game
                    .exiled_with_source_entries()
                    .map(|(source, linked)| (source.0, raw_ids(linked)))
                    .collect::<Vec<_>>();
                entries.sort_unstable_by_key(|(source, _)| *source);
                entries
            },
            return_exiled_when_source_leaves: {
                let mut ids = self
                    .game
                    .return_exiled_when_source_leaves_ids()
                    .map(|id| id.0)
                    .collect::<Vec<_>>();
                ids.sort_unstable();
                ids
            },
            rules: self.sync_rules_state().map_err(|error| JsValue::from_str(&error))?,
            id_counters: SyncIdCounters::from_game(&self.game),
        })
    }

    /// Only the shared hidden-claim ledger fields of [`SyncRulesState`], as
    /// `sync_rules_state` encodes them (the public audit checkpoint digests
    /// them on every action, so the rest is not built).
    fn hidden_claim_ledger_rules_state(&self) -> Result<SyncRulesState, String> {
        if self.game.hidden_identity_obligations().is_empty()
            && self.game.hidden_face_down_cast_claims().is_empty()
            && self.game.hidden_claim_subjects().is_empty()
            && self.game.hidden_library_anchors().is_empty()
        {
            return Ok(SyncRulesState::default());
        }
        let full = self.sync_rules_state()?;
        Ok(SyncRulesState {
            hidden_identity_obligations: full.hidden_identity_obligations,
            hidden_face_down_cast_claims: full.hidden_face_down_cast_claims,
            hidden_claim_subjects: full.hidden_claim_subjects,
            hidden_library_anchors: full.hidden_library_anchors,
            ..SyncRulesState::default()
        })
    }

    fn sync_rules_state(&self) -> Result<SyncRulesState, String> {
        // Grand Melee keeps combat and the extra-turn queue per marker lane, and
        // its restore loads the focused lane, so the main copy is not repeated.
        let grand_melee = self.game.grand_melee().is_some();
        let hidden_identity_obligations = self
            .game
            .hidden_identity_obligations()
            .iter()
            .map(sync_hidden_identity_obligation)
            .collect::<Result<Vec<_>, _>>()?;
        let face_down_cast_permissions = self
            .game
            .face_down_cast_permissions()
            .iter()
            .map(|permission| {
                Ok(SyncFaceDownCastPermission {
                    source: permission.source.0,
                    player: permission.player.0,
                    zone: sync_zone_name(permission.zone).to_string(),
                    filter: serde_json::to_string(&permission.filter).map_err(|error| {
                        format!(
                            "face-down cast permission \"{}\" cannot be encoded: {error}",
                            permission.description
                        )
                    })?,
                    description: permission.description.clone(),
                    requires_source_on_battlefield: permission.requires_source_on_battlefield,
                    expires_after_turn: permission.expires_after_turn,
                    single_use: permission.single_use,
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        let (regeneration_shields, regenerated_this_turn) = self.game.regeneration_state();
        Ok(SyncRulesState {
            regeneration_shields: regeneration_shields.into_iter().map(|(id, count)| (id.0, count)).collect(),
            regenerated_this_turn: regenerated_this_turn.into_iter().map(|(id, count)| (id.0, count)).collect(),
            combat: if grand_melee {
                None
            } else {
                self.game.combat.as_ref().map(sync_grand_melee_combat)
            },
            monarch: self.game.monarch.map(|player| player.0),
            initiative: self.game.initiative.map(|player| player.0),
            has_day_night: self.game.has_day_night,
            is_night: self.game.is_night,
            extra_turns: if grand_melee {
                Vec::new()
            } else {
                self.game
                    .turn_store
                    .extra_turns
                    .iter()
                    .map(|player| player.0)
                    .collect()
            },
            pending_restart_battlefield_entries: self
                .game
                .pending_restart_battlefield_entries()
                .iter()
                .map(|entry| SyncRestartBattlefieldEntry {
                    cards: entry.cards.iter().map(|id| id.0).collect(),
                    controller: entry.controller.map(|player| player.0),
                    enters_tapped: entry.enters_tapped,
                })
                .collect(),
            extra_turns_after_next_turn: self
                .game
                .turn_store
                .extra_turns_after_next_turn
                .iter()
                .map(|(player, turn)| (player.0, *turn))
                .collect(),
            turns_taken: self
                .game
                .turn_store
                .turns_taken
                .iter()
                .map(|(player, count)| (player.0, *count))
                .collect(),
            current_turn_is_extra: self.game.turn_store.current_turn_is_extra,
            normal_turn_anchor: self
                .game
                .turn_store
                .normal_turn_anchor
                .map(|player| player.0),
            combat_phases_started_this_turn: self.game.turn_store.combat_phases_started_this_turn,
            main_phases_started_this_turn: self.game.turn_store.main_phases_started_this_turn,
            came_under_control_since_last_upkeep: {
                let mut ids: Vec<u64> = self
                    .game
                    .turn_store
                    .came_under_control_since_last_upkeep
                    .iter()
                    .map(|id| id.0)
                    .collect();
                ids.sort_unstable();
                ids
            },
            echo_eligible_this_upkeep: {
                let mut ids: Vec<u64> = self
                    .game
                    .turn_store
                    .echo_eligible_this_upkeep
                    .iter()
                    .map(|id| id.0)
                    .collect();
                ids.sort_unstable();
                ids
            },
            hidden_draw_reveal_players: self
                .game
                .hidden_draw_reveal_players()
                .into_iter()
                .map(|player| player.0)
                .collect(),
            hidden_splice_players: self
                .game
                .hidden_splice_players()
                .into_iter()
                .map(|player| player.0)
                .collect(),
            publicly_revealed_hidden_cards: self
                .game
                .publicly_revealed_hidden_cards()
                .into_iter()
                .map(|id| id.0)
                .collect(),
            pending_hidden_draw_reveals: self
                .game
                .pending_hidden_draw_reveals()
                .into_iter()
                .map(|(player, card)| (player.0, card.0))
                .collect(),
            pending_hidden_automatic_draw_reveals: self
                .game
                .pending_hidden_automatic_draw_reveals()
                .into_iter()
                .map(|pending| {
                    (
                        pending.player.0,
                        pending.card.0,
                        pending.source.0,
                        pending.optional,
                    )
                })
                .collect(),
            commander_damage: self
                .game
                .players
                .iter()
                .filter(|player| !player.commander_damage.is_empty())
                .map(|player| {
                    let mut damage: Vec<(u64, u32)> = player
                        .commander_damage
                        .iter()
                        .map(|(commander, amount)| (commander.0, *amount))
                        .collect();
                    damage.sort_unstable();
                    (player.id.0, damage)
                })
                .collect(),
            hidden_obligation_ledger_perspective: self
                .game
                .hidden_card_entries()
                .next()
                .map(|_| self.perspective.0),
            hidden_identity_obligations,
            hidden_face_down_cast_claims: self
                .game
                .hidden_face_down_cast_claims()
                .into_iter()
                .map(|(object, kind)| {
                    let (kind, permission_source) = sync_face_down_kind_fields(kind);
                    SyncFaceDownCastClaim {
                        object: object.0,
                        kind,
                        permission_source,
                    }
                })
                .collect(),
            hidden_claim_subjects: self
                .game
                .hidden_claim_subjects()
                .into_iter()
                .map(|stable_id| stable_id.0.0)
                .collect(),
            hidden_library_anchors: self
                .game
                .hidden_library_anchors()
                .iter()
                .map(|anchor| SyncHiddenLibraryAnchor {
                    owner: anchor.owner.0,
                    object_id: anchor.object_id.0,
                    slot: anchor.slot,
                    commitment: anchor.commitment.clone(),
                    origin_slot: anchor.origin_slot,
                    origin_commitment: anchor.origin_commitment.clone(),
                    public_slot: anchor.public_slot,
                    public_commitment: anchor.public_commitment.clone(),
                    known_name: anchor.known_name.clone(),
                })
                .collect(),
            departed_hidden_cards: self
                .game
                .departed_hidden_cards()
                .iter()
                .map(|departed| SyncDepartedHiddenCard {
                    id: departed.object.id.0,
                    stable_id: departed.object.stable_id.0.0,
                    owner: departed.object.owner.0,
                    zone: sync_zone_name(departed.object.zone).to_string(),
                    face_down: departed.face_down,
                    name: departed
                        .object
                        .card
                        .as_ref()
                        .map(|_| departed.object.identity_name().to_string()),
                    hidden: sync_hidden_card(&departed.info),
                })
                .collect(),
            face_down_cast_permissions,
        })
    }

    /// Restore the hidden-claim state of a checkpoint: face-down cast claims
    /// and permissions, claim subjects, library anchors, departed snapshots,
    /// and the obligation ledger. Runs after the checkpoint's objects and
    /// hidden-card metadata are restored.
    ///
    /// The ledger is shared and its encoding lossless, so the imported
    /// ledger replaces this engine's verbatim, whether the checkpoint is this
    /// engine's own savepoint or another peer's (redacted, hash-checked)
    /// checkpoint: nothing held in memory is merged back in. An entry that
    /// does not decode is an error, never dropped.
    fn restore_hidden_claim_state(&mut self, rules: &SyncRulesState) -> Result<(), String> {
        let face_down_claims = rules
            .hidden_face_down_cast_claims
            .iter()
            .map(|claim| {
                face_down_kind_from_sync(&claim.kind, claim.permission_source)
                    .map(|kind| (ObjectId::from_raw(claim.object), kind))
                    .ok_or_else(|| {
                        format!(
                            "face-down cast claim of object {} has an unknown kind {}",
                            claim.object, claim.kind
                        )
                    })
            })
            .collect::<Result<Vec<_>, _>>()?;
        self.game.restore_hidden_face_down_cast_claims(face_down_claims);
        let permissions = rules
            .face_down_cast_permissions
            .iter()
            .map(|permission| {
                let undecodable = |what: &str| {
                    format!(
                        "face-down cast permission \"{}\" has an undecodable {what}",
                        permission.description
                    )
                };
                Ok(ironsmith::game_state::FaceDownCastPermission {
                    source: ObjectId::from_raw(permission.source),
                    player: PlayerId::from_index(permission.player),
                    zone: sync_zone_from_name(&permission.zone).map_err(|_| undecodable("zone"))?,
                    filter: serde_json::from_str(&permission.filter)
                        .map_err(|_| undecodable("filter"))?,
                    description: permission.description.clone(),
                    requires_source_on_battlefield: permission.requires_source_on_battlefield,
                    expires_after_turn: permission.expires_after_turn,
                    single_use: permission.single_use,
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        self.game.restore_face_down_cast_permissions(permissions);
        self.game.restore_hidden_claim_subjects(
            rules
                .hidden_claim_subjects
                .iter()
                .copied()
                .map(StableId::from_raw),
        );
        self.game.restore_hidden_library_anchors(
            rules
                .hidden_library_anchors
                .iter()
                .map(|anchor| ironsmith::game_state::HiddenLibraryAnchor {
                    owner: PlayerId::from_index(anchor.owner),
                    object_id: ObjectId::from_raw(anchor.object_id),
                    slot: anchor.slot,
                    commitment: anchor.commitment.clone(),
                    origin_slot: anchor.origin_slot,
                    origin_commitment: anchor.origin_commitment.clone(),
                    public_slot: anchor.public_slot,
                    public_commitment: anchor.public_commitment.clone(),
                    known_name: anchor.known_name.clone(),
                }),
        );
        let departed: Vec<_> = rules
            .departed_hidden_cards
            .iter()
            .map(|departed| self.departed_hidden_card_from_sync(departed))
            .collect::<Result<Vec<_>, String>>()?;
        self.game.restore_departed_hidden_cards(departed);

        let ledger = rules
            .hidden_identity_obligations
            .iter()
            .map(hidden_identity_obligation_from_sync)
            .collect::<Result<Vec<_>, String>>()?;
        self.game.restore_hidden_identity_obligations(ledger);
        Ok(())
    }

    fn departed_hidden_card_from_sync(
        &mut self,
        departed: &SyncDepartedHiddenCard,
    ) -> Result<ironsmith::game_state::DepartedHiddenCard, String> {
        let id = ObjectId::from_raw(departed.id);
        let owner = PlayerId::from_index(departed.owner);
        let zone = sync_zone_from_name(&departed.zone)
            .map_err(|error| format!("invalid departed hidden-card {}: {error}", departed.id))?;
        let known = departed.name.as_deref()
            .map(|name| self.load_compilable_card_definition_result(name)
                .map_err(|error| format!("invalid departed hidden-card identity: {error}")))
            .transpose()?;
        let mut object = match known {
            Some(definition) => Object::from_card_definition(id, &definition, owner, zone),
            None => Object::new_hidden_card(id, owner, zone),
        };
        object.zone = zone;
        object.stable_id = StableId::from_raw(departed.stable_id);
        Ok(ironsmith::game_state::DepartedHiddenCard {
            object,
            info: HiddenCardInfo {
                owner: PlayerId::from_index(departed.hidden.owner),
                zone,
                slot: departed.hidden.slot,
                commitment: departed.hidden.commitment.clone(),
                origin_slot: departed.hidden.origin_slot,
                origin_commitment: departed.hidden.origin_commitment.clone(),
                public_slot: departed.hidden.public_slot,
                public_commitment: departed.hidden.public_commitment.clone(),
            },
            face_down: departed.face_down,
        })
    }

    fn restore_sync_rules_state(&mut self, rules: &SyncRulesState, grand_melee: bool) {
        if !grand_melee {
            self.game.combat = rules.combat.as_ref().map(grand_melee_combat_from_sync);
            self.game.turn_store.extra_turns = rules
                .extra_turns
                .iter()
                .copied()
                .map(PlayerId::from_index)
                .collect();
        }
        // Assign the designations directly: the setters would replay their
        // side effects (UI events, day/night transformations, returns from
        // exile) that already happened before the checkpoint was taken.
        self.game.monarch = rules.monarch.map(PlayerId::from_index);
        self.game.initiative = rules.initiative.map(PlayerId::from_index);
        self.game.has_day_night = rules.has_day_night;
        self.game.is_night = rules.has_day_night && rules.is_night;
        self.game.set_pending_restart_battlefield_entries(
            rules
                .pending_restart_battlefield_entries
                .iter()
                .map(|entry| ironsmith::game_state::PendingRestartBattlefieldEntry {
                    cards: entry.cards.iter().copied().map(ObjectId::from_raw).collect(),
                    controller: entry.controller.map(PlayerId::from_index),
                    enters_tapped: entry.enters_tapped,
                })
                .collect(),
        );
        self.game.turn_store.extra_turns_after_next_turn = rules
            .extra_turns_after_next_turn
            .iter()
            .map(|(player, turn)| (PlayerId::from_index(*player), *turn))
            .collect();
        self.game.turn_store.turns_taken = rules
            .turns_taken
            .iter()
            .map(|(player, count)| (PlayerId::from_index(*player), *count))
            .collect();
        self.game.turn_store.current_turn_is_extra = rules.current_turn_is_extra;
        self.game.turn_store.normal_turn_anchor =
            rules.normal_turn_anchor.map(PlayerId::from_index);
        self.game.turn_store.combat_phases_started_this_turn =
            rules.combat_phases_started_this_turn;
        self.game.turn_store.main_phases_started_this_turn = rules.main_phases_started_this_turn;
        self.game.turn_store.came_under_control_since_last_upkeep = rules
            .came_under_control_since_last_upkeep
            .iter()
            .copied()
            .map(ObjectId::from_raw)
            .collect();
        self.game.turn_store.echo_eligible_this_upkeep = rules
            .echo_eligible_this_upkeep
            .iter()
            .copied()
            .map(ObjectId::from_raw)
            .collect();
        self.game.set_hidden_draw_reveal_players(
            rules
                .hidden_draw_reveal_players
                .iter()
                .copied()
                .map(PlayerId::from_index),
        );
        self.game.set_hidden_splice_players(
            rules
                .hidden_splice_players
                .iter()
                .copied()
                .map(PlayerId::from_index),
        );
        self.game.restore_publicly_revealed_hidden_cards(
            rules
                .publicly_revealed_hidden_cards
                .iter()
                .copied()
                .map(ObjectId::from_raw),
        );
        self.game.restore_pending_hidden_draw_reveals(
            rules
                .pending_hidden_draw_reveals
                .iter()
                .map(|&(player, card)| (PlayerId::from_index(player), ObjectId::from_raw(card)))
                .collect(),
        );
        self.game.restore_pending_hidden_automatic_draw_reveals(
            rules
                .pending_hidden_automatic_draw_reveals
                .iter()
                .map(|&(player, card, source, optional)| {
                    ironsmith::game_state::PendingAutomaticDrawReveal {
                        player: PlayerId::from_index(player),
                        card: ObjectId::from_raw(card),
                        source: ObjectId::from_raw(source),
                        optional,
                    }
                })
                .collect(),
        );
        for player in &mut self.game.players {
            player.commander_damage = rules
                .commander_damage
                .iter()
                .find(|(seat, _)| *seat == player.id.0)
                .map(|(_, damage)| {
                    damage
                        .iter()
                        .map(|&(commander, amount)| (ObjectId::from_raw(commander), amount))
                        .collect()
                })
                .unwrap_or_default();
        }
    }

    fn public_audit_exile_ids(&self) -> Vec<ObjectId> {
        self.game
            .exile
            .iter()
            .copied()
            .filter(|id| self.public_audit_object_identity_is_public(*id))
            .collect()
    }

    fn public_audit_command_ids(&self) -> Vec<ObjectId> {
        self.game
            .command_zone
            .iter()
            .copied()
            .filter(|id| !self.game.is_planar_card(*id) || self.game.is_face_up_planar_object(*id))
            .filter(|id| !self.game.is_scheme_card(*id) || self.game.is_face_up_scheme(*id))
            .collect()
    }

    fn public_audit_object_ids(&self) -> Vec<ObjectId> {
        let mut ids = Vec::new();
        for player in &self.game.players {
            ids.extend(player.graveyard.iter().copied());
            ids.extend(player.attachments.iter().copied());
            ids.extend(player.commanders.iter().copied());
        }
        ids.extend(self.game.battlefield.iter().copied());
        ids.extend(self.public_audit_exile_ids());
        ids.extend(self.public_audit_command_ids());
        ids.extend(self.game.ante.iter().copied());
        ids.extend(self.game.stack.iter().filter_map(|entry| self.game.object(entry.object_id).map(|_| entry.object_id)));
        ids.sort_unstable();
        ids.dedup();
        ids
    }

    fn public_audit_object_identity_is_public(&self, id: ObjectId) -> bool {
        let Some(object) = self.game.object(id) else {
            return false;
        };
        if matches!(object.zone, Zone::Library | Zone::Hand | Zone::OutsideGame) {
            return false;
        }
        if self.game.is_planar_card(id) && !self.game.is_face_up_planar_object(id) {
            return false;
        }
        if self.game.is_face_down(id)
            || self.game.is_face_down_conspiracy(id)
            || self.game.is_foretold(id)
        {
            return false;
        }
        true
    }

    fn public_audit_object_identity(
        &self,
        id: ObjectId,
        object: &Object,
    ) -> Option<PublicAuditObjectIdentity> {
        self.public_audit_object_identity_is_public(id)
            .then(|| Self::public_audit_known_object_identity(object))
    }

    /// Test shorthand for [`WasmGame::try_build_public_audit_checkpoint`].
    #[cfg(test)]
    pub(crate) fn build_public_audit_checkpoint(&self) -> PublicAuditCheckpoint {
        self.try_build_public_audit_checkpoint()
            .expect("public audit checkpoint should encode")
    }

    pub(crate) fn try_build_public_audit_checkpoint(
        &self,
    ) -> Result<PublicAuditCheckpoint, JsValue> {
        let hidden_claim_ledger_digest = self
            .hidden_claim_ledger_rules_state()
            .and_then(|rules| hidden_claim_ledger_digest(&rules))
            .map_err(|error| JsValue::from_str(&error))?;
        let (consecutive_priority_passes, priority_players_in_game) =
            self.priority_state.priority_tracker_snapshot();
        let players = self
            .game
            .players
            .iter()
            .map(|player| Ok(PublicAuditPlayer {
                id: player.id.0,
                name: player.name.clone(),
                starting_life: player.starting_life,
                life: player.life,
                mana_pool: SyncManaPool::from(&player.mana_pool),
                restricted_mana: sync_restricted_mana(&player.restricted_mana)
                    .map_err(|error| JsValue::from_str(&error))?,
                poison_counters: player.poison_counters,
                energy_counters: player.energy_counters,
                experience_counters: player.experience_counters,
                ring_temptations: player.ring_temptations,
                lands_played_this_turn: player.lands_played_this_turn,
                land_plays_per_turn: player.land_plays_per_turn,
                max_hand_size: player.max_hand_size,
                has_lost: player.has_lost,
                has_won: player.has_won,
                has_left_game: player.has_left_game,
                library_count: player.library.len(),
                hand_count: player.hand.len(),
                sideboard_count: player.sideboard.len(),
                graveyard: raw_ids(&player.graveyard),
                commanders: raw_ids(&player.commanders),
            }))
            .collect::<Result<Vec<_>, JsValue>>()?;

        let objects = self
            .public_audit_object_ids()
            .into_iter()
            .filter_map(|id| {
                let object = self.game.object(id)?;
                // Printed stats of a card whose identity is not public (a
                // face-down exiled or foretold card) are known only to the
                // peers that opened it; hashing them would desync the peers
                // that hold a placeholder. Face-down permanents and spells
                // keep their stats: the face-down overlay makes them public.
                let stats_public = self.public_audit_object_identity_is_public(id)
                    || object.face_down_cast_state.is_some();
                Some(PublicAuditObject {
                    id: object.id.0,
                    stable_id: object.stable_id.0.0,
                    owner: object.owner.0,
                    initial_controller: object.initial_controller.0,
                    controller: self.game.controller_of(object).0,
                    zone: sync_zone_name(object.zone).to_string(),
                    identity: self.public_audit_object_identity(id, object),
                    chosen_subtype: self.game.chosen_subtype(id),
                    chosen_subtypes: {
                        let mut types: Vec<_> = self.game.chosen_subtypes(id)
                            .into_iter().flatten().copied().collect();
                        types.sort_by_key(|subtype| subtype.display_name());
                        types
                    },
                    token: matches!(object.kind, ironsmith::object::ObjectKind::Token),
                    power: stats_public.then(|| object.power()).flatten(),
                    toughness: stats_public.then(|| object.toughness()).flatten(),
                    loyalty: stats_public.then(|| object.loyalty()).flatten(),
                    defense: stats_public.then(|| object.defense()).flatten(),
                    counters: object
                        .counters
                        .iter()
                        .map(|(kind, amount)| SyncCounter {
                            kind: sync_counter_kind(*kind),
                            counter_type: Some(*kind),
                            amount: *amount,
                        })
                        .collect(),
                    attached_to: object.attached_to.map(sync_attachment_target),
                    attachments: raw_ids(&object.attachments),
                    tapped: self.game.is_tapped(id),
                    summoning_sick: self.game.is_summoning_sick(id),
                    monstrous: self.game.is_monstrous(id),
                    renowned: self.game.is_renowned(id),
                    saga_entry_lore_processed: self.game.has_processed_saga_entry_lore(id),
                    saddled: self.game.is_saddled(id),
                    flipped: self.game.is_flipped(id),
                    face_down: self.game.is_face_down(id) || self.game.is_face_down_conspiracy(id),
                    manifested: self.game.is_manifested(id),
                    phased_out: self.game.is_phased_out(id),
                    madness_exiled: self.game.is_madness_exiled(id),
                    foretold: self.game.is_foretold(id),
                    foretold_turn: self.game.foretold_turn(id),
                    suspected: self.game.is_suspected(id),
                    prepared: self.game.is_prepared(id),
                    plotted_by: self.game.plotted_by(id).map(|player| player.0),
                    plotted_turn: self.game.plotted_turn(id),
                    damage_marked: self.game.damage_on(id),
                    commander: self.game.is_commander_object(id),
                })
            })
            .collect();

        let mut hidden_zones = Vec::new();
        for player in &self.game.players {
            hidden_zones.push(PublicAuditHiddenZone {
                owner: player.id.0,
                zone: "library".to_string(),
                count: player.library.len(),
                protocol: public_audit_protocol_name(),
                commitment_root: self.public_audit_commitment_root(
                    player.id,
                    "library",
                    &player.library,
                ),
            });
            hidden_zones.push(PublicAuditHiddenZone {
                owner: player.id.0,
                zone: "hand".to_string(),
                count: player.hand.len(),
                protocol: public_audit_protocol_name(),
                commitment_root: self.public_audit_commitment_root(player.id, "hand", &player.hand),
            });
            if !player.sideboard.is_empty() {
                hidden_zones.push(PublicAuditHiddenZone {
                    owner: player.id.0,
                    zone: "outside_game".to_string(),
                    count: player.sideboard.len(),
                    protocol: public_audit_protocol_name(),
                    commitment_root: self.public_audit_commitment_root(
                        player.id,
                        "outside_game",
                        &player.sideboard,
                    ),
                });
            }
            let hidden_exile_ids = self
                .game
                .exile
                .iter()
                .filter_map(|id| self.game.object(*id).map(|object| (*id, object)))
                .filter(|(_, object)| object.owner == player.id)
                .filter(|(id, _)| !self.public_audit_object_identity_is_public(*id))
                .map(|(id, _)| id)
                .collect::<Vec<_>>();
            if !hidden_exile_ids.is_empty() {
                hidden_zones.push(PublicAuditHiddenZone {
                    owner: player.id.0,
                    zone: "hidden_exile".to_string(),
                    count: hidden_exile_ids.len(),
                    protocol: public_audit_protocol_name(),
                    commitment_root: self.public_audit_commitment_root(
                        player.id,
                        "hidden_exile",
                        &hidden_exile_ids,
                    ),
                });
            }
        }

        if let Some(planechase) = self.game.planechase.as_ref() {
            for (owner, deck) in &planechase.decks {
                hidden_zones.push(PublicAuditHiddenZone {
                    owner: owner.0,
                    zone: "planar_deck".to_string(),
                    count: deck.len(),
                    protocol: public_audit_protocol_name(),
                    commitment_root: self.public_audit_commitment_root(*owner, "planar_deck", deck),
                });
            }
            if let Some(deck) = planechase.communal_deck.as_ref() {
                hidden_zones.push(PublicAuditHiddenZone {
                    owner: planechase.planar_controller.0,
                    zone: "communal_planar_deck".to_string(),
                    count: deck.len(),
                    protocol: public_audit_protocol_name(),
                    commitment_root: self.public_audit_commitment_root(
                        planechase.planar_controller,
                        "communal_planar_deck",
                        deck,
                    ),
                });
            }
        }
        if let Some(archenemy) = self.game.archenemy.as_ref() {
            for (owner, deck) in &archenemy.scheme_decks {
                hidden_zones.push(PublicAuditHiddenZone {
                    owner: owner.0,
                    zone: "scheme_deck".to_string(),
                    count: deck.len(),
                    protocol: public_audit_protocol_name(),
                    commitment_root: self.public_audit_commitment_root(*owner, "scheme_deck", deck),
                });
            }
        }

        Ok(PublicAuditCheckpoint {
            version: PUBLIC_AUDIT_CHECKPOINT_VERSION,
            format: self.match_format,
            perspective: 0,
            snapshot_serial: 0,
            turn: SyncTurn {
                active_player: self.game.turn.active_player.0,
                priority_player: self.game.turn.priority_player.map(|player| player.0),
                turn_number: self.game.turn.turn_number,
                phase: sync_phase_name(self.game.turn.phase).to_string(),
                step: self.game.turn.step.map(sync_step_name).map(str::to_string),
                turn_order: self
                    .game
                    .turn_store
                    .turn_order
                    .iter()
                    .map(|player| player.0)
                    .collect(),
            },
            priority_runtime: SyncPriorityRuntime {
                runner_awaiting_priority: self.runner_awaiting_priority,
                runner_pending_decision: self.runner_pending_decision,
                turn_runner_state: self
                    .runner
                    .as_ref()
                    .map(|runner| runner.state().sync_name().to_string()),
                consecutive_priority_passes,
                priority_players_in_game,
            },
            players,
            objects,
            battlefield: raw_ids(&self.game.battlefield),
            public_exile: raw_ids(&self.public_audit_exile_ids()),
            command: raw_ids(&self.public_audit_command_ids()),
            ante: raw_ids(&self.game.ante),
            planechase: public_audit_planechase_state(&self.game),
            vanguard: sync_vanguard_state(&self.game),
            archenemy: public_audit_archenemy_state(&self.game),
            conspiracy: public_audit_conspiracy_state(&self.game),
            free_for_all: self.game.free_for_all().map(|state| SyncFreeForAll {
                seats: state.seats().iter().map(|player| player.0).collect(),
                attack: match state.attack_option() {
                    ironsmith::FreeForAllAttackOption::Left => FreeForAllAttackInput::Left,
                    ironsmith::FreeForAllAttackOption::Right => FreeForAllAttackInput::Right,
                    ironsmith::FreeForAllAttackOption::MultiplePlayers => {
                        FreeForAllAttackInput::MultiplePlayers
                    }
                },
                range_of_influence: state.range_of_influence(),
            }),
            team_vs_team: self.game.team_vs_team().map(|state| SyncTeamVsTeam {
                teams: state
                    .teams()
                    .iter()
                    .map(|team| team.iter().map(|player| player.0).collect())
                    .collect(),
                seats: state.seats().iter().map(|player| player.0).collect(),
                starting_team: state.starting_team(),
                starting_player: state.starting_player().0,
            }),
            emperor: self.game.emperor().map(|state| SyncEmperor {
                teams: state
                    .teams()
                    .iter()
                    .map(|team| team.iter().map(|player| player.0).collect())
                    .collect(),
                seats: state.seats().iter().map(|player| player.0).collect(),
                ranges: state.ranges().to_vec(),
                starting_team: state.starting_team(),
                starting_emperor: state.starting_emperor().0,
            }),
            two_headed_giant: self
                .game
                .two_headed_giant()
                .map(|state| SyncTwoHeadedGiant {
                    teams: state
                        .teams()
                        .iter()
                        .map(|team| team.iter().map(|player| player.0).collect())
                        .collect(),
                    seats: state.seats().iter().map(|player| player.0).collect(),
                    starting_team: state.starting_team(),
                    starting_player: state.starting_player().0,
                    starting_life: state.starting_life(),
                    poison_threshold: state.poison_threshold(),
                }),
            alternating_teams: self
                .game
                .alternating_teams()
                .map(|state| SyncAlternatingTeams {
                    teams: state
                        .teams()
                        .iter()
                        .map(|team| team.iter().map(|player| player.0).collect())
                        .collect(),
                    seats: state.seats().iter().map(|player| player.0).collect(),
                    starting_player: state.starting_player().0,
                    attack: match state.attack_option() {
                        ironsmith::FreeForAllAttackOption::Left => FreeForAllAttackInput::Left,
                        ironsmith::FreeForAllAttackOption::Right => FreeForAllAttackInput::Right,
                        ironsmith::FreeForAllAttackOption::MultiplePlayers => {
                            FreeForAllAttackInput::MultiplePlayers
                        }
                    },
                    range_of_influence: state.range_of_influence(),
                    deploy_creatures: state.deploy_creatures(),
                }),
            grand_melee: sync_grand_melee_state(self),
            stack: self
                .game
                .stack
                .iter()
                .map(|entry| SyncStackEntry {
                    object_id: entry.object_id.0,
                    ability_id: entry.ability_id.map(|id| id.0),
                    ninjutsu_attack_target: entry.ninjutsu_attack_target.as_ref().map(sync_attack_target),
                    controller: entry.controller.0,
                    targets: entry
                        .targets
                        .iter()
                        .copied()
                        .map(sync_target_input)
                        .collect(),
                    is_ability: entry.is_ability,
                    x_value: entry.x_value,
                    source_stable_id: entry.source_stable_id.map(|id| id.0.0),
                    source_name: entry.source_name.clone(),
                })
                .collect(),
            hidden_zones,
            hidden_claim_ledger_digest,
        })
    }

    fn should_redact_for_perspective(&self, object: &SyncObject, perspective: PlayerId) -> bool {
        let owner = PlayerId::from_index(object.owner);
        match object.zone.as_str() {
            "library" => true,
            "hand" => {
                owner != perspective && !self.game.can_review_teammate_hand(perspective, owner)
            }
            "outside_game" => owner != perspective,
            _ => object.face_down || object.foretold,
        }
    }

    fn redact_sync_object(&self, object: &mut SyncObject) -> Result<(), JsValue> {
        let object_id = ObjectId::from_raw(object.id);
        let Some(info) = self.game.hidden_card_info(object_id) else {
            return Err(JsValue::from_str(&format!(
                "cannot redact object {} without hidden-card commitment metadata",
                object.id
            )));
        };
        object.name = "Hidden Card".to_string();
        object.original_card_name = None;
        object.token = false;
        object.card_types.clear();
        object.subtypes.clear();
        object.power = None;
        object.toughness = None;
        object.loyalty = None;
        object.defense = None;
        object.oracle_text.clear();
        object.hidden_card = Some(SyncHiddenCard {
            owner: info.owner.0,
            slot: info.slot,
            commitment: info.commitment.clone(),
            origin_slot: info.origin_slot,
            origin_commitment: info.origin_commitment.clone(),
            public_slot: info.public_slot,
            public_commitment: info.public_commitment.clone(),
        });
        Ok(())
    }

    fn build_redacted_sync_checkpoint(&self, perspective: PlayerId) -> Result<SyncCheckpoint, JsValue> {
        self.try_build_redacted_executable_checkpoint(perspective).map_err(|error| JsValue::from_str(&error))
    }
    fn try_build_redacted_executable_checkpoint(&self, perspective: PlayerId) -> Result<SyncCheckpoint, String> {
        let mut checkpoint = self.build_redacted_sync_checkpoint_metadata(perspective)
            .map_err(|error| format!("checkpoint metadata failed: {error:?}"))?;
        let policy = self.sync_perspective_policy(&checkpoint, perspective)?;
        let objects = self.sync_checkpoint_object_ids().into_iter().map(|id| {
            let object = self.game.object(id).cloned().ok_or_else(|| "missing perspective object".to_string())?;
            if policy.opaque.contains(&id) {
                // Foretold cards in exile have public physical flags and a
                // public foretell claim. Their hidden face/cost is supplied by
                // a later authenticated opening, not by this executable root.
                let opaque_foretell = self.game.is_foretold(id) && object.zone == Zone::Exile;
                if (self.game.is_face_down(id) || self.game.is_foretold(id)) && !opaque_foretell {
                    return Err("hidden physical face requires executable projection approval".into());
                }
                Ok(opaque_sync_executable_object(object))
            } else { Ok(object) }
        }).collect::<Result<Vec<_>, String>>()?;
        checkpoint.executable_state = Some(SyncExecutableState::retain_with_root_history_and_reference_policy(
            &self.game, &self.registry, objects,
            self.game.effect_store.continuous_effects.registered_state(),
            self.game.effect_store.replacement_effects.registered_state()?,
            self.game.effect_store.prevention_effects.retained_state()?,
            |roots| policy.roots(roots), |history| policy.history(history), |_| Ok(()),
        )?);
        checkpoint.execution_kind = SyncCheckpointExecutionKind::PerspectiveExecutable;
        Ok(checkpoint)
    }
    fn sync_perspective_policy(&self, checkpoint: &SyncCheckpoint, perspective: PlayerId) -> Result<SyncPerspectiveExecutionPolicy, String> {
        let mut visible = std::collections::BTreeSet::new();
        let mut opaque = std::collections::BTreeSet::new();
        for object in &checkpoint.objects {
            let id = ObjectId::from_raw(object.id);
            if self.should_redact_for_perspective(object, perspective) {
                let opaque_foretell = object.foretold && object.zone == "exile";
                if (object.face_down || object.foretold) && !opaque_foretell {
                    return Err("hidden physical face requires executable projection approval".into());
                }
                if object.name != "Hidden Card" || object.original_card_name.is_some()
                    || !object.oracle_text.is_empty() || !object.card_types.is_empty() || !object.subtypes.is_empty() {
                    return Err("perspective object metadata contains private identity".into());
                }
                opaque.insert(id);
            } else { visible.insert(id); }
        }
        Ok(SyncPerspectiveExecutionPolicy { visible, opaque, perspective })
    }
    fn build_redacted_sync_checkpoint_metadata(
        &self,
        perspective: PlayerId,
    ) -> Result<SyncCheckpoint, JsValue> {
        let mut checkpoint = self.try_build_sync_checkpoint_metadata()?;
        checkpoint.perspective = perspective.0;
        for object in &mut checkpoint.objects {
            if self.should_redact_for_perspective(object, perspective) {
                self.redact_sync_object(object)?;
                // Like anchors below: once another owner's card has a public
                // ziffle position, that position is all the perspective may
                // hold (this engine's deck-manifest slot would link the card
                // across shuffles).
                if let Some(hidden) = object.hidden_card.as_mut()
                    && hidden.owner != perspective.0
                {
                    redact_hidden_slot_for_other_perspective(
                        &mut hidden.slot,
                        &mut hidden.commitment,
                        hidden.public_slot,
                        hidden.public_commitment.as_deref(),
                    );
                }
            }
        }
        // The shared claim ledger (obligations, face-down cast claims, claim
        // subjects) is public by construction and exported unchanged.
        // Library anchors and departed snapshots of cards the perspective does
        // not own carry only what it may know: no printed name and, once the
        // card has a public ziffle position, no deck-manifest slot.
        for anchor in &mut checkpoint.rules.hidden_library_anchors {
            if anchor.owner == perspective.0 {
                continue;
            }
            anchor.known_name = None;
            redact_hidden_slot_for_other_perspective(
                &mut anchor.slot,
                &mut anchor.commitment,
                anchor.public_slot,
                anchor.public_commitment.as_deref(),
            );
        }
        for departed in &mut checkpoint.rules.departed_hidden_cards {
            if departed.owner == perspective.0 {
                continue;
            }
            departed.name = None;
            let hidden = &mut departed.hidden;
            redact_hidden_slot_for_other_perspective(
                &mut hidden.slot,
                &mut hidden.commitment,
                hidden.public_slot,
                hidden.public_commitment.as_deref(),
            );
        }
        Ok(checkpoint)
    }

    fn reset_runtime_for_sync_checkpoint(&mut self, checkpoint: &SyncCheckpoint) {
        let player_names = checkpoint
            .players
            .iter()
            .map(|player| player.name.clone())
            .collect::<Vec<_>>();
        let starting_life = checkpoint
            .players
            .first()
            .map(|player| player.starting_life)
            .unwrap_or(20);

        self.game = GameState::new_with_runtime_id_reset(player_names, starting_life);
        // Keep the session card catalog intact. Checkpoint import is game-state
        // reset, but browser-loaded lean-build card definitions must remain
        // available for visible objects and later hidden-card openings.
        self.trigger_queue = TriggerQueue::new();
        self.priority_state = PriorityLoopState::new(checkpoint.players.len());
        self.priority_state.restore_priority_tracker_for_sync(
            checkpoint.priority_runtime.consecutive_priority_passes,
            checkpoint.priority_runtime.priority_players_in_game,
        );
        self.pregame = None;
        self.match_format = checkpoint.format;
        self.game
            .set_commander_damage_loss_enabled(checkpoint.format.commander_damage_loss_enabled());
        self.pending_decision = None;
        self.pending_replay_action = None;
        self.pending_action_checkpoint = None;
        self.pending_live_action_root = None;
        self.pending_live_continuation = None;
        self.game_over = None;
        self.runner = checkpoint
            .priority_runtime
            .turn_runner_state
            .as_deref()
            .and_then(RunnerTurnState::from_sync_name)
            .map(TurnRunner::from_state_for_sync);
        self.grand_melee_host_lanes.clear();
        if self.runner.is_none()
            && (checkpoint.priority_runtime.runner_awaiting_priority
                || checkpoint.priority_runtime.runner_pending_decision)
        {
            self.runner = Some(TurnRunner::new());
        }
        self.runner_awaiting_priority = checkpoint.priority_runtime.runner_awaiting_priority;
        self.runner_pending_decision = checkpoint.priority_runtime.runner_pending_decision;
        self.auto_cleanup_discard = checkpoint.auto_cleanup_discard;
        self.game.set_auto_choose_single_object_decisions(
            checkpoint.auto_choose_single_object_decisions,
        );
        self.priority_epoch_checkpoint = None;
        self.priority_epoch_has_undoable_action = false;
        self.priority_epoch_undo_locked_by_mana = false;
        self.priority_epoch_undo_land_stable_id = None;
        self.semantic_threshold = checkpoint.semantic_threshold;
        self.snapshot_serial = checkpoint.snapshot_serial;
        self.active_viewed_cards = None;
        self.pending_decision_game = None;
        self.active_audit_viewed_cards.clear();
        self.active_resolving_stack_object = None;
        self.last_crypto_requirements.clear();
        self.pending_crypto_audit_before = None;
        self.loaded_decks = Vec::new();
        self.last_snapshot_perf = None;
        self.last_replay_execution_perf = None;
        self.last_advance_until_decision_perf = None;
        self.last_dispatch_perf = None;
        self.dispatch_advance_until_decision_perfs.clear();
        self.cached_snapshot = None;
        *self.mana_activation_inventory_cache.get_mut() = None;
    }

    fn sync_object_from_checkpoint(&mut self, object: &SyncObject) -> Result<Object, String> {
        let id = ObjectId::from_raw(object.id);
        let owner = PlayerId::from_index(object.owner);
        let zone = sync_zone_from_name(&object.zone)?;

        let is_redacted_hidden_card = object.hidden_card.is_some() && object.name == "Hidden Card";
        let mut restored = if is_redacted_hidden_card {
            Object::new_hidden_card(id, owner, zone)
        } else if object.token {
            let card_types = object
                .card_types
                .iter()
                .filter_map(|name| sync_card_type_from_name(name))
                .collect::<Vec<_>>();
            let subtypes = object
                .subtypes
                .iter()
                .filter_map(|name| sync_subtype_from_name(name))
                .collect::<Vec<_>>();
            Object::new_token(
                id,
                owner,
                object.name.clone(),
                if card_types.is_empty() {
                    vec![CardType::Creature]
                } else {
                    card_types
                },
                subtypes,
                object.power,
                object.toughness,
                ColorSet::COLORLESS,
            )
        } else {
            self.ensure_card_definitions_loaded([object.name.as_str()]);
            let definition = self.load_compilable_card_definition_result(&object.name)?;
            self.game
                .register_linked_face_family_from_catalog(&definition, &self.registry);
            Object::from_card_definition(id, &definition, owner, zone)
        };

        // Reconstruct current characteristics above, then restore the original
        // physical identity used to authenticate openings. A copied permanent
        // or alternate face must not become a different physical card on import.
        if !is_redacted_hidden_card && !object.token
            && let Some(original_name) = object.original_card_name.as_deref()
        {
            self.ensure_card_definitions_loaded([original_name]);
            let definition = self.load_compilable_card_definition_result(original_name)?;
            self.game
                .register_linked_face_family_from_catalog(&definition, &self.registry);
            restored.card = Some(definition.card.id);
        }

        restored.zone = zone;
        restored.initial_controller = PlayerId::from_index(object.initial_controller);
        restored.stable_id = StableId::from_raw(object.stable_id);
        restored.hand_modifier = object.hand_modifier;
        restored.life_modifier = object.life_modifier;
        if object.token {
            restored.compiled_card_text = object.oracle_text.clone().into();
            restored.base_loyalty = object.loyalty;
            restored.base_defense = object.defense;
        }
        restored.counters = sync_counters_from_checkpoint(object)?;
        restored.attached_to = object.attached_to.clone().map(attachment_target_from_sync);
        restored.attachments = object_ids(object.attachments.clone());

        Ok(restored)
    }

    fn apply_sync_checkpoint(&mut self, checkpoint: SyncCheckpoint) -> Result<(), String> {
        self.with_runtime_transaction(|candidate| candidate.apply_sync_checkpoint_in_branch(checkpoint))
    }

    fn apply_sync_checkpoint_in_branch(&mut self, checkpoint: SyncCheckpoint) -> Result<(), String> {
        if checkpoint.executable_state.is_some()
        {
            // Validate the entire imported world, including discovery, effective
            // control and priority analysis, before reserving final CardIds.
            // Exchange restores the exact live runtime and its analysis caches
            // on both success and failure; the session catalog stays shared.
            let mut original = RuntimeSavepoint::capture(self);
            original.exchange(self);
            let checked = self.apply_sync_checkpoint_with_bindings(checkpoint.clone(), true).and_then(|()| {
                if matches!(checkpoint.execution_kind, SyncCheckpointExecutionKind::PerspectiveExecutable) {
                    let policy = self.sync_perspective_policy(&checkpoint, PlayerId::from_index(checkpoint.perspective))?;
                    let objects = self.sync_checkpoint_object_ids().into_iter().map(|id|
                        self.game.object(id).cloned().ok_or_else(|| "missing perspective object".to_string())).collect::<Result<Vec<_>, _>>()?;
                    let continuous = self.game.effect_store.continuous_effects.registered_state();
                    let replacement = self.game.effect_store.replacement_effects.registered_state()?;
                    let prevention = self.game.effect_store.prevention_effects.retained_state()?;
                    policy.roots(SyncExecutableRootView { objects: &objects, continuous: &continuous, replacement: &replacement,
                        prevention: &prevention, provenance: self.game.provenance_graph(), grant_registry: &self.game.effect_store.grant_registry,
                        used_grant_permissions: &self.game.turn_store.grant_cast_uses_this_turn, player_count: self.game.players.len(),
                        delayed_triggers: &self.game.effect_store.delayed_triggers, stack: &self.game.stack })?;
                    // Visit every supplied history, including unused matcher captures.
                    // Incoming redaction cannot be inferred from a label or by silently
                    // pruning private/unreachable definitions and occurrence cells.
                    let canonical = SyncExecutableState::retain_with_history_policy(&self.game, &self.registry, objects,
                        continuous, replacement, prevention, |history| policy.history(history))?;
                    if serde_json::to_value(&canonical).map_err(|e| e.to_string())?
                        != serde_json::to_value(checkpoint.executable_state.as_ref().unwrap()).map_err(|e| e.to_string())? {
                        return Err("perspective executable graph has noncanonical or unreachable payloads".into());
                    }
                }
                Ok(())
            });
            original.exchange(self);
            checked?;
        }
        self.apply_sync_checkpoint_with_bindings(checkpoint, false)
    }

    fn apply_foreign_sync_checkpoint_for_perspective(&mut self, checkpoint: SyncCheckpoint, perspective_index: u8) -> Result<(), String> {
        if checkpoint.perspective != perspective_index {
            return Err(format!("foreign sync checkpoint is redacted for seat {}, not seat {perspective_index}", checkpoint.perspective));
        }
        match checkpoint.execution_kind {
            SyncCheckpointExecutionKind::FullExecutable => {
                return Err("foreign sync checkpoint cannot admit a full executable carrier; perspective approval is required".into());
            }
            SyncCheckpointExecutionKind::PerspectiveMetadata | SyncCheckpointExecutionKind::PerspectiveExecutable => {}
        }
        self.apply_sync_checkpoint_for_perspective(checkpoint, perspective_index)
    }

    // String-returning owner adapter is also exercised by native tests; only
    // the WASM boundary maps errors to JsValue.
    fn apply_sync_checkpoint_for_perspective(&mut self, checkpoint: SyncCheckpoint, perspective_index: u8) -> Result<(), String> {
        let player = PlayerId::from_index(perspective_index);
        if checkpoint.players.get(usize::from(perspective_index))
            .is_none_or(|seat| seat.id != perspective_index)
        {
            return Err("invalid player index".into());
        }
        self.apply_sync_checkpoint_in_branch(checkpoint)?;
        self.perspective = player;
        Ok(())
    }

    fn apply_sync_checkpoint_with_bindings(&mut self, checkpoint: SyncCheckpoint, validation_only: bool) -> Result<(), String> {
        if checkpoint.version != SYNC_CHECKPOINT_VERSION {
            return Err(format!(
                "unsupported checkpoint version: {}",
                checkpoint.version
            ));
        }
        match (checkpoint.execution_kind, checkpoint.executable_state.is_some()) {
            (SyncCheckpointExecutionKind::FullExecutable | SyncCheckpointExecutionKind::PerspectiveExecutable, false) => {
                return Err("full checkpoint is missing its executable payload".into());
            }
            (SyncCheckpointExecutionKind::PerspectiveMetadata, true) => {
                return Err("perspective metadata checkpoint cannot contain an unapproved executable payload".into());
            }
            _ => {}
        }
        if checkpoint.players.is_empty() {
            return Err("checkpoint has no players".to_string());
        }

        // Runtime construction assigns consecutive IDs in this exact order.
        // Validate that the wire refers to those same seats, before any reset.
        for (seat, player) in checkpoint.players.iter().enumerate() {
            if usize::from(player.id) != seat {
                return Err(format!("invalid checkpoint player seat {seat}: declared id {}", player.id));
            }
        }
        let player_ids: std::collections::HashSet<_> = checkpoint.players.iter()
            .map(|player| player.id).collect();
        for object in &checkpoint.objects {
            for (role, player) in [("owner", object.owner),
                ("initial controller", object.initial_controller), ("controller", object.controller)] {
                if !player_ids.contains(&player) {
                    return Err(format!("object {} has invalid {role} {player}", object.id));
                }
            }
        }
        // Reject malformed redundant facts before allocating peer-local graph IDs.
        let declared_counters = checkpoint.objects.iter().map(|object| {
            Ok((object.id, sync_counters_from_checkpoint(object)?))
        }).collect::<Result<std::collections::BTreeMap<_, _>, String>>()?;

        let declared_timestamps = checkpoint.continuous_timestamps.clone()
            .map(SyncContinuousTimestamps::into_runtime).transpose()?;
        if let Some(timestamps) = &declared_timestamps {
            // Use the owning clock validator before peer-local allocation. A
            // rejected flat record must not advance exported identity counters.
            ironsmith::continuous::ContinuousEffectManager::new()
                .restore_timestamp_state(timestamps.clone())
                .map_err(|error| format!("invalid continuous chronology: {error}"))?;
        }

        // Decode every executable root before changing the candidate world.
        // CardId allocation is peer-local; its monotonic high-water mark cannot
        // be rewound by runtime rollback, so codec validation precedes allocation.
        let restored_executable = checkpoint.executable_state.as_ref().map(|state| {
            // Counter counts and origin identities contain no peer-local CardIds.
            // Compare these retained facts before allocating the executable graph.
            for object in &state.objects {
                let flat = declared_counters.get(&object.id.0)
                    .ok_or_else(|| "executable object missing from checkpoint metadata".to_string())?;
                let counts = flat.counts().iter().map(|(kind, count)| (*kind, *count))
                    .collect::<Vec<_>>();
                let registrations = ironsmith::object::CounterAbilityState {
                    next_serial: object.counters.next_serial.clone(),
                    origins: object.counters.occurrences.iter()
                        .map(|occurrence| occurrence.origin.clone()).collect(),
                };
                if object.counters.counts != counts || registrations != flat.ability_state() {
                    return Err(format!("executable counter state for object {} contradicts checkpoint metadata", object.id.0));
                }
            }
            // Decode and validate using ordered local bindings that never enter
            // the game or catalog. Malformed roots must not reserve global IDs.
            let validation_graph = (1..=state.graph_card_count).map(CardId).collect::<Vec<_>>();
            for body in &state.event_bodies { validate_sync_event_body_actors(body, checkpoint.players.len())?; }
            let restored = state.restore(&validation_graph)?;
            SyncExecutableState::validate_delayed_actors(&restored.delayed_triggers, checkpoint.players.len())?;
            SyncExecutableState::validate_stack_roots(&restored.provenance_graph, &restored.stack, checkpoint.players.len())?;
            if restored.stack.len() != checkpoint.stack.len() {
                return Err("executable stack length contradicts checkpoint metadata".into());
            }
            for (entry, flat) in restored.stack.iter().zip(&checkpoint.stack) {
                if serde_json::to_value(sync_stack_entry(entry)).map_err(|e| e.to_string())?
                    != serde_json::to_value(flat).map_err(|e| e.to_string())? {
                    return Err("executable stack entry contradicts checkpoint metadata".into());
                }
                if usize::from(entry.controller.0) >= checkpoint.players.len()
                    || entry.defending_player.is_some_and(|p| usize::from(p.0) >= checkpoint.players.len())
                    || entry.chosen_player.is_some_and(|p| usize::from(p.0) >= checkpoint.players.len()) {
                    return Err("executable stack has invalid player".into());
                }
                restored.provenance_graph.validate_reference(entry.provenance)?;
                for assignment in &entry.target_assignments {
                    if assignment.range.start > assignment.range.end || assignment.range.end > entry.targets.len() {
                        return Err("executable stack has invalid target assignment".into());
                    }
                }
            }
            for (player, _) in &restored.used_grant_permissions {
                if !player_ids.contains(&player.0) {
                    return Err("used grant permission has invalid player".into());
                }
            }
            if restored.objects.len() != checkpoint.objects.len() {
                return Err("executable object roots disagree with checkpoint metadata".into());
            }
            let mut declared = std::collections::BTreeMap::new();
            for object in &checkpoint.objects {
                if declared.insert(object.id, object).is_some() {
                    return Err("duplicate checkpoint metadata object".into());
                }
            }
            for object in &restored.objects {
                let flat = declared.get(&object.id.0)
                    .ok_or_else(|| "executable object missing from checkpoint metadata".to_string())?;
                let counters = &declared_counters[&object.id.0];
                if object.counters.counts() != counters.counts()
                    || object.counters.ability_state() != counters.ability_state()
                {
                    return Err(format!("executable counter state for object {} contradicts checkpoint metadata", object.id.0));
                }
                if object.stable_id.0.0 != flat.stable_id
                    || object.owner.0 != flat.owner
                    || object.initial_controller.0 != flat.initial_controller
                    || sync_zone_name(object.zone) != flat.zone
                    || object.name.as_ref() != flat.name
                {
                    return Err(format!("executable object {} contradicts checkpoint metadata", object.id.0));
                }
            }
            if let Some(timestamps) = &declared_timestamps {
                if *timestamps != restored.continuous.timestamps {
                    return Err("executable chronology contradicts checkpoint metadata".into());
                }
            }
            if validation_only {
                // These local bindings remain inside the disposable probe world.
                return Ok(restored);
            }
            // The wrapper has also validated the complete world on local
            // bindings. Only the publication pass reserves peer identities.
            let graph = (0..state.graph_card_count).map(|_| CardId::new()).collect::<Vec<_>>();
            state.restore(&graph)
        }).transpose()?;
        self.reset_runtime_for_sync_checkpoint(&checkpoint);
        if let Some(restored) = &restored_executable {
            for definition in &restored.definitions {
                // Imported programs belong to this world. Do not admit them to
                // the trusted session catalog or overwrite an existing name.
                // The game-local cache is part of the owning transaction.
                self.game.register_linked_face_definition(definition);
            }
        }

        for object in checkpoint.objects.iter() {
            let restored = if let Some(executable) = &restored_executable {
                executable.objects.iter().find(|root| root.id.0 == object.id)
                    .expect("complete root identity validated before reset").clone()
            } else {
                self.sync_object_from_checkpoint(object)?
            };
            let restored_id = restored.id;
            let restored_zone = restored.zone;
            self.game.add_object(restored);
            if let Some(hidden) = &object.hidden_card {
                self.game.set_hidden_card_info(
                    restored_id,
                    HiddenCardInfo {
                        owner: PlayerId::from_index(hidden.owner),
                        zone: restored_zone,
                        slot: hidden.slot,
                        commitment: hidden.commitment.clone(),
                        origin_slot: hidden.origin_slot,
                        origin_commitment: hidden.origin_commitment.clone(),
                        public_slot: hidden.public_slot,
                        public_commitment: hidden.public_commitment.clone(),
                    },
                );
            }
        }

        for player_checkpoint in checkpoint.players.iter() {
            let player_id = PlayerId::from_index(player_checkpoint.id);
            if let Some(player) = self.game.player_mut(player_id) {
                player.life = player_checkpoint.life;
                let restricted_mana = player_checkpoint.restricted_mana.iter().map(|unit| {
                    Ok(ironsmith::ability::RestrictedManaUnit {
                        symbol: unit.symbol,
                        source: unit.source,
                        source_chosen_creature_type: unit.source_chosen_creature_type,
                        restrictions: unit.restrictions.iter().cloned().map(|restriction|
                            restriction.try_map_effects(&mut |_| Err::<ironsmith::effect::Effect, String>("restricted mana executable payload requires an approved graph".into()))
                        ).collect::<Result<_, String>>()?,
                    })
                }).collect::<Result<Vec<_>, String>>()?;
                // The planner matches restricted units through production
                // provenance. Restoring only the side ledger would free this
                // mana in compact payment assignment.
                for unit in restricted_mana { player.add_restricted_mana(unit); }
                player.mana_pool = ManaPool::from(player_checkpoint.mana_pool.clone());
                player.poison_counters = player_checkpoint.poison_counters;
                player.energy_counters = player_checkpoint.energy_counters;
                player.experience_counters = player_checkpoint.experience_counters;
                player.ring_temptations = player_checkpoint.ring_temptations;
                player.lands_played_this_turn = player_checkpoint.lands_played_this_turn;
                player.land_plays_per_turn = player_checkpoint.land_plays_per_turn;
                player.max_hand_size = player_checkpoint.max_hand_size;
                player.has_lost = player_checkpoint.has_lost;
                player.has_won = player_checkpoint.has_won;
                player.has_left_game = player_checkpoint.has_left_game;
                player.library = object_ids(player_checkpoint.library.clone()).into();
                player.hand = object_ids(player_checkpoint.hand.clone()).into();
                player.graveyard = object_ids(player_checkpoint.graveyard.clone()).into();
                player.sideboard = object_ids(player_checkpoint.sideboard.clone()).into();
                player.commanders = object_ids(player_checkpoint.commanders.clone());
                player.commander_color_identities = player_checkpoint
                    .commander_color_identities
                    .iter()
                    .map(|(id, identity)| (ObjectId::from_raw(*id), *identity))
                    .collect();
            }
        }

        self.game.battlefield = object_ids(checkpoint.battlefield.clone()).into();
        self.game.exile = object_ids(checkpoint.exile.clone()).into();
        self.game.command_zone = object_ids(checkpoint.command.clone()).into();
        self.game.ante = object_ids(checkpoint.ante.clone()).into();
        self.game.planechase = checkpoint
            .planechase
            .as_ref()
            .map(planechase_state_from_sync)
            .transpose()?;
        self.game.synchronize_planar_ability_zones();
        self.game.vanguard = checkpoint.vanguard.as_ref().map(vanguard_state_from_sync);
        self.game.synchronize_vanguard_ability_zones();
        self.game.archenemy = checkpoint
            .archenemy
            .as_ref()
            .map(archenemy_state_from_sync)
            .transpose()?;
        self.game.synchronize_scheme_ability_zones();
        self.game.conspiracy = checkpoint
            .conspiracy
            .as_ref()
            .map(conspiracy_state_from_sync);
        if let Some(state) = self.game.conspiracy.as_ref() {
            let names = state
                .agenda_names
                .iter()
                .map(|(object, names)| (*object, names.join("\n")))
                .collect::<Vec<_>>();
            for (object, names) in names {
                self.game.set_chosen_named_option(object, names);
            }
        }
        self.game.synchronize_conspiracy_ability_zones();
        self.game.stack = checkpoint
            .stack
            .iter()
            .map(|entry| {
                let mut stack_entry = StackEntry::new(
                    ObjectId::from_raw(entry.object_id),
                    PlayerId::from_index(entry.controller),
                );
                stack_entry.targets = entry
                    .targets
                    .iter()
                    .cloned()
                    .map(target_from_sync_input)
                    .collect();
                stack_entry.is_ability = entry.is_ability;
                stack_entry.ability_id = entry.ability_id.map(ObjectId::from_raw);
                stack_entry.ninjutsu_attack_target = entry.ninjutsu_attack_target.as_ref().map(attack_target_from_sync);
                stack_entry.x_value = entry.x_value;
                stack_entry.source_stable_id = entry.source_stable_id.map(StableId::from_raw);
                stack_entry.source_name = entry.source_name.clone();
                stack_entry
            })
            .collect();
        self.game.replace_exiled_with_source_links(
            checkpoint
                .exiled_with_source
                .iter()
                .map(|(source, linked)| (ObjectId::from_raw(*source), object_ids(linked.clone())))
                .collect(),
        );
        self.game.replace_return_exiled_when_source_leaves(
            checkpoint
                .return_exiled_when_source_leaves
                .iter()
                .map(|id| ObjectId::from_raw(*id))
                .collect(),
        );

        if let Some(free_for_all) = checkpoint.free_for_all.as_ref() {
            let attack = match free_for_all.attack {
                FreeForAllAttackInput::Left => ironsmith::FreeForAllAttackOption::Left,
                FreeForAllAttackInput::Right => ironsmith::FreeForAllAttackOption::Right,
                FreeForAllAttackInput::MultiplePlayers => {
                    ironsmith::FreeForAllAttackOption::MultiplePlayers
                }
            };
            self.game
                .restore_free_for_all(
                    free_for_all
                        .seats
                        .iter()
                        .copied()
                        .map(PlayerId::from_index)
                        .collect(),
                    attack,
                    free_for_all.range_of_influence,
                )?;
        }
        if let Some(team_vs_team) = checkpoint.team_vs_team.as_ref() {
            self.game
                .restore_team_vs_team(
                    team_vs_team
                        .teams
                        .iter()
                        .map(|team| team.iter().copied().map(PlayerId::from_index).collect())
                        .collect(),
                    team_vs_team
                        .seats
                        .iter()
                        .copied()
                        .map(PlayerId::from_index)
                        .collect(),
                    team_vs_team.starting_team,
                    PlayerId::from_index(team_vs_team.starting_player),
                )?;
        }
        if let Some(emperor) = checkpoint.emperor.as_ref() {
            self.game
                .restore_emperor(
                    emperor
                        .teams
                        .iter()
                        .map(|team| team.iter().copied().map(PlayerId::from_index).collect())
                        .collect(),
                    emperor
                        .seats
                        .iter()
                        .copied()
                        .map(PlayerId::from_index)
                        .collect(),
                    emperor.starting_team,
                    PlayerId::from_index(emperor.starting_emperor),
                    emperor.ranges.clone(),
                )?;
        }
        if let Some(two_headed_giant) = checkpoint.two_headed_giant.as_ref() {
            self.game
                .restore_two_headed_giant(
                    two_headed_giant
                        .teams
                        .iter()
                        .map(|team| team.iter().copied().map(PlayerId::from_index).collect())
                        .collect(),
                    two_headed_giant.starting_team,
                    PlayerId::from_index(two_headed_giant.starting_player),
                )?;
            let profile = self
                .game
                .two_headed_giant()
                .expect("restored Two-Headed Giant profile");
            if profile
                .seats()
                .iter()
                .map(|player| player.0)
                .collect::<Vec<_>>()
                != two_headed_giant.seats
                || profile.starting_life() != two_headed_giant.starting_life
                || profile.poison_threshold() != two_headed_giant.poison_threshold
            {
                return Err("Two-Headed Giant checkpoint profile does not match its team size".to_string());
            }
        }
        if let Some(alternating_teams) = checkpoint.alternating_teams.as_ref() {
            let attack = match alternating_teams.attack {
                FreeForAllAttackInput::Left => ironsmith::FreeForAllAttackOption::Left,
                FreeForAllAttackInput::Right => ironsmith::FreeForAllAttackOption::Right,
                FreeForAllAttackInput::MultiplePlayers => {
                    ironsmith::FreeForAllAttackOption::MultiplePlayers
                }
            };
            self.game
                .restore_alternating_teams(
                    alternating_teams
                        .teams
                        .iter()
                        .map(|team| team.iter().copied().map(PlayerId::from_index).collect())
                        .collect(),
                    alternating_teams
                        .seats
                        .iter()
                        .copied()
                        .map(PlayerId::from_index)
                        .collect(),
                    PlayerId::from_index(alternating_teams.starting_player),
                    attack,
                    alternating_teams.range_of_influence,
                    alternating_teams.deploy_creatures,
                )?;
        }

        self.game.turn = TurnState {
            active_player: PlayerId::from_index(checkpoint.turn.active_player),
            priority_player: checkpoint.turn.priority_player.map(PlayerId::from_index),
            turn_number: checkpoint.turn.turn_number,
            phase: sync_phase_from_name(&checkpoint.turn.phase)?,
            step: checkpoint
                .turn
                .step
                .as_deref()
                .map(sync_step_from_name)
                .transpose()?,
        };
        // Formats without a seating profile keep their starting seat only in
        // the turn order, so restore it before anything reads the rotation.
        // Pre-rotation checkpoints carry no order and keep the default seating.
        if !checkpoint.turn.turn_order.is_empty() {
            let restored = checkpoint
                .turn
                .turn_order
                .iter()
                .copied()
                .map(PlayerId::from_index)
                .collect::<Vec<_>>();
            let seated = restored
                .iter()
                .copied()
                .collect::<std::collections::HashSet<_>>();
            let known = self
                .game
                .players
                .iter()
                .map(|player| player.id)
                .collect::<std::collections::HashSet<_>>();
            if seated.len() == restored.len() && seated == known {
                self.game.turn_store.turn_order = restored;
            }
        }
        if let Some(range) = checkpoint.limited_range_of_influence.as_ref() {
            self.game
                .restore_limited_range_of_influence(
                    range
                        .seats
                        .iter()
                        .copied()
                        .map(PlayerId::from_index)
                        .collect(),
                    range.ranges.clone(),
                    range
                        .turn_snapshot
                        .iter()
                        .map(|(observer, players)| {
                            (
                                PlayerId::from_index(*observer),
                                players.iter().copied().map(PlayerId::from_index).collect(),
                            )
                        })
                        .collect(),
                )?;
        }
        if let Some(grand_melee) = checkpoint.grand_melee.as_ref() {
            self.game
                .restore_grand_melee_snapshot(grand_melee_restore_from_sync(grand_melee)?)?;
            self.grand_melee_host_lanes.clear();
            for marker in &grand_melee.markers {
                if marker.number == grand_melee.focused_marker {
                    continue;
                }
                let mut priority_state = PriorityLoopState::new(
                    marker
                        .priority_players_in_game
                        .max(self.game.players_in_game()),
                );
                priority_state.restore_priority_tracker_for_sync(
                    marker.consecutive_priority_passes,
                    marker.priority_players_in_game,
                );
                self.grand_melee_host_lanes.insert(
                    marker.number,
                    GrandMeleeHostLane {
                        runner: marker
                            .runner_state
                            .as_deref()
                            .and_then(RunnerTurnState::from_sync_name)
                            .map(TurnRunner::from_state_for_sync),
                        runner_awaiting_priority: marker.runner_awaiting_priority,
                        trigger_queue: TriggerQueue::new(),
                        priority_state,
                    },
                );
            }
        }
        self.game
            .set_attack_direction(
                checkpoint
                    .attack_direction
                    .map(|direction| match direction {
                        SyncAttackDirection::Left => ironsmith::game_state::AttackDirection::Left,
                        SyncAttackDirection::Right => ironsmith::game_state::AttackDirection::Right,
                    }),
            );
        if checkpoint.team_vs_team.is_none()
            && checkpoint.emperor.is_none()
            && checkpoint.two_headed_giant.is_none()
            && checkpoint.alternating_teams.is_none()
            && let Some(teams) = checkpoint.teams.as_ref()
        {
            self.game
                .set_teams(
                    teams
                        .iter()
                        .map(|team| team.iter().copied().map(PlayerId::from_index).collect())
                        .collect(),
                )?;
        }
        if checkpoint.shared_team_turns {
            if checkpoint.two_headed_giant.is_none() {
                self.game
                    .enable_shared_team_turns()?;
            }
            for (team, order) in checkpoint.shared_team_member_orders.iter().enumerate() {
                self.game
                    .set_shared_team_member_order(
                        team,
                        order.iter().copied().map(PlayerId::from_index).collect(),
                    )?;
            }
        }
        self.game.set_deploy_creatures(checkpoint.deploy_creatures);
        for object in &checkpoint.objects {
            let id = ObjectId::from_raw(object.id);
            for subtype in &object.chosen_subtypes {
                self.game.set_chosen_subtype(id, *subtype);
            }
            if let Some(subtype) = object.chosen_subtype {
                self.game.set_chosen_subtype(id, subtype);
            }
        }
        self.restore_sync_rules_state(&checkpoint.rules, checkpoint.grand_melee.is_some());
        self.game.restore_regeneration_state(
            checkpoint.rules.regeneration_shields.iter().map(|&(id, count)| (ObjectId::from_raw(id), count)).collect(),
            checkpoint.rules.regenerated_this_turn.iter().map(|&(id, count)| (ObjectId::from_raw(id), count)).collect(),
        )?;
        self.restore_hidden_claim_state(&checkpoint.rules)?;

        for object in checkpoint.objects.iter() {
            let id = ObjectId::from_raw(object.id);
            if object.tapped {
                self.game.tap(id);
            }
            if object.summoning_sick {
                self.game.set_summoning_sick(id);
            }
            if object.monstrous {
                self.game.set_monstrous(id);
            }
            // The prepare spell copy is restored with the rest of exile, so
            // relink it rather than preparing again (which would mint a second
            // copy). The copy carries the link, so this runs once per pair.
            if let Some(source) = object.prepared_spell_source {
                self.game
                    .restore_prepared_link(ObjectId::from_raw(source), id);
            }
            if object.renowned {
                self.game.set_renowned(id);
            }
            if object.saga_entry_lore_processed {
                self.game.mark_saga_entry_lore_processed(id);
            }
            // Battlefield designations that aren't counters (CR 716.2b,
            // 709.5d-e, 719.3).
            if object.class_level > 1 {
                self.game.set_class_level(id, object.class_level);
            }
            if object.room_no_unlocked_door {
                self.game.mark_room_entered_with_no_unlocked_door(id);
            }
            if object.room_fully_unlocked {
                self.game.mark_room_fully_unlocked(id);
            }
            if object.case_solved {
                self.game.solve_case(id);
            }
            if object.saddled {
                self.game.set_saddled_until_end_of_turn(id);
            }
            if object.flipped {
                self.game.flip(id);
            }
            if object.face_down {
                self.game.set_face_down(id);
            }
            if object.manifested {
                self.game.set_manifested(id);
            }
            if object.phased_out {
                self.game.phase_out(id);
            }
            if object.madness_exiled {
                self.game.set_madness_exiled(id);
            }
            if object.foretold {
                self.game
                    .set_foretold_on_turn(id, object.foretold_turn.unwrap_or(0));
            }
            if object.suspected {
                self.game.set_suspected(id);
            }
            if let Some(player) = object.plotted_by {
                self.game.set_plotted_on_turn(
                    id,
                    PlayerId::from_index(player),
                    object.plotted_turn.unwrap_or(0),
                );
            }
            if object.damage_marked > 0 {
                self.game.set_damage_marked(id, object.damage_marked);
            }
            if object.commander {
                self.game.set_commander(id);
            }
            // Initial control was restored as an object fact. Effective control
            // must come from actual effects, never a reconstructed assignment.
        }

        if let Some(timestamps) = checkpoint.continuous_timestamps {
            self.game
                .restore_continuous_timestamp_state(timestamps.into_runtime()?)
                .map_err(|error| format!("invalid continuous chronology: {error}"))?;
        }

        if let Some(restored) = restored_executable {
            *self.game.provenance_graph_mut() = restored.provenance_graph;
            self.game.effect_store.continuous_effects.restore_registered_state(restored.continuous)?;
            self.game.effect_store.replacement_effects.restore_registered_state(restored.replacement)?;
            self.game.effect_store.prevention_effects.restore_retained_state(restored.prevention)?;
            self.game.effect_store.grant_registry.restore_registered_state(restored.grants)?;
            self.game.turn_store.grant_cast_uses_this_turn = restored.used_grant_permissions.into_iter().collect();
            self.game.effect_store.delayed_triggers = restored.delayed_triggers;
            self.game.stack.clear();
            self.game.stack.extend(restored.stack);
        }

        let mut id_counters = IdCountersSnapshot::from(checkpoint.id_counters.clone());
        // CardIds belong to this peer's session catalog and retained graph.
        // The sender's catalog high-water mark is not a gameplay identity;
        // importing it would mutate this allocator even in a disposable probe.
        // Preserve every actual local allocation, including catalog admission.
        id_counters.card = snapshot_id_counters().card;
        restore_id_counters(id_counters);
        self.game.set_next_object_id_counter(id_counters.object);
        self.game
            .set_next_stack_ability_id_counter(checkpoint.id_counters.stack_ability);
        self.game = self.game.continuous_query_snapshot()
            .map_err(|error| format!("imported checkpoint continuous discovery failed: {error}"))?;
        for object in &checkpoint.objects {
            let restored = self.game.current_controller(ObjectId::from_raw(object.id));
            if restored != Some(PlayerId::from_index(object.controller)) {
                return Err(format!("checkpoint cannot restore effective controller for object {}: recorded {}, restored {:?}; actual control effects are required",
                    object.id, object.controller, restored));
            }
        }
        self.pending_decision = self.game.turn.priority_player.map(|player| {
            ironsmith::game_loop::priority_context(&self.game, player).map(DecisionContext::Priority)
        }).transpose().map_err(|error| format!("priority action analysis failed: {error}"))?;
        Ok(())
    }

    /// Export a WASM-owned resync checkpoint that can hydrate another peer's
    /// engine. Fails (never drops a claim) when the shared hidden-claim ledger
    /// holds an entry without a lossless encoding.
    #[wasm_bindgen(js_name = exportSyncCheckpoint)]
    pub fn export_sync_checkpoint(&self) -> Result<JsValue, JsValue> {
        serde_wasm_bindgen::to_value(&self.try_build_sync_checkpoint()?)
            .map_err(|e| JsValue::from_str(&format!("sync checkpoint encode failed: {e}")))
    }

    /// Export an importable checkpoint redacted for one peer's legal knowledge.
    #[wasm_bindgen(js_name = exportRedactedSyncCheckpoint)]
    pub fn export_redacted_sync_checkpoint(
        &self,
        perspective_index: u8,
    ) -> Result<JsValue, JsValue> {
        let checkpoint =
            self.build_redacted_sync_checkpoint(PlayerId::from_index(perspective_index))?;
        serde_wasm_bindgen::to_value(&checkpoint)
            .map_err(|e| JsValue::from_str(&format!("redacted sync checkpoint encode failed: {e}")))
    }

    /// Export a redacted checkpoint suitable for peer audit logs.
    #[wasm_bindgen(js_name = exportPublicAuditCheckpoint)]
    pub fn export_public_audit_checkpoint(&self) -> Result<JsValue, JsValue> {
        serde_wasm_bindgen::to_value(&self.try_build_public_audit_checkpoint()?)
            .map_err(|e| JsValue::from_str(&format!("public audit checkpoint encode failed: {e}")))
    }

    /// Replace this WASM engine with a checkpoint from the current authoritative host.
    #[wasm_bindgen(js_name = importSyncCheckpoint)]
    pub fn import_sync_checkpoint(
        &mut self,
        checkpoint: JsValue,
        perspective_index: u8,
    ) -> Result<JsValue, JsValue> {
        let checkpoint: SyncCheckpoint = serde_wasm_bindgen::from_value(checkpoint)
            .map_err(|e| JsValue::from_str(&format!("invalid sync checkpoint: {e}")))?;
        self.with_runtime_transaction(|candidate| {
            candidate.apply_sync_checkpoint_for_perspective(checkpoint, perspective_index)
                .map_err(|error| JsValue::from_str(&error))?;
            candidate.snapshot()
        })
    }

    /// Import a checkpoint another peer exported for this engine's
    /// perspective (`exportRedactedSyncCheckpoint(perspective)` on the
    /// exporter), after the caller checked it against the agreed public
    /// checkpoint hash. Returns this engine's resulting public audit
    /// checkpoint (`exportPublicAuditCheckpoint()` shape) so the caller can
    /// recompute its hash immediately and compare.
    ///
    /// The shared hidden-claim ledger (obligations, face-down cast claims,
    /// claim subjects, library anchors) is REPLACED by the imported one, as
    /// is every other piece of state: nothing this engine held is merged
    /// back. (`importSyncCheckpoint` has the same replace semantics; this
    /// entry point additionally requires the checkpoint to be redacted for
    /// `perspective_index` and returns the audit checkpoint instead of a UI
    /// snapshot.)
    ///
    /// A foreign checkpoint carries only what the exporter may tell this
    /// perspective. Missing afterwards, and to be re-hydrated by the caller
    /// (reopening its own openings / private views), are exactly this
    /// perspective's own private facts the exporter does not know:
    ///
    /// * the identities of this player's own hidden hand cards (they arrive
    ///   as the exporter's "Hidden Card" placeholders, with the exporter's
    ///   commitment metadata), unless the exporter legitimately knew them
    ///   (a public reveal, or a private reveal to the exporter);
    /// * the identities of this player's own face-down permanents / spells /
    ///   foretold exile cards (every face-down or foretold object is redacted
    ///   for every perspective, the owner's included);
    /// * everything this player learned through private views: cards of
    ///   other players' hands it was shown, library cards it looked at or
    ///   searched (every library card is redacted), and cards revealed only
    ///   to it;
    /// * its own library-order knowledge (library cards are anonymous
    ///   placeholders) and its own cards' deck-manifest slots where the card
    ///   has a public ziffle position (only that position is exported for
    ///   hidden cards of other owners; for its own cards it gets what the
    ///   exporter held);
    /// * printed names on library anchors and departed hidden-card snapshots
    ///   of its own cards, when the exporter did not know them;
    /// * a teammate's hand, when this perspective may review it but the
    ///   exporter only held placeholders.
    ///
    /// Everything public, including the whole claim ledger, is exact.
    #[wasm_bindgen(js_name = importForeignSyncCheckpoint)]
    pub fn import_foreign_sync_checkpoint(
        &mut self,
        checkpoint: JsValue,
        perspective_index: u8,
    ) -> Result<JsValue, JsValue> {
        let checkpoint: SyncCheckpoint = serde_wasm_bindgen::from_value(checkpoint)
            .map_err(|e| JsValue::from_str(&format!("invalid sync checkpoint: {e}")))?;
        self.with_runtime_transaction(|candidate| {
            candidate.apply_foreign_sync_checkpoint_for_perspective(checkpoint, perspective_index)
                .map_err(|error| JsValue::from_str(&error))?;
            candidate.export_public_audit_checkpoint()
        })
    }
}

#[cfg(test)]
fn test_delayed_registration(turn: u32, alice: PlayerId) -> ironsmith::triggers::DelayedTrigger {
    ironsmith::triggers::DelayedTrigger {
            trigger: ironsmith::triggers::Trigger::beginning_of_end_step(ironsmith::target::PlayerFilter::You),
            effects: ironsmith::resolution::ResolutionProgram::from_effects(vec![ironsmith::Effect::gain_life(7)]),
            one_shot: true, x_value: Some(3), not_before_turn: Some(turn),
            expires_at_turn: None, expires_before_controller_turn_after: None,
            expires_at_end_of_combat: false, bound_extra_turn_index: None,
            while_any_tagged_object_in_zone: None, target_objects: vec![],
            ability_source: None, ability_source_stable_id: None, ability_source_name: None,
            ability_source_snapshot: None, controller: alice, choices: vec![],
            tagged_objects: std::collections::HashMap::new(),
            tagged_players: std::collections::HashMap::new(), prepayment: None, prevention_shield: None,
        }
}

#[cfg(test)]
mod sync_checkpoint_tests {
    use super::*;

    #[test]
    fn public_audit_is_independent_of_local_definition_registration_order() {
        let _guard = crate::test_id_counter_guard();
        fn build(reverse: bool) -> (WasmGame, Vec<ObjectId>) {
            let mut wasm = WasmGame::new();
            wasm.initialize_empty_match(vec!["Alice".into(), "Bob".into()], 20, 1);
            let flying = ironsmith::static_abilities::StaticAbility::flying();
            let definitions: Vec<_> = ["Public graph A", "Public graph B"].into_iter()
                .enumerate().map(|(index, name)| {
                    let raw = if reverse { 9001 - index as u32 } else { 8000 + index as u32 };
                    ironsmith::cards::builders::CardDefinitionBuilder::new(CardId::from_raw(raw), name)
                        .card_types(vec![CardType::Creature])
                        .power_toughness(ironsmith::card::PowerToughness::fixed(1, 1))
                        .with_ability(ironsmith::Ability::static_ability(flying.clone()))
                        .build()
                }).collect();
            if reverse {
                wasm.registry.register(ironsmith::cards::builders::CardDefinitionBuilder::new(
                    CardId::from_raw(9900), "Private unobserved registration")
                    .card_types(vec![CardType::Sorcery]).with_spell_effect(vec![ironsmith::Effect::gain_life(7777)])
                    .build());
                for definition in definitions.iter().rev() { wasm.registry.register(definition.clone()); }
            } else {
                for definition in &definitions { wasm.registry.register(definition.clone()); }
            }
            let ids = definitions.iter().map(|definition| wasm.game.create_object_from_definition(
                definition, PlayerId::from_index(0), Zone::Battlefield)).collect();
            wasm.game.refresh_continuous_state().unwrap();
            (wasm, ids)
        }
        let (left, left_ids) = build(false);
        let (mut right, right_ids) = build(true);
        assert_eq!(left_ids, right_ids, "same public gameplay identities");
        assert_ne!(left.game.object(left_ids[0]).unwrap().card,
            right.game.object(right_ids[0]).unwrap().card);
        let public = serde_json::to_value(left.build_public_audit_checkpoint()).unwrap();
        assert_eq!(public, serde_json::to_value(right.build_public_audit_checkpoint()).unwrap());
        right.apply_sync_checkpoint(serde_json::from_value(
            serde_json::to_value(left.build_sync_checkpoint()).unwrap()).unwrap()).unwrap();
        assert_eq!(public, serde_json::to_value(right.build_public_audit_checkpoint()).unwrap());
        let static_id = |id| right.game.object(id).unwrap().abilities.iter().find_map(|ability| {
            match &ability.kind {
                ironsmith::AbilityKind::Static(value) => Some(value.instance_id()),
                _ => None,
            }
        }).unwrap();
        assert_eq!(static_id(right_ids[0]), static_id(right_ids[1]),
            "shared receiver occurrences must remain shared");
    }

    #[test]
    fn ninjutsu_destination_survives_stack_checkpoint_serialization() {
        for target in [
            AttackTarget::Player(PlayerId::from_index(2)),
            AttackTarget::Planeswalker(ObjectId::from_raw(50)),
            AttackTarget::Battle(ObjectId::from_raw(51)),
            AttackTarget::Nothing { defending_player: Some(PlayerId::from_index(1)), was_planeswalker: true },
        ] {
            let mut entry = StackEntry::new(ObjectId::from_raw(10), PlayerId::from_index(0));
            entry.is_ability = true;
            entry.ninjutsu_attack_target = Some(target.clone());
            let json = serde_json::to_value(sync_stack_entry(&entry)).unwrap();
            let stored: SyncStackEntry = serde_json::from_value(json.clone()).unwrap();
            assert_eq!(stack_entry_from_sync(&stored).ninjutsu_attack_target, Some(target));
            let mut legacy = json;
            legacy.as_object_mut().unwrap().remove("ninjutsuAttackTarget");
            let stored: SyncStackEntry = serde_json::from_value(legacy).unwrap();
            assert_eq!(stack_entry_from_sync(&stored).ninjutsu_attack_target, None);
        }
    }


    fn hidden_foretell_fixture(known: bool) -> (WasmGame, ObjectId, CardDefinition) {
        let mut wasm = WasmGame::new();
        wasm.initialize_empty_match(vec!["Alice".to_string(), "Bob".to_string()], 20, 1);
        let owner = PlayerId::from_index(0);
        wasm.game.turn.active_player = owner;
        wasm.game.turn.priority_player = Some(owner);
        wasm.game.turn.phase = Phase::FirstMain;
        wasm.game.turn.step = None;
        wasm.game.player_mut(owner).unwrap().mana_pool.add(ManaSymbol::Blue, 4);
        wasm.ensure_card_definitions_loaded(["Behold the Multiverse", "Lightning Bolt"]);
        let definition = wasm.find_card_definition("Behold the Multiverse").unwrap().clone();
        let id = wasm.game.create_hidden_card_placeholder(
            owner, Zone::Hand, 4, "ziffle:foretell:4".to_string(),
        );
        if known {
            wasm.game.reveal_hidden_card_with_definition(id, &definition).unwrap();
        }
        (wasm, id, definition)
    }

    fn perform_hidden_foretell(wasm: &mut WasmGame, id: ObjectId) -> ObjectId {
        let action = ironsmith::special_actions::SpecialAction::Foretell { card_id: id };
        ironsmith::special_actions::perform(action, &mut wasm.game, PlayerId::from_index(0),
            &mut ironsmith::decision::SelectFirstDecisionMaker).unwrap();
        *wasm.game.exile.last().unwrap()
    }

    #[test]
    fn foretell_placeholder_replay_checks_public_legality_and_defers_keyword_validation() {
        let _id_counter_guard = crate::test_id_counter_guard();
        let (mut wasm, id, definition) = hidden_foretell_fixture(false);
        let owner = PlayerId::from_index(0);
        let priority = ironsmith::decisions::context::PriorityContext::new(&wasm.game, owner, vec![LegalAction::PassPriority]).expect("fixture has complete replacement state");
        let action_ref = PriorityActionRef::SpecialAction { action: SpecialActionRef::Foretell { card_id: id.0 } };
        assert!(resolve_priority_action(&wasm.game, &priority, None, Some(&action_ref)).expect("fixture has complete replacement state").is_some());
        wasm.game.turn.active_player = PlayerId::from_index(1);
        assert!(resolve_priority_action(&wasm.game, &priority, None, Some(&action_ref)).expect("fixture has complete replacement state").is_none());
        wasm.game.turn.active_player = owner;
        let exiled = perform_hidden_foretell(&mut wasm, id);
        assert!(wasm.game.is_hidden_card_placeholder(exiled));
        assert!(wasm.game.is_face_down(exiled));
        assert!(wasm.game.is_foretold(exiled));
        assert!(wasm.game.has_hidden_identity_obligation(exiled));
        assert_eq!(wasm.game.player(owner).unwrap().mana_pool.total(), 2);
        let wrong = wasm.find_card_definition("Lightning Bolt").unwrap().clone();
        assert!(wasm.validate_hidden_normal_reveal(owner, exiled, &wrong).unwrap_err()
            .contains("Hidden identity obligation violated"));
        assert!(wasm.game.is_hidden_card_placeholder(exiled), "a rejected claim must not learn the false identity");
        assert!(wasm.game.end_of_match_disclosure_violation(exiled, &wrong).is_some());
        assert!(wasm.game.end_of_match_disclosure_cards(owner).iter().any(|card| card.object_id == exiled));
        wasm.validate_hidden_normal_reveal(owner, exiled, &definition).unwrap();
        wasm.game.reveal_hidden_card_with_definition(exiled, &definition).unwrap();
        // The shared claim ledger is never settled by an opening (openings
        // are not symmetric across peers); the claim stays until the card
        // leaves a public zone face up.
        assert!(wasm.game.has_hidden_identity_obligation(exiled));
        assert!(!wasm.game.foretold_card_is_castable(exiled));
        wasm.game.turn.turn_number += 1;
        assert!(ironsmith::decision::compute_legal_actions(&wasm.game, owner).expect("fixture has complete replacement state").iter().any(|action| matches!(action,
            LegalAction::CastSpell { spell_id, from_zone: Zone::Exile, .. } if *spell_id == exiled)));

        let (mut known, id, _) = hidden_foretell_fixture(false);
        known.game.reveal_hidden_card_with_definition(id, &wrong).unwrap();
        let action_ref = PriorityActionRef::SpecialAction { action: SpecialActionRef::Foretell { card_id: id.0 } };
        assert!(resolve_priority_action(&known.game, &priority, None, Some(&action_ref)).expect("fixture has complete replacement state").is_none(),
            "the placeholder path must never authorize a known non-foretell card");
    }

    #[test]
    fn foretell_live_dispatch_matches_known_and_placeholder_payment_boundaries() {
        let _id_counter_guard = crate::test_id_counter_guard();
        for known in [false, true] {
            let (mut wasm, id, _) = hidden_foretell_fixture(known);
            let owner = PlayerId::from_index(0);
            wasm.game.player_mut(owner).unwrap().mana_pool = Default::default();
            wasm.ensure_card_definitions_loaded(["Island"]);
            let island = wasm.find_card_definition("Island").unwrap().clone();
            for _ in 0..2 {
                wasm.game.create_object_from_definition(&island, owner, Zone::Battlefield);
            }
            wasm.runner = Some(ironsmith::turn_runner::TurnRunner::from_state_for_sync(
                ironsmith::turn_runner::TurnState::FirstMainPriority,
            ));
            wasm.runner_awaiting_priority = true;
            wasm.priority_state.restore_priority_tracker_for_sync(0, 2);
            let context = DecisionContext::Priority(ironsmith::decisions::context::PriorityContext::new(&wasm.game,
                owner, ironsmith::decision::compute_legal_actions(&wasm.game, owner).expect("fixture has complete replacement state"),
            ).expect("fixture has complete replacement state"));
            wasm.dispatch_live_priority_response(context, UiCommand::PriorityAction {
                action_index: None,
                action_ref: Some(PriorityActionRef::SpecialAction {
                    action: SpecialActionRef::Foretell { card_id: id.0 },
                }),
            }).unwrap();
            let Some(DecisionContext::ManaPayment(payment)) = wasm.pending_decision.as_ref() else {
                panic!("known={known}: foretell must request payment, got {:?}", wasm.pending_decision);
            };
            let command = UiCommand::ManaPayment { response: ManaPaymentCommand::Confirm {
                plan_id: payment.plan.id.to_string(), request_hash: payment.plan.request_hash.to_string(),
            } };
            let context = wasm.pending_decision.take().unwrap();
            if wasm.pending_live_continuation.is_some() {
                wasm.dispatch_live_priority_continuation(context, command).unwrap();
            } else {
                wasm.dispatch_live_priority_response(context, command).unwrap();
            }
            assert!(matches!(wasm.pending_decision, Some(DecisionContext::Priority(_))));
            let exiled = *wasm.game.exile.last().expect("paid foretell must exile the card");
            assert!(wasm.game.is_foretold(exiled));
            assert!(wasm.game.has_hidden_identity_obligation(exiled));
            assert_eq!(wasm.game.is_hidden_card_placeholder(exiled), !known);
        }
    }

    #[test]
    fn foretell_checkpoint_preserves_public_claim_and_reconstructs_legacy_foretold_cards() {
        let _id_counter_guard = crate::test_id_counter_guard();
        let (mut host, id, _) = hidden_foretell_fixture(true);
        let exiled = perform_hidden_foretell(&mut host, id);
        for legacy in [false, true] {
            let mut checkpoint = host.build_redacted_sync_checkpoint(PlayerId::from_index(1)).unwrap();
            assert!(checkpoint.rules.hidden_identity_obligations.iter().any(|claim| claim.check == "foretell"));
            assert!(checkpoint.objects.iter().find(|object| object.id == exiled.0).unwrap().original_card_name.is_none());
            let executable = checkpoint.executable_state.as_ref().expect("foretell physical root is retained");
            let opaque = executable.objects.iter().find(|object| object.id == exiled).unwrap();
            assert!(opaque.card.is_none());
            assert_eq!(opaque.name, "Hidden Card");
            assert!(executable.definitions.is_empty(), "foretell carrier cannot disclose the hidden definition");
            if legacy {
                checkpoint.rules.hidden_identity_obligations.clear();
                checkpoint.rules.hidden_claim_subjects.clear();
            }
            let mut guest = WasmGame::new();
            guest.apply_sync_checkpoint(checkpoint).unwrap();
            assert!(guest.game.is_hidden_card_placeholder(exiled));
            assert!(guest.game.has_hidden_identity_obligation(exiled));
            assert_eq!(guest.game.hidden_identity_obligations().len(), 1);
            guest.ensure_card_definitions_loaded(["Lightning Bolt"]);
            let wrong = guest.find_card_definition("Lightning Bolt").unwrap().clone();
            assert!(guest.validate_hidden_normal_reveal(PlayerId::from_index(0), exiled, &wrong).is_err());
            let cards = guest.game.end_of_match_disclosure_cards(PlayerId::from_index(0));
            assert!(cards.iter().any(|card| card.object_id == exiled));
            let second_checkpoint = guest.build_sync_checkpoint();
            guest.apply_sync_checkpoint(second_checkpoint).unwrap();
            assert_eq!(guest.game.hidden_identity_obligations().len(), 1, "restore must not duplicate the claim");
        }
    }

    #[test]
    fn foretell_claim_follows_a_return_to_library_for_end_match_disclosure() {
        let _id_counter_guard = crate::test_id_counter_guard();
        let (mut wasm, id, _) = hidden_foretell_fixture(false);
        let owner = PlayerId::from_index(0);
        let exiled = perform_hidden_foretell(&mut wasm, id);
        let library_id = wasm.game.move_object(exiled, Zone::Library,
            ironsmith::events::cause::EventCause::from_special_action(Some(exiled), owner)).unwrap();
        let disclosure = wasm.game.end_of_match_disclosure_cards(owner);
        let anchored = disclosure.iter().find(|card| card.object_id == library_id).unwrap();
        assert!(anchored.library_anchor.is_some());
        let wrong = wasm.find_card_definition("Lightning Bolt").unwrap().clone();
        assert!(wasm.game.end_of_match_disclosure_card_violation(anchored, &wrong).is_some());
    }

    #[test]
    fn combat_checkpoint_preserves_blocked_status_after_the_last_blocker_leaves() {
        let attacker = ObjectId::from_raw(71);
        let combat = ironsmith::combat_state::CombatState {
            attackers: vec![ironsmith::combat_state::AttackerInfo {
                creature: attacker,
                target: AttackTarget::Player(PlayerId::from_index(1)),
            }],
            blocked_attackers: std::collections::HashSet::from([attacker]),
            ..Default::default()
        };
        let encoded = serde_json::to_value(sync_grand_melee_combat(&combat)).unwrap();
        let decoded: SyncGrandMeleeCombat = serde_json::from_value(encoded.clone()).unwrap();
        let restored = grand_melee_combat_from_sync(&decoded);
        assert!(ironsmith::combat_state::is_blocked(&restored, attacker));
        assert!(restored.blockers.is_empty());

        // Older checkpoints lack the persistent set but still carry blockers.
        let mut legacy = encoded;
        legacy.as_object_mut().unwrap().remove("blockedAttackers");
        legacy["blockers"] = serde_json::json!([[71, [72]]]);
        let decoded: SyncGrandMeleeCombat = serde_json::from_value(legacy).unwrap();
        assert!(ironsmith::combat_state::is_blocked(&grand_melee_combat_from_sync(&decoded), attacker));
    }

    #[test]
    fn normalized_shuffle_after_order_uses_live_order_when_remap_duplicates_ids() {
        let _id_counter_guard = crate::test_id_counter_guard();
        let alice = PlayerId::from_index(0);
        let mut before = CryptoAuditState::default();
        let mut after = CryptoAuditState::default();

        let stale_order = vec![
            ObjectId::from_raw(101),
            ObjectId::from_raw(102),
            ObjectId::from_raw(103),
            ObjectId::from_raw(104),
        ];
        let live_order = vec![
            ObjectId::from_raw(201),
            ObjectId::from_raw(202),
            ObjectId::from_raw(203),
            ObjectId::from_raw(204),
        ];
        before
            .stable_by_id
            .insert(stale_order[0], StableId::from_raw(1));
        before
            .stable_by_id
            .insert(stale_order[1], StableId::from_raw(1));
        before
            .stable_by_id
            .insert(stale_order[2], StableId::from_raw(3));
        before
            .stable_by_id
            .insert(stale_order[3], StableId::from_raw(4));
        after
            .id_by_stable
            .insert(StableId::from_raw(1), live_order[0]);
        after
            .id_by_stable
            .insert(StableId::from_raw(3), live_order[2]);
        after
            .id_by_stable
            .insert(StableId::from_raw(4), live_order[3]);
        after.libraries.insert(alice, live_order.clone());

        let normalized = normalized_after_shuffle_order(alice, &before, &after, &stale_order);

        assert_eq!(normalized, live_order);
        assert!(
            object_order_has_unique_ids(&normalized),
            "normalized post-shuffle order should not contain duplicate object ids"
        );
    }

    #[test]
    fn sync_checkpoint_restores_battlefield_state_for_guest_perspective() {
        let _id_counter_guard = crate::test_id_counter_guard();
        let mut host = WasmGame::new();
        host.initialize_empty_match(vec!["Alice".to_string(), "Bob".to_string()], 20, 1);
        let object_id = ObjectId::from_raw(
            host.add_card_to_zone(
                0,
                "Ornithopter".to_string(),
                "battlefield".to_string(),
                true,
            )
            .expect("host should add a battlefield card"),
        );
        host.game.tap(object_id);
        host.game
            .object_mut(object_id)
            .expect("host object should exist")
            .add_counters(ironsmith::object::CounterType::PlusOnePlusOne, 2);

        let checkpoint = host.build_sync_checkpoint();

        let mut guest = WasmGame::new();
        guest
            .apply_sync_checkpoint(checkpoint)
            .expect("guest checkpoint should import");
        guest
            .set_perspective(1)
            .expect("guest perspective should switch");

        assert_eq!(guest.perspective, PlayerId::from_index(1));
        assert_eq!(guest.game.battlefield.len(), 1);

        let restored_id = guest.game.battlefield[0];
        let restored = guest
            .game
            .object(restored_id)
            .expect("guest battlefield object should exist");
        assert_eq!(restored.id, object_id);
        assert_eq!(
            restored.stable_id,
            ironsmith::ids::StableId::from_raw(object_id.0)
        );
        assert_eq!(restored.name, "Ornithopter");
        assert_eq!(restored.owner, PlayerId::from_index(0));
        assert!(guest.game.is_tapped(restored_id));
        assert_eq!(
            restored
                .counters
                .get(&ironsmith::object::CounterType::PlusOnePlusOne)
                .copied()
                .unwrap_or(0),
            2
        );
    }

    #[test]
    fn sync_checkpoint_preserves_cavern_choice_and_restricted_mana_legality() {
        let _id_counter_guard = crate::test_id_counter_guard();
        use ironsmith::ability::{
            ManaUsageRestriction, ManaUsageSubtypeRequirement, RestrictedManaUnit,
        };
        let owner = PlayerId::from_index(0);
        for (chosen, with_trigger) in [
            (Subtype::Dwarf, false),
            (Subtype::Elf, false),
            (Subtype::Dwarf, true),
            (Subtype::Elf, true),
        ] {
            let mut host = WasmGame::new();
            host.initialize_empty_match(vec!["Alice".into(), "Bob".into()], 20, 1);
            host.game.turn.active_player = owner;
            host.game.turn.priority_player = Some(owner);
            host.game.turn.phase = Phase::FirstMain;
            host.game.turn.step = None;
            host.runner = Some(TurnRunner::from_state_for_sync(
                RunnerTurnState::FirstMainPriority,
            ));
            host.runner_awaiting_priority = true;
            let definitions: Vec<_> = [
                    ("Cavern of Souls", "Type: Land\nAs this land enters, choose a creature type.\n{T}: Add {C}.\n{T}: Add one mana of any color. Spend this mana only to cast a creature spell of the chosen type, and that spell can't be countered.", Zone::Battlefield),
                    ("Dáin's Company", "Mana cost: {R}{W}\nType: Creature — Dwarf Warrior\nPower/Toughness: 2/2", Zone::Hand),
                    ("Plains", "Type: Basic Land — Plains\n{T}: Add {W}.", Zone::Battlefield),
                    ("Mana trigger probe", "Type: Enchantment\nWhenever you tap a creature for mana, add {G}.", Zone::Battlefield),
                ].into_iter().filter(|(name, _, _)| with_trigger || *name != "Mana trigger probe").map(|(name, text, zone)| {
                    let definition = ironsmith_registry_test::compile_to_runtime_definition(name, text, false).unwrap();
                    let id = host.game.create_object_from_definition(&definition, owner, zone);
                    host.registry.register(definition.clone());
                    (definition, id)
                }).collect();
            let cavern = definitions[0].1;
            let spell = definitions[1].1;
            let can_cast = |wasm: &WasmGame| {
                ironsmith::decision::compute_actions_for_source(&wasm.game, owner, Some(spell)).unwrap().iter()
                        .any(|action| matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == spell))
            };
            host.game.set_chosen_creature_type(cavern, chosen);

            assert_eq!(can_cast(&host), chosen == Subtype::Dwarf);
            let mut peer = WasmGame::new();
            for (definition, _) in &definitions {
                peer.registry.register(definition.clone());
            }
            peer.apply_sync_checkpoint(host.build_sync_checkpoint())
                .unwrap();
            assert_eq!(peer.game.chosen_creature_type(cavern), Some(chosen));
            assert_eq!(
                can_cast(&peer),
                can_cast(&host),
                "untapped Cavern affordability must agree"
            );
            host.game.tap(cavern);
            host.game
                .player_mut(owner)
                .unwrap()
                .add_restricted_mana(RestrictedManaUnit {
                    symbol: ManaSymbol::Red,
                    source: cavern,
                    source_chosen_creature_type: Some(chosen),
                    restrictions: vec![ManaUsageRestriction::CastSpell {
                        card_types: vec![CardType::Creature],
                        subtype_requirement: Some(ManaUsageSubtypeRequirement::ChosenTypeOfSource),
                        restrict_to_matching_spell: true,
                        grant_uncounterable: true,
                        enters_with_counters: vec![],
                        granted_abilities: vec![],
                    }],
                });
            let checkpoint = host.build_sync_checkpoint();
            // Round-trip the wire representation too, rather than clone native state.
            let checkpoint = serde_json::from_value(serde_json::to_value(checkpoint).unwrap()).unwrap();
            peer.apply_sync_checkpoint(checkpoint).unwrap();
            assert_eq!(
                peer.game.player(owner).unwrap().restricted_mana,
                host.game.player(owner).unwrap().restricted_mana
            );
            assert_eq!(can_cast(&host), chosen == Subtype::Dwarf);
            assert_eq!(
                can_cast(&peer),
                can_cast(&host),
                "floated restricted mana must not become unrestricted"
            );
            assert_eq!(
                serde_json::to_value(peer.build_public_audit_checkpoint()).unwrap(),
                serde_json::to_value(host.build_public_audit_checkpoint()).unwrap()
            );
            peer.game.object_mut(spell).unwrap().zone = Zone::Stack;
            peer.game
                .player_mut(owner)
                .unwrap()
                .hand
                .retain(|id| *id != spell);
            peer.game
                .stack
                .push(ironsmith::game_state::StackEntry::new(spell, owner));
            peer.game
                .player_mut(owner)
                .unwrap()
                .mana_pool
                .add(ManaSymbol::White, 1);
            let cost = peer.game.object(spell).unwrap().mana_cost.clone().unwrap();
            assert_eq!(
                peer.game.try_pay_mana_cost_with_reason(
                    owner,
                    Some(spell),
                    &cost,
                    0,
                    ironsmith::costs::PaymentReason::CastSpell
                ),
                chosen == Subtype::Dwarf
            );
            if chosen == Subtype::Dwarf {
                assert_eq!(peer.game.player(owner).unwrap().mana_pool.total(), 0);
                assert!(peer.game.player(owner).unwrap().restricted_mana.is_empty());
            }
        }
    }

    #[test]
    fn sync_checkpoint_restores_in_progress_priority_pass_tracker() {
        let _id_counter_guard = crate::test_id_counter_guard();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let mut host = WasmGame::new();
        host.initialize_empty_match(vec!["Alice".to_string(), "Bob".to_string()], 20, 1);
        host.game.turn = TurnState {
            active_player: alice,
            priority_player: Some(bob),
            turn_number: 1,
            phase: Phase::FirstMain,
            step: None,
        };
        host.runner = Some(TurnRunner::from_state_for_sync(
            RunnerTurnState::FirstMainPriority,
        ));
        host.runner_awaiting_priority = true;
        host.runner_pending_decision = false;
        host.priority_state.restore_priority_tracker_for_sync(1, 2);

        let public_checkpoint = host.build_public_audit_checkpoint();
        assert_eq!(
            public_checkpoint
                .priority_runtime
                .consecutive_priority_passes,
            1
        );
        assert_eq!(
            public_checkpoint
                .priority_runtime
                .turn_runner_state
                .as_deref(),
            Some("first_main_priority")
        );

        let checkpoint = host.build_sync_checkpoint();
        let mut guest = WasmGame::new();
        guest
            .apply_sync_checkpoint(checkpoint)
            .expect("guest checkpoint should import");

        assert_eq!(guest.priority_state.priority_tracker_snapshot(), (1, 2));
        assert!(guest.runner_awaiting_priority);

        let pending = guest
            .pending_decision
            .take()
            .expect("guest should have a priority decision");
        let DecisionContext::Priority(priority) = &pending else {
            panic!("expected priority decision, got {pending:?}");
        };
        assert_eq!(priority.player, bob);
        let pass_index = priority
            .actions
            .iter()
            .position(|action| matches!(action, LegalAction::PassPriority))
            .expect("pass priority should be legal");

        guest
            .dispatch_live_priority_response(
                pending,
                UiCommand::PriorityAction {
                    action_index: Some(pass_index),
                    action_ref: None,
                },
            )
            .expect("restored pass should complete the priority window");

        assert_eq!(guest.game.turn.phase, Phase::Combat);
        assert_eq!(guest.game.turn.step, Some(Step::BeginCombat));
        assert_eq!(guest.game.turn.priority_player, Some(alice));
        assert_eq!(guest.priority_state.priority_tracker_snapshot(), (0, 2));
    }

    #[test]
    fn public_audit_checkpoint_redacts_hidden_zone_card_identities() {
        let _id_counter_guard = crate::test_id_counter_guard();
        let mut host = WasmGame::new();
        host.initialize_empty_match(vec!["Alice".to_string(), "Bob".to_string()], 20, 1);
        host.add_card_to_zone(
            0,
            "Ornithopter".to_string(),
            "battlefield".to_string(),
            true,
        )
        .expect("host should add a public battlefield card");
        host.add_card_to_zone(0, "Forest".to_string(), "graveyard".to_string(), true)
            .expect("host should add a public graveyard card");
        host.add_card_to_zone(1, "Lightning Bolt".to_string(), "library".to_string(), true)
            .expect("host should add a hidden library card");
        host.add_card_to_zone(1, "Counterspell".to_string(), "hand".to_string(), true)
            .expect("host should add a hidden hand card");

        let checkpoint = host.build_public_audit_checkpoint();
        let bob = checkpoint
            .players
            .iter()
            .find(|player| player.id == 1)
            .expect("Bob should be present");
        assert_eq!(bob.library_count, 1);
        assert_eq!(bob.hand_count, 1);

        let public_names = checkpoint
            .objects
            .iter()
            .filter_map(|object| {
                object
                    .identity
                    .as_ref()
                    .map(|identity| identity.name.as_str())
            })
            .collect::<Vec<_>>();
        assert!(public_names.contains(&"Ornithopter"));
        assert!(public_names.contains(&"Forest"));
        assert!(!public_names.contains(&"Lightning Bolt"));
        assert!(!public_names.contains(&"Counterspell"));

        assert!(
            checkpoint
                .hidden_zones
                .iter()
                .any(|zone| zone.owner == 1 && zone.zone == "library" && zone.count == 1)
        );
        assert!(
            checkpoint
                .hidden_zones
                .iter()
                .any(|zone| zone.owner == 1 && zone.zone == "hand" && zone.count == 1)
        );
    }

    #[test]
    fn public_audit_checkpoint_uses_stable_public_hidden_commitments() {
        let _id_counter_guard = crate::test_id_counter_guard();
        let mut game = WasmGame::new();
        game.initialize_empty_match(vec!["Alice".to_string(), "Bob".to_string()], 20, 1);
        let object_id = game.game.create_hidden_card_placeholder(
            PlayerId::from_index(0),
            Zone::Hand,
            3,
            "ziffle:deck-hash:3".to_string(),
        );
        let before = game
            .build_public_audit_checkpoint()
            .hidden_zones
            .into_iter()
            .find(|zone| zone.owner == 0 && zone.zone == "hand")
            .and_then(|zone| zone.commitment_root)
            .expect("hidden hand should have a public commitment root");

        game.game.set_hidden_card_info(
            object_id,
            HiddenCardInfo {
                owner: PlayerId::from_index(0),
                zone: Zone::Hand,
                slot: 42,
                commitment: "deck-slot-42".to_string(),
                origin_slot: None,
                origin_commitment: None,
                public_slot: Some(3),
                public_commitment: Some("ziffle:deck-hash:3".to_string()),
            },
        );
        let after = game
            .build_public_audit_checkpoint()
            .hidden_zones
            .into_iter()
            .find(|zone| zone.owner == 0 && zone.zone == "hand")
            .and_then(|zone| zone.commitment_root)
            .expect("hidden hand should keep a public commitment root");

        assert_eq!(after, before);
    }

    #[test]
    fn public_audit_hidden_zone_root_commits_known_cards_without_hidden_metadata() {
        let _id_counter_guard = crate::test_id_counter_guard();
        let alice = PlayerId::from_index(0);
        let mut game = WasmGame::new();
        game.initialize_empty_match(vec!["Alice".to_string(), "Bob".to_string()], 20, 1);
        let known_id = ObjectId::from_raw(
            game.add_card_to_zone(0, "Forest".to_string(), "hand".to_string(), true)
                .expect("known card should be added to hand"),
        );
        let hidden_id = game.game.create_hidden_card_placeholder(
            alice,
            Zone::Hand,
            9,
            "hidden-slot-9".to_string(),
        );
        assert!(
            game.game.hidden_card_info(known_id).is_none(),
            "manual known hand card should not be tracked as hidden"
        );
        assert!(game.game.hidden_card_info(hidden_id).is_some());

        let hand_root = |game: &WasmGame| {
            let checkpoint = game.build_public_audit_checkpoint();
            checkpoint
                .hidden_zones
                .into_iter()
                .find(|zone| zone.owner == alice.0 && zone.zone == "hand")
                .expect("hand hidden zone should be exported")
                .commitment_root
                .expect("mixed hand should still have a commitment root")
        };

        let original = hand_root(&game);
        game.game
            .player_mut(alice)
            .expect("Alice should exist")
            .hand
            .swap(0, 1);
        let reordered = hand_root(&game);
        assert_ne!(
            reordered, original,
            "root should commit to the order of known and hidden hand objects"
        );

        game.game
            .player_mut(alice)
            .expect("Alice should exist")
            .hand
            .swap(0, 1);
        game.game
            .object_mut(known_id)
            .expect("known hand object should exist")
            .name = "Island".to_string().into();
        let renamed = hand_root(&game);
        assert_ne!(
            renamed, original,
            "root should commit to known object identity when no hidden metadata is present"
        );
    }

    #[test]
    fn sync_checkpoint_retains_physical_card_identity_across_faces_and_copies() {
        let _id_counter_guard = crate::test_id_counter_guard();
        for (original_name, current_name, is_copy) in [
            ("Sink into Stupor", "Soporific Springs", false),
            ("Bonecrusher Giant", "Stomp", false),
            ("Phantasmal Image", "Grizzly Bears", true),
        ] {
            let mut host = WasmGame::new();
            host.initialize_empty_match(vec!["Alice".to_string(), "Bob".to_string()], 20, 1);
            host.ensure_card_definitions_loaded([original_name, current_name]);
            let original = host.find_card_definition(original_name).unwrap().clone();
            let current = host.find_card_definition(current_name).unwrap().clone();
            let owner = PlayerId::from_index(1);
            let id = host.game.create_hidden_card_placeholder(
                owner, Zone::Battlefield, 4, "ziffle:identity:4".to_string(),
            );
            host.game.reveal_hidden_card_with_definition(id, &original).unwrap();
            let object = host.game.object_mut(id).unwrap();
            if is_copy {
                let source = Object::from_card_definition(
                    ObjectId::from_raw(999_999), &current, owner, Zone::Battlefield,
                );
                object.capture_enters_as_copy_restore_state();
                object.copy_copiable_values_from(&source);
            } else {
                object.apply_definition_face(&current);
            }
            assert_eq!(object.card, Some(original.card.id));

            let checkpoint = host.build_sync_checkpoint();
            let exported = checkpoint.objects.iter().find(|object| object.id == id.0).unwrap();
            assert_eq!(exported.name, current_name);
            assert_eq!(exported.original_card_name.as_deref(), Some(original_name));
            let audit = host.capture_crypto_audit_state();
            assert_eq!(audit.hidden_by_id.get(&id).unwrap().card.as_deref(), Some(original_name));

            let mut guest = WasmGame::new();
            guest.apply_sync_checkpoint(checkpoint).unwrap();
            // Incoming executable definitions belong to this world's exact
            // graph nodes, not the trusted name-based session catalog.
            let physical = guest.game.object(id).unwrap().card.expect("physical CardId retained");
            let original = guest.game.retained_card_definition(physical)
                .expect("physical definition retained in imported world").clone();
            assert_eq!(original.name(), original_name, "copied or alternate face never replaces physical identity");
            assert!(guest.find_card_definition(original_name).is_none(), "world import does not enroll definitions in the session catalog");
            assert_eq!(guest.game.object(id).unwrap().card, Some(original.card.id));
            assert_eq!(guest.game.object(id).unwrap().name, current_name);
            let before = guest.game.hidden_card_info(id).unwrap().clone();
            guest.game.reveal_hidden_card_with_definition(id, &original).unwrap();
            assert_eq!(guest.game.object(id).unwrap().name, current_name);
            assert_eq!(guest.game.hidden_card_info(id).unwrap(), &before);
        }
    }

    #[test]
    fn sync_checkpoint_redacts_physical_card_identity_and_accepts_legacy_objects() {
        let _id_counter_guard = crate::test_id_counter_guard();
        let mut host = WasmGame::new();
        host.initialize_empty_match(vec!["Alice".to_string(), "Bob".to_string()], 20, 1);
        host.ensure_card_definitions_loaded(["Lightning Bolt"]);
        let definition = host.find_card_definition("Lightning Bolt").unwrap().clone();
        let owner = PlayerId::from_index(1);
        for (slot, zone) in [(0, Zone::Hand), (1, Zone::Library)] {
            let id = host.game.create_hidden_card_placeholder(
                owner, zone, slot, format!("private:{slot}"),
            );
            host.game.reveal_hidden_card_with_definition(id, &definition).unwrap();
        }
        let own = host.build_sync_checkpoint();
        assert!(own.objects.iter().all(|object| {
            object.original_card_name.as_deref() == Some("Lightning Bolt")
        }));
        let redacted = host.build_redacted_sync_checkpoint(PlayerId::from_index(0)).unwrap();
        assert!(redacted.objects.iter().all(|object| {
            object.name == "Hidden Card" && object.original_card_name.is_none()
        }));
        let mut guest = WasmGame::new();
        guest.apply_sync_checkpoint(redacted).unwrap();
        assert!(guest.game.objects_in_deterministic_order().iter().all(|object| object.card.is_none()));

        let mut legacy = serde_json::to_value(&own.objects[0]).unwrap();
        legacy.as_object_mut().unwrap().remove("originalCardName");
        let legacy: SyncObject = serde_json::from_value(legacy).unwrap();
        assert!(legacy.original_card_name.is_none());
        let restored = guest.sync_object_from_checkpoint(&legacy).unwrap();
        assert_eq!(restored.name, "Lightning Bolt");
        assert!(restored.card.is_some());
    }

    #[test]
    fn sync_checkpoint_includes_proposed_spell_origin_before_cast_costs_are_paid() {
        use ironsmith::alternative_cast::CastingMethod;
        use ironsmith::cost::OptionalCostsPaid;
        use ironsmith::game_loop::PendingCast;
        use ironsmith::provenance::ProvNodeId;

        let _id_counter_guard = crate::test_id_counter_guard();
        let mut game = WasmGame::new();
        game.initialize_empty_match(vec!["Alice".to_string(), "Bob".to_string()], 20, 1);
        let owner = PlayerId::from_index(0);
        let hand_id = game.game.create_hidden_card_placeholder(
            owner, Zone::Hand, 50, "ziffle:initial:50".to_string(),
        );
        let mut info = game.game.hidden_card_info(hand_id).unwrap().clone();
        info.slot = 22;
        info.commitment = "private-original-slot-22".to_string();
        info.public_slot = Some(60);
        info.public_commitment = Some("ziffle:mulligan:60".to_string());
        game.game.set_hidden_card_info(hand_id, info);
        game.ensure_card_definitions_loaded(["Goblin Guide"]);
        let definition = game.find_card_definition("Goblin Guide").unwrap().clone();
        game.game.reveal_hidden_card_with_definition(hand_id, &definition).unwrap();
        let stack_id = game.game.move_object_by_game_rule(hand_id, Zone::Stack).unwrap();
        assert!(game.game.stack.is_empty(), "a proposed spell is not yet a completed stack entry");
        game.priority_state.pending_cast = Some(PendingCast::new(
            hand_id, Zone::Hand, owner, ProvNodeId::default(), CastStage::PayingMana,
            None, Vec::new(), CastingMethod::Normal, OptionalCostsPaid::new(0), None, stack_id,
        ));

        let checkpoint = game.build_sync_checkpoint();
        let proposed = checkpoint.objects.iter().find(|object| object.id == stack_id.0)
            .expect("pending cast must remain in the checkpoint");
        assert_eq!(proposed.name, "Goblin Guide");
        let hidden = proposed.hidden_card.as_ref().unwrap();
        assert_eq!(hidden.origin_slot, Some(50));
        assert_eq!(hidden.origin_commitment.as_deref(), Some("ziffle:initial:50"));
        assert_eq!(hidden.public_slot, Some(60));
        assert!(!checkpoint.objects.iter().any(|object| object.id == hand_id.0));
        assert_eq!(game.sync_checkpoint_object_ids().iter().filter(|id| **id == stack_id).count(), 1);
    }

    #[test]
    fn sync_checkpoint_keeps_resolving_spell_origin_while_a_choice_is_pending() {
        use ironsmith::decisions::context::{SelectOptionsContext, SelectableOption};

        let _id_counter_guard = crate::test_id_counter_guard();
        let mut game = WasmGame::new();
        game.initialize_empty_match(vec!["Alice".to_string(), "Bob".to_string()], 20, 1);
        let owner = PlayerId::from_index(1);
        let hand_id = game.game.create_hidden_card_placeholder(
            owner,
            Zone::Hand,
            53,
            "ziffle:initial:53".to_string(),
        );
        let mut info = game.game.hidden_card_info(hand_id).unwrap().clone();
        info.slot = 57;
        info.commitment = "private-original-slot-57".to_string();
        info.public_slot = Some(12);
        info.public_commitment = Some("ziffle:current:12".to_string());
        game.game.set_hidden_card_info(hand_id, info);
        game.ensure_card_definitions_loaded(["Wrath of the Skies"]);
        let definition = game.find_card_definition("Wrath of the Skies").unwrap().clone();
        game.game.reveal_hidden_card_with_definition(hand_id, &definition).unwrap();
        let stack_id = game.game.move_object_by_game_rule(hand_id, Zone::Stack).unwrap();
        game.game.push_to_stack(StackEntry::new(stack_id, owner));
        assert_eq!(
            game.sync_checkpoint_object_ids().iter().filter(|id| **id == stack_id).count(),
            1,
            "a spell with a stack entry must be included exactly once",
        );

        // Resolution pops the entry before executing effects. An interactive
        // choice suspends those effects while the physical spell stays in Stack.
        assert_eq!(game.game.pop_from_stack().unwrap().object_id, stack_id);
        game.pending_decision = Some(DecisionContext::SelectOptions(
            SelectOptionsContext::new(
                PlayerId::from_index(0),
                Some(stack_id),
                "Order cards put into your graveyard simultaneously",
                vec![
                    SelectableOption::new(0, "First card"),
                    SelectableOption::new(1, "Second card"),
                ],
                2,
                2,
            ),
        ));
        assert!(game.game.stack.is_empty());
        assert!(game.priority_state.pending_cast.is_none());
        assert!(game.active_resolving_stack_object.is_none(), "the export must not depend on a UI snapshot");
        assert_eq!(game.game.object(stack_id).unwrap().zone, Zone::Stack);

        let checkpoint = game.build_sync_checkpoint();
        let resolving = checkpoint.objects.iter().find(|object| object.id == stack_id.0)
            .expect("a resolving spell must remain available to authenticate its opening");
        assert_eq!(resolving.name, "Wrath of the Skies");
        assert_eq!(resolving.zone, "stack");
        let hidden = resolving.hidden_card.as_ref().unwrap();
        assert_eq!(hidden.owner, 1);
        assert_eq!(hidden.slot, 57);
        assert_eq!(hidden.commitment, "private-original-slot-57");
        assert_eq!(hidden.origin_slot, Some(53));
        assert_eq!(hidden.origin_commitment.as_deref(), Some("ziffle:initial:53"));
        assert_eq!(hidden.public_slot, Some(12));
        assert_eq!(hidden.public_commitment.as_deref(), Some("ziffle:current:12"));
        assert_eq!(checkpoint.objects.iter().filter(|object| object.id == stack_id.0).count(), 1);
        assert!(!checkpoint.objects.iter().any(|object| object.id == hand_id.0));
    }

    #[test]
    fn verified_library_epoch_hydrates_nexus_of_fate_before_its_mill_destination_replacement() {
        use ironsmith::effects::EffectExecutor;
        let _id_counter_guard = crate::test_id_counter_guard();
        for mill_count in [1, 2] {
            let mut wasm = WasmGame::new();
            wasm.initialize_empty_match(vec!["Alice".into(), "Bob".into()], 20, 1);
            let owner = PlayerId::from_index(0);
            for slot in 0..3 {
                wasm.game.create_hidden_card_placeholder(owner, Zone::Library, slot, format!("ziffle:original:{slot}"));
            }
            wasm.queue_verified_hidden_library_epoch_input(VerifiedHiddenLibraryEpochInput {
                owner: 0, deck_hash: "nexus-first".into(), count: 3, random_count_before: Some(0),
                expected_inputs: Some((0..3).map(|slot| format!("ziffle:original:{slot}")).collect()),
            }).unwrap();
            for (position, name) in [(2, "Nexus of Fate"), (1, "Mountain")] {
                wasm.queue_verified_hidden_library_opening_input(VerifiedHiddenLibraryOpeningInput {
                    owner: 0, deck_hash: "nexus-first".into(), position, card_name: name.into(),
                    original_slot: Some(position + 10), commitment: Some(format!("manifest:{}", position + 10)),
                }).unwrap();
            }
            wasm.game.shuffle_player_library(owner);
            let top = *wasm.game.player(owner).unwrap().library.last().unwrap();
            assert_eq!(wasm.game.object(top).unwrap().name, "Nexus of Fate");
            let source = ObjectId::from_raw(900_000);
            let mut context = ironsmith::effects::EffectContext::new_default(source, owner);
            ironsmith::effects::MillEffect::you(mill_count).execute(&mut wasm.game, &mut context).unwrap();
            assert!(wasm.game.verified_hidden_library_epoch_error().is_none());
            assert_eq!(wasm.game.player(owner).unwrap().graveyard.len(), (mill_count - 1) as usize,
                "the simultaneous mill must still move every non-replaced selected card (mill {mill_count})");
            assert_eq!(wasm.game.player(owner).unwrap().library.len(), (4 - mill_count) as usize);
            assert!(wasm.game.player(owner).unwrap().library.iter().any(|id|
                wasm.game.object(*id).unwrap().name == "Nexus of Fate"),
                "the verified identity must participate in intrinsic replacement processing");
            // This regression checks hydration and destination replacement.
            // The existing static ability currently omits the subsequent shuffle
            // outside spell resolution; that separate engine gap is not a proof failure.
        }
    }

    #[test]
    fn verified_library_epoch_shuffle_then_draw_or_mill_keeps_crypto_opening_requirements() {
        let _id_counter_guard = crate::test_id_counter_guard();
        for destination in [Zone::Hand, Zone::Graveyard] {
            let mut wasm = WasmGame::new();
            wasm.initialize_empty_match(vec!["Alice".into(), "Bob".into()], 20, 1);
            let owner = PlayerId::from_index(0);
            for slot in 0..3 {
                wasm.game.create_hidden_card_placeholder(owner, Zone::Library, slot, format!("ziffle:before:{slot}"));
            }
            wasm.queue_verified_hidden_library_epoch_input(VerifiedHiddenLibraryEpochInput {
                owner: 0, deck_hash: "after".into(), count: 3, random_count_before: Some(0),
                expected_inputs: Some((0..3).map(|slot| format!("ziffle:before:{slot}")).collect()),
            }).unwrap();
            let before = wasm.capture_crypto_audit_state();
            wasm.game.shuffle_player_library(owner);
            let shuffled = wasm.game.player(owner).unwrap().library.to_vec();
            let moved = wasm.game.move_object_by_game_rule(shuffled[2], destination).unwrap();
            wasm.update_crypto_requirements_from(before);
            let shuffle = wasm.last_crypto_requirements.iter()
                .find(|requirement| requirement.requirement_type == "verifiable_shuffle").unwrap();
            assert_eq!(shuffle.input_commitments.as_ref().unwrap(), &vec![
                "ziffle:before:0".to_string(), "ziffle:before:1".to_string(), "ziffle:before:2".to_string(),
            ]);
            let expected_type = if destination == Zone::Hand { "private_open" } else { "public_open" };
            let opening = wasm.last_crypto_requirements.iter().find(|requirement| requirement.requirement_type == expected_type)
                .expect("a post-shuffle move must retain its authenticated opening requirement");
            assert_eq!(opening.object_id, Some(moved.0));
            assert_eq!(opening.public_commitment.as_deref(), Some("ziffle:after:2"));
            assert_eq!(opening.origin_commitment.as_deref(), Some("ziffle:after:2"));
        }
    }

    #[test]
    fn verified_library_epoch_checkpoint_contains_only_fresh_anonymous_objects() {
        let _id_counter_guard = crate::test_id_counter_guard();
        let mut host = WasmGame::new();
        host.initialize_empty_match(vec!["Alice".into(), "Bob".into()], 20, 1);
        let owner = PlayerId::from_index(0);
        host.ensure_card_definitions_loaded(["Mountain"]);
        let mountain = host.find_card_definition("Mountain").unwrap().clone();
        let old = (0..3).map(|slot| host.game.create_hidden_card_placeholder(
            owner, Zone::Library, slot, format!("ziffle:retired:{slot}"),
        )).collect::<Vec<_>>();
        host.game.reveal_hidden_card_with_definition(old[0], &mountain).unwrap();
        host.queue_verified_hidden_library_epoch_input(VerifiedHiddenLibraryEpochInput {
            owner: 0, deck_hash: "anonymous".into(), count: 3, random_count_before: Some(0),
            expected_inputs: Some((0..3).map(|slot| format!("ziffle:retired:{slot}")).collect()),
        }).unwrap();
        host.game.shuffle_player_library(owner);
        assert!(host.game.verified_hidden_library_epoch_error().is_none());
        let checkpoint = host.build_redacted_sync_checkpoint(PlayerId::from_index(1)).unwrap();
        assert_eq!(checkpoint.objects.len(), 3);
        for object in &checkpoint.objects {
            assert!(!old.iter().any(|id| id.0 == object.id));
            assert_eq!(object.name, "Hidden Card");
            assert!(object.original_card_name.is_none());
            let hidden = object.hidden_card.as_ref().unwrap();
            assert!(hidden.commitment.starts_with("ziffle:anonymous:"));
            assert_eq!(hidden.origin_commitment.as_deref(), Some(hidden.commitment.as_str()));
            assert_eq!(hidden.public_commitment.as_deref(), Some(hidden.commitment.as_str()));
        }
        let encoded = serde_json::to_string(&checkpoint.objects).unwrap();
        assert!(!encoded.contains("retired"));
        assert!(!encoded.contains("Mountain"));
        let mut guest = WasmGame::new();
        guest.apply_sync_checkpoint(checkpoint).unwrap();
        assert_eq!(guest.game.player(owner).unwrap().library, host.game.player(owner).unwrap().library);
        for id in old {
            assert!(guest.game.object(id).is_none());
            assert!(guest.game.current_object_id_after_zone_change(id).is_none());
        }
        let drawn = guest.game.draw_cards(owner, 1)[0];
        assert_eq!(guest.game.hidden_card_info(drawn).unwrap().origin_commitment.as_deref(), Some("ziffle:anonymous:2"));
        guest.game.reveal_hidden_card_with_definition(drawn, &mountain).unwrap();
        assert_eq!(guest.game.object(drawn).unwrap().name, "Mountain");
    }

    #[test]
    fn verified_library_epoch_preserves_anchored_claim_without_linking_new_positions() {
        let _id_counter_guard = crate::test_id_counter_guard();
        let (mut host, id, _) = hidden_foretell_fixture(false);
        let owner = PlayerId::from_index(0);
        let exiled = perform_hidden_foretell(&mut host, id);
        let returned = host.game.move_object_by_game_rule(exiled, Zone::Library).unwrap();
        host.queue_verified_hidden_library_epoch_input(VerifiedHiddenLibraryEpochInput {
            owner: 0, deck_hash: "unlinked".into(), count: 1, random_count_before: Some(0),
            expected_inputs: Some(vec!["ziffle:foretell:4".into()]),
        }).unwrap();
        host.game.shuffle_player_library(owner);
        assert!(host.game.verified_hidden_library_epoch_error().is_none());
        let anonymous = host.game.player(owner).unwrap().library[0];
        assert_ne!(anonymous, returned);
        assert!(!host.game.has_hidden_identity_obligation(anonymous));
        let checkpoint = host.build_redacted_sync_checkpoint(PlayerId::from_index(1)).unwrap();
        let mut guest = WasmGame::new();
        guest.apply_sync_checkpoint(checkpoint).unwrap();
        guest.ensure_card_definitions_loaded(["Lightning Bolt"]);
        let wrong = guest.find_card_definition("Lightning Bolt").unwrap().clone();
        let disclosure = guest.game.end_of_match_disclosure_cards(owner);
        let claim = disclosure.iter().find(|card| card.library_anchor.is_some()).unwrap();
        assert!(claim.anchor_only);
        assert_eq!(claim.object_id, returned);
        assert_eq!(claim.info.commitment, "ziffle:foretell:4");
        assert!(guest.game.end_of_match_disclosure_card_violation(claim, &wrong).is_some());
        assert_eq!(guest.game.hidden_card_info(anonymous).unwrap().commitment, "ziffle:unlinked:0");
    }

    #[test]
    fn hidden_card_origin_survives_hydration_zone_changes_reseal_and_checkpoint() {
        let _id_counter_guard = crate::test_id_counter_guard();
        let mut game = WasmGame::new();
        game.initialize_empty_match(vec!["Alice".to_string(), "Bob".to_string()], 20, 1);
        let owner = PlayerId::from_index(1);
        let library_id = game.game.create_hidden_card_placeholder(
            owner, Zone::Library, 51, "ziffle:initial:51".to_string(),
        );
        let stable_id = game.game.object(library_id).unwrap().stable_id;
        let mut hydrated = game.game.hidden_card_info(library_id).unwrap().clone();
        hydrated.slot = 4;
        hydrated.commitment = "private-original-slot-4".to_string();
        hydrated.public_slot = Some(2);
        hydrated.public_commitment = Some("ziffle:later:2".to_string());
        hydrated.origin_slot = Some(999);
        hydrated.origin_commitment = Some("attempted-overwrite".to_string());
        game.game.set_hidden_card_info(library_id, hydrated);
        game.ensure_card_definitions_loaded(["Mountain"]);
        let definition = game.find_card_definition("Mountain").unwrap().clone();
        game.game.reveal_hidden_card_with_definition(library_id, &definition).unwrap();
        let hand_id = game.game.draw_cards(owner, 1)[0];
        let field_id = game.game.move_object_by_game_rule(hand_id, Zone::Battlefield).unwrap();
        assert_ne!(field_id, hand_id);
        assert!(game.game.object(hand_id).is_none());
        assert_eq!(game.game.object(field_id).unwrap().stable_id, stable_id);
        let exported = game.hidden_card_opening_export(field_id).unwrap();
        assert_eq!(exported.slot, 4);
        assert_eq!(exported.origin_slot, Some(51));
        assert_eq!(exported.origin_commitment.as_deref(), Some("ziffle:initial:51"));
        let library_id = game.game.move_object_by_game_rule(field_id, Zone::Library).unwrap();
        game.reseal_verified_hidden_library_shuffle(ApplyHiddenLibraryShuffleInput {
            owner: 1, deck_hash: "newest".to_string(), after_order: vec![library_id.0], enforce_library_order: None,
        }).unwrap();
        let redacted = game.build_redacted_sync_checkpoint(PlayerId::from_index(0)).unwrap();
        let hidden = redacted.objects.iter().find(|object| object.id == library_id.0).unwrap().hidden_card.as_ref().unwrap();
        assert_eq!(hidden.public_slot, Some(0));
        assert_eq!(hidden.origin_slot, Some(51));
        assert_eq!(hidden.origin_commitment.as_deref(), Some("ziffle:initial:51"));
        let checkpoint = game.build_sync_checkpoint();
        let mut restored = WasmGame::new();
        restored.apply_sync_checkpoint(checkpoint).unwrap();
        let info = restored.game.hidden_card_info(library_id).unwrap();
        assert_eq!(info.origin_slot, Some(51));
        assert_eq!(info.origin_commitment.as_deref(), Some("ziffle:initial:51"));
        assert_eq!(info.public_commitment.as_deref(), Some("ziffle:newest:0"));
        let requirement = CryptoRequirementView::hidden_open("public_open", &HiddenAuditCard {
            object_id: library_id, owner, zone: Zone::Library, slot: info.slot, commitment: info.commitment.clone(),
            origin_slot: info.origin_slot, origin_commitment: info.origin_commitment.clone(),
            public_slot: info.public_slot, public_commitment: info.public_commitment.clone(),
            card: None, face_down: false, foretold: false,
        }, None, "public", "origin regression");
        let value = serde_json::to_value(requirement).unwrap();
        assert_eq!(value["originSlot"], 51);
        assert_eq!(value["originCommitment"], "ziffle:initial:51");
    }

    #[test]
    fn hidden_card_placeholder_moves_and_reveals_in_place() {
        let _id_counter_guard = crate::test_id_counter_guard();
        let mut game = WasmGame::new();
        game.initialize_empty_match(vec!["Alice".to_string(), "Bob".to_string()], 20, 1);
        let hidden_id = game.game.create_hidden_card_placeholder(
            PlayerId::from_index(1),
            Zone::Library,
            0,
            "commitment-0".to_string(),
        );
        assert!(game.game.is_hidden_card_placeholder(hidden_id));

        let drawn = game.game.draw_cards(PlayerId::from_index(1), 1);
        assert_eq!(drawn.len(), 1);
        let hand_id = drawn[0];
        assert!(game.game.is_hidden_card_placeholder(hand_id));
        assert_eq!(
            game.game
                .hidden_card_info(hand_id)
                .expect("hidden metadata follows zone changes")
                .slot,
            0
        );

        game.ensure_card_definitions_loaded(["Lightning Bolt"]);
        let definition = game
            .find_card_definition("Lightning Bolt")
            .expect("fixture card should load")
            .clone();
        game.game
            .reveal_hidden_card_with_definition(hand_id, &definition)
            .expect("hidden card should reveal");
        assert!(!game.game.is_hidden_card_placeholder(hand_id));
        assert_eq!(
            game.game
                .hidden_card_info(hand_id)
                .expect("commitment metadata remains after private reveal")
                .commitment,
            "commitment-0"
        );
        assert_eq!(
            game.game
                .object(hand_id)
                .expect("revealed object should exist")
                .name,
            "Lightning Bolt"
        );
    }

    #[test]
    fn mulligan_shuffle_requirement_reseals_drawn_hidden_hand_cards() {
        let _id_counter_guard = crate::test_id_counter_guard();
        let mut game = WasmGame::new();
        game.initialize_empty_match(vec!["Alice".to_string(), "Bob".to_string()], 20, 1);
        let bob = PlayerId::from_index(1);
        for slot in 0..10 {
            game.game.create_hidden_card_placeholder(
                bob,
                Zone::Library,
                slot,
                format!("bob-slot-{slot}"),
            );
        }
        assert_eq!(game.game.draw_cards(bob, 2).len(), 2);

        let before = game.capture_crypto_audit_state();
        let hand_ids = game
            .game
            .player(bob)
            .expect("Bob should still exist")
            .hand
            .clone();
        for id in hand_ids {
            let _ = game.game.move_object_by_effect(id, Zone::Library);
        }
        game.game.shuffle_player_library(bob);
        assert_eq!(game.game.draw_cards(bob, 2).len(), 2);
        game.update_crypto_requirements_from(before);

        let requirement = game
            .last_crypto_requirements
            .iter()
            .find(|requirement| {
                requirement.requirement_type == "verifiable_shuffle" && requirement.owner == 1
            })
            .expect("mulligan redraw should require a verifiable shuffle");
        let before_order = requirement
            .before_order
            .as_ref()
            .expect("shuffle requirement should include before order");
        let after_order = requirement
            .after_order
            .as_ref()
            .expect("shuffle requirement should include after order")
            .clone();
        assert_eq!(before_order.len(), 10);
        assert_eq!(after_order.len(), 10);
        assert_eq!(
            requirement.count,
            Some(8),
            "shuffle requirement count tracks the post-shuffle library prefix"
        );

        let bob_hand = game
            .game
            .player(bob)
            .expect("Bob should still exist")
            .hand
            .clone();
        assert_eq!(bob_hand.len(), 2);
        assert!(
            bob_hand
                .iter()
                .all(|id| after_order.contains(&id.0)),
            "drawn hand cards must be part of the post-shuffle ziffle order"
        );

        game.reseal_verified_hidden_library_shuffle(ApplyHiddenLibraryShuffleInput {
            owner: 1,
            deck_hash: "mulligan-deck".to_string(),
            after_order: after_order.clone(),
            enforce_library_order: None,
        })
        .expect("verified shuffle should reseal library and drawn hand cards");

        for (position, raw_id) in after_order.iter().copied().enumerate() {
            let info = game
                .game
                .hidden_card_info(ObjectId::from_raw(raw_id))
                .expect("all shuffled hidden cards should still have metadata");
            assert_eq!(info.owner, bob);
            assert_eq!(
                info.public_slot,
                Some(position as u16),
                "reseal should publish the post-shuffle public position without replacing private identity"
            );
            assert_eq!(
                info.public_commitment.as_deref(),
                Some(format!("ziffle:mulligan-deck:{position}").as_str())
            );
        }
    }

    #[test]
    fn verified_shuffle_reseal_accepts_pre_draw_after_order_ids() {
        let _id_counter_guard = crate::test_id_counter_guard();
        let mut game = WasmGame::new();
        game.initialize_empty_match(vec!["Alice".to_string(), "Bob".to_string()], 20, 1);
        let bob = PlayerId::from_index(1);
        for slot in 0..10 {
            game.game.create_hidden_card_placeholder(
                bob,
                Zone::Library,
                slot,
                format!("bob-slot-{slot}"),
            );
        }
        let initial_hand = game.game.draw_cards(bob, 2);
        assert_eq!(initial_hand.len(), 2);

        game.ensure_card_definitions_loaded(["Swamp"]);
        let definition = game
            .find_card_definition("Swamp")
            .expect("fixture card should load")
            .clone();
        for hand_id in &initial_hand {
            game.game
                .reveal_hidden_card_with_definition(*hand_id, &definition)
                .expect("drawn hand card should reveal");
        }

        for id in initial_hand {
            let _ = game.game.move_object_by_effect(id, Zone::Library);
        }
        game.game.shuffle_player_library(bob);
        let pre_draw_after_order = game
            .game
            .player(bob)
            .expect("Bob should still exist")
            .library
            .clone();
        assert_eq!(game.game.draw_cards(bob, 2).len(), 2);

        game.reseal_verified_hidden_library_shuffle(ApplyHiddenLibraryShuffleInput {
            owner: 1,
            deck_hash: "mulligan-deck".to_string(),
            after_order: pre_draw_after_order.iter().map(|id| id.0).collect(),
            enforce_library_order: None,
        })
        .expect("verified shuffle should resolve pre-draw ids through zone-change results");

        for (position, stale_id) in pre_draw_after_order.iter().copied().enumerate() {
            let current_id = game
                .game
                .current_object_id_after_zone_change(stale_id)
                .expect("stale shuffle id should resolve to a live object");
            let info = game
                .game
                .hidden_card_info(current_id)
                .expect("all shuffled hidden cards should still have metadata");
            assert_eq!(info.owner, bob);
            assert!(
                game.game.is_hidden_card_placeholder(current_id),
                "resealing a hidden-library shuffle should redact cards in hidden zones"
            );
            assert_eq!(info.public_slot, Some(position as u16));
            assert_eq!(
                info.public_commitment.as_deref(),
                Some(format!("ziffle:mulligan-deck:{position}").as_str())
            );
        }

        let second_hand = game
            .game
            .player(bob)
            .expect("Bob should still exist")
            .hand
            .clone();
        assert_eq!(second_hand.len(), 2);
        for id in second_hand {
            let _ = game.game.move_object_by_effect(id, Zone::Library);
        }
        game.game.shuffle_player_library(bob);
        let second_pre_draw_after_order = game
            .game
            .player(bob)
            .expect("Bob should still exist")
            .library
            .clone();
        assert_eq!(game.game.draw_cards(bob, 2).len(), 2);

        game.reseal_verified_hidden_library_shuffle(ApplyHiddenLibraryShuffleInput {
            owner: 1,
            deck_hash: "second-mulligan-deck".to_string(),
            after_order: second_pre_draw_after_order.iter().map(|id| id.0).collect(),
            enforce_library_order: None,
        })
        .expect("verified shuffle should follow multi-zone-change id chains");

        for stale_id in second_pre_draw_after_order {
            let current_id = game
                .game
                .current_object_id_after_zone_change(stale_id)
                .expect("multi-hop stale shuffle id should resolve to a live object");
            let info = game
                .game
                .hidden_card_info(current_id)
                .expect("all reshuffled hidden cards should still have metadata");
            assert_eq!(info.owner, bob);
        }
    }

    #[test]
    fn verified_shuffle_reseal_reorders_current_library_to_public_order() {
        let _id_counter_guard = crate::test_id_counter_guard();
        let mut game = WasmGame::new();
        game.initialize_empty_match(vec!["Alice".to_string(), "Bob".to_string()], 20, 1);
        let bob = PlayerId::from_index(1);
        for slot in 0..6 {
            game.game.create_hidden_card_placeholder(
                bob,
                Zone::Library,
                slot,
                format!("bob-slot-{slot}"),
            );
        }

        game.game.shuffle_player_library(bob);
        let verified_full_order = game
            .game
            .player(bob)
            .expect("Bob should still exist")
            .library
            .clone();
        assert_eq!(game.game.draw_cards(bob, 2).len(), 2);
        let current_library_set = game
            .game
            .player(bob)
            .expect("Bob should still exist")
            .library
            .iter()
            .copied()
            .collect::<HashSet<_>>();
        let expected_library = verified_full_order
            .iter()
            .copied()
            .filter(|object_id| current_library_set.contains(object_id))
            .collect::<Vec<_>>();

        game.game
            .player_mut(bob)
            .expect("Bob should still exist")
            .library
            .reverse();
        assert_ne!(
            game.game
                .player(bob)
                .expect("Bob should still exist")
                .library,
            expected_library,
            "test setup should perturb the local engine order"
        );

        game.reseal_verified_hidden_library_shuffle(ApplyHiddenLibraryShuffleInput {
            owner: 1,
            deck_hash: "verified-deck".to_string(),
            after_order: verified_full_order.iter().map(|id| id.0).collect(),
            enforce_library_order: None,
        })
        .expect("verified shuffle should impose the authenticated public order");

        assert_eq!(
            game.game
                .player(bob)
                .expect("Bob should still exist")
                .library,
            expected_library,
            "verified ziffle order must become the engine's top-of-library order"
        );
        for (position, stale_id) in verified_full_order.iter().copied().enumerate() {
            let current_id = game
                .game
                .current_object_id_after_zone_change(stale_id)
                .unwrap_or(stale_id);
            let info = game
                .game
                .hidden_card_info(current_id)
                .expect("all verified hidden cards should still have metadata");
            assert_eq!(info.public_slot, Some(position as u16));
            assert_eq!(
                info.public_commitment.as_deref(),
                Some(format!("ziffle:verified-deck:{position}").as_str())
            );
        }
    }

    #[test]
    fn repeated_mulligan_shuffle_requirements_keep_unique_after_orders() {
        let _id_counter_guard = crate::test_id_counter_guard();
        let mut game = WasmGame::new();
        game.initialize_empty_match(vec!["Alice".to_string(), "Bob".to_string()], 20, 1);
        let alice = PlayerId::from_index(0);
        for slot in 0..60 {
            game.game.create_hidden_card_placeholder(
                alice,
                Zone::Library,
                slot,
                format!("alice-slot-{slot}"),
            );
        }
        game.ensure_card_definitions_loaded(["Swamp"]);
        let definition = game
            .find_card_definition("Swamp")
            .expect("fixture card should load")
            .clone();

        let mut hand = game.game.draw_cards(alice, 7);
        for hand_id in &hand {
            game.game
                .reveal_hidden_card_with_definition(*hand_id, &definition)
                .expect("drawn hand card should reveal");
        }

        for mulligan_index in 0..4 {
            let before = game.capture_crypto_audit_state();
            for id in hand.drain(..) {
                let _ = game.game.move_object_by_effect(id, Zone::Library);
            }
            game.game.shuffle_player_library(alice);
            hand = game.game.draw_cards(alice, 7);
            for hand_id in &hand {
                if game.game.is_hidden_card_placeholder(*hand_id) {
                    game.game
                        .reveal_hidden_card_with_definition(*hand_id, &definition)
                        .expect("drawn hand card should reveal");
                }
            }
            game.update_crypto_requirements_from(before);

            let requirement = game
                .last_crypto_requirements
                .iter()
                .find(|requirement| {
                    requirement.requirement_type == "verifiable_shuffle" && requirement.owner == 0
                })
                .expect("mulligan redraw should require a verifiable shuffle");
            let after_order = requirement
                .after_order
                .as_ref()
                .expect("shuffle requirement should include after order")
                .clone();
            let mut seen = std::collections::HashSet::new();
            assert!(
                after_order.iter().all(|id| seen.insert(*id)),
                "mulligan {mulligan_index} produced duplicate after-order ids: {after_order:?}"
            );

            game.reseal_verified_hidden_library_shuffle(ApplyHiddenLibraryShuffleInput {
                owner: 0,
                deck_hash: format!("mulligan-{mulligan_index}"),
                after_order,
                enforce_library_order: None,
            })
            .expect("verified shuffle should reseal repeated mulligan order");
        }
    }

    #[test]
    fn mulligan_bottoming_revealed_hand_card_does_not_require_library_shuffle() {
        let _id_counter_guard = crate::test_id_counter_guard();
        let mut game = WasmGame::new();
        game.initialize_empty_match(vec!["Alice".to_string(), "Bob".to_string()], 20, 1);
        let alice = PlayerId::from_index(0);
        for slot in 0..10 {
            game.game.create_hidden_card_placeholder(
                alice,
                Zone::Library,
                slot,
                format!("alice-slot-{slot}"),
            );
        }
        assert_eq!(game.game.draw_cards(alice, 7).len(), 7);
        game.ensure_card_definitions_loaded(["Mountain"]);
        let mountain = game
            .find_card_definition("Mountain")
            .expect("fixture card should load")
            .clone();
        for hand_id in game
            .game
            .player(alice)
            .expect("Alice should exist")
            .hand
            .clone()
        {
            game.game
                .reveal_hidden_card_with_definition(hand_id, &mountain)
                .expect("hand card should reveal privately");
        }

        let before = game.capture_crypto_audit_state();
        let bottom_card = game
            .game
            .player(alice)
            .expect("Alice should exist")
            .hand
            .first()
            .copied()
            .expect("Alice should have a hand card to bottom");
        let Some(moved) = game.game.move_object_by_effect(bottom_card, Zone::Library) else {
            panic!("bottomed card should move into library");
        };
        let player = game.game.player_mut(alice).expect("Alice should exist");
        let index = player
            .library
            .iter()
            .rposition(|candidate| *candidate == moved)
            .expect("moved card should be in library");
        let moved = player.library.remove(index);
        player.library.insert(0, moved);

        game.update_crypto_requirements_from(before);
        assert!(
            !game.last_crypto_requirements.iter().any(|requirement| {
                requirement.requirement_type == "verifiable_shuffle"
                    && requirement.owner == alice.index() as u8
            }),
            "bottoming a revealed hand card should not be treated as a library shuffle: {:?}",
            game.last_crypto_requirements
        );
    }

    #[test]
    fn sync_checkpoint_preserves_hidden_card_placeholders() {
        let _id_counter_guard = crate::test_id_counter_guard();
        let mut host = WasmGame::new();
        host.initialize_empty_match(vec!["Alice".to_string(), "Bob".to_string()], 20, 1);
        host.game.create_hidden_card_placeholder(
            PlayerId::from_index(1),
            Zone::Library,
            3,
            "commitment-3".to_string(),
        );

        let checkpoint = host.build_sync_checkpoint();
        let mut guest = WasmGame::new();
        guest
            .apply_sync_checkpoint(checkpoint)
            .expect("checkpoint should import hidden placeholders");
        let bob = guest
            .game
            .player(PlayerId::from_index(1))
            .expect("Bob should exist");
        assert_eq!(bob.library.len(), 1);
        let hidden_id = bob.library[0];
        let info = guest
            .game
            .hidden_card_info(hidden_id)
            .expect("hidden metadata should be restored");
        assert_eq!(info.slot, 3);
        assert_eq!(info.commitment, "commitment-3");
        assert_eq!(
            guest
                .game
                .object(hidden_id)
                .expect("hidden object should exist")
                .name,
            "Hidden Card"
        );
    }

    #[test]
    fn attack_direction_dispatch_and_sync_checkpoint_round_trip() {
        let _id_counter_guard = crate::test_id_counter_guard();
        let mut host = WasmGame::new();
        host.initialize_empty_match(
            vec![
                "Alice".to_string(),
                "Bob".to_string(),
                "Charlie".to_string(),
                "Diana".to_string(),
            ],
            20,
            803,
        );
        host.set_attack_direction(Some("right".to_string()))
            .expect("valid attack direction");

        let checkpoint = host.build_sync_checkpoint();
        assert!(matches!(
            checkpoint.attack_direction,
            Some(SyncAttackDirection::Right)
        ));
        let mut guest = WasmGame::new();
        guest
            .apply_sync_checkpoint(checkpoint)
            .expect("checkpoint should preserve attack direction");
        assert_eq!(
            guest.game.attack_direction(),
            Some(ironsmith::game_state::AttackDirection::Right)
        );
    }

    #[test]
    fn sync_checkpoint_round_trip_keeps_a_randomly_chosen_starting_seat() {
        let _id_counter_guard = crate::test_id_counter_guard();
        let mut host = WasmGame::new();
        host.initialize_empty_match(
            vec![
                "Alice".to_string(),
                "Bob".to_string(),
                "Charlie".to_string(),
            ],
            20,
            103,
        );
        host.game.set_starting_player(PlayerId::from_index(2));
        let expected = host.game.turn_store.turn_order.clone();
        assert_eq!(
            expected,
            vec![
                PlayerId::from_index(2),
                PlayerId::from_index(0),
                PlayerId::from_index(1),
            ],
            "the chosen seat heads the order, seating otherwise intact"
        );

        let checkpoint = host.build_sync_checkpoint();
        assert_eq!(checkpoint.turn.turn_order, vec![2, 0, 1]);

        let mut guest = WasmGame::new();
        guest
            .apply_sync_checkpoint(checkpoint)
            .expect("checkpoint should preserve the starting seat");

        assert_eq!(guest.game.turn_store.turn_order, expected);
        assert_eq!(guest.game.turn.active_player, PlayerId::from_index(2));
    }

    #[test]
    fn free_for_all_profile_sync_checkpoint_round_trip() {
        let _id_counter_guard = crate::test_id_counter_guard();
        let mut host = WasmGame::new();
        host.initialize_empty_match(
            vec![
                "Alice".to_string(),
                "Bob".to_string(),
                "Charlie".to_string(),
                "Diana".to_string(),
            ],
            20,
            806,
        );
        let seats = vec![
            PlayerId::from_index(2),
            PlayerId::from_index(0),
            PlayerId::from_index(3),
            PlayerId::from_index(1),
        ];
        host.match_format = MatchFormatInput::FreeForAll;
        host.game
            .restore_free_for_all(
                seats.clone(),
                ironsmith::FreeForAllAttackOption::Right,
                Some(1),
            )
            .expect("host profile");

        let checkpoint = host.build_sync_checkpoint();
        let serialized = checkpoint.free_for_all.as_ref().expect("profile encoded");
        assert_eq!(serialized.seats, vec![2, 0, 3, 1]);
        assert_eq!(serialized.attack, FreeForAllAttackInput::Right);
        assert_eq!(serialized.range_of_influence, Some(1));

        let mut guest = WasmGame::new();
        guest
            .apply_sync_checkpoint(checkpoint)
            .expect("checkpoint should preserve Free-for-All");
        assert_eq!(guest.match_format, MatchFormatInput::FreeForAll);
        let state = guest.game.free_for_all().expect("guest profile");
        assert_eq!(state.seats(), seats);
        assert_eq!(
            state.attack_option(),
            ironsmith::FreeForAllAttackOption::Right
        );
        assert_eq!(state.range_of_influence(), Some(1));
        assert_eq!(guest.game.physical_seats(), seats);
    }

    #[test]
    fn team_vs_team_profile_sync_checkpoint_round_trip() {
        let _id_counter_guard = crate::test_id_counter_guard();
        let mut host = WasmGame::new();
        host.initialize_empty_match(
            vec![
                "Alice".to_string(),
                "Bob".to_string(),
                "Charlie".to_string(),
                "Diana".to_string(),
            ],
            20,
            808,
        );
        let teams = vec![
            vec![PlayerId::from_index(0), PlayerId::from_index(1)],
            vec![PlayerId::from_index(2), PlayerId::from_index(3)],
        ];
        let seats = teams.iter().flatten().copied().collect::<Vec<_>>();
        host.match_format = MatchFormatInput::TeamVsTeam;
        host.game
            .restore_team_vs_team(teams.clone(), seats.clone(), 1, PlayerId::from_index(2))
            .expect("host profile");

        let checkpoint = host.build_sync_checkpoint();
        let serialized = checkpoint.team_vs_team.as_ref().expect("profile encoded");
        assert_eq!(serialized.teams, vec![vec![0, 1], vec![2, 3]]);
        assert_eq!(serialized.seats, vec![0, 1, 2, 3]);
        assert_eq!(serialized.starting_team, 1);
        assert_eq!(serialized.starting_player, 2);

        let mut guest = WasmGame::new();
        guest
            .apply_sync_checkpoint(checkpoint)
            .expect("checkpoint should preserve Team vs. Team");
        assert_eq!(guest.match_format, MatchFormatInput::TeamVsTeam);
        let state = guest.game.team_vs_team().expect("guest profile");
        assert_eq!(state.teams(), teams);
        assert_eq!(state.seats(), seats);
        assert_eq!(state.starting_team(), 1);
        assert_eq!(state.starting_player(), PlayerId::from_index(2));
        assert_eq!(
            guest.game.turn_store.turn_order,
            vec![
                PlayerId::from_index(2),
                PlayerId::from_index(3),
                PlayerId::from_index(0),
                PlayerId::from_index(1),
            ]
        );
    }

    #[test]
    fn team_vs_team_redacted_checkpoint_reveals_a_teammates_hand_only() {
        let _id_counter_guard = crate::test_id_counter_guard();
        let mut host = WasmGame::new();
        host.initialize_empty_match(
            vec![
                "Alice".to_string(),
                "Bob".to_string(),
                "Charlie".to_string(),
                "Diana".to_string(),
            ],
            20,
            808,
        );
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let charlie = PlayerId::from_index(2);
        host.game
            .restore_team_vs_team(
                vec![vec![alice, bob], vec![charlie, PlayerId::from_index(3)]],
                vec![alice, bob, charlie, PlayerId::from_index(3)],
                0,
                alice,
            )
            .expect("Team vs. Team profile");
        let card = ironsmith::card::CardBuilder::new(
            ironsmith::ids::CardId::from_raw(808_001),
            "Teammate Secret",
        )
        .card_types(vec![CardType::Instant])
        .build();
        let object = host.game.create_object_from_card(&card, bob, Zone::Hand);
        host.game.set_hidden_card_info(
            object,
            HiddenCardInfo {
                owner: bob,
                zone: Zone::Hand,
                slot: 0,
                commitment: "bob-hand-0".to_string(),
                origin_slot: None,
                origin_commitment: None,
                public_slot: None,
                public_commitment: None,
            },
        );

        let teammate = host
            .build_redacted_sync_checkpoint(alice)
            .expect("teammate checkpoint");
        let teammate_card = teammate
            .objects
            .iter()
            .find(|candidate| candidate.id == object.0)
            .expect("teammate card");
        assert_eq!(teammate_card.name, "Teammate Secret");

        let opponent = host
            .build_redacted_sync_checkpoint(charlie)
            .expect("opponent checkpoint");
        let opponent_card = opponent
            .objects
            .iter()
            .find(|candidate| candidate.id == object.0)
            .expect("opponent card");
        assert_eq!(opponent_card.name, "Hidden Card");
    }

    #[test]
    fn emperor_profile_sync_checkpoint_round_trip() {
        let _id_counter_guard = crate::test_id_counter_guard();
        let mut host = WasmGame::new();
        host.initialize_empty_match(
            (0..6).map(|index| format!("Player {index}")).collect(),
            20,
            809,
        );
        let seats = (0..6)
            .map(|index| PlayerId::from_index(index as u8))
            .collect::<Vec<_>>();
        let teams = vec![seats[0..3].to_vec(), seats[3..6].to_vec()];
        host.match_format = MatchFormatInput::Emperor;
        host.game
            .restore_emperor(
                teams.clone(),
                seats.clone(),
                1,
                seats[4],
                vec![1, 2, 1, 1, 2, 1],
            )
            .expect("host profile");
        assert!(host.game.leave_game(seats[3]));
        let frozen_range = host
            .game
            .limited_range_of_influence()
            .unwrap()
            .players_in_turn_snapshot(seats[2]);

        let checkpoint = host.build_sync_checkpoint();
        let encoded = checkpoint.emperor.as_ref().expect("profile encoded");
        assert_eq!(encoded.teams, vec![vec![0, 1, 2], vec![3, 4, 5]]);
        assert_eq!(encoded.ranges, vec![1, 2, 1, 1, 2, 1]);
        assert_eq!(encoded.starting_emperor, 4);

        let mut guest = WasmGame::new();
        guest
            .apply_sync_checkpoint(checkpoint)
            .expect("checkpoint should preserve Emperor");
        assert_eq!(guest.match_format, MatchFormatInput::Emperor);
        let profile = guest.game.emperor().expect("guest profile");
        assert_eq!(profile.teams(), teams);
        assert_eq!(profile.seats(), seats);
        assert_eq!(profile.ranges(), &[1, 2, 1, 1, 2, 1]);
        assert_eq!(profile.starting_emperor(), PlayerId::from_index(4));
        assert!(guest.game.deploy_creatures_enabled());
        assert_eq!(
            guest
                .game
                .limited_range_of_influence()
                .unwrap()
                .players_in_turn_snapshot(PlayerId::from_index(2)),
            frozen_range
        );
    }

    #[test]
    fn two_headed_giant_profile_and_shared_pools_sync_checkpoint_round_trip() {
        let _id_counter_guard = crate::test_id_counter_guard();
        let mut host = WasmGame::new();
        host.initialize_empty_match(
            (0..4).map(|index| format!("Player {index}")).collect(),
            20,
            810,
        );
        let seats = (0..4)
            .map(|index| PlayerId::from_index(index as u8))
            .collect::<Vec<_>>();
        let teams = vec![seats[0..2].to_vec(), seats[2..4].to_vec()];
        host.match_format = MatchFormatInput::TwoHeadedGiant;
        host.game.set_random_seed(810);
        host.game
            .enable_two_headed_giant(teams.clone())
            .expect("host profile");
        host.game.lose_life(seats[0], 7);
        host.game.add_player_counters_with_source(
            seats[1],
            ironsmith::CounterType::Poison,
            4,
            None,
            None,
        ).unwrap();
        host.game
            .set_shared_team_member_order(0, vec![seats[1], seats[0]])
            .unwrap();

        let checkpoint = host.build_sync_checkpoint();
        let encoded = checkpoint
            .two_headed_giant
            .as_ref()
            .expect("profile encoded");
        assert_eq!(encoded.teams, vec![vec![0, 1], vec![2, 3]]);
        assert_eq!(encoded.seats, vec![0, 1, 2, 3]);
        assert_eq!(encoded.starting_life, 30);
        assert_eq!(encoded.poison_threshold, 15);

        let mut guest = WasmGame::new();
        guest
            .apply_sync_checkpoint(checkpoint)
            .expect("checkpoint should preserve Two-Headed Giant");
        assert_eq!(guest.match_format, MatchFormatInput::TwoHeadedGiant);
        let profile = guest.game.two_headed_giant().expect("guest profile");
        assert_eq!(profile.teams(), teams);
        assert_eq!(profile.seats(), seats);
        assert!(guest.game.shared_team_turns_enabled());
        assert_eq!(
            guest.game.shared_team_turns().unwrap().member_orders()[0],
            vec![seats[1], seats[0]]
        );
        assert_eq!(guest.game.player(seats[0]).unwrap().life, 23);
        assert_eq!(guest.game.player(seats[1]).unwrap().life, 23);
        assert_eq!(guest.game.player(seats[0]).unwrap().poison_counters, 4);
        assert_eq!(guest.game.player(seats[1]).unwrap().poison_counters, 4);
    }

    #[test]
    fn alternating_teams_profile_sync_checkpoint_round_trip() {
        let _id_counter_guard = crate::test_id_counter_guard();
        let mut host = WasmGame::new();
        host.initialize_empty_match(
            (0..6).map(|index| format!("Player {index}")).collect(),
            20,
            811,
        );
        let players = (0..6)
            .map(|index| PlayerId::from_index(index as u8))
            .collect::<Vec<_>>();
        let teams = vec![
            vec![players[0], players[1]],
            vec![players[2], players[3]],
            vec![players[4], players[5]],
        ];
        let seats = vec![
            players[0], players[2], players[4], players[1], players[3], players[5],
        ];
        host.match_format = MatchFormatInput::AlternatingTeams;
        host.game
            .restore_alternating_teams(
                teams.clone(),
                seats.clone(),
                players[4],
                ironsmith::FreeForAllAttackOption::Right,
                Some(2),
                true,
            )
            .expect("host profile");
        assert!(host.game.leave_game(players[2]));
        let frozen_range = host
            .game
            .limited_range_of_influence()
            .unwrap()
            .players_in_turn_snapshot(players[0]);

        let checkpoint = host.build_sync_checkpoint();
        let encoded = checkpoint
            .alternating_teams
            .as_ref()
            .expect("profile encoded");
        assert_eq!(encoded.teams, vec![vec![0, 1], vec![2, 3], vec![4, 5]]);
        assert_eq!(encoded.seats, vec![0, 2, 4, 1, 3, 5]);
        assert_eq!(encoded.starting_player, 4);
        assert_eq!(encoded.attack, FreeForAllAttackInput::Right);
        assert_eq!(encoded.range_of_influence, Some(2));
        assert!(encoded.deploy_creatures);

        let mut guest = WasmGame::new();
        guest
            .apply_sync_checkpoint(checkpoint)
            .expect("checkpoint should preserve Alternating Teams");
        assert_eq!(guest.match_format, MatchFormatInput::AlternatingTeams);
        let profile = guest.game.alternating_teams().expect("guest profile");
        assert_eq!(profile.teams(), teams);
        assert_eq!(profile.seats(), seats);
        assert_eq!(profile.starting_player(), players[4]);
        assert_eq!(
            profile.attack_option(),
            ironsmith::FreeForAllAttackOption::Right
        );
        assert_eq!(profile.range_of_influence(), Some(2));
        assert!(profile.deploy_creatures());
        assert_eq!(
            guest
                .game
                .limited_range_of_influence()
                .unwrap()
                .players_in_turn_snapshot(players[0]),
            frozen_range
        );
    }

    #[test]
    fn grand_melee_marker_lanes_sync_checkpoint_round_trip() {
        let _id_counter_guard = crate::test_id_counter_guard();
        let mut host = WasmGame::new();
        host.initialize_empty_match(
            (0..10).map(|index| format!("Player {index}")).collect(),
            20,
            807,
        );
        let seats = (0..10)
            .map(|index| PlayerId::from_index(index as u8))
            .collect::<Vec<_>>();
        host.match_format = MatchFormatInput::GrandMelee;
        host.game.restore_grand_melee(seats.clone()).unwrap();
        host.game.next_turn();
        host.game.next_turn();
        host.game
            .turn_store
            .extra_turns
            .push(PlayerId::from_index(3));
        host.game.combat = Some(ironsmith::combat_state::CombatState {
            attacking_bands: vec![vec![ObjectId::from_raw(701), ObjectId::from_raw(702)]],
            ..Default::default()
        });
        let expected_views = host.game.grand_melee_marker_views();
        let expected_focus = host.game.grand_melee().unwrap().focused_marker();

        let checkpoint = host.build_sync_checkpoint();
        let encoded = checkpoint.grand_melee.as_ref().expect("profile encoded");
        assert_eq!(encoded.markers.len(), 2);
        assert_eq!(encoded.focused_marker, expected_focus);
        let encoded_focus = encoded
            .markers
            .iter()
            .find(|marker| marker.number == expected_focus)
            .unwrap();
        assert_eq!(encoded_focus.extra_turns, vec![3]);
        assert!(encoded_focus.combat.is_some());
        assert!(!encoded_focus.range_turn_snapshot.is_empty());

        let mut guest = WasmGame::new();
        guest
            .apply_sync_checkpoint(checkpoint)
            .expect("checkpoint should preserve Grand Melee lanes");
        assert_eq!(guest.match_format, MatchFormatInput::GrandMelee);
        assert_eq!(guest.game.grand_melee().unwrap().seats(), seats);
        assert_eq!(
            guest.game.grand_melee().unwrap().focused_marker(),
            expected_focus
        );
        assert_eq!(guest.game.grand_melee_marker_views(), expected_views);
        let restored = guest.game.grand_melee_restore_snapshot().unwrap();
        let restored_focus = restored
            .markers
            .iter()
            .find(|marker| marker.number == expected_focus)
            .unwrap();
        assert_eq!(
            restored_focus.turn_store.extra_turns,
            vec![PlayerId::from_index(3)]
        );
        assert_eq!(
            restored_focus.combat.as_ref().unwrap().attacking_bands,
            vec![vec![ObjectId::from_raw(701), ObjectId::from_raw(702)]]
        );
    }

    #[test]
    fn team_and_deploy_creatures_sync_checkpoint_round_trip() {
        let _id_counter_guard = crate::test_id_counter_guard();
        let mut host = WasmGame::new();
        host.initialize_empty_match(
            vec![
                "Alice".to_string(),
                "Bob".to_string(),
                "Charlie".to_string(),
                "Diana".to_string(),
            ],
            20,
            804,
        );
        host.game
            .set_teams(vec![
                vec![PlayerId::from_index(0), PlayerId::from_index(1)],
                vec![PlayerId::from_index(2), PlayerId::from_index(3)],
            ])
            .expect("valid team assignment");
        host.set_deploy_creatures(true);

        let checkpoint = host.build_sync_checkpoint();
        assert_eq!(checkpoint.teams, Some(vec![vec![0, 1], vec![2, 3]]));
        assert!(checkpoint.deploy_creatures);

        let mut guest = WasmGame::new();
        guest
            .apply_sync_checkpoint(checkpoint)
            .expect("checkpoint should preserve team deploy state");
        assert!(
            guest
                .game
                .are_teammates(PlayerId::from_index(0), PlayerId::from_index(1))
        );
        assert!(
            guest
                .game
                .are_opponents(PlayerId::from_index(0), PlayerId::from_index(2))
        );
        assert!(guest.game.deploy_creatures_enabled());
    }

    #[test]
    fn shared_team_turns_dispatch_and_sync_checkpoint_round_trip() {
        let _id_counter_guard = crate::test_id_counter_guard();
        let mut host = WasmGame::new();
        host.initialize_empty_match(
            vec![
                "Alice".to_string(),
                "Bob".to_string(),
                "Charlie".to_string(),
                "Diana".to_string(),
            ],
            20,
            805,
        );
        host.game
            .set_teams(vec![
                vec![PlayerId::from_index(0), PlayerId::from_index(1)],
                vec![PlayerId::from_index(2), PlayerId::from_index(3)],
            ])
            .expect("valid team assignment");
        host.set_shared_team_turns(true)
            .expect("adjacent teams can share turns");
        host.game
            .set_shared_team_member_order(0, vec![PlayerId::from_index(1), PlayerId::from_index(0)])
            .expect("team order selected");

        let checkpoint = host.build_sync_checkpoint();
        assert!(checkpoint.shared_team_turns);
        assert_eq!(checkpoint.shared_team_member_orders[0], vec![1, 0]);
        assert_eq!(checkpoint.turn.active_player, 1);

        let mut guest = WasmGame::new();
        guest
            .apply_sync_checkpoint(checkpoint)
            .expect("checkpoint should preserve shared team turns");
        assert!(guest.game.shared_team_turns_enabled());
        assert_eq!(guest.game.turn.active_player, PlayerId::from_index(1));
        assert_eq!(
            guest.game.active_players(),
            vec![PlayerId::from_index(1), PlayerId::from_index(0)]
        );
    }

    #[test]
    fn sync_checkpoint_preserves_public_ante_zone_and_ownership() {
        let _id_counter_guard = crate::test_id_counter_guard();
        let mut host = WasmGame::new();
        host.initialize_empty_match(vec!["Alice".to_string(), "Bob".to_string()], 20, 4072);
        let alice = PlayerId::from_index(0);
        let library_id = ObjectId::from_raw(
            host.add_card_to_zone(0, "Ornithopter".to_string(), "library".to_string(), true)
                .expect("host should add a library card"),
        );
        let ante_id = host
            .game
            .ante_owned_object(alice, library_id)
            .expect("owner should ante the card");

        let checkpoint = host.build_sync_checkpoint();
        assert_eq!(checkpoint.ante, vec![ante_id.0]);
        let public_audit = host.build_public_audit_checkpoint();
        assert_eq!(public_audit.ante, vec![ante_id.0]);

        let mut guest = WasmGame::new();
        guest
            .apply_sync_checkpoint(checkpoint)
            .expect("checkpoint should import ante");
        assert_eq!(guest.game.ante, vec![ante_id]);
        let restored = guest
            .game
            .object(ante_id)
            .expect("ante card should restore");
        assert_eq!(restored.zone, Zone::Ante);
        assert_eq!(restored.owner, alice);
    }

    #[test]
    fn planechase_snapshot_action_and_sync_checkpoint_round_trip() {
        let _id_counter_guard = crate::test_id_counter_guard();
        let mut host = WasmGame::new();
        host.initialize_empty_match(vec!["Alice".to_string(), "Bob".to_string()], 20, 901);
        host.match_format = MatchFormatInput::Planechase;
        let alice = PlayerId::from_index(0);
        let cards = (0..20)
            .map(|index| {
                let mut definition = CardDefinition::new(
                    ironsmith::CardBuilder::new(CardId::new(), format!("Sync Plane {index}"))
                        .card_types(vec![CardType::Plane])
                        .build(),
                );
                definition.abilities.push(ironsmith::Ability::triggered(
                    ironsmith::triggers::Trigger::player_rolls_die(ironsmith::PlayerFilter::You),
                    vec![ironsmith::Effect::gain_life(1)],
                ));
                (definition, ironsmith::game_state::PlanarCardKind::Plane)
            })
            .collect::<Vec<_>>();
        let definitions = cards
            .iter()
            .map(|(definition, _)| definition.clone())
            .collect::<Vec<_>>();
        for definition in &definitions {
            host.registry.register(definition.clone());
        }
        host.game
            .enable_planechase_communal(cards)
            .expect("communal plane deck should enable");
        let face_up = host.game.reveal_starting_plane().unwrap();
        host.game.force_next_die_roll(6);
        host.game.roll_planar_die(alice, true).unwrap();

        assert_eq!(
            special_action_ref(&ironsmith::special_actions::SpecialAction::RollPlanarDie),
            SpecialActionRef::RollPlanarDie
        );
        let snapshot = GameSnapshot::from_game(
            &host.game,
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
            1,
        );
        let planar = snapshot
            .planechase
            .expect("snapshot should expose Planechase");
        assert_eq!(planar.planar_controller, alice.0);
        assert_eq!(planar.die_roll_cost, 1);
        assert_eq!(planar.face_up[0].id, face_up.0);

        let checkpoint = host.build_sync_checkpoint();
        assert!(checkpoint.planechase.is_some());
        let public_audit = host.build_public_audit_checkpoint();
        let public_planar = public_audit
            .planechase
            .as_ref()
            .expect("public audit should expose planar public state");
        assert_eq!(public_planar.communal_deck_size, Some(19));
        assert_eq!(public_planar.face_up, vec![face_up.0]);
        assert_eq!(public_audit.command, vec![face_up.0]);
        assert_eq!(public_audit.objects.len(), 1);
        assert!(public_audit.hidden_zones.iter().any(|zone| {
            zone.zone == "communal_planar_deck"
                && zone.count == 19
                && zone.commitment_root.is_some()
        }));
        let mut guest = WasmGame::new();
        for definition in definitions {
            guest.registry.register(definition);
        }
        guest
            .apply_sync_checkpoint(checkpoint)
            .expect("Planechase checkpoint should import");
        assert_eq!(guest.match_format, MatchFormatInput::Planechase);
        assert_eq!(guest.game.face_up_planar_objects(), &[face_up]);
        assert!(
            guest
                .game
                .object(face_up)
                .unwrap()
                .abilities
                .iter()
                .all(|ability| ability.functional_zones == vec![Zone::Command])
        );
        assert!(guest.game.planar_deck(alice).unwrap().iter().all(|object| {
            guest
                .game
                .object(*object)
                .unwrap()
                .abilities
                .iter()
                .all(|ability| ability.functional_zones.is_empty())
        }));
        assert_eq!(guest.game.planar_die_roll_cost(alice), Some(1));
        assert_eq!(
            guest.game.planar_card_kind(face_up),
            Some(ironsmith::game_state::PlanarCardKind::Plane)
        );
    }

    #[test]
    fn vanguard_snapshot_and_sync_checkpoint_round_trip() {
        let _id_counter_guard = crate::test_id_counter_guard();
        let mut host = WasmGame::new();
        host.initialize_empty_match(vec!["Alice".to_string(), "Bob".to_string()], 20, 902);
        host.match_format = MatchFormatInput::Vanguard;
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let cards = [
            (alice, "Patient Avatar", 2, -3),
            (bob, "Fierce Avatar", -1, 4),
        ]
        .into_iter()
        .map(|(owner, name, hand, life)| {
            let mut definition = CardDefinition::new(
                ironsmith::CardBuilder::new(CardId::new(), name)
                    .card_types(vec![CardType::Vanguard])
                    .vanguard_modifiers(hand, life)
                    .build(),
            );
            definition.abilities.push(ironsmith::Ability::triggered(
                ironsmith::triggers::Trigger::player_rolls_die(ironsmith::PlayerFilter::You),
                vec![ironsmith::Effect::gain_life(1)],
            ));
            (owner, definition)
        })
        .collect::<Vec<_>>();
        let definitions = cards
            .iter()
            .map(|(_, definition)| definition.clone())
            .collect::<Vec<_>>();
        for definition in &definitions {
            host.registry.register(definition.clone());
        }
        host.game
            .enable_vanguard(cards)
            .expect("Vanguard should enable");

        let alice_card = host.game.vanguard_card(alice).unwrap();
        let snapshot = GameSnapshot::from_game(
            &host.game,
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
            1,
        );
        let vanguard = snapshot.vanguard.expect("snapshot should expose Vanguard");
        assert_eq!(vanguard.cards.len(), 2);
        assert_eq!(vanguard.cards[0].id, alice_card.0);
        assert_eq!(vanguard.cards[0].hand_modifier, 2);
        assert_eq!(vanguard.cards[0].life_modifier, -3);

        let checkpoint = host.build_sync_checkpoint();
        assert!(checkpoint.vanguard.is_some());
        let public_audit = host.build_public_audit_checkpoint();
        assert!(public_audit.vanguard.is_some());
        assert!(public_audit.command.contains(&alice_card.0));

        let mut guest = WasmGame::new();
        for definition in definitions {
            guest.registry.register(definition);
        }
        guest
            .apply_sync_checkpoint(checkpoint)
            .expect("Vanguard checkpoint should import");
        assert_eq!(guest.match_format, MatchFormatInput::Vanguard);
        assert_eq!(guest.game.vanguard_hand_modifier(alice), 2);
        assert_eq!(guest.game.vanguard_life_modifier(bob), 4);
        assert_eq!(guest.game.vanguard_card(alice), Some(alice_card));
        assert_eq!(guest.game.player(alice).unwrap().life, 17);
        assert_eq!(guest.game.player(alice).unwrap().max_hand_size, 9);
        assert!(
            guest
                .game
                .object(alice_card)
                .unwrap()
                .abilities
                .iter()
                .all(|ability| ability.functional_zones == vec![Zone::Command])
        );
    }

    #[test]
    fn archenemy_snapshot_public_audit_and_sync_checkpoint_round_trip() {
        let _id_counter_guard = crate::test_id_counter_guard();
        let mut host = WasmGame::new();
        host.initialize_empty_match(vec!["Alice".to_string(), "Bob".to_string()], 20, 903);
        host.match_format = MatchFormatInput::Archenemy;
        let alice = PlayerId::from_index(0);
        let definitions = (0..20)
            .map(|index| {
                CardDefinition::new(
                    ironsmith::CardBuilder::new(CardId::new(), format!("Sync Scheme {index}"))
                        .card_types(vec![CardType::Scheme])
                        .build(),
                )
            })
            .collect::<Vec<_>>();
        for definition in &definitions {
            host.registry.register(definition.clone());
        }
        host.game
            .enable_archenemy(
                ironsmith::game_state::ArchenemyVariant::Default,
                vec![(alice, definitions.clone())],
            )
            .unwrap();
        let face_up = host.game.set_scheme_in_motion(alice).unwrap();

        let snapshot = GameSnapshot::from_game(
            &host.game,
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
            1,
        );
        let archenemy = snapshot
            .archenemy
            .expect("snapshot should expose Archenemy");
        assert_eq!(archenemy.archenemies, vec![alice.0]);
        assert_eq!(archenemy.deck_sizes[0].size, 19);
        assert_eq!(archenemy.face_up[0].id, face_up.0);

        let checkpoint = host.build_sync_checkpoint();
        assert!(checkpoint.archenemy.is_some());
        let public_audit = host.build_public_audit_checkpoint();
        let public_archenemy = public_audit
            .archenemy
            .as_ref()
            .expect("public audit should expose Archenemy public state");
        assert_eq!(public_archenemy.decks, vec![(alice.0, 19)]);
        assert_eq!(public_archenemy.face_up, vec![face_up.0]);
        assert_eq!(public_audit.command, vec![face_up.0]);
        assert!(public_audit.hidden_zones.iter().any(|zone| {
            zone.zone == "scheme_deck" && zone.count == 19 && zone.commitment_root.is_some()
        }));

        let mut guest = WasmGame::new();
        for definition in definitions {
            guest.registry.register(definition);
        }
        guest
            .apply_sync_checkpoint(checkpoint)
            .expect("Archenemy checkpoint should import");
        assert_eq!(guest.match_format, MatchFormatInput::Archenemy);
        assert!(guest.game.is_archenemy(alice));
        assert_eq!(guest.game.face_up_schemes(), &[face_up]);
        assert_eq!(guest.game.scheme_deck(alice).unwrap().len(), 19);
    }

    #[test]
    fn conspiracy_snapshot_public_audit_and_sync_checkpoint_preserve_secrecy() {
        let _id_counter_guard = crate::test_id_counter_guard();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let definition = ironsmith_registry_test::cards::builders::CardDefinitionBuilder::new(
            CardId::new(),
            "Checkpoint Secret",
        )
        .card_types(vec![CardType::Conspiracy])
        .parse_text("Hidden agenda")
        .expect("synthetic hidden-agenda conspiracy should compile");

        let mut host = WasmGame::new();
        host.initialize_empty_match(vec!["Alice".to_string(), "Bob".to_string()], 20, 905);
        host.match_format = MatchFormatInput::ConspiracyDraft;
        host.registry.register(definition.clone());
        host.game
            .enable_conspiracy(vec![(
                alice,
                vec![ironsmith::ConspiracySetupCard {
                    definition: definition.clone(),
                    agenda_names: vec!["Grizzly Bears".to_string()],
                }],
            )])
            .unwrap();
        let conspiracy_id = host.game.conspiracy_cards()[0];

        let owner_snapshot = GameSnapshot::from_game(
            &host.game,
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
            1,
        );
        let owner_card = &owner_snapshot.conspiracy.unwrap().cards[0];
        assert_eq!(owner_card.name.as_deref(), Some("Checkpoint Secret"));
        assert_eq!(
            owner_card.agenda_names.as_deref().unwrap(),
            ["Grizzly Bears"]
        );

        let opponent_snapshot = GameSnapshot::from_game(
            &host.game,
            bob,
            None,
            None,
            None,
            None,
            None,
            Vec::new(),
            None,
            false,
            None,
            2,
        );
        let opponent_card = &opponent_snapshot.conspiracy.unwrap().cards[0];
        assert!(opponent_card.face_down);
        assert!(opponent_card.name.is_none());
        assert!(opponent_card.oracle_text.is_none());
        assert!(opponent_card.agenda_names.is_none());

        let public_audit = host.build_public_audit_checkpoint();
        let public_conspiracy = public_audit
            .conspiracy
            .as_ref()
            .expect("public audit should include redacted conspiracy topology");
        assert_eq!(
            public_conspiracy.cards,
            vec![(alice.0, vec![conspiracy_id.0])]
        );
        assert_eq!(public_conspiracy.face_down, vec![conspiracy_id.0]);
        let public_object = public_audit
            .objects
            .iter()
            .find(|object| object.id == conspiracy_id.0)
            .expect("face-down conspiracy should have a public card back");
        assert!(public_object.face_down);
        assert!(public_object.identity.is_none());
        assert!(
            !serde_json::to_string(&public_audit)
                .unwrap()
                .contains("Grizzly Bears")
        );

        let checkpoint = host.build_sync_checkpoint();
        assert_eq!(
            checkpoint.conspiracy.as_ref().unwrap().agenda_names,
            vec![(conspiracy_id.0, vec!["Grizzly Bears".to_string()])]
        );
        let mut guest = WasmGame::new();
        guest.registry.register(definition);
        guest
            .apply_sync_checkpoint(checkpoint)
            .expect("Conspiracy checkpoint should import");
        assert_eq!(guest.match_format, MatchFormatInput::ConspiracyDraft);
        assert!(guest.game.is_face_down_conspiracy(conspiracy_id));
        assert_eq!(
            guest.game.agenda_names_for(alice, conspiracy_id).unwrap(),
            ["Grizzly Bears"]
        );
        assert!(guest.game.agenda_names_for(bob, conspiracy_id).is_none());
        assert!(
            guest
                .game
                .object(conspiracy_id)
                .unwrap()
                .abilities
                .iter()
                .all(|ability| ability.functional_zones.is_empty())
        );
        guest
            .game
            .turn_conspiracy_face_up(alice, conspiracy_id)
            .unwrap();
        assert_eq!(
            guest.game.agenda_names_for(bob, conspiracy_id).unwrap(),
            ["Grizzly Bears"]
        );
    }

    #[test]
    fn hidden_deck_manifest_populates_committed_library_placeholders() {
        let _id_counter_guard = crate::test_id_counter_guard();
        let mut game = WasmGame::new();
        game.initialize_empty_match(vec!["Alice".to_string(), "Bob".to_string()], 20, 1);
        game.populate_libraries_with_hidden_manifests(
            &[vec!["Forest".to_string()], Vec::new()],
            &[HiddenDeckManifestInput {
                owner: 1,
                deck_count: 2,
                sideboard_count: 0,
                commander_count: 0,
                decklist_hash: "deck-hash".to_string(),
                commitment_root: "root".to_string(),
                slot_commitments: vec![
                    HiddenDeckSlotInput {
                        slot: 0,
                        commitment: "commitment-0".to_string(),
                    },
                    HiddenDeckSlotInput {
                        slot: 1,
                        commitment: "commitment-1".to_string(),
                    },
                ],
            }],
        )
        .expect("manifest should populate hidden placeholders");

        let bob = game
            .game
            .player(PlayerId::from_index(1))
            .expect("Bob should exist");
        assert_eq!(bob.library.len(), 2);
        assert!(
            bob.library
                .iter()
                .all(|id| game.game.is_hidden_card_placeholder(*id))
        );
    }

    #[test]
    fn local_committed_card_exports_opening_metadata() {
        let _id_counter_guard = crate::test_id_counter_guard();
        let mut game = WasmGame::new();
        game.initialize_empty_match(vec!["Alice".to_string(), "Bob".to_string()], 20, 1);
        game.populate_libraries_with_hidden_manifests(
            &[vec!["Lightning Bolt".to_string()], Vec::new()],
            &[HiddenDeckManifestInput {
                owner: 0,
                deck_count: 1,
                sideboard_count: 0,
                commander_count: 0,
                decklist_hash: "alice-deck".to_string(),
                commitment_root: "alice-root".to_string(),
                slot_commitments: vec![HiddenDeckSlotInput {
                    slot: 0,
                    commitment: "alice-slot-0".to_string(),
                }],
            }],
        )
        .expect("local manifest should tag real cards");

        let alice = game
            .game
            .player(PlayerId::from_index(0))
            .expect("Alice should exist");
        let object_id = alice.library[0];
        let opening = game
            .hidden_card_opening_export(object_id)
            .expect("local committed card should export opening metadata");

        assert_eq!(opening.object_id, object_id.0);
        assert_eq!(opening.owner, 0);
        assert_eq!(opening.slot, 0);
        assert_eq!(opening.card, "Lightning Bolt");
        assert_eq!(opening.commitment, "alice-slot-0");
    }

    #[test]
    fn redacted_sync_checkpoint_hides_opponent_hidden_zones_and_imports() {
        let _id_counter_guard = crate::test_id_counter_guard();
        let mut host = WasmGame::new();
        host.initialize_empty_match(vec!["Alice".to_string(), "Bob".to_string()], 20, 1);
        host.populate_libraries_with_hidden_manifests(
            &[
                vec!["Forest".to_string()],
                vec!["Lightning Bolt".to_string(), "Counterspell".to_string()],
            ],
            &[
                HiddenDeckManifestInput {
                    owner: 0,
                    deck_count: 1,
                    sideboard_count: 0,
                    commander_count: 0,
                    decklist_hash: "alice-deck".to_string(),
                    commitment_root: "alice-root".to_string(),
                    slot_commitments: vec![HiddenDeckSlotInput {
                        slot: 0,
                        commitment: "alice-slot-0".to_string(),
                    }],
                },
                HiddenDeckManifestInput {
                    owner: 1,
                    deck_count: 2,
                    sideboard_count: 0,
                    commander_count: 0,
                    decklist_hash: "bob-deck".to_string(),
                    commitment_root: "bob-root".to_string(),
                    slot_commitments: vec![
                        HiddenDeckSlotInput {
                            slot: 0,
                            commitment: "bob-slot-0".to_string(),
                        },
                        HiddenDeckSlotInput {
                            slot: 1,
                            commitment: "bob-slot-1".to_string(),
                        },
                    ],
                },
            ],
        )
        .expect("host should populate committed decks");
        let _ = host.game.draw_cards(PlayerId::from_index(1), 1);

        let checkpoint = host
            .build_redacted_sync_checkpoint(PlayerId::from_index(0))
            .expect("redacted checkpoint should build");
        assert!(
            checkpoint
                .objects
                .iter()
                .filter(|object| object.owner == 1
                    && (object.zone == "hand" || object.zone == "library"))
                .all(|object| object.name == "Hidden Card" && object.hidden_card.is_some())
        );
        assert!(
            !checkpoint
                .objects
                .iter()
                .any(|object| object.name == "Lightning Bolt" || object.name == "Counterspell")
        );

        let mut guest = WasmGame::new();
        guest
            .apply_sync_checkpoint(checkpoint)
            .expect("redacted checkpoint should import");
        let bob = guest
            .game
            .player(PlayerId::from_index(1))
            .expect("Bob should exist");
        assert_eq!(bob.hand.len() + bob.library.len(), 2);
        for id in bob.hand.iter().chain(bob.library.iter()) {
            assert!(guest.game.is_hidden_card_placeholder(*id));
        }
    }

    #[test]
    fn redacted_sync_checkpoint_hides_opened_opponent_hand_cards() {
        let _id_counter_guard = crate::test_id_counter_guard();
        let mut host = WasmGame::new();
        host.initialize_empty_match(vec!["Alice".to_string(), "Bob".to_string()], 20, 1);
        host.populate_libraries_with_hidden_manifests(
            &[Vec::new(), vec!["Lightning Bolt".to_string()]],
            &[HiddenDeckManifestInput {
                owner: 1,
                deck_count: 1,
                sideboard_count: 0,
                commander_count: 0,
                decklist_hash: "bob-deck".to_string(),
                commitment_root: "bob-root".to_string(),
                slot_commitments: vec![HiddenDeckSlotInput {
                    slot: 0,
                    commitment: "bob-slot-0".to_string(),
                }],
            }],
        )
        .expect("host should populate committed decks");
        let drawn = host.game.draw_cards(PlayerId::from_index(1), 1);
        let hand_id = drawn[0];
        host.ensure_card_definitions_loaded(["Lightning Bolt"]);
        let definition = host
            .find_card_definition("Lightning Bolt")
            .expect("fixture card should load")
            .clone();
        host.game
            .reveal_hidden_card_with_definition(hand_id, &definition)
            .expect("Bob should be able to open their hand card locally");

        let checkpoint = host
            .build_redacted_sync_checkpoint(PlayerId::from_index(0))
            .expect("redacted checkpoint should build after private reveal");
        let redacted = checkpoint
            .objects
            .iter()
            .find(|object| object.id == hand_id.0)
            .expect("opened hand card should be present in checkpoint");
        assert_eq!(redacted.name, "Hidden Card");
        assert_eq!(
            redacted
                .hidden_card
                .as_ref()
                .expect("redacted card should carry commitment")
                .commitment,
            "bob-slot-0"
        );
    }
    #[test]
    fn sync_checkpoint_rebuilds_static_control_without_freezing_its_result() {
        let _id_counter_guard = crate::test_id_counter_guard();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let mut host = WasmGame::new();
        host.initialize_empty_match(vec!["Alice".into(), "Bob".into()], 20, 1);
        let recipient = CardDefinition::new(ironsmith::CardBuilder::new(CardId::new(), "Static control checkpoint recipient")
            .card_types(vec![CardType::Artifact]).build());
        let mut control = CardDefinition::new(ironsmith::CardBuilder::new(CardId::new(), "Static control checkpoint aura")
            .card_types(vec![CardType::Enchantment]).subtypes(vec![ironsmith::types::Subtype::Aura]).build());
        control.abilities.push(ironsmith::Ability::static_ability(ironsmith::static_abilities::StaticAbility::enchant(
            ironsmith::object::AuraAttachmentFilter::Object(ironsmith::target::ObjectFilter::permanent()))));
        control.abilities.push(ironsmith::Ability::static_ability(ironsmith::static_abilities::StaticAbility::control_attached_permanent(
            "You control enchanted permanent".into())));
        host.registry.register(recipient.clone());
        host.registry.register(control.clone());
        let permanent = host.game.create_object_from_definition(&recipient, alice, Zone::Battlefield);
        host.game.stage_initial_controller_for_assembly(permanent, bob);
        let aura = host.game.create_object_from_definition(&control, alice, Zone::Battlefield);
        host.game.object_mut(aura).unwrap().attached_to = Some(ironsmith::object::AttachmentTarget::Object(permanent));
        host.game.object_mut(permanent).unwrap().attachments.push(aura);
        host.game.refresh_continuous_state().expect("static control completes");
        assert_eq!(host.game.current_controller(permanent), Some(alice));
        let checkpoint = host.build_sync_checkpoint();
        let audit = host.build_public_audit_checkpoint();
        assert_eq!(audit.objects.iter().find(|object| object.id == permanent.0).unwrap().initial_controller, bob.0);
        let mut guest = WasmGame::new();
        guest.registry.register(recipient);
        guest.registry.register(control);
        guest.apply_sync_checkpoint(checkpoint).expect("static control checkpoint imports");
        assert_eq!(guest.game.object(permanent).unwrap().initial_controller, bob);
        assert_eq!(guest.game.current_controller(permanent), Some(alice));
        assert!(guest.game.effect_store.continuous_effects.effects().is_empty());
        guest.game.move_object(aura, Zone::Graveyard,
            ironsmith::events::cause::EventCause::from_effect(aura, alice)).expect("control aura leaves");
        guest.game.refresh_continuous_state().expect("source loss completes");
        assert_eq!(guest.game.current_controller(permanent), Some(bob),
            "removing the actual control source reveals initial control, not a frozen imported assignment");
        assert_eq!(guest.game.object(permanent).unwrap().owner, alice);
    }

    #[test]
    fn sync_checkpoint_rejects_declared_player_ids_that_do_not_match_runtime_seats() {
        let _id_counter_guard = crate::test_id_counter_guard();
        let alice = PlayerId::from_index(0);
        let mut host = WasmGame::new();
        host.initialize_empty_match(vec!["Alice".into(), "Bob".into()], 20, 1);
        let definition = CardDefinition::new(ironsmith::CardBuilder::new(CardId::new(), "Checkpoint declared player fixture")
            .card_types(vec![CardType::Artifact]).build());
        host.registry.register(definition.clone());
        host.game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let valid = host.build_sync_checkpoint();
        for sparse in [false, true] {
            let mut malformed = valid.clone();
            if sparse {
                malformed.players[0].id = 255;
                for object in &mut malformed.objects {
                    object.owner = 255;
                    object.initial_controller = 255;
                    object.controller = 255;
                }
            } else {
                malformed.players[1].id = 0;
            }
            let mut guest = WasmGame::new();
            guest.initialize_empty_match(vec!["Carol".into(), "Dan".into()], 20, 2);
            guest.registry.register(definition.clone());
            let error = guest.apply_sync_checkpoint(malformed)
                .expect_err("declared IDs must name actual imported engine seats");
            assert!(error.contains("player seat"), "sparse={sparse}: {error}");
            assert_eq!(guest.game.players[0].name, "Carol");
            assert_eq!(guest.game.players[1].name, "Dan");
            assert!(guest.game.object_ids_in_deterministic_order().is_empty());
            assert!(guest.game.effect_store.continuous_effects.effects().is_empty());
        }
    }

    #[test]
    fn sync_checkpoint_requires_initial_control_and_rejects_invalid_player_roles() {
        let _id_counter_guard = crate::test_id_counter_guard();
        let alice = PlayerId::from_index(0);
        let mut host = WasmGame::new();
        host.initialize_empty_match(vec!["Alice".into(), "Bob".into()], 20, 1);
        let definition = CardDefinition::new(ironsmith::CardBuilder::new(CardId::new(), "Checkpoint controller validation fixture")
            .card_types(vec![CardType::Artifact]).build());
        host.registry.register(definition.clone());
        host.game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let checkpoint = host.build_sync_checkpoint();
        assert_eq!(checkpoint.version, 4);
        let mut missing = serde_json::to_value(&checkpoint).unwrap();
        missing["objects"][0].as_object_mut().unwrap().remove("initialController");
        assert!(serde_json::from_value::<SyncCheckpoint>(missing).unwrap_err().to_string().contains("initialController"));
        let mut guest = WasmGame::new();
        guest.initialize_empty_match(vec!["Carol".into(), "Dan".into()], 20, 2);
        guest.registry.register(definition);
        let before = guest.game.object_ids_in_deterministic_order();
        let mut legacy = checkpoint.clone();
        legacy.version = 1;
        assert!(guest.apply_sync_checkpoint(legacy).unwrap_err().contains("unsupported checkpoint version"));
        let mut invalid = checkpoint;
        invalid.objects[0].initial_controller = 255;
        assert!(guest.apply_sync_checkpoint(invalid).unwrap_err().contains("invalid initial controller"));
        assert_eq!(guest.game.players[0].name, "Carol");
        assert_eq!(guest.game.object_ids_in_deterministic_order(), before);
    }

    #[test]
    fn sync_checkpoint_preserves_registered_color_effect_and_its_turn_anchor() {
        let _id_counter_guard = crate::test_id_counter_guard();
        let alice = PlayerId::from_index(0);
        let mut host = WasmGame::new();
        host.initialize_empty_match(vec!["Alice".into(), "Bob".into()], 20, 1);
        let definition = CardDefinition::new(ironsmith::CardBuilder::new(CardId::new(), "Registered color checkpoint fixture")
            .card_types(vec![CardType::Artifact]).build());
        host.registry.register(definition.clone());
        let permanent = host.game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        host.game.effect_store.continuous_effects.add_effect(ironsmith::continuous::ContinuousEffect::from_resolution(
            permanent, alice, vec![permanent], ironsmith::continuous::Modification::SetColors(ColorSet::RED))
            .until(ironsmith::effect::Until::EndOfTurn)
            .with_expires_end_of_turn(host.game.turn.turn_number));
        host.game.refresh_continuous_state().expect("registered color completes");
        assert_eq!(host.game.current_colors(permanent), Some(ColorSet::RED));
        let expected_effects = host.game.effect_store.continuous_effects.registered_state();
        let checkpoint = host.build_sync_checkpoint();
        let mut guest = WasmGame::new();
        guest.registry.register(definition);
        guest.apply_sync_checkpoint(checkpoint).expect("color effect checkpoint imports");
        assert_eq!(guest.game.current_colors(permanent), Some(ColorSet::RED),
            "successful import cannot silently discard a registered characteristic effect");
        assert_eq!(guest.game.effect_store.continuous_effects.registered_state(), expected_effects,
            "effect payload, identity, captured controller and duration must survive import");
        for peer in [&mut host, &mut guest] {
            peer.game.effect_store.continuous_effects.remove_effect(expected_effects.effects[0].id);
            peer.game.refresh_continuous_state().expect("effect removal completes");
            assert_eq!(peer.game.current_colors(permanent), Some(ColorSet::COLORLESS));
        }
    }

    #[test]
    fn sync_checkpoint_rejects_unencoded_control_effect_instead_of_inventing_assignment() {
        let _id_counter_guard = crate::test_id_counter_guard();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let mut host = WasmGame::new();
        host.initialize_empty_match(vec!["Alice".into(), "Bob".into()], 20, 1);
        let definition = CardDefinition::new(ironsmith::CardBuilder::new(CardId::new(), "Unencoded control checkpoint fixture")
            .card_types(vec![CardType::Artifact]).build());
        host.registry.register(definition.clone());
        let permanent = host.game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        host.game.stage_initial_controller_for_assembly(permanent, bob);
        host.game.effect_store.continuous_effects.add_effect(ironsmith::continuous::ContinuousEffect::gain_control(
            permanent, alice, permanent, alice));
        host.game.refresh_continuous_state().expect("captured control completes");
        assert_eq!(host.game.current_controller(permanent), Some(alice));
        // This negative case deliberately supplies the metadata-only carrier;
        // full checkpoints now include the actual registered control effect.
        let checkpoint = host.try_build_sync_checkpoint_metadata().unwrap();
        let mut guest = WasmGame::new();
        guest.initialize_empty_match(vec!["Carol".into(), "Dan".into()], 20, 2);
        guest.registry.register(definition);
        let error = guest.apply_sync_checkpoint(checkpoint).expect_err("missing actual effect cannot be inferred from controller");
        assert!(error.contains("actual control effects are required"), "{error}");
        assert_eq!(guest.game.players[0].name, "Carol");
        assert!(guest.game.object_ids_in_deterministic_order().is_empty());
        assert!(guest.game.effect_store.continuous_effects.effects().is_empty());
    }

    #[test]
    fn sync_checkpoint_preserves_initial_control_without_inventing_control_effects() {
        let _id_counter_guard = crate::test_id_counter_guard();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let mut host = WasmGame::new();
        host.initialize_empty_match(vec!["Alice".into(), "Bob".into()], 20, 1);
        let definition = CardDefinition::new(
            ironsmith::CardBuilder::new(CardId::new(), "Initial control checkpoint fixture")
                .card_types(vec![CardType::Artifact]).build());
        host.registry.register(definition.clone());
        let permanent = host.game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        host.game.stage_initial_controller_for_assembly(permanent, bob);
        host.game.refresh_continuous_state().expect("initial-control graph is finite");
        assert_eq!(host.game.current_controller(permanent), Some(bob));
        assert_eq!(host.game.object(permanent).unwrap().initial_controller, bob);
        assert!(host.game.effect_store.continuous_effects.effects().is_empty());
        let checkpoint = host.build_sync_checkpoint();
        let mut guest = WasmGame::new();
        guest.registry.register(definition);
        guest.apply_sync_checkpoint(checkpoint).expect("initial-control checkpoint imports");
        assert_eq!(guest.game.object(permanent).unwrap().owner, alice);
        assert_eq!(guest.game.current_controller(permanent), Some(bob));
        assert_eq!(guest.game.object(permanent).unwrap().initial_controller, bob,
            "checkpoint preserves initial control as a fact, independently of ownership");
        assert!(guest.game.effect_store.continuous_effects.effects().is_empty(),
            "initial control cannot be reconstructed as an ordinary control modification");
        assert_eq!(guest.game.object(permanent).unwrap().zone, Zone::Battlefield);
    }

    #[test]
    fn sync_checkpoint_preserves_structured_token_abilities_without_text_reparsing() {
        let _guard = crate::test_id_counter_guard();
        let alice = PlayerId::from_index(0);
        let mut host = WasmGame::new();
        host.initialize_empty_match(vec!["Alice".into(), "Bob".into()], 20, 1);
        let definition = ironsmith::cards::builders::CardDefinitionBuilder::new(
            CardId::new(), "Structured checkpoint token")
            .token()
            .card_types(vec![CardType::Creature])
            .power_toughness(ironsmith::card::PowerToughness::fixed(1, 1))
            .with_ability(ironsmith::Ability::static_ability(
                ironsmith::static_abilities::StaticAbility::flying()))
            .with_ability(ironsmith::Ability::mana(ironsmith::TotalCost::free(),
                vec![ironsmith::mana::ManaSymbol::Blue]))
            .build();
        host.registry.register(definition.clone());
        let token = host.game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        // Display text is deliberately not a program: checkpoint correctness
        // must preserve the structured ability graph and its choices/costs.
        host.game.object_mut(token).unwrap().compiled_card_text = "Display only".into();
        assert_eq!(host.game.object(token).unwrap().abilities.len(), 2);
        let checkpoint: SyncCheckpoint = serde_json::from_value(
            serde_json::to_value(host.build_sync_checkpoint()).unwrap()).unwrap();
        let mut guest = WasmGame::new();
        guest.apply_sync_checkpoint(checkpoint).expect("token checkpoint imports");
        let restored = guest.game.object(token).unwrap();
        assert_eq!(restored.kind, ironsmith::object::ObjectKind::Token);
        assert_eq!(restored.abilities.len(), 2,
            "successful checkpoint import must retain both static and activated token abilities");
        assert_eq!(restored.compiled_card_text.as_ref(), "Display only");
        assert!(matches!(&restored.abilities[0].kind, ironsmith::ability::AbilityKind::Static(_)));
        assert!(matches!(&restored.abilities[1].kind, ironsmith::ability::AbilityKind::Activated(_)));
    }

    #[test]
    fn sync_checkpoint_preserves_granted_public_zone_permissions_and_expiry() {
        let _guard = crate::test_id_counter_guard();
        let alice = PlayerId::from_index(0);
        let mut host = WasmGame::new();
        host.initialize_empty_match(vec!["Alice".into(), "Bob".into()], 20, 1);
        let definition = CardDefinition::new(ironsmith::CardBuilder::new(
            CardId::new(), "Granted checkpoint spell")
            .card_types(vec![CardType::Sorcery]).build());
        host.registry.register(definition.clone());
        let card = host.game.create_object_from_definition(&definition, alice, Zone::Exile);
        host.game.effect_store.grant_registry.grant_play_from_to_card(
            card, Zone::Exile, alice, Default::default(),
            ironsmith::grant_registry::GrantSource::Effect {
                source_id: card, expires_end_of_turn: host.game.turn.turn_number,
            });
        let original = host.game.effect_store.grant_registry.granted_play_from_for_card(
            &host.game, card, Zone::Exile, alice);
        assert_eq!(original.len(), 1);
        let checkpoint: SyncCheckpoint = serde_json::from_value(
            serde_json::to_value(host.build_sync_checkpoint()).unwrap()).unwrap();
        let mut guest = WasmGame::new();
        guest.registry.register(definition);
        guest.apply_sync_checkpoint(checkpoint).expect("permission checkpoint imports");
        let imported = guest.game.effect_store.grant_registry.granted_play_from_for_card(
            &guest.game, card, Zone::Exile, alice);
        assert_eq!(imported.len(), 1,
            "successful import must not remove a live public-zone cast permission");
        assert_eq!(imported[0].permission_identity, original[0].permission_identity);
        guest.game.turn.turn_number += 1;
        assert!(guest.game.effect_store.grant_registry.granted_play_from_for_card(
            &guest.game, card, Zone::Exile, alice).is_empty(), "expiry remains effective");
        assert_eq!(host.game.effect_store.grant_registry.granted_play_from_for_card(
            &host.game, card, Zone::Exile, alice).len(), 1, "receiver expiry does not mutate sender");
    }

    #[test]
    fn sync_checkpoint_preserves_delayed_program_at_settled_priority() {
        let _guard = crate::test_id_counter_guard();
        let mut source = WasmGame::new();
        source.initialize_empty_match(vec!["Alice".into(), "Bob".into()], 20, 1);
        let alice = PlayerId::from_index(0);
        source.game.effect_store.delayed_triggers.push(test_delayed_registration(source.game.turn.turn_number, alice));
        assert!(source.game.stack.is_empty());
        let checkpoint = source.build_sync_checkpoint();
        let mut receiver = WasmGame::new();
        receiver.apply_sync_checkpoint(checkpoint).unwrap();
        assert_eq!(receiver.game.effect_store.delayed_triggers.len(), 1,
            "settled priority must retain a pending delayed registration");
        let retained = &receiver.game.effect_store.delayed_triggers[0];
        assert!(retained.one_shot);
        assert_eq!(retained.x_value, Some(3));
        assert_eq!(retained.controller, alice);
        assert_eq!(retained.effects.all_effects().len(), 1);
    }

    #[test]
    fn sync_checkpoint_preserves_regeneration_shields_and_departed_turn_counts() {
        let _guard = crate::test_id_counter_guard();
        let alice = PlayerId::from_index(0);
        let mut host = WasmGame::new();
        host.initialize_empty_match(vec!["Alice".into(), "Bob".into()], 20, 1);
        let definition = CardDefinition::new(ironsmith::CardBuilder::new(
            CardId::new(), "Regeneration checkpoint permanent")
            .card_types(vec![CardType::Artifact]).build());
        host.registry.register(definition.clone());
        let permanent = host.game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let departed = host.game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        host.game.add_regeneration_shield(permanent, 2);
        host.game.add_regeneration_shield(departed, 1);
        assert!(host.game.use_regeneration_shield(permanent));
        assert!(host.game.use_regeneration_shield(departed));
        host.game.move_object(departed, Zone::Graveyard,
            ironsmith::events::cause::EventCause::effect()).unwrap();
        assert_eq!(host.game.regenerated_this_turn_count(departed), 1);
        let checkpoint: SyncCheckpoint = serde_json::from_value(
            serde_json::to_value(host.build_sync_checkpoint()).unwrap()).unwrap();
        let mut guest = WasmGame::new();
        guest.registry.register(definition);
        guest.apply_sync_checkpoint(checkpoint.clone()).unwrap();
        assert_eq!(guest.game.regeneration_state(), host.game.regeneration_state());
        assert!(guest.game.use_regeneration_shield(permanent));
        assert!(!guest.game.use_regeneration_shield(permanent));
        assert_eq!(guest.game.regenerated_this_turn_count(permanent), 2);
        assert_eq!(guest.game.regeneration_shield_count(permanent), 0);
        assert_eq!(host.game.regeneration_shield_count(permanent), 1,
            "receiver consumption must not mutate sender");
        assert_eq!(guest.game.regenerated_this_turn_count(departed), 1,
            "departed incarnation history remains available until cleanup");
        let before = guest.game.regeneration_state();
        let mut malformed = checkpoint;
        malformed.rules.regeneration_shields.push((permanent.0, 9));
        assert!(guest.apply_sync_checkpoint(malformed).unwrap_err().contains("duplicate regeneration"));
        assert_eq!(guest.game.regeneration_state(), before,
            "invalid checkpoint must not partially replace either count table");
    }

    fn sync_checkpoint_preserves_zero_counter_saga_entry_completion() {
        let _id_counter_guard = crate::test_id_counter_guard();
        let alice = PlayerId::from_index(0);
        let mut host = WasmGame::new();
        host.initialize_empty_match(vec!["Alice".into(), "Bob".into()], 20, 1);
        let mut definition = CardDefinition::new(
            ironsmith::CardBuilder::new(CardId::new(), "Checkpoint Saga Fixture")
                .card_types(vec![CardType::Enchantment])
                .subtypes(vec![ironsmith::types::Subtype::Saga])
                .build(),
        );
        definition.abilities.push(ironsmith::Ability::triggered(
            ironsmith::triggers::Trigger::saga_chapter(vec![1]),
            vec![ironsmith::Effect::gain_life(1)],
        ));
        host.registry.register(definition.clone());
        let saga = host.game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        host.game.mark_saga_entry_lore_processed(saga);
        assert_eq!(host.game.counter_count(saga, ironsmith::CounterType::Lore), 0);
        let checkpoint = host.build_sync_checkpoint();
        let stored = checkpoint.objects.iter().find(|object| object.id == saga.0).unwrap();
        assert!(stored.saga_entry_lore_processed);
        // Older checkpoint object shapes remain readable with explicit default state.
        let mut legacy = serde_json::to_value(stored).unwrap();
        legacy.as_object_mut().unwrap().remove("sagaEntryLoreProcessed");
        let legacy: SyncObject = serde_json::from_value(legacy).unwrap();
        assert!(!legacy.saga_entry_lore_processed);
        let mut guest = WasmGame::new();
        guest.registry.register(definition.clone());
        guest.apply_sync_checkpoint(checkpoint).unwrap();
        assert!(guest.game.has_processed_saga_entry_lore(saga));
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
        ironsmith::game_loop::handle_saga_enters_battlefield(&mut guest.game, saga, &mut queue, &mut dm).unwrap();
        assert_eq!(guest.game.counter_count(saga, ironsmith::CounterType::Lore), 0);
        assert!(queue.is_empty());
        let exported = guest.build_sync_checkpoint();
        assert!(exported.objects.iter().find(|object| object.id == saga.0).unwrap().saga_entry_lore_processed);
        // Positive control: this registered fixture really has a chapter, so
        // losing the completion flag would place lore and queue that chapter.
        let fresh = guest.game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        assert!(!guest.game.has_processed_saga_entry_lore(fresh));
        ironsmith::game_loop::handle_saga_enters_battlefield(&mut guest.game, fresh, &mut queue, &mut dm).unwrap();
        assert_eq!(guest.game.counter_count(fresh, ironsmith::CounterType::Lore), 1);
        assert_eq!(queue.entries.len(), 1);

    }

    #[test]
    fn sync_checkpoint_preserves_counter_ability_occurrences_and_allocator() {
        let _id_counter_guard = crate::test_id_counter_guard();
        let alice = PlayerId::from_index(0);
        let mut host = WasmGame::new();
        host.initialize_empty_match(vec!["Alice".into(), "Bob".into()], 20, 1);
        let definition = CardDefinition::new(
            ironsmith::CardBuilder::new(CardId::new(), "Counter Checkpoint Fixture")
                .card_types(vec![CardType::Creature]).build(),
        );
        host.registry.register(definition.clone());
        let source = host.game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        for kind in [ironsmith::CounterType::Flying, ironsmith::CounterType::Named("exalted".into()), ironsmith::CounterType::Named("shadow".into())] {
            let object = host.game.object_mut(source).expect("counter recipient exists");
            object.add_counters(kind, 2);
            assert_eq!(object.remove_counters(kind, 1), 1);
            object.add_counters(kind, 1);
        }
        let origins = |state: &ironsmith::GameState| {
            let chars = state.calculated_characteristics(source).expect("counter recipient exists");
            chars.abilities.iter().enumerate().filter_map(|(slot, _)| {
                let origin = chars.abilities.origin(slot).expect("ability origin is paired");
                matches!(origin, ironsmith::continuous::AbilityOrigin::Counter { .. }).then(|| origin.clone())
            }).collect::<Vec<_>>()
        };
        let before = origins(&host.game);
        assert_eq!(before.len(), 6, "two flying, two shadow abilities and two exalted triggers survive");
        let encoded = serde_json::to_vec(&host.build_sync_checkpoint()).expect("checkpoint serializes");
        let checkpoint: SyncCheckpoint = serde_json::from_slice(&encoded).expect("checkpoint deserializes");
        let mut guest = WasmGame::new();
        guest.registry.register(definition.clone());
        guest.apply_sync_checkpoint(checkpoint).expect("checkpoint imports");
        assert_eq!(origins(&guest.game), before, "import preserves surviving and newly registered occurrences");
        for kind in [ironsmith::CounterType::Flying, ironsmith::CounterType::Named("exalted".into()), ironsmith::CounterType::Named("shadow".into())] {
            for state in [&mut host.game, &mut guest.game] {
                let object = state.object_mut(source).expect("counter recipient exists");
                assert_eq!(object.remove_counters(kind, 2), 2);
                object.add_counters(kind, 1);
            }
        }
        let after = origins(&host.game);
        assert_eq!(after.len(), 3);
        assert!(after.iter().all(|origin| !before.contains(origin)), "re-additions never reuse removed occurrences");
        assert_eq!(origins(&guest.game), after, "import also retains the next registration identity");
    }

    #[test]
    fn sync_checkpoint_preserves_counter_timestamps_against_printed_ability_loss() {
        let _id_counter_guard = crate::test_id_counter_guard();
        for same_kind in [false, true] {
            let alice = PlayerId::from_index(0);
            let mut host = WasmGame::new();
            host.initialize_empty_match(vec!["Alice".into(), "Bob".into()], 20, 1);
            let recipient_definition = CardDefinition::new(
                ironsmith::CardBuilder::new(CardId::new(), "Timestamp Counter Recipient")
                    .card_types(vec![CardType::Creature]).build(),
            );
            let mut loss_definition = CardDefinition::new(
                ironsmith::CardBuilder::new(CardId::new(), "Timestamp Ability Loss")
                    .card_types(vec![CardType::Enchantment]).build(),
            );
            loss_definition.abilities.push(ironsmith::Ability::static_ability(
                ironsmith::static_abilities::StaticAbility::remove_all_abilities(
                    ironsmith::target::ObjectFilter::creature(),
                ),
            ));
            host.registry.register(recipient_definition.clone());
            host.registry.register(loss_definition.clone());
            let source = host.game.create_object_from_definition(&recipient_definition, alice, Zone::Battlefield);
            let ability_counts = |state: &GameState| {
                let chars = state.calculated_characteristics(source).expect("recipient has current characteristics");
                let flying = chars.static_abilities.iter().filter(|ability|
                    ability.id() == ironsmith::static_abilities::StaticAbilityId::Flying).count();
                let triggered = chars.abilities.iter().filter(|ability|
                    matches!(ability.kind, ironsmith::ability::AbilityKind::Triggered(_))).count();
                (flying, triggered)
            };
            host.game.add_counters(source, ironsmith::CounterType::Flying, 1)
                .expect("positive flying placement records event and timestamp");
            assert_eq!(ability_counts(&host.game), (1, 0));
            let loss = host.game.create_object_from_definition(&loss_definition, alice, Zone::Battlefield);
            assert_eq!(ability_counts(&host.game), (0, 0), "later printed ability loss removes older counter ability");
            let later_kind = if same_kind { ironsmith::CounterType::Flying }
                else { ironsmith::CounterType::Named("exalted".into()) };
            host.game.add_counters(source, later_kind, 1)
                .expect("later placement rebases only that counter kind");
            let expected = if same_kind { (2, 0) } else { (0, 1) };
            assert_eq!(ability_counts(&host.game), expected, "live game establishes shared-kind ordering");
            let timestamps = host.game.effect_store.continuous_effects.counter_timestamps_snapshot();
            let entries = host.game.effect_store.continuous_effects.object_entry_timestamps_snapshot();
            let clock = host.game.effect_store.continuous_effects.current_timestamp();
            let encoded = serde_json::to_vec(&host.build_sync_checkpoint()).expect("timestamp checkpoint serializes");
            let checkpoint: SyncCheckpoint = serde_json::from_slice(&encoded).expect("timestamp checkpoint deserializes");
            let mut guest = WasmGame::new();
            guest.registry.register(recipient_definition.clone());
            guest.registry.register(loss_definition.clone());
            guest.apply_sync_checkpoint(checkpoint).expect("timestamp checkpoint imports");
            assert!(guest.game.object(loss).is_some(), "the printed loss source survives import");
            assert_eq!(ability_counts(&guest.game), expected,
                "counter timestamp ordering against a surviving printed source must survive JSON import");
            assert_eq!(guest.game.effect_store.continuous_effects.counter_timestamps_snapshot(), timestamps);
            assert_eq!(guest.game.effect_store.continuous_effects.object_entry_timestamps_snapshot(), entries);
            assert_eq!(guest.game.effect_store.continuous_effects.current_timestamp(), clock,
                "the timestamp allocator cannot reset behind imported effects");
            for state in [&mut host.game, &mut guest.game] {
                state.add_counters(source, ironsmith::CounterType::Flying, 1)
                    .expect("future placement gets a new shared-kind timestamp");
            }
            assert_eq!(ability_counts(&host.game), ability_counts(&guest.game));
            assert_eq!(guest.game.effect_store.continuous_effects.counter_timestamps_snapshot(),
                host.game.effect_store.continuous_effects.counter_timestamps_snapshot());
        }
    }


fn assert_failed_checkpoint_import_preserves_live_runtime(late_chronology_error: bool) {
    let _id_counter_guard = crate::test_id_counter_guard();
    let alice = PlayerId::from_index(0);
    let mut host = WasmGame::new();
    host.initialize_empty_match(vec!["Alice".into(), "Bob".into()], 20, 9);
    let definition = CardDefinition::new(
        ironsmith::CardBuilder::new(CardId::new(), "Import Transaction Recipient")
            .card_types(vec![CardType::Creature])
            .build(),
    );
    host.registry.register(definition.clone());
    let source = host
        .game
        .create_object_from_definition(&definition, alice, Zone::Battlefield);
    host.game
        .player_mut(alice)
        .expect("live player exists")
        .life = 27;
    host.game
        .add_counters(source, ironsmith::CounterType::Flying, 2)
        .expect("counter placement succeeds");
    let effect = host.game.effect_store.continuous_effects.add_effect(
        ironsmith::continuous::ContinuousEffect::new(
            source,
            alice,
            ironsmith::continuous::EffectTarget::Specific(source),
            ironsmith::continuous::Modification::RemoveAllAbilities,
        ),
    );
    assert!(
        host.game
            .calculated_characteristics(source)
            .expect("live source exists")
            .abilities
            .is_empty(),
        "live resolution effect really removes counter abilities"
    );
    host.snapshot_serial = 88;
    host.loaded_decks = vec![vec!["Original Deck Entry".into()]];
    host.pending_action_checkpoint = Some(host.capture_replay_checkpoint());
    host.pending_replay_action = Some(PendingReplayAction {
        checkpoint: host.capture_replay_checkpoint(),
        root: ReplayRoot::Advance,
        nested_answers: vec![ReplayDecisionAnswer::Number(7)],
    });
    host.last_snapshot_perf = Some(SnapshotPerfMetrics {
        snapshot_id: 88,
        ..Default::default()
    });
    let savepoint = host
        .create_runtime_savepoint()
        .expect("retained branch created");
    let before =
        serde_json::to_value(host.build_sync_checkpoint()).expect("live checkpoint encodes");
    let before_ids = snapshot_id_counters();
    let answers = format!(
        "{:?}",
        host.pending_replay_action
            .as_ref()
            .expect("pending replay exists")
            .nested_answers
    );
    let mut incoming: SyncCheckpoint =
        serde_json::from_value(before.clone()).expect("checkpoint decodes");
    incoming.players[0].life = 3;
    incoming.snapshot_serial = 500;
    let valid_incoming = incoming.clone();
    let expected_error = if late_chronology_error {
        incoming
            .continuous_timestamps
            .as_mut()
            .expect("chronology exported")
            .current_timestamp = u64::MAX;
        "invalid continuous chronology"
    } else {
        let object = incoming
            .objects
            .iter_mut()
            .find(|object| object.id == source.0)
            .expect("source exported");
        object
            .counter_ability_state
            .as_mut()
            .expect("counter identity exported")
            .next_serial
            .push(0);
        "invalid counter registrations"
    };
    let error = host
        .apply_sync_checkpoint(incoming)
        .expect_err("malformed import fails");
    assert!(
        error.contains(expected_error),
        "specific validation failure: {error}"
    );
    assert_eq!(
        host.game.player(alice).expect("live player retained").life,
        27,
        "failed import must not replace live player state"
    );
    assert_eq!(
        serde_json::to_value(host.build_sync_checkpoint()).expect("live checkpoint encodes"),
        before,
        "failed import must leave all exported state intact"
    );
    assert!(
        host.game
            .effect_store
            .continuous_effects
            .effects()
            .iter()
            .any(|e| e.id == effect),
        "runtime-only resolution effects must survive failure"
    );
    assert!(
        host.game
            .calculated_characteristics(source)
            .expect("original source retained")
            .abilities
            .is_empty()
    );
    assert_eq!(
        host.loaded_decks,
        vec![vec!["Original Deck Entry".to_string()]]
    );
    assert!(
        host.pending_action_checkpoint.is_some(),
        "live action rollback point retained"
    );
    assert_eq!(
        format!(
            "{:?}",
            host.pending_replay_action
                .as_ref()
                .expect("pending replay retained")
                .nested_answers
        ),
        answers
    );
    assert_eq!(
        host.last_snapshot_perf
            .as_ref()
            .expect("branch diagnostic retained")
            .snapshot_id,
        88
    );
    assert!(
        host.runtime_savepoints.contains_key(&savepoint),
        "existing branch handle retained"
    );
    let after_ids = snapshot_id_counters();
    assert_eq!(
        (after_ids.player, after_ids.object, after_ids.card),
        (before_ids.player, before_ids.object, before_ids.card),
        "no IDs consumed on failed import"
    );
    host.apply_sync_checkpoint(valid_incoming)
        .expect("corrected checkpoint imports");
    assert_eq!(
        host.game
            .player(alice)
            .expect("imported player exists")
            .life,
        3
    );
    assert_eq!(host.snapshot_serial, 500);
    assert!(
        host.pending_action_checkpoint.is_none(),
        "successful import installs incoming runtime"
    );
    assert!(host.pending_replay_action.is_none());
    assert_eq!(
        host.game
            .counter_count(source, ironsmith::CounterType::Flying),
        2
    );
}

#[test]
fn sync_checkpoint_counter_validation_failure_preserves_live_runtime() {
    assert_failed_checkpoint_import_preserves_live_runtime(false);
}

#[test]
fn sync_checkpoint_late_chronology_failure_preserves_live_runtime() {
    assert_failed_checkpoint_import_preserves_live_runtime(true);
}


fn actual_departed_hidden_card_checkpoint() -> (SyncCheckpoint, CardDefinition) {
    let mut host = WasmGame::new();
    host.initialize_empty_match(vec!["Alice".into(), "Bob".into(), "Carol".into()], 20, 17);
    let bob = PlayerId::from_index(1);
    let definition = CardDefinition::new(
        ironsmith::CardBuilder::new(CardId::new(), "Departing Hidden Import Fixture")
            .card_types(vec![CardType::Creature])
            .build(),
    );
    host.registry.register(definition.clone());
    let card = host
        .game
        .create_object_from_definition(&definition, bob, Zone::Hand);
    host.game.set_hidden_card_info(
        card,
        HiddenCardInfo {
            owner: bob,
            zone: Zone::Hand,
            slot: 17,
            commitment: "departed-commitment".into(),
            origin_slot: Some(2),
            origin_commitment: Some("original-commitment".into()),
            public_slot: Some(5),
            public_commitment: Some("public-commitment".into()),
        },
    );
    assert!(host.game.leave_game(bob), "real departure completes");
    assert!(
        host.game.object(card).is_none(),
        "departed card is no longer live"
    );
    assert_eq!(
        host.game.departed_hidden_cards().len(),
        1,
        "departure records disclosure history"
    );
    let checkpoint = host.build_sync_checkpoint();
    assert_eq!(checkpoint.rules.departed_hidden_cards.len(), 1);
    assert_eq!(
        checkpoint.rules.departed_hidden_cards[0].name.as_deref(),
        Some("Departing Hidden Import Fixture")
    );
    (checkpoint, definition)
}

#[test]
fn sync_checkpoint_invalid_departed_hidden_zone_is_an_error_not_a_dropped_record() {
    let _id_counter_guard = crate::test_id_counter_guard();
    let (mut checkpoint, definition) = actual_departed_hidden_card_checkpoint();
    checkpoint.rules.departed_hidden_cards[0].zone = "unrecognized-hidden-zone".into();
    let mut guest = WasmGame::new();
    guest.initialize_empty_match(vec!["Original Alice".into(), "Original Bob".into()], 27, 99);
    guest.registry.register(definition);
    let before =
        serde_json::to_value(guest.build_sync_checkpoint()).expect("existing game encodes");
    let error = guest
        .apply_sync_checkpoint(checkpoint)
        .expect_err("invalid record must fail import");
    assert!(
        error.contains("unknown checkpoint zone"),
        "specific decode error: {error}"
    );
    assert_eq!(
        serde_json::to_value(guest.build_sync_checkpoint()).expect("retained game encodes"),
        before,
        "malformed hidden history must not replace the existing runtime"
    );
}

#[test]
fn sync_checkpoint_departed_hidden_known_and_redacted_records_preserve_disclosure_history() {
    let _id_counter_guard = crate::test_id_counter_guard();
    let (checkpoint, definition) = actual_departed_hidden_card_checkpoint();
    for redacted in [false, true] {
        let mut incoming = checkpoint.clone();
        if redacted {
            incoming.rules.departed_hidden_cards[0].name = None;
        }
        let expected = serde_json::to_value(&incoming.rules.departed_hidden_cards)
            .expect("incoming history encodes");
        let mut guest = WasmGame::new();
        guest.registry.register(definition.clone());
        guest
            .apply_sync_checkpoint(incoming)
            .expect("valid history imports");
        let history = guest.game.departed_hidden_cards();
        assert_eq!(
            history.len(),
            1,
            "disclosure record retained for either visibility"
        );
        assert_eq!(
            history[0].object.card.is_none(),
            redacted,
            "only redaction creates an anonymous card"
        );
        let exported = guest.build_sync_checkpoint();
        assert_eq!(
            serde_json::to_value(exported.rules.departed_hidden_cards)
                .expect("restored history encodes"),
            expected,
            "all identity and commitment metadata retained"
        );
    }
}

    #[test]
    fn sync_checkpoint_unknown_departed_hidden_identity_is_an_error_not_anonymous() {
        let _id_counter_guard = crate::test_id_counter_guard();
        let (mut checkpoint, definition) = actual_departed_hidden_card_checkpoint();
        checkpoint.rules.departed_hidden_cards[0].name = Some("Unknown Departed Checkpoint Identity".into());
        let mut guest = WasmGame::new();
        guest.initialize_empty_match(vec!["Original Alice".into(), "Original Bob".into()], 27, 99);
        guest.registry.register(definition);
        let before = serde_json::to_value(guest.build_sync_checkpoint()).expect("existing game encodes");
        let error = guest.apply_sync_checkpoint(checkpoint).expect_err("named card must not become anonymous");
        assert!(error.contains("invalid departed hidden-card identity"), "specific identity failure: {error}");
        assert_eq!(serde_json::to_value(guest.build_sync_checkpoint()).expect("retained game encodes"), before);
    }


#[test]
fn sync_checkpoint_import_preserves_session_catalog_allocator_and_restores_gameplay_cursor() {
    let _id_counter_guard = crate::test_id_counter_guard();
    let alice = PlayerId::from_index(0);
    let mut host = WasmGame::new();
    host.initialize_empty_match(vec!["Alice".into(), "Bob".into()], 20, 21);
    let original = CardDefinition::new(
        ironsmith::CardBuilder::new(CardId::new(), "Original Catalog Entry")
            .card_types(vec![CardType::Land])
            .build(),
    );
    host.registry.register(original.clone());
    host.game
        .create_object_from_definition(&original, alice, Zone::Hand);
    let checkpoint = host.build_sync_checkpoint();
    let expected_gameplay_cursor = checkpoint.id_counters.object;
    let retained = CardDefinition::new(
        ironsmith::CardBuilder::new(CardId::new(), "Retained Catalog Entry")
            .card_types(vec![CardType::Land])
            .build(),
    );
    let retained_id = retained.card.id;
    host.registry.register(retained);
    let discarded = host
        .game
        .create_object_from_definition(&original, alice, Zone::Hand);
    assert!(host.game.next_object_id_counter() > expected_gameplay_cursor);
    host.apply_sync_checkpoint(checkpoint)
        .expect("valid checkpoint imports");
    assert_eq!(
        host.find_card_definition("Retained Catalog Entry")
            .expect("session catalog entry survives import")
            .card
            .id,
        retained_id
    );
    assert!(
        host.game.object(discarded).is_none(),
        "incoming game replaces the discarded branch"
    );
    assert_eq!(
        host.game.next_object_id_counter(),
        expected_gameplay_cursor,
        "game-local cursor follows the authoritative checkpoint"
    );
    assert_eq!(snapshot_id_counters().object, expected_gameplay_cursor,
        "global compatibility cursor also reflects the imported checkpoint");
    assert!(
        CardId::new().0 > retained_id.0,
        "new definitions must not reuse identities already retained by the session catalog"
    );
}

fn assert_sync_checkpoint_preserves_counter_kind_identity(kinds: &[(ironsmith::CounterType, u32)]) {
    let _id_counter_guard = crate::test_id_counter_guard();
    let alice = PlayerId::from_index(0);
    let mut host = WasmGame::new();
    host.initialize_empty_match(vec!["Alice".into(), "Bob".into()], 20, 1);
    let definition = CardDefinition::new(
        ironsmith::CardBuilder::new(CardId::new(), "Counter Kind Wire Fixture")
            .card_types(vec![CardType::Creature])
            .build(),
    );
    host.registry.register(definition.clone());
    let source = host
        .game
        .create_object_from_definition(&definition, alice, Zone::Battlefield);
    for &(kind, amount) in kinds {
        host.game
            .add_counters(source, kind, amount)
            .expect("authored counter placement succeeds");
    }
    let expected_counts = host
        .game
        .object(source)
        .expect("host object")
        .counters
        .counts()
        .clone();
    let expected_origins = host
        .game
        .object(source)
        .expect("host object")
        .counters
        .ability_state();
    let expected_timestamps = host
        .game
        .effect_store
        .continuous_effects
        .counter_timestamps_snapshot();
    let checkpoint =
        serde_json::to_value(host.build_sync_checkpoint()).expect("checkpoint encodes");
    let checkpoint: SyncCheckpoint =
        serde_json::from_value(checkpoint).expect("checkpoint decodes");
    let mut guest = WasmGame::new();
    guest.registry.register(definition);
    guest
        .apply_sync_checkpoint(checkpoint)
        .expect("valid authored counter identities must import");
    let restored = guest.game.object(source).expect("restored source");
    assert_eq!(
        restored.counters.counts(),
        &expected_counts,
        "wire transport must preserve exact counter kinds and counts"
    );
    assert_eq!(
        restored.counters.ability_state(),
        expected_origins,
        "wire transport must preserve registration origins"
    );
    assert_eq!(
        guest
            .game
            .effect_store
            .continuous_effects
            .counter_timestamps_snapshot(),
        expected_timestamps,
        "counter chronology keys must retain the same counter identity"
    );
}

#[test]
fn sync_checkpoint_preserves_named_keyword_counter_case_identity() {
    assert_sync_checkpoint_preserves_counter_kind_identity(&[
        (ironsmith::CounterType::Named("Exalted".into()), 2),
        (ironsmith::CounterType::Named("SHADOW".into()), 1),
    ]);
}

#[test]
fn sync_checkpoint_preserves_named_counter_distinct_from_builtin_description() {
    assert_sync_checkpoint_preserves_counter_kind_identity(&[
        (ironsmith::CounterType::Flying, 2),
        (ironsmith::CounterType::Named("flying".into()), 4),
    ]);
}

#[test]
fn sync_checkpoint_preserves_custom_counter_case_and_whitespace_identity() {
    assert_sync_checkpoint_preserves_counter_kind_identity(&[(
        ironsmith::CounterType::Named(" Mixed Custom ".into()),
        3,
    )]);
}

#[test]
fn sync_checkpoint_counter_codec_preserves_every_builtin_kind() {
    use ironsmith::CounterType;
    let kinds = [
        CounterType::PlusOnePlusOne,
        CounterType::MinusOneMinusOne,
        CounterType::PlusOnePlusZero,
        CounterType::PlusZeroPlusOne,
        CounterType::PlusOnePlusTwo,
        CounterType::PlusTwoPlusTwo,
        CounterType::MinusZeroMinusOne,
        CounterType::MinusZeroMinusTwo,
        CounterType::MinusTwoMinusOne,
        CounterType::MinusTwoMinusTwo,
        CounterType::Deathtouch,
        CounterType::Decayed,
        CounterType::DoubleStrike,
        CounterType::FirstStrike,
        CounterType::Flying,
        CounterType::Haste,
        CounterType::Hexproof,
        CounterType::Indestructible,
        CounterType::Lifelink,
        CounterType::Menace,
        CounterType::Reach,
        CounterType::Trample,
        CounterType::Vigilance,
        CounterType::Loyalty,
        CounterType::Charge,
        CounterType::Age,
        CounterType::Aim,
        CounterType::Arrow,
        CounterType::Awakening,
        CounterType::Blood,
        CounterType::Brain,
        CounterType::Bounty,
        CounterType::Brick,
        CounterType::Corpse,
        CounterType::Credit,
        CounterType::Crystal,
        CounterType::Cube,
        CounterType::Currency,
        CounterType::Death,
        CounterType::Defense,
        CounterType::Depletion,
        CounterType::Despair,
        CounterType::Devotion,
        CounterType::Divinity,
        CounterType::Doom,
        CounterType::Dream,
        CounterType::Echo,
        CounterType::Egg,
        CounterType::Energy,
        CounterType::Enlightened,
        CounterType::Eon,
        CounterType::Experience,
        CounterType::Eyeball,
        CounterType::Fade,
        CounterType::Fate,
        CounterType::Feather,
        CounterType::Filibuster,
        CounterType::Finality,
        CounterType::Flame,
        CounterType::Flood,
        CounterType::Foreshadow,
        CounterType::Fungus,
        CounterType::Fuse,
        CounterType::Gem,
        CounterType::Glyph,
        CounterType::Gold,
        CounterType::Growth,
        CounterType::Hatchling,
        CounterType::Healing,
        CounterType::Hit,
        CounterType::Hoofprint,
        CounterType::Hour,
        CounterType::Hunger,
        CounterType::Ice,
        CounterType::Incarnation,
        CounterType::Infection,
        CounterType::Intervention,
        CounterType::Isolation,
        CounterType::Javelin,
        CounterType::Ki,
        CounterType::Keyword,
        CounterType::Knowledge,
        CounterType::Level,
        CounterType::Lore,
        CounterType::Luck,
        CounterType::Magnet,
        CounterType::Manifestation,
        CounterType::Mannequin,
        CounterType::Matrix,
        CounterType::Mine,
        CounterType::Mining,
        CounterType::Mire,
        CounterType::Music,
        CounterType::Muster,
        CounterType::Net,
        CounterType::Night,
        CounterType::Oil,
        CounterType::Omen,
        CounterType::Ore,
        CounterType::Page,
        CounterType::Pain,
        CounterType::Paralyzation,
        CounterType::Petal,
        CounterType::Petrification,
        CounterType::Phylactery,
        CounterType::Pin,
        CounterType::Plague,
        CounterType::Plot,
        CounterType::Polyp,
        CounterType::Poison,
        CounterType::Pressure,
        CounterType::Prey,
        CounterType::Pupa,
        CounterType::Quest,
        CounterType::Rad,
        CounterType::Scream,
        CounterType::Shield,
        CounterType::Silver,
        CounterType::Sleep,
        CounterType::Slime,
        CounterType::Slumber,
        CounterType::Soot,
        CounterType::Soul,
        CounterType::Spore,
        CounterType::Storage,
        CounterType::Strife,
        CounterType::Study,
        CounterType::Stun,
        CounterType::Void,
        CounterType::Task,
        CounterType::Theft,
        CounterType::Tide,
        CounterType::Time,
        CounterType::Tower,
        CounterType::Training,
        CounterType::Trap,
        CounterType::Treasure,
        CounterType::Unity,
        CounterType::Velocity,
        CounterType::Verse,
        CounterType::Vitality,
        CounterType::Volatile,
        CounterType::Voyage,
        CounterType::Wage,
        CounterType::Winch,
        CounterType::Wind,
        CounterType::Wish,
    ];
    let mismatches: Vec<_> = kinds
        .into_iter()
        .filter_map(|kind| {
            let decoded = sync_counter_from_name(&sync_counter_kind(kind));
            (decoded != kind).then_some((kind, decoded))
        })
        .collect();
    assert!(
        mismatches.is_empty(),
        "builtin counter kinds changed during wire decoding: {mismatches:?}"
    );
}

#[test]
fn sync_checkpoint_preserves_additional_builtin_counter_identities() {
    assert_sync_checkpoint_preserves_counter_kind_identity(&[
        (ironsmith::CounterType::MinusTwoMinusOne, 1),
        (ironsmith::CounterType::Decayed, 2),
        (ironsmith::CounterType::Defense, 3),
    ]);
}

fn assert_invalid_counter_identity_import_is_atomic(case: &str) {
    let _id_counter_guard = crate::test_id_counter_guard();
    let alice = PlayerId::from_index(0);
    let mut host = WasmGame::new();
    host.initialize_empty_match(vec!["Alice".into(), "Bob".into()], 20, 1);
    let definition = CardDefinition::new(
        ironsmith::CardBuilder::new(CardId::new(), "Counter Wire Validation Fixture")
            .card_types(vec![CardType::Creature])
            .build(),
    );
    host.registry.register(definition.clone());
    let source = host
        .game
        .create_object_from_definition(&definition, alice, Zone::Battlefield);
    host.game
        .add_counters(source, ironsmith::CounterType::Flying, 2)
        .expect("counter placement succeeds");
    host.game.player_mut(alice).expect("live player").life = 27;
    let before = serde_json::to_value(host.build_sync_checkpoint()).expect("live state encodes");
    let mut incoming: SyncCheckpoint =
        serde_json::from_value(before.clone()).expect("incoming decodes");
    incoming.players[0].life = 3;
    let object = incoming
        .objects
        .iter_mut()
        .find(|object| object.id == source.0)
        .expect("counter object exported");
    let expected = match case {
        "count" => {
            object.counters[0].counter_type = Some(ironsmith::CounterType::Reach);
            "counter identity disagrees"
        }
        "origin" => {
            object
                .counter_ability_state
                .as_mut()
                .expect("registrations exported")
                .origins[0]
                .counter_type = Some(ironsmith::CounterType::Reach);
            "counter identity disagrees"
        }
        "duplicate" => {
            object.counters.push(object.counters[0].clone());
            "duplicate counter kind"
        }
        "valid-count-conflict" => {
            object.counters.push(SyncCounter {
                kind: sync_counter_kind(ironsmith::CounterType::PlusOnePlusOne),
                amount: 7,
                counter_type: Some(ironsmith::CounterType::PlusOnePlusOne),
            });
            sync_counters_from_checkpoint(object).expect("modified flat counts are valid");
            "executable counter state"
        }
        "valid-registration-conflict" => {
            object.counter_ability_state.as_mut().expect("registrations exported")
                .next_serial = vec![3];
            sync_counters_from_checkpoint(object).expect("modified flat allocator is valid");
            "executable counter state"
        }
        "chronology" => {
            incoming
                .continuous_timestamps
                .as_mut()
                .expect("chronology exported")
                .typed_counters
                .as_mut()
                .expect("typed chronology exported")[0]
                .1 = ironsmith::CounterType::Reach;
            "typed counter chronology disagrees"
        }
        _ => panic!("unknown fixture case"),
    };
    let error = host
        .apply_sync_checkpoint(incoming)
        .expect_err("malformed identity must reject import");
    assert!(error.contains(expected), "wrong error for {case}: {error}");
    assert_eq!(
        serde_json::to_value(host.build_sync_checkpoint()).expect("retained state encodes"),
        before,
        "malformed {case} must preserve the whole live checkpoint"
    );
}

#[test]
fn sync_checkpoint_rejects_mismatched_counter_identity_without_mutation() {
    assert_invalid_counter_identity_import_is_atomic("count");
}
#[test]
fn sync_checkpoint_rejects_mismatched_counter_origin_identity_without_mutation() {
    assert_invalid_counter_identity_import_is_atomic("origin");
}
#[test]
fn sync_checkpoint_rejects_valid_counter_counts_conflicting_with_executable_without_mutation() {
    assert_invalid_counter_identity_import_is_atomic("valid-count-conflict");
}
#[test]
fn sync_checkpoint_rejects_valid_counter_registrations_conflicting_with_executable_without_mutation() {
    assert_invalid_counter_identity_import_is_atomic("valid-registration-conflict");
}
#[test]
fn sync_checkpoint_rejects_duplicate_counter_kinds_without_mutation() {
    assert_invalid_counter_identity_import_is_atomic("duplicate");
}
#[test]
fn sync_checkpoint_rejects_mismatched_typed_counter_chronology_without_mutation() {
    assert_invalid_counter_identity_import_is_atomic("chronology");
}

#[test]
fn sync_checkpoint_accepts_legacy_canonical_ordinary_counter_counts_and_chronology() {
    let _id_counter_guard = crate::test_id_counter_guard();
    let alice = PlayerId::from_index(0);
    let mut host = WasmGame::new();
    host.initialize_empty_match(vec!["Alice".into(), "Bob".into()], 20, 1);
    let definition = CardDefinition::new(
        ironsmith::CardBuilder::new(CardId::new(), "Legacy Counter Kind Fixture")
            .card_types(vec![CardType::Artifact])
            .build(),
    );
    host.registry.register(definition.clone());
    let source = host
        .game
        .create_object_from_definition(&definition, alice, Zone::Battlefield);
    host.game
        .add_counters(source, ironsmith::CounterType::Charge, 3)
        .expect("ordinary counters placed");
    let expected = host
        .game
        .effect_store
        .continuous_effects
        .counter_timestamps_snapshot();
    let mut checkpoint = host.build_sync_checkpoint();
    for object in &mut checkpoint.objects {
        for counter in &mut object.counters {
            counter.counter_type = None;
        }
        object.counter_ability_state = None;
    }
    checkpoint
        .continuous_timestamps
        .as_mut()
        .expect("chronology exported")
        .typed_counters = None;
    let mut guest = WasmGame::new();
    guest.registry.register(definition);
    guest
        .apply_sync_checkpoint(checkpoint)
        .expect("canonical ordinary legacy kind imports");
    assert_eq!(
        guest
            .game
            .counter_count(source, ironsmith::CounterType::Charge),
        3
    );
    assert_eq!(
        guest
            .game
            .effect_store
            .continuous_effects
            .counter_timestamps_snapshot(),
        expected
    );
}

}

#[cfg(test)]
mod owning_sync_executable_tests {
    use super::*;
    use ironsmith::ability::{Ability, AbilityKind};
    use ironsmith::continuous::{ContinuousEffect, Modification};
    use ironsmith::effect::Effect;
    use ironsmith::static_abilities::StaticAbility;
    fn fixture(compiled: bool) -> (GameState, ironsmith::cards::CardRegistry, Vec<Object>) {
        let alice = PlayerId::from_index(0);
        let mut flying = StaticAbility::flying();
        let token = ironsmith::cards::builders::CardDefinitionBuilder::new(
            CardId::new(),
            "Shared template",
        )
        .token()
        .card_types(vec![CardType::Creature])
        .power_toughness(ironsmith::card::PowerToughness::fixed(1, 1))
        .with_ability(Ability::static_ability(flying.clone()))
        .build();
        let mut definition = ironsmith::cards::builders::CardDefinitionBuilder::new(
            CardId::new(),
            "Saved owning program",
        )
        .card_types(vec![CardType::Sorcery])
        .with_ability(Ability::static_ability(flying.clone()))
        .with_spell_effect(vec![Effect::new(
            ironsmith::effects::CreateTokenEffect::new(
                token,
                1,
                ironsmith::target::PlayerFilter::You,
            ),
        )])
        .build();

        if compiled {
            definition = ironsmith_registry_test::compile_to_runtime_definition(
                "Saved owning program",
                "Type: Sorcery\nCreate a 1/1 white Soldier creature token with flying.",
                false,
            )
            .unwrap();
            let mut effect = definition.spell_effect.as_ref().unwrap().all_effects()[0];
            while let Some(child) = effect.transparent_child_effect() {
                effect = child;
            }
            let template = &effect
                .downcast_ref::<ironsmith::effects::CreateTokenEffect>()
                .unwrap()
                .token;
            flying = template
                .abilities
                .iter()
                .find_map(|ability| match &ability.kind {
                    AbilityKind::Static(value) => Some(value.clone()),
                    _ => None,
                })
                .unwrap();
            definition
                .abilities
                .push(Ability::static_ability(flying.clone()));
            definition.ability_labels.push("Flying".into());
            definition.canonical_text.push_str("\nFlying");
        }
        let active = if compiled {
            ironsmith_registry_test::compile_to_runtime_definition(
                "Active owning program",
                "Type: Sorcery\nYou gain 7 life.",
                false,
            )
            .unwrap()
            .spell_effect
            .unwrap()
            .all_effects()[0]
                .clone()
        } else {
            Effect::gain_life(7)
        };
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let spell = game.create_object_from_definition(&definition, alice, Zone::Stack);
        let raw_card = ironsmith::CardBuilder::new(CardId::new(), "Raw physical artifact")
            .card_types(vec![CardType::Artifact])
            .build();
        let artifact = game.create_object_from_card(&raw_card, alice, Zone::Battlefield);
        assert!(
            game.retained_card_definition(raw_card.id)
                .is_some_and(|definition| definition.card == raw_card),
            "raw creation retains original definition"
        );
        let object = game.object_mut(spell).unwrap();
        object.begin_stack_program_overlay();
        object.spell_effect = Some(
            ironsmith::resolution::ResolutionProgram::from_effects(vec![active.clone()]).into(),
        );
        object.cast_alternative_method = Some(Box::new(
            ironsmith::alternative_cast::AlternativeCastingMethod::Overload {
                cost: ironsmith::mana::ManaCost::new(),
                effects: vec![active.clone()],
            },
        ));
        game.effect_store.continuous_effects.add_effect(
            ContinuousEffect::from_resolution(
                artifact,
                alice,
                vec![artifact],
                Modification::SetColors(ColorSet::RED),
            )
            .until(ironsmith::effect::Until::EndOfTurn)
            .with_expires_end_of_turn(game.turn.turn_number),
        );
        game.effect_store
            .continuous_effects
            .add_effect(ContinuousEffect::from_resolution(
                artifact,
                alice,
                vec![spell],
                Modification::AddAbility(flying),
            ));
        game.refresh_continuous_state().unwrap();
        assert_eq!(game.current_colors(artifact), Some(ColorSet::RED));
        let objects = vec![
            game.object(artifact).unwrap().clone(),
            game.object(spell).unwrap().clone(),
        ];
        let mut registry = ironsmith::cards::CardRegistry::new();
        registry.register(definition);
        (game, registry, objects)
    }
    #[test]
    fn owning_sync_executable_root_rejection_precedes_history_and_preserves_managers() {
        let _guard = crate::test_id_counter_guard();
        for compiled in [false, true] {
            let (game, registry, objects) = fixture(compiled);
            let continuous = game.effect_store.continuous_effects.registered_state();
            let replacement = game.effect_store.replacement_effects.registered_state().unwrap();
            let prevention = game.effect_store.prevention_effects.retained_state().unwrap();
            let before = serde_json::to_value(SyncExecutableState::retain(&game, &registry, objects.clone(),
                continuous.clone(), replacement.clone(), prevention.clone()).unwrap()).unwrap();
            let mut approvals = 0;
            let mut history_calls = 0;
            let mut reference_calls = 0;
            let error = SyncExecutableState::retain_with_root_history_and_reference_policy(
                &game, &registry, objects.clone(), continuous, replacement, prevention,
                |roots| {
                    approvals += 1;
                    assert_eq!(roots.objects.len(), objects.len());
                    assert!(roots.continuous.effects.iter().any(|effect|
                        matches!(effect.modification, ironsmith::continuous::Modification::AddAbility(_))));
                    assert_eq!(roots.replacement.effects.len(), game.effect_store.replacement_effects.registered_state().unwrap().effects.len());
                    assert_eq!(roots.prevention.next_id, game.effect_store.prevention_effects.retained_state().unwrap().next_id);
                    assert_eq!(roots.provenance.retained_state(), game.provenance_graph().retained_state());
                    Err::<(), _>("continuous executable grant requires approval")
                },
                |snapshot| { history_calls += 1; Ok::<_, &str>(snapshot) },
                |_| { reference_calls += 1; Ok(()) },
            ).unwrap_err();
            assert!(error.contains("continuous executable grant requires approval"));
            assert_eq!(approvals, 1);
            assert_eq!(history_calls, 0);
            assert_eq!(reference_calls, 0);
            let after = serde_json::to_value(SyncExecutableState::retain(&game, &registry, objects,
                game.effect_store.continuous_effects.registered_state(),
                game.effect_store.replacement_effects.registered_state().unwrap(),
                game.effect_store.prevention_effects.retained_state().unwrap()).unwrap()).unwrap();
            assert_eq!(before, after, "authorization failure cannot mutate authoritative native roots");
        }
    }
    #[test]
    fn owning_sync_executable_invalid_root_fails_before_authorization() {
        let _guard = crate::test_id_counter_guard();
        let (game, registry, mut objects) = fixture(false);
        objects.push(objects[0].clone());
        let mut approvals = 0;
        let mut history_calls = 0;
        let error = SyncExecutableState::retain_with_root_approval_and_history_policy(
            &game, &registry, objects, game.effect_store.continuous_effects.registered_state(),
            game.effect_store.replacement_effects.registered_state().unwrap(),
            game.effect_store.prevention_effects.retained_state().unwrap(),
            |_| { approvals += 1; Ok::<_, &str>(()) },
            |snapshot| { history_calls += 1; Ok::<_, &str>(snapshot) },
        ).unwrap_err();
        assert!(error.contains("duplicate executable object root"));
        assert_eq!((approvals, history_calls), (0, 0));
    }
    #[test]
    fn owning_sync_executable_rejects_face_before_catalog_lookup() {
        let _guard = crate::test_id_counter_guard();
        let (game, registry, mut objects) = fixture(false);
        let missing = CardId::from_raw(u32::MAX);
        objects[0].card = Some(missing);
        let mut calls = Vec::new();
        let error = SyncExecutableState::retain_with_card_reference_policy(&game, &registry, objects,
            game.effect_store.continuous_effects.registered_state(),
            game.effect_store.replacement_effects.registered_state().unwrap(),
            game.effect_store.prevention_effects.retained_state().unwrap(),
            |face| { calls.push(face); if face == missing { Err("unapproved live face".into()) } else { Ok(()) } },
        ).unwrap_err();
        assert!(error.contains("unapproved live face"), "authorization must precede missing catalog lookup: {error}");
        assert_eq!(calls.iter().filter(|face| **face == missing).count(), 1);
    }
    #[test]
    fn owning_sync_executable_rejects_nested_native_and_compiled_template_faces() {
        let _guard = crate::test_id_counter_guard();
        for compiled in [false, true] {
            let (game, registry, objects) = fixture(compiled);
            let definition = registry.get("Saved owning program").unwrap();
            let mut effect = definition.spell_effect.as_ref().unwrap().all_effects()[0];
            while let Some(child) = effect.transparent_child_effect() { effect = child; }
            let forbidden = effect.downcast_ref::<ironsmith::effects::CreateTokenEffect>().unwrap().token.card.id;
            let continuous = game.effect_store.continuous_effects.registered_state();
            let replacement = game.effect_store.replacement_effects.registered_state().unwrap();
            let prevention = game.effect_store.prevention_effects.retained_state().unwrap();
            let original = serde_json::to_value(SyncExecutableState::retain(&game, &registry, objects.clone(),
                continuous.clone(), replacement.clone(), prevention.clone()).unwrap()).unwrap();
            let mut calls = Vec::new();
            let error = SyncExecutableState::retain_with_card_reference_policy(&game, &registry, objects.clone(),
                continuous, replacement, prevention,
                |face| { calls.push(face); if face == forbidden { Err("private embedded face".into()) } else { Ok(()) } },
            ).unwrap_err();
            assert!(error.contains("private embedded face"));
            assert_eq!(calls.iter().filter(|face| **face == forbidden).count(), 1);
            let after = serde_json::to_value(SyncExecutableState::retain(&game, &registry, objects,
                game.effect_store.continuous_effects.registered_state(),
                game.effect_store.replacement_effects.registered_state().unwrap(),
                game.effect_store.prevention_effects.retained_state().unwrap()).unwrap()).unwrap();
            assert_eq!(original, after, "reference rejection cannot mutate any authoritative root");
        }
    }
    #[test]
    fn owning_sync_executable_delayed_root_and_history_rejection_precede_discovery() {
        let _guard = crate::test_id_counter_guard();
        for compiled in [false, true] {
            let (mut game, registry, objects) = fixture(compiled);
            let alice = PlayerId::from_index(0);
            let snapshot = ironsmith::snapshot::ObjectSnapshot::from_object(&objects[0], &game);
            let mut trigger = test_delayed_registration(game.turn.turn_number, alice);
            trigger.ability_source = Some(snapshot.object_id);
            trigger.ability_source_snapshot = Some(snapshot.clone());
            trigger.tagged_objects.insert("delayed source capture".into(), vec![snapshot.clone()]);
            game.effect_store.delayed_triggers.push(trigger);
            let continuous = game.effect_store.continuous_effects.registered_state();
            let replacement = game.effect_store.replacement_effects.registered_state().unwrap();
            let prevention = game.effect_store.prevention_effects.retained_state().unwrap();
            let before = serde_json::to_value(SyncExecutableState::retain(&game, &registry, objects.clone(),
                continuous.clone(), replacement.clone(), prevention.clone()).unwrap()).unwrap();
            let mut histories = 0;
            let mut references = 0;
            let error = SyncExecutableState::retain_with_root_history_and_reference_policy(&game, &registry, objects.clone(),
                continuous.clone(), replacement.clone(), prevention.clone(),
                |roots| {
                    assert_eq!(roots.delayed_triggers.len(), 1);
                    assert_eq!(roots.delayed_triggers[0].effects.all_effects().len(), 1);
                    Err::<(), String>("delayed executable root is private".into())
                },
                |snapshot| { histories += 1; Ok::<_, String>(snapshot) },
                |_| { references += 1; Ok(()) }).unwrap_err();
            assert!(error.contains("delayed executable root is private"));
            assert_eq!(histories, 0);
            assert_eq!(references, 0);
            let mut history_visits = Vec::new();
            let error = SyncExecutableState::retain_with_root_history_and_reference_policy(&game, &registry, objects.clone(),
                continuous.clone(), replacement.clone(), prevention.clone(),
                |_| Ok::<_, String>(()),
                |saved| { history_visits.push(saved.object_id); Err::<ironsmith::snapshot::ObjectSnapshot, String>("delayed capture is private".into()) },
                |_| { references += 1; Ok(()) }).unwrap_err();
            assert!(error.contains("delayed capture is private"));
            assert!(history_visits.contains(&snapshot.object_id));
            assert_eq!(references, 0, "history rejection precedes graph references");
            let after = serde_json::to_value(SyncExecutableState::retain(&game, &registry, objects,
                continuous, replacement, prevention).unwrap()).unwrap();
            assert_eq!(before, after, "rejected delayed publication leaves complete owner graph unchanged");
        }
    }
    #[test]
    fn owning_sync_executable_reference_approval_is_once_and_canonical() {
        let _guard = crate::test_id_counter_guard();
        for compiled in [false, true] {
            let (game, registry, objects) = fixture(compiled);
            let continuous = game.effect_store.continuous_effects.registered_state();
            let replacement = game.effect_store.replacement_effects.registered_state().unwrap();
            let prevention = game.effect_store.prevention_effects.retained_state().unwrap();
            let expected = SyncExecutableState::retain(&game, &registry, objects.clone(),
                continuous.clone(), replacement.clone(), prevention.clone()).unwrap();
            let mut calls = Vec::new();
            let mut root_approvals = 0;
            let actual = SyncExecutableState::retain_with_root_history_and_reference_policy(&game, &registry, objects,
                continuous, replacement, prevention,
                |roots| { root_approvals += 1; assert_eq!(roots.objects.len(), 2); Ok::<_, String>(()) },
                Ok::<_, String>, |face| { calls.push(face); Ok(()) }).unwrap();
            assert_eq!(root_approvals, 1);
            assert_eq!(calls.len(), calls.iter().map(|face| face.0).collect::<std::collections::BTreeSet<_>>().len());
            assert_eq!(calls.len(), actual.graph_card_count as usize);
            assert_eq!(serde_json::to_value(actual).unwrap(), serde_json::to_value(expected).unwrap(),
                "authorization cannot change graph slots, aliases or the retained programs");
        }
    }
    #[test]
    fn owning_sync_executable_private_copy_program_requires_payload_approval_without_private_face_reference() {
        let _guard = crate::test_id_counter_guard();
        for compiled in [false, true] {
            let (mut game, mut registry, mut objects) = fixture(compiled);
            let alice = PlayerId::from_index(0);
            let private = if compiled {
                ironsmith_registry_test::compile_to_runtime_definition("Private copy capture marker",
                    "Type: Artifact\nPay 1 life: You gain 7777 life.", false).unwrap()
            } else {
                ironsmith::cards::builders::CardDefinitionBuilder::new(CardId::new(), "Private copy capture marker")
                    .card_types(vec![CardType::Artifact])
                    .with_ability(Ability::activated(ironsmith::TotalCost::free(), vec![Effect::gain_life(7777)])).build()
            };
            registry.register(private.clone());
            let hidden = game.create_object_from_definition(&private, alice, Zone::Library);
            let mut values = ironsmith::snapshot::CopiableValues::from_object(game.object(hidden).unwrap());
            values.name = "Opaque copy".into();
            values.compiled_card_text.clear();
            values.ability_labels.clear();
            assert!(!values.abilities.is_empty());
            objects.push(opaque_sync_executable_object(game.object(hidden).unwrap().clone()));
            let source = objects[0].id;
            let effect_id = game.effect_store.continuous_effects.add_effect(ContinuousEffect::from_resolution(
                source, alice, vec![hidden], Modification::CopyOf {
                    target_id: hidden, copiable_values: Box::new(values), preserve_source_abilities: false,
                    name_override: None, name_override_surface: None, add_supertypes: Vec::new(),
                }));
            let continuous = game.effect_store.continuous_effects.registered_state();
            let replacement = game.effect_store.replacement_effects.registered_state().unwrap();
            let prevention = game.effect_store.prevention_effects.retained_state().unwrap();
            let mut face_calls = Vec::new();
            let mut history_calls = 0;
            let full = SyncExecutableState::retain_with_root_history_and_reference_policy(&game, &registry, objects.clone(),
                continuous.clone(), replacement.clone(), prevention.clone(), |_| Ok::<_, String>(()),
                |snapshot| { history_calls += 1; Ok(snapshot) },
                |face| { face_calls.push(face); if face == private.card.id { Err("private face cannot be approved".into()) } else { Ok(()) } },
            ).unwrap();
            let json = serde_json::to_string(&full).unwrap();
            assert!(!face_calls.contains(&private.card.id));
            assert_eq!(history_calls, 0);
            assert!(!json.contains("Private copy capture marker"));
            assert!(json.contains("7777"), "display redaction and face/history policy do not authorize native copy programs");
            let mut inspected_copy = false;
            let mut post_history_calls = 0;
            let mut post_reference_calls = 0;
            let error = SyncExecutableState::retain_with_root_history_and_reference_policy(&game, &registry, objects.clone(),
                continuous.clone(), replacement, prevention,
                |roots| approve_sync_continuous_payloads(roots.continuous, |effect, payload| {
                    if let SyncContinuousExecutablePayload::Copy(values) = payload {
                        assert_eq!(effect.id, effect_id);
                        assert!(values.compiled_card_text.is_empty());
                        assert!(values.ability_labels.is_empty());
                        assert!(!values.abilities.is_empty());
                        inspected_copy = true;
                        return Err("private copied executable requires disclosure".to_string());
                    }
                    Ok(())
                }),
                |snapshot| { post_history_calls += 1; Ok(snapshot) },
                |_| { post_reference_calls += 1; Ok(()) },
            ).unwrap_err();
            assert!(error.contains("private copied executable requires disclosure"));
            assert!(inspected_copy);
            assert_eq!((post_history_calls, post_reference_calls), (0, 0));
            // Native Effect::eq intentionally returns false even for a clone.
            // Compare the complete executable graph, including copied programs,
            // registrations, chronology and occurrence aliases instead.
            let after = SyncExecutableState::retain(&game, &registry, objects,
                game.effect_store.continuous_effects.registered_state(),
                game.effect_store.replacement_effects.registered_state().unwrap(),
                game.effect_store.prevention_effects.retained_state().unwrap()).unwrap();
            assert_eq!(serde_json::to_value(after).unwrap(), serde_json::to_value(full).unwrap(),
                "payload authorization failure preserves the complete owning graph");
        }
    }
    fn peer_bindings(state: &SyncExecutableState) -> Vec<CardId> {
        (0..state.graph_card_count).map(|_| CardId::new()).collect()
    }
    fn check_restore(compiled: bool) {
        let _id_counter_guard = crate::test_id_counter_guard();
        let (game, registry, objects) = fixture(compiled);
        let alice = PlayerId::from_index(0);
        let artifact = objects[0].id;
        let spell = objects[1].id;
        let mut approvals = 0;
        let encoded = SyncExecutableState::retain_with_root_approval_and_history_policy(
            &game,
            &registry,
            objects,
            game.effect_store.continuous_effects.registered_state(),
            game.effect_store.replacement_effects.registered_state().unwrap(),
            game.effect_store.prevention_effects.retained_state().unwrap(),
            |roots| {
                approvals += 1;
                assert_eq!(roots.objects.len(), 2);
                assert!(roots.continuous.effects.iter().any(|effect|
                    matches!(effect.modification, ironsmith::continuous::Modification::AddAbility(_))));
                Ok::<_, &str>(())
            },
            Ok::<_, &str>,
        )
        .unwrap();
        assert_eq!(approvals, 1, "whole roots are authorized once before publication");
        let json = serde_json::to_value(&encoded).unwrap();
        let wire: SyncExecutableState = serde_json::from_value(json.clone()).unwrap();
        let peers = peer_bindings(&wire);
        let restored = wire.restore(&peers).unwrap();
        assert_eq!(
            restored.definitions.len(),
            2,
            "embedded template identity has its own explicit snapshot, not a guessed catalog definition"
        );
        let object = restored
            .objects
            .iter()
            .find(|object| object.id == spell)
            .unwrap();
        let AbilityKind::Static(flying) = &object.abilities[0].kind else {
            panic!("native static ability")
        };
        let grant = restored
            .continuous
            .effects
            .iter()
            .find_map(|effect| match &effect.modification {
                Modification::AddAbility(value) => Some(value),
                _ => None,
            })
            .unwrap();
        assert_eq!(
            grant.instance_id(),
            flying.instance_id(),
            "object and registered descriptor share receiver occurrence"
        );
        let definition = restored
            .definitions
            .iter()
            .find(|definition| definition.card.name == "Saved owning program")
            .unwrap();
        let AbilityKind::Static(printed) = &definition.abilities[0].kind else {
            panic!("printed static")
        };
        assert_eq!(printed.instance_id(), flying.instance_id());
        let flying = flying.instance_id();
        let mut peer = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        *peer.provenance_graph_mut() = restored.provenance_graph.clone();
        let mut peer_registry = ironsmith::cards::CardRegistry::new();
        for definition in &restored.definitions {
            peer_registry.register(definition.clone());
            peer.register_linked_face_definition(definition);
        }
        for object in &restored.objects {
            peer.add_object(object.clone());
        }
        peer.effect_store
            .continuous_effects
            .restore_registered_state(restored.continuous.clone())
            .unwrap();
        peer.refresh_continuous_state().unwrap();
        assert_eq!(peer.current_colors(artifact), Some(ColorSet::RED));
        let reencoded = SyncExecutableState::retain(
            &peer,
            &peer_registry,
            restored.objects,
            restored.continuous.clone(),
            restored.replacement.clone(),
            restored.prevention.clone(),
        )
        .unwrap();
        assert_eq!(
            serde_json::to_value(reencoded).unwrap(),
            json,
            "whole owning executable graph canonical reencoding"
        );
        let program = peer.object(spell).unwrap().spell_effect_owned().unwrap();
        let mut context = ironsmith::effects::EffectContext::new_default(spell, alice);
        for effect in program.all_effects() {
            ironsmith::effects::execute_effect(&mut peer, effect, &mut context).unwrap();
        }
        assert_eq!(
            peer.player(alice).unwrap().life,
            27,
            "restored active overlay executes"
        );

        let destination = peer
            .move_object(
                spell,
                Zone::Graveyard,
                ironsmith::events::cause::EventCause::effect(),
            )
            .unwrap();
        assert!(
            peer.object(destination)
                .unwrap()
                .splice_cast_state
                .is_none()
        );
        let program = peer
            .object(destination)
            .unwrap()
            .spell_effect_owned()
            .unwrap();
        let mut context = ironsmith::effects::EffectContext::new_default(destination, alice);
        for effect in program.all_effects() {
            ironsmith::effects::execute_effect(&mut peer, effect, &mut context).unwrap();
        }
        assert_eq!(peer.player(alice).unwrap().life, 27);
        let token = peer
            .battlefield
            .iter()
            .filter_map(|id| peer.object(*id))
            .find(|object| object.kind == ironsmith::object::ObjectKind::Token)
            .unwrap();
        assert!(token.abilities.iter().any(|ability|matches!(&ability.kind,AbilityKind::Static(value) if value.instance_id()==flying)),"saved original template uses same native receiver occurrence");
        let color = restored
            .continuous
            .effects
            .iter()
            .find(|effect| matches!(effect.modification, Modification::SetColors(_)))
            .unwrap()
            .id;
        peer.effect_store.continuous_effects.remove_effect(color);
        peer.refresh_continuous_state().unwrap();
        assert_eq!(peer.current_colors(artifact), Some(ColorSet::COLORLESS));
    }
    fn check_rejections(compiled: bool) {
        let _id_counter_guard = crate::test_id_counter_guard();
        let (game, registry, objects) = fixture(compiled);
        let state = SyncExecutableState::retain(
            &game,
            &registry,
            objects,
            game.effect_store.continuous_effects.registered_state(),
            game.effect_store.replacement_effects.registered_state().unwrap(),
            game.effect_store.prevention_effects.retained_state().unwrap(),
        )
        .unwrap();
        let peers = peer_bindings(&state);
        for field in [
            "graphCardCount",
            "provenanceGraph",
            "occurrences",
            "definitions",
            "objects",
            "continuous",
            "replacement",
            "prevention",
        ] {
            let mut missing = serde_json::to_value(&state).unwrap();
            missing.as_object_mut().unwrap().remove(field);
            assert!(
                serde_json::from_value::<SyncExecutableState>(missing).is_err(),
                "required owner field {field}"
            );
        }
        assert!(state.restore(&peers[..peers.len() - 1]).is_err());
        let mut duplicate_binding = peers.clone();
        duplicate_binding[1] = duplicate_binding[0];
        assert!(state.restore(&duplicate_binding).is_err());
        let mut reversed = peers.clone();
        reversed.reverse();
        assert!(
            state.restore(&reversed).is_err(),
            "allocation-order face semantics cannot silently invert"
        );
        let mut bad = state.clone();
        bad.definitions.push(bad.definitions[0].clone());
        assert!(bad.restore(&peers).is_err());
        let mut bad = state.clone();
        bad.definitions[0].0 = u32::MAX;
        assert!(bad.restore(&peers).is_err());
        let mut bad = state.clone();
        bad.definitions[0].0 = (bad.definitions[0].0 + 1) % bad.graph_card_count;
        assert!(bad.restore(&peers).is_err());
        let mut bad = state.clone();
        bad.objects.push(bad.objects[0].clone());
        assert!(bad.restore(&peers).is_err());
        let mut bad = state.clone();
        bad.occurrences.model_card_references.clear();
        assert!(bad.restore(&peers).is_err());
        let mut bad = state.clone();
        bad.continuous.next_id = u64::MAX;
        assert!(bad.restore(&peers).is_err());
        assert!(
            state.restore(&peers).is_ok(),
            "valid immutable state remains usable after every rejection"
        );
    }
    fn check_approved_hidden_root(compiled: bool) {
        let _id_counter_guard = crate::test_id_counter_guard();
        let alice = PlayerId::from_index(0);
        let definition = if compiled {
            ironsmith_registry_test::compile_to_runtime_definition(
                "Private executable root",
                "Type: Sorcery\nYou gain 7777 life.",
                false,
            )
            .unwrap()
        } else {
            ironsmith::cards::builders::CardDefinitionBuilder::new(
                CardId::new(),
                "Private executable root",
            )
            .card_types(vec![CardType::Sorcery])
            .with_spell_effect(vec![Effect::gain_life(7777)])
            .build()
        };
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let id = game.create_object_from_definition(&definition, alice, Zone::Library);
        let original = game.object(id).unwrap();
        let mut registry = ironsmith::cards::CardRegistry::new();
        registry.register(definition);
        let full = SyncExecutableState::retain(
            &game,
            &registry,
            vec![original.clone()],
            game.effect_store.continuous_effects.registered_state(),
            game.effect_store.replacement_effects.registered_state().unwrap(),
            game.effect_store.prevention_effects.retained_state().unwrap(),
        )
        .unwrap();
        assert!(
            serde_json::to_string(&full)
                .unwrap()
                .contains("Private executable root")
        );
        let mut placeholder = Object::new_hidden_card(id, alice, Zone::Library);
        placeholder.stable_id = original.stable_id;
        let redacted = SyncExecutableState::retain(
            &game,
            &registry,
            vec![placeholder],
            game.effect_store.continuous_effects.registered_state(),
            game.effect_store.replacement_effects.registered_state().unwrap(),
            game.effect_store.prevention_effects.retained_state().unwrap(),
        )
        .unwrap();
        assert_eq!(redacted.graph_card_count, 0);
        assert!(redacted.definitions.is_empty());
        assert!(redacted.occurrences.models.is_empty());
        let json = serde_json::to_string(&redacted).unwrap();
        assert!(!json.contains("Private executable root"));
        assert!(!json.contains("7777"));
        assert!(
            redacted.restore(&[]).unwrap().objects[0]
                .spell_effect
                .is_none()
        );
        // This tests approved root closure, not the complete perspective/privacy
        // projection of captured costs, historical sources or descriptors.
    }

    fn check_registered_predicate_graph(compiled:bool, dependent:bool){
        let _guard=crate::test_id_counter_guard();
        let (mut game,mut registry,mut objects)=fixture(compiled);
        let source=objects[0].id;let alice=PlayerId::from_index(0);
        let private=if compiled{
            ironsmith_registry_test::compile_to_runtime_definition("Private graph capture marker",
                "Type: Artifact\nPay 1 life: You gain 7777 life.",false).unwrap()
        }else{
            ironsmith::cards::builders::CardDefinitionBuilder::new(CardId::new(),"Private graph capture marker")
                .card_types(vec![CardType::Artifact])
                .with_ability(Ability::activated(ironsmith::cost::TotalCost::free(),vec![Effect::gain_life(7777)])).build()
        };
        registry.register(private.clone());
        let hidden=game.create_object_from_definition(&private,alice,Zone::Library);
        game.set_hidden_card_info(hidden,HiddenCardInfo{owner:alice,zone:Zone::Library,slot:0,
            commitment:"graph-private-commitment".into(),origin_slot:None,origin_commitment:None,
            public_slot:None,public_commitment:None});
        let mut saved=ironsmith::snapshot::ObjectSnapshot::from_object(game.object(hidden).unwrap(),&game);
        saved.compiled_card_text.clear();saved.ability_labels.clear();
        saved.copiable_values.compiled_card_text.clear();saved.copiable_values.ability_labels.clear();
        saved.chosen_object=Some(Box::new(saved.clone()));
        let mut filter=ironsmith::target::ObjectFilter::permanent();
        if dependent{filter.controller=Some(ironsmith::target::PlayerFilter::OwnerOf(ironsmith::target::ObjectRef::Specific(hidden)));}
        let matcher=ironsmith::events::zones::matchers::WouldChangeZoneMatcher::new(filter,Some(Zone::Battlefield),Some(Zone::Graveyard))
            .with_frozen_tagged_objects(std::collections::HashMap::from([("private-history".into(),vec![saved])]));
        let id=game.effect_store.replacement_effects.add_one_shot_effect(ironsmith::replacement::ReplacementEffect::with_matcher(
            source,alice,matcher,ironsmith::replacement::ReplacementAction::ChangeDestination(Zone::Exile)));
        let key=game.effect_store.replacement_effects.get_effect(id).unwrap().application_key();
        let mut placeholder=Object::new_hidden_card(hidden,alice,Zone::Library);
        placeholder.stable_id=game.object(hidden).unwrap().stable_id;objects.push(placeholder);
        let complete=SyncExecutableState::retain(&game,&registry,objects.clone(),game.effect_store.continuous_effects.registered_state(),
            game.effect_store.replacement_effects.registered_state().unwrap(),game.effect_store.prevention_effects.retained_state().unwrap()).unwrap();
        let full_json=serde_json::to_string(&complete).unwrap();
        assert!(full_json.contains("Private graph capture marker"));assert!(full_json.contains("7777"));
        let mut calls=0;
        let state=SyncExecutableState::retain_with_registered_predicate_history_policy(&game,&registry,objects,
            game.effect_store.continuous_effects.registered_state(),game.effect_store.replacement_effects.registered_state().unwrap(),
            game.effect_store.prevention_effects.retained_state().unwrap(),|snapshot|{
                calls+=1;
                if !dependent{return Err("unused private capture must not require disclosure");}
                assert_eq!(snapshot.object_id,hidden);
                // Only the public owner/controller/identity tuple is required.
                Ok(ironsmith::snapshot::ObjectSnapshot::public_placeholder(snapshot.object_id,snapshot.stable_id,
                    snapshot.owner,snapshot.controller,snapshot.zone))
            }).unwrap();
        assert_eq!(calls,usize::from(dependent),"selection and policy run once before graph discovery");
        let json=serde_json::to_value(&state).unwrap();let text=serde_json::to_string(&json).unwrap();
        assert!(!text.contains("Private graph capture marker"));assert!(!text.contains("7777"));
        let wire:SyncExecutableState=serde_json::from_value(json.clone()).unwrap();let peers=peer_bindings(&wire);let restored=wire.restore(&peers).unwrap();
        let mut peer=GameState::new(vec!["Alice".into(),"Bob".into()],20);*peer.provenance_graph_mut()=restored.provenance_graph.clone();
        let mut peer_registry=ironsmith::cards::CardRegistry::new();for definition in &restored.definitions{peer_registry.register(definition.clone());}
        for object in &restored.objects{peer.add_object(object.clone());}
        // The auxiliary metadata carrier is outside this executable graph.
        peer.set_hidden_card_info(hidden,game.hidden_card_info(hidden).unwrap().clone());
        peer.effect_store.continuous_effects.restore_registered_state(restored.continuous.clone()).unwrap();
        peer.effect_store.replacement_effects.restore_registered_state(restored.replacement.clone()).unwrap();
        peer.effect_store.prevention_effects.restore_retained_state(restored.prevention.clone()).unwrap();peer.refresh_continuous_state().unwrap();
        assert_eq!(peer.effect_store.replacement_effects.get_effect(id).unwrap().application_key(),key);
        let recoded=SyncExecutableState::retain(&peer,&peer_registry,restored.objects,restored.continuous,restored.replacement,restored.prevention).unwrap();
        assert_eq!(serde_json::to_value(recoded).unwrap(),json,"projected graph is canonical on fresh peer bindings");
        assert!(peer.is_hidden_card_placeholder(hidden));
        let mut context=ironsmith::effects::EffectContext::new_default(source,alice);
        ironsmith::effects::execute_effect(&mut peer,&Effect::move_to_zone(ironsmith::target::ChooseSpec::SpecificObject(source),Zone::Graveyard,true),&mut context).unwrap();
        assert!(peer.exile.iter().any(|id|peer.object(*id).unwrap().name=="Raw physical artifact"));
        assert!(peer.effect_store.replacement_effects.get_effect(id).is_none());
        assert!(game.effect_store.replacement_effects.get_effect(id).is_some());
    }
    #[test]fn owning_registered_predicate_graph_native_drops_unused_private_program(){check_registered_predicate_graph(false,false);}
    #[test]fn owning_registered_predicate_graph_compiled_drops_unused_private_program(){check_registered_predicate_graph(true,false);}
    #[test]fn owning_registered_predicate_graph_native_preserves_specific_owner_fact(){check_registered_predicate_graph(false,true);}
    #[test]fn owning_registered_predicate_graph_compiled_preserves_specific_owner_fact(){check_registered_predicate_graph(true,true);}
    fn history_inputs(compiled:bool)->(GameState,ironsmith::cards::CardRegistry,Vec<Object>,ironsmith::replacement::ReplacementEffectId){
        let (mut game,mut registry,mut objects)=fixture(compiled);let alice=PlayerId::from_index(0);let bob=PlayerId::from_index(1);let source=objects[0].id;
        let private=if compiled{ironsmith_registry_test::compile_to_runtime_definition("Private owner history marker","Type: Artifact\nPay 1 life: You gain 7777 life.",false).unwrap()}else{ironsmith::cards::builders::CardDefinitionBuilder::new(CardId::new(),"Private owner history marker").card_types(vec![CardType::Artifact]).with_ability(Ability::activated(ironsmith::cost::TotalCost::free(),vec![Effect::gain_life(7777)])).build()};
        registry.register(private.clone());let hidden=game.create_object_from_definition(&private,bob,Zone::Library);
        let mut secret=ironsmith::snapshot::ObjectSnapshot::from_object(game.object(hidden).unwrap(),&game);assert!(secret.abilities.iter().any(|ability|matches!(&ability.kind,AbilityKind::Activated(_))));secret.compiled_card_text.clear();secret.ability_labels.clear();secret.copiable_values.compiled_card_text.clear();secret.copiable_values.ability_labels.clear();secret.chosen_object=Some(Box::new(secret.clone()));
        let mut saved=ironsmith::snapshot::ObjectSnapshot::from_object(game.object(source).unwrap(),&game);
        saved.chosen_object=Some(Box::new(secret.clone()));saved.mana_sources_spent_to_cast=vec![secret.clone()];saved.attachment_snapshots=vec![secret];
        objects[0].cast_tagged_objects.insert(ironsmith::tag::TagKey::from("paid_history"),vec![saved.clone()]);
        let matcher=ironsmith::events::zones::matchers::WouldChangeZoneMatcher::new(ironsmith::target::ObjectFilter::permanent(),Some(Zone::Battlefield),Some(Zone::Graveyard)).with_frozen_tagged_objects(std::collections::HashMap::from([(ironsmith::tag::TagKey::from("history"),vec![saved.clone()])]));
        let id=game.effect_store.replacement_effects.add_one_shot_effect(ironsmith::replacement::ReplacementEffect::with_matcher(source,alice,matcher,ironsmith::replacement::ReplacementAction::ChangeDestination(Zone::Exile)));
        let mut capture=ironsmith::replacement_entry_capture::RetainedEntryEvent::capture(ironsmith::events::EnterBattlefieldEvent::new(source,Zone::Hand));capture.program_choices.as_enters_tagged_objects=vec![(ironsmith::tag::TagKey::from("entry"),vec![saved.clone()])];capture.prepared_choices=Some(capture.program_choices.clone());
        let mut scope=ironsmith::effects::ReplacementExecutionContext::default();scope.entry_event=Some(Box::new(capture.into_native().unwrap()));scope.additional_replacement_effects=vec![game.effect_store.replacement_effects.get_effect(id).unwrap().clone()];scope.suppressed_replacement_effects.insert(id);scope.suppressed_replacement_effect_keys.insert(scope.additional_replacement_effects[0].application_key());
        let shield=ironsmith::prevention::PreventionShield::prevent_next_n(source,alice,ironsmith::prevention::PreventionTarget::You,3).with_follow_up_effects(vec![Effect::gain_life(2)]);let shield=game.effect_store.prevention_effects.add_shield(shield);let follow=game.effect_store.prevention_effects.apply_chosen_shield(shield,1,true,None).follow_ups.remove(0);
        let mut damage=ironsmith::events::DamageEvent::with_cause(source,ironsmith::events::DamageTarget::Player(alice),5,true,ironsmith::events::cause::EventCause::effect());damage.target_snapshot=Some(saved.clone());game.effect_store.prevention_effects.queue_follow_up(follow,damage,Default::default());
        let mut prevention=game.effect_store.prevention_effects.retained_state().unwrap();let pending=prevention.pending_follow_ups.remove(0);let mut pending=pending.try_map_payloads(|_|Ok::<_,String>(scope.clone()),Ok,Ok,Ok).unwrap();pending.source_snapshot=Some(saved);prevention.pending_follow_ups.push(pending);prevention.follow_up_deferral_depth=1;prevention.follow_up_replacement_scopes=vec![scope];game.effect_store.prevention_effects.restore_retained_state(prevention).unwrap();
        let mut placeholder=Object::new_hidden_card(hidden,bob,Zone::Library);placeholder.stable_id=game.object(hidden).unwrap().stable_id;objects.push(placeholder);
        (game,registry,objects,id)
    }
    fn check_projected_owner(compiled:bool){
        let _guard=crate::test_id_counter_guard();let (game,registry,objects,id)=history_inputs(compiled);let source=objects[0].id;let alice=PlayerId::from_index(0);
        let full=SyncExecutableState::retain(&game,&registry,objects.clone(),game.effect_store.continuous_effects.registered_state(),game.effect_store.replacement_effects.registered_state().unwrap(),game.effect_store.prevention_effects.retained_state().unwrap()).unwrap();let control=serde_json::to_string(&full).unwrap();assert!(control.contains("Private owner history marker"));assert!(control.contains("7777"));assert!(serde_json::to_string(&full.replacement).unwrap().contains("7777"),"private activated executable reaches the complete descriptor graph even with snapshot surface text removed");
        let mut visits=0;
        let state=SyncExecutableState::retain_with_history_policy(&game,&registry,objects,game.effect_store.continuous_effects.registered_state(),game.effect_store.replacement_effects.registered_state().unwrap(),game.effect_store.prevention_effects.retained_state().unwrap(),|snapshot|{
            visits+=1;Ok::<_,String>(if snapshot.owner==PlayerId::from_index(1){ironsmith::snapshot::ObjectSnapshot::public_placeholder(snapshot.object_id,snapshot.stable_id,snapshot.owner,snapshot.controller,snapshot.zone)}else{snapshot})
        }).unwrap();assert_eq!(visits,40,"one policy pass for ten complete history trees, never replayed by discovery or encoding");
        let json=serde_json::to_string(&state).unwrap();assert!(!json.contains("Private owner history marker"));assert!(!json.contains("7777"));
        let wire:SyncExecutableState=serde_json::from_str(&json).unwrap();let peers=peer_bindings(&wire);let restored=wire.restore(&peers).unwrap();let mut peer=GameState::new(vec!["Alice".into(),"Bob".into()],20);*peer.provenance_graph_mut()=restored.provenance_graph.clone();
        let mut peer_registry=ironsmith::cards::CardRegistry::new();for definition in &restored.definitions{peer_registry.register(definition.clone());}for object in &restored.objects{peer.add_object(object.clone());}
        peer.effect_store.continuous_effects.restore_registered_state(restored.continuous).unwrap();peer.effect_store.replacement_effects.restore_registered_state(restored.replacement).unwrap();peer.effect_store.prevention_effects.restore_retained_state(restored.prevention).unwrap();peer.refresh_continuous_state().unwrap();assert_eq!(peer.current_colors(source),Some(ColorSet::RED));
        let pending=&peer.effect_store.prevention_effects.retained_state().unwrap().pending_follow_ups[0];let body=pending.follow_up.effects[0].clone();assert_eq!(peer.effect_store.prevention_effects.retained_state().unwrap().shields[0].amount_remaining,Some(2));
        let mut ctx=ironsmith::effects::EffectContext::new_default(source,alice);ironsmith::effects::execute_effect(&mut peer,&Effect::move_to_zone(ironsmith::target::ChooseSpec::SpecificObject(source),Zone::Graveyard,true),&mut ctx).unwrap();assert!(peer.object(source).is_none());assert!(peer.exile.iter().any(|id|peer.object(*id).unwrap().name=="Raw physical artifact"));assert!(peer.effect_store.replacement_effects.get_effect(id).is_none());
        ironsmith::effects::execute_effect(&mut peer,&body,&mut ctx).unwrap();assert_eq!(peer.player(alice).unwrap().life,22);assert!(game.effect_store.replacement_effects.get_effect(id).is_some());
    }
    #[test]fn owning_sync_history_policy_native_projects_once_and_preserves_gameplay(){check_projected_owner(false);}
    #[test]fn owning_sync_history_policy_compiled_projects_once_and_preserves_gameplay(){check_projected_owner(true);}
    #[test]fn owning_sync_history_policy_bad_manager_rejects_before_policy_callbacks(){
        let _guard=crate::test_id_counter_guard();let (game,registry,objects,_)=history_inputs(false);let mut prevention=game.effect_store.prevention_effects.retained_state().unwrap();prevention.next_id=0;let mut calls=0;
        let error=SyncExecutableState::retain_with_history_policy(&game,&registry,objects,game.effect_store.continuous_effects.registered_state(),game.effect_store.replacement_effects.registered_state().unwrap(),prevention,|snapshot|{calls+=1;Ok::<_,String>(snapshot)}).unwrap_err();assert!(!error.is_empty());assert_eq!(calls,0);
    }
    fn check_all_managers(compiled:bool){
        let _id_counter_guard=crate::test_id_counter_guard();
        let (mut game,registry,objects)=fixture(compiled);let source=objects[0].id;let alice=PlayerId::from_index(0);
        let replacement=game.effect_store.replacement_effects.add_one_shot_effect(ironsmith::replacement::ReplacementEffect::with_matcher(source,alice,ironsmith::events::life::matchers::WouldGainLifeMatcher::you(),ironsmith::replacement::ReplacementAction::Modify(ironsmith::replacement::EventModification::Multiply(2))));
        let shield=game.effect_store.prevention_effects.add_shield(ironsmith::prevention::PreventionShield::prevent_next_n(source,alice,ironsmith::prevention::PreventionTarget::You,3));
        let encoded=SyncExecutableState::retain(&game,&registry,objects,game.effect_store.continuous_effects.registered_state(),game.effect_store.replacement_effects.registered_state().unwrap(),game.effect_store.prevention_effects.retained_state().unwrap()).unwrap();
        let json=serde_json::to_value(&encoded).unwrap();let wire:SyncExecutableState=serde_json::from_value(json.clone()).unwrap();let peers=peer_bindings(&wire);let restored=wire.restore(&peers).unwrap();
        let mut peer=GameState::new(vec!["Alice".into(),"Bob".into()],20);*peer.provenance_graph_mut()=restored.provenance_graph.clone();let mut peer_registry=ironsmith::cards::CardRegistry::new();for definition in &restored.definitions{peer_registry.register(definition.clone());}for object in &restored.objects{peer.add_object(object.clone());}peer.effect_store.continuous_effects.restore_registered_state(restored.continuous.clone()).unwrap();peer.effect_store.replacement_effects.restore_registered_state(restored.replacement.clone()).unwrap();peer.effect_store.prevention_effects.restore_retained_state(restored.prevention.clone()).unwrap();peer.refresh_continuous_state().unwrap();assert_eq!(peer.current_colors(source),Some(ColorSet::RED));
        let reencoded=SyncExecutableState::retain(&peer,&peer_registry,restored.objects,restored.continuous,restored.replacement,restored.prevention).unwrap();assert_eq!(serde_json::to_value(reencoded).unwrap(),json,"all manager graph canonical reencoding");
        let mut ctx=ironsmith::effects::EffectContext::new_default(source,alice);let gain=Effect::new(ironsmith::effects::GainLifeEffect::new(3,ironsmith::target::ChooseSpec::SpecificPlayer(alice)));let out=ironsmith::effects::execute_effect(&mut peer,&gain,&mut ctx).unwrap();assert_eq!(out.as_count(),Some(6));assert_eq!(peer.player(alice).unwrap().life,26);assert!(peer.effect_store.replacement_effects.get_effect(replacement).is_none());let damage=Effect::new(ironsmith::effects::DealDamageEffect::new(5,ironsmith::target::ChooseSpec::SpecificPlayer(alice)));let out=ironsmith::effects::execute_effect(&mut peer,&damage,&mut ctx).unwrap();assert_eq!(peer.player(alice).unwrap().life,24);let amounts:Vec<_>=out.events.iter().filter_map(|e|e.downcast::<ironsmith::events::DamageEvent>()).map(|e|e.amount).collect();assert_eq!(amounts,vec![2]);assert_eq!(peer.effect_store.prevention_effects.prevented_by_shield(shield),3);assert!(peer.effect_store.prevention_effects.get_shield_mut(shield).is_none());
        let independent=Effect::new(ironsmith::effects::GainLifeEffect::new(1,ironsmith::target::ChooseSpec::SpecificPlayer(alice)));let out=ironsmith::effects::execute_effect(&mut peer,&independent,&mut ctx).unwrap();assert_eq!(out.as_count(),Some(1));assert_eq!(peer.player(alice).unwrap().life,25);
    }
    #[test] fn owning_sync_executable_all_managers_retain_and_restore_actual_native_gameplay(){check_all_managers(false);}
    #[test] fn owning_sync_executable_all_managers_retain_and_restore_actual_compiled_gameplay(){check_all_managers(true);}
    #[test]
    fn owning_sync_executable_restores_live_programs_shared_aliases_and_registered_effects() {
        check_restore(false);
    }
    #[test]
    fn owning_sync_executable_rejects_missing_fields_duplicate_roots_and_bad_bindings() {
        check_rejections(false);
    }
    #[test]
    fn owning_sync_executable_approved_hidden_placeholder_does_not_enroll_private_program() {
        check_approved_hidden_root(false);
    }
    #[test]
    fn owning_sync_executable_compiled_restores_live_programs_shared_aliases_and_registered_effects()
     {
        check_restore(true);
    }
    #[test]
    fn owning_sync_executable_compiled_rejects_missing_fields_duplicate_roots_and_bad_bindings() {
        check_rejections(true);
    }
    #[test]
    fn owning_sync_executable_compiled_approved_hidden_placeholder_does_not_enroll_private_program()
    {
        check_approved_hidden_root(true);
    }
    #[test]
    fn owning_sync_executable_retains_queued_provenance_graph() {
        let _guard=crate::test_id_counter_guard();
        let (mut game,registry,objects,_)=history_inputs(false);
        let source=objects[0].id;let alice=PlayerId::from_index(0);
        let root=game.provenance_graph_mut().alloc_root_event(ironsmith::events::EventKind::Damage);
        let child=game.provenance_graph_mut().alloc_child(root,ironsmith::provenance::ProvenanceNodeKind::EffectExecution{source,controller:alice});
        let mut prevention=game.effect_store.prevention_effects.retained_state().unwrap();prevention.pending_follow_ups[0].provenance=child;
        let state=SyncExecutableState::retain(&game,&registry,objects,game.effect_store.continuous_effects.registered_state(),game.effect_store.replacement_effects.registered_state().unwrap(),prevention).unwrap();
        let json=serde_json::to_value(&state).unwrap();
        let graph=json.get("provenanceGraph").expect("queued provenance needs its complete owning graph");
        assert_eq!(graph["next_id"],child.raw());
        assert_eq!(graph["nodes"][child.raw() as usize-1]["id"],child.raw());
        assert_eq!(graph["nodes"][child.raw() as usize-1]["parent"],root.raw());
        assert_eq!(json["prevention"]["pending_follow_ups"][0]["provenance"],child.raw());
        let restored=state.restore(&peer_bindings(&state)).unwrap();
        assert!(restored.provenance_graph.is_descendant_of(child,root));
        let mut peer=GameState::new(vec!["Alice".into(),"Bob".into()],20);
        *peer.provenance_graph_mut()=restored.provenance_graph.clone();
        for object in restored.objects {peer.add_object(object);}
        peer.effect_store.continuous_effects.restore_registered_state(restored.continuous).unwrap();
        peer.effect_store.replacement_effects.restore_registered_state(restored.replacement).unwrap();
        let mut pending=restored.prevention;pending.shields.clear();pending.follow_up_deferral_depth=0;pending.follow_up_replacement_scopes.clear();
        peer.effect_store.prevention_effects.restore_retained_state(pending).unwrap();
        let mut dm=ironsmith::decision::SelectFirstDecisionMaker;
        let run=|peer:&mut GameState,dm:&mut ironsmith::decision::SelectFirstDecisionMaker|ironsmith::events::processing::process_damage_assignments_with_event_with_source_snapshot_opts_with_dm(peer,source,ironsmith::events::DamageTarget::Player(alice),1,false,true,ironsmith::events::cause::EventCause::effect(),None,dm).unwrap();
        let _=run(&mut peer,&mut dm);
        assert_eq!(peer.player(alice).unwrap().life,22,"restored queue executes through owning damage processing");
        assert!(peer.effect_store.prevention_effects.retained_state().unwrap().pending_follow_ups.is_empty());
        let gains:Vec<_>=peer.take_pending_trigger_events().into_iter().filter(|event|event.downcast::<ironsmith::events::LifeGainEvent>().is_some()).collect();
        assert_eq!(gains.len(),1);assert!(peer.provenance_graph().is_descendant_of(gains[0].provenance(),child));
        let _=run(&mut peer,&mut dm);assert_eq!(peer.player(alice).unwrap().life,22);assert!(peer.effect_store.prevention_effects.retained_state().unwrap().pending_follow_ups.is_empty());
        assert!(peer.take_pending_trigger_events().iter().all(|event|event.downcast::<ironsmith::events::LifeGainEvent>().is_none()));
        let mut invalid=serde_json::to_value(&state).unwrap();invalid["provenanceGraph"]["nodes"][child.raw() as usize-1]["parent"]=serde_json::json!(child.raw());
        let invalid:SyncExecutableState=serde_json::from_value(invalid).unwrap();assert!(invalid.restore(&peer_bindings(&state)).is_err());
        let mut invalid=state.clone();invalid.prevention.pending_follow_ups[0].provenance=serde_json::from_value(serde_json::json!(u64::MAX)).unwrap();assert!(invalid.restore(&peer_bindings(&state)).is_err());

    }
    #[test]
    fn owning_sync_executable_rejects_dangling_queued_provenance_before_policy() {
        let _guard=crate::test_id_counter_guard();
        let (game,registry,objects,_)=history_inputs(false);
        let mut prevention=game.effect_store.prevention_effects.retained_state().unwrap();
        prevention.pending_follow_ups[0].provenance=serde_json::from_value(serde_json::json!(u64::MAX)).unwrap();
        let mut calls=0;
        let error=SyncExecutableState::retain_with_history_policy(&game,&registry,objects,game.effect_store.continuous_effects.registered_state(),game.effect_store.replacement_effects.registered_state().unwrap(),prevention,|snapshot|{calls+=1;Ok::<_,String>(snapshot)}).unwrap_err();
        assert!(error.contains("provenance"));assert_eq!(calls,0,"invalid owning reference must fail before disclosure callbacks");
    }

}

#[cfg(test)]
mod public_registered_replacement_prevention_checkpoint_contract_tests {
    use super::*;
    use ironsmith::effects::{EffectContext,execute_effect,GainLifeEffect,DealDamageEffect};
    use ironsmith::target::{ChooseSpec,PlayerFilter};
    fn setup()->(WasmGame,CardDefinition,ObjectId,PlayerId) {
        let alice=PlayerId::from_index(0);let mut host=WasmGame::new();host.initialize_empty_match(vec!["Alice".into(),"Bob".into()],20,1);
        let definition=CardDefinition::new(ironsmith::CardBuilder::new(CardId::new(),"Registered event checkpoint owner").card_types(vec![CardType::Artifact]).build());host.registry.register(definition.clone());let source=host.game.create_object_from_definition(&definition,alice,Zone::Battlefield);(host,definition,source,alice)
    }
    fn restore(host:&WasmGame,definition:CardDefinition)->WasmGame {
        let checkpoint=host.build_sync_checkpoint();let wire=serde_json::to_string(&checkpoint).unwrap();let checkpoint:SyncCheckpoint=serde_json::from_str(&wire).unwrap();let mut guest=WasmGame::new();guest.initialize_empty_match(vec!["Carol".into(),"Dan".into()],20,2);guest.registry.register(definition);guest.apply_sync_checkpoint(checkpoint).expect("public registered-event checkpoint imports");guest
    }
    #[test]fn public_plain_checkpoint_gameplay_control(){let _g=crate::test_id_counter_guard();let (mut host,definition,source,alice)=setup();let mut guest=restore(&host,definition);for peer in [&mut host,&mut guest]{let mut ctx=EffectContext::new_default(source,alice);let out=execute_effect(&mut peer.game,&ironsmith::Effect::new(GainLifeEffect::new(3,ChooseSpec::SpecificPlayer(alice))),&mut ctx).unwrap();assert_eq!(out.as_count(),Some(3));assert_eq!(peer.game.player(alice).unwrap().life,23);}}
    #[test]fn public_checkpoint_preserves_registered_one_shot_replacement_gameplay(){let _g=crate::test_id_counter_guard();let (mut host,definition,source,alice)=setup();let effect=ironsmith::replacement::ReplacementEffect::with_matcher(source,alice,ironsmith::events::life::matchers::WouldGainLifeMatcher::new(PlayerFilter::Specific(alice)),ironsmith::replacement::ReplacementAction::Modify(ironsmith::replacement::EventModification::Multiply(2)));let id=host.game.effect_store.replacement_effects.add_one_shot_effect(effect);let mut guest=restore(&host,definition);for peer in [&mut host,&mut guest]{let mut ctx=EffectContext::new_default(source,alice);let out=execute_effect(&mut peer.game,&ironsmith::Effect::new(GainLifeEffect::new(3,ChooseSpec::SpecificPlayer(alice))),&mut ctx).unwrap();assert_eq!(out.as_count(),Some(6),"public import must retain the executable replacement, not just object metadata");assert_eq!(peer.game.player(alice).unwrap().life,26);assert!(peer.game.effect_store.replacement_effects.get_effect(id).is_none(),"one shot consumed once");}}
    #[test]fn public_checkpoint_preserves_registered_prevention_gameplay(){let _g=crate::test_id_counter_guard();let (mut host,definition,source,alice)=setup();let shield=ironsmith::prevention::PreventionShield::new(source,alice,ironsmith::prevention::PreventionTarget::Player(alice),Some(3),ironsmith::effect::Until::EndOfTurn);let id=host.game.effect_store.prevention_effects.add_shield(shield);let mut guest=restore(&host,definition);for peer in [&mut host,&mut guest]{let mut ctx=EffectContext::new_default(source,alice);execute_effect(&mut peer.game,&ironsmith::Effect::new(DealDamageEffect::new(5,ChooseSpec::SpecificPlayer(alice))),&mut ctx).unwrap();assert_eq!(peer.game.player(alice).unwrap().life,18,"public import must retain the shield before damage occurs");assert_eq!(peer.game.effect_store.prevention_effects.prevented_by_shield(id),3);}}
}

#[cfg(test)]
mod public_replacement_hidden_history_checkpoint_contract_tests {
    use super::*;
    fn check_private_capture(compiled:bool){
        let _guard=crate::test_id_counter_guard();let alice=PlayerId::from_index(0);let bob=PlayerId::from_index(1);let mut host=WasmGame::new();host.initialize_empty_match(vec!["Alice".into(),"Bob".into()],20,1);
        let definition=CardDefinition::new(ironsmith::CardBuilder::new(CardId::new(),"Public replacement capture owner").card_types(vec![CardType::Artifact]).build());host.registry.register(definition.clone());let source=host.game.create_object_from_definition(&definition,alice,Zone::Battlefield);let stable=host.game.object(source).unwrap().stable_id;
        let private=if compiled{ironsmith_registry_test::compile_to_runtime_definition("Private replacement capture marker","Type: Sorcery\nYou gain 7777 life.",false).unwrap()}else{ironsmith::cards::builders::CardDefinitionBuilder::new(CardId::new(),"Private replacement capture marker").card_types(vec![CardType::Sorcery]).with_spell_effect(vec![ironsmith::Effect::gain_life(7777)]).build()};host.registry.register(private.clone());let hidden=host.game.create_object_from_definition(&private,bob,Zone::Library);host.game.set_hidden_card_info(hidden,HiddenCardInfo{owner:bob,zone:Zone::Library,slot:0,commitment:"private-capture-commitment".into(),origin_slot:None,origin_commitment:None,public_slot:None,public_commitment:None});let mut saved=ironsmith::snapshot::ObjectSnapshot::from_object(host.game.object(hidden).unwrap(),&host.game);saved.chosen_object=Some(Box::new(saved.clone()));saved.attachment_snapshots.push(saved.chosen_object.as_ref().unwrap().as_ref().clone());
        let matcher=ironsmith::events::zones::matchers::WouldChangeZoneMatcher::new(ironsmith::target::ObjectFilter::permanent(),Some(Zone::Battlefield),Some(Zone::Graveyard)).with_frozen_tagged_objects(std::collections::HashMap::from([(ironsmith::tag::TagKey::from("private-history"),vec![saved])]));host.game.effect_store.replacement_effects.add_resolution_effect(ironsmith::replacement::ReplacementEffect::with_matcher(source,alice,matcher,ironsmith::replacement::ReplacementAction::ChangeDestination(Zone::Exile)));
        let checkpoint=host.try_build_redacted_executable_checkpoint(alice).unwrap();let json=serde_json::to_string(&checkpoint).unwrap();assert!(!json.contains("Private replacement capture marker"),"hidden captured identity must not be reachable from public manager graph");assert!(!json.contains("7777"),"hidden captured program must not be reachable from public manager graph");let checkpoint:SyncCheckpoint=serde_json::from_str(&json).unwrap();let mut guest=WasmGame::new();guest.initialize_empty_match(vec!["Carol".into(),"Dan".into()],20,2);guest.registry.register(definition);guest.apply_sync_checkpoint(checkpoint).unwrap();assert!(guest.game.is_hidden_card_placeholder(hidden));
        for peer in [&mut host,&mut guest]{let mut ctx=ironsmith::effects::EffectContext::new_default(source,alice);ironsmith::effects::execute_effect(&mut peer.game,&ironsmith::Effect::move_to_zone(ironsmith::target::ChooseSpec::SpecificObject(source),Zone::Graveyard,true),&mut ctx).unwrap();let moved=peer.game.objects_in_deterministic_order().into_iter().find(|object|object.stable_id==stable).unwrap();assert_eq!(moved.zone,Zone::Exile,"public import must retain registered replacement while projecting its private historical captures");}
    }
    #[test]fn public_native_replacement_hidden_history_preserves_behavior_without_private_program(){check_private_capture(false);}
    #[test]fn public_compiled_replacement_hidden_history_preserves_behavior_without_private_program(){check_private_capture(true);}
}

#[cfg(test)]
mod public_full_executable_checkpoint_validation_tests {
    use super::*;
    fn full_fixture() -> (WasmGame, ObjectId, PlayerId) {
        let alice = PlayerId::from_index(0);
        let mut host = WasmGame::new();
        host.initialize_empty_match(vec!["Alice".into(), "Bob".into()], 20, 1);
        let definition = ironsmith::cards::builders::CardDefinitionBuilder::new(CardId::new(), "Full checkpoint typed owner")
            .card_types(vec![CardType::Artifact])
            .with_spell_effect(vec![ironsmith::Effect::gain_life(2)])
            .build();
        host.registry.register(definition.clone());
        let source = host.game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        host.game.effect_store.replacement_effects.add_one_shot_effect(ironsmith::replacement::ReplacementEffect::with_matcher(
            source, alice, ironsmith::events::life::matchers::WouldGainLifeMatcher::you(),
            ironsmith::replacement::ReplacementAction::Modify(ironsmith::replacement::EventModification::Multiply(2)),
        ));
        let root = host.game.provenance_graph_mut().alloc_root_event(ironsmith::events::EventKind::Damage);
        host.game.provenance_graph_mut().alloc_child(root, ironsmith::provenance::ProvenanceNodeKind::EffectExecution { source, controller: alice });
        (host, source, alice)
    }
    type StackFixtureHistory = ironsmith_runtime_catalog::artifact_materializer::RetainedOccurrenceObjectSnapshot<u32>;
    type StackFixtureEvent = ironsmith::events::raw_event::RetainedRawEvent<PlayerId, StackFixtureHistory>;
    type Carrier = ironsmith::game_state::RetainedStackEntry<ironsmith_runtime_catalog::artifact_materializer::RetainedOccurrenceProgram, StackFixtureHistory, StackFixtureEvent, ironsmith::effect::RetainedEffectOutcome<StackFixtureEvent>, ()>;
    fn unsupported<T>() -> Result<T, ironsmith_runtime_catalog::artifact_materializer::OccurrenceBindingError> {
        Err(ironsmith_runtime_catalog::artifact_materializer::OccurrenceBindingError::InvalidModel { detail: "fixture has no such payload".into() })
    }
    fn retain_stack_fixture_event(encoder: &mut ironsmith_runtime_catalog::artifact_materializer::StaticAbilityOccurrenceEncoder,
        event: ironsmith::triggers::TriggerEvent) -> Result<StackFixtureEvent, ironsmith_runtime_catalog::artifact_materializer::OccurrenceBindingError> {
        event.try_retain(encoder,
            |_, body| body.as_any().downcast_ref::<ironsmith::events::phase::BeginningOfEndStepEvent>()
                .map(|phase| phase.player).ok_or_else(|| ironsmith_runtime_catalog::artifact_materializer::OccurrenceBindingError::InvalidModel { detail: "fixture event kind differs".into() }),
            |encoder, history| encoder.encode_snapshot(history, |id| Ok(id.0)))
    }
    fn restore_stack_fixture_event(decoder: &mut ironsmith_runtime_catalog::artifact_materializer::StaticAbilityOccurrenceDecoder,
        event: StackFixtureEvent, peer_face: ironsmith::CardId) -> Result<ironsmith::triggers::TriggerEvent, ironsmith_runtime_catalog::artifact_materializer::OccurrenceBindingError> {
        event.try_restore(decoder,
            |_, player| Ok(std::sync::Arc::new(ironsmith::events::phase::BeginningOfEndStepEvent::new(player)) as std::sync::Arc<dyn ironsmith::events::GameEventType>),
            |decoder, history| decoder.restore_snapshot(history, |_| Ok(peer_face)))
    }
    #[test]
    fn complete_stack_carrier_preserves_program_and_announced_context_through_json() {
        let _guard = crate::test_id_counter_guard();
        let (mut host, source, alice) = full_fixture();
        let mut entry = ironsmith::game_state::StackEntry::ability(source, alice, vec![ironsmith::Effect::gain_life(ironsmith::effect::Value::X)]);
        let snapshot = ironsmith::snapshot::ObjectSnapshot::from_object(host.game.object(source).unwrap(), &host.game);
        entry.source_snapshot = Some(snapshot.clone());
        let event_provenance = host.game.provenance_graph_mut().alloc_root_event(ironsmith::events::EventKind::BeginningOfEndStep);
        entry.provenance = event_provenance;
        entry.triggering_event = Some(ironsmith::triggers::TriggerEvent::new_with_provenance(
            ironsmith::events::phase::BeginningOfEndStepEvent::new(alice), event_provenance)
            .with_simultaneous_batch(event_provenance).with_source_snapshot(snapshot.clone())
            .with_lookback_source_snapshots(vec![snapshot.clone()])
            .with_player_tags([("captured player".into(), vec![alice])].into_iter().collect()));
        entry.tagged_objects.insert("captured source".into(), vec![snapshot.clone()]);
        let event = entry.triggering_event.as_ref().unwrap().clone();
        let mut outcome = ironsmith::effect::EffectOutcome::count(9).with_event(event.clone());
        outcome.execution_facts.push(ironsmith::effect::ExecutionFact::PlayerCounts(vec![(alice, 9)]));
        outcome.instruction_result = Some(Box::new(ironsmith::effect::EffectOutcome::count(11).with_event(event)));
        assert!(serde_json::to_value(&outcome).unwrap().get("events").is_none(),
            "legacy event-free serializer cannot serve executable stack outcomes");
        entry.effect_outcomes.insert(ironsmith::effect::EffectId(7), outcome);
        entry.x_value = Some(3);
        entry.activation_cost_has_x = true;
        entry.activation_cost_has_tap = true;
        entry.mana_spent_on_activation.green = 2;
        entry.casting_method = ironsmith::CastingMethod::GrantedEscape { source, exile_count: 4 };
        entry.optional_costs_paid.cast_at_sorcery_timing = true;
        entry.defending_player = Some(PlayerId::from_index(1));
        entry.chosen_player = Some(alice);
        entry.chapter_ability_source = Some(source);
        entry.battle_defeat_source = Some(source);
        entry.event_value_amount = Some(9);
        entry.trigger_identity = Some(ironsmith::triggers::TriggerIdentity(17));
        entry.ability_index = Some(2);
        entry.chosen_modes = Some(vec![1, 3]);
        entry.crew_contributors = vec![ObjectId::from_raw(72)];
        entry.saddle_contributors = vec![ObjectId::from_raw(73)];
        let mut encoder = ironsmith_runtime_catalog::artifact_materializer::StaticAbilityOccurrenceEncoder::default();
        let carrier: Carrier = entry.try_retain(&mut encoder,
            |encoder, program| encoder.encode_program_with_card_graph(program, |id| Ok(id.0)),
            |encoder, history| encoder.encode_snapshot(history, |id| Ok(id.0)),
            retain_stack_fixture_event,
            |encoder, outcome| outcome.try_retain(encoder, retain_stack_fixture_event),
            |_, _| unsupported()).unwrap();
        let wire = serde_json::to_value(&carrier).unwrap();
        for field in ["ability_effects", "source_snapshot", "triggering_event", "x_value", "defending_player", "chosen_modes"] {
            let mut omitted = wire.clone();
            omitted.as_object_mut().unwrap().remove(field);
            assert!(serde_json::from_value::<Carrier>(omitted).is_err(), "missing {field} cannot restore a default");
        }
        let carrier: Carrier = serde_json::from_value(wire).unwrap();
        let peer_face = ironsmith::CardId::new();
        let mut decoder = ironsmith_runtime_catalog::artifact_materializer::StaticAbilityOccurrenceDecoder::restore_with_card_graph::<u32>(encoder.into_table(),
            |_| Ok(peer_face)).unwrap();
        let restored = carrier.try_restore(&mut decoder,
            |decoder, program| decoder.restore_program(program),
            |decoder, history| decoder.restore_snapshot(history, |_| Ok(peer_face)),
            |decoder, event| restore_stack_fixture_event(decoder, event, peer_face),
            |decoder, outcome| outcome.try_restore(decoder, |decoder, event| restore_stack_fixture_event(decoder, event, peer_face)),
            |_, _| unsupported()).unwrap();
        let outcome = &restored.effect_outcomes[&ironsmith::effect::EffectId(7)];
        assert_eq!(outcome.value, ironsmith::effect::OutcomeValue::Count(9));
        assert_eq!(outcome.execution_facts, vec![ironsmith::effect::ExecutionFact::PlayerCounts(vec![(alice, 9)])]);
        assert_eq!(outcome.events.len(), 1);
        assert_eq!(outcome.events[0].source_snapshot().unwrap().card, Some(peer_face));
        let authored = outcome.instruction_result.as_ref().unwrap();
        assert_eq!(authored.value, ironsmith::effect::OutcomeValue::Count(11));
        assert_eq!(authored.events.len(), 1);
        assert_eq!(authored.events[0].lookback_source_snapshots()[0].card, Some(peer_face));
        let event = restored.triggering_event.as_ref().unwrap();
        assert_eq!(restored.provenance, event_provenance);
        assert_eq!(event.provenance(), event_provenance);
        assert_eq!(event.simultaneous_batch(), Some(event_provenance));
        assert_eq!(event.downcast::<ironsmith::events::phase::BeginningOfEndStepEvent>().unwrap().player, alice);
        assert_eq!(event.source_snapshot().unwrap().card, Some(peer_face));
        assert_eq!(event.lookback_source_snapshots()[0].card, Some(peer_face));
        assert_eq!(event.player_tags()["captured player"], vec![alice]);
        assert_ne!(snapshot.card, Some(peer_face));
        assert_eq!(restored.source_snapshot.as_ref().unwrap().card, Some(peer_face));
        assert_eq!(restored.tagged_objects["captured source"][0].card, Some(peer_face),
            "source and tagged histories share the receiver face binding");
        assert!(restored.activation_cost_has_x && restored.activation_cost_has_tap);
        assert_eq!(restored.mana_spent_on_activation.green, 2);
        assert_eq!(restored.casting_method, ironsmith::CastingMethod::GrantedEscape { source, exile_count: 4 });
        assert!(restored.optional_costs_paid.cast_at_sorcery_timing);
        assert_eq!(restored.defending_player, Some(PlayerId::from_index(1)));
        assert_eq!(restored.chosen_player, Some(alice));
        assert_eq!(restored.chapter_ability_source, Some(source));
        assert_eq!(restored.battle_defeat_source, Some(source));
        assert_eq!(restored.event_value_amount, Some(9));
        assert_eq!(restored.trigger_identity, Some(ironsmith::triggers::TriggerIdentity(17)));
        assert_eq!(restored.ability_index, Some(2));
        assert_eq!(restored.chosen_modes, Some(vec![1, 3]));
        assert_eq!(restored.crew_contributors, vec![ObjectId::from_raw(72)]);
        assert_eq!(restored.saddle_contributors, vec![ObjectId::from_raw(73)]);
        let mut game = host.game;
        game.push_to_stack(restored);
        ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
        assert_eq!(game.player(alice).unwrap().life, 26,
            "restored program executes captured X instead of printed source text");
    }
    #[test]
    fn public_full_executable_checkpoint_preserves_pending_stack_ability_program() {
        let _guard = crate::test_id_counter_guard();
        let (mut host, source, alice) = full_fixture();
        let mut entry = ironsmith::game_state::StackEntry::ability(source, alice,
            vec![ironsmith::Effect::gain_life(ironsmith::effect::Value::X)]);
        entry.x_value = Some(3);
        entry.source_stable_id = Some(host.game.object(source).unwrap().stable_id);
        entry.source_snapshot = Some(ironsmith::snapshot::ObjectSnapshot::from_object(host.game.object(source).unwrap(), &host.game));
        host.game.push_to_stack(entry);
        let checkpoint = host.try_build_full_sync_checkpoint().unwrap();
        let checkpoint = serde_json::from_slice(&serde_json::to_vec(&checkpoint).unwrap()).unwrap();
        let mut guest = WasmGame::new();
        guest.initialize_empty_match(vec!["Carol".into(), "Dan".into()], 20, 2);
        guest.apply_sync_checkpoint(checkpoint).unwrap();
        ironsmith::game_loop::resolve_stack_entry(&mut host.game).unwrap();
        assert_eq!(host.game.player(alice).unwrap().life, 26);
        ironsmith::game_loop::resolve_stack_entry(&mut guest.game).unwrap();
        assert_eq!(guest.game.player(alice).unwrap().life, 26,
            "checkpoint must preserve captured X and the stack ability program independently of printed source text");
    }
    #[test]
    fn public_full_executable_checkpoint_preserves_rich_stack_context_and_event_aliases() {
        let _guard = crate::test_id_counter_guard();
        for compiled in [false, true] {
            for departed in [false, true] {
                let (mut host, source, alice) = full_fixture();
                let program = if compiled {
                    ironsmith_registry_test::compile_to_runtime_definition("Captured stack program", "Type: Sorcery\nYou gain X life.", false).unwrap().spell_effect.unwrap()
                } else { ironsmith::resolution::ResolutionProgram::from_effects(vec![ironsmith::Effect::gain_life(ironsmith::effect::Value::X)]) };
                let snapshot = ironsmith::snapshot::ObjectSnapshot::from_object(host.game.object(source).unwrap(), &host.game);
                let provenance = host.game.provenance_graph_mut().alloc_root_event(ironsmith::events::EventKind::BeginningOfEndStep);
                let shared = ironsmith::triggers::TriggerEvent::new_with_provenance(
                    ironsmith::events::phase::BeginningOfEndStepEvent::new(alice), provenance)
                    .with_simultaneous_batch(provenance).with_source_snapshot(snapshot.clone())
                    .with_lookback_source_snapshots(vec![snapshot.clone()])
                    .with_player_tags([("captured actor".into(), vec![alice])].into_iter().collect());
                let distinct = ironsmith::triggers::TriggerEvent::new_with_provenance(
                    ironsmith::events::phase::BeginningOfEndStepEvent::new(alice), provenance);
                let mut entry = StackEntry::ability(source, alice, program);
                entry.x_value = Some(3); entry.provenance = provenance;
                entry.activation_cost_has_x = true; entry.activation_cost_has_tap = true;
                entry.mana_spent_on_activation.green = 2;
                entry.mana_usage_restrictions = vec![ironsmith::ability::ManaUsageRestriction::ActivateAbility];
                entry.optional_costs_paid.cast_at_sorcery_timing = true;
                entry.source_stable_id = Some(snapshot.stable_id); entry.source_snapshot = Some(snapshot.clone());
                entry.source_name = Some("Captured source context".into());
                entry.triggering_event = Some(shared.clone()); entry.event_value_amount = Some(9);
                entry.tagged_objects.insert("captured source".into(), vec![snapshot.clone()]);
                let mut outcome = ironsmith::effect::EffectOutcome::count(9).with_event(shared.clone()).with_event(distinct);
                outcome.execution_facts.push(ironsmith::effect::ExecutionFact::PlayerCounts(vec![(alice, 9)]));
                outcome.instruction_result = Some(Box::new(ironsmith::effect::EffectOutcome::count(11).with_event(shared)));
                entry.effect_outcomes.insert(ironsmith::effect::EffectId(7), outcome);
                host.game.push_to_stack(entry);
                let live_source = if departed { host.game.move_object(source, Zone::Graveyard, ironsmith::events::cause::EventCause::effect()).unwrap() } else { source };
                let checkpoint = host.try_build_full_sync_checkpoint().unwrap();
                assert_eq!(checkpoint.executable_state.as_ref().unwrap().event_bodies.len(), 2,
                    "cloned body is shared, equal-looking separate event is distinct");
                let checkpoint = serde_json::from_slice(&serde_json::to_vec(&checkpoint).unwrap()).unwrap();
                let mut guest = WasmGame::new();
                guest.initialize_empty_match(vec!["Carol".into(), "Dan".into()], 20, 2);
                guest.apply_sync_checkpoint(checkpoint).unwrap();
                let restored = &guest.game.stack[0];
                assert_eq!(restored.x_value, Some(3));
                assert!(restored.activation_cost_has_x && restored.activation_cost_has_tap);
                assert_eq!(restored.mana_spent_on_activation.green, 2);
                assert!(matches!(restored.mana_usage_restrictions.as_slice(), [ironsmith::ability::ManaUsageRestriction::ActivateAbility]));
                assert!(restored.optional_costs_paid.cast_at_sorcery_timing);
                let face = guest.game.object(live_source).unwrap().card;
                assert_eq!(restored.source_snapshot.as_ref().unwrap().card, face);
                assert_eq!(restored.tagged_objects["captured source"][0].card, face);
                let event = restored.triggering_event.as_ref().unwrap();
                let outcome = &restored.effect_outcomes[&ironsmith::effect::EffectId(7)];
                assert!(std::ptr::eq(event.inner(), outcome.events[0].inner()), "cloned event occurrence lost its shared identity");
                assert!(!std::ptr::eq(event.inner(), outcome.events[1].inner()), "equal-looking separate events were merged");
                assert!(std::ptr::eq(event.inner(), outcome.instruction_result.as_ref().unwrap().events[0].inner()));
                assert_eq!(event.source_snapshot().unwrap().card, face);
                assert_eq!(event.simultaneous_batch(), Some(provenance));
                assert_eq!(event.player_tags()["captured actor"], vec![alice]);
                ironsmith::game_loop::resolve_stack_entry(&mut host.game).unwrap();
                ironsmith::game_loop::resolve_stack_entry(&mut guest.game).unwrap();
                assert!(host.game.player(alice).unwrap().life > 20);
                assert_eq!(guest.game.player(alice).unwrap().life, host.game.player(alice).unwrap().life,
                    "native/compiled captured program and context must survive source departure and checkpoint import");
            }
        }
    }
    #[test]
    fn public_full_executable_checkpoint_rejects_malformed_stack_without_mutation() {
        let _guard = crate::test_id_counter_guard();
        for case in 0..5 {
            let (mut host, source, alice) = full_fixture();
            let provenance = host.game.provenance_graph_mut().alloc_root_event(ironsmith::events::EventKind::BeginningOfEndStep);
            let mut entry = StackEntry::ability(source, alice, vec![ironsmith::Effect::gain_life(3)]);
            entry.triggering_event = Some(ironsmith::triggers::TriggerEvent::new_with_provenance(
                ironsmith::events::phase::BeginningOfEndStepEvent::new(alice), provenance));
            host.game.push_to_stack(entry);
            let mut incoming = host.try_build_full_sync_checkpoint().unwrap();
            match case {
                0 => incoming.executable_state.as_mut().unwrap().stack.clear(),
                1 => incoming.executable_state.as_mut().unwrap().stack[0].triggering_event.as_mut().unwrap().inner = u32::MAX,
                2 => { incoming.stack[0].controller = 99; incoming.executable_state.as_mut().unwrap().stack[0].controller = PlayerId::from_index(99); },
                3 => { let SyncEventBody::BeginningOfEndStep { player } = &mut incoming.executable_state.as_mut().unwrap().event_bodies[0] else { panic!("fixture body"); }; *player = PlayerId::from_index(99); },
                _ => incoming.executable_state.as_mut().unwrap().stack[0].target_assignments.push(ironsmith::game_state::TargetAssignment { spec: ironsmith::target::ChooseSpec::Source, range: 0..1 }),
            }
            let before = serde_json::to_value(host.try_build_full_sync_checkpoint().unwrap()).unwrap();
            host.apply_sync_checkpoint(incoming).expect_err("malformed stack cannot publish");
            assert_eq!(serde_json::to_value(host.try_build_full_sync_checkpoint().unwrap()).unwrap(), before,
                "failed stack import must preserve runtime and exported identity allocators");
        }
    }
    #[test]
    fn public_full_executable_checkpoint_preserves_executable_event_payloads() {
        let _guard = crate::test_id_counter_guard();
        let (mut host, source, alice) = full_fixture();
        let shared = ironsmith::Ability::static_ability(ironsmith::static_abilities::StaticAbility::flying());
        std::sync::Arc::make_mut(&mut host.game.object_mut(source).unwrap().abilities).push(shared.clone());
        let activated = ironsmith_registry_test::compile_to_runtime_definition("Captured executable ability", "Type: Artifact\n{T}: You gain X life.", false).unwrap().abilities[0].clone();
        let snapshot = ironsmith::snapshot::ObjectSnapshot::from_object(host.game.object(source).unwrap(), &host.game);
        let mut token = ironsmith::object::Object::new_token(ObjectId::from_raw(9001), alice, "Captured template".into(), vec![CardType::Creature], vec![ironsmith::types::Subtype::Soldier], Some(2), Some(3), ironsmith::color::ColorSet::GREEN);
        token.abilities = vec![shared.clone(), activated.clone()].into();
        let provenance = host.game.provenance_graph_mut().alloc_root_event(ironsmith::events::EventKind::BeginningOfEndStep);
        let mut entry_capture = ironsmith::replacement_entry_capture::RetainedEntryEvent::capture(ironsmith::events::zones::EnterBattlefieldEvent::new(source, Zone::Hand));
        entry_capture.object = source;
        entry_capture.from = Zone::Hand;
        entry_capture.enters_tapped = true;
        entry_capture.enters_with_counters = vec![(ironsmith::CounterType::Named("Entry counter".into()), 7)];
        entry_capture.linked_exile_with_entering = vec![source];
        entry_capture.enters_as_copy_of = Some(source);
        entry_capture.copy_followups = vec![ironsmith_core::EnterAsCopyFollowup::ExileCopiedObject];
        entry_capture.copy_duration = Some(ironsmith::effect::Until::EndOfTurn);
        entry_capture.copy_name_override = Some("Saved copy name".into());
        entry_capture.added_colors = ironsmith::color::ColorSet::GREEN;
        entry_capture.added_card_types = vec![CardType::Creature];
        entry_capture.removes_other_card_types = true;
        entry_capture.added_supertypes = vec![ironsmith::types::Supertype::Legendary];
        entry_capture.removed_supertypes = vec![ironsmith::types::Supertype::Legendary];
        entry_capture.added_subtypes = vec![ironsmith::types::Subtype::Soldier];
        entry_capture.added_abilities = vec![shared.clone(), activated.clone()];
        entry_capture.set_base_power_toughness = Some((4,5));
        entry_capture.controller_override = Some(alice);
        let ironsmith::ability::AbilityKind::Activated(a) = &activated.kind else { panic!("activated fixture"); };
        entry_capture.pending_program = Some((a.effects.clone(), alice));
        entry_capture.program_choices.chosen_player = Some(alice);
        entry_capture.program_choices.noted_life_total = Some(17);
        let ironsmith::ability::AbilityKind::Static(shared_static) = &shared.kind else { panic!("static fixture"); };
        entry_capture.program_choices.power_toughness_choices = vec![(7, 8, vec![shared_static.clone()])];
        entry_capture.program_choices.as_enters_tagged_objects = vec![("captured entry history".into(), vec![snapshot.clone()])];
        entry_capture.prepared_choices = Some(entry_capture.program_choices.clone());
        let bodies: Vec<std::sync::Arc<dyn ironsmith::events::GameEventType>> = vec![
std::sync::Arc::new(ironsmith::events::spells::AbilityActivatedEvent { source: source, activator: alice, is_mana_ability: true, is_loyalty_ability: true, activation_cost_has_x: true, activation_cost_has_tap: true, x_value: Some(3), stack_entry_provenance: Some(provenance), snapshot: Some(snapshot.clone()), activated_ability: Some(activated.clone()), mana_sources_spent: vec![snapshot.clone()], mana_spent_total: 3 }),
std::sync::Arc::new(ironsmith::events::tokens::CreateTokensEvent { controller: alice, count: 3, cause: ironsmith::events::cause::EventCause::from_effect(source, alice), token: Some(token.clone()), additional_tokens: vec![(ironsmith_core::AdditionalTokenKind::Treasure, 2)] }),
std::sync::Arc::new(entry_capture.into_native().unwrap())
        ];
        let mut entry = StackEntry::ability(source, alice, vec![ironsmith::Effect::gain_life(3)]);
        let mut outcome = ironsmith::effect::EffectOutcome::count(9);
        for body in bodies { outcome.events.push(ironsmith::triggers::TriggerEvent::from_boxed_with_provenance(body.clone_box(), provenance)); }
        entry.effect_outcomes.insert(ironsmith::effect::EffectId(7), outcome); host.game.push_to_stack(entry);
        let before = host.try_build_full_sync_checkpoint().unwrap();
        assert_eq!(before.executable_state.as_ref().unwrap().event_bodies.len(), 3);
        let unchanged = serde_json::to_value(&before).unwrap();
        for case in 0..11 {
            let mut forged = before.clone(); let bad = PlayerId::from_index(99);
            let bodies = &mut forged.executable_state.as_mut().unwrap().event_bodies;
            match case {
                0 => { let SyncEventBody::AbilityActivatedEvent { activator, .. } = &mut bodies[0] else { panic!("fixture"); }; *activator = bad; },
                1 => { let SyncEventBody::AbilityActivatedEvent { mana_sources_spent, .. } = &mut bodies[0] else { panic!("fixture"); }; mana_sources_spent[0].owner = bad; },
                2 => { let SyncEventBody::CreateTokensEvent { token, .. } = &mut bodies[1] else { panic!("fixture"); }; token.as_mut().unwrap().owner = bad; },
                3 => { let SyncEventBody::CreateTokensEvent { token, .. } = &mut bodies[1] else { panic!("fixture"); }; token.as_mut().unwrap().initial_controller = bad; },
                4 => { let SyncEventBody::EnterBattlefieldEvent { value } = &mut bodies[2] else { panic!("fixture"); }; value.controller_override = Some(bad); },
                5 => { let SyncEventBody::EnterBattlefieldEvent { value } = &mut bodies[2] else { panic!("fixture"); }; value.pending_program.as_mut().unwrap().1 = bad; },
                6 => { let SyncEventBody::EnterBattlefieldEvent { value } = &mut bodies[2] else { panic!("fixture"); }; value.program_choices.chosen_player = Some(bad); },
                7 => { let SyncEventBody::EnterBattlefieldEvent { value } = &mut bodies[2] else { panic!("fixture"); }; value.prepared_choices.as_mut().unwrap().battle_protector = Some(bad); },
                8 => { let SyncEventBody::EnterBattlefieldEvent { value } = &mut bodies[2] else { panic!("fixture"); }; value.program_choices.as_enters_tagged_objects[0].1[0].controller = bad; },
                9 => { let SyncEventBody::EnterBattlefieldEvent { value } = &mut bodies[2] else { panic!("fixture"); }; value.program_choices.power_toughness_choices[0].2[0].0 = u32::MAX; },
                _ => { let SyncEventBody::AbilityActivatedEvent { stack_entry_provenance, .. } = &mut bodies[0] else { panic!("fixture"); }; *stack_entry_provenance = Some(serde_json::from_value(serde_json::json!(u64::MAX)).unwrap()); },
            }
            let forged = serde_json::from_slice(&serde_json::to_vec(&forged).unwrap()).unwrap();
            assert!(host.apply_sync_checkpoint(forged).is_err(), "invalid executable capture case {case} cannot publish");
            assert_eq!(serde_json::to_value(host.try_build_full_sync_checkpoint().unwrap()).unwrap(), unchanged,
                "failed executable body import must preserve runtime and exported allocators");
        }
        let expected = serde_json::to_value(&before.executable_state.as_ref().unwrap().event_bodies).unwrap();
        let mut projected_count = 0;
        let _ = project_sync_stack_history(&host.game.stack, &mut |h| { projected_count += 1; Ok(h) }).unwrap();
        assert_eq!(projected_count, 5, "activation/mana-source, both prepared histories and stack source");
        let checkpoint = serde_json::from_slice(&serde_json::to_vec(&before).unwrap()).unwrap();
        let mut guest = WasmGame::new(); guest.initialize_empty_match(vec!["Carol".into(), "Dan".into()], 20, 2); guest.apply_sync_checkpoint(checkpoint).unwrap();
        let restored = guest.try_build_full_sync_checkpoint().unwrap();
        assert_eq!(serde_json::to_value(&restored.executable_state.as_ref().unwrap().event_bodies).unwrap(), expected);
        for peer in [&mut host, &mut guest] {
            let events = &peer.game.stack[0].effect_outcomes[&ironsmith::effect::EffectId(7)].events;
            let activated = events[0].downcast::<ironsmith::events::spells::AbilityActivatedEvent>().unwrap();
            let token = events[1].downcast::<ironsmith::events::tokens::CreateTokensEvent>().unwrap().token.as_ref().unwrap();
            let enter = events[2].downcast::<ironsmith::events::zones::EnterBattlefieldEvent>().unwrap();
            let identity = |ability: &ironsmith::Ability| { let ironsmith::ability::AbilityKind::Static(a) = &ability.kind else { panic!("static fixture"); }; a.instance_id() };
            assert_eq!(identity(&peer.game.object(source).unwrap().abilities[0]), identity(&token.abilities[0]));
            assert_eq!(identity(&token.abilities[0]), identity(&enter.added_abilities[0]), "body templates and live roots must share executable identities");
            let native_capture = ironsmith::replacement_entry_capture::RetainedEntryEvent::capture(enter.clone());
            assert_eq!(native_capture.program_choices.noted_life_total, Some(17));
            assert_eq!(native_capture.prepared_choices.as_ref().unwrap().chosen_player, Some(alice));
            assert_eq!(native_capture.program_choices.power_toughness_choices[0].2[0].instance_id(), identity(&token.abilities[0]));
            assert_eq!(native_capture.program_choices.as_enters_tagged_objects[0].1[0].card, peer.game.object(source).unwrap().card);
            let pending = native_capture.pending_program.as_ref().unwrap().0.clone();
            let programs = [activated.activated_ability.as_ref().unwrap(), &token.abilities[1], &enter.added_abilities[1]].map(|ability| {
                let ironsmith::ability::AbilityKind::Activated(a) = &ability.kind else { panic!("captured activated fixture"); }; a.effects.clone()
            });
            ironsmith::game_loop::resolve_stack_entry(&mut peer.game).unwrap();
            assert_eq!(peer.game.player(alice).unwrap().life, 26);
            for program in programs.into_iter().chain(std::iter::once(pending)) { let mut captured = StackEntry::ability(source, alice, program); captured.x_value = Some(3); peer.game.push_to_stack(captured); ironsmith::game_loop::resolve_stack_entry(&mut peer.game).unwrap(); }
            assert_eq!(peer.game.player(alice).unwrap().life, 38, "all captured programs including private pending entry program remain executable after import");
        }
    }
    #[test]
    fn public_full_executable_checkpoint_preserves_data_event_capture_matrix() {
        let _guard = crate::test_id_counter_guard();
        let (mut host, source, alice) = full_fixture();
        let snapshot = ironsmith::snapshot::ObjectSnapshot::from_object(host.game.object(source).unwrap(), &host.game);
        let vote = ironsmith::events::other::PlayerVote { player: alice, option_index: 1, option_name: "Second".into(), object_vote: Some(source) };
        let provenance = host.game.provenance_graph_mut().alloc_root_event(ironsmith::events::EventKind::BeginningOfEndStep);
        let bodies: Vec<std::sync::Arc<dyn ironsmith::events::GameEventType>> = vec![
std::sync::Arc::new(ironsmith::events::damage::DamagePreventedEvent { damage_source: source, target: ironsmith::events::DamageTarget::Player(alice), amount: 3, prevention_source: source, prevention_controller: alice, is_combat: true, target_snapshot: Some(snapshot.clone()), applications: vec![ironsmith::events::damage::PreventedDamage { damage_source: source, target: ironsmith::events::DamageTarget::Player(alice), amount: 2, is_combat: false, target_snapshot: Some(snapshot.clone()) }], prevention_shield: Some(ironsmith::prevention::PreventionShieldId(17)) }),
std::sync::Arc::new(ironsmith::events::other::KeywordActionEvent { action: ironsmith_core::KeywordActionKind::Scry, player: alice, source: source, amount: 3, votes: Some(vec![vote.clone()]), snapshot: Some(snapshot.clone()), player_tags: std::collections::HashMap::from([("captured player".into(), vec![alice])]), object_tags: std::collections::HashMap::from([("captured card".into(), vec![snapshot.clone()])]), combat_phase: Some(3), unlocked_door_triggers: Some(vec![ironsmith::triggers::TriggerIdentity(19)]), unlocked_door_ability_range: Some(1..4), x_value: Some(3), voter_teams: vec![(alice, 2)] }),
std::sync::Arc::new(ironsmith::events::other::MarkersChangedEvent { change_type: ironsmith::events::other::MarkerChangeType::Removed, marker: ironsmith::marker::Marker::Counter(ironsmith::CounterType::Named("Event marker".into())), location: ironsmith::marker::MarkerLocation::Player(alice), amount: 3, count_after: Some(3), source: Some(source), source_controller: Some(alice) }),
std::sync::Arc::new(ironsmith::events::other::PlayersFinishedVotingEvent { source: source, controller: alice, votes: vec![vote.clone()], vote_counts: std::collections::HashMap::from([(1, 1)]), option_names: vec!["First".into(), "Second".into()], player_tags: std::collections::HashMap::from([("captured player".into(), vec![alice])]), voter_teams: vec![(alice, 2)] }),
std::sync::Arc::new(ironsmith::events::spells::AbilityTriggeredEvent { source: source, source_stable_id: snapshot.stable_id, controller: alice, trigger_identity: ironsmith::triggers::TriggerIdentity(23), source_snapshot: Some(snapshot.clone()), cause_kind: Some(ironsmith::events::EventKind::BeginningOfEndStep), cause_object: Some(source), zone_change_cause: Some(ironsmith::events::spells::AbilityTriggerZoneChangeCause { from: Zone::Graveyard, to: Zone::Battlefield, destination_objects: vec![source] }) })
        ];
        let mut entry = StackEntry::ability(source, alice, vec![ironsmith::Effect::gain_life(3)]);
        let mut outcome = ironsmith::effect::EffectOutcome::count(9);
        for body in bodies { outcome.events.push(ironsmith::triggers::TriggerEvent::from_boxed_with_provenance(body.clone_box(), provenance)); }
        entry.effect_outcomes.insert(ironsmith::effect::EffectId(7), outcome); host.game.push_to_stack(entry);
        let before = host.try_build_full_sync_checkpoint().unwrap();
        assert_eq!(before.executable_state.as_ref().unwrap().event_bodies.len(), 5);
        let unchanged = serde_json::to_value(&before).unwrap();
        for case in 0..10 {
            let mut forged = before.clone(); let bad = PlayerId::from_index(99);
            let bodies = &mut forged.executable_state.as_mut().unwrap().event_bodies;
            match case {
                0 => { let SyncEventBody::DamagePreventedEvent { applications, .. } = &mut bodies[0] else { panic!("fixture"); }; applications[0].target = SyncTarget::Player { player: 99 }; },
                1 => { let SyncEventBody::KeywordActionEvent { votes, .. } = &mut bodies[1] else { panic!("fixture"); }; votes.as_mut().unwrap()[0].player = bad; },
                2 => { let SyncEventBody::MarkersChangedEvent { location, .. } = &mut bodies[2] else { panic!("fixture"); }; *location = SyncTarget::Player { player: 99 }; },
                3 => { let SyncEventBody::PlayersFinishedVotingEvent { votes, .. } = &mut bodies[3] else { panic!("fixture"); }; votes[0].player = bad; },
                4 => { let SyncEventBody::AbilityTriggeredEvent { controller, .. } = &mut bodies[4] else { panic!("fixture"); }; *controller = bad; },
                5 => { let SyncEventBody::DamagePreventedEvent { applications, .. } = &mut bodies[0] else { panic!("fixture"); }; applications[0].target_snapshot.as_mut().unwrap().owner = bad; },
                6 => { let SyncEventBody::KeywordActionEvent { object_tags, .. } = &mut bodies[1] else { panic!("fixture"); }; object_tags.values_mut().next().unwrap()[0].controller = bad; },
                7 => { let SyncEventBody::KeywordActionEvent { player_tags, .. } = &mut bodies[1] else { panic!("fixture"); }; player_tags.values_mut().next().unwrap()[0] = bad; },
                8 => { let SyncEventBody::PlayersFinishedVotingEvent { voter_teams, .. } = &mut bodies[3] else { panic!("fixture"); }; voter_teams[0].0 = bad; },
                _ => { let SyncEventBody::PlayersFinishedVotingEvent { vote_counts, .. } = &mut bodies[3] else { panic!("fixture"); }; vote_counts.push((1, 4)); },
            }
            let forged = serde_json::from_slice(&serde_json::to_vec(&forged).unwrap()).unwrap();
            let error = host.apply_sync_checkpoint(forged).unwrap_err();
            assert!(error.contains(if case == 9 { "unique and sorted" } else { "invalid player" }), "capture case {case}: {error}");
            assert_eq!(serde_json::to_value(host.try_build_full_sync_checkpoint().unwrap()).unwrap(), unchanged,
                "failed data event import must preserve runtime and exported allocators");
        }
        assert!(sync_restore_vote_counts(vec![(2, 1), (1, 1)]).is_err());
        let missing = serde_json::json!({"player":0,"option_index":1,"option_name":"Second"});
        assert!(serde_json::from_value::<SyncPlayerVote>(missing).is_err(), "object vote capture must be explicitly present");
        let expected = serde_json::to_value(&before.executable_state.as_ref().unwrap().event_bodies).unwrap();
        let mut projected_count = 0;
        let _ = project_sync_stack_history(&host.game.stack, &mut |h| { projected_count += 1; Ok(h) }).unwrap();
        assert_eq!(projected_count, 6, "five body snapshots and the stack source");
        let checkpoint = serde_json::from_slice(&serde_json::to_vec(&before).unwrap()).unwrap();
        let mut guest = WasmGame::new(); guest.initialize_empty_match(vec!["Carol".into(), "Dan".into()], 20, 2); guest.apply_sync_checkpoint(checkpoint).unwrap();
        let restored = guest.try_build_full_sync_checkpoint().unwrap();
        assert_eq!(serde_json::to_value(&restored.executable_state.as_ref().unwrap().event_bodies).unwrap(), expected);
        let events = &guest.game.stack[0].effect_outcomes[&ironsmith::effect::EffectId(7)].events;
        let prevented = events[0].downcast::<ironsmith::events::damage::DamagePreventedEvent>().unwrap();
        assert_eq!(prevented.applications[0].target_snapshot.as_ref().unwrap().card, guest.game.object(source).unwrap().card);
        let keyword = events[1].downcast::<ironsmith::events::other::KeywordActionEvent>().unwrap();
        assert_eq!(keyword.object_tags["captured card"][0].card, guest.game.object(source).unwrap().card);
        assert_eq!(keyword.unlocked_door_triggers, Some(vec![ironsmith::triggers::TriggerIdentity(19)]));
        assert_eq!(keyword.unlocked_door_ability_range, Some(1..4));
        assert_eq!(keyword.votes.as_ref().unwrap()[0], vote);
        ironsmith::game_loop::resolve_stack_entry(&mut host.game).unwrap(); ironsmith::game_loop::resolve_stack_entry(&mut guest.game).unwrap();
        assert_eq!(host.game.player(alice).unwrap().life, 26); assert_eq!(guest.game.player(alice).unwrap().life, 26);
    }
    #[test]
    fn public_full_executable_checkpoint_preserves_mana_combat_event_captures() {
        let _guard = crate::test_id_counter_guard();
        let (mut host, source, alice) = full_fixture();
        let snapshot = ironsmith::snapshot::ObjectSnapshot::from_object(host.game.object(source).unwrap(), &host.game);
        let provenance = host.game.provenance_graph_mut().alloc_root_event(ironsmith::events::EventKind::BeginningOfEndStep);
        let bodies: Vec<std::sync::Arc<dyn ironsmith::events::GameEventType>> = vec![
std::sync::Arc::new(ironsmith::events::combat::CreatureAttackedEvent { attacker: source, target: ironsmith::triggers::event::AttackEventTarget::Player(alice), total_attackers: 2, declared_attackers: Some(vec![ironsmith::combat_state::AttackerInfo { creature: source, target: ironsmith::combat_state::AttackTarget::Nothing { defending_player: Some(alice), was_planeswalker: true } }].into()) }),
std::sync::Arc::new(ironsmith::events::combat::CreatureAttackedAndUnblockedEvent { attacker: source, target: ironsmith::triggers::event::AttackEventTarget::Player(alice) }),
std::sync::Arc::new(ironsmith::events::combat::CreatureBecameBlockedEvent { attacker: source, blocker_count: 3, blockers: vec![source], attack_target: Some(ironsmith::triggers::event::AttackEventTarget::Battle(source)), attacker_snapshot: Some(snapshot.clone()), blocker_snapshots: vec![snapshot.clone()] }),
std::sync::Arc::new(ironsmith::events::mana::ManaAddedEvent { source: source, controller: alice, player: alice, mana: vec![ironsmith::mana::ManaSymbol::Green, ironsmith::mana::ManaSymbol::Colorless], snapshot: Some(snapshot.clone()), provenance: ironsmith::events::mana::ManaProductionProvenance::TappedSourceForMana }),
std::sync::Arc::new(ironsmith::events::mana::ManaUnitSpentEvent { player: alice, mana_source: source, payment_source: Some(source), symbol: ironsmith::mana::ManaSymbol::Green, purpose: ironsmith::ability::ManaPaymentPurpose::ActivateManaAbility, source_snapshot: Some(snapshot.clone()) }),
std::sync::Arc::new(ironsmith::events::other::ObjectBecameUnattachedEvent { object: source, previous_target: ironsmith::object::AttachmentTarget::Player(alice), controller: alice, snapshot: Some(snapshot.clone()) })
        ];
        let mut entry = StackEntry::ability(source, alice, vec![ironsmith::Effect::gain_life(3)]);
        let mut outcome = ironsmith::effect::EffectOutcome::count(9);
        for body in bodies { outcome.events.push(ironsmith::triggers::TriggerEvent::from_boxed_with_provenance(body.clone_box(), provenance)); }
        entry.effect_outcomes.insert(ironsmith::effect::EffectId(7), outcome);
        host.game.push_to_stack(entry);
        let before = host.try_build_full_sync_checkpoint().unwrap();
        assert_eq!(before.executable_state.as_ref().unwrap().event_bodies.len(), 6);
        let unchanged = serde_json::to_value(&before).unwrap();
        for case in 0..7 {
            let mut forged = before.clone(); let bad = PlayerId::from_index(99);
            let bodies = &mut forged.executable_state.as_mut().unwrap().event_bodies;
            match case {
                0 => { let SyncEventBody::CreatureAttackedEvent { target, .. } = &mut bodies[0] else { panic!("fixture"); }; *target = SyncAttackEventTarget::Player { player: bad }; },
                1 => { let SyncEventBody::CreatureAttackedAndUnblockedEvent { target, .. } = &mut bodies[1] else { panic!("fixture"); }; *target = SyncAttackEventTarget::Player { player: bad }; },
                2 => { let SyncEventBody::CreatureBecameBlockedEvent { attack_target, .. } = &mut bodies[2] else { panic!("fixture"); }; *attack_target = Some(SyncAttackEventTarget::Player { player: bad }); },
                3 => { let SyncEventBody::ManaAddedEvent { controller, .. } = &mut bodies[3] else { panic!("fixture"); }; *controller = bad; },
                4 => { let SyncEventBody::ManaUnitSpentEvent { player, .. } = &mut bodies[4] else { panic!("fixture"); }; *player = bad; },
                5 => { let SyncEventBody::ObjectBecameUnattachedEvent { previous_target, .. } = &mut bodies[5] else { panic!("fixture"); }; *previous_target = ironsmith::object::AttachmentTarget::Player(bad); },
                _ => { let SyncEventBody::CreatureAttackedEvent { declared_attackers, .. } = &mut bodies[0] else { panic!("fixture"); }; declared_attackers.as_mut().unwrap()[0].target = SyncDeclaredAttackTarget::Nothing { defending_player: Some(bad), was_planeswalker: true }; },
            }
            let forged = serde_json::from_slice(&serde_json::to_vec(&forged).unwrap()).unwrap();
            let error = host.apply_sync_checkpoint(forged).unwrap_err();
            assert!(error.contains("invalid player"), "capture case {case}: {error}");
            assert_eq!(serde_json::to_value(host.try_build_full_sync_checkpoint().unwrap()).unwrap(), unchanged,
                "failed capture import must preserve runtime and exported allocators");
        }
        let missing = serde_json::json!({"kind":"Nothing", "was_planeswalker":true});
        assert!(serde_json::from_value::<SyncDeclaredAttackTarget>(missing).is_err(), "captured defender cannot be silently defaulted");
        let expected = serde_json::to_value(&before.executable_state.as_ref().unwrap().event_bodies).unwrap();
        let mut projected_count = 0;
        let _ = project_sync_stack_history(&host.game.stack, &mut |h| { projected_count += 1; Ok(h) }).unwrap();
        assert_eq!(projected_count, 6, "five body captures plus the pushed stack source history");
        let checkpoint = serde_json::from_slice(&serde_json::to_vec(&before).unwrap()).unwrap();
        let mut guest = WasmGame::new(); guest.initialize_empty_match(vec!["Carol".into(), "Dan".into()], 20, 2);
        guest.apply_sync_checkpoint(checkpoint).unwrap();
        let restored = guest.try_build_full_sync_checkpoint().unwrap();
        assert_eq!(serde_json::to_value(&restored.executable_state.as_ref().unwrap().event_bodies).unwrap(), expected);
        let events = &guest.game.stack[0].effect_outcomes[&ironsmith::effect::EffectId(7)].events;
        let attack = events[0].downcast::<ironsmith::events::combat::CreatureAttackedEvent>().unwrap();
        assert_eq!(attack.total_attackers, 2);
        assert!(matches!(attack.declared_attackers.as_ref().unwrap()[0].target, ironsmith::combat_state::AttackTarget::Nothing { defending_player: Some(p), was_planeswalker: true } if p == alice));
        let mana = events[3].downcast::<ironsmith::events::mana::ManaAddedEvent>().unwrap();
        assert_eq!(mana.provenance, ironsmith::events::mana::ManaProductionProvenance::TappedSourceForMana);
        assert_eq!(mana.snapshot.as_ref().unwrap().card, guest.game.object(source).unwrap().card);
        let spent = events[4].downcast::<ironsmith::events::mana::ManaUnitSpentEvent>().unwrap();
        assert_eq!(spent.purpose, ironsmith::ability::ManaPaymentPurpose::ActivateManaAbility);
        assert_eq!(spent.source_snapshot.as_ref().unwrap().card, guest.game.object(source).unwrap().card);
        ironsmith::game_loop::resolve_stack_entry(&mut host.game).unwrap(); ironsmith::game_loop::resolve_stack_entry(&mut guest.game).unwrap();
        assert_eq!(host.game.player(alice).unwrap().life, 26); assert_eq!(guest.game.player(alice).unwrap().life, 26);
    }
    #[test]
    fn public_full_executable_checkpoint_preserves_history_event_body_matrix() {
        let _guard = crate::test_id_counter_guard();
        let (mut host, source, alice) = full_fixture();
        let snapshot = ironsmith::snapshot::ObjectSnapshot::from_object(host.game.object(source).unwrap(), &host.game);
        let provenance = host.game.provenance_graph_mut().alloc_root_event(ironsmith::events::EventKind::BeginningOfEndStep);
        let bodies: Vec<std::sync::Arc<dyn ironsmith::events::GameEventType>> = vec![
std::sync::Arc::new(ironsmith::events::combat::CreatureBlockedEvent { blocker: source, attacker: source, blocker_snapshot: Some(snapshot.clone()), attacker_snapshot: Some(snapshot.clone()) }),
std::sync::Arc::new(ironsmith::events::other::CardDiscardedEvent { player: alice, card: source, cause: Some(ironsmith::events::cause::EventCause::from_effect(source, alice)), snapshot: Some(snapshot.clone()), batch_cards: vec![source], batch_snapshots: vec![snapshot.clone()], batch_index: Some(2) }),
std::sync::Arc::new(ironsmith::events::other::CardRevealedEvent { player: alice, card: source, zone: Zone::Graveyard, source: Some(source), snapshot: Some(snapshot.clone()), reveal_context_amount: Some(17) }),
std::sync::Arc::new(ironsmith::events::other::PermanentPhasedOutEvent { permanent: source, controller: alice, snapshot: Some(snapshot.clone()) }),
std::sync::Arc::new(ironsmith::events::other::SpellCounteredEvent { spell: source, controller: alice, snapshot: Some(snapshot.clone()) }),
std::sync::Arc::new(ironsmith::events::permanents::DestroyEvent { permanent: source, source: Some(source), snapshot: Some(snapshot.clone()), final_zone: Some(Zone::Exile) }),
std::sync::Arc::new(ironsmith::events::permanents::SacrificeEvent { permanent: source, source: Some(source), snapshot: Some(snapshot.clone()), sacrificing_player: Some(alice) }),
std::sync::Arc::new(ironsmith::events::spells::SpellCastEvent { spell: source, caster: alice, from_zone: Zone::Graveyard, snapshot: Some(snapshot.clone()) }),
std::sync::Arc::new(ironsmith::events::zones::ObjectLeavesGameEvent { object: source, snapshot: snapshot.clone(), cause: ironsmith::events::cause::EventCause::from_effect(source, alice) }),
std::sync::Arc::new(ironsmith::events::zones::ZoneChangeEvent { objects: vec![source], result_objects: vec![source], from: Zone::Graveyard, to: Zone::Graveyard, cause: ironsmith::events::cause::EventCause::from_effect(source, alice), snapshot: Some(snapshot.clone()), snapshots: vec![snapshot.clone()], object_tags: std::collections::HashMap::from([("captured card".into(), vec![snapshot.clone()])]) })
        ];
        let mut entry = StackEntry::ability(source, alice, vec![ironsmith::Effect::gain_life(3)]);
        let mut outcome = ironsmith::effect::EffectOutcome::count(9);
        for body in bodies { outcome.events.push(ironsmith::triggers::TriggerEvent::from_boxed_with_provenance(body.clone_box(), provenance)); }
        entry.effect_outcomes.insert(ironsmith::effect::EffectId(7), outcome);
        let mut projected_count = 0;
        let projected = project_sync_stack_history(&[entry.clone()], &mut |mut history| {
            projected_count += 1; history.name = "Approved historical view".into(); Ok(history)
        }).unwrap();
        assert_eq!(projected_count, 14, "every scalar, vector and tagged history is projected");
        let event = &projected[0].effect_outcomes[&ironsmith::effect::EffectId(7)].events[9];
        let zone = event.downcast::<ironsmith::events::zones::ZoneChangeEvent>().unwrap();
        assert_eq!(zone.snapshot.as_ref().unwrap().name, "Approved historical view");
        assert_eq!(zone.snapshots[0].name, "Approved historical view");
        assert_eq!(zone.object_tags["captured card"][0].name, "Approved historical view");
        host.game.push_to_stack(entry);
        let before = host.try_build_full_sync_checkpoint().unwrap();
        let expected = serde_json::to_value(&before.executable_state.as_ref().unwrap().event_bodies).unwrap();
        assert_eq!(before.executable_state.as_ref().unwrap().event_bodies.len(), 10);
        let unchanged = serde_json::to_value(&before).unwrap();
        for index in 0..10 {
            let mut forged = before.clone();
            let bad = PlayerId::from_index(99);
            match &mut forged.executable_state.as_mut().unwrap().event_bodies[index] {
                SyncEventBody::CreatureBlockedEvent { attacker_snapshot, .. } => attacker_snapshot.as_mut().unwrap().owner = bad,
                SyncEventBody::CardDiscardedEvent { batch_snapshots, .. } => batch_snapshots[0].controller = bad,
                SyncEventBody::CardRevealedEvent { snapshot, .. }
                | SyncEventBody::PermanentPhasedOutEvent { snapshot, .. }
                | SyncEventBody::SpellCounteredEvent { snapshot, .. }
                | SyncEventBody::DestroyEvent { snapshot, .. }
                | SyncEventBody::SacrificeEvent { snapshot, .. }
                | SyncEventBody::SpellCastEvent { snapshot, .. } => snapshot.as_mut().unwrap().controller = bad,
                SyncEventBody::ObjectLeavesGameEvent { snapshot, .. } => snapshot.owner = bad,
                SyncEventBody::ZoneChangeEvent { object_tags, .. } => object_tags.values_mut().next().unwrap()[0].controller = bad,
                _ => panic!("unexpected history matrix body"),
            }
            let forged = serde_json::from_slice(&serde_json::to_vec(&forged).unwrap()).unwrap();
            let error = host.apply_sync_checkpoint(forged).unwrap_err();
            assert!(error.contains("invalid player"), "history body {index}: {error}");
            assert_eq!(serde_json::to_value(host.try_build_full_sync_checkpoint().unwrap()).unwrap(), unchanged,
                "rejected captured history actor must preserve the world and allocators");
        }
        let checkpoint = serde_json::from_slice(&serde_json::to_vec(&before).unwrap()).unwrap();
        let mut guest = WasmGame::new();
        guest.initialize_empty_match(vec!["Carol".into(), "Dan".into()], 20, 2);
        guest.apply_sync_checkpoint(checkpoint).unwrap();
        let restored = guest.try_build_full_sync_checkpoint().unwrap();
        assert_eq!(serde_json::to_value(&restored.executable_state.as_ref().unwrap().event_bodies).unwrap(), expected);
        let events = &guest.game.stack[0].effect_outcomes[&ironsmith::effect::EffectId(7)].events;
        let zone = events[9].downcast::<ironsmith::events::zones::ZoneChangeEvent>().unwrap();
        let peer_face = guest.game.object(source).unwrap().card;
        assert_eq!(zone.snapshot.as_ref().unwrap().card, peer_face);
        assert_eq!(zone.snapshots[0].card, peer_face);
        assert_eq!(zone.object_tags["captured card"][0].card, peer_face);
        assert_eq!(events[0].downcast::<ironsmith::events::combat::CreatureBlockedEvent>().unwrap().blocker_snapshot.as_ref().unwrap().card, peer_face);
        ironsmith::game_loop::resolve_stack_entry(&mut host.game).unwrap();
        ironsmith::game_loop::resolve_stack_entry(&mut guest.game).unwrap();
        assert_eq!(host.game.player(alice).unwrap().life, 26);
        assert_eq!(guest.game.player(alice).unwrap().life, 26);
    }
    #[test]
    fn public_full_executable_checkpoint_rejects_invalid_event_history_actor() {
        let _guard = crate::test_id_counter_guard();
        let (mut host, source, alice) = full_fixture();
        let mut snapshot = ironsmith::snapshot::ObjectSnapshot::from_object(host.game.object(source).unwrap(), &host.game);
        snapshot.controller = PlayerId::from_index(99);
        let provenance = host.game.provenance_graph_mut().alloc_root_event(ironsmith::events::EventKind::BeginningOfEndStep);
        let mut entry = StackEntry::ability(source, alice, vec![ironsmith::Effect::gain_life(3)]);
        entry.triggering_event = Some(ironsmith::triggers::TriggerEvent::new_with_provenance(
            ironsmith::events::zones::ZoneChangeEvent { objects: vec![source], result_objects: vec![source], from: Zone::Battlefield, to: Zone::Graveyard,
                cause: ironsmith::events::cause::EventCause::from_effect(source, alice), snapshot: None, snapshots: vec![],
                object_tags: std::collections::HashMap::from([("captured card".into(), vec![snapshot])]) }, provenance));
        host.game.push_to_stack(entry);
        assert!(host.try_build_full_sync_checkpoint().unwrap_err().contains("invalid player"));
    }
    #[test]
    fn public_full_executable_checkpoint_preserves_scalar_event_body_matrix() {
        let _guard = crate::test_id_counter_guard();
        let (mut host, source, alice) = full_fixture();
        let provenance = host.game.provenance_graph_mut().alloc_root_event(ironsmith::events::EventKind::BeginningOfEndStep);
        let bodies: Vec<std::sync::Arc<dyn ironsmith::events::GameEventType>> = vec![
                    std::sync::Arc::new(ironsmith::events::cards::DiscardEvent { card: source, player: alice, destination: Zone::Exile, cause: ironsmith::events::cause::EventCause::from_effect(source, alice), requires_type_verification: true, madness_applied: true }),
                    std::sync::Arc::new(ironsmith::events::cards::DrawEvent { player: alice, count: 3, is_first_this_turn: true, first_of_instruction: true, first_of_draw_step: true }),
                    std::sync::Arc::new(ironsmith::events::counters::MoveCountersEvent { from: source, to: source, counter_type: Some(ironsmith::CounterType::Named("Checkpoint event counter".into())), count: Some(3) }),
                    std::sync::Arc::new(ironsmith::events::counters::PutCountersEvent { target: ironsmith::game_state::Target::Player(alice), counter_type: ironsmith::CounterType::Named("Checkpoint event counter".into()), count: 3, maximum_count: Some(3), cause: ironsmith::events::cause::EventCause::from_effect(source, alice) }),
                    std::sync::Arc::new(ironsmith::events::counters::RemoveCountersEvent { target: source, counter_type: ironsmith::CounterType::Named("Checkpoint event counter".into()), count: 3 }),
                    std::sync::Arc::new(ironsmith::events::other::BecameMonstrousEvent { creature: source, controller: alice, n: 3 }),
                    std::sync::Arc::new(ironsmith::events::other::CardsDrawnEvent { player: alice, cards: vec![source], is_first_this_turn: true, is_during_players_draw_step: true, cards_previously_drawn_this_draw_step: 3 }),
                    std::sync::Arc::new(ironsmith::events::other::ChapterAbilityResolvedEvent { saga: source, controller: alice, final_chapter: true }),
                    std::sync::Arc::new(ironsmith::events::other::CoinFlippedEvent { player: alice, source: source, face: ironsmith_core::CoinFace::Heads, call: Some(ironsmith_core::CoinFace::Tails), winner: Some(alice), loser: Some(alice) }),
                    std::sync::Arc::new(ironsmith::events::other::ControlChangedEvent { permanent: source, previous_controller: alice, new_controller: alice }),
                    std::sync::Arc::new(ironsmith::events::other::ConvertedEvent { permanent: source }),
                    std::sync::Arc::new(ironsmith::events::other::CounterPlacedEvent { permanent: source, counter_type: ironsmith::CounterType::Named("Checkpoint event counter".into()), amount: 3, previous_count: Some(3) }),
                    std::sync::Arc::new(ironsmith::events::other::DayNightChangedEvent { is_daytime: true }),
                    std::sync::Arc::new(ironsmith::events::other::DieRolledEvent { player: alice, source: source, natural_result: 3, result: 3, sides: 3, is_planar: true, is_attraction_visit: true }),
                    std::sync::Arc::new(ironsmith::events::other::GiftGivenEvent { player: alice, recipient: alice, source: source }),
                    std::sync::Arc::new(ironsmith::events::other::LandPlayedEvent { land: source, player: alice, from_zone: Zone::Exile }),
                    std::sync::Arc::new(ironsmith::events::other::MutatedEvent { permanent: source, controller: alice }),
                    std::sync::Arc::new(ironsmith::events::other::PermanentTappedEvent { permanent: source }),
                    std::sync::Arc::new(ironsmith::events::other::PermanentUntappedEvent { permanent: source }),
                    std::sync::Arc::new(ironsmith::events::other::PlayerLosesGameEvent { player: alice }),
                    std::sync::Arc::new(ironsmith::events::other::SearchLibraryEvent { player: alice, library_owner: Some(alice) }),
                    std::sync::Arc::new(ironsmith::events::other::ShuffleLibraryEvent { player: alice, cause: ironsmith::events::cause::EventCause::from_effect(source, alice) }),
                    std::sync::Arc::new(ironsmith::events::other::StateTriggerEvent { source: source }),
                    std::sync::Arc::new(ironsmith::events::other::TransformedEvent { permanent: source }),
                    std::sync::Arc::new(ironsmith::events::other::TurnedFaceUpEvent { permanent: source, player: alice }),
                    std::sync::Arc::new(ironsmith::events::permanents::TapEvent { permanent: source }),
                    std::sync::Arc::new(ironsmith::events::permanents::UntapEvent { permanent: source }),
                    std::sync::Arc::new(ironsmith::events::phase::BeginningOfCleanupStepEvent { player: alice }),
                    std::sync::Arc::new(ironsmith::events::phase::BeginningOfDrawStepEvent { player: alice }),
                    std::sync::Arc::new(ironsmith::events::phase::BeginningOfPrecombatMainPhaseEvent { player: alice }),
                    std::sync::Arc::new(ironsmith::events::phase::BeginningOfPostcombatMainPhaseEvent { player: alice, main_phase_ordinal: Some(3) }),
                    std::sync::Arc::new(ironsmith::events::phase::EndOfCombatEvent),
                    std::sync::Arc::new(ironsmith::events::phase::PermanentsUntapStepEvent { player: alice }),
                    std::sync::Arc::new(ironsmith::events::spells::BecomesTargetedEvent { target: ironsmith::game_state::Target::Player(alice), source: source, source_controller: alice, by_ability: true, stack_ability: Some(source) }),
                    std::sync::Arc::new(ironsmith::events::spells::SpellCopiedEvent { spell: source, copier: alice }),
        ];
        let mut entry = StackEntry::ability(source, alice, vec![ironsmith::Effect::gain_life(3)]);
        let mut outcome = ironsmith::effect::EffectOutcome::count(9);
        for body in bodies { outcome.events.push(ironsmith::triggers::TriggerEvent::from_boxed_with_provenance(body.clone_box(), provenance)); }
        entry.effect_outcomes.insert(ironsmith::effect::EffectId(7), outcome);
        host.game.push_to_stack(entry);
        let before = host.try_build_full_sync_checkpoint().unwrap();
        assert_eq!(before.executable_state.as_ref().unwrap().event_bodies.len(), 35);
        let expected = serde_json::to_value(&before.executable_state.as_ref().unwrap().event_bodies).unwrap();
        let checkpoint = serde_json::from_slice(&serde_json::to_vec(&before).unwrap()).unwrap();
        let mut guest = WasmGame::new();
        guest.initialize_empty_match(vec!["Carol".into(), "Dan".into()], 20, 2);
        guest.apply_sync_checkpoint(checkpoint).unwrap();
        let restored = guest.try_build_full_sync_checkpoint().unwrap();
        assert_eq!(serde_json::to_value(&restored.executable_state.as_ref().unwrap().event_bodies).unwrap(), expected,
            "all scalar body fields, including draw flags, counter limits, causes, search ownership and random outcomes must round-trip");
        ironsmith::game_loop::resolve_stack_entry(&mut host.game).unwrap();
        ironsmith::game_loop::resolve_stack_entry(&mut guest.game).unwrap();
        assert_eq!(host.game.player(alice).unwrap().life, 26);
        assert_eq!(guest.game.player(alice).unwrap().life, 26);
    }
    #[test]
    fn public_full_executable_checkpoint_restores_without_receiver_catalog_definition() {
        let _guard = crate::test_id_counter_guard();
        let (host, source, alice) = full_fixture();
        let checkpoint = host.build_sync_checkpoint();
        assert!(checkpoint.executable_state.is_some());
        let wire = serde_json::to_string(&checkpoint).unwrap();
        let checkpoint: SyncCheckpoint = serde_json::from_str(&wire).unwrap();
        let mut guest = WasmGame::new();
        guest.initialize_empty_match(vec!["Carol".into(), "Dan".into()], 20, 2);
        assert!(guest.registry.get("Full checkpoint typed owner").is_none());
        guest.apply_sync_checkpoint(checkpoint).unwrap();
        let object = guest.game.object(source).unwrap();
        let definition = guest.game.retained_card_definition(object.card.unwrap()).unwrap();
        let program = definition.spell_effect.as_ref().unwrap().all_effects_owned();
        let mut ctx = ironsmith::effects::EffectContext::new_default(source, alice);
        let out = ironsmith::effects::execute_effect(&mut guest.game, &program[0], &mut ctx).unwrap();
        assert_eq!(out.as_count(), Some(4));
        assert_eq!(guest.game.player(alice).unwrap().life, 24);
        assert_eq!(serde_json::to_value(&guest.game.provenance_graph().retained_state().nodes[..2]).unwrap(), serde_json::to_value(&host.game.provenance_graph().retained_state().nodes[..2]).unwrap());
    }
    #[test]
    fn public_full_executable_checkpoint_preserves_registered_mana_grant_and_removal() {
        let _guard = crate::test_id_counter_guard();
        let (mut host, source, alice) = full_fixture();
        let effect_id = host.game.effect_store.continuous_effects.add_effect(
            ironsmith::continuous::ContinuousEffect::from_resolution(
                source, alice, vec![source],
                ironsmith::continuous::Modification::AddAbilityGeneric(ironsmith::Ability::mana(
                    ironsmith::TotalCost::free(), vec![ironsmith::mana::ManaSymbol::Green]))));
        host.game.refresh_continuous_state().unwrap();
        assert!(host.game.current_abilities(source).unwrap().iter().any(|ability| ability.is_mana_ability()));
        let original = host.game.effect_store.continuous_effects.registered_state();
        // Exercise the actual owning exporter, JSON wire, fresh catalog and
        // importer: a manager-only codec test cannot catch publication gaps.
        let checkpoint = host.try_build_full_sync_checkpoint().expect("executable grant exports");
        let checkpoint: SyncCheckpoint = serde_json::from_slice(&serde_json::to_vec(&checkpoint).unwrap()).unwrap();
        let mut guest = WasmGame::new();
        guest.initialize_empty_match(vec!["Carol".into(), "Dan".into()], 20, 2);
        assert!(guest.registry.get("Full checkpoint typed owner").is_none());
        guest.apply_sync_checkpoint(checkpoint).expect("executable grant imports");
        for peer in [&mut host, &mut guest] {
            let ability = peer.game.current_abilities(source).unwrap().into_iter()
                .find(|ability| ability.is_mana_ability()).expect("registered mana grant survives");
            let ironsmith::ability::AbilityKind::Activated(activated) = ability.kind else { panic!("mana grant is activated"); };
            let mut context = ironsmith::effects::EffectContext::new_default(source, alice);
            for effect in activated.effects.all_effects_owned() {
                ironsmith::effects::execute_effect(&mut peer.game, &effect, &mut context).unwrap();
            }
            peer.game.effect_store.continuous_effects.remove_effect(effect_id);
            peer.game.refresh_continuous_state().unwrap();
            assert!(!peer.game.current_abilities(source).unwrap().iter().any(|ability| ability.is_mana_ability()),
                "removing the granting effect removes its imported executable ability");
        }
        assert_eq!(host.game.player(alice).unwrap().mana_pool, guest.game.player(alice).unwrap().mana_pool,
            "restored ability executes the same mana production as the source");
        assert_eq!(original.effects.len(), 1);
    }
    #[test]
    fn public_perspective_checkpoint_preserves_registered_mana_grant() {
        let _guard = crate::test_id_counter_guard();
        let (mut host, source, alice) = full_fixture();
        host.game.effect_store.continuous_effects.add_effect(
            ironsmith::continuous::ContinuousEffect::from_resolution(
                source, alice, vec![source],
                ironsmith::continuous::Modification::AddAbilityGeneric(ironsmith::Ability::mana(
                    ironsmith::TotalCost::free(), vec![ironsmith::mana::ManaSymbol::Green]))));
        host.game.refresh_continuous_state().unwrap();
        assert!(host.game.current_abilities(source).unwrap().iter().any(|ability| ability.is_mana_ability()));
        let checkpoint = host.try_build_redacted_executable_checkpoint(alice).expect("public grant exports");
        let checkpoint: SyncCheckpoint = serde_json::from_slice(&serde_json::to_vec(&checkpoint).unwrap()).unwrap();
        let mut guest = WasmGame::new();
        guest.initialize_empty_match(vec!["Carol".into(), "Dan".into()], 20, 2);
        // Metadata reconstruction has the printed source, but the granted
        // program exists only in the registered runtime effect.
        guest.registry.register(host.registry.get("Full checkpoint typed owner").unwrap().clone());
        guest.apply_foreign_sync_checkpoint_for_perspective(checkpoint, alice.0).expect("public grant imports");
        assert!(guest.game.current_abilities(source).unwrap().iter().any(|ability| ability.is_mana_ability()),
            "perspective import cannot silently lose the registered executable grant");
        for peer in [&mut host, &mut guest] {
            let index = peer.game.current_abilities(source).unwrap().iter().position(|ability| ability.is_mana_ability()).unwrap();
            let before = peer.game.player(alice).unwrap().mana_pool.green;
            let mut dm = ironsmith::decision::AutoPassDecisionMaker;
            ironsmith::special_actions::perform_activate_mana_ability(&mut peer.game, alice, source, index, &mut dm).unwrap();
            assert_eq!(peer.game.player(alice).unwrap().mana_pool.green, before + 1,
                "retained mana output and source-tap cost execute through the real activation path");
            assert!(peer.game.is_tapped(source));
        }

    }
    fn perspective_private_fixture(compiled: bool) -> (WasmGame, ObjectId, ObjectId, PlayerId) {
        let (mut host, source, alice) = full_fixture();
        let hidden = add_perspective_private_card(&mut host, compiled);
        (host, source, hidden, alice)
    }
    fn perspective_origin_fixture(compiled: bool) -> (WasmGame, ObjectId, ObjectId, PlayerId) {
        let (mut host, _, alice) = full_fixture();
        let definition = ironsmith::cards::builders::CardDefinitionBuilder::new(CardId::new(), "Public origin owner")
            .card_types(vec![CardType::Artifact])
            .with_ability(ironsmith::Ability::mana(ironsmith::TotalCost::free(), vec![ironsmith::mana::ManaSymbol::Green]))
            .build();
        host.registry.register(definition.clone());
        let source = host.game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let hidden = add_perspective_private_card(&mut host, compiled);
        (host, source, hidden, alice)
    }
    fn add_perspective_private_card(host: &mut WasmGame, compiled: bool) -> ObjectId {
        let bob = PlayerId::from_index(1);
        let private = if compiled {
            ironsmith_registry_test::compile_to_runtime_definition("Private perspective program marker",
                "Type: Artifact\nPay 1 life: You gain 7777 life.", false).unwrap()
        } else {
            ironsmith::cards::builders::CardDefinitionBuilder::new(CardId::new(), "Private perspective program marker")
                .card_types(vec![CardType::Artifact]).with_ability(ironsmith::Ability::activated(
                    ironsmith::TotalCost::free(), vec![ironsmith::Effect::gain_life(7777)])).build()
        };
        host.registry.register(private.clone());
        let hidden = host.game.create_object_from_definition(&private, bob, Zone::Library);
        host.game.set_hidden_card_info(hidden, HiddenCardInfo { owner: bob, zone: Zone::Library, slot: 0,
            commitment: "private-perspective-commitment".into(), origin_slot: None, origin_commitment: None,
            public_slot: None, public_commitment: None });
        hidden
    }
    #[test]
    fn public_perspective_checkpoint_rejects_private_continuous_filter_capture() {
        let _guard = crate::test_id_counter_guard();
        for compiled in [false, true] {
            let (mut host, source, hidden, alice) = perspective_private_fixture(compiled);
            let mut filter = ironsmith::target::ObjectFilter::default();
            filter.name = Some(host.game.object(hidden).unwrap().name.to_string());
            let mut effect = ironsmith::continuous::ContinuousEffect::from_resolution(source, alice, vec![],
                ironsmith::continuous::Modification::ModifyPowerToughness { power: 1, toughness: 1 });
            effect.applies_to = ironsmith::continuous::EffectTarget::Filter(filter);
            host.game.effect_store.continuous_effects.add_effect(effect);
            let before = serde_json::to_value(host.try_build_full_sync_checkpoint().unwrap()).unwrap();
            let result = host.try_build_redacted_executable_checkpoint(alice);
            if let Ok(checkpoint) = &result {
                assert!(!serde_json::to_string(checkpoint).unwrap().contains("Private perspective program marker"),
                    "scalar filter captured from an opaque library root must not publish its private name");
            }
            assert!(result.is_err(), "unapproved captured filter cannot be silently dropped or published");
            assert_eq!(serde_json::to_value(host.try_build_full_sync_checkpoint().unwrap()).unwrap(), before);
        }
    }
    #[test]
    fn public_perspective_checkpoint_preserves_disclosed_origin_face() {
        let _guard = crate::test_id_counter_guard();
        let (mut host, source, _, alice) = perspective_origin_fixture(false);
        let definition = ironsmith::cards::builders::CardDefinitionBuilder::new(CardId::new(), "Public origin recipient")
            .card_types(vec![CardType::Creature])
            .power_toughness(ironsmith::card::PowerToughness::fixed(2, 3)).build();
        host.registry.register(definition.clone());
        let recipient = host.game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let mut effect = ironsmith::continuous::ContinuousEffect::from_resolution(source, alice, vec![recipient],
            ironsmith::continuous::Modification::ModifyPowerToughness { power: 1, toughness: 1 });
        effect.originating_ability = Some(Box::new(ironsmith::continuous::ContinuousAbilityOrigin {
            host: source, ability: ironsmith::continuous::AbilityOrigin::Printed(0),
            printed_face: host.game.object(source).unwrap().card, branch: 0,
        }));
        host.game.effect_store.continuous_effects.add_effect(effect);
        host.game.refresh_continuous_state().unwrap();
        let checkpoint = host.try_build_redacted_executable_checkpoint(alice).unwrap();
        let checkpoint: SyncCheckpoint = serde_json::from_slice(&serde_json::to_vec(&checkpoint).unwrap()).unwrap();
        let mut guest = WasmGame::new();
        guest.initialize_empty_match(vec!["Carol".into(), "Dan".into()], 20, 2);
        guest.registry.register(host.registry.get("Full checkpoint typed owner").unwrap().clone());
        guest.registry.register(host.registry.get("Public origin owner").unwrap().clone());
        guest.registry.register(definition);
        guest.apply_foreign_sync_checkpoint_for_perspective(checkpoint, alice.0).unwrap();
        for peer in [&host, &guest] {
            let chars = peer.game.calculated_characteristics(recipient).unwrap();
            assert_eq!((chars.power, chars.toughness), (Some(3), Some(4)));
            let state = peer.game.effect_store.continuous_effects.registered_state();
            let origin = state.effects[0].originating_ability.as_ref().unwrap();
            assert_eq!(origin.printed_face, peer.game.object(source).unwrap().card,
                "origin binds to recipient allocation of the disclosed printed face");
            assert_eq!(origin.host, source);
        }
    }
    #[test]
    fn public_perspective_checkpoint_rejects_forged_private_origin_face_atomically() {
        let _guard = crate::test_id_counter_guard();
        for (compiled, nested) in [(false, false), (false, true), (true, false), (true, true)] {
            let (mut host, source, hidden, alice) = perspective_origin_fixture(compiled);
            let mut effect = ironsmith::continuous::ContinuousEffect::from_resolution(source, alice, vec![source],
                ironsmith::continuous::Modification::ModifyPowerToughness { power: 1, toughness: 1 });
            effect.originating_ability = Some(Box::new(ironsmith::continuous::ContinuousAbilityOrigin {
                host: source, ability: ironsmith::continuous::AbilityOrigin::Printed(0),
                printed_face: host.game.object(source).unwrap().card, branch: 0,
            }));
            host.game.effect_store.continuous_effects.add_effect(effect);
            let mut checkpoint = host.try_build_redacted_executable_checkpoint(alice).unwrap();
            let full = host.try_build_full_sync_checkpoint().unwrap().executable_state.unwrap();
            let private_face = full.objects.iter().find(|object| object.id == hidden).unwrap().card.unwrap();
            let state = checkpoint.executable_state.as_mut().unwrap();
            state.graph_card_count = full.graph_card_count;
            state.definitions = full.definitions;
            state.occurrences = full.occurrences;
            let origin = state.continuous.effects[0].originating_ability.as_mut().unwrap();
            if nested {
                origin.ability = ironsmith::continuous::AbilityOrigin::Level {
                    printed_face: Some(private_face), parent: Box::new(ironsmith::continuous::AbilityOrigin::Printed(0)),
                    tier: 0, slot: 0,
                };
            } else { origin.printed_face = Some(private_face); }
            let checkpoint: SyncCheckpoint = serde_json::from_slice(&serde_json::to_vec(&checkpoint).unwrap()).unwrap();
            assert!(serde_json::to_string(&checkpoint).unwrap().contains("7777"));
            let before = serde_json::to_value(host.try_build_full_sync_checkpoint().unwrap()).unwrap();
            let error = host.with_runtime_transaction(|candidate|
                candidate.apply_foreign_sync_checkpoint_for_perspective(checkpoint, alice.0)).unwrap_err();
            assert!(error.contains("continuous origin face"), "{error}");
            assert_eq!(serde_json::to_value(host.try_build_full_sync_checkpoint().unwrap()).unwrap(), before);
        }
    }
    #[test]
    fn public_perspective_checkpoint_preserves_registered_origin_metadata() {
        let _guard = crate::test_id_counter_guard();
        for (temporary, borrowed) in [(false, false), (false, true), (true, false), (true, true)] {
            let (mut host, source, _, alice) = perspective_private_fixture(false);
            let definition = ironsmith::cards::builders::CardDefinitionBuilder::new(CardId::new(), "Public registration recipient")
                .card_types(vec![CardType::Creature])
                .power_toughness(ironsmith::card::PowerToughness::fixed(2, 3)).build();
            host.registry.register(definition.clone());
            let recipient = host.game.create_object_from_definition(&definition, alice, Zone::Battlefield);
            let registration_source = if borrowed {
                let provider = host.registry.get("Full checkpoint typed owner").unwrap().clone();
                host.game.create_object_from_definition(&provider, alice, Zone::Battlefield)
            } else { source };
            let origin = if temporary {
                host.game.grant_temporary_static_ability_payload_to_object_until_end_of_turn(
                    registration_source, ironsmith::static_abilities::StaticAbilityId::Flying, None);
                ironsmith::continuous::AbilityOrigin::Temporary(host.game.object(registration_source).unwrap()
                    .temporary_static_ability_grants.origin(0).unwrap().clone())
            } else {
                host.game.add_counters(registration_source, ironsmith::CounterType::Flying, 1).unwrap();
                ironsmith::continuous::AbilityOrigin::Counter {
                    occurrence: host.game.object(registration_source).unwrap().counters.ability_state().origins[0].clone(), slot: 0,
                }
            };
            let mut effect = ironsmith::continuous::ContinuousEffect::from_resolution(source, alice, vec![recipient],
                ironsmith::continuous::Modification::ModifyPowerToughness { power: 1, toughness: 1 });
            let origin = if borrowed { ironsmith::continuous::AbilityOrigin::Borrowed {
                effect: (&effect).into(), source: registration_source, origin: Box::new(origin),
            } } else { origin };
            effect.originating_ability = Some(Box::new(ironsmith::continuous::ContinuousAbilityOrigin {
                host: source, ability: origin, printed_face: None, branch: 0,
            }));
            host.game.effect_store.continuous_effects.add_effect(effect);
            host.game.refresh_continuous_state().unwrap();
            let checkpoint = host.try_build_redacted_executable_checkpoint(alice).unwrap();
            let checkpoint: SyncCheckpoint = serde_json::from_slice(&serde_json::to_vec(&checkpoint).unwrap()).unwrap();
            let mut guest = WasmGame::new();
            guest.initialize_empty_match(vec!["Carol".into(), "Dan".into()], 20, 2);
            guest.registry.register(host.registry.get("Full checkpoint typed owner").unwrap().clone());
            guest.registry.register(definition);
            guest.apply_foreign_sync_checkpoint_for_perspective(checkpoint, alice.0).unwrap();
            for peer in [&host, &guest] {
                let chars = peer.game.calculated_characteristics(recipient).unwrap();
                assert_eq!((chars.power, chars.toughness), (Some(3), Some(4)));
                let state = peer.game.effect_store.continuous_effects.registered_state();
                let value = &state.effects[0].originating_ability.as_ref().unwrap().ability;
                let value = if borrowed {
                    let ironsmith::continuous::AbilityOrigin::Borrowed { source: provider, origin, .. } = value else { panic!("borrowed fixture"); };
                    assert_eq!(*provider, registration_source);
                    origin.as_ref()
                } else { value };
                let object = peer.game.object(registration_source).unwrap();
                match value {
                    ironsmith::continuous::AbilityOrigin::Counter { occurrence, slot } =>
                        assert!(object.counters.contains_ability_origin(occurrence, *slot)),
                    ironsmith::continuous::AbilityOrigin::Temporary(registration) =>
                        assert!(object.temporary_static_ability_grants.contains_origin(registration)),
                    _ => panic!("fixture origin"),
                }
            }
        }
    }
    #[test]
    fn public_perspective_checkpoint_rejects_forged_origin_metadata_atomically() {
        let _guard = crate::test_id_counter_guard();
        for case in 0..5 {
            let (mut host, source, hidden, alice) = perspective_private_fixture(false);
            host.game.add_counters(source, ironsmith::CounterType::Flying, 1).unwrap();
            let registration = host.game.object(source).unwrap().counters.ability_state().origins[0].clone();
            let mut effect = ironsmith::continuous::ContinuousEffect::from_resolution(source, alice, vec![source],
                ironsmith::continuous::Modification::ModifyPower(1));
            effect.originating_ability = Some(Box::new(ironsmith::continuous::ContinuousAbilityOrigin {
                host: source, ability: ironsmith::continuous::AbilityOrigin::Counter { occurrence: registration, slot: 0 },
                printed_face: None, branch: 0,
            }));
            host.game.effect_store.continuous_effects.add_effect(effect);
            let mut checkpoint = host.try_build_redacted_executable_checkpoint(alice).unwrap();
            let origin = checkpoint.executable_state.as_mut().unwrap().continuous.effects[0].originating_ability.as_mut().unwrap();
            match case {
                0 => {
                    let ironsmith::continuous::AbilityOrigin::Counter { occurrence, .. } = &mut origin.ability else { panic!("fixture"); };
                    occurrence.counter_type = ironsmith::CounterType::Named("Private perspective program marker".into());
                },
                1 => {
                    let ironsmith::continuous::AbilityOrigin::Counter { occurrence, .. } = &mut origin.ability else { panic!("fixture"); };
                    occurrence.serial = vec![u32::MAX];
                },
                2 => {
                    let ironsmith::continuous::AbilityOrigin::Counter { slot, .. } = &mut origin.ability else { panic!("fixture"); };
                    *slot = usize::MAX;
                },
                3 => origin.ability = ironsmith::continuous::AbilityOrigin::Printed(usize::MAX),
                _ => origin.host = hidden,
            }
            let checkpoint = serde_json::from_slice(&serde_json::to_vec(&checkpoint).unwrap()).unwrap();
            let before = serde_json::to_value(host.try_build_full_sync_checkpoint().unwrap()).unwrap();
            let error = host.with_runtime_transaction(|candidate|
                candidate.apply_foreign_sync_checkpoint_for_perspective(checkpoint, alice.0)).unwrap_err();
            assert!(error.contains(if case == 4 { "disclosed identity" } else { "origin requires association approval" }), "{error}");
            assert_eq!(serde_json::to_value(host.try_build_full_sync_checkpoint().unwrap()).unwrap(), before);
        }
    }
    #[test]
    fn public_perspective_checkpoint_rejects_unapproved_origin_metadata() {
        let _guard = crate::test_id_counter_guard();
        let mut accepted = vec![];
        for compiled in [false, true] {
            for case in 0..4 {
                let (mut host, source, hidden, alice) = perspective_private_fixture(compiled);
                let mut effect = ironsmith::continuous::ContinuousEffect::from_resolution(source, alice, vec![source],
                    ironsmith::continuous::Modification::ModifyPowerToughness { power: 1, toughness: 1 });
                let leaf = match case {
                    0 => ironsmith::continuous::AbilityOrigin::Counter {
                        occurrence: ironsmith::object::CounterAbilityOrigin {
                            counter_type: ironsmith::CounterType::Named("Private perspective program marker".into()), serial: vec![1],
                        }, slot: 0,
                    },
                    1 => {
                        let parent = ironsmith::continuous::ContinuousEffect::from_resolution(hidden, alice, vec![],
                            ironsmith::continuous::Modification::ModifyPower(1));
                        ironsmith::continuous::AbilityOrigin::Effect { effect: (&parent).into(), slot: 0 }
                    },
                    2 => ironsmith::continuous::AbilityOrigin::Borrowed {
                        effect: (&effect).into(), source: hidden, origin: Box::new(ironsmith::continuous::AbilityOrigin::Printed(0)),
                    },
                    _ => {
                        let mut grants = ironsmith::object::TemporaryStaticAbilityGrants::new(hidden);
                        grants.push(ironsmith::object::TemporaryStaticAbilityGrant {
                            ability: ironsmith::static_abilities::StaticAbilityId::Flying,
                            ability_payload: None, expires_end_of_turn: 4,
                        });
                        ironsmith::continuous::AbilityOrigin::Temporary(grants.origin(0).unwrap().clone())
                    },
                };
                effect.originating_ability = Some(Box::new(ironsmith::continuous::ContinuousAbilityOrigin {
                    host: source, ability: leaf, printed_face: host.game.object(source).unwrap().card, branch: 0,
                }));
                host.game.effect_store.continuous_effects.add_effect(effect);
                let before = serde_json::to_value(host.try_build_full_sync_checkpoint().unwrap()).unwrap();
                if let Ok(checkpoint) = host.try_build_redacted_executable_checkpoint(alice) {
                    accepted.push((compiled, case, serde_json::to_string(&checkpoint).unwrap().contains("Private perspective program marker")));
                }
                assert_eq!(serde_json::to_value(host.try_build_full_sync_checkpoint().unwrap()).unwrap(), before);
            }
        }
        assert!(accepted.is_empty(), "unapproved origin metadata admitted (compiled, case, name leaked): {accepted:?}");
    }
    #[test]
    fn public_perspective_checkpoint_rejects_private_origin_face_capture() {
        let _guard = crate::test_id_counter_guard();
        let mut accepted = vec![];
        for (compiled, nested) in [(false, 0), (false, 1), (false, 2), (true, 0), (true, 1), (true, 2)] {
            let (mut host, source, hidden, alice) = perspective_origin_fixture(compiled);
            let mut effect = ironsmith::continuous::ContinuousEffect::from_resolution(source, alice, vec![source],
                ironsmith::continuous::Modification::ModifyPowerToughness { power: 1, toughness: 1 });
            effect.originating_ability = Some(Box::new(ironsmith::continuous::ContinuousAbilityOrigin {
                host: source, ability: ironsmith::continuous::AbilityOrigin::Printed(0),
                printed_face: host.game.object(hidden).unwrap().card, branch: 0,
            }));
            if nested == 1 {
                effect.originating_ability = Some(Box::new(ironsmith::continuous::ContinuousAbilityOrigin {
                    host: source, ability: ironsmith::continuous::AbilityOrigin::Level {
                        printed_face: host.game.object(hidden).unwrap().card,
                        parent: Box::new(ironsmith::continuous::AbilityOrigin::Printed(0)), tier: 0, slot: 0,
                    }, printed_face: host.game.object(source).unwrap().card, branch: 0,
                }));
            } else if nested == 2 {
                let parent: ironsmith::continuous::AbilityEffectOrigin = (&effect).into();
                effect.originating_ability = Some(Box::new(ironsmith::continuous::ContinuousAbilityOrigin {
                    host: source, ability: ironsmith::continuous::AbilityOrigin::Effect { effect: parent, slot: 0 },
                    printed_face: host.game.object(source).unwrap().card, branch: 0,
                }));
            }
            host.game.effect_store.continuous_effects.add_effect(effect);
            let before = serde_json::to_value(host.try_build_full_sync_checkpoint().unwrap()).unwrap();
            if let Ok(checkpoint) = host.try_build_redacted_executable_checkpoint(alice) {
                let wire = serde_json::to_string(&checkpoint).unwrap();
                accepted.push((compiled, nested, wire.contains("Private perspective program marker"), wire.contains("7777")));
            }
            assert_eq!(serde_json::to_value(host.try_build_full_sync_checkpoint().unwrap()).unwrap(), before);
        }
        assert!(accepted.is_empty(), "origin admitted hidden printed face (compiled, name leaked, programme leaked): {accepted:?}");
    }
    #[test]
    fn public_perspective_checkpoint_rejects_private_continuous_metadata_captures() {
        let _guard = crate::test_id_counter_guard();
        let mut accepted = vec![];
        for compiled in [false, true] {
            for case in 0..4 {
                let (mut host, source, hidden, alice) = perspective_private_fixture(compiled);
                let marker = host.game.object(hidden).unwrap().name.to_string();
                let mut captured = ironsmith::target::ObjectFilter::default();
                captured.name = Some(marker.clone());
                let mut effect = ironsmith::continuous::ContinuousEffect::from_resolution(source, alice, vec![source],
                    ironsmith::continuous::Modification::ModifyPowerToughness { power: 1, toughness: 1 });
                match case {
                    0 => effect.condition = Some(ironsmith::ConditionExpr::Not(Box::new(
                        ironsmith::ConditionExpr::YouControl(captured)))),
                    1 => effect.modification = ironsmith::continuous::Modification::SetName(marker.clone()),
                    2 => effect.modification = ironsmith::continuous::Modification::SetPower {
                        value: ironsmith::effect::Value::Add(Box::new(ironsmith::effect::Value::Fixed(1)),
                            Box::new(ironsmith::effect::Value::Count(captured))),
                        sublayer: ironsmith::continuous::PtSublayer::Setting,
                    },
                    _ => effect.duration = ironsmith::effect::Until::ForAsLongAs(
                        ironsmith_core::effect::ContinuousDurationPredicate::ObjectHasCounter {
                            object: ironsmith_core::effect::ContinuousDurationObject::Source,
                            counter_type: ironsmith::CounterType::Named(marker.into()), minimum: 1,
                        }),
                }
                host.game.effect_store.continuous_effects.add_effect(effect);
                let before = serde_json::to_value(host.try_build_full_sync_checkpoint().unwrap()).unwrap();
                if host.try_build_redacted_executable_checkpoint(alice).is_ok() { accepted.push((compiled, case)); }
                assert_eq!(serde_json::to_value(host.try_build_full_sync_checkpoint().unwrap()).unwrap(), before);
            }
        }
        assert!(accepted.is_empty(), "private metadata captures were admitted: {accepted:?}");
    }
    #[test]
    fn public_perspective_checkpoint_preserves_continuous_condition_value_and_duration() {
        let _guard = crate::test_id_counter_guard();
        let (mut host, source, _, alice) = perspective_private_fixture(false);
        let definition = ironsmith::cards::builders::CardDefinitionBuilder::new(CardId::new(), "Public metadata recipient")
            .card_types(vec![CardType::Creature])
            .power_toughness(ironsmith::card::PowerToughness::fixed(2, 3)).build();
        host.registry.register(definition.clone());
        let recipient = host.game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let mut filter = ironsmith::target::ObjectFilter::creature();
        filter.controller = Some(ironsmith::target::PlayerFilter::You);
        let mut effect = ironsmith::continuous::ContinuousEffect::anthem(source, alice, filter.clone(), 0, 0);
        effect.modification = ironsmith::continuous::Modification::ModifyPowerToughnessValue {
            power: ironsmith::effect::Value::Count(filter.clone()), toughness: ironsmith::effect::Value::Fixed(1),
        };
        effect.condition = Some(ironsmith::ConditionExpr::And(Box::new(ironsmith::ConditionExpr::YouControl(filter)),
            Box::new(ironsmith::ConditionExpr::Not(Box::new(ironsmith::ConditionExpr::SourceIsEquipped)))));
        effect.duration = ironsmith::effect::Until::ForAsLongAs(
            ironsmith_core::effect::ContinuousDurationPredicate::ObjectOnBattlefield(
                ironsmith_core::effect::ContinuousDurationObject::Specific(source)));
        host.game.effect_store.continuous_effects.add_effect(effect);
        host.game.refresh_continuous_state().unwrap();
        let checkpoint = host.try_build_redacted_executable_checkpoint(alice).unwrap();
        let checkpoint: SyncCheckpoint = serde_json::from_slice(&serde_json::to_vec(&checkpoint).unwrap()).unwrap();
        let mut guest = WasmGame::new();
        guest.initialize_empty_match(vec!["Carol".into(), "Dan".into()], 20, 2);
        guest.registry.register(host.registry.get("Full checkpoint typed owner").unwrap().clone());
        guest.registry.register(definition.clone());
        guest.apply_foreign_sync_checkpoint_for_perspective(checkpoint, alice.0).unwrap();
        for peer in [&mut host, &mut guest] {
            let stats = |state: &GameState| {
                let value = state.calculated_characteristics(recipient).unwrap();
                (value.power, value.toughness)
            };
            assert_eq!(stats(&peer.game), (Some(3), Some(4)));
            peer.game.create_object_from_definition(&definition, alice, Zone::Battlefield);
            peer.game.refresh_continuous_state().unwrap();
            assert_eq!(stats(&peer.game), (Some(4), Some(4)), "count expression reevaluates after import");
            let departed = peer.game.move_object(source, Zone::Graveyard, ironsmith::events::cause::EventCause::effect()).unwrap();
            peer.game.refresh_continuous_state().unwrap();
            assert_eq!(stats(&peer.game), (Some(2), Some(3)), "captured source duration expires after import");
            peer.game.move_object(departed, Zone::Battlefield, ironsmith::events::cause::EventCause::effect()).unwrap();
            peer.game.refresh_continuous_state().unwrap();
            assert_eq!(stats(&peer.game), (Some(2), Some(3)), "expired duration cannot resume on return");
        }
    }
    #[test]
    fn public_perspective_checkpoint_rejects_forged_continuous_metadata_atomically() {
        let _guard = crate::test_id_counter_guard();
        for case in 0..4 {
            let (mut host, source, _, alice) = perspective_private_fixture(false);
            let mut initial = ironsmith::continuous::ContinuousEffect::anthem(
                source, alice, ironsmith::target::ObjectFilter::creature(), 1, 1);
            if case == 3 {
                initial.duration = ironsmith::effect::Until::ForAsLongAs(
                    ironsmith_core::effect::ContinuousDurationPredicate::ObjectOnBattlefield(
                        ironsmith_core::effect::ContinuousDurationObject::Source));
            }
            host.game.effect_store.continuous_effects.add_effect(initial);
            host.game.refresh_continuous_state().unwrap();
            let mut checkpoint = host.try_build_redacted_executable_checkpoint(alice).unwrap();
            let effect = &mut checkpoint.executable_state.as_mut().unwrap().continuous.effects[0];
            let marker = "Private perspective program marker";
            let mut captured = ironsmith::target::ObjectFilter::default();
            captured.name = Some(marker.into());
            use ironsmith::continuous::ContinuousModification as Modification;
            match case {
                0 => effect.condition = Some(ironsmith::ConditionExpr::Not(Box::new(ironsmith::ConditionExpr::YouControl(captured)))),
                1 => effect.modification = Modification::SetName(marker.into()),
                2 => effect.modification = Modification::SetPower {
                    value: ironsmith::effect::Value::Add(Box::new(ironsmith::effect::Value::Fixed(1)),
                        Box::new(ironsmith::effect::Value::Count(captured))),
                    sublayer: ironsmith::continuous::PtSublayer::Setting,
                },
                _ => effect.duration = ironsmith::effect::Until::ForAsLongAs(
                    ironsmith_core::effect::ContinuousDurationPredicate::ObjectHasCounter {
                        object: ironsmith_core::effect::ContinuousDurationObject::Source,
                        counter_type: ironsmith::CounterType::Named(marker.into()), minimum: 1,
                    }),
            }
            let checkpoint = serde_json::from_slice(&serde_json::to_vec(&checkpoint).unwrap()).unwrap();
            let before = serde_json::to_value(host.try_build_full_sync_checkpoint().unwrap()).unwrap();
            let error = host.with_runtime_transaction(|candidate|
                candidate.apply_foreign_sync_checkpoint_for_perspective(checkpoint, alice.0)).unwrap_err();
            assert!(error.contains("perspective continuous"), "{error}");
            assert_eq!(serde_json::to_value(host.try_build_full_sync_checkpoint().unwrap()).unwrap(), before);
        }
    }
    #[test]
    fn public_perspective_checkpoint_preserves_structural_continuous_filter() {
        let _guard = crate::test_id_counter_guard();
        let (mut host, source, _, alice) = perspective_private_fixture(false);
        let definition = ironsmith::cards::builders::CardDefinitionBuilder::new(CardId::new(), "Public filter recipient")
            .card_types(vec![CardType::Creature])
            .power_toughness(ironsmith::card::PowerToughness::fixed(2, 3)).build();
        host.registry.register(definition.clone());
        let recipient = host.game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let bob = PlayerId::from_index(1);
        let other = host.game.create_object_from_definition(&definition, bob, Zone::Battlefield);
        let mut filter = ironsmith::target::ObjectFilter::default();
        filter.controller = Some(ironsmith::target::PlayerFilter::You);
        filter.zone = Some(Zone::Battlefield);
        filter.any_of = vec![ironsmith::target::ObjectFilter::creature()];
        host.game.effect_store.continuous_effects.add_effect(
            ironsmith::continuous::ContinuousEffect::anthem(source, alice, filter, 1, 1));
        host.game.refresh_continuous_state().unwrap();
        let checkpoint = host.try_build_redacted_executable_checkpoint(alice).unwrap();
        let checkpoint: SyncCheckpoint = serde_json::from_slice(&serde_json::to_vec(&checkpoint).unwrap()).unwrap();
        let mut guest = WasmGame::new();
        guest.initialize_empty_match(vec!["Carol".into(), "Dan".into()], 20, 2);
        guest.registry.register(host.registry.get("Full checkpoint typed owner").unwrap().clone());
        guest.registry.register(definition);
        guest.apply_foreign_sync_checkpoint_for_perspective(checkpoint, alice.0).unwrap();
        for peer in [&host, &guest] {
            let boosted = peer.game.calculated_characteristics(recipient).unwrap();
            let excluded = peer.game.calculated_characteristics(other).unwrap();
            assert_eq!((boosted.power, boosted.toughness), (Some(3), Some(4)));
            assert_eq!((excluded.power, excluded.toughness), (Some(2), Some(3)));
        }
    }
    #[test]
    fn public_perspective_checkpoint_rejects_forged_continuous_targets_atomically() {
        let _guard = crate::test_id_counter_guard();
        for case in 0..4 {
            let (mut host, source, hidden, alice) = perspective_private_fixture(false);
            host.game.effect_store.continuous_effects.add_effect(
                ironsmith::continuous::ContinuousEffect::anthem(source, alice,
                    ironsmith::target::ObjectFilter::creature(), 1, 1));
            let mut checkpoint = host.try_build_redacted_executable_checkpoint(alice).unwrap();
            let effect = &mut checkpoint.executable_state.as_mut().unwrap().continuous.effects[0];
            use ironsmith::continuous::{EffectSourceType, EffectTarget};
            match case {
                0 | 1 => {
                    let mut captured = ironsmith::target::ObjectFilter::default();
                    captured.name = Some("Private perspective program marker".into());
                    if case == 1 {
                        let mut outer = ironsmith::target::ObjectFilter::default();
                        outer.any_of.push(captured);
                        captured = outer;
                    }
                    effect.applies_to = EffectTarget::Filter(captured);
                },
                2 => effect.applies_to = EffectTarget::Specific(hidden),
                _ => effect.source_type = EffectSourceType::Resolution { locked_targets: vec![hidden] },
            }
            let checkpoint = serde_json::from_slice(&serde_json::to_vec(&checkpoint).unwrap()).unwrap();
            let before = serde_json::to_value(host.try_build_full_sync_checkpoint().unwrap()).unwrap();
            let error = host.with_runtime_transaction(|candidate|
                candidate.apply_foreign_sync_checkpoint_for_perspective(checkpoint, alice.0)).unwrap_err();
            assert!(error.contains(if case < 2 { "continuous filter" } else { "disclosed identity" }), "{error}");
            assert_eq!(serde_json::to_value(host.try_build_full_sync_checkpoint().unwrap()).unwrap(), before);
        }
    }
    #[test]
    fn public_perspective_checkpoint_rejects_private_grant_face_constraint() {
        let _guard = crate::test_id_counter_guard();
        let (mut host, source, hidden, alice) = perspective_private_fixture(false);
        let bob = PlayerId::from_index(1);
        host.game.effect_store.grant_registry.grant_play_from_to_card(hidden, Zone::Library, alice,
            ironsmith::grant_registry::PlayFromConstraints::default(),
            ironsmith::grant_registry::GrantSource::until_end_of_turn(source, 4));
        let mut grants = host.game.effect_store.grant_registry.registered_state();
        grants.grants[0].required_face_name = Some("Private perspective program marker".into());
        host.game.effect_store.grant_registry.restore_registered_state(grants).unwrap();
        let before = serde_json::to_value(host.try_build_full_sync_checkpoint().unwrap()).unwrap();
        let result = host.try_build_redacted_executable_checkpoint(bob);
        if let Ok(checkpoint) = &result {
            assert!(!serde_json::to_string(checkpoint).unwrap().contains("Private perspective program marker"),
                "opponent-only grant constraint disclosed a hidden library identity despite opaque live root");
        }
        assert!(result.is_err(), "unapproved face constraint must not be silently removed");
        assert_eq!(serde_json::to_value(host.try_build_full_sync_checkpoint().unwrap()).unwrap(), before);
    }
    #[test]
    fn public_perspective_checkpoint_preserves_disclosed_play_permission_constraints() {
        let _guard = crate::test_id_counter_guard();
        for enters_tapped in [false, true] {
            let (mut host, source, alice) = full_fixture();
            host.game.turn.phase = Phase::FirstMain;
            host.game.turn.step = None;
            host.game.turn.active_player = alice;
            host.game.turn.priority_player = Some(alice);
            let definition = ironsmith::cards::builders::CardDefinitionBuilder::new(CardId::new(), "Disclosed permission land")
                .card_types(vec![CardType::Land]).build();
            host.registry.register(definition.clone());
            let target = host.game.create_object_from_definition(&definition, alice, Zone::Exile);
            let constraints = ironsmith::grant_registry::PlayFromConstraints {
                lands_enter_tapped: enters_tapped,
                spell_cost_increase: Some(ironsmith::mana::ManaCost::new()),
                spell_cost_reduction: None,
            };
            host.game.effect_store.grant_registry.grant_play_from_to_card(target, Zone::Exile, alice,
                constraints, ironsmith::grant_registry::GrantSource::until_end_of_turn(source, 4));
            let expected = host.game.effect_store.grant_registry.registered_state();
            let checkpoint = host.try_build_redacted_executable_checkpoint(alice).unwrap();
            let checkpoint = serde_json::from_slice(&serde_json::to_vec(&checkpoint).unwrap()).unwrap();
            let mut guest = WasmGame::new();
            guest.initialize_empty_match(vec!["Carol".into(), "Dan".into()], 20, 2);
            guest.apply_foreign_sync_checkpoint_for_perspective(checkpoint, alice.0).unwrap();
            assert_eq!(guest.game.effect_store.grant_registry.registered_state(), expected,
                "public permissions retain their constraints, lifetime and permission allocators");
            for peer in [&mut host, &mut guest] {
                let mut dm = ironsmith::decision::AutoPassDecisionMaker;
                ironsmith::special_actions::perform(ironsmith::special_actions::SpecialAction::PlayLand { card_id: target },
                    &mut peer.game, alice, &mut dm).unwrap();
                let entered = peer.game.battlefield.iter().copied().find(|id|
                    peer.game.object(*id).is_some_and(|object| object.name == "Disclosed permission land")).unwrap();
                assert_eq!(peer.game.is_tapped(entered), enters_tapped,
                    "real land play must honor the retained entry constraint, including untapped control");
                assert!(!peer.game.player(alice).unwrap().can_play_land());
            }
        }
    }
    #[test]
    fn public_perspective_checkpoint_rejects_unapproved_grant_metadata() {
        let _guard = crate::test_id_counter_guard();
        for case in 0..4 {
            let (mut host, source, hidden, alice) = perspective_private_fixture(false);
            let target = if case == 0 { hidden } else { source };
            host.game.effect_store.grant_registry.grant_play_from_to_card(target, Zone::Library, alice,
                ironsmith::grant_registry::PlayFromConstraints::default(),
                ironsmith::grant_registry::GrantSource::until_end_of_turn(source, 4));
            let mut grants = host.game.effect_store.grant_registry.registered_state();
            match case {
                0 => {},
                1 => grants.grants[0].required_face_name = Some("Private perspective program marker".into()),
                2 => grants.grants[0].filter = Some(Default::default()),
                _ => grants.grants[0].cast_this_way_filter = Some(Default::default()),
            }
            host.game.effect_store.grant_registry.restore_registered_state(grants).unwrap();
            let before = serde_json::to_value(host.try_build_full_sync_checkpoint().unwrap()).unwrap();
            let error = host.try_build_redacted_executable_checkpoint(alice).unwrap_err();
            assert!(error.contains(if case == 0 { "disclosed identity" } else { "grant constraints" }), "{error}");
            assert_eq!(serde_json::to_value(host.try_build_full_sync_checkpoint().unwrap()).unwrap(), before);
        }
    }
    #[test]
    fn public_perspective_checkpoint_rejects_forged_grant_constraints_atomically() {
        let _guard = crate::test_id_counter_guard();
        for case in 0..4 {
            let (mut host, source, hidden, alice) = perspective_private_fixture(false);
            host.game.effect_store.grant_registry.grant_play_from_to_card(source, Zone::Battlefield, alice,
                ironsmith::grant_registry::PlayFromConstraints::default(),
                ironsmith::grant_registry::GrantSource::until_end_of_turn(source, 4));
            let mut checkpoint = host.try_build_redacted_executable_checkpoint(alice).unwrap();
            let grant = &mut checkpoint.executable_state.as_mut().unwrap().grants.grants[0];
            match case {
                0 => { grant.target_id = Some(hidden); grant.target_stable_id = Some(host.game.object(hidden).unwrap().stable_id); },
                1 => grant.required_face_name = Some("Private perspective program marker".into()),
                2 => grant.filter = Some(Default::default()),
                _ => grant.cast_this_way_filter = Some(Default::default()),
            }
            let checkpoint = serde_json::from_slice(&serde_json::to_vec(&checkpoint).unwrap()).unwrap();
            let before = serde_json::to_value(host.try_build_full_sync_checkpoint().unwrap()).unwrap();
            let error = host.with_runtime_transaction(|candidate|
                candidate.apply_foreign_sync_checkpoint_for_perspective(checkpoint, alice.0)).unwrap_err();
            assert!(error.contains(if case == 0 { "disclosed identity" } else { "grant constraints" }), "{error}");
            assert_eq!(serde_json::to_value(host.try_build_full_sync_checkpoint().unwrap()).unwrap(), before,
                "forged grant rejection must preserve all executable roots and allocator state");
        }
    }
    #[test]
    fn public_perspective_checkpoint_rejects_anonymous_private_copy_without_mutation() {
        let _guard = crate::test_id_counter_guard();
        for (compiled, disclosed_target) in [(false, false), (false, true), (true, false), (true, true)] {
            let (mut host, source, hidden, alice) = perspective_private_fixture(compiled);
            let target = if disclosed_target { source } else { hidden };
            let mut values = ironsmith::snapshot::CopiableValues::from_object(host.game.object(hidden).unwrap());
            values.name = "Opaque copy".into();
            values.compiled_card_text.clear();
            values.ability_labels.clear();
            host.game.effect_store.continuous_effects.add_effect(ironsmith::continuous::ContinuousEffect::from_resolution(
                source, alice, vec![target], ironsmith::continuous::Modification::CopyOf {
                    target_id: target, copiable_values: Box::new(values), preserve_source_abilities: false,
                    name_override: None, name_override_surface: None, add_supertypes: vec![],
                }));
            let before = serde_json::to_value(host.try_build_full_sync_checkpoint().unwrap()).unwrap();
            assert!(serde_json::to_string(&before).unwrap().contains("7777"), "control retains private executable body despite cleared text/labels");
            let error = host.try_build_redacted_executable_checkpoint(alice).expect_err("anonymous copy body is not authorized by source visibility");
            assert!(error.contains(if disclosed_target { "perspective continuous payload requires content approval" }
                else { "disclosed identity" }), "{error}");
            assert_eq!(serde_json::to_value(host.try_build_full_sync_checkpoint().unwrap()).unwrap(), before);
        }
    }
    #[test]
    fn public_perspective_checkpoint_rejects_relabeled_full_and_unreachable_private_payloads() {
        let _guard = crate::test_id_counter_guard();
        for compiled in [false, true] {
            for case in ["full-label", "unreachable"] {
                let (mut host, _, hidden, alice) = perspective_private_fixture(compiled);
                let full = host.try_build_full_sync_checkpoint().unwrap();
                let mut redacted = host.try_build_redacted_executable_checkpoint(alice).unwrap();
                assert!(!serde_json::to_string(&redacted).unwrap().contains("7777"));
                assert!(redacted.executable_state.as_ref().unwrap().objects.iter().find(|o| o.id.0 == hidden.0).unwrap().card.is_none());
                if case == "full-label" {
                    redacted.executable_state = full.executable_state;
                } else {
                    let incoming = full.executable_state.unwrap();
                    let state = redacted.executable_state.as_mut().unwrap();
                    state.graph_card_count = incoming.graph_card_count;
                    state.definitions = incoming.definitions;
                    state.occurrences = incoming.occurrences;
                }
                assert!(serde_json::to_string(&redacted).unwrap().contains("7777"), "malformed carrier actually contains private body");
                let before = serde_json::to_value(host.try_build_full_sync_checkpoint().unwrap()).unwrap();
                let error = host.with_runtime_transaction(|candidate|
                    candidate.apply_foreign_sync_checkpoint_for_perspective(redacted, alice.0)).expect_err("carrier label cannot authorize hidden or unreachable body");
                if case == "full-label" { assert!(error.contains("contradicts checkpoint metadata"), "{error}"); }
                else { assert!(error.contains("noncanonical or unreachable payloads"), "{error}"); }
                assert_eq!(serde_json::to_value(host.try_build_full_sync_checkpoint().unwrap()).unwrap(), before,
                    "recipient rejection must preserve runtime and allocator");
            }
        }
    }
    #[test]
    fn public_opaque_executable_root_preserves_physical_state_without_private_program() {
        let _guard = crate::test_id_counter_guard();
        let (mut host, source, alice) = full_fixture();
        let definition = ironsmith::cards::builders::CardDefinitionBuilder::new(CardId::new(), "Private opaque executable marker")
            .card_types(vec![CardType::Artifact])
            .with_ability(ironsmith::Ability::mana(ironsmith::TotalCost::free(), vec![ironsmith::mana::ManaSymbol::Green]))
            .with_spell_effect(vec![ironsmith::Effect::gain_life(7777)])
            .build();
        host.registry.register(definition.clone());
        let hidden = host.game.create_object_from_definition(&definition, alice, Zone::Library);
        host.game.add_counters(hidden, ironsmith::CounterType::Charge, 3).unwrap();
        let mut original = host.game.object(hidden).unwrap().clone();
        original.initial_controller = PlayerId::from_index(1);
        original.last_modified = 17;
        original.attached_to = Some(ironsmith::object::AttachmentTarget::Object(source));
        original.attachments = vec![source];
        validate_opaque_sync_executable_object(&original).expect_err("private identity cannot be admitted as opaque");
        let projected = opaque_sync_executable_object(original.clone());
        validate_opaque_sync_executable_object(&projected).unwrap();
        assert_eq!((projected.id, projected.stable_id, projected.owner, projected.initial_controller, projected.zone),
            (original.id, original.stable_id, original.owner, original.initial_controller, original.zone));
        assert_eq!(projected.last_modified, original.last_modified);
        assert_eq!(projected.counters, original.counters);
        assert_eq!(projected.counters.get(&ironsmith::CounterType::Charge), Some(&3));
        assert_eq!(projected.attached_to, original.attached_to);
        assert_eq!(projected.attachments, original.attachments);
        assert!(projected.card.is_none() && projected.abilities.is_empty() && projected.spell_effect.is_none());
        let state = SyncExecutableState::retain(&host.game, &host.registry, vec![projected, host.game.object(source).unwrap().clone()],
            host.game.effect_store.continuous_effects.registered_state(),
            ironsmith::replacement::ReplacementEffectManager::new().registered_state().unwrap(),
            ironsmith::prevention::PreventionEffectManager::new().retained_state().unwrap()).unwrap();
        let json = serde_json::to_string(&state).unwrap();
        assert!(!json.contains("Private opaque executable marker"));
        assert!(!json.contains("7777"));
    }
    #[test]
    fn public_opaque_executable_root_rejects_nested_captures_costs_and_programs() {
        let _guard = crate::test_id_counter_guard();
        let (host, source, _) = full_fixture();
        let base = opaque_sync_executable_object(host.game.object(source).unwrap().clone());
        for case in ["ability", "spell", "cast-tags", "face", "cost", "chosen-x"] {
            let mut forged = base.clone();
            match case {
                "ability" => forged.abilities = std::sync::Arc::new(vec![ironsmith::Ability::activated(
                    ironsmith::TotalCost::free(), vec![ironsmith::Effect::gain_life(7777)])]),
                "spell" => forged.spell_effect = host.game.object(source).unwrap().spell_effect.clone(),
                "cast-tags" => { forged.cast_tagged_objects.insert("private-history".into(), vec![ironsmith::snapshot::ObjectSnapshot::from_object(host.game.object(source).unwrap(), &host.game)]); },
                "face" => forged.other_face = host.game.object(source).unwrap().card,
                "cost" => forged.additional_cost = ironsmith::TotalCost::from_costs(vec![ironsmith::costs::Cost::tap()]).into(),
                "chosen-x" => forged.x_value = Some(7777),
                _ => unreachable!(),
            }
            assert!(validate_opaque_sync_executable_object(&forged).unwrap_err().contains("private identity or executable state"),
                "complete native carrier rejects {case}");
            validate_opaque_sync_executable_object(&base).unwrap();
        }
    }
    #[test]
    fn public_full_executable_checkpoint_preserves_grant_registry_budgets_and_allocators() {
        let _guard = crate::test_id_counter_guard();
        let (mut host, source, alice) = full_fixture();
        let definition = ironsmith::cards::builders::CardDefinitionBuilder::new(CardId::new(), "Granted checkpoint card")
            .card_types(vec![CardType::Artifact]).build();
        host.registry.register(definition.clone());
        let target = host.game.create_object_from_definition(&definition, alice, Zone::Exile);
        let registry = &mut host.game.effect_store.grant_registry;
        let exhausted = registry.create_shared_usage_budget(1);
        let unused = registry.create_shared_usage_budget(3);
        registry.grant_play_from_to_card_in_shared_budget(target, None, Zone::Exile, alice,
            ironsmith::grant_registry::PlayFromConstraints::default(),
            ironsmith::grant_registry::GrantSource::until_end_of_turn(source, 4), exhausted);
        assert!(registry.consume_shared_usage(exhausted));
        let expected = registry.registered_state();
        assert_eq!(expected.grants.len(), 1);
        let checkpoint = host.try_build_full_sync_checkpoint().unwrap();
        let checkpoint: SyncCheckpoint = serde_json::from_slice(&serde_json::to_vec(&checkpoint).unwrap()).unwrap();
        let mut guest = WasmGame::new();
        guest.initialize_empty_match(vec!["Carol".into(), "Dan".into()], 20, 2);
        guest.apply_sync_checkpoint(checkpoint).unwrap();
        assert_eq!(guest.game.effect_store.grant_registry.registered_state(), expected,
            "owning checkpoint must preserve runtime grants, consumed budgets and both allocator high-water marks");
        assert!(!guest.game.effect_store.grant_registry.consume_shared_usage(exhausted));
        assert!(guest.game.effect_store.grant_registry.consume_shared_usage(unused));
        assert_eq!(host.game.effect_store.grant_registry.registered_state(), expected,
            "peer use cannot consume the sender's budget");
        let next = guest.game.effect_store.grant_registry.create_shared_usage_budget(2);
        assert_ne!(next, exhausted);
        assert_ne!(next, unused);
    }
    #[test]
    fn public_full_executable_checkpoint_preserves_used_permissions_after_provider_departure() {
        let _guard = crate::test_id_counter_guard();
        for variant in 0..3 {
            let (mut host, _, alice) = full_fixture();
            host.game.turn.phase = Phase::FirstMain;
            let definition = ironsmith::cards::builders::CardDefinitionBuilder::new(CardId::new(), "Once turn checkpoint spell")
                .card_types(vec![CardType::Artifact]).mana_cost(ironsmith::mana::ManaCost::new()).with_spell_effect(vec![ironsmith::Effect::gain_life(1)]).build();
            host.registry.register(definition.clone());
            let target = host.game.create_object_from_definition(&definition, alice, Zone::Exile);
            let provider = host.game.create_object_from_definition(&definition, alice, Zone::Battlefield);
            host.game.effect_store.grant_registry.grant_play_from_to_card(target, Zone::Exile, alice,
                ironsmith::grant_registry::PlayFromConstraints::default(),
                ironsmith::grant_registry::GrantSource::until_end_of_turn(provider, 4));
            let mut state = host.game.effect_store.grant_registry.registered_state();
            let permission = match variant {
                0 => state.grants[0].permission_identity.clone().unwrap(),
                1 => ironsmith::grant_registry::GrantPermissionIdentity::Static {
                    source: provider, origin: ironsmith::continuous::AbilityOrigin::Printed(3),
                    printed_face: Some(definition.card.id),
                },
                _ => ironsmith::grant_registry::GrantPermissionIdentity::LinkedFace {
                    source: provider, face: definition.card.id, slot: 4,
                },
            };
            state.grants[0].permission_identity = Some(permission.clone());
            state.grants[0].usage_limit = Some(ironsmith::grant::GrantUsageLimit::OnceEachTurn);
            host.game.effect_store.grant_registry.restore_registered_state(state).unwrap();
            let can_play = |game: &ironsmith::game_state::GameState| {
                ironsmith::decision::compute_legal_actions(game, alice).unwrap().iter()
                    .any(|action| matches!(action, ironsmith::decision::LegalAction::CastSpell { spell_id, .. } if *spell_id == target))
            };
            assert!(can_play(&host.game), "unused permission initially offers spell cast");
            host.game.turn_store.grant_cast_uses_this_turn.extend([
                (alice, permission),
                (alice, ironsmith::grant_registry::GrantPermissionIdentity::Stored(0)),
                (alice, ironsmith::grant_registry::GrantPermissionIdentity::Static {
                    source: provider, origin: ironsmith::continuous::AbilityOrigin::Printed(3),
                    printed_face: Some(definition.card.id),
                }),
                (alice, ironsmith::grant_registry::GrantPermissionIdentity::LinkedFace {
                    source: provider, face: definition.card.id, slot: 4,
                }),
            ]);
            host.game.move_object(provider, Zone::Graveyard, ironsmith::events::cause::EventCause::effect()).unwrap();
            assert!(host.game.object(provider).is_none());
            assert!(!can_play(&host.game), "provider departure must not reset use");
            let checkpoint = host.try_build_full_sync_checkpoint().unwrap();
            let wire = serde_json::to_vec(&checkpoint).unwrap();
            let mut rows = host.game.turn_store.grant_cast_uses_this_turn.iter().cloned().collect::<Vec<_>>();
            rows.reverse();
            host.game.turn_store.grant_cast_uses_this_turn = rows.into_iter().collect();
            assert_eq!(serde_json::to_vec(&host.try_build_full_sync_checkpoint().unwrap()).unwrap(), wire,
                "HashSet order cannot change canonical checkpoint encoding");
            let checkpoint: SyncCheckpoint = serde_json::from_slice(&wire).unwrap();
            let mut guest = WasmGame::new();
            guest.initialize_empty_match(vec!["Carol".into(), "Dan".into()], 20, 2);
            guest.apply_sync_checkpoint(checkpoint).unwrap();
            assert!(!can_play(&guest.game), "checkpoint import reset consumed permission variant {variant}");
            let restored_permission = guest.game.effect_store.grant_registry.registered_state().grants[0].permission_identity.clone().unwrap();
            assert!(guest.game.turn_store.grant_cast_uses_this_turn.contains(&(alice, restored_permission)),
                "ledger and permission must share rebound face identities");
            let used = guest.game.turn_store.grant_cast_uses_this_turn.clone();
            guest.game.turn_store.grant_cast_uses_this_turn.clear();
            assert!(can_play(&guest.game), "turn usage reset makes retained permission usable again");
            guest.game.turn_store.grant_cast_uses_this_turn = used;
            let mut empty = guest.game.effect_store.grant_registry.registered_state();
            empty.grants.clear();
            guest.game.effect_store.grant_registry.restore_registered_state(empty).unwrap();
            let checkpoint = guest.try_build_full_sync_checkpoint().unwrap();
            host.apply_sync_checkpoint(checkpoint).unwrap();
            assert_eq!(host.game.turn_store.grant_cast_uses_this_turn.len(), 3,
                "historical use survives even with no active grant");
        }
    }
    #[test]
    fn public_full_executable_checkpoint_rejects_invalid_permission_ledger_without_mutation() {
        let _guard = crate::test_id_counter_guard();
        use ironsmith::grant_registry::RetainedGrantPermissionIdentity as Permission;
        for case in ["allocator", "duplicate", "player", "face"] {
            let (mut host, source, alice) = full_fixture();
            host.game.effect_store.grant_registry.grant_play_from_to_card(source, Zone::Battlefield, alice,
                ironsmith::grant_registry::PlayFromConstraints::default(),
                ironsmith::grant_registry::GrantSource::until_end_of_turn(source, 4));
            let before = serde_json::to_value(host.try_build_full_sync_checkpoint().unwrap()).unwrap();
            let mut checkpoint: SyncCheckpoint = serde_json::from_value(before.clone()).unwrap();
            checkpoint.players[0].life = 3;
            let state = checkpoint.executable_state.as_mut().unwrap();
            let expected = match case {
                "allocator" => {
                    state.used_grant_permissions.push((alice, Permission::Stored(state.grants.next_permission_identity)));
                    "used grant permission exceeds its allocator"
                },
                "duplicate" => {
                    state.used_grant_permissions.extend([(alice, Permission::Stored(0)), (alice, Permission::Stored(0))]);
                    "duplicate used grant permission root"
                },
                "player" => {
                    state.used_grant_permissions.push((PlayerId::from_index(2), Permission::Stored(0)));
                    "used grant permission has invalid player"
                },
                "face" => {
                    state.used_grant_permissions.push((alice, Permission::LinkedFace { source, face: state.graph_card_count, slot: 4 }));
                    "unknown executable graph reference"
                },
                _ => unreachable!(),
            };
            let error = host.apply_sync_checkpoint(checkpoint).expect_err("malformed ledger must reject before publication");
            assert!(error.contains(expected), "wrong rejection for {case}: {error}");
            assert_eq!(serde_json::to_value(host.try_build_full_sync_checkpoint().unwrap()).unwrap(), before,
                "rejected {case} cannot mutate runtime or identity allocators");
        }
    }
    fn check_delayed_checkpoint_execution(compiled: bool) {
        let _guard = crate::test_id_counter_guard();
        let (mut host, _, alice) = full_fixture();
        let definition = if compiled {
            ironsmith_registry_test::compile_to_runtime_definition("Delayed checkpoint source",
                "Type: Sorcery\nYou gain X life.", false).unwrap()
        } else {
            ironsmith::cards::builders::CardDefinitionBuilder::new(CardId::new(), "Delayed checkpoint source")
                .card_types(vec![CardType::Sorcery]).with_spell_effect(vec![ironsmith::Effect::gain_life(ironsmith::effect::Value::X)]).build()
        };
        host.registry.register(definition.clone());
        let source = host.game.create_object_from_definition(&definition, alice, Zone::Exile);
        let snapshot = ironsmith::snapshot::ObjectSnapshot::from_object(host.game.object(source).unwrap(), &host.game);
        let turn = host.game.turn.turn_number;
        let delayed = ironsmith::triggers::DelayedTrigger {
            trigger: ironsmith::triggers::Trigger::beginning_of_end_step(ironsmith::target::PlayerFilter::You),
            effects: definition.spell_effect.clone().unwrap(), one_shot: true, x_value: Some(3),
            not_before_turn: Some(turn + 1), expires_at_turn: Some(turn + 2),
            expires_before_controller_turn_after: Some(turn + 2), expires_at_end_of_combat: false,
            bound_extra_turn_index: None, while_any_tagged_object_in_zone: None,
            target_objects: vec![source], ability_source: Some(source),
            ability_source_stable_id: Some(snapshot.stable_id), ability_source_name: Some("Captured delayed source".into()),
            ability_source_snapshot: Some(snapshot.clone()), controller: alice, choices: vec![],
            tagged_objects: [("captured delayed object".into(), vec![snapshot.clone()])].into_iter().collect(),
            tagged_players: [("captured delayed player".into(), vec![alice])].into_iter().collect(),
            prepayment: Some(ironsmith::triggers::PendingDelayedTriggerPayment {
                player: alice, source, cost: ironsmith::cost::TotalCost::mana(ironsmith::mana::ManaCost::from_symbols(vec![ironsmith::mana::ManaSymbol::Generic(2)])),
            }), prevention_shield: None,
        };
        host.game.effect_store.delayed_triggers.push(delayed);
        let moved = host.game.move_object(source, Zone::Graveyard, ironsmith::events::cause::EventCause::effect()).unwrap();
        assert!(host.game.object(source).is_none());
        let checkpoint = host.try_build_full_sync_checkpoint().unwrap();
        let checkpoint: SyncCheckpoint = serde_json::from_slice(&serde_json::to_vec(&checkpoint).unwrap()).unwrap();
        let mut guest = WasmGame::new();
        guest.initialize_empty_match(vec!["Carol".into(), "Dan".into()], 20, 2);
        guest.apply_sync_checkpoint(checkpoint).unwrap();
        assert_eq!(guest.game.effect_store.delayed_triggers.len(), 1,
            "actual full checkpoint must preserve delayed registration and executable program");
        let retained = &guest.game.effect_store.delayed_triggers[0];
        assert_eq!(retained.x_value, Some(3));
        assert_eq!(retained.not_before_turn, Some(turn + 1));
        assert_eq!(retained.expires_at_turn, Some(turn + 2));
        assert_eq!(retained.expires_before_controller_turn_after, Some(turn + 2));
        assert_eq!(retained.ability_source, Some(source));
        assert_eq!(retained.ability_source_name.as_deref(), Some("Captured delayed source"));
        assert_eq!(retained.ability_source_stable_id, Some(snapshot.stable_id));
        assert_eq!(retained.ability_source_snapshot.as_ref().unwrap().card, guest.game.object(moved).unwrap().card);
        assert_eq!(retained.tagged_objects["captured delayed object"][0].card,
            retained.ability_source_snapshot.as_ref().unwrap().card);
        assert_eq!(retained.tagged_players["captured delayed player"], vec![alice]);
        assert_eq!(retained.prepayment.as_ref().unwrap().player, alice);
        assert_eq!(retained.prepayment.as_ref().unwrap().source, source);
        assert_eq!(retained.prepayment.as_ref().unwrap().cost, host.game.effect_store.delayed_triggers[0].prepayment.as_ref().unwrap().cost);
        for peer in [&mut host, &mut guest] {
            let event = ironsmith::triggers::TriggerEvent::new_with_provenance(
                ironsmith::events::phase::BeginningOfEndStepEvent::new(alice), ironsmith::provenance::ProvNodeId::default());
            assert!(ironsmith::triggers::check_delayed_triggers(&mut peer.game, &event).is_empty(), "not-before gate survives import");
            peer.game.turn.turn_number = turn + 1;
            let wrong_player = ironsmith::triggers::TriggerEvent::new_with_provenance(
                ironsmith::events::phase::BeginningOfEndStepEvent::new(PlayerId::from_index(1)), ironsmith::provenance::ProvNodeId::default());
            assert!(ironsmith::triggers::check_delayed_triggers(&mut peer.game, &wrong_player).is_empty());
            let entries = ironsmith::triggers::check_delayed_triggers(&mut peer.game, &event);
            assert_eq!(entries.len(), 1);
            assert_eq!(entries[0].x_value, Some(3));
            assert_eq!(entries[0].source, source);
            assert_eq!(entries[0].source_stable_id, snapshot.stable_id);
            assert!(peer.game.effect_store.delayed_triggers.is_empty(), "one-shot registration consumed exactly once");
            let mut queue = ironsmith::triggers::TriggerQueue::new();
            for entry in entries { queue.add(entry); }
            ironsmith::game_loop::put_triggers_on_stack(&mut peer.game, &mut queue).unwrap();
            assert_eq!(peer.game.stack.len(), 1);
            ironsmith::game_loop::resolve_stack_entry(&mut peer.game).unwrap();
            assert_eq!(peer.game.player(alice).unwrap().life, 26,
                "captured X program executes through stack resolution and retained life replacement");
            assert!(ironsmith::triggers::check_delayed_triggers(&mut peer.game, &event).is_empty());
        }
    }
    #[test]
    fn public_full_executable_checkpoint_preserves_native_delayed_execution() { check_delayed_checkpoint_execution(false); }
    #[test]
    fn public_full_executable_checkpoint_preserves_compiled_delayed_execution() { check_delayed_checkpoint_execution(true); }
    #[test]
    fn public_full_executable_checkpoint_rejects_invalid_delayed_actors_without_mutation() {
        let _guard = crate::test_id_counter_guard();
        for case in ["controller", "payment", "tagged-player", "face"] {
            let (mut host, source, alice) = full_fixture();
            let mut trigger = test_delayed_registration(host.game.turn.turn_number, alice);
            trigger.ability_source = Some(source);
            trigger.ability_source_snapshot = Some(ironsmith::snapshot::ObjectSnapshot::from_object(host.game.object(source).unwrap(), &host.game));
            trigger.prepayment = Some(ironsmith::triggers::PendingDelayedTriggerPayment {
                player: alice, source, cost: ironsmith::cost::TotalCost::free(),
            });
            trigger.tagged_players.insert("captured player".into(), vec![alice]);
            host.game.effect_store.delayed_triggers.push(trigger);
            let before = serde_json::to_value(host.try_build_full_sync_checkpoint().unwrap()).unwrap();
            let mut checkpoint: SyncCheckpoint = serde_json::from_value(before.clone()).unwrap();
            checkpoint.players[0].life = 3;
            let state = checkpoint.executable_state.as_mut().unwrap();
            let trigger = &mut state.delayed_triggers[0];
            let expected = match case {
                "controller" => { trigger.controller = PlayerId::from_index(2); "delayed trigger has invalid controller" },
                "payment" => { trigger.prepayment.as_mut().unwrap().player = PlayerId::from_index(2); "delayed payment has invalid player" },
                "tagged-player" => { trigger.tagged_players.insert("captured player".into(), vec![PlayerId::from_index(2)]); "delayed player capture has invalid player" },
                "face" => { trigger.ability_source_snapshot.as_mut().unwrap().card = Some(state.graph_card_count); "unknown executable graph reference" },
                _ => unreachable!(),
            };
            let error = host.apply_sync_checkpoint(checkpoint).expect_err("malformed delayed actor/capture must reject before publication");
            assert!(error.contains(expected), "wrong rejection for {case}: {error}");
            assert_eq!(serde_json::to_value(host.try_build_full_sync_checkpoint().unwrap()).unwrap(), before,
                "rejection cannot mutate life, registration, program, provenance, history or allocator");
        }
    }
    #[test]
    fn public_full_executable_checkpoint_requires_its_payload() {
        let _guard = crate::test_id_counter_guard();
        let (host, _, _) = full_fixture();
        let mut checkpoint = host.build_sync_checkpoint();
        let mut missing_kind = serde_json::to_value(&checkpoint).unwrap();
        missing_kind.as_object_mut().unwrap().remove("executionKind");
        assert!(serde_json::from_value::<SyncCheckpoint>(missing_kind).is_err(),
            "carrier purpose is required, never inferred from payload absence");
        checkpoint.executable_state = None;
        let mut guest = WasmGame::new();
        guest.initialize_empty_match(vec!["Carol".into(), "Dan".into()], 31, 2);
        // A missing catalog entry must not make the malformed carrier fail
        // accidentally before testing whether executable state is mandatory.
        guest.registry.register(host.registry.get("Full checkpoint typed owner").unwrap().clone());
        guest.apply_sync_checkpoint(checkpoint).expect_err("full executable payload cannot silently disappear");
        assert_eq!(guest.game.players[0].name, "Carol");
        assert_eq!(guest.game.players[0].life, 31);
    }
    fn reject_root_without_allocating_card_identity(case: &str) {
        let _guard = crate::test_id_counter_guard();
        let (mut host, _, _) = full_fixture();
        let before = serde_json::to_value(host.build_sync_checkpoint()).unwrap();
        let mut incoming: SyncCheckpoint = serde_json::from_value(before.clone()).unwrap();
        incoming.players[0].life = 3;
        let expected = match case {
            "object-reference" => {
                incoming.executable_state.as_mut().unwrap().objects[0].card = Some(u32::MAX);
                "unknown executable graph reference"
            }
            "definition-reference" => {
                incoming.executable_state.as_mut().unwrap().definitions[0].0 = u32::MAX;
                "unknown executable graph reference"
            }
            "flat-name" => {
                incoming.objects[0].name = "Contradictory executable name".into();
                "contradicts checkpoint metadata"
            }
            "flat-stable" => {
                incoming.objects[0].stable_id += 1;
                "contradicts checkpoint metadata"
            }
            "late-controller" => {
                incoming.objects[0].controller = 1;
                "cannot restore effective controller"
            }
            "foreign-card-counter-late-controller" => {
                incoming.objects[0].controller = 1;
                incoming.id_counters.card = incoming.id_counters.card.checked_add(100).unwrap();
                "cannot restore effective controller"
            }
            _ => panic!("unknown root fixture case"),
        };
        let error = host.apply_sync_checkpoint(incoming).expect_err("invalid root must reject before publication");
        assert!(error.contains(expected), "wrong error for {case}: {error}");
        assert_eq!(serde_json::to_value(host.build_sync_checkpoint()).unwrap(), before,
            "rejected {case} cannot change any live exported state, including identity allocators");
    }
    #[test]
    fn public_foreign_checkpoint_rejects_unapproved_full_carrier_without_mutation() {
        let _guard = crate::test_id_counter_guard();
        let (mut host, _, alice) = full_fixture();
        let private = ironsmith::cards::builders::CardDefinitionBuilder::new(CardId::new(), "Private foreign executable marker")
            .card_types(vec![CardType::Sorcery]).with_spell_effect(vec![ironsmith::Effect::gain_life(7777)]).build();
        host.registry.register(private.clone());
        host.game.create_object_from_definition(&private, PlayerId::from_index(1), Zone::Library);
        let before = serde_json::to_value(host.build_sync_checkpoint()).unwrap();
        let incoming: SyncCheckpoint = serde_json::from_value(before.clone()).unwrap();
        assert!(serde_json::to_string(&incoming).unwrap().contains("7777"), "fixture full graph carries private executable data");
        let error = host.with_runtime_transaction(|candidate|
            candidate.apply_foreign_sync_checkpoint_for_perspective(incoming, alice.0))
            .expect_err("foreign import cannot admit a full owner-only executable graph");
        assert!(error.contains("full executable carrier"), "wrong rejection: {error}");
        assert_eq!(serde_json::to_value(host.build_sync_checkpoint()).unwrap(), before,
            "unapproved carrier must preserve the complete live runtime and identity allocators");
    }
    #[test]
    fn public_foreign_checkpoint_accepts_metadata_carrier_for_matching_perspective() {
        let _guard = crate::test_id_counter_guard();
        let (mut host, source, alice) = full_fixture();
        let incoming = host.try_build_sync_checkpoint_metadata().unwrap();
        assert!(incoming.executable_state.is_none());
        host.with_runtime_transaction(|candidate|
            candidate.apply_foreign_sync_checkpoint_for_perspective(incoming, alice.0)).unwrap();
        assert!(host.game.object(source).is_some());
        assert_eq!(host.perspective, alice);
    }
    #[test]
    fn public_full_executable_checkpoint_rejects_invalid_import_perspective_without_mutation() {
        let _guard = crate::test_id_counter_guard();
        let (mut host, _, _) = full_fixture();
        let before = serde_json::to_value(host.build_sync_checkpoint()).unwrap();
        let incoming: SyncCheckpoint = serde_json::from_value(before.clone()).unwrap();
        let error = host.with_runtime_transaction(|candidate|
            candidate.apply_sync_checkpoint_for_perspective(incoming, 7))
            .expect_err("invalid target perspective cannot publish an imported world");
        assert_eq!(error, "invalid player index");
        assert_eq!(serde_json::to_value(host.build_sync_checkpoint()).unwrap(), before,
            "invalid import perspective must preserve all live state and allocators");
    }
    #[test]
    fn public_full_executable_checkpoint_import_selects_valid_incoming_perspective() {
        let _guard = crate::test_id_counter_guard();
        let (mut host, source, _) = full_fixture();
        let incoming = host.build_sync_checkpoint();
        host.with_runtime_transaction(|candidate|
            candidate.apply_sync_checkpoint_for_perspective(incoming, 1)).unwrap();
        assert_eq!(host.perspective, PlayerId::from_index(1));
        assert!(host.game.object(source).is_some());
    }
    #[test]
    fn public_full_executable_checkpoint_rejects_late_controller_without_allocator_mutation() {
        reject_root_without_allocating_card_identity("late-controller");
    }
    #[test]
    fn public_full_executable_checkpoint_rejects_foreign_card_counter_and_late_controller_without_mutation() {
        reject_root_without_allocating_card_identity("foreign-card-counter-late-controller");
    }
    #[test]
    fn public_full_executable_checkpoint_keeps_card_allocator_peer_local() {
        let _guard = crate::test_id_counter_guard();
        let (mut host, _, _) = full_fixture();
        let mut incoming = host.build_sync_checkpoint();
        let graph_count = incoming.executable_state.as_ref().unwrap().graph_card_count;
        let before = snapshot_id_counters().card;
        incoming.id_counters.card = before.checked_add(100).unwrap();
        host.apply_sync_checkpoint(incoming).expect("valid world imports independently of sender CardId high-water mark");
        assert_eq!(snapshot_id_counters().card, before + graph_count,
            "CardId graph nodes use only peer-local allocation, not the sender's catalog allocator");
    }
    #[test]
    fn public_full_executable_checkpoint_rejects_unknown_object_card_without_allocator_mutation() {
        reject_root_without_allocating_card_identity("object-reference");
    }
    #[test]
    fn public_full_executable_checkpoint_rejects_unknown_definition_slot_without_allocator_mutation() {
        reject_root_without_allocating_card_identity("definition-reference");
    }
    #[test]
    fn public_full_executable_checkpoint_rejects_conflicting_name_without_allocator_mutation() {
        reject_root_without_allocating_card_identity("flat-name");
    }
    #[test]
    fn public_full_executable_checkpoint_rejects_conflicting_stable_id_without_allocator_mutation() {
        reject_root_without_allocating_card_identity("flat-stable");
    }
    #[test]
    fn public_full_executable_checkpoint_rejects_conflicting_roots_and_rolls_back_late_failure() {
        let _guard = crate::test_id_counter_guard();
        let (host, _, _) = full_fixture();
        let valid = host.build_sync_checkpoint();
        for case in 0..7 {
            let mut incoming = valid.clone();
            match case {
                0 => incoming.objects[0].name = "Contradictory name".into(),
                1 => { let state = incoming.executable_state.as_mut().unwrap(); state.objects.push(state.objects[0].clone()); }
                2 => incoming.objects[0].stable_id += 1,
                3 => incoming.executable_state.as_mut().unwrap().definitions[0].0 = u32::MAX,
                4 => incoming.continuous_timestamps.as_mut().unwrap().current_timestamp += 1,
                5 => incoming.objects[0].controller = 1, // Fails after installation/discovery in candidate world.
                _ => incoming.objects.pop().map(|_| ()).unwrap(),
            }
            let mut guest = WasmGame::new();
            guest.initialize_empty_match(vec!["Carol".into(), "Dan".into()], 31, 2);
            let before_objects = guest.game.object_ids_in_deterministic_order();
            let before_replacements = guest.game.effect_store.replacement_effects.registered_state().unwrap();
            let before_provenance = serde_json::to_value(guest.game.provenance_graph().retained_state()).unwrap();
            let error = guest.apply_sync_checkpoint(incoming).expect_err("malformed full world cannot publish");
            assert!(!error.is_empty(), "case {case}");
            assert_eq!(guest.game.players[0].name, "Carol", "case {case}");
            assert_eq!(guest.game.players[0].life, 31, "case {case}");
            assert_eq!(guest.game.object_ids_in_deterministic_order(), before_objects, "case {case}");
            let after_replacements = guest.game.effect_store.replacement_effects.registered_state().unwrap();
            assert_eq!(after_replacements.effects.len(), before_replacements.effects.len(), "case {case}");
            assert_eq!(after_replacements.next_id, before_replacements.next_id, "case {case}");
            assert_eq!(after_replacements.one_shot_effects, before_replacements.one_shot_effects, "case {case}");
            assert_eq!(serde_json::to_value(guest.game.provenance_graph().retained_state()).unwrap(), before_provenance, "case {case}");
            assert!(guest.registry.get("Full checkpoint typed owner").is_none(), "case {case}: catalog publication also rolls back");
            guest.apply_sync_checkpoint(valid.clone()).expect("retry after rollback imports the complete world");
        }
    }
}

#[cfg(test)]
#[test]
#[ignore = "developer diagnostic requires IRONSMITH_PAYMENT_CHECKPOINT"]
fn inspect_payment_projection_checkpoint() {
    let _id_counter_guard = crate::test_id_counter_guard();
    let path = std::env::var("IRONSMITH_PAYMENT_CHECKPOINT").unwrap();
    let value: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let mut checkpoint: SyncCheckpoint = serde_json::from_value(value.get("checkpoint").unwrap_or(&value).clone()).unwrap();
    checkpoint.turn.priority_player = None;
    let ids: Vec<_> = checkpoint.objects.iter().map(|object| ObjectId::from_raw(object.id)).collect();
    let mut wasm = WasmGame::new();
    eprintln!("Loading checkpoint card definitions");
    let names: Vec<_> = checkpoint.objects.iter().map(|object| object.name.clone()).collect();
    let payloads = ironsmith_tools::load_card_payloads_by_names(
        ironsmith_tools::default_cards_path().to_str().unwrap(), &names,
    ).unwrap();
    for payload in payloads.values().flatten() {
        wasm.registry.register(ironsmith_tools::compile_runtime_definition_from_payload(payload).unwrap());
    }
    eprintln!("Importing checkpoint");
    wasm.apply_sync_checkpoint(checkpoint).unwrap();
    for object in ids.iter().filter_map(|id| wasm.game.object(*id)) {
        for ability in object.abilities.iter().filter(|ability| ability.functions_in(&object.zone)) {
            if let ironsmith::ability::AbilityKind::Static(ability) = &ability.kind {
                eprintln!("STATIC {} {:?}: {:?}", object.name, object.zone, ability);
            }
        }
    }
    let replacements = ironsmith::replacement_ability_processor::generate_replacement_effects_from_abilities(&wasm.game).unwrap();
    for effect in replacements {
        let relevant = effect.matcher.as_ref().map(|matcher| [ironsmith::events::EventKind::BecomeTapped, ironsmith::events::EventKind::ManaAdded, ironsmith::events::EventKind::AbilityActivated].map(|kind| matcher.may_match_event_kind(kind)));
        eprintln!("REPLACEMENT source={:?} mana_relevance={relevant:?} {effect:?}", wasm.game.object(effect.source).map(|object| &object.name));
    }
    eprintln!("CONTINUOUS {:?}", wasm.game.try_all_continuous_effects().unwrap());
    if let Ok(source) = std::env::var("IRONSMITH_PAYMENT_SOURCE") {
        let source = ObjectId::from_raw(source.parse().unwrap());
        let object = wasm.game.object(source).unwrap();
        let generic = std::env::var("IRONSMITH_PAYMENT_GENERIC").ok().map(|value| value.parse::<u32>().unwrap());
        let mut request = ironsmith::mana_payment::ManaPaymentRequest::new(
            wasm.game.controller_of(object), source,
            if generic.is_some() { ironsmith::costs::PaymentReason::ActivateAbility } else { ironsmith::costs::PaymentReason::CastSpell },
            generic.map(|amount| ironsmith::mana::ManaCost::new().add_generic(amount)).unwrap_or_else(|| object.mana_cost_owned().unwrap()),
        );
        if generic.is_some() { request.reserved_tap_sources.push(source); }
        if std::env::var("IRONSMITH_PAYMENT_REASON").as_deref() == Ok("mana_ability") {
            request.reason = ironsmith::costs::PaymentReason::ActivateManaAbility;
        }
        if std::env::var("IRONSMITH_PAYMENT_MANUAL").is_ok() {
            let start = std::time::Instant::now();
            let manual = ironsmith::mana_payment::manual_mana_abilities(&wasm.game, &request);
            eprintln!("MANUAL {:?} options={}", start.elapsed(), manual.len());
        }
        let start = std::time::Instant::now();
        let result = ironsmith::mana_payment::plan_first_mana_payment(&wasm.game, &request);
        eprintln!("PAYMENT {:?} {:?} {:?}", start.elapsed(), result.as_ref().map(|plan| (plan.payable, plan.mana_ability_steps.len())), ironsmith::mana_payment::last_mana_payment_perf());
        assert!(result.unwrap().payable);
    }
}
