use std::ops::Range;

use winnow::combinator::{alt, opt, peek, repeat_till};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::{any, rest};

use crate::color::ColorSet;
use crate::zone::Zone;

use super::super::lexer::{LexStream, OwnedLexToken, TokenKind, TokenWordView, trim_lexed_commas};
use super::{leaf, primitives};

mod addition_shapes;
mod anthem_keyword_shapes;
mod clause_shapes;
mod compound_shapes;
mod condition_quantities;
mod condition_shapes;
mod continuing_shapes;
mod count_shapes;

pub use addition_shapes::{
    TypeColorScope, parse_anthem_and_addition_shape, parse_type_color_addition_shape,
    parse_where_x_y_bindings_shape,
};
pub use anthem_keyword_shapes::{
    AnthemKeywordOrder, AnthemKeywordTrailingConditionError, parse_anthem_keyword_color_segment,
    parse_anthem_keyword_compound_split, parse_anthem_keyword_head, parse_colon_tail_split,
    parse_keyword_before_anthem_shape, split_anthem_keyword_and_have, split_anthem_keyword_and_is,
    split_anthem_keyword_trailing_condition,
};
mod granted_tail_shapes;
mod misc_shapes;
mod soulbond_shapes;
mod special_grant_shapes;
mod static_grant_facts;
mod subject_shapes;
mod tail_static_shapes;

