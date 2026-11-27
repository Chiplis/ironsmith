//! CR 732.2 loop shortcuts: recognise a sequence of player actions that
//! returned the game to an equivalent state, and let the player who performed
//! it repeat the sequence a chosen number of times.
//!
//! # What is recorded
//!
//! Every outermost call into the priority loop (`advance_priority_with_dm`,
//! `apply_priority_response_with_dm`, `apply_decision_context_with_dm`) is one
//! *step*: the call itself plus every answer its decision maker gave. Object
//! references are stored as [`ObjKey`]s (a card's stable identity, or a
//! token's public description) so a step can be replayed after the objects it
//! named changed zones or were re-created.
//!
//! # What "the same state modulo a delta" means
//!
//! A [`LoopStateSummary`] splits the visible game state in two.
//!
//! *Fixed* (must be identical): turn, phase, step, active and priority player;
//! the count of irreversible random operations; every library's exact card
//! order; for every card outside a library its zone, owner, controller,
//! tapped / summoning-sick / face-down / flipped / phased status, marked
//! damage, name and attachment; and the stack (source, controller, ability,
//! targets). Hands and libraries are therefore part of the fixed state: a
//! lap that draws, mills, shuffles or flips a coin is never a loop here.
//!
//! *Quantities* (may differ; the difference is the per-lap delta): life,
//! poison and other player counters, each colour of floating mana, restricted
//! mana, counters on each card, power and toughness of battlefield creatures,
//! and how many tokens / spell copies / emblems of each public description
//! exist.
//!
//! Deliberately not compared: per-turn tallies (spells cast, abilities
//! activated, life gained this turn, ...). They can make a later lap illegal
//! or different. That is safe because the summary only decides when to
//! *offer*; correctness comes from execution (below).
//!
//! # Offer
//!
//! When a priority prompt for player P is reached and the state matches an
//! earlier point this turn where P took a non-pass action, the steps in
//! between are a candidate lap. It is offered only if every non-pass step and
//! every answer in it is P's (other players only passed priority), every step
//! is replayable, and a dry run of one more lap on a copy of the game is
//! legal and produces exactly the same delta. The offer is the
//! [`LegalAction::ShortcutLoop`] action in P's priority menu.
//!
//! # Execution
//!
//! Taking the action asks P for a number of laps (0 declines). Each other
//! player who received priority during the lap is then asked, in turn order,
//! whether they want to respond instead and after how many laps (CR 732.2b);
//! the shortest count wins. The laps are then *really executed*, one by one,
//! through the ordinary priority functions with a scripted decision maker.
//! Before each lap the game is checkpointed; a lap that hits an illegal
//! action, an unexpected prompt, or a delta different from the recorded one is
//! rolled back and the shortcut stops there. The result is therefore always a
//! state reachable by performing whole laps by hand.

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use super::{GameLoopError, PriorityLoopState, PriorityResponse};
use crate::decision::{DecisionMaker, GameProgress, LegalAction};
use crate::decisions::context::{self, DecisionContext};
use crate::game_state::{GameState, Target};
use crate::ids::{ObjectId, PlayerId, StableId};
use crate::object::{AttachmentTarget, Object, ObjectKind};
use crate::triggers::TriggerQueue;
use crate::zone::Zone;

/// Most laps one shortcut executes. Laps are really performed, so the bound
/// keeps one action's cost predictable; the offer stays available afterwards.
pub const MAX_LOOP_SHORTCUT_LAPS: u32 = 1000;

const MAX_RECORDED_STEPS: usize = 256;

// ---------------------------------------------------------------------------
// Object references that survive zone changes and token re-creation
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum ObjKey {
    /// A card, by stable identity, in the zone it was in when referenced.
    Card(StableId, Zone),
    /// A token, spell copy or emblem, by public description.
    Other(u64, Zone),
    Missing,
}

fn counters_hash(object: &Object, hasher: &mut DefaultHasher) {
    for (kind, count) in object.counters.counts() {
        if *count > 0 {
            kind.hash(hasher);
            count.hash(hasher);
        }
    }
}

fn attachment_hash(game: &GameState, object: &Object, hasher: &mut DefaultHasher) {
    match object.attached_to {
        None => 0u8.hash(hasher),
        Some(AttachmentTarget::Player(player)) => (1u8, player).hash(hasher),
        Some(AttachmentTarget::Object(host)) => match game.object(host) {
            Some(host) if host.kind == ObjectKind::Card => (2u8, host.stable_id).hash(hasher),
            Some(host) => (3u8, host.name.to_string(), game.controller_of(host)).hash(hasher),
            None => 4u8.hash(hasher),
        },
    }
}

fn status_hash(game: &GameState, object: &Object, hasher: &mut DefaultHasher) {
    object.zone.hash(hasher);
    object.owner.hash(hasher);
    object.name.to_string().hash(hasher);
    if object.zone == Zone::Battlefield {
        game.controller_of(object).hash(hasher);
        game.is_tapped(object.id).hash(hasher);
        game.is_summoning_sick(object.id).hash(hasher);
        game.is_face_down(object.id).hash(hasher);
        game.is_flipped(object.id).hash(hasher);
        game.is_phased_out(object.id).hash(hasher);
        game.damage_on(object.id).hash(hasher);
    }
    attachment_hash(game, object, hasher);
}

/// Public description of a non-card object; equal descriptions are
/// interchangeable for loop purposes.
fn other_descriptor(game: &GameState, object: &Object) -> u64 {
    let mut hasher = DefaultHasher::new();
    object.kind.name().hash(&mut hasher);
    status_hash(game, object, &mut hasher);
    counters_hash(object, &mut hasher);
    format!("{:?}{:?}", object.base_power, object.base_toughness).hash(&mut hasher);
    hasher.finish()
}

fn obj_key(game: &GameState, id: ObjectId) -> ObjKey {
    match game.object(id) {
        Some(object) if object.kind == ObjectKind::Card => {
            ObjKey::Card(object.stable_id, object.zone)
        }
        Some(object) => ObjKey::Other(other_descriptor(game, object), object.zone),
        None => ObjKey::Missing,
    }
}

fn resolve_obj(game: &GameState, key: &ObjKey, used: &[ObjectId]) -> Option<ObjectId> {
    match key {
        ObjKey::Card(stable, zone) => game
            .find_object_by_stable_id(*stable)
            .filter(|id| !used.contains(id))
            .filter(|id| game.object(*id).is_some_and(|object| object.zone == *zone)),
        ObjKey::Other(descriptor, zone) => game
            .objects_in_deterministic_order()
            .into_iter()
            .find(|object| {
                object.kind != ObjectKind::Card
                    && object.zone == *zone
                    && !used.contains(&object.id)
                    && other_descriptor(game, object) == *descriptor
            })
            .map(|object| object.id),
        ObjKey::Missing => None,
    }
}

