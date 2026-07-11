use crate::PtValue;
use crate::runtime_backend::front_end::lexer::{OwnedLexToken, TokenKind, parser_token_word_refs};
use crate::runtime_backend::token_definition::{
    EquipmentDamageGrantShape, EquipmentGrantCountShape, EquipmentRuleLineShape,
    EquipmentRulesShape, EquipmentScaledPowerToughnessShape, TokenKeywordShape,
};
use winnow::combinator::{alt, peek, repeat_till};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

use super::super::{filters, leaf, primitives};
use super::common;
use super::rules;

fn join_and_list(parts: &[&str]) -> String {
    match parts.len() {
        0 => String::new(),
        1 => parts[0].to_string(),
        2 => format!("{} and {}", parts[0], parts[1]),
        _ => {
            let mut out = parts[..parts.len() - 1].join(", ");
            out.push_str(", and ");
            out.push_str(parts.last().copied().unwrap_or_default());
            out
        }
    }
}

fn token_kind_present(tokens: &[OwnedLexToken], kind: TokenKind) -> bool {
    tokens.iter().any(|token| token.kind == kind)
}

fn granted_ability_tokens(tokens: &[OwnedLexToken]) -> Option<&[OwnedLexToken]> {
    let (_, _, tail) = primitives::find_prefix(tokens, || {
        primitives::phrase(&["equipped", "creature", "has"])
    })?;
    let mut inside_quotes = false;
    let mut equip_idx = None;
    let mut idx = 0usize;
    while idx < tail.len() {
        let token = &tail[idx];
        if matches!(token.kind, TokenKind::Quote | TokenKind::Apostrophe) {
            inside_quotes = !inside_quotes;
        } else if !inside_quotes && token.is_word("equip") {
            equip_idx = Some(idx);
            break;
        }
        idx += 1;
    }
    let end = match equip_idx {
        Some(idx) if idx > 0 && tail[idx - 1].is_word("and") => idx - 1,
        Some(idx) => idx,
        None => tail.len(),
    };
    let ability_tokens = tail.get(..end)?;
    (!ability_tokens.is_empty()).then_some(ability_tokens)
}

#[path = "equipment/scaled_grants.rs"]
mod scaled_grants;
use scaled_grants::scaled_equipment_grant_tokens;

