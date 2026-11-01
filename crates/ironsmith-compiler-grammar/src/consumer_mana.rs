//! Consumer-side spending requirements. Producer restrictions ("spend this
//! mana only ...") belong to the mana-producing ability and are separate.
use crate::word_primitives::WordWindows as _;
use crate::lexer::{OwnedLexToken, TokenWordView};
use ironsmith_core::mana::{ManaProducerFilter, ManaSpendingRestriction};

pub(crate) fn source_spending_rule(
    tokens: &[OwnedLexToken],
    alternative_only: bool,
) -> Option<ManaSpendingRestriction> {
    let words = TokenWordView::new(tokens).word_refs();
    let body = words.strip_prefix(&["spend", "only", "mana", "produced", "by"])?;
    let producer = if alternative_only {
        body.strip_suffix(&["to", "cast", "it", "this", "way"])?
    } else {
        body.strip_suffix(&["to", "cast", "this", "spell"])?
    };
    Some(ManaSpendingRestriction::ProducedBy(mana_producer_filter(producer)?))
}

/// The producer a "mana produced by <producers>" rule names: optional
/// supertypes ("basic", "snow", "legendary") then one plural card type or
/// subtype ("lands", "creatures", "Treasures", "Forests"). The rule judges
/// the producer's characteristics as the mana was produced.
fn mana_producer_filter(words: &[&str]) -> Option<ManaProducerFilter> {
    let (noun, modifiers) = words.split_last()?;
    let mut parts = Vec::new();
    if let Some(card_type) = crate::util::parse_card_type(noun) {
        parts.push(ManaProducerFilter::CardType(card_type));
    } else if let Some(subtype) = crate::util::parse_subtype_flexible(noun) {
        parts.push(ManaProducerFilter::Subtype(subtype));
    } else {
        return None;
    }
    for modifier in modifiers {
        if let Some(supertype) = crate::util::parse_supertype_word(modifier) {
            parts.push(ManaProducerFilter::Supertype(supertype));
        } else if let Some(card_type) = crate::util::parse_card_type(modifier) {
            parts.push(ManaProducerFilter::CardType(card_type));
        } else if let Some(subtype) = crate::util::parse_subtype_flexible(modifier) {
            parts.push(ManaProducerFilter::Subtype(subtype));
        } else {
            return None;
        }
    }
    Some(if parts.len() == 1 {
        parts.remove(0)
    } else {
        ManaProducerFilter::All(parts)
    })
}

/// The mana a per-unit spending rule admits: "mana of the chosen color",
/// "black and/or red mana", "colored mana", "colorless mana", "snow mana",
/// "mana produced by <producers>".
fn unit_spending_requirement(words: &[&str]) -> Option<ManaSpendingRestriction> {
    use ironsmith_core::color::{Color, ColorSet};
    Some(match words {
        ["mana", "of", "the", "chosen", "color"] | ["mana", "of", "chosen", "color"] => {
            ManaSpendingRestriction::SourceChosenColor
        }
        ["colored", "mana"] => {
            ManaSpendingRestriction::Colors(Color::ALL.into_iter().map(ColorSet::from_color).fold(ColorSet::new(), ColorSet::union))
        }
        ["colorless", "mana"] => ManaSpendingRestriction::Colorless,
        ["snow", "mana"] => ManaSpendingRestriction::Snow,
        ["mana", "produced", "by", producer @ ..] => {
            ManaSpendingRestriction::ProducedBy(mana_producer_filter(producer)?)
        }
        [colors @ .., "mana"] if !colors.is_empty() => {
            // "black and/or red" lexes its connective as "and or"; a list is
            // colors separated by one or more connective words.
            let mut set = ColorSet::new();
            let mut expect_color = true;
            for word in colors.iter() {
                if matches!(*word, "and" | "or" | "and/or") {
                    if set.is_empty() {
                        return None;
                    }
                    expect_color = true;
                } else if expect_color {
                    set = set.with(Color::from_name(word)?);
                    expect_color = false;
                } else {
                    return None;
                }
            }
            if expect_color {
                return None;
            }
            ManaSpendingRestriction::Colors(set)
        }
        _ => return None,
    })
}

/// "Spend only mana of the chosen color to activate this ability." (Throne
/// of Eldraine), "Spend only black mana to activate this ability.": a
/// consumer-side rule every mana unit paid for this ability's cost obeys.
pub(crate) fn activation_spending_rule(tokens: &[OwnedLexToken]) -> Option<ManaSpendingRestriction> {
    crate::word_claims::speculative_keep_if(
        || {
            let clean = crate::util::trim_edge_punctuation_tokens(tokens);
            let words = TokenWordView::new(clean).word_refs();
            let body = words
                .strip_prefix(&["spend", "only"])?
                .strip_suffix(&["to", "activate", "this", "ability"])?;
            let rule = unit_spending_requirement(body)?;
            // A complete sentence match: every word was read.
            crate::word_claims::claim_words(&words);
            Some(rule)
        },
        Option::is_some,
    )
}

pub(crate) fn spell_source_spending_ability(
    tokens: &[OwnedLexToken],
) -> Option<crate::model::CompilerStaticAbilityCore> {
    let rule = source_spending_rule(tokens, false)
        .or_else(|| x_spending_rule(tokens))
        .or_else(|| no_mana_spending_rule(tokens))?;
    Some(
        crate::model::CompilerStaticAbilityCore::spell_mana_spending_restriction(
            rule,
            crate::lexer::render_token_slice(tokens)
                .trim()
                .trim_end_matches('.'),
        ),
    )
}