fn resolve_objs(game: &GameState, keys: &[ObjKey]) -> Option<Vec<ObjectId>> {
    let mut out = Vec::with_capacity(keys.len());
    for key in keys {
        let id = resolve_obj(game, key, &out)?;
        out.push(id);
    }
    Some(out)
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum TargetKey {
    Object(ObjKey),
    Player(PlayerId),
}

fn target_key(game: &GameState, target: &Target) -> TargetKey {
    match target {
        Target::Object(id) => TargetKey::Object(obj_key(game, *id)),
        Target::Player(player) => TargetKey::Player(*player),
    }
}

fn resolve_targets(game: &GameState, keys: &[TargetKey]) -> Option<Vec<Target>> {
    let mut used = Vec::new();
    let mut out = Vec::with_capacity(keys.len());
    for key in keys {
        out.push(match key {
            TargetKey::Player(player) => Target::Player(*player),
            TargetKey::Object(key) => {
                // The same object may legally be named twice (two requirements).
                let id = resolve_obj(game, key, &used).or_else(|| resolve_obj(game, key, &[]))?;
                used.push(id);
                Target::Object(id)
            }
        });
    }
    Some(out)
}

// ---------------------------------------------------------------------------
// Recorded calls and answers
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
enum ActionKey {
    Pass,
    Cast {
        spell: ObjKey,
        from_zone: Zone,
        casting_method: MethodKey,
    },
    Activate { source: ObjKey, ability_index: usize },
    ActivateMana { source: ObjKey, ability_index: usize },
    Unsupported,
}

#[derive(Debug, Clone, PartialEq)]
enum MethodKey {
    /// A casting method that names no other object.
    Plain(crate::alternative_cast::CastingMethod),
    /// A permission to cast from a zone, by the object that grants it.
    PlayFrom {
        source: ObjKey,
        zone: Zone,
        use_alternative: Option<usize>,
    },
}

fn action_key(game: &GameState, action: &LegalAction) -> ActionKey {
    use crate::alternative_cast::CastingMethod;
    match action {
        LegalAction::PassPriority => ActionKey::Pass,
        LegalAction::CastSpell {
            spell_id,
            from_zone,
            casting_method,
        } => match casting_method {
            // Methods that name another object are not replayed.
            CastingMethod::Normal
            | CastingMethod::Alternative(_)
            | CastingMethod::GrantedFlashback
            | CastingMethod::SplitOtherHalf
            | CastingMethod::Fuse => ActionKey::Cast {
                spell: obj_key(game, *spell_id),
                from_zone: *from_zone,
                casting_method: MethodKey::Plain(casting_method.clone()),
            },
            CastingMethod::PlayFrom {
                source,
                zone,
                use_alternative,
            } => ActionKey::Cast {
                spell: obj_key(game, *spell_id),
                from_zone: *from_zone,
                casting_method: MethodKey::PlayFrom {
                    source: obj_key(game, *source),
                    zone: *zone,
                    use_alternative: *use_alternative,
                },
            },
            // Other methods carry exact permissions or prices that are
            // chosen per cast; they are not replayed.
            _ => ActionKey::Unsupported,
        },
        LegalAction::ActivateAbility {
            source,
            ability_index,
        } => ActionKey::Activate {
            source: obj_key(game, *source),
            ability_index: *ability_index,
        },
        LegalAction::ActivateManaAbility {
            source,
            ability_index,
        } => ActionKey::ActivateMana {
            source: obj_key(game, *source),
            ability_index: *ability_index,
        },
        _ => ActionKey::Unsupported,
    }
}

fn resolve_action(game: &GameState, key: &ActionKey) -> Option<LegalAction> {
    Some(match key {
        ActionKey::Pass => LegalAction::PassPriority,
        ActionKey::Cast {
            spell,
            from_zone,
            casting_method,
        } => LegalAction::CastSpell {
            spell_id: resolve_obj(game, spell, &[])?,
            from_zone: *from_zone,
            casting_method: match casting_method {
                MethodKey::Plain(method) => method.clone(),
                MethodKey::PlayFrom {
                    source,
                    zone,
                    use_alternative,
                } => crate::alternative_cast::CastingMethod::PlayFrom {
                    source: resolve_obj(game, source, &[])?,
                    zone: *zone,
                    use_alternative: *use_alternative,
                },
            },
        },
        ActionKey::Activate {
            source,
            ability_index,
        } => LegalAction::ActivateAbility {
            source: resolve_obj(game, source, &[])?,
            ability_index: *ability_index,
        },
        ActionKey::ActivateMana {
            source,
            ability_index,
        } => LegalAction::ActivateManaAbility {
            source: resolve_obj(game, source, &[])?,
            ability_index: *ability_index,
        },
        ActionKey::Unsupported => return None,
    })
}

#[derive(Debug, Clone, PartialEq)]
enum PaymentKey {
    Confirm,
    Activate { source: ObjKey, ability_index: usize },
    Unsupported,
}

fn payment_key(game: &GameState, response: &crate::mana_payment::ManaPaymentResponse) -> PaymentKey {
    use crate::mana_payment::ManaPaymentResponse;
    match response {
        ManaPaymentResponse::Confirm { .. } => PaymentKey::Confirm,
        ManaPaymentResponse::Activate {
            source,
            ability_index,
        } => PaymentKey::Activate {
            source: obj_key(game, *source),
            ability_index: *ability_index,
        },
        // A replan carries plan-specific identifiers; a cancel is not a lap.
        _ => PaymentKey::Unsupported,
    }
}

fn resolve_payment(
    game: &GameState,
    key: &PaymentKey,
    plan: Option<(u64, u64)>,
) -> Option<crate::mana_payment::ManaPaymentResponse> {
    use crate::mana_payment::ManaPaymentResponse;
    Some(match key {
        PaymentKey::Confirm => {
            let (plan_id, request_hash) = plan?;
            ManaPaymentResponse::Confirm {
                plan_id,
                request_hash,
            }
        }
        PaymentKey::Activate {
            source,
            ability_index,
        } => ManaPaymentResponse::Activate {
            source: resolve_obj(game, source, &[])?,
            ability_index: *ability_index,
        },
        PaymentKey::Unsupported => return None,
    })
}

#[derive(Debug, Clone, PartialEq)]
enum ResponseKey {
    Action(ActionKey),
    Targets(Vec<TargetKey>),
    Distribution(Vec<(TargetKey, u32)>),
    XValue(u32),
    NumberChoice(u32),
    Modes(Vec<usize>),
    OptionalCosts(Vec<(usize, u32)>),
    ManaPayment(PaymentKey),
    NextCostChoice(usize),
    SacrificeTarget(ObjKey),
    CardCostChoice(ObjKey),
    HybridChoice(usize),
    CastingMethodChoice(usize),
    Unsupported,
}

fn response_key(game: &GameState, response: &PriorityResponse) -> ResponseKey {
    match response {
        PriorityResponse::PriorityAction(action) => ResponseKey::Action(action_key(game, action)),
        PriorityResponse::Targets(targets) => {
            ResponseKey::Targets(targets.iter().map(|target| target_key(game, target)).collect())
        }
        PriorityResponse::Distribution(parts) => ResponseKey::Distribution(
            parts
                .iter()
                .map(|(target, amount)| (target_key(game, target), *amount))
                .collect(),
        ),
        PriorityResponse::XValue(value) => ResponseKey::XValue(*value),
        PriorityResponse::NumberChoice(value) => ResponseKey::NumberChoice(*value),
        PriorityResponse::Modes(modes) => ResponseKey::Modes(modes.clone()),
        PriorityResponse::OptionalCosts(costs) => ResponseKey::OptionalCosts(costs.clone()),
        PriorityResponse::ManaPaymentPlan(payment) => {
            ResponseKey::ManaPayment(payment_key(game, payment))
        }
        PriorityResponse::NextCostChoice(choice) => ResponseKey::NextCostChoice(*choice),
        PriorityResponse::SacrificeTarget(id) => ResponseKey::SacrificeTarget(obj_key(game, *id)),
        PriorityResponse::CardCostChoice(id) => ResponseKey::CardCostChoice(obj_key(game, *id)),
        PriorityResponse::HybridChoice(choice) => ResponseKey::HybridChoice(*choice),
        PriorityResponse::CastingMethodChoice(choice) => ResponseKey::CastingMethodChoice(*choice),
        _ => ResponseKey::Unsupported,
    }
}

fn plan_identity(progress: Option<&GameProgress>) -> Option<(u64, u64)> {
    match progress {
        Some(GameProgress::NeedsDecisionCtx(DecisionContext::ManaPayment(ctx))) => {
            Some((ctx.plan.id, ctx.plan.request_hash))
        }
        _ => None,
    }
}

fn resolve_response(
    game: &GameState,
    key: &ResponseKey,
    plan: Option<(u64, u64)>,
) -> Option<PriorityResponse> {
    Some(match key {
        ResponseKey::Action(action) => PriorityResponse::PriorityAction(resolve_action(game, action)?),
        ResponseKey::Targets(targets) => PriorityResponse::Targets(resolve_targets(game, targets)?),
        ResponseKey::Distribution(parts) => {
            let keys: Vec<TargetKey> = parts.iter().map(|(key, _)| key.clone()).collect();
            let targets = resolve_targets(game, &keys)?;
            PriorityResponse::Distribution(
                targets
                    .into_iter()
                    .zip(parts.iter().map(|(_, amount)| *amount))
                    .collect(),
            )
        }
        ResponseKey::XValue(value) => PriorityResponse::XValue(*value),
        ResponseKey::NumberChoice(value) => PriorityResponse::NumberChoice(*value),
        ResponseKey::Modes(modes) => PriorityResponse::Modes(modes.clone()),
        ResponseKey::OptionalCosts(costs) => PriorityResponse::OptionalCosts(costs.clone()),
        ResponseKey::ManaPayment(payment) => {
            PriorityResponse::ManaPaymentPlan(resolve_payment(game, payment, plan)?)
        }
        ResponseKey::NextCostChoice(choice) => PriorityResponse::NextCostChoice(*choice),
        ResponseKey::SacrificeTarget(key) => {
            PriorityResponse::SacrificeTarget(resolve_obj(game, key, &[])?)
        }
        ResponseKey::CardCostChoice(key) => {
            PriorityResponse::CardCostChoice(resolve_obj(game, key, &[])?)
        }
        ResponseKey::HybridChoice(choice) => PriorityResponse::HybridChoice(*choice),
        ResponseKey::CastingMethodChoice(choice) => PriorityResponse::CastingMethodChoice(*choice),
        ResponseKey::Unsupported => return None,
    })
}

#[derive(Debug, Clone, PartialEq)]
enum Call {
    /// `advance_priority_with_dm`.
    Advance,
    /// `apply_priority_response_with_dm` with this response.
    Response(ResponseKey),
    /// `apply_decision_context_with_dm` on the pending context; the first
    /// recorded answer is the answer to that context.
    Context,
}

#[derive(Debug, Clone, PartialEq)]
enum AnswerKind {
    Boolean(bool),
    Number(u32),
    Text(String),
    Objects(Vec<ObjKey>),
    Options(Vec<usize>),
    ManaPayment(PaymentKey),
    Order(Vec<ObjKey>),
    Distribute(Vec<(TargetKey, u32)>),
    Colors(Vec<crate::color::Color>),
    Counters(Vec<(crate::object::CounterType, u32)>),
    Partition(Vec<ObjKey>),
    Proliferate(Vec<ObjKey>, Vec<PlayerId>),
    Priority(ActionKey),
    Targets(Vec<TargetKey>),
    /// A decision this module does not replay (combat declarations).
    Unsupported,
}

#[derive(Debug, Clone, PartialEq)]
struct Answer {
    player: PlayerId,
    kind: AnswerKind,
}

#[derive(Debug, Clone)]
struct Step {
    call: Call,
    /// The player with priority when the call was made.
    actor: Option<PlayerId>,
    answers: Vec<Answer>,
    /// State before the call, when the call began a non-pass priority action.
    anchor: Option<Arc<LoopStateSummary>>,
    /// The decision maker supplied something this module cannot reproduce.
    opaque: bool,
    /// The tracker's step counter when this step was committed.
    serial: u64,
}

impl Step {
    /// A priority pass. Answers recorded with it were asked while the pass
    /// resolved the top of the stack; they are not the passer's.
    fn is_pass(&self) -> bool {
        match &self.call {
            Call::Response(ResponseKey::Action(ActionKey::Pass)) => true,
            Call::Context => matches!(
                self.answers.first(),
                Some(Answer {
                    kind: AnswerKind::Priority(ActionKey::Pass),
                    ..
                })
            ),
            _ => false,
        }
    }

    /// Answers other than the pass itself.
    fn choices(&self) -> &[Answer] {
        match &self.call {
            Call::Context if self.is_pass() => &self.answers[1..],
            _ => &self.answers,
        }
    }

    fn replayable(&self) -> bool {
        if self.opaque {
            return false;
        }
        let call_ok = match &self.call {
            Call::Response(ResponseKey::Unsupported)
            | Call::Response(ResponseKey::Action(ActionKey::Unsupported))
            | Call::Response(ResponseKey::ManaPayment(PaymentKey::Unsupported)) => false,
            Call::Context => !self.answers.is_empty(),
            _ => true,
        };
        call_ok
            && self.answers.iter().all(|answer| {
                !matches!(
                    answer.kind,
                    AnswerKind::Unsupported
                        | AnswerKind::Priority(ActionKey::Unsupported)
                        | AnswerKind::ManaPayment(PaymentKey::Unsupported)
                )
            })
    }
}

// ---------------------------------------------------------------------------
// State summary
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum Quantity {
    Life(PlayerId),
    Poison(PlayerId),
    Energy(PlayerId),
    Experience(PlayerId),
    PlayerCounter(PlayerId, String),
    /// Floating mana by colour index (W U B R G C).
    Mana(PlayerId, u8),
    RestrictedMana(PlayerId, String),
    Counter(StableId, String, String),
    Power(StableId, String),
    Toughness(StableId, String),
    /// Tokens, spell copies and emblems, by description hash and label.
    Other(u64, String),
}

/// The parts of the state that must repeat exactly, for diagnostics.
const FIXED_SECTIONS: [&str; 6] = [
    "turn, step or priority",
    "random operations",
    "libraries or players",
    "a card's zone or status",
    "a permanent's types",
    "the stack",
];

const MANA_NAMES: [&str; 6] = ["white", "blue", "black", "red", "green", "colorless"];

/// The visible game state split into a part that must repeat exactly and
/// quantities whose change is a loop's per-lap delta. See the module docs.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct LoopStateSummary {
    /// One fingerprint per section named in `FIXED_SECTIONS`.
    fixed: [u64; 6],
    quantities: BTreeMap<Quantity, i64>,
}

