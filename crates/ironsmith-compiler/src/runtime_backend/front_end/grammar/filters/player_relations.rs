use super::*;
use winnow::combinator::{alt, opt};
use winnow::error::ModalResult as WResult;
use winnow::token::any;

pub(super) type GrammarFilterNormalizedWords<'a> = TokenWordView<'a>;

pub(super) fn push_unique_filter_value<T: Copy + PartialEq>(items: &mut Vec<T>, value: T) {
    crate::slice_primitives::push_unique(items, value);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SpellFilterComparisonAxis {
    Power,
    Toughness,
    ManaValue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum PlayerRelationVerb {
    Cast,
    Control,
    Own,
}

#[derive(Clone, Copy)]
pub(super) struct SegmentPhraseVariant {
    words: &'static [&'static str],
    drain_start_offset: usize,
}

const LEADING_TAGGED_REFERENCE_WORDS: &[&str] = &["that", "those", "chosen"];
const IT_OR_THEM_WORDS: &[&str] = &["it", "them"];
const PUT_ONTO_BATTLEFIELD_WITH_SOURCE_PHRASES: &[&[&str]] = &[
    &["put", "onto", "battlefield", "with", "this", "artifact"],
    &["put", "onto", "battlefield", "with", "this", "enchantment"],
    &["put", "onto", "battlefield", "with", "this", "permanent"],
    &["put", "onto", "battlefield", "with", "this", "source"],
];
const CREATED_WITH_SOURCE_PHRASES: &[&[&str]] = &[
    &["created", "with", "it"],
    &["created", "with", "this", "artifact"],
    &["created", "with", "this", "creature"],
    &["created", "with", "this", "enchantment"],
    &["created", "with", "this", "permanent"],
    &["created", "with", "this", "source"],
];
const DIDNT_ATTACK_OR_ENTER_THIS_TURN_PHRASES: &[&[&str]] = &[
    &["didn't", "attack", "or", "enter", "this", "turn"],
    &["didnt", "attack", "or", "enter", "this", "turn"],
    &["did", "not", "attack", "or", "enter", "this", "turn"],
];
const DIDNT_ENTER_BATTLEFIELD_THIS_TURN_PHRASES: &[&[&str]] = &[
    &["didn't", "enter", "this", "turn"],
    &["didnt", "enter", "this", "turn"],
    &["did", "not", "enter", "this", "turn"],
    &["didn't", "enter", "the", "battlefield", "this", "turn"],
    &["didnt", "enter", "the", "battlefield", "this", "turn"],
    &["did", "not", "enter", "the", "battlefield", "this", "turn"],
];
const TARGET_PLAYER_OR_PLANESWALKER_CONTROLLER_CONTROL_PHRASES: &[&[&str]] = &[
    &[
        "that",
        "player",
        "or",
        "that",
        "planeswalkers",
        "controller",
        "control",
    ],
    &[
        "that",
        "player",
        "or",
        "that",
        "planeswalkers",
        "controller",
        "controls",
    ],
    &[
        "that",
        "opponent",
        "or",
        "that",
        "planeswalkers",
        "controller",
        "control",
    ],
    &[
        "that",
        "opponent",
        "or",
        "that",
        "planeswalkers",
        "controller",
        "controls",
    ],
];

fn relation_phrase<'a>(
    expected: &'static [&'static str],
) -> impl Parser<primitives::WordSliceInput<'a>, (), winnow::error::ErrMode<winnow::error::ContextError>>
{
    move |input: &mut primitives::WordSliceInput<'a>| parse_relation_phrase(input, expected)
}

fn parse_relation_axis_word_slice(
    input: &mut primitives::WordSliceInput<'_>,
) -> WResult<SpellFilterComparisonAxis> {
    alt((
        relation_phrase(&["mana", "value"]).value(SpellFilterComparisonAxis::ManaValue),
        relation_phrase(&["power"]).value(SpellFilterComparisonAxis::Power),
        relation_phrase(&["toughness"]).value(SpellFilterComparisonAxis::Toughness),
    ))
    .parse_next(input)
}

fn parse_relation_verb_word_slice(
    input: &mut primitives::WordSliceInput<'_>,
) -> WResult<PlayerRelationVerb> {
    alt((
        relation_phrase(&["cast"]).value(PlayerRelationVerb::Cast),
        relation_phrase(&["casts"]).value(PlayerRelationVerb::Cast),
        relation_phrase(&["control"]).value(PlayerRelationVerb::Control),
        relation_phrase(&["controls"]).value(PlayerRelationVerb::Control),
        relation_phrase(&["own"]).value(PlayerRelationVerb::Own),
        relation_phrase(&["owns"]).value(PlayerRelationVerb::Own),
    ))
    .parse_next(input)
}

fn parse_passive_relation_verb_word_slice(
    input: &mut primitives::WordSliceInput<'_>,
) -> WResult<PlayerRelationVerb> {
    alt((
        relation_phrase(&["cast", "by"]).value(PlayerRelationVerb::Cast),
        relation_phrase(&["controlled", "by"]).value(PlayerRelationVerb::Control),
        relation_phrase(&["owned", "by"]).value(PlayerRelationVerb::Own),
    ))
    .parse_next(input)
}

fn parse_relation_subject_word_slice(
    input: &mut primitives::WordSliceInput<'_>,
    pronoun_player_filter: &PlayerFilter,
) -> WResult<PlayerFilter> {
    alt((
        alt((
            alt((
                relation_phrase(&[
                    "that",
                    "player",
                    "or",
                    "that",
                    "planeswalkers",
                    "controller",
                ]),
                relation_phrase(&[
                    "that",
                    "opponent",
                    "or",
                    "that",
                    "planeswalkers",
                    "controller",
                ]),
            ))
            .value(PlayerFilter::TargetPlayerOrControllerOfTarget),
            relation_phrase(&["your", "team"]).map(|()| PlayerFilter::your_team()),
            relation_phrase(&["your", "opponents"]).value(PlayerFilter::Opponent),
            relation_phrase(&["that", "player"]).value(PlayerFilter::IteratedPlayer),
            relation_phrase(&["target", "player"]).map(|()| PlayerFilter::target_player()),
            relation_phrase(&["target", "opponent"]).map(|()| PlayerFilter::target_opponent()),
            relation_phrase(&["defending", "player"]).value(PlayerFilter::Defending),
            relation_phrase(&["attacking", "player"]).value(PlayerFilter::Attacking),
            relation_phrase(&["its", "controller"])
                .map(|()| PlayerFilter::ControllerOf(crate::filter::ObjectRef::Target)),
        )),
        alt((
            relation_phrase(&["its", "controllers"])
                .map(|()| PlayerFilter::ControllerOf(crate::filter::ObjectRef::Target)),
            relation_phrase(&["enchanted", "player"])
                .map(|()| PlayerFilter::TaggedPlayer(crate::TagKey::from("enchanted"))),
            relation_phrase(&["their", "controller"])
                .map(|()| PlayerFilter::ControllerOf(crate::filter::ObjectRef::Target)),
            relation_phrase(&["their", "controllers"])
                .map(|()| PlayerFilter::ControllerOf(crate::filter::ObjectRef::Target)),
            relation_phrase(&["those", "opponents"])
                .map(|()| PlayerFilter::AliasedTarget(Box::new(PlayerFilter::Opponent))),
            relation_phrase(&["you"]).value(PlayerFilter::You),
            relation_phrase(&["opponent"]).value(PlayerFilter::Opponent),
            relation_phrase(&["opponents"]).value(PlayerFilter::Opponent),
            alt((
                relation_phrase(&["voter"]).value(PlayerFilter::IteratedPlayer),
                relation_phrase(&["they"]).map(|()| pronoun_player_filter.clone()),
            )),
        )),
    ))
    .parse_next(input)
}

