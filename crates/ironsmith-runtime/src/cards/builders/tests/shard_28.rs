use super::shard_16::{assert_oracle_card_parses_strict, parse_oracle_card_definition};
use super::*;

#[test]
pub(super) fn tyvar_kell_keeps_plural_grant_and_triggered_emblem_structure() {
    assert_oracle_card_parses_strict("Tyvar Kell");
    let definition = parse_oracle_card_definition("Tyvar Kell");

    let static_grant = definition
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability)
                if static_ability.id()
                    == crate::static_abilities::StaticAbilityId::GrantObjectAbilityForFilter =>
            {
                Some(static_ability)
            }
            _ => None,
        })
        .next()
        .expect("Tyvar should grant Elves a mana ability");
    assert_eq!(
        static_grant.display(),
        "Elves you control have \"{T}: Add {B}.\""
    );

    let mut emblem = None;
    for ability in &definition.abilities {
        let AbilityKind::Activated(activated) = &ability.kind else {
            continue;
        };
        let effects = activated.effects.flattened_default_effects();
        let Some(created) = effects
            .iter()
            .find_map(|effect| effect.downcast_ref::<crate::effects::CreateEmblemEffect>())
        else {
            continue;
        };
        assert_eq!(
            effects.len(),
            1,
            "the loyalty ability should create the emblem instead of executing its ability immediately: {effects:#?}"
        );
        assert!(emblem.replace(created.emblem.clone()).is_none());
    }

    let emblem = emblem.expect("Tyvar's ultimate should create an emblem");
    let [emblem_ability] = emblem.abilities.as_slice() else {
        panic!("Tyvar's emblem should contain one triggered ability: {emblem:#?}");
    };
    let AbilityKind::Triggered(triggered) = &emblem_ability.kind else {
        panic!("Tyvar's emblem ability should be triggered: {emblem_ability:#?}");
    };
    let cast = triggered
        .trigger
        .downcast_ref::<crate::triggers::SpellCastTrigger>()
        .expect("Tyvar's emblem should trigger when its controller casts a spell");
    assert_eq!(cast.caster, crate::target::PlayerFilter::You);
    assert!(
        cast.filter
            .as_ref()
            .is_some_and(|filter| filter.subtypes.contains(&crate::types::Subtype::Elf)),
        "Tyvar's emblem trigger should be restricted to Elf spells: {cast:#?}"
    );
    let trigger_effects = triggered.effects.flattened_default_effects();
    let trigger_debug = format!("{trigger_effects:#?}");
    assert!(
        trigger_debug.contains("TagTriggeringObjectEffect")
            && trigger_debug.contains("triggering")
            && trigger_debug.contains("AddAbility")
            && trigger_debug.contains("Haste")
            && trigger_debug.contains("DrawCardsEffect"),
        "the emblem trigger should tag and grant the cast Elf haste, then draw two cards: {trigger_debug}"
    );

    let compiled = canonical_compiled_lines(&definition).join("\n");
    for expected in [
        "Elves you control have \"{T}: Add {B}.\"",
        "+1: Put a +1/+1 counter on up to one target Elf. Untap it. It gains deathtouch until end of turn.",
        "0: Create a 1/1 green Elf Warrior creature token.",
        "−6: You get an emblem with \"Whenever you cast an Elf spell, it gains haste until end of turn and you draw two cards.\"",
    ] {
        assert!(
            compiled.contains(expected),
            "missing `{expected}` in:\n{compiled}"
        );
    }
}
