//! UNVALIDATED source proposals; authored scenarios are intentionally unrun.
use ironsmith::ability::AbilityKind;
use ironsmith::cards::CardDefinition;
use ironsmith::costs::PaymentReason;
use ironsmith::decision::{LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm,
};
use ironsmith::game_state::{Phase, StackEntry};
use ironsmith::mana::{ManaCost, ManaSymbol};
use ironsmith::triggers::TriggerQueue;
use ironsmith::{GameProgress, GameState, ObjectId, PlayerId, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;

fn definitions(name: &str) -> [CardDefinition; 2] {
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../fixtures/mana_payment_predicate_restrictions.json.fixture"
    ))
    .unwrap();
    let row = rows.iter().find(|r| r["name"] == name).unwrap();
    let text = row["text"].as_str().unwrap();
    let direct = compile_to_runtime_definition(name, text, false).unwrap();
    let (artifact, _) = compile_to_artifact(name, text, false).unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(restored, artifact);
    [direct, materialize_artifact(&restored).unwrap()]
}
fn game() -> GameState {
    let mut g = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    g.turn.active_player = PlayerId::from_index(0);
    g.turn.priority_player = Some(PlayerId::from_index(0));
    g.turn.phase = Phase::FirstMain;
    g.turn.step = None;
    g
}
fn object(g: &mut GameState, text: &str, zone: Zone, owner: PlayerId) -> ObjectId {
    let d = compile_to_runtime_definition("Payment target", text, false).unwrap();
    let id = g.create_object_from_definition(&d, owner, zone);
    if zone == Zone::Stack {
        g.stack.push(StackEntry::new(id, owner));
    }
    id
}
fn activate(g: &mut GameState, source: ObjectId) {
    let index = g.current_abilities(source).unwrap().iter().position(|a| matches!(&a.kind, AbilityKind::Activated(a) if !a.mana_usage_restrictions.is_empty())).unwrap();
    g.remove_summoning_sickness(source);
    let action = LegalAction::ActivateManaAbility {
        source,
        ability_index: index,
    };
    assert!(
        compute_legal_actions(g, PlayerId::from_index(0))
            .unwrap()
            .contains(&action)
    );
    let mut q = TriggerQueue::new();
    let mut s = PriorityLoopState::new(2);
    let mut dm = SelectFirstDecisionMaker;
    let mut progress = apply_priority_response_with_dm(
        g,
        &mut q,
        &mut s,
        &PriorityResponse::PriorityAction(action),
        &mut dm,
    )
    .unwrap();
    for _ in 0..32 {
        if s.pending_mana_ability.is_none() && s.pending_activation.is_none() {
            break;
        }
        let GameProgress::NeedsDecisionCtx(ctx) = progress else {
            panic!("{progress:?}");
        };
        progress = apply_decision_context_with_dm(g, &mut q, &mut s, &ctx, &mut dm).unwrap();
    }
    assert!(s.pending_mana_ability.is_none() && s.pending_activation.is_none());
    assert!(
        !g.player(PlayerId::from_index(0))
            .unwrap()
            .restricted_mana
            .is_empty()
    );
}
fn can(g: &GameState, target: ObjectId, reason: PaymentReason, cost: Vec<ManaSymbol>) -> bool {
    g.can_pay_mana_cost_with_reason(
        PlayerId::from_index(0),
        Some(target),
        &ManaCost::from_symbols(cost),
        0,
        reason,
    )
}
#[test]
fn actual_colorless_credit_respects_each_transaction_arm_after_source_departure() {
    let alice = PlayerId::from_index(0);
    for definition in definitions("Cultivator Drone") {
        let mut g = game();
        let source = g.create_object_from_definition(&definition, alice, Zone::Battlefield);
        activate(&mut g, source);
        assert!(
            g.player(alice)
                .unwrap()
                .restricted_mana
                .iter()
                .all(|u| u.source_controller == Some(alice))
        );
        g.move_object_by_effect(source, Zone::Graveyard).unwrap();
        let colorless = object(&mut g, "Mana cost: {2}\nType: Artifact", Zone::Stack, alice);
        let red = object(
            &mut g,
            "Mana cost: {1}{R}\nType: Instant\nThis spell deals 1 damage to any target.",
            Zone::Stack,
            alice,
        );
        let permanent = object(
            &mut g,
            "Mana cost: {2}\nType: Artifact\n{1}: Draw a card.",
            Zone::Battlefield,
            alice,
        );
        assert!(can(
            &g,
            colorless,
            PaymentReason::CastSpell,
            vec![ManaSymbol::Generic(1)]
        ));
        assert!(!can(
            &g,
            red,
            PaymentReason::CastSpell,
            vec![ManaSymbol::Generic(1)]
        ));
        assert!(can(
            &g,
            permanent,
            PaymentReason::ActivateAbility,
            vec![ManaSymbol::Generic(1)]
        ));
        assert!(!can(
            &g,
            permanent,
            PaymentReason::Effect,
            vec![ManaSymbol::Generic(1)]
        ));
        assert!(can(
            &g,
            red,
            PaymentReason::Effect,
            vec![ManaSymbol::Colorless]
        ));
        assert!(g.try_pay_mana_cost_with_reason(
            alice,
            Some(red),
            &ManaCost::from_symbols(vec![ManaSymbol::Colorless]),
            0,
            PaymentReason::Effect
        ).expect("checked fixture mana payment"));
        assert_eq!(g.player(alice).unwrap().mana_pool.total(), 0);
    }
}
#[test]
fn face_up_payment_is_distinct_from_cast_and_ordinary_activation() {
    let alice = PlayerId::from_index(0);
    for name in ["Overgrown Zealot", "Tin Street Gossip"] {
        for d in definitions(name) {
            let mut g = game();
            let source = g.create_object_from_definition(&d, alice, Zone::Battlefield);
            activate(&mut g, source);
            let facedown = object(
                &mut g,
                "Mana cost: {1}\nType: Creature — Human\nPower/Toughness: 1/1",
                Zone::Battlefield,
                alice,
            );
            g.set_face_down(facedown);
            assert!(can(
                &g,
                facedown,
                PaymentReason::TurnFaceUp,
                vec![ManaSymbol::Generic(1)]
            ));
            assert!(!can(
                &g,
                facedown,
                PaymentReason::ActivateAbility,
                vec![ManaSymbol::Generic(1)]
            ));
            let faceup = object(&mut g, "Type: Artifact", Zone::Battlefield, alice);
            assert!(!can(
                &g,
                faceup,
                PaymentReason::TurnFaceUp,
                vec![ManaSymbol::Generic(1)]
            ));
            let spell = object(
                &mut g,
                "Mana cost: {1}\nType: Creature — Human\nPower/Toughness: 1/1",
                Zone::Stack,
                alice,
            );
            g.set_face_down(spell);
            assert_eq!(
                can(
                    &g,
                    spell,
                    PaymentReason::CastSpell,
                    vec![ManaSymbol::Generic(1)]
                ),
                name == "Tin Street Gossip"
            );
            assert!(!can(
                &g,
                spell,
                PaymentReason::Effect,
                vec![ManaSymbol::Generic(1)]
            ));
        }
    }
}
#[test]
fn actual_aura_grant_preserves_outlaw_union_without_allowing_any_creature() {
    use ironsmith::object::AttachmentTarget;
    let alice = PlayerId::from_index(0);
    for d in definitions("Discreet Retreat") {
        let mut g = game();
        let aura = g.create_object_from_definition(&d, alice, Zone::Battlefield);
        let land = object(&mut g, "Type: Land", Zone::Battlefield, alice);
        assert!(g.attach_object_to_target(aura, AttachmentTarget::Object(land)));
        activate(&mut g, land);
        g.move_object_by_effect(aura, Zone::Graveyard).unwrap();
        g.move_object_by_effect(land, Zone::Graveyard).unwrap();
        for subtype in [
            "Assassin",
            "Mercenary",
            "Pirate",
            "Rogue",
            "Warlock",
            "Human",
        ] {
            let text =
                format!("Mana cost: {{1}}\nType: Creature — {subtype}\nPower/Toughness: 1/1");
            let spell = object(&mut g, &text, Zone::Stack, alice);
            let permanent = object(&mut g, &text, Zone::Battlefield, alice);
            assert_eq!(
                can(
                    &g,
                    spell,
                    PaymentReason::CastSpell,
                    vec![ManaSymbol::Generic(1)]
                ),
                subtype != "Human"
            );
            assert_eq!(
                can(
                    &g,
                    permanent,
                    PaymentReason::ActivateAbility,
                    vec![ManaSymbol::Generic(1)]
                ),
                subtype != "Human"
            );
            assert!(!can(
                &g,
                permanent,
                PaymentReason::Effect,
                vec![ManaSymbol::Generic(1)]
            ));
        }
    }
}


