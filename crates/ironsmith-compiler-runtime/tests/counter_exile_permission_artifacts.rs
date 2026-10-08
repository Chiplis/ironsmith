//! Grammar-independent typed artifact controls. Authored only; all tests UNRUN.
//! These synthetic definitions establish no recovery credit for frozen cards.
use ironsmith::ability::Ability;
use ironsmith::card::PowerToughness;
use ironsmith::cards::{CardDefinition, builders::CardDefinitionBuilder};
use ironsmith::decision::{LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
use ironsmith::effect::Effect;
use ironsmith::effects::{CounterEffect, EffectContext, ResolvedTarget, execute_effect};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm,
};
use ironsmith::game_state::{Phase, StackEntry};
use ironsmith::mana::{ManaCost, ManaSymbol};
use ironsmith::target::{ChooseSpec, ObjectFilter};
use ironsmith::triggers::TriggerQueue;
use ironsmith::{CardId, CardType, GameState, ObjectId, PlayerId, Zone};
use ironsmith_compiled_artifact::{
    ArtifactCardId, ArtifactCardIdentity, CompiledCardArtifact, CompiledCardPayload, WireEffect,
};
use ironsmith_core::{CounterExileGate, CounterExilePermission};
use ironsmith_runtime_catalog::artifact_materializer::{
    encode_runtime_definition, encode_runtime_effect, materialize_artifact,
};

const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);

fn native(permission: Option<CounterExilePermission>) -> CardDefinition {
    let mut counter = CounterEffect::new(ChooseSpec::target(ChooseSpec::Object(ObjectFilter::spell())));
    counter.exile_permission = permission;
    CardDefinitionBuilder::new(CardId::from_raw(0), "Typed counter permission control")
        .card_types(vec![CardType::Instant])
        .with_spell_effect(vec![Effect::new(counter)])
        .build()
}

fn artifact(definition: &CardDefinition) -> CompiledCardArtifact {
    CompiledCardArtifact::new(
        ArtifactCardIdentity {
            local_id: ArtifactCardId(0),
            name: definition.card.name.clone(),
            face_name: None,
            other_face: None,
            linked_face_layout: None,
        },
        CompiledCardPayload {
            definition: encode_runtime_definition(definition.clone()).unwrap(),
            canonical_text: ironsmith_text::canonical_compiled_lines(definition).join("\n"),
            ability_labels: vec![],
        },
        "source-only-native-control",
        b"synthetic typed counter permission; no Oracle recovery claim",
    )
}

fn counter(definition: &CardDefinition) -> &CounterEffect {
    definition.spell_effect.as_ref().unwrap().segments[0].default_effects[0]
        .downcast_ref::<CounterEffect>().unwrap()
}

fn routes(permission: Option<CounterExilePermission>) -> [CardDefinition; 2] {
    let native = native(permission);
    let artifact = artifact(&native);
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, restored);
    let materialized = materialize_artifact(&restored).unwrap();
    assert_eq!(counter(&native), counter(&materialized));
    [native, materialized]
}

fn change_payload(artifact: &mut CompiledCardArtifact, change: impl FnOnce(&mut serde_json::Value)) {
    let effect = &mut artifact.payload.definition.spell_effect.as_mut().unwrap()
        .segments[0].default_effects[0];
    assert_eq!(effect.kind(), "CounterEffect");
    let mut payload = effect.payload().clone();
    change(&mut payload);
    *effect = WireEffect::new("CounterEffect", payload);
}

#[test]
fn atomic_rider_roundtrips_without_a_separate_tagged_consumer() {
    use ironsmith_core::tag::TagKeyWalk;
    for gate in [CounterExileGate::AnySpell, CounterExileGate::PermanentSpell] {
        for allow_land in [false, true] {
            let permission = CounterExilePermission { gate, allow_land };
            for definition in routes(Some(permission)) {
                let effects = definition.spell_effect.as_ref().unwrap().all_effects();
                assert_eq!(effects.len(), 1, "producer and priced permission are atomic");
                assert_eq!(counter(&definition).exile_permission, Some(permission));
                let mut tags = vec![];
                counter(&definition).for_each_tag_key(&mut |tag| tags.push(tag.clone()));
                assert!(tags.is_empty(), "the rider must not manufacture a dangling consumer");
                let encoded = encode_runtime_effect((*effects[0]).clone()).unwrap();
                assert_eq!(encoded.payload()["exile_permission"], serde_json::to_value(permission).unwrap());
                let cloned = definition.clone();
                assert_eq!(counter(&cloned), counter(&definition));
                let text = ironsmith_text::canonical_compiled_lines(&definition).join("\n");
                assert!(text.contains(if gate == CounterExileGate::PermanentSpell {
                    "If a permanent spell is countered this way"
                } else { "If that spell is countered this way" }));
                assert!(text.contains(if allow_land { "You may play it" } else { "You may cast that card" }));
            }
        }
    }
}

