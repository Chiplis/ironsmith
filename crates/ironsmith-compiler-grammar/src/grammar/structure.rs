use winnow::combinator::alt;
use winnow::error::{ContextError, ErrMode, StrContext, StrContextValue};
use winnow::prelude::*;
use winnow::stream::Stream;
use winnow::token::any;

use crate::cards::TextSpan;
use crate::cards::builders::{
    CardTextError, EffectAst, IfResultPredicate, PlayerAst, PredicateAst, SubjectVerbActionAst,
    SubjectVerbRoleAst,
};
use crate::effect::{Comparison, Value, ValueComparisonOperator};
use crate::target::PlayerFilter;
use ironsmith_core::ValueSurfaceHint;

use super::super::lexer::{
    LexStream, LexToken, OwnedLexToken, TokenKind, TokenWordView, contains_token_kind,
    trim_lexed_commas, word_slice_strip_prefix,
};
use super::super::util::{parse_card_type, parse_subtype_flexible};
use super::{leaf, primitives, values};

const QUOTED_GRANT_EXPLICIT_HEAD_PREFIXES: &[&[&str]] = &[&["this"], &["it"], &["all"], &["each"]];
const QUOTED_GRANT_OBJECT_FILTER_WORDS: &[&str] = &[
    "commander",
    "commanders",
    "permanent",
    "permanents",
    "spell",
    "spells",
];
#[path = "structure/trigger_shapes.rs"]
mod trigger_shapes;

fn token_matches_dynamic_word(token: &OwnedLexToken, expected: &str) -> bool {
    token
        .as_word()
        .is_some_and(|word| word.eq_ignore_ascii_case(expected))
}

fn structure_token_is(token: &OwnedLexToken, expected: &str) -> bool {
    token
        .as_word()
        .is_some_and(|_| token.parser_text() == expected)
}

fn structure_token_is_any(token: &OwnedLexToken, candidates: &[&str]) -> bool {
    token
        .as_word()
        .is_some_and(|_| structure_word_is_any(token.parser_text(), candidates))
}

fn find_token_word(tokens: &[OwnedLexToken], expected: &str) -> Option<usize> {
    let mut idx = 0usize;
    while idx < tokens.len() {
        if structure_token_is(&tokens[idx], expected) {
            return Some(idx);
        }
        idx += 1;
    }
    None
}

fn structure_token_kind_index(tokens: &[OwnedLexToken], kind: TokenKind) -> Option<usize> {
    let mut idx = 0usize;
    while idx < tokens.len() {
        if tokens[idx].kind == kind {
            return Some(idx);
        }
        idx += 1;
    }
    None
}

fn structure_token_kind_rindex(tokens: &[OwnedLexToken], kind: TokenKind) -> Option<usize> {
    let mut idx = tokens.len();
    while idx > 0 {
        idx -= 1;
        if tokens[idx].kind == kind {
            return Some(idx);
        }
    }
    None
}

fn structure_word_is_any(word: &str, candidates: &[&str]) -> bool {
    candidates.contains(&word)
}

fn phrase_occurs(tokens: &[OwnedLexToken], phrase: &'static [&'static str]) -> bool {
    primitives::find_prefix(tokens, || primitives::phrase(phrase)).is_some()
}

fn word_phrase_occurs(tokens: &[OwnedLexToken], phrase: &[&str]) -> bool {
    let words = crate::lexer::token_word_refs(tokens);
    crate::word_primitives::sequence_or_empty_occurs(&words, phrase)
}

fn predicate_candidate_contains_search_action(tokens: &[OwnedLexToken]) -> bool {
    phrase_occurs(tokens, &["search", "your", "library"])
}

fn predicate_candidate_contains_damage_action(tokens: &[OwnedLexToken]) -> bool {
    let Some(deal_idx) = crate::slice_primitives::select_position(tokens, |token| {
        structure_token_is_any(token, &["deal", "deals"])
    }) else {
        return false;
    };
    crate::slice_primitives::select_position(&tokens[deal_idx + 1..], |token| {
        structure_token_is(token, "damage")
    })
    .is_some()
}

