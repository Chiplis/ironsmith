use super::*;
use crate::cards::builders::DelayedEffectAst;

pub(super) fn parse_exile_collection_each_upkeep_return_bundle(
    exile_sentence: &[OwnedLexToken],
    upkeep_sentence: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if bundle_grammar::parse_collection_scoped_each_upkeep_return_shape(
        exile_sentence,
        upkeep_sentence,
    )
    .is_none()
    {
        return Ok(None);
    }

    let exile_effects = effect_sentences::parse_effect_sentence_lexed(exile_sentence)?;
    let [exile_effect] = exile_effects.as_slice() else {
        return Ok(None);
    };
    if !matches!(
        exile_effect,
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ExileAll {
                face_down: false,
                ..
            }),
            ..
        })
    ) {
        return Ok(None);
    }

    let exiled_tag = crate::tag::CompilerReferenceTag::SourceExiled.bind();
    let chosen_tag = crate::tag::CompilerReferenceTag::DelayedOwnedExiledChoice.bind();
    let mut owned_exiled_filter = ObjectFilter::default();
    owned_exiled_filter.zone = Some(Zone::Exile);
    owned_exiled_filter.owner = Some(PlayerFilter::Active);
    owned_exiled_filter
        .tagged_constraints
        .push(TaggedObjectConstraint {
            tag: exiled_tag.clone().into(),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });

    let delayed_effects = vec![
        EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects {
            filter: owned_exiled_filter,
            count: ChoiceCount::exactly(1),
            count_value: None,
            player: PlayerAst::Active,
            tag: chosen_tag.clone(),
        }),
        EffectAst::subject_verb_return_to_battlefield(
            TargetAst::Tagged(chosen_tag, span_from_tokens(upkeep_sentence)),
            false,
            false,
            false,
            ReturnControllerAst::Owner,
            None,
        ),
    ];

    Ok(Some(vec![
        exile_effect.clone(),
        EffectAst::Delayed(DelayedEffectAst::DelayedTriggerForDuration {
            trigger: crate::cards::builders::TriggerSpec::BeginningOfUpkeep(PlayerFilter::Any),
            effects: delayed_effects,
            one_shot: false,
            duration: crate::effect::Until::Forever,
            either_of_watched_objects: false,
            while_any_tagged_object_in_zone: Some((exiled_tag, Zone::Exile)),
        }),
    ]))
}