fn parse_negated_you_relation_word_slice(
    input: &mut primitives::WordSliceInput<'_>,
) -> WResult<PlayerRelationVerb> {
    opt(primitives::word_slice_exact("you"))
        .void()
        .parse_next(input)?;
    alt((
        alt((
            relation_phrase(&["do", "not", "control"]).value(PlayerRelationVerb::Control),
            relation_phrase(&["do", "not", "controls"]).value(PlayerRelationVerb::Control),
            relation_phrase(&["dont", "control"]).value(PlayerRelationVerb::Control),
            relation_phrase(&["dont", "controls"]).value(PlayerRelationVerb::Control),
            relation_phrase(&["don't", "control"]).value(PlayerRelationVerb::Control),
            relation_phrase(&["don't", "controls"]).value(PlayerRelationVerb::Control),
        )),
        alt((
            relation_phrase(&["do", "not", "own"]).value(PlayerRelationVerb::Own),
            relation_phrase(&["do", "not", "owns"]).value(PlayerRelationVerb::Own),
            relation_phrase(&["dont", "own"]).value(PlayerRelationVerb::Own),
            relation_phrase(&["dont", "owns"]).value(PlayerRelationVerb::Own),
            relation_phrase(&["don't", "own"]).value(PlayerRelationVerb::Own),
            relation_phrase(&["don't", "owns"]).value(PlayerRelationVerb::Own),
        )),
    ))
    .parse_next(input)
}

fn parse_chosen_player_graveyard_word_slice(
    input: &mut primitives::WordSliceInput<'_>,
) -> WResult<(PlayerFilter, Zone)> {
    opt(primitives::word_slice_exact("the"))
        .void()
        .parse_next(input)?;
    primitives::word_slice_exact("chosen")
        .void()
        .parse_next(input)?;
    alt((
        primitives::word_slice_exact("player"),
        primitives::word_slice_exact("players"),
    ))
    .void()
    .parse_next(input)?;
    primitives::word_slice_exact("graveyard")
        .void()
        .parse_next(input)?;
    Ok((PlayerFilter::ChosenPlayer, Zone::Graveyard))
}

fn parse_ownership_verb_word_slice(
    input: &mut primitives::WordSliceInput<'_>,
) -> WResult<PlayerRelationVerb> {
    alt((
        relation_phrase(&["own"]).value(PlayerRelationVerb::Own),
        relation_phrase(&["owns"]).value(PlayerRelationVerb::Own),
        relation_phrase(&["control"]).value(PlayerRelationVerb::Control),
        relation_phrase(&["controls"]).value(PlayerRelationVerb::Control),
    ))
    .parse_next(input)
}

fn parse_owner_controller_pair_word_slice(
    input: &mut primitives::WordSliceInput<'_>,
    separator: &'static str,
    leading_both: bool,
) -> WResult<(PlayerRelationVerb, PlayerRelationVerb)> {
    if leading_both {
        primitives::word_slice_exact("both")
            .void()
            .parse_next(input)?;
    }
    let first = parse_ownership_verb_word_slice(input)?;
    primitives::word_slice_exact(separator)
        .void()
        .parse_next(input)?;
    let second = parse_ownership_verb_word_slice(input)?;
    if matches!(
        (first, second),
        (PlayerRelationVerb::Own, PlayerRelationVerb::Control)
            | (PlayerRelationVerb::Control, PlayerRelationVerb::Own)
    ) {
        Ok((first, second))
    } else {
        Err(primitives::backtrack_err(
            "owner/controller relation",
            "one ownership and one control verb",
        ))
    }
}

fn parse_put_there_from_battlefield_this_turn_word_slice(
    input: &mut primitives::WordSliceInput<'_>,
) -> WResult<()> {
    primitives::word_slice_exact("that")
        .void()
        .parse_next(input)?;
    alt((
        primitives::word_slice_exact("was"),
        primitives::word_slice_exact("were"),
    ))
    .void()
    .parse_next(input)?;
    relation_phrase(&["put", "there", "from", "battlefield", "this", "turn"]).parse_next(input)
}

fn parse_put_there_from_anywhere_this_turn_word_slice(
    input: &mut primitives::WordSliceInput<'_>,
) -> WResult<()> {
    primitives::word_slice_exact("that")
        .void()
        .parse_next(input)?;
    alt((
        primitives::word_slice_exact("was"),
        primitives::word_slice_exact("were"),
    ))
    .void()
    .parse_next(input)?;
    relation_phrase(&["put", "there", "from", "anywhere", "this", "turn"]).parse_next(input)
}

fn parse_graveyard_from_battlefield_this_turn_word_slice(
    input: &mut primitives::WordSliceInput<'_>,
) -> WResult<()> {
    alt((
        primitives::word_slice_exact("graveyard"),
        primitives::word_slice_exact("graveyards"),
    ))
    .void()
    .parse_next(input)?;
    relation_phrase(&["from", "battlefield", "this", "turn"]).parse_next(input)
}

fn parse_entered_battlefield_this_turn_word_slice(
    input: &mut primitives::WordSliceInput<'_>,
) -> WResult<Option<PlayerFilter>> {
    primitives::word_slice_exact("entered")
        .void()
        .parse_next(input)?;
    opt((
        opt(primitives::word_slice_exact("the")).void(),
        primitives::word_slice_exact("battlefield").void(),
    ))
    .void()
    .parse_next(input)?;
    let controller = opt(alt((
        relation_phrase(&["under", "your", "control"]).value(PlayerFilter::You),
        relation_phrase(&["under", "opponent", "control"]).value(PlayerFilter::Opponent),
        relation_phrase(&["under", "opponents", "control"]).value(PlayerFilter::Opponent),
    )))
    .parse_next(input)?;
    relation_phrase(&["this", "turn"]).parse_next(input)?;
    Ok(controller)
}

fn parse_drawn_this_turn_word_slice(input: &mut primitives::WordSliceInput<'_>) -> WResult<()> {
    relation_phrase(&["drawn", "this", "turn"]).parse_next(input)
}

fn parse_relation_axis_shape(words: &[&str]) -> Option<(SpellFilterComparisonAxis, usize)> {
    let mut input: primitives::WordSliceInput<'_> = words;
    let axis = parse_relation_axis_word_slice(&mut input).ok()?;
    Some((axis, words.len().saturating_sub(input.len())))
}

fn parse_relation_verb_shape(words: &[&str]) -> Option<(PlayerRelationVerb, usize)> {
    let mut input: primitives::WordSliceInput<'_> = words;
    let verb = parse_relation_verb_word_slice(&mut input).ok()?;
    Some((verb, words.len().saturating_sub(input.len())))
}

fn parse_passive_relation_verb_shape(words: &[&str]) -> Option<(PlayerRelationVerb, usize)> {
    let mut input: primitives::WordSliceInput<'_> = words;
    let verb = parse_passive_relation_verb_word_slice(&mut input).ok()?;
    Some((verb, words.len().saturating_sub(input.len())))
}

fn parse_relation_subject_shape(
    words: &[&str],
    pronoun_player_filter: &PlayerFilter,
) -> Option<(PlayerFilter, usize)> {
    let mut input: primitives::WordSliceInput<'_> = words;
    let player = parse_relation_subject_word_slice(&mut input, pronoun_player_filter).ok()?;
    Some((player, words.len().saturating_sub(input.len())))
}

fn parse_negated_you_relation_shape(words: &[&str]) -> Option<(PlayerRelationVerb, usize)> {
    let mut input: primitives::WordSliceInput<'_> = words;
    let verb = parse_negated_you_relation_word_slice(&mut input).ok()?;
    Some((verb, words.len().saturating_sub(input.len())))
}