#[test]
fn absent_rider_keeps_legacy_wire_bytes_and_plain_counter_semantics() {
    let definition = native(None);
    let baseline = artifact(&definition);
    let bytes = baseline.to_json().unwrap();
    assert!(!std::str::from_utf8(&bytes).unwrap().contains("exile_permission"));
    let restored = CompiledCardArtifact::from_json(&bytes).unwrap();
    assert_eq!(restored.to_json().unwrap(), bytes);
    for definition in routes(None) {
        assert!(counter(&definition).exile_permission.is_none());
        assert_eq!(ironsmith_text::canonical_compiled_lines(&definition), vec!["Counter target spell."]);
    }
}

#[test]
fn native_stack_program_clone_keeps_atomic_rider_and_retained_effect_model() {
    let permission = CounterExilePermission {
        gate: CounterExileGate::PermanentSpell, allow_land: false,
    };
    for definition in routes(Some(permission)) {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let mut entry = StackEntry::new(source, A);
        entry.is_ability = true;
        entry.ability_effects = definition.spell_effect.clone();
        game.stack.push(entry);
        let saved = game.clone();
        game.stack.clear();
        let retained = &saved.stack[0].ability_effects.as_ref().unwrap()
            .segments[0].default_effects[0];
        assert_eq!(retained.downcast_ref::<CounterEffect>().unwrap().exile_permission, Some(permission));
        assert_eq!(encode_runtime_effect(retained.clone()).unwrap().payload()["exile_permission"],
            serde_json::to_value(permission).unwrap());
    }
}

#[test]
fn triggered_artifact_keeps_the_entire_atomic_counter_body() {
    let model = counter(&native(Some(CounterExilePermission {
        gate: CounterExileGate::AnySpell, allow_land: false,
    }))).clone();
    let definition = CardDefinitionBuilder::new(CardId::from_raw(0), "Triggered counter artifact control")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(3, 3))
        .with_ability(Ability::triggered(ironsmith::triggers::Trigger::this_is_turned_face_up(),
            vec![Effect::new(model)]))
        .build();
    let envelope = artifact(&definition);
    let decoded = CompiledCardArtifact::from_json(&envelope.to_json().unwrap()).unwrap();
    for definition in [definition, materialize_artifact(&decoded).unwrap()] {
        let text = ironsmith_text::canonical_compiled_lines(&definition).join("\n");
        assert!(text.contains("is turned face up, counter target spell"), "{text}");
        assert!(text.contains("If that spell is countered this way, exile it instead"), "{text}");
        assert!(text.contains("You may cast that card without paying its mana cost for as long as it remains exiled"), "{text}");
    }
}

#[test]
fn nested_public_effect_projection_distinguishes_permission_contracts() {
    // This is the same typed mapping used by public_audit::sync_restricted_mana.
    // It is not a substitute for an authorized full WASM checkpoint/digest gate.
    let mut projections = vec![];
    for permission in [
        None,
        Some(CounterExilePermission { gate: CounterExileGate::AnySpell, allow_land: false }),
        Some(CounterExilePermission { gate: CounterExileGate::PermanentSpell, allow_land: false }),
        Some(CounterExilePermission { gate: CounterExileGate::AnySpell, allow_land: true }),
    ] {
        let definition = native(permission);
        let restriction = ironsmith_core::ManaUsageRestriction::PaymentTransaction {
            restriction: Some(ironsmith_core::ManaPaymentPredicate::Any),
            on_spend: vec![ironsmith_core::ManaSpendPayload {
                predicate: ironsmith_core::ManaPaymentPredicate::Any,
                effects: definition.spell_effect.unwrap(), choices: vec![],
            }],
        };
        let mapped = restriction.try_map_effects(&mut encode_runtime_effect).unwrap();
        let encoded = serde_json::to_value(mapped).unwrap();
        assert_eq!(encoded.to_string().contains("exile_permission"), permission.is_some());
        assert!(projections.iter().all(|previous| previous != &encoded),
            "public typed projection must not collapse a dropped gate or play/cast distinction");
        projections.push(encoded);
    }
}

