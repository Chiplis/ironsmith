// Public consensus projection and hidden-identity commitments only.
// Recovery uses native savepoints or signed action replay; this has no importer.
use ironsmith::game_state::{ArchenemyVariant, Phase, Step, TurnState};
use ironsmith::object::{AttachmentTarget, Object};
use ironsmith::player::ManaPool;
use ironsmith::types::Subtype;
// Coordinated with artifact17 and signed audit31. Nested counter riders and
// canonical counter text extend the public vocabulary through existing carriers.
// Historical digests retain their bytes; this is never a gameplay importer.
const PUBLIC_AUDIT_VERSION: u32 = 11;
type SyncRestrictedManaUnit = ironsmith_core::RestrictedManaUnit<ironsmith_compiled_artifact::WireEffect>;
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Serialize)]
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


fn sync_restricted_mana(
    units: &[ironsmith::ability::RestrictedManaUnit],
) -> Result<Vec<SyncRestrictedManaUnit>, String> {
    units
        .iter()
        .map(|unit| {
            Ok(SyncRestrictedManaUnit {
                symbol: unit.symbol,
                source: unit.source,
                source_controller: unit.source_controller,
                source_chosen_creature_type: unit.source_chosen_creature_type,
                restrictions: unit
                    .restrictions
                    .iter()
                    .cloned()
                    .map(|restriction| {
                        // Audit the same typed semantic payload used by compiled
                        // cards. This projection cannot restore executable state.
                        restriction.try_map_effects(&mut |effect| {
                            ironsmith_runtime_catalog::artifact_materializer::encode_runtime_effect(effect)
                                .map_err(|error| error.to_string())
                        })
                    })
                    .collect::<Result<_, String>>()?,
            })
        })
        .collect()
}

#[cfg(test)]
mod public_audit_boundary_11_7_24_tests {
    include!("public_audit_boundary_11_7_24_tests.rs");
}

#[cfg(test)]
mod public_audit_boundary_12_8_25_tests {
    include!("public_audit_boundary_12_8_25_tests.rs");
}

#[cfg(test)]
mod public_audit_boundary_13_9_26_tests {
    include!("public_audit_boundary_13_9_26_tests.rs");
}

#[cfg(test)]
mod public_audit_boundary_17_11_31_tests {
    include!("public_audit_boundary_17_11_31_tests.rs");
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct SyncTurn {
    active_player: u8,
    priority_player: Option<u8>,
    turn_number: u32,
    phase: String,
    step: Option<String>,
    /// Seating order rotated onto the seat that took the first turn.
    #[serde(default)]
    turn_order: Vec<u8>,
}


#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
enum SyncAttachmentTarget {
    Object { object: u64 },
    Player { player: u8 },
}


#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct SyncCounter {
    kind: String,
    amount: u32,
    /// Exact identity; display names cannot distinguish named and built-in kinds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    counter_type: Option<ironsmith::CounterType>,
}


#[derive(Debug, Clone, Serialize)]
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


#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct PublicAuditObjectIdentity {
    name: String,
    card_types: Vec<String>,
    subtypes: Vec<String>,
    oracle_text: String,
}


#[derive(Debug, Clone, Serialize)]
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
    numeric_choices: ironsmith::source_numbers::NumberChoicePublicProof,
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
    cloaked: bool,
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


#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct PublicAuditHiddenZone {
    owner: u8,
    zone: String,
    count: usize,
    protocol: String,
    commitment_root: Option<String>,
}


#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PublicAuditCheckpoint {
    version: u32,
    hidden_incarnation_high_water: u64,
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
    /// Latest begun declare-attackers-step evidence retained with combat:
    /// null is absent/uncommitted, [] committed empty, otherwise sorted players.
    /// It is not evidence that the current arbitrary step contains an attack.
    last_attack_declaration_step_players: Option<Vec<u8>>,
    hidden_zones: Vec<PublicAuditHiddenZone>,
    /// SHA-256 (hex) of the canonical JSON of the shared hidden-claim ledger
    /// (obligations, face-down cast claims, claim subjects, library anchor
    /// keys; see `PublicHiddenClaimLedger`). Identical on every peer, so the
    /// checkpoint hash every signed action carries commits to the ledger.
    /// Omitted while the ledger is empty.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    hidden_claim_ledger_digest: Option<String>,
}


