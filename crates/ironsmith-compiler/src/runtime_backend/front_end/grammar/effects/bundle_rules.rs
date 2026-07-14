//! Typed grammar results for multi-sentence effect bundles.

use std::ops::Range;

use winnow::combinator::{alt, opt, peek, repeat};
use winnow::error::{ContextError, ErrMode, ModalResult as WResult};
use winnow::prelude::*;
use winnow::token::any;

use crate::cards::builders::LibraryBottomOrderAst;
use crate::filter::AlternativeCastKind;
use crate::runtime_backend::front_end::grammar::leaf;
use crate::runtime_backend::front_end::grammar::primitives::{self, WordSliceInput};
use crate::runtime_backend::front_end::lexer::{
    LexStream, OwnedLexToken, TokenWordView, parser_token_word_refs, render_token_slice,
    split_lexed_sentences, trim_lexed_commas,
};

#[path = "bundle_rules/replacement_sequences.rs"]
mod replacement_sequences;
pub(crate) use replacement_sequences::*;

#[path = "bundle_rules/consult_sequences.rs"]
mod consult_sequences;
pub(crate) use consult_sequences::*;

#[path = "bundle_rules/selection_sequences.rs"]
mod selection_sequences;
pub(crate) use selection_sequences::*;

#[path = "bundle_rules/resource_sequences.rs"]
mod resource_sequences;
pub(crate) use resource_sequences::*;

fn atom<'a>(
    expected: &'static str,
) -> impl Parser<WordSliceInput<'a>, &'a str, ErrMode<ContextError>> {
    primitives::word_slice_exact(expected)
}

fn sequence<'a>(
    expected: &'static [&'static str],
) -> impl Parser<WordSliceInput<'a>, (), ErrMode<ContextError>> {
    move |input: &mut WordSliceInput<'a>| {
        for expected_word in expected {
            atom(expected_word).void().parse_next(input)?;
        }
        Ok(())
    }
}

fn complete<'a, O>(
    words: &'a [&'a str],
    parser: impl Parser<WordSliceInput<'a>, O, ErrMode<ContextError>>,
) -> Option<O> {
    let mut input: WordSliceInput<'a> = words;
    (parser, primitives::word_slice_eof)
        .map(|(value, ())| value)
        .parse_next(&mut input)
        .ok()
}

fn consume_head<'a>(
    words: &'a [&'a str],
    expected: &'static [&'static str],
) -> Option<&'a [&'a str]> {
    let mut input: WordSliceInput<'a> = words;
    sequence(expected).parse_next(&mut input).ok()?;
    Some(input)
}

fn sequence_offset(words: &[&str], expected: &'static [&'static str]) -> Option<usize> {
    let mut input: WordSliceInput<'_> = words;
    while !input.is_empty() {
        let mut probe = input;
        if sequence(expected).parse_next(&mut probe).is_ok() {
            return words.len().checked_sub(input.len());
        }
        next_atom.parse_next(&mut input).ok()?;
    }
    None
}

fn atom_offset(words: &[&str], expected: &'static str) -> Option<usize> {
    let mut input: WordSliceInput<'_> = words;
    while !input.is_empty() {
        let offset = words.len().checked_sub(input.len())?;
        let mut probe = input;
        if atom(expected).parse_next(&mut probe).is_ok() {
            return Some(offset);
        }
        next_atom.parse_next(&mut input).ok()?;
    }
    None
}

fn next_atom<'a>(input: &mut WordSliceInput<'a>) -> WResult<&'a str> {
    let Some((word, tail)) = input.split_first() else {
        return Err(primitives::backtrack_err("bundle word", "word"));
    };
    *input = tail;
    Ok(*word)
}

fn has_atom(words: &[&str], expected: &'static str) -> bool {
    atom_offset(words, expected).is_some()
}

fn has_sequence(words: &[&str], expected: &'static [&'static str]) -> bool {
    sequence_offset(words, expected).is_some()
}

fn exact_surface(tokens: &[OwnedLexToken], expected: &'static [&'static str]) -> bool {
    let words = parser_token_word_refs(tokens);
    complete(&words, sequence(expected)).is_some()
}

