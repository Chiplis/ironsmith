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
    let aura_effects = crate::effect_sentences::parse_effect_sentence_lexed(&aura_base).ok()?;
    let [EffectAst::SubjectVerb(aura_subject_verb)] = aura_effects.as_slice() else {
        return None;
    };
    let SubjectVerbActionAst::BecomeAuraEnchantment {
        target,
        attachment_filter,
        granted_abilities,
        ..
    } = &aura_subject_verb.action
    else {
        return None;
    };
    // The Aura has to animate the object the first sentence returned; an Aura
    // aimed anywhere else is a different line.
    if !matches!(target, TargetAst::Tagged(tag, _)
        if tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str())
    {
        return None;
    }
    if !granted_abilities.is_empty() {
        return None;
    }

    // The return is the node this line produces. Its Aura payload is assembled
    // here rather than by concatenating three parsed fragments and asking the
    // normalizer to fuse them back together: every part is already in hand, and
    // the ability loss was matched literally in the split above.
    let [EffectAst::SubjectVerb(return_subject_verb)] = effects.as_mut_slice() else {
        return None;
    };
    let SubjectVerbActionAst::ReturnToBattlefield { as_aura, .. } = &mut return_subject_verb.action
    else {
        return None;
    };
    if as_aura.is_some() {
        return None;
    }
    *as_aura = Some(crate::model::ast::ReturnAsAuraAst {
        attachment_filter: attachment_filter.clone(),
        remove_all_abilities: true,
        granted_abilities: vec![granted_ability],
    });
    Some(effects)
}