fn one_of_phrases_occurs(
    tokens: &[OwnedLexToken],
    phrases: &'static [&'static [&'static str]],
) -> bool {
    primitives::find_prefix(tokens, || primitives::any_phrase(phrases)).is_some()
}

fn structure_word_index_any(words: &[&str], candidates: &[&str]) -> Option<usize> {
    let mut idx = 0usize;
    while idx < words.len() {
        if structure_word_is_any(words[idx], candidates) {
            return Some(idx);
        }
        idx += 1;
    }
    None
}

fn structure_push_unique<T: PartialEq>(items: &mut Vec<T>, value: T) {
    let mut idx = 0usize;
    while idx < items.len() {
        if items[idx] == value {
            return;
        }
        idx += 1;
    }
    items.push(value);
}

#[derive(Debug, Clone, PartialEq)]
pub struct ModalHeaderChooseSpec {
    pub choose_idx: usize,
    pub min: Value,
    pub max: Option<Value>,
    pub random: bool,
    pub x_clause_start: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModalHeaderFlags {
    pub commander_allows_both: bool,
    pub choose_both_control_card_types: Vec<crate::types::CardType>,
    pub choose_both_exact_life_total: Option<i32>,
    pub same_mode_more_than_once: bool,
    pub mode_must_be_unchosen: bool,
    pub mode_must_be_unchosen_this_turn: bool,
    pub distinct_player_targets_per_mode: bool,
    pub if_kicked_choose_any_number: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TrailingModalGateSpec<'a> {
    pub prefix_tokens: &'a [OwnedLexToken],
    pub predicate: IfResultPredicate,
    pub remove_mode_only: bool,
    pub reflexive: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetadataLineKind {
    ManaCost,
    TypeLine,
    FirstPrintedSet,
    AttractionLights,
    PowerToughness,
    Loyalty,
    Defense,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetadataLineSpec<'a> {
    pub kind: MetadataLineKind,
    pub value_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatementLineFamily {
    Emblem,
    PactNextUpkeep,
    NextTurnCantCast,
    Divvy,
    ArtRating,
    ExilePlayCostsMore,
    BidLife,
    Vote,
    Generic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StaticLineFamily {
    UntapAllDuringEachOtherPlayersUntapStep,
    GrantedQuotedAbility,
    Generic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeadingResultPrefixKind {
    If,
    When,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LeadingResultPrefixSpec<'a> {
    pub kind: LeadingResultPrefixKind,
    pub predicate: IfResultPredicate,
    pub trailing_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, PartialEq)]
pub struct TrailingIfClauseSpec<'a> {
    pub leading_tokens: &'a [OwnedLexToken],
    pub predicate: PredicateAst,
}

#[derive(Debug, Clone, PartialEq)]
pub enum IfClausePredicateSpec {
    Conditional(PredicateAst),
    Result(IfResultPredicate),
}

#[derive(Debug, Clone)]
pub struct IfClauseSplitSpec {
    pub predicate: IfClausePredicateSpec,
    pub effects: Vec<EffectAst>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConditionalPredicateTailSpec {
    Plain(PredicateAst),
    InsteadIf {
        base_predicate: PredicateAst,
        outer_predicate: PredicateAst,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct TriggeredConditionalClauseSpec<'a> {
    pub trigger_tokens: &'a [OwnedLexToken],
    pub predicate: PredicateAst,
    pub effects_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, PartialEq)]
pub struct StateTriggeredClauseSpec<'a> {
    pub trigger_tokens: &'a [OwnedLexToken],
    pub display_tokens: &'a [OwnedLexToken],
    pub predicate: PredicateAst,
    pub effects_tokens: &'a [OwnedLexToken],
}

fn is_sentence_quote(token: &LexToken) -> bool {
    token.kind == TokenKind::Quote && matches!(token.slice.as_str(), "\"" | "“" | "”")
}

fn parse_remove_mode_only_prefix<'a>(
    input: &mut LexStream<'a>,
) -> Result<(), ErrMode<ContextError>> {
    (
        alt((primitives::kw("you"), primitives::kw("they"))),
        alt((primitives::kw("remove"), primitives::kw("removed"))),
    )
        .void()
        .parse_next(input)
}

pub fn split_metadata_line_lexed(tokens: &[OwnedLexToken]) -> Option<MetadataLineSpec<'_>> {
    fn match_metadata_prefix<'a>(
        tokens: &'a [OwnedLexToken],
        phrase: &'static [&'static str],
        kind: MetadataLineKind,
    ) -> Option<MetadataLineSpec<'a>> {
        let (_, value_tokens) =
            primitives::parse_prefix(tokens, (primitives::phrase(phrase), primitives::colon()))?;
        Some(MetadataLineSpec { kind, value_tokens })
    }

    match_metadata_prefix(tokens, &["mana", "cost"], MetadataLineKind::ManaCost)
        .or_else(|| match_metadata_prefix(tokens, &["type", "line"], MetadataLineKind::TypeLine))
        .or_else(|| match_metadata_prefix(tokens, &["type"], MetadataLineKind::TypeLine))
        .or_else(|| {
            match_metadata_prefix(
                tokens,
                &["first", "printed", "set"],
                MetadataLineKind::FirstPrintedSet,
            )
        })
        .or_else(|| {
            match_metadata_prefix(
                tokens,
                &["attraction", "lights"],
                MetadataLineKind::AttractionLights,
            )
        })
        .or_else(|| {
            match_metadata_prefix(
                tokens,
                &["power/toughness"],
                MetadataLineKind::PowerToughness,
            )
        })
        .or_else(|| match_metadata_prefix(tokens, &["loyalty"], MetadataLineKind::Loyalty))
        .or_else(|| match_metadata_prefix(tokens, &["defense"], MetadataLineKind::Defense))
}

pub fn classify_statement_line_family_lexed(
    tokens: &[OwnedLexToken],
) -> Option<StatementLineFamily> {
    if super::effects::emblem_shapes::parse_emblem_payload_tokens(tokens).is_some() {
        return Some(StatementLineFamily::Emblem);
    }

    if (primitives::parse_prefix(tokens, primitives::phrase(&["starting", "with"])).is_some()
        || primitives::parse_prefix(tokens, primitives::phrase(&["each", "player", "votes"]))
            .is_some()
        || primitives::parse_prefix(
            tokens,
            primitives::phrase(&["each", "player", "secretly", "votes"]),
        )
        .is_some())
        && (primitives::contains_word(tokens, "vote")
            || primitives::contains_word(tokens, "votes")
            || primitives::contains_word(tokens, "voting"))
    {
        return Some(StatementLineFamily::Vote);
    }

    if super::effects::clause_dispatch_shapes::parse_copular_animation_shape(tokens).is_some() {
        return Some(StatementLineFamily::Generic);
    }

    // The duration distinguishes this resolving player rule from the
    // otherwise identical static ability. Route it through statement
    // lowering before the broad "has/have" anthem probe can claim it.
    if super::effects::parse_persistent_no_maximum_hand_size_player_lexed(tokens).is_some() {
        return Some(StatementLineFamily::Generic);
    }

    // Quantified object subjects can contain an arbitrary descriptor between
    // the quantifier and the verb (for example, `Each non-Vampire creature
    // gets ...`).  Let the typed subject/verb grammar recognize that shape
    // instead of guessing the verb from a fixed word offset.
    if super::effects::clause_dispatch_shapes::parse_clause_subject_verb_shape(tokens).is_some_and(
        |shape| {
            matches!(
                shape.kind,
                super::effects::chain_splitting::ChainVerbKind::Get
            ) && !shape.subject_tokens.is_empty()
        },
    ) {
        return Some(StatementLineFamily::Generic);
    }

    if phrase_occurs(
        tokens,
        &["at", "the", "beginning", "of", "your", "next", "upkeep"],
    ) && phrase_occurs(tokens, &["lose", "the", "game"])
        && (phrase_occurs(tokens, &["if", "you", "dont"])
            || phrase_occurs(tokens, &["if", "you", "don't"])
            || phrase_occurs(tokens, &["if", "you", "do", "not"]))
    {
        return Some(StatementLineFamily::PactNextUpkeep);
    }

    if one_of_phrases_occurs(
        tokens,
        &[
            &["during", "that", "player's", "next", "turn"],
            &["during", "that", "players", "next", "turn"],
        ],
    ) && one_of_phrases_occurs(
        tokens,
        &[
            &["can't", "cast"],
            &["cant", "cast"],
            &["can", "not", "cast"],
        ],
    ) {
        return Some(StatementLineFamily::NextTurnCantCast);
    }

    if one_of_phrases_occurs(
        tokens,
        &[
            &["into", "two", "piles"],
            &["into", "three", "piles"],
            &["chooses", "two", "of", "those", "cards"],
            &["chooses", "one", "of", "those", "piles"],
            &["pile", "of", "your", "choice"],
            &["pile", "of", "that", "player's", "choice"],
            &["chosen", "pile"],
            &["chosen", "piles"],
        ],
    ) {
        return Some(StatementLineFamily::Divvy);
    }

    if phrase_occurs(
        tokens,
        &[
            "ask", "a", "person", "outside", "the", "game", "to", "rate", "its", "new", "art",
            "on", "a", "scale", "from", "1", "to", "5",
        ],
    ) {
        return Some(StatementLineFamily::ArtRating);
    }

    if one_of_phrases_occurs(tokens, &[&["become"], &["becomes"]])
        && phrase_occurs(tokens, &["until", "end", "of", "turn"])
    {
        return Some(StatementLineFamily::Generic);
    }

    if primitives::parse_prefix(
        tokens,
        primitives::phrase(&["after", "you", "roll", "a", "die"]),
    )
    .is_some()
        && phrase_occurs(tokens, &["you", "may", "pay"])
        && phrase_occurs(tokens, &["increase", "or", "decrease", "the", "result"])
    {
        return Some(StatementLineFamily::Generic);
    }

    let sentence_words_match = |sentence_tokens: &[OwnedLexToken], expected: &[&str]| {
        let words = TokenWordView::new(sentence_tokens);
        words.len() == expected.len() && words.slice_eq(0, expected)
    };
    let sentences = split_lexed_sentences(tokens)
        .into_iter()
        .filter(|sentence| !sentence.is_empty())
        .collect::<Vec<_>>();
    if matches!(
        sentences.as_slice(),
        [first, second, third]
            if sentence_words_match(first, &["exile", "target", "nonland", "permanent"])
                && sentence_words_match(
                    second,
                    &[
                        "for", "as", "long", "as", "that", "card", "remains", "exiled", "its",
                        "owner", "may", "play", "it",
                    ],
                )
                && sentence_words_match(
                    third,
                    &[
                        "a", "spell", "cast", "by", "an", "opponent", "this", "way", "costs",
                        "2", "more", "to", "cast",
                    ],
                )
    ) {
        return Some(StatementLineFamily::ExilePlayCostsMore);
    }

    if primitives::parse_prefix(tokens, primitives::phrase(&["starting", "with", "you"])).is_some()
        && phrase_occurs(tokens, &["each", "player"])
        && primitives::contains_word(tokens, "pay")
    {
        return Some(StatementLineFamily::Generic);
    }

    if phrase_occurs(tokens, &["bid", "life"])
        && phrase_occurs(tokens, &["high", "bid"])
        && phrase_occurs(tokens, &["high", "bidder"])
    {
        return Some(StatementLineFamily::BidLife);
    }

    let words = tokens
        .iter()
        .filter_map(OwnedLexToken::as_word)
        .map(|word| word.to_ascii_lowercase())
        .collect::<Vec<_>>();
    let word_refs = words.iter().map(String::as_str).collect::<Vec<_>>();
    if word_refs.is_empty() {
        return None;
    }

    let starts_with_each_player_statement =
        word_slice_strip_prefix(&word_refs, &["each", "player"])
            .and_then(|rest| rest.first())
            .is_some_and(|word| is_statement_verb_word(word));
    let starts_with_each_other_player_statement =
        word_slice_strip_prefix(&word_refs, &["each", "other", "player"])
            .or_else(|| word_slice_strip_prefix(&word_refs, &["each", "other", "players"]))
            .and_then(|rest| rest.first())
            .is_some_and(|word| is_statement_verb_word(word));
    let starts_with_all_quantified_statement = word_slice_strip_prefix(&word_refs, &["all"])
        .is_some_and(|rest| rest.iter().any(|word| is_statement_verb_word(word)));
    let starts_with_quantified_target_player_statement = word_refs
        .get(1..)
        .and_then(|tail| {
            word_slice_strip_prefix(tail, &["target", "player"])
                .or_else(|| word_slice_strip_prefix(tail, &["target", "players"]))
        })
        .and_then(|rest| rest.first())
        .is_some_and(|word| is_statement_verb_word(word));
    let starts_with_this_spell_statement = word_slice_strip_prefix(&word_refs, &["this", "spell"])
        .and_then(|rest| rest.first())
        .is_some_and(|word| is_statement_verb_word(word));

    (starts_with_each_player_statement
        || starts_with_each_other_player_statement
        || starts_with_all_quantified_statement
        || starts_with_quantified_target_player_statement
        || is_statement_verb_word(word_refs[0])
        || starts_with_this_spell_statement
        || word_refs
            .get(1)
            .is_some_and(|word| is_statement_verb_word(word))
        || word_slice_strip_prefix(&word_refs, &["target"])
            .is_some_and(|rest| rest.iter().any(|word| is_statement_verb_word(word))))
    .then_some(StatementLineFamily::Generic)
}

