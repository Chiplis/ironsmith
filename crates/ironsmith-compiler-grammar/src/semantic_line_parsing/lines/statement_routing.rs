use super::*;

/// The shapes a statement group parses as effects before statics; every
/// recognizer answers the same question, so the table is a disjunction.
const EFFECTS_FIRST_RECOGNIZERS: &[fn(&[OwnedLexToken]) -> bool] = &[
    |tokens| linked_statement_should_stay_grouped(tokens),
    |tokens| {
        crate::grammar::effects::followup_shapes::parse_counter_linked_land_subtype_followup(tokens)
            .is_some()
    },
    |tokens| {
        crate::grammar::effects::parse_persistent_no_maximum_hand_size_player_lexed(tokens)
            .is_some()
    },
    |tokens| {
        matches!(
            classify_statement_line_family_lexed(tokens),
            Some(StatementLineFamily::Vote)
        )
    },
    |tokens| {
        crate::grammar::effects::clause_pattern_shapes::parse_keyword_mechanic_tokens(tokens)
            .is_some()
    },
    |tokens| semantic_grammar::parse_statement_effect_preference_tokens(tokens).is_some(),
];

pub(super) fn statement_group_should_parse_as_effects_first(tokens: &[OwnedLexToken]) -> bool {
    if matches!(
        crate::keyword_static::parse_double_counters_replacement_line(tokens,),
        Ok(Some(_))
    ) {
        return false;
    }
    EFFECTS_FIRST_RECOGNIZERS
        .iter()
        .any(|recognizes| recognizes(tokens))
}
