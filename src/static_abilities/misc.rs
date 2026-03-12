//! Miscellaneous static abilities.
//!
//! This module contains static abilities that don't fit neatly into other categories.

use super::{
    ChooseBasicLandTypeAsEntersSpec, ChooseColorAsEntersSpec, ChooseCreatureTypeAsEntersSpec,
    ConditionalSpellKeywordKind, ConditionalSpellKeywordSpec, EnterAsCopyAsEntersSpec,
    GraveyardCountMetric, StaticAbilityId, StaticAbilityKind, ThisSpellCastRestrictionKind,
    text_utils::{capitalize_first, join_with_and, number_word_u32},
};
use crate::ability::LevelAbility;
use crate::color::Color;
use crate::compiled_text::describe_value;
use crate::effect::{Condition, Effect, Value};
use crate::events::cards::matchers::{WouldDiscardMatcher, WouldDrawCardMatcher};
use crate::events::damage::matchers::{
    DamageFromSelfMatcher, DamageToObjectMatcher, DamageToPlayerOrObjectMatcher,
    DamageToSelfCombatMatcher, DamageToSelfFromSourceFilterMatcher,
};
use crate::events::permanents::matchers::AttachedPermanentWouldBeDestroyedMatcher;
use crate::events::traits::{EventKind, ReplacementMatcher, ReplacementPriority, downcast_event};
use crate::events::zones::matchers::{
    ThisWouldEnterBattlefieldMatcher, ThisWouldGoToGraveyardMatcher, WouldEnterBattlefieldMatcher,
};
use crate::events::zones::{EnterBattlefieldEvent, ZoneChangeEvent};
use crate::game_state::GameState;
use crate::grant::GrantSpec;
use crate::ids::{ObjectId, PlayerId};
use crate::mana::ManaCost;
use crate::object::CounterType;
use crate::replacement::{RedirectTarget, RedirectWhich, ReplacementAction, ReplacementEffect};
use crate::tag::SOURCE_EXILED_TAG;
use crate::target::{ChooseSpec, ObjectFilter, PlayerFilter};
use crate::types::Subtype;
use crate::zone::Zone;

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
    }
}

/// Morph keyword ability (turn face up by paying morph cost as a special action).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Morph {
    pub cost: ManaCost,
}

impl Morph {
    pub fn new(cost: ManaCost) -> Self {
        Self { cost }
    }
}

impl StaticAbilityKind for Morph {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::Morph
    }

    fn display(&self) -> String {
        format!("Morph {}", self.cost.to_oracle())
    }

    fn is_keyword(&self) -> bool {
        true
    }

    fn turn_face_up_cost(&self) -> Option<&ManaCost> {
        Some(&self.cost)
    }
}

/// Megamorph keyword ability (turn face up by paying megamorph cost as a special action).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Megamorph {
    pub cost: ManaCost,
}

impl Megamorph {
    pub fn new(cost: ManaCost) -> Self {
        Self { cost }
    }
}

impl StaticAbilityKind for Megamorph {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::Megamorph
    }

    fn display(&self) -> String {
        format!("Megamorph {}", self.cost.to_oracle())
    }

    fn is_keyword(&self) -> bool {
        true
    }

    fn turn_face_up_cost(&self) -> Option<&ManaCost> {
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
        Some(
            ReplacementEffect::with_matcher(
                source,
                controller,
                ThisWouldEnterBattlefieldMatcher,
                ReplacementAction::EnterTapped,
            )
            .self_replacing(),
        )
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
        Some(
            ReplacementEffect::with_matcher(
                source,
                controller,
                ThisWouldEnterTappedUnlessControlTwoOrMoreOtherLandsMatcher,
                ReplacementAction::EnterTapped,
            )
            .self_replacing(),
        )
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
        Some(
            ReplacementEffect::with_matcher(
                source,
                controller,
                ThisWouldEnterTappedUnlessControlTwoOrFewerOtherLandsMatcher,
                ReplacementAction::EnterTapped,
            )
            .self_replacing(),
        )
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
        Some(
            ReplacementEffect::with_matcher(
                source,
                controller,
                ThisWouldEnterTappedUnlessControlTwoOrMoreBasicLandsMatcher,
                ReplacementAction::EnterTapped,
            )
            .self_replacing(),
        )
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
        Some(
            ReplacementEffect::with_matcher(
                source,
                controller,
                ThisWouldEnterTappedUnlessAPlayerHas13OrLessLifeMatcher,
                ReplacementAction::EnterTapped,
            )
            .self_replacing(),
        )
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
        Some(
            ReplacementEffect::with_matcher(
                source,
                controller,
                ThisWouldEnterTappedUnlessTwoOrMoreOpponentsMatcher,
                ReplacementAction::EnterTapped,
            )
            .self_replacing(),
        )
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
            .filter(|obj| obj.controller == ctx.controller && obj.is_land())
            .count();
        land_count < 2
    }

    fn priority(&self) -> ReplacementPriority {
        ReplacementPriority::SelfReplacement
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
            .filter(|obj| obj.controller == ctx.controller && obj.is_land())
            .count();
        land_count > 2
    }

    fn priority(&self) -> ReplacementPriority {
        ReplacementPriority::SelfReplacement
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
                obj.controller == ctx.controller
                    && obj.is_land()
                    && obj.supertypes.contains(&crate::types::Supertype::Basic)
            })
            .count();
        basic_land_count < 2
    }

    fn priority(&self) -> ReplacementPriority {
        ReplacementPriority::SelfReplacement
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
        ReplacementPriority::SelfReplacement
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
        ReplacementPriority::SelfReplacement
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
        Some(
            ReplacementEffect::with_matcher(
                source,
                controller,
                ThisWouldEnterTappedUnlessConditionMatcher {
                    condition: self.condition.clone(),
                    display: self.display.clone(),
                },
                ReplacementAction::EnterTapped,
            )
            .self_replacing(),
        )
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
            triggering_event: None,
            trigger_identity: None,
            ability_index: None,
            options: Default::default(),
        };

        // Replacement applies when the "unless" condition is false.
        !crate::condition_eval::evaluate_condition_external(ctx.game, &self.condition, &eval_ctx)
    }

    fn priority(&self) -> ReplacementPriority {
        ReplacementPriority::SelfReplacement
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
        Some(
            ReplacementEffect::with_matcher(
                source,
                controller,
                ThisWouldEnterWithBloodthirstMatcher,
                ReplacementAction::EnterWithCounters {
                    counter_type: CounterType::PlusOnePlusOne,
                    count: Value::Fixed(self.amount as i32),
                    added_subtypes: Vec::new(),
                },
            )
            .self_replacing(),
        )
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
                    .damage_to_players_this_turn
                    .get(&player.id)
                    .copied()
                    .unwrap_or(0)
                    > 0
        })
    }

    fn priority(&self) -> ReplacementPriority {
        ReplacementPriority::SelfReplacement
    }

    fn display(&self) -> String {
        "When this would enter with bloodthirst counters".to_string()
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
                        .map(str::to_string)
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
            Value::Count(filter) => {
                format!(
                    "Enters the battlefield with X {counter} counters on it, where X is the number of {}",
                    filter.description()
                )
            }
            Value::CountScaled(filter, scale) => {
                if *scale == 1 {
                    format!(
                        "Enters the battlefield with X {counter} counters on it, where X is the number of {}",
                        filter.description()
                    )
                } else {
                    format!(
                        "Enters the battlefield with X {counter} counters on it, where X is {} times the number of {}",
                        scale,
                        filter.description()
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
        Some(
            ReplacementEffect::with_matcher(
                source,
                controller,
                ThisWouldEnterBattlefieldMatcher,
                ReplacementAction::EnterWithCounters {
                    counter_type: self.counter_type,
                    count: self.count.clone(),
                    added_subtypes: Vec::new(),
                },
            )
            .self_replacing(),
        )
    }
}

/// Enters the battlefield with counters if a condition is true.
#[derive(Debug, Clone, PartialEq)]
pub struct EntersWithCountersIfCondition {
    pub counter_type: CounterType,
    pub count: Value,
    pub condition: Condition,
    pub condition_display: String,
}

impl EntersWithCountersIfCondition {
    pub fn new(
        counter_type: CounterType,
        count: Value,
        condition: Condition,
        condition_display: String,
    ) -> Self {
        Self {
            counter_type,
            count,
            condition,
            condition_display: condition_display.trim().to_string(),
        }
    }
}

impl StaticAbilityKind for EntersWithCountersIfCondition {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::EnterWithCountersIfCondition
    }

    fn display(&self) -> String {
        let base = EntersWithCounters::new(self.counter_type, self.count.clone()).display();
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
        Some(
            ReplacementEffect::with_matcher(
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
                },
            )
            .self_replacing(),
        )
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
            triggering_event: None,
            trigger_identity: None,
            ability_index: None,
            options: Default::default(),
        };

        crate::condition_eval::evaluate_condition_external(ctx.game, &self.condition, &eval_ctx)
    }

    fn priority(&self) -> ReplacementPriority {
        ReplacementPriority::SelfReplacement
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
            ReplacementEffect::with_matcher(
                source,
                controller,
                ThisWouldGoToGraveyardMatcher,
                ReplacementAction::ChangeDestination(crate::zone::Zone::Library),
            )
            .self_replacing(),
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

/// Players may spend mana as though it were mana of any color.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SpendManaAsAnyColor;

impl StaticAbilityKind for SpendManaAsAnyColor {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::SpendManaAsAnyColor
    }

    fn display(&self) -> String {
        "Players may spend mana as though it were mana of any color".to_string()
    }

    fn apply_restrictions(&self, game: &mut GameState, _source: ObjectId, _controller: PlayerId) {
        for player in &game.players {
            if player.is_in_game() {
                game.mana_spend_effects.any_color_players.insert(player.id);
            }
        }
    }
}

