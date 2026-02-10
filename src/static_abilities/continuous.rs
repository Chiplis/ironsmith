//! Effect-generating static abilities.
//!
//! These abilities generate continuous effects that modify other objects
//! through the layer system.

use super::{StaticAbility, StaticAbilityId, StaticAbilityKind};
use crate::ability::Ability;
use crate::continuous::{
    ContinuousEffect, EffectSourceType, EffectTarget, Modification, PtSublayer,
};
use crate::effect::{Comparison, Value};
use crate::filter::TaggedOpbjectRelation;
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::object::CounterType;
use crate::target::ObjectFilter;
use crate::types::{CardType, Subtype, Supertype};
use crate::zone::Zone;

fn attached_subject(filter: &ObjectFilter) -> Option<String> {
    let attachment = filter.tagged_constraints.iter().find_map(|constraint| {
        if constraint.relation != TaggedOpbjectRelation::IsTaggedObject {
            return None;
        }
        match constraint.tag.as_str() {
            "enchanted" => Some("enchanted"),
            "equipped" => Some("equipped"),
            _ => None,
        }
    })?;

    let noun = if filter.card_types.len() == 1 {
        format!("{:?}", filter.card_types[0]).to_ascii_lowercase()
    } else {
        "permanent".to_string()
    };
    Some(format!("{attachment} {noun}"))
}

fn effect_target_for_filter(source: ObjectId, filter: &ObjectFilter) -> EffectTarget {
    if attached_subject(filter).is_some() {
        EffectTarget::AttachedTo(source)
    } else {
        EffectTarget::Filter(filter.clone())
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
            out.push_str(items.last().map(String::as_str).unwrap_or_default());
            out
        }
    }
}

fn color_list(colors: crate::color::ColorSet) -> Vec<String> {
    let mut list = Vec::new();
    if colors.contains(crate::color::Color::White) {
        list.push("white".to_string());
    }
    if colors.contains(crate::color::Color::Blue) {
        list.push("blue".to_string());
    }
    if colors.contains(crate::color::Color::Black) {
        list.push("black".to_string());
    }
    if colors.contains(crate::color::Color::Red) {
        list.push("red".to_string());
    }
    if colors.contains(crate::color::Color::Green) {
        list.push("green".to_string());
    }
    list
}

fn subject_text(filter: &ObjectFilter) -> String {
    attached_subject(filter).unwrap_or_else(|| filter.description())
}

fn strip_indefinite_article(text: &str) -> &str {
    if let Some(rest) = text.strip_prefix("a ") {
        return rest;
    }
    if let Some(rest) = text.strip_prefix("an ") {
        return rest;
    }
    text
}

fn split_subject_suffix(subject: &str) -> (&str, &str) {
    const SUFFIXES: &[&str] = &[
        " you control",
        " that player controls",
        " you own",
        " an opponent owns",
        " a player owns",
        " the active player owns",
        " that player owns",
        " a teammate owns",
        " the defending player owns",
        " an attacking player owns",
        " the damaged player owns",
        " target player owns",
        " target opponent owns",
        " that object's controller owns",
        " that object's owner owns",
    ];
    for suffix in SUFFIXES {
        if let Some(base) = subject.strip_suffix(suffix) {
            return (base, suffix);
        }
    }
    (subject, "")
}

fn pluralize_terminal_noun(base: &str) -> Option<String> {
    const NOUNS: &[&str] = &[
        "permanent",
        "creature",
        "artifact",
        "enchantment",
        "land",
        "planeswalker",
        "battle",
        "spell",
        "card",
        "token",
    ];
    for noun in NOUNS {
        if let Some(stem) = base.strip_suffix(noun) {
            if stem.is_empty() || stem.ends_with(' ') {
                return Some(format!("{stem}{noun}s"));
            }
        }
    }
    None
}

fn pluralized_subject_text(filter: &ObjectFilter) -> String {
    let mut subject = subject_text(filter);
    if subject.starts_with("another ") {
        subject = subject.replacen("another ", "other ", 1);
    }
    if subject.starts_with("enchanted ")
        || subject.starts_with("equipped ")
        || subject.starts_with("this ")
        || subject.starts_with("that ")
    {
        return subject;
    }

    let (base, suffix) = split_subject_suffix(&subject);
    let base = strip_indefinite_article(base);
    if let Some(plural_base) = pluralize_terminal_noun(base) {
        return format!("{plural_base}{suffix}");
    }
    subject
}

fn grant_subject_text(filter: &ObjectFilter) -> String {
    pluralized_subject_text(filter)
}

fn subject_verb_and_possessive(subject: &str) -> (&'static str, &'static str) {
    let singular = subject.starts_with("enchanted ")
        || subject.starts_with("equipped ")
        || subject.starts_with("this ")
        || subject.starts_with("that ");
    if singular {
        ("is", "its")
    } else {
        ("are", "their")
    }
}

/// Anthem effect: "Creatures you control get +N/+M"
#[derive(Debug, Clone, PartialEq)]
pub enum AnthemCountExpression {
    /// Count all objects matching a filter.
    MatchingFilter(ObjectFilter),
    /// Count attachments on the source that match a filter.
    AttachedToSource(ObjectFilter),
    /// Count distinct basic land types among matching lands.
    BasicLandTypesAmong(ObjectFilter),
}

#[derive(Debug, Clone, PartialEq)]
pub enum AnthemValue {
    /// A fixed power/toughness modifier.
    Fixed(i32),
    /// A modifier that scales by a counted quantity.
    PerCount {
        multiplier: i32,
        count: AnthemCountExpression,
    },
}

impl AnthemValue {
    pub fn scaled(multiplier: i32, count: AnthemCountExpression) -> Self {
        if multiplier == 0 {
            Self::Fixed(0)
        } else {
            Self::PerCount { multiplier, count }
        }
    }

