//! Complete active/passive token creation replacements. Token descriptions are
//! retained as token slices for the ordinary typed create-token grammar.
use winnow::combinator::{alt, opt, peek, repeat_till};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;
use crate::lexer::{LexStream, OwnedLexToken};
use super::super::primitives;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenTemplateReplacement<'a> {
    pub source_descriptor: &'a [OwnedLexToken],
    pub templates: Vec<&'a [OwnedLexToken]>,
    pub mode: ironsmith_core::TokenCreationTemplateMode,
    pub choose_one: bool,
    pub optional: bool,
    /// "The first time you would create one or more tokens each turn"
    /// (Moonlit Meditation).
    pub first_time_each_turn: bool,
}

fn header<'a>(input: &mut LexStream<'a>) -> WResult<(&'a [OwnedLexToken], bool, bool, bool)> {
    let first_time = opt(primitives::phrase(&["the", "first", "time"])).parse_next(input)?.is_some();
    if !first_time { primitives::kw("if").parse_next(input)?; }
    let active = opt(primitives::kw("you")).parse_next(input)?.is_some();
    if first_time && !active { return Err(primitives::backtrack_err("token replacement", "you")); }
    if active { primitives::phrase(&["would", "create"]).parse_next(input)?; }
    let singular = alt((
        primitives::phrase(&["one", "or", "more"]).value(false),
        alt((primitives::kw("a"), primitives::kw("an"))).value(true),
    )).parse_next(input)?;
    let descriptor = repeat_till::<_, _, (), _, _, _, _>(0.., any.void(),
        peek(alt((primitives::kw("token"), primitives::kw("tokens")))))
        .map(|((), _)| ()).take().parse_next(input)?;
    alt((primitives::kw("token"), primitives::kw("tokens"))).parse_next(input)?;
    if !active { primitives::phrase(&["would", "be", "created", "under", "your", "control"]).parse_next(input)?; }
    if first_time { primitives::phrase(&["each", "turn"]).parse_next(input)?; }
    opt(primitives::comma()).parse_next(input)?;
    Ok((descriptor, singular, active, first_time))
}
fn complete_tail<'a>(tokens: &'a [OwnedLexToken], words: &'static [&'static str]) -> Option<&'a [OwnedLexToken]> {
    let mut quoted = false;
    for (index, token) in tokens.iter().enumerate() {
        if token.is_quote() { quoted = !quoted; continue; }
        if quoted { continue; }
        if let Some(((), rest)) = primitives::parse_prefix(&tokens[index..], primitives::phrase(words))
            && primitives::probe_all(rest, primitives::sentence_end(), "complete token replacement tail").is_some()
        { return Some(&tokens[..index]); }
    }
    None
}
fn quantity(tokens: &[OwnedLexToken]) -> Option<(&[OwnedLexToken], bool)> {
    if let Some(((), rest)) = primitives::parse_prefix(tokens, primitives::phrase(&["that", "many"])) {
        return Some((rest, true));
    }
    let (_, rest) = primitives::parse_prefix(tokens, alt((primitives::kw("a"), primitives::kw("an"), primitives::kw("one"), primitives::kw("1"))))?;
    let rest = primitives::parse_prefix(rest, primitives::kw("additional")).map_or(rest, |(_, rest)| rest);
    Some((rest, false))
}

