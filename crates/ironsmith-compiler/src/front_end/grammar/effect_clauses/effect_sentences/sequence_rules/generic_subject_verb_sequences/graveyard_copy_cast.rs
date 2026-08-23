use super::super::SentenceInput;
use crate::activation_and_restrictions::{
    build_may_cast_tagged_effect, parse_may_cast_it_sentence,
};
use crate::cards::builders::{
    CardTextError, EffectAst, PlayerAst, SubjectVerbActionAst, SubjectVerbEffectAst,
    SubjectVerbSubjectAst, TagKey, TargetAst,
};
use crate::effect::Value;
use crate::effect_sentences;
use crate::util::helper_tag_for_tokens;
use crate::zone::Zone;

fn target_is_in_graveyard(target: &TargetAst) -> bool {
    match target {
        TargetAst::Object(filter, _, _) => {
            filter.zone == Some(Zone::Graveyard)
                || (filter.zone.is_none()
                    && !filter.any_of.is_empty()
                    && filter
                        .any_of
                        .iter()
                        .all(|arm| arm.zone == Some(Zone::Graveyard)))
        }
        TargetAst::WithCount(inner, _) | TargetAst::WithCountValue(inner, _, _) => {
            target_is_in_graveyard(inner)
        }
        _ => false,
    }
}

/// Propagate a trailing shared graveyard qualifier across a coordinated card
/// union. The ordinary branch parser can attach `from your graveyard` only to
/// the final arm of `Assassin card or card with freerunning`; within this
/// exact graveyard copy family the qualifier grammatically scopes both arms.
fn normalize_shared_graveyard_union_target(target: &mut TargetAst) {
    match target {
        TargetAst::WithCount(inner, _) | TargetAst::WithCountValue(inner, _, _) => {
            normalize_shared_graveyard_union_target(inner);
        }
        TargetAst::Object(filter, _, _) if filter.zone.is_none() && !filter.any_of.is_empty() => {
            let scoped_owner = filter
                .any_of
                .iter()
                .find_map(|arm| (arm.zone == Some(Zone::Graveyard)).then(|| arm.owner.clone()));
            let Some(scoped_owner) = scoped_owner else {
                return;
            };
            if filter.any_of.iter().any(|arm| {
                !matches!(arm.zone, None | Some(Zone::Graveyard))
                    || (arm.zone == Some(Zone::Graveyard) && arm.owner != scoped_owner)
                    || (arm.zone.is_none() && arm.owner.is_some())
            }) {
                return;
            }
            for arm in &mut filter.any_of {
                if arm.zone.is_none() {
                    arm.zone = Some(Zone::Graveyard);
                    arm.owner = scoped_owner.clone();
                }
            }
        }
        _ => {}
    }
}

fn normalize_shared_graveyard_union_exile(effect: &mut EffectAst) {
    let effect = match effect {
        EffectAst::TagAffected { effect, .. } => effect.as_mut(),
        effect => effect,
    };
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action: SubjectVerbActionAst::Exile { target, .. },
        ..
    }) = effect
    else {
        return;
    };
    normalize_shared_graveyard_union_target(target);
}

fn is_exact_graveyard_exile(effect: &EffectAst) -> bool {
    matches!(
        effect,
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            subject: SubjectVerbSubjectAst {
                player: PlayerAst::Implicit | PlayerAst::You,
                ..
            },
            action: SubjectVerbActionAst::Exile {
                target,
                face_down: false,
                source_top_only: false,
                ..
            },
            ..
        }) if target_is_in_graveyard(target)
    )
}

fn exact_tagged_graveyard_exile_tag(effect: &EffectAst) -> Option<TagKey> {
    let EffectAst::TagAffected { effect, tag } = effect else {
        return None;
    };
    let tag = tag.clone();
    is_exact_graveyard_exile(effect).then_some(tag)
}

fn is_exact_tagged_graveyard_exile(effect: &EffectAst, expected_tag: &TagKey) -> bool {
    exact_tagged_graveyard_exile_tag(effect).as_ref() == Some(expected_tag)
}

