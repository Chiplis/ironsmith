//! Object-filter operands for custom keyword identities and named payments.
use crate::lexer::{OwnedLexToken, TokenKind};
/// The custom-mechanic binder writes "a [qualified] spell using <name>".
/// A following controller clause belongs to the spell, outside the payment name.
fn bare_payment_operand(tokens: &[OwnedLexToken], start: usize) -> Option<(String, usize)> {
    if !tokens[..start].last()?.is_any_word(&["spell", "spells"]) {
        return None;
    }
    let tail = &tokens[start + 1..];
    let end = tail
        .iter()
        .position(|token| token.is_any_word(&["you", "and", "or", "and/or"]))
        .unwrap_or(tail.len());
    if tail.get(end).is_some_and(|token| token.is_word("you"))
        && !tail
            .get(end + 1)
            .is_some_and(|token| token.is_word("control"))
    {
        return None;
    }
    Some((bare_payment_name(&tail[..end])?, start + 1 + end))
}

/// Legacy unquoted names consist of up to four words. Native keywords and
/// mana provenance have their own readers.
pub(crate) fn bare_payment_name(tokens: &[OwnedLexToken]) -> Option<String> {
    if !(1..=4).contains(&tokens.len()) {
        return None;
    }
    let words = crate::lexer::parser_token_word_refs(tokens);
    if words.len() != tokens.len() || words[0] == "mana" || words.starts_with(&["custom", "cost"]) {
        return None;
    }
    let name = words.join(" ");
    if name == "teamwork"
        || crate::keyword_forms::official(&name).is_some()
        || crate::keyword_forms::is_participle(&name)
    {
        return None;
    }
    Some(name)
}

pub(crate) fn filter_clause(
    tokens: &[OwnedLexToken],
) -> Option<(Vec<OwnedLexToken>, String, bool)> {
    let mut quoted = false;
    for (start, token) in tokens.iter().enumerate() {
        if token.kind == TokenKind::Quote {
            quoted = !quoted;
            continue;
        }
        let payment = token.is_word("using");
        if quoted || !(payment || token.is_any_word(&["with", "without"])) {
            continue;
        }
        let operand =
            start + 1 + usize::from(tokens.get(start + 1).is_some_and(|t| t.is_word("the")));
        let kind = if payment { "cost" } else { "keyword" };
        let (name, end) = if tokens.get(operand).is_some_and(|t| t.is_word("custom"))
            && tokens.get(operand + 1).is_some_and(|t| t.is_word(kind))
            && tokens
                .get(operand + 2)
                .is_some_and(|t| t.kind == TokenKind::Quote)
        {
            // Clause trimming can remove the closing quote of a final operand.
            let body = operand + 3;
            let close = (body..tokens.len())
                .find(|i| tokens[*i].kind == TokenKind::Quote)
                .unwrap_or(tokens.len());
            let name = crate::lexer::render_literal_token_slice(&tokens[body..close])
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
                .to_ascii_lowercase();
            if name.is_empty() {
                return None;
            }
            (name, (close + 1).min(tokens.len()))
        } else if payment && let Some(pair) = bare_payment_operand(tokens, start) {
            pair
        } else {
            continue;
        };
        let cast = payment && tokens[..start].last().is_some_and(|t| t.is_word("cast"));
        let marker_start = if cast
            && start >= 3
            && tokens[start - 3].is_word("that")
            && tokens[start - 2].is_word("was")
        {
            start - 3
        } else {
            start - usize::from(cast)
        };
        let mut base = tokens[..marker_start].to_vec();
        base.extend_from_slice(&tokens[end..]);
        crate::word_claims::claim_tokens(&tokens[marker_start..end]);
        let marker = if cast {
            format!("cast using {name}")
        } else if payment {
            format!("using {name}")
        } else {
            name
        };
        return Some((base, marker, token.is_word("without")));
    }
    None
}