fn parse_chosen_player_graveyard_shape(words: &[&str]) -> Option<(PlayerFilter, Zone, usize)> {
    let mut input: primitives::WordSliceInput<'_> = words;
    let (owner, zone) = parse_chosen_player_graveyard_word_slice(&mut input).ok()?;
    Some((owner, zone, words.len().saturating_sub(input.len())))
}

fn parse_joint_owner_controller_shape(words: &[&str]) -> Option<usize> {
    let mut input: primitives::WordSliceInput<'_> = words;
    parse_owner_controller_pair_word_slice(&mut input, "and", true).ok()?;
    Some(words.len().saturating_sub(input.len()))
}

fn parse_owner_or_controller_shape(words: &[&str]) -> Option<usize> {
    let mut input: primitives::WordSliceInput<'_> = words;
    parse_owner_controller_pair_word_slice(&mut input, "or", false).ok()?;
    Some(words.len().saturating_sub(input.len()))
}

fn parse_put_there_from_battlefield_this_turn_shape(words: &[&str]) -> Option<usize> {
    let mut input: primitives::WordSliceInput<'_> = words;
    parse_put_there_from_battlefield_this_turn_word_slice(&mut input).ok()?;
    Some(words.len().saturating_sub(input.len()))
}

fn parse_put_there_from_anywhere_this_turn_shape(words: &[&str]) -> Option<usize> {
    let mut input: primitives::WordSliceInput<'_> = words;
    parse_put_there_from_anywhere_this_turn_word_slice(&mut input).ok()?;
    Some(words.len().saturating_sub(input.len()))
}

fn parse_graveyard_from_battlefield_this_turn_shape(words: &[&str]) -> Option<usize> {
    let mut input: primitives::WordSliceInput<'_> = words;
    parse_graveyard_from_battlefield_this_turn_word_slice(&mut input).ok()?;
    Some(words.len().saturating_sub(input.len()))
}

fn parse_entered_battlefield_this_turn_shape(
    words: &[&str],
) -> Option<(Option<PlayerFilter>, usize)> {
    let mut input: primitives::WordSliceInput<'_> = words;
    let controller = parse_entered_battlefield_this_turn_word_slice(&mut input).ok()?;
    Some((controller, words.len().saturating_sub(input.len())))
}

fn parse_drawn_this_turn_shape(words: &[&str]) -> Option<usize> {
    let mut input: primitives::WordSliceInput<'_> = words;
    parse_drawn_this_turn_word_slice(&mut input).ok()?;
    Some(words.len().saturating_sub(input.len()))
}

fn parse_was_dealt_damage_this_turn_shape(words: &[&str]) -> Option<usize> {
    const PHRASES: &[&[&str]] = &[
        &["that", "was", "dealt", "damage", "this", "turn"],
        &["that", "were", "dealt", "damage", "this", "turn"],
        &["was", "dealt", "damage", "this", "turn"],
        &["were", "dealt", "damage", "this", "turn"],
        // Oracle sometimes elides "was" after a quantified object, as in
        // "each creature dealt damage this turn" (Inflame).
        &["dealt", "damage", "this", "turn"],
    ];
    PHRASES
        .iter()
        .find(|phrase| words.starts_with(phrase))
        .map(|phrase| phrase.len())
}

fn parse_put_there_this_turn_shape(words: &[&str]) -> Option<usize> {
    const PHRASES: &[&[&str]] = &[
        &["that", "was", "put", "there", "this", "turn"],
        &["that", "were", "put", "there", "this", "turn"],
    ];
    PHRASES
        .iter()
        .find(|phrase| words.starts_with(phrase))
        .map(|phrase| phrase.len())
}

impl SpellFilterComparisonAxis {
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::Power => "power",
            Self::Toughness => "toughness",
            Self::ManaValue => "mana value",
        }
    }

    pub(super) fn assign(self, filter: &mut ObjectFilter, comparison: crate::target::Comparison) {
        match self {
            Self::Power => filter.power = Some(comparison),
            Self::Toughness => filter.toughness = Some(comparison),
            Self::ManaValue => filter.mana_value = Some(comparison),
        }
    }
}

pub(super) fn parse_spell_filter_comparison_axis_words(
    words: &[&str],
) -> Option<(SpellFilterComparisonAxis, usize)> {
    parse_relation_axis_shape(words)
}

pub(super) fn parse_player_relation_verb(words: &[&str]) -> Option<(PlayerRelationVerb, usize)> {
    parse_relation_verb_shape(words)
}

pub(super) fn parse_player_relation_subject(
    words: &[&str],
    pronoun_player_filter: &PlayerFilter,
) -> Option<(PlayerFilter, usize)> {
    parse_relation_subject_shape(words, pronoun_player_filter)
}

pub(super) fn apply_player_relation(
    filter: &mut ObjectFilter,
    player: PlayerFilter,
    verb: PlayerRelationVerb,
) {
    match verb {
        PlayerRelationVerb::Cast => filter.cast_by = Some(player),
        PlayerRelationVerb::Control => filter.controller = Some(player),
        PlayerRelationVerb::Own => filter.owner = Some(player),
    }
}

pub(super) fn try_apply_player_relation_clause(
    filter: &mut ObjectFilter,
    words: &[&str],
    pronoun_player_filter: &PlayerFilter,
) -> Option<usize> {
    let (player, subject_consumed) = parse_player_relation_subject(words, pronoun_player_filter)?;
    let (verb, verb_consumed) = parse_player_relation_verb(&words[subject_consumed..])?;

    if matches!(player, PlayerFilter::Defending | PlayerFilter::Attacking)
        && !matches!(verb, PlayerRelationVerb::Control)
    {
        return None;
    }
    if matches!(player, PlayerFilter::ControllerOf(_))
        && !matches!(verb, PlayerRelationVerb::Control)
    {
        return None;
    }

    apply_player_relation(filter, player, verb);
    Some(subject_consumed + verb_consumed)
}

pub(super) fn try_apply_passive_player_relation_clause(
    filter: &mut ObjectFilter,
    words: &[&str],
    pronoun_player_filter: &PlayerFilter,
) -> Option<usize> {
    let (verb, verb_consumed) = parse_passive_relation_verb_shape(words)?;
    let (player, subject_consumed) =
        parse_player_relation_subject(&words[verb_consumed..], pronoun_player_filter)?;

    if matches!(player, PlayerFilter::Defending | PlayerFilter::Attacking)
        && !matches!(verb, PlayerRelationVerb::Control)
    {
        return None;
    }
    if matches!(player, PlayerFilter::ControllerOf(_))
        && !matches!(verb, PlayerRelationVerb::Control)
    {
        return None;
    }

    apply_player_relation(filter, player, verb);
    Some(verb_consumed + subject_consumed)
}

pub(super) fn try_apply_negated_you_relation_clause(
    filter: &mut ObjectFilter,
    words: &[&str],
    pronoun_player_filter: &PlayerFilter,
) -> Option<usize> {
    if words.first() == Some(&"they") {
        let (verb, consumed) = parse_negated_you_relation_shape(&words[1..])?;
        let excluding_pronoun =
            PlayerFilter::excluding(PlayerFilter::Any, pronoun_player_filter.clone());
        match verb {
            PlayerRelationVerb::Control => filter.controller = Some(excluding_pronoun),
            PlayerRelationVerb::Own => filter.owner = Some(excluding_pronoun),
            PlayerRelationVerb::Cast => return None,
        }
        return Some(consumed + 1);
    }

    let (verb, consumed) = parse_negated_you_relation_shape(words)?;
    match verb {
        PlayerRelationVerb::Control => filter.controller = Some(PlayerFilter::NotYou),
        PlayerRelationVerb::Own => filter.owner = Some(PlayerFilter::NotYou),
        PlayerRelationVerb::Cast => return None,
    }
    Some(consumed)
}