pub use clause_shapes::{
    AnthemPrefixConditionKind, AnthemTailShape, parse_fixed_prefix_condition_shape,
    parse_modifier_shape, parse_prefix_condition_shape, parse_tail_shape,
    parse_word_token_candidates, split_trailing_modifier_maximum,
};
pub use compound_shapes::{
    parse_carried_conditional_anthem_grant, parse_carried_subject_type_addition,
    parse_conditional_anthem_otherwise, parse_conditional_anthem_replacement,
};
pub use condition_shapes::{
    DevotionConditionError, DevotionPlayerKind, ExistentialConditionTail, FixedStaticConditionKind,
    SourceCounterConditionError, parse_blocking_source_condition, parse_conjoined_condition_splits,
    parse_devotion_condition_shape, parse_entered_count_condition,
    parse_existential_condition_shape, parse_fixed_static_condition_kind,
    parse_life_total_or_less_condition, parse_source_counter_condition,
    parse_source_in_graveyard_condition, parse_x_value_at_least_condition,
};
pub use continuing_shapes::{
    ContinuingSegmentShape, parse_continuing_segment_shape, parse_direct_have_tail,
    parse_persistent_anthem_tail_head, split_keyword_and_activated, strip_must_attack_suffix,
};
pub use count_shapes::{
    ForEachSpecialShape, StickerCountKind, parse_compound_count_segments, parse_for_each_rest,
    parse_for_each_special_shape, parse_sticker_count_shape, strip_each_or_every,
};
pub use granted_tail_shapes::{
    GrantedAbilityConditionKind, SpecialGrantedKeyword, parse_granted_ability_candidates,
    parse_granted_subject_facts, parse_special_granted_keyword, split_granted_ability_condition,
    split_type_addition_subject,
};
pub use misc_shapes::{
    parse_equipment_equip_shape, parse_keyword_if_color_shape, parse_trailing_grant_segment,
    split_keyword_if_color_segments, split_trailing_grant_segments,
};
pub use soulbond_shapes::{SoulbondSharedEffect, parse_soulbond_shared_shape};
pub use special_grant_shapes::{
    parse_anthem_goaded_shape, parse_anthem_no_defender_grant_tokens,
    parse_colored_spell_protection_tokens, parse_commander_creature_subject_tokens,
    parse_no_defender_granted_fragment_tokens, parse_subject_color_and_grant_tokens,
    parse_unblockable_keyword_fragment_tokens,
};
pub use static_grant_facts::{
    GrantedAlternativeCastKeyword, parse_every_subtype_family_tokens,
    parse_granted_alternative_cast_keyword_tokens, parse_static_grant_duration_fact,
};
pub use subject_shapes::{
    AnthemSubjectGrammarMatch, object_filter_specificity_score, parse_exact_anthem_subject_grammar,
};
pub use tail_static_shapes::{
    BasePowerToughnessConditionShape, IsntCreatureShapeError, parse_base_power_grant_shape,
    parse_base_power_toughness_grant_shape, parse_base_power_toughness_shape,
    parse_base_power_toughness_type_addition_shape, parse_isnt_creature_shape,
    parse_multi_subject_segments, persistent_anthem_subject_facts,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TurnThresholdPlayer {
    You,
    Opponent,
    Any,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TurnThreshold {
    pub player: TurnThresholdPlayer,
    pub count: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceCounterCountClause<'a> {
    pub counter_type_word: &'a str,
    pub source_tokens: &'a [OwnedLexToken],
    pub starts_with_source_pronoun: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FirstSpellEachTurnClause<'a> {
    pub filter_tokens: &'a [OwnedLexToken],
    pub mana_source_tokens: Option<&'a [OwnedLexToken]>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CantBeBlockedAsLongAsClause<'a> {
    pub subject_tokens: &'a [OwnedLexToken],
    pub condition_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CantBeBlockedClause<'a> {
    pub subject_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeywordsAndCantBeBlockedClause<'a> {
    pub keyword_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeywordsAndCantBeBlockedByMoreThanClause<'a> {
    pub subject_tokens: &'a [OwnedLexToken],
    pub keyword_tokens: &'a [OwnedLexToken],
    pub blocker_threshold_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CantBeBlockedAndHasKeywordsClause<'a> {
    pub subject_tokens: &'a [OwnedLexToken],
    pub keyword_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LandwalkBlockOverrideClause<'a> {
    pub subject_tokens: &'a [OwnedLexToken],
    pub ability_word: &'a str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GrantedEscapeCostTail<'a> {
    pub exile_count_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GrantedMiracleCostReductionTail<'a> {
    pub reduction_cost_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CantBeBlockedByMoreThanClause<'a> {
    pub subject_tokens: &'a [OwnedLexToken],
    pub blocker_threshold_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CanBlockAdditionalCreatureClause<'a> {
    pub subject_tokens: &'a [OwnedLexToken],
    pub additional_count_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CantBeBlockedSubjectFacts {
    pub has_conjunction_or_comma: bool,
    pub starts_with_source_pronoun: bool,
    pub has_rejected_clause_word: bool,
    pub mentions_power_or_toughness: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HasKeywordUnblockableHead {
    pub has_token: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GrantedKeywordVerbFacts {
    pub have_token: usize,
    pub prefix_has_get: bool,
    pub starts_with_as_long_as: bool,
    pub tail_has_have: bool,
    pub tail_has_get_or_be: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrailingAsLongAsClause<'a> {
    pub keyword_tokens: &'a [OwnedLexToken],
    pub condition_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MustAttackKeywordTail<'a> {
    pub keyword_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GrantedKeywordTokenKind {
    Blitz,
    Emerge,
    Scavenge,
    Exploit,
    IgnoredReminder,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SubjectLosesKeywordsClause<'a> {
    pub subject_tokens: &'a [OwnedLexToken],
    pub loss_tokens: &'a [OwnedLexToken],
    pub additional_gain_tokens: Option<&'a [OwnedLexToken]>,
    pub loss_mode: ironsmith_core::AbilityLossMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EachCreatureSubject<'a> {
    pub filter_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IndestructibleGrantClause<'a> {
    pub subject_tokens: &'a [OwnedLexToken],
    pub ability_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoseAllTransformShape {
    pub subject_word_end: usize,
    pub descriptor_words: Range<usize>,
    pub power_toughness_word: usize,
    pub name_words: Option<Range<usize>>,
    pub except_mana_abilities: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LoseAllAbilitiesShape {
    pub subject_word_end: usize,
    pub becomes: bool,
    pub except_mana_abilities: bool,
    pub base_power_toughness_word: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeywordTypeAdditionSplit<'a> {
    pub keyword_tokens: &'a [OwnedLexToken],
    pub addition_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SuffixFilterHead {
    Other,
    Pronoun,
    Normal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SharedSuffixCandidate {
    pub and_token: usize,
    pub split_token: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermanentCardOwner {
    You,
    Opponent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PermanentCardCountFacts {
    pub zone: Zone,
    pub owner: Option<PermanentCardOwner>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConditionalMustBlockTarget {
    Source,
    EnchantedCreature,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConditionalMustBlockShape<'a> {
    pub condition_tokens: &'a [OwnedLexToken],
    pub target: ConditionalMustBlockTarget,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NoDefenderConditionalShape<'a> {
    pub subject_tokens: &'a [OwnedLexToken],
    pub condition_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NoDefenderSubjectShape<'a> {
    pub subject_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GetsAttacksShape {
    pub get_token: usize,
    pub and_token: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnthemGrantedTailKind {
    CantBeBlocked,
    BeEverySubtype(crate::types::SubtypeFamily),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnthemAndGrantedTail {
    pub get_token: usize,
    pub and_token: usize,
    pub tail_kind: AnthemGrantedTailKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SubjectEverySubtypeShape<'a> {
    pub condition_tokens: Option<&'a [OwnedLexToken]>,
    pub subject_tokens: &'a [OwnedLexToken],
    pub family: crate::types::SubtypeFamily,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnthemModifierHead {
    pub get_token: usize,
    pub modifier_token: usize,
    pub has_target: bool,
    pub temporary: bool,
}

const CANT_BE_BLOCKED_PHRASES: &[&[&str]] = &[
    &["cant", "be", "blocked"],
    &["can't", "be", "blocked"],
    &["cannot", "be", "blocked"],
    &["can", "t", "be", "blocked"],
];
const CANT_BE_BLOCKED_AS_LONG_AS_PHRASES: &[&[&str]] = &[
    &["cant", "be", "blocked", "as", "long", "as"],
    &["can't", "be", "blocked", "as", "long", "as"],
    &["cannot", "be", "blocked", "as", "long", "as"],
    &["can", "t", "be", "blocked", "as", "long", "as"],
];
const AND_CANT_BE_BLOCKED_PHRASES: &[&[&str]] = &[
    &["and", "cant", "be", "blocked"],
    &["and", "can't", "be", "blocked"],
    &["and", "cannot", "be", "blocked"],
    &["and", "can", "t", "be", "blocked"],
];
const CAN_BE_BLOCKED_AS_THOUGH_NO_ABILITY_PHRASES: &[&[&str]] = &[
    &[
        "can", "be", "blocked", "as", "though", "they", "didnt", "have",
    ],
    &[
        "can", "be", "blocked", "as", "though", "they", "didn't", "have",
    ],
];
const CANT_BE_BLOCKED_BY_PHRASES: &[&[&str]] = &[
    &["cant", "be", "blocked", "by"],
    &["can't", "be", "blocked", "by"],
    &["cannot", "be", "blocked", "by"],
    &["can", "t", "be", "blocked", "by"],
];
const GRANTED_ESCAPE_COST_PREFIX_PHRASES: &[&[&str]] = &[
    &[
        "the", "escape", "cost", "is", "equal", "to", "the", "cards", "mana", "cost", "plus",
    ],
    &[
        "the", "escape", "cost", "is", "equal", "to", "the", "card's", "mana", "cost", "plus",
    ],
    &[
        "the", "escape", "cost", "is", "equal", "to", "the", "card’s", "mana", "cost", "plus",
    ],
    &[
        "its", "escape", "cost", "is", "equal", "to", "its", "mana", "cost", "plus",
    ],
];
const GRANTED_ESCAPE_EXILE_TAIL_PHRASES: &[&[&str]] = &[
    &["other", "cards", "from", "your", "graveyard"],
    &["other", "card", "from", "your", "graveyard"],
];
const GRANTED_MIRACLE_COST_REDUCED_PREFIX_PHRASES: &[&[&str]] = &[
    &[
        "the", "miracle", "cost", "is", "equal", "to", "its", "mana", "cost", "reduced", "by",
    ],
    &[
        "its", "miracle", "cost", "is", "equal", "to", "its", "mana", "cost", "reduced", "by",
    ],
];

pub fn parse_first_spell_each_turn_clause(
    tokens: &[OwnedLexToken],
) -> Option<FirstSpellEachTurnClause<'_>> {
    let tokens = trim_anthem_clause_tokens(tokens);
    primitives::parse_all(
        tokens,
        parse_first_spell_each_turn_clause_lexed,
        "first-spell-each-turn-clause",
    )
    .ok()
}

pub fn parse_cant_be_blocked_as_long_as_clause(
    tokens: &[OwnedLexToken],
) -> Option<CantBeBlockedAsLongAsClause<'_>> {
    let tokens = trim_anthem_clause_tokens(tokens);
    primitives::parse_all(
        tokens,
        parse_cant_be_blocked_as_long_as_clause_lexed,
        "cant-be-blocked-as-long-as-clause",
    )
    .ok()
}

pub fn parse_defending_player_controls_most_creatures_or_tied_condition(
    tokens: &[OwnedLexToken],
) -> bool {
    token_phrase_complete(
        tokens,
        &[
            "defending",
            "player",
            "controls",
            "the",
            "most",
            "creatures",
            "or",
            "is",
            "tied",
            "for",
            "the",
            "most",
        ],
    )
}

pub fn parse_cant_be_blocked_clause(tokens: &[OwnedLexToken]) -> Option<CantBeBlockedClause<'_>> {
    let tokens = trim_anthem_clause_tokens(tokens);
    let (subject_tokens, _) = primitives::split_lexed_once_before_suffix(tokens, 1, || {
        primitives::any_phrase(CANT_BE_BLOCKED_PHRASES)
    })?;
    let subject_tokens = trim_lexed_commas(subject_tokens);
    (!subject_tokens.is_empty()).then_some(CantBeBlockedClause { subject_tokens })
}

pub fn parse_keywords_and_cant_be_blocked_clause(
    tokens: &[OwnedLexToken],
) -> Option<KeywordsAndCantBeBlockedClause<'_>> {
    let tokens = trim_anthem_clause_tokens(tokens);
    let (keyword_tokens, _) = primitives::split_lexed_once_before_suffix(tokens, 1, || {
        primitives::any_phrase(AND_CANT_BE_BLOCKED_PHRASES)
    })?;
    let keyword_tokens = trim_lexed_commas(keyword_tokens);
    (!keyword_tokens.is_empty()).then_some(KeywordsAndCantBeBlockedClause { keyword_tokens })
}

pub fn parse_keywords_and_cant_be_blocked_by_more_than_clause(
    tokens: &[OwnedLexToken],
) -> Option<KeywordsAndCantBeBlockedByMoreThanClause<'_>> {
    let tokens = trim_anthem_clause_tokens(tokens);
    let has_token = crate::slice_primitives::select_position(tokens, |token| {
        token.is_any_word(&["has", "have"])
    })?;
    if has_token == 0 {
        return None;
    }
    for and_token in has_token + 2..tokens.len() {
        if !tokens[and_token].is_word("and") {
            continue;
        }
        let tail = tokens.get(and_token + 1..)?;
        let blocked_prefix = if primitives::parse_prefix(
            tail,
            primitives::any_phrase(&[
                &["can't", "be", "blocked", "by"],
                &["cant", "be", "blocked", "by"],
                &["cannot", "be", "blocked", "by"],
            ]),
        )
        .is_some()
        {
            4
        } else if primitives::parse_prefix(
            tail,
            primitives::phrase(&["can", "t", "be", "blocked", "by"]),
        )
        .is_some()
        {
            5
        } else {
            continue;
        };
        let blocker_noun = tail.len().checked_sub(1)?;
        if !tail[blocker_noun].is_any_word(&["creature", "creatures"])
            || blocker_noun <= blocked_prefix
        {
            continue;
        }
        let subject_tokens = trim_lexed_commas(&tokens[..has_token]);
        let keyword_tokens = trim_lexed_commas(&tokens[has_token + 1..and_token]);
        let blocker_threshold_tokens = trim_lexed_commas(&tail[blocked_prefix..blocker_noun]);
        if subject_tokens.is_empty()
            || keyword_tokens.is_empty()
            || blocker_threshold_tokens.is_empty()
        {
            return None;
        }
        return Some(KeywordsAndCantBeBlockedByMoreThanClause {
            subject_tokens,
            keyword_tokens,
            blocker_threshold_tokens,
        });
    }
    None
}

pub fn parse_cant_be_blocked_and_has_keywords_clause(
    tokens: &[OwnedLexToken],
) -> Option<CantBeBlockedAndHasKeywordsClause<'_>> {
    let tokens = trim_anthem_clause_tokens(tokens);
    for and_token in 1..tokens.len().saturating_sub(2) {
        if !tokens[and_token].is_word("and") || !tokens[and_token + 1].is_any_word(&["has", "have"])
        {
            continue;
        }
        let blocked = parse_cant_be_blocked_clause(&tokens[..and_token])?;
        let keyword_tokens = trim_lexed_commas(&tokens[and_token + 2..]);
        if keyword_tokens.is_empty() {
            return None;
        }
        return Some(CantBeBlockedAndHasKeywordsClause {
            subject_tokens: blocked.subject_tokens,
            keyword_tokens,
        });
    }
    None
}

pub fn parse_landwalk_block_override_clause(
    tokens: &[OwnedLexToken],
) -> Option<LandwalkBlockOverrideClause<'_>> {
    let tokens = trim_anthem_clause_tokens(tokens);
    primitives::parse_all(
        tokens,
        parse_landwalk_block_override_clause_lexed,
        "landwalk-block-override-clause",
    )
    .ok()
}

pub fn parse_granted_escape_cost_tail_clause(
    tokens: &[OwnedLexToken],
) -> Option<GrantedEscapeCostTail<'_>> {
    let tokens = trim_anthem_clause_tokens(tokens);
    primitives::parse_all(
        tokens,
        parse_granted_escape_cost_tail_clause_lexed,
        "granted-escape-cost-tail",
    )
    .ok()
}

pub fn parse_granted_miracle_cost_reduction_tail_clause(
    tokens: &[OwnedLexToken],
) -> Option<GrantedMiracleCostReductionTail<'_>> {
    let tokens = trim_anthem_clause_tokens(tokens);
    primitives::parse_all(
        tokens,
        parse_granted_miracle_cost_reduction_tail_clause_lexed,
        "granted-miracle-cost-reduction-tail",
    )
    .ok()
}

pub fn parse_cant_be_blocked_by_more_than_clause(
    tokens: &[OwnedLexToken],
) -> Option<CantBeBlockedByMoreThanClause<'_>> {
    let tokens = trim_anthem_clause_tokens(tokens);
    primitives::parse_all(
        tokens,
        parse_cant_be_blocked_by_more_than_clause_lexed,
        "cant-be-blocked-by-more-than-clause",
    )
    .ok()
}

pub fn parse_can_block_additional_creature_clause(
    tokens: &[OwnedLexToken],
) -> Option<CanBlockAdditionalCreatureClause<'_>> {
    let tokens = trim_anthem_clause_tokens(tokens);
    primitives::parse_all(
        tokens,
        parse_can_block_additional_creature_clause_lexed,
        "can-block-additional-creature-clause",
    )
    .ok()
}

pub fn parse_cant_be_blocked_subject_facts(tokens: &[OwnedLexToken]) -> CantBeBlockedSubjectFacts {
    let words = TokenWordView::new(tokens).word_refs();
    let starts_with_source_pronoun = words
        .first()
        .is_some_and(|word| matches!(*word, "this" | "it"));
    let has_rejected_clause_word = words.iter().any(|word| {
        matches!(
            *word,
            "as" | "long"
                | "if"
                | "when"
                | "whenever"
                | "get"
                | "gets"
                | "gain"
                | "gains"
                | "have"
                | "has"
        )
    });
    let mentions_power_or_toughness = word_phrase_occurs(&words, &["power", "or", "toughness"])
        || word_phrase_occurs(&words, &["toughness", "or", "power"]);
    let mut input = LexStream::new(tokens);
    let mut has_conjunction_or_comma = false;
    while let Ok(token) = take_token(&mut input) {
        if token.is_comma() || token.is_word("and") {
            has_conjunction_or_comma = true;
        }
    }
    CantBeBlockedSubjectFacts {
        has_conjunction_or_comma,
        starts_with_source_pronoun,
        has_rejected_clause_word,
        mentions_power_or_toughness,
    }
}

pub fn parse_has_keyword_unblockable_head(
    tokens: &[OwnedLexToken],
) -> Option<HasKeywordUnblockableHead> {
    let mut input = LexStream::new(tokens);
    let initial_len = input.len();
    let mut has_token = None;
    while let Ok(token) = take_token(&mut input) {
        if token.is_any_word(&["has", "have"]) {
            has_token = Some(initial_len.saturating_sub(input.len() + 1));
        }
    }
    let has_token = has_token?;
    (has_token > 0 && has_token + 1 < tokens.len())
        .then_some(HasKeywordUnblockableHead { has_token })
}

pub fn parse_granted_keyword_verb_facts(
    tokens: &[OwnedLexToken],
) -> Option<GrantedKeywordVerbFacts> {
    let mut input = LexStream::new(tokens);
    let initial_len = input.len();
    let mut have_token = None;
    while let Ok(token) = take_token(&mut input) {
        if token.is_any_word(&["has", "have"]) {
            have_token.get_or_insert(initial_len.saturating_sub(input.len() + 1));
        }
    }
    let have_token = have_token?;
    let prefix_has_get = token_word_occurs(&tokens[..have_token], AnthemWordClass::Get);
    let tail = tokens.get(have_token + 1..).unwrap_or_default();
    let starts_with_as_long_as = token_phrase_prefix(tokens, &["as", "long", "as"]);
    Some(GrantedKeywordVerbFacts {
        have_token,
        prefix_has_get,
        starts_with_as_long_as,
        tail_has_have: token_word_occurs(tail, AnthemWordClass::Have),
        tail_has_get_or_be: token_word_occurs(tail, AnthemWordClass::GetOrBe),
    })
}

pub fn granted_keyword_subject_is_rejected(tokens: &[OwnedLexToken]) -> bool {
    let words = TokenWordView::new(tokens).word_refs();
    let has_attached_marker = words
        .iter()
        .any(|word| matches!(*word, "equipped" | "enchanted"));
    let has_bare_mana = crate::slice_primitives::contains(&words, &"mana")
        && !word_phrase_occurs(&words, &["mana", "value"])
        && !parse_first_spell_each_turn_clause(tokens)
            .is_some_and(|clause| clause.mana_source_tokens.is_some());
    let has_rejected_word = words.iter().any(|word| {
        matches!(
            *word,
            "can"
                | "cant"
                | "cannot"
                | "attack"
                | "attacks"
                | "block"
                | "blocks"
                | "blocked"
                | "blocking"
                | "during"
                | "until"
                | "unless"
                | "when"
                | "whenever"
                | "if"
                | "though"
        )
    });
    let attached_marker_is_typed = has_attached_marker
        && matches!(
            parse_exact_anthem_subject_grammar(tokens),
            Some(AnthemSubjectGrammarMatch::Filter(_))
        );
    (has_attached_marker && !attached_marker_is_typed) || has_bare_mana || has_rejected_word
}

pub fn split_trailing_as_long_as_clause(
    tokens: &[OwnedLexToken],
) -> Option<TrailingAsLongAsClause<'_>> {
    let tokens = trim_anthem_clause_tokens(tokens);
    let mut input = LexStream::new(tokens);
    let initial_len = input.len();
    loop {
        let start = initial_len.saturating_sub(input.len());
        let mut candidate = input.clone();
        if primitives::phrase(&["as", "long", "as"])
            .parse_next(&mut candidate)
            .is_ok()
        {
            let condition_start = initial_len.saturating_sub(candidate.len());
            let keyword_tokens = trim_lexed_commas(&tokens[..start]);
            let condition_tokens = trim_lexed_commas(&tokens[condition_start..]);
            if keyword_tokens.is_empty() || condition_tokens.is_empty() {
                return None;
            }
            return Some(TrailingAsLongAsClause {
                keyword_tokens,
                condition_tokens,
            });
        }
        let parsed: WResult<&OwnedLexToken> = any.parse_next(&mut input);
        parsed.ok()?;
    }
}

/// Split a granted-keyword clause whose timing is authored as a trailing
/// "during your turn" phrase, such as "This creature has first strike during
/// your turn."  Keeping this structural avoids letting the keyword parser
/// silently discard the timing suffix.
pub fn split_trailing_during_your_turn_clause(
    tokens: &[OwnedLexToken],
) -> Option<&[OwnedLexToken]> {
    let tokens = trim_anthem_clause_tokens(tokens);
    let (keyword_tokens, ()) = primitives::split_lexed_once_before_suffix(tokens, 1, || {
        primitives::phrase(&["during", "your", "turn"]).void()
    })?;
    let keyword_tokens = trim_lexed_commas(keyword_tokens);
    (!keyword_tokens.is_empty()).then_some(keyword_tokens)
}

pub fn split_must_attack_keyword_tail(
    tokens: &[OwnedLexToken],
) -> Option<MustAttackKeywordTail<'_>> {
    let tokens = trim_anthem_clause_tokens(tokens);
    let (keyword_tokens, _) = primitives::split_lexed_once_before_suffix(tokens, 1, || {
        primitives::any_phrase(&[
            &["attacks", "each", "combat", "if", "able"],
            &["attack", "each", "combat", "if", "able"],
            &["and", "attack", "each", "combat", "if", "able"],
            &["and", "attacks", "each", "combat", "if", "able"],
        ])
    })?;
    let keyword_tokens = trim_lexed_commas(keyword_tokens);
    (!keyword_tokens.is_empty()).then_some(MustAttackKeywordTail { keyword_tokens })
}

pub fn classify_granted_keyword_tokens(tokens: &[OwnedLexToken]) -> GrantedKeywordTokenKind {
    let words = TokenWordView::new(trim_anthem_clause_tokens(tokens)).word_refs();
    if primitives::parse_word_sequence_complete(&words, &["blitz"]).is_some() {
        GrantedKeywordTokenKind::Blitz
    } else if primitives::parse_word_sequence_complete(&words, &["emerge"]).is_some() {
        GrantedKeywordTokenKind::Emerge
    } else if primitives::parse_word_sequence_complete(&words, &["scavenge"]).is_some() {
        GrantedKeywordTokenKind::Scavenge
    } else if primitives::parse_word_sequence_complete(&words, &["exploit"]).is_some() {
        GrantedKeywordTokenKind::Exploit
    } else if primitives::parse_word_sequence_prefix(&words, &["unearth"]).is_some()
        || primitives::parse_word_sequence_prefix(&words, &["conspire"]).is_some()
    {
        GrantedKeywordTokenKind::IgnoredReminder
    } else {
        GrantedKeywordTokenKind::Other
    }
}

pub fn parse_granted_flashback_cost_equals_mana(tokens: &[OwnedLexToken]) -> bool {
    token_phrase_complete(
        tokens,
        &[
            "its",
            "flashback",
            "cost",
            "is",
            "equal",
            "to",
            "its",
            "mana",
            "cost",
        ],
    )
}

pub fn parse_granted_blitz_cost_equals_mana(tokens: &[OwnedLexToken]) -> bool {
    token_any_phrase_complete(
        tokens,
        &[
            &[
                "the", "blitz", "cost", "is", "equal", "to", "its", "mana", "cost",
            ],
            &[
                "its", "blitz", "cost", "is", "equal", "to", "its", "mana", "cost",
            ],
        ],
    )
}

pub fn parse_granted_emerge_cost_equals_mana(tokens: &[OwnedLexToken]) -> bool {
    token_any_phrase_complete(
        tokens,
        &[
            &[
                "the", "emerge", "cost", "is", "equal", "to", "its", "mana", "cost",
            ],
            &[
                "its", "emerge", "cost", "is", "equal", "to", "its", "mana", "cost",
            ],
        ],
    )
}

pub fn parse_granted_scavenge_cost_equals_mana(tokens: &[OwnedLexToken]) -> bool {
    token_any_phrase_complete(
        tokens,
        &[
            &[
                "the", "scavenge", "cost", "is", "equal", "to", "its", "mana", "cost",
            ],
            &[
                "its", "scavenge", "cost", "is", "equal", "to", "its", "mana", "cost",
            ],
        ],
    )
}

pub fn parse_all_creatures_lose_flying(tokens: &[OwnedLexToken]) -> bool {
    token_phrase_complete(tokens, &["all", "creatures", "lose", "flying"])
}

pub fn parse_subject_loses_keywords_clause(
    tokens: &[OwnedLexToken],
) -> Option<SubjectLosesKeywordsClause<'_>> {
    let tokens = trim_anthem_clause_tokens(tokens);
    // A "lose" word inside quoted ability text (e.g. a granted trigger body
    // like "... create a token ... it has haste and loses soulbond ...")
    // belongs to that quoted ability, not to a subject-loses-keywords line.
    if tokens.iter().any(OwnedLexToken::is_quote) {
        return None;
    }
    let lose_token = first_token_word(tokens, AnthemWordClass::Lose)?;
    if lose_token == 0 {
        return None;
    }
    let subject_tokens = trim_lexed_commas(&tokens[..lose_token]);
    let subject_words = TokenWordView::new(subject_tokens).word_refs();
    if subject_words.first().copied() == Some("target")
        || subject_words
            .iter()
            .any(|word| matches!(*word, "get" | "gets"))
    {
        return None;
    }
    let tail = trim_lexed_commas(&tokens[lose_token + 1..]);
    if tail.is_empty() {
        return None;
    }
    if let Some((and_token, gain_start, loss_mode)) = find_cant_gain_tail(tail) {
        let loss_tokens = trim_lexed_commas(&tail[..and_token]);
        let additional_gain_tokens = trim_lexed_commas(&tail[gain_start..]);
        if loss_tokens.is_empty() || additional_gain_tokens.is_empty() {
            return None;
        }
        return Some(SubjectLosesKeywordsClause {
            subject_tokens,
            loss_tokens,
            additional_gain_tokens: Some(additional_gain_tokens),
            loss_mode,
        });
    }
    Some(SubjectLosesKeywordsClause {
        subject_tokens,
        loss_tokens: tail,
        additional_gain_tokens: None,
        loss_mode: ironsmith_core::AbilityLossMode::Lose,
    })
}

pub fn parse_each_creature_subject(tokens: &[OwnedLexToken]) -> Option<EachCreatureSubject<'_>> {
    let tokens = trim_anthem_clause_tokens(tokens);
    let (_, rest) = primitives::parse_prefix(tokens, primitives::phrase(&["each", "creature"]))?;
    let consumed = tokens.len().saturating_sub(rest.len());
    let filter_tokens = trim_lexed_commas(&tokens[1..]);
    (consumed == 2 && !filter_tokens.is_empty()).then_some(EachCreatureSubject { filter_tokens })
}

pub fn parse_additional_creature_count(tokens: &[OwnedLexToken]) -> Option<usize> {
    let tokens = trim_anthem_clause_tokens(tokens);
    if primitives::parse_all(tokens, primitives::kw("an"), "additional-creature-article").is_ok() {
        return Some(1);
    }
    let count = primitives::parse_all(
        tokens,
        leaf::parse_leaf_number_prefix_lexed,
        "additional-creature-count",
    )
    .ok()?;
    usize::try_from(count).ok()
}

pub fn parse_indestructible_grant_clause(
    tokens: &[OwnedLexToken],
) -> Option<IndestructibleGrantClause<'_>> {
    let tokens = trim_anthem_clause_tokens(tokens);
    let have_token = first_token_word(tokens, AnthemWordClass::Have)?;
    if token_word_occurs(&tokens[..have_token], AnthemWordClass::Get) {
        return None;
    }
    let subject_tokens = trim_lexed_commas(&tokens[..have_token]);
    let ability_tokens = trim_lexed_commas(&tokens[have_token + 1..]);
    (!subject_tokens.is_empty() && !ability_tokens.is_empty()).then_some(
        IndestructibleGrantClause {
            subject_tokens,
            ability_tokens,
        },
    )
}

pub fn parse_lose_all_transform_shape(tokens: &[OwnedLexToken]) -> Option<LoseAllTransformShape> {
    let words = TokenWordView::new(tokens).word_refs();
    if words.len() < 8 {
        return None;
    }
    let is_word = first_word_offset(&words, &["is", "are"])?;
    let with_word = first_phrase_offset(&words, &["with", "base", "power", "and", "toughness"])?;
    if with_word <= is_word {
        return None;
    }
    let power_toughness_word = with_word + 5;
    words.get(power_toughness_word)?;
    if !word_phrase_occurs(&words, &["lose", "all", "abilities"])
        && !word_phrase_occurs(&words, &["loses", "all", "abilities"])
    {
        return None;
    }
    let lose_word = first_word_offset(&words, &["lose", "loses"]).unwrap_or(is_word);
    let subject_word_end = is_word.min(lose_word);
    if subject_word_end == 0 || with_word <= is_word + 1 {
        return None;
    }
    let tail_first = with_word + 6;
    let name_words = first_word_offset(words.get(tail_first..).unwrap_or_default(), &["named"])
        .and_then(|relative_named| {
            let named = tail_first + relative_named;
            let end = first_word_offset(
                words.get(named + 1..).unwrap_or_default(),
                &[
                    "and", "lose", "loses", "with", "it", "that", "those", "this",
                ],
            )
            .map(|relative| named + 1 + relative)
            .unwrap_or(words.len());
            (end > named + 1).then_some(named + 1..end)
        });
    Some(LoseAllTransformShape {
        subject_word_end,
        descriptor_words: is_word + 1..with_word,
        power_toughness_word,
        name_words,
        except_mana_abilities: word_phrase_occurs(&words, &["except", "mana", "abilities"]),
    })
}

pub fn parse_lose_all_abilities_shape(tokens: &[OwnedLexToken]) -> Option<LoseAllAbilitiesShape> {
    let words = TokenWordView::new(tokens).word_refs();
    if first_phrase_offset(&words, &["with", "base", "power", "and", "toughness"]).is_some()
        && first_word_offset(&words, &["is", "are"]).is_some()
    {
        return None;
    }
    let lose_word = first_word_offset(&words, &["lose", "loses"])?;
    // A preceding `get(s)` belongs to an anthem compound such as
    // "gets -5/-0 and loses all abilities". It is not part of the affected
    // object's filter, and accepting it here would silently discard the P/T
    // modification before the dedicated compound parser gets a chance.
    if first_word_offset(words.get(..lose_word).unwrap_or_default(), &["get", "gets"]).is_some() {
        return None;
    }
    if !word_phrase_occurs(
        words.get(lose_word + 1..).unwrap_or_default(),
        &["all", "abilities"],
    ) || first_word_offset(&words, &["until"]).is_some()
    {
        return None;
    }
    let base_power_toughness_word = first_word_offset(&words, &["have", "has"]).and_then(|have| {
        let tail = words.get(have + 1..)?;
        let mut input: primitives::WordSliceInput<'_> = tail;
        parse_word_phrase_input(&mut input, &["base", "power", "and", "toughness"]).ok()?;
        let candidate = words.len().saturating_sub(input.len());
        words
            .get(candidate)
            .and_then(|word| leaf::parse_leaf_power_toughness_complete(word).ok())
            .map(|_| candidate)
    });
    Some(LoseAllAbilitiesShape {
        subject_word_end: lose_word,
        becomes: first_word_offset(&words, &["becomes"]).is_some(),
        except_mana_abilities: word_phrase_occurs(&words, &["except", "mana", "abilities"]),
        base_power_toughness_word,
    })
}

pub fn emerge_subject_is_spell_cast(tokens: &[OwnedLexToken]) -> bool {
    let words = TokenWordView::new(tokens).word_refs();
    first_word_offset(&words, &["spell"]).is_some()
        && first_word_offset(&words, &["cast"]).is_some()
}

pub fn parse_source_counter_threshold_head(
    tokens: &[OwnedLexToken],
) -> Option<GrantedKeywordVerbFacts> {
    let facts = parse_granted_keyword_verb_facts(tokens)?;
    facts.starts_with_as_long_as.then_some(facts)
}

pub fn split_keyword_and_type_addition(
    tokens: &[OwnedLexToken],
) -> Option<KeywordTypeAdditionSplit<'_>> {
    let tokens = trim_anthem_clause_tokens(tokens);
    let mut input = LexStream::new(tokens);
    let initial_len = input.len();
    loop {
        let and_token = initial_len.saturating_sub(input.len());
        let mut candidate = input.clone();
        if primitives::phrase(&["and", "is"])
            .parse_next(&mut candidate)
            .is_ok()
            || primitives::phrase(&["and", "are"])
                .parse_next(&mut candidate)
                .is_ok()
        {
            if and_token == 0 {
                return None;
            }
            let addition_token = and_token + 1;
            let keyword_tokens = trim_lexed_commas(&tokens[..and_token]);
            let addition_tokens = trim_lexed_commas(&tokens[addition_token..]);
            return (!keyword_tokens.is_empty() && !addition_tokens.is_empty()).then_some(
                KeywordTypeAdditionSplit {
                    keyword_tokens,
                    addition_tokens,
                },
            );
        }
        take_token(&mut input).ok()?;
    }
}

pub fn classify_suffix_filter_head(tokens: &[OwnedLexToken]) -> SuffixFilterHead {
    let words = TokenWordView::new(tokens).word_refs();
    if primitives::parse_word_sequence_prefix(&words, &["other"]).is_some()
        || primitives::parse_word_sequence_prefix(&words, &["another"]).is_some()
    {
        SuffixFilterHead::Other
    } else if primitives::parse_word_sequence_complete(&words, &["it"]).is_some()
        || primitives::parse_word_sequence_complete(&words, &["them"]).is_some()
    {
        SuffixFilterHead::Pronoun
    } else {
        SuffixFilterHead::Normal
    }
}

pub fn parse_shared_suffix_candidates(tokens: &[OwnedLexToken]) -> Vec<SharedSuffixCandidate> {
    let mut candidates = Vec::new();
    let mut input = LexStream::new(tokens);
    let initial_len = input.len();
    while let Ok(token) = take_token(&mut input) {
        let and_token = initial_len.saturating_sub(input.len() + 1);
        if !token.is_word("and") {
            continue;
        }
        let right_tail = &tokens[and_token + 1..];
        if right_tail.len() < 2 {
            continue;
        }
        let mut right_input = LexStream::new(right_tail);
        let right_len = right_input.len();
        while take_token(&mut right_input).is_ok() {
            let split_relative = right_len.saturating_sub(right_input.len());
            if split_relative == 0 || split_relative >= right_tail.len() {
                continue;
            }
            let mut shared_head = LexStream::new(&right_tail[split_relative..]);
            if parse_shared_suffix_head(&mut shared_head).is_ok() {
                candidates.push(SharedSuffixCandidate {
                    and_token,
                    split_token: and_token + 1 + split_relative,
                });
            }
        }
    }
    candidates
}

fn parse_shared_suffix_head(input: &mut LexStream<'_>) -> WResult<()> {
    alt((
        alt((
            primitives::kw("you").void(),
            primitives::kw("your").void(),
            primitives::kw("that").void(),
            primitives::kw("those").void(),
            primitives::kw("with").void(),
            primitives::kw("without").void(),
        )),
        alt((
            primitives::kw("named").void(),
            primitives::kw("in").void(),
            primitives::kw("from").void(),
            primitives::kw("on").void(),
            primitives::kw("among").void(),
            primitives::kw("under").void(),
        )),
        primitives::kw("during").void(),
        alt((
            primitives::kw("creature").void(),
            primitives::kw("creatures").void(),
            primitives::kw("permanent").void(),
            primitives::kw("permanents").void(),
            primitives::kw("spell").void(),
            primitives::kw("spells").void(),
            primitives::kw("card").void(),
            primitives::kw("cards").void(),
        )),
    ))
    .parse_next(input)
}

pub fn is_source_it_subject(tokens: &[OwnedLexToken]) -> bool {
    token_phrase_complete(tokens, &["it"]) || token_phrase_complete(tokens, &["this", "token"])
}

pub fn parse_enchanted_player_controls_prefix(
    tokens: &[OwnedLexToken],
) -> Option<&[OwnedLexToken]> {
    let tokens = trim_anthem_clause_tokens(tokens);
    let (prefix, _) = primitives::split_lexed_once_before_suffix(tokens, 1, || {
        primitives::phrase(&["enchanted", "player", "controls"])
    })?;
    let prefix = trim_lexed_commas(prefix);
    (!prefix.is_empty()).then_some(prefix)
}

pub fn parse_attached_condition_subject(tokens: &[OwnedLexToken]) -> Option<&[OwnedLexToken]> {
    let tokens = trim_anthem_clause_tokens(tokens);
    let (_, rest) = primitives::parse_prefix(
        tokens,
        primitives::any_phrase(&[
            &["enchanted", "artifact"],
            &["enchanted", "creature"],
            &["enchanted", "land"],
            &["enchanted", "permanent"],
            &["equipped", "creature"],
            &["equipped", "permanent"],
        ]),
    )?;
    let consumed = tokens.len().saturating_sub(rest.len());
    (consumed > 0).then_some(&tokens[..consumed])
}

pub fn parse_permanent_card_count_facts(
    tokens: &[OwnedLexToken],
) -> Option<PermanentCardCountFacts> {
    let words = TokenWordView::new(tokens).word_refs();
    let mut input: primitives::WordSliceInput<'_> = &words;
    primitives::word_slice_exact("permanent")
        .parse_next(&mut input)
        .ok()?;
    alt((
        primitives::word_slice_exact("card"),
        primitives::word_slice_exact("cards"),
    ))
    .parse_next(&mut input)
    .ok()?;
    for (index, word) in words.iter().enumerate() {
        let Ok(zone) = leaf::parse_leaf_zone_complete(word) else {
            continue;
        };
        let owner = match index
            .checked_sub(1)
            .and_then(|previous| words.get(previous))
        {
            Some(&"your") => Some(PermanentCardOwner::You),
            Some(&("opponent" | "opponents")) => Some(PermanentCardOwner::Opponent),
            _ => None,
        };
        return Some(PermanentCardCountFacts { zone, owner });
    }
    None
}

pub fn parse_conditional_must_block_shape(
    tokens: &[OwnedLexToken],
) -> Option<ConditionalMustBlockShape<'_>> {
    let tokens = trim_anthem_clause_tokens(tokens);
    let (_, after_prefix) =
        primitives::parse_prefix(tokens, primitives::phrase(&["as", "long", "as"]))?;
    let prefix_len = tokens.len().saturating_sub(after_prefix.len());
    let comma = first_token_kind_from(tokens, prefix_len, TokenKind::Comma)?;
    let condition_tokens = trim_lexed_commas(&tokens[prefix_len..comma]);
    if condition_tokens.is_empty() {
        return None;
    }
    let remainder = trim_lexed_commas(&tokens[comma + 1..]);
    let target = if token_any_phrase_complete(
        remainder,
        &[
            &[
                "all",
                "creatures",
                "able",
                "to",
                "block",
                "this",
                "creature",
                "do",
                "so",
            ],
            &[
                "all",
                "creatures",
                "able",
                "to",
                "block",
                "this",
                "do",
                "so",
            ],
        ],
    ) {
        ConditionalMustBlockTarget::Source
    } else if token_phrase_complete(
        remainder,
        &[
            "all",
            "creatures",
            "able",
            "to",
            "block",
            "enchanted",
            "creature",
            "do",
            "so",
        ],
    ) {
        ConditionalMustBlockTarget::EnchantedCreature
    } else {
        return None;
    };
    Some(ConditionalMustBlockShape {
        condition_tokens,
        target,
    })
}

pub fn parse_subject_no_defender_as_long_shape(
    tokens: &[OwnedLexToken],
) -> Option<NoDefenderConditionalShape<'_>> {
    let tokens = trim_anthem_clause_tokens(tokens);
    let (phrase_start, phrase_end) = find_no_defender_phrase(tokens, true)?;
    let subject_tokens = trim_lexed_commas(&tokens[..phrase_start]);
    let condition_tokens = trim_lexed_commas(&tokens[phrase_end..]);
    (!subject_tokens.is_empty() && !condition_tokens.is_empty()).then_some(
        NoDefenderConditionalShape {
            subject_tokens,
            condition_tokens,
        },
    )
}

