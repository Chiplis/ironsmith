use super::*;

pub(super) fn parse_filter_face_state(input: &mut WordInput<'_>) -> WResult<FilterFaceState> {
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

pub(super) fn parse_named_object_filter_atom(
    input: &mut WordInput<'_>,
) -> WResult<NamedObjectFilterAtom> {
    alt((
        word_phrase(&["chosen", "color"]).value(NamedObjectFilterAtom::ChosenColor),
        word_phrase(&["that", "color"]).value(NamedObjectFilterAtom::ChosenColor),
        word_phrase(&["chosen", "type"]).value(NamedObjectFilterAtom::ChosenType),
        word_phrase(&["that", "type"]).value(NamedObjectFilterAtom::ChosenType),
        word_phrase(&["nonchosen", "type"]).value(NamedObjectFilterAtom::NonChosenType),
    ))
    .parse_next(input)
}

pub(super) fn contains_simple_filter_reject(words: &[&str]) -> bool {
    for index in 0..words.len() {
        let mut input: WordInput<'_> = &words[index..];
        if parse_simple_filter_reject.parse_next(&mut input).is_ok() {
            return true;
        }
    }
    false
}

pub(super) fn parse_simple_filter_reject(input: &mut WordInput<'_>) -> WResult<()> {
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

pub(super) fn parse_other_than_filter(
    split: OtherThanSplit<'_>,
    other: bool,
) -> Option<ObjectFilter> {
    let mut filter = parse_simple_object_filter_words(split.base, other)?;
    let mut input: WordInput<'_> = split.exclusions;
    let mut saw_exclusion = false;
    while !input.is_empty() {
        match crate::grammar::primitives::take_leaf(&mut input, parse_excluded_object_filter_atom)?
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

pub(super) fn parse_excluded_object_filter_atom(
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

pub(super) fn parse_simple_object_filter_suffix(
    words: &[&str],
) -> Option<(SimpleObjectFilterSuffix, usize)> {
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
    for suffix_len in (2..=5).rev() {
        let Some(tail) = suffix_tail(words, suffix_len) else {
            continue;
        };
        if let Some(suffix) = parse_full_word_slice(tail, parse_location_suffix) {
            return Some((suffix, suffix_len));
        }
    }
    for suffix_len in (2..=7).rev() {
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

pub(super) fn parse_controller_player(input: &mut WordInput<'_>) -> WResult<PlayerFilter> {
    alt((
        alt((
            word_phrase(&["another", "target", "player"]).value(PlayerFilter::Target(Box::new(
                PlayerFilter::excluding(PlayerFilter::Any, PlayerFilter::target_player()),
            ))),
            alt((
                word_phrase(&["target", "opponent"]).value(PlayerFilter::target_opponent()),
                word_phrase(&["target", "player"]).value(PlayerFilter::target_player()),
                word_phrase(&["the", "chosen", "player"]).value(PlayerFilter::ChosenPlayer),
                word_phrase(&["chosen", "player"]).value(PlayerFilter::ChosenPlayer),
                word_phrase(&["that", "player"]).value(PlayerFilter::IteratedPlayer),
                word_phrase(&["your", "team"]).map(|()| PlayerFilter::your_team()),
                primitives::word_slice_exact("opponents").value(PlayerFilter::Opponent),
                primitives::word_slice_exact("opponent").value(PlayerFilter::Opponent),
                primitives::word_slice_exact("you").value(PlayerFilter::You),
            )),
        )),
        parse_target_player_or_planeswalker_controller,
    ))
    .parse_next(input)
}

pub(super) fn parse_target_player_or_planeswalker_controller(
    input: &mut WordInput<'_>,
) -> WResult<PlayerFilter> {
    let checkpoint = *input;
    let Some((player, consumed)) =
        super::super::parse_player_relation_subject(input, &PlayerFilter::IteratedPlayer)
    else {
        return Err(primitives::backtrack_err(
            "simple object filter controller",
            "player-or-planeswalker target reference",
        ));
    };
    if player != PlayerFilter::TargetPlayerOrControllerOfTarget {
        return Err(primitives::backtrack_err(
            "simple object filter controller",
            "player-or-planeswalker target reference",
        ));
    }
    *input = &checkpoint[consumed..];
    Ok(player)
}

pub(super) fn apply_simple_object_filter_suffix(
    filter: &mut ObjectFilter,
    suffix: SimpleObjectFilterSuffix,
) {
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