pub(crate) fn parse_equipment_rules_tokens(
    tokens: &[OwnedLexToken],
) -> Option<EquipmentRulesShape> {
    let words = parser_token_word_refs(&tokens);
    if !common::phrase_present(&words, &["equipped", "creature"]) {
        return None;
    }

    let mut lines = Vec::new();
    let mut typed_lines = Vec::new();
    if let Some(ability_tokens) = granted_ability_tokens(&tokens)
        && token_kind_present(ability_tokens, TokenKind::Colon)
    {
        let mut granted_text = rules::rendered_unquoted(ability_tokens);
        if !matches!(granted_text.chars().last(), Some('.' | '!' | '?')) {
            granted_text.push('.');
        }
        let display_text = format!("Equipped creature has \"{granted_text}\"");
        typed_lines.push(EquipmentRuleLineShape::GrantedDamage {
            display_text: display_text.clone(),
            grant: parse_equipment_damage_grant_tokens(ability_tokens)?,
        });
        lines.push(display_text);
    }

    if lines.is_empty() {
        let has_plus_one = common::phrase_present(&words, &["gets", "+1/+1"]);
        let scaled_power_toughness = scaled_equipment_grant_tokens(tokens);
        let mut granted_keywords = Vec::new();
        for keyword in [
            "vigilance",
            "trample",
            "haste",
            "flying",
            "lifelink",
            "deathtouch",
            "menace",
            "reach",
            "hexproof",
            "indestructible",
        ] {
            if common::word_present(&words, keyword) {
                granted_keywords.push(keyword);
            }
        }
        if let Some(scaled) = scaled_power_toughness {
            let EquipmentGrantCountShape::CountersAmongPermanentsYouControl(counter_type) =
                scaled.count;
            let display_text = format!(
                "Equipped creature gets {:+}/{:+} for each {} counter among permanents you control.",
                scaled.power,
                scaled.toughness,
                counter_type.description()
            );
            typed_lines.push(EquipmentRuleLineShape::StaticGrant {
                display_text: display_text.clone(),
                power_toughness: None,
                scaled_power_toughness: Some(scaled),
                keywords: token_keywords(&words),
            });
            lines.push(display_text);
        } else if has_plus_one {
            if granted_keywords.is_empty() {
                let display_text = "Equipped creature gets +1/+1.".to_string();
                typed_lines.push(EquipmentRuleLineShape::StaticGrant {
                    display_text: display_text.clone(),
                    power_toughness: Some((1, 1)),
                    scaled_power_toughness: None,
                    keywords: Vec::new(),
                });
                lines.push(display_text);
            } else {
                let display_text = format!(
                    "Equipped creature gets +1/+1 and has {}.",
                    join_and_list(&granted_keywords)
                );
                typed_lines.push(EquipmentRuleLineShape::StaticGrant {
                    display_text: display_text.clone(),
                    power_toughness: Some((1, 1)),
                    scaled_power_toughness: None,
                    keywords: token_keywords(&words),
                });
                lines.push(display_text);
            }
        } else if !granted_keywords.is_empty() {
            let display_text = format!(
                "Equipped creature has {}.",
                join_and_list(&granted_keywords)
            );
            typed_lines.push(EquipmentRuleLineShape::StaticGrant {
                display_text: display_text.clone(),
                power_toughness: None,
                scaled_power_toughness: None,
                keywords: token_keywords(&words),
            });
            lines.push(display_text);
        }
    }

    if let Some(equip) = rules::parse_token_equip_shape_words(&words) {
        lines.push(format!("Equip {{{}}}", equip.amount));
        typed_lines.push(EquipmentRuleLineShape::Equip(equip));
    }

    if lines.is_empty() {
        return None;
    }
    let text = lines.join("\n");
    Some(EquipmentRulesShape {
        text,
        lines: typed_lines,
    })
}

pub(super) fn token_keywords(words: &[&str]) -> Vec<TokenKeywordShape> {
    let mut keywords = Vec::new();
    for (word, keyword) in [
        ("vigilance", TokenKeywordShape::Vigilance),
        ("trample", TokenKeywordShape::Trample),
        ("haste", TokenKeywordShape::Haste),
        ("flying", TokenKeywordShape::Flying),
        ("lifelink", TokenKeywordShape::Lifelink),
        ("deathtouch", TokenKeywordShape::Deathtouch),
        ("menace", TokenKeywordShape::Menace),
        ("reach", TokenKeywordShape::Reach),
    ] {
        if common::word_present(words, word) {
            keywords.push(keyword);
        }
    }
    keywords
}

pub(super) fn first_generic_mana(tokens: &[OwnedLexToken]) -> Option<u32> {
    for token in tokens {
        if let Some(inner) = token.mana_group_inner()
            && let Ok(amount) = inner.parse::<u32>()
        {
            return Some(amount);
        }
    }
    None
}

pub(crate) fn parse_equipment_damage_grant_tokens(
    tokens: &[OwnedLexToken],
) -> Option<EquipmentDamageGrantShape> {
    let (colon_idx, _, effect_tokens) =
        primitives::find_prefix(tokens, || primitives::colon().void())?;
    let cost_tokens = tokens.get(..colon_idx)?;
    let effect_words = parser_token_word_refs(effect_tokens);
    if !common::phrase_present(&effect_words, &["this", "creature", "deals"])
        || !common::phrase_present(&effect_words, &["any", "target"])
    {
        return None;
    }
    let all_words = parser_token_word_refs(tokens);
    Some(EquipmentDamageGrantShape {
        generic_amount: first_generic_mana(cost_tokens),
        tap_cost: tokens.iter().any(|token| token.parser_text == "{t}"),
        sacrifice_equipment: common::word_present(&all_words, "sacrifice"),
        damage_amount: rules::damage_amount(&effect_words)?,
    })
}
