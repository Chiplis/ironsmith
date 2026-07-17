use super::*;
use ironsmith_core::{CopyStaticAbilityVariants, StaticAbilityId, StaticAbilityVariantSelector};

fn selectors_for_borrowed_keyword(phrase: &str) -> Option<Vec<StaticAbilityVariantSelector>> {
    use StaticAbilityId::*;
    use StaticAbilityVariantSelector::{Any, ProtectionFromColor};

    let selectors = match phrase {
        "flying" => vec![Any(Flying)],
        "fear" => vec![Any(Fear)],
        "first strike" => vec![Any(FirstStrike)],
        "double strike" => vec![Any(DoubleStrike)],
        "deathtouch" => vec![Any(Deathtouch)],
        "haste" => vec![Any(Haste)],
        "landwalk" => vec![Any(Landwalk)],
        "lifelink" => vec![Any(Lifelink)],
        "protection" => vec![Any(Protection)],
        "protection from any color" => vec![ProtectionFromColor],
        "reach" => vec![Any(Reach)],
        "trample" => vec![Any(Trample)],
        "shroud" => vec![Any(Shroud)],
        "vigilance" => vec![Any(Vigilance)],
        "hexproof" => vec![Any(Hexproof), Any(HexproofFrom)],
        "indestructible" => vec![Any(Indestructible)],
        "menace" => vec![Any(Menace)],
        "shadow" => vec![Any(Shadow)],
        "skulk" => vec![Any(Skulk)],
        _ => return None,
    };
    Some(selectors)
}

fn words_without_exact_phrase(text: &str, phrase: &str) -> Option<Vec<String>> {
    let tokens = crate::runtime_backend::front_end::lexer::lex_line(text, 0).ok()?;
    let words = parser_token_word_refs(&tokens);
    let phrase_tokens = crate::runtime_backend::front_end::lexer::lex_line(phrase, 0).ok()?;
    let phrase_words = parser_token_word_refs(&phrase_tokens);
    let matching_starts = words
        .windows(phrase_words.len())
        .enumerate()
        .filter_map(|(index, window)| (window == phrase_words.as_slice()).then_some(index))
        .collect::<Vec<_>>();
    let [start] = matching_starts.as_slice() else {
        return None;
    };
    Some(
        words
            .iter()
            .enumerate()
            .filter(|(index, _)| *index < *start || *index >= *start + phrase_words.len())
            .map(|(_, word)| (*word).to_string())
            .collect(),
    )
}

fn condition_matching_filter(
    condition_text: &str,
    required_id: StaticAbilityId,
) -> Option<ObjectFilter> {
    // This receives the normalized condition after borrow preprocessing. Some
    // supported surfaces have already become `there is ... with <ability>`,
    // while others (such as opponent-controls conditions) remain unchanged.
    // Re-running the pre-rewrite surface parser here rejects both shapes even
    // though the semantic condition parser below has proved the exact typed
    // matching-filter form we need.
    let tokens = crate::runtime_backend::front_end::lexer::lex_line(condition_text, 0).ok()?;
    let condition = parse_static_condition_clause(&tokens).ok()?;
    let crate::ConditionExpr::CountComparison {
        count: AnthemCountExpression::MatchingFilter(mut filter),
        ..
    } = condition
    else {
        return None;
    };
    if filter.static_abilities.as_slice() != [required_id] || !filter.ability_markers.is_empty() {
        return None;
    }
    filter.static_abilities.clear();
    Some(filter)
}

fn oracle_list(items: &[String]) -> Option<String> {
    match items {
        [] => None,
        [only] => Some(only.clone()),
        [left, right] => Some(format!("{left} and {right}")),
        many => {
            let (last, leading) = many.split_last()?;
            Some(format!("{}, and {last}", leading.join(", ")))
        }
    }
}

