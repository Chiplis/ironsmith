//! cf8 p01: mandatory Waterbend spell additional costs keep their printed
//! line. Payment behaviour is covered by ironsmith-tools
//! waterbend_payment_bodies.rs. Source-authored, deliberately unrun.
#[path = "p01_support/mod.rs"]
mod support;

#[test]
fn mandatory_waterbend_additional_costs_are_scoped_and_rendered() {
    for (name, amount) in [
        ("Benevolent River Spirit", "{5}"),
        ("Water Whip", "{5}"),
        ("Foggy Swamp Visions", "{X}"),
        ("Waterbender's Restoration", "{X}"),
    ] {
        for definition in support::definitions(name) {
            let mana = definition
                .additional_cost
                .mana_cost()
                .unwrap_or_else(|| panic!("{name}: missing additional mana component"));
            assert!(mana.has_waterbend_obligation(), "{name}: {mana:?}");
            assert!(definition.additional_cost.non_mana_costs().next().is_none(), "{name}");
            let text = support::rendered(&definition);
            assert!(
                text.contains(&format!(
                    "as an additional cost to cast this spell, waterbend {}",
                    amount.to_ascii_lowercase()
                )),
                "{name}: {text}"
            );
        }
    }
}
