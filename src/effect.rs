//! Effect system for spells and abilities.
//!
//! Effects are one-shot game actions that occur when a spell or ability resolves.
//! This module defines the vocabulary of effects that can be composed into abilities.
//!
//! ## Effect Results and Outcomes
//!
//! Effects return an `EffectOutcome` when executed, which contains:
//! - An `EffectResult` indicating what happened (success/failure variants)
//! - A `Vec<TriggerEvent>` of events that occurred during execution
//!
//! `EffectResult` variants:
//! - Success variants: `Count(n)`, `Resolved`, `ManaAdded`, `Objects`
//! - Failure variants: `Declined`, `TargetInvalid`, `Prevented`, `Protected`, `Impossible`
//!
//! Effects can be labeled with `EffectId` using `Effect::with_id`, and later effects
//! can reference those results using `Effect::if_` with an `EffectPredicate`.

use crate::effects::EffectExecutor;
use crate::ids::ObjectId;
use crate::mana::ManaSymbol;
use crate::object::CounterType;
use crate::tag::TagKey;
use crate::target::{ChooseSpec, ObjectFilter, ObjectRef, PlayerFilter};
use crate::zone::Zone;

// ============================================================================
// Effect Identity and Results
// ============================================================================

/// Identifier for an effect within an effect sequence.
///
/// Used to reference effects for conditional logic ("if you do" patterns).
/// Effects are labeled with `Effect::WithId` and referenced by `Effect::If`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EffectId(pub u32);

impl EffectId {
    /// Special ID used by ForEachControllerOfTaggedEffect to store the count
    /// of tagged objects for the current controller during iteration.
    pub const TAGGED_COUNT: Self = Self(u32::MAX);
}

/// Specifies how many objects/players to choose.
///
/// Used for effects like "Exile any number of target spells" (Mindbreak Trap)
/// or "Choose up to two target creatures".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChoiceCount {
    /// Minimum number to choose (0 for "any number" or "up to").
    pub min: usize,
    /// Maximum number to choose. None means unlimited ("any number").
    pub max: Option<usize>,
    /// Whether this count came from a dynamic `X target ...` clause.
    pub dynamic_x: bool,
}

impl Default for ChoiceCount {
    fn default() -> Self {
        Self::exactly(1)
    }
}

impl ChoiceCount {
    /// Exactly N (the default for most effects).
    pub const fn exactly(n: usize) -> Self {
        Self {
            min: n,
            max: Some(n),
            dynamic_x: false,
        }
    }

    /// Any number (0 or more, unlimited).
    pub const fn any_number() -> Self {
        Self {
            min: 0,
            max: None,
            dynamic_x: false,
        }
    }

    /// At least N (N or more, unlimited).
    pub const fn at_least(n: usize) -> Self {
        Self {
            min: n,
            max: None,
            dynamic_x: false,
        }
    }

    /// Up to N (0 to N).
    pub const fn up_to(n: usize) -> Self {
        Self {
            min: 0,
            max: Some(n),
            dynamic_x: false,
        }
    }

    /// Dynamic X-target count (rendered as `X target ...`).
    pub const fn dynamic_x() -> Self {
        Self {
            min: 0,
            max: None,
            dynamic_x: true,
        }
    }

    /// Returns true if this is "any number" (min 0, no max).
    pub fn is_any_number(&self) -> bool {
        self.min == 0 && self.max.is_none() && !self.dynamic_x
    }

    /// Returns true if this is exactly 1.
    pub fn is_single(&self) -> bool {
        self.min == 1 && self.max == Some(1)
    }

    pub const fn is_dynamic_x(&self) -> bool {
        self.dynamic_x
    }
}

impl From<usize> for ChoiceCount {
    fn from(value: usize) -> Self {
        ChoiceCount::exactly(value)
    }
}

impl From<i32> for ChoiceCount {
    fn from(value: i32) -> Self {
        if value <= 0 {
            ChoiceCount::exactly(0)
        } else {
            ChoiceCount::exactly(value as usize)
        }
    }
}

/// The result of executing an effect.
///
/// Each effect produces a result that indicates what happened. Results are
/// categorized as either success (something happened) or failure (nothing happened).
///
/// For "if you do" logic, use `something_happened()` which returns true for
/// success variants where the count > 0 or a single-target effect resolved.
#[derive(Debug, Clone, PartialEq)]
pub enum EffectResult {
    // === Success variants ===
    /// Effect produced a numeric count.
    ///
    /// Used for: cards drawn, damage dealt, life gained/lost, tokens created,
    /// permanents sacrificed, counters placed, etc.
    ///
    /// Note: `Count(0)` is technically a success (effect executed) but
    /// `something_happened()` returns false for it.
    Count(i32),

    /// Single-target effect resolved successfully.
    ///
    /// Used for: destroy, exile, counter, tap/untap, return to hand, etc.
    Resolved,

    /// Mana was added to a player's mana pool.
    ManaAdded(Vec<ManaSymbol>),

    /// Specific objects were affected.
    ///
    /// Used when effects need to track which objects were affected for
    /// later reference (e.g., "exile that token at end of combat").
    Objects(Vec<ObjectId>),

    /// Monstrosity was applied to a creature.
    ///
    /// Used to trigger "when this becomes monstrous" abilities.
    MonstrosityApplied { creature: ObjectId, n: u32 },

    // === Failure variants ===
    /// Player declined a "you may" choice.
    Declined,

    /// Target was invalid (doesn't exist, wrong zone, wrong characteristics).
    TargetInvalid,

    /// Effect was actively prevented (e.g., damage prevention shield).
    Prevented,

    /// Target was protected (indestructible, hexproof, "can't be countered", etc.).
    Protected,

    /// Effect was impossible to perform (empty library, no sacrifice targets, etc.).
    Impossible,

    /// Effect was replaced by another effect.
    ///
    /// Note: Usually still counts as "success" since something happened,
    /// just not what was originally intended.
    Replaced,
}

impl EffectResult {
    /// Create a success result with a count.
    pub fn count(n: i32) -> Self {
        Self::Count(n)
    }

    /// Create a success result for a resolved single-target effect.
    pub fn resolved() -> Self {
        Self::Resolved
    }

    /// Create a failure result for a declined choice.
    pub fn declined() -> Self {
        Self::Declined
    }

    /// Is this a success result (not a failure variant)?
    pub fn is_success(&self) -> bool {
        matches!(
            self,
            Self::Count(_)
                | Self::Resolved
                | Self::ManaAdded(_)
                | Self::Objects(_)
                | Self::MonstrosityApplied { .. }
                | Self::Replaced
        )
    }

    /// Is this a failure result?
    pub fn is_failure(&self) -> bool {
        !self.is_success()
    }

    /// Did something actually happen?
    ///
    /// This is the typical "if you do" check:
    /// - `Count(n)` where n > 0: true
    /// - `Resolved`: true
    /// - `ManaAdded(_)`: true
    /// - `Objects(_)` where non-empty: true
    /// - `MonstrosityApplied`: true
    /// - `Replaced`: true (something happened, just different)
    /// - Everything else: false
    pub fn something_happened(&self) -> bool {
        match self {
            Self::Count(n) => *n > 0,
            Self::Resolved => true,
            Self::ManaAdded(mana) => !mana.is_empty(),
            Self::Objects(objs) => !objs.is_empty(),
            Self::MonstrosityApplied { .. } => true,
            Self::Replaced => true,
            _ => false,
        }
    }

    /// Get the count value, if this is a Count result.
    pub fn as_count(&self) -> Option<i32> {
        match self {
            Self::Count(n) => Some(*n),
            _ => None,
        }
    }

    /// Get the count value, defaulting to 0 for non-Count results.
    pub fn count_or_zero(&self) -> i32 {
        self.as_count().unwrap_or(0)
    }
}

impl Default for EffectResult {
    fn default() -> Self {
        Self::Resolved
    }
}

// ============================================================================
// Effect Outcome (result + events)
// ============================================================================

/// The outcome of executing an effect, including both the result and any events.
///
/// This type combines an `EffectResult` with a list of `TriggerEvent`s that occurred
/// during execution. This enables centralized trigger checking in the game loop -
/// instead of effects directly firing triggers, they return events that the game
/// loop can process.
///
/// # Example
///
/// ```ignore
/// // Simple effect with no events
/// Ok(EffectOutcome::resolved())
///
/// // Effect that generated an event
/// Ok(EffectOutcome::count(3).with_event(TriggerEvent::new(DamageEvent { ... })))
///
/// // Aggregating multiple outcomes from child effects
/// let outcomes: Vec<EffectOutcome> = child_effects.iter()
///     .map(|e| execute_effect(game, e, ctx))
///     .collect::<Result<_, _>>()?;
/// Ok(EffectOutcome::aggregate(outcomes))
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct EffectOutcome {
    /// The result of the effect execution.
    pub result: EffectResult,
    /// Events that occurred during execution (for trigger checking).
    pub events: Vec<crate::triggers::TriggerEvent>,
}

impl EffectOutcome {
    /// Create an outcome from just a result (no events).
    pub fn from_result(result: EffectResult) -> Self {
        Self {
            result,
            events: Vec::new(),
        }
    }

    /// Create an outcome with both result and events.
    pub fn new(result: EffectResult, events: Vec<crate::triggers::TriggerEvent>) -> Self {
        Self { result, events }
    }

    /// Create a resolved outcome (no events).
    pub fn resolved() -> Self {
        Self::from_result(EffectResult::Resolved)
    }

    /// Create a count outcome (no events).
    pub fn count(n: i32) -> Self {
        Self::from_result(EffectResult::Count(n))
    }

    /// Add a single event to this outcome.
    pub fn with_event(mut self, event: crate::triggers::TriggerEvent) -> Self {
        self.events.push(event);
        self
    }

    /// Add multiple events to this outcome.
    pub fn with_events(
        mut self,
        events: impl IntoIterator<Item = crate::triggers::TriggerEvent>,
    ) -> Self {
        self.events.extend(events);
        self
    }

    /// Aggregate multiple outcomes into a single outcome.
    ///
    /// The result is determined by:
    /// - If any outcome has a Count result, sum them
    /// - Otherwise use the last non-default result
    /// - Events are concatenated from all outcomes
    pub fn aggregate(outcomes: impl IntoIterator<Item = EffectOutcome>) -> Self {
        let mut total_count: i32 = 0;
        let mut has_count = false;
        let mut last_result = EffectResult::Resolved;
        let mut all_events = Vec::new();

        for outcome in outcomes {
            all_events.extend(outcome.events);
            match outcome.result {
                EffectResult::Count(n) => {
                    total_count += n;
                    has_count = true;
                }
                other => {
                    last_result = other;
                }
            }
        }

        let result = if has_count {
            EffectResult::Count(total_count)
        } else {
            last_result
        };

        Self {
            result,
            events: all_events,
        }
    }

    /// Get the result of this outcome.
    pub fn result(&self) -> &EffectResult {
        &self.result
    }

    /// Check if something happened (delegates to EffectResult::something_happened).
    pub fn something_happened(&self) -> bool {
        self.result.something_happened()
    }

    /// Get the count value, or zero if not a Count result.
    pub fn count_or_zero(&self) -> i32 {
        self.result.count_or_zero()
    }

    /// Get the count value if this is a Count result.
    pub fn as_count(&self) -> Option<i32> {
        self.result.as_count()
    }
}

impl From<EffectResult> for EffectOutcome {
    fn from(result: EffectResult) -> Self {
        Self::from_result(result)
    }
}

impl Default for EffectOutcome {
    fn default() -> Self {
        Self::resolved()
    }
}

// ============================================================================
// Effect Predicates (for conditional effects)
// ============================================================================

/// Predicate to evaluate an effect's result.
///
/// Used with `Effect::If` to conditionally execute effects based on
/// a prior effect's result.
#[derive(Debug, Clone, PartialEq)]
pub enum EffectPredicate {
    /// Effect succeeded (is_success() returns true).
    Succeeded,

    /// Effect failed (is_failure() returns true).
    Failed,

    /// Something actually happened (something_happened() returns true).
    /// This is the typical "if you do" meaning.
    Happened,

    /// Nothing happened (something_happened() returns false).
    /// This is the typical "if you don't" meaning.
    DidNotHappen,

    /// Something happened and it was not replaced.
    HappenedNotReplaced,

    /// Compare the count value.
    /// Only meaningful for `EffectResult::Count` results.
    Value(Comparison),

    /// Player chose to do it (result is not Declined).
    Chosen,

    /// Was the result Declined?
    WasDeclined,
}

impl EffectPredicate {
    /// Evaluate this predicate against an effect result.
    pub fn evaluate(&self, result: &EffectResult) -> bool {
        match self {
            Self::Succeeded => result.is_success(),
            Self::Failed => result.is_failure(),
            Self::Happened => result.something_happened(),
            Self::DidNotHappen => !result.something_happened(),
            Self::HappenedNotReplaced => {
                result.something_happened() && !matches!(result, EffectResult::Replaced)
            }
            Self::Value(cmp) => {
                if let Some(n) = result.as_count() {
                    cmp.evaluate(n)
                } else {
                    false
                }
            }
            Self::Chosen => !matches!(result, EffectResult::Declined),
            Self::WasDeclined => matches!(result, EffectResult::Declined),
        }
    }
}

