//! Typed facts for keyword lines historically recognized in shared `util`.

use winnow::combinator::{alt, eof, repeat};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;

use crate::lexer::{LexStream, OwnedLexToken, TokenKind, trim_lexed_commas};
use crate::mana::ManaCost;

use super::super::{leaf, primitives};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NamedCostKeyword {
    Replicate,
    Escalate,
    Evoke,
    Prowl,
    Eternalize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SharedKeywordHead {
    LevelUp,
    Madness,
    Bargain,
    Replicate,
    Escalate,
    Evoke,
    Prowl,
    Eternalize,
    Epic,
    Retrace,
    Harmonize,
    Warp,
    Reinforce,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LevelUpLineFact {
    pub mana_cost: Option<ManaCost>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MadnessCostFact<'a> {
    RepeatedMana(ManaCost),
    ActivationTokens(&'a [OwnedLexToken]),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MadnessLineFact<'a> {
    pub cost: MadnessCostFact<'a>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BargainLineFact;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NamedCostLineFact<'a> {
    pub cost_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EpicLineFact;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetraceLineFact;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HarmonizeLineFact<'a> {
    pub cost_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WarpLineFact<'a> {
    pub cost_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReinforceLineFact<'a> {
    pub amount: Option<leaf::LeafNumber>,
    pub cost_tokens: &'a [OwnedLexToken],
}

pub fn parse_level_up_line_tokens(tokens: &[OwnedLexToken]) -> Option<LevelUpLineFact> {
    let rest = parse_expected_head(tokens, SharedKeywordHead::LevelUp)?;
    let mana_cost = primitives::parse_prefix(rest, leaf::parse_leaf_mana_cost_prefix_lexed)
        .map(|(prefix, _)| prefix.cost);
    Some(LevelUpLineFact { mana_cost })
}

pub fn parse_madness_line_tokens(tokens: &[OwnedLexToken]) -> Option<MadnessLineFact<'_>> {
    let rest = parse_expected_head(tokens, SharedKeywordHead::Madness)?;
    let comma = primitives::find_prefix(rest, primitives::comma).map(|(idx, _, _)| idx);
    let cost_tokens = strip_leading_cost_separators(&rest[..comma.unwrap_or(rest.len())]);
    let cost = crate::grammar::primitives::probe_all(
        cost_tokens,
        parse_repeated_mana_payment_lexed,
        "repeated-mana-payment",
    )
    .map(MadnessCostFact::RepeatedMana)
    .unwrap_or(MadnessCostFact::ActivationTokens(cost_tokens));
    Some(MadnessLineFact { cost })
}

pub fn parse_bargain_line_tokens(tokens: &[OwnedLexToken]) -> Option<BargainLineFact> {
    parse_expected_head(tokens, SharedKeywordHead::Bargain)?;
    Some(BargainLineFact)
}

pub fn parse_named_cost_line_tokens(
    tokens: &[OwnedLexToken],
    expected: NamedCostKeyword,
) -> Option<NamedCostLineFact<'_>> {
    let head = named_cost_head(expected);
    let rest = parse_expected_head(tokens, head)?;
    let rest = strip_one_cost_separator(rest);
    let boundary = primitives::find_prefix(rest, || {
        alt((
            primitives::token_kind(TokenKind::LParen),
            primitives::period(),
        ))
    })
    .map(|(idx, _, _)| idx)
    .unwrap_or(rest.len());
    Some(NamedCostLineFact {
        cost_tokens: trim_lexed_commas(&rest[..boundary]),
    })
}

pub fn parse_epic_line_tokens(tokens: &[OwnedLexToken]) -> Option<EpicLineFact> {
    parse_expected_head(tokens, SharedKeywordHead::Epic)?;
    Some(EpicLineFact)
}

pub fn parse_retrace_line_tokens(tokens: &[OwnedLexToken]) -> Option<RetraceLineFact> {
    parse_expected_head(tokens, SharedKeywordHead::Retrace)?;
    Some(RetraceLineFact)
}

pub fn parse_harmonize_line_tokens(tokens: &[OwnedLexToken]) -> Option<HarmonizeLineFact<'_>> {
    let cost_tokens = parse_expected_head(tokens, SharedKeywordHead::Harmonize)?;
    Some(HarmonizeLineFact { cost_tokens })
}

pub fn parse_warp_line_tokens(tokens: &[OwnedLexToken]) -> Option<WarpLineFact<'_>> {
    let cost_tokens = parse_expected_head(tokens, SharedKeywordHead::Warp)?;
    Some(WarpLineFact { cost_tokens })
}

pub fn parse_reinforce_line_tokens(tokens: &[OwnedLexToken]) -> Option<ReinforceLineFact<'_>> {
    let tail = parse_expected_head(tokens, SharedKeywordHead::Reinforce)?;
    if primitives::find_prefix(tokens, || {
        alt((primitives::kw("has"), primitives::kw("have")))
    })
    .is_some()
    {
        return None;
    }
    let parsed = primitives::parse_prefix(tail, leaf::parse_leaf_number_or_x_prefix_lexed);
    let (amount, cost_tokens) = match parsed {
        Some((amount, rest)) => (Some(amount), rest),
        None => (None, tail),
    };
    Some(ReinforceLineFact {
        amount,
        cost_tokens,
    })
}

