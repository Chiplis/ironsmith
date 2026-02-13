use crate::ability::AbilityKind;
use crate::alternative_cast::CastingMethod;
use crate::color::Color;
use crate::combat_state::{AttackTarget, CombatState};
use crate::game_state::{GameState, StackEntry, TurnCounterKey};
use crate::ids::{ObjectId, PlayerId};
use crate::mana::ManaCost;
use crate::object::{CounterType, Object, ObjectKind};
use crate::player::Player;
use crate::zone::Zone;

use super::adapters::{optional_costs_to_spec, targets_from_game};
use super::crypto::{
    DOMAIN_COMBAT_STATE, DOMAIN_PUBLIC_OBJECT, DOMAIN_PUBLIC_STATE, DOMAIN_STACK_STATE,
    DOMAIN_TRACKERS_STATE, hash_bytes,
};
use super::{
    ActionPropose, CanonicalEncode, ContribRequest, GameObjectId, GamePlayerId, Hash32,
    ManaPoolSpec, ManaSymbolSpec, ObjectKindCode, PubKey, PublicObjectState, PublicPlayerState,
    PublicStackEntry, PublicStateSnapshot, PublicTurnState, PublicZoneIndex, TargetSpec, ZoneCode,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StateRootError {
    Mismatch { expected: Hash32, found: Hash32 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActionVerifyError {
    InvalidSignature,
    PrevState(StateRootError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContribVerifyError {
    InvalidSignature,
    PrevState(StateRootError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AbilityPublicSpec {
    kind_tag: u8,
    functional_zones: Vec<ZoneCode>,
    text: Option<String>,
}

impl CanonicalEncode for AbilityPublicSpec {
    fn encode(&self, out: &mut Vec<u8>) {
        self.kind_tag.encode(out);
        self.functional_zones.encode(out);
        self.text.encode(out);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PublicObjectDigest {
    name: String,
    oracle_text: String,
    mana_cost: Option<Vec<Vec<ManaSymbolSpec>>>,
    color_override: Option<u8>,
    supertypes: Vec<String>,
    card_types: Vec<String>,
    subtypes: Vec<String>,
    base_power: Option<String>,
    base_toughness: Option<String>,
    base_loyalty: Option<u32>,
    abilities: Vec<AbilityPublicSpec>,
    counters: Vec<(String, u32)>,
    attached_to: Option<GameObjectId>,
    attachments: Vec<GameObjectId>,
    tapped: bool,
    flipped: bool,
    face_down: bool,
    phased_out: bool,
    summoning_sick: bool,
    monstrous: bool,
    madness_exiled: bool,
    saga_final_chapter_resolved: bool,
    commander: bool,
    damage_marked: u32,
    regeneration_shields: u32,
    imprinted_cards: Vec<GameObjectId>,
    optional_costs_paid: Vec<u32>,
}

impl CanonicalEncode for PublicObjectDigest {
    fn encode(&self, out: &mut Vec<u8>) {
        self.name.encode(out);
        self.oracle_text.encode(out);
        self.mana_cost.encode(out);
        self.color_override.encode(out);
        self.supertypes.encode(out);
        self.card_types.encode(out);
        self.subtypes.encode(out);
        self.base_power.encode(out);
        self.base_toughness.encode(out);
        self.base_loyalty.encode(out);
        self.abilities.encode(out);
        self.counters.encode(out);
        self.attached_to.encode(out);
        self.attachments.encode(out);
        self.tapped.encode(out);
        self.flipped.encode(out);
        self.face_down.encode(out);
        self.phased_out.encode(out);
        self.summoning_sick.encode(out);
        self.monstrous.encode(out);
        self.madness_exiled.encode(out);
        self.saga_final_chapter_resolved.encode(out);
        self.commander.encode(out);
        self.damage_marked.encode(out);
        self.regeneration_shields.encode(out);
        self.imprinted_cards.encode(out);
        self.optional_costs_paid.encode(out);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum CastingMethodSpec {
    Normal,
    Alternative(u32),
    GrantedEscape {
        source: GameObjectId,
        exile_count: u32,
    },
    GrantedFlashback,
    PlayFrom {
        source: GameObjectId,
        zone: ZoneCode,
        use_alternative: Option<u32>,
    },
}

impl CanonicalEncode for CastingMethodSpec {
    fn encode(&self, out: &mut Vec<u8>) {
        match self {
            CastingMethodSpec::Normal => {
                0u8.encode(out);
            }
            CastingMethodSpec::Alternative(index) => {
                1u8.encode(out);
                index.encode(out);
            }
            CastingMethodSpec::GrantedEscape {
                source,
                exile_count,
            } => {
                2u8.encode(out);
                source.encode(out);
                exile_count.encode(out);
            }
            CastingMethodSpec::GrantedFlashback => {
                3u8.encode(out);
            }
            CastingMethodSpec::PlayFrom {
                source,
                zone,
                use_alternative,
            } => {
                4u8.encode(out);
                source.encode(out);
                zone.encode(out);
                use_alternative.encode(out);
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PublicStackDigest {
    object_id: GameObjectId,
    controller: GamePlayerId,
    targets: Vec<TargetSpec>,
    x_value: Option<u32>,
    is_ability: bool,
    casting_method: CastingMethodSpec,
    optional_costs: Vec<u32>,
    defending_player: Option<GamePlayerId>,
    saga_final_chapter_source: Option<GameObjectId>,
    source_stable_id: Option<GameObjectId>,
    source_name: Option<String>,
    chosen_modes: Option<Vec<u32>>,
}

impl CanonicalEncode for PublicStackDigest {
    fn encode(&self, out: &mut Vec<u8>) {
        self.object_id.encode(out);
        self.controller.encode(out);
        self.targets.encode(out);
        self.x_value.encode(out);
        self.is_ability.encode(out);
        self.casting_method.encode(out);
        self.optional_costs.encode(out);
        self.defending_player.encode(out);
        self.saga_final_chapter_source.encode(out);
        self.source_stable_id.encode(out);
        self.source_name.encode(out);
        self.chosen_modes.encode(out);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum AttackTargetSpec {
    Player(GamePlayerId),
    Planeswalker(GameObjectId),
}

impl CanonicalEncode for AttackTargetSpec {
    fn encode(&self, out: &mut Vec<u8>) {
        match self {
            AttackTargetSpec::Player(id) => {
                0u8.encode(out);
                id.encode(out);
            }
            AttackTargetSpec::Planeswalker(id) => {
                1u8.encode(out);
                id.encode(out);
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CombatDigest {
    attackers: Vec<(GameObjectId, AttackTargetSpec)>,
    blockers: Vec<(GameObjectId, Vec<GameObjectId>)>,
    assignment_order: Vec<(GameObjectId, Vec<GameObjectId>)>,
}

impl CanonicalEncode for CombatDigest {
    fn encode(&self, out: &mut Vec<u8>) {
        self.attackers.encode(out);
        self.blockers.encode(out);
        self.assignment_order.encode(out);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ContinuousEffectsDigest {
    effects: Vec<String>,
    static_effects: Vec<String>,
    next_id: u64,
    current_timestamp: u64,
    object_entry_timestamps: Vec<(GameObjectId, u64)>,
    counter_timestamps: Vec<(GameObjectId, u64)>,
    attachment_timestamps: Vec<(GameObjectId, u64)>,
}

impl CanonicalEncode for ContinuousEffectsDigest {
    fn encode(&self, out: &mut Vec<u8>) {
        self.effects.encode(out);
        self.static_effects.encode(out);
        self.next_id.encode(out);
        self.current_timestamp.encode(out);
        self.object_entry_timestamps.encode(out);
        self.counter_timestamps.encode(out);
        self.attachment_timestamps.encode(out);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ReplacementEffectsDigest {
    effects: Vec<String>,
    effect_sources: Vec<(u64, u8)>,
    one_shot_effects: Vec<u64>,
    next_id: u64,
}

impl CanonicalEncode for ReplacementEffectsDigest {
    fn encode(&self, out: &mut Vec<u8>) {
        self.effects.encode(out);
        self.effect_sources.encode(out);
        self.one_shot_effects.encode(out);
        self.next_id.encode(out);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PreventionEffectsDigest {
    shields: Vec<String>,
    next_id: u64,
    current_turn: u32,
}

impl CanonicalEncode for PreventionEffectsDigest {
    fn encode(&self, out: &mut Vec<u8>) {
        self.shields.encode(out);
        self.next_id.encode(out);
        self.current_turn.encode(out);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TrackerDigest {
    cant_gain_life: Vec<GamePlayerId>,
    cant_search: Vec<GamePlayerId>,
    cant_attack: Vec<GameObjectId>,
    cant_block: Vec<GameObjectId>,
    cant_untap: Vec<GameObjectId>,
    cant_be_destroyed: Vec<GameObjectId>,
    cant_be_sacrificed: Vec<GameObjectId>,
    cant_cast_spells: Vec<GamePlayerId>,
    cant_draw: Vec<GamePlayerId>,
    cant_draw_extra_cards: Vec<GamePlayerId>,
    cant_be_blocked: Vec<GameObjectId>,
    cant_have_counters_placed: Vec<GameObjectId>,
    damage_cant_be_prevented: bool,
    life_total_cant_change: Vec<GamePlayerId>,
    cant_lose_game: Vec<GamePlayerId>,
    cant_win_game: Vec<GamePlayerId>,
    cant_be_targeted: Vec<GameObjectId>,
    cant_be_countered: Vec<GameObjectId>,
    any_color_players: Vec<GamePlayerId>,
    any_color_activation_sources: Vec<GameObjectId>,
    activated_abilities_this_turn: Vec<(GameObjectId, u32)>,
    cards_drawn_this_turn: Vec<(GamePlayerId, u32)>,
    spells_cast_this_turn: Vec<(GamePlayerId, u32)>,
    spells_cast_last_turn_total: u32,
    library_searches_this_turn: Vec<GamePlayerId>,
    creatures_entered_this_turn: Vec<(GamePlayerId, u32)>,
    creature_damage_to_players_this_turn: Vec<(GamePlayerId, u32)>,
    damage_to_players_this_turn: Vec<(GamePlayerId, u32)>,
    extra_turns: Vec<GamePlayerId>,
    skip_next_turn: Vec<GamePlayerId>,
    skip_next_combat_phases: Vec<GamePlayerId>,
    creatures_died_this_turn: u32,
    turn_counters: Vec<(String, u32)>,
    continuous_effects: ContinuousEffectsDigest,
    replacement_effects: ReplacementEffectsDigest,
    prevention_effects: PreventionEffectsDigest,
    delayed_triggers: Vec<String>,
    pending_trigger_events: Vec<String>,
    pending_replacement_choice: Option<String>,
    restriction_effects: Vec<String>,
    grant_registry: Vec<String>,
    player_control_effects: Vec<String>,
    player_control_timestamp: u64,
}

impl CanonicalEncode for TrackerDigest {
    fn encode(&self, out: &mut Vec<u8>) {
        self.cant_gain_life.encode(out);
        self.cant_search.encode(out);
        self.cant_attack.encode(out);
        self.cant_block.encode(out);
        self.cant_untap.encode(out);
        self.cant_be_destroyed.encode(out);
        self.cant_be_sacrificed.encode(out);
        self.cant_cast_spells.encode(out);
        self.cant_draw.encode(out);
        self.cant_draw_extra_cards.encode(out);
        self.cant_be_blocked.encode(out);
        self.cant_have_counters_placed.encode(out);
        self.damage_cant_be_prevented.encode(out);
        self.life_total_cant_change.encode(out);
        self.cant_lose_game.encode(out);
        self.cant_win_game.encode(out);
        self.cant_be_targeted.encode(out);
        self.cant_be_countered.encode(out);
        self.any_color_players.encode(out);
        self.any_color_activation_sources.encode(out);
        self.activated_abilities_this_turn.encode(out);
        self.cards_drawn_this_turn.encode(out);
        self.spells_cast_this_turn.encode(out);
        self.spells_cast_last_turn_total.encode(out);
        self.library_searches_this_turn.encode(out);
        self.creatures_entered_this_turn.encode(out);
        self.creature_damage_to_players_this_turn.encode(out);
        self.damage_to_players_this_turn.encode(out);
        self.extra_turns.encode(out);
        self.skip_next_turn.encode(out);
        self.skip_next_combat_phases.encode(out);
        self.creatures_died_this_turn.encode(out);
        self.turn_counters.encode(out);
        self.continuous_effects.encode(out);
        self.replacement_effects.encode(out);
        self.prevention_effects.encode(out);
        self.delayed_triggers.encode(out);
        self.pending_trigger_events.encode(out);
        self.pending_replacement_choice.encode(out);
        self.restriction_effects.encode(out);
        self.grant_registry.encode(out);
        self.player_control_effects.encode(out);
        self.player_control_timestamp.encode(out);
    }
}

pub fn build_public_state_snapshot(game: &GameState) -> PublicStateSnapshot {
    let turn = PublicTurnState {
        active_player: game.turn.active_player.into(),
        priority_player: game.turn.priority_player.map(GamePlayerId::from),
        turn_number: game.turn.turn_number,
        phase: game.turn.phase.into(),
        step: game.turn.step.map(Into::into),
    };

    let mut players: Vec<PublicPlayerState> = game
        .players
        .iter()
        .map(|player| public_player_state(player))
        .collect();
    players.sort_by_key(|p| p.id.0);

    let zones = PublicZoneIndex {
        battlefield: sort_objects(game.battlefield.iter().copied()),
        exile: sort_objects(game.exile.iter().copied()),
        command_zone: sort_objects(game.command_zone.iter().copied()),
    };

    let mut objects: Vec<PublicObjectState> = game
        .objects_iter()
        .filter(|obj| is_public_zone(obj.zone))
        .map(|obj| public_object_state(game, obj))
        .collect();
    objects.sort_by_key(|obj| obj.id.0);

    let stack: Vec<PublicStackEntry> = game
        .stack
        .iter()
        .map(|entry| public_stack_entry(game, entry))
        .collect();

    let combat_hash = game.combat.as_ref().map(hash_combat_state);
    let trackers_hash = hash_trackers_state(game);

    PublicStateSnapshot {
        version: 1,
        turn,
        turn_order: game.turn_order.iter().copied().map(Into::into).collect(),
        players,
        zones,
        objects,
        stack,
        combat_hash,
        trackers_hash,
    }
}

pub fn hash_public_state_snapshot(snapshot: &PublicStateSnapshot) -> Hash32 {
    hash_bytes(DOMAIN_PUBLIC_STATE, snapshot)
}

pub fn hash_public_state(game: &GameState) -> Hash32 {
    hash_public_state_snapshot(&build_public_state_snapshot(game))
}

pub fn verify_prev_state_hash(
    game: &GameState,
    prev_state_hash: Hash32,
) -> Result<(), StateRootError> {
    let expected = hash_public_state(game);
    if expected == prev_state_hash {
        Ok(())
    } else {
        Err(StateRootError::Mismatch {
            expected,
            found: prev_state_hash,
        })
    }
}

pub fn verify_action_propose_for_game(
    game: &GameState,
    proposer_pubkey: PubKey,
    propose: &ActionPropose,
) -> Result<(), ActionVerifyError> {
    if !super::crypto::verify_action_propose(proposer_pubkey, propose) {
        return Err(ActionVerifyError::InvalidSignature);
    }
    verify_prev_state_hash(game, propose.prev_state_hash).map_err(ActionVerifyError::PrevState)
}

pub fn verify_contrib_request_for_game(
    game: &GameState,
    requester_pubkey: PubKey,
    request: &ContribRequest,
) -> Result<(), ContribVerifyError> {
    if !super::crypto::verify_contrib_request(requester_pubkey, request) {
        return Err(ContribVerifyError::InvalidSignature);
    }
    verify_prev_state_hash(game, request.prev_state_hash).map_err(ContribVerifyError::PrevState)
}

fn public_player_state(player: &Player) -> PublicPlayerState {
    let mut commander_damage: Vec<(GamePlayerId, u32)> = player
        .commander_damage
        .iter()
        .map(|(player_id, amount)| (GamePlayerId::from(*player_id), *amount))
        .collect();
    commander_damage.sort_by_key(|(player_id, _)| player_id.0);

    PublicPlayerState {
        id: player.id.into(),
        life: player.life,
        mana_pool: ManaPoolSpec {
            white: player.mana_pool.white,
            blue: player.mana_pool.blue,
            black: player.mana_pool.black,
            red: player.mana_pool.red,
            green: player.mana_pool.green,
            colorless: player.mana_pool.colorless,
        },
        poison_counters: player.poison_counters,
        energy_counters: player.energy_counters,
        experience_counters: player.experience_counters,
        lands_played_this_turn: player.lands_played_this_turn,
        land_plays_per_turn: player.land_plays_per_turn,
        max_hand_size: player.max_hand_size,
        has_lost: player.has_lost,
        has_won: player.has_won,
        has_left_game: player.has_left_game,
        hand_size: player.hand.len() as u32,
        library_size: player.library.len() as u32,
        graveyard: sort_objects(player.graveyard.iter().copied()),
        commanders: sort_objects(player.commanders.iter().copied()),
        commander_damage,
    }
}

fn public_object_state(game: &GameState, obj: &Object) -> PublicObjectState {
    let is_face_down = game.is_face_down(obj.id);
    let card_ref = if is_face_down {
        None
    } else {
        match obj.kind {
            ObjectKind::Token | ObjectKind::SpellCopy | ObjectKind::Emblem => None,
            _ => obj.card.map(|card_id| card_id.0),
        }
    };

    PublicObjectState {
        id: obj.id.into(),
        stable_id: GameObjectId::from(obj.stable_id.object_id()),
        kind: object_kind_code(obj.kind),
        card_ref,
        zone: obj.zone.into(),
        owner: obj.owner.into(),
        controller: obj.controller.into(),
        object_hash: hash_public_object(game, obj),
    }
}

fn public_stack_entry(game: &GameState, entry: &StackEntry) -> PublicStackEntry {
    let is_face_down = game
        .object(entry.object_id)
        .is_some_and(|obj| game.is_face_down(obj.id));

    let digest = PublicStackDigest {
        object_id: entry.object_id.into(),
        controller: entry.controller.into(),
        targets: targets_from_game(&entry.targets),
        x_value: entry.x_value,
        is_ability: entry.is_ability,
        casting_method: casting_method_spec(&entry.casting_method),
        optional_costs: if is_face_down {
            Vec::new()
        } else {
            optional_costs_to_spec(&entry.optional_costs_paid)
        },
        defending_player: entry.defending_player.map(Into::into),
        saga_final_chapter_source: entry.saga_final_chapter_source.map(Into::into),
        source_stable_id: entry
            .source_stable_id
            .map(|stable| GameObjectId::from(stable.object_id())),
        source_name: if is_face_down {
            None
        } else {
            entry.source_name.clone()
        },
        chosen_modes: entry
            .chosen_modes
            .as_ref()
            .map(|modes| modes.iter().map(|mode| *mode as u32).collect()),
    };
    let entry_hash = hash_bytes(DOMAIN_STACK_STATE, &digest);

    PublicStackEntry {
        object_id: entry.object_id.into(),
        controller: entry.controller.into(),
        entry_hash,
    }
}

fn hash_combat_state(combat: &CombatState) -> Hash32 {
    let mut attackers = combat.attackers.clone();
    attackers.sort_by_key(|attacker| attacker.creature);

    let attackers: Vec<(GameObjectId, AttackTargetSpec)> = attackers
        .into_iter()
        .map(|info| {
            let target = match info.target {
                AttackTarget::Player(player) => {
                    AttackTargetSpec::Player(GamePlayerId::from(player))
                }
                AttackTarget::Planeswalker(id) => {
                    AttackTargetSpec::Planeswalker(GameObjectId::from(id))
                }
            };
            (GameObjectId::from(info.creature), target)
        })
        .collect();

    let mut blockers: Vec<(GameObjectId, Vec<GameObjectId>)> = combat
        .blockers
        .iter()
        .map(|(attacker, blockers)| {
            (
                GameObjectId::from(*attacker),
                blockers.iter().copied().map(Into::into).collect(),
            )
        })
        .collect();
    blockers.sort_by_key(|(attacker, _)| attacker.0);

    let mut assignment_order: Vec<(GameObjectId, Vec<GameObjectId>)> = combat
        .damage_assignment_order
        .iter()
        .map(|(attacker, blockers)| {
            (
                GameObjectId::from(*attacker),
                blockers.iter().copied().map(Into::into).collect(),
            )
        })
        .collect();
    assignment_order.sort_by_key(|(attacker, _)| attacker.0);

    let digest = CombatDigest {
        attackers,
        blockers,
        assignment_order,
    };

    hash_bytes(DOMAIN_COMBAT_STATE, &digest)
}

fn hash_trackers_state(game: &GameState) -> Hash32 {
    let continuous_effects = ContinuousEffectsDigest {
        effects: debug_list(game.continuous_effects.effects().iter()),
        static_effects: debug_list(game.continuous_effects.static_ability_effects().iter()),
        next_id: game.continuous_effects.next_id(),
        current_timestamp: game.continuous_effects.current_timestamp(),
        object_entry_timestamps: map_object_u64_pairs(
            game.continuous_effects.object_entry_timestamps_snapshot(),
        ),
        counter_timestamps: map_object_u64_pairs(
            game.continuous_effects.counter_timestamps_snapshot(),
        ),
        attachment_timestamps: map_object_u64_pairs(
            game.continuous_effects.attachment_timestamps_snapshot(),
        ),
    };

    let replacement_effects = ReplacementEffectsDigest {
        effects: debug_list(game.replacement_effects.effects().iter()),
        effect_sources: game
            .replacement_effects
            .effect_sources_snapshot()
            .into_iter()
            .map(|(id, source)| (id, replacement_source_tag(source)))
            .collect(),
        one_shot_effects: game.replacement_effects.one_shot_effects_snapshot(),
        next_id: game.replacement_effects.next_id(),
    };

    let prevention_effects = PreventionEffectsDigest {
        shields: debug_list(game.prevention_effects.shields().iter()),
        next_id: game.prevention_effects.next_id(),
        current_turn: game.prevention_effects.current_turn(),
    };

    let delayed_triggers = debug_list(game.delayed_triggers.iter());
    let pending_trigger_events = debug_list(game.pending_trigger_events.iter());
    let pending_replacement_choice = game
        .pending_replacement_choice
        .as_ref()
        .map(|choice| format!("{:?}", choice));
    let restriction_effects = debug_list(game.restriction_effects.iter());
    let grant_registry = debug_list(game.grant_registry.grants.iter());
    let player_control_effects = debug_list(game.player_control_effects.iter());
    let digest = TrackerDigest {
        cant_gain_life: sort_players(game.cant_effects.cant_gain_life.iter().copied()),
        cant_search: sort_players(game.cant_effects.cant_search.iter().copied()),
        cant_attack: sort_objects(game.cant_effects.cant_attack.iter().copied()),
        cant_block: sort_objects(game.cant_effects.cant_block.iter().copied()),
        cant_untap: sort_objects(game.cant_effects.cant_untap.iter().copied()),
        cant_be_destroyed: sort_objects(game.cant_effects.cant_be_destroyed.iter().copied()),
        cant_be_sacrificed: sort_objects(game.cant_effects.cant_be_sacrificed.iter().copied()),
        cant_cast_spells: sort_players(game.cant_effects.cant_cast_spells.iter().copied()),
        cant_draw: sort_players(game.cant_effects.cant_draw.iter().copied()),
        cant_draw_extra_cards: sort_players(
            game.cant_effects.cant_draw_extra_cards.iter().copied(),
        ),
        cant_be_blocked: sort_objects(game.cant_effects.cant_be_blocked.iter().copied()),
        cant_have_counters_placed: sort_objects(
            game.cant_effects.cant_have_counters_placed.iter().copied(),
        ),
        damage_cant_be_prevented: game.cant_effects.damage_cant_be_prevented,
        life_total_cant_change: sort_players(
            game.cant_effects.life_total_cant_change.iter().copied(),
        ),
        cant_lose_game: sort_players(game.cant_effects.cant_lose_game.iter().copied()),
        cant_win_game: sort_players(game.cant_effects.cant_win_game.iter().copied()),
        cant_be_targeted: sort_objects(game.cant_effects.cant_be_targeted.iter().copied()),
        cant_be_countered: sort_objects(game.cant_effects.cant_be_countered.iter().copied()),
        any_color_players: sort_players(game.mana_spend_effects.any_color_players.iter().copied()),
        any_color_activation_sources: sort_objects(
            game.mana_spend_effects
                .any_color_activation_sources
                .iter()
                .copied(),
        ),
        activated_abilities_this_turn: sort_object_pairs(game.activated_abilities_this_turn.iter()),
        cards_drawn_this_turn: sort_player_counts(game.cards_drawn_this_turn.iter()),
        spells_cast_this_turn: sort_player_counts(game.spells_cast_this_turn.iter()),
        spells_cast_last_turn_total: game.spells_cast_last_turn_total,
        library_searches_this_turn: sort_players(game.library_searches_this_turn.iter().copied()),
        creatures_entered_this_turn: sort_player_counts(game.creatures_entered_this_turn.iter()),
        creature_damage_to_players_this_turn: sort_player_counts(
            game.creature_damage_to_players_this_turn.iter(),
        ),
        damage_to_players_this_turn: sort_player_counts(game.damage_to_players_this_turn.iter()),
        extra_turns: game.extra_turns.iter().copied().map(Into::into).collect(),
        skip_next_turn: sort_players(game.skip_next_turn.iter().copied()),
        skip_next_combat_phases: sort_players(game.skip_next_combat_phases.iter().copied()),
        creatures_died_this_turn: game.creatures_died_this_turn,
        turn_counters: sort_turn_counters(game.turn_counters.snapshot()),
        continuous_effects,
        replacement_effects,
        prevention_effects,
        delayed_triggers,
        pending_trigger_events,
        pending_replacement_choice,
        restriction_effects,
        grant_registry,
        player_control_effects,
        player_control_timestamp: game.player_control_timestamp,
    };

    hash_bytes(DOMAIN_TRACKERS_STATE, &digest)
}

fn hash_public_object(game: &GameState, obj: &Object) -> Hash32 {
    let counters = sorted_counter_list(&obj.counters);
    let face_down = game.is_face_down(obj.id);

    let (
        name,
        oracle_text,
        mana_cost,
        color_override,
        supertypes,
        card_types,
        subtypes,
        base_power,
        base_toughness,
        base_loyalty,
        abilities,
        optional_costs_paid,
    ) = if face_down {
        (
            "Face-down".to_string(),
            String::new(),
            None,
            None,
            Vec::new(),
            vec![enum_name(&crate::types::CardType::Creature)],
            Vec::new(),
            Some("Fixed(2)".to_string()),
            Some("Fixed(2)".to_string()),
            None,
            Vec::new(),
            Vec::new(),
        )
    } else {
        (
            obj.name.clone(),
            obj.oracle_text.clone(),
            obj.mana_cost.as_ref().map(mana_cost_spec),
            obj.color_override.map(color_mask),
            obj.supertypes.iter().map(enum_name).collect(),
            obj.card_types.iter().map(enum_name).collect(),
            obj.subtypes.iter().map(enum_name).collect(),
            obj.base_power.map(pt_value_string),
            obj.base_toughness.map(pt_value_string),
            obj.base_loyalty,
            obj.abilities
                .iter()
                .map(ability_public_spec)
                .collect::<Vec<_>>(),
            optional_costs_to_spec(&obj.optional_costs_paid),
        )
    };

    let digest = PublicObjectDigest {
        name,
        oracle_text,
        mana_cost,
        color_override,
        supertypes,
        card_types,
        subtypes,
        base_power,
        base_toughness,
        base_loyalty,
        abilities,
        counters,
        attached_to: obj.attached_to.map(Into::into),
        attachments: sort_objects(obj.attachments.iter().copied()),
        tapped: game.is_tapped(obj.id),
        flipped: game.is_flipped(obj.id),
        face_down,
        phased_out: game.is_phased_out(obj.id),
        summoning_sick: game.is_summoning_sick(obj.id),
        monstrous: game.is_monstrous(obj.id),
        madness_exiled: game.is_madness_exiled(obj.id),
        saga_final_chapter_resolved: game.is_saga_final_chapter_resolved(obj.id),
        commander: game.is_commander_object(obj.id),
        damage_marked: game.damage_on(obj.id),
        regeneration_shields: game.regeneration_shield_count(obj.id),
        imprinted_cards: sort_objects(game.get_imprinted_cards(obj.id).iter().copied()),
        optional_costs_paid,
    };

    hash_bytes(DOMAIN_PUBLIC_OBJECT, &digest)
}

fn ability_public_spec(ability: &crate::ability::Ability) -> AbilityPublicSpec {
    AbilityPublicSpec {
        kind_tag: ability_kind_tag(&ability.kind),
        functional_zones: ability
            .functional_zones
            .iter()
            .copied()
            .map(Into::into)
            .collect(),
        text: ability.text.clone(),
    }
}

fn ability_kind_tag(kind: &AbilityKind) -> u8 {
    match kind {
        AbilityKind::Static(_) => 0,
        AbilityKind::Triggered(_) => 1,
        AbilityKind::Activated(_) => 2,
        AbilityKind::Mana(_) => 3,
    }
}

fn mana_cost_spec(cost: &ManaCost) -> Vec<Vec<ManaSymbolSpec>> {
    cost.pips()
        .iter()
        .map(|pip| pip.iter().copied().map(Into::into).collect())
        .collect()
}

fn pt_value_string(value: crate::card::PtValue) -> String {
    format!("{:?}", value)
}

fn enum_name<T: std::fmt::Debug>(value: &T) -> String {
    format!("{:?}", value)
}

fn color_mask(colors: crate::color::ColorSet) -> u8 {
    let mut mask = 0u8;
    if colors.contains(Color::White) {
        mask |= 1 << 0;
    }
    if colors.contains(Color::Blue) {
        mask |= 1 << 1;
    }
    if colors.contains(Color::Black) {
        mask |= 1 << 2;
    }
    if colors.contains(Color::Red) {
        mask |= 1 << 3;
    }
    if colors.contains(Color::Green) {
        mask |= 1 << 4;
    }
    mask
}

fn object_kind_code(kind: ObjectKind) -> ObjectKindCode {
    match kind {
        ObjectKind::Card => ObjectKindCode::Card,
        ObjectKind::Token => ObjectKindCode::Token,
        ObjectKind::SpellCopy => ObjectKindCode::SpellCopy,
        ObjectKind::Emblem => ObjectKindCode::Emblem,
    }
}

fn is_public_zone(zone: Zone) -> bool {
    matches!(
        zone,
        Zone::Battlefield | Zone::Stack | Zone::Graveyard | Zone::Exile | Zone::Command
    )
}

fn sorted_counter_list(
    counters: &std::collections::HashMap<CounterType, u32>,
) -> Vec<(String, u32)> {
    let mut list: Vec<(String, u32)> = counters
        .iter()
        .map(|(counter, count)| (format!("{:?}", counter), *count))
        .collect();
    list.sort_by(|(name_a, _), (name_b, _)| name_a.cmp(name_b));
    list
}

fn casting_method_spec(method: &CastingMethod) -> CastingMethodSpec {
    match method {
        CastingMethod::Normal => CastingMethodSpec::Normal,
        CastingMethod::Alternative(index) => CastingMethodSpec::Alternative(*index as u32),
        CastingMethod::GrantedEscape {
            source,
            exile_count,
        } => CastingMethodSpec::GrantedEscape {
            source: GameObjectId::from(*source),
            exile_count: *exile_count,
        },
        CastingMethod::GrantedFlashback => CastingMethodSpec::GrantedFlashback,
        CastingMethod::PlayFrom {
            source,
            zone,
            use_alternative,
        } => CastingMethodSpec::PlayFrom {
            source: GameObjectId::from(*source),
            zone: (*zone).into(),
            use_alternative: use_alternative.map(|idx| idx as u32),
        },
    }
}

fn sort_players(players: impl Iterator<Item = PlayerId>) -> Vec<GamePlayerId> {
    let mut list: Vec<GamePlayerId> = players.map(Into::into).collect();
    list.sort_by_key(|player| player.0);
    list
}

fn sort_objects(objects: impl Iterator<Item = ObjectId>) -> Vec<GameObjectId> {
    let mut list: Vec<GameObjectId> = objects.map(Into::into).collect();
    list.sort_by_key(|object| object.0);
    list
}

fn sort_object_pairs<'a>(
    entries: impl Iterator<Item = &'a (ObjectId, usize)>,
) -> Vec<(GameObjectId, u32)> {
    let mut list: Vec<(GameObjectId, u32)> = entries
        .map(|(object_id, index)| (GameObjectId::from(*object_id), *index as u32))
        .collect();
    list.sort_by_key(|(object_id, index)| (object_id.0, *index));
    list
}

fn sort_player_counts<'a>(
    entries: impl Iterator<Item = (&'a PlayerId, &'a u32)>,
) -> Vec<(GamePlayerId, u32)> {
    let mut list: Vec<(GamePlayerId, u32)> = entries
        .map(|(player_id, count)| (GamePlayerId::from(*player_id), *count))
        .collect();
    list.sort_by_key(|(player_id, _)| player_id.0);
    list
}

fn sort_turn_counters(counters: Vec<(TurnCounterKey, u32)>) -> Vec<(String, u32)> {
    let mut list: Vec<(String, u32)> = counters
        .into_iter()
        .map(|(key, count)| (format!("{:?}", key), count))
        .collect();
    list.sort_by(|(name_a, _), (name_b, _)| name_a.cmp(name_b));
    list
}

fn debug_list<T: std::fmt::Debug>(items: impl IntoIterator<Item = T>) -> Vec<String> {
    items
        .into_iter()
        .map(|item| format!("{:?}", item))
        .collect()
}

fn map_object_u64_pairs(pairs: Vec<(ObjectId, u64)>) -> Vec<(GameObjectId, u64)> {
    pairs
        .into_iter()
        .map(|(object_id, value)| (GameObjectId::from(object_id), value))
        .collect()
}

fn replacement_source_tag(source: crate::replacement::ReplacementEffectSource) -> u8 {
    match source {
        crate::replacement::ReplacementEffectSource::StaticAbility => 0,
        crate::replacement::ReplacementEffectSource::Resolution => 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::game_state::GameState;
    use crate::ids::{CardId, PlayerId, reset_id_counters};
    use crate::types::CardType;
    use crate::zone::Zone;

    use super::super::ActionKind;
    use super::super::crypto::Secp256k1Signer;
    use super::super::crypto::Signer;
    use super::super::message::build_action_propose_for_game;
    use super::super::message::build_contrib_request_for_game;
    use super::super::{ActionPayload, ProofBundle, Sig64};

    struct DummySigner;

    impl Signer for DummySigner {
        fn sign(&self, _msg: &[u8]) -> Sig64 {
            Sig64([0u8; 64])
        }
    }

    fn test_creature_card() -> crate::card::Card {
        CardBuilder::new(CardId::new(), "Test Creature")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(3, 3))
            .build()
    }

    #[test]
    fn public_state_hash_deterministic_across_ordering() {
        reset_id_counters();
        let mut game1 = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let card = test_creature_card();
        let obj1 = game1.create_object_from_card(&card, PlayerId::from_index(0), Zone::Battlefield);
        let obj2 = game1.create_object_from_card(&card, PlayerId::from_index(0), Zone::Battlefield);
        game1.cant_effects.add_cant_attack(obj1);
        game1.cant_effects.add_cant_attack(obj2);
        game1.activated_abilities_this_turn.insert((obj2, 1));
        game1.activated_abilities_this_turn.insert((obj1, 0));
        let hash1 = hash_public_state(&game1);

        reset_id_counters();
        let mut game2 = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let card2 = test_creature_card();
        let obj1b =
            game2.create_object_from_card(&card2, PlayerId::from_index(0), Zone::Battlefield);
        let obj2b =
            game2.create_object_from_card(&card2, PlayerId::from_index(0), Zone::Battlefield);
        game2.cant_effects.add_cant_attack(obj2b);
        game2.cant_effects.add_cant_attack(obj1b);
        game2.activated_abilities_this_turn.insert((obj1b, 0));
        game2.activated_abilities_this_turn.insert((obj2b, 1));
        let hash2 = hash_public_state(&game2);

        assert_eq!(hash1, hash2);
    }

    #[test]
    fn public_state_face_down_redacts_card_ref() {
        reset_id_counters();
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let card = test_creature_card();
        let obj_id =
            game.create_object_from_card(&card, PlayerId::from_index(0), Zone::Battlefield);

        let snapshot_face_up = build_public_state_snapshot(&game);
        let face_up = snapshot_face_up
            .objects
            .iter()
            .find(|obj| obj.id == GameObjectId::from(obj_id))
            .expect("object should be in snapshot");
        let hash_face_up = face_up.object_hash;

        game.set_face_down(obj_id);

        let snapshot_face_down = build_public_state_snapshot(&game);
        let face_down = snapshot_face_down
            .objects
            .iter()
            .find(|obj| obj.id == GameObjectId::from(obj_id))
            .expect("object should be in snapshot");

        assert!(face_down.card_ref.is_none());
        assert_ne!(hash_face_up, face_down.object_hash);
    }

    #[test]
    fn build_action_propose_for_game_uses_state_root() {
        reset_id_counters();
        let game = GameState::new(vec!["Alice".to_string()], 20);
        let signer = DummySigner;
        let propose = build_action_propose_for_game(
            &signer,
            &game,
            ActionPayload::PassPriority { policy_id: None },
            ProofBundle::default(),
            None,
        );

        assert_eq!(propose.prev_state_hash, hash_public_state(&game));
    }

    #[test]
    fn verify_action_propose_for_game_detects_mismatch() {
        reset_id_counters();
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let signer = Secp256k1Signer::from_secret_bytes([7u8; 32]).expect("signer");
        let pubkey = signer.public_key();
        let propose = build_action_propose_for_game(
            &signer,
            &game,
            ActionPayload::PassPriority { policy_id: None },
            ProofBundle::default(),
            None,
        );

        assert!(verify_action_propose_for_game(&game, pubkey, &propose).is_ok());

        game.players[0].life -= 1;
        let err =
            verify_action_propose_for_game(&game, pubkey, &propose).expect_err("expected mismatch");
        assert!(matches!(err, ActionVerifyError::PrevState(_)));
    }

    #[test]
    fn verify_contrib_request_for_game_detects_mismatch() {
        reset_id_counters();
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let signer = Secp256k1Signer::from_secret_bytes([9u8; 32]).expect("signer");
        let pubkey = signer.public_key();
        let request =
            build_contrib_request_for_game(&signer, &game, ActionKind::DrawCard, vec![], 1_000);

        assert!(verify_contrib_request_for_game(&game, pubkey, &request).is_ok());

        game.players[0].life -= 1;
        let err = verify_contrib_request_for_game(&game, pubkey, &request)
            .expect_err("expected mismatch");
        assert!(matches!(err, ContribVerifyError::PrevState(_)));
    }
}
