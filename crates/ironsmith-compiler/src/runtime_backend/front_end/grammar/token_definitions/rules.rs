use crate::mana::ManaSymbol;
use crate::object::CounterType;
use crate::runtime_backend::front_end::lexer::{
    OwnedLexToken, TokenKind, parser_token_word_refs, render_token_slice,
};
use crate::runtime_backend::token_definition::{
    BuiltinTokenShape, InlineNoncreatureSpellDamageShape, TokenCrewShape, TokenEmbeddedRuleShape,
    TokenEquipShape, TokenPowerAsThoughGreaterShape, TokenRulesSurfaces, TokenSacrificeReturnShape,
    TokenTapManaAbilityShape, TokenTapSacrificeManaLifeShape,
};
use crate::{effect::Value, filter::ObjectFilter};
use winnow::combinator::{alt, opt, peek, repeat_till, separated};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

use super::super::{effects, filters, leaf, primitives};
use super::common;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct LandEntersCounterRuleShape<'a> {
    counter_type: CounterType,
    count: u32,
    target_tokens: &'a [OwnedLexToken],
}

fn parse_damage_clause(input: &mut primitives::WordSliceInput<'_>) -> WResult<i32> {
    alt((
        primitives::word_slice_exact("deals"),
        primitives::word_slice_exact("deal"),
    ))
    .parse_next(input)?;
    let Some((amount_word, rest)) = input.split_first() else {
        return Err(primitives::backtrack_err("token damage", "damage amount"));
    };
    let amount = leaf::parse_number_i32_complete(amount_word)
        .map_err(|_| primitives::backtrack_err("token damage", "damage amount"))?;
    *input = rest;
    primitives::word_slice_exact("damage").parse_next(input)?;
    Ok(amount)
}

pub(super) fn damage_amount(words: &[&str]) -> Option<i32> {
    let mut input: primitives::WordSliceInput<'_> = words;
    let (_, amount) = repeat_till::<_, _, (), _, _, _, _>(
        0..,
        any.void(),
        |candidate: &mut primitives::WordSliceInput<'_>| parse_damage_clause(candidate),
    )
    .parse_next(&mut input)
    .ok()?;
    Some(amount)
}

fn unsigned_amount_after(words: &[&str], marker: &str) -> Option<u32> {
    let marker_idx = common::first_word_offset(words, marker)?;
    let amount_word = words.get(marker_idx + 1)?;
    leaf::parse_number_complete(amount_word).ok()
}

pub(crate) fn parse_token_crew_shape_words(words: &[&str]) -> Option<TokenCrewShape> {
    Some(TokenCrewShape {
        amount: unsigned_amount_after(words, "crew")?,
    })
}

pub(crate) fn parse_token_equip_shape_words(words: &[&str]) -> Option<TokenEquipShape> {
    Some(TokenEquipShape {
        amount: unsigned_amount_after(words, "equip")?,
    })
}

pub(crate) fn parse_token_power_as_though_greater_shape_words(
    words: &[&str],
) -> Option<TokenPowerAsThoughGreaterShape> {
    let were_idx = common::first_word_offset(words, "were")?;
    let amount = leaf::parse_number_complete(words.get(were_idx + 1)?).ok()?;
    if words.get(were_idx + 2).copied() != Some("greater") {
        return None;
    }
    Some(TokenPowerAsThoughGreaterShape { amount })
}

fn parse_token_power_as_though_greater_shape_lexed<'a>(
    input: &mut crate::runtime_backend::front_end::lexer::LexStream<'a>,
) -> WResult<TokenPowerAsThoughGreaterShape> {
    primitives::kw("this").parse_next(input)?;
    alt((primitives::kw("creature"), primitives::kw("token"))).parse_next(input)?;
    opt(primitives::phrase(&["saddles", "mounts", "and"])).parse_next(input)?;
    primitives::phrase(&["crews", "vehicles", "as", "though", "its", "power", "were"])
        .parse_next(input)?;
    let amount = leaf::parse_leaf_number_prefix_lexed.parse_next(input)?;
    primitives::kw("greater").parse_next(input)?;
    Ok(TokenPowerAsThoughGreaterShape { amount })
}

pub(crate) fn parse_token_power_as_though_greater_shape_tokens(
    tokens: &[OwnedLexToken],
) -> Option<TokenPowerAsThoughGreaterShape> {
    primitives::parse_all(
        tokens,
        (
            parse_token_power_as_though_greater_shape_lexed,
            primitives::sentence_end(),
        )
            .map(|(shape, ())| shape),
        "token saddle/crew power bonus",
    )
    .ok()
}

#[path = "rules/embedded_rules.rs"]
mod embedded_rules;
pub(crate) use embedded_rules::parse_embedded_token_rule_tokens;
pub(crate) use embedded_rules::parse_inline_noncreature_spell_damage_tokens;

