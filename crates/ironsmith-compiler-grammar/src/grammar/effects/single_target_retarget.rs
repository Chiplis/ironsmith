//! "If target spell has only one target and that target is a creature,
//! change that spell's target to another creature." (Meddle, Quicksilver
//! Dragon). The spell is targeted without restriction; whether it has exactly
//! one target and what that target is are checked as the ability resolves
//! (CR 608.2b), and the new target must be a different object matching the
//! authored description (CR 115.7, "another").
use crate::cards::builders::{
    CardTextError, ConditionalEffectAst, EffectAst, PlayerAst, PredicateAst, RetargetModeAst,
    TargetAst,
};
use crate::grammar::primitives;
use crate::lexer::{OwnedLexToken, token_word_refs, trim_lexed_commas};
use crate::target::ObjectFilter;
use winnow::Parser as _;
use winnow::combinator::alt;

fn described_object(tokens: &[OwnedLexToken]) -> Result<Option<ObjectFilter>, CardTextError> {
    let tokens = trim_lexed_commas(crate::util::trim_edge_punctuation_tokens(tokens));
    if tokens.is_empty() {
        return Ok(None);
    }
    if crate::util::is_source_reference_words(&token_word_refs(tokens)) {
        return Ok(Some(ObjectFilter::source()));
    }
    let tokens = primitives::parse_prefix(tokens, alt((primitives::kw("a"), primitives::kw("an"))))
        .map_or(tokens, |(_, rest)| rest);
    let mut filter = crate::object_filters::parse_object_filter(tokens, false)?;
    filter.zone = None;
    Ok(Some(filter))
}

pub fn parse(tokens: &[OwnedLexToken]) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = crate::util::trim_edge_punctuation_tokens(tokens);
    let Some(((), after_if)) = primitives::parse_prefix(tokens, primitives::kw("if").void()) else {
        return Ok(None);
    };
    // "<target spell> has only one target and that target is <X>"
    let Some((has_index, (), after_has)) = primitives::find_prefix(after_if, || {
        primitives::phrase(&["has", "only", "one", "target", "and", "that", "target", "is"])
    }) else {
        return Ok(None);
    };
    let target_tokens = &after_if[..has_index];
    let Some(comma) = after_has.iter().position(OwnedLexToken::is_comma) else {
        return Ok(None);
    };
    let (current_tokens, consequence) = (&after_has[..comma], &after_has[comma + 1..]);
    // "change that spell's target to another <Y>"
    let Some(((), after_change)) = primitives::parse_prefix(
        trim_lexed_commas(consequence),
        (primitives::kw("change"), primitives::kw("that")).void(),
    ) else {
        return Ok(None);
    };
    let Some((target_word, (), new_target_tokens)) = primitives::find_prefix(after_change, || {
        primitives::phrase(&["target", "to", "another"])
    }) else {
        return Ok(None);
    };
    if !matches!(
        token_word_refs(&after_change[..target_word]).as_slice(),
        ["spell's"] | ["spells"] | ["spell", "s"] | ["spell"]
    ) {
        return Ok(None);
    }
    if !token_word_refs(target_tokens).first().is_some_and(|word| *word == "target") {
        return Ok(None);
    }
    let Some(current) = described_object(current_tokens)? else {
        return Ok(None);
    };
    let Some(replacement) = described_object(new_target_tokens)? else {
        return Ok(None);
    };
    let target = crate::util::parse_target_phrase(target_tokens)?;
    let matching = ObjectFilter::default()
        .targeting_only_object(current)
        .target_count_exact(1);
    let retarget = EffectAst::subject_verb_retarget_stack_object(
        PlayerAst::Implicit,
        TargetAst::Tagged(crate::tag::CompilerReferenceTag::It.bind(), None),
        RetargetModeAst::All,
        true,
    )
    .with_retarget_new_target_restriction(replacement);
    Ok(Some(vec![
        EffectAst::subject_verb_explicit_target_only(target),
        EffectAst::Conditionals(ConditionalEffectAst::Conditional {
            predicate: PredicateAst::TargetMatches(matching),
            if_true: vec![retarget],
            if_false: Vec::new(),
        }),
    ]))
}
