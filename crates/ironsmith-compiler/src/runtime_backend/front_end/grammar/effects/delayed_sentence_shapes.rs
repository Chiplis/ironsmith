use winnow::combinator::{alt, eof, opt, repeat};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

use crate::runtime_backend::front_end::grammar::primitives;
use crate::runtime_backend::front_end::lexer::{LexStream, LexedClause, OwnedLexToken};
use crate::target::PlayerFilter;

#[path = "delayed_sentence_shapes/schedule.rs"]
mod schedule;
pub(crate) use schedule::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DelayedObjectKind {
    Creature,
    Permanent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DelayedLeavesObjectKind {
    Creature,
    Permanent,
    Token,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DelayedTaggedLeavesShape<'a> {
    pub(crate) kind: DelayedLeavesObjectKind,
    pub(crate) effect_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DelayedNextCombatShape<'a> {
    pub(crate) effect_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct DelayedEndStepShape<'a> {
    pub(crate) player: PlayerFilter,
    pub(crate) start_next_turn: bool,
    pub(crate) effect_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DelayedThisTurnPlacement {
    LeadingDuration,
    TrailingDuration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DelayedThisTurnShape<'a> {
    pub(crate) placement: DelayedThisTurnPlacement,
    pub(crate) trigger_tokens: &'a [OwnedLexToken],
    pub(crate) effect_tokens: &'a [OwnedLexToken],
    pub(crate) references_previous_creature: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DelayedTaggedDamageShape {
    pub(crate) kind: DelayedObjectKind,
    pub(crate) combat: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CopyTwiceShape {
    pub(crate) may_choose_new_targets: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DelayedDiesShape<'a> {
    ThatReference {
        effect_tokens: &'a [OwnedLexToken],
    },
    ThisWay {
        subject_tokens: &'a [OwnedLexToken],
        effect_tokens: &'a [OwnedLexToken],
    },
}

fn trimmed(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    LexedClause::new(tokens).trimmed().tokens()
}

fn semantic_kw<'a>(
    expected: &'static str,
) -> impl Parser<LexStream<'a>, (), winnow::error::ErrMode<winnow::error::ContextError>> {
    (
        repeat::<_, _, (), _, _>(0.., semantic_noise),
        any.verify(move |token: &&OwnedLexToken| {
            token.is_word(expected)
                || matches!(token.parser_word_pieces(), [piece] if piece.text == expected)
        }),
    )
        .void()
}

fn semantic_noise<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        primitives::comma(),
        primitives::period(),
        primitives::semicolon(),
    ))
    .void()
    .parse_next(input)
}

fn semantic_phrase<'a>(
    expected: &'static [&'static str],
) -> impl Parser<LexStream<'a>, (), winnow::error::ErrMode<winnow::error::ContextError>> {
    move |input: &mut LexStream<'a>| {
        for word in expected {
            semantic_kw(word).parse_next(input)?;
        }
        Ok(())
    }
}

fn trigger_intro<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((primitives::kw("when"), primitives::kw("whenever")))
        .void()
        .parse_next(input)
}

fn dies_intro<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        primitives::kw("when"),
        primitives::kw("whenever"),
        primitives::kw("if"),
    ))
    .void()
    .parse_next(input)
}

fn object_kind<'a>(input: &mut LexStream<'a>) -> WResult<DelayedObjectKind> {
    alt((
        primitives::kw("creature").value(DelayedObjectKind::Creature),
        primitives::kw("permanent").value(DelayedObjectKind::Permanent),
    ))
    .parse_next(input)
}

fn leaves_object_kind<'a>(input: &mut LexStream<'a>) -> WResult<DelayedLeavesObjectKind> {
    alt((
        primitives::kw("creature").value(DelayedLeavesObjectKind::Creature),
        primitives::kw("permanent").value(DelayedLeavesObjectKind::Permanent),
        primitives::kw("token").value(DelayedLeavesObjectKind::Token),
    ))
    .parse_next(input)
}

