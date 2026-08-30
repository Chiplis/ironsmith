use super::*;

pub fn trim_leading_commas(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    let start = first_non_comma_token_index(tokens);
    &tokens[start..]
}