pub(super) fn try_apply_chosen_player_graveyard_clause(
    filter: &mut ObjectFilter,
    words: &[&str],
) -> Option<usize> {
    let (owner, zone, consumed) = parse_chosen_player_graveyard_shape(words)?;
    filter.owner = Some(owner);
    filter.zone = Some(zone);
    Some(consumed)
}

pub(super) fn try_apply_joint_owner_controller_clause(
    filter: &mut ObjectFilter,
    words: &[&str],
    pronoun_player_filter: &PlayerFilter,
) -> Option<usize> {
    let (player, subject_consumed) = parse_player_relation_subject(words, pronoun_player_filter)?;
    let consumed = parse_joint_owner_controller_shape(&words[subject_consumed..])?;
    filter.owner = Some(player.clone());
    filter.controller = Some(player);
    Some(subject_consumed + consumed)
}

pub(super) fn parse_owner_or_controller_disjunction_player(
    words: &[&str],
    pronoun_player_filter: &PlayerFilter,
) -> Option<(PlayerFilter, usize)> {
    let (player, subject_consumed) = parse_player_relation_subject(words, pronoun_player_filter)?;
    if matches!(
        player,
        PlayerFilter::Defending | PlayerFilter::Attacking | PlayerFilter::ControllerOf(_)
    ) {
        return None;
    }
    let consumed = parse_owner_or_controller_shape(&words[subject_consumed..])?;
    Some((player, subject_consumed + consumed))
}

pub(super) fn find_filter_prefix_consumed<F>(words: &[&str], parser: F) -> Option<(usize, usize)>
where
    F: Fn(&[&str]) -> Option<usize>,
{
    words
        .iter()
        .enumerate()
        .find_map(|(idx, _)| parser(&words[idx..]).map(|consumed| (idx, consumed)))
}

pub(super) fn drain_segment_phrase_variants(
    segment_tokens: &mut Vec<OwnedLexToken>,
    variants: &[SegmentPhraseVariant],
) {
    let segment_words_view = GrammarFilterNormalizedWords::new(segment_tokens.as_slice());
    let segment_words = segment_words_view.to_word_refs();

    let mut segment_match = None;
    for variant in variants {
        if let Some((first, end)) = relation_phrase_word_span(&segment_words, variant.words) {
            segment_match = Some((first + variant.drain_start_offset, end));
            break;
        }
    }

    if let Some((start_word_idx, end_word_idx)) = segment_match
        && let Some(token_range) =
            segment_words_view.token_span_for_words(start_word_idx, end_word_idx)
    {
        segment_tokens.drain(token_range);
    }
}

fn drain_segment_matching_phrase(segment_tokens: &mut Vec<OwnedLexToken>, phrases: &[&[&str]]) {
    let segment_words_view = GrammarFilterNormalizedWords::new(segment_tokens.as_slice());
    let segment_words = segment_words_view.to_word_refs();
    let matched = phrases
        .iter()
        .find_map(|phrase| relation_phrase_word_span(&segment_words, phrase));

    if let Some((start_word_idx, end_word_idx)) = matched
        && let Some(token_range) =
            segment_words_view.token_span_for_words(start_word_idx, end_word_idx)
    {
        segment_tokens.drain(token_range);
    }
}

fn relation_phrase_word_span(words: &[&str], phrase: &[&str]) -> Option<(usize, usize)> {
    let mut input: primitives::WordSliceInput<'_> = words;
    let initial_len = input.len();
    loop {
        let first = initial_len.saturating_sub(input.len());
        let mut candidate = input;
        if parse_relation_phrase(&mut candidate, phrase).is_ok() {
            let end = initial_len.saturating_sub(candidate.len());
            return Some((first, end));
        }
        let consumed: WResult<&str> = any.parse_next(&mut input);
        consumed.ok()?;
    }
}

fn parse_relation_phrase(
    input: &mut primitives::WordSliceInput<'_>,
    phrase: &[&str],
) -> WResult<()> {
    if phrase.is_empty() {
        return Err(primitives::backtrack_err(
            "player-relation phrase",
            "non-empty phrase",
        ));
    }
    for expected in phrase {
        let word: &str = any.parse_next(input)?;
        if word != *expected {
            return Err(primitives::backtrack_err(
                "player-relation phrase",
                "expected phrase word",
            ));
        }
    }
    Ok(())
}

pub(super) fn parse_put_there_from_battlefield_this_turn_words(words: &[&str]) -> Option<usize> {
    parse_put_there_from_battlefield_this_turn_shape(words)
}

pub(super) fn parse_put_there_from_anywhere_this_turn_words(words: &[&str]) -> Option<usize> {
    parse_put_there_from_anywhere_this_turn_shape(words)
}

pub(super) fn parse_graveyard_from_battlefield_this_turn_words(words: &[&str]) -> Option<usize> {
    parse_graveyard_from_battlefield_this_turn_shape(words)
}

pub(super) fn parse_entered_battlefield_this_turn_words(
    words: &[&str],
) -> Option<(Option<PlayerFilter>, usize)> {
    parse_entered_battlefield_this_turn_shape(words)
}

pub(super) fn try_apply_put_there_from_battlefield_this_turn_clause(
    filter: &mut ObjectFilter,
    all_words: &mut Vec<&str>,
    segment_tokens: &mut Vec<OwnedLexToken>,
) -> bool {
    let Some((word_start, consumed)) = find_filter_prefix_consumed(
        all_words.as_slice(),
        parse_put_there_from_battlefield_this_turn_words,
    ) else {
        return false;
    };
    filter.entered_graveyard_this_turn = true;
    filter.entered_graveyard_from_battlefield_this_turn = true;
    filter.set_graveyard_entry_history_surface(Some(
        crate::filter::GraveyardEntryHistorySurface::PutThereFromBattlefieldThisTurn,
    ));
    all_words.drain(word_start..word_start + consumed);
    drain_segment_phrase_variants(
        segment_tokens,
        &[
            SegmentPhraseVariant {
                words: &[
                    "that",
                    "was",
                    "put",
                    "there",
                    "from",
                    "the",
                    "battlefield",
                    "this",
                    "turn",
                ],
                drain_start_offset: 0,
            },
            SegmentPhraseVariant {
                words: &[
                    "that",
                    "was",
                    "put",
                    "there",
                    "from",
                    "battlefield",
                    "this",
                    "turn",
                ],
                drain_start_offset: 0,
            },
            SegmentPhraseVariant {
                words: &[
                    "that",
                    "were",
                    "put",
                    "there",
                    "from",
                    "the",
                    "battlefield",
                    "this",
                    "turn",
                ],
                drain_start_offset: 0,
            },
            SegmentPhraseVariant {
                words: &[
                    "that",
                    "were",
                    "put",
                    "there",
                    "from",
                    "battlefield",
                    "this",
                    "turn",
                ],
                drain_start_offset: 0,
            },
        ],
    );
    true
}

pub(super) fn try_apply_put_there_from_anywhere_this_turn_clause(
    filter: &mut ObjectFilter,
    all_words: &mut Vec<&str>,
    segment_tokens: &mut Vec<OwnedLexToken>,
) -> bool {
    let Some((word_start, consumed)) = find_filter_prefix_consumed(
        all_words.as_slice(),
        parse_put_there_from_anywhere_this_turn_words,
    ) else {
        return false;
    };
    filter.entered_graveyard_this_turn = true;
    filter.set_graveyard_entry_history_surface(Some(
        crate::filter::GraveyardEntryHistorySurface::PutThereFromAnywhereThisTurn,
    ));
    all_words.drain(word_start..word_start + consumed);
    drain_segment_phrase_variants(
        segment_tokens,
        &[
            SegmentPhraseVariant {
                words: &[
                    "that", "was", "put", "there", "from", "anywhere", "this", "turn",
                ],
                drain_start_offset: 0,
            },
            SegmentPhraseVariant {
                words: &[
                    "that", "were", "put", "there", "from", "anywhere", "this", "turn",
                ],
                drain_start_offset: 0,
            },
        ],
    );
    true
}