/// Parses a delayed follow-up that watches the object selected or created by
/// the preceding effect: `When that creature/token/permanent leaves the
/// battlefield, ...`.
pub(crate) fn parse_delayed_tagged_leaves_shape(
    tokens: &[OwnedLexToken],
) -> Option<DelayedTaggedLeavesShape<'_>> {
    let tokens = trimmed(tokens);
    let (header_tokens, effect_tokens) =
        primitives::split_lexed_once_on_separator(tokens, || primitives::comma().void())?;
    let kind = primitives::parse_all(
        trimmed(header_tokens),
        (
            trigger_intro,
            primitives::kw("that"),
            leaves_object_kind,
            primitives::phrase(&["leaves", "the", "battlefield"]),
            eof,
        )
            .map(|(_, _, kind, _, _)| kind),
        "delayed tagged-object leaves trigger",
    )
    .ok()?;
    let effect_tokens = trimmed(effect_tokens);
    (!effect_tokens.is_empty()).then_some(DelayedTaggedLeavesShape {
        kind,
        effect_tokens,
    })
}

pub(crate) fn parse_delayed_next_combat_shape(
    tokens: &[OwnedLexToken],
) -> Option<DelayedNextCombatShape<'_>> {
    let tokens = trimmed(tokens);
    let (_, after_header) = primitives::parse_prefix(
        tokens,
        (
            primitives::phrase(&["at", "the", "beginning", "of", "the", "next", "combat"]),
            opt(primitives::kw("phase")),
            primitives::phrase(&["this", "turn"]),
            primitives::comma(),
        ),
    )?;
    let effect_tokens = trimmed(after_header);
    (!effect_tokens.is_empty()).then_some(DelayedNextCombatShape { effect_tokens })
}

fn end_step_owner<'a>(input: &mut LexStream<'a>) -> WResult<PlayerFilter> {
    alt((
        semantic_kw("your").value(PlayerFilter::You),
        (
            primitives::kw("that"),
            alt((semantic_kw("player"), semantic_kw("players"))),
        )
            .value(PlayerFilter::IteratedPlayer),
        (
            primitives::kw("target"),
            alt((semantic_kw("player"), semantic_kw("players"))),
        )
            .value(PlayerFilter::Target(Box::new(PlayerFilter::Any))),
    ))
    .parse_next(input)
}

fn end_step_header<'a>(input: &mut LexStream<'a>) -> WResult<(PlayerFilter, bool)> {
    primitives::kw("at").parse_next(input)?;
    opt(primitives::kw("the")).parse_next(input)?;
    primitives::phrase(&["beginning", "of"]).parse_next(input)?;
    opt(primitives::kw("the")).parse_next(input)?;
    let step_owner = opt(semantic_kw("your").value(PlayerFilter::You)).parse_next(input)?;
    alt((
        primitives::phrase(&["next", "end", "step"]),
        primitives::phrase(&["end", "step"]),
    ))
    .parse_next(input)?;
    let turn_owner = opt((
        primitives::kw("of"),
        end_step_owner,
        primitives::phrase(&["next", "turn"]),
    ))
    .parse_next(input)?;
    let start_next_turn = turn_owner.is_some();
    let player = turn_owner
        .map(|(_, player, _)| player)
        .or(step_owner)
        .unwrap_or(PlayerFilter::Any);
    Ok((player, start_next_turn))
}

pub(crate) fn parse_delayed_end_step_shape(
    tokens: &[OwnedLexToken],
) -> Option<DelayedEndStepShape<'_>> {
    let tokens = trimmed(tokens);
    let ((player, start_next_turn), after_comma) = primitives::parse_prefix(
        tokens,
        (end_step_header, primitives::comma()).map(|(header, _)| header),
    )?;
    let effect_tokens = trimmed(after_comma);
    (!effect_tokens.is_empty()).then_some(DelayedEndStepShape {
        player,
        start_next_turn,
        effect_tokens,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DelayedTriggerCore<'a> {
    tokens: &'a [OwnedLexToken],
    references_previous_creature: bool,
}

fn parse_intro_and_trigger(tokens: &[OwnedLexToken]) -> Option<DelayedTriggerCore<'_>> {
    let (_, trigger_tokens) = primitives::parse_prefix(trimmed(tokens), trigger_intro)?;
    let trigger_tokens = trimmed(trigger_tokens);
    if trigger_tokens.is_empty() {
        return None;
    }
    let references_previous_creature =
        primitives::parse_prefix(trigger_tokens, semantic_phrase(&["that", "creature"])).is_some();
    Some(DelayedTriggerCore {
        tokens: trigger_tokens,
        references_previous_creature,
    })
}