    fn evaluate(&self, game: &GameState, source: ObjectId, controller: PlayerId) -> i32 {
        match self {
            Self::Fixed(value) => *value,
            Self::PerCount { multiplier, count } => {
                multiplier * resolve_anthem_count_expression(count, game, source, controller)
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum StaticCondition {
    /// "During your turn" / "as long as it's your turn"
    YourTurn,
    /// "As long as this creature is equipped"
    SourceIsEquipped,
    /// "As long as equipped creature is tapped"
    EquippedCreatureTapped,
    /// "As long as equipped creature is untapped"
    EquippedCreatureUntapped,
    /// Count-based condition ("as long as you control three or more artifacts", etc.)
    CountComparison {
        count: AnthemCountExpression,
        comparison: Comparison,
        display: Option<String>,
    },
}

fn strip_article(text: String) -> String {
    if let Some(rest) = text.strip_prefix("a ") {
        return rest.to_string();
    }
    if let Some(rest) = text.strip_prefix("an ") {
        return rest.to_string();
    }
    text
}

fn describe_anthem_count_expression(expr: &AnthemCountExpression) -> String {
    match expr {
        AnthemCountExpression::MatchingFilter(filter) => strip_article(filter.description()),
        AnthemCountExpression::AttachedToSource(filter) => {
            format!(
                "{} attached to this creature",
                strip_article(filter.description())
            )
        }
        AnthemCountExpression::BasicLandTypesAmong(_) => {
            "basic land type among lands you control".to_string()
        }
    }
}

fn comparison_display(cmp: &Comparison) -> String {
    match cmp {
        Comparison::GreaterThan(n) => format!("more than {n}"),
        Comparison::GreaterThanOrEqual(n) => format!("{n} or more"),
        Comparison::Equal(n) => n.to_string(),
        Comparison::LessThan(n) => format!("less than {n}"),
        Comparison::LessThanOrEqual(0) => "no".to_string(),
        Comparison::LessThanOrEqual(n) => format!("{n} or less"),
        Comparison::NotEqual(n) => format!("not {n}"),
    }
}

fn describe_static_condition(condition: &StaticCondition) -> String {
    match condition {
        StaticCondition::YourTurn => "as long as it's your turn".to_string(),
        StaticCondition::SourceIsEquipped => "as long as this creature is equipped".to_string(),
        StaticCondition::EquippedCreatureTapped => {
            "as long as equipped creature is tapped".to_string()
        }
        StaticCondition::EquippedCreatureUntapped => {
            "as long as equipped creature is untapped".to_string()
        }
        StaticCondition::CountComparison {
            count,
            comparison,
            display,
        } => {
            if let Some(display) = display {
                return format!("as long as {display}");
            }
            format!(
                "as long as there are {} {}",
                comparison_display(comparison),
                describe_anthem_count_expression(count)
            )
        }
    }
}

fn all_game_object_ids(game: &GameState) -> Vec<ObjectId> {
    let mut ids = Vec::new();
    ids.extend(game.battlefield.iter().copied());
    ids.extend(game.exile.iter().copied());
    ids.extend(game.command_zone.iter().copied());
    ids.extend(game.stack.iter().map(|entry| entry.object_id));
    for player in &game.players {
        ids.extend(player.library.iter().copied());
        ids.extend(player.hand.iter().copied());
        ids.extend(player.graveyard.iter().copied());
    }
    ids
}

fn resolve_anthem_count_expression(
    count: &AnthemCountExpression,
    game: &GameState,
    source: ObjectId,
    controller: PlayerId,
) -> i32 {
    let filter_ctx = game.filter_context_for(controller, Some(source));
    match count {
        AnthemCountExpression::MatchingFilter(filter) => all_game_object_ids(game)
            .into_iter()
            .filter_map(|id| game.object(id))
            .filter(|obj| filter.matches(obj, &filter_ctx, game))
            .count() as i32,
        AnthemCountExpression::AttachedToSource(filter) => game
            .object(source)
            .map(|source_obj| {
                source_obj
                    .attachments
                    .iter()
                    .filter_map(|id| game.object(*id))
                    .filter(|obj| filter.matches(obj, &filter_ctx, game))
                    .count() as i32
            })
            .unwrap_or(0),
        AnthemCountExpression::BasicLandTypesAmong(filter) => {
            use std::collections::HashSet;

            let mut seen = HashSet::new();
            for obj in all_game_object_ids(game)
                .into_iter()
                .filter_map(|id| game.object(id))
                .filter(|obj| filter.matches(obj, &filter_ctx, game))
            {
                for subtype in &obj.subtypes {
                    if matches!(
                        subtype,
                        Subtype::Plains
                            | Subtype::Island
                            | Subtype::Swamp
                            | Subtype::Mountain
                            | Subtype::Forest
                    ) {
                        seen.insert(subtype.clone());
                    }
                }
            }
            seen.len() as i32
        }
    }
}

fn static_condition_is_active(
    condition: &StaticCondition,
    game: &GameState,
    source: ObjectId,
    controller: PlayerId,
) -> bool {
    match condition {
        StaticCondition::YourTurn => game.turn.active_player == controller,
        StaticCondition::SourceIsEquipped => game.object(source).is_some_and(|source_obj| {
            source_obj.attachments.iter().any(|id| {
                game.object(*id)
                    .is_some_and(|obj| obj.subtypes.contains(&Subtype::Equipment))
            })
        }),
        StaticCondition::EquippedCreatureTapped => game
            .object(source)
            .and_then(|source_obj| source_obj.attached_to)
            .is_some_and(|attached| game.is_tapped(attached)),
        StaticCondition::EquippedCreatureUntapped => game
            .object(source)
            .and_then(|source_obj| source_obj.attached_to)
            .is_some_and(|attached| !game.is_tapped(attached)),
        StaticCondition::CountComparison {
            count, comparison, ..
        } => comparison.evaluate(resolve_anthem_count_expression(
            count, game, source, controller,
        )),
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Anthem {
    /// Filter for which permanents are affected.
    pub filter: ObjectFilter,
    /// If true, the source permanent itself is the only affected object.
    pub source_only: bool,
    /// Power modification.
    pub power: AnthemValue,
    /// Toughness modification.
    pub toughness: AnthemValue,
    /// Optional activation condition.
    pub condition: Option<StaticCondition>,
}

impl Anthem {
    pub fn new(filter: ObjectFilter, power: i32, toughness: i32) -> Self {
        Self {
            filter,
            source_only: false,
            power: AnthemValue::Fixed(power),
            toughness: AnthemValue::Fixed(toughness),
            condition: None,
        }
    }

    pub fn for_source(power: i32, toughness: i32) -> Self {
        Self {
            filter: ObjectFilter::creature(),
            source_only: true,
            power: AnthemValue::Fixed(power),
            toughness: AnthemValue::Fixed(toughness),
            condition: None,
        }
    }

    pub fn with_values(mut self, power: AnthemValue, toughness: AnthemValue) -> Self {
        self.power = power;
        self.toughness = toughness;
        self
    }

    pub fn with_condition(mut self, condition: StaticCondition) -> Self {
        self.condition = Some(condition);
        self
    }

    /// Create a standard anthem for creatures you control.
    pub fn creatures_you_control(power: i32, toughness: i32) -> Self {
        Self::new(ObjectFilter::creature().you_control(), power, toughness)
    }
}

impl StaticAbilityKind for Anthem {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::Anthem
    }

    fn display(&self) -> String {
        let subject = if self.source_only {
            "this creature".to_string()
        } else {
            pluralized_subject_text(&self.filter)
        };
        let singular = self.source_only
            || subject.starts_with("a ")
            || subject.starts_with("an ")
            || subject.starts_with("this ")
            || subject.starts_with("that ")
            || subject.starts_with("enchanted ")
            || subject.starts_with("equipped ");
        let verb = if singular { "gets" } else { "get" };

        let signed = |value: i32| {
            if value >= 0 {
                format!("+{value}")
            } else {
                value.to_string()
            }
        };

        let mut text = match (&self.power, &self.toughness) {
            (AnthemValue::Fixed(power), AnthemValue::Fixed(toughness)) => {
                format!("{subject} {verb} {}/{}", signed(*power), signed(*toughness),)
            }
            (
                AnthemValue::PerCount {
                    multiplier: power,
                    count: power_count,
                },
                AnthemValue::PerCount {
                    multiplier: toughness,
                    count: toughness_count,
                },
            ) if power_count == toughness_count => {
                format!(
                    "{subject} {verb} {}/{} for each {}",
                    signed(*power),
                    signed(*toughness),
                    describe_anthem_count_expression(power_count),
                )
            }
            (
                AnthemValue::PerCount {
                    multiplier: power,
                    count,
                },
                AnthemValue::Fixed(toughness),
            ) => format!(
                "{subject} {verb} {}/{} for each {}",
                signed(*power),
                signed(*toughness),
                describe_anthem_count_expression(count),
            ),
            (
                AnthemValue::Fixed(power),
                AnthemValue::PerCount {
                    multiplier: toughness,
                    count,
                },
            ) => format!(
                "{subject} {verb} {}/{} for each {}",
                signed(*power),
                signed(*toughness),
                describe_anthem_count_expression(count),
            ),
            _ => format!("{subject} {verb} dynamic power/toughness"),
        };

        if let Some(condition) = &self.condition {
            text.push(' ');
            text.push_str(&describe_static_condition(condition));
        }
        text
    }

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(self.clone())
    }

    fn generate_effects(
        &self,
        source: ObjectId,
        controller: PlayerId,
        game: &GameState,
    ) -> Vec<ContinuousEffect> {
        let power = self.power.evaluate(game, source, controller);
        let toughness = self.toughness.evaluate(game, source, controller);
        let target = if self.source_only {
            EffectTarget::Source
        } else {
            effect_target_for_filter(source, &self.filter)
        };
        vec![
            ContinuousEffect::new(
                source,
                controller,
                target,
                Modification::ModifyPowerToughness { power, toughness },
            )
            .with_source_type(EffectSourceType::StaticAbility),
        ]
    }

    fn is_active(&self, game: &GameState, source: ObjectId) -> bool {
        let Some(condition) = &self.condition else {
            return true;
        };
        let Some(source_obj) = game.object(source) else {
            return false;
        };
        static_condition_is_active(condition, game, source, source_obj.controller)
    }

    fn is_anthem(&self) -> bool {
        true
    }
}

/// Grant ability: "Creatures you control have [ability]"
#[derive(Debug, Clone)]
pub struct GrantAbility {
    /// Filter for which permanents gain the ability.
    pub filter: ObjectFilter,
    /// If true, this grants only to the source object.
    pub source_only: bool,
    /// The ability to grant.
    pub ability: StaticAbility,
    /// Optional activation condition.
    pub condition: Option<StaticCondition>,
}

impl GrantAbility {
    pub fn new(filter: ObjectFilter, ability: StaticAbility) -> Self {
        Self {
            filter,
            source_only: false,
            ability,
            condition: None,
        }
    }

    pub fn source(ability: StaticAbility) -> Self {
        Self {
            filter: ObjectFilter::creature(),
            source_only: true,
            ability,
            condition: None,
        }
    }

    pub fn with_condition(mut self, condition: StaticCondition) -> Self {
        self.condition = Some(condition);
        self
    }
}

impl PartialEq for GrantAbility {
    fn eq(&self, other: &Self) -> bool {
        self.filter == other.filter
            && self.source_only == other.source_only
            && self.ability == other.ability
            && self.condition == other.condition
    }
}

impl StaticAbilityKind for GrantAbility {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::GrantAbility
    }

    fn display(&self) -> String {
        let mut text = if self.source_only {
            format!("this creature has {}", self.ability.display())
        } else {
            let subject = grant_subject_text(&self.filter);
            let singular_subject = subject.starts_with("enchanted ")
                || subject.starts_with("equipped ")
                || subject.starts_with("this ")
                || subject.starts_with("that ");
            let verb = if singular_subject { "has" } else { "have" };
            format!("{subject} {verb} {}", self.ability.display())
        };
        if let Some(condition) = &self.condition {
            text.push(' ');
            text.push_str(&describe_static_condition(condition));
        }
        text
    }

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(self.clone())
    }

    fn grants_abilities(&self) -> bool {
        true
    }

    fn generate_effects(
        &self,
        source: ObjectId,
        controller: PlayerId,
        _game: &GameState,
    ) -> Vec<ContinuousEffect> {
        let target = if self.source_only {
            EffectTarget::Source
        } else {
            effect_target_for_filter(source, &self.filter)
        };
        vec![
            ContinuousEffect::new(
                source,
                controller,
                target,
                Modification::AddAbility(self.ability.clone()),
            )
            .with_source_type(EffectSourceType::StaticAbility),
        ]
    }

    fn apply_restrictions(&self, game: &mut GameState, _source: ObjectId, controller: PlayerId) {
        if self.source_only {
            self.ability.apply_restrictions(game, _source, controller);
            return;
        }

        // Find permanents matching the filter
        let filter_ctx = game.filter_context_for(controller, None);
        let matching: Vec<ObjectId> = game
            .battlefield
            .iter()
            .filter(|&&id| {
                game.object(id)
                    .map(|obj| self.filter.matches(obj, &filter_ctx, game))
                    .unwrap_or(false)
            })
            .copied()
            .collect();

        // Apply the granted ability's restrictions to each matching permanent
        for perm_id in matching {
            self.ability.apply_restrictions(game, perm_id, controller);
        }
    }

    fn is_active(&self, game: &GameState, source: ObjectId) -> bool {
        let Some(condition) = &self.condition else {
            return true;
        };
        let Some(source_obj) = game.object(source) else {
            return false;
        };
        static_condition_is_active(condition, game, source, source_obj.controller)
    }
}

/// Remove ability: "Creatures lose [ability]"
#[derive(Debug, Clone)]
pub struct RemoveAbilityForFilter {
    /// Filter for which permanents lose the ability.
    pub filter: ObjectFilter,
    /// The ability to remove.
    pub ability: StaticAbility,
}

impl RemoveAbilityForFilter {
    pub fn new(filter: ObjectFilter, ability: StaticAbility) -> Self {
        Self { filter, ability }
    }
}

impl PartialEq for RemoveAbilityForFilter {
    fn eq(&self, other: &Self) -> bool {
        self.filter == other.filter && self.ability == other.ability
    }
}

impl StaticAbilityKind for RemoveAbilityForFilter {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::RemoveAbilityForFilter
    }

    fn display(&self) -> String {
        format!(
            "{} lose {}",
            subject_text(&self.filter),
            self.ability.display()
        )
    }

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(self.clone())
    }