pub(super) fn try_apply_put_there_this_turn_clause(
    filter: &mut ObjectFilter,
    all_words: &mut Vec<&str>,
    segment_tokens: &mut Vec<OwnedLexToken>,
) -> bool {
    let Some((word_start, consumed)) = all_words.iter().enumerate().find_map(|(idx, _)| {
        parse_put_there_this_turn_shape(&all_words[idx..]).map(|consumed| (idx, consumed))
    }) else {
        return false;
    };
    filter.entered_graveyard_this_turn = true;
    filter.set_graveyard_entry_history_surface(Some(
        crate::filter::GraveyardEntryHistorySurface::PutThereThisTurn,
    ));
    all_words.drain(word_start..word_start + consumed);
    drain_segment_phrase_variants(
        segment_tokens,
        &[
            SegmentPhraseVariant {
                words: &["that", "was", "put", "there", "this", "turn"],
                drain_start_offset: 0,
            },
            SegmentPhraseVariant {
                words: &["that", "were", "put", "there", "this", "turn"],
                drain_start_offset: 0,
            },
        ],
    );
    true
}

pub(super) fn try_apply_graveyard_from_battlefield_this_turn_clause(
    filter: &mut ObjectFilter,
    all_words: &mut Vec<&str>,
    segment_tokens: &mut Vec<OwnedLexToken>,
) -> bool {
    let Some((word_start, consumed)) = find_filter_prefix_consumed(
        all_words.as_slice(),
        parse_graveyard_from_battlefield_this_turn_words,
    ) else {
        return false;
    };
    filter.entered_graveyard_from_battlefield_this_turn = true;
    all_words.drain(word_start + 1..word_start + consumed);
    drain_segment_phrase_variants(
        segment_tokens,
        &[
            SegmentPhraseVariant {
                words: &["graveyard", "from", "the", "battlefield", "this", "turn"],
                drain_start_offset: 1,
            },
            SegmentPhraseVariant {
                words: &["graveyard", "from", "battlefield", "this", "turn"],
                drain_start_offset: 1,
            },
            SegmentPhraseVariant {
                words: &["graveyards", "from", "the", "battlefield", "this", "turn"],
                drain_start_offset: 1,
            },
            SegmentPhraseVariant {
                words: &["graveyards", "from", "battlefield", "this", "turn"],
                drain_start_offset: 1,
            },
        ],
    );
    true
}

pub(super) fn try_apply_entered_battlefield_this_turn_clause(
    filter: &mut ObjectFilter,
    all_words: &mut Vec<&str>,
    segment_tokens: &mut Vec<OwnedLexToken>,
) -> bool {
    let Some((word_start, (controller, consumed))) =
        all_words.iter().enumerate().find_map(|(idx, _)| {
            parse_entered_battlefield_this_turn_words(&all_words[idx..])
                .map(|matched| (idx, matched))
        })
    else {
        return false;
    };
    filter.entered_battlefield_this_turn = true;
    filter.entered_battlefield_controller = controller;
    filter.zone = Some(Zone::Battlefield);
    all_words.drain(word_start..word_start + consumed);
    drain_segment_phrase_variants(
        segment_tokens,
        &[
            SegmentPhraseVariant {
                words: &[
                    "entered",
                    "the",
                    "battlefield",
                    "under",
                    "your",
                    "control",
                    "this",
                    "turn",
                ],
                drain_start_offset: 0,
            },
            SegmentPhraseVariant {
                words: &[
                    "entered",
                    "battlefield",
                    "under",
                    "your",
                    "control",
                    "this",
                    "turn",
                ],
                drain_start_offset: 0,
            },
            SegmentPhraseVariant {
                words: &["entered", "under", "your", "control", "this", "turn"],
                drain_start_offset: 0,
            },
            SegmentPhraseVariant {
                words: &[
                    "entered",
                    "the",
                    "battlefield",
                    "under",
                    "opponent",
                    "control",
                    "this",
                    "turn",
                ],
                drain_start_offset: 0,
            },
            SegmentPhraseVariant {
                words: &[
                    "entered",
                    "the",
                    "battlefield",
                    "under",
                    "opponents",
                    "control",
                    "this",
                    "turn",
                ],
                drain_start_offset: 0,
            },
            SegmentPhraseVariant {
                words: &[
                    "entered",
                    "battlefield",
                    "under",
                    "opponent",
                    "control",
                    "this",
                    "turn",
                ],
                drain_start_offset: 0,
            },
            SegmentPhraseVariant {
                words: &[
                    "entered",
                    "battlefield",
                    "under",
                    "opponents",
                    "control",
                    "this",
                    "turn",
                ],
                drain_start_offset: 0,
            },
            SegmentPhraseVariant {
                words: &["entered", "under", "opponent", "control", "this", "turn"],
                drain_start_offset: 0,
            },
            SegmentPhraseVariant {
                words: &["entered", "under", "opponents", "control", "this", "turn"],
                drain_start_offset: 0,
            },
            SegmentPhraseVariant {
                words: &["entered", "the", "battlefield", "this", "turn"],
                drain_start_offset: 0,
            },
            SegmentPhraseVariant {
                words: &["entered", "battlefield", "this", "turn"],
                drain_start_offset: 0,
            },
            SegmentPhraseVariant {
                words: &["entered", "this", "turn"],
                drain_start_offset: 0,
            },
        ],
    );
    true
}

pub(super) fn try_apply_didnt_enter_battlefield_this_turn_clause(
    filter: &mut ObjectFilter,
    all_words: &mut Vec<&str>,
    segment_tokens: &mut Vec<OwnedLexToken>,
) -> bool {
    let combined_match = DIDNT_ATTACK_OR_ENTER_THIS_TURN_PHRASES
        .iter()
        .find_map(|phrase| relation_phrase_word_span(all_words, phrase));
    let standalone_match = DIDNT_ENTER_BATTLEFIELD_THIS_TURN_PHRASES
        .iter()
        .find_map(|phrase| relation_phrase_word_span(all_words, phrase));
    let Some((word_start, word_end, also_didnt_attack)) = combined_match
        .map(|(start, end)| (start, end, true))
        .or_else(|| standalone_match.map(|(start, end)| (start, end, false)))
    else {
        return false;
    };

    filter.didnt_enter_battlefield_this_turn = true;
    filter.zone = Some(Zone::Battlefield);
    if also_didnt_attack {
        filter.didnt_attack_this_turn = true;
        filter.attacked_this_turn = false;
    }
    all_words.drain(word_start..word_end);
    drain_segment_phrase_variants(
        segment_tokens,
        &[
            SegmentPhraseVariant {
                words: &["didn't", "attack", "or", "enter", "this", "turn"],
                drain_start_offset: 0,
            },
            SegmentPhraseVariant {
                words: &["didnt", "attack", "or", "enter", "this", "turn"],
                drain_start_offset: 0,
            },
            SegmentPhraseVariant {
                words: &["did", "not", "attack", "or", "enter", "this", "turn"],
                drain_start_offset: 0,
            },
            SegmentPhraseVariant {
                words: &["didn't", "enter", "this", "turn"],
                drain_start_offset: 0,
            },
            SegmentPhraseVariant {
                words: &["didnt", "enter", "this", "turn"],
                drain_start_offset: 0,
            },
            SegmentPhraseVariant {
                words: &["did", "not", "enter", "this", "turn"],
                drain_start_offset: 0,
            },
            SegmentPhraseVariant {
                words: &["didn't", "enter", "the", "battlefield", "this", "turn"],
                drain_start_offset: 0,
            },
            SegmentPhraseVariant {
                words: &["didnt", "enter", "the", "battlefield", "this", "turn"],
                drain_start_offset: 0,
            },
            SegmentPhraseVariant {
                words: &["did", "not", "enter", "the", "battlefield", "this", "turn"],
                drain_start_offset: 0,
            },
        ],
    );
    true
}

