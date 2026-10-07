use ironsmith_compiler_ast::TagRef;
use ironsmith_core::tag::TagKeyWalk;

#[path = "predicates/player.rs"]
mod player;
pub use player::*;
#[path = "predicates/source.rs"]
mod source;
pub use source::*;
#[path = "predicates/triggering.rs"]
mod triggering;
pub use triggering::*;
#[path = "predicates/turn_events.rs"]
mod turn_events;
pub use turn_events::*;

use super::*;

/// A stated limit on how often a triggered ability may fire.
#[derive(Debug, Clone, PartialEq, TagKeyWalk)]
pub enum TriggerFrequencyPredicateAst {
    /// "for the first time this turn"
    FirstTimeThisTurn,
    /// "the first time this creature becomes crewed each turn"
    SourceFirstCrewedThisTurn,
    /// "only once each turn", or a stated maximum
    MaxTimesEachTurn(u32),
    /// A maximum stated on the effect rather than the trigger
    DoThisMaxTimesEachTurn(u32),
}

#[derive(Debug, Clone, PartialEq, TagKeyWalk)]
pub enum PredicateAst {
    /// TurnEvents: see [`TurnEventPredicateAst`].
    TurnEvents(TurnEventPredicateAst),
    /// Triggering: see [`TriggeringPredicateAst`].
    Triggering(TriggeringPredicateAst),
    /// Source: see [`SourcePredicateAst`].
    Source(SourcePredicateAst),
    /// Player: see [`PlayerPredicateAst`].
    Player(PlayerPredicateAst),
    ItIsNight,
    FirstCombatPhaseOfTurn,
    ItIsLandCard,
    ItIsSoulbondPaired,
    ItMatches(ObjectFilter),
    /// The implicit object matched this filter immediately before its zone change.
    ///
    /// This is distinct from `ItMatches`: phrases such as "if it was a creature"
    /// use last-known information and must not become true from the object's
    /// characteristics in its new zone.
    ItMatchedLastKnown(ObjectFilter),
    TargetMatches(ObjectFilter),
    TaggedMatches(TagRef, ObjectFilter),
    /// Explicit tagged object, evaluated exclusively from retained pre-move evidence.
    TaggedMatchedLastKnown(TagRef, ObjectFilter),
    TaggedWasCast(TagRef),
    EnchantedPermanentAttackedThisTurn,
    EnchantedPermanentAttackedOrBlockedSinceLastUpkeep,
    TargetObjectsHaveDifferentColorSets,
    AnOpponentHasFewerThanPlayer {
        player: PlayerAst,
        filter: ObjectFilter,
    },
    CountParity {
        count: crate::static_abilities::AnthemCountExpression,
        even: bool,
        display: Option<String>,
    },
    /// "you control a Forest", "there are two or more cards in your graveyard" —
    /// a stated count compared against a threshold.
    CountComparison {
        count: crate::static_abilities::AnthemCountExpression,
        comparison: crate::effect::Comparison,
        display: Option<String>,
    },
    VoteOptionGetsMoreVotes {
        option: String,
    },
    SecretChoicesMatch,
    VoteOptionGetsMoreVotesOrTied {
        option: String,
    },
    NoVoteObjectsMatched {
        filter: ObjectFilter,
    },
    YouHaveNoCardsInHand,
    /// The battlefield object this Aura or Equipment source is attached to
    /// matches the filter at the time the trigger is checked.
    AttachedToSourceMatches(ObjectFilter),