fn is_statement_verb_word(word: &str) -> bool {
    matches!(
        word,
        "add"
            | "adds"
            | "choose"
            | "chooses"
            | "counter"
            | "counters"
            | "create"
            | "creates"
            | "deal"
            | "deals"
            | "destroy"
            | "destroys"
            | "discard"
            | "discards"
            | "draw"
            | "draws"
            | "become"
            | "becomes"
            | "bid"
            | "bids"
            | "enchant"
            | "enchants"
            | "exchange"
            | "exchanges"
            | "exile"
            | "exiles"
            | "gain"
            | "gains"
            | "get"
            | "gets"
            | "look"
            | "looks"
            | "lose"
            | "loses"
            | "mill"
            | "mills"
            | "note"
            | "notes"
            | "put"
            | "puts"
            | "return"
            | "returns"
            | "reveal"
            | "reveals"
            | "roll"
            | "rolls"
            | "sacrifice"
            | "sacrifices"
            | "search"
            | "searches"
            | "shuffle"
            | "shuffles"
            | "surveil"
            | "tap"
            | "taps"
            | "until"
            | "untap"
            | "untaps"
    )
}

fn quoted_grant_head_looks_like_object_filter(head: &[OwnedLexToken]) -> bool {
    if primitives::parse_prefix(
        head,
        primitives::any_phrase(QUOTED_GRANT_EXPLICIT_HEAD_PREFIXES),
    )
    .is_some()
    {
        return true;
    }
    let words = TokenWordView::new(head).to_word_refs();
    let Some(have_idx) = structure_word_index_any(&words, &["has", "have"]) else {
        return false;
    };
    words[..have_idx].iter().any(|word| {
        structure_word_is_any(word, QUOTED_GRANT_OBJECT_FILTER_WORDS)
            || parse_card_type(word).is_some()
            || parse_subtype_flexible(word).is_some()
    })
}