#[test]
fn incomplete_or_unknown_nested_gate_never_materializes_as_plain_counter() {
    use serde_json::json;
    let baseline = artifact(&native(Some(CounterExilePermission {
        gate: CounterExileGate::PermanentSpell, allow_land: false,
    })));
    for malformed in [
        json!({ "allow_land": false }),
        json!({ "gate": null, "allow_land": false }),
        json!({ "gate": "AnyPermanent", "allow_land": false }),
        json!({ "gate": "PermanentSpell" }),
        json!({ "gate": "PermanentSpell", "allow_land": 0 }),
        json!({ "gate": "PermanentSpell", "allow_land": false, "tag": "stale" }),
    ] {
        let mut malformed_artifact = baseline.clone();
        change_payload(&mut malformed_artifact, |payload| payload["exile_permission"] = malformed);
        malformed_artifact.refresh_checksum();
        // Envelope integrity is distinct from typed executable admission.
        let restored = CompiledCardArtifact::from_json(&malformed_artifact.to_json().unwrap()).unwrap();
        assert!(materialize_artifact(&restored).is_err());
    }
    let mut dropped = baseline.clone();
    change_payload(&mut dropped, |payload| {
        payload.as_object_mut().unwrap().remove("exile_permission");
    });
    assert!(dropped.validate().is_err(), "unacknowledged deletion changes the checksum");
    assert!(materialize_artifact(&dropped).is_err(), "never bypass a rejected envelope");
    // Deliberately no assertion that a rechecksummed legacy-shaped payload is
    // rejected: absent means None for old payloads. Source semantic comparison
    // and a later release boundary own detection of an entirely dropped rider.
}

#[test]
fn artifact_permission_refuses_dangling_or_broad_target_contracts() {
    let baseline = artifact(&native(Some(CounterExilePermission {
        gate: CounterExileGate::AnySpell, allow_land: false,
    })));
    let mut source_filter = ObjectFilter::spell();
    source_filter.source = true;
    let mut nested_tag_filter = ObjectFilter::spell();
    nested_tag_filter.any_of.push(ObjectFilter::default().match_tagged(
        "stale", ironsmith_core::filter_model::TaggedOpbjectRelation::IsTaggedObject));
    for target in [
        ChooseSpec::Object(source_filter),
        ChooseSpec::target(ChooseSpec::Object(nested_tag_filter)),
        ChooseSpec::Tagged("missing_counter_output".into()),
        ChooseSpec::All(ObjectFilter::spell()),
        ChooseSpec::target(ChooseSpec::Object(ObjectFilter::creature())),
    ] {
        let mut invalid = baseline.clone();
        change_payload(&mut invalid, |payload| payload["target"] = serde_json::to_value(target).unwrap());
        invalid.refresh_checksum();
        let restored = CompiledCardArtifact::from_json(&invalid.to_json().unwrap()).unwrap();
        assert!(materialize_artifact(&restored).is_err(), "no plain-counter fallback on a bad rider target");
    }
}

fn game() -> GameState {
    let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
    game.turn.active_player = A;
    game.turn.priority_player = Some(A);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game
}

fn target_definition(kind: CardType, protected: bool, life_cost: u32) -> CardDefinition {
    let mut builder = CardDefinitionBuilder::new(CardId::new(), "Countered artifact witness")
        .card_types(vec![kind])
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(7)]]))
        .power_toughness(PowerToughness::fixed(2, 2));
    if protected {
        builder = builder.with_ability(Ability::static_ability(
            ironsmith::static_abilities::StaticAbility::cant_be_countered_ability(),
        ));
    }
    if life_cost != 0 {
        builder = builder.additional_cost(ironsmith::cost::TotalCost::from_cost(
            ironsmith::costs::Cost::life(life_cost),
        ));
    }
    builder.build()
}

fn resolve_counter(game: &mut GameState, definition: &CardDefinition,
    target_definition: &CardDefinition) -> (ObjectId, ObjectId, ObjectId) {
    let source = game.create_object_from_definition(definition, A, Zone::Battlefield);
    let target = game.create_object_from_definition(target_definition, B, Zone::Stack);
    let stable = game.object(target).unwrap().stable_id;
    game.stack.push(StackEntry::new(target, B));
    let mut ctx = EffectContext::new_default(source, A)
        .with_targets(vec![ResolvedTarget::Object(target)]);
    execute_effect(game, &definition.spell_effect.as_ref().unwrap().segments[0].default_effects[0], &mut ctx).unwrap();
    (source, target, game.find_object_by_stable_id(stable).unwrap())
}

