//! Live regressions relocated from the unwired legacy parse_compile_tests file.
use super::{compile_effect, EffectAst, EffectLoweringContext, TargetAst, Until};
use crate::{effect::Effect, ids::CardId, target::ChooseSpec, types::CardType};
use ironsmith_compiler::{CardDefinitionBuilder, ParseCardText};

#[test]
fn unsized_animation_never_invents_or_freezes_a_base_characteristic() {
    let ast = EffectAst::subject_verb_become_object_template(
        None, TargetAst::Source(None), vec![CardType::Artifact, CardType::Creature],
        vec![], vec![], None, vec![], vec![], false, None, None, None, Until::EndOfTurn,
    );
    let (effects, choices) = compile_effect(&ast, &mut EffectLoweringContext::new()).unwrap();
    assert!(choices.is_empty());
    let debug = format!("{effects:?}");
    assert!(debug.contains("AddCardTypes"));
    assert!(!debug.contains("SetPowerToughness"));
    assert!(!debug.contains("resolve_set_pt_values_at_resolution: true"));
}

#[test]
fn explicit_damage_source_keeps_target_legality_after_reference_capture() {
    let definition = CardDefinitionBuilder::new(CardId::new(), "Targeted damage source probe")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Target creature an opponent controls deals damage equal to its power to another target creature that player controls. The Ring tempts you.")
        .expect("dependent targeted damage should compile");
    fn visit(effect: &Effect, sources: &mut Vec<ChooseSpec>) {
        if let Some(bound) = effect.downcast_ref::<crate::effects::ExecuteWithSourceEffect>() {
            sources.push(bound.source.clone());
        }
        effect.visit_child_effects(&mut |child| visit(child, sources));
    }
    let mut sources = Vec::new();
    for effect in definition.spell_effect.as_ref().unwrap().all_effects() {
        visit(effect, &mut sources);
    }
    assert_eq!(sources.len(), 1, "{sources:#?}");
    assert!(sources[0].is_target(), "an explicit damage source must retain CR 608.2b legality: {sources:#?}");
    assert!(matches!(sources[0].base(), ChooseSpec::Object(_)), "{sources:#?}");
}