/// Preserve the controller selected by a preceding player-or-planeswalker
/// target. The `planeswalker` noun belongs to the target reference, not to the
/// counted object filter, so consume the complete relation before the ordinary
/// type-union pass sees its embedded `or`.
pub(super) fn try_apply_target_player_or_planeswalker_controller_clause(
    filter: &mut ObjectFilter,
    all_words: &mut Vec<&str>,
    segment_tokens: &mut Vec<OwnedLexToken>,
) -> bool {
    let Some((word_start, word_end)) = TARGET_PLAYER_OR_PLANESWALKER_CONTROLLER_CONTROL_PHRASES
        .iter()
        .find_map(|phrase| relation_phrase_word_span(all_words, phrase))
    else {
        return false;
    };

    filter.controller = Some(PlayerFilter::TargetPlayerOrControllerOfTarget);
    all_words.drain(word_start..word_end);
    drain_segment_matching_phrase(
        segment_tokens,
        TARGET_PLAYER_OR_PLANESWALKER_CONTROLLER_CONTROL_PHRASES,
    );
    true
}

pub(super) fn try_apply_put_onto_battlefield_with_source_clause(
    filter: &mut ObjectFilter,
    all_words: &mut Vec<&str>,
    segment_tokens: &mut Vec<OwnedLexToken>,
) -> bool {
    let Some((word_start, phrase)) =
        PUT_ONTO_BATTLEFIELD_WITH_SOURCE_PHRASES
            .iter()
            .find_map(|phrase| {
                relation_phrase_word_span(all_words, phrase).map(|(start, _)| (start, *phrase))
            })
    else {
        return false;
    };

    filter.put_onto_battlefield_with_source = true;
    filter.put_onto_battlefield_with_source_surface =
        Some(crate::target::SourceReferenceSurface::ThisPermanentType(
            format!("this {}", phrase.last().copied().unwrap_or("permanent")),
        ));
    filter.zone = Some(Zone::Battlefield);
    all_words.drain(word_start..word_start + phrase.len());
    drain_segment_phrase_variants(
        segment_tokens,
        &[
            SegmentPhraseVariant {
                words: &[
                    "put",
                    "onto",
                    "the",
                    "battlefield",
                    "with",
                    "this",
                    "artifact",
                ],
                drain_start_offset: 0,
            },
            SegmentPhraseVariant {
                words: &[
                    "put",
                    "onto",
                    "the",
                    "battlefield",
                    "with",
                    "this",
                    "enchantment",
                ],
                drain_start_offset: 0,
            },
            SegmentPhraseVariant {
                words: &[
                    "put",
                    "onto",
                    "the",
                    "battlefield",
                    "with",
                    "this",
                    "permanent",
                ],
                drain_start_offset: 0,
            },
            SegmentPhraseVariant {
                words: &[
                    "put",
                    "onto",
                    "the",
                    "battlefield",
                    "with",
                    "this",
                    "source",
                ],
                drain_start_offset: 0,
            },
        ],
    );
    true
}

pub(super) fn try_apply_created_with_source_clause(
    filter: &mut ObjectFilter,
    all_words: &mut Vec<&str>,
    segment_tokens: &mut Vec<OwnedLexToken>,
) -> bool {
    let Some((word_start, phrase)) = CREATED_WITH_SOURCE_PHRASES.iter().find_map(|phrase| {
        relation_phrase_word_span(all_words, phrase).map(|(start, _)| (start, *phrase))
    }) else {
        return false;
    };

    let source_words = &phrase[2..];
    let source_surface = if source_words == ["it"] {
        Some(SourceReferenceSurface::ThisPermanentType("it".to_string()))
    } else {
        this_source_surface_for_words(source_words)
            .or_else(|| source_reference_surface_for_words(source_words))
    };
    let Some(source_surface) = source_surface else {
        return false;
    };

    filter.created_with_source = true;
    filter.created_with_source_surface = Some(source_surface);
    all_words.drain(word_start..word_start + phrase.len());
    drain_segment_phrase_variants(
        segment_tokens,
        &[
            SegmentPhraseVariant {
                words: &["created", "with", "it"],
                drain_start_offset: 0,
            },
            SegmentPhraseVariant {
                words: &["created", "with", "this", "artifact"],
                drain_start_offset: 0,
            },
            SegmentPhraseVariant {
                words: &["created", "with", "this", "creature"],
                drain_start_offset: 0,
            },
            SegmentPhraseVariant {
                words: &["created", "with", "this", "enchantment"],
                drain_start_offset: 0,
            },
            SegmentPhraseVariant {
                words: &["created", "with", "this", "permanent"],
                drain_start_offset: 0,
            },
            SegmentPhraseVariant {
                words: &["created", "with", "this", "source"],
                drain_start_offset: 0,
            },
        ],
    );
    true
}

pub(super) fn parse_drawn_this_turn_words(words: &[&str]) -> Option<usize> {
    parse_drawn_this_turn_shape(words)
}

pub(super) fn try_apply_drawn_this_turn_clause(
    filter: &mut ObjectFilter,
    all_words: &mut Vec<&str>,
    segment_tokens: &mut Vec<OwnedLexToken>,
) -> bool {
    let Some((word_start, consumed)) =
        find_filter_prefix_consumed(all_words.as_slice(), parse_drawn_this_turn_words)
    else {
        return false;
    };
    filter.drawn_this_turn = true;
    all_words.drain(word_start..word_start + consumed);
    drain_segment_phrase_variants(
        segment_tokens,
        &[SegmentPhraseVariant {
            words: &["drawn", "this", "turn"],
            drain_start_offset: 0,
        }],
    );
    true
}

pub(super) fn try_apply_was_dealt_damage_this_turn_clause(
    filter: &mut ObjectFilter,
    all_words: &mut Vec<&str>,
    segment_tokens: &mut Vec<OwnedLexToken>,
) -> bool {
    let Some((word_start, consumed)) = all_words.iter().enumerate().find_map(|(idx, _)| {
        let consumed = parse_was_dealt_damage_this_turn_shape(&all_words[idx..])?;
        // "that dealt damage this turn" is active voice and needs a distinct
        // history fact; do not flatten it into "was dealt damage" merely
        // because its suffix begins with the same words.
        if all_words[idx..].starts_with(&["dealt", "damage", "this", "turn"])
            && idx > 0
            && all_words[idx - 1] == "that"
        {
            return None;
        }
        Some((idx, consumed))
    }) else {
        return false;
    };

    filter.was_dealt_damage_this_turn = true;
    all_words.drain(word_start..word_start + consumed);
    drain_segment_phrase_variants(
        segment_tokens,
        &[
            SegmentPhraseVariant {
                words: &["that", "was", "dealt", "damage", "this", "turn"],
                drain_start_offset: 0,
            },
            SegmentPhraseVariant {
                words: &["that", "were", "dealt", "damage", "this", "turn"],
                drain_start_offset: 0,
            },
            SegmentPhraseVariant {
                words: &["was", "dealt", "damage", "this", "turn"],
                drain_start_offset: 0,
            },
            SegmentPhraseVariant {
                words: &["were", "dealt", "damage", "this", "turn"],
                drain_start_offset: 0,
            },
            SegmentPhraseVariant {
                words: &["dealt", "damage", "this", "turn"],
                drain_start_offset: 0,
            },
        ],
    );
    true
}

