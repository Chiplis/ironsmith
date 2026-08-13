use super::super::SentenceInput;
use crate::cards::builders::{
    CardTextError, EffectAst, IfResultPredicate, ObjectFilter, SubjectVerbActionAst,
    SubjectVerbEffectAst, TriggerSpec,
};
use crate::effect_sentences;
use crate::grammar::effects::{
    ExileLibraryPlayerShape, ExilePermissionFollowupKind, parse_exile_dynamic_top_library_shape,
    parse_exile_permission_followup_shape,
};
use crate::util::strip_leading_token_words_any;
use crate::permission_helpers::parse_cast_or_play_tagged_clause;
use crate::util::helper_tag_for_tokens;
use crate::target::PlayerFilter;
use crate::types::CardType;

pub(super) fn rebind_permission_tag(
    mut permission: EffectAst,
    tag: crate::tag::TagKey,
) -> Option<EffectAst> {
    let EffectAst::SubjectVerb(SubjectVerbEffectAst { action, .. }) = &mut permission else {
        return None;
    };
    match action {
        SubjectVerbActionAst::GrantPlayTaggedUntilEndOfTurn {
            tag: permission_tag,
            ..
        }
        | SubjectVerbActionAst::GrantPlayTaggedUntilYourNextTurn {
            tag: permission_tag,
            ..
        }
        | SubjectVerbActionAst::GrantPlayTaggedForAsLongAsExiled {
            tag: permission_tag,
            ..
        } => *permission_tag = tag,
        _ => return None,
    }
    Some(permission)
}

/// Preserve the dynamic count, library owner, and exact exiled collection
/// across the persistent permission sentence. Parsing the two sentences in
/// isolation loses all three links for possessive LKI forms such as "its
/// power" and "its owner's library".
pub(crate) fn parse_dynamic_exile_top_then_play_for_as_long_as_exiled(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    // Keep authored possessives and collection cardinality. The normalized
    // sentence rewrites `its power`/`its owner's library` into standalone
    // references before this two-sentence rule can bind them to the
    // triggering object.
    let first_tokens = sentences[sentence_idx].lexed();
    let exile_body = strip_leading_token_words_any(first_tokens, &["exile"]);
    if exile_body.len() == first_tokens.len() {
        return Ok(None);
    }
    let Some(shape) = parse_exile_dynamic_top_library_shape(
        exile_body,
        crate::cards::builders::PlayerAst::Implicit,
    ) else {
        return Ok(None);
    };
    let ExileLibraryPlayerShape::Player(player) = shape.player else {
        return Ok(None);
    };
    if shape.face_down {
        return Ok(None);
    }

    let tag = helper_tag_for_tokens(first_tokens, "exiled");
    let Some(permission) = parse_cast_or_play_tagged_clause(sentences[sentence_idx + 1].lexed())?
    else {
        return Ok(None);
    };
    let Some(permission) = rebind_permission_tag(permission, tag.clone()) else {
        return Ok(None);
    };
    if !matches!(
        &permission,
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::GrantPlayTaggedForAsLongAsExiled { .. },
            ..
        })
    ) {
        return Ok(None);
    }

    Ok(Some(vec![
        EffectAst::subject_verb_exile_top_of_library(player, shape.count, vec![tag], Vec::new()),
        permission,
    ]))
}

