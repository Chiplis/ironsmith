use super::*;

pub(super) fn parse_typed_word_atom(input: &mut WordInput<'_>) -> WResult<SimpleObjectFilterAtom> {
    let checkpoint = *input;
    let word = parse_any_word.parse_next(input)?;
    // Flexible positive characteristic parsers intentionally tolerate several
    // surface prefixes. Test the explicit `non-` forms first so a word such
    // as `non-Equipment` cannot be recorded as both Equipment and an
    // Equipment exclusion, producing an impossible branch.
    let atom = if let Some(card_type) = parse_non_type(word) {
        SimpleObjectFilterAtom::ExcludedCardType(card_type)
    } else if let Some(subtype) = parse_non_subtype(word) {
        SimpleObjectFilterAtom::ExcludedSubtype(subtype)
    } else if let Some(supertype) = parse_non_supertype(word) {
        SimpleObjectFilterAtom::ExcludedSupertype(supertype)
    } else if let Some(color) = parse_non_color(word) {
        SimpleObjectFilterAtom::ExcludedColor(color)
    } else if let Some(card_type) = parse_card_type(word) {
        SimpleObjectFilterAtom::CardType(card_type)
    } else if let Some(subtype) = parse_subtype_flexible(word) {
        SimpleObjectFilterAtom::Subtype(subtype)
    } else if let Some(subtype) = super::super::super::leaf::classify_token_definition_subtype(word)
        .filter(|_| {
            input
                .first()
                .and_then(|next| parse_subtype_flexible(next))
                .is_some()
        })
    {
        // Ambiguous English nouns such as "Sand" are rejected by the broad
        // subtype parser, but become unambiguous when immediately followed by
        // another subtype in a compound type phrase ("Sand Warriors").
        SimpleObjectFilterAtom::Subtype(subtype)
    } else if let Some(supertype) = parse_supertype_word(word) {
        SimpleObjectFilterAtom::Supertype(supertype)
    } else if let Some(color) = parse_color(word) {
        SimpleObjectFilterAtom::Color(color)
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

pub(super) fn apply_named_atom(filter: &mut ObjectFilter, atom: NamedObjectFilterAtom) {
    match atom {
        NamedObjectFilterAtom::ChosenColor => filter.chosen_color = true,
        NamedObjectFilterAtom::ChosenType => filter.chosen_creature_type = true,
        NamedObjectFilterAtom::NonChosenType => filter.excluded_chosen_creature_type = true,
    }
}

pub(super) fn parse_other_than_split<'a>(words: &'a [&'a str]) -> Option<OtherThanSplit<'a>> {
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

pub(super) fn parse_location_suffix(
    input: &mut WordInput<'_>,
) -> WResult<SimpleObjectFilterSuffix> {
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

pub(super) fn parse_owner_suffix(input: &mut WordInput<'_>) -> WResult<SimpleObjectFilterSuffix> {
    primitives::word_slice_exact("you")
        .void()
        .parse_next(input)?;
    parse_own_action.parse_next(input)?;
    Ok(SimpleObjectFilterSuffix::Owner(PlayerFilter::You))
}

pub(super) fn parse_own_action(input: &mut WordInput<'_>) -> WResult<()> {
    alt((
        primitives::word_slice_exact("own"),
        primitives::word_slice_exact("owns"),
    ))
    .void()
    .parse_next(input)
}

pub(super) fn parse_location(input: &mut WordInput<'_>) -> WResult<(Option<PlayerFilter>, Zone)> {
    alt((
        alt((
            parse_chosen_player_location,
            word_phrase(&["defending", "player", "graveyard"])
                .value((Some(PlayerFilter::Defending), Zone::Graveyard)),
            word_phrase(&["defending", "players", "graveyard"])
                .value((Some(PlayerFilter::Defending), Zone::Graveyard)),
            word_phrase(&["your", "graveyard"]).value((Some(PlayerFilter::You), Zone::Graveyard)),
            word_phrase(&["your", "hand"]).value((Some(PlayerFilter::You), Zone::Hand)),
            word_phrase(&["your", "library"]).value((Some(PlayerFilter::You), Zone::Library)),
        )),
        alt((
            word_phrase(&["all", "graveyards"]).value((None, Zone::Graveyard)),
            primitives::word_slice_exact("graveyard").value((None, Zone::Graveyard)),
            primitives::word_slice_exact("hand").value((None, Zone::Hand)),
            primitives::word_slice_exact("library").value((None, Zone::Library)),
            primitives::word_slice_exact("exile").value((None, Zone::Exile)),
        )),
    ))
    .parse_next(input)
}

pub(super) fn suffix_tail<'a>(words: &'a [&'a str], suffix_len: usize) -> Option<&'a [&'a str]> {
    words.get(words.len().checked_sub(suffix_len)?..)
}

pub(super) fn word_phrase<'a>(
    expected: &'static [&'static str],
) -> impl Parser<WordInput<'a>, (), ErrMode<ContextError>> {
    move |input: &mut WordInput<'a>| {
        let checkpoint = *input;
        for word in expected {
            if let Err(err) = primitives::word_slice_exact(word).void().parse_next(input) {
                *input = checkpoint;
                return Err(err);
            }
        }
        Ok(())
    }
}

pub(super) fn parse_any_word<'a>(input: &mut WordInput<'a>) -> WResult<&'a str> {
    let Some((word, rest)) = input.split_first() else {
        return Err(primitives::backtrack_err(
            "simple object filter word",
            "word",
        ));
    };
    *input = rest;
    Ok(*word)
}

pub(super) fn push_unique<T: Copy + PartialEq>(items: &mut Vec<T>, value: T) {
    crate::slice_primitives::push_unique(items, value);
}
