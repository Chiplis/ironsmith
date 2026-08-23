use winnow::combinator::{alt, peek, repeat, repeat_till};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

use super::super::super::lexer::{LexStream, OwnedLexToken, TokenKind, trim_lexed_commas};
use super::super::primitives;
use super::restrictions::{AttachedCombatRestrictionKind, parse_attached_restriction_tail_tokens};
use super::subjects::{
    AttachedSubject, parse_attached_subject_lexed, parse_equipped_creature_has_tokens,
    semantic_finish, semantic_kw, semantic_phrase,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AttachedHasAndLosesSpec<'a> {
    pub subject: AttachedSubject,
    pub subject_tokens: &'a [OwnedLexToken],
    pub grant_tokens: &'a [OwnedLexToken],
    pub lose_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AttachedKeywordsAndTriggerSpec<'a> {
    pub subject: AttachedSubject,
    pub subject_tokens: &'a [OwnedLexToken],
    pub keyword_tokens: &'a [OwnedLexToken],
    pub trigger_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttachedGetsTailKind<'a> {
    Restriction(AttachedCombatRestrictionKind),
    Loses(&'a [OwnedLexToken]),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AttachedGetsTailSpec<'a> {
    pub subject: AttachedSubject,
    pub get_token: usize,
    pub and_token: usize,
    pub tail: AttachedGetsTailKind<'a>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AttachedLegendaryGetsHasSpec<'a> {
    pub subject: AttachedSubject,
    pub subject_tokens: &'a [OwnedLexToken],
    pub get_token: usize,
    pub modifier_token: &'a OwnedLexToken,
    pub has_token: usize,
    pub keyword_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AttachedGetsAndHasSpec<'a> {
    pub subject: AttachedSubject,
    pub get_token: usize,
    pub and_token: usize,
    pub has_token: usize,
    pub ability_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AttachedAbilitySplit<'a> {
    pub and_token: usize,
    pub keyword_tokens: &'a [OwnedLexToken],
    pub granted_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EquippedActivatedGrantSpec<'a> {
    pub has_token: usize,
    pub ability_tokens: &'a [OwnedLexToken],
    pub anthem_bounds: Option<(usize, usize)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttachedLandAbilityResetSpec<'a> {
    pub subject_tokens: &'a [OwnedLexToken],
    pub granted_abilities: Vec<&'a [OwnedLexToken]>,
}

#[path = "grant_shapes/land_reset.rs"]
mod land_reset;
pub use land_reset::parse_attached_land_ability_reset_tokens;

pub fn parse_attached_has_and_loses_tokens(
    tokens: &[OwnedLexToken],
) -> Option<AttachedHasAndLosesSpec<'_>> {
    primitives::parse_all(
        tokens,
        parse_attached_has_and_loses_lexed,
        "attached has-and-loses keywords",
    )
    .ok()
}

pub fn parse_attached_keywords_and_trigger_tokens(
    tokens: &[OwnedLexToken],
) -> Option<AttachedKeywordsAndTriggerSpec<'_>> {
    primitives::parse_all(
        tokens,
        parse_attached_keywords_and_trigger_lexed,
        "attached keywords and trigger",
    )
    .ok()
}

pub fn parse_attached_gets_tail_tokens(
    tokens: &[OwnedLexToken],
) -> Option<AttachedGetsTailSpec<'_>> {
    let (subject, _) = primitives::parse_prefix(tokens, parse_attached_subject_lexed)?;
    let get_token = find_get(tokens)?;
    let relative_and = find_and(tokens.get(get_token + 1..)?)?;
    let and_token = get_token + 1 + relative_and;
    let tail_tokens = trim_lexed_commas(tokens.get(and_token + 1..)?);
    let tail = if let Some(kind) = parse_attached_restriction_tail_tokens(tail_tokens) {
        AttachedGetsTailKind::Restriction(kind)
    } else {
        let (_, loses_tokens) = primitives::parse_prefix(
            tail_tokens,
            alt((semantic_kw("lose"), semantic_kw("loses"))),
        )?;
        let loses_tokens = trim_lexed_commas(loses_tokens);
        if loses_tokens.is_empty() {
            return None;
        }
        AttachedGetsTailKind::Loses(loses_tokens)
    };
    Some(AttachedGetsTailSpec {
        subject,
        get_token,
        and_token,
        tail,
    })
}

