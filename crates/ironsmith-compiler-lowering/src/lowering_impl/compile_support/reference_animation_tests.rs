//! Live regressions relocated from the unwired legacy parse_compile_tests file.
use super::{compile_effect, EffectAst, EffectLoweringContext, TargetAst, Until};
use crate::types::CardType;

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
