//! "Choose one [at random | that hasn't been chosen this turn]. Create a
//! [<colors>] creature token with those characteristics." followed by bullet
//! modes that are characteristic lists ("• 3/1 Human Warrior with trample and
//! haste"): each mode creates a token with the header's colors and the
//! bullet's power/toughness, creature types and abilities (CR 111.4,
//! CR 700.2). Each bullet is read through the ordinary create grammar.

use super::*;

/// How a characteristic bullet is applied.
#[derive(Debug, Clone)]
pub(super) enum CharacteristicsTemplate {
    /// "Create a <colors> creature token with those characteristics."
    CreateToken(Vec<String>),
    /// "Until end of turn, <subject> has that base power and toughness,
    /// becomes that creature type, and gains that ability." (Wild Shape):
    /// the subject's words.
    BecomeUntilEndOfTurn(Vec<String>),
}

/// The template sentence that follows the modal choice sentence.
pub(super) fn characteristics_template(
    header_tokens: &[OwnedLexToken],
) -> Option<CharacteristicsTemplate> {
    if let Some(colors) = token_template_colors(header_tokens) {
        return Some(CharacteristicsTemplate::CreateToken(colors));
    }
    split_lexed_sentences(header_tokens).iter().find_map(|sentence| {
        let words = crate::lexer::parser_token_word_refs(sentence);
        let rest = words.strip_prefix(&["until", "end", "of", "turn"][..])?;
        let subject = rest.strip_suffix(
            &[
                "has", "that", "base", "power", "and", "toughness", "becomes", "that", "creature",
                "type", "and", "gains", "that", "ability",
            ][..],
        )?;
        (!subject.is_empty()).then(|| {
            CharacteristicsTemplate::BecomeUntilEndOfTurn(
                subject.iter().map(|word| (*word).to_string()).collect(),
            )
        })
    })
}

/// The color words of a "create a <colors> creature token with those
/// characteristics" sentence that follows the modal choice sentence.
pub(super) fn token_template_colors(header_tokens: &[OwnedLexToken]) -> Option<Vec<String>> {
    let sentences = split_lexed_sentences(header_tokens);
    sentences.iter().find_map(|sentence| {
        let words = crate::lexer::parser_token_word_refs(sentence);
        let rest = words
            .strip_prefix(&["create", "a"][..])
            .or_else(|| words.strip_prefix(&["create", "an"][..]))?;
        let colors = rest.strip_suffix(
            &["creature", "token", "with", "those", "characteristics"][..],
        )?;
        colors
            .iter()
            .all(|word| *word == "and" || crate::util::parse_color(word).is_some())
            .then(|| colors.iter().map(|word| (*word).to_string()).collect())
    })
}

/// Read one characteristic bullet as the creation of that token.
pub(super) fn recognize_characteristics_mode(
    line: &PreprocessedLine,
    template: &CharacteristicsTemplate,
) -> Result<RecognizedModalMode, CardTextError> {
    let body = match line.tokens.first() {
        Some(token) if matches!(token.kind, TokenKind::Bullet | TokenKind::Dash) => {
            line.tokens.get(1..).unwrap_or_default()
        }
        _ => line.tokens.as_slice(),
    };
    let words = crate::lexer::parser_token_word_refs(body);
    let unsupported = || {
        CardTextError::ParseError(format!(
            "unsupported characteristic mode (clause: '{}')",
            words.join(" ")
        ))
    };
    // The abilities after "with" keep their authored tokens, so a quoted
    // ability ("with haste and \"When this creature enters, ...\"") reaches
    // the create grammar intact.
    let with_token = body.iter().position(|token| token.is_word("with"));
    let description_tokens = with_token.map_or(body, |idx| &body[..idx]);
    let description = crate::lexer::parser_token_word_refs(description_tokens);
    let Some((pt, characteristics)) = description.split_first() else {
        return Err(unsupported());
    };
    if !pt.contains('/') || characteristics.is_empty() {
        return Err(unsupported());
    }
    let text = match template {
        CharacteristicsTemplate::CreateToken(template_colors) => {
            let mut text = format!("create a {pt}");
            for word in template_colors {
                text.push(' ');
                text.push_str(word);
            }
            for word in characteristics {
                text.push(' ');
                text.push_str(word);
            }
            text.push_str(" creature token");
            text
        }
        CharacteristicsTemplate::BecomeUntilEndOfTurn(subject) => {
            // CR 205.1a: "becomes that creature type" replaces its creature
            // types; the base power and toughness are set in layer 7b.
            let mut text = format!("until end of turn, {} becomes a", subject.join(" "));
            for word in characteristics {
                text.push(' ');
                text.push_str(word);
            }
            text.push_str(&format!(" with base power and toughness {pt}"));
            if with_token.is_some() {
                text.push_str(" and gains");
            }
            text
        }
    };
    let mut tokens = crate::lexer::lex_line(&text, line.info.line_index)?;
    match (with_token, template) {
        (Some(idx), CharacteristicsTemplate::BecomeUntilEndOfTurn(_)) => {
            tokens.extend(body[idx + 1..].iter().cloned())
        }
        (Some(idx), CharacteristicsTemplate::CreateToken(_)) => {
            tokens.extend(body[idx..].iter().cloned())
        }
        (None, _) => tokens.extend(body.iter().filter(|token| token.is_period()).cloned()),
    }
    let effects_ast = parse_effect_sentences_lexed(&tokens)?;
    if effects_ast.is_empty() {
        return Err(unsupported());
    }
    let mode_text = render_original_text_for_token_slice(line, body)
        .unwrap_or_else(|| render_token_slice(body))
        .trim()
        .to_string();
    Ok(RecognizedModalMode {
        info: line.info.clone(),
        text: mode_text,
        point_cost: None,
        additional_mana_cost: None,
        effects_ast,
    })
}