    fn generate_effects(
        &self,
        source: ObjectId,
        controller: PlayerId,
        _game: &GameState,
    ) -> Vec<ContinuousEffect> {
        vec![
            ContinuousEffect::new(
                source,
                controller,
                EffectTarget::Filter(self.filter.clone()),
                Modification::RemoveAbility(self.ability.clone()),
            )
            .with_source_type(EffectSourceType::StaticAbility),
        ]
    }
}

/// Remove all abilities: "Creatures lose all abilities"
#[derive(Debug, Clone, PartialEq)]
pub struct RemoveAllAbilitiesForFilter {
    /// Filter for which permanents lose all abilities.
    pub filter: ObjectFilter,
}

impl RemoveAllAbilitiesForFilter {
    pub fn new(filter: ObjectFilter) -> Self {
        Self { filter }
    }
}

impl StaticAbilityKind for RemoveAllAbilitiesForFilter {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::RemoveAllAbilitiesForFilter
    }

    fn display(&self) -> String {
        format!("{} lose all abilities", subject_text(&self.filter))
    }

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(self.clone())
    }

    fn generate_effects(
        &self,
        source: ObjectId,
        controller: PlayerId,
        _game: &GameState,
    ) -> Vec<ContinuousEffect> {
        vec![
            ContinuousEffect::new(
                source,
                controller,
                EffectTarget::Filter(self.filter.clone()),
                Modification::RemoveAllAbilities,
            )
            .with_source_type(EffectSourceType::StaticAbility),
        ]
    }
}

