use crate::lexer::{OwnedLexToken, TokenKind, parser_token_word_refs, render_token_slice};
use crate::parse_context::ParseContextView;
use crate::target::SourceReferenceSurface;
use winnow::Parser;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceOperandSurfaceShape {
    pub surface: SourceReferenceSurface,
}

fn operand_end(tokens: &[OwnedLexToken], start: usize) -> usize {
    for (offset, token) in tokens[start..].iter().enumerate() {
        if matches!(
            token.kind,
            TokenKind::Comma | TokenKind::Period | TokenKind::Semicolon
        ) || token.is_word("then")
        {
            return start + offset;
        }
    }
    tokens.len()
}

fn operand_after(tokens: &[OwnedLexToken], action_index: usize) -> Option<&[OwnedLexToken]> {
    let start = action_index.checked_add(1)?;
    let end = operand_end(tokens, start);
    (end > start).then(|| &tokens[start..end])
}

pub fn parse_named_surface(tokens: &[OwnedLexToken]) -> Option<SourceReferenceSurface> {
    if !crate::lexer::is_authored_proper_name_phrase(tokens) {
        return None;
    }
    let text = render_token_slice(tokens).trim().to_string();
    Some(if parser_token_word_refs(tokens).len() == 1 {
        SourceReferenceSurface::ShortName(text)
    } else {
        SourceReferenceSurface::FullName(text)
    })
}

pub fn parse_leading_named_surface(tokens: &[OwnedLexToken]) -> Option<SourceOperandSurfaceShape> {
    let candidate = tokens.get(..operand_end(tokens, 0))?;
    parse_named_surface(candidate).map(|surface| SourceOperandSurfaceShape { surface })
}

fn push_unique_surface(
    surfaces: &mut Vec<SourceReferenceSurface>,
    surface: SourceReferenceSurface,
) {
    if !surfaces.iter().any(|existing| existing == &surface) {
        surfaces.push(surface);
    }
}

fn unique_surface(surfaces: Vec<SourceReferenceSurface>) -> Option<SourceOperandSurfaceShape> {
    let mut surfaces = surfaces.into_iter();
    let surface = surfaces.next()?;
    surfaces
        .next()
        .is_none()
        .then_some(SourceOperandSurfaceShape { surface })
}

pub fn parse_unique_named_operand_after(
    context: Option<ParseContextView<'_>>,
    tokens: &[OwnedLexToken],
    action_word: &str,
) -> Option<SourceOperandSurfaceShape> {
    let mut surfaces = Vec::new();
    for (action_index, token) in tokens.iter().enumerate() {
        if !token.is_word(action_word) {
            continue;
        }
        let candidate = operand_after(tokens, action_index)?;
        let surface = context
            .and_then(|context| {
                crate::util::authored_named_source_reference_surface(context, candidate)
            })
            .or_else(|| parse_named_surface(candidate));
        if let Some(surface) = surface {
            push_unique_surface(&mut surfaces, surface);
        }
    }
    unique_surface(surfaces)
}

pub fn parse_unique_source_operand_after(
    context: ParseContextView<'_>,
    tokens: &[OwnedLexToken],
    action_word: &str,
) -> Option<SourceOperandSurfaceShape> {
    let mut surfaces = Vec::new();
    for (action_index, token) in tokens.iter().enumerate() {
        if !token.is_word(action_word) {
            continue;
        }
        let candidate = operand_after(tokens, action_index)?;
        let words = parser_token_word_refs(candidate);
        let surface =
            if candidate.len() == 1 && matches!(candidate[0].parser_text(), "it" | "him" | "her") {
                Some(SourceReferenceSurface::ThisPermanentType(
                    candidate[0].slice.clone(),
                ))
            } else {
                crate::util::this_source_surface_for_words(&words)
                    .or_else(|| {
                        crate::util::authored_named_source_reference_surface(context, candidate)
                    })
                    .or_else(|| parse_named_surface(candidate))
            };
        if let Some(surface) = surface {
            push_unique_surface(&mut surfaces, surface);
        }
    }
    unique_surface(surfaces)
}

pub fn parse_unique_named_counter_on_operand(
    tokens: &[OwnedLexToken],
) -> Option<SourceOperandSurfaceShape> {
    let mut surfaces = Vec::new();
    for (counter_index, token) in tokens.iter().enumerate() {
        if !matches!(token.parser_text(), "counter" | "counters")
            || !tokens
                .get(counter_index + 1)
                .is_some_and(|token| token.is_word("on"))
        {
            continue;
        }
        let candidate = operand_after(tokens, counter_index + 1)?;
        if let Some(surface) = parse_named_surface(candidate) {
            push_unique_surface(&mut surfaces, surface);
        }
    }
    unique_surface(surfaces)
}

pub fn parse_unique_pronoun_operand_after(
    tokens: &[OwnedLexToken],
    action_word: &str,
) -> Option<SourceOperandSurfaceShape> {
    let mut surfaces = Vec::new();
    for (action_index, token) in tokens.iter().enumerate() {
        if !token.is_word(action_word) {
            continue;
        }
        let candidate = tokens.get(action_index + 1)?;
        if !matches!(candidate.parser_text(), "it" | "him" | "her") {
            continue;
        }
        push_unique_surface(
            &mut surfaces,
            SourceReferenceSurface::ThisPermanentType(candidate.slice.clone()),
        );
    }
    unique_surface(surfaces)
}

pub fn parse_named_operand_nearest_to(
    tokens: &[OwnedLexToken],
    action_word: &str,
    source_offset: usize,
) -> Option<SourceOperandSurfaceShape> {
    let mut best: Option<(usize, SourceReferenceSurface)> = None;
    for (action_index, token) in tokens.iter().enumerate() {
        if !token.is_word(action_word) {
            continue;
        }
        let candidate = operand_after(tokens, action_index)?;
        let Some(surface) = parse_named_surface(candidate) else {
            continue;
        };
        let distance = token.span.start.abs_diff(source_offset);
        if best
            .as_ref()
            .is_none_or(|(best_distance, _)| distance < *best_distance)
        {
            best = Some((distance, surface));
        }
    }
    best.map(|(_, surface)| SourceOperandSurfaceShape { surface })
}

pub fn parse_chosen_complement_surface(tokens: &[OwnedLexToken]) -> bool {
    crate::grammar::primitives::find_prefix(tokens, || {
        crate::grammar::primitives::phrase(&["creatures", "other", "than"]).void()
    })
    .is_some()
        && crate::grammar::primitives::find_prefix(tokens, || {
            crate::grammar::primitives::phrase(&["and", "the", "chosen", "creature"]).void()
        })
        .is_some()
}
