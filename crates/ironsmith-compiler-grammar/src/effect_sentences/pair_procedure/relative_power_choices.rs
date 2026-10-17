//! "Choose a creature you control, then each opponent chooses a creature they
//! control with equal or lesser power. If you chose a creature this way,
//! exile each creature not chosen by any player this way." (Sculpted
//! Sunburst): each opponent's choice is filtered by the power of the
//! creature you chose (a dynamic comparison read as each opponent chooses),
//! and "not chosen by any player this way" is the complement of both chosen
//! sets. Without your choice there is no power to compare against and the
//! whole remainder does nothing.
use super::*;
use crate::cards::builders::{ConditionalEffectAst, ForEachEffectAst, ObjectChoiceEffectAst};
use crate::effect::ChoiceCount;
use crate::lexer::{OwnedLexToken, TokenKind, TokenWordView};
use crate::target::ChooseSpec;
use crate::util::trim_edge_punctuation_tokens;

fn relative_power(words: &[&str]) -> Option<(usize, fn(Box<Value>) -> crate::filter::Comparison)> {
    use crate::filter::Comparison;
    const SHAPES: &[(&[&str], fn(Box<Value>) -> Comparison)] = &[
        (&["with", "equal", "or", "lesser", "power"], Comparison::LessThanOrEqualExpr),
        (&["with", "equal", "or", "greater", "power"], Comparison::GreaterThanOrEqualExpr),
        (&["with", "lesser", "power"], Comparison::LessThanExpr),
        (&["with", "greater", "power"], Comparison::GreaterThanExpr),
        (&["with", "equal", "power"], Comparison::EqualExpr),
    ];
    SHAPES
        .iter()
        .find(|(suffix, _)| words.ends_with(suffix))
        .map(|(suffix, comparison)| (suffix.len(), *comparison))
}

fn tokens_for_words<'a>(
    tokens: &'a [OwnedLexToken],
    view: &TokenWordView<'_>,
    start: usize,
    end: usize,
) -> Option<&'a [OwnedLexToken]> {
    let range = view.token_span_for_words(start, end)?;
    Some(trim_edge_punctuation_tokens(&tokens[range]))
}

pub(super) fn read(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let (Some(first), Some(second)) = (sentences.get(sentence_idx), sentences.get(sentence_idx + 1))
    else {
        return Ok(None);
    };
    // Sentence one: "choose a <yours>, then each opponent chooses a <theirs>
    // <relative power>".
    let first_tokens = trim_edge_punctuation_tokens(first.lowered());
    let first_view = TokenWordView::new(first_tokens);
    let first_words = first_view.to_word_refs();
    if !matches!(first_words.as_slice(), ["choose", "a" | "an", ..]) {
        return Ok(None);
    }
    let then_marker = ["then", "each", "opponent", "chooses"];
    let Some(then_idx) = first_words
        .windows(then_marker.len())
        .position(|window| window == then_marker)
    else {
        return Ok(None);
    };
    if !first_tokens.iter().any(|token| token.kind == TokenKind::Comma) {
        return Ok(None);
    }
    let theirs_start = then_idx + then_marker.len();
    if !matches!(first_words.get(theirs_start), Some(&"a" | &"an")) {
        return Ok(None);
    }
    let theirs_words = &first_words[theirs_start + 1..];
    let Some((suffix_len, comparison)) = relative_power(theirs_words) else {
        return Ok(None);
    };
    let theirs_words = &theirs_words[..theirs_words.len() - suffix_len];
    let Some(theirs_noun_len) = theirs_words
        .strip_suffix(&["they", "control"][..])
        .map(<[&str]>::len)
    else {
        return Ok(None);
    };
    if theirs_noun_len == 0 || then_idx <= 2 {
        return Ok(None);
    }

    // Sentence two: "if you chose a <noun> this way, exile each <filter> not
    // chosen by any player this way".
    let second_tokens = trim_edge_punctuation_tokens(second.lowered());
    let second_view = TokenWordView::new(second_tokens);
    let second_words = second_view.to_word_refs();
    let Some(after_if) = second_words.strip_prefix(&["if", "you", "chose", "a"][..])
        .or_else(|| second_words.strip_prefix(&["if", "you", "chose", "an"][..]))
    else {
        return Ok(None);
    };
    let Some(this_way) = after_if.windows(2).position(|window| window == ["this", "way"]) else {
        return Ok(None);
    };
    let consequence = &after_if[this_way + 2..];
    let Some(exiled) = consequence
        .strip_prefix(&["exile", "each"][..])
        .or_else(|| consequence.strip_prefix(&["exile", "all"][..]))
    else {
        return Ok(None);
    };
    let Some(exiled_noun) = exiled
        .strip_suffix(&["not", "chosen", "by", "any", "player", "this", "way"][..])
        .or_else(|| exiled.strip_suffix(&["not", "chosen", "this", "way"][..]))
    else {
        return Ok(None);
    };
    if exiled_noun.is_empty() {
        return Ok(None);
    }
    let exiled_start = second_words.len() - exiled.len();

    let Some(yours_tokens) = tokens_for_words(first_tokens, &first_view, 2, then_idx) else {
        return Ok(None);
    };
    let Some(theirs_tokens) = tokens_for_words(
        first_tokens,
        &first_view,
        theirs_start + 1,
        theirs_start + 1 + theirs_noun_len,
    ) else {
        return Ok(None);
    };
    let Some(exiled_tokens) = tokens_for_words(
        second_tokens,
        &second_view,
        exiled_start,
        exiled_start + exiled_noun.len(),
    ) else {
        return Ok(None);
    };

    let yours_filter =
        crate::object_filters::parse_object_filter(yours_tokens, false)?.in_zone(Zone::Battlefield);
    let mut theirs_filter = crate::object_filters::parse_object_filter(theirs_tokens, false)?
        .in_zone(Zone::Battlefield)
        .controlled_by(PlayerFilter::IteratedPlayer);
    let exiled_filter =
        crate::object_filters::parse_object_filter(exiled_tokens, false)?.in_zone(Zone::Battlefield);

    let yours = crate::util::helper_tag_for_tokens(first_tokens, "relative_power_anchor");
    let theirs = crate::util::helper_tag_for_tokens(second_tokens, "relative_power_choices");
    theirs_filter.power = Some(comparison(Box::new(Value::PowerOf(Box::new(
        ChooseSpec::Tagged(yours.key.clone()),
    )))));
    let exiled_filter = exiled_filter
        .not_tagged(yours.key.clone())
        .not_tagged(theirs.key.clone());

    let opponents_choose = EffectAst::ForEach(ForEachEffectAst::ForEachOpponent {
        effects: vec![EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects {
            filter: theirs_filter,
            count: ChoiceCount::exactly(1),
            count_value: None,
            player: PlayerAst::That,
            tag: theirs.clone(),
        })],
    });
    Ok(Some(vec![
        EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects {
            filter: yours_filter,
            count: ChoiceCount::exactly(1),
            count_value: None,
            player: PlayerAst::You,
            tag: yours.clone(),
        }),
        EffectAst::Conditionals(ConditionalEffectAst::Conditional {
            predicate: PredicateAst::TaggedMatches(yours, ObjectFilter::default()),
            if_true: vec![
                opponents_choose,
                EffectAst::subject_verb_exile_all(exiled_filter, false),
            ],
            if_false: Vec::new(),
        }),
    ]))
}
