use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

use crate::color::Color;
use crate::mana::ManaSymbol;

use super::super::lexer::{OwnedLexToken, TokenWordView};
use super::{leaf, primitives};

const ADD_MANA_THAT_COLOR_AMOUNT_PREFIX: &[&str] =
    &["an", "amount", "of", "mana", "of", "that", "color"];
const ADD_MANA_ONE_THAT_COLOR_PREFIX: &[&str] = &["one", "mana", "of", "that", "color"];
const MANA_OF_CHOSEN_COLOR_SUFFIXES: &[&[&str]] = &[&["mana", "of", "the"], &["mana", "of"]];
const FOR_EACH_COLOR_AMONG_PHRASE: &[&str] = &["for", "each", "color", "among"];
const ADD_ONE_MANA_OF_THAT_COLOR_PHRASE: &[&str] = &["add", "one", "mana", "of", "that", "color"];
const ONE_MANA_OF_ANY_COLOR_AMONG_PHRASE: &[&str] = &["one", "mana", "of", "any", "color", "among"];
const ANY_COMBINATION_OF_PHRASE: &[&str] = &["any", "combination", "of"];
const CHOSEN_COLOR_PHRASE: &[&str] = &["chosen", "color"];
const FOR_EACH_PREFIX: &[&str] = &["for", "each"];
const CHOSEN_COLOR_MANA_TAIL_PREFIX: &[&str] =
    &["or", "one", "mana", "of", "the", "chosen", "color"];
const MANA_POOL_TAIL_WORDS: &[&str] = &[
    "to", "your", "their", "its", "that", "player", "players", "player's", "players'", "mana",
    "pool",
];
const SIMPLE_MANA_FILLER_WORDS: &[&str] = &["mana", "to", "your", "pool"];
const MANA_CHOICE_TAIL_WORDS: &[&str] = &["to", "your", "their", "its", "mana", "pool"];
const MANA_OPTION_SEPARATOR_WORDS: &[&str] = &[
    "and", "or", "and/or", "mana", "to", "your", "their", "its", "pool",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddManaChoiceKind {
    AnyOneColor,
    AnyColor,
    AnyOneType,
    AnyType,
}

impl AddManaChoiceKind {
    pub fn any_one(self) -> bool {
        matches!(self, Self::AnyOneColor | Self::AnyOneType)
    }

    pub fn allow_colorless(self) -> bool {
        matches!(self, Self::AnyOneType | Self::AnyType)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct AddManaChoiceClause<'a> {
    pub kind: AddManaChoiceKind,
    pub tail_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy)]
pub struct AddManaClauseFacts<'a> {
    pub imprinted_colors: bool,
    pub commander_identity: bool,
    pub different_colors: bool,
    pub chosen_color_reference: bool,
    pub one_that_color_tail: Option<&'a [OwnedLexToken]>,
    pub amount_that_color: bool,
    pub choice: Option<AddManaChoiceClause<'a>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixedManaOutput {
    pub mana: Vec<ManaSymbol>,
    pub has_explicit_symbol: bool,
    pub last_mana_token: Option<usize>,
    pub first_for_each_token: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FixedManaTailKind {
    ChosenColor,
    Pool,
    Instead,
    Unsupported,
}

#[derive(Debug, Clone, Copy)]
pub struct ColorsAmongSpan<'a> {
    pub filter_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy)]
pub struct AnyColorAmongSpan<'a> {
    pub filter_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy)]