/// "You can't spend mana to cast this spell." (Hogaak, Arisen Necropolis):
/// the whole cost must be paid without spending mana (convoke, delve).
pub(crate) fn no_mana_spending_rule(tokens: &[OwnedLexToken]) -> Option<ManaSpendingRestriction> {
    let words = TokenWordView::new(tokens).word_refs();
    matches!(
        words.as_slice(),
        ["you", "can't" | "cant" | "cannot", "spend", "mana", "to", "cast", "this", "spell"]
    )
    .then_some(ManaSpendingRestriction::NoMana)
}

/// "For each mana in this ability's activation cost, you may tap an untapped
/// creature you control rather than pay that mana." (Heirloom Epic): the
/// activation price admits a creature tap per generic mana. It is an
/// alternative way to pay the cost, never a resolving instruction.
pub(crate) fn is_creature_tap_payment_sentence(tokens: &[OwnedLexToken]) -> bool {
    let words = TokenWordView::new(tokens).word_refs();
    matches!(
        words.as_slice(),
        [
            "for", "each", "mana", "in", "this", "ability's" | "abilitys" | "ability",
            "activation", "cost", "you", "may", "tap", "an", "untapped", "creature", "you",
            "control", "rather", "than", "pay", "that", "mana",
        ] | [
            "for", "each", "mana", "in", "this", "ability's" | "abilitys" | "ability", "s",
            "activation", "cost", "you", "may", "tap", "an", "untapped", "creature", "you",
            "control", "rather", "than", "pay", "that", "mana",
        ]
    )
}

/// Let every mana component of an activation price be paid by tapping
/// untapped creatures the activator controls, one {1} each.
pub(crate) fn allow_creature_tap_payment(
    cost: ironsmith_core::TotalCost<crate::model::CompilerCost>,
) -> ironsmith_core::TotalCost<crate::model::CompilerCost> {
    cost.try_map(|component| Ok::<_, std::convert::Infallible>(match component {
        crate::model::CompilerCost::Mana(cost) => {
            crate::model::CompilerCost::Mana(cost.with_creature_tap_payment())
        }
        other => other,
    })).unwrap_or_else(|never| match never {})
}

/// A restriction on actual mana spent on X. It is never lowered as a colored
/// mana symbol or as a resolution effect.
pub(crate) fn x_spending_rule(tokens: &[OwnedLexToken]) -> Option<ManaSpendingRestriction> {
    use ironsmith_core::color::{Color, ColorSet};
    let words = TokenWordView::new(tokens).word_refs();
    let body = words.strip_prefix(&["spend", "only"])?;
    let split = body.word_windows(3).position(|part| part == ["mana", "on", "x"])?;
    let colors = match &body[..split] {
        ["colored"] => Color::ALL.into_iter().collect(),
        [single] => ColorSet::from_color(Color::from_name(single)?),
        [first, "and/or", second] | [first, "or", second] =>
            ColorSet::from_color(Color::from_name(first)?).with(Color::from_name(second)?),
        [first, "and", "or", second] =>
            ColorSet::from_color(Color::from_name(first)?).with(Color::from_name(second)?),
        _ => return None,
    };
    let maximum_per_color = match &body[split + 3..] {
        [] => None,
        ["no", "more", "than", "one" | "1", "mana", "of", "each", "color", "may", "be", "spent", "this", "way"] => Some(1),
        _ => return None,
    };
    // A complete sentence match: every word was read.
    crate::word_claims::claim_words(&words);
    Some(ManaSpendingRestriction::OnX { colors, maximum_per_color })
}

/// Split only complete recognized spending sentences. The enclosing ability
/// or modal owner must consume the returned rules as announcement costs.
pub(crate) fn split_x_spending_sentences(tokens: &[OwnedLexToken])
    -> (Vec<OwnedLexToken>, Vec<ManaSpendingRestriction>) {
    let mut kept = Vec::new();
    let mut rules = Vec::new();
    for sentence in crate::lexer::split_lexed_sentences(tokens) {
        if let Some(rule) = x_spending_rule(sentence) {
            if !rules.contains(&rule) { rules.push(rule); }
        } else { kept.push(sentence.to_vec()); }
    }
    (crate::util::join_sentences_with_period(&kept), rules)
}

pub(crate) fn constrain_activation_cost(
    cost: ironsmith_core::TotalCost<crate::model::CompilerCost>,
    rules: &[ManaSpendingRestriction],
) -> ironsmith_core::TotalCost<crate::model::CompilerCost> {
    cost.try_map(|component| Ok::<_, std::convert::Infallible>(match component {
        crate::model::CompilerCost::Mana(mut cost) => {
            for rule in rules { cost = cost.with_spending_restriction(rule.clone()); }
            crate::model::CompilerCost::Mana(cost)
        },
        other => other,
    })).unwrap_or_else(|never| match never {})
}

#[cfg(test)]
mod tests {
    use super::*;
    fn tokens(text: &str) -> Vec<OwnedLexToken> { crate::lexer::lex_line(text, 0).unwrap() }
    #[test]
    fn x_spending_reader_requires_the_exact_scope_and_color_relationship() {
        use ironsmith_core::color::ColorSet;
        assert_eq!(x_spending_rule(&tokens("Spend only black and/or red mana on X.")),
            Some(ManaSpendingRestriction::OnX { colors: ColorSet::BLACK.union(ColorSet::RED), maximum_per_color: None }));
        assert!(matches!(x_spending_rule(&tokens("Spend only colored mana on X. No more than one mana of each color may be spent this way.")),
            Some(ManaSpendingRestriction::OnX { maximum_per_color: Some(1), .. })));
        for unsupported in ["Spend only black mana to cast this spell.", "Spend this mana only on X.",
            "Spend only black mana on Y.", "Spend only red mana on X if you attacked this turn."] {
            assert!(x_spending_rule(&tokens(unsupported)).is_none(), "{unsupported}");
        }
    }
}
