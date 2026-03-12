//! Effect-generating static abilities.
//!
//! These abilities generate continuous effects that modify other objects
//! through the layer system.

use super::{StaticAbility, StaticAbilityId, StaticAbilityKind, text_utils::join_with_and};
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
    if filter.controller.is_some() || filter.owner.is_some() || filter.other {
        return None;
    }
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
        filter.card_types[0].name().to_string()
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

#[allow(dead_code)]
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

#[allow(dead_code)]
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
    let pluralize_word = |word: &str| {
        let lower = word.to_ascii_lowercase();
        if lower.ends_with('y')
            && lower.len() > 1
            && !matches!(
                lower.chars().nth(lower.len() - 2),
                Some('a' | 'e' | 'i' | 'o' | 'u')
            )
        {
            return format!("{}ies", &word[..word.len() - 1]);
        }
        if lower.ends_with('s')
            || lower.ends_with('x')
            || lower.ends_with('z')
            || lower.ends_with("ch")
            || lower.ends_with("sh")
        {
            return format!("{word}es");
        }
        format!("{word}s")
    };
    for noun in NOUNS {
        if let Some(stem) = base.strip_suffix(noun) {
            if stem.is_empty() || stem.ends_with(' ') {
                return Some(format!("{stem}{}", pluralize_word(noun)));
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
    let should_preserve_singular = (subject.starts_with("enchanted ")
        || subject.starts_with("equipped "))
        && filter.controller.is_none()
        && filter.owner.is_none()
        && !filter.other;
    if should_preserve_singular || subject.starts_with("this ") || subject.starts_with("that ") {
        return subject;
    }

    // Strip indefinite article from the beginning.
    let subject = if let Some(rest) = subject.strip_prefix("a ") {
        rest.to_string()
    } else if let Some(rest) = subject.strip_prefix("an ") {
        rest.to_string()
    } else {
        subject
    };

    // Find the first known singular noun in the subject and pluralize it.
    // This handles subjects like "card in graveyard", "creature you control with a counter on it"
    // correctly, since the noun appears before zone/controller/qualifier suffixes.
    const NOUNS: &[(&str, &str)] = &[
        ("permanent", "permanents"),
        ("creature", "creatures"),
        ("artifact", "artifacts"),
        ("enchantment", "enchantments"),
        ("land", "lands"),
        ("planeswalker", "planeswalkers"),
        ("battle", "battles"),
        ("spell", "spells"),
        ("card", "cards"),
        ("token", "tokens"),
    ];

    for &(singular, plural) in NOUNS {
        // Look for the noun as a whole word in the subject.
        if let Some(pos) = subject.to_ascii_lowercase().find(singular) {
            let before_ok = pos == 0 || subject.as_bytes()[pos - 1] == b' ';
            let after_pos = pos + singular.len();
            let after_ok = after_pos >= subject.len()
                || subject.as_bytes()[after_pos] == b' '
                || subject.as_bytes()[after_pos] == b'.';
            if before_ok && after_ok {
                let prefix = &subject[..pos];
                let suffix = &subject[after_pos..];
                return format!("{prefix}{plural}{suffix}");
            }
        }
    }

    // Fallback for subtype-only filters (e.g., "Zombie you control", "Rat you control"):
    // find the main noun (the word before " you control", " in graveyard", or similar suffixes)
    // and pluralize it.
    let (base, suffix) = split_subject_suffix(&subject);
    if !base.is_empty() {
        let prefix_all = suffix.is_empty()
            && !base.contains(' ')
            && base
                .chars()
                .next()
                .is_some_and(|ch| ch.is_ascii_uppercase());
        // Pluralize the last word of the base (the main noun/subtype).
        if let Some((head, noun)) = base.rsplit_once(' ') {
            let plural = simple_pluralize(noun);
            let subject = format!("{head} {plural}{suffix}");
            return if prefix_all {
                format!("All {subject}")
            } else {
                subject
            };
        }
        // Single word base (e.g., just "Zombie").
        let plural = simple_pluralize(base);
        let subject = format!("{plural}{suffix}");
        return if prefix_all {
            format!("All {subject}")
        } else {
            subject
        };
    }

    subject
}

fn simple_pluralize(word: &str) -> String {
    let lower = word.to_ascii_lowercase();
    if lower == "plains" || lower == "urzas" {
        return word.to_string();
    }
    if lower == "elf" {
        return if word
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_uppercase())
        {
            "Elves".to_string()
        } else {
            "elves".to_string()
        };
    }
    if lower == "dwarf" {
        return if word
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_uppercase())
        {
            "Dwarves".to_string()
        } else {
            "dwarves".to_string()
        };
    }
    if lower == "myr" {
        return word.to_string();
    }
    if lower == "mouse" {
        return if word
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_uppercase())
        {
            "Mice".to_string()
        } else {
            "mice".to_string()
        };
    }
    if lower.ends_with('s')
        || lower.ends_with('x')
        || lower.ends_with('z')
        || lower.ends_with("ch")
        || lower.ends_with("sh")
    {
        format!("{word}es")
    } else if lower.ends_with('y')
        && lower.len() > 1
        && !matches!(
            lower.chars().nth(lower.len() - 2),
            Some('a' | 'e' | 'i' | 'o' | 'u')
        )
    {
        format!("{}ies", &word[..word.len() - 1])
    } else {
        format!("{word}s")
    }
}

