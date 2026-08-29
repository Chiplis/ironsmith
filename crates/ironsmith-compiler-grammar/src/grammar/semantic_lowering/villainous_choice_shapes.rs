use winnow::combinator::{alt, eof, opt, peek, repeat, repeat_till};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

use crate::effect::ChoiceCount;

use super::super::super::lexer::{LexStream, OwnedLexToken, TokenKind};
use super::super::{leaf, primitives};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VillainousChoiceTarget {
    CreaturesYouDontControl,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VillainousChoiceIteration {
    EachOfThem,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VillainousChoiceChooser {
    IteratedCreaturesController,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VillainousChoiceSharedSubjectPair<'a> {
    pub subject_tokens: &'a [OwnedLexToken],
    pub first_action_tokens: &'a [OwnedLexToken],
    pub second_action_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VillainousChoiceModeProgram<'a> {
    Direct(&'a [OwnedLexToken]),
    SharedSubjectPair(VillainousChoiceSharedSubjectPair<'a>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VillainousChoiceStatementShape<'a> {
    pub count: ChoiceCount,
    pub target: VillainousChoiceTarget,
    pub iteration: VillainousChoiceIteration,
    pub chooser: VillainousChoiceChooser,
    pub chooser_tokens: &'a [OwnedLexToken],
    pub first_mode_tokens: &'a [OwnedLexToken],
    pub second_mode_tokens: &'a [OwnedLexToken],
    pub first_mode_program: VillainousChoiceModeProgram<'a>,
    pub second_mode_program: VillainousChoiceModeProgram<'a>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VillainousChoicePlayerIteration {
    EachOpponent,
    TargetOpponent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VillainousChoicePlayerStatementShape<'a> {
    pub leading_then: bool,
    pub iteration: VillainousChoicePlayerIteration,
    pub minimum_life_lost_this_turn: Option<u32>,
    pub chooser_tokens: &'a [OwnedLexToken],
    pub first_mode_tokens: &'a [OwnedLexToken],
    pub second_mode_tokens: &'a [OwnedLexToken],
    pub first_mode_program: VillainousChoiceModeProgram<'a>,
    pub second_mode_program: VillainousChoiceModeProgram<'a>,
}

fn parse_creature_or_creatures<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((primitives::kw("creature"), primitives::kw("creatures")))
        .void()
        .parse_next(input)
}

fn parse_dont<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((primitives::kw("don't"), primitives::kw("dont")))
        .void()
        .parse_next(input)
}

fn parse_choice_separator<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        primitives::token_kind(TokenKind::EmDash),
        primitives::token_kind(TokenKind::Dash),
        primitives::colon(),
    ))
    .void()
    .parse_next(input)
}

fn parse_mode_separator<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        (primitives::comma(), primitives::kw("or")).void(),
        primitives::kw("or").void(),
    ))
    .parse_next(input)
}

fn parse_mode_action_head<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        alt((
            primitives::kw("become"),
            primitives::kw("becomes"),
            primitives::kw("create"),
            primitives::kw("creates"),
            primitives::kw("destroy"),
            primitives::kw("destroys"),
            primitives::kw("discard"),
            primitives::kw("discards"),
            primitives::kw("draw"),
        )),
        alt((
            alt((
                primitives::kw("draws"),
                primitives::kw("gain"),
                primitives::kw("gains"),
                primitives::kw("get"),
                primitives::kw("gets"),
            )),
            alt((
                primitives::kw("lose"),
                primitives::kw("loses"),
                primitives::kw("put"),
                primitives::kw("puts"),
                primitives::kw("sacrifice"),
                primitives::kw("sacrifices"),
            )),
        )),
    ))
    .void()
    .parse_next(input)
}

