use super::super::super::lexer::LexedClause;
use super::*;
use crate::runtime_backend::effect_sentences::clause_pattern_helpers::{ClauseShape, clause_shape};

const THEN_RETURN_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["then", "return"]);
const ALL_OR_EACH_PATTERN: ClauseShape<'static> = clause_shape!(exact_any & [&["all"], &["each"]]);
const OR_UNTAP_PREFIX: &[&str] = &["or", "untap"];
const OR_UNTAP_ALL_PREFIX: &[&str] = &["or", "untap", "all"];
const CHOSEN_TYPE_MARKER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_any_phrases & [&[&["chosen", "type"], &["that", "type"],]]);
const TARGET_PLAYER_CONTROLS_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_phrases & [&["target", "player", "controls"]]);
const THAT_PLAYER_CONTROLS_PATTERN: ClauseShape<'static> = clause_shape!(
    contains_any_phrases
        & [&[
            &["that", "player", "controls"],
            &["that", "players", "control"],
        ]]
);

const TYPE_CHOICE_QUALIFIER_PHRASES: &[&[&str]] = &[
    &["of", "the", "chosen", "type"],
    &["of", "chosen", "type"],
    &["of", "that", "type"],
    &["that", "type"],
];

pub(crate) fn collapse_leading_signed_pt_modifier_tokens(
    tokens: &[OwnedLexToken],
) -> Option<Vec<OwnedLexToken>> {
    let sign = match tokens.first()?.kind {
        crate::runtime_backend::lexer::TokenKind::Dash => "-",
        crate::runtime_backend::lexer::TokenKind::Plus => "+",
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

pub(crate) fn parse_tap(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let clause = LexedClause::new(tokens);
    if clause.is_empty() {
        return Err(CardTextError::ParseError(
            "tap clause missing target".to_string(),
        ));
    }
    if let Some(effect) = parse_tap_or_untap_all(tokens, subject)? {
        return Ok(effect);
    }
    if clause
        .first_word()
        .is_some_and(|word| ALL_OR_EACH_PATTERN.matches_word(word))
    {
        let filter_clause = clause
            .after_words(1)
            .unwrap_or_else(|| clause.from(clause.len()));
        let mut filter = parse_object_filter(filter_clause.tokens(), false)?;
        super::super::bind_iterated_player_pronouns_to_subject(&mut filter, subject);
        return Ok(EffectAst::subject_verb_tap_all(filter));
    }
    if let Some(then_idx) = tokens
        .windows(2)
        .position(|window| THEN_RETURN_PATTERN.matches(LexedClause::new(window)))
    {
        let tap_tokens = trim_commas(&tokens[..then_idx]);
        let return_tokens = trim_commas(&tokens[then_idx + 1..]);
        if !tap_tokens.is_empty() && !return_tokens.is_empty() {
            let target = parse_target_phrase(&tap_tokens)?;
            let return_effect = parse_return(&return_tokens)?;
            return Ok(EffectAst::Sequence {
                effects: vec![EffectAst::subject_verb_tap(target), return_effect],
            });
        }
    }
    // Handle "tap or untap <target>" as a choice between tapping and untapping.
    if let Some(target_clause) = clause.strip_prefix_clause(OR_UNTAP_PREFIX) {
        let target = parse_target_phrase(target_clause.tokens())?;
        return Ok(EffectAst::subject_verb_tap_or_untap(target.clone()));
    }
    let target = parse_target_phrase(tokens)?;
    Ok(EffectAst::subject_verb_tap(target))
}

fn parse_tap_or_untap_all(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<Option<EffectAst>, CardTextError> {
    let clause = LexedClause::new(tokens);
    if !clause
        .first_word()
        .is_some_and(|word| ALL_OR_EACH_PATTERN.matches_word(word))
    {
        return Ok(None);
    }
    let Some(after_quantifier) = clause.from_word(1) else {
        return Ok(None);
    };
    let Some((left_clause, right_with_separator)) =
        after_quantifier.split_once_before_phrase(OR_UNTAP_ALL_PREFIX)
    else {
        return Ok(None);
    };
    let Some(right_clause) = right_with_separator.strip_prefix_clause(OR_UNTAP_ALL_PREFIX) else {
        return Ok(None);
    };
    let left_clause = left_clause.trimmed();
    let right_clause = right_clause.trimmed();
    if left_clause.is_empty() || right_clause.is_empty() {
        return Ok(None);
    }

    let left_tokens = left_clause.tokens().to_vec();
    let right_tokens = right_clause.tokens().to_vec();

    let analyze_type_choice_reference = |tokens: &[OwnedLexToken]| {
        let clause = LexedClause::new(tokens);
        let stripped = clause.without_any_phrase_trimmed(TYPE_CHOICE_QUALIFIER_PHRASES);
        let mentions = stripped.is_some() || CHOSEN_TYPE_MARKER_PATTERN.matches(clause);
        let tokens = if let Some((_, tokens)) = stripped {
            tokens
        } else {
            clause.trim()
        };
        (tokens, mentions)
    };

    let left_clause = LexedClause::new(&left_tokens);
    let right_clause = LexedClause::new(&right_tokens);
    let (cleaned_left, left_mentions_chosen_type) = analyze_type_choice_reference(&left_tokens);
    let (cleaned_right, right_mentions_chosen_type) = analyze_type_choice_reference(&right_tokens);

    let mut tap_filter = parse_object_filter(&cleaned_left, false)?;
    let mut untap_filter = parse_object_filter(&cleaned_right, false)?;
    super::super::bind_iterated_player_pronouns_to_subject(&mut tap_filter, subject);
    super::super::bind_iterated_player_pronouns_to_subject(&mut untap_filter, subject);
    if left_mentions_chosen_type {
        tap_filter.chosen_creature_type = true;
    }
    if right_mentions_chosen_type {
        untap_filter.chosen_creature_type = true;
    }
    if TARGET_PLAYER_CONTROLS_PATTERN.matches(left_clause) {
        tap_filter.controller = Some(PlayerFilter::target_player());
    }
    if THAT_PLAYER_CONTROLS_PATTERN.matches(right_clause) {
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
