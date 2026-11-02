//! Generic selected characteristic-conversion costs; requires the scratch overwrite and cost patches.
use ironsmith::{GameState,PlayerId,Zone};
use ironsmith::decision::{compute_legal_actions,LegalAction};
use ironsmith_compiled_artifact::CompiledCardArtifact;
#[path="cf8_p08/play.rs"] mod play;
const A:PlayerId=PlayerId(0);
const B:PlayerId=PlayerId(1);
fn definitions(name:&str,text:&str)->[ironsmith::cards::CardDefinition;2] {
    let (result,loss)=ironsmith_compiler::parse_loss::capture(||ironsmith_compiler::word_claims::with_enforcement_options(true,true,||ironsmith_compiler_runtime::compile_to_artifact(name,text,false)));
    assert!(!loss.is_lossy(),"{}",loss.reasons_text());
    let (artifact,direct)=result.unwrap_or_else(|e|panic!("{name}: {text}\n{e:?}"));
    let decoded=CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(decoded,artifact);
    [direct,ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&decoded).unwrap()]
}
#[test]
fn selected_conversion_cost_requires_exact_own_eligible_set_and_cannot_reuse_it() {
    let text="Type: Creature\nPower/Toughness: 1/1\nHave two lands you control not named Wastes become a basic land named Wastes with \"{T}: Add {C}\" and loses all other supertypes, card types, subtypes, and abilities: You gain 1 life.";
    let land=definitions("Grove","Type: Legendary Snow Land — Forest\n{T}: Add {G}.").into_iter().next().unwrap();
    let already=definitions("Wastes","Type: Basic Land\n{T}: Add {C}.").into_iter().next().unwrap();
    for definition in definitions("Conversion witness",text) {for resources in [0,1,2,3] {
        let mut game=play::game();
        let source=game.create_object_from_definition(&definition,A,Zone::Battlefield);
        let index=play::activated_at(&definition,0);
        let own=(0..resources).map(|_|game.create_object_from_definition(&land,A,Zone::Battlefield)).collect::<Vec<_>>();
        let opponent=game.create_object_from_definition(&land,B,Zone::Battlefield);
        let wastes=game.create_object_from_definition(&already,A,Zone::Battlefield);
        let action=LegalAction::ActivateAbility {source,ability_index:index};
        assert_eq!(compute_legal_actions(&game,A).unwrap().contains(&action),resources>=2,"resources={resources}");
        if resources<2 {continue;}
        play::activate(&mut game,source,index,&mut play::Script::default());
        assert_eq!(game.stack.len(),1);
        let converted=own.iter().filter(|id|game.current_characteristics(**id).unwrap().name.as_ref()=="Wastes").count();
        assert_eq!(converted,2,"conversion pays before the ability resolves");
        for id in own.iter().filter(|id|game.current_characteristics(**id).unwrap().name.as_ref()=="Wastes") {
            let view=game.current_characteristics(*id).unwrap();
            assert_eq!(view.card_types.as_slice(),&[ironsmith::types::CardType::Land]);
            assert_eq!(view.supertypes.as_slice(),&[ironsmith::types::Supertype::Basic]);
            assert!(view.subtypes.is_empty());
            let abilities=game.current_abilities(*id).unwrap();assert_eq!(abilities.len(),1);assert!(abilities[0].is_mana_ability());
        }
        assert_eq!(game.player(A).unwrap().life,20);
        assert_eq!(game.current_characteristics(opponent).unwrap().name.as_ref(),"Grove");
        assert_eq!(game.current_characteristics(wastes).unwrap().name.as_ref(),"Wastes");
        assert!(!compute_legal_actions(&game,A).unwrap().contains(&action),"the same converted lands cannot pay again");
        // Countering the ability does not reverse its characteristic payment.
        game.stack.pop();
        assert_eq!(own.iter().filter(|id|game.current_characteristics(**id).unwrap().name.as_ref()=="Wastes").count(),2);
    }}
}
#[test]
fn conversion_cost_is_not_tied_to_wastes_name_or_land_type() {
    let text="Type: Artifact\nHave a creature you control not named Relic become a legendary artifact named Relic and loses all other card types and subtypes: You gain 1 life.";
    for definition in definitions("Relic forge",text) {
        let mut game=play::game();
        let source=game.create_object_from_definition(&definition,A,Zone::Battlefield);
        let body=definitions("Apprentice","Type: Creature — Human\nPower/Toughness: 1/1").into_iter().next().unwrap();
        let object=game.create_object_from_definition(&body,A,Zone::Battlefield);
        let index=play::activated_at(&definition,0);
        play::activate(&mut game,source,index,&mut play::Script::default());
        let view=game.current_characteristics(object).unwrap();
        assert_eq!(view.name.as_ref(),"Relic");
        assert_eq!(view.card_types.as_slice(),&[ironsmith::types::CardType::Artifact]);
        assert!(view.supertypes.contains(&ironsmith::types::Supertype::Legendary));
        assert!(view.subtypes.is_empty());
    }
}
#[test]
fn compound_conversion_costs_cannot_double_use_one_land_and_failed_costs_leave_no_mutation() {
    let conversion="Have a land you control not named Wastes become a basic land named Wastes with \"{T}: Add {C}\" and loses all other supertypes, card types, subtypes, and abilities";
    let land=definitions("Reserve","Type: Land\n{T}: Add {G}.").into_iter().next().unwrap();
    for suffix in [conversion.to_string(),"Pay 3 life".to_string()] {
        let text=format!("Type: Artifact\n{conversion}, {suffix}: You gain 1 life.");
        for definition in definitions("Compound converter",&text) { for resources in [1,2] {
            let mut game=play::game();
            let source=game.create_object_from_definition(&definition,A,Zone::Battlefield);
            let own=(0..resources).map(|_|game.create_object_from_definition(&land,A,Zone::Battlefield)).collect::<Vec<_>>();
            if suffix=="Pay 3 life" {game.player_mut(A).unwrap().life=2;}
            let index=play::activated_at(&definition,0);
            let action=LegalAction::ActivateAbility {source,ability_index:index};
            let expected=suffix!="Pay 3 life"&&resources>=2;
            assert_eq!(compute_legal_actions(&game,A).unwrap().contains(&action),expected);
            if expected {
                play::activate(&mut game,source,index,&mut play::Script::default());
                assert_eq!(own.iter().filter(|id|game.current_characteristics(**id).unwrap().name.as_ref()=="Wastes").count(),2);
            }else{
                assert!(own.iter().all(|id|game.current_characteristics(*id).unwrap().name.as_ref()=="Reserve"));
                assert_eq!(game.player(A).unwrap().life,if suffix=="Pay 3 life" {2}else{20});
                assert!(game.stack.is_empty());
            }
        }}
    }
}