/// Set base P/T: "... have base power and toughness N/M"
#[derive(Debug, Clone, PartialEq)]
pub struct SetBasePowerToughnessForFilter {
    /// Filter for which permanents get base P/T set.
    pub filter: ObjectFilter,
    /// Base power value.
    pub power: i32,
    /// Base toughness value.
    pub toughness: i32,
}

impl SetBasePowerToughnessForFilter {
    pub fn new(filter: ObjectFilter, power: i32, toughness: i32) -> Self {
        Self {
            filter,
            power,
            toughness,
        }
    }
}

impl StaticAbilityKind for SetBasePowerToughnessForFilter {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::SetBasePowerToughnessForFilter
    }

    fn display(&self) -> String {
        format!(
            "Affected permanents have base power and toughness {}/{}",
            self.power, self.toughness
        )
    }

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(self.clone())
    }

    fn generate_effects(
        &self,
        source: ObjectId,
        controller: PlayerId,
        _game: &GameState,
    ) -> Vec<ContinuousEffect> {
        vec![
            ContinuousEffect::new(
                source,
                controller,
                EffectTarget::Filter(self.filter.clone()),
                Modification::SetPowerToughness {
                    power: Value::Fixed(self.power),
                    toughness: Value::Fixed(self.toughness),
                    sublayer: PtSublayer::Setting,
                },
            )
            .with_source_type(EffectSourceType::StaticAbility),
        ]
    }
}

/// Condition for CopyActivatedAbilities.
#[derive(Debug, Clone, PartialEq)]
pub enum CopyActivatedAbilitiesCondition {
    /// "As long as you own a card exiled with a <counter> counter"
    OwnsCardExiledWithCounter(CounterType),
}

/// Copy activated abilities from objects matching a filter.
#[derive(Debug, Clone, PartialEq)]
pub struct CopyActivatedAbilities {
    pub filter: ObjectFilter,
    pub counter: Option<CounterType>,
    pub include_mana: bool,
    pub exclude_source_name: bool,
    pub exclude_source_id: bool,
    pub condition: Option<CopyActivatedAbilitiesCondition>,
    pub display: String,
}

impl CopyActivatedAbilities {
    pub fn new(filter: ObjectFilter) -> Self {
        Self {
            filter,
            counter: None,
            include_mana: true,
            exclude_source_name: false,
            exclude_source_id: true,
            condition: None,
            display: "Has all activated abilities of matching objects".to_string(),
        }
    }

    pub fn with_counter(mut self, counter: CounterType) -> Self {
        self.counter = Some(counter);
        self
    }

    pub fn with_exclude_source_name(mut self, exclude: bool) -> Self {
        self.exclude_source_name = exclude;
        self
    }

    pub fn with_exclude_source_id(mut self, exclude: bool) -> Self {
        self.exclude_source_id = exclude;
        self
    }