/// You may spend mana as though it were mana of any color to pay activation costs of this.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SpendManaAsAnyColorForSourceActivation;

impl StaticAbilityKind for SpendManaAsAnyColorForSourceActivation {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::SpendManaAsAnyColorActivationCosts
    }

    fn display(&self) -> String {
        "You may spend mana as though it were mana of any color to pay activation costs of this"
            .to_string()
    }

    fn apply_restrictions(&self, game: &mut GameState, source: ObjectId, _controller: PlayerId) {
        game.mana_spend_effects
            .any_color_activation_sources
            .insert(source);
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

/// "If damage would be dealt to this creature, prevent that damage.
/// Remove N <counter> counter(s) from this creature."
#[derive(Debug, Clone, PartialEq)]
pub struct PreventDamageToSelfRemoveCounter {
    pub counter_type: CounterType,
    pub amount: u32,
}

impl PreventDamageToSelfRemoveCounter {
    pub const fn new(counter_type: CounterType, amount: u32) -> Self {
        Self {
            counter_type,
            amount,
        }
    }
}

impl StaticAbilityKind for PreventDamageToSelfRemoveCounter {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::PreventDamageToSelfRemoveCounter
    }

    fn display(&self) -> String {
        let counter = self.counter_type.description().into_owned();
        let amount_word = number_word_u32(self.amount)
            .map(str::to_string)
            .unwrap_or_else(|| self.amount.to_string());
        let suffix = if self.amount == 1 { "" } else { "s" };
        format!(
            "If damage would be dealt to this creature, prevent that damage. Remove {amount_word} {counter} counter{suffix} from this creature."
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
            crate::events::DamageToSelfMatcher::new(),
            ReplacementAction::Instead(vec![Effect::remove_counters(
                self.counter_type,
                Value::Fixed(self.amount as i32),
                ChooseSpec::Source,
            )]),
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
}

impl EnterWithCountersForFilter {
    pub fn new(filter: ObjectFilter, counter_type: CounterType, count: Value) -> Self {
        Self {
            filter,
            counter_type,
            count,
            added_subtypes: Vec::new(),
        }
    }

    pub fn with_added_subtypes(mut self, subtypes: Vec<Subtype>) -> Self {
        self.added_subtypes = subtypes;
        self
    }
}

impl StaticAbilityKind for EnterWithCountersForFilter {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::EnterWithCountersForFilter
    }

    fn display(&self) -> String {
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
                    .map(str::to_string)
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

        format!("{subject} {enters} {counter_clause}{subtype_clause}")
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
            ReplacementAction::EnterWithCounters {
                counter_type: self.counter_type,
                count: self.count.clone(),
                added_subtypes: self.added_subtypes.clone(),
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PlayersSkipUpkeep;

impl StaticAbilityKind for PlayersSkipUpkeep {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::PlayersSkipUpkeep
    }

    fn display(&self) -> String {
        "Players skip their upkeep steps".to_string()
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

/// You may play an additional land on each of your turns.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AdditionalLandPlay;

impl StaticAbilityKind for AdditionalLandPlay {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::AdditionalLandPlay
    }

    fn display(&self) -> String {
        "You may play an additional land on each of your turns".to_string()
    }

    fn apply_restrictions(&self, game: &mut GameState, _source: ObjectId, controller: PlayerId) {
        if let Some(player) = game.player_mut(controller) {
            player.land_plays_per_turn = player.land_plays_per_turn.saturating_add(1);
        }
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
}

/// Can be your commander.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CanBeCommander;

impl StaticAbilityKind for CanBeCommander {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::CanBeCommander
    }

    fn display(&self) -> String {
        "Can be your commander".to_string()
    }
}

// =============================================================================
// Unified Grant System
// =============================================================================

/// Unified grant ability that grants abilities or alternative casting methods
/// to cards matching a filter in a specific zone.
///
/// This is the generic version that replaces bespoke types like `GrantEscape`
/// and `GrantFlashToNoncreatureSpells`. It provides a uniform way to express
/// "cards matching X in zone Y have Z".
///
/// # Examples
///
/// ```ignore
/// // Valley Floodcaller: "You may cast noncreature spells as though they had flash."
/// StaticAbility::grants(GrantSpec::flash_to_noncreature_spells())
///
/// // Underworld Breach: "Each nonland card in your graveyard has escape."
/// StaticAbility::grants(GrantSpec::escape_to_nonland(3))
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Grants {
    pub spec: GrantSpec,
}

impl Grants {
    /// Create a new Grants ability from a grant specification.
    pub fn new(spec: GrantSpec) -> Self {
        Self { spec }
    }
}

impl StaticAbilityKind for Grants {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::Grants
    }

    fn display(&self) -> String {
        self.spec.display()
    }

    fn grant_spec(&self) -> Option<GrantSpec> {
        Some(self.spec.clone())
    }
}

/// Level abilities for level-up creatures.
#[derive(Debug, Clone, PartialEq)]
pub struct LevelAbilities {
    pub levels: Vec<LevelAbility>,
}

impl LevelAbilities {
    pub fn new(levels: Vec<LevelAbility>) -> Self {
        Self { levels }
    }
}

impl StaticAbilityKind for LevelAbilities {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::LevelAbilities
    }

    fn display(&self) -> String {
        if self.levels.is_empty() {
            return "Level up abilities".to_string();
        }

        let rendered_levels = self
            .levels
            .iter()
            .map(|level| {
                let range = match level.max_level {
                    Some(max) if max == level.min_level => format!("Level {}", level.min_level),
                    Some(max) => format!("Level {}-{}", level.min_level, max),
                    None => format!("Level {}+", level.min_level),
                };
                let mut details = Vec::new();
                if let Some((power, toughness)) = level.power_toughness {
                    details.push(format!("{power}/{toughness}"));
                }
                details.extend(level.abilities.iter().map(|ability| ability.display()));
                if details.is_empty() {
                    range
                } else {
                    format!("{range}: {}", details.join(", "))
                }
            })
            .collect::<Vec<_>>()
            .join("; ");

        format!("Level up abilities ({rendered_levels})")
    }

    fn level_abilities(&self) -> Option<&[LevelAbility]> {
        Some(&self.levels)
    }
}

/// "You have no maximum hand size"
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct NoMaximumHandSize;

impl StaticAbilityKind for NoMaximumHandSize {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::NoMaximumHandSize
    }

    fn display(&self) -> String {
        "You have no maximum hand size".to_string()
    }

    fn apply_restrictions(&self, game: &mut GameState, _source: ObjectId, controller: PlayerId) {
        if let Some(player) = game.player_mut(controller) {
            player.max_hand_size = i32::MAX;
        }
    }
}

/// "Your/Each opponent's maximum hand size is reduced by N."
#[derive(Debug, Clone, PartialEq)]
pub struct ReduceMaximumHandSize {
    pub player: PlayerFilter,
    pub amount: u32,
}

impl ReduceMaximumHandSize {
    pub fn new(player: PlayerFilter, amount: u32) -> Self {
        Self { player, amount }
    }
}

impl StaticAbilityKind for ReduceMaximumHandSize {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::ReduceMaximumHandSize
    }

    fn display(&self) -> String {
        match self.player {
            PlayerFilter::You => {
                format!("Your maximum hand size is reduced by {}.", self.amount)
            }
            PlayerFilter::Opponent => {
                format!(
                    "Each opponent's maximum hand size is reduced by {}.",
                    self.amount
                )
            }
            PlayerFilter::Any => {
                format!(
                    "Each player's maximum hand size is reduced by {}.",
                    self.amount
                )
            }
            _ => format!("Maximum hand size is reduced by {}.", self.amount),
        }
    }

    fn apply_restrictions(&self, game: &mut GameState, _source: ObjectId, controller: PlayerId) {
        use crate::game_loop::player_matches_filter_with_combat;

        let combat = game.combat.as_ref();
        let affected: Vec<PlayerId> = game
            .players
            .iter()
            .filter(|player| {
                player.is_in_game()
                    && player_matches_filter_with_combat(
                        player.id,
                        &self.player,
                        game,
                        controller,
                        combat,
                    )
            })
            .map(|player| player.id)
            .collect();

        let reduction = self.amount as i32;
        for player_id in affected {
            if let Some(player) = game.player_mut(player_id) {
                player.max_hand_size = player.max_hand_size.saturating_sub(reduction);
            }
        }
    }
}