pub(crate) fn parse_exile_top_play_then_event_followup(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first_effects =
        effect_sentences::parse_effect_sentence_lexed(sentences[sentence_idx].lowered())?;
    let [
        exile_effect @ EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::ExileTopOfLibrary { tags, .. },
            ..
        }),
    ] = first_effects.as_slice()
    else {
        return Ok(None);
    };
    let Some(exiled_tag) = tags.first().cloned() else {
        return Ok(None);
    };

    let Some(permission) = parse_cast_or_play_tagged_clause(sentences[sentence_idx + 1].lowered())?
    else {
        return Ok(None);
    };
    let Some(permission) = rebind_permission_tag(permission, exiled_tag.clone()) else {
        return Ok(None);
    };
    if !matches!(
        &permission,
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::GrantPlayTaggedUntilEndOfTurn { .. },
            ..
        })
    ) {
        return Ok(None);
    }

    let Some(shape) = parse_exile_permission_followup_shape(sentences[sentence_idx + 2].lowered())
    else {
        return Ok(None);
    };
    let followup_effects = effect_sentences::parse_effect_chain(shape.effect_tokens)?;
    if followup_effects.is_empty() {
        return Ok(None);
    }

    let mut effects = vec![exile_effect.clone()];
    match shape.kind {
        ExilePermissionFollowupKind::ReflexiveExileNonland => {
            effects.push(EffectAst::WhenResult {
                predicate: IfResultPredicate::AffectedObjectMatchesCardType {
                    card_type: CardType::Land,
                    negated: true,
                },
                effects: followup_effects,
            });
            effects.push(permission);
        }
        ExilePermissionFollowupKind::DelayedPlayCard => {
            effects.push(permission);
            let tagged = ObjectFilter::tagged(exiled_tag);
            let trigger = TriggerSpec::Either(
                Box::new(TriggerSpec::SpellCast {
                    filter: Some(tagged.clone()),
                    mana_source_filter: None,
                    caster: PlayerFilter::You,
                    timing: None,
                    during_turn: None,
                    min_spells_this_turn: None,
                    exact_spells_this_turn: None,
                    from_not_hand: false,
                }),
                Box::new(TriggerSpec::PlayerPlaysLand {
                    player: PlayerFilter::You,
                    filter: tagged,
                }),
            );
            effects.push(EffectAst::DelayedTriggerThisTurn {
                trigger,
                effects: followup_effects,
                one_shot: true,
                until_end_of_combat: false,
                attach_to_previous_ability: false,
            });
        }
    }

    Ok(Some(effects))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::front_end::lexer::{lex_line, split_lexed_sentences};

    fn parse(text: &str) -> Vec<EffectAst> {
        let tokens = lex_line(text, 0).expect("lex");
        let split = split_lexed_sentences(&tokens);
        let sentences = split
            .iter()
            .map(|tokens| SentenceInput::from_lexed(tokens))
            .collect::<Vec<_>>();
        parse_exile_top_play_then_event_followup(&sentences, 0)
            .expect("parse")
            .expect("shape")
    }

    #[test]
    fn exile_nonland_followup_is_reflexive_and_play_followup_is_delayed() {
        let reflexive = parse(
            "Exile the top card of your library. You may play that card this turn. When you exile a nonland card this way, this creature deals damage equal to its mana value to any target.",
        );
        assert!(matches!(
            reflexive.as_slice(),
            [
                EffectAst::SubjectVerb(_),
                EffectAst::WhenResult {
                    predicate: IfResultPredicate::AffectedObjectMatchesCardType {
                        card_type: CardType::Land,
                        negated: true,
                    },
                    ..
                },
                EffectAst::SubjectVerb(_),
            ]
        ));

        let delayed = parse(
            "Exile the top card of your library. You may play that card this turn. When you play a card this way, this enchantment deals 2 damage to each player.",
        );
        assert!(matches!(
            delayed.last(),
            Some(EffectAst::DelayedTriggerThisTurn {
                trigger: TriggerSpec::Either(_, _),
                ..
            })
        ));
    }

    #[test]
    fn dynamic_possessive_library_exile_reuses_collection_for_persistent_permission() {
        let tokens = lex_line(
            "Exile cards equal to its power from the top of its owner's library. You may cast spells from among those cards for as long as they remain exiled, and mana of any type can be spent to cast them.",
            0,
        )
        .expect("lex dynamic exile permission");
        let split = split_lexed_sentences(&tokens);
        let sentences = split
            .iter()
            .map(|tokens| SentenceInput::from_lexed(tokens))
            .collect::<Vec<_>>();
        let effects = parse_dynamic_exile_top_then_play_for_as_long_as_exiled(&sentences, 0)
            .expect("parse dynamic exile permission")
            .expect("linked dynamic exile permission should match");
        let [
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::ExileTopOfLibrary {
                        count,
                        tags,
                        face_down: false,
                        ..
                    },
                subject:
                    crate::cards::builders::SubjectVerbSubjectAst {
                        player: crate::cards::builders::PlayerAst::ItsOwner,
                        ..
                    },
                ..
            }),
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::GrantPlayTaggedForAsLongAsExiled {
                        tag,
                        allow_land: false,
                        ..
                    },
                ..
            }),
        ] = effects.as_slice()
        else {
            panic!("expected linked persistent dynamic exile permission: {effects:#?}");
        };
        assert_eq!(tags, std::slice::from_ref(tag));
        assert!(matches!(
            count.unhinted(),
            crate::effect::Value::PowerOf(spec)
                if matches!(spec.as_ref(), crate::target::ChooseSpec::Tagged(tag) if tag.as_str() == "triggering")
        ));

        let near_miss = lex_line(
            "Exile cards equal to its power from the top of your library. You may cast spells from among those cards this turn.",
            0,
        )
        .expect("lex temporary near miss");
        let split = split_lexed_sentences(&near_miss);
        let sentences = split
            .iter()
            .map(|tokens| SentenceInput::from_lexed(tokens))
            .collect::<Vec<_>>();
        assert!(
            parse_dynamic_exile_top_then_play_for_as_long_as_exiled(&sentences, 0)
                .expect("near miss should remain parseable")
                .is_none(),
            "temporary permissions must not be promoted to persistent permissions"
        );
    }
}
