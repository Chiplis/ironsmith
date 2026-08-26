use crate::cost_model::OptionalCostRef;
use crate::{
    ActivationTiming, AnthemCountExpression, Comparison, CounterType, EffectId, EventValueSpec,
    ManaSymbol, ObjectFilter, PlayerFilter, PlayerId, StableId, StaticAbilityId, Subtype, TagKey,
    ValueComparisonOperator, Zone,
};

use crate::{
    ChooseSpec, ChooseSpecSurfaceHint, Color, ColorSet, SacrificedObjectKind,
    SourceReferenceSurface,
};

/// Selects the object whose attachments are counted by an attachment
/// relationship condition.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[expect(
    clippy::large_enum_variant,
    reason = "attachment conditions preserve typed object filters inline"
)]
pub enum AttachmentConditionHost {
    /// Count attachments on the ability's source object.
    Source,
    /// Count attachments on the permanent the Aura or Equipment source is
    /// attached to.
    SourceAttachedObject,
    /// Count attachments on each matching battlefield object, succeeding when
    /// at least one host satisfies the comparison.
    Matching(ObjectFilter),
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EffectMetricSource {
    Outcome,
    ChosenObjects,
    AffectedObjects,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EffectMetric {
    Count,
    ChosenCount,
    AffectedCount,
    LifeLost,
    LifeGained,
    DamageDealt,
    /// Total damage in excess of lethal damage dealt by the referenced
    /// effect. This is recorded as an execution fact because DamageEvent
    /// intentionally carries the applied amount, not the lethal threshold.
    ExcessDamage,
    DamagePrevented,
    FirstPower,
    FirstToughness,
    FirstManaValue,
    TotalPower,
    TotalToughness,
    TotalManaValue,
    GreatestPower,
    GreatestToughness,
    GreatestManaValue,
    ColorsAmong,
    CardTypesAmong,
    GreatestPlayerCount,
    IteratedPlayerCount,
    PlayersWithPositiveCount,
    OtherNumber,
}

/// The authored action that produced a prior-effect metric query.
///
/// This is presentation metadata for phrases such as "creatures destroyed
/// this way". Runtime identity comes from the exact producer [`EffectId`], not
/// from guessing an action from a generated tag name.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PriorEffectAction {
    Cast,
    Chosen,
    Connived,
    Countered,
    CountersPut,
    DealtDamage,
    Destroyed,
    Discarded,
    Drawn,
    Exiled,
    Goaded,
    Milled,
    PhasedOut,
    Prevented,
    PutOntoBattlefield,
    Removed,
    Returned,
    Revealed,
    Sacrificed,
    Searched,
    Shuffled,
    Tapped,
}

/// A metric over the last-known-information memory emitted by one exact
/// producer effect.
///
/// `filter` is evaluated against captured object memory rather than live game
/// objects. `player` optionally selects a per-player memory partition before
/// the filter and aggregate are applied.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
pub struct PriorEffectMetricQuery {
    pub source: EffectMetricSource,
    pub metric: EffectMetric,
    pub filter: Option<ObjectFilter>,
    pub player: Option<PlayerFilter>,
    pub action: Option<PriorEffectAction>,
    /// Authored counter kind for counter-action result surfaces. Runtime
    /// identity still comes from `effect_id`; this preserves wording such as
    /// "stun counters removed this way".
    pub counter_type: Option<CounterType>,
}

impl PriorEffectMetricQuery {
    pub fn new(source: EffectMetricSource, metric: EffectMetric) -> Self {
        Self {
            source,
            metric,
            filter: None,
            player: None,
            action: None,
            counter_type: None,
        }
    }

    pub fn with_filter(mut self, filter: ObjectFilter) -> Self {
        self.filter = Some(filter);
        self
    }

    pub fn with_player(mut self, player: PlayerFilter) -> Self {
        self.player = Some(player);
        self
    }

    pub fn with_action(mut self, action: PriorEffectAction) -> Self {
        self.action = Some(action);
        self
    }