pub(super) fn push_it_tagged_object_constraint(filter: &mut ObjectFilter) {
    filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: TagKey::from(IT_TAG),
        relation: TaggedOpbjectRelation::IsTaggedObject,
    });
}

pub(super) fn try_apply_leading_tagged_reference_prefix(
    filter: &mut ObjectFilter,
    all_words: &mut Vec<&str>,
) -> bool {
    if all_words.len() >= 2 && LEADING_TAGGED_REFERENCE_WORDS.contains(&all_words[0]) {
        let plural_demonstrative = all_words[0] == "those";
        let demonstrative_reference = matches!(all_words[0], "that" | "those");
        let noun_idx = if all_words.get(1).is_some_and(|word| *word == "other") {
            2
        } else {
            1
        };
        if all_words
            .get(noun_idx)
            .is_some_and(|word| {
                is_demonstrative_object_head(word)
                    || (demonstrative_reference && parse_subtype_flexible(word).is_some())
            })
        {
            push_it_tagged_object_constraint(filter);
            if plural_demonstrative {
                filter.set_plural_object_noun_surface(true);
            }
            all_words.remove(0);
            return true;
        }
    }

    if all_words
        .first()
        .is_some_and(|word| IT_OR_THEM_WORDS.contains(word))
    {
        push_it_tagged_object_constraint(filter);
        all_words.remove(0);
        return true;
    }

    false
}

/// Bind authored chooser-relative object references to stable target-choice
/// aliases. A bare last-object tag is insufficient when two players choose
/// different targets before either one is referenced.
pub(super) fn try_apply_target_choice_attribution_reference(
    filter: &mut ObjectFilter,
    all_words: &mut Vec<&str>,
) -> bool {
    let (suffix, tag) = if all_words.ends_with(&["you", "chose"]) {
        (&["you", "chose"][..], ABILITY_CONTROLLER_TARGET_CHOICE_TAG)
    } else if all_words.ends_with(&["your", "opponent", "chose"]) {
        (
            &["your", "opponent", "chose"][..],
            OPPONENT_TARGET_CHOICE_TAG,
        )
    } else {
        return false;
    };
    let Some(noun) = all_words.get(all_words.len().saturating_sub(suffix.len() + 1)) else {
        return false;
    };
    if !is_demonstrative_object_head(noun) {
        return false;
    }
    filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: TagKey::from(tag),
        relation: TaggedOpbjectRelation::IsTaggedObject,
    });
    all_words.truncate(all_words.len() - suffix.len());
    true
}

pub(super) fn is_name_clause_boundary(word: &str) -> bool {
    matches!(
        word,
        "in" | "from"
            | "with"
            | "without"
            | "that"
            | "which"
            | "who"
            | "whose"
            | "under"
            | "among"
            | "on"
            | "you"
            | "your"
            | "opponent"
            | "opponents"
            | "their"
            | "its"
            | "controller"
            | "controllers"
            | "owner"
            | "owners"
    )
}

pub(super) fn find_name_clause_end(all_words: &[&str], name_start: usize) -> usize {
    let mut name_end = all_words.len();
    for idx in (name_start + 1)..all_words.len() {
        if is_name_clause_boundary(all_words[idx]) {
            name_end = idx;
            break;
        }
    }
    name_end
}

