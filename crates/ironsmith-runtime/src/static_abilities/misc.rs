//! Miscellaneous static abilities.
//!
//! This module contains static abilities that don't fit neatly into other categories.

use super::{
    ChooseBasicLandTypeAsEntersSpec, ChooseCardNameAsEntersSpec, ChooseColorAsBecomesAttachedSpec,
    ChooseColorAsEntersSpec, ChooseCreatureTypeAsEntersSpec, ChooseLandTypeAsEntersSpec,
    ChooseNamedOptionAsEntersSpec, ChoosePlayerAsEntersSpec,
    ChoosePowerToughnessAsEntersOrTurnsFaceUpSpec, ConditionalSpellKeywordKind,
    ConditionalSpellKeywordSpec, CountAsCardNamedForSpellEffectSpec, DieRollResultAdjustmentSpec,
    EnterAsCopyAsEntersSpec, GraveyardCountMetric, NoteLifeTotalAsEntersSpec,
    PowerToughnessChoiceOption, RevealFromHandAsEntersSpec, StaticAbility, StaticAbilityId,
    StaticAbilityKind, ThisSpellCastRestrictionKind, TriggerDuplicationSourceMatcher,
    TriggerDuplicationSpec, TriggerSuppressionSpec,
    text_utils::{capitalize_first, join_with_and, number_word_u32},
};
use crate::ability::{Ability, AbilityKind, LevelAbility};
use crate::color::Color;
use crate::compiled_text::describe_value;
use crate::effect::{Condition, Effect, EventValueSpec, Value};
use crate::events::cards::DiscardEvent;
use crate::events::cards::matchers::{
    WouldDiscardMatcher, WouldDrawCardMatcher, WouldDrawCardWhileLibraryEmptyMatcher,
};
use crate::events::cause::CauseType;
use crate::events::context::EventContext;
use crate::events::damage::DamageEvent;
use crate::events::damage::matchers::{
    DamageFromSelfCombatMatcher, DamageFromSelfMatcher, DamageFromSourceToObjectMatcher,
    DamageFromSourceToPlayerMatcher, DamageToObjectMatcher, DamageToOtherCreatureYouControlMatcher,
    DamageToPlayerOrObjectMatcher, DamageToSelfCombatMatcher, DamageToSelfConstraintMatcher,
    DamageToSelfFromSourceFilterMatcher, PreventableCombatDamageToObjectMatcher,
    PreventableNoncombatDamageToObjectMatcher,
};
use crate::events::permanents::matchers::AttachedPermanentWouldBeDestroyedMatcher;
use crate::events::traits::{
    EventKind, GameEventType, ReplacementMatcher, ReplacementPriority, downcast_event,
};
use crate::events::zones::matchers::{
    ThisWouldEnterBattlefieldMatcher, WouldDieDamagedBySourceThisTurnMatcher,
    WouldEnterBattlefieldMatcher,
};
use crate::events::zones::{EnterBattlefieldEvent, ZoneChangeEvent};
use crate::filter::{ObjectFilterExt as _, PlayerFilterExt as _};
use crate::game_state::GameState;
use crate::grant::GrantSpec;
use crate::ids::{ObjectId, PlayerId};
use crate::object::CounterType;
use crate::replacement::{
    EventModification, RedirectTarget, RedirectWhich, ReplacementAction, ReplacementEffect,
    ZoneReplacementSpec,
};
use crate::target::{
    ChooseSpec, ObjectFilter, PlayerFilter, TaggedObjectConstraint, TaggedOpbjectRelation,
};
use crate::types::{CardType, Subtype};
use crate::zone::Zone;
use ironsmith_core::{DamagedBySource, TagKey, ValueSurfaceHint};

mod replacements_and_rules;
pub use replacements_and_rules::*;

#[cfg(test)]
mod tests;

fn card_type_word(card_type: crate::types::CardType) -> &'static str {
    card_type.name()
}

fn pluralize(word: &str) -> String {
    if word.ends_with('s') {
        word.to_string()
    } else {
        format!("{word}s")
    }
}

fn pluralize_filter_description(description: &str) -> String {
    let base = description
        .trim()
        .strip_prefix("a ")
        .or_else(|| description.trim().strip_prefix("an "))
        .unwrap_or_else(|| description.trim());
    for suffix in [
        " you control",
        " you own",
        " an opponent controls",
        " an opponent owns",
        " that player controls",
        " that player owns",
    ] {
        if let Some(head) = base.strip_suffix(suffix) {
            return format!("{}{}", pluralize(head.trim_end()), suffix);
        }
    }
    pluralize(base)
}

fn indefinite_article(text: &str) -> &'static str {
    let first = text
        .chars()
        .find(|ch| ch.is_ascii_alphabetic())
        .map(|ch| ch.to_ascii_lowercase());
    match first {
        Some('a' | 'e' | 'i' | 'o' | 'u') => "an",
        _ => "a",
    }
}

fn enters_with_counters_where_x_value(count: &Value) -> Option<String> {
    let unhinted = count.unhinted();
    if matches!(
        unhinted,
        Value::Fixed(_)
            | Value::X
            | Value::ManaSpentToCastThisSpell
            | Value::ColorsOfManaSpentToCastThisSpell
    ) {
        return None;
    }

    let prefers_where_x = count.has_surface_hint(ValueSurfaceHint::WhereXIs)
        || matches!(
            unhinted,
            Value::TotalPower(_)
                | Value::TotalToughness(_)
                | Value::TotalManaValue(_)
                | Value::GreatestPower(_)
                | Value::GreatestToughness(_)
                | Value::GreatestManaValue(_)
        );
    prefers_where_x.then(|| describe_value(count))
}

fn describe_enters_with_counters_equal_to_value(count: &Value) -> String {
    let value = describe_value(count);
    if let Some(rest) = value.strip_prefix("1 plus ") {
        format!("one plus {rest}")
    } else {
        value
    }
}

fn is_revealed_this_way_count_filter(filter: &ObjectFilter) -> bool {
    let mut bare = filter.clone();
    bare.tagged_constraints.clear();
    bare == ObjectFilter::default()
        && filter.tagged_constraints.len() == 1
        && filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag.as_str() == "__public_revealed"
                && matches!(constraint.relation, TaggedOpbjectRelation::IsTaggedObject)
        })
}

fn describe_enters_with_counters_count_filter(filter: &ObjectFilter) -> String {
    if is_revealed_this_way_count_filter(filter) {
        "cards revealed this way".to_string()
    } else {
        filter.description()
    }
}

fn counter_indefinite_article(counter: &str) -> &'static str {
    match counter.chars().next().map(|ch| ch.to_ascii_lowercase()) {
        Some('a' | 'e' | 'i' | 'o' | 'u') => "an",
        _ => "a",
    }
}

fn describe_discard_filter_card_phrase(filter: &ObjectFilter) -> String {
    let mut phrase = filter.description().trim().to_string();
    if phrase.is_empty() {
        return "a card".to_string();
    }

    let lower = phrase.to_ascii_lowercase();
    let has_determiner = lower.starts_with("a ")
        || lower.starts_with("an ")
        || lower.starts_with("the ")
        || lower.starts_with("target ")
        || lower.starts_with("another ")
        || lower.starts_with("any ")
        || lower.starts_with("each ");
    if !has_determiner {
        phrase = format!("{} {}", indefinite_article(&phrase), phrase);
    }

    let lower = phrase.to_ascii_lowercase();
    if !lower.contains(" card") && !lower.ends_with("card") {
        phrase.push_str(" card");
    }
    phrase
}

fn describe_redirect_zone_phrase(zone: Zone) -> &'static str {
    match zone {
        Zone::Graveyard => "its owner's graveyard",
        Zone::Hand => "its owner's hand",
        Zone::Library => "its owner's library",
        Zone::Battlefield => "the battlefield",
        Zone::Stack => "the stack",
        Zone::Exile => "exile",
        Zone::Command => "the command zone",
        Zone::OutsideGame => "outside the game",
    }
}

/// Daybound keyword static ability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Daybound;

#[derive(Debug, Clone, PartialEq)]
pub struct DieRollResultAdjustment {
    player: PlayerFilter,
    life_cost: u32,
    amount: u32,
    once_each_turn: bool,
    display: String,
}

impl DieRollResultAdjustment {
    pub fn new(
        player: PlayerFilter,
        life_cost: u32,
        amount: u32,
        once_each_turn: bool,
        display: impl Into<String>,
    ) -> Self {
        Self {
            player,
            life_cost,
            amount,
            once_each_turn,
            display: display.into(),
        }
    }
}

impl StaticAbilityKind for DieRollResultAdjustment {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::DieRollResultAdjustment
    }

    fn display(&self) -> String {
        self.display.clone()
    }

    fn die_roll_result_adjustment_spec(&self) -> Option<DieRollResultAdjustmentSpec> {
        Some(DieRollResultAdjustmentSpec {
            player: self.player.clone(),
            life_cost: self.life_cost,
            amount: self.amount,
            once_each_turn: self.once_each_turn,
        })
    }
}

impl StaticAbilityKind for Daybound {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::Daybound
    }

    fn display(&self) -> String {
        "Daybound".to_string()
    }

    fn is_keyword(&self) -> bool {
        true
    }
}

/// Nightbound keyword static ability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Nightbound;

impl StaticAbilityKind for Nightbound {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::Nightbound
    }

    fn display(&self) -> String {
        "Nightbound".to_string()
    }

    fn is_keyword(&self) -> bool {
        true
    }
}

/// Starts the day/night designation as day if it is unset as this enters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DayNightStartsDayAsEnters;

impl StaticAbilityKind for DayNightStartsDayAsEnters {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::DayNightStartsDayAsEnters
    }

    fn display(&self) -> String {
        "If it's neither day nor night, it becomes day as this creature enters".to_string()
    }
}

/// Morph keyword ability (turn face up by paying morph cost as a special action).
#[derive(Debug, Clone, PartialEq)]
pub struct Morph {
    pub cost: crate::cost::TotalCost,
}

impl Morph {
    pub fn new(cost: crate::cost::TotalCost) -> Self {
        Self { cost }
    }
}

impl StaticAbilityKind for Morph {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::Morph
    }

    fn display(&self) -> String {
        format!("Morph {}", self.cost.display())
    }

    fn is_keyword(&self) -> bool {
        true
    }

    fn turn_face_up_cost(&self) -> Option<&crate::cost::TotalCost> {
        Some(&self.cost)
    }
}

/// Disguise keyword ability (turn face up by paying disguise cost as a special action).
#[derive(Debug, Clone, PartialEq)]
pub struct Disguise {
    pub cost: crate::cost::TotalCost,
}

impl Disguise {
    pub fn new(cost: crate::cost::TotalCost) -> Self {
        Self { cost }
    }
}

impl StaticAbilityKind for Disguise {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::Disguise
    }

    fn display(&self) -> String {
        format!("Disguise {}", self.cost.display())
    }

    fn is_keyword(&self) -> bool {
        true
    }

    fn turn_face_up_cost(&self) -> Option<&crate::cost::TotalCost> {
        Some(&self.cost)
    }

    fn is_disguise(&self) -> bool {
        true
    }
}

/// Megamorph keyword ability (turn face up by paying megamorph cost as a special action).
#[derive(Debug, Clone, PartialEq)]
pub struct Megamorph {
    pub cost: crate::cost::TotalCost,
}

impl Megamorph {
    pub fn new(cost: crate::cost::TotalCost) -> Self {
        Self { cost }
    }
}

impl StaticAbilityKind for Megamorph {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::Megamorph
    }

    fn display(&self) -> String {
        format!("Megamorph {}", self.cost.display())
    }

    fn is_keyword(&self) -> bool {
        true
    }

    fn turn_face_up_cost(&self) -> Option<&crate::cost::TotalCost> {
        Some(&self.cost)
    }

    fn is_megamorph(&self) -> bool {
        true
    }
}

/// Doesn't untap during your untap step.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DoesntUntap;

impl StaticAbilityKind for DoesntUntap {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::DoesntUntap
    }

    fn display(&self) -> String {
        "Doesn't untap during your untap step".to_string()
    }

    fn affects_untap(&self) -> bool {
        true
    }
}

