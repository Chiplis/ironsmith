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
pub(crate) use carry_facts::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ChainOwner {
    You,
    TargetPlayer,
    TargetOpponent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ExileLibraryShuffleSpec {
    pub(crate) owner: ChainOwner,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ChainPlayerScope {
    EachOpponent,
    EachPlayer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct OrActionSplit<'a> {
    pub(crate) first_tokens: &'a [OwnedLexToken],
    pub(crate) second_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DestroyRestrictionSplit<'a> {
    pub(crate) destroy_tokens: &'a [OwnedLexToken],
    pub(crate) restriction_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DelayedCopyTiming {
    EndStep { player_is_you: bool },
    Upkeep { player_is_you: bool },
    EndOfCombat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DelayedCopyFacts {
    pub(crate) has_exile: bool,
    pub(crate) has_sacrifice: bool,
    pub(crate) has_token: bool,
    pub(crate) timing: Option<DelayedCopyTiming>,
}

pub(crate) fn parse_choose_each_basic_land_type_tokens(tokens: &[OwnedLexToken]) -> bool {
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

pub(crate) fn parse_create_fragment_tokens(tokens: &[OwnedLexToken]) -> bool {
    let starts_like_count = primitives::parse_prefix(tokens, parse_create_fragment_count).is_some();
    starts_like_count && contains_semantic_word(tokens, "token", "tokens")
}

pub(crate) fn parse_exile_library_shuffle_tokens(
    tokens: &[OwnedLexToken],
) -> Option<ExileLibraryShuffleSpec> {
    primitives::parse_all(
        tokens,
        parse_exile_library_shuffle_lexed,
        "exile library then shuffle graveyard",
    )
    .ok()
}

pub(crate) fn count_token_mentions(tokens: &[OwnedLexToken]) -> usize {
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

pub(crate) fn parse_meld_them_into_tokens(tokens: &[OwnedLexToken]) -> Option<&[OwnedLexToken]> {
    primitives::parse_all(tokens, parse_meld_them_into_lexed, "meld-them chain").ok()
}

pub(crate) fn parse_leading_chain_scope_tokens(
    tokens: &[OwnedLexToken],
) -> Option<ChainPlayerScope> {
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

pub(crate) fn strip_leading_have_tokens(tokens: &[OwnedLexToken]) -> Option<&[OwnedLexToken]> {
    primitives::parse_prefix(tokens, alt((semantic_kw("have"), semantic_kw("has"))))
        .map(|(_, rest)| trim_lexed_commas(rest))
}

pub(crate) fn strip_leading_choose_to_tokens(tokens: &[OwnedLexToken]) -> Option<&[OwnedLexToken]> {
    primitives::parse_prefix(tokens, semantic_phrase(&["choose", "to"]))
        .map(|(_, rest)| trim_lexed_commas(rest))
}

pub(crate) fn starts_with_may_tokens(tokens: &[OwnedLexToken]) -> bool {
    primitives::parse_prefix(tokens, semantic_kw("may")).is_some()
}

pub(crate) fn strip_leading_and_tokens(tokens: &[OwnedLexToken]) -> Option<&[OwnedLexToken]> {
    primitives::parse_prefix(tokens, semantic_kw("and")).map(|(_, rest)| trim_lexed_commas(rest))
}

pub(crate) fn starts_with_unless_tokens(tokens: &[OwnedLexToken]) -> bool {
    primitives::parse_prefix(tokens, semantic_kw("unless")).is_some()
}

pub(crate) fn starts_with_destroy_tokens(tokens: &[OwnedLexToken]) -> bool {
    primitives::parse_prefix(tokens, semantic_kw("destroy")).is_some()
}

pub(crate) fn parse_tap_or_untap_all_choice_tokens(tokens: &[OwnedLexToken]) -> bool {
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

pub(crate) fn parse_temporary_attack_block_tail_tokens(tokens: &[OwnedLexToken]) -> bool {
    contains_semantic_word(tokens, "cant", "cannot")
        && (contains_semantic_word(tokens, "attack", "attacks")
            || contains_semantic_word(tokens, "block", "blocks"))
        && find_semantic_phrase(tokens, &["this", "turn"]).is_some()
}

pub(crate) fn parse_destroy_restriction_splits_tokens(
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

pub(crate) fn parse_until_end_of_turn_trigger_tokens(tokens: &[OwnedLexToken]) -> bool {
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

pub(crate) fn parse_would_enter_replacement_tokens(tokens: &[OwnedLexToken]) -> bool {
    contains_semantic_word(tokens, "would", "would")
        && contains_semantic_word(tokens, "instead", "instead")
        && contains_semantic_word(tokens, "enter", "enters")
}

pub(crate) fn parse_or_action_splits_tokens(tokens: &[OwnedLexToken]) -> Vec<OrActionSplit<'_>> {
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
        {
            continue;
        }
        let separator = token.kind == TokenKind::Comma
            || (token.is_word("or") && !comparison_or_delimiter(tokens, idx));
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

pub(crate) fn parse_tap_then_unattach_tokens(tokens: &[OwnedLexToken]) -> bool {
    primitives::parse_prefix(tokens, semantic_phrase(&["tap", "those"])).is_some()
        && find_semantic_phrase(
            tokens,
            &["then", "unattach", "all", "equipment", "from", "them"],
        )
        .is_some()
}

pub(crate) fn split_return_then_loses_tokens(tokens: &[OwnedLexToken]) -> Option<&[OwnedLexToken]> {
    if primitives::parse_prefix(tokens, semantic_phrase(&["return", "it"])).is_none() {
        return None;
    }
    let (idx, (), _) = primitives::find_prefix(tokens, || {
        semantic_phrase(&["and", "it", "loses", "all", "abilities"])
    })?;
    let return_tokens = trim_lexed_commas(tokens.get(..idx)?);
    find_semantic_phrase(return_tokens, &["battlefield"])?;
    (!return_tokens.is_empty()).then_some(return_tokens)
}

pub(crate) fn is_rounded_up_segment_tokens(tokens: &[OwnedLexToken]) -> bool {
    primitives::parse_all(
        tokens,
        (semantic_phrase(&["rounded", "up"]), semantic_finish).void(),
        "rounded-up carry segment",
    )
    .is_ok()
}

pub(crate) fn has_where_x_is_half_tokens(tokens: &[OwnedLexToken]) -> bool {
    find_semantic_phrase(tokens, &["where", "x", "is", "half"]).is_some()
}

pub(crate) fn split_all_abilities_and_gain_tokens(
    tokens: &[OwnedLexToken],
) -> Option<&[OwnedLexToken]> {
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

pub(crate) fn parse_delayed_copy_facts_tokens(tokens: &[OwnedLexToken]) -> DelayedCopyFacts {
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

pub(crate) fn has_token_rules_tail_tokens(tokens: &[OwnedLexToken]) -> bool {
    find_semantic_phrase(tokens, &["when", "this", "token"]).is_some()
        || find_semantic_phrase(tokens, &["whenever", "this", "token"]).is_some()
        || find_semantic_phrase(tokens, &["this", "token"]).is_some()
        || find_semantic_phrase(tokens, &["that", "token"]).is_some()
        || find_semantic_phrase(tokens, &["those", "tokens"]).is_some()
        || find_semantic_phrase(tokens, &["it", "has"]).is_some()
        || find_semantic_phrase(tokens, &["they", "have"]).is_some()
}

pub(crate) fn is_causative_have_player_tokens(tokens: &[OwnedLexToken]) -> bool {
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

fn last_semantic_word(tokens: &[OwnedLexToken]) -> Option<&str> {
    let mut input = LexStream::new(tokens);
    let mut last = None;
    loop {
        let token: WResult<&OwnedLexToken> = any.parse_next(&mut input);
        let Ok(token) = token else {
            break;
        };
        if let Some(piece) = token.parser_word_pieces().last() {
            last = Some(piece.text.as_str());
        }
    }
    last
}

fn normalize_action_option(mut tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    loop {
        let Some(((), rest)) =
            primitives::parse_prefix(tokens, alt((semantic_kw("and"), semantic_kw("or"))).void())
        else {
            break;
        };
        tokens = rest;
    }
    trim_lexed_commas(tokens)
}

fn contains_beginning_end_step_tokens(tokens: &[OwnedLexToken]) -> bool {
    find_semantic_phrase(tokens, &["beginning", "of", "your", "next", "end", "step"])
        .or_else(|| find_semantic_phrase(tokens, &["beginning", "of", "the", "end", "step"]))
        .or_else(|| find_semantic_phrase(tokens, &["beginning", "of", "next", "end", "step"]))
        .or_else(|| {
            find_semantic_phrase(tokens, &["beginning", "of", "the", "next", "end", "step"])
        })
        .is_some()
}

fn contains_beginning_upkeep_tokens(tokens: &[OwnedLexToken]) -> bool {
    find_semantic_phrase(tokens, &["beginning", "of", "your", "next", "upkeep"])
        .or_else(|| find_semantic_phrase(tokens, &["beginning", "of", "next", "upkeep"]))
        .or_else(|| find_semantic_phrase(tokens, &["beginning", "of", "the", "next", "upkeep"]))
        .is_some()
}

fn contains_semantic_word(
    tokens: &[OwnedLexToken],
    singular: &'static str,
    plural: &'static str,
) -> bool {
    primitives::find_prefix(tokens, || alt((semantic_kw(singular), semantic_kw(plural)))).is_some()
}

fn find_semantic_phrase(
    tokens: &[OwnedLexToken],
    phrase: &'static [&'static str],
) -> Option<usize> {
    primitives::find_prefix(tokens, || semantic_phrase(phrase)).map(|(idx, (), _)| idx)
}

fn semantic_kw<'a>(
    expected: &'static str,
) -> impl Parser<LexStream<'a>, (), ErrMode<ContextError>> {
    (
        repeat::<_, _, (), _, _>(
            0..,
            any.verify(move |token: &&OwnedLexToken| {
                token.parser_word_pieces().is_empty()
                    || ((token.is_word("a") || token.is_word("an") || token.is_word("the"))
                        && !token.is_word(expected))
            })
            .void(),
        ),
        any.verify(move |token: &&OwnedLexToken| {
            token.is_word(expected)
                || matches!(token.parser_word_pieces(), [piece] if piece.text == expected)
        }),
    )
        .void()
}

fn semantic_phrase<'a>(
    expected: &'static [&'static str],
) -> impl Parser<LexStream<'a>, (), ErrMode<ContextError>> {
    move |input: &mut LexStream<'a>| {
        for word in expected {
            semantic_kw(word).parse_next(input)?;
        }
        Ok(())
    }
}

fn semantic_noise<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    any.verify(|token: &&OwnedLexToken| {
        token.parser_word_pieces().is_empty()
            || token.is_word("a")
            || token.is_word("an")
            || token.is_word("the")
    })
    .void()
    .parse_next(input)
}

fn semantic_finish<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    repeat::<_, _, (), _, _>(0.., semantic_noise).parse_next(input)?;
    eof.void().parse_next(input)
}

#[cfg(test)]
mod tests {
    use super::super::super::super::lexer::lex_line;
    use super::*;

    #[test]
    fn parses_chain_carry_leaf_shapes() {
        let tokens = lex_line("Choose a land of each basic land type.", 0).unwrap();
        assert!(parse_choose_each_basic_land_type_tokens(&tokens));
        let tokens = lex_line("Two 1/1 white Soldier creature tokens", 0).unwrap();
        assert!(parse_create_fragment_tokens(&tokens));
        let tokens = lex_line("Tap those, then unattach all Equipment from them.", 0).unwrap();
        assert!(parse_tap_then_unattach_tokens(&tokens));

        let tokens = lex_line("Then sacrifice the rest.", 0).unwrap();
        assert_eq!(
            parse_rest_action_tokens(&tokens),
            Some(RestActionShape::Sacrifice)
        );

        let tokens = lex_line("Until your next untap step, it gains flying.", 0).unwrap();
        let duration = parse_carry_duration_prefix_tokens(&tokens).unwrap();
        assert_eq!(duration.duration, Until::ControllersNextUntapStep);
        assert!(
            duration
                .rest
                .first()
                .is_some_and(|token| token.is_word("it"))
        );

        let tokens = lex_line("And draw a card.", 0).unwrap();
        assert_eq!(
            parse_carry_clause_head_tokens(&tokens),
            CarryClauseHead::Draw
        );
    }

    #[test]
    fn parses_owner_and_delay_facts() {
        let tokens = lex_line(
            "Exile all cards from your library face down, then shuffle all cards from your graveyard into your library.",
            0,
        )
        .unwrap();
        assert_eq!(
            parse_exile_library_shuffle_tokens(&tokens).map(|spec| spec.owner),
            Some(ChainOwner::You)
        );
        let tokens = lex_line(
            "At the beginning of your next end step, exile the token.",
            0,
        )
        .unwrap();
        let facts = parse_delayed_copy_facts_tokens(&tokens);
        assert!(facts.has_exile && facts.has_token);
        assert_eq!(
            facts.timing,
            Some(DelayedCopyTiming::EndStep {
                player_is_you: true
            })
        );

        let tokens = lex_line(
            "At the beginning of your next upkeep, sacrifice the token.",
            0,
        )
        .unwrap();
        assert_eq!(
            parse_delayed_copy_facts_tokens(&tokens).timing,
            Some(DelayedCopyTiming::Upkeep {
                player_is_you: true
            })
        );
    }

    #[test]
    fn action_splits_preserve_card_type_lists() {
        let tokens = lex_line(
            "Discard two cards or sacrifice a creature or planeswalker of your choice.",
            0,
        )
        .unwrap();
        let splits = parse_or_action_splits_tokens(&tokens);
        assert_eq!(splits.len(), 1);

        let tokens = lex_line("Destroy target artifact, creature, or enchantment.", 0).unwrap();
        assert!(parse_or_action_splits_tokens(&tokens).is_empty());
    }

    #[test]
    fn destroy_split_requires_a_temporary_restriction_tail() {
        let tokens = lex_line(
            "Destroy target creature, and that creature can't attack or block this turn.",
            0,
        )
        .unwrap();
        assert_eq!(parse_destroy_restriction_splits_tokens(&tokens).len(), 1);

        let tokens = lex_line("Destroy target creature and draw a card.", 0).unwrap();
        assert!(parse_destroy_restriction_splits_tokens(&tokens).is_empty());
    }
}
