//! "If the creature deals damage to a creature this turn, the creature dealt
//! damage can't be regenerated this turn. If a creature dealt damage by the
//! targeted creature would die this turn, exile that creature instead."
//! (Runesword): both riders are about creatures dealt damage this turn by the
//! referenced creature. They are turn-scoped effects of the resolving ability
//! (a regeneration restriction and a would-die replacement over creatures
//! that creature dealt damage to this turn), so each is evaluated as
//! regeneration or the death would happen (CR 701.19c, 614.1a) and persists
//! after the referenced creature leaves the battlefield.
use super::*;

fn is_cant_be_regenerated_rider(words: &[&str]) -> bool {
    matches!(
        words,
        [
            "if",
            "the" | "that",
            "creature",
            "deals",
            "damage",
            "to",
            "a",
            "creature",
            "this",
            "turn",
            "the" | "that",
            "creature",
            "dealt",
            "damage",
            "cant" | "can't",
            "be",
            "regenerated",
            "this",
            "turn",
        ]
    )
}

fn is_exile_instead_rider(words: &[&str]) -> bool {
    matches!(
        words,
        [
            "if",
            "a",
            "creature",
            "dealt",
            "damage",
            "by",
            "the" | "that",
            "targeted" | "enchanted" | "equipped",
            "creature",
            "would",
            "die",
            "this",
            "turn",
            "exile",
            "that" | "the",
            "creature",
            "instead",
        ] | [
            "if",
            "a",
            "creature",
            "dealt",
            "damage",
            "by",
            "the" | "that",
            "targeted",
            "creature",
            "would",
            "die",
            "this",
            "turn",
            "exile",
            "it",
            "instead",
        ]
    )
}

pub(super) fn read(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let (Some(first), Some(second)) = (sentences.get(sentence_idx), sentences.get(sentence_idx + 1))
    else {
        return Ok(None);
    };
    let first_words = crate::lexer::token_word_refs(first.lowered());
    let second_words = crate::lexer::token_word_refs(second.lowered());
    if !is_cant_be_regenerated_rider(&first_words) || !is_exile_instead_rider(&second_words) {
        return Ok(None);
    }
    // Both riders are turn-scoped effects of the resolving ability about
    // creatures the referenced creature deals damage to this turn. They are
    // not abilities of that creature: they keep applying after it leaves the
    // battlefield (CR 611.2a). The regeneration rule is a restriction checked
    // when regeneration would apply (CR 701.19c); the exile is a replacement
    // of the death event (CR 614.1a).
    let damaged_by_it = ObjectFilter::creature().match_tagged(
        crate::tag::CompilerReferenceTag::It.bind(),
        crate::filter::TaggedOpbjectRelation::WasDealtDamageByTaggedThisTurn,
    );
    Ok(Some(vec![
        EffectAst::subject_verb_cant(
            crate::effect::Restriction::be_regenerated(damaged_by_it.clone()),
            crate::effect::Until::EndOfTurn,
            None,
        ),
        EffectAst::subject_verb_register_future_zone_replacement(
            damaged_by_it,
            Some(crate::zone::Zone::Battlefield),
            Some(crate::zone::Zone::Graveyard),
            crate::zone::Zone::Exile,
            crate::cards::builders::ZoneReplacementDurationAst::UntilEndOfTurn,
            crate::cards::builders::FutureZoneReplacementCausePolicyAst::Any,
            false,
        ),
    ]))
}