/// "Target spell's controller exiles it with X delay counters on it. At the
/// beginning of each of that player's upkeeps, if that card is exiled,
/// remove a delay counter from it. If the card has no delay counters on it,
/// the player puts it onto the stack as a copy of the original spell."
/// (Ertai's Meddling.)
///
/// The target spell's announced choices are kept with its card before it
/// leaves the stack; a recurring delayed trigger, live while that card stays
/// in exile, removes a counter at each upkeep of the spell's controller
/// ("that player"), and once none remain puts the card back onto the stack
/// as a copy of the original spell (not cast; CR 707.10).
pub(super) fn parse_retained_spell_copy_bundle(
    exile_sentence: &[OwnedLexToken],
    upkeep_sentence: &[OwnedLexToken],
    copy_sentence: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    use crate::cards::builders::{ConditionalEffectAst, PredicateAst};
    let words = |tokens: &[OwnedLexToken]| -> Vec<String> {
        crate::lexer::token_word_refs(tokens)
            .into_iter()
            .map(|word| word.replace(['\'', '’'], ""))
            .collect()
    };
    let possessive = |word: &str| word.trim_end_matches('s').to_string();

    // "target spell's controller exiles it with <amount> <kind> counters on it"
    let exile_words = words(exile_sentence);
    let [target, spell, controller, exiles, it, with, ..] = exile_words.as_slice() else {
        return Ok(None);
    };
    if target != "target"
        || possessive(spell) != "spell"
        || controller != "controller"
        || exiles != "exiles"
        || it != "it"
        || with != "with"
        || !exile_words.ends_with(&["on".to_string(), "it".to_string()])
    {
        return Ok(None);
    }
    let Some(with_idx) = exile_sentence.iter().position(|token| token.is_word("with")) else {
        return Ok(None);
    };
    let after_with = &exile_sentence[with_idx + 1..];
    let Some((amount, used)) = crate::util::parse_value(after_with) else {
        return Ok(None);
    };
    let kind_tokens = &after_with[used..];
    let Some(counter_idx) = kind_tokens
        .iter()
        .position(|token| token.is_word("counters") || token.is_word("counter"))
    else {
        return Ok(None);
    };
    let Some(kind) = crate::util::parse_counter_type_from_tokens(&kind_tokens[..=counter_idx])
    else {
        return Ok(None);
    };
    if words(&kind_tokens[counter_idx + 1..]) != ["on", "it"] {
        return Ok(None);
    }

    // "at the beginning of each of that player's upkeeps, if that card is
    // exiled, remove a <kind> counter from it"
    let upkeep_words = words(upkeep_sentence);
    let head = ["at", "the", "beginning", "of", "each", "of", "that"];
    if upkeep_words.len() < 10
        || upkeep_words[..head.len()] != head
        || possessive(&upkeep_words[7]) != "player"
        || upkeep_words[8] != "upkeeps"
        || upkeep_words[9..] .get(..5).is_none_or(|guard| guard != ["if", "that", "card", "is", "exiled"])
    {
        return Ok(None);
    }
    let Some(remove_idx) = upkeep_sentence.iter().position(|token| token.is_word("remove")) else {
        return Ok(None);
    };
    let remove_tokens = &upkeep_sentence[remove_idx..];
    let remove_words = words(remove_tokens);
    if remove_words.len() < 4
        || remove_words[remove_words.len() - 2..] != ["from", "it"]
        || crate::util::parse_counter_type_from_tokens(remove_tokens) != Some(kind)
    {
        return Ok(None);
    }

    // "if the card has no <kind> counters on it, the player puts it onto the
    // stack as a copy of the original spell"
    let copy_words = words(copy_sentence);
    const COPY_TAIL: [&str; 14] = [
        "the", "player", "puts", "it", "onto", "the", "stack", "as", "a", "copy", "of", "the",
        "original", "spell",
    ];
    if copy_words.len() < COPY_TAIL.len() + 6
        || copy_words.first().map(String::as_str) != Some("if")
        || copy_words[copy_words.len() - COPY_TAIL.len()..] != COPY_TAIL
        || !copy_words.windows(2).any(|pair| pair == ["has", "no"])
    {
        return Ok(None);
    }
    let Some(no_idx) = copy_sentence.iter().position(|token| token.is_word("no")) else {
        return Ok(None);
    };
    if crate::util::parse_counter_type_from_tokens(&copy_sentence[no_idx + 1..]) != Some(kind) {
        return Ok(None);
    }

    let spell_tag = crate::util::helper_tag_for_tokens(exile_sentence, "retained_spell");
    let exiled_tag = crate::util::helper_tag_for_tokens(exile_sentence, "retained_spell_card");
    let controller_tag =
        crate::util::helper_tag_for_tokens(exile_sentence, "retained_spell_controller");
    let target_spell =
        parse_target_phrase(&crate::lexer::synthetic_word_tokens(["target", "spell"]))?;
    let remove_effects = effect_sentences::parse_effect_sentence_lexed(remove_tokens)?;
    if remove_effects.is_empty() {
        return Ok(None);
    }
    let mut delayed_effects = remove_effects;
    delayed_effects.push(EffectAst::Conditionals(ConditionalEffectAst::Conditional {
        predicate: PredicateAst::ValueComparison {
            left: crate::effect::Value::CountersOn(
                Box::new(crate::target::ChooseSpec::Tagged(exiled_tag.clone().into())),
                Some(kind),
            ),
            operator: crate::effect::ValueComparisonOperator::Equal,
            right: crate::effect::Value::Fixed(0),
        },
        if_true: vec![EffectAst::PutRetainedSpellCopyOntoStack {
            target: TargetAst::Tagged(exiled_tag.clone(), span_from_tokens(copy_sentence)),
        }],
        if_false: Vec::new(),
    }));

    Ok(Some(vec![
        EffectAst::TagAffected {
            effect: Box::new(EffectAst::subject_verb_target_only(target_spell)),
            tag: spell_tag.clone(),
        },
        EffectAst::RetainSpellChoices {
            target: TargetAst::Tagged(spell_tag.clone(), None),
            controller_tag: controller_tag.clone(),
        },
        EffectAst::TagAffected {
            effect: Box::new(EffectAst::subject_verb_exile(
                TargetAst::Tagged(spell_tag, None),
                false,
            )),
            tag: exiled_tag.clone(),
        },
        EffectAst::subject_verb_put_counters(
            kind,
            amount,
            TargetAst::Tagged(exiled_tag.clone(), None),
            None,
            false,
        ),
        // "that player": the target spell's controller, tagged by
        // `RetainSpellChoicesEffect` and captured by the delayed trigger.
        EffectAst::Delayed(DelayedEffectAst::DelayedTriggerForDuration {
            trigger: crate::cards::builders::TriggerSpec::BeginningOfUpkeep(
                PlayerFilter::TaggedPlayer(controller_tag.into()),
            ),
            effects: delayed_effects,
            one_shot: false,
            duration: crate::effect::Until::Forever,
            either_of_watched_objects: false,
            while_any_tagged_object_in_zone: Some((exiled_tag, Zone::Exile)),
        }),
    ]))
}
