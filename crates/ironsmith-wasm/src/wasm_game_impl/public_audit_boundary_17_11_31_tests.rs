// SOURCE ONLY: every executable gate UNRUN. Actual public projection owner.
use super::*;
use ironsmith::ability::RestrictedManaUnit;
use ironsmith::effects::CounterEffect;
use ironsmith::{Effect, ObjectId, PlayerId};
use ironsmith_core::{ChooseSpec, CounterExileGate, CounterExilePermission,
    ManaPaymentPredicate, ManaSpendPayload, ManaSymbol, ManaUsageRestriction,
    ObjectFilter, ResolutionProgram};

#[test]
fn digest11_checkpoint_distinguishes_exact_counter_rider_shape_and_values() {
    let _ids = crate::test_id_counter_guard();
    let mut wasm = WasmGame::new();
    wasm.initialize_empty_match(vec!["A".into(), "B".into()], 20, 1);
    let mut projections = vec![];
    for permission in [None,
        Some(CounterExilePermission { gate: CounterExileGate::AnySpell, allow_land: false }),
        Some(CounterExilePermission { gate: CounterExileGate::PermanentSpell, allow_land: false }),
        Some(CounterExilePermission { gate: CounterExileGate::AnySpell, allow_land: true }),
        Some(CounterExilePermission { gate: CounterExileGate::PermanentSpell, allow_land: true }),
    ] {
        let mut effect = CounterEffect::new(ChooseSpec::target(ChooseSpec::Object(ObjectFilter::spell())));
        effect.exile_permission = permission;
        wasm.game.players[0].restricted_mana = vec![RestrictedManaUnit {
            symbol: ManaSymbol::Blue, source: ObjectId::from_raw(17),
            source_controller: Some(PlayerId::from_index(0)), source_chosen_creature_type: None,
            restrictions: vec![ManaUsageRestriction::PaymentTransaction {
                restriction: Some(ManaPaymentPredicate::Any),
                on_spend: vec![ManaSpendPayload { predicate: ManaPaymentPredicate::Any,
                    effects: ResolutionProgram::from_effects(vec![Effect::new(effect)]), choices: vec![] }],
            }],
        }];
        let typed = sync_restricted_mana(&wasm.game.players[0].restricted_mana).unwrap();
        let ManaUsageRestriction::PaymentTransaction { on_spend, .. } = &typed[0].restrictions[0]
            else { panic!("retain payment carrier") };
        let effects = on_spend[0].effects.all_effects();
        assert_eq!(effects.len(), 1);
        assert_eq!(effects[0].kind(), "CounterEffect");
        let restored: ironsmith_core::CounterEffect = serde_json::from_value(effects[0].payload().clone()).unwrap();
        assert_eq!(restored.exile_permission, permission);
        assert_eq!(effects[0].payload().get("exile_permission").is_some(), permission.is_some());
        let checkpoint = serde_json::to_value(wasm.build_public_audit_checkpoint()).unwrap();
        assert_eq!(PUBLIC_AUDIT_VERSION, 11);
        assert_eq!(checkpoint["version"], 11);
        assert_eq!(checkpoint, serde_json::to_value(wasm.build_public_audit_checkpoint()).unwrap());
        assert!(projections.iter().all(|old| old != &checkpoint));
        projections.push(checkpoint);
    }
}

#[test]
fn public_known_identity_keeps_materialized_canonical_counter_text() {
    // Synthetic presentation input isolates the projection owner. The full
    // compiler/runtime and renderer suites own generation of these strings.
    let _ids = crate::test_id_counter_guard();
    let mut wasm = WasmGame::new();
    wasm.initialize_empty_match(vec!["A".into(), "B".into()], 20, 1);
    let mut definition = ironsmith::cards::builders::CardDefinitionBuilder::new(
        CardId::from_raw(0), "Public canonical counter control")
        .card_types(vec![CardType::Instant]).build();
    for text in ["Counter target spell.",
        "Counter target spell. If a permanent spell is countered this way, exile it instead of putting it into its owner's graveyard. You may cast that card without paying its mana cost for as long as it remains exiled.",
        "Counter target spell. If that spell is countered this way, exile it instead of putting it into its owner's graveyard. You may play that card without paying its mana cost for as long as it remains exiled."] {
        definition.canonical_text = text.into();
        let id = wasm.game.create_object_from_definition(&definition, PlayerId::from_index(0), Zone::Graveyard);
        let object = wasm.game.object(id).unwrap();
        let identity = serde_json::to_value(WasmGame::public_audit_known_object_identity(object)).unwrap();
        assert_eq!(identity["oracleText"], text);
    }
}

#[test]
fn fresh_wasm_constructor_projects_digest11_without_match_initialization() {
    // Source-authored native gate for the genuine constructor. This does not
    // require damaged current state to export before authenticated recovery.
    let _ids = crate::test_id_counter_guard();
    let wasm = WasmGame::new();
    let checkpoint = serde_json::to_value(wasm.build_public_audit_checkpoint()).unwrap();
    assert_eq!(checkpoint["version"], 11);
    assert_eq!(checkpoint, serde_json::to_value(wasm.build_public_audit_checkpoint()).unwrap());
}
