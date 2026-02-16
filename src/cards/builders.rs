//! Extended card builder with ability support.
//!
//! This module extends the CardBuilder with methods for adding abilities,
//! making it easy to define cards with their complete gameplay mechanics.

use crate::ability::{
    self, Ability, AbilityKind, ActivatedAbility, ActivationTiming, LevelAbility, ManaAbility,
    ManaAbilityCondition, TriggeredAbility,
};
use crate::alternative_cast::AlternativeCastingMethod;
use crate::card::{CardBuilder, PowerToughness, PtValue};
use crate::color::ColorSet;
use crate::cost::{OptionalCost, TotalCost};
use crate::effect::{
    ChoiceCount, Condition, Effect, EffectId, EffectMode, EffectPredicate, EventValueSpec, Until,
    Value,
};
use crate::effects::VoteOption;
use crate::filter::AlternativeCastKind;
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::object::CounterType;
use crate::static_abilities::{
    Anthem, AnthemCountExpression, AnthemValue, GrantAbility, StaticAbility, StaticAbilityId,
    StaticCondition,
};
use crate::tag::TagKey;
use crate::target::{
    ChooseSpec, ObjectFilter, ObjectRef, PlayerFilter, TaggedObjectConstraint,
    TaggedOpbjectRelation,
};
use crate::triggers::Trigger;
use crate::types::{CardType, Subtype, Supertype};
use crate::zone::Zone;
use std::collections::HashMap;

