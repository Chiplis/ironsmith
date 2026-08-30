use winnow::combinator::{alt, eof, opt, peek, repeat, repeat_till};
use winnow::error::{ContextError, ErrMode, ModalResult as WResult};
use winnow::prelude::*;
use winnow::token::any;

use super::super::super::lexer::{LexStream, OwnedLexToken, TokenKind, trim_lexed_commas};
use super::super::{leaf, primitives};
#[cfg(test)]
use crate::effect::Until;

#[path = "chain_carry/carry_facts.rs"]
mod carry_facts;
pub use carry_facts::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChainOwner {
    You,
    TargetPlayer,
    TargetOpponent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExileLibraryShuffleSpec {
    pub owner: ChainOwner,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChainPlayerScope {
    EachOpponent,
    EachPlayer,
}

pub fn explicit_target_player_count(tokens: &[OwnedLexToken]) -> usize {
    let mut count = 0usize;
    let mut remaining = tokens;
    while let Some((_, (), tail)) = primitives::find_prefix(remaining, || {
        primitives::phrase(&["target", "player"]).void()
    }) {
        count += 1;
        remaining = tail;
    }
    count
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrActionSplit<'a> {
    pub first_tokens: &'a [OwnedLexToken],
    pub second_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DestroyRestrictionSplit<'a> {
    pub destroy_tokens: &'a [OwnedLexToken],
    pub restriction_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoordinatedTargetActionKind {
    Destroy,
    Exile,
    Return,
}

/// Recognizes an independently targeted same-action prefix joined by `and`.
/// A later `then` clause is deliberately excluded from the coordinated run;
/// for example, the two returns in "return A and B, then discard" remain
/// independent targets while the discard stays sequential.
pub fn coordinated_target_action_kind(
    tokens: &[OwnedLexToken],
) -> Option<CoordinatedTargetActionKind> {
    let words = tokens
        .iter()
        .filter_map(OwnedLexToken::as_word)
        .collect::<Vec<_>>();
    let prefix_end =
        crate::word_primitives::parse_sequence_start(&words, &["then"]).unwrap_or(words.len());
    let prefix = &words[..prefix_end];
    if prefix.iter().filter(|word| **word == "target").count() < 2
        || !crate::word_primitives::sequence_occurs(prefix, &["and"])
    {
        return None;
    }
    match words.first().copied() {
        Some("destroy") => Some(CoordinatedTargetActionKind::Destroy),
        Some("exile") if prefix_end == words.len() => Some(CoordinatedTargetActionKind::Exile),
        Some("return") => Some(CoordinatedTargetActionKind::Return),
        _ => None,
    }
}

/// Returns a sequential discard tail following a coordinated action prefix.
/// This is kept as a token slice so the ordinary discard parser remains the
/// sole authority for counts, filters, randomness, and player binding.
pub fn trailing_then_discard_tokens(tokens: &[OwnedLexToken]) -> Option<&[OwnedLexToken]> {
    let then_idx = crate::slice_primitives::select_position(tokens, |token| token.is_word("then"))?;
    let trailing = trim_lexed_commas(tokens.get(then_idx + 1..)?);
    trailing
        .first()
        .is_some_and(|token| token.is_word("discard") || token.is_word("discards"))
        .then_some(trailing)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DelayedCopyTiming {
    EndStep { player_is_you: bool },
    Upkeep { player_is_you: bool },
    EndOfCombat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DelayedCopyFacts {
    pub has_exile: bool,
    pub has_sacrifice: bool,
    pub has_token: bool,
    pub timing: Option<DelayedCopyTiming>,
}

/// Recognizes one Oracle clause containing two or more explicit target
/// subjects, each with its own `gets` modifier, joined by `and`. The return
/// value records whether the shared duration was printed at the front.
pub fn coordinated_target_stat_modifier_leading_duration(tokens: &[OwnedLexToken]) -> Option<bool> {
    let words = tokens
        .iter()
        .filter_map(OwnedLexToken::as_word)
        .collect::<Vec<_>>();
    let target_count = words.iter().filter(|word| **word == "target").count();
    let gets_count = words
        .iter()
        .filter(|word| matches!(**word, "get" | "gets"))
        .count();
    if target_count < 2
        || gets_count < 2
        || !crate::word_primitives::sequence_occurs(&words, &["and"])
        || crate::word_primitives::sequence_occurs(&words, &["then"])
    {
        return None;
    }
    Some(matches!(words.first(), Some(&"until")))
}

/// Recognizes a single source-subject clause of the form
/// "this source deals ... and gains ...". Semantic AST validation remains in
/// the sentence layer; this helper records only the coordinated word order.
pub fn coordinated_source_damage_then_gain(tokens: &[OwnedLexToken]) -> bool {
    let words = tokens
        .iter()
        .filter_map(OwnedLexToken::as_word)
        .collect::<Vec<_>>();
    if crate::word_primitives::sequence_occurs(&words, &["then"]) {
        return false;
    }
    let Some(damage_idx) = crate::word_primitives::parse_sequence_start(&words, &["damage"]) else {
        return false;
    };
    let Some(gain_idx) = words
        .iter()
        .enumerate()
        .skip(damage_idx + 1)
        .find_map(|(idx, word)| matches!(*word, "gain" | "gains").then_some(idx))
    else {
        return false;
    };
    crate::word_primitives::sequence_occurs(&words[damage_idx + 1..gain_idx], &["and"])
}

/// Records the printed coordination in the common freeze clause
/// "tap ... and it doesn't untap during its controller's next untap step."
///
/// Semantic validation remains in the sentence layer: this helper only
/// distinguishes a single coordinated Oracle clause from the equally common
/// two-sentence surface "Tap ... . It doesn't untap ...". Both lower to the
/// same runtime actions, but the typed sequence surface lets the renderer
/// preserve the original relationship without guessing from adjacency.
pub fn coordinated_tap_then_next_untap(tokens: &[OwnedLexToken]) -> bool {
    let words = tokens
        .iter()
        .filter_map(OwnedLexToken::as_word)
        .collect::<Vec<_>>();
    if crate::word_primitives::sequence_occurs(&words, &["then"]) {
        return false;
    }
    let Some(tap_idx) = crate::word_primitives::parse_sequence_start(&words, &["tap"]) else {
        return false;
    };
    let Some(untap_idx) = words
        .iter()
        .enumerate()
        .skip(tap_idx + 1)
        .find_map(|(idx, word)| (*word == "untap").then_some(idx))
    else {
        return false;
    };
    let has_negative_untap = words[tap_idx + 1..untap_idx]
        .iter()
        .any(|word| matches!(*word, "doesnt" | "doesn't"));
    has_negative_untap
        && crate::word_primitives::sequence_occurs(&words[tap_idx + 1..untap_idx], &["and"])
        && crate::word_primitives::sequence_occurs(&words[untap_idx + 1..], &["its"])
        && words[untap_idx + 1..]
            .iter()
            .any(|word| matches!(*word, "controller" | "controller's" | "controllers"))
        && crate::word_primitives::sequence_occurs(&words[untap_idx + 1..], &["next"])
        && crate::word_primitives::sequence_occurs(&words[untap_idx + 1..], &["step"])
}

/// Records that the clause contains a real top-level effect conjunction.
///
/// The chain splitter already rejects non-effect uses of `and` (card-type
/// lists, quoted abilities, shared subjects, and similar shapes). Keeping
/// this small surface fact next to that grammar lets the sentence layer
/// preserve one printed Oracle clause without inferring coordination from
/// the lowered effects. A `then` chain is sequential even when another
/// conjunction also appears in the sentence.
pub fn coordinated_effect_chain_leading_duration(tokens: &[OwnedLexToken]) -> Option<bool> {
    // A gain/get compound has one grammatical subject and is owned by the
    // typed gain-ability parser. Treating its `and gets` tail as an
    // independent action loses that subject after a leading duration (for
    // example, "Until end of turn, creatures you control gain trample and
    // get ...").
    if super::gain_ability_shapes::parse_gain_then_get_shape(tokens).is_some() {
        return None;
    }

    // "and so on for" introduces the remainder of a keyword list, not a
    // second executable action. The keyword-bundle parser expands that list
    // into one conditional effect per ability; wrapping those effects as an
    // authored conjunction would misclassify the list surface as chain carry.
    if primitives::find_prefix(tokens, || primitives::phrase(&["and", "so", "on", "for"])).is_some()
    {
        return None;
    }

    let mut inside_quotes = false;
    for token in tokens {
        if token.kind == TokenKind::Quote {
            inside_quotes = !inside_quotes;
            continue;
        }
        if !inside_quotes && token.is_word("then") {
            return None;
        }
    }

    (super::chain_splitting::split_effect_chain_on_and_tokens(tokens, true).len() > 1)
        .then(|| parse_carry_duration_prefix_tokens(tokens).is_some())
}

pub fn parse_choose_each_basic_land_type_tokens(tokens: &[OwnedLexToken]) -> bool {
    primitives::parse_all(
        tokens,
        (
            opt(semantic_kw("you")),
            semantic_kw("choose"),
            opt(semantic_kw("a")),
            semantic_phrase(&["land", "of", "each", "basic", "land", "type"]),
            semantic_finish,
        )
            .void(),
        "choose land of each basic land type",
    )
    .is_ok()
}

pub fn parse_create_fragment_tokens(tokens: &[OwnedLexToken]) -> bool {
    let starts_like_count = primitives::parse_prefix(tokens, parse_create_fragment_count).is_some();
    starts_like_count && contains_semantic_word(tokens, "token", "tokens")
}

pub fn parse_exile_library_shuffle_tokens(
    tokens: &[OwnedLexToken],
) -> Option<ExileLibraryShuffleSpec> {
    primitives::parse_all(
        tokens,
        parse_exile_library_shuffle_lexed,
        "exile library then shuffle graveyard",
    )
    .ok()
}

pub fn count_token_mentions(tokens: &[OwnedLexToken]) -> usize {
    let mut input = LexStream::new(tokens);
    let mut count = 0usize;
    loop {
        let parsed: WResult<()> = semantic_kw("token").parse_next(&mut input);
        if parsed.is_ok() {
            count += 1;
            continue;
        }
        let parsed: WResult<()> = semantic_kw("tokens").parse_next(&mut input);
        if parsed.is_ok() {
            count += 1;
            continue;
        }
        let skipped: WResult<&OwnedLexToken> = any.parse_next(&mut input);
        if skipped.is_err() {
            break;
        }
    }
    count
}

pub fn parse_meld_them_into_tokens(tokens: &[OwnedLexToken]) -> Option<&[OwnedLexToken]> {
    primitives::parse_all(tokens, parse_meld_them_into_lexed, "meld-them chain").ok()
}

pub fn parse_leading_chain_scope_tokens(tokens: &[OwnedLexToken]) -> Option<ChainPlayerScope> {
    primitives::parse_prefix(
        tokens,
        alt((
            semantic_phrase(&["each", "opponent"]).value(ChainPlayerScope::EachOpponent),
            semantic_phrase(&["each", "opponents"]).value(ChainPlayerScope::EachOpponent),
            semantic_phrase(&["each", "player"]).value(ChainPlayerScope::EachPlayer),
            semantic_phrase(&["each", "players"]).value(ChainPlayerScope::EachPlayer),
        )),
    )
    .map(|(scope, _)| scope)
}

pub fn strip_leading_have_tokens(tokens: &[OwnedLexToken]) -> Option<&[OwnedLexToken]> {
    primitives::parse_prefix(tokens, alt((semantic_kw("have"), semantic_kw("has"))))
        .map(|(_, rest)| trim_lexed_commas(rest))
}

pub fn strip_leading_choose_to_tokens(tokens: &[OwnedLexToken]) -> Option<&[OwnedLexToken]> {
    primitives::parse_prefix(tokens, semantic_phrase(&["choose", "to"]))
        .map(|(_, rest)| trim_lexed_commas(rest))
}

pub fn starts_with_may_tokens(tokens: &[OwnedLexToken]) -> bool {
    primitives::parse_prefix(tokens, semantic_kw("may")).is_some()
}

pub fn strip_leading_and_tokens(tokens: &[OwnedLexToken]) -> Option<&[OwnedLexToken]> {
    primitives::parse_prefix(tokens, semantic_kw("and")).map(|(_, rest)| trim_lexed_commas(rest))
}

pub fn starts_with_unless_tokens(tokens: &[OwnedLexToken]) -> bool {
    primitives::parse_prefix(tokens, semantic_kw("unless")).is_some()
}

pub fn starts_with_destroy_tokens(tokens: &[OwnedLexToken]) -> bool {
    primitives::parse_prefix(tokens, semantic_kw("destroy")).is_some()
}

pub fn parse_tap_or_untap_all_choice_tokens(tokens: &[OwnedLexToken]) -> bool {
    let starts = primitives::parse_prefix(
        tokens,
        (
            semantic_kw("tap"),
            alt((semantic_kw("all"), semantic_kw("each"))),
        )
            .void(),
    )
    .is_some();
    starts
        && find_semantic_phrase(tokens, &["or", "untap", "all"])
            .or_else(|| find_semantic_phrase(tokens, &["or", "untap", "each"]))
            .is_some()
}

pub fn parse_temporary_attack_block_tail_tokens(tokens: &[OwnedLexToken]) -> bool {
    contains_semantic_word(tokens, "cant", "cannot")
        && (contains_semantic_word(tokens, "attack", "attacks")
            || contains_semantic_word(tokens, "block", "blocks"))
        && find_semantic_phrase(tokens, &["this", "turn"]).is_some()
}

pub fn parse_destroy_restriction_splits_tokens(
    tokens: &[OwnedLexToken],
) -> Vec<DestroyRestrictionSplit<'_>> {
    if !starts_with_destroy_tokens(tokens) {
        return Vec::new();
    }
    let mut input = LexStream::new(tokens);
    let mut splits = Vec::new();
    while !input.is_empty() {
        let idx = tokens.len().saturating_sub(input.len());
        let parsed: WResult<&OwnedLexToken> = any.parse_next(&mut input);
        let Ok(token) = parsed else {
            break;
        };
        if token.kind != TokenKind::Comma && !token.is_word("and") {
            continue;
        }
        if token.is_word("and")
            && tokens
                .get(..idx)
                .and_then(|prefix| prefix.last())
                .is_some_and(|previous| previous.kind == TokenKind::Comma)
        {
            continue;
        }
        let destroy_tokens = trim_lexed_commas(tokens.get(..idx).unwrap_or_default());
        let mut restriction_tokens = trim_lexed_commas(tokens.get(idx + 1..).unwrap_or_default());
        if let Some(rest) = strip_leading_and_tokens(restriction_tokens) {
            restriction_tokens = rest;
        }
        if !destroy_tokens.is_empty()
            && !restriction_tokens.is_empty()
            && parse_temporary_attack_block_tail_tokens(restriction_tokens)
        {
            splits.push(DestroyRestrictionSplit {
                destroy_tokens,
                restriction_tokens,
            });
        }
    }
    splits
}

pub fn parse_until_end_of_turn_trigger_tokens(tokens: &[OwnedLexToken]) -> bool {
    primitives::parse_prefix(
        tokens,
        (
            semantic_kw("until"),
            opt(semantic_kw("the")),
            semantic_phrase(&["end", "of", "turn"]),
            alt((
                semantic_kw("when"),
                semantic_kw("whenever"),
                semantic_kw("at"),
            )),
        )
            .void(),
    )
    .is_some()
}

pub fn parse_would_enter_replacement_tokens(tokens: &[OwnedLexToken]) -> bool {
    contains_semantic_word(tokens, "would", "would")
        && contains_semantic_word(tokens, "instead", "instead")
        && contains_semantic_word(tokens, "enter", "enters")
}

pub fn parse_or_action_splits_tokens(tokens: &[OwnedLexToken]) -> Vec<OrActionSplit<'_>> {
    if !contains_semantic_word(tokens, "or", "or") {
        return Vec::new();
    }
    let mut input = LexStream::new(tokens);
    let mut inside_quotes = false;
    let mut splits = Vec::new();
    while !input.is_empty() {
        let idx = tokens.len().saturating_sub(input.len());
        let parsed: WResult<&OwnedLexToken> = any.parse_next(&mut input);
        let token = match parsed {
            Ok(token) => token,
            Err(_) => break,
        };
        if token.kind == TokenKind::Quote {
            inside_quotes = !inside_quotes;
            continue;
        }
        if inside_quotes
            || comma_belongs_to_card_type_list(tokens, idx)
            || or_belongs_to_card_type_list(tokens, idx)
            || crate::util::starts_filter_keyword_list_continuation_words(
                &crate::lexer::parser_token_word_refs(&tokens[idx..]),
            )
        {
            continue;
        }
        let comparison_or = token.is_word("or") && comparison_or_delimiter(tokens, idx);
        let separator = token.kind == TokenKind::Comma || (token.is_word("or") && !comparison_or);
        if !separator {
            continue;
        }
        let first_tokens = normalize_action_option(tokens.get(..idx).unwrap_or_default());
        let second_tokens = normalize_action_option(tokens.get(idx + 1..).unwrap_or_default());
        if !first_tokens.is_empty() && !second_tokens.is_empty() {
            splits.push(OrActionSplit {
                first_tokens,
                second_tokens,
            });
        }
    }
    splits
}

pub fn parse_tap_then_unattach_tokens(tokens: &[OwnedLexToken]) -> bool {
    primitives::parse_prefix(tokens, semantic_phrase(&["tap", "those"])).is_some()
        && find_semantic_phrase(
            tokens,
            &["then", "unattach", "all", "equipment", "from", "them"],
        )
        .is_some()
}

pub fn split_return_then_loses_tokens(tokens: &[OwnedLexToken]) -> Option<&[OwnedLexToken]> {
    primitives::parse_prefix(tokens, semantic_phrase(&["return", "it"]))?;
    let (idx, (), _) = primitives::find_prefix(tokens, || {
        semantic_phrase(&["and", "it", "loses", "all", "abilities"])
    })?;
    let return_tokens = trim_lexed_commas(tokens.get(..idx)?);
    find_semantic_phrase(return_tokens, &["battlefield"])?;
    (!return_tokens.is_empty()).then_some(return_tokens)
}

pub fn is_rounded_up_segment_tokens(tokens: &[OwnedLexToken]) -> bool {
    primitives::parse_all(
        tokens,
        (
            alt((
                semantic_phrase(&["rounded", "up"]),
                semantic_phrase(&["round", "up", "each", "time"]),
            )),
            semantic_finish,
        )
            .void(),
        "rounded-up carry segment",
    )
    .is_ok()
}

pub fn has_where_x_is_half_tokens(tokens: &[OwnedLexToken]) -> bool {
    find_semantic_phrase(tokens, &["where", "x", "is", "half"]).is_some()
}

pub fn split_all_abilities_and_gain_tokens(tokens: &[OwnedLexToken]) -> Option<&[OwnedLexToken]> {
    primitives::parse_prefix(
        tokens,
        (
            semantic_phrase(&["all", "abilities", "and"]),
            peek(alt((semantic_kw("gain"), semantic_kw("gains")))),
        )
            .void(),
    )
    .map(|(_, rest)| trim_lexed_commas(rest))
}

pub fn parse_delayed_copy_facts_tokens(tokens: &[OwnedLexToken]) -> DelayedCopyFacts {
    let has_exile = contains_semantic_word(tokens, "exile", "exiles");
    let has_sacrifice = contains_semantic_word(tokens, "sacrifice", "sacrifices");
    let has_token = contains_semantic_word(tokens, "token", "tokens");
    let player_is_you =
        find_semantic_phrase(tokens, &["beginning", "of", "your", "next", "end", "step"]).is_some();
    let timing = if contains_beginning_end_step_tokens(tokens)
        || find_semantic_phrase(tokens, &["next", "end", "step", "repeat"]).is_some()
    {
        Some(DelayedCopyTiming::EndStep { player_is_you })
    } else if contains_beginning_upkeep_tokens(tokens) {
        Some(DelayedCopyTiming::Upkeep {
            player_is_you: find_semantic_phrase(
                tokens,
                &["beginning", "of", "your", "next", "upkeep"],
            )
            .is_some(),
        })
    } else if find_semantic_phrase(tokens, &["end", "of", "combat"]).is_some() {
        Some(DelayedCopyTiming::EndOfCombat)
    } else {
        None
    };
    DelayedCopyFacts {
        has_exile,
        has_sacrifice,
        has_token,
        timing,
    }
}

pub fn has_token_rules_tail_tokens(tokens: &[OwnedLexToken]) -> bool {
    find_semantic_phrase(tokens, &["when", "this", "token"]).is_some()
        || find_semantic_phrase(tokens, &["whenever", "this", "token"]).is_some()
        || find_semantic_phrase(tokens, &["this", "token"]).is_some()
        || find_semantic_phrase(tokens, &["that", "token"]).is_some()
        || find_semantic_phrase(tokens, &["those", "tokens"]).is_some()
        || find_semantic_phrase(tokens, &["it", "has"]).is_some()
        || find_semantic_phrase(tokens, &["they", "have"]).is_some()
}

pub fn is_causative_have_player_tokens(tokens: &[OwnedLexToken]) -> bool {
    primitives::parse_prefix(
        tokens,
        (
            alt((semantic_kw("have"), semantic_kw("has"))),
            alt((
                semantic_kw("that"),
                semantic_kw("each"),
                semantic_kw("those"),
                semantic_kw("target"),
                semantic_kw("another"),
            )),
            alt((
                semantic_kw("player"),
                semantic_kw("players"),
                semantic_kw("opponent"),
                semantic_kw("opponents"),
            )),
        )
            .void(),
    )
    .is_some()
}

fn parse_create_fragment_count<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        leaf::parse_leaf_number_prefix_lexed.void(),
        any.verify(|token: &&OwnedLexToken| {
            leaf::parse_leaf_power_toughness_complete(token.parser_text()).is_ok()
        })
        .void(),
        semantic_kw("x"),
        semantic_kw("a"),
        semantic_kw("an"),
        semantic_kw("the"),
    ))
    .parse_next(input)
}