pub enum LandCouldProduceShape<'a> {
    CouldProduceFilter(&'a [OwnedLexToken]),
    TriggeringEventProducedFilter(&'a [OwnedLexToken]),
    UnsupportedTrailing,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnyCombinationManaError {
    MissingColors,
    UnsupportedSymbol(String),
    NonColoredSymbol(String),
}

pub fn parse_add_mana_clause_facts(tokens: &[OwnedLexToken]) -> AddManaClauseFacts<'_> {
    let view = TokenWordView::new(tokens);
    let words = view.word_refs();
    let has_card = word_occurs(&words, &["card", "cards"]);
    let imprinted_colors =
        has_card && word_occurs(&words, &["exiled"]) && word_occurs(&words, &["colors"]);
    let commander_identity = word_occurs(&words, &["commander", "commanders"])
        && word_occurs(&words, &["color"])
        && word_occurs(&words, &["identity"]);
    let different_colors = phrase_offset(&words, &["different", "colors"]).is_some();
    let chosen_color_reference = parse_chosen_color_reference(&words);
    let one_that_color_tail = prefix_tail_tokens(tokens, &view, ADD_MANA_ONE_THAT_COLOR_PREFIX);
    let amount_that_color = phrase_is_prefix(&words, ADD_MANA_THAT_COLOR_AMOUNT_PREFIX);
    let choice = parse_add_mana_choice_clause(tokens, &view, &words);

    AddManaClauseFacts {
        imprinted_colors,
        commander_identity,
        different_colors,
        chosen_color_reference,
        one_that_color_tail,
        amount_that_color,
        choice,
    }
}

pub fn parse_fixed_mana_output(tokens: &[OwnedLexToken]) -> FixedManaOutput {
    let view = TokenWordView::new(tokens);
    let words = view.word_refs();
    let first_for_each_token = phrase_offset(&words, FOR_EACH_PREFIX)
        .and_then(|word| view.token_start_indices().get(word).copied());
    let scan_end = first_for_each_token.unwrap_or(tokens.len());
    let has_explicit_symbol = tokens
        .iter()
        .any(|token| leaf::parse_leaf_surface_mana_pip_token(token).is_some());
    let mut mana = Vec::new();
    let mut last_mana_token = None;
    for (index, token) in tokens[..scan_end].iter().enumerate() {
        if let Some(pip) = leaf::parse_leaf_surface_mana_pip_token(token) {
            mana.extend(pip.into_pip());
            last_mana_token = Some(index);
            continue;
        }
        if token_matches_any_word(token, SIMPLE_MANA_FILLER_WORDS) {
            continue;
        }
    }
    FixedManaOutput {
        mana,
        has_explicit_symbol,
        last_mana_token,
        first_for_each_token,
    }
}

pub fn classify_fixed_mana_tail(tokens: &[OwnedLexToken]) -> FixedManaTailKind {
    let words = TokenWordView::new(tokens).word_refs();
    if phrase_is_prefix(&words, CHOSEN_COLOR_MANA_TAIL_PREFIX)
        && words_after_prefix_are_allowed(
            &words,
            CHOSEN_COLOR_MANA_TAIL_PREFIX.len(),
            MANA_CHOICE_TAIL_WORDS,
        )
    {
        return FixedManaTailKind::ChosenColor;
    }
    if words_are_allowed(&words, MANA_CHOICE_TAIL_WORDS) {
        return FixedManaTailKind::Pool;
    }
    if phrase_is_complete(&words, &["instead"]) {
        return FixedManaTailKind::Instead;
    }
    FixedManaTailKind::Unsupported
}

pub fn parse_colors_among_span(tokens: &[OwnedLexToken]) -> Option<ColorsAmongSpan<'_>> {
    let view = TokenWordView::new(tokens);
    let words = view.word_refs();
    phrase_offset(&words, FOR_EACH_COLOR_AMONG_PHRASE)?;
    phrase_offset(&words, ADD_ONE_MANA_OF_THAT_COLOR_PHRASE)?;
    let among_word = word_offset(&words, &["among"])?;
    let add_word = word_offset(&words, &["add"])?;
    let among_token = view.token_start_indices().get(among_word).copied()?;
    let add_token = view.token_start_indices().get(add_word).copied()?;
    if add_token <= among_token + 1 {
        return None;
    }
    Some(ColorsAmongSpan {
        filter_tokens: trim_commas(&tokens[among_token + 1..add_token]),
    })
}

