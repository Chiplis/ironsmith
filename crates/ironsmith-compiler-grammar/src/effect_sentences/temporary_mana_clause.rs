//! "Until end of turn, red creatures get +1/+1 and whenever a player taps a
//! Mountain for mana, that player adds an additional {R}." /
//! "Until end of turn, red creatures get -1/-1 and if a player taps a
//! Mountain for mana, that Mountain produces colorless mana instead of any
//! other type." (Chaos Moon; also Bubbling Muck's lone trigger form).
//! One leading duration scopes a characteristic change and a temporary mana
//! rule: either a delayed "whenever ... for mana" trigger for the rest of the
//! turn (a triggered mana ability, CR 605.1b) or a registered mana rewrite
//! (a replacement effect, CR 614.1a).
use crate::cards::builders::{CardTextError, DelayedEffectAst, EffectAst, LineAst, OwnedLexToken};

fn until_end_of_turn_prefix(tokens: &[OwnedLexToken]) -> Option<usize> {
    let words = ["until", "end", "of", "turn"];
    if tokens.len() < 5 || !tokens[..4].iter().zip(words).all(|(token, word)| token.is_word(word)) {
        return None;
    }
    Some(if tokens[4].is_comma() { 5 } else { 4 })
}

fn mana_rule(
    prefix: &[OwnedLexToken],
    clause: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    if clause.first().is_some_and(|token| token.is_word("whenever")) {
        let Ok(LineAst::Triggered { trigger, effects, .. }) =
            crate::clause_support::parse_triggered_line_lexed(clause)
        else {
            return Ok(None);
        };
        let mut event = &trigger;
        while let crate::cards::builders::TriggerSpec::WithIntro { trigger, .. } = event {
            event = trigger;
        }
        if !matches!(event, crate::cards::builders::TriggerSpec::PlayerTapsForMana { .. }) {
            return Ok(None);
        }
        return Ok(Some(EffectAst::Delayed(DelayedEffectAst::DelayedTriggerThisTurn {
            trigger,
            effects,
            one_shot: false,
            until_end_of_combat: false,
            attach_to_previous_ability: false,
        })));
    }
    if clause.first().is_some_and(|token| token.is_word("if")) {
        let mut scoped = prefix.to_vec();
        scoped.extend_from_slice(clause);
        let Some(parsed) = crate::keyword_static::parse_mana_output_rewrite_definition(&scoped)?
        else {
            return Ok(None);
        };
        return Ok(Some(EffectAst::subject_verb_register_mana_rewrite(
            parsed.rule,
            parsed.target,
            parsed
                .mode
                .unwrap_or(crate::effects::ReplacementApplyMode::UntilEndOfTurn),
            parsed.display,
        )));
    }
    Ok(None)
}

pub(super) fn parse(tokens: &[OwnedLexToken]) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = crate::util::trim_edge_punctuation_tokens(tokens);
    let Some(body_start) = until_end_of_turn_prefix(tokens) else {
        return Ok(None);
    };
    let prefix = &tokens[..body_start];
    let body = &tokens[body_start..];
    // Bubbling Muck: the whole body is the temporary mana trigger.
    if body.first().is_some_and(|token| token.is_word("whenever")) {
        return Ok(mana_rule(prefix, body)?.map(|effect| vec![effect]));
    }
    let Some(split) = (1..body.len()).find(|&index| {
        body[index].is_word("and")
            && body
                .get(index + 1)
                .is_some_and(|token| token.is_any_word(&["whenever", "if"]))
            && body[index + 1..]
                .iter()
                .any(|token| token.is_word("mana"))
    }) else {
        return Ok(None);
    };
    let Some(rule) = mana_rule(prefix, &body[split + 1..])? else {
        return Ok(None);
    };
    let mut scoped_change = prefix.to_vec();
    scoped_change.extend_from_slice(&body[..split]);
    let mut effects = super::parse_effect_chain_lexed(&scoped_change)?;
    effects.push(rule);
    Ok(Some(effects))
}
