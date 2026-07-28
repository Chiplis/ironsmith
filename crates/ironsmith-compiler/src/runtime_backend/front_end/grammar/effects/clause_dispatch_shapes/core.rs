use super::super::*;

use crate::effect::Until;
use crate::runtime_backend::front_end::grammar::leaf;
use crate::runtime_backend::front_end::lexer::TokenWordView;
use winnow::combinator::{alt, repeat};
use winnow::error::ModalResult as WResult;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ClauseSubjectVerbShape<'a> {
    pub(crate) kind: chain_splitting::ChainVerbKind,
    pub(crate) subject_tokens: &'a [OwnedLexToken],
    pub(crate) action_tokens: &'a [OwnedLexToken],
}

pub(crate) fn parse_clause_subject_verb_shape(
    tokens: &[OwnedLexToken],
) -> Option<ClauseSubjectVerbShape<'_>> {
    let found = chain_splitting::find_chain_verb_tokens(tokens)?;
    let words = TokenWordView::new(tokens);
    let verb_range = words.token_span_for_words(found.word_index, found.word_index + 1)?;
    Some(ClauseSubjectVerbShape {
        kind: found.kind,
        subject_tokens: tokens.get(..verb_range.start)?,
        action_tokens: trim_lexed_commas(tokens.get(verb_range.end..)?),
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LeadingMayActorShape {
    Player(PlayerAst),
    Implicit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LeadingMayClauseShape<'a> {
    pub(crate) actor: LeadingMayActorShape,
    pub(crate) effect_tokens: &'a [OwnedLexToken],
}

fn player_word<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((primitives::kw("player"), primitives::kw("players")))
        .void()
        .parse_next(input)
}

fn opponent_word<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((primitives::kw("opponent"), primitives::kw("opponents")))
        .void()
        .parse_next(input)
}

fn controller_subject_word<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    primitives::any_phrase(&[
        &["creature"],
        &["creatures"],
        &["permanent"],
        &["permanents"],
        &["planeswalker"],
        &["planeswalkers"],
        &["source"],
        &["sources"],
        &["spell"],
        &["spells"],
    ])
    .void()
    .parse_next(input)
}

fn may_actor<'a>(input: &mut LexStream<'a>) -> WResult<LeadingMayActorShape> {
    repeat::<_, _, (), _, _>(
        0..,
        alt((primitives::kw("then"), primitives::kw("and"))).void(),
    )
    .parse_next(input)?;
    alt((
        alt((
            primitives::phrase(&["you", "may"]).value(LeadingMayActorShape::Player(PlayerAst::You)),
            (primitives::kw("any"), player_word, primitives::kw("may"))
                .value(LeadingMayActorShape::Player(PlayerAst::Any)),
            (primitives::kw("any"), opponent_word, primitives::kw("may"))
                .value(LeadingMayActorShape::Player(PlayerAst::Opponent)),
            (
                primitives::kw("target"),
                opponent_word,
                primitives::kw("may"),
            )
                .value(LeadingMayActorShape::Player(PlayerAst::TargetOpponent)),
            (primitives::kw("target"), player_word, primitives::kw("may"))
                .value(LeadingMayActorShape::Player(PlayerAst::Target)),
        )),
        alt((
            (primitives::kw("that"), player_word, primitives::kw("may"))
                .value(LeadingMayActorShape::Player(PlayerAst::That)),
            (primitives::kw("that"), opponent_word, primitives::kw("may"))
                .value(LeadingMayActorShape::Player(PlayerAst::That)),
            primitives::phrase(&["they", "may"])
                .value(LeadingMayActorShape::Player(PlayerAst::That)),
            (
                primitives::phrase(&["that", "player", "or", "that"]),
                controller_subject_word,
                primitives::phrase(&["controller", "may"]),
            )
                .value(LeadingMayActorShape::Player(
                    PlayerAst::ThatPlayerOrTargetController,
                )),
        )),
        alt((
            (
                primitives::kw("that"),
                controller_subject_word,
                primitives::phrase(&["controller", "may"]),
            )
                .value(LeadingMayActorShape::Player(PlayerAst::ItsController)),
            (
                primitives::kw("that"),
                controller_subject_word,
                primitives::phrase(&["owner", "may"]),
            )
                .value(LeadingMayActorShape::Player(PlayerAst::ItsOwner)),
            primitives::phrase(&["the", "player", "whose", "turn", "it", "is", "may"])
                .value(LeadingMayActorShape::Player(PlayerAst::Active)),
            (primitives::kw("the"), player_word, primitives::kw("may"))
                .value(LeadingMayActorShape::Player(PlayerAst::That)),
            primitives::phrase(&["defending", "player", "may"])
                .value(LeadingMayActorShape::Player(PlayerAst::Defending)),
            primitives::any_phrase(&[
                &["attacking", "player", "may"],
                &["that", "attacking", "player", "may"],
                &["the", "attacking", "player", "may"],
            ])
            .value(LeadingMayActorShape::Player(PlayerAst::Attacking)),
        )),
        alt((
            (
                alt((primitives::kw("its"), primitives::kw("their"))),
                primitives::phrase(&["controller", "may"]),
            )
                .value(LeadingMayActorShape::Player(PlayerAst::ItsController)),
            (
                alt((primitives::kw("its"), primitives::kw("their"))),
                primitives::phrase(&["owner", "may"]),
            )
                .value(LeadingMayActorShape::Player(PlayerAst::ItsOwner)),
            (opponent_word, primitives::kw("may"))
                .value(LeadingMayActorShape::Player(PlayerAst::Opponent)),
            primitives::phrase(&["an", "opponent", "may"])
                .value(LeadingMayActorShape::Player(PlayerAst::Opponent)),
            primitives::kw("may").value(LeadingMayActorShape::Implicit),
        )),
    ))
    .parse_next(input)
}

