//! cf8 p01: "You may pay <mana> rather than pay the mana cost for <spells>
//! you cast" has exactly one static owner (the fixed alternative-mana-cost
//! grant, CR 118.9). Source-authored, deliberately unrun.
use ironsmith::ability::AbilityKind;

#[path = "p01_support/mod.rs"]
mod support;

#[test]
fn pure_mana_price_grants_compile_as_one_static_grant_each() {
    for (name, price) in [
        ("Fist of Suns", "{w}{u}{b}{r}{g}"),
        ("Jodah, Archmage Eternal", "{w}{u}{b}{r}{g}"),
        ("Leyline of Mutation", "{w}{u}{b}{r}{g}"),
        ("Rooftop Storm", "{0}"),
        ("Runeforge Champion", "{1}"),
    ] {
        for definition in support::definitions(name) {
            let text = support::rendered(&definition);
            assert!(
                text.contains(&format!("you may pay {price} rather than pay the mana cost for")),
                "{name}: {text}"
            );
            // The line is a static grant on the permanent, never a resolving
            // "You may pay {..}" payment.
            assert!(definition.spell_effect.is_none(), "{name}");
            assert!(
                definition
                    .abilities
                    .iter()
                    .any(|ability| matches!(ability.kind, AbilityKind::Static(_))),
                "{name}"
            );
        }
    }
    let text = support::rendered(&support::definitions("Rooftop Storm")[0]);
    assert!(text.contains("zombie creature spells you cast"), "{text}");
    let text = support::rendered(&support::definitions("Runeforge Champion")[0]);
    assert!(text.contains("rune spells you cast"), "{text}");
}