type Delta = Vec<(Quantity, i64)>;

impl LoopStateSummary {
    fn of(game: &GameState) -> Self {
        let mut turn = DefaultHasher::new();
        let mut players = DefaultHasher::new();
        let mut cards = DefaultHasher::new();
        let mut types = DefaultHasher::new();
        let mut stack = DefaultHasher::new();
        let mut quantities = BTreeMap::new();
        let mut add = |quantity: Quantity, amount: i64| {
            if amount != 0 {
                *quantities.entry(quantity).or_insert(0) += amount;
            }
        };
        game.turn.turn_number.hash(&mut turn);
        format!("{:?}{:?}", game.turn.phase, game.turn.step).hash(&mut turn);
        game.turn.active_player.hash(&mut turn);
        game.turn.priority_player.hash(&mut turn);
        let random = game.irreversible_random_count();
        for player in game.players.iter() {
            player.id.hash(&mut players);
            (player.has_lost, player.has_won, player.has_left_game).hash(&mut players);
            for id in player.library.iter() {
                id.hash(&mut players);
            }
            0xFFu8.hash(&mut players);
            add(Quantity::Life(player.id), i64::from(player.life));
            add(Quantity::Poison(player.id), i64::from(player.poison_counters));
            add(Quantity::Energy(player.id), i64::from(player.energy_counters));
            add(
                Quantity::Experience(player.id),
                i64::from(player.experience_counters),
            );
            for (kind, count) in &player.other_counters {
                add(
                    Quantity::PlayerCounter(player.id, format!("{kind:?}")),
                    i64::from(*count),
                );
            }
            let pool = &player.mana_pool;
            for (index, amount) in [
                pool.white,
                pool.blue,
                pool.black,
                pool.red,
                pool.green,
                pool.colorless,
            ]
            .into_iter()
            .enumerate()
            {
                add(
                    Quantity::Mana(player.id, index as u8),
                    i64::try_from(amount).unwrap_or(i64::MAX),
                );
            }
            for unit in &player.restricted_mana {
                add(
                    Quantity::RestrictedMana(
                        player.id,
                        format!("{:?} {:?}", unit.symbol, unit.restrictions),
                    ),
                    1,
                );
            }
        }
        for object in game.objects_in_deterministic_order() {
            if object.zone == Zone::Library {
                continue;
            }
            if object.kind != ObjectKind::Card {
                let label = format!("{} ({})", object.name, object.kind.name());
                add(Quantity::Other(other_descriptor(game, object), label), 1);
                continue;
            }
            object.stable_id.hash(&mut cards);
            status_hash(game, object, &mut cards);
            let name = object.name.to_string();
            for (kind, count) in object.counters.counts() {
                add(
                    Quantity::Counter(object.stable_id, name.clone(), format!("{kind:?}")),
                    i64::from(*count),
                );
            }
            if object.zone == Zone::Battlefield
                && let Some(chars) = game.current_characteristics(object.id)
            {
                object.stable_id.hash(&mut types);
                format!("{:?}{:?}", chars.card_types, chars.subtypes).hash(&mut types);
                if let (Some(power), Some(toughness)) = (chars.power, chars.toughness) {
                    add(
                        Quantity::Power(object.stable_id, name.clone()),
                        i64::from(power),
                    );
                    add(
                        Quantity::Toughness(object.stable_id, name.clone()),
                        i64::from(toughness),
                    );
                }
            }
        }
        for entry in game.stack.iter() {
            match entry.source_stable_id {
                Some(stable) => stable.hash(&mut stack),
                None => obj_key(game, entry.object_id).hash(&mut stack),
            }
            entry.controller.hash(&mut stack);
            (entry.is_ability, entry.ability_index, entry.x_value).hash(&mut stack);
            for target in &entry.targets {
                target_key(game, target).hash(&mut stack);
            }
        }
        Self {
            fixed: [
                turn.finish(),
                random,
                players.finish(),
                cards.finish(),
                types.finish(),
                stack.finish(),
            ],
            quantities,
        }
    }

    /// Names of the fixed sections in which `self` and `later` differ.
    fn fixed_differences(&self, later: &Self) -> Vec<&'static str> {
        FIXED_SECTIONS
            .iter()
            .zip(self.fixed.iter().zip(later.fixed.iter()))
            .filter(|(_, (before, after))| before != after)
            .map(|(name, _)| *name)
            .collect()
    }

    /// The change from `self` to `later` when the fixed parts agree.
    fn delta_to(&self, later: &Self) -> Option<Delta> {
        if self.fixed != later.fixed {
            return None;
        }
        let mut delta = Vec::new();
        for (quantity, before) in &self.quantities {
            let after = later.quantities.get(quantity).copied().unwrap_or(0);
            if after != *before {
                delta.push((quantity.clone(), after - before));
            }
        }
        for (quantity, after) in &later.quantities {
            if !self.quantities.contains_key(quantity) {
                delta.push((quantity.clone(), *after));
            }
        }
        delta.sort();
        Some(delta)
    }
}

// ---------------------------------------------------------------------------
// Public offer / report types
// ---------------------------------------------------------------------------

/// One resource a lap changes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoopDeltaEntry {
    /// Player whose resource changes, when it belongs to a player.
    pub player: Option<PlayerId>,
    /// What changes ("life", "colorless mana", "Treasure (token)", ...).
    pub resource: String,
    /// Signed change per lap.
    pub amount: i64,
}

