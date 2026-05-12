use ironsmith::game_state::{HiddenCardInfo, Phase, Step, TurnState};
use ironsmith::ids::{IdCountersSnapshot, StableId};
use ironsmith::object::{AttachmentTarget, Object};
use ironsmith::player::ManaPool;
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
struct PublicAuditCheckpoint {
    version: u32,
    format: MatchFormatInput,
    perspective: u8,
    snapshot_serial: u64,
    turn: SyncTurn,
    players: Vec<PublicAuditPlayer>,
    objects: Vec<PublicAuditObject>,
    battlefield: Vec<u64>,
    public_exile: Vec<u64>,
    command: Vec<u64>,
    stack: Vec<SyncStackEntry>,
    hidden_zones: Vec<PublicAuditHiddenZone>,
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
struct SyncCheckpoint {
    version: u32,
    format: MatchFormatInput,
    perspective: u8,
    snapshot_serial: u64,
    auto_cleanup_discard: bool,
    #[serde(default = "default_auto_choose_single_object_decisions")]
    auto_choose_single_object_decisions: bool,
    semantic_threshold: f32,
    turn: SyncTurn,
    players: Vec<SyncPlayer>,
    objects: Vec<SyncObject>,
    battlefield: Vec<u64>,
    exile: Vec<u64>,
    command: Vec<u64>,
    stack: Vec<SyncStackEntry>,
    #[serde(default)]
    exiled_with_source: Vec<(u64, Vec<u64>)>,
    #[serde(default)]
    return_exiled_when_source_leaves: Vec<u64>,
    id_counters: SyncIdCounters,
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
        "sideboard" | "outside_game" | "outside game" | "outside the game" => {
            Ok(Zone::OutsideGame)
        }
        other => Err(JsValue::from_str(&format!("unknown checkpoint zone: {other}"))),
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
        other => Err(JsValue::from_str(&format!("unknown checkpoint phase: {other}"))),
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
        other => Err(JsValue::from_str(&format!("unknown checkpoint step: {other}"))),
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

fn raw_ids(ids: &[ObjectId]) -> Vec<u64> {
    ids.iter().map(|id| id.0).collect()
}

fn object_ids(ids: Vec<u64>) -> Vec<ObjectId> {
    ids.into_iter().map(ObjectId::from_raw).collect()
}

fn public_audit_protocol_name() -> String {
    "mental_poker_bayer_groth_v1".to_string()
}

#[wasm_bindgen]
impl WasmGame {
    fn public_audit_commitment_root(
        &self,
        owner: PlayerId,
        zone_name: &str,
        ids: &[ObjectId],
    ) -> Option<String> {
        let entries = ids
            .iter()
            .enumerate()
            .map(|(position, id)| {
                self.game.hidden_card_info(*id).map(|info| {
                    let public_slot = info.public_slot.unwrap_or(info.slot);
                    let public_commitment = info
                        .public_commitment
                        .as_deref()
                        .unwrap_or(info.commitment.as_str());
                    serde_json::json!({
                        "position": position,
                        "owner": info.owner.0,
                        "slot": public_slot,
                        "commitment": public_commitment,
                    })
                })
            })
            .collect::<Option<Vec<_>>>()?;
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
        ids.extend(self.game.stack.iter().map(|entry| entry.object_id));
        ids.sort_unstable();
        ids.dedup();
        ids
    }

