//! "The first spell you cast during each of your turns has assist. When you
//! cast that spell, if another player spent mana to cast it, you and that
//! player each draw a card." (Yume, Chronicler of Valor)
//!
//! A keyword grant to the first matching spell a player casts each turn,
//! paired with a cast trigger for that same spell. "During each of your
//! turns" restricts both the grant (it functions only during your turn) and
//! the trigger (the spell is cast during your turn). The trigger's condition
//! reads the completed cast: some mana spent to cast it was paid by another
//! player, which only assist allows (CR 702.132a). "That player" is the
//! assisting player, carried by the cast event.
use super::*;
use crate::cards::builders::{EffectAst, ForEachEffectAst, LineAst, PlayerAst, TriggerSpec};
use crate::model::ast::TriggerIntroSurfaceAst;

fn word_tokens(words: &[&str]) -> Vec<OwnedLexToken> {
    crate::lexer::synthetic_word_tokens(words)
}

/// Replace every effect performed by "that player" with the same effect
/// iterated over the assisting players. Returns how many were rebound.
fn rebind_that_player_to_assistant(effects: &mut Vec<EffectAst>) -> usize {
    let assistants = PlayerFilter::TaggedPlayer(ironsmith_core::tag::ASSISTING_PLAYERS_TAG.into());
    let mut rebound = 0;
    for effect in effects.iter_mut() {
        match effect {
            EffectAst::SubjectVerb(subject_verb) if subject_verb.subject.player == PlayerAst::That => {
                let inner = effect.clone();
                *effect = EffectAst::ForEach(ForEachEffectAst::ForEachPlayersFiltered {
                    sequential: false,
                    filter: assistants.clone(),
                    effects: vec![inner],
                });
                rebound += 1;
            }
            EffectAst::Sequence { effects: inner }
            | EffectAst::CommaThen { effects: inner }
            | EffectAst::Coordinated { effects: inner, .. } => {
                rebound += rebind_that_player_to_assistant(inner);
            }
            _ => {}
        }
    }
    rebound
}

pub fn parse_first_spell_grant_with_linked_cast_trigger_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<LineAst>, CardTextError> {
    let tokens = trim_edge_punctuation(tokens);
    let sentences = crate::lexer::split_lexed_sentences(&tokens);
    let [grant_sentence, trigger_sentence] = sentences.as_slice() else {
        return Ok(None);
    };
    // "When you cast that spell, <body>"
    let trigger_words = crate::lexer::parser_token_word_refs(trigger_sentence);
    if !trigger_words.starts_with(&["when", "you", "cast", "that", "spell"]) {
        return Ok(None);
    }
    let Some(comma) = trigger_sentence.iter().position(OwnedLexToken::is_comma) else {
        return Ok(None);
    };
    if crate::lexer::parser_token_word_refs(&trigger_sentence[..comma]).len() != 5 {
        return Ok(None);
    }
    let body = trim_lexed_commas(&trigger_sentence[comma + 1..]);
    // "The first <spells> you cast [during each of your turns] has|have <keyword>."
    let grant_view = crate::lexer::TokenWordView::new(grant_sentence);
    let grant_words = grant_view.word_refs();
    if !(grant_words.starts_with(&["the", "first"]) || grant_words.starts_with(&["first"])) {
        return Ok(None);
    }
    let during = grant_words
        .windows(5)
        .position(|window| window == ["during", "each", "of", "your", "turns"]);
    let (grant_tokens, your_turns) = match during {
        Some(start) => {
            let starts = grant_view.token_start_indices();
            let begin = starts[start];
            let end = starts.get(start + 5).copied().unwrap_or(grant_sentence.len());
            let mut rewritten = grant_sentence[..begin].to_vec();
            rewritten.extend(word_tokens(&["each", "turn"]));
            rewritten.extend_from_slice(&grant_sentence[end..]);
            (rewritten, true)
        }
        None if grant_words.windows(2).any(|window| window == ["each", "turn"]) => {
            (grant_sentence.to_vec(), false)
        }
        None => return Ok(None),
    };
    let Some(mut grants) = parse_static_ability_ast_line_lexed(&grant_tokens)? else {
        return Ok(None);
    };
    let [grant] = grants.as_mut_slice() else {
        return Ok(None);
    };
    let (filter, condition) = match grant {
        StaticAbilityAst::GrantStaticAbility { filter, condition, .. }
        | StaticAbilityAst::GrantKeywordAction { filter, condition, .. } => (filter.clone(), condition),
        _ => return Ok(None),
    };
    if !filter.first_spell_cast_each_turn {
        return Ok(None);
    }
    if your_turns {
        if condition.is_some() {
            return Ok(None);
        }
        *condition = Some(PredicateAst::YourTurn);
    }
    // The trigger's own body: an optional "if another player spent mana to
    // cast it," then the instruction.
    let body_words = crate::lexer::parser_token_word_refs(body);
    let (assisted_only, effect_tokens) = if body_words.starts_with(&[
        "if", "another", "player", "spent", "mana", "to", "cast", "it",
    ]) {
        let Some(comma) = body.iter().position(OwnedLexToken::is_comma) else {
            return Ok(None);
        };
        if crate::lexer::parser_token_word_refs(&body[..comma]).len() != 8 {
            return Ok(None);
        }
        (true, trim_lexed_commas(&body[comma + 1..]))
    } else {
        (false, body)
    };
    let mut effects = crate::clause_support::parse_effect_sentences_lexed(effect_tokens)?;
    if effects.is_empty() {
        return Ok(None);
    }
    if assisted_only {
        rebind_that_player_to_assistant(&mut effects);
    }
    let mut spell_filter = filter.clone();
    spell_filter.cast_by = Some(PlayerFilter::IteratedPlayer);
    // "When you cast that spell" names the spell the grant sentence
    // described; the demonstrative is presentation only.
    spell_filter.set_demonstrative_antecedent_surface(Some(
        ironsmith_core::DemonstrativeAntecedentSurface::Spell,
    ));
    let cast = TriggerSpec::SpellCast {
        filter: Some(spell_filter),
        mana_source_filter: None,
        caster: PlayerFilter::You,
        timing: None,
        during_turn: your_turns.then_some(PlayerFilter::You),
        min_spells_this_turn: None,
        exact_spells_this_turn: Some(1),
        from_not_hand: false,
    };
    let cast = TriggerSpec::WithIntro {
        intro: TriggerIntroSurfaceAst::When,
        trigger: Box::new(cast),
    };
    // Mana paid by any player other than the caster (CR 702.132a): the
    // spell's total spent mana exceeds the caster's own payment.
    let trigger = if assisted_only {
        TriggerSpec::ConditionQualified {
            trigger: Box::new(cast),
            condition: PredicateAst::ValueComparison {
                left: crate::effect::Value::ManaSpentToCastTriggeringObject,
                operator: crate::effect::ValueComparisonOperator::GreaterThan,
                right: crate::effect::Value::CasterManaSpentToCastTriggeringObject,
            },
            surface: "if another player spent mana to cast it".to_string(),
        }
    } else {
        cast
    };
    crate::word_claims::claim_tokens(&tokens);
    Ok(Some(LineAst::Multiple(vec![
        LineAst::StaticAbilities(grants),
        LineAst::Triggered {
            trigger,
            effects,
            max_triggers_per_turn: None,
        },
    ])))
}
