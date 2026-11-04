//! Actual source Harness is a named optional casting payment, not a designation.
use ironsmith::decision::{
    DecisionMaker, LegalAction, SelectFirstDecisionMaker, compute_legal_actions,
};
use ironsmith::decisions::context::{BooleanContext, SelectObjectsContext, SelectOptionsContext};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm,
};
use ironsmith::{CardDefinition, GameProgress, GameState, ObjectId, PlayerId, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
struct Choice {
    pay: bool,
    resource: ObjectId,
}
impl DecisionMaker for Choice {
    fn decide_boolean(&mut self, _: &GameState, _: &BooleanContext) -> bool {
        true
    }
    fn decide_options(&mut self, game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
        if ctx.description.starts_with("Choose optional costs") {
            return ctx
                .options
                .iter()
                .filter(|option| option.legal && self.pay)
                .map(|option| option.index)
                .collect();
        }
        SelectFirstDecisionMaker.decide_options(game, ctx)
    }
    fn decide_objects(&mut self, game: &GameState, ctx: &SelectObjectsContext) -> Vec<ObjectId> {
        if ctx
            .candidates
            .iter()
            .any(|candidate| candidate.id == self.resource)
        {
            return vec![self.resource];
        }
        SelectFirstDecisionMaker.decide_objects(game, ctx)
    }
}
fn definitions(id: &str) -> [CardDefinition; 2] {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/custom_mechanics/msem-harness-next.json"
    ))
    .unwrap();
    let row = fixture["mechanics"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["id"] == id)
        .unwrap();
    let card = &row["representative_cards"][0];
    let text = card["executable_text"].as_str().unwrap();
    let vocabulary: Vec<ironsmith_core::custom_subtypes::CustomSubtypeDefinition> =
        serde_json::from_value(card["custom_subtypes"].clone()).unwrap();
    ironsmith_compiler::with_custom_subtypes(&vocabulary, || {
        let mut builder = ironsmith_compiler::CardDefinitionBuilder::new(
            ironsmith_compiler::ids::CardId::new(),
            card["name"].as_str().unwrap(),
        );
        for identity in card["custom_keyword_identities"]
            .as_array()
            .into_iter()
            .flatten()
        {
            builder = builder.with_custom_keyword(identity.as_str().unwrap());
        }
        let (compiled, loss) = ironsmith_compiler::parse_loss::capture(|| {
            ironsmith_compiler::word_claims::with_enforcement_options(true, true, || {
                ironsmith_compiler::CompilerFacade::new().compile_definition(
                    builder,
                    text,
                    ironsmith_compiler::CompilePolicy {
                        allow_unsupported: false,
                    },
                )
            })
        });
        assert!(!loss.is_lossy(), "{}", loss.reasons_text());
        let mut compiled = compiled.unwrap();
        for keyword in card["custom_keyword_payments"]
            .as_array()
            .into_iter()
            .flatten()
        {
            assert!(
                compiled
                    .definition
                    .attribute_custom_keyword_payment(keyword.as_str().unwrap())
            );
        }
        let wire =
            ironsmith_compiled_artifact::wire_definition_from_serializable(&compiled.definition)
                .unwrap();
        let direct =
            ironsmith_runtime_catalog::artifact_materializer::materialize_definition(wire.clone())
                .unwrap();
        let artifact = CompiledCardArtifact::new(
            ironsmith_compiled_artifact::ArtifactCardIdentity {
                local_id: ironsmith_compiled_artifact::ArtifactCardId(direct.card.id.0),
                name: direct.card.name.clone(),
                face_name: None,
                other_face: None,
                linked_face_layout: None,
            },
            ironsmith_compiled_artifact::CompiledCardPayload {
                definition: wire,
                canonical_text: ironsmith_text::compiled_text_lines(&direct).join("\n"),
                ability_labels: ironsmith_text::ability_surface_texts(&direct),
            },
            "harness-source",
            text.as_bytes(),
        );
        let decoded = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
        assert_eq!(artifact, decoded);
        [
            direct,
            ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&decoded)
                .unwrap(),
        ]
    })
    .unwrap()
}
fn game() -> GameState {
    let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
    game.turn.active_player = A;
    game.turn.priority_player = Some(A);
    game.turn.phase = ironsmith::game_state::Phase::FirstMain;
    game.turn.step = None;
    game
}
fn body(game: &mut GameState, kind: &str, power: i32, owner: PlayerId) -> ObjectId {
    let definition = ironsmith_compiler_runtime::compile_to_runtime_definition(
        "Payment creature",
        &format!("Type: Creature — {kind}\nPower/Toughness: {power}/3"),
        false,
    )
    .unwrap();
    game.create_object_from_definition(&definition, owner, Zone::Battlefield)
}
fn mana(game: &mut GameState, symbol: ironsmith::mana::ManaSymbol, count: u32) {
    game.player_mut(A).unwrap().mana_pool.add(symbol, count);
}
fn announce(game: &mut GameState, spell: ObjectId, dm: &mut Choice) {
    let action = compute_legal_actions(game, A)
        .unwrap()
        .into_iter()
        .find(|action| matches!(action,LegalAction::CastSpell{spell_id,..}if *spell_id==spell))
        .expect("actual source spell castable");
    let mut state = PriorityLoopState::new(2);
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    let mut progress = apply_priority_response_with_dm(
        game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        dm,
    )
    .unwrap();
    for _ in 0..96 {
        if !state.has_pending_action() {
            ironsmith::game_loop::put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap();
            return;
        }
        let GameProgress::NeedsDecisionCtx(ctx) = progress else {
            panic!("{progress:?}")
        };
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &ctx, dm).unwrap();
    }
    panic!("casting did not finish");
}
fn flush(game: &mut GameState, dm: &mut Choice) {
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    ironsmith::game_loop::drain_pending_trigger_events_with_dm(game, &mut queue, dm).unwrap();
    ironsmith::game_loop::put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap();
}
fn relocate(game: &mut GameState, id: ObjectId, zone: Zone) -> ObjectId {
    let stable = game.object(id).unwrap().stable_id;
    let result = ironsmith::effects::execute_effect(
        game,
        &ironsmith::Effect::new(ironsmith::effects::MoveToZoneEffect::new(
            ironsmith::target::ChooseSpec::SpecificObject(id),
            zone,
            false,
        )),
        &mut ironsmith::effects::EffectContext::new_default(id, A),
    )
    .unwrap();
    for event in result.events {
        game.queue_trigger_event(Default::default(), event);
    }
    game.find_object_by_stable_id(stable).unwrap()
}
#[test]
fn plow_remembers_only_its_actual_paid_tap_creature_through_own_entry() {
    for definition in definitions("msem-692") {
        for mode in [
            "paid-ox",
            "declined",
            "paid-elf",
            "paid-ox-left",
            "paid-ox-reentered",
            "paid-type-changed",
            "no-cast",
            "copied-permanent",
        ] {
            let mut game = game();
            let kind = if mode == "paid-elf" { "Elf" } else { "Ox" };
            let mut payer = body(&mut game, kind, 2, A);
            let bystander = body(&mut game, "Ox", 8, A);
            let opponent = body(&mut game, "Ox", 8, B);
            game.tap(bystander);
            game.tap(opponent);
            mana(&mut game, ironsmith::mana::ManaSymbol::White, 1);
            mana(&mut game, ironsmith::mana::ManaSymbol::Colorless, 2);
            let source = game.create_object_from_definition(&definition, A, Zone::Hand);
            let source_stable = game.object(source).unwrap().stable_id;
            let mut dm = Choice {
                pay: mode != "declined",
                resource: payer,
            };
            if mode == "no-cast" {
                relocate(&mut game, source, Zone::Battlefield);
                flush(&mut game, &mut dm);
                assert!(game.stack.is_empty());
                continue;
            }
            announce(&mut game, source, &mut dm);
            assert_eq!(
                game.player(A).unwrap().mana_pool.total(),
                if mode == "declined" { 0 } else { 2 }
            );
            assert_eq!(game.is_tapped(payer), mode != "declined");
            assert!(
                !game.is_harnessed(payer),
                "casting payment never confers official Harness designation"
            );
            if matches!(mode, "paid-ox-left" | "paid-ox-reentered") {
                payer = relocate(&mut game, payer, Zone::Hand);
                if mode == "paid-ox-reentered" {
                    payer = relocate(&mut game, payer, Zone::Battlefield);
                    game.tap(payer);
                }
            }
            if mode == "paid-type-changed" {
                game.object_mut(payer).unwrap().subtypes = vec![ironsmith::Subtype::Elf].into();
            }
            ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
            let plow = game.find_object_by_stable_id(source_stable).unwrap();
            flush(&mut game, &mut dm);
            let expected = !matches!(mode, "declined" | "paid-elf");
            assert_eq!(game.stack.len(), usize::from(expected), "{mode}");
            if expected {
                ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
            }
            let attached = matches!(mode, "paid-ox" | "paid-type-changed" | "copied-permanent");
            assert_eq!(
                game.object(plow).unwrap().attached_to,
                attached.then_some(ironsmith::object::AttachmentTarget::Object(payer)),
                "{mode}"
            );
            if attached {
                assert!(!game.is_tapped(payer));
            }
            if mode == "paid-ox-reentered" {
                assert!(
                    game.is_tapped(payer),
                    "new incarnation is not the retained payment object"
                );
            }
            assert!(game.is_tapped(bystander));
            assert!(game.is_tapped(opponent));
            if mode == "copied-permanent" {
                let effect =
                    ironsmith::Effect::new(ironsmith::effects::CreateTokenCopyEffect::one(
                        ironsmith::target::ChooseSpec::SpecificObject(plow),
                    ));
                let outcome = ironsmith::effects::execute_effect(
                    &mut game,
                    &effect,
                    &mut ironsmith::effects::EffectContext::new_default(plow, A),
                )
                .unwrap();
                for event in outcome.events {
                    game.queue_trigger_event(Default::default(), event);
                }
                flush(&mut game, &mut dm);
                assert!(
                    game.stack.is_empty(),
                    "copiable characteristics do not copy payment evidence"
                );
            }
        }
    }
}
#[test]
fn darutu_grants_actual_optional_reducer_only_to_own_enchantment_spells_and_only_ox_payment() {
    for definition in definitions("msem-691") {
        for mode in [
            "ox",
            "elf",
            "opponent-ox",
            "tapped-ox",
            "wrong-spell-type",
            "provider-absent",
        ] {
            let mut game = game();
            if mode != "provider-absent" {
                game.create_object_from_definition(&definition, A, Zone::Battlefield);
            }
            let owner = if mode == "opponent-ox" { B } else { A };
            let payer = body(
                &mut game,
                if mode == "elf" { "Elf" } else { "Ox" },
                4,
                owner,
            );
            if mode == "tapped-ox" {
                game.tap(payer);
            }
            let kind = if mode == "wrong-spell-type" {
                "Sorcery"
            } else {
                "Enchantment"
            };
            let text = if kind == "Sorcery" {
                "Mana cost: {4}{G}\nType: Sorcery\nYou gain 1 life."
            } else {
                "Mana cost: {4}{G}\nType: Enchantment"
            };
            let recipient = ironsmith_compiler_runtime::compile_to_runtime_definition(
                "Granted payment recipient",
                text,
                false,
            )
            .unwrap();
            let spell = game.create_object_from_definition(&recipient, A, Zone::Hand);
            mana(&mut game, ironsmith::mana::ManaSymbol::Green, 1);
            let offered = compute_legal_actions(&game, A).unwrap().into_iter().any(
                |action| matches!(action,LegalAction::CastSpell{spell_id,..}if spell_id==spell),
            );
            assert_eq!(offered, mode == "ox", "{mode}");
            if offered {
                let mut dm = Choice {
                    pay: true,
                    resource: payer,
                };
                announce(&mut game, spell, &mut dm);
                assert!(game.is_tapped(payer));
                assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
                ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
                assert!(game.battlefield.iter().any(|id| {
                    game.object(*id)
                        .is_some_and(|object| object.name.as_ref() == "Granted payment recipient")
                }));
            }
        }
    }
}
#[test]
fn actual_confusion_payment_controls_draw_without_a_harness_designation() {
    for definition in definitions("msem-690") {
        for paid in [false, true] {
            let mut game = game();
            let payer = body(&mut game, "Ox", 2, A);
            let draw = ironsmith_compiler_runtime::compile_to_runtime_definition(
                "Card to draw",
                "Type: Land",
                false,
            )
            .unwrap();
            let draw = game.create_object_from_definition(&draw, A, Zone::Library);
            let stable = game.object(draw).unwrap().stable_id;
            mana(&mut game, ironsmith::mana::ManaSymbol::Blue, 1);
            mana(&mut game, ironsmith::mana::ManaSymbol::Colorless, 2);
            let source = game.create_object_from_definition(&definition, A, Zone::Hand);
            let mut dm = Choice {
                pay: paid,
                resource: payer,
            };
            announce(&mut game, source, &mut dm);
            assert_eq!(game.is_tapped(payer), paid);
            assert!(!game.is_harnessed(payer));
            ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
            assert_eq!(
                game.object(game.find_object_by_stable_id(stable).unwrap())
                    .unwrap()
                    .zone,
                if paid { Zone::Hand } else { Zone::Library }
            );
        }
    }
}
#[test]
fn provider_ordinary_battlefield_static_does_not_grant_its_own_cast_from_hand() {
    for definition in definitions("msem-691") {
        let mut game = game();
        body(&mut game, "Ox", 2, A);
        let source = game.create_object_from_definition(&definition, A, Zone::Hand);
        mana(&mut game, ironsmith::mana::ManaSymbol::Green, 1);
        mana(&mut game, ironsmith::mana::ManaSymbol::White, 1);
        assert!(!compute_legal_actions(&game, A).unwrap().iter().any(
            |action| matches!(action,LegalAction::CastSpell{spell_id,..}if *spell_id==source)
        ));
    }
}
#[test]
fn action_qualified_tags_exclude_other_affected_objects_in_the_same_program() {
    let mut game = game();
    let tapped = body(&mut game, "Ox", 2, A);
    let exiled = body(&mut game, "Elf", 3, A);
    let sequence = ironsmith::effects::SequenceEffect::new(vec![
        ironsmith::Effect::tap(ironsmith::target::ChooseSpec::SpecificObject(tapped)),
        ironsmith::Effect::new(ironsmith::effects::MoveToZoneEffect::new(
            ironsmith::target::ChooseSpec::SpecificObject(exiled),
            Zone::Exile,
            false,
        )),
    ]);
    let mut tagged = ironsmith::effects::TaggedEffect::new(
        "completed payment",
        ironsmith::Effect::new(sequence),
    );
    tagged.action = Some(ironsmith_core::PriorEffectAction::Tapped);
    tagged.outcome_only = true;
    let mut context = ironsmith::effects::EffectContext::new_default(tapped, A);
    ironsmith::effects::execute_effect(&mut game, &ironsmith::Effect::new(tagged), &mut context)
        .unwrap();
    let receipt = context.get_tagged_all("completed payment").unwrap();
    assert_eq!(receipt.len(), 1);
    assert_eq!(receipt[0].object_id, tapped);
}