fn player_ids_for_filter(
    game: &GameState,
    player_filter: PlayerFilter,
    controller: PlayerId,
) -> Vec<PlayerId> {
    use crate::game_loop::player_matches_filter_with_combat;

    let combat = game.combat.as_ref();
    game.players
        .iter()
        .filter(|player| {
            player.is_in_game()
                && player_matches_filter_with_combat(
                    player.id,
                    &player_filter,
                    game,
                    controller,
                    combat,
                )
        })
        .map(|player| player.id)
        .collect()
}

fn count_distinct_card_types_in_graveyard(game: &GameState, player_id: PlayerId) -> i32 {
    use crate::types::CardType;

    let mut types: Vec<CardType> = Vec::new();
    let Some(player) = game.player(player_id) else {
        return 0;
    };
    for &card_id in &player.graveyard {
        let Some(obj) = game.object(card_id) else {
            continue;
        };
        for card_type in &obj.card_types {
            if !types.contains(card_type) {
                types.push(*card_type);
            }
        }
    }
    types.len() as i32
}

fn count_distinct_mana_values_in_graveyard(game: &GameState, player_id: PlayerId) -> i32 {
    let Some(player) = game.player(player_id) else {
        return 0;
    };

    let mut values: Vec<u32> = Vec::new();
    for &card_id in &player.graveyard {
        let Some(obj) = game.object(card_id) else {
            continue;
        };
        let mana_value = obj.mana_cost.as_ref().map_or(0, |cost| cost.mana_value());
        if !values.contains(&mana_value) {
            values.push(mana_value);
        }
    }
    values.len() as i32
}

pub(crate) fn conditional_spell_keyword_active(
    spec: ConditionalSpellKeywordSpec,
    game: &GameState,
    controller: PlayerId,
) -> bool {
    let count = match spec.metric {
        GraveyardCountMetric::CardTypes => count_distinct_card_types_in_graveyard(game, controller),
        GraveyardCountMetric::ManaValues => {
            count_distinct_mana_values_in_graveyard(game, controller)
        }
    };
    count >= spec.threshold as i32
}

/// "This spell has flash/cascade as long as there are N or more ... in your graveyard."
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConditionalSpellKeyword {
    pub spec: ConditionalSpellKeywordSpec,
}

impl ConditionalSpellKeyword {
    pub const fn new(spec: ConditionalSpellKeywordSpec) -> Self {
        Self { spec }
    }
}

impl StaticAbilityKind for ConditionalSpellKeyword {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::ConditionalSpellKeyword
    }

    fn display(&self) -> String {
        let keyword = match self.spec.keyword {
            ConditionalSpellKeywordKind::Flash => "flash",
            ConditionalSpellKeywordKind::Cascade => "cascade",
        };
        let metric = match self.spec.metric {
            GraveyardCountMetric::CardTypes => "card types",
            GraveyardCountMetric::ManaValues => "mana values",
        };
        let threshold = number_word_u32(self.spec.threshold)
            .map(str::to_string)
            .unwrap_or_else(|| self.spec.threshold.to_string());
        format!(
            "This spell has {keyword} as long as there are {threshold} or more {metric} among cards in your graveyard."
        )
    }

    fn conditional_spell_keyword_spec(&self) -> Option<ConditionalSpellKeywordSpec> {
        Some(self.spec)
    }
}

/// "Cast this spell only ..." cast-time restriction.
#[derive(Debug, Clone, PartialEq)]
pub struct ThisSpellCastRestriction {
    pub kind: ThisSpellCastRestrictionKind,
    pub display: String,
}