fn causative_player_subject<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    (
        alt((primitives::kw("have"), primitives::kw("has"))),
        primitives::any_phrase(&[
            &["that", "player"],
            &["that", "players"],
            &["that", "opponent"],
            &["that", "opponents"],
            &["each", "player"],
            &["each", "opponent"],
            &["those", "players"],
            &["those", "opponents"],
            &["target", "player"],
            &["target", "opponent"],
            &["another", "player"],
            &["another", "opponent"],
        ]),
    )
        .void()
        .parse_next(input)
}

pub(crate) fn parse_leading_may_clause_shape(
    tokens: &[OwnedLexToken],
) -> Option<LeadingMayClauseShape<'_>> {
    let (actor, effect_tokens) = primitives::parse_prefix(tokens, may_actor)?;
    let effect_tokens =
        if primitives::parse_prefix(effect_tokens, causative_player_subject).is_some() {
            effect_tokens
        } else {
            primitives::parse_prefix(
                effect_tokens,
                alt((primitives::kw("have"), primitives::kw("has"))),
            )
            .map(|(_, rest)| rest)
            .unwrap_or(effect_tokens)
        };
    (!effect_tokens.is_empty()).then_some(LeadingMayClauseShape {
        actor,
        effect_tokens: trim_lexed_commas(effect_tokens),
    })
}

