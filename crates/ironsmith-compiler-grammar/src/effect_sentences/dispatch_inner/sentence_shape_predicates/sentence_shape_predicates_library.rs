use super::*;

pub(super) fn parse_prefix_then_look_at_top_exile_one(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    for then_idx in (1..tokens.len()).filter(|idx| tokens[*idx].is_word("then")) {
        let prefix = trim_edge_punctuation(&tokens[..then_idx]);
        let followup = trim_edge_punctuation(&tokens[then_idx + 1..]);
        if prefix.is_empty() || followup.is_empty() {
            continue;
        }
        let Some(mut looked) = parse_look_at_top_then_exile_one_sentence(&followup)? else {
            continue;
        };
        let mut effects = parse_effect_sentence_lexed_inner(&prefix)?;
        if effects.is_empty() {
            continue;
        }
        effects.append(&mut looked);
        return Ok(Some(effects));
    }
    Ok(None)
}

pub(super) fn parse_manifest_dread_graveyard_card_to_hand(
    tokens: &[OwnedLexToken],
) -> Option<Vec<EffectAst>> {
    let words = crate::lexer::token_word_refs(tokens);
    if !crate::word_primitives::parse_sequence_complete(
        &words,
        &[
            "put",
            "a",
            "card",
            "you",
            "put",
            "into",
            "your",
            "graveyard",
            "this",
            "way",
            "into",
            "your",
            "hand",
        ],
    ) {
        return None;
    }

    let mut filter =
        ObjectFilter::tagged(crate::tag::CompilerReferenceTag::ManifestDreadGraveyard.key());
    filter.zone = Some(Zone::Graveyard);
    Some(vec![EffectAst::subject_verb_move_to_zone(
        TargetAst::Object(filter, None, None),
        Zone::Hand,
        false,
        ReturnControllerAst::Preserve,
        false,
        None,
    )])
}

pub(super) fn parse_source_and_blocked_creatures_top_library_shuffle_sentence(
    tokens: &[OwnedLexToken],
) -> Option<EffectAst> {
    sentence_shapes::parse_source_blocked_library_shuffle_tokens(tokens)?;

    let mut blocked_creature = ObjectFilter::creature();
    blocked_creature.blocked_by_source = true;
    let mut moved_objects = ObjectFilter::default();
    moved_objects.any_of = vec![ObjectFilter::source(), blocked_creature];

    Some(EffectAst::ForEachObject {
        filter: moved_objects,
        effects: vec![
            EffectAst::subject_verb_move_to_zone(
                TargetAst::Tagged(crate::tag::CompilerReferenceTag::It.key(), None),
                Zone::Library,
                true,
                crate::cards::builders::ReturnControllerAst::Preserve,
                false,
                None,
            ),
            EffectAst::subject_verb(
                SubjectVerbRoleAst::LibraryOwner,
                PlayerAst::ItsOwner,
                SubjectVerbActionAst::ShuffleLibrary,
            ),
        ],
    })
}

pub(super) fn parse_put_cards_from_single_graveyard_on_bottom_owner_library_sentence(
    tokens: &[OwnedLexToken],
) -> Option<EffectAst> {
    let shape = sentence_shapes::parse_single_graveyard_library_bottom_tokens(tokens)?;
    let count = usize::try_from(shape.count).ok()?;

    let filter = ObjectFilter::default()
        .in_zone(Zone::Graveyard)
        .single_graveyard();
    Some(EffectAst::subject_verb_move_to_zone(
        TargetAst::WithCount(
            Box::new(TargetAst::Object(filter, None, None)),
            ChoiceCount::exactly(count),
        ),
        Zone::Library,
        false,
        crate::cards::builders::ReturnControllerAst::Preserve,
        false,
        None,
    ))
}

#[cfg(test)]
mod source_and_blocked_creatures_library_shuffle_tests {
    use super::*;
    use crate::util::tokenize_line;

    #[test]
    fn strict_joint_object_route_preempts_partial_put_and_rejects_changed_relation() {
        let tokens = tokenize_line(
            "Put this creature and each creature it's blocking on top of their owners' libraries, then those players shuffle.",
            0,
        );
        assert!(parse_source_and_blocked_creatures_top_library_shuffle_sentence(&tokens).is_some());
        let routed = crate::effect_sentences::parse_effect_sentence_lexed(&tokens)
            .expect("public sentence route should parse");
        let debug = format!("{routed:#?}");
        assert!(debug.contains("ForEachObject"), "{debug}");
        assert!(debug.contains("blocked_by_source: true"), "{debug}");
        assert!(debug.contains("MoveToZone"), "{debug}");
        assert!(debug.contains("ShuffleLibrary"), "{debug}");

        let changed = tokenize_line(
            "Put this creature and each creature it's blocked by on top of their owners' libraries, then those players shuffle.",
            0,
        );
        assert!(
            parse_source_and_blocked_creatures_top_library_shuffle_sentence(&changed).is_none()
        );
    }
}