#[test]
fn duplicate_provider_recipes_keep_independent_prices_and_require_distinct_contributors() {
    for definition in definitions("msem-691") {
        for payers in [1, 2] {
            let mut game = game();
            let first = body(&mut game, "Ox", 3, A);
            let other = (payers == 2).then(|| body(&mut game, "Ox", 3, A));
            game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let second = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            // A changed-name copy may legally coexist with the legendary source.
            game.object_mut(second).unwrap().name = "Second payment provider".into();
            let recipient = ironsmith_compiler_runtime::compile_to_runtime_definition(
                "Two-recipe recipient",
                "Mana cost: {6}{G}\nType: Enchantment",
                false,
            )
            .unwrap();
            let spell = game.create_object_from_definition(&recipient, A, Zone::Hand);
            mana(&mut game, ironsmith::mana::ManaSymbol::Green, 1);
            let offered = compute_legal_actions(&game, A).unwrap().iter().any(
                |action| matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == spell)
            );
            assert_eq!(
                offered,
                payers == 2,
                "one contributor cannot pay both instances"
            );
            for _ in 0..5 {
                assert_eq!(compute_legal_actions(&game, A).unwrap().iter().any(
                    |action| matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == spell)
                ), offered);
            }
            // Preview cannot publish an acquired recipe on the real hand object.
            assert!(game.object(spell).unwrap().optional_costs.is_empty());
            if offered {
                let mut dm = Choice {
                    pay: true,
                    resource: first,
                };
                announce(&mut game, spell, &mut dm);
                assert!(game.is_tapped(first));
                assert!(game.is_tapped(other.unwrap()));
                let spell = game.object(spell).unwrap();
                assert_eq!(spell.optional_costs.len(), 2);
                assert_ne!(
                    spell.optional_costs[0].reference,
                    spell.optional_costs[1].reference
                );
                assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
                let receipt = spell
                    .cast_tagged_objects
                    .get(&ironsmith_core::tag::additional_cost_tap_receipt(
                        "power price",
                    ))
                    .unwrap();
                assert_eq!(receipt.len(), 2, "each actual contributor retained once");
                assert_ne!(receipt[0].object_id, receipt[1].object_id);
            }
        }
    }
}

#[test]
fn retained_payment_snapshots_must_recursively_be_public_claims() {
    let mut game = game();
    let payer = body(&mut game, "Ox", 3, A);
    let nested =
        ironsmith::snapshot::ObjectSnapshot::from_object(game.object(payer).unwrap(), &game);
    // The engine's stripping helper is crate-private; the public placeholder
    // is the public claim form of the same object identity.
    let public = |snapshot: &ironsmith::snapshot::ObjectSnapshot| {
        ironsmith::snapshot::ObjectSnapshot::public_placeholder(
            snapshot.object_id,
            snapshot.stable_id,
            snapshot.owner,
            snapshot.controller,
            snapshot.zone,
        )
    };
    assert!(!nested.is_public_claim_form());
    let mut outer = public(&nested);
    assert!(outer.is_public_claim_form());
    outer
        .paid_optional_cost_objects
        .insert("receipt".into(), vec![nested]);
    assert!(
        !outer.is_public_claim_form(),
        "public outer cannot hide a private nested receipt"
    );
    for snapshot in outer.paid_optional_cost_objects.values_mut().flatten() {
        *snapshot = public(snapshot);
    }
    assert!(outer.is_public_claim_form());
}