pub fn parse_attached_legendary_gets_has_tokens(
    tokens: &[OwnedLexToken],
) -> Option<AttachedLegendaryGetsHasSpec<'_>> {
    let initial_len = tokens.len();
    let mut input = LexStream::new(tokens);
    let subject = parse_attached_subject_lexed(&mut input).ok()?;
    if !matches!(
        subject,
        AttachedSubject::EnchantedCreature | AttachedSubject::EquippedCreature
    ) {
        return None;
    }
    let subject_end = initial_len.checked_sub(input.len())?;
    semantic_kw("is").parse_next(&mut input).ok()?;
    semantic_kw("legendary").parse_next(&mut input).ok()?;
    let search_start = initial_len.checked_sub(input.len())?;
    let get_token = search_start + find_get(tokens.get(search_start..)?)?;
    let modifier_token = tokens.get(get_token + 1)?;
    let relative_has = find_has(tokens.get(get_token + 2..)?)?;
    let has_token = get_token + 2 + relative_has;
    let keyword_tokens = trim_lexed_commas(tokens.get(has_token + 1..)?);
    if keyword_tokens.is_empty() {
        return None;
    }
    Some(AttachedLegendaryGetsHasSpec {
        subject,
        subject_tokens: tokens.get(..subject_end)?,
        get_token,
        modifier_token,
        has_token,
        keyword_tokens,
    })
}

pub fn parse_attached_gets_and_has_tokens(
    tokens: &[OwnedLexToken],
) -> Option<AttachedGetsAndHasSpec<'_>> {
    let (subject, _) = primitives::parse_prefix(tokens, parse_attached_subject_lexed)?;
    if !matches!(
        subject,
        AttachedSubject::EnchantedCreature
            | AttachedSubject::EnchantedPermanent
            | AttachedSubject::EquippedCreature
            | AttachedSubject::EquippedPermanent
    ) {
        return None;
    }
    let get_token = find_get(tokens)?;
    let relative_and = find_and(tokens.get(get_token + 1..)?)?;
    let and_token = get_token + 1 + relative_and;
    let relative_has = find_has(tokens.get(and_token + 1..)?)?;
    let has_token = and_token + 1 + relative_has;
    let ability_tokens = trim_lexed_commas(tokens.get(has_token + 1..)?);
    if ability_tokens.is_empty() {
        return None;
    }
    Some(AttachedGetsAndHasSpec {
        subject,
        get_token,
        and_token,
        has_token,
        ability_tokens,
    })
}

pub fn parse_attached_ability_splits_tokens(
    tokens: &[OwnedLexToken],
) -> Vec<AttachedAbilitySplit<'_>> {
    let mut splits = Vec::new();
    let mut inside_double_quotes = false;
    let mut inside_single_quotes = false;
    let mut parenthesis_depth = 0usize;

    for (and_token, token) in tokens.iter().enumerate() {
        match token.kind {
            TokenKind::Quote => {
                inside_double_quotes = !inside_double_quotes;
                continue;
            }
            TokenKind::Apostrophe => {
                inside_single_quotes = !inside_single_quotes;
                continue;
            }
            TokenKind::LParen if !inside_double_quotes && !inside_single_quotes => {
                parenthesis_depth = parenthesis_depth.saturating_add(1);
                continue;
            }
            TokenKind::RParen if !inside_double_quotes && !inside_single_quotes => {
                parenthesis_depth = parenthesis_depth.saturating_sub(1);
                continue;
            }
            _ => {}
        }
        if inside_double_quotes
            || inside_single_quotes
            || parenthesis_depth > 0
            || !token.is_word("and")
        {
            continue;
        }

        let keyword_tokens = trim_lexed_commas(tokens.get(..and_token).unwrap_or_default());
        let granted_tokens = trim_lexed_commas(tokens.get(and_token + 1..).unwrap_or_default());
        if !keyword_tokens.is_empty() && !granted_tokens.is_empty() {
            splits.push(AttachedAbilitySplit {
                and_token,
                keyword_tokens,
                granted_tokens,
            });
        }
    }
    splits
}

pub fn parse_trigger_intro_tokens(tokens: &[OwnedLexToken]) -> bool {
    primitives::parse_prefix(
        tokens,
        alt((
            semantic_kw("when"),
            semantic_kw("whenever"),
            semantic_kw("at"),
        )),
    )
    .is_some()
}

