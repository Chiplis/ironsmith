use super::*;
use crate::grammar::combat_requirements::{CombatRequirement, parse_combat_requirement};

/// Subjects accepted here are complete battlefield selectors. Keep unsupported
/// relative clauses with the ordinary parser instead of salvaging a suffix.
fn combat_rule_subject(tokens: &[OwnedLexToken]) -> Option<AnthemSubjectAst> {
    if tokens.iter().any(|token| token.as_word().is_none()) {
        return None;
    }
    let words = crate::lexer::parser_token_word_refs(tokens);
    if matches!(words.as_slice(), [] | ["this"] | ["this", "creature"] | ["it"]) {
        return Some(AnthemSubjectAst::Source);
    }
    if matches!(words.as_slice(), ["enchanted" | "equipped", "creature"]) {
        let mut filter = ObjectFilter::creature()
            .match_tagged(words[0], crate::filter::TaggedOpbjectRelation::IsTaggedObject);
        // An absent intrinsic attachment tag can mean "any enchanted/equipped
        // creature" to a general filter. This rule names this source's host,
        // including when bestow restores the source's creature subtypes.
        filter.with_attached_object = Some(Box::new(ObjectFilter::source()));
        return Some(AnthemSubjectAst::Filter(filter));
    }
    let words = words.strip_prefix(&["all"]).unwrap_or(&words);
    let mut filter = ObjectFilter::creature();
    let ["creature" | "creatures", rest @ ..] = words else { return None; };
    let rest = match rest {
        ["you", "control", tail @ ..] => {
            filter.controller = Some(PlayerFilter::You);
            tail
        }
        ["your", "opponents", "control", tail @ ..] => {
            filter.controller = Some(PlayerFilter::Opponent);
            tail
        }
        rest => rest,
    };
    match rest {
        [] => {},
        ["with", "flying"] => filter = filter.with_static_ability(crate::static_abilities::StaticAbilityId::Flying),
        ["without", "flying"] => filter = filter.without_static_ability(crate::static_abilities::StaticAbilityId::Flying),
        _ => return None,
    }
    Some(AnthemSubjectAst::Filter(filter))
}

/// The complete requirement tail and complete subject determine which reader
/// owns this clause. The owner still validates every preceding stat/keyword
/// sibling; yielding here cannot turn an unknown sibling into partial coverage.
pub(super) fn owns_combat_requirement_line(tokens: &[OwnedLexToken]) -> bool {
    crate::grammar::combat_requirements::parse_combat_requirement_line(tokens)
        .is_some_and(|shape| combat_rule_subject(shape.clause.subject_tokens).is_some())
}

pub fn parse_self_combat_requirement_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let Some(clause) = parse_combat_requirement(tokens) else { return Ok(None); };
    if !matches!(combat_rule_subject(clause.subject_tokens), Some(AnthemSubjectAst::Source)) {
        return Ok(None);
    }
    Ok(Some(source_requirements(clause.requirement)))
}

fn source_requirements(requirement: CombatRequirement) -> Vec<StaticAbilityAst> {
    let abilities = match requirement {
        CombatRequirement::Attack => vec![StaticAbility::must_attack()],
        CombatRequirement::Block => vec![StaticAbility::must_block()],
        CombatRequirement::AttackOrBlock => vec![StaticAbility::must_attack(), StaticAbility::must_block()],
    };
    abilities.into_iter().map(StaticAbilityAst::Static).collect()
}

/// "This creature must be blocked by exactly one creature if able." (Nacatl
/// War-Pride): it must be blocked if able and can't be blocked by more than
/// one creature.
pub fn parse_source_must_be_blocked_by_exactly_one_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let words = crate::lexer::parser_token_word_refs(tokens);
    let tail: &[&str] = &["must", "be", "blocked", "by", "exactly", "one", "creature", "if", "able"];
    let Some(subject) = words.strip_suffix(tail) else {
        return Ok(None);
    };
    if !matches!(subject, ["this", "creature"] | ["this"] | ["it"]) {
        return Ok(None);
    }
    Ok(Some(vec![
        StaticAbilityAst::Static(StaticAbility::restriction(
            crate::effect::Restriction::must_be_blocked(ObjectFilter::source()),
            "this creature must be blocked by exactly one creature if able".to_string(),
        )),
        StaticAbilityAst::Static(StaticAbility::cant_be_blocked_by_more_than(1)),
    ]))
}