pub fn parse_any_color_among_span(tokens: &[OwnedLexToken]) -> Option<AnyColorAmongSpan<'_>> {
    let view = TokenWordView::new(tokens);
    let words = view.word_refs();
    if !phrase_is_prefix(&words, ONE_MANA_OF_ANY_COLOR_AMONG_PHRASE) {
        return None;
    }
    let filter_word = ONE_MANA_OF_ANY_COLOR_AMONG_PHRASE.len();
    let filter_token = view.token_start_indices().get(filter_word).copied()?;
    let filter_tokens = trim_commas(&tokens[filter_token..]);
    (!filter_tokens.is_empty()).then_some(AnyColorAmongSpan { filter_tokens })
}

pub fn parse_or_mana_color_choices(tokens: &[OwnedLexToken]) -> Option<Vec<Color>> {
    let mut has_or = false;
    let mut colors = Vec::new();
    for token in tokens {
        if token.is_word("or") {
            has_or = true;
            continue;
        }
        if let Some(pip) = leaf::parse_leaf_surface_mana_pip_token(token) {
            for symbol in pip.into_pip() {
                let color = mana_symbol_color(symbol)?;
                push_unique_color(&mut colors, color);
            }
            continue;
        }
        if token.as_word().is_none() {
            continue;
        }
        if token_matches_any_word(token, MANA_CHOICE_TAIL_WORDS) {
            continue;
        }
        return None;
    }
    if !has_or || colors.len() < 2 {
        return None;
    }
    Some(colors)
}

pub fn parse_any_combination_mana_colors(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<Color>>, AnyCombinationManaError> {
    let words = TokenWordView::new(tokens).word_refs();
    let Some(offset) = phrase_offset(&words, ANY_COMBINATION_OF_PHRASE) else {
        return Ok(None);
    };
    let mut input: primitives::WordSliceInput<'_> = &words[offset + 3..];
    let mut colors = Vec::new();
    while let Ok(word) = take_word(&mut input) {
        if word == "where" {
            break;
        }
        if word_is_any(word, MANA_OPTION_SEPARATOR_WORDS) {
            continue;
        }
        if word_is_any(word, &["color", "colors"]) {
            for color in Color::ALL {
                push_unique_color(&mut colors, color);
            }
            continue;
        }
        let symbol = leaf::parse_leaf_mana_symbol_complete(word)
            .map_err(|_| AnyCombinationManaError::UnsupportedSymbol(word.to_string()))?;
        let color = mana_symbol_color(symbol)
            .ok_or_else(|| AnyCombinationManaError::NonColoredSymbol(word.to_string()))?;
        push_unique_color(&mut colors, color);
    }
    if colors.is_empty() {
        return Err(AnyCombinationManaError::MissingColors);
    }
    Ok(Some(colors))
}

pub fn parse_any_combination_mana_tail(tokens: &[OwnedLexToken]) -> Option<&[OwnedLexToken]> {
    let words = TokenWordView::new(tokens).word_refs();
    let offset = phrase_offset(&words, ANY_COMBINATION_OF_PHRASE)?;
    let where_word = words
        .iter()
        .enumerate()
        .skip(offset + 3)
        .find_map(|(index, word)| (*word == "where").then_some(index))?;
    let where_token = TokenWordView::new(tokens)
        .token_start_indices()
        .get(where_word)
        .copied()?;
    tokens.get(where_token..)
}

pub fn is_mana_pool_tail(tokens: &[OwnedLexToken]) -> bool {
    let words = TokenWordView::new(tokens).word_refs();
    phrase_is_prefix(&words, &["to"])
        && word_occurs(&words, &["mana"])
        && word_occurs(&words, &["pool"])
        && words_are_allowed(&words, MANA_POOL_TAIL_WORDS)
}

pub fn parse_land_could_produce_shape(
    tokens: &[OwnedLexToken],
) -> Option<LandCouldProduceShape<'_>> {
    let view = TokenWordView::new(tokens);
    let words = view.word_refs();
    if words.len() < 3 || !phrase_is_prefix(&words, &["that"]) {
        return None;
    }
    let (marker_word, marker_length, triggering_event) =
        if let Some(offset) = phrase_offset(&words, &["could", "produce"]) {
            (offset, 2, false)
        } else {
            (word_offset(&words, &["produced"])?, 1, true)
        };
    if marker_word + marker_length != words.len() {
        return Some(LandCouldProduceShape::UnsupportedTrailing);
    }
    let marker_token = view.token_start_indices().get(marker_word).copied()?;
    let filter = trim_leading_commas(&tokens[1..marker_token]);
    if triggering_event {
        Some(LandCouldProduceShape::TriggeringEventProducedFilter(filter))
    } else {
        Some(LandCouldProduceShape::CouldProduceFilter(filter))
    }
}