/// Comparison operations for numeric values.
#[derive(Debug, Clone, PartialEq)]
pub enum Comparison {
    GreaterThan(i32),
    GreaterThanOrEqual(i32),
    Equal(i32),
    LessThan(i32),
    LessThanOrEqual(i32),
    NotEqual(i32),
}

impl Comparison {
    /// Evaluate this comparison against a value.
    pub fn evaluate(&self, value: i32) -> bool {
        match self {
            Self::GreaterThan(n) => value > *n,
            Self::GreaterThanOrEqual(n) => value >= *n,
            Self::Equal(n) => value == *n,
            Self::LessThan(n) => value < *n,
            Self::LessThanOrEqual(n) => value <= *n,
            Self::NotEqual(n) => value != *n,
        }
    }
}

// ============================================================================
// Values
// ============================================================================

/// A value that can be fixed, variable (X), or computed.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// A fixed numeric value
    Fixed(i32),

    /// Sum of two values.
    Add(Box<Value>, Box<Value>),

    /// The X value from the cost
    X,

    /// X multiplied by a factor
    XTimes(i32),

    /// The number of objects matching a filter
    Count(ObjectFilter),

    /// A scaled count of objects matching a filter (factor * count(filter)).
    CountScaled(ObjectFilter, i32),

    /// Number of creatures that have died this turn.
    CreaturesDiedThisTurn,

    /// The number of players matching a filter
    CountPlayers(PlayerFilter),

    /// The power of the source creature
    SourcePower,

    /// The toughness of the source creature
    SourceToughness,

    /// The power of a specific object
    PowerOf(Box<ChooseSpec>),

    /// The toughness of a specific object
    ToughnessOf(Box<ChooseSpec>),

    /// The mana value of a specific object.
    ManaValueOf(Box<ChooseSpec>),

    /// Life total of a player
    LifeTotal(PlayerFilter),

    /// Number of cards in a player's hand
    CardsInHand(PlayerFilter),

    /// Number of cards in a player's graveyard
    CardsInGraveyard(PlayerFilter),

    /// Number of spells cast this turn by players matching a filter.
    SpellsCastThisTurn(PlayerFilter),

    /// Number of spells cast before this spell this turn by players matching a filter.
    ///
    /// Primarily used for storm on "you cast this spell" triggers.
    SpellsCastBeforeThisTurn(PlayerFilter),

    /// Number of distinct card types among cards in a player's graveyard
    CardTypesInGraveyard(PlayerFilter),

    /// Devotion to a color for players matching the filter.
    ///
    /// Devotion counts the number of colored mana symbols of the chosen color
    /// in mana costs of permanents that player controls.
    Devotion {
        player: PlayerFilter,
        color: crate::color::Color,
    },

    /// Number of distinct colors of mana spent to cast this spell.
    ///
    /// Colorless mana does not count toward this value.
    ColorsOfManaSpentToCastThisSpell,

    /// The value from a prior effect's result.
    ///
    /// Used for effects like "Draw cards equal to the damage dealt."
    /// References an effect that was labeled with `Effect::WithId`.
    EffectValue(EffectId),

    /// The value from a prior effect's result plus or minus a fixed offset.
    ///
    /// Useful for patterns like "draw that many cards minus one".
    EffectValueOffset(EffectId, i32),

    /// Value pulled from the triggering event payload.
    EventValue(EventValueSpec),

    /// Trigger/event-derived value plus or minus a fixed offset.
    EventValueOffset(EventValueSpec, i32),

    /// 1 if the spell was kicked (any "Kicker" or "Multikicker" cost was paid), 0 otherwise.
    WasKicked,

    /// 1 if buyback was paid, 0 otherwise.
    WasBoughtBack,

    /// 1 if entwine was paid, 0 otherwise.
    WasEntwined,

    /// 1 if the optional cost at the given index was paid, 0 otherwise.
    WasPaid(usize),

    /// 1 if the optional cost with the given label was paid, 0 otherwise.
    /// More readable than index-based: `WasPaidLabel("Kicker")` vs `WasPaid(0)`.
    WasPaidLabel(&'static str),

    /// The number of times the optional cost at the given index was paid.
    /// Useful for multikicker: "create a token for each time it was kicked."
    TimesPaid(usize),

    /// The number of times the optional cost with the given label was paid.
    TimesPaidLabel(&'static str),

    /// The number of times the kicker was paid.
    /// Convenience for multikicker cards.
    KickCount,

    /// The number of counters of a specific type on the source permanent.
    CountersOnSource(CounterType),

    /// The number of counters on object(s) resolved from a choose spec.
    ///
    /// When `counter_type` is `Some`, counts only that type.
    /// When `counter_type` is `None`, counts all counters.
    CountersOn(Box<ChooseSpec>, Option<CounterType>),

    /// The count of tagged objects for the current controller in a ForEachControllerOfTagged loop.
    ///
    /// This is set automatically by `ForEachControllerOfTaggedEffect` during iteration,
    /// providing the number of objects that the current iterated player controlled.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // "Their controllers each create a 3/3 for each creature they controlled"
    /// Effect::for_each_controller_of_tagged("destroyed", vec![
    ///     Effect::create_tokens_player(
    ///         elephant_token(),
    ///         Value::TaggedCount,  // Gets the count for this controller
    ///         PlayerFilter::IteratedPlayer,
    ///     ),
    /// ])
    /// ```
    TaggedCount,
}

/// Event payload fields that can be referenced by [`Value::EventValue`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventValueSpec {
    /// Generic "that much" amount from triggering life gain/loss or damage events.
    Amount,
    /// The amount from a triggering life gain/loss event.
    LifeAmount,
    /// Number of blockers beyond the first from a creature-becomes-blocked event,
    /// scaled by `multiplier`.
    BlockersBeyondFirst { multiplier: i32 },
}

/// A rule restriction ("can't" effect) specification.
#[derive(Debug, Clone, PartialEq)]
pub enum Restriction {
    GainLife(PlayerFilter),
    SearchLibraries(PlayerFilter),
    CastSpells(PlayerFilter),
    DrawCards(PlayerFilter),
    DrawExtraCards(PlayerFilter),
    ChangeLifeTotal(PlayerFilter),
    LoseGame(PlayerFilter),
    WinGame(PlayerFilter),
    PreventDamage,
    Attack(ObjectFilter),
    Block(ObjectFilter),
    Untap(ObjectFilter),
    BeBlocked(ObjectFilter),
    BeDestroyed(ObjectFilter),
    BeSacrificed(ObjectFilter),
    HaveCountersPlaced(ObjectFilter),
    BeTargeted(ObjectFilter),
    BeCountered(ObjectFilter),
    Transform(ObjectFilter),
}

impl Restriction {
    pub fn gain_life(filter: PlayerFilter) -> Self {
        Self::GainLife(filter)
    }

    pub fn search_libraries(filter: PlayerFilter) -> Self {
        Self::SearchLibraries(filter)
    }

    pub fn cast_spells(filter: PlayerFilter) -> Self {
        Self::CastSpells(filter)
    }

    pub fn draw_cards(filter: PlayerFilter) -> Self {
        Self::DrawCards(filter)
    }

    pub fn draw_extra_cards(filter: PlayerFilter) -> Self {
        Self::DrawExtraCards(filter)
    }

    pub fn change_life_total(filter: PlayerFilter) -> Self {
        Self::ChangeLifeTotal(filter)
    }

    pub fn lose_game(filter: PlayerFilter) -> Self {
        Self::LoseGame(filter)
    }

    pub fn win_game(filter: PlayerFilter) -> Self {
        Self::WinGame(filter)
    }

    pub fn prevent_damage() -> Self {
        Self::PreventDamage
    }

    pub fn attack(filter: ObjectFilter) -> Self {
        Self::Attack(filter)
    }

    pub fn block(filter: ObjectFilter) -> Self {
        Self::Block(filter)
    }

    pub fn untap(filter: ObjectFilter) -> Self {
        Self::Untap(filter)
    }

    pub fn be_blocked(filter: ObjectFilter) -> Self {
        Self::BeBlocked(filter)
    }

    pub fn be_destroyed(filter: ObjectFilter) -> Self {
        Self::BeDestroyed(filter)
    }

    pub fn be_sacrificed(filter: ObjectFilter) -> Self {
        Self::BeSacrificed(filter)
    }

    pub fn have_counters_placed(filter: ObjectFilter) -> Self {
        Self::HaveCountersPlaced(filter)
    }

    pub fn be_targeted(filter: ObjectFilter) -> Self {
        Self::BeTargeted(filter)
    }

    pub fn be_countered(filter: ObjectFilter) -> Self {
        Self::BeCountered(filter)
    }

    pub fn transform(filter: ObjectFilter) -> Self {
        Self::Transform(filter)
    }

    pub fn apply(
        &self,
        game: &crate::game_state::GameState,
        tracker: &mut crate::game_state::CantEffectTracker,
        controller: crate::ids::PlayerId,
        source: Option<crate::ids::ObjectId>,
    ) {
        use crate::game_loop::player_matches_filter_with_combat;

        let combat = game.combat.as_ref();
        let ctx = game.filter_context_for_combat(controller, source, None, None);

        match self {
            Restriction::GainLife(filter) => {
                for player in &game.players {
                    if player.is_in_game()
                        && player_matches_filter_with_combat(
                            player.id, filter, game, controller, combat,
                        )
                    {
                        tracker.cant_gain_life.insert(player.id);
                    }
                }
            }
            Restriction::SearchLibraries(filter) => {
                for player in &game.players {
                    if player.is_in_game()
                        && player_matches_filter_with_combat(
                            player.id, filter, game, controller, combat,
                        )
                    {
                        tracker.cant_search.insert(player.id);
                    }
                }
            }
            Restriction::CastSpells(filter) => {
                for player in &game.players {
                    if player.is_in_game()
                        && player_matches_filter_with_combat(
                            player.id, filter, game, controller, combat,
                        )
                    {
                        tracker.cant_cast_spells.insert(player.id);
                    }
                }
            }
            Restriction::DrawCards(filter) => {
                for player in &game.players {
                    if player.is_in_game()
                        && player_matches_filter_with_combat(
                            player.id, filter, game, controller, combat,
                        )
                    {
                        tracker.cant_draw.insert(player.id);
                    }
                }
            }
            Restriction::DrawExtraCards(filter) => {
                for player in &game.players {
                    if player.is_in_game()
                        && player_matches_filter_with_combat(
                            player.id, filter, game, controller, combat,
                        )
                    {
                        tracker.cant_draw_extra_cards.insert(player.id);
                    }
                }
            }
            Restriction::ChangeLifeTotal(filter) => {
                for player in &game.players {
                    if player.is_in_game()
                        && player_matches_filter_with_combat(
                            player.id, filter, game, controller, combat,
                        )
                    {
                        tracker.life_total_cant_change.insert(player.id);
                    }
                }
            }
            Restriction::LoseGame(filter) => {
                for player in &game.players {
                    if player.is_in_game()
                        && player_matches_filter_with_combat(
                            player.id, filter, game, controller, combat,
                        )
                    {
                        tracker.cant_lose_game.insert(player.id);
                    }
                }
            }
            Restriction::WinGame(filter) => {
                for player in &game.players {
                    if player.is_in_game()
                        && player_matches_filter_with_combat(
                            player.id, filter, game, controller, combat,
                        )
                    {
                        tracker.cant_win_game.insert(player.id);
                    }
                }
            }
            Restriction::PreventDamage => {
                tracker.damage_cant_be_prevented = true;
            }
            Restriction::Attack(filter) => {
                for &obj_id in &game.battlefield {
                    if let Some(obj) = game.object(obj_id)
                        && filter.matches(obj, &ctx, game)
                    {
                        tracker.cant_attack.insert(obj_id);
                    }
                }
            }
            Restriction::Block(filter) => {
                for &obj_id in &game.battlefield {
                    if let Some(obj) = game.object(obj_id)
                        && filter.matches(obj, &ctx, game)
                    {
                        tracker.cant_block.insert(obj_id);
                    }
                }
            }
            Restriction::Untap(filter) => {
                for &obj_id in &game.battlefield {
                    if let Some(obj) = game.object(obj_id)
                        && filter.matches(obj, &ctx, game)
                    {
                        tracker.cant_untap.insert(obj_id);
                    }
                }
            }
            Restriction::BeBlocked(filter) => {
                for &obj_id in &game.battlefield {
                    if let Some(obj) = game.object(obj_id)
                        && filter.matches(obj, &ctx, game)
                    {
                        tracker.cant_be_blocked.insert(obj_id);
                    }
                }
            }
            Restriction::BeDestroyed(filter) => {
                for &obj_id in &game.battlefield {
                    if let Some(obj) = game.object(obj_id)
                        && filter.matches(obj, &ctx, game)
                    {
                        tracker.cant_be_destroyed.insert(obj_id);
                    }
                }
            }
            Restriction::BeSacrificed(filter) => {
                for &obj_id in &game.battlefield {
                    if let Some(obj) = game.object(obj_id)
                        && filter.matches(obj, &ctx, game)
                    {
                        tracker.cant_be_sacrificed.insert(obj_id);
                    }
                }
            }
            Restriction::HaveCountersPlaced(filter) => {
                for &obj_id in &game.battlefield {
                    if let Some(obj) = game.object(obj_id)
                        && filter.matches(obj, &ctx, game)
                    {
                        tracker.cant_have_counters_placed.insert(obj_id);
                    }
                }
            }
            Restriction::BeTargeted(filter) => {
                for &obj_id in &game.battlefield {
                    if let Some(obj) = game.object(obj_id)
                        && filter.matches(obj, &ctx, game)
                    {
                        tracker.cant_be_targeted.insert(obj_id);
                    }
                }
            }
            Restriction::BeCountered(filter) => {
                for entry in &game.stack {
                    let obj_id = entry.object_id;
                    if let Some(obj) = game.object(obj_id)
                        && filter.matches(obj, &ctx, game)
                    {
                        tracker.cant_be_countered.insert(obj_id);
                    }
                }
            }
            Restriction::Transform(filter) => {
                for &obj_id in &game.battlefield {
                    if let Some(obj) = game.object(obj_id)
                        && filter.matches(obj, &ctx, game)
                    {
                        tracker.cant_transform.insert(obj_id);
                    }
                }
            }
        }
    }
}

