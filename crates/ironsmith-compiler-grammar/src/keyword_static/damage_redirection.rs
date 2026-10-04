use super::*;

pub(crate) fn redirection_recipient_filters(tokens: &[OwnedLexToken]) -> Result<(Option<PlayerFilter>, Option<ObjectFilter>), CardTextError> {
    damage_prevention::prevention_recipient_filters(tokens)
}

pub fn parse_scoped_damage_redirection_line(tokens: &[OwnedLexToken]) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(shape) = keyword_static_lines::parse_static_damage_redirection(tokens) else { return Ok(None); };
    if shape.untapped_source.is_some_and(|source| !is_source_reference_words(&parser_token_word_refs(source))) { return Ok(None); }
    let destination_words = parser_token_word_refs(shape.destination);
    let recipient_words = parser_token_word_refs(shape.recipient);
    let destination = if is_source_reference_words(&destination_words) {
        // The legacy source-and-other-permanents rule remains the sole owner of
        // its existing shape, preserving its representation and rule identity.
        if static_mid_facts::parse_redirect_damage_to_source_fact(tokens).is_some() { return Ok(None); }
        ironsmith_core::StaticDamageRedirectDestination::Source
    } else if matches!(destination_words.as_slice(), ["enchanted", "creature"] | ["equipped", "creature"]) {
        ironsmith_core::StaticDamageRedirectDestination::AttachedPermanent
    } else if destination_words == ["its", "controller"] && recipient_words == ["enchanted", "creature"] {
        ironsmith_core::StaticDamageRedirectDestination::DamagedPermanentController
    } else { return Ok(None); };
    let (target_player_filter, target_object_filter) = damage_prevention::prevention_recipient_filters(shape.recipient)?;
    let source_filter = match shape.source {
        Some(source) if !source.is_empty() => parse_object_filter_lexed(source, false)?,
        Some(_) => return Ok(None),
        None => ObjectFilter::default(),
    };
    Ok(Some(StaticAbility::redirect_matching_damage(ironsmith_core::StaticDamageRedirectionSpec {
        source_filter, target_player_filter, target_object_filter, combat_only: shape.combat_only,
        source_must_be_untapped: shape.untapped_source.is_some(), destination,
        display: render_token_slice(tokens),
    })))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex_line;
    #[test]
    fn persistent_redirect_retains_attachment_combat_and_untapped_scope() {
        for (text, destination, combat, untapped) in [
            ("All damage that would be dealt to you is dealt to this creature instead.", ironsmith_core::StaticDamageRedirectDestination::Source, false, false),
            ("All damage that would be dealt to you is dealt to enchanted creature instead.", ironsmith_core::StaticDamageRedirectDestination::AttachedPermanent, false, false),
            ("All damage that would be dealt to enchanted creature is dealt to its controller instead.", ironsmith_core::StaticDamageRedirectDestination::DamagedPermanentController, false, false),
            ("As long as this creature is untapped, all combat damage that would be dealt to you by unblocked creatures is dealt to this creature instead.", ironsmith_core::StaticDamageRedirectDestination::Source, true, true),
        ] {
            let ability = parse_scoped_damage_redirection_line(&lex_line(text, 0).unwrap()).unwrap().unwrap();
            let ironsmith_core::StaticAbilityPayload::RedirectMatchingDamage(spec) = ability.payload else { panic!("typed redirection required"); };
            assert_eq!(spec.destination, destination); assert_eq!(spec.combat_only, combat); assert_eq!(spec.source_must_be_untapped, untapped);
            if combat { assert!(spec.source_filter.unblocked); }
        }
    }
    #[test]
    fn persistent_reader_rejects_optional_timed_and_trailing_programs() {
        for text in [
            "All damage that would be dealt to you this turn is dealt to this creature instead.",
            "All damage that would be dealt to you is dealt to this creature instead. Draw a card.",
            "All damage that would be dealt to you is dealt to target creature instead.",
            "As long as enchanted creature is untapped, all damage that would be dealt to you is dealt to this creature instead.",
        ] { assert!(!matches!(parse_scoped_damage_redirection_line(&lex_line(text, 0).unwrap()), Ok(Some(_))), "{text}"); }
    }
}
