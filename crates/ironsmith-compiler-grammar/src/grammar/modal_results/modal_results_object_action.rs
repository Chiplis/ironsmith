use super::*;

pub fn parse_if_result_predicate_tokens(tokens: &[OwnedLexToken]) -> Option<IfResultPredicate> {
    let normalized = normalized_word_tokens(tokens);
    match parse_modal_result_shape(&normalized)? {
        ModalResultShape::ThisWay {
            subject: ModalResultSubject::If | ModalResultSubject::When,
            negated: false,
        }
        | ModalResultShape::ExactNegated {
            subject: ModalResultSubject::If | ModalResultSubject::When,
        } => Some(IfResultPredicate::Did),
        ModalResultShape::ThisWay {
            subject: ModalResultSubject::If | ModalResultSubject::When,
            negated: true,
        } => Some(IfResultPredicate::DidNot),
        _ => None,
    }
}

pub fn parse_if_result_predicate_lexed_tokens(
    tokens: &[OwnedLexToken],
) -> Option<IfResultPredicate> {
    let normalized = normalized_word_tokens(tokens);
    let shape = parse_modal_result_shape(&normalized);
    let word_count = normalized.len();
    if (starts_with_phrase(&normalized, &["you", "win"])
        || starts_with_phrase(&normalized, &["you", "won"]))
        && (word_count == 2 || has_phrase(&normalized, &["clash"]))
    {
        return Some(IfResultPredicate::Value(
            crate::effect::Comparison::GreaterThan(0),
        ));
    }
    let direct_surface = parse_direct_prior_effect_result_surface(tokens);
    // A passive, unfiltered negated result such as "no counters were removed
    // this way" asks whether the antecedent action changed anything at all.
    // Preserve that negation as the ordinary executable DidNot predicate.
    // Qualified object-result negatives need a filter-aware negated model and
    // deliberately do not enter this action-only equivalence.
    let passive_no_result = normalized.first().is_some_and(|token| token.is_word("no"))
        && ends_with_phrase(&normalized, &["this", "way"]);
    let explicit_negated_result = matches!(
        shape,
        Some(ModalResultShape::ThisWay {
            subject: ModalResultSubject::If | ModalResultSubject::When,
            negated: true,
        })
    );
    if (passive_no_result || explicit_negated_result)
        && direct_surface.as_ref().is_some_and(|surface| {
            surface.actor == PriorEffectResultActor::Passive
                && surface.quantifier == PriorEffectResultQuantifier::ActionOnly
                && surface.filter == crate::target::ObjectFilter::default()
                && surface.required_count.is_none()
                && surface.shared_characteristic.is_none()
        })
    {
        return Some(IfResultPredicate::DidNot);
    }
    if let Some(surface) =
        direct_surface.or_else(|| parse_typed_prior_effect_result_surface(tokens))
    {
        return Some(IfResultPredicate::PriorEffectResult(surface));
    }
    let words = normalized
        .iter()
        .map(OwnedLexToken::parser_text)
        .collect::<Vec<_>>();

    if matches_phrase(&normalized, &["you", "do"])
        || matches_phrase(&normalized, &["they", "do"])
        || matches_phrase(&normalized, &["player", "do"])
        || matches_phrase(&normalized, &["player", "does"])
        || matches_phrase(&normalized, &["players", "do"])
        || matches_phrase(&normalized, &["players", "does"])
        || matches_phrase(&normalized, &["that", "player", "do"])
        || matches_phrase(&normalized, &["that", "player", "does"])
        || matches_phrase(&normalized, &["first", "player", "do"])
        || matches_phrase(&normalized, &["first", "player", "does"])
        || matches_phrase(&normalized, &["it", "connive", "this", "way"])
        || matches_phrase(&normalized, &["it", "connives", "this", "way"])
    {
        return Some(IfResultPredicate::Did);
    }
    if matches_phrase(
        &normalized,
        &["you", "pay", "this", "cost", "one", "or", "more", "times"],
    ) {
        return Some(IfResultPredicate::Value(
            crate::effect::Comparison::GreaterThan(0),
        ));
    }
    if word_count == 3
        && (starts_with_phrase(&normalized, &["result", "is"])
            || starts_with_phrase(&normalized, &["result", "was"]))
        && let Ok(value) = leaf::parse_number_i32_complete(normalized[2].parser_text())
    {
        return Some(IfResultPredicate::Value(crate::effect::Comparison::Equal(
            value,
        )));
    }
    if (starts_with_phrase(&normalized, &["you", "win"])
        || starts_with_phrase(&normalized, &["you", "won"]))
        && has_phrase(&normalized, &["flip"])
    {
        return Some(IfResultPredicate::Did);
    }
    if starts_with_phrase(&normalized, &["you", "searched"])
        && ends_with_phrase(&normalized, &["this", "way"])
    {
        return Some(IfResultPredicate::Did);
    }
    if (starts_with_phrase(&normalized, &["you", "reveal"])
        || starts_with_phrase(&normalized, &["you", "revealed"])
        || starts_with_phrase(&normalized, &["they", "reveal"])
        || starts_with_phrase(&normalized, &["they", "revealed"])
        || starts_with_phrase(&normalized, &["that", "player", "reveals"])
        || starts_with_phrase(&normalized, &["that", "player", "revealed"]))
        && ends_with_phrase(&normalized, &["this", "way"])
    {
        return Some(IfResultPredicate::Did);
    }
    if primitives::parse_all(
        &normalized,
        parse_searched_library_result,
        "searched-library-result",
    )
    .is_ok()
    {
        return Some(IfResultPredicate::SearchedLibrary);
    }
    if matches!(
        shape,
        Some(ModalResultShape::ThisWay {
            subject: ModalResultSubject::You | ModalResultSubject::They,
            negated: false,
        })
    ) {
        return Some(IfResultPredicate::Did);
    }
    if matches_phrase(&normalized, &["no", "one", "do"])
        || matches_phrase(&normalized, &["no", "one", "does"])
    {
        return Some(IfResultPredicate::DidNot);
    }
    if matches_phrase(
        &normalized,
        &["player", "is", "dealt", "damage", "this", "way"],
    ) {
        return Some(IfResultPredicate::DealtDamageToPlayer);
    }
    // A typed object affected by the immediately preceding zone move is a
    // reflexive-result predicate, not a new trigger subject.  This covers
    // surfaces such as "When a creature is put onto the battlefield this
    // way" while retaining the affected object for the follow-up effects.
    let card_type_idx = usize::from(
        words
            .first()
            .is_some_and(|word| matches!(*word, "a" | "an" | "the")),
    );
    let affected_card_type = words.get(card_type_idx).and_then(|word| match *word {
        "artifact" => Some(crate::types::CardType::Artifact),
        "battle" => Some(crate::types::CardType::Battle),
        "creature" => Some(crate::types::CardType::Creature),
        "enchantment" => Some(crate::types::CardType::Enchantment),
        "land" => Some(crate::types::CardType::Land),
        "planeswalker" => Some(crate::types::CardType::Planeswalker),
        _ => None,
    });
    if let Some(card_type) = affected_card_type
        && words
            .get(card_type_idx + 1)
            .is_some_and(|word| matches!(*word, "is" | "was"))
        && words
            .get(card_type_idx + 2)
            .is_some_and(|word| matches!(*word, "put" | "moved" | "returned"))
        && ends_with_phrase(&normalized, &["this", "way"])
    {
        return Some(IfResultPredicate::AffectedObjectMatchesCardType {
            card_type,
            negated: false,
        });
    }
    let one_or_more_result = word_count >= 6
        && starts_with_phrase(&normalized, &["one", "or", "more"])
        && ends_with_phrase(&normalized, &["this", "way"])
        && words.get(3).is_some_and(|word| {
            matches!(
                *word,
                "card" | "cards" | "creature" | "creatures" | "permanent" | "permanents"
            )
        })
        && if words
            .get(4)
            .is_some_and(|word| matches!(*word, "is" | "are"))
        {
            words.get(5).is_some_and(|word| {
                matches!(
                    *word,
                    "remove"
                        | "removed"
                        | "sacrifice"
                        | "sacrificed"
                        | "discard"
                        | "discarded"
                        | "exile"
                        | "exiled"
                        | "mill"
                        | "milled"
                )
            })
        } else {
            words.get(4).is_some_and(|word| {
                matches!(
                    *word,
                    "remove"
                        | "removed"
                        | "sacrifice"
                        | "sacrificed"
                        | "discard"
                        | "discarded"
                        | "exile"
                        | "exiled"
                        | "mill"
                        | "milled"
                )
            })
        };
    if one_or_more_result {
        return Some(IfResultPredicate::Did);
    }
    if word_count >= 5
        && (starts_with_phrase(&normalized, &["that", "spell"])
            || starts_with_phrase(&normalized, &["it", "spell"]))
        && has_phrase(&normalized, &["countered"])
        && ends_with_phrase(&normalized, &["this", "way"])
    {
        return Some(IfResultPredicate::Did);
    }
    if word_count >= 5
        && (starts_with_phrase(&normalized, &["that", "creature", "dies", "this", "way"])
            || starts_with_phrase(&normalized, &["that", "permanent", "dies", "this", "way"])
            || starts_with_phrase(&normalized, &["that", "card", "dies", "this", "way"])
            || starts_with_phrase(&normalized, &["it", "creature", "dies", "this", "way"])
            || starts_with_phrase(&normalized, &["it", "permanent", "dies", "this", "way"])
            || starts_with_phrase(&normalized, &["it", "card", "dies", "this", "way"]))
    {
        return Some(IfResultPredicate::DiesThisWay);
    }
    if word_count >= 8
        && (starts_with_phrase(
            &normalized,
            &[
                "creature", "dealt", "damage", "this", "way", "would", "die", "this", "turn",
            ],
        ) || starts_with_phrase(
            &normalized,
            &[
                "permanent",
                "dealt",
                "damage",
                "this",
                "way",
                "would",
                "die",
                "this",
                "turn",
            ],
        ) || starts_with_phrase(
            &normalized,
            &[
                "card", "dealt", "damage", "this", "way", "would", "die", "this", "turn",
            ],
        ))
    {
        return Some(IfResultPredicate::DiesThisWay);
    }
    if (starts_with_phrase(&normalized, &["excess", "damage", "was", "dealt", "to"])
        || starts_with_phrase(&normalized, &["excess", "damage", "is", "dealt", "to"]))
        && has_phrase(&normalized, &["creature"])
        && ends_with_phrase(&normalized, &["this", "way"])
    {
        return Some(IfResultPredicate::ExcessDamageDealt);
    }
    if matches_phrase(
        &normalized,
        &["it", "deals", "excess", "damage", "this", "way"],
    ) {
        return Some(IfResultPredicate::ExcessDamageDealt);
    }
    if word_count == 6
        && (starts_with_phrase(&normalized, &["its", "power", "becomes"])
            || starts_with_phrase(&normalized, &["it", "power", "becomes"]))
        && ends_with_phrase(&normalized, &["this", "way"])
    {
        return Some(IfResultPredicate::Did);
    }
    if (starts_with_phrase(&normalized, &["you", "lose"])
        || starts_with_phrase(&normalized, &["you", "lost"]))
        && has_phrase(&normalized, &["flip"])
    {
        return Some(IfResultPredicate::DidNot);
    }
    if matches!(
        shape,
        Some(
            ModalResultShape::ThisWay {
                subject: ModalResultSubject::You
                    | ModalResultSubject::They
                    | ModalResultSubject::Player
                    | ModalResultSubject::Players
                    | ModalResultSubject::ThatPlayer,
                negated: true,
            } | ModalResultShape::ExactNegated {
                subject: ModalResultSubject::You
                    | ModalResultSubject::They
                    | ModalResultSubject::Player
                    | ModalResultSubject::Players
                    | ModalResultSubject::ThatPlayer,
            }
        )
    ) {
        return Some(IfResultPredicate::DidNot);
    }
    None
}