pub fn parse_attached_no_defender_shape(
    tokens: &[OwnedLexToken],
) -> Option<NoDefenderSubjectShape<'_>> {
    let tokens = trim_anthem_clause_tokens(tokens);
    let (phrase_start, phrase_end) = find_no_defender_phrase(tokens, false)?;
    if phrase_end != tokens.len() {
        return None;
    }
    let subject_tokens = trim_lexed_commas(&tokens[..phrase_start]);
    let first = subject_tokens.first()?.as_word()?;
    (matches!(first, "enchanted" | "equipped" | "attached") && !subject_tokens.is_empty())
        .then_some(NoDefenderSubjectShape { subject_tokens })
}

pub fn parse_plain_no_defender_shape(
    tokens: &[OwnedLexToken],
) -> Option<NoDefenderSubjectShape<'_>> {
    let tokens = trim_anthem_clause_tokens(tokens);
    let (phrase_start, phrase_end) = find_no_defender_phrase(tokens, false)?;
    if phrase_end != tokens.len() {
        return None;
    }
    let subject_tokens = trim_lexed_commas(&tokens[..phrase_start]);
    (!subject_tokens.is_empty()).then_some(NoDefenderSubjectShape { subject_tokens })
}

pub fn parse_leading_condition_no_defender_shape(
    tokens: &[OwnedLexToken],
) -> Option<NoDefenderConditionalShape<'_>> {
    let tokens = trim_anthem_clause_tokens(tokens);
    let (_, after_prefix) =
        primitives::parse_prefix(tokens, primitives::phrase(&["as", "long", "as"]))?;
    let prefix_len = tokens.len().saturating_sub(after_prefix.len());
    let comma = first_token_kind_from(tokens, prefix_len, TokenKind::Comma)?;
    let condition_tokens = trim_lexed_commas(&tokens[prefix_len..comma]);
    let remainder = trim_lexed_commas(&tokens[comma + 1..]);
    let (phrase_start, phrase_end) = find_no_defender_phrase(remainder, false)?;
    if phrase_end != remainder.len() {
        return None;
    }
    let subject_tokens = trim_lexed_commas(&remainder[..phrase_start]);
    (!condition_tokens.is_empty() && !subject_tokens.is_empty()).then_some(
        NoDefenderConditionalShape {
            subject_tokens,
            condition_tokens,
        },
    )
}