/// "Untap all [matching permanents] during each other player's untap step."
#[derive(Debug, Clone, PartialEq)]
pub struct UntapDuringEachOtherPlayersUntapStep {
    pub filter: ObjectFilter,
    pub display: String,
}

impl UntapDuringEachOtherPlayersUntapStep {
    pub fn new(filter: ObjectFilter, display: String) -> Self {
        Self { filter, display }
    }
}

impl StaticAbilityKind for UntapDuringEachOtherPlayersUntapStep {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::UntapDuringEachOtherPlayersUntapStep
    }

    fn display(&self) -> String {
        self.display.clone()
    }

    fn untap_during_each_other_players_untap_step_filter(&self) -> Option<&ObjectFilter> {
        Some(&self.filter)
    }
}

/// "Creatures you control can boast twice during each of your turns rather than once."
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BoastTwiceEachTurn;

impl StaticAbilityKind for BoastTwiceEachTurn {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::BoastTwiceEachTurn
    }

    fn display(&self) -> String {
        "Creatures you control can boast twice during each of your turns rather than once"
            .to_string()
    }
}

/// "You may pay {0} rather than pay the equip cost of the first equip ability
/// you activate each turn." (Bruenor Battlehammer, Forge Anew, etc.)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FirstEquipCostAlternative {
    pub display_text: String,
}

impl FirstEquipCostAlternative {
    pub fn new(display_text: impl Into<String>) -> Self {
        Self {
            display_text: display_text.into(),
        }
    }
}

impl StaticAbilityKind for FirstEquipCostAlternative {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::FirstEquipCostAlternative
    }

    fn display(&self) -> String {
        self.display_text.clone()
    }
}

/// "You may activate equip abilities any time you could cast an instant."
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct EquipAbilitiesAnyTime;

impl StaticAbilityKind for EquipAbilitiesAnyTime {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::EquipAbilitiesAnyTime
    }

    fn display(&self) -> String {
        "You may activate equip abilities any time you could cast an instant".to_string()
    }
}

/// "During your turn, as long as you haven't activated an exhaust ability this turn,
/// you may activate exhaust abilities as though they haven't been activated."
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ExhaustAbilitiesAsThoughUnactivatedThisTurn;

impl StaticAbilityKind for ExhaustAbilitiesAsThoughUnactivatedThisTurn {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::ExhaustAbilitiesAsThoughUnactivatedThisTurn
    }

    fn display(&self) -> String {
        "During your turn, as long as you haven't activated an exhaust ability this turn, you may activate exhaust abilities as though they haven't been activated".to_string()
    }
}

/// "While voting, you may vote an additional time."
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct VoteAdditionalTimeWhileVoting;

impl StaticAbilityKind for VoteAdditionalTimeWhileVoting {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::VoteAdditionalTimeWhileVoting
    }

    fn display(&self) -> String {
        "While voting, you may vote an additional time.".to_string()
    }

    fn optional_additional_votes_while_voting(&self) -> u32 {
        1
    }
}

/// "While voting, you get an additional vote."
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct VoteAdditionalVoteWhileVoting;

impl StaticAbilityKind for VoteAdditionalVoteWhileVoting {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::VoteAdditionalVoteWhileVoting
    }

    fn display(&self) -> String {
        "While voting, you get an additional vote.".to_string()
    }

    fn additional_votes_while_voting(&self) -> u32 {
        1
    }
}

/// "Reveal the first card you draw each turn/on each of your turns."
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RevealFirstCardYouDrawEachTurn {
    pub optional: bool,
    pub your_turns_only: bool,
}

impl RevealFirstCardYouDrawEachTurn {
    pub fn new(optional: bool, your_turns_only: bool) -> Self {
        Self {
            optional,
            your_turns_only,
        }
    }
}

impl StaticAbilityKind for RevealFirstCardYouDrawEachTurn {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::RevealFirstCardYouDrawEachTurn
    }

    fn display(&self) -> String {
        match (self.optional, self.your_turns_only) {
            (false, false) => "Reveal the first card you draw each turn.".to_string(),
            (false, true) => "Reveal the first card you draw on each of your turns.".to_string(),
            (true, false) => {
                "You may reveal the first card you draw each turn as you draw it.".to_string()
            }
            (true, true) => {
                "You may reveal the first card you draw on each of your turns as you draw it."
                    .to_string()
            }
        }
    }

    fn reveal_drawn_card_spec(&self) -> Option<crate::static_abilities::RevealDrawnCardSpec> {
        Some(crate::static_abilities::RevealDrawnCardSpec {
            card_number: 1,
            optional: self.optional,
            your_turns_only: self.your_turns_only,
        })
    }
}

/// "Effects from spells named N count this as a card named M."
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CountAsCardNamedForSpellEffect {
    pub spell_name: String,
    pub counted_name: String,
}

impl CountAsCardNamedForSpellEffect {
    pub fn new(spell_name: String, counted_name: String) -> Self {
        Self {
            spell_name,
            counted_name,
        }
    }
}

impl StaticAbilityKind for CountAsCardNamedForSpellEffect {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::CountAsCardNamedForSpellEffect
    }

    fn display(&self) -> String {
        format!(
            "If this card is in a graveyard, effects from spells named {} count it as a card named {}.",
            self.spell_name, self.counted_name
        )
    }

    fn count_as_card_named_for_spell_effect_spec(
        &self,
    ) -> Option<CountAsCardNamedForSpellEffectSpec> {
        Some(CountAsCardNamedForSpellEffectSpec {
            spell_name: self.spell_name.clone(),
            counted_name: self.counted_name.clone(),
        })
    }
}

/// "You may choose not to untap ... during your untap step."
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MayChooseNotToUntapDuringUntapStep {
    pub subject: String,
}

impl MayChooseNotToUntapDuringUntapStep {
    pub fn new(subject: impl Into<String>) -> Self {
        Self {
            subject: subject.into(),
        }
    }
}

impl StaticAbilityKind for MayChooseNotToUntapDuringUntapStep {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::MayChooseNotToUntapDuringUntapStep
    }

    fn display(&self) -> String {
        format!(
            "You may choose not to untap {} during your untap step",
            self.subject
        )
    }
}

/// Enters the battlefield tapped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct EntersTapped;

impl StaticAbilityKind for EntersTapped {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::EntersTapped
    }

    fn display(&self) -> String {
        "This enters tapped".to_string()
    }

    fn enters_tapped(&self) -> bool {
        true
    }

    fn generate_replacement_effect(
        &self,
        source: ObjectId,
        controller: PlayerId,
    ) -> Option<ReplacementEffect> {
        Some(ReplacementEffect::with_matcher(
            source,
            controller,
            ThisWouldEnterBattlefieldMatcher,
            ReplacementAction::EnterTapped,
        ))
    }
}

/// "This enters the battlefield tapped unless you control two or more other lands."
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct EntersTappedUnlessControlTwoOrMoreOtherLands;

impl StaticAbilityKind for EntersTappedUnlessControlTwoOrMoreOtherLands {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::EntersTappedUnlessControlTwoOrMoreOtherLands
    }

    fn display(&self) -> String {
        "This enters the battlefield tapped unless you control two or more other lands".to_string()
    }

    fn generate_replacement_effect(
        &self,
        source: ObjectId,
        controller: PlayerId,
    ) -> Option<ReplacementEffect> {
        Some(ReplacementEffect::with_matcher(
            source,
            controller,
            ThisWouldEnterTappedUnlessControlTwoOrMoreOtherLandsMatcher,
            ReplacementAction::EnterTapped,
        ))
    }
}

/// "This enters the battlefield tapped unless you control two or fewer other lands."
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct EntersTappedUnlessControlTwoOrFewerOtherLands;

impl StaticAbilityKind for EntersTappedUnlessControlTwoOrFewerOtherLands {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::EntersTappedUnlessControlTwoOrFewerOtherLands
    }

    fn display(&self) -> String {
        "This enters the battlefield tapped unless you control two or fewer other lands".to_string()
    }

    fn generate_replacement_effect(
        &self,
        source: ObjectId,
        controller: PlayerId,
    ) -> Option<ReplacementEffect> {
        Some(ReplacementEffect::with_matcher(
            source,
            controller,
            ThisWouldEnterTappedUnlessControlTwoOrFewerOtherLandsMatcher,
            ReplacementAction::EnterTapped,
        ))
    }
}

/// "This enters the battlefield tapped unless you control two or more basic lands."
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct EntersTappedUnlessControlTwoOrMoreBasicLands;

impl StaticAbilityKind for EntersTappedUnlessControlTwoOrMoreBasicLands {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::EntersTappedUnlessControlTwoOrMoreBasicLands
    }

    fn display(&self) -> String {
        "This enters the battlefield tapped unless you control two or more basic lands".to_string()
    }

    fn generate_replacement_effect(
        &self,
        source: ObjectId,
        controller: PlayerId,
    ) -> Option<ReplacementEffect> {
        Some(ReplacementEffect::with_matcher(
            source,
            controller,
            ThisWouldEnterTappedUnlessControlTwoOrMoreBasicLandsMatcher,
            ReplacementAction::EnterTapped,
        ))
    }
}

/// "This enters the battlefield tapped unless a player has 13 or less life."
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct EntersTappedUnlessAPlayerHas13OrLessLife;

impl StaticAbilityKind for EntersTappedUnlessAPlayerHas13OrLessLife {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::EntersTappedUnlessAPlayerHas13OrLessLife
    }

    fn display(&self) -> String {
        "This enters the battlefield tapped unless a player has 13 or less life".to_string()
    }

    fn generate_replacement_effect(
        &self,
        source: ObjectId,
        controller: PlayerId,
    ) -> Option<ReplacementEffect> {
        Some(ReplacementEffect::with_matcher(
            source,
            controller,
            ThisWouldEnterTappedUnlessAPlayerHas13OrLessLifeMatcher,
            ReplacementAction::EnterTapped,
        ))
    }
}

/// "This enters tapped unless you have two or more opponents."
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct EntersTappedUnlessTwoOrMoreOpponents;

