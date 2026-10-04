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