/// A detected loop the proposer may shortcut.
#[derive(Debug, Clone)]
pub struct LoopShortcutOffer {
    /// The player who performed the loop and may repeat it.
    pub proposer: PlayerId,
    /// What one lap changes.
    pub per_lap: Vec<LoopDeltaEntry>,
    /// Other players who received priority during the lap, in turn order.
    /// Each may shorten the shortcut (CR 732.2b).
    pub responders: Vec<PlayerId>,
    /// Number of recorded engine calls in one lap.
    pub steps: usize,
    /// `GameState::loop_shortcut_serial` as it was when the lap's first
    /// action was taken. A driver that keeps its own log of submitted
    /// commands, tagged with the serial they were submitted against, finds the
    /// lap as the commands tagged with this serial or a later one.
    pub start_serial: u64,
    lap: Arc<[Step]>,
    delta: Delta,
}

impl LoopShortcutOffer {
    /// "+3 colorless mana (player 0), -1 life (player 1)".
    pub fn describe_per_lap(&self, game: &GameState) -> String {
        self.per_lap
            .iter()
            .map(|entry| {
                let owner = entry
                    .player
                    .and_then(|player| game.player(player))
                    .map(|player| format!(" ({})", player.name))
                    .unwrap_or_default();
                format!("{:+} {}{}", entry.amount, entry.resource, owner)
            })
            .collect::<Vec<_>>()
            .join(", ")
    }
}

/// Why a shortcut executed fewer laps than were requested.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoopShortcutStop {
    /// Every accepted lap was executed.
    Completed,
    /// The proposer chose zero laps.
    Declined,
    /// Another player chose to respond after this many laps (CR 732.2b).
    Shortened { by: PlayerId },
    /// A lap could not be repeated identically and was rolled back.
    Diverged { reason: String },
    /// The game ended during a lap.
    GameOver,
}

/// Outcome of the most recent shortcut.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoopShortcutReport {
    pub proposer: PlayerId,
    pub requested: u32,
    pub accepted: u32,
    pub executed: u32,
    pub stop: LoopShortcutStop,
}