fn parse_borrowed_static_variant_chain(
    sentences: &[&[OwnedLexToken]],
) -> Option<Vec<StaticAbilityAst>> {
    let mut common_filter = None;
    let mut condition_signature = None;
    let mut consequence_signature = None;
    let mut selectors = Vec::new();
    let mut phrases = Vec::new();

    for sentence in sentences {
        let rendered = render_token_slice(sentence);
        let crate::runtime_backend::grammar::preprocess::BorrowStaticSentenceSurface::Leading {
            condition,
            consequence,
        } = crate::runtime_backend::grammar::preprocess::parse_borrow_static_sentence_surface(
            &rendered,
        )?
        else {
            return None;
        };
        let phrase = crate::runtime_backend::grammar::preprocess::parse_borrow_ability_surface(
            &consequence,
        )?
        .phrase;
        let sentence_selectors = selectors_for_borrowed_keyword(phrase)?;
        let required_id = sentence_selectors.first()?.ability_id();
        let filter = condition_matching_filter(&condition, required_id)?;
        match &common_filter {
            Some(expected) if expected != &filter => return None,
            None => common_filter = Some(filter),
            _ => {}
        }

        let next_condition_signature = words_without_exact_phrase(&condition, phrase)?;
        match &condition_signature {
            Some(expected) if expected != &next_condition_signature => return None,
            None => condition_signature = Some(next_condition_signature),
            _ => {}
        }
        let next_consequence_signature = words_without_exact_phrase(&consequence, phrase)?;
        match &consequence_signature {
            Some(expected) if expected != &next_consequence_signature => return None,
            None => consequence_signature = Some(next_consequence_signature),
            _ => {}
        }

        for selector in sentence_selectors {
            if !selectors.contains(&selector) {
                selectors.push(selector);
            }
        }
        phrases.push(phrase.to_string());
    }

    let first_sentence = render_token_slice(sentences.first()?)
        .trim()
        .trim_end_matches('.')
        .to_string();
    let trailing = oracle_list(phrases.get(1..)?)?;
    let display = format!("{first_sentence}. The same is true for {trailing}.");
    Some(vec![StaticAbilityAst::Static(
        StaticAbility::copy_static_ability_variants(CopyStaticAbilityVariants::new(
            common_filter?,
            selectors,
            display,
        )),
    )])
}

