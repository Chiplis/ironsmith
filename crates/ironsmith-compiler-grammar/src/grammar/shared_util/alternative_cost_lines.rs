use crate::activation_and_restrictions::parse_payment_clause_as_total_cost;
use crate::cards::builders::CardTextError;
use crate::grammar::{leaf, permission_shapes};
use crate::keyword_static::parse_this_spell_cost_condition;
use crate::lexer::{OwnedLexToken, TokenWordView, lex_line, render_token_slice};
use crate::mana::ManaCost;
use crate::model::CompilerAlternativeCastingMethod as AlternativeCastingMethod;
use crate::static_abilities::ThisSpellCostCondition;

pub fn parse_self_free_cast(tokens: &[OwnedLexToken]) -> Option<AlternativeCastingMethod> {
    let words = TokenWordView::new(tokens).word_refs();
    if !exact_one_of(
        &words,
        &[
            &[
                "you", "may", "cast", "this", "spell", "without", "paying", "its", "mana", "cost",
            ],
            &[
                "you", "may", "cast", "this", "spell", "without", "paying", "this", "spells",
                "mana", "cost",
            ],
        ],
    ) {
        return None;
    }
    Some(AlternativeCastingMethod::alternative_cost(
        "Parsed alternative cost",
        None,
        Vec::new(),
    ))
}

pub fn parse_flash_with_additional_cost(
    tokens: &[OwnedLexToken],
) -> Option<AlternativeCastingMethod> {
    let words = TokenWordView::new(tokens);
    if !permission_shapes::prefix_words(
        &words.word_refs(),
        &[
            "you", "may", "cast", "this", "spell", "as", "though", "it", "had", "flash", "if",
            "you", "pay",
        ],
    ) {
        return None;
    }
    let cost_start = words.token_start_indices().get(13).copied()?;
    let parsed = leaf::parse_leaf_mana_cost_prefix_tokens(&tokens[cost_start..])?;
    let suffix = TokenWordView::new(&tokens[cost_start + parsed.consumed..]).word_refs();
    if !permission_shapes::exact_words(&suffix, &["more", "to", "cast", "it"]) {
        return None;
    }
    Some(AlternativeCastingMethod::flash_with_additional_cost(
        parsed.cost,
        ironsmith_core::TotalCost::<crate::model::CompilerCost>::free(),
    ))
}

pub fn parse_you_may_rather_than_spell_cost(
    tokens: &[OwnedLexToken],
    line: &str,
) -> Result<Option<AlternativeCastingMethod>, CardTextError> {
    let word_view = TokenWordView::new(tokens);
    let words = word_view.word_refs();
    if !permission_shapes::prefix_words(&words, &["you", "may"]) {
        return Ok(None);
    }
    let Some(rather_word) = permission_shapes::find_words(&words, &["rather"]) else {
        return Ok(None);
    };
    let Some(rather_token) = word_view.token_start_indices().get(rather_word).copied() else {
        return Ok(None);
    };
    let rather_tail =
        TokenWordView::new(tokens.get(rather_token + 1..).unwrap_or_default()).word_refs();
    if !is_rather_than_spell_cost_tail(&rather_tail) {
        return Ok(None);
    }
    let cost_clause_end = last_cost_word_token(tokens.get(rather_token + 1..).unwrap_or_default())
        .map(|relative| rather_token + 1 + relative)
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "alternative cost line missing terminal cost word (line: '{}')",
                line
            ))
        })?;
    if !TokenWordView::new(&tokens[cost_clause_end + 1..])
        .word_refs()
        .is_empty()
    {
        return Err(CardTextError::ParseError(format!(
            "unsupported trailing clause after alternative cost (line: '{}', trailing: '{}')",
            line,
            TokenWordView::new(&tokens[cost_clause_end + 1..])
                .word_refs()
                .join(" ")
        )));
    }
    let cost_tokens = tokens.get(2..rather_token).unwrap_or_default();
    if cost_tokens.is_empty() {
        return Err(CardTextError::ParseError(
            "alternative cost line missing cost clause".to_string(),
        ));
    }
    let total_cost = parse_payment_clause_as_total_cost(cost_tokens)?.ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unsupported alternative cost clause (line: '{}', cost: '{}')",
            line,
            render_token_slice(cost_tokens).trim()
        ))
    })?;
    Ok(Some(AlternativeCastingMethod::Composed {
        name: "Parsed alternative cost".into(),
        total_cost,
        condition: None,
        prototype_power_toughness: None,
    }))
}

