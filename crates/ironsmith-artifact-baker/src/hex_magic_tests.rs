use super::*;
use ironsmith_runtime_catalog as engine;

fn compile(text: &str) -> engine::CardDefinition {
    let text = format!("Mana cost: {{2}}{{R}}\nType: Sorcery — Arcane\n{}", text);
    let artifact = compile_artifact(CompileInput {
        name: "Exile Hand Result Probe",
        text: &text,
        score: Some(1.0),
        local_id: 1,
        other_face_id: None,
        other_face_name: None,
        layout: LinkedFaceLayout::None,
    })
    .expect("strict artifact compilation");
    engine::artifact_materializer::materialize_artifact(&artifact).unwrap()
}

#[test]
fn exile_hand_then_draw_preserves_nested_result_id() {
    let definition = compile(
        "Exile all the cards from your hand, then draw that many cards. Until the end of your next turn, you may play cards exiled this way.",
    );
    let program = definition.spell_effect.as_ref().expect("spell program");
    let effects = program.flattened_default_effects();
    let sequence = effects[0]
        .downcast_ref::<engine::effects::SequenceEffect>()
        .expect("comma-then sequence");
    let producer = sequence.effects[0]
        .downcast_ref::<engine::effects::WithIdEffect>()
        .expect("exile result must be recorded");
    let tagged = producer
        .effect
        .downcast_ref::<engine::effects::TaggedEffect>()
        .expect("exiled cards remain tagged for play permission");
    assert!(
        tagged
            .effect
            .downcast_ref::<engine::effects::ExileEffect>()
            .is_some()
    );
    let draw = sequence.effects[1]
        .downcast_ref::<engine::effects::DrawCardsEffect>()
        .expect("draw action");
    assert_eq!(
        draw.count.unhinted(),
        &engine::effect::Value::EffectValue(producer.id)
    );
    assert!(
        effects[1]
            .downcast_ref::<engine::effects::GrantPlayTaggedEffect>()
            .is_some()
    );
}