    pub fn with_condition(mut self, condition: CopyActivatedAbilitiesCondition) -> Self {
        self.condition = Some(condition);
        self
    }

    pub fn with_display(mut self, display: String) -> Self {
        self.display = display;
        self
    }
}

impl StaticAbilityKind for CopyActivatedAbilities {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::CopyActivatedAbilities
    }

    fn display(&self) -> String {
        self.display.clone()
    }

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(self.clone())
    }

    fn generate_effects(
        &self,
        source: ObjectId,
        controller: PlayerId,
        _game: &GameState,
    ) -> Vec<ContinuousEffect> {
        vec![
            ContinuousEffect::new(
                source,
                controller,
                EffectTarget::Source,
                Modification::CopyActivatedAbilities {
                    filter: self.filter.clone(),
                    counter: self.counter,
                    include_mana: self.include_mana,
                    exclude_source_name: self.exclude_source_name,
                    exclude_source_id: self.exclude_source_id,
                },
            )
            .with_source_type(EffectSourceType::StaticAbility),
        ]
    }

    fn is_active(&self, game: &GameState, source: ObjectId) -> bool {
        let Some(condition) = &self.condition else {
            return true;
        };

        let Some(source_obj) = game.object(source) else {
            return false;
        };
        let controller = source_obj.controller;

        match condition {
            CopyActivatedAbilitiesCondition::OwnsCardExiledWithCounter(counter) => {
                game.exile.iter().any(|&id| {
                    game.object(id).is_some_and(|obj| {
                        obj.owner == controller
                            && obj.counters.get(counter).copied().unwrap_or(0) > 0
                    })
                })
            }
        }
    }
}

/// Equipment grant: "Equipped creature has [abilities]"
#[derive(Debug, Clone)]
pub struct EquipmentGrant {
    /// The abilities to grant to the equipped creature.
    pub abilities: Vec<StaticAbility>,
}

/// Set colors: "All creatures are black."
#[derive(Debug, Clone)]
pub struct SetColorsForFilter {
    pub filter: ObjectFilter,
    pub colors: crate::color::ColorSet,
}

impl SetColorsForFilter {
    pub fn new(filter: ObjectFilter, colors: crate::color::ColorSet) -> Self {
        Self { filter, colors }
    }
}

impl PartialEq for SetColorsForFilter {
    fn eq(&self, other: &Self) -> bool {
        self.filter == other.filter && self.colors == other.colors
    }
}

impl StaticAbilityKind for SetColorsForFilter {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::SetColors
    }

    fn display(&self) -> String {
        let subject = subject_text(&self.filter);
        let (verb, _) = subject_verb_and_possessive(&subject);
        let colors = join_with_and(&color_list(self.colors));
        format!("{subject} {verb} {colors}")
    }

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(self.clone())
    }

    fn generate_effects(
        &self,
        source: ObjectId,
        controller: PlayerId,
        _game: &GameState,
    ) -> Vec<ContinuousEffect> {
        vec![
            ContinuousEffect::new(
                source,
                controller,
                effect_target_for_filter(source, &self.filter),
                Modification::SetColors(self.colors),
            )
            .with_source_type(EffectSourceType::StaticAbility),
        ]
    }
}

/// Add colors: "Enchanted creature is black in addition to its other colors."
#[derive(Debug, Clone)]
pub struct AddColorsForFilter {
    pub filter: ObjectFilter,
    pub colors: crate::color::ColorSet,
}

impl AddColorsForFilter {
    pub fn new(filter: ObjectFilter, colors: crate::color::ColorSet) -> Self {
        Self { filter, colors }
    }
}

impl PartialEq for AddColorsForFilter {
    fn eq(&self, other: &Self) -> bool {
        self.filter == other.filter && self.colors == other.colors
    }
}

impl StaticAbilityKind for AddColorsForFilter {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::AddColors
    }

    fn display(&self) -> String {
        let subject = subject_text(&self.filter);
        let (verb, possessive) = subject_verb_and_possessive(&subject);
        let colors = join_with_and(&color_list(self.colors));
        format!("{subject} {verb} {colors} in addition to {possessive} other colors")
    }

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(self.clone())
    }

    fn generate_effects(
        &self,
        source: ObjectId,
        controller: PlayerId,
        _game: &GameState,
    ) -> Vec<ContinuousEffect> {
        vec![
            ContinuousEffect::new(
                source,
                controller,
                effect_target_for_filter(source, &self.filter),
                Modification::AddColors(self.colors),
            )
            .with_source_type(EffectSourceType::StaticAbility),
        ]
    }
}

/// Add card types: "All permanents are artifacts in addition to their other types."
#[derive(Debug, Clone)]
pub struct AddCardTypesForFilter {
    pub filter: ObjectFilter,
    pub card_types: Vec<CardType>,
}

impl AddCardTypesForFilter {
    pub fn new(filter: ObjectFilter, card_types: Vec<CardType>) -> Self {
        Self { filter, card_types }
    }
}

impl PartialEq for AddCardTypesForFilter {
    fn eq(&self, other: &Self) -> bool {
        self.filter == other.filter && self.card_types == other.card_types
    }
}

impl StaticAbilityKind for AddCardTypesForFilter {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::AddCardTypes
    }

    fn display(&self) -> String {
        let subject = subject_text(&self.filter);
        let (verb, possessive) = subject_verb_and_possessive(&subject);
        let types = self
            .card_types
            .iter()
            .map(|card_type| format!("{card_type:?}").to_ascii_lowercase())
            .collect::<Vec<_>>();
        format!(
            "{subject} {verb} {} in addition to {possessive} other types",
            join_with_and(&types)
        )
    }

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(self.clone())
    }

    fn generate_effects(
        &self,
        source: ObjectId,
        controller: PlayerId,
        _game: &GameState,
    ) -> Vec<ContinuousEffect> {
        vec![
            ContinuousEffect::new(
                source,
                controller,
                effect_target_for_filter(source, &self.filter),
                Modification::AddCardTypes(self.card_types.clone()),
            )
            .with_source_type(EffectSourceType::StaticAbility),
        ]
    }
}

/// Add subtypes: "Enchanted creature is a Zombie in addition to its other types."
#[derive(Debug, Clone)]
pub struct AddSubtypesForFilter {
    pub filter: ObjectFilter,
    pub subtypes: Vec<Subtype>,
}