fn exact_single_card_copy_tag(effect: &EffectAst) -> Option<TagKey> {
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        subject:
            SubjectVerbSubjectAst {
                player: PlayerAst::Implicit | PlayerAst::You,
                ..
            },
        action:
            SubjectVerbActionAst::CopySpell {
                target: TargetAst::Tagged(tag, _),
                target_reference_kind: None,
                target_reference_pronoun: true,
                all_matches: false,
                count: Value::Fixed(1),
                count_surface: None,
                player: PlayerAst::Implicit | PlayerAst::You,
                may_choose_new_targets: false,
                choose_new_target_singular: false,
                removed_supertypes,
                set_colors: None,
                added_card_types,
                added_subtypes,
                set_base_power_toughness: None,
            },
        ..
    }) = effect
    else {
        return None;
    };
    if !removed_supertypes.is_empty() || !added_card_types.is_empty() || !added_subtypes.is_empty()
    {
        return None;
    }
    Some(tag.clone())
}

fn exact_terminal_card_copy_tag(effect: &EffectAst) -> Option<TagKey> {
    exact_single_card_copy_tag(effect)
        .filter(|tag| crate::util::is_sentence_helper_tag(tag.as_str(), "exiled"))
}

/// Composes the card-copy procedure
///
/// `exile <target card> from a graveyard and copy it. You may cast the copy`
///
/// without lowering the card copy as a stack-spell copy. `CastTagged`'s typed
/// `as_copy` mode is the existing executable primitive for copying a card in a
/// non-stack zone and casting that copy. The shared exile tag preserves the
/// exact selected card and the optional cast remains a `May` action.
pub fn parse_graveyard_exile_copy_then_may_cast_copy(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Ok(mut first_effects) =
        effect_sentences::parse_effect_sentence_lexed(sentences[sentence_idx].lowered())
    else {
        return Ok(None);
    };
    for effect in &mut first_effects {
        normalize_shared_graveyard_union_exile(effect);
    }
    let [
        EffectAst::Coordinated {
            effects: coordinated,
            leading_duration: false,
            result_conjunction: false,
        },
    ] = first_effects.as_mut_slice()
    else {
        return Ok(None);
    };
    let [exile_effect, copy_effect] = coordinated.as_slice() else {
        return Ok(None);
    };
    let Some(exiled_tag) = exact_terminal_card_copy_tag(copy_effect) else {
        return Ok(None);
    };
    if !is_exact_tagged_graveyard_exile(exile_effect, &exiled_tag) {
        return Ok(None);
    }

    let Some(mut cast) = parse_may_cast_it_sentence(sentences[sentence_idx + 1].lowered()) else {
        return Ok(None);
    };
    if !cast.as_copy || !matches!(cast.player, PlayerAst::Implicit | PlayerAst::You) {
        return Ok(None);
    }
    cast.tag = exiled_tag;

    let exile_effect = coordinated.remove(0);
    Ok(Some(vec![
        exile_effect,
        build_may_cast_tagged_effect(&cast),
    ]))
}

