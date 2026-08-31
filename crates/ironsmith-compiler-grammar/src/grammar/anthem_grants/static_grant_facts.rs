use winnow::combinator::{alt, opt};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;

use crate::types::SubtypeFamily;

use super::super::super::lexer::{LexStream, OwnedLexToken};
use super::super::primitives;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GrantedAlternativeCastKeyword {
    Flashback,
    Blitz,
    Emerge,
    Miracle,
    Escape,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FirstSpellEachTurnSubject;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StaticGrantDurationFact {
    UntilEndOfTurn,
}

pub fn parse_granted_alternative_cast_keyword_tokens(
    tokens: &[OwnedLexToken],
) -> Option<GrantedAlternativeCastKeyword> {
    let tokens = super::trim_anthem_clause_tokens(tokens);
    crate::grammar::primitives::probe_all(
        tokens,
        granted_alternative_cast_keyword,
        "granted-alternative-cast-keyword",
    )
}

pub fn parse_first_spell_each_turn_subject_tokens(
    tokens: &[OwnedLexToken],
) -> Option<FirstSpellEachTurnSubject> {
    let tokens = super::trim_anthem_clause_tokens(tokens);
    crate::grammar::primitives::probe_all(
        tokens,
        first_spell_each_turn_subject,
        "first-spell-each-turn-subject",
    )
}

pub fn parse_every_subtype_family_tokens(tokens: &[OwnedLexToken]) -> Option<SubtypeFamily> {
    let tokens = super::trim_anthem_clause_tokens(tokens);
    crate::grammar::primitives::probe_all(tokens, every_subtype_family, "every-subtype-family")
}

pub fn parse_static_grant_duration_fact(
    tokens: &[OwnedLexToken],
) -> Option<StaticGrantDurationFact> {
    primitives::find_prefix(tokens, || {
        primitives::phrase(&["until", "end", "of", "turn"])
            .value(StaticGrantDurationFact::UntilEndOfTurn)
    })
    .map(|(_, fact, _)| fact)
}

fn granted_alternative_cast_keyword(
    input: &mut LexStream<'_>,
) -> WResult<GrantedAlternativeCastKeyword> {
    alt((
        primitives::kw("flashback").value(GrantedAlternativeCastKeyword::Flashback),
        primitives::kw("blitz").value(GrantedAlternativeCastKeyword::Blitz),
        primitives::kw("emerge").value(GrantedAlternativeCastKeyword::Emerge),
        primitives::kw("miracle").value(GrantedAlternativeCastKeyword::Miracle),
        primitives::kw("escape").value(GrantedAlternativeCastKeyword::Escape),
    ))
    .parse_next(input)
}

fn first_spell_each_turn_subject(input: &mut LexStream<'_>) -> WResult<FirstSpellEachTurnSubject> {
    opt(primitives::kw("the")).parse_next(input)?;
    primitives::phrase(&["first", "spell", "you", "cast", "each", "turn"]).parse_next(input)?;
    Ok(FirstSpellEachTurnSubject)
}

fn every_subtype_family(input: &mut LexStream<'_>) -> WResult<SubtypeFamily> {
    primitives::kw("every").parse_next(input)?;
    let family = alt((
        primitives::kw("creature").value(SubtypeFamily::Creature),
        primitives::kw("land").value(SubtypeFamily::Land),
        primitives::kw("artifact").value(SubtypeFamily::Artifact),
        primitives::kw("enchantment").value(SubtypeFamily::Enchantment),
        primitives::kw("spell").value(SubtypeFamily::Spell),
        primitives::kw("planeswalker").value(SubtypeFamily::Planeswalker),
    ))
    .parse_next(input)?;
    alt((primitives::kw("type"), primitives::kw("types"))).parse_next(input)?;
    Ok(family)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex_line;

    fn lex(text: &str) -> Vec<OwnedLexToken> {
        lex_line(text, 0).expect("static-grant fixture should lex")
    }

    #[test]
    fn typed_static_grant_migration_parses_atomic_grant_facts() {
        assert_eq!(
            parse_granted_alternative_cast_keyword_tokens(&lex("Miracle")),
            Some(GrantedAlternativeCastKeyword::Miracle)
        );
        assert!(
            parse_first_spell_each_turn_subject_tokens(&lex("The first spell you cast each turn"))
                .is_some()
        );
        assert_eq!(
            parse_every_subtype_family_tokens(&lex("every creature type")),
            Some(SubtypeFamily::Creature)
        );
        assert_eq!(
            parse_static_grant_duration_fact(&lex(
                "Creatures you control get +1/+1 until end of turn"
            )),
            Some(StaticGrantDurationFact::UntilEndOfTurn)
        );
    }
}