impl Value {
    /// Create a fixed value.
    pub fn fixed(n: i32) -> Self {
        Self::Fixed(n)
    }

    /// Create a count of creatures you control.
    pub fn creatures_you_control() -> Self {
        Self::Count(ObjectFilter::creature().you_control())
    }
}

impl From<i32> for Value {
    fn from(n: i32) -> Self {
        Value::Fixed(n)
    }
}

impl From<u32> for Value {
    fn from(n: u32) -> Self {
        Value::Fixed(n as i32)
    }
}

/// A one-shot effect that occurs when a spell or ability resolves.
///
/// Effects are implemented via the `EffectExecutor` trait, allowing for modular
/// effect implementations with co-located tests. This struct wraps a trait object
/// that can execute any effect type.
///
/// Use the helper constructors (e.g., `Effect::draw()`, `Effect::damage()`) to
/// create effects rather than constructing directly.
#[derive(Debug)]
pub struct Effect(pub Box<dyn EffectExecutor>);

impl Clone for Effect {
    fn clone(&self) -> Self {
        Effect(self.0.clone_box())
    }
}

impl PartialEq for Effect {
    fn eq(&self, _other: &Self) -> bool {
        // Two effects are never considered equal via PartialEq.
        // This is a limitation, but acceptable since Effect equality
        // is primarily used for testing where effects can be
        // compared via their behavior or debug output instead.
        false
    }
}

impl Effect {
    /// Create a new effect from an EffectExecutor implementation.
    pub fn new<E: EffectExecutor + 'static>(executor: E) -> Self {
        Effect(Box::new(executor))
    }

    /// Attempt to downcast this effect to a concrete executor type.
    pub fn downcast_ref<T: 'static>(&self) -> Option<&T> {
        (self.0.as_ref() as &dyn std::any::Any).downcast_ref::<T>()
    }

    /// Tag this effect's target for reference by subsequent effects.
    ///
    /// This wraps the effect in a `TaggedEffect` that captures a snapshot of
    /// the first object target before executing the inner effect. Subsequent
    /// effects can then reference this object using:
    /// - `PlayerFilter::ControllerOf(ObjectRef::tagged("tag_name"))`
    /// - `PlayerFilter::OwnerOf(ObjectRef::tagged("tag_name"))`
    ///
    /// # Example
    ///
    /// ```ignore
    /// // "Destroy target permanent. Its controller creates a 3/3 token."
    /// vec![
    ///     Effect::destroy(ChooseSpec::permanent()).tag("destroyed"),
    ///     Effect::create_tokens_player(
    ///         elephant_token(),
    ///         1,
    ///         PlayerFilter::ControllerOf(ObjectRef::tagged("destroyed")),
    ///     ),
    /// ]
    /// ```
    pub fn tag(self, tag: impl Into<TagKey>) -> Self {
        use crate::effects::TaggedEffect;
        Self::new(TaggedEffect::new(tag.into(), self))
    }

    /// Wrap this effect to tag ALL object targets for later reference by subsequent effects.
    ///
    /// Unlike `tag()` which only tags the first target, this method tags all object
    /// targets. This is useful for effects like "destroy all creatures" where
    /// subsequent effects need to reference all the destroyed creatures.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // "Destroy all creatures. Their controllers each create a 3/3 for each
    /// // creature they controlled that was destroyed this way."
    /// vec![
    ///     Effect::destroy_all(ObjectFilter::creature()).tag_all("destroyed"),
    ///     Effect::for_each_controller_of_tagged("destroyed", vec![
    ///         Effect::create_tokens_player(
    ///             elephant_token(),
    ///             Value::TaggedCount,
    ///             PlayerFilter::IteratedPlayer,
    ///         ),
    ///     ]),
    /// ]
    /// ```
    pub fn tag_all(self, tag: impl Into<TagKey>) -> Self {
        use crate::effects::TagAllEffect;
        Self::new(TagAllEffect::new(tag.into(), self))
    }

    /// Tag the triggering object for later reference by subsequent effects.
    ///
    /// This is used for triggered abilities that refer to "that creature/permanent".
    pub fn tag_triggering_object(tag: impl Into<TagKey>) -> Self {
        use crate::effects::TagTriggeringObjectEffect;
        Self::new(TagTriggeringObjectEffect::new(tag.into()))
    }

    /// Tag the damaged object from the triggering damage event.
    pub fn tag_triggering_damage_target(tag: impl Into<TagKey>) -> Self {
        use crate::effects::TagTriggeringDamageTargetEffect;
        Self::new(TagTriggeringDamageTargetEffect::new(tag.into()))
    }

    /// Tag the object attached to the source (equipment/aura) for later reference.
    pub fn tag_attached_to_source(tag: impl Into<TagKey>) -> Self {
        use crate::effects::TagAttachedToSourceEffect;
        Self::new(TagAttachedToSourceEffect::new(tag.into()))
    }

    /// Create a "can't" restriction effect with a specific duration.
    pub fn cant_until(restriction: Restriction, duration: Until) -> Self {
        use crate::effects::CantEffect;
        Self::new(CantEffect::new(restriction, duration))
    }
}

/// Duration for temporary effects.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum Until {
    /// Permanent (until removed)
    #[default]
    Forever,

    /// Until end of turn
    EndOfTurn,

    /// Until your next turn
    YourNextTurn,

    /// Until that permanent's controller's next untap step
    ControllersNextUntapStep,

    /// Until end of combat
    EndOfCombat,

    /// As long as source remains on battlefield
    ThisLeavesTheBattlefield,

    /// As long as you control source
    YouStopControllingThis,

    /// For a number of turns
    TurnsPass(Value),
}

/// A mode for modal spells.
#[derive(Debug, Clone, PartialEq)]
pub struct EffectMode {
    pub description: String,
    pub effects: Vec<Effect>,
}

/// A condition for conditional effects.
#[derive(Debug, Clone, PartialEq)]
pub enum Condition {
    /// You control an object matching filter
    YouControl(ObjectFilter),

    /// An opponent controls an object matching filter
    OpponentControls(ObjectFilter),

    /// A specific player controls an object matching filter.
    PlayerControls {
        player: PlayerFilter,
        filter: ObjectFilter,
    },

    /// A specific player has less life than you.
    PlayerHasLessLifeThanYou { player: PlayerFilter },

    /// Your life total is N or less
    LifeTotalOrLess(i32),

    /// Your life total is N or greater
    LifeTotalOrGreater(i32),

    /// You have N or more cards in hand
    CardsInHandOrMore(i32),

    /// It's your turn
    YourTurn,

    /// A creature died this turn
    CreatureDiedThisTurn,

    /// You cast a spell this turn
    CastSpellThisTurn,

    /// You attacked with one or more creatures this turn
    AttackedThisTurn,

    /// No spells were cast last turn
    NoSpellsWereCastLastTurn,

    /// Target is tapped
    TargetIsTapped,

    /// Target is attacking
    TargetIsAttacking,

    /// Target is blocked
    TargetIsBlocked,

    /// The targeted spell was kicked.
    TargetWasKicked,

    /// The targeted spell was the Nth spell cast this turn.
    TargetSpellCastOrderThisTurn(u32),

    /// The targeted spell's controller is poisoned.
    TargetSpellControllerIsPoisoned,

    /// At least N mana of a specific color/type was spent to cast the targeted spell.
    /// If `symbol` is `None`, checks total mana spent.
    TargetSpellManaSpentToCastAtLeast {
        amount: u32,
        symbol: Option<ManaSymbol>,
    },

    /// You control more creatures than the targeted spell's controller.
    YouControlMoreCreaturesThanTargetSpellController,

    /// The targeted creature has the greatest power among creatures on the battlefield.
    TargetHasGreatestPowerAmongCreatures,

    /// The target's mana value is less than or equal to the number of colors of mana spent to cast this spell.
    TargetManaValueLteColorsSpentToCastThisSpell,

    /// Source object is tapped
    SourceIsTapped,

    /// Source object has no counters of a specific type.
    SourceHasNoCounter(CounterType),

    /// At least N mana of a specific color/type was spent to cast this spell.
    /// If `symbol` is `None`, checks total mana spent instead.
    ManaSpentToCastThisSpellAtLeast {
        amount: u32,
        symbol: Option<ManaSymbol>,
    },

    /// You control your commander on the battlefield.
    /// Used for Commander-specific effects like Akroma's Will.
    YouControlCommander,

    /// A tagged object matches a filter.
    TaggedObjectMatches(TagKey, ObjectFilter),

    /// Negate another condition
    Not(Box<Condition>),

    /// Both conditions must be true
    And(Box<Condition>, Box<Condition>),

    /// Either condition must be true
    Or(Box<Condition>, Box<Condition>),
}

/// Description for creating an emblem.
#[derive(Debug, Clone)]
pub struct EmblemDescription {
    pub name: String,
    pub text: String,
    /// Abilities granted by this emblem.
    pub abilities: Vec<crate::ability::Ability>,
}

impl EmblemDescription {
    /// Create a new emblem description.
    pub fn new(name: &str, text: &str) -> Self {
        Self {
            name: name.to_string(),
            text: text.to_string(),
            abilities: Vec::new(),
        }
    }

    /// Add an ability to the emblem.
    pub fn with_ability(mut self, ability: crate::ability::Ability) -> Self {
        self.abilities.push(ability);
        self
    }
}

// === Builder methods for common effects ===

impl Effect {
    /// Create a "deal N damage to target" effect.
    pub fn deal_damage(amount: impl Into<Value>, target: ChooseSpec) -> Self {
        use crate::effects::DealDamageEffect;
        Self::new(DealDamageEffect::new(amount, target))
    }

    /// Create a "draw N cards" effect.
    pub fn draw(count: impl Into<Value>) -> Self {
        use crate::effects::DrawCardsEffect;
        Self::new(DrawCardsEffect::you(count))
    }

    /// Create a "target player draws N cards" effect.
    pub fn target_draws(count: impl Into<Value>, player: PlayerFilter) -> Self {
        use crate::effects::DrawCardsEffect;
        Self::new(DrawCardsEffect::new(count, player))
    }

    /// Create a "target creature connives" effect.
    pub fn connive(target: ChooseSpec) -> Self {
        use crate::effects::ConniveEffect;
        Self::new(ConniveEffect::new(target))
    }

    /// Create a "goad target creature" effect.
    pub fn goad(target: ChooseSpec) -> Self {
        use crate::effects::GoadEffect;
        Self::new(GoadEffect::new(target))
    }

    /// Create an "explore" effect for a chosen object.
    pub fn explore(target: ChooseSpec) -> Self {
        use crate::effects::ExploreEffect;
        Self::new(ExploreEffect::new(target))
    }

    /// Create an "open an Attraction" effect.
    pub fn open_attraction() -> Self {
        use crate::effects::OpenAttractionEffect;
        Self::new(OpenAttractionEffect::new())
    }

    /// Create a "manifest dread" effect.
    pub fn manifest_dread() -> Self {
        use crate::effects::ManifestDreadEffect;
        Self::new(ManifestDreadEffect::new())
    }

    /// Create a "bolster N" effect.
    pub fn bolster(amount: u32) -> Self {
        use crate::effects::BolsterEffect;
        Self::new(BolsterEffect::new(amount))
    }

    /// Create a "support N" effect.
    pub fn support(amount: u32) -> Self {
        use crate::effects::SupportEffect;
        Self::new(SupportEffect::new(amount))
    }

    /// Create an "adapt N" effect.
    pub fn adapt(amount: u32) -> Self {
        use crate::effects::AdaptEffect;
        Self::new(AdaptEffect::new(amount))
    }