impl StaticAbilityKind for EntersTappedUnlessTwoOrMoreOpponents {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::EntersTappedUnlessTwoOrMoreOpponents
    }

    fn display(&self) -> String {
        "This enters the battlefield tapped unless you have two or more opponents".to_string()
    }

    fn generate_replacement_effect(
        &self,
        source: ObjectId,
        controller: PlayerId,
    ) -> Option<ReplacementEffect> {
        Some(ReplacementEffect::with_matcher(
            source,
            controller,
            ThisWouldEnterTappedUnlessTwoOrMoreOpponentsMatcher,
            ReplacementAction::EnterTapped,
        ))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct ThisWouldEnterTappedUnlessControlTwoOrMoreOtherLandsMatcher;

impl ReplacementMatcher for ThisWouldEnterTappedUnlessControlTwoOrMoreOtherLandsMatcher {
    fn matches_event(
        &self,
        event: &dyn crate::events::traits::GameEventType,
        ctx: &crate::events::EventContext,
    ) -> bool {
        if !matches_this_would_enter_battlefield(event, ctx) {
            return false;
        }

        let land_count = ctx
            .game
            .battlefield
            .iter()
            .filter_map(|&id| ctx.game.object(id))
            .filter(|obj| ctx.game.controller_of(obj) == ctx.controller && obj.is_land())
            .count();
        land_count < 2
    }

    fn priority(&self) -> ReplacementPriority {
        ReplacementPriority::Other
    }

    fn display(&self) -> String {
        "When this would enter tapped unless you control two or more other lands".to_string()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct ThisWouldEnterTappedUnlessControlTwoOrFewerOtherLandsMatcher;

impl ReplacementMatcher for ThisWouldEnterTappedUnlessControlTwoOrFewerOtherLandsMatcher {
    fn matches_event(
        &self,
        event: &dyn crate::events::traits::GameEventType,
        ctx: &crate::events::EventContext,
    ) -> bool {
        if !matches_this_would_enter_battlefield(event, ctx) {
            return false;
        }

        let land_count = ctx
            .game
            .battlefield
            .iter()
            .filter_map(|&id| ctx.game.object(id))
            .filter(|obj| ctx.game.controller_of(obj) == ctx.controller && obj.is_land())
            .count();
        land_count > 2
    }

    fn priority(&self) -> ReplacementPriority {
        ReplacementPriority::Other
    }

    fn display(&self) -> String {
        "When this would enter tapped unless you control two or fewer other lands".to_string()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct ThisWouldEnterTappedUnlessControlTwoOrMoreBasicLandsMatcher;

impl ReplacementMatcher for ThisWouldEnterTappedUnlessControlTwoOrMoreBasicLandsMatcher {
    fn matches_event(
        &self,
        event: &dyn crate::events::traits::GameEventType,
        ctx: &crate::events::EventContext,
    ) -> bool {
        if !matches_this_would_enter_battlefield(event, ctx) {
            return false;
        }

        let basic_land_count = ctx
            .game
            .battlefield
            .iter()
            .filter_map(|&id| ctx.game.object(id))
            .filter(|obj| {
                ctx.game.controller_of(obj) == ctx.controller
                    && obj.is_land()
                    && obj.supertypes.contains(&crate::types::Supertype::Basic)
            })
            .count();
        basic_land_count < 2
    }

    fn priority(&self) -> ReplacementPriority {
        ReplacementPriority::Other
    }

    fn display(&self) -> String {
        "When this would enter tapped unless you control two or more basic lands".to_string()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct ThisWouldEnterTappedUnlessAPlayerHas13OrLessLifeMatcher;

impl ReplacementMatcher for ThisWouldEnterTappedUnlessAPlayerHas13OrLessLifeMatcher {
    fn matches_event(
        &self,
        event: &dyn crate::events::traits::GameEventType,
        ctx: &crate::events::EventContext,
    ) -> bool {
        if !matches_this_would_enter_battlefield(event, ctx) {
            return false;
        }

        !ctx.game
            .players
            .iter()
            .any(|player| player.is_in_game() && player.life <= 13)
    }

    fn priority(&self) -> ReplacementPriority {
        ReplacementPriority::Other
    }

    fn display(&self) -> String {
        "When this would enter tapped unless a player has 13 or less life".to_string()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct ThisWouldEnterTappedUnlessTwoOrMoreOpponentsMatcher;

impl ReplacementMatcher for ThisWouldEnterTappedUnlessTwoOrMoreOpponentsMatcher {
    fn matches_event(
        &self,
        event: &dyn crate::events::traits::GameEventType,
        ctx: &crate::events::EventContext,
    ) -> bool {
        if !matches_this_would_enter_battlefield(event, ctx) {
            return false;
        }

        let opponents = ctx
            .game
            .players
            .iter()
            .filter(|player| player.is_in_game() && player.id != ctx.controller)
            .count();
        opponents < 2
    }

    fn priority(&self) -> ReplacementPriority {
        ReplacementPriority::Other
    }

    fn display(&self) -> String {
        "When this would enter tapped unless you have two or more opponents".to_string()
    }
}

/// "This enters the battlefield tapped unless <condition>."
///
/// This is a generic ETB replacement that evaluates a `Condition` at the moment
/// the object would enter the battlefield.
#[derive(Debug, Clone, PartialEq)]
pub struct EntersTappedUnlessCondition {
    pub condition: Condition,
    pub display: String,
}

impl EntersTappedUnlessCondition {
    pub fn new(condition: Condition, display: String) -> Self {
        Self { condition, display }
    }
}

impl StaticAbilityKind for EntersTappedUnlessCondition {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::EntersTappedUnlessCondition
    }

    fn display(&self) -> String {
        self.display.clone()
    }

    fn generate_replacement_effect(
        &self,
        source: ObjectId,
        controller: PlayerId,
    ) -> Option<ReplacementEffect> {
        Some(ReplacementEffect::with_matcher(
            source,
            controller,
            ThisWouldEnterTappedUnlessConditionMatcher {
                condition: self.condition.clone(),
                display: self.display.clone(),
            },
            ReplacementAction::EnterTapped,
        ))
    }

    fn enters_tapped(&self) -> bool {
        // Conditionally enters tapped; replacement determines final state.
        false
    }
}

#[derive(Debug, Clone, PartialEq)]
struct ThisWouldEnterTappedUnlessConditionMatcher {
    condition: Condition,
    display: String,
}

impl ReplacementMatcher for ThisWouldEnterTappedUnlessConditionMatcher {
    fn matches_event(
        &self,
        event: &dyn crate::events::traits::GameEventType,
        ctx: &crate::events::EventContext,
    ) -> bool {
        if !matches_this_would_enter_battlefield(event, ctx) {
            return false;
        }

        let Some(source) = ctx.source else {
            return false;
        };
        let eval_ctx = crate::condition_eval::ExternalEvaluationContext {
            controller: ctx.controller,
            source,
            defending_player: None,
            attacking_player: None,
            filter_source: None,
            iterated_player: None,
            triggering_event: None,
            trigger_identity: None,
            ability_index: None,
            options: Default::default(),
        };

        // Replacement applies when the "unless" condition is false.
        !crate::condition_eval::evaluate_condition_external(ctx.game, &self.condition, &eval_ctx)
    }

    fn priority(&self) -> ReplacementPriority {
        ReplacementPriority::Other
    }

    fn display(&self) -> String {
        format!("When this would enter tapped unless {}", self.display)
    }
}

fn matches_this_would_enter_battlefield(
    event: &dyn crate::events::traits::GameEventType,
    ctx: &crate::events::EventContext,
) -> bool {
    let object_id = match event.event_kind() {
        EventKind::ZoneChange => {
            let Some(zone_change) = downcast_event::<ZoneChangeEvent>(event) else {
                return false;
            };
            if zone_change.to != Zone::Battlefield {
                return false;
            }
            let Some(&object_id) = zone_change.objects.first() else {
                return false;
            };
            object_id
        }
        EventKind::EnterBattlefield => {
            let Some(etb) = downcast_event::<EnterBattlefieldEvent>(event) else {
                return false;
            };
            etb.object
        }
        _ => return false,
    };

    ctx.source == Some(object_id)
}

/// Bloodthirst N.
///
/// If an opponent was dealt damage this turn, this creature enters
/// the battlefield with N +1/+1 counters on it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Bloodthirst {
    pub amount: u32,
}

impl Bloodthirst {
    pub const fn new(amount: u32) -> Self {
        Self { amount }
    }
}

impl StaticAbilityKind for Bloodthirst {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::Bloodthirst
    }

    fn display(&self) -> String {
        format!("Bloodthirst {}", self.amount)
    }

    fn is_keyword(&self) -> bool {
        true
    }

    fn generate_replacement_effect(
        &self,
        source: ObjectId,
        controller: PlayerId,
    ) -> Option<ReplacementEffect> {
        Some(ReplacementEffect::with_matcher(
            source,
            controller,
            ThisWouldEnterWithBloodthirstMatcher,
            ReplacementAction::EnterWithCounters {
                counter_type: CounterType::PlusOnePlusOne,
                count: Value::Fixed(self.amount as i32),
                added_subtypes: Vec::new(),
                added_abilities: Vec::new(),
            },
        ))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct ThisWouldEnterWithBloodthirstMatcher;

impl ReplacementMatcher for ThisWouldEnterWithBloodthirstMatcher {
    fn matches_event(
        &self,
        event: &dyn crate::events::traits::GameEventType,
        ctx: &crate::events::EventContext,
    ) -> bool {
        if !matches_this_would_enter_battlefield(event, ctx) {
            return false;
        }

        ctx.game.players.iter().any(|player| {
            player.is_in_game()
                && player.id != ctx.controller
                && ctx
                    .game
                    .turn_store
                    .turn_history
                    .player_was_dealt_damage_this_turn(player.id)
        })
    }

    fn priority(&self) -> ReplacementPriority {
        ReplacementPriority::Other
    }

    fn display(&self) -> String {
        "When this would enter with bloodthirst counters".to_string()
    }
}

/// Tribute N.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tribute {
    pub amount: u32,
}

impl Tribute {
    pub const fn new(amount: u32) -> Self {
        Self { amount }
    }
}

impl StaticAbilityKind for Tribute {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::Tribute
    }

    fn display(&self) -> String {
        format!("Tribute {}", self.amount)
    }

    fn is_keyword(&self) -> bool {
        true
    }

    fn generate_replacement_effect(
        &self,
        source: ObjectId,
        controller: PlayerId,
    ) -> Option<ReplacementEffect> {
        Some(ReplacementEffect::with_matcher(
            source,
            controller,
            ThisWouldEnterBattlefieldMatcher,
            ReplacementAction::Tribute {
                counter_type: CounterType::PlusOnePlusOne,
                count: self.amount,
                paid_label: "Tribute".to_string(),
            },
        ))
    }
}

/// Enters the battlefield with counters.
#[derive(Debug, Clone, PartialEq)]
pub struct EntersWithCounters {
    pub counter_type: CounterType,
    pub count: Value,
}

impl EntersWithCounters {
    pub fn new(counter_type: CounterType, count: Value) -> Self {
        Self {
            counter_type,
            count,
        }
    }
}

impl StaticAbilityKind for EntersWithCounters {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::EnterWithCounters
    }

    fn display(&self) -> String {
        let counter = self.counter_type.description().into_owned();
        if let Some(where_x_value) = enters_with_counters_where_x_value(&self.count) {
            return format!(
                "Enters the battlefield with X {counter} counters on it, where X is {where_x_value}"
            );
        }
        if self.count.has_surface_hint(ValueSurfaceHint::EqualTo) {
            return format!(
                "Enters the battlefield with a number of {counter} counters on it equal to {}",
                describe_enters_with_counters_equal_to_value(&self.count)
            );
        }

        match &self.count {
            Value::Fixed(v) => {
                if *v == 1 {
                    let article = match counter.chars().next().map(|ch| ch.to_ascii_lowercase()) {
                        Some('a' | 'e' | 'i' | 'o' | 'u') => "an",
                        _ => "a",
                    };
                    format!("Enters the battlefield with {article} {counter} counter on it")
                } else {
                    let rendered = u32::try_from(*v)
                        .ok()
                        .and_then(number_word_u32)
                        .unwrap_or_else(|| v.to_string());
                    format!("Enters the battlefield with {rendered} {counter} counters on it")
                }
            }
            Value::X => {
                format!("Enters the battlefield with X {counter} counters on it")
            }
            Value::ColorsOfManaSpentToCastThisSpell => {
                format!(
                    "Enters the battlefield with a {counter} counter on it for each color of mana spent to cast it"
                )
            }
            Value::ManaSpentToCastThisSpell => {
                format!(
                    "Enters the battlefield with a number of {counter} counters on it equal to the amount of mana spent to cast it"
                )
            }
            Value::Count(filter) => {
                if is_revealed_this_way_count_filter(filter) {
                    let article = counter_indefinite_article(&counter);
                    return format!(
                        "Enters the battlefield with {article} {counter} counter on it for each card revealed this way"
                    );
                }
                let count_filter = describe_enters_with_counters_count_filter(filter);
                format!(
                    "Enters the battlefield with X {counter} counters on it, where X is the number of {}",
                    count_filter
                )
            }
            Value::CountScaled(filter, scale) => {
                if *scale == 1 && is_revealed_this_way_count_filter(filter) {
                    let article = counter_indefinite_article(&counter);
                    return format!(
                        "Enters the battlefield with {article} {counter} counter on it for each card revealed this way"
                    );
                }
                let count_filter = describe_enters_with_counters_count_filter(filter);
                if *scale == 1 {
                    format!(
                        "Enters the battlefield with X {counter} counters on it, where X is the number of {}",
                        count_filter
                    )
                } else {
                    format!(
                        "Enters the battlefield with X {counter} counters on it, where X is {} times the number of {}",
                        scale, count_filter
                    )
                }
            }
            _ => format!(
                "Enters the battlefield with {} {counter} counters on it",
                describe_value(&self.count)
            ),
        }
    }

    fn generate_replacement_effect(
        &self,
        source: ObjectId,
        controller: PlayerId,
    ) -> Option<ReplacementEffect> {
        Some(ReplacementEffect::with_matcher(
            source,
            controller,
            ThisWouldEnterBattlefieldMatcher,
            ReplacementAction::EnterWithCounters {
                counter_type: self.counter_type,
                count: self.count.clone(),
                added_subtypes: Vec::new(),
                added_abilities: Vec::new(),
            },
        ))
    }
}

/// Enters the battlefield with the controller's choice of one counter type.
#[derive(Debug, Clone, PartialEq)]
pub struct EntersWithCounterChoice {
    pub counter_types: Vec<CounterType>,
    pub count: Value,
}

impl EntersWithCounterChoice {
    pub fn new(counter_types: Vec<CounterType>, count: Value) -> Self {
        Self {
            counter_types,
            count,
        }
    }
}

impl StaticAbilityKind for EntersWithCounterChoice {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::EnterWithCounters
    }

    fn display(&self) -> String {
        let counter_choices = self
            .counter_types
            .iter()
            .map(|counter_type| {
                let counter = counter_type.description();
                let article = match counter.chars().next().map(|ch| ch.to_ascii_lowercase()) {
                    Some('a' | 'e' | 'i' | 'o' | 'u') => "an",
                    _ => "a",
                };
                format!("{article} {counter} counter")
            })
            .collect::<Vec<_>>();
        let choices = match counter_choices.as_slice() {
            [] => "a counter".to_string(),
            [single] => single.clone(),
            [head @ .., last] => format!("{} or {last}", head.join(", ")),
        };
        let count_prefix = match &self.count {
            Value::Fixed(1) => String::new(),
            value => format!("{} ", describe_value(value)),
        };
        format!("Enters the battlefield with your choice of {count_prefix}{choices} on it")
    }

    fn generate_replacement_effect(
        &self,
        source: ObjectId,
        controller: PlayerId,
    ) -> Option<ReplacementEffect> {
        Some(ReplacementEffect::with_matcher(
            source,
            controller,
            ThisWouldEnterBattlefieldMatcher,
            ReplacementAction::EnterWithCounterChoice {
                counter_types: self.counter_types.clone(),
                count: self.count.clone(),
            },
        ))
    }
}

/// Enters the battlefield with counters if a condition is true.
#[derive(Debug, Clone, PartialEq)]
pub struct EntersWithCountersIfCondition {
    pub counter_type: CounterType,
    pub count: Value,
    pub condition: Condition,
    pub condition_display: String,
    pub added_abilities: Vec<Ability>,
}

impl EntersWithCountersIfCondition {
    pub fn new(
        counter_type: CounterType,
        count: Value,
        condition: Condition,
        condition_display: String,
    ) -> Self {
        Self::new_with_abilities(
            counter_type,
            count,
            condition,
            condition_display,
            Vec::new(),
        )
    }

    pub fn new_with_abilities(
        counter_type: CounterType,
        count: Value,
        condition: Condition,
        condition_display: String,
        added_abilities: Vec<Ability>,
    ) -> Self {
        Self {
            counter_type,
            count,
            condition,
            condition_display: condition_display.trim().to_string(),
            added_abilities,
        }
    }
}

impl StaticAbilityKind for EntersWithCountersIfCondition {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::EnterWithCountersIfCondition
    }

    fn display(&self) -> String {
        let mut base = EntersWithCounters::new(self.counter_type, self.count.clone()).display();
        if self.condition == Condition::ThisSpellEscaped
            && self.added_abilities.is_empty()
            && let Some(rest) = base.strip_prefix("Enters the battlefield with ")
        {
            return format!("Escapes with {rest}");
        }
        if !self.added_abilities.is_empty() {
            let ability_text = self
                .added_abilities
                .iter()
                .filter_map(entered_ability_display)
                .collect::<Vec<_>>()
                .join(" and ");
            if !ability_text.is_empty() {
                let addition = format!(" and with {ability_text}");
                if let Some((head, where_clause)) = base.split_once(", where X is ") {
                    base = format!("{head}{addition}, where X is {where_clause}");
                } else {
                    base.push_str(&addition);
                }
            }
        }
        let condition = self.condition_display.trim();
        if condition.is_empty() {
            base
        } else {
            format!("{base} if {condition}")
        }
    }

    fn generate_replacement_effect(
        &self,
        source: ObjectId,
        controller: PlayerId,
    ) -> Option<ReplacementEffect> {
        Some(ReplacementEffect::with_matcher(
            source,
            controller,
            ThisWouldEnterWithCountersIfConditionMatcher {
                condition: self.condition.clone(),
                condition_display: self.condition_display.clone(),
            },
            ReplacementAction::EnterWithCounters {
                counter_type: self.counter_type,
                count: self.count.clone(),
                added_subtypes: Vec::new(),
                added_abilities: self.added_abilities.clone(),
            },
        ))
    }
}

fn entered_ability_display(ability: &Ability) -> Option<String> {
    match &ability.kind {
        AbilityKind::Static(static_ability) => Some(static_ability.display().to_ascii_lowercase()),
        _ => None,
    }
}

#[derive(Debug, Clone, PartialEq)]
struct ThisWouldEnterWithCountersIfConditionMatcher {
    condition: Condition,
    condition_display: String,
}

impl ReplacementMatcher for ThisWouldEnterWithCountersIfConditionMatcher {
    fn matches_event(
        &self,
        event: &dyn crate::events::traits::GameEventType,
        ctx: &crate::events::EventContext,
    ) -> bool {
        if !matches_this_would_enter_battlefield(event, ctx) {
            return false;
        }

        let Some(source) = ctx.source else {
            return false;
        };
        let eval_ctx = crate::condition_eval::ExternalEvaluationContext {
            controller: ctx.controller,
            source,
            defending_player: None,
            attacking_player: None,
            filter_source: None,
            iterated_player: None,
            triggering_event: None,
            trigger_identity: None,
            ability_index: None,
            options: Default::default(),
        };

        crate::condition_eval::evaluate_condition_external(ctx.game, &self.condition, &eval_ctx)
    }

    fn priority(&self) -> ReplacementPriority {
        ReplacementPriority::Other
    }

    fn display(&self) -> String {
        let condition = self.condition_display.trim();
        if condition.is_empty() {
            "When this would enter with counters".to_string()
        } else {
            format!("When this would enter with counters if {condition}")
        }
    }
}

/// If this would be put into a graveyard from anywhere, shuffle into library instead.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ShuffleIntoLibraryFromGraveyard;

impl StaticAbilityKind for ShuffleIntoLibraryFromGraveyard {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::ShuffleIntoLibraryFromGraveyard
    }

    fn display(&self) -> String {
        "If this would be put into a graveyard from anywhere, shuffle it into its owner's library instead".to_string()
    }

    fn generate_replacement_effect(
        &self,
        source: ObjectId,
        controller: PlayerId,
    ) -> Option<ReplacementEffect> {
        Some(
            ZoneReplacementSpec::new(ObjectFilter::specific(source), crate::zone::Zone::Library)
                .to_zone(crate::zone::Zone::Graveyard)
                .build(source, controller),
        )
    }
}

/// All permanents enter the battlefield tapped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AllPermanentsEnterTapped;

impl StaticAbilityKind for AllPermanentsEnterTapped {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::AllPermanentsEnterTapped
    }

    fn display(&self) -> String {
        "Permanents enter the battlefield tapped".to_string()
    }

    fn generate_replacement_effect(
        &self,
        source: ObjectId,
        controller: PlayerId,
    ) -> Option<ReplacementEffect> {
        Some(ReplacementEffect::with_matcher(
            source,
            controller,
            WouldEnterBattlefieldMatcher::any(),
            ReplacementAction::EnterTapped,
        ))
    }
}

/// Generic static mana-spend permission.
#[derive(Debug, Clone, PartialEq)]
pub struct ManaSpendPermissionAbility {
    pub permission: crate::effect::ManaSpendPermission,
    pub display: String,
}

impl ManaSpendPermissionAbility {
    pub fn new(permission: crate::effect::ManaSpendPermission, display: String) -> Self {
        Self {
            permission,
            display,
        }
    }
}

impl StaticAbilityKind for ManaSpendPermissionAbility {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::ManaSpendPermission
    }

    fn display(&self) -> String {
        self.display.clone()
    }

    fn apply_restrictions(&self, game: &mut GameState, _source: ObjectId, controller: PlayerId) {
        game.effect_store.mana_spend_effects.permissions.push(
            crate::game_state::ActiveManaSpendPermission {
                permission: self.permission.clone(),
                controller,
                source: crate::game_state::ManaSpendPermissionSource::StaticAbility,
            },
        );
    }
}

