use winnow::combinator::{alt, opt};
use winnow::error::{ContextError, ErrMode, ModalResult as WResult};
use winnow::prelude::*;

use crate::filter::{AlternativeCastKind, ObjectFilterUnionConnective, StackObjectKind};
use crate::{CardType, ColorSet, ObjectFilter, PlayerFilter, Subtype, Supertype, Zone};

use super::super::primitives::{self, WordSliceInput, parse_full_word_slice};
use crate::lexer::{OwnedLexToken, TokenKind, TokenWordView};
use crate::util::{
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
    Foretold,
    Historic,
    Nonhistoric,
    Modified,
    Suspected,
    Tapped,
    Untapped,
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
    CompoundSubtypes(Subtype, Subtype),
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

pub fn parse_filter_face_state_words(words: &[&str]) -> Option<(bool, usize)> {
    let mut input: WordInput<'_> = words;
    let state = crate::grammar::primitives::take_leaf(&mut input, parse_filter_face_state)?;
    Some((state.is_face_down(), words.len().checked_sub(input.len())?))
}

pub fn parse_simple_object_filter_words(input_words: &[&str], other: bool) -> Option<ObjectFilter> {
    parse_simple_object_filter_words_with_list_marker(input_words, other, false)
}

pub fn parse_simple_object_filter_lexed(
    tokens: &[OwnedLexToken],
    other: bool,
) -> Option<ObjectFilter> {
    if super::is_attack_destination_relation(tokens) {
        // The trailing controller belongs to the attacked planeswalker, not
        // to the candidate creature. The simple suffix grammar cannot retain
        // that relationship, so leave the complete clause to the relational
        // object-filter grammar.
        return None;
    }

    let word_view = TokenWordView::new(tokens);
    let saw_type_list_separator = tokens.iter().any(|token| token.kind == TokenKind::Comma);
    let mut filter = parse_simple_object_filter_words_with_list_marker(
        &word_view.to_word_refs(),
        other,
        saw_type_list_separator,
    )?;
    preserve_branch_scoped_card_type_union(&mut filter, tokens, other);
    Some(filter)
}

/// Preserve exclusions that grammatically belong to only one arm of a card-type union.
///
/// A flattened filter cannot represent `artifact or non-Aura enchantment`: storing
/// `Artifact` and `Enchantment` alongside a global `Aura` exclusion also rejects Aura
/// artifacts. `ObjectFilter::any_of` already carries the required inclusive-union
/// semantics, so keep the shared domain on the outer filter and put each type arm (and
/// its local exclusions) in a nested selector.
pub fn preserve_branch_scoped_card_type_union(
    filter: &mut ObjectFilter,
    tokens: &[OwnedLexToken],
    other: bool,
) {
    if other || !filter.any_of.is_empty() || tokens_contain_other_than(tokens) {
        return;
    }

    let segments = split_card_type_union_segments(tokens);
    if segments.len() < 2 {
        return;
    }

    let branches = segments
        .iter()
        .copied()
        .map(parse_card_type_union_branch)
        .collect::<Option<Vec<_>>>()
        .or_else(|| infer_card_type_union_branches(filter, &segments));
    let Some(branches) = branches else { return };

    let has_local_exclusion = branches.iter().any(card_type_branch_has_exclusion);
    let has_unexcluded_branch = branches
        .iter()
        .any(|branch| !card_type_branch_has_exclusion(branch));
    if !has_local_exclusion || !has_unexcluded_branch {
        return;
    }

    let mut branch_types = Vec::new();
    for branch in &branches {
        let card_type = branch.card_types[0];
        if crate::slice_primitives::contains(&branch_types, &card_type) {
            return;
        }
        branch_types.push(card_type);
    }
    if branch_types.len() != filter.card_types.len()
        || branch_types
            .iter()
            .any(|card_type| !crate::slice_primitives::contains(&filter.card_types, card_type))
    {
        return;
    }

    filter.card_types.clear();
    filter.all_card_types.clear();
    filter.excluded_card_types.clear();
    filter.subtypes.clear();
    filter.all_subtypes.clear();
    filter.type_or_subtype_union = false;
    filter.excluded_subtypes.clear();
    filter.supertypes.clear();
    filter.excluded_supertypes.clear();
    filter.colors = None;
    filter.required_colors = None;
    filter.excluded_colors = ColorSet::new();
    filter.colorless = false;
    filter.multicolored = false;
    filter.monocolored = false;
    filter.any_of = branches;
}

fn infer_card_type_union_branches(
    filter: &ObjectFilter,
    segments: &[&[OwnedLexToken]],
) -> Option<Vec<ObjectFilter>> {
    if filter.card_types.len() < 2 || !filter.all_card_types.is_empty() {
        return None;
    }

    let mut branches = Vec::new();
    for segment in segments {
        let word_view = TokenWordView::new(segment);
        let words = word_view.to_word_refs();
        let segment_types = words
            .iter()
            .filter_map(|word| parse_card_type(word))
            .filter(|card_type| crate::slice_primitives::contains(&filter.card_types, card_type))
            .collect::<Vec<_>>();
        if segment_types.len() != 1
            || branches.iter().any(|branch: &ObjectFilter| {
                branch.card_types.len() == 1 && branch.card_types.first() == segment_types.first()
            })
        {
            return None;
        }

        let mut branch = ObjectFilter::default().with_type(segment_types[0]);
        for word in &words {
            if let Some(card_type) = parse_non_type(word)
                && crate::slice_primitives::contains(&filter.excluded_card_types, &card_type)
            {
                push_unique(&mut branch.excluded_card_types, card_type);
            }
            if let Some(subtype) = parse_non_subtype(word)
                && crate::slice_primitives::contains(&filter.excluded_subtypes, &subtype)
            {
                push_unique(&mut branch.excluded_subtypes, subtype);
            }
            if let Some(supertype) = parse_non_supertype(word)
                && crate::slice_primitives::contains(&filter.excluded_supertypes, &supertype)
            {
                push_unique(&mut branch.excluded_supertypes, supertype);
            }
            if let Some(color) = parse_non_color(word) {
                branch.excluded_colors = branch.excluded_colors.union(color);
            }
        }
        for pair_start in 0..words.len().saturating_sub(1) {
            if words[pair_start] != "non" {
                continue;
            }
            let qualified = words[pair_start + 1];
            if let Some(card_type) = parse_card_type(qualified)
                && crate::slice_primitives::contains(&filter.excluded_card_types, &card_type)
            {
                push_unique(&mut branch.excluded_card_types, card_type);
            }
            if let Some(subtype) = parse_subtype_flexible(qualified)
                && crate::slice_primitives::contains(&filter.excluded_subtypes, &subtype)
            {
                push_unique(&mut branch.excluded_subtypes, subtype);
            }
            if let Some(supertype) = parse_supertype_word(qualified)
                && crate::slice_primitives::contains(&filter.excluded_supertypes, &supertype)
            {
                push_unique(&mut branch.excluded_supertypes, supertype);
            }
            if let Some(color) = parse_color(qualified) {
                branch.excluded_colors = branch.excluded_colors.union(color);
            }
        }
        branches.push(branch);
    }

    let assigned_card_types = branches
        .iter()
        .flat_map(|branch| branch.excluded_card_types.iter())
        .collect::<Vec<_>>();
    let assigned_subtypes = branches
        .iter()
        .flat_map(|branch| branch.excluded_subtypes.iter())
        .collect::<Vec<_>>();
    let assigned_supertypes = branches
        .iter()
        .flat_map(|branch| branch.excluded_supertypes.iter())
        .collect::<Vec<_>>();
    if branches.len() != filter.card_types.len()
        || filter
            .excluded_card_types
            .iter()
            .any(|value| !crate::slice_primitives::contains(&assigned_card_types, &value))
        || filter
            .excluded_subtypes
            .iter()
            .any(|value| !crate::slice_primitives::contains(&assigned_subtypes, &value))
        || filter
            .excluded_supertypes
            .iter()
            .any(|value| !crate::slice_primitives::contains(&assigned_supertypes, &value))
        || branches.iter().fold(ColorSet::new(), |colors, branch| {
            colors.union(branch.excluded_colors)
        }) != filter.excluded_colors
    {
        return None;
    }
    Some(branches)
}

fn split_card_type_union_segments(tokens: &[OwnedLexToken]) -> Vec<&[OwnedLexToken]> {
    if tokens.iter().any(|token| token.kind == TokenKind::Comma) {
        let mut segments = Vec::new();
        let mut start = 0usize;
        for (idx, token) in tokens.iter().enumerate() {
            if token.kind == TokenKind::Comma {
                if start < idx {
                    segments.push(&tokens[start..idx]);
                }
                start = idx + 1;
                continue;
            }

            // An Oxford-list conjunction immediately after a comma introduces
            // the next selector rather than belonging to it. Internal
            // conjunctions later in the selector (for example, "owned and
            // controlled") remain part of that branch.
            if idx == start
                && (token.is_word("and") || token.is_word("or") || token.is_word("and/or"))
            {
                start = idx + 1;
            }
        }
        if start < tokens.len() {
            segments.push(&tokens[start..]);
        }
        return segments;
    }

    if !tokens.iter().any(|token| token.is_word("and/or")) {
        return primitives::split_lexed_slices_on_or(tokens);
    }

    let mut segments = Vec::new();
    let mut start = 0usize;
    for (idx, token) in tokens.iter().enumerate() {
        if token.kind != TokenKind::Comma && !token.is_word("and/or") {
            continue;
        }
        if start < idx {
            segments.push(&tokens[start..idx]);
        }
        start = idx + 1;
    }
    if start < tokens.len() {
        segments.push(&tokens[start..]);
    }
    segments
}

fn parse_card_type_union_branch(tokens: &[OwnedLexToken]) -> Option<ObjectFilter> {
    let word_view = TokenWordView::new(tokens);
    let mut branch =
        parse_simple_object_filter_words_with_list_marker(&word_view.to_word_refs(), false, false)?;

    // The final arm owns the shared noun/domain suffix in Oracle syntax. Those
    // facts stay on the outer filter; nested arms carry only their selectors.
    branch.zone = None;
    branch.controller = None;
    branch.owner = None;
    branch.single_graveyard = false;
    branch.stack_kind = None;
    branch.has_mana_cost = false;
    branch.union_surface = Default::default();

    if branch.card_types.len() != 1 || !branch.all_card_types.is_empty() {
        return None;
    }

    let mut selector_remainder = branch.clone();
    selector_remainder.card_types.clear();
    selector_remainder.excluded_card_types.clear();
    selector_remainder.excluded_subtypes.clear();
    selector_remainder.excluded_supertypes.clear();
    selector_remainder.excluded_colors = ColorSet::new();
    if selector_remainder != ObjectFilter::default() {
        return None;
    }

    Some(branch)
}

fn card_type_branch_has_exclusion(branch: &ObjectFilter) -> bool {
    !branch.excluded_card_types.is_empty()
        || !branch.excluded_subtypes.is_empty()
        || !branch.excluded_supertypes.is_empty()
        || !branch.excluded_colors.is_empty()
}

fn tokens_contain_other_than(tokens: &[OwnedLexToken]) -> bool {
    primitives::find_prefix(tokens, || primitives::phrase(&["other", "than"])).is_some()
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
    let mut first_type_atom_is_card_type = None;
    let mut terminal_noun_follows_last_characteristic = false;
    let mut saw_type_subtype_disjunction = false;
    let mut saw_and_or_union = false;
    let mut saw_inclusive_subtype_bundle = false;

    while !input.is_empty() {
        let atom =
            crate::grammar::primitives::take_leaf(&mut input, parse_simple_object_filter_atom)?;
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
            SimpleObjectFilterAtom::Foretold => filter.foretold = true,
            SimpleObjectFilterAtom::Historic => filter.historic = true,
            SimpleObjectFilterAtom::Nonhistoric => filter.nonhistoric = true,
            SimpleObjectFilterAtom::Modified => filter.modified = true,
            SimpleObjectFilterAtom::Suspected => filter.suspected = true,
            SimpleObjectFilterAtom::Tapped => filter.tapped = true,
            SimpleObjectFilterAtom::Untapped => filter.untapped = true,
            SimpleObjectFilterAtom::Colorless => filter.colorless = true,
            SimpleObjectFilterAtom::Multicolored => filter.multicolored = true,
            SimpleObjectFilterAtom::Monocolored => filter.monocolored = true,
            SimpleObjectFilterAtom::CardMarker => {
                saw_card = true;
                terminal_noun_follows_last_characteristic = last_type_atom_is_card_type.is_some();
            }
            SimpleObjectFilterAtom::PermanentMarker => saw_permanent = true,
            SimpleObjectFilterAtom::SpellMarker => {
                saw_spell = true;
                terminal_noun_follows_last_characteristic = last_type_atom_is_card_type.is_some();
            }
            SimpleObjectFilterAtom::Named(atom) => apply_named_atom(&mut filter, atom),
            SimpleObjectFilterAtom::CardType(card_type) => {
                filter.set_explicit_card_type_noun(Some(card_type));
                first_type_atom_is_card_type.get_or_insert(true);
                terminal_noun_follows_last_characteristic = false;
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
                first_type_atom_is_card_type.get_or_insert(false);
                terminal_noun_follows_last_characteristic = false;
                if pending_type_separator.is_some_and(TypeListSeparator::is_disjunction)
                    && last_type_atom_is_card_type == Some(true)
                {
                    saw_type_subtype_disjunction = true;
                }
                push_unique(&mut filter.subtypes, subtype);
                last_type_atom_is_card_type = Some(false);
                pending_type_separator = None;
            }
            SimpleObjectFilterAtom::CompoundSubtypes(first, second) => {
                first_type_atom_is_card_type.get_or_insert(false);
                terminal_noun_follows_last_characteristic = false;
                if pending_type_separator.is_some_and(TypeListSeparator::is_disjunction)
                    && last_type_atom_is_card_type == Some(true)
                {
                    saw_type_subtype_disjunction = true;
                }
                push_unique(&mut filter.subtypes, first);
                push_unique(&mut filter.subtypes, second);
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
            SimpleObjectFilterAtom::Outlaw => {
                push_outlaw_subtypes(&mut filter.subtypes);
                saw_inclusive_subtype_bundle = true;
            }
            SimpleObjectFilterAtom::NonOutlaw => {
                push_outlaw_subtypes(&mut filter.excluded_subtypes);
            }
        }
    }

    if filter.card_types.len() > 1 && filter.all_card_types.is_empty() && !saw_type_list_conjunction
    {
        filter.all_card_types = std::mem::take(&mut filter.card_types);
    }
    if filter.subtypes.len() > 1
        && filter.all_subtypes.is_empty()
        && !saw_type_list_conjunction
        && !saw_inclusive_subtype_bundle
    {
        filter.all_subtypes = std::mem::take(&mut filter.subtypes);
    }
    filter.type_or_subtype_union = saw_type_subtype_disjunction;
    if saw_type_subtype_disjunction {
        filter.set_subtype_before_card_type_union_surface(
            first_type_atom_is_card_type == Some(false),
        );
        filter.set_terminal_noun_after_type_subtype_union_surface(
            terminal_noun_follows_last_characteristic,
        );
    }
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
        parse_compound_subtype_atom,
        parse_typed_word_atom,
    ))
    .parse_next(input)
}