use super::CardDefinition;
mod parser;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CardTextError {
    UnsupportedLine(String),
    ParseError(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum KeywordAction {
    Flying,
    Menace,
    Hexproof,
    Haste,
    Improvise,
    Convoke,
    AffinityForArtifacts,
    Delve,
    FirstStrike,
    DoubleStrike,
    Deathtouch,
    Lifelink,
    Vigilance,
    Trample,
    Reach,
    Defender,
    Flash,
    Phasing,
    Indestructible,
    Shroud,
    Ward(u32),
    Wither,
    Infect,
    Undying,
    Persist,
    Prowess,
    Exalted,
    Storm,
    Toxic(u32),
    Fear,
    Intimidate,
    Shadow,
    Horsemanship,
    Flanking,
    Landwalk(Subtype),
    Bloodthirst(u32),
    Rampage(u32),
    Bushido(u32),
    Changeling,
    ProtectionFrom(ColorSet),
    ProtectionFromAllColors,
    ProtectionFromColorless,
    ProtectionFromCardType(CardType),
    ProtectionFromSubtype(Subtype),
    Unblockable,
    Devoid,
    Marker(&'static str),
    MarkerText(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextSpan {
    pub line: usize,
    pub start: usize,
    pub end: usize,
}

impl TextSpan {
    fn synthetic() -> Self {
        Self {
            line: 0,
            start: 0,
            end: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Token {
    Word(String, TextSpan),
    Comma(TextSpan),
    Period(TextSpan),
    Colon(TextSpan),
    Semicolon(TextSpan),
}

impl Token {
    fn as_word(&self) -> Option<&str> {
        match self {
            Token::Word(word, _) => Some(word.as_str()),
            _ => None,
        }
    }

    fn is_word(&self, value: &str) -> bool {
        matches!(self, Token::Word(word, _) if word == value)
    }

    fn span(&self) -> TextSpan {
        match self {
            Token::Word(_, span)
            | Token::Comma(span)
            | Token::Period(span)
            | Token::Colon(span)
            | Token::Semicolon(span) => *span,
        }
    }
}

#[derive(Debug, Clone)]
enum LineAst {
    Abilities(Vec<KeywordAction>),
    StaticAbility(StaticAbility),
    StaticAbilities(Vec<StaticAbility>),
    Ability(ParsedAbility),
    Triggered {
        trigger: TriggerSpec,
        effects: Vec<EffectAst>,
        once_each_turn: bool,
    },
    Statement {
        effects: Vec<EffectAst>,
    },
    AdditionalCost {
        effects: Vec<EffectAst>,
    },
    AdditionalCostChoice {
        options: Vec<AdditionalCostChoiceOptionAst>,
    },
    AlternativeCost {
        mana_cost: Option<ManaCost>,
        cost_effects: Vec<Effect>,
    },
    AlternativeCastingMethod(AlternativeCastingMethod),
}

#[derive(Debug, Clone)]
struct AdditionalCostChoiceOptionAst {
    description: String,
    effects: Vec<EffectAst>,
}

#[derive(Debug, Clone)]
struct ParsedAbility {
    ability: Ability,
    effects_ast: Option<Vec<EffectAst>>,
}

#[derive(Debug, Clone)]
enum TriggerSpec {
    ThisAttacks,
    ThisAttacksWithNOthers(u32),
    Attacks(ObjectFilter),
    ThisBlocks,
    ThisBlocksObject(ObjectFilter),
    ThisBecomesBlocked,
    ThisBlocksOrBecomesBlocked,
    ThisDies,
    ThisLeavesBattlefield,
    ThisBecomesMonstrous,
    ThisBecomesTapped,
    ThisBecomesUntapped,
    ThisTurnedFaceUp,
    ThisBecomesTargeted,
    ThisDealsDamage,
    ThisDealsDamageTo(ObjectFilter),
    DealsDamage(ObjectFilter),
    ThisIsDealtDamage,
    YouGainLife,
    PlayerLosesLife(PlayerFilter),
    YouDrawCard,
    PlayerDrawsCard(PlayerFilter),
    PlayerSacrifices {
        player: PlayerFilter,
        filter: ObjectFilter,
    },
    Dies(ObjectFilter),
    SpellCast {
        filter: Option<ObjectFilter>,
        caster: PlayerFilter,
        during_turn: Option<PlayerFilter>,
        min_spells_this_turn: Option<u32>,
        from_not_hand: bool,
    },
    SpellCopied {
        filter: Option<ObjectFilter>,
        copier: PlayerFilter,
    },
    EntersBattlefield(ObjectFilter),
    EntersBattlefieldTapped(ObjectFilter),
    EntersBattlefieldUntapped(ObjectFilter),
    BeginningOfUpkeep(PlayerFilter),
    BeginningOfDrawStep(PlayerFilter),
    BeginningOfCombat(PlayerFilter),
    BeginningOfEndStep(PlayerFilter),
    BeginningOfPrecombatMain(PlayerFilter),
    ThisEntersBattlefield,
    ThisDealsCombatDamageToPlayer,
    DealsCombatDamageToPlayer(ObjectFilter),
    YouCastThisSpell,
    KeywordAction {
        action: crate::events::KeywordActionKind,
        player: PlayerFilter,
    },
    Custom(String),
    SagaChapter(Vec<u32>),
    Either(Box<TriggerSpec>, Box<TriggerSpec>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PlayerAst {
    You,
    Any,
    Defending,
    Attacking,
    Target,
    TargetOpponent,
    Opponent,
    That,
    ItsController,
    ItsOwner,
    Implicit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReturnControllerAst {
    Preserve,
    Owner,
    You,
}

#[derive(Debug, Clone, PartialEq)]
enum TargetAst {
    Source(Option<TextSpan>),
    AnyTarget(Option<TextSpan>),
    PlayerOrPlaneswalker(PlayerFilter, Option<TextSpan>),
    Spell(Option<TextSpan>),
    Player(PlayerFilter, Option<TextSpan>),
    Object(ObjectFilter, Option<TextSpan>, Option<TextSpan>),
    Tagged(TagKey, Option<TextSpan>),
    WithCount(Box<TargetAst>, ChoiceCount),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ObjectRefAst {
    It,
}

#[derive(Debug, Clone, PartialEq)]
enum PredicateAst {
    ItIsLandCard,
    ItMatches(ObjectFilter),
    TaggedMatches(TagKey, ObjectFilter),
    PlayerControls {
        player: PlayerAst,
        filter: ObjectFilter,
    },
    PlayerHasLessLifeThanYou {
        player: PlayerAst,
    },
    SourceIsTapped,
    SourceHasNoCounter(CounterType),
    YouAttackedThisTurn,
    NoSpellsWereCastLastTurn,
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ControlDurationAst {
    UntilEndOfTurn,
    DuringNextTurn,
    AsLongAsYouControlSource,
    Forever,
}

#[derive(Debug, Clone)]
enum EffectAst {
    DealDamage {
        amount: Value,
        target: TargetAst,
    },
    DealDamageEqualToPower {
        source: TargetAst,
        target: TargetAst,
    },
    Fight {
        creature1: TargetAst,
        creature2: TargetAst,
    },
    FightIterated {
        creature2: TargetAst,
    },
    DealDamageEach {
        amount: Value,
        filter: ObjectFilter,
    },
    Draw {
        count: Value,
        player: PlayerAst,
    },
    Counter {
        target: TargetAst,
    },
    CounterUnlessPays {
        target: TargetAst,
        mana: Vec<ManaSymbol>,
    },
    UnlessPays {
        effects: Vec<EffectAst>,
        player: PlayerAst,
        mana: Vec<ManaSymbol>,
    },
    UnlessAction {
        effects: Vec<EffectAst>,
        alternative: Vec<EffectAst>,
        player: PlayerAst,
    },
    PutCounters {
        counter_type: CounterType,
        count: Value,
        target: TargetAst,
        target_count: Option<ChoiceCount>,
        distributed: bool,
    },
    PutCountersAll {
        counter_type: CounterType,
        count: Value,
        filter: ObjectFilter,
    },
    DoubleCountersOnEach {
        counter_type: CounterType,
        filter: ObjectFilter,
    },
    Proliferate,
    Tap {
        target: TargetAst,
    },
    TapAll {
        filter: ObjectFilter,
    },
    Untap {
        target: TargetAst,
    },
    TapOrUntap {
        target: TargetAst,
    },
    UntapAll {
        filter: ObjectFilter,
    },
    LoseLife {
        amount: Value,
        player: PlayerAst,
    },
    GainLife {
        amount: Value,
        player: PlayerAst,
    },
    LoseGame {
        player: PlayerAst,
    },
    PreventAllCombatDamage {
        duration: Until,
    },
    PreventAllCombatDamageFromSource {
        duration: Until,
        source: TargetAst,
    },
    PreventAllCombatDamageToPlayers {
        duration: Until,
    },
    PreventAllCombatDamageToYou {
        duration: Until,
    },
    PreventDamage {
        amount: Value,
        target: TargetAst,
        duration: Until,
    },
    PreventAllDamageToTarget {
        target: TargetAst,
        duration: Until,
    },
    PreventDamageEach {
        amount: Value,
        filter: ObjectFilter,
        duration: Until,
    },
    GrantProtectionChoice {
        target: TargetAst,
        allow_colorless: bool,
    },
    Earthbend {
        counters: u32,
    },
    Explore {
        target: TargetAst,
    },
    OpenAttraction,
    ManifestDread,
    Bolster {
        amount: u32,
    },
    Support {
        amount: u32,
    },
    Adapt {
        amount: u32,
    },
    CounterActivatedOrTriggeredAbility,
    AddMana {
        mana: Vec<ManaSymbol>,
        player: PlayerAst,
    },
    AddManaScaled {
        mana: Vec<ManaSymbol>,
        amount: Value,
        player: PlayerAst,
    },
    AddManaAnyColor {
        amount: Value,
        player: PlayerAst,
        available_colors: Option<Vec<crate::color::Color>>,
    },
    AddManaAnyOneColor {
        amount: Value,
        player: PlayerAst,
    },
    AddManaChosenColor {
        amount: Value,
        player: PlayerAst,
        fixed_option: Option<crate::color::Color>,
    },
    AddManaFromLandCouldProduce {
        amount: Value,
        player: PlayerAst,
        land_filter: ObjectFilter,
        allow_colorless: bool,
        same_type: bool,
    },
    AddManaCommanderIdentity {
        amount: Value,
        player: PlayerAst,
    },
    AddManaImprintedColors,
    Scry {
        count: Value,
        player: PlayerAst,
    },
    Surveil {
        count: Value,
        player: PlayerAst,
    },
    PayMana {
        cost: ManaCost,
        player: PlayerAst,
    },
    PayEnergy {
        amount: Value,
        player: PlayerAst,
    },
    Cant {
        restriction: crate::effect::Restriction,
        duration: crate::effect::Until,
    },
    PlayFromGraveyardUntilEot {
        player: PlayerAst,
    },
    ExileInsteadOfGraveyardThisTurn {
        player: PlayerAst,
    },
    GainControl {
        target: TargetAst,
        duration: Until,
    },
    ControlPlayer {
        player: PlayerFilter,
        duration: ControlDurationAst,
    },
    ExtraTurnAfterTurn {
        player: PlayerAst,
    },
    DelayedUntilNextEndStep {
        player: PlayerFilter,
        effects: Vec<EffectAst>,
    },
    DelayedUntilEndStepOfExtraTurn {
        player: PlayerAst,
        effects: Vec<EffectAst>,
    },
    DelayedUntilEndOfCombat {
        effects: Vec<EffectAst>,
    },
    DelayedWhenLastObjectDiesThisTurn {
        effects: Vec<EffectAst>,
    },
    RevealTop {
        player: PlayerAst,
    },
    LookAtTopCards {
        player: PlayerAst,
        count: u32,
        tag: TagKey,
    },
    RevealHand {
        player: PlayerAst,
    },
    PutIntoHand {
        player: PlayerAst,
        object: ObjectRefAst,
    },
    CopySpell {
        target: TargetAst,
        count: Value,
        player: PlayerAst,
        may_choose_new_targets: bool,
    },
    Conditional {
        predicate: PredicateAst,
        if_true: Vec<EffectAst>,
        if_false: Vec<EffectAst>,
    },
    ChooseObjects {
        filter: ObjectFilter,
        count: ChoiceCount,
        player: PlayerAst,
        tag: TagKey,
    },
    Sacrifice {
        filter: ObjectFilter,
        player: PlayerAst,
        count: u32,
    },
    SacrificeAll {
        filter: ObjectFilter,
        player: PlayerAst,
    },
    DiscardHand {
        player: PlayerAst,
    },
    Discard {
        count: Value,
        player: PlayerAst,
        random: bool,
    },
    Connive {
        target: TargetAst,
    },
    ConniveIterated,
    Goad {
        target: TargetAst,
    },
    Transform {
        target: TargetAst,
    },
    Regenerate {
        target: TargetAst,
    },
    RegenerateAll {
        filter: ObjectFilter,
    },
    Mill {
        count: Value,
        player: PlayerAst,
    },
    ReturnToHand {
        target: TargetAst,
    },
    ReturnToBattlefield {
        target: TargetAst,
        tapped: bool,
        controller: ReturnControllerAst,
    },
    MoveToZone {
        target: TargetAst,
        zone: Zone,
        to_top: bool,
        battlefield_controller: ReturnControllerAst,
    },
    ReturnAllToHand {
        filter: ObjectFilter,
    },
    ReturnAllToBattlefield {
        filter: ObjectFilter,
        tapped: bool,
    },
    ExchangeControl {
        filter: ObjectFilter,
        count: u32,
    },
    SetLifeTotal {
        amount: Value,
        player: PlayerAst,
    },
    SkipTurn {
        player: PlayerAst,
    },
    SkipCombatPhases {
        player: PlayerAst,
    },
    SkipNextCombatPhaseThisTurn {
        player: PlayerAst,
    },
    SkipDrawStep {
        player: PlayerAst,
    },
    PoisonCounters {
        count: Value,
        player: PlayerAst,
    },
    EnergyCounters {
        count: Value,
        player: PlayerAst,
    },
    May {
        effects: Vec<EffectAst>,
    },
    MayByPlayer {
        player: PlayerAst,
        effects: Vec<EffectAst>,
    },
    MayByTaggedController {
        tag: TagKey,
        effects: Vec<EffectAst>,
    },
    IfResult {
        predicate: IfResultPredicate,
        effects: Vec<EffectAst>,
    },
    ForEachOpponent {
        effects: Vec<EffectAst>,
    },
    ForEachPlayer {
        effects: Vec<EffectAst>,
    },
    ForEachObject {
        filter: ObjectFilter,
        effects: Vec<EffectAst>,
    },
    ForEachTagged {
        tag: TagKey,
        effects: Vec<EffectAst>,
    },
    ForEachOpponentDoesNot {
        effects: Vec<EffectAst>,
    },
    ForEachPlayerDoesNot {
        effects: Vec<EffectAst>,
    },
    ForEachOpponentDid {
        effects: Vec<EffectAst>,
    },
    ForEachPlayerDid {
        effects: Vec<EffectAst>,
    },
    ForEachTaggedPlayer {
        tag: TagKey,
        effects: Vec<EffectAst>,
    },
    Enchant {
        filter: ObjectFilter,
    },
    Attach {
        object: TargetAst,
        target: TargetAst,
    },
    Investigate,
    Destroy {
        target: TargetAst,
    },
    DestroyAll {
        filter: ObjectFilter,
    },
    DestroyAllAttachedTo {
        filter: ObjectFilter,
        target: TargetAst,
    },
    Exile {
        target: TargetAst,
    },
    ExileUntilSourceLeaves {
        target: TargetAst,
    },
    ExileAll {
        filter: ObjectFilter,
    },
    LookAtHand {
        target: TargetAst,
    },
    TargetOnly {
        target: TargetAst,
    },
    #[allow(dead_code)]
    CreateToken {
        name: String,
        count: Value,
        player: PlayerAst,
    },
    CreateTokenCopy {
        object: ObjectRefAst,
        count: Value,
        player: PlayerAst,
        half_power_toughness_round_up: bool,
        has_haste: bool,
        sacrifice_at_next_end_step: bool,
        exile_at_next_end_step: bool,
        added_card_types: Vec<CardType>,
        added_subtypes: Vec<Subtype>,
        removed_supertypes: Vec<Supertype>,
        set_base_power_toughness: Option<(i32, i32)>,
        granted_abilities: Vec<StaticAbility>,
    },
    CreateTokenCopyFromSource {
        source: TargetAst,
        count: Value,
        player: PlayerAst,
        half_power_toughness_round_up: bool,
        has_haste: bool,
        sacrifice_at_next_end_step: bool,
        exile_at_next_end_step: bool,
        added_card_types: Vec<CardType>,
        added_subtypes: Vec<Subtype>,
        removed_supertypes: Vec<Supertype>,
        set_base_power_toughness: Option<(i32, i32)>,
        granted_abilities: Vec<StaticAbility>,
    },
    CreateTokenWithMods {
        name: String,
        count: Value,
        player: PlayerAst,
        tapped: bool,
        attacking: bool,
        exile_at_end_of_combat: bool,
    },
    ExileThatTokenAtEndOfCombat,
    Monstrosity {
        amount: Value,
    },
    RemoveUpToAnyCounters {
        amount: Value,
        target: TargetAst,
    },
    MoveAllCounters {
        from: TargetAst,
        to: TargetAst,
    },
    Pump {
        power: Value,
        toughness: Value,
        target: TargetAst,
        duration: Until,
    },
    SetBasePowerToughness {
        power: Value,
        toughness: Value,
        target: TargetAst,
        duration: Until,
    },
    SetBasePower {
        power: Value,
        target: TargetAst,
        duration: Until,
    },
    PumpForEach {
        power_per: i32,
        toughness_per: i32,
        target: TargetAst,
        count_filter: ObjectFilter,
        duration: Until,
    },
    PumpAll {
        filter: ObjectFilter,
        power: Value,
        toughness: Value,
        duration: Until,
    },
    PumpByLastEffect {
        power: i32,
        toughness: i32,
        target: TargetAst,
        duration: Until,
    },
    GrantAbilitiesAll {
        filter: ObjectFilter,
        abilities: Vec<StaticAbility>,
        duration: Until,
    },
    GrantAbilitiesToTarget {
        target: TargetAst,
        abilities: Vec<StaticAbility>,
        duration: Until,
    },
    GrantAbilityToSource {
        ability: Ability,
    },
    SearchLibrary {
        filter: ObjectFilter,
        destination: Zone,
        player: PlayerAst,
        reveal: bool,
        shuffle: bool,
        count: ChoiceCount,
        tapped: bool,
    },
    ShuffleGraveyardIntoLibrary {
        player: PlayerAst,
    },
    #[allow(dead_code)]
    ShuffleLibrary {
        player: PlayerAst,
    },
    VoteStart {
        options: Vec<String>,
    },
    VoteOption {
        option: String,
        effects: Vec<EffectAst>,
    },
    VoteExtra {
        count: u32,
        optional: bool,
    },
    TokenCopyGainHasteUntilEot,
    TokenCopySacrificeAtNextEndStep,
    TokenCopyExileAtNextEndStep,
}

#[derive(Debug, Clone, Default)]
pub struct ParseAnnotations {
    pub tag_spans: HashMap<TagKey, Vec<TextSpan>>,
    pub normalized_lines: HashMap<usize, String>,
    pub original_lines: HashMap<usize, String>,
    pub normalized_char_maps: HashMap<usize, Vec<usize>>,
}

impl ParseAnnotations {
    fn record_tag_span(&mut self, tag: &TagKey, span: TextSpan) {
        self.tag_spans.entry(tag.clone()).or_default().push(span);
    }

    fn record_normalized_line(&mut self, line_index: usize, line: &str) {
        self.normalized_lines
            .entry(line_index)
            .or_insert_with(|| line.to_string());
    }

    fn record_original_line(&mut self, line_index: usize, line: &str) {
        self.original_lines
            .entry(line_index)
            .or_insert_with(|| line.to_string());
    }

    fn record_char_map(&mut self, line_index: usize, map: Vec<usize>) {
        self.normalized_char_maps.entry(line_index).or_insert(map);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IfResultPredicate {
    Did,
    DidNot,
    DiesThisWay,
}

const IT_TAG: &str = "__it__";

include!("builders/parse_parsing.rs");

include!("builders/parse_compile.rs");

/// Builder for creating CardDefinitions with abilities.
#[derive(Debug, Clone)]
pub struct CardDefinitionBuilder {
    /// The underlying card builder
    card_builder: CardBuilder,

    /// Abilities to add to the card
    abilities: Vec<Ability>,

    /// Spell effects for instants/sorceries
    spell_effect: Option<Vec<Effect>>,

    /// Alternative casting methods (flashback, escape, etc.)
    alternative_casts: Vec<AlternativeCastingMethod>,

    /// Optional costs (kicker, buyback, etc.)
    optional_costs: Vec<OptionalCost>,

    /// For sagas: the maximum chapter number
    max_saga_chapter: Option<u32>,

    /// Cost effects (new unified model) - effects that are executed as part of paying costs.
    cost_effects: Vec<Effect>,

    /// For Auras: what this card can enchant (used for non-target attachments)
    aura_attach_filter: Option<ObjectFilter>,
}

impl CardDefinitionBuilder {
    fn pt_value_text(value: PtValue) -> String {
        match value {
            PtValue::Fixed(n) => n.to_string(),
            PtValue::Star => "*".to_string(),
            PtValue::StarPlus(n) => {
                if n >= 0 {
                    format!("*+{n}")
                } else {
                    format!("*{n}")
                }
            }
        }
    }

    fn type_line_text(
        supertypes: &[Supertype],
        card_types: &[CardType],
        subtypes: &[Subtype],
    ) -> Option<String> {
        if supertypes.is_empty() && card_types.is_empty() && subtypes.is_empty() {
            return None;
        }

        let mut left = Vec::new();
        for supertype in supertypes {
            left.push(format!("{:?}", supertype));
        }
        for card_type in card_types {
            left.push(format!("{:?}", card_type));
        }

        let mut line = left.join(" ");
        if !subtypes.is_empty() {
            let right = subtypes
                .iter()
                .map(|subtype| format!("{:?}", subtype))
                .collect::<Vec<_>>()
                .join(" ");
            if !line.is_empty() {
                line.push_str(" — ");
            }
            line.push_str(&right);
        }
        Some(line)
    }

    fn build_text_with_metadata(&self, rules: &str) -> String {
        let mut lines = Vec::new();
        if let Some(cost) = self.card_builder.mana_cost_ref() {
            lines.push(format!("Mana cost: {}", cost.to_oracle()));
        }
        if let Some(type_line) = Self::type_line_text(
            self.card_builder.supertypes_ref(),
            self.card_builder.card_types_ref(),
            self.card_builder.subtypes_ref(),
        ) {
            lines.push(format!("Type: {type_line}"));
        }
        if let Some(pt) = self.card_builder.power_toughness_ref() {
            lines.push(format!(
                "Power/Toughness: {}/{}",
                Self::pt_value_text(pt.power),
                Self::pt_value_text(pt.toughness)
            ));
        }
        if let Some(loyalty) = self.card_builder.loyalty_ref() {
            lines.push(format!("Loyalty: {loyalty}"));
        }
        if let Some(defense) = self.card_builder.defense_ref() {
            lines.push(format!("Defense: {defense}"));
        }

        if !rules.trim().is_empty() {
            lines.push(rules.trim().to_string());
        }

        lines.join("\n")
    }

    /// Create a new card definition builder.
    pub fn new(id: CardId, name: impl Into<String>) -> Self {
        Self {
            card_builder: CardBuilder::new(id, name),
            abilities: Vec::new(),
            spell_effect: None,
            alternative_casts: Vec::new(),
            optional_costs: Vec::new(),
            max_saga_chapter: None,
            cost_effects: Vec::new(),
            aura_attach_filter: None,
        }
    }

    // === Card properties (delegated to CardBuilder) ===

    /// Set the mana cost.
    pub fn mana_cost(mut self, cost: ManaCost) -> Self {
        self.card_builder = self.card_builder.mana_cost(cost);
        self
    }

    /// Set the color indicator.
    pub fn color_indicator(mut self, colors: ColorSet) -> Self {
        self.card_builder = self.card_builder.color_indicator(colors);
        self
    }

    /// Set the supertypes.
    pub fn supertypes(mut self, supertypes: Vec<Supertype>) -> Self {
        self.card_builder = self.card_builder.supertypes(supertypes);
        self
    }

    /// Set the card types.
    pub fn card_types(mut self, types: Vec<CardType>) -> Self {
        self.card_builder = self.card_builder.card_types(types);
        self
    }

    /// Set the subtypes.
    pub fn subtypes(mut self, subtypes: Vec<Subtype>) -> Self {
        self.card_builder = self.card_builder.subtypes(subtypes);
        self
    }

    /// Set the oracle text.
    pub fn oracle_text(mut self, text: impl Into<String>) -> Self {
        self.card_builder = self.card_builder.oracle_text(text);
        self
    }

    fn apply_keyword_action(self, action: KeywordAction) -> Self {
        match action {
            KeywordAction::Flying => self.flying(),
            KeywordAction::Menace => self.menace(),
            KeywordAction::Hexproof => self.hexproof(),
            KeywordAction::Haste => self.haste(),
            KeywordAction::Improvise => self.improvise(),
            KeywordAction::Convoke => self.convoke(),
            KeywordAction::AffinityForArtifacts => self.affinity_for_artifacts(),
            KeywordAction::Delve => self.delve(),
            KeywordAction::FirstStrike => self.first_strike(),
            KeywordAction::DoubleStrike => self.double_strike(),
            KeywordAction::Deathtouch => self.deathtouch(),
            KeywordAction::Lifelink => self.lifelink(),
            KeywordAction::Vigilance => self.vigilance(),
            KeywordAction::Trample => self.trample(),
            KeywordAction::Reach => self.reach(),
            KeywordAction::Defender => self.defender(),
            KeywordAction::Flash => self.flash(),
            KeywordAction::Phasing => {
                self.with_ability(Ability::static_ability(StaticAbility::phasing()))
            }
            KeywordAction::Indestructible => self.indestructible(),
            KeywordAction::Shroud => self.shroud(),
            KeywordAction::Ward(amount) => self.ward_generic(amount),
            KeywordAction::Wither => self.wither(),
            KeywordAction::Infect => self.infect(),
            KeywordAction::Undying => self.undying(),
            KeywordAction::Persist => self.persist(),
            KeywordAction::Prowess => self.prowess(),
            KeywordAction::Exalted => self.exalted(),
            KeywordAction::Storm => self.storm(),
            KeywordAction::Toxic(amount) => self.toxic(amount),
            KeywordAction::Fear => self.fear(),
            KeywordAction::Intimidate => self.intimidate(),
            KeywordAction::Shadow => self.shadow(),
            KeywordAction::Horsemanship => self.horsemanship(),
            KeywordAction::Flanking => {
                self.with_ability(Ability::static_ability(StaticAbility::flanking()))
            }
            KeywordAction::Landwalk(subtype) => {
                self.with_ability(Ability::static_ability(StaticAbility::landwalk(subtype)))
            }
            KeywordAction::Bloodthirst(amount) => self.bloodthirst(amount),
            KeywordAction::Rampage(amount) => self.rampage(amount),
            KeywordAction::Bushido(amount) => self.bushido(amount),
            KeywordAction::Changeling => {
                self.with_ability(Ability::static_ability(StaticAbility::changeling()))
            }
            KeywordAction::ProtectionFrom(colors) => self.protection_from(colors),
            KeywordAction::ProtectionFromAllColors => self.with_ability(Ability::static_ability(
                StaticAbility::protection(crate::ability::ProtectionFrom::AllColors),
            )),
            KeywordAction::ProtectionFromColorless => self.with_ability(Ability::static_ability(
                StaticAbility::protection(crate::ability::ProtectionFrom::Colorless),
            )),
            KeywordAction::ProtectionFromCardType(card_type) => {
                self.protection_from_card_type(card_type)
            }
            KeywordAction::ProtectionFromSubtype(subtype) => self.protection_from_subtype(subtype),
            KeywordAction::Unblockable => self.unblockable(),
            KeywordAction::Devoid => self.with_ability(
                Ability::static_ability(StaticAbility::make_colorless(ObjectFilter::source()))
                    .in_zones(vec![
                        Zone::Battlefield,
                        Zone::Stack,
                        Zone::Hand,
                        Zone::Library,
                        Zone::Graveyard,
                        Zone::Exile,
                        Zone::Command,
                    ]),
            ),
            KeywordAction::Marker(name) => self.with_ability(Ability::static_ability(
                StaticAbility::custom(name, name.to_string()),
            )),
            KeywordAction::MarkerText(text) => self.with_ability(Ability::static_ability(
                StaticAbility::custom("keyword_marker", text),
            )),
        }
    }

    /// Build a CardDefinition from oracle text.
    pub fn parse_text(self, text: impl Into<String>) -> Result<CardDefinition, CardTextError> {
        let (definition, _) = self.parse_text_with_annotations(text)?;
        Ok(definition)
    }

    /// Build a CardDefinition from oracle text, returning parse annotations.
    pub fn parse_text_with_annotations(
        self,
        text: impl Into<String>,
    ) -> Result<(CardDefinition, ParseAnnotations), CardTextError> {
        parser::parse_text_with_annotations(self, text.into())
    }

    /// Build a CardDefinition from oracle text, prepending metadata lines
    /// derived from the builder's current fields (mana cost, type line, etc.).
    pub fn from_text_with_metadata(
        self,
        text: impl Into<String>,
    ) -> Result<CardDefinition, CardTextError> {
        let rules = text.into();
        let combined = self.build_text_with_metadata(rules.as_str());

        let parse_builder = self.clone();
        let mut parse_builder = parse_builder;
        parse_builder.cost_effects.clear();
        parse_builder.parse_text(combined)
    }

    /// Backwards-compatible wrapper for prepending metadata to rules text.
    pub fn text_box(self, text: impl Into<String>) -> Result<CardDefinition, CardTextError> {
        let rules = text.into();
        let combined = self.build_text_with_metadata(rules.as_str());

        // Treat the text box as authoritative: drop any previously added abilities if parsing succeeds.
        let mut parse_builder = self.clone();
        parse_builder.abilities.clear();
        parse_builder.cost_effects.clear();
        parse_builder.parse_text(combined)
    }

    /// Build a CardDefinition from oracle text with metadata, without parsing rules text.
    /// Useful for cards with custom/manual abilities where parsing may be incomplete.
    pub fn from_text_with_metadata_oracle_only(self, text: impl Into<String>) -> CardDefinition {
        fn pt_value_text(value: PtValue) -> String {
            match value {
                PtValue::Fixed(n) => n.to_string(),
                PtValue::Star => "*".to_string(),
                PtValue::StarPlus(n) => {
                    if n >= 0 {
                        format!("*+{n}")
                    } else {
                        format!("*{n}")
                    }
                }
            }
        }

        fn type_line_text(
            supertypes: &[Supertype],
            card_types: &[CardType],
            subtypes: &[Subtype],
        ) -> Option<String> {
            if supertypes.is_empty() && card_types.is_empty() && subtypes.is_empty() {
                return None;
            }

            let mut left = Vec::new();
            for supertype in supertypes {
                left.push(format!("{:?}", supertype));
            }
            for card_type in card_types {
                left.push(format!("{:?}", card_type));
            }

            let mut line = left.join(" ");
            if !subtypes.is_empty() {
                let right = subtypes
                    .iter()
                    .map(|subtype| format!("{:?}", subtype))
                    .collect::<Vec<_>>()
                    .join(" ");
                if !line.is_empty() {
                    line.push_str(" — ");
                }
                line.push_str(&right);
            }
            Some(line)
        }

        let mut lines = Vec::new();
        if let Some(cost) = self.card_builder.mana_cost_ref() {
            lines.push(format!("Mana cost: {}", cost.to_oracle()));
        }
        if let Some(type_line) = type_line_text(
            self.card_builder.supertypes_ref(),
            self.card_builder.card_types_ref(),
            self.card_builder.subtypes_ref(),
        ) {
            lines.push(format!("Type: {type_line}"));
        }
        if let Some(pt) = self.card_builder.power_toughness_ref() {
            lines.push(format!(
                "Power/Toughness: {}/{}",
                pt_value_text(pt.power),
                pt_value_text(pt.toughness)
            ));
        }
        if let Some(loyalty) = self.card_builder.loyalty_ref() {
            lines.push(format!("Loyalty: {loyalty}"));
        }
        if let Some(defense) = self.card_builder.defense_ref() {
            lines.push(format!("Defense: {defense}"));
        }

        let rules = text.into();
        if !rules.trim().is_empty() {
            lines.push(rules.trim().to_string());
        }

        let combined = lines.join("\n");
        self.oracle_text(combined).build()
    }

    fn apply_metadata(mut self, meta: MetadataLine) -> Result<Self, CardTextError> {
        match meta {
            MetadataLine::ManaCost(raw) => {
                let cost = parse_scryfall_mana_cost(&raw)?;
                if !cost.is_empty() {
                    self.card_builder = self.card_builder.mana_cost(cost);
                }
            }
            MetadataLine::TypeLine(raw) => {
                let (supertypes, card_types, subtypes) = parse_type_line(&raw)?;
                if !supertypes.is_empty() {
                    self.card_builder = self.card_builder.supertypes(supertypes);
                }
                if !card_types.is_empty() {
                    self.card_builder = self.card_builder.card_types(card_types);
                }
                if !subtypes.is_empty() {
                    self.card_builder = self.card_builder.subtypes(subtypes);
                }
            }
            MetadataLine::PowerToughness(raw) => {
                if let Some(pt) = parse_power_toughness(&raw) {
                    self.card_builder = self.card_builder.power_toughness(pt);
                }
            }
            MetadataLine::Loyalty(raw) => {
                if let Ok(value) = raw.trim().parse::<u32>() {
                    self.card_builder = self.card_builder.loyalty(value);
                }
            }
            MetadataLine::Defense(raw) => {
                if let Ok(value) = raw.trim().parse::<u32>() {
                    self.card_builder = self.card_builder.defense(value);
                }
            }
        }

        Ok(self)
    }

    /// Set the power/toughness.
    pub fn power_toughness(mut self, pt: PowerToughness) -> Self {
        self.card_builder = self.card_builder.power_toughness(pt);
        self
    }

    /// Set the starting loyalty.
    pub fn loyalty(mut self, loyalty: u32) -> Self {
        self.card_builder = self.card_builder.loyalty(loyalty);
        self
    }

    /// Set the defense value.
    pub fn defense(mut self, defense: u32) -> Self {
        self.card_builder = self.card_builder.defense(defense);
        self
    }

    /// Mark this card as a token.
    ///
    /// Tokens are not real cards - they are created by effects and cease to exist
    /// when they leave the battlefield.
    pub fn token(mut self) -> Self {
        self.card_builder = self.card_builder.token();
        self
    }

    // === Ability methods ===

    /// Add abilities to the card.
    pub fn with_abilities(mut self, abilities: Vec<Ability>) -> Self {
        self.abilities.extend(abilities);
        self
    }

    /// Add a single ability to the card.
    pub fn with_ability(mut self, ability: Ability) -> Self {
        self.abilities.push(ability);
        self
    }

    // === Keyword shortcuts ===

    /// Add flying.
    pub fn flying(self) -> Self {
        self.with_ability(ability::flying())
    }

    /// Add first strike.
    pub fn first_strike(self) -> Self {
        self.with_ability(ability::first_strike())
    }

    /// Add double strike.
    pub fn double_strike(self) -> Self {
        self.with_ability(ability::double_strike())
    }

    /// Add deathtouch.
    pub fn deathtouch(self) -> Self {
        self.with_ability(ability::deathtouch())
    }

    /// Add lifelink.
    pub fn lifelink(self) -> Self {
        self.with_ability(ability::lifelink())
    }

    /// Add vigilance.
    pub fn vigilance(self) -> Self {
        self.with_ability(ability::vigilance())
    }

    /// Add trample.
    pub fn trample(self) -> Self {
        self.with_ability(ability::trample())
    }

    /// Add haste.
    pub fn haste(self) -> Self {
        self.with_ability(ability::haste())
    }

    /// Add reach.
    pub fn reach(self) -> Self {
        self.with_ability(ability::reach())
    }

    /// Add defender.
    pub fn defender(self) -> Self {
        self.with_ability(ability::defender())
    }

    /// Add hexproof.
    pub fn hexproof(self) -> Self {
        self.with_ability(ability::hexproof())
    }

    /// Add ward with a mana cost.
    ///
    /// Ward is a triggered ability that counters spells or abilities that target
    /// this permanent unless the opponent pays the ward cost.
    ///
    /// Example: `ward(TotalCost::mana("{3}"))` for "Ward {3}"
    pub fn ward(self, cost: TotalCost) -> Self {
        self.with_ability(Ability::static_ability(StaticAbility::ward(cost)).with_text("Ward"))
    }

    /// Add ward with a generic mana cost.
    ///
    /// Convenience method for the common case of ward with just generic mana.
    /// Example: `ward_generic(3)` for "Ward {3}"
    pub fn ward_generic(self, amount: u32) -> Self {
        use crate::mana::{ManaCost, ManaSymbol};
        let mana = ManaCost::from_pips(vec![vec![ManaSymbol::Generic(amount as u8)]]);
        self.ward(TotalCost::mana(mana))
    }

    /// Add indestructible.
    pub fn indestructible(self) -> Self {
        self.with_ability(ability::indestructible())
    }

    /// Add menace.
    pub fn menace(self) -> Self {
        self.with_ability(ability::menace())
    }

    /// Add flash.
    pub fn flash(self) -> Self {
        self.with_ability(ability::flash())
    }

    /// Add shroud.
    pub fn shroud(self) -> Self {
        self.with_ability(Ability::static_ability(StaticAbility::shroud()).with_text("Shroud"))
    }

    /// Add wither.
    pub fn wither(self) -> Self {
        self.with_ability(Ability::static_ability(StaticAbility::wither()).with_text("Wither"))
    }

    /// Add infect.
    pub fn infect(self) -> Self {
        self.with_ability(Ability::static_ability(StaticAbility::infect()).with_text("Infect"))
    }

    /// Add undying.
    ///
    /// Undying is a triggered ability: "When this creature dies, if it had no +1/+1
    /// counters on it, return it to the battlefield under its owner's control with
    /// a +1/+1 counter on it."
    pub fn undying(self) -> Self {
        use crate::effect::Effect;
        use crate::object::CounterType;
        use crate::target::{ChooseSpec, ObjectFilter, PlayerFilter};
        use crate::triggers::Trigger;
        use crate::zone::Zone;

        let trigger_tag = "undying_trigger";
        let return_tag = "undying_return";
        let returned_tag = "undying_returned";

        let filter = ObjectFilter::default()
            .in_zone(Zone::Graveyard)
            .same_stable_id_as_tagged(trigger_tag);

        let choose = Effect::choose_objects(filter, 1, PlayerFilter::You, return_tag);
        let move_to_battlefield = Effect::move_to_zone(
            ChooseSpec::Tagged(return_tag.into()),
            Zone::Battlefield,
            true,
        )
        .tag(returned_tag);
        let counters = Effect::for_each_tagged(
            returned_tag,
            vec![Effect::put_counters(
                CounterType::PlusOnePlusOne,
                1,
                ChooseSpec::Iterated,
            )],
        );
        let effects = vec![
            Effect::tag_triggering_object(trigger_tag),
            choose,
            move_to_battlefield,
            counters,
        ];
        self.with_ability(Ability {
            kind: AbilityKind::Triggered(TriggeredAbility {
                trigger: Trigger::undying(),
                effects,
                choices: vec![],
                intervening_if: None,
                once_each_turn: false,
            }),
            // Functions from both zones because triggers can be checked at different points:
            // - From Battlefield: SBAs check triggers BEFORE moving object to graveyard
            // - From Graveyard: Sacrifices check triggers AFTER moving object
            functional_zones: vec![crate::zone::Zone::Battlefield, crate::zone::Zone::Graveyard],
            text: Some("Undying".to_string()),
        })
    }

    /// Add persist.
    ///
    /// Persist is a triggered ability: "When this creature dies, if it had no -1/-1
    /// counters on it, return it to the battlefield under its owner's control with
    /// a -1/-1 counter on it."
    pub fn persist(self) -> Self {
        use crate::effect::Effect;
        use crate::object::CounterType;
        use crate::target::{ChooseSpec, ObjectFilter, PlayerFilter};
        use crate::triggers::Trigger;
        use crate::zone::Zone;

        let trigger_tag = "persist_trigger";
        let return_tag = "persist_return";
        let returned_tag = "persist_returned";

        let filter = ObjectFilter::default()
            .in_zone(Zone::Graveyard)
            .same_stable_id_as_tagged(trigger_tag);

        let choose = Effect::choose_objects(filter, 1, PlayerFilter::You, return_tag);
        let move_to_battlefield = Effect::move_to_zone(
            ChooseSpec::Tagged(return_tag.into()),
            Zone::Battlefield,
            true,
        )
        .tag(returned_tag);
        let counters = Effect::for_each_tagged(
            returned_tag,
            vec![Effect::put_counters(
                CounterType::MinusOneMinusOne,
                1,
                ChooseSpec::Iterated,
            )],
        );
        let effects = vec![
            Effect::tag_triggering_object(trigger_tag),
            choose,
            move_to_battlefield,
            counters,
        ];
        self.with_ability(Ability {
            kind: AbilityKind::Triggered(TriggeredAbility {
                trigger: Trigger::persist(),
                effects,
                choices: vec![],
                intervening_if: None,
                once_each_turn: false,
            }),
            // Functions from both zones because triggers can be checked at different points:
            // - From Battlefield: SBAs check triggers BEFORE moving object to graveyard
            // - From Graveyard: Sacrifices check triggers AFTER moving object
            functional_zones: vec![crate::zone::Zone::Battlefield, crate::zone::Zone::Graveyard],
            text: Some("Persist".to_string()),
        })
    }

    /// Add prowess.
    ///
    /// Prowess means "Whenever you cast a noncreature spell, this creature gets +1/+1 until
    /// end of turn."
    pub fn prowess(self) -> Self {
        self.with_ability(
            Ability::triggered(
                Trigger::spell_cast(Some(ObjectFilter::noncreature_spell()), PlayerFilter::You),
                vec![Effect::pump(1, 1, ChooseSpec::Source, Until::EndOfTurn)],
            )
            .with_text("Prowess"),
        )
    }

    /// Add exalted.
    ///
    /// Exalted means "Whenever a creature you control attacks alone, that creature gets +1/+1
    /// until end of turn."
    pub fn exalted(self) -> Self {
        let attacker_tag = "exalted_attacker";
        self.with_ability(
            Ability::triggered(
                Trigger::attacks_alone(ObjectFilter::creature().you_control()),
                vec![
                    Effect::tag_triggering_object(attacker_tag),
                    Effect::pump(
                        1,
                        1,
                        ChooseSpec::Tagged(attacker_tag.into()),
                        Until::EndOfTurn,
                    ),
                ],
            )
            .with_text("Exalted"),
        )
    }

    /// Add toxic N.
    ///
    /// Toxic N means "Players dealt combat damage by this creature also get N poison counters."
    pub fn toxic(self, amount: u32) -> Self {
        let text = format!("Toxic {amount}");
        self.with_ability(
            Ability::triggered(
                Trigger::this_deals_combat_damage_to_player(),
                vec![Effect::poison_counters_player(
                    amount as i32,
                    PlayerFilter::DamagedPlayer,
                )],
            )
            .with_text(&text),
        )
    }

    /// Add storm.
    ///
    /// Storm means "When you cast this spell, copy it for each spell cast before it this turn."
    pub fn storm(self) -> Self {
        self.with_ability(Ability {
            kind: AbilityKind::Triggered(TriggeredAbility {
                trigger: Trigger::you_cast_this_spell(),
                effects: vec![
                    Effect::with_id(
                        0,
                        Effect::copy_spell_n(
                            ChooseSpec::Source,
                            Value::SpellsCastBeforeThisTurn(PlayerFilter::You),
                        ),
                    ),
                    Effect::may_choose_new_targets(EffectId(0)),
                ],
                choices: vec![],
                intervening_if: None,
                once_each_turn: false,
            }),
            functional_zones: vec![Zone::Stack],
            text: Some("Storm".to_string()),
        })
    }

    /// Add fear.
    pub fn fear(self) -> Self {
        self.with_ability(Ability::static_ability(StaticAbility::fear()).with_text("Fear"))
    }

    /// Add intimidate.
    pub fn intimidate(self) -> Self {
        self.with_ability(
            Ability::static_ability(StaticAbility::intimidate()).with_text("Intimidate"),
        )
    }

    /// Add shadow.
    pub fn shadow(self) -> Self {
        self.with_ability(Ability::static_ability(StaticAbility::shadow()).with_text("Shadow"))
    }

    /// Add horsemanship.
    pub fn horsemanship(self) -> Self {
        self.with_ability(
            Ability::static_ability(StaticAbility::horsemanship()).with_text("Horsemanship"),
        )
    }

    /// Add bushido N.
    ///
    /// Bushido means "Whenever this creature blocks or becomes blocked, it gets +N/+N until
    /// end of turn."
    pub fn bushido(self, amount: u32) -> Self {
        use crate::effect::Until;
        let text = format!("Bushido {amount}");
        self.with_ability(
            Ability::triggered(
                Trigger::this_blocks_or_becomes_blocked(),
                vec![Effect::pump(
                    amount,
                    amount,
                    ChooseSpec::Source,
                    Until::EndOfTurn,
                )],
            )
            .with_text(&text),
        )
    }

    /// Add bloodthirst N.
    ///
    /// Bloodthirst means "If an opponent was dealt damage this turn, this creature enters
    /// the battlefield with N +1/+1 counters on it."
    pub fn bloodthirst(self, amount: u32) -> Self {
        let text = format!("Bloodthirst {amount}");
        self.with_ability(
            Ability::static_ability(StaticAbility::bloodthirst(amount)).with_text(&text),
        )
    }

    /// Add rampage N.
    ///
    /// Rampage means "Whenever this creature becomes blocked, it gets +N/+N until end of turn
    /// for each creature blocking it beyond the first."
    pub fn rampage(self, amount: u32) -> Self {
        let text = format!("Rampage {amount}");
        self.with_ability(
            Ability::triggered(
                Trigger::this_becomes_blocked(),
                vec![Effect::pump(
                    Value::EventValue(EventValueSpec::BlockersBeyondFirst {
                        multiplier: amount as i32,
                    }),
                    Value::EventValue(EventValueSpec::BlockersBeyondFirst {
                        multiplier: amount as i32,
                    }),
                    ChooseSpec::Source,
                    Until::EndOfTurn,
                )],
            )
            .with_text(&text),
        )
    }

    /// Add unblockable (can't be blocked).
    pub fn unblockable(self) -> Self {
        self.with_ability(
            Ability::static_ability(StaticAbility::unblockable())
                .with_text("This creature can't be blocked."),
        )
    }

    /// Add "may assign combat damage as though unblocked" (Thorn Elemental ability).
    pub fn may_assign_damage_as_unblocked(self) -> Self {
        self.with_ability(
            Ability::static_ability(StaticAbility::may_assign_damage_as_unblocked())
                .with_text("You may have ~ assign its combat damage as though it weren't blocked."),
        )
    }

    /// Add "shuffle into library from graveyard" (Darksteel Colossus ability).
    pub fn shuffle_into_library_from_graveyard(self) -> Self {
        use crate::zone::Zone;
        self.with_ability(
            Ability::static_ability(StaticAbility::shuffle_into_library_from_graveyard())
                .in_zones(vec![
                    Zone::Battlefield,
                    Zone::Stack,
                    Zone::Hand,
                    Zone::Library,
                    Zone::Graveyard,
                    Zone::Exile,
                ])
                .with_text("If ~ would be put into a graveyard from anywhere, reveal it and shuffle it into its owner's library instead."),
        )
    }

    // === Cost Modifier Abilities ===

    /// Add affinity for artifacts (cost reduction based on artifacts you control).
    pub fn affinity_for_artifacts(self) -> Self {
        self.with_ability(
            Ability::static_ability(StaticAbility::affinity_for_artifacts())
                .with_text("Affinity for artifacts"),
        )
    }

    /// Add delve (exile cards from graveyard to pay generic mana).
    pub fn delve(self) -> Self {
        self.with_ability(Ability::static_ability(StaticAbility::delve()).with_text("Delve"))
    }

    /// Add convoke (tap creatures to help pay for this spell).
    pub fn convoke(self) -> Self {
        self.with_ability(Ability::static_ability(StaticAbility::convoke()).with_text("Convoke"))
    }

    /// Add improvise (tap artifacts to pay generic mana).
    pub fn improvise(self) -> Self {
        self.with_ability(
            Ability::static_ability(StaticAbility::improvise()).with_text("Improvise"),
        )
    }

    /// Add protection from a color.
    pub fn protection_from(self, colors: ColorSet) -> Self {
        use crate::ability::ProtectionFrom;
        self.with_ability(
            Ability::static_ability(StaticAbility::protection(ProtectionFrom::Color(colors)))
                .with_text(&format!("Protection from {:?}", colors)),
        )
    }

    /// Add protection from a card type.
    pub fn protection_from_card_type(self, card_type: CardType) -> Self {
        use crate::ability::ProtectionFrom;
        self.with_ability(
            Ability::static_ability(StaticAbility::protection(ProtectionFrom::CardType(
                card_type,
            )))
            .with_text(&format!("Protection from {:?}s", card_type)),
        )
    }

    /// Add protection from a creature subtype (e.g., "Protection from Humans").
    pub fn protection_from_subtype(self, subtype: Subtype) -> Self {
        use crate::ability::ProtectionFrom;
        self.with_ability(
            Ability::static_ability(StaticAbility::protection(ProtectionFrom::Permanents(
                ObjectFilter::default().with_subtype(subtype),
            )))
            .with_text(&format!("Protection from {:?}", subtype)),
        )
    }

    // === Triggered ability shortcuts ===

    /// Add an enters-the-battlefield trigger.
    pub fn with_etb(self, effects: Vec<Effect>) -> Self {
        self.with_ability(ability::etb_trigger(effects))
    }

    /// Add a dies trigger.
    pub fn with_dies_trigger(self, effects: Vec<Effect>) -> Self {
        self.with_ability(ability::dies_trigger(effects))
    }

    /// Add an upkeep trigger.
    pub fn with_upkeep_trigger(self, effects: Vec<Effect>) -> Self {
        self.with_ability(ability::upkeep_trigger(effects))
    }

    /// Add a custom triggered ability.
    pub fn with_trigger(self, trigger: crate::triggers::Trigger, effects: Vec<Effect>) -> Self {
        self.with_ability(Ability::triggered(trigger, effects))
    }

    /// Add a targeted ETB trigger (e.g., Snapcaster Mage).
    pub fn with_targeted_etb(
        self,
        target_spec: crate::target::ChooseSpec,
        effects: Vec<Effect>,
    ) -> Self {
        use crate::ability::{AbilityKind, TriggeredAbility};
        use crate::triggers::Trigger;
        use crate::zone::Zone;

        let ability = Ability {
            kind: AbilityKind::Triggered(TriggeredAbility {
                trigger: Trigger::this_enters_battlefield(),
                effects,
                choices: vec![target_spec],
                intervening_if: None,
                once_each_turn: false,
            }),
            functional_zones: vec![Zone::Battlefield],
            text: None,
        };
        self.with_ability(ability)
    }

    /// Add an optional triggered ability ("you may").
    pub fn with_optional_trigger(
        self,
        trigger: crate::triggers::Trigger,
        effects: Vec<Effect>,
    ) -> Self {
        use crate::ability::{AbilityKind, TriggeredAbility};
        use crate::zone::Zone;

        let ability = Ability {
            kind: AbilityKind::Triggered(TriggeredAbility {
                trigger,
                effects,
                choices: vec![],
                intervening_if: None,
                once_each_turn: false,
            }),
            functional_zones: vec![Zone::Battlefield],
            text: None,
        };
        self.with_ability(ability)
    }

    // === Activated ability shortcuts ===

    /// Add an activated ability.
    pub fn with_activated(self, cost: TotalCost, effects: Vec<Effect>) -> Self {
        self.with_ability(Ability::activated(cost, effects))
    }

    /// Add a tap ability that does something.
    pub fn with_tap_ability(self, effects: Vec<Effect>) -> Self {
        self.with_ability(Ability::activated_with_cost_effects(
            TotalCost::free(),
            vec![Effect::tap_source()],
            effects,
        ))
    }

    // === Mana ability shortcuts ===

    /// Add a mana ability that taps for a single color.
    pub fn taps_for(self, mana: ManaSymbol) -> Self {
        self.with_ability(Ability::mana(TotalCost::free(), vec![mana]))
    }

    /// Add a mana ability that taps for multiple mana.
    pub fn taps_for_mana(self, mana: Vec<ManaSymbol>) -> Self {
        self.with_ability(Ability::mana(TotalCost::free(), mana))
    }

    // === Spell effect shortcuts ===

    /// Set the spell effects (for instants/sorceries).
    pub fn with_spell_effect(mut self, effects: Vec<Effect>) -> Self {
        self.spell_effect = Some(effects);
        self
    }

    // === Alternative Casting Methods ===

    /// Add flashback with the given cost.
    pub fn flashback(mut self, cost: ManaCost) -> Self {
        self.alternative_casts
            .push(AlternativeCastingMethod::Flashback { cost });
        self
    }

    /// Add jump-start (cast from graveyard, discard a card).
    pub fn jump_start(mut self) -> Self {
        self.alternative_casts
            .push(AlternativeCastingMethod::JumpStart);
        self
    }

    /// Add escape with the given cost and exile count.
    pub fn escape(mut self, cost: ManaCost, exile_count: u32) -> Self {
        self.alternative_casts
            .push(AlternativeCastingMethod::Escape {
                cost: Some(cost),
                exile_count,
            });
        self
    }

    /// Add madness with the given cost.
    pub fn madness(mut self, cost: ManaCost) -> Self {
        self.alternative_casts
            .push(AlternativeCastingMethod::Madness { cost });
        self
    }

    /// Add miracle with the given cost.
    ///
    /// Miracle is both an alternative casting method and a triggered ability:
    /// "When you draw this card, if it's the first card you've drawn this turn,
    /// you may reveal it. If you do, you may cast it for its miracle cost."
    pub fn miracle(mut self, cost: ManaCost) -> Self {
        use crate::effect::Effect;
        use crate::triggers::Trigger;

        // Add the alternative casting method
        self.alternative_casts
            .push(AlternativeCastingMethod::Miracle { cost });

        // Add the miracle trigger
        self.with_ability(Ability {
            kind: AbilityKind::Triggered(TriggeredAbility {
                trigger: Trigger::miracle(),
                effects: vec![Effect::may_cast_for_miracle_cost()],
                choices: vec![],
                intervening_if: None,
                once_each_turn: false,
            }),
            functional_zones: vec![crate::zone::Zone::Hand], // Only triggers from hand
            text: Some("Miracle".to_string()),
        })
    }

    /// Add a custom alternative casting method.
    pub fn alternative_cast(mut self, method: AlternativeCastingMethod) -> Self {
        self.alternative_casts.push(method);
        self
    }

    // === Optional Costs (Kicker, Buyback, etc.) ===

    /// Add a kicker cost (can pay once for additional effect).
    pub fn kicker(mut self, cost: TotalCost) -> Self {
        self.optional_costs.push(OptionalCost::kicker(cost));
        self
    }

    /// Add a kicker cost using just mana.
    pub fn kicker_mana(self, cost: ManaCost) -> Self {
        self.kicker(TotalCost::mana(cost))
    }

    /// Add a multikicker cost (can pay any number of times).
    pub fn multikicker(mut self, cost: TotalCost) -> Self {
        self.optional_costs.push(OptionalCost::multikicker(cost));
        self
    }

    /// Add a multikicker cost using just mana.
    pub fn multikicker_mana(self, cost: ManaCost) -> Self {
        self.multikicker(TotalCost::mana(cost))
    }

    /// Add a buyback cost (return spell to hand after resolution).
    pub fn buyback(mut self, cost: TotalCost) -> Self {
        self.optional_costs.push(OptionalCost::buyback(cost));
        self
    }

    /// Add a buyback cost using just mana.
    pub fn buyback_mana(self, cost: ManaCost) -> Self {
        self.buyback(TotalCost::mana(cost))
    }

    /// Add an entwine cost (for modal spells, choose all modes).
    pub fn entwine(mut self, cost: TotalCost) -> Self {
        self.optional_costs.push(OptionalCost::entwine(cost));
        self
    }

    /// Add an entwine cost using just mana.
    pub fn entwine_mana(self, cost: ManaCost) -> Self {
        self.entwine(TotalCost::mana(cost))
    }

    /// Add a custom optional cost.
    pub fn optional_cost(mut self, cost: OptionalCost) -> Self {
        self.optional_costs.push(cost);
        self
    }

    /// Set cost effects (new unified model).
    ///
    /// Cost effects are executed as part of paying costs, with `EventCause::from_cost()`.
    /// This enables triggers like "Whenever a creature is sacrificed to pay a cost".
    pub fn cost_effects(mut self, effects: Vec<Effect>) -> Self {
        self.cost_effects = effects;
        self
    }

    // === Saga Support ===

    /// Configure this card as a saga with the given number of chapters.
    ///
    /// Sagas automatically gain a lore counter at the start of each precombat main phase.
    /// When a lore counter is added, any chapters at or below that number that haven't
    /// triggered yet will trigger.
    pub fn saga(mut self, max_chapters: u32) -> Self {
        self.max_saga_chapter = Some(max_chapters);
        self
    }

    /// Add a saga chapter ability that triggers on a single chapter.
    ///
    /// # Example
    /// ```ignore
    /// .with_chapter(1, vec![Effect::sacrifice(ObjectFilter::creature(), 1)])  // Chapter I
    /// ```
    pub fn with_chapter(self, chapter: u32, effects: Vec<Effect>) -> Self {
        use crate::triggers::Trigger;
        self.with_trigger(Trigger::saga_chapter(vec![chapter]), effects)
    }

    /// Add a saga chapter ability that triggers on multiple chapters.
    ///
    /// Use this for "I, II" style abilities that trigger on multiple chapters.
    ///
    /// # Example
    /// ```ignore
    /// .with_chapters(vec![1, 2], vec![Effect::draw(1)])  // Chapters I, II
    /// ```
    pub fn with_chapters(self, chapters: Vec<u32>, effects: Vec<Effect>) -> Self {
        use crate::triggers::Trigger;
        self.with_trigger(Trigger::saga_chapter(chapters), effects)
    }

    // === Level-Up Support ===

    /// Add a level-up activated ability.
    ///
    /// Level-up is an activated ability that can only be activated at sorcery speed.
    /// It adds a level counter to the creature.
    ///
    /// # Arguments
    /// * `cost` - The mana cost to level up
    ///
    /// # Example
    /// ```ignore
    /// .level_up(ManaCost::from_pips(vec![vec![ManaSymbol::White]]))
    /// ```
    pub fn level_up(self, cost: ManaCost) -> Self {
        use crate::ability::{AbilityKind, ActivatedAbility};
        use crate::zone::Zone;
        let level_up_text = format!("Level up {}", cost.to_oracle());

        let ability = Ability {
            kind: AbilityKind::Activated(ActivatedAbility {
                mana_cost: TotalCost::mana(cost),
                effects: vec![Effect::put_counters_on_source(CounterType::Level, 1)],
                choices: vec![],
                timing: ActivationTiming::SorcerySpeed,
                additional_restrictions: vec![],
            }),
            functional_zones: vec![Zone::Battlefield],
            text: Some(level_up_text),
        };
        self.with_ability(ability)
    }

    /// Add level-based abilities.
    ///
    /// Level abilities grant different P/T and abilities based on the number of
    /// level counters on the creature. Only one tier applies at a time.
    ///
    /// # Example
    /// ```ignore
    /// .with_level_abilities(vec![
    ///     LevelAbility::new(2, Some(6)).with_pt(3, 3).with_ability(StaticAbility::first_strike()),
    ///     LevelAbility::new(7, None).with_pt(4, 4).with_ability(StaticAbility::double_strike()),
    /// ])
    /// ```
    pub fn with_level_abilities(self, levels: Vec<LevelAbility>) -> Self {
        self.with_ability(Ability::static_ability(
            StaticAbility::with_level_abilities(levels),
        ))
    }

    // === Build ===

    /// Build the card definition.
    pub fn build(self) -> CardDefinition {
        CardDefinition {
            card: self.card_builder.build(),
            abilities: self.abilities,
            spell_effect: self.spell_effect,
            aura_attach_filter: self.aura_attach_filter,
            alternative_casts: self.alternative_casts,
            optional_costs: self.optional_costs,
            max_saga_chapter: self.max_saga_chapter,
            cost_effects: self.cost_effects,
        }
    }
}

#[cfg(all(test, feature = "parser-tests"))]
mod target_parse_tests {
    use super::*;

    #[test]
    fn parse_target_creature() {
        let tokens = tokenize_line("target creature", 0);
        let target = parse_target_phrase(&tokens).expect("parse target creature");
        match target {
            TargetAst::Object(filter, _, _) => {
                assert_eq!(filter, ObjectFilter::creature());
            }
            _ => panic!("expected object target"),
        }
    }

    #[test]
    fn parse_target_nonland_permanent() {
        let tokens = tokenize_line("target nonland permanent", 0);
        let target = parse_target_phrase(&tokens).expect("parse target nonland permanent");
        match target {
            TargetAst::Object(filter, _, _) => {
                assert_eq!(filter, ObjectFilter::nonland_permanent());
            }
            _ => panic!("expected object target"),
        }
    }

    #[test]
    fn parse_target_artifact_or_enchantment() {
        let tokens = tokenize_line("target artifact or enchantment", 0);
        let target = parse_target_phrase(&tokens).expect("parse target artifact or enchantment");
        match target {
            TargetAst::Object(filter, _, _) => {
                let expected =
                    ObjectFilter::any_of_types(&[CardType::Artifact, CardType::Enchantment]);
                assert_eq!(filter, expected);
            }
            _ => panic!("expected object target"),
        }
    }

    #[test]
    fn parse_target_battle() {
        let tokens = tokenize_line("target battle", 0);
        let target = parse_target_phrase(&tokens).expect("parse target battle");
        match target {
            TargetAst::Object(filter, _, _) => {
                let expected = ObjectFilter::default()
                    .in_zone(Zone::Battlefield)
                    .with_type(CardType::Battle);
                assert_eq!(filter, expected);
            }
            _ => panic!("expected object target"),
        }
    }

    #[test]
    fn parse_target_creature_you_control() {
        let tokens = tokenize_line("target creature you control", 0);
        let target = parse_target_phrase(&tokens).expect("parse target creature you control");
        match target {
            TargetAst::Object(filter, _, _) => {
                assert_eq!(filter, ObjectFilter::creature().you_control());
            }
            _ => panic!("expected object target"),
        }
    }

    #[test]
    fn parse_another_target_creature_you_control() {
        let tokens = tokenize_line("another target creature you control", 0);
        let target = parse_target_phrase(&tokens).expect("parse another target creature");
        match target {
            TargetAst::Object(filter, _, _) => {
                assert_eq!(filter, ObjectFilter::creature().you_control().other());
            }
            _ => panic!("expected object target"),
        }
    }

    #[test]
    fn parse_target_nonblack_creature() {
        let tokens = tokenize_line("target nonblack creature", 0);
        let target = parse_target_phrase(&tokens).expect("parse target nonblack creature");
        match target {
            TargetAst::Object(filter, _, _) => {
                let expected = ObjectFilter::creature().without_colors(ColorSet::BLACK);
                assert_eq!(filter, expected);
            }
            _ => panic!("expected object target"),
        }
    }

    #[test]
    fn parse_target_on_it() {
        let tokens = tokenize_line("on it", 0);
        let target = parse_target_phrase(&tokens).expect("parse on it");
        match target {
            TargetAst::Tagged(tag, _) => {
                assert_eq!(tag.as_str(), IT_TAG);
            }
            TargetAst::Object(filter, _, _) => {
                assert_eq!(filter.tagged_constraints.len(), 1);
                let constraint = &filter.tagged_constraints[0];
                assert_eq!(constraint.tag.as_str(), IT_TAG);
                assert_eq!(constraint.relation, TaggedOpbjectRelation::IsTaggedObject);
            }
            _ => panic!("expected object target"),
        }
    }

    #[test]
    fn parse_target_this_as_source() {
        let tokens = tokenize_line("this", 0);
        let target = parse_target_phrase(&tokens).expect("parse this");
        assert!(matches!(target, TargetAst::Source(_)));
    }

    #[test]
    fn parse_target_this_creature_as_source() {
        let tokens = tokenize_line("this creature", 0);
        let target = parse_target_phrase(&tokens).expect("parse this creature");
        assert!(matches!(target, TargetAst::Source(_)));
    }

    #[test]
    fn parse_target_this_card_from_your_graveyard_as_source() {
        let tokens = tokenize_line("this card from your graveyard", 0);
        let target = parse_target_phrase(&tokens).expect("parse this card from your graveyard");
        match target {
            TargetAst::Object(filter, _, _) => {
                assert!(filter.source, "expected source filter");
                assert_eq!(filter.zone, Some(Zone::Graveyard));
                assert_eq!(filter.owner, Some(PlayerFilter::You));
            }
            _ => panic!("expected source-object graveyard target"),
        }
    }

    #[test]
    fn parse_permanent_shares_card_type_with_it() {
        let tokens = tokenize_line("a permanent that shares a card type with it", 0);
        let filter = parse_object_filter(&tokens, false).expect("parse shared card type filter");
        assert_eq!(filter.tagged_constraints.len(), 1);
        let constraint = &filter.tagged_constraints[0];
        assert_eq!(constraint.tag.as_str(), IT_TAG);
        assert_eq!(constraint.relation, TaggedOpbjectRelation::SharesCardType);
    }

    #[test]
    fn parse_object_filter_enchanted_creature_adds_attachment_tag() {
        let tokens = tokenize_line("enchanted creature", 0);
        let filter = parse_object_filter(&tokens, false).expect("parse enchanted creature filter");
        assert!(
            filter.card_types.contains(&CardType::Creature),
            "expected creature type in filter"
        );
        assert!(
            filter.tagged_constraints.iter().any(|constraint| {
                constraint.tag.as_str() == "enchanted"
                    && constraint.relation == TaggedOpbjectRelation::IsTaggedObject
            }),
            "expected enchanted attachment constraint, got {:?}",
            filter.tagged_constraints
        );
    }

    #[test]
    fn parse_object_filter_equipped_creature_adds_attachment_tag() {
        let tokens = tokenize_line("equipped creature", 0);
        let filter = parse_object_filter(&tokens, false).expect("parse equipped creature filter");
        assert!(
            filter.card_types.contains(&CardType::Creature),
            "expected creature type in filter"
        );
        assert!(
            filter.tagged_constraints.iter().any(|constraint| {
                constraint.tag.as_str() == "equipped"
                    && constraint.relation == TaggedOpbjectRelation::IsTaggedObject
            }),
            "expected equipped attachment constraint, got {:?}",
            filter.tagged_constraints
        );
    }

    #[test]
    fn parse_object_filter_flashback_cards_from_their_graveyard() {
        let tokens = tokenize_line("cards with flashback from their graveyard", 0);
        let filter = parse_object_filter(&tokens, false).expect("parse flashback graveyard filter");
        assert_eq!(filter.zone, Some(Zone::Graveyard));
        assert_eq!(filter.owner, Some(PlayerFilter::target_player()));
        assert_eq!(
            filter.alternative_cast,
            Some(crate::filter::AlternativeCastKind::Flashback)
        );
    }

    #[test]
    fn parse_object_filter_commanders_you_own_sets_commander_and_owner() {
        let tokens = tokenize_line("commander creatures you own", 0);
        let filter =
            parse_object_filter(&tokens, false).expect("parse commander creatures you own filter");
        assert!(filter.is_commander, "expected commander marker");
        assert_eq!(filter.owner, Some(PlayerFilter::You));
        assert!(filter.card_types.contains(&CardType::Creature));
    }

    #[test]
    fn parse_object_filter_you_do_not_own_sets_owner_to_opponent() {
        let tokens = tokenize_line("target creature you do not own", 0);
        let target = parse_target_phrase(&tokens).expect("parse target creature you do not own");
        let TargetAst::Object(filter, _, _) = target else {
            panic!("expected object target");
        };
        assert_eq!(filter.owner, Some(PlayerFilter::Opponent));
    }

    #[test]
    fn parse_target_djinn_or_efreet_includes_both_subtypes() {
        let tokens = tokenize_line("target Djinn or Efreet", 0);
        let target = parse_target_phrase(&tokens).expect("parse subtype-or target phrase");
        let TargetAst::Object(filter, _, _) = target else {
            panic!("expected object target");
        };
        assert!(
            filter.subtypes.contains(&Subtype::Djinn),
            "expected Djinn subtype in filter"
        );
        assert!(
            filter.subtypes.contains(&Subtype::Efreet),
            "expected Efreet subtype in filter"
        );
    }

    #[test]
    fn parse_target_non_subtypes_populates_excluded_subtypes() {
        let tokens = tokenize_line("target non-Vampire, non-Werewolf, non-Zombie creature", 0);
        let target = parse_target_phrase(&tokens).expect("parse excluded subtype target");
        let TargetAst::Object(filter, _, _) = target else {
            panic!("expected object target");
        };
        assert!(
            filter.card_types.contains(&CardType::Creature),
            "expected creature type in filter"
        );
        assert!(
            filter.excluded_subtypes.contains(&Subtype::Vampire),
            "expected excluded Vampire subtype"
        );
        assert!(
            filter.excluded_subtypes.contains(&Subtype::Werewolf),
            "expected excluded Werewolf subtype"
        );
        assert!(
            filter.excluded_subtypes.contains(&Subtype::Zombie),
            "expected excluded Zombie subtype"
        );
    }

    #[test]
    fn parse_target_non_army_creature_populates_excluded_army_subtype() {
        let tokens = tokenize_line("target non-Army creature", 0);
        let target = parse_target_phrase(&tokens).expect("parse non-Army creature target");
        let TargetAst::Object(filter, _, _) = target else {
            panic!("expected object target");
        };
        assert!(
            filter.card_types.contains(&CardType::Creature),
            "expected creature type in filter"
        );
        assert!(
            filter.excluded_subtypes.contains(&Subtype::Army),
            "expected excluded Army subtype"
        );
    }

    #[test]
    fn parse_anthem_subject_prefers_non_subtype_filter_suffix() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Ferocity Variant")
            .parse_text("Attacking non-Human creatures you control get +1/+0 and have trample.")
            .expect("parse non-human attacking anthem");

        let displays: Vec<String> = def
            .abilities
            .iter()
            .filter_map(|ability| match &ability.kind {
                AbilityKind::Static(static_ability) => Some(static_ability.display()),
                _ => None,
            })
            .collect();

        assert!(
            displays.iter().any(|display| {
                display.contains("nonHuman")
                    && display.contains("attacking")
                    && display.contains("+1/+0")
            }),
            "expected non-Human attacking subject in anthem display, got {displays:?}"
        );
        assert!(
            !displays
                .iter()
                .any(|display| display.contains("creature Human gets +1/+0")),
            "did not expect positive Human filter in anthem display, got {displays:?}"
        );
    }
}

#[cfg(all(test, feature = "parser-tests"))]
mod effect_parse_tests {
    use super::*;
    use crate::alternative_cast::AlternativeCastingMethod;
    use crate::compiled_text::compiled_lines;
    use crate::effect::{Condition, Until, Value};
    use crate::effects::CantEffect;
    use crate::effects::{
        AddManaOfAnyColorEffect, AddManaOfAnyOneColorEffect, AddManaOfLandProducedTypesEffect,
        AddScaledManaEffect, BattlefieldController, ConniveEffect, CounterEffect,
        CreateTokenCopyEffect, DestroyEffect,
        DiscardEffect, DrawCardsEffect, EnergyCountersEffect, ExchangeControlEffect, ExileEffect,
        ExileInsteadOfGraveyardEffect, ForEachObject, ForPlayersEffect, GainControlEffect,
        GrantPlayFromGraveyardEffect, LookAtHandEffect, ModifyPowerToughnessEffect,
        ModifyPowerToughnessForEachEffect, PutCountersEffect, RemoveUpToAnyCountersEffect,
        ReturnAllToBattlefieldEffect, MoveToZoneEffect,
        ReturnFromGraveyardToBattlefieldEffect, ReturnToHandEffect, SacrificeEffect,
        ScryEffect, SetBasePowerToughnessEffect, SetLifeTotalEffect, SkipCombatPhasesEffect,
        SkipDrawStepEffect, SkipNextCombatPhaseThisTurnEffect, SkipTurnEffect, SurveilEffect,
        TapEffect, TargetOnlyEffect, TransformEffect,
    };
    use crate::ids::CardId;
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::object::CounterType;
    use crate::target::{ChooseSpec, PlayerFilter};
    use crate::types::CardType;
    use crate::types::Subtype;
    use crate::zone::Zone;

    #[test]
    fn parse_yawgmoths_will_from_text() {
        let text = "Until end of turn, you may play lands and cast spells from your graveyard.\n\
If a card would be put into your graveyard from anywhere this turn, exile that card instead.";
        let def = CardDefinitionBuilder::new(CardId::new(), "Yawgmoth's Will")
            .parse_text(text)
            .expect("parse yawgmoth's will");

        let effects = def.spell_effect.as_ref().expect("spell effect");
        assert_eq!(effects.len(), 2);
        assert!(
            effects
                .iter()
                .any(|e| e.downcast_ref::<GrantPlayFromGraveyardEffect>().is_some()),
            "should include play-from-graveyard effect"
        );
        assert!(
            effects
                .iter()
                .any(|e| e.downcast_ref::<ExileInsteadOfGraveyardEffect>().is_some()),
            "should include exile-instead replacement effect"
        );
    }

    #[test]
    fn parse_cant_gain_life_until_eot_from_text() {
        let text = "Until end of turn, players can't gain life.";
        let def = CardDefinitionBuilder::new(CardId::new(), "No Life")
            .parse_text(text)
            .expect("parse cant gain life");

        let effects = def.spell_effect.as_ref().expect("spell effect");
        assert!(
            effects
                .iter()
                .any(|e| e.downcast_ref::<CantEffect>().is_some()),
            "should include cant effect"
        );
    }

    #[test]
    fn parse_source_cant_be_blocked_until_eot_from_text() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Horizons Variant")
            .card_types(vec![CardType::Creature])
            .parse_text("{2}{U}: This creature can't be blocked this turn.")
            .expect("source cant-be-blocked clause should parse");

        let activated = def
            .abilities
            .iter()
            .find_map(|ability| match &ability.kind {
                AbilityKind::Activated(activated) => Some(activated),
                _ => None,
            })
            .expect("expected activated ability");

        let cant = activated
            .effects
            .iter()
            .find_map(|effect| effect.downcast_ref::<CantEffect>())
            .expect("expected cant effect");
        assert_eq!(cant.duration, crate::effect::Until::EndOfTurn);
        match &cant.restriction {
            crate::effect::Restriction::BeBlocked(filter) => {
                assert!(
                    filter.source,
                    "expected source-bound restriction filter, got {filter:?}"
                );
            }
            other => panic!("expected be-blocked restriction, got {other:?}"),
        }
    }

    #[test]
    fn parse_source_doesnt_untap_during_next_untap_step_from_text() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Cloudcrest Lake Variant")
            .card_types(vec![CardType::Land])
            .parse_text(
                "{T}: Add {W}.\n{T}: Add {U}. This land doesn't untap during your next untap step.",
            )
            .expect("next-untap-step negated untap clause should parse");

        let abilities: Vec<_> = def
            .abilities
            .iter()
            .filter_map(|ability| match &ability.kind {
                AbilityKind::Mana(mana) => Some(mana),
                _ => None,
            })
            .collect();
        assert!(abilities.len() >= 2, "expected two mana abilities");

        let slow_mana = abilities
            .iter()
            .find(|mana| {
                mana.effects.as_ref().is_some_and(|effects| {
                    effects
                        .iter()
                        .any(|effect| effect.downcast_ref::<CantEffect>().is_some())
                })
            })
            .expect("expected mana ability with untap restriction");

        let effects = slow_mana.effects.as_ref().expect("mana effects");
        let cant = effects
            .iter()
            .find_map(|effect| effect.downcast_ref::<CantEffect>())
            .expect("expected untap restriction effect");
        assert_eq!(cant.duration, crate::effect::Until::YourNextTurn);
        match &cant.restriction {
            crate::effect::Restriction::Untap(filter) => {
                assert!(
                    filter.source,
                    "expected source-bound untap restriction filter, got {filter:?}"
                );
            }
            other => panic!("expected untap restriction, got {other:?}"),
        }
    }

    #[test]
    fn parse_targets_dont_untap_during_controller_next_untap_step_uses_controller_duration() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Frost Breath Variant")
            .card_types(vec![CardType::Instant])
            .parse_text(
                "Tap up to two target creatures. Those creatures don't untap during their controller's next untap step.",
            )
            .expect("controller-next-untap-step tap clause should parse");

        let effects = def.spell_effect.as_ref().expect("spell effect");
        let cant = effects
            .iter()
            .find_map(|effect| effect.downcast_ref::<CantEffect>())
            .expect("expected untap restriction");
        assert_eq!(
            cant.duration,
            crate::effect::Until::ControllersNextUntapStep
        );
    }

    #[test]
    fn parse_enchanted_creature_dies_return_under_your_control_uses_move_to_zone() {
        let def = CardDefinitionBuilder::new(CardId::new(), "False Demise Variant")
            .card_types(vec![CardType::Enchantment])
            .subtypes(vec![Subtype::Aura])
            .parse_text(
                "Enchant creature\nWhen enchanted creature dies, return that card to the battlefield under your control.",
            )
            .expect("false-demise style trigger should parse");

        let abilities_debug = format!("{:#?}", def.abilities);
        assert!(
            abilities_debug.contains("MoveToZoneEffect"),
            "expected move-to-zone return effect, got {abilities_debug}"
        );
        assert!(
            abilities_debug.contains("battlefield_controller: You"),
            "expected under-your-control return semantics, got {abilities_debug}"
        );
        assert!(
            !abilities_debug.contains("ReturnFromGraveyardToBattlefieldEffect"),
            "expected compile to avoid target-only graveyard return helper, got {abilities_debug}"
        );
    }

    #[test]
    fn parse_return_to_hand_from_text() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Unsummon")
            .parse_text("Return target creature to its owner's hand.")
            .expect("parse return to hand");

        let effects = def.spell_effect.as_ref().expect("spell effect");
        assert!(
            effects
                .iter()
                .any(|e| e.downcast_ref::<ReturnToHandEffect>().is_some()),
            "should include return-to-hand effect"
        );
    }

    #[test]
    fn parse_return_up_to_cards_from_graveyard_to_hand_from_text() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Macabre Reconstruction")
            .parse_text("Return up to two target creature cards from your graveyard to your hand.")
            .expect("parse up-to return to hand");

        let effects = def.spell_effect.as_ref().expect("spell effect");
        let bounce = effects
            .iter()
            .find_map(|e| e.downcast_ref::<ReturnToHandEffect>())
            .expect("should include return-to-hand effect");
        assert_eq!(bounce.spec.count().min, 0);
        assert_eq!(bounce.spec.count().max, Some(2));
    }

    #[test]
    fn parse_tap_one_or_two_targets_preserves_choice_count() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Probe Tap Two")
            .parse_text("Tap one or two target creatures.")
            .expect("parse tap one-or-two targets");

        let effects = def.spell_effect.as_ref().expect("spell effect");
        let tap = effects
            .iter()
            .find_map(|e| e.downcast_ref::<TapEffect>())
            .expect("should include tap effect");
        assert_eq!(tap.spec.count().min, 1);
        assert_eq!(tap.spec.count().max, Some(2));
    }

    #[test]
    fn parse_exile_target_players_graveyard_as_all_filter() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Boggart Bog Variant")
            .card_types(vec![CardType::Creature])
            .parse_text("When this creature enters, exile target player's graveyard.")
            .expect("parse target player's graveyard exile");

        let triggered = def
            .abilities
            .iter()
            .find_map(|ability| match &ability.kind {
                AbilityKind::Triggered(triggered) => Some(triggered),
                _ => None,
            })
            .expect("expected triggered ability");

        let exile = triggered
            .effects
            .iter()
            .find_map(|effect| effect.downcast_ref::<ExileEffect>())
            .expect("expected exile effect");
        let ChooseSpec::All(filter) = &exile.spec else {
            panic!(
                "expected non-targeted all-filter exile, got {:?}",
                exile.spec
            );
        };
        assert_eq!(filter.zone, Some(Zone::Graveyard));
        assert_eq!(
            filter.controller,
            Some(PlayerFilter::Target(Box::new(PlayerFilter::Any)))
        );
    }

    #[test]
    fn parse_tap_all_spirits_compiles_as_non_targeted_all() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Probe Tap All Spirits")
            .parse_text("Tap all Spirits.")
            .expect("parse tap-all clause");

        let effects = def.spell_effect.as_ref().expect("spell effect");
        let tap = effects
            .iter()
            .find_map(|e| e.downcast_ref::<TapEffect>())
            .expect("should include tap effect");
        let ChooseSpec::All(filter) = &tap.spec else {
            panic!("expected non-targeted tap-all spec, got {:?}", tap.spec);
        };
        assert!(
            filter.subtypes.contains(&Subtype::Spirit),
            "expected Spirit subtype filter, got {filter:?}"
        );
    }

    #[test]
    fn parse_exile_any_number_of_target_spells_preserves_choice_count() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Probe Exile Any")
            .parse_text("Exile any number of target spells.")
            .expect("parse exile any-number targets");

        let debug = format!("{:?}", def.spell_effect);
        assert!(
            debug.contains("min: 0") && debug.contains("max: None"),
            "expected any-number target count in runtime effect, got {debug}"
        );

        let lines = compiled_lines(&def);
        let spell_line = lines
            .iter()
            .find(|line| line.starts_with("Spell effects:"))
            .expect("expected spell effects line");
        assert!(
            spell_line.contains("any number of target spell"),
            "expected rendered any-number target text, got {spell_line}"
        );
    }

    #[test]
    fn parse_return_to_battlefield_from_graveyard_from_text() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Reanimate Variant")
            .parse_text(
                "Return target creature card from your graveyard to the battlefield tapped.",
            )
            .expect("parse return to battlefield");

        let effects = def.spell_effect.as_ref().expect("spell effect");
        assert!(
            effects.iter().any(|e| e
                .downcast_ref::<ReturnFromGraveyardToBattlefieldEffect>()
                .is_some()),
            "should include return-to-battlefield effect"
        );
    }

    #[test]
    fn parse_return_all_from_graveyards_to_battlefield_tapped_from_text() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Planar Birth Variant")
            .parse_text(
                "Return all basic land cards from all graveyards to the battlefield tapped under their owners' control.",
            )
            .expect("parse return all cards to battlefield");

        let effects = def.spell_effect.as_ref().expect("spell effect");
        let return_all = effects
            .iter()
            .find_map(|e| e.downcast_ref::<ReturnAllToBattlefieldEffect>())
            .expect("should include return-all-to-battlefield effect");
        assert!(return_all.tapped, "expected tapped return-all effect");
    }

    #[test]
    fn parse_exchange_control_from_text() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Switcheroo")
            .parse_text("Exchange control of two target creatures.")
            .expect("parse exchange control");

        let effects = def.spell_effect.as_ref().expect("spell effect");
        assert!(
            effects
                .iter()
                .any(|e| e.downcast_ref::<ExchangeControlEffect>().is_some()),
            "should include exchange control effect"
        );
    }

    #[test]
    fn parse_target_player_draw_then_discard_shares_player() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Cephalid Looter Variant")
            .card_types(vec![CardType::Creature])
            .parse_text("{T}: Target player draws a card, then discards a card.")
            .expect("parse target-player loot ability");

        let lines = crate::compiled_text::compiled_lines(&def);
        let activated_line = lines
            .iter()
            .find(|line| line.contains("Activated ability"))
            .expect("expected activated ability line");

        assert!(
            activated_line
                .contains("target a player draws a card. target a player discards a card"),
            "expected carried target player in discard clause, got {activated_line}"
        );
        assert!(
            !activated_line.contains("you discards"),
            "discard should not default to you: {activated_line}"
        );
    }

    #[test]
    fn parse_target_player_loses_then_reveals_shares_player() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Thoughtcutter Variant")
            .card_types(vec![CardType::Creature])
            .parse_text("{U}{B}, {T}: Target player loses 1 life and reveals their hand.")
            .expect("parse target-player lose-and-reveal ability");

        let lines = crate::compiled_text::compiled_lines(&def);
        let activated_line = lines
            .iter()
            .find(|line| line.contains("Activated ability"))
            .expect("expected activated ability line");

        assert!(
            activated_line.contains("target a player loses 1 life"),
            "expected carried target player in lose clause, got {activated_line}"
        );
        assert!(
            activated_line.contains("Look at a player's hand"),
            "expected carried target player in reveal clause, got {activated_line}"
        );
        assert!(
            !activated_line.contains("your hand"),
            "reveal should not default to your hand: {activated_line}"
        );
    }