pub fn is_player_choice_tail(tokens: &[OwnedLexToken]) -> bool {
    let words = TokenWordView::new(tokens).word_refs();
    const PHRASES: &[&[&str]] = &[
        &["they", "choose"],
        &["that", "player", "chooses"],
        &["they", "choose", "to", "their", "mana", "pool"],
        &["that", "player", "chooses", "to", "their", "mana", "pool"],
    ];
    PHRASES
        .iter()
        .any(|phrase| phrase_is_complete(&words, phrase))
}

pub fn is_removed_this_way_tail(tokens: &[OwnedLexToken]) -> bool {
    let words = TokenWordView::new(tokens).word_refs();
    phrase_is_prefix(&words, &["for", "each"])
        && phrase_is_suffix(&words, &["removed", "this", "way"])
}

pub fn is_among_tail(tokens: &[OwnedLexToken]) -> bool {
    let words = TokenWordView::new(tokens).word_refs();
    phrase_is_prefix(&words, &["among"])
}

pub fn is_instead_if_tail(tokens: &[OwnedLexToken]) -> bool {
    let words = TokenWordView::new(tokens).word_refs();
    phrase_is_prefix(&words, &["instead", "if"])
}

fn parse_add_mana_choice_clause<'a>(
    tokens: &'a [OwnedLexToken],
    view: &TokenWordView<'a>,
    words: &[&str],
) -> Option<AddManaChoiceClause<'a>> {
    const CHOICES: &[(&[&str], AddManaChoiceKind)] = &[
        (&["any", "one", "color"], AddManaChoiceKind::AnyOneColor),
        (&["any", "color"], AddManaChoiceKind::AnyColor),
        (&["one", "color"], AddManaChoiceKind::AnyColor),
        (&["any", "one", "type"], AddManaChoiceKind::AnyOneType),
        (&["any", "type"], AddManaChoiceKind::AnyType),
        (&["one", "type"], AddManaChoiceKind::AnyType),
    ];
    let mut best: Option<(usize, usize, AddManaChoiceKind)> = None;
    for (phrase, kind) in CHOICES {
        let Some(offset) = phrase_offset(words, phrase) else {
            continue;
        };
        if best.is_none_or(|(best_offset, _, _)| offset < best_offset) {
            best = Some((offset, phrase.len(), *kind));
        }
    }
    let (offset, length, kind) = best?;
    let tail_start = view.token_index_after_words(offset + length)?;
    Some(AddManaChoiceClause {
        kind,
        tail_tokens: trim_leading_commas(&tokens[tail_start..]),
    })
}

fn parse_chosen_color_reference(words: &[&str]) -> bool {
    let Some(chosen) = phrase_offset(words, CHOSEN_COLOR_PHRASE) else {
        return false;
    };
    let prefix = &words[..chosen];
    let suffix_matches = MANA_OF_CHOSEN_COLOR_SUFFIXES
        .iter()
        .any(|suffix| phrase_is_suffix(prefix, suffix));
    suffix_matches
        && words_after_prefix_are_allowed(
            words,
            chosen + CHOSEN_COLOR_PHRASE.len(),
            MANA_POOL_TAIL_WORDS,
        )
}

