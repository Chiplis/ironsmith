//! UNVALIDATED source scenarios; no execution evidence is claimed.
use ironsmith::ability::{Ability, AbilityKind};
use ironsmith::cards::CardDefinition;
use ironsmith::effects::{EffectContext, execute_effect};
use ironsmith::{GameState, PlayerId, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_core::ActivatedAbilityKeyword;
const A: PlayerId = PlayerId::from_index(0);
fn definitions(name: &str, text: &str) -> [CardDefinition; 2] {
    let direct = compile_to_runtime_definition(name, text, false).unwrap();
    let (artifact, _) = compile_to_artifact(name, text, false).unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, restored);
    [direct, ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap()]
}
fn keywords(abilities: &[Ability]) -> Vec<Option<ActivatedAbilityKeyword>> {
    abilities.iter().filter_map(|a| match &a.kind {
        AbilityKind::Activated(a) => Some(a.keyword), _ => None,
    }).collect()
}
#[test]
fn direct_and_artifact_paths_keep_keyword_identity_not_effect_resemblance() {
    for d in definitions("Attachment identity", "Type: Artifact — Equipment\nEquip {1}\n{1}: Attach this Equipment to target creature you control.") {
        assert_eq!(keywords(&d.abilities), vec![Some(ActivatedAbilityKeyword::Equip), None]);
    }
    for d in definitions("Power identity", "Type: Creature — Human\nPower/Toughness: 2/2\nPower-up — {4}{R}: Put two +1/+1 counters on this creature.\n{4}{R}: Put two +1/+1 counters on this creature.") {
        assert_eq!(keywords(&d.abilities), vec![Some(ActivatedAbilityKeyword::PowerUp), None]);
    }
}
#[test]
fn conditional_continuous_equip_grant_keeps_identity() {
    for grant in definitions("Equip grant", "Type: Enchantment\nMetalcraft — Equipment you control have equip {0} as long as you control three artifacts.") {
        let mut game = GameState::new(vec!["Alice".into(),"Bob".into()],20);
        let d = compile_to_runtime_definition("Host", "Type: Artifact — Equipment\n{1}: Attach this Equipment to target creature you control.",false).unwrap();
        let source = game.create_object_from_definition(&d,A,Zone::Battlefield);
        let d = compile_to_runtime_definition("Artifact","Type: Artifact",false).unwrap();
        game.create_object_from_definition(&d,A,Zone::Battlefield);
        game.create_object_from_definition(&d,A,Zone::Battlefield);
        game.create_object_from_definition(&grant,A,Zone::Battlefield);
        let ids = keywords(&game.current_abilities(source).unwrap());
        assert!(ids.contains(&Some(ActivatedAbilityKeyword::Equip)));
        assert!(ids.contains(&None));
    }
}
#[test]
fn generated_token_equip_keeps_identity() {
    for d in definitions("Make Rock", "Type: Sorcery\nCreate a colorless Equipment artifact token named Rock with \"Equipped creature gets +1/+0\" and equip {1}.") {
        let mut game = GameState::new(vec!["Alice".into(),"Bob".into()],20);
        let source = game.create_object_from_definition(&d,A,Zone::Stack);
        let mut ctx = EffectContext::new_default(source,A);
        for effect in d.spell_effect.as_ref().unwrap().flattened_default_effects() {
            execute_effect(&mut game,&effect,&mut ctx).unwrap();
        }
        let rock = game.battlefield.iter().filter_map(|id|game.object(*id)).find(|o|o.name=="Rock").unwrap();
        assert_eq!(keywords(&rock.abilities),vec![Some(ActivatedAbilityKeyword::Equip)]);
    }
}
#[test]
fn plain_old_ability_json_defaults_to_no_keyword() {
    type Activated = ironsmith_core::ActivatedAbility<(), ironsmith_core::Cost<()>>;
    let ordinary = Activated::basic_mana(ironsmith_core::ManaSymbol::Green);
    let mut json = serde_json::to_value(&ordinary).unwrap();
    assert!(json.get("keyword").is_none());
    assert_eq!(serde_json::from_value::<Activated>(json.clone()).unwrap().keyword,None);
    json["keyword"]=serde_json::json!("Equip");
    assert_eq!(serde_json::from_value::<Activated>(json).unwrap().keyword,Some(ActivatedAbilityKeyword::Equip));
}
