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
}

impl From<IdCountersSnapshot> for SyncIdCounters {
    fn from(value: IdCountersSnapshot) -> Self {
        Self {
            player: value.player,
            object: value.object,
            card: value.card,
        }
    }
}

impl SyncIdCounters {
    fn from_game(game: &ironsmith::game_state::GameState) -> Self {
        let mut counters = Self::from(ironsmith::ids::snapshot_id_counters());
        counters.object = game.next_object_id_counter();
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SyncTurn {
    active_player: u8,
    priority_player: Option<u8>,
    turn_number: u32,
    phase: String,
    step: Option<String>,
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
    attached_to: Option<SyncAttachmentTarget>,
    attachments: Vec<u64>,
    tapped: bool,
    summoning_sick: bool,
    monstrous: bool,
    renowned: bool,
    saddled: bool,
    flipped: bool,
    face_down: bool,
    manifested: bool,
    phased_out: bool,
    madness_exiled: bool,
    foretold: bool,
    #[serde(default)]
    suspected: bool,
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
    saddled: bool,
    flipped: bool,
    face_down: bool,
    manifested: bool,
    phased_out: bool,
    madness_exiled: bool,
    foretold: bool,
    #[serde(default)]
    suspected: bool,
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
pub(crate) struct SyncCheckpoint {
    version: u32,
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
    id_counters: SyncIdCounters,
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
    damage_assignment_order: Vec<(u64, Vec<u64>)>,
    attacking_bands: Vec<Vec<u64>>,
    had_to_attack_this_combat: Vec<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
enum SyncGrandMeleeAttackTarget {
    Player { player: u8 },
    Planeswalker { object: u64 },
    Battle { object: u64 },
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

fn sync_zone_from_name(raw: &str) -> Result<Zone, JsValue> {
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
        other => Err(JsValue::from_str(&format!(
            "unknown checkpoint zone: {other}"
        ))),
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

fn sync_phase_from_name(raw: &str) -> Result<Phase, JsValue> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "beginning" | "beginning_phase" => Ok(Phase::Beginning),
        "first_main" | "first main" | "precombat_main" => Ok(Phase::FirstMain),
        "combat" | "combat_phase" => Ok(Phase::Combat),
        "next_main" | "second_main" | "postcombat_main" => Ok(Phase::NextMain),
        "ending" | "ending_phase" => Ok(Phase::Ending),
        other => Err(JsValue::from_str(&format!(
            "unknown checkpoint phase: {other}"
        ))),
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

fn sync_step_from_name(raw: &str) -> Result<Step, JsValue> {
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
        other => Err(JsValue::from_str(&format!(
            "unknown checkpoint step: {other}"
        ))),
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
        "-2/-2" => CounterType::MinusTwoMinusTwo,
        "deathtouch" => CounterType::Deathtouch,
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
        _ => CounterType::Named(Box::leak(normalized.into_boxed_str())),
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

fn sync_stack_entry(entry: &StackEntry) -> SyncStackEntry {
    SyncStackEntry {
        object_id: entry.object_id.0,
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
    }
}

fn sync_grand_melee_combat(combat: &ironsmith::combat_state::CombatState) -> SyncGrandMeleeCombat {
    let mut blockers = combat
        .blockers
        .iter()
        .map(|(attacker, blockers)| (attacker.0, raw_ids(blockers)))
        .collect::<Vec<_>>();
    blockers.sort_by_key(|(attacker, _)| *attacker);
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
                };
                (attacker.creature.0, target)
            })
            .collect(),
        blockers,
        damage_assignment_order,
        attacking_bands: combat
            .attacking_bands
            .iter()
            .map(|band| raw_ids(band))
            .collect(),
        had_to_attack_this_combat,
    }
}

fn grand_melee_combat_from_sync(
    combat: &SyncGrandMeleeCombat,
) -> ironsmith::combat_state::CombatState {
    ironsmith::combat_state::CombatState {
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
) -> Result<ironsmith::GrandMeleeRestore, JsValue> {
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
                        return Err(JsValue::from_str(&format!(
                            "unknown Grand Melee marker status: {other}"
                        )));
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
                        )
                        .map_err(|error| JsValue::from_str(&error))?)
                    },
                })
            })
            .collect::<Result<Vec<_>, JsValue>>()?,
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

