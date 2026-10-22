//! "You may play a land from the top of your library or cast a spell with
//! mana value less than or equal to the damage dealt from the top of your
//! library without paying its mana cost." (Ziatora's Envoy): after looking at
//! the top card, one optional play of that card, admitted when it is a land
//! (a land play, CR 305.1) or a nonland card matching the spell qualifier
//! (a cast, CR 601.2; the free price belongs to the spell). It is the same
//! instruction as "If it's a land card or a nonland card <qualifier>, you may
//! play it without paying its mana cost", so it is read through that complete
//! sentence grammar (keeping the authored spans for result tags).
use crate::cards::builders::{CardTextError, EffectAst};
use crate::lexer::OwnedLexToken;

const TOP: &[&str] = &["from", "the", "top", "of", "your", "library"];

pub(super) fn parse(tokens: &[OwnedLexToken]) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = crate::util::trim_edge_punctuation_tokens(tokens);
    let words = crate::lexer::token_word_refs(tokens);
    let Some(rest) = words.strip_prefix(&["you", "may", "play", "a", "land"][..]) else {
        return Ok(None);
    };
    let Some(rest) = rest.strip_prefix(TOP) else { return Ok(None) };
    let Some(rest) = rest.strip_prefix(&["or", "cast", "a", "spell"][..]) else {
        return Ok(None);
    };
    let (qualifier, free) = if let Some(head) = rest.strip_suffix(&["without", "paying", "its", "mana", "cost"][..]) {
        (head, true)
    } else {
        (rest, false)
    };
    let Some(qualifier) = qualifier.strip_suffix(TOP) else { return Ok(None) };
    // The qualifier is an ordinary card-filter tail ("with mana value ...").
    if qualifier.first().is_some_and(|word| *word != "with") || qualifier.contains(&"from") {
        return Ok(None);
    }
    let span = crate::util::span_from_tokens(tokens)
        .unwrap_or_else(crate::cards::builders::TextSpan::synthetic);
    let words_at_source = |words: &[&str]| words.iter()
        .map(|word| OwnedLexToken::word(*word, span)).collect::<Vec<_>>();
    let mut rewritten = words_at_source(&["if", "it's", "a", "land", "card", "or", "a", "nonland", "card"]);
    // Retain the original qualifier tokens, including quantities and spans.
    let view = crate::lexer::TokenWordView::new(tokens);
    if let Some(range) = view.token_span_for_words(15, 15 + qualifier.len()) {
        rewritten.extend_from_slice(&tokens[range]);
    }
    rewritten.push(OwnedLexToken::comma(span));
    rewritten.extend(words_at_source(&["you", "may", "play", "it"]));
    if free {
        rewritten.extend(words_at_source(&["without", "paying", "its", "mana", "cost"]));
    }
    let gated = crate::clause_support::parse_effect_sentences_lexed(&rewritten)?;
    // CR 601.3e: the qualifier constrains the spell being cast (the face put
    // on the stack: an Adventure, a split half, a modal back face), not the
    // card. Read the qualifier through the gated sentence, then offer the one
    // optional play directly with that spell requirement: a land is played
    // (CR 305.1), anything else is cast only as a matching spell.
    let Some(spell_requirement) = gated.iter().find_map(nonland_spell_requirement) else {
        return Ok((!gated.is_empty()).then_some(gated));
    };
    let mut direct = words_at_source(&["you", "may", "play", "it"]);
    if free {
        direct.extend(words_at_source(&["without", "paying", "its", "mana", "cost"]));
    }
    let mut effects = crate::clause_support::parse_effect_sentences_lexed(&direct)?;
    let mut installed = false;
    for effect in &mut effects {
        install_required_spell(effect, &spell_requirement, &mut installed);
    }
    if !installed {
        return Ok((!gated.is_empty()).then_some(gated));
    }
    Ok(Some(effects))
}

/// The nonland arm of "if it's a land card or a nonland card <qualifier>".
fn nonland_spell_requirement(effect: &EffectAst) -> Option<crate::filter::ObjectFilter> {
    use crate::cards::builders::{ConditionalEffectAst, PredicateAst};
    let EffectAst::Conditionals(ConditionalEffectAst::Conditional { predicate, .. }) = effect else {
        return None;
    };
    let PredicateAst::Or(_, nonland) = predicate else {
        return None;
    };
    let (PredicateAst::ItMatches(filter) | PredicateAst::TaggedMatches(_, filter)) =
        nonland.as_ref()
    else {
        return None;
    };
    if filter.excluded_card_types != [crate::types::CardType::Land] || filter.mana_value.is_none() {
        return None;
    }
    let mut spell = crate::filter::ObjectFilter::default();
    spell.mana_value = filter.mana_value.clone();
    Some(spell)
}

fn install_required_spell(
    effect: &mut EffectAst,
    requirement: &crate::filter::ObjectFilter,
    installed: &mut bool,
) {
    use crate::cards::builders::{StackActionAst, SubjectVerbActionAst, SubjectVerbEffectAst};
    if let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action: SubjectVerbActionAst::Stack(StackActionAst::CastTagged { allow_land: true, required_spell, .. }),
        ..
    }) = effect
        && required_spell.is_none()
    {
        *required_spell = Some(requirement.clone());
        *installed = true;
        return;
    }
    crate::effect_ast_traversal::for_each_nested_effects_mut(effect, true, |nested| {
        for effect in nested {
            install_required_spell(effect, requirement, installed);
        }
    });
}
