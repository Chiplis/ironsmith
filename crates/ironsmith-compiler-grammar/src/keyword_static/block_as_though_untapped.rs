//! "Tapped creatures you control can block as though they were untapped."
//! (Masako the Humorless): each matching creature has the blocking
//! permission (CR 509.1a), read through the shared object-filter grammar.
use super::*;

pub fn parse_can_block_as_though_untapped_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    let clean = trim_edge_punctuation_tokens(tokens);
    let Some((subject, ())) = crate::grammar::primitives::split_lexed_once_before_suffix(clean, 1, || {
        crate::grammar::primitives::phrase(&[
            "can", "block", "as", "though", "they", "were", "untapped",
        ])
    }) else {
        return Ok(None);
    };
    let Ok(filter) = crate::object_filters::parse_object_filter_lexed(subject, false) else {
        return Ok(None);
    };
    if !filter.card_types.contains(&crate::types::CardType::Creature) {
        return Ok(None);
    }
    Ok(Some(StaticAbilityAst::GrantStaticAbility {
        filter,
        ability: Box::new(StaticAbilityAst::Static(
            StaticAbility::can_block_as_though_untapped(),
        )),
        condition: None,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tapped_creatures_gain_the_untapped_blocking_permission() {
        let tokens = crate::lexer::lex_line(
            "Tapped creatures you control can block as though they were untapped.",
            0,
        )
        .unwrap();
        let ability = parse_can_block_as_though_untapped_line(&tokens)
            .unwrap()
            .expect("blocking permission");
        let text = format!("{ability:?}");
        assert!(text.contains("CanBlockAsThoughUntapped"), "{text}");
        assert!(text.contains("tapped: true"), "{text}");
    }
}