fn trimmed_render(tokens: &[OwnedLexToken]) -> String {
    render_token_slice(tokens).trim().to_string()
}

fn trimmed_quote_chars(text: &str) -> &str {
    let mut start = 0usize;
    for (idx, ch) in text.char_indices() {
        if !matches!(
            ch as u32,
            0x20 | 0x09 | 0x0A | 0x0D | 0x0B | 0x0C | 0x27 | 0x22 | 0x201C | 0x201D
        ) {
            start = idx;
            break;
        }
        start = idx + ch.len_utf8();
    }
    let mut end = text.len();
    for (idx, ch) in text.char_indices().rev() {
        if idx < start {
            break;
        }
        if !matches!(
            ch as u32,
            0x20 | 0x09 | 0x0A | 0x0D | 0x0B | 0x0C | 0x27 | 0x22 | 0x201C | 0x201D
        ) {
            end = idx + ch.len_utf8();
            break;
        }
        end = idx;
    }
    &text[start..end]
}

fn strip_quote_tokens(mut tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    loop {
        let mut changed = false;
        if tokens
            .first()
            .is_some_and(|token| matches!(token.kind, TokenKind::Quote | TokenKind::Apostrophe))
        {
            tokens = &tokens[1..];
            changed = true;
        }
        if tokens
            .last()
            .is_some_and(|token| matches!(token.kind, TokenKind::Quote | TokenKind::Apostrophe))
        {
            tokens = &tokens[..tokens.len().saturating_sub(1)];
            changed = true;
        }
        if !changed {
            break;
        }
    }
    tokens
}

pub(super) fn rendered_unquoted(tokens: &[OwnedLexToken]) -> String {
    trimmed_quote_chars(&trimmed_render(strip_quote_tokens(tokens))).to_string()
}

pub(super) fn first_double_quoted_tokens(tokens: &[OwnedLexToken]) -> Option<&[OwnedLexToken]> {
    let (open_idx, _, after_open) = primitives::find_prefix(tokens, || primitives::quote().void())?;
    let consumed_through_open = tokens.len().checked_sub(after_open.len())?;
    let (relative_close, _, _) =
        primitives::find_prefix(after_open, || primitives::quote().void())?;
    let close_idx = consumed_through_open + relative_close;
    let quoted = tokens.get(open_idx + 1..close_idx)?;
    (!quoted.is_empty()).then_some(quoted)
}

fn parse_tap_symbol<'a>(
    input: &mut crate::runtime_backend::front_end::lexer::LexStream<'a>,
) -> WResult<()> {
    any.verify(|token: &&OwnedLexToken| {
        token.kind == TokenKind::ManaGroup
            && token
                .mana_group_inner()
                .is_some_and(|inner| inner.eq_ignore_ascii_case("t"))
    })
    .void()
    .parse_next(input)
}

fn parse_tap_add_mana_head<'a>(
    input: &mut crate::runtime_backend::front_end::lexer::LexStream<'a>,
) -> WResult<Vec<ManaSymbol>> {
    parse_tap_symbol.parse_next(input)?;
    primitives::colon().parse_next(input)?;
    primitives::kw("add").parse_next(input)?;
    let mana = leaf::parse_leaf_mana_group_token
        .verify(|symbols: &Vec<ManaSymbol>| {
            !symbols.is_empty()
                && symbols.iter().all(|symbol| {
                    matches!(
                        symbol,
                        ManaSymbol::White
                            | ManaSymbol::Blue
                            | ManaSymbol::Black
                            | ManaSymbol::Red
                            | ManaSymbol::Green
                            | ManaSymbol::Colorless
                    )
                })
        })
        .parse_next(input)?;
    primitives::end_of_sentence().parse_next(input)?;
    Ok(mana)
}

pub(crate) fn parse_token_tap_mana_ability_tokens(
    tokens: &[OwnedLexToken],
) -> Option<TokenTapManaAbilityShape> {
    let rule_tokens = first_double_quoted_tokens(tokens).or_else(|| inline_rules_tokens(tokens))?;
    let (mana, restriction_tokens) =
        primitives::parse_prefix(rule_tokens, parse_tap_add_mana_head)?;
    let restriction_tokens =
        crate::runtime_backend::front_end::lexer::trim_lexed_commas(restriction_tokens);
    let restrictions = if restriction_tokens.is_empty() {
        Vec::new()
    } else {
        vec![
            super::super::abilities::parse_mana_usage_restriction_sentence_lexed(
                restriction_tokens,
            )?,
        ]
    };
    Some(TokenTapManaAbilityShape { mana, restrictions })
}

fn contains_kind(tokens: &[OwnedLexToken], kind: TokenKind) -> bool {
    tokens.iter().any(|token| token.kind == kind)
}