pub fn parse_gets_attacks_shape(tokens: &[OwnedLexToken]) -> Option<GetsAttacksShape> {
    let tokens = trim_anthem_clause_tokens(tokens);
    let get_token = first_token_word(tokens, AnthemWordClass::Get)?;
    let relative_and = first_token_word(&tokens[get_token + 1..], AnthemWordClass::And)?;
    let and_token = get_token + 1 + relative_and;
    let attack_token = and_token + 1;
    token_any_phrase_complete(
        &tokens[attack_token..],
        &[
            &["attacks", "each", "combat", "if", "able"],
            &["attack", "each", "combat", "if", "able"],
        ],
    )
    .then_some(GetsAttacksShape {
        get_token,
        and_token,
    })
}

pub fn parse_anthem_and_granted_tail(tokens: &[OwnedLexToken]) -> Option<AnthemAndGrantedTail> {
    let tokens = trim_anthem_clause_tokens(tokens);
    let get_token = first_token_word(tokens, AnthemWordClass::Get)?;
    let relative_and = first_token_word(&tokens[get_token + 1..], AnthemWordClass::And)?;
    let and_token = get_token + 1 + relative_and;
    let tail = trim_lexed_commas(&tokens[and_token + 1..]);
    if token_any_phrase_complete(
        tail,
        &[
            &["can't", "be", "blocked"],
            &["cant", "be", "blocked"],
            &["cannot", "be", "blocked"],
        ],
    ) {
        return Some(AnthemAndGrantedTail {
            get_token,
            and_token,
            tail_kind: AnthemGrantedTailKind::CantBeBlocked,
        });
    }
    let (_, family_tokens) = primitives::parse_prefix(
        tail,
        alt((primitives::kw("is"), primitives::kw("are"))).void(),
    )?;
    let family = parse_every_subtype_family_tokens(family_tokens)?;
    Some(AnthemAndGrantedTail {
        get_token,
        and_token,
        tail_kind: AnthemGrantedTailKind::BeEverySubtype(family),
    })
}

