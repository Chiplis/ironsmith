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

/// "When this creature dies, you may cast it from your graveyard as an
/// Adventure until the end of your next turn." (Mosswood Dreadknight): a
/// lasting permission over the source card restricted to its Adventure face
/// (CR 715.3); the Adventure rules then exile it as it resolves.
fn parse_adventure_until_next_turn(tokens: &[OwnedLexToken]) -> Option<EffectAst> {
    let words = crate::lexer::token_word_refs(tokens);
    let rest = words.strip_prefix(&["you", "may", "cast"][..])?;
    let rest = rest
        .strip_prefix(&["it"][..])
        .or_else(|| rest.strip_prefix(&["this", "card"][..]))?;
    if rest
        != [
            "from", "your", "graveyard", "as", "an", "adventure", "until", "the", "end", "of",
            "your", "next", "turn",
        ]
    {
        return None;
    }
    let mut effect = EffectAst::subject_verb_grant_play_tagged_until_your_next_turn(
        crate::tag::CompilerReferenceTag::SourceObject.bind(),
        PlayerAst::You,
        false,
        ironsmith_core::value_model::ManaSpendMode::Normal,
    );
    if let EffectAst::SubjectVerb(subject) = &mut effect
        && let crate::cards::builders::SubjectVerbActionAst::Grants(
            crate::cards::builders::GrantActionAst::GrantPlayTaggedUntilYourNextTurn {
                spell_filter,
                ..
            },
        ) = &mut subject.action
    {
        let mut adventure = crate::target::ObjectFilter::default();
        adventure.subtypes.push(crate::types::Subtype::Adventure);
        // "from your graveyard": the permission exists only for the card in
        // that zone (lowered as the grant's resolution-time zone check).
        adventure.zone = Some(crate::zone::Zone::Graveyard);
        *spell_filter = Some(adventure);
    }
    Some(effect)
}

pub(super) fn parse(tokens: &[OwnedLexToken]) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = crate::util::trim_edge_punctuation_tokens(tokens);
    if let Some(effect) = parse_adventure_until_next_turn(tokens) {
        return Ok(Some(effect));
    }
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
    let mut cast = EffectAst::subject_verb_cast_tagged(
        tag,
        PlayerAst::You,
        false,
        false,
        without_paying_mana_cost,
        None,
    );
    // "from your graveyard" names where the card is cast from: a card a
    // replacement put elsewhere (a discard redirected to exile) isn't cast.
    if let EffectAst::SubjectVerb(subject) = &mut cast
        && let crate::cards::builders::SubjectVerbActionAst::Stack(
            crate::cards::builders::StackActionAst::CastTagged { required_zone, .. },
        ) = &mut subject.action
    {
        *required_zone = Some(crate::zone::Zone::Graveyard);
    }
    Ok(Some(EffectAst::Permissions(PermissionEffectAst::May {
        effects: vec![cast],
    })))
}