    YourTurn,
    YouHaveFullParty,
    ThisSpellWasCastAtSorceryTiming,
    ThisSpellEscaped,
    ThisSpellWasKicked,
    ThisSpellPaidLabel(OptionalCostRef),
    /// A condition that arrived already bound.
    ///
    /// The spell-cost model states its conditions in the resolved vocabulary
    /// and has no recognized form to offer, so a static ability that combines
    /// one with a recognized condition carries it through here. This is the
    /// only place a bound condition may sit inside a predicate, and it exists
    /// so that bridge is one named variant rather than a scatter of binds.
    Bound(Box<crate::ConditionExpr>),
    /// "if you control a basic land" — the plain "you control one of these"
    /// check, distinct from a counted comparison.
    YouControl(ObjectFilter),
    /// "as long as it has two or more Auras attached to it"
    AttachmentCount {
        attachment: ObjectFilter,
        host: crate::AttachmentConditionHost,
        comparison: crate::effect::Comparison,
        display: String,
    },
    /// "if you have 5 or less life"
    LifeTotalOrLess(i32),
    /// "if you have one or more cards in hand"
    CardsInHandOrMore(i32),
    /// "if that card is the top card of your library"
    TaggedObjectIsTopOfLibrary {
        tag: TagRef,
        player: PlayerAst,
    },
    /// "if X is 5 or greater"
    XValueAtLeast(u32),
    /// "if both targets are still legal as this ability resolves"
    AllTargetsStillLegal,
    /// "if two or more colors of mana were spent to cast it"
    ColorsOfManaSpentToCastThisSpellOrMore(u32),
    /// "if you have a card in hand matching this"
    YouHaveCardInHandMatching(ObjectFilter),
    /// "during your first turn of the game"
    YourFirstTurnsOfTheGameOrFewer(u32),
    /// "as long as equipped creature is attacking"
    EquippedCreatureAttacking,
    /// "as long as equipped creature is tapped"
    EquippedCreatureTapped,
    /// "as long as equipped creature is untapped"
    EquippedCreatureUntapped,
    /// "as long as enchanted permanent is a creature"
    EnchantedPermanentIsCreature,
    /// "as long as enchanted permanent is a land"
    EnchantedPermanentIsLand,
    /// "as long as enchanted permanent is an Equipment"
    EnchantedPermanentIsEquipment,
    /// "as long as enchanted permanent is a Vehicle"
    EnchantedPermanentIsVehicle,
    /// "if creatures you control have total power 10 or greater"
    ControlCreaturesTotalPowerAtLeast(u32),
    /// "if there is a creature card in your graveyard"
    CardInYourGraveyard {
        card_types: Vec<crate::types::CardType>,
        subtypes: Vec<crate::types::Subtype>,
    },
    /// "Activate only as a sorcery", "only during combat" — when the text says
    /// the ability may be activated.
    ActivationTiming(crate::ability::ActivationTiming),
    /// "only once each turn", as a cap on activations.
    MaxActivationsPerTurn(u32),
    /// Lifetime activation cap for the current object instance and ability.
    MaxActivationsPerObject(u32),
    /// "if that turn is an extra turn"
    CurrentTurnIsExtra,
    /// How often the ability may fire, as the text states it — "only once each
    /// turn", "for the first time this turn". It is a fact about the text, so
    /// it is recognized as a predicate rather than resolved on the spot.
    TriggerFrequency(TriggerFrequencyPredicateAst),
    TargetWasKicked,
    TargetSpellCastOrderThisTurn(u32),
    TargetSpellControllerIsPoisoned,
    TargetSpellNoManaSpentToCast,
    YouControlMoreCreaturesThanTargetSpellController,
    TargetIsBlocked,
    TargetHasGreatestPowerAmongCreatures,
    TargetManaValueLteColorsSpentToCastThisSpell,
    ManaSpentToCastThisSpellAtLeast {
        amount: u32,
        symbol: Option<ManaSymbol>,
    },
    ColoredManaSpentToCastThisSpellAtLeast(u32),
    SnowManaOfAnySpellColorSpentToCastThisSpell,
    SameColorManaSpentToCastThisSpellAtLeast(u32),
    ThisSpellWasCastFromZone(Zone),
    ThisSpellWasCastFromNonHand,
    ThisSpellWasForetold,
    TurnHistory(TurnHistoryPredicateAst),
    ValueComparison {
        left: Value,
        operator: crate::effect::ValueComparisonOperator,
        right: Value,
    },
    ValueIsPrime(Value),
    Not(Box<PredicateAst>),
    And(Box<PredicateAst>, Box<PredicateAst>),
    Or(Box<PredicateAst>, Box<PredicateAst>),
}

