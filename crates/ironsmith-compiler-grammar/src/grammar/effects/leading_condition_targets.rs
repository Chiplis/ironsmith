use super::*;

/// "If target creature has toughness 5 or greater, it gets +4/-4 until end of
/// turn." (Blood Lust): the condition introduces the spell's object target.
/// The target is chosen as the spell is cast (CR 601.2c) and is not a
/// targeting restriction; the condition tests it as the spell resolves
/// (CR 608.2b). The target is declared first, then the conditional reads it,
/// and the consequence's "it" names the same object.
///
/// Only the "has <quality>" form is read: the quality is an ordinary object
/// description ("creature with toughness 5 or greater"). Any part that does
/// not read declines, leaving the sentence to the other readers.
pub(super) fn leading_object_target_condition(
    tokens: &[OwnedLexToken],
    parse_effect_chain_lexed: fn(&[OwnedLexToken]) -> Result<Vec<EffectAst>, CardTextError>,
) -> Option<Vec<EffectAst>> {
    if !tokens.first().is_some_and(|token| token.is_word("if"))
        || !tokens.get(1).is_some_and(|token| token.is_word("target"))
    {
        return None;
    }
    let comma = tokens.iter().position(OwnedLexToken::is_comma)?;
    let verb = tokens[..comma]
        .iter()
        .position(|token| token.is_word("has"))?;
    // "target" plus at least one noun word.
    if verb < 3 {
        return None;
    }
    let noun = &tokens[2..verb];
    // Player targets ("target player has ...") are read by the player
    // quantity predicates and their own declaration.
    if noun
        .iter()
        .any(|token| token.is_any_word(&["player", "players", "opponent", "opponents", "spell"]))
    {
        return None;
    }
    let quality = &tokens[verb + 1..comma];
    if quality.is_empty() {
        return None;
    }
    let target = parse_target_phrase(&tokens[1..verb]).ok()?;
    if !matches!(target, TargetAst::Object(..)) {
        return None;
    }
    // "<noun> with <quality>": the tested description of the target.
    let mut description = noun.to_vec();
    description.push(OwnedLexToken::word("with".to_string(), tokens[verb].span()));
    description.extend(quality.iter().cloned());
    let filter = parse_object_filter_lexed(&description, false).ok()?;
    let mut bare = parse_object_filter_lexed(noun, false).ok()?;
    bare.zone = filter.zone;
    // The quality must have contributed a constraint of its own.
    if filter == bare {
        return None;
    }
    let if_true = parse_effect_chain_lexed(&tokens[comma + 1..]).ok()?;
    if if_true.is_empty() {
        return None;
    }
    Some(vec![
        EffectAst::subject_verb_explicit_target_only(target),
        EffectAst::Conditionals(ConditionalEffectAst::Conditional {
            predicate: PredicateAst::TargetMatches(filter),
            if_true,
            if_false: Vec::new(),
        }),
    ])
}