fn parse_exile_library_shuffle_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<ExileLibraryShuffleSpec> {
    semantic_phrase(&["exile", "all", "cards", "from"]).parse_next(input)?;
    let owner = parse_chain_owner(input)?;
    semantic_phrase(&[
        "library", "face", "down", "then", "shuffle", "all", "cards", "from",
    ])
    .parse_next(input)?;
    let graveyard_owner = parse_chain_owner(input)?;
    alt((semantic_kw("graveyard"), semantic_kw("graveyards"))).parse_next(input)?;
    semantic_kw("into").parse_next(input)?;
    let destination_owner = parse_chain_owner(input)?;
    semantic_kw("library").parse_next(input)?;
    semantic_finish(input)?;
    if owner != graveyard_owner || owner != destination_owner {
        return Err(primitives::backtrack_err(
            "exile-library shuffle",
            "matching owner references",
        ));
    }
    Ok(ExileLibraryShuffleSpec { owner })
}

fn parse_chain_owner<'a>(input: &mut LexStream<'a>) -> WResult<ChainOwner> {
    alt((
        semantic_kw("your").value(ChainOwner::You),
        (
            semantic_kw("target"),
            alt((semantic_kw("player"), semantic_kw("players"))),
        )
            .value(ChainOwner::TargetPlayer),
        (
            semantic_kw("target"),
            alt((semantic_kw("opponent"), semantic_kw("opponents"))),
        )
            .value(ChainOwner::TargetOpponent),
    ))
    .parse_next(input)
}