    /// Create a "counter target activated or triggered ability" effect.
    pub fn counter_activated_or_triggered_ability() -> Self {
        use crate::effects::CounterAbilityEffect;
        Self::new(CounterAbilityEffect::new())
    }

    /// Create a "gain N life" effect.
    pub fn gain_life(amount: impl Into<Value>) -> Self {
        use crate::effects::GainLifeEffect;
        Self::new(GainLifeEffect::you(amount))
    }

    /// Create a "gain N life" effect for a specific player.
    pub fn gain_life_player(amount: impl Into<Value>, player: ChooseSpec) -> Self {
        use crate::effects::GainLifeEffect;
        Self::new(GainLifeEffect::new(amount, player))
    }

    /// Create a "target player gains N life" effect.
    pub fn gain_life_target(amount: impl Into<Value>) -> Self {
        use crate::effects::GainLifeEffect;
        Self::new(GainLifeEffect::target_player(amount))
    }

    /// Create a "lose N life" effect.
    pub fn lose_life(amount: impl Into<Value>) -> Self {
        use crate::effects::LoseLifeEffect;
        Self::new(LoseLifeEffect::you(amount))
    }

    /// Create a "lose N life" effect for a specific player.
    pub fn lose_life_player(amount: impl Into<Value>, player: PlayerFilter) -> Self {
        use crate::effects::LoseLifeEffect;
        Self::new(LoseLifeEffect::with_filter(amount, player))
    }

    /// Create a "target player loses N life" effect.
    pub fn lose_life_target(amount: impl Into<Value>) -> Self {
        use crate::effects::LoseLifeEffect;
        Self::new(LoseLifeEffect::target_player(amount))
    }

    /// Create a "set life total to N" effect.
    pub fn set_life_total(amount: impl Into<Value>) -> Self {
        use crate::effects::SetLifeTotalEffect;
        Self::new(SetLifeTotalEffect::you(amount))
    }

    /// Create a "set life total to N" effect for a specific player.
    pub fn set_life_total_player(amount: impl Into<Value>, player: PlayerFilter) -> Self {
        use crate::effects::SetLifeTotalEffect;
        Self::new(SetLifeTotalEffect::new(amount, player))
    }

    /// Create an "exchange life totals" effect.
    pub fn exchange_life_totals(player1: PlayerFilter, player2: PlayerFilter) -> Self {
        use crate::effects::ExchangeLifeTotalsEffect;
        Self::new(ExchangeLifeTotalsEffect::new(player1, player2))
    }

    /// Create a "destroy target permanent" effect.
    pub fn destroy(choice: ChooseSpec) -> Self {
        use crate::effects::DestroyEffect;
        Self::new(DestroyEffect::target(choice))
    }

    /// Create an "exile target" effect.
    pub fn exile(choice: ChooseSpec) -> Self {
        use crate::effects::ExileEffect;
        Self::new(ExileEffect::target(choice))
    }

    /// Create an "exile any number of targets" effect.
    pub fn exile_any_number(choice: ChooseSpec) -> Self {
        use crate::effects::ExileEffect;
        Self::new(ExileEffect::any_number(choice))
    }

    /// Create an "exile target until this source leaves the battlefield" effect.
    pub fn exile_until_source_leaves(choice: ChooseSpec) -> Self {
        use crate::effects::ExileUntilEffect;
        Self::new(ExileUntilEffect::source_leaves(choice))
    }

    /// Create an "exile ... until <duration>" effect.
    pub fn exile_until(choice: ChooseSpec, duration: crate::effects::ExileUntilDuration) -> Self {
        use crate::effects::ExileUntilEffect;
        Self::new(ExileUntilEffect::new(choice, duration))
    }

    /// Create a "sacrifice" effect.
    pub fn sacrifice(filter: ObjectFilter, count: impl Into<Value>) -> Self {
        use crate::effects::SacrificeEffect;
        Self::new(SacrificeEffect::you(filter, count))
    }

    /// Create a "sacrifice" effect for a specific player.
    pub fn sacrifice_player(
        filter: ObjectFilter,
        count: impl Into<Value>,
        player: PlayerFilter,
    ) -> Self {
        use crate::effects::SacrificeEffect;
        Self::new(SacrificeEffect::player(filter, count, player))
    }

    /// Create a "sacrifice source" effect (self-sacrifice).
    pub fn sacrifice_source() -> Self {
        use crate::effects::SacrificeTargetEffect;
        Self::new(SacrificeTargetEffect::source())
    }

    /// Create a "return target card to its owner's hand" effect.
    pub fn return_to_hand(objects: ObjectFilter) -> Self {
        use crate::effects::ReturnToHandEffect;
        Self::new(ReturnToHandEffect::target(ChooseSpec::Object(objects)))
    }

    /// Create a "destroy all permanents matching filter" effect.
    pub fn destroy_all(filter: ObjectFilter) -> Self {
        use crate::effects::DestroyEffect;
        Self::new(DestroyEffect::all(filter))
    }

    /// Create an "exile all permanents matching filter" effect.
    pub fn exile_all(filter: ObjectFilter) -> Self {
        use crate::effects::ExileEffect;
        Self::new(ExileEffect::all(filter))
    }

    /// Create a "return all permanents matching filter to owners' hands" effect.
    pub fn return_all_to_hand(filter: ObjectFilter) -> Self {
        use crate::effects::ReturnToHandEffect;
        Self::new(ReturnToHandEffect::all(filter))
    }

    /// Create an "each player sacrifices permanents matching filter" effect.
    ///
    /// This is a composed effect using `for_players` with `sacrifice_player`.
    pub fn each_player_sacrifices(filter: ObjectFilter, count: impl Into<Value>) -> Self {
        let count_value = count.into();
        Self::for_players(
            PlayerFilter::Any,
            vec![Self::sacrifice_player(
                filter,
                count_value,
                PlayerFilter::IteratedPlayer,
            )],
        )
    }

    /// Create a "move target to zone" effect.
    pub fn move_to_zone(target: ChooseSpec, zone: Zone, to_top: bool) -> Self {
        use crate::effects::MoveToZoneEffect;
        Self::new(MoveToZoneEffect::new(target, zone, to_top))
    }

    /// Create a "return target card from graveyard to hand" effect.
    pub fn return_from_graveyard_to_hand(target: ChooseSpec) -> Self {
        use crate::effects::ReturnFromGraveyardToHandEffect;
        Self::new(ReturnFromGraveyardToHandEffect::new(target))
    }

    /// Create a "return target card from graveyard to battlefield" effect.
    pub fn return_from_graveyard_to_battlefield(target: ChooseSpec, tapped: bool) -> Self {
        use crate::effects::ReturnFromGraveyardToBattlefieldEffect;
        Self::new(ReturnFromGraveyardToBattlefieldEffect::new(target, tapped))
    }

    /// Create a "return this card from graveyard or exile to battlefield" effect.
    ///
    /// This effect uses the triggering event's snapshot to locate the card.
    pub fn return_from_graveyard_or_exile_to_battlefield(tapped: bool) -> Self {
        use crate::effects::ReturnFromGraveyardOrExileToBattlefieldEffect;
        Self::new(ReturnFromGraveyardOrExileToBattlefieldEffect::new(tapped))
    }

    /// Create a "put onto battlefield" effect.
    pub fn put_onto_battlefield(
        target: ChooseSpec,
        tapped: bool,
        controller: PlayerFilter,
    ) -> Self {
        use crate::effects::PutOntoBattlefieldEffect;
        Self::new(PutOntoBattlefieldEffect::new(target, tapped, controller))
    }

    /// Create a "counter target spell" effect.
    pub fn counter(target: ChooseSpec) -> Self {
        use crate::effects::CounterEffect;
        Self::new(CounterEffect::new(target))
    }

    /// Create a "counter unless pays" effect.
    pub fn counter_unless_pays(target: ChooseSpec, mana: Vec<ManaSymbol>) -> Self {
        let player = match target.base() {
            ChooseSpec::SpecificObject(id) => PlayerFilter::ControllerOf(ObjectRef::Specific(*id)),
            ChooseSpec::Tagged(tag) => PlayerFilter::ControllerOf(ObjectRef::Tagged(tag.clone())),
            _ => PlayerFilter::ControllerOf(ObjectRef::Target),
        };
        Self::unless_pays(vec![Self::counter(target)], player, mana)
    }

    /// Create a "copy target spell" effect.
    pub fn copy_spell(target: ChooseSpec) -> Self {
        use crate::effects::CopySpellEffect;
        Self::new(CopySpellEffect::single(target))
    }

    /// Create a "copy target spell N times" effect.
    pub fn copy_spell_n(target: ChooseSpec, count: impl Into<Value>) -> Self {
        use crate::effects::CopySpellEffect;
        Self::new(CopySpellEffect::new(target, count))
    }

    /// Create a "choose new targets" effect for objects from a prior effect result.
    pub fn choose_new_targets(from_effect: EffectId) -> Self {
        use crate::effects::ChooseNewTargetsEffect;
        Self::new(ChooseNewTargetsEffect::must(from_effect))
    }

    /// Create a "you may choose new targets" effect for objects from a prior effect result.
    pub fn may_choose_new_targets(from_effect: EffectId) -> Self {
        use crate::effects::ChooseNewTargetsEffect;
        Self::new(ChooseNewTargetsEffect::may(from_effect))
    }

    /// Create a "[player] may choose new targets" effect.
    pub fn may_choose_new_targets_player(from_effect: EffectId, chooser: PlayerFilter) -> Self {
        use crate::effects::ChooseNewTargetsEffect;
        Self::new(ChooseNewTargetsEffect::may_for_player(from_effect, chooser))
    }

    /// Create a "create N tokens" effect.
    pub fn create_tokens(token: crate::cards::CardDefinition, count: impl Into<Value>) -> Self {
        use crate::effects::CreateTokenEffect;
        Self::new(CreateTokenEffect::you(token, count))
    }

    /// Create an "investigate N times" effect.
    pub fn investigate(count: impl Into<Value>) -> Self {
        use crate::effects::InvestigateEffect;
        Self::new(InvestigateEffect::new(count))
    }

    /// Create a "create N tokens" effect for a specific player.
    pub fn create_tokens_player(
        token: crate::cards::CardDefinition,
        count: impl Into<Value>,
        controller: PlayerFilter,
    ) -> Self {
        use crate::effects::CreateTokenEffect;
        Self::new(CreateTokenEffect::new(token, count, controller))
    }

    /// Create a "create token copy of target" effect.
    pub fn create_token_copy(target: ChooseSpec) -> Self {
        use crate::effects::CreateTokenCopyEffect;
        Self::new(CreateTokenCopyEffect::one(target))
    }

    /// Create a "create token copy with haste that's exiled at end of combat" effect.
    /// Used for Kiki-Jiki style effects.
    pub fn create_token_copy_kiki_jiki(target: ChooseSpec) -> Self {
        use crate::effects::CreateTokenCopyEffect;
        Self::new(CreateTokenCopyEffect::kiki_jiki_style(target))
    }

    /// Create a "create token copy with haste" effect.
    pub fn create_token_copy_with_haste(target: ChooseSpec) -> Self {
        use crate::effects::CreateTokenCopyEffect;
        Self::new(CreateTokenCopyEffect::with_haste(target))
    }

    /// Create a "put N +1/+1 counters on target" effect.
    pub fn plus_one_counters(count: impl Into<Value>, target: ChooseSpec) -> Self {
        use crate::effects::PutCountersEffect;
        Self::new(PutCountersEffect::plus_one_counters(count, target))
    }

    /// Create a "put counters on target" effect.
    pub fn put_counters(
        counter_type: CounterType,
        count: impl Into<Value>,
        target: ChooseSpec,
    ) -> Self {
        use crate::effects::PutCountersEffect;
        Self::new(PutCountersEffect::new(counter_type, count, target))
    }

    /// Create a "put counters on source" effect.
    pub fn put_counters_on_source(counter_type: CounterType, count: impl Into<Value>) -> Self {
        use crate::effects::PutCountersEffect;
        Self::new(PutCountersEffect::on_source(counter_type, count))
    }

    /// Create a "remove counters from target" effect.
    pub fn remove_counters(
        counter_type: CounterType,
        count: impl Into<Value>,
        target: ChooseSpec,
    ) -> Self {
        use crate::effects::RemoveCountersEffect;
        Self::new(RemoveCountersEffect::new(counter_type, count, target))
    }

    /// Create a "remove up to N counters from target" effect (player chooses how many).
    pub fn remove_up_to_counters(
        counter_type: CounterType,
        max_count: impl Into<Value>,
        target: ChooseSpec,
    ) -> Self {
        use crate::effects::RemoveUpToCountersEffect;
        Self::new(RemoveUpToCountersEffect::new(
            counter_type,
            max_count,
            target,
        ))
    }

    /// Create a "remove up to N counters of any type from target" effect.
    pub fn remove_up_to_any_counters(max_count: impl Into<Value>, target: ChooseSpec) -> Self {
        use crate::effects::RemoveUpToAnyCountersEffect;
        Self::new(RemoveUpToAnyCountersEffect::new(max_count, target))
    }