fn parse_compound_subtype_atom(input: &mut WordInput<'_>) -> WResult<SimpleObjectFilterAtom> {
    alt((
        parse_time_lord_compound_atom,
        parse_urzas_land_compound_atom,
    ))
    .parse_next(input)
}

/// Parse Magic's compound `Time Lord` creature type.
fn parse_time_lord_compound_atom(input: &mut WordInput<'_>) -> WResult<SimpleObjectFilterAtom> {
    word_phrase(&["time", "lord"])
        .value(SimpleObjectFilterAtom::Subtype(Subtype::TimeLord))
        .parse_next(input)
}

/// Parse the three compound Urza's land subtypes in their unambiguous rules-text
/// context. `Mine` and `Tower` are deliberately rejected by the broad subtype
/// parser because they are common English nouns, while `Power-Plant` must remain
/// one subtype rather than `Plant`. The `Urza's` prefix removes that ambiguity.
fn parse_urzas_land_compound_atom(input: &mut WordInput<'_>) -> WResult<SimpleObjectFilterAtom> {
    let checkpoint = *input;
    let first = parse_any_word.parse_next(input)?;
    if parse_subtype_flexible(first) != Some(Subtype::Urzas) {
        *input = checkpoint;
        return Err(primitives::backtrack_err(
            "compound Urza's land subtype",
            "Urza's followed by Mine, Power-Plant, or Tower",
        ));
    }

    let second_word = parse_any_word.parse_next(input)?;
    let second = if second_word.eq_ignore_ascii_case("power")
        && input
            .first()
            .is_some_and(|word| word.eq_ignore_ascii_case("plant"))
    {
        *input = &input[1..];
        Some(Subtype::PowerPlant)
    } else {
        super::super::leaf::classify_token_definition_subtype(second_word)
    };
    let Some(second @ (Subtype::Mine | Subtype::PowerPlant | Subtype::Tower)) = second else {
        *input = checkpoint;
        return Err(primitives::backtrack_err(
            "compound Urza's land subtype",
            "Mine, Power-Plant, or Tower after Urza's",
        ));
    };

    Ok(SimpleObjectFilterAtom::CompoundSubtypes(
        Subtype::Urzas,
        second,
    ))
}