    pub fn with_counter_type(mut self, counter_type: Option<CounterType>) -> Self {
        self.counter_type = counter_type;
        self
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ValueSurfaceHint {
    /// Preserve an unbounded choice with a minimum of one, such as
    /// "discard one or more land cards." Effects that support optional
    /// unbounded selection use this typed marker to distinguish it from
    /// "any number," whose minimum is zero.
    OneOrMoreChoice,
    WhereXIs,
    EqualTo,
    /// Preserve an authored exact-equality comparison ("is exactly N") as
    /// distinct from the semantically equivalent "is equal to N" surface.
    /// The comparison operator remains the executable source of truth.
    ExactComparison,
    /// Preserve an authored maximum-selection phrase of the form
    /// "A or B, whichever is greater." The executable value is represented
    /// with ordinary arithmetic primitives (`A + B - min(A, B)`), while this
    /// marker retains the authored extremum surface.
    WhicheverIsGreater,
    /// Preserve an authored resolution-time sampling clause on a dynamic
    /// value, such as "where X is this creature's power as this ability
    /// resolves." The effect using the value remains responsible for
    /// actually freezing it at resolution.
    AsThisAbilityResolves,
    /// Preserve an authored numeric reference to "the result" of the prior
    /// effect. Unlike an ambient trigger event amount, this value must bind to
    /// the immediately exported effect result (for example, a die roll).
    PriorEffectResult,
    /// Preserve the authored threshold idiom "had another land enter ... this
    /// turn". The underlying value remains the reusable count of lands that
    /// entered under the matching player's control, compared against two.
    AnotherLandEnteredThisTurn,
    /// Preserve an authored possessive "that player's" reference on a value.
    /// Player identity remains encoded by the value's `PlayerFilter`; this is
    /// presentation provenance only, distinct from the ordinary "their"
    /// surface for the same resolved player.
    ThatPlayerPossessive,
    /// Preserve an authored object pronoun ("them") for the recipient of a
    /// damage instruction. The typed damage target still identifies the
    /// player; this only distinguishes that surface from "that player" when
    /// both lower to the same iterated-player target.
    DamageRecipientPronoun,
    /// Preserve an authored ability noun in an object-count predicate, such
    /// as "cards with a cycling ability". The filter's typed ability marker
    /// remains the source of the actual constraint.
    ExplicitAbilityNoun,
    /// Preserve counter wording where the target precedes the equality basis:
    /// "put a number of counters on it equal to ...". This is presentation
    /// metadata only; the numeric value remains unchanged.
    EqualToAfterTarget,
    /// Preserve an explicit "then" before a counter-placement follow-up.
    /// This is presentation metadata only; the numeric value is unchanged.
    CounterFollowupThen,
    /// Preserve a sentence boundary before a counter-placement follow-up.
    /// This is presentation metadata only; the numeric value is unchanged.
    CounterFollowupSeparateSentence,
    /// Preserve a sentence boundary after counter placement when the next
    /// authored sentence grants an ability to the countered object.
    /// This is presentation metadata only; the numeric value is unchanged.
    CounterGrantSeparateSentence,
    /// Marks a counter parsed from the same battlefield-entry clause as its
    /// zone move ("with a ... counter on it"). This lets lowering distinguish
    /// an entry modifier from a genuinely subsequent counter action even when
    /// an earlier action in the sentence is coordinated with "then".
    InlineBattlefieldEntryCounter,
    /// Preserve the authored word "additional" in a battlefield-entry
    /// counter clause. This is presentation metadata only; the numeric value
    /// is unchanged.
    AdditionalEntryCounter,
    /// The extremum comparison set is implicit in the selected object filter
    /// ("with the greatest power"), so rendering should not append an
    /// explicit "among ..." clause.
    ExtremumImplicitScope,
    /// Preserve an explicit full tie clause such as "or tied for the greatest
    /// power" after an extremum predicate.
    ExtremumTiedForCharacteristic,
    /// Preserve the shortened tie clause used after an explicit comparison
    /// set ("or tied for greatest").
    ExtremumTiedShort,
    /// Preserve an authored full or short card-name subject on a static
    /// enters-with-counters clause. The counter value remains unchanged.
    SourceNameSubject,
    /// Preserve the authored indefinite commander reference in history
    /// counts ("cast a commander from the command zone") as distinct from
    /// the equally executable "your commander" surface.
    IndefiniteCommanderReference,
    ForEach,
    /// Preserve a power/toughness modifier whose duration precedes its
    /// per-object scaling clause: "until end of turn for each ...".
    /// Runtime evaluation is unchanged; this records only authored order.
    DurationBeforeForEach,
    /// Preserve an explicit prefix comparison such as "less than or equal
    /// to <value>" instead of the semantically equivalent postfix surface
    /// "<value> or less". This is presentation metadata only.
    ExplicitComparison,
    /// Preserve a masculine source possessive on a dynamic characteristic
    /// value ("his power"). The underlying value still resolves against the
    /// source object.
    MasculineSourcePossessive,
    /// Preserve a feminine source possessive on a dynamic characteristic
    /// value ("her power"). The underlying value still resolves against the
    /// source object.
    FeminineSourcePossessive,
    /// Preserve an explicit Oracle reference to "the revealed card" after
    /// reference resolution has mapped it to the reusable revealed-object
    /// tag. This is presentation metadata only; the tagged object remains
    /// the source of the characteristic value.
    RevealedCardReference,
    /// Preserve an authored characteristic of the object affected by a
    /// preceding cost or effect, such as "the power of the creature tapped
    /// this way". The underlying characteristic value still uses the exact
    /// tagged object selected by the producer.
    CharacteristicOfObjectThisWay {
        card_type: crate::CardType,
        action: PriorEffectAction,
    },
    /// Preserve Oracle's "additional card(s)" wording on a draw count.
    /// This is presentation metadata only; resolving the value still yields
    /// the same number of cards.
    AdditionalCards,
    /// Preserve an authored "that many cards" reference after the numeric
    /// value has been bound to the preceding effect's outcome.
    ThatManyCards,
    /// Preserve the generic authored "that many" surface after semantic
    /// lowering has bound it to an explicit reusable value.
    ThatMany,
    /// Preserve an authored "as many cards as ... this way" reference after
    /// the numeric value has been bound to the preceding effect's outcome.
    AsManyCardsThisWay,
    CardsDrawnThisWay,
    CardsRevealedThisWay,
    /// Preserve the authored "put into your graveyard this way" surface for
    /// a prior mill outcome. The underlying action remains a typed mill so
    /// reference resolution still binds to the exact producing effect.
    CardsPutIntoYourGraveyardThisWay,
    CardsExiledThisWay,
    CardsDiscardedThisWay,
    /// Preserve the authored result surface "creatures that died this way"
    /// while the underlying query remains bound to the exact destroy effect
    /// that produced the captured object set.
    DiedThisWay,
    /// Preserve the blocker-count basis used by a becomes-blocked trigger.
    CreaturesBlockingIt,
    /// Preserve the Scry event's authored magnitude description.
    CardsLookedAtWhileScryingThisWay,
    /// Preserve the relative ordering clause on an ordered object iteration.
    CreaturesChosenBeforeIt,
    /// A dynamic object-choice count which chooses the complete matching set
    /// while preserving the chooser's explicit order.
    ChooseAllInOrder,
    /// Preserve "an additional" on a power/toughness modifier. This is
    /// presentation metadata only; the numeric modifier is unchanged.
    AdditionalPowerToughnessModifier,
    /// Preserve an explicit reference to damage dealt when the runtime value
    /// is supplied by the triggering event's generic numeric payload.
    DamageDealt,
    /// Preserve the authored count basis for a grouped combat-damage trigger:
    /// "the number of opponents dealt damage this way." The runtime value is
    /// still supplied by the trigger's generic numeric payload.
    OpponentsDealtDamageThisWay,
    AllCardsInHand,
    PermanentsSacrificedThisWay,
    /// Preserve a counter-removal result reference which intentionally omits
    /// "this way", as in "For each counter removed, ...".
    CountersRemoved,
    CountersRemovedThisWay,
    /// Preserve the aggregate wording of counts distributed across a set of
    /// objects ("counters among creatures") rather than an explicit
    /// per-object reference ("counters on that creature").
    CountersAmong,
    /// Preserve a last-known-information counter total from the object that
    /// caused the current trigger: "the number of counters it had on it."
    /// The underlying value remains a typed CountersOn(triggering) lookup.
    TriggeringObjectCountersItHad,
    EnergyPaidThisWay,
    /// Preserve Oracle's "Repeat this process once" surface while executing
    /// the typed repeated effect body exactly twice.
    RepeatThisProcessOnce,
    ManaValueOfPermanentExiledThisWay,
    Difference,
    /// Preserve the authored subtraction connective "in excess of". The
    /// underlying value remains ordinary subtraction for runtime evaluation.
    InExcessOf,
    UpTo,
    BlightKeywordAction,
    SacrificedObject(SacrificedObjectKind),
}

/// Authored reference used after "spent to cast" in a mana-spent count.
///
/// This is presentation metadata only; all variants resolve against the
/// current spell's recorded mana payment.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ManaSpentCastReferenceSurface {
    It,
    #[default]
    ThisSpell,
    ThisCreature,
}

impl ManaSpentCastReferenceSurface {
    pub const fn text(self) -> &'static str {
        match self {
            Self::It => "it",
            Self::ThisSpell => "this spell",
            Self::ThisCreature => "this creature",
        }
    }
}

/// A count derived from immutable observations in the current turn's event
/// history.  These queries deliberately carry the same typed object/player
/// filters used by the rest of the engine: a creature which died, a spell
/// which left the stack, or a token which no longer exists must still be
/// counted from its event snapshot rather than from the current zone state.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DeathHistoryControllerSurface {
    /// Oracle placed the controller after the event:
    /// "creatures that died under your control this turn."
    #[default]
    DiedUnderControl,
    /// Oracle placed the past controller before the event:
    /// "creatures you controlled that died this turn."
    ControlledThenDied,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
pub enum TurnHistoryCount {
    /// Objects matching the filter which moved from the battlefield to a
    /// graveyard this turn.
    Died {
        filter: ObjectFilter,
        /// Printed placement of an authored controller qualification. This
        /// does not change which event snapshots the filter matches.
        controller_surface: DeathHistoryControllerSurface,
    },
    /// Objects matching the filter which entered the battlefield this turn.
    EnteredBattlefield(ObjectFilter),
    /// Tokens created under the control of matching players this turn.
    TokensCreated(PlayerFilter),
    /// Cards owned by matching players which were put into a graveyard this
    /// turn. An empty `from` list means "from anywhere".
    PutIntoGraveyard {
        owner: PlayerFilter,
        from: Vec<Zone>,
    },
    /// Objects matching the LKI filter which changed between the requested
    /// zones this turn. `None` on either side means any origin/destination.
    MovedZones {
        filter: ObjectFilter,
        from: Option<Zone>,
        to: Option<Zone>,
    },
    /// Permanents matching the filter sacrificed by matching players.
    Sacrificed {
        player: PlayerFilter,
        filter: ObjectFilter,
    },
    /// Counters of the requested kind put on matching objects this turn.
    CountersPutOn {
        counter_type: Option<CounterType>,
        filter: ObjectFilter,
    },
    /// Distinct creatures matching the filter a matching player attacked with
    /// this turn.
    CreaturesAttackedWith {
        player: PlayerFilter,
        filter: ObjectFilter,
    },
    /// Distinct opposing players attacked by matching players this turn.
    OpponentsAttacked(PlayerFilter),
    /// Distinct matching players who discarded one or more cards this turn.
    PlayersDiscarded(PlayerFilter),
    /// Distinct matching players who were dealt damage this turn.
    PlayersDealtDamage(PlayerFilter),
    /// Distinct matching players dealt combat damage by a matching source this
    /// turn.
    PlayersDealtCombatDamageBy {
        players: PlayerFilter,
        sources: ObjectFilter,
    },
    /// Cards discarded or cycled by matching players this turn, de-duplicated
    /// by stable object identity so cycling a card is not counted twice.
    DiscardedOrCycled(PlayerFilter),
    /// Cards cycled by matching players this turn.
    Cycled(PlayerFilter),
    /// Matching players who lost life this turn.
    PlayersLostLife(PlayerFilter),
    /// Lands which were untapped under a matching player's control at the
    /// beginning of the current turn. This is a frozen turn-boundary value:
    /// tapping or changing control of a land later in the turn does not alter
    /// the count.
    UntappedLandsAtTurnStart(PlayerFilter),
    /// The number of times matching players descended this turn. Descend is
    /// evaluated from zone-change LKI because the permanent card may no longer
    /// be in the graveyard when this value resolves.
    Descended(PlayerFilter),
    /// The total damage dealt to the source object this turn. Stable object
    /// identity keeps the count valid after the source changes zones.
    DamageDealtToSource,
    /// Spells matching the filter cast by matching players this turn.  The
    /// origin switch supports Paradox-style "from anywhere other than your
    /// hand" counts without pretending origin is a current-zone property.
    SpellsCast {
        player: PlayerFilter,
        filter: ObjectFilter,
        from_zone: Option<Zone>,
        from_outside_hand: bool,
        exclude_source: bool,
        /// Only count casts which occurred before the spell-cast event that
        /// triggered the currently resolving ability. This is an event-order
        /// boundary, not `total - 1`: spells cast in response to the trigger
        /// must not be included.
        before_triggering_spell: bool,
    },
    /// Colors among matching permanents currently controlled by the player and
    /// spells that player cast this turn.
    ColorsAmongPermanentsAndSpellsCast(PlayerFilter),
}

impl TurnHistoryCount {
    pub fn died(filter: ObjectFilter) -> Self {
        Self::Died {
            filter,
            controller_surface: DeathHistoryControllerSurface::default(),
        }
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    SurfaceHinted {
        value: Box<Value>,
        hints: Vec<ValueSurfaceHint>,
    },
    Fixed(i32),
    Add(Box<Value>, Box<Value>),
    X,
    XTimes(i32),
    Scaled(Box<Value>, i32),
    DividedRoundedDown(Box<Value>, i32),
    HalfRoundedDown(Box<Value>),
    Count(ObjectFilter),
    CountScaled(ObjectFilter, i32),
    GreatestCount(ObjectFilter),
    /// The largest cohort of matching creatures that share one creature type.
    /// A creature with multiple creature types contributes once to each of its
    /// type cohorts; the value is the size of the largest cohort, not the sum.
    GreatestSharedCreatureTypeCount(ObjectFilter),
    TotalPower(ObjectFilter),
    TotalToughness(ObjectFilter),
    TotalManaValue(ObjectFilter),
    GreatestPower(ObjectFilter),
    GreatestToughness(ObjectFilter),
    GreatestManaValue(ObjectFilter),
    LeastPower(ObjectFilter),
    LeastToughness(ObjectFilter),
    LeastManaValue(ObjectFilter),
    Min(Box<Value>, Box<Value>),
    BasicLandTypesAmong(ObjectFilter),
    CreatureTypesAmong(ObjectFilter),
    CardTypesAmong(ObjectFilter),
    StaticAbilitiesAmong {
        filter: ObjectFilter,
        abilities: Vec<StaticAbilityId>,
    },
    ColorsAmong(ObjectFilter),
    /// The number of distinct two-color combinations among matching objects
    /// that are exactly two colors ("the number of different color pairs
    /// among permanents you control that are exactly two colors").
    ColorPairsAmong(ObjectFilter),
    /// The number of distinct counter types present among matching objects.
    /// Multiple counters of one type and the same type on multiple objects
    /// each contribute only one to this value.
    DistinctCounterTypesAmong(ObjectFilter),
    DistinctNames(ObjectFilter),
    DistinctPowers(ObjectFilter),
    TurnHistoryCount(TurnHistoryCount),
    CreaturesDiedThisTurn,
    CreaturesDiedThisTurnControlledBy(PlayerFilter),
    PlayersBeingAttacked,
    CountPlayers(PlayerFilter),
    /// Number of matching players whose current hand contains at least the
    /// authored number of cards.
    CountPlayersWithCardsInHandAtLeast(PlayerFilter, u32),
    /// The number of matching players whose matching-object count exceeds
    /// yours.
    PlayersWhoControlMoreThanYou {
        players: PlayerFilter,
        filter: ObjectFilter,
    },
    /// The number of players whose matching-object count exceeds yours by at
    /// least `minimum_difference`.
    PlayersWhoControlAtLeastMoreThanYou {
        players: PlayerFilter,
        filter: ObjectFilter,
        minimum_difference: u32,
    },
    PartySize(PlayerFilter),
    SourcePower,
    SourceToughness,
    PowerOf(Box<ChooseSpec>),
    ToughnessOf(Box<ChooseSpec>),
    ManaValueOf(Box<ChooseSpec>),
    /// The number of mana symbols of `color` in the referenced object's
    /// printed mana cost. A hybrid or Phyrexian pip containing that color is
    /// one symbol, regardless of how many payment alternatives it has.
    ManaSymbolsInManaCostOf {
        spec: Box<ChooseSpec>,
        color: Color,
    },
    /// Number of occurrences of one character across the name stickers on
    /// the source object.
    NameStickerCharacterCountOnSource {
        character: char,
        surface: Option<SourceReferenceSurface>,
    },
    LifeTotal(PlayerFilter),
    LifeTotalAsTurnBegan(PlayerFilter),
    LifeTotalDifference(PlayerFilter),
    UnspentMana(PlayerFilter),
    Speed(PlayerFilter),
    StartingLifeTotal(PlayerFilter),
    HalfLifeTotalRoundedUp(PlayerFilter),
    HalfLifeTotalRoundedDown(PlayerFilter),
    HalfStartingLifeTotalRoundedUp(PlayerFilter),
    HalfStartingLifeTotalRoundedDown(PlayerFilter),
    CardsInHand(PlayerFilter),
    CardsInLibrary(PlayerFilter),
    DevotionToChosenColor(PlayerFilter),
    LifeGainedThisTurn(PlayerFilter),
    LifeLostThisTurn(PlayerFilter),
    CardsDiscardedThisTurn(PlayerFilter),
    /// Number of Attraction visit events performed by matching players during
    /// the current turn. This is history-based: an Attraction still counts if
    /// it has since left the battlefield, and repeated visits count again.
    AttractionsVisitedThisTurn(PlayerFilter),
    DamageDealtToPlayersThisTurn(PlayerFilter),
    NoncombatDamageDealtToPlayersThisTurn(PlayerFilter),
    NoncombatDamageDealtBySourcesControlledThisTurn {
        player: PlayerFilter,
        colors: Option<ColorSet>,
    },
    MaxCardsDrawnThisTurn(PlayerFilter),
    MaxDiceRolledThisTurn(PlayerFilter),
    LandsEnteredBattlefieldThisTurn(PlayerFilter),
    MaxCardsInHand(PlayerFilter),
    CardsInGraveyard(PlayerFilter),
    SpellsCastThisTurn(PlayerFilter),
    SpellsCastBeforeThisTurn(PlayerFilter),
    SpellsCastThisTurnMatching {
        player: PlayerFilter,
        filter: ObjectFilter,
        exclude_source: bool,
    },
    /// Total mana value of matching spells cast during the current turn.
    ///
    /// This deliberately reads cast history rather than the current stack so
    /// spells that have already resolved still contribute to the aggregate.
    TotalManaValueOfSpellsCastThisTurnMatching {
        player: PlayerFilter,
        filter: ObjectFilter,
        exclude_source: bool,
    },
    CommanderCastCount(PlayerFilter),
    ThisAbilityResolvedThisTurnCount,
    SourceRegeneratedThisTurnCount,
    /// Number of times the source permanent has mutated since it entered the battlefield.
    SourceMutationCount,
    DamageDealtThisTurnByTaggedSpellCast(TagKey),
    CardTypesInGraveyard(PlayerFilter),
    Devotion {
        player: PlayerFilter,
        color: Color,
    },
    ManaSpentToCastThisSpell,
    /// Mana of one specific type spent to cast this spell. Composing this
    /// with `DividedRoundedDown` models repeated-symbol groups such as
    /// "for each {U}{U} spent to cast it."
    ManaSymbolSpentToCastThisSpell {
        symbol: ManaSymbol,
        reference: ManaSpentCastReferenceSurface,
    },
    /// Mana spent to cast this spell whose producing source matched the
    /// captured last-known-information filter. The surface fields preserve
    /// whether oracle used the generic noun "source" after the filter and
    /// how it referred back to the spell being cast.
    ManaFromSourceSpentToCastThisSpell {
        source_filter: ObjectFilter,
        include_source_noun: bool,
        reference: ManaSpentCastReferenceSurface,
    },
    /// Total mana spent to cast the spell whose cast event triggered the
    /// currently resolving ability.
    ManaSpentToCastTriggeringObject,
    ColorsOfManaSpentToCastThisSpell,
    EffectValue(EffectId),
    EffectValueOffset(EffectId, i32),
    EffectMetric {
        effect_id: EffectId,
        source: EffectMetricSource,
        metric: EffectMetric,
    },
    EffectMetricOffset {
        effect_id: EffectId,
        source: EffectMetricSource,
        metric: EffectMetric,
        offset: i32,
    },
    PendingEffectMetric {
        source: EffectMetricSource,
        metric: EffectMetric,
    },
    PendingEffectMetricOffset {
        source: EffectMetricSource,
        metric: EffectMetric,
        offset: i32,
    },
    /// A filtered metric bound to one exact prior producer effect.
    PriorEffectMetric {
        effect_id: EffectId,
        query: PriorEffectMetricQuery,
    },
    /// Parse-time form of [`Value::PriorEffectMetric`]. Reference resolution
    /// binds it to the nearest compatible memory-producing effect.
    PendingPriorEffectMetric(PriorEffectMetricQuery),
    EventValue(EventValueSpec),
    EventValueOffset(EventValueSpec, i32),
    WasKicked,
    WasBoughtBack,
    WasEntwined,
    WasPaid(usize),
    WasPaidLabel(OptionalCostRef),
    TimesPaid(usize),
    TimesPaidLabel(OptionalCostRef),
    KickCount,
    MagicGamesLostToOpponentsSinceLastWin,
    DraftNotedHighestNumber {
        card_name: String,
    },
    LastNotedLifeTotal,
    PlayerCounters(PlayerFilter, CounterType),
    CountersOnSource(CounterType),
    CountersOn(Box<ChooseSpec>, Option<CounterType>),
    TaggedCount,
    VoteCount(String),
    PlayerVoteCount(PlayerFilter),
}

impl Value {
    pub fn fixed(n: i32) -> Self {
        Self::Fixed(n)
    }

