use winnow::combinator::{alt, opt};
use winnow::prelude::*;
use winnow::token::any;

use crate::cards::builders::CardTextError;
use crate::runtime_backend::front_end::grammar::effects::chain_splitting;
use crate::runtime_backend::front_end::grammar::primitives;
use crate::runtime_backend::front_end::lexer::{LexStream, OwnedLexToken};
use crate::runtime_backend::front_end::shared::util::{
    parse_greater_than_or_equal_quantity_prefix, parse_quantity_comparison_prefix,
    trim_edge_punctuation_tokens,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ForEachParticipantScope {
    Opponent,
    OpponentExceptDefending,
    Player,
    PlayerExceptYou,
    PlayerExceptTarget,
    PlayerExceptItsController,
    PlayerOnYourTeam,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ForEachParticipantClauseShape<'a> {
    pub(crate) scope: ForEachParticipantScope,
    /// `Each player/opponent <verbs>` names the iterated participant as the
    /// actor. `For each player/opponent, <imperative>` keeps the ability's
    /// controller as the implicit actor.
    pub(crate) participant_is_actor: bool,
    pub(crate) inner_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct RelativeControlClauseShape<'a> {
    pub(crate) controls_most: bool,
    pub(crate) count_comparison: Option<crate::effect::Comparison>,
    pub(crate) fewer_than_most_filter_tokens: Option<&'a [OwnedLexToken]>,
    pub(crate) fewer_than_you: bool,
    pub(crate) filter_tokens: &'a [OwnedLexToken],
    pub(crate) effect_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct SourceAttackedPlayerClauseShape<'a> {
    pub(crate) effect_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum WhoClauseShape<'a> {
    TappedLandForMana {
        effect_tokens: &'a [OwnedLexToken],
    },
    Negated {
        effect_tokens: &'a [OwnedLexToken],
        tagged_filter_tokens: Option<&'a [OwnedLexToken]>,
    },
    DidThisWay {
        effect_tokens: &'a [OwnedLexToken],
        tagged_filter_tokens: Option<&'a [OwnedLexToken]>,
    },
    DidAction {
        effect_tokens: &'a [OwnedLexToken],
        implicit_player_is_you: bool,
    },
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum OpponentSpecialShape<'a> {
    IgnoreScryOrSurveil,
    ChooseReturnUnlessDraw {
        target_tokens: &'a [OwnedLexToken],
    },
    LessLifeThanYou {
        effect_tokens: &'a [OwnedLexToken],
    },
    PoisonCounters {
        count: u32,
        effect_tokens: &'a [OwnedLexToken],
    },
}

fn semantic_kw<'a>(
    expected: &'static str,
) -> impl Parser<LexStream<'a>, (), winnow::error::ErrMode<winnow::error::ContextError>> {
    any.verify(move |token: &&OwnedLexToken| {
        token.is_word(expected)
            || matches!(token.parser_word_pieces(), [piece] if piece.text == expected)
    })
    .void()
}

fn opponent_prefix<'a>(input: &mut LexStream<'a>) -> winnow::error::ModalResult<()> {
    alt((
        (
            primitives::phrase(&["for", "each"]),
            semantic_kw("opponent"),
        )
            .void(),
        (
            primitives::phrase(&["for", "each"]),
            semantic_kw("opponents"),
        )
            .void(),
        (primitives::kw("each"), semantic_kw("opponent")).void(),
        (primitives::kw("each"), semantic_kw("opponents")).void(),
    ))
    .void()
    .parse_next(input)
}

fn player_prefix<'a>(input: &mut LexStream<'a>) -> winnow::error::ModalResult<()> {
    alt((
        (primitives::phrase(&["for", "each"]), semantic_kw("player")).void(),
        (primitives::phrase(&["for", "each"]), semantic_kw("players")).void(),
        (primitives::kw("each"), semantic_kw("player")).void(),
        (primitives::kw("each"), semantic_kw("players")).void(),
    ))
    .void()
    .parse_next(input)
}

