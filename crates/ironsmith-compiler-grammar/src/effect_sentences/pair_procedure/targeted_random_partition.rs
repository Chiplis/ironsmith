//! A random subset of an announced graveyard set and its exact complement.
use super::*;
use crate::cards::builders::ObjectChoiceEffectAst;

pub(super) fn read(sentences: &[SentenceInput], index: usize) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let (Some(first), Some(second)) = (sentences.get(index), sentences.get(index + 1)) else {
        return Ok(None);
    };
    let first_tokens = first.lowered();
    if !first_tokens.first().is_some_and(|token| token.is_word("choose")) {
        return Ok(None);
    }
    // The complete second statement belongs to this rule. A word-only view
    // cannot silently erase mana symbols, counters, parentheses or operators.
    let second_tokens = second.lexed();
    let second_tokens = if second_tokens.last().is_some_and(|token| token.kind == crate::lexer::TokenKind::Period) {
        &second_tokens[..second_tokens.len() - 1]
    } else { second_tokens };
    if second_tokens.iter().any(|token| !matches!(token.kind,
        crate::lexer::TokenKind::Word | crate::lexer::TokenKind::Number)) {
        return Ok(None);
    }
    let words = crate::lexer::token_word_refs(second.lowered());
    let ["return", amount, "of", "them", "at", "random", "to", "the", "battlefield",
        "and", "put", "the", rest, "on", "the", "bottom", "of", "your", "library"] = words.as_slice() else {
        return Ok(None);
    };
    if !matches!(*rest, "other" | "rest") { return Ok(None); }
    let Some(amount) = crate::util::parse_number_word_u32(amount).filter(|amount| *amount > 0) else {
        return Ok(None);
    };
    let target = crate::util::parse_target_phrase(&first_tokens[1..])?;
    let TargetAst::WithCount(inner, count) = &target else { return Ok(None); };
    let TargetAst::Object(filter, Some(_), _) = inner.as_ref() else { return Ok(None); };
    if count.dynamic_x || count.random || count.max != Some(count.min)
        || count.min <= amount as usize || filter.zone != Some(Zone::Graveyard)
        || filter.owner != Some(PlayerFilter::You)
    { return Ok(None); }
    let pool = helper_tag_for_tokens(first_tokens, "announced_pool");
    let chosen = helper_tag_for_tokens(second.lowered(), "random_subset");
    let remainder = helper_tag_for_tokens(second.lowered(), "unselected_remainder");
    Ok(Some(vec![
        EffectAst::TagAffected {
            effect: Box::new(EffectAst::subject_verb_explicit_target_only(target)),
            tag: crate::tag::TagRef::of(pool.clone()),
        },
        EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseTaggedObjectsInZone {
            filter: ObjectFilter::tagged(pool.clone()),
            count: ChoiceCount::exactly(amount as usize).at_random(),
            player: PlayerAst::You,
            tag: crate::tag::TagRef::of(chosen.clone()),
            zone: Zone::Graveyard,
        }),
        // Freeze the complement before replacement effects can change the
        // chosen cards' destination or keep them in their original zone.
        EffectAst::subject_verb_tag_matching_objects(
            ObjectFilter::tagged(pool).not_tagged(chosen.clone()),
            vec![Zone::Graveyard],
            crate::tag::TagRef::of(remainder.clone()),
        ),
        EffectAst::subject_verb_return_to_battlefield(
            TargetAst::Tagged(crate::tag::TagRef::of(chosen), None),
            false, false, false, ReturnControllerAst::Preserve, None,
        ),
        EffectAst::subject_verb_move_all_to_zone(
            TargetAst::Object(ObjectFilter::default().in_zone(Zone::Graveyard).match_tagged(
                remainder, crate::target::TaggedOpbjectRelation::SameObjectId,
            ), None, None),
            Zone::Library, false, ReturnControllerAst::Preserve, false, None,
        ),
    ]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn declaration_random_subset_and_captured_complement_have_distinct_typed_owners() {
        let first = crate::lexer::lex_line("Choose three target creature cards in your graveyard", 0).unwrap();
        let second = crate::lexer::lex_line("Return two of them at random to the battlefield and put the other on the bottom of your library", 0).unwrap();
        let sentences = [SentenceInput::from_lexed(&first), SentenceInput::from_lexed(&second)];
        let effects = read(&sentences, 0).unwrap().unwrap();
        let [EffectAst::TagAffected { tag: pool, .. },
            EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseTaggedObjectsInZone { filter, count, tag: selected, zone, .. }),
            EffectAst::SubjectVerb(SubjectVerbEffectAst { action: SubjectVerbActionAst::TagMatchingObjects { filter: complement, tag: rest, .. }, .. }),
            EffectAst::SubjectVerb(SubjectVerbEffectAst { action: SubjectVerbActionAst::ZoneMoves(crate::cards::builders::ZoneMoveActionAst::ReturnToBattlefield { target: TargetAst::Tagged(returned, _), .. }), .. }),
            EffectAst::SubjectVerb(SubjectVerbEffectAst { action: SubjectVerbActionAst::ZoneMoves(crate::cards::builders::ZoneMoveActionAst::MoveToZone { target: TargetAst::Object(bottom, None, _), zone: destination, .. }), .. })] = effects.as_slice() else {
                panic!("expected declaration, random choice, captured complement, and two moves: {effects:?}");
            };
        assert_eq!(*count, ChoiceCount::exactly(2).at_random());
        assert_eq!(*zone, Zone::Graveyard);
        assert_eq!(filter, &ObjectFilter::tagged(pool.clone()));
        assert_eq!(complement, &ObjectFilter::tagged(pool.clone()).not_tagged(selected.clone()));
        assert_ne!(pool, selected);
        assert_ne!(selected, rest);
        assert_eq!(returned, selected);
        assert_eq!(bottom, &ObjectFilter::default().in_zone(Zone::Graveyard).match_tagged(
            rest.clone(), crate::target::TaggedOpbjectRelation::SameObjectId,
        ));
        assert_eq!(*destination, Zone::Library);
    }

    #[test]
    fn random_partition_reader_does_not_erase_nonword_payloads_or_trailing_instructions() {
        let first = crate::lexer::lex_line("Choose three target creature cards in your graveyard", 0).unwrap();
        for suffix in ["{G}", "+", "(1)", ";", "and draw a card"] {
            let second = crate::lexer::lex_line(&format!(
                "Return two of them at random to the battlefield and put the other on the bottom of your library {suffix}"), 0).unwrap();
            let sentences = [SentenceInput::from_lexed(&first), SentenceInput::from_lexed(&second)];
            assert!(read(&sentences, 0).unwrap().is_none(), "{suffix}");
        }
    }
}
