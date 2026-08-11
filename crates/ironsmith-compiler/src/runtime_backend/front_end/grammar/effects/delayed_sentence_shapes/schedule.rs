use winnow::combinator::{alt, opt};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;

use crate::cards::builders::PlayerAst;
use crate::runtime_backend::front_end::grammar::primitives;
use crate::runtime_backend::front_end::lexer::{LexStream, OwnedLexToken};

use super::{semantic_kw, semantic_phrase, trimmed};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DelayedScheduleStep {
    UntapStep,
    Upkeep,
    DrawStep,
    FirstMainPhase,
    MainPhase,
    EndStep,
}

/// A leading delayed-step sentence such as "At the beginning of your next
/// upkeep, ...". This is a semantic effect shape, not a printed triggered
/// ability: resolving the surrounding spell registers the one-shot trigger.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct DelayedScheduleSentenceShape<'a> {
    pub(crate) step: DelayedScheduleStep,
    pub(crate) player: PlayerAst,
    pub(crate) start_next_turn: bool,
    pub(crate) effect_tokens: &'a [OwnedLexToken],
}

fn delayed_schedule_player<'a>(input: &mut LexStream<'a>) -> WResult<PlayerAst> {
    alt((
        semantic_kw("your").value(PlayerAst::You),
        semantic_kw("their").value(PlayerAst::That),
        (
            primitives::kw("that"),
            alt((semantic_kw("player"), semantic_kw("players"))),
        )
            .value(PlayerAst::That),
        (
            primitives::kw("target"),
            alt((semantic_kw("player"), semantic_kw("players"))),
        )
            .value(PlayerAst::Target),
    ))
    .parse_next(input)
}

fn delayed_schedule_step<'a>(input: &mut LexStream<'a>) -> WResult<DelayedScheduleStep> {
    alt((
        semantic_phrase(&["draw", "step"]).value(DelayedScheduleStep::DrawStep),
        semantic_phrase(&["first", "main", "phase"]).value(DelayedScheduleStep::FirstMainPhase),
        semantic_phrase(&["main", "phase"]).value(DelayedScheduleStep::MainPhase),
        semantic_phrase(&["end", "step"]).value(DelayedScheduleStep::EndStep),
        (semantic_kw("upkeep"), opt(semantic_kw("step"))).value(DelayedScheduleStep::Upkeep),
    ))
    .parse_next(input)
}

fn delayed_untap_schedule_header<'a>(
    input: &mut LexStream<'a>,
) -> WResult<(DelayedScheduleStep, PlayerAst, bool)> {
    primitives::kw("during").parse_next(input)?;
    let player = delayed_schedule_player.parse_next(input)?;
    semantic_phrase(&["next", "untap", "step"]).parse_next(input)?;
    primitives::comma().parse_next(input)?;
    primitives::kw("as").parse_next(input)?;
    match player {
        PlayerAst::You => {
            semantic_phrase(&["you", "untap", "your", "permanents"]).parse_next(input)?;
        }
        PlayerAst::That | PlayerAst::Target | PlayerAst::TargetOpponent => {
            semantic_phrase(&["they", "untap", "their", "permanents"]).parse_next(input)?;
        }
        _ => {
            return Err(primitives::backtrack_err(
                "delayed untap-step player",
                "delayed untap-step player",
            ));
        }
    }
    Ok((DelayedScheduleStep::UntapStep, player, true))
}

fn delayed_schedule_header<'a>(
    input: &mut LexStream<'a>,
) -> WResult<(DelayedScheduleStep, PlayerAst, bool)> {
    primitives::kw("at").parse_next(input)?;
    opt(primitives::kw("the")).parse_next(input)?;
    primitives::phrase(&["beginning", "of"]).parse_next(input)?;
    opt(primitives::kw("the")).parse_next(input)?;

    alt((
        (
            semantic_phrase(&["end", "step", "of"]),
            delayed_schedule_player,
            semantic_phrase(&["next", "turn"]),
        )
            .map(|(_, player, _)| (DelayedScheduleStep::EndStep, player, true)),
        semantic_phrase(&["that", "turns", "end", "step"]).value((
            DelayedScheduleStep::EndStep,
            PlayerAst::That,
            true,
        )),
        primitives::phrase(&["that", "turn's", "end", "step"]).value((
            DelayedScheduleStep::EndStep,
            PlayerAst::That,
            true,
        )),
        (
            delayed_schedule_player,
            semantic_kw("next"),
            delayed_schedule_step,
        )
            .map(|(player, _, step)| {
                let start_next_turn = matches!(
                    step,
                    DelayedScheduleStep::Upkeep | DelayedScheduleStep::DrawStep
                );
                (step, player, start_next_turn)
            }),
        (semantic_phrase(&["next", "turns"]), delayed_schedule_step)
            .map(|(_, step)| (step, PlayerAst::Any, true)),
        (semantic_kw("next"), delayed_schedule_step).map(|(_, step)| {
            let start_next_turn = matches!(
                step,
                DelayedScheduleStep::Upkeep | DelayedScheduleStep::DrawStep
            );
            (step, PlayerAst::Any, start_next_turn)
        }),
    ))
    .parse_next(input)
}