    /// Create a "move counters from one permanent to another" effect.
    pub fn move_counters(
        counter_type: CounterType,
        count: impl Into<Value>,
        from: ChooseSpec,
        to: ChooseSpec,
    ) -> Self {
        use crate::effects::MoveCountersEffect;
        Self::new(MoveCountersEffect::new(counter_type, count, from, to))
    }

    /// Create a "move all counters from one creature to another" effect (Fate Transfer).
    pub fn move_all_counters(from: ChooseSpec, to: ChooseSpec) -> Self {
        use crate::effects::MoveAllCountersEffect;
        Self::new(MoveAllCountersEffect::new(from, to))
    }

    /// Create a "proliferate" effect.
    pub fn proliferate() -> Self {
        use crate::effects::ProliferateEffect;
        Self::new(ProliferateEffect::new())
    }

    /// Create a "+N/+M" effect with explicit duration.
    pub fn pump(
        power: impl Into<Value>,
        toughness: impl Into<Value>,
        target: ChooseSpec,
        duration: Until,
    ) -> Self {
        use crate::effects::ModifyPowerToughnessEffect;
        Self::new(ModifyPowerToughnessEffect::new(
            target, power, toughness, duration,
        ))
    }

    /// Create a "set base power/toughness to N/M" effect with explicit duration.
    pub fn set_base_power_toughness(
        power: impl Into<Value>,
        toughness: impl Into<Value>,
        target: ChooseSpec,
        duration: Until,
    ) -> Self {
        use crate::effects::SetBasePowerToughnessEffect;
        Self::new(SetBasePowerToughnessEffect::new(
            target, power, toughness, duration,
        ))
    }

    /// Create a "+N/+M" effect for all creatures matching a filter with explicit duration.
    pub fn pump_all(
        filter: ObjectFilter,
        power: impl Into<Value>,
        toughness: impl Into<Value>,
        duration: Until,
    ) -> Self {
        use crate::effects::ModifyPowerToughnessAllEffect;
        Self::new(ModifyPowerToughnessAllEffect::new(
            filter, power, toughness, duration,
        ))
    }

    /// Create a "+X/+X per count" effect with explicit duration.
    pub fn pump_for_each(
        target: ChooseSpec,
        power_per: i32,
        toughness_per: i32,
        count: Value,
        duration: Until,
    ) -> Self {
        use crate::effects::ModifyPowerToughnessForEachEffect;
        Self::new(ModifyPowerToughnessForEachEffect::new(
            target,
            power_per,
            toughness_per,
            count,
            duration,
        ))
    }

    /// Create a "fight" effect.
    pub fn fight(creature1: ChooseSpec, creature2: ChooseSpec) -> Self {
        use crate::effects::FightEffect;
        Self::new(FightEffect::new(creature1, creature2))
    }

    /// Create a "prevent damage" effect with explicit duration.
    pub fn prevent_damage(amount: impl Into<Value>, target: ChooseSpec, duration: Until) -> Self {
        use crate::effects::PreventDamageEffect;
        Self::new(PreventDamageEffect::new(amount, target, duration))
    }

    /// Create a "prevent all damage" effect.
    pub fn prevent_all_damage(until: Until) -> Self {
        use crate::effects::PreventAllDamageEffect;
        Self::new(PreventAllDamageEffect::all(until))
    }

    /// Create a "prevent all combat damage" effect.
    pub fn prevent_all_combat_damage(until: Until) -> Self {
        use crate::effects::PreventAllDamageEffect;
        use crate::prevention::DamageFilter;
        Self::new(PreventAllDamageEffect::all_with_filter(
            DamageFilter::combat(),
            until,
        ))
    }

    /// Create a "prevent all combat damage from a chosen source" effect.
    pub fn prevent_all_combat_damage_from(source: ChooseSpec, until: Until) -> Self {
        use crate::effects::PreventAllCombatDamageFromEffect;
        Self::new(PreventAllCombatDamageFromEffect::new(source, until))
    }

    /// Create a "prevent all combat damage to players" effect.
    pub fn prevent_all_combat_damage_to_players(until: Until) -> Self {
        use crate::effects::PreventAllDamageEffect;
        use crate::prevention::{DamageFilter, PreventionTarget};
        Self::new(PreventAllDamageEffect::new(
            PreventionTarget::Players,
            DamageFilter::combat(),
            until,
        ))
    }

    /// Create a "prevent all combat damage to you" effect.
    pub fn prevent_all_combat_damage_to_you(until: Until) -> Self {
        use crate::effects::PreventAllDamageEffect;
        use crate::prevention::{DamageFilter, PreventionTarget};
        Self::new(PreventAllDamageEffect::new(
            PreventionTarget::You,
            DamageFilter::combat(),
            until,
        ))
    }

    /// Create a "prevent all damage to matching permanents" effect.
    pub fn prevent_all_damage_to(filter: ObjectFilter, until: Until) -> Self {
        use crate::effects::PreventAllDamageEffect;
        Self::new(PreventAllDamageEffect::matching(filter, until))
    }

    /// Create a "prevent all damage to target" effect.
    pub fn prevent_all_damage_to_target(target: ChooseSpec, until: Until) -> Self {
        use crate::effects::PreventAllDamageToTargetEffect;
        Self::new(PreventAllDamageToTargetEffect::new(target, until))
    }

    /// Create a "grant abilities to all matching creatures" effect with explicit duration.
    pub fn grant_abilities_all(
        filter: ObjectFilter,
        abilities: Vec<crate::static_abilities::StaticAbility>,
        duration: Until,
    ) -> Self {
        use crate::effects::GrantAbilitiesAllEffect;
        Self::new(GrantAbilitiesAllEffect::new(filter, abilities, duration))
    }

    /// Create an "add mana" effect.
    pub fn add_mana(mana: Vec<ManaSymbol>) -> Self {
        use crate::effects::AddManaEffect;
        Self::new(AddManaEffect::you(mana))
    }

    /// Create an "add mana" effect for a specific player.
    pub fn add_mana_player(mana: Vec<ManaSymbol>, player: PlayerFilter) -> Self {
        use crate::effects::AddManaEffect;
        Self::new(AddManaEffect::new(mana, player))
    }

    /// Create an "add colorless mana" effect.
    pub fn add_colorless_mana(amount: impl Into<Value>) -> Self {
        use crate::effects::AddColorlessManaEffect;
        Self::new(AddColorlessManaEffect::you(amount))
    }

    /// Create an "add colorless mana" effect for a specific player.
    pub fn add_colorless_mana_player(amount: impl Into<Value>, player: PlayerFilter) -> Self {
        use crate::effects::AddColorlessManaEffect;
        Self::new(AddColorlessManaEffect::new(amount, player))
    }

    /// Create an "add mana of any color" effect (can choose different colors).
    pub fn add_mana_of_any_color(amount: impl Into<Value>) -> Self {
        use crate::effects::AddManaOfAnyColorEffect;
        Self::new(AddManaOfAnyColorEffect::you(amount))
    }

    /// Create an "add mana of any color" effect for a specific player.
    pub fn add_mana_of_any_color_player(amount: impl Into<Value>, player: PlayerFilter) -> Self {
        use crate::effects::AddManaOfAnyColorEffect;
        Self::new(AddManaOfAnyColorEffect::new(amount, player))
    }

    /// Create an "add mana of restricted colors" effect (can choose different colors).
    pub fn add_mana_of_any_color_restricted(
        amount: impl Into<Value>,
        colors: Vec<crate::color::Color>,
    ) -> Self {
        use crate::effects::AddManaOfAnyColorEffect;
        Self::new(AddManaOfAnyColorEffect::you_restricted(amount, colors))
    }

    /// Create an "add mana of restricted colors" effect for a specific player.
    pub fn add_mana_of_any_color_restricted_player(
        amount: impl Into<Value>,
        player: PlayerFilter,
        colors: Vec<crate::color::Color>,
    ) -> Self {
        use crate::effects::AddManaOfAnyColorEffect;
        Self::new(AddManaOfAnyColorEffect::restricted(amount, player, colors))
    }

    /// Create an "add mana of any one color" effect (all must be same color).
    pub fn add_mana_of_any_one_color(amount: impl Into<Value>) -> Self {
        use crate::effects::AddManaOfAnyOneColorEffect;
        Self::new(AddManaOfAnyOneColorEffect::you(amount))
    }

    /// Create an "add mana of any one color" effect for a specific player.
    pub fn add_mana_of_any_one_color_player(
        amount: impl Into<Value>,
        player: PlayerFilter,
    ) -> Self {
        use crate::effects::AddManaOfAnyOneColorEffect;
        Self::new(AddManaOfAnyOneColorEffect::new(amount, player))
    }

    /// Create an "add mana constrained by what matching lands could produce" effect.
    pub fn add_mana_of_land_produced_types_player(
        amount: impl Into<Value>,
        player: PlayerFilter,
        land_filter: ObjectFilter,
        allow_colorless: bool,
        same_type: bool,
    ) -> Self {
        use crate::effects::AddManaOfLandProducedTypesEffect;
        Self::new(AddManaOfLandProducedTypesEffect::new(
            amount,
            player,
            land_filter,
            allow_colorless,
            same_type,
        ))
    }

    /// Create an "add mana from commander color identity" effect.
    pub fn add_mana_from_commander_color_identity(amount: impl Into<Value>) -> Self {
        use crate::effects::AddManaFromCommanderColorIdentityEffect;
        Self::new(AddManaFromCommanderColorIdentityEffect::you(amount))
    }

    /// Create an "add mana from commander color identity" effect for a specific player.
    pub fn add_mana_from_commander_color_identity_player(
        amount: impl Into<Value>,
        player: PlayerFilter,
    ) -> Self {
        use crate::effects::AddManaFromCommanderColorIdentityEffect;
        Self::new(AddManaFromCommanderColorIdentityEffect::new(amount, player))
    }

    /// Create a "tap target permanent" effect.
    pub fn tap(target: ChooseSpec) -> Self {
        use crate::effects::TapEffect;
        Self::new(TapEffect::target(target))
    }

    /// Create an "untap target permanent" effect.
    pub fn untap(target: ChooseSpec) -> Self {
        use crate::effects::UntapEffect;
        Self::new(UntapEffect::target(target))
    }

    /// Create a "tap all permanents matching filter" effect.
    pub fn tap_all(filter: ObjectFilter) -> Self {
        use crate::effects::TapEffect;
        Self::new(TapEffect::all(filter))
    }

    /// Create a "tap the source permanent" effect.
    ///
    /// Used for cost effects that require tapping the ability's source.
    pub fn tap_source() -> Self {
        use crate::effects::TapEffect;
        Self::new(TapEffect::source())
    }

    /// Create a "pay life" effect (controller loses life).
    ///
    /// Used for cost effects that require paying life.
    /// Note: Paying life is identical to losing life in the rules.
    pub fn pay_life(amount: u32) -> Self {
        use crate::effects::LoseLifeEffect;
        Self::new(LoseLifeEffect::you(amount))
    }

    /// Create an "exile card(s) from hand as cost" effect.
    ///
    /// Used for alternative casting costs like Force of Will.
    /// The controller exiles the specified number of cards from their hand,
    /// optionally filtered by color.
    ///
    /// Note: This effect requires player choice when there are more cards
    /// than needed. The game loop handles prompting for the selection.
    pub fn exile_from_hand_as_cost(
        count: u32,
        color_filter: Option<crate::color::ColorSet>,
    ) -> Self {
        use crate::effects::ExileFromHandAsCostEffect;
        Self::new(ExileFromHandAsCostEffect::new(count, color_filter))
    }

    /// Create an "untap all permanents matching filter" effect.
    pub fn untap_all(filter: ObjectFilter) -> Self {
        use crate::effects::UntapEffect;
        Self::new(UntapEffect::all(filter))
    }

    /// Create a "clear all damage from target creature" effect.
    pub fn clear_damage(target: ChooseSpec) -> Self {
        use crate::effects::ClearDamageEffect;
        Self::new(ClearDamageEffect::new(target))
    }

    /// Create a "monstrosity N" effect.
    pub fn monstrosity(n: impl Into<Value>) -> Self {
        use crate::effects::MonstrosityEffect;
        Self::new(MonstrosityEffect::new(n))
    }

    /// Create a "regenerate" effect with explicit duration.
    pub fn regenerate(target: ChooseSpec, duration: Until) -> Self {
        use crate::effects::RegenerateEffect;
        Self::new(RegenerateEffect::new(target, duration))
    }

    /// Create a "transform" effect for double-faced cards.
    pub fn transform(target: ChooseSpec) -> Self {
        use crate::effects::TransformEffect;
        Self::new(TransformEffect::new(target))
    }

    /// Create a "create emblem" effect.
    pub fn create_emblem(emblem: EmblemDescription) -> Self {
        use crate::effects::CreateEmblemEffect;
        Self::new(CreateEmblemEffect::new(emblem))
    }