impl AddSubtypesForFilter {
    pub fn new(filter: ObjectFilter, subtypes: Vec<Subtype>) -> Self {
        Self { filter, subtypes }
    }
}

impl PartialEq for AddSubtypesForFilter {
    fn eq(&self, other: &Self) -> bool {
        self.filter == other.filter && self.subtypes == other.subtypes
    }
}

impl StaticAbilityKind for AddSubtypesForFilter {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::AddSubtypes
    }

    fn display(&self) -> String {
        let subject = subject_text(&self.filter);
        let (verb, possessive) = subject_verb_and_possessive(&subject);
        let subtypes = self
            .subtypes
            .iter()
            .map(|subtype| format!("{subtype:?}").to_ascii_lowercase())
            .collect::<Vec<_>>();
        format!(
            "{subject} {verb} {} in addition to {possessive} other types",
            join_with_and(&subtypes)
        )
    }

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(self.clone())
    }

    fn generate_effects(
        &self,
        source: ObjectId,
        controller: PlayerId,
        _game: &GameState,
    ) -> Vec<ContinuousEffect> {
        vec![
            ContinuousEffect::new(
                source,
                controller,
                effect_target_for_filter(source, &self.filter),
                Modification::AddSubtypes(self.subtypes.clone()),
            )
            .with_source_type(EffectSourceType::StaticAbility),
        ]
    }
}

/// Make colorless: "All permanents are colorless."
#[derive(Debug, Clone)]
pub struct MakeColorlessForFilter {
    pub filter: ObjectFilter,
}

impl MakeColorlessForFilter {
    pub fn new(filter: ObjectFilter) -> Self {
        Self { filter }
    }
}

impl PartialEq for MakeColorlessForFilter {
    fn eq(&self, other: &Self) -> bool {
        self.filter == other.filter
    }
}

impl StaticAbilityKind for MakeColorlessForFilter {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::MakeColorless
    }

    fn display(&self) -> String {
        "Permanents are colorless".to_string()
    }

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(self.clone())
    }

    fn generate_effects(
        &self,
        source: ObjectId,
        controller: PlayerId,
        _game: &GameState,
    ) -> Vec<ContinuousEffect> {
        vec![
            ContinuousEffect::new(
                source,
                controller,
                EffectTarget::Filter(self.filter.clone()),
                Modification::MakeColorless,
            )
            .with_source_type(EffectSourceType::StaticAbility),
        ]
    }
}

/// Remove supertypes: "All lands are no longer snow."
#[derive(Debug, Clone)]
pub struct RemoveSupertypesForFilter {
    pub filter: ObjectFilter,
    pub supertypes: Vec<Supertype>,
}

impl RemoveSupertypesForFilter {
    pub fn new(filter: ObjectFilter, supertypes: Vec<Supertype>) -> Self {
        Self { filter, supertypes }
    }
}

impl PartialEq for RemoveSupertypesForFilter {
    fn eq(&self, other: &Self) -> bool {
        self.filter == other.filter && self.supertypes == other.supertypes
    }
}

impl StaticAbilityKind for RemoveSupertypesForFilter {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::RemoveSupertypes
    }

    fn display(&self) -> String {
        let subject = self.filter.description();
        let supertypes = self
            .supertypes
            .iter()
            .map(|supertype| format!("{supertype:?}").to_ascii_lowercase())
            .collect::<Vec<_>>()
            .join(" and ");
        format!("{subject} is no longer {supertypes}")
    }

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(self.clone())
    }

    fn generate_effects(
        &self,
        source: ObjectId,
        controller: PlayerId,
        _game: &GameState,
    ) -> Vec<ContinuousEffect> {
        vec![
            ContinuousEffect::new(
                source,
                controller,
                EffectTarget::Filter(self.filter.clone()),
                Modification::RemoveSupertypes(self.supertypes.clone()),
            )
            .with_source_type(EffectSourceType::StaticAbility),
        ]
    }
}

impl EquipmentGrant {
    pub fn new(abilities: Vec<StaticAbility>) -> Self {
        Self { abilities }
    }
}

impl PartialEq for EquipmentGrant {
    fn eq(&self, other: &Self) -> bool {
        self.abilities == other.abilities
    }
}

impl StaticAbilityKind for EquipmentGrant {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::EquipmentGrant
    }

    fn display(&self) -> String {
        let ability_names: Vec<String> = self.abilities.iter().map(|a| a.display()).collect();
        format!("Equipped creature has {}", ability_names.join(", "))
    }

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(self.clone())
    }

    fn grants_abilities(&self) -> bool {
        true
    }

    fn equipment_grant_abilities(&self) -> Option<&[StaticAbility]> {
        Some(&self.abilities)
    }

    fn generate_effects(
        &self,
        source: ObjectId,
        controller: PlayerId,
        _game: &GameState,
    ) -> Vec<ContinuousEffect> {
        self.abilities
            .iter()
            .map(|ability| {
                ContinuousEffect::new(
                    source,
                    controller,
                    EffectTarget::AttachedTo(source),
                    Modification::AddAbility(ability.clone()),
                )
                .with_source_type(EffectSourceType::StaticAbility)
            })
            .collect()
    }
}

/// Enchanted/attached permanent has an activated or triggered ability.
#[derive(Debug, Clone, PartialEq)]
pub struct AttachedAbilityGrant {
    pub ability: Ability,
    pub display: String,
}

impl AttachedAbilityGrant {
    pub fn new(ability: Ability, display: String) -> Self {
        Self { ability, display }
    }
}

impl StaticAbilityKind for AttachedAbilityGrant {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::AttachedAbilityGrant
    }

    fn display(&self) -> String {
        self.display.clone()
    }

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(self.clone())
    }

    fn generate_effects(
        &self,
        source: ObjectId,
        controller: PlayerId,
        _game: &GameState,
    ) -> Vec<ContinuousEffect> {
        vec![
            ContinuousEffect::new(
                source,
                controller,
                EffectTarget::AttachedTo(source),
                Modification::AddAbilityGeneric(self.ability.clone()),
            )
            .with_source_type(EffectSourceType::StaticAbility),
        ]
    }
}

