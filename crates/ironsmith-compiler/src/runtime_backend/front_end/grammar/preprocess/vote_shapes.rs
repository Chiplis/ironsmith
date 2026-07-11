use super::super::permission_shapes;
use crate::runtime_backend::lexer::{TokenWordView, lex_line, render_token_slice};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum VoteCountRewriteSurface {
    TruthDraw,
    ConsequencesDamage,
    DeathAndTaxes { left: String, middle: String },
    TrailingForEach { head: String, tail: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ReturnSourceLeavesSurface {
    pub(crate) subject: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PreviousExileSurface;

pub(crate) fn parse_vote_count_rewrite_surface(sentence: &str) -> Option<VoteCountRewriteSurface> {
    let tokens = lex_line(sentence.trim(), 0).ok()?;
    let words = TokenWordView::new(&tokens);
    let word_refs = words.word_refs();
    if permission_shapes::exact_words(
        &word_refs,
        &[
            "you", "draw", "cards", "equal", "to", "the", "number", "of", "truth", "votes",
        ],
    ) {
        return Some(VoteCountRewriteSurface::TruthDraw);
    }
    if permission_shapes::exact_words(
        &word_refs,
        &[
            "truth",
            "or",
            "consequences",
            "deals",
            "3",
            "damage",
            "to",
            "that",
            "player",
            "for",
            "each",
            "consequences",
            "vote",
        ],
    ) {
        return Some(VoteCountRewriteSurface::ConsequencesDamage);
    }

    if let Some(death) =
        permission_shapes::find_words(&word_refs, &["for", "each", "death", "vote", "and"])
    {
        let after_death = death + 5;
        if let Some(taxes_relative) = permission_shapes::find_words(
            word_refs.get(after_death..)?,
            &["for", "each", "taxes", "vote"],
        ) {
            let taxes = after_death + taxes_relative;
            let left_range = words.token_span_for_words(0, death)?;
            let middle_range = words.token_span_for_words(after_death, taxes)?;
            let left = render_token_slice(tokens.get(left_range)?)
                .trim()
                .to_string();
            let middle = render_token_slice(tokens.get(middle_range)?)
                .trim()
                .to_string();
            if !left.is_empty() && !middle.is_empty() {
                return Some(VoteCountRewriteSurface::DeathAndTaxes { left, middle });
            }
        }
    }

    let mut marker = None;
    let mut search_start = 0usize;
    while search_start < word_refs.len() {
        let Some(relative) =
            permission_shapes::find_words(word_refs.get(search_start..)?, &["for", "each"])
        else {
            break;
        };
        let found = search_start + relative;
        marker = Some(found);
        search_start = found + 1;
    }
    let marker = marker?;
    let tail_words = word_refs.get(marker + 2..)?;
    if tail_words.len() < 2 || !matches!(tail_words.last().copied(), Some("vote") | Some("votes")) {
        return None;
    }
    let head_range = words.token_span_for_words(0, marker)?;
    let tail_range = words.token_span_for_words(marker + 2, words.len())?;
    let head = render_token_slice(tokens.get(head_range)?)
        .trim()
        .to_string();
    let tail = render_token_slice(tokens.get(tail_range)?)
        .trim()
        .to_string();
    (!head.is_empty() && !tail.is_empty())
        .then_some(VoteCountRewriteSurface::TrailingForEach { head, tail })
}

pub(crate) fn parse_return_source_leaves_surface(
    sentence: &str,
) -> Option<ReturnSourceLeavesSurface> {
    let tokens = lex_line(sentence.trim(), 0).ok()?;
    let words = TokenWordView::new(&tokens);
    let word_refs = words.word_refs();
    let prefix = [
        "return",
        "that",
        "card",
        "to",
        "the",
        "battlefield",
        "under",
        "its",
        "owners",
        "control",
        "when",
        "this",
    ];
    if !permission_shapes::prefix_words(&word_refs, &prefix) {
        return None;
    }
    let suffix_relative = permission_shapes::find_words(
        word_refs.get(prefix.len()..)?,
        &["leaves", "the", "battlefield"],
    )?;
    let suffix = prefix.len() + suffix_relative;
    let subject_range = words.token_span_for_words(prefix.len(), suffix)?;
    let subject = render_token_slice(tokens.get(subject_range)?)
        .trim()
        .to_string();
    (!subject.is_empty()).then_some(ReturnSourceLeavesSurface { subject })
}

pub(crate) fn parse_previous_exile_surface(previous: &str) -> Option<PreviousExileSurface> {
    let tokens = lex_line(previous.trim(), 0).ok()?;
    let words = TokenWordView::new(&tokens).word_refs();
    (permission_shapes::find_words(&words, &["exile"]).is_some()
        && permission_shapes::find_words(&words, &["until", "this"]).is_none())
    .then_some(PreviousExileSurface)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_vote_and_return_surfaces() {
        assert_eq!(
            parse_vote_count_rewrite_surface("You draw cards equal to the number of truth votes"),
            Some(VoteCountRewriteSurface::TruthDraw)
        );
        let returned = parse_return_source_leaves_surface(
            "Return that card to the battlefield under its owner's control when this artifact leaves the battlefield",
        )
        .expect("return surface");
        assert_eq!(returned.subject, "artifact");
        assert!(parse_previous_exile_surface("Exile target creature").is_some());
    }
}
