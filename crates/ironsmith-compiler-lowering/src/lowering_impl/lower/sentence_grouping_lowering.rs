use super::*;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cards::builders::{CardDefinitionBuilder, LineAst, LineInfo, NormalizedLine};
    use crate::ids::CardId;
    use ironsmith_compiler::ir::{RewriteKeywordLineKind, RewriteSemanticDocument};
    use ironsmith_core::card::CardBuilder;

    use crate::types::CardType;
    use ironsmith_compiler::lexer::{
        lex_line, render_token_slice, split_lexed_sentences, trim_lexed_commas,
    };
    use ironsmith_compiler::parse_context::ParseContext;

    fn parse_text_to_semantic_document(
        card: CardBuilder,
        text: String,
        allow_unsupported: bool,
    ) -> Result<(RewriteSemanticDocument, crate::cards::ParseAnnotations), CardTextError> {
        let mut context =
            ironsmith_compiler::parse_context_for_builder(&card, &text, allow_unsupported);
        ironsmith_compiler::compiler_pipeline::parse_text_to_semantic_document_with_context(
            &mut context,
            card,
            text,
        )
    }

    #[test]
    fn rewrite_exert_followup_subject_rewrite_uses_existing_tokens() {
        let tokens = lex_line("he can't block this turn.", 0)
            .expect("rewrite lexer should classify exert followup");

        let normalized = normalize_exert_followup_source_reference_tokens(
            &crate::front_end::lexer::synthetic_phrase_tokens("Champion"),
            trim_lexed_commas(&tokens),
        );

        assert_eq!(
            render_token_slice(&normalized).trim(),
            "this creature can't block this turn."
        );
    }

    #[test]
    fn rewrite_divvy_suffix_trim_reuses_first_sentence_tokens() -> Result<(), CardTextError> {
        let tokens = lex_line(
            "Exile up to five target permanent cards from your graveyard and separate them into two piles.",
            0,
        )
        .expect("rewrite lexer should classify divvy exile sentence");
        let first_sentence = split_lexed_sentences(&tokens)
            .into_iter()
            .next()
            .expect("expected first sentence tokens");
        let trimmed = strip_lexed_suffix_phrase(
            first_sentence,
            &["and", "separate", "them", "into", "two", "piles"],
        )
        .expect("expected divvy pile suffix to trim");

        assert_eq!(
            render_token_slice(trimmed).trim(),
            "Exile up to five target permanent cards from your graveyard"
        );
        assert!(matches!(
            parse_single_effect_lexed(trimmed)?,
            EffectAst::SubjectVerb(crate::model::ast::SubjectVerbEffectAst {
                action: crate::model::ast::SubjectVerbActionAst::Exile { .. },
                ..
            })
        ));

        Ok(())
    }

    #[test]
    fn rewrite_triggered_normalization_keeps_explicit_intervening_if_predicate()
    -> Result<(), CardTextError> {
        let card = CardBuilder::new(CardId::new(), "Portcullis Variant")
            .card_types(vec![CardType::Artifact]);
        let (doc, _) = parse_text_to_semantic_document(
            card,
            "Whenever a creature enters, if there are two or more other creatures on the battlefield, exile that creature. Return that card to the battlefield under its owner's control when this artifact leaves the battlefield.".to_string(),
            false,
        )?;
        let normalized = document_to_normalized_card_ast(doc)?;
        let parsed = normalized
            .items
            .into_iter()
            .find_map(|item| match item {
                NormalizedCardItem::Line(line) => line.chunks.into_iter().find_map(|chunk| {
                    if let NormalizedLineChunk::Ability(parsed) = chunk {
                        Some(parsed)
                    } else {
                        None
                    }
                }),
                _ => None,
            })
            .expect("expected Portcullis-style line to normalize into a triggered ability");

        let crate::model::CompilerAbilityKindCore::Triggered(_triggered) = parsed.parsed.kind()
        else {
            panic!(
                "expected Portcullis-style line to normalize into a triggered ability, got {:?}",
                parsed.parsed.kind()
            );
        };
        let debug = format!("{:?}", parsed.prepared);
        assert!(
            matches!(
                parsed.prepared.as_ref(),
                Some(NormalizedPreparedAbility::Triggered { prepared, .. })
                    if prepared.intervening_if.is_some()
            ),
            "expected trigger predicate to survive normalization, got {debug}"
        );
        assert!(
            debug.contains("ValueComparison"),
            "expected battlefield-count predicate to survive normalization, got {debug}"
        );

        Ok(())
    }
}