fn parse_expected_head(
    tokens: &[OwnedLexToken],
    expected: SharedKeywordHead,
) -> Option<&[OwnedLexToken]> {
    let (actual, rest) = primitives::parse_prefix(tokens, parse_shared_keyword_head_lexed)?;
    (actual == expected).then_some(rest)
}

fn parse_shared_keyword_head_lexed<'a>(input: &mut LexStream<'a>) -> WResult<SharedKeywordHead> {
    alt((
        alt((
            primitives::phrase(&["level", "up"]).value(SharedKeywordHead::LevelUp),
            primitives::kw("madness").value(SharedKeywordHead::Madness),
            primitives::kw("bargain").value(SharedKeywordHead::Bargain),
            primitives::kw("replicate").value(SharedKeywordHead::Replicate),
            primitives::kw("escalate").value(SharedKeywordHead::Escalate),
            primitives::kw("evoke").value(SharedKeywordHead::Evoke),
            primitives::kw("prowl").value(SharedKeywordHead::Prowl),
        )),
        alt((
            primitives::kw("eternalize").value(SharedKeywordHead::Eternalize),
            primitives::kw("epic").value(SharedKeywordHead::Epic),
            primitives::kw("retrace").value(SharedKeywordHead::Retrace),
            primitives::kw("harmonize").value(SharedKeywordHead::Harmonize),
            primitives::kw("warp").value(SharedKeywordHead::Warp),
            primitives::kw("reinforce").value(SharedKeywordHead::Reinforce),
        )),
    ))
    .parse_next(input)
}

fn parse_repeated_mana_payment_lexed<'a>(input: &mut LexStream<'a>) -> WResult<ManaCost> {
    primitives::kw("pay").parse_next(input)?;
    let count = leaf::parse_leaf_number_prefix_lexed.parse_next(input)?;
    let pip = leaf::parse_leaf_surface_mana_pip_lexed
        .map(leaf::LeafManaPipToken::into_pip)
        .verify(|pip: &Vec<crate::mana::ManaSymbol>| pip.len() == 1)
        .parse_next(input)?;
    repeat::<_, _, (), _, _>(0.., primitives::period().void()).parse_next(input)?;
    eof.parse_next(input)?;
    Ok(ManaCost::from_pips(
        (0..count).map(|_| pip.clone()).collect(),
    ))
}

fn named_cost_head(keyword: NamedCostKeyword) -> SharedKeywordHead {
    match keyword {
        NamedCostKeyword::Replicate => SharedKeywordHead::Replicate,
        NamedCostKeyword::Escalate => SharedKeywordHead::Escalate,
        NamedCostKeyword::Evoke => SharedKeywordHead::Evoke,
        NamedCostKeyword::Prowl => SharedKeywordHead::Prowl,
        NamedCostKeyword::Eternalize => SharedKeywordHead::Eternalize,
    }
}

fn strip_leading_cost_separators(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    primitives::parse_prefix(
        tokens,
        repeat::<_, _, (), _, _>(
            0..,
            alt((
                primitives::token_kind(TokenKind::Dash),
                primitives::token_kind(TokenKind::EmDash),
            ))
            .void(),
        ),
    )
    .map(|(_, rest)| rest)
    .unwrap_or(tokens)
}

fn strip_one_cost_separator(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    primitives::parse_prefix(
        tokens,
        alt((
            primitives::token_kind(TokenKind::Dash),
            primitives::token_kind(TokenKind::EmDash),
        )),
    )
    .map(|(_, rest)| rest)
    .unwrap_or(tokens)
}

#[cfg(test)]
mod tests {
    use crate::lexer::{lex_line, parser_token_word_refs};

    use super::*;

    #[test]
    fn parses_level_up_and_madness_facts() {
        let level = lex_line("Level up {2}{U}", 0).unwrap();
        assert!(
            parse_level_up_line_tokens(&level)
                .unwrap()
                .mana_cost
                .is_some()
        );

        let madness = lex_line("Madness—Pay three {B}.", 0).unwrap();
        assert!(matches!(
            parse_madness_line_tokens(&madness).unwrap().cost,
            MadnessCostFact::RepeatedMana(_)
        ));
    }

    #[test]
    fn parses_named_cost_tail_before_reminder_text() {
        let tokens = lex_line("Replicate—{1}{U}. (Reminder text.)", 0).unwrap();
        let fact = parse_named_cost_line_tokens(&tokens, NamedCostKeyword::Replicate).unwrap();
        assert_eq!(parser_token_word_refs(fact.cost_tokens), ["1", "u"]);
    }

    #[test]
    fn reinforce_fact_rejects_static_grant_sentences() {
        let line = lex_line("Reinforce 2 {1}{G}", 0).unwrap();
        let fact = parse_reinforce_line_tokens(&line).unwrap();
        assert_eq!(fact.amount, Some(leaf::LeafNumber::Fixed(2)));

        let grant = lex_line("Creature cards in your hand have reinforce 2 {1}{G}.", 0).unwrap();
        assert!(parse_reinforce_line_tokens(&grant).is_none());
    }
}
