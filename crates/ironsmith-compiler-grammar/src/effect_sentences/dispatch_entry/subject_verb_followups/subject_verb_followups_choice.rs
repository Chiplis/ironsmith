use crate::cards::builders::ForEachEffectAst;
use super::*;

pub(super) fn rewrite_each_player_choice_complement_chooser(effect: &mut EffectAst) -> bool {
    let EffectAst::ForEach(ForEachEffectAst::ForEachPlayer { effects }) = effect else {
        return false;
    };
    let effects = match effects.as_mut_slice() {
        [EffectAst::CommaThen { effects }] => effects,
        _ => effects,
    };
    let Some((sacrifice, choices)) = effects.split_last_mut() else {
        return false;
    };
    if choices.is_empty() {
        return false;
    }

    let mut keep_tag = None::<TagKey>;
    for choice in choices {
        let EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects {
            filter,
            player,
            tag,
            ..
        }) = choice
        else {
            return false;
        };
        if *player != PlayerAst::That
            || filter.zone != Some(Zone::Battlefield)
            || filter.controller != Some(PlayerFilter::IteratedPlayer)
        {
            return false;
        }
        if let Some(expected) = keep_tag.as_ref() {
            if *expected != tag.key {
                return false;
            }
        } else {
            keep_tag = Some(tag.clone().into());
        }
    }
    let Some(keep_tag) = keep_tag else {
        return false;
    };
    let valid_complement = matches!(
        sacrifice,
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            subject: SubjectVerbSubjectAst { player: PlayerAst::That, .. },
            action: SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::SacrificeAll { filter }),
            ..
        }) if filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag == keep_tag
                && constraint.relation == TaggedOpbjectRelation::IsNotTaggedObject
        })
    );
    if !valid_complement {
        return false;
    }

    let choice_count = effects.len() - 1;
    for choice in effects.iter_mut().take(choice_count) {
        let EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects { player, .. }) = choice else {
            unreachable!("choice-complement shape was validated above");
        };
        *player = PlayerAst::You;
    }
    true
}

/// Materialize a chooser-only self replacement for a preceding per-player
/// choose-and-sacrifice-complement procedure. The replacement changes who
/// makes each selection; the iterated player's eligible permanents and their
/// sacrifice of the unchosen remainder remain unchanged.
pub(super) fn pre_rule_choose_for_each_player_instead(
    state: &mut SentenceDispatchState<'_>,
    _sentences: &[SentenceInput],
    _sentence_idx: usize,
    sentence_tokens: &[OwnedLexToken],
) -> Result<Option<PreParseFollowupResult>, CardTextError> {
    let Some((condition_tokens, replacement_tokens)) =
        grammar::split_lexed_once_on_delimiter(sentence_tokens, TokenKind::Comma)
    else {
        return Ok(None);
    };
    let replacement_words = crate::lexer::token_word_refs(replacement_tokens);
    if !crate::word_primitives::parse_sequence_complete(
        &replacement_words,
        &[
            "you",
            "choose",
            "the",
            "permanents",
            "for",
            "each",
            "player",
            "instead",
        ],
    ) {
        return Ok(None);
    }
    let Some(predicate) = parse_trailing_if_predicate_lexed(condition_tokens) else {
        return Ok(None);
    };
    let Some(default) = state.effects.last().cloned() else {
        return Ok(None);
    };
    let mut replacement = default.clone();
    if !rewrite_each_player_choice_complement_chooser(&mut replacement) {
        return Ok(None);
    }

    state.effects.pop();
    state.effects.push(EffectAst::SelfReplacement {
        predicate,
        if_true: vec![replacement],
        if_false: vec![default],
        attach_to_previous_ability: false,
    });
    Ok(Some(PreParseFollowupResult::Handled {
        consumed_sentences: 1,
        route: None,
    }))
}

pub(super) fn target_is_explicitly_chosen(target: &TargetAst) -> bool {
    match target {
        TargetAst::AnyTarget(span)
        | TargetAst::AnyOtherTarget(span)
        | TargetAst::AttackedPlayerOrPlaneswalker(span)
        | TargetAst::Spell(span) => span.is_some(),
        TargetAst::Player(_, span)
        | TargetAst::PlayerOrPlaneswalker(_, span)
        | TargetAst::ObjectOrPlayer(_, _, span) => span.is_some(),
        TargetAst::Object(_, target_span, _) => target_span.is_some(),
        TargetAst::WithCount(inner, _) | TargetAst::WithCountValue(inner, _, _) => {
            target_is_explicitly_chosen(inner)
        }
        TargetAst::Source(_) | TargetAst::Tagged(_, _) => false,
    }
}