pub(crate) fn parse_delayed_this_turn_shape(
    tokens: &[OwnedLexToken],
) -> Option<DelayedThisTurnShape<'_>> {
    let tokens = trimmed(tokens);

    if let Some(((), after_duration)) = primitives::parse_prefix(
        tokens,
        (
            primitives::phrase(&["this", "turn"]),
            opt(primitives::comma()),
        )
            .void(),
    ) && let Some((trigger_header, effect_tokens)) =
        primitives::split_lexed_once_on_separator(after_duration, || primitives::comma().void())
        && let Some(trigger) = parse_intro_and_trigger(trigger_header)
    {
        let effect_tokens = trimmed(effect_tokens);
        if !effect_tokens.is_empty() {
            return Some(DelayedThisTurnShape {
                placement: DelayedThisTurnPlacement::LeadingDuration,
                trigger_tokens: trigger.tokens,
                effect_tokens,
                references_previous_creature: trigger.references_previous_creature,
            });
        }
    }

    let (intro_and_trigger, effect_tokens) =
        primitives::split_lexed_once_before_suffix(tokens, 1, || {
            (
                primitives::phrase(&["this", "turn"]),
                primitives::comma(),
                repeat::<_, _, (), _, _>(1.., any.void()).take(),
                eof,
            )
                .map(|(_, _, effect_tokens, _)| effect_tokens)
        })?;
    let trigger = parse_intro_and_trigger(intro_and_trigger)?;
    let effect_tokens = trimmed(effect_tokens);
    (!effect_tokens.is_empty()).then_some(DelayedThisTurnShape {
        placement: DelayedThisTurnPlacement::TrailingDuration,
        trigger_tokens: trigger.tokens,
        effect_tokens,
        references_previous_creature: trigger.references_previous_creature,
    })
}

pub(crate) fn parse_delayed_attack_unblocked_subject(
    tokens: &[OwnedLexToken],
) -> Option<&[OwnedLexToken]> {
    let tokens = trimmed(tokens);
    let (_, after_target) = primitives::parse_prefix(tokens, primitives::kw("target"))?;
    let (subject_tokens, ()) = primitives::split_lexed_once_before_suffix(after_target, 1, || {
        (
            primitives::phrase(&["attacks", "and"]),
            alt((primitives::kw("isn't"), primitives::kw("isnt"))),
            primitives::kw("blocked"),
            eof,
        )
            .void()
    })?;
    let subject_tokens = trimmed(subject_tokens);
    (!subject_tokens.is_empty()).then_some(subject_tokens)
}

pub(crate) fn parse_delayed_tagged_damage_shape(
    tokens: &[OwnedLexToken],
) -> Option<DelayedTaggedDamageShape> {
    primitives::parse_all(
        trimmed(tokens),
        (
            primitives::kw("that"),
            object_kind,
            primitives::phrase(&["is", "dealt"]),
            opt(primitives::kw("combat")),
            primitives::kw("damage"),
            eof,
        )
            .map(|(_, kind, _, combat, _, _)| DelayedTaggedDamageShape {
                kind,
                combat: combat.is_some(),
            }),
        "delayed tagged damage trigger",
    )
    .ok()
}

pub(crate) fn parse_delayed_deals_combat_damage_kind(
    tokens: &[OwnedLexToken],
) -> Option<DelayedObjectKind> {
    primitives::parse_all(
        trimmed(tokens),
        (
            primitives::kw("that"),
            object_kind,
            primitives::phrase(&["deals", "combat", "damage", "to", "a", "player"]),
            eof,
        )
            .map(|(_, kind, _, _)| kind),
        "delayed combat-damage trigger",
    )
    .ok()
}

