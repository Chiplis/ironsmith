//! cf8 p01: exact draw replacements are claimed by a single static owner
//! instead of falling to a resolving "If custom condition ..." effect
//! (CR 614.1a). Source-authored, deliberately unrun.
use ironsmith::ability::AbilityKind;
use ironsmith::static_abilities::StaticAbilityId;

#[path = "p01_support/mod.rs"]
mod support;

fn static_ids(definition: &ironsmith::cards::CardDefinition) -> Vec<StaticAbilityId> {
    definition
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect()
}

#[test]
fn thought_reflection_and_asmodeus_keep_their_draw_replacements() {
    for (name, id, phrase) in [
        (
            "Thought Reflection",
            StaticAbilityId::DrawReplacementDouble,
            "if you would draw a card, draw two cards instead",
        ),
        (
            "Asmodeus the Archfiend",
            StaticAbilityId::DrawReplacementExileTopFaceDown,
            "exile the top card of your library face down instead",
        ),
    ] {
        for definition in support::definitions(name) {
            assert!(definition.spell_effect.is_none(), "{name}");
            let ids = static_ids(&definition);
            assert_eq!(ids.iter().filter(|candidate| **candidate == id).count(), 1, "{name}: {ids:?}");
            let text = support::rendered(&definition);
            assert!(text.contains(phrase), "{name}: {text}");
            support::assert_no_internal_markers(name, &text);
        }
    }
}
