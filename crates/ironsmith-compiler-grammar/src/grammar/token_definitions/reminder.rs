use crate::cards::builders::TagKey;
use crate::effect::Value;
use crate::lexer::{LexStream, OwnedLexToken, parser_token_word_refs};
use crate::model::token_definition::{CreatureTokenRulesShape, TokenKeywordShape};
use crate::target::{ChooseSpec, PlayerFilter};
use winnow::combinator::alt;
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;

use super::super::effects;
use super::super::primitives;
use super::super::shared_util::value_expr;
use super::{common, equipment, rules, surface};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenReminderSentenceKind {
    GrantedAbility,
    PronounTrigger,
    PowerToughness,
    DelayedLifecycle,
    ExplicitTokenReference,
}

const DELAYED_LIFECYCLE_TIMING_PHRASES: &[&[&str]] = &[
    &["beginning", "of", "your", "next", "end", "step"],
    &["beginning", "of", "the", "end", "step"],
    &["beginning", "of", "next", "end", "step"],
    &["beginning", "of", "the", "next", "end", "step"],
    &["end", "of", "combat"],
];

fn token_reminder_sentence_head<'a>(
    input: &mut LexStream<'a>,
) -> WResult<TokenReminderSentenceKind> {
    alt((
        alt((
            primitives::phrase(&["it", "has"]),
            primitives::phrase(&["they", "have"]),
        ))
        .value(TokenReminderSentenceKind::GrantedAbility),
        alt((
            primitives::phrase(&["when", "it"]),
            primitives::phrase(&["whenever", "it"]),
            primitives::phrase(&["when", "they"]),
            primitives::phrase(&["whenever", "they"]),
        ))
        .value(TokenReminderSentenceKind::PronounTrigger),
        alt((
            primitives::phrase(&["its", "power"]),
            primitives::phrase(&["its", "toughness"]),
            // The lexer removes possessive apostrophes, so "token's" is the
            // parser-word piece "tokens". The token-level grammar still sees
            // the normalized lexical token "token's".
            primitives::phrase(&["this", "token's", "power"]),
            primitives::phrase(&["this", "token's", "toughness"]),
            primitives::phrase(&["this", "tokens", "power"]),
            primitives::phrase(&["this", "tokens", "toughness"]),
        ))
        .value(TokenReminderSentenceKind::PowerToughness),
        alt((primitives::kw("exile"), primitives::kw("sacrifice")))
            .value(TokenReminderSentenceKind::DelayedLifecycle),
        alt((
            primitives::phrase(&["when", "this", "token"]),
            primitives::phrase(&["whenever", "this", "token"]),
            primitives::phrase(&["this", "token"]),
            primitives::phrase(&["those", "tokens"]),
        ))
        .value(TokenReminderSentenceKind::ExplicitTokenReference),
    ))
    .parse_next(input)
}

fn contains_reminder_reference(tokens: &[OwnedLexToken]) -> bool {
    primitives::find_prefix(tokens, || {
        alt((
            primitives::kw("token"),
            primitives::kw("tokens"),
            primitives::kw("it"),
            primitives::kw("them"),
        ))
    })
    .is_some()
}

fn contains_delayed_lifecycle_timing(tokens: &[OwnedLexToken]) -> bool {
    DELAYED_LIFECYCLE_TIMING_PHRASES
        .iter()
        .copied()
        .any(|phrase| primitives::find_prefix(tokens, || primitives::phrase(phrase)).is_some())
}

pub fn parse_token_reminder_sentence_kind_tokens(
    tokens: &[OwnedLexToken],
) -> Option<TokenReminderSentenceKind> {
    let (kind, _) = primitives::parse_prefix(tokens, token_reminder_sentence_head)?;
    if kind == TokenReminderSentenceKind::DelayedLifecycle
        && (!contains_delayed_lifecycle_timing(tokens) || !contains_reminder_reference(tokens))
    {
        return None;
    }
    Some(kind)
}

