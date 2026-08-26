use super::*;

const ROOTS_ORACLE: &str = "Mill three cards, then return a land card or Elf card from your graveyard to your hand. If you can't, draw a card.";
const AURA_GRAFT_ORACLE: &str = "Gain control of target Aura that's attached to a permanent. Attach it to another permanent it can enchant.";

#[test]
fn failed_sequence_draw_uses_the_same_result_id() {
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Roots of Wisdom")
            .card_types(vec![CardType::Sorcery])
            .parse_text(ROOTS_ORACLE)
            .expect("failed return fallback should compile");
    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition),
        [ROOTS_ORACLE]
    );

    let mut changed = definition
        .spell_effect
        .clone()
        .expect("sorcery should retain a resolution program");
    changed.segments[1].default_effects[0] = Effect::if_then(
        crate::effect::EffectId(99),
        crate::effect::EffectPredicate::DidNotHappen,
        vec![Effect::draw(1)],
    );
    assert_ne!(
        super::super::ast_render::describe_resolution_program(&changed),
        ROOTS_ORACLE.trim_end_matches('.')
    );
}

#[test]
fn controlled_aura_and_new_host_tags_prove_the_legal_attachment_surface() {
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Aura Graft")
        .card_types(vec![CardType::Instant])
        .parse_text(AURA_GRAFT_ORACLE)
        .expect("controlled Aura move should compile");
    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition),
        [AURA_GRAFT_ORACLE]
    );

    let mut changed = definition
        .spell_effect
        .clone()
        .expect("instant should retain a resolution program");
    changed.segments[1].default_effects[1] = Effect::new(crate::effects::AttachObjectsEffect::new(
        ChooseSpec::Tagged(TagKey::from("wrong_aura")),
        ChooseSpec::Tagged(TagKey::from("attachment_target_1")),
    ));
    assert_ne!(
        super::super::ast_render::describe_resolution_program(&changed),
        AURA_GRAFT_ORACLE.trim_end_matches('.')
    );
}