/// Parse the object-kind portion of a delayed "target ... dies" trigger.
///
/// The target is chosen while the enclosing spell or ability resolves; the
/// delayed trigger must subsequently watch that exact object rather than every
/// object matching the descriptive filter.
pub(crate) fn parse_delayed_target_dies_subject(
    tokens: &[OwnedLexToken],
) -> Option<&[OwnedLexToken]> {
    let tokens = trimmed(tokens);
    let (_, after_target) = primitives::parse_prefix(tokens, primitives::kw("target"))?;
    let (subject_tokens, ()) = primitives::split_lexed_once_before_suffix(after_target, 1, || {
        (primitives::kw("dies"), eof).void()
    })?;
    let subject_tokens = trimmed(subject_tokens);
    (!subject_tokens.is_empty()).then_some(subject_tokens)
}

/// Parse the object-kind portion of a delayed
/// "target ... is put into your graveyard" trigger.
pub(crate) fn parse_delayed_target_put_into_your_graveyard_subject(
    tokens: &[OwnedLexToken],
) -> Option<&[OwnedLexToken]> {
    let tokens = trimmed(tokens);
    let (_, after_target) = primitives::parse_prefix(tokens, primitives::kw("target"))?;
    let (subject_tokens, ()) = primitives::split_lexed_once_before_suffix(after_target, 1, || {
        (
            primitives::phrase(&["is", "put", "into", "your", "graveyard"]),
            eof,
        )
            .void()
    })?;
    let subject_tokens = trimmed(subject_tokens);
    (!subject_tokens.is_empty()).then_some(subject_tokens)
}

pub(crate) fn is_next_cast_spell_or_loyalty_shape(tokens: &[OwnedLexToken]) -> bool {
    primitives::parse_all(
        trimmed(tokens),
        alt((
            semantic_phrase(&[
                "you", "next", "cast", "an", "instant", "spell", "cast", "a", "sorcery", "spell",
                "or", "activate", "a", "loyalty", "ability",
            ]),
            semantic_phrase(&[
                "you", "next", "cast", "an", "instant", "or", "sorcery", "spell", "or", "activate",
                "a", "loyalty", "ability",
            ]),
        )),
        "next spell-or-loyalty trigger",
    )
    .is_ok()
}

pub(crate) fn parse_copy_twice_shape(tokens: &[OwnedLexToken]) -> Option<CopyTwiceShape> {
    let (_, tail) = primitives::parse_prefix(
        trimmed(tokens),
        semantic_phrase(&["copy", "that", "spell", "or", "ability", "twice"]),
    )?;
    let tail = trimmed(tail);
    if tail.is_empty()
        || primitives::parse_all(
            tail,
            (repeat::<_, _, (), _, _>(0.., semantic_noise), eof).void(),
            "copy twice punctuation tail",
        )
        .is_ok()
    {
        return Some(CopyTwiceShape {
            may_choose_new_targets: false,
        });
    }
    primitives::parse_all(
        tail,
        (
            semantic_phrase(&[
                "you", "may", "choose", "new", "targets", "for", "the", "copies",
            ]),
            repeat::<_, _, (), _, _>(0.., semantic_noise),
            eof,
        )
            .void(),
        "copy twice target tail",
    )
    .ok()
    .map(|()| CopyTwiceShape {
        may_choose_new_targets: true,
    })
}

pub(crate) fn delayed_trigger_has_next_marker(tokens: &[OwnedLexToken]) -> bool {
    primitives::find_prefix(trimmed(tokens), || primitives::kw("next")).is_some()
}

fn dies_this_way_suffix<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        primitives::phrase(&["dealt", "damage", "this", "way", "dies", "this", "turn"]),
        primitives::phrase(&[
            "dealt", "damage", "this", "way", "would", "die", "this", "turn",
        ]),
    ))
    .void()
    .parse_next(input)
}

