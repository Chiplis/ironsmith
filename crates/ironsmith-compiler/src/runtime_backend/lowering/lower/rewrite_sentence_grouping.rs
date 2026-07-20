use super::*;
use crate::runtime_backend::lexer::{
    lex_line, render_token_slice, split_lexed_sentences, trim_lexed_commas,
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cards::builders::{CardDefinitionBuilder, LineAst, LineInfo, NormalizedLine};
    use crate::ids::CardId;
    use crate::runtime_backend::RewriteKeywordLineKind;
    use crate::runtime_backend::pipeline::parse_text_to_semantic_document;
    use crate::types::CardType;

    #[test]
    fn rewrite_exert_followup_subject_rewrite_uses_existing_tokens() {
        let tokens = lex_line("he can't block this turn.", 0)
            .expect("rewrite lexer should classify exert followup");

        let normalized = normalize_exert_followup_source_reference_tokens(
            "Champion",
            trim_lexed_commas(&tokens),
        );

        assert_eq!(
            render_token_slice(&normalized).trim(),
            "this creature can't block this turn."
        );
    }

    #[test]
    fn rewrite_exert_keyword_lowering_reuses_token_followup_for_linked_trigger()
    -> Result<(), CardTextError> {
        let text = "you may exert champion as it attacks. when you do, he can't block this turn.";
        let tokens = lex_line(text, 0).expect("rewrite lexer should classify exert keyword line");

        let parsed = parse_keyword_line_for_test(
            LineInfo {
                line_index: 0,
                display_line_index: 0,
                raw_line: text.to_string(),
                source_tokens: tokens.clone(),
                normalized: NormalizedLine {
                    original: text.to_string(),
                    normalized: text.to_string(),
                    char_map: Vec::new(),
                },
                semantic_facts: Default::default(),
            },
            text,
            &tokens,
            RewriteKeywordLineKind::ExertAttack,
        )?;

        match parsed {
            LineAst::StaticAbility(ability) => {
                let debug = format!("{ability:?}");
                assert!(
                    debug.contains("exert attack") || debug.contains("ExertAttack"),
                    "{debug}"
                );
            }
            other => panic!("expected exert static ability, got {other:?}"),
        }

        Ok(())
    }

    #[test]
    fn rewrite_exert_keyword_lowering_uses_parse_tokens_when_text_is_stale()
    -> Result<(), CardTextError> {
        let token_text = "if this creature hasn't been exerted this turn, you may exert champion as it attacks. when you do, he can't block this turn.";
        let tokens =
            lex_line(token_text, 0).expect("rewrite lexer should classify exert keyword line");

        let parsed = parse_keyword_line_for_test(
            LineInfo {
                line_index: 0,
                display_line_index: 0,
                raw_line: "placeholder exert text".to_string(),
                source_tokens: tokens.clone(),
                normalized: NormalizedLine {
                    original: "placeholder exert text".to_string(),
                    normalized: "placeholder exert text".to_string(),
                    char_map: Vec::new(),
                },
                semantic_facts: Default::default(),
            },
            "placeholder exert text",
            &tokens,
            RewriteKeywordLineKind::ExertAttack,
        )?;

        match parsed {
            LineAst::StaticAbility(ability) => {
                let debug = format!("{ability:?}");
                assert!(
                    debug.contains("exert attack") || debug.contains("ExertAttack"),
                    "{debug}"
                );
                assert!(
                    debug.contains("only_if_not_exerted_this_turn: true") || debug.contains("true"),
                    "{debug}"
                );
            }
            other => panic!("expected exert static ability, got {other:?}"),
        }

        Ok(())
    }

    #[test]
    fn rewrite_special_triggered_burning_rune_demon_accepts_stored_parse_tokens()
    -> Result<(), CardTextError> {
        let full_text = "when this creature enters, you may search your library for exactly two cards not named burning rune demon that have different names. if you do, reveal those cards. an opponent chooses one of them. put the chosen card into your hand and the other into your graveyard, then shuffle.";
        let trigger_text = "when this creature enters";
        let effect_text = "you may search your library for exactly two cards not named burning rune demon that have different names. if you do, reveal those cards. an opponent chooses one of them. put the chosen card into your hand and the other into your graveyard, then shuffle.";
        let full_tokens =
            lex_line(full_text, 0).expect("rewrite lexer should classify burning rune demon line");
        let trigger_tokens = lex_line(trigger_text, 0)
            .expect("rewrite lexer should classify burning rune demon trigger");
        let effect_tokens = lex_line(effect_text, 0)
            .expect("rewrite lexer should classify burning rune demon effect");

        let parsed = parse_triggered_line(
            LineInfo {
                line_index: 0,
                display_line_index: 0,
                raw_line: full_text.to_string(),
                source_tokens: full_tokens.clone(),
                normalized: NormalizedLine {
                    original: full_text.to_string(),
                    normalized: full_text.to_string(),
                    char_map: Vec::new(),
                },
                semantic_facts: Default::default(),
            },
            full_text,
            &full_tokens,
            &trigger_tokens,
            &effect_tokens,
            None,
            None,
            None,
            None,
        )?;

        let debug = format!("{parsed:?}");
        assert!(debug.contains("Triggered"), "{debug}");
        assert!(debug.contains("divvy_source"), "{debug}");
        assert!(debug.contains("divvy_chosen"), "{debug}");
        assert!(debug.contains("ShuffleLibrary"), "{debug}");

        Ok(())
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
            EffectAst::SubjectVerb(crate::runtime_backend::ast::SubjectVerbEffectAst {
                action: crate::runtime_backend::ast::SubjectVerbActionAst::Exile { .. },
                ..
            })
        ));

        Ok(())
    }

    #[test]
    fn rewrite_triggered_normalization_keeps_explicit_intervening_if_predicate()
    -> Result<(), CardTextError> {
        let builder = CardDefinitionBuilder::new(CardId::new(), "Portcullis Variant")
            .card_types(vec![CardType::Artifact]);
        let (doc, _) = parse_text_to_semantic_document(
            builder,
            "Whenever a creature enters, if there are two or more other creatures on the battlefield, exile that creature. Return that card to the battlefield under its owner's control when this artifact leaves the battlefield.".to_string(),
            false,
        )?;
        let normalized = rewrite_document_to_normalized_card_ast(doc)?;
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

        let AbilityKind::Triggered(_triggered) = parsed.parsed.kind() else {
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
