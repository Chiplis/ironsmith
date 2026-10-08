//! Absorb N (CR 702.64a: "If a source would deal damage to this creature,
//! prevent N of that damage."), printed or granted ("All Sliver creatures
//! have absorb 1." — Lymph Sliver). Lowered to the same typed self-prevention
//! the spelled-out sentence produces.

use super::*;

fn absorb_prevention(amount: u32) -> StaticAbility {
    StaticAbility::prevent_matching_damage(ironsmith_core::PreventMatchingDamageSpec {
        source_filter: ObjectFilter::default(),
        target_player_filter: None,
        target_object_filter: Some(ObjectFilter::source()),
        combat_only: false,
        noncombat_only: false,
        maximum_damage: None,
        amount: ironsmith_core::StaticDamagePreventionAmount::Amount(Value::Fixed(
            i32::try_from(amount).unwrap_or(i32::MAX),
        )),
        display: format!("Absorb {amount}"),
    })
}

fn absorb_amount(tokens: &[OwnedLexToken]) -> Option<u32> {
    let [keyword, amount] = tokens else {
        return None;
    };
    if !keyword.is_word("absorb") {
        return None;
    }
    parse_number_word_i32(amount.parser_text())
        .and_then(|value| u32::try_from(value).ok())
        .filter(|value| *value > 0)
}

pub fn parse_absorb_keyword_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let tokens = trim_edge_punctuation(tokens);
    if let Some(amount) = absorb_amount(&tokens) {
        return Ok(Some(vec![StaticAbilityAst::Static(absorb_prevention(amount))]));
    }
    let Some(have_idx) = tokens
        .iter()
        .position(|token| token.is_any_word(&["have", "has"]))
    else {
        return Ok(None);
    };
    if have_idx == 0 {
        return Ok(None);
    }
    let Some(amount) = absorb_amount(&tokens[have_idx + 1..]) else {
        return Ok(None);
    };
    let ability = StaticAbilityAst::Static(absorb_prevention(amount));
    Ok(Some(vec![match parse_anthem_subject(&tokens[..have_idx])? {
        AnthemSubjectAst::Source => ability,
        AnthemSubjectAst::Filter(filter) => StaticAbilityAst::GrantStaticAbility {
            filter,
            ability: Box::new(ability),
            condition: None,
        },
    }]))
}