pub fn parse_token_template_replacement(tokens: &[OwnedLexToken]) -> Option<TokenTemplateReplacement<'_>> {
    use ironsmith_core::TokenCreationTemplateMode;
    let ((source_descriptor, singular, active, first_time_each_turn), mut body) =
        primitives::parse_prefix(tokens, header)?;
    let mut optional = false;
    let mut leading_instead = false;
    if active {
        if let Some(((), rest)) = primitives::parse_prefix(body, primitives::phrase(&["you", "may"])) {
            optional = true; body = rest;
        }
        if let Some((_, rest)) = primitives::parse_prefix(body, primitives::kw("instead")) {
            leading_instead = true; body = rest;
        }
        let (_, rest) = primitives::parse_prefix(body, primitives::kw("create"))?; body = rest;
        if let Some(tail) = complete_tail(body, &["instead"]) {
            if leading_instead { return None; }
            body = tail;
        } else if leading_instead {
            body = if body.last().is_some_and(OwnedLexToken::is_period) { &body[..body.len() - 1] } else { body };
        } else { return None; }
    } else {
        body = complete_tail(body, &["are", "created", "instead"])?;
    }
    // Own exactly one outer sentence. Periods inside a quoted token ability
    // belong to that definition; another outside sentence is a separate effect.
    let mut quoted = false;
    for token in body {
        if token.is_quote() { quoted = !quoted; }
        else if !quoted && (token.is_period() || token.is_semicolon()) { return None; }
    }
    if quoted { return None; }
    let addition = primitives::parse_prefix(body, primitives::phrase(&["those", "tokens", "plus"]));
    let (mode, template_body) = if let Some(((), tail)) = addition {
        let (template, per_token) = quantity(tail)?;
        (if per_token || singular { TokenCreationTemplateMode::AppendForEach } else { TokenCreationTemplateMode::AppendOnce }, template)
    } else {
        (TokenCreationTemplateMode::ReplaceEach, body)
    };
    let mut templates = Vec::new();
    let mut choose_one = false;
    if addition.is_some() {
        templates.push(template_body);
    } else {
        // Only a complete repeated quantity starts another recipe. "and" in
        // a token's keyword list or quoted ability remains inside its template.
        let mut rest = template_body;
        loop {
            let (descriptor, echoed) = quantity(rest)?;
            if !singular && !echoed { return None; }
            let mut quoted = false;
            let split = descriptor.iter().enumerate().find_map(|(index, token)| {
                if token.is_quote() { quoted = !quoted; return None; }
                if quoted || !(token.is_word("and") || token.is_word("or")) { return None; }
                quantity(&descriptor[index + 1..]).map(|_| (index, token.is_word("or")))
            });
            if let Some((index, alternative)) = split {
                if !templates.is_empty() && choose_one != alternative { return None; }
                choose_one = alternative;
                templates.push(&descriptor[..index]); rest = &descriptor[index + 1..];
            } else { templates.push(descriptor); break; }
        }
    }
    if templates.iter().any(|template| template.is_empty()) { return None; }
    Some(TokenTemplateReplacement { source_descriptor, templates, mode, choose_one, optional, first_time_each_turn })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex_line;
    use ironsmith_core::TokenCreationTemplateMode::*;
    #[test]
    fn complete_template_shapes_distinguish_per_event_per_token_and_choices() {
        for (text, mode, count, choice, optional) in [
            ("If you would create a Food token, instead create a Food token and a Treasure token.", ReplaceEach, 2, false, false),
            ("If one or more creature tokens would be created under your control, that many 4/4 white Angel creature tokens with flying and vigilance are created instead.", ReplaceEach, 1, false, false),
            ("If one or more artifact tokens would be created under your control, those tokens plus an additional 1/1 colorless Thopter artifact creature token with flying are created instead.", AppendOnce, 1, false, false),
            ("If you would create a Fish token, create a 3/3 blue Shark creature token instead.", ReplaceEach, 1, false, false),
            ("If you would create one or more tokens, you may instead create that many 2/2 green Cat creature tokens with haste or that many 3/1 green Dog creature tokens with vigilance.", ReplaceEach, 2, true, true),
        ] {
            let tokens = lex_line(text, 0).unwrap();
            let shape = parse_token_template_replacement(&tokens).unwrap_or_else(|| panic!("{text}"));
            assert_eq!((shape.mode, shape.templates.len(), shape.choose_one, shape.optional), (mode, count, choice, optional), "{text}");
        }
    }
    #[test]
    fn token_template_boundary_does_not_drop_conditions_extra_sentences_or_choice_words() {
        for text in [
            "If you would create one or more tokens for the first time each turn, instead create that many Food tokens.",
            "If you would create a Fish token, create a Shark token instead unless an opponent pays {1}.",
            "If you would create a Fish token, instead create a Shark token. Create a Food token.",
            "If you would create a Fish token, instead create a Shark token instead.",
            "If one or more tokens would be created under an opponent's control, those tokens plus a Food token are created instead.",
            "If you would create one or more tokens, create a Food token instead.",
        ] { assert!(parse_token_template_replacement(&lex_line(text, 0).unwrap()).is_none(), "{text}"); }
    }
}