fn parse_type_list_separator(input: &mut WordInput<'_>) -> WResult<SimpleObjectFilterAtom> {
    alt((
        (
            primitives::word_slice_exact("and"),
            primitives::word_slice_exact("or"),
        )
            .value(SimpleObjectFilterAtom::TypeListSeparator(
                TypeListSeparator::AndOr,
            )),
        primitives::word_slice_exact("and").value(SimpleObjectFilterAtom::TypeListSeparator(
            TypeListSeparator::Conjunction,
        )),
        primitives::word_slice_exact("or").value(SimpleObjectFilterAtom::TypeListSeparator(
            TypeListSeparator::Disjunction,
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
        "foretold" => SimpleObjectFilterAtom::Foretold,
        "historic" => SimpleObjectFilterAtom::Historic,
        "nonhistoric" | "non-historic" => SimpleObjectFilterAtom::Nonhistoric,
        "modified" => SimpleObjectFilterAtom::Modified,
        "suspected" => SimpleObjectFilterAtom::Suspected,
        "tapped" => SimpleObjectFilterAtom::Tapped,
        "untapped" => SimpleObjectFilterAtom::Untapped,
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

#[cfg(test)]
#[path = "simple/tests.rs"]
mod tests;

#[path = "simple/simple_core.rs"]
mod simple_core_programs;
use simple_core_programs::{
    apply_named_atom, parse_any_word, parse_location, parse_location_suffix,
    parse_other_than_split, parse_own_action, parse_owner_suffix, parse_typed_word_atom,
    push_unique, suffix_tail, word_phrase,
};
#[path = "simple/simple_reference.rs"]
mod simple_reference_programs;
use simple_reference_programs::{
    apply_simple_object_filter_suffix, contains_simple_filter_reject, parse_controller_player,
    parse_excluded_object_filter_atom, parse_filter_face_state, parse_named_object_filter_atom,
    parse_other_than_filter, parse_simple_filter_reject, parse_simple_object_filter_suffix,
    parse_target_player_or_planeswalker_controller,
};
#[path = "simple/simple_choice.rs"]
mod simple_choice_programs;
use simple_choice_programs::parse_chosen_player_location;
#[path = "simple/simple_object_action.rs"]
mod simple_object_action_programs;
use simple_object_action_programs::{
    parse_control_action, parse_control_negation, parse_controller_owner_suffix,
    parse_controller_suffix, parse_negated_controller_suffix,
};
