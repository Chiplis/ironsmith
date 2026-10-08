//! "you may cast this card from your graveyard" inside a triggered ability
//! (Syrix, Carrier of the Flame; Sproutback Trudge) and "you may cast it from
//! your graveyard" (Oskar, Rubbish Reclaimer): a cast during the ability's
//! resolution (CR 608.2g), from the graveyard the card is in. An ability that
//! moves its own card out of the graveyard functions there (CR 113.6k; see
//! the trigger functional-zone facts).
use crate::cards::builders::{CardTextError, EffectAst, PermissionEffectAst, PlayerAst};
use crate::grammar::primitives;
use crate::lexer::OwnedLexToken;
use winnow::Parser as _;
use winnow::combinator::{alt, opt};

#[derive(Clone, Copy)]
enum Cast {
    SourceCard,
    Antecedent,
}

pub(super) fn parse(tokens: &[OwnedLexToken]) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = crate::util::trim_edge_punctuation_tokens(tokens);
    let Some((cast, without_paying_mana_cost)) = primitives::probe_all(
        tokens,
        (
            primitives::phrase(&["you", "may", "cast"]),
            alt((
                (
                    primitives::kw("this"),
                    alt((
                        primitives::kw("card"),
                        primitives::kw("creature"),
                        primitives::kw("permanent"),
                    )),
                )
                    .value(Cast::SourceCard),
                primitives::kw("it").value(Cast::Antecedent),
            )),
            primitives::phrase(&["from", "your", "graveyard"]),
            opt(primitives::phrase(&["without", "paying", "its", "mana", "cost"])),
            primitives::sentence_end(),
        )
            .map(|((), cast, (), free, ())| (cast, free.is_some())),
        "graveyard resolution cast",
    ) else {
        return Ok(None);
    };
    let tag = match cast {
        Cast::SourceCard => crate::tag::CompilerReferenceTag::SourceObject.bind(),
        Cast::Antecedent => crate::tag::CompilerReferenceTag::It.bind(),
    };
    Ok(Some(EffectAst::Permissions(PermissionEffectAst::May {
        effects: vec![EffectAst::subject_verb_cast_tagged(
            tag,
            PlayerAst::You,
            false,
            false,
            without_paying_mana_cost,
            None,
        )],
    })))
}