pub fn classify_static_line_family_lexed(tokens: &[OwnedLexToken]) -> Option<StaticLineFamily> {
    if super::abilities::split_untap_each_other_players_untap_step_line_lexed(tokens).is_some() {
        return Some(StaticLineFamily::UntapAllDuringEachOtherPlayersUntapStep);
    }

    if one_of_phrases_occurs(
        tokens,
        &[
            &[
                "you",
                "may",
                "cast",
                "this",
                "card",
                "from",
                "your",
                "graveyard",
            ],
            &[
                "you",
                "may",
                "cast",
                "this",
                "spell",
                "from",
                "your",
                "graveyard",
            ],
            &[
                "once", "each", "turn", "you", "may", "cast", "a", "spell", "from", "the", "top",
                "of", "your", "library",
            ],
            &[
                "once", "each", "turn", "you", "may", "cast", "spells", "from", "the", "top", "of",
                "your", "library",
            ],
            &["if", "you", "would", "draw", "a", "card"],
        ],
    ) || contains_characteristic_equal_to_shape(tokens)
    {
        return Some(StaticLineFamily::Generic);
    }

    if let Some(quote_idx) = structure_token_kind_index(tokens, TokenKind::Quote) {
        let head = trim_lexed_commas(&tokens[..quote_idx]);
        if !head.is_empty()
            && !contains_token_kind(head, TokenKind::Period)
            && quoted_grant_head_looks_like_object_filter(head)
            && (primitives::contains_word(head, "has") || primitives::contains_word(head, "have"))
        {
            return Some(StaticLineFamily::GrantedQuotedAbility);
        }
    }

    (primitives::parse_prefix(tokens, primitives::phrase(&["this"])).is_some()
        || primitives::parse_prefix(tokens, primitives::phrase(&["enchanted"])).is_some()
        || primitives::parse_prefix(tokens, primitives::phrase(&["equipped"])).is_some()
        || primitives::parse_prefix(tokens, primitives::phrase(&["fortified"])).is_some()
        || primitives::parse_prefix(tokens, primitives::phrase(&["spells"])).is_some()
        || primitives::parse_prefix(tokens, primitives::phrase(&["creatures"])).is_some()
        || primitives::parse_prefix(tokens, primitives::phrase(&["other"])).is_some()
        || primitives::parse_prefix(tokens, primitives::phrase(&["each"])).is_some()
        || primitives::parse_prefix(tokens, primitives::phrase(&["as", "long", "as"])).is_some()
        || primitives::contains_word(tokens, "can't")
        || primitives::contains_word(tokens, "cant")
        || primitives::contains_word(tokens, "cannot")
        || primitives::contains_word(tokens, "can")
        || primitives::contains_word(tokens, "has")
        || primitives::contains_word(tokens, "have")
        || phrase_occurs(tokens, &["maximum", "hand", "size"]))
    .then_some(StaticLineFamily::Generic)
}

fn contains_characteristic_equal_to_shape(tokens: &[OwnedLexToken]) -> bool {
    primitives::has_phrase(tokens, &["power", "is", "equal", "to"])
        || primitives::has_phrase(tokens, &["toughness", "is", "equal", "to"])
}

fn parse_modeled_predicate(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    fn life_relation_predicate(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
        use crate::grammar::conditions::PlayerLifeRelationAst;

        let relation = crate::grammar::conditions::parse_player_life_relation_condition(tokens)?;
        let player = match relation.player {
            PlayerFilter::You => PlayerAst::You,
            PlayerFilter::Opponent => PlayerAst::Opponent,
            PlayerFilter::Any => PlayerAst::Any,
            PlayerFilter::IteratedPlayer => PlayerAst::That,
            _ => return None,
        };
        match relation.relation {
            PlayerLifeRelationAst::HasMoreLifeThanYou => {
                Some(PredicateAst::PlayerHasMoreLifeThanYou { player })
            }
            PlayerLifeRelationAst::HasLessLifeThanYou => {
                Some(PredicateAst::PlayerHasLessLifeThanYou { player })
            }
            PlayerLifeRelationAst::HasNoOpponentWithMoreLifeThan => {
                Some(PredicateAst::PlayerHasNoOpponentWithMoreLifeThan { player })
            }
            PlayerLifeRelationAst::HasMoreLifeThanEachOtherPlayer => {
                Some(PredicateAst::PlayerHasMoreLifeThanEachOtherPlayer { player })
            }
        }
    }

    // Intervening-if clauses may list several complete predicates separated by
    // commas, with the final item introduced by "and". Preserve those clauses
    // as one conjunction instead of parsing only the final predicate fragment.
    let mut parts = Vec::new();
    let mut start = 0;
    let mut inside_quotes = false;
    for (index, token) in tokens.iter().enumerate() {
        if is_sentence_quote(token) {
            inside_quotes = !inside_quotes;
        } else if !inside_quotes && token.is_comma() {
            let part = trim_lexed_commas(&tokens[start..index]);
            if !part.is_empty() {
                parts.push(part);
            }
            start = index + 1;
        }
    }
    let tail = trim_lexed_commas(&tokens[start..]);
    if !tail.is_empty() {
        parts.push(tail);
    }
    if parts.len() < 2 {
        return parse_predicate_with_grammar_entrypoint_lexed(tokens)
            .ok()
            .or_else(|| life_relation_predicate(tokens));
    }

    let mut predicates = Vec::with_capacity(parts.len());
    for mut part in parts {
        if part
            .first()
            .is_some_and(|token| structure_token_is(token, "and"))
        {
            part = trim_lexed_commas(&part[1..]);
        }
        let Ok(predicate) = parse_predicate_with_grammar_entrypoint_lexed(part) else {
            return parse_predicate_with_grammar_entrypoint_lexed(tokens)
                .ok()
                .or_else(|| life_relation_predicate(tokens));
        };
        predicates.push(predicate);
    }
    let mut predicates = predicates.into_iter();
    let first = predicates.next()?;
    Some(predicates.fold(first, |left, right| {
        PredicateAst::And(Box::new(left), Box::new(right))
    }))
}

pub fn parse_if_result_predicate(tokens: &[OwnedLexToken]) -> Option<IfResultPredicate> {
    super::modal_results::parse_if_result_predicate_lexed_tokens(tokens)
}

fn parse_sentence_segment_len<'a>(
    input: &mut LexStream<'a>,
) -> Result<usize, ErrMode<ContextError>> {
    fn quoted_period_continues_sentence(next: Option<&LexToken>) -> bool {
        match next {
            Some(token) if token.kind == TokenKind::Comma => true,
            Some(token)
                if token.kind == TokenKind::Word
                    && matches!(
                        token.parser_text(),
                        "and" | "during" | "for" | "this" | "until" | "where" | "with" | "without"
                    ) =>
            {
                true
            }
            _ => false,
        }
    }

    let initial_len = input.len();
    let mut inside_quotes = false;
    let mut last_inner_token_was_period = false;

    while let Some(token) = input.peek_token() {
        if is_sentence_quote(token) {
            primitives::quote().parse_next(input)?;
            if inside_quotes
                && last_inner_token_was_period
                && !quoted_period_continues_sentence(input.peek_token())
            {
                let consumed = initial_len - input.len();
                return Ok(consumed);
            }
            inside_quotes = !inside_quotes;
            last_inner_token_was_period = false;
            continue;
        }

        if token.kind == TokenKind::Period {
            primitives::period().parse_next(input)?;
            if inside_quotes {
                last_inner_token_was_period = true;
                continue;
            }

            let consumed = initial_len - input.len();
            return Ok(consumed.saturating_sub(1));
        }

        any.parse_next(input)?;
        last_inner_token_was_period = false;
    }

    Ok(initial_len - input.len())
}

pub fn split_lexed_sentences(tokens: &[OwnedLexToken]) -> Vec<&[OwnedLexToken]> {
    let mut segments = Vec::new();
    let mut remaining = tokens;

    while !remaining.is_empty() {
        let Some((segment_len, rest)) =
            primitives::parse_prefix(remaining, parse_sentence_segment_len)
        else {
            break;
        };

        if segment_len > 0 {
            segments.push(&remaining[..segment_len]);
        }

        if rest.len() == remaining.len() {
            break;
        }
        remaining = rest;
    }

    segments
}

