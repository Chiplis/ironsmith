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
    // "you were dealt damage this turn by a red instant or sorcery spell"
    // (Suffocation): an existential source class, read from the source's
    // characteristics when it dealt the damage.
    if words.len() == tokens.len()
        && let ["you", "were", "dealt", rest @ ..] = words.as_slice()
    {
        let (combat, rest) = match rest {
            ["damage", rest @ ..] => (None, rest),
            ["combat", "damage", rest @ ..] => (Some(true), rest),
            ["noncombat", "damage", rest @ ..] => (Some(false), rest),
            _ => (None, &[][..]),
        };
        if let ["this", "turn", "by", "a" | "an", source @ ..] = rest
            && !source.is_empty()
            && let Ok(sources) = crate::object_filters::parse_object_filter(
                &tokens[tokens.len() - source.len()..],
                false,
            )
        {
            return Some(compared(
                DamageHistoryQuery {
                    sources: DamageHistorySources::Matching(sources),
                    recipients: DamageHistoryRecipients::Players(crate::target::PlayerFilter::You),
                    combat,
                    reduction: DamageHistoryReduction::Total,
                },
                ValueComparisonOperator::GreaterThan,
                0,
            ));
        }
    }
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
    // An existential typed source ("a Giant, Wizard, or spell you controlled")
    // reads the source's receipt, not its current characteristics or controller.
    let body_tokens = crate::grammar::primitives::strip_lexed_suffix_phrase(tokens, &["this", "turn"])?;
    for (phrase, combat) in [
        (&["dealt", "damage", "to"][..], None),
        (&["dealt", "combat", "damage", "to"][..], Some(true)),
        (&["dealt", "noncombat", "damage", "to"][..], Some(false)),
    ] {
        let Some((split, (), recipient)) = crate::grammar::primitives::find_prefix(body_tokens, || crate::grammar::primitives::phrase(phrase)) else { continue; };
        if split == 0 || !matches!(body_tokens[0].as_word(), Some("a" | "an" | "another")) { continue; }
        let recipient = reference(&crate::lexer::token_word_refs(recipient))?;
        let mut source_tokens = body_tokens[..split].to_vec();
        if source_tokens.len() >= 2 && source_tokens[source_tokens.len()-2].is_word("you")
            && source_tokens.last().is_some_and(|token| token.is_word("controlled")) {
            source_tokens.last_mut()?.replace_word("control");
        }
        let sources = crate::object_filters::parse_object_filter(&source_tokens, false).ok()?;
        return Some(compared(DamageHistoryQuery {
            sources: DamageHistorySources::Matching(sources),
            recipients: DamageHistoryRecipients::Reference(Box::new(recipient)),
            combat, reduction: DamageHistoryReduction::Total,
        }, ValueComparisonOperator::GreaterThan, 0));
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
    fn typed_source_history_preserves_permanent_or_spell_domains() {
        let parsed = predicate("a Giant, Wizard, or spell you controlled dealt damage to it this turn");
        let DamageHistorySources::Matching(filter) = &query(&parsed).sources else { panic!("typed sources required"); };
        assert_eq!(filter.controller, Some(crate::filter::PlayerFilter::You), "{filter:#?}");
        assert_eq!(filter.zone, None, "mixed domains must stay branch-local: {filter:#?}");
        assert_eq!(filter.any_of.len(), 3, "{filter:#?}");
        for (branch, subtype) in filter.any_of[..2].iter().zip([crate::Subtype::Giant, crate::Subtype::Wizard]) {
            assert_eq!(branch.subtypes, vec![subtype]);
            assert!(branch.card_types.is_empty(), "a subtype permanent need not be a creature");
            assert_eq!(branch.zone, Some(crate::Zone::Battlefield), "{branch:#?}");
        }
        assert_eq!(filter.any_of[2].zone, Some(crate::Zone::Stack));
        assert_eq!(filter.any_of[2].stack_kind, Some(crate::filter::StackObjectKind::Spell));
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
