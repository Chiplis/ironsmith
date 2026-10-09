//! Encore granted to graveyard cards with a per-card cost (CR 702.141):
//! the card's mana cost (Wire Surgeons) or {X} where X is its mana value
//! (Graywater's Fixer, Sliver Gravemother). Source-authored, deliberately unrun.
use ironsmith::ability::AbilityKind;

#[path = "p02_line_families/compile.rs"]
mod compile;

const CARDS: &[(&str, &str, &str)] = &[
    (
        "Wire Surgeons",
        "Mana cost: {4}{B}{B}\nType: Creature — Human Artificer\nPower/Toughness: 6/5\nFear (This creature can't be blocked except by artifact creatures and/or black creatures.)\nEach artifact creature card in your graveyard has encore. Its encore cost is equal to its mana cost. (Exile it and pay its mana cost: For each opponent, create a token copy that attacks that opponent this turn if able. They gain haste. Sacrifice them at the beginning of the next end step. Activate only as a sorcery.)",
        "source_mana_cost: true",
    ),
    (
        "Graywater's Fixer",
        "Mana cost: {2}{B}{R}\nType: Creature — Lizard Mercenary\nPower/Toughness: 4/4\nEach outlaw creature card in your graveyard has encore {X}, where X is its mana value. (Exile it and pay its encore cost: For each opponent, create a token copy that attacks that opponent this turn if able. They gain haste. Sacrifice them at the beginning of the next end step. Activate only as a sorcery.)",
        "ManaValueOf(Source)",
    ),
    (
        "Sliver Gravemother",
        "Mana cost: {W}{U}{B}{R}{G}\nType: Legendary Creature — Sliver\nPower/Toughness: 6/6\nThe \"legend rule\" doesn't apply to Slivers you control.\nEach Sliver creature card in your graveyard has encore {X}, where X is its mana value.\nEncore {5} ({5}, Exile this card from your graveyard: For each opponent, create a token copy that attacks that opponent this turn if able. They gain haste. Sacrifice them at the beginning of the next end step. Activate only as a sorcery.)",
        "ManaValueOf(Source)",
    ),
];

#[test]
fn granted_encore_grants_a_graveyard_activated_ability_with_a_derived_cost() {
    for &(name, text, cost_marker) in CARDS {
        for definition in compile::compile_both(name, text) {
            let grant = definition
                .abilities
                .iter()
                .filter_map(|ability| match &ability.kind {
                    AbilityKind::Static(ability) => Some(format!("{ability:?}")),
                    _ => None,
                })
                .find(|debug| debug.contains("CreateTokenCopyEffect") && debug.contains("Graveyard"))
                .unwrap_or_else(|| panic!("{name}: granted encore"));
            assert!(grant.contains(cost_marker), "{name}: {grant}");
            assert!(grant.contains("SorcerySpeed"), "{name}: {grant}");
        }
    }
}