pub fn find_trigger_effect_list_tail_split_lexed(
    trigger_prefix_tokens: &[OwnedLexToken],
    tail_tokens: &[OwnedLexToken],
) -> Option<usize> {
    trigger_shapes::parse_trigger_effect_list_tail_split_lexed(trigger_prefix_tokens, tail_tokens)
        .map(|split| split.split_token_idx)
}

pub fn split_first_time_each_turn_trigger_suffix_lexed(
    trigger_tokens: &[OwnedLexToken],
) -> (&[OwnedLexToken], Option<u32>) {
    trigger_shapes::parse_first_time_each_turn_trigger_suffix_lexed(trigger_tokens)
        .map(|split| (split.trigger_tokens, Some(split.limit)))
        .unwrap_or((trigger_tokens, None))
}

pub fn rewrite_attached_controller_trigger_effect_tokens_lexed(
    trigger_tokens: &[OwnedLexToken],
    effects_tokens: &[OwnedLexToken],
) -> Vec<OwnedLexToken> {
    trigger_shapes::rewrite_attached_controller_effect_tokens_lexed(trigger_tokens, effects_tokens)
}
pub fn scan_modal_header_flags(tokens: &[OwnedLexToken]) -> ModalHeaderFlags {
    let mode_must_be_unchosen_this_turn = one_of_phrases_occurs(
        tokens,
        &[
            &["that", "hasnt", "been", "chosen", "this", "turn"],
            &["that", "hasn't", "been", "chosen", "this", "turn"],
            &["that", "has", "not", "been", "chosen", "this", "turn"],
        ],
    );
    let mode_must_be_unchosen = mode_must_be_unchosen_this_turn
        || one_of_phrases_occurs(
            tokens,
            &[
                &["that", "hasnt", "been", "chosen"],
                &["that", "hasn't", "been", "chosen"],
                &["that", "has", "not", "been", "chosen"],
            ],
        );

    let choose_both_control_card_types = scan_choose_both_control_card_types(tokens);
    let choose_both_exact_life_total = scan_choose_both_exact_life_total(tokens);

    ModalHeaderFlags {
        commander_allows_both: primitives::contains_word(tokens, "commander")
            && primitives::contains_word(tokens, "both"),
        choose_both_control_card_types,
        choose_both_exact_life_total,
        same_mode_more_than_once: phrase_occurs(tokens, &["same", "mode", "more", "than", "once"]),
        mode_must_be_unchosen,
        mode_must_be_unchosen_this_turn,
        distinct_player_targets_per_mode: phrase_occurs(
            tokens,
            &["each", "mode", "must", "target", "a", "different", "player"],
        ),
        if_kicked_choose_any_number: word_phrase_occurs(
            tokens,
            &[
                "if", "this", "spell", "was", "kicked", "choose", "any", "number", "instead",
            ],
        ),
    }
}

fn scan_choose_both_control_card_types(tokens: &[OwnedLexToken]) -> Vec<crate::types::CardType> {
    if !phrase_occurs(tokens, &["you", "may", "choose", "both", "instead"]) {
        return Vec::new();
    }
    let Some(if_idx) = find_token_word(tokens, "if") else {
        return Vec::new();
    };
    let Some(control_idx) =
        find_token_word(&tokens[if_idx + 1..], "control").map(|idx| if_idx + 1 + idx)
    else {
        return Vec::new();
    };
    let Some(as_idx) =
        find_token_word(&tokens[control_idx + 1..], "as").map(|idx| control_idx + 1 + idx)
    else {
        return Vec::new();
    };
    if as_idx <= control_idx + 1 {
        return Vec::new();
    }

    let mut card_types = Vec::new();
    for token in &tokens[control_idx + 1..as_idx] {
        if token.kind != TokenKind::Word {
            continue;
        }
        if let Some(card_type) = parse_card_type(token.parser_text()) {
            structure_push_unique(&mut card_types, card_type);
        }
    }
    card_types
}

fn scan_choose_both_exact_life_total(tokens: &[OwnedLexToken]) -> Option<i32> {
    if !phrase_occurs(tokens, &["you", "may", "choose", "both", "instead"]) {
        return None;
    }

    let mut idx = 0usize;
    while idx + 4 < tokens.len() {
        if tokens[idx].is_word("you")
            && tokens[idx + 1].is_word("have")
            && tokens[idx + 2].is_word("exactly")
            && tokens[idx + 4].is_word("life")
        {
            return match tokens[idx + 3].kind {
                TokenKind::Number | TokenKind::Word => {
                    super::leaf::parse_number_i32_complete(tokens[idx + 3].parser_text()).ok()
                }
                _ => None,
            };
        }
        idx += 1;
    }

    None
}

pub fn split_leading_result_prefix_lexed<'a>(
    tokens: &'a [OwnedLexToken],
) -> Option<LeadingResultPrefixSpec<'a>> {
    let trimmed = trim_lexed_commas(tokens);
    if let Some((predicate, trailing_tokens)) = split_leading_numeric_result_prefix_lexed(trimmed) {
        return Some(LeadingResultPrefixSpec {
            kind: LeadingResultPrefixKind::If,
            predicate,
            trailing_tokens,
        });
    }
    let kind = if trimmed
        .first()
        .is_some_and(|token| structure_token_is(token, "if"))
    {
        LeadingResultPrefixKind::If
    } else if trimmed
        .first()
        .is_some_and(|token| structure_token_is(token, "when"))
    {
        LeadingResultPrefixKind::When
    } else {
        return None;
    };

    let comma_idx = structure_token_kind_index(trimmed, TokenKind::Comma)?;
    if comma_idx <= 1 || comma_idx + 1 >= trimmed.len() {
        return None;
    }

    let predicate_tokens = trim_lexed_commas(&trimmed[1..comma_idx]);
    if predicate_tokens.is_empty() {
        return None;
    }
    let predicate = parse_if_result_predicate(predicate_tokens)?;

    let trailing_tokens = trim_lexed_commas(&trimmed[comma_idx + 1..]);
    if trailing_tokens.is_empty() {
        return None;
    }

    Some(LeadingResultPrefixSpec {
        kind,
        predicate,
        trailing_tokens,
    })
}