/// Variant with the copy instruction gated by the immediately preceding
/// exile: `Exile ... . If you do, copy it. You may cast the copy.`
///
/// `CastTagged(as_copy)` remains the executable card-copy primitive. Keeping
/// it inside `IfResult::Did` makes the cast proposal contingent on the exile
/// while avoiding an invalid stack-spell `CopySpell` action for a card in
/// exile.
pub fn parse_graveyard_exile_if_copy_then_may_cast_copy(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Ok(mut first_effects) =
        effect_sentences::parse_effect_sentence_lexed(sentences[sentence_idx].lowered())
    else {
        return Ok(None);
    };
    for effect in &mut first_effects {
        normalize_shared_graveyard_union_exile(effect);
    }
    let [exile_effect] = first_effects.as_mut_slice() else {
        return Ok(None);
    };
    let exiled_tag = if let Some(tag) = exact_tagged_graveyard_exile_tag(exile_effect) {
        tag
    } else if is_exact_graveyard_exile(exile_effect) {
        let tag = helper_tag_for_tokens(sentences[sentence_idx].lowered(), "exiled");
        let exile = exile_effect.clone();
        *exile_effect = EffectAst::TagAffected {
            effect: Box::new(exile),
            tag: tag.clone(),
        };
        tag
    } else {
        return Ok(None);
    };

    let Ok(copy_gate) =
        effect_sentences::parse_effect_sentence_lexed(sentences[sentence_idx + 1].lowered())
    else {
        return Ok(None);
    };
    let [
        EffectAst::IfResult {
            predicate: crate::cards::builders::IfResultPredicate::Did,
            effects: copy_effects,
        },
    ] = copy_gate.as_slice()
    else {
        return Ok(None);
    };
    let [copy_effect] = copy_effects.as_slice() else {
        return Ok(None);
    };
    let Some(copy_reference_tag) = exact_single_card_copy_tag(copy_effect) else {
        return Ok(None);
    };
    if copy_reference_tag.as_str() != crate::cards::builders::IT_TAG
        && copy_reference_tag != exiled_tag
        && !crate::util::is_sentence_helper_tag(copy_reference_tag.as_str(), "exiled")
    {
        return Ok(None);
    }

    let Some(mut cast) = parse_may_cast_it_sentence(sentences[sentence_idx + 2].lowered()) else {
        return Ok(None);
    };
    if !cast.as_copy || !matches!(cast.player, PlayerAst::Implicit | PlayerAst::You) {
        return Ok(None);
    }
    cast.tag = exiled_tag;

    Ok(Some(vec![
        exile_effect.clone(),
        EffectAst::IfResult {
            predicate: crate::cards::builders::IfResultPredicate::Did,
            effects: vec![build_may_cast_tagged_effect(&cast)],
        },
    ]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::{lex_line, split_lexed_sentences};

    fn registry_match(text: &str) -> super::super::super::SequenceRuleMatch {
        let tokens = lex_line(text, 0).expect("graveyard copy/cast fixture should lex");
        let sentences = split_lexed_sentences(&tokens)
            .into_iter()
            .map(SentenceInput::from_lexed)
            .collect::<Vec<_>>();
        super::super::super::try_parse_subject_verb_sequence_rule(&sentences, 0)
            .expect("graveyard copy/cast registry should not error")
            .expect("graveyard copy/cast registry should match")
    }

    #[test]
    fn graveyard_card_copy_cast_uses_one_tagged_copy_action_not_copy_spell() {
        let cases = [
            "Exile target noncreature, nonland card with mana value less than this creature's power from a graveyard and copy it. You may cast the copy without paying its mana cost.",
            "Exile up to one target black card from your graveyard and copy it. You may cast the copy.",
            "Exile target instant or sorcery card from a graveyard and copy it. You may cast the copy without paying its mana cost.",
            "Exile up to one target legendary or Rat card from your graveyard and copy it. You may cast the copy.",
        ];

        for text in cases {
            let matched = registry_match(text);
            assert_eq!(matched.name, "graveyard-exile-copy-cast-copy", "{text}");
            assert_eq!(matched.consumed_sentences, 2, "{text}");
            let debug = format!("{:#?}", matched.effects);
            assert!(debug.contains("Exile"), "{text}: {debug}");
            assert!(debug.contains("CastTagged"), "{text}: {debug}");
            assert!(debug.contains("as_copy: true"), "{text}: {debug}");
            assert!(
                debug.contains("__sentence_helper_exiled"),
                "{text}: {debug}"
            );
            assert!(!debug.contains("CopySpell"), "{text}: {debug}");
        }
    }

    #[test]
    fn conditional_graveyard_card_copy_cast_keeps_result_gate_and_one_copy_action() {
        let matched = registry_match(
            "Exile up to one target Assassin card or card with freerunning from your graveyard. If you do, copy it. You may cast the copy.",
        );
        assert_eq!(matched.name, "graveyard-exile-if-copy-cast-copy");
        assert_eq!(matched.consumed_sentences, 3);
        let debug = format!("{:#?}", matched.effects);
        assert!(debug.contains("IfResult"), "{debug}");
        assert!(debug.contains("CastTagged"), "{debug}");
        assert!(debug.contains("as_copy: true"), "{debug}");
        assert!(!debug.contains("CopySpell"), "{debug}");
    }
}
