use winnow::combinator::{alt, eof, opt, peek, repeat_till};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::{any, rest};

use crate::target::PlayerFilter;

use super::super::super::lexer::{LexStream, OwnedLexToken, render_token_slice};
use super::super::primitives;
use super::{
    PlayerAchievementAst, PlayerAchievementConditionAst, PlayerStatusAst, StatusConditionStateAst,
    StatusConditionSubjectAst, SubjectDescriptorConditionSubjectAst, SubjectStatusConditionAst,
};

#[derive(Debug, Clone, Copy)]
pub(super) struct PlayerStatusTokenShape<'a> {
    pub(super) subject_tokens: Option<&'a [OwnedLexToken]>,
    pub(super) status: PlayerStatusAst,
}

pub(super) fn parse_subject_status(tokens: &[OwnedLexToken]) -> Option<SubjectStatusConditionAst> {
    let tokens = trim_clause(tokens);
    parse_subject_status_with_copula(tokens).or_else(|| parse_subject_status_without_copula(tokens))
}

pub(super) fn parse_subject_descriptor_subject(
    tokens: &[OwnedLexToken],
) -> Option<SubjectDescriptorConditionSubjectAst> {
    let tokens = trim_clause(tokens);
    let kind = primitives::parse_all(
        tokens,
        alt((
            primitives::phrase(&["enchanted", "permanent"])
                .value(SubjectDescriptorConditionSubjectAst::EnchantedPermanent),
            primitives::any_phrase(&[
                &["equipped", "creature"],
                &["equipped", "permanent"],
                &["enchanted", "artifact"],
                &["enchanted", "creature"],
                &["enchanted", "land"],
            ])
            .value(SubjectDescriptorConditionSubjectAst::AttachedObject),
        )),
        "condition descriptor subject",
    )
    .ok()?;
    Some(kind)
}

pub(super) fn parse_player_status_tokens(
    tokens: &[OwnedLexToken],
) -> Option<PlayerStatusTokenShape<'_>> {
    let tokens = trim_clause(tokens);
    let mut shortcut = LexStream::new(tokens);
    if alt((primitives::kw("you're"), primitives::kw("youre")))
        .parse_next(&mut shortcut)
        .is_ok()
    {
        let status_tokens = take_remaining(&mut shortcut).ok()?;
        return Some(PlayerStatusTokenShape {
            subject_tokens: None,
            status: parse_player_status_tail_tokens(status_tokens)?,
        });
    }

    let mut input = LexStream::new(tokens);
    let subject_tokens = take_until_player_status_action(&mut input).ok()?;
    parse_player_status_action(&mut input).ok()?;
    let status_tokens = take_remaining(&mut input).ok()?;
    Some(PlayerStatusTokenShape {
        subject_tokens: Some(subject_tokens),
        status: parse_player_status_tail_tokens(status_tokens)?,
    })
}

pub(super) fn parse_player_status_tail_tokens(tokens: &[OwnedLexToken]) -> Option<PlayerStatusAst> {
    let tokens = trim_clause(tokens);
    primitives::parse_all(tokens, parse_player_status_tail_lexed, "player status tail").ok()
}

pub(super) fn parse_player_achievement(
    tokens: &[OwnedLexToken],
) -> Option<PlayerAchievementConditionAst> {
    let tokens = trim_clause(tokens);
    let (tail, negated) = parse_achievement_head(tokens)?;
    Some(PlayerAchievementConditionAst {
        player: PlayerFilter::You,
        achievement: parse_player_achievement_tail(tail)?,
        negated,
    })
}

pub(super) fn parse_player_achievement_tail(
    tokens: &[OwnedLexToken],
) -> Option<PlayerAchievementAst> {
    let tokens = trim_clause(tokens);
    if parse_complete(tokens, parse_citys_blessing)
        || primitives::parse_prefix(tokens, parse_citys_blessing_for_each).is_some()
    {
        return Some(PlayerAchievementAst::CitysBlessing);
    }
    if parse_complete(tokens, parse_full_party) {
        return Some(PlayerAchievementAst::FullParty);
    }
    if parse_complete(tokens, parse_visited_attraction_this_turn) {
        return Some(PlayerAchievementAst::VisitedAttractionThisTurn);
    }

    let (_, dungeon_tokens) = primitives::parse_prefix(tokens, |input: &mut LexStream<'_>| {
        opt(parse_article).parse_next(input)?;
        primitives::kw("completed").void().parse_next(input)
    })?;
    parse_completed_dungeon(dungeon_tokens)
}

pub(super) fn is_you_subject(tokens: &[OwnedLexToken]) -> bool {
    parse_complete(trim_clause(tokens), primitives::kw("you").void())
}