fn parse_shared_subject_pair_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<VillainousChoiceSharedSubjectPair<'a>> {
    let subject_tokens =
        repeat_till::<_, _, (), _, _, _, _>(1.., any.void(), peek(parse_mode_action_head))
            .map(|((), ())| ())
            .take()
            .parse_next(input)?;
    let first_action_tokens = repeat_till::<_, _, (), _, _, _, _>(
        1..,
        any.void(),
        peek((primitives::kw("and"), parse_mode_action_head)),
    )
    .map(|((), (_, ()))| ())
    .take()
    .parse_next(input)?;
    primitives::kw("and").parse_next(input)?;
    let second_action_tokens = repeat::<_, _, (), _, _>(1.., any.void())
        .take()
        .parse_next(input)?;
    eof.parse_next(input)?;
    Ok(VillainousChoiceSharedSubjectPair {
        subject_tokens,
        first_action_tokens,
        second_action_tokens,
    })
}

fn classify_mode_program(tokens: &[OwnedLexToken]) -> VillainousChoiceModeProgram<'_> {
    primitives::parse_all(
        tokens,
        parse_shared_subject_pair_lexed,
        "villainous-choice mode pair",
    )
    .map(VillainousChoiceModeProgram::SharedSubjectPair)
    .unwrap_or(VillainousChoiceModeProgram::Direct(tokens))
}

fn parse_villainous_choice_statement_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<VillainousChoiceStatementShape<'a>> {
    let ((), count, (), ()) = (
        primitives::kw("choose").void(),
        leaf::parse_leaf_choice_count_prefix_lexed,
        (primitives::kw("target"), parse_creature_or_creatures).void(),
        (primitives::kw("you"), parse_dont, primitives::kw("control")).void(),
    )
        .parse_next(input)?;
    if count.min != 0 || (count.max.is_none() && !count.is_up_to_dynamic_x()) {
        return Err(primitives::backtrack_err(
            "villainous choice target count",
            "up to a leaf count",
        ));
    }

    primitives::period().parse_next(input)?;
    primitives::phrase(&["for", "each", "of", "them"]).parse_next(input)?;
    primitives::comma().parse_next(input)?;
    let chooser_tokens = primitives::phrase(&["that", "creature's", "controller"])
        .take()
        .parse_next(input)?;
    primitives::phrase(&["faces", "a", "villainous", "choice"]).parse_next(input)?;
    opt(parse_choice_separator).parse_next(input)?;

    let first_mode_tokens =
        repeat_till::<_, _, (), _, _, _, _>(1.., any.void(), peek(parse_mode_separator))
            .map(|((), ())| ())
            .take()
            .parse_next(input)?;
    parse_mode_separator.parse_next(input)?;
    let second_mode_tokens = repeat_till::<_, _, (), _, _, _, _>(
        1..,
        any.void(),
        peek(alt((primitives::period().void(), eof.void()))),
    )
    .map(|((), ())| ())
    .take()
    .parse_next(input)?;
    opt(primitives::period()).parse_next(input)?;
    eof.parse_next(input)?;

    Ok(VillainousChoiceStatementShape {
        count,
        target: VillainousChoiceTarget::CreaturesYouDontControl,
        iteration: VillainousChoiceIteration::EachOfThem,
        chooser: VillainousChoiceChooser::IteratedCreaturesController,
        chooser_tokens,
        first_mode_tokens,
        second_mode_tokens,
        first_mode_program: classify_mode_program(first_mode_tokens),
        second_mode_program: classify_mode_program(second_mode_tokens),
    })
}

