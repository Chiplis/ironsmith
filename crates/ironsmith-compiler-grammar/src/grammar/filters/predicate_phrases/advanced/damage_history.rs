use crate::target::ChooseSpec;
use super::*;
use ironsmith_core::{
    DamageHistoryQuery, DamageHistoryRecipients, DamageHistoryReduction, DamageHistorySources,
};

fn compared(
    query: DamageHistoryQuery,
    operator: ValueComparisonOperator,
    amount: i32,
) -> PredicateAst {
    PredicateAst::ValueComparison {
        left: Value::DamageHistory(Box::new(query)),
        operator,
        right: Value::Fixed(amount),
    }
}
fn number(word: &str) -> Option<i32> {
    crate::util::parse_number_word_i32(word)
}
fn reference(words: &[&str]) -> Option<ChooseSpec> {
    let (spec, used) =
        crate::grammar::shared_util::value_expr::damage_history_quantities::object_reference(
            words,
        )?;
    (used == words.len()).then_some(spec)
}
fn guard_reference(spec: &ChooseSpec, predicate: PredicateAst) -> PredicateAst {
    if matches!(spec.base(),ChooseSpec::Tagged(tag) if tag.as_str()==crate::tag::CompilerReferenceTag::It.as_str())
    {
        // A missing optional target is not an object that received zero damage.
        // Past-tense references can still use the real pre-move snapshot.
        PredicateAst::And(
            Box::new(PredicateAst::ItMatchedLastKnown(ObjectFilter::default())),
            Box::new(predicate),
        )
    } else {
        predicate
    }
}

