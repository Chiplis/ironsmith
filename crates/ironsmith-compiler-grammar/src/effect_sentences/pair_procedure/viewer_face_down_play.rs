//! "Its controller looks at the top card of that opponent's library and exiles
//! it face down. They may play that card for as long as it remains exiled.
//! [Mana of any type can be spent to cast a spell this way.]" (Gonti, Night
//! Minister): the look is private to the damaging creature's controller (not
//! the ability's controller), the face-down exile keeps that viewer's private
//! view (the engine remembers look viewers for a following face-down exile,
//! "non-owner private views are viewer-produced"), and the play permission
//! (with its any-type mana rider) belongs to the same player.
use super::*;

fn words(sentence: &SentenceInput) -> Vec<&str> {
    crate::lexer::parser_token_word_refs(sentence.lowered())
}

fn look_and_exile(sentence: &SentenceInput) -> Option<PlayerAst> {
    let words = words(sentence);
    let rest = match words.as_slice() {
        ["its", "controller", "looks", "at", "the", "top", "card", "of", rest @ ..] => rest,
        _ => return None,
    };
    let owner = match rest {
        [
            "that",
            "opponent's" | "opponents" | "opponents'" | "player's" | "players",
            "library",
            "and",
            "exiles",
            "it",
            "face",
            "down",
        ] => PlayerAst::That,
        _ => return None,
    };
    Some(owner)
}

fn play_while_exiled(sentence: &SentenceInput) -> bool {
    matches!(
        words(sentence).as_slice(),
        ["they", "may", "play", "that", "card", "for", "as", "long", "as", "it", "remains", "exiled"]
            | ["they", "may", "play", "it", "for", "as", "long", "as", "it", "remains", "exiled"]
    )
}

fn any_type_mana_rider(sentence: &SentenceInput) -> bool {
    let words = words(sentence);
    words.starts_with(&[
        "mana", "of", "any", "type", "can", "be", "spent", "to", "cast",
    ]) && matches!(
        &words[9..],
        ["a", "spell", "this", "way"] | ["that", "spell", "this", "way"] | ["it"]
    )
}

pub(super) fn read(
    sentences: &[SentenceInput],
    index: usize,
    with_mana_rider: bool,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(first) = sentences.get(index) else {
        return Ok(None);
    };
    let Some(library_owner) = look_and_exile(first) else {
        return Ok(None);
    };
    if !sentences.get(index + 1).is_some_and(play_while_exiled) {
        return Ok(None);
    }
    if with_mana_rider && !sentences.get(index + 2).is_some_and(any_type_mana_rider) {
        return Ok(None);
    }
    // The damaging creature is the triggering event's source; its controller
    // is the viewer and the permission holder.
    let holder = PlayerAst::TriggeringSourceController;
    let it = crate::tag::CompilerReferenceTag::It.bind();
    Ok(Some(vec![
        EffectAst::PlayerLooksAtTopCardsOfLibrary {
            viewer: holder,
            library_owner,
            count: Value::Fixed(1),
            tag: it.clone(),
        },
        EffectAst::subject_verb_exile(TargetAst::Tagged(it.clone(), None), true),
        EffectAst::subject_verb_grant_play_tagged_for_as_long_as_exiled(
            it,
            holder,
            true,
            false,
            if with_mana_rider {
                ironsmith_core::value_model::ManaSpendMode::AnyType
            } else {
                ironsmith_core::value_model::ManaSpendMode::Normal
            },
            None,
        ),
    ]))
}