fn negated_auxiliary<'a>(input: &mut LexStream<'a>) -> winnow::error::ModalResult<()> {
    alt((
        alt((
            primitives::kw("doesn't"),
            primitives::kw("doesnt"),
            primitives::kw("don't"),
            primitives::kw("dont"),
            primitives::kw("didn't"),
            primitives::kw("didnt"),
            primitives::kw("can't"),
            primitives::kw("cant"),
            primitives::kw("cannot"),
        ))
        .void(),
        alt((
            primitives::phrase(&["does", "not"]),
            primitives::phrase(&["do", "not"]),
            primitives::phrase(&["did", "not"]),
            primitives::phrase(&["can", "not"]),
        ))
        .void(),
        alt((
            primitives::phrase(&["doesn", "t"]),
            primitives::phrase(&["don", "t"]),
            primitives::phrase(&["didn", "t"]),
            primitives::phrase(&["can", "t"]),
        ))
        .void(),
    ))
    .void()
    .parse_next(input)
}

fn tagged_action<'a>(input: &mut LexStream<'a>) -> winnow::error::ModalResult<()> {
    alt((
        primitives::kw("sacrificed"),
        primitives::kw("destroyed"),
        primitives::kw("exiled"),
        primitives::kw("discarded"),
    ))
    .void()
    .parse_next(input)
}

fn discard_action<'a>(input: &mut LexStream<'a>) -> winnow::error::ModalResult<()> {
    alt((primitives::kw("discard"), primitives::kw("discarded")))
        .void()
        .parse_next(input)
}

fn trim(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    trim_edge_punctuation_tokens(tokens)
}

pub(crate) fn parse_participant_clause_shape(
    tokens: &[OwnedLexToken],
) -> Option<ForEachParticipantClauseShape<'_>> {
    let tokens = trim(tokens);
    let tokens = primitives::parse_prefix(tokens, opt(primitives::kw("then")).void())
        .map(|(_, rest)| trim(rest))
        .unwrap_or(tokens);
    let participant_is_actor =
        primitives::parse_prefix(tokens, primitives::kw("each").void()).is_some();
    if let Some((_, rest)) = primitives::parse_prefix(tokens, opponent_prefix) {
        let mut scope = ForEachParticipantScope::Opponent;
        let mut inner_tokens = trim(rest);
        if let Some((_, rest)) = primitives::parse_prefix(
            inner_tokens,
            primitives::phrase(&["other", "than", "defending", "player"]),
        ) {
            scope = ForEachParticipantScope::OpponentExceptDefending;
            inner_tokens = trim(rest);
        }
        return Some(ForEachParticipantClauseShape {
            scope,
            participant_is_actor,
            inner_tokens,
        });
    }
    if let Some((_, rest)) = primitives::parse_prefix(
        tokens,
        (
            primitives::kw("each"),
            primitives::kw("other"),
            semantic_kw("player"),
        )
            .void(),
    ) {
        return Some(ForEachParticipantClauseShape {
            scope: ForEachParticipantScope::PlayerExceptYou,
            participant_is_actor: true,
            inner_tokens: trim(rest),
        });
    }
    let (_, rest) = primitives::parse_prefix(tokens, player_prefix)?;
    let mut scope = ForEachParticipantScope::Player;
    let mut inner_tokens = trim(rest);
    if let Some((_, rest)) = primitives::parse_prefix(
        inner_tokens,
        primitives::phrase(&["other", "than", "its", "controller"]),
    ) {
        scope = ForEachParticipantScope::PlayerExceptItsController;
        inner_tokens = trim(rest);
    }
    if let Some((_, rest)) = primitives::parse_prefix(
        inner_tokens,
        primitives::phrase(&["other", "than", "target", "player"]),
    ) {
        scope = ForEachParticipantScope::PlayerExceptTarget;
        inner_tokens = trim(rest);
    }
    if let Some((_, rest)) =
        primitives::parse_prefix(inner_tokens, primitives::phrase(&["on", "your", "team"]))
    {
        scope = ForEachParticipantScope::PlayerOnYourTeam;
        inner_tokens = trim(rest);
    }
    Some(ForEachParticipantClauseShape {
        scope,
        participant_is_actor,
        inner_tokens,
    })
}

