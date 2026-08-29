use crate::effect::Value;
use crate::target::{ObjectFilter, PlayerFilter};
use winnow::combinator::{alt, opt};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;

use super::super::super::super::lexer::{LexStream, OwnedLexToken};
use super::super::super::primitives;

#[derive(Debug, Clone, PartialEq)]
pub struct TriggeredSpellOpponentDamageShape {
    pub amount: Value,
}

fn parse_triggered_spell_opponent_damage_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<TriggeredSpellOpponentDamageShape> {
    primitives::phrase(&[
        "that", "spell", "deals", "damage", "to", "each", "opponent", "equal", "to",
    ])
    .parse_next(input)?;
    opt(primitives::kw("the")).parse_next(input)?;
    primitives::phrase(&["number", "of"]).parse_next(input)?;
    primitives::kw("instant").parse_next(input)?;
    alt((primitives::kw("and"), primitives::kw("or"))).parse_next(input)?;
    primitives::phrase(&["sorcery", "spells"]).parse_next(input)?;
    alt((
        primitives::phrase(&["you've", "cast", "this", "turn"]),
        primitives::phrase(&["youve", "cast", "this", "turn"]),
        primitives::phrase(&["you", "have", "cast", "this", "turn"]),
        primitives::phrase(&["its", "controller", "has", "cast", "this", "turn"]),
        primitives::phrase(&[
            "that",
            "spell's",
            "controller",
            "has",
            "cast",
            "this",
            "turn",
        ]),
        primitives::phrase(&[
            "that",
            "spells",
            "controller",
            "has",
            "cast",
            "this",
            "turn",
        ]),
    ))
    .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;

    Ok(TriggeredSpellOpponentDamageShape {
        amount: Value::SpellsCastThisTurnMatching {
            player: PlayerFilter::You,
            filter: ObjectFilter::instant_or_sorcery(),
            exclude_source: false,
        },
    })
}

pub fn parse_triggered_spell_opponent_damage_shape(
    tokens: &[OwnedLexToken],
) -> Option<TriggeredSpellOpponentDamageShape> {
    primitives::parse_all(
        tokens,
        parse_triggered_spell_opponent_damage_lexed,
        "triggered-spell-opponent-damage",
    )
    .ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex_line;

    #[test]
    fn parses_triggering_spell_damage_directly_to_matching_turn_count() {
        let tokens = lex_line(
            "That spell deals damage to each opponent equal to the number of instant and sorcery spells you've cast this turn.",
            0,
        )
        .expect("triggering spell damage fixture");
        let parsed = parse_triggered_spell_opponent_damage_shape(&tokens)
            .expect("triggering spell damage shape");

        let Value::SpellsCastThisTurnMatching { player, filter, .. } = parsed.amount else {
            panic!("expected matching spell-count value");
        };
        assert_eq!(player, PlayerFilter::You);
        assert_eq!(
            filter.card_types,
            vec![
                crate::types::CardType::Instant,
                crate::types::CardType::Sorcery
            ]
        );
    }
}
