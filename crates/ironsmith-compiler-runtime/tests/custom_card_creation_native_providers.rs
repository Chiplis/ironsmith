//! Independent native scope/provider regression. No compiler payload constructs this rule.
use ironsmith::effects::cards::{CardDefinitionScopeEffect, bind_card_creation_scope_runtime};
use ironsmith::effects::{EffectContext, execute_effect};
use ironsmith::static_abilities::{CardCreationSelectionReplacement, StaticAbility};
use ironsmith::types::CardType;
use ironsmith::{
    Ability, CardDefinition, CardDefinitionBuilder, Effect, GameState, PlayerId, Zone,
};
use ironsmith_core::{
    CardCreationLibraryPosition, CardCreationSelection, CardId, CreateCardsEffect, PlayerFilter,
};
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
fn leaf(id: CardId, name: &str, life: i32) -> CardDefinition {
    CardDefinitionBuilder::new(id, name)
        .card_types(vec![CardType::Sorcery])
        .with_spell_effect(vec![Effect::gain_life(life)])
        .build()
}
fn creation(scope: CardId, name: &str, count: i32, label: Option<&str>) -> Effect {
    Effect::new(CreateCardsEffect {
        definitions: vec![],
        definition_scope: Some(scope),
        definition_refs: vec![scope],
        operation_label: label.map(str::to_owned),
        selection: CardCreationSelection::Named(name.into()),
        count: count.into(),
        player: PlayerFilter::You,
        zone: Zone::Hand,
        library_position: CardCreationLibraryPosition::Top,
    })
}
fn provider(scope: CardId) -> CardDefinition {
    let target = leaf(scope, "Provider selected identity", 9);
    let rule = CardCreationSelectionReplacement {
        operation_label: Some("independent native operation".into()),
        player: PlayerFilter::Any,
        replacement: creation(scope, "Provider selected identity", 1, None),
        display: "Use the provider's supplied definition instead.".into(),
    };
    let native = StaticAbility::new(rule);
    assert!(
        native.canonical_model().is_some(),
        "native provider needs typed wrapping/export independently of compiler-model input"
    );
    let root = CardDefinitionBuilder::new(CardId(700002), "Native definition provider")
        .card_types(vec![CardType::Artifact])
        .with_ability(Ability::static_ability(native))
        .build();
    bind_card_creation_scope_runtime(root, scope, &[target]).unwrap()
}
fn resolve_program(game: &mut GameState, definition: &CardDefinition, source: ironsmith::ObjectId) {
    let mut ctx = EffectContext::new_default(source, A);
    for effect in definition
        .spell_effect
        .as_ref()
        .unwrap()
        .flattened_default_effects()
    {
        execute_effect(game, effect, &mut ctx).unwrap();
    }
}
#[test]
fn native_provider_uses_its_own_closed_scope_even_when_creator_reuses_local_scope_number() {
    for artifact in [false, true] {
        let scope = CardId(700001);
        let provider = provider(scope);
        let provider = if artifact {
            let wire =
                ironsmith_runtime_catalog::artifact_materializer::encode_runtime_definition(provider).unwrap();
            ironsmith_runtime_catalog::artifact_materializer::materialize_definition(wire).unwrap()
        } else {
            provider
        };
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        game.create_object_from_definition(&provider, B, Zone::Battlefield);
        let source = game.create_object_from_definition(
            &CardDefinitionBuilder::new(CardId(700003), "Native creator source")
                .card_types(vec![CardType::Artifact])
                .build(),
            A,
            Zone::Battlefield,
        );
        // Equal numeric local scope anchors are deliberate. Lexical provider data
        // must survive the caller's unrelated active table.
        let caller = Effect::new(CardDefinitionScopeEffect {
            scope,
            definitions: vec![leaf(scope, "Creator admitted identity", 1)],
            effect: Box::new(creation(
                scope,
                "Creator admitted identity",
                2,
                Some("independent native operation"),
            )),
        });
        let mut ctx = EffectContext::new_default(source, A);
        let outcome = execute_effect(&mut game, &caller, &mut ctx).unwrap();
        assert_eq!(outcome.as_count(), Some(2));
        let ids = outcome.output_objects().to_vec();
        assert_eq!(ids.len(), 2);
        for id in &ids {
            let object = game.object(*id).unwrap();
            assert_eq!(object.name.as_str(), "Provider selected identity");
            assert_eq!(object.owner, A);
            let definition = object.physical_card_definition();
            resolve_program(&mut game, &definition, *id);
        }
        assert_eq!(
            game.player(A).unwrap().life,
            38,
            "both actual cards retain the provider's life-nine program, not the caller's life-one program"
        );
        let produced = game.object(ids[0]).unwrap().physical_card_definition();
        let wire = ironsmith_runtime_catalog::artifact_materializer::encode_runtime_definition(produced).unwrap();
        let exported = ironsmith_runtime_catalog::artifact_materializer::materialize_definition(wire).unwrap();
        let mut fresh = GameState::new(vec!["A".into(), "B".into()], 20);
        let id = fresh.create_object_from_definition(&exported, A, Zone::Stack);
        resolve_program(&mut fresh, &exported, id);
        assert_eq!(
            fresh.player(A).unwrap().life,
            29,
            "standalone generated definition retains its lexical closure after the original provider is absent"
        );
    }
}


#[test]
fn independently_materialized_same_source_provider_carriers_agree_or_fail_on_conflicting_programs() {
    for conflicting in [false,true] {
        let scope=CardId(700101);
        let mut root=provider(scope);
        let wire=ironsmith_runtime_catalog::artifact_materializer::encode_runtime_definition(root.clone()).unwrap();
        let restored=ironsmith_runtime_catalog::artifact_materializer::materialize_definition(wire).unwrap();
        // Both independently materialized carriers belong to one exact source.
        // They must agree structurally despite distinct native executor Arcs.
        root.abilities.extend(restored.abilities);
        if conflicting {
            let conflicting_template=Effect::new(CardDefinitionScopeEffect {
                scope,definitions:vec![leaf(scope,"Provider selected identity",1)],
                effect:Box::new(creation(scope,"Provider selected identity",1,None)),
            });
            root.abilities.push(Ability::static_ability(StaticAbility::new(CardCreationSelectionReplacement {
                operation_label:Some("independent native operation".into()),player:PlayerFilter::Any,
                replacement:conflicting_template,display:"Conflicting supplied program".into(),
            })));
        }
        let mut game=GameState::new(vec!["A".into(),"B".into()],20);
        let source=game.create_object_from_definition(&root,B,Zone::Battlefield);
        let caller=Effect::new(CardDefinitionScopeEffect {
            scope,definitions:vec![leaf(scope,"Creator admitted identity",1)],
            effect:Box::new(creation(scope,"Creator admitted identity",1,Some("independent native operation"))),
        });
        let mut ctx=EffectContext::new_default(source,A);
        let result=execute_effect(&mut game,&caller,&mut ctx);
        if conflicting {
            assert!(matches!(result,Err(ironsmith::effects::ExecutionError::IncompleteEvidence(_))),
                "same source/CardId/name with different executable programs cannot choose an arbitrary provider");
        } else {
            let outcome=result.expect("equivalent artifact/native carriers are one unambiguous source-owned identity");
            let id=outcome.output_objects()[0];
            let generated=game.object(id).unwrap().physical_card_definition();
            resolve_program(&mut game,&generated,id);
            assert_eq!(game.player(A).unwrap().life,29,"actual creation retains the shared provider's life-nine program");
        }
    }
}