pub fn parse_if_conditional_alternative_cost(
    tokens: &[OwnedLexToken],
    line_tokens: &[OwnedLexToken],
) -> Result<Option<AlternativeCastingMethod>, CardTextError> {
    let line = render_token_slice(line_tokens);
    let line = line.as_str();
    let clause_words = TokenWordView::new(tokens).word_refs();
    if !permission_shapes::prefix_words(&clause_words, &["if"]) {
        return Ok(None);
    }
    let Some((condition_tokens, tail_tokens)) = split_condition_and_cost_tail(tokens) else {
        return Ok(None);
    };
    if parse_self_free_cast(tail_tokens).is_none()
        && parse_you_may_rather_than_spell_cost(tail_tokens, line)?.is_none()
    {
        return Ok(None);
    }

    let condition = if let Some(condition) = parse_this_spell_cost_condition(condition_tokens) {
        condition
    } else {
        parse_special_cost_condition(condition_tokens).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unsupported this-spell cost condition (clause: '{}')",
                clause_words.join(" ")
            ))
        })?
    };

    if parse_self_free_cast(tail_tokens).is_some() {
        let method = AlternativeCastingMethod::alternative_cost_with_condition(
            "Parsed alternative cost",
            None,
            Vec::new(),
            condition,
        );
        return Ok(Some(normalize_trap_method(method)));
    }

    let Some(method) = parse_you_may_rather_than_spell_cost(tail_tokens, line)? else {
        return Ok(None);
    };
    if permission_shapes::prefix_tokens(line_tokens, &["freerunning"])
        && let Some(cost) = method.mana_cost().cloned()
    {
        return Ok(Some(
            AlternativeCastingMethod::alternative_cost_with_condition(
                "Freerunning",
                Some(cost),
                method.non_mana_costs(),
                condition,
            ),
        ));
    }
    Ok(Some(normalize_trap_method(
        method.with_cast_condition(condition),
    )))
}

fn split_condition_and_cost_tail(
    tokens: &[OwnedLexToken],
) -> Option<(&[OwnedLexToken], &[OwnedLexToken])> {
    if let Some(comma) = first_comma(tokens) {
        return Some((
            trim_commas(&tokens[1..comma]),
            trim_commas(tokens.get(comma + 1..).unwrap_or_default()),
        ));
    }
    let view = TokenWordView::new(tokens);
    let may_word = permission_shapes::find_words(&view.word_refs(), &["you", "may", "pay"])?;
    let may_token = view.token_start_indices().get(may_word).copied()?;
    Some((
        trim_commas(&tokens[1..may_token]),
        trim_commas(&tokens[may_token..]),
    ))
}