/// "Gorm must be blocked if able, and Gorm must be blocked by two or more
/// creatures if able." (Gorm the Great): two blocking requirements on the
/// source (CR 509.1c). The first is obeyed by any block; the second only by
/// at least that many blockers. A lone "must be blocked by two or more
/// creatures if able" is the second requirement alone.
pub fn parse_source_must_be_blocked_by_at_least_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let words = crate::lexer::parser_token_word_refs(tokens);
    let is_self = |subject: &[&str]| matches!(subject, ["this", "creature"] | ["this"] | ["it"]);
    let at_least = |clause: &[&str]| -> Option<usize> {
        let tail = clause.strip_prefix(&["must", "be", "blocked", "by"][..])?;
        let [count, "or", "more", "creatures", "if", "able"] = tail else {
            return None;
        };
        let minimum = crate::util::parse_number_word_u32(count)? as usize;
        (minimum >= 2).then_some(minimum)
    };
    let split = |clause: &[&str]| -> Option<usize> {
        (1..clause.len()).find(|&index| clause[index] == "must")
    };
    let mut abilities = Vec::new();
    let second = if let Some(and) = words.iter().position(|word| *word == "and") {
        let first = &words[..and];
        let Some(must) = split(first) else {
            return Ok(None);
        };
        if !is_self(&first[..must]) || first[must..] != ["must", "be", "blocked", "if", "able"] {
            return Ok(None);
        }
        abilities.push(StaticAbilityAst::Static(StaticAbility::restriction(
            crate::effect::Restriction::must_be_blocked(ObjectFilter::source()),
            "this creature must be blocked if able".to_string(),
        )));
        &words[and + 1..]
    } else {
        &words[..]
    };
    let Some(must) = split(second) else {
        return Ok(None);
    };
    if !is_self(&second[..must]) {
        return Ok(None);
    }
    let Some(minimum) = at_least(&second[must..]) else {
        return Ok(None);
    };
    abilities.push(StaticAbilityAst::Static(StaticAbility::restriction(
        crate::effect::Restriction::must_be_blocked_by_at_least(ObjectFilter::source(), minimum),
        format!("this creature must be blocked by {minimum} or more creatures if able"),
    )));
    Ok(Some(abilities))
}

pub fn parse_combat_requirement_static_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let Some(shape) = crate::grammar::combat_requirements::parse_combat_requirement_line(tokens) else {
        return Ok(None);
    };
    let Some(subject) = combat_rule_subject(shape.clause.subject_tokens) else { return Ok(None); };
    let mut result = if let Some(preceding) = shape.preceding_tokens {
        let Some(result) = parse_static_ability_ast_line_lexed(preceding)? else { return Ok(None); };
        result
    } else { Vec::new() };
    match subject {
        AnthemSubjectAst::Source => result.extend(source_requirements(shape.clause.requirement)),
        AnthemSubjectAst::Filter(filter) => {
            use crate::effect::Restriction;
            let rules = match shape.clause.requirement {
                CombatRequirement::Attack => vec![Restriction::must_attack(filter)],
                CombatRequirement::Block => vec![Restriction::must_block(filter)],
                CombatRequirement::AttackOrBlock => vec![Restriction::must_attack(filter.clone()), Restriction::must_block(filter)],
            };
            // "Enchanted creature attacks each combat if able": a singular
            // authored subject keeps verb agreement in the display.
            let singular_subject = matches!(
                crate::lexer::parser_token_word_refs(shape.clause.subject_tokens).first(),
                Some(&("enchanted" | "equipped" | "each" | "target" | "fortified"))
            );
            let suffix = match (shape.clause.requirement, singular_subject) {
                (CombatRequirement::Attack, false) => "attack each combat if able",
                (CombatRequirement::Attack, true) => "attacks each combat if able",
                (CombatRequirement::Block, false) => "block each combat if able",
                (CombatRequirement::Block, true) => "blocks each combat if able",
                (CombatRequirement::AttackOrBlock, false) => "attack or block each combat if able",
                (CombatRequirement::AttackOrBlock, true) => {
                    "attacks or blocks each combat if able"
                }
            };
            result.push(StaticAbilityAst::Static(StaticAbility::restrictions(rules,
                format!("{} {suffix}", crate::lexer::render_token_slice(shape.clause.subject_tokens)))));
        }
    }
    Ok(Some(result))
}

