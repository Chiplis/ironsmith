use crate::effect::Value;
use crate::host::{CardTextError, EffectAst, OwnedLexToken, PlayerAst, SubjectAst, TagKey, ConditionalEffectAst};
use crate::mana::ManaSymbol;
use crate::target::ObjectFilter;

use super::activation_and_restrictions::activated_line_core::parse_devotion_value_from_add_clause;
use super::effect_sentences::clause_pattern_helpers::extract_subject_player;
use super::grammar::activation_helpers as activation_grammar;
use super::grammar::structure::parse_trailing_instead_if_predicate_lexed;
use super::keyword_static::{
    parse_add_mana_equal_amount_value, parse_add_mana_that_much_value,
    parse_dynamic_cost_modifier_value, parse_value_binding_clause_lexed,
    parse_where_x_is_number_of_filter_value,
};
use super::lexer::TokenWordView;
use super::object_filters::parse_object_filter;
pub use super::util::{
    find_activation_cost_start, join_sentences_with_period, non_article_word_refs,
    parse_subtype_flexible, parse_value, strip_leading_article_tokens,
    trim_edge_punctuation_tokens, value_contains_unbound_x,
};
pub use crate::grammar::shared_util::value_semantics::{
    parse_equal_to_aggregate_filter_value, parse_filter_comparison_tokens,
};
fn bind_revealed_this_way_count_to_last_object(value: Value) -> Value {
    match value {
        Value::Count(mut filter) => {
            for constraint in &mut filter.tagged_constraints {
                if constraint.tag.as_str()
                    == crate::tag::CompilerReferenceTag::PublicRevealed.as_str()
                {
                    constraint.tag = (crate::tag::CompilerReferenceTag::It.bind()).into();
                }
            }
            Value::Count(filter)
        }
        Value::SurfaceHinted { value, hints } => Value::SurfaceHinted {
            value: Box::new(bind_revealed_this_way_count_to_last_object(*value)),
            hints,
        },
        other => other,
    }
}

fn first_non_comma_token_index(tokens: &[OwnedLexToken]) -> usize {
    for (idx, token) in tokens.iter().enumerate() {
        if !token.is_comma() {
            return idx;
        }
    }
    tokens.len()
}

fn parse_add_mana_amount(tokens: &[OwnedLexToken]) -> Option<Value> {
    let amount_tokens = match tokens {
        [article, additional, rest @ ..]
            if article.is_word("an") && additional.is_word("additional") =>
        {
            rest
        }
        [additional, rest @ ..] if additional.is_word("additional") => rest,
        _ => tokens,
    };
    parse_value(amount_tokens).map(|(value, _)| value)
}

