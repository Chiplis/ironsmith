#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

fn find_copy_spell(effect: &crate::effect::Effect) -> Option<&crate::effects::CopySpellEffect> {
    if let Some(copy) = effect.downcast_ref::<crate::effects::CopySpellEffect>() {
        return Some(copy);
    }
    if let Some(tagged) = effect.downcast_ref::<TaggedEffect>() {
        return find_copy_spell(&tagged.effect);
    }
    if let Some(with_id) = effect.downcast_ref::<WithIdEffect>() {
        return find_copy_spell(&with_id.effect);
    }
    None
}

#[test]
fn league_guildmage_keeps_the_complete_typed_copy_target() {
    let definition = parse_oracle_card_definition("League Guildmage");
    let copy = definition
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .flat_map(|activated| activated.effects.flattened_default_effects())
        .find_map(find_copy_spell)
        .expect("League Guildmage should retain its typed copy-spell effect");

    let ChooseSpec::Target(target) = &copy.target else {
        panic!("League Guildmage copy choice should be a target: {copy:#?}");
    };
    let ChooseSpec::Object(filter) = target.as_ref() else {
        panic!("League Guildmage should target a filtered stack object: {copy:#?}");
    };

    assert!(filter.any_of.is_empty(), "{filter:#?}");
    assert_eq!(
        filter.card_types,
        [CardType::Instant, CardType::Sorcery],
        "{filter:#?}"
    );
    assert_eq!(filter.zone, Some(Zone::Stack), "{filter:#?}");
    assert_eq!(filter.controller, Some(PlayerFilter::You), "{filter:#?}");
    assert_eq!(
        filter.stack_kind,
        Some(crate::filter::StackObjectKind::Spell),
        "{filter:#?}"
    );
    assert!(matches!(
        filter.mana_value.as_ref(),
        Some(crate::filter::Comparison::EqualExpr(value))
            if value.unhinted() == &crate::effect::Value::X
    ));

    assert_eq!(
        canonical_compiled_lines(&definition).join("\n"),
        "{3}{U}, {T}: Draw a card.\n{X}{R}, {T}: Copy target instant or sorcery spell you control with mana value X. You may choose new targets for the copy."
    );
}