// Fresh reconstruction: these native scenarios are authored, never executed.
fn announce_action(g: &mut GameState, action: LegalAction) {
    announce_with_dm(g, action, &mut SelectFirstDecisionMaker);
}
fn announce_with_dm(g: &mut GameState, action: LegalAction, dm: &mut impl ironsmith::decision::DecisionMaker) {
    let mut state = PriorityLoopState::new(2);
    let mut queue = TriggerQueue::new();
    let mut progress = apply_priority_response_with_dm(g, &mut queue, &mut state,
        &PriorityResponse::PriorityAction(action), dm).unwrap();
    for _ in 0..40 {
        if state.pending_cast.is_none() && state.pending_activation.is_none() && state.pending_mana_ability.is_none() { return; }
        let GameProgress::NeedsDecisionCtx(context) = progress else { panic!("{progress:?}"); };
        progress = apply_decision_context_with_dm(g, &mut queue, &mut state, &context, dm).unwrap();
    }
    panic!("announcement did not finish");
}
fn sliced_actions(g: &GameState) -> Vec<LegalAction> {
    let mut analysis = ironsmith::decision::ManaAnalysisSession::default();
    for _ in 0..256 {
        let (actions, complete) = analysis.run_for_game(g, 16, || compute_legal_actions(g, PlayerId(0)));
        let actions = actions.unwrap();
        if complete { return actions; }
    }
    panic!("bounded fixture analysis did not finish");
}
fn linked_card(g: &mut GameState, front_text: &str, back_text: &str, zone: Zone) -> ObjectId {
    let mut front = compile_to_runtime_definition("Physical creature", front_text, false).unwrap();
    let mut back = compile_to_runtime_definition("Chosen spell face", back_text, false).unwrap();
    front.card.other_face = Some(back.card.id);
    front.card.other_face_name = Some(back.card.name.to_string());
    front.card.linked_face_layout = ironsmith::card::LinkedFaceLayout::TransformLike;
    back.card.other_face = Some(front.card.id);
    back.card.other_face_name = Some(front.card.name.to_string());
    back.card.linked_face_layout = ironsmith::card::LinkedFaceLayout::TransformLike;
    g.register_linked_face_definition(&front);
    g.register_linked_face_definition(&back);
    g.create_object_from_definition(&front, PlayerId(0), zone)
}
#[test]
fn qarsi_uses_exact_origin_and_announced_turn_up_method() {
    use ironsmith::special_actions::{SpecialAction, TurnFaceUpMethod, can_perform_check, perform};
    let alice = PlayerId(0);
    for definition in definitions("Qarsi Deceiver") {
        let rendered = ironsmith_text::compiled_text_lines(&definition).join("\n");
        assert!(rendered.contains("manifested creature") && rendered.contains("morph cost"), "{rendered}");
        for (cloak, method, expected) in [
            (false, TurnFaceUpMethod::PrintedManaCost, true),
            (true, TurnFaceUpMethod::PrintedManaCost, false),
            (false, TurnFaceUpMethod::TurnFaceUpAbility, true),
            (true, TurnFaceUpMethod::TurnFaceUpAbility, true),
            (false, TurnFaceUpMethod::MegamorphAbility, true),
            (true, TurnFaceUpMethod::DisguiseAbility, false),
        ] {
            let mut g = game();
            let source = g.create_object_from_definition(&definition, alice, Zone::Battlefield);
            activate(&mut g, source);
            object(&mut g, "Mana cost: {1}\nType: Creature — Human\nPower/Toughness: 2/2\nMorph {1}\nMegamorph {1}\nDisguise {1}", Zone::Library, alice);
            let effect = ironsmith::effect::Effect::new(ironsmith::effects::ManifestTopCardOfLibraryEffect {
                player: ironsmith_core::PlayerFilter::You, cloak,
            });
            let mut context = ironsmith::effects::EffectContext::new_default(source, alice);
            let outcome = ironsmith::effects::execute_effect(&mut g, &effect, &mut context).unwrap();
            let target = outcome.value.objects().unwrap()[0];
            assert_eq!(g.is_manifested(target), !cloak);
            assert_eq!(g.is_cloaked(target), cloak);
            g.move_object_by_effect(source, Zone::Graveyard).unwrap();
            let action = SpecialAction::TurnFaceUp { permanent_id: target, method };
            assert_eq!(can_perform_check(&action, &g, alice).is_ok(), expected);
            let before = g.player(alice).unwrap().mana_pool.clone();
            assert_eq!(perform(action, &mut g, alice, &mut SelectFirstDecisionMaker).is_ok(), expected);
            if expected {
                assert!(!g.is_face_down(target) && !g.is_manifested(target) && !g.is_cloaked(target));
            } else {
                assert!(g.is_face_down(target));
                assert_eq!(g.player(alice).unwrap().mana_pool, before);
            }
        }
    }
}
#[test]
fn qarsi_face_down_cast_differs_from_equal_price_normal_creature_cast() {
    use ironsmith::alternative_cast::CastingMethod;
    let alice = PlayerId(0);
    for definition in definitions("Qarsi Deceiver") {
        let mut g = game();
        for _ in 0..3 {
            let source = g.create_object_from_definition(&definition, alice, Zone::Battlefield);
            activate(&mut g, source);
        }
        let card = object(&mut g, "Mana cost: {3}\nType: Creature — Human\nPower/Toughness: 3/3\nMorph {1}", Zone::Hand, alice);
        let normal = LegalAction::CastSpell { spell_id: card, from_zone: Zone::Hand, casting_method: CastingMethod::Normal };
        let down = LegalAction::CastSpell { spell_id: card, from_zone: Zone::Hand, casting_method: CastingMethod::FaceDown };
        let direct = compute_legal_actions(&g, alice).unwrap();
        assert!(!direct.contains(&normal) && direct.contains(&down));
        assert_eq!(sliced_actions(&g), direct);
        assert_eq!(g.object(card).unwrap().zone, Zone::Hand);
        assert!(!g.is_face_down(card));
        announce_action(&mut g, down);
        assert_eq!(g.player(alice).unwrap().mana_pool.total(), 0);
        assert!(g.is_face_down(g.stack.last().unwrap().object_id));
    }
}
#[test]
fn karfell_pays_foretell_action_not_a_foretell_creature_cast() {
    use ironsmith::special_actions::{SpecialAction, can_perform_check, perform};
    let alice = PlayerId(0);
    for definition in definitions("Karfell Harbinger") {
        let mut g = game();
        let first = g.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let second = g.create_object_from_definition(&definition, alice, Zone::Battlefield);
        activate(&mut g, first); activate(&mut g, second);
        let card = object(&mut g, "Mana cost: {1}{U}\nType: Creature — Human\nPower/Toughness: 2/2\nForetell {U}", Zone::Hand, alice);
        assert!(!can(&g, card, PaymentReason::Other, vec![ManaSymbol::Generic(2)]));
        let action = SpecialAction::Foretell { card_id: card };
        assert!(can_perform_check(&action, &g, alice).is_ok());
        perform(action, &mut g, alice, &mut SelectFirstDecisionMaker).unwrap();
        assert_eq!(g.player(alice).unwrap().mana_pool.total(), 0);
        assert!(g.is_foretold(*g.exile.last().unwrap()));
        g.untap(first); activate(&mut g, first);
        g.move_object_by_effect(first, Zone::Graveyard).unwrap();
        let spell = object(&mut g, "Mana cost: {U}\nType: Instant\nDraw a card.", Zone::Stack, alice);
        let creature = object(&mut g, "Mana cost: {U}\nType: Creature — Human\nPower/Toughness: 1/1\nForetell {U}", Zone::Stack, alice);
        assert!(can(&g, spell, PaymentReason::CastSpell, vec![ManaSymbol::Blue]));
        assert!(!can(&g, creature, PaymentReason::CastSpell, vec![ManaSymbol::Blue]));
        assert!(!can(&g, spell, PaymentReason::ActivateAbility, vec![ManaSymbol::Blue]));
    }
}
#[test]
fn karfell_pays_an_adventure_face_not_the_physical_creature_front() {
    use ironsmith::alternative_cast::CastingMethod;
    let alice = PlayerId(0);
    for definition in definitions("Karfell Harbinger") {
        let mut g = game();
        let source = g.create_object_from_definition(&definition, alice, Zone::Battlefield);
        activate(&mut g, source);
        let card = linked_card(&mut g, "Mana cost: {U}\nType: Creature — Human\nPower/Toughness: 1/1",
            "Mana cost: {U}\nType: Instant — Adventure\nYou gain 1 life.", Zone::Hand);
        let normal = LegalAction::CastSpell { spell_id: card, from_zone: Zone::Hand, casting_method: CastingMethod::Normal };
        let adventure = LegalAction::CastSpell { spell_id: card, from_zone: Zone::Hand, casting_method: CastingMethod::SplitOtherHalf };
        let actions = compute_legal_actions(&g, alice).unwrap();
        assert!(!actions.contains(&normal) && actions.contains(&adventure));
        assert_eq!(sliced_actions(&g), actions);
        announce_action(&mut g, adventure);
        assert_eq!(g.player(alice).unwrap().mana_pool.total(), 0);
        assert!(g.object(g.stack.last().unwrap().object_id).unwrap().has_card_type(ironsmith::types::CardType::Instant));
    }
}
#[test]
fn observer_pays_disturb_price_not_normal_cast_of_a_card_with_disturb() {
    use ironsmith::alternative_cast::CastingMethod;
    let alice = PlayerId(0);
    for definition in definitions("Unblinking Observer") {
        let mut g = game();
        let source = g.create_object_from_definition(&definition, alice, Zone::Battlefield);
        activate(&mut g, source);
        g.move_object_by_effect(source, Zone::Graveyard).unwrap();
        let front = "Mana cost: {U}\nType: Creature — Human\nPower/Toughness: 1/1\nDisturb {U}";
        let back = "Type: Creature — Spirit\nPower/Toughness: 2/2\nFlying";
        let hand = linked_card(&mut g, front, back, Zone::Hand);
        let grave = linked_card(&mut g, front, back, Zone::Graveyard);
        let instant = object(&mut g, "Mana cost: {U}\nType: Instant\nYou gain 1 life.", Zone::Hand, alice);
        let actions = compute_legal_actions(&g, alice).unwrap();
        assert!(!actions.iter().any(|a| matches!(a, LegalAction::CastSpell { spell_id, .. } if *spell_id == hand)));
        assert!(actions.iter().any(|a| matches!(a, LegalAction::CastSpell { spell_id, .. } if *spell_id == instant)));
        let disturb = actions.iter().find(|a| matches!(a, LegalAction::CastSpell { spell_id, casting_method: CastingMethod::Alternative(_), .. } if *spell_id == grave)).unwrap().clone();
        assert_eq!(sliced_actions(&g), actions);
        announce_action(&mut g, disturb);
        assert_eq!(g.player(alice).unwrap().mana_pool.total(), 0);
        let spell = g.object(g.stack.last().unwrap().object_id).unwrap();
        assert!(matches!(spell.cast_alternative_method.as_deref(), Some(ironsmith::alternative_cast::AlternativeCastingMethod::Disturb { .. })));
        assert_eq!(spell.name, "Chosen spell face");
    }
}
#[test]
fn equipment_mana_pays_exact_equip_but_not_other_attachment_abilities() {
    let alice = PlayerId(0);
    for name in ["Freya Crescent", "Ronin, Shadow Stalker"] {
        for definition in definitions(name) {
            let mut g = game();
            let source = g.create_object_from_definition(&definition, alice, Zone::Battlefield);
            activate(&mut g, source);
            if name == "Ronin, Shadow Stalker" {
                assert_eq!(g.player(alice).unwrap().life, 18);
                assert!(!compute_legal_actions(&g, alice).unwrap().iter().any(|a|
                    matches!(a, LegalAction::ActivateManaAbility { source: id, .. } if *id == source)));
            }
            g.move_object_by_effect(source, Zone::Graveyard).unwrap();
            let host = object(&mut g, "Mana cost: {1}\nType: Creature — Human\nPower/Toughness: 2/2", Zone::Battlefield, alice);
            let equip = object(&mut g, "Mana cost: {1}\nType: Artifact — Equipment\nEquip {1}\n{1}: Attach this Equipment to target creature you control.", Zone::Battlefield, alice);
            let abilities = g.current_abilities(equip).unwrap();
            let typed = abilities.iter().position(|a| matches!(&a.kind, AbilityKind::Activated(a) if a.keyword == Some(ironsmith_core::ActivatedAbilityKeyword::Equip))).unwrap();
            let ordinary = abilities.iter().position(|a| matches!(&a.kind, AbilityKind::Activated(a) if a.keyword.is_none())).unwrap();
            let action = LegalAction::ActivateAbility { source: equip, ability_index: typed };
            let actions = compute_legal_actions(&g, alice).unwrap();
            assert!(actions.contains(&action));
            assert!(!actions.contains(&LegalAction::ActivateAbility { source: equip, ability_index: ordinary }));
            announce_action(&mut g, action);
            ironsmith::game_loop::resolve_stack_entry_with(&mut g, &mut SelectFirstDecisionMaker).unwrap();
            assert_eq!(g.object(equip).unwrap().attached_to, Some(ironsmith::object::AttachmentTarget::Object(host)));
        }
    }
}
#[test]
fn equipment_mana_casts_selected_equipment_back_not_creature_front() {
    use ironsmith::alternative_cast::CastingMethod;
    let alice = PlayerId(0);
    for name in ["Freya Crescent", "Ronin, Shadow Stalker"] {
        for definition in definitions(name) {
            let mut g = game();
            let source = g.create_object_from_definition(&definition, alice, Zone::Battlefield);
            activate(&mut g, source);
            let card = linked_card(&mut g, "Mana cost: {1}\nType: Creature — Human\nPower/Toughness: 1/1",
                "Mana cost: {1}\nType: Artifact — Equipment\nEquip {1}", Zone::Hand);
            let actions = compute_legal_actions(&g, alice).unwrap();
            assert!(!actions.contains(&LegalAction::CastSpell { spell_id: card, from_zone: Zone::Hand, casting_method: CastingMethod::Normal }));
            let action = LegalAction::CastSpell { spell_id: card, from_zone: Zone::Hand, casting_method: CastingMethod::SplitOtherHalf };
            assert!(actions.contains(&action));
            assert_eq!(sliced_actions(&g), actions);
            let before = g.player(alice).unwrap().mana_pool.total();
            announce_action(&mut g, action);
            assert_eq!(g.player(alice).unwrap().mana_pool.total(), before - 1);
        }
    }
}
#[test]
fn freya_flying_exists_only_during_her_controllers_turn() {
    for definition in definitions("Freya Crescent") {
        let mut g = game();
        let source = g.create_object_from_definition(&definition, PlayerId(0), Zone::Battlefield);
        assert!(g.current_has_static_ability_id(source, ironsmith::static_abilities::StaticAbilityId::Flying));
        g.next_turn();
        assert_eq!(g.turn.active_player, PlayerId(1));
        assert!(!g.current_has_static_ability_id(source, ironsmith::static_abilities::StaticAbilityId::Flying));
        g.next_turn();
        assert!(g.current_has_static_ability_id(source, ironsmith::static_abilities::StaticAbilityId::Flying));
    }
}
#[test]
fn quinjet_restricted_and_unrestricted_abilities_remain_distinct() {
    let alice = PlayerId(0);
    for definition in definitions("Quinjet Technician") {
        let mut g = game();
        let source = g.create_object_from_definition(&definition, alice, Zone::Battlefield);
        activate(&mut g, source);
        assert_eq!(g.player(alice).unwrap().mana_pool.red, 2);
        g.move_object_by_effect(source, Zone::Graveyard).unwrap();
        let target = object(&mut g, "Mana cost: {1}\nType: Artifact\nPower-up — {2}: You gain 1 life.\n{2}: You gain 1 life.", Zone::Battlefield, alice);
        let abilities = g.current_abilities(target).unwrap();
        let typed = abilities.iter().position(|a| matches!(&a.kind, AbilityKind::Activated(a) if a.keyword == Some(ironsmith_core::ActivatedAbilityKeyword::PowerUp))).unwrap();
        let ordinary = abilities.iter().position(|a| matches!(&a.kind, AbilityKind::Activated(a) if a.keyword.is_none())).unwrap();
        let action = LegalAction::ActivateAbility { source: target, ability_index: typed };
        let actions = compute_legal_actions(&g, alice).unwrap();
        assert!(actions.contains(&action));
        assert!(!actions.contains(&LegalAction::ActivateAbility { source: target, ability_index: ordinary }));
        announce_action(&mut g, action);
        ironsmith::game_loop::resolve_stack_entry_with(&mut g, &mut SelectFirstDecisionMaker).unwrap();
        assert_eq!(g.player(alice).unwrap().life, 21);
        assert!(!compute_legal_actions(&g, alice).unwrap().contains(&LegalAction::ActivateAbility { source: target, ability_index: typed }));

        let mut g = game();
        let source = g.create_object_from_definition(&definition, alice, Zone::Battlefield);
        g.remove_summoning_sickness(source);
        let index = g.current_abilities(source).unwrap().iter().position(|a|
            matches!(&a.kind, AbilityKind::Activated(a) if a.mana_usage_restrictions.is_empty())).unwrap();
        announce_action(&mut g, LegalAction::ActivateManaAbility { source, ability_index: index });
        assert_eq!(g.player(alice).unwrap().mana_pool.red, 1);
        let creature = object(&mut g, "Mana cost: {R}\nType: Creature — Human\nPower/Toughness: 1/1", Zone::Stack, alice);
        assert!(g.try_pay_mana_cost_with_reason(alice, Some(creature), &ManaCost::from_symbols(vec![ManaSymbol::Red]), 0, PaymentReason::CastSpell).unwrap());
    }
}
struct TargetCreature(ObjectId);
impl ironsmith::decision::DecisionMaker for TargetCreature {
    fn answers_player_choices(&self) -> bool { false }
    fn decide_targets(&mut self, _: &GameState, ctx: &ironsmith::decisions::context::TargetsContext) -> Vec<ironsmith::Target> {
        let target = ironsmith::Target::Object(self.0);
        assert!(ctx.requirements.iter().any(|r| r.legal_targets.contains(&target)));
        vec![target]
    }
}
#[test]
fn ronin_sacrifices_attached_equipment_at_sorcery_speed_for_real_minus_four_effect() {
    let alice = PlayerId(0);
    for definition in definitions("Ronin, Shadow Stalker") {
        let mut g = game();
        let source = g.create_object_from_definition(&definition, alice, Zone::Battlefield);
        g.remove_summoning_sickness(source);
        let victim = object(&mut g, "Mana cost: {1}\nType: Creature — Human\nPower/Toughness: 6/6", Zone::Battlefield, PlayerId(1));
        let equip = object(&mut g, "Mana cost: {1}\nType: Artifact — Equipment\nEquip {1}", Zone::Battlefield, alice);
        let stable = g.object(equip).unwrap().stable_id;
        let index = g.current_abilities(source).unwrap().iter().position(|a|
            matches!(&a.kind, AbilityKind::Activated(a) if a.mana_usage_restrictions.is_empty())).unwrap();
        let action = LegalAction::ActivateAbility { source, ability_index: index };
        assert!(!compute_legal_actions(&g, alice).unwrap().contains(&action));
        assert!(g.attach_object_to_target(equip, ironsmith::object::AttachmentTarget::Object(source)));
        g.turn.phase = Phase::Combat;
        assert!(!compute_legal_actions(&g, alice).unwrap().contains(&action));
        g.turn.phase = Phase::FirstMain;
        assert!(compute_legal_actions(&g, alice).unwrap().contains(&action));
        announce_with_dm(&mut g, action, &mut TargetCreature(victim));
        assert!(g.is_tapped(source));
        assert_eq!(g.object(g.find_object_by_stable_id(stable).unwrap()).unwrap().zone, Zone::Graveyard);
        ironsmith::game_loop::resolve_stack_entry_with(&mut g, &mut SelectFirstDecisionMaker).unwrap();
        assert_eq!(g.current_power(victim), Some(2));
        assert_eq!(g.current_toughness(victim), Some(2));
    }
}
#[test]
fn origin_does_not_survive_a_new_battlefield_incarnation() {
    let mut g = game();
    let id = object(&mut g, "Mana cost: {1}\nType: Creature — Human\nPower/Toughness: 1/1", Zone::Battlefield, PlayerId(0));
    g.set_face_down(id); g.set_cloaked(id);
    let grave = g.move_object_by_effect(id, Zone::Graveyard).unwrap();
    let returned = g.move_object_by_effect(grave, Zone::Battlefield).unwrap();
    g.set_face_down(returned);
    assert!(!g.is_cloaked(id) && !g.is_cloaked(returned) && !g.is_manifested(returned));
}
#[test]
fn prototype_uses_colored_selected_face_for_cultivator_restriction() {
    use ironsmith::alternative_cast::CastingMethod;
    let alice = PlayerId(0);
    for definition in definitions("Cultivator Drone") {
        let mut g = game();
        let source = g.create_object_from_definition(&definition, alice, Zone::Battlefield);
        activate(&mut g, source);
        g.player_mut(alice).unwrap().mana_pool.add(ManaSymbol::Blue, 1);
        let card = object(&mut g, "Mana cost: {2}\nType: Artifact Creature — Construct\nPower/Toughness: 5/5\nPrototype {1}{U} — 1/1", Zone::Hand, alice);
        let actions = compute_legal_actions(&g, alice).unwrap();
        assert!(actions.iter().any(|a| matches!(a, LegalAction::CastSpell { spell_id, casting_method: CastingMethod::Normal, .. } if *spell_id == card)));
        assert!(!actions.iter().any(|a| matches!(a, LegalAction::CastSpell { spell_id, casting_method: CastingMethod::Alternative(_), .. } if *spell_id == card)));
        assert_eq!(sliced_actions(&g), actions);
    }
}
#[test]
fn harmonize_explicit_request_keeps_the_selected_spell_projection() {
    use ironsmith::alternative_cast::CastingMethod;
    let alice = PlayerId(0);
    for name in ["Karfell Harbinger", "Unblinking Observer"] {
        for definition in definitions(name) {
            let mut g = game();
            let source = g.create_object_from_definition(&definition, alice, Zone::Battlefield);
            activate(&mut g, source);
            let card = object(&mut g, "Mana cost: {5}\nType: Sorcery\nYou gain 1 life.\nHarmonize {U}", Zone::Graveyard, alice);
            let action = compute_legal_actions(&g, alice).unwrap().into_iter().find(|a|
                matches!(a, LegalAction::CastSpell { spell_id, casting_method: CastingMethod::Alternative(_), .. } if *spell_id == card)).unwrap();
            announce_action(&mut g, action);
            assert_eq!(g.player(alice).unwrap().mana_pool.total(), 0);
        }
    }
}