fn split_leading_numeric_result_prefix_lexed(
    tokens: &[OwnedLexToken],
) -> Option<(IfResultPredicate, &[OwnedLexToken])> {
    let first = tokens.first()?;
    let pipe_idx = structure_token_kind_index(tokens, TokenKind::Pipe)?;
    if pipe_idx != 1 && pipe_idx < 3 {
        return None;
    }

    let compact_range = compact_ascii_numeric_range(first);
    let predicate = if pipe_idx == 1 {
        if let Some((min, max)) = compact_range {
            IfResultPredicate::Value(Comparison::BetweenInclusive(min, max))
        } else {
            let min = match first.kind {
                TokenKind::Number => first.parser_text().parse::<i32>().ok()?,
                _ => return None,
            };
            IfResultPredicate::Value(Comparison::Equal(min))
        }
    } else {
        let min = match first.kind {
            TokenKind::Number => first.parser_text().parse::<i32>().ok()?,
            _ => return None,
        };
        let second = tokens.get(1)?;
        let third = tokens.get(2)?;
        if !matches!(second.kind, TokenKind::Dash | TokenKind::EmDash) {
            return None;
        }
        let max = match third.kind {
            TokenKind::Number => third.parser_text().parse::<i32>().ok()?,
            _ => return None,
        };
        if min > max {
            return None;
        }
        IfResultPredicate::Value(Comparison::BetweenInclusive(min, max))
    };

    let trailing_tokens = trim_lexed_commas(&tokens[pipe_idx + 1..]);
    if trailing_tokens.is_empty() {
        return None;
    }

    Some((predicate, trailing_tokens))
}

fn compact_ascii_numeric_range(token: &OwnedLexToken) -> Option<(i32, i32)> {
    if token.kind != TokenKind::Word {
        return None;
    }
    let (min, max) = crate::word_primitives::parse_ascii_numeric_range(token.parser_text())?;
    (min <= max).then_some((min, max))
}

pub fn split_trailing_if_clause_lexed<'a>(
    tokens: &'a [OwnedLexToken],
) -> Option<TrailingIfClauseSpec<'a>> {
    split_trailing_predicate_clause_lexed(tokens, "if")
}

pub fn parse_predicate_with_grammar_entrypoint_lexed(
    tokens: &[OwnedLexToken],
) -> Result<PredicateAst, CardTextError> {
    super::filters::parse_predicate(tokens)
}