    pub fn creatures_you_control() -> Self {
        Self::Count(ObjectFilter::creature().you_control())
    }

    /// The nonnegative difference between two dynamic values, represented in
    /// terms of the existing composable arithmetic nodes so every value
    /// resolver can evaluate it without a bespoke runtime branch.
    pub fn absolute_difference(left: Self, right: Self) -> Self {
        let forward = Self::Add(
            Box::new(left.clone()),
            Box::new(Self::Scaled(Box::new(right.clone()), -1)),
        );
        let reverse = Self::Add(Box::new(right), Box::new(Self::Scaled(Box::new(left), -1)));
        Self::Scaled(
            Box::new(Self::Min(Box::new(forward), Box::new(reverse))),
            -1,
        )
    }

    pub fn counters_on_source_reference(
        counter_type: Option<CounterType>,
        surface: Option<SourceReferenceSurface>,
    ) -> Self {
        if let Some(surface) = surface {
            return Self::CountersOn(
                Box::new(
                    ChooseSpec::Source
                        .with_surface_hint(ChooseSpecSurfaceHint::SourceReference(surface)),
                ),
                counter_type,
            );
        }

        match counter_type {
            Some(counter_type) => Self::CountersOnSource(counter_type),
            None => Self::CountersOn(Box::new(ChooseSpec::Source), None),
        }
    }