pub(crate) fn parse_delayed_schedule_sentence_shape(
    tokens: &[OwnedLexToken],
) -> Option<DelayedScheduleSentenceShape<'_>> {
    let tokens = trimmed(tokens);
    let ((step, player, start_next_turn), after_comma) = primitives::parse_prefix(
        tokens,
        (
            alt((delayed_untap_schedule_header, delayed_schedule_header)),
            primitives::comma(),
        )
            .map(|(header, _)| header),
    )?;
    let effect_tokens = trimmed(after_comma);
    (!effect_tokens.is_empty()).then_some(DelayedScheduleSentenceShape {
        step,
        player,
        start_next_turn,
        effect_tokens,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::front_end::lexer::{LexedClause, lex_line};

    fn tokens(text: &str) -> Vec<OwnedLexToken> {
        lex_line(text, 0).unwrap()
    }

    #[test]
    fn parses_leading_delayed_schedule_sentences() {
        let untap = tokens(
            "During your next untap step, as you untap your permanents, return this land to its owner's hand.",
        );
        let untap = parse_delayed_schedule_sentence_shape(&untap).unwrap();
        assert_eq!(untap.step, DelayedScheduleStep::UntapStep);
        assert_eq!(untap.player, PlayerAst::You);
        assert!(untap.start_next_turn);
        assert_eq!(
            LexedClause::new(untap.effect_tokens).word_refs(),
            ["return", "this", "land", "to", "its", "owners", "hand"]
        );

        let upkeep = tokens("At the beginning of your next upkeep, draw a card.");
        let upkeep = parse_delayed_schedule_sentence_shape(&upkeep).unwrap();
        assert_eq!(upkeep.step, DelayedScheduleStep::Upkeep);
        assert_eq!(upkeep.player, PlayerAst::You);
        assert!(upkeep.start_next_turn);
        assert_eq!(
            LexedClause::new(upkeep.effect_tokens).word_refs(),
            ["draw", "a", "card"]
        );

        let end_step =
            tokens("At the beginning of the end step of that player's next turn, draw a card.");
        let end_step = parse_delayed_schedule_sentence_shape(&end_step).unwrap();
        assert_eq!(end_step.step, DelayedScheduleStep::EndStep);
        assert_eq!(end_step.player, PlayerAst::That);
        assert!(end_step.start_next_turn);

        let next_end_step = tokens("At the beginning of the next end step, draw a card.");
        let next_end_step = parse_delayed_schedule_sentence_shape(&next_end_step).unwrap();
        assert_eq!(next_end_step.step, DelayedScheduleStep::EndStep);
        assert_eq!(next_end_step.player, PlayerAst::Any);
        assert!(!next_end_step.start_next_turn);

        let main_phase = tokens("At the beginning of your next main phase, add {C}.");
        let main_phase = parse_delayed_schedule_sentence_shape(&main_phase).unwrap();
        assert_eq!(main_phase.step, DelayedScheduleStep::MainPhase);
        assert_eq!(main_phase.player, PlayerAst::You);
        assert!(!main_phase.start_next_turn);
        assert_eq!(
            LexedClause::new(main_phase.effect_tokens).word_refs(),
            ["add", "c"]
        );

        let first_main_phase = tokens("At the beginning of your next first main phase, add {C}.");
        let first_main_phase = parse_delayed_schedule_sentence_shape(&first_main_phase).unwrap();
        assert_eq!(first_main_phase.step, DelayedScheduleStep::FirstMainPhase);
        assert_eq!(first_main_phase.player, PlayerAst::You);
        assert!(!first_main_phase.start_next_turn);

        let recurring = tokens("At the beginning of your upkeep, draw a card.");
        assert!(parse_delayed_schedule_sentence_shape(&recurring).is_none());
    }
}