    #[test]
    fn parse_draw_for_each_tapped_creature_target_opponent_controls() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Borrowing Arrows Variant")
            .parse_text("Draw a card for each tapped creature target opponent controls.")
            .expect("draw-for-each clause should parse");

        let effects = def.spell_effect.expect("spell effect");
        let draw = effects
            .iter()
            .find_map(|effect| effect.downcast_ref::<DrawCardsEffect>())
            .expect("expected draw cards effect");
        match &draw.count {
            Value::Count(filter) => {
                assert!(
                    filter.card_types.contains(&CardType::Creature),
                    "expected creature filter, got {:?}",
                    filter.card_types
                );
                assert!(filter.tapped, "expected tapped filter");
                assert!(
                    filter.controller.is_some(),
                    "expected controlled-by-opponent filter"
                );
            }
            other => panic!("expected count-based draw, got {other:?}"),
        }
    }

    #[test]
    fn parse_draw_with_unsupported_tail_errors() {
        let result = CardDefinitionBuilder::new(CardId::new(), "Bad Draw Tail")
            .parse_text("Draw a card whenever this is weird.");
        assert!(
            result.is_err(),
            "unknown draw tail should fail instead of silently compiling fixed draw"
        );
    }

    #[test]
    fn parse_self_return_from_graveyard_activated_ability() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Durable Coilbug Variant")
            .card_types(vec![CardType::Creature])
            .parse_text("{4}{B}: Return this card from your graveyard to your hand.")
            .expect("parse self return from graveyard");

        let ability = def
            .abilities
            .iter()
            .find_map(|ability| match &ability.kind {
                AbilityKind::Activated(activated) => Some((ability, activated)),
                _ => None,
            })
            .expect("expected activated ability");

        assert_eq!(ability.0.functional_zones, vec![Zone::Graveyard]);
        assert!(ability.1.choices.is_empty(), "should not target");

        let lines = crate::compiled_text::compiled_lines(&def);
        assert!(
            lines.iter().any(|line| line.contains("Return this source")),
            "expected source return text, got {lines:?}"
        );
    }

    #[test]
    fn parse_counter_spell_with_graveyard_reference_from_text() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Drown in the Loch Variant")
            .parse_text(
                "Counter target spell with mana value less than or equal to the number of cards in its controller's graveyard.",
            )
            .expect("parse counter spell with graveyard reference");

        let effects = def.spell_effect.expect("spell effect");
        assert!(
            effects
                .iter()
                .any(|e| e.downcast_ref::<CounterEffect>().is_some()),
            "should include counter effect"
        );
    }

    #[test]
    fn parse_target_creature_has_base_power_toughness_until_end_of_turn() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Diminish Variant")
            .parse_text("Target creature has base power and toughness 1/1 until end of turn.")
            .expect("set-base-power/toughness clause should parse");

        let effects = def.spell_effect.as_ref().expect("spell effect");
        let set_base = effects
            .iter()
            .find_map(|effect| effect.downcast_ref::<SetBasePowerToughnessEffect>())
            .expect("expected SetBasePowerToughnessEffect");
        assert_eq!(set_base.power, Value::Fixed(1));
        assert_eq!(set_base.toughness, Value::Fixed(1));
        assert_eq!(set_base.duration, Until::EndOfTurn);

        let lines = crate::compiled_text::compiled_lines(&def);
        let spell_line = lines
            .iter()
            .find(|line| line.starts_with("Spell effects:"))
            .expect("expected spell effects line");
        assert!(
            spell_line.contains("base power and toughness 1/1"),
            "compiled text should include base P/T wording, got {spell_line}"
        );
    }

    #[test]
    fn parse_enchanted_creature_has_base_power_toughness_as_static() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Illusory Wrappings Variant")
            .parse_text("Enchant creature\nEnchanted creature has base power and toughness 0/2.")
            .expect("base power/toughness Aura line should parse as static ability");

        let static_ids: Vec<_> = def
            .abilities
            .iter()
            .filter_map(|ability| match &ability.kind {
                AbilityKind::Static(static_ability) => Some(static_ability.id()),
                _ => None,
            })
            .collect();
        assert!(
            static_ids.contains(&StaticAbilityId::SetBasePowerToughnessForFilter),
            "expected static SetBasePowerToughnessForFilter, got {static_ids:?}"
        );

        let spell_has_set_base = def.spell_effect.as_ref().is_some_and(|effects| {
            effects
                .iter()
                .any(|effect| effect.downcast_ref::<SetBasePowerToughnessEffect>().is_some())
        });
        assert!(
            !spell_has_set_base,
            "base P/T for Aura text should not be a spell-effect duration modification"
        );

        let lines = crate::compiled_text::compiled_lines(&def);
        let has_base_line = lines
            .iter()
            .find(|line| line.contains("base power and toughness 0/2"))
            .expect("compiled text should include base P/T static wording");
        assert!(
            !has_base_line.contains("until end of turn"),
            "static base P/T line must not be temporary: {has_base_line}"
        );
    }

    #[test]
    fn parse_enchanted_creature_loses_abilities_and_transforms_with_base_pt() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Ichthyomorphosis Variant")
            .parse_text(
                "Enchant creature\nEnchanted creature loses all abilities and is a blue Fish with base power and toughness 0/1.",
            )
            .expect("transforming lose-all-abilities Aura line should parse");

        let static_ids: Vec<_> = def
            .abilities
            .iter()
            .filter_map(|ability| match &ability.kind {
                AbilityKind::Static(static_ability) => Some(static_ability.id()),
                _ => None,
            })
            .collect();
        assert!(
            static_ids.contains(&StaticAbilityId::RemoveAllAbilitiesForFilter),
            "expected lose-all-abilities static, got {static_ids:?}"
        );
        assert!(
            static_ids.contains(&StaticAbilityId::SetCardTypes),
            "expected set-card-types static, got {static_ids:?}"
        );
        assert!(
            static_ids.contains(&StaticAbilityId::SetCreatureSubtypes),
            "expected creature-subtype reset static, got {static_ids:?}"
        );
        assert!(
            static_ids.contains(&StaticAbilityId::SetColors),
            "expected set-colors static, got {static_ids:?}"
        );
        assert!(
            static_ids.contains(&StaticAbilityId::SetBasePowerToughnessForFilter),
            "expected set-base-power/toughness static, got {static_ids:?}"
        );
    }

    #[test]
    fn parse_target_creature_has_base_power_until_end_of_turn() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Wak-Wak Variant")
            .parse_text("Target attacking creature has base power 0 until end of turn.")
            .expect("base-power-only clause should parse");

        let lines = crate::compiled_text::compiled_lines(&def);
        let spell_line = lines
            .iter()
            .find(|line| line.starts_with("Spell effects:"))
            .expect("expected spell effects line");
        assert!(
            spell_line.contains("base power 0") && spell_line.contains("until end of turn"),
            "compiled text should include temporary base power wording, got {spell_line}"
        );
        assert!(
            !spell_line.contains("Choose target"),
            "base-power-only clause should compile to an effect, not target-only text: {spell_line}"
        );
    }

    #[test]
    fn parse_destroy_each_creature_isnt_all_colors_clause() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Iridian Variant")
            .parse_text("Destroy each creature that isn't all colors.")
            .expect("destroy non-all-colors clause should parse");

        let lines = crate::compiled_text::compiled_lines(&def);
        let spell_line = lines
            .iter()
            .find(|line| line.starts_with("Spell effects:"))
            .expect("expected spell effects line");
        assert!(
            spell_line.contains("not all colors"),
            "compiled text should preserve all-colors exclusion, got {spell_line}"
        );
    }

    #[test]
    fn parse_exile_target_nonland_not_exactly_two_colors_clause() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Ravnica Variant")
            .parse_text(
                "Exile target nonland permanent an opponent controls that isn't exactly two colors.",
            )
            .expect("exile target not-exactly-two-colors clause should parse");

        let lines = crate::compiled_text::compiled_lines(&def);
        let spell_line = lines
            .iter()
            .find(|line| line.starts_with("Spell effects:"))
            .expect("expected spell effects line");
        assert!(
            spell_line.contains("not exactly two colors"),
            "compiled text should preserve exact-two-colors exclusion, got {spell_line}"
        );
    }

    #[test]
    fn parse_base_power_toughness_with_unknown_tail_errors() {
        let result = CardDefinitionBuilder::new(CardId::new(), "Bad Base PT Tail")
            .parse_text("Target creature has base power and toughness 1/1 while enchanted.");
        assert!(
            result.is_err(),
            "unsupported base P/T tail should fail instead of partial target-only parse"
        );
    }

    #[test]
    fn parse_search_split_to_battlefield_and_hand_from_text() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Cultivate Variant")
            .parse_text("Search your library for up to two basic land cards, reveal those cards, put one onto the battlefield tapped and the other into your hand, then shuffle.")
            .expect("parse split search destinations");

        let lines = crate::compiled_text::compiled_lines(&def);
        let spell_line = lines
            .iter()
            .find(|line| line.starts_with("Spell effects:"))
            .expect("expected spell effects line");

        assert!(
            spell_line.contains("onto the battlefield tapped"),
            "expected tapped battlefield placement, got {spell_line}"
        );
        assert!(
            spell_line.contains("put it into hand"),
            "expected hand placement leg, got {spell_line}"
        );
    }

    #[test]
    fn parse_search_to_battlefield_tapped_preserves_tapped_flag() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Roiling Regrowth Variant")
            .parse_text(
                "Search your library for up to two basic land cards, put them onto the battlefield tapped, then shuffle.",
            )
            .expect("parse tapped battlefield search");

        let lines = crate::compiled_text::compiled_lines(&def);
        let spell_line = lines
            .iter()
            .find(|line| line.starts_with("Spell effects:"))
            .expect("expected spell effects line");
        assert!(
            spell_line.contains("onto the battlefield tapped"),
            "expected tapped battlefield placement in compiled text, got {spell_line}"
        );
        assert!(
            spell_line.contains("Search your library"),
            "expected compact search wording, got {spell_line}"
        );
        assert!(
            !spell_line.contains("chooses up to"),
            "should not leak choose-object internals in search display: {spell_line}"
        );
    }

    #[test]
    fn parse_activated_search_to_hand_renders_compact_search_text() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Armillary Sphere Variant")
            .parse_text(
                "{2}, {T}, Sacrifice this artifact: Search your library for up to two basic land cards, reveal them, put them into your hand, then shuffle.",
            )
            .expect("parse search-to-hand activated ability");

        let lines = crate::compiled_text::compiled_lines(&def);
        let ability_line = lines
            .iter()
            .find(|line| line.starts_with("Activated ability"))
            .expect("expected activated ability line");
        assert!(
            ability_line.contains("Search your library"),
            "expected compact search wording, got {ability_line}"
        );
        assert!(
            ability_line.contains("put them into hand"),
            "expected hand destination wording, got {ability_line}"
        );
        assert!(
            !ability_line.contains("chooses up to"),
            "should not leak choose-object internals in search display: {ability_line}"
        );
    }

    #[test]
    fn parse_double_counters_on_each_creature_from_text() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Kalonian Hydra Variant")
            .parse_text(
                "Whenever this creature attacks, double the number of +1/+1 counters on each creature you control.",
            )
            .expect("parse kalonian hydra attack trigger");

        let ability = def
            .abilities
            .iter()
            .find(|ability| matches!(ability.kind, AbilityKind::Triggered(_)))
            .expect("should have triggered ability");
        let AbilityKind::Triggered(triggered) = &ability.kind else {
            panic!("expected triggered ability");
        };
        let for_each = triggered
            .effects
            .iter()
            .find_map(|effect| effect.downcast_ref::<ForEachObject>())
            .expect("triggered ability should compile through ForEachObject");
        let put = for_each
            .effects
            .iter()
            .find_map(|effect| effect.downcast_ref::<PutCountersEffect>())
            .expect("ForEachObject should include PutCountersEffect");

        assert_eq!(put.counter_type, CounterType::PlusOnePlusOne);
        assert_eq!(
            put.count,
            Value::CountersOn(
                Box::new(ChooseSpec::Iterated),
                Some(CounterType::PlusOnePlusOne)
            )
        );
        assert_eq!(put.target, ChooseSpec::Iterated);
    }

    #[test]
    fn parse_remove_typed_counter_from_text() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Power Conduit Variant")
            .parse_text("Remove a +1/+1 counter from target creature.")
            .expect("parse typed counter removal");

        let effects = def.spell_effect.expect("spell effect");
        assert!(
            effects.iter().any(|e| {
                e.downcast_ref::<RemoveUpToAnyCountersEffect>().is_some()
                    || format!("{e:?}").contains("RemoveUpToAnyCountersEffect")
            }),
            "should include remove counters effect"
        );
    }

    #[test]
    fn parse_create_token_copy_of_target_from_text() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Copy Variant")
            .parse_text("Create a token that's a copy of target artifact.")
            .expect("parse copy token from target");

        let effects = def.spell_effect.expect("spell effect");
        assert!(
            effects.iter().any(|e| {
                e.downcast_ref::<CreateTokenCopyEffect>().is_some()
                    || format!("{e:?}").contains("CreateTokenCopyEffect")
            }),
            "should include create-token-copy effect"
        );
    }

    #[test]
    fn parse_molten_duplication_style_copy_haste_and_delayed_sacrifice() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Molten Duplication Variant")
            .parse_text("Create a token that's a copy of target artifact or creature you control, except it's an artifact in addition to its other types. It gains haste until end of turn. Sacrifice it at the beginning of the next end step.")
            .expect("parse molten duplication style text");

        let effects = def.spell_effect.expect("spell effect");
        let copy = effects
            .iter()
            .find_map(|e| e.downcast_ref::<CreateTokenCopyEffect>())
            .expect("should include create-token-copy effect");
        assert!(copy.has_haste, "copy should gain haste");
        assert!(
            copy.sacrifice_at_next_end_step,
            "copy should sacrifice at next end step"
        );
    }

    #[test]
    fn parse_shaleskin_bruiser_style_scaling_attack_trigger() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Shaleskin Bruiser Variant")
            .parse_text(
                "Trample\nWhenever this creature attacks, it gets +3/+0 until end of turn for each other attacking Beast.",
            )
            .expect("parse shaleskin bruiser style text");

        let triggered = def
            .abilities
            .iter()
            .find_map(|ability| match &ability.kind {
                AbilityKind::Triggered(triggered) => Some(triggered),
                _ => None,
            })
            .expect("expected triggered ability");

        let modify = triggered
            .effects
            .iter()
            .find_map(|effect| effect.downcast_ref::<ModifyPowerToughnessForEachEffect>())
            .expect("trigger should include ModifyPowerToughnessForEachEffect");
        assert_eq!(modify.power_per, 3);
        assert_eq!(modify.toughness_per, 0);
        let Value::Count(filter) = &modify.count else {
            panic!("expected count-based scaling");
        };
        assert!(filter.other, "filter should require other permanents");
        assert!(
            filter.attacking,
            "filter should require attacking permanents"
        );
        assert!(
            filter.subtypes.contains(&Subtype::Beast),
            "filter should require Beast subtype"
        );
    }

    #[test]
    fn compiled_text_cleans_duplicate_target_mentions() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Torch Fiend Variant")
            .parse_text("{R}, Sacrifice this creature: Destroy target artifact.")
            .expect("parse torch fiend style text");
        let lines = crate::compiled_text::compiled_lines(&def);
        let joined = lines.join("\n");
        assert!(
            joined.contains("Destroy target artifact"),
            "compiled text should include destroy target artifact: {joined}"
        );
        assert!(
            !joined.contains("target target"),
            "compiled text should not duplicate target wording: {joined}"
        );
    }

    #[test]
    fn parse_restriction_line_now_errors_instead_of_silent_success() {
        let result = CardDefinitionBuilder::new(CardId::new(), "Restriction Variant")
            .parse_text("Activate only as a sorcery.");
        assert!(
            result.is_err(),
            "restriction-only line should fail instead of being silently ignored"
        );
    }

    #[test]
    fn parse_adamant_mana_spent_conditional_compiles_semantically() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Turn into a Pumpkin Variant")
            .parse_text(
                "Return target nonland permanent to its owner's hand. Draw a card.\nAdamant — If at least three blue mana was spent to cast this spell, create a Food token.",
            )
            .expect("adamant conditional should parse");

        let effects = def.spell_effect.as_ref().expect("spell effect");
        let conditional = effects
            .iter()
            .find_map(|effect| effect.downcast_ref::<crate::effects::ConditionalEffect>())
            .expect("expected conditional effect");

        assert!(
            matches!(
                conditional.condition,
                Condition::ManaSpentToCastThisSpellAtLeast {
                    amount: 3,
                    symbol: Some(ManaSymbol::Blue),
                }
            ),
            "expected mana-spent condition, got {:?}",
            conditional.condition
        );

        let lines = crate::compiled_text::compiled_lines(&def);
        let joined = lines.join("\n");
        assert!(
            joined.contains("If at least 3 {U} mana was spent to cast this spell"),
            "compiled text should reflect mana-spent condition: {joined}"
        );
    }

    #[test]
    fn parse_adamant_mana_spent_conditional_rejects_unparsed_tail() {
        let result = CardDefinitionBuilder::new(CardId::new(), "Broken Adamant Variant")
            .parse_text(
                "Adamant — If at least three blue mana was spent to cast this spell while you control a creature, create a Food token.",
            );
        assert!(
            result.is_err(),
            "unsupported predicate tail should fail parse instead of partial success"
        );
    }

    #[test]
    fn parse_no_spells_cast_last_turn_conditional_predicate() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Werewolf Transform Variant")
            .parse_text(
                "At the beginning of each upkeep, if no spells were cast last turn, transform this creature.",
            )
            .expect("no-spells-last-turn predicate should parse");

        let triggered = def
            .abilities
            .iter()
            .find_map(|ability| match &ability.kind {
                AbilityKind::Triggered(triggered) => Some(triggered),
                _ => None,
            })
            .expect("expected triggered ability");

        let conditional = triggered
            .effects
            .iter()
            .find_map(|effect| effect.downcast_ref::<crate::effects::ConditionalEffect>())
            .expect("expected conditional effect for no-spells predicate");

        assert!(
            matches!(conditional.condition, Condition::NoSpellsWereCastLastTurn),
            "expected no-spells-last-turn condition, got {:?}",
            conditional.condition
        );
    }

    #[test]
    fn create_token_render_includes_compiled_token_semantics() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Experimental Aviator Variant")
            .parse_text(
                "Flying\nWhen this creature enters, create two 1/1 colorless Thopter artifact creature tokens with flying.",
            )
            .expect("experimental aviator style text should parse");

        let lines = compiled_lines(&def);
        let joined = lines.join("\n").to_ascii_lowercase();
        assert!(
            joined.contains("create 2 1/1 colorless thopter artifact creature token with flying"),
            "compiled token text should include token characteristics and keyword, got: {joined}"
        );
    }

    #[test]
    fn parse_ring_tempts_now_errors_instead_of_silent_success() {
        let result = CardDefinitionBuilder::new(CardId::new(), "Ring Variant")
            .parse_text("The Ring tempts you.");
        assert!(
            result.is_err(),
            "ring tempts clause should fail instead of being silently ignored"
        );
    }

    #[test]
    fn from_text_with_metadata_no_longer_falls_back_on_parse_failure() {
        let result = CardDefinitionBuilder::new(CardId::new(), "Fallback Variant")
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Blue]]))
            .card_types(vec![CardType::Instant])
            .from_text_with_metadata("This line should not parse and used to fallback silently.");
        assert!(
            result.is_err(),
            "metadata parse should return an error instead of silent oracle-only fallback"
        );
    }

    #[test]
    fn parse_negated_untap_clause_compiles_to_untap_restriction() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Ty Lee Variant")
            .parse_text("When this creature enters, tap up to one target creature. It doesn't untap during its controller's untap step for as long as you control this creature.");
        let def = def.expect("Ty Lee-style untap restriction should parse");
        let triggered = def
            .abilities
            .iter()
            .find_map(|ability| match &ability.kind {
                AbilityKind::Triggered(triggered) => Some(triggered),
                _ => None,
            })
            .expect("expected triggered ability");
        let debug = format!("{:?}", triggered.effects);
        assert!(
            debug.contains("CantEffect"),
            "expected restriction effect, got {debug}"
        );
        assert!(
            debug.contains("Untap("),
            "expected untap restriction, got {debug}"
        );
        assert!(
            debug.contains("YouStopControllingThis"),
            "expected source-control duration, got {debug}"
        );
    }

    #[test]
    fn parse_ty_lee_named_duration_now_errors_instead_of_partial_compile() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Ty Lee")
            .parse_text(
                "When Ty Lee enters, tap up to one target creature. It doesn't untap during its controller's untap step for as long as you control Ty Lee.",
            )
            .expect("Ty Lee named-source duration should parse");
        let triggered = def
            .abilities
            .iter()
            .find_map(|ability| match &ability.kind {
                AbilityKind::Triggered(triggered) => Some(triggered),
                _ => None,
            })
            .expect("expected triggered ability");
        let debug = format!("{:?}", triggered.effects);
        assert!(
            debug.contains("CantEffect"),
            "expected untap restriction effect, got {debug}"
        );
    }

    #[test]
    fn parse_enters_tapped_unless_two_or_more_other_lands_line() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Shattered Sanctum Variant")
            .parse_text(
                "Shattered Sanctum enters the battlefield tapped unless you control two or more other lands.\n{T}: Add {W}.",
            )
            .expect("should parse conditional ETB line");

        let has_conditional_etb = def.abilities.iter().any(|ability| {
            matches!(
                &ability.kind,
                AbilityKind::Static(static_ability)
                    if static_ability.id()
                        == crate::static_abilities::StaticAbilityId::EntersTappedUnlessControlTwoOrMoreOtherLands
            )
        });
        assert!(
            has_conditional_etb,
            "expected conditional ETB static ability, got {:?}",
            def.abilities
        );
    }

    #[test]
    fn parse_enters_tapped_unless_two_or_fewer_other_lands_line() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Blackcleave Cliffs Variant")
            .parse_text(
                "This land enters tapped unless you control two or fewer other lands.\n{T}: Add {B}.",
            )
            .expect("should parse fast-land conditional ETB line");

        let has_conditional_etb = def.abilities.iter().any(|ability| {
            matches!(
                &ability.kind,
                AbilityKind::Static(static_ability)
                    if static_ability.id()
                        == crate::static_abilities::StaticAbilityId::EntersTappedUnlessControlTwoOrFewerOtherLands
            )
        });
        assert!(
            has_conditional_etb,
            "expected two-or-fewer-other-lands ETB static ability, got {:?}",
            def.abilities
        );
    }

    #[test]
    fn parse_enters_tapped_unless_two_or_more_basic_lands_line() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Canopy Vista Variant")
            .parse_text(
                "This land enters tapped unless you control two or more basic lands.\n{T}: Add {G}.",
            )
            .expect("should parse battle-land conditional ETB line");

        let has_conditional_etb = def.abilities.iter().any(|ability| {
            matches!(
                &ability.kind,
                AbilityKind::Static(static_ability)
                    if static_ability.id()
                        == crate::static_abilities::StaticAbilityId::EntersTappedUnlessControlTwoOrMoreBasicLands
            )
        });
        assert!(
            has_conditional_etb,
            "expected two-or-more-basic-lands ETB static ability, got {:?}",
            def.abilities
        );
    }

    #[test]
    fn parse_enters_tapped_unless_any_player_has_13_or_less_life_line() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Abandoned Campground Variant")
            .parse_text(
                "This land enters tapped unless a player has 13 or less life.\n{T}: Add {W}.",
            )
            .expect("should parse life-threshold conditional ETB line");

        let has_conditional_etb = def.abilities.iter().any(|ability| {
            matches!(
                &ability.kind,
                AbilityKind::Static(static_ability)
                    if static_ability.id()
                        == crate::static_abilities::StaticAbilityId::EntersTappedUnlessAPlayerHas13OrLessLife
            )
        });
        assert!(
            has_conditional_etb,
            "expected life-threshold ETB static ability, got {:?}",
            def.abilities
        );
    }

    #[test]
    fn parse_enters_tapped_unless_two_or_more_opponents_line() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Vault of Champions Variant")
            .parse_text(
                "This land enters tapped unless you have two or more opponents.\n{T}: Add {W}.",
            )
            .expect("should parse conditional ETB opponents line");

        let has_conditional_etb = def.abilities.iter().any(|ability| {
            matches!(
                &ability.kind,
                AbilityKind::Static(static_ability)
                    if static_ability.id()
                        == crate::static_abilities::StaticAbilityId::EntersTappedUnlessTwoOrMoreOpponents
            )
        });
        assert!(
            has_conditional_etb,
            "expected conditional-opponents ETB static ability, got {:?}",
            def.abilities
        );
    }

    #[test]
    fn parse_opponents_control_enter_tapped_preserves_controller_filter() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Frozen Aether Variant")
            .parse_text("Artifacts, creatures, and lands your opponents control enter the battlefield tapped.")
            .expect("should parse opponents-control enters tapped line");

        let rendered = compiled_lines(&def).join(" | ").to_ascii_lowercase();
        assert!(
            rendered.contains("opponent"),
            "expected rendered line to preserve opponents controller filter, got {rendered}"
        );
    }

    #[test]
    fn parse_played_by_your_opponents_enter_tapped_preserves_controller_filter() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Uphill Battle Variant")
            .parse_text("Creatures played by your opponents enter tapped.")
            .expect("should parse played-by-opponents enters tapped line");

        let rendered = compiled_lines(&def).join(" | ").to_ascii_lowercase();
        assert!(
            rendered.contains("opponent"),
            "expected rendered line to preserve opponents controller filter, got {rendered}"
        );
    }

    #[test]
    fn parse_pay_life_or_enter_tapped_shockland_line() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Blood Crypt Variant")
            .parse_text(
                "({T}: Add {B} or {R}.)\nAs this land enters, you may pay 2 life. If you don't, it enters tapped.",
            )
            .expect("shockland ETB payment line should parse");

        let has_pay_life_replacement = def.abilities.iter().any(|ability| {
            matches!(
                &ability.kind,
                AbilityKind::Static(static_ability)
                    if static_ability.id()
                        == crate::static_abilities::StaticAbilityId::PayLifeOrEnterTappedReplacement
            )
        });
        assert!(
            has_pay_life_replacement,
            "expected pay-life replacement ability, got {:?}",
            def.abilities
        );

        let has_broad_land_tap_replacement = def.abilities.iter().any(|ability| {
            matches!(
                &ability.kind,
                AbilityKind::Static(static_ability)
                    if static_ability.id()
                        == crate::static_abilities::StaticAbilityId::EnterTappedForFilter
            )
        });
        assert!(
            !has_broad_land_tap_replacement,
            "shockland text must not compile as broad land tap replacement: {:?}",
            def.abilities
        );
    }

    #[test]
    fn parse_pay_life_or_enter_tapped_requires_if_you_dont_tail() {
        let result = CardDefinitionBuilder::new(CardId::new(), "Broken Shockland Variant")
            .parse_text("As this land enters, you may pay 2 life. If you do, it enters tapped.");
        assert!(
            result.is_err(),
            "unsupported trailing clause must error instead of partial parse"
        );
    }

    #[test]
    fn tokenize_line_keeps_hybrid_slash_inside_mana_braces() {
        let tokens = tokenize_line("{U/R}, {T}: Add {C}.", 0);
        let words = words(&tokens);
        assert!(
            words.contains(&"u/r"),
            "hybrid mana symbol should preserve slash in token stream: {words:?}"
        );
    }

    #[test]
    fn parse_filter_land_mana_options_preserve_grouped_outputs() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Cascade Bluffs Variant")
            .parse_text("{T}: Add {C}.\n{U/R}, {T}: Add {U}{U}, {U}{R}, or {R}{R}.")
            .expect("filter-land mana line should parse");

        let lines = compiled_lines(&def);
        let mana_lines = lines
            .iter()
            .filter(|line| line.starts_with("Mana ability"))
            .collect::<Vec<_>>();
        assert_eq!(
            mana_lines.len(),
            4,
            "expected 4 mana abilities, got {lines:?}"
        );

        assert!(
            lines
                .iter()
                .any(|line| line.contains("{U/R}, {T}, Add {U}{U}")),
            "missing {{U}}{{U}} filtered output: {lines:?}"
        );
        assert!(
            lines
                .iter()
                .any(|line| line.contains("{U/R}, {T}, Add {U}{R}")),
            "missing {{U}}{{R}} filtered output: {lines:?}"
        );
        assert!(
            lines
                .iter()
                .any(|line| line.contains("{U/R}, {T}, Add {R}{R}")),
            "missing {{R}}{{R}} filtered output: {lines:?}"
        );
    }

    #[test]
    fn parse_mana_vault_upkeep_pay_clause_includes_pay_mana_effect() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Mana Vault Trigger Variant")
            .parse_text("At the beginning of your upkeep, you may pay {4}. If you do, untap this.")
            .expect("mana vault upkeep line should parse");

        let triggered = def
            .abilities
            .iter()
            .find_map(|ability| match &ability.kind {
                AbilityKind::Triggered(triggered) => Some(triggered),
                _ => None,
            })
            .expect("expected triggered ability");

        let debug = format!("{:?}", triggered.effects);
        assert!(
            debug.contains("PayManaEffect"),
            "expected pay mana effect, got {debug}"
        );
        assert!(
            debug.contains("UntapEffect"),
            "expected untap effect in if-you-do branch, got {debug}"
        );
    }

    #[test]
    fn parse_energy_pay_clause_includes_pay_energy_effect() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Energy Pay Trigger Variant")
            .parse_text(
                "Whenever this creature attacks, you may pay {E}. If you do, put a +1/+1 counter on this creature.",
            )
            .expect("energy pay trigger line should parse");

        let triggered = def
            .abilities
            .iter()
            .find_map(|ability| match &ability.kind {
                AbilityKind::Triggered(triggered) => Some(triggered),
                _ => None,
            })
            .expect("expected triggered ability");

        let debug = format!("{:?}", triggered.effects);
        assert!(
            debug.contains("PayEnergyEffect"),
            "expected pay energy effect, got {debug}"
        );
        assert!(
            debug.contains("PutCountersEffect"),
            "expected +1/+1 counter effect in if-you-do branch, got {debug}"
        );
    }

    #[test]
    fn parse_get_energy_equal_to_tagged_spell_mana_value() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Electrosiphon Variant")
            .parse_text("Counter target spell. You get an amount of {E} (energy counters) equal to its mana value.")
            .expect("mana-value-scaled energy clause should parse");

        let effects = def.spell_effect.as_ref().expect("spell effects");
        let energy = effects
            .iter()
            .find_map(|effect| effect.downcast_ref::<EnergyCountersEffect>())
            .expect("expected EnergyCountersEffect");

        match &energy.count {
            Value::ManaValueOf(spec) => match spec.as_ref() {
                ChooseSpec::Tagged(tag) => assert_eq!(tag.as_str(), IT_TAG),
                other => panic!("expected tagged mana-value reference, got {other:?}"),
            },
            other => panic!("expected mana-value scaling for energy counters, got {other:?}"),
        }
    }

    #[test]
    fn parse_add_black_for_each_creature_in_graveyard_compiles_scaled_mana() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Crypt Probe")
            .parse_text("Add {B} for each creature card in your graveyard.")
            .expect("dynamic add-mana line should parse");
        let effects = def.spell_effect.as_ref().expect("spell effects");
        assert_eq!(effects.len(), 1, "expected exactly one spell effect");

        let add_scaled = effects[0]
            .downcast_ref::<AddScaledManaEffect>()
            .expect("expected AddScaledManaEffect");
        assert_eq!(add_scaled.mana, vec![ManaSymbol::Black]);
        assert_eq!(add_scaled.player, PlayerFilter::You);

        match &add_scaled.amount {
            Value::Count(filter) => {
                assert_eq!(filter.zone, Some(Zone::Graveyard));
                assert_eq!(filter.owner, Some(PlayerFilter::You));
                assert!(
                    filter.card_types.contains(&CardType::Creature),
                    "expected creature type filter, got {:?}",
                    filter.card_types
                );
            }
            other => panic!("expected graveyard creature count, got {other:?}"),
        }
    }

    #[test]
    fn parse_activated_add_for_each_creature_compiles_scaled_mana() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Gaea Probe")
            .parse_text("{T}: Add {G} for each creature you control.")
            .expect("for-each mana ability should parse");

        let mana_ability = def
            .abilities
            .iter()
            .find_map(|ability| match &ability.kind {
                AbilityKind::Mana(mana) => Some(mana),
                _ => None,
            })
            .expect("expected mana ability");

        assert!(
            mana_ability.mana.is_empty(),
            "scaled mana should compile via effects, got direct mana {:?}",
            mana_ability.mana
        );
        let effects = mana_ability
            .effects
            .as_ref()
            .expect("scaled mana ability should have effects");
        let add_scaled = effects
            .iter()
            .find_map(|effect| effect.downcast_ref::<AddScaledManaEffect>())
            .expect("expected AddScaledManaEffect");
        assert_eq!(add_scaled.mana, vec![ManaSymbol::Green]);
        assert_eq!(add_scaled.player, PlayerFilter::You);
        match &add_scaled.amount {
            Value::Count(filter) => {
                assert!(
                    filter.card_types.contains(&CardType::Creature),
                    "expected creature filter, got {:?}",
                    filter.card_types
                );
                assert_eq!(filter.controller, Some(PlayerFilter::You));
            }
            other => panic!("expected count-based scaling, got {other:?}"),
        }

        let lines = compiled_lines(&def);
        let mana_line = lines
            .iter()
            .find(|line| line.starts_with("Mana ability"))
            .expect("expected mana line");
        assert!(
            mana_line.contains("for each"),
            "compiled text should preserve for-each semantics: {mana_line}"
        );
    }

    #[test]
    fn parse_activated_add_for_each_swamp_compiles_scaled_mana() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Coffers Probe")
            .parse_text("{2}, {T}: Add {B} for each Swamp you control.")
            .expect("for-each swamp mana ability should parse");

        let mana_ability = def
            .abilities
            .iter()
            .find_map(|ability| match &ability.kind {
                AbilityKind::Mana(mana) => Some(mana),
                _ => None,
            })
            .expect("expected mana ability");

        let effects = mana_ability
            .effects
            .as_ref()
            .expect("scaled mana ability should have effects");
        let add_scaled = effects
            .iter()
            .find_map(|effect| effect.downcast_ref::<AddScaledManaEffect>())
            .expect("expected AddScaledManaEffect");
        assert_eq!(add_scaled.mana, vec![ManaSymbol::Black]);
        match &add_scaled.amount {
            Value::Count(filter) => {
                assert!(
                    filter.subtypes.contains(&Subtype::Swamp),
                    "expected swamp subtype filter, got {:?}",
                    filter.subtypes
                );
                assert_eq!(filter.controller, Some(PlayerFilter::You));
            }
            other => panic!("expected count-based scaling, got {other:?}"),
        }
    }

    #[test]
    fn parse_activated_add_equal_to_devotion_compiles_scaled_mana() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Karametra Probe")
            .parse_text("{T}: Add an amount of {G} equal to your devotion to green.")
            .expect("devotion mana ability should parse");

        let mana_ability = def
            .abilities
            .iter()
            .find_map(|ability| match &ability.kind {
                AbilityKind::Mana(mana) => Some(mana),
                _ => None,
            })
            .expect("expected mana ability");

        assert!(
            mana_ability.mana.is_empty(),
            "devotion-scaled mana should compile via effects"
        );
        let effects = mana_ability
            .effects
            .as_ref()
            .expect("expected devotion mana effects");
        let add_scaled = effects
            .iter()
            .find_map(|effect| effect.downcast_ref::<AddScaledManaEffect>())
            .expect("expected AddScaledManaEffect");
        assert_eq!(add_scaled.mana, vec![ManaSymbol::Green]);
        assert_eq!(
            add_scaled.amount,
            Value::Devotion {
                player: PlayerFilter::You,
                color: crate::color::Color::Green,
            }
        );

        let lines = compiled_lines(&def);
        let mana_line = lines
            .iter()
            .find(|line| line.starts_with("Mana ability"))
            .expect("expected mana line");
        assert!(
            mana_line.contains("devotion to green"),
            "compiled text should preserve devotion semantics: {mana_line}"
        );
    }

    #[test]
    fn parse_spell_add_equal_to_devotion_compiles_scaled_mana() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Devotion Ritual Probe")
            .parse_text("Add an amount of {R} equal to your devotion to red.")
            .expect("devotion ritual line should parse");
        let effects = def.spell_effect.as_ref().expect("spell effects");
        assert_eq!(effects.len(), 1, "expected exactly one spell effect");
        let add_scaled = effects[0]
            .downcast_ref::<AddScaledManaEffect>()
            .expect("expected AddScaledManaEffect");
        assert_eq!(add_scaled.mana, vec![ManaSymbol::Red]);
        assert_eq!(
            add_scaled.amount,
            Value::Devotion {
                player: PlayerFilter::You,
                color: crate::color::Color::Red,
            }
        );
    }

    #[test]
    fn parse_add_equal_to_source_power_compiles_scaled_mana() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Viridian Joiner Variant")
            .parse_text("{T}: Add an amount of {G} equal to this creature's power.")
            .expect("power-scaled mana ability should parse");

        let mana_ability = def
            .abilities
            .iter()
            .find_map(|ability| match &ability.kind {
                AbilityKind::Mana(mana) => Some(mana),
                _ => None,
            })
            .expect("expected mana ability");
        let effects = mana_ability
            .effects
            .as_ref()
            .expect("expected scaled mana effects");
        let add_scaled = effects
            .iter()
            .find_map(|effect| effect.downcast_ref::<AddScaledManaEffect>())
            .expect("expected AddScaledManaEffect");
        assert_eq!(add_scaled.mana, vec![ManaSymbol::Green]);
        assert_eq!(
            add_scaled.amount,
            Value::PowerOf(Box::new(ChooseSpec::Source))
        );
    }

    #[test]
    fn parse_add_equal_to_sacrificed_creature_mana_value_uses_sacrifice_cost_tag() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Szeras Variant")
            .parse_text(
                "{T}, Sacrifice another creature: Add an amount of {B} equal to the sacrificed creature's mana value.",
            )
            .expect("sacrifice-scaled mana ability should parse");

        let mana_ability = def
            .abilities
            .iter()
            .find_map(|ability| match &ability.kind {
                AbilityKind::Mana(mana) => Some(mana),
                _ => None,
            })
            .expect("expected mana ability");
        let effects = mana_ability
            .effects
            .as_ref()
            .expect("expected scaled mana effects");
        let add_scaled = effects
            .iter()
            .find_map(|effect| effect.downcast_ref::<AddScaledManaEffect>())
            .expect("expected AddScaledManaEffect");
        assert_eq!(add_scaled.mana, vec![ManaSymbol::Black]);
        match &add_scaled.amount {
            Value::ManaValueOf(spec) => match spec.as_ref() {
                ChooseSpec::Tagged(tag) => assert_eq!(tag.as_str(), "sacrifice_cost_0"),
                other => panic!("expected sacrifice-cost tag reference, got {other:?}"),
            },
            other => panic!("expected mana-value scaling, got {other:?}"),
        }
    }

    #[test]
    fn parse_destroy_same_mana_value_as_sacrificed_creature_uses_sacrifice_cost_tag() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Sanguine Praetor Variant")
            .parse_text(
                "{B}, Sacrifice a creature: Destroy each creature with the same mana value as the sacrificed creature.",
            )
            .expect("same-mana-value destroy ability should parse");

        let activated = def
            .abilities
            .iter()
            .find_map(|ability| match &ability.kind {
                AbilityKind::Activated(ability) => Some(ability),
                _ => None,
            })
            .expect("expected activated ability");
        let destroy = activated
            .effects
            .iter()
            .find_map(|effect| effect.downcast_ref::<DestroyEffect>())
            .expect("expected destroy effect");

        let ChooseSpec::All(filter) = &destroy.spec else {
            panic!("expected destroy-all filter");
        };

        let tag_constraint = filter
            .tagged_constraints
            .iter()
            .find(|constraint| {
                matches!(
                    constraint.relation,
                    crate::filter::TaggedOpbjectRelation::SameManaValueAsTagged
                )
            })
            .expect("expected same-mana-value tagged constraint");
        assert_eq!(tag_constraint.tag.as_str(), "sacrifice_cost_0");
    }

    #[test]
    fn parse_add_that_much_colorless_uses_previous_effect_count() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Mana Seism Variant")
            .parse_text("Sacrifice any number of lands, then add that much {C}.")
            .expect("that-much mana spell should parse");

        let effects = def.spell_effect.as_ref().expect("expected spell effects");
        let add_scaled = effects
            .iter()
            .find_map(|effect| effect.downcast_ref::<AddScaledManaEffect>())
            .expect("expected AddScaledManaEffect");
        assert_eq!(add_scaled.mana, vec![ManaSymbol::Colorless]);
        assert!(
            matches!(
                add_scaled.amount,
                Value::EffectValue(_) | Value::EffectValueOffset(_, _) | Value::EventValue(_)
            ),
            "expected dynamic backreference amount, got {:?}",
            add_scaled.amount
        );
    }

    #[test]
    fn parse_add_x_any_one_color_where_count_keeps_dynamic_amount() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Harabaz Druid Variant")
            .parse_text(
                "{T}: Add X mana of any one color, where X is the number of Allies you control.",
            )
            .expect("dynamic any-one-color mana line should parse");

        let mana_ability = def
            .abilities
            .iter()
            .find_map(|ability| match &ability.kind {
                AbilityKind::Mana(mana) => Some(mana),
                _ => None,
            })
            .expect("expected mana ability");
        let effects = mana_ability
            .effects
            .as_ref()
            .expect("dynamic any-one-color mana should compile via effects");
        let add_any_one = effects
            .iter()
            .find_map(|effect| effect.downcast_ref::<AddManaOfAnyOneColorEffect>())
            .expect("expected AddManaOfAnyOneColorEffect");
        match &add_any_one.amount {
            Value::Count(filter) => {
                assert_eq!(filter.controller, Some(PlayerFilter::You));
                assert!(
                    filter.subtypes.contains(&Subtype::Ally),
                    "expected ally subtype filter, got {:?}",
                    filter.subtypes
                );
            }
            other => panic!("expected count-based amount, got {other:?}"),
        }

        let lines = compiled_lines(&def);
        let mana_line = lines
            .iter()
            .find(|line| line.starts_with("Mana ability"))
            .expect("expected mana ability line");
        assert!(
            mana_line.contains("any one color"),
            "compiled text should preserve any-one-color semantics: {mana_line}"
        );
        assert!(
            !mana_line.contains("{X}{X}"),
            "compiled text should not duplicate X as mana symbols: {mana_line}"
        );
    }

    #[test]
    fn parse_add_any_combination_of_two_colors_keeps_amount_and_restriction() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Lumberjack Variant")
            .parse_text(
                "{T}, Sacrifice a Forest: Add three mana in any combination of {R} and/or {G}.",
            )
            .expect("restricted any-combination mana ability should parse");

        let mana_ability = def
            .abilities
            .iter()
            .find_map(|ability| match &ability.kind {
                AbilityKind::Mana(mana) => Some(mana),
                _ => None,
            })
            .expect("expected mana ability");
        let effects = mana_ability
            .effects
            .as_ref()
            .expect("restricted any-combination mana should compile via effects");
        let add_any = effects
            .iter()
            .find_map(|effect| effect.downcast_ref::<AddManaOfAnyColorEffect>())
            .expect("expected AddManaOfAnyColorEffect");
        assert_eq!(add_any.amount, Value::Fixed(3));
        let colors = add_any
            .available_colors
            .as_ref()
            .expect("expected restricted colors");
        assert!(
            colors.contains(&crate::color::Color::Red)
                && colors.contains(&crate::color::Color::Green)
                && colors.len() == 2,
            "expected red/green restriction, got {colors:?}"
        );

        let lines = compiled_lines(&def);
        let mana_line = lines
            .iter()
            .find(|line| line.starts_with("Mana ability"))
            .expect("expected mana ability line");
        assert!(
            mana_line.contains("in any combination of {R} and/or {G}"),
            "compiled text should preserve restricted color combination, got: {mana_line}"
        );
    }

    #[test]
    fn parse_add_any_color_that_opponent_land_could_produce_compiles_restricted_mana_effect() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Exotic Orchard Variant")
            .parse_text(
                "{T}: Add one mana of any color that a land an opponent controls could produce.",
            )
            .expect("land-could-produce mana line should parse");

        let mana_ability = def
            .abilities
            .iter()
            .find_map(|ability| match &ability.kind {
                AbilityKind::Mana(mana) => Some(mana),
                _ => None,
            })
            .expect("expected mana ability");
        let effects = mana_ability
            .effects
            .as_ref()
            .expect("restricted mana ability should compile via effects");
        let restricted = effects
            .iter()
            .find_map(|effect| effect.downcast_ref::<AddManaOfLandProducedTypesEffect>())
            .expect("expected AddManaOfLandProducedTypesEffect");
        assert_eq!(restricted.amount, Value::Fixed(1));
        assert_eq!(restricted.player, PlayerFilter::You);
        assert!(
            !restricted.allow_colorless,
            "any color clause must not allow colorless"
        );
        assert!(
            !restricted.same_type,
            "any color clause should allow independent color choices"
        );
        assert!(
            restricted.land_filter.card_types.contains(&CardType::Land),
            "expected land filter, got {:?}",
            restricted.land_filter
        );
        assert_eq!(
            restricted.land_filter.controller,
            Some(PlayerFilter::Opponent),
            "expected opponent-controlled land filter"
        );

        let lines = compiled_lines(&def);
        let mana_line = lines
            .iter()
            .find(|line| line.starts_with("Mana ability"))
            .expect("expected mana ability line");
        assert!(
            mana_line.contains("could produce"),
            "compiled text should preserve could-produce clause, got {mana_line}"
        );
    }

    #[test]
    fn parse_add_any_type_that_gate_you_control_could_produce_keeps_type_semantics() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Gond Gate Variant")
            .parse_text("{T}: Add one mana of any type that a Gate you control could produce.")
            .expect("gate could-produce mana line should parse");

        let mana_ability = def
            .abilities
            .iter()
            .find_map(|ability| match &ability.kind {
                AbilityKind::Mana(mana) => Some(mana),
                _ => None,
            })
            .expect("expected mana ability");
        let effects = mana_ability
            .effects
            .as_ref()
            .expect("restricted mana ability should compile via effects");
        let restricted = effects
            .iter()
            .find_map(|effect| effect.downcast_ref::<AddManaOfLandProducedTypesEffect>())
            .expect("expected AddManaOfLandProducedTypesEffect");
        assert!(
            restricted.allow_colorless,
            "any type clause must allow colorless"
        );
        assert_eq!(
            restricted.land_filter.controller,
            Some(PlayerFilter::You),
            "expected you-control filter for gates"
        );
        assert!(
            restricted.land_filter.subtypes.contains(&Subtype::Gate),
            "expected gate subtype filter, got {:?}",
            restricted.land_filter
        );
    }

    #[test]
    fn parse_mana_ability_activate_only_if_you_control_an_artifact() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Spire Variant")
            .parse_text(
                "{T}: Add {C}.\n{T}, Pay 1 life: Add one mana of any color. Activate only if you control an artifact.",
            )
            .expect("artifact-gated mana ability should parse");

        let lines = compiled_lines(&def);
        let gated = lines
            .iter()
            .find(|line| {
                line.starts_with("Mana ability")
                    && line.contains("Pay 1 life")
                    && line.contains("Activate only if you control an artifact")
            })
            .expect("expected mana line with artifact activation condition");
        assert!(
            gated.contains("Add one mana of any color"),
            "expected gated rainbow mana text, got: {gated}"
        );
    }

    #[test]
    fn parse_add_any_color_with_unsupported_trailing_clause_fails() {
        let err = CardDefinitionBuilder::new(CardId::new(), "Broken Orchard Variant")
            .parse_text(
                "{T}: Add one mana of any color that a land an opponent controls could produce unless it's your turn.",
            )
            .expect_err("unsupported could-produce tail should fail");
        let message = format!("{err:?}");
        assert!(
            message.contains("unsupported trailing mana clause"),
            "expected strict-tail parse error, got: {message}"
        );
    }

    #[test]
    fn parse_spell_cost_increase_per_target_beyond_first_line() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Fireball Variant")
            .parse_text("This spell costs {1} more to cast for each target beyond the first.")
            .expect("fireball cost line should parse");

        let has_target_cost_increase = def.abilities.iter().any(|ability| {
            matches!(
                &ability.kind,
                AbilityKind::Static(static_ability)
                    if static_ability.id()
                        == crate::static_abilities::StaticAbilityId::CostIncreasePerAdditionalTarget
            )
        });
        assert!(
            has_target_cost_increase,
            "expected additional-target cost increase ability, got {:?}",
            def.abilities
        );
    }

    #[test]
    fn parse_object_filter_rejects_controller_only_phrase() {
        let tokens = tokenize_line("you control", 0);
        let result = parse_object_filter(&tokens, false);
        assert!(
            result.is_err(),
            "controller-only phrase should not be treated as a valid object target"
        );
    }

    #[test]
    fn parse_set_life_total_from_text() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Blessed Wind")
            .parse_text("Target player's life total becomes 20.")
            .expect("parse set life total");

        let effects = def.spell_effect.expect("spell effect");
        assert!(
            effects
                .iter()
                .any(|e| e.downcast_ref::<SetLifeTotalEffect>().is_some()),
            "should include set life total effect"
        );
    }

    #[test]
    fn parse_discard_random_from_text() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Specter's Wail")
            .parse_text("Target player discards a card at random.")
            .expect("parse random discard");

        let effects = def.spell_effect.expect("spell effect");
        let discard = effects
            .iter()
            .find_map(|e| e.downcast_ref::<DiscardEffect>())
            .expect("should include discard effect");
        assert!(discard.random, "discard should be random");
    }

    #[test]
    fn parse_mindculling_draw_then_target_opponent_discards() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Mindculling Variant")
            .parse_text("You draw two cards and target opponent discards two cards.")
            .expect("parse mindculling-like text");

        let effects = def.spell_effect.expect("spell effect");
        let draw = effects
            .iter()
            .find_map(|e| e.downcast_ref::<DrawCardsEffect>())
            .expect("should include draw effect");
        assert_eq!(draw.count, Value::Fixed(2));
        assert_eq!(draw.player, PlayerFilter::You);

        let discard = effects
            .iter()
            .find_map(|e| e.downcast_ref::<DiscardEffect>())
            .expect("should include discard effect");
        assert_eq!(discard.count, Value::Fixed(2));
        assert_eq!(
            discard.player,
            PlayerFilter::Target(Box::new(PlayerFilter::Opponent))
        );
    }

    #[test]
    fn parse_target_player_shuffles_library_activation() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Soldier of Fortune Variant")
            .parse_text("{R}, {T}: Target player shuffles their library.")
            .expect("parse shuffle-target-player activation");

        let debug = format!("{:?}", def.abilities);
        assert!(
            debug.contains("ShuffleLibraryEffect"),
            "expected shuffle-library effect, got {debug}"
        );
        assert!(
            !debug.contains("TargetOnlyEffect"),
            "shuffle activation must not compile as target-only effect, got {debug}"
        );
    }

    #[test]
    fn parse_draw_then_discard_renders_then_clause() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Bazaar Variant")
            .card_types(vec![CardType::Creature])
            .parse_text("{T}: Draw two cards, then discard three cards.")
            .expect("parse draw-then-discard activation");

        let lines = compiled_lines(&def);
        let activated = lines
            .iter()
            .find(|line| line.starts_with("Activated ability"))
            .expect("expected activated ability line");
        assert!(
            activated.contains("draw 2 cards, then discard 3 cards"),
            "expected draw/discard to render as one then-clause, got {activated}"
        );
    }

    #[test]
    fn parse_additional_cost_line_renders_before_spell_effects() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Altar's Reap Variant")
            .parse_text(
                "As an additional cost to cast this spell, sacrifice a creature.\nDraw two cards.",
            )
            .expect("parse additional cost spell");

        let lines = compiled_lines(&def);
        let cost_idx = lines
            .iter()
            .position(|line| line.starts_with("As an additional cost to cast this spell:"))
            .expect("expected spell cost effects line");
        let effects_idx = lines
            .iter()
            .position(|line| line.starts_with("Spell effects:"))
            .expect("expected spell effects line");
        assert!(
            cost_idx < effects_idx,
            "spell cost effects should render before spell effects: {lines:?}"
        );
    }

    #[test]
    fn parse_non_mana_additional_cost_modifier_as_static_ability() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Brutal Suppression Variant")
            .parse_text(
                "Activated abilities of nontoken Rebels cost an additional \"Sacrifice a land\" to activate.",
            )
            .expect("parse non-mana additional cost modifier");

        assert!(
            def.spell_effect.is_none(),
            "expected no spell effects, got {:?}",
            def.spell_effect
        );
        let lines = compiled_lines(&def);
        assert!(
            lines.iter().any(|line| {
                line.contains(
                    "Activated abilities of nontoken Rebels cost an additional \"Sacrifice a land\" to activate."
                )
            }),
            "expected static additional cost modifier text, got {lines:?}"
        );
    }

    #[test]
    fn parse_drought_additional_cost_lines_as_static_abilities() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Drought Variant")
            .parse_text(
                "At the beginning of your upkeep, sacrifice this enchantment unless you pay {W}{W}.\nSpells cost an additional \"Sacrifice a Swamp\" to cast for each black mana symbol in their mana costs.\nActivated abilities cost an additional \"Sacrifice a Swamp\" to activate for each black mana symbol in their activation costs.",
            )
            .expect("parse drought-style additional costs");

        assert!(
            def.spell_effect.is_none(),
            "expected no top-level spell effects, got {:?}",
            def.spell_effect
        );
        let lines = compiled_lines(&def);
        assert!(
            lines.iter().any(|line| line.starts_with("Triggered ability 1:")),
            "expected upkeep trigger to remain triggered ability, got {lines:?}"
        );
        assert!(
            lines.iter().any(|line| line.contains("Spells cost an additional \"Sacrifice a Swamp\" to cast")),
            "expected spell additional cost line as static ability, got {lines:?}"
        );
        assert!(
            lines
                .iter()
                .any(|line| line.contains("Activated abilities cost an additional \"Sacrifice a Swamp\" to activate")),
            "expected activated additional cost line as static ability, got {lines:?}"
        );
    }

    #[test]
    fn parse_search_library_named_card_with_leading_you_may() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Wretched Throng Variant")
            .parse_text(
                "When this creature dies, you may search your library for a card named Wretched Throng, reveal it, put it into your hand, then shuffle.",
            )
            .expect("parse wretched throng search line");

        let triggered = def
            .abilities
            .iter()
            .find_map(|ability| match &ability.kind {
                AbilityKind::Triggered(triggered) => Some(triggered),
                _ => None,
            })
            .expect("expected triggered ability");

        let debug = format!("{:?}", triggered.effects);
        assert!(
            debug.contains("MayEffect"),
            "expected optional may wrapper, got {debug}"
        );
        assert!(
            debug.contains("SearchLibraryEffect"),
            "expected search effect, got {debug}"
        );
        assert!(
            debug.contains("destination: Hand"),
            "expected hand destination, got {debug}"
        );
        assert!(
            debug.contains("reveal: true"),
            "expected reveal=true, got {debug}"
        );
        assert!(
            debug.contains("ShuffleLibraryEffect"),
            "expected shuffle effect, got {debug}"
        );
        assert!(
            debug.contains("name: Some(\"wretched throng\")"),
            "expected named search filter, got {debug}"
        );
    }

    #[test]
    fn parse_put_counters_on_each_creature_you_control_compiles_foreach() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Saga Counter Variant")
            .parse_text("Put a +1/+1 counter on each creature you control.")
            .expect("parse put counter on each");

        let effects = def.spell_effect.expect("spell effect");
        let foreach = effects
            .iter()
            .find_map(|effect| effect.downcast_ref::<ForEachObject>())
            .expect("expected ForEachObject");
        assert_eq!(foreach.filter, ObjectFilter::creature().you_control());

        let put = foreach
            .effects
            .iter()
            .find_map(|effect| effect.downcast_ref::<PutCountersEffect>())
            .expect("expected nested PutCountersEffect");
        assert_eq!(put.target, ChooseSpec::Iterated);
    }

    #[test]
    fn parse_put_counters_on_each_of_any_number_of_targets() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Molten Duplication Variant")
            .parse_text(
                "When this creature enters, put a +1/+1 counter on each of any number of target creatures and a charge counter on each of any number of target artifacts.",
            )
            .expect("parse molten-duplication-style counters");

        let triggered = def
            .abilities
            .iter()
            .find_map(|ability| match &ability.kind {
                AbilityKind::Triggered(triggered) => Some(triggered),
                _ => None,
            })
            .expect("expected triggered ability");

        let put_effects: Vec<&PutCountersEffect> = triggered
            .effects
            .iter()
            .filter_map(|effect| effect.downcast_ref::<PutCountersEffect>())
            .collect();
        assert_eq!(put_effects.len(), 2, "expected two put-counters effects");

        let plus_one = put_effects
            .iter()
            .find(|effect| effect.counter_type == CounterType::PlusOnePlusOne)
            .expect("expected +1/+1 counters effect");
        assert!(
            plus_one.target.count().is_any_number(),
            "expected any-number target count for creature counters"
        );

        let charge = put_effects
            .iter()
            .find(|effect| effect.counter_type == CounterType::Charge)
            .expect("expected charge counters effect");
        assert!(
            charge.target.count().is_any_number(),
            "expected any-number target count for artifact counters"
        );

        let lines = compiled_lines(&def);
        let line = lines
            .iter()
            .find(|line| line.contains("When this permanent enters the battlefield"))
            .expect("expected triggered compiled line");
        assert!(
            line.contains("Put 1 +1/+1 counter(s) on any number of target"),
            "expected any-number target text for +1/+1 counters, got {line}"
        );
        assert!(
            line.contains("Put 1 Charge counter(s) on any number of target"),
            "expected any-number target text for charge counters, got {line}"
        );
    }

    #[test]
    fn parse_realm_seekers_keeps_x_and_source_counter_cost() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Realm Seekers Variant")
            .parse_text(
                "This creature enters with X +1/+1 counters on it, where X is the total number of cards in all players' hands.\n{2}{G}, Remove a +1/+1 counter from this creature: Search your library for a land card, reveal it, put it into your hand, then shuffle.",
            )
            .expect("parse realm seekers text");

        let lines = compiled_lines(&def);
        assert!(
            lines.iter().any(|line| line.contains("PlusOnePlusOne")),
            "expected +1/+1 counters in static line, got {lines:?}"
        );
        let static_debug = format!("{:?}", def.abilities);
        assert!(
            static_debug.contains("zone: Some(Hand)"),
            "expected where-X hand-count filter to be compiled, got {static_debug}"
        );

        let activated = def
            .abilities
            .iter()
            .find_map(|ability| match &ability.kind {
                AbilityKind::Activated(activated) => Some(activated),
                _ => None,
            })
            .expect("expected activated ability");
        let cost_debug = format!("{:?}", activated.mana_cost);
        assert!(
            cost_debug.contains("RemoveCountersCost"),
            "expected source remove-counters cost, got {cost_debug}"
        );

        let effect_debug = format!("{:?}", activated.effects);
        assert!(
            effect_debug.contains("SearchLibraryEffect"),
            "expected search library effect, got {effect_debug}"
        );
    }

    #[test]
    fn parse_remove_counters_from_among_creatures_cost() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Tayam Cost Variant")
            .card_types(vec![CardType::Creature])
            .parse_text("{3}, Remove three counters from among creatures you control: Draw a card.")
            .expect("distributed remove-counters cost should parse");

        let activated = def
            .abilities
            .iter()
            .find_map(|ability| match &ability.kind {
                AbilityKind::Activated(activated) => Some(activated),
                _ => None,
            })
            .expect("expected activated ability");
        let cost_debug = format!("{:?}", activated.mana_cost);
        assert!(
            cost_debug.contains("RemoveAnyCountersAmongCost"),
            "expected distributed counter-removal cost, got {cost_debug}"
        );
        assert!(
            cost_debug.contains("count: 3"),
            "expected count 3 in distributed counter-removal cost, got {cost_debug}"
        );
        assert!(
            cost_debug.contains("card_types: [Creature]"),
            "expected creature filter in distributed counter-removal cost, got {cost_debug}"
        );
    }

    #[test]
    fn parse_remove_charge_counter_from_this_artifact_cost() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Ox Cart Variant")
            .card_types(vec![CardType::Artifact])
            .parse_text(
                "{1}, {T}, Remove a charge counter from this artifact: Destroy target creature.",
            )
            .expect("source-specific remove-counter artifact cost should parse");

        let activated = def
            .abilities
            .iter()
            .find_map(|ability| match &ability.kind {
                AbilityKind::Activated(activated) => Some(activated),
                _ => None,
            })
            .expect("expected activated ability");
        let cost_debug = format!("{:?}", activated.mana_cost);
        assert!(
            cost_debug.contains("RemoveCountersCost"),
            "expected source remove-counters cost, got {cost_debug}"
        );
        assert!(
            !cost_debug.contains("RemoveAnyCountersAmongCost"),
            "expected source-specific cost, got distributed remove cost: {cost_debug}"
        );
    }

    #[test]
    fn parse_exile_all_creatures_with_power_constraint() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Power Exile Variant")
            .parse_text("Exile all creatures with power 4 or greater.")
            .expect("parse exile all creatures with power filter");

        let effects = def.spell_effect.expect("spell effect");
        let exile = effects
            .iter()
            .find_map(|effect| effect.downcast_ref::<ExileEffect>())
            .expect("expected exile effect");
        let ChooseSpec::All(filter) = &exile.spec else {
            panic!("expected non-targeted exile-all spec");
        };
        assert_eq!(
            filter.power,
            Some(crate::filter::Comparison::GreaterThanOrEqual(4))
        );
    }

    #[test]
    fn parse_destroy_each_nonland_permanent_compiles_as_destroy_all() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Destroy Each Variant")
            .parse_text("Destroy each nonland permanent with mana value X or less.")
            .expect("parse destroy-each clause");

        let effects = def.spell_effect.expect("spell effect");
        let destroy = effects
            .iter()
            .find_map(|effect| effect.downcast_ref::<DestroyEffect>())
            .expect("expected destroy effect");
        let debug = format!("{destroy:?}");
        assert!(
            debug.contains("spec: All("),
            "expected non-targeted destroy-all spec, got {debug}"
        );
        assert!(
            debug.contains("mana value X or less") || debug.contains("mana_value"),
            "expected mana-value filter to remain on destroy-all spec, got {debug}"
        );
    }

    #[test]
    fn parse_destroy_target_creature_with_flying_keeps_keyword_filter() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Destroy Flying Variant")
            .parse_text("Destroy target creature with flying.")
            .expect("parse flying-qualified destroy");

        let debug = format!("{:?}", def.spell_effect);
        assert!(
            debug.contains("static_abilities: [Flying]"),
            "expected flying ability filter in runtime effect, got {debug}"
        );

        let lines = compiled_lines(&def);
        let spell_line = lines
            .iter()
            .find(|line| line.starts_with("Spell effects:"))
            .expect("expected spell effects line");
        assert!(
            spell_line.contains("Destroy target creature with flying"),
            "expected rendered destroy filter to include flying qualifier, got {spell_line}"
        );
    }

    #[test]
    fn parse_destroy_target_creature_with_islandwalk_keeps_marker_filter() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Destroy Islandwalk Variant")
            .parse_text("Destroy target creature with islandwalk.")
            .expect("parse islandwalk-qualified destroy");

        let debug = format!("{:?}", def.spell_effect);
        assert!(
            debug.contains("custom_static_markers: [\"islandwalk\"]"),
            "expected islandwalk marker filter in runtime effect, got {debug}"
        );

        let lines = compiled_lines(&def);
        let spell_line = lines
            .iter()
            .find(|line| line.starts_with("Spell effects:"))
            .expect("expected spell effects line");
        assert!(
            spell_line.contains("Destroy target creature with islandwalk"),
            "expected rendered destroy filter to include islandwalk qualifier, got {spell_line}"
        );
    }

    #[test]
    fn parse_destroy_target_creature_without_flying_keeps_exclusion_filter() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Destroy NonFlying Variant")
            .parse_text("Destroy target creature without flying.")
            .expect("parse without-flying destroy");

        let debug = format!("{:?}", def.spell_effect);
        assert!(
            debug.contains("excluded_static_abilities: [Flying]"),
            "expected flying exclusion in runtime effect, got {debug}"
        );

        let lines = compiled_lines(&def);
        let spell_line = lines
            .iter()
            .find(|line| line.starts_with("Spell effects:"))
            .expect("expected spell effects line");
        assert!(
            spell_line.contains("Destroy target creature without flying"),
            "expected rendered destroy filter to include without-flying qualifier, got {spell_line}"
        );
    }

    #[test]
    fn parse_target_player_exiles_flashback_cards_from_their_graveyard() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Tombfire Variant")
            .parse_text("Target player exiles all cards with flashback from their graveyard.")
            .expect("parse tombfire-like text");

        let effects = def.spell_effect.expect("spell effects");
        assert!(
            effects
                .iter()
                .any(|effect| effect.downcast_ref::<TargetOnlyEffect>().is_some()),
            "expected explicit target-context effect for target player"
        );

        let exile = effects
            .iter()
            .find_map(|effect| effect.downcast_ref::<ExileEffect>())
            .expect("expected exile effect");
        let ChooseSpec::All(filter) = &exile.spec else {
            panic!("expected non-targeted exile-all spec");
        };

        assert_eq!(filter.zone, Some(Zone::Graveyard));
        assert_eq!(filter.owner, Some(PlayerFilter::target_player()));
        assert_eq!(
            filter.alternative_cast,
            Some(crate::filter::AlternativeCastKind::Flashback)
        );
    }

    #[test]
    fn parse_each_player_sacrifices_creature_of_their_choice_renders_compactly() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Fleshbag Variant")
            .card_types(vec![CardType::Creature])
            .parse_text(
                "When this creature enters, each player sacrifices a creature of their choice.",
            )
            .expect("parse fleshbag-style trigger");

        let lines = compiled_lines(&def);
        let triggered = lines
            .iter()
            .find(|line| line.starts_with("Triggered ability"))
            .expect("expected triggered ability line");
        assert!(
            triggered.contains("Each player sacrifices a creature of their choice"),
            "expected compact each-player sacrifice text, got {triggered}"
        );
    }

    #[test]
    fn parse_each_opponent_sacrifices_creature_of_their_choice_renders_compactly() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Each Opponent Sacrifice Variant")
            .parse_text("Each opponent sacrifices a creature of their choice.")
            .expect("parse each-opponent sacrifice text");

        let lines = compiled_lines(&def);
        let spell_line = lines
            .iter()
            .find(|line| line.starts_with("Spell effects:"))
            .expect("expected spell effects line");
        assert!(
            spell_line.contains("Each opponent sacrifices a creature of their choice"),
            "expected compact each-opponent sacrifice text, got {spell_line}"
        );
    }

    #[test]
    fn parse_sacrifice_cost_renders_compactly() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Stronghold Assassin Variant")
            .card_types(vec![CardType::Creature])
            .parse_text("{T}, Sacrifice a creature: Destroy target nonblack creature.")
            .expect("parse sacrifice-cost activation");

        let lines = compiled_lines(&def);
        let activated = lines
            .iter()
            .find(|line| line.starts_with("Activated ability"))
            .expect("expected activated ability line");
        assert!(
            activated.contains("you sacrifice a creature"),
            "expected compact sacrifice-cost text, got {activated}"
        );
        assert!(
            !activated.contains("chooses exactly 1"),
            "should not leak choose-object internals in cost text: {activated}"
        );
    }

    #[test]
    fn parse_etb_sacrifice_creature_renders_compactly() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Kjeldoran Dead Variant")
            .card_types(vec![CardType::Creature])
            .parse_text("When this creature enters, sacrifice a creature.")
            .expect("parse etb sacrifice line");

        let lines = compiled_lines(&def);
        let triggered = lines
            .iter()
            .find(|line| line.starts_with("Triggered ability"))
            .expect("expected triggered ability line");
        assert!(
            triggered.contains("you sacrifice a creature"),
            "expected compact sacrifice text, got {triggered}"
        );
        assert!(
            !triggered.contains("chooses exactly 1"),
            "should not leak choose-object internals in trigger text: {triggered}"
        );
    }

    #[test]
    fn parse_unless_action_sacrifice_clause_renders_compactly() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Unless Sacrifice Variant")
            .parse_text("Sacrifice a creature unless you discard a card.")
            .expect("parse unless-action sacrifice clause");

        let lines = compiled_lines(&def);
        let spell_line = lines
            .iter()
            .find(|line| line.starts_with("Spell effects:"))
            .expect("expected spell effects line");
        assert!(
            spell_line.contains("you sacrifice a creature unless you discard a card"),
            "expected compact unless-action sacrifice text, got {spell_line}"
        );
        assert!(
            !spell_line.contains("chooses exactly 1"),
            "should not leak choose-object internals in unless-action text: {spell_line}"
        );
    }

    #[test]
    fn parse_unless_controller_pays_life_keeps_unless_branch() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Unless Life Variant")
            .card_types(vec![CardType::Creature])
            .parse_text("{T}: Tap target creature unless its controller pays 2 life.")
            .expect("parse unless-pays-life clause");

        let lines = compiled_lines(&def);
        let activated = lines
            .iter()
            .find(|line| line.starts_with("Activated ability"))
            .expect("expected activated ability line");
        assert!(
            activated.contains("unless"),
            "expected unless branch in render, got {activated}"
        );
        assert!(
            activated.contains("2 life"),
            "expected life-payment alternative in render, got {activated}"
        );
    }

    #[test]
    fn parse_damage_unless_controller_has_source_deal_damage() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Blazing Salvo Variant")
            .parse_text(
                "This spell deals 3 damage to target creature unless that creature's controller has this spell deal 5 damage to them.",
            )
            .expect("parse damage-unless-controller alternative");

        let lines = compiled_lines(&def);
        let spell_line = lines
            .iter()
            .find(|line| line.starts_with("Spell effects:"))
            .expect("expected spell effects line");
        assert!(
            spell_line.contains("unless its controller")
                && spell_line.contains("Deal 5 damage"),
            "expected unless-controller alternative damage text, got {spell_line}"
        );
    }

    #[test]
    fn parse_equip_keyword_displays_as_keyword_ability() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Strider Harness Equip Variant")
            .parse_text(
                "Equip {1} ({1}: Attach to target creature you control. Equip only as a sorcery.)",
            )
            .expect("parse equip line");

        assert_eq!(def.abilities.len(), 1);
        let ability = &def.abilities[0];
        assert!(matches!(ability.kind, AbilityKind::Activated(_)));
        assert_eq!(ability.text.as_deref(), Some("Equip {1}"));

        let lines = compiled_lines(&def);
        assert!(
            lines
                .iter()
                .any(|line| line == "Keyword ability 1: Equip {1}"),
            "expected keyword ability line, got {:?}",
            lines
        );
    }

    #[test]
    fn parse_skip_turn_from_text() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Meditate")
            .parse_text("You skip your next turn.")
            .expect("parse skip turn");

        let effects = def.spell_effect.expect("spell effect");
        assert!(
            effects
                .iter()
                .any(|e| e.downcast_ref::<SkipTurnEffect>().is_some()),
            "should include skip turn effect"
        );
    }

    #[test]
    fn parse_skip_draw_step_from_text() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Fatigue")
            .parse_text("Target player skips their next draw step.")
            .expect("parse skip draw step");

        let effects = def.spell_effect.expect("spell effect");
        assert!(
            effects
                .iter()
                .any(|e| e.downcast_ref::<SkipDrawStepEffect>().is_some()),
            "should include skip draw step effect"
        );
    }

    #[test]
    fn parse_skip_combat_phases_from_text() {
        let def = CardDefinitionBuilder::new(CardId::new(), "False Peace")
            .parse_text("Target player skips all combat phases of their next turn.")
            .expect("parse skip combat phases");

        let effects = def.spell_effect.expect("spell effect");
        assert!(
            effects
                .iter()
                .any(|e| e.downcast_ref::<SkipCombatPhasesEffect>().is_some()),
            "should include skip combat phases effect"
        );
    }

    #[test]
    fn parse_skip_next_combat_phase_this_turn_from_text() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Moment of Silence")
            .parse_text("Target player skips their next combat phase this turn.")
            .expect("parse skip next combat phase this turn");

        let effects = def.spell_effect.expect("spell effect");
        assert!(
            effects.iter().any(|e| e
                .downcast_ref::<SkipNextCombatPhaseThisTurnEffect>()
                .is_some()),
            "should include skip-next-combat-phase-this-turn effect"
        );
    }

    #[test]
    fn parse_exile_all_mixed_zones_split_clause() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Decree of Annihilation Probe")
            .parse_text(
                "Exile all artifacts, creatures, and lands from the battlefield, all cards from all graveyards, and all cards from all hands.",
            )
            .expect("parse mixed-zone exile-all split clause");

        let lines = compiled_lines(&def);
        let joined = lines.join(" ");
        assert!(
            joined.contains("Exile all artifact")
                && joined.contains("Exile all creature")
                && joined.contains("Exile all land"),
            "expected battlefield-type exile split coverage, got {joined}"
        );
        assert!(
            joined.contains("Exile all card from a graveyard"),
            "expected graveyard exile segment, got {joined}"
        );
        assert!(
            joined.contains("Exile all card in hand"),
            "expected hand exile segment, got {joined}"
        );
    }

    #[test]
    fn parse_spell_cast_from_graveyard_trigger_text() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Secrets of the Dead Probe")
            .parse_text("Whenever you cast a spell from your graveyard, draw a card.")
            .expect("parse spell-cast-from-graveyard trigger");

        let lines = compiled_lines(&def);
        let joined = lines.join(" ");
        assert!(
            joined.contains("Whenever you cast a spell from your graveyard"),
            "expected graveyard origin qualifier in trigger text, got {joined}"
        );
    }

    #[test]
    fn parse_spell_cast_another_during_your_turn_trigger_text() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Geralf Qualifier Probe")
            .parse_text(
                "Whenever you cast a spell during your turn other than your first spell that turn, draw a card.",
            )
            .expect("parse qualified spell-cast trigger");

        let lines = compiled_lines(&def);
        let joined = lines.join(" ");
        assert!(
            joined.contains("Whenever you cast another spell during your turn"),
            "expected spell-order + turn qualifier in trigger text, got {joined}"
        );
    }

    #[test]
    fn parse_spell_cast_second_each_turn_trigger_text() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Second Spell Probe")
            .parse_text("Whenever you cast your second spell each turn, draw a card.")
            .expect("parse second-spell-each-turn trigger");

        let lines = compiled_lines(&def);
        let joined = lines.join(" ");
        assert!(
            joined.contains("Whenever you cast another spell"),
            "expected second-spell qualifier in trigger text, got {joined}"
        );
    }

    #[test]
    fn parse_player_casts_their_second_spell_trigger_text() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Any Player Second Spell Probe")
            .parse_text("Whenever a player casts their second spell each turn, draw a card.")
            .expect("parse player-second-spell-each-turn trigger");

        let lines = compiled_lines(&def);
        let joined = lines.join(" ");
        assert!(
            joined.contains("Whenever a player casts another spell"),
            "expected player second-spell qualifier in trigger text, got {joined}"
        );
    }

    #[test]
    fn parse_put_counter_for_each_filter_on_target() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Moldgraf Millipede Probe")
            .parse_text(
                "When this creature enters, mill three cards, then put a +1/+1 counter on this creature for each creature card in your graveyard.",
            )
            .expect("parse put counter for-each filter on target");

        let lines = compiled_lines(&def);
        let joined = lines.join(" ");
        assert!(
            joined.contains(
                "Put a +1/+1 counter on this creature for each creature card in your graveyard"
            ),
            "expected counter target + graveyard count phrase, got {joined}"
        );
    }

    #[test]
    fn parse_pest_token_subtype_in_token_rendering() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Pest Summoning Probe")
            .parse_text(
                "Create two 1/1 black and green Pest creature tokens with \"When this token dies, you gain 1 life.\"",
            )
            .expect("parse pest token creation with dies lifegain text");

        let lines = compiled_lines(&def);
        let joined = lines.join(" ");
        assert!(
            joined.contains("Pest creature token"),
            "expected Pest subtype to be retained in token rendering, got {joined}"
        );
    }

    #[test]
    fn parse_token_with_prowess_keyword_in_rendering() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Prowess Token Probe")
            .parse_text("Create a 4/4 red Dragon Elemental creature token with flying and prowess.")
            .expect("parse token creation with prowess");

        let lines = compiled_lines(&def);
        let joined = lines.join(" ");
        assert!(
            joined.contains("Prowess"),
            "expected prowess keyword in token rendering, got: {joined}"
        );
    }

    #[test]
    fn parse_token_with_noncreature_cast_damage_trigger_reminder() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Wizard Trigger Token Probe")
            .parse_text(
                "When this creature enters, create a 0/1 black Wizard creature token with \"Whenever you cast a noncreature spell, this token deals 1 damage to each opponent.\"",
            )
            .expect("parse token creation with noncreature-cast trigger reminder");

        let abilities_debug = format!("{:?}", def.abilities);
        assert!(
            abilities_debug.contains("CreateTokenEffect"),
            "expected token creation in compiled trigger effects, got {abilities_debug}"
        );
        assert!(
            !abilities_debug.contains("ForPlayersEffect"),
            "inline token reminder should not become immediate for-each-opponent damage, got {abilities_debug}"
        );
        assert!(
            abilities_debug.contains("SpellCastTrigger")
                && abilities_debug.contains("without_type: [Creature]")
                && abilities_debug.contains("IteratedPlayer"),
            "expected token to receive a noncreature-spell cast trigger that pings each opponent, got {abilities_debug}"
        );
    }

    #[test]
    fn parse_rejects_enters_as_copy_with_except_ability_clause() {
        let result = CardDefinitionBuilder::new(CardId::new(), "Evil Twin Variant")
            .parse_text(
                "You may have this creature enter as a copy of any creature on the battlefield, except it has \"{U}{B}, {T}: Destroy target creature with the same name as this creature.\"",
            );
        assert!(
            result.is_err(),
            "unsupported enters-as-copy replacement should fail parse instead of producing partial statement semantics"
        );
    }

    #[test]
    fn parse_rejects_divided_damage_distribution_clause() {
        let result = CardDefinitionBuilder::new(CardId::new(), "Fire at Will Variant").parse_text(
            "Fire at Will deals 3 damage divided as you choose among one, two, or three target attacking or blocking creatures.",
        );
        assert!(
            result.is_err(),
            "unsupported divided-damage distribution should fail parse instead of collapsing into a single target damage effect"
        );
    }

    #[test]
    fn parse_verb_leading_line_does_not_fallback_to_static_clause() {
        let result = CardDefinitionBuilder::new(CardId::new(), "Nahiri Lithoforming Variant")
            .parse_text(
                "Sacrifice X lands. For each land sacrificed this way, draw a card. You may play X additional lands this turn. Lands you control enter tapped this turn.",
            );
        assert!(
            result.is_err(),
            "unsupported verb-leading spell text should fail parse instead of falling back to a partial static ability"
        );
    }

    #[test]
    fn parse_choose_leading_line_does_not_fallback_to_static_clause() {
        let result = CardDefinitionBuilder::new(CardId::new(), "Rebuild City Variant").parse_text(
            "Choose target land. Create three tokens that are copies of it, except they're 3/3 creatures in addition to their other types and they have vigilance and menace.",
        );
        assert!(
            result.is_err(),
            "unsupported choose-leading spell text should fail parse instead of falling back to a partial static ability"
        );
    }

    #[test]
    fn parse_anthem_and_keyword_trailing_condition_applies_to_keyword() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Delirium Anthem Keyword Variant")
            .parse_text(
                "This creature gets +1/+0 and has first strike as long as there are four or more card types among cards in your graveyard.",
            )
            .expect("parse anthem + keyword with trailing condition");

        let joined = compiled_lines(&def).join(" ");
        assert!(
            joined.contains("First strike"),
            "expected first strike keyword in compiled text, got {joined}"
        );
        assert!(
            joined.contains("as long as there are four or more card types among cards in your graveyard"),
            "expected trailing condition to be preserved for anthem+keyword line, got {joined}"
        );
    }

    #[test]
    fn parse_rejects_spent_to_cast_conditional_clause() {
        let result = CardDefinitionBuilder::new(CardId::new(), "Firespout Variant").parse_text(
            "Firespout deals 3 damage to each creature without flying if {R} was spent to cast this spell and 3 damage to each creature with flying if {G} was spent to cast this spell.",
        );
        assert!(
            result.is_err(),
            "unsupported spent-to-cast conditional clause should fail parse instead of partially compiling damage effects"
        );
    }

    #[test]
    fn parse_rejects_would_enter_replacement_clause() {
        let result = CardDefinitionBuilder::new(CardId::new(), "Mistcaller Variant").parse_text(
            "Sacrifice this creature: Until end of turn, if a nontoken creature would enter and it wasn't cast, exile it instead.",
        );
        assert!(
            result.is_err(),
            "unsupported would-enter replacement clause should fail parse instead of collapsing to an immediate exile effect"
        );
    }

    #[test]
    fn parse_rejects_different_mana_value_constraint_clause() {
        let result =
            CardDefinitionBuilder::new(CardId::new(), "Agadeem Awakening Variant").parse_text(
                "Return from your graveyard to the battlefield any number of target creature cards that each have a different mana value X or less.",
            );
        assert!(
            result.is_err(),
            "unsupported different-mana-value target constraint should fail parse instead of collapsing target restrictions"
        );
    }

    #[test]
    fn parse_rejects_most_common_color_constraint_clause() {
        let result = CardDefinitionBuilder::new(CardId::new(), "Barrin Unmaking Variant")
            .parse_text(
                "Return target permanent to its owner's hand if that permanent shares a color with the most common color among all permanents or a color tied for most common.",
            );
        assert!(
            result.is_err(),
            "unsupported most-common-color conditional should fail parse instead of dropping the condition"
        );
    }

    #[test]
    fn parse_rejects_power_vs_count_conditional_clause() {
        let result = CardDefinitionBuilder::new(CardId::new(), "Unified Strike Variant")
            .parse_text(
                "Exile target attacking creature if its power is less than or equal to the number of Soldiers on the battlefield.",
            );
        assert!(
            result.is_err(),
            "unsupported power-vs-count conditional should fail parse instead of narrowing target type"
        );
    }

    #[test]
    fn parse_rejects_put_into_graveyards_from_battlefield_count_clause() {
        let result = CardDefinitionBuilder::new(CardId::new(), "Structural Assault Variant")
            .parse_text(
                "Destroy all artifacts, then this spell deals damage to each creature equal to the number of artifacts that were put into graveyards from the battlefield this turn.",
            );
        assert!(
            result.is_err(),
            "unsupported put-into-graveyards-from-battlefield count clause should fail parse instead of collapsing to a graveyard destroy effect"
        );
    }

    #[test]
    fn parse_spell_with_it_has_token_trigger_stays_as_spell_effects() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Make Mischief Variant")
            .parse_text(
                "This spell deals 1 damage to any target. Create a 1/1 red Devil creature token. It has \"When this token dies, it deals 1 damage to any target.\"",
            )
            .expect("parse spell with token dies trigger rider");

        assert!(
            def.abilities.is_empty(),
            "spell line with token trigger rider should not collapse into a granted static ability"
        );
        let spell_debug = format!("{:?}", def.spell_effect);
        assert!(
            spell_debug.contains("DealDamageEffect") && spell_debug.contains("CreateTokenEffect"),
            "expected direct damage + token creation effects, got {spell_debug}"
        );
    }

    #[test]
    fn parse_rejects_standalone_token_reminder_sentence() {
        let result = CardDefinitionBuilder::new(CardId::new(), "Sound the Call Variant").parse_text(
            "Create a 1/1 green Wolf creature token. It has \"This token gets +1/+1 for each card named Sound the Call in each graveyard.\"",
        );
        assert!(
            result.is_err(),
            "standalone token reminder sentences should fail parse until they compile to token semantics instead of reminder-text masking"
        );
    }

    #[test]
    fn parse_cumulative_upkeep_line_as_keyword_marker() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Cumulative Upkeep Variant")
            .parse_text("Cumulative upkeep—Sacrifice a creature.")
            .expect("parse cumulative upkeep keyword line");

        assert!(
            def.spell_effect.is_none(),
            "cumulative upkeep line should compile as an ability, not a spell effect"
        );
        let joined = compiled_lines(&def).join(" ");
        assert!(
            joined.to_ascii_lowercase().contains("cumulative upkeep"),
            "expected cumulative upkeep text in compiled abilities, got {joined}"
        );
    }

    #[test]
    fn parse_rejects_investigate_for_each_clause() {
        let result = CardDefinitionBuilder::new(CardId::new(), "Declaration Variant").parse_text(
            "Exile target creature and all other creatures its controller controls with the same name as that creature. That player investigates for each nontoken creature exiled this way.",
        );
        assert!(
            result.is_err(),
            "unsupported investigate-for-each clause should fail parse instead of collapsing to a single investigate"
        );
    }

    #[test]
    fn parse_same_name_exile_until_source_leaves_clause() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Deputy Variant")
            .parse_text(
                "Exile target nonland permanent an opponent controls and all other nonland permanents that player controls with the same name as that permanent until this creature leaves the battlefield.",
            )
            .expect("same-name exile-until clause should parse");

        let lines = compiled_lines(&def);
        let spell_line = lines
            .iter()
            .find(|line| line.starts_with("Spell effects:"))
            .expect("expected spell effects line");
        assert!(
            spell_line.contains("until this permanent leaves the battlefield"),
            "compiled text should preserve exile duration, got {spell_line}"
        );
    }

    #[test]
    fn parse_exile_target_until_source_leaves_clause() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Static Prison Variant")
            .parse_text(
                "Exile target nonland permanent an opponent controls until this enchantment leaves the battlefield.",
            )
            .expect("target exile-until clause should parse");

        let lines = compiled_lines(&def);
        let spell_line = lines
            .iter()
            .find(|line| line.starts_with("Spell effects:"))
            .expect("expected spell effects line");
        assert!(
            spell_line.contains("until this permanent leaves the battlefield"),
            "compiled text should preserve exile-until duration, got {spell_line}"
        );
    }

    #[test]
    fn parse_rejects_phase_out_until_leaves_clause() {
        let result = CardDefinitionBuilder::new(CardId::new(), "Oubliette Variant").parse_text(
            "When this enchantment enters, target creature phases out until this enchantment leaves the battlefield.",
        );
        assert!(
            result.is_err(),
            "unsupported phase-out-until-leaves clause should fail parse instead of mis-targeting objects"
        );
    }

    #[test]
    fn parse_rejects_same_name_as_another_in_hand_clause() {
        let result = CardDefinitionBuilder::new(CardId::new(), "Hint Insanity Variant").parse_text(
            "Target player reveals their hand. That player discards all nonland cards with the same name as another card in their hand.",
        );
        assert!(
            result.is_err(),
            "unsupported same-name-as-another-in-hand discard clause should fail parse instead of discarding entire hand"
        );
    }

    #[test]
    fn parse_rejects_for_each_mana_from_spent_clause() {
        let result =
            CardDefinitionBuilder::new(CardId::new(), "Cataclysmic Prospecting Variant").parse_text(
                "For each mana from a Desert spent to cast this spell, create a tapped Treasure token.",
            );
        assert!(
            result.is_err(),
            "unsupported for-each-mana-from-spent clause should fail parse instead of iterating over spells"
        );
    }

    #[test]
    fn parse_labeled_trigger_line_as_triggered_ability() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Heroic Label Variant")
            .parse_text(
                "Heroic — Whenever you cast a spell that targets this creature, put a +1/+1 counter on this creature, then scry 1.",
            )
            .expect("parse heroic labeled trigger");

        assert!(
            def.spell_effect.is_none(),
            "labeled trigger should not collapse into spell-effect text"
        );
        let triggered = def
            .abilities
            .iter()
            .find_map(|ability| match &ability.kind {
                AbilityKind::Triggered(triggered) => Some(triggered),
                _ => None,
            })
            .expect("expected triggered ability from labeled trigger line");
        let effects_debug = format!("{:?}", triggered.effects);
        assert!(
            effects_debug.contains("PutCountersEffect") && effects_debug.contains("ScryEffect"),
            "expected +1/+1 counter and scry effects in heroic trigger, got {effects_debug}"
        );
    }

    #[test]
    fn parse_labeled_trigger_line_preserves_once_each_turn_suffix() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Reach Label Variant")
            .parse_text(
                "Reach\nThe Allagan Eye — Whenever one or more other creatures and/or artifacts you control die, draw a card. This ability triggers only once each turn.",
            )
            .expect("parse reach line plus labeled once-each-turn trigger");

        assert!(
            def.abilities
                .iter()
                .any(|ability| matches!(ability.kind, AbilityKind::Static(_))),
            "expected the standalone Reach line to compile to a static ability"
        );
        let triggered = def
            .abilities
            .iter()
            .find_map(|ability| match &ability.kind {
                AbilityKind::Triggered(triggered) => Some(triggered),
                _ => None,
            })
            .expect("expected triggered ability from labeled trigger line");
        assert!(
            triggered.once_each_turn,
            "expected 'This ability triggers only once each turn' suffix to set once_each_turn=true"
        );
    }

    #[test]
    fn parse_gain_control_target_creature_from_text() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Threaten")
            .parse_text("Gain control of target creature until end of turn.")
            .expect("parse gain control");

        let effects = def.spell_effect.expect("spell effect");
        assert!(
            effects.iter().any(|e| {
                e.downcast_ref::<GainControlEffect>().is_some()
                    || format!("{e:?}").contains("GainControlEffect")
            }),
            "should include gain control effect"
        );
    }

    #[test]
    fn reject_conditional_gain_control_clause_instead_of_partial_parse() {
        let err = CardDefinitionBuilder::new(CardId::new(), "Exert Influence Variant")
            .parse_text(
                "Gain control of target creature if its power is less than or equal to the number of colors of mana spent to cast this spell.",
            )
            .expect_err("conditional gain-control clause should fail until fully supported");
        let debug = format!("{err:?}");
        assert!(
            debug.contains("unsupported conditional gain-control clause"),
            "expected strict conditional gain-control rejection, got {debug}"
        );
    }

    #[test]
    fn parse_commander_creatures_have_granted_static_anthem() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Inspiring Leader Variant")
            .parse_text(
                "Commander creatures you own have \"Creature tokens you control get +2/+2.\"",
            )
            .expect("commander granted static anthem should parse");
        let debug = format!("{:?}", def.abilities);
        assert!(
            debug.contains("GrantAbility"),
            "expected commander-granted static ability wrapper, got {debug}"
        );
        let lines = crate::compiled_text::compiled_lines(&def);
        let joined = lines.join("\n");
        assert!(
            joined.contains("commander creature")
                && joined.contains("you own")
                && joined.contains("token creature")
                && joined.contains("+2/+2"),
            "expected rendered commander grant context, got {joined}"
        );
    }

    #[test]
    fn parse_commander_creatures_have_granted_cost_reduction() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Acolyte of Bahamut Variant")
            .parse_text(
                "Commander creatures you own have \"The first Dragon spell you cast each turn costs {2} less to cast.\"",
            )
            .expect("commander granted cost-reduction static ability should parse");
        let debug = format!("{:?}", def.abilities);
        assert!(
            debug.contains("GrantAbility"),
            "expected commander-granted static ability wrapper, got {debug}"
        );
        let lines = crate::compiled_text::compiled_lines(&def);
        let joined = lines.join("\n");
        assert!(
            joined.contains("commander creature")
                && joined.contains("you own")
                && joined.contains("dragon")
                && joined.contains("costs less"),
            "expected rendered granted cost reduction context, got {joined}"
        );
    }

    #[test]
    fn parse_threaten_style_activated_keeps_followup_haste_sentence() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Captivating Crew Variant")
            .parse_text(
                "{3}{R}: Gain control of target creature an opponent controls until end of turn. Untap that creature. It gains haste until end of turn. Activate only as a sorcery.",
            )
            .expect("threaten-style activated ability should parse");

        let lines = crate::compiled_text::compiled_lines(&def);
        let activated = lines
            .iter()
            .find(|line| line.contains("Activated ability"))
            .expect("expected activated ability line");
        assert!(
            activated.contains("Haste"),
            "expected followup haste clause to be preserved, got {activated}"
        );
    }

    #[test]
    fn parse_threaten_style_followup_keeps_multiple_keywords() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Lose Calm Variant")
            .parse_text(
                "Gain control of target creature until end of turn. Untap that creature. It gains haste and menace until end of turn.",
            )
            .expect("threaten-style followup with multiple keywords should parse");

        let lines = crate::compiled_text::compiled_lines(&def);
        let spell_line = lines
            .iter()
            .find(|line| line.starts_with("Spell effects:"))
            .expect("expected spell effects line");
        assert!(
            spell_line.contains("Haste") && spell_line.contains("Menace"),
            "expected both granted keywords to be preserved, got {spell_line}"
        );
    }

    #[test]
    fn parse_reveal_targets_hand_from_text() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Peek Variant")
            .parse_text("Target player reveals their hand.")
            .expect("parse reveal hand");

        let effects = def.spell_effect.expect("spell effect");
        assert!(
            effects
                .iter()
                .any(|e| e.downcast_ref::<LookAtHandEffect>().is_some()),
            "should include look-at-hand effect"
        );
    }

    #[test]
    fn parse_surveil_from_text() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Surveil Card")
            .parse_text("Surveil 1.")
            .expect("parse surveil");

        let effects = def.spell_effect.expect("spell effect");
        assert!(
            effects
                .iter()
                .any(|e| e.downcast_ref::<SurveilEffect>().is_some()),
            "should include surveil effect"
        );
    }

    #[test]
    fn parse_transform_from_text() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Werewolf Shift")
            .parse_text("Transform this creature.")
            .expect("parse transform");

        let effects = def.spell_effect.expect("spell effect");
        assert!(
            effects
                .iter()
                .any(|e| e.downcast_ref::<TransformEffect>().is_some()),
            "should include transform effect"
        );
    }

    #[test]
    fn parse_targeted_gets_modifier_as_spell_effect_not_static_anthem() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Deal Gone Bad Variant")
            .parse_text("Target creature gets -3/-3 until end of turn.")
            .expect("parse targeted gets modifier");

        let effects = def.spell_effect.expect("spell effect");
        assert!(
            effects
                .iter()
                .any(|e| e.downcast_ref::<ModifyPowerToughnessEffect>().is_some()),
            "should include targeted temporary power/toughness effect, got: {:?}",
            effects
        );
        assert!(
            def.abilities.is_empty(),
            "targeted temporary buff/debuff should not parse as static ability"
        );
    }

    #[test]
    fn parse_targeted_gets_dynamic_x_modifier() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Dynamic Gets Variant")
            .parse_text("Target creature gets +X/+0 until end of turn.")
            .expect("dynamic targeted gets should parse");

        let effects = def.spell_effect.expect("spell effect");
        let debug = format!("{effects:?}");
        assert!(
            debug.contains("ModifyPowerToughnessEffect"),
            "expected temporary pump effect, got {debug}"
        );
        assert!(
            debug.contains("power: X"),
            "expected dynamic X power modifier, got {debug}"
        );
        assert!(
            debug.contains("toughness: Fixed(0)"),
            "expected fixed +0 toughness modifier, got {debug}"
        );
    }

    #[test]
    fn parse_all_creatures_get_dynamic_plus_x_minus_x() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Flowstone Slide Variant")
            .parse_text("All creatures get +X/-X until end of turn.")
            .expect("global dynamic gets should parse");

        let effects = def.spell_effect.expect("spell effect");
        let debug = format!("{effects:?}");
        assert!(
            debug.contains("ModifyPowerToughnessAllEffect"),
            "expected global pump-all effect, got {debug}"
        );
        assert!(
            debug.contains("power: X"),
            "expected dynamic +X power modifier, got {debug}"
        );
        assert!(
            debug.contains("toughness: XTimes(-1)"),
            "expected dynamic -X toughness modifier, got {debug}"
        );
    }

    #[test]
    fn parse_activated_gets_dynamic_minus_x_plus_x() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Belbe's Armor Variant")
            .parse_text("{X}, {T}: Target creature gets -X/+X until end of turn.")
            .expect("activated dynamic gets should parse");
        let lines = crate::compiled_text::compiled_lines(&def);
        let joined = lines.join("\n");
        assert!(
            joined.contains("Activated ability"),
            "expected activated ability line, got {joined}"
        );
        assert!(
            joined.contains("X"),
            "expected dynamic X modifier in rendering, got {joined}"
        );
    }

    #[test]
    fn parse_targeted_gets_where_x_is_number_of_filter() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Where X Gets Variant")
            .parse_text("Target creature gets +X/+X until end of turn, where X is the number of creatures you control.")
            .expect("where-X targeted gets should parse");

        let effects = def.spell_effect.expect("spell effect");
        let debug = format!("{effects:?}");
        assert!(
            debug.contains("ModifyPowerToughnessEffect"),
            "expected targeted pump effect, got {debug}"
        );
        assert!(
            debug.contains("power: Count"),
            "expected where-X to compile into count value, got {debug}"
        );
        assert!(
            debug.contains("toughness: Count"),
            "expected where-X to compile into count value, got {debug}"
        );
    }

    #[test]
    fn reject_gets_where_x_requires_unsupported_signed_dynamic_replacement() {
        let err = CardDefinitionBuilder::new(CardId::new(), "Signed Where X Variant")
            .parse_text(
                "Each non-Vampire creature gets -X/-X until end of turn, where X is the number of Vampires you control.",
            )
            .expect_err("signed dynamic where-X should fail until represented exactly");
        let debug = format!("{err:?}");
        assert!(
            debug.contains("unsupported signed dynamic X replacement in gets clause")
                || debug.contains("unsupported parser line"),
            "expected strict where-X rejection, got {debug}"
        );
    }

    #[test]
    fn parse_enchanted_creature_pump_keeps_enchanted_subject_in_display() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Flowstone Embrace Variant")
            .card_types(vec![CardType::Enchantment])
            .subtypes(vec![Subtype::Aura])
            .parse_text("Enchant creature\n{T}: Enchanted creature gets +2/-2 until end of turn.")
            .expect("parse aura activated pump");

        let lines = compiled_lines(&def);
        assert!(
            lines.first().is_some_and(|line| line == "Enchant creature"),
            "expected enchant line first, got {lines:?}"
        );
        let activated = lines
            .iter()
            .find(|line| line.starts_with("Activated ability"))
            .expect("expected activated ability line");
        assert!(
            activated.contains("enchanted creature get +2/-2"),
            "expected enchanted subject in compiled display, got {activated}"
        );
    }

    #[test]
    fn parse_metalcraft_self_buff_preserves_condition_and_subject() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Ardent Recruit Variant")
            .parse_text(
                "Metalcraft — This creature gets +2/+2 as long as you control three or more artifacts.",
            )
            .expect("parse metalcraft static buff");

        assert_eq!(def.abilities.len(), 1, "expected one static ability");
        let display = match &def.abilities[0].kind {
            AbilityKind::Static(static_ability) => static_ability.display(),
            other => panic!("expected static ability, got {other:?}"),
        };
        assert!(
            display.contains("this creature gets +2/+2"),
            "expected source-scoped anthem display, got: {display}"
        );
        assert!(
            display.contains("as long as you control three or more artifacts"),
            "expected condition to be preserved, got: {display}"
        );
    }

    #[test]
    fn parse_domain_self_buff_preserves_for_each_clause() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Kavu Scout Variant")
            .parse_text(
                "Domain — This creature gets +1/+0 for each basic land type among lands you control.",
            )
            .expect("parse domain static buff");

        assert_eq!(def.abilities.len(), 1, "expected one static ability");
        let display = match &def.abilities[0].kind {
            AbilityKind::Static(static_ability) => static_ability.display(),
            other => panic!("expected static ability, got {other:?}"),
        };
        assert!(
            display.contains(
                "this creature gets +1/+0 for each basic land type among lands you control"
            ),
            "expected dynamic domain display, got: {display}"
        );
    }

    #[test]
    fn parse_descend_condition_keeps_permanent_cards_qualifier() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Basking Capybara Variant")
            .parse_text(
                "Descend 4 — This creature gets +3/+0 as long as there are four or more permanent cards in your graveyard.",
            )
            .expect("parse descend static buff");

        assert_eq!(def.abilities.len(), 1, "expected one static ability");
        let display = match &def.abilities[0].kind {
            AbilityKind::Static(static_ability) => static_ability.display(),
            other => panic!("expected static ability, got {other:?}"),
        };
        assert!(
            display.contains("as long as there are four or more permanent cards in your graveyard"),
            "expected descend condition text to be preserved, got: {display}"
        );
    }

    #[test]
    fn parse_attached_for_each_self_buff_keeps_attachment_scope() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Kor Spiritdancer Variant")
            .parse_text("This creature gets +2/+2 for each Aura attached to it.")
            .expect("parse attached dynamic static buff");

        assert_eq!(def.abilities.len(), 1, "expected one static ability");
        let display = match &def.abilities[0].kind {
            AbilityKind::Static(static_ability) => static_ability.display(),
            other => panic!("expected static ability, got {other:?}"),
        };
        assert!(
            display.contains("for each aura"),
            "expected attachment-based scaling in display, got: {display}"
        );
        assert!(
            display.contains("attached to this creature"),
            "expected source attachment scope in display, got: {display}"
        );
    }

    #[test]
    fn parse_conditional_anthem_and_keyword_applies_condition_to_both() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Conditional Grant Variant")
            .parse_text(
                "As long as you control an artifact, this creature gets +1/+1 and has deathtouch.",
            )
            .expect("parse conditional anthem and keyword");

        assert_eq!(def.abilities.len(), 2, "expected two static abilities");
        let displays: Vec<String> = def
            .abilities
            .iter()
            .filter_map(|ability| match &ability.kind {
                AbilityKind::Static(static_ability) => Some(static_ability.display()),
                _ => None,
            })
            .collect();
        assert!(
            displays
                .iter()
                .any(|display| display.contains("this creature gets +1/+1")
                    && display.contains("as long as you control an artifact")),
            "expected conditional self buff ability, got: {displays:?}"
        );
        assert!(
            displays
                .iter()
                .any(|display| display.contains("has Deathtouch")
                    && display.contains("as long as you control an artifact")),
            "expected conditional grant ability, got: {displays:?}"
        );
    }

    #[test]
    fn parse_conditional_anthem_and_haste_keeps_pump_and_keyword() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Conditional Haste Variant")
            .parse_text(
                "As long as you control another multicolored permanent, this creature gets +1/+1 and has haste.",
            )
            .expect("parse conditional anthem and haste");

        assert_eq!(def.abilities.len(), 2, "expected two static abilities");
        let displays: Vec<String> = def
            .abilities
            .iter()
            .filter_map(|ability| match &ability.kind {
                AbilityKind::Static(static_ability) => Some(static_ability.display()),
                _ => None,
            })
            .collect();
        assert!(
            displays.iter().any(|display| {
                display.contains("this creature gets +1/+1")
                    && display.contains("as long as you control another multicolored permanent")
            }),
            "expected conditional self buff ability, got: {displays:?}"
        );
        assert!(
            displays.iter().any(|display| {
                display.contains("has Haste")
                    && display.contains("as long as you control another multicolored permanent")
            }),
            "expected conditional haste ability, got: {displays:?}"
        );
    }

    #[test]
    fn parse_conditional_multi_keyword_grant_keeps_all_keywords() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Conditional Multi Keyword Variant")
            .parse_text(
                "As long as you control an artifact, this creature has trample and indestructible.",
            )
            .expect("parse conditional multi-keyword grant");

        let displays: Vec<String> = def
            .abilities
            .iter()
            .filter_map(|ability| match &ability.kind {
                AbilityKind::Static(static_ability) => Some(static_ability.display()),
                _ => None,
            })
            .collect();
        assert!(
            displays.iter().any(|display| {
                display.contains("has Trample")
                    && display.contains("as long as you control an artifact")
            }),
            "expected conditional trample ability, got: {displays:?}"
        );
        assert!(
            displays.iter().any(|display| {
                display.contains("has Indestructible")
                    && display.contains("as long as you control an artifact")
            }),
            "expected conditional indestructible ability, got: {displays:?}"
        );
    }

    #[test]
    fn parse_static_anthem_with_terminal_period() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Dead Weight Style Variant")
            .parse_text("Enchanted creature gets -2/-2.")
            .expect("terminal period should not break static anthem parsing");

        assert_eq!(def.abilities.len(), 1, "expected one static ability");
        let display = match &def.abilities[0].kind {
            AbilityKind::Static(static_ability) => static_ability.display(),
            other => panic!("expected static ability, got {other:?}"),
        };
        assert!(
            display.contains("enchanted creature gets -2/-2"),
            "expected enchanted anthem display, got: {display}"
        );
    }

    #[test]
    fn parse_creatures_you_control_anthem_with_terminal_period() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Simple Team Anthem Variant")
            .parse_text("Creatures you control get +1/+1.")
            .expect("terminal period should not break team anthem parsing");

        assert_eq!(def.abilities.len(), 1, "expected one static ability");
        let display = match &def.abilities[0].kind {
            AbilityKind::Static(static_ability) => static_ability.display(),
            other => panic!("expected static ability, got {other:?}"),
        };
        assert!(
            display.contains("+1/+1"),
            "expected parsed anthem modifier in display, got: {display}"
        );
    }

    #[test]
    fn parse_granted_keyword_and_must_attack_clause_keeps_both_parts() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Hellraiser Variant")
            .parse_text("Creatures you control have haste and attack each combat if able.")
            .expect("parse granted keyword + must-attack line");

        let displays: Vec<String> = def
            .abilities
            .iter()
            .filter_map(|ability| match &ability.kind {
                AbilityKind::Static(static_ability) => Some(static_ability.display()),
                _ => None,
            })
            .collect();
        assert!(
            displays
                .iter()
                .any(|display| display.contains("have Haste")),
            "expected granted haste ability, got: {displays:?}"
        );
        assert!(
            displays
                .iter()
                .any(|display| display.contains("attack each combat if able")),
            "expected granted must-attack ability, got: {displays:?}"
        );
    }

    #[test]
    fn parse_static_gets_rejects_unsupported_trailing_clause() {
        let err = CardDefinitionBuilder::new(CardId::new(), "Unsupported Static Tail Variant")
            .parse_text("This creature gets +1/+1 unless you control an artifact.")
            .expect_err("unsupported static tail should fail parsing");

        let message = format!("{err:?}");
        assert!(
            message.contains("unsupported trailing anthem clause"),
            "expected trailing-clause parse error, got: {message}"
        );
    }

    #[test]
    fn parse_put_from_among_into_hand_fails_instead_of_misparsing() {
        let err = CardDefinitionBuilder::new(CardId::new(), "Ainok Wayfarer Variant")
            .parse_text(
                "When this creature enters, mill three cards. You may put a land card from among them into your hand. If you don't, put a +1/+1 counter on this creature.",
            )
            .expect_err("put-from-among clause should not silently parse as returning source");

        let message = format!("{err:?}");
        assert!(
            message.contains("unsupported put-from-among clause"),
            "expected strict put-from-among parse error, got {message}"
        );
    }

    #[test]
    fn parse_mill_with_trailing_clause_fails_instead_of_silently_partial_parsing() {
        let err = CardDefinitionBuilder::new(CardId::new(), "Midnight Tilling Variant")
            .parse_text("Mill four cards, then you may return a permanent card from among them to your hand.")
            .expect_err("mill with trailing from-among clause should fail until supported");

        let message = format!("{err:?}");
        assert!(
            message.contains("unsupported trailing mill clause"),
            "expected strict trailing-clause mill parse error, got {message}"
        );
    }

    #[test]
    fn parse_fireblast_style_alternative_cost_line_from_text() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Fireblast Variant")
            .parse_text(
                "You may sacrifice two Mountains rather than pay this spell's mana cost.\nFireblast deals 4 damage to any target.",
            )
            .expect("parse fireblast-style alternative cost");

        assert_eq!(def.alternative_casts.len(), 1);
        let alt = &def.alternative_casts[0];
        match alt {
            AlternativeCastingMethod::AlternativeCost {
                mana_cost,
                cost_effects,
                ..
            } => {
                assert!(mana_cost.is_none(), "fireblast alt cost should be no mana");
                let has_sacrifice = cost_effects
                    .iter()
                    .any(|effect| effect.downcast_ref::<SacrificeEffect>().is_some());
                assert!(
                    has_sacrifice,
                    "expected sacrifice effect in alternative cost"
                );
                let sacrifice = cost_effects
                    .iter()
                    .find_map(|effect| effect.downcast_ref::<SacrificeEffect>())
                    .expect("missing sacrifice effect");
                assert_eq!(sacrifice.count, Value::Fixed(2));
            }
            other => panic!("expected AlternativeCost, got {other:?}"),
        }

        let spell_effect = def.spell_effect.expect("spell effect");
        assert!(
            spell_effect.iter().any(|effect| effect
                .downcast_ref::<crate::effects::DealDamageEffect>()
                .is_some()),
            "expected damage spell effect"
        );
    }

    #[test]
    fn parse_zero_mana_alternative_cost_line_from_text() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Trap Variant")
            .parse_text("You may pay {0} rather than pay this spell's mana cost.\nDraw a card.")
            .expect("parse zero-mana alternative cost");

        assert_eq!(def.alternative_casts.len(), 1);
        let alt = &def.alternative_casts[0];
        match alt {
            AlternativeCastingMethod::AlternativeCost { mana_cost, .. } => {
                let mana = mana_cost.as_ref().expect("expected mana alt cost");
                assert_eq!(mana.to_oracle(), "{0}");
            }
            other => panic!("expected AlternativeCost, got {other:?}"),
        }
    }

    #[test]
    fn parse_alternative_cost_with_trailing_clause_fails() {
        let err = CardDefinitionBuilder::new(CardId::new(), "Treasure Alt Cost Variant")
            .parse_text(
                "You may pay {R}{G} rather than pay this spell's mana cost. Spend only mana produced by Treasures to cast it this way.",
            )
            .expect_err("alternative cost line with trailing clause should fail");

        let message = format!("{err:?}");
        assert!(
            message.contains("unsupported trailing clause after alternative cost"),
            "expected strict trailing-clause error, got {message}"
        );
    }

    #[test]
    fn parse_gain_control_for_as_long_as_you_control_source_duration() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Landfall Control Variant")
            .parse_text(
                "Whenever a land you control enters, you may gain control of target creature for as long as you control this creature.",
            )
            .expect("parse gain control with source-control duration");

        let triggered = def
            .abilities
            .iter()
            .find_map(|ability| match &ability.kind {
                AbilityKind::Triggered(triggered) => Some(triggered),
                _ => None,
            })
            .expect("expected triggered ability");
        let debug = format!("{:?}", triggered.effects);
        assert!(
            debug.contains("GainControlEffect"),
            "expected gain control effect, got {debug}"
        );
        assert!(
            debug.contains("YouStopControllingThis"),
            "expected source-control duration, got {debug}"
        );
    }

    #[test]
    fn parse_chaotic_transformation_followup_with_shared_type_filter() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Chaotic Transformation Variant")
            .parse_text(
                "Exile up to one target artifact, up to one target creature, up to one target enchantment, up to one target planeswalker, and/or up to one target land.\nFor each permanent exiled this way, its controller reveals cards from the top of their library until they reveal a card that shares a card type with it, puts that card onto the battlefield, then shuffles.",
            )
            .expect("parse chaotic transformation follow-up");

        let effects = def.spell_effect.expect("spell effects");
        let debug = format!("{effects:?}");
        assert!(
            debug.contains("SharesCardType"),
            "expected shared-card-type filter in compiled effects, got {debug}"
        );
        assert!(
            debug.contains("ForEachTaggedEffect"),
            "expected per-exiled-object iteration, got {debug}"
        );
        assert!(
            debug.contains("chooser: IteratedPlayer"),
            "expected each exiled permanent controller to choose from their library, got {debug}"
        );
        let choose_count = debug.matches("ChooseObjectsEffect").count();
        assert!(
            choose_count >= 6,
            "expected one choose per permanent type plus follow-up search chooser (at least 6 total), got {choose_count} in {debug}"
        );
        assert!(
            debug.contains("count: ChoiceCount { min: 0, max: Some(1)")
                && debug.contains("dynamic_x: false"),
            "expected up-to-one selection count for type picks, got {debug}"
        );
        assert!(
            debug.contains("tag: TagKey(\"exiled_0\")"),
            "expected shared exiled_0 tag for chained follow-up, got {debug}"
        );
    }

    #[test]
    fn parse_unless_any_player_pays_mana_prefix() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Rhystic Tutor Variant")
            .parse_text(
                "Unless any player pays {2}, search your library for a card, put that card into your hand, then shuffle.",
            )
            .expect("parse unless-any-player-pays prefix");

        let effects = def.spell_effect.expect("spell effects");
        let debug = format!("{effects:?}");
        assert!(
            debug.contains("UnlessPaysEffect"),
            "expected unless-pays wrapper in compiled effects, got {debug}"
        );
        assert!(
            debug.contains("player: Any"),
            "expected any-player payment choice, got {debug}"
        );
    }

    #[test]
    fn parse_construct_token_with_explicit_pt_does_not_force_karnstruct_stats() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Sokenzan Smelter Variant")
            .parse_text(
                "At the beginning of combat on your turn, you may pay {1} and sacrifice an artifact. If you do, create a 3/1 red Construct artifact creature token with haste.",
            )
            .expect("parse explicit-pt construct token");

        let rendered = compiled_lines(&def).join(" ").to_ascii_lowercase();
        assert!(
            rendered.contains("create a 3/1 red construct artifact creature token with haste"),
            "expected explicit 3/1 haste construct token text, got {rendered}"
        );
        assert!(
            !rendered.contains("power and toughness are each equal to the number of artifacts you control"),
            "explicit 3/1 construct token should not be forced into karnstruct stats, got {rendered}"
        );
    }

    #[test]
    fn parse_exile_up_to_one_single_disjunction_stays_single_choice() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Scrollshift Variant")
            .parse_text(
                "Exile up to one target artifact, creature, or enchantment you control, then return it to the battlefield under its owner's control.",
            )
            .expect("parse single-disjunction exile");

        let effects = def.spell_effect.expect("spell effects");
        let debug = format!("{effects:?}");
        let choose_count = debug.matches("ChooseObjectsEffect").count();
        assert!(
            choose_count <= 1,
            "single disjunctive target should not fan out into per-type choices, got {choose_count} in {debug}"
        );
        assert!(
            debug.contains("ExileEffect") && debug.contains("MoveToZoneEffect"),
            "expected exile-then-return sequence, got {debug}"
        );
        assert!(
            debug.contains("card_types: [Artifact, Creature, Enchantment]")
                || debug.contains("card_types: [Artifact, Enchantment, Creature]")
                || debug.contains("card_types: [Creature, Artifact, Enchantment]")
                || debug.contains("card_types: [Creature, Enchantment, Artifact]")
                || debug.contains("card_types: [Enchantment, Artifact, Creature]")
                || debug.contains("card_types: [Enchantment, Creature, Artifact]"),
            "expected combined disjunctive type filter, got {debug}"
        );
    }

    #[test]
    fn parse_target_player_choose_cards_then_put_on_top_of_library() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Stunted Growth Variant")
            .parse_text(
                "Target player chooses three cards from their hand and puts them on top of their library in any order.",
            )
            .expect("parse choose-and-put-on-top sequence");

        let effects = def.spell_effect.expect("spell effects");
        let debug = format!("{effects:?}");
        assert!(
            debug.contains("ChooseObjectsEffect"),
            "expected choose-objects step, got {debug}"
        );
        assert!(
            debug.contains("count: ChoiceCount { min: 3, max: Some(3)")
                && debug.contains("dynamic_x: false"),
            "expected exact three-card choice count, got {debug}"
        );
        assert!(
            debug.contains("zone: Hand"),
            "expected choose filter in hand, got {debug}"
        );
        assert!(
            debug.contains("MoveToZoneEffect")
                && debug.contains("to: Library")
                && debug.contains("to_top: true"),
            "expected move-to-library-top follow-up, got {debug}"
        );
    }

    #[test]
    fn parse_for_each_player_who_didnt_tracks_did_not_result() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Strongarm Tactics Variant")
            .parse_text(
                "Each player discards a card. Then each player who didn't discard a creature card this way loses 4 life.",
            )
            .expect("parse each-player-who-didnt branch");

        let effects = def.spell_effect.expect("spell effects");
        let debug = format!("{effects:?}");
        assert!(
            debug.contains("DidNotHappen"),
            "expected did-not branch keyed to prior discard result, got {debug}"
        );
        assert!(
            debug.contains("LoseLifeEffect"),
            "expected lose-life consequence branch, got {debug}"
        );
        assert!(
            !debug.contains("predicate: Happened"),
            "did-not branch should not collapse into generic happened check, got {debug}"
        );
    }

    #[test]
    fn parse_exile_target_player_hand_and_graveyard_bundle_sets_owner() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Identity Crisis Variant")
            .parse_text("Exile all cards from target player's hand and graveyard.")
            .expect("parse target hand+graveyard exile");

        let effects = def.spell_effect.expect("spell effects");
        let debug = format!("{effects:?}");
        assert!(
            debug.contains("zone: Some(Hand)") && debug.contains("zone: Some(Graveyard)"),
            "expected both hand and graveyard exile filters, got {debug}"
        );
        assert!(
            debug.matches("owner: Some(Target(Any))").count() >= 2,
            "expected both exile filters to track target player ownership, got {debug}"
        );
    }

    #[test]
    fn parse_self_enters_with_counters_as_static_not_spell_effect() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Self ETB Counter Variant")
            .parse_text("This creature enters with four +1/+1 counters on it.")
            .expect("parse self enters with counters");

        assert!(
            def.spell_effect.is_none(),
            "self ETB counters should not compile as spell effect"
        );

        let has_etb_replacement = def.abilities.iter().any(|ability| {
            matches!(
                &ability.kind,
                AbilityKind::Static(static_ability)
                    if static_ability.id() == crate::static_abilities::StaticAbilityId::EnterWithCounters
            )
        });
        assert!(
            has_etb_replacement,
            "expected self ETB replacement static ability, got {:?}",
            def.abilities
        );
    }

    #[test]
    fn parse_this_artifact_enters_with_counters_and_source_remove_cost() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Ox Cart Variant")
            .card_types(vec![CardType::Artifact])
            .parse_text(
                "This artifact enters with three charge counters on it.\n{1}, {T}, Remove a charge counter from this artifact: Destroy target creature.",
            )
            .expect("parse artifact enters counters plus source remove cost");

        assert!(
            def.spell_effect.is_none(),
            "artifact ETB counters should not compile as spell effect"
        );

        let has_etb_replacement = def.abilities.iter().any(|ability| {
            matches!(
                &ability.kind,
                AbilityKind::Static(static_ability)
                    if static_ability.id() == crate::static_abilities::StaticAbilityId::EnterWithCounters
            )
        });
        assert!(
            has_etb_replacement,
            "expected ETB counters static ability, got {:?}",
            def.abilities
        );

        let activated = def
            .abilities
            .iter()
            .find_map(|ability| match &ability.kind {
                AbilityKind::Activated(activated) => Some(activated),
                _ => None,
            })
            .expect("expected activated ability");
        let cost_debug = format!("{:?}", activated.mana_cost);
        assert!(
            cost_debug.contains("RemoveCountersCost"),
            "expected source-specific remove-counters cost, got {cost_debug}"
        );
        assert!(
            !cost_debug.contains("RemoveAnyCountersAmongCost"),
            "expected no distributed remove-counter fallback, got {cost_debug}"
        );
    }

    #[test]
    fn parse_return_two_target_cards_uses_exact_target_count() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Soul Strings Count Variant")
            .parse_text("Return two target creature cards from your graveyard to your hand.")
            .expect("parse exact-count return target");

        let effects = def.spell_effect.expect("spell effects");
        let debug = format!("{effects:?}");
        assert!(
            debug.contains("ChoiceCount { min: 2, max: Some(2)")
                && debug.contains("dynamic_x: false"),
            "expected exact two-target choice count, got {debug}"
        );
    }

    #[test]
    fn reject_unless_any_player_pays_clause() {
        let err = CardDefinitionBuilder::new(CardId::new(), "Soul Strings Unless Variant")
            .parse_text("Return two target creature cards from your graveyard to your hand unless any player pays {X}.")
            .expect_err("unless any player pays should fail until multi-payer semantics are supported");

        let message = format!("{err:?}");
        assert!(
            message.contains("unsupported return-unless clause"),
            "expected strict return-unless error, got {message}"
        );
    }

    #[test]
    fn reject_multi_target_return_clause() {
        let err = CardDefinitionBuilder::new(CardId::new(), "Peel Variant")
            .parse_text(
                "Return target creature you control and target creature you don't control to their owners' hands.",
            )
            .expect_err("multi-target return should fail until split-target semantics are supported");

        let message = format!("{err:?}");
        assert!(
            message.contains("unsupported multi-target return clause"),
            "expected strict multi-target return error, got {message}"
        );
    }

    #[test]
    fn parse_if_you_cant_as_if_result_did_not() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Ravenous Demon Predicate Variant")
            .parse_text(
                "At the beginning of your upkeep, sacrifice a Human. If you can't, tap this creature and it deals 9 damage to you.",
            )
            .expect("parse if-you-cant conditional");

        let triggered = def
            .abilities
            .iter()
            .find_map(|ability| match &ability.kind {
                AbilityKind::Triggered(triggered) => Some(triggered),
                _ => None,
            })
            .expect("expected triggered ability");
        let debug = format!("{:?}", triggered.effects);
        assert!(
            debug.contains("IfResultEffect") && debug.contains("DidNot"),
            "expected if-result DidNot conditional, got {debug}"
        );
    }

    #[test]
    fn reject_target_player_dealt_damage_by_this_turn_subject() {
        let err = CardDefinitionBuilder::new(CardId::new(), "Wicked Akuba Subject Variant")
            .parse_text("{B}: Target player dealt damage by this creature this turn loses 1 life.")
            .expect_err(
                "combat-history player subject should fail until per-source turn history is modeled",
            );

        let message = format!("{err:?}");
        assert!(
            message.contains("unsupported combat-history player subject"),
            "expected strict combat-history subject error, got {message}"
        );
    }

    #[test]
    fn parse_static_condition_equipped_creature_tapped_or_untapped() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Sword Condition Variant")
            .parse_text(
                "As long as equipped creature is tapped, tapped creatures you control get +2/+0.\nAs long as equipped creature is untapped, untapped creatures you control get +0/+2.",
            )
            .expect("parse equipped-creature tapped/untapped static conditions");

        let displays = def
            .abilities
            .iter()
            .filter_map(|ability| match &ability.kind {
                AbilityKind::Static(static_ability) => Some(static_ability.display()),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert!(
            displays
                .iter()
                .any(|display| display.contains("as long as equipped creature is tapped")),
            "missing tapped equipped-creature condition in displays: {displays:?}"
        );
        assert!(
            displays
                .iter()
                .any(|display| display.contains("as long as equipped creature is untapped")),
            "missing untapped equipped-creature condition in displays: {displays:?}"
        );
    }

    #[test]
    fn reject_counter_ability_target_clause() {
        let err = CardDefinitionBuilder::new(CardId::new(), "Tales End Variant")
            .parse_text("Counter target activated ability, triggered ability, or legendary spell.")
            .expect_err(
                "countering abilities should fail until ability-target semantics are implemented",
            );

        let message = format!("{err:?}");
        assert!(
            message.contains("unsupported counter-ability target clause"),
            "expected strict counter-ability target error, got {message}"
        );
    }

    #[test]
    fn reject_dynamic_create_for_each_clause() {
        let err = CardDefinitionBuilder::new(CardId::new(), "Mysterio Create Variant")
            .parse_text(
                "When this creature enters, create a 3/3 blue Illusion creature token for each nontoken Villain you control.",
            )
            .expect_err("dynamic create-for-each token count should fail until modeled");

        let message = format!("{err:?}");
        assert!(
            message.contains("unsupported dynamic token count"),
            "expected dynamic-token-count parse error, got {message}"
        );
    }

    #[test]
    fn reject_curly_apostrophe_negated_untap_clause() {
        let err = CardDefinitionBuilder::new(CardId::new(), "Kill Switch Apostrophe Variant")
            .parse_text(
                "{2}, {T}: Tap all other artifacts. They don’t untap during their controllers’ untap steps for as long as this artifact remains tapped.",
            )
            .expect_err("negated untap clause should fail strictly");

        let message = format!("{err:?}");
        assert!(
            message.contains("unsupported negated untap clause"),
            "expected strict negated-untap parse error, got {message}"
        );
    }

    #[test]
    fn create_creature_token_with_food_reminder_stays_creature_token() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Wolf Quarry Token Variant")
            .parse_text(
                "Create three 1/1 green Boar creature tokens with \"When this token dies, create a Food token.\"",
            )
            .expect("parse boar token creation with food reminder");

        let effects = def.spell_effect.expect("spell effects");
        let debug = format!("{effects:?}");
        assert!(
            debug.contains("Boar") && !debug.contains("name: \"Food\""),
            "expected creature token to remain Boar rather than Food, got {debug}"
        );
    }

    #[test]
    fn parse_for_each_player_put_from_graveyard_keeps_choice_non_targeted() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Exhume Variant")
            .parse_text(
                "Each player puts a creature card from their graveyard onto the battlefield.",
            )
            .expect("for-each player put-from-graveyard should parse");

        let joined = crate::compiled_text::compiled_lines(&def).join(" ");
        assert!(
            !joined.contains("target creature card in that player's graveyard"),
            "for-each choice should not become a target selection: {joined}"
        );
    }

    #[test]
    fn parse_for_each_player_may_put_from_hand_keeps_choice_non_targeted() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Show and Tell Variant")
            .parse_text(
                "Each player may put an artifact, creature, enchantment, or land card from their hand onto the battlefield.",
            )
            .expect("for-each player may-put-from-hand should parse");

        let joined = crate::compiled_text::compiled_lines(&def).join(" ");
        assert!(
            !joined.contains("target artifact or creature or enchantment or land card"),
            "for-each choice should not force target wording: {joined}"
        );
    }

    #[test]
    fn parse_you_sacrifice_trigger_clause_uses_sacrifice_trigger() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Crime Novelist Trigger Variant")
            .parse_text(
                "Whenever you sacrifice an artifact, put a +1/+1 counter on this creature and add {R}.",
            )
            .expect("sacrifice trigger clause should parse as a trigger");

        let joined = crate::compiled_text::compiled_lines(&def).join(" ");
        assert!(
            joined.contains("Whenever you sacrifice an artifact"),
            "expected sacrifice trigger wording, got {joined}"
        );
    }

    #[test]
    fn parse_unstable_experiment_draw_then_connive_preserves_draw() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Unstable Experiment Variant")
            .parse_text(
                "Target player draws a card, then up to one target creature you control connives. (Draw a card, then discard a card. If you discarded a nonland card, put a +1/+1 counter on that creature.)",
            )
            .expect("draw-then-connive sentence should parse");

        let effects = def.spell_effect.as_ref().expect("spell effects");
        assert!(
            effects
                .iter()
                .any(|effect| effect.downcast_ref::<DrawCardsEffect>().is_some()),
            "expected DrawCardsEffect, got {effects:?}"
        );
        assert!(
            effects
                .iter()
                .any(|effect| effect.downcast_ref::<ConniveEffect>().is_some()),
            "expected ConniveEffect, got {effects:?}"
        );
    }

    #[test]
    fn parse_grim_captains_call_then_do_same_for_subtypes_expands_each_return() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Grim Captain's Call Variant")
            .parse_text(
                "Return a Pirate card from your graveyard to your hand, then do the same for Vampire, Dinosaur, and Merfolk.",
            )
            .expect("then-do-the-same-for subtype sentence should parse");

        let lines = compiled_lines(&def);
        let spell_line = lines
            .iter()
            .find(|line| line.starts_with("Spell effects"))
            .expect("expected spell effects line");
        assert!(
            spell_line.contains("Pirate")
                && spell_line.contains("Vampire")
                && spell_line.contains("Dinosaur")
                && spell_line.contains("Merfolk"),
            "expected all subtype returns in compiled output, got {spell_line}"
        );
    }

    #[test]
    fn parse_each_player_return_with_additional_counter_appends_counter_effect() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Pyrrhic Revival Variant")
            .parse_text(
                "Each player returns each creature card from their graveyard to the battlefield with an additional -1/-1 counter on it.",
            )
            .expect("for-each return-with-additional-counter sentence should parse");

        let effects = def.spell_effect.as_ref().expect("spell effects");
        let for_players = effects
            .iter()
            .find_map(|effect| effect.downcast_ref::<ForPlayersEffect>())
            .expect("expected ForPlayersEffect");
        let debug = format!("{for_players:?}");
        assert!(
            debug.contains("ReturnAllToBattlefieldEffect")
                && debug.contains("PutCountersEffect")
                && debug.contains("MinusOneMinusOne"),
            "expected return + -1/-1 counter effects in for-players branch, got {debug}"
        );
    }

    #[test]
    fn parse_destroy_then_land_controller_graveyard_count_damage_clause() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Roiling Terrain Variant")
            .parse_text(
                "Destroy target land, then Roiling Terrain deals damage to that land's controller equal to the number of land cards in that player's graveyard.",
            )
            .expect("destroy-then-graveyard-count damage sentence should parse");

        let effects = def.spell_effect.as_ref().expect("spell effects");
        assert!(
            effects
                .iter()
                .any(|effect| effect.downcast_ref::<DestroyEffect>().is_some()),
            "expected destroy effect, got {effects:?}"
        );

        let deal_damage = effects
            .iter()
            .find_map(|effect| effect.downcast_ref::<crate::effects::DealDamageEffect>())
            .expect("expected deal-damage effect");
        match &deal_damage.amount {
            Value::Count(filter) => {
                assert_eq!(filter.zone, Some(Zone::Graveyard));
                assert!(
                    filter.card_types.contains(&CardType::Land),
                    "expected land-card count filter, got {filter:?}"
                );
                assert!(
                    matches!(filter.owner, Some(PlayerFilter::ControllerOf(_))),
                    "expected controller-owned graveyard count, got {filter:?}"
                );
            }
            other => panic!("expected count-based damage amount, got {other:?}"),
        }
    }

    #[test]
    fn parse_spin_into_myth_fateseal_appends_scry_effect() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Spin into Myth Variant")
            .parse_text(
                "Put target creature on top of its owner's library, then fateseal 2.",
            )
            .expect("fateseal tail should parse");

        let effects = def.spell_effect.as_ref().expect("spell effects");
        assert!(
            effects
                .iter()
                .any(|effect| effect.downcast_ref::<MoveToZoneEffect>().is_some()),
            "expected move-to-library effect, got {effects:?}"
        );
        let scry = effects
            .iter()
            .find_map(|effect| effect.downcast_ref::<ScryEffect>())
            .expect("expected scry effect for fateseal");
        assert_eq!(scry.player, PlayerFilter::Opponent);
        assert_eq!(scry.count, Value::Fixed(2));
    }

    #[test]
    fn parse_amass_clause_is_rejected_until_supported() {
        let err = CardDefinitionBuilder::new(CardId::new(), "Widespread Brutality Variant")
            .parse_text(
                "Amass Zombies 2, then the Army you amassed deals damage equal to its power to each non-Army creature.",
            )
            .expect_err("amass should fail until mechanic support is implemented");

        let message = format!("{err:?}");
        assert!(
            message.contains("unsupported amass mechanic"),
            "expected unsupported amass parse error, got {message}"
        );
    }

    #[test]
    fn parse_choose_from_graveyard_then_put_under_your_control() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Scrounge Variant")
            .parse_text(
                "Target opponent chooses an artifact card in their graveyard. Put that card onto the battlefield under your control.",
            )
            .expect("choose-from-graveyard then put-under-your-control should parse");

        let effects = def.spell_effect.as_ref().expect("spell effects");
        let choose = effects
            .iter()
            .find_map(|effect| effect.downcast_ref::<crate::effects::ChooseObjectsEffect>())
            .expect("expected choose-objects effect");
        assert_eq!(choose.filter.zone, Some(Zone::Graveyard));
        assert_eq!(choose.filter.owner, Some(PlayerFilter::IteratedPlayer));
        assert_eq!(
            choose.filter.controller, None,
            "graveyard selection should rely on owner context, got {choose:?}"
        );

        let move_to = effects
            .iter()
            .find_map(|effect| effect.downcast_ref::<MoveToZoneEffect>())
            .expect("expected move-to-zone follow-up");
        assert_eq!(move_to.zone, Zone::Battlefield);
        assert_eq!(move_to.battlefield_controller, BattlefieldController::You);
    }
}

#[cfg(all(test, feature = "parser-tests"))]
mod tests;
