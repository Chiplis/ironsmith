//! "If the creature deals damage to a creature this turn, the creature dealt
//! damage can't be regenerated this turn. If a creature dealt damage by the
//! targeted creature would die this turn, exile that creature instead."
//! (Runesword): both riders are about creatures dealt damage this turn by the
//! referenced creature. They are given to that creature until end of turn as
//! the statics "Creatures dealt damage by this creature this turn can't be
//! regenerated this turn." (Bone Shaman's rule) and "If a creature dealt
//! damage by this creature this turn would die, exile it instead.", so each is
//! evaluated as regeneration or the death would happen (CR 701.19c, 614.1a),
//! not by a trigger that resolves after state-based actions.
use super::*;
use crate::cards::builders::{GrantedAbilityAst, StaticAbilityAst};
use crate::model::CompilerStaticAbilityCore as StaticAbility;

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
    let mut damaged = ObjectFilter::creature();
    damaged.dealt_damage_by_source_this_turn = Some(ironsmith_core::DamagedBySource::ThisCreature);
    let cant_be_regenerated = StaticAbility::restriction(
        crate::effect::Restriction::be_regenerated(damaged),
        "Creatures dealt damage by this creature this turn can't be regenerated this turn.",
    );
    let exile_instead = StaticAbility::exile_would_die_instead_with_damage_source(
        ObjectFilter::creature(),
        Some(ironsmith_core::DamagedBySource::ThisCreature),
    );
    Ok(Some(vec![EffectAst::subject_verb_grant_abilities_to_target(
        TargetAst::Tagged(crate::tag::CompilerReferenceTag::It.bind(), None),
        vec![
            GrantedAbilityAst::StaticAbility(Box::new(StaticAbilityAst::Static(
                cant_be_regenerated,
            ))),
            GrantedAbilityAst::StaticAbility(Box::new(StaticAbilityAst::Static(exile_instead))),
        ],
        crate::effect::Until::EndOfTurn,
    )]))
}