fn indefinite_article_for(word: &str) -> &'static str {
    match word.chars().next().map(|ch| ch.to_ascii_lowercase()) {
        Some('a' | 'e' | 'i' | 'o' | 'u') => "an",
        _ => "a",
    }
}

fn pluralize_terminal_word(phrase: &str) -> String {
    if let Some((head, tail)) = phrase.rsplit_once(' ') {
        if head.trim().is_empty() {
            simple_pluralize(tail)
        } else {
            format!("{head} {}", simple_pluralize(tail))
        }
    } else {
        simple_pluralize(phrase)
    }
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

fn flatten_static_condition_and(
    condition: &crate::ConditionExpr,
    out: &mut Vec<crate::ConditionExpr>,
) {
    match condition {
        crate::ConditionExpr::And(left, right) => {
            flatten_static_condition_and(left, out);
            flatten_static_condition_and(right, out);
        }
        _ => out.push(condition.clone()),
    }
}

fn describe_static_condition(condition: &crate::ConditionExpr) -> String {
    match condition {
        crate::ConditionExpr::And(_, _) => {
            let mut clauses = Vec::new();
            flatten_static_condition_and(condition, &mut clauses);
            let described = clauses
                .iter()
                .map(describe_static_condition)
                .collect::<Vec<_>>();
            if described
                .iter()
                .all(|clause| clause.starts_with("as long as "))
            {
                let joined = described
                    .iter()
                    .map(|clause| clause.trim_start_matches("as long as "))
                    .collect::<Vec<_>>()
                    .join(" and ");
                return format!("as long as {joined}");
            }
            return described.join(" and ");
        }
        crate::ConditionExpr::YourTurn => "as long as it's your turn".to_string(),
        crate::ConditionExpr::Not(inner)
            if matches!(inner.as_ref(), crate::ConditionExpr::YourTurn) =>
        {
            "during turns other than yours".to_string()
        }
        crate::ConditionExpr::SourceIsEquipped => {
            "as long as this creature is equipped".to_string()
        }
        crate::ConditionExpr::SourceIsEnchanted => {
            "as long as this creature is enchanted".to_string()
        }
        crate::ConditionExpr::EnchantedPermanentIsCreature => {
            "as long as enchanted permanent is a creature".to_string()
        }
        crate::ConditionExpr::EnchantedPermanentIsEquipment => {
            "as long as enchanted permanent is an equipment".to_string()
        }
        crate::ConditionExpr::EnchantedPermanentIsVehicle => {
            "as long as enchanted permanent is a vehicle".to_string()
        }
        crate::ConditionExpr::EquippedCreatureTapped => {
            "as long as equipped creature is tapped".to_string()
        }
        crate::ConditionExpr::EquippedCreatureUntapped => {
            "as long as equipped creature is untapped".to_string()
        }
        crate::ConditionExpr::SourceIsAttacking => {
            "as long as this creature is attacking".to_string()
        }
        crate::ConditionExpr::SourceIsUntapped => {
            "as long as this creature is untapped".to_string()
        }
        crate::ConditionExpr::SourceIsSoulbondPaired => {
            "as long as this creature is paired with another creature".to_string()
        }
        crate::ConditionExpr::PlayerHasCardTypesInGraveyardOrMore { player, count } => {
            let graveyard_owner = match player {
                crate::target::PlayerFilter::You => "your".to_string(),
                crate::target::PlayerFilter::Opponent => "an opponent's".to_string(),
                crate::target::PlayerFilter::Any => "a player's".to_string(),
                _ => "that player's".to_string(),
            };
            format!(
                "as long as there are {count} or more card types among cards in {graveyard_owner} graveyard"
            )
        }
        crate::ConditionExpr::CountComparison {
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
        crate::ConditionExpr::Unmodeled(text) if !text.is_empty() => format!("as long as {text}"),
        _ => format!("as long as {condition:?}"),
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

pub(crate) fn resolve_anthem_count_expression(
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
    condition: &crate::ConditionExpr,
    game: &GameState,
    source: ObjectId,
    controller: PlayerId,
) -> bool {
    let eval_ctx = crate::condition_eval::ExternalEvaluationContext {
        controller,
        source,
        defending_player: None,
        attacking_player: None,
        filter_source: Some(source),
        triggering_event: None,
        trigger_identity: None,
        ability_index: None,
        options: Default::default(),
    };
    crate::condition_eval::evaluate_condition_external(game, condition, &eval_ctx)
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
    pub condition: Option<crate::ConditionExpr>,
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

    pub fn with_condition(mut self, condition: crate::ConditionExpr) -> Self {
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
        let subject_mentions_plural = subject.contains("creatures")
            || subject.contains("tokens")
            || subject.contains("permanents")
            || subject.contains("artifacts")
            || subject.contains("enchantments")
            || subject.contains("lands")
            || subject.contains("planeswalkers")
            || subject.contains("battles")
            || subject.contains("spells")
            || subject.contains("cards")
            || subject.contains("allies");
        let singular = self.source_only
            || subject.starts_with("a ")
            || subject.starts_with("an ")
            || subject.starts_with("this ")
            || subject.starts_with("that ")
            || ((subject.starts_with("enchanted ") || subject.starts_with("equipped "))
                && !subject_mentions_plural);
        let verb = if singular { "gets" } else { "get" };

        let signed = |value: i32| {
            if value >= 0 {
                format!("+{value}")
            } else {
                value.to_string()
            }
        };
        let signed_toughness = |power: i32, toughness: i32| {
            if power < 0 && toughness == 0 {
                "-0".to_string()
            } else {
                signed(toughness)
            }
        };

        let mut text = match (&self.power, &self.toughness) {
            (AnthemValue::Fixed(power), AnthemValue::Fixed(toughness)) => {
                format!(
                    "{subject} {verb} {}/{}",
                    signed(*power),
                    signed_toughness(*power, *toughness),
                )
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
                    signed_toughness(*power, *toughness),
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
                signed_toughness(*power, *toughness),
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
                signed_toughness(*power, *toughness),
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

    fn with_static_condition(&self, condition: crate::ConditionExpr) -> Option<StaticAbility> {
        Some(StaticAbility::new(self.clone().with_condition(condition)))
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
    pub condition: Option<crate::ConditionExpr>,
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

    pub fn with_condition(mut self, condition: crate::ConditionExpr) -> Self {
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

    fn with_static_condition(&self, condition: crate::ConditionExpr) -> Option<StaticAbility> {
        Some(StaticAbility::new(self.clone().with_condition(condition)))
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

#[derive(Debug, Clone, PartialEq)]
pub enum SoulbondSharedMode {
    PowerToughness { power: i32, toughness: i32 },
    Ability(StaticAbility),
    ObjectAbility(Ability),
}

#[derive(Debug, Clone, PartialEq)]
pub struct SoulbondSharedBonus {
    pub mode: SoulbondSharedMode,
}

impl SoulbondSharedBonus {
    pub fn power_toughness(power: i32, toughness: i32) -> Self {
        Self {
            mode: SoulbondSharedMode::PowerToughness { power, toughness },
        }
    }

    pub fn ability(ability: StaticAbility) -> Self {
        Self {
            mode: SoulbondSharedMode::Ability(ability),
        }
    }

    pub fn object_ability(ability: Ability) -> Self {
        Self {
            mode: SoulbondSharedMode::ObjectAbility(ability),
        }
    }
}

impl StaticAbilityKind for SoulbondSharedBonus {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::SoulbondSharedBonus
    }

    fn display(&self) -> String {
        match &self.mode {
            SoulbondSharedMode::PowerToughness { power, toughness } => {
                let signed = |value: i32| {
                    if value >= 0 {
                        format!("+{value}")
                    } else {
                        value.to_string()
                    }
                };
                format!(
                    "As long as this creature is paired with another creature, each of those creatures gets {}/{}",
                    signed(*power),
                    signed(*toughness)
                )
            }
            SoulbondSharedMode::Ability(ability) => format!(
                "As long as this creature is paired with another creature, both creatures have {}",
                ability.display()
            ),
            SoulbondSharedMode::ObjectAbility(ability) => {
                let text = ability
                    .text
                    .clone()
                    .unwrap_or_else(|| "an ability".to_string());
                format!(
                    "As long as this creature is paired with another creature, both creatures have \"{}\"",
                    text
                )
            }
        }
    }

    fn generate_effects(
        &self,
        source: ObjectId,
        controller: PlayerId,
        game: &GameState,
    ) -> Vec<ContinuousEffect> {
        let Some(partner) = game.soulbond_partner(source) else {
            return Vec::new();
        };

        let modification = |mode: &SoulbondSharedMode| match mode {
            SoulbondSharedMode::PowerToughness { power, toughness } => {
                Modification::ModifyPowerToughness {
                    power: *power,
                    toughness: *toughness,
                }
            }
            SoulbondSharedMode::Ability(ability) => Modification::AddAbility(ability.clone()),
            SoulbondSharedMode::ObjectAbility(ability) => {
                Modification::AddAbilityGeneric(ability.clone())
            }
        };

        vec![
            ContinuousEffect::new(
                source,
                controller,
                EffectTarget::Specific(source),
                modification(&self.mode),
            )
            .with_source_type(EffectSourceType::StaticAbility),
            ContinuousEffect::new(
                source,
                controller,
                EffectTarget::Specific(partner),
                modification(&self.mode),
            )
            .with_source_type(EffectSourceType::StaticAbility),
        ]
    }

    fn apply_restrictions(&self, game: &mut GameState, source: ObjectId, controller: PlayerId) {
        let SoulbondSharedMode::Ability(ability) = &self.mode else {
            return;
        };
        let Some(partner) = game.soulbond_partner(source) else {
            return;
        };
        ability.apply_restrictions(game, source, controller);
        ability.apply_restrictions(game, partner, controller);
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
        let subject = pluralized_subject_text(&self.filter);
        let singular_subject = subject.starts_with("enchanted ")
            || subject.starts_with("equipped ")
            || subject.starts_with("this ")
            || subject.starts_with("that ");
        let verb = if singular_subject { "loses" } else { "lose" };
        format!("{subject} {verb} {}", self.ability.display())
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
        format!(
            "{} lose all abilities",
            pluralized_subject_text(&self.filter)
        )
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

/// Remove all non-mana abilities: "Lands lose all abilities except mana abilities"
#[derive(Debug, Clone, PartialEq)]
pub struct RemoveAllAbilitiesExceptManaForFilter {
    /// Filter for which permanents lose non-mana abilities.
    pub filter: ObjectFilter,
}

impl RemoveAllAbilitiesExceptManaForFilter {
    pub fn new(filter: ObjectFilter) -> Self {
        Self { filter }
    }
}

impl StaticAbilityKind for RemoveAllAbilitiesExceptManaForFilter {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::RemoveAllAbilitiesExceptManaForFilter
    }

    fn display(&self) -> String {
        format!(
            "{} lose all abilities except mana abilities",
            pluralized_subject_text(&self.filter)
        )
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
                Modification::RemoveAllAbilitiesExceptMana,
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
    pub condition: Option<crate::ConditionExpr>,
}

impl SetBasePowerToughnessForFilter {
    pub fn new(filter: ObjectFilter, power: i32, toughness: i32) -> Self {
        Self {
            filter,
            power,
            toughness,
            condition: None,
        }
    }

    pub fn with_condition(mut self, condition: crate::ConditionExpr) -> Self {
        self.condition = Some(condition);
        self
    }
}

impl StaticAbilityKind for SetBasePowerToughnessForFilter {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::SetBasePowerToughnessForFilter
    }

    fn display(&self) -> String {
        let subject = pluralized_subject_text(&self.filter);
        let singular = subject.starts_with("enchanted ")
            || subject.starts_with("equipped ")
            || subject.starts_with("this ")
            || subject.starts_with("that ");
        let verb = if singular { "has" } else { "have" };
        let mut text = format!(
            "{subject} {verb} base power and toughness {}/{}",
            self.power, self.toughness
        );
        if let Some(condition) = &self.condition {
            text.push(' ');
            text.push_str(&describe_static_condition(condition));
        }
        text
    }

    fn with_static_condition(&self, condition: crate::ConditionExpr) -> Option<StaticAbility> {
        Some(StaticAbility::new(self.clone().with_condition(condition)))
    }

    fn generate_effects(
        &self,
        source: ObjectId,
        controller: PlayerId,
        game: &GameState,
    ) -> Vec<ContinuousEffect> {
        if let Some(condition) = &self.condition
            && !static_condition_is_active(condition, game, source, controller)
        {
            return Vec::new();
        }
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

/// Copy activated abilities from objects matching a filter.
#[derive(Debug, Clone, PartialEq)]
pub struct CopyActivatedAbilities {
    pub filter: ObjectFilter,
    pub counter: Option<CounterType>,
    pub include_mana: bool,
    pub exclude_source_name: bool,
    pub exclude_source_id: bool,
    pub condition: Option<crate::ConditionExpr>,
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

    pub fn with_condition(mut self, condition: crate::ConditionExpr) -> Self {
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

    fn with_static_condition(&self, condition: crate::ConditionExpr) -> Option<StaticAbility> {
        Some(StaticAbility::new(self.clone().with_condition(condition)))
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

        let eval_ctx = crate::condition_eval::ExternalEvaluationContext {
            controller,
            source,
            defending_player: None,
            attacking_player: None,
            filter_source: Some(source),
            triggering_event: None,
            trigger_identity: None,
            ability_index: None,
            options: Default::default(),
        };
        crate::condition_eval::evaluate_condition_external(game, condition, &eval_ctx)
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
    pub condition: Option<crate::ConditionExpr>,
}

impl SetColorsForFilter {
    pub fn new(filter: ObjectFilter, colors: crate::color::ColorSet) -> Self {
        Self {
            filter,
            colors,
            condition: None,
        }
    }

    pub fn with_condition(mut self, condition: crate::ConditionExpr) -> Self {
        self.condition = Some(condition);
        self
    }
}

impl PartialEq for SetColorsForFilter {
    fn eq(&self, other: &Self) -> bool {
        self.filter == other.filter
            && self.colors == other.colors
            && self.condition == other.condition
    }
}

impl StaticAbilityKind for SetColorsForFilter {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::SetColors
    }

    fn display(&self) -> String {
        let subject = pluralized_subject_text(&self.filter);
        let (verb, _) = subject_verb_and_possessive(&subject);
        let colors = join_with_and(&color_list(self.colors));
        let mut text = format!("{subject} {verb} {colors}");
        if let Some(condition) = &self.condition {
            text.push(' ');
            text.push_str(&describe_static_condition(condition));
        }
        text
    }

    fn with_static_condition(&self, condition: crate::ConditionExpr) -> Option<StaticAbility> {
        Some(StaticAbility::new(self.clone().with_condition(condition)))
    }

    fn generate_effects(
        &self,
        source: ObjectId,
        controller: PlayerId,
        game: &GameState,
    ) -> Vec<ContinuousEffect> {
        if let Some(condition) = &self.condition
            && !static_condition_is_active(condition, game, source, controller)
        {
            return Vec::new();
        }
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

/// Set name: "Enchanted creature is named Legitimate Businessperson."
#[derive(Debug, Clone, PartialEq)]
pub struct SetNameForFilter {
    pub filter: ObjectFilter,
    pub name: String,
}

impl SetNameForFilter {
    pub fn new(filter: ObjectFilter, name: String) -> Self {
        Self { filter, name }
    }
}

impl StaticAbilityKind for SetNameForFilter {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::SetName
    }

    fn display(&self) -> String {
        let subject = pluralized_subject_text(&self.filter);
        let (verb, _) = subject_verb_and_possessive(&subject);
        format!("{subject} {verb} named {}", self.name)
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
                Modification::SetName(self.name.clone()),
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
        let subject = pluralized_subject_text(&self.filter);
        let (verb, possessive) = subject_verb_and_possessive(&subject);
        let colors = join_with_and(&color_list(self.colors));
        format!("{subject} {verb} {colors} in addition to {possessive} other colors")
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
    pub condition: Option<crate::ConditionExpr>,
}

impl AddCardTypesForFilter {
    pub fn new(filter: ObjectFilter, card_types: Vec<CardType>) -> Self {
        Self {
            filter,
            card_types,
            condition: None,
        }
    }

    pub fn with_condition(mut self, condition: crate::ConditionExpr) -> Self {
        self.condition = Some(condition);
        self
    }
}

impl PartialEq for AddCardTypesForFilter {
    fn eq(&self, other: &Self) -> bool {
        self.filter == other.filter
            && self.card_types == other.card_types
            && self.condition == other.condition
    }
}

impl StaticAbilityKind for AddCardTypesForFilter {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::AddCardTypes
    }

    fn display(&self) -> String {
        let subject = pluralized_subject_text(&self.filter);
        let (verb, possessive) = subject_verb_and_possessive(&subject);
        let types = self
            .card_types
            .iter()
            .map(|card_type| card_type.name().to_string())
            .collect::<Vec<_>>();
        let mut text = format!(
            "{subject} {verb} {} in addition to {possessive} other types",
            join_with_and(&types)
        );
        if let Some(condition) = &self.condition {
            text.push(' ');
            text.push_str(&describe_static_condition(condition));
        }
        text
    }

    fn with_static_condition(&self, condition: crate::ConditionExpr) -> Option<StaticAbility> {
        Some(StaticAbility::new(self.clone().with_condition(condition)))
    }

    fn generate_effects(
        &self,
        source: ObjectId,
        controller: PlayerId,
        game: &GameState,
    ) -> Vec<ContinuousEffect> {
        if let Some(condition) = &self.condition
            && !static_condition_is_active(condition, game, source, controller)
        {
            return Vec::new();
        }
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

/// Remove card types: "This creature isn't a creature."
#[derive(Debug, Clone)]
pub struct RemoveCardTypesForFilter {
    pub filter: ObjectFilter,
    pub card_types: Vec<CardType>,
    pub condition: Option<crate::ConditionExpr>,
}

impl RemoveCardTypesForFilter {
    pub fn new(filter: ObjectFilter, card_types: Vec<CardType>) -> Self {
        Self {
            filter,
            card_types,
            condition: None,
        }
    }

    pub fn with_condition(mut self, condition: crate::ConditionExpr) -> Self {
        self.condition = Some(condition);
        self
    }
}

impl PartialEq for RemoveCardTypesForFilter {
    fn eq(&self, other: &Self) -> bool {
        self.filter == other.filter
            && self.card_types == other.card_types
            && self.condition == other.condition
    }
}

impl StaticAbilityKind for RemoveCardTypesForFilter {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::RemoveCardTypes
    }

    fn display(&self) -> String {
        let subject = pluralized_subject_text(&self.filter);
        let (verb, _) = subject_verb_and_possessive(&subject);
        let types = self
            .card_types
            .iter()
            .map(|card_type| card_type.name().to_string())
            .collect::<Vec<_>>();
        let mut text = format!("{subject} {verb} no longer {}", join_with_and(&types));
        if let Some(condition) = &self.condition {
            text.push(' ');
            text.push_str(&describe_static_condition(condition));
        }
        text
    }

    fn with_static_condition(&self, condition: crate::ConditionExpr) -> Option<StaticAbility> {
        Some(StaticAbility::new(self.clone().with_condition(condition)))
    }

    fn generate_effects(
        &self,
        source: ObjectId,
        controller: PlayerId,
        game: &GameState,
    ) -> Vec<ContinuousEffect> {
        if let Some(condition) = &self.condition
            && !static_condition_is_active(condition, game, source, controller)
        {
            return Vec::new();
        }
        vec![
            ContinuousEffect::new(
                source,
                controller,
                effect_target_for_filter(source, &self.filter),
                Modification::RemoveCardTypes(self.card_types.clone()),
            )
            .with_source_type(EffectSourceType::StaticAbility),
        ]
    }
}

/// Set card types: "Enchanted permanent is a creature."
#[derive(Debug, Clone)]
pub struct SetCardTypesForFilter {
    pub filter: ObjectFilter,
    pub card_types: Vec<CardType>,
}

impl SetCardTypesForFilter {
    pub fn new(filter: ObjectFilter, card_types: Vec<CardType>) -> Self {
        Self { filter, card_types }
    }
}

impl PartialEq for SetCardTypesForFilter {
    fn eq(&self, other: &Self) -> bool {
        self.filter == other.filter && self.card_types == other.card_types
    }
}

impl StaticAbilityKind for SetCardTypesForFilter {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::SetCardTypes
    }

    fn display(&self) -> String {
        let subject = pluralized_subject_text(&self.filter);
        let (verb, _) = subject_verb_and_possessive(&subject);
        let types = self
            .card_types
            .iter()
            .map(|card_type| card_type.name().to_string())
            .collect::<Vec<_>>();
        format!("{subject} {verb} {}", join_with_and(&types))
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
                Modification::SetCardTypes(self.card_types.clone()),
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
    pub condition: Option<crate::ConditionExpr>,
}

impl AddSubtypesForFilter {
    pub fn new(filter: ObjectFilter, subtypes: Vec<Subtype>) -> Self {
        Self {
            filter,
            subtypes,
            condition: None,
        }
    }

    pub fn with_condition(mut self, condition: crate::ConditionExpr) -> Self {
        self.condition = Some(condition);
        self
    }
}

impl PartialEq for AddSubtypesForFilter {
    fn eq(&self, other: &Self) -> bool {
        self.filter == other.filter
            && self.subtypes == other.subtypes
            && self.condition == other.condition
    }
}

impl StaticAbilityKind for AddSubtypesForFilter {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::AddSubtypes
    }

    fn display(&self) -> String {
        let subject = pluralized_subject_text(&self.filter);
        let (verb, possessive) = subject_verb_and_possessive(&subject);
        let subtype_words = self
            .subtypes
            .iter()
            .map(std::string::ToString::to_string)
            .collect::<Vec<_>>();
        let base_phrase = subtype_words.join(" ");
        let subtype_phrase = if verb == "are" {
            pluralize_terminal_word(&base_phrase)
        } else if let Some(first) = subtype_words.first() {
            format!("{} {base_phrase}", indefinite_article_for(first))
        } else {
            base_phrase
        };
        let filter_subject = subject_text(&self.filter);
        let (base, suffix) = split_subject_suffix(&filter_subject);
        let filter_is_single_creature_type = suffix.is_empty()
            && !base.contains(' ')
            && base
                .chars()
                .next()
                .is_some_and(|ch| ch.is_ascii_uppercase());
        let adding_creature_types = self.subtypes.iter().all(Subtype::is_creature_type);
        let other_types = if filter_is_single_creature_type && adding_creature_types {
            "other creature types"
        } else {
            "other types"
        };
        let mut text =
            format!("{subject} {verb} {subtype_phrase} in addition to {possessive} {other_types}",);
        if let Some(condition) = &self.condition {
            text.push(' ');
            text.push_str(&describe_static_condition(condition));
        }
        text
    }

    fn with_static_condition(&self, condition: crate::ConditionExpr) -> Option<StaticAbility> {
        Some(StaticAbility::new(self.clone().with_condition(condition)))
    }

    fn generate_effects(
        &self,
        source: ObjectId,
        controller: PlayerId,
        game: &GameState,
    ) -> Vec<ContinuousEffect> {
        if let Some(condition) = &self.condition
            && !static_condition_is_active(condition, game, source, controller)
        {
            return Vec::new();
        }
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

/// Set creature subtypes by removing all creature types first, then adding the new list.
#[derive(Debug, Clone)]
pub struct SetCreatureSubtypesForFilter {
    pub filter: ObjectFilter,
    pub subtypes: Vec<Subtype>,
}

impl SetCreatureSubtypesForFilter {
    pub fn new(filter: ObjectFilter, subtypes: Vec<Subtype>) -> Self {
        Self { filter, subtypes }
    }
}

impl PartialEq for SetCreatureSubtypesForFilter {
    fn eq(&self, other: &Self) -> bool {
        self.filter == other.filter && self.subtypes == other.subtypes
    }
}

impl StaticAbilityKind for SetCreatureSubtypesForFilter {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::SetCreatureSubtypes
    }

    fn display(&self) -> String {
        let subject = pluralized_subject_text(&self.filter);
        let (verb, _) = subject_verb_and_possessive(&subject);
        let subtypes = self
            .subtypes
            .iter()
            .map(|subtype| subtype.to_string().to_ascii_lowercase())
            .collect::<Vec<_>>();
        format!("{subject} {verb} {}", join_with_and(&subtypes))
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
                Modification::RemoveAllCreatureTypes,
            )
            .with_source_type(EffectSourceType::StaticAbility),
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
        if self.filter == ObjectFilter::source() {
            "Devoid".to_string()
        } else {
            "Permanents are colorless".to_string()
        }
    }

    fn is_devoid(&self) -> bool {
        self.filter == ObjectFilter::source()
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

/// Add supertypes: "Enchanted creature is legendary."
#[derive(Debug, Clone)]
pub struct AddSupertypesForFilter {
    pub filter: ObjectFilter,
    pub supertypes: Vec<Supertype>,
}

impl AddSupertypesForFilter {
    pub fn new(filter: ObjectFilter, supertypes: Vec<Supertype>) -> Self {
        Self { filter, supertypes }
    }
}

impl PartialEq for AddSupertypesForFilter {
    fn eq(&self, other: &Self) -> bool {
        self.filter == other.filter && self.supertypes == other.supertypes
    }
}

impl StaticAbilityKind for AddSupertypesForFilter {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::AddSupertypes
    }

    fn display(&self) -> String {
        let mut subject = pluralized_subject_text(&self.filter);
        if subject == "lands" {
            subject = "land".to_string();
        }
        let singular = subject.starts_with("enchanted ")
            || subject.starts_with("equipped ")
            || subject.starts_with("this ")
            || subject.starts_with("that ")
            || subject == "land";
        let verb = if singular { "is" } else { "are" };
        let supertypes = self
            .supertypes
            .iter()
            .map(|supertype| supertype.name().to_string())
            .collect::<Vec<_>>()
            .join(" and ");
        format!("{subject} {verb} {supertypes}")
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
                Modification::AddSupertypes(self.supertypes.clone()),
            )
            .with_source_type(EffectSourceType::StaticAbility),
        ]
    }
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
        let mut subject = pluralized_subject_text(&self.filter);
        if subject == "lands" {
            subject = "land".to_string();
        }
        let singular = subject.starts_with("enchanted ")
            || subject.starts_with("equipped ")
            || subject.starts_with("this ")
            || subject.starts_with("that ")
            || subject == "land";
        let verb = if singular { "is" } else { "are" };
        let supertypes = self
            .supertypes
            .iter()
            .map(|supertype| supertype.name().to_string())
            .collect::<Vec<_>>()
            .join(" and ");
        format!("{subject} {verb} no longer {supertypes}")
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
    pub condition: Option<crate::ConditionExpr>,
}

impl AttachedAbilityGrant {
    pub fn new(ability: Ability, display: String) -> Self {
        Self {
            ability,
            display,
            condition: None,
        }
    }

    pub fn with_condition(mut self, condition: crate::ConditionExpr) -> Self {
        self.condition = Some(condition);
        self
    }
}

impl StaticAbilityKind for AttachedAbilityGrant {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::AttachedAbilityGrant
    }

    fn display(&self) -> String {
        let mut text = self.display.clone();
        if let Some(condition) = &self.condition {
            text.push(' ');
            text.push_str(&describe_static_condition(condition));
        }
        text
    }

    fn with_static_condition(&self, condition: crate::ConditionExpr) -> Option<StaticAbility> {
        Some(StaticAbility::new(self.clone().with_condition(condition)))
    }

    fn granted_inline_ability(&self) -> Option<&crate::ability::Ability> {
        Some(&self.ability)
    }

    fn generate_effects(
        &self,
        source: ObjectId,
        controller: PlayerId,
        game: &GameState,
    ) -> Vec<ContinuousEffect> {
        if let Some(condition) = &self.condition
            && !static_condition_is_active(condition, game, source, controller)
        {
            return Vec::new();
        }
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

/// Controller of source controls the permanent attached to source.
#[derive(Debug, Clone, PartialEq)]
pub struct ControlAttachedPermanent {
    pub display: String,
}

impl ControlAttachedPermanent {
    pub fn new(display: String) -> Self {
        Self { display }
    }
}

impl StaticAbilityKind for ControlAttachedPermanent {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::ControlAttachedPermanent
    }

    fn display(&self) -> String {
        self.display.clone()
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
                Modification::ChangeController(controller),
            )
            .with_source_type(EffectSourceType::StaticAbility),
        ]
    }
}

/// "Enchanted land is the chosen type."
#[derive(Debug, Clone, PartialEq)]
pub struct EnchantedLandIsChosenType {
    pub display: String,
}

impl EnchantedLandIsChosenType {
    pub fn new(display: String) -> Self {
        Self { display }
    }
}

impl StaticAbilityKind for EnchantedLandIsChosenType {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::EnchantedLandIsChosenType
    }

    fn display(&self) -> String {
        self.display.clone()
    }

    fn generate_effects(
        &self,
        source: ObjectId,
        controller: PlayerId,
        game: &GameState,
    ) -> Vec<ContinuousEffect> {
        let Some(chosen_type) = game.chosen_basic_land_type(source) else {
            return Vec::new();
        };

        vec![
            ContinuousEffect::new(
                source,
                controller,
                EffectTarget::AttachedTo(source),
                Modification::SetSubtypes(vec![chosen_type]),
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
    pub condition: Option<crate::ConditionExpr>,
}

impl GrantObjectAbilityForFilter {
    pub fn new(filter: ObjectFilter, ability: Ability, display: String) -> Self {
        Self {
            filter,
            ability,
            display,
            condition: None,
        }
    }

    pub fn with_condition(mut self, condition: crate::ConditionExpr) -> Self {
        self.condition = Some(condition);
        self
    }
}

impl StaticAbilityKind for GrantObjectAbilityForFilter {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::GrantObjectAbilityForFilter
    }

    fn display(&self) -> String {
        self.display.clone()
    }

    fn with_static_condition(&self, condition: crate::ConditionExpr) -> Option<StaticAbility> {
        Some(StaticAbility::new(self.clone().with_condition(condition)))
    }

    fn granted_inline_ability(&self) -> Option<&crate::ability::Ability> {
        Some(&self.ability)
    }

    fn generate_effects(
        &self,
        source: ObjectId,
        controller: PlayerId,
        game: &GameState,
    ) -> Vec<ContinuousEffect> {
        if let Some(condition) = &self.condition
            && !static_condition_is_active(condition, game, source, controller)
        {
            return Vec::new();
        }
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

        let anthem =
            Anthem::for_source(2, 2).with_condition(crate::ConditionExpr::CountComparison {
                count: AnthemCountExpression::MatchingFilter(
                    ObjectFilter::artifact().you_control(),
                ),
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
    fn test_control_attached_permanent_changes_controller() {
        let ability = ControlAttachedPermanent::new("You control enchanted creature.".to_string());
        assert_eq!(ability.id(), StaticAbilityId::ControlAttachedPermanent);
        assert_eq!(ability.display(), "You control enchanted creature.");

        let game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let source = ObjectId::from_raw(1);
        let controller = PlayerId::from_index(0);
        let effects = ability.generate_effects(source, controller, &game);
        assert_eq!(effects.len(), 1);
        assert!(matches!(
            effects[0].applies_to,
            EffectTarget::AttachedTo(id) if id == source
        ));
        assert!(matches!(
            effects[0].modification,
            Modification::ChangeController(player) if player == controller
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
    fn test_remove_all_abilities_except_mana_for_filter() {
        let ability = RemoveAllAbilitiesExceptManaForFilter::new(ObjectFilter::land());
        assert_eq!(
            ability.id(),
            StaticAbilityId::RemoveAllAbilitiesExceptManaForFilter
        );

        let game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let effects =
            ability.generate_effects(ObjectId::from_raw(1), PlayerId::from_index(0), &game);
        assert_eq!(effects.len(), 1);
        assert!(matches!(
            effects[0].modification,
            Modification::RemoveAllAbilitiesExceptMana
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