fn map_pump_duration(duration: leaf::LeafDurationPhrase) -> Option<Until> {
    match duration {
        leaf::LeafDurationPhrase::UntilEndOfTurn => Some(Until::EndOfTurn),
        leaf::LeafDurationPhrase::UntilYourNextTurn => Some(Until::YourNextTurn),
        leaf::LeafDurationPhrase::UntilEndOfCombat => Some(Until::EndOfCombat),
        _ => None,
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum PumpSubjectKind<'a> {
    Tagged,
    DemonstrativeTarget,
    ControlledFilter {
        filter_tokens: &'a [OwnedLexToken],
        controller: PlayerFilter,
    },
    DirectTarget(&'a [OwnedLexToken]),
    Equipped,
    Enchanted,
    FilterCandidate {
        filter_tokens: &'a [OwnedLexToken],
        mentions_this: bool,
        disallowed_pronoun: bool,
        demonstrative_reference: bool,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PumpSubjectShape<'a> {
    pub(crate) subject_tokens: &'a [OwnedLexToken],
    pub(crate) duration: Option<Until>,
    pub(crate) kind: PumpSubjectKind<'a>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TaggedPluralPumpShape<'a> {
    pub(crate) subject_tokens: &'a [OwnedLexToken],
    pub(crate) modifier_tokens: &'a [OwnedLexToken],
}

pub(crate) fn parse_tagged_plural_pump_shape(
    tokens: &[OwnedLexToken],
) -> Option<TaggedPluralPumpShape<'_>> {
    let (_, after_subject) = primitives::parse_prefix(
        tokens,
        primitives::any_phrase(&[&["they", "each"], &["them", "each"]]).void(),
    )?;
    let subject_end = tokens.len().checked_sub(after_subject.len())?;
    let (_, modifier_tokens) = primitives::parse_prefix(
        after_subject,
        alt((primitives::kw("get"), primitives::kw("gets"))).void(),
    )?;
    let modifier_tokens = trim_lexed_commas(modifier_tokens);
    (!modifier_tokens.is_empty()).then_some(TaggedPluralPumpShape {
        subject_tokens: &tokens[..subject_end],
        modifier_tokens,
    })
}

fn exact<'a, P>(tokens: &'a [OwnedLexToken], parser: P) -> bool
where
    P: Parser<LexStream<'a>, (), winnow::error::ErrMode<winnow::error::ContextError>>,
{
    primitives::parse_all(
        tokens,
        (parser, primitives::sentence_end()).void(),
        "dispatch exact",
    )
    .is_ok()
}

pub(crate) fn parse_pump_subject_shape(tokens: &[OwnedLexToken]) -> Option<PumpSubjectShape<'_>> {
    let (subject_tokens, duration) = leaf::parse_leaf_restriction_duration_prefix_tokens(tokens)
        .and_then(|parsed| {
            map_pump_duration(parsed.duration).map(|duration| (parsed.rest, duration))
        })
        .map(|(rest, duration)| (trim_lexed_commas(rest), Some(duration)))
        .unwrap_or((tokens, None));
    if subject_tokens.is_empty() {
        return None;
    }

    let normalized = primitives::parse_prefix(subject_tokens, primitives::kw("each"))
        .map(|(_, rest)| rest)
        .unwrap_or(subject_tokens);
    let normalized = primitives::parse_prefix(normalized, primitives::kw("of"))
        .map(|(_, rest)| rest)
        .unwrap_or(normalized);
    if exact(
        normalized,
        primitives::any_phrase(&[
            &["they", "each"],
            &["them", "each"],
            &["it"],
            &["they"],
            &["them"],
        ])
        .void(),
    ) {
        return Some(PumpSubjectShape {
            subject_tokens,
            duration,
            kind: PumpSubjectKind::Tagged,
        });
    }
    if primitives::parse_prefix(
        normalized,
        alt((primitives::kw("that"), primitives::kw("those"))),
    )
    .is_some()
        || crate::runtime_backend::front_end::grammar::targets::parse_chosen_object_target(
            normalized,
        )
        .is_some()
    {
        return Some(PumpSubjectShape {
            subject_tokens,
            duration,
            kind: PumpSubjectKind::DemonstrativeTarget,
        });
    }

    let controlled = primitives::find_prefix(subject_tokens, || {
        alt((
            primitives::any_phrase(&[
                &["target", "opponent", "controls"],
                &["target", "opponents", "control"],
            ])
            .value(PlayerFilter::target_opponent()),
            primitives::any_phrase(&[
                &["target", "player", "controls"],
                &["target", "players", "control"],
            ])
            .value(PlayerFilter::target_player()),
        ))
    });
    if let Some((target_start, controller, _)) = controlled {
        let filter_tokens = trim_lexed_commas(subject_tokens.get(..target_start)?);
        if !filter_tokens.is_empty() {
            return Some(PumpSubjectShape {
                subject_tokens,
                duration,
                kind: PumpSubjectKind::ControlledFilter {
                    filter_tokens,
                    controller,
                },
            });
        }
    }

    if primitives::find_prefix(subject_tokens, || primitives::kw("target")).is_some() {
        let target_tokens = primitives::parse_prefix(
            subject_tokens,
            alt((primitives::kw("have"), primitives::kw("has"))),
        )
        .map(|(_, rest)| rest)
        .unwrap_or(subject_tokens);
        return Some(PumpSubjectShape {
            subject_tokens,
            duration,
            kind: PumpSubjectKind::DirectTarget(target_tokens),
        });
    }

    if exact(
        subject_tokens,
        primitives::any_phrase(&[&["equipped", "creature"], &["equipped", "permanent"]]).void(),
    ) {
        return Some(PumpSubjectShape {
            subject_tokens,
            duration,
            kind: PumpSubjectKind::Equipped,
        });
    }
    if exact(
        subject_tokens,
        primitives::any_phrase(&[&["enchanted", "creature"], &["enchanted", "permanent"]]).void(),
    ) {
        return Some(PumpSubjectShape {
            subject_tokens,
            duration,
            kind: PumpSubjectKind::Enchanted,
        });
    }

    let words = TokenWordView::new(subject_tokens).word_refs();
    let demonstrative_reference = for_each_shapes::has_demonstrative_object_reference_words(&words);
    let mentions_this =
        primitives::find_prefix(subject_tokens, || primitives::kw("this")).is_some();
    let has_pronoun = primitives::find_prefix(subject_tokens, || {
        alt((
            primitives::kw("it"),
            primitives::kw("they"),
            primitives::kw("them"),
        ))
    })
    .is_some();
    let counter_state = become_shapes::parse_counter_state_pronoun_tokens(subject_tokens);
    Some(PumpSubjectShape {
        subject_tokens,
        duration,
        kind: PumpSubjectKind::FilterCandidate {
            filter_tokens: subject_tokens,
            mentions_this,
            disallowed_pronoun: has_pronoun && !counter_state,
            demonstrative_reference,
        },
    })
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct AbilityTailShape<'a> {
    pub(crate) ability_tokens: &'a [OwnedLexToken],
    pub(crate) trailing_tokens: &'a [OwnedLexToken],
    pub(crate) duration: Until,
}

pub(crate) fn parse_ability_tail_shape(tokens: &[OwnedLexToken]) -> AbilityTailShape<'_> {
    let words = TokenWordView::new(tokens);
    let word_refs = words.word_refs();
    let Some(duration) = gain_ability_shapes::parse_simple_ability_duration_shape(&word_refs)
    else {
        return AbilityTailShape {
            ability_tokens: trim_lexed_commas(tokens),
            trailing_tokens: &[],
            duration: Until::Forever,
        };
    };
    let start = words
        .token_span_for_words(duration.start, duration.start + duration.len)
        .map(|range| range.start)
        .unwrap_or(tokens.len());
    AbilityTailShape {
        ability_tokens: trim_lexed_commas(&tokens[..start]),
        trailing_tokens: trim_lexed_commas(&tokens[start..]),
        duration: duration.duration,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ReferenceSubjectShape {
    Source,
    Tagged,
    Other,
}

pub(crate) fn parse_reference_subject_shape(tokens: &[OwnedLexToken]) -> ReferenceSubjectShape {
    if exact(tokens, primitives::kw("this").void()) {
        ReferenceSubjectShape::Source
    } else if exact(
        tokens,
        primitives::any_phrase(&[&["it"], &["they"], &["them"]]).void(),
    ) {
        ReferenceSubjectShape::Tagged
    } else {
        ReferenceSubjectShape::Other
    }
}

pub(crate) fn is_return_tagged_reference_shape(tokens: &[OwnedLexToken]) -> bool {
    exact(
        tokens,
        primitives::any_phrase(&[
            &["it"],
            &["them"],
            &["that", "card"],
            &["that", "creature"],
            &["that", "permanent"],
            &["that", "object"],
            &["those", "cards"],
            &["those", "creatures"],
            &["those", "permanents"],
            &["those", "objects"],
        ])
        .void(),
    )
}

pub(crate) fn is_exiled_cards_to_hand_shape(
    subject_tokens: &[OwnedLexToken],
    action_tokens: &[OwnedLexToken],
) -> bool {
    let subject_has_quantifier = primitives::parse_prefix(
        subject_tokens,
        alt((primitives::kw("all"), primitives::kw("each"))),
    )
    .is_some();
    subject_has_quantifier
        && primitives::find_prefix(subject_tokens, || {
            alt((primitives::kw("card"), primitives::kw("cards")))
        })
        .is_some()
        && primitives::find_prefix(subject_tokens, || primitives::kw("exiled")).is_some()
        && primitives::find_prefix(action_tokens, || {
            alt((primitives::kw("hand"), primitives::kw("hands")))
        })
        .is_some()
}

#[cfg(test)]
#[path = "core/tests.rs"]
mod tests;