/// "Damage isn't removed from this creature during cleanup steps."
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DamageNotRemovedDuringCleanup;

impl StaticAbilityKind for DamageNotRemovedDuringCleanup {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::DamageNotRemovedDuringCleanup
    }

    fn display(&self) -> String {
        "Damage isn't removed from this creature during cleanup steps.".to_string()
    }

    fn apply_restrictions(&self, game: &mut GameState, source: ObjectId, _controller: PlayerId) {
        game.keep_damage_marked(source);
    }
}

/// "As this enters, choose a color other than [color]."
#[derive(Debug, Clone, PartialEq)]
pub struct ChooseColorAsEnters {
    pub excluded: Option<Color>,
    pub display: String,
}

impl ChooseColorAsEnters {
    pub fn new(excluded: Option<Color>, display: String) -> Self {
        Self { excluded, display }
    }
}

impl StaticAbilityKind for ChooseColorAsEnters {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::ChooseColorAsEnters
    }

    fn display(&self) -> String {
        self.display.clone()
    }

    fn color_choice_as_enters(&self) -> Option<ChooseColorAsEntersSpec> {
        Some(ChooseColorAsEntersSpec {
            excluded: self.excluded,
        })
    }
}

/// "As this becomes attached, choose a color."
#[derive(Debug, Clone, PartialEq)]
pub struct ChooseColorAsBecomesAttached {
    pub display: String,
}

impl ChooseColorAsBecomesAttached {
    pub fn new(display: String) -> Self {
        Self { display }
    }
}

impl StaticAbilityKind for ChooseColorAsBecomesAttached {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::ChooseColorAsBecomesAttached
    }

    fn display(&self) -> String {
        self.display.clone()
    }

    fn color_choice_as_becomes_attached(&self) -> Option<ChooseColorAsBecomesAttachedSpec> {
        Some(ChooseColorAsBecomesAttachedSpec)
    }
}

/// "As this enters, choose a player."
#[derive(Debug, Clone, PartialEq)]
pub struct ChoosePlayerAsEnters {
    pub display: String,
}

impl ChoosePlayerAsEnters {
    pub fn new(display: String) -> Self {
        Self { display }
    }
}

impl StaticAbilityKind for ChoosePlayerAsEnters {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::ChoosePlayerAsEnters
    }

    fn display(&self) -> String {
        self.display.clone()
    }

    fn player_choice_as_enters(&self) -> Option<ChoosePlayerAsEntersSpec> {
        Some(ChoosePlayerAsEntersSpec)
    }
}

/// "As this enters, note your life total."
#[derive(Debug, Clone, PartialEq)]
pub struct NoteLifeTotalAsEnters {
    pub display: String,
}

impl NoteLifeTotalAsEnters {
    pub fn new(display: String) -> Self {
        Self { display }
    }
}

impl StaticAbilityKind for NoteLifeTotalAsEnters {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::NoteLifeTotalAsEnters
    }

    fn display(&self) -> String {
        self.display.clone()
    }

    fn life_total_note_as_enters(&self) -> Option<NoteLifeTotalAsEntersSpec> {
        Some(NoteLifeTotalAsEntersSpec)
    }
}

/// "As this enters, you may reveal cards from your hand."
#[derive(Debug, Clone, PartialEq)]
pub struct RevealFromHandAsEnters {
    pub filter: ObjectFilter,
    pub count: crate::ChoiceCount,
    pub optional: bool,
    pub display: String,
}

impl RevealFromHandAsEnters {
    pub fn new(
        filter: ObjectFilter,
        count: crate::ChoiceCount,
        optional: bool,
        display: String,
    ) -> Self {
        Self {
            filter,
            count,
            optional,
            display,
        }
    }
}

impl StaticAbilityKind for RevealFromHandAsEnters {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::RevealFromHandAsEnters
    }

    fn display(&self) -> String {
        self.display.clone()
    }

    fn reveal_from_hand_as_enters(&self) -> Option<RevealFromHandAsEntersSpec> {
        Some(RevealFromHandAsEntersSpec {
            filter: self.filter.clone(),
            count: self.count,
            optional: self.optional,
        })
    }
}

