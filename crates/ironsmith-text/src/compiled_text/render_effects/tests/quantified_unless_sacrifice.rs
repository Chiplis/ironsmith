use super::*;

#[test]
fn each_opponent_sacrifice_cost_survives_provenance_wrappers() {
    let text = "Whenever Acererak attacks, for each opponent, you create a 2/2 black Zombie creature token unless that player sacrifices a creature of their choice.";
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Acererak the Archlich")
            .supertypes(vec![Supertype::Legendary])
            .card_types(vec![CardType::Creature])
            .parse_text(text)
            .expect("quantified sacrifice-unless trigger should compile");
    let debug = format!("{definition:#?}");

    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join("\n"),
        text,
        "{debug}"
    );
    assert!(debug.contains("ForPlayersEffect"), "{debug}");
    assert!(debug.contains("UnlessPaysEffect"), "{debug}");
    assert!(debug.contains("SacrificeEffect"), "{debug}");
    assert!(debug.contains("player: IteratedPlayer"), "{debug}");
}
