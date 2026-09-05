use crate::cards::builders::CardTextError;
#[cfg(test)]
use crate::filter::Comparison;
use crate::filter::ObjectFilterUnionConnective;
#[cfg(test)]
use crate::{CardType, Subtype};
use crate::{ColorSet, ObjectFilter, PlayerFilter, TaggedOpbjectRelation, Zone};

pub use super::grammar::filters::parse_simple_object_filter_words;
use super::grammar::filters::{
    apply_filter_tail_decoration, parse_branch_scoped_object_filter_union_lexed,
    parse_extremum_object_filter_lexed, parse_extremum_object_filter_words,
    parse_filter_distinct_names_tokens, parse_filter_lexed_envelope,
    parse_filter_tail_decoration_split_words, parse_filter_tail_decoration_tokens,
    parse_filter_word_envelope, parse_repeated_selector_domain_union_lexed,
    parse_simple_object_filter_lexed, preserve_branch_scoped_card_type_union,
    preserve_filter_counter_constraint_surface_tokens,
    preserve_filter_counter_constraint_surface_words,
};
use super::grammar::primitives::split_lexed_slices_on_or;
use super::lexer::{
    OwnedLexToken, TokenWordView, parser_token_word_refs, render_token_slice, token_slice_at_is,
};
use super::util::{
    apply_filter_keyword_constraint, non_article_word_refs, parse_card_type,
    parse_filter_keyword_constraint_words, parse_number_word_u32, parse_subtype_flexible,
    parse_supertype_word,
};

const ORIGINAL_PRINTING_SET_PREFIX: &[&str] =
    &["with", "a", "name", "originally", "printed", "in", "the"];
const SACRIFICED_AS_IT_ENTERED_SUFFIX: &[&str] = &["sacrificed", "as", "it", "entered"];
const DRAFTED_COLOR_QUALIFIER: &[&str] = &[
    "one", "or", "more", "of", "the", "colors", "chosen", "as", "you", "drafted", "cards", "named",
];

pub(crate) fn split_drafted_color_qualifier_tokens(
    tokens: &[OwnedLexToken],
) -> Option<(Vec<OwnedLexToken>, String)> {
    let view = TokenWordView::new(tokens);
    let words = view.to_word_refs();
    let qualifier_start =
        crate::word_primitives::parse_sequence_start(&words, DRAFTED_COLOR_QUALIFIER)?;
    let name_start = qualifier_start + DRAFTED_COLOR_QUALIFIER.len();
    if qualifier_start == 0 || name_start >= words.len() {
        return None;
    }
    let base_word_end = if matches!(words[qualifier_start - 1], "thats" | "that's") {
        qualifier_start - 1
    } else {
        qualifier_start
    };
    if base_word_end == 0 {
        return None;
    }
    let base_token_end = view.map_word_or_end_to_token_boundary(base_word_end)?;
    let name_token_start = view.map_word_or_end_to_token_boundary(name_start)?;
    let name = render_token_slice(tokens.get(name_token_start..)?)
        .trim()
        .trim_end_matches('.')
        .to_string();
    (!name.is_empty()).then(|| (super::util::trim_commas(&tokens[..base_token_end]), name))
}

fn split_distinct_combat_damage_controller_tokens(
    tokens: &[OwnedLexToken],
) -> Option<(Vec<OwnedLexToken>, Vec<OwnedLexToken>, u32)> {
    const PREFIX: &[&str] = &[
        "controlled",
        "by",
        "a",
        "player",
        "who",
        "was",
        "dealt",
        "combat",
        "damage",
        "by",
    ];
    const TURN_SUFFIX: &[&str] = &["this", "turn"];

    let view = TokenWordView::new(tokens);
    let words = view.to_word_refs();
    let prefix_start = crate::word_primitives::parse_sequence_start(&words, PREFIX)?;
    if prefix_start == 0 || !crate::word_primitives::parse_sequence_suffix(&words, TURN_SUFFIX) {
        return None;
    }
    let threshold_start = prefix_start + PREFIX.len();
    if !words
        .get(threshold_start + 1..)
        .is_some_and(|tail| crate::word_primitives::parse_sequence_prefix(tail, &["or", "more"]))
    {
        return None;
    }
    let minimum = parse_number_word_u32(words.get(threshold_start).copied()?)?;
    let source_start = threshold_start + 3;
    let source_end = words.len().checked_sub(TURN_SUFFIX.len())?;
    if minimum == 0 || source_start >= source_end {
        return None;
    }

    let base_end = view.map_word_or_end_to_token_boundary(prefix_start)?;
    let source_token_start = view.map_word_or_end_to_token_boundary(source_start)?;
    let source_token_end = view.map_word_or_end_to_token_boundary(source_end)?;
    Some((
        super::util::trim_commas(&tokens[..base_end]),
        super::util::trim_commas(&tokens[source_token_start..source_token_end]),
        minimum,
    ))
}

fn split_sacrificed_as_it_entered_tokens(tokens: &[OwnedLexToken]) -> Option<Vec<OwnedLexToken>> {
    let word_view = TokenWordView::new(tokens);
    let words = word_view.to_word_refs();
    if words.len() <= SACRIFICED_AS_IT_ENTERED_SUFFIX.len()
        || !crate::word_primitives::parse_sequence_suffix(&words, SACRIFICED_AS_IT_ENTERED_SUFFIX)
    {
        return None;
    }
    let base_word_count = words.len() - SACRIFICED_AS_IT_ENTERED_SUFFIX.len();
    let base_token_end = word_view.map_word_or_end_to_token_boundary(base_word_count)?;
    Some(super::util::trim_commas(&tokens[..base_token_end]))
}

fn apply_sacrificed_as_it_entered_relation(
    mut filter: ObjectFilter,
    present: bool,
) -> ObjectFilter {
    if present {
        filter = filter.match_tagged(
            "sacrificed_0",
            TaggedOpbjectRelation::IsTaggedObjectSacrificedAsSourceEntered,
        );
    }
    filter
}

fn apply_sacrificed_card_type_relation(
    mut filter: ObjectFilter,
    tokens: &[OwnedLexToken],
) -> ObjectFilter {
    let words = TokenWordView::new(tokens).to_word_refs();
    let shares_card_type = crate::word_primitives::any_sequence_occurs(
        &words,
        &[
            &["shares", "a", "card", "type"],
            &["shares", "card", "type", "with"],
        ],
    );
    let references_sacrificed_permanent = crate::word_primitives::any_sequence_occurs(
        &words,
        &[
            &["sacrificed", "permanent"],
            &["the", "sacrificed", "permanent"],
        ],
    );
    if shares_card_type && references_sacrificed_permanent {
        filter = filter.match_tagged("sacrificed_0", TaggedOpbjectRelation::SharesCardType);
    }
    filter
}

fn deduplicate_tagged_constraints(mut filter: ObjectFilter) -> ObjectFilter {
    for branch in &mut filter.any_of {
        *branch = deduplicate_tagged_constraints(std::mem::take(branch));
    }
    let mut unique = Vec::with_capacity(filter.tagged_constraints.len());
    for constraint in filter.tagged_constraints.drain(..) {
        if !unique.contains(&constraint) {
            unique.push(constraint);
        }
    }
    filter.tagged_constraints = unique;
    filter
}

fn original_printing_set_word_span(words: &[&str]) -> Option<(usize, std::ops::Range<usize>)> {
    if words.last().copied() != Some("expansion") {
        return None;
    }
    let set_end = words.len().checked_sub(1)?;
    for suffix_start in (0..set_end).rev() {
        let prefix_end = suffix_start.checked_add(ORIGINAL_PRINTING_SET_PREFIX.len())?;
        if prefix_end < set_end
            && words.get(suffix_start..prefix_end) == Some(ORIGINAL_PRINTING_SET_PREFIX)
        {
            return Some((suffix_start, prefix_end..set_end));
        }
    }
    None
}

