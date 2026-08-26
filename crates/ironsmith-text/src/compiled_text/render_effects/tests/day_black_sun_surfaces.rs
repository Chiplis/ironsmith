use super::*;

const ORACLE: &str = "Each creature with mana value X or less loses all abilities until end of turn. Destroy those creatures.";

#[test]
fn ability_loss_set_is_reused_by_the_followup_destruction() {
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Day of Black Sun")
            .card_types(vec![CardType::Sorcery])
            .parse_text(ORACLE)
            .expect("ability-loss set back-reference should compile");
    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition),
        [ORACLE]
    );

    let mut changed = definition
        .spell_effect
        .clone()
        .expect("sorcery should retain a resolution program");
    changed.segments[1].default_effects[0] = Effect::destroy_all(ObjectFilter::creature());
    assert_ne!(
        super::super::ast_render::describe_resolution_program(&changed),
        ORACLE.trim_end_matches('.')
    );
}