impl ThisSpellCastRestriction {
    pub fn new(kind: ThisSpellCastRestrictionKind, display: impl Into<String>) -> Self {
        Self {
            kind,
            display: display.into(),
        }
    }
}

impl StaticAbilityKind for ThisSpellCastRestriction {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::ThisSpellCastRestriction
    }

    fn display(&self) -> String {
        self.display.clone()
    }

    fn this_spell_cast_restriction_kind(&self) -> Option<ThisSpellCastRestrictionKind> {
        Some(self.kind.clone())
    }
}

/// "Each opponent's maximum hand size is equal to seven minus the number of card types in your graveyard."
#[derive(Debug, Clone, PartialEq)]
pub struct MaximumHandSizeSevenMinusYourGraveyardCardTypes {
    pub player: PlayerFilter,
    pub minimum_types: u32,
}

impl MaximumHandSizeSevenMinusYourGraveyardCardTypes {
    pub const fn new(player: PlayerFilter, minimum_types: u32) -> Self {
        Self {
            player,
            minimum_types,
        }
    }
}

impl StaticAbilityKind for MaximumHandSizeSevenMinusYourGraveyardCardTypes {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::MaximumHandSizeSevenMinusYourGraveyardCardTypes
    }

    fn display(&self) -> String {
        let who = match self.player {
            PlayerFilter::You => "Your",
            PlayerFilter::Opponent => "Each opponent's",
            PlayerFilter::Any => "Each player's",
            _ => "Affected players'",
        };
        format!(
            "As long as there are {} or more card types among cards in your graveyard, {who} maximum hand size is equal to seven minus the number of those card types.",
            self.minimum_types
        )
    }

    fn apply_restrictions(&self, game: &mut GameState, _source: ObjectId, controller: PlayerId) {
        let card_types = count_distinct_card_types_in_graveyard(game, controller);
        if card_types < self.minimum_types as i32 {
            return;
        }

        let max_hand_size = (7 - card_types).max(0);
        let affected = player_ids_for_filter(game, self.player.clone(), controller);
        for player_id in affected {
            if let Some(player) = game.player_mut(player_id) {
                player.max_hand_size = max_hand_size;
            }
        }
    }
}

/// Library of Leng's discard replacement effect.
///
/// "If an effect causes you to discard a card, you may put it on top of
/// your library instead of into your graveyard."
///
/// Key rules:
/// - Only applies to discards from effects (not costs)
/// - Uses the composable EventCause system to filter on cause type
/// - Offers an interactive choice between graveyard and library
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LibraryOfLengDiscardReplacement;

impl StaticAbilityKind for LibraryOfLengDiscardReplacement {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::LibraryOfLengDiscardReplacement
    }

    fn display(&self) -> String {
        "If an effect causes you to discard a card, you may put it on top of your library instead"
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
            // Use the composable matcher that filters on cause type
            WouldDiscardMatcher::you_from_effect(),
            ReplacementAction::InteractiveChooseDestination {
                destinations: vec![Zone::Graveyard, Zone::Library],
                description:
                    "Library of Leng: Put discarded card on top of library instead of graveyard?"
                        .to_string(),
            },
        ))
    }
}

/// "If you would draw a card, exile the top card of your library face down instead."
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DrawReplacementExileTopFaceDown;

impl StaticAbilityKind for DrawReplacementExileTopFaceDown {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::DrawReplacementExileTopFaceDown
    }

    fn display(&self) -> String {
        "If you would draw a card, exile the top card of your library face down instead."
            .to_string()
    }

    fn generate_replacement_effect(
        &self,
        source: ObjectId,
        controller: PlayerId,
    ) -> Option<ReplacementEffect> {
        const TOP_CARD_TAG: &str = "draw_replacement_top_card";

        Some(ReplacementEffect::with_matcher(
            source,
            controller,
            WouldDrawCardMatcher::you(),
            ReplacementAction::Instead(vec![
                Effect::new(
                    crate::effects::ChooseObjectsEffect::new(
                        ObjectFilter::default()
                            .in_zone(Zone::Library)
                            .owned_by(PlayerFilter::You),
                        1,
                        PlayerFilter::You,
                        TOP_CARD_TAG,
                    )
                    .top_only(),
                ),
                Effect::new(
                    crate::effects::ExileEffect::with_spec(ChooseSpec::tagged(TOP_CARD_TAG))
                        .with_face_down(true),
                ),
            ]),
        ))
    }
}

/// "If a card would be put into an opponent's graveyard from anywhere, instead exile it with a
/// void counter on it."
#[derive(Debug, Clone, PartialEq)]
pub struct ExileToCounteredExileInsteadOfGraveyard {
    pub player: PlayerFilter,
    pub counter_type: CounterType,
}

impl ExileToCounteredExileInsteadOfGraveyard {
    pub fn new(player: PlayerFilter, counter_type: CounterType) -> Self {
        Self {
            player,
            counter_type,
        }
    }

    fn graveyard_owner_phrase(&self) -> &'static str {
        match self.player {
            PlayerFilter::You => "your",
            PlayerFilter::Opponent => "an opponent's",
            _ => "a player's",
        }
    }
}

impl StaticAbilityKind for ExileToCounteredExileInsteadOfGraveyard {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::ExileToCounteredExileInsteadOfGraveyard
    }

    fn display(&self) -> String {
        let counter = self.counter_type.description().into_owned();
        format!(
            "If a card would be put into {} graveyard from anywhere, instead exile it with a {} counter on it.",
            self.graveyard_owner_phrase(),
            counter
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
            crate::events::zones::matchers::WouldGoToGraveyardMatcher::new(
                ObjectFilter::default().owned_by(self.player.clone()),
            ),
            ReplacementAction::Instead(vec![
                Effect::new(crate::effects::ExileEffect::with_spec(ChooseSpec::Source)),
                Effect::new(crate::effects::PutCountersEffect::new(
                    self.counter_type,
                    Value::Fixed(1),
                    ChooseSpec::All(ObjectFilter::tagged(SOURCE_EXILED_TAG)),
                )),
            ]),
        ))
    }
}

// =============================================================================
// Interactive ETB Replacement Abilities (Unified System)
// =============================================================================

/// "You may discard a card matching [filter]. If you don't, put this into [zone]."
///
/// Used by: Mox Diamond (discard land or goes to graveyard)
///
/// This is an interactive replacement effect that uses the unified replacement
/// system rather than the deprecated EtbReplacementHandler trait.
#[derive(Debug, Clone, PartialEq)]
pub struct DiscardOrRedirectReplacement {
    /// Filter for cards that can be discarded to satisfy the replacement.
    pub filter: ObjectFilter,
    /// Where the permanent goes if no card is discarded.
    pub redirect_zone: Zone,
}

impl DiscardOrRedirectReplacement {
    /// Create a new discard-or-redirect replacement ability.
    pub fn new(filter: ObjectFilter, redirect_zone: Zone) -> Self {
        Self {
            filter,
            redirect_zone,
        }
    }
}