    pub fn with_surface_hint(self, hint: ValueSurfaceHint) -> Self {
        self.with_surface_hints([hint])
    }

    pub fn with_surface_hints(self, hints: impl IntoIterator<Item = ValueSurfaceHint>) -> Self {
        let mut hints_to_add: Vec<ValueSurfaceHint> = hints.into_iter().collect();
        if hints_to_add.is_empty() {
            return self;
        }

        match self {
            Value::SurfaceHinted { value, mut hints } => {
                for hint in hints_to_add.drain(..) {
                    hints.push(hint);
                }
                Value::SurfaceHinted { value, hints }
            }
            value => Value::SurfaceHinted {
                value: Box::new(value),
                hints: hints_to_add,
            },
        }
    }

    pub fn surface_hints(&self) -> &[ValueSurfaceHint] {
        match self {
            Value::SurfaceHinted { hints, .. } => hints,
            _ => &[],
        }
    }

    pub fn has_surface_hint(&self, hint: ValueSurfaceHint) -> bool {
        self.surface_hints().contains(&hint)
    }

    pub fn unhinted(&self) -> &Value {
        match self {
            Value::SurfaceHinted { value, .. } => value.unhinted(),
            value => value,
        }
    }

    pub fn into_unhinted(self) -> Value {
        match self {
            Value::SurfaceHinted { value, .. } => value.into_unhinted(),
            value => value,
        }
    }

    pub fn without_surface_hint(self, hint_to_remove: ValueSurfaceHint) -> Value {
        match self {
            Value::SurfaceHinted { value, hints } => {
                let hints = hints
                    .into_iter()
                    .filter(|hint| *hint != hint_to_remove)
                    .collect::<Vec<_>>();
                if hints.is_empty() {
                    value.without_surface_hint(hint_to_remove)
                } else {
                    Value::SurfaceHinted {
                        value: Box::new(value.without_surface_hint(hint_to_remove)),
                        hints,
                    }
                }
            }
            value => value,
        }
    }
}

impl From<i32> for Value {
    fn from(n: i32) -> Self {
        Self::Fixed(n)
    }
}

impl From<u32> for Value {
    fn from(n: u32) -> Self {
        Self::Fixed(n as i32)
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
pub enum Restriction {
    AdditionalLandPlays(PlayerFilter, u32),
    /// A lasting player rule that removes the cleanup-step hand-size limit.
    ///
    /// This lives in the restriction/rule family because resolving effects
    /// can establish it independently of a battlefield static ability.
    NoMaximumHandSize(PlayerFilter),
    GainLife(PlayerFilter),
    SearchLibraries(PlayerFilter),
    CastSpellsMatching(PlayerFilter, ObjectFilter),
    CastSpellsOnlyAsSorcery(PlayerFilter),
    ActivateNonManaAbilities(PlayerFilter),
    ActivateAbilitiesOf(ObjectFilter),
    ActivateTapAbilitiesOf(ObjectFilter),
    ActivateNonManaAbilitiesOf(ObjectFilter),
    CastMoreThanOneSpellEachTurn(PlayerFilter, ObjectFilter),
    DrawCards(PlayerFilter),
    DrawExtraCards(PlayerFilter),
    PoisonCounters(PlayerFilter),
    LoseLife(PlayerFilter),
    DamageCauseLifeLoss(PlayerFilter),
    ChangeLifeTotal(PlayerFilter),
    LoseGame(PlayerFilter),
    WinGame(PlayerFilter),
    BecomeMonarch(PlayerFilter),
    /// "[Players/You] don't lose unspent [color] mana as steps and phases end."
    /// A color of `None` retains the player's whole mana pool.
    LoseUnspentMana(PlayerFilter, Option<crate::color::Color>),
    PreventDamage,
    Attack(ObjectFilter),
    AttackPlayerOrPlaneswalkersControlledBy {
        attackers: ObjectFilter,
        player: PlayerFilter,
    },
    /// "can't attack you" — bans attacking the player directly while leaving
    /// their planeswalkers and battles attackable (CR 508.1 target choice).
    AttackPlayer {
        attackers: ObjectFilter,
        player: PlayerFilter,
    },
    /// A resolving effect's temporary tax on attacking its controller or a
    /// planeswalker that player controls. Unlike a battlefield static
    /// ability, this restriction remains active for its stated duration if
    /// the source leaves the battlefield. The bool records whether the
    /// authored text extends the tax to "or planeswalkers you control".
    AttackYouUnlessControllerPaysPerAttacker(u32, bool),
    AttackAlone(ObjectFilter),
    Block(ObjectFilter),
    BlockSpecificAttacker {
        blockers: ObjectFilter,
        attacker: ObjectFilter,
    },
    MustBlockSpecificAttacker {
        blockers: ObjectFilter,
        attacker: ObjectFilter,
    },
    MustBeBlocked(ObjectFilter),
    BlockAlone(ObjectFilter),
    Untap(ObjectFilter),
    BeBlocked(ObjectFilter),
    BeDestroyed(ObjectFilter),
    BeRegenerated(ObjectFilter),
    BeSacrificed(ObjectFilter),
    HaveCountersPlaced(ObjectFilter),
    BeTargeted(ObjectFilter),
    BeTargetedFrom(ObjectFilter, ObjectFilter),
    BeTargetedPlayer(PlayerFilter),
    BeTargetedPlayerFrom(PlayerFilter, ObjectFilter),
    BeCountered(ObjectFilter),
    Transform(ObjectFilter),
    PhaseOut(ObjectFilter),
    PhaseIn(ObjectFilter),
    AttackOrBlock(ObjectFilter),
    AttackOrBlockAlone(ObjectFilter),
}

/// How mana may be spent relative to its produced type.
///
/// "Any color" permits mana to satisfy any colored symbol, but does not let
/// colored mana satisfy a colorless `{C}` symbol. "Any type" includes
/// colorless, so it permits either conversion.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum ManaSpendMode {
    #[default]
    Normal,
    AnyColor,
    AnyType,
}

impl ManaSpendMode {
    pub fn allows_any_color(self) -> bool {
        matches!(self, Self::AnyColor | Self::AnyType)
    }