pub fn split_if_clause_lexed(
    tokens: &[OwnedLexToken],
    mut parse_effects: impl FnMut(&[OwnedLexToken]) -> Result<Vec<EffectAst>, CardTextError>,
) -> Result<IfClauseSplitSpec, CardTextError> {
    if let Some(tie_choice) = super::conditions::parse_player_life_tie_choice_condition(tokens) {
        let tied_players = tie_choice.tied_players;
        let mut effects = vec![EffectAst::subject_verb_choose_player(
            PlayerAst::You,
            tied_players.clone(),
            crate::tag::CompilerReferenceTag::It.key(),
            false,
            0,
        )];
        effects.extend(parse_effects(tie_choice.consequence_tokens)?);
        return Ok(IfClauseSplitSpec {
            predicate: IfClausePredicateSpec::Conditional(PredicateAst::ValueComparison {
                left: Value::CountPlayers(tied_players),
                operator: ValueComparisonOperator::GreaterThanOrEqual,
                right: Value::Fixed(tie_choice.minimum_players as i32),
            }),
            effects,
        });
    }

    fn split_leaves_player_may_search_subject(tokens: &[OwnedLexToken], split_idx: usize) -> bool {
        const PLAYER_MAY_SUFFIXES: &[&[&str]] = &[
            &["its", "controller", "may"],
            &["its", "controllers", "may"],
            &["its", "owner", "may"],
            &["its", "owners", "may"],
            &["that", "player", "may"],
            &["target", "player", "may"],
            &["that", "opponent", "may"],
            &["target", "opponent", "may"],
        ];
        let effect_tokens = trim_lexed_commas(&tokens[split_idx..]);
        if primitives::parse_prefix(
            effect_tokens,
            alt((primitives::kw("search"), primitives::kw("searches"))).void(),
        )
        .is_none()
        {
            return false;
        }

        let predicate_tokens = trim_lexed_commas(&tokens[..split_idx]);
        primitives::strip_lexed_suffix_phrases(predicate_tokens, PLAYER_MAY_SUFFIXES).is_some()
    }

    let parse_effects_with_leading_instead =
        |effect_tokens: &[OwnedLexToken],
         parse_effects: &mut dyn FnMut(
            &[OwnedLexToken],
        ) -> Result<Vec<EffectAst>, CardTextError>| {
            let trimmed = trim_lexed_commas(effect_tokens);
            let without_instead = if trimmed
                .first()
                .is_some_and(|token| structure_token_is(token, "instead"))
            {
                trim_lexed_commas(&trimmed[1..])
            } else {
                trimmed
            };
            parse_effects(without_instead)
        };

    if let Some(effect_token_idx) =
        primitives::find_phrase_start(tokens, &["exile", "them", "then", "meld", "them", "into"])
    {
        let predicate_tokens = trim_lexed_commas(&tokens[1..effect_token_idx]);
        let predicate_tokens_without_commas = predicate_tokens
            .iter()
            .filter(|token| !token.is_comma())
            .cloned()
            .collect::<Vec<_>>();
        let effect_tokens = &tokens[effect_token_idx..];
        if !predicate_tokens_without_commas.is_empty() {
            if let Ok(predicate) =
                parse_predicate_with_grammar_entrypoint_lexed(&predicate_tokens_without_commas)
                && let Ok(effects) =
                    parse_effects_with_leading_instead(effect_tokens, &mut parse_effects)
                && !effects.is_empty()
            {
                return Ok(IfClauseSplitSpec {
                    predicate: IfClausePredicateSpec::Conditional(predicate),
                    effects,
                });
            }
            if let Some(predicate) = parse_if_result_predicate(&predicate_tokens_without_commas)
                && let Ok(effects) =
                    parse_effects_with_leading_instead(effect_tokens, &mut parse_effects)
                && !effects.is_empty()
            {
                return Ok(IfClauseSplitSpec {
                    predicate: IfClausePredicateSpec::Result(predicate),
                    effects,
                });
            }
        }
    }

    // A quoted granted ability is part of the consequence, so punctuation
    // inside that quote must not become a candidate boundary for the outer
    // `if <predicate>, <effect>` clause.
    let mut inside_quote = false;
    let comma_indices = tokens
        .iter()
        .enumerate()
        .filter_map(|(idx, token)| {
            if token.is_quote() {
                inside_quote = !inside_quote;
                return None;
            }
            (token.is_comma() && !inside_quote).then_some(idx)
        })
        .collect::<Vec<_>>();
    if comma_indices.is_empty() {
        // With no authored comma, a broad object predicate can also accept a
        // suffix of the consequence (for example, the final `this creature`
        // in a counter-removal action). Prefer the grammar-proven ability-
        // resolution ordinal boundary before the reverse fallback searches
        // for the longest merely parseable predicate.
        for split_idx in 2..tokens.len() {
            let predicate_tokens = &tokens[1..split_idx];
            let Ok(predicate) = parse_predicate_with_grammar_entrypoint_lexed(predicate_tokens)
            else {
                continue;
            };
            if !matches!(
                predicate,
                PredicateAst::ThisAbilityResolvedThisTurnExactly(_)
            ) {
                continue;
            }
            let effect_tokens = trim_lexed_commas(&tokens[split_idx..]);
            if let Ok(effects) =
                parse_effects_with_leading_instead(effect_tokens, &mut parse_effects)
                && !effects.is_empty()
            {
                return Ok(IfClauseSplitSpec {
                    predicate: IfClausePredicateSpec::Conditional(predicate),
                    effects,
                });
            }
        }
        for split_idx in (2..tokens.len()).rev() {
            if split_leaves_player_may_search_subject(tokens, split_idx) {
                continue;
            }
            let predicate_tokens = &tokens[1..split_idx];
            let effect_tokens = trim_lexed_commas(&tokens[split_idx..]);
            if effect_tokens.is_empty() {
                continue;
            }
            if let Ok(predicate) = parse_predicate_with_grammar_entrypoint_lexed(predicate_tokens) {
                if let Some(effects) = parse_cards_in_hand_difference_draw_effect(
                    &predicate,
                    predicate_tokens,
                    effect_tokens,
                ) {
                    return Ok(IfClauseSplitSpec {
                        predicate: IfClausePredicateSpec::Conditional(predicate),
                        effects,
                    });
                }
                if let Ok(effects) =
                    parse_effects_with_leading_instead(effect_tokens, &mut parse_effects)
                    && !effects.is_empty()
                {
                    return Ok(IfClauseSplitSpec {
                        predicate: IfClausePredicateSpec::Conditional(predicate),
                        effects,
                    });
                }
            }
            if let Some(predicate) = parse_if_result_predicate(predicate_tokens)
                && let Ok(effects) =
                    parse_effects_with_leading_instead(effect_tokens, &mut parse_effects)
                && !effects.is_empty()
            {
                return Ok(IfClauseSplitSpec {
                    predicate: IfClausePredicateSpec::Result(predicate),
                    effects,
                });
            }
        }
        return Err(CardTextError::ParseError(
            "missing comma in if clause".to_string(),
        ));
    }

    let first_comma_idx = comma_indices[0];
    if first_comma_idx > 1 {
        let predicate_tokens = &tokens[1..first_comma_idx];
        // Result-memory predicates such as
        // "If two nonland cards that share a color were milled this way"
        // are also valid ordinary object predicates. Prefer the typed result
        // parser so cardinality and cross-object characteristic constraints
        // survive into RepeatProcess/IfResult lowering.
        if let Some(predicate) = parse_if_result_predicate(predicate_tokens) {
            let effect_tokens = &tokens[first_comma_idx + 1..];
            let effects = parse_effects_with_leading_instead(effect_tokens, &mut parse_effects)?;
            return Ok(IfClauseSplitSpec {
                predicate: IfClausePredicateSpec::Result(predicate),
                effects,
            });
        }
        if let Ok(predicate) = parse_predicate_with_grammar_entrypoint_lexed(predicate_tokens) {
            let effect_tokens = &tokens[first_comma_idx + 1..];
            if let Some(effects) = parse_cards_in_hand_difference_draw_effect(
                &predicate,
                predicate_tokens,
                effect_tokens,
            ) {
                return Ok(IfClauseSplitSpec {
                    predicate: IfClausePredicateSpec::Conditional(predicate),
                    effects,
                });
            }
            // A leading `instead` unambiguously belongs to the consequence,
            // even when that consequence has another internal comma-then
            // boundary. Do not require the first consequence fragment to
            // parse independently: an anaphoric first action such as
            // `instead exile it` needs the preceding default branch for its
            // reference, while the complete `exile it, then return that card`
            // program is still a valid replacement. Otherwise the reverse
            // comma fallback can absorb that first action into the predicate
            // and retain only the terminal action.
            if trim_lexed_commas(effect_tokens)
                .first()
                .is_some_and(|token| structure_token_is(token, "instead"))
                && let Ok(effects) =
                    parse_effects_with_leading_instead(effect_tokens, &mut parse_effects)
                && !effects.is_empty()
            {
                return Ok(IfClauseSplitSpec {
                    predicate: IfClausePredicateSpec::Conditional(predicate),
                    effects,
                });
            }
            if effect_tokens
                .first()
                .is_some_and(|token| token.is_word("search") || token.is_word("searches"))
                && let Ok(effects) =
                    parse_effects_with_leading_instead(effect_tokens, &mut parse_effects)
                && !effects.is_empty()
            {
                return Ok(IfClauseSplitSpec {
                    predicate: IfClausePredicateSpec::Conditional(predicate),
                    effects,
                });
            }
            let comma_fragment_looks_like_effect = if comma_indices.len() > 1 {
                let fragment_tokens = &tokens[first_comma_idx + 1..comma_indices[1]];
                super::effects::for_each_shapes::parse_for_each_object_subject_shape(
                    fragment_tokens,
                )
                .is_some()
                    || parse_effects_with_leading_instead(fragment_tokens, &mut parse_effects)
                        .map(|effects| !effects.is_empty())
                        .unwrap_or(false)
            } else {
                true
            };
            let comma_fragment_looks_like_delayed_trigger = effect_tokens
                .first()
                .is_some_and(|token| token.is_word("when") || token.is_word("whenever"));
            if (comma_fragment_looks_like_effect || comma_fragment_looks_like_delayed_trigger)
                && let Ok(effects) =
                    parse_effects_with_leading_instead(effect_tokens, &mut parse_effects)
                && !effects.is_empty()
            {
                return Ok(IfClauseSplitSpec {
                    predicate: IfClausePredicateSpec::Conditional(predicate),
                    effects,
                });
            }
        }
    }

    let mut split: Option<(usize, Vec<EffectAst>)> = None;
    for idx in comma_indices.iter().rev().copied() {
        let effect_tokens = &tokens[idx + 1..];
        if effect_tokens.is_empty() {
            continue;
        }
        if let Ok(effects) = parse_effects_with_leading_instead(effect_tokens, &mut parse_effects)
            && !effects.is_empty()
        {
            split = Some((idx, effects));
            break;
        }
    }

    let (comma_idx, effects) = if let Some(split) = split {
        split
    } else {
        let first_idx = comma_indices[0];
        let effect_tokens = &tokens[first_idx + 1..];
        (
            first_idx,
            parse_effects_with_leading_instead(effect_tokens, &mut parse_effects)?,
        )
    };
    let predicate_tokens = &tokens[1..comma_idx];

    if let Ok(predicate) = parse_predicate_with_grammar_entrypoint_lexed(predicate_tokens) {
        return Ok(IfClauseSplitSpec {
            predicate: IfClausePredicateSpec::Conditional(predicate),
            effects,
        });
    }
    let Some(predicate) = parse_if_result_predicate(predicate_tokens) else {
        let predicate = parse_predicate_with_grammar_entrypoint_lexed(predicate_tokens)?;
        return Ok(IfClauseSplitSpec {
            predicate: IfClausePredicateSpec::Conditional(predicate),
            effects,
        });
    };

    Ok(IfClauseSplitSpec {
        predicate: IfClausePredicateSpec::Result(predicate),
        effects,
    })
}

