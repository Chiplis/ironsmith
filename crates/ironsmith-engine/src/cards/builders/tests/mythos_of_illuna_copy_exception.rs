#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

fn token_copy(effect: &Effect) -> Option<&crate::effects::CreateTokenCopyEffect> {
    if let Some(copy) = effect.downcast_ref::<crate::effects::CreateTokenCopyEffect>() {
        return Some(copy);
    }
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        return token_copy(&tagged.effect);
    }
    None
}

#[test]
fn mythos_of_illuna_keeps_the_quoted_trigger_on_the_replacement_copy() {
    let definition = parse_oracle_card_definition("Mythos of Illuna");
    assert_eq!(
        canonical_compiled_lines(&definition),
        [
            "Create a token that's a copy of target permanent. If {R}{G} was spent to cast this spell, instead create a token that's a copy of that permanent, except the token has \"When this token enters, if it's a creature, it fights up to one target creature you don't control.\""
        ],
        "{definition:#?}"
    );

    let program = definition
        .spell_effect
        .as_ref()
        .expect("Mythos spell effect");
    let [segment] = program.segments.as_slice() else {
        panic!("expected one self-replacement segment: {program:#?}");
    };
    let [branch] = segment.self_replacements.as_slice() else {
        panic!("expected one mana-spent replacement: {segment:#?}");
    };
    let replacement_copy = branch
        .replacement_effects
        .iter()
        .find_map(token_copy)
        .expect("replacement branch must create a token copy");
    assert!(
        replacement_copy
            .granted_static_abilities
            .iter()
            .any(|ability| {
                let display = ability.display().to_ascii_lowercase();
                display.contains("fight") && display.contains("when")
            }),
        "the replacement copy must retain its quoted ETB fight trigger: {replacement_copy:#?}"
    );
}
