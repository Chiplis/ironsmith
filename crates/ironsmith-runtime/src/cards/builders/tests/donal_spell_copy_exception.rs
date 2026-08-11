#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

const ORACLE: &str = "Whenever you cast a nonlegendary creature spell with flying, you may copy it, except the copy is a 1/1 Spirit in addition to its other types. Do this only once each turn.";

fn copy_spell(effect: &Effect) -> Option<&crate::effects::CopySpellEffect> {
    if let Some(copy) = effect.downcast_ref::<crate::effects::CopySpellEffect>() {
        return Some(copy);
    }
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        return copy_spell(&tagged.effect);
    }
    if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
        return copy_spell(&with_id.effect);
    }
    None
}

#[test]
fn donal_keeps_the_fixed_pt_spirit_exception_on_the_spell_copy() {
    let definition = parse_oracle_card_definition("Donal, Herald of Wings");
    let triggered = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Donal should retain its cast trigger");

    let cast = triggered
        .trigger
        .downcast_ref::<crate::triggers::SpellCastTrigger>()
        .expect("Donal should use the typed spell-cast trigger");
    let filter = cast
        .filter
        .as_ref()
        .expect("the trigger should be filtered");
    assert_eq!(filter.card_types, [CardType::Creature], "{filter:#?}");
    assert!(filter.excluded_supertypes.contains(&Supertype::Legendary));
    assert!(
        filter
            .static_abilities
            .contains(&crate::static_abilities::StaticAbilityId::Flying),
        "{filter:#?}"
    );
    assert_eq!(
        triggered.intervening_if,
        Some(crate::ConditionExpr::DoThisMaxTimesEachTurn(1))
    );

    let may = triggered
        .effects
        .flattened_default_effects()
        .iter()
        .find_map(|effect| effect.downcast_ref::<crate::effects::MayEffect>())
        .expect("the copy should be optional");
    let copy = may
        .effects
        .iter()
        .find_map(copy_spell)
        .expect("the copied spell should retain its characteristic exception");
    assert_eq!(copy.added_subtypes, [Subtype::Spirit]);
    assert_eq!(copy.set_base_power_toughness, Some((1, 1)));
    assert!(copy.added_card_types.is_empty());
    assert!(copy.set_colors.is_none());
    assert_eq!(canonical_compiled_lines(&definition), [ORACLE]);
}
