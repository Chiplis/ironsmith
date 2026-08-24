use super::*;

pub(super) fn exact_atomic_return_as_aura_bundle(
    effect_parse_tokens: &[OwnedLexToken],
) -> Option<Vec<EffectAst>> {
    let sentences = split_lexed_sentences(effect_parse_tokens);
    let [return_sentence, aura_sentence] = sentences.as_slice() else {
        return None;
    };
    let mut effects = crate::effect_sentences::parse_effect_sentence_lexed(return_sentence).ok()?;
    // The ordinary complete-sentence dispatcher may claim the trailing
    // outside-quote ability loss before the preceding Aura animation. Split
    // only the exact authored conjunction after the balanced quoted grant,
    // then feed both typed leaves to the normal AST fusion pass.
    let mut in_quote = false;
    let loss_start = aura_sentence.iter().enumerate().find_map(|(idx, token)| {
        if token.kind == TokenKind::Quote {
            in_quote = !in_quote;
            return None;
        }
        (!in_quote
            && token.is_word("and")
            && matches!(
                token_word_refs(&aura_sentence[idx + 1..]).as_slice(),
                ["it", "loses", "all", "other", "abilities"]
            ))
        .then_some(idx)
    })?;
    let aura_prefix = trim_lexed_commas(&aura_sentence[..loss_start]);
    let loss_suffix = trim_lexed_commas(&aura_sentence[loss_start + 1..]);
    let quote_positions = aura_prefix
        .iter()
        .enumerate()
        .filter_map(|(idx, token)| (token.kind == TokenKind::Quote).then_some(idx))
        .collect::<Vec<_>>();
    let [open_quote, close_quote] = quote_positions.as_slice() else {
        return None;
    };
    let quoted_ability_tokens = &aura_prefix[*open_quote + 1..*close_quote];
    let quoted_words = token_word_refs(quoted_ability_tokens);
    let granted_ability =
        crate::effect_sentences::parse_granted_activated_or_triggered_ability_for_gain(
            quoted_ability_tokens,
            &quoted_words,
        )
        .ok()??;
    // Parse the Aura animation without the quoted rule, then put the rule on
    // the typed Aura payload. This avoids letting the colon inside the quoted
    // activation turn the entire authored sentence into an activated line.
    let mut aura_base = aura_prefix[..*open_quote].to_vec();
    while aura_base
        .last()
        .is_some_and(|token| token.kind == TokenKind::Comma || token.is_word("and"))
    {
        aura_base.pop();
    }
    let mut aura_effects = crate::effect_sentences::parse_effect_sentence_lexed(&aura_base).ok()?;
    let [EffectAst::SubjectVerb(aura_subject_verb)] = aura_effects.as_mut_slice() else {
        return None;
    };
    let SubjectVerbActionAst::BecomeAuraEnchantment {
        granted_abilities, ..
    } = &mut aura_subject_verb.action
    else {
        return None;
    };
    if !granted_abilities.is_empty() {
        return None;
    }
    granted_abilities.push(granted_ability);
    let loss_effects = crate::effect_sentences::parse_effect_sentence_lexed(loss_suffix).ok()?;
    aura_effects.extend(loss_effects);
    match aura_effects.as_slice() {
        [
            EffectAst::Coordinated {
                effects: coordinated,
                leading_duration: false,
                result_conjunction: false,
            },
        ] => effects.extend(coordinated.iter().cloned()),
        _ => effects.extend(aura_effects),
    }
    let effects = crate::effect_ast_normalization::normalize_effects_ast(&effects);
    let [
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::ReturnToBattlefield {
                    as_aura: Some(as_aura),
                    ..
                },
            ..
        }),
    ] = effects.as_slice()
    else {
        return None;
    };
    if !as_aura.remove_all_abilities || as_aura.granted_abilities.is_empty() {
        return None;
    }
    Some(effects)
}