/// "As this enters, choose a card name."
#[derive(Debug, Clone, PartialEq)]
pub struct ChooseCardNameAsEnters {
    pub display: String,
    pub reveal_opponents_hands: bool,
    pub require_nonland_from_revealed_opponents: bool,
}

impl ChooseCardNameAsEnters {
    pub fn new(display: String) -> Self {
        Self::with_spec(display, ChooseCardNameAsEntersSpec::default())
    }

    pub fn with_spec(display: String, spec: ChooseCardNameAsEntersSpec) -> Self {
        Self {
            display,
            reveal_opponents_hands: spec.reveal_opponents_hands,
            require_nonland_from_revealed_opponents: spec.require_nonland_from_revealed_opponents,
        }
    }
}

impl StaticAbilityKind for ChooseCardNameAsEnters {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::ChooseCardNameAsEnters
    }

    fn display(&self) -> String {
        self.display.clone()
    }

    fn card_name_choice_as_enters(&self) -> Option<ChooseCardNameAsEntersSpec> {
        Some(ChooseCardNameAsEntersSpec {
            reveal_opponents_hands: self.reveal_opponents_hands,
            require_nonland_from_revealed_opponents: self.require_nonland_from_revealed_opponents,
        })
    }
}

/// "As this Aura enters, choose a basic land type."
#[derive(Debug, Clone, PartialEq)]
pub struct ChooseBasicLandTypeAsEnters {
    pub display: String,
}

impl ChooseBasicLandTypeAsEnters {
    pub fn new(display: String) -> Self {
        Self { display }
    }
}

impl StaticAbilityKind for ChooseBasicLandTypeAsEnters {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::ChooseBasicLandTypeAsEnters
    }

    fn display(&self) -> String {
        self.display.clone()
    }

    fn basic_land_type_choice_as_enters(&self) -> Option<ChooseBasicLandTypeAsEntersSpec> {
        Some(ChooseBasicLandTypeAsEntersSpec)
    }
}

/// "As this enters, choose a land type."
#[derive(Debug, Clone, PartialEq)]
pub struct ChooseLandTypeAsEnters {
    pub display: String,
}

impl ChooseLandTypeAsEnters {
    pub fn new(display: String) -> Self {
        Self { display }
    }
}

impl StaticAbilityKind for ChooseLandTypeAsEnters {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::ChooseLandTypeAsEnters
    }

    fn display(&self) -> String {
        self.display.clone()
    }

    fn land_type_choice_as_enters(&self) -> Option<ChooseLandTypeAsEntersSpec> {
        Some(ChooseLandTypeAsEntersSpec)
    }
}

/// "As this enters, choose a creature type."
#[derive(Debug, Clone, PartialEq)]
pub struct ChooseCreatureTypeAsEnters {
    pub display: String,
}

impl ChooseCreatureTypeAsEnters {
    pub fn new(display: String) -> Self {
        Self { display }
    }
}

impl StaticAbilityKind for ChooseCreatureTypeAsEnters {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::ChooseCreatureTypeAsEnters
    }

    fn display(&self) -> String {
        self.display.clone()
    }

    fn creature_type_choice_as_enters(&self) -> Option<ChooseCreatureTypeAsEntersSpec> {
        Some(ChooseCreatureTypeAsEntersSpec)
    }
}

/// "As this enters, choose <A> or <B>."
#[derive(Debug, Clone, PartialEq)]
pub struct ChooseNamedOptionAsEnters {
    pub options: Vec<String>,
    pub display: String,
}

impl ChooseNamedOptionAsEnters {
    pub fn new(options: Vec<String>, display: String) -> Self {
        Self { options, display }
    }
}

impl StaticAbilityKind for ChooseNamedOptionAsEnters {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::ChooseNamedOptionAsEnters
    }

    fn display(&self) -> String {
        self.display.clone()
    }

    fn named_option_choice_as_enters(&self) -> Option<ChooseNamedOptionAsEntersSpec> {
        Some(ChooseNamedOptionAsEntersSpec {
            options: self.options.clone(),
        })
    }
}

/// "As this enters or is turned face up, choose a power/toughness."
#[derive(Debug, Clone, PartialEq)]
pub struct ChoosePowerToughnessAsEntersOrTurnsFaceUp {
    pub options: Vec<PowerToughnessChoiceOption>,
    pub display: String,
}

impl ChoosePowerToughnessAsEntersOrTurnsFaceUp {
    pub fn new(options: Vec<(i32, i32)>, display: String) -> Self {
        let options = options
            .into_iter()
            .map(|(power, toughness)| PowerToughnessChoiceOption::new(power, toughness))
            .collect();
        Self { options, display }
    }

    pub fn new_with_options(options: Vec<PowerToughnessChoiceOption>, display: String) -> Self {
        Self { options, display }
    }
}

impl StaticAbilityKind for ChoosePowerToughnessAsEntersOrTurnsFaceUp {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::ChoosePowerToughnessAsEntersOrTurnsFaceUp
    }

    fn display(&self) -> String {
        self.display.clone()
    }

    fn power_toughness_choice_as_enters_or_turns_face_up(
        &self,
    ) -> Option<ChoosePowerToughnessAsEntersOrTurnsFaceUpSpec> {
        Some(ChoosePowerToughnessAsEntersOrTurnsFaceUpSpec {
            options: self.options.clone(),
        })
    }
}

/// "You may have this enter tapped as a copy of ..."
#[derive(Debug, Clone, PartialEq)]
pub struct EnterAsCopyAsEnters {
    pub spec: EnterAsCopyAsEntersSpec,
    pub display: String,
}

impl EnterAsCopyAsEnters {
    pub fn new(spec: EnterAsCopyAsEntersSpec, display: String) -> Self {
        Self { spec, display }
    }
}

impl StaticAbilityKind for EnterAsCopyAsEnters {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::EnterAsCopyAsEnters
    }

    fn display(&self) -> String {
        self.display.clone()
    }

    fn enter_as_copy_as_enters(&self) -> Option<&EnterAsCopyAsEntersSpec> {
        Some(&self.spec)
    }
}

/// "All damage that would be dealt to you and other permanents you control is dealt to this creature instead."
#[derive(Debug, Clone, PartialEq)]
pub struct RedirectDamageToSource {
    pub player_filter: PlayerFilter,
    pub object_filter: ObjectFilter,
    pub display: String,
}

impl RedirectDamageToSource {
    pub fn new(player_filter: PlayerFilter, object_filter: ObjectFilter, display: String) -> Self {
        Self {
            player_filter,
            object_filter,
            display,
        }
    }
}

impl StaticAbilityKind for RedirectDamageToSource {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::RedirectDamageToSource
    }

    fn display(&self) -> String {
        self.display.clone()
    }

    fn generate_replacement_effect(
        &self,
        source: ObjectId,
        controller: PlayerId,
    ) -> Option<ReplacementEffect> {
        Some(ReplacementEffect::with_matcher(
            source,
            controller,
            DamageToPlayerOrObjectMatcher::new(
                self.player_filter.clone(),
                self.object_filter.clone(),
            ),
            ReplacementAction::Redirect {
                target: RedirectTarget::ToSource,
                which: RedirectWhich::First,
            },
        ))
    }
}

/// "Prevent all damage that would be dealt to and dealt by this permanent."
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PreventAllDamageDealtToAndByThisPermanent;

impl StaticAbilityKind for PreventAllDamageDealtToAndByThisPermanent {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::PreventAllDamageDealtToAndByThisPermanent
    }

    fn display(&self) -> String {
        "Prevent all damage that would be dealt to and dealt by this permanent.".to_string()
    }

    fn generate_replacement_effect(
        &self,
        source: ObjectId,
        controller: PlayerId,
    ) -> Option<ReplacementEffect> {
        Some(ReplacementEffect::with_matcher(
            source,
            controller,
            crate::events::DamageToOrFromSelfMatcher::new(),
            ReplacementAction::Prevent,
        ))
    }
}

/// "Prevent all damage that would be dealt by this permanent."
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PreventAllDamageDealtByThisPermanent;

impl StaticAbilityKind for PreventAllDamageDealtByThisPermanent {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::PreventAllDamageDealtByThisPermanent
    }

    fn display(&self) -> String {
        "Prevent all damage that would be dealt by this permanent.".to_string()
    }

    fn generate_replacement_effect(
        &self,
        source: ObjectId,
        controller: PlayerId,
    ) -> Option<ReplacementEffect> {
        Some(ReplacementEffect::with_matcher(
            source,
            controller,
            DamageFromSelfMatcher::new(),
            ReplacementAction::Prevent,
        ))
    }
}

/// "Prevent all combat damage that would be dealt by this permanent."
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PreventAllCombatDamageDealtByThisPermanent;

impl StaticAbilityKind for PreventAllCombatDamageDealtByThisPermanent {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::PreventAllCombatDamageDealtByThisPermanent
    }

    fn display(&self) -> String {
        "Prevent all combat damage that would be dealt by this permanent.".to_string()
    }

    fn generate_replacement_effect(
        &self,
        source: ObjectId,
        controller: PlayerId,
    ) -> Option<ReplacementEffect> {
        Some(ReplacementEffect::with_matcher(
            source,
            controller,
            DamageFromSelfCombatMatcher::new(),
            ReplacementAction::Prevent,
        ))
    }
}

/// "Prevent all damage that would be dealt to creatures."
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PreventAllDamageDealtToCreatures;

impl StaticAbilityKind for PreventAllDamageDealtToCreatures {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::PreventAllDamageDealtToCreatures
    }

    fn display(&self) -> String {
        "Prevent all damage that would be dealt to creatures.".to_string()
    }

    fn generate_replacement_effect(
        &self,
        source: ObjectId,
        controller: PlayerId,
    ) -> Option<ReplacementEffect> {
        Some(ReplacementEffect::with_matcher(
            source,
            controller,
            DamageToObjectMatcher::to_creature(),
            ReplacementAction::Prevent,
        ))
    }
}

/// "Prevent all combat damage that would be dealt to this creature."
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PreventAllCombatDamageToSelf;

impl StaticAbilityKind for PreventAllCombatDamageToSelf {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::PreventAllCombatDamageToSelf
    }

    fn display(&self) -> String {
        "Prevent all combat damage that would be dealt to this creature.".to_string()
    }

    fn generate_replacement_effect(
        &self,
        source: ObjectId,
        controller: PlayerId,
    ) -> Option<ReplacementEffect> {
        Some(ReplacementEffect::with_matcher(
            source,
            controller,
            DamageToSelfCombatMatcher::new(),
            ReplacementAction::Prevent,
        ))
    }
}

/// "Prevent all combat damage that would be dealt to [matching permanents]."
#[derive(Debug, Clone, PartialEq)]
pub struct PreventAllCombatDamageToPermanentsMatching {
    pub filter: ObjectFilter,
}

impl PreventAllCombatDamageToPermanentsMatching {
    pub fn new(filter: ObjectFilter) -> Self {
        Self { filter }
    }
}

impl StaticAbilityKind for PreventAllCombatDamageToPermanentsMatching {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::PreventAllCombatDamageToPermanentsMatching
    }

    fn display(&self) -> String {
        format!(
            "Prevent all combat damage that would be dealt to {}.",
            pluralize_filter_description(&self.filter.description())
        )
    }

    fn generate_replacement_effect(
        &self,
        source: ObjectId,
        controller: PlayerId,
    ) -> Option<ReplacementEffect> {
        Some(ReplacementEffect::with_matcher(
            source,
            controller,
            PreventableCombatDamageToObjectMatcher::new(self.filter.clone()),
            ReplacementAction::Prevent,
        ))
    }
}

/// "Prevent all noncombat damage that would be dealt to [matching permanents]."
#[derive(Debug, Clone, PartialEq)]
pub struct PreventAllNoncombatDamageToPermanentsMatching {
    pub filter: ObjectFilter,
}

impl PreventAllNoncombatDamageToPermanentsMatching {
    pub fn new(filter: ObjectFilter) -> Self {
        Self { filter }
    }
}

