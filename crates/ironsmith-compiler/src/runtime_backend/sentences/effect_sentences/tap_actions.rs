use super::super::super::lex_patterns::{LexCaptureKind, LexCaptureRole, LexPattern};
use super::super::super::lexer::{
    LexedClause, token_word_refs, word_slice_contains_any_phrase, word_slice_eq,
};
use super::*;

const THEN_RETURN_WORDS: &[&str] = &["then", "return"];
const OR_UNTAP_PREFIX: &[&str] = &["or", "untap"];
const OR_UNTAP_ALL_PREFIX: &[&str] = &["or", "untap", "all"];
const CHOSEN_TYPE_MARKER_PHRASES: &[&[&str]] = &[&["chosen", "type"], &["that", "type"]];
const TAP_CONTROL_ACTION_WORDS: &[&str] = &["control", "controls"];
const TARGET_PLAYER_CONTROL_SUBJECT_PHRASES: &[&[&str]] = &[&["target", "player"]];
const THAT_PLAYER_CONTROL_SUBJECT_PHRASES: &[&[&str]] =
    &[&["that", "player"], &["that", "players"]];

const TYPE_CHOICE_QUALIFIER_PHRASES: &[&[&str]] = &[
    &["of", "the", "chosen", "type"],
    &["of", "chosen", "type"],
    &["of", "that", "type"],
    &["that", "type"],
];
const TYPE_CHOICE_REFERENCE_PATTERN: LexPattern<'static> = LexPattern::new(&[
    LexPattern::object(
        "filter_before_type_choice",
        LexCaptureKind::UntilAnyPhrase(TYPE_CHOICE_QUALIFIER_PHRASES),
    ),
    LexPattern::modifier(
        "type_choice",
        LexCaptureKind::OneOfPhrase(TYPE_CHOICE_QUALIFIER_PHRASES),
    ),
    LexPattern::tail("filter_after_type_choice", LexCaptureKind::Rest),
]);

fn word_is_all_or_each(word: &str) -> bool {
    matches!(word, "all" | "each")
}

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

pub(crate) fn parse_tap(tokens: &[OwnedLexToken]) -> Result<EffectAst, CardTextError> {
    let clause = LexedClause::new(tokens);
    if clause.is_empty() {
        return Err(CardTextError::ParseError(
            "tap clause missing target".to_string(),
        ));
    }
    if let Some(effect) = parse_tap_or_untap_all(tokens)? {
        return Ok(effect);
    }
    if clause.first_word().is_some_and(word_is_all_or_each) {
        let filter_clause = clause
            .after_words(1)
            .unwrap_or_else(|| clause.from(clause.len()));
        let filter = parse_object_filter(filter_clause.tokens(), false)?;
        return Ok(EffectAst::subject_verb_tap_all(filter));
    }
    if let Some(then_idx) = tokens
        .windows(2)
        .position(|window| word_slice_eq(&token_word_refs(window), THEN_RETURN_WORDS))
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

fn parse_tap_or_untap_all(tokens: &[OwnedLexToken]) -> Result<Option<EffectAst>, CardTextError> {
    let clause = LexedClause::new(tokens);
    if !clause.first_word().is_some_and(word_is_all_or_each) {
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

    let left_clause = LexedClause::new(&left_tokens);
    let right_clause = LexedClause::new(&right_tokens);
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
    if clause_contains_control_relation(left_clause, TARGET_PLAYER_CONTROL_SUBJECT_PHRASES) {
        tap_filter.controller = Some(PlayerFilter::target_player());
    }
    if clause_contains_control_relation(right_clause, THAT_PLAYER_CONTROL_SUBJECT_PHRASES) {
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

fn clause_contains_control_relation(
    clause: LexedClause<'_>,
    subject_phrases: &'static [&'static [&'static str]],
) -> bool {
    let atoms = [
        LexPattern::subject("controller", LexCaptureKind::OneOfPhrase(subject_phrases)),
        LexPattern::action("action", LexCaptureKind::OneOf(TAP_CONTROL_ACTION_WORDS)),
    ];
    LexPattern::new(&atoms).find_in_clause(clause).is_some()
}

fn analyze_tap_type_choice_reference(tokens: &[OwnedLexToken]) -> (Vec<OwnedLexToken>, bool) {
    let clause = LexedClause::new(tokens).trimmed();
    if let Some(matched) = TYPE_CHOICE_REFERENCE_PATTERN.match_clause(clause) {
        let before = matched
            .capture_clause_by_role(LexCaptureRole::Object, clause)
            .map(|clause| trim_commas(clause.tokens()).to_vec())
            .unwrap_or_default();
        let after = matched
            .capture_clause_by_role(LexCaptureRole::Tail, clause)
            .map(|clause| trim_commas(clause.tokens()).to_vec())
            .unwrap_or_default();
        let mut cleaned = Vec::with_capacity(before.len() + after.len());
        cleaned.extend(before);
        cleaned.extend(after);
        return (cleaned, true);
    }

    (
        clause.trim(),
        word_slice_contains_any_phrase(&token_word_refs(tokens), CHOSEN_TYPE_MARKER_PHRASES),
    )
}
