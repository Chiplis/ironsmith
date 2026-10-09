//! Silent-miscompile guard: one placement verb governing two independently
//! named objects keeps both (CR 115.1d). Source-authored, deliberately UNRUN.
#[path = "cf8_p10_support/mod.rs"]
mod support;

use ironsmith::effects::MoveToZoneEffect;
use ironsmith::target::ChooseSpec;
use ironsmith::Zone;

fn moves(text: &str) -> Vec<MoveToZoneEffect> {
    support::definitions("Pair probe", text)
        .iter()
        .flat_map(support::find_all::<MoveToZoneEffect>)
        .collect()
}

#[test]
fn source_and_target_both_go_on_top_of_their_owners_libraries() {
    let text = "Mana cost: {2}{U}\nType: Creature — Elemental\nPower/Toughness: 2/2\n{2}{U}, {T}: Put this creature and target creature on top of their owners' libraries.";
    let moves = moves(text);
    assert_eq!(moves.len(), 4, "two moves per route");
    for pair in moves.chunks(2) {
        assert!(pair.iter().all(|m| m.zone == Zone::Library && m.to_top));
        assert!(pair.iter().any(|m| matches!(m.target, ChooseSpec::Source)), "{pair:?}");
        assert!(pair.iter().any(|m| m.target.is_target()), "the target is not dropped: {pair:?}");
    }
}

#[test]
fn two_targets_are_two_slots_not_a_type_union() {
    let text = "Mana cost: {3}{U}\nType: Sorcery\nPut target creature and target land on top of their owners' libraries.";
    let moves = moves(text);
    assert_eq!(moves.len(), 4);
    for pair in moves.chunks(2) {
        assert!(pair.iter().all(|m| m.target.is_target()));
        let rendered: Vec<String> = pair.iter().map(|m| format!("{:?}", m.target)).collect();
        assert!(rendered[0].contains("Creature") && rendered[1].contains("Land"), "{rendered:?}");
    }
}
