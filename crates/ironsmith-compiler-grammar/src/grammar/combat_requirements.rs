//! Complete combat-obligation clauses. Requirements never imply permission to
//! ignore attack/block restrictions, and a coordination retains both duties.
use crate::grammar::primitives;
use crate::lexer::{LexStream, OwnedLexToken, trim_lexed_commas};
use winnow::Parser;
use winnow::combinator::{alt, opt, peek, repeat_till};
use winnow::error::ModalResult;
use winnow::token::any;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CombatRequirement {
    Attack,
    Block,
    AttackOrBlock,
}

#[derive(Debug, Clone, Copy)]
pub struct CombatRequirementClause<'a> {
    pub subject_tokens: &'a [OwnedLexToken],
    pub requirement: CombatRequirement,
}

pub fn parse_combat_requirement(tokens: &[OwnedLexToken]) -> Option<CombatRequirementClause<'_>> {
    primitives::probe_all(tokens, parse_lexed, "combat requirement")
}

fn parse_lexed<'a>(input: &mut LexStream<'a>) -> ModalResult<CombatRequirementClause<'a>> {
    let subject_tokens = repeat_till(
        0..,
        any.void(),
        peek(alt((
            primitives::kw("attack"), primitives::kw("attacks"),
            primitives::kw("block"), primitives::kw("blocks"),
        ))),
    ).map(|((), _)| ()).take().parse_next(input)?;
    let requirement = alt((
        alt((
            primitives::phrase(&["attack", "or", "block"]),
            primitives::phrase(&["attacks", "or", "blocks"]),
        )).value(CombatRequirement::AttackOrBlock),
        alt((primitives::kw("attack"), primitives::kw("attacks")))
            .value(CombatRequirement::Attack),
        alt((primitives::kw("block"), primitives::kw("blocks")))
            .value(CombatRequirement::Block),
    )).parse_next(input)?;
    primitives::phrase(&["each", "combat", "if", "able"]).parse_next(input)?;
    opt(primitives::sentence_end()).parse_next(input)?;
    Ok(CombatRequirementClause { subject_tokens: trim_lexed_commas(subject_tokens), requirement })
}

#[derive(Debug, Clone, Copy)]
pub struct CombatRequirementLine<'a> {
    pub clause: CombatRequirementClause<'a>,
    pub preceding_tokens: Option<&'a [OwnedLexToken]>,
}

pub fn parse_combat_requirement_line(tokens: &[OwnedLexToken]) -> Option<CombatRequirementLine<'_>> {
    // Quoted grants are parsed by their granting-ability owner. Looking through
    // their quotes would move the rule from the recipient onto the granter.
    if tokens.iter().any(|token| token.kind == crate::lexer::TokenKind::Quote) {
        return None;
    }
    if let Some(clause) = parse_combat_requirement(tokens)
        && clean_subject(clause.subject_tokens)
    {
        return Some(CombatRequirementLine { clause, preceding_tokens: None });
    }
    let (preceding, tail) = primitives::split_lexed_once_on_separator(tokens, || {
        (primitives::kw("and"), peek(alt((
            primitives::kw("attack"), primitives::kw("attacks"),
            primitives::kw("block"), primitives::kw("blocks"),
        )))).void()
    })?;
    let mut clause = parse_combat_requirement(tail)?;
    if !clause.subject_tokens.is_empty() {
        return None;
    }
    let (subject, _) = primitives::split_lexed_once_on_separator(preceding, || {
        alt((primitives::kw("get"), primitives::kw("gets"),
            primitives::kw("has"), primitives::kw("have"))).void()
    })?;
    clause.subject_tokens = trim_lexed_commas(subject);
    if clause.subject_tokens.is_empty() || !clean_subject(clause.subject_tokens) {
        return None;
    }
    Some(CombatRequirementLine { clause, preceding_tokens: Some(trim_lexed_commas(preceding)) })
}

fn clean_subject(tokens: &[OwnedLexToken]) -> bool {
    !tokens.iter().any(|token| {
        token.is_period() || token.kind == crate::lexer::TokenKind::Quote
            || token.is_any_word(&["gets", "get", "has", "have", "gains", "gain", "and", "target"])
    })
}

pub fn parse_flying_block_limit(tokens: &[OwnedLexToken]) -> Option<&[OwnedLexToken]> {
    fn parse<'a>(input: &mut LexStream<'a>) -> ModalResult<&'a [OwnedLexToken]> {
        let subject = repeat_till(0.., any.void(), peek(primitives::phrase(&["can", "block"])))
            .map(|((), _)| ()).take().parse_next(input)?;
        primitives::phrase(&["can", "block", "only"]).parse_next(input)?;
        alt((primitives::kw("creature"), primitives::kw("creatures"))).parse_next(input)?;
        primitives::phrase(&["with", "flying"]).parse_next(input)?;
        opt(primitives::sentence_end()).parse_next(input)?;
        Ok(subject)
    }
    let subject = primitives::probe_all(tokens, parse, "flying block limit")?;
    clean_subject(subject).then_some(subject)
}