pub(crate) fn has_unquoted_union(tokens:&[OwnedLexToken])->bool{
 let mut quoted=false;for token in tokens{if token.kind==TokenKind::Quote{quoted=!quoted;}else if !quoted&&token.is_any_word(&["and","or","and/or"]){return true;}}false
}
#[cfg(test)]mod tests{
 #[test]fn an_explicit_named_price_operand_is_distinct_from_keyword_identity(){
  use crate::object_filters::parse_object_filter;
  let tokens=crate::lexer::lex_line("another creature you control using custom cost \"Aerial Passage\"",0).unwrap();
  let filter=parse_object_filter(&tokens,false).unwrap();
  assert_eq!(filter.ability_markers,vec!["using aerial passage"]);
  assert!(filter.other);
  assert_eq!(filter.controller,Some(crate::target::PlayerFilter::You));
  let tokens=crate::lexer::lex_line("a creature with custom keyword \"Aerial Passage\"",0).unwrap();
  let identity=parse_object_filter(&tokens,false).unwrap();
  assert_eq!(identity.ability_markers,vec!["aerial passage"]);
  let tokens=crate::lexer::lex_line("another creature you control that was cast using custom cost \"Aerial Passage\"",0).unwrap();
  let actual=parse_object_filter(&tokens,false).unwrap();
  assert_eq!(actual.ability_markers,vec!["cast using aerial passage"]);
 }
 #[test]fn identity_operand_names_and_union_branches_are_independent(){
  use crate::object_filters::parse_object_filter;
  for name in ["breach","seven seas","doctor's spark"]{
   let tokens=crate::lexer::lex_line(&format!("creatures without custom keyword \"{name}\" you control"),0).unwrap();let filter=parse_object_filter(&tokens,false).unwrap();assert_eq!(filter.excluded_ability_markers,vec![name]);
  }
  let tokens=crate::lexer::lex_line("Curses and creatures with custom keyword \"breach\" you control",0).unwrap();let filter=parse_object_filter(&tokens,false).unwrap();assert_eq!(filter.any_of.len(),2);assert!(filter.any_of[0].ability_markers.is_empty());assert_eq!(filter.any_of[1].ability_markers,vec!["breach"]);
 }
}

#[cfg(test)]
mod recursive_branch_entry_tests {
    use super::*;
    #[test]
    fn grammar_union_arms_preserve_custom_identity_operands() {
        for (name, without) in [("breach", false), ("moon compose", true)] {
            let relation = if without { "without" } else { "with" };
            let text = format!("other Curses and creatures {relation} custom keyword \"{name}\" you control");
            let tokens = crate::lexer::lex_line(&text, 0).unwrap();
            let filter = crate::object_filters::parse_object_filter_lexed(&tokens, false).unwrap();
            assert_eq!(filter.any_of.len(), 2, "{filter:?}");
            let creature = &filter.any_of[1];
            assert!(filter.any_of[0].ability_markers.is_empty());
            assert!(filter.any_of[0].excluded_ability_markers.is_empty());
            if without { assert_eq!(creature.excluded_ability_markers, [name]); }
            else { assert_eq!(creature.ability_markers, [name]); }
            assert!(creature.other || filter.other, "shared other constraint survives");
            assert!(creature.controller.is_some() || filter.controller.is_some(), "shared controller survives");
        }
    }
    #[test]
    fn grammar_direct_identity_operand_keeps_exact_name() {
        let tokens = crate::lexer::lex_line("creatures with custom keyword \"moon compose\"", 0).unwrap();
        let filter = crate::grammar::filters::parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false).unwrap();
        assert_eq!(filter.ability_markers, ["moon compose"]);
    }
}

#[cfg(test)]
mod named_payment_tests {
    #[test]
    fn a_bare_payment_and_a_quoted_payment_keep_the_same_controller_and_marker() {
        let parse = |text| {
            let tokens = crate::lexer::lex_line(text, 0).unwrap();
            crate::object_filters::parse_object_filter_lexed(&tokens, false).unwrap()
        };
        let bare = parse("creature spells using aerial passage you control");
        assert_eq!(bare, parse("creature spells using custom cost \"aerial passage\" you control"));
        assert_eq!(bare.controller, Some(crate::target::PlayerFilter::You));
        assert_eq!(bare.ability_markers, ["using aerial passage"]);
        for other in ["a spell using teamwork", "a spell using flashback", "a spell using mana from lands"] {
            let tokens = crate::lexer::lex_line(other, 0).unwrap();
            assert!(super::filter_clause(&tokens).is_none(), "{other}");
        }
    }
}
