use crate::cards::builders::ForEachEffectAst;
use crate::cards::builders::LibraryActionAst;
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
        ObjectFilter::tagged(crate::tag::CompilerReferenceTag::ManifestDreadGraveyard.bind());
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

/// "Put <objects> on top of their owners' libraries, then those players
/// shuffle [their libraries]." (Gomazoa, Vortex Elemental, Void Stalker):
/// every named object moves first, then each distinct owner shuffles exactly
/// once (CR 701.24a). The moved objects share one outcome tag; the shuffle's
/// owner-of-tagged player set is deduplicated by the engine, so a player who
/// owns two of them shuffles once and shuffle triggers fire once.
pub(super) fn parse_source_and_blocked_creatures_top_library_shuffle_sentence(
    tokens: &[OwnedLexToken],
) -> Option<EffectAst> {
    use crate::grammar::primitives;
    use winnow::Parser as _;
    let tokens = trim_edge_punctuation(tokens);
    let (_, body) = primitives::parse_prefix(&tokens, primitives::kw("put"))?;
    let (top_idx, (), tail) = primitives::find_prefix(body, || {
        primitives::phrase(&["on", "top", "of", "their"])
    })?;
    let ((), _) = primitives::parse_prefix(
        tail,
        (
            winnow::combinator::alt((
                primitives::kw("owners'"),
                primitives::kw("owners"),
                primitives::kw("owner's"),
            )),
            primitives::kw("libraries"),
            winnow::combinator::opt(primitives::comma()),
            primitives::phrase(&["then", "those", "players"]),
            primitives::kw("shuffle"),
            winnow::combinator::opt(primitives::phrase(&["their", "libraries"])),
            primitives::sentence_end(),
        )
            .void(),
    )?;
    let operand = &body[..top_idx];
    let operand_words = crate::lexer::parser_token_word_refs(operand);
    let moved_tag = crate::util::helper_tag_for_tokens(&tokens, "moved_to_owners_libraries");
    let joint_relation = match operand_words.as_slice() {
        ["this", "creature", "and", "each", "creature", "it's" | "its", "blocking"] => {
            let mut blocked_creature = ObjectFilter::creature();
            blocked_creature.blocked_by_source = true;
            Some(blocked_creature)
        }
        [
            "this",
            "creature",
            "and",
            "each",
            "creature",
            "blocking",
            "or",
            "blocked",
            "by",
            "it",
        ] => {
            let mut fighting_creature = ObjectFilter::creature();
            fighting_creature.in_combat_with_source = true;
            Some(fighting_creature)
        }
        _ => None,
    };
    let mut effects = Vec::new();
    if let Some(related) = joint_relation {
        let mut moved_objects = ObjectFilter::default();
        moved_objects.any_of = vec![ObjectFilter::source(), related];
        effects.push(EffectAst::TagAffected {
            tag: moved_tag.clone(),
            effect: Box::new(EffectAst::ForEach(ForEachEffectAst::ForEachObject {
                filter: moved_objects,
                effects: vec![EffectAst::subject_verb_move_to_zone(
                    TargetAst::Tagged(crate::tag::CompilerReferenceTag::It.bind(), None),
                    Zone::Library,
                    true,
                    crate::cards::builders::ReturnControllerAst::Preserve,
                    false,
                    None,
                )],
            })),
        });
    } else {
        // Two independently named objects ("this creature and target
        // creature"): each is its own reference with the shared destination.
        let and_positions = operand
            .iter()
            .enumerate()
            .filter(|(_, token)| token.is_word("and"))
            .map(|(index, _)| index)
            .collect::<Vec<_>>();
        let [and_idx] = and_positions.as_slice() else {
            return None;
        };
        let (left, right) = (&operand[..*and_idx], &operand[*and_idx + 1..]);
        let left_is_reference = left.first().is_some_and(|token| token.is_word("target"))
            || (left.first().is_some_and(|token| token.is_word("this")) && left.len() <= 3);
        if !left_is_reference
            || !right.first().is_some_and(|token| token.is_word("target"))
            || operand.iter().any(|token| token.is_comma())
        {
            return None;
        }
        let destination = body.get(top_idx..top_idx + 6)?;
        for half in [left, right] {
            let mut half_tokens = half.to_vec();
            half_tokens.extend_from_slice(destination);
            let moved = crate::grammar::primitives::probe_shape(
                crate::effect_sentences::verb_handlers::parse_put_into_hand(&half_tokens, None),
            )?;
            effects.push(EffectAst::TagAffected {
                tag: moved_tag.clone(),
                effect: Box::new(moved),
            });
        }
    }
    effects.push(EffectAst::subject_verb(
        SubjectVerbRoleAst::LibraryOwner,
        PlayerAst::ItsOwner,
        SubjectVerbActionAst::Library(LibraryActionAst::ShuffleLibrary),
    ));
    Some(EffectAst::Sequence { effects })
}

pub(super) fn parse_put_cards_from_single_graveyard_on_bottom_owner_library_sentence(
    tokens: &[OwnedLexToken],
) -> Option<EffectAst> {
    let shape = sentence_shapes::parse_single_graveyard_library_bottom_tokens(tokens)?;
    let count = crate::util::narrowed_usize(shape.count)?;

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
