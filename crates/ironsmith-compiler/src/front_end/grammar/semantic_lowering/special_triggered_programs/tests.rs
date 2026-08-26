use super::super::super::super::lexer::lex_line;
use super::*;

fn parse(text: &str) -> Option<SpecialTriggeredProgram> {
    let tokens = lex_line(text, 0).unwrap();
    parse_special_triggered_program_tokens(&tokens)
}

#[test]
fn parses_all_special_triggered_programs() {
    assert_eq!(
        parse(
            "At the beginning of each upkeep, if another creature entered the battlefield under your control last turn, draw a card."
        ),
        Some(SpecialTriggeredProgram::PreviousTurnCreatureEntryDraw)
    );
    assert_eq!(
        parse(
            "Whenever you cast your second spell each turn, copy it, then exile the spell you cast with three time counters on it. If it doesn't have suspend, it gains suspend."
        ),
        Some(SpecialTriggeredProgram::SecondSpellSuspend)
    );
    assert_eq!(
        parse(
            "Search your library for exactly two cards not named Fblthp that have different names. An opponent chooses one of them. Put the chosen card into your hand and the other into your graveyard."
        ),
        Some(SpecialTriggeredProgram::DifferentNamesLibraryDivvy)
    );
    assert_eq!(
        parse(
            "At the beginning of each player's upkeep, that player chooses target player who controls more creatures than they do. Reveal cards from the top of their library until they reveal a creature card. That player puts that card onto the battlefield."
        ),
        Some(SpecialTriggeredProgram::OpponentCreatureMajorityConsult)
    );
    assert_eq!(
        parse(
            "At the beginning of each player's upkeep, that player chooses target player who controls more creatures than they do and is their opponent. The first player may reveal cards from the top of their library until they reveal a creature card. If the first player does, that player puts that card onto the battlefield and all other cards revealed this way into their graveyard."
        ),
        Some(SpecialTriggeredProgram::OpponentCreatureMajorityConsult)
    );
    assert_eq!(
        parse(
            "At the beginning of your end step, if a land entered the battlefield under your control this turn and you control a prime number of lands, create Primo, the Indivisible, a legendary 0/0 green and blue Fractal creature token, then put that many +1/+1 counters on it."
        ),
        Some(SpecialTriggeredProgram::PrimeControlledLandCountToken)
    );
    assert_eq!(
        parse(
            "At the beginning of each player's upkeep, that player chooses target player who controls more lands than they do and is their opponent. The first player may search their library for a basic land card, put that card onto the battlefield, then shuffle."
        ),
        Some(SpecialTriggeredProgram::OpponentLandMajoritySearch)
    );
    assert_eq!(
        parse(
            "At the beginning of each player's upkeep, that player chooses target player whose graveyard has fewer creature cards than their graveyard. Return a creature card from their graveyard to their hand."
        ),
        Some(SpecialTriggeredProgram::OpponentGraveyardMinorityReturn)
    );
    assert_eq!(
        parse(
            "At the beginning of your upkeep, discard a card at random. If you discard a creature card this way, return it from your graveyard to the battlefield unless any player pays 5 life."
        ),
        Some(SpecialTriggeredProgram::RandomDiscardCreatureReturnUnlessLife { life: 5 })
    );
    assert_eq!(
        parse(
            "At the beginning of combat on each opponent's turn, separate all creatures that player controls into two piles. Only creatures in the pile of their choice can attack this turn."
        ),
        Some(SpecialTriggeredProgram::OpponentCombatAttackPile)
    );
}
