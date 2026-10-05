//! Name-matching discard restricted to the random subset just revealed.
use super::*;
use crate::cards::builders::{ObjectChoiceEffectAst, RevealLookActionAst};
fn complete_words(tokens: &[crate::lexer::OwnedLexToken]) -> bool {
    let tokens = if tokens.last().is_some_and(|token| token.kind == crate::lexer::TokenKind::Period) {
        &tokens[..tokens.len() - 1]
    } else { tokens };
    tokens.iter().all(|token| matches!(token.kind, crate::lexer::TokenKind::Word | crate::lexer::TokenKind::Number))
}
pub(super) fn read(sentences: &[SentenceInput], index: usize) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let (Some(first), Some(second)) = (sentences.get(index), sentences.get(index + 1)) else { return Ok(None); };
    if !complete_words(first.lexed()) || !complete_words(second.lexed()) { return Ok(None); }
    let words = crate::lexer::token_word_refs(second.lowered());
    let words = words.strip_prefix(&["then"]).unwrap_or(&words);
    if words != ["that", "player", "discards", "all", "cards", "with", "that", "name", "revealed", "this", "way"] {
        return Ok(None);
    }
    let Some(mut effects) = crate::effect_sentences::subject_verb_primitives::parse_sentence_target_player_reveals_random_card_from_hand(
        crate::effect_sentences::SubjectVerbPrimitiveClause::new(first.lowered()),
    )? else { return Ok(None); };
    let [EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects { filter, count, tag, .. }),
        EffectAst::SubjectVerb(SubjectVerbEffectAst { action: SubjectVerbActionAst::RevealLook(RevealLookActionAst::RevealTagged { tag: revealed }), .. })] = effects.as_slice() else {
        return Ok(None);
    };
    if !count.is_random() || tag != revealed || filter.zone != Some(Zone::Hand) { return Ok(None); }
    let Some(owner) = filter.owner.clone() else { return Ok(None); };
    let matched = ObjectFilter::default().in_zone(Zone::Hand).owned_by(owner)
        .match_tagged(tag.clone(), crate::target::TaggedOpbjectRelation::SameObjectId)
        .match_tagged(crate::tag::CompilerReferenceTag::ChosenName.bind(),
            crate::target::TaggedOpbjectRelation::SameNameAsTagged);
    effects.push(EffectAst::subject_verb_discard(PlayerAst::That,
        Value::Count(matched.clone()), false, false, Some(matched), None));
    Ok(Some(effects))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn x_random_reveal_and_name_match_keep_independent_tags_and_one_player() {
        let first = crate::lexer::lex_line("Target opponent reveals X cards at random from their hand", 0).unwrap();
        let second = crate::lexer::lex_line("Then that player discards all cards with that name revealed this way", 0).unwrap();
        let effects = read(&[SentenceInput::from_lexed(&first), SentenceInput::from_lexed(&second)], 0).unwrap().unwrap();
        let EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects { count, tag, filter, .. }) = &effects[0] else { panic!("random hand choice") };
        assert_eq!(*count, ChoiceCount::dynamic_x().at_random());
        assert_eq!(filter.owner, Some(PlayerFilter::target_opponent()));
        let EffectAst::SubjectVerb(SubjectVerbEffectAst { action: SubjectVerbActionAst::ZoneMoves(crate::cards::builders::ZoneMoveActionAst::Discard { count: Value::Count(counted), filter: Some(matched), .. }), .. }) = &effects[2] else { panic!("complete subset discard") };
        assert_eq!(counted, matched);
        assert!(matched.tagged_constraints.iter().any(|constraint| constraint.tag == tag.key && constraint.relation == crate::target::TaggedOpbjectRelation::SameObjectId));
        assert!(matched.tagged_constraints.iter().any(|constraint| constraint.tag.as_str() == crate::tag::CompilerReferenceTag::ChosenName.as_str() && constraint.relation == crate::target::TaggedOpbjectRelation::SameNameAsTagged));
        let malformed = crate::lexer::lex_line("Then that player discards all cards with that name revealed this way {G}", 0).unwrap();
        assert!(read(&[SentenceInput::from_lexed(&first), SentenceInput::from_lexed(&malformed)], 0).unwrap().is_none());
    }
}
