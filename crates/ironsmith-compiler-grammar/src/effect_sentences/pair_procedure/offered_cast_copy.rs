//! "Target opponent whose turn it is may cast a spell from their hand without
//! paying its mana cost. If they do, you copy that spell. You may choose new
//! targets for the copy. If they don't, you draw a card." (Cmdr. Troi,
//! Outgoing Empath)
//!
//! A player is offered a cast while the ability resolves (CR 608.2g); the
//! spell that offer puts on the stack is the one "that spell" names, so the
//! offer tags what it cast. The copy is made on the stack and its controller
//! may choose new targets for it (CR 707.10, 707.10c). "If they do" / "If
//! they don't" read whether the offered cast happened; the fallback sentence
//! is optional.
use super::*;
use crate::cards::builders::{ConditionalEffectAst, PermissionEffectAst};

fn words(sentence: &SentenceInput) -> Vec<&str> {
    crate::lexer::parser_token_word_refs(crate::lexer::trim_lexed_commas(sentence.lowered()))
}

/// The effects of the offer sentence, with the offered cast tagged.
fn offer_effects(
    sentence: &SentenceInput,
    cast_tag: &crate::tag::TagRef,
) -> Option<Vec<EffectAst>> {
    fn is_offered_cast(effect: &EffectAst) -> bool {
        matches!(
            effect,
            EffectAst::Permissions(PermissionEffectAst::MayCastMatchingSpellWithoutPayingManaCost { .. })
        )
    }
    // Wrap the one offered cast in place, through an optional "<player> may"
    // wrapper. The general sentence parser already names the wrapper's player
    // as the caster and hand owner.
    fn tag_offered_cast(effect: &mut EffectAst, cast_tag: &crate::tag::TagRef) -> bool {
        if is_offered_cast(effect) {
            let inner = std::mem::replace(effect, EffectAst::Sequence { effects: Vec::new() });
            *effect = EffectAst::TagAffected {
                effect: Box::new(inner),
                tag: cast_tag.clone(),
            };
            return true;
        }
        match effect {
            EffectAst::Permissions(
                PermissionEffectAst::MayByPlayer { effects, .. } | PermissionEffectAst::May { effects },
            ) => match effects.as_mut_slice() {
                [only] => tag_offered_cast(only, cast_tag),
                _ => false,
            },
            _ => false,
        }
    }
    let tokens = crate::util::trim_edge_punctuation_tokens(sentence.lowered());
    let mut effects = super::super::dispatch_entry::parse_target_active_opponent_sentence(tokens)
        .or_else(|| {
            crate::grammar::primitives::probe_shape(super::super::parse_effect_sentence_lexed(tokens))
        })?;
    let last = effects.last_mut()?;
    tag_offered_cast(last, cast_tag).then_some(effects)
}

/// "If they don't, <effects>." — the fallback when the offered cast didn't
/// happen.
fn declined_effects(sentence: &SentenceInput) -> Option<Vec<EffectAst>> {
    let tokens = crate::util::trim_edge_punctuation_tokens(sentence.lowered());
    let head = match words(sentence).as_slice() {
        ["if", "they", "don't" | "dont", _, ..] => 3,
        ["if", "they", "do", "not", _, ..] => 4,
        _ => return None,
    };
    let view = crate::lexer::TokenWordView::new(tokens);
    let start = view.token_start_indices().get(head).copied()?;
    let rest = crate::util::trim_commas(&tokens[start..]);
    let effects = crate::grammar::primitives::probe_shape(super::super::parse_effect_sentence_lexed(&rest))?;
    (!effects.is_empty()).then_some(effects)
}

pub(super) fn read(
    sentences: &[SentenceInput],
    index: usize,
    with_fallback: bool,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let count = if with_fallback { 4 } else { 3 };
    let Some(group) = sentences.get(index..index + count) else {
        return Ok(None);
    };
    // 2. "If they do, [you] copy that spell."
    if !matches!(
        words(&group[1]).as_slice(),
        ["if", "they", "do", "you", "copy", "that", "spell"] | ["if", "they", "do", "copy", "that", "spell"]
    ) {
        return Ok(None);
    }
    // 3. "You may choose new targets for the copy."
    let Some(retarget) = crate::grammar::effects::clause_pattern_shapes::parse_copy_retarget_shape_tokens(group[2].lowered()) else {
        return Ok(None);
    };
    if !retarget.may_choose || !retarget.has_new {
        return Ok(None);
    }
    // 4. "If they don't, <effects>."
    let declined = if with_fallback {
        let Some(declined) = declined_effects(&group[3]) else {
            return Ok(None);
        };
        Some(declined)
    } else {
        None
    };
    // 1. "<player> may cast a spell ... without paying its mana cost."
    let cast_tag = crate::tag::TagRef::of(helper_tag_for_tokens(group[0].lowered(), "offered_cast"));
    let Some(mut effects) = offer_effects(&group[0], &cast_tag) else {
        return Ok(None);
    };
    for sentence in group {
        crate::word_claims::claim_tokens(sentence.lowered());
    }
    let copy = EffectAst::subject_verb_copy_spell(
        TargetAst::Tagged(cast_tag, None),
        Value::Fixed(1),
        PlayerAst::You,
        true,
        retarget.single_target,
        Vec::new(),
    )
    .with_copy_target_reference_kind(crate::filter::StackObjectKind::Spell);
    effects.push(EffectAst::Conditionals(ConditionalEffectAst::IfResult {
        predicate: IfResultPredicate::Did,
        effects: vec![copy],
    }));
    if let Some(declined) = declined {
        effects.push(EffectAst::Conditionals(ConditionalEffectAst::IfResult {
            predicate: IfResultPredicate::DidNot,
            effects: declined,
        }));
    }
    Ok(Some(effects))
}
