use ironsmith::effects::ResolvedTarget;
use ironsmith::effects::{EffectContext, execute_effect};
use ironsmith::types::Subtype;
use ironsmith::{AbilityKind, CardType, GameState, PlayerId, Zone};
use ironsmith_tools::{
    ParseStatus, compile_authoritative_snapshot_from_payload, compile_definition_from_payload,
    default_cards_path, load_card_payloads_by_name,
};

#[test]
fn factory_strict_compile_animation_pump_and_cleanup() {
    let payloads =
        load_card_payloads_by_name(default_cards_path().to_str().unwrap(), "Mishra's Factory")
            .unwrap();
    assert_eq!(payloads.len(), 1);
    let snapshot = compile_authoritative_snapshot_from_payload(&payloads[0]);
    assert_eq!(
        snapshot.parse_status,
        ParseStatus::StrictCompiled,
        "{snapshot:#?}"
    );
    assert!(!snapshot.parse_lossy && !snapshot.has_unimplemented);
    assert!(snapshot.similarity_score >= 0.99, "{snapshot:#?}");
    let definition = compile_definition_from_payload(&payloads[0]).unwrap();
    assert_eq!(definition.abilities.len(), 3);
    let abilities = definition
        .abilities
        .iter()
        .map(|a| {
            let AbilityKind::Activated(a) = &a.kind else {
                panic!("expected activation")
            };
            a
        })
        .collect::<Vec<_>>();
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let pump_effects = abilities[2].effects.flattened_default_effects();
    let pump = pump_effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<ironsmith::effects::ApplyContinuousEffect>())
        .expect("pump must have a structured continuous effect");
    let spec = pump.target_spec.as_ref().expect("pump must target");
    assert!(
        ironsmith::targeting::compute_legal_targets(&game, spec, alice, Some(source)).is_empty()
    );
    let mut ctx = EffectContext::new_default(source, alice);
    for effect in abilities[1].effects.flattened_default_effects() {
        execute_effect(&mut game, effect, &mut ctx).unwrap();
    }
    let animated = game.calculated_characteristics(source).unwrap();
    for kind in [CardType::Land, CardType::Artifact, CardType::Creature] {
        assert!(animated.card_types.contains(&kind), "{animated:#?}");
    }
    assert!(animated.subtypes.contains(&Subtype::AssemblyWorker));
    assert_eq!((animated.power, animated.toughness), (Some(2), Some(2)));
    assert_eq!(
        ironsmith::targeting::compute_legal_targets(&game, spec, alice, Some(source)),
        vec![ironsmith::game_state::Target::Object(source)]
    );
    for (types, subtype, legal) in [
        (vec![CardType::Creature], Subtype::AssemblyWorker, true),
        (vec![CardType::Creature], Subtype::Goblin, false),
        (vec![CardType::Artifact], Subtype::AssemblyWorker, false),
    ] {
        let mut fixture = ironsmith::cards::builders::CardDefinitionBuilder::new(
            ironsmith::ids::CardId::new(),
            "Target fixture",
        )
        .card_types(types)
        .build();
        fixture.card.subtypes.push(subtype);
        let other = game.create_object_from_definition(
            &fixture,
            PlayerId::from_index(1),
            Zone::Battlefield,
        );
        assert_eq!(
            ironsmith::targeting::compute_legal_targets(&game, spec, alice, Some(source))
                .contains(&ironsmith::game_state::Target::Object(other)),
            legal
        );
    }
    let mut ctx = EffectContext::new_default(source, alice)
        .with_targets(vec![ResolvedTarget::Object(source)]);
    for effect in abilities[2].effects.flattened_default_effects() {
        execute_effect(&mut game, effect, &mut ctx).unwrap();
    }
    let pumped = game.calculated_characteristics(source).unwrap();
    assert_eq!((pumped.power, pumped.toughness), (Some(3), Some(3)));
    ironsmith::turn::execute_cleanup_step(&mut game);
    let expired = game.calculated_characteristics(source).unwrap();
    assert_eq!(expired.card_types, [CardType::Land]);
    assert!(!expired.subtypes.contains(&Subtype::AssemblyWorker));
    assert_eq!((expired.power, expired.toughness), (None, None));
}