/// Parse the source-relative participant qualifier in clauses such as
/// "each player this creature attacked this turn loses the game."
pub(crate) fn parse_source_attacked_player_clause_shape(
    tokens: &[OwnedLexToken],
) -> Option<SourceAttackedPlayerClauseShape<'_>> {
    let (_, effect_tokens) = primitives::parse_prefix(
        trim(tokens),
        (
            alt((
                primitives::phrase(&["this", "creature"]),
                primitives::phrase(&["this", "permanent"]),
                primitives::phrase(&["this", "source"]),
            )),
            primitives::phrase(&["attacked", "this", "turn"]),
        )
            .void(),
    )?;
    let effect_tokens = trim(effect_tokens);
    (!effect_tokens.is_empty()).then_some(SourceAttackedPlayerClauseShape { effect_tokens })
}

fn effect_start(tokens: &[OwnedLexToken]) -> Option<usize> {
    let mut index = 0usize;
    while index < tokens.len() {
        if tokens[index].is_word("may")
            // Search and choice programs are parsed by dedicated sentence
            // grammars, so they are intentionally absent from the generic
            // chain-verb registry. They are still valid action boundaries
            // after a participant-relative predicate:
            //
            //   each player who controls ... chooses ...
            //   each player who controls ... searches ...
            //
            // Missing either boundary makes the object-filter parser absorb
            // the action and resume at a later generic verb such as
            // `sacrifices` or `puts`.
            || tokens[index].is_word("choose")
            || tokens[index].is_word("chooses")
            || tokens[index].is_word("search")
            || tokens[index].is_word("searches")
            || chain_splitting::find_chain_verb_tokens(&tokens[index..])
                .is_some_and(|found| found.word_index == 0)
        {
            return Some(index);
        }
        index += 1;
    }
    None
}

