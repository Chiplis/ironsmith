//! Consumer-side spending requirements. Producer restrictions ("spend this
//! mana only ...") belong to the mana-producing ability and are separate.
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
    let filter = match producer {
        ["basic", "lands"] => ManaProducerFilter::All(vec![
            ManaProducerFilter::CardType(ironsmith_core::CardType::Land),
            ManaProducerFilter::Supertype(ironsmith_core::Supertype::Basic),
        ]),
        ["creatures"] => ManaProducerFilter::CardType(ironsmith_core::CardType::Creature),
        ["treasures"] => ManaProducerFilter::Subtype(ironsmith_core::Subtype::Treasure),
        _ => return None,
    };
    Some(ManaSpendingRestriction::ProducedBy(filter))
}

pub(crate) fn spell_source_spending_ability(
    tokens: &[OwnedLexToken],
) -> Option<crate::model::CompilerStaticAbilityCore> {
    let rule = source_spending_rule(tokens, false).or_else(|| x_spending_rule(tokens))?;
    Some(
        crate::model::CompilerStaticAbilityCore::spell_mana_spending_restriction(
            rule,
            crate::lexer::render_token_slice(tokens)
                .trim()
                .trim_end_matches('.'),
        ),
    )
}

/// A restriction on actual mana spent on X. It is never lowered as a colored
/// mana symbol or as a resolution effect.
pub(crate) fn x_spending_rule(tokens: &[OwnedLexToken]) -> Option<ManaSpendingRestriction> {
    use ironsmith_core::color::{Color, ColorSet};
    let words = TokenWordView::new(tokens).word_refs();
    let body = words.strip_prefix(&["spend", "only"])?;
    let split = body.windows(3).position(|part| part == ["mana", "on", "x"])?;
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