fn delta_entries(delta: &Delta) -> Vec<LoopDeltaEntry> {
    delta
        .iter()
        .map(|(quantity, amount)| {
            let (player, resource) = match quantity {
                Quantity::Life(player) => (Some(*player), "life".to_string()),
                Quantity::Poison(player) => (Some(*player), "poison counters".to_string()),
                Quantity::Energy(player) => (Some(*player), "energy counters".to_string()),
                Quantity::Experience(player) => (Some(*player), "experience counters".to_string()),
                Quantity::PlayerCounter(player, kind) => (Some(*player), format!("{kind} counters")),
                Quantity::Mana(player, index) => (
                    Some(*player),
                    format!("{} mana", MANA_NAMES[usize::from(*index).min(5)]),
                ),
                Quantity::RestrictedMana(player, _) => (Some(*player), "restricted mana".to_string()),
                Quantity::Counter(_, name, kind) => (None, format!("{kind} counters on {name}")),
                Quantity::Power(_, name) => (None, format!("power of {name}")),
                Quantity::Toughness(_, name) => (None, format!("toughness of {name}")),
                Quantity::Other(_, label) => (None, label.clone()),
            };
            LoopDeltaEntry {
                player,
                resource,
                amount: *amount,
            }
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Tracker (lives in GameState so it follows every checkpoint and restore)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub(crate) struct LoopShortcutTracker {
    /// Detection is on by default; the priority action (`as_action`) is not.
    enabled: bool,
    /// Offer the loop as a priority action. Off when a driver only wants the
    /// detection and runs its own shortcut protocol.
    as_action: bool,
    /// Bumped on every committed step; detects a restore during a call.
    serial: u64,
    epoch: Option<(u32, String, PlayerId)>,
    steps: Vec<Step>,
    offer: Option<LoopShortcutOffer>,
    /// The serial at which detection last ran, so a driver that advances
    /// repeatedly at one prompt does not repeat a failed proof.
    checked_serial: Option<u64>,
    /// Why the last detection attempt made no offer.
    note: Option<String>,
    report: Option<LoopShortcutReport>,
}

impl Default for LoopShortcutTracker {
    fn default() -> Self {
        Self {
            enabled: true,
            as_action: false,
            serial: 0,
            epoch: None,
            steps: Vec::new(),
            offer: None,
            checked_serial: None,
            note: None,
            report: None,
        }
    }
}

impl LoopShortcutTracker {
    fn forget(&mut self) {
        self.steps.clear();
        self.offer = None;
    }
}

impl GameState {
    /// Turn CR 732.2 loop detection and the shortcut priority action on or
    /// off. A new game detects loops by default but does not add the action.
    pub fn set_loop_shortcuts_enabled(&mut self, enabled: bool) {
        if self.loop_shortcut.enabled == enabled && self.loop_shortcut.as_action == enabled {
            return;
        }
        self.loop_shortcut.enabled = enabled;
        self.loop_shortcut.as_action = enabled;
        if !enabled {
            self.loop_shortcut.forget();
        }
    }

    /// Detect loops and report them through [`GameState::loop_shortcut_offer`]
    /// without adding [`LegalAction::ShortcutLoop`] to any menu. For drivers
    /// that run their own agreement and replay (the browser session does, so
    /// that every repetition is an ordinary replicated command).
    pub fn set_loop_shortcut_detection_enabled(&mut self, enabled: bool) {
        if self.loop_shortcut.enabled == enabled && !self.loop_shortcut.as_action {
            return;
        }
        self.loop_shortcut.enabled = enabled;
        self.loop_shortcut.as_action = false;
        if !enabled {
            self.loop_shortcut.forget();
        }
    }

    /// Why the most recent look for a loop made no offer, for diagnostics
    /// ("why was my loop not offered?"). `None` after an offer or before any look.
    pub fn loop_shortcut_note(&self) -> Option<&str> {
        self.loop_shortcut.note.as_deref()
    }

    /// Counter of recorded steps; see [`LoopShortcutOffer::start_serial`].
    pub fn loop_shortcut_serial(&self) -> u64 {
        self.loop_shortcut.serial
    }

    pub fn loop_shortcuts_enabled(&self) -> bool {
        self.loop_shortcut.enabled
    }

    /// The loop currently offered to the player with priority, if any.
    pub fn loop_shortcut_offer(&self) -> Option<&LoopShortcutOffer> {
        self.loop_shortcut.offer.as_ref().filter(|offer| {
            self.loop_shortcut.enabled
                && self.turn.priority_player == Some(offer.proposer)
                && self.loop_shortcut.epoch.as_ref() == Some(&epoch_of(self))
        })
    }

    /// Outcome of the most recently taken shortcut.
    pub fn last_loop_shortcut_report(&self) -> Option<&LoopShortcutReport> {
        self.loop_shortcut.report.as_ref()
    }

    pub(crate) fn loop_shortcut_offered_to(&self, player: PlayerId) -> bool {
        self.loop_shortcut.enabled
            && self.loop_shortcut.as_action
            && self.turn.priority_player == Some(player)
            && self.loop_shortcut.epoch.as_ref() == Some(&epoch_of(self))
            && self
                .loop_shortcut
                .offer
                .as_ref()
                .is_some_and(|offer| offer.proposer == player)
    }
}

fn epoch_of(game: &GameState) -> (u32, String, PlayerId) {
    (
        game.turn.turn_number,
        format!("{:?}{:?}", game.turn.phase, game.turn.step),
        game.turn.active_player,
    )
}

// ---------------------------------------------------------------------------
// Recording
// ---------------------------------------------------------------------------

struct Frame {
    answers: Vec<Answer>,
    anchor: Option<Arc<LoopStateSummary>>,
    opaque: bool,
    discard: bool,
}

thread_local! {
    static DEPTH: Cell<u32> = const { Cell::new(0) };
    static FRAME: RefCell<Option<Frame>> = const { RefCell::new(None) };
}

struct DepthGuard;
impl DepthGuard {
    fn enter() -> Self {
        DEPTH.with(|depth| depth.set(depth.get() + 1));
        DepthGuard
    }
}
impl Drop for DepthGuard {
    fn drop(&mut self) {
        DEPTH.with(|depth| depth.set(depth.get().saturating_sub(1)));
    }
}

fn with_frame(apply: impl FnOnce(&mut Frame)) {
    FRAME.with(|frame| {
        if let Some(frame) = frame.borrow_mut().as_mut() {
            apply(frame);
        }
    });
}

pub(super) enum CallSpec<'a> {
    Advance,
    Response(&'a PriorityResponse),
    Context,
}

pub(super) struct CallScope {
    recording: bool,
    serial: u64,
    call: Option<Call>,
    actor: Option<PlayerId>,
    _depth: DepthGuard,
}

impl CallScope {
    pub(super) fn recording(&self) -> bool {
        self.recording
    }
}

/// Open the recording scope for one public priority-loop call. Nested calls
/// (the loop calling itself, a replay, a dry run) are never recorded.
pub(super) fn begin(game: &mut GameState, pending_action: bool, spec: CallSpec<'_>) -> CallScope {
    let outermost = DEPTH.with(Cell::get) == 0;
    let depth = DepthGuard::enter();
    if !outermost || !game.loop_shortcut.enabled {
        return CallScope {
            recording: false,
            serial: 0,
            call: None,
            actor: None,
            _depth: depth,
        };
    }
    let is_shortcut = matches!(
        spec,
        CallSpec::Response(PriorityResponse::PriorityAction(LegalAction::ShortcutLoop))
    );
    let epoch = epoch_of(game);
    if game.loop_shortcut.epoch.as_ref() != Some(&epoch) {
        game.loop_shortcut.forget();
        game.loop_shortcut.epoch = Some(epoch);
    }
    // An offer describes the state at one prompt. A shortcut response consumes
    // it; any other response invalidates it. Context and advance calls drop it
    // when they commit a step.
    if !is_shortcut && matches!(spec, CallSpec::Response(_)) {
        game.loop_shortcut.offer = None;
    }
    let mut anchor = None;
    let call = match spec {
        CallSpec::Advance => Call::Advance,
        CallSpec::Context => Call::Context,
        CallSpec::Response(response) => {
            if !pending_action
                && !is_shortcut
                && matches!(response, PriorityResponse::PriorityAction(action)
                    if !matches!(action, LegalAction::PassPriority))
            {
                anchor = Some(Arc::new(LoopStateSummary::of(game)));
            }
            Call::Response(response_key(game, response))
        }
    };
    FRAME.with(|frame| {
        *frame.borrow_mut() = Some(Frame {
            answers: Vec::new(),
            anchor,
            opaque: false,
            discard: is_shortcut,
        });
    });
    CallScope {
        recording: true,
        serial: game.loop_shortcut.serial,
        call: Some(call),
        actor: game.turn.priority_player,
        _depth: depth,
    }
}

/// Close the scope: commit the step, look for a loop, and add the offer to a
/// priority prompt that is about to be shown.
pub(super) fn finish(
    game: &mut GameState,
    trigger_queue: &TriggerQueue,
    state: Option<&PriorityLoopState>,
    scope: CallScope,
    mut result: Result<GameProgress, GameLoopError>,
    awaiting_choice: bool,
) -> Result<GameProgress, GameLoopError> {
    if !scope.recording {
        return result;
    }
    let frame = FRAME.with(|frame| frame.borrow_mut().take());
    let Some(frame) = frame else {
        return result;
    };
    let restored = game.loop_shortcut.serial != scope.serial;
    let completed = result.is_ok()
        && !awaiting_choice
        && !frame.discard
        && !restored
        && game.loop_shortcut.enabled;
    if completed {
        let step = Step {
            call: scope.call.clone().unwrap_or(Call::Advance),
            actor: scope.actor,
            answers: frame.answers,
            anchor: frame.anchor,
            opaque: frame.opaque,
            serial: game.loop_shortcut.serial,
        };
        // A bare advance that asked nothing changes nothing a replay needs.
        let trivial = matches!(step.call, Call::Advance) && step.answers.is_empty();
        if !trivial {
            let tracker = &mut game.loop_shortcut;
            tracker.offer = None;
            tracker.steps.push(step);
            tracker.serial += 1;
            if tracker.steps.len() > MAX_RECORDED_STEPS {
                let overflow = tracker.steps.len() - MAX_RECORDED_STEPS;
                tracker.steps.drain(..overflow);
            }
        }
        // A driver may reach the prompt through a later bare advance rather
        // than through the call that finished the lap, so look whenever a
        // prompt is reached without an offer.
        if game.loop_shortcut.offer.is_none()
            && game.loop_shortcut.checked_serial != Some(game.loop_shortcut.serial)
            && let Ok(progress @ GameProgress::NeedsDecisionCtx(DecisionContext::Priority(_))) =
                &result
            && !state.is_some_and(PriorityLoopState::has_pending_action)
        {
            let progress = progress.clone();
            game.loop_shortcut.checked_serial = Some(game.loop_shortcut.serial);
            if let GameProgress::NeedsDecisionCtx(DecisionContext::Priority(ctx)) = &progress {
                detect(game, trigger_queue, state, ctx.player, &progress);
            }
        }
    } else if result.is_err() && !frame.discard && !restored {
        // The call failed; whatever it left behind is not a state a recorded
        // lap can be replayed from.
        game.loop_shortcut.offer = None;
    }
    if let Ok(GameProgress::NeedsDecisionCtx(DecisionContext::Priority(ctx))) = &mut result
        && ctx.analysis_complete
        && game.loop_shortcut_offered_to(ctx.player)
        && !ctx.actions.contains(&LegalAction::ShortcutLoop)
    {
        ctx.actions.push(game, LegalAction::ShortcutLoop);
    }
    result
}

/// Mark the active call as one that must not become a recorded step.
fn discard_active_frame() {
    with_frame(|frame| frame.discard = true);
}

/// Decision maker wrapper that records answers into the active call's frame.
pub(super) struct RecordingDm<'a> {
    inner: &'a mut dyn DecisionMaker,
    record: bool,
}

impl<'a> RecordingDm<'a> {
    pub(super) fn new(inner: &'a mut dyn DecisionMaker, scope: &CallScope) -> Self {
        Self {
            inner,
            record: scope.recording(),
        }
    }

    fn push(&self, player: PlayerId, kind: AnswerKind) {
        if self.record {
            with_frame(|frame| frame.answers.push(Answer { player, kind }));
        }
    }
}

impl DecisionMaker for RecordingDm<'_> {
    fn planned_mana_output(
        &mut self,
        game: &GameState,
        choice: &crate::mana_payment::ManaProductionChoice,
    ) -> Result<Option<Vec<crate::mana::ManaSymbol>>, String> {
        let planned = self.inner.planned_mana_output(game, choice);
        if self.record && matches!(planned, Ok(Some(_))) {
            with_frame(|frame| frame.opaque = true);
        }
        planned
    }

    fn take_mana_replacement_witness(
        &mut self,
        game: &GameState,
        event: &crate::events::ManaAddedEvent,
    ) -> Result<Option<crate::mana_payment::ManaReplacementWitness>, String> {
        let witness = self.inner.take_mana_replacement_witness(game, event);
        if self.record && matches!(witness, Ok(Some(_))) {
            with_frame(|frame| frame.opaque = true);
        }
        witness
    }

    fn on_auto_pass(&mut self, game: &GameState, player: PlayerId) {
        self.inner.on_auto_pass(game, player);
    }

    fn on_action_cancelled(&mut self, game: &GameState, reason: &str) {
        self.inner.on_action_cancelled(game, reason);
    }

    fn awaiting_choice(&self) -> bool {
        self.inner.awaiting_choice()
    }

    fn answers_player_choices(&self) -> bool {
        self.inner.answers_player_choices()
    }

    fn decide_boolean(&mut self, game: &GameState, ctx: &context::BooleanContext) -> bool {
        let answer = self.inner.decide_boolean(game, ctx);
        self.push(ctx.player, AnswerKind::Boolean(answer));
        answer
    }

    fn decide_number(&mut self, game: &GameState, ctx: &context::NumberContext) -> u32 {
        let answer = self.inner.decide_number(game, ctx);
        self.push(ctx.player, AnswerKind::Number(answer));
        answer
    }

    fn decide_text(&mut self, game: &GameState, ctx: &context::TextInputContext) -> String {
        let answer = self.inner.decide_text(game, ctx);
        self.push(ctx.player, AnswerKind::Text(answer.clone()));
        answer
    }

    fn decide_objects(
        &mut self,
        game: &GameState,
        ctx: &context::SelectObjectsContext,
    ) -> Vec<ObjectId> {
        let answer = self.inner.decide_objects(game, ctx);
        if self.record {
            let keys = answer.iter().map(|id| obj_key(game, *id)).collect();
            self.push(ctx.player, AnswerKind::Objects(keys));
        }
        answer
    }

    fn decide_options(
        &mut self,
        game: &GameState,
        ctx: &context::SelectOptionsContext,
    ) -> Vec<usize> {
        let answer = self.inner.decide_options(game, ctx);
        self.push(ctx.player, AnswerKind::Options(answer.clone()));
        answer
    }

    fn decide_mana_payment(
        &mut self,
        game: &GameState,
        ctx: &context::ManaPaymentContext,
    ) -> crate::mana_payment::ManaPaymentResponse {
        let answer = self.inner.decide_mana_payment(game, ctx);
        if self.record {
            self.push(ctx.player, AnswerKind::ManaPayment(payment_key(game, &answer)));
        }
        answer
    }

    fn decide_order(&mut self, game: &GameState, ctx: &context::OrderContext) -> Vec<ObjectId> {
        let answer = self.inner.decide_order(game, ctx);
        if self.record {
            // Items being ordered are often stack entries, which have no
            // object identity; an order is replayed by position instead.
            let positions: Option<Vec<usize>> = answer
                .iter()
                .map(|id| ctx.items.iter().position(|(item, _)| item == id))
                .collect();
            match positions {
                Some(positions) => self.push(ctx.player, AnswerKind::Options(positions)),
                None => self.push(ctx.player, AnswerKind::Unsupported),
            }
        }
        answer
    }

    fn view_cards(
        &mut self,
        game: &GameState,
        viewer: PlayerId,
        cards: &[ObjectId],
        ctx: &context::ViewCardsContext,
    ) {
        self.inner.view_cards(game, viewer, cards, ctx);
    }

    fn decide_attackers(
        &mut self,
        game: &GameState,
        ctx: &context::AttackersContext,
    ) -> Vec<crate::decisions::spec::AttackerDeclaration> {
        self.push(ctx.player, AnswerKind::Unsupported);
        self.inner.decide_attackers(game, ctx)
    }

    fn decide_blockers(
        &mut self,
        game: &GameState,
        ctx: &context::BlockersContext,
    ) -> Vec<crate::decisions::spec::BlockerDeclaration> {
        self.push(ctx.player, AnswerKind::Unsupported);
        self.inner.decide_blockers(game, ctx)
    }

    fn decide_distribute(
        &mut self,
        game: &GameState,
        ctx: &context::DistributeContext,
    ) -> Vec<(Target, u32)> {
        let answer = self.inner.decide_distribute(game, ctx);
        if self.record {
            let keys = answer
                .iter()
                .map(|(target, amount)| (target_key(game, target), *amount))
                .collect();
            self.push(ctx.player, AnswerKind::Distribute(keys));
        }
        answer
    }

    fn decide_colors(
        &mut self,
        game: &GameState,
        ctx: &context::ColorsContext,
    ) -> Vec<crate::color::Color> {
        let answer = self.inner.decide_colors(game, ctx);
        self.push(ctx.player, AnswerKind::Colors(answer.clone()));
        answer
    }

    fn decide_counters(
        &mut self,
        game: &GameState,
        ctx: &context::CountersContext,
    ) -> Vec<(crate::object::CounterType, u32)> {
        let answer = self.inner.decide_counters(game, ctx);
        self.push(ctx.player, AnswerKind::Counters(answer.clone()));
        answer
    }

    fn decide_partition(
        &mut self,
        game: &GameState,
        ctx: &context::PartitionContext,
    ) -> Vec<ObjectId> {
        let answer = self.inner.decide_partition(game, ctx);
        if self.record {
            let keys = answer.iter().map(|id| obj_key(game, *id)).collect();
            self.push(ctx.player, AnswerKind::Partition(keys));
        }
        answer
    }

    fn decide_proliferate(
        &mut self,
        game: &GameState,
        ctx: &context::ProliferateContext,
    ) -> crate::decisions::specs::ProliferateResponse {
        let answer = self.inner.decide_proliferate(game, ctx);
        if self.record {
            let keys = answer.permanents.iter().map(|id| obj_key(game, *id)).collect();
            self.push(
                ctx.player,
                AnswerKind::Proliferate(keys, answer.players.clone()),
            );
        }
        answer
    }

    fn decide_priority(&mut self, game: &GameState, ctx: &context::PriorityContext) -> LegalAction {
        let answer = self.inner.decide_priority(game, ctx);
        if self.record {
            let first = FRAME.with(|frame| {
                frame
                    .borrow()
                    .as_ref()
                    .is_some_and(|frame| frame.answers.is_empty())
            });
            if matches!(answer, LegalAction::ShortcutLoop) {
                discard_active_frame();
            } else if first && !matches!(answer, LegalAction::PassPriority) {
                // `game` is still the state before the action is applied.
                let anchor = Arc::new(LoopStateSummary::of(game));
                with_frame(|frame| frame.anchor = Some(anchor));
            }
            self.push(ctx.player, AnswerKind::Priority(action_key(game, &answer)));
        }
        answer
    }

    fn decide_targets(&mut self, game: &GameState, ctx: &context::TargetsContext) -> Vec<Target> {
        let answer = self.inner.decide_targets(game, ctx);
        if self.record {
            let keys = answer.iter().map(|target| target_key(game, target)).collect();
            self.push(ctx.player, AnswerKind::Targets(keys));
        }
        answer
    }
}

// ---------------------------------------------------------------------------
// Replay
// ---------------------------------------------------------------------------

/// Answers a lap's decisions from its recording. Any mismatch marks the lap
/// failed; the caller then discards the lap's game state.
struct ScriptedDm<'a> {
    answers: &'a [Answer],
    next: usize,
    failure: Option<String>,
}