fn token_slice_for_words(
    tokens: &[OwnedLexToken],
    word_range: Range<usize>,
) -> Option<&[OwnedLexToken]> {
    let token_range =
        TokenWordView::new(tokens).token_span_for_words(word_range.start, word_range.end)?;
    tokens.get(token_range)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AlternativeCostBundleShape {
    pub(crate) kind: AlternativeCastKind,
}

pub(crate) fn parse_alternative_cost_bundle_shape(
    first: &[OwnedLexToken],
    second: &[OwnedLexToken],
) -> Option<AlternativeCostBundleShape> {
    let first_words = parser_token_word_refs(first);
    let first_tail = consume_head(&first_words, &["you", "may", "cast", "a", "spell", "with"])?;
    let first_kind = leaf::parse_leaf_alternative_cast_prefix_words(first_tail)?;
    let first_remainder = first_tail.get(first_kind.consumed..)?;
    complete(first_remainder, sequence(&["from", "your", "hand"]))?;

    let second_words = parser_token_word_refs(second);
    let second_tail = consume_head(&second_words, &["if", "you", "do", "pay", "its"])?;
    let second_kind = leaf::parse_leaf_alternative_cast_prefix_words(second_tail)?;
    if second_kind.kind != first_kind.kind {
        return None;
    }
    let second_remainder = second_tail.get(second_kind.consumed..)?;
    complete(
        second_remainder,
        sequence(&["cost", "rather", "than", "its", "mana", "cost"]),
    )?;

    Some(AlternativeCostBundleShape {
        kind: first_kind.kind,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ChosenTypeReferenceShape;

pub(crate) fn parse_chosen_type_reference_shape(
    tokens: &[OwnedLexToken],
) -> Option<ChosenTypeReferenceShape> {
    let words = parser_token_word_refs(tokens);
    if !has_atom(&words, "type") || !(has_atom(&words, "that") || has_atom(&words, "chosen")) {
        return None;
    }
    Some(ChosenTypeReferenceShape)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SourceLeavesReturnShape;

pub(crate) fn parse_source_leaves_return_shape(
    tokens: &[OwnedLexToken],
) -> Option<SourceLeavesReturnShape> {
    let words = parser_token_word_refs(tokens);
    consume_head(&words, &["return"])?;
    for required in ["when", "leaves", "battlefield", "control"] {
        if !has_atom(&words, required) {
            return None;
        }
    }
    if !has_sequence(&words, &["to", "the", "battlefield"])
        || !(has_atom(&words, "owner")
            || has_atom(&words, "owners")
            || has_atom(&words, "owner's")
            || has_atom(&words, "owners'"))
    {
        return None;
    }
    Some(SourceLeavesReturnShape)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OutsideChoiceShapeError {
    MissingOutsideGameFrom,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OutsideGameChoiceShape<'a> {
    pub(crate) reveal_filter: &'a [OwnedLexToken],
    pub(crate) choose_filter: &'a [OwnedLexToken],
}

pub(crate) fn parse_outside_game_choice_shape<'a>(
    first: &'a [OwnedLexToken],
    second: &[OwnedLexToken],
) -> Result<Option<OutsideGameChoiceShape<'a>>, OutsideChoiceShapeError> {
    if !exact_surface(
        trim_lexed_commas(second),
        &["put", "that", "card", "into", "your", "hand"],
    ) {
        return Ok(None);
    }

    let first = trim_lexed_commas(first);
    let words = parser_token_word_refs(first);
    let Some(or_word) = atom_offset(&words, "or") else {
        return Ok(None);
    };
    if or_word == 0 || or_word + 1 >= words.len() {
        return Ok(None);
    }
    let reveal_words = &words[..or_word];
    let choose_words = &words[or_word + 1..];
    if !has_atom(reveal_words, "outside") || !has_atom(reveal_words, "game") {
        return Ok(None);
    }
    let face_up = has_atom(choose_words, "face-up")
        || has_atom(choose_words, "faceup")
        || has_sequence(choose_words, &["face", "up"]);
    if !face_up || !has_atom(choose_words, "exile") {
        return Ok(None);
    }

    let Some(from_word) = atom_offset(reveal_words, "from") else {
        return Err(OutsideChoiceShapeError::MissingOutsideGameFrom);
    };
    if from_word < 3 || choose_words.len() < 2 {
        return Ok(None);
    }
    let reveal_filter = token_slice_for_words(first, 3..from_word)
        .ok_or(OutsideChoiceShapeError::MissingOutsideGameFrom)?;
    let choose_filter = token_slice_for_words(first, or_word + 2..words.len())
        .ok_or(OutsideChoiceShapeError::MissingOutsideGameFrom)?;
    Ok(Some(OutsideGameChoiceShape {
        reveal_filter: trim_lexed_commas(reveal_filter),
        choose_filter: trim_lexed_commas(choose_filter),
    }))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OutsideGameWishShape {
    pub(crate) filter_tokens: Vec<OwnedLexToken>,
    pub(crate) exile_source: bool,
}

pub(crate) fn parse_outside_game_wish_shape(
    tokens: &[OwnedLexToken],
) -> Option<OutsideGameWishShape> {
    let tokens = trim_lexed_commas(tokens);
    let words = parser_token_word_refs(tokens);
    if !has_atom(&words, "outside") || !has_atom(&words, "game") {
        return None;
    }
    let reveal_word = atom_offset(&words, "reveal")?;
    let from_word = atom_offset(&words, "from")?;
    if from_word <= reveal_word + 1 {
        return None;
    }
    let put_word = sequence_offset(&words, &["and", "put", "it", "into", "your", "hand"])?;
    if put_word <= from_word {
        return None;
    }

    let mut filter_end = from_word;
    let filter_words = &words[reveal_word + 1..filter_end];
    let ownership_in_filter = has_sequence(filter_words, &["you", "own"]);
    let ownership_in_source = has_sequence(&words[from_word..put_word], &["you", "own"]);
    if !ownership_in_filter && !ownership_in_source {
        return None;
    }
    while filter_end > reveal_word + 1 {
        let trailing = words[filter_end - 1];
        if trailing != "you" && trailing != "own" {
            break;
        }
        filter_end -= 1;
    }
    let filter_tokens = token_slice_for_words(tokens, reveal_word + 1..filter_end)?.to_vec();
    let exile_source = has_atom(words.get(put_word + 6..).unwrap_or_default(), "exile");
    Some(OutsideGameWishShape {
        filter_tokens,
        exile_source,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ForEachChosenShape<'a> {
    pub(crate) body: &'a [OwnedLexToken],
}

pub(crate) fn parse_for_each_chosen_shape(
    tokens: &[OwnedLexToken],
) -> Option<ForEachChosenShape<'_>> {
    let words = parser_token_word_refs(tokens);
    if words.len() < 5 {
        return None;
    }
    let prefix_ok = consume_head(&words, &["for", "each", "of", "those"]).is_some()
        || consume_head(&words, &["for", "each", "of", "them"]).is_some();
    if !prefix_ok {
        return None;
    }
    let (_, body) =
        primitives::split_lexed_once_on_separator(tokens, || primitives::comma().void())?;
    let body = trim_lexed_commas(body);
    (!body.is_empty()).then_some(ForEachChosenShape { body })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RevealedHandPlayer {
    TargetPlayer,
    TargetOpponent,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DiscardRevealChoiceShape<'a> {
    pub(crate) revealed_player: RevealedHandPlayer,
    pub(crate) choose_clause: &'a [OwnedLexToken],
}

pub(crate) fn parse_discard_reveal_choice_shape<'a>(
    first: &[OwnedLexToken],
    second: &'a [OwnedLexToken],
    third: &[OwnedLexToken],
) -> Option<DiscardRevealChoiceShape<'a>> {
    if !exact_surface(first, &["discard", "any", "number", "of", "cards"])
        || !exact_surface(third, &["that", "player", "discards", "those", "cards"])
    {
        return None;
    }
    let (reveal, choose_clause) =
        primitives::split_lexed_once_on_separator(second, || primitives::kw("then").void())?;
    let reveal = trim_lexed_commas(reveal);
    let revealed_player =
        if exact_surface(reveal, &["target", "player", "reveals", "their", "hand"]) {
            RevealedHandPlayer::TargetPlayer
        } else if exact_surface(reveal, &["target", "opponent", "reveals", "their", "hand"]) {
            RevealedHandPlayer::TargetOpponent
        } else {
            return None;
        };
    Some(DiscardRevealChoiceShape {
        revealed_player,
        choose_clause: trim_lexed_commas(choose_clause),
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SelectedHandDoubleChoiceShape<'a> {
    pub(crate) revealed_player: RevealedHandPlayer,
    pub(crate) choice_prefix: &'a [OwnedLexToken],
    pub(crate) first_choice: &'a [OwnedLexToken],
    pub(crate) second_choice: &'a [OwnedLexToken],
}

/// Recognize a revealed-hand instruction that selects two independently
/// filtered cards before discarding the combined selection. Keeping the two
/// filter spans distinct prevents a conjunction from being collapsed into a
/// single, over-constrained object filter.
pub(crate) fn parse_selected_hand_double_choice_shape<'a>(
    first: &[OwnedLexToken],
    second: &'a [OwnedLexToken],
    third: &[OwnedLexToken],
) -> Option<SelectedHandDoubleChoiceShape<'a>> {
    let revealed_player = if exact_surface(first, &["target", "player", "reveals", "their", "hand"])
    {
        RevealedHandPlayer::TargetPlayer
    } else if exact_surface(first, &["target", "opponent", "reveals", "their", "hand"]) {
        RevealedHandPlayer::TargetOpponent
    } else {
        return None;
    };
    if !exact_surface(third, &["that", "player", "discards", "those", "cards"]) {
        return None;
    }

    let words = parser_token_word_refs(second);
    consume_head(&words, &["you", "choose", "from", "it"])?;
    let choice_start = 4;
    let separator = sequence_offset(words.get(choice_start..)?, &["and"])? + choice_start;
    if separator == choice_start || separator + 1 >= words.len() {
        return None;
    }

    Some(SelectedHandDoubleChoiceShape {
        revealed_player,
        choice_prefix: token_slice_for_words(second, 0..choice_start)?,
        first_choice: token_slice_for_words(second, choice_start..separator)?,
        second_choice: token_slice_for_words(second, separator + 1..words.len())?,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ChosenCounterAction {
    PutOrRemove,
    PutAdditional,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ChosenCounterTarget<'a> {
    PermanentOrSuspendedCard,
    Clause(&'a [OwnedLexToken]),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ChosenCounterBundleShape<'a> {
    pub(crate) action: ChosenCounterAction,
    pub(crate) target: ChosenCounterTarget<'a>,
}

pub(crate) fn parse_chosen_counter_bundle_shape<'a>(
    first: &'a [OwnedLexToken],
    second: &[OwnedLexToken],
) -> Option<ChosenCounterBundleShape<'a>> {
    let words = parser_token_word_refs(first);
    let target_words = consume_head(&words, &["choose", "a", "counter", "on"])?;
    if target_words.is_empty() {
        return None;
    }
    let action = if exact_surface(
        second,
        &[
            "remove",
            "that",
            "counter",
            "from",
            "that",
            "permanent",
            "or",
            "card",
            "or",
            "put",
            "another",
            "of",
            "those",
            "counters",
            "on",
            "it",
        ],
    ) {
        ChosenCounterAction::PutOrRemove
    } else if exact_surface(
        second,
        &[
            "put",
            "an",
            "additional",
            "counter",
            "of",
            "that",
            "kind",
            "on",
            "that",
            "permanent",
        ],
    ) || exact_surface(
        second,
        &[
            "put",
            "an",
            "additional",
            "counter",
            "of",
            "that",
            "kind",
            "on",
            "it",
        ],
    ) {
        ChosenCounterAction::PutAdditional
    } else {
        return None;
    };

    let target = if complete(
        target_words,
        sequence(&["target", "permanent", "or", "suspended", "card"]),
    )
    .is_some()
    {
        ChosenCounterTarget::PermanentOrSuspendedCard
    } else {
        let target = token_slice_for_words(first, 4..words.len())?;
        ChosenCounterTarget::Clause(trim_lexed_commas(target))
    };
    Some(ChosenCounterBundleShape { action, target })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RevealUntilLandPlayer {
    TargetPlayer,
    TargetOpponent,
    ThatPlayer,
    DefendingPlayer,
}

pub(crate) fn parse_reveal_until_land_player(
    tokens: &[OwnedLexToken],
) -> Option<RevealUntilLandPlayer> {
    let words = parser_token_word_refs(tokens);
    let tail = &[
        "reveals",
        "cards",
        "from",
        "the",
        "top",
        "of",
        "their",
        "library",
        "until",
        "they",
        "reveal",
        "a",
        "land",
        "card",
        "then",
        "puts",
        "those",
        "cards",
        "into",
        "their",
        "graveyard",
    ];
    for player in [
        RevealUntilLandPlayer::TargetPlayer,
        RevealUntilLandPlayer::TargetOpponent,
        RevealUntilLandPlayer::ThatPlayer,
        RevealUntilLandPlayer::DefendingPlayer,
    ] {
        let prefix: &'static [&'static str] = match player {
            RevealUntilLandPlayer::TargetPlayer => &["target", "player"],
            RevealUntilLandPlayer::TargetOpponent => &["target", "opponent"],
            RevealUntilLandPlayer::ThatPlayer => &["that", "player"],
            RevealUntilLandPlayer::DefendingPlayer => &["defending", "player"],
        };
        let Some(remainder) = consume_head(&words, prefix) else {
            continue;
        };
        if complete(remainder, sequence(tail)).is_some() {
            return Some(player);
        }
    }
    None
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ConsultBattlefieldFollowupShape {
    pub(crate) order: LibraryBottomOrderAst,
    pub(crate) enters_tapped: bool,
}

pub(crate) fn parse_consult_battlefield_followup_shape(
    tokens: &[OwnedLexToken],
) -> Option<ConsultBattlefieldFollowupShape> {
    let words = parser_token_word_refs(tokens);
    consume_head(&words, &["put", "those"])?;
    for required in ["battlefield", "rest", "bottom", "library"] {
        if !has_atom(&words, required) {
            return None;
        }
    }
    let order = if has_sequence(&words, &["random", "order"]) {
        LibraryBottomOrderAst::Random
    } else if has_sequence(&words, &["any", "order"]) {
        LibraryBottomOrderAst::ChooserChooses
    } else {
        return None;
    };
    Some(ConsultBattlefieldFollowupShape {
        order,
        enters_tapped: has_atom(&words, "tapped"),
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LifeBidShape<'a> {
    pub(crate) target: &'a [OwnedLexToken],
}

pub(crate) fn parse_life_bid_shape(tokens: &[OwnedLexToken]) -> Option<LifeBidShape<'_>> {
    let sentences = split_lexed_sentences(tokens);
    let [first, start, top, stands, reward] = sentences.as_slice() else {
        return None;
    };
    let first_words = parser_token_word_refs(first);
    consume_head(
        &first_words,
        &[
            "each", "player", "may", "bid", "life", "for", "control", "of",
        ],
    )?;
    if !exact_surface(
        start,
        &[
            "you", "start", "the", "bidding", "with", "a", "bid", "of", "0",
        ],
    ) || !exact_surface(
        top,
        &[
            "in", "turn", "order", "each", "player", "may", "top", "the", "high", "bid",
        ],
    ) || !exact_surface(
        stands,
        &[
            "the", "bidding", "ends", "if", "the", "high", "bid", "stands",
        ],
    ) || !exact_surface(
        reward,
        &[
            "the", "high", "bidder", "loses", "life", "equal", "to", "the", "high", "bid", "and",
            "gains", "control", "of", "the", "creature",
        ],
    ) {
        return None;
    }
    let control_word = sequence_offset(&first_words, &["control", "of"])?;
    let target = token_slice_for_words(first, control_word + 2..first_words.len())?;
    Some(LifeBidShape {
        target: trim_lexed_commas(target),
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RegenerateControlShape<'a> {
    pub(crate) regenerate_target: &'a [OwnedLexToken],
    pub(crate) control_target: &'a [OwnedLexToken],
}

pub(crate) fn parse_regenerate_control_shape<'a>(
    first: &'a [OwnedLexToken],
    second: &'a [OwnedLexToken],
) -> Option<RegenerateControlShape<'a>> {
    let first_words = parser_token_word_refs(first);
    let regenerate_words = consume_head(&first_words, &["regenerate"])?;
    if regenerate_words.is_empty() {
        return None;
    }
    let regenerate_target = token_slice_for_words(first, 1..first_words.len())?;

    let second_words = parser_token_word_refs(second);
    let mut target_word = if consume_head(&second_words, &["you", "gain", "control"]).is_some() {
        3
    } else if consume_head(&second_words, &["gain", "control"]).is_some() {
        2
    } else {
        return None;
    };
    if second_words.get(target_word).copied() == Some("of") {
        target_word += 1;
    }
    let suffix_word = sequence_offset(&second_words, &["if", "it", "regenerates", "this", "way"])
        .or_else(|| {
        sequence_offset(
            &second_words,
            &["if", "that", "creature", "regenerates", "this", "way"],
        )
    })?;
    if suffix_word <= target_word {
        return None;
    }
    let control_target = token_slice_for_words(second, target_word..suffix_word)?;
    Some(RegenerateControlShape {
        regenerate_target: trim_lexed_commas(regenerate_target),
        control_target: trim_lexed_commas(control_target),
    })
}

fn slot_separator<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    let commas = || repeat::<_, _, (), _, _>(0.., primitives::comma().void());
    alt((
        (
            repeat::<_, _, (), _, _>(1.., primitives::comma().void()),
            opt(primitives::kw("and").void()),
            commas(),
            peek(alt((primitives::kw("a"), primitives::kw("an")))),
        )
            .void(),
        (
            primitives::kw("and").void(),
            commas(),
            peek(alt((primitives::kw("a"), primitives::kw("an")))),
        )
            .void(),
    ))
    .parse_next(input)
}

fn slot_item<'a>(input: &mut LexStream<'a>) -> WResult<&'a [OwnedLexToken]> {
    let item = (|input: &mut LexStream<'a>| {
        while !input.is_empty() && peek(slot_separator).parse_next(input).is_err() {
            any.parse_next(input)?;
        }
        Ok(())
    })
    .take()
    .parse_next(input)?;
    if !input.is_empty() {
        slot_separator.parse_next(input)?;
    }
    Ok(item)
}

fn parse_slot_items(tokens: &[OwnedLexToken]) -> Option<Vec<Vec<OwnedLexToken>>> {
    let mut input = LexStream::new(tokens);
    let mut items = Vec::new();
    while !input.is_empty() {
        let item = slot_item.parse_next(&mut input).ok()?;
        let item = trim_lexed_commas(item);
        if item.is_empty() {
            return None;
        }
        items.push(item.to_vec());
    }
    (items.len() >= 2).then_some(items)
}

pub(crate) fn parse_explicit_card_name_surface_tokens(tokens: &[OwnedLexToken]) -> Option<String> {
    let words = parser_token_word_refs(tokens);
    let named_word = atom_offset(&words, "named")?;
    let name_start = named_word.checked_add(1)?;
    if name_start >= words.len() {
        return None;
    }
    let name_tokens = token_slice_for_words(tokens, name_start..words.len())?;
    let name = render_token_slice(name_tokens).trim().to_string();
    (!name.is_empty()).then_some(name)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SearchLibrarySlotsShape {
    pub(crate) multi_zone: bool,
    pub(crate) filters: Vec<Vec<OwnedLexToken>>,
}

pub(crate) fn parse_search_library_slots_shape(
    tokens: &[OwnedLexToken],
) -> Option<SearchLibrarySlotsShape> {
    let words = parser_token_word_refs(tokens);
    let (multi_zone, for_word) =
        if consume_head(&words, &["search", "your", "library", "for"]).is_some() {
            (false, 3)
        } else if consume_head(
            &words,
            &["search", "your", "library", "and", "graveyard", "for"],
        )
        .is_some()
            || consume_head(
                &words,
                &["search", "your", "library", "or", "graveyard", "for"],
            )
            .is_some()
        {
            (true, 5)
        } else if consume_head(
            &words,
            &["search", "your", "library", "and", "or", "graveyard", "for"],
        )
        .is_some()
        {
            (true, 6)
        } else {
            return None;
        };

    let (reveal_word, reveal_len) =
        if let Some(offset) = sequence_offset(&words, &["reveal", "those", "cards"]) {
            (offset, 3)
        } else {
            (sequence_offset(&words, &["reveal", "them"])?, 2)
        };
    let tail = words.get(reveal_word + reveal_len..)?;
    if complete(
        tail,
        sequence(&["put", "them", "into", "your", "hand", "then", "shuffle"]),
    )
    .is_none()
        && complete(
            tail,
            sequence(&[
                "put", "those", "cards", "into", "your", "hand", "then", "shuffle",
            ]),
        )
        .is_none()
    {
        return None;
    }
    if reveal_word <= for_word + 1 {
        return None;
    }
    let filters = token_slice_for_words(tokens, for_word + 1..reveal_word)?;
    Some(SearchLibrarySlotsShape {
        multi_zone,
        filters: parse_slot_items(trim_lexed_commas(filters))?,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct KickedSearchLibrarySlotsShape {
    pub(crate) default_filter: Vec<OwnedLexToken>,
    pub(crate) replacement_filters: Vec<Vec<OwnedLexToken>>,
}

pub(crate) fn parse_kicked_search_library_slots_shape(
    tokens: &[OwnedLexToken],
) -> Option<KickedSearchLibrarySlotsShape> {
    let sentences = split_lexed_sentences(tokens);
    let [first, second, third] = sentences.as_slice() else {
        return None;
    };
    if !exact_surface(
        first,
        &[
            "search", "your", "library", "for", "a", "basic", "land", "card",
        ],
    ) || !exact_surface(
        third,
        &[
            "reveal", "those", "cards", "put", "them", "into", "your", "hand", "then", "shuffle",
        ],
    ) {
        return None;
    }
    let second_words = parser_token_word_refs(second);
    let replacement_words = consume_head(
        &second_words,
        &[
            "if", "this", "spell", "was", "kicked", "instead", "search", "your", "library", "for",
        ],
    )?;
    if replacement_words.is_empty() {
        return None;
    }
    let first_words = parser_token_word_refs(first);
    let default_filter = token_slice_for_words(first, 4..first_words.len())?.to_vec();
    let replacement_start = second_words.len().checked_sub(replacement_words.len())?;
    let replacement_tokens = token_slice_for_words(second, replacement_start..second_words.len())?;
    Some(KickedSearchLibrarySlotsShape {
        default_filter,
        replacement_filters: parse_slot_items(trim_lexed_commas(replacement_tokens))?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::front_end::lexer::lex_line;

    fn lex(text: &str) -> Vec<OwnedLexToken> {
        lex_line(text, 0).expect("bundle grammar fixture should lex")
    }

    #[test]
    fn alternative_cost_bundle_returns_typed_kind() {
        let first = lex("You may cast a spell with flashback from your hand.");
        let second = lex("If you do, pay its flashback cost rather than its mana cost.");
        assert_eq!(
            parse_alternative_cost_bundle_shape(&first, &second),
            Some(AlternativeCostBundleShape {
                kind: AlternativeCastKind::Flashback,
            })
        );
    }

    #[test]
    fn outside_game_choice_carries_filter_boundaries() {
        let first = lex(
            "You may reveal an Eldrazi card you own from outside the game or choose a face-up Eldrazi card you own in exile.",
        );
        let second = lex("Put that card into your hand.");
        let shape = parse_outside_game_choice_shape(&first, &second)
            .expect("shape parse should not be malformed")
            .expect("shape should match");
        assert!(has_atom(
            &parser_token_word_refs(shape.reveal_filter),
            "eldrazi"
        ));
        assert!(has_atom(
            &parser_token_word_refs(shape.choose_filter),
            "eldrazi"
        ));
    }

    #[test]
    fn chosen_counter_shape_preserves_target_and_action() {
        let first = lex("Choose a counter on target permanent.");
        let second = lex("Put an additional counter of that kind on it.");
        let shape = parse_chosen_counter_bundle_shape(&first, &second).expect("shape");
        assert_eq!(shape.action, ChosenCounterAction::PutAdditional);
        assert!(matches!(shape.target, ChosenCounterTarget::Clause(_)));
    }

    #[test]
    fn search_slot_shape_splits_article_led_filters() {
        let tokens = lex(
            "Search your library for a Plains card, an Island card, and a Swamp card, reveal those cards, put them into your hand, then shuffle.",
        );
        let shape = parse_search_library_slots_shape(&tokens).expect("shape");
        assert!(!shape.multi_zone);
        assert_eq!(shape.filters.len(), 3);
    }

    #[test]
    fn explicit_slot_name_preserves_internal_punctuation() {
        let tokens = lex("a card named Nissa, Genesis Mage");
        assert_eq!(
            parse_explicit_card_name_surface_tokens(&tokens).as_deref(),
            Some("Nissa, Genesis Mage")
        );
    }

    #[test]
    fn selected_hand_double_choice_keeps_both_filter_spans() {
        let first = lex("Target opponent reveals their hand.");
        let second = lex(
            "You choose from it a nonland card with mana value 3 or less and a card with mana value 4 or greater.",
        );
        let third = lex("That player discards those cards.");
        let shape = parse_selected_hand_double_choice_shape(&first, &second, &third)
            .expect("selected-hand double choice shape");

        assert_eq!(shape.revealed_player, RevealedHandPlayer::TargetOpponent);
        assert_eq!(
            parser_token_word_refs(shape.first_choice),
            [
                "a", "nonland", "card", "with", "mana", "value", "3", "or", "less"
            ]
        );
        assert_eq!(
            parser_token_word_refs(shape.second_choice),
            ["a", "card", "with", "mana", "value", "4", "or", "greater"]
        );
    }
}