#[test]
fn restored_counter_grants_only_after_its_successful_matching_exile() {
    for gate in [CounterExileGate::AnySpell, CounterExileGate::PermanentSpell] {
        for definition in routes(Some(CounterExilePermission { gate, allow_land: false })) {
            for kind in [CardType::Creature, CardType::Instant, CardType::Sorcery] {
                for protected in [false, true] {
                    let mut game = game();
                    let target_definition = target_definition(kind, protected, 0);
                    let (_, before, after) = resolve_counter(&mut game, &definition, &target_definition);
                    let granted = !protected && (gate == CounterExileGate::AnySpell || kind == CardType::Creature);
                    assert_eq!(game.object(after).unwrap().zone, if protected { Zone::Stack }
                        else if granted { Zone::Exile } else { Zone::Graveyard });
                    let grants = game.effect_store.grant_registry.granted_alternative_casts_for_card(
                        &game, after, Zone::Exile, A);
                    assert_eq!(grants.len(), usize::from(granted));
                    if granted {
                        assert_ne!(before, after);
                        assert!(grants[0].method.total_cost().unwrap().is_free());
                        assert!(game.effect_store.grant_registry.grants.iter().all(|grant|
                            grant.target_id == Some(after) && grant.target_stable_id.is_none()));
                    }
                    assert!(game.effect_store.grant_registry.granted_alternative_casts_for_card(
                        &game, after, Zone::Exile, B).is_empty());
                }
            }
        }
    }
    for definition in routes(None) {
        let mut game = game();
        let (_, _, after) = resolve_counter(&mut game, &definition,
            &target_definition(CardType::Creature, false, 0));
        assert_eq!(game.object(after).unwrap().zone, Zone::Graveyard);
        assert!(game.effect_store.grant_registry.grants.is_empty());
    }
}

#[test]
fn play_rider_adds_only_a_land_domain_without_an_unpriced_spell_origin() {
    for allow_land in [false, true] {
        for definition in routes(Some(CounterExilePermission {
            gate: CounterExileGate::AnySpell, allow_land,
        })) {
            let mut game = game();
            let (_, _, exiled) = resolve_counter(&mut game, &definition,
                &target_definition(CardType::Creature, false, 0));
            let grants = &game.effect_store.grant_registry.grants;
            assert_eq!(grants.len(), if allow_land { 2 } else { 1 });
            assert!(grants.iter().all(|grant| grant.target_id == Some(exiled)
                && grant.target_stable_id.is_none()));
            // The creature face is accessible only through the priced grant.
            assert!(!game.effect_store.grant_registry.card_can_play_from_zone(
                &game, exiled, Zone::Exile, A));
            assert_eq!(game.effect_store.grant_registry.granted_alternative_casts_for_card(
                &game, exiled, Zone::Exile, A).len(), 1);
            let land = grants.iter().find(|grant| matches!(&grant.grantable,
                ironsmith::grant::Grantable::PlayFrom));
            assert_eq!(land.is_some(), allow_land);
            if let Some(land) = land {
                assert_eq!(land.filter.as_ref().unwrap().card_types, vec![CardType::Land]);
            }
        }
    }
}

fn cast_actions(game: &GameState, player: PlayerId, id: ObjectId) -> Vec<LegalAction> {
    compute_legal_actions(game, player).unwrap().into_iter().filter(|action|
        matches!(action, LegalAction::CastSpell { spell_id, from_zone: Zone::Exile, .. }
            if *spell_id == id)).collect()
}

fn finish_cast(game: &mut GameState, action: LegalAction) {
    let mut state = PriorityLoopState::new(game.players.len());
    let mut queue = TriggerQueue::new();
    let mut dm = SelectFirstDecisionMaker;
    let mut progress = apply_priority_response_with_dm(game, &mut queue, &mut state,
        &PriorityResponse::PriorityAction(action), &mut dm).unwrap();
    for _ in 0..40 {
        if !state.has_pending_action() { return; }
        let ironsmith::GameProgress::NeedsDecisionCtx(context) = progress else {
            panic!("cast pending without a decision: {progress:?}");
        };
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &context, &mut dm).unwrap();
    }
    panic!("counter-granted cast did not finish");
}