impl<'a> ScriptedDm<'a> {
    fn new(answers: &'a [Answer]) -> Self {
        Self {
            answers,
            next: 0,
            failure: None,
        }
    }

    fn fail(&mut self, reason: impl Into<String>) {
        if self.failure.is_none() {
            self.failure = Some(reason.into());
        }
    }

    fn take(&mut self, player: PlayerId, asked: &str) -> Option<&'a AnswerKind> {
        let Some(answer) = self.answers.get(self.next) else {
            self.fail(format!("an unrecorded {asked} decision was asked"));
            return None;
        };
        self.next += 1;
        if answer.player != player {
            self.fail(format!("a {asked} decision was asked of a different player"));
            return None;
        }
        Some(&answer.kind)
    }

    fn finished(&mut self) -> Result<(), String> {
        if let Some(failure) = self.failure.take() {
            return Err(failure);
        }
        if self.next != self.answers.len() {
            return Err("a recorded decision was not asked again".to_string());
        }
        Ok(())
    }
}

impl DecisionMaker for ScriptedDm<'_> {
    fn decide_boolean(&mut self, _game: &GameState, ctx: &context::BooleanContext) -> bool {
        match self.take(ctx.player, "yes/no") {
            Some(AnswerKind::Boolean(answer)) => *answer,
            Some(_) => {
                self.fail("a different kind of decision was asked");
                false
            }
            None => false,
        }
    }

    fn decide_number(&mut self, _game: &GameState, ctx: &context::NumberContext) -> u32 {
        match self.take(ctx.player, "number") {
            Some(AnswerKind::Number(answer)) if (ctx.min..=ctx.max).contains(answer) => *answer,
            Some(_) => {
                self.fail("the recorded number is no longer a legal choice");
                ctx.min
            }
            None => ctx.min,
        }
    }

    fn decide_text(&mut self, _game: &GameState, ctx: &context::TextInputContext) -> String {
        match self.take(ctx.player, "text") {
            Some(AnswerKind::Text(answer)) => answer.clone(),
            Some(_) => {
                self.fail("a different kind of decision was asked");
                String::new()
            }
            None => String::new(),
        }
    }

    fn decide_objects(
        &mut self,
        game: &GameState,
        ctx: &context::SelectObjectsContext,
    ) -> Vec<ObjectId> {
        let resolved = match self.take(ctx.player, "object") {
            Some(AnswerKind::Objects(keys)) => resolve_objs(game, keys),
            Some(_) => None,
            None => return Vec::new(),
        };
        match resolved {
            Some(ids)
                if ids.iter().all(|id| {
                    ctx.candidates
                        .iter()
                        .any(|candidate| candidate.id == *id && candidate.legal)
                }) =>
            {
                ids
            }
            _ => {
                self.fail("a recorded object choice is no longer available");
                Vec::new()
            }
        }
    }

    fn decide_options(
        &mut self,
        _game: &GameState,
        ctx: &context::SelectOptionsContext,
    ) -> Vec<usize> {
        match self.take(ctx.player, "option") {
            Some(AnswerKind::Options(answer))
                if answer.iter().all(|index| {
                    ctx.options
                        .iter()
                        .any(|option| option.index == *index && option.legal)
                }) =>
            {
                answer.clone()
            }
            Some(_) => {
                self.fail("a recorded option is no longer available");
                Vec::new()
            }
            None => Vec::new(),
        }
    }

    fn decide_mana_payment(
        &mut self,
        game: &GameState,
        ctx: &context::ManaPaymentContext,
    ) -> crate::mana_payment::ManaPaymentResponse {
        let resolved = match self.take(ctx.player, "mana payment") {
            Some(AnswerKind::ManaPayment(key)) => {
                resolve_payment(game, key, Some((ctx.plan.id, ctx.plan.request_hash)))
            }
            Some(_) => None,
            None => return crate::mana_payment::ManaPaymentResponse::Cancel,
        };
        resolved.unwrap_or_else(|| {
            self.fail("a recorded mana payment is no longer available");
            crate::mana_payment::ManaPaymentResponse::Cancel
        })
    }

    fn decide_order(&mut self, _game: &GameState, ctx: &context::OrderContext) -> Vec<ObjectId> {
        let fallback = || ctx.items.iter().map(|(id, _)| *id).collect::<Vec<_>>();
        match self.take(ctx.player, "ordering") {
            Some(AnswerKind::Options(positions))
                if positions.len() == ctx.items.len()
                    && positions.iter().all(|position| *position < ctx.items.len()) =>
            {
                positions.iter().map(|position| ctx.items[*position].0).collect()
            }
            Some(_) => {
                self.fail("a recorded ordering no longer applies");
                fallback()
            }
            None => fallback(),
        }
    }

    fn decide_attackers(
        &mut self,
        _game: &GameState,
        _ctx: &context::AttackersContext,
    ) -> Vec<crate::decisions::spec::AttackerDeclaration> {
        self.fail("combat declarations are not replayed");
        Vec::new()
    }

    fn decide_blockers(
        &mut self,
        _game: &GameState,
        _ctx: &context::BlockersContext,
    ) -> Vec<crate::decisions::spec::BlockerDeclaration> {
        self.fail("combat declarations are not replayed");
        Vec::new()
    }

    fn decide_distribute(
        &mut self,
        game: &GameState,
        ctx: &context::DistributeContext,
    ) -> Vec<(Target, u32)> {
        let resolved = match self.take(ctx.player, "distribution") {
            Some(AnswerKind::Distribute(parts)) => {
                let keys: Vec<TargetKey> = parts.iter().map(|(key, _)| key.clone()).collect();
                resolve_targets(game, &keys).map(|targets| {
                    targets
                        .into_iter()
                        .zip(parts.iter().map(|(_, amount)| *amount))
                        .collect::<Vec<_>>()
                })
            }
            Some(_) => None,
            None => return Vec::new(),
        };
        resolved.unwrap_or_else(|| {
            self.fail("a recorded distribution target is gone");
            Vec::new()
        })
    }

    fn decide_colors(
        &mut self,
        _game: &GameState,
        ctx: &context::ColorsContext,
    ) -> Vec<crate::color::Color> {
        match self.take(ctx.player, "color") {
            Some(AnswerKind::Colors(answer)) => answer.clone(),
            Some(_) => {
                self.fail("a different kind of decision was asked");
                vec![crate::color::Color::Green; ctx.count as usize]
            }
            None => vec![crate::color::Color::Green; ctx.count as usize],
        }
    }

    fn decide_counters(
        &mut self,
        _game: &GameState,
        ctx: &context::CountersContext,
    ) -> Vec<(crate::object::CounterType, u32)> {
        match self.take(ctx.player, "counter") {
            Some(AnswerKind::Counters(answer)) => answer.clone(),
            Some(_) => {
                self.fail("a different kind of decision was asked");
                Vec::new()
            }
            None => Vec::new(),
        }
    }

    fn decide_partition(
        &mut self,
        game: &GameState,
        ctx: &context::PartitionContext,
    ) -> Vec<ObjectId> {
        let resolved = match self.take(ctx.player, "partition") {
            Some(AnswerKind::Partition(keys)) => resolve_objs(game, keys),
            Some(_) => None,
            None => return Vec::new(),
        };
        resolved.unwrap_or_else(|| {
            self.fail("a recorded card choice is gone");
            Vec::new()
        })
    }

    fn decide_proliferate(
        &mut self,
        game: &GameState,
        ctx: &context::ProliferateContext,
    ) -> crate::decisions::specs::ProliferateResponse {
        let mut response = crate::decisions::specs::ProliferateResponse::default();
        match self.take(ctx.player, "proliferate") {
            Some(AnswerKind::Proliferate(keys, players)) => match resolve_objs(game, keys) {
                Some(permanents) => {
                    response.permanents = permanents;
                    response.players = players.clone();
                }
                None => self.fail("a recorded proliferate choice is gone"),
            },
            Some(_) => self.fail("a different kind of decision was asked"),
            None => {}
        }
        response
    }

    fn decide_priority(&mut self, game: &GameState, ctx: &context::PriorityContext) -> LegalAction {
        let resolved = match self.take(ctx.player, "priority") {
            Some(AnswerKind::Priority(key)) => resolve_action(game, key),
            Some(_) => None,
            None => return LegalAction::PassPriority,
        };
        match resolved {
            Some(action) if ctx.actions.contains(&action) => action,
            _ => {
                self.fail("a recorded action is no longer legal");
                LegalAction::PassPriority
            }
        }
    }

    fn decide_targets(&mut self, game: &GameState, ctx: &context::TargetsContext) -> Vec<Target> {
        let resolved = match self.take(ctx.player, "target") {
            Some(AnswerKind::Targets(keys)) => resolve_targets(game, keys),
            Some(_) => None,
            None => return Vec::new(),
        };
        resolved.unwrap_or_else(|| {
            self.fail("a recorded target is gone");
            Vec::new()
        })
    }
}

