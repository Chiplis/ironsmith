use super::*;

#[test]
fn source_attack_history_keeps_battle_target_kind_and_turn_window() {
    let oracle =
        "Reach\nThis creature has indestructible as long as it attacked a battle this turn.";
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "War Historian")
        .card_types(vec![CardType::Creature])
        .parse_text(oracle)
        .expect("battle attack-history condition should compile");
    let debug = format!("{definition:#?}");

    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join("\n"),
        oracle,
        "{debug}"
    );
    assert!(debug.contains("SourceAttackedBattleThisTurn"), "{debug}");
    assert!(!debug.contains("TargetMatches"), "{debug}");
}
