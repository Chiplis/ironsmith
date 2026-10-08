//! "Dash costs you pay cost {2} less" (Warbringer) and "Blitz costs you pay
//! cost {1} less for each time you've cast your commander from the command
//! zone this game" (Henzie): cost modifiers that apply only when that
//! alternative cost is the one being paid (the engine's cost-modifier match
//! checks the casting method, CR 601.2f). Source-authored, deliberately unrun.
use ironsmith::ability::AbilityKind;

#[path = "p02_line_families/compile.rs"]
mod compile;

const WARBRINGER: &str = "Mana cost: {3}{R}\nType: Creature — Orc Berserker\nPower/Toughness: 3/3\nDash costs you pay cost {2} less (as long as this creature is on the battlefield).\nDash {2}{R} (You may cast this spell for its dash cost. If you do, it gains haste, and it's returned from the battlefield to its owner's hand at the beginning of the next end step.)";
const HENZIE: &str = "Mana cost: {B}{R}{G}\nType: Legendary Creature — Devil Rogue\nPower/Toughness: 3/3\nEach creature spell you cast with mana value 4 or greater has blitz. The blitz cost is equal to its mana cost. (You may choose to cast that spell for its blitz cost. If you do, it gains haste and \"When this creature dies, draw a card.\" Sacrifice it at the beginning of the next end step.)\nBlitz costs you pay cost {1} less for each time you've cast your commander from the command zone this game.";

fn statics(definition: &ironsmith::cards::CardDefinition) -> Vec<String> {
    definition
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(ability) => Some(format!("{ability:?}")),
            _ => None,
        })
        .collect()
}

#[test]
fn warbringer_reduces_only_dash_costs() {
    for definition in compile::compile_both("Warbringer", WARBRINGER) {
        let reduction = statics(&definition)
            .into_iter()
            .find(|debug| debug.contains("alternative_cast: Some(Dash)"))
            .expect("dash cost reduction");
        assert!(reduction.contains("Fixed(2)"), "{reduction}");
        assert!(reduction.contains("cast_by: Some(You)"), "{reduction}");
    }
}

#[test]
fn henzie_scales_blitz_reduction_by_commander_casts() {
    for definition in compile::compile_both("Henzie \"Toolbox\" Torre", HENZIE) {
        let reduction = statics(&definition)
            .into_iter()
            .find(|debug| debug.contains("alternative_cast: Some(Blitz)"))
            .expect("blitz cost reduction");
        assert!(reduction.contains("CommanderCastCount"), "{reduction}");
    }
}