pub(crate) fn parse_delayed_dies_shape(tokens: &[OwnedLexToken]) -> Option<DelayedDiesShape<'_>> {
    let tokens = trimmed(tokens);
    let (header_tokens, effect_tokens) =
        primitives::split_lexed_once_on_separator(tokens, || primitives::comma().void())?;
    let effect_tokens = trimmed(effect_tokens);
    if effect_tokens.is_empty() {
        return None;
    }
    let (_, trigger_tokens) = primitives::parse_prefix(trimmed(header_tokens), dies_intro)?;

    if let Some(((), after_that)) =
        primitives::parse_prefix(trigger_tokens, primitives::kw("that").void())
        && primitives::split_lexed_once_before_suffix(after_that, 0, || {
            (primitives::phrase(&["dies", "this", "turn"]), eof).void()
        })
        .is_some()
    {
        return Some(DelayedDiesShape::ThatReference { effect_tokens });
    }

    let (subject_tokens, ()) =
        primitives::split_lexed_once_before_suffix(trigger_tokens, 1, || {
            (dies_this_way_suffix, eof).void()
        })?;
    let subject_tokens = trimmed(subject_tokens);
    (!subject_tokens.is_empty()).then_some(DelayedDiesShape::ThisWay {
        subject_tokens,
        effect_tokens,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::front_end::lexer::lex_line;

    fn tokens(text: &str) -> Vec<OwnedLexToken> {
        lex_line(text, 0).unwrap()
    }

    #[test]
    fn parses_delayed_headers_and_typed_trigger_facts() {
        let next_combat =
            tokens("At the beginning of the next combat phase this turn, target creature attacks.");
        assert!(parse_delayed_next_combat_shape(&next_combat).is_some());

        let end_step = tokens("At the beginning of your next end step, draw a card.");
        assert_eq!(
            parse_delayed_end_step_shape(&end_step).unwrap().player,
            PlayerFilter::You
        );

        let delayed =
            tokens("This turn, when target creature attacks and isn't blocked, draw a card.");
        let shape = parse_delayed_this_turn_shape(&delayed).unwrap();
        assert_eq!(shape.placement, DelayedThisTurnPlacement::LeadingDuration);
        assert!(parse_delayed_attack_unblocked_subject(shape.trigger_tokens).is_some());
        assert!(!shape.references_previous_creature);

        let prior_creature =
            tokens("Whenever that creature is dealt damage this turn, draw a card.");
        assert!(
            parse_delayed_this_turn_shape(&prior_creature)
                .unwrap()
                .references_previous_creature
        );

        let unrelated = tokens("Whenever you draw a card this turn, gain 1 life.");
        assert!(
            !parse_delayed_this_turn_shape(&unrelated)
                .unwrap()
                .references_previous_creature
        );

        let target_dies = tokens("When target creature dies this turn, draw a card.");
        let shape = parse_delayed_this_turn_shape(&target_dies).unwrap();
        assert!(parse_delayed_target_dies_subject(shape.trigger_tokens).is_some());

        let target_graveyard =
            tokens("When target creature is put into your graveyard this turn, draw a card.");
        let shape = parse_delayed_this_turn_shape(&target_graveyard).unwrap();
        assert!(
            parse_delayed_target_put_into_your_graveyard_subject(shape.trigger_tokens).is_some()
        );
    }

    #[test]
    fn parses_dies_and_copy_twice_shapes() {
        let dies = tokens("When that creature dies this turn, return it to your hand.");
        assert!(matches!(
            parse_delayed_dies_shape(&dies),
            Some(DelayedDiesShape::ThatReference { .. })
        ));
        let copy =
            tokens("copy that spell or ability twice you may choose new targets for the copies");
        assert!(
            parse_copy_twice_shape(&copy)
                .unwrap()
                .may_choose_new_targets
        );

        let leaves = tokens(
            "When that creature leaves the battlefield, return this card from exile to the battlefield.",
        );
        let shape = parse_delayed_tagged_leaves_shape(&leaves).unwrap();
        assert_eq!(shape.kind, DelayedLeavesObjectKind::Creature);
        assert!(!shape.effect_tokens.is_empty());
    }
}