impl StaticAbilityKind for PreventAllNoncombatDamageToPermanentsMatching {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::PreventAllNoncombatDamageToPermanentsMatching
    }

    fn display(&self) -> String {
        format!(
            "Prevent all noncombat damage that would be dealt to {}.",
            pluralize_filter_description(&self.filter.description())
        )
    }

    fn generate_replacement_effect(
        &self,
        source: ObjectId,
        controller: PlayerId,
    ) -> Option<ReplacementEffect> {
        Some(ReplacementEffect::with_matcher(
            source,
            controller,
            PreventableNoncombatDamageToObjectMatcher::new(self.filter.clone()),
            ReplacementAction::Prevent,
        ))
    }
}

/// "Prevent all damage that would be dealt to this creature."
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PreventAllDamageToSelf;

impl StaticAbilityKind for PreventAllDamageToSelf {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::PreventAllDamageToSelf
    }

    fn display(&self) -> String {
        "Prevent all damage that would be dealt to this creature.".to_string()
    }

    fn generate_replacement_effect(
        &self,
        source: ObjectId,
        controller: PlayerId,
    ) -> Option<ReplacementEffect> {
        Some(ReplacementEffect::with_matcher(
            source,
            controller,
            crate::events::DamageToSelfMatcher::new(),
            ReplacementAction::Prevent,
        ))
    }
}

/// "Prevent all damage that would be dealt to this creature by creatures."
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PreventAllDamageToSelfByCreatures;

impl StaticAbilityKind for PreventAllDamageToSelfByCreatures {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::PreventAllDamageToSelfByCreatures
    }

    fn display(&self) -> String {
        "Prevent all damage that would be dealt to this creature by creatures.".to_string()
    }

    fn generate_replacement_effect(
        &self,
        source: ObjectId,
        controller: PlayerId,
    ) -> Option<ReplacementEffect> {
        Some(ReplacementEffect::with_matcher(
            source,
            controller,
            DamageToSelfFromSourceFilterMatcher::from_creature(),
            ReplacementAction::Prevent,
        ))
    }
}

/// "If a matching source would deal damage to you, prevent N of that damage."
#[derive(Debug, Clone, PartialEq)]
pub struct PreventDamageToYouFromSourceFilter {
    pub amount: u32,
    pub source_filter: ObjectFilter,
    pub display: String,
}

impl PreventDamageToYouFromSourceFilter {
    pub fn new(amount: u32, source_filter: ObjectFilter, display: impl Into<String>) -> Self {
        Self {
            amount,
            source_filter,
            display: display.into(),
        }
    }
}

impl StaticAbilityKind for PreventDamageToYouFromSourceFilter {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::PreventDamageToYouFromSourceFilter
    }

    fn display(&self) -> String {
        self.display.clone()
    }

    fn generate_replacement_effect(
        &self,
        source: ObjectId,
        controller: PlayerId,
    ) -> Option<ReplacementEffect> {
        Some(ReplacementEffect::with_matcher(
            source,
            controller,
            DamageFromSourceToPlayerMatcher::to_you(self.source_filter.clone()),
            ReplacementAction::Modify(EventModification::Subtract(self.amount)),
        ))
    }
}

/// "If damage would be dealt to this creature, prevent that damage.
/// Remove N <counter> counter(s) from this creature."
#[derive(Debug, Clone, PartialEq)]
pub struct PreventDamageToSelfRemoveCounter {
    pub counter_type: CounterType,
    pub amount: Value,
    pub follow_up: Option<ironsmith_core::CounterRemovalFollowUp>,
}

impl PreventDamageToSelfRemoveCounter {
    pub fn new(counter_type: CounterType, amount: impl Into<Value>) -> Self {
        Self::new_with_follow_up(counter_type, amount, None)
    }

    pub fn new_with_follow_up(
        counter_type: CounterType,
        amount: impl Into<Value>,
        follow_up: Option<ironsmith_core::CounterRemovalFollowUp>,
    ) -> Self {
        Self {
            counter_type,
            amount: amount.into(),
            follow_up,
        }
    }
}

impl StaticAbilityKind for PreventDamageToSelfRemoveCounter {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::PreventDamageToSelfRemoveCounter
    }

    fn display(&self) -> String {
        let counter = self.counter_type.description().into_owned();
        let (amount_word, suffix) = match &self.amount {
            Value::Fixed(amount) => {
                let rendered = u32::try_from(*amount)
                    .ok()
                    .and_then(number_word_u32)
                    .unwrap_or_else(|| amount.to_string());
                (rendered, if *amount == 1 { "" } else { "s" })
            }
            Value::EventValue(EventValueSpec::Amount) => ("that many".to_string(), "s"),
            amount => (describe_value(amount), "s"),
        };
        let mut display = format!(
            "If damage would be dealt to this creature, prevent that damage. Remove {amount_word} {counter} counter{suffix} from this creature."
        );
        if let Some(ironsmith_core::CounterRemovalFollowUp::EachPlayerGetsCounters {
            counter_type,
            counters_per_removed,
        }) = self.follow_up
        {
            let gained = counter_type.description();
            let amount = number_word_u32(counters_per_removed)
                .unwrap_or_else(|| counters_per_removed.to_string());
            let article_or_amount = if counters_per_removed == 1 {
                "a".to_string()
            } else {
                amount
            };
            let plural = if counters_per_removed == 1 { "" } else { "s" };
            display.push_str(&format!(
                " Then give each player {article_or_amount} {gained} counter{plural} for each {counter} counter removed this way."
            ));
        }
        display
    }

    fn generate_replacement_effect(
        &self,
        source: ObjectId,
        controller: PlayerId,
    ) -> Option<ReplacementEffect> {
        let mut effects = vec![Effect::remove_counters(
            self.counter_type,
            self.amount.clone(),
            ChooseSpec::Source,
        )];
        if let Some(ironsmith_core::CounterRemovalFollowUp::EachPlayerGetsCounters {
            counter_type,
            counters_per_removed,
        }) = self.follow_up
        {
            const REMOVED_COUNTER_COUNT_EFFECT_ID: u32 = 0;
            let remove_effect = effects.remove(0);
            effects.push(Effect::with_id(
                REMOVED_COUNTER_COUNT_EFFECT_ID,
                remove_effect,
            ));
            let removed_count =
                Value::EffectValue(crate::effect::EffectId(REMOVED_COUNTER_COUNT_EFFECT_ID));
            let count = if counters_per_removed == 1 {
                removed_count
            } else {
                Value::Scaled(Box::new(removed_count), counters_per_removed as i32)
            };
            effects.push(Effect::for_players(
                crate::target::PlayerFilter::Any,
                vec![Effect::new(crate::effects::PlayerCountersEffect::new(
                    counter_type,
                    count,
                    crate::target::PlayerFilter::IteratedPlayer,
                ))],
            ));
        }
        Some(ReplacementEffect::with_matcher(
            source,
            controller,
            crate::events::DamageToSelfMatcher::new(),
            ReplacementAction::Instead(effects),
        ))
    }
}

/// "If damage would be dealt to this creature, put that many +1/+1 counters on it instead."
#[derive(Debug, Clone, PartialEq)]
pub struct PreventDamageToSelfPutCountersInstead {
    pub counter_type: CounterType,
    pub display: String,
}

impl PreventDamageToSelfPutCountersInstead {
    pub fn new(counter_type: CounterType, display: impl Into<String>) -> Self {
        Self {
            counter_type,
            display: display.into(),
        }
    }
}

impl StaticAbilityKind for PreventDamageToSelfPutCountersInstead {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::PreventDamageToSelfPutCountersInstead
    }

    fn display(&self) -> String {
        self.display.clone()
    }

    fn generate_replacement_effect(
        &self,
        source: ObjectId,
        controller: PlayerId,
    ) -> Option<ReplacementEffect> {
        Some(ReplacementEffect::with_matcher(
            source,
            controller,
            crate::events::DamageToSelfMatcher::new(),
            ReplacementAction::Instead(vec![Effect::put_counters_on_source(
                self.counter_type,
                Value::EventValue(EventValueSpec::Amount),
            )]),
        ))
    }
}

/// Constrained self-damage replacement that turns prevented damage into counters.
#[derive(Debug, Clone, PartialEq)]
pub struct PreventConstrainedDamageToSelfPutCountersInstead {
    pub counter_type: CounterType,
    pub display: String,
    pub source_filter: Option<ObjectFilter>,
    pub combat_only: Option<bool>,
}

impl PreventConstrainedDamageToSelfPutCountersInstead {
    pub fn new(
        counter_type: CounterType,
        display: impl Into<String>,
        source_filter: Option<ObjectFilter>,
        combat_only: Option<bool>,
    ) -> Self {
        Self {
            counter_type,
            display: display.into(),
            source_filter,
            combat_only,
        }
    }
}

impl StaticAbilityKind for PreventConstrainedDamageToSelfPutCountersInstead {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::PreventConstrainedDamageToSelfPutCountersInstead
    }

    fn display(&self) -> String {
        self.display.clone()
    }

    fn generate_replacement_effect(
        &self,
        source: ObjectId,
        controller: PlayerId,
    ) -> Option<ReplacementEffect> {
        let matcher = match (&self.source_filter, self.combat_only) {
            (Some(filter), Some(true)) => {
                DamageToSelfConstraintMatcher::combat_from_source_filter(filter.clone())
            }
            (Some(filter), _) => DamageToSelfConstraintMatcher::from_source_filter(filter.clone()),
            (None, Some(false)) => DamageToSelfConstraintMatcher::noncombat(),
            _ => DamageToSelfConstraintMatcher::new(),
        };
        Some(ReplacementEffect::with_matcher(
            source,
            controller,
            matcher,
            ReplacementAction::Instead(vec![Effect::put_counters_on_source(
                self.counter_type,
                Value::EventValue(EventValueSpec::Amount),
            )]),
        ))
    }
}

/// "If matching damage would be dealt to a matching creature, put counters on it instead."
#[derive(Debug, Clone, PartialEq)]
pub struct ReplaceDamageWithCountersInstead {
    pub counter_type: CounterType,
    pub display: String,
    pub source_filter: ObjectFilter,
    pub target_filter: ObjectFilter,
    pub combat_only: Option<bool>,
}

impl ReplaceDamageWithCountersInstead {
    pub fn new(
        counter_type: CounterType,
        source_filter: ObjectFilter,
        target_filter: ObjectFilter,
        combat_only: Option<bool>,
        display: impl Into<String>,
    ) -> Self {
        Self {
            counter_type,
            display: display.into(),
            source_filter,
            target_filter,
            combat_only,
        }
    }
}

impl StaticAbilityKind for ReplaceDamageWithCountersInstead {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::ReplaceDamageWithCountersInstead
    }

    fn display(&self) -> String {
        self.display.clone()
    }

    fn generate_replacement_effect(
        &self,
        source: ObjectId,
        controller: PlayerId,
    ) -> Option<ReplacementEffect> {
        Some(ReplacementEffect::with_matcher(
            source,
            controller,
            DamageFromSourceToObjectMatcher::new(
                self.source_filter.clone(),
                self.target_filter.clone(),
            )
            .with_combat_only(self.combat_only),
            ReplacementAction::Instead(vec![Effect::put_counters(
                self.counter_type,
                Value::EventValue(EventValueSpec::Amount),
                ChooseSpec::AnyTarget,
            )]),
        ))
    }
}

/// "If damage would be dealt to another creature you control, prevent that damage..."
#[derive(Debug, Clone, PartialEq)]
pub struct PreventDamageToOtherCreatureYouControlPutCountersInstead {
    pub counter_type: CounterType,
    pub display: String,
}

impl PreventDamageToOtherCreatureYouControlPutCountersInstead {
    pub fn new(counter_type: CounterType, display: impl Into<String>) -> Self {
        Self {
            counter_type,
            display: display.into(),
        }
    }
}

impl StaticAbilityKind for PreventDamageToOtherCreatureYouControlPutCountersInstead {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::PreventDamageToOtherCreatureYouControlPutCountersInstead
    }

    fn display(&self) -> String {
        self.display.clone()
    }

    fn generate_replacement_effect(
        &self,
        source: ObjectId,
        controller: PlayerId,
    ) -> Option<ReplacementEffect> {
        Some(ReplacementEffect::with_matcher(
            source,
            controller,
            DamageToOtherCreatureYouControlMatcher::new(),
            ReplacementAction::Instead(vec![Effect::put_counters(
                self.counter_type,
                Value::EventValue(EventValueSpec::Amount),
                ChooseSpec::AnyTarget,
            )]),
        ))
    }
}