/// Distinguish the authored outer grant verb from words inside a quoted token
/// ability. For example, `They have "When this token dies, you gain 1 life"`
/// uses `have` even though the quoted rule contains `gain`.
pub fn token_ability_sentence_uses_gain_verb(tokens: &[OwnedLexToken]) -> bool {
    let words = parser_token_word_refs(tokens);
    crate::word_primitives::parse_any_sequence_prefix(
        &words,
        &[
            &["it", "gains"],
            &["they", "gain"],
            &["that", "token", "gains"],
            &["those", "tokens", "gain"],
        ],
    )
}

#[derive(Debug, Clone, PartialEq)]
pub struct TokenReminderFacts {
    pub dynamic_power_toughness: Option<(Value, Value)>,
    pub has_haste: bool,
    pub exile_at_end_of_combat: bool,
    pub sacrifice_at_end_of_combat: bool,
    pub sacrifice_at_next_end_step: bool,
    pub exile_at_next_end_step: bool,
    pub next_end_step_player: PlayerFilter,
    pub(super) definition: TokenDefinitionReminderFacts,
}

impl TokenReminderFacts {
    /// Expose only the intrinsic creature-combat slot needed to distinguish
    /// two independently quoted token rules. The rest of the parsed reminder
    /// definition remains owned by the token-definition merge layer.
    pub fn creature_combat_restriction(
        &self,
    ) -> Option<&crate::model::token_definition::TokenCombatRestrictionShape> {
        self.definition.creature_rules.combat_restriction.as_ref()
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(super) struct TokenDefinitionReminderFacts {
    pub(super) keywords: Vec<TokenKeywordShape>,
    pub(super) creature_rules: CreatureTokenRulesShape,
    pub(super) equipment_rules: Option<crate::model::token_definition::EquipmentRulesShape>,
    pub(super) artifact_leaves_damage_any_target: Option<i32>,
    pub(super) vehicle_flying: bool,
    pub(super) vehicle_crew_amount: Option<u32>,
}

#[path = "reminder/dynamic_power_toughness.rs"]
mod dynamic_power_toughness;
pub use dynamic_power_toughness::parse_token_dynamic_power_toughness_tokens;
use dynamic_power_toughness::{normalized_reminder_words, parse_dynamic_power_toughness};

pub fn parse_token_reminder_facts_tokens(tokens: &[OwnedLexToken]) -> TokenReminderFacts {
    let raw_words = parser_token_word_refs(tokens);
    let words = normalized_reminder_words(&raw_words);
    let delay = effects::parse_next_end_step_delay_words(&words);
    let end_of_combat = common::phrase_present(&words, &["end", "of", "combat"]);
    let exile_at_end_of_combat = end_of_combat && common::word_present(&words, "exile");
    let sacrifice_at_end_of_combat = end_of_combat && common::word_present(&words, "sacrifice");
    let artifact_leaves_damage_any_target = common::all_words_present(
        &words,
        &[
            "when",
            "token",
            "leaves",
            "battlefield",
            "deals",
            "damage",
            "target",
        ],
    )
    .then(|| rules::damage_amount(&words))
    .flatten();
    let definition = TokenDefinitionReminderFacts {
        keywords: surface::token_keywords(&words),
        creature_rules: surface::creature_rules(tokens, &words, None),
        equipment_rules: equipment::parse_equipment_rules_tokens(tokens),
        artifact_leaves_damage_any_target,
        vehicle_flying: common::word_present(&words, "flying"),
        vehicle_crew_amount: rules::parse_token_crew_shape_words(&words).map(|shape| shape.amount),
    };

    TokenReminderFacts {
        dynamic_power_toughness: parse_dynamic_power_toughness(&words),
        has_haste: common::phrase_exact(&words, &["haste"]),
        exile_at_end_of_combat,
        sacrifice_at_end_of_combat,
        sacrifice_at_next_end_step: delay
            .as_ref()
            .is_some_and(|facts| facts.sacrifice_reference),
        exile_at_next_end_step: delay.as_ref().is_some_and(|facts| facts.exile_reference),
        next_end_step_player: delay.map(|facts| facts.player).unwrap_or(PlayerFilter::Any),
        definition,
    }
}

#[cfg(test)]
#[path = "reminder/tests.rs"]
mod tests;
