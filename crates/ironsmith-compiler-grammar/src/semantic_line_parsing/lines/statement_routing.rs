use super::*;

pub(super) fn statement_group_should_parse_as_effects_first(tokens: &[OwnedLexToken]) -> bool {
    if matches!(
        crate::keyword_static::parse_double_counters_replacement_line(tokens,),
        Ok(Some(_))
    ) {
        return false;
    }
    if linked_statement_should_stay_grouped(tokens) {
        return true;
    }
    if crate::grammar::effects::followup_shapes::parse_counter_linked_land_subtype_followup(tokens)
        .is_some()
    {
        return true;
    }
    if crate::grammar::effects::parse_persistent_no_maximum_hand_size_player_lexed(tokens).is_some()
    {
        return true;
    }
    if matches!(
        classify_statement_line_family_lexed(tokens),
        Some(StatementLineFamily::Vote)
    ) {
        return true;
    }

    if crate::grammar::effects::clause_pattern_shapes::parse_keyword_mechanic_tokens(tokens)
        .is_some()
    {
        return true;
    }

    semantic_grammar::parse_statement_effect_preference_tokens(tokens).is_some()
}