/// "Prevent all noncombat damage that would be dealt to other creatures you control."
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PreventAllNoncombatDamageToOtherCreaturesYouControl;

impl StaticAbilityKind for PreventAllNoncombatDamageToOtherCreaturesYouControl {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::PreventAllNoncombatDamageToOtherCreaturesYouControl
    }

    fn display(&self) -> String {
        "Prevent all noncombat damage that would be dealt to other creatures you control."
            .to_string()
    }

    fn generate_replacement_effect(
        &self,
        source: ObjectId,
        controller: PlayerId,
    ) -> Option<ReplacementEffect> {
        Some(ReplacementEffect::with_matcher(
            source,
            controller,
            DamageToOtherCreatureYouControlMatcher::noncombat_only(),
            ReplacementAction::Prevent,
        ))
    }
}

/// Umbra armor.
///
/// If the permanent this Aura is attached to would be destroyed, instead
/// remove all damage marked on that permanent and destroy this Aura.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UmbraArmor;

impl StaticAbilityKind for UmbraArmor {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::UmbraArmor
    }

    fn display(&self) -> String {
        "Umbra armor".to_string()
    }

    fn is_keyword(&self) -> bool {
        true
    }

    fn generate_replacement_effect(
        &self,
        source: ObjectId,
        controller: PlayerId,
    ) -> Option<ReplacementEffect> {
        Some(ReplacementEffect::with_matcher(
            source,
            controller,
            AttachedPermanentWouldBeDestroyedMatcher::new(source),
            ReplacementAction::Instead(vec![Effect::new(crate::effects::UmbraArmorEffect::new(
                source,
            ))]),
        ))
    }
}

/// Permanents matching a filter enter the battlefield tapped.
#[derive(Debug, Clone, PartialEq)]
pub struct EnterTappedForFilter {
    pub filter: ObjectFilter,
}

impl EnterTappedForFilter {
    pub fn new(filter: ObjectFilter) -> Self {
        Self { filter }
    }
}

impl StaticAbilityKind for EnterTappedForFilter {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::EnterTappedForFilter
    }

    fn display(&self) -> String {
        let filter = &self.filter;
        let is_simple_type_list = !filter.card_types.is_empty()
            && filter.all_card_types.is_empty()
            && filter.subtypes.is_empty()
            && filter.supertypes.is_empty()
            && filter.colors.is_none()
            && filter.excluded_card_types.is_empty()
            && filter.excluded_subtypes.is_empty()
            && filter.excluded_supertypes.is_empty()
            && filter.excluded_colors.is_empty()
            && !filter.token
            && !filter.nontoken
            && !filter.tapped
            && !filter.untapped
            && !filter.attacking
            && !filter.nonattacking
            && !filter.blocking
            && !filter.nonblocking
            && filter.controller.is_none()
            && filter.owner.is_none()
            && matches!(filter.zone, None | Some(Zone::Battlefield))
            && filter.tagged_constraints.is_empty()
            && filter.targets_object.is_none()
            && filter.targets_player.is_none()
            && filter.ability_markers.is_empty()
            && filter.excluded_ability_markers.is_empty()
            && !filter.noncommander;

        let has_all_permanent_types = {
            let required = [
                crate::types::CardType::Artifact,
                crate::types::CardType::Creature,
                crate::types::CardType::Enchantment,
                crate::types::CardType::Land,
                crate::types::CardType::Planeswalker,
                crate::types::CardType::Battle,
            ];
            filter.card_types.len() == required.len()
                && required
                    .iter()
                    .all(|card_type| filter.card_types.contains(card_type))
        };

        if is_simple_type_list && has_all_permanent_types {
            return "Permanents enter tapped".to_string();
        }

        if is_simple_type_list && filter.card_types.len() >= 2 {
            let words = filter
                .card_types
                .iter()
                .map(|card_type| pluralize(card_type_word(*card_type)))
                .collect::<Vec<_>>();
            let list = join_with_and(&words);
            return format!("{} enter tapped", capitalize_first(&list));
        }

        format!("{} enter the battlefield tapped", self.filter.description())
    }

    fn generate_replacement_effect(
        &self,
        source: ObjectId,
        controller: PlayerId,
    ) -> Option<ReplacementEffect> {
        Some(ReplacementEffect::with_matcher(
            source,
            controller,
            WouldEnterBattlefieldMatcher::new(self.filter.clone()),
            ReplacementAction::EnterTapped,
        ))
    }
}

/// Permanents matching a filter enter the battlefield untapped.
#[derive(Debug, Clone, PartialEq)]
pub struct EnterUntappedForFilter {
    pub filter: ObjectFilter,
}

impl EnterUntappedForFilter {
    pub fn new(filter: ObjectFilter) -> Self {
        Self { filter }
    }
}

impl StaticAbilityKind for EnterUntappedForFilter {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::EnterUntappedForFilter
    }

    fn display(&self) -> String {
        let filter = &self.filter;
        let is_simple_lands_you_control = filter.card_types == vec![crate::types::CardType::Land]
            && filter.controller == Some(PlayerFilter::You)
            && matches!(filter.zone, None | Some(Zone::Battlefield))
            && filter.subtypes.is_empty()
            && filter.supertypes.is_empty()
            && filter.colors.is_none()
            && filter.excluded_card_types.is_empty()
            && filter.excluded_subtypes.is_empty()
            && filter.excluded_supertypes.is_empty()
            && filter.excluded_colors.is_empty()
            && !filter.token
            && !filter.nontoken
            && !filter.tapped
            && !filter.untapped
            && !filter.attacking
            && !filter.nonattacking
            && !filter.blocking
            && !filter.nonblocking
            && filter.owner.is_none()
            && filter.tagged_constraints.is_empty()
            && filter.targets_object.is_none()
            && filter.targets_player.is_none()
            && filter.ability_markers.is_empty()
            && filter.excluded_ability_markers.is_empty()
            && !filter.noncommander;
        if is_simple_lands_you_control {
            return "Lands you control enter untapped".to_string();
        }

        format!(
            "{} enter untapped",
            capitalize_first(&self.filter.description())
        )
    }

    fn generate_replacement_effect(
        &self,
        source: ObjectId,
        controller: PlayerId,
    ) -> Option<ReplacementEffect> {
        Some(ReplacementEffect::with_matcher(
            source,
            controller,
            WouldEnterBattlefieldMatcher::new(self.filter.clone()),
            ReplacementAction::EnterUntapped,
        ))
    }
}

/// Permanents matching a filter enter the battlefield with counters.
#[derive(Debug, Clone, PartialEq)]
pub struct EnterWithCountersForFilter {
    pub filter: ObjectFilter,
    pub counter_type: CounterType,
    pub count: Value,
    pub added_subtypes: Vec<Subtype>,
    pub condition: Option<crate::ConditionExpr>,
}

impl EnterWithCountersForFilter {
    pub fn new(filter: ObjectFilter, counter_type: CounterType, count: Value) -> Self {
        Self {
            filter,
            counter_type,
            count,
            added_subtypes: Vec::new(),
            condition: None,
        }
    }

    pub fn with_added_subtypes(mut self, subtypes: Vec<Subtype>) -> Self {
        self.added_subtypes = subtypes;
        self
    }

    pub fn with_condition(mut self, condition: crate::ConditionExpr) -> Self {
        self.condition = Some(condition);
        self
    }
}

fn spell_cast_snow_mana_enters_with_counter_display(
    ability: &EnterWithCountersForFilter,
) -> Option<String> {
    if !matches!(
        ability.condition,
        Some(Condition::SnowManaOfAnySpellColorSpentToCastThisSpell)
    ) || ability.counter_type != CounterType::PlusOnePlusOne
        || ability.count != Value::Fixed(1)
        || !ability.added_subtypes.is_empty()
    {
        return None;
    }

    let mut expected_filter = ObjectFilter::default();
    expected_filter.zone = Some(Zone::Battlefield);
    expected_filter.controller = Some(PlayerFilter::You);
    expected_filter.card_types = vec![CardType::Creature];
    if ability.filter != expected_filter {
        return None;
    }

    Some(
        "Whenever you cast a creature spell, if {S} of any of that spell's colors was spent to cast it, that creature enters with an additional +1/+1 counter on it"
            .to_string(),
    )
}

impl StaticAbilityKind for EnterWithCountersForFilter {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::EnterWithCountersForFilter
    }

    fn display(&self) -> String {
        if let Some(text) = spell_cast_snow_mana_enters_with_counter_display(self) {
            return text;
        }

        let mut subject = self.filter.description();
        if subject.starts_with("another ") {
            subject = subject.replacen("another ", "Each other ", 1);
        } else {
            subject = capitalize_first(&subject);
        }
        let enters = if matches!(
            subject.split_whitespace().next(),
            Some("A" | "An" | "This" | "That" | "Each")
        ) {
            "enters"
        } else {
            "enter"
        };

        let counter = self.counter_type.description().into_owned();
        let counter_clause = match &self.count {
            Value::Fixed(1) => {
                format!("with an additional {counter} counter on it")
            }
            Value::Fixed(v) => {
                let rendered = u32::try_from(*v)
                    .ok()
                    .and_then(number_word_u32)
                    .unwrap_or_else(|| v.to_string());
                format!("with {rendered} additional {counter} counters on it")
            }
            value => format!(
                "with a number of additional {counter} counters on it equal to {}",
                describe_value(value)
            ),
        };

        let subtype_clause = if self.added_subtypes.is_empty() {
            String::new()
        } else {
            let subtype_words = self
                .added_subtypes
                .iter()
                .map(std::string::ToString::to_string)
                .collect::<Vec<_>>();
            let article = subtype_words
                .first()
                .map(|word| {
                    if matches!(
                        word.chars().next().map(|ch| ch.to_ascii_lowercase()),
                        Some('a' | 'e' | 'i' | 'o' | 'u')
                    ) {
                        "an"
                    } else {
                        "a"
                    }
                })
                .unwrap_or("a");
            format!(
                " and as {article} {} in addition to its other types",
                subtype_words.join(" ")
            )
        };

        let text = format!("{subject} {enters} {counter_clause}{subtype_clause}");
        let Some(condition) = &self.condition else {
            return text;
        };
        let condition = super::describe_static_condition(condition);
        if let Some(rest) = condition.strip_prefix("as long as ") {
            let mut chars = text.chars();
            let lowered = match chars.next() {
                Some(first) => format!("{}{}", first.to_ascii_lowercase(), chars.as_str()),
                None => String::new(),
            };
            return format!("As long as {rest}, {lowered}");
        }
        format!("{text} {condition}")
    }

    fn with_static_condition(&self, condition: crate::ConditionExpr) -> Option<StaticAbility> {
        Some(StaticAbility::new(self.clone().with_condition(condition)))
    }

    fn generate_replacement_effect(
        &self,
        source: ObjectId,
        controller: PlayerId,
    ) -> Option<ReplacementEffect> {
        Some(ReplacementEffect::with_matcher(
            source,
            controller,
            ConditionalWouldEnterBattlefieldMatcher {
                enter_matcher: WouldEnterBattlefieldMatcher::new(self.filter.clone()),
                condition: self.condition.clone(),
            },
            ReplacementAction::EnterWithCounters {
                counter_type: self.counter_type,
                count: self.count.clone(),
                added_subtypes: self.added_subtypes.clone(),
                added_abilities: Vec::new(),
            },
        ))
    }
}

#[derive(Debug, Clone)]
struct ConditionalWouldEnterBattlefieldMatcher {
    enter_matcher: WouldEnterBattlefieldMatcher,
    condition: Option<crate::ConditionExpr>,
}

impl ConditionalWouldEnterBattlefieldMatcher {
    fn condition_matches(
        &self,
        event: &dyn GameEventType,
        ctx: &crate::events::context::EventContext<'_>,
    ) -> bool {
        let Some(condition) = &self.condition else {
            return true;
        };
        let Some(source) = condition_source_for_enter_with_counter_filter(condition, event, ctx)
        else {
            return false;
        };
        let eval_ctx = crate::condition_eval::ExternalEvaluationContext {
            controller: ctx.controller,
            source,
            defending_player: None,
            attacking_player: None,
            filter_source: Some(source),
            iterated_player: None,
            triggering_event: None,
            trigger_identity: None,
            ability_index: None,
            options: Default::default(),
        };
        crate::condition_eval::evaluate_condition_external(ctx.game, condition, &eval_ctx)
    }
}