fn inline_rules_start(tokens: &[OwnedLexToken]) -> Option<usize> {
    let mut earliest = None;
    for phrase in [
        &["tap"][..],
        &["sacrifice"],
        &["this"],
        &["power"],
        &["whenever"],
        &["when"],
        &["at"],
    ] {
        if let Some((idx, _, _)) =
            primitives::find_prefix(tokens, || primitives::phrase(phrase).void())
            && earliest.is_none_or(|current| idx < current)
        {
            earliest = Some(idx);
        }
    }
    let mut idx = 0usize;
    while idx < tokens.len() {
        let token = &tokens[idx];
        if token.kind == TokenKind::ManaGroup
            && matches!(token.mana_group_inner(), Some("t" | "T" | "q" | "Q"))
            && earliest.is_none_or(|current| idx < current)
        {
            earliest = Some(idx);
        }
        idx += 1;
    }
    earliest
}

fn inline_rules_tokens(tokens: &[OwnedLexToken]) -> Option<&[OwnedLexToken]> {
    let (_, _, after_with) = primitives::find_prefix(tokens, || primitives::kw("with").void())?;
    if contains_kind(after_with, TokenKind::Colon) {
        return Some(strip_quote_tokens(after_with));
    }
    let start = inline_rules_start(after_with)?;
    Some(strip_quote_tokens(after_with.get(start..)?))
}

fn pronoun_rules_tokens(tokens: &[OwnedLexToken]) -> Option<&[OwnedLexToken]> {
    let (_, tail) = primitives::parse_prefix(
        tokens,
        alt((
            primitives::phrase(&["it", "has"]),
            primitives::phrase(&["they", "have"]),
        )),
    )?;
    let tail = strip_quote_tokens(tail);
    (!tail.is_empty()).then_some(tail)
}

pub(crate) fn parse_token_rules_surfaces_tokens(tokens: &[OwnedLexToken]) -> TokenRulesSurfaces {
    parse_token_rules_surfaces_for_named_token(tokens, None)
}

pub(crate) fn parse_token_rules_surfaces_for_named_token(
    tokens: &[OwnedLexToken],
    named_token: Option<&str>,
) -> TokenRulesSurfaces {
    let quoted_or_inline = first_double_quoted_tokens(tokens)
        .or_else(|| pronoun_rules_tokens(tokens))
        .or_else(|| inline_rules_tokens(tokens));
    let embedded_rules = quoted_or_inline
        .and_then(|rule_tokens| {
            if let Some(rule) = parse_embedded_token_rule_tokens(rule_tokens, named_token) {
                return Some(rule);
            }
            let words = parser_token_word_refs(rule_tokens);
            let all = |expected: &[&str]| common::all_words_present(&words, expected);
            if all(&[
                "whenever", "opponent", "casts", "creature", "spell", "isnt", "creature", "until",
                "end", "turn",
            ]) {
                return Some(
                    TokenEmbeddedRuleShape::OpponentCastsCreatureRemoveCreatureTypeUntilEndOfTurn,
                );
            }
            let creatures_you_control = Value::Count(ObjectFilter::creature().you_control());
            if matches!(
                super::reminder::parse_token_dynamic_power_toughness_tokens(rule_tokens),
                Some((power, toughness))
                    if power == creatures_you_control && toughness == creatures_you_control
            ) {
                return Some(TokenEmbeddedRuleShape::PowerToughnessEqualCreaturesYouControl);
            }
            None
        })
        .into_iter()
        .collect();
    TokenRulesSurfaces { embedded_rules }
}

pub(super) fn cumulative_upkeep_mana_symbols(words: &[&str]) -> Option<Vec<ManaSymbol>> {
    let upkeep_idx = common::phrase_offset(words, &["cumulative", "upkeep"])?;
    let mut cost_symbols = Vec::new();
    for word in words.get(upkeep_idx + 2..)? {
        if matches!(*word, "when" | "whenever" | "at") {
            break;
        }
        let Ok(symbol) = leaf::parse_leaf_bare_mana_symbol_complete(word) else {
            break;
        };
        cost_symbols.push(symbol);
    }
    Some(cost_symbols)
}

pub(super) fn sacrifice_return_shape(
    words: &[&str],
    card_name: Option<&str>,
) -> Option<TokenSacrificeReturnShape> {
    let card_name = card_name?.to_string();
    let sacrifice_idx = common::first_word_offset(words, "sacrifice")?;
    let mut mana_symbols = Vec::new();
    let mut tap_cost = false;
    for word in words.get(..sacrifice_idx)? {
        if *word == "t" {
            tap_cost = true;
            continue;
        }
        if let Ok(symbol) = leaf::parse_leaf_bare_mana_symbol_complete(word) {
            mana_symbols.push(symbol);
        }
    }
    Some(TokenSacrificeReturnShape {
        card_name,
        mana_symbols,
        tap_cost,
    })
}

pub(super) fn toxic_amount(words: &[&str]) -> Option<u32> {
    unsigned_amount_after(words, "toxic")
}

#[cfg(test)]
#[path = "rules/tests.rs"]
mod tests;
