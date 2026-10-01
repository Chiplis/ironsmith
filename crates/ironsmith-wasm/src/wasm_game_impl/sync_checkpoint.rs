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

const SYNC_CHECKPOINT_VERSION: u32 = 1;

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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SyncPlayer {
    id: u8,
    name: String,
    starting_life: i32,
    life: i32,
    mana_pool: SyncManaPool,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SyncObject {
    id: u64,
    stable_id: u64,
    owner: u8,
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
    controller: u8,
    zone: String,
    identity: Option<PublicAuditObjectIdentity>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SyncCheckpoint {
    version: u32,
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
        ids.extend(self.game.stack.iter().map(|entry| entry.object_id));
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
        self.try_build_sync_checkpoint()
            .expect("sync checkpoint should encode")
    }

    /// Build this engine's full checkpoint. Fails when the shared hidden-claim
    /// ledger holds an entry without a lossless encoding (it is never
    /// silently dropped).
    pub(crate) fn try_build_sync_checkpoint(&self) -> Result<SyncCheckpoint, JsValue> {
        let players = self
            .game
            .players
            .iter()
            .map(|player| SyncPlayer {
                id: player.id.0,
                name: player.name.clone(),
                starting_life: player.starting_life,
                life: player.life,
                mana_pool: SyncManaPool::from(&player.mana_pool),
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
            })
            .collect();

        let objects = self
            .sync_checkpoint_object_ids()
            .into_iter()
            .filter_map(|id| {
                let object = self.game.object(id)?;
                Some(SyncObject {
                    id: object.id.0,
                    stable_id: object.stable_id.0.0,
                    owner: object.owner.0,
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
        Ok(SyncRulesState {
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
        ids.extend(self.game.stack.iter().map(|entry| entry.object_id));
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
            .map(|player| PublicAuditPlayer {
                id: player.id.0,
                name: player.name.clone(),
                starting_life: player.starting_life,
                life: player.life,
                mana_pool: SyncManaPool::from(&player.mana_pool),
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
            })
            .collect();

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
                    controller: self.game.controller_of(object).0,
                    zone: sync_zone_name(object.zone).to_string(),
                    identity: self.public_audit_object_identity(id, object),
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
            version: SYNC_CHECKPOINT_VERSION,
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

    fn build_redacted_sync_checkpoint(
        &self,
        perspective: PlayerId,
    ) -> Result<SyncCheckpoint, JsValue> {
        let mut checkpoint = self.try_build_sync_checkpoint()?;
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
        restored.stable_id = StableId::from_raw(object.stable_id);
        restored.hand_modifier = object.hand_modifier;
        restored.life_modifier = object.life_modifier;
        if object.token {
            restored.compiled_card_text = object.oracle_text.clone().into();
            restored.base_loyalty = object.loyalty;
            restored.base_defense = object.defense;
        }
        let mut counts = std::collections::BTreeMap::new();
        for counter in &object.counters {
            let kind = sync_counter_from_wire(&counter.kind, counter.counter_type)?;
            if counts.insert(kind, counter.amount).is_some() {
                return Err("duplicate counter kind in checkpoint".into());
            }
        }
        let registrations = object.counter_ability_state.clone()
            .map(SyncCounterAbilityState::into_runtime).transpose()?;
        restored.counters = ironsmith::object::ObjectCounters::from_checkpoint(
            counts, registrations,
        ).map_err(|error| format!("invalid counter registrations: {error}"))?;
        restored.attached_to = object.attached_to.clone().map(attachment_target_from_sync);
        restored.attachments = object_ids(object.attachments.clone());

        Ok(restored)
    }

    fn apply_sync_checkpoint(&mut self, checkpoint: SyncCheckpoint) -> Result<(), String> {
        self.with_runtime_transaction(|candidate| candidate.apply_sync_checkpoint_in_branch(checkpoint))
    }

    fn apply_sync_checkpoint_in_branch(&mut self, checkpoint: SyncCheckpoint) -> Result<(), String> {
        if checkpoint.version != SYNC_CHECKPOINT_VERSION {
            return Err(format!(
                "unsupported checkpoint version: {}",
                checkpoint.version
            ));
        }
        if checkpoint.players.is_empty() {
            return Err("checkpoint has no players".to_string());
        }

        self.reset_runtime_for_sync_checkpoint(&checkpoint);

        for object in checkpoint.objects.iter() {
            let restored = self.sync_object_from_checkpoint(object)?;
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
        self.restore_sync_rules_state(&checkpoint.rules, checkpoint.grand_melee.is_some());
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
            let controller = PlayerId::from_index(object.controller);
            if controller != PlayerId::from_index(object.owner) {
                self.game.set_current_controller(id, controller);
            }
        }

        if let Some(timestamps) = checkpoint.continuous_timestamps {
            self.game
                .restore_continuous_timestamp_state(timestamps.into_runtime()?)
                .map_err(|error| format!("invalid continuous chronology: {error}"))?;
        }

        let mut id_counters = IdCountersSnapshot::from(checkpoint.id_counters.clone());
        // Retained session definitions include allocations made before and
        // during import. Their identity allocator must remain monotonic.
        id_counters.card = id_counters.card.max(snapshot_id_counters().card);
        restore_id_counters(id_counters);
        self.game.set_next_object_id_counter(id_counters.object);
        self.game
            .set_next_stack_ability_id_counter(checkpoint.id_counters.stack_ability);
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
            candidate.apply_sync_checkpoint_in_branch(checkpoint)
                .map_err(|error| JsValue::from_str(&error))?;
            candidate.set_perspective(perspective_index)?;
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
        if checkpoint.perspective != perspective_index {
            return Err(JsValue::from_str(&format!(
                "foreign sync checkpoint is redacted for seat {}, not seat {perspective_index}",
                checkpoint.perspective
            )));
        }
        self.with_runtime_transaction(|candidate| {
            candidate.apply_sync_checkpoint_in_branch(checkpoint)
                .map_err(|error| JsValue::from_str(&error))?;
            candidate.set_perspective(perspective_index)?;
            candidate.export_public_audit_checkpoint()
        })
    }
}

#[cfg(test)]
mod sync_checkpoint_tests {
    use super::*;

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
            let original = guest.find_card_definition(original_name).unwrap().clone();
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