impl StaticAbilityKind for DiscardOrRedirectReplacement {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::DiscardOrRedirectReplacement
    }

    fn display(&self) -> String {
        let discard_phrase = describe_discard_filter_card_phrase(&self.filter);
        let redirect_phrase = describe_redirect_zone_phrase(self.redirect_zone);
        format!(
            "If this would enter the battlefield, you may discard {} instead. If you do, put it onto the battlefield. If you don't, put it into {}.",
            discard_phrase, redirect_phrase
        )
    }

    fn generate_replacement_effect(
        &self,
        source: ObjectId,
        controller: PlayerId,
    ) -> Option<ReplacementEffect> {
        Some(
            ReplacementEffect::with_matcher(
                source,
                controller,
                ThisWouldEnterBattlefieldMatcher,
                ReplacementAction::InteractiveDiscardOrRedirect {
                    filter: self.filter.clone(),
                    redirect_zone: self.redirect_zone,
                },
            )
            .self_replacing(),
        )
    }
}

/// "As this enters the battlefield, you may pay N life. If you don't, it enters tapped."
///
/// Used by: Shock lands (Godless Shrine, etc.), slow fetches (Vault of Champions, etc.)
///
/// This is an interactive replacement effect that uses the unified replacement
/// system rather than the deprecated EtbReplacementHandler trait.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PayLifeOrEnterTappedReplacement {
    /// The amount of life to pay to enter untapped.
    pub life_cost: u32,
}

impl PayLifeOrEnterTappedReplacement {
    /// Create a new pay-life-or-enter-tapped replacement ability.
    pub fn new(life_cost: u32) -> Self {
        Self { life_cost }
    }
}

impl StaticAbilityKind for PayLifeOrEnterTappedReplacement {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::PayLifeOrEnterTappedReplacement
    }

    fn display(&self) -> String {
        format!(
            "As this enters the battlefield, you may pay {} life. If you don't, it enters tapped.",
            self.life_cost
        )
    }

    fn generate_replacement_effect(
        &self,
        source: ObjectId,
        controller: PlayerId,
    ) -> Option<ReplacementEffect> {
        Some(
            ReplacementEffect::with_matcher(
                source,
                controller,
                ThisWouldEnterBattlefieldMatcher,
                ReplacementAction::InteractivePayLifeOrEnterTapped {
                    life_cost: self.life_cost,
                },
            )
            .self_replacing(),
        )
    }

    fn enters_tapped(&self) -> bool {
        // This is conditionally enters tapped, so we return false here
        // The actual tapped state is determined by the replacement effect
        false
    }
}

/// Parser-backed pregame action from opening hand.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PregameAction {
    pub kind: crate::static_abilities::PregameActionKind,
    pub text: String,
}

impl PregameAction {
    pub fn new(kind: crate::static_abilities::PregameActionKind, text: impl Into<String>) -> Self {
        Self {
            kind,
            text: text.into(),
        }
    }
}

impl StaticAbilityKind for PregameAction {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::PregameAction
    }

    fn display(&self) -> String {
        self.text.clone()
    }

    fn pregame_action_kind(&self) -> Option<crate::static_abilities::PregameActionKind> {
        Some(self.kind.clone())
    }
}

// =============================================================================
// Placeholder / Marker Abilities
// =============================================================================

/// Non-semantic keyword-like marker preserved by parser/builder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeywordMarker {
    pub marker: String,
}

impl KeywordMarker {
    pub fn new(marker: impl Into<String>) -> Self {
        Self {
            marker: marker.into(),
        }
    }
}

impl StaticAbilityKind for KeywordMarker {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::KeywordMarker
    }

    fn display(&self) -> String {
        self.marker.clone()
    }
}

/// Non-semantic static rule text placeholder preserved by parser/builder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleTextPlaceholder {
    pub text: String,
}

impl RuleTextPlaceholder {
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into() }
    }
}

impl StaticAbilityKind for RuleTextPlaceholder {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::RuleTextPlaceholder
    }

    fn display(&self) -> String {
        self.text.clone()
    }
}

/// Typed fallback keyword text preserved from parser/builder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeywordFallbackText {
    pub text: String,
}

impl KeywordFallbackText {
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into() }
    }
}

impl StaticAbilityKind for KeywordFallbackText {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::KeywordFallbackText
    }

    fn display(&self) -> String {
        self.text.clone()
    }
}

/// Typed fallback static rule text preserved from parser/builder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleFallbackText {
    pub text: String,
}

impl RuleFallbackText {
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into() }
    }
}

impl StaticAbilityKind for RuleFallbackText {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::RuleFallbackText
    }

    fn display(&self) -> String {
        self.text.clone()
    }
}

/// Parser fallback marker used in allow-unsupported mode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnsupportedParserLine {
    pub raw_line: String,
    pub reason: String,
}

impl UnsupportedParserLine {
    pub fn new(raw_line: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            raw_line: raw_line.into(),
            reason: reason.into(),
        }
    }
}