fn parse_meld_them_into_lexed<'a>(input: &mut LexStream<'a>) -> WResult<&'a [OwnedLexToken]> {
    semantic_phrase(&["exile", "them"]).parse_next(input)?;
    repeat_till::<_, _, (), _, _, _, _>(
        0..,
        any.void(),
        peek(semantic_phrase(&["then", "meld", "them", "into"])),
    )
    .void()
    .parse_next(input)?;
    semantic_phrase(&["then", "meld", "them", "into"]).parse_next(input)?;
    let result = repeat_till::<_, _, (), _, _, _, _>(1.., any.void(), peek(semantic_finish))
        .map(|((), _)| ())
        .take()
        .parse_next(input)?;
    semantic_finish(input)?;
    Ok(trim_lexed_commas(result))
}

fn comma_belongs_to_card_type_list(tokens: &[OwnedLexToken], idx: usize) -> bool {
    let Some(token) = tokens.get(idx) else {
        return false;
    };
    if token.kind != TokenKind::Comma {
        return false;
    }
    let before = tokens.get(..idx).unwrap_or_default();
    let after = trim_lexed_commas(tokens.get(idx + 1..).unwrap_or_default());
    primitives::contains_word(before, "target")
        && primitives::parse_prefix(after, (opt(semantic_kw("or")), parse_card_type_word).void())
            .is_some()
}