pub(crate) fn parse_relative_control_clause_shape(
    tokens: &[OwnedLexToken],
) -> Option<RelativeControlClauseShape<'_>> {
    let (_, tail) = primitives::parse_prefix(
        trim(tokens),
        (
            primitives::kw("who"),
            alt((primitives::kw("control"), primitives::kw("controls"))),
        )
            .void(),
    )?;

    // "who controls fewer lands than the player who controls the most
    // lands ..." compares the current participant's count with a global
    // per-controller maximum. Split after the nested relative clause before
    // looking for the action verb; otherwise the inner "controls" is easily
    // mistaken for the outer action boundary.
    if let Some((_, after_fewer)) = primitives::parse_prefix(tail, primitives::kw("fewer").void())
        && let Some((than_index, _, after_most)) = primitives::find_prefix(after_fewer, || {
            primitives::phrase(&["than", "the", "player", "who", "controls", "the", "most"]).void()
        })
    {
        let filter_tokens = trim(after_fewer.get(..than_index)?);
        let split = effect_start(after_most)?;
        let most_filter_tokens = trim(after_most.get(..split)?);
        let effect_tokens = trim(after_most.get(split..)?);
        if !filter_tokens.is_empty() && !most_filter_tokens.is_empty() && !effect_tokens.is_empty()
        {
            return Some(RelativeControlClauseShape {
                controls_most: false,
                count_comparison: None,
                fewer_than_most_filter_tokens: Some(most_filter_tokens),
                fewer_than_you: false,
                filter_tokens,
                effect_tokens,
            });
        }
    }

    // The participant is compared against the ability controller using the
    // same counted object set:
    //
    //   each opponent who controls fewer creatures than you draws a card
    if let Some((_, after_fewer)) = primitives::parse_prefix(tail, primitives::kw("fewer").void())
        && let Some((than_index, _, after_you)) =
            primitives::find_prefix(after_fewer, || primitives::phrase(&["than", "you"]).void())
    {
        let filter_tokens = trim(after_fewer.get(..than_index)?);
        let split = effect_start(after_you)?;
        let effect_tokens = trim(after_you.get(split..)?);
        if !filter_tokens.is_empty() && !effect_tokens.is_empty() {
            return Some(RelativeControlClauseShape {
                controls_most: false,
                count_comparison: None,
                fewer_than_most_filter_tokens: None,
                fewer_than_you: true,
                filter_tokens,
                effect_tokens,
            });
        }
    }

    // Preserve authored numeric thresholds ("six or more lands", "four or
    // fewer lands") as an actual count comparison rather than allowing the
    // object-filter parser to discard the quantity.
    if let Ok((comparison, used)) =
        parse_quantity_comparison_prefix(tail, false, false, "for-each relative control predicate")
        && !matches!(comparison, crate::effect::Comparison::Equal(_))
    {
        let after_count = tail.get(used..)?;
        let split = effect_start(after_count)?;
        let filter_tokens = trim(after_count.get(..split)?);
        let effect_tokens = trim(after_count.get(split..)?);
        if !filter_tokens.is_empty() && !effect_tokens.is_empty() {
            return Some(RelativeControlClauseShape {
                controls_most: false,
                count_comparison: Some(comparison),
                fewer_than_most_filter_tokens: None,
                fewer_than_you: false,
                filter_tokens,
                effect_tokens,
            });
        }
    }

    let split = effect_start(tail)?;
    let mut filter_tokens = trim(tail.get(..split)?);
    let effect_tokens = trim(tail.get(split..)?);
    let mut controls_most = false;
    if let Some((_, rest)) = primitives::parse_prefix(
        filter_tokens,
        alt((
            primitives::phrase(&["the", "most"]),
            primitives::kw("most").void(),
        ))
        .void(),
    ) {
        controls_most = true;
        filter_tokens = trim(rest);
    }
    (!filter_tokens.is_empty() && !effect_tokens.is_empty()).then_some(RelativeControlClauseShape {
        controls_most,
        count_comparison: None,
        fewer_than_most_filter_tokens: None,
        fewer_than_you: false,
        filter_tokens,
        effect_tokens,
    })
}

fn tagged_filter_after_action(tokens: &[OwnedLexToken]) -> Option<&[OwnedLexToken]> {
    let (_, after_who) = primitives::parse_prefix(trim(tokens), primitives::kw("who"))?;
    let (_, after_action) = primitives::parse_prefix(after_who, tagged_action)?;
    let (way_index, _, _) =
        primitives::find_prefix(after_action, || primitives::phrase(&["this", "way"]).void())?;
    let filter = trim(after_action.get(..way_index)?);
    (!filter.is_empty()).then_some(filter)
}

fn tagged_filter_after_negation(tokens: &[OwnedLexToken]) -> Option<&[OwnedLexToken]> {
    let (_, after_who) = primitives::parse_prefix(trim(tokens), primitives::kw("who"))?;
    let (_, after_negation) = primitives::parse_prefix(after_who, negated_auxiliary)?;
    let (_, after_action) = primitives::parse_prefix(after_negation, discard_action)?;
    let (way_index, _, _) =
        primitives::find_prefix(after_action, || primitives::phrase(&["this", "way"]).void())?;
    let filter = trim(after_action.get(..way_index)?);
    (!filter.is_empty()).then_some(filter)
}

pub(crate) fn parse_who_tagged_filter_shape(tokens: &[OwnedLexToken]) -> Option<&[OwnedLexToken]> {
    tagged_filter_after_action(tokens)
}