#[derive(Debug, Clone, PartialEq, TagKeyWalk)]
pub enum TurnHistoryPredicateAst {
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
    PlayerPlayedLandThisTurn(PlayerAst),
    PlayerActivatedLoyaltyAbilityThisTurn(PlayerAst),
    TriggeringObjectDied,
    PlayerPlayedCardFromZoneThisTurn {
        player: PlayerAst,
        zone: Zone,
    },
    PlayerCastSpellFromZoneThisTurn {
        player: PlayerAst,
        zone: Zone,
    },
    PlayerActivatedAbilityOfCardInZoneThisTurn {
        player: PlayerAst,
        zone: Zone,
    },
    PlayerVisitedAttractionThisTurn(PlayerAst),
    TriggeringPlayerAttackedControllerLastTurn,
    PlayerLostLifeLastTurn(PlayerAst),
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

/// Whether a compared value measures the source object itself ("the number
/// of charge counters on this artifact", "this creature's power").
fn value_measures_source_object(value: &Value) -> bool {
    match value {
        Value::SurfaceHinted { value, .. } => value_measures_source_object(value),
        Value::CountersOnSource(_) | Value::SourcePower | Value::SourceToughness => true,
        Value::CountersOn(spec, _)
        | Value::PowerOf(spec)
        | Value::BasePowerOf(spec)
        | Value::ToughnessOf(spec)
        | Value::ManaValueOf(spec) => matches!(spec.base(), ChooseSpec::Source),
        // A stated source that damaged an existential class remains the
        // subject ("put a counter on him"). An explicit second object, as in
        // "this creature dealt damage to it", must retain its own antecedent.
        Value::DamageHistory(query) => {
            matches!(&query.sources, ironsmith_core::DamageHistorySources::Reference(spec)
                if matches!(spec.base(), ChooseSpec::Source))
                && !matches!(
                    query.recipients,
                    ironsmith_core::DamageHistoryRecipients::Reference(_)
                )
        }
        _ => false,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, TagKeyWalk)]
pub enum PredicateReferenceAntecedent {
    SourceObject,
}

impl PredicateAst {
    /// Whether this predicate evaluates the object denoted by an implicit
    /// `it`/`that object` reference supplied by the surrounding effect.
    pub fn uses_implicit_object_reference(&self) -> bool {
        match self {
            PredicateAst::ItIsLandCard
            | PredicateAst::ItIsSoulbondPaired
            | PredicateAst::ItMatches(_)
            | PredicateAst::ItMatchedLastKnown(_)
            | PredicateAst::TargetMatches(_) => true,
            PredicateAst::Not(inner) => inner.uses_implicit_object_reference(),
            PredicateAst::And(left, right) | PredicateAst::Or(left, right) => {
                left.uses_implicit_object_reference() || right.uses_implicit_object_reference()
            }
            _ => false,
        }
    }