enum LapEnd {
    /// The lap ran to its end; the progress is the prompt that follows it.
    Completed(GameProgress),
    GameOver(GameProgress),
}

/// Bring the game to its next prompt the way any driver does after a stack
/// resolution or a completed action. Nothing may be asked on the way.
fn settle(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    state: &PriorityLoopState,
    progress: GameProgress,
) -> Result<GameProgress, String> {
    if !matches!(progress, GameProgress::Continue | GameProgress::StackResolved)
        || state.has_pending_action()
    {
        return Ok(progress);
    }
    let mut dm = ScriptedDm::new(&[]);
    let next = super::advance_priority_with_dm(game, trigger_queue, &mut dm)
        .map_err(|error| format!("the game could not continue: {error}"))?;
    dm.finished()?;
    Ok(next)
}

fn pending_plan_identity(state: &PriorityLoopState) -> Option<(u64, u64)> {
    let payment = state
        .pending_cast
        .as_ref()
        .and_then(|pending| pending.pending_mana_payment.as_ref())
        .or_else(|| {
            state
                .pending_activation
                .as_ref()
                .and_then(|pending| pending.pending_mana_payment.as_ref())
        })
        .or_else(|| {
            state
                .pending_mana_ability
                .as_ref()
                .and_then(|pending| pending.pending_mana_payment.as_ref())
        })?;
    Some((payment.plan.id, payment.plan.request_hash))
}

/// Perform one recorded lap. On `Err` the game is in an undefined
/// intermediate state and the caller must restore its checkpoint.
fn run_lap(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    state: &mut PriorityLoopState,
    lap: &[Step],
    proposer: PlayerId,
    mut progress: GameProgress,
) -> Result<LapEnd, String> {
    for step in lap {
        if !matches!(step.call, Call::Advance) {
            progress = settle(game, trigger_queue, state, progress)?;
        }
        if matches!(progress, GameProgress::GameOver(_)) {
            return Ok(LapEnd::GameOver(progress));
        }
        if game.turn.priority_player != step.actor {
            return Err("priority reached a different player".to_string());
        }
        let mut dm = ScriptedDm::new(&step.answers);
        let result = match &step.call {
            Call::Advance => super::advance_priority_with_dm(game, trigger_queue, &mut dm),
            Call::Response(key) => {
                let plan = plan_identity(Some(&progress)).or_else(|| pending_plan_identity(state));
                let response = resolve_response(game, key, plan)
                    .ok_or_else(|| "a recorded action names something that is gone".to_string())?;
                super::apply_priority_response_with_dm(game, trigger_queue, state, &response, &mut dm)
            }
            Call::Context => {
                let GameProgress::NeedsDecisionCtx(ctx) = &progress else {
                    return Err("an expected prompt was not raised".to_string());
                };
                super::apply_decision_context_with_dm(game, trigger_queue, state, ctx, &mut dm)
            }
        };
        progress = result.map_err(|error| format!("a recorded action failed: {error}"))?;
        // A game that ends mid-call legitimately leaves later answers unasked.
        if matches!(progress, GameProgress::GameOver(_)) {
            return Ok(LapEnd::GameOver(progress));
        }
        dm.finished()?;
    }
    progress = settle(game, trigger_queue, state, progress)?;
    match &progress {
        GameProgress::GameOver(_) => Ok(LapEnd::GameOver(progress)),
        GameProgress::NeedsDecisionCtx(DecisionContext::Priority(ctx))
            if ctx.player == proposer && !state.has_pending_action() =>
        {
            Ok(LapEnd::Completed(progress))
        }
        _ => Err("the repetition did not return priority to its player".to_string()),
    }
}

fn lap_shape(lap: &[Step], proposer: PlayerId) -> Option<Vec<PlayerId>> {
    let mut responders = Vec::new();
    let mut acted = false;
    for step in lap {
        if !step.replayable() {
            return None;
        }
        // Every choice in a lap is the proposer's, whoever's pass it was
        // asked under.
        if step.choices().iter().any(|answer| answer.player != proposer) {
            return None;
        }
        if step.is_pass() {
            if let Some(actor) = step.actor
                && actor != proposer
                && !responders.contains(&actor)
            {
                responders.push(actor);
            }
            continue;
        }
        if step.actor != Some(proposer) {
            return None;
        }
        acted |= !matches!(step.call, Call::Advance);
    }
    acted.then_some(responders)
}

/// Look for a lap ending at the prompt in `progress`, and prove it by a dry
/// run before offering it.
fn detect(
    game: &mut GameState,
    trigger_queue: &TriggerQueue,
    state: Option<&PriorityLoopState>,
    proposer: PlayerId,
    progress: &GameProgress,
) {
    let outcome = try_detect(game, trigger_queue, state, proposer, progress);
    match outcome {
        Ok(offer) => {
            game.loop_shortcut.note = None;
            game.loop_shortcut.offer = Some(offer);
        }
        Err(reason) => game.loop_shortcut.note = Some(reason),
    }
}