fn title_case_set_surface(surface: &str) -> String {
    let surface = surface.trim();
    if surface.chars().any(char::is_uppercase) {
        return surface.to_string();
    }
    surface
        .split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn split_original_printing_set_tokens(
    tokens: &[OwnedLexToken],
) -> Option<(Vec<OwnedLexToken>, String)> {
    let word_view = TokenWordView::new(tokens);
    let words = word_view.to_word_refs();
    let (suffix_start, set_words) = original_printing_set_word_span(&words)?;
    let suffix_token_start = word_view.map_word_or_end_to_token_boundary(suffix_start)?;
    let set_tokens = word_view.token_span_for_words(set_words.start, set_words.end)?;
    let set_name = title_case_set_surface(&render_token_slice(&tokens[set_tokens]));
    if set_name.is_empty() {
        return None;
    }
    Some((
        super::util::trim_commas(&tokens[..suffix_token_start]),
        set_name,
    ))
}

fn split_original_printing_set_words<'a>(words: &'a [&'a str]) -> Option<(&'a [&'a str], String)> {
    let (suffix_start, set_words) = original_printing_set_word_span(words)?;
    let set_name = title_case_set_surface(&words[set_words].join(" "));
    (!set_name.is_empty()).then(|| (&words[..suffix_start], set_name))
}

fn apply_original_printing_set(mut filter: ObjectFilter, set_name: Option<String>) -> ObjectFilter {
    if let Some(set_name) = set_name {
        filter.name_originally_printed_in_set = Some(set_name);
    }
    filter
}

fn split_trailing_where_x_filter_clause(tokens: &[OwnedLexToken]) -> Option<&[OwnedLexToken]> {
    let view = TokenWordView::new(tokens);
    let where_word = view.parse_phrase_start(&["where", "x", "is"])?;
    let where_token = view.map_word_to_token_start(where_word)?;
    let comma = where_token.checked_sub(1)?;
    (comma > 0
        && tokens.get(comma).is_some_and(OwnedLexToken::is_comma)
        && view.len() > where_word + 3)
        .then_some(&tokens[..comma])
}

fn excluded_cast_origin_zone(tokens: &[OwnedLexToken]) -> Option<crate::Zone> {
    let words = parser_token_word_refs(tokens);
    let origin_start = crate::word_primitives::parse_last_sequence_start(
        &words,
        &["that", "wasnt", "cast", "from"],
    )
    .map(|index| index + 4)
    .or_else(|| {
        crate::word_primitives::parse_last_sequence_start(
            &words,
            &["that", "was", "not", "cast", "from"],
        )
        .map(|index| index + 5)
    })?;
    let origin_words = words.get(origin_start..)?;

    let last = origin_words.last().copied()?;
    match last {
        "hand" => Some(crate::Zone::Hand),
        "graveyard" | "graveyards" => Some(crate::Zone::Graveyard),
        "library" | "libraries" => Some(crate::Zone::Library),
        "exile" => Some(crate::Zone::Exile),
        "battlefield" => Some(crate::Zone::Battlefield),
        "stack" => Some(crate::Zone::Stack),
        "ante" => Some(crate::Zone::Ante),
        "game"
            if crate::word_primitives::parse_sequence_suffix(
                origin_words,
                &["outside", "the", "game"],
            ) =>
        {
            Some(crate::Zone::OutsideGame)
        }
        "zone"
            if crate::word_primitives::parse_sequence_suffix(
                origin_words,
                &["command", "zone"],
            ) =>
        {
            Some(crate::Zone::Command)
        }
        _ => None,
    }
}

fn positive_cast_origin_zone(tokens: &[OwnedLexToken]) -> Option<crate::Zone> {
    let words = parser_token_word_refs(tokens);
    let from = crate::word_primitives::parse_last_sequence_start(&words, &["cast", "from"])
        .map(|index| index + 2)?;
    let origin_words = words.get(from..)?;

    if crate::word_primitives::parse_sequence_prefix(origin_words, &["outside", "the", "game"]) {
        return Some(crate::Zone::OutsideGame);
    }
    if crate::word_primitives::parse_any_sequence_prefix(
        origin_words,
        &[&["the", "command", "zone"], &["command", "zone"]],
    ) {
        return Some(crate::Zone::Command);
    }
    // The origin is a prefix of the remaining subject, not necessarily its
    // final word: `from your hand with mana value 7 or greater` must retain
    // Hand even though characteristic qualifiers follow it.
    origin_words
        .iter()
        .take(4)
        .find_map(|word| super::util::parse_zone_word(word))
}

fn object_filter_word_is_any(word: &str, candidates: &[&str]) -> bool {
    candidates.contains(&word)
}

fn object_filter_word_is_other_or_another(word: &str) -> bool {
    object_filter_word_is_any(word, &["other", "another"])
}

fn preserve_union_surface(filter: &mut ObjectFilter, tokens: &[OwnedLexToken]) {
    if tokens.iter().any(|token| token.is_word("and/or")) {
        filter.set_union_connective(ObjectFilterUnionConnective::AndOr);
    }

    let selector_member_count = filter.card_types.len()
        + filter.all_card_types.len()
        + filter.subtypes.len()
        + filter.all_subtypes.len()
        + filter.supertypes.len();
    let words = parser_token_word_refs(tokens);
    let is_selector_member = |word: &str| {
        parse_card_type(word).is_some_and(|card_type| {
            filter.card_types.iter().any(|item| item == &card_type)
                || filter.all_card_types.iter().any(|item| item == &card_type)
        }) || parse_subtype_flexible(word).is_some_and(|subtype| {
            filter.subtypes.iter().any(|item| item == &subtype)
                || filter.all_subtypes.iter().any(|item| item == &subtype)
        }) || parse_supertype_word(word)
            .is_some_and(|supertype| filter.supertypes.iter().any(|item| item == &supertype))
    };
    let has_plain_and = words.iter().enumerate().any(|(index, word)| {
        *word == "and"
            && index > 0
            && words
                .get(index - 1)
                .is_some_and(|left| is_selector_member(left))
            && words
                .get(index + 1)
                .is_some_and(|right| is_selector_member(right))
    });
    let has_disjunction = tokens
        .iter()
        .any(|token| token.is_word("or") || token.is_word("and/or"));
    if selector_member_count >= 2 && has_plain_and && !has_disjunction {
        // A shared terminal noun can flatten "instant and sorcery card" to
        // one executable filter. Retain the inclusive authored conjunction
        // even though no `any_of` branches are needed for matching.
        filter.set_conjunctive_set_surface(true);
    }

    let subtype_member_count = filter.subtypes.len() + filter.excluded_subtypes.len();
    if subtype_member_count < 2 {
        return;
    }

    let serial_or = subtype_member_count >= 3
        && filter.union_connective() == ObjectFilterUnionConnective::Or
        && tokens.iter().any(OwnedLexToken::is_comma)
        && crate::word_primitives::sequence_occurs(&words, &["or"]);
    filter.set_serial_or_list_surface(serial_or);

    let first_member = crate::slice_primitives::select_position(&words, |word| {
        parse_subtype_flexible(word).is_some_and(|subtype| {
            filter.subtypes.iter().any(|item| item == &subtype)
                || filter.excluded_subtypes.iter().any(|item| item == &subtype)
        })
    });
    let shared_article = first_member
        .and_then(|index| index.checked_sub(1))
        .and_then(|index| words.get(index))
        .is_some_and(|word| matches!(*word, "a" | "an"))
        && !filter.has_explicit_union_branch_articles();
    filter.set_shared_indefinite_article_surface(shared_article);
}

fn preserve_controller_qualifier_order_words(filter: &mut ObjectFilter, words: &[&str]) {
    if filter.controller.is_none() {
        return;
    }

    // This is presentation-only metadata: retain any authored restrictive
    // phrase that precedes controller scope, rather than limiting the signal
    // to keyword-ability tails. Numeric comparisons ("with power 4 or
    // greater") and chosen-characteristic predicates ("of the chosen type")
    // need the same ordering treatment.
    let qualifier = words.iter().enumerate().find_map(|(idx, word)| {
        (matches!(
            *word,
            "with" | "without" | "chosen" | "that's" | "named" | "attached" | "cast" | "put"
        ) || matches!(*word, "that" | "which")
            && words.get(idx + 1).is_some_and(|next| {
                matches!(
                    *next,
                    "has"
                        | "have"
                        | "is"
                        | "isn't"
                        | "isnt"
                        | "was"
                        | "were"
                        | "shares"
                        | "shared"
                        | "entered"
                        | "dealt"
                        | "attacked"
                        | "blocked"
                        | "died"
                )
            }))
        .then_some(idx)
    });
    let controller = crate::slice_primitives::select_position(words, |word| {
        matches!(*word, "control" | "controls")
    });
    filter.set_controller_after_qualifiers_surface(
        matches!((qualifier, controller), (Some(qualifier), Some(controller)) if qualifier < controller),
    );
}

fn preserve_controller_qualifier_order(filter: &mut ObjectFilter, tokens: &[OwnedLexToken]) {
    let words = parser_token_word_refs(tokens);
    preserve_controller_qualifier_order_words(filter, &words);
}

/// Parse an explicitly generic card noun whose only characteristic is a
/// typed tail decoration, such as "a card with doctor's companion". Sending
/// the whole phrase through the characteristic grammar lets words inside the
/// ability name masquerade as card characteristics (for example, `doctor's`
/// as the Doctor subtype) before the same tail is also recognized as an
/// ability marker.
fn parse_generic_card_tail_filter(tokens: &[OwnedLexToken]) -> Option<ObjectFilter> {
    let words = TokenWordView::new(tokens).to_word_refs();
    let words = non_article_word_refs(&words);
    if words.len() > 2
        && matches!(words[0], "card" | "cards")
        && matches!(words[1], "with" | "without")
        && let Some((constraint, consumed)) = parse_filter_keyword_constraint_words(&words[2..])
        && consumed == words.len() - 2
    {
        let mut filter = ObjectFilter::default();
        filter.set_explicit_card_noun(true);
        apply_filter_keyword_constraint(&mut filter, constraint, words[1] == "without");
        return Some(filter);
    }

    let split = parse_filter_tail_decoration_tokens(tokens)?;
    let base_words = parser_token_word_refs(&split.base_tokens);
    let base_words = non_article_word_refs(&base_words);
    if !crate::word_primitives::parse_any_sequence_complete(&base_words, &[&["card"], &["cards"]]) {
        return None;
    }

    let mut filter = ObjectFilter::default();
    filter.set_explicit_card_noun(true);
    apply_filter_tail_decoration(&mut filter, split.decoration);
    Some(filter)
}

fn normalize_generic_card_ability_tail(tokens: &[OwnedLexToken], filter: &mut ObjectFilter) {
    let words = TokenWordView::new(tokens).to_word_refs();
    let words = non_article_word_refs(&words);
    let generic_card_tail = words.len() > 2
        && matches!(words[0], "card" | "cards")
        && matches!(words[1], "with" | "without");
    let has_typed_ability_constraint = !filter.static_abilities.is_empty()
        || !filter.excluded_static_abilities.is_empty()
        || !filter.ability_markers.is_empty()
        || !filter.excluded_ability_markers.is_empty();
    if !generic_card_tail || !has_typed_ability_constraint {
        return;
    }

    // The noun before the tail is deliberately untyped. Characteristic words
    // found inside an ability name therefore cannot also constrain the card's
    // type (for example, Doctor in "doctor's companion").
    // Only clear the battlefield zone that was inferred from a characteristic
    // word inside the ability name (for example, Doctor in "doctor's
    // companion"). Explicit source locations such as "card with cycling from
    // your graveyard" are semantic and must survive this normalization.
    if filter.zone == Some(crate::Zone::Battlefield)
        && !crate::word_primitives::sequence_occurs(&words, &["battlefield"])
    {
        filter.zone = None;
    }
    filter.card_types.clear();
    filter.all_card_types.clear();
    filter.excluded_card_types.clear();
    filter.subtypes.clear();
    filter.all_subtypes.clear();
    filter.excluded_subtypes.clear();
    filter.supertypes.clear();
    filter.excluded_supertypes.clear();
    filter.set_explicit_card_noun(true);
}

/// Parse an explicit union whose Oracle text repeats a complete card noun for
/// every arm, such as "a Doctor card, a card with cycling, or a Vehicle card"
/// or "Assassin card or card with freerunning from your graveyard."
/// The ordinary characteristic-union grammar intentionally merges shorter
/// forms like "an instant or sorcery card"; repeated complete nouns instead
/// need independent filters so branch-local predicates are not lost.
fn parse_explicit_card_filter_disjunction(
    tokens: &[OwnedLexToken],
    other: bool,
) -> Result<Option<ObjectFilter>, CardTextError> {
    let segments = split_lexed_slices_on_or(tokens);
    let explicit_branch_articles = segments.iter().all(|segment| {
        parser_token_word_refs(segment)
            .first()
            .is_some_and(|word| matches!(*word, "a" | "an"))
    });
    if segments.len() < 2
        || !segments.iter().all(|segment| {
            let words = parser_token_word_refs(segment);
            words.iter().any(|word| matches!(*word, "card" | "cards"))
        })
    {
        return Ok(None);
    }

    let mut arms = Vec::with_capacity(segments.len());
    for &segment in &segments {
        arms.push(parse_object_filter(segment, other)?);
    }

    // A source-zone phrase authored only after the last complete union arm
    // scopes the whole selection: "Assassin card or card with freerunning
    // from your graveyard." The standalone arm parser necessarily gives the
    // earlier card noun its ordinary battlefield default. Lift the explicit
    // nonbattlefield scope only when every earlier arm has exactly that
    // default and the final arm's own words name its parsed zone.
    let final_words = segments
        .last()
        .copied()
        .map(parser_token_word_refs)
        .unwrap_or_default();
    let shared_zone = arms.last().and_then(|arm| arm.zone).filter(|zone| {
        *zone != Zone::Battlefield
            && final_words.iter().any(|word| {
                matches!(
                    (*zone, *word),
                    (Zone::Graveyard, "graveyard")
                        | (Zone::Hand, "hand")
                        | (Zone::Library, "library")
                        | (Zone::Exile, "exile")
                        | (Zone::Command, "command")
                )
            })
    });
    let shared_owner = arms.last().and_then(|arm| arm.owner.clone());
    if let Some(shared_zone) = shared_zone
        && arms[..arms.len() - 1].iter().all(|arm| {
            matches!(arm.zone, None | Some(Zone::Battlefield))
                && arm.owner.is_none()
                && arm.controller.is_none()
        })
    {
        for arm in &mut arms {
            arm.zone = None;
            arm.owner = None;
        }
        let mut union = ObjectFilter {
            zone: Some(shared_zone),
            owner: shared_owner,
            any_of: arms,
            ..ObjectFilter::default()
        };
        preserve_union_surface(&mut union, tokens);
        union.set_explicit_union_branch_articles(explicit_branch_articles);
        return Ok(Some(union));
    }

    let mut union = ObjectFilter {
        any_of: arms,
        ..ObjectFilter::default()
    };
    preserve_union_surface(&mut union, tokens);
    union.set_explicit_union_branch_articles(explicit_branch_articles);
    Ok(Some(union))
}

/// Preserve a subtype-or-colored-permanent selector as a real disjunction.
///
/// A shared terminal noun normally lets the characteristic grammar flatten
/// forms such as `instant or sorcery spell`. That is not valid for
/// `an Island or blue permanent`: the subtype belongs to one arm while the
/// color and permanent domain belong to the other. Reuse the branch-scoped
/// union parser, but claim only this mixed selector shape so ordinary shared
/// terminal type lists keep their established representation.
fn parse_subtype_or_colored_permanent_disjunction(
    tokens: &[OwnedLexToken],
    other: bool,
) -> Option<ObjectFilter> {
    let words = parser_token_word_refs(tokens);
    if !words
        .iter()
        .any(|word| matches!(*word, "permanent" | "permanents"))
        || words.iter().filter(|word| **word == "or").count() != 1
    {
        return None;
    }

    let union = parse_branch_scoped_object_filter_union_lexed(tokens, other)?;
    let has_subtype_arm = union.any_of.iter().any(|branch| {
        !branch.subtypes.is_empty()
            && branch.colors.is_none()
            && branch.card_types.is_empty()
            && branch.all_card_types.is_empty()
    });
    let has_colored_permanent_arm = union.any_of.iter().any(|branch| {
        branch.colors.is_some()
            && (!branch.card_types.is_empty() || !branch.all_card_types.is_empty())
    });
    (has_subtype_arm && has_colored_permanent_arm).then_some(union)
}

pub(super) fn slice_has<T: PartialEq>(items: &[T], expected: &T) -> bool {
    crate::slice_primitives::contains(items, expected)
}

pub(super) fn set_has<T: Eq + std::hash::Hash>(
    items: &std::collections::HashSet<T>,
    expected: &T,
) -> bool {
    items.iter().any(|item| item == expected)
}

pub(super) fn push_unique<T: PartialEq>(items: &mut Vec<T>, value: T) {
    if !items.contains(&value) {
        items.push(value);
    }
}

pub(super) fn parse_attached_reference_or_another_disjunction(
    tokens: &[OwnedLexToken],
) -> Result<Option<ObjectFilter>, CardTextError> {
    let segments = split_lexed_slices_on_or(tokens);
    if segments.len() != 2 {
        return Ok(None);
    }

    let first_word_view = TokenWordView::new(segments[0]);
    let first_words = non_article_word_refs(&first_word_view.to_word_refs());
    let second_word_view = TokenWordView::new(segments[1]);
    let second_words = non_article_word_refs(&second_word_view.to_word_refs());

    let first_is_attached_reference = first_words.first().is_some_and(|word| {
        object_filter_word_is_any(word, &["attached", "equipped", "enchanted"])
    });
    let second_starts_with_other = second_words
        .first()
        .is_some_and(|word| object_filter_word_is_other_or_another(word));
    if !first_is_attached_reference || !second_starts_with_other {
        return Ok(None);
    }

    let first_other = first_words
        .first()
        .is_some_and(|word| object_filter_word_is_other_or_another(word));
    let second_other = second_words
        .first()
        .is_some_and(|word| object_filter_word_is_other_or_another(word));

    let first_filter = parse_object_filter(segments[0], first_other)?;
    let second_filter = parse_object_filter(segments[1], second_other)?;

    let mut disjunction = ObjectFilter::default();
    disjunction.any_of = vec![first_filter, second_filter];
    Ok(Some(disjunction))
}

/// A terminal `spell` noun scopes over every characteristic in a preceding
/// coordinated list.
///
/// For example, `instant or sorcery spell you control with mana value X`
/// names one stack-object filter. Treating the two card types as independently
/// scoped union arms strands `spell`, controller, and mana-value facts on only
/// the final arm. The ordinary complete-filter grammar already preserves the
/// shared typed facts, so keep this shape out of the branch-scoped union path.
pub fn has_shared_terminal_object_noun(tokens: &[OwnedLexToken]) -> bool {
    let is_shared_noun = |token: &OwnedLexToken| {
        token.is_word("card")
            || token.is_word("cards")
            || token.is_word("spell")
            || token.is_word("spells")
            || token.is_word("permanent")
            || token.is_word("permanents")
    };
    let Some(noun_idx) = crate::slice_primitives::select_last_position(tokens, is_shared_noun)
    else {
        return false;
    };
    if tokens[..=noun_idx]
        .iter()
        .filter(|token| is_shared_noun(token))
        .count()
        != 1
    {
        return false;
    }
    if !tokens[..noun_idx]
        .iter()
        .any(|token| token.is_word("and") || token.is_word("or") || token.is_word("and/or"))
    {
        return false;
    }

    let Some(head) = parse_simple_object_filter_lexed(&tokens[..=noun_idx], false) else {
        return false;
    };
    // A branch-local exclusion such as `non-Aura` is represented by the
    // simple grammar as selector-only `any_of` arms. Those arms still share
    // the one terminal `card` noun and its trailing domain; treating them as
    // independently scoped domains strands the graveyard and mana-value
    // qualifiers on only the final arm.
    let selector_only_card_type_union = !head.any_of.is_empty()
        && head.any_of.iter().all(|branch| {
            if branch.card_types.len() != 1 || !branch.all_card_types.is_empty() {
                return false;
            }
            let mut remainder = branch.clone();
            remainder.card_types.clear();
            remainder.excluded_card_types.clear();
            remainder.excluded_subtypes.clear();
            remainder.excluded_supertypes.clear();
            remainder.excluded_colors = ColorSet::new();
            remainder == ObjectFilter::default()
        });
    let characteristic_count = if selector_only_card_type_union {
        head.any_of
            .iter()
            .map(|branch| branch.card_types.len())
            .sum()
    } else {
        head.card_types.len()
            + head.all_card_types.len()
            + head.subtypes.len()
            + head.all_subtypes.len()
            + head.supertypes.len()
    };
    let shared_spell_noun = tokens[noun_idx].is_word("spell") || tokens[noun_idx].is_word("spells");
    characteristic_count >= 2
        && (head.any_of.is_empty() || selector_only_card_type_union)
        && head.excluded_card_types.is_empty()
        && head.excluded_subtypes.is_empty()
        && head.excluded_supertypes.is_empty()
        && (!shared_spell_noun
            || (head.zone == Some(crate::Zone::Stack)
                && head.stack_kind == Some(crate::filter::StackObjectKind::Spell)))
}

fn has_requantified_comma_collection(tokens: &[OwnedLexToken]) -> bool {
    tokens.iter().filter(|token| token.is_comma()).count() >= 2
        && tokens
            .iter()
            .filter(|token| token.is_word("all") || token.is_word("each"))
            .count()
            >= 2
}

fn preserve_explicit_spell_domain(filter: &mut ObjectFilter, tokens: &[OwnedLexToken]) {
    let words = TokenWordView::new(tokens).to_word_refs();
    let has_domain_noun = words.iter().enumerate().any(|(index, word)| {
        matches!(*word, "spell" | "spells")
            && !index
                .checked_sub(1)
                .and_then(|previous| words.get(previous))
                .is_some_and(|previous| matches!(*previous, "this" | "that" | "the" | "triggering"))
    });
    if has_domain_noun {
        filter.zone = Some(crate::Zone::Stack);
        filter.stack_kind = Some(crate::filter::StackObjectKind::Spell);
        filter.has_mana_cost = true;
    }
}

fn preserve_public_spell_filter_facts(filter: &mut ObjectFilter, tokens: &[OwnedLexToken]) {
    if let Some(zone) = excluded_cast_origin_zone(tokens) {
        filter.excluded_cast_origin_zone = Some(zone);
        preserve_explicit_spell_domain(filter, tokens);
    }

    let words = parser_token_word_refs(tokens);
    if filter.zone == Some(Zone::Stack)
        && filter.stack_kind == Some(crate::filter::StackObjectKind::Spell)
        && crate::word_primitives::parse_sequence_suffix(&words, &["with", "a", "single", "target"])
    {
        filter.target_count = Some(crate::effect::ChoiceCount::exactly(1));
    }
}

fn preserve_terminal_characteristic_union_domain(
    filter: &mut ObjectFilter,
    tokens: &[OwnedLexToken],
) {
    let words = parser_token_word_refs(tokens);
    let has_one_shared_card_noun = words
        .iter()
        .filter(|word| matches!(**word, "card" | "cards"))
        .count()
        == 1;
    let characteristic_only_union = filter.any_of.len() >= 2
        && filter.any_of.iter().all(|branch| {
            branch.card_types.is_empty()
                && branch.all_card_types.is_empty()
                && branch.controller.is_none()
                && (!branch.supertypes.is_empty() || !branch.subtypes.is_empty())
        });
    if !has_one_shared_card_noun || !characteristic_only_union {
        return;
    }

    let mut shared_zone = filter.zone;
    if shared_zone.is_none() {
        for branch in &filter.any_of {
            if let Some(zone) = branch.zone
                && zone != Zone::Battlefield
            {
                shared_zone = Some(zone);
                break;
            }
        }
    }
    let shared_owner = filter
        .owner
        .clone()
        .or_else(|| filter.any_of.iter().find_map(|branch| branch.owner.clone()));
    if shared_zone.is_none() && shared_owner.is_none() {
        return;
    }

    filter.zone = shared_zone;
    filter.owner = shared_owner;
    for branch in &mut filter.any_of {
        branch.zone = None;
        branch.owner = None;
    }
}

fn finalize_public_object_filter(
    mut filter: ObjectFilter,
    tokens: &[OwnedLexToken],
) -> ObjectFilter {
    apply_phyrexian_mana_cost_predicate(&mut filter, tokens);
    super::grammar::filters::apply_supertype_or_mana_capability_union(&mut filter, tokens);
    preserve_combat_role_disjunction(&mut filter, tokens);
    preserve_public_spell_filter_facts(&mut filter, tokens);
    preserve_terminal_characteristic_union_domain(&mut filter, tokens);
    deduplicate_tagged_constraints(filter)
}

/// Preserve a shared-terminal combat-role union as alternatives rather than
/// an impossible intersection. The ordinary atom parser intentionally
/// carries the terminal noun backward across `attacking or blocking
/// creature`, but its flat boolean representation would otherwise require a
/// candidate to be both attacking and blocking at once.
fn preserve_combat_role_disjunction(filter: &mut ObjectFilter, tokens: &[OwnedLexToken]) {
    if !filter.any_of.is_empty() || !filter.attacking || !filter.blocking {
        return;
    }
    let words = parser_token_word_refs(tokens);
    let attacking_first =
        crate::word_primitives::sequence_occurs(&words, &["attacking", "or", "blocking"]);
    let blocking_first =
        crate::word_primitives::sequence_occurs(&words, &["blocking", "or", "attacking"]);
    if !attacking_first && !blocking_first {
        return;
    }

    filter.attacking = false;
    filter.blocking = false;
    let card_types = std::mem::take(&mut filter.card_types);
    let all_card_types = std::mem::take(&mut filter.all_card_types);
    let explicit_card_type_noun = filter.explicit_card_type_noun();
    filter.set_explicit_card_type_noun(None);
    let mut attacking = ObjectFilter {
        attacking: true,
        ..ObjectFilter::default()
    };
    let mut blocking = ObjectFilter {
        blocking: true,
        ..ObjectFilter::default()
    };
    for branch in [&mut attacking, &mut blocking] {
        branch.card_types = card_types.clone();
        branch.all_card_types = all_card_types.clone();
        branch.set_explicit_card_type_noun(explicit_card_type_noun);
    }
    filter.any_of = if attacking_first {
        vec![attacking, blocking]
    } else {
        vec![blocking, attacking]
    };
    filter.set_union_connective(ObjectFilterUnionConnective::Or);
}

pub fn parse_object_filter(
    tokens: &[OwnedLexToken],
    other: bool,
) -> Result<ObjectFilter, CardTextError> {
    if let Some((base_tokens, card_name)) = split_drafted_color_qualifier_tokens(tokens) {
        let mut filter = parse_object_filter_inner(&base_tokens, other)?;
        filter.colors_chosen_while_drafting_named = Some(card_name);
        return Ok(finalize_public_object_filter(filter, &base_tokens));
    }
    let filter = parse_object_filter_inner(tokens, other)?;
    Ok(finalize_public_object_filter(filter, tokens))
}

fn apply_phyrexian_mana_cost_predicate(filter: &mut ObjectFilter, tokens: &[OwnedLexToken]) {
    if tokens.iter().enumerate().any(|(index, token)| {
        token
            .mana_group_inner()
            .is_some_and(|inner| inner.eq_ignore_ascii_case("H"))
            && tokens
                .get(index + 1)
                .is_some_and(|token| token.is_word("in"))
            && tokens
                .get(index + 2)
                .is_some_and(|token| token.is_word("its") || token.is_word("their"))
            && tokens
                .get(index + 3)
                .is_some_and(|token| token.is_word("mana"))
            && tokens
                .get(index + 4)
                .is_some_and(|token| token.is_word("cost"))
    }) {
        filter.has_mana_cost = true;
        filter.has_phyrexian_mana_symbol = true;
    }
}

#[path = "object_filters/filter_shape_readings.rs"]
mod filter_shape_readings;

fn parse_object_filter_inner(
    tokens: &[OwnedLexToken],
    other: bool,
) -> Result<ObjectFilter, CardTextError> {
    let input = filter_shape_readings::FilterPhrase {
        tokens,
        other,
        read_by_cache: Default::default(),
    };
    match filter_shape_readings::read(&input) {
        crate::recognition::ParseOutcome::Match(matched) => return Ok(matched.value.value),
        crate::recognition::ParseOutcome::NoMatch => {}
        crate::recognition::ParseOutcome::Error(diagnostic) => {
            return Err(diagnostic.into_card_text_error());
        }
    }
    let (entry_sacrifice_tokens, sacrificed_as_it_entered) =
        split_sacrificed_as_it_entered_tokens(tokens)
            .map(|tokens| (tokens, true))
            .unwrap_or_else(|| (tokens.to_vec(), false));
    let tokens = entry_sacrifice_tokens.as_slice();
    let (original_printing_tokens, original_printing_set) =
        split_original_printing_set_tokens(tokens)
            .map(|(tokens, set_name)| (tokens, Some(set_name)))
            .unwrap_or_else(|| (tokens.to_vec(), None));
    let tokens = original_printing_tokens.as_slice();
    let envelope = parse_filter_distinct_names_tokens(tokens);
    let tokens = envelope.core_tokens.as_slice();
    let mut filter = if let Some(split) = parse_filter_tail_decoration_tokens(tokens) {
        let mut filter = super::grammar::filters::parse_object_filter_with_grammar_entrypoint(
            &split.base_tokens,
            other,
        )?;
        preserve_branch_scoped_card_type_union(&mut filter, &split.base_tokens, other);
        apply_filter_tail_decoration(&mut filter, split.decoration);
        filter
    } else {
        let mut filter =
            super::grammar::filters::parse_object_filter_with_grammar_entrypoint(tokens, other)?;
        preserve_branch_scoped_card_type_union(&mut filter, tokens, other);
        filter
    };
    filter = envelope.decorations.apply_distinct_names_only(filter);
    preserve_explicit_spell_domain(&mut filter, tokens);
    if let Some(zone) = excluded_cast_origin_zone(tokens) {
        filter.excluded_cast_origin_zone = Some(zone);
    }
    let is_explicit_stack_spell = filter.zone == Some(Zone::Stack)
        && filter.stack_kind == Some(crate::filter::StackObjectKind::Spell);
    if ((filter.has_mana_cost && filter.cast_by.is_some()) || is_explicit_stack_spell)
        && let Some(zone) = positive_cast_origin_zone(tokens)
    {
        // Filters such as "spells you cast from exile" and "spell cast from
        // a graveyard" describe the spell's cast origin, not its current
        // stack location. Cast-event matching and rendering both consume this
        // zone as provenance.
        filter.zone = Some(zone);
    }
    preserve_union_surface(&mut filter, tokens);
    preserve_controller_qualifier_order(&mut filter, tokens);
    preserve_filter_counter_constraint_surface_tokens(&mut filter, tokens);
    normalize_generic_card_ability_tail(tokens, &mut filter);
    let filter = apply_sacrificed_card_type_relation(filter, tokens);
    Ok(apply_sacrificed_as_it_entered_relation(
        apply_original_printing_set(filter, original_printing_set),
        sacrificed_as_it_entered,
    ))
}

pub fn parse_object_filter_words(
    word_refs: &[&str],
    other: bool,
) -> Result<ObjectFilter, CardTextError> {
    let (entry_sacrifice_words, sacrificed_as_it_entered) = if word_refs.len()
        > SACRIFICED_AS_IT_ENTERED_SUFFIX.len()
        && crate::word_primitives::parse_sequence_suffix(word_refs, SACRIFICED_AS_IT_ENTERED_SUFFIX)
    {
        (
            &word_refs[..word_refs.len() - SACRIFICED_AS_IT_ENTERED_SUFFIX.len()],
            true,
        )
    } else {
        (word_refs, false)
    };
    let (original_printing_words, original_printing_set) =
        split_original_printing_set_words(entry_sacrifice_words)
            .map(|(words, set_name)| (words, Some(set_name)))
            .unwrap_or((entry_sacrifice_words, None));
    let envelope = parse_filter_word_envelope(original_printing_words);
    if let Some(mut filter) = parse_extremum_object_filter_words(&envelope.core_words, other)? {
        preserve_controller_qualifier_order_words(&mut filter, &envelope.core_words);
        preserve_filter_counter_constraint_surface_words(&mut filter, &envelope.core_words);
        return Ok(apply_sacrificed_as_it_entered_relation(
            apply_original_printing_set(envelope.decorations.apply(filter), original_printing_set),
            sacrificed_as_it_entered,
        ));
    }
    if let Some(mut filter) = parse_simple_object_filter_words(&envelope.core_words, other) {
        preserve_controller_qualifier_order_words(&mut filter, &envelope.core_words);
        preserve_filter_counter_constraint_surface_words(&mut filter, &envelope.core_words);
        return Ok(apply_sacrificed_as_it_entered_relation(
            apply_original_printing_set(envelope.decorations.apply(filter), original_printing_set),
            sacrificed_as_it_entered,
        ));
    }
    if let Some(split) = parse_filter_tail_decoration_split_words(&envelope.core_words)
        && let Some(mut filter) = parse_simple_object_filter_words(split.base_words, other)
    {
        apply_filter_tail_decoration(&mut filter, split.decoration);
        preserve_controller_qualifier_order_words(&mut filter, &envelope.core_words);
        preserve_filter_counter_constraint_surface_words(&mut filter, &envelope.core_words);
        return Ok(apply_sacrificed_as_it_entered_relation(
            apply_original_printing_set(envelope.decorations.apply(filter), original_printing_set),
            sacrificed_as_it_entered,
        ));
    }

    // The lexed parser owns envelope normalization for the complex path. The
    // historical-printing suffix has already been preserved as a typed field.
    let tokens = super::lexer::synthetic_word_tokens(original_printing_words.iter().copied());
    let filter = parse_object_filter_lexed(&tokens, other)?;
    Ok(apply_sacrificed_as_it_entered_relation(
        apply_original_printing_set(filter, original_printing_set),
        sacrificed_as_it_entered,
    ))
}

pub fn parse_object_filter_lexed(
    tokens: &[OwnedLexToken],
    other: bool,
) -> Result<ObjectFilter, CardTextError> {
    if let Some((base_tokens, card_name)) = split_drafted_color_qualifier_tokens(tokens) {
        let mut filter = parse_object_filter_lexed_inner(&base_tokens, other)?;
        filter.colors_chosen_while_drafting_named = Some(card_name);
        return Ok(finalize_public_object_filter(filter, &base_tokens));
    }
    let filter = parse_object_filter_lexed_inner(tokens, other)?;
    Ok(finalize_public_object_filter(filter, tokens))
}

#[path = "object_filters/lexed_filter_shape_readings.rs"]
mod lexed_filter_shape_readings;

fn parse_object_filter_lexed_inner(
    tokens: &[OwnedLexToken],
    other: bool,
) -> Result<ObjectFilter, CardTextError> {
    let input = lexed_filter_shape_readings::FilterPhrase {
        tokens,
        other,
        read_by_cache: Default::default(),
    };
    match lexed_filter_shape_readings::read(&input) {
        crate::recognition::ParseOutcome::Match(matched) => return Ok(matched.value.value),
        crate::recognition::ParseOutcome::NoMatch => {}
        crate::recognition::ParseOutcome::Error(diagnostic) => {
            return Err(diagnostic.into_card_text_error());
        }
    }
    let has_shared_terminal_noun = has_shared_terminal_object_noun(tokens);
    let (original_printing_tokens, original_printing_set) =
        split_original_printing_set_tokens(tokens)
            .map(|(tokens, set_name)| (tokens, Some(set_name)))
            .unwrap_or_else(|| (tokens.to_vec(), None));
    let tokens = original_printing_tokens.as_slice();
    let envelope = parse_filter_lexed_envelope(tokens);
    if tokens_contain_permanent_or_suspended_card_disjunction(&envelope.core_tokens) {
        let mut filter = super::grammar::filters::parse_object_filter_with_grammar_entrypoint(
            &envelope.core_tokens,
            other,
        )?;
        preserve_union_surface(&mut filter, &envelope.core_tokens);
        preserve_controller_qualifier_order(&mut filter, &envelope.core_tokens);
        return Ok(apply_original_printing_set(
            envelope.decorations.apply(filter),
            original_printing_set,
        ));
    }
    if !has_shared_terminal_noun
        && let Some(mut filter) = super::grammar::filters::parse_domain_union_object_filter_lexed(
            &envelope.core_tokens,
            other,
        )
    {
        preserve_union_surface(&mut filter, &envelope.core_tokens);
        preserve_controller_qualifier_order(&mut filter, &envelope.core_tokens);
        preserve_filter_counter_constraint_surface_tokens(&mut filter, &envelope.core_tokens);
        return Ok(apply_original_printing_set(
            envelope.decorations.apply(filter),
            original_printing_set,
        ));
    }
    if let Some(mut filter) = parse_extremum_object_filter_lexed(&envelope.core_tokens, other)? {
        preserve_union_surface(&mut filter, &envelope.core_tokens);
        preserve_controller_qualifier_order(&mut filter, &envelope.core_tokens);
        preserve_filter_counter_constraint_surface_tokens(&mut filter, &envelope.core_tokens);
        return Ok(apply_original_printing_set(
            envelope.decorations.apply(filter),
            original_printing_set,
        ));
    }
    if let Some(mut filter) = parse_simple_object_filter_lexed(&envelope.core_tokens, other) {
        preserve_union_surface(&mut filter, &envelope.core_tokens);
        preserve_controller_qualifier_order(&mut filter, &envelope.core_tokens);
        preserve_filter_counter_constraint_surface_tokens(&mut filter, &envelope.core_tokens);
        preserve_explicit_spell_domain(&mut filter, &envelope.core_tokens);
        return Ok(apply_original_printing_set(
            envelope.decorations.apply(filter),
            original_printing_set,
        ));
    }
    let filter = parse_object_filter(&envelope.core_tokens, other)?;
    // Historical behavior intentionally drops the vote-winner tag on this
    // complex fallback while retaining the different-names fact.
    Ok(apply_original_printing_set(
        envelope.decorations.apply_distinct_names_only(filter),
        original_printing_set,
    ))
}

fn tokens_contain_permanent_or_suspended_card_disjunction(tokens: &[OwnedLexToken]) -> bool {
    let words = parser_token_word_refs(tokens);
    let has_permanent = words
        .iter()
        .any(|word| matches!(*word, "permanent" | "permanents"));
    let has_suspended_card = crate::word_primitives::any_sequence_occurs(
        &words,
        &[&["suspended", "card"], &["suspended", "cards"]],
    );
    let has_connector = words
        .iter()
        .any(|word| matches!(*word, "and" | "or" | "and/or"));
    has_permanent && has_suspended_card && has_connector
}

pub fn spell_filter_has_identity(filter: &ObjectFilter) -> bool {
    !filter.card_types.is_empty()
        || !filter.all_card_types.is_empty()
        || !filter.excluded_card_types.is_empty()
        || !filter.subtypes.is_empty()
        || !filter.all_subtypes.is_empty()
        || !filter.excluded_subtypes.is_empty()
        || !filter.supertypes.is_empty()
        || !filter.excluded_supertypes.is_empty()
        || !filter.static_abilities.is_empty()
        || !filter.excluded_static_abilities.is_empty()
        || !filter.ability_markers.is_empty()
        || !filter.excluded_ability_markers.is_empty()
        || filter.chosen_color
        || filter.colors_chosen_while_drafting_named.is_some()
        || filter.chosen_creature_type
        || filter.chosen_card_type
        || filter.excluded_chosen_creature_type
        || filter.excluded_any_chosen_creature_type
        || filter.colors.is_some()
        || filter.required_colors.is_some()
        || filter.colorless
        || filter.multicolored
        || filter.monocolored
        || filter.historic
        || filter.nonhistoric
        || filter.sticker.is_some()
        || filter.color_count.is_some()
        || filter.power.is_some()
        || filter.power_parity.is_some()
        || filter.toughness.is_some()
        || filter.mana_value.is_some()
        || filter.mana_value_parity.is_some()
        || filter.total_counters_parity.is_some()
        || filter.cast_by.is_some()
        || filter.owner.is_some()
        || filter.zone.is_some()
        || filter.name.is_some()
        || filter.name_originally_printed_in_set.is_some()
        || filter.targets_player.is_some()
        || filter.targets_object.is_some()
        || filter.targets_only_player.is_some()
        || filter.targets_only_object.is_some()
        || filter.target_count.is_some()
        || filter.could_be_targeted_by.is_some()
        || filter.alternative_cast.is_some()
        || !filter.characteristic_relations.is_empty()
        || filter.shares_creature_type_with_source
        || !filter.tagged_constraints.is_empty()
        || !filter.any_of.is_empty()
}

pub fn merge_spell_filters(base: &mut ObjectFilter, extra: ObjectFilter) {
    for card_type in extra.card_types {
        push_unique(&mut base.card_types, card_type);
    }
    for card_type in extra.excluded_card_types {
        push_unique(&mut base.excluded_card_types, card_type);
    }
    for subtype in extra.subtypes {
        push_unique(&mut base.subtypes, subtype);
    }
    for subtype in extra.excluded_subtypes {
        push_unique(&mut base.excluded_subtypes, subtype);
    }
    for supertype in extra.supertypes {
        push_unique(&mut base.supertypes, supertype);
    }
    for supertype in extra.excluded_supertypes {
        push_unique(&mut base.excluded_supertypes, supertype);
    }
    for ability in extra.static_abilities {
        push_unique(&mut base.static_abilities, ability);
    }
    for ability in extra.excluded_static_abilities {
        push_unique(&mut base.excluded_static_abilities, ability);
    }
    for marker in extra.ability_markers {
        push_unique(&mut base.ability_markers, marker);
    }
    for marker in extra.excluded_ability_markers {
        push_unique(&mut base.excluded_ability_markers, marker);
    }
    if let Some(colors) = extra.colors {
        let existing = base.colors.unwrap_or_default();
        base.colors = Some(existing.union(colors));
    }
    if let Some(colors) = extra.required_colors {
        let existing = base.required_colors.unwrap_or_default();
        base.required_colors = Some(existing.union(colors));
    }
    base.colorless |= extra.colorless;
    base.multicolored |= extra.multicolored;
    base.monocolored |= extra.monocolored;
    base.historic |= extra.historic;
    base.nonhistoric |= extra.nonhistoric;
    base.has_mana_cost |= extra.has_mana_cost;
    base.has_phyrexian_mana_symbol |= extra.has_phyrexian_mana_symbol;
    base.chosen_color |= extra.chosen_color;
    if base.colors_chosen_while_drafting_named.is_none() {
        base.colors_chosen_while_drafting_named = extra.colors_chosen_while_drafting_named;
    }
    base.chosen_creature_type |= extra.chosen_creature_type;
    base.chosen_card_type |= extra.chosen_card_type;
    base.excluded_chosen_creature_type |= extra.excluded_chosen_creature_type;
    base.excluded_any_chosen_creature_type |= extra.excluded_any_chosen_creature_type;
    if base.color_count.is_none() {
        base.color_count = extra.color_count;
    }
    if base.alternative_cast.is_none() {
        base.alternative_cast = extra.alternative_cast;
    }
    if base.power.is_none() {
        base.power = extra.power;
    }
    if base.power_parity.is_none() {
        base.power_parity = extra.power_parity;
    }
    if base.toughness.is_none() {
        base.toughness = extra.toughness;
    }
    if base.mana_value.is_none() {
        base.mana_value = extra.mana_value;
    }
    if base.mana_value_parity.is_none() {
        base.mana_value_parity = extra.mana_value_parity;
    }
    if base.total_counters_parity.is_none() {
        base.total_counters_parity = extra.total_counters_parity;
    }
    if base.cast_by.is_none() {
        base.cast_by = extra.cast_by;
    }
    if base.excluded_cast_origin_zone.is_none() {
        base.excluded_cast_origin_zone = extra.excluded_cast_origin_zone;
    }
    if base.owner.is_none() {
        base.owner = extra.owner;
    }
    if base.zone.is_none() {
        base.zone = extra.zone;
    }
    if base.name.is_none() {
        base.name = extra.name;
    }
    if base.name_originally_printed_in_set.is_none() {
        base.name_originally_printed_in_set = extra.name_originally_printed_in_set;
    }
    if base.targets_player.is_none() {
        base.targets_player = extra.targets_player;
    }
    if base.targets_object.is_none() {
        base.targets_object = extra.targets_object;
    }
    base.targets_any_of |= extra.targets_any_of;
    if base.targets_only_player.is_none() {
        base.targets_only_player = extra.targets_only_player;
    }
    if base.targets_only_object.is_none() {
        base.targets_only_object = extra.targets_only_object;
    }
    base.targets_only_any_of |= extra.targets_only_any_of;
    if base.target_count.is_none() {
        base.target_count = extra.target_count;
    }
    if base.could_be_targeted_by.is_none() {
        base.could_be_targeted_by = extra.could_be_targeted_by;
    }
    base.shares_creature_type_with_source |= extra.shares_creature_type_with_source;
    for relation in extra.characteristic_relations {
        crate::slice_primitives::push_unique(&mut base.characteristic_relations, relation);
    }
    for constraint in extra.tagged_constraints {
        crate::slice_primitives::push_unique(&mut base.tagged_constraints, constraint);
    }
    for branch in extra.any_of {
        crate::slice_primitives::push_unique(&mut base.any_of, branch);
    }
}

pub fn is_comparison_or_delimiter(tokens: &[OwnedLexToken], idx: usize) -> bool {
    if !token_slice_at_is(tokens, idx, "or") {
        return false;
    }
    let previous_word = (0..idx).rev().find_map(|i| tokens[i].as_word());
    let next_word = tokens.get(idx + 1).and_then(OwnedLexToken::as_word);
    if next_word
        .is_some_and(|word| object_filter_word_is_any(word, &["less", "greater", "more", "fewer"]))
    {
        return true;
    }
    if previous_word.is_some_and(|word| word == "than")
        && next_word.is_some_and(|word| word == "equal")
    {
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TagKey;
    use crate::mana::ManaSymbol;
    use crate::util::tokenize_line;

    fn assert_supertype_or_mana_capability_union(filter: &ObjectFilter) {
        assert_eq!(filter.card_types, [CardType::Land], "{filter:#?}");
        assert!(filter.supertypes.is_empty(), "{filter:#?}");
        assert_eq!(filter.any_of.len(), 2, "{filter:#?}");
        assert!(filter.any_of.iter().any(|branch| {
            branch.supertypes == [crate::types::Supertype::Snow]
                && branch.could_produce_mana.is_empty()
        }));
        assert!(filter.any_of.iter().any(|branch| {
            branch.supertypes.is_empty() && branch.could_produce_mana == [ManaSymbol::Colorless]
        }));
    }

    #[test]
    fn public_filter_roots_apply_terminal_supertype_or_mana_capability_union() {
        let tokens = tokenize_line("land that is snow or could produce {C}", 0);

        let ordinary = parse_object_filter(&tokens, false)
            .expect("ordinary family filter root should preserve the capability union");
        assert_supertype_or_mana_capability_union(&ordinary);

        let lexed = parse_object_filter_lexed(&tokens, false)
            .expect("lexed family filter root should preserve the capability union");
        assert_supertype_or_mana_capability_union(&lexed);
    }

    fn assert_distinct_combat_damage_controller_filter(filter: &ObjectFilter) {
        assert_eq!(filter.excluded_card_types, [CardType::Land], "{filter:#?}");
        assert!(filter.subtypes.is_empty(), "{filter:#?}");
        let Some(PlayerFilter::WasDealtCombatDamageByDistinctSourcesThisTurn {
            base,
            sources,
            minimum,
        }) = filter.controller.as_ref()
        else {
            panic!("expected historical controller filter, got {filter:#?}");
        };
        assert_eq!(base.as_ref(), &PlayerFilter::Any);
        assert_eq!(*minimum, 3);
        assert_eq!(sources.subtypes, [Subtype::Pirate], "{sources:#?}");
        assert_eq!(
            filter.description(),
            "a nonland permanent controlled by a player who was dealt combat damage by three or more Pirates this turn"
        );
    }

    #[test]
    fn public_filter_roots_preserve_distinct_combat_damage_controller_scope() {
        let tokens = tokenize_line(
            "nonland permanent controlled by a player who was dealt combat damage by three or more Pirates this turn",
            0,
        );

        let ordinary = parse_object_filter(&tokens, false)
            .expect("ordinary root should preserve the historical controller filter");
        assert_distinct_combat_damage_controller_filter(&ordinary);

        let lexed = parse_object_filter_lexed(&tokens, false)
            .expect("lexed root should preserve the historical controller filter");
        assert_distinct_combat_damage_controller_filter(&lexed);
    }

    #[test]
    fn public_filter_roots_preserve_bare_single_target_spell_arity() {
        let tokens = tokenize_line("instant or sorcery spell with a single target", 0);

        for filter in [
            parse_object_filter(&tokens, false)
                .expect("ordinary family root should preserve bare target arity"),
            parse_object_filter_lexed(&tokens, false)
                .expect("lexed family root should preserve bare target arity"),
        ] {
            assert_eq!(filter.zone, Some(Zone::Stack), "{filter:#?}");
            assert_eq!(
                filter.stack_kind,
                Some(crate::filter::StackObjectKind::Spell),
                "{filter:#?}"
            );
            assert_eq!(
                filter.target_count,
                Some(crate::effect::ChoiceCount::exactly(1)),
                "{filter:#?}"
            );
            assert_eq!(
                filter.description(),
                "instant or sorcery spell with a single target"
            );
        }

        let ordinary = tokenize_line("instant or sorcery spell", 0);
        let ordinary = parse_object_filter(&ordinary, false)
            .expect("ordinary spell-domain near miss should remain parseable");
        assert_eq!(ordinary.target_count, None, "{ordinary:#?}");
    }

    #[test]
    fn couldnt_attack_exception_is_an_executable_eligibility_constraint() {
        let tokens = tokenize_line(
            "untapped creatures that didn't attack this turn except for creatures that couldn't attack",
            0,
        );
        let filter = parse_object_filter(&tokens, false)
            .expect("attack-eligibility exception should remain one creature filter");

        assert_eq!(filter.card_types, [CardType::Creature], "{filter:#?}");
        assert!(filter.excluded_card_types.is_empty(), "{filter:#?}");
        assert!(filter.untapped, "{filter:#?}");
        assert!(filter.didnt_attack_this_turn, "{filter:#?}");
        assert!(filter.could_have_attacked_this_turn, "{filter:#?}");
        assert_eq!(
            filter.description(),
            "untapped creature that didn't attack this turn, except for creatures that couldn't attack"
        );
    }

    #[test]
    fn hyphenated_compound_subtype_survives_an_attacking_filter() {
        let tokens = tokenize_line("attacking Assembly-Worker", 0);
        let filter = parse_object_filter(&tokens, false)
            .expect("hyphenated compound subtype should remain typed");
        assert!(filter.attacking, "{filter:#?}");
        assert_eq!(filter.subtypes, [Subtype::AssemblyWorker], "{filter:#?}");
    }

    #[test]
    fn shared_untapped_type_union_keeps_controller_scope() {
        let tokens = tokenize_line("untapped artifacts and/or creatures you control", 0);
        let filter =
            parse_object_filter(&tokens, false).expect("shared-state type union should parse");

        assert_eq!(filter.controller, Some(PlayerFilter::You), "{filter:#?}");
        assert_eq!(filter.zone, Some(Zone::Battlefield), "{filter:#?}");
        assert_eq!(
            filter.union_connective(),
            ironsmith_core::ObjectFilterUnionConnective::AndOr
        );
        assert_eq!(filter.any_of.len(), 2, "{filter:#?}");
        assert!(
            filter.any_of.iter().all(|branch| branch.untapped),
            "{filter:#?}"
        );
        assert_eq!(
            filter.description(),
            "an untapped artifact and/or creature you control"
        );
    }

    #[test]
    fn explicit_card_filter_disjunction_preserves_branch_local_predicates() {
        let tokens = tokenize_line(
            "a Doctor card, a card with doctor's companion, or a Vehicle card",
            0,
        );

        let filter = parse_object_filter(&tokens, false)
            .expect("explicit repeated-card filter union should parse");

        assert_eq!(filter.any_of.len(), 3, "{filter:#?}");
        assert_eq!(filter.any_of[0].subtypes, [Subtype::Doctor]);
        assert!(filter.any_of[1].subtypes.is_empty(), "{filter:#?}");
        assert_eq!(filter.any_of[1].zone, None, "{filter:#?}");
        assert_eq!(
            filter.any_of[1].ability_markers,
            ["doctor's companion".to_string()]
        );
        assert_eq!(filter.any_of[2].subtypes, [Subtype::Vehicle]);
        assert_eq!(
            filter.description(),
            "a Doctor card, a card with doctor's companion, or a Vehicle card"
        );
    }

    #[test]
    fn lexed_generic_card_ability_tail_does_not_invent_a_subtype() {
        let tokens = crate::lexer::lex_line("a card with doctor's companion", 0)
            .expect("generic card ability filter should lex");
        let filter =
            parse_object_filter(&tokens, false).expect("generic card ability filter should parse");

        assert!(filter.subtypes.is_empty(), "{filter:#?}");
        assert_eq!(filter.zone, None, "{filter:#?}");
        assert_eq!(filter.ability_markers, ["doctor's companion".to_string()]);
        assert_eq!(filter.description(), "card with doctor's companion");
    }

    #[test]
    fn generic_card_ability_tail_preserves_explicit_graveyard_location() {
        let tokens = crate::lexer::lex_line("cards with cycling from your graveyard", 0)
            .expect("generic card ability filter should lex");

        let filter =
            parse_object_filter(&tokens, false).expect("generic card ability filter should parse");

        assert_eq!(filter.zone, Some(Zone::Graveyard), "{filter:#?}");
        assert_eq!(filter.owner, Some(PlayerFilter::You), "{filter:#?}");
        assert_eq!(filter.ability_markers, ["cycling".to_string()]);
        assert_eq!(filter.description(), "card with cycling in your graveyard");
    }

    #[test]
    fn repeated_card_union_lifts_shared_graveyard_scope_and_keeps_ability_marker() {
        let tokens = crate::lexer::lex_line(
            "Assassin card or card with freerunning from your graveyard",
            0,
        )
        .expect("repeated card union should lex");

        let filter = parse_object_filter(&tokens, false).expect("repeated card union should parse");

        assert_eq!(filter.zone, Some(Zone::Graveyard), "{filter:#?}");
        assert_eq!(filter.owner, Some(PlayerFilter::You), "{filter:#?}");
        assert_eq!(filter.any_of.len(), 2, "{filter:#?}");
        assert!(filter.any_of.iter().all(|arm| arm.zone.is_none()));
        assert!(
            filter
                .any_of
                .iter()
                .any(|arm| { arm.subtypes == [Subtype::Assassin] && arm.has_explicit_card_noun() })
        );
        assert!(filter.any_of.iter().any(|arm| {
            arm.ability_markers == ["freerunning".to_string()] && arm.has_explicit_card_noun()
        }));
    }

    #[test]
    fn ability_filter_preserves_authored_controller_qualifier_order() {
        let permanent_tokens = tokenize_line("permanent with fading you control", 0);
        let permanent = parse_object_filter(&permanent_tokens, false)
            .expect("postpositive permanent ability filter should parse");
        assert!(permanent.has_all_permanent_card_types(), "{permanent:#?}");
        assert_eq!(permanent.ability_markers, ["fading".to_string()]);
        assert!(
            permanent.has_controller_after_qualifiers_surface(),
            "{permanent:#?}"
        );
        let permanent_lexed = parse_object_filter_lexed(&permanent_tokens, false)
            .expect("lexed postpositive permanent ability filter should parse");
        assert!(
            permanent_lexed.has_controller_after_qualifiers_surface(),
            "{permanent_lexed:#?}"
        );
        let permanent_words =
            parse_object_filter_words(&["permanent", "with", "fading", "you", "control"], false)
                .expect("word-based postpositive permanent ability filter should parse");
        assert!(
            permanent_words.has_controller_after_qualifiers_surface(),
            "{permanent_words:#?}"
        );

        let postpositive_creature_tokens = tokenize_line("creature with defender you control", 0);
        let postpositive_creature = parse_object_filter(&postpositive_creature_tokens, false)
            .expect("postpositive creature ability filter should parse");
        assert_eq!(postpositive_creature.card_types, [CardType::Creature]);
        assert!(
            postpositive_creature.has_controller_after_qualifiers_surface(),
            "{postpositive_creature:#?}"
        );

        let canonical_tokens = tokenize_line("creature you control with defender", 0);
        let canonical = parse_object_filter(&canonical_tokens, false)
            .expect("canonical creature ability filter should parse");
        assert_eq!(canonical.card_types, [CardType::Creature]);
        assert!(
            !canonical.has_controller_after_qualifiers_surface(),
            "{canonical:#?}"
        );
    }

    #[test]
    fn numeric_and_chosen_filters_preserve_authored_controller_qualifier_order() {
        let numeric_tokens = tokenize_line("creature with power 4 or greater you control", 0);
        let numeric = parse_object_filter(&numeric_tokens, false)
            .expect("postpositive numeric filter should parse");
        assert_eq!(numeric.power, Some(Comparison::GreaterThanOrEqual(4)));
        assert!(
            numeric.has_controller_after_qualifiers_surface(),
            "{numeric:#?}"
        );

        let chosen_tokens = tokenize_line("permanent of the chosen type you control", 0);
        let chosen = parse_object_filter(&chosen_tokens, false)
            .expect("postpositive chosen-type filter should parse");
        assert!(chosen.chosen_creature_type, "{chosen:#?}");
        assert!(
            chosen.has_controller_after_qualifiers_surface(),
            "{chosen:#?}"
        );

        let canonical_tokens = tokenize_line("creature you control with power 4 or greater", 0);
        let canonical = parse_object_filter(&canonical_tokens, false)
            .expect("canonical numeric filter should parse");
        assert!(
            !canonical.has_controller_after_qualifiers_surface(),
            "{canonical:#?}"
        );

        let that_player_tokens =
            tokenize_line("creature that player controls with power 4 or greater", 0);
        let that_player = parse_object_filter(&that_player_tokens, false)
            .expect("that-player controller filter should parse");
        assert!(
            !that_player.has_controller_after_qualifiers_surface(),
            "{that_player:#?}"
        );
    }

    #[test]
    fn subtype_union_preserves_shared_article_and_serial_or_surface() {
        let tokens = tokenize_line("a Kraken, Leviathan, Merfolk, Octopus, or Serpent", 0);
        let filter =
            parse_object_filter(&tokens, false).expect("serial subtype union should parse");

        assert_eq!(
            filter.subtypes,
            [
                Subtype::Kraken,
                Subtype::Leviathan,
                Subtype::Merfolk,
                Subtype::Octopus,
                Subtype::Serpent,
            ]
        );
        assert!(filter.has_serial_or_list_surface(), "{filter:#?}");
        assert!(
            filter.has_shared_indefinite_article_surface(),
            "{filter:#?}"
        );
        assert_eq!(
            filter.description(),
            "a Kraken, Leviathan, Merfolk, Octopus, or Serpent"
        );

        let excluded_tokens = tokenize_line(
            "creature that isn't a Kraken, Leviathan, Merfolk, Octopus, or Serpent",
            0,
        );
        let excluded = parse_object_filter(&excluded_tokens, false)
            .expect("negative serial subtype union should parse");
        assert!(excluded.has_serial_or_list_surface(), "{excluded:#?}");
        assert_eq!(excluded.card_types, [CardType::Creature]);
        assert!(excluded.subtypes.is_empty(), "{excluded:#?}");
        assert_eq!(
            excluded.excluded_subtypes,
            [
                Subtype::Kraken,
                Subtype::Leviathan,
                Subtype::Merfolk,
                Subtype::Octopus,
                Subtype::Serpent,
            ],
        );
        assert!(excluded.any_of.is_empty(), "{excluded:#?}");
        assert_eq!(
            ironsmith_core::filter_model::describe_relative_characteristic_list_filter(&excluded)
                .as_deref(),
            Some("creature that isn't a Kraken, Leviathan, Merfolk, Octopus, or Serpent")
        );

        let each_tokens = tokenize_line(
            "each creature that isn't an Insect, Rat, Spider, or Squirrel",
            0,
        );
        let each = parse_object_filter(&each_tokens, false)
            .expect("quantified negative subtype list should parse");
        assert_eq!(each.card_types, [CardType::Creature]);
        assert_eq!(
            each.excluded_subtypes,
            [
                Subtype::Insect,
                Subtype::Rat,
                Subtype::Spider,
                Subtype::Squirrel,
            ],
        );
        assert!(
            each.subtypes.is_empty() && each.any_of.is_empty(),
            "{each:#?}"
        );
        assert_eq!(
            ironsmith_core::filter_model::describe_relative_characteristic_list_filter(&each)
                .as_deref(),
            Some("creature that isn't an Insect, Rat, Spider, or Squirrel"),
        );
    }

    #[test]
    fn general_filter_entrypoint_keeps_branch_scoped_comma_collection() {
        let tokens = tokenize_line(
            "enchantments you both own and control, all Auras you own attached to permanents you control, and all Auras you own attached to attacking creatures your opponents control",
            0,
        );
        let filter =
            parse_object_filter(&tokens, false).expect("branch-scoped collection should parse");

        assert_eq!(filter.any_of.len(), 3, "{filter:#?}");
        assert!(filter.has_conjunctive_set_surface());
        assert_eq!(filter.owner, Some(PlayerFilter::You));
        assert!(filter.any_of.iter().any(|branch| {
            branch.subtypes == [Subtype::Aura]
                && branch.attached_to_object.as_deref().is_some_and(|host| {
                    host.attacking && host.controller == Some(PlayerFilter::Opponent)
                })
        }));
    }

    #[test]
    fn repeated_or_subtype_union_does_not_infer_serial_comma_surface() {
        let tokens = tokenize_line("a Kraken or Leviathan or Merfolk or Octopus or Serpent", 0);
        let filter =
            parse_object_filter(&tokens, false).expect("repeated-or subtype union should parse");

        assert!(!filter.has_serial_or_list_surface(), "{filter:#?}");
        assert!(
            filter.has_shared_indefinite_article_surface(),
            "{filter:#?}"
        );
        assert_eq!(
            filter.description(),
            "a Kraken or Leviathan or Merfolk or Octopus or Serpent"
        );
    }

    #[test]
    fn parsed_card_noun_remains_visible_after_context_clears_the_zone() {
        let tokens = tokenize_line("instant or sorcery card", 0);
        let mut filter =
            parse_object_filter_lexed(&tokens, false).expect("typed card filter should parse");

        filter.zone = None;
        assert!(filter.has_explicit_card_noun());
        assert_eq!(filter.description(), "instant or sorcery card");
    }

    #[test]
    fn shared_card_noun_preserves_plain_and_as_an_inclusive_set_surface() {
        let tokens = tokenize_line("instant and sorcery card in your graveyard", 0);
        let filter =
            parse_object_filter(&tokens, false).expect("conjunctive card filter should parse");

        assert!(filter.any_of.is_empty(), "{filter:#?}");
        assert_eq!(
            filter.card_types,
            [CardType::Instant, CardType::Sorcery],
            "{filter:#?}"
        );
        assert!(filter.has_conjunctive_set_surface(), "{filter:#?}");
        assert_eq!(
            filter.description(),
            "instant and sorcery card in your graveyard"
        );

        let disjunctive = tokenize_line("instant or sorcery card in your graveyard", 0);
        let disjunctive =
            parse_object_filter(&disjunctive, false).expect("disjunctive filter should parse");
        assert!(
            !disjunctive.has_conjunctive_set_surface(),
            "{disjunctive:#?}"
        );
        assert_eq!(
            disjunctive.description(),
            "instant or sorcery card in your graveyard"
        );

        let compound_type = tokenize_line("artifact creature with flying and vigilance", 0);
        let compound_type =
            parse_object_filter(&compound_type, false).expect("compound type filter should parse");
        assert!(
            !compound_type.has_conjunctive_set_surface(),
            "an `and` in a later qualifier must not join adjacent card types: {compound_type:#?}"
        );
        assert!(
            !compound_type
                .description()
                .starts_with("artifact and creature"),
            "{}",
            compound_type.description()
        );
    }

    #[test]
    fn parse_object_filter_preserves_original_printing_set_qualifier() {
        let tokens = tokenize_line(
            "nontoken permanent with a name originally printed in the Antiquities expansion",
            0,
        );

        let filter = parse_object_filter_lexed(&tokens, false)
            .expect("historical printing qualifier should parse");

        assert!(filter.nontoken);
        assert_eq!(filter.zone, Some(Zone::Battlefield));
        assert_eq!(
            filter.name_originally_printed_in_set.as_deref(),
            Some("Antiquities")
        );
        assert_eq!(
            filter.description(),
            "nontoken permanent with a name originally printed in the Antiquities expansion"
        );
    }

    #[test]
    fn parse_object_filter_preserves_named_draft_color_history() {
        let tokens = crate::lexer::lex_line(
            "creature that's one or more of the colors chosen as you drafted cards named Draft Test",
            0,
        )
        .expect("draft-color filter should lex losslessly");
        let filter =
            parse_object_filter_lexed(&tokens, false).expect("draft-color qualifier should parse");

        assert_eq!(filter.card_types, [CardType::Creature]);
        assert!(filter.name.is_none(), "{filter:#?}");
        assert_eq!(
            filter.colors_chosen_while_drafting_named.as_deref(),
            Some("Draft Test")
        );
        assert_eq!(
            filter.description(),
            "creature that's one or more of the colors chosen as you drafted cards named Draft Test"
        );
    }

    #[test]
    fn parse_object_filter_preserves_plural_counter_surface_semantically() {
        let plural_tokens = tokenize_line("creatures you control with +1/+1 counters on them", 0);
        let plural = parse_object_filter_lexed(&plural_tokens, false)
            .expect("plural counter filter should parse");
        assert_eq!(
            plural.with_counter,
            Some(crate::filter::CounterConstraint::Typed(
                crate::object::CounterType::PlusOnePlusOne,
            ))
        );
        assert!(
            plural
                .description()
                .ends_with("with +1/+1 counters on them"),
            "{}",
            plural.description()
        );

        let singular_tokens = tokenize_line("a creature you control with a +1/+1 counter on it", 0);
        let singular = parse_object_filter_lexed(&singular_tokens, false)
            .expect("singular counter filter should parse");
        assert!(
            singular
                .description()
                .ends_with("with a +1/+1 counter on it"),
            "{}",
            singular.description()
        );
        assert_eq!(plural, singular);
    }

    #[test]
    fn parse_attached_reference_or_another_disjunction_handles_articles_without_word_view() {
        let tokens = tokenize_line("enchanted creature or another creature", 0);

        let filter = parse_attached_reference_or_another_disjunction(&tokens)
            .expect("attached-reference disjunction should parse")
            .expect("attached-reference disjunction should be recognized");

        assert_eq!(filter.any_of.len(), 2);
        assert!(
            filter.any_of[0]
                .tagged_constraints
                .iter()
                .any(|constraint| {
                    constraint.tag.as_str() == crate::tag::CompilerReferenceTag::Enchanted.as_str()
                        && constraint.relation == TaggedOpbjectRelation::IsTaggedObject
                }),
            "{filter:?}"
        );
        assert_eq!(filter.any_of[0].card_types, vec![CardType::Creature]);
        assert_eq!(filter.any_of[1].card_types, vec![CardType::Creature]);
        assert!(filter.any_of[1].other);
    }

    #[test]
    fn parse_object_filter_lexed_parses_suffix_owned_zone() {
        let tokens = tokenize_line("artifact card from your graveyard", 0);

        let filter = parse_object_filter_lexed(&tokens, false).expect("object filter should parse");

        assert_eq!(filter.owner, Some(PlayerFilter::You));
        assert_eq!(filter.zone, Some(Zone::Graveyard));
        assert_eq!(filter.card_types, vec![CardType::Artifact]);
    }

    #[test]
    fn spell_filter_keeps_shared_type_comparison_domains_nested() {
        let tokens = tokenize_line(
            "creature spell that doesn't share a creature type with a creature you control or a creature card in your graveyard",
            0,
        );

        let filter = parse_object_filter_lexed(&tokens, false)
            .expect("shared-creature-type spell filter should parse");

        assert_eq!(filter.zone, Some(Zone::Stack), "{filter:#?}");
        assert_eq!(
            filter.stack_kind,
            Some(crate::filter::StackObjectKind::Spell),
            "{filter:#?}"
        );
        assert_eq!(filter.card_types, vec![CardType::Creature]);
        assert_eq!(filter.no_shared_creature_types_with.len(), 2);
        assert_eq!(
            filter.no_shared_creature_types_with[0].zone,
            Some(Zone::Battlefield)
        );
        assert_eq!(
            filter.no_shared_creature_types_with[1].zone,
            Some(Zone::Graveyard)
        );
        assert_eq!(
            filter.owner, None,
            "the comparison owner is not the spell owner"
        );
        assert!(
            filter
                .description()
                .contains("doesn't share a creature type with"),
            "{}",
            filter.description()
        );
    }

    #[test]
    fn mana_value_comparison_rhs_does_not_leak_source_type_into_filter_union() {
        let tokens = tokenize_line(
            "instant or sorcery card with mana value less than or equal to this creature's power from your graveyard",
            0,
        );

        let filter = parse_object_filter_lexed(&tokens, false).expect("object filter should parse");

        assert_eq!(
            filter.card_types,
            vec![CardType::Instant, CardType::Sorcery]
        );
        assert!(!filter.card_types.contains(&CardType::Creature));
        assert_eq!(filter.owner, Some(PlayerFilter::You));
        assert_eq!(filter.zone, Some(Zone::Graveyard));
        assert!(
            matches!(
                filter.mana_value.as_ref(),
                Some(crate::filter::Comparison::LessThanOrEqualExpr(value))
                    if matches!(
                        value.unhinted(),
                        crate::effect::Value::PowerOf(spec)
                            if matches!(spec.unhinted(), crate::target::ChooseSpec::Source)
                    )
            ),
            "{filter:#?}"
        );
    }

    #[test]
    fn shared_terminal_card_noun_lifts_dynamic_mana_value_over_type_union() {
        let tokens = tokenize_line(
            "instant or sorcery card with mana value less than or equal to his power from your graveyard",
            0,
        );

        let filter = parse_object_filter_lexed(&tokens, false).expect("object filter should parse");

        assert!(filter.any_of.is_empty(), "{filter:#?}");
        assert_eq!(
            filter.card_types,
            vec![CardType::Instant, CardType::Sorcery]
        );
        assert_eq!(filter.owner, Some(PlayerFilter::You));
        assert_eq!(filter.zone, Some(Zone::Graveyard));
        assert!(matches!(
            filter.mana_value.as_ref(),
            Some(crate::filter::Comparison::LessThanOrEqualExpr(value))
                if value.unhinted() == &crate::effect::Value::SourcePower
                    && value.has_surface_hint(
                        ironsmith_core::ValueSurfaceHint::MasculineSourcePossessive
                    )
        ));
    }

    #[test]
    fn shared_terminal_card_noun_scopes_graveyard_over_and_or_type_arms() {
        let tokens = crate::lexer::lex_line("artifact and/or creature card in your graveyard", 0)
            .expect("shared-domain filter should lex");
        assert!(
            has_shared_terminal_object_noun(&tokens),
            "the one terminal card noun must scope both selector arms"
        );

        let filter = parse_object_filter(&tokens, false).expect("shared card domain should parse");

        assert!(filter.any_of.is_empty(), "{filter:#?}");
        assert_eq!(
            filter.card_types,
            [CardType::Artifact, CardType::Creature],
            "{filter:#?}"
        );
        assert_eq!(filter.owner, Some(PlayerFilter::You), "{filter:#?}");
        assert_eq!(filter.zone, Some(Zone::Graveyard), "{filter:#?}");
        assert_eq!(
            filter.union_connective(),
            ObjectFilterUnionConnective::AndOr
        );
        assert_eq!(
            filter.description(),
            "artifact and/or creature card in your graveyard"
        );
    }

    #[test]
    fn independently_nouned_and_or_arms_keep_their_own_domains() {
        let tokens = crate::lexer::lex_line(
            "artifacts you control and/or creature cards in your graveyard",
            0,
        )
        .expect("independently scoped filter should lex");
        assert!(
            !has_shared_terminal_object_noun(&tokens),
            "independently nouned domains must not inherit a shared suffix"
        );

        let filter =
            parse_object_filter(&tokens, false).expect("independently scoped domains should parse");

        assert_eq!(filter.any_of.len(), 2, "{filter:#?}");
        assert!(filter.any_of.iter().any(|branch| {
            branch.card_types == [CardType::Artifact]
                && branch.controller == Some(PlayerFilter::You)
                && branch.zone == Some(Zone::Battlefield)
                && branch.owner.is_none()
        }));
        assert!(filter.any_of.iter().any(|branch| {
            branch.card_types == [CardType::Creature]
                && branch.owner == Some(PlayerFilter::You)
                && branch.zone == Some(Zone::Graveyard)
                && branch.controller.is_none()
        }));
    }

    #[test]
    fn shared_terminal_card_noun_keeps_branch_exclusion_and_common_graveyard_domain() {
        for (text, expected_types, has_mana_value) in [
            (
                "artifact or non-Aura enchantment card from your graveyard",
                vec![CardType::Artifact, CardType::Enchantment],
                false,
            ),
            (
                "artifact, creature, or non-Aura enchantment card with mana value 3 or less from your graveyard",
                vec![
                    CardType::Artifact,
                    CardType::Creature,
                    CardType::Enchantment,
                ],
                true,
            ),
        ] {
            let tokens = tokenize_line(text, 0);
            assert!(
                has_shared_terminal_object_noun(&tokens),
                "the terminal card noun must scope every union arm: {text}"
            );
            let filter =
                parse_object_filter_lexed(&tokens, false).expect("object filter should parse");

            assert_eq!(filter.owner, Some(PlayerFilter::You), "{filter:#?}");
            assert_eq!(filter.zone, Some(Zone::Graveyard), "{filter:#?}");
            assert_eq!(filter.mana_value.is_some(), has_mana_value, "{filter:#?}");
            assert!(filter.card_types.is_empty(), "{filter:#?}");
            assert!(filter.excluded_subtypes.is_empty(), "{filter:#?}");
            assert_eq!(filter.any_of.len(), expected_types.len(), "{filter:#?}");
            for card_type in expected_types {
                let branch = filter
                    .any_of
                    .iter()
                    .find(|branch| branch.card_types == [card_type])
                    .unwrap_or_else(|| panic!("missing {card_type:?} arm: {filter:#?}"));
                if card_type == CardType::Enchantment {
                    assert_eq!(branch.excluded_subtypes, [Subtype::Aura], "{filter:#?}");
                } else {
                    assert!(branch.excluded_subtypes.is_empty(), "{filter:#?}");
                }
            }
        }
    }

    #[test]
    fn repeated_card_nouns_keep_trailing_mana_value_branch_local() {
        let tokens = tokenize_line(
            "land card or creature card with mana value less than or equal to his power from your graveyard",
            0,
        );

        let filter = parse_object_filter_lexed(&tokens, false).expect("object filter should parse");

        assert_eq!(filter.any_of.len(), 2, "{filter:#?}");
        let land = filter
            .any_of
            .iter()
            .find(|branch| branch.card_types == [CardType::Land])
            .expect("land arm");
        let creature = filter
            .any_of
            .iter()
            .find(|branch| branch.card_types == [CardType::Creature])
            .expect("creature arm");
        assert!(land.mana_value.is_none(), "{land:#?}");
        assert!(matches!(
            creature.mana_value.as_ref(),
            Some(crate::filter::Comparison::LessThanOrEqualExpr(value))
                if value.unhinted() == &crate::effect::Value::SourcePower
        ));
    }

    #[test]
    fn parse_object_filter_lexed_parses_controller_without_owner_suffix() {
        let tokens = tokenize_line("land you control but don't own", 0);

        let filter = parse_object_filter_lexed(&tokens, false).expect("object filter should parse");

        assert_eq!(filter.controller, Some(PlayerFilter::You));
        assert_eq!(filter.owner, Some(PlayerFilter::NotYou));
        assert_eq!(filter.zone, Some(Zone::Battlefield));
        assert_eq!(filter.card_types, vec![CardType::Land]);
    }

    #[test]
    fn parse_object_filter_lexed_distinguishes_basic_land_type_from_basic_supertype() {
        for text in [
            "land card with a basic land type",
            "land cards that each have a basic land type",
        ] {
            let tokens = tokenize_line(text, 0);
            let filter = parse_object_filter_lexed(&tokens, false).expect("basic-land-type filter");

            assert_eq!(filter.card_types, vec![CardType::Land], "{filter:#?}");
            assert!(filter.has_basic_land_type, "{filter:#?}");
            assert!(filter.supertypes.is_empty(), "{filter:#?}");
            assert_eq!(filter.description(), "land card with a basic land type");
        }
    }

    #[test]
    fn parse_object_filter_lexed_treats_adjacent_card_types_as_conjunctive() {
        let tokens = tokenize_line("artifact creature", 0);

        let filter = parse_object_filter_lexed(&tokens, false).expect("object filter should parse");

        assert!(filter.card_types.is_empty(), "{filter:#?}");
        assert_eq!(
            filter.all_card_types,
            vec![CardType::Artifact, CardType::Creature]
        );
        assert_eq!(filter.zone, Some(Zone::Battlefield));
    }

    #[test]
    fn parse_object_filter_lexed_preserves_adjacent_compound_subtypes() {
        let tokens = crate::lexer::lex_line("Eldrazi Spawn creatures you control", 0)
            .expect("compound subtype fixture should lex");
        let filter =
            parse_object_filter_lexed(&tokens, false).expect("compound subtype should parse");

        assert!(filter.subtypes.is_empty(), "{filter:#?}");
        assert_eq!(filter.all_subtypes, vec![Subtype::Eldrazi, Subtype::Spawn]);
    }

    #[test]
    fn parse_object_filter_lexed_keeps_explicit_type_lists_disjunctive() {
        let tokens = tokenize_line("artifact, creature, or land", 0);

        let filter = parse_object_filter_lexed(&tokens, false).expect("object filter should parse");

        assert_eq!(
            filter.card_types,
            vec![CardType::Artifact, CardType::Creature, CardType::Land]
        );
        assert!(filter.all_card_types.is_empty(), "{filter:#?}");
        assert_eq!(filter.zone, Some(Zone::Battlefield));
    }

    #[test]
    fn parse_object_filter_lexed_keeps_comma_only_type_lists_disjunctive() {
        let tokens = tokenize_line("artifact, creature, enchantment", 0);

        let filter = parse_object_filter_lexed(&tokens, false).expect("object filter should parse");

        assert_eq!(
            filter.card_types,
            vec![
                CardType::Artifact,
                CardType::Creature,
                CardType::Enchantment
            ]
        );
        assert!(filter.all_card_types.is_empty(), "{filter:#?}");
        assert_eq!(filter.zone, Some(Zone::Battlefield));
    }

    #[test]
    fn parse_object_filter_words_parses_target_and_iterated_controller_suffixes() {
        let target_filter =
            parse_simple_object_filter_words(&["artifact", "target", "player", "controls"], false)
                .expect("target-player controller suffix should parse");
        assert_eq!(
            target_filter.controller,
            Some(PlayerFilter::target_player())
        );
        assert_eq!(target_filter.zone, Some(Zone::Battlefield));
        assert_eq!(target_filter.card_types, vec![CardType::Artifact]);

        let iterated_filter =
            parse_simple_object_filter_words(&["creature", "that", "player", "controls"], false)
                .expect("that-player controller suffix should parse");
        assert_eq!(
            iterated_filter.controller,
            Some(PlayerFilter::IteratedPlayer)
        );
        assert_eq!(iterated_filter.zone, Some(Zone::Battlefield));
        assert_eq!(iterated_filter.card_types, vec![CardType::Creature]);
    }

    #[test]
    fn parse_object_filter_lexed_parses_controlled_spells_on_stack() {
        let tokens = tokenize_line("spells you control", 0);

        let filter = parse_object_filter_lexed(&tokens, false).expect("object filter should parse");

        assert_eq!(filter.controller, Some(PlayerFilter::You));
        assert_eq!(filter.zone, Some(Zone::Stack));
        assert_eq!(
            filter.stack_kind,
            Some(crate::filter::StackObjectKind::Spell)
        );
        assert!(filter.has_mana_cost);
    }

    #[test]
    fn phyrexian_mana_placeholder_becomes_an_executable_mana_cost_predicate() {
        let tokens = tokenize_line("spell with {H} in its mana cost", 0);
        let filter = parse_object_filter(&tokens, false)
            .expect("the Phyrexian-mana spell filter should parse");

        assert_eq!(filter.zone, Some(Zone::Stack), "{filter:#?}");
        assert_eq!(
            filter.stack_kind,
            Some(crate::filter::StackObjectKind::Spell),
            "{filter:#?}"
        );
        assert!(filter.has_mana_cost, "{filter:#?}");
        assert!(filter.has_phyrexian_mana_symbol, "{filter:#?}");
        assert_eq!(filter.description(), "spell with {H} in its mana cost");

        let lexed = parse_object_filter_lexed(&tokens, false)
            .expect("the trigger-facing lexed parser should retain the predicate");
        assert!(lexed.has_phyrexian_mana_symbol, "{lexed:#?}");

        let ordinary = parse_object_filter(&tokenize_line("spell with a mana cost", 0), false)
            .expect("ordinary mana-cost filter should parse");
        assert!(!ordinary.has_phyrexian_mana_symbol, "{ordinary:#?}");
    }

    #[test]
    fn parse_object_filter_preserves_excluded_cast_origin_zone() {
        let tokens = tokenize_line("spell that wasn't cast from its owner's hand", 0);

        let filter = parse_object_filter(&tokens, false)
            .expect("negative spell cast-origin filter should parse");

        assert_eq!(filter.zone, Some(Zone::Stack), "{filter:#?}");
        assert_eq!(
            filter.stack_kind,
            Some(crate::filter::StackObjectKind::Spell),
            "{filter:#?}"
        );
        assert_eq!(
            filter.excluded_cast_origin_zone,
            Some(Zone::Hand),
            "{filter:#?}"
        );
        assert_eq!(
            filter.description(),
            "spell that wasn't cast from its owner's hand"
        );
    }

    #[test]
    fn parse_object_filter_preserves_positive_cast_origin_zone() {
        let tokens = tokenize_line("spells you cast from exile", 0);

        let filter =
            parse_object_filter(&tokens, false).expect("positive cast-origin filter should parse");

        assert_eq!(filter.zone, Some(Zone::Exile), "{filter:#?}");
        assert_eq!(filter.cast_by, Some(PlayerFilter::You), "{filter:#?}");
        assert!(filter.has_mana_cost, "{filter:#?}");

        let qualified = tokenize_line(
            "colorless spells you cast from your hand with mana value 7 or greater",
            0,
        );
        let qualified = parse_object_filter(&qualified, false)
            .expect("trailing spell qualifiers must not hide the cast origin");
        assert_eq!(qualified.zone, Some(Zone::Hand), "{qualified:#?}");
        assert_eq!(qualified.cast_by, Some(PlayerFilter::You), "{qualified:#?}");
        assert_eq!(
            qualified.mana_value,
            Some(Comparison::GreaterThanOrEqual(7)),
            "{qualified:#?}"
        );
    }

    #[test]
    fn explicit_stack_spell_preserves_positive_cast_origin_without_cast_by() {
        let tokens = tokenize_line("spell cast from a graveyard", 0);

        let filter = parse_object_filter_lexed(&tokens, false)
            .expect("positive cast-origin stack-spell filter should parse");

        assert_eq!(filter.zone, Some(Zone::Graveyard), "{filter:#?}");
        assert_eq!(filter.cast_by, None, "{filter:#?}");
        assert_eq!(
            filter.stack_kind,
            Some(crate::filter::StackObjectKind::Spell),
            "{filter:#?}"
        );
        assert!(filter.has_mana_cost, "{filter:#?}");
    }

    #[test]
    fn shared_terminal_spell_noun_scopes_over_type_union_and_qualifiers() {
        let tokens = tokenize_line("instant or sorcery spell you control with mana value X", 0);

        let filter =
            parse_object_filter(&tokens, false).expect("shared-noun spell filter should parse");

        assert!(filter.any_of.is_empty(), "{filter:#?}");
        assert_eq!(
            filter.card_types,
            [CardType::Instant, CardType::Sorcery],
            "{filter:#?}"
        );
        assert_eq!(filter.zone, Some(Zone::Stack), "{filter:#?}");
        assert_eq!(filter.controller, Some(PlayerFilter::You), "{filter:#?}");
        assert_eq!(
            filter.stack_kind,
            Some(crate::filter::StackObjectKind::Spell),
            "{filter:#?}"
        );
        assert!(matches!(
            filter.mana_value.as_ref(),
            Some(Comparison::EqualExpr(value))
                if value.unhinted() == &crate::effect::Value::X
        ));
        assert_eq!(
            filter.description(),
            "an instant or sorcery spell you control with mana value X"
        );
    }

    #[test]
    fn parse_object_filter_lexed_parses_split_face_state_and_chosen_type_atoms() {
        let tokens = tokenize_line("face down chosen type creatures", 0);

        let filter = parse_object_filter_lexed(&tokens, false).expect("object filter should parse");

        assert_eq!(filter.face_down, Some(true));
        assert!(filter.chosen_creature_type);
        assert_eq!(filter.card_types, vec![CardType::Creature]);
        assert_eq!(filter.zone, Some(Zone::Battlefield));
    }

    #[test]
    fn parse_object_filter_lexed_parses_hyphenated_face_state_and_nonchosen_type_atoms() {
        let tokens = tokenize_line("face-up nonchosen type creatures", 0);

        let filter = parse_object_filter_lexed(&tokens, false).expect("object filter should parse");

        assert_eq!(filter.face_down, Some(false));
        assert!(filter.excluded_chosen_creature_type);
        assert_eq!(filter.card_types, vec![CardType::Creature]);
        assert_eq!(filter.zone, Some(Zone::Battlefield));
    }

    #[test]
    fn parse_object_filter_lexed_parses_negated_chosen_type_suffix() {
        let tokens = tokenize_line("creatures that aren't of the chosen type", 0);

        let filter = parse_object_filter_lexed(&tokens, false).expect("object filter should parse");

        assert_eq!(filter.card_types, vec![CardType::Creature]);
        assert!(!filter.chosen_creature_type);
        assert!(filter.excluded_chosen_creature_type);
        assert_eq!(filter.zone, Some(Zone::Battlefield));
    }

    #[test]
    fn parse_object_filter_lexed_preserves_all_types_chosen_this_way() {
        let tokens = tokenize_line("creatures that aren't of a type chosen this way", 0);

        let filter = parse_object_filter_lexed(&tokens, false).expect("object filter should parse");

        assert_eq!(filter.card_types, vec![CardType::Creature]);
        assert!(!filter.excluded_chosen_creature_type);
        assert!(filter.excluded_any_chosen_creature_type, "{filter:#?}");
        assert!(filter.has_chosen_type_this_way_surface(), "{filter:#?}");
        assert_eq!(
            filter.description(),
            "creature that aren't of a type chosen this way"
        );
    }

    #[test]
    fn relative_unions_keep_the_common_domain_outside_the_union_arms() {
        let mycotyrant = tokenize_line("creatures you control that are Fungi and/or Saprolings", 0);
        let mycotyrant =
            parse_object_filter_lexed(&mycotyrant, false).expect("relative subtype union");
        assert_eq!(mycotyrant.card_types, [CardType::Creature]);
        assert_eq!(mycotyrant.subtypes, [Subtype::Fungus, Subtype::Saproling]);
        assert!(!mycotyrant.type_or_subtype_union, "{mycotyrant:#?}");
        assert!(mycotyrant.any_of.is_empty(), "{mycotyrant:#?}");
        assert!(mycotyrant.has_relative_characteristic_list_surface());
        assert_eq!(
            mycotyrant.union_connective(),
            ObjectFilterUnionConnective::AndOr
        );

        let zombies_or_tokens =
            tokenize_line("creatures you control that are Zombies and/or tokens", 0);
        let zombies_or_tokens =
            parse_object_filter_lexed(&zombies_or_tokens, false).expect("relative mixed union");
        assert_eq!(zombies_or_tokens.card_types, [CardType::Creature]);
        assert_eq!(zombies_or_tokens.controller, Some(PlayerFilter::You));
        assert!(!zombies_or_tokens.token, "{zombies_or_tokens:#?}");
        assert!(
            zombies_or_tokens.subtypes.is_empty(),
            "{zombies_or_tokens:#?}"
        );
        assert_eq!(zombies_or_tokens.any_of.len(), 2, "{zombies_or_tokens:#?}");
        assert!(
            zombies_or_tokens
                .any_of
                .iter()
                .any(|arm| arm.subtypes == [Subtype::Zombie]),
            "{zombies_or_tokens:#?}"
        );
        assert!(
            zombies_or_tokens.any_of.iter().any(|arm| arm.token),
            "{zombies_or_tokens:#?}"
        );
        assert!(zombies_or_tokens.has_relative_characteristic_list_surface());

        let token_or_rabbit =
            tokenize_line("other creature you control that's a token or a Rabbit", 0);
        let token_or_rabbit =
            parse_object_filter_lexed(&token_or_rabbit, false).expect("relative mixed union");
        assert_eq!(token_or_rabbit.card_types, [CardType::Creature]);
        assert_eq!(token_or_rabbit.controller, Some(PlayerFilter::You));
        assert!(token_or_rabbit.other);
        assert_eq!(token_or_rabbit.any_of.len(), 2, "{token_or_rabbit:#?}");
        assert!(
            token_or_rabbit.any_of.iter().any(|arm| arm.token),
            "{token_or_rabbit:#?}"
        );
        assert!(
            token_or_rabbit
                .any_of
                .iter()
                .any(|arm| arm.subtypes == [Subtype::Rabbit]),
            "{token_or_rabbit:#?}"
        );
        assert!(token_or_rabbit.has_relative_characteristic_list_surface());
        assert_eq!(
            token_or_rabbit.description(),
            "another creature you control that's a token or a Rabbit"
        );

        let spirits_or_enchantments = tokenize_line(
            "permanents you control that are Spirits and/or enchantments",
            0,
        );
        let spirits_or_enchantments = parse_object_filter_lexed(&spirits_or_enchantments, false)
            .expect("relative type-or-subtype union");
        assert!(spirits_or_enchantments.card_types.is_empty());
        assert!(spirits_or_enchantments.subtypes.is_empty());
        assert_eq!(spirits_or_enchantments.any_of.len(), 2);
        assert!(
            spirits_or_enchantments
                .any_of
                .iter()
                .any(|arm| arm.subtypes == [Subtype::Spirit])
        );
        assert!(
            spirits_or_enchantments
                .any_of
                .iter()
                .any(|arm| arm.card_types == [CardType::Enchantment])
        );
        assert!(spirits_or_enchantments.has_relative_characteristic_list_surface());
    }

    #[test]
    fn comparison_qualified_union_keeps_the_comparison_on_its_authored_arm() {
        let tokens = tokenize_line("creatures with power 2 or less and/or Walls", 0);
        let filter = parse_object_filter_lexed(&tokens, false)
            .expect("comparison-qualified characteristic union");

        assert_eq!(filter.zone, Some(Zone::Battlefield));
        assert!(filter.card_types.is_empty(), "{filter:#?}");
        assert!(filter.subtypes.is_empty(), "{filter:#?}");
        assert!(filter.power.is_none(), "{filter:#?}");
        assert_eq!(filter.any_of.len(), 2, "{filter:#?}");

        let creature_arm = filter
            .any_of
            .iter()
            .find(|arm| arm.card_types == [CardType::Creature])
            .expect("creature arm");
        assert_eq!(creature_arm.power, Some(Comparison::LessThanOrEqual(2)));
        let wall_arm = filter
            .any_of
            .iter()
            .find(|arm| arm.subtypes == [Subtype::Wall])
            .expect("Wall arm");
        assert!(wall_arm.power.is_none());
        assert_eq!(
            filter.description(),
            "creature with power 2 or less and/or Wall"
        );
    }

    #[test]
    fn subtype_or_colored_permanent_keeps_independent_union_arms() {
        for (text, subtype, color) in [
            (
                "an Island or blue permanent",
                Subtype::Island,
                ColorSet::BLUE,
            ),
            (
                "a Swamp or black permanent",
                Subtype::Swamp,
                ColorSet::BLACK,
            ),
        ] {
            let tokens = tokenize_line(text, 0);
            let filter = parse_object_filter_lexed(&tokens, false)
                .expect("mixed permanent selector should parse");

            assert_eq!(filter.any_of.len(), 2, "{text}: {filter:#?}");
            assert!(
                filter.any_of.iter().any(|arm| arm.subtypes == [subtype]
                    && arm.colors.is_none()
                    && arm.card_types.is_empty()),
                "{text}: {filter:#?}"
            );
            assert!(
                filter.any_of.iter().any(|arm| arm.colors == Some(color)
                    && !arm.card_types.is_empty()
                    && arm.subtypes.is_empty()),
                "{text}: {filter:#?}"
            );
        }
    }

    #[test]
    fn ordinary_shared_terminal_type_union_is_not_claimed_as_mixed_permanents() {
        let tokens = tokenize_line("an artifact or creature permanent", 0);
        let filter = parse_object_filter_lexed(&tokens, false)
            .expect("shared-terminal type union should parse");

        assert!(filter.any_of.is_empty(), "{filter:#?}");
        assert!(filter.card_types.contains(&CardType::Artifact));
        assert!(filter.card_types.contains(&CardType::Creature));
    }

    #[test]
    fn where_x_comparison_clause_is_not_an_object_filter_union_arm() {
        let tokens = tokenize_line(
            "creature with toughness X or less, where X is the number of Shrines you control",
            0,
        );
        let filter = parse_object_filter_lexed(&tokens, false)
            .expect("dynamic toughness filter should parse");

        assert_eq!(filter.card_types, [CardType::Creature], "{filter:#?}");
        assert!(filter.controller.is_none(), "{filter:#?}");
        assert!(filter.any_of.is_empty(), "{filter:#?}");
        let Some(crate::filter::Comparison::LessThanOrEqualExpr(value)) = filter.toughness.as_ref()
        else {
            panic!("expected dynamic toughness comparison: {filter:#?}");
        };
        assert_eq!(value.unhinted(), &crate::effect::Value::X, "{filter:#?}");
    }

    #[test]
    fn parse_object_filter_lexed_parses_split_hyphenated_non_subtype_and_type() {
        let tokens = tokenize_line("Non-Elf creatures", 0);
        let filter = parse_object_filter_lexed(&tokens, false).expect("object filter should parse");

        assert_eq!(filter.card_types, vec![CardType::Creature]);
        assert_eq!(filter.excluded_subtypes, vec![Subtype::Elf]);
        assert_eq!(filter.zone, Some(Zone::Battlefield));

        let tokens = tokenize_line("non-artifact creatures", 0);
        let filter = parse_object_filter_lexed(&tokens, false).expect("object filter should parse");

        assert_eq!(filter.card_types, vec![CardType::Creature]);
        assert_eq!(filter.excluded_card_types, vec![CardType::Artifact]);
        assert_eq!(filter.zone, Some(Zone::Battlefield));
    }

    #[test]
    fn parse_object_filter_lexed_preserves_chosen_color_and_type_qualifiers() {
        let color_tokens = tokenize_line("creatures you control of the chosen color", 0);
        let color_filter =
            parse_object_filter_lexed(&color_tokens, false).expect("object filter should parse");

        assert!(color_filter.chosen_color);
        assert_eq!(color_filter.controller, Some(PlayerFilter::You));
        assert_eq!(color_filter.card_types, vec![CardType::Creature]);

        let demonstrative_color_tokens = tokenize_line("creatures of that color", 0);
        let demonstrative_color_filter =
            parse_object_filter_lexed(&demonstrative_color_tokens, false)
                .expect("demonstrative chosen-color filter should parse");
        assert!(demonstrative_color_filter.chosen_color);
        assert_eq!(
            demonstrative_color_filter.card_types,
            vec![CardType::Creature]
        );

        let type_tokens = tokenize_line("other creatures you control of the chosen type", 0);
        let type_filter =
            parse_object_filter_lexed(&type_tokens, false).expect("object filter should parse");

        assert!(type_filter.other);
        assert!(type_filter.chosen_creature_type);
        assert_eq!(type_filter.controller, Some(PlayerFilter::You));
        assert_eq!(type_filter.card_types, vec![CardType::Creature]);

        let that_type_tokens = tokenize_line("cards of that type from their graveyard", 0);
        let that_type_filter = parse_object_filter_lexed(&that_type_tokens, false)
            .expect("that-type graveyard filter should parse");

        assert!(that_type_filter.chosen_creature_type);
        assert_eq!(that_type_filter.zone, Some(Zone::Graveyard));
        assert_eq!(that_type_filter.owner, Some(PlayerFilter::IteratedPlayer));
    }

    #[test]
    fn parse_object_filter_lexed_treats_other_than_types_as_exclusions() {
        let tokens = tokenize_line("creatures other than Werewolves and Wolves", 0);

        let filter = parse_object_filter_lexed(&tokens, false).expect("object filter should parse");

        assert!(!filter.other);
        assert_eq!(filter.card_types, vec![CardType::Creature]);
        assert_eq!(
            filter.excluded_subtypes,
            vec![Subtype::Werewolf, Subtype::Wolf]
        );
    }

    #[test]
    fn parse_object_filter_lexed_preserves_power_toughness_inequality() {
        let tokens = tokenize_line(
            "target non-Elf creature whose power and toughness aren't equal",
            0,
        );

        let filter = parse_object_filter(&tokens, false).expect("object filter should parse");

        assert_eq!(
            filter.power_toughness_relation,
            Some(crate::filter::PowerToughnessRelation::NotEqual)
        );
        assert_eq!(filter.card_types, vec![CardType::Creature]);
        assert_eq!(filter.excluded_subtypes, vec![Subtype::Elf]);
    }

    #[test]
    fn public_filter_route_preserves_supertype_or_subtype_disjunction() {
        let tokens = tokenize_line(
            "up to one target legendary or Rat card from your graveyard",
            0,
        );

        let filter = parse_object_filter(&tokens, false).expect("object filter should parse");

        assert_eq!(
            filter.zone,
            Some(crate::zone::Zone::Graveyard),
            "{filter:#?}"
        );
        assert_eq!(
            filter.owner,
            Some(crate::target::PlayerFilter::You),
            "{filter:#?}"
        );
        assert_eq!(filter.any_of.len(), 2, "{filter:#?}");
        assert!(filter.any_of.iter().any(|branch| {
            branch.supertypes == [crate::types::Supertype::Legendary] && branch.subtypes.is_empty()
        }));
        assert!(
            filter.any_of.iter().any(|branch| {
                branch.subtypes == [Subtype::Rat] && branch.supertypes.is_empty()
            })
        );
        assert!(
            filter
                .any_of
                .iter()
                .all(|branch| branch.zone.is_none() && branch.owner.is_none())
        );
    }

    #[test]
    fn sacrificed_permanent_card_type_relation_is_deduplicated() {
        let tokens = tokenize_line(
            "a permanent that shares a card type with the sacrificed permanent",
            0,
        );

        let filter = parse_object_filter(&tokens, false).expect("object filter should parse");
        let matching = filter
            .tagged_constraints
            .iter()
            .filter(|constraint| {
                constraint.tag == crate::tag::CompilerReferenceTag::Sacrificed0.bind()
                    && constraint.relation == TaggedOpbjectRelation::SharesCardType
            })
            .count();

        assert_eq!(matching, 1, "{filter:#?}");
    }

    #[test]
    fn parse_object_filter_words_treats_other_than_types_as_exclusions_without_synthetic_tokens() {
        let filter = parse_simple_object_filter_words(
            &["creatures", "other", "than", "werewolves", "and", "wolves"],
            false,
        )
        .expect("object filter should parse");

        assert!(!filter.other);
        assert_eq!(filter.card_types, vec![CardType::Creature]);
        assert_eq!(
            filter.excluded_subtypes,
            vec![Subtype::Werewolf, Subtype::Wolf]
        );
    }

    #[test]
    fn parse_object_filter_lexed_preserves_outer_controller_across_aggregate_scope() {
        let tokens = tokenize_line(
            "a creature an opponent controls with the greatest power among creatures that player controls",
            0,
        );

        let filter = parse_object_filter_lexed(&tokens, false).expect("object filter should parse");

        assert_eq!(filter.controller, Some(PlayerFilter::Opponent));
        assert_eq!(filter.card_types, vec![CardType::Creature]);
    }

    #[test]
    fn parse_object_filter_lexed_parses_permanent_or_owned_suspended_card_disjunction() {
        let tokens = tokenize_line("a permanent you control or suspended card you own", 0);

        let filter = parse_object_filter_lexed(&tokens, false).expect("object filter should parse");

        assert_eq!(filter.any_of.len(), 2, "{filter:?}");
        assert!(
            filter.any_of.iter().any(|arm| {
                arm.zone == Some(Zone::Battlefield)
                    && arm.controller == Some(PlayerFilter::You)
                    && arm.alternative_cast.is_none()
            }),
            "{filter:?}"
        );
        assert!(
            filter.any_of.iter().any(|arm| {
                arm.zone == Some(Zone::Exile)
                    && arm.owner == Some(PlayerFilter::You)
                    && arm.alternative_cast == Some(crate::filter::AlternativeCastKind::Suspend)
            }),
            "{filter:?}"
        );
    }

    #[test]
    fn parse_object_filter_lexed_parses_nonland_permanent_or_suspended_card_disjunction() {
        let tokens = tokenize_line("target nonland permanent or suspended card", 0);

        let filter = parse_object_filter_lexed(&tokens, false).expect("object filter should parse");

        assert_eq!(filter.any_of.len(), 2, "{filter:#?}");
        assert!(filter.any_of.iter().any(|arm| {
            arm.zone == Some(Zone::Battlefield)
                && arm.excluded_card_types == [CardType::Land]
                && arm.alternative_cast.is_none()
        }));
        assert!(filter.any_of.iter().any(|arm| {
            arm.zone == Some(Zone::Exile)
                && arm.alternative_cast == Some(crate::filter::AlternativeCastKind::Suspend)
                && arm.with_counter
                    == Some(crate::filter::CounterConstraint::Typed(
                        crate::object::CounterType::Time,
                    ))
        }));
    }

    #[test]
    fn compound_subtype_outranks_matching_current_source_alias_in_object_filters() {
        let target =
            parse_object_filter_lexed(&tokenize_line("target Time Lord you control", 0), false)
                .expect("typed target should parse");
        assert!(!target.source, "{target:#?}");
        assert_eq!(target.subtypes, [Subtype::TimeLord]);
        assert_eq!(target.controller, Some(PlayerFilter::You));

        let card = parse_object_filter_lexed(&tokenize_line("a Time Lord creature card", 0), false)
            .expect("typed card should parse");
        assert!(!card.source, "{card:#?}");
        assert_eq!(card.subtypes, [Subtype::TimeLord]);
        assert_eq!(card.card_types, [CardType::Creature]);
    }

    #[test]
    fn parse_object_filter_lexed_keeps_repeated_each_suspended_and_permanent_domains() {
        let tokens = tokenize_line(
            "suspended card you own and each other permanent you control with a time counter on it",
            0,
        );

        let filter = parse_object_filter_lexed(&tokens, false).expect("object filter should parse");

        assert_eq!(filter.any_of.len(), 2, "{filter:#?}");
        assert!(filter.any_of.iter().any(|arm| {
            arm.zone == Some(Zone::Exile)
                && arm.owner == Some(PlayerFilter::You)
                && arm.alternative_cast == Some(crate::filter::AlternativeCastKind::Suspend)
        }));
        assert!(filter.any_of.iter().any(|arm| {
            arm.zone == Some(Zone::Battlefield)
                && arm.controller == Some(PlayerFilter::You)
                && arm.other
                && arm.with_counter
                    == Some(crate::filter::CounterConstraint::Typed(
                        crate::object::CounterType::Time,
                    ))
        }));
    }
}