    fn build_sync_checkpoint(&self) -> SyncCheckpoint {
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
                    name: object.name.clone(),
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
                    oracle_text: object.compiled_card_text.clone(),
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
                    plotted_by: self
                        .game
                        .plotted_cards
                        .get(&id)
                        .map(|(player, _)| player.0),
                    plotted_turn: self.game.plotted_turn(id),
                    damage_marked: self.game.damage_marked.get(&id).copied().unwrap_or(0),
                    commander: self.game.is_commander_object(id),
                    hidden_card: self
                        .game
                        .hidden_card_info(id)
                        .map(|info| SyncHiddenCard {
                            owner: info.owner.0,
                            slot: info.slot,
                            commitment: info.commitment.clone(),
                            public_slot: info.public_slot,
                            public_commitment: info.public_commitment.clone(),
                        }),
                })
            })
            .collect();

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
            players,
            objects,
            battlefield: raw_ids(&self.game.battlefield),
            exile: raw_ids(&self.game.exile),
            command: raw_ids(&self.game.command_zone),
            stack: self
                .game
                .stack
                .iter()
                .map(|entry| SyncStackEntry {
                    object_id: entry.object_id.0,
                    controller: entry.controller.0,
                    targets: entry.targets.iter().copied().map(sync_target_input).collect(),
                    is_ability: entry.is_ability,
                    x_value: entry.x_value,
                    source_stable_id: entry.source_stable_id.map(|id| id.0.0),
                    source_name: entry.source_name.clone(),
                })
                .collect(),
            exiled_with_source: self
                .game
                .exiled_with_source
                .iter()
                .map(|(source, linked)| (source.0, raw_ids(linked)))
                .collect(),
            return_exiled_when_source_leaves: self
                .game
                .return_exiled_when_source_leaves
                .iter()
                .map(|id| id.0)
                .collect(),
            id_counters: SyncIdCounters::from(snapshot_id_counters()),
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

    fn public_audit_object_ids(&self) -> Vec<ObjectId> {
        let mut ids = Vec::new();
        for player in &self.game.players {
            ids.extend(player.graveyard.iter().copied());
            ids.extend(player.attachments.iter().copied());
            ids.extend(player.commanders.iter().copied());
        }
        ids.extend(self.game.battlefield.iter().copied());
        ids.extend(self.public_audit_exile_ids());
        ids.extend(self.game.command_zone.iter().copied());
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
        if self.game.is_face_down(id) || self.game.is_foretold(id) {
            return false;
        }
        true
    }

    fn public_audit_object_identity(&self, id: ObjectId, object: &Object) -> Option<PublicAuditObjectIdentity> {
        self.public_audit_object_identity_is_public(id)
            .then(|| PublicAuditObjectIdentity {
                name: object.name.clone(),
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
                oracle_text: object.compiled_card_text.clone(),
            })
    }

    fn build_public_audit_checkpoint(&self) -> PublicAuditCheckpoint {
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
                    face_down: self.game.is_face_down(id),
                    manifested: self.game.is_manifested(id),
                    phased_out: self.game.is_phased_out(id),
                    madness_exiled: self.game.is_madness_exiled(id),
                    foretold: self.game.is_foretold(id),
                    plotted_by: self
                        .game
                        .plotted_cards
                        .get(&id)
                        .map(|(player, _)| player.0),
                    plotted_turn: self.game.plotted_turn(id),
                    damage_marked: self.game.damage_marked.get(&id).copied().unwrap_or(0),
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
                commitment_root: self.public_audit_commitment_root(
                    player.id,
                    "hand",
                    &player.hand,
                ),
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
            players,
            objects,
            battlefield: raw_ids(&self.game.battlefield),
            public_exile: raw_ids(&self.public_audit_exile_ids()),
            command: raw_ids(&self.game.command_zone),
            stack: self
                .game
                .stack
                .iter()
                .map(|entry| SyncStackEntry {
                    object_id: entry.object_id.0,
                    controller: entry.controller.0,
                    targets: entry.targets.iter().copied().map(sync_target_input).collect(),
                    is_ability: entry.is_ability,
                    x_value: entry.x_value,
                    source_stable_id: entry.source_stable_id.map(|id| id.0.0),
                    source_name: entry.source_name.clone(),
                })
                .collect(),
            hidden_zones,
        }
    }

    fn should_redact_for_perspective(
        &self,
        object: &SyncObject,
        perspective: PlayerId,
    ) -> bool {
        let owner = PlayerId::from_index(object.owner);
        match object.zone.as_str() {
            "library" => true,
            "hand" | "outside_game" => owner != perspective,
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
        self.registry = CardRegistry::new();
        self.trigger_queue = TriggerQueue::new();
        self.priority_state = PriorityLoopState::new(checkpoint.players.len());
        self.priority_state.set_auto_choose_single_pip_payment(false);
        self.pregame = None;
        self.match_format = checkpoint.format;
        self.pending_decision = None;
        self.pending_replay_action = None;
        self.pending_action_checkpoint = None;
        self.pending_live_action_root = None;
        self.pending_live_continuation = None;
        self.game_over = None;
        self.runner = None;
        self.runner_awaiting_priority = false;
        self.runner_pending_decision = false;
        self.auto_cleanup_discard = checkpoint.auto_cleanup_discard;
        self.game
            .set_auto_choose_single_object_decisions(checkpoint.auto_choose_single_object_decisions);
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

        let is_redacted_hidden_card =
            object.hidden_card.is_some() && object.name == "Hidden Card";
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
        if object.token {
            restored.compiled_card_text = object.oracle_text.clone();
            restored.base_loyalty = object.loyalty;
            restored.base_defense = object.defense;
        }
        restored.counters = object
            .counters
            .iter()
            .map(|counter| (sync_counter_from_name(&counter.kind), counter.amount))
            .collect();
        restored.attached_to = object
            .attached_to
            .clone()
            .map(attachment_target_from_sync);
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
                self.game.hidden_cards.insert(
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
        self.game.exiled_with_source = checkpoint
            .exiled_with_source
            .iter()
            .map(|(source, linked)| (ObjectId::from_raw(*source), object_ids(linked.clone())))
            .collect();
        self.game.return_exiled_when_source_leaves = checkpoint
            .return_exiled_when_source_leaves
            .iter()
            .map(|id| ObjectId::from_raw(*id))
            .collect();

        self.game.turn = TurnState {
            active_player: PlayerId::from_index(checkpoint.turn.active_player),
            priority_player: checkpoint
                .turn
                .priority_player
                .map(PlayerId::from_index),
            turn_number: checkpoint.turn.turn_number,
            phase: sync_phase_from_name(&checkpoint.turn.phase)?,
            step: checkpoint
                .turn
                .step
                .as_deref()
                .map(sync_step_from_name)
                .transpose()?,
        };

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
            if let Some(player) = object.plotted_by {
                self.game
                    .plotted_cards
                    .insert(id, (PlayerId::from_index(player), object.plotted_turn.unwrap_or(0)));
            }
            if object.damage_marked > 0 {
                self.game.damage_marked.insert(id, object.damage_marked);
            }
            if object.commander {
                self.game.set_commander(id);
            }
            let controller = PlayerId::from_index(object.controller);
            if controller != PlayerId::from_index(object.owner) {
                self.game.set_current_controller(id, controller);
            }
        }

        restore_id_counters(IdCountersSnapshot::from(checkpoint.id_counters));
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
        let checkpoint = self.build_redacted_sync_checkpoint(PlayerId::from_index(perspective_index))?;
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
    fn sync_checkpoint_restores_battlefield_state_for_guest_perspective() {
        let mut host = WasmGame::new();
        host.initialize_empty_match(vec!["Alice".to_string(), "Bob".to_string()], 20, 1);
        let object_id = ObjectId::from_raw(
            host.add_card_to_zone(0, "Ornithopter".to_string(), "battlefield".to_string(), true)
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
    fn public_audit_checkpoint_redacts_hidden_zone_card_identities() {
        let mut host = WasmGame::new();
        host.initialize_empty_match(vec!["Alice".to_string(), "Bob".to_string()], 20, 1);
        host.add_card_to_zone(0, "Ornithopter".to_string(), "battlefield".to_string(), true)
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
            .filter_map(|object| object.identity.as_ref().map(|identity| identity.name.as_str()))
            .collect::<Vec<_>>();
        assert!(public_names.contains(&"Ornithopter"));
        assert!(public_names.contains(&"Forest"));
        assert!(!public_names.contains(&"Lightning Bolt"));
        assert!(!public_names.contains(&"Counterspell"));

        assert!(checkpoint
            .hidden_zones
            .iter()
            .any(|zone| zone.owner == 1 && zone.zone == "library" && zone.count == 1));
        assert!(checkpoint
            .hidden_zones
            .iter()
            .any(|zone| zone.owner == 1 && zone.zone == "hand" && zone.count == 1));
    }

    #[test]
    fn public_audit_checkpoint_uses_stable_public_hidden_commitments() {
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
    fn hidden_card_placeholder_moves_and_reveals_in_place() {
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
        game.game
            .player_mut(bob)
            .expect("Bob should still exist")
            .library
            .reverse();
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
            assert_eq!(info.slot, position as u16);
            assert_eq!(
                info.commitment,
                format!("ziffle:mulligan-deck:{position}")
            );
        }
    }

    #[test]
    fn sync_checkpoint_preserves_hidden_card_placeholders() {
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
    fn hidden_deck_manifest_populates_committed_library_placeholders() {
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
        assert!(bob.library.iter().all(|id| game.game.is_hidden_card_placeholder(*id)));
    }

    #[test]
    fn local_committed_card_exports_opening_metadata() {
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
        assert!(checkpoint
            .objects
            .iter()
            .filter(|object| object.owner == 1 && (object.zone == "hand" || object.zone == "library"))
            .all(|object| object.name == "Hidden Card" && object.hidden_card.is_some()));
        assert!(!checkpoint
            .objects
            .iter()
            .any(|object| object.name == "Lightning Bolt" || object.name == "Counterspell"));

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
