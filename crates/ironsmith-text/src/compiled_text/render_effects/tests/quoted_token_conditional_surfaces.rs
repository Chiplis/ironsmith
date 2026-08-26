use super::*;

#[test]
fn quoted_token_rule_keeps_the_following_conditional_transform_sentence() {
    let text = "At the beginning of your end step, create a tapped 0/1 black Wizard creature token with \"Whenever you cast a noncreature spell, this token deals 1 damage to each opponent.\" Then if you control four or more Wizards, transform Kuja.";
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Kuja, Genome Sorcerer")
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Human, Subtype::Mutant, Subtype::Wizard])
            .parse_text(text)
            .expect("quoted token rule and conditional transform should compile");

    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join("\n"),
        text
    );
}

#[test]
fn quoted_token_rule_keeps_the_following_counter_kind_distribution_sentence() {
    let text = "Create two 1/1 blue Fish creature tokens with \"This token can't be blocked.\" Then for each kind of counter among creatures you control, put a counter of that kind on either of those tokens.";
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Exotic Pets")
        .card_types(vec![CardType::Sorcery])
        .parse_text(text)
        .expect("quoted token rule and counter-kind distribution should compile");

    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join("\n"),
        text
    );
}

fn token_copy(effect: &Effect) -> Option<&crate::effects::CreateTokenCopyEffect> {
    if let Some(copy) = effect.downcast_ref::<crate::effects::CreateTokenCopyEffect>() {
        return Some(copy);
    }
    effect
        .downcast_ref::<crate::effects::TaggedEffect>()
        .and_then(|tagged| token_copy(&tagged.effect))
}

#[test]
fn quoted_copy_exception_belongs_only_to_the_self_replacement_copy() {
    let text = "Create a token that's a copy of target permanent. If {R}{G} was spent to cast this spell, instead create a token that's a copy of that permanent, except the token has \"When this token enters, if it's a creature, it fights up to one target creature you don't control.\"";
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Mythos of Illuna")
            .card_types(vec![CardType::Sorcery])
            .parse_text(text)
            .expect("quoted copy self replacement should compile");

    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join("\n"),
        text
    );
    let [segment] = definition
        .spell_effect
        .as_ref()
        .expect("spell effect")
        .segments
        .as_slice()
    else {
        panic!("expected one replacement segment: {definition:#?}");
    };
    let [default] = segment.default_effects.as_slice() else {
        panic!("expected one default copy: {segment:#?}");
    };
    assert!(
        token_copy(default)
            .expect("default token copy")
            .granted_static_abilities
            .is_empty(),
        "the quoted exception must not leak into the default copy: {segment:#?}"
    );
    let [branch] = segment.self_replacements.as_slice() else {
        panic!("expected one replacement branch: {segment:#?}");
    };
    let [replacement] = branch.replacement_effects.as_slice() else {
        panic!("the inline grant must not survive as a duplicate set effect: {branch:#?}");
    };
    assert_eq!(
        token_copy(replacement)
            .expect("replacement token copy")
            .granted_static_abilities
            .len(),
        1,
        "{replacement:#?}"
    );
}
