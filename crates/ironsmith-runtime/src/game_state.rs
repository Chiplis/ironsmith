use crate::effect::RestrictionExt as _;
use crate::filter::ObjectFilterExt as _;
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet, VecDeque};
use std::ops::{Deref, DerefMut, Range};
use std::sync::Arc;

use rand::SeedableRng;
use rand::seq::SliceRandom;
use rand_chacha::ChaCha12Rng;

use crate::FxMap;
use crate::ability::{Ability, AbilityKind, ActivatedAbility};
use crate::alternative_cast::CastingMethod;
use crate::card::{Card, LinkedFaceLayout};
use crate::cards::CardRegistry;
use crate::continuous::{
    CalculatedCharacteristics, ContinuousEffect, ContinuousEffectId, ContinuousEffectManager,
    EffectSourceType, EffectTarget, Modification,
};
use crate::cost::OptionalCostsPaid;
use crate::decision::KeywordPaymentContribution;
use crate::derived_view::DerivedGameView;
use crate::dungeon::ActiveDungeonProgress;
use crate::effect::Until;
use crate::events::{Event, EventKind, KeywordActionKind};
use crate::filter::PlayerFilterExt;
use crate::ids::{CardId, ObjectId, PlayerId, StableId, reset_runtime_id_counters};
use crate::object::{AttachmentTarget, AuraAttachmentFilter, CardSharedHandles, Object};
use crate::player::{ManaPool, Player};
use crate::prevention::PreventionEffectManager;
use crate::provenance::{ProvNodeId, ProvenanceGraph, ProvenanceNodeKind};
use crate::replacement::{ReplacementEffectId, ReplacementEffectKey, ReplacementEffectManager};
use crate::snapshot::ObjectSnapshot;
use crate::static_abilities::{AnthemCountExpression, StaticAbility};
use crate::target::ChooseSpec;
use crate::triggers::TriggerIdentity;
use crate::turn_history::TurnHistory;
use crate::types::{CardType, Subtype};
use crate::zone::Zone;

mod mana_and_permissions;
mod object_state_and_events;
mod restart;
mod turns_and_tracking;
mod zones_and_characteristics;

#[cfg(test)]
mod tests;

fn parse_dredge_amount(text: &str) -> Option<usize> {
    let mut words = text.split_whitespace();
    let head = words.next()?;
    if !head.eq_ignore_ascii_case("dredge") {
        return None;
    }
    words
        .next()?
        .trim_matches(|ch: char| !ch.is_ascii_digit())
        .parse::<usize>()
        .ok()
}

/// Pending replacement effect choice when multiple effects apply to the same event.
///
/// Per Rule 616.1e, when multiple replacement effects at the same priority level
/// could apply to an event, the affected player (or controller of the affected
/// object) must choose which one to apply first.
#[derive(Debug, Clone)]
pub struct PendingReplacementChoice {
    /// The event that replacement effects are trying to modify (new trait-based Event)
    pub event: Event,
    /// IDs of the applicable replacement effects
    pub applicable_effects: Vec<ReplacementEffectId>,
    /// Replacement effects that already affected this event before the choice.
    pub applied_effects: HashSet<ReplacementEffectId>,
    /// Stable replacement identities that already affected this event before the choice.
    pub applied_effect_keys: HashSet<ReplacementEffectKey>,
    /// The player who must choose which effect to apply
    pub player: PlayerId,
}

/// Result of moving an object to the battlefield with ETB replacement processing.
///
/// This captures all the modifications that were applied by replacement effects.
#[derive(Debug, Clone)]
pub struct EntersResult {
    /// The new object ID (zone changes create new IDs per rule 400.7)
    pub new_id: ObjectId,
    /// Whether the permanent entered tapped
    pub enters_tapped: bool,
}

/// Linked exile group metadata for "exile ... until ..." effects.
#[derive(Debug, Clone)]
pub struct LinkedExileGroup {
    /// Stable identities of objects exiled as part of this linked group.
    pub stable_ids: Vec<StableId>,
    /// Zone to return objects to when the delayed condition is met.
    pub return_zone: Zone,
    /// If returning to the battlefield, reset controller to owner.
    pub return_under_owner_control: bool,
}

/// Stored front-face identity for a melded permanent's component card.
#[derive(Debug, Clone)]
pub struct MeldComponentState {
    pub stable_id: StableId,
    pub owner: PlayerId,
    pub name: String,
}

/// Battlefield metadata for a melded permanent.
#[derive(Debug, Clone)]
pub struct MeldedPermanentState {
    pub components: Vec<MeldComponentState>,
}

/// One-shot battlefield transition hints for the UI animation layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiBattlefieldTransitionKind {
    Damaged,
    Destroyed,
    Sacrificed,
    Exiled,
}

/// A UI-only battlefield transition record keyed by stable object identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UiBattlefieldTransition {
    pub stable_id: StableId,
    pub kind: UiBattlefieldTransitionKind,
}

/// A UI-only zone transition record for inspector/navigation labels.
///
/// This is intentionally separate from gameplay zone-change events: gameplay
/// events drive rules/triggers, while this bounded feed gives the frontend the
/// original `from` and `to` zones without having to infer them from snapshots.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiZoneTransition {
    pub id: u64,
    pub old_object_id: ObjectId,
    pub new_object_id: ObjectId,
    pub stable_id: StableId,
    pub owner: PlayerId,
    pub controller: PlayerId,
    pub from: Zone,
    pub to: Zone,
}