fn archenemy_state_from_sync(sync: &SyncArchenemy) -> Result<ArchenemyState, JsValue> {
    let variant = match sync.variant.as_str() {
        "default" => ArchenemyVariant::Default,
        "supervillain_rumble" => ArchenemyVariant::SupervillainRumble,
        "commander" => ArchenemyVariant::Commander,
        other => {
            return Err(JsValue::from_str(&format!(
                "unknown Archenemy variant in checkpoint: {other}"
            )));
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

fn planechase_state_from_sync(sync: &SyncPlanechase) -> Result<PlanechaseState, JsValue> {
    let mut card_kinds = HashMap::new();
    for (object, kind) in &sync.card_kinds {
        let kind = match kind.as_str() {
            "plane" => PlanarCardKind::Plane,
            "phenomenon" => PlanarCardKind::Phenomenon,
            other => {
                return Err(JsValue::from_str(&format!(
                    "unknown planar card kind in checkpoint: {other}"
                )));
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
            HashSet::from([planar_controller])
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
                    amount: *amount,
                })
                .collect::<Vec<_>>(),
            "faceDown": self.game.is_face_down(id),
            "manifested": self.game.is_manifested(id),
            "foretold": self.game.is_foretold(id),
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
        ids.sort_unstable();
        ids.dedup();
        ids
    }

    pub(crate) fn build_sync_checkpoint(&self) -> SyncCheckpoint {
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
                            amount: *amount,
                        })
                        .collect(),
                    attached_to: object.attached_to.map(sync_attachment_target),
                    attachments: raw_ids(&object.attachments),
                    tapped: self.game.is_tapped(id),
                    summoning_sick: self.game.is_summoning_sick(id),
                    monstrous: self.game.is_monstrous(id),
                    renowned: self.game.is_renowned(id),
                    saddled: self.game.is_saddled(id),
                    flipped: self.game.is_flipped(id),
                    face_down: self.game.is_face_down(id),
                    manifested: self.game.is_manifested(id),
                    phased_out: self.game.is_phased_out(id),
                    madness_exiled: self.game.is_madness_exiled(id),
                    foretold: self.game.is_foretold(id),
                    suspected: self.game.is_suspected(id),
                    plotted_by: self.game.plotted_by(id).map(|player| player.0),
                    plotted_turn: self.game.plotted_turn(id),
                    damage_marked: self.game.damage_on(id),
                    commander: self.game.is_commander_object(id),
                    hidden_card: self.game.hidden_card_info(id).map(|info| SyncHiddenCard {
                        owner: info.owner.0,
                        slot: info.slot,
                        commitment: info.commitment.clone(),
                        public_slot: info.public_slot,
                        public_commitment: info.public_commitment.clone(),
                    }),
                })
            })
            .collect();

        let (consecutive_priority_passes, priority_players_in_game) =
            self.priority_state.priority_tracker_snapshot();

        SyncCheckpoint {
            version: SYNC_CHECKPOINT_VERSION,
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
            exiled_with_source: self
                .game
                .exiled_with_source_entries()
                .map(|(source, linked)| (source.0, raw_ids(linked)))
                .collect(),
            return_exiled_when_source_leaves: self
                .game
                .return_exiled_when_source_leaves_ids()
                .map(|id| id.0)
                .collect(),
            id_counters: SyncIdCounters::from_game(&self.game),
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

    pub(crate) fn build_public_audit_checkpoint(&self) -> PublicAuditCheckpoint {
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
                Some(PublicAuditObject {
                    id: object.id.0,
                    stable_id: object.stable_id.0.0,
                    owner: object.owner.0,
                    controller: self.game.controller_of(object).0,
                    zone: sync_zone_name(object.zone).to_string(),
                    identity: self.public_audit_object_identity(id, object),
                    token: matches!(object.kind, ironsmith::object::ObjectKind::Token),
                    power: object.power(),
                    toughness: object.toughness(),
                    loyalty: object.loyalty(),
                    defense: object.defense(),
                    counters: object
                        .counters
                        .iter()
                        .map(|(kind, amount)| SyncCounter {
                            kind: sync_counter_kind(*kind),
                            amount: *amount,
                        })
                        .collect(),
                    attached_to: object.attached_to.map(sync_attachment_target),
                    attachments: raw_ids(&object.attachments),
                    tapped: self.game.is_tapped(id),
                    summoning_sick: self.game.is_summoning_sick(id),
                    monstrous: self.game.is_monstrous(id),
                    renowned: self.game.is_renowned(id),
                    saddled: self.game.is_saddled(id),
                    flipped: self.game.is_flipped(id),
                    face_down: self.game.is_face_down(id) || self.game.is_face_down_conspiracy(id),
                    manifested: self.game.is_manifested(id),
                    phased_out: self.game.is_phased_out(id),
                    madness_exiled: self.game.is_madness_exiled(id),
                    foretold: self.game.is_foretold(id),
                    suspected: self.game.is_suspected(id),
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

        PublicAuditCheckpoint {
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
        }
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
            public_slot: info.public_slot,
            public_commitment: info.public_commitment.clone(),
        });
        Ok(())
    }

    fn build_redacted_sync_checkpoint(
        &self,
        perspective: PlayerId,
    ) -> Result<SyncCheckpoint, JsValue> {
        let mut checkpoint = self.build_sync_checkpoint();
        checkpoint.perspective = perspective.0;
        for object in &mut checkpoint.objects {
            if self.should_redact_for_perspective(object, perspective) {
                self.redact_sync_object(object)?;
            }
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
        self.priority_state
            .set_auto_choose_single_pip_payment(false);
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
    }

    fn sync_object_from_checkpoint(&mut self, object: &SyncObject) -> Result<Object, JsValue> {
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
            self.registry.ensure_cards_loaded([object.name.as_str()]);
            let definition = self.load_compilable_card_definition(&object.name)?;
            self.game
                .register_linked_face_family_from_catalog(&definition, &self.registry);
            Object::from_card_definition(id, &definition, owner, zone)
        };

        restored.zone = zone;
        restored.stable_id = StableId::from_raw(object.stable_id);
        restored.hand_modifier = object.hand_modifier;
        restored.life_modifier = object.life_modifier;
        if object.token {
            restored.compiled_card_text = object.oracle_text.clone().into();
            restored.base_loyalty = object.loyalty;
            restored.base_defense = object.defense;
        }
        restored.counters = object
            .counters
            .iter()
            .map(|counter| (sync_counter_from_name(&counter.kind), counter.amount))
            .collect();
        restored.attached_to = object.attached_to.clone().map(attachment_target_from_sync);
        restored.attachments = object_ids(object.attachments.clone());

        Ok(restored)
    }

    fn apply_sync_checkpoint(&mut self, checkpoint: SyncCheckpoint) -> Result<(), JsValue> {
        if checkpoint.version != SYNC_CHECKPOINT_VERSION {
            return Err(JsValue::from_str(&format!(
                "unsupported checkpoint version: {}",
                checkpoint.version
            )));
        }
        if checkpoint.players.is_empty() {
            return Err(JsValue::from_str("checkpoint has no players"));
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
                player.library = object_ids(player_checkpoint.library.clone());
                player.hand = object_ids(player_checkpoint.hand.clone());
                player.graveyard = object_ids(player_checkpoint.graveyard.clone());
                player.sideboard = object_ids(player_checkpoint.sideboard.clone());
                player.commanders = object_ids(player_checkpoint.commanders.clone());
            }
        }

        self.game.battlefield = object_ids(checkpoint.battlefield.clone());
        self.game.exile = object_ids(checkpoint.exile.clone());
        self.game.command_zone = object_ids(checkpoint.command.clone());
        self.game.ante = object_ids(checkpoint.ante.clone());
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
                )
                .map_err(|error| JsValue::from_str(&error))?;
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
                )
                .map_err(|error| JsValue::from_str(&error))?;
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
                )
                .map_err(|error| JsValue::from_str(&error))?;
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
                )
                .map_err(|error| JsValue::from_str(&error))?;
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
                return Err(JsValue::from_str(
                    "Two-Headed Giant checkpoint profile does not match its team size",
                ));
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
                )
                .map_err(|error| JsValue::from_str(&error))?;
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
                )
                .map_err(|error| JsValue::from_str(&error))?;
        }
        if let Some(grand_melee) = checkpoint.grand_melee.as_ref() {
            self.game
                .restore_grand_melee_snapshot(grand_melee_restore_from_sync(grand_melee)?)
                .map_err(|error| JsValue::from_str(&error))?;
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
                priority_state.set_auto_choose_single_pip_payment(false);
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
                )
                .map_err(|error| JsValue::from_str(&error))?;
        }
        if checkpoint.shared_team_turns {
            if checkpoint.two_headed_giant.is_none() {
                self.game
                    .enable_shared_team_turns()
                    .map_err(|error| JsValue::from_str(&error))?;
            }
            for (team, order) in checkpoint.shared_team_member_orders.iter().enumerate() {
                self.game
                    .set_shared_team_member_order(
                        team,
                        order.iter().copied().map(PlayerId::from_index).collect(),
                    )
                    .map_err(|error| JsValue::from_str(&error))?;
            }
        }
        self.game.set_deploy_creatures(checkpoint.deploy_creatures);

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
            if object.renowned {
                self.game.set_renowned(id);
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
                self.game.set_foretold(id);
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

        let id_counters = IdCountersSnapshot::from(checkpoint.id_counters.clone());
        restore_id_counters(id_counters);
        self.game.set_next_object_id_counter(id_counters.object);
        self.pending_decision = self.game.turn.priority_player.map(|player| {
            DecisionContext::Priority(ironsmith::decisions::context::PriorityContext::new(
                player,
                ironsmith::decision::compute_legal_actions(&self.game, player),
            ))
        });
        Ok(())
    }

    /// Export a WASM-owned resync checkpoint that can hydrate another peer's engine.
    #[wasm_bindgen(js_name = exportSyncCheckpoint)]
    pub fn export_sync_checkpoint(&self) -> Result<JsValue, JsValue> {
        serde_wasm_bindgen::to_value(&self.build_sync_checkpoint())
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
        serde_wasm_bindgen::to_value(&self.build_public_audit_checkpoint())
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
        self.apply_sync_checkpoint(checkpoint)?;
        self.set_perspective(perspective_index)?;
        self.snapshot()
    }
}

#[cfg(test)]
mod sync_checkpoint_tests {
    use super::*;

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

        game.registry.ensure_cards_loaded(["Lightning Bolt"]);
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
                .all(|id| after_order.iter().any(|raw| *raw == id.0)),
            "drawn hand cards must be part of the post-shuffle ziffle order"
        );

        game.reseal_verified_hidden_library_shuffle(ApplyHiddenLibraryShuffleInput {
            owner: 1,
            deck_hash: "mulligan-deck".to_string(),
            after_order: after_order.clone(),
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

        game.registry.ensure_cards_loaded(["Swamp"]);
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
        game.registry.ensure_cards_loaded(["Swamp"]);
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
        game.registry.ensure_cards_loaded(["Mountain"]);
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
        );
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
        let definition = ironsmith::cards::builders::CardDefinitionBuilder::new(
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
        host.registry.ensure_cards_loaded(["Lightning Bolt"]);
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
}