fn tapped_land_shape(tokens: &[OwnedLexToken]) -> Option<WhoClauseShape<'_>> {
    primitives::parse_prefix(trim(tokens), primitives::kw("who"))?;
    let (_, _, rest) = primitives::find_prefix(tokens, || {
        (
            primitives::kw("tapped"),
            opt(primitives::kw("a")),
            primitives::phrase(&["land", "for", "mana", "this", "turn"]),
        )
            .void()
    })?;
    Some(WhoClauseShape::TappedLandForMana {
        effect_tokens: trim(rest),
    })
}

fn negated_shape(tokens: &[OwnedLexToken]) -> Option<WhoClauseShape<'_>> {
    let (_, after_who) = primitives::parse_prefix(trim(tokens), primitives::kw("who"))?;
    let (_, after_negation) = primitives::parse_prefix(after_who, negated_auxiliary)?;
    let effect_tokens = if let Some((_, _, rest)) =
        primitives::find_prefix(tokens, || primitives::comma().void())
    {
        trim(rest)
    } else if let Some((_, _, rest)) =
        primitives::find_prefix(tokens, || primitives::phrase(&["this", "way"]).void())
    {
        trim(rest)
    } else {
        trim(after_negation)
    };
    Some(WhoClauseShape::Negated {
        effect_tokens,
        tagged_filter_tokens: tagged_filter_after_negation(tokens),
    })
}

fn did_this_way_shape(tokens: &[OwnedLexToken]) -> Option<WhoClauseShape<'_>> {
    primitives::parse_prefix(trim(tokens), primitives::kw("who"))?;
    let (_, _, rest) =
        primitives::find_prefix(tokens, || primitives::phrase(&["this", "way"]).void())?;
    Some(WhoClauseShape::DidThisWay {
        effect_tokens: trim(rest),
        tagged_filter_tokens: tagged_filter_after_action(tokens),
    })
}

fn did_action_shape(tokens: &[OwnedLexToken]) -> Option<WhoClauseShape<'_>> {
    let (_, after_action) = primitives::parse_prefix(
        trim(tokens),
        (
            primitives::kw("who"),
            alt((
                primitives::kw("does"),
                primitives::kw("do"),
                primitives::kw("did"),
            )),
        )
            .void(),
    )?;
    let (effect_tokens, implicit_player_is_you) =
        primitives::find_prefix(tokens, || primitives::comma().void())
            .map(|(_, _, rest)| (trim(rest), true))
            .unwrap_or_else(|| (trim(after_action), false));
    Some(WhoClauseShape::DidAction {
        effect_tokens,
        implicit_player_is_you,
    })
}

pub(crate) fn parse_who_clause_shape(tokens: &[OwnedLexToken]) -> Option<WhoClauseShape<'_>> {
    tapped_land_shape(tokens)
        .or_else(|| negated_shape(tokens))
        .or_else(|| did_this_way_shape(tokens))
        .or_else(|| did_action_shape(tokens))
}

fn ignore_scry_or_surveil(tokens: &[OwnedLexToken]) -> bool {
    let Some((_, amount)) = primitives::parse_prefix(
        trim(tokens),
        alt((
            primitives::kw("scries"),
            primitives::kw("scry"),
            primitives::kw("surveils"),
            primitives::kw("surveil"),
        ))
        .void(),
    ) else {
        return false;
    };
    let Some(parsed) =
        crate::runtime_backend::front_end::grammar::leaf::parse_leaf_number_prefix_tokens(amount)
    else {
        return false;
    };
    trim(amount.get(parsed.consumed..).unwrap_or_default()).is_empty()
}

fn choose_return_unless(tokens: &[OwnedLexToken]) -> Option<&[OwnedLexToken]> {
    let (_, after_choose) = primitives::parse_prefix(trim(tokens), primitives::kw("choose"))?;
    let (return_index, _, after_return) = primitives::find_prefix(after_choose, || {
        primitives::phrase(&["then", "return"]).void()
    })?;
    let target_tokens = trim(after_choose.get(..return_index)?);
    if target_tokens.is_empty() {
        return None;
    }
    let (_, _, after_unless) =
        primitives::find_prefix(after_return, || primitives::kw("unless").void())?;
    primitives::parse_prefix(
        after_unless,
        primitives::phrase(&["its", "controller", "has", "you", "draw", "a", "card"]),
    )?;
    Some(target_tokens)
}

