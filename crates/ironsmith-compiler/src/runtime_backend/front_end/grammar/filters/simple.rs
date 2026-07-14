use winnow::combinator::alt;
use winnow::error::{ContextError, ErrMode, ModalResult as WResult};
use winnow::prelude::*;

use crate::filter::{AlternativeCastKind, ObjectFilterUnionConnective, StackObjectKind};
use crate::{CardType, ColorSet, ObjectFilter, PlayerFilter, Subtype, Supertype, Zone};

use super::super::primitives::{self, WordSliceInput, parse_full_word_slice};
use crate::runtime_backend::lexer::{OwnedLexToken, TokenKind, TokenWordView};
use crate::runtime_backend::util::{
    is_non_outlaw_word, is_outlaw_word, is_permanent_type, non_article_word_refs,
    parse_alternative_cast_words, parse_card_type, parse_color, parse_non_color, parse_non_subtype,
    parse_non_supertype, parse_non_type, parse_subtype_flexible, parse_supertype_word,
    push_outlaw_subtypes,
};

type WordInput<'a> = WordSliceInput<'a>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FilterFaceState {
    FaceDown,
    FaceUp,
}

impl FilterFaceState {
    fn is_face_down(self) -> bool {
        matches!(self, Self::FaceDown)
    }
}