#[derive(Debug, Clone, Serialize)]
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


#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct PublicAuditArchenemy {
    variant: String,
    archenemies: Vec<u8>,
    decks: Vec<(u8, usize)>,
    face_up: Vec<u64>,
}


#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct PublicAuditConspiracy {
    cards: Vec<(u8, Vec<u64>)>,
    face_down: Vec<u64>,
}


#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct SyncStackEntry {
    object_id: u64,
    /// The ability's own target id; absent in older checkpoints.
    #[serde(default)]
    ability_id: Option<u64>,
    #[serde(default)]
    ninjutsu_attack_target: Option<SyncGrandMeleeAttackTarget>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    iterated_player: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    iterated_object: Option<u64>,
    controller: u8,
    targets: Vec<SyncTarget>,
    is_ability: bool,
    x_value: Option<u32>,
    source_stable_id: Option<u64>,
    source_name: Option<String>,
}


#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
enum SyncTarget {
    Player { player: u8 },
    Object { object: u64 },
}



#[derive(Debug, Clone, Default, Serialize)]
struct PublicAuditClaimState {
    hidden_identity_obligations: Vec<SyncHiddenIdentityObligation>,
    hidden_face_down_cast_claims: Vec<SyncFaceDownCastClaim>,
    hidden_claim_subjects: Vec<u64>,
    hidden_library_anchors: Vec<SyncHiddenLibraryAnchor>,
}

/// A face-down cast claim `(object, kind)` of a hidden hand card.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct SyncFaceDownCastClaim {
    #[serde(skip_serializing_if = "Option::is_none")]
    object: Option<u64>,
    kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    permission_source: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    blind_exile_origin: Option<SyncBlindExileClaimOrigin>,
}

/// Public evidence only. Explicit field names keep this distinct from the
/// native exact ObjectId authority and from legacy ordinary cast claim IDs.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct SyncBlindExileClaimOrigin {
    card_stable_id: u64,
    incarnation: Option<u64>,
    permission_source_stable_id: Option<u64>,
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
#[derive(Debug, Clone, Default, Serialize)]
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
#[derive(Debug, Clone, Default, Serialize)]
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
}

/// One pending claim of the obligation ledger. The filter travels as JSON
/// text (a stable, self-describing encoding of the compiled filter).
#[derive(Debug, Clone, Serialize)]
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


#[derive(Debug, Clone, Serialize)]
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


fn sync_face_down_kind_fields(
    kind: ironsmith::game_state::FaceDownCastKind,
) -> (String, Option<u64>) {
    (
        kind.as_str().to_string(),
        kind.permission_source().map(|source| source.0),
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


fn public_hidden_claim_ledger(rules: &PublicAuditClaimState) -> PublicHiddenClaimLedger<'_> {
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
fn hidden_claim_ledger_digest(rules: &PublicAuditClaimState) -> Result<Option<String>, String> {
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


#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct SyncFreeForAll {
    seats: Vec<u8>,
    attack: FreeForAllAttackInput,
    range_of_influence: Option<u8>,
}


#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct SyncTeamVsTeam {
    teams: Vec<Vec<u8>>,
    seats: Vec<u8>,
    starting_team: usize,
    starting_player: u8,
}


#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct SyncEmperor {
    teams: Vec<Vec<u8>>,
    seats: Vec<u8>,
    ranges: Vec<u8>,
    starting_team: usize,
    starting_emperor: u8,
}


#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct SyncTwoHeadedGiant {
    teams: Vec<Vec<u8>>,
    seats: Vec<u8>,
    starting_team: usize,
    starting_player: u8,
    starting_life: i32,
    poison_threshold: u32,
}


#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct SyncAlternatingTeams {
    teams: Vec<Vec<u8>>,
    seats: Vec<u8>,
    starting_player: u8,
    attack: FreeForAllAttackInput,
    range_of_influence: Option<u8>,
    deploy_creatures: bool,
}


#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct SyncGrandMelee {
    seats: Vec<u8>,
    starting_player_count: usize,
    focused_marker: u32,
    markers: Vec<SyncGrandMeleeMarker>,
    deferred_extra_turns: Vec<(u8, usize)>,
}


#[derive(Debug, Clone, Serialize)]
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
    #[serde(skip_serializing_if = "Option::is_none")]
    opened_exile_play: Option<SyncOpenedExilePlay>,
    #[serde(skip_serializing_if = "Option::is_none")]
    exile_face_down: Option<SyncExileFaceDownDeclaration>,
}