pub(super) fn parse(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let words = crate::lexer::token_word_refs(tokens);
    let body = words.strip_suffix(&["this", "turn"])?;
    if let [
        "a",
        "source",
        "you",
        "controlled",
        "dealt",
        amount,
        "or",
        "more",
        "damage",
    ] = body
    {
        return Some(compared(
            DamageHistoryQuery {
                sources: DamageHistorySources::Matching(ObjectFilter::default().you_control()),
                recipients: DamageHistoryRecipients::Any,
                combat: None,
                reduction: DamageHistoryReduction::LargestSourceTotal,
            },
            ValueComparisonOperator::GreaterThanOrEqual,
            number(amount)?,
        ));
    }
    if let [
        amount,
        "or",
        "more",
        "sources",
        "you",
        "controlled",
        "dealt",
        "damage",
    ] = body
    {
        return Some(compared(
            DamageHistoryQuery {
                sources: DamageHistorySources::Matching(ObjectFilter::default().you_control()),
                recipients: DamageHistoryRecipients::Any,
                combat: None,
                reduction: DamageHistoryReduction::DistinctSources,
            },
            ValueComparisonOperator::GreaterThanOrEqual,
            number(amount)?,
        ));
    }
    // "4 or more damage was dealt to it this turn".
    if let [
        amount,
        "or",
        "more",
        "damage",
        "was",
        "dealt",
        "to",
        object @ ..,
    ] = body
    {
        let spec = reference(object)?;
        let predicate = compared(
            DamageHistoryQuery {
                sources: DamageHistorySources::Any,
                recipients: DamageHistoryRecipients::Reference(Box::new(spec.clone())),
                combat: None,
                reduction: DamageHistoryReduction::Total,
            },
            ValueComparisonOperator::GreaterThanOrEqual,
            number(amount)?,
        );
        return Some(guard_reference(&spec, predicate));
    }
    // Passive object-specific history, including a prior zone-moving effect.
    for split in 1..body.len() {
        let Some(spec) = reference(&body[..split]) else {
            continue;
        };
        let (negative, tail) = match &body[split..] {
            ["was", "dealt", tail @ ..] => (false, tail),
            ["wasnt" | "wasn't", "dealt", tail @ ..] => (true, tail),
            ["was", "not", "dealt", tail @ ..] => (true, tail),
            _ => continue,
        };
        let combat = match tail {
            ["damage"] => None,
            ["combat", "damage"] => Some(true),
            ["noncombat", "damage"] => Some(false),
            _ => continue,
        };
        let predicate = compared(
            DamageHistoryQuery {
                sources: DamageHistorySources::Any,
                recipients: DamageHistoryRecipients::Reference(Box::new(spec.clone())),
                combat,
                reduction: DamageHistoryReduction::Total,
            },
            if negative {
                ValueComparisonOperator::Equal
            } else {
                ValueComparisonOperator::GreaterThan
            },
            0,
        );
        return Some(guard_reference(&spec, predicate));
    }
    // Active source -> object history. Preserve the actor separately from
    // the damaged trigger/target object, and don't inspect current damage marks.
    for split in 1..body.len() {
        let source = if body[..split] == ["equipped", "creature"] {
            DamageHistorySources::SourceAttachedObject
        } else if let Some(spec) = reference(&body[..split]) {
            DamageHistorySources::Reference(Box::new(spec))
        } else {
            continue;
        };
        let (negative, tail) = match &body[split..] {
            ["dealt", tail @ ..] => (false, tail),
            ["didnt" | "didn't", "deal", tail @ ..] => (true, tail),
            ["did", "not", "deal", tail @ ..] => (true, tail),
            _ => continue,
        };
        let (combat, tail) = match tail {
            ["damage", "to", tail @ ..] => (None, tail),
            ["combat", "damage", "to", tail @ ..] => (Some(true), tail),
            ["noncombat", "damage", "to", tail @ ..] => (Some(false), tail),
            _ => continue,
        };
        let recipients = match tail {
            ["a", "creature"] => DamageHistoryRecipients::MatchingObjects(ObjectFilter::creature()),
            ["another", "creature"] => {
                DamageHistoryRecipients::MatchingObjects(ObjectFilter::creature().other())
            }
            _ => DamageHistoryRecipients::Reference(Box::new(reference(tail)?)),
        };
        let attached = matches!(source, DamageHistorySources::SourceAttachedObject);
        let predicate = compared(
            DamageHistoryQuery {
                sources: source,
                recipients,
                combat,
                reduction: DamageHistoryReduction::Total,
            },
            if negative {
                ValueComparisonOperator::Equal
            } else {
                ValueComparisonOperator::GreaterThan
            },
            0,
        );
        return Some(if attached {
            PredicateAst::And(
                Box::new(PredicateAst::AttachedToSourceMatches(
                    ObjectFilter::default(),
                )),
                Box::new(predicate),
            )
        } else {
            predicate
        });
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    fn predicate(text: &str) -> PredicateAst {
        parse(&crate::lexer::lex_line(text, 0).unwrap()).unwrap()
    }
    fn query(predicate: &PredicateAst) -> &DamageHistoryQuery {
        match predicate {
            PredicateAst::ValueComparison {
                left: Value::DamageHistory(query),
                ..
            } => query,
            PredicateAst::And(_, right) => query(right),
            _ => panic!("{predicate:?}"),
        }
    }
    #[test]
    fn predicates_distinguish_actual_totals_per_source_and_distinct_source_counts() {
        assert_eq!(
            query(&predicate(
                "a source you controlled dealt 5 or more damage this turn"
            ))
            .reduction,
            DamageHistoryReduction::LargestSourceTotal
        );
        assert_eq!(
            query(&predicate(
                "three or more sources you controlled dealt damage this turn"
            ))
            .reduction,
            DamageHistoryReduction::DistinctSources
        );
        assert_eq!(
            query(&predicate("4 or more damage was dealt to it this turn")).reduction,
            DamageHistoryReduction::Total
        );
        assert_eq!(
            query(&predicate("it was dealt noncombat damage this turn")).combat,
            Some(false)
        );
        assert!(
            matches!(predicate("that creature wasn't dealt damage this turn"),PredicateAst::And(guard,_) if matches!(guard.as_ref(),PredicateAst::ItMatchedLastKnown(_)))
        );
        assert!(matches!(
            query(&predicate(
                "equipped creature didn't deal combat damage to a creature this turn"
            ))
            .sources,
            DamageHistorySources::SourceAttachedObject
        ));
        assert!(
            matches!(query(&predicate("this creature dealt damage to another creature this turn")).recipients,DamageHistoryRecipients::MatchingObjects(ref filter) if filter.other)
        );
        for text in [
            "it would be dealt damage this turn",
            "this creature dealt damage to another creature last turn",
            "three or more sources you own dealt damage this turn",
            "it was dealt damage this turn twice",
        ] {
            assert!(
                parse(&crate::lexer::lex_line(text, 0).unwrap()).is_none(),
                "{text}"
            );
        }
    }
}
