use super::super::super::front_end::grammar::effects::misc_action_shapes::parse_chosen_object_set_filter_tokens;
use super::super::super::front_end::grammar::effects::{
    TapControlRelation, parse_tap_control_relation_tokens, parse_tap_or_untap_all_shape_tokens,
    parse_tap_or_untap_target_tokens, parse_tap_quantified_filter_tokens,
    parse_tap_then_return_tokens, parse_tap_type_choice_tokens, tap_tokens_mention_chosen_type,
};
use super::super::super::lexer::LexedClause;
use super::*;

pub fn collapse_leading_signed_pt_modifier_tokens(
    tokens: &[OwnedLexToken],
) -> Option<Vec<OwnedLexToken>> {
    let sign = match tokens.first()?.kind {
        crate::lexer::TokenKind::Dash => "-",
        crate::lexer::TokenKind::Plus => "+",
        _ => return None,
    };
    let modifier = tokens.get(1)?.as_word()?;
    if !crate::string_primitives::contains_char(modifier, '/') {
        return None;
    }

    let mut collapsed = Vec::with_capacity(tokens.len().saturating_sub(1));
    collapsed.push(OwnedLexToken::word(
        format!("{sign}{modifier}"),
        tokens[0].span(),
    ));
    collapsed.extend(tokens.iter().skip(2).cloned());
    Some(collapsed)
}

#[path = "tap_actions/tap_readings.rs"]
mod tap_readings;

pub fn parse_tap(tokens: &[OwnedLexToken]) -> Result<EffectAst, CardTextError> {
    let clause = LexedClause::new(tokens);
    if clause.is_empty() {
        return Err(CardTextError::ParseError(
            "tap clause missing target".to_string(),
        ));
    }
    let input = tap_readings::TapClause {
        tokens,
        read_by_cache: Default::default(),
    };
    match tap_readings::read(&input) {
        crate::recognition::ParseOutcome::Match(matched) => return Ok(matched.value.value),
        crate::recognition::ParseOutcome::NoMatch => {}
        crate::recognition::ParseOutcome::Error(diagnostic) => {
            return Err(diagnostic.into_card_text_error());
        }
    }
    let target = if crate::word_primitives::parse_sequence_complete(
        &crate::lexer::parser_token_word_refs(tokens),
        &["it"],
    ) {
        TargetAst::Tagged(
            crate::tag::CompilerReferenceTag::It.bind(),
            span_from_tokens(tokens),
        )
    } else {
        parse_target_phrase(tokens)?
    };
    Ok(EffectAst::subject_verb_tap(target))
}

fn parse_tap_or_untap_all(tokens: &[OwnedLexToken]) -> Result<Option<EffectAst>, CardTextError> {
    let Some(shape) = parse_tap_or_untap_all_shape_tokens(tokens) else {
        return Ok(None);
    };
    let left_tokens = shape.tap_filter_tokens.to_vec();
    let right_tokens = shape.untap_filter_tokens.to_vec();

    let (cleaned_left, left_mentions_chosen_type) = analyze_tap_type_choice_reference(&left_tokens);
    let (cleaned_right, right_mentions_chosen_type) =
        analyze_tap_type_choice_reference(&right_tokens);

    let mut tap_filter = parse_object_filter(&cleaned_left, false)?;
    let mut untap_filter = parse_object_filter(&cleaned_right, false)?;
    if left_mentions_chosen_type {
        tap_filter.chosen_creature_type = true;
    }
    if right_mentions_chosen_type {
        untap_filter.chosen_creature_type = true;
    }
    if parse_tap_control_relation_tokens(&left_tokens) == Some(TapControlRelation::TargetPlayer) {
        tap_filter.controller = Some(PlayerFilter::target_player());
    }
    if parse_tap_control_relation_tokens(&right_tokens) == Some(TapControlRelation::ThatPlayer) {
        untap_filter.controller = tap_filter
            .controller
            .clone()
            .or_else(|| Some(PlayerFilter::target_player()));
    }

    Ok(Some(EffectAst::subject_verb_tap_or_untap_all(
        tap_filter,
        untap_filter,
    )))
}

fn analyze_tap_type_choice_reference(tokens: &[OwnedLexToken]) -> (Vec<OwnedLexToken>, bool) {
    let clause = LexedClause::new(tokens).trimmed();
    if let Some(shape) = parse_tap_type_choice_tokens(clause.tokens()) {
        let before = trim_commas(shape.before_tokens).to_vec();
        let after = trim_commas(shape.after_tokens).to_vec();
        let mut cleaned = Vec::with_capacity(before.len() + after.len());
        cleaned.extend(before);
        cleaned.extend(after);
        return (cleaned, true);
    }

    (clause.trim(), tap_tokens_mention_chosen_type(tokens))
}