impl ReplacementMatcher for ConditionalWouldEnterBattlefieldMatcher {
    fn matches_event(&self, event: &dyn GameEventType, ctx: &EventContext) -> bool {
        self.enter_matcher.matches_event(event, ctx) && self.condition_matches(event, ctx)
    }

    fn priority(&self) -> ReplacementPriority {
        self.enter_matcher.priority()
    }

    fn display(&self) -> String {
        self.enter_matcher.display()
    }
}

fn enter_with_counter_filter_condition_uses_entering_object(condition: &Condition) -> bool {
    match condition {
        Condition::ManaSpentToCastThisSpellAtLeast { .. }
        | Condition::ColoredManaSpentToCastThisSpellAtLeast(_)
        | Condition::SnowManaOfAnySpellColorSpentToCastThisSpell
        | Condition::SameColorManaSpentToCastThisSpellAtLeast(_)
        | Condition::ColorsOfManaSpentToCastThisSpellOrMore(_) => true,
        Condition::Not(inner) => enter_with_counter_filter_condition_uses_entering_object(inner),
        Condition::And(left, right) | Condition::Or(left, right) => {
            enter_with_counter_filter_condition_uses_entering_object(left)
                || enter_with_counter_filter_condition_uses_entering_object(right)
        }
        _ => false,
    }
}

fn entering_object_from_event(event: &dyn GameEventType) -> Option<ObjectId> {
    match event.event_kind() {
        EventKind::ZoneChange => {
            let zone_change = downcast_event::<ZoneChangeEvent>(event)?;
            (zone_change.to == crate::zone::Zone::Battlefield)
                .then(|| zone_change.objects.first().copied())
                .flatten()
        }
        EventKind::EnterBattlefield => {
            downcast_event::<EnterBattlefieldEvent>(event).map(|enter_event| enter_event.object)
        }
        _ => None,
    }
}

fn condition_source_for_enter_with_counter_filter(
    condition: &Condition,
    event: &dyn GameEventType,
    ctx: &crate::events::context::EventContext<'_>,
) -> Option<ObjectId> {
    if enter_with_counter_filter_condition_uses_entering_object(condition) {
        entering_object_from_event(event)
    } else {
        ctx.source
    }
}

/// Permanents matching a filter enter with permanent characteristic changes.
#[derive(Debug, Clone, PartialEq)]
pub struct EnterWithCharacteristicsForFilter {
    pub filter: ObjectFilter,
    pub added_card_types: Vec<CardType>,
    pub added_subtypes: Vec<Subtype>,
    pub power: i32,
    pub toughness: i32,
}

impl EnterWithCharacteristicsForFilter {
    pub fn new(
        filter: ObjectFilter,
        added_card_types: Vec<CardType>,
        added_subtypes: Vec<Subtype>,
        power: i32,
        toughness: i32,
    ) -> Self {
        Self {
            filter,
            added_card_types,
            added_subtypes,
            power,
            toughness,
        }
    }
}

impl StaticAbilityKind for EnterWithCharacteristicsForFilter {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::EnterWithCharacteristicsForFilter
    }

    fn display(&self) -> String {
        let subject = self.filter.description();
        let plural = !subject.starts_with("a ")
            && !subject.starts_with("an ")
            && !subject.starts_with("this ")
            && !subject.starts_with("that ");
        let (enter_verb, subject_pronoun, become_verb, article, possessive_pronoun) = if plural {
            ("enter", "they", "become", "", "their")
        } else {
            ("enters", "it", "becomes", "a ", "its")
        };

        let mut type_words = self
            .added_subtypes
            .iter()
            .map(std::string::ToString::to_string)
            .collect::<Vec<_>>();
        let card_type_count = self.added_card_types.len();
        for card_type in &self.added_card_types {
            let mut word = card_type_word(*card_type).to_string();
            if plural && card_type_count == 1 {
                word = pluralize(&word);
            }
            type_words.push(word);
        }
        let type_phrase = type_words.join(" ");

        format!(
            "As {subject} {enter_verb}, {subject_pronoun} {become_verb} {article}{}/{} {type_phrase} in addition to {possessive_pronoun} other types",
            self.power, self.toughness
        )
    }

    fn generate_replacement_effect(
        &self,
        source: ObjectId,
        controller: PlayerId,
    ) -> Option<ReplacementEffect> {
        Some(ReplacementEffect::with_matcher(
            source,
            controller,
            WouldEnterBattlefieldMatcher::new(self.filter.clone()),
            ReplacementAction::EnterWithCharacteristics {
                added_card_types: self.added_card_types.clone(),
                added_subtypes: self.added_subtypes.clone(),
                set_base_power_toughness: Some((self.power, self.toughness)),
            },
        ))
    }
}

/// Players can't cycle cards.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PlayersCantCycle;

impl StaticAbilityKind for PlayersCantCycle {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::PlayersCantCycle
    }

    fn display(&self) -> String {
        "Players can't cycle cards".to_string()
    }
}

/// Start the game with an additional amount of life.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StartingLifeBonus {
    pub amount: i32,
}

impl StartingLifeBonus {
    pub fn new(amount: i32) -> Self {
        Self { amount }
    }
}

impl StaticAbilityKind for StartingLifeBonus {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::StartingLifeBonus
    }

    fn display(&self) -> String {
        format!("You start the game with an additional {} life", self.amount)
    }
}

/// Buyback costs cost less (placeholder ability).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BuybackCostReduction {
    pub amount: u32,
}

impl BuybackCostReduction {
    pub fn new(amount: u32) -> Self {
        Self { amount }
    }
}

impl StaticAbilityKind for BuybackCostReduction {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::BuybackCostReduction
    }

    fn display(&self) -> String {
        format!("Buyback costs cost {{{}}} less", self.amount)
    }
}

/// Players skip their upkeep steps.
#[derive(Debug, Clone, PartialEq)]
pub struct PlayersSkipUpkeep {
    pub player: PlayerFilter,
    pub condition: Option<crate::ConditionExpr>,
}

impl PlayersSkipUpkeep {
    pub fn new(player: PlayerFilter) -> Self {
        Self {
            player,
            condition: None,
        }
    }

    pub fn with_condition(mut self, condition: crate::ConditionExpr) -> Self {
        self.condition = Some(condition);
        self
    }

    fn condition_matches(&self, game: &GameState, source: ObjectId, controller: PlayerId) -> bool {
        self.condition.as_ref().is_none_or(|condition| {
            super::static_condition_is_active(condition, game, source, controller)
        })
    }
}

impl StaticAbilityKind for PlayersSkipUpkeep {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::PlayersSkipUpkeep
    }

    fn display(&self) -> String {
        let base = match self.player {
            PlayerFilter::You => "Skip your upkeep step".to_string(),
            PlayerFilter::Opponent => "Each opponent skips their upkeep step".to_string(),
            PlayerFilter::Any => "Players skip their upkeep steps".to_string(),
            _ => "Matching players skip their upkeep steps".to_string(),
        };
        if let Some(condition) = &self.condition {
            if matches!(self.player, PlayerFilter::You)
                && let crate::ConditionExpr::PlayerCardsInHandOrFewer {
                    player: PlayerFilter::You,
                    count: 0,
                } = condition
            {
                return "Hellbent — Skip your upkeep step if you have no cards in hand".to_string();
            }
            let condition = super::describe_static_condition(condition);
            format!("{base} {condition}")
        } else {
            base
        }
    }

    fn with_static_condition(&self, condition: crate::ConditionExpr) -> Option<StaticAbility> {
        Some(StaticAbility::new(self.clone().with_condition(condition)))
    }

    fn skips_upkeep_for_player(
        &self,
        game: &GameState,
        source: ObjectId,
        controller: PlayerId,
        player: PlayerId,
    ) -> bool {
        self.condition_matches(game, source, controller)
            && self
                .player
                .matches_player(player, &game.filter_context_for(controller, Some(source)))
    }
}

/// The legend rule doesn't apply.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LegendRuleDoesntApply;

impl StaticAbilityKind for LegendRuleDoesntApply {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::LegendRuleDoesntApply
    }

    fn display(&self) -> String {
        "The legend rule doesn't apply".to_string()
    }
}

/// Creatures entering don't cause abilities to trigger.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CreaturesEnteringDontCauseAbilitiesToTrigger;

impl StaticAbilityKind for CreaturesEnteringDontCauseAbilitiesToTrigger {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::CreaturesEnteringDontCauseAbilitiesToTrigger
    }

    fn display(&self) -> String {
        "Creatures entering don't cause abilities to trigger.".to_string()
    }

    fn trigger_suppression_spec(&self) -> Option<TriggerSuppressionSpec> {
        Some(TriggerSuppressionSpec {
            source_filter: None,
            event_matcher: Some(crate::triggers::Trigger::enters_battlefield(
                ObjectFilter::creature(),
                None,
            )),
        })
    }
}

/// "If a triggered ability of another creature you control of the chosen type triggers,
/// it triggers an additional time."
#[derive(Debug, Clone, PartialEq)]
pub struct DuplicateMatchingTriggeredAbilities {
    pub source_filter: Option<ObjectFilter>,
    pub event_matcher: Option<crate::triggers::Trigger>,
    pub copies: usize,
    pub display: String,
}

impl DuplicateMatchingTriggeredAbilities {
    pub fn new(
        source_filter: Option<ObjectFilter>,
        event_matcher: Option<crate::triggers::Trigger>,
        copies: usize,
        display: String,
    ) -> Self {
        Self {
            source_filter,
            event_matcher,
            copies,
            display,
        }
    }
}

impl StaticAbilityKind for DuplicateMatchingTriggeredAbilities {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::DuplicateMatchingTriggeredAbilities
    }

    fn display(&self) -> String {
        self.display.clone()
    }

    fn trigger_duplication_spec(&self) -> Option<TriggerDuplicationSpec> {
        Some(TriggerDuplicationSpec {
            source_filter: self.source_filter.clone(),
            event_matcher: self.event_matcher.clone(),
            source_matcher: TriggerDuplicationSourceMatcher::ObjectAbility,
            copies: self.copies,
        })
    }
}

/// "Room abilities of dungeons you own trigger an additional time."
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DungeonRoomTriggerDuplication {
    pub display: String,
}

impl DungeonRoomTriggerDuplication {
    pub fn new(display: impl Into<String>) -> Self {
        Self {
            display: display.into(),
        }
    }
}

impl StaticAbilityKind for DungeonRoomTriggerDuplication {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::DungeonRoomTriggerDuplication
    }

    fn display(&self) -> String {
        self.display.clone()
    }

    fn trigger_duplication_spec(&self) -> Option<TriggerDuplicationSpec> {
        Some(TriggerDuplicationSpec {
            source_filter: None,
            event_matcher: None,
            source_matcher:
                TriggerDuplicationSourceMatcher::DungeonRoomAbilityOwnedByStaticController,
            copies: 1,
        })
    }
}

/// "If [matching event] would cause [matching source] to trigger, it doesn't."
#[derive(Debug, Clone, PartialEq)]
pub struct SuppressMatchingTriggeredAbilities {
    pub source_filter: Option<ObjectFilter>,
    pub event_matcher: Option<crate::triggers::Trigger>,
    pub display: String,
}

impl SuppressMatchingTriggeredAbilities {
    pub fn new(
        source_filter: Option<ObjectFilter>,
        event_matcher: Option<crate::triggers::Trigger>,
        display: String,
    ) -> Self {
        Self {
            source_filter,
            event_matcher,
            display,
        }
    }
}

impl StaticAbilityKind for SuppressMatchingTriggeredAbilities {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::SuppressMatchingTriggeredAbilities
    }

    fn display(&self) -> String {
        self.display.clone()
    }

    fn trigger_suppression_spec(&self) -> Option<TriggerSuppressionSpec> {
        Some(TriggerSuppressionSpec {
            source_filter: self.source_filter.clone(),
            event_matcher: self.event_matcher.clone(),
        })
    }
}