/// A UI-only effect event record for the frontend animation layer.
/// Bounded feed, same lifecycle as `ui_zone_transitions`.
#[derive(Debug, Clone, PartialEq)]
pub struct UiEffectEvent {
    pub id: u64,
    pub kind: String,
    pub player: Option<PlayerId>,
    pub other_player: Option<PlayerId>,
    pub stable_ids: Vec<StableId>,
    pub value: Option<i64>,
    pub text: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HiddenCardInfo {
    pub owner: PlayerId,
    pub zone: Zone,
    pub slot: u16,
    pub commitment: String,
    pub public_slot: Option<u16>,
    pub public_commitment: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HiddenInfoOperation {
    HiddenMove {
        owner: PlayerId,
        old_object_id: ObjectId,
        new_object_id: ObjectId,
        from: Zone,
        to: Zone,
        slot: u16,
        commitment: String,
    },
    LibraryShuffle {
        player: PlayerId,
        before_order: Vec<ObjectId>,
        after_order: Vec<ObjectId>,
        random_count_before: u64,
        random_count_after: u64,
    },
    LibraryReorder {
        player: PlayerId,
        before_order: Vec<ObjectId>,
        after_order: Vec<ObjectId>,
        reason: String,
    },
    FairRandom {
        random_count_before: u64,
        random_count_after: u64,
        reason: String,
    },
}

#[derive(Debug, Clone, Default)]
struct MetadataStateStore {
    ui_battlefield_transitions: im::Vector<UiBattlefieldTransition>,
    ui_zone_transitions: im::Vector<UiZoneTransition>,
    next_ui_zone_transition_id: u64,
    ui_effect_events: im::Vector<UiEffectEvent>,
    next_ui_effect_event_id: u64,
    provenance_graph: ProvenanceGraph,
}

/// Battlefield-only flags grouped behind copy-on-write storage for cheap clones.
#[derive(Debug, Clone, Default)]
struct BattlefieldFlags {
    /// Tapped permanents on the battlefield.
    tapped_permanents: HashSet<ObjectId>,
    /// Creatures that have summoning sickness.
    summoning_sick: HashSet<ObjectId>,
    /// Damage marked on creatures (cleared at cleanup step).
    damage_marked: HashMap<ObjectId, u32>,
    /// Creatures that are monstrous (from monstrosity ability).
    monstrous: HashSet<ObjectId>,
    /// Permanents that are suspected.
    suspected: HashSet<ObjectId>,
    /// Mounts that are saddled until end of turn.
    saddled_until_end_of_turn: HashSet<ObjectId>,
    /// Creatures dealt nonzero damage by a source with deathtouch since last SBA check.
    dealt_deathtouch_damage_since_sba: HashSet<ObjectId>,
    /// Permanents whose damage is not removed during cleanup.
    damage_persists: HashSet<ObjectId>,
    /// Regeneration shields on permanents.
    regeneration_shields: HashMap<ObjectId, u32>,
    /// Number of times each permanent successfully regenerated this turn.
    regenerated_this_turn: HashMap<ObjectId, u32>,
    /// Number of permanents sacrificed as a result of this permanent's devour ability.
    devoured_counts: HashMap<ObjectId, u32>,
    /// Cases that have become solved.
    solved_cases: HashSet<ObjectId>,
    /// Creatures that are renowned.
    renowned: HashSet<ObjectId>,
    /// Flipped permanents.
    flipped: HashSet<ObjectId>,
    /// Face-down permanents.
    face_down: HashSet<ObjectId>,
    /// Face-down permanents created via manifest.
    manifested: HashSet<ObjectId>,
    /// Split Room permanents whose linked locked door has been unlocked.
    fully_unlocked_rooms: HashSet<ObjectId>,
    /// Number of times each battlefield permanent has transformed.
    transform_count: HashMap<ObjectId, u64>,
    /// Number of times each battlefield permanent has mutated.
    mutation_count: HashMap<ObjectId, u32>,
    /// Phased-out permanents.
    phased_out: HashSet<ObjectId>,
}

/// Exile/casting permission flags grouped behind copy-on-write storage.
#[derive(Debug, Clone, Default)]
struct CastPermissionFlags {
    /// Cards exiled via Madness.
    madness_exiled: HashSet<ObjectId>,
    /// Cards exiled via Foretell.
    foretold_cards: HashSet<ObjectId>,
    /// Cards exiled after resolving as Adventure spells.
    adventure_exiled: HashSet<ObjectId>,
}

/// Per-object annotation state grouped behind copy-on-write storage.
#[derive(Debug, Clone, Default)]
struct ObjectAnnotationStore {
    /// Last life total noted for a battlefield source object.
    noted_life_totals: HashMap<ObjectId, i32>,
    /// Stickers attached to an object, keyed by stable object identity.
    object_stickers: HashMap<StableId, Vec<StickerMarker>>,
}

/// Commander-format tracking grouped behind copy-on-write storage.
#[derive(Debug, Clone, Default)]
struct CommanderTracking {
    /// Objects designated as commanders.
    commanders: HashSet<ObjectId>,
    /// Number of times each commander has been cast from the command zone.
    commander_casts_from_command_zone: HashMap<ObjectId, u32>,
    /// Commanders whose owner declined the current move-to-command-zone choice.
    declined_command_zone_moves: HashSet<ObjectId>,
    /// Component-card identity for battlefield melded permanents.
    melded_permanents: HashMap<StableId, MeldedPermanentState>,
}

/// Exile and stack-origin tracking grouped behind copy-on-write storage.
#[derive(Debug, Clone, Default)]
struct ExileTracking {
    /// Which players may inspect a face-down card in exile.
    face_down_exile_viewers: HashMap<ObjectId, HashSet<PlayerId>>,
    /// Snapshot of a card just before it moved to the stack for casting.
    cast_origin_snapshots: HashMap<ObjectId, ObjectSnapshot>,
    /// Cards exiled via Plot, keyed by object id -> (player who plotted it, turn plotted).
    plotted_cards: HashMap<ObjectId, (PlayerId, u32)>,
    /// Imprinted cards keyed by source permanent.
    imprinted_cards: HashMap<ObjectId, Vec<ObjectId>>,
    /// Cards exiled by a specific source object ID.
    exiled_with_source: HashMap<ObjectId, Vec<ObjectId>>,
    /// Battlefield object identities put there by an effect of a specific
    /// source. Object IDs intentionally preserve zone-change identity: if the
    /// card later returns independently, it is no longer "the creature put
    /// onto the battlefield with" that source.
    battlefield_put_with_source: HashMap<ObjectId, HashSet<ObjectId>>,
    /// Return zones for cards exiled by a source-leaves duration effect.
    exiled_with_source_return_zones: HashMap<ObjectId, HashMap<ObjectId, Zone>>,
    /// Sources whose linked exiled cards return when the source leaves.
    return_exiled_when_source_leaves: HashSet<ObjectId>,
    /// Linked exile groups keyed by generated runtime ID.
    linked_exile_groups: HashMap<u64, LinkedExileGroup>,
    /// Monotonic ID generator for linked exile groups.
    next_linked_exile_group_id: u64,
}

/// Combat and per-turn transient tracking grouped behind copy-on-write storage.
#[derive(Debug, Clone, Default)]
struct CombatTransientState {
    /// Soulbond pairings (stored bidirectionally: A -> B and B -> A).
    soulbond_pairs: HashMap<ObjectId, ObjectId>,
    /// Attack targets captured while paying Ninjutsu costs.
    ninjutsu_attack_targets: HashMap<ObjectId, Vec<crate::combat_state::AttackTarget>>,
    /// Attack targets captured while paying Sneak costs.
    sneak_attack_targets: HashMap<ObjectId, Vec<crate::combat_state::AttackTarget>>,
    /// Combat-damage-to-player hits processed in the current trigger batch.
    combat_damage_player_batch_hits: Vec<(ObjectId, PlayerId)>,
    /// Players whose inherent speed trigger has already fired this turn.
    speed_increase_triggered_this_turn: HashSet<PlayerId>,
}

/// Miscellaneous checkpoint-heavy state grouped behind copy-on-write storage.
#[derive(Debug, Clone, Default)]
struct AuxiliaryTrackingState {
    /// Current dungeon progress for each player, if any.
    active_dungeons: HashMap<PlayerId, ActiveDungeonProgress>,
    /// Named dungeons each player has completed this game.
    completed_dungeons: HashMap<PlayerId, Vec<String>>,
    /// Active and pending player-control effects.
    player_control_effects: Vec<PlayerControlEffect>,
    /// Player-control effects active only while a resolving instruction is in scope.
    scoped_player_control_effects: Vec<ScopedPlayerControlEffect>,
    /// Timestamp counter for player-control effects.
    player_control_timestamp: u64,
    /// Temporary effects that redirect attacker/blocker choices this turn.
    combat_choice_control_effects: Vec<CombatChoiceControlEffect>,
    /// Timestamp counter for combat-choice control effects.
    combat_choice_control_timestamp: u64,
    /// Highest pregame draft-note number recorded by a player for a named card.
    draft_noted_highest_numbers: HashMap<(PlayerId, String), u32>,
    /// Cryptographic hidden-card slots that have not been opened on this peer.
    hidden_cards: HashMap<ObjectId, HiddenCardInfo>,
}

/// Storage and denormalized zone indexes for live objects in the game.
pub(crate) type ObjectMap = FxMap<ObjectId, Arc<Object>>;

#[derive(Debug, Clone, Default)]
pub struct ObjectStore {
    objects: ObjectMap,
    /// Fast index: stable id -> current object id.
    stable_id_index: FxMap<StableId, ObjectId>,
    /// Game-local cache for linked-face definitions so transform/split/disturb
    /// resolution doesn't depend on the shared runtime custom-card registry.
    linked_face_definitions_by_id: HashMap<crate::ids::CardId, crate::cards::CardDefinition>,
    linked_face_definitions_by_name: HashMap<String, crate::cards::CardDefinition>,
    /// Game-local Arc-backed payload cache for repeated objects from one card definition.
    card_shared: HashMap<CardId, CardSharedHandles>,
    /// Zone indexes (denormalized for efficiency).
    pub battlefield: Vec<ObjectId>,
    pub command_zone: Vec<ObjectId>,
    pub exile: Vec<ObjectId>,
    /// The full set of destination object IDs created by the most recent move
    /// of a given source object.
    zone_change_result_objects: FxMap<ObjectId, Vec<ObjectId>>,
}

impl ObjectStore {
    fn object(&self, id: ObjectId) -> Option<&Object> {
        self.objects.get(&id).map(Arc::as_ref)
    }

    fn object_mut(&mut self, id: ObjectId) -> Option<&mut Object> {
        self.objects.get_mut(&id).map(Arc::make_mut)
    }

    fn objects_map(&self) -> &ObjectMap {
        &self.objects
    }

    fn into_owned_object(object: Arc<Object>) -> Object {
        match Arc::try_unwrap(object) {
            Ok(object) => object,
            Err(shared) => (*shared).clone(),
        }
    }

    fn shared_handles_for_definition(
        &mut self,
        def: &crate::cards::CardDefinition,
    ) -> CardSharedHandles {
        if let Some(handles) = self.card_shared.get(&def.card.id) {
            return handles.clone();
        }
        let handles = CardSharedHandles::from_definition(def);
        self.card_shared.insert(def.card.id, handles.clone());
        handles
    }
}

/// Turn-order, skip/extra-turn, and per-turn history state.
#[derive(Debug, Clone, Default)]
pub struct TurnStore {
    pub turn_order: Vec<PlayerId>,
    /// Extra turns queued up (Time Walk, etc.).
    /// Players take these turns in order after the current turn ends.
    pub extra_turns: Vec<PlayerId>,
    /// Additional phases inserted after the current phase.
    /// These are consumed before the normal turn sequence advances.
    pub additional_phases: Vec<Phase>,
    /// Number of combat phases that have started during the current turn.
    pub combat_phases_started_this_turn: u32,
    /// Normal phase to resume after inserted additional phases finish.
    pub additional_phase_continuation: Option<Phase>,
    /// Players who will skip their next turn.
    /// Checked and cleared when a player would start their turn.
    pub skip_next_turn: HashSet<PlayerId>,
    /// Players who will skip their next draw step.
    /// Checked and cleared when a player would draw in draw step.
    pub skip_next_draw_step: HashSet<PlayerId>,
    /// The active player whose draw step is currently being tracked for draw-count-sensitive triggers.
    pub tracked_draw_step_player: Option<PlayerId>,
    /// Cards the tracked player has already drawn in the current draw step.
    pub cards_drawn_this_draw_step: u32,
    /// Players who will skip all combat phases on their next turn.
    /// Checked and cleared when entering combat phase.
    pub skip_next_combat_phases: HashSet<PlayerId>,
    /// Players who will skip each remaining combat phase this turn.
    /// Cleared when the turn advances.
    pub skip_current_turn_combat_phases: HashSet<PlayerId>,
    /// Players who will skip each remaining main phase this turn.
    /// Cleared when the turn advances.
    pub skip_current_turn_main_phases: HashSet<PlayerId>,
    /// Unified owner for per-turn event and action history.
    pub turn_history: TurnHistory,
    /// Hand sizes captured as the current turn began, before the untap step.
    pub hand_sizes_at_turn_start: HashMap<PlayerId, usize>,
    /// Total number of spells cast during the immediately previous turn.
    /// Updated when turn advances.
    pub spells_cast_last_turn_total: u32,
    /// Last-known snapshots for objects that entered the battlefield during the immediately
    /// previous turn.
    pub entered_battlefield_last_turn: Vec<ObjectSnapshot>,
    /// Static or temporary grant sources whose once-per-turn cast permission was used.
    pub grant_cast_uses_this_turn: HashSet<(PlayerId, ObjectId)>,
    /// Exhaust activated abilities that have been activated by this object instance.
    pub exhaust_abilities_activated: HashSet<(ObjectId, usize)>,
    /// Explicit combat damage assignments keyed by attacker then damage recipient.
    pub combat_damage_assignments: HashMap<ObjectId, HashMap<ObjectId, u32>>,
    /// Objects that assign no combat damage for the rest of the current turn.
    pub no_combat_damage_this_turn: HashSet<ObjectId>,
    /// Objects that assign no combat damage for the rest of the current combat.
    pub no_combat_damage_this_combat: HashSet<ObjectId>,
}

/// Runtime effect managers, queued trigger state, and temporary effect registries.
#[derive(Debug, Clone)]
pub struct EffectStore {
    pub continuous_effects: ContinuousEffectManager,
    pub replacement_effects: ReplacementEffectManager,
    pub prevention_effects: PreventionEffectManager,
    /// Tracker for "can't" effects (Rule 614.17).
    /// These are checked BEFORE events happen, not as replacements.
    pub cant_effects: CantEffectTracker,
    /// Tracker for "spend mana as though it were mana of any color" effects.
    pub mana_spend_effects: ManaSpendEffectTracker,
    pub delayed_triggers: Vec<crate::triggers::DelayedTrigger>,
    pub pending_trigger_events: Vec<crate::triggers::TriggerEvent>,
    pub active_state_trigger_conditions: HashSet<crate::triggers::ActiveStateTriggerKey>,
    /// Pending replacement effect choice when multiple effects could apply.
    /// When set, advance_priority returns a ChooseReplacementEffect decision
    /// before continuing with normal game flow.
    pub pending_replacement_choice: Option<PendingReplacementChoice>,
    /// Registry for tracking granted alternative casts and abilities.
    pub grant_registry: crate::grant_registry::GrantRegistry,
    /// Monotonic per-library revision for one-shot "while this remains on top" effects.
    pub library_top_revisions: HashMap<PlayerId, u64>,
    /// Temporary mana abilities granted to players (e.g., Channel), expiring at end of turn.
    pub granted_mana_abilities: Vec<GrantedManaAbility>,
    /// Temporary spell-cost reductions waiting for the next matching spell this turn.
    pub temporary_spell_cost_reductions: Vec<TemporarySpellCostReductionEffectInstance>,
    /// Temporary spell-ability grants waiting for the next matching spell this turn.
    pub temporary_spell_ability_grants: Vec<TemporarySpellAbilityGrantEffectInstance>,
    /// Active restriction effects (spell/ability-based "can't" effects).
    pub restriction_effects: Vec<RestrictionEffectInstance>,
    /// Active goad effects (a creature attacks each combat and attacks a player
    /// other than the goader if able).
    pub goad_effects: Vec<GoadEffectInstance>,
}

impl Default for EffectStore {
    fn default() -> Self {
        Self {
            continuous_effects: ContinuousEffectManager::new(),
            replacement_effects: ReplacementEffectManager::new(),
            prevention_effects: PreventionEffectManager::new(),
            cant_effects: CantEffectTracker::new(),
            mana_spend_effects: ManaSpendEffectTracker::new(),
            delayed_triggers: Vec::new(),
            pending_trigger_events: Vec::new(),
            active_state_trigger_conditions: HashSet::new(),
            pending_replacement_choice: None,
            grant_registry: crate::grant_registry::GrantRegistry::new(),
            library_top_revisions: HashMap::new(),
            granted_mana_abilities: Vec::new(),
            temporary_spell_cost_reductions: Vec::new(),
            temporary_spell_ability_grants: Vec::new(),
            restriction_effects: Vec::new(),
            goad_effects: Vec::new(),
        }
    }
}

/// Persisted chosen values and modal selections keyed by source object.
#[derive(Debug, Clone, Default)]
pub struct ChoiceStore {
    /// Tracks modal choices that were already selected for an activated ability.
    /// Key is (source ObjectId, ability index), value is the set of chosen mode indices.
    pub chosen_modes_by_ability: HashMap<(ObjectId, usize), HashSet<usize>>,
    /// Chosen colors for permanents ("as this enters, choose a color").
    pub chosen_colors: HashMap<ObjectId, crate::color::Color>,
    /// Chosen basic land types for permanents ("as this Aura enters, choose a basic land type").
    pub chosen_basic_land_types: HashMap<ObjectId, crate::types::Subtype>,
    /// Chosen land types for permanents ("as this enters, choose a land type").
    pub chosen_land_types: HashMap<ObjectId, crate::types::Subtype>,
    /// Chosen creature types for permanents ("as this enters, choose a creature type").
    pub chosen_creature_types: HashMap<ObjectId, crate::types::Subtype>,
    /// Chosen card types for spells and abilities that ask a player to choose a card type.
    pub chosen_card_types: HashMap<ObjectId, crate::types::CardType>,
    /// Chosen players for permanents ("as this enters, choose a player").
    pub chosen_players: HashMap<ObjectId, PlayerId>,
    /// Chosen named options for permanents ("as this enters, choose A or B").
    pub chosen_named_options: HashMap<ObjectId, String>,
}

#[derive(Debug, Clone)]
struct TranscriptLibraryShuffleOrder {
    player: PlayerId,
    before_order: Vec<ObjectId>,
    after_order: Vec<ObjectId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PaymentRestrictionPresenceCache {
    mutation_revision: u64,
    effect_revision: u64,
    turn_number: u32,
    active_player: PlayerId,
    phase: Phase,
    step: Option<Step>,
    may_have_restriction: bool,
}

#[derive(Debug, Clone)]
struct EnterAsCopySourceCache {
    mutation_revision: u64,
    effect_revision: u64,
    zone_revision: u64,
    continuous_context_revision: u64,
    turn_number: u32,
    active_player: PlayerId,
    phase: Phase,
    step: Option<Step>,
    /// `None` means a continuous ability modification can change whether the
    /// ability exists, so callers must use layered characteristics. `Some`
    /// contains the exact active printed/level/temporary abilities that can be
    /// inspected without entering the layer system.
    sparse_candidates: Option<Arc<Vec<(ObjectId, StaticAbility)>>>,
}

#[derive(Debug)]
struct RuntimeCacheState {
    random_state: Cell<u64>,
    irreversible_random_count: Cell<u64>,
    forced_die_rolls: RefCell<VecDeque<u32>>,
    transcript_random_seeds: RefCell<VecDeque<u64>>,
    transcript_library_shuffle_orders: RefCell<VecDeque<TranscriptLibraryShuffleOrder>>,
    hidden_info_audit_log: RefCell<im::Vector<HiddenInfoOperation>>,
    continuous_state_dirty: Cell<bool>,
    continuous_state_revision: Cell<u64>,
    continuous_state_turn_number: Cell<u32>,
    continuous_state_active_player: Cell<PlayerId>,
    continuous_state_phase: Cell<Phase>,
    continuous_state_step: Cell<Option<Step>>,
    /// Monotonic generation for state changes that can alter calculated
    /// characteristics without changing the object, zone, or effect lists.
    /// Combat declarations are the important example: a conditional ability
    /// grant can turn on when its source starts attacking or blocking.
    continuous_context_revision: Cell<u64>,
    /// Memoized "any effect is turn-context-sensitive" keyed by effects
    /// revision — the classification walks every effect's filter recursively
    /// and would otherwise run on every cache-validity check.
    turn_sensitivity: Cell<Option<(u64, bool)>>,
    effects_snapshot: RefCell<Option<(u64, Arc<Vec<ContinuousEffect>>)>>,
    controller_cache: RefCell<Option<ControllerCache>>,
    payment_restriction_presence: Cell<Option<PaymentRestrictionPresenceCache>>,
    enter_as_copy_sources: RefCell<Option<EnterAsCopySourceCache>>,
    static_effects_cache: RefCell<crate::static_ability_processor::StaticEffectsCache>,
    trigger_registry: RefCell<Option<crate::triggers::check::TriggerRegistry>>,
    object_snapshot_cache: RefCell<ObjectSnapshotCache>,
    characteristics_cache: CharacteristicsCache,
    work_counters: WorkCounters,
}

impl Clone for RuntimeCacheState {
    fn clone(&self) -> Self {
        Self {
            random_state: Cell::new(self.random_state.get()),
            irreversible_random_count: Cell::new(self.irreversible_random_count.get()),
            forced_die_rolls: RefCell::new(self.forced_die_rolls.borrow().clone()),
            transcript_random_seeds: RefCell::new(self.transcript_random_seeds.borrow().clone()),
            transcript_library_shuffle_orders: RefCell::new(
                self.transcript_library_shuffle_orders.borrow().clone(),
            ),
            hidden_info_audit_log: RefCell::new(self.hidden_info_audit_log.borrow().clone()),
            continuous_state_dirty: Cell::new(self.continuous_state_dirty.get()),
            continuous_state_revision: Cell::new(self.continuous_state_revision.get()),
            continuous_state_turn_number: Cell::new(self.continuous_state_turn_number.get()),
            turn_sensitivity: Cell::new(self.turn_sensitivity.get()),
            continuous_state_active_player: Cell::new(self.continuous_state_active_player.get()),
            continuous_state_phase: Cell::new(self.continuous_state_phase.get()),
            continuous_state_step: Cell::new(self.continuous_state_step.get()),
            continuous_context_revision: Cell::new(self.continuous_context_revision.get()),
            // Preserve the revision/epoch-keyed caches: checkpoint restores
            // and hypothetical clones start warm instead of re-deriving the
            // whole board. All keys (revisions, epochs) clone with the state,
            // and payloads are Arcs, so this is spine copies + refcount bumps.
            effects_snapshot: RefCell::new(self.effects_snapshot.borrow().clone()),
            controller_cache: RefCell::new(self.controller_cache.borrow().clone()),
            payment_restriction_presence: Cell::new(self.payment_restriction_presence.get()),
            enter_as_copy_sources: RefCell::new(self.enter_as_copy_sources.borrow().clone()),
            static_effects_cache: RefCell::new(self.static_effects_cache.borrow().clone()),
            trigger_registry: RefCell::new(self.trigger_registry.borrow().clone()),
            object_snapshot_cache: RefCell::new(self.object_snapshot_cache.borrow().clone()),
            characteristics_cache: CharacteristicsCache::cloned_from(&self.characteristics_cache),
            work_counters: WorkCounters::default(),
        }
    }
}

impl RuntimeCacheState {
    fn new(active_player: PlayerId) -> Self {
        Self {
            random_state: Cell::new(GameState::normalize_random_seed(0)),
            irreversible_random_count: Cell::new(0),
            forced_die_rolls: RefCell::new(VecDeque::new()),
            transcript_random_seeds: RefCell::new(VecDeque::new()),
            transcript_library_shuffle_orders: RefCell::new(VecDeque::new()),
            hidden_info_audit_log: RefCell::new(im::Vector::new()),
            continuous_state_dirty: Cell::new(true),
            continuous_state_revision: Cell::new(0),
            continuous_state_turn_number: Cell::new(1),
            turn_sensitivity: Cell::new(None),
            continuous_state_active_player: Cell::new(active_player),
            continuous_state_phase: Cell::new(Phase::Beginning),
            continuous_state_step: Cell::new(Some(Step::Untap)),
            continuous_context_revision: Cell::new(0),
            effects_snapshot: RefCell::new(None),
            controller_cache: RefCell::new(None),
            payment_restriction_presence: Cell::new(None),
            enter_as_copy_sources: RefCell::new(None),
            static_effects_cache: RefCell::new(
                crate::static_ability_processor::StaticEffectsCache::default(),
            ),
            trigger_registry: RefCell::new(None),
            object_snapshot_cache: RefCell::new(ObjectSnapshotCache::default()),
            characteristics_cache: CharacteristicsCache::default(),
            work_counters: WorkCounters::default(),
        }
    }
}

/// LKI snapshot memo scoped to the current (mutation, effect) revision pair.
/// Lookups only ever use the current revisions, so entries from older
/// revisions are unreachable; scoping keeps the map bounded by object count
/// instead of growing for the whole session.
#[derive(Debug, Default, Clone)]
struct ObjectSnapshotCache {
    mutation_revision: u64,
    effect_revision: u64,
    entries: FxMap<ObjectId, Arc<ObjectSnapshot>>,
}

#[derive(Debug, Default)]
struct CharacteristicsCache {
    epoch: Cell<u64>,
    effect_revision: Cell<u64>,
    object_revisions: RefCell<FxMap<ObjectId, u64>>,
    entries: RefCell<FxMap<ObjectId, CharacteristicsCacheEntry>>,
}

#[derive(Debug, Clone)]
struct CharacteristicsCacheEntry {
    epoch: u64,
    effect_revision: u64,
    self_rev: u64,
    chars: Option<Arc<CalculatedCharacteristics>>,
}

impl CharacteristicsCache {
    /// Full preserving clone: entries are revision/epoch-keyed pure memos and
    /// the keys are cloned alongside, so a cloned state's cache stays valid.
    /// Entry payloads are `Arc`s, so this is a map-spine copy.
    fn cloned_from(other: &Self) -> Self {
        Self {
            epoch: Cell::new(other.epoch.get()),
            effect_revision: Cell::new(other.effect_revision.get()),
            object_revisions: RefCell::new(other.object_revisions.borrow().clone()),
            entries: RefCell::new(other.entries.borrow().clone()),
        }
    }

    fn bump_epoch(&self) {
        self.epoch.set(self.epoch.get().saturating_add(1));
        self.entries.borrow_mut().clear();
        // Entries are cleared, so per-object revisions restart consistently;
        // this prunes ids of objects that no longer exist (zone changes mint
        // new ids), keeping the map bounded by live objects.
        self.object_revisions.borrow_mut().clear();
    }

    fn bump_object_revision(&self, id: ObjectId, revision: u64) {
        self.object_revisions.borrow_mut().insert(id, revision);
    }

    fn prepare_for_effect_revision(&self, effect_revision: u64) {
        if self.effect_revision.get() == effect_revision {
            return;
        }
        self.entries.borrow_mut().clear();
        self.effect_revision.set(effect_revision);
        self.epoch.set(self.epoch.get().saturating_add(1));
    }

    fn object_revision(&self, id: ObjectId) -> u64 {
        self.object_revisions
            .borrow()
            .get(&id)
            .copied()
            .unwrap_or(0)
    }

    fn get(
        &self,
        id: ObjectId,
        effect_revision: u64,
    ) -> Option<Option<Arc<CalculatedCharacteristics>>> {
        self.prepare_for_effect_revision(effect_revision);
        let entry = self.entries.borrow().get(&id).cloned()?;
        if entry.epoch == self.epoch.get()
            && entry.effect_revision == effect_revision
            && entry.self_rev == self.object_revision(id)
        {
            Some(entry.chars)
        } else {
            None
        }
    }

    fn contains_valid_entry(&self, id: ObjectId, effect_revision: u64) -> bool {
        self.get(id, effect_revision).is_some()
    }

    fn insert(
        &self,
        id: ObjectId,
        effect_revision: u64,
        chars: Option<CalculatedCharacteristics>,
    ) -> Option<Arc<CalculatedCharacteristics>> {
        let chars = chars.map(Arc::new);
        self.insert_arc(id, effect_revision, chars)
    }

    fn insert_arc(
        &self,
        id: ObjectId,
        effect_revision: u64,
        chars: Option<Arc<CalculatedCharacteristics>>,
    ) -> Option<Arc<CalculatedCharacteristics>> {
        self.prepare_for_effect_revision(effect_revision);
        self.entries.borrow_mut().insert(
            id,
            CharacteristicsCacheEntry {
                epoch: self.epoch.get(),
                effect_revision,
                self_rev: self.object_revision(id),
                chars,
            },
        );
        self.entries
            .borrow()
            .get(&id)
            .and_then(|entry| entry.chars.clone())
    }
}

#[derive(Debug, Clone)]
struct ControllerCache {
    revision: u64,
    turn_number: u32,
    active_player: PlayerId,
    phase: Phase,
    step: Option<Step>,
    change_effects: Arc<Vec<ContinuousEffect>>,
    resolved: RefCell<FxMap<ObjectId, PlayerId>>,
}

impl ControllerCache {
    fn matches_state(&self, game: &GameState) -> bool {
        self.revision == game.effect_store.continuous_effects.revision()
            && self.turn_number == game.turn.turn_number
            && self.active_player == game.turn.active_player
            && self.phase == game.turn.phase
            && self.step == game.turn.step
    }
}

#[derive(Debug, Default)]
struct WorkCounters {
    characteristics_full_recomputes: Cell<u64>,
    characteristics_cache_hits: Cell<u64>,
    static_ability_regens: Cell<u64>,
    effects_considered: Cell<u64>,
    objects_scanned_in_sba: Cell<u64>,
    derived_view_rebuilds: Cell<u64>,
    dependency_sorts: Cell<u64>,
    dependency_pairs_probed: Cell<u64>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[cfg_attr(
    feature = "serialization",
    derive(serde::Serialize, serde::Deserialize)
)]
pub struct WorkCounterSnapshot {
    pub characteristics_full_recomputes: u64,
    pub characteristics_cache_hits: u64,
    pub static_ability_regens: u64,
    pub effects_considered: u64,
    pub objects_scanned_in_sba: u64,
    pub derived_view_rebuilds: u64,
    pub dependency_sorts: u64,
    pub dependency_pairs_probed: u64,
}

impl WorkCounters {
    fn snapshot(&self) -> WorkCounterSnapshot {
        WorkCounterSnapshot {
            characteristics_full_recomputes: self.characteristics_full_recomputes.get(),
            characteristics_cache_hits: self.characteristics_cache_hits.get(),
            static_ability_regens: self.static_ability_regens.get(),
            effects_considered: self.effects_considered.get(),
            objects_scanned_in_sba: self.objects_scanned_in_sba.get(),
            derived_view_rebuilds: self.derived_view_rebuilds.get(),
            dependency_sorts: self.dependency_sorts.get(),
            dependency_pairs_probed: self.dependency_pairs_probed.get(),
        }
    }

    pub(crate) fn bump_dependency_sorts(&self) {
        self.dependency_sorts
            .set(self.dependency_sorts.get().saturating_add(1));
    }

    pub(crate) fn bump_dependency_pairs_probed(&self) {
        self.dependency_pairs_probed
            .set(self.dependency_pairs_probed.get().saturating_add(1));
    }

    fn bump_characteristics_full_recomputes(&self) {
        self.characteristics_full_recomputes
            .set(self.characteristics_full_recomputes.get().saturating_add(1));
    }

    fn bump_characteristics_cache_hits(&self) {
        self.characteristics_cache_hits
            .set(self.characteristics_cache_hits.get().saturating_add(1));
    }

    fn bump_static_ability_regens(&self) {
        self.static_ability_regens
            .set(self.static_ability_regens.get().saturating_add(1));
    }

    fn add_effects_considered(&self, count: u64) {
        self.effects_considered
            .set(self.effects_considered.get().saturating_add(count));
    }

    fn add_objects_scanned_in_sba(&self, count: u64) {
        self.objects_scanned_in_sba
            .set(self.objects_scanned_in_sba.get().saturating_add(count));
    }

    fn bump_derived_view_rebuilds(&self) {
        self.derived_view_rebuilds
            .set(self.derived_view_rebuilds.get().saturating_add(1));
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ZoneRevisionSnapshot {
    pub battlefield: u64,
    pub command: u64,
    pub exile: u64,
    pub library: u64,
    pub hand: u64,
    pub graveyard: u64,
    pub outside_game: u64,
    pub stack: u64,
    pub all: u64,
}

/// Key type for extensible per-turn counters.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TurnCounterKey {
    /// Count by trigger event kind.
    EventKind(EventKind),
    /// Count by structural trigger identity.
    TriggerIdentity(TriggerIdentity),
    /// Arbitrary named counters (cards drawn, ETBs, etc.).
    Named(String),
}

/// Generic per-turn counter tracker.
#[derive(Debug, Clone, Default)]
pub struct TurnCounterTracker {
    counters: HashMap<TurnCounterKey, u32>,
}

fn activated_ability_turn_counter_name(source: ObjectId, ability_index: usize) -> String {
    format!("activated_ability:{}:{}", source.0, ability_index)
}

fn exhaust_ability_turn_counter_name(player: PlayerId) -> String {
    format!("exhaust_ability:{}", player.0)
}

fn activated_ability_resolution_turn_counter_name(
    source: ObjectId,
    ability_index: usize,
) -> String {
    format!("activated_ability_resolved:{}:{}", source.0, ability_index)
}

fn triggered_ability_resolution_turn_counter_name(
    source: ObjectId,
    trigger_id: TriggerIdentity,
) -> String {
    format!("triggered_ability_resolved:{}:{}", source.0, trigger_id.0)
}

impl TurnCounterTracker {
    pub fn increment(&mut self, key: TurnCounterKey) {
        *self.counters.entry(key).or_insert(0) += 1;
    }

    pub fn increment_event_kind(&mut self, event_kind: EventKind) {
        self.increment(TurnCounterKey::EventKind(event_kind));
    }

    pub fn increment_trigger_identity(&mut self, trigger_id: TriggerIdentity) {
        self.increment(TurnCounterKey::TriggerIdentity(trigger_id));
    }

    pub fn increment_named(&mut self, name: impl Into<String>) {
        self.increment(TurnCounterKey::Named(name.into()));
    }

    pub fn get(&self, key: &TurnCounterKey) -> u32 {
        self.counters.get(key).copied().unwrap_or(0)
    }

    pub fn clear(&mut self) {
        self.counters.clear();
    }

    pub fn snapshot(&self) -> Vec<(TurnCounterKey, u32)> {
        self.counters
            .iter()
            .map(|(key, count)| (key.clone(), *count))
            .collect()
    }
}

// =============================================================================
// "Can't" Effect Tracking (Rule 614.17)
// =============================================================================
//
// "Can't" effects are NOT replacement effects. They are prohibitions that must
// be checked BEFORE attempting an action or event. Per Rule 614.17a, events
// that "can't" happen simply don't happen.
//
// Examples:
// - "You can't gain life" (Sulfuric Vortex)
// - "Players can't search libraries" (Stranglehold)
// - "This creature can't attack" (Pacifism)
// - "That creature can't block" (Goblin War Drums)
// - "Damage can't be prevented" (Leyline of Punishment)
// - "This permanent can't be destroyed" (Indestructible)

/// A per-player cast prohibition and the source object needed to evaluate
/// source-dependent spell filters such as "of the chosen type".
#[derive(Debug, Clone, PartialEq)]
pub struct CastRestrictionFilter {
    pub filter: crate::target::ObjectFilter,
    pub source: Option<ObjectId>,
}

/// Tracks active "can't" effects in the game.
///
/// Per Rule 614.17, "can't" effects are not replacement effects - they are
/// prohibitions that prevent events from happening at all. They must be
/// checked BEFORE attempting an action or event.
#[derive(Debug, Clone, Default)]
pub struct CantEffectTracker {
    /// Players who can't gain life.
    /// Example: Sulfuric Vortex, Erebos, God of the Dead
    pub cant_gain_life: HashSet<PlayerId>,

    /// Players who can't search libraries.
    /// Example: Stranglehold, Aven Mindcensor (partial)
    pub cant_search: HashSet<PlayerId>,

    /// Creatures that can't attack.
    /// Example: Pacifism, Propaganda (if unpaid), Maze of Ith
    pub cant_attack: HashSet<ObjectId>,

    /// Creature -> defending players this creature can't attack or attack planeswalkers of.
    /// Example: "Creatures that player controls can't attack you or planeswalkers you control."
    pub cant_attack_defenders: HashMap<ObjectId, HashSet<PlayerId>>,

    /// Creatures that can't attack alone.
    /// Example: "This creature can't attack alone."
    pub cant_attack_alone: HashSet<ObjectId>,

    /// Creatures that can't block.
    /// Example: Goblin War Drums, Madcap Skills
    pub cant_block: HashSet<ObjectId>,

    /// Blocker -> attackers this blocker can't block this turn.
    /// Example: "Target creature can't block this creature this turn."
    pub cant_block_specific_attackers: HashMap<ObjectId, HashSet<ObjectId>>,

    /// Blocker -> attackers this blocker must block this turn if able.
    /// Example: "Target creature blocks this creature this turn if able."
    pub must_block_specific_attackers: HashMap<ObjectId, HashSet<ObjectId>>,

    /// Attackers that must be blocked this turn if able.
    /// Example: "Target creature must be blocked this turn if able."
    pub must_be_blocked: HashSet<ObjectId>,

    /// Creatures that can't block alone.
    /// Example: "This creature can't block alone."
    pub cant_block_alone: HashSet<ObjectId>,

    /// Permanents that can't untap during their controller's untap step.
    /// Example: "It doesn't untap during its controller's untap step"
    pub cant_untap: HashSet<ObjectId>,

    /// Permanents that can't be destroyed (indestructible via effect, not ability).
    /// Note: Intrinsic indestructible keyword is checked separately on the object.
    pub cant_be_destroyed: HashSet<ObjectId>,

    /// Permanents that can't be regenerated.
    /// Example: "Target creature can't be regenerated this turn."
    pub cant_be_regenerated: HashSet<ObjectId>,

    /// Permanents that can't be sacrificed.
    /// Example: Sigarda, Host of Herons (for creatures you control)
    pub cant_be_sacrificed: HashSet<ObjectId>,

    /// Per-player spell filters that cannot be cast.
    ///
    /// Examples:
    /// - default filter => "can't cast spells"
    /// - creature filter => "can't cast creature spells"
    pub cant_cast_filters: HashMap<PlayerId, Vec<CastRestrictionFilter>>,

    /// Players who can cast spells only any time they could cast a sorcery.
    pub cast_spells_only_as_sorcery: HashSet<PlayerId>,

    /// Players who can't activate non-mana abilities.
    /// Example: Split second while a split-second spell is on the stack.
    pub cant_activate_non_mana_abilities: HashSet<PlayerId>,

    /// Permanents whose activated abilities can't be activated (including mana abilities).
    /// Example: Collector Ouphe ("Activated abilities of artifacts can't be activated.")
    pub cant_activate_abilities_of: HashSet<ObjectId>,

    /// Permanents whose activated abilities with {T} in their costs can't be activated.
    pub cant_activate_tap_abilities_of: HashSet<ObjectId>,

    /// Permanents whose non-mana activated abilities can't be activated.
    /// Example: Damping Matrix ("... can't be activated unless they're mana abilities.")
    pub cant_activate_non_mana_abilities_of: HashSet<ObjectId>,

    /// Per-player "can't cast more than one matching spell each turn" restrictions.
    ///
    /// Each filter applies to both:
    /// - the spell being cast now, and
    /// - spells this player has already cast this turn.
    ///
    /// This keeps cast-limit restrictions generic (nonartifact, non-Phyrexian, etc.)
    /// without hard-coding one tracker set per variant.
    pub cant_cast_limit_filters: HashMap<PlayerId, Vec<crate::target::ObjectFilter>>,

    /// Players who can't draw cards.
    /// Example: Notion Thief redirecting draws
    pub cant_draw: HashSet<PlayerId>,

    /// Players who can't draw extra cards (more than one per turn).
    /// Maps: restricted player -> restricting player (e.g., opponent of Narset controller)
    /// Example: Narset, Parter of Veils ("Your opponents can't draw more than one card each turn")
    pub cant_draw_extra_cards: HashSet<PlayerId>,

    /// Players who can't get poison counters.
    pub cant_get_poison_counters: HashSet<PlayerId>,

    /// Creatures that can't be blocked.
    /// Example: Whispersilk Cloak, Invisible Stalker
    pub cant_be_blocked: HashSet<ObjectId>,

    /// Permanents that can't have counters placed on them.
    /// Example: Melira, Sylvok Outcast (for -1/-1 counters on creatures you control)
    /// Note: This is actually a replacement effect in Melira's case, but some
    /// effects truly prevent counters.
    pub cant_have_counters_placed: HashSet<ObjectId>,

    /// Whether damage prevention is globally disabled.
    /// Example: Leyline of Punishment, Everlasting Torment
    pub damage_cant_be_prevented: bool,

    /// Players whose life total can't change.
    /// Example: Platinum Emperion
    pub life_total_cant_change: HashSet<PlayerId>,

    /// Players who can't lose life.
    pub cant_lose_life: HashSet<PlayerId>,

    /// Players whose damage dealt to them does not cause life loss.
    pub damage_cant_cause_life_loss: HashSet<PlayerId>,

    /// Players who can't lose the game.
    /// Example: Platinum Angel
    pub cant_lose_game: HashSet<PlayerId>,

    /// Players who can't win the game.
    /// Example: Angel's Grace preventing opponent's win
    pub cant_win_game: HashSet<PlayerId>,

    /// Players who can't become the monarch.
    pub cant_become_monarch: HashSet<PlayerId>,

    /// Permanents that can't be targeted.
    /// Example: Hexproof/Shroud (tracked separately), but also effects like
    /// "can't be the target of spells or abilities"
    pub cant_be_targeted: HashSet<ObjectId>,

    /// Permanents that can't be targeted by sources matching a filter.
    pub cant_be_targeted_from: Vec<ObjectCantBeTargetedFrom>,

    /// Players that can't be targeted.
    pub cant_target_players: HashSet<PlayerId>,

    /// Players that can't be targeted by sources matching a filter.
    pub cant_target_players_from: Vec<PlayerCantBeTargetedFrom>,

    /// Permanents that can't be countered while on the stack.
    /// Example: Vexing Shusher, Prowling Serpopard
    pub cant_be_countered: HashSet<ObjectId>,

    /// Permanents that can't transform.
    /// Example: "Non-Human Werewolves you control can't transform."
    pub cant_transform: HashSet<ObjectId>,

    /// Permanents that can't phase out.
    /// Example: "Target permanent can't phase out."
    pub cant_phase_out: HashSet<ObjectId>,

    /// Players who don't lose unspent mana as steps and phases end.
    /// A `None` entry retains the whole pool; `Some(color)` retains that color.
    /// Example: Upwelling, Kruphix (all mana); Omnath, Locus of Mana (green)
    pub dont_lose_unspent_mana: HashMap<PlayerId, HashSet<Option<crate::color::Color>>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PlayerCantBeTargetedFrom {
    pub player: PlayerId,
    pub source_filter: crate::target::ObjectFilter,
    pub controller: PlayerId,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ObjectCantBeTargetedFrom {
    pub object: ObjectId,
    pub source_filter: crate::target::ObjectFilter,
    pub controller: PlayerId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StickerMarker {
    pub action: KeywordActionKind,
    pub name_letter_count: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct RestrictionEffectInstance {
    pub restriction: crate::effect::Restriction,
    pub controller: PlayerId,
    pub source: ObjectId,
    pub iterated_player: Option<PlayerId>,
    pub tagged_objects: HashMap<crate::tag::TagKey, Vec<ObjectSnapshot>>,
    pub duration: crate::effect::Until,
    pub expires_end_of_turn: u32,
    pub consumed_next_untap: bool,
}

impl RestrictionEffectInstance {
    pub fn is_expired(&self, current_turn: u32) -> bool {
        if matches!(
            self.duration,
            crate::effect::Until::ControllersNextUntapStep
        ) && self.consumed_next_untap
        {
            return true;
        }
        matches!(self.duration, crate::effect::Until::EndOfTurn)
            && current_turn > self.expires_end_of_turn
    }

    pub fn is_active(&self, game: &GameState, current_turn: u32) -> bool {
        if self.is_expired(current_turn) {
            return false;
        }

        match self.duration {
            crate::effect::Until::YourNextTurn => {
                !(current_turn > self.expires_end_of_turn
                    && game.turn.active_player == self.controller)
            }
            crate::effect::Until::YourNextTurnEnd => current_turn <= self.expires_end_of_turn,
            crate::effect::Until::YourNextUpkeep => {
                if current_turn <= self.expires_end_of_turn
                    || game.turn.active_player != self.controller
                {
                    true
                } else if matches!(game.turn.phase, Phase::Beginning) {
                    !matches!(game.turn.step, Some(Step::Upkeep | Step::Draw))
                } else {
                    false
                }
            }
            crate::effect::Until::ControllersNextUntapStep => {
                game.turn.active_player == self.controller
                    && matches!(game.turn.phase, Phase::Beginning)
                    && matches!(game.turn.step, Some(Step::Untap))
            }
            crate::effect::Until::ThisLeavesTheBattlefield => game
                .object(self.source)
                .is_some_and(|obj| obj.zone == Zone::Battlefield),
            crate::effect::Until::SourceUntaps => game
                .object(self.source)
                .is_some_and(|obj| obj.zone == Zone::Battlefield && game.is_tapped(self.source)),
            crate::effect::Until::YouStopControllingThis => {
                game.object(self.source).is_some_and(|obj| {
                    obj.zone == Zone::Battlefield && game.controller_of(obj) == self.controller
                })
            }
            _ => true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct GoadEffectInstance {
    pub creature: ObjectId,
    pub goaded_by: PlayerId,
    pub source: ObjectId,
    pub duration: crate::effect::Until,
    pub expires_end_of_turn: u32,
}

impl GoadEffectInstance {
    pub fn is_expired(&self, current_turn: u32) -> bool {
        matches!(self.duration, crate::effect::Until::EndOfTurn)
            && current_turn > self.expires_end_of_turn
    }

    pub fn is_active(&self, game: &GameState, current_turn: u32) -> bool {
        if self.is_expired(current_turn) {
            return false;
        }

        match self.duration {
            crate::effect::Until::YourNextTurn => {
                !(current_turn > self.expires_end_of_turn
                    && game.turn.active_player == self.goaded_by)
            }
            crate::effect::Until::YourNextTurnEnd => current_turn <= self.expires_end_of_turn,
            crate::effect::Until::ThisLeavesTheBattlefield => game
                .object(self.source)
                .is_some_and(|obj| obj.zone == Zone::Battlefield),
            crate::effect::Until::SourceUntaps => game
                .object(self.source)
                .is_some_and(|obj| obj.zone == Zone::Battlefield && game.is_tapped(self.source)),
            crate::effect::Until::YouStopControllingThis => {
                game.object(self.source).is_some_and(|obj| {
                    obj.zone == Zone::Battlefield && game.controller_of(obj) == self.goaded_by
                })
            }
            _ => true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TemporarySpellCostReductionEffectInstance {
    pub player: PlayerId,
    pub source: ObjectId,
    pub duration_controller: PlayerId,
    pub filter: crate::target::ObjectFilter,
    pub reduction: crate::mana::ManaCost,
    pub generic_reduction: Option<crate::effect::Value>,
    pub applies_to_all_matching_this_turn: bool,
    pub duration: crate::effect::Until,
    pub remaining_uses: u32,
    pub expires_end_of_turn: u32,
}

impl TemporarySpellCostReductionEffectInstance {
    pub fn is_expired(&self, current_turn: u32, active_player: PlayerId) -> bool {
        if self.remaining_uses == 0 {
            return true;
        }
        match self.duration {
            crate::effect::Until::YourNextTurn => {
                current_turn > self.expires_end_of_turn && active_player == self.duration_controller
            }
            crate::effect::Until::Forever => false,
            _ => current_turn > self.expires_end_of_turn,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TemporarySpellAbilityGrantEffectInstance {
    pub player: PlayerId,
    pub source: ObjectId,
    pub filter: crate::target::ObjectFilter,
    pub ability: crate::static_abilities::StaticAbility,
    pub remaining_uses: u32,
    pub expires_end_of_turn: u32,
}

impl TemporarySpellAbilityGrantEffectInstance {
    pub fn is_expired(&self, current_turn: u32) -> bool {
        self.remaining_uses == 0 || current_turn > self.expires_end_of_turn
    }
}

impl CantEffectTracker {
    /// Create a new empty tracker.
    pub fn new() -> Self {
        Self::default()
    }

    pub fn merge(&mut self, other: CantEffectTracker) {
        self.cant_gain_life.extend(other.cant_gain_life);
        self.cant_search.extend(other.cant_search);
        self.cant_attack.extend(other.cant_attack);
        for (creature, defenders) in other.cant_attack_defenders {
            self.cant_attack_defenders
                .entry(creature)
                .or_default()
                .extend(defenders);
        }
        self.cant_attack_alone.extend(other.cant_attack_alone);
        self.cant_block.extend(other.cant_block);
        for (blocker, attackers) in other.cant_block_specific_attackers {
            self.cant_block_specific_attackers
                .entry(blocker)
                .or_default()
                .extend(attackers);
        }
        for (blocker, attackers) in other.must_block_specific_attackers {
            self.must_block_specific_attackers
                .entry(blocker)
                .or_default()
                .extend(attackers);
        }
        self.must_be_blocked.extend(other.must_be_blocked);
        self.cant_block_alone.extend(other.cant_block_alone);
        self.cant_untap.extend(other.cant_untap);
        self.cant_be_destroyed.extend(other.cant_be_destroyed);
        self.cant_be_regenerated.extend(other.cant_be_regenerated);
        self.cant_be_sacrificed.extend(other.cant_be_sacrificed);
        for (player, filters) in other.cant_cast_filters {
            for restriction in filters {
                self.add_cant_cast_filter_from_source(
                    player,
                    restriction.filter,
                    restriction.source,
                );
            }
        }
        self.cast_spells_only_as_sorcery
            .extend(other.cast_spells_only_as_sorcery);
        self.cant_activate_non_mana_abilities
            .extend(other.cant_activate_non_mana_abilities);
        self.cant_activate_abilities_of
            .extend(other.cant_activate_abilities_of);
        self.cant_activate_tap_abilities_of
            .extend(other.cant_activate_tap_abilities_of);
        self.cant_activate_non_mana_abilities_of
            .extend(other.cant_activate_non_mana_abilities_of);
        for (player, filters) in other.cant_cast_limit_filters {
            for filter in filters {
                self.add_cast_limit_filter(player, filter);
            }
        }
        self.cant_draw.extend(other.cant_draw);
        self.cant_draw_extra_cards
            .extend(other.cant_draw_extra_cards);
        self.cant_get_poison_counters
            .extend(other.cant_get_poison_counters);
        self.cant_be_blocked.extend(other.cant_be_blocked);
        self.cant_have_counters_placed
            .extend(other.cant_have_counters_placed);
        self.damage_cant_be_prevented |= other.damage_cant_be_prevented;
        self.life_total_cant_change
            .extend(other.life_total_cant_change);
        self.cant_lose_life.extend(other.cant_lose_life);
        self.damage_cant_cause_life_loss
            .extend(other.damage_cant_cause_life_loss);
        self.cant_lose_game.extend(other.cant_lose_game);
        self.cant_win_game.extend(other.cant_win_game);
        self.cant_become_monarch.extend(other.cant_become_monarch);
        self.cant_be_targeted.extend(other.cant_be_targeted);
        self.cant_be_targeted_from
            .extend(other.cant_be_targeted_from.clone());
        self.cant_target_players.extend(other.cant_target_players);
        self.cant_target_players_from
            .extend(other.cant_target_players_from.clone());
        self.cant_be_countered.extend(other.cant_be_countered);
        self.cant_transform.extend(other.cant_transform);
        self.cant_phase_out.extend(other.cant_phase_out);
        for (player, scopes) in other.dont_lose_unspent_mana {
            self.dont_lose_unspent_mana
                .entry(player)
                .or_default()
                .extend(scopes);
        }
    }

    /// Clear all tracked "can't" effects.
    /// Called when rebuilding the tracker from current game state.
    pub fn clear(&mut self) {
        self.cant_gain_life.clear();
        self.cant_search.clear();
        self.cant_attack.clear();
        self.cant_attack_defenders.clear();
        self.cant_attack_alone.clear();
        self.cant_block.clear();
        self.cant_block_specific_attackers.clear();
        self.must_block_specific_attackers.clear();
        self.must_be_blocked.clear();
        self.cant_block_alone.clear();
        self.cant_untap.clear();
        self.cant_be_destroyed.clear();
        self.cant_be_regenerated.clear();
        self.cant_be_sacrificed.clear();
        self.cant_cast_filters.clear();
        self.cast_spells_only_as_sorcery.clear();
        self.cant_activate_non_mana_abilities.clear();
        self.cant_activate_abilities_of.clear();
        self.cant_activate_tap_abilities_of.clear();
        self.cant_activate_non_mana_abilities_of.clear();
        self.cant_cast_limit_filters.clear();
        self.cant_draw.clear();
        self.cant_draw_extra_cards.clear();
        self.cant_get_poison_counters.clear();
        self.cant_be_blocked.clear();
        self.cant_have_counters_placed.clear();
        self.damage_cant_be_prevented = false;
        self.life_total_cant_change.clear();
        self.cant_lose_life.clear();
        self.damage_cant_cause_life_loss.clear();
        self.cant_lose_game.clear();
        self.cant_win_game.clear();
        self.cant_become_monarch.clear();
        self.cant_be_targeted.clear();
        self.cant_be_targeted_from.clear();
        self.cant_target_players.clear();
        self.cant_target_players_from.clear();
        self.cant_be_countered.clear();
        self.cant_transform.clear();
        self.cant_phase_out.clear();
        self.dont_lose_unspent_mana.clear();
    }

    /// Mana-retention scopes for a player, if any.
    /// `Some` containing `None` means the whole pool is retained.
    pub fn retained_mana_scopes(
        &self,
        player: PlayerId,
    ) -> Option<&HashSet<Option<crate::color::Color>>> {
        self.dont_lose_unspent_mana.get(&player)
    }

    /// Check if a player can gain life.
    pub fn can_gain_life(&self, player: PlayerId) -> bool {
        !self.cant_gain_life.contains(&player) && !self.life_total_cant_change.contains(&player)
    }

    /// Check if a player can lose life (not from damage).
    pub fn can_lose_life(&self, player: PlayerId) -> bool {
        !self.cant_lose_life.contains(&player) && !self.life_total_cant_change.contains(&player)
    }

    /// Check if damage dealt to a player can cause that player to lose life.
    pub fn can_damage_cause_life_loss(&self, player: PlayerId) -> bool {
        self.can_lose_life(player) && !self.damage_cant_cause_life_loss.contains(&player)
    }

    /// Check if a player's life total can change (Platinum Emperion, etc.).
    pub fn can_change_life_total(&self, player: PlayerId) -> bool {
        !self.life_total_cant_change.contains(&player)
    }

    /// Check if a player can search their library.
    pub fn can_search_library(&self, player: PlayerId) -> bool {
        !self.cant_search.contains(&player)
    }

    /// Check if a creature can attack.
    pub fn can_attack(&self, creature: ObjectId) -> bool {
        !self.cant_attack.contains(&creature)
    }

    /// Check if a creature can attack a defending player or planeswalker they control.
    pub fn can_attack_defending_player(
        &self,
        creature: ObjectId,
        defending_player: PlayerId,
    ) -> bool {
        self.can_attack(creature)
            && self
                .cant_attack_defenders
                .get(&creature)
                .is_none_or(|defenders| !defenders.contains(&defending_player))
    }

    /// Check if a creature can attack alone (as the only attacker).
    pub fn can_attack_alone(&self, creature: ObjectId) -> bool {
        !self.cant_attack_alone.contains(&creature)
    }

    /// Check if a creature can block.
    pub fn can_block(&self, creature: ObjectId) -> bool {
        !self.cant_block.contains(&creature)
    }

    /// Check if a creature can block a specific attacker.
    pub fn can_block_attacker(&self, blocker: ObjectId, attacker: ObjectId) -> bool {
        self.can_block(blocker)
            && self
                .cant_block_specific_attackers
                .get(&blocker)
                .is_none_or(|attackers| !attackers.contains(&attacker))
    }

    /// Check if a creature can block alone (as the only blocker).
    pub fn can_block_alone(&self, creature: ObjectId) -> bool {
        !self.cant_block_alone.contains(&creature)
    }

    /// Check if a creature must block a specific attacker this turn if able.
    pub fn must_block_attacker(&self, blocker: ObjectId, attacker: ObjectId) -> bool {
        self.must_block_specific_attackers
            .get(&blocker)
            .is_some_and(|attackers| attackers.contains(&attacker))
    }

    /// Check if an attacker must be blocked this turn if able.
    pub fn must_be_blocked(&self, attacker: ObjectId) -> bool {
        self.must_be_blocked.contains(&attacker)
    }

    /// Get required attackers for a blocker, if any.
    pub fn required_attackers_for_blocker(&self, blocker: ObjectId) -> Option<&HashSet<ObjectId>> {
        self.must_block_specific_attackers.get(&blocker)
    }

    /// Check if a permanent can untap during untap step.
    pub fn can_untap(&self, permanent: ObjectId) -> bool {
        !self.cant_untap.contains(&permanent)
    }

    /// Check if a permanent can untap during the specified player's untap step.
    pub fn can_untap_during_step(
        &self,
        permanent: ObjectId,
        permanent_controller: PlayerId,
        untap_player: PlayerId,
    ) -> bool {
        permanent_controller != untap_player || !self.cant_untap.contains(&permanent)
    }

    /// Check if damage can be prevented.
    pub fn can_prevent_damage(&self) -> bool {
        !self.damage_cant_be_prevented
    }

    /// Check if a permanent can be destroyed.
    pub fn can_be_destroyed(&self, permanent: ObjectId) -> bool {
        !self.cant_be_destroyed.contains(&permanent)
    }

    /// Check if a permanent can be regenerated.
    pub fn can_be_regenerated(&self, permanent: ObjectId) -> bool {
        !self.cant_be_regenerated.contains(&permanent)
    }

    /// Check if a permanent can be sacrificed.
    pub fn can_be_sacrificed(&self, permanent: ObjectId) -> bool {
        !self.cant_be_sacrificed.contains(&permanent)
    }

    /// Check if a creature can be blocked.
    pub fn can_be_blocked(&self, creature: ObjectId) -> bool {
        !self.cant_be_blocked.contains(&creature)
    }

    /// Check if a player can lose the game.
    pub fn can_lose_game(&self, player: PlayerId) -> bool {
        !self.cant_lose_game.contains(&player)
    }

    /// Check if a player can win the game.
    pub fn can_win_game(&self, player: PlayerId) -> bool {
        !self.cant_win_game.contains(&player)
    }

    /// Check if a player can become the monarch.
    pub fn can_become_monarch(&self, player: PlayerId) -> bool {
        !self.cant_become_monarch.contains(&player)
    }

    /// Check if a player can draw cards at all.
    pub fn can_draw(&self, player: PlayerId) -> bool {
        !self.cant_draw.contains(&player)
    }

    /// Check if a player can draw extra cards this turn.
    pub fn can_draw_extra_cards(&self, player: PlayerId) -> bool {
        !self.cant_draw_extra_cards.contains(&player)
    }

    /// Check if a player can get poison counters.
    pub fn can_get_poison_counters(&self, player: PlayerId) -> bool {
        !self.cant_get_poison_counters.contains(&player)
    }

    /// Check if a player can cast spells.
    pub fn can_cast_spells(&self, player: PlayerId) -> bool {
        self.cast_filters_for_player(player).is_none_or(|filters| {
            !filters
                .iter()
                .any(|restriction| restriction.filter == crate::target::ObjectFilter::default())
        })
    }

    /// Check if a player can activate non-mana abilities.
    pub fn can_activate_non_mana_abilities(&self, player: PlayerId) -> bool {
        !self.cant_activate_non_mana_abilities.contains(&player)
    }

    /// Check if activated abilities of a permanent can be activated (including mana abilities).
    pub fn can_activate_abilities_of(&self, source: ObjectId) -> bool {
        !self.cant_activate_abilities_of.contains(&source)
    }

    /// Check if activated abilities with {T} in their costs of a permanent can be activated.
    pub fn can_activate_tap_abilities_of(&self, source: ObjectId) -> bool {
        !self.cant_activate_tap_abilities_of.contains(&source)
    }

    /// Check if non-mana activated abilities of a permanent can be activated.
    pub fn can_activate_non_mana_abilities_of(&self, source: ObjectId) -> bool {
        !self.cant_activate_non_mana_abilities_of.contains(&source)
    }

    /// Check if a player can cast creature spells.
    pub fn can_cast_creature_spells(&self, player: PlayerId) -> bool {
        self.cast_filters_for_player(player).is_none_or(|filters| {
            !filters.iter().any(|restriction| {
                restriction.filter
                    == crate::target::ObjectFilter::default()
                        .with_type(crate::types::CardType::Creature)
            })
        })
    }

    /// Add a cast-prohibition filter for a player ("can't cast [matching] spells").
    pub fn add_cant_cast_filter(
        &mut self,
        player: PlayerId,
        spell_filter: crate::target::ObjectFilter,
    ) {
        self.add_cant_cast_filter_from_source(player, spell_filter, None);
    }

    pub fn add_cant_cast_filter_from_source(
        &mut self,
        player: PlayerId,
        spell_filter: crate::target::ObjectFilter,
        source: Option<ObjectId>,
    ) {
        let restriction = CastRestrictionFilter {
            filter: spell_filter,
            source,
        };
        let filters = self.cant_cast_filters.entry(player).or_default();
        if !filters.iter().any(|existing| existing == &restriction) {
            filters.push(restriction);
        }
    }

    /// Get active cast-prohibition filters for a player, if any.
    pub fn cast_filters_for_player(&self, player: PlayerId) -> Option<&[CastRestrictionFilter]> {
        self.cant_cast_filters.get(&player).map(Vec::as_slice)
    }

    /// Add a cast-limit filter for a player ("can't cast more than one matching spell each turn").
    pub fn add_cast_limit_filter(
        &mut self,
        player: PlayerId,
        spell_filter: crate::target::ObjectFilter,
    ) {
        let filters = self.cant_cast_limit_filters.entry(player).or_default();
        if !filters.iter().any(|existing| existing == &spell_filter) {
            filters.push(spell_filter);
        }
    }

    /// Get active cast-limit filters for a player, if any.
    pub fn cast_limit_filters_for_player(
        &self,
        player: PlayerId,
    ) -> Option<&[crate::target::ObjectFilter]> {
        self.cant_cast_limit_filters.get(&player).map(Vec::as_slice)
    }

    /// Check if a player can cast an additional spell matching a specific filter this turn.
    pub fn can_cast_additional_spell_matching_this_turn(
        &self,
        player: PlayerId,
        spell_filter: &crate::target::ObjectFilter,
    ) -> bool {
        !self
            .cast_limit_filters_for_player(player)
            .is_some_and(|filters| filters.iter().any(|filter| filter == spell_filter))
    }

    /// Check if a player can cast an additional spell this turn.
    pub fn can_cast_additional_spell_this_turn(&self, player: PlayerId) -> bool {
        self.can_cast_additional_spell_matching_this_turn(
            player,
            &crate::target::ObjectFilter::default(),
        )
    }

    /// Check if a player can cast an additional noncreature spell this turn.
    pub fn can_cast_additional_noncreature_spell_this_turn(&self, player: PlayerId) -> bool {
        self.can_cast_additional_spell_matching_this_turn(
            player,
            &crate::target::ObjectFilter::default().without_type(crate::types::CardType::Creature),
        )
    }

    /// Check if a player can cast an additional nonartifact spell this turn.
    pub fn can_cast_additional_nonartifact_spell_this_turn(&self, player: PlayerId) -> bool {
        self.can_cast_additional_spell_matching_this_turn(
            player,
            &crate::target::ObjectFilter::default().without_type(crate::types::CardType::Artifact),
        )
    }

    /// Check if a player can cast an additional non-Phyrexian spell this turn.
    pub fn can_cast_additional_nonphyrexian_spell_this_turn(&self, player: PlayerId) -> bool {
        self.can_cast_additional_spell_matching_this_turn(
            player,
            &crate::target::ObjectFilter::default()
                .without_subtype(crate::types::Subtype::Phyrexian),
        )
    }

    /// Check if a permanent can have counters placed on it.
    pub fn can_have_counters_placed(&self, permanent: ObjectId) -> bool {
        !self.cant_have_counters_placed.contains(&permanent)
    }

    /// Check if a permanent is untargetable by the rules tracker.
    pub fn is_untargetable(&self, permanent: ObjectId) -> bool {
        self.cant_be_targeted.contains(&permanent)
    }

    pub fn can_target_object_from_source(
        &self,
        game: &GameState,
        object: ObjectId,
        source_id: ObjectId,
    ) -> bool {
        let Some(source) = game.object(source_id) else {
            return true;
        };

        !self.cant_be_targeted_from.iter().any(|restriction| {
            if restriction.object != object {
                return false;
            }
            let filter_ctx = game.filter_context_for(restriction.controller, Some(source_id));
            restriction.source_filter.matches(source, &filter_ctx, game)
        })
    }

    /// Check if a player can be targeted.
    pub fn can_target_player(&self, player: PlayerId) -> bool {
        !self.cant_target_players.contains(&player)
    }

    /// Check if a player can be targeted by a specific source.
    pub fn can_target_player_from_source(
        &self,
        game: &GameState,
        player: PlayerId,
        source_id: ObjectId,
    ) -> bool {
        if !self.can_target_player(player) {
            return false;
        }

        let Some(source) = game.object(source_id) else {
            return true;
        };

        !self.cant_target_players_from.iter().any(|restriction| {
            if restriction.player != player {
                return false;
            }
            let filter_ctx = game.filter_context_for(restriction.controller, Some(source_id));
            restriction.source_filter.matches(source, &filter_ctx, game)
        })
    }

    /// Check if a spell on the stack can be countered by effects.
    pub fn can_be_countered(&self, spell: ObjectId) -> bool {
        !self.cant_be_countered.contains(&spell)
    }

    /// Check if a permanent can transform.
    pub fn can_transform(&self, permanent: ObjectId) -> bool {
        !self.cant_transform.contains(&permanent)
    }

    /// Check if a permanent can phase out.
    pub fn can_phase_out(&self, permanent: ObjectId) -> bool {
        !self.cant_phase_out.contains(&permanent)
    }

    /// Add a player to the "can't gain life" set.
    pub fn add_cant_gain_life(&mut self, player: PlayerId) {
        self.cant_gain_life.insert(player);
    }

    /// Add a creature to the "can't attack" set.
    pub fn add_cant_attack(&mut self, creature: ObjectId) {
        self.cant_attack.insert(creature);
    }

    /// Add defender-specific attack prohibitions for a creature.
    pub fn add_cant_attack_defenders<I>(&mut self, creature: ObjectId, defenders: I)
    where
        I: IntoIterator<Item = PlayerId>,
    {
        self.cant_attack_defenders
            .entry(creature)
            .or_default()
            .extend(defenders);
    }

    /// Add a creature to the "can't attack alone" set.
    pub fn add_cant_attack_alone(&mut self, creature: ObjectId) {
        self.cant_attack_alone.insert(creature);
    }

    /// Add a creature to the "can't block" set.
    pub fn add_cant_block(&mut self, creature: ObjectId) {
        self.cant_block.insert(creature);
    }

    /// Add a creature to the "can't block alone" set.
    pub fn add_cant_block_alone(&mut self, creature: ObjectId) {
        self.cant_block_alone.insert(creature);
    }

    /// Add a permanent to the "can't untap" set.
    pub fn add_cant_untap(&mut self, permanent: ObjectId) {
        self.cant_untap.insert(permanent);
    }

    /// Add a creature to the "can't be blocked" set.
    pub fn add_cant_be_blocked(&mut self, creature: ObjectId) {
        self.cant_be_blocked.insert(creature);
    }

    /// Set that damage can't be prevented.
    pub fn set_damage_cant_be_prevented(&mut self, value: bool) {
        self.damage_cant_be_prevented = value;
    }

    /// Add a player to the "can't lose game" set.
    pub fn add_cant_lose_game(&mut self, player: PlayerId) {
        self.cant_lose_game.insert(player);
    }

    /// Add a player to the "life total can't change" set.
    pub fn add_life_total_cant_change(&mut self, player: PlayerId) {
        self.life_total_cant_change.insert(player);
    }
}

// =============================================================================
// "Spend Mana As Though Any Color" Tracking
// =============================================================================
//
// These effects allow mana to be spent as though it were any color.
// They are not replacement effects and must be consulted during mana payment.
//
// Examples:
// - "Players may spend mana as though it were mana of any color." (Mycosynth Lattice)
// - "You may spend mana as though it were mana of any color to pay activation costs
//    of ~'s abilities." (Manascape Refractor)

#[derive(Debug, Clone, Default)]
pub struct ManaSpendEffectTracker {
    /// Active permissions that let a player spend mana as though it were mana
    /// of any color.
    pub permissions: Vec<ActiveManaSpendPermission>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ActiveManaSpendPermission {
    pub permission: crate::effect::ManaSpendPermission,
    pub controller: PlayerId,
    pub source: ManaSpendPermissionSource,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ManaSpendPermissionSource {
    StaticAbility,
    Effect {
        source_id: ObjectId,
        expires_end_of_turn: u32,
    },
}

impl ManaSpendEffectTracker {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clear(&mut self) {
        self.permissions.clear();
    }

    pub fn retain_effect_permissions(&mut self, current_turn: u32) {
        self.permissions.retain(|permission| {
            matches!(
                permission.source,
                ManaSpendPermissionSource::Effect {
                    expires_end_of_turn,
                    ..
                } if current_turn <= expires_end_of_turn
            )
        });
    }

    pub fn cleanup_expired(&mut self, current_turn: u32) {
        self.permissions
            .retain(|permission| match permission.source {
                ManaSpendPermissionSource::StaticAbility => true,
                ManaSpendPermissionSource::Effect {
                    expires_end_of_turn,
                    ..
                } => current_turn <= expires_end_of_turn,
            });
    }
}

impl ActiveManaSpendPermission {
    pub fn allows(&self, game: &GameState, payer: PlayerId, source: Option<ObjectId>) -> bool {
        if self.permission.mana_source_filter.is_some() {
            return false;
        }
        self.allows_scope(game, payer, source)
    }

    pub fn allows_for_mana_source(
        &self,
        game: &GameState,
        payer: PlayerId,
        payment_source: Option<ObjectId>,
        mana_source: ObjectId,
    ) -> bool {
        if !self.allows_scope(game, payer, payment_source) {
            return false;
        }

        let Some(filter) = &self.permission.mana_source_filter else {
            return true;
        };
        let Some(source_obj) = game.object(mana_source) else {
            return false;
        };
        let filter_ctx = game.filter_context_for(self.controller, Some(mana_source));
        filter.matches(source_obj, &filter_ctx, game)
    }

    pub fn allows_with_source_filtered_mana(
        &self,
        game: &GameState,
        payer: PlayerId,
        payment_source: Option<ObjectId>,
    ) -> bool {
        self.permission.mana_source_filter.is_some()
            && self.allows_scope(game, payer, payment_source)
    }

    fn allows_scope(&self, game: &GameState, payer: PlayerId, source: Option<ObjectId>) -> bool {
        if matches!(
            self.source,
            ManaSpendPermissionSource::Effect {
                expires_end_of_turn,
                ..
            } if game.turn.turn_number > expires_end_of_turn
        ) {
            return false;
        }

        let combat = game.combat.as_ref();
        if !crate::game_loop::player_matches_filter_with_combat(
            payer,
            &self.permission.player,
            game,
            self.controller,
            combat,
        ) {
            return false;
        }

        match &self.permission.scope {
            crate::effect::ManaSpendScope::AllCosts => true,
            crate::effect::ManaSpendScope::ActivationCostsOf(filter) => {
                let Some(source_id) = source else {
                    return false;
                };
                let Some(source_obj) = game.object(source_id) else {
                    return false;
                };
                let filter_ctx = game.filter_context_for(self.controller, Some(source_id));
                filter.matches(source_obj, &filter_ctx, game)
            }
            crate::effect::ManaSpendScope::CastingSpellsWithStableIds(stable_ids) => {
                let Some(source_id) = source else {
                    return false;
                };
                let Some(source_obj) = game.object(source_id) else {
                    return false;
                };
                if !stable_ids.contains(&source_obj.stable_id) {
                    return false;
                }
                source_obj.zone == Zone::Exile
                    || (source_obj.zone == Zone::Stack
                        && game
                            .cast_origin_snapshot(source_id)
                            .is_some_and(|snapshot| snapshot.zone == Zone::Exile))
            }
            crate::effect::ManaSpendScope::CastingSpellsMatching(filter) => {
                let Some(source_id) = source else {
                    return false;
                };
                let Some(source_obj) = game.object(source_id) else {
                    return false;
                };
                let filter_ctx = game.filter_context_for(self.controller, Some(source_id));
                filter.matches(source_obj, &filter_ctx, game)
                    || (source_obj.zone == Zone::Stack
                        && game
                            .cast_origin_snapshot(source_id)
                            .is_some_and(|snapshot| {
                                filter.matches_snapshot(snapshot, &filter_ctx, game)
                            }))
            }
        }
    }
}

/// Game phases.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Beginning,
    FirstMain,
    Combat,
    NextMain,
    Ending,
}

impl Phase {
    pub fn name(self) -> &'static str {
        match self {
            Phase::Beginning => "beginning phase",
            Phase::FirstMain => "first main phase",
            Phase::Combat => "combat phase",
            Phase::NextMain => "second main phase",
            Phase::Ending => "ending phase",
        }
    }
}

impl std::fmt::Display for Phase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

/// Steps within phases.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    // Beginning phase
    Untap,
    Upkeep,
    Draw,
    // Combat phase
    BeginCombat,
    DeclareAttackers,
    DeclareBlockers,
    CombatDamage,
    EndCombat,
    // Ending phase
    End,
    Cleanup,
}

impl Step {
    pub fn name(self) -> &'static str {
        match self {
            Step::Untap => "untap step",
            Step::Upkeep => "upkeep step",
            Step::Draw => "draw step",
            Step::BeginCombat => "begin combat step",
            Step::DeclareAttackers => "declare attackers step",
            Step::DeclareBlockers => "declare blockers step",
            Step::CombatDamage => "combat damage step",
            Step::EndCombat => "end combat step",
            Step::End => "end step",
            Step::Cleanup => "cleanup step",
        }
    }
}

impl std::fmt::Display for Step {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

/// Turn state tracking.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TurnState {
    pub active_player: PlayerId,
    pub priority_player: Option<PlayerId>,
    pub turn_number: u32,
    pub phase: Phase,
    pub step: Option<Step>,
}

impl TurnState {
    pub fn new(active_player: PlayerId) -> Self {
        Self {
            active_player,
            priority_player: Some(active_player),
            turn_number: 1,
            phase: Phase::Beginning,
            step: Some(Step::Untap),
        }
    }
}

pub use ironsmith_core::{PlayerControlDuration, PlayerControlStart};

/// An effect that lets a player choose attackers and/or blockers this turn.
#[derive(Debug, Clone)]
pub struct CombatChoiceControlEffect {
    pub controller: PlayerId,
    pub choose_attackers: bool,
    pub choose_blockers: bool,
    pub expires_on_turn: u32,
    pub timestamp: u64,
}

/// An effect that causes one player to control another player's decisions.
#[derive(Debug, Clone)]
pub struct PlayerControlEffect {
    pub controller: PlayerId,
    pub target: PlayerId,
    pub start: PlayerControlStart,
    pub duration: PlayerControlDuration,
    pub source: Option<StableId>,
    pub timestamp: u64,
    pub active: bool,
    pub expires_on_turn: Option<u32>,
}

/// A currently resolving scope that causes one player to control another
/// player's decisions.
#[derive(Debug, Clone)]
pub struct ScopedPlayerControlEffect {
    pub controller: PlayerId,
    pub target: PlayerId,
    pub source: Option<StableId>,
    pub timestamp: u64,
}

/// A target for spells or abilities.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Target {
    Object(ObjectId),
    Player(PlayerId),
}

/// A chosen target requirement bound to a range within the flattened target list.
#[derive(Debug, Clone, PartialEq)]
pub struct TargetAssignment {
    pub spec: ChooseSpec,
    pub range: Range<usize>,
}

/// An entry on the stack.
#[derive(Debug, Clone)]
pub struct StackEntry {
    pub object_id: ObjectId,
    pub controller: PlayerId,
    pub provenance: ProvNodeId,
    pub targets: Vec<Target>,
    pub target_assignments: Vec<TargetAssignment>,
    pub x_value: Option<u32>,
    /// For activated abilities, whether the activation cost contained X.
    pub activation_cost_has_x: bool,
    /// For activated abilities, whether the activation cost contained {T}.
    pub activation_cost_has_tap: bool,
    /// For triggered/activated abilities, the effects to execute.
    /// For spells, this is None and effects come from the spell itself.
    pub ability_effects: Option<crate::resolution::ResolutionProgram>,
    /// Spending restrictions for mana produced while resolving this stack entry.
    pub mana_usage_restrictions: Vec<crate::ability::ManaUsageRestriction>,
    /// Chosen creature type snapshot for restricted mana produced by this entry.
    pub mana_source_chosen_creature_type: Option<crate::types::Subtype>,
    /// Whether this is an ability (triggered or activated) vs a spell.
    pub is_ability: bool,
    /// The casting method used (normal or alternative like flashback).
    pub casting_method: CastingMethod,
    /// Which optional costs were paid (kicker, buyback, etc.).
    pub optional_costs_paid: OptionalCostsPaid,
    /// The defending player for combat-related triggers.
    pub defending_player: Option<PlayerId>,
    /// The chosen player linked to this source at the time the stack entry was created.
    pub chosen_player: Option<PlayerId>,
    /// If this is a chapter ability, the source object's ID.
    ///
    /// Saga state-based actions use this to delay sacrifice while a chapter
    /// ability from that Saga is still on the stack.
    pub chapter_ability_source: Option<ObjectId>,
    /// The stable instance ID of the source (persists across zone changes).
    /// Used to track the source even after it leaves the battlefield.
    pub source_stable_id: Option<StableId>,
    /// Last known snapshot of the source at the time this stack entry was created.
    /// Used for source-dependent checks when the source object no longer exists.
    pub source_snapshot: Option<crate::snapshot::ObjectSnapshot>,
    /// The name of the source card/permanent for display purposes.
    /// Captured at the time the ability is put on the stack.
    pub source_name: Option<String>,
    /// The event that triggered this ability (for triggered abilities).
    /// Contains information about what caused the trigger (e.g., which object entered the battlefield).
    pub triggering_event: Option<crate::triggers::TriggerEvent>,
    /// Numeric value computed by the trigger matcher for resolving "that many".
    pub event_value_amount: Option<i32>,
    /// Structural identity of the triggered ability represented by this stack entry.
    pub trigger_identity: Option<TriggerIdentity>,
    /// Index of the activated ability represented by this stack entry.
    pub ability_index: Option<usize>,
    /// Intervening-if condition that must be true at resolution time (for triggered abilities).
    /// If this condition is false when the ability would resolve, the ability does nothing.
    pub intervening_if: Option<crate::ConditionExpr>,
    /// Pre-chosen modes for modal spells (chosen during casting per rule 601.2b).
    /// If Some, resolution should use these instead of prompting.
    pub chosen_modes: Option<Vec<usize>>,
    /// Permanents that contributed keyword-ability alternative payments to this spell cast.
    pub keyword_payment_contributions: Vec<KeywordPaymentContribution>,
    /// Creatures that crewed this object this turn, captured when the entry was created.
    ///
    /// Used to populate runtime tags for filters like "each creature that crewed it this turn".
    pub crew_contributors: Vec<ObjectId>,

    /// Creatures that saddled this object this turn, captured when the entry was created.
    ///
    /// Used to populate runtime tags for filters like "each creature that saddled it this turn".
    pub saddle_contributors: Vec<ObjectId>,
    /// Tagged object snapshots preserved from cost payment and targeting flows.
    ///
    /// This supports resolution-time references like `sacrifice_cost_0`.
    pub tagged_objects:
        std::collections::HashMap<crate::tag::TagKey, Vec<crate::snapshot::ObjectSnapshot>>,
    /// Outcomes preserved from cost effects labeled with `WithIdEffect`.
    pub effect_outcomes:
        std::collections::HashMap<crate::effect::EffectId, crate::effect::EffectOutcome>,
}

/// A mana ability granted to a player until end of turn.
///
/// This models effects like Channel that temporarily give a player a mana ability
/// not tied to any permanent.
#[derive(Debug, Clone)]
pub struct GrantedManaAbility {
    pub controller: PlayerId,
    pub ability: crate::ability::ActivatedAbility,
    pub expires_end_of_turn: u32,
}

impl StackEntry {
    pub fn new(object_id: ObjectId, controller: PlayerId) -> Self {
        Self {
            object_id,
            controller,
            provenance: ProvNodeId::default(),
            targets: Vec::new(),
            target_assignments: Vec::new(),
            x_value: None,
            activation_cost_has_x: false,
            activation_cost_has_tap: false,
            ability_effects: None,
            mana_usage_restrictions: Vec::new(),
            mana_source_chosen_creature_type: None,
            is_ability: false,
            casting_method: CastingMethod::Normal,
            optional_costs_paid: OptionalCostsPaid::default(),
            defending_player: None,
            chosen_player: None,
            chapter_ability_source: None,
            source_stable_id: None,
            source_snapshot: None,
            source_name: None,
            triggering_event: None,
            event_value_amount: None,
            trigger_identity: None,
            ability_index: None,
            intervening_if: None,
            chosen_modes: None,
            keyword_payment_contributions: Vec::new(),
            crew_contributors: Vec::new(),
            saddle_contributors: Vec::new(),
            tagged_objects: std::collections::HashMap::new(),
            effect_outcomes: std::collections::HashMap::new(),
        }
    }

    /// Create a stack entry for a triggered or activated ability.
    pub fn ability(
        source_id: ObjectId,
        controller: PlayerId,
        effects: impl Into<crate::resolution::ResolutionProgram>,
    ) -> Self {
        Self {
            object_id: source_id,
            controller,
            provenance: ProvNodeId::default(),
            targets: Vec::new(),
            target_assignments: Vec::new(),
            x_value: None,
            activation_cost_has_x: false,
            activation_cost_has_tap: false,
            ability_effects: Some(effects.into()),
            mana_usage_restrictions: Vec::new(),
            mana_source_chosen_creature_type: None,
            is_ability: true,
            casting_method: CastingMethod::Normal,
            optional_costs_paid: OptionalCostsPaid::default(),
            defending_player: None,
            chosen_player: None,
            chapter_ability_source: None,
            source_stable_id: None,
            source_snapshot: None,
            source_name: None,
            triggering_event: None,
            event_value_amount: None,
            trigger_identity: None,
            ability_index: None,
            intervening_if: None,
            chosen_modes: None,
            keyword_payment_contributions: Vec::new(),
            crew_contributors: Vec::new(),
            saddle_contributors: Vec::new(),
            tagged_objects: std::collections::HashMap::new(),
            effect_outcomes: std::collections::HashMap::new(),
        }
    }

    /// Mark this as a chapter ability from the given source.
    pub fn with_chapter_ability_source(mut self, source_id: ObjectId) -> Self {
        self.chapter_ability_source = Some(source_id);
        self
    }

    pub fn with_targets(mut self, targets: Vec<Target>) -> Self {
        self.targets = targets;
        self
    }

    pub fn with_target_assignments(mut self, target_assignments: Vec<TargetAssignment>) -> Self {
        self.target_assignments = target_assignments;
        self
    }

    pub fn with_provenance(mut self, provenance: ProvNodeId) -> Self {
        self.provenance = provenance;
        self
    }

    pub fn with_x(mut self, x: u32) -> Self {
        self.x_value = Some(x);
        self
    }

    pub fn with_activation_cost_has_x(mut self, has_x: bool) -> Self {
        self.activation_cost_has_x = has_x;
        self
    }

    pub fn with_activation_cost_has_tap(mut self, has_tap: bool) -> Self {
        self.activation_cost_has_tap = has_tap;
        self
    }

    pub fn with_casting_method(mut self, method: CastingMethod) -> Self {
        self.casting_method = method;
        self
    }

    pub fn with_optional_costs_paid(mut self, paid: OptionalCostsPaid) -> Self {
        self.optional_costs_paid = paid;
        self
    }

    pub fn with_defending_player(mut self, player: PlayerId) -> Self {
        self.defending_player = Some(player);
        self
    }

    pub fn with_chosen_player(mut self, player: Option<PlayerId>) -> Self {
        self.chosen_player = player;
        self
    }

    /// Set the source instance ID (stable identifier across zone changes).
    pub fn with_source_stable_id(mut self, stable_id: StableId) -> Self {
        self.source_stable_id = Some(stable_id);
        self
    }

    /// Set the source snapshot for source-LKI lookups during resolution.
    pub fn with_source_snapshot(mut self, snapshot: crate::snapshot::ObjectSnapshot) -> Self {
        self.source_snapshot = Some(snapshot);
        self
    }

    pub fn with_mana_usage_restrictions(
        mut self,
        restrictions: Vec<crate::ability::ManaUsageRestriction>,
        source_chosen_creature_type: Option<crate::types::Subtype>,
    ) -> Self {
        self.mana_usage_restrictions = restrictions;
        self.mana_source_chosen_creature_type = source_chosen_creature_type;
        self
    }

    /// Set the source name for display purposes.
    pub fn with_source_name(mut self, name: String) -> Self {
        self.source_name = Some(name);
        self
    }

    /// Set both source instance ID and name from a source object.
    pub fn with_source_info(mut self, stable_id: StableId, name: String) -> Self {
        self.source_stable_id = Some(stable_id);
        self.source_name = Some(name);
        self
    }

    /// Set the triggering event for this triggered ability.
    pub fn with_triggering_event(mut self, event: crate::triggers::TriggerEvent) -> Self {
        self.triggering_event = Some(event);
        self
    }

    /// Set a numeric value computed by the trigger matcher for grouped events.
    pub fn with_event_value_amount(mut self, amount: i32) -> Self {
        self.event_value_amount = Some(amount);
        self
    }

    /// Set the structural trigger identity for this triggered ability stack entry.
    pub fn with_trigger_identity(mut self, trigger_identity: TriggerIdentity) -> Self {
        self.trigger_identity = Some(trigger_identity);
        self
    }

    /// Set the activated ability index for this activated ability stack entry.
    pub fn with_ability_index(mut self, ability_index: usize) -> Self {
        self.ability_index = Some(ability_index);
        self
    }

    /// Set the intervening-if condition that must be true at resolution time.
    pub fn with_intervening_if(mut self, condition: crate::ConditionExpr) -> Self {
        self.intervening_if = Some(condition);
        self
    }

    /// Set pre-chosen modes for modal spells (per MTG rule 601.2b).
    pub fn with_chosen_modes(mut self, modes: Option<Vec<usize>>) -> Self {
        self.chosen_modes = modes;
        self
    }

    /// Set keyword-ability payment contributors for this stack entry.
    pub fn with_keyword_payment_contributions(
        mut self,
        contributions: Vec<KeywordPaymentContribution>,
    ) -> Self {
        self.keyword_payment_contributions = contributions;
        self
    }

    /// Carry tagged object snapshots into stack resolution context.
    pub fn with_tagged_objects(
        mut self,
        tagged: std::collections::HashMap<crate::tag::TagKey, Vec<crate::snapshot::ObjectSnapshot>>,
    ) -> Self {
        self.tagged_objects = tagged;
        self
    }

    pub fn with_effect_outcomes(
        mut self,
        outcomes: std::collections::HashMap<crate::effect::EffectId, crate::effect::EffectOutcome>,
    ) -> Self {
        self.effect_outcomes = outcomes;
        self
    }
}

/// Complete game state.
#[derive(Debug, Clone)]
pub struct GameState {
    // Players
    pub players: Vec<Player>,

    // Objects and denormalized zone indexes
    pub object_store: ObjectStore,

    // The stack
    pub stack: Vec<StackEntry>,

    // Turn tracking
    pub turn: TurnState,
    pub turn_store: TurnStore,
    pub effect_store: EffectStore,
    choice_store: Arc<ChoiceStore>,
    metadata: MetadataStateStore,
    mutation_revision: u64,
    next_object_id: u64,
    zone_revisions: ZoneRevisionSnapshot,

    /// Current combat state (Some during combat phase, None otherwise).
    /// Effects can directly add creatures to combat when this is set.
    pub combat: Option<crate::combat_state::CombatState>,
    /// Whether the game currently has a day/night designation.
    pub has_day_night: bool,
    /// Whether the game is currently in night mode (day/night designation).
    pub is_night: bool,
    /// Current monarch designation holder, if any.
    pub monarch: Option<PlayerId>,
    /// Current initiative designation holder, if any.
    pub initiative: Option<PlayerId>,
    /// Players who have earned the city's blessing designation this game.
    citys_blessing: HashSet<PlayerId>,
    battlefield_flags: Arc<BattlefieldFlags>,

    combat_transients: Arc<CombatTransientState>,

    auxiliary_tracking: Arc<AuxiliaryTrackingState>,

    object_annotations: Arc<ObjectAnnotationStore>,

    cast_permission_flags: Arc<CastPermissionFlags>,

    commander_tracking: Arc<CommanderTracking>,

    exile_tracking: Arc<ExileTracking>,

    /// Whether required single-object choices with exactly one legal candidate
    /// may be resolved by the generic decision layer without surfacing a prompt.
    auto_choose_single_object_decisions: bool,
    runtime_cache: RuntimeCacheState,
}

impl Deref for GameState {
    type Target = ObjectStore;

    fn deref(&self) -> &Self::Target {
        &self.object_store
    }
}

impl DerefMut for GameState {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.object_store
    }
}

fn normalize_draft_note_card_name(name: &str) -> String {
    name.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase()
}

impl GameState {
    fn battlefield_flags_mut(&mut self) -> &mut BattlefieldFlags {
        Arc::make_mut(&mut self.battlefield_flags)
    }

    fn combat_transients_mut(&mut self) -> &mut CombatTransientState {
        Arc::make_mut(&mut self.combat_transients)
    }

    fn auxiliary_tracking_mut(&mut self) -> &mut AuxiliaryTrackingState {
        Arc::make_mut(&mut self.auxiliary_tracking)
    }

    fn choice_store_mut(&mut self) -> &mut ChoiceStore {
        Arc::make_mut(&mut self.choice_store)
    }

    fn cast_permission_flags_mut(&mut self) -> &mut CastPermissionFlags {
        Arc::make_mut(&mut self.cast_permission_flags)
    }

    fn object_annotations_mut(&mut self) -> &mut ObjectAnnotationStore {
        Arc::make_mut(&mut self.object_annotations)
    }

    fn commander_tracking_mut(&mut self) -> &mut CommanderTracking {
        Arc::make_mut(&mut self.commander_tracking)
    }

    fn exile_tracking_mut(&mut self) -> &mut ExileTracking {
        Arc::make_mut(&mut self.exile_tracking)
    }

    pub fn commander_objects(&self) -> &HashSet<ObjectId> {
        &self.commander_tracking.commanders
    }

    /// Creates a new game state with the given players.
    pub fn new(player_names: Vec<String>, starting_life: i32) -> Self {
        let players: Vec<Player> = player_names
            .into_iter()
            .enumerate()
            .map(|(i, name)| Player::new(PlayerId::from_index(i as u8), name, starting_life))
            .collect();

        let turn_order: Vec<PlayerId> = players.iter().map(|p| p.id).collect();
        let active_player = turn_order
            .first()
            .copied()
            .unwrap_or(PlayerId::from_index(0));

        Self {
            players,
            object_store: ObjectStore::default(),
            stack: Vec::new(),
            turn: TurnState::new(active_player),
            turn_store: TurnStore {
                turn_order,
                turn_history: TurnHistory::default(),
                ..TurnStore::default()
            },
            effect_store: EffectStore::default(),
            choice_store: Arc::new(ChoiceStore::default()),
            metadata: MetadataStateStore {
                ui_battlefield_transitions: im::Vector::new(),
                ui_zone_transitions: im::Vector::new(),
                next_ui_zone_transition_id: 0,
                ui_effect_events: im::Vector::new(),
                next_ui_effect_event_id: 0,
                provenance_graph: ProvenanceGraph::new(),
            },
            mutation_revision: 0,
            next_object_id: 1,
            zone_revisions: ZoneRevisionSnapshot::default(),
            combat: None,
            has_day_night: false,
            is_night: false,
            monarch: None,
            initiative: None,
            citys_blessing: HashSet::new(),
            battlefield_flags: Arc::new(BattlefieldFlags::default()),
            combat_transients: Arc::new(CombatTransientState::default()),
            auxiliary_tracking: Arc::new(AuxiliaryTrackingState::default()),
            object_annotations: Arc::new(ObjectAnnotationStore::default()),
            cast_permission_flags: Arc::new(CastPermissionFlags::default()),
            commander_tracking: Arc::new(CommanderTracking::default()),
            exile_tracking: Arc::new(ExileTracking::default()),
            auto_choose_single_object_decisions: true,
            runtime_cache: RuntimeCacheState::new(active_player),
        }
    }

    pub fn auto_choose_single_object_decisions(&self) -> bool {
        self.auto_choose_single_object_decisions
    }

    pub fn set_auto_choose_single_object_decisions(&mut self, enabled: bool) {
        self.auto_choose_single_object_decisions = enabled;
    }

    pub fn set_draft_noted_highest_number(
        &mut self,
        player: PlayerId,
        card_name: impl AsRef<str>,
        count: u32,
    ) {
        self.auxiliary_tracking_mut()
            .draft_noted_highest_numbers
            .insert(
                (player, normalize_draft_note_card_name(card_name.as_ref())),
                count,
            );
    }

    pub fn draft_noted_highest_number(&self, player: PlayerId, card_name: impl AsRef<str>) -> u32 {
        self.auxiliary_tracking
            .draft_noted_highest_numbers
            .get(&(player, normalize_draft_note_card_name(card_name.as_ref())))
            .copied()
            .unwrap_or(0)
    }

    pub fn note_life_total_for_source(
        &mut self,
        source: ObjectId,
        player: PlayerId,
    ) -> Option<i32> {
        let life_total = self.player(player)?.life;
        self.object_annotations_mut()
            .noted_life_totals
            .insert(source, life_total);
        Some(life_total)
    }

    pub fn noted_life_total_for_source(&self, source: ObjectId) -> Option<i32> {
        self.object_annotations
            .noted_life_totals
            .get(&source)
            .copied()
    }

    pub fn put_sticker_on_object(&mut self, object_id: ObjectId, action: KeywordActionKind) {
        let Some(stable_id) = self.object(object_id).map(|object| object.stable_id) else {
            return;
        };
        self.object_annotations_mut()
            .object_stickers
            .entry(stable_id)
            .or_default()
            .push(StickerMarker {
                action,
                name_letter_count: None,
            });
    }

    pub fn sticker_count_on_object(
        &self,
        object_id: ObjectId,
        action: KeywordActionKind,
        max_name_letters: Option<u32>,
    ) -> u32 {
        let Some(stable_id) = self.object(object_id).map(|object| object.stable_id) else {
            return 0;
        };
        self.object_annotations
            .object_stickers
            .get(&stable_id)
            .into_iter()
            .flatten()
            .filter(|sticker| {
                sticker.action == action
                    && max_name_letters.is_none_or(|max| {
                        sticker
                            .name_letter_count
                            .is_none_or(|letter_count| letter_count <= max)
                    })
            })
            .count() as u32
    }

    /// Creates a new game state after explicitly resetting runtime player/object IDs.
    ///
    /// Frontend/bootstrap paths should prefer this constructor when they need a
    /// fresh match identity space. Plain `new()` intentionally does not reset
    /// global counters so existing engine tests and embedded callers keep their
    /// current behavior.
    pub fn new_with_runtime_id_reset(player_names: Vec<String>, starting_life: i32) -> Self {
        reset_runtime_id_counters();
        Self::new(player_names, starting_life)
    }

    fn normalize_random_seed(seed: u64) -> u64 {
        if seed == 0 {
            0x9e37_79b9_7f4a_7c15
        } else {
            seed
        }
    }

    pub(crate) fn mark_continuous_state_dirty(&self) {
        self.runtime_cache.payment_restriction_presence.set(None);
        self.runtime_cache.continuous_context_revision.set(
            self.runtime_cache
                .continuous_context_revision
                .get()
                .saturating_add(1),
        );
        if self.runtime_cache.continuous_state_dirty.replace(true) {
            return;
        }
        self.runtime_cache.characteristics_cache.bump_epoch();
    }

    pub(crate) fn continuous_context_revision(&self) -> u64 {
        self.runtime_cache.continuous_context_revision.get()
    }

    fn mark_object_characteristics_dirty(&mut self, id: ObjectId) {
        let revision = self.bump_mutation_revision();
        self.runtime_cache
            .characteristics_cache
            .bump_object_revision(id, revision);
    }

    fn bump_mutation_revision(&mut self) -> u64 {
        self.mutation_revision = self.mutation_revision.saturating_add(1);
        self.mutation_revision
    }

    fn stamp_object_modified(&mut self, id: ObjectId) {
        let revision = self.bump_mutation_revision();
        if let Some(object) = self.object_store.object_mut(id) {
            object.last_modified = revision;
        }
        self.runtime_cache
            .characteristics_cache
            .bump_object_revision(id, revision);
    }

    fn bump_zone_revision(&mut self, zone: Zone) {
        self.zone_revisions.all = self.zone_revisions.all.saturating_add(1);
        match zone {
            Zone::Battlefield => {
                self.zone_revisions.battlefield = self.zone_revisions.battlefield.saturating_add(1);
            }
            Zone::Command => {
                self.zone_revisions.command = self.zone_revisions.command.saturating_add(1);
            }
            Zone::Exile => {
                self.zone_revisions.exile = self.zone_revisions.exile.saturating_add(1);
            }
            Zone::Library => {
                self.zone_revisions.library = self.zone_revisions.library.saturating_add(1);
            }
            Zone::Hand => {
                self.zone_revisions.hand = self.zone_revisions.hand.saturating_add(1);
            }
            Zone::Graveyard => {
                self.zone_revisions.graveyard = self.zone_revisions.graveyard.saturating_add(1);
            }
            Zone::OutsideGame => {
                self.zone_revisions.outside_game =
                    self.zone_revisions.outside_game.saturating_add(1);
            }
            Zone::Stack => {
                self.zone_revisions.stack = self.zone_revisions.stack.saturating_add(1);
            }
        }
    }

    pub fn mutation_revision(&self) -> u64 {
        self.mutation_revision
    }

    pub fn zone_revisions(&self) -> ZoneRevisionSnapshot {
        self.zone_revisions
    }

    pub fn work_counters(&self) -> WorkCounterSnapshot {
        self.runtime_cache.work_counters.snapshot()
    }

    pub(crate) fn note_dependency_sort(&self) {
        self.runtime_cache.work_counters.bump_dependency_sorts();
    }

    pub(crate) fn note_dependency_pair_probed(&self) {
        self.runtime_cache
            .work_counters
            .bump_dependency_pairs_probed();
    }

    pub(crate) fn count_static_ability_regen(&self) {
        self.runtime_cache
            .work_counters
            .bump_static_ability_regens();
    }

    pub(crate) fn count_sba_scan_objects(&self, count: usize) {
        self.runtime_cache
            .work_counters
            .add_objects_scanned_in_sba(count as u64);
    }

    pub(crate) fn count_derived_view_rebuild(&self) {
        self.runtime_cache
            .work_counters
            .bump_derived_view_rebuilds();
    }

    pub(crate) fn cached_trigger_registry(
        &self,
        key: crate::triggers::check::TriggerRegistryKey,
        build: impl FnOnce() -> crate::triggers::check::TriggerRegistry,
    ) -> crate::triggers::check::TriggerRegistry {
        let mut cached = self.runtime_cache.trigger_registry.borrow_mut();
        if cached.as_ref().is_none_or(|registry| registry.key != key) {
            *cached = Some(build());
        }
        cached
            .as_ref()
            .expect("trigger registry cache should be populated")
            .clone()
    }

    fn mark_continuous_state_clean(&self) {
        if !self.cached_continuous_turn_state_matches_current() {
            self.runtime_cache.characteristics_cache.bump_epoch();
        }
        self.runtime_cache.continuous_state_dirty.set(false);
        self.runtime_cache
            .continuous_state_revision
            .set(self.effect_store.continuous_effects.revision());
        self.runtime_cache
            .continuous_state_turn_number
            .set(self.turn.turn_number);
        self.runtime_cache
            .continuous_state_active_player
            .set(self.turn.active_player);
        self.runtime_cache
            .continuous_state_phase
            .set(self.turn.phase);
        self.runtime_cache.continuous_state_step.set(self.turn.step);
    }

    fn cached_continuous_turn_state_matches_current(&self) -> bool {
        if self.runtime_cache.continuous_state_turn_number.get() == self.turn.turn_number
            && self.runtime_cache.continuous_state_active_player.get() == self.turn.active_player
            && self.runtime_cache.continuous_state_phase.get() == self.turn.phase
            && self.runtime_cache.continuous_state_step.get() == self.turn.step
        {
            return true;
        }

        !self.cached_continuous_effects_are_turn_context_sensitive()
    }

    pub(crate) fn continuous_state_is_clean(&self) -> bool {
        !self.runtime_cache.continuous_state_dirty.get()
            && self.runtime_cache.continuous_state_revision.get()
                == self.effect_store.continuous_effects.revision()
            && self.cached_continuous_turn_state_matches_current()
    }

    fn cached_continuous_effects_are_turn_context_sensitive(&self) -> bool {
        let revision = self.effect_store.continuous_effects.revision();
        if let Some((cached_revision, sensitive)) = self.runtime_cache.turn_sensitivity.get()
            && cached_revision == revision
        {
            return sensitive;
        }
        let sensitive = self
            .cached_continuous_effects_snapshot_arc()
            .iter()
            .any(Self::continuous_effect_is_turn_context_sensitive);
        self.runtime_cache
            .turn_sensitivity
            .set(Some((revision, sensitive)));
        sensitive
    }

    fn continuous_effect_is_turn_context_sensitive(effect: &ContinuousEffect) -> bool {
        !matches!(effect.duration, Until::Forever)
            || effect.condition.is_some()
            || Self::effect_target_is_turn_context_sensitive(&effect.applies_to)
            || Self::modification_is_turn_context_sensitive(&effect.modification)
    }

    fn effect_target_is_turn_context_sensitive(target: &EffectTarget) -> bool {
        match target {
            EffectTarget::Filter(filter) => Self::object_filter_is_turn_context_sensitive(filter),
            _ => false,
        }
    }

    fn modification_is_turn_context_sensitive(modification: &Modification) -> bool {
        match modification {
            Modification::SetPower { value, .. } | Modification::SetToughness { value, .. } => {
                Self::value_is_turn_context_sensitive(value)
            }
            Modification::SetPowerToughness {
                power, toughness, ..
            } => {
                Self::value_is_turn_context_sensitive(power)
                    || Self::value_is_turn_context_sensitive(toughness)
            }
            _ => false,
        }
    }

    fn value_is_turn_context_sensitive(value: &crate::effect::Value) -> bool {
        match value {
            crate::effect::Value::SurfaceHinted { value, .. }
            | crate::effect::Value::Scaled(value, _)
            | crate::effect::Value::DividedRoundedDown(value, _)
            | crate::effect::Value::HalfRoundedDown(value) => {
                Self::value_is_turn_context_sensitive(value)
            }
            crate::effect::Value::Add(left, right) | crate::effect::Value::Min(left, right) => {
                Self::value_is_turn_context_sensitive(left)
                    || Self::value_is_turn_context_sensitive(right)
            }
            crate::effect::Value::Count(filter)
            | crate::effect::Value::CountScaled(filter, _)
            | crate::effect::Value::GreatestCount(filter)
            | crate::effect::Value::TotalPower(filter)
            | crate::effect::Value::TotalToughness(filter)
            | crate::effect::Value::TotalManaValue(filter)
            | crate::effect::Value::GreatestPower(filter)
            | crate::effect::Value::GreatestToughness(filter)
            | crate::effect::Value::GreatestManaValue(filter)
            | crate::effect::Value::BasicLandTypesAmong(filter)
            | crate::effect::Value::CreatureTypesAmong(filter)
            | crate::effect::Value::CardTypesAmong(filter)
            | crate::effect::Value::ColorsAmong(filter)
            | crate::effect::Value::DistinctNames(filter)
            | crate::effect::Value::DistinctPowers(filter)
            | crate::effect::Value::PlayersWhoControlMoreThanYou(filter)
            | crate::effect::Value::StaticAbilitiesAmong { filter, .. } => {
                Self::object_filter_is_turn_context_sensitive(filter)
            }
            crate::effect::Value::CreaturesDiedThisTurn
            | crate::effect::Value::CreaturesDiedThisTurnControlledBy(_)
            | crate::effect::Value::PlayersBeingAttacked
            | crate::effect::Value::LifeTotalAsTurnBegan(_)
            | crate::effect::Value::LifeGainedThisTurn(_)
            | crate::effect::Value::LifeLostThisTurn(_)
            | crate::effect::Value::CardsDiscardedThisTurn(_)
            | crate::effect::Value::DamageDealtToPlayersThisTurn(_)
            | crate::effect::Value::NoncombatDamageDealtToPlayersThisTurn(_)
            | crate::effect::Value::NoncombatDamageDealtBySourcesControlledThisTurn { .. }
            | crate::effect::Value::MaxCardsDrawnThisTurn(_)
            | crate::effect::Value::MaxDiceRolledThisTurn(_)
            | crate::effect::Value::LandsEnteredBattlefieldThisTurn(_)
            | crate::effect::Value::SpellsCastThisTurn(_)
            | crate::effect::Value::SpellsCastBeforeThisTurn(_)
            | crate::effect::Value::SpellsCastThisTurnMatching { .. }
            | crate::effect::Value::CommanderCastCount(_)
            | crate::effect::Value::ThisAbilityResolvedThisTurnCount
            | crate::effect::Value::SourceRegeneratedThisTurnCount
            | crate::effect::Value::DamageDealtThisTurnByTaggedSpellCast(_) => true,
            crate::effect::Value::CountPlayers(player)
            | crate::effect::Value::PartySize(player)
            | crate::effect::Value::LifeTotal(player)
            | crate::effect::Value::LifeTotalDifference(player)
            | crate::effect::Value::UnspentMana(player)
            | crate::effect::Value::Speed(player)
            | crate::effect::Value::StartingLifeTotal(player)
            | crate::effect::Value::HalfLifeTotalRoundedUp(player)
            | crate::effect::Value::HalfLifeTotalRoundedDown(player)
            | crate::effect::Value::HalfStartingLifeTotalRoundedUp(player)
            | crate::effect::Value::HalfStartingLifeTotalRoundedDown(player)
            | crate::effect::Value::CardsInHand(player)
            | crate::effect::Value::CardsInLibrary(player)
            | crate::effect::Value::DevotionToChosenColor(player)
            | crate::effect::Value::MaxCardsInHand(player)
            | crate::effect::Value::CardsInGraveyard(player)
            | crate::effect::Value::CardTypesInGraveyard(player)
            | crate::effect::Value::Devotion { player, .. } => {
                Self::player_filter_is_turn_context_sensitive(player)
            }
            _ => false,
        }
    }

    fn object_filter_is_turn_context_sensitive(filter: &crate::target::ObjectFilter) -> bool {
        filter.cast_this_turn
            || filter.first_spell_cast_each_turn
            || filter.attacking
            || filter.attacked_this_turn
            || filter.nonattacking
            || filter.enlist_eligible
            || filter.blocking
            || filter.nonblocking
            || filter.blocked
            || filter.blocked_by.is_some()
            || filter.blocked_by_source
            || filter.unblocked
            || filter.in_combat_with_source
            || filter.entered_since_your_last_turn_ended
            || filter.entered_battlefield_this_turn
            || filter.entered_graveyard_this_turn
            || filter.entered_graveyard_from_battlefield_this_turn
            || filter.surveilled_this_turn
            || filter.was_dealt_damage_this_turn
            || filter.dealt_damage_by_source_this_turn.is_some()
            || filter.drawn_this_turn
            || Self::player_filter_option_is_turn_context_sensitive(filter.controller.as_ref())
            || Self::player_filter_option_is_turn_context_sensitive(filter.cast_by.as_ref())
            || Self::player_filter_option_is_turn_context_sensitive(filter.owner.as_ref())
            || Self::player_filter_option_is_turn_context_sensitive(filter.targets_player.as_ref())
            || Self::player_filter_option_is_turn_context_sensitive(
                filter.targets_only_player.as_ref(),
            )
            || Self::player_filter_option_is_turn_context_sensitive(
                filter
                    .attacking_player_or_planeswalker_controlled_by
                    .as_ref(),
            )
            || Self::player_filter_option_is_turn_context_sensitive(
                filter.attached_to_player.as_ref(),
            )
            || filter
                .attached_to_object
                .as_deref()
                .is_some_and(Self::object_filter_is_turn_context_sensitive)
            || Self::player_filter_option_is_turn_context_sensitive(
                filter.entered_battlefield_controller.as_ref(),
            )
            || Self::player_filter_option_is_turn_context_sensitive(
                filter.discarded_or_cycled_this_turn_by.as_ref(),
            )
            || Self::player_filter_option_is_turn_context_sensitive(
                filter.dealt_damage_to_player_this_turn.as_ref(),
            )
            || filter
                .targets_object
                .as_deref()
                .is_some_and(Self::object_filter_is_turn_context_sensitive)
            || filter
                .targets_only_object
                .as_deref()
                .is_some_and(Self::object_filter_is_turn_context_sensitive)
            || filter
                .no_shared_creature_types_with
                .iter()
                .any(Self::object_filter_is_turn_context_sensitive)
            || filter
                .any_of
                .iter()
                .any(Self::object_filter_is_turn_context_sensitive)
    }

    fn player_filter_option_is_turn_context_sensitive(
        filter: Option<&crate::target::PlayerFilter>,
    ) -> bool {
        filter.is_some_and(Self::player_filter_is_turn_context_sensitive)
    }

    fn player_filter_is_turn_context_sensitive(filter: &crate::target::PlayerFilter) -> bool {
        match filter {
            crate::target::PlayerFilter::Active
            | crate::target::PlayerFilter::Attacking
            | crate::target::PlayerFilter::Defending
            | crate::target::PlayerFilter::CastCardTypeThisTurn(_) => true,
            crate::target::PlayerFilter::CardsInHandAtLeastMoreThanYou { base, .. }
            | crate::target::PlayerFilter::HasMoreLifeThanYou { base }
            | crate::target::PlayerFilter::MaxSpeed { base, .. }
            | crate::target::PlayerFilter::Target(base) => {
                Self::player_filter_is_turn_context_sensitive(base)
            }
            crate::target::PlayerFilter::Excluding { base, excluded } => {
                Self::player_filter_is_turn_context_sensitive(base)
                    || Self::player_filter_is_turn_context_sensitive(excluded)
            }
            _ => false,
        }
    }

    /// Set the deterministic RNG seed for this match.
    pub fn set_random_seed(&mut self, seed: u64) {
        self.runtime_cache
            .random_state
            .set(Self::normalize_random_seed(seed));
    }

    /// Return the current deterministic RNG state.
    pub fn random_seed(&self) -> u64 {
        self.runtime_cache.random_state.get()
    }

    /// Return the count of irreversible random gameplay operations that have occurred.
    pub fn irreversible_random_count(&self) -> u64 {
        self.runtime_cache.irreversible_random_count.get()
    }

    pub fn crypto_audit_checkpoint(&self) -> usize {
        self.runtime_cache.hidden_info_audit_log.borrow().len()
    }

    pub fn crypto_audit_operations_since(&self, checkpoint: usize) -> Vec<HiddenInfoOperation> {
        let log = self.runtime_cache.hidden_info_audit_log.borrow();
        if checkpoint >= log.len() {
            return Vec::new();
        }
        log.iter().skip(checkpoint).cloned().collect()
    }

    fn push_hidden_info_operation(&self, operation: HiddenInfoOperation) {
        self.runtime_cache
            .hidden_info_audit_log
            .borrow_mut()
            .push_back(operation);
    }

    /// Queue a deterministic die result for test harnesses that mirror external fixtures.
    pub fn force_next_die_roll(&mut self, result: u32) {
        self.runtime_cache
            .forced_die_rolls
            .borrow_mut()
            .push_back(result);
    }

    /// Consume a queued die result, if one was supplied by a test harness.
    pub fn take_forced_die_roll(&self) -> Option<u32> {
        self.runtime_cache.forced_die_rolls.borrow_mut().pop_front()
    }

    /// Queue transcript-derived random seeds supplied by the multiplayer audit
    /// protocol. These seeds are consumed before deterministic local RNG output.
    pub fn queue_transcript_random_seeds<I>(&self, seeds: I)
    where
        I: IntoIterator<Item = u64>,
    {
        self.runtime_cache
            .transcript_random_seeds
            .borrow_mut()
            .extend(seeds.into_iter().map(Self::normalize_random_seed));
    }

    /// Queue an externally verified library order for the next shuffle of that
    /// player's library. The order is expressed in the same object-id space as
    /// the matching transcript requirement; at shuffle time it is localized to
    /// the live pre-shuffle library by before-order position.
    pub fn queue_transcript_library_shuffle_order(
        &self,
        player: PlayerId,
        before_order: Vec<ObjectId>,
        after_order: Vec<ObjectId>,
    ) {
        if before_order.is_empty() || before_order.len() != after_order.len() {
            return;
        }
        self.runtime_cache
            .transcript_library_shuffle_orders
            .borrow_mut()
            .push_back(TranscriptLibraryShuffleOrder {
                player,
                before_order,
                after_order,
            });
    }

    fn take_transcript_library_shuffle_order(
        &self,
        player: PlayerId,
    ) -> Option<TranscriptLibraryShuffleOrder> {
        let mut orders = self
            .runtime_cache
            .transcript_library_shuffle_orders
            .borrow_mut();
        let index = orders.iter().position(|order| order.player == player)?;
        orders.remove(index)
    }

    fn record_irreversible_random(&self) -> (u64, u64) {
        let before = self.runtime_cache.irreversible_random_count.get();
        let after = before.wrapping_add(1);
        self.runtime_cache.irreversible_random_count.set(after);
        (before, after)
    }

    /// Advance the deterministic RNG and return the next 64 random bits.
    pub fn next_random_u64(&self) -> u64 {
        if let Some(seed) = self
            .runtime_cache
            .transcript_random_seeds
            .borrow_mut()
            .pop_front()
        {
            self.runtime_cache.random_state.set(seed);
            return seed;
        }
        let mut z = self
            .runtime_cache
            .random_state
            .get()
            .wrapping_add(0x9e37_79b9_7f4a_7c15);
        self.runtime_cache.random_state.set(z);
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    /// Shuffle a slice using the deterministic match RNG.
    pub fn shuffle_slice<T>(&self, values: &mut [T]) {
        let (random_count_before, random_count_after) = self.record_irreversible_random();
        let mut rng = ChaCha12Rng::seed_from_u64(self.next_random_u64());
        values.shuffle(&mut rng);
        self.push_hidden_info_operation(HiddenInfoOperation::FairRandom {
            random_count_before,
            random_count_after,
            reason: "runtime consumed irreversible random output".to_string(),
        });
    }

    /// Shuffle a player's library using the deterministic match RNG.
    pub fn shuffle_player_library(&mut self, player_id: PlayerId) {
        let (random_count_before, random_count_after) = self.record_irreversible_random();
        let seed = self.next_random_u64();
        let Some(index) = self
            .players
            .iter()
            .position(|player| player.id == player_id)
        else {
            self.push_hidden_info_operation(HiddenInfoOperation::FairRandom {
                random_count_before,
                random_count_after,
                reason: "shuffle requested for missing player".to_string(),
            });
            return;
        };
        let before_order = self.players[index].library.clone();
        if let Some(transcript_order) = self.take_transcript_library_shuffle_order(player_id) {
            let mut id_map = HashMap::with_capacity(before_order.len());
            if transcript_order.before_order.len() == before_order.len()
                && transcript_order.after_order.len() == before_order.len()
            {
                for (transcript_id, live_id) in transcript_order
                    .before_order
                    .iter()
                    .copied()
                    .zip(before_order.iter().copied())
                {
                    id_map.insert(transcript_id, live_id);
                }
                let localized_after = transcript_order
                    .after_order
                    .iter()
                    .copied()
                    .filter_map(|id| id_map.get(&id).copied())
                    .collect::<Vec<_>>();
                let before_set = before_order.iter().copied().collect::<HashSet<_>>();
                let after_set = localized_after.iter().copied().collect::<HashSet<_>>();
                if localized_after.len() == before_order.len()
                    && after_set.len() == before_set.len()
                    && before_set.iter().all(|id| after_set.contains(id))
                {
                    self.players[index].library = localized_after;
                } else {
                    let mut rng = ChaCha12Rng::seed_from_u64(seed);
                    self.players[index].library.shuffle(&mut rng);
                }
            } else {
                let mut rng = ChaCha12Rng::seed_from_u64(seed);
                self.players[index].library.shuffle(&mut rng);
            }
        } else {
            let mut rng = ChaCha12Rng::seed_from_u64(seed);
            self.players[index].library.shuffle(&mut rng);
        }
        let after_order = self.players[index].library.clone();
        if before_order.last() != after_order.last() {
            self.bump_library_top_revision(player_id);
        }
        self.push_hidden_info_operation(HiddenInfoOperation::LibraryShuffle {
            player: player_id,
            before_order,
            after_order,
            random_count_before,
            random_count_after,
        });
    }

    fn same_object_multiset(left: &[ObjectId], right: &[ObjectId]) -> bool {
        if left.len() != right.len() {
            return false;
        }
        let mut counts: HashMap<ObjectId, usize> = HashMap::new();
        for &id in left {
            *counts.entry(id).or_default() += 1;
        }
        for &id in right {
            let Some(count) = counts.get_mut(&id) else {
                return false;
            };
            if *count == 0 {
                return false;
            }
            *count -= 1;
        }
        counts.values().all(|count| *count == 0)
    }

    fn record_library_reorder(
        &self,
        player: PlayerId,
        before_order: Vec<ObjectId>,
        after_order: Vec<ObjectId>,
        reason: impl Into<String>,
    ) {
        if before_order == after_order || !Self::same_object_multiset(&before_order, &after_order) {
            return;
        }
        self.push_hidden_info_operation(HiddenInfoOperation::LibraryReorder {
            player,
            before_order,
            after_order,
            reason: reason.into(),
        });
    }

    pub fn set_player_library_order_with_audit(
        &mut self,
        player: PlayerId,
        after_order: Vec<ObjectId>,
        reason: impl Into<String>,
    ) -> bool {
        let Some(before_order) = self.player(player).map(|player| player.library.clone()) else {
            return false;
        };
        if !Self::same_object_multiset(&before_order, &after_order) {
            return false;
        }
        if let Some(player_state) = self.player_mut(player) {
            player_state.library = after_order.clone();
        }
        if before_order.last() != after_order.last() {
            self.bump_library_top_revision(player);
        }
        self.record_library_reorder(player, before_order, after_order, reason);
        true
    }

    pub fn move_library_card_to_top(
        &mut self,
        player: PlayerId,
        card: ObjectId,
        reason: impl Into<String>,
    ) -> bool {
        let Some(before_order) = self.player(player).map(|player| player.library.clone()) else {
            return false;
        };
        if !before_order.contains(&card) {
            return false;
        }
        let mut after_order: Vec<_> = before_order
            .iter()
            .copied()
            .filter(|id| *id != card)
            .collect();
        after_order.push(card);
        self.set_player_library_order_with_audit(player, after_order, reason)
    }

    pub fn move_library_card_to_bottom(
        &mut self,
        player: PlayerId,
        card: ObjectId,
        reason: impl Into<String>,
    ) -> bool {
        let Some(before_order) = self.player(player).map(|player| player.library.clone()) else {
            return false;
        };
        if !before_order.contains(&card) {
            return false;
        }
        let mut after_order = Vec::with_capacity(before_order.len());
        after_order.push(card);
        after_order.extend(before_order.iter().copied().filter(|id| *id != card));
        self.set_player_library_order_with_audit(player, after_order, reason)
    }

    pub fn move_library_card_to_nth_from_top(
        &mut self,
        player: PlayerId,
        card: ObjectId,
        position_from_top: usize,
        reason: impl Into<String>,
    ) -> bool {
        let Some(before_order) = self.player(player).map(|player| player.library.clone()) else {
            return false;
        };
        if !before_order.contains(&card) {
            return false;
        }
        let mut after_order: Vec<_> = before_order
            .iter()
            .copied()
            .filter(|id| *id != card)
            .collect();
        let position = position_from_top.max(1);
        let insert_idx = after_order.len().saturating_sub(position - 1);
        after_order.insert(insert_idx, card);
        self.set_player_library_order_with_audit(player, after_order, reason)
    }

    pub fn shuffle_library_except_then_insert_from_top(
        &mut self,
        player: PlayerId,
        cards_in_insert_order: &[ObjectId],
        position_from_top: usize,
        reason: impl Into<String>,
    ) -> bool {
        let Some(before_order) = self.player(player).map(|player| player.library.clone()) else {
            return false;
        };
        if cards_in_insert_order.is_empty() {
            self.shuffle_player_library(player);
            return true;
        }
        let selected: Vec<_> = cards_in_insert_order
            .iter()
            .copied()
            .filter(|id| before_order.contains(id))
            .collect();
        if selected.is_empty() {
            self.shuffle_player_library(player);
            return true;
        }
        let selected_set: HashSet<_> = selected.iter().copied().collect();
        if let Some(player_state) = self.player_mut(player) {
            player_state.library.retain(|id| !selected_set.contains(id));
        }
        self.shuffle_player_library(player);
        let after_order = if let Some(player_state) = self.player_mut(player) {
            let position = position_from_top.max(1);
            let insert_idx = player_state.library.len().saturating_sub(position - 1);
            player_state
                .library
                .splice(insert_idx..insert_idx, selected.iter().copied());
            player_state.library.clone()
        } else {
            return false;
        };
        if before_order.last() != after_order.last() {
            self.bump_library_top_revision(player);
        }
        self.record_library_reorder(player, before_order, after_order, reason);
        true
    }

    pub fn shuffle_library_except_then_put_on_top(
        &mut self,
        player: PlayerId,
        top_cards_in_push_order: &[ObjectId],
        reason: impl Into<String>,
    ) -> bool {
        self.shuffle_library_except_then_insert_from_top(player, top_cards_in_push_order, 1, reason)
    }

    /// Generates a new unique object ID.
    pub fn new_object_id(&mut self) -> ObjectId {
        let id = ObjectId::from_raw(self.next_object_id);
        self.next_object_id = self.next_object_id.saturating_add(1);
        id
    }

    pub fn next_object_id_counter(&self) -> u64 {
        self.next_object_id
    }

    pub fn set_next_object_id_counter(&mut self, next_object_id: u64) {
        self.next_object_id = next_object_id.max(1);
    }

    pub fn add_restriction_effect(
        &mut self,
        restriction: crate::effect::Restriction,
        duration: crate::effect::Until,
        source: ObjectId,
        controller: PlayerId,
        iterated_player: Option<PlayerId>,
    ) {
        self.add_restriction_effect_with_tagged_objects(
            restriction,
            duration,
            source,
            controller,
            iterated_player,
            HashMap::new(),
        );
    }

    pub fn add_restriction_effect_with_tagged_objects(
        &mut self,
        restriction: crate::effect::Restriction,
        duration: crate::effect::Until,
        source: ObjectId,
        controller: PlayerId,
        iterated_player: Option<PlayerId>,
        tagged_objects: HashMap<crate::tag::TagKey, Vec<ObjectSnapshot>>,
    ) {
        let expires_end_of_turn = match duration {
            crate::effect::Until::EndOfTurn => self.turn.turn_number,
            crate::effect::Until::Forever => u32::MAX,
            _ => self.turn.turn_number,
        };

        self.effect_store
            .restriction_effects
            .push(RestrictionEffectInstance {
                restriction,
                controller,
                source,
                iterated_player,
                tagged_objects,
                duration,
                expires_end_of_turn,
                consumed_next_untap: false,
            });
    }

    pub fn add_goad_effect(
        &mut self,
        creature: ObjectId,
        goaded_by: PlayerId,
        duration: crate::effect::Until,
        source: ObjectId,
    ) {
        let current_turn = self.turn.turn_number;
        if self.effect_store.goad_effects.iter().any(|effect| {
            effect.creature == creature
                && effect.goaded_by == goaded_by
                && effect.is_active(self, current_turn)
        }) {
            return;
        }

        let expires_end_of_turn = match duration {
            crate::effect::Until::EndOfTurn => self.turn.turn_number,
            crate::effect::Until::Forever => u32::MAX,
            _ => self.turn.turn_number,
        };

        self.effect_store.goad_effects.push(GoadEffectInstance {
            creature,
            goaded_by,
            source,
            duration,
            expires_end_of_turn,
        });
    }

    pub fn add_temporary_spell_cost_reduction(
        &mut self,
        player: PlayerId,
        source: ObjectId,
        filter: crate::target::ObjectFilter,
        reduction: crate::mana::ManaCost,
        remaining_uses: u32,
    ) {
        self.add_temporary_spell_cost_reduction_until(
            player,
            source,
            player,
            filter,
            reduction,
            remaining_uses,
            crate::effect::Until::EndOfTurn,
        );
    }

    pub fn add_temporary_spell_cost_reduction_until(
        &mut self,
        player: PlayerId,
        source: ObjectId,
        duration_controller: PlayerId,
        filter: crate::target::ObjectFilter,
        reduction: crate::mana::ManaCost,
        remaining_uses: u32,
        duration: crate::effect::Until,
    ) {
        self.effect_store.temporary_spell_cost_reductions.push(
            TemporarySpellCostReductionEffectInstance {
                player,
                source,
                duration_controller,
                filter,
                reduction,
                generic_reduction: None,
                applies_to_all_matching_this_turn: false,
                duration,
                remaining_uses,
                expires_end_of_turn: self.turn.turn_number,
            },
        );
    }

    pub fn add_temporary_matching_spell_cost_reduction_this_turn(
        &mut self,
        player: PlayerId,
        source: ObjectId,
        filter: crate::target::ObjectFilter,
        generic_reduction: crate::effect::Value,
    ) {
        self.add_temporary_matching_spell_cost_reduction_until(
            player,
            source,
            player,
            filter,
            generic_reduction,
            crate::effect::Until::EndOfTurn,
        );
    }

    pub fn add_temporary_matching_spell_cost_reduction_until(
        &mut self,
        player: PlayerId,
        source: ObjectId,
        duration_controller: PlayerId,
        filter: crate::target::ObjectFilter,
        generic_reduction: crate::effect::Value,
        duration: crate::effect::Until,
    ) {
        self.effect_store.temporary_spell_cost_reductions.push(
            TemporarySpellCostReductionEffectInstance {
                player,
                source,
                duration_controller,
                filter,
                reduction: crate::mana::ManaCost::new(),
                generic_reduction: Some(generic_reduction),
                applies_to_all_matching_this_turn: true,
                duration,
                remaining_uses: u32::MAX,
                expires_end_of_turn: self.turn.turn_number,
            },
        );
    }

    pub fn add_temporary_spell_ability_grant(
        &mut self,
        player: PlayerId,
        source: ObjectId,
        filter: crate::target::ObjectFilter,
        ability: crate::static_abilities::StaticAbility,
        remaining_uses: u32,
    ) {
        self.effect_store.temporary_spell_ability_grants.push(
            TemporarySpellAbilityGrantEffectInstance {
                player,
                source,
                filter,
                ability,
                remaining_uses,
                expires_end_of_turn: self.turn.turn_number,
            },
        );
    }

    pub fn grant_temporary_static_ability_to_object_until_end_of_turn(
        &mut self,
        object_id: ObjectId,
        ability: crate::static_abilities::StaticAbilityId,
    ) {
        self.grant_temporary_static_ability_payload_to_object_until_end_of_turn(
            object_id, ability, None,
        );
    }

    pub fn grant_temporary_static_ability_payload_to_object_until_end_of_turn(
        &mut self,
        object_id: ObjectId,
        ability: crate::static_abilities::StaticAbilityId,
        ability_payload: Option<crate::static_abilities::StaticAbility>,
    ) {
        let expires_end_of_turn = self.turn.turn_number;
        let Some(object) = self.object_mut(object_id) else {
            return;
        };
        if object.temporary_static_ability_grants.iter().any(|grant| {
            grant.ability == ability
                && grant.ability_payload == ability_payload
                && grant.expires_end_of_turn >= expires_end_of_turn
        }) {
            return;
        }
        object
            .temporary_static_ability_grants
            .push(crate::object::TemporaryStaticAbilityGrant {
                ability,
                ability_payload,
                expires_end_of_turn,
            });
    }

    pub fn temporary_granted_spell_abilities(
        &self,
        spell_id: ObjectId,
        player: PlayerId,
    ) -> Vec<crate::static_abilities::StaticAbility> {
        let Some(spell_obj) = self.object(spell_id).cloned() else {
            return Vec::new();
        };
        let current_turn = self.turn.turn_number;
        let ctx = crate::filter::FilterContext::new(player)
            .with_source(spell_id)
            .with_active_player(self.turn.active_player)
            .with_opponents(
                self.turn_store
                    .turn_order
                    .iter()
                    .copied()
                    .filter(|player_id| *player_id != player)
                    .collect(),
            )
            .with_caster(Some(player));
        self.effect_store
            .temporary_spell_ability_grants
            .iter()
            .filter(|effect| {
                effect.player == player
                    && !effect.is_expired(current_turn)
                    && self.temporary_spell_filter_matches(
                        &effect.filter,
                        spell_id,
                        &spell_obj,
                        &ctx,
                    )
            })
            .map(|effect| effect.ability.clone())
            .collect()
    }

    pub fn consume_temporary_spell_ability_grants_for_spell(
        &mut self,
        spell_id: ObjectId,
        player: PlayerId,
    ) {
        let Some(spell_obj) = self.object(spell_id).cloned() else {
            return;
        };
        let current_turn = self.turn.turn_number;
        let ctx = crate::filter::FilterContext::new(player)
            .with_source(spell_id)
            .with_active_player(self.turn.active_player)
            .with_opponents(
                self.turn_store
                    .turn_order
                    .iter()
                    .copied()
                    .filter(|player_id| *player_id != player)
                    .collect(),
            )
            .with_caster(Some(player));
        let matching = self
            .effect_store
            .temporary_spell_ability_grants
            .iter()
            .enumerate()
            .filter_map(|(idx, effect)| {
                (effect.player == player
                    && !effect.is_expired(current_turn)
                    && self.temporary_spell_filter_matches(
                        &effect.filter,
                        spell_id,
                        &spell_obj,
                        &ctx,
                    ))
                .then_some(idx)
            })
            .collect::<Vec<_>>();
        let granted_abilities = matching
            .iter()
            .filter_map(|idx| {
                self.effect_store
                    .temporary_spell_ability_grants
                    .get(*idx)
                    .map(|effect| effect.ability.clone())
            })
            .collect::<Vec<_>>();
        if let Some(spell) = self.object_mut(spell_id) {
            for ability in granted_abilities {
                let already_present = spell.abilities.iter().any(|existing| {
                    matches!(
                        &existing.kind,
                        crate::ability::AbilityKind::Static(static_ability)
                            if static_ability.id() == ability.id()
                    )
                });
                if !already_present {
                    spell
                        .abilities_mut()
                        .push(crate::ability::Ability::static_ability(ability));
                }
            }
        }
        for idx in matching {
            if let Some(effect) = self
                .effect_store
                .temporary_spell_ability_grants
                .get_mut(idx)
                && effect.remaining_uses > 0
            {
                effect.remaining_uses -= 1;
            }
        }
    }

    /// Match a temporary next-spell filter against either the stack object or
    /// its immutable cast-origin snapshot.  Origin clauses ("from your hand",
    /// "from exile", and so on) are facts about the pre-cast card, not the
    /// current stack object's zone, and must not be approximated by ownership.
    fn temporary_spell_filter_matches(
        &self,
        filter: &crate::target::ObjectFilter,
        spell_id: ObjectId,
        spell: &crate::object::Object,
        ctx: &crate::filter::FilterContext,
    ) -> bool {
        filter.matches(spell, ctx, self)
            || (spell.zone == Zone::Stack
                && self
                    .cast_origin_snapshot(spell_id)
                    .is_some_and(|snapshot| filter.matches_snapshot(snapshot, ctx, self)))
    }

    pub fn active_goaders_for(&self, creature: ObjectId) -> HashSet<PlayerId> {
        let current_turn = self.turn.turn_number;
        let mut goaders: HashSet<PlayerId> = self
            .effect_store
            .goad_effects
            .iter()
            .filter(|effect| effect.creature == creature && effect.is_active(self, current_turn))
            .map(|effect| effect.goaded_by)
            .collect();

        let view = DerivedGameView::new(self);
        let static_abilities = view
            .calculated_characteristics(creature)
            .map(|chars| chars.static_abilities)
            .or_else(|| {
                self.object(creature).map(|object| {
                    object
                        .abilities
                        .iter()
                        .filter_map(|ability| match &ability.kind {
                            AbilityKind::Static(static_ability) => Some(static_ability.clone()),
                            _ => None,
                        })
                        .collect()
                })
            })
            .unwrap_or_default();

        if let Some(object) = self.object(creature) {
            let controller = self.controller_of(object);
            for ability in static_abilities {
                if let Some(player) = ability.goaded_by_player(self, creature, controller) {
                    goaders.insert(player);
                }
            }
        }

        goaders
    }

    pub fn is_goaded(&self, creature: ObjectId) -> bool {
        !self.active_goaders_for(creature).is_empty()
    }

    pub fn cleanup_restrictions_end_of_turn(&mut self) {
        let current_turn = self.turn.turn_number;
        self.effect_store.restriction_effects.retain(|effect| {
            !matches!(effect.duration, crate::effect::Until::EndOfTurn)
                || effect.expires_end_of_turn > current_turn
        });
    }

    pub fn cleanup_restrictions_end_of_combat(&mut self) {
        let before = self.effect_store.restriction_effects.len();
        self.effect_store
            .restriction_effects
            .retain(|effect| !matches!(effect.duration, crate::effect::Until::EndOfCombat));
        if self.effect_store.restriction_effects.len() != before {
            self.update_cant_effects();
        }
    }

    pub fn cleanup_combat_damage_assignment_suppressions_end_of_combat(&mut self) {
        self.turn_store.no_combat_damage_this_combat.clear();
    }

    pub fn cleanup_granted_mana_abilities_end_of_turn(&mut self) {
        let current_turn = self.turn.turn_number;
        self.effect_store
            .granted_mana_abilities
            .retain(|grant| grant.expires_end_of_turn > current_turn);
    }

    pub fn cleanup_mana_spend_permissions_end_of_turn(&mut self) {
        self.effect_store
            .mana_spend_effects
            .cleanup_expired(self.turn.turn_number);
    }

    pub fn cleanup_temporary_spell_cost_reductions_end_of_turn(&mut self) {
        let current_turn = self.turn.turn_number;
        let active_player = self.turn.active_player;
        self.effect_store
            .temporary_spell_cost_reductions
            .retain(|effect| !effect.is_expired(current_turn, active_player));
    }

    pub fn cleanup_temporary_spell_ability_grants_end_of_turn(&mut self) {
        let current_turn = self.turn.turn_number;
        self.effect_store
            .temporary_spell_ability_grants
            .retain(|effect| !effect.is_expired(current_turn));
    }

    pub fn cleanup_temporary_object_static_ability_grants_end_of_turn(&mut self) {
        let current_turn = self.turn.turn_number;
        let ids = self
            .objects
            .iter()
            .filter_map(|(&id, object)| {
                object
                    .temporary_static_ability_grants
                    .iter()
                    .any(|grant| grant.expires_end_of_turn <= current_turn)
                    .then_some(id)
            })
            .collect::<Vec<_>>();
        for id in ids {
            if let Some(object) = self.object_mut(id) {
                object
                    .temporary_static_ability_grants
                    .retain(|grant| grant.expires_end_of_turn > current_turn);
            }
        }
    }

    /// Can the player draw any cards?
    pub fn can_draw(&self, player: PlayerId) -> bool {
        self.effect_store.cant_effects.can_draw(player)
    }

    /// Can the player gain life?
    pub fn can_gain_life(&self, player: PlayerId) -> bool {
        self.effect_store.cant_effects.can_gain_life(player)
    }

    /// Can the player lose life (not from damage)?
    pub fn can_lose_life(&self, player: PlayerId) -> bool {
        self.effect_store.cant_effects.can_lose_life(player)
    }

    /// Can damage dealt to the player cause life loss?
    pub fn can_damage_cause_life_loss(&self, player: PlayerId) -> bool {
        self.effect_store
            .cant_effects
            .can_damage_cause_life_loss(player)
    }

    /// Can the player's life total change?
    pub fn can_change_life_total(&self, player: PlayerId) -> bool {
        self.effect_store.cant_effects.can_change_life_total(player)
    }

    /// Returns true if a player can currently pay the given amount of life.
    pub fn can_pay_life(&self, player: PlayerId, amount: u32) -> bool {
        if amount == 0 {
            return self.player(player).is_some();
        }
        self.can_lose_life(player) && self.player(player).is_some_and(|p| p.life >= amount as i32)
    }

    /// Returns true if a player can currently pay life for the given reason.
    pub fn can_pay_life_with_reason(
        &self,
        player: PlayerId,
        amount: u32,
        reason: crate::costs::PaymentReason,
    ) -> bool {
        if reason.is_cast_or_ability_payment()
            && self.player_cant_pay_life_to_cast_or_activate(player)
            && amount > 0
        {
            return false;
        }
        self.can_pay_life(player, amount)
    }

    /// Makes a player lose life if their life total can change.
    ///
    /// Returns the amount of life actually lost.
    pub fn lose_life(&mut self, player: PlayerId, amount: u32) -> u32 {
        if amount == 0 || !self.can_lose_life(player) {
            return 0;
        }
        if let Some(p) = self.player_mut(player) {
            p.lose_life(amount);
            return amount;
        }
        0
    }

    /// Marks a player as having lost the game and emits the trigger-visible event once.
    pub fn mark_player_lost(&mut self, player: PlayerId) -> bool {
        let lookback_source_snapshots = self.trigger_source_lookback_snapshots();
        let should_emit = if let Some(p) = self.player_mut(player) {
            if !p.is_in_game() {
                false
            } else {
                p.has_lost = true;
                true
            }
        } else {
            false
        };

        if should_emit {
            self.queue_trigger_event(
                crate::provenance::ProvNodeId::default(),
                crate::events::Event::player_loses_game(player)
                    .into_raw()
                    .with_lookback_source_snapshots(lookback_source_snapshots),
            );
        }

        should_emit
    }

    /// Pays life as a cost.
    ///
    /// Returns true if the player could pay and life was deducted.
    pub fn pay_life(&mut self, player: PlayerId, amount: u32) -> bool {
        if amount == 0 {
            return self.player(player).is_some();
        }
        if !self.can_pay_life(player, amount) {
            return false;
        }
        self.lose_life(player, amount) == amount
    }

    /// Can the player search their library?
    pub fn can_search_library(&self, player: PlayerId) -> bool {
        self.effect_store.cant_effects.can_search_library(player)
    }

    /// Can the player draw extra cards this turn?
    pub fn can_draw_extra_cards(&self, player: PlayerId) -> bool {
        self.effect_store.cant_effects.can_draw_extra_cards(player)
    }

    /// Sync draw-step tracking to the current turn position.
    pub fn sync_draw_step_tracking(&mut self) {
        if self.turn.phase == Phase::Beginning && self.turn.step == Some(Step::Draw) {
            if self.turn_store.tracked_draw_step_player != Some(self.turn.active_player) {
                self.turn_store.tracked_draw_step_player = Some(self.turn.active_player);
                self.turn_store.cards_drawn_this_draw_step = 0;
            }
        } else {
            self.turn_store.tracked_draw_step_player = None;
            self.turn_store.cards_drawn_this_draw_step = 0;
        }
    }

    /// Returns whether the given player is drawing during their own draw step, plus prior draws in that step.
    pub fn draw_step_context_for_player(&mut self, player: PlayerId) -> (bool, u32) {
        self.sync_draw_step_tracking();
        if self.turn_store.tracked_draw_step_player == Some(player) {
            (true, self.turn_store.cards_drawn_this_draw_step)
        } else {
            (false, 0)
        }
    }

    /// Records cards drawn in the currently tracked draw step.
    pub fn record_cards_drawn_in_current_draw_step(&mut self, player: PlayerId, amount: u32) {
        self.sync_draw_step_tracking();
        if self.turn_store.tracked_draw_step_player == Some(player) {
            self.turn_store.cards_drawn_this_draw_step = self
                .turn_store
                .cards_drawn_this_draw_step
                .saturating_add(amount);
        }
    }

    /// Can the creature attack?
    pub fn can_attack(&self, creature: ObjectId) -> bool {
        self.effect_store.cant_effects.can_attack(creature)
    }

    /// Can the creature attack this player or planeswalkers they control?
    pub fn can_attack_defending_player(
        &self,
        creature: ObjectId,
        defending_player: PlayerId,
    ) -> bool {
        self.effect_store
            .cant_effects
            .can_attack_defending_player(creature, defending_player)
    }

    /// Can the creature attack as the only attacker?
    pub fn can_attack_alone(&self, creature: ObjectId) -> bool {
        self.effect_store.cant_effects.can_attack_alone(creature)
    }

    /// Can the creature block?
    pub fn can_block(&self, creature: ObjectId) -> bool {
        self.effect_store.cant_effects.can_block(creature)
    }

    /// Can the creature block a specific attacker?
    pub fn can_block_attacker(&self, blocker: ObjectId, attacker: ObjectId) -> bool {
        self.effect_store
            .cant_effects
            .can_block_attacker(blocker, attacker)
    }

    /// Must the creature block a specific attacker this turn if able?
    pub fn must_block_attacker(&self, blocker: ObjectId, attacker: ObjectId) -> bool {
        self.effect_store
            .cant_effects
            .must_block_attacker(blocker, attacker)
    }

    /// Must the attacker be blocked this turn if able?
    pub fn must_be_blocked(&self, attacker: ObjectId) -> bool {
        self.effect_store.cant_effects.must_be_blocked(attacker)
    }

    /// Get required attackers for a blocker, if any.
    pub fn required_attackers_for_blocker(&self, blocker: ObjectId) -> Option<&HashSet<ObjectId>> {
        self.effect_store
            .cant_effects
            .required_attackers_for_blocker(blocker)
    }

    /// Can the creature block as the only blocker?
    pub fn can_block_alone(&self, creature: ObjectId) -> bool {
        self.effect_store.cant_effects.can_block_alone(creature)
    }

    /// Can the permanent untap during untap step?
    pub fn can_untap(&self, permanent: ObjectId) -> bool {
        self.effect_store.cant_effects.can_untap(permanent)
    }

    /// Can the permanent untap during the specified player's untap step?
    pub fn can_untap_during_step(&self, permanent: ObjectId, untap_player: PlayerId) -> bool {
        self.object(permanent).is_some_and(|object| {
            self.effect_store.cant_effects.can_untap_during_step(
                permanent,
                self.controller_of(object),
                untap_player,
            )
        })
    }

    /// Can damage be prevented?
    pub fn can_prevent_damage(&self) -> bool {
        self.effect_store.cant_effects.can_prevent_damage()
    }

    /// Can the permanent be destroyed?
    pub fn can_be_destroyed(&self, permanent: ObjectId) -> bool {
        self.effect_store.cant_effects.can_be_destroyed(permanent)
    }

    /// Can the permanent be regenerated?
    pub fn can_be_regenerated(&self, permanent: ObjectId) -> bool {
        self.effect_store.cant_effects.can_be_regenerated(permanent)
    }

    /// Can the permanent be sacrificed?
    pub fn can_be_sacrificed(&self, permanent: ObjectId) -> bool {
        self.effect_store.cant_effects.can_be_sacrificed(permanent)
    }

    /// Can the creature be blocked?
    pub fn can_be_blocked(&self, creature: ObjectId) -> bool {
        self.effect_store.cant_effects.can_be_blocked(creature)
    }

    /// Can the player lose the game?
    pub fn can_lose_game(&self, player: PlayerId) -> bool {
        self.effect_store.cant_effects.can_lose_game(player)
    }

    /// Can the player win the game?
    pub fn can_win_game(&self, player: PlayerId) -> bool {
        self.effect_store.cant_effects.can_win_game(player)
    }

    /// Can the player become the monarch?
    pub fn can_become_monarch(&self, player: PlayerId) -> bool {
        self.effect_store.cant_effects.can_become_monarch(player)
    }

    /// Can the player cast spells?
    pub fn can_cast_spells(&self, player: PlayerId) -> bool {
        self.effect_store.cant_effects.can_cast_spells(player)
    }

    /// Can the player activate non-mana abilities?
    pub fn can_activate_non_mana_abilities(&self, player: PlayerId) -> bool {
        self.effect_store
            .cant_effects
            .can_activate_non_mana_abilities(player)
    }

    /// Can activated abilities of this permanent be activated (including mana abilities)?
    pub fn can_activate_abilities_of(&self, source: ObjectId) -> bool {
        self.effect_store
            .cant_effects
            .can_activate_abilities_of(source)
    }

    /// Can activated abilities with {T} in their costs of this permanent be activated?
    pub fn can_activate_tap_abilities_of(&self, source: ObjectId) -> bool {
        self.effect_store
            .cant_effects
            .can_activate_tap_abilities_of(source)
    }

    /// Can non-mana activated abilities of this permanent be activated?
    pub fn can_activate_non_mana_abilities_of(&self, source: ObjectId) -> bool {
        self.effect_store
            .cant_effects
            .can_activate_non_mana_abilities_of(source)
    }

    /// Can the player cast creature spells?
    pub fn can_cast_creature_spells(&self, player: PlayerId) -> bool {
        self.effect_store
            .cant_effects
            .can_cast_creature_spells(player)
    }

    /// Can the player cast another spell this turn?
    pub fn can_cast_additional_spell_this_turn(&self, player: PlayerId) -> bool {
        self.effect_store
            .cant_effects
            .can_cast_additional_spell_this_turn(player)
    }

    /// Can the player cast another noncreature spell this turn?
    pub fn can_cast_additional_noncreature_spell_this_turn(&self, player: PlayerId) -> bool {
        self.effect_store
            .cant_effects
            .can_cast_additional_noncreature_spell_this_turn(player)
    }

    /// Can the player cast another nonartifact spell this turn?
    pub fn can_cast_additional_nonartifact_spell_this_turn(&self, player: PlayerId) -> bool {
        self.effect_store
            .cant_effects
            .can_cast_additional_nonartifact_spell_this_turn(player)
    }

    /// Can the player cast another non-Phyrexian spell this turn?
    pub fn can_cast_additional_nonphyrexian_spell_this_turn(&self, player: PlayerId) -> bool {
        self.effect_store
            .cant_effects
            .can_cast_additional_nonphyrexian_spell_this_turn(player)
    }

    /// Can counters be placed on this permanent?
    pub fn can_have_counters_placed(&self, permanent: ObjectId) -> bool {
        self.effect_store
            .cant_effects
            .can_have_counters_placed(permanent)
    }

    /// Is this permanent untargetable (by shroud/hexproof-style effects)?
    pub fn is_untargetable(&self, permanent: ObjectId) -> bool {
        self.effect_store.cant_effects.is_untargetable(permanent)
    }

    pub fn can_target_object_from_source(&self, object: ObjectId, source_id: ObjectId) -> bool {
        self.effect_store
            .cant_effects
            .can_target_object_from_source(self, object, source_id)
    }

    /// Can this player be targeted?
    pub fn can_target_player(&self, player: PlayerId) -> bool {
        self.effect_store.cant_effects.can_target_player(player)
    }

    /// Can this player be targeted by the specified source object?
    pub fn can_target_player_from_source(&self, player: PlayerId, source_id: ObjectId) -> bool {
        self.effect_store
            .cant_effects
            .can_target_player_from_source(self, player, source_id)
    }

    /// Can this spell on the stack be countered?
    pub fn can_be_countered(&self, spell: ObjectId) -> bool {
        self.effect_store.cant_effects.can_be_countered(spell)
    }

    /// Can this permanent transform?
    pub fn can_transform(&self, permanent: ObjectId) -> bool {
        self.effect_store.cant_effects.can_transform(permanent)
    }

    /// Can this permanent phase out?
    pub fn can_phase_out(&self, permanent: ObjectId) -> bool {
        self.effect_store.cant_effects.can_phase_out(permanent)
    }

    /// Adds an object to the game.
    pub fn add_object(&mut self, object: Object) {
        self.mark_continuous_state_dirty();
        self.bump_mutation_revision();
        let zone = object.zone;
        let id = object.id;
        let owner = object.owner;
        let stable_id = object.stable_id;

        self.next_object_id = self.next_object_id.max(id.0.saturating_add(1));
        self.objects.insert(id, Arc::new(object));
        self.stable_id_index.insert(stable_id, id);
        self.bump_zone_revision(zone);

        // Update zone indexes
        match zone {
            Zone::Battlefield => self.battlefield.push(id),
            Zone::Command => self.command_zone.push(id),
            Zone::Exile => self.exile.push(id),
            Zone::Library => {
                if let Some(player) = self.player_mut(owner) {
                    player.library.push(id);
                }
                self.bump_library_top_revision(owner);
            }
            Zone::Hand => {
                if let Some(player) = self.player_mut(owner) {
                    player.hand.push(id);
                }
            }
            Zone::Graveyard => {
                if let Some(player) = self.player_mut(owner) {
                    player.graveyard.push(id);
                }
            }
            Zone::OutsideGame => {
                if let Some(player) = self.player_mut(owner) {
                    player.sideboard.push(id);
                }
            }
            Zone::Stack => {
                // Stack entries are managed separately via StackEntry
            }
        }

        // Validate zone consistency in debug builds
        #[cfg(debug_assertions)]
        self.debug_assert_zone_consistency();
    }

    /// Creates an object from a card and adds it to the specified zone.
    pub fn create_object_from_card(
        &mut self,
        card: &Card,
        owner: PlayerId,
        zone: Zone,
    ) -> ObjectId {
        self.prime_linked_face_lookup(card.other_face_name.as_deref(), card.other_face);
        let id = self.new_object_id();
        let mut object = Object::from_card(id, card, owner, zone);
        if zone == Zone::Battlefield
            && let Some(loyalty) = object.base_loyalty
            && loyalty > 0
        {
            object.add_counters(crate::object::CounterType::Loyalty, loyalty);
        }
        self.add_object(object);
        if zone == Zone::Battlefield {
            // Seed battlefield objects with an entry timestamp so layer timestamp
            // ordering is deterministic (replay setup, fixtures, etc.).
            self.effect_store.continuous_effects.record_entry(id);
            self.handle_day_night_object_entered(id);
        }
        id
    }

    /// Creates an object from a CardDefinition (includes abilities and spell effects).
    pub fn create_object_from_definition(
        &mut self,
        def: &crate::cards::CardDefinition,
        owner: PlayerId,
        zone: Zone,
    ) -> ObjectId {
        self.prime_linked_face_definitions(def);
        let id = self.new_object_id();
        let handles = self.object_store.shared_handles_for_definition(def);
        let mut object = Object::from_card_definition_with_shared(id, def, owner, zone, &handles);
        if zone == Zone::Battlefield
            && let Some(loyalty) = object.base_loyalty
            && loyalty > 0
        {
            object.add_counters(crate::object::CounterType::Loyalty, loyalty);
        }
        self.add_object(object);
        if zone == Zone::Battlefield {
            // Seed battlefield objects with an entry timestamp so static ability
            // effects use proper timestamp order in layers.
            self.effect_store.continuous_effects.record_entry(id);
            self.handle_day_night_object_entered(id);
            if self.object(id).is_some_and(|object| {
                object.abilities.iter().any(|ability| match &ability.kind {
                    crate::ability::AbilityKind::Static(static_ability) => {
                        static_ability.life_total_note_as_enters().is_some()
                    }
                    _ => false,
                })
            }) {
                self.note_life_total_for_source(id, owner);
            }
        }
        id
    }

    pub(crate) fn object_from_token_definition(
        &mut self,
        id: ObjectId,
        def: &crate::cards::CardDefinition,
        controller: PlayerId,
    ) -> Object {
        self.prime_linked_face_definitions(def);
        let handles = self.object_store.shared_handles_for_definition(def);
        Object::from_token_definition_with_shared(id, def, controller, &handles)
    }

    /// Cache a linked-face definition for later runtime lookups.
    pub fn register_linked_face_definition(&mut self, def: &crate::cards::CardDefinition) {
        self.cache_linked_face_definition(def);
    }

    /// Cache a definition and its linked face using an explicit catalog.
    pub fn register_linked_face_family_from_catalog<C: crate::session::CardCatalog>(
        &mut self,
        def: &crate::cards::CardDefinition,
        catalog: &C,
    ) {
        self.cache_linked_face_definition(def);
        if let Some(other_def) = catalog
            .linked_face_definition(def.card.other_face_name.as_deref(), def.card.other_face)
            .cloned()
        {
            self.cache_linked_face_definition(&other_def);
        }
    }

    /// Create an object after explicitly priming linked-face lookup from a catalog.
    pub fn create_object_from_catalog_definition<C: crate::session::CardCatalog>(
        &mut self,
        def: &crate::cards::CardDefinition,
        catalog: &C,
        owner: PlayerId,
        zone: Zone,
    ) -> ObjectId {
        self.register_linked_face_family_from_catalog(def, catalog);
        self.create_object_from_definition(def, owner, zone)
    }

    pub fn create_hidden_card_placeholder(
        &mut self,
        owner: PlayerId,
        zone: Zone,
        slot: u16,
        commitment: String,
    ) -> ObjectId {
        let id = self.new_object_id();
        let object = Object::new_hidden_card(id, owner, zone);
        self.add_object(object);
        self.auxiliary_tracking_mut().hidden_cards.insert(
            id,
            HiddenCardInfo {
                owner,
                zone,
                slot,
                commitment,
                public_slot: None,
                public_commitment: None,
            },
        );
        id
    }

    pub fn hidden_card_info(&self, id: ObjectId) -> Option<&HiddenCardInfo> {
        self.auxiliary_tracking.hidden_cards.get(&id)
    }

    pub fn hidden_card_entries(&self) -> impl Iterator<Item = (&ObjectId, &HiddenCardInfo)> + '_ {
        self.auxiliary_tracking.hidden_cards.iter()
    }

    pub fn set_hidden_card_info(&mut self, id: ObjectId, info: HiddenCardInfo) {
        self.auxiliary_tracking_mut().hidden_cards.insert(id, info);
    }

    pub fn is_hidden_card_placeholder(&self, id: ObjectId) -> bool {
        self.auxiliary_tracking.hidden_cards.contains_key(&id)
            && self.object(id).is_some_and(|object| object.card.is_none())
    }

    pub fn reveal_hidden_card_with_definition(
        &mut self,
        id: ObjectId,
        def: &crate::cards::CardDefinition,
    ) -> Option<HiddenCardInfo> {
        self.prime_linked_face_definitions(def);
        let info = self.auxiliary_tracking.hidden_cards.get(&id)?.clone();
        let zone = self.object(id)?.zone;
        let handles = self.object_store.shared_handles_for_definition(def);
        let object = self.object_mut(id)?;
        object.apply_card_definition_with_shared(def, &handles);
        if zone == Zone::Battlefield
            && let Some(loyalty) = object.base_loyalty
            && loyalty > 0
        {
            object.add_counters(crate::object::CounterType::Loyalty, loyalty);
        }
        self.mark_continuous_state_dirty();
        Some(info)
    }

    /// Draws cards for a player, moving them from library to hand.
    /// Uses move_object to properly update the object's zone.
    /// Returns the new ObjectIds of the drawn cards.
    pub fn draw_cards(&mut self, player: PlayerId, count: usize) -> Vec<ObjectId> {
        let mut drawn = Vec::new();
        for _ in 0..count {
            // Get the top card of the library (last element)
            let card_id = if let Some(player_obj) = self.player(player) {
                player_obj.library.last().copied()
            } else {
                None
            };

            if let Some(id) = card_id {
                // Move from library to hand
                if let Some(new_id) = self.move_object_by_game_rule(id, Zone::Hand) {
                    drawn.push(new_id);
                }
            } else {
                // Can't draw from empty library
                break;
            }
        }
        drawn
    }

    /// Draws cards for a player, allowing commander draw replacements to be chosen.
    ///
    /// Only cards that actually move to hand are returned.
    pub fn draw_cards_with_dm(
        &mut self,
        player: PlayerId,
        count: usize,
        decision_maker: &mut dyn crate::decision::DecisionMaker,
    ) -> Vec<ObjectId> {
        let mut drawn = Vec::new();
        for _ in 0..count {
            if self
                .try_replace_draw_with_dredge(player, decision_maker)
                .is_some()
            {
                continue;
            }

            let card_id = if let Some(player_obj) = self.player(player) {
                player_obj.library.last().copied()
            } else {
                None
            };

            let Some(id) = card_id else {
                break;
            };

            let final_zone =
                self.resolve_commander_move_destination(id, Zone::Hand, decision_maker);
            if let Some(new_id) = self.move_object_by_game_rule(id, final_zone)
                && final_zone == Zone::Hand
            {
                drawn.push(new_id);
            }
        }
        drawn
    }

    pub(crate) fn dredge_replacement_candidate(
        &self,
        player: PlayerId,
    ) -> Option<(ObjectId, usize)> {
        self.player(player)?
            .graveyard
            .iter()
            .copied()
            .find_map(|object_id| {
                let amount = self.dredge_amount_for_object(object_id)?;
                (self.player(player)?.library.len() >= amount).then_some((object_id, amount))
            })
    }

    pub(crate) fn dredge_replacement_context(
        &self,
        player: PlayerId,
        dredge_card: ObjectId,
        amount: usize,
    ) -> crate::decisions::context::BooleanContext {
        let description = self
            .current_name(dredge_card)
            .map(|name| {
                format!(
                    "mill {amount} cards instead of drawing a card to return {name} to your hand"
                )
            })
            .unwrap_or_else(|| {
                format!(
                    "mill {amount} cards instead of drawing a card to return this card to your hand"
                )
            });
        let mut ctx =
            crate::decisions::context::BooleanContext::new(player, Some(dredge_card), description);
        if let Some(name) = self.current_name(dredge_card) {
            ctx = ctx.with_source_name(name);
        }
        ctx
    }

    pub(crate) fn replace_draw_with_dredge(
        &mut self,
        player: PlayerId,
        dredge_card: ObjectId,
        amount: usize,
        decision_maker: &mut dyn crate::decision::DecisionMaker,
    ) -> Option<Vec<ObjectId>> {
        let card_is_still_in_graveyard = self
            .player(player)?
            .graveyard
            .iter()
            .any(|&candidate| candidate == dredge_card);
        if !card_is_still_in_graveyard || self.player(player)?.library.len() < amount {
            return None;
        }

        let cause = crate::events::cause::EventCause::from_effect(dredge_card, player);
        let cards_to_mill: Vec<ObjectId> = self
            .player(player)
            .map(|p| p.library.iter().rev().take(amount).copied().collect())
            .unwrap_or_default();
        if cards_to_mill.len() < amount {
            return None;
        }

        for card_id in cards_to_mill {
            let Some(from_zone) = self.object(card_id).map(|obj| obj.zone) else {
                continue;
            };
            let outcome = crate::events::processing::process_zone_change(
                self,
                card_id,
                from_zone,
                Zone::Graveyard,
                cause.clone(),
                decision_maker,
            );
            if let crate::events::processing::ZoneChangeOutcome::Proceed(final_zone) = outcome
                && final_zone == Zone::Graveyard
            {
                let _ = self.move_object(card_id, Zone::Graveyard, cause.clone());
            }
        }

        self.move_object(dredge_card, Zone::Hand, cause)
            .map(|new_id| vec![new_id])
    }

    fn try_replace_draw_with_dredge(
        &mut self,
        player: PlayerId,
        decision_maker: &mut dyn crate::decision::DecisionMaker,
    ) -> Option<Vec<ObjectId>> {
        let (dredge_card, amount) = self.dredge_replacement_candidate(player)?;
        let ctx = self.dredge_replacement_context(player, dredge_card, amount);
        let spec = crate::decisions::MaySpec::new(dredge_card, ctx.description);
        if !crate::decisions::make_decision_with_fallback(
            self,
            decision_maker,
            player,
            Some(dredge_card),
            spec,
            crate::decision::FallbackStrategy::Decline,
        ) {
            return None;
        }

        self.replace_draw_with_dredge(player, dredge_card, amount, decision_maker)
    }

    fn dredge_amount_for_object(&self, object_id: ObjectId) -> Option<usize> {
        let object = self.object(object_id)?;
        object.abilities.iter().find_map(|ability| {
            let AbilityKind::Static(static_ability) = &ability.kind else {
                return None;
            };
            parse_dredge_amount(&static_ability.display())
        })
    }

    /// Moves an object to a new zone.
    /// Per MTG rule 400.7, this creates a new object (new ID).
    /// Returns the new ObjectId.
    fn create_meld_component_object(
        &mut self,
        component: &MeldComponentState,
        zone: Zone,
    ) -> Option<ObjectId> {
        let def = self.linked_face_definition_by_name_or_id(Some(&component.name), None)?;
        let new_id = self.new_object_id();
        let handles = self.object_store.shared_handles_for_definition(&def);
        let mut object = crate::object::Object::from_card_definition_with_shared(
            new_id,
            &def,
            component.owner,
            zone,
            &handles,
        );
        object.stable_id = component.stable_id;
        self.add_object(object);
        Some(new_id)
    }

    fn cache_linked_face_definition(&mut self, def: &crate::cards::CardDefinition) {
        self.object_store.shared_handles_for_definition(def);
        self.linked_face_definitions_by_id
            .insert(def.card.id, def.clone());
        self.linked_face_definitions_by_name
            .insert(def.card.name.to_string(), def.clone());
    }

    fn load_linked_face_definition(
        &self,
        name: Option<&str>,
        id: Option<crate::ids::CardId>,
    ) -> Option<crate::cards::CardDefinition> {
        #[cfg(test)]
        if let Some(definition) = crate::cards::linked_face_definition_by_name_or_id(name, id) {
            return Some(definition);
        }

        if let Some(face_name) = name {
            if let Ok(definition) = crate::cards::CardRegistry::try_compile_card(face_name) {
                return Some(definition);
            }

            let mut registry = crate::cards::CardRegistry::new();
            registry.ensure_cards_loaded([face_name]);
            if let Some(definition) = registry.get(face_name).cloned() {
                return Some(definition);
            }
        }

        let card_id = id?;
        let registry = crate::cards::CardRegistry::with_builtin_cards();
        registry.get_by_id(card_id).cloned()
    }

    fn prime_linked_face_lookup(&mut self, name: Option<&str>, id: Option<crate::ids::CardId>) {
        if let Some(face_name) = name
            && self.linked_face_definitions_by_name.contains_key(face_name)
        {
            return;
        }

        if let Some(card_id) = id
            && self.linked_face_definitions_by_id.contains_key(&card_id)
        {
            return;
        }

        if let Some(other_def) = self.load_linked_face_definition(name, id) {
            self.cache_linked_face_definition(&other_def);
        }
    }

    fn prime_linked_face_definitions(&mut self, def: &crate::cards::CardDefinition) {
        if def.card.other_face.is_none() && def.card.other_face_name.is_none() {
            return;
        }

        self.cache_linked_face_definition(def);
        self.prime_linked_face_lookup(def.card.other_face_name.as_deref(), def.card.other_face);
    }

    pub fn linked_face_definition_by_name_or_id(
        &self,
        name: Option<&str>,
        id: Option<crate::ids::CardId>,
    ) -> Option<crate::cards::CardDefinition> {
        if let Some(face_name) = name
            && let Some(definition) = self.linked_face_definitions_by_name.get(face_name)
        {
            return Some(definition.clone());
        }

        if let Some(card_id) = id
            && let Some(definition) = self.linked_face_definitions_by_id.get(&card_id)
        {
            return Some(definition.clone());
        }

        self.load_linked_face_definition(name, id)
    }
}