impl StaticAbilityKind for UnsupportedParserLine {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::UnsupportedParserLine
    }

    fn display(&self) -> String {
        format!(
            "Unsupported parser line fallback: {} ({})",
            self.raw_line.trim(),
            self.reason
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StaticAbility;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::events::DamageEvent;
    use crate::events::EventContext;
    use crate::events::zones::ZoneChangeEvent;
    use crate::game_event::DamageTarget;
    use crate::ids::{CardId, PlayerId};
    use crate::rules::state_based::apply_state_based_actions_with;
    use crate::types::{CardType, Subtype};
    use crate::zone::Zone;

    #[test]
    fn test_doesnt_untap() {
        let ability = DoesntUntap;
        assert_eq!(ability.id(), StaticAbilityId::DoesntUntap);
        assert!(ability.affects_untap());
    }

    #[test]
    fn test_may_choose_not_to_untap_during_untap_step() {
        let ability = MayChooseNotToUntapDuringUntapStep::new("this artifact");
        assert_eq!(
            ability.id(),
            StaticAbilityId::MayChooseNotToUntapDuringUntapStep
        );
        assert_eq!(
            ability.display(),
            "You may choose not to untap this artifact during your untap step"
        );
    }

    #[test]
    fn test_enters_tapped() {
        let ability = EntersTapped;
        assert_eq!(ability.id(), StaticAbilityId::EntersTapped);
        assert!(ability.enters_tapped());
    }

    #[test]
    fn test_no_maximum_hand_size() {
        let ability = NoMaximumHandSize;
        assert_eq!(ability.id(), StaticAbilityId::NoMaximumHandSize);

        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let source = ObjectId::from_raw(42);
        ability.apply_restrictions(&mut game, source, alice);
        assert_eq!(
            game.player(alice)
                .expect("alice should exist")
                .max_hand_size,
            i32::MAX
        );
    }

    #[test]
    fn test_reduce_maximum_hand_size_for_opponents() {
        let ability = ReduceMaximumHandSize::new(PlayerFilter::Opponent, 4);
        assert_eq!(ability.id(), StaticAbilityId::ReduceMaximumHandSize);

        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = ObjectId::from_raw(43);
        ability.apply_restrictions(&mut game, source, alice);

        assert_eq!(
            game.player(alice)
                .expect("alice should exist")
                .max_hand_size,
            7
        );
        assert_eq!(game.player(bob).expect("bob should exist").max_hand_size, 3);
    }

    #[test]
    fn test_conditional_spell_keyword_active_by_mana_values() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);

        for (idx, mv) in [1u8, 2, 3, 4, 5].into_iter().enumerate() {
            let card = crate::card::CardBuilder::new(
                crate::ids::CardId::from_raw(800 + idx as u32),
                &format!("MV{mv}"),
            )
            .card_types(vec![crate::types::CardType::Instant])
            .mana_cost(crate::mana::ManaCost::from_pips(vec![vec![
                crate::mana::ManaSymbol::Generic(mv),
            ]]))
            .build();
            game.create_object_from_card(&card, alice, Zone::Graveyard);
        }

        let spec = ConditionalSpellKeywordSpec {
            keyword: ConditionalSpellKeywordKind::Flash,
            metric: GraveyardCountMetric::ManaValues,
            threshold: 5,
        };
        assert!(
            conditional_spell_keyword_active(spec, &game, alice),
            "expected mana-value threshold to be active"
        );
    }

    #[test]
    fn test_this_spell_cast_restriction_kind_roundtrip() {
        let ability = ThisSpellCastRestriction::new(
            ThisSpellCastRestrictionKind::during_declare_attackers_step_if_you_were_attacked_this_step(),
            "Cast this spell only during the declare attackers step and only if you've been attacked this step.",
        );
        assert_eq!(ability.id(), StaticAbilityId::ThisSpellCastRestriction);
        assert_eq!(
            ability.this_spell_cast_restriction_kind(),
            Some(ThisSpellCastRestrictionKind::during_declare_attackers_step_if_you_were_attacked_this_step())
        );
    }

    #[test]
    fn test_maximum_hand_size_seven_minus_card_types_applies_only_at_threshold() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let ability =
            MaximumHandSizeSevenMinusYourGraveyardCardTypes::new(PlayerFilter::Opponent, 4);
        let source = ObjectId::from_raw(900);

        for (idx, card_type) in [
            crate::types::CardType::Artifact,
            crate::types::CardType::Creature,
            crate::types::CardType::Enchantment,
        ]
        .into_iter()
        .enumerate()
        {
            let card = crate::card::CardBuilder::new(
                crate::ids::CardId::from_raw(900 + idx as u32),
                &format!("Type{idx}"),
            )
            .card_types(vec![card_type])
            .build();
            game.create_object_from_card(&card, alice, Zone::Graveyard);
        }

        ability.apply_restrictions(&mut game, source, alice);
        assert_eq!(
            game.player(bob).expect("bob should exist").max_hand_size,
            7,
            "threshold not met: max hand size should remain default"
        );

        let fourth = crate::card::CardBuilder::new(crate::ids::CardId::from_raw(999), "Type4")
            .card_types(vec![crate::types::CardType::Land])
            .build();
        game.create_object_from_card(&fourth, alice, Zone::Graveyard);

        ability.apply_restrictions(&mut game, source, alice);
        assert_eq!(
            game.player(bob).expect("bob should exist").max_hand_size,
            3,
            "with four card types, max hand size should be seven minus four"
        );
    }

    #[test]
    fn test_draw_replacement_exile_top_face_down() {
        let ability = DrawReplacementExileTopFaceDown;
        assert_eq!(
            ability.id(),
            StaticAbilityId::DrawReplacementExileTopFaceDown
        );

        let replacement = ability
            .generate_replacement_effect(ObjectId::from_raw(1), PlayerId::from_index(0))
            .expect("draw replacement should create replacement effect");
        let ReplacementAction::Instead(effects) = &replacement.replacement else {
            panic!("expected draw replacement to use an Instead action");
        };
        assert_eq!(effects.len(), 2, "expected choose+exile effect sequence");
        let choose_debug = format!("{:?}", effects[0]);
        assert!(
            choose_debug.contains("ChooseObjectsEffect"),
            "expected first effect to choose top library card, got {choose_debug}"
        );
        assert!(
            !choose_debug.contains("RevealTopEffect"),
            "draw replacement should not reveal the card, got {choose_debug}"
        );
    }

    #[test]
    fn test_exile_to_countered_exile_instead_of_graveyard_generates_replacement() {
        let ability =
            ExileToCounteredExileInsteadOfGraveyard::new(PlayerFilter::Opponent, CounterType::Void);
        assert_eq!(
            ability.id(),
            StaticAbilityId::ExileToCounteredExileInsteadOfGraveyard
        );

        let replacement = ability
            .generate_replacement_effect(ObjectId::from_raw(1), PlayerId::from_index(0))
            .expect("replacement should be generated");
        let ReplacementAction::Instead(effects) = &replacement.replacement else {
            panic!("expected opponent-graveyard replacement to use an Instead action");
        };
        assert_eq!(effects.len(), 2, "expected exile+counter effect sequence");

        let exile = effects[0]
            .downcast_ref::<crate::effects::ExileEffect>()
            .expect("expected exile effect");
        assert!(matches!(exile.spec, ChooseSpec::Source));

        let counters = effects[1]
            .downcast_ref::<crate::effects::PutCountersEffect>()
            .expect("expected put-counters effect");
        assert_eq!(counters.counter_type, CounterType::Void);
        assert!(matches!(
            counters.target,
            ChooseSpec::All(ref filter) if filter.tagged_constraints.iter().any(
                |constraint| constraint.tag.as_str() == SOURCE_EXILED_TAG
            )
        ));
    }

    #[test]
    fn test_keyword_marker() {
        let ability = KeywordMarker::new("test marker");
        assert_eq!(ability.id(), StaticAbilityId::KeywordMarker);
        assert_eq!(ability.display(), "test marker");
    }

    #[test]
    fn test_rule_text_placeholder() {
        let ability = RuleTextPlaceholder::new("Test rule text.");
        assert_eq!(ability.id(), StaticAbilityId::RuleTextPlaceholder);
        assert_eq!(ability.display(), "Test rule text.");
    }

    #[test]
    fn test_unsupported_parser_line() {
        let ability = UnsupportedParserLine::new("Some unsupported line.", "ParseError(\"mock\")");
        assert_eq!(ability.id(), StaticAbilityId::UnsupportedParserLine);
        assert_eq!(
            ability.display(),
            "Unsupported parser line fallback: Some unsupported line. (ParseError(\"mock\"))"
        );
    }

    #[test]
    fn test_morph_static_ability_reports_turn_face_up_cost() {
        let cost = ManaCost::from_pips(vec![vec![crate::mana::ManaSymbol::Generic(3)]]);
        let ability = Morph::new(cost.clone());
        assert_eq!(ability.id(), StaticAbilityId::Morph);
        assert_eq!(ability.turn_face_up_cost(), Some(&cost));
        assert!(!ability.is_megamorph());
    }

    #[test]
    fn test_megamorph_static_ability_reports_turn_face_up_cost() {
        let cost = ManaCost::from_pips(vec![vec![crate::mana::ManaSymbol::Green]]);
        let ability = Megamorph::new(cost.clone());
        assert_eq!(ability.id(), StaticAbilityId::Megamorph);
        assert_eq!(ability.turn_face_up_cost(), Some(&cost));
        assert!(ability.is_megamorph());
    }

    #[test]
    fn test_bloodthirst_replacement_matches_when_opponent_was_dealt_damage() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let source = ObjectId::from_raw(42);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        game.damage_to_players_this_turn.insert(bob, 3);

        let ability = Bloodthirst::new(2);
        let replacement = ability
            .generate_replacement_effect(source, alice)
            .expect("bloodthirst should create replacement");
        let matcher = replacement
            .matcher
            .as_ref()
            .expect("bloodthirst replacement must have matcher");
        let event = ZoneChangeEvent::new(source, Zone::Stack, Zone::Battlefield, None);
        let ctx = EventContext::for_replacement_effect(alice, source, &game);

        assert!(
            matcher.matches_event(&event, &ctx),
            "bloodthirst should match when an opponent was dealt damage"
        );
    }

    #[test]
    fn test_bloodthirst_replacement_does_not_match_without_opponent_damage() {
        let game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let source = ObjectId::from_raw(42);
        let alice = PlayerId::from_index(0);

        let ability = Bloodthirst::new(2);
        let replacement = ability
            .generate_replacement_effect(source, alice)
            .expect("bloodthirst should create replacement");
        let matcher = replacement
            .matcher
            .as_ref()
            .expect("bloodthirst replacement must have matcher");
        let event = ZoneChangeEvent::new(source, Zone::Stack, Zone::Battlefield, None);
        let ctx = EventContext::for_replacement_effect(alice, source, &game);

        assert!(
            !matcher.matches_event(&event, &ctx),
            "bloodthirst should not match when no opponent was dealt damage"
        );
    }

    #[test]
    fn test_enters_with_counters_if_condition_matches_when_true() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let source = ObjectId::from_raw(52);
        let alice = PlayerId::from_index(0);
        game.players_attacked_this_turn.insert(alice);

        let ability = EntersWithCountersIfCondition::new(
            CounterType::PlusOnePlusOne,
            Value::Fixed(1),
            Condition::AttackedThisTurn,
            "you attacked this turn".to_string(),
        );
        let replacement = ability
            .generate_replacement_effect(source, alice)
            .expect("conditional enters-with-counters should create replacement");
        let matcher = replacement
            .matcher
            .as_ref()
            .expect("conditional enters-with-counters replacement must have matcher");
        let event = ZoneChangeEvent::new(source, Zone::Stack, Zone::Battlefield, None);
        let ctx = EventContext::for_replacement_effect(alice, source, &game);
        assert!(
            matcher.matches_event(&event, &ctx),
            "conditional enters-with-counters should match when condition is true"
        );
    }

    #[test]
    fn test_enters_with_counters_if_condition_does_not_match_when_false() {
        let game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let source = ObjectId::from_raw(52);
        let alice = PlayerId::from_index(0);

        let ability = EntersWithCountersIfCondition::new(
            CounterType::PlusOnePlusOne,
            Value::Fixed(1),
            Condition::AttackedThisTurn,
            "you attacked this turn".to_string(),
        );
        let replacement = ability
            .generate_replacement_effect(source, alice)
            .expect("conditional enters-with-counters should create replacement");
        let matcher = replacement
            .matcher
            .as_ref()
            .expect("conditional enters-with-counters replacement must have matcher");
        let event = ZoneChangeEvent::new(source, Zone::Stack, Zone::Battlefield, None);
        let ctx = EventContext::for_replacement_effect(alice, source, &game);
        assert!(
            !matcher.matches_event(&event, &ctx),
            "conditional enters-with-counters should not match when condition is false"
        );
    }

    #[test]
    fn test_enters_with_counters_if_condition_matches_when_opponent_lost_life() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let source = ObjectId::from_raw(52);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        game.life_lost_this_turn.insert(bob, 2);

        let ability = EntersWithCountersIfCondition::new(
            CounterType::PlusOnePlusOne,
            Value::Fixed(1),
            Condition::OpponentLostLifeThisTurn,
            "an opponent lost life this turn".to_string(),
        );
        let replacement = ability
            .generate_replacement_effect(source, alice)
            .expect("conditional enters-with-counters should create replacement");
        let matcher = replacement
            .matcher
            .as_ref()
            .expect("conditional enters-with-counters replacement must have matcher");
        let event = ZoneChangeEvent::new(source, Zone::Stack, Zone::Battlefield, None);
        let ctx = EventContext::for_replacement_effect(alice, source, &game);
        assert!(
            matcher.matches_event(&event, &ctx),
            "conditional enters-with-counters should match when an opponent lost life this turn"
        );
    }

    #[test]
    fn test_enters_with_counters_if_condition_matches_when_permanent_left_battlefield() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let source = ObjectId::from_raw(52);
        let alice = PlayerId::from_index(0);
        game.permanents_left_battlefield_under_controller_this_turn
            .insert(alice, 1);

        let ability = EntersWithCountersIfCondition::new(
            CounterType::PlusOnePlusOne,
            Value::Fixed(1),
            Condition::PermanentLeftBattlefieldUnderYourControlThisTurn,
            "a permanent left the battlefield under your control this turn".to_string(),
        );
        let replacement = ability
            .generate_replacement_effect(source, alice)
            .expect("conditional enters-with-counters should create replacement");
        let matcher = replacement
            .matcher
            .as_ref()
            .expect("conditional enters-with-counters replacement must have matcher");
        let event = ZoneChangeEvent::new(source, Zone::Stack, Zone::Battlefield, None);
        let ctx = EventContext::for_replacement_effect(alice, source, &game);
        assert!(
            matcher.matches_event(&event, &ctx),
            "conditional enters-with-counters should match when a permanent left under your control"
        );
    }

    #[test]
    fn test_prevent_all_damage_dealt_by_this_permanent_generates_replacement() {
        let game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let src = ObjectId::from_raw(42);
        let alice = PlayerId::from_index(0);

        let ability = PreventAllDamageDealtByThisPermanent;
        let replacement = ability
            .generate_replacement_effect(src, alice)
            .expect("should generate replacement effect");
        assert_eq!(replacement.replacement, ReplacementAction::Prevent);

        let matcher = replacement
            .matcher
            .as_ref()
            .expect("replacement must have a matcher");
        let ctx = EventContext::for_replacement_effect(alice, src, &game);

        // Preventable damage from this permanent matches.
        let dmg = DamageEvent::new(src, DamageTarget::Player(alice), 3, false);
        assert!(matcher.matches_event(&dmg, &ctx));

        // Unpreventable damage from this permanent does not match.
        let unpreventable = DamageEvent::unpreventable(src, DamageTarget::Player(alice), 3, false);
        assert!(!matcher.matches_event(&unpreventable, &ctx));
    }

    #[test]
    fn test_prevent_all_damage_dealt_to_creatures_generates_replacement() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let src = ObjectId::from_raw(42);
        let alice = PlayerId::from_index(0);
        let card = CardBuilder::new(CardId::new(), "Creature Target")
            .card_types(vec![CardType::Creature])
            .build();
        let creature_id = game.create_object_from_card(&card, alice, Zone::Battlefield);

        let ability = PreventAllDamageDealtToCreatures;
        let replacement = ability
            .generate_replacement_effect(src, alice)
            .expect("should generate replacement effect");
        assert_eq!(replacement.replacement, ReplacementAction::Prevent);

        let matcher = replacement
            .matcher
            .as_ref()
            .expect("replacement must have a matcher");
        let ctx = EventContext::for_replacement_effect(alice, src, &game);

        let creature_damage = DamageEvent::new(src, DamageTarget::Object(creature_id), 3, false);
        assert!(matcher.matches_event(&creature_damage, &ctx));

        let player_damage = DamageEvent::new(src, DamageTarget::Player(alice), 3, false);
        assert!(!matcher.matches_event(&player_damage, &ctx));
    }

    #[test]
    fn test_prevent_all_damage_to_self_by_creatures_generates_replacement() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);

        let protected_card = CardBuilder::new(CardId::new(), "Protected Creature")
            .card_types(vec![CardType::Creature])
            .build();
        let protected = game.create_object_from_card(&protected_card, alice, Zone::Battlefield);

        let creature_source_card = CardBuilder::new(CardId::new(), "Creature Source")
            .card_types(vec![CardType::Creature])
            .build();
        let creature_source =
            game.create_object_from_card(&creature_source_card, alice, Zone::Battlefield);

        let noncreature_source_card = CardBuilder::new(CardId::new(), "Artifact Source")
            .card_types(vec![CardType::Artifact])
            .build();
        let noncreature_source =
            game.create_object_from_card(&noncreature_source_card, alice, Zone::Battlefield);

        let ability = PreventAllDamageToSelfByCreatures;
        let replacement = ability
            .generate_replacement_effect(protected, alice)
            .expect("should generate replacement effect");
        assert_eq!(replacement.replacement, ReplacementAction::Prevent);

        let matcher = replacement
            .matcher
            .as_ref()
            .expect("replacement must have a matcher");
        let ctx = EventContext::for_replacement_effect(alice, protected, &game);

        let creature_damage =
            DamageEvent::new(creature_source, DamageTarget::Object(protected), 3, false);
        assert!(matcher.matches_event(&creature_damage, &ctx));

        let noncreature_damage = DamageEvent::new(
            noncreature_source,
            DamageTarget::Object(protected),
            3,
            false,
        );
        assert!(!matcher.matches_event(&noncreature_damage, &ctx));
    }

    #[test]
    fn test_prevent_all_combat_damage_to_self_generates_replacement() {
        let game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let protected = ObjectId::from_raw(42);
        let source = ObjectId::from_raw(7);

        let ability = PreventAllCombatDamageToSelf;
        let replacement = ability
            .generate_replacement_effect(protected, alice)
            .expect("should generate replacement effect");
        assert_eq!(replacement.replacement, ReplacementAction::Prevent);

        let matcher = replacement
            .matcher
            .as_ref()
            .expect("replacement must have a matcher");
        let ctx = EventContext::for_replacement_effect(alice, protected, &game);

        let combat_damage = DamageEvent::new(source, DamageTarget::Object(protected), 3, true);
        assert!(matcher.matches_event(&combat_damage, &ctx));

        let noncombat_damage = DamageEvent::new(source, DamageTarget::Object(protected), 3, false);
        assert!(!matcher.matches_event(&noncombat_damage, &ctx));

        let unpreventable =
            DamageEvent::unpreventable(source, DamageTarget::Object(protected), 3, true);
        assert!(!matcher.matches_event(&unpreventable, &ctx));
    }

    #[test]
    fn test_prevent_damage_to_self_remove_counter_generates_replacement() {
        let src = ObjectId::from_raw(42);
        let alice = PlayerId::from_index(0);

        let ability = PreventDamageToSelfRemoveCounter::new(CounterType::PlusOnePlusOne, 1);
        let replacement = ability
            .generate_replacement_effect(src, alice)
            .expect("should generate replacement effect");

        let ReplacementAction::Instead(effects) = &replacement.replacement else {
            panic!("expected replacement to use Instead action");
        };
        assert_eq!(effects.len(), 1, "expected one removal effect");
        let remove = effects[0]
            .downcast_ref::<crate::effects::RemoveCountersEffect>()
            .expect("expected remove counters effect");
        assert_eq!(remove.counter_type, CounterType::PlusOnePlusOne);
        assert_eq!(remove.count, Value::Fixed(1));
        assert!(matches!(remove.target, ChooseSpec::Source));
    }

    #[test]
    fn test_umbra_armor_replaces_destroy_effect() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);

        let creature = CardBuilder::new(CardId::new(), "Protected Bear")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        let creature_id = game.create_object_from_card(&creature, alice, Zone::Battlefield);
        game.mark_damage(creature_id, 2);

        let mut aura_def = crate::cards::CardDefinition::new(
            CardBuilder::new(CardId::new(), "Umbra Shell")
                .card_types(vec![CardType::Enchantment])
                .subtypes(vec![Subtype::Aura])
                .build(),
        );
        aura_def
            .abilities
            .push(crate::ability::Ability::static_ability(
                StaticAbility::umbra_armor(),
            ));
        let aura_id = game.create_object_from_definition(&aura_def, alice, Zone::Battlefield);
        game.object_mut(aura_id).expect("aura exists").attached_to = Some(creature_id);
        game.object_mut(creature_id)
            .expect("creature exists")
            .attachments
            .push(aura_id);

        let result = crate::event_processor::process_destroy_full(&mut game, creature_id, None);
        assert_eq!(result, crate::event_processor::DestroyResult::Replaced);
        assert!(
            game.object(creature_id).is_some(),
            "creature should survive"
        );
        assert_eq!(
            game.damage_on(creature_id),
            0,
            "umbra armor should clear marked damage"
        );
        assert_eq!(
            game.player(alice)
                .expect("alice exists")
                .graveyard
                .iter()
                .filter_map(|&id| game.object(id))
                .any(|obj| obj.name == "Umbra Shell"),
            true,
            "umbra armor aura should be destroyed"
        );
    }

    #[test]
    fn test_umbra_armor_replaces_lethal_damage_state_based_destruction() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);

        let creature = CardBuilder::new(CardId::new(), "Protected Bear")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        let creature_id = game.create_object_from_card(&creature, alice, Zone::Battlefield);
        game.mark_damage(creature_id, 2);

        let mut aura_def = crate::cards::CardDefinition::new(
            CardBuilder::new(CardId::new(), "Umbra Shell")
                .card_types(vec![CardType::Enchantment])
                .subtypes(vec![Subtype::Aura])
                .build(),
        );
        aura_def
            .abilities
            .push(crate::ability::Ability::static_ability(
                StaticAbility::umbra_armor(),
            ));
        let aura_id = game.create_object_from_definition(&aura_def, alice, Zone::Battlefield);
        game.object_mut(aura_id).expect("aura exists").attached_to = Some(creature_id);
        game.object_mut(creature_id)
            .expect("creature exists")
            .attachments
            .push(aura_id);

        let mut dm = crate::decision::SelectFirstDecisionMaker;
        assert!(
            apply_state_based_actions_with(&mut game, &mut dm),
            "lethal damage should apply a state-based action"
        );
        assert!(
            game.object(creature_id).is_some(),
            "creature should survive"
        );
        assert_eq!(game.damage_on(creature_id), 0);
        assert!(
            game.player(alice)
                .expect("alice exists")
                .graveyard
                .iter()
                .filter_map(|&id| game.object(id))
                .any(|obj| obj.name == "Umbra Shell"),
            "umbra armor aura should be in the graveyard after replacing lethal damage"
        );
    }
}