fn parse_cards_in_hand_difference_draw_effect(
    predicate: &PredicateAst,
    predicate_tokens: &[OwnedLexToken],
    effect_tokens: &[OwnedLexToken],
) -> Option<Vec<EffectAst>> {
    let PredicateAst::PlayerCardsInHandOrFewer { player, count } = predicate else {
        return None;
    };
    if !one_of_phrases_occurs(predicate_tokens, &[&["fewer", "than"], &["less", "than"]]) {
        return None;
    }

    let effect_words = TokenWordView::new(trim_lexed_commas(effect_tokens)).to_word_refs();
    let implicit_phrases: &[&[&str]] = &[
        &["draw", "cards", "equal", "to", "the", "difference"],
        &["draw", "cards", "equal", "to", "difference"],
    ];
    let you_phrases: &[&[&str]] = &[
        &["you", "draw", "cards", "equal", "to", "the", "difference"],
        &["you", "draw", "cards", "equal", "to", "difference"],
    ];
    let subject =
        if implicit_phrases.iter().any(|expected| {
            primitives::parse_word_sequence_complete(&effect_words, expected).is_some()
        }) {
            PlayerAst::Implicit
        } else if you_phrases.iter().any(|expected| {
            primitives::parse_word_sequence_complete(&effect_words, expected).is_some()
        }) {
            PlayerAst::You
        } else {
            return None;
        };
    let hand_player = match player {
        PlayerAst::You | PlayerAst::Implicit => PlayerFilter::You,
        _ => return None,
    };
    let threshold = (*count as i32) + 1;
    let difference = Value::Add(
        Box::new(Value::Fixed(threshold)),
        Box::new(Value::Scaled(Box::new(Value::CardsInHand(hand_player)), -1)),
    )
    .with_surface_hint(ValueSurfaceHint::Difference);

    Some(vec![EffectAst::subject_verb(
        SubjectVerbRoleAst::AffectedPlayer,
        subject,
        SubjectVerbActionAst::Draw { count: difference },
    )])
}

pub fn split_trailing_unless_clause_lexed<'a>(
    tokens: &'a [OwnedLexToken],
) -> Option<TrailingIfClauseSpec<'a>> {
    split_trailing_predicate_clause_lexed(tokens, "unless")
}

pub fn parse_trailing_if_predicate_lexed(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let trimmed = trim_lexed_commas(tokens);
    if !trimmed
        .first()
        .is_some_and(|token| structure_token_is(token, "if"))
    {
        return None;
    }

    let predicate_tokens = trim_lexed_commas(&trimmed[1..]);
    if predicate_tokens.is_empty() {
        return None;
    }

    parse_predicate_with_grammar_entrypoint_lexed(predicate_tokens).ok()
}

pub fn parse_conditional_predicate_tail_lexed(
    tokens: &[OwnedLexToken],
) -> Option<ConditionalPredicateTailSpec> {
    let mut trimmed = trim_lexed_commas(tokens).to_vec();
    while trimmed
        .last()
        .is_some_and(|token| structure_token_is(token, "instead"))
    {
        trimmed.pop();
    }
    let trimmed = trim_lexed_commas(&trimmed);
    if trimmed.is_empty() {
        return None;
    }

    // A repeated conditional introducer joins peer predicates rather than
    // nesting the second gate around the first one. Keeping this boundary
    // explicit is especially important when the left predicate refers to the
    // chosen target ("it's a creature") and the right predicate refers to
    // how the spell was cast.
    if let Some(or_idx) = crate::slice_primitives::find_window_by(trimmed, 2, |pair| {
        structure_token_is(&pair[0], "or") && structure_token_is(&pair[1], "if")
    }) {
        let left_tokens = trim_lexed_commas(&trimmed[..or_idx]);
        let right_tokens = trim_lexed_commas(&trimmed[or_idx + 2..]);
        if !left_tokens.is_empty()
            && !right_tokens.is_empty()
            && let Ok(left) = parse_predicate_with_grammar_entrypoint_lexed(left_tokens)
            && let Ok(right) = parse_predicate_with_grammar_entrypoint_lexed(right_tokens)
        {
            return Some(ConditionalPredicateTailSpec::Plain(PredicateAst::Or(
                Box::new(left),
                Box::new(right),
            )));
        }
    }

    let mut instead_if_idx = None;
    let mut idx = 0usize;
    while idx < trimmed.len() {
        if primitives::parse_prefix(
            &trimmed[idx..],
            (primitives::kw("instead"), primitives::kw("if")),
        )
        .is_some()
        {
            instead_if_idx = Some(idx);
            break;
        }
        idx += 1;
    }

    if let Some(instead_if_idx) = instead_if_idx {
        let base_predicate_tokens = trim_lexed_commas(&trimmed[..instead_if_idx]);
        let outer_predicate_tokens = trim_lexed_commas(&trimmed[instead_if_idx + 2..]);
        if base_predicate_tokens.is_empty() || outer_predicate_tokens.is_empty() {
            return None;
        }

        let base_predicate =
            parse_predicate_with_grammar_entrypoint_lexed(base_predicate_tokens).ok()?;
        let outer_predicate =
            parse_predicate_with_grammar_entrypoint_lexed(outer_predicate_tokens).ok()?;
        return Some(ConditionalPredicateTailSpec::InsteadIf {
            base_predicate,
            outer_predicate,
        });
    }

    let predicate = parse_predicate_with_grammar_entrypoint_lexed(trimmed).ok()?;
    Some(ConditionalPredicateTailSpec::Plain(predicate))
}

#[cfg(test)]
#[path = "structure_inline_leading_result_prefix_regressions.rs"]
mod leading_result_prefix_regressions;

#[path = "structure/structure_choice.rs"]
mod structure_choice_programs;
use structure_choice_programs::parse_modal_header_choose_spec_inner;
pub use structure_choice_programs::{
    parse_modal_header_choose_spec, split_trailing_modal_gate_clause,
};
#[path = "structure/structure_trigger.rs"]
mod structure_trigger_programs;
pub use structure_trigger_programs::{
    split_state_triggered_clause_lexed, split_triggered_conditional_clause_lexed,
};
#[path = "structure/structure_condition.rs"]
mod structure_condition_programs;
pub use structure_condition_programs::parse_trailing_instead_if_predicate_lexed;
use structure_condition_programs::split_trailing_predicate_clause_lexed;
#[path = "structure/structure_reference.rs"]
mod structure_reference_programs;
pub use structure_reference_programs::parse_who_player_predicate_lexed;
#[path = "structure/structure_core.rs"]
mod structure_core_programs;
use structure_core_programs::rfind_unquoted_dynamic_word;