/// Permanents matching a filter have an activated or triggered ability.
#[derive(Debug, Clone, PartialEq)]
pub struct GrantObjectAbilityForFilter {
    pub filter: ObjectFilter,
    pub ability: Ability,
    pub display: String,
}

impl GrantObjectAbilityForFilter {
    pub fn new(filter: ObjectFilter, ability: Ability, display: String) -> Self {
        Self {
            filter,
            ability,
            display,
        }
    }
}

impl StaticAbilityKind for GrantObjectAbilityForFilter {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::GrantObjectAbilityForFilter
    }

    fn display(&self) -> String {
        self.display.clone()
    }

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(self.clone())
    }

    fn generate_effects(
        &self,
        source: ObjectId,
        controller: PlayerId,
        _game: &GameState,
    ) -> Vec<ContinuousEffect> {
        vec![
            ContinuousEffect::new(
                source,
                controller,
                EffectTarget::Filter(self.filter.clone()),
                Modification::AddAbilityGeneric(self.ability.clone()),
            )
            .with_source_type(EffectSourceType::StaticAbility),
        ]
    }
}

/// Blood Moon: "Nonbasic lands are Mountains"
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BloodMoon;

impl StaticAbilityKind for BloodMoon {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::BloodMoon
    }

    fn display(&self) -> String {
        "Nonbasic lands are Mountains".to_string()
    }

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(*self)
    }

    fn generate_effects(
        &self,
        source: ObjectId,
        controller: PlayerId,
        _game: &GameState,
    ) -> Vec<ContinuousEffect> {
        let nonbasic_land_filter = ObjectFilter {
            zone: Some(Zone::Battlefield),
            card_types: vec![CardType::Land],
            excluded_supertypes: vec![Supertype::Basic],
            ..Default::default()
        };

        vec![
            // Layer 4: Set land subtypes to Mountain
            ContinuousEffect::new(
                source,
                controller,
                EffectTarget::Filter(nonbasic_land_filter.clone()),
                Modification::SetSubtypes(vec![Subtype::Mountain]),
            )
            .with_source_type(EffectSourceType::StaticAbility),
            // Layer 6: Remove all abilities
            ContinuousEffect::new(
                source,
                controller,
                EffectTarget::Filter(nonbasic_land_filter),
                Modification::RemoveAllAbilities,
            )
            .with_source_type(EffectSourceType::StaticAbility),
        ]
    }
}

/// Toph, the First Metalbender: "Nontoken artifacts you control are lands in addition to their other types."
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TophFirstMetalbender;