fn parse_villainous_choice_player_statement_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<VillainousChoicePlayerStatementShape<'a>> {
    let leading_then = opt(primitives::kw("then")).parse_next(input)?.is_some();
    let (iteration, chooser_tokens, minimum_life_lost_this_turn) = alt((
        (
            primitives::phrase(&["each", "opponent"]).take(),
            opt((
                primitives::phrase(&["who", "lost"]),
                leaf::parse_leaf_number_prefix_lexed,
                primitives::phrase(&["or", "more", "life", "this", "turn"]),
            )
                .map(|(_, count, _)| count)),
        )
            .map(|(tokens, minimum)| {
                (
                    VillainousChoicePlayerIteration::EachOpponent,
                    tokens,
                    minimum,
                )
            }),
        primitives::phrase(&["target", "opponent"])
            .take()
            .map(|tokens| {
                (
                    VillainousChoicePlayerIteration::TargetOpponent,
                    tokens,
                    None,
                )
            }),
    ))
    .parse_next(input)?;
    primitives::phrase(&["faces", "a", "villainous", "choice"]).parse_next(input)?;
    opt(parse_choice_separator).parse_next(input)?;

    let first_mode_tokens =
        repeat_till::<_, _, (), _, _, _, _>(1.., any.void(), peek(parse_mode_separator))
            .map(|((), ())| ())
            .take()
            .parse_next(input)?;
    parse_mode_separator.parse_next(input)?;
    let second_mode_tokens = repeat_till::<_, _, (), _, _, _, _>(
        1..,
        any.void(),
        peek(alt((primitives::period().void(), eof.void()))),
    )
    .map(|((), ())| ())
    .take()
    .parse_next(input)?;
    opt(primitives::period()).parse_next(input)?;
    eof.parse_next(input)?;

    Ok(VillainousChoicePlayerStatementShape {
        leading_then,
        iteration,
        minimum_life_lost_this_turn,
        chooser_tokens,
        first_mode_tokens,
        second_mode_tokens,
        first_mode_program: classify_mode_program(first_mode_tokens),
        second_mode_program: classify_mode_program(second_mode_tokens),
    })
}

pub fn parse_villainous_choice_statement_tokens(
    tokens: &[OwnedLexToken],
) -> Option<VillainousChoiceStatementShape<'_>> {
    primitives::parse_all(
        tokens,
        parse_villainous_choice_statement_lexed,
        "villainous-choice statement",
    )
    .ok()
}

pub fn parse_villainous_choice_player_statement_tokens(
    tokens: &[OwnedLexToken],
) -> Option<VillainousChoicePlayerStatementShape<'_>> {
    primitives::parse_all(
        tokens,
        parse_villainous_choice_player_statement_lexed,
        "villainous-choice player statement",
    )
    .ok()
}

#[cfg(test)]
mod tests {
    use super::super::super::super::lexer::{lex_line, render_token_slice};
    use super::*;

    #[test]
    fn parses_typed_villainous_choice_statement_and_borrows_clauses() {
        let tokens = lex_line(
            "Choose up to four target creatures you don't control. For each of them, that creature's controller faces a villainous choice — That creature becomes a 1/1 white Human creature and loses all abilities, or you create a token that's a copy of it.",
            0,
        )
        .unwrap();
        let shape = parse_villainous_choice_statement_tokens(&tokens).unwrap();

        assert_eq!(shape.count, ChoiceCount::up_to(4));
        assert_eq!(
            shape.target,
            VillainousChoiceTarget::CreaturesYouDontControl
        );
        assert_eq!(shape.iteration, VillainousChoiceIteration::EachOfThem);
        assert_eq!(
            shape.chooser,
            VillainousChoiceChooser::IteratedCreaturesController
        );
        assert_eq!(
            render_token_slice(shape.chooser_tokens),
            "that creature's controller"
        );
        assert_eq!(
            render_token_slice(shape.first_mode_tokens),
            "That creature becomes a 1/1 white Human creature and loses all abilities"
        );
        assert_eq!(
            render_token_slice(shape.second_mode_tokens),
            "you create a token that's a copy of it"
        );
        let VillainousChoiceModeProgram::SharedSubjectPair(first_mode) = shape.first_mode_program
        else {
            panic!("expected shared-subject first mode");
        };
        assert_eq!(
            render_token_slice(first_mode.subject_tokens),
            "That creature"
        );
        assert_eq!(
            render_token_slice(first_mode.first_action_tokens),
            "becomes a 1/1 white Human creature"
        );
        assert_eq!(
            render_token_slice(first_mode.second_action_tokens),
            "loses all abilities"
        );
        assert_eq!(
            shape.second_mode_program,
            VillainousChoiceModeProgram::Direct(shape.second_mode_tokens)
        );
    }

