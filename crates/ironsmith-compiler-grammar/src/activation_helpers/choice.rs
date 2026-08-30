use super::*;

pub fn parse_or_mana_color_choices(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<crate::color::Color>>, CardTextError> {
    Ok(activation_grammar::parse_or_mana_color_choices(tokens))
}
