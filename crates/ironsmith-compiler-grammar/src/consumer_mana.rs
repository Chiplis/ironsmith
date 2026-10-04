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
    let rule = source_spending_rule(tokens, false)?;
    Some(
        crate::model::CompilerStaticAbilityCore::spell_mana_spending_restriction(
            rule,
            crate::lexer::render_token_slice(tokens)
                .trim()
                .trim_end_matches('.'),
        ),
    )
}
