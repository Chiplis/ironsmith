//! "All Sliver creatures have provoke." (Hunter Sliver): provoke (CR 702.39)
//! granted like the other attack-trigger keywords. Source-authored, unrun.
use ironsmith::ability::AbilityKind;

#[path = "p02_line_families/compile.rs"]
mod compile;

const HUNTER_SLIVER: &str = "Mana cost: {1}{R}\nType: Creature — Sliver\nPower/Toughness: 1/1\nAll Sliver creatures have provoke. (Whenever a Sliver attacks, its controller may have target creature defending player controls untap and block it if able.)";

#[test]
fn hunter_sliver_grants_provoke_to_all_slivers() {
    for definition in compile::compile_both("Hunter Sliver", HUNTER_SLIVER) {
        let grants: Vec<String> = definition
            .abilities
            .iter()
            .filter_map(|ability| match &ability.kind {
                AbilityKind::Static(ability) => Some(format!("{ability:?}")),
                _ => None,
            })
            .collect();
        assert_eq!(grants.len(), 1, "{grants:?}");
        assert!(grants[0].contains("Provoke"), "{}", grants[0]);
        assert!(grants[0].contains("Sliver"), "{}", grants[0]);
        assert!(!grants[0].contains("controller: Some(You)"), "all Slivers, not only yours: {}", grants[0]);
    }
}