#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct SyncGrandMeleeCombat {
    last_attack_declaration_step_players: Option<Vec<u8>>,
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


#[derive(Debug, Clone, Serialize)]
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


#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct SyncVanguard {
    cards: Vec<(u8, u64)>,
    hand_modifiers: Vec<(u8, i32)>,
    life_modifiers: Vec<(u8, i32)>,
}


#[derive(Debug, Clone, Default, Serialize)]
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
    #[serde(skip_serializing_if = "Option::is_none")]
    opened_exile_play: Option<SyncOpenedExilePlay>,
    #[serde(skip_serializing_if = "Option::is_none")]
    exile_face_down: Option<SyncExileFaceDownDeclaration>,
}



#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct SyncOpenedExilePlay {
    card_id: u64,
    incarnation: Option<u64>,
    card_stable_id: Option<u64>,
    player: u8,
    permission: GrantSelectionRef,
    permission_source_stable_id: Option<u64>,
    choice_pending: bool,
}

fn sync_opened_exile_play(state: &PriorityLoopState) -> Option<SyncOpenedExilePlay> {
    let opened = state.opened_exile_play.as_ref()?;
    Some(SyncOpenedExilePlay {
        card_id: opened.card_id.0,
        incarnation: opened.incarnation,
        card_stable_id: state.checkpoint.as_ref().and_then(|game| game.object(opened.card_id)).map(|card| card.stable_id.0.0),
        player: opened.player.0,
        permission: GrantSelectionRef { source: opened.permission.source.0, index: opened.permission.index },
        permission_source_stable_id: state.checkpoint.as_ref().and_then(|game| game.object(opened.permission.source)).map(|source| source.stable_id.0.0),
        choice_pending: state.pending_exile_play.is_some(),
    })
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct SyncExileFaceDownDeclaration {
    card_id: u64,
    incarnation: Option<u64>,
    card_stable_id: Option<u64>,
    player: u8,
    permission: GrantSelectionRef,
    permission_source_stable_id: Option<u64>,
    kinds: Vec<SyncExileFaceDownKind>,
    declared_kind: Option<SyncExileFaceDownKind>,
    choice_pending: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct SyncExileFaceDownKind {
    kind: String,
    permission_source: Option<u64>,
    permission_source_stable_id: Option<u64>,
}

fn validate_exile_face_down_public_sources(state: &PriorityLoopState) -> Result<(), String> {
    for declaration in [state.pending_exile_face_down.as_ref(), state.declared_exile_face_down.as_ref()].into_iter().flatten() {
        for kind in declaration.kinds.iter().copied().chain(declaration.declared_kind) {
            if kind.permission_source().is_some_and(|source| !declaration.kind_source_public_ids.contains_key(&source)) {
                return Err("face-down declaration omitted its exact permission source public identity".into());
            }
        }
    }
    Ok(())
}

fn sync_exile_face_down(state: &PriorityLoopState, game: &GameState) -> Option<SyncExileFaceDownDeclaration> {
    let declaration = state.pending_exile_face_down.as_ref().or(state.declared_exile_face_down.as_ref())?;
    let origin_game = state.checkpoint.as_ref().unwrap_or(game);
    let kind = |kind: ironsmith::game_state::FaceDownCastKind| SyncExileFaceDownKind {
        kind: kind.as_str().to_string(),
        permission_source: kind.permission_source().map(|source| source.0),
        permission_source_stable_id: kind.permission_source()
            .and_then(|source| declaration.kind_source_public_ids.get(&source)).map(|id| id.0.0),
    };
    Some(SyncExileFaceDownDeclaration {
        card_id: declaration.card_id.0, incarnation: declaration.incarnation,
        card_stable_id: origin_game.object(declaration.card_id).map(|object| object.stable_id.0.0),
        player: declaration.player.0,
        permission: GrantSelectionRef { source: declaration.permission.source.0, index: declaration.permission.index },
        permission_source_stable_id: origin_game.object(declaration.permission.source).map(|object| object.stable_id.0.0),
        kinds: declaration.kinds.iter().copied().map(&kind).collect(),
        // Accepted kinds remain public through payment rollback. Failed new
        // declarations restore their command savepoint and add no kind lock.
        declared_kind: declaration.declared_kind.map(kind),
        choice_pending: state.pending_exile_face_down.is_some(),
    })
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


fn sync_phase_name(phase: Phase) -> &'static str {
    match phase {
        Phase::Beginning => "beginning",
        Phase::FirstMain => "first_main",
        Phase::Combat => "combat",
        Phase::NextMain => "next_main",
        Phase::Ending => "ending",
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


fn sync_counter_kind(counter: ironsmith::object::CounterType) -> String {
    counter.description().to_string()
}


fn sync_attachment_target(target: AttachmentTarget) -> SyncAttachmentTarget {
    match target {
        AttachmentTarget::Object(object) => SyncAttachmentTarget::Object { object: object.0 },
        AttachmentTarget::Player(player) => SyncAttachmentTarget::Player { player: player.0 },
    }
}


fn sync_target_input(target: Target) -> SyncTarget {
    match target {
        Target::Player(player) => SyncTarget::Player { player: player.0 },
        Target::Object(object) => SyncTarget::Object { object: object.0 },
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


fn sync_stack_entry(entry: &StackEntry) -> SyncStackEntry {
    SyncStackEntry {
        object_id: entry.object_id.0,
        ability_id: entry.ability_id.map(|id| id.0),
        ninjutsu_attack_target: entry
            .ninjutsu_attack_target
            .as_ref()
            .map(sync_attack_target),
        iterated_player: entry.iteration.iterated_player.map(|player| player.0),
        iterated_object: entry.iteration.iterated_object.map(|object| object.0),
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


fn sync_last_attack_declaration_step_players(combat: Option<&ironsmith::combat_state::CombatState>) -> Option<Vec<u8>> {
    combat?.last_attack_declaration_step_players.as_ref()
        .map(|players| players.iter().map(|player| player.0).collect())
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
        last_attack_declaration_step_players: sync_last_attack_declaration_step_players(Some(combat)),
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
                    opened_exile_play: if focused { sync_opened_exile_play(&host.priority_state) }
                        else { lane.and_then(|lane| sync_opened_exile_play(&lane.priority_state)) },
                    exile_face_down: if focused { sync_exile_face_down(&host.priority_state, &host.game) }
                        else { lane.and_then(|lane| sync_exile_face_down(&lane.priority_state, &host.game)) },
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


fn raw_ids(ids: &[ObjectId]) -> Vec<u64> {
    ids.iter().map(|id| id.0).collect()
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
                "incarnation": info.incarnation,
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
            "cloaked": self.game.is_cloaked(id),
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


    fn committed_object_ids(&self) -> Vec<ObjectId> {
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


    fn sync_face_down_cast_claim(&self, object: ObjectId, kind: ironsmith::game_state::FaceDownCastKind) -> Result<SyncFaceDownCastClaim, String> {
        let declaration = std::iter::once(&self.priority_state)
            .chain(self.grand_melee_host_lanes.values().map(|lane| &lane.priority_state))
            .flat_map(|state| [state.pending_exile_face_down.as_ref(), state.declared_exile_face_down.as_ref()])
            .flatten().find(|declaration| declaration.card_id == object);
        if let Some(declaration) = declaration {
            if declaration.declared_kind != Some(kind) {
                return Err("blind exile claim and accepted declaration disagree".into());
            }
            let card = self.game.object(object).ok_or_else(|| "blind exile claim lost its exact original object".to_string())?;
            let permission_source_stable_id = kind.permission_source().map(|source|
                declaration.kind_source_public_ids.get(&source).map(|id| id.0.0)
                    .ok_or_else(|| "blind exile claim lost its captured permission source identity".to_string())).transpose()?;
            return Ok(SyncFaceDownCastClaim {
                object: None, kind: kind.as_str().to_string(), permission_source: None,
                blind_exile_origin: Some(SyncBlindExileClaimOrigin {
                    card_stable_id: card.stable_id.0.0, incarnation: declaration.incarnation,
                    permission_source_stable_id,
                }),
            });
        }
        let (kind, permission_source) = sync_face_down_kind_fields(kind);
        Ok(SyncFaceDownCastClaim { object: Some(object.0), kind, permission_source, blind_exile_origin: None })
    }

    fn hidden_claim_ledger_rules_state(&self) -> Result<PublicAuditClaimState, String> {
        Ok(PublicAuditClaimState {
            hidden_identity_obligations: self.game.hidden_identity_obligations().iter()
                .map(sync_hidden_identity_obligation).collect::<Result<_, _>>()?,
            hidden_face_down_cast_claims: {
                let mut claims = self.game.hidden_face_down_cast_claims().into_iter()
                    .map(|(object, kind)| self.sync_face_down_cast_claim(object, kind)).collect::<Result<Vec<_>, _>>()?;
                // Legacy rows retain their prior raw-ID order. Blind rows use
                // their explicit captured public origin, never local allocation order.
                claims.sort_by_key(|claim| match &claim.blind_exile_origin {
                    Some(origin) => (1, origin.card_stable_id, origin.incarnation),
                    None => (0, claim.object.unwrap_or(0), None),
                });
                claims
            },
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
        })
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
        for state in std::iter::once(&self.priority_state)
            .chain(self.grand_melee_host_lanes.values().map(|lane| &lane.priority_state)) {
            validate_exile_face_down_public_sources(state).map_err(|error| JsValue::from_str(&error))?;
        }
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
                let identity_public = self.public_audit_object_identity_is_public(id);
                let stats_public = identity_public || object.face_down_cast_state.is_some();
                Some((||Ok(PublicAuditObject {
                    id: object.id.0,
                    stable_id: object.stable_id.0.0,
                    owner: object.owner.0,
                    initial_controller: object.initial_controller.0,
                    controller: self.game.controller_of(object).0,
                    zone: sync_zone_name(object.zone).to_string(),
                    identity: self.public_audit_object_identity(id, object),
                    chosen_subtype: self.game.chosen_subtype(id),
                    numeric_choices: ironsmith::source_numbers::public_proof(&self.game,id,identity_public)
                        .map_err(|error|JsValue::from_str(&format!("numeric choice proof unavailable: {error:?}")))?,
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
                    cloaked: self.game.is_cloaked(id),
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
                }))())
            })
            .collect::<Result<Vec<_>,JsValue>>()?;

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
            version: PUBLIC_AUDIT_VERSION,
            last_attack_declaration_step_players: sync_last_attack_declaration_step_players(self.game.combat.as_ref()),
            hidden_incarnation_high_water: self.game.hidden_incarnation_high_water(),
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
                opened_exile_play: sync_opened_exile_play(&self.priority_state),
                exile_face_down: sync_exile_face_down(&self.priority_state, &self.game),
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
                    iterated_player: entry.iteration.iterated_player.map(|player| player.0),
        iterated_object: entry.iteration.iterated_object.map(|object| object.0),
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


    /// Export a redacted checkpoint suitable for peer audit logs.
    #[wasm_bindgen(js_name = exportPublicAuditCheckpoint)]
    pub fn export_public_audit_checkpoint(&self) -> Result<JsValue, JsValue> {
        serde_wasm_bindgen::to_value(&self.try_build_public_audit_checkpoint()?)
            .map_err(|e| JsValue::from_str(&format!("public audit checkpoint encode failed: {e}")))
    }

}

#[cfg(test)]
mod public_audit_tests {
    use super::*;
    use ironsmith::game_state::HiddenCardInfo;

    #[test]
    fn attacked_step_projection_distinguishes_absent_empty_and_sorted_committed_players() {
        let _ids = crate::test_id_counter_guard();
        let mut wasm = WasmGame::new();
        wasm.initialize_empty_match(vec!["A".into(), "B".into(), "C".into()], 20, 1);
        let evidence = |wasm: &WasmGame| serde_json::to_value(wasm.build_public_audit_checkpoint()).unwrap();
        wasm.game.combat = Some(ironsmith::combat_state::CombatState::default());
        let absent = evidence(&wasm);
        assert_eq!(PUBLIC_AUDIT_VERSION, 11);
        assert_eq!(absent["version"], PUBLIC_AUDIT_VERSION);
        assert!(absent.as_object().unwrap().contains_key("lastAttackDeclarationStepPlayers"));
        assert!(absent["lastAttackDeclarationStepPlayers"].is_null());
        let lane_absent = serde_json::to_value(sync_grand_melee_combat(wasm.game.combat.as_ref().unwrap())).unwrap();
        assert!(lane_absent.as_object().unwrap().contains_key("lastAttackDeclarationStepPlayers"));
        assert!(lane_absent["lastAttackDeclarationStepPlayers"].is_null());
        wasm.game.combat.as_mut().unwrap().last_attack_declaration_step_players = Some(Default::default());
        let empty = evidence(&wasm);
        assert_eq!(empty["lastAttackDeclarationStepPlayers"], serde_json::json!([]));
        assert_ne!(absent, empty);
        let lane_empty = serde_json::to_value(sync_grand_melee_combat(wasm.game.combat.as_ref().unwrap())).unwrap();
        assert_eq!(lane_empty["lastAttackDeclarationStepPlayers"], serde_json::json!([]));
        assert_ne!(lane_absent, lane_empty);
        let players = [PlayerId::from_index(2), PlayerId::from_index(0)].into_iter().collect();
        wasm.game.combat.as_mut().unwrap().last_attack_declaration_step_players = Some(players);
        let committed = evidence(&wasm);
        assert_eq!(committed["lastAttackDeclarationStepPlayers"], serde_json::json!([0, 2]));
        assert_ne!(empty, committed);
        let lane = serde_json::to_value(sync_grand_melee_combat(wasm.game.combat.as_ref().unwrap())).unwrap();
        assert_eq!(lane["lastAttackDeclarationStepPlayers"], serde_json::json!([0, 2]));
        assert_ne!(lane_empty, lane);
        assert_eq!(serde_json::to_vec(&wasm.build_public_audit_checkpoint()).unwrap(),
            serde_json::to_vec(&wasm.build_public_audit_checkpoint()).unwrap());
        let saved = wasm.game.clone();
        wasm.game.combat.as_mut().unwrap().last_attack_declaration_step_players = None;
        wasm.game = saved;
        assert_eq!(evidence(&wasm), committed);
    }


    #[test]
    fn public_audit_v7_distinguishes_unset_zero_and_large_source_numbers_and_native_restore() {
        let _id_counter_guard = crate::test_id_counter_guard();
        let mut wasm = WasmGame::new();
        wasm.initialize_empty_match(vec!["Alice".into(), "Bob".into()], 20, 1);
        let id = ObjectId::from_raw(wasm.add_card_to_zone(0, "Ornithopter".into(), "battlefield".into(), true).unwrap());
        let checkpoint = |wasm: &WasmGame| serde_json::to_value(wasm.build_public_audit_checkpoint()).unwrap();
        let unset = checkpoint(&wasm);
        assert_eq!(unset["version"], PUBLIC_AUDIT_VERSION);
        let owner=ironsmith::linked_exile::LinkedExileOwner{host:id,
            pair:ironsmith_core::LinkedExilePair{definition:ironsmith_core::LinkedExileDefinition([81;32]),pair:0},
            acquisition:ironsmith::linked_exile::LinkedExileAcquisition::Printed};
        wasm.game.set_number_for_acquisition(owner.clone(), 0).unwrap();
        let zero = checkpoint(&wasm); assert_ne!(unset, zero);
        let saved = wasm.game.clone();
        wasm.game.set_number_for_acquisition(owner, u32::MAX).unwrap();
        let large = checkpoint(&wasm); assert_ne!(zero, large);
        let object = large["objects"].as_array().unwrap().iter().find(|object| object["id"] == id.0).unwrap();
        assert_eq!(object["numericChoices"]["records"][0]["number"], u32::MAX);
        assert_eq!(object["numericChoices"]["records"][0]["group"], 0);
        wasm.game = saved; assert_eq!(checkpoint(&wasm), zero);
    }

    #[test]
    fn public_audit_v7_retains_exact_manifest_and_cloak_provenance() {
        let _id_counter_guard = crate::test_id_counter_guard();
        let mut wasm = WasmGame::new();
        wasm.initialize_empty_match(vec!["Alice".into(), "Bob".into()], 20, 1);
        let id = ObjectId::from_raw(wasm.add_card_to_zone(
            0, "Ornithopter".into(), "battlefield".into(), true,
        ).unwrap());
        wasm.game.set_face_down(id);
        let baseline = wasm.game.clone();
        let object_evidence = |wasm: &WasmGame| {
            let checkpoint = serde_json::to_value(wasm.build_public_audit_checkpoint()).unwrap();
            assert_eq!(checkpoint["version"], PUBLIC_AUDIT_VERSION);
            checkpoint["objects"].as_array().unwrap().iter()
                .find(|object| object["id"] == id.0).unwrap().clone()
        };
        let ordinary = object_evidence(&wasm);
        assert_eq!(ordinary["manifested"], false);
        assert_eq!(ordinary["cloaked"], false);
        wasm.game.set_manifested(id);
        let manifested = object_evidence(&wasm);
        assert_eq!(manifested["manifested"], true);
        assert_eq!(manifested["cloaked"], false);
        wasm.game = baseline;
        wasm.game.set_cloaked(id);
        let cloaked = object_evidence(&wasm);
        assert_eq!(cloaked["manifested"], false);
        assert_eq!(cloaked["cloaked"], true);
        assert_ne!(manifested, cloaked);
        assert_ne!(ordinary, cloaked);
    }

    #[test]
    fn known_hidden_object_commitment_distinguishes_manifest_from_cloak() {
        let _id_counter_guard = crate::test_id_counter_guard();
        let mut wasm = WasmGame::new();
        wasm.initialize_empty_match(vec!["Alice".into(), "Bob".into()], 20, 1);
        let id = ObjectId::from_raw(wasm.add_card_to_zone(
            0, "Forest".into(), "hand".into(), true,
        ).unwrap());
        assert!(wasm.game.hidden_card_info(id).is_none());
        let baseline = wasm.game.clone();
        let root = |wasm: &WasmGame| {
            wasm.build_public_audit_checkpoint().hidden_zones.into_iter()
                .find(|zone| zone.owner == 0 && zone.zone == "hand")
                .unwrap().commitment_root.unwrap()
        };
        let ordinary = root(&wasm);
        let entry = wasm.public_audit_hidden_zone_entry(0, id);
        assert_eq!(entry["manifested"], false);
        assert_eq!(entry["cloaked"], false);
        wasm.game.set_manifested(id);
        let manifested = root(&wasm);
        wasm.game = baseline;
        wasm.game.set_cloaked(id);
        let cloaked = root(&wasm);
        let entry = wasm.public_audit_hidden_zone_entry(0, id);
        assert_eq!(entry["manifested"], false);
        assert_eq!(entry["cloaked"], true);
        assert_ne!(ordinary, manifested);
        assert_ne!(ordinary, cloaked);
        assert_ne!(manifested, cloaked);
    }

    #[test]
    fn public_audit_preserves_mana_spend_program_semantics_without_identity_graphs() {
        fn unit(life: i32) -> ironsmith::ability::RestrictedManaUnit {
            ironsmith::ability::RestrictedManaUnit {
                symbol: ironsmith::mana::ManaSymbol::Green,
                source: ObjectId::from_raw(17),
                source_controller: Some(PlayerId(0)),
                source_chosen_creature_type: None,
                restrictions: vec![ironsmith_core::ManaUsageRestriction::PaymentTransaction {
                    restriction: Some(ironsmith_core::ManaPaymentPredicate::Any),
                    on_spend: vec![ironsmith_core::ManaSpendPayload {
                        predicate: ironsmith_core::ManaPaymentPredicate::Any,
                        effects: ironsmith_core::ResolutionProgram::from_effects(vec![ironsmith::Effect::gain_life(life)]),
                        choices: vec![],
                    }],
                }],
            }
        }
        let original = unit(2);
        let projected = sync_restricted_mana(&[original.clone()]).expect("typed mana programs are auditable");
        let repeated = sync_restricted_mana(&[original]).unwrap();
        let changed = sync_restricted_mana(&[unit(3)]).unwrap();
        let encoded = serde_json::to_value(projected).unwrap();
        assert_eq!(encoded, serde_json::to_value(repeated).unwrap());
        assert_ne!(encoded, serde_json::to_value(changed).unwrap());
        assert!(encoded.to_string().contains("GainLifeEffect"));
    }

    #[test]
    fn public_audit_preserves_new_protection_programs_in_mana_spend_payloads() {
        use ironsmith::ability::ProtectionFrom;
        use ironsmith::effects::player::GrantNextSpellAbilityEffect;
        use ironsmith::static_abilities::StaticAbility;
        use ironsmith_core::{ObjectFilter, PlayerFilter};

        fn unit(effects: Vec<ironsmith::Effect>) -> ironsmith::ability::RestrictedManaUnit {
            ironsmith::ability::RestrictedManaUnit {
                symbol: ironsmith::mana::ManaSymbol::Green,
                source: ObjectId::from_raw(17),
                source_controller: Some(PlayerId(0)),
                source_chosen_creature_type: None,
                restrictions: vec![ironsmith_core::ManaUsageRestriction::PaymentTransaction {
                    restriction: Some(ironsmith_core::ManaPaymentPredicate::Any),
                    on_spend: vec![ironsmith_core::ManaSpendPayload {
                        predicate: ironsmith_core::ManaPaymentPredicate::Any,
                        effects: ironsmith_core::ResolutionProgram::from_effects(effects),
                        choices: vec![],
                    }],
                }],
            }
        }
        let filter = ObjectFilter::creature().you_control();
        let variants = [
            ProtectionFrom::OwnColors,
            ProtectionFrom::ColorsAmong { filter: filter.clone(), reference_source: None },
            ProtectionFrom::ColorsAmong { filter: filter.clone(), reference_source: Some(ObjectId::from_raw(40)) },
            ProtectionFrom::ColorsAmong { filter: filter.clone(), reference_source: Some(ObjectId::from_raw(41)) },
            ProtectionFrom::ColorsAmongAtResolution(filter),
        ];
        let mut encoded = Vec::new();
        for protection in variants {
            let source = unit(vec![ironsmith::Effect::new(GrantNextSpellAbilityEffect::new(
                PlayerFilter::You,
                ObjectFilter::creature(),
                StaticAbility::protection(protection).into(),
            ))]);
            let first = serde_json::to_value(sync_restricted_mana(&[source.clone()]).unwrap()).unwrap();
            let repeated = serde_json::to_value(sync_restricted_mana(&[source]).unwrap()).unwrap();
            assert_eq!(first, repeated, "projection must retain the same typed program");
            for previous in &encoded {
                assert_ne!(&first, previous, "different color sources and capture semantics remain distinct");
            }
            assert!(first.to_string().contains("GrantNextSpellAbilityEffect"));
            encoded.push(first);
        }
        assert!(encoded[0].to_string().contains("OwnColors"));
        assert!(encoded[1].to_string().contains("ColorsAmong"));
        assert!(encoded[4].to_string().contains("ColorsAmongAtResolution"));

        let source_filter = ObjectFilter::creature();
        let restrictions = [
            ironsmith_core::Restriction::BeTargetedPlayerFrom(PlayerFilter::You, source_filter.clone()),
            ironsmith_core::Restriction::PlayerHexproofFrom(PlayerFilter::You, source_filter),
        ];
        let projected_restrictions: Vec<_> = restrictions.into_iter().map(|restriction| {
            let source = unit(vec![ironsmith::Effect::new(GrantNextSpellAbilityEffect::new(
                PlayerFilter::You,
                ObjectFilter::creature(),
                StaticAbility::from_model(ironsmith_core::StaticAbility::restriction(
                    restriction, "targeting restriction",
                )).into(),
            ))]);
            serde_json::to_value(sync_restricted_mana(&[source]).unwrap()).unwrap()
        }).collect();
        assert_ne!(projected_restrictions[0], projected_restrictions[1],
            "source protection and retained-controller hexproof are distinct public programs");
        assert!(projected_restrictions[1].to_string().contains("PlayerHexproofFrom"));

        #[derive(Debug, Clone)]
        struct UnencodedProgram;
        impl ironsmith::effects::EffectExecutor for UnencodedProgram {
            fn execute(
                &self,
                _game: &mut ironsmith::GameState,
                _ctx: &mut ironsmith::effects::EffectContext,
            ) -> Result<ironsmith::effect::EffectOutcome, ironsmith::effects::ExecutionError> {
                panic!("audit encoding must never execute an opaque mana program");
            }
        }
        let unsupported = unit(vec![ironsmith::Effect::new(UnencodedProgram)]);
        assert!(sync_restricted_mana(&[unsupported]).is_err(),
            "an unencodable on-spend body must not disappear from public evidence");
    }

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
        let (right, right_ids) = build(true);
        assert_eq!(left_ids, right_ids, "same public gameplay identities");
        assert_ne!(left.game.object(left_ids[0]).unwrap().card,
            right.game.object(right_ids[0]).unwrap().card);
        let public = serde_json::to_value(left.build_public_audit_checkpoint()).unwrap();
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
            wasm.priority_state.seed_priority_tracker_for_test(0, 2);
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
                incarnation: Some(0),
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

}