fn prefix_tail_tokens<'a>(
    tokens: &'a [OwnedLexToken],
    view: &TokenWordView<'a>,
    prefix: &[&str],
) -> Option<&'a [OwnedLexToken]> {
    let words = view.word_refs();
    if !phrase_is_prefix(&words, prefix) {
        return None;
    }
    let tail = view.token_index_after_words(prefix.len())?;
    Some(trim_leading_commas(&tokens[tail..]))
}

fn phrase_offset(words: &[&str], expected: &[&str]) -> Option<usize> {
    let mut input: primitives::WordSliceInput<'_> = words;
    let initial_len = input.len();
    loop {
        let offset = initial_len.saturating_sub(input.len());
        let mut candidate = input;
        if parse_phrase(&mut candidate, expected).is_ok() {
            return Some(offset);
        }
        take_word(&mut input).ok()?;
    }
}

fn word_offset(words: &[&str], expected: &[&str]) -> Option<usize> {
    let mut input: primitives::WordSliceInput<'_> = words;
    let initial_len = input.len();
    loop {
        let offset = initial_len.saturating_sub(input.len());
        let word = take_word(&mut input).ok()?;
        if word_is_any(word, expected) {
            return Some(offset);
        }
    }
}

fn word_occurs(words: &[&str], expected: &[&str]) -> bool {
    word_offset(words, expected).is_some()
}

fn phrase_is_prefix(words: &[&str], expected: &[&str]) -> bool {
    let mut input: primitives::WordSliceInput<'_> = words;
    parse_phrase(&mut input, expected).is_ok()
}

fn phrase_is_complete(words: &[&str], expected: &[&str]) -> bool {
    let mut input: primitives::WordSliceInput<'_> = words;
    parse_phrase(&mut input, expected).is_ok() && input.is_empty()
}

fn phrase_is_suffix(words: &[&str], expected: &[&str]) -> bool {
    let Some(offset) = words.len().checked_sub(expected.len()) else {
        return false;
    };
    phrase_is_complete(&words[offset..], expected)
}

fn words_after_prefix_are_allowed(words: &[&str], offset: usize, allowed: &[&str]) -> bool {
    let Some(tail) = words.get(offset..) else {
        return false;
    };
    words_are_allowed(tail, allowed)
}

fn words_are_allowed(words: &[&str], allowed: &[&str]) -> bool {
    let mut input: primitives::WordSliceInput<'_> = words;
    while let Ok(word) = take_word(&mut input) {
        if !word_is_any(word, allowed) {
            return false;
        }
    }
    true
}

fn parse_phrase<'a>(input: &mut primitives::WordSliceInput<'a>, expected: &[&str]) -> WResult<()> {
    for expected_word in expected {
        let actual = take_word(input)?;
        if actual != *expected_word {
            return Err(primitives::backtrack_err(
                "mana clause",
                "expected word phrase",
            ));
        }
    }
    Ok(())
}

fn take_word<'word>(input: &mut &[&'word str]) -> WResult<&'word str> {
    any.parse_next(input)
}

fn word_is_any(word: &str, expected: &[&str]) -> bool {
    expected.contains(&word)
}

fn token_matches_any_word(token: &OwnedLexToken, expected: &[&str]) -> bool {
    expected.iter().any(|candidate| token.is_word(candidate))
}

fn mana_symbol_color(symbol: ManaSymbol) -> Option<Color> {
    match symbol {
        ManaSymbol::White => Some(Color::White),
        ManaSymbol::Blue => Some(Color::Blue),
        ManaSymbol::Black => Some(Color::Black),
        ManaSymbol::Red => Some(Color::Red),
        ManaSymbol::Green => Some(Color::Green),
        _ => None,
    }
}