    #[test]
    fn rejects_non_up_to_target_count_and_wrong_iteration_surface() {
        let exact = lex_line(
            "Choose four target creatures you don't control. For each of them, that creature's controller faces a villainous choice — Sacrifice it, or lose 2 life.",
            0,
        )
        .unwrap();
        assert!(parse_villainous_choice_statement_tokens(&exact).is_none());

        let wrong_iteration = lex_line(
            "Choose up to four target creatures you don't control. For each target, that creature's controller faces a villainous choice — Sacrifice it, or lose 2 life.",
            0,
        )
        .unwrap();
        assert!(parse_villainous_choice_statement_tokens(&wrong_iteration).is_none());
    }

    #[test]
    fn parses_each_opponent_villainous_choice_as_typed_modes() {
        let tokens = lex_line(
            "Then each opponent faces a villainous choice — That player discards a card, or you may put a Construct, Robot, or Vehicle card from your hand onto the battlefield.",
            0,
        )
        .unwrap();
        let shape = parse_villainous_choice_player_statement_tokens(&tokens).unwrap();
        assert!(shape.leading_then);
        assert_eq!(
            shape.iteration,
            VillainousChoicePlayerIteration::EachOpponent
        );
        assert_eq!(render_token_slice(shape.chooser_tokens), "each opponent");
        assert_eq!(shape.minimum_life_lost_this_turn, None);
        assert_eq!(
            render_token_slice(shape.first_mode_tokens),
            "That player discards a card"
        );
        assert_eq!(
            render_token_slice(shape.second_mode_tokens),
            "you may put a Construct, Robot, or Vehicle card from your hand onto the battlefield"
        );
        assert_eq!(
            shape.first_mode_program,
            VillainousChoiceModeProgram::Direct(shape.first_mode_tokens)
        );
        assert_eq!(
            shape.second_mode_program,
            VillainousChoiceModeProgram::Direct(shape.second_mode_tokens)
        );
    }

    #[test]
    fn parses_life_loss_qualified_each_opponent_choice() {
        let tokens = lex_line(
            "Then each opponent who lost 3 or more life this turn faces a villainous choice — You draw a card, or that player discards a card.",
            0,
        )
        .unwrap();
        let shape = parse_villainous_choice_player_statement_tokens(&tokens).unwrap();

        assert!(shape.leading_then);
        assert_eq!(
            shape.iteration,
            VillainousChoicePlayerIteration::EachOpponent
        );
        assert_eq!(shape.minimum_life_lost_this_turn, Some(3));
        assert_eq!(render_token_slice(shape.chooser_tokens), "each opponent");
    }

    #[test]
    fn parses_target_opponent_villainous_choice_without_widening_the_chooser() {
        let tokens = lex_line(
            "Then target opponent faces a villainous choice — They discard three cards, or you may cast a spell from your hand without paying its mana cost.",
            0,
        )
        .unwrap();
        let shape = parse_villainous_choice_player_statement_tokens(&tokens).unwrap();
        assert!(shape.leading_then);
        assert_eq!(
            shape.iteration,
            VillainousChoicePlayerIteration::TargetOpponent
        );
        assert_eq!(render_token_slice(shape.chooser_tokens), "target opponent");
        assert_eq!(
            render_token_slice(shape.first_mode_tokens),
            "They discard three cards"
        );
        assert_eq!(
            render_token_slice(shape.second_mode_tokens),
            "you may cast a spell from your hand without paying its mana cost"
        );

        let untargeted = lex_line(
            "Then an opponent faces a villainous choice — They discard three cards, or you draw three cards.",
            0,
        )
        .unwrap();
        assert!(parse_villainous_choice_player_statement_tokens(&untargeted).is_none());
    }
}