pub fn parse_subject_every_subtype_shape(
    tokens: &[OwnedLexToken],
) -> Option<SubjectEverySubtypeShape<'_>> {
    let tokens = trim_anthem_clause_tokens(tokens);
    let mut clause_tokens = tokens;
    let mut condition_tokens = None;
    if let Some((_, after_prefix)) =
        primitives::parse_prefix(tokens, primitives::phrase(&["as", "long", "as"]))
    {
        let prefix_len = tokens.len().saturating_sub(after_prefix.len());
        let comma = first_token_kind_from(tokens, prefix_len, TokenKind::Comma)?;
        let condition = trim_lexed_commas(&tokens[prefix_len..comma]);
        if condition.is_empty() {
            return None;
        }
        condition_tokens = Some(condition);
        clause_tokens = trim_lexed_commas(&tokens[comma + 1..]);
    }
    let be_token = first_token_word(clause_tokens, AnthemWordClass::Be)?;
    if be_token == 0 {
        return None;
    }
    let subject_tokens = trim_lexed_commas(&clause_tokens[..be_token]);
    let family_tokens = trim_lexed_commas(&clause_tokens[be_token + 1..]);
    let family = parse_every_subtype_family_tokens(family_tokens)?;
    (!subject_tokens.is_empty()).then_some(SubjectEverySubtypeShape {
        condition_tokens,
        subject_tokens,
        family,
    })
}