    /// Create a unified grant effect.
    ///
    /// This is the preferred way to create effects that grant abilities or
    /// alternative casting methods to cards.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// // Grant flashback until end of turn (Snapcaster Mage)
    /// Effect::grant(
    ///     Grantable::flashback_use_targets_cost(),
    ///     target,
    ///     GrantDuration::UntilEndOfTurn,
    /// )
    ///
    /// // Grant flying until end of turn
    /// Effect::grant(
    ///     Grantable::ability(StaticAbility::flying()),
    ///     target,
    ///     GrantDuration::UntilEndOfTurn,
    /// )
    /// ```
    pub fn grant(
        grantable: crate::grant::Grantable,
        target: ChooseSpec,
        duration: crate::grant::GrantDuration,
    ) -> Self {
        use crate::effects::GrantEffect;
        Self::new(GrantEffect::new(grantable, target, duration))
    }

    /// Create a "grant flashback until end of turn" effect using the unified grant system.
    ///
    /// This is a convenience method for the Snapcaster Mage pattern.
    pub fn grant_flashback_until_eot_unified(target: ChooseSpec) -> Self {
        use crate::effects::GrantEffect;
        Self::new(GrantEffect::flashback_until_eot(target))
    }

    /// Create an effect that grants an ability directly to an object.
    ///
    /// This is useful for effects like saga chapters that say "this permanent gains ...".
    pub fn grant_object_ability(ability: crate::ability::Ability, target: ChooseSpec) -> Self {
        use crate::effects::GrantObjectAbilityEffect;
        Self::new(GrantObjectAbilityEffect::new(ability, target))
    }

    /// Create an effect that grants an ability directly to the source object.
    pub fn grant_object_ability_to_source(ability: crate::ability::Ability) -> Self {
        use crate::effects::GrantObjectAbilityEffect;
        Self::new(GrantObjectAbilityEffect::to_source(ability))
    }

    /// Create an "attach to" effect for Auras and Equipment.
    pub fn attach_to(target: ChooseSpec) -> Self {
        use crate::effects::AttachToEffect;
        Self::new(AttachToEffect::new(target))
    }

    /// Create an effect that attaches one or more objects to a target object.
    pub fn attach_objects(objects: ChooseSpec, target: ChooseSpec) -> Self {
        use crate::effects::AttachObjectsEffect;
        Self::new(AttachObjectsEffect::new(objects, target))
    }

    /// Create a "mill N cards" effect.
    pub fn mill(count: impl Into<Value>) -> Self {
        use crate::effects::MillEffect;
        Self::new(MillEffect::you(count))
    }

    /// Create a "mill N cards" effect for a specific player.
    pub fn mill_player(count: impl Into<Value>, player: PlayerFilter) -> Self {
        use crate::effects::MillEffect;
        Self::new(MillEffect::new(count, player))
    }

    /// Create a "shuffle library" effect.
    pub fn shuffle_library() -> Self {
        use crate::effects::ShuffleLibraryEffect;
        Self::new(ShuffleLibraryEffect::you())
    }

    /// Create a "shuffle library" effect for a specific player.
    pub fn shuffle_library_player(player: PlayerFilter) -> Self {
        use crate::effects::ShuffleLibraryEffect;
        Self::new(ShuffleLibraryEffect::new(player))
    }

    /// Create a "shuffle graveyard into library" effect for a specific player.
    pub fn shuffle_graveyard_into_library_player(player: PlayerFilter) -> Self {
        use crate::effects::ShuffleGraveyardIntoLibraryEffect;
        Self::new(ShuffleGraveyardIntoLibraryEffect::new(player))
    }

    /// Create a "poison counters" effect.
    pub fn poison_counters(count: impl Into<Value>) -> Self {
        use crate::effects::PoisonCountersEffect;
        Self::new(PoisonCountersEffect::you(count))
    }

    /// Create a "poison counters" effect for a specific player.
    pub fn poison_counters_player(count: impl Into<Value>, player: PlayerFilter) -> Self {
        use crate::effects::PoisonCountersEffect;
        Self::new(PoisonCountersEffect::new(count, player))
    }

    /// Create an "energy counters" effect.
    pub fn energy_counters(count: impl Into<Value>) -> Self {
        use crate::effects::EnergyCountersEffect;
        Self::new(EnergyCountersEffect::you(count))
    }

    /// Create an "energy counters" effect for a specific player.
    pub fn energy_counters_player(count: impl Into<Value>, player: PlayerFilter) -> Self {
        use crate::effects::EnergyCountersEffect;
        Self::new(EnergyCountersEffect::new(count, player))
    }

    /// Create an "experience counters" effect.
    pub fn experience_counters(count: impl Into<Value>) -> Self {
        use crate::effects::ExperienceCountersEffect;
        Self::new(ExperienceCountersEffect::you(count))
    }

    /// Create an "experience counters" effect for a specific player.
    pub fn experience_counters_player(count: impl Into<Value>, player: PlayerFilter) -> Self {
        use crate::effects::ExperienceCountersEffect;
        Self::new(ExperienceCountersEffect::new(count, player))
    }

    // === Effect composition builders ===

    /// Wrap an effect with an ID for later reference.
    ///
    /// Example: Label a damage effect so we can reference how much damage was dealt.
    /// ```ignore
    /// Effect::with_id(0, Effect::deal_damage(3, target))
    /// ```
    pub fn with_id(id: u32, effect: Effect) -> Self {
        use crate::effects::WithIdEffect;
        Self::new(WithIdEffect::new(EffectId(id), effect))
    }

    /// "You may X" - wrap effects in a player choice.
    ///
    /// Example: "You may draw a card."
    /// ```ignore
    /// Effect::may(vec![Effect::draw(1)])
    ///
    /// // "You may sacrifice a creature" - composed effects
    /// Effect::may(vec![
    ///     Effect::choose_objects(ObjectFilter::creature().you_control(), 1, PlayerFilter::You, "sac"),
    ///     Effect::sacrifice(ChooseSpec::tagged("sac")),
    /// ])
    /// ```
    pub fn may(effects: Vec<Effect>) -> Self {
        use crate::effects::MayEffect;
        Self::new(MayEffect::new(effects))
    }

    /// "[player] may X" - wrap effects in a choice made by the specified player.
    pub fn may_player(player: PlayerFilter, effects: Vec<Effect>) -> Self {
        use crate::effects::MayEffect;
        Self::new(MayEffect::new_for_player(effects, player))
    }

    /// "You may X" - wrap a single effect in a player choice (convenience).
    ///
    /// Example: "You may draw a card."
    /// ```ignore
    /// Effect::may_single(Effect::draw(1))
    /// ```
    pub fn may_single(effect: Effect) -> Self {
        use crate::effects::MayEffect;
        Self::new(MayEffect::single(effect))
    }

    /// "X unless you/they pay {mana}" - execute effects unless the player pays.
    ///
    /// Example: "Sacrifice this creature unless you pay {U}."
    /// ```ignore
    /// Effect::unless_pays(
    ///     vec![Effect::sacrifice_self()],
    ///     PlayerFilter::You,
    ///     vec![ManaSymbol::Blue],
    /// )
    /// ```
    pub fn unless_pays(effects: Vec<Effect>, player: PlayerFilter, mana: Vec<ManaSymbol>) -> Self {
        use crate::effects::UnlessPaysEffect;
        Self::new(UnlessPaysEffect::new(effects, player, mana))
    }

    /// "X unless you/they [action]" - execute effects unless the player performs an action.
    ///
    /// Example: "Sacrifice this creature unless you sacrifice another creature."
    pub fn unless_action(
        effects: Vec<Effect>,
        alternative: Vec<Effect>,
        player: PlayerFilter,
    ) -> Self {
        use crate::effects::UnlessActionEffect;
        Self::new(UnlessActionEffect::new(effects, alternative, player))
    }

    /// "If [prior effect satisfied predicate], then [effects]."
    ///
    /// Example: "If you do, draw two cards."
    /// ```ignore
    /// Effect::if_then(EffectId(0), EffectPredicate::Happened, vec![Effect::draw(2)])
    /// ```
    pub fn if_then(condition: EffectId, predicate: EffectPredicate, then: Vec<Effect>) -> Self {
        use crate::effects::IfEffect;
        Self::new(IfEffect::new(condition, predicate, then, vec![]))
    }

    /// "If [prior effect satisfied predicate], then [then], else [else_]."
    ///
    /// Example: "If you don't, you lose the game."
    /// ```ignore
    /// Effect::if_then_else(
    ///     EffectId(0),
    ///     EffectPredicate::Happened,
    ///     vec![],
    ///     vec![Effect::LoseTheGame { player: PlayerFilter::You }],
    /// )
    /// ```
    pub fn if_then_else(
        condition: EffectId,
        predicate: EffectPredicate,
        then: Vec<Effect>,
        else_: Vec<Effect>,
    ) -> Self {
        use crate::effects::IfEffect;
        Self::new(IfEffect::new(condition, predicate, then, else_))
    }

    /// Create a "for each object matching filter" effect.
    ///
    /// Example: "For each creature you control, gain 1 life."
    /// ```ignore
    /// Effect::for_each(ObjectFilter::creature().you_control(), vec![Effect::gain_life(1)])
    /// ```
    pub fn for_each(filter: ObjectFilter, effects: Vec<Effect>) -> Self {
        use crate::effects::ForEachObject;
        Self::new(ForEachObject::new(filter, effects))
    }

    /// Create a "for each opponent" effect.
    ///
    /// Example: "Deal 3 damage to each opponent."
    /// ```ignore
    /// Effect::for_each_opponent(vec![Effect::deal_damage(3, ChooseSpec::Player(PlayerFilter::IteratedPlayer))])
    /// ```
    ///
    /// Note: This is a convenience wrapper around `for_players(PlayerFilter::Opponent, ...)`.
    pub fn for_each_opponent(effects: Vec<Effect>) -> Self {
        Self::for_players(crate::filter::PlayerFilter::Opponent, effects)
    }

    /// Create an effect that executes for each player matching a filter.
    ///
    /// Sets `ctx.iterated_player` for each iteration, allowing inner effects
    /// to reference the current player via `PlayerFilter::IteratedPlayer`.
    ///
    /// # Examples
    ///
    /// Deal 3 damage to each opponent:
    /// ```ignore
    /// Effect::for_players(PlayerFilter::Opponent, vec![
    ///     Effect::deal_damage(3, ChooseSpec::Player(PlayerFilter::IteratedPlayer)),
    /// ])
    /// ```
    ///
    /// Each player draws a card:
    /// ```ignore
    /// Effect::for_players(PlayerFilter::Any, vec![
    ///     Effect::target_draws(1, PlayerFilter::IteratedPlayer),
    /// ])
    /// ```
    pub fn for_players(filter: PlayerFilter, effects: Vec<Effect>) -> Self {
        use crate::effects::ForPlayersEffect;
        Self::new(ForPlayersEffect::new(filter, effects))
    }

    /// Create an effect that executes for each tagged object.
    ///
    /// Sets `ctx.iterated_object` for each iteration, allowing inner effects
    /// to reference the current object.
    ///
    /// Example: "For each creature destroyed this way, its controller loses 1 life."
    /// ```ignore
    /// vec![
    ///     Effect::destroy_all(ObjectFilter::creature()).tag_all("destroyed"),
    ///     Effect::for_each_tagged("destroyed", vec![
    ///         Effect::lose_life_player(1, PlayerFilter::ControllerOf(ObjectRef::Iterated)),
    ///     ]),
    /// ]
    /// ```
    pub fn for_each_tagged(tag: impl Into<TagKey>, effects: Vec<Effect>) -> Self {
        use crate::effects::ForEachTaggedEffect;
        Self::new(ForEachTaggedEffect::new(tag.into(), effects))
    }

    /// Create an effect that groups tagged objects by controller and executes for each.
    ///
    /// Sets `ctx.iterated_player` for each iteration and provides a count via
    /// `Value::TaggedCount`. This enables patterns like "each player creates a
    /// token for each creature they controlled that was destroyed."
    ///
    /// Example: "Destroy all creatures. Their controllers each create a 3/3 Elephant
    /// for each creature they controlled that was destroyed this way."
    /// ```ignore
    /// vec![
    ///     Effect::destroy_all(ObjectFilter::creature()).tag_all("destroyed"),
    ///     Effect::for_each_controller_of_tagged("destroyed", vec![
    ///         Effect::create_tokens_player(
    ///             elephant_token(),
    ///             Value::TaggedCount,
    ///             PlayerFilter::IteratedPlayer,
    ///         ),
    ///     ]),
    /// ]
    /// ```
    pub fn for_each_controller_of_tagged(tag: impl Into<TagKey>, effects: Vec<Effect>) -> Self {
        use crate::effects::ForEachControllerOfTaggedEffect;
        Self::new(ForEachControllerOfTaggedEffect::new(tag.into(), effects))
    }

