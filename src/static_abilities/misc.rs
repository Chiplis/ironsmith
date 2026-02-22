//! Miscellaneous static abilities.
//!
//! This module contains static abilities that don't fit neatly into other categories.

use super::{ChooseColorAsEntersSpec, StaticAbilityId, StaticAbilityKind};
use crate::ability::LevelAbility;
use crate::color::Color;
use crate::effect::{Effect, Value};
use crate::events::cards::matchers::{WouldDiscardMatcher, WouldDrawCardMatcher};
use crate::events::damage::matchers::DamageToPlayerOrObjectMatcher;
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
use crate::target::{ChooseSpec, ObjectFilter, PlayerFilter};
use crate::zone::Zone;

fn describe_counter_type(counter_type: CounterType) -> String {
    match counter_type {
        CounterType::PlusOnePlusOne => "+1/+1".to_string(),
        CounterType::MinusOneMinusOne => "-1/-1".to_string(),
        other => format!("{other:?}").to_ascii_lowercase(),
    }
}

fn card_type_word(card_type: crate::types::CardType) -> &'static str {
    match card_type {
        crate::types::CardType::Artifact => "artifact",
        crate::types::CardType::Battle => "battle",
        crate::types::CardType::Creature => "creature",
        crate::types::CardType::Enchantment => "enchantment",
        crate::types::CardType::Instant => "instant",
        crate::types::CardType::Kindred => "kindred",
        crate::types::CardType::Land => "land",
        crate::types::CardType::Planeswalker => "planeswalker",
        crate::types::CardType::Sorcery => "sorcery",
    }
}

fn pluralize(word: &str) -> String {
    if word.ends_with('s') {
        word.to_string()
    } else {
        format!("{word}s")
    }
}

fn number_word(n: u32) -> Option<&'static str> {
    match n {
        0 => Some("zero"),
        1 => Some("one"),
        2 => Some("two"),
        3 => Some("three"),
        4 => Some("four"),
        5 => Some("five"),
        6 => Some("six"),
        7 => Some("seven"),
        8 => Some("eight"),
        9 => Some("nine"),
        10 => Some("ten"),
        _ => None,
    }
}

fn join_with_and(items: &[String]) -> String {
    match items.len() {
        0 => String::new(),
        1 => items[0].clone(),
        2 => format!("{} and {}", items[0], items[1]),
        _ => {
            let mut out = items[..items.len() - 1].join(", ");
            out.push_str(", and ");
            out.push_str(&items[items.len() - 1]);
            out
        }
    }
}

fn capitalize_first(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => format!("{}{}", first.to_ascii_uppercase(), chars.as_str()),
        None => String::new(),
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

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(self.clone())
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

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(self.clone())
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

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(*self)
    }

    fn affects_untap(&self) -> bool {
        true
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

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(*self)
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

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(*self)
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

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(*self)
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

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(*self)
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

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(*self)
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

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(*self)
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

    fn clone_box(&self) -> Box<dyn ReplacementMatcher> {
        Box::new(*self)
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

    fn clone_box(&self) -> Box<dyn ReplacementMatcher> {
        Box::new(*self)
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

    fn clone_box(&self) -> Box<dyn ReplacementMatcher> {
        Box::new(*self)
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

    fn clone_box(&self) -> Box<dyn ReplacementMatcher> {
        Box::new(*self)
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

    fn clone_box(&self) -> Box<dyn ReplacementMatcher> {
        Box::new(*self)
    }

    fn display(&self) -> String {
        "When this would enter tapped unless you have two or more opponents".to_string()
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

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(*self)
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

    fn clone_box(&self) -> Box<dyn ReplacementMatcher> {
        Box::new(*self)
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
        let counter = describe_counter_type(self.counter_type);
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
                        .and_then(number_word)
                        .map(str::to_string)
                        .unwrap_or_else(|| v.to_string());
                    format!("Enters the battlefield with {rendered} {counter} counters on it")
                }
            }
            Value::X => {
                format!("Enters the battlefield with X {counter} counters on it")
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
                "Enters the battlefield with {:?} {} counters on it",
                self.count, counter
            ),
        }
    }

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(self.clone())
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
                },
            )
            .self_replacing(),
        )
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

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(*self)
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

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(*self)
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

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(*self)
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

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(*self)
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

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(*self)
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

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(self.clone())
    }

    fn color_choice_as_enters(&self) -> Option<ChooseColorAsEntersSpec> {
        Some(ChooseColorAsEntersSpec {
            excluded: self.excluded,
        })
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

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(self.clone())
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
            && filter.custom_static_markers.is_empty()
            && filter.excluded_custom_static_markers.is_empty()
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

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(self.clone())
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

/// Permanents matching a filter enter the battlefield with counters.
#[derive(Debug, Clone, PartialEq)]
pub struct EnterWithCountersForFilter {
    pub filter: ObjectFilter,
    pub counter_type: CounterType,
    pub count: u32,
}

impl EnterWithCountersForFilter {
    pub fn new(filter: ObjectFilter, counter_type: CounterType, count: u32) -> Self {
        Self {
            filter,
            counter_type,
            count,
        }
    }
}

impl StaticAbilityKind for EnterWithCountersForFilter {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::EnterWithCountersForFilter
    }

    fn display(&self) -> String {
        "Permanents enter the battlefield with counters".to_string()
    }

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(self.clone())
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
                count: Value::Fixed(self.count as i32),
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

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(*self)
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

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(*self)
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

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(*self)
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

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(*self)
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

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(*self)
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

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(*self)
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

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(*self)
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

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(self.clone())
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

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(self.clone())
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

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(*self)
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

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(*self)
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

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(*self)
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
        format!(
            "If this would enter the battlefield, you may discard a card. If you don't, put it into {:?}",
            self.redirect_zone
        )
    }

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(self.clone())
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

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(*self)
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

// =============================================================================
// Custom Abilities
// =============================================================================

/// Custom static ability with a unique ID.
#[derive(Debug, Clone, PartialEq)]
pub struct Custom {
    pub custom_id: &'static str,
    pub description: String,
}

impl Custom {
    pub fn new(id: &'static str, description: String) -> Self {
        Self {
            custom_id: id,
            description,
        }
    }
}

impl StaticAbilityKind for Custom {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::Custom
    }

    fn display(&self) -> String {
        self.description.clone()
    }

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::EventContext;
    use crate::events::zones::ZoneChangeEvent;
    use crate::zone::Zone;

    #[test]
    fn test_doesnt_untap() {
        let ability = DoesntUntap;
        assert_eq!(ability.id(), StaticAbilityId::DoesntUntap);
        assert!(ability.affects_untap());
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
    fn test_custom() {
        let ability = Custom::new("test_ability", "Test description".to_string());
        assert_eq!(ability.id(), StaticAbilityId::Custom);
        assert_eq!(ability.display(), "Test description");
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
}