pub fn parse_anthem_modifier_head(tokens: &[OwnedLexToken]) -> Option<AnthemModifierHead> {
    let tokens = trim_anthem_clause_tokens(tokens);
    let words = TokenWordView::new(tokens).word_refs();
    let get_token = first_token_word(tokens, AnthemWordClass::Get)?;
    let mut modifier_token = get_token + 1;
    if tokens
        .get(modifier_token)
        .and_then(OwnedLexToken::as_word)
        .is_some_and(|word| matches!(word, "a" | "an"))
        && tokens
            .get(modifier_token + 1)
            .is_some_and(|token| token.is_word("additional"))
    {
        modifier_token += 2;
    }
    tokens.get(modifier_token)?.as_word()?;
    Some(AnthemModifierHead {
        get_token,
        modifier_token,
        has_target: first_word_offset(&words, &["target"]).is_some(),
        temporary: parse_static_grant_duration_fact(tokens).is_some(),
    })
}

/// Parses the complete static-condition shape
/// "<player> (has|have) drawn N or more card(s) this turn".
pub fn parse_cards_drawn_this_turn_threshold(tokens: &[OwnedLexToken]) -> Option<TurnThreshold> {
    primitives::parse_all(
        tokens,
        parse_cards_drawn_this_turn_threshold_lexed,
        "cards-drawn-this-turn-threshold",
    )
    .ok()
}

/// Parses the complete static-condition shape
/// "<player> (has|have) rolled N or more die/dice this turn".
pub fn parse_dice_rolled_this_turn_threshold(tokens: &[OwnedLexToken]) -> Option<TurnThreshold> {
    primitives::parse_all(
        tokens,
        parse_dice_rolled_this_turn_threshold_lexed,
        "dice-rolled-this-turn-threshold",
    )
    .ok()
}

/// Parses the complete granted-keyword color condition after `if`.
pub fn parse_if_source_is_color(tokens: &[OwnedLexToken]) -> Option<ColorSet> {
    primitives::parse_all(tokens, parse_if_source_is_color_lexed, "if-source-is-color").ok()
}

