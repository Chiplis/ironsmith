use super::*;

pub fn mana_symbol_to_color(symbol: ManaSymbol) -> Option<crate::color::Color> {
    match symbol {
        ManaSymbol::White => Some(crate::color::Color::White),
        ManaSymbol::Blue => Some(crate::color::Color::Blue),
        ManaSymbol::Black => Some(crate::color::Color::Black),
        ManaSymbol::Red => Some(crate::color::Color::Red),
        ManaSymbol::Green => Some(crate::color::Color::Green),
        _ => None,
    }
}

pub fn parse_any_combination_mana_colors(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<crate::color::Color>>, CardTextError> {
    let clause_words = TokenWordView::new(tokens).to_word_refs();
    activation_grammar::parse_any_combination_mana_colors(tokens).map_err(|error| {
        let detail = match error {
            activation_grammar::AnyCombinationManaError::MissingColors => {
                "missing color options".to_string()
            }
            activation_grammar::AnyCombinationManaError::UnsupportedSymbol(word) => {
                format!("unsupported restricted mana symbol '{word}'")
            }
            activation_grammar::AnyCombinationManaError::NonColoredSymbol(word) => {
                format!("unsupported non-colored mana symbol '{word}'")
            }
        };
        CardTextError::ParseError(format!(
            "{detail} in any-combination mana clause (clause: '{}')",
            clause_words.join(" ")
        ))
    })
}

pub fn is_mana_pool_tail_tokens(tokens: &[OwnedLexToken]) -> bool {
    activation_grammar::is_mana_pool_tail(tokens)
}