    pub fn reference_antecedent(&self) -> Option<PredicateReferenceAntecedent> {
        match self {
            PredicateAst::Source(SourcePredicateAst::SourceChosenOption(_))
            | PredicateAst::Source(SourcePredicateAst::SourceIsTapped)
            | PredicateAst::Source(SourcePredicateAst::SourceIsEquipped)
            | PredicateAst::Source(SourcePredicateAst::SourceIsEnchanted)
            | PredicateAst::Source(SourcePredicateAst::SourceIsSaddled)
            | PredicateAst::Source(SourcePredicateAst::SourceIsRenowned)
            | PredicateAst::Source(SourcePredicateAst::SourceCrewedByExactly { .. })
            | PredicateAst::Source(SourcePredicateAst::SourceMatches(_))
            | PredicateAst::AttachedToSourceMatches(_)
            | PredicateAst::Source(SourcePredicateAst::SourceHasNoCounter(_))
            | PredicateAst::Source(SourcePredicateAst::SourceHasCounterAtLeast { .. })
            | PredicateAst::Source(SourcePredicateAst::SourceHasCountersAtLeast(_))
            | PredicateAst::Source(SourcePredicateAst::SourceHasAttachmentsMatching { .. })
            | PredicateAst::Source(SourcePredicateAst::SourcePowerAtLeast(_))
            | PredicateAst::Source(SourcePredicateAst::SourceAttackedThisTurn)
            | PredicateAst::Source(SourcePredicateAst::SourceSuspected)
            | PredicateAst::Source(SourcePredicateAst::SourceCameUnderYourControlThisTurn)
            | PredicateAst::Source(SourcePredicateAst::SourceAttackedOrBlockedThisTurn)
            | PredicateAst::Source(SourcePredicateAst::SourceAttackedOrBlockedThisCombat)
            | PredicateAst::Source(SourcePredicateAst::SourceInGraveyardWithCardsAbove {
                ..
            })
            | PredicateAst::Source(SourcePredicateAst::SourceIsInZone(_))
            | PredicateAst::Source(SourcePredicateAst::SourceWasCast)
            | PredicateAst::ThisSpellWasCastAtSorceryTiming
            | PredicateAst::ThisSpellEscaped
            | PredicateAst::ThisSpellWasKicked
            | PredicateAst::ThisSpellPaidLabel(_)
            | PredicateAst::ThisSpellWasCastFromZone(_)
            | PredicateAst::ThisSpellWasCastFromNonHand
            | PredicateAst::ThisSpellWasForetold
            | PredicateAst::TurnHistory(
                TurnHistoryPredicateAst::SourceCrewedByAtLeast { .. }
                | TurnHistoryPredicateAst::SourceWasCast { .. }
                | TurnHistoryPredicateAst::SourceWasCastByController { .. }
                | TurnHistoryPredicateAst::SourceWasKicked { .. }
                | TurnHistoryPredicateAst::SourceEnteredBattlefieldThisTurn { .. }
                | TurnHistoryPredicateAst::SourceAttackedThisTurn { .. },
            ) => Some(PredicateReferenceAntecedent::SourceObject),
            // "if this artifact has fewer than three charge counters on it":
            // a comparison whose subject is the source names the source as
            // the clause's object antecedent, exactly like the typed
            // source-state predicates above.
            PredicateAst::ValueComparison { left, right, .. }
                if value_measures_source_object(left) || value_measures_source_object(right) =>
            {
                Some(PredicateReferenceAntecedent::SourceObject)
            }
            PredicateAst::And(left, right) | PredicateAst::Or(left, right) => left
                .reference_antecedent()
                .or_else(|| right.reference_antecedent()),
            PredicateAst::Not(inner) => inner.reference_antecedent(),
            _ => None,
        }
    }

    pub fn establishes_source_object_antecedent(&self) -> bool {
        matches!(
            self.reference_antecedent(),
            Some(PredicateReferenceAntecedent::SourceObject)
        )
    }
}

impl ironsmith_core::ConditionConjunction for PredicateAst {
    fn and(self, other: Self) -> Self {
        PredicateAst::And(Box::new(self), Box::new(other))
    }
}

#[cfg(test)]
mod damage_history_antecedent_tests {
    use super::*;
    use ironsmith_core::{
        DamageHistoryQuery, DamageHistoryRecipients, DamageHistoryReduction, DamageHistorySources,
    };

    #[test]
    fn damage_history_source_subject_does_not_shadow_an_explicit_recipient() {
        let make = |recipients| PredicateAst::ValueComparison {
            left: Value::DamageHistory(Box::new(DamageHistoryQuery {
                sources: DamageHistorySources::Reference(Box::new(ChooseSpec::Source)),
                recipients,
                combat: None,
                reduction: DamageHistoryReduction::Total,
            })),
            operator: crate::effect::ValueComparisonOperator::GreaterThan,
            right: Value::Fixed(0),
        };
        assert!(
            make(DamageHistoryRecipients::MatchingObjects(
                ObjectFilter::creature().other()
            ))
            .establishes_source_object_antecedent()
        );
        assert!(
            !make(DamageHistoryRecipients::Reference(Box::new(
                ChooseSpec::Tagged(crate::tag::CompilerReferenceTag::It.key(),)
            )))
            .establishes_source_object_antecedent()
        );
    }
}
