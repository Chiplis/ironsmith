//! A terminal spell destination replacement and its ordinary continuation.
use super::*;

fn phrase(tokens: &[OwnedLexToken], cursor: &mut usize, words: &[&str]) -> bool {
    let start = *cursor;
    for word in words {
        while tokens.get(*cursor).is_some_and(|token| token.is_comma()) {
            *cursor += 1;
        }
        if !tokens.get(*cursor).is_some_and(|token| token.is_word(word)) {
            *cursor = start;
            return false;
        }
        *cursor += 1;
    }
    true
}

pub(super) fn parse_spell_resolution_destination_program_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let mut cursor = 0;
    if !phrase(
        tokens,
        &mut cursor,
        &["if", "this", "spell", "was", "cast", "from", "your"],
    ) {
        return Ok(None);
    }
    let origin_zone = if phrase(tokens, &mut cursor, &["hand"]) {
        Zone::Hand
    } else if phrase(tokens, &mut cursor, &["graveyard"]) {
        Zone::Graveyard
    } else {
        return Ok(None);
    };
    if !phrase(
        tokens,
        &mut cursor,
        &[
            "instead",
            "of",
            "putting",
            "it",
            "into",
            "your",
            "graveyard",
            "as",
            "it",
            "resolves",
            "exile",
            "it",
        ],
    ) {
        return Ok(None);
    }
    let mut multiplicity =
        ironsmith_core::static_ability_model::SpellResolutionProgramMultiplicity::PerInstance;
    if tokens
        .get(cursor)
        .is_some_and(|token| token.is_word("once"))
    {
        if !phrase(
            tokens,
            &mut cursor,
            &["once", "for", "the", "resolution", "recipe", "named"],
        ) || !tokens.get(cursor).is_some_and(|token| {
            matches!(
                token.kind,
                crate::lexer::TokenKind::Quote | crate::lexer::TokenKind::Apostrophe
            )
        }) {
            return Err(CardTextError::ParseError(
                "shared resolution recipe requires a quoted declared key".into(),
            ));
        }
        let delimiter = tokens[cursor].kind;
        cursor += 1;
        let start = cursor;
        // The shared lexer deliberately keeps a trailing apostrophe in a
        // Word (possessive names such as players'). Within this explicitly
        // opened literal slot only, that final character closes the key.
        // Do not split possessives or change quote ownership in its callback.
        let mut word_closes_literal = false;
        while let Some(token) = tokens.get(cursor) {
            if token.kind == delimiter { break; }
            if delimiter == crate::lexer::TokenKind::Apostrophe
                && token.kind == crate::lexer::TokenKind::Word
                && token.slice.ends_with(['\'', '’', '‘'])
            {
                word_closes_literal = true;
                break;
            }
            cursor += 1;
        }
        let end = cursor + usize::from(word_closes_literal);
        if end == start || cursor == tokens.len() {
            return Err(CardTextError::ParseError(
                "shared resolution recipe key is empty or unclosed".into(),
            ));
        }
        let mut key = crate::lexer::render_token_slice(&tokens[start..end]);
        if word_closes_literal { key.pop(); }
        let key = key.trim().to_ascii_lowercase();
        if key.len() > 96
            || !key
                .chars()
                .next()
                .is_some_and(|character| character.is_ascii_alphabetic())
            || !key.chars().all(|character| {
                character.is_ascii_alphanumeric() || character == ' ' || character == '-'
            })
        {
            return Err(CardTextError::ParseError(
                "shared resolution recipe key must be plain authored words".into(),
            ));
        }
        cursor += 1;
        multiplicity=ironsmith_core::static_ability_model::SpellResolutionProgramMultiplicity::OncePerDeclaredKey{key};
    }
    if !phrase(tokens, &mut cursor, &["and"]) {
        if matches!(multiplicity, ironsmith_core::static_ability_model::SpellResolutionProgramMultiplicity::OncePerDeclaredKey { .. }) {
            return Err(CardTextError::ParseError(
                "shared resolution recipe key must be followed by its complete continuation".into(),
            ));
        }
        return Ok(None);
    }
    let body = &tokens[cursor..];
    if body.is_empty() {
        return Ok(None);
    }
    let effects = crate::clause_support::parse_effect_sentences_lexed(body)?;
    if effects.is_empty() {
        return Ok(None);
    }
    let free =
        ironsmith_compiler_resolve::tag_support::granted_cast_object_reference_imports(&effects);
    let receipt = crate::tag::declared_key(ironsmith_core::SOURCE_EXILED_THIS_RESOLUTION_TAG);
    let mut scope = vec![
        crate::cards::builders::EffectAst::subject_verb_filtered_tagged_objects(
            crate::target::ObjectFilter::default(),
            receipt.clone(),
            vec![receipt.clone()],
        ),
    ];
    if free.contains(&crate::tag::CompilerReferenceTag::PriorExiledCard.key()) {
        scope.push(
            crate::cards::builders::EffectAst::subject_verb_filtered_tagged_objects(
                crate::target::ObjectFilter::default(),
                crate::tag::CompilerReferenceTag::PriorExiledCard.bind(),
                vec![receipt],
            ),
        );
    }
    scope.extend(effects);
    // One lexical sequence supplies the header's exact receipt before its
    // children, including when this static ability is nested inside a grant.
    let effects = vec![crate::cards::builders::EffectAst::Sequence { effects: scope }];
    crate::word_claims::claim_tokens(&tokens[..cursor]);
    Ok(Some(
        StaticAbility::spell_resolution_destination_program(
            Some(origin_zone),
            Zone::Exile,
            ironsmith_core::ResolutionProgram::from_effects(effects),
        )
        .with_spell_resolution_program_multiplicity(multiplicity),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn destination_and_arbitrary_followup_are_independent_of_keyword_identity() {
        for body in [
            "You gain 2 life.",
            "Draw a card.",
            "Create a colorless Saga enchantment token with \"III — Copy the exiled card. You may cast the copy without paying its mana cost.\".",
        ] {
            let text = format!(
                "If this spell was cast from your hand, instead of putting it into your graveyard as it resolves, exile it and {body}"
            );
            let tokens = crate::util::tokenize_line(&text, 0);
            let parsed = parse_spell_resolution_destination_program_line(&tokens)
                .unwrap()
                .unwrap();
            assert!(matches!(
                parsed.payload,
                ironsmith_core::StaticAbilityPayload::SpellResolutionDestinationProgram {
                    origin_zone: Some(Zone::Hand),
                    destination: Zone::Exile,
                    ..
                }
            ));
        }
    }
    #[test]
    fn similar_ordinary_exile_and_cast_time_phrases_are_not_terminal_replacements() {
        for text in [
            "If this spell was cast from your hand, exile it and draw a card.",
            "As you cast this spell, exile it and draw a card.",
            "If this spell was cast from your hand, instead of putting it into your graveyard as it resolves, exile it.",
        ] {
            assert!(
                parse_spell_resolution_destination_program_line(&crate::util::tokenize_line(
                    text, 0
                ))
                .unwrap()
                .is_none(),
                "{text}"
            );
        }
    }
}

#[cfg(test)]
mod declared_recipe_tests {
    use super::*;
    #[test]
    fn authored_once_key_and_nested_quote_delimiters_are_typed_fields() {
        for (open, close) in [("\"", "\""), ("'", "'"), ("‘", "’")] {
            let text = format!(
                "If this spell was cast from your hand, instead of putting it into your graveyard as it resolves, exile it once for the resolution recipe named {open}authored recipe{close} and you gain 2 life."
            );
            let tokens = crate::util::tokenize_line(&text, 0);
            let parsed = parse_spell_resolution_destination_program_line(&tokens)
                .unwrap()
                .unwrap();
            let ironsmith_core::StaticAbilityPayload::SpellResolutionDestinationProgram {
                multiplicity,
                follow_up,
                ..
            } = parsed.payload
            else {
                panic!()
            };
            assert_eq!(multiplicity,ironsmith_core::static_ability_model::SpellResolutionProgramMultiplicity::OncePerDeclaredKey{key:"authored recipe".into()});
            assert_eq!(
                follow_up.all_effects().len(),
                1,
                "one lexical sequence survives nested generic static mapping"
            );
        }
    }
    #[test]
    fn single_word_and_multiword_keys_leave_nested_callback_quotes_owned_by_the_token() {
        for (open, close) in [("'", "'"), ("‘", "’"), ("\"", "\"")] {
            for key in ["shared", "shared recipe-2"] {
                let text = format!("If this spell was cast from your hand, instead of putting it into your graveyard as it resolves, exile it once for the resolution recipe named {open}{key}{close} and create a colorless Saga enchantment token with \"III — Copy the exiled card. You may cast the copy without paying its mana cost.\".");
                let tokens = crate::lexer::lex_line(&text, 0).unwrap();
                let original = tokens.clone();
                let ability = parse_spell_resolution_destination_program_line(&tokens).unwrap().unwrap();
                assert_eq!(tokens, original, "literal-slot ownership must not mutate lexer tokens");
                let ironsmith_core::StaticAbilityPayload::SpellResolutionDestinationProgram { multiplicity, follow_up, .. } = ability.payload else { panic!() };
                assert_eq!(multiplicity, ironsmith_core::static_ability_model::SpellResolutionProgramMultiplicity::OncePerDeclaredKey { key: key.into() });
                // SubjectVerbActionAst intentionally omits token definitions
                // and granted abilities from Debug. Inspect their typed owner.
                fn token_actions<'a>(effect: &'a crate::cards::builders::EffectAst, out: &mut Vec<&'a crate::cards::builders::TokenActionAst>) {
                    use crate::cards::builders::{EffectAst, SubjectVerbActionAst};
                    match effect {
                        EffectAst::Sequence { effects } => for child in effects { token_actions(child, out); },
                        EffectAst::SubjectVerb(verb) => if let SubjectVerbActionAst::Tokens(token) = &verb.action { out.push(token); },
                        _ => {},
                    }
                }
                let mut tokens = Vec::new();
                for effect in follow_up.all_effects() { token_actions(effect, &mut tokens); }
                let [crate::cards::builders::TokenActionAst::CreateTokenWithMods { definition, granted_abilities, .. }] = tokens.as_slice() else {
                    panic!("exactly one CreateTokenWithMods callback must survive; found {} token actions", tokens.len());
                };
                let crate::model::token_definition::TokenDefinitionSpec::Enchantment(shape) = definition else {
                    panic!("callback must create the authored enchantment blueprint: {definition:?}");
                };
                assert!(shape.subtypes.contains(&crate::types::Subtype::Saga));
                assert!(shape.token_rules.unread_rules.is_empty(), "a retained but unread chapter is not executable: {:?}", shape.token_rules);
                let mut abilities = shape.token_rules.parsed_abilities.iter().collect::<Vec<_>>();
                for grant in granted_abilities.iter() {
                    for leaf in grant.leaves() {
                        if let crate::cards::builders::GrantedAbilityAst::ParsedObjectAbility { ability, .. } = leaf { abilities.push(ability.as_ref()); }
                    }
                }
                let chapters = abilities.into_iter().filter(|ability| matches!(ability.trigger_spec.as_deref(), Some(crate::cards::builders::TriggerSpec::SagaChapter(chapters)) if chapters == &[3])).collect::<Vec<_>>();
                assert_eq!(chapters.len(), 1, "one executable chapter III owner; blueprint={definition:?}, grants={granted_abilities:?}");
                fn copy_casts(effect: &crate::cards::builders::EffectAst, optional: bool) -> usize {
                    use crate::cards::builders::{EffectAst, PermissionEffectAst, StackActionAst, SubjectVerbActionAst};
                    match effect {
                        EffectAst::Sequence { effects } => effects.iter().map(|effect| copy_casts(effect, optional)).sum(),
                        EffectAst::Permissions(PermissionEffectAst::May { effects } | PermissionEffectAst::MayByPlayer { effects, .. }) => effects.iter().map(|effect| copy_casts(effect, true)).sum(),
                        EffectAst::SubjectVerb(verb) => usize::from(optional && matches!(&verb.action, SubjectVerbActionAst::Stack(StackActionAst::CastTagged { as_copy: true, without_paying_mana_cost: true, copy_instruction_surface: Some(_), .. }))),
                        _ => 0,
                    }
                }
                let effects = chapters[0].effects_ast.as_ref().expect("chapter body must retain its typed program");
                assert_eq!(effects.iter().map(|effect| copy_casts(effect, false)).sum::<usize>(), 1, "the complete optional copy-and-free-cast program must survive: {effects:?}");
            }
        }
    }

    #[test]
    fn malformed_declared_keys_do_not_fall_back_to_unkeyed_programs() {
        for suffix in [
            "once for the resolution recipe named authored recipe and draw a card.",
            "once for the resolution recipe named \"\" and draw a card.",
            "once for the resolution recipe named '' and draw a card.",
            "once for the resolution recipe named 'unclosed and draw a card.",
            "once for the resolution recipe named 'key' ' and draw a card.",
        ] {
            let text = format!(
                "If this spell was cast from your hand, instead of putting it into your graveyard as it resolves, exile it {suffix}"
            );
            assert!(
                parse_spell_resolution_destination_program_line(&crate::util::tokenize_line(
                    &text, 0
                ))
                .is_err()
            );
        }
    }
}
