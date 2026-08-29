use super::*;

pub fn parse_compound_buff_and_unblockable_static_chunk(
    parse_tokens: &[OwnedLexToken],
) -> Result<Option<LineAst>, CardTextError> {
    let Some(parsed) = effect_grammar::parse_compound_buff_unblockable_tokens(parse_tokens) else {
        return Ok(None);
    };
    let buff_tokens = parsed.buff_tokens.to_vec();
    let mut unblockable_tokens =
        Vec::with_capacity(parsed.subject_tokens.len() + parsed.unblockable_tail_tokens.len());
    unblockable_tokens.extend_from_slice(parsed.subject_tokens);
    unblockable_tokens.extend_from_slice(parsed.unblockable_tail_tokens);

    if let Some(abilities) = parse_static_ability_ast_line_lexed(parse_tokens)? {
        return Ok(Some(LineAst::StaticAbilities(abilities)));
    }

    let Some(mut abilities) = parse_static_ability_ast_line_lexed(&buff_tokens)? else {
        return Ok(None);
    };
    let Some(unblockable_abilities) = parse_static_ability_ast_line_lexed(&unblockable_tokens)?
    else {
        return Ok(None);
    };
    abilities.extend(unblockable_abilities);
    Ok(Some(LineAst::StaticAbilities(abilities)))
}

pub fn parse_split_static_chunk(
    line: &RewriteStaticLine,
    parse_tokens: &[OwnedLexToken],
) -> Result<Option<LineAst>, CardTextError> {
    let sentences = split_lexed_sentences(parse_tokens);
    if sentences.len() <= 1 {
        return Ok(None);
    }

    let mut abilities = Vec::new();
    for sentence_tokens in sentences {
        if let Some(ability) = parse_if_this_spell_costs_less_to_cast_line_lexed(sentence_tokens)? {
            abilities.push(ability.into());
            continue;
        }
        if let Some(parsed) = parse_static_ability_ast_line_lexed(sentence_tokens)? {
            abilities.extend(parsed);
            continue;
        }
        return Ok(None);
    }

    wrap_chosen_option_static_chunk(
        LineAst::StaticAbilities(abilities),
        line.chosen_option.as_ref(),
    )
    .map(Some)
}