pub fn parse_add_mana(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);
    let clause_word_view = TokenWordView::new(tokens);
    let clause_words = clause_word_view.to_word_refs();
    let facts = activation_grammar::parse_add_mana_clause_facts(tokens);
    let wrap_instead_if_tail = |base_effect: EffectAst,
                                tail_tokens: &[OwnedLexToken]|
     -> Result<Option<EffectAst>, CardTextError> {
        if !activation_grammar::is_instead_if_tail(tail_tokens) {
            return Ok(None);
        }
        let predicate =
            parse_trailing_instead_if_predicate_lexed(tail_tokens).ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unsupported trailing mana clause (clause: '{}')",
                    clause_words.join(" ")
                ))
            })?;
        Ok(Some(EffectAst::Conditionals(ConditionalEffectAst::Conditional {
            predicate,
            if_true: vec![base_effect],
            if_false: Vec::new(),
        })))
    };

    if let Some(any_color_among) = parse_add_one_mana_any_color_among_filter(tokens)? {
        return Ok(EffectAst::subject_verb_add_one_mana_any_color_among(
            player,
            any_color_among,
        ));
    }
    if let Some(colors_among) = parse_add_mana_colors_among_filter(tokens)? {
        return Ok(EffectAst::subject_verb_add_mana_colors_among(
            player,
            colors_among,
        ));
    }
    if facts.imprinted_colors {
        return Ok(EffectAst::subject_verb_add_mana_imprinted_colors());
    }

    if facts.commander_identity {
        let amount = parse_add_mana_amount(tokens).unwrap_or(Value::Fixed(1));
        return Ok(EffectAst::subject_verb_add_mana_commander_identity(
            player, amount,
        ));
    }

    if facts.different_colors {
        let amount = parse_add_mana_amount(tokens).unwrap_or(Value::Fixed(1));
        return Ok(EffectAst::subject_verb_add_mana_any_color_with_distinct(
            player, amount, None, true,
        ));
    }

    if let Some(available_colors) = parse_any_combination_mana_colors(tokens)? {
        let mut amount = parse_add_mana_amount(tokens).unwrap_or(Value::Fixed(1));
        if matches!(amount, Value::X)
            && let Some(tail) = activation_grammar::parse_any_combination_mana_tail(tokens)
            && let Some(dynamic_amount) = parse_value_binding_clause_lexed(tail)
        {
            amount = dynamic_amount;
        }
        return Ok(EffectAst::subject_verb_add_mana_any_color(
            player,
            amount,
            Some(available_colors),
        ));
    }

    let fixed_output = activation_grammar::parse_fixed_mana_output(tokens);
    if let Some(for_each_idx) = fixed_output.first_for_each_token
        && let Some(available_colors) = parse_or_mana_color_choices(&tokens[..for_each_idx])?
    {
        let amount =
            parse_dynamic_cost_modifier_value(&tokens[for_each_idx..])?.ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unsupported dynamic color-choice mana amount (clause: '{}')",
                    clause_words.join(" ")
                ))
            })?;
        return Ok(EffectAst::subject_verb_add_mana_any_color(
            player,
            bind_revealed_this_way_count_to_last_object(amount),
            Some(available_colors),
        ));
    }

    if let Some(available_colors) = parse_or_mana_color_choices(tokens)? {
        return Ok(EffectAst::subject_verb_add_mana_any_color(
            player,
            Value::Fixed(1),
            Some(available_colors),
        ));
    }

    if !fixed_output.has_explicit_symbol && facts.chosen_color_reference {
        let amount = parse_add_mana_amount(tokens).unwrap_or(Value::Fixed(1));
        return Ok(EffectAst::subject_verb_add_mana_chosen_color(
            player, amount, None,
        ));
    }
    if let Some(tail_tokens) = facts.one_that_color_tail {
        if tail_tokens.is_empty() || is_mana_pool_tail_tokens(tail_tokens) {
            return Ok(EffectAst::subject_verb_add_mana_chosen_color(
                player,
                Value::Fixed(1),
                None,
            ));
        }
        if let Some(amount) = parse_dynamic_cost_modifier_value(tail_tokens)? {
            let amount = bind_revealed_this_way_count_to_last_object(amount);
            return Ok(EffectAst::subject_verb_add_mana_chosen_color(
                player, amount, None,
            ));
        }
        return Err(CardTextError::ParseError(format!(
            "unsupported dynamic chosen-color mana amount (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    if facts.amount_that_color {
        let amount = parse_devotion_value_from_add_clause(tokens)?
            .or_else(|| parse_add_mana_equal_amount_value(tokens))
            .unwrap_or(Value::Fixed(1));
        return Ok(EffectAst::subject_verb_add_mana_chosen_color(
            player, amount, None,
        ));
    }

    if let Some(mana_choice) = facts.choice {
        let mut amount = parse_add_mana_amount(tokens).unwrap_or(Value::Fixed(1));
        let any_one = mana_choice.kind.any_one();
        let any_type = mana_choice.kind.allow_colorless();
        let tail_tokens = mana_choice.tail_tokens;

        if tail_tokens.is_empty() || is_mana_pool_tail_tokens(tail_tokens) {
            if any_type {
                return Err(CardTextError::ParseError(format!(
                    "unsupported any-type mana clause without producer filter (clause: '{}')",
                    clause_words.join(" ")
                )));
            }
            if any_one {
                return Ok(EffectAst::subject_verb_add_mana_any_one_color(
                    player, amount,
                ));
            }
            return Ok(EffectAst::subject_verb_add_mana_any_color(
                player, amount, None,
            ));
        }

        if let Some((filter, mana_type_source)) = parse_land_could_produce_filter(tail_tokens)? {
            return Ok(EffectAst::subject_verb_add_mana_from_land_could_produce(
                player,
                amount,
                filter,
                any_type,
                any_one,
                mana_type_source,
            ));
        }

        if matches!(amount, Value::X)
            && let Some(dynamic_amount) = parse_where_x_is_number_of_filter_value(tail_tokens)
        {
            amount = dynamic_amount;
            if any_type {
                return Err(CardTextError::ParseError(format!(
                    "unsupported any-type mana clause without producer filter (clause: '{}')",
                    clause_words.join(" ")
                )));
            }
            if any_one {
                return Ok(EffectAst::subject_verb_add_mana_any_one_color(
                    player, amount,
                ));
            }
            return Ok(EffectAst::subject_verb_add_mana_any_color(
                player, amount, None,
            ));
        }

        if activation_grammar::is_player_choice_tail(tail_tokens) {
            if any_type {
                return Err(CardTextError::ParseError(format!(
                    "unsupported any-type mana clause without producer filter (clause: '{}')",
                    clause_words.join(" ")
                )));
            }
            if any_one {
                return Ok(EffectAst::subject_verb_add_mana_any_one_color(
                    player, amount,
                ));
            }
            return Ok(EffectAst::subject_verb_add_mana_any_color(
                player, amount, None,
            ));
        }
        if activation_grammar::is_removed_this_way_tail(tail_tokens)
            && let Some(dynamic_amount) = parse_dynamic_cost_modifier_value(tail_tokens)?
        {
            amount = bind_revealed_this_way_count_to_last_object(dynamic_amount);
            if any_type {
                return Err(CardTextError::ParseError(format!(
                    "unsupported any-type mana clause without producer filter (clause: '{}')",
                    clause_words.join(" ")
                )));
            }
            if any_one {
                return Ok(EffectAst::subject_verb_add_mana_any_one_color(
                    player, amount,
                ));
            }
            return Ok(EffectAst::subject_verb_add_mana_any_color(
                player, amount, None,
            ));
        }

        if activation_grammar::is_among_tail(tail_tokens) {
            if any_type {
                return Err(CardTextError::ParseError(format!(
                    "unsupported any-type mana clause without producer filter (clause: '{}')",
                    clause_words.join(" ")
                )));
            }
            if any_one {
                return Ok(EffectAst::subject_verb_add_mana_any_one_color(
                    player, amount,
                ));
            }
            return Ok(EffectAst::subject_verb_add_mana_any_color(
                player, amount, None,
            ));
        }

        let base_effect = if any_one {
            EffectAst::subject_verb_add_mana_any_one_color(player, amount)
        } else {
            EffectAst::subject_verb_add_mana_any_color(player, amount, None)
        };
        if let Some(conditional) = wrap_instead_if_tail(base_effect, tail_tokens)? {
            return Ok(conditional);
        }

        return Err(CardTextError::ParseError(format!(
            "unsupported trailing mana clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let mana = fixed_output.mana;
    let last_mana_idx = fixed_output.last_mana_token;
    let for_each_idx = fixed_output.first_for_each_token;

    if !mana.is_empty() {
        if let Some(amount) = parse_add_mana_that_much_value(tokens) {
            return Ok(EffectAst::subject_verb_add_mana_scaled(
                player, mana, amount,
            ));
        }
        if let Some(amount) = parse_devotion_value_from_add_clause(tokens)? {
            return Ok(EffectAst::subject_verb_add_mana_scaled(
                player, mana, amount,
            ));
        }
        if let Some(for_each_idx) = for_each_idx {
            let amount_tokens = &tokens[for_each_idx..];
            let amount = parse_dynamic_cost_modifier_value(amount_tokens)?.ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unsupported dynamic mana amount (clause: '{}')",
                    clause_words.join(" ")
                ))
            })?;
            let amount = bind_revealed_this_way_count_to_last_object(amount);
            return Ok(EffectAst::subject_verb_add_mana_scaled(
                player, mana, amount,
            ));
        }
        if let Some(amount) = parse_equal_to_aggregate_filter_value(tokens)
            .or_else(|| parse_add_mana_equal_amount_value(tokens))
        {
            return Ok(EffectAst::subject_verb_add_mana_scaled(
                player, mana, amount,
            ));
        }
        let trailing_tokens = last_mana_idx
            .map(|last_idx| &tokens[last_idx + 1..])
            .unwrap_or_default();
        let tail_kind = activation_grammar::classify_fixed_mana_tail(trailing_tokens);
        if !trailing_tokens.is_empty()
            && tail_kind == activation_grammar::FixedManaTailKind::ChosenColor
        {
            if mana.len() != 1 {
                return Err(CardTextError::ParseError(format!(
                    "unsupported chosen-color mana clause with multiple symbols (clause: '{}')",
                    clause_words.join(" ")
                )));
            }
            let Some(color) = mana_symbol_to_color(mana[0]) else {
                return Err(CardTextError::ParseError(format!(
                    "unsupported chosen-color mana clause with non-colored symbol (clause: '{}')",
                    clause_words.join(" ")
                )));
            };
            return Ok(EffectAst::subject_verb_add_mana_chosen_color(
                player,
                Value::Fixed(1),
                Some(color),
            ));
        }
        let supported_tail = matches!(
            tail_kind,
            activation_grammar::FixedManaTailKind::Pool
                | activation_grammar::FixedManaTailKind::Instead
        );
        if !trailing_tokens.is_empty() && !supported_tail {
            if let Some(last_idx) = last_mana_idx
                && let Some(conditional) = wrap_instead_if_tail(
                    EffectAst::subject_verb_add_mana(player, mana.clone()),
                    trim_leading_commas(&tokens[last_idx + 1..]),
                )?
            {
                return Ok(conditional);
            }
            return Err(CardTextError::ParseError(format!(
                "unsupported trailing mana clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        if mana.len() == 1
            && let Some(Value::Fixed(amount)) = parse_add_mana_amount(tokens)
            && amount > 1
        {
            return Ok(EffectAst::subject_verb_add_mana(
                player,
                std::iter::repeat_n(mana[0], amount as usize).collect(),
            ));
        }
        return Ok(EffectAst::subject_verb_add_mana(player, mana));
    }

    Err(CardTextError::ParseError(format!(
        "missing mana symbols (clause: '{}')",
        clause_words.join(" ")
    )))
}

fn parse_add_mana_colors_among_filter(
    tokens: &[OwnedLexToken],
) -> Result<Option<ObjectFilter>, CardTextError> {
    let Some(span) = activation_grammar::parse_colors_among_span(tokens) else {
        return Ok(None);
    };
    let filter_tokens = span.filter_tokens;
    if filter_tokens.is_empty() {
        return Ok(None);
    }
    let filter = parse_object_filter(filter_tokens, false)?;
    Ok(Some(filter))
}

#[path = "activation_helpers/reference.rs"]
mod reference_programs;
use reference_programs::parse_add_one_mana_any_color_among_filter;
pub use reference_programs::parse_land_could_produce_filter;
#[path = "activation_helpers/resource.rs"]
mod resource_programs;
pub use resource_programs::{
    is_mana_pool_tail_tokens, mana_symbol_to_color, parse_any_combination_mana_colors,
};
#[path = "activation_helpers/core.rs"]
mod core_programs;
pub use core_programs::trim_leading_commas;
#[path = "activation_helpers/choice.rs"]
mod choice_programs;
pub use choice_programs::parse_or_mana_color_choices;