/// The loop ending at this prompt, or why none is offered.
fn try_detect(
    game: &GameState,
    trigger_queue: &TriggerQueue,
    state: Option<&PriorityLoopState>,
    proposer: PlayerId,
    progress: &GameProgress,
) -> Result<LoopShortcutOffer, String> {
    if game.turn.priority_player != Some(proposer) {
        return Err("the player does not have priority".to_string());
    }
    let tracker = &game.loop_shortcut;
    let anchors = tracker
        .steps
        .iter()
        .filter(|step| step.anchor.is_some() && step.actor == Some(proposer))
        .count();
    if anchors == 0 {
        return Err(format!(
            "none of the {} recorded steps this step of the turn began an action of this player",
            tracker.steps.len()
        ));
    }
    let current = LoopStateSummary::of(game);
    let mut found = None;
    for (index, step) in tracker.steps.iter().enumerate().rev() {
        let Some(anchor) = step.anchor.as_ref().filter(|_| step.actor == Some(proposer)) else {
            continue;
        };
        if let Some(delta) = anchor.delta_to(&current) {
            // An unchanged state is a no-op, not a loop worth repeating; a
            // longer lap ending here would only repeat it.
            if delta.is_empty() {
                return Err("the game returned to exactly the same state".to_string());
            }
            found = Some((index, delta));
            break;
        }
    }
    let Some((start, delta)) = found else {
        let latest = tracker
            .steps
            .iter()
            .rev()
            .find_map(|step| step.anchor.as_ref().filter(|_| step.actor == Some(proposer)))
            .map(|anchor| anchor.fixed_differences(&current).join("; "))
            .unwrap_or_default();
        return Err(format!(
            "the game is not in a state it was in before any of this player's {anchors} action(s), apart from countable resources (since the latest one, changed: {latest})"
        ));
    };
    let lap: Arc<[Step]> = tracker.steps[start..].to_vec().into();
    let Some(mut responders) = lap_shape(&lap, proposer) else {
        return Err(
            "the repeated actions include a choice by another player or a step that cannot be replayed"
                .to_string(),
        );
    };
    let order = game.turn_store.turn_order.clone();
    responders.sort_by_key(|player| order.iter().position(|seat| seat == player));

    // Proof: one more lap on a copy must be legal and change exactly the same.
    let _nested = DepthGuard::enter();
    let mut trial = game.clone();
    let mut trial_queue = trigger_queue.clone();
    let mut trial_state = state
        .cloned()
        .unwrap_or_else(|| PriorityLoopState::new(game.players_in_game()));
    match run_lap(
        &mut trial,
        &mut trial_queue,
        &mut trial_state,
        &lap,
        proposer,
        progress.clone(),
    ) {
        Ok(LapEnd::Completed(_)) => {
            if current.delta_to(&LoopStateSummary::of(&trial)).as_ref() != Some(&delta) {
                return Err("repeating the actions once more changed the game differently".to_string());
            }
        }
        // A lap that ends the game is repeatable exactly once more.
        Ok(LapEnd::GameOver(_)) => {}
        Err(reason) => return Err(format!("the actions could not be repeated: {reason}")),
    }
    Ok(LoopShortcutOffer {
        proposer,
        per_lap: delta_entries(&delta),
        responders,
        steps: lap.len(),
        start_serial: lap[0].serial,
        lap,
        delta,
    })
}

/// Look for a loop at the current priority prompt.
///
/// The priority-loop functions do this themselves whenever they return a
/// priority prompt. A driver that works out the prompt on its own after a
/// call returned [`GameProgress::Continue`] (the browser session does after a
/// mana ability) calls this instead, once no action is pending. It only ever
/// changes [`GameState::loop_shortcut_offer`], and does nothing when
/// detection is off or already ran for the current step.
pub fn detect_loop_shortcut(
    game: &mut GameState,
    trigger_queue: &TriggerQueue,
    state: &PriorityLoopState,
) {
    let tracker = &game.loop_shortcut;
    if !tracker.enabled
        || tracker.offer.is_some()
        || tracker.checked_serial == Some(tracker.serial)
        || tracker.epoch.as_ref() != Some(&epoch_of(game))
        || DEPTH.with(Cell::get) != 0
        || state.has_pending_action()
    {
        return;
    }
    let Some(proposer) = game.turn.priority_player else {
        return;
    };
    game.loop_shortcut.checked_serial = Some(game.loop_shortcut.serial);
    let _nested = DepthGuard::enter();
    // The lap replay settles an unknown starting point by advancing first.
    detect(game, trigger_queue, Some(state), proposer, &GameProgress::Continue);
}

/// Take the offered shortcut: ask for the number of laps, let the other
/// players who would receive priority shorten it, then perform the laps.
pub(super) fn execute(
    game: &mut GameState,
    trigger_queue: &mut TriggerQueue,
    state: &mut PriorityLoopState,
    decision_maker: &mut dyn DecisionMaker,
) -> Result<GameProgress, GameLoopError> {
    discard_active_frame();
    let Some(proposer) = game.turn.priority_player else {
        return Err(GameLoopError::InvalidState(
            "no player has priority to shortcut a loop".to_string(),
        ));
    };
    if !game.loop_shortcut_offered_to(proposer) || state.has_pending_action() {
        return Err(GameLoopError::InvalidState(
            "no loop shortcut is offered at this point".to_string(),
        ));
    }
    let offer = game
        .loop_shortcut
        .offer
        .clone()
        .expect("offer presence was just checked");
    let per_lap = offer.describe_per_lap(game);

    let number = context::NumberContext::new(
        proposer,
        None,
        0,
        MAX_LOOP_SHORTCUT_LAPS,
        "Repeat this loop how many times? (0 to decline)",
    )
    .with_context_text(format!("Each repetition: {per_lap}"));
    let requested = decision_maker.decide_number(game, &number);
    if decision_maker.awaiting_choice() {
        return Ok(GameProgress::Continue);
    }
    let requested = requested.min(MAX_LOOP_SHORTCUT_LAPS);
    let mut accepted = requested;
    let mut stop = LoopShortcutStop::Completed;
    if requested == 0 {
        stop = LoopShortcutStop::Declined;
    }

    // CR 732.2b: in turn order, each other player may accept or name an
    // earlier point at which they will act instead.
    let proposer_name = game
        .player(proposer)
        .map(|player| player.name.clone())
        .unwrap_or_default();
    for responder in offer.responders.iter().copied() {
        if accepted == 0 {
            break;
        }
        let question = context::BooleanContext::new(
            responder,
            None,
            format!(
                "{proposer_name} proposes repeating a loop {accepted} times. Respond instead of letting it finish?"
            ),
        )
        .with_context_text(format!("Each repetition: {per_lap}"));
        let interrupts = decision_maker.decide_boolean(game, &question);
        if decision_maker.awaiting_choice() {
            return Ok(GameProgress::Continue);
        }
        if !interrupts {
            continue;
        }
        let limit = context::NumberContext::new(
            responder,
            None,
            0,
            accepted - 1,
            "Let the loop repeat how many times before you respond?",
        );
        let allowed = decision_maker.decide_number(game, &limit);
        if decision_maker.awaiting_choice() {
            return Ok(GameProgress::Continue);
        }
        accepted = allowed.min(accepted - 1);
        stop = LoopShortcutStop::Shortened { by: responder };
    }

    // From here on the game changes; nothing below asks the outer decision maker.
    game.loop_shortcut.offer = None;
    let _nested = DepthGuard::enter();
    let mut progress = super::advance_priority_with_dm(
        game,
        trigger_queue,
        &mut crate::decision::AutoPassDecisionMaker,
    )?;
    let mut executed = 0;
    let mut before = LoopStateSummary::of(game);
    let mut game_over = None;
    while executed < accepted {
        let checkpoint = (game.clone(), trigger_queue.clone(), state.clone());
        let outcome = run_lap(game, trigger_queue, state, &offer.lap, proposer, progress.clone());
        let failure = match outcome {
            Ok(LapEnd::GameOver(end)) => {
                executed += 1;
                game_over = Some(end);
                stop = LoopShortcutStop::GameOver;
                break;
            }
            Ok(LapEnd::Completed(next)) => {
                let after = LoopStateSummary::of(game);
                if before.delta_to(&after).as_ref() == Some(&offer.delta) {
                    executed += 1;
                    before = after;
                    progress = next;
                    continue;
                }
                "a repetition changed the game differently".to_string()
            }
            Err(reason) => reason,
        };
        *game = checkpoint.0;
        *trigger_queue = checkpoint.1;
        *state = checkpoint.2;
        stop = LoopShortcutStop::Diverged { reason: failure };
        break;
    }
    drop(_nested);

    game.loop_shortcut.steps.clear();
    game.loop_shortcut.serial += 1;
    game.loop_shortcut.epoch = Some(epoch_of(game));
    game.loop_shortcut.report = Some(LoopShortcutReport {
        proposer,
        requested,
        accepted,
        executed,
        stop: stop.clone(),
    });
    if let Some(end) = game_over {
        return Ok(end);
    }
    // The loop is still available after a complete run. A responder who
    // shortened it wants a real priority window, so the next lap is manual.
    if matches!(stop, LoopShortcutStop::Completed) && executed > 0 {
        game.loop_shortcut.offer = Some(offer);
    }
    let _nested = DepthGuard::enter();
    super::advance_priority_with_dm(
        game,
        trigger_queue,
        &mut crate::decision::AutoPassDecisionMaker,
    )
}
