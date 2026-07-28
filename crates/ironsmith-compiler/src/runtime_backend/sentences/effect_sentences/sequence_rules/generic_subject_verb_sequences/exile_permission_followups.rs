use super::super::SentenceInput;
use crate::cards::builders::{
    CardTextError, EffectAst, IfResultPredicate, ObjectFilter, SubjectVerbActionAst,
    SubjectVerbEffectAst, TriggerSpec,
};
use crate::runtime_backend::effect_sentences;
use crate::runtime_backend::front_end::grammar::effects::{
    ExilePermissionFollowupKind, parse_exile_permission_followup_shape,
};
use crate::runtime_backend::permission_helpers::parse_cast_or_play_tagged_clause;
use crate::target::PlayerFilter;
use crate::types::CardType;

fn rebind_permission_tag(permission: EffectAst, tag: crate::tag::TagKey) -> Option<EffectAst> {
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action:
            SubjectVerbActionAst::GrantPlayTaggedUntilEndOfTurn {
                tag: _,
                player,
                allow_land,
                without_paying_mana_cost,
                allow_any_color_for_cast,
                while_on_top_of_library,
                free_cast_from_current_zone,
                until_source_exiles_another,
                surface,
            },
        ..
    }) = permission
    else {
        return None;
    };
    Some(EffectAst::subject_verb(
        crate::cards::builders::SubjectVerbRoleAst::Actor,
        crate::cards::builders::PlayerAst::Implicit,
        SubjectVerbActionAst::GrantPlayTaggedUntilEndOfTurn {
            tag,
            player,
            allow_land,
            without_paying_mana_cost,
            allow_any_color_for_cast,
            while_on_top_of_library,
            free_cast_from_current_zone,
            until_source_exiles_another,
            surface,
        },
    ))
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
}
