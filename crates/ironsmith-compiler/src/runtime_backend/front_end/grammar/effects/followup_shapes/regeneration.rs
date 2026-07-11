use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CantBeRegeneratedSubject {
    It,
    They,
    CreatureDestroyedThisWay,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CantBeRegeneratedFollowupShape {
    pub(crate) subject: CantBeRegeneratedSubject,
    pub(crate) this_turn: bool,
}

fn regeneration_subject<'a>(input: &mut LexStream<'a>) -> WResult<CantBeRegeneratedSubject> {
    alt((
        primitives::kw("it").value(CantBeRegeneratedSubject::It),
        primitives::kw("they").value(CantBeRegeneratedSubject::They),
        alt((
            primitives::phrase(&["creature", "destroyed", "this", "way"]),
            primitives::phrase(&["creatures", "destroyed", "this", "way"]),
            primitives::phrase(&["a", "creature", "destroyed", "this", "way"]),
        ))
        .value(CantBeRegeneratedSubject::CreatureDestroyedThisWay),
    ))
    .parse_next(input)
}

fn cant<'a>(input: &mut LexStream<'a>) -> WResult<()> {
    alt((
        primitives::kw("cant"),
        primitives::kw("can't"),
        primitives::kw("cannot"),
    ))
    .void()
    .parse_next(input)
}

fn parse_cant_be_regenerated_followup_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<CantBeRegeneratedFollowupShape> {
    let subject = regeneration_subject.parse_next(input)?;
    cant.parse_next(input)?;
    primitives::phrase(&["be", "regenerated"]).parse_next(input)?;
    let this_turn = opt(primitives::phrase(&["this", "turn"]))
        .parse_next(input)?
        .is_some();
    primitives::sentence_end().parse_next(input)?;
    Ok(CantBeRegeneratedFollowupShape { subject, this_turn })
}

pub(crate) fn parse_cant_be_regenerated_followup(
    tokens: &[OwnedLexToken],
) -> Option<CantBeRegeneratedFollowupShape> {
    primitives::parse_all(
        tokens,
        parse_cant_be_regenerated_followup_lexed,
        "can't-be-regenerated followup",
    )
    .ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::front_end::lexer::lex_line;

    #[test]
    fn parses_pronoun_and_destroyed_this_way_regeneration_followups() {
        let they = lex_line("They can't be regenerated.", 0).unwrap();
        assert_eq!(
            parse_cant_be_regenerated_followup(&they),
            Some(CantBeRegeneratedFollowupShape {
                subject: CantBeRegeneratedSubject::They,
                this_turn: false,
            })
        );

        let this_turn = lex_line(
            "A creature destroyed this way cannot be regenerated this turn.",
            0,
        )
        .unwrap();
        assert_eq!(
            parse_cant_be_regenerated_followup(&this_turn),
            Some(CantBeRegeneratedFollowupShape {
                subject: CantBeRegeneratedSubject::CreatureDestroyedThisWay,
                this_turn: true,
            })
        );
    }
}
