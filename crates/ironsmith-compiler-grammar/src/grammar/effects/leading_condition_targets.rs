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

/// "Choose target creature you control and target creature you don't
/// control. If the creature you control entered this turn, put a +1/+1
/// counter on it." (Malamet Battle Glyph): a definite description in a
/// leading condition names one earlier target, not the newest object. The
/// description is first re-declared as a reference to that target (no new
/// choice), so both the condition, tested as the spell resolves (CR 608.2b),
/// and the consequence's "it" name the described target.
pub(super) fn leading_definite_reference_condition(
    tokens: &[OwnedLexToken],
    parse_effect_chain_lexed: fn(&[OwnedLexToken]) -> Result<Vec<EffectAst>, CardTextError>,
) -> Option<Vec<EffectAst>> {
    const DESCRIPTOR_HEADS: &[&str] = &[
        "entered", "is", "isnt", "isn't", "was", "wasnt", "wasn't", "has", "had", "attacked",
        "blocked",
    ];
    if !tokens.first().is_some_and(|token| token.is_word("if"))
        || !tokens.get(1).is_some_and(|token| token.is_word("the"))
    {
        return None;
    }
    let comma = tokens.iter().position(OwnedLexToken::is_comma)?;
    let consequence = &tokens[comma + 1..];
    // A replacement ("..., <action> instead") belongs to its own readers.
    if consequence.iter().any(|token| token.is_word("instead")) {
        return None;
    }
    // "the" plus at least one noun word before the descriptor.
    let verb = (3..comma).find(|&index| tokens[index].is_any_word(DESCRIPTOR_HEADS))?;
    let noun = &tokens[2..verb];
    if noun.iter().any(|token| {
        token.is_any_word(&["player", "players", "opponent", "opponents", "turn", "number"])
    }) {
        return None;
    }
    let reference = parse_target_phrase(&tokens[1..verb]).ok()?;
    // Only a fresh description that tells earlier targets apart by who
    // controls or owns them ("the creature you control"); anaphoric
    // references ("the sacrificed creature", "the exiled card") keep their
    // own antecedent readers.
    let TargetAst::Object(filter, None, Some(_)) = &reference else {
        return None;
    };
    if !filter.tagged_constraints.is_empty()
        || filter.source
        || (filter.controller.is_none() && filter.owner.is_none())
    {
        return None;
    }
    let mut it_tokens = vec![OwnedLexToken::word("it".to_string(), tokens[1].span())];
    it_tokens.extend(tokens[verb..comma].iter().cloned());
    let predicate =
        crate::grammar::structure::parse_predicate_with_grammar_entrypoint_lexed(&it_tokens)
            .ok()?;
    let if_true = parse_effect_chain_lexed(consequence).ok()?;
    if if_true.is_empty() {
        return None;
    }
    Some(vec![
        EffectAst::subject_verb_target_only(reference),
        EffectAst::Conditionals(ConditionalEffectAst::Conditional {
            predicate,
            if_true,
            if_false: Vec::new(),
        }),
    ])
}
