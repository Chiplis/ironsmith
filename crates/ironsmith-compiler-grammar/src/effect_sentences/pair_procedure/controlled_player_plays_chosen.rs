//! "You control that player until this finishes resolving. The player plays
//! that card if able. While doing so, the player can activate mana abilities
//! only if they're from lands that player controls and only if mana they
//! produce is spent to activate other mana abilities of lands the player
//! controls and/or to play that card. If the chosen card is cast as a spell,
//! you control the player while that spell is resolving." (Word of Command):
//! one controlled play of the chosen card (CR 722). The control lasts for the
//! rest of this resolution; the play is the referenced player's mandatory
//! cast or land play ("if able" only excuses an impossible play, CR 101.3);
//! the mana rider and the resolving-spell control ride on that play.
use super::*;
use crate::cards::builders::ControlDurationAst;

fn words(sentence: &SentenceInput) -> Vec<&str> {
    crate::lexer::parser_token_word_refs(sentence.lowered())
        .into_iter()
        .flat_map(|word| match word {
            "theyre" => vec!["they're"],
            "and/or" | "andor" => vec!["and", "or"],
            other => vec![other],
        })
        .collect()
}

fn control_until_resolved(sentence: &SentenceInput) -> bool {
    matches!(
        words(sentence).as_slice(),
        ["you", "control", "that" | "the", "player", "until", "this", "finishes", "resolving"]
            | [
                "you", "control", "that" | "the", "player", "until", "this", "spell", "finishes",
                "resolving"
            ]
    )
}

/// The play sentence; true when it names a play (lands allowed) rather than a
/// cast.
fn plays_chosen_if_able(sentence: &SentenceInput) -> Option<bool> {
    match words(sentence).as_slice() {
        ["the" | "that", "player", "plays", "that", "card", "if", "able"] => Some(true),
        ["the" | "that", "player", "casts", "that", "card", "if", "able"] => Some(false),
        _ => None,
    }
}

fn land_mana_only_rider(sentence: &SentenceInput, play_verb: &str) -> bool {
    let words = words(sentence);
    let Some(rest) = words.strip_prefix(
        &[
            "while", "doing", "so", "the", "player", "can", "activate", "mana", "abilities",
            "only", "if", "they're", "from", "lands", "that", "player", "controls", "and",
            "only", "if", "mana", "they", "produce", "is", "spent", "to", "activate", "other",
            "mana", "abilities", "of", "lands", "the", "player", "controls", "and", "or", "to",
        ][..],
    ) else {
        return false;
    };
    matches!(rest, [verb, "that", "card"] if *verb == play_verb)
}

fn control_while_cast_spell_resolves(sentence: &SentenceInput) -> bool {
    matches!(
        words(sentence).as_slice(),
        [
            "if", "the", "chosen", "card", "is", "cast", "as", "a", "spell", "you", "control",
            "the" | "that", "player", "while", "that", "spell", "is", "resolving"
        ]
    )
}

pub(super) fn read(
    sentences: &[SentenceInput],
    index: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let (Some(control), Some(play), Some(rider), Some(resolving)) = (
        sentences.get(index),
        sentences.get(index + 1),
        sentences.get(index + 2),
        sentences.get(index + 3),
    ) else {
        return Ok(None);
    };
    if !control_until_resolved(control) {
        return Ok(None);
    }
    let Some(allow_land) = plays_chosen_if_able(play) else {
        return Ok(None);
    };
    if !land_mana_only_rider(rider, if allow_land { "play" } else { "cast" })
        || !control_while_cast_spell_resolves(resolving)
    {
        return Ok(None);
    }
    // "that player" is the player the preceding instruction introduced;
    // lowering binds the iterated-player placeholder to it.
    let control = EffectAst::subject_verb_control_player(
        PlayerAst::You,
        PlayerFilter::IteratedPlayer,
        ControlDurationAst::UntilSourceFinishesResolving,
    );
    let mut cast = EffectAst::subject_verb_cast_tagged(
        crate::tag::CompilerReferenceTag::It.bind(),
        PlayerAst::That,
        allow_land,
        false,
        false,
        None,
    );
    if let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action:
            SubjectVerbActionAst::Stack(crate::cards::builders::StackActionAst::CastTagged {
                caster_land_mana_abilities_only,
                control_caster_while_spell_resolves,
                ..
            }),
        ..
    }) = &mut cast
    {
        *caster_land_mana_abilities_only = true;
        *control_caster_while_spell_resolves = true;
    } else {
        return Ok(None);
    }
    Ok(Some(vec![control, cast]))
}