pub(super) fn extract_name_clause_text<'a, F, G>(
    all_words: &[&'a str],
    all_words_with_articles: &[&'a str],
    marker_idx: usize,
    marker_len: usize,
    map_non_article_index: &F,
    map_non_article_end: &G,
    error_label: &str,
) -> Result<(String, usize), CardTextError>
where
    F: Fn(usize) -> Option<usize>,
    G: Fn(usize) -> Option<usize>,
{
    let name_start = marker_idx + marker_len;
    let name_end = find_name_clause_end(all_words, name_start);
    let full_marker_idx = map_non_article_index(marker_idx).unwrap_or(marker_idx);
    let full_name_end = map_non_article_end(name_end).unwrap_or(name_end);
    let name_words = if full_marker_idx + marker_len <= full_name_end
        && full_name_end <= all_words_with_articles.len()
    {
        &all_words_with_articles[full_marker_idx + marker_len..full_name_end]
    } else {
        &all_words[name_start..name_end]
    };
    if name_words.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing card name in {error_label} object filter (clause: '{}')",
            all_words.join(" ")
        )));
    }

    Ok((name_words.join(" "), name_end))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_comparison_axes_with_consumed_words() {
        assert_eq!(
            parse_spell_filter_comparison_axis_words(&["power", "greater"]),
            Some((SpellFilterComparisonAxis::Power, 1))
        );
        assert_eq!(
            parse_spell_filter_comparison_axis_words(&["toughness", "less"]),
            Some((SpellFilterComparisonAxis::Toughness, 1))
        );
        assert_eq!(
            parse_spell_filter_comparison_axis_words(&["mana", "value", "equal"]),
            Some((SpellFilterComparisonAxis::ManaValue, 2))
        );
    }

    #[test]
    fn parses_player_relation_subjects_directly() {
        let pronoun = PlayerFilter::ChosenPlayer;
        for (words, expected, consumed) in [
            (
                &[
                    "that",
                    "opponent",
                    "or",
                    "that",
                    "planeswalkers",
                    "controller",
                    "controls",
                ][..],
                PlayerFilter::TargetPlayerOrControllerOfTarget,
                6,
            ),
            (
                &[
                    "that",
                    "player",
                    "or",
                    "that",
                    "planeswalkers",
                    "controller",
                    "controls",
                ][..],
                PlayerFilter::TargetPlayerOrControllerOfTarget,
                6,
            ),
            (
                &["your", "team", "controls"][..],
                PlayerFilter::your_team(),
                2,
            ),
            (
                &["your", "opponents", "control"][..],
                PlayerFilter::Opponent,
                2,
            ),
            (
                &["that", "player", "owns"][..],
                PlayerFilter::IteratedPlayer,
                2,
            ),
            (
                &["target", "opponent", "controls"][..],
                PlayerFilter::target_opponent(),
                2,
            ),
            (
                &["those", "opponents", "control"][..],
                PlayerFilter::AliasedTarget(Box::new(PlayerFilter::Opponent)),
                2,
            ),
            (
                &["attacking", "player", "controls"][..],
                PlayerFilter::Attacking,
                2,
            ),
            (
                &["their", "controllers", "control"][..],
                PlayerFilter::ControllerOf(crate::filter::ObjectRef::Target),
                2,
            ),
            (&["they", "control"][..], pronoun.clone(), 1),
            (&["voter", "owns"][..], PlayerFilter::IteratedPlayer, 1),
        ] {
            assert_eq!(
                parse_player_relation_subject(words, &pronoun),
                Some((expected, consumed))
            );
        }
    }

    #[test]
    fn player_or_planeswalker_controller_reference_does_not_expand_counted_object_types() {
        for subject in [
            "that opponent or that planeswalker's controller",
            "that player or that planeswalker's controller",
        ] {
            let tokens =
                crate::runtime_backend::lex_line(&format!("creatures {subject} controls"), 0)
                    .expect("controller-relative object filter should lex");
            let filter = parse_object_filter_with_grammar_entrypoint_lexed(&tokens, false)
                .expect("controller-relative object filter should parse");

            assert_eq!(filter.card_types, vec![CardType::Creature], "{subject}");
            assert_eq!(
                filter.controller,
                Some(PlayerFilter::TargetPlayerOrControllerOfTarget),
                "{subject}"
            );
            assert_eq!(filter.zone, Some(Zone::Battlefield), "{subject}");
            assert!(
                !filter.card_types.contains(&CardType::Planeswalker),
                "{subject}: {filter:#?}"
            );
        }
    }

    #[test]
    fn applies_passive_voter_owner_relation() {
        let mut filter = ObjectFilter::default();
        assert_eq!(
            try_apply_passive_player_relation_clause(
                &mut filter,
                &["owned", "by", "voter", "tail"],
                &PlayerFilter::Any,
            ),
            Some(3)
        );
        assert_eq!(filter.owner, Some(PlayerFilter::IteratedPlayer));
    }

    #[test]
    fn applies_negated_you_relations() {
        let mut control_filter = ObjectFilter::default();
        assert_eq!(
            try_apply_negated_you_relation_clause(
                &mut control_filter,
                &["you", "do", "not", "control", "creatures"],
                &PlayerFilter::IteratedPlayer,
            ),
            Some(4)
        );
        assert_eq!(control_filter.controller, Some(PlayerFilter::NotYou));

        let mut owner_filter = ObjectFilter::default();
        assert_eq!(
            try_apply_negated_you_relation_clause(
                &mut owner_filter,
                &["don't", "owns", "cards"],
                &PlayerFilter::IteratedPlayer,
            ),
            Some(2)
        );
        assert_eq!(owner_filter.owner, Some(PlayerFilter::NotYou));

        let mut participant_filter = ObjectFilter::default();
        assert_eq!(
            try_apply_negated_you_relation_clause(
                &mut participant_filter,
                &["they", "don't", "control", "permanents"],
                &PlayerFilter::IteratedPlayer,
            ),
            Some(3)
        );
        assert_eq!(
            participant_filter.controller,
            Some(PlayerFilter::excluding(
                PlayerFilter::Any,
                PlayerFilter::IteratedPlayer,
            ))
        );
    }

    #[test]
    fn applies_chosen_player_graveyard_fact() {
        let mut filter = ObjectFilter::default();
        assert_eq!(
            try_apply_chosen_player_graveyard_clause(
                &mut filter,
                &["the", "chosen", "players", "graveyard", "cards"]
            ),
            Some(4)
        );
        assert_eq!(filter.owner, Some(PlayerFilter::ChosenPlayer));
        assert_eq!(filter.zone, Some(Zone::Graveyard));
    }

    #[test]
    fn parses_joint_and_disjunctive_owner_controller_relations() {
        let mut filter = ObjectFilter::default();
        assert_eq!(
            try_apply_joint_owner_controller_clause(
                &mut filter,
                &["you", "both", "own", "and", "controls", "cards"],
                &PlayerFilter::Any,
            ),
            Some(5)
        );
        assert_eq!(filter.owner, Some(PlayerFilter::You));
        assert_eq!(filter.controller, Some(PlayerFilter::You));

        assert_eq!(
            parse_owner_or_controller_disjunction_player(
                &["opponents", "control", "or", "owns", "cards"],
                &PlayerFilter::Any,
            ),
            Some((PlayerFilter::Opponent, 4))
        );
        assert_eq!(
            parse_owner_or_controller_disjunction_player(
                &["you", "own", "or", "owns", "cards"],
                &PlayerFilter::Any,
            ),
            None
        );
    }

    #[test]
    fn parses_entered_battlefield_variants() {
        for (words, expected_controller, expected_consumed) in [
            (
                &[
                    "entered",
                    "the",
                    "battlefield",
                    "under",
                    "your",
                    "control",
                    "this",
                    "turn",
                    "tail",
                ][..],
                Some(PlayerFilter::You),
                8,
            ),
            (
                &[
                    "entered",
                    "battlefield",
                    "under",
                    "opponent",
                    "control",
                    "this",
                    "turn",
                ][..],
                Some(PlayerFilter::Opponent),
                7,
            ),
            (
                &["entered", "under", "opponents", "control", "this", "turn"][..],
                Some(PlayerFilter::Opponent),
                6,
            ),
            (
                &["entered", "the", "battlefield", "this", "turn"][..],
                None,
                5,
            ),
            (&["entered", "battlefield", "this", "turn"][..], None, 4),
            (&["entered", "this", "turn"][..], None, 3),
        ] {
            assert_eq!(
                parse_entered_battlefield_this_turn_words(words),
                Some((expected_controller, expected_consumed))
            );
        }
    }

    #[test]
    fn parses_graveyard_and_drawn_turn_events() {
        assert_eq!(
            parse_put_there_from_battlefield_this_turn_words(&[
                "that",
                "were",
                "put",
                "there",
                "from",
                "battlefield",
                "this",
                "turn",
                "tail",
            ]),
            Some(8)
        );
        assert_eq!(
            parse_put_there_from_anywhere_this_turn_words(&[
                "that", "was", "put", "there", "from", "anywhere", "this", "turn",
            ]),
            Some(8)
        );
        assert_eq!(
            parse_graveyard_from_battlefield_this_turn_words(&[
                "graveyards",
                "from",
                "battlefield",
                "this",
                "turn",
            ]),
            Some(5)
        );
        assert_eq!(
            parse_drawn_this_turn_words(&["drawn", "this", "turn", "tail"]),
            Some(3)
        );
        assert_eq!(
            parse_drawn_this_turn_words(&["drawn", "last", "turn"]),
            None
        );
    }

    #[test]
    fn applies_was_dealt_damage_history_without_conflating_active_voice() {
        for mut words in [
            vec![
                "target", "creature", "that", "was", "dealt", "damage", "this", "turn",
            ],
            vec!["each", "creature", "dealt", "damage", "this", "turn"],
        ] {
            let mut filter = ObjectFilter::default();
            let mut tokens = Vec::new();
            assert!(try_apply_was_dealt_damage_this_turn_clause(
                &mut filter,
                &mut words,
                &mut tokens,
            ));
            assert!(filter.was_dealt_damage_this_turn);
        }

        let mut active_words = vec![
            "target", "creature", "that", "dealt", "damage", "this", "turn",
        ];
        let mut active_filter = ObjectFilter::default();
        let mut tokens = Vec::new();
        assert!(!try_apply_was_dealt_damage_this_turn_clause(
            &mut active_filter,
            &mut active_words,
            &mut tokens,
        ));
        assert!(!active_filter.was_dealt_damage_this_turn);
    }

    #[test]
    fn target_choice_references_retain_the_authored_chooser() {
        for (mut words, expected_tag) in [
            (
                vec!["creature", "you", "chose"],
                ABILITY_CONTROLLER_TARGET_CHOICE_TAG,
            ),
            (
                vec!["creature", "your", "opponent", "chose"],
                OPPONENT_TARGET_CHOICE_TAG,
            ),
        ] {
            let mut filter = ObjectFilter::default();
            assert!(try_apply_target_choice_attribution_reference(
                &mut filter,
                &mut words,
            ));
            assert_eq!(words, ["creature"]);
            assert!(filter.tagged_constraints.iter().any(|constraint| {
                constraint.relation == TaggedOpbjectRelation::IsTaggedObject
                    && constraint.tag.as_str() == expected_tag
            }));
        }
    }

    #[test]
    fn plural_demonstrative_reference_preserves_plural_noun_surface() {
        let mut words = vec!["those", "creature", "cards"];
        let mut filter = ObjectFilter::default();

        assert!(try_apply_leading_tagged_reference_prefix(
            &mut filter,
            &mut words,
        ));
        assert_eq!(words, ["creature", "cards"]);
        assert!(filter.has_plural_object_noun_surface());
        assert!(filter.tagged_constraints.iter().any(|constraint| {
            constraint.relation == TaggedOpbjectRelation::IsTaggedObject
                && constraint.tag.as_str() == IT_TAG
        }));
    }
}