pub fn parse_equipped_activated_grant_tokens(
    tokens: &[OwnedLexToken],
) -> Option<EquippedActivatedGrantSpec<'_>> {
    let has = parse_equipped_creature_has_tokens(tokens)?;
    let anthem_bounds = find_get(tokens.get(..has.has_token)?).map(|get_token| {
        let anthem_end = if has.has_token > get_token + 2
            && primitives::parse_all(
                tokens
                    .get(has.has_token - 1..has.has_token)
                    .unwrap_or_default(),
                primitives::kw("and").void(),
                "attached anthem connector",
            )
            .is_ok()
        {
            has.has_token - 1
        } else {
            has.has_token
        };
        (get_token, anthem_end)
    });
    Some(EquippedActivatedGrantSpec {
        has_token: has.has_token,
        ability_tokens: has.ability_tokens,
        anthem_bounds,
    })
}

fn parse_attached_has_and_loses_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<AttachedHasAndLosesSpec<'a>> {
    let (subject, subject_tokens) = parse_attached_subject_lexed
        .with_taken()
        .parse_next(input)?;
    if !matches!(
        subject,
        AttachedSubject::EnchantedCreature
            | AttachedSubject::EnchantedPermanent
            | AttachedSubject::EquippedCreature
    ) {
        return Err(primitives::backtrack_err(
            "attached has-and-loses subject",
            "creature or permanent attachment subject",
        ));
    }
    semantic_kw("has").parse_next(input)?;
    let grant_tokens = repeat_till::<_, _, (), _, _, _, _>(
        1..,
        any.void(),
        peek((
            semantic_kw("and"),
            alt((semantic_kw("lose"), semantic_kw("loses"))),
        )),
    )
    .map(|((), _)| ())
    .take()
    .parse_next(input)?;
    semantic_kw("and").parse_next(input)?;
    alt((semantic_kw("lose"), semantic_kw("loses"))).parse_next(input)?;
    let lose_tokens = take_sentence_body(input)?;
    Ok(AttachedHasAndLosesSpec {
        subject,
        subject_tokens,
        grant_tokens: trim_lexed_commas(grant_tokens),
        lose_tokens: trim_lexed_commas(lose_tokens),
    })
}

fn parse_attached_keywords_and_trigger_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<AttachedKeywordsAndTriggerSpec<'a>> {
    let (subject, subject_tokens) = parse_attached_subject_lexed
        .with_taken()
        .parse_next(input)?;
    if !matches!(
        subject,
        AttachedSubject::EnchantedCreature | AttachedSubject::EquippedCreature
    ) {
        return Err(primitives::backtrack_err(
            "attached keyword-trigger subject",
            "attached creature",
        ));
    }
    semantic_kw("has").parse_next(input)?;
    let keyword_tokens = repeat_till::<_, _, (), _, _, _, _>(
        1..,
        any.void(),
        peek((
            semantic_kw("and"),
            alt((
                semantic_kw("when"),
                semantic_kw("whenever"),
                semantic_kw("at"),
            )),
        )),
    )
    .map(|((), _)| ())
    .take()
    .parse_next(input)?;
    semantic_kw("and").parse_next(input)?;
    let trigger_tokens = take_sentence_body(input)?;
    Ok(AttachedKeywordsAndTriggerSpec {
        subject,
        subject_tokens,
        keyword_tokens: trim_lexed_commas(keyword_tokens),
        trigger_tokens: trim_lexed_commas(trigger_tokens),
    })
}

fn take_sentence_body<'a>(input: &mut LexStream<'a>) -> WResult<&'a [OwnedLexToken]> {
    let body = repeat_till::<_, _, (), _, _, _, _>(1.., any.void(), peek(semantic_finish))
        .map(|((), _)| ())
        .take()
        .parse_next(input)?;
    semantic_finish(input)?;
    Ok(body)
}

fn find_get(tokens: &[OwnedLexToken]) -> Option<usize> {
    primitives::find_prefix(tokens, || {
        alt((primitives::kw("get"), primitives::kw("gets"))).void()
    })
    .map(|(idx, (), _)| idx)
}

fn find_and(tokens: &[OwnedLexToken]) -> Option<usize> {
    primitives::find_prefix(tokens, || primitives::kw("and").void()).map(|(idx, (), _)| idx)
}

fn find_has(tokens: &[OwnedLexToken]) -> Option<usize> {
    primitives::find_prefix(tokens, || primitives::kw("has").void()).map(|(idx, (), _)| idx)
}

#[cfg(test)]
#[path = "grant_shapes/tests.rs"]
mod tests;