fn or_belongs_to_card_type_list(tokens: &[OwnedLexToken], idx: usize) -> bool {
    if !tokens.get(idx).is_some_and(|token| token.is_word("or")) {
        return false;
    }
    let after = tokens.get(idx + 1..).unwrap_or_default();
    if primitives::parse_prefix(after, parse_card_type_word).is_none() {
        return false;
    }
    tokens.get(idx.saturating_sub(1)).is_some_and(|previous| {
        previous.kind == TokenKind::Comma || token_is_card_type_word(previous)
    })
}

fn token_is_card_type_word(token: &OwnedLexToken) -> bool {
    primitives::parse_all(
        std::slice::from_ref(token),
        (parse_card_type_word, semantic_finish).void(),
        "card type token",
    )
    .is_ok()
}

fn parse_card_type_word<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        semantic_kw("artifact"),
        semantic_kw("battle"),
        semantic_kw("creature"),
        semantic_kw("enchantment"),
        semantic_kw("instant"),
        semantic_kw("land"),
        semantic_kw("planeswalker"),
        semantic_kw("sorcery"),
    ))
    .parse_next(input)
}

fn comparison_or_delimiter(tokens: &[OwnedLexToken], idx: usize) -> bool {
    let after = tokens.get(idx + 1..).unwrap_or_default();
    if primitives::parse_prefix(
        after,
        alt((
            semantic_kw("less"),
            semantic_kw("greater"),
            semantic_kw("more"),
            semantic_kw("fewer"),
        )),
    )
    .is_some()
    {
        return true;
    }
    tokens.get(..idx).and_then(last_semantic_word) == Some("than")
        && primitives::parse_prefix(after, semantic_kw("equal")).is_some()
}

#[cfg(test)]
#[path = "chain_carry_inline_tests.rs"]
mod tests;

#[path = "chain_carry/core.rs"]
mod core_programs;
use core_programs::{
    contains_semantic_word, find_semantic_phrase, last_semantic_word, normalize_action_option,
    semantic_finish, semantic_kw, semantic_noise, semantic_phrase,
};
#[path = "chain_carry/object_action.rs"]
mod object_action_programs;
use object_action_programs::{
    contains_beginning_end_step_tokens, contains_beginning_upkeep_tokens,
};