impl StaticAbilityKind for TophFirstMetalbender {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::TophFirstMetalbender
    }

    fn display(&self) -> String {
        "Nontoken artifacts you control are lands in addition to their other types.".to_string()
    }

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(*self)
    }

    fn generate_effects(
        &self,
        source: ObjectId,
        controller: PlayerId,
        _game: &GameState,
    ) -> Vec<ContinuousEffect> {
        let filter = ObjectFilter::artifact().you_control().nontoken();
        vec![
            ContinuousEffect::new(
                source,
                controller,
                EffectTarget::Filter(filter),
                Modification::AddCardTypes(vec![CardType::Land]),
            )
            .with_source_type(EffectSourceType::StaticAbility),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::ids::CardId;
    use crate::types::{Subtype, Supertype};
    use crate::zone::Zone;

    #[test]
    fn test_anthem() {
        let anthem = Anthem::creatures_you_control(1, 1);
        assert_eq!(anthem.id(), StaticAbilityId::Anthem);
        assert!(anthem.is_anthem());
        assert_eq!(anthem.display(), "creatures you control get +1/+1");
    }

    #[test]
    fn test_remove_supertypes_display_mentions_scope_and_supertype() {
        let remove = RemoveSupertypesForFilter::new(ObjectFilter::land(), vec![Supertype::Snow]);
        assert_eq!(remove.display(), "land is no longer snow");
    }

    #[test]
    fn test_anthem_generates_effects() {
        let anthem = Anthem::creatures_you_control(2, 2);
        let game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let source = ObjectId::from_raw(1);
        let controller = PlayerId::from_index(0);

        let effects = anthem.generate_effects(source, controller, &game);
        assert_eq!(effects.len(), 1);
        assert!(matches!(
            effects[0].modification,
            Modification::ModifyPowerToughness {
                power: 2,
                toughness: 2
            }
        ));
    }

    #[test]
    fn test_attached_anthem_uses_attached_target() {
        let mut filter = ObjectFilter::creature();
        filter
            .tagged_constraints
            .push(crate::filter::TaggedObjectConstraint {
                tag: crate::tag::TagKey::from("enchanted"),
                relation: crate::filter::TaggedOpbjectRelation::IsTaggedObject,
            });
        let anthem = Anthem::new(filter, 1, 1);
        assert_eq!(anthem.display(), "enchanted creature gets +1/+1");

        let game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let source = ObjectId::from_raw(1);
        let controller = PlayerId::from_index(0);
        let effects = anthem.generate_effects(source, controller, &game);
        assert_eq!(effects.len(), 1);
        assert!(matches!(
            effects[0].applies_to,
            EffectTarget::AttachedTo(id) if id == source
        ));
    }

    #[test]
    fn test_source_dynamic_anthem_scales_from_filter_count() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);

        let source_card = CardBuilder::new(CardId::new(), "Nim Lasher")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(1, 1))
            .build();
        let source = game.create_object_from_card(&source_card, alice, Zone::Battlefield);

        let artifact_card = CardBuilder::new(CardId::new(), "Myr Token")
            .card_types(vec![CardType::Artifact])
            .build();
        game.create_object_from_card(&artifact_card, alice, Zone::Battlefield);
        game.create_object_from_card(&artifact_card, alice, Zone::Battlefield);

        let anthem = Anthem::for_source(0, 0).with_values(
            AnthemValue::scaled(
                1,
                AnthemCountExpression::MatchingFilter(ObjectFilter::artifact().you_control()),
            ),
            AnthemValue::Fixed(0),
        );

        let effects = anthem.generate_effects(source, alice, &game);
        assert_eq!(effects.len(), 1);
        assert!(matches!(
            effects[0].modification,
            Modification::ModifyPowerToughness {
                power: 2,
                toughness: 0
            }
        ));
        assert!(matches!(effects[0].applies_to, EffectTarget::Source));
    }

    #[test]
    fn test_conditional_anthem_is_active_only_when_condition_matches() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);

        let source_card = CardBuilder::new(CardId::new(), "Ardent Recruit")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(1, 1))
            .build();
        let source = game.create_object_from_card(&source_card, alice, Zone::Battlefield);

        let artifact_card = CardBuilder::new(CardId::new(), "Myr Token")
            .card_types(vec![CardType::Artifact])
            .build();
        game.create_object_from_card(&artifact_card, alice, Zone::Battlefield);
        game.create_object_from_card(&artifact_card, alice, Zone::Battlefield);

        let anthem = Anthem::for_source(2, 2).with_condition(StaticCondition::CountComparison {
            count: AnthemCountExpression::MatchingFilter(ObjectFilter::artifact().you_control()),
            comparison: Comparison::GreaterThanOrEqual(3),
            display: Some("you control three or more artifacts".to_string()),
        });

        assert!(
            !anthem.is_active(&game, source),
            "condition should be false with only two artifacts"
        );

        game.create_object_from_card(&artifact_card, alice, Zone::Battlefield);
        assert!(
            anthem.is_active(&game, source),
            "condition should be true with three artifacts"
        );
    }

    #[test]
    fn test_domain_count_expression_counts_distinct_basic_land_types() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);

        let source_card = CardBuilder::new(CardId::new(), "Kavu Scout")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(0, 2))
            .build();
        let source = game.create_object_from_card(&source_card, alice, Zone::Battlefield);

        let plains = CardBuilder::new(CardId::new(), "Plains")
            .card_types(vec![CardType::Land])
            .subtypes(vec![Subtype::Plains])
            .build();
        let forest = CardBuilder::new(CardId::new(), "Forest")
            .card_types(vec![CardType::Land])
            .subtypes(vec![Subtype::Forest])
            .build();
        let second_plains = CardBuilder::new(CardId::new(), "Snow Plains")
            .card_types(vec![CardType::Land])
            .subtypes(vec![Subtype::Plains])
            .build();

        game.create_object_from_card(&plains, alice, Zone::Battlefield);
        game.create_object_from_card(&forest, alice, Zone::Battlefield);
        game.create_object_from_card(&second_plains, alice, Zone::Battlefield);

        let anthem = Anthem::for_source(0, 0).with_values(
            AnthemValue::scaled(
                1,
                AnthemCountExpression::BasicLandTypesAmong(ObjectFilter::land().you_control()),
            ),
            AnthemValue::Fixed(0),
        );
        let effects = anthem.generate_effects(source, alice, &game);
        assert_eq!(effects.len(), 1);
        assert!(matches!(
            effects[0].modification,
            Modification::ModifyPowerToughness {
                power: 2,
                toughness: 0
            }
        ));
    }

    #[test]
    fn test_blood_moon() {
        let blood_moon = BloodMoon;
        assert_eq!(blood_moon.id(), StaticAbilityId::BloodMoon);
        assert_eq!(blood_moon.display(), "Nonbasic lands are Mountains");
    }

    #[test]
    fn test_blood_moon_generates_two_effects() {
        let blood_moon = BloodMoon;
        let game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let source = ObjectId::from_raw(1);
        let controller = PlayerId::from_index(0);

        let effects = blood_moon.generate_effects(source, controller, &game);
        assert_eq!(effects.len(), 2);
    }

    #[test]
    fn test_grant_ability() {
        let grant = GrantAbility::new(
            ObjectFilter::creature().you_control(),
            StaticAbility::flying(),
        );
        assert_eq!(grant.id(), StaticAbilityId::GrantAbility);
        assert!(grant.grants_abilities());
        assert_eq!(grant.display(), "creatures you control have Flying");
    }

    #[test]
    fn test_attached_grant_ability_uses_attached_target() {
        let mut filter = ObjectFilter::creature();
        filter
            .tagged_constraints
            .push(crate::filter::TaggedObjectConstraint {
                tag: crate::tag::TagKey::from("equipped"),
                relation: crate::filter::TaggedOpbjectRelation::IsTaggedObject,
            });
        let grant = GrantAbility::new(filter, StaticAbility::trample());
        assert_eq!(grant.display(), "equipped creature has Trample");

        let game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let source = ObjectId::from_raw(1);
        let controller = PlayerId::from_index(0);
        let effects = grant.generate_effects(source, controller, &game);
        assert_eq!(effects.len(), 1);
        assert!(matches!(
            effects[0].applies_to,
            EffectTarget::AttachedTo(id) if id == source
        ));
    }

    #[test]
    fn test_equipment_grant() {
        let grant = EquipmentGrant::new(vec![StaticAbility::haste(), StaticAbility::shroud()]);
        assert_eq!(grant.id(), StaticAbilityId::EquipmentGrant);
        assert!(grant.grants_abilities());
        assert!(grant.display().contains("Haste"));
        assert!(grant.display().contains("Shroud"));
    }

    #[test]
    fn test_remove_all_abilities_for_filter() {
        let ability = RemoveAllAbilitiesForFilter::new(ObjectFilter::creature());
        assert_eq!(ability.id(), StaticAbilityId::RemoveAllAbilitiesForFilter);

        let game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let effects =
            ability.generate_effects(ObjectId::from_raw(1), PlayerId::from_index(0), &game);
        assert_eq!(effects.len(), 1);
        assert!(matches!(
            effects[0].modification,
            Modification::RemoveAllAbilities
        ));
    }

    #[test]
    fn test_set_base_power_toughness_for_filter() {
        let ability = SetBasePowerToughnessForFilter::new(ObjectFilter::creature(), 1, 1);
        assert_eq!(
            ability.id(),
            StaticAbilityId::SetBasePowerToughnessForFilter
        );

        let game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let effects =
            ability.generate_effects(ObjectId::from_raw(1), PlayerId::from_index(0), &game);
        assert_eq!(effects.len(), 1);
        assert!(matches!(
            effects[0].modification,
            Modification::SetPowerToughness {
                power: Value::Fixed(1),
                toughness: Value::Fixed(1),
                sublayer: PtSublayer::Setting,
            }
        ));
    }
}
