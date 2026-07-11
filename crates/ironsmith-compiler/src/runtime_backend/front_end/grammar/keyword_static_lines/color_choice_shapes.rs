use winnow::combinator::{alt, opt, peek, repeat_till};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

use super::super::super::lexer::{LexStream, OwnedLexToken, TokenWordView, trim_lexed_commas};
use super::super::{leaf, primitives};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ChosenColorSubjectSurface {
    ThisCreature,
    ThisPermanent,
    ThisCard,
    This,
    It,
    NamedSource,
}

impl ChosenColorSubjectSurface {
    pub(crate) fn display(self) -> &'static str {
        match self {
            Self::ThisCreature => "This creature",
            Self::ThisPermanent => "This permanent",
            Self::ThisCard => "This card",
            Self::This => "This",
            Self::It => "It",
            Self::NamedSource => "This",
        }
    }
}

pub(crate) fn parse_pregame_choose_color_tokens(tokens: &[OwnedLexToken]) -> bool {
    primitives::parse_all(
        tokens,
        parse_pregame_choose_color_lexed,
        "pregame choose-color line",
    )
    .is_ok()
}

pub(crate) fn parse_source_is_chosen_color_tokens(
    tokens: &[OwnedLexToken],
) -> Option<(ChosenColorSubjectSurface, bool)> {
    primitives::parse_all(
        tokens,
        parse_source_is_chosen_color_lexed,
        "source is chosen-color line",
    )
    .ok()
}

fn parse_pregame_choose_color_lexed<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    repeat_till::<_, _, (), _, _, _, _>(0.., any.void(), peek(primitives::kw("choose")))
        .void()
        .parse_next(input)?;
    primitives::kw("choose").parse_next(input)?;
    opt(alt((
        primitives::kw("a"),
        primitives::kw("an"),
        primitives::kw("one"),
    )))
    .parse_next(input)?;
    primitives::phrase(&["color", "before", "the", "game", "begins"]).parse_next(input)?;
    primitives::sentence_end().parse_next(input)
}

fn parse_source_is_chosen_color_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<(ChosenColorSubjectSurface, bool)> {
    let subject_tokens =
        repeat_till::<_, _, (), _, _, _, _>(1.., any.void(), peek(primitives::kw("is")))
            .map(|((), _)| ())
            .take()
            .parse_next(input)?;
    primitives::kw("is").parse_next(input)?;
    let has_article = opt(primitives::kw("the"))
        .map(|article| article.is_some())
        .parse_next(input)?;
    primitives::phrase(&["chosen", "color"]).parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    let subject = classify_chosen_color_subject(trim_lexed_commas(subject_tokens))?;
    Ok((subject, has_article))
}

fn classify_chosen_color_subject(tokens: &[OwnedLexToken]) -> WResult<ChosenColorSubjectSurface> {
    let exact = |phrase: &'static [&'static str]| {
        primitives::parse_all(tokens, primitives::phrase(phrase), "chosen-color subject").is_ok()
    };
    if exact(&["this", "creature"]) {
        return Ok(ChosenColorSubjectSurface::ThisCreature);
    }
    if exact(&["this", "permanent"]) {
        return Ok(ChosenColorSubjectSurface::ThisPermanent);
    }
    if exact(&["this", "card"]) {
        return Ok(ChosenColorSubjectSurface::ThisCard);
    }
    if exact(&["this"]) {
        return Ok(ChosenColorSubjectSurface::This);
    }
    if exact(&["it"]) {
        return Ok(ChosenColorSubjectSurface::It);
    }
    let words = TokenWordView::new(tokens).word_refs();
    if leaf::parse_leaf_this_source_reference_words(&words).is_some()
        || crate::runtime_backend::front_end::shared::util::source_reference_surface_for_words(
            &words,
        )
        .is_some()
    {
        return Ok(ChosenColorSubjectSurface::NamedSource);
    }
    Err(primitives::backtrack_err(
        "chosen-color subject",
        "source reference",
    ))
}

#[cfg(test)]
mod tests {
    use super::super::super::super::lexer::lex_line;
    use super::*;

    #[test]
    fn parses_pregame_choice_and_chosen_color_assignment() {
        let tokens = lex_line(
            "If this card is your commander, choose a color before the game begins.",
            0,
        )
        .unwrap();
        assert!(parse_pregame_choose_color_tokens(&tokens));

        let tokens = lex_line("This card is the chosen color.", 0).unwrap();
        assert_eq!(
            parse_source_is_chosen_color_tokens(&tokens),
            Some((ChosenColorSubjectSurface::ThisCard, true))
        );
    }
}
