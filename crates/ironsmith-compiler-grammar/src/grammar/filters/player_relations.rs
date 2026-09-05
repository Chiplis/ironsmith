use super::*;
use crate::target::CountersPutOnThisTurnConstraint;
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
            relation_phrase(&["another", "target", "player"]).map(|()| {
                PlayerFilter::Target(Box::new(PlayerFilter::excluding(
                    PlayerFilter::Any,
                    pronoun_player_filter.clone(),
                )))
            }),
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
        )),
        alt((
            relation_phrase(&["its", "controllers"])
                .map(|()| PlayerFilter::ControllerOf(crate::filter::ObjectRef::Target)),
            relation_phrase(&["enchanted", "player"]).map(|()| {
                PlayerFilter::TaggedPlayer(crate::tag::CompilerReferenceTag::Enchanted.bind())
            }),
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
    let axis = crate::grammar::primitives::take_leaf(&mut input, parse_relation_axis_word_slice)?;
    Some((axis, words.len().saturating_sub(input.len())))
}

fn parse_relation_verb_shape(words: &[&str]) -> Option<(PlayerRelationVerb, usize)> {
    let mut input: primitives::WordSliceInput<'_> = words;
    let verb = crate::grammar::primitives::take_leaf(&mut input, parse_relation_verb_word_slice)?;
    Some((verb, words.len().saturating_sub(input.len())))
}

fn parse_passive_relation_verb_shape(words: &[&str]) -> Option<(PlayerRelationVerb, usize)> {
    let mut input: primitives::WordSliceInput<'_> = words;
    let verb =
        crate::grammar::primitives::take_leaf(&mut input, parse_passive_relation_verb_word_slice)?;
    Some((verb, words.len().saturating_sub(input.len())))
}

fn parse_relation_subject_shape(
    words: &[&str],
    pronoun_player_filter: &PlayerFilter,
) -> Option<(PlayerFilter, usize)> {
    let mut input: primitives::WordSliceInput<'_> = words;
    let player = crate::grammar::primitives::take_leaf(&mut input, |input: &mut _| {
        parse_relation_subject_word_slice(input, pronoun_player_filter)
    })?;
    Some((player, words.len().saturating_sub(input.len())))
}

fn parse_negated_you_relation_shape(words: &[&str]) -> Option<(PlayerRelationVerb, usize)> {
    let mut input: primitives::WordSliceInput<'_> = words;
    let verb =
        crate::grammar::primitives::take_leaf(&mut input, parse_negated_you_relation_word_slice)?;
    Some((verb, words.len().saturating_sub(input.len())))
}

fn parse_chosen_player_graveyard_shape(words: &[&str]) -> Option<(PlayerFilter, Zone, usize)> {
    let mut input: primitives::WordSliceInput<'_> = words;
    let (owner, zone) = crate::grammar::primitives::take_leaf(
        &mut input,
        parse_chosen_player_graveyard_word_slice,
    )?;
    Some((owner, zone, words.len().saturating_sub(input.len())))
}

fn parse_joint_owner_controller_shape(words: &[&str]) -> Option<usize> {
    let mut input: primitives::WordSliceInput<'_> = words;
    crate::grammar::primitives::take_leaf(&mut input, |input: &mut _| {
        parse_owner_controller_pair_word_slice(input, "and", true)
    })?;
    Some(words.len().saturating_sub(input.len()))
}

fn parse_owner_or_controller_shape(words: &[&str]) -> Option<usize> {
    let mut input: primitives::WordSliceInput<'_> = words;
    crate::grammar::primitives::take_leaf(&mut input, |input: &mut _| {
        parse_owner_controller_pair_word_slice(input, "or", false)
    })?;
    Some(words.len().saturating_sub(input.len()))
}

fn parse_put_there_from_battlefield_this_turn_shape(words: &[&str]) -> Option<usize> {
    let mut input: primitives::WordSliceInput<'_> = words;
    crate::grammar::primitives::take_leaf(
        &mut input,
        parse_put_there_from_battlefield_this_turn_word_slice,
    )?;
    Some(words.len().saturating_sub(input.len()))
}

fn parse_put_there_from_anywhere_this_turn_shape(words: &[&str]) -> Option<usize> {
    let mut input: primitives::WordSliceInput<'_> = words;
    crate::grammar::primitives::take_leaf(
        &mut input,
        parse_put_there_from_anywhere_this_turn_word_slice,
    )?;
    Some(words.len().saturating_sub(input.len()))
}

fn parse_put_there_from_their_library_this_turn_shape(words: &[&str]) -> Option<usize> {
    const PHRASES: &[&[&str]] = &[
        &[
            "that", "was", "put", "there", "from", "their", "library", "this", "turn",
        ],
        &[
            "that", "were", "put", "there", "from", "their", "library", "this", "turn",
        ],
    ];
    crate::word_primitives::find_any_phrase_start(words, PHRASES)
        .filter(|(_, start)| *start == 0)
        .map(|(phrase, _)| phrase.len())
}

fn parse_graveyard_from_battlefield_this_turn_shape(words: &[&str]) -> Option<usize> {
    let mut input: primitives::WordSliceInput<'_> = words;
    crate::grammar::primitives::take_leaf(
        &mut input,
        parse_graveyard_from_battlefield_this_turn_word_slice,
    )?;
    Some(words.len().saturating_sub(input.len()))
}

fn parse_entered_battlefield_this_turn_shape(
    words: &[&str],
) -> Option<(Option<PlayerFilter>, usize)> {
    let mut input: primitives::WordSliceInput<'_> = words;
    let controller = crate::grammar::primitives::take_leaf(
        &mut input,
        parse_entered_battlefield_this_turn_word_slice,
    )?;
    Some((controller, words.len().saturating_sub(input.len())))
}

fn parse_drawn_this_turn_shape(words: &[&str]) -> Option<usize> {
    let mut input: primitives::WordSliceInput<'_> = words;
    crate::grammar::primitives::take_leaf(&mut input, parse_drawn_this_turn_word_slice)?;
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
    crate::word_primitives::find_any_phrase_start(words, PHRASES)
        .filter(|(_, start)| *start == 0)
        .map(|(phrase, _)| phrase.len())
}

fn parse_put_there_this_turn_shape(words: &[&str]) -> Option<usize> {
    const PHRASES: &[&[&str]] = &[
        &["that", "was", "put", "there", "this", "turn"],
        &["that", "were", "put", "there", "this", "turn"],
    ];
    crate::word_primitives::find_any_phrase_start(words, PHRASES)
        .filter(|(_, start)| *start == 0)
        .map(|(phrase, _)| phrase.len())
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

/// Apply an authored joint negative relation such as "you neither own nor
/// control". Both independent predicates remain executable; this parser only
/// keeps the shared negation from being mistaken for an unsupported noun
/// suffix.
pub(super) fn try_apply_neither_owned_nor_controlled_clause(
    filter: &mut ObjectFilter,
    words: &[&str],
) -> Option<usize> {
    let relation = words.get(..5)?;
    if !matches!(
        relation,
        ["you", "neither", "own", "nor", "control"] | ["you", "neither", "control", "nor", "own"]
    ) {
        return None;
    }
    filter.owner = Some(PlayerFilter::NotYou);
    filter.controller = Some(PlayerFilter::NotYou);
    Some(5)
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

pub(super) fn parse_put_there_from_their_library_this_turn_words(words: &[&str]) -> Option<usize> {
    parse_put_there_from_their_library_this_turn_shape(words)
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

pub(super) fn try_apply_put_there_from_their_library_this_turn_clause(
    filter: &mut ObjectFilter,
    all_words: &mut Vec<&str>,
    segment_tokens: &mut Vec<OwnedLexToken>,
) -> bool {
    let Some((word_start, consumed)) = find_filter_prefix_consumed(
        all_words.as_slice(),
        parse_put_there_from_their_library_this_turn_words,
    ) else {
        return false;
    };
    filter.entered_graveyard_this_turn = true;
    filter.entered_graveyard_from_library_this_turn = true;
    all_words.drain(word_start..word_start + consumed);
    drain_segment_phrase_variants(
        segment_tokens,
        &[
            SegmentPhraseVariant {
                words: &[
                    "that", "was", "put", "there", "from", "their", "library", "this", "turn",
                ],
                drain_start_offset: 0,
            },
            SegmentPhraseVariant {
                words: &[
                    "that", "were", "put", "there", "from", "their", "library", "this", "turn",
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
    let explicitly_named_battlefield = crate::word_primitives::sequence_occurs(
        &all_words[word_start..word_start + consumed],
        &["battlefield"],
    );
    filter.entered_battlefield_this_turn = true;
    filter.entered_battlefield_controller = controller;
    filter.zone = Some(Zone::Battlefield);
    filter.set_entered_battlefield_explicit_surface(explicitly_named_battlefield);
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
    let source_surface = if crate::word_primitives::parse_sequence_complete(source_words, &["it"]) {
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

fn parse_counters_put_on_this_turn_words(
    words: &[&str],
) -> Option<(CountersPutOnThisTurnConstraint, usize)> {
    if words.len() < 8
        || words.first().copied() != Some("that")
        || !matches!(words.get(1).copied(), Some("youve" | "you've" | "you’ve"))
        || words.get(2).copied() != Some("put")
    {
        return None;
    }

    let mut descriptor_start = 3usize;
    let minimum = if words.get(descriptor_start..).is_some_and(|tail| {
        crate::word_primitives::parse_sequence_prefix(tail, &["one", "or", "more"])
    }) {
        descriptor_start += 3;
        1
    } else if words.get(descriptor_start + 1).copied() == Some("or")
        && words.get(descriptor_start + 2).copied() == Some("more")
    {
        let minimum = parse_number_word_u32(words[descriptor_start])?;
        descriptor_start += 3;
        minimum
    } else {
        1
    };

    let counter_noun =
        crate::slice_primitives::select_position(&words[descriptor_start..], |word| {
            matches!(*word, "counter" | "counters")
        })? + descriptor_start;
    if !words.get(counter_noun + 1..).is_some_and(|tail| {
        crate::word_primitives::parse_sequence_prefix(tail, &["on", "this", "turn"])
    }) {
        return None;
    }

    let counter_words = &words[descriptor_start..=counter_noun];
    let counter_type = if counter_words.len() == 1 {
        None
    } else {
        Some(parse_counter_type_words(counter_words)?)
    };
    Some((
        CountersPutOnThisTurnConstraint::new(counter_type, PlayerFilter::You, minimum),
        counter_noun + 4,
    ))
}

pub(super) fn try_apply_counters_put_on_this_turn_clause(
    filter: &mut ObjectFilter,
    all_words: &mut Vec<&str>,
    segment_tokens: &mut Vec<OwnedLexToken>,
) -> bool {
    let Some((word_start, (constraint, consumed))) =
        all_words.iter().enumerate().find_map(|(idx, _)| {
            parse_counters_put_on_this_turn_words(&all_words[idx..]).map(|matched| (idx, matched))
        })
    else {
        return false;
    };

    let phrase_words = all_words[word_start..word_start + consumed].to_vec();
    filter.counters_put_on_this_turn = Some(constraint);
    all_words.drain(word_start..word_start + consumed);
    drain_segment_matching_phrase(segment_tokens, &[phrase_words.as_slice()]);
    true
}

/// Active voice: "target creature that dealt damage this turn" — the object
/// is the damage DEALER, not the recipient.
pub(super) fn try_apply_dealt_damage_this_turn_clause(
    filter: &mut ObjectFilter,
    all_words: &mut Vec<&str>,
    segment_tokens: &mut Vec<OwnedLexToken>,
) -> bool {
    const ACTIVE_PHRASE: &[&str] = &["that", "dealt", "damage", "this", "turn"];
    let Some(word_start) = crate::word_primitives::parse_sequence_start(all_words, ACTIVE_PHRASE)
    else {
        return false;
    };

    filter.dealt_damage_this_turn = true;
    all_words.drain(word_start..word_start + ACTIVE_PHRASE.len());
    drain_segment_phrase_variants(
        segment_tokens,
        &[SegmentPhraseVariant {
            words: ACTIVE_PHRASE,
            drain_start_offset: 0,
        }],
    );
    true
}

pub(super) fn try_apply_ability_activated_this_turn_clause(
    filter: &mut ObjectFilter,
    all_words: &mut Vec<&str>,
    segment_tokens: &mut Vec<OwnedLexToken>,
) -> bool {
    const PHRASES: &[SegmentPhraseVariant] = &[
        SegmentPhraseVariant {
            words: &["that", "was", "activated", "this", "turn"],
            drain_start_offset: 0,
        },
        SegmentPhraseVariant {
            words: &["that", "had", "an", "ability", "activated", "this", "turn"],
            drain_start_offset: 0,
        },
    ];
    let phrases = PHRASES
        .iter()
        .map(|variant| variant.words)
        .collect::<Vec<_>>();
    let Some((phrase, word_start)) =
        crate::word_primitives::find_any_phrase_start(all_words, &phrases)
    else {
        return false;
    };
    let consumed = phrase.len();

    filter.ability_activated_this_turn = true;
    all_words.drain(word_start..word_start + consumed);
    drain_segment_phrase_variants(segment_tokens, PHRASES);
    true
}

pub(super) fn try_apply_not_enchanted_clause(
    filter: &mut ObjectFilter,
    all_words: &mut Vec<&str>,
    segment_tokens: &mut Vec<OwnedLexToken>,
) -> bool {
    const PHRASES: &[SegmentPhraseVariant] = &[
        SegmentPhraseVariant {
            words: &["that", "aren't", "enchanted"],
            drain_start_offset: 0,
        },
        SegmentPhraseVariant {
            words: &["that", "arent", "enchanted"],
            drain_start_offset: 0,
        },
        SegmentPhraseVariant {
            words: &["that", "isn't", "enchanted"],
            drain_start_offset: 0,
        },
        SegmentPhraseVariant {
            words: &["that", "isnt", "enchanted"],
            drain_start_offset: 0,
        },
        SegmentPhraseVariant {
            words: &["that", "are", "not", "enchanted"],
            drain_start_offset: 0,
        },
    ];
    let phrases = PHRASES
        .iter()
        .map(|variant| variant.words)
        .collect::<Vec<_>>();
    let Some((phrase, word_start)) =
        crate::word_primitives::find_any_phrase_start(all_words, &phrases)
    else {
        return false;
    };
    let consumed = phrase.len();

    let mut aura = ObjectFilter::enchantment();
    aura.subtypes.push(Subtype::Aura);
    filter.without_attached_object = Some(Box::new(aura));
    all_words.drain(word_start..word_start + consumed);
    drain_segment_phrase_variants(segment_tokens, PHRASES);
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
        if crate::word_primitives::parse_sequence_prefix(
            &all_words[idx..],
            &["dealt", "damage", "this", "turn"],
        ) && idx > 0
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
        tag: crate::tag::CompilerReferenceTag::It.bind(),
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
        if all_words.get(noun_idx).is_some_and(|word| {
            is_demonstrative_object_head(word)
                || (demonstrative_reference && parse_subtype_flexible(word).is_some())
        }) {
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
    let (suffix, tag) =
        if crate::word_primitives::parse_sequence_suffix(all_words, &["you", "chose"]) {
            (
                &["you", "chose"][..],
                crate::tag::CompilerReferenceTag::AbilityControllerTargetChoice,
            )
        } else if crate::word_primitives::parse_sequence_suffix(
            all_words,
            &["your", "opponent", "chose"],
        ) {
            (
                &["your", "opponent", "chose"][..],
                crate::tag::CompilerReferenceTag::OpponentTargetChoice,
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
        tag: tag.bind(),
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
    for (idx, word) in all_words.iter().enumerate().skip(name_start + 1) {
        if is_name_clause_boundary(word) {
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
#[path = "player_relations_inline_tests.rs"]
mod tests;