/// Parses the structural tail of an anthem count such as
/// "lore counters on this enchantment". The caller owns semantic validation of
/// the counter type and the source-reference surface.
pub fn parse_source_counter_count_clause(
    tokens: &[OwnedLexToken],
) -> Option<SourceCounterCountClause<'_>> {
    let (counter_type_word, source_tokens) =
        primitives::parse_prefix(tokens, parse_source_counter_head_lexed)?;
    if source_tokens.is_empty() {
        return None;
    }
    let starts_with_source_pronoun =
        primitives::parse_prefix(source_tokens, parse_source_pronoun_lexed).is_some();
    Some(SourceCounterCountClause {
        counter_type_word,
        source_tokens,
        starts_with_source_pronoun,
    })
}

fn parse_cards_drawn_this_turn_threshold_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<TurnThreshold> {
    let player = parse_cards_drawn_subject_lexed.parse_next(input)?;
    let count = leaf::parse_leaf_number_prefix_lexed.parse_next(input)?;
    primitives::phrase(&["or", "more"]).parse_next(input)?;
    alt((primitives::kw("card"), primitives::kw("cards"))).parse_next(input)?;
    primitives::phrase(&["this", "turn"]).parse_next(input)?;
    Ok(TurnThreshold { player, count })
}

fn parse_dice_rolled_this_turn_threshold_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<TurnThreshold> {
    let player = parse_dice_rolled_subject_lexed.parse_next(input)?;
    let count = leaf::parse_leaf_number_prefix_lexed.parse_next(input)?;
    primitives::phrase(&["or", "more"]).parse_next(input)?;
    alt((primitives::kw("die"), primitives::kw("dice"))).parse_next(input)?;
    primitives::phrase(&["this", "turn"]).parse_next(input)?;
    Ok(TurnThreshold { player, count })
}

fn parse_cards_drawn_subject_lexed<'a>(input: &mut LexStream<'a>) -> WResult<TurnThresholdPlayer> {
    alt((
        (
            alt((
                primitives::kw("youve"),
                primitives::kw("you've"),
                primitives::kw("you’ve"),
            )),
            primitives::kw("drawn"),
        )
            .value(TurnThresholdPlayer::You),
        primitives::phrase(&["you", "have", "drawn"]).value(TurnThresholdPlayer::You),
        primitives::phrase(&["you", "ve", "drawn"]).value(TurnThresholdPlayer::You),
        primitives::phrase(&["an", "opponent", "has", "drawn"])
            .value(TurnThresholdPlayer::Opponent),
        primitives::phrase(&["opponent", "has", "drawn"]).value(TurnThresholdPlayer::Opponent),
        primitives::phrase(&["opponents", "have", "drawn"]).value(TurnThresholdPlayer::Opponent),
        primitives::phrase(&["a", "player", "has", "drawn"]).value(TurnThresholdPlayer::Any),
        primitives::phrase(&["player", "has", "drawn"]).value(TurnThresholdPlayer::Any),
        primitives::phrase(&["players", "have", "drawn"]).value(TurnThresholdPlayer::Any),
    ))
    .parse_next(input)
}

fn parse_dice_rolled_subject_lexed<'a>(input: &mut LexStream<'a>) -> WResult<TurnThresholdPlayer> {
    alt((
        (
            alt((
                primitives::kw("youve"),
                primitives::kw("you've"),
                primitives::kw("you’ve"),
            )),
            primitives::kw("rolled"),
        )
            .value(TurnThresholdPlayer::You),
        primitives::phrase(&["you", "have", "rolled"]).value(TurnThresholdPlayer::You),
        primitives::phrase(&["you", "ve", "rolled"]).value(TurnThresholdPlayer::You),
        primitives::phrase(&["an", "opponent", "has", "rolled"])
            .value(TurnThresholdPlayer::Opponent),
        primitives::phrase(&["opponent", "has", "rolled"]).value(TurnThresholdPlayer::Opponent),
        primitives::phrase(&["opponents", "have", "rolled"]).value(TurnThresholdPlayer::Opponent),
        primitives::phrase(&["a", "player", "has", "rolled"]).value(TurnThresholdPlayer::Any),
        primitives::phrase(&["player", "has", "rolled"]).value(TurnThresholdPlayer::Any),
        primitives::phrase(&["players", "have", "rolled"]).value(TurnThresholdPlayer::Any),
    ))
    .parse_next(input)
}

fn parse_if_source_is_color_lexed<'a>(input: &mut LexStream<'a>) -> WResult<ColorSet> {
    alt((
        primitives::kw("its").value(()),
        primitives::kw("it's").value(()),
        primitives::kw("it’s").value(()),
        primitives::phrase(&["it", "is"]).value(()),
        primitives::phrase(&["it", "s"]).value(()),
        primitives::phrase(&["this", "creature", "is"]).value(()),
        primitives::phrase(&["that", "creature", "is"]).value(()),
    ))
    .parse_next(input)?;
    let color = primitives::word_parser_text.parse_next(input)?;
    leaf::parse_leaf_color_complete(color)
        .map_err(|_| primitives::backtrack_err("color", "Magic color word"))
}

fn parse_source_counter_head_lexed<'a>(input: &mut LexStream<'a>) -> WResult<&'a str> {
    let mut previous_word: Option<&'a str> = None;
    loop {
        let token: &'a OwnedLexToken = any.parse_next(input)?;
        let Some(word) = token.as_word() else {
            continue;
        };
        if matches!(word, "counter" | "counters") {
            let counter_type_word = previous_word.ok_or_else(|| {
                primitives::backtrack_err("counter type", "word before counter noun")
            })?;
            primitives::kw("on").parse_next(input)?;
            return Ok(counter_type_word);
        }
        previous_word = Some(word);
    }
}

fn parse_source_pronoun_lexed<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        primitives::kw("it"),
        primitives::kw("this"),
        primitives::kw("him"),
        primitives::kw("her"),
    ))
    .void()
    .parse_next(input)
}

fn parse_first_spell_each_turn_clause_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<FirstSpellEachTurnClause<'a>> {
    opt(primitives::kw("the")).parse_next(input)?;
    primitives::kw("first").parse_next(input)?;
    let filter_tokens = take_until_phrase(input, &[&["each", "turn"]])?;
    primitives::phrase(&["each", "turn"]).parse_next(input)?;
    let filter_tokens = trim_lexed_commas(filter_tokens);
    let trailing: &'a [OwnedLexToken] = rest.parse_next(input)?;
    let trailing = trim_lexed_commas(trailing);
    let mana_source_tokens = if trailing.is_empty() {
        None
    } else {
        let (_, after_relative) =
            primitives::parse_prefix(trailing, primitives::phrase(&["that", "mana", "from"]))
                .ok_or_else(|| {
                    primitives::backtrack_err(
                        "first-spell-each-turn mana source",
                        "'that mana from' after 'each turn'",
                    )
                })?;
        let (source_end, _, after_suffix) = primitives::find_prefix(after_relative, || {
            primitives::phrase(&["was", "spent", "to", "cast"])
        })
        .ok_or_else(|| {
            primitives::backtrack_err(
                "first-spell-each-turn mana source",
                "a source followed by 'was spent to cast'",
            )
        })?;
        if !trim_lexed_commas(after_suffix).is_empty() {
            return Err(primitives::backtrack_err(
                "first-spell-each-turn mana source",
                "the end of the mana-source relative clause",
            ));
        }
        let source_tokens = trim_lexed_commas(after_relative.get(..source_end).unwrap_or_default());
        if source_tokens.is_empty() {
            return Err(primitives::backtrack_err(
                "first-spell-each-turn mana source",
                "a nonempty mana source",
            ));
        }
        Some(source_tokens)
    };
    if primitives::find_prefix(filter_tokens, || primitives::phrase(&["you", "cast"])).is_none() {
        return Err(primitives::backtrack_err(
            "first-spell-each-turn subject",
            "a spell subject followed by 'you cast'",
        ));
    }
    Ok(FirstSpellEachTurnClause {
        filter_tokens,
        mana_source_tokens,
    })
}

fn parse_cant_be_blocked_as_long_as_clause_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<CantBeBlockedAsLongAsClause<'a>> {
    let subject_tokens = take_until_phrase(input, CANT_BE_BLOCKED_AS_LONG_AS_PHRASES)?;
    primitives::any_phrase(CANT_BE_BLOCKED_AS_LONG_AS_PHRASES).parse_next(input)?;
    let condition_tokens: &'a [OwnedLexToken] = rest.parse_next(input)?;
    let subject_tokens = trim_lexed_commas(subject_tokens);
    let condition_tokens = trim_lexed_commas(condition_tokens);
    if subject_tokens.is_empty() || condition_tokens.is_empty() {
        return Err(primitives::backtrack_err(
            "blocked condition",
            "nonempty subject and condition",
        ));
    }
    Ok(CantBeBlockedAsLongAsClause {
        subject_tokens,
        condition_tokens,
    })
}