pub fn parse_source_owned_flying_block_limit_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    let Some(subject_tokens) = crate::grammar::combat_requirements::parse_flying_block_limit(tokens) else {
        return Ok(None);
    };
    let Some(subject) = combat_rule_subject(subject_tokens) else { return Ok(None); };
    Ok(Some(StaticAbilityAst::Static(match subject {
        AnthemSubjectAst::Source => StaticAbility::can_block_only_flying(),
        AnthemSubjectAst::Filter(blockers) => StaticAbility::restriction(
            crate::effect::Restriction::block_specific_attacker(blockers,
                ObjectFilter::creature().without_static_ability(crate::static_abilities::StaticAbilityId::Flying)),
            crate::lexer::render_token_slice(tokens),
        ),
    })))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex_line;

    #[test]
    fn self_obligations_keep_both_combat_roles_and_reject_unconsumed_tails() {
        for (text, expected) in [
            ("This creature blocks each combat if able.", vec![StaticAbility::must_block()]),
            ("This creature attacks or blocks each combat if able.", vec![StaticAbility::must_attack(), StaticAbility::must_block()]),
        ] {
            assert_eq!(parse_self_combat_requirement_line(&lex_line(text, 0).unwrap()).unwrap(),
                Some(expected.into_iter().map(StaticAbilityAst::Static).collect()));
        }
        for text in [
            "This creature blocks each combat if able and draws a card.",
            "This {R} creature blocks each combat if able.",
            "This creature: blocks each combat if able.",
            "Target creature blocks each combat if able this turn.",
            "Creatures have \"This creature blocks each combat if able.\"",
        ] {
            let tokens = lex_line(text, 0).unwrap();
            assert!(parse_self_combat_requirement_line(&tokens).unwrap().is_none());
            if !text.contains("have") {
                assert!(!matches!(parse_static_ability_ast_line_lexed(&tokens), Ok(Some(_))), "{text}");
            }
        }
    }

    #[test]
    fn unquoted_filtered_rules_stay_on_the_source_and_conjunctions_keep_siblings() {
        for (text, count, marker) in [
            ("All creatures block each combat if able.", 1, "MustBlock"),
            ("All creatures have double strike and attack each combat if able.", 2, "MustAttack"),
            ("Creatures you control have haste and attack each combat if able.", 2, "MustAttack"),
            ("Enchanted creature gets +4/+1 and blocks each combat if able.", 2, "MustBlock"),
        ] {
            let parsed = parse_static_ability_ast_line_lexed(&lex_line(text, 0).unwrap())
                .unwrap().expect(text);
            assert_eq!(parsed.len(), count, "{text}: {parsed:?}");
            let debug = format!("{parsed:?}");
            assert!(debug.contains("RuleRestriction") && debug.contains(marker), "{debug}");
        }
        for text in ["All creatures {R} block each combat if able.",
            "Enchanted creature can block only creatures with flying and draw a card.",
            "Creatures have \"This creature blocks each combat if able.\""] {
            let tokens = lex_line(text, 0).unwrap();
            assert!(parse_combat_requirement_static_line(&tokens).unwrap().is_none());
            assert!(parse_source_owned_flying_block_limit_line(&tokens).unwrap().is_none());
        }
    }

}