fn parse_subject_status_with_copula(tokens: &[OwnedLexToken]) -> Option<SubjectStatusConditionAst> {
    let mut input = LexStream::new(tokens);
    let subject_tokens = repeat_till::<_, _, (), _, _, _, _>(1.., any.void(), peek(parse_copula))
        .map(|((), ())| ())
        .take()
        .parse_next(&mut input)
        .ok()?;
    parse_copula(&mut input).ok()?;
    let state = parse_status_state(&mut input).ok()?;
    parse_end(&mut input).ok()?;
    Some(SubjectStatusConditionAst {
        subject: parse_status_subject(subject_tokens)?,
        state,
    })
}

fn parse_subject_status_without_copula(
    tokens: &[OwnedLexToken],
) -> Option<SubjectStatusConditionAst> {
    let mut search_start = 0usize;
    let mut last = None;
    while search_start < tokens.len() {
        let Some((relative, state, _)) =
            primitives::find_prefix(&tokens[search_start..], || parse_status_state)
        else {
            break;
        };
        let state_token = search_start + relative;
        if state_token > 0 {
            last = Some((state_token, state));
        }
        search_start = state_token + 1;
    }
    let (state_token, state) = last?;
    Some(SubjectStatusConditionAst {
        subject: parse_status_subject(&tokens[..state_token])?,
        state,
    })
}

fn parse_status_subject(tokens: &[OwnedLexToken]) -> Option<StatusConditionSubjectAst> {
    primitives::parse_all(
        trim_clause(tokens),
        alt((
            alt((
                primitives::any_phrase(&[
                    &["this", "creature"],
                    &["this", "permanent"],
                    &["this"],
                    &["it"],
                    &["its"],
                ])
                .void(),
                any.verify(|token: &&OwnedLexToken| {
                    matches!(
                        token.parser_word_pieces(),
                        [piece] if piece.text == "it" || piece.text == "its"
                    )
                })
                .void(),
            ))
            .value(StatusConditionSubjectAst::Source),
            primitives::phrase(&["equipped", "creature"])
                .value(StatusConditionSubjectAst::EquippedCreature),
        )),
        "condition status subject",
    )
    .ok()
}

fn parse_status_state(input: &mut LexStream<'_>) -> WResult<StatusConditionStateAst> {
    alt((
        primitives::kw("equipped").value(StatusConditionStateAst::Equipped),
        primitives::kw("enchanted").value(StatusConditionStateAst::Enchanted),
        primitives::kw("tapped").value(StatusConditionStateAst::Tapped),
        primitives::kw("untapped").value(StatusConditionStateAst::Untapped),
        primitives::phrase(&["attacking", "alone"]).value(StatusConditionStateAst::AttackingAlone),
        primitives::kw("attacking").value(StatusConditionStateAst::Attacking),
        primitives::kw("monstrous").value(StatusConditionStateAst::Monstrous),
    ))
    .parse_next(input)
}

fn parse_player_status_tail_lexed(input: &mut LexStream<'_>) -> WResult<PlayerStatusAst> {
    opt(parse_article).parse_next(input)?;
    alt((
        primitives::kw("monarch").value(PlayerStatusAst::Monarch),
        primitives::kw("initiative").value(PlayerStatusAst::Initiative),
        (
            alt((primitives::kw("max"), primitives::kw("maximum"))),
            primitives::kw("speed"),
        )
            .value(PlayerStatusAst::MaxSpeed),
    ))
    .parse_next(input)
}