fn less_life(tokens: &[OwnedLexToken]) -> Option<&[OwnedLexToken]> {
    primitives::parse_prefix(
        trim(tokens),
        primitives::phrase(&["who", "has", "less", "life", "than", "you"]),
    )
    .map(|(_, rest)| trim(rest))
}

fn poison_counters(
    tokens: &[OwnedLexToken],
) -> Result<Option<(u32, &[OwnedLexToken])>, CardTextError> {
    let Some((_, after_has)) =
        primitives::parse_prefix(trim(tokens), primitives::phrase(&["who", "has"]))
    else {
        return Ok(None);
    };
    let Some((count, used)) = parse_greater_than_or_equal_quantity_prefix(
        after_has,
        false,
        false,
        "for-each poison-counter predicate",
    )?
    else {
        return Ok(None);
    };
    let Some(after_count) = after_has.get(used..) else {
        return Ok(None);
    };
    let Some((_, rest)) = primitives::parse_prefix(
        after_count,
        alt((
            primitives::phrase(&["poison", "counter"]),
            primitives::phrase(&["poison", "counters"]),
        ))
        .void(),
    ) else {
        return Ok(None);
    };
    Ok(Some((count, trim(rest))))
}

pub(crate) fn parse_opponent_special_shape(
    tokens: &[OwnedLexToken],
) -> Result<Option<OpponentSpecialShape<'_>>, CardTextError> {
    if ignore_scry_or_surveil(tokens) {
        return Ok(Some(OpponentSpecialShape::IgnoreScryOrSurveil));
    }
    if let Some(target_tokens) = choose_return_unless(tokens) {
        return Ok(Some(OpponentSpecialShape::ChooseReturnUnlessDraw {
            target_tokens,
        }));
    }
    if let Some(effect_tokens) = less_life(tokens) {
        return Ok(Some(OpponentSpecialShape::LessLifeThanYou {
            effect_tokens,
        }));
    }
    if let Some((count, effect_tokens)) = poison_counters(tokens)? {
        return Ok(Some(OpponentSpecialShape::PoisonCounters {
            count,
            effect_tokens,
        }));
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::front_end::lexer::{TokenWordView, lex_line};

    #[test]
    fn parses_participant_and_who_shapes() {
        let clause = lex_line(
            "for each opponent other than defending player who did this way draw a card",
            0,
        )
        .unwrap();
        let outer = parse_participant_clause_shape(&clause).unwrap();
        assert!(!outer.participant_is_actor);
        assert_eq!(
            outer.scope,
            ForEachParticipantScope::OpponentExceptDefending
        );
        let WhoClauseShape::DidThisWay { effect_tokens, .. } =
            parse_who_clause_shape(outer.inner_tokens).unwrap()
        else {
            panic!("expected this-way shape");
        };
        assert_eq!(
            TokenWordView::new(effect_tokens).to_word_refs(),
            vec!["draw", "a", "card"]
        );
    }

    #[test]
    fn distinguishes_participant_subjects_from_controller_imperatives() {
        let subject = lex_line("Each opponent chooses a creature", 0).unwrap();
        let imperative = lex_line("For each opponent, choose a creature", 0).unwrap();

        assert!(
            parse_participant_clause_shape(&subject)
                .unwrap()
                .participant_is_actor
        );
        assert!(
            !parse_participant_clause_shape(&imperative)
                .unwrap()
                .participant_is_actor
        );
    }

    #[test]
    fn parses_each_other_player_as_filtered_participant_subject() {
        let tokens = lex_line("Each other player draws a card", 0).unwrap();
        let shape = parse_participant_clause_shape(&tokens).unwrap();
        assert!(shape.participant_is_actor);
        assert_eq!(shape.scope, ForEachParticipantScope::PlayerExceptYou);
        assert_eq!(
            TokenWordView::new(shape.inner_tokens).to_word_refs(),
            vec!["draws", "a", "card"]
        );

        let imperative = lex_line("For each other player, draw a card", 0).unwrap();
        assert!(
            parse_participant_clause_shape(&imperative).is_none(),
            "the quantified-subject family must not claim imperative fanout"
        );
    }

    #[test]
    fn parses_source_attacked_player_qualifier_without_absorbing_action() {
        let tokens = lex_line("this creature attacked this turn loses the game", 0).unwrap();
        let shape = parse_source_attacked_player_clause_shape(&tokens).unwrap();
        assert_eq!(
            TokenWordView::new(shape.effect_tokens).to_word_refs(),
            vec!["loses", "the", "game"]
        );
    }

    #[test]
    fn parses_relative_control_and_poison_shapes() {
        let control = lex_line("who controls the most creatures draws a card", 0).unwrap();
        let shape = parse_relative_control_clause_shape(&control).unwrap();
        assert!(shape.controls_most);

        let threshold = lex_line(
            "who controls four or fewer lands may search their library",
            0,
        )
        .unwrap();
        let shape = parse_relative_control_clause_shape(&threshold).unwrap();
        assert_eq!(
            shape.count_comparison,
            Some(crate::effect::Comparison::LessThanOrEqual(4))
        );
        assert_eq!(
            TokenWordView::new(shape.filter_tokens).to_word_refs(),
            vec!["lands"]
        );
        assert_eq!(
            TokenWordView::new(shape.effect_tokens).to_word_refs(),
            vec!["may", "search", "their", "library"]
        );

        let counted_choice = lex_line(
            "who controls six or more lands chooses five lands they control and sacrifices the rest",
            0,
        )
        .unwrap();
        let shape = parse_relative_control_clause_shape(&counted_choice).unwrap();
        assert_eq!(
            TokenWordView::new(shape.filter_tokens).to_word_refs(),
            vec!["lands"]
        );
        assert_eq!(
            TokenWordView::new(shape.effect_tokens).to_word_refs(),
            vec![
                "chooses",
                "five",
                "lands",
                "they",
                "control",
                "and",
                "sacrifices",
                "the",
                "rest"
            ]
        );

        let fewer_than_most = lex_line(
            "who controls fewer lands than the player who controls the most lands searches their library",
            0,
        )
        .unwrap();
        let shape = parse_relative_control_clause_shape(&fewer_than_most).unwrap();
        assert_eq!(
            TokenWordView::new(shape.filter_tokens).to_word_refs(),
            vec!["lands"]
        );
        assert_eq!(
            TokenWordView::new(shape.fewer_than_most_filter_tokens.unwrap()).to_word_refs(),
            vec!["lands"]
        );
        assert_eq!(
            TokenWordView::new(shape.effect_tokens).to_word_refs(),
            vec!["searches", "their", "library"]
        );

        let fewer_than_you =
            lex_line("who controls fewer creatures than you draws a card", 0).unwrap();
        let shape = parse_relative_control_clause_shape(&fewer_than_you).unwrap();
        assert!(shape.fewer_than_you);
        assert_eq!(
            TokenWordView::new(shape.filter_tokens).to_word_refs(),
            vec!["creatures"]
        );
        assert_eq!(
            TokenWordView::new(shape.effect_tokens).to_word_refs(),
            vec!["draws", "a", "card"]
        );

        let poison = lex_line("who has three or more poison counters loses the game", 0).unwrap();
        assert!(matches!(
            parse_opponent_special_shape(&poison).unwrap(),
            Some(OpponentSpecialShape::PoisonCounters { count: 3, .. })
        ));
    }
}