#[test]
fn direct_and_restored_permission_casts_free_but_pays_mandatory_cost_after_source_leaves() {
    for definition in routes(Some(CounterExilePermission {
        gate: CounterExileGate::PermanentSpell, allow_land: false,
    })) {
        let mut game = game();
        let (source, _, exiled) = resolve_counter(&mut game, &definition,
            &target_definition(CardType::Creature, false, 3));
        let stable = game.object(exiled).unwrap().stable_id;
        game.move_object_by_game_rule(source, Zone::Graveyard).unwrap();
        game.turn.turn_number += 4;
        game.turn.active_player = B;
        assert!(cast_actions(&game, A, exiled).is_empty(), "permission does not change sorcery timing");
        game.turn.active_player = A;
        let mut poor = game.clone();
        poor.player_mut(A).unwrap().life = 2;
        assert!(cast_actions(&poor, A, exiled).is_empty(), "mandatory additional costs remain payable");
        let saved = game.clone();
        for mut branch in [game, saved] {
            let action = cast_actions(&branch, A, exiled).into_iter().next().expect("zero-mana free cast");
            finish_cast(&mut branch, action);
            let cast = branch.find_object_by_stable_id(stable).unwrap();
            assert_ne!(cast, exiled);
            assert_eq!(branch.object(cast).unwrap().zone, Zone::Stack);
            assert_eq!(branch.object(cast).unwrap().caster_mana_spent_to_cast, Some(0));
            assert_eq!(branch.player(A).unwrap().life, 17);
            let returned = branch.move_object_by_game_rule(cast, Zone::Exile).unwrap();
            assert!(branch.effect_store.grant_registry.granted_alternative_casts_for_card(
                &branch, returned, Zone::Exile, A).is_empty(), "a new incarnation cannot revive the free price");
        }
    }
}

#[test]
fn direct_model_interpretation_rejects_contextual_counter_targets() {
    let mut source_filter = ObjectFilter::spell();
    source_filter.source = true;
    let mut nested_tag_filter = ObjectFilter::spell();
    nested_tag_filter.any_of.push(ObjectFilter::default().match_tagged(
        "stale", ironsmith_core::filter_model::TaggedOpbjectRelation::IsTaggedObject));
    for target in [ChooseSpec::Object(source_filter), ChooseSpec::target(ChooseSpec::Object(nested_tag_filter))] {
        for marked in [false, true] {
            let mut counter = CounterEffect::new(target.clone());
            if marked {
                counter.exile_permission = Some(CounterExilePermission {
                    gate: CounterExileGate::PermanentSpell, allow_land: false,
                });
            }
            assert_eq!(counter.exile_permission_target_is_supported(), !marked);
            let mut builder = ironsmith_compiler::CardDefinitionBuilder::new(CardId::new(), "Direct selector contract")
                .card_types(vec![CardType::Instant]);
            builder.spell_effect = Some(ironsmith_core::resolution_model::ResolutionProgram::from_effects(
                vec![ironsmith_compiler::effect::Effect::new(counter)]));
            let result = ironsmith_compiler_runtime::into_runtime_definition(builder.build());
            assert_eq!(result.is_err(), marked, "legacy vocabulary remains unchanged; the atomic rider is closed");
        }
    }
    let hinted = CounterEffect::new(ChooseSpec::spell().with_surface_hint(
        ironsmith_core::ChooseSpecSurfaceHint::SourceReference(
            ironsmith_core::SourceReferenceSurface::FullName("Presentation only".into()))))
        .with_exile_permission(CounterExilePermission { gate: CounterExileGate::AnySpell, allow_land: true });
    assert!(hinted.exile_permission_target_is_supported(), "presentation metadata does not alter the selector");
}

#[test]
fn consistent_whole_rider_removal_or_relabeling_is_not_provenance_authentication() {
    // SOURCE ONLY / UNRUN. These are valid different programs. Admission cannot
    // infer the original source after a producer rewrites all unsigned claims.
    let baseline = artifact(&native(Some(CounterExilePermission {
        gate: CounterExileGate::PermanentSpell, allow_land: false,
    })));
    let mut removed = baseline.clone();
    change_payload(&mut removed, |payload| {
        payload.as_object_mut().unwrap().remove("exile_permission");
    });
    removed.payload.canonical_text = "Counter target spell.".into();
    removed.refresh_checksum();
    removed.validate().unwrap();
    assert_eq!(counter(&materialize_artifact(&removed).unwrap()).exile_permission, None);
    let mut relabeled = baseline.clone();
    change_payload(&mut relabeled, |payload| {
        payload["exile_permission"]["gate"] = "AnySpell".into();
    });
    relabeled.payload.canonical_text = artifact(&native(Some(CounterExilePermission {
        gate: CounterExileGate::AnySpell, allow_land: false,
    }))).payload.canonical_text;
    relabeled.refresh_checksum();
    relabeled.validate().unwrap();
    assert_eq!(counter(&materialize_artifact(&relabeled).unwrap()).exile_permission,
        Some(CounterExilePermission { gate: CounterExileGate::AnySpell, allow_land: false }));
    assert_ne!(baseline.payload, removed.payload);
    assert_ne!(baseline.payload, relabeled.payload);
}