#[derive(Debug, Clone, PartialEq)]
enum SimpleObjectFilterSuffix {
    Controller(PlayerFilter),
    Owner(PlayerFilter),
    ControllerOwner(PlayerFilter, PlayerFilter),
    OwnerZone(PlayerFilter, Zone),
    Zone(Zone),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NamedObjectFilterAtom {
    ChosenColor,
    ChosenType,
    NonChosenType,
}

#[derive(Debug, Clone, PartialEq)]
enum SimpleObjectFilterAtom {
    TypeListSeparator(TypeListSeparator),
    AlternativeCast(AlternativeCastKind),
    FaceState(FilterFaceState),
    Other,
    Token,
    Nontoken,
    Historic,
    Nonhistoric,
    Modified,
    Colorless,
    Multicolored,
    Monocolored,
    CardMarker,
    PermanentMarker,
    SpellMarker,
    Named(NamedObjectFilterAtom),
    CardType(CardType),
    ExcludedCardType(CardType),
    Subtype(Subtype),
    ExcludedSubtype(Subtype),
    Supertype(Supertype),
    ExcludedSupertype(Supertype),
    Color(ColorSet),
    ExcludedColor(ColorSet),
    Outlaw,
    NonOutlaw,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TypeListSeparator {
    Conjunction,
    Disjunction,
    AndOr,
}

impl TypeListSeparator {
    fn is_disjunction(self) -> bool {
        matches!(self, Self::Disjunction | Self::AndOr)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExcludedObjectFilterAtom {
    Separator,
    CardType(CardType),
    Subtype(Subtype),
    Supertype(Supertype),
    Color(ColorSet),
    Outlaw,
}

#[derive(Debug, Clone, Copy)]
struct OtherThanSplit<'a> {
    base: &'a [&'a str],
    exclusions: &'a [&'a str],
}

pub(crate) fn parse_filter_face_state_words(words: &[&str]) -> Option<(bool, usize)> {
    let mut input: WordInput<'_> = words;
    let state = parse_filter_face_state.parse_next(&mut input).ok()?;
    Some((state.is_face_down(), words.len().checked_sub(input.len())?))
}

pub(crate) fn parse_simple_object_filter_words(
    input_words: &[&str],
    other: bool,
) -> Option<ObjectFilter> {
    parse_simple_object_filter_words_with_list_marker(input_words, other, false)
}

pub(crate) fn parse_simple_object_filter_lexed(
    tokens: &[OwnedLexToken],
    other: bool,
) -> Option<ObjectFilter> {
    let word_view = TokenWordView::new(tokens);
    let saw_type_list_separator = tokens.iter().any(|token| token.kind == TokenKind::Comma);
    parse_simple_object_filter_words_with_list_marker(
        &word_view.to_word_refs(),
        other,
        saw_type_list_separator,
    )
}

fn parse_simple_object_filter_words_with_list_marker(
    input_words: &[&str],
    other: bool,
    saw_type_list_separator: bool,
) -> Option<ObjectFilter> {
    let mut words = non_article_word_refs(input_words);
    words.retain(|word| *word != "instead");
    if words.is_empty() {
        return None;
    }

    let parsed_suffix = parse_simple_object_filter_suffix(&words);
    if let Some((_suffix, suffix_len)) = parsed_suffix.as_ref() {
        words.truncate(words.len().saturating_sub(*suffix_len));
    }
    if contains_simple_filter_reject(&words) {
        return None;
    }

    if let Some(split) = parse_other_than_split(&words) {
        let mut filter = parse_other_than_filter(split, other)?;
        if let Some((suffix, _suffix_len)) = parsed_suffix {
            apply_simple_object_filter_suffix(&mut filter, suffix);
        }
        Some(filter)
    } else {
        parse_simple_filter_body(
            &words,
            other,
            saw_type_list_separator,
            parsed_suffix.map(|(suffix, _suffix_len)| suffix),
        )
    }
}

fn parse_simple_filter_body(
    words: &[&str],
    other: bool,
    initial_type_list_separator: bool,
    suffix: Option<SimpleObjectFilterSuffix>,
) -> Option<ObjectFilter> {
    let mut input: WordInput<'_> = words;
    let mut filter = ObjectFilter::default();
    filter.other = other;
    if let Some(suffix) = suffix {
        apply_simple_object_filter_suffix(&mut filter, suffix);
    }

    let mut saw_permanent_type = false;
    let mut saw_spell = false;
    let mut saw_card = false;
    let mut saw_permanent = false;
    let mut saw_type_list_conjunction = initial_type_list_separator;
    let mut pending_type_separator = None;
    let mut last_type_atom_is_card_type = None;
    let mut saw_type_subtype_disjunction = false;
    let mut saw_and_or_union = false;

    while !input.is_empty() {
        let atom = parse_simple_object_filter_atom
            .parse_next(&mut input)
            .ok()?;
        match atom {
            SimpleObjectFilterAtom::TypeListSeparator(separator) => {
                saw_type_list_conjunction = true;
                saw_and_or_union |= separator == TypeListSeparator::AndOr;
                pending_type_separator = Some(separator);
            }
            SimpleObjectFilterAtom::AlternativeCast(kind) => {
                filter.alternative_cast = Some(kind);
                saw_spell = true;
            }
            SimpleObjectFilterAtom::FaceState(state) => {
                filter.face_down = Some(state.is_face_down());
            }
            SimpleObjectFilterAtom::Other => filter.other = true,
            SimpleObjectFilterAtom::Token => filter.token = true,
            SimpleObjectFilterAtom::Nontoken => filter.nontoken = true,
            SimpleObjectFilterAtom::Historic => filter.historic = true,
            SimpleObjectFilterAtom::Nonhistoric => filter.nonhistoric = true,
            SimpleObjectFilterAtom::Modified => filter.modified = true,
            SimpleObjectFilterAtom::Colorless => filter.colorless = true,
            SimpleObjectFilterAtom::Multicolored => filter.multicolored = true,
            SimpleObjectFilterAtom::Monocolored => filter.monocolored = true,
            SimpleObjectFilterAtom::CardMarker => saw_card = true,
            SimpleObjectFilterAtom::PermanentMarker => saw_permanent = true,
            SimpleObjectFilterAtom::SpellMarker => saw_spell = true,
            SimpleObjectFilterAtom::Named(atom) => apply_named_atom(&mut filter, atom),
            SimpleObjectFilterAtom::CardType(card_type) => {
                if pending_type_separator.is_some_and(TypeListSeparator::is_disjunction)
                    && last_type_atom_is_card_type == Some(false)
                {
                    saw_type_subtype_disjunction = true;
                }
                push_unique(&mut filter.card_types, card_type);
                saw_permanent_type |= is_permanent_type(card_type);
                last_type_atom_is_card_type = Some(true);
                pending_type_separator = None;
            }
            SimpleObjectFilterAtom::ExcludedCardType(card_type) => {
                push_unique(&mut filter.excluded_card_types, card_type);
            }
            SimpleObjectFilterAtom::Subtype(subtype) => {
                if pending_type_separator.is_some_and(TypeListSeparator::is_disjunction)
                    && last_type_atom_is_card_type == Some(true)
                {
                    saw_type_subtype_disjunction = true;
                }
                push_unique(&mut filter.subtypes, subtype);
                last_type_atom_is_card_type = Some(false);
                pending_type_separator = None;
            }
            SimpleObjectFilterAtom::ExcludedSubtype(subtype) => {
                push_unique(&mut filter.excluded_subtypes, subtype);
            }
            SimpleObjectFilterAtom::Supertype(supertype) => {
                push_unique(&mut filter.supertypes, supertype);
            }
            SimpleObjectFilterAtom::ExcludedSupertype(supertype) => {
                push_unique(&mut filter.excluded_supertypes, supertype);
            }
            SimpleObjectFilterAtom::Color(color) => {
                let existing = filter.colors.unwrap_or(ColorSet::new());
                filter.colors = Some(existing.union(color));
            }
            SimpleObjectFilterAtom::ExcludedColor(color) => {
                filter.excluded_colors = filter.excluded_colors.union(color);
            }
            SimpleObjectFilterAtom::Outlaw => push_outlaw_subtypes(&mut filter.subtypes),
            SimpleObjectFilterAtom::NonOutlaw => {
                push_outlaw_subtypes(&mut filter.excluded_subtypes);
            }
        }
    }

    if filter.card_types.len() > 1 && filter.all_card_types.is_empty() && !saw_type_list_conjunction
    {
        filter.all_card_types = std::mem::take(&mut filter.card_types);
    }
    filter.type_or_subtype_union = saw_type_subtype_disjunction;
    if saw_and_or_union {
        filter.set_union_connective(ObjectFilterUnionConnective::AndOr);
    }
    filter.set_explicit_card_noun(saw_card);

    if saw_permanent && filter.card_types.is_empty() && filter.all_card_types.is_empty() {
        filter.card_types = ObjectFilter::permanent_card().card_types;
    }
    if filter.zone.is_none() {
        if saw_spell {
            filter.zone = Some(Zone::Stack);
        } else if saw_permanent || saw_permanent_type || filter.token {
            filter.zone = Some(Zone::Battlefield);
        } else if saw_card {
            filter.zone = None;
        }
    }
    if saw_spell {
        if filter.zone == Some(Zone::Battlefield) {
            filter.zone = Some(Zone::Stack);
        }
        filter.stack_kind = Some(StackObjectKind::Spell);
        filter.has_mana_cost = true;
    }

    Some(filter)
}

fn parse_simple_object_filter_atom(input: &mut WordInput<'_>) -> WResult<SimpleObjectFilterAtom> {
    alt((
        parse_type_list_separator,
        parse_of_named_atom,
        parse_alternative_cast_atom,
        parse_filter_face_state.map(SimpleObjectFilterAtom::FaceState),
        parse_simple_flag_atom,
        parse_named_object_filter_atom.map(SimpleObjectFilterAtom::Named),
        parse_split_non_atom,
        parse_typed_word_atom,
    ))
    .parse_next(input)
}

fn parse_type_list_separator(input: &mut WordInput<'_>) -> WResult<SimpleObjectFilterAtom> {
    alt((
        primitives::word_slice_exact("and").value(SimpleObjectFilterAtom::TypeListSeparator(
            TypeListSeparator::Conjunction,
        )),
        primitives::word_slice_exact("or").value(SimpleObjectFilterAtom::TypeListSeparator(
            TypeListSeparator::Disjunction,
        )),
        primitives::word_slice_exact("and/or").value(SimpleObjectFilterAtom::TypeListSeparator(
            TypeListSeparator::AndOr,
        )),
    ))
    .parse_next(input)
}

fn parse_of_named_atom(input: &mut WordInput<'_>) -> WResult<SimpleObjectFilterAtom> {
    (
        primitives::word_slice_exact("of"),
        parse_named_object_filter_atom,
    )
        .map(|(_, atom)| SimpleObjectFilterAtom::Named(atom))
        .parse_next(input)
}

fn parse_alternative_cast_atom(input: &mut WordInput<'_>) -> WResult<SimpleObjectFilterAtom> {
    let Some((kind, consumed)) = parse_alternative_cast_words(input) else {
        return Err(primitives::backtrack_err(
            "simple object filter atom",
            "alternative-cast phrase",
        ));
    };
    if consumed == 0 || consumed > input.len() {
        return Err(primitives::backtrack_err(
            "simple object filter atom",
            "nonempty alternative-cast phrase",
        ));
    }
    *input = &input[consumed..];
    Ok(SimpleObjectFilterAtom::AlternativeCast(kind))
}

fn parse_simple_flag_atom(input: &mut WordInput<'_>) -> WResult<SimpleObjectFilterAtom> {
    let checkpoint = *input;
    let word = parse_any_word.parse_next(input)?;
    let atom = match word {
        "other" | "another" => SimpleObjectFilterAtom::Other,
        "token" | "tokens" => SimpleObjectFilterAtom::Token,
        "nontoken" | "non-token" => SimpleObjectFilterAtom::Nontoken,
        "historic" => SimpleObjectFilterAtom::Historic,
        "nonhistoric" | "non-historic" => SimpleObjectFilterAtom::Nonhistoric,
        "modified" => SimpleObjectFilterAtom::Modified,
        "colorless" => SimpleObjectFilterAtom::Colorless,
        "multicolored" | "multicolour" | "multicoloured" => SimpleObjectFilterAtom::Multicolored,
        "monocolored" | "monocolour" | "monocoloured" => SimpleObjectFilterAtom::Monocolored,
        "card" | "cards" => SimpleObjectFilterAtom::CardMarker,
        "permanent" | "permanents" => SimpleObjectFilterAtom::PermanentMarker,
        "spell" | "spells" => SimpleObjectFilterAtom::SpellMarker,
        _ => {
            *input = checkpoint;
            return Err(primitives::backtrack_err(
                "simple object filter atom",
                "filter flag or object marker",
            ));
        }
    };
    Ok(atom)
}

fn parse_split_non_atom(input: &mut WordInput<'_>) -> WResult<SimpleObjectFilterAtom> {
    let checkpoint = *input;
    primitives::word_slice_exact("non")
        .void()
        .parse_next(input)?;
    let word = parse_any_word.parse_next(input)?;
    let atom = if let Some(card_type) = parse_card_type(word) {
        SimpleObjectFilterAtom::ExcludedCardType(card_type)
    } else if let Some(subtype) = parse_subtype_flexible(word) {
        SimpleObjectFilterAtom::ExcludedSubtype(subtype)
    } else if let Some(supertype) = parse_supertype_word(word) {
        SimpleObjectFilterAtom::ExcludedSupertype(supertype)
    } else if let Some(color) = parse_color(word) {
        SimpleObjectFilterAtom::ExcludedColor(color)
    } else if is_outlaw_word(word) {
        SimpleObjectFilterAtom::NonOutlaw
    } else {
        *input = checkpoint;
        return Err(primitives::backtrack_err(
            "simple object filter atom",
            "type, subtype, supertype, color, or outlaw after non",
        ));
    };
    Ok(atom)
}

fn parse_typed_word_atom(input: &mut WordInput<'_>) -> WResult<SimpleObjectFilterAtom> {
    let checkpoint = *input;
    let word = parse_any_word.parse_next(input)?;
    let atom = if let Some(card_type) = parse_card_type(word) {
        SimpleObjectFilterAtom::CardType(card_type)
    } else if let Some(card_type) = parse_non_type(word) {
        SimpleObjectFilterAtom::ExcludedCardType(card_type)
    } else if let Some(subtype) = parse_subtype_flexible(word) {
        SimpleObjectFilterAtom::Subtype(subtype)
    } else if let Some(subtype) = parse_non_subtype(word) {
        SimpleObjectFilterAtom::ExcludedSubtype(subtype)
    } else if let Some(supertype) = parse_supertype_word(word) {
        SimpleObjectFilterAtom::Supertype(supertype)
    } else if let Some(supertype) = parse_non_supertype(word) {
        SimpleObjectFilterAtom::ExcludedSupertype(supertype)
    } else if let Some(color) = parse_color(word) {
        SimpleObjectFilterAtom::Color(color)
    } else if let Some(color) = parse_non_color(word) {
        SimpleObjectFilterAtom::ExcludedColor(color)
    } else if is_outlaw_word(word) {
        SimpleObjectFilterAtom::Outlaw
    } else if is_non_outlaw_word(word) {
        SimpleObjectFilterAtom::NonOutlaw
    } else {
        *input = checkpoint;
        return Err(primitives::backtrack_err(
            "simple object filter atom",
            "typed card characteristic",
        ));
    };
    Ok(atom)
}

fn parse_filter_face_state(input: &mut WordInput<'_>) -> WResult<FilterFaceState> {
    alt((
        word_phrase(&["face", "down"]).value(FilterFaceState::FaceDown),
        primitives::word_slice_exact("face-down").value(FilterFaceState::FaceDown),
        primitives::word_slice_exact("facedown").value(FilterFaceState::FaceDown),
        word_phrase(&["face", "up"]).value(FilterFaceState::FaceUp),
        primitives::word_slice_exact("face-up").value(FilterFaceState::FaceUp),
        primitives::word_slice_exact("faceup").value(FilterFaceState::FaceUp),
    ))
    .parse_next(input)
}

fn parse_named_object_filter_atom(input: &mut WordInput<'_>) -> WResult<NamedObjectFilterAtom> {
    alt((
        word_phrase(&["chosen", "color"]).value(NamedObjectFilterAtom::ChosenColor),
        word_phrase(&["chosen", "type"]).value(NamedObjectFilterAtom::ChosenType),
        word_phrase(&["that", "type"]).value(NamedObjectFilterAtom::ChosenType),
        word_phrase(&["nonchosen", "type"]).value(NamedObjectFilterAtom::NonChosenType),
    ))
    .parse_next(input)
}

fn apply_named_atom(filter: &mut ObjectFilter, atom: NamedObjectFilterAtom) {
    match atom {
        NamedObjectFilterAtom::ChosenColor => filter.chosen_color = true,
        NamedObjectFilterAtom::ChosenType => filter.chosen_creature_type = true,
        NamedObjectFilterAtom::NonChosenType => filter.excluded_chosen_creature_type = true,
    }
}

fn contains_simple_filter_reject(words: &[&str]) -> bool {
    for index in 0..words.len() {
        let mut input: WordInput<'_> = &words[index..];
        if parse_simple_filter_reject.parse_next(&mut input).is_ok() {
            return true;
        }
    }
    false
}

fn parse_simple_filter_reject(input: &mut WordInput<'_>) -> WResult<()> {
    let checkpoint = *input;
    let word = parse_any_word.parse_next(input)?;
    if matches!(
        word,
        "target"
            | "targets"
            | "that"
            | "which"
            | "whose"
            | "where"
            | "there"
            | "shares"
            | "share"
            | "dealt"
            | "entered"
            | "put"
            | "this"
            | "way"
    ) {
        Ok(())
    } else {
        *input = checkpoint;
        Err(primitives::backtrack_err(
            "simple object filter reject",
            "word requiring complex object-filter grammar",
        ))
    }
}

fn parse_other_than_split<'a>(words: &'a [&'a str]) -> Option<OtherThanSplit<'a>> {
    for delimiter_start in 0..words.len() {
        let mut input: WordInput<'a> = &words[delimiter_start..];
        if word_phrase(&["other", "than"])
            .parse_next(&mut input)
            .is_ok()
        {
            if delimiter_start == 0 {
                return None;
            }
            let exclusions = words.get(delimiter_start + 2..)?;
            if !exclusions.is_empty() {
                return Some(OtherThanSplit {
                    base: &words[..delimiter_start],
                    exclusions,
                });
            }
        }
    }
    None
}

fn parse_other_than_filter(split: OtherThanSplit<'_>, other: bool) -> Option<ObjectFilter> {
    let mut filter = parse_simple_object_filter_words(split.base, other)?;
    let mut input: WordInput<'_> = split.exclusions;
    let mut saw_exclusion = false;
    while !input.is_empty() {
        match parse_excluded_object_filter_atom
            .parse_next(&mut input)
            .ok()?
        {
            ExcludedObjectFilterAtom::Separator => {}
            ExcludedObjectFilterAtom::CardType(card_type) => {
                push_unique(&mut filter.excluded_card_types, card_type);
                saw_exclusion = true;
            }
            ExcludedObjectFilterAtom::Subtype(subtype) => {
                push_unique(&mut filter.excluded_subtypes, subtype);
                saw_exclusion = true;
            }
            ExcludedObjectFilterAtom::Supertype(supertype) => {
                push_unique(&mut filter.excluded_supertypes, supertype);
                saw_exclusion = true;
            }
            ExcludedObjectFilterAtom::Color(color) => {
                filter.excluded_colors = filter.excluded_colors.union(color);
                saw_exclusion = true;
            }
            ExcludedObjectFilterAtom::Outlaw => {
                push_outlaw_subtypes(&mut filter.excluded_subtypes);
                saw_exclusion = true;
            }
        }
    }
    saw_exclusion.then_some(filter)
}

fn parse_excluded_object_filter_atom(
    input: &mut WordInput<'_>,
) -> WResult<ExcludedObjectFilterAtom> {
    let checkpoint = *input;
    let word = parse_any_word.parse_next(input)?;
    let atom = match word {
        "and" | "or" => ExcludedObjectFilterAtom::Separator,
        _ => {
            if let Some(card_type) = parse_card_type(word) {
                ExcludedObjectFilterAtom::CardType(card_type)
            } else if let Some(subtype) = parse_subtype_flexible(word) {
                ExcludedObjectFilterAtom::Subtype(subtype)
            } else if let Some(supertype) = parse_supertype_word(word) {
                ExcludedObjectFilterAtom::Supertype(supertype)
            } else if let Some(color) = parse_color(word) {
                ExcludedObjectFilterAtom::Color(color)
            } else if is_outlaw_word(word) {
                ExcludedObjectFilterAtom::Outlaw
            } else {
                *input = checkpoint;
                return Err(primitives::backtrack_err(
                    "simple object filter exclusion",
                    "card type, subtype, supertype, color, or outlaw",
                ));
            }
        }
    };
    Ok(atom)
}

fn parse_simple_object_filter_suffix(words: &[&str]) -> Option<(SimpleObjectFilterSuffix, usize)> {
    for suffix_len in (5..=6).rev() {
        let Some(tail) = suffix_tail(words, suffix_len) else {
            continue;
        };
        if let Some(suffix) = parse_full_word_slice(tail, parse_controller_owner_suffix) {
            return Some((suffix, suffix_len));
        }
    }
    for suffix_len in (3..=4).rev() {
        let Some(tail) = suffix_tail(words, suffix_len) else {
            continue;
        };
        if let Some(suffix) = parse_full_word_slice(tail, parse_negated_controller_suffix) {
            return Some((suffix, suffix_len));
        }
    }
    for suffix_len in (2..=3).rev() {
        let Some(tail) = suffix_tail(words, suffix_len) else {
            continue;
        };
        if let Some(suffix) = parse_full_word_slice(tail, parse_location_suffix) {
            return Some((suffix, suffix_len));
        }
    }
    for suffix_len in (2..=3).rev() {
        let Some(tail) = suffix_tail(words, suffix_len) else {
            continue;
        };
        if let Some(suffix) = parse_full_word_slice(tail, parse_controller_suffix) {
            return Some((suffix, suffix_len));
        }
    }
    for suffix_len in (2..=3).rev() {
        let Some(tail) = suffix_tail(words, suffix_len) else {
            continue;
        };
        if let Some(suffix) = parse_full_word_slice(tail, parse_owner_suffix) {
            return Some((suffix, suffix_len));
        }
    }
    None
}

fn parse_controller_owner_suffix(input: &mut WordInput<'_>) -> WResult<SimpleObjectFilterSuffix> {
    let controller = parse_controller_player.parse_next(input)?;
    parse_control_action.parse_next(input)?;
    primitives::word_slice_exact("but")
        .void()
        .parse_next(input)?;
    parse_control_negation.parse_next(input)?;
    parse_own_action.parse_next(input)?;
    if controller != PlayerFilter::You {
        return Err(primitives::backtrack_err(
            "simple object filter suffix",
            "you control but do not own",
        ));
    }
    Ok(SimpleObjectFilterSuffix::ControllerOwner(
        controller,
        PlayerFilter::NotYou,
    ))
}

fn parse_negated_controller_suffix(input: &mut WordInput<'_>) -> WResult<SimpleObjectFilterSuffix> {
    primitives::word_slice_exact("you")
        .void()
        .parse_next(input)?;
    parse_control_negation.parse_next(input)?;
    parse_control_action.parse_next(input)?;
    Ok(SimpleObjectFilterSuffix::Controller(PlayerFilter::NotYou))
}

fn parse_location_suffix(input: &mut WordInput<'_>) -> WResult<SimpleObjectFilterSuffix> {
    alt((
        primitives::word_slice_exact("in"),
        primitives::word_slice_exact("from"),
    ))
    .void()
    .parse_next(input)?;
    let (owner, zone) = parse_location.parse_next(input)?;
    Ok(match owner {
        Some(owner) => SimpleObjectFilterSuffix::OwnerZone(owner, zone),
        None => SimpleObjectFilterSuffix::Zone(zone),
    })
}

fn parse_controller_suffix(input: &mut WordInput<'_>) -> WResult<SimpleObjectFilterSuffix> {
    let controller = parse_controller_player.parse_next(input)?;
    parse_control_action.parse_next(input)?;
    Ok(SimpleObjectFilterSuffix::Controller(controller))
}

fn parse_owner_suffix(input: &mut WordInput<'_>) -> WResult<SimpleObjectFilterSuffix> {
    primitives::word_slice_exact("you")
        .void()
        .parse_next(input)?;
    parse_own_action.parse_next(input)?;
    Ok(SimpleObjectFilterSuffix::Owner(PlayerFilter::You))
}

fn parse_controller_player(input: &mut WordInput<'_>) -> WResult<PlayerFilter> {
    alt((
        word_phrase(&["target", "opponent"]).value(PlayerFilter::target_opponent()),
        word_phrase(&["target", "player"]).value(PlayerFilter::target_player()),
        word_phrase(&["that", "player"]).value(PlayerFilter::IteratedPlayer),
        primitives::word_slice_exact("opponents").value(PlayerFilter::Opponent),
        primitives::word_slice_exact("opponent").value(PlayerFilter::Opponent),
        primitives::word_slice_exact("you").value(PlayerFilter::You),
    ))
    .parse_next(input)
}

fn parse_control_action(input: &mut WordInput<'_>) -> WResult<()> {
    alt((
        primitives::word_slice_exact("control"),
        primitives::word_slice_exact("controls"),
    ))
    .void()
    .parse_next(input)
}

fn parse_own_action(input: &mut WordInput<'_>) -> WResult<()> {
    alt((
        primitives::word_slice_exact("own"),
        primitives::word_slice_exact("owns"),
    ))
    .void()
    .parse_next(input)
}

fn parse_control_negation(input: &mut WordInput<'_>) -> WResult<()> {
    alt((
        word_phrase(&["do", "not"]),
        primitives::word_slice_exact("dont").void(),
        primitives::word_slice_exact("don't").void(),
    ))
    .parse_next(input)
}

fn parse_location(input: &mut WordInput<'_>) -> WResult<(Option<PlayerFilter>, Zone)> {
    alt((
        word_phrase(&["your", "graveyard"]).value((Some(PlayerFilter::You), Zone::Graveyard)),
        word_phrase(&["your", "hand"]).value((Some(PlayerFilter::You), Zone::Hand)),
        word_phrase(&["your", "library"]).value((Some(PlayerFilter::You), Zone::Library)),
        word_phrase(&["all", "graveyards"]).value((None, Zone::Graveyard)),
        primitives::word_slice_exact("graveyard").value((None, Zone::Graveyard)),
        primitives::word_slice_exact("hand").value((None, Zone::Hand)),
        primitives::word_slice_exact("library").value((None, Zone::Library)),
        primitives::word_slice_exact("exile").value((None, Zone::Exile)),
    ))
    .parse_next(input)
}

fn apply_simple_object_filter_suffix(filter: &mut ObjectFilter, suffix: SimpleObjectFilterSuffix) {
    match suffix {
        SimpleObjectFilterSuffix::Controller(controller) => {
            filter.controller = Some(controller);
            filter.zone = Some(Zone::Battlefield);
        }
        SimpleObjectFilterSuffix::Owner(owner) => filter.owner = Some(owner),
        SimpleObjectFilterSuffix::ControllerOwner(controller, owner) => {
            filter.controller = Some(controller);
            filter.owner = Some(owner);
            filter.zone = Some(Zone::Battlefield);
        }
        SimpleObjectFilterSuffix::OwnerZone(owner, zone) => {
            filter.owner = Some(owner);
            filter.zone = Some(zone);
        }
        SimpleObjectFilterSuffix::Zone(zone) => filter.zone = Some(zone),
    }
}

fn suffix_tail<'a>(words: &'a [&'a str], suffix_len: usize) -> Option<&'a [&'a str]> {
    words.get(words.len().checked_sub(suffix_len)?..)
}

fn word_phrase<'a>(
    expected: &'static [&'static str],
) -> impl Parser<WordInput<'a>, (), ErrMode<ContextError>> {
    move |input: &mut WordInput<'a>| {
        let checkpoint = *input;
        for word in expected {
            if let Err(err) = primitives::word_slice_exact(*word).void().parse_next(input) {
                *input = checkpoint;
                return Err(err);
            }
        }
        Ok(())
    }
}

fn parse_any_word<'a>(input: &mut WordInput<'a>) -> WResult<&'a str> {
    let Some((word, rest)) = input.split_first() else {
        return Err(primitives::backtrack_err(
            "simple object filter word",
            "word",
        ));
    };
    *input = rest;
    Ok(*word)
}

fn push_unique<T: Copy + PartialEq>(items: &mut Vec<T>, value: T) {
    crate::slice_primitives::push_unique(items, value);
}

#[cfg(test)]
#[path = "simple/tests.rs"]
mod tests;
