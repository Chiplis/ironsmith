//! Existing token descriptions must install each quoted ability exactly once.
use ironsmith::ability::AbilityKind;
use ironsmith::decision::SelectFirstDecisionMaker;
use ironsmith::effects::{EffectContext, execute_effect};
use ironsmith::{CardDefinition, GameState, PlayerId, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
const A: PlayerId = PlayerId(0);
fn definitions(text: &str) -> [CardDefinition; 2] {
    let (result, loss) = ironsmith_compiler::parse_loss::capture(|| ironsmith_compiler::word_claims::with_enforcement_options(true, true, || ironsmith_compiler_runtime::compile_to_artifact("Token instruction", text, false)));
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let (artifact, direct) = result.unwrap();
    let decoded = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, decoded);
    [direct, ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&decoded).unwrap()]
}
fn create(game: &mut GameState, definition: &CardDefinition) -> ironsmith::ObjectId {
    let source = game.create_object_from_definition(definition, A, Zone::Stack);
    let mut context = EffectContext::new_default(source, A);
    for effect in definition.spell_effect.as_ref().unwrap().all_effects() { execute_effect(game, effect, &mut context).unwrap(); }
    game.battlefield[0]
}
#[test]
fn quoted_pump_is_installed_once_and_intrinsic_flying_survives() {
    for definition in definitions("Type: Sorcery\nCreate a 2/2 red Dragon creature token with flying and \"{R}: This creature gets +1/+0 until end of turn.\".") {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let dragon = create(&mut game, &definition);
        let activated: Vec<_> = game.object(dragon).unwrap().abilities.iter().filter_map(|a| match &a.kind { AbilityKind::Activated(a) => Some(a.clone()), _ => None }).collect();
        assert_eq!(activated.len(), 1, "ordinary quoted reader and legacy red_pump flag must not both install the ability");
        assert!(game.object_has_ability(dragon, &ironsmith::static_abilities::StaticAbility::flying()));
        let mut context = EffectContext::new_default(dragon, A);
        for effect in activated[0].effects.all_effects() { execute_effect(&mut game, effect, &mut context).unwrap(); }
        assert_eq!(game.current_power(dragon), Some(3));
        assert_eq!(game.current_toughness(dragon), Some(2));
    }
}
#[test]
fn quoted_mana_ability_is_installed_once_and_intrinsic_vigilance_survives() {
    for definition in definitions("Type: Sorcery\nCreate a 1/1 green Elf creature token with vigilance and \"{T}: Add {G}.\".") {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let elf = create(&mut game, &definition);
        game.remove_summoning_sickness(elf);
        let abilities: Vec<_> = game.object(elf).unwrap().abilities.iter().enumerate().filter_map(|(i,a)| matches!(a.kind, AbilityKind::Activated(_)).then_some(i)).collect();
        assert_eq!(abilities.len(), 1, "ordinary quoted reader and legacy tap_mana flag must not both install the ability");
        assert!(game.object_has_ability(elf, &ironsmith::static_abilities::StaticAbility::vigilance()));
        ironsmith::special_actions::perform_activate_mana_ability(&mut game, A, elf, abilities[0], &mut SelectFirstDecisionMaker).unwrap();
        assert!(game.is_tapped(elf));
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 1);
    }
}
