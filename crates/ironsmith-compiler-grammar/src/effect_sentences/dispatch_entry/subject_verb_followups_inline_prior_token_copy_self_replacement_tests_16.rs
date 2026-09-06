use super::*;
#[cfg(test)]
use ironsmith_compiler::ParseCardText;

fn copy_count(effects: &[EffectAst]) -> Option<Value> {
    for effect in effects {
        if let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::Tokens(TokenActionAst::CreateTokenCopy { count, .. })
                | SubjectVerbActionAst::Tokens(TokenActionAst::CreateTokenCopyFromSource { count, .. }),
            ..
        }) = effect
        {
            return Some(count.clone());
        }
        let mut nested_count = None;
        for_each_nested_effects(effect, true, |nested| {
            if nested_count.is_none() {
                nested_count = copy_count(nested);
            }
        });
        if nested_count.is_some() {
            return nested_count;
        }
    }
    None
}

fn copy_source(effects: &[EffectAst]) -> Option<TargetAst> {
    for effect in effects {
        if let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::Tokens(TokenActionAst::CreateTokenCopyFromSource { source, .. }),
            ..
        }) = effect
        {
            return Some(source.clone());
        }
        let mut nested_source = None;
        for_each_nested_effects(effect, true, |nested| {
            if nested_source.is_none() {
                nested_source = copy_source(nested);
            }
        });
        if nested_source.is_some() {
            return nested_source;
        }
    }
    None
}

#[test]
fn conditional_of_those_tokens_replaces_copy_token_count() {
    let lexed = crate::lexer::lex_line(
            "Create a tapped and attacking token that's a copy of another target attacking creature. If that creature is a Kraken, Leviathan, Octopus, or Serpent, create two of those tokens instead.",
            0,
        )
        .expect("copy-token replacement should lex");
    let parsed = parse_effect_sentences_lexed(&lexed).expect("copy-token replacement should parse");
    let [
        EffectAst::SelfReplacement {
            predicate,
            if_true,
            if_false,
            attach_to_previous_ability: false,
        },
    ] = parsed.as_slice()
    else {
        panic!("expected one typed copy-token self-replacement: {parsed:#?}");
    };

    assert_eq!(
        copy_count(if_false),
        Some(Value::Fixed(1)),
        "default copy branch: {if_false:#?}"
    );
    assert_eq!(
        copy_count(if_true),
        Some(Value::Fixed(2)),
        "replacement copy branch: {if_true:#?}"
    );
    let PredicateAst::TargetMatches(filter) = predicate else {
        panic!(
            "the demonstrative subtype condition must test the copy source target: {predicate:#?}"
        );
    };
    for subtype in ["Kraken", "Leviathan", "Octopus", "Serpent"] {
        assert!(format!("{filter:#?}").contains(subtype), "{filter:#?}");
    }
    let default_source = copy_source(if_false).expect("default copy source target");
    let replacement_source = copy_source(if_true).expect("replacement copy source target");
    assert_eq!(replacement_source, default_source);
    assert!(target_is_explicitly_chosen(&default_source));
}

#[test]
fn triggered_copy_replacement_lowers_subtype_check_to_declared_target() {
    let definition = crate::CardDefinitionBuilder::new(
            crate::CardId::new(),
            "Triggered Copy Replacement",
        )
        .card_types(vec![crate::CardType::Creature])
        .parse_text(
            "Whenever this creature attacks, create a tapped and attacking token that's a copy of another target attacking creature. If that creature is a Kraken, Leviathan, Octopus, or Serpent, create two of those tokens instead.",
        )
        .expect("triggered copy replacement should lower");
    let debug = format!("{:#?}", definition.abilities);
    assert!(debug.contains("condition: TargetMatches"), "{debug}");
    assert!(!debug.contains("condition: TaggedObjectMatches"), "{debug}");
}