fn push_unique_color(colors: &mut Vec<Color>, color: Color) {
    if !colors.contains(&color) {
        colors.push(color);
    }
}

fn trim_leading_commas(mut tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    while tokens.first().is_some_and(OwnedLexToken::is_comma) {
        tokens = &tokens[1..];
    }
    tokens
}

fn trim_commas(mut tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    while tokens.first().is_some_and(OwnedLexToken::is_comma) {
        tokens = &tokens[1..];
    }
    while tokens.last().is_some_and(OwnedLexToken::is_comma) {
        tokens = &tokens[..tokens.len() - 1];
    }
    tokens
}

#[cfg(test)]
mod tests {
    use super::super::super::lexer::lex_line;
    use super::*;

    #[test]
    fn typed_add_mana_facts_preserve_choice_and_chosen_color_tails() {
        let choice = lex_line("any one type that a land you control could produce", 0).unwrap();
        let facts = parse_add_mana_clause_facts(&choice);
        let parsed_choice = facts.choice.unwrap();
        assert_eq!(parsed_choice.kind, AddManaChoiceKind::AnyOneType);
        assert_eq!(
            TokenWordView::new(parsed_choice.tail_tokens).word_refs(),
            ["that", "a", "land", "you", "control", "could", "produce"]
        );

        let chosen = lex_line("one mana of that color to your mana pool", 0).unwrap();
        let facts = parse_add_mana_clause_facts(&chosen);
        assert!(facts.one_that_color_tail.is_some());
        assert!(is_mana_pool_tail(facts.one_that_color_tail.unwrap()));
    }

    #[test]
    fn typed_land_production_shape_preserves_filter_span_and_rejects_trailing_words() {
        let tokens = lex_line("that a land you control could produce", 0).unwrap();
        let LandCouldProduceShape::CouldProduceFilter(filter) =
            parse_land_could_produce_shape(&tokens).unwrap()
        else {
            panic!("expected a land-production filter");
        };
        assert_eq!(
            TokenWordView::new(filter).word_refs(),
            ["a", "land", "you", "control"]
        );

        let produced = lex_line("that land produced", 0).unwrap();
        let LandCouldProduceShape::TriggeringEventProducedFilter(filter) =
            parse_land_could_produce_shape(&produced).unwrap()
        else {
            panic!("expected a triggering-event production filter");
        };
        assert_eq!(TokenWordView::new(filter).word_refs(), ["land"]);

        let trailing = lex_line("that a land could produce this turn", 0).unwrap();
        assert!(matches!(
            parse_land_could_produce_shape(&trailing),
            Some(LandCouldProduceShape::UnsupportedTrailing)
        ));
    }

    #[test]
    fn typed_mana_choice_parsers_return_colors_and_fixed_output_boundaries() {
        let options = lex_line("any combination of w u and b mana", 0).unwrap();
        assert_eq!(
            parse_any_combination_mana_colors(&options).unwrap(),
            Some(vec![Color::White, Color::Blue, Color::Black])
        );

        let fixed = lex_line("{W} {U} for each creature you control", 0).unwrap();
        let output = parse_fixed_mana_output(&fixed);
        assert_eq!(output.mana, vec![ManaSymbol::White, ManaSymbol::Blue]);
        assert_eq!(output.first_for_each_token, Some(2));
    }

    #[test]
    fn any_color_among_parser_returns_the_dynamic_filter_span() {
        let tokens = lex_line(
            "one mana of any color among legendary permanents you control",
            0,
        )
        .unwrap();
        let span = parse_any_color_among_span(&tokens).expect("expected any-color-among span");
        assert_eq!(
            TokenWordView::new(span.filter_tokens).word_refs(),
            ["legendary", "permanents", "you", "control"]
        );

        let unrestricted = lex_line("one mana of any color", 0).unwrap();
        assert!(parse_any_color_among_span(&unrestricted).is_none());
    }
}
