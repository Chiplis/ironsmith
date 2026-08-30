use super::*;

pub(super) fn full_text_has_non_mana_activated_ability_qualifier(tokens: &[OwnedLexToken]) -> bool {
    let words = crate::lexer::parser_token_word_refs(tokens);
    crate::word_primitives::any_sequence_occurs(
        &words,
        &[
            &["if", "it", "isnt", "a", "mana", "ability"],
            &["if", "it", "isn't", "a", "mana", "ability"],
            &["if", "it", "is", "not", "a", "mana", "ability"],
        ],
    )
}

pub(super) fn mark_non_mana_activated_line(line: &mut LineAst) {
    match line {
        LineAst::Multiple(chunks) => {
            for chunk in chunks {
                mark_non_mana_activated_line(chunk);
            }
        }
        LineAst::Triggered { trigger, .. } => mark_non_mana_activated_trigger(trigger),
        _ => {}
    }
}

/// Build the display text for the first-equip-cost alternative static ability.
/// Capitalises the leading "you" and strips the trailing period.
pub(super) fn capitalize_first_equip_cost_alternative_display(tokens: &[OwnedLexToken]) -> String {
    let rendered = render_token_slice(tokens);
    let s = rendered.trim().trim_end_matches('.');
    let mut chars = s.chars();
    match chars.next() {
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

pub fn try_parse_optional_waterbend_additional_cost(
    line: &RewriteKeywordLine,
    parse_tokens: &[OwnedLexToken],
) -> Result<Option<LineAst>, CardTextError> {
    if line.kind != RewriteKeywordLineKind::AdditionalCost {
        return Ok(None);
    }

    let Some(generic) = semantic_grammar::parse_optional_waterbend_generic_tokens(parse_tokens)
    else {
        return Ok(None);
    };

    let total_cost =
        ironsmith_core::TotalCost::from_cost(crate::model::CompilerCost::VariableMana { generic });
    Ok(Some(LineAst::OptionalCost(OptionalCost::custom(
        line.info.raw_line.trim(),
        total_cost,
    ))))
}

pub fn try_parse_optional_behold_additional_cost(
    line: &RewriteKeywordLine,
    parse_tokens: &[OwnedLexToken],
) -> Result<Option<LineAst>, CardTextError> {
    if line.kind != RewriteKeywordLineKind::AdditionalCost {
        return Ok(None);
    }

    let Some(shape) =
        keyword_special_grammar::parse_optional_keyword_additional_cost_tokens(parse_tokens)
    else {
        return Ok(None);
    };

    let total_cost = parse_activation_cost(shape.cost_tokens)?;
    if total_cost.mana_cost().is_some() || total_cost.costs().len() != 1 {
        return Ok(None);
    }

    let mut optional_cost = OptionalCost::custom(line.info.raw_line.trim(), total_cost);
    if let Some(subtype) = shape.behold_subtype {
        optional_cost.reference = crate::cost::OptionalCostRef::with_discriminator(
            crate::cost::OptionalCostKind::Behold,
            subtype.to_string(),
        );
    }

    Ok(Some(LineAst::OptionalCost(optional_cost)))
}