    /// Execute effects for each player tagged under the given tag.
    ///
    /// Sets `ctx.iterated_player` for each iteration, allowing inner effects
    /// to reference the current player via `PlayerFilter::IteratedPlayer`.
    ///
    /// # Arguments
    ///
    /// * `tag` - The tag name to iterate over (e.g., "voted_with_you")
    /// * `effects` - Effects to execute for each tagged player
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Each opponent who voted with you may scry 2
    /// Effect::for_each_tagged_player("voted_with_you", vec![
    ///     Effect::may_player(PlayerFilter::IteratedPlayer, vec![Effect::scry(2)]),
    /// ])
    /// ```
    pub fn for_each_tagged_player(tag: impl Into<TagKey>, effects: Vec<Effect>) -> Self {
        use crate::effects::ForEachTaggedPlayerEffect;
        Self::new(ForEachTaggedPlayerEffect::new(tag.into(), effects))
    }

    /// Create an effect that prompts a player to choose objects and tags them.
    ///
    /// This enables interactive sacrifice patterns and cost effects:
    /// - "Sacrifice a creature" → choose_objects + sacrifice
    /// - "Choose a creature an opponent controls" → choose_objects for later reference
    ///
    /// The chosen objects are stored under the given tag and can be referenced by
    /// subsequent effects using `ChooseSpec::tagged("tag_name")`.
    ///
    /// Example: "Sacrifice a creature" as composed effects:
    /// ```ignore
    /// vec![
    ///     Effect::choose_objects(
    ///         ObjectFilter::creature().you_control(),
    ///         1,
    ///         PlayerFilter::You,
    ///         "sacrificed",
    ///     ),
    ///     Effect::sacrifice(ChooseSpec::tagged("sacrificed")),
    /// ]
    /// ```
    pub fn choose_objects(
        filter: ObjectFilter,
        count: impl Into<ChoiceCount>,
        chooser: PlayerFilter,
        tag: impl Into<TagKey>,
    ) -> Self {
        use crate::effects::ChooseObjectsEffect;
        Self::new(ChooseObjectsEffect::new(filter, count, chooser, tag.into()))
    }

    /// Create a conditional effect based on game state.
    ///
    /// Example: "If you control a creature, draw a card. Otherwise, gain 3 life."
    /// ```ignore
    /// Effect::conditional(
    ///     Condition::YouControl(ObjectFilter::creature()),
    ///     vec![Effect::draw(1)],
    ///     vec![Effect::gain_life(3)],
    /// )
    /// ```
    pub fn conditional(condition: Condition, if_true: Vec<Effect>, if_false: Vec<Effect>) -> Self {
        use crate::effects::ConditionalEffect;
        Self::new(ConditionalEffect::new(condition, if_true, if_false))
    }

    /// Create a conditional effect with only a true branch.
    ///
    /// Example: "If you control a creature, draw a card."
    /// ```ignore
    /// Effect::conditional_only(Condition::YouControl(ObjectFilter::creature()), vec![Effect::draw(1)])
    /// ```
    pub fn conditional_only(condition: Condition, if_true: Vec<Effect>) -> Self {
        use crate::effects::ConditionalEffect;
        Self::new(ConditionalEffect::if_only(condition, if_true))
    }

    /// Create a "choose one" modal effect.
    ///
    /// Example: Modal spell with two options.
    /// ```ignore
    /// Effect::choose_one(vec![
    ///     EffectMode { description: "Draw 2 cards".to_string(), effects: vec![Effect::draw(2)] },
    ///     EffectMode { description: "Gain 5 life".to_string(), effects: vec![Effect::gain_life(5)] },
    /// ])
    /// ```
    pub fn choose_one(modes: Vec<EffectMode>) -> Self {
        use crate::effects::ChooseModeEffect;
        Self::new(ChooseModeEffect::choose_one(modes))
    }

    /// Create a "choose exactly N" modal effect.
    ///
    /// Example: "Choose two modes."
    /// ```ignore
    /// Effect::choose_exactly(2, modes)
    /// ```
    pub fn choose_exactly(count: impl Into<Value>, modes: Vec<EffectMode>) -> Self {
        use crate::effects::ChooseModeEffect;
        Self::new(ChooseModeEffect::choose_exactly(count, modes))
    }

    /// Create a "choose exactly N" modal effect that allows choosing the same mode more than once.
    pub fn choose_exactly_allow_repeated_modes(
        count: impl Into<Value>,
        modes: Vec<EffectMode>,
    ) -> Self {
        use crate::effects::ChooseModeEffect;
        Self::new(ChooseModeEffect::choose_exactly(count, modes).with_repeated_modes())
    }

    /// Create a "choose up to N" modal effect.
    ///
    /// Example: "Choose one or both modes."
    /// ```ignore
    /// Effect::choose_up_to(2, 1, modes) // min 1, max 2
    /// ```
    pub fn choose_up_to(
        max: impl Into<Value>,
        min: impl Into<Value>,
        modes: Vec<EffectMode>,
    ) -> Self {
        use crate::effects::ChooseModeEffect;
        Self::new(ChooseModeEffect::choose_up_to(max, min, modes))
    }

    /// Convenience: "You may X. If you do, Y."
    ///
    /// This is such a common pattern that it deserves a helper.
    /// Equivalent to: `WithId(0, May(effect))` + `If(0, Happened, then)`
    ///
    /// Example: "You may draw a card. If you do, discard a card."
    /// ```ignore
    /// Effect::may_if_do(
    ///     0,
    ///     Effect::draw(1),
    ///     vec![Effect::discard(1)],
    /// )
    /// ```
    pub fn may_if_do(id: u32, effect: Effect, then: Vec<Effect>) -> Vec<Effect> {
        vec![
            Self::with_id(id, Self::may_single(effect)),
            Self::if_then(EffectId(id), EffectPredicate::Happened, then),
        ]
    }

    /// Convenience: "X. If you do, Y."
    ///
    /// For non-optional effects with conditional follow-up.
    ///
    /// Example: "Sacrifice a creature. If you do, draw two cards."
    /// ```ignore
    /// Effect::do_if_do(
    ///     0,
    ///     Effect::sacrifice(ObjectFilter::creature(), 1),
    ///     vec![Effect::draw(2)],
    /// )
    /// ```
    pub fn do_if_do(id: u32, effect: Effect, then: Vec<Effect>) -> Vec<Effect> {
        vec![
            Self::with_id(id, effect),
            Self::if_then(EffectId(id), EffectPredicate::Happened, then),
        ]
    }

    // === Player/Game State Effects ===

    /// Create a "lose the game" effect for the controller.
    pub fn lose_the_game() -> Self {
        use crate::effects::LoseTheGameEffect;
        Self::new(LoseTheGameEffect::you())
    }

    /// Create a "lose the game" effect for a specific player.
    pub fn lose_the_game_player(player: PlayerFilter) -> Self {
        use crate::effects::LoseTheGameEffect;
        Self::new(LoseTheGameEffect::new(player))
    }

    /// Create a "win the game" effect for the controller.
    pub fn win_the_game() -> Self {
        use crate::effects::WinTheGameEffect;
        Self::new(WinTheGameEffect::you())
    }

    /// Create a "win the game" effect for a specific player.
    pub fn win_the_game_player(player: PlayerFilter) -> Self {
        use crate::effects::WinTheGameEffect;
        Self::new(WinTheGameEffect::new(player))
    }

    /// Create an "extra turn" effect for the controller.
    pub fn extra_turn() -> Self {
        use crate::effects::ExtraTurnEffect;
        Self::new(ExtraTurnEffect::you())
    }

    /// Create an "extra turn" effect for a specific player.
    pub fn extra_turn_player(player: PlayerFilter) -> Self {
        use crate::effects::ExtraTurnEffect;
        Self::new(ExtraTurnEffect::new(player))
    }

    /// Create a "skip turn" effect for the controller.
    pub fn skip_turn() -> Self {
        use crate::effects::SkipTurnEffect;
        Self::new(SkipTurnEffect::you())
    }

    /// Create a "skip turn" effect for a specific player.
    pub fn skip_turn_player(player: PlayerFilter) -> Self {
        use crate::effects::SkipTurnEffect;
        Self::new(SkipTurnEffect::new(player))
    }

    /// Create a "skip all combat phases of next turn" effect for the controller.
    pub fn skip_combat_phases() -> Self {
        use crate::effects::SkipCombatPhasesEffect;
        Self::new(SkipCombatPhasesEffect::you())
    }

    /// Create a "skip all combat phases of next turn" effect for a specific player.
    pub fn skip_combat_phases_player(player: PlayerFilter) -> Self {
        use crate::effects::SkipCombatPhasesEffect;
        Self::new(SkipCombatPhasesEffect::new(player))
    }

    /// Create a "skip next combat phase this turn" effect for the controller.
    pub fn skip_next_combat_phase_this_turn() -> Self {
        use crate::effects::SkipNextCombatPhaseThisTurnEffect;
        Self::new(SkipNextCombatPhaseThisTurnEffect::you())
    }

    /// Create a "skip next combat phase this turn" effect for a specific player.
    pub fn skip_next_combat_phase_this_turn_player(player: PlayerFilter) -> Self {
        use crate::effects::SkipNextCombatPhaseThisTurnEffect;
        Self::new(SkipNextCombatPhaseThisTurnEffect::new(player))
    }

    /// Create a "skip next draw step" effect for the controller.
    pub fn skip_draw_step() -> Self {
        use crate::effects::SkipDrawStepEffect;
        Self::new(SkipDrawStepEffect::you())
    }

    /// Create a "skip next draw step" effect for a specific player.
    pub fn skip_draw_step_player(player: PlayerFilter) -> Self {
        use crate::effects::SkipDrawStepEffect;
        Self::new(SkipDrawStepEffect::new(player))
    }

    // === Control Effects ===

    /// Create a "gain control" effect with a specific duration.
    pub fn gain_control_with_duration(target: ChooseSpec, duration: Until) -> Self {
        use crate::effects::GainControlEffect;
        Self::new(GainControlEffect::new(target, duration))
    }

    /// Create an "exchange control" effect between two permanents.
    pub fn exchange_control(permanent1: ChooseSpec, permanent2: ChooseSpec) -> Self {
        use crate::effects::ExchangeControlEffect;
        Self::new(ExchangeControlEffect::new(permanent1, permanent2))
    }

    /// Create a "control player" effect with explicit timing.
    pub fn control_player(
        player: PlayerFilter,
        start: crate::game_state::PlayerControlStart,
        duration: crate::game_state::PlayerControlDuration,
    ) -> Self {
        use crate::effects::ControlPlayerEffect;
        Self::new(ControlPlayerEffect::new(player, start, duration))
    }

    /// Create a "control player until end of turn" effect.
    pub fn control_player_until_end_of_turn(player: PlayerFilter) -> Self {
        use crate::effects::ControlPlayerEffect;
        Self::new(ControlPlayerEffect::until_end_of_turn(player))
    }

    /// Create a "control player during their next turn" effect.
    pub fn control_player_next_turn(player: PlayerFilter) -> Self {
        use crate::effects::ControlPlayerEffect;
        Self::new(ControlPlayerEffect::during_next_turn(player))
    }

    // === Card Manipulation Effects ===

    /// Create a "discard" effect.
    pub fn discard(count: impl Into<Value>) -> Self {
        use crate::effects::DiscardEffect;
        Self::new(DiscardEffect::you(count))
    }

    /// Create a "discard" effect for a specific player.
    pub fn discard_player(count: impl Into<Value>, player: PlayerFilter, random: bool) -> Self {
        Self::discard_player_filtered(count, player, random, None)
    }

    /// Create a "discard" effect for a specific player, optionally restricted
    /// to cards matching an object filter (for example, "discard a creature card").
    pub fn discard_player_filtered(
        count: impl Into<Value>,
        player: PlayerFilter,
        random: bool,
        card_filter: Option<crate::filter::ObjectFilter>,
    ) -> Self {
        use crate::effects::DiscardEffect;
        Self::new(DiscardEffect::new_with_filter(
            count,
            player,
            random,
            card_filter,
        ))
    }

    /// Create a "discard hand" effect.
    pub fn discard_hand() -> Self {
        use crate::effects::DiscardHandEffect;
        Self::new(DiscardHandEffect::you())
    }

    /// Create a "discard hand" effect for a specific player.
    pub fn discard_hand_player(player: PlayerFilter) -> Self {
        use crate::effects::DiscardHandEffect;
        Self::new(DiscardHandEffect::new(player))
    }

    /// Create a "scry" effect.
    pub fn scry(count: impl Into<Value>) -> Self {
        use crate::effects::ScryEffect;
        Self::new(ScryEffect::you(count))
    }

    /// Create a "scry" effect for a specific player.
    pub fn scry_player(count: impl Into<Value>, player: PlayerFilter) -> Self {
        use crate::effects::ScryEffect;
        Self::new(ScryEffect::new(count, player))
    }

    /// Create a "surveil" effect.
    pub fn surveil(count: impl Into<Value>) -> Self {
        use crate::effects::SurveilEffect;
        Self::new(SurveilEffect::you(count))
    }

    /// Create a "surveil" effect for a specific player.
    pub fn surveil_player(count: impl Into<Value>, player: PlayerFilter) -> Self {
        use crate::effects::SurveilEffect;
        Self::new(SurveilEffect::new(count, player))
    }

