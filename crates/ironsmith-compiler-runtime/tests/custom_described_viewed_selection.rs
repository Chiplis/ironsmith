//! Optional viewed selection must remain optional without a separate "you may".
use ironsmith::{Effect, GameState, PlayerId, Zone};
use ironsmith::effects::{EffectContext, execute_effect};
use ironsmith_compiled_artifact::CompiledCardArtifact;
const A: PlayerId = PlayerId(0);
#[test]
fn up_to_one_face_down_selection_can_select_nothing_when_no_viewed_card_is_eligible() {
    let text = "Type: Sorcery\nLook at the top four cards of your library. Put up to one noncreature card from among them onto the battlefield face down as a 2/2 creature. You may exile and cast it face up any time you have priority if it is a noncreature card with a mana cost. You still pay its costs. Put the rest into your graveyard.";
    let (result, loss) = ironsmith_compiler::parse_loss::capture(|| ironsmith_compiler::word_claims::with_enforcement_options(true, true, || ironsmith_compiler_runtime::compile_to_artifact("Viewed optional witness", text, false)));
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let (artifact, direct) = result.unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, restored);
    for definition in [direct, ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap()] {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let source = game.create_object_from_definition(&definition, A, Zone::Stack);
        let creature = ironsmith_compiler_runtime::compile_to_runtime_definition("Ineligible viewed card", "Mana cost: {G}\nType: Creature\nPower/Toughness: 2/2", false).unwrap();
        let sentinel = game.create_object_from_definition(&creature, A, Zone::Library);
        let viewed: Vec<_> = (0..4).map(|_| { let id=game.create_object_from_definition(&creature,A,Zone::Library);game.object(id).unwrap().stable_id }).collect();
        let effects = definition.spell_effect.as_ref().unwrap().all_effects_owned();
        let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
        execute_effect(&mut game, &Effect::new(ironsmith::effects::SequenceEffect::new(effects)), &mut EffectContext::new_default(source, A).with_decision_maker(&mut dm)).unwrap();
        assert!(game.battlefield.is_empty());
        assert_eq!(game.object(sentinel).unwrap().zone, Zone::Library);
        for stable in viewed { let id=game.find_object_by_stable_id(stable).unwrap();assert_eq!(game.object(id).unwrap().zone,Zone::Graveyard); }
        assert!(game.effect_store.repeatable_mana_payment_actions.iter().all(|action| action.restrictions.face_down_exile_cast.is_none()));
    }
}