    pub fn allows_any_type(self) -> bool {
        self == Self::AnyType
    }

    pub fn combine(self, other: Self) -> Self {
        self.max(other)
    }
}

impl From<bool> for ManaSpendMode {
    fn from(allow_any_color: bool) -> Self {
        if allow_any_color {
            Self::AnyColor
        } else {
            Self::Normal
        }
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
pub struct ManaSpendPermission {
    pub player: PlayerFilter,
    pub scope: ManaSpendScope,
    pub mode: ManaSpendMode,
    pub mana_source_filter: Option<ObjectFilter>,
    pub any_color_mana_symbol: Option<ManaSymbol>,
    pub other_mana_only_as_colorless: bool,
}

impl ManaSpendPermission {
    pub fn any_color(player: PlayerFilter) -> Self {
        Self {
            player,
            scope: ManaSpendScope::AllCosts,
            mode: ManaSpendMode::AnyColor,
            mana_source_filter: None,
            any_color_mana_symbol: None,
            other_mana_only_as_colorless: false,
        }
    }

    pub fn mana_symbol_as_any_color_other_as_colorless(
        player: PlayerFilter,
        symbol: ManaSymbol,
    ) -> Self {
        Self {
            player,
            scope: ManaSpendScope::AllCosts,
            mode: ManaSpendMode::Normal,
            mana_source_filter: None,
            any_color_mana_symbol: Some(symbol),
            other_mana_only_as_colorless: true,
        }
    }

    pub fn any_color_for_activation(player: PlayerFilter, filter: ObjectFilter) -> Self {
        Self {
            player,
            scope: ManaSpendScope::ActivationCostsOf(filter),
            mode: ManaSpendMode::AnyColor,
            mana_source_filter: None,
            any_color_mana_symbol: None,
            other_mana_only_as_colorless: false,
        }
    }

    pub fn any_color_for_casting_stable_ids(
        player: PlayerFilter,
        stable_ids: Vec<StableId>,
    ) -> Self {
        Self {
            player,
            scope: ManaSpendScope::CastingSpellsWithStableIds(stable_ids),
            mode: ManaSpendMode::AnyColor,
            mana_source_filter: None,
            any_color_mana_symbol: None,
            other_mana_only_as_colorless: false,
        }
    }

    pub fn any_color_for_casting_matching(player: PlayerFilter, filter: ObjectFilter) -> Self {
        Self {
            player,
            scope: ManaSpendScope::CastingSpellsMatching(filter),
            mode: ManaSpendMode::AnyColor,
            mana_source_filter: None,
            any_color_mana_symbol: None,
            other_mana_only_as_colorless: false,
        }
    }

    pub fn any_color_from_sources_for_casting_matching(
        player: PlayerFilter,
        filter: ObjectFilter,
        mana_source_filter: ObjectFilter,
    ) -> Self {
        Self {
            player,
            scope: ManaSpendScope::CastingSpellsMatching(filter),
            mode: ManaSpendMode::AnyColor,
            mana_source_filter: Some(mana_source_filter),
            any_color_mana_symbol: None,
            other_mana_only_as_colorless: false,
        }
    }

    pub fn with_mana_source_filter(mut self, filter: ObjectFilter) -> Self {
        self.mana_source_filter = Some(filter);
        self
    }

    pub fn any_type_for_casting_stable_ids(
        player: PlayerFilter,
        stable_ids: Vec<StableId>,
    ) -> Self {
        Self {
            player,
            scope: ManaSpendScope::CastingSpellsWithStableIds(stable_ids),
            mode: ManaSpendMode::AnyType,
            mana_source_filter: None,
            any_color_mana_symbol: None,
            other_mana_only_as_colorless: false,
        }
    }

    pub fn any_type_for_casting_matching(player: PlayerFilter, filter: ObjectFilter) -> Self {
        Self {
            player,
            scope: ManaSpendScope::CastingSpellsMatching(filter),
            mode: ManaSpendMode::AnyType,
            mana_source_filter: None,
            any_color_mana_symbol: None,
            other_mana_only_as_colorless: false,
        }
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
pub enum ManaSpendScope {
    AllCosts,
    ActivationCostsOf(ObjectFilter),
    CastingSpellsWithStableIds(Vec<StableId>),
    CastingSpellsMatching(ObjectFilter),
}

impl Restriction {
    pub fn additional_land_plays(filter: PlayerFilter, count: u32) -> Self {
        Self::AdditionalLandPlays(filter, count)
    }

    pub fn no_maximum_hand_size(filter: PlayerFilter) -> Self {
        Self::NoMaximumHandSize(filter)
    }

    pub fn gain_life(filter: PlayerFilter) -> Self {
        Self::GainLife(filter)
    }

    pub fn lose_unspent_mana(filter: PlayerFilter, color: Option<crate::color::Color>) -> Self {
        Self::LoseUnspentMana(filter, color)
    }

    pub fn search_libraries(filter: PlayerFilter) -> Self {
        Self::SearchLibraries(filter)
    }

    pub fn cast_spells(filter: PlayerFilter) -> Self {
        Self::cast_spells_matching(filter, ObjectFilter::default())
    }

    pub fn activate_non_mana_abilities(filter: PlayerFilter) -> Self {
        Self::ActivateNonManaAbilities(filter)
    }

    pub fn activate_abilities_of(filter: ObjectFilter) -> Self {
        Self::ActivateAbilitiesOf(filter)
    }

    pub fn activate_tap_abilities_of(filter: ObjectFilter) -> Self {
        Self::ActivateTapAbilitiesOf(filter)
    }

    pub fn activate_non_mana_abilities_of(filter: ObjectFilter) -> Self {
        Self::ActivateNonManaAbilitiesOf(filter)
    }

    pub fn cast_spells_matching(filter: PlayerFilter, spell_filter: ObjectFilter) -> Self {
        Self::CastSpellsMatching(filter, spell_filter)
    }

    pub fn cast_spells_only_as_sorcery(filter: PlayerFilter) -> Self {
        Self::CastSpellsOnlyAsSorcery(filter)
    }

    pub fn cast_creature_spells(filter: PlayerFilter) -> Self {
        Self::cast_spells_matching(
            filter,
            ObjectFilter::default().with_type(crate::CardType::Creature),
        )
    }

    pub fn cast_more_than_one_spell_each_turn_matching(
        filter: PlayerFilter,
        spell_filter: ObjectFilter,
    ) -> Self {
        Self::CastMoreThanOneSpellEachTurn(filter, spell_filter)
    }

    pub fn cast_more_than_one_spell_each_turn(filter: PlayerFilter) -> Self {
        Self::cast_more_than_one_spell_each_turn_matching(filter, ObjectFilter::default())
    }

    pub fn cast_more_than_one_noncreature_spell_each_turn(filter: PlayerFilter) -> Self {
        Self::cast_more_than_one_spell_each_turn_matching(
            filter,
            ObjectFilter::default().without_type(crate::CardType::Creature),
        )
    }

    pub fn cast_more_than_one_nonartifact_spell_each_turn(filter: PlayerFilter) -> Self {
        Self::cast_more_than_one_spell_each_turn_matching(
            filter,
            ObjectFilter::default().without_type(crate::CardType::Artifact),
        )
    }

    pub fn cast_more_than_one_nonphyrexian_spell_each_turn(filter: PlayerFilter) -> Self {
        Self::cast_more_than_one_spell_each_turn_matching(
            filter,
            ObjectFilter::default().without_subtype(crate::Subtype::Phyrexian),
        )
    }

    pub fn draw_cards(filter: PlayerFilter) -> Self {
        Self::DrawCards(filter)
    }

    pub fn draw_extra_cards(filter: PlayerFilter) -> Self {
        Self::DrawExtraCards(filter)
    }

    pub fn poison_counters(filter: PlayerFilter) -> Self {
        Self::PoisonCounters(filter)
    }

    pub fn lose_life(filter: PlayerFilter) -> Self {
        Self::LoseLife(filter)
    }

    pub fn damage_cause_life_loss(filter: PlayerFilter) -> Self {
        Self::DamageCauseLifeLoss(filter)
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

    pub fn become_monarch(filter: PlayerFilter) -> Self {
        Self::BecomeMonarch(filter)
    }

    pub fn prevent_damage() -> Self {
        Self::PreventDamage
    }

    pub fn attack(filter: ObjectFilter) -> Self {
        Self::Attack(filter)
    }

    pub fn attack_player_or_planeswalkers_controlled_by(
        attackers: ObjectFilter,
        player: PlayerFilter,
    ) -> Self {
        Self::AttackPlayerOrPlaneswalkersControlledBy { attackers, player }
    }

    pub fn attack_player(attackers: ObjectFilter, player: PlayerFilter) -> Self {
        Self::AttackPlayer { attackers, player }
    }

    pub fn attack_you_unless_controller_pays_per_attacker(
        generic_mana: u32,
        covers_planeswalkers: bool,
    ) -> Self {
        Self::AttackYouUnlessControllerPaysPerAttacker(generic_mana, covers_planeswalkers)
    }

    pub fn attack_alone(filter: ObjectFilter) -> Self {
        Self::AttackAlone(filter)
    }

    pub fn block(filter: ObjectFilter) -> Self {
        Self::Block(filter)
    }

    pub fn block_specific_attacker(blockers: ObjectFilter, attacker: ObjectFilter) -> Self {
        Self::BlockSpecificAttacker { blockers, attacker }
    }

    pub fn must_block_specific_attacker(blockers: ObjectFilter, attacker: ObjectFilter) -> Self {
        Self::MustBlockSpecificAttacker { blockers, attacker }
    }

    pub fn must_be_blocked(filter: ObjectFilter) -> Self {
        Self::MustBeBlocked(filter)
    }

    pub fn block_alone(filter: ObjectFilter) -> Self {
        Self::BlockAlone(filter)
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

    pub fn be_regenerated(filter: ObjectFilter) -> Self {
        Self::BeRegenerated(filter)
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

    pub fn be_targeted_from(filter: ObjectFilter, source_filter: ObjectFilter) -> Self {
        Self::BeTargetedFrom(filter, source_filter)
    }

    pub fn be_targeted_player(filter: PlayerFilter) -> Self {
        Self::BeTargetedPlayer(filter)
    }

    pub fn be_targeted_player_from(player: PlayerFilter, source_filter: ObjectFilter) -> Self {
        Self::BeTargetedPlayerFrom(player, source_filter)
    }

    pub fn be_countered(filter: ObjectFilter) -> Self {
        Self::BeCountered(filter)
    }

    pub fn transform(filter: ObjectFilter) -> Self {
        Self::Transform(filter)
    }

    pub fn phase_out(filter: ObjectFilter) -> Self {
        Self::PhaseOut(filter)
    }

    pub fn phase_in(filter: ObjectFilter) -> Self {
        Self::PhaseIn(filter)
    }

    pub fn attack_or_block(filter: ObjectFilter) -> Self {
        Self::AttackOrBlock(filter)
    }

    pub fn attack_or_block_alone(filter: ObjectFilter) -> Self {
        Self::AttackOrBlockAlone(filter)
    }
}

/// Oracle-facing wording for a typed counter threshold on the source.
///
/// This does not affect condition evaluation. It preserves whether the text
/// described the source as having counters or used an existential
/// "there are ... counters on ..." clause, including that clause's source
/// reference.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum SourceCounterThresholdSurface {
    #[default]
    SourceHas,
    /// The source text explicitly used "one or more ... counters" rather
    /// than the semantically equivalent indefinite singular "a ... counter".
    SourceHasOneOrMore,
    ThereAreOn(SourceReferenceSurface),
}

/// Oracle-facing word order for a permanent that left the battlefield after
/// being controlled by the condition's controller.
///
/// Both variants have identical event-history semantics. This only preserves
/// whether Oracle used "left ... under your control" or "you controlled left".
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PermanentLeftBattlefieldControlSurface {
    #[default]
    LeftUnderYourControl,
    YouControlledLeft,
}

/// Typed predicates that depend on a triggering event, the current combat, or
/// the current/immediately previous turn's event history. These are grouped
/// under one condition family so intervening-if clauses can retain actor,
/// origin-zone, source-reference, and grouped-event semantics without falling
/// back to untyped text.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[expect(
    clippy::large_enum_variant,
    reason = "turn-history conditions preserve typed object filters inline"
)]
pub enum TurnHistoryCondition {
    SpellsCastLastTurnAtLeast(u32),
    SourceCrewedByAtLeast {
        count: u32,
        filter: ObjectFilter,
    },
    SourceWasCast {
        surface: SourceReferenceSurface,
    },
    SourceWasCastByController {
        surface: SourceReferenceSurface,
    },
    SourceWasKicked {
        surface: SourceReferenceSurface,
    },
    SourceEnteredBattlefieldThisTurn {
        surface: SourceReferenceSurface,
    },
    SourceAttackedThisTurn {
        surface: SourceReferenceSurface,
    },
    TriggeringObjectEnlistedThisCombat,
    TriggeringObjectWasCast,
    TriggeringObjectWasCastFromZone(Zone),
    PlayerPlayedLandThisTurn(PlayerFilter),
    TriggeringObjectDied,
    PlayerPlayedCardFromZoneThisTurn {
        player: PlayerFilter,
        zone: Zone,
    },
    PlayerCastSpellFromZoneThisTurn {
        player: PlayerFilter,
        zone: Zone,
    },
    PlayerActivatedAbilityOfCardInZoneThisTurn {
        player: PlayerFilter,
        zone: Zone,
    },
    PlayerVisitedAttractionThisTurn(PlayerFilter),
    TriggeringPlayerAttackedControllerLastTurn,
    PlayerLostLifeLastTurn(PlayerFilter),
    TriggeringPlayersTurn {
        definite_player: bool,
    },
    ControllerTeamGainedLifeThisTurn,
    TriggeringObjectsNoneWereCastOrNoManaSpent,
    ManaFromSourceSpentOnTriggeringAction {
        source_filter: ObjectFilter,
    },
    AllPlayersLifeAtMost(i32),
    AnotherOpponentControlsPotentialTarget {
        filter: ObjectFilter,
    },
    TriggeringAttackerBlockers {
        required: ObjectFilter,
        required_count: u32,
        prohibited: ObjectFilter,
    },
    TriggeringAbilityIsManaAbility,
}

/// Selects which captured representation of a tagged object is used when a
/// player's relationship to that object is tested.
///
/// Most "this way" predicates inspect the current object when it still exists
/// and fall back to its captured snapshot after it ceases to exist. Past-tense
/// control predicates such as "if you controlled that permanent" must instead
/// inspect the snapshot even when the moved object still exists in its new
/// zone.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TaggedObjectMatchMode {
    #[default]
    CurrentOrLastKnown,
    LastKnown,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
pub enum Condition {
    YouControl(ObjectFilter),
    OpponentControls(ObjectFilter),
    PlayerControls {
        player: PlayerFilter,
        filter: ObjectFilter,
    },
    PlayerHasAtLeast {
        player: PlayerFilter,
        filter: ObjectFilter,
        count: u32,
    },
    PlayerControlsBasicLandTypesAmongLandsOrMore {
        player: PlayerFilter,
        count: u32,
    },
    PlayerControlsExactly {
        player: PlayerFilter,
        filter: ObjectFilter,
        count: u32,
    },
    PlayerHasAtLeastWithDifferentPowers {
        player: PlayerFilter,
        filter: ObjectFilter,
        count: u32,
    },
    PlayerControlsMost {
        player: PlayerFilter,
        filter: ObjectFilter,
    },
    PlayerControlsMoreThanEachOtherPlayer {
        player: PlayerFilter,
        filter: ObjectFilter,
    },
    PlayerControlsMoreThanYou {
        player: PlayerFilter,
        filter: ObjectFilter,
    },
    AnOpponentControlsMoreThanPlayer {
        player: PlayerFilter,
        filter: ObjectFilter,
    },
    AnOpponentHasFewerThanPlayer {
        player: PlayerFilter,
        filter: ObjectFilter,
    },
    PlayerLifeAtMostHalfStartingLifeTotal {
        player: PlayerFilter,
    },
    PlayerLifeLessThanHalfStartingLifeTotal {
        player: PlayerFilter,
    },
    PlayerHasLessLifeThanYou {
        player: PlayerFilter,
    },
    PlayerHasMoreLifeThanYou {
        player: PlayerFilter,
    },
    PlayerHasNoOpponentWithMoreLifeThan {
        player: PlayerFilter,
    },
    PlayerHasMoreLifeThanEachOtherPlayer {
        player: PlayerFilter,
    },
    PlayerIsMonarch {
        player: PlayerFilter,
    },
    PlayerHasInitiative {
        player: PlayerFilter,
    },
    PlayerHasCitysBlessing {
        player: PlayerFilter,
    },
    SourceIsRingBearer {
        player: PlayerFilter,
    },
    PlayerRingTemptedThisGameOrMore {
        player: PlayerFilter,
        count: u32,
    },
    PlayerCommittedCrimeThisTurn {
        player: PlayerFilter,
    },
    PlayerRolledResultThisTurn {
        player: PlayerFilter,
        result: u32,
    },
    PlayerCompletedDungeon {
        player: PlayerFilter,
        dungeon_name: Option<String>,
    },
    /// A pregame draft record contains a card matching `filter` that the
    /// indicated player removed from the draft with the named card group.
    ///
    /// The filter remains the ordinary characteristic/ability filter so
    /// mechanics can ask about any reusable printed quality rather than a
    /// card-specific flag.
    PlayerRemovedDraftCardMatching {
        player: PlayerFilter,
        filter: ObjectFilter,
        with_cards_named: String,
    },
    LifeTotalOrLess(i32),
    LifeTotalOrGreater(i32),
    CardsInHandOrMore(i32),
    PlayerCardsInHandOrMore {
        player: PlayerFilter,
        count: i32,
    },
    PlayerCardsInHandOrFewer {
        player: PlayerFilter,
        count: i32,
    },
    PlayerCardsInHandAtTurnStartOrMore {
        player: PlayerFilter,
        count: i32,
    },
    PlayerCardsInHandAtTurnStartOrFewer {
        player: PlayerFilter,
        count: i32,
    },
    PlayerHasMoreCardsInHandThanYou {
        player: PlayerFilter,
    },
    PlayerHasMoreCardsInHandThanEachOtherPlayer {
        player: PlayerFilter,
    },
    PlayerHasPoisonCountersOrMore {
        player: PlayerFilter,
        count: u32,
    },
    PlayerHasCountersOrMore {
        player: PlayerFilter,
        counter_type: CounterType,
        count: u32,
    },
    YouHaveCardInHandMatching(ObjectFilter),
    YourTurn,
    /// The turn currently being played was created as an extra turn rather
    /// than reached through the normal turn order.
    CurrentTurnIsExtra,
    YourFirstTurnsOfTheGameOrFewer(u32),
    CreatureDiedThisTurn,
    CreatureDiedThisTurnOrMore(u32),
    CreatureDealtDamageBySourceDiedThisTurn {
        victim: ObjectFilter,
        damager: crate::DamagedBySource,
        count: u32,
    },
    CreatureCardPutIntoYourGraveyardThisTurn,
    CastSpellThisTurn,
    PlayerCastSpellsThisTurnOrMore {
        player: PlayerFilter,
        count: u32,
    },
    AttackedThisTurn,
    /// "you attacked with N or more creatures this turn"
    AttackedWithNOrMoreCreaturesThisTurn(u32),
    OpponentLostLifeThisTurn,
    AnyPlayerLostLifeThisTurnOrMore {
        count: u32,
    },
    OpponentWasDealtDamageThisTurn,
    PermanentLeftBattlefieldThisTurn,
    NonlandPermanentLeftBattlefieldThisTurn,
    SpellWasWarpedThisTurn,
    PermanentLeftBattlefieldUnderYourControlThisTurn {
        surface: PermanentLeftBattlefieldControlSurface,
    },
    ObjectEnteredBattlefieldThisTurn(ObjectFilter),
    ObjectEnteredBattlefieldLastTurn(ObjectFilter),
    ObjectPutIntoGraveyardFromBattlefieldThisTurn(ObjectFilter),
    SourceWasCast,
    ThisSpellWasCastAtSorceryTiming,
    ThisSpellEscaped,
    ThisSpellWasCastFromZone(Zone),
    ThisSpellWasCastFromNonHand,
    PlayerTappedLandForManaThisTurn {
        player: PlayerFilter,
    },
    PlayerGainedLifeThisTurnOrMore {
        player: PlayerFilter,
        count: u32,
    },
    PlayerHadLandEnterBattlefieldThisTurn {
        player: PlayerFilter,
    },
    /// The player descended this turn: one or more permanent cards were put
    /// into that player's graveyard from anywhere this turn.
    PlayerDescendedThisTurn {
        player: PlayerFilter,
    },
    ValueComparison {
        left: Value,
        operator: ValueComparisonOperator,
        right: Value,
    },
    /// The resolved integer value is a prime number. This stays value-based
    /// so control counts, hand sizes, counters, and future numeric sources can
    /// share one rules-correct predicate.
    ValueIsPrime(Value),
    NoSpellsWereCastLastTurn,
    SpellsWereCastLastTurnOrMore(u32),
    PlayerHasCardTypesInGraveyardOrMore {
        player: PlayerFilter,
        count: u32,
    },
    TargetIsTapped,
    TargetIsAttacking,
    TargetIsBlocked,
    TargetWasKicked,
    ThisSpellWasKicked,
    ThisSpellPaidLabel(OptionalCostRef),
    YouHaveFullParty,
    TargetSpellCastOrderThisTurn(u32),
    TargetSpellControllerIsPoisoned,
    TargetSpellManaSpentToCastAtLeast {
        amount: u32,
        symbol: Option<ManaSymbol>,
    },
    TriggeringSpellManaSpentToCastAtLeast {
        amount: u32,
        symbol: Option<ManaSymbol>,
    },
    ColoredManaSpentToCastThisSpellAtLeast(u32),
    TriggeringSpellColoredManaSpentToCastAtLeast(u32),
    YouControlMoreCreaturesThanTargetSpellController,
    TargetHasGreatestPowerAmongCreatures,
    TargetManaValueLteColorsSpentToCastThisSpell,
    ItIsNight,
    FirstCombatPhaseOfTurn,
    /// This condition's controller is the active player and the game is in
    /// either a precombat or postcombat main phase.
    SourceControllersMainPhase,
    SourceControllersEndStep,
    SourceIsTapped,
    SourceIsSaddled,
    SourceCrewedByExactly {
        count: u32,
        filter: ObjectFilter,
    },
    SourceDevouredCreaturesOrMore(u32),
    SourceIsMonstrous,
    SourceIsRenowned,
    SourceIsFaceDown,
    SourceMatches(ObjectFilter),
    /// The battlefield object this Aura or Equipment source is attached to
    /// matches the filter.
    AttachedToSourceMatches(ObjectFilter),
    /// A numeric count of matching attachments on one semantically selected
    /// host satisfies the authored comparison.
    AttachmentCount {
        attachment: ObjectFilter,
        host: AttachmentConditionHost,
        comparison: Comparison,
        display: String,
    },
    SourceHasNoCounter(CounterType),
    SourceHasCounterAtLeast {
        counter_type: CounterType,
        count: u32,
        surface: SourceCounterThresholdSurface,
    },
    SourceHasCountersAtLeast(u32),
    SourcePowerAtLeast(u32),
    SourceDealtCombatDamageToPlayerThisTurn,
    PlayerWasDealtCombatDamageByCreatureSubtypeThisTurn {
        player: PlayerFilter,
        subtype: Subtype,
    },
    ManaSpentToCastThisSpellAtLeast {
        amount: u32,
        symbol: Option<ManaSymbol>,
    },
    SnowManaOfAnySpellColorSpentToCastThisSpell,
    SameColorManaSpentToCastThisSpellAtLeast(u32),
    ColorsOfManaSpentToCastThisSpellOrMore(u32),
    YouControlCommander,
    TaggedObjectMatches(TagKey, ObjectFilter),
    /// Match only the snapshot stored for a tagged object.
    ///
    /// Used for past-tense zone-change predicates such as "if it was a
    /// creature". Unlike `TaggedObjectMatches`, this never falls back to the
    /// object's current characteristics.
    TaggedObjectMatchedLastKnown(TagKey, ObjectFilter),
    TaggedObjectIsTopOfLibrary {
        tag: TagKey,
        player: PlayerFilter,
    },
    StableObjectIsTopOfLibrary {
        stable_id: StableId,
        player: PlayerId,
        library_top_revision: u64,
    },
    TaggedObjectWasCast(TagKey),
    TaggedObjectIsSoulbondPaired(TagKey),
    EnchantedPermanentAttackedThisTurn,
    EnchantedPermanentAttackedOrBlockedSinceLastUpkeep,
    /// The resolving ability's source either blocked or became blocked since
    /// its controller's previous upkeep.
    SourceBlockedOrBecameBlockedSinceLastUpkeep,
    /// Two or more selected object targets do not all have the same current color set.
    ///
    /// This is resolution-context-only because it compares the objects selected for
    /// the resolving spell or ability after continuous effects are applied.
    TargetObjectsHaveDifferentColorSets,
    TargetMatches(ObjectFilter),
    TargetIsSoulbondPaired,
    PlayerTaggedObjectMatches {
        player: PlayerFilter,
        tag: TagKey,
        filter: ObjectFilter,
        mode: TaggedObjectMatchMode,
    },
    PlayerTaggedObjectEnteredBattlefieldThisTurn {
        player: PlayerFilter,
        tag: TagKey,
    },
    PlayerOwnsCardNamedInZones {
        player: PlayerFilter,
        name: String,
        zones: Vec<Zone>,
    },
    ThisAbilityResolvedThisTurnExactly(u32),
    FirstTimeThisTurn,
    SourceFirstCrewedThisTurn,
    MaxTimesEachTurn(u32),
    DoThisMaxTimesEachTurn(u32),
    TriggeringObjectWasEnchanted,
    /// The object named by the triggering tap event has exactly one tap event
    /// in the current turn's history (the event currently being checked).
    TriggeringObjectBecameTappedFirstTimeThisTurn,
    /// The object named by the triggering counter event has exactly one
    /// counter-addition event in the current turn's history (the event
    /// currently being checked). This is deliberately per object rather than
    /// per triggered ability.
    TriggeringObjectHadCountersPutFirstTimeThisTurn,
    TriggeringObjectHadToAttackThisCombat,
    TriggeringObjectHadCounters {
        counter_type: CounterType,
        min_count: u32,
    },
    ControlCreaturesTotalPowerAtLeast(u32),
    CardInYourGraveyard {
        card_types: Vec<crate::CardType>,
        subtypes: Vec<crate::Subtype>,
    },
    /// The source card is in its owner's ordered graveyard with at least
    /// `count` matching cards closer to the top of that graveyard.
    SourceInGraveyardWithCardsAbove {
        filter: ObjectFilter,
        count: u32,
    },
    SourceIsInZone(Zone),
    ActivationTiming(ActivationTiming),
    MaxActivationsPerTurn(u32),
    SourceIsEquipped,
    SourceIsEnchanted,
    EnchantedPermanentIsCreature,
    EnchantedPermanentIsLand,
    EnchantedPermanentIsEquipment,
    EnchantedPermanentIsVehicle,
    EquippedCreatureTapped,
    EquippedCreatureUntapped,
    EquippedCreatureAttacking,
    SourceChosenOption(String),
    SecretChoicesMatch,
    VoteOptionGetsMoreVotes(String),
    VoteOptionGetsMoreVotesOrTied(String),
    CountComparison {
        count: AnthemCountExpression,
        comparison: Comparison,
        display: Option<String>,
    },
    CountParity {
        count: AnthemCountExpression,
        even: bool,
        display: Option<String>,
    },
    OwnsCardExiledWithCounter(CounterType),
    SourceAttackedThisTurn,
    /// The resolving ability's source attacked a Battle during the current turn.
    SourceAttackedBattleThisTurn,
    /// "this creature is suspected"
    SourceSuspected,
    SourceCameUnderYourControlThisTurn,
    SourceAttackedOrBlockedThisTurn,
    SourceIsUntapped,
    SourceIsAttacking,
    SourceIsBlocking,
    SourceIsSoulbondPaired,
    TurnHistory(TurnHistoryCondition),
    PlayerGraveyardHasCardsAtLeast {
        player: crate::PlayerId,
        count: usize,
    },
    XValueAtLeast(u32),
    Custom(crate::InternedStr),
    Not(Box<Condition>),
    And(Box<Condition>, Box<Condition>),
    Or(Box<Condition>, Box<Condition>),
}

#[cfg(test)]
mod tests {
    use super::{Condition, ManaSpendPermission, ManaSpendScope, Restriction, Value};
    use crate::{AnthemCountExpression, Comparison, ObjectFilter, PlayerFilter};

    #[test]
    fn value_builders_stay_core_owned() {
        assert_eq!(Value::fixed(3), Value::Fixed(3));
        assert_eq!(
            Value::creatures_you_control(),
            Value::Count(ObjectFilter::creature().you_control())
        );
    }

    #[test]
    fn restriction_builders_stay_pure() {
        assert_eq!(
            Restriction::cast_creature_spells(PlayerFilter::Opponent),
            Restriction::CastSpellsMatching(
                PlayerFilter::Opponent,
                ObjectFilter::default().with_type(crate::CardType::Creature)
            )
        );
    }

    #[test]
    fn mana_spend_permission_builders_keep_scope() {
        let permission = ManaSpendPermission::any_color_for_activation(
            PlayerFilter::You,
            ObjectFilter::creature(),
        );
        assert_eq!(permission.player, PlayerFilter::You);
        assert_eq!(
            permission.scope,
            ManaSpendScope::ActivationCostsOf(ObjectFilter::creature())
        );
    }

    #[test]
    fn count_comparison_condition_keeps_core_payloads() {
        let condition = Condition::CountComparison {
            count: AnthemCountExpression::MatchingFilter(ObjectFilter::artifact()),
            comparison: Comparison::GreaterThanOrEqual(1),
            display: Some("artifacts".to_string()),
        };
        match condition {
            Condition::CountComparison { display, .. } => {
                assert_eq!(display.as_deref(), Some("artifacts"));
            }
            _ => panic!("wrong condition variant"),
        }
    }
}