fn parse_special_cost_condition(tokens: &[OwnedLexToken]) -> Option<ThisSpellCostCondition> {
    let words = TokenWordView::new(tokens).word_refs();
    if permission_shapes::prefix_words(
        &words,
        &[
            "you",
            "dealt",
            "combat",
            "damage",
            "to",
            "a",
            "player",
            "this",
            "turn",
            "with",
            "an",
            "assassin",
            "or",
            "commander",
        ],
    ) {
        return Some(
            ThisSpellCostCondition::YouDealtCombatDamageToPlayerWithSubtypeOrCommanderThisTurn(
                crate::types::Subtype::Assassin,
            ),
        );
    }
    let count_start =
        if permission_shapes::prefix_words(&words, &["youve", "been", "dealt", "damage", "by"]) {
            5
        } else if permission_shapes::prefix_words(
            &words,
            &["you", "have", "been", "dealt", "damage", "by"],
        ) {
            6
        } else {
            return None;
        };
    if !permission_shapes::suffix_words(&words, &["creatures", "this", "turn"]) {
        return None;
    }
    let count_token = TokenWordView::new(tokens)
        .token_start_indices()
        .get(count_start)
        .copied()?;
    let (count, _) = leaf::parse_leaf_number_prefix_tokens(&tokens[count_token..])?.into_fixed()?;
    Some(ThisSpellCostCondition::YouWereDealtDamageByCreaturesThisTurnOrMore(count))
}

fn normalize_trap_method(method: AlternativeCastingMethod) -> AlternativeCastingMethod {
    let trap = method
        .cast_condition()
        .and_then(trap_condition_from_this_spell_cost_condition)
        .zip(simple_trap_cost_from_alternative_method(&method));
    if let Some((condition, cost)) = trap {
        AlternativeCastingMethod::trap("Trap", cost, condition)
    } else {
        method
    }
}

fn trap_condition_from_this_spell_cost_condition(
    condition: &ThisSpellCostCondition,
) -> Option<crate::TrapCondition> {
    match condition {
        ThisSpellCostCondition::OpponentCastSpellsThisTurnOrMore(count) => {
            Some(crate::TrapCondition::OpponentCastSpells { count: *count })
        }
        ThisSpellCostCondition::YouWereDealtDamageByCreaturesThisTurnOrMore(_) => {
            Some(crate::TrapCondition::CreatureDealtDamageToYou)
        }
        _ => None,
    }
}

fn simple_trap_cost_from_alternative_method(method: &AlternativeCastingMethod) -> Option<ManaCost> {
    let AlternativeCastingMethod::Composed { total_cost, .. } = method else {
        return None;
    };
    if total_cost.non_mana_costs().next().is_some() {
        return None;
    }
    Some(
        total_cost
            .mana_cost()
            .cloned()
            .unwrap_or_else(ManaCost::new),
    )
}

fn is_rather_than_spell_cost_tail(words: &[&str]) -> bool {
    permission_shapes::prefix_words(words, &["than", "pay", "this"])
        && permission_shapes::find_words(words, &["mana", "cost"]).is_some()
        && ["spell", "spells"]
            .iter()
            .any(|word| permission_shapes::find_words(words, &[*word]).is_some())
}

fn last_cost_word_token(tokens: &[OwnedLexToken]) -> Option<usize> {
    let mut idx = tokens.len();
    while idx > 0 {
        idx -= 1;
        if tokens[idx].as_word().is_some_and(|word| {
            permission_shapes::exact_words(&[word], &["cost"])
                || permission_shapes::exact_words(&[word], &["costs"])
        }) {
            return Some(idx);
        }
    }
    None
}

fn first_comma(tokens: &[OwnedLexToken]) -> Option<usize> {
    let mut idx = 0;
    while idx < tokens.len() {
        if tokens[idx].is_comma() {
            return Some(idx);
        }
        idx += 1;
    }
    None
}

fn trim_commas(mut tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    while tokens.first().is_some_and(OwnedLexToken::is_comma) {
        tokens = &tokens[1..];
    }
    while tokens.last().is_some_and(OwnedLexToken::is_comma) {
        tokens = &tokens[..tokens.len() - 1];
    }
    tokens
}

fn exact_one_of(words: &[&str], alternatives: &[&[&str]]) -> bool {
    alternatives
        .iter()
        .any(|expected| permission_shapes::exact_words(words, expected))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_free_cast_surfaces() {
        let tokens = lex_line("you may cast this spell without paying its mana cost", 0)
            .expect("lex fixture");
        assert!(parse_self_free_cast(&tokens).is_some());
    }
}
