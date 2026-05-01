use crate::{
    ActivationTiming, AnthemCountExpression, Comparison, CounterType, EffectId, EventValueSpec,
    ManaSymbol, ObjectFilter, PlayerFilter, StableId, TagKey, ValueComparisonOperator, Zone,
};
use crate::{ChooseSpec, Color};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EffectMetricSource {
    Outcome,
    ChosenObjects,
    AffectedObjects,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EffectMetric {
    Count,
    ChosenCount,
    AffectedCount,
    LifeLost,
    LifeGained,
    DamageDealt,
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ValueSurfaceHint {
    WhereXIs,
    EqualTo,
    ForEach,
}

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
    HalfRoundedDown(Box<Value>),
    Count(ObjectFilter),
    CountScaled(ObjectFilter, i32),
    TotalPower(ObjectFilter),
    TotalToughness(ObjectFilter),
    TotalManaValue(ObjectFilter),
    GreatestPower(ObjectFilter),
    GreatestToughness(ObjectFilter),
    GreatestManaValue(ObjectFilter),
    Min(Box<Value>, Box<Value>),
    BasicLandTypesAmong(ObjectFilter),
    CreatureTypesAmong(ObjectFilter),
    CardTypesAmong(ObjectFilter),
    ColorsAmong(ObjectFilter),
    DistinctNames(ObjectFilter),
    DistinctPowers(ObjectFilter),
    CreaturesDiedThisTurn,
    CreaturesDiedThisTurnControlledBy(PlayerFilter),
    CountPlayers(PlayerFilter),
    PlayersWhoControlMoreThanYou(ObjectFilter),
    PartySize(PlayerFilter),
    SourcePower,
    SourceToughness,
    PowerOf(Box<ChooseSpec>),
    ToughnessOf(Box<ChooseSpec>),
    ManaValueOf(Box<ChooseSpec>),
    LifeTotal(PlayerFilter),
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
    NoncombatDamageDealtToPlayersThisTurn(PlayerFilter),
    MaxCardsDrawnThisTurn(PlayerFilter),
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
    CommanderCastCount(PlayerFilter),
    DamageDealtThisTurnByTaggedSpellCast(TagKey),
    CardTypesInGraveyard(PlayerFilter),
    Devotion {
        player: PlayerFilter,
        color: Color,
    },
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
    EventValue(EventValueSpec),
    EventValueOffset(EventValueSpec, i32),
    WasKicked,
    WasBoughtBack,
    WasEntwined,
    WasPaid(usize),
    WasPaidLabel(String),
    TimesPaid(usize),
    TimesPaidLabel(String),
    KickCount,
    MagicGamesLostToOpponentsSinceLastWin,
    CountersOnSource(CounterType),
    CountersOn(Box<ChooseSpec>, Option<CounterType>),
    TaggedCount,
    VoteCount(String),
}

impl Value {
    pub fn fixed(n: i32) -> Self {
        Self::Fixed(n)
    }

    pub fn creatures_you_control() -> Self {
        Self::Count(ObjectFilter::creature().you_control())
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

#[derive(Debug, Clone, PartialEq)]
pub enum Restriction {
    AdditionalLandPlays(PlayerFilter, u32),
    GainLife(PlayerFilter),
    SearchLibraries(PlayerFilter),
    CastSpellsMatching(PlayerFilter, ObjectFilter),
    ActivateNonManaAbilities(PlayerFilter),
    ActivateAbilitiesOf(ObjectFilter),
    ActivateTapAbilitiesOf(ObjectFilter),
    ActivateNonManaAbilitiesOf(ObjectFilter),
    CastMoreThanOneSpellEachTurn(PlayerFilter, ObjectFilter),
    DrawCards(PlayerFilter),
    DrawExtraCards(PlayerFilter),
    ChangeLifeTotal(PlayerFilter),
    LoseGame(PlayerFilter),
    WinGame(PlayerFilter),
    BecomeMonarch(PlayerFilter),
    PreventDamage,
    Attack(ObjectFilter),
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
    BeTargetedPlayer(PlayerFilter),
    BeTargetedPlayerFrom(PlayerFilter, ObjectFilter),
    BeCountered(ObjectFilter),
    Transform(ObjectFilter),
    PhaseOut(ObjectFilter),
    AttackOrBlock(ObjectFilter),
    AttackOrBlockAlone(ObjectFilter),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ManaSpendPermission {
    pub player: PlayerFilter,
    pub scope: ManaSpendScope,
}

impl ManaSpendPermission {
    pub fn any_color(player: PlayerFilter) -> Self {
        Self {
            player,
            scope: ManaSpendScope::AllCosts,
        }
    }

    pub fn any_color_for_activation(player: PlayerFilter, filter: ObjectFilter) -> Self {
        Self {
            player,
            scope: ManaSpendScope::ActivationCostsOf(filter),
        }
    }

    pub fn any_color_for_casting_stable_ids(
        player: PlayerFilter,
        stable_ids: Vec<StableId>,
    ) -> Self {
        Self {
            player,
            scope: ManaSpendScope::CastingSpellsWithStableIds(stable_ids),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ManaSpendScope {
    AllCosts,
    ActivationCostsOf(ObjectFilter),
    CastingSpellsWithStableIds(Vec<StableId>),
}

impl Restriction {
    pub fn additional_land_plays(filter: PlayerFilter, count: u32) -> Self {
        Self::AdditionalLandPlays(filter, count)
    }

    pub fn gain_life(filter: PlayerFilter) -> Self {
        Self::GainLife(filter)
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

    pub fn attack_or_block(filter: ObjectFilter) -> Self {
        Self::AttackOrBlock(filter)
    }

    pub fn attack_or_block_alone(filter: ObjectFilter) -> Self {
        Self::AttackOrBlockAlone(filter)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Condition {
    YouControl(ObjectFilter),
    OpponentControls(ObjectFilter),
    PlayerControls {
        player: PlayerFilter,
        filter: ObjectFilter,
    },
    PlayerControlsAtLeast {
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
    PlayerControlsAtLeastWithDifferentPowers {
        player: PlayerFilter,
        filter: ObjectFilter,
        count: u32,
    },
    PlayerControlsMost {
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
    PlayerCommittedCrimeThisTurn {
        player: PlayerFilter,
    },
    PlayerCompletedDungeon {
        player: PlayerFilter,
        dungeon_name: Option<String>,
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
    PlayerHasMoreCardsInHandThanYou {
        player: PlayerFilter,
    },
    PlayerHasMoreCardsInHandThanEachOtherPlayer {
        player: PlayerFilter,
    },
    YouHaveCardInHandMatching(ObjectFilter),
    YourTurn,
    YourFirstTurnsOfTheGameOrFewer(u32),
    CreatureDiedThisTurn,
    CreatureDiedThisTurnOrMore(u32),
    CastSpellThisTurn,
    PlayerCastSpellsThisTurnOrMore {
        player: PlayerFilter,
        count: u32,
    },
    AttackedThisTurn,
    OpponentLostLifeThisTurn,
    PermanentLeftBattlefieldThisTurn,
    PermanentLeftBattlefieldUnderYourControlThisTurn,
    ObjectEnteredBattlefieldThisTurn(ObjectFilter),
    ObjectPutIntoGraveyardFromBattlefieldThisTurn(ObjectFilter),
    SourceWasCast,
    ThisSpellEscaped,
    ThisSpellWasCastFromZone(Zone),
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
    ValueComparison {
        left: Value,
        operator: ValueComparisonOperator,
        right: Value,
    },
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
    ThisSpellPaidLabel(String),
    YouHaveFullParty,
    TargetSpellCastOrderThisTurn(u32),
    TargetSpellControllerIsPoisoned,
    TargetSpellManaSpentToCastAtLeast {
        amount: u32,
        symbol: Option<ManaSymbol>,
    },
    YouControlMoreCreaturesThanTargetSpellController,
    TargetHasGreatestPowerAmongCreatures,
    TargetManaValueLteColorsSpentToCastThisSpell,
    SourceIsTapped,
    SourceIsSaddled,
    SourceIsMonstrous,
    SourceIsFaceDown,
    SourceMatches(ObjectFilter),
    SourceHasNoCounter(CounterType),
    SourceHasCounterAtLeast {
        counter_type: CounterType,
        count: u32,
    },
    SourcePowerAtLeast(u32),
    ManaSpentToCastThisSpellAtLeast {
        amount: u32,
        symbol: Option<ManaSymbol>,
    },
    SameColorManaSpentToCastThisSpellAtLeast(u32),
    ColorsOfManaSpentToCastThisSpellOrMore(u32),
    YouControlCommander,
    TaggedObjectMatches(TagKey, ObjectFilter),
    TaggedObjectWasCast(TagKey),
    TaggedObjectIsSoulbondPaired(TagKey),
    EnchantedPermanentAttackedThisTurn,
    TargetMatches(ObjectFilter),
    TargetIsSoulbondPaired,
    PlayerTaggedObjectMatches {
        player: PlayerFilter,
        tag: TagKey,
        filter: ObjectFilter,
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
    MaxTimesEachTurn(u32),
    DoThisMaxTimesEachTurn(u32),
    TriggeringObjectWasEnchanted,
    TriggeringObjectHadCounters {
        counter_type: CounterType,
        min_count: u32,
    },
    ControlCreaturesTotalPowerAtLeast(u32),
    CardInYourGraveyard {
        card_types: Vec<crate::CardType>,
        subtypes: Vec<crate::Subtype>,
    },
    SourceIsInZone(Zone),
    ActivationTiming(ActivationTiming),
    MaxActivationsPerTurn(u32),
    SourceIsEquipped,
    SourceIsEnchanted,
    EnchantedPermanentIsCreature,
    EnchantedPermanentIsEquipment,
    EnchantedPermanentIsVehicle,
    EquippedCreatureTapped,
    EquippedCreatureUntapped,
    EquippedCreatureAttacking,
    SourceChosenOption(String),
    VoteOptionGetsMoreVotes(String),
    VoteOptionGetsMoreVotesOrTied(String),
    CountComparison {
        count: AnthemCountExpression,
        comparison: Comparison,
        display: Option<String>,
    },
    OwnsCardExiledWithCounter(CounterType),
    SourceAttackedThisTurn,
    SourceCameUnderYourControlThisTurn,
    SourceAttackedOrBlockedThisTurn,
    SourceIsUntapped,
    SourceIsAttacking,
    SourceIsBlocking,
    SourceIsSoulbondPaired,
    PlayerGraveyardHasCardsAtLeast {
        player: crate::PlayerId,
        count: usize,
    },
    XValueAtLeast(u32),
    Custom(&'static str),
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