fn parse_achievement_head(tokens: &[OwnedLexToken]) -> Option<(&[OwnedLexToken], bool)> {
    let mut shortcut = LexStream::new(tokens);
    if alt((primitives::kw("you've"), primitives::kw("youve")))
        .parse_next(&mut shortcut)
        .is_ok()
    {
        return Some((take_remaining(&mut shortcut).ok()?, false));
    }

    for negated in [true, false] {
        let mut input = LexStream::new(tokens);
        let Ok(subject) = repeat_till::<_, _, (), _, _, _, _>(
            1..,
            any.void(),
            peek(|input: &mut LexStream<'_>| parse_achievement_action(input, negated)),
        )
        .map(|((), ())| ())
        .take()
        .parse_next(&mut input) else {
            continue;
        };
        if !is_you_subject(subject) {
            continue;
        }
        if parse_achievement_action(&mut input, negated).is_err() {
            continue;
        }
        if let Ok(tail) = take_remaining(&mut input) {
            return Some((tail, negated));
        }
    }
    None
}

fn parse_achievement_action(input: &mut LexStream<'_>, negated: bool) -> WResult<()> {
    if negated {
        alt((
            primitives::phrase(&["have", "not"]),
            alt((primitives::kw("haven't"), primitives::kw("havent"))).void(),
        ))
        .parse_next(input)
    } else {
        primitives::kw("have").void().parse_next(input)
    }
}

fn parse_citys_blessing(input: &mut LexStream<'_>) -> WResult<()> {
    opt(alt((primitives::kw("a"), primitives::kw("the")))).parse_next(input)?;
    alt((
        primitives::kw("city's"),
        primitives::kw("citys"),
        primitives::kw("city"),
    ))
    .parse_next(input)?;
    primitives::kw("blessing").void().parse_next(input)
}

fn parse_citys_blessing_for_each(input: &mut LexStream<'_>) -> WResult<()> {
    parse_citys_blessing(input)?;
    primitives::phrase(&["for", "each"]).parse_next(input)?;
    take_remaining(input).map(|_| ())
}

fn parse_full_party(input: &mut LexStream<'_>) -> WResult<()> {
    opt(alt((primitives::kw("a"), primitives::kw("the")))).parse_next(input)?;
    primitives::phrase(&["full", "party"]).parse_next(input)
}

fn parse_visited_attraction_this_turn(input: &mut LexStream<'_>) -> WResult<()> {
    primitives::kw("visited").parse_next(input)?;
    opt(alt((primitives::kw("a"), primitives::kw("an")))).parse_next(input)?;
    primitives::kw("attraction").parse_next(input)?;
    primitives::phrase(&["this", "turn"])
        .void()
        .parse_next(input)
}

fn parse_completed_dungeon(tokens: &[OwnedLexToken]) -> Option<PlayerAchievementAst> {
    let tokens = trim_clause(tokens);
    if primitives::parse_all(
        tokens,
        (opt(parse_article), primitives::kw("dungeon")).void(),
        "completed dungeon",
    )
    .is_ok()
    {
        return Some(PlayerAchievementAst::CompletedDungeon { dungeon_name: None });
    }
    let (_, name_tokens) = primitives::parse_prefix(tokens, opt(parse_article))?;
    if name_tokens.is_empty() {
        return None;
    }
    Some(PlayerAchievementAst::CompletedDungeon {
        dungeon_name: Some(render_token_slice(name_tokens).trim().to_string()),
    })
}

fn take_until_player_status_action<'a>(input: &mut LexStream<'a>) -> WResult<&'a [OwnedLexToken]> {
    repeat_till::<_, _, (), _, _, _, _>(1.., any.void(), peek(parse_player_status_action))
        .map(|((), ())| ())
        .take()
        .parse_next(input)
}

fn parse_player_status_action(input: &mut LexStream<'_>) -> WResult<()> {
    alt((
        primitives::kw("are"),
        primitives::kw("have"),
        primitives::kw("has"),
        primitives::kw("is"),
    ))
    .void()
    .parse_next(input)
}

fn parse_copula(input: &mut LexStream<'_>) -> WResult<()> {
    alt((primitives::kw("is"), primitives::kw("are")))
        .void()
        .parse_next(input)
}

fn parse_article(input: &mut LexStream<'_>) -> WResult<()> {
    alt((
        primitives::kw("a"),
        primitives::kw("an"),
        primitives::kw("the"),
    ))
    .void()
    .parse_next(input)
}

fn parse_complete<'a, O>(
    tokens: &'a [OwnedLexToken],
    parser: impl Parser<LexStream<'a>, O, winnow::error::ErrMode<winnow::error::ContextError>>,
) -> bool {
    primitives::parse_all(tokens, parser, "condition exact shape").is_ok()
}

fn take_remaining<'a>(input: &mut LexStream<'a>) -> WResult<&'a [OwnedLexToken]> {
    rest.parse_next(input)
}

fn parse_end(input: &mut LexStream<'_>) -> WResult<()> {
    eof.void().parse_next(input)
}

fn trim_clause(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    super::super::super::util::trim_edge_punctuation_tokens(tokens)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::front_end::lexer::lex_line;

    fn lex(text: &str) -> Vec<OwnedLexToken> {
        lex_line(text, 0).expect("lex fixture")
    }

    #[test]
    fn parses_typed_status_and_achievement_shapes() {
        let status = lex("This creature is tapped.");
        assert_eq!(
            parse_subject_status(&status),
            Some(SubjectStatusConditionAst {
                subject: StatusConditionSubjectAst::Source,
                state: StatusConditionStateAst::Tapped,
            })
        );

        let achievement = lex("You haven't completed a dungeon.");
        assert_eq!(
            parse_player_achievement(&achievement),
            Some(PlayerAchievementConditionAst {
                player: PlayerFilter::You,
                achievement: PlayerAchievementAst::CompletedDungeon { dungeon_name: None },
                negated: true,
            })
        );

        let attraction = lex("You've visited an Attraction this turn.");
        assert_eq!(
            parse_player_achievement(&attraction),
            Some(PlayerAchievementConditionAst {
                player: PlayerFilter::You,
                achievement: PlayerAchievementAst::VisitedAttractionThisTurn,
                negated: false,
            })
        );

        let opening = lex("You've opened an Attraction this turn.");
        assert_eq!(
            parse_player_achievement(&opening),
            None,
            "opening an Attraction is not the same history event as visiting one"
        );
    }
}