    /// Create a "reveal top card" effect, tagging the revealed card.
    pub fn reveal_top(player: PlayerFilter, tag: impl Into<TagKey>) -> Self {
        use crate::effects::RevealTopEffect;
        Self::new(RevealTopEffect::tagged(player, tag))
    }

    /// Create a "look at the top N cards" effect, tagging the viewed cards.
    pub fn look_at_top_cards(player: PlayerFilter, count: usize, tag: impl Into<TagKey>) -> Self {
        use crate::effects::LookAtTopCardsEffect;
        Self::new(LookAtTopCardsEffect::new(player, count, tag))
    }

    /// Create a "search library" effect.
    pub fn search_library(
        filter: ObjectFilter,
        destination: Zone,
        player: PlayerFilter,
        reveal: bool,
    ) -> Self {
        use crate::effects::SearchLibraryEffect;
        Self::new(SearchLibraryEffect::new(
            filter,
            destination,
            player,
            reveal,
        ))
    }

    /// Create a "search library to hand" effect.
    pub fn search_library_to_hand(filter: ObjectFilter, reveal: bool) -> Self {
        use crate::effects::SearchLibraryEffect;
        Self::new(SearchLibraryEffect::to_hand(
            filter,
            PlayerFilter::You,
            reveal,
        ))
    }

    /// Grant play from graveyard until end of turn.
    pub fn grant_play_from_graveyard_until_eot(player: PlayerFilter) -> Self {
        use crate::effects::GrantPlayFromGraveyardEffect;
        Self::new(GrantPlayFromGraveyardEffect::new(player))
    }

    /// Exile cards instead of going to graveyard this turn.
    pub fn exile_instead_of_graveyard_this_turn(player: PlayerFilter) -> Self {
        use crate::effects::ExileInsteadOfGraveyardEffect;
        Self::new(ExileInsteadOfGraveyardEffect::new(player))
    }

    /// Create a "may cast for miracle cost" effect.
    ///
    /// This effect is used by Miracle triggers to present the player with the choice
    /// to cast the spell for its miracle cost.
    ///
    /// Gets the card and owner from the triggering CardsDrawnEvent.
    pub fn may_cast_for_miracle_cost() -> Self {
        use crate::effects::player::MayCastForMiracleCostEffect;
        Self::new(MayCastForMiracleCostEffect::new())
    }

    // === Voting Effects ===

    /// Create a vote effect for council's dilemma and similar mechanics.
    ///
    /// Each player votes for one of the options. After all votes, effects are
    /// executed based on vote counts (once per vote).
    ///
    /// # Arguments
    ///
    /// * `options` - The vote options (e.g., "evidence" -> investigate)
    /// * `controller_extra_votes` - Mandatory extra votes for the controller
    ///
    /// # Example
    ///
    /// Tivit's council's dilemma:
    /// ```ignore
    /// Effect::vote_with_optional_extra(
    ///     vec![
    ///         VoteOption::new("evidence", vec![Effect::investigate()]),
    ///         VoteOption::new("bribery", vec![Effect::create_tokens(treasure_token(), 1)]),
    ///     ],
    ///     0, // Mandatory extra votes
    ///     1, // Optional extra votes ("you may vote an additional time")
    /// )
    /// ```
    pub fn vote(options: Vec<crate::effects::VoteOption>, controller_extra_votes: u32) -> Self {
        use crate::effects::VoteEffect;
        Self::new(VoteEffect::new(options, controller_extra_votes))
    }

    /// Create a vote effect with optional extra votes for the controller.
    pub fn vote_with_optional_extra(
        options: Vec<crate::effects::VoteOption>,
        controller_extra_votes: u32,
        controller_optional_extra_votes: u32,
    ) -> Self {
        use crate::effects::VoteEffect;
        Self::new(VoteEffect::with_optional_extra(
            options,
            controller_extra_votes,
            controller_optional_extra_votes,
        ))
    }

    /// Create a council's dilemma vote effect (controller may vote an additional time).
    ///
    /// This is a convenience method for the common council's dilemma pattern.
    pub fn councils_dilemma(options: Vec<crate::effects::VoteOption>) -> Self {
        use crate::effects::VoteEffect;
        Self::new(VoteEffect::councils_dilemma(options))
    }

    /// Create a basic vote effect (no extra votes for controller).
    pub fn vote_basic(options: Vec<crate::effects::VoteOption>) -> Self {
        use crate::effects::VoteEffect;
        Self::new(VoteEffect::basic(options))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deal_damage_effect() {
        let effect = Effect::deal_damage(3, ChooseSpec::AnyTarget);
        // Verify it has the expected target spec via the trait method
        assert!(effect.0.get_target_spec().is_some());
        assert!(matches!(
            effect.0.get_target_spec().unwrap(),
            ChooseSpec::AnyTarget
        ));
    }

    #[test]
    fn test_draw_cards_effect() {
        let effect = Effect::draw(2);
        let debug_str = format!("{:?}", effect);
        assert!(debug_str.contains("DrawCardsEffect"));
        assert!(debug_str.contains("Fixed(2)"));
    }

    #[test]
    fn test_complex_effect() {
        // "Draw cards equal to the number of creatures you control"
        let effect = Effect::target_draws(Value::creatures_you_control(), PlayerFilter::You);

        let debug_str = format!("{:?}", effect);
        assert!(debug_str.contains("DrawCardsEffect"));
        assert!(debug_str.contains("Count"));
    }

    #[test]
    fn test_conditional_effect() {
        // "If you control a creature, draw a card. Otherwise, gain 3 life."
        let effect = Effect::conditional(
            Condition::YouControl(ObjectFilter::creature()),
            vec![Effect::draw(1)],
            vec![Effect::gain_life(3)],
        );

        let debug_str = format!("{:?}", effect);
        assert!(debug_str.contains("ConditionalEffect"));
    }

    #[test]
    fn test_pump_spell() {
        // "Target creature gets +3/+3 until end of turn"
        let effect = Effect::pump(3, 3, ChooseSpec::creature(), Until::EndOfTurn);
        let debug_str = format!("{:?}", effect);
        assert!(debug_str.contains("ModifyPowerToughnessEffect"));
    }

    // === EffectResult tests ===

    #[test]
    fn test_effect_result_count() {
        let result = EffectResult::Count(3);
        assert!(result.is_success());
        assert!(!result.is_failure());
        assert!(result.something_happened());
        assert_eq!(result.as_count(), Some(3));
        assert_eq!(result.count_or_zero(), 3);
    }

    #[test]
    fn test_effect_result_count_zero() {
        let result = EffectResult::Count(0);
        assert!(result.is_success()); // Still a success (effect executed)
        assert!(!result.something_happened()); // But nothing happened
        assert_eq!(result.as_count(), Some(0));
    }

    #[test]
    fn test_effect_result_resolved() {
        let result = EffectResult::Resolved;
        assert!(result.is_success());
        assert!(result.something_happened());
        assert_eq!(result.as_count(), None);
        assert_eq!(result.count_or_zero(), 0);
    }

    #[test]
    fn test_effect_result_declined() {
        let result = EffectResult::Declined;
        assert!(result.is_failure());
        assert!(!result.something_happened());
    }

    #[test]
    fn test_effect_result_failures() {
        for result in [
            EffectResult::Declined,
            EffectResult::TargetInvalid,
            EffectResult::Prevented,
            EffectResult::Protected,
            EffectResult::Impossible,
        ] {
            assert!(result.is_failure());
            assert!(!result.something_happened());
        }
    }

    #[test]
    fn test_effect_result_replaced() {
        // Replaced is a special case - it's a success and something happened
        let result = EffectResult::Replaced;
        assert!(result.is_success());
        assert!(result.something_happened());
    }

    // === EffectPredicate tests ===

    #[test]
    fn test_predicate_succeeded() {
        let pred = EffectPredicate::Succeeded;
        assert!(pred.evaluate(&EffectResult::Count(0)));
        assert!(pred.evaluate(&EffectResult::Resolved));
        assert!(!pred.evaluate(&EffectResult::Declined));
        assert!(!pred.evaluate(&EffectResult::TargetInvalid));
    }

    #[test]
    fn test_predicate_happened() {
        let pred = EffectPredicate::Happened;
        assert!(pred.evaluate(&EffectResult::Count(1)));
        assert!(!pred.evaluate(&EffectResult::Count(0)));
        assert!(pred.evaluate(&EffectResult::Resolved));
        assert!(!pred.evaluate(&EffectResult::Declined));
    }

    #[test]
    fn test_predicate_did_not_happen() {
        let pred = EffectPredicate::DidNotHappen;
        assert!(!pred.evaluate(&EffectResult::Count(1)));
        assert!(pred.evaluate(&EffectResult::Count(0)));
        assert!(!pred.evaluate(&EffectResult::Resolved));
        assert!(pred.evaluate(&EffectResult::Declined));
    }

    #[test]
    fn test_predicate_value_comparison() {
        let pred = EffectPredicate::Value(Comparison::GreaterThan(2));
        assert!(pred.evaluate(&EffectResult::Count(3)));
        assert!(!pred.evaluate(&EffectResult::Count(2)));
        assert!(!pred.evaluate(&EffectResult::Count(1)));
        assert!(!pred.evaluate(&EffectResult::Resolved)); // Not a count
    }

    #[test]
    fn test_predicate_chosen_vs_declined() {
        let chosen = EffectPredicate::Chosen;
        let declined = EffectPredicate::WasDeclined;

        assert!(chosen.evaluate(&EffectResult::Count(1)));
        assert!(chosen.evaluate(&EffectResult::Resolved));
        assert!(!chosen.evaluate(&EffectResult::Declined));

        assert!(!declined.evaluate(&EffectResult::Count(1)));
        assert!(declined.evaluate(&EffectResult::Declined));
    }

    // === Comparison tests ===

    #[test]
    fn test_comparison_operations() {
        assert!(Comparison::GreaterThan(5).evaluate(6));
        assert!(!Comparison::GreaterThan(5).evaluate(5));

        assert!(Comparison::GreaterThanOrEqual(5).evaluate(5));
        assert!(Comparison::GreaterThanOrEqual(5).evaluate(6));
        assert!(!Comparison::GreaterThanOrEqual(5).evaluate(4));

        assert!(Comparison::Equal(5).evaluate(5));
        assert!(!Comparison::Equal(5).evaluate(4));

        assert!(Comparison::LessThan(5).evaluate(4));
        assert!(!Comparison::LessThan(5).evaluate(5));

        assert!(Comparison::LessThanOrEqual(5).evaluate(5));
        assert!(Comparison::LessThanOrEqual(5).evaluate(4));

        assert!(Comparison::NotEqual(5).evaluate(4));
        assert!(!Comparison::NotEqual(5).evaluate(5));
    }

    // === Effect composition tests ===

    #[test]
    fn test_effect_with_id() {
        let effect = Effect::with_id(0, Effect::draw(1));
        let debug_str = format!("{:?}", effect);
        assert!(debug_str.contains("WithIdEffect"));
    }

    #[test]
    fn test_effect_may() {
        let effect = Effect::may_single(Effect::draw(1));
        let debug_str = format!("{:?}", effect);
        assert!(debug_str.contains("MayEffect"));
    }

    #[test]
    fn test_effect_if_then() {
        let effect = Effect::if_then(
            EffectId(0),
            EffectPredicate::Happened,
            vec![Effect::draw(2)],
        );
        let debug_str = format!("{:?}", effect);
        assert!(debug_str.contains("IfEffect"));
    }

    #[test]
    fn test_effect_may_if_do() {
        // "You may draw a card. If you do, discard a card."
        let effects = Effect::may_if_do(0, Effect::draw(1), vec![Effect::discard(1)]);

        assert_eq!(effects.len(), 2);

        // First effect should be WithIdEffect(MayEffect)
        let debug_str_0 = format!("{:?}", &effects[0]);
        assert!(debug_str_0.contains("WithIdEffect"));
        assert!(debug_str_0.contains("MayEffect"));

        // Second effect should be IfEffect
        let debug_str_1 = format!("{:?}", &effects[1]);
        assert!(debug_str_1.contains("IfEffect"));
    }

    #[test]
    fn test_effect_do_if_do() {
        // "Sacrifice a creature. If you do, draw two cards."
        let effects = Effect::do_if_do(
            0,
            Effect::sacrifice(ObjectFilter::creature(), 1),
            vec![Effect::draw(2)],
        );

        assert_eq!(effects.len(), 2);

        // First effect should be WithIdEffect(sacrifice)
        let debug_str_0 = format!("{:?}", &effects[0]);
        assert!(debug_str_0.contains("WithIdEffect"));

        // Second effect should be IfEffect
        let debug_str_1 = format!("{:?}", &effects[1]);
        assert!(debug_str_1.contains("IfEffect"));
    }

    #[test]
    fn test_value_effect_value() {
        // "Draw cards equal to the damage dealt"
        let value = Value::EffectValue(EffectId(0));
        assert!(matches!(value, Value::EffectValue(EffectId(0))));
    }
}