/// Parse a multi-sentence static line as independent abilities when every
/// sentence proves that it owns a leading condition. Sentence splitting omits
/// terminal periods, so restore one before invoking the single-sentence parser.
///
/// This deliberately declines mixed or dependent sentence chains. Those still
/// belong to the compound parsers in the parent module.
pub(super) fn parse_independent_leading_conditional_static_sentence_chain(
    tokens: &[OwnedLexToken],
) -> Option<Vec<StaticAbilityAst>> {
    let sentences = split_lexed_sentences(tokens);
    if sentences.len() < 2
        || !sentences
            .iter()
            .all(|sentence| split_as_long_as_condition_prefix_lexed(sentence).is_some())
    {
        return None;
    }

    if let Some(copied_variants) = parse_borrowed_static_variant_chain(&sentences) {
        return Some(copied_variants);
    }

    let mut combined = Vec::new();
    for sentence in sentences {
        let mut terminated_sentence = sentence.to_vec();
        terminated_sentence.push(OwnedLexToken::period(TextSpan::synthetic()));

        let parsed = parse_static_ability_ast_line_lexed_single(&terminated_sentence)
            .ok()
            .flatten()?;
        if parsed.is_empty() || !parsed.iter().all(static_ability_ast_has_explicit_condition) {
            return None;
        }
        combined.extend(parsed);
    }

    Some(combined)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::AbilityKind;
    use crate::cards::builders::{CardDefinitionBuilder, CardId};
    use crate::static_abilities::{StaticAbilityId, StaticAbilityPayload};
    use ironsmith_core::StaticAbilityVariantSelector::{Any, ProtectionFromColor};

    #[test]
    fn same_is_true_keyword_ladders_lower_to_payload_preserving_variant_copy() {
        const CAIRN_SELECTORS: &[StaticAbilityVariantSelector] = &[
            Any(StaticAbilityId::Flying),
            Any(StaticAbilityId::Fear),
            Any(StaticAbilityId::FirstStrike),
            Any(StaticAbilityId::DoubleStrike),
            Any(StaticAbilityId::Deathtouch),
            Any(StaticAbilityId::Haste),
            Any(StaticAbilityId::Landwalk),
            Any(StaticAbilityId::Lifelink),
            Any(StaticAbilityId::Protection),
            Any(StaticAbilityId::Reach),
            Any(StaticAbilityId::Trample),
            Any(StaticAbilityId::Shroud),
            Any(StaticAbilityId::Vigilance),
        ];
        const DEATH_MASK_SELECTORS: &[StaticAbilityVariantSelector] = &[
            Any(StaticAbilityId::Flying),
            Any(StaticAbilityId::Fear),
            Any(StaticAbilityId::FirstStrike),
            Any(StaticAbilityId::DoubleStrike),
            Any(StaticAbilityId::Haste),
            Any(StaticAbilityId::Landwalk),
            Any(StaticAbilityId::Protection),
            Any(StaticAbilityId::Trample),
        ];
        const ESCAPED_SELECTORS: &[StaticAbilityVariantSelector] = &[
            Any(StaticAbilityId::Flying),
            Any(StaticAbilityId::FirstStrike),
            Any(StaticAbilityId::Trample),
            ProtectionFromColor,
        ];
        const RAYAMI_SELECTORS: &[StaticAbilityVariantSelector] = &[
            Any(StaticAbilityId::Flying),
            Any(StaticAbilityId::FirstStrike),
            Any(StaticAbilityId::DoubleStrike),
            Any(StaticAbilityId::Deathtouch),
            Any(StaticAbilityId::Haste),
            Any(StaticAbilityId::Hexproof),
            Any(StaticAbilityId::HexproofFrom),
            Any(StaticAbilityId::Indestructible),
            Any(StaticAbilityId::Lifelink),
            Any(StaticAbilityId::Menace),
            Any(StaticAbilityId::Protection),
            Any(StaticAbilityId::Reach),
            Any(StaticAbilityId::Trample),
            Any(StaticAbilityId::Vigilance),
        ];

        let cases = [
            (
                "Cairn Wanderer",
                "As long as a creature card with flying is in a graveyard, this creature has flying. The same is true for fear, first strike, double strike, deathtouch, haste, landwalk, lifelink, protection, reach, trample, shroud, and vigilance.",
                CAIRN_SELECTORS,
                &["Graveyard", "Creature"][..],
            ),
            (
                "Death-Mask Duplicant",
                "As long as a card exiled with this creature has flying, this creature has flying. The same is true for fear, first strike, double strike, haste, landwalk, protection, and trample.",
                DEATH_MASK_SELECTORS,
                &["Exile", "TaggedObjectConstraint"][..],
            ),
            (
                "Escaped Shapeshifter",
                "As long as an opponent controls a creature with flying not named Escaped Shapeshifter, this creature has flying. The same is true for first strike, trample, and protection from any color.",
                ESCAPED_SELECTORS,
                &["Opponent", "Escaped Shapeshifter"][..],
            ),
            (
                "Rayami, First of the Fallen",
                "As long as an exiled creature card with a blood counter on it has flying, Rayami has flying. The same is true for first strike, double strike, deathtouch, haste, hexproof, indestructible, lifelink, menace, protection, reach, trample, and vigilance.",
                RAYAMI_SELECTORS,
                &["Exile", "Blood", "Creature"][..],
            ),
        ];

        for (name, oracle, expected_selectors, expected_filter_fragments) in cases {
            let definition = CardDefinitionBuilder::new(CardId::new(), name)
                .parse_text(oracle)
                .unwrap_or_else(|error| panic!("{name} should parse: {error}"));
            let copies = definition
                .abilities
                .iter()
                .filter_map(|ability| {
                    let AbilityKind::Static(static_ability) = &ability.kind else {
                        return None;
                    };
                    let StaticAbilityPayload::CopyStaticAbilityVariants(copy) =
                        &static_ability.payload
                    else {
                        return None;
                    };
                    assert_eq!(
                        static_ability.display(),
                        oracle,
                        "{name} must retain the authored line as its rendered label"
                    );
                    Some(copy)
                })
                .collect::<Vec<_>>();

            assert_eq!(
                copies.len(),
                1,
                "{name} must lower one reusable variant-copy carrier: {copies:#?}"
            );
            let copy = copies[0];
            assert_eq!(copy.selectors.as_slice(), expected_selectors, "{name}");
            assert!(copy.filter.static_abilities.is_empty(), "{name}: {copy:#?}");
            assert_eq!(copy.display, oracle, "{name}");
            let filter_debug = format!("{:?}", copy.filter);
            for fragment in expected_filter_fragments {
                assert!(
                    filter_debug.contains(fragment),
                    "{name} must preserve {fragment:?}: {copy:#?}"
                );
            }
        }
    }

    #[test]
    fn borrowed_variant_chain_rejects_mismatched_base_filters() {
        let tokens = super::super::super::lexer::lex_line(
            "As long as a creature card with flying is in a graveyard, this creature has flying. As long as a creature card with first strike is in exile, this creature has first strike.",
            0,
        )
        .expect("mismatched chain should lex");
        let sentences = split_lexed_sentences(&tokens);
        assert!(
            parse_borrowed_static_variant_chain(&sentences).is_none(),
            "different zone constraints must not be merged into one variant-copy carrier"
        );
    }

    #[test]
    fn leading_conditional_chain_declines_an_unparseable_dependent_near_miss() {
        let tokens = super::super::super::lexer::lex_line(
            "As long as there is a creature card with flying in a graveyard, this creature has flying. As long as there is a creature card with vigilance in a graveyard, the same is true.",
            0,
        )
        .expect("near-miss line should lex");

        assert!(
            parse_independent_leading_conditional_static_sentence_chain(&tokens).is_none(),
            "a dependent consequence must not be treated as an independent static ability"
        );
    }
}