fn parse_landwalk_block_override_clause_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<LandwalkBlockOverrideClause<'a>> {
    let subject_tokens = take_until_phrase(input, CAN_BE_BLOCKED_AS_THOUGH_NO_ABILITY_PHRASES)?;
    primitives::any_phrase(CAN_BE_BLOCKED_AS_THOUGH_NO_ABILITY_PHRASES).parse_next(input)?;
    let ability_token: &'a OwnedLexToken = any.parse_next(input)?;
    let ability_word = ability_token
        .as_word()
        .ok_or_else(|| primitives::backtrack_err("landwalk ability", "single ability word"))?;
    Ok(LandwalkBlockOverrideClause {
        subject_tokens: trim_lexed_commas(subject_tokens),
        ability_word,
    })
}

fn parse_granted_escape_cost_tail_clause_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<GrantedEscapeCostTail<'a>> {
    primitives::any_phrase(GRANTED_ESCAPE_COST_PREFIX_PHRASES).parse_next(input)?;
    primitives::kw("exile").parse_next(input)?;
    let exile_count_tokens = take_until_phrase(input, GRANTED_ESCAPE_EXILE_TAIL_PHRASES)?;
    primitives::any_phrase(GRANTED_ESCAPE_EXILE_TAIL_PHRASES).parse_next(input)?;
    Ok(GrantedEscapeCostTail {
        exile_count_tokens: trim_lexed_commas(exile_count_tokens),
    })
}

fn parse_granted_miracle_cost_reduction_tail_clause_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<GrantedMiracleCostReductionTail<'a>> {
    primitives::any_phrase(GRANTED_MIRACLE_COST_REDUCED_PREFIX_PHRASES).parse_next(input)?;
    let reduction_cost_tokens: &'a [OwnedLexToken] = rest.parse_next(input)?;
    let reduction_cost_tokens = trim_lexed_commas(reduction_cost_tokens);
    if reduction_cost_tokens.is_empty() {
        return Err(primitives::backtrack_err(
            "miracle reduction",
            "nonempty reduction cost",
        ));
    }
    Ok(GrantedMiracleCostReductionTail {
        reduction_cost_tokens,
    })
}

fn parse_cant_be_blocked_by_more_than_clause_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<CantBeBlockedByMoreThanClause<'a>> {
    let subject_tokens = take_until_phrase(input, CANT_BE_BLOCKED_BY_PHRASES)?;
    primitives::any_phrase(CANT_BE_BLOCKED_BY_PHRASES).parse_next(input)?;
    let blocker_threshold_tokens = take_until_phrase(input, &[&["creature"], &["creatures"]])?;
    primitives::any_phrase(&[&["creature"], &["creatures"]]).parse_next(input)?;
    Ok(CantBeBlockedByMoreThanClause {
        subject_tokens: trim_lexed_commas(subject_tokens),
        blocker_threshold_tokens: trim_lexed_commas(blocker_threshold_tokens),
    })
}

fn parse_can_block_additional_creature_clause_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<CanBlockAdditionalCreatureClause<'a>> {
    let subject_tokens = take_until_phrase(input, &[&["can", "block"]])?;
    primitives::phrase(&["can", "block"]).parse_next(input)?;
    let additional_count_tokens = take_until_phrase(
        input,
        &[&["additional", "creature"], &["additional", "creatures"]],
    )?;
    primitives::any_phrase(&[&["additional", "creature"], &["additional", "creatures"]])
        .parse_next(input)?;
    primitives::phrase(&["each", "combat"]).parse_next(input)?;
    Ok(CanBlockAdditionalCreatureClause {
        subject_tokens: trim_lexed_commas(subject_tokens),
        additional_count_tokens: trim_lexed_commas(additional_count_tokens),
    })
}

fn take_until_phrase<'a>(
    input: &mut LexStream<'a>,
    phrases: &'static [&'static [&'static str]],
) -> WResult<&'a [OwnedLexToken]> {
    repeat_till(1.., any.void(), peek(primitives::any_phrase(phrases)))
        .map(|((), _)| ())
        .take()
        .parse_next(input)
}

fn trim_anthem_clause_tokens(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    let tokens = trim_lexed_commas(tokens);
    let end = if tokens
        .last()
        .is_some_and(|token| token.kind == TokenKind::Period)
    {
        tokens.len().saturating_sub(1)
    } else {
        tokens.len()
    };
    trim_lexed_commas(&tokens[..end])
}

fn word_phrase_occurs(words: &[&str], expected: &'static [&'static str]) -> bool {
    let mut input: primitives::WordSliceInput<'_> = words;
    loop {
        let mut candidate = input;
        if parse_word_phrase_input(&mut candidate, expected).is_ok() {
            return true;
        }
        let parsed: WResult<&str> = any.parse_next(&mut input);
        if parsed.is_err() {
            return false;
        }
    }
}

fn parse_word_phrase_input<'a>(
    input: &mut primitives::WordSliceInput<'a>,
    expected: &'static [&'static str],
) -> WResult<()> {
    for word in expected {
        primitives::word_slice_exact(word).parse_next(input)?;
    }
    Ok(())
}

fn first_word_offset(words: &[&str], expected: &[&str]) -> Option<usize> {
    let mut input: primitives::WordSliceInput<'_> = words;
    let initial_len = input.len();
    while let Ok(word) = take_word(&mut input) {
        if expected.contains(&word) {
            return Some(initial_len.saturating_sub(input.len() + 1));
        }
    }
    None
}

fn first_phrase_offset(words: &[&str], expected: &'static [&'static str]) -> Option<usize> {
    let mut input: primitives::WordSliceInput<'_> = words;
    let initial_len = input.len();
    loop {
        let offset = initial_len.saturating_sub(input.len());
        let mut candidate = input;
        if parse_word_phrase_input(&mut candidate, expected).is_ok() {
            return Some(offset);
        }
        let parsed: WResult<&str> = any.parse_next(&mut input);
        parsed.ok()?;
    }
}

fn take_word<'a>(input: &mut primitives::WordSliceInput<'a>) -> WResult<&'a str> {
    any.parse_next(input)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AnthemWordClass {
    And,
    Be,
    Get,
    GetOrBe,
    Have,
    Lose,
}

fn parse_anthem_word_class(input: &mut LexStream<'_>, expected: AnthemWordClass) -> WResult<()> {
    match expected {
        AnthemWordClass::And => primitives::kw("and").void().parse_next(input),
        AnthemWordClass::Be => alt((primitives::kw("is"), primitives::kw("are")))
            .void()
            .parse_next(input),
        AnthemWordClass::Get => alt((primitives::kw("get"), primitives::kw("gets")))
            .void()
            .parse_next(input),
        AnthemWordClass::GetOrBe => alt((
            primitives::kw("get"),
            primitives::kw("gets"),
            primitives::kw("is"),
            primitives::kw("are"),
        ))
        .void()
        .parse_next(input),
        AnthemWordClass::Have => alt((primitives::kw("has"), primitives::kw("have")))
            .void()
            .parse_next(input),
        AnthemWordClass::Lose => alt((primitives::kw("lose"), primitives::kw("loses")))
            .void()
            .parse_next(input),
    }
}

fn token_word_occurs(tokens: &[OwnedLexToken], expected: AnthemWordClass) -> bool {
    let mut input = LexStream::new(tokens);
    loop {
        let mut candidate = input.clone();
        if parse_anthem_word_class(&mut candidate, expected).is_ok() {
            return true;
        }
        if take_token(&mut input).is_err() {
            return false;
        }
    }
}

fn take_token<'a>(input: &mut LexStream<'a>) -> WResult<&'a OwnedLexToken> {
    any.parse_next(input)
}

fn first_token_word(tokens: &[OwnedLexToken], expected: AnthemWordClass) -> Option<usize> {
    let mut input = LexStream::new(tokens);
    let initial_len = input.len();
    loop {
        let offset = initial_len.saturating_sub(input.len());
        let mut candidate = input.clone();
        if parse_anthem_word_class(&mut candidate, expected).is_ok() {
            return Some(offset);
        }
        take_token(&mut input).ok()?;
    }
}

#[cfg(test)]
#[path = "anthem_grants_inline_tests.rs"]
mod tests;

#[path = "anthem_grants/object_action_programs.rs"]
mod object_action_programs;
use object_action_programs::{
    first_token_kind_from, token_any_phrase_complete, token_phrase_complete, token_phrase_prefix,
};
#[path = "anthem_grants/condition_programs.rs"]
mod condition_programs;
use condition_programs::find_cant_gain_tail;
#[path = "anthem_grants/combat_programs.rs"]
mod combat_programs;
use combat_programs::find_no_defender_phrase;
