//! Public compiler -> serialized artifact -> live-game graveyard permission coverage.
use ironsmith::ability::AbilityKind;
use ironsmith::cards::{CardDefinition, generated_definition_has_unimplemented_content};
use ironsmith::decision::{LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, resolve_stack_entry,
};
use ironsmith::game_state::Phase;
use ironsmith::mana::ManaSymbol;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{GameProgress, GameState, ObjectId, PlayerId, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_runtime_catalog::CardRegistryArtifactExt;

const DISCARD: &str = "You may cast this card from your graveyard by discarding two cards in addition to paying its other costs.";
const LIFE_DISCARD: &str = "You may cast this card from your graveyard by paying 3 life and discarding a card in addition to paying its other costs.";
const EXILE: &str = "You may cast this card from your graveyard by exiling another creature card from your graveyard in addition to paying its other costs.";
const LIFE_SACRIFICE: &str = "You may cast this card from your graveyard by paying 2 life and sacrificing an artifact or creature in addition to paying its other costs.";
const ONCE: &str = "Once during each of your turns, you may cast a creature spell from your graveyard by exiling three other cards from your graveyard in addition to paying its other costs.";

fn definitions(name: &str, text: &str) -> [CardDefinition; 2] {
    let direct =
        compile_to_runtime_definition(name, text, false).unwrap_or_else(|e| panic!("{name}: {e}"));
    let (artifact, _) = compile_to_artifact(name, text, false).unwrap();
    let restored: CompiledCardArtifact =
        serde_json::from_slice(&serde_json::to_vec(&artifact).unwrap()).unwrap();
    restored.validate().unwrap();
    let mut registry = ironsmith::cards::CardRegistry::new();
    registry.register_compiled_artifact(&restored).unwrap();
    [direct, registry.get(name).unwrap().clone()]
}

fn probes(permission: &str) -> [CardDefinition; 2] {
    definitions(
        "Graveyard permission probe",
        &format!("Mana cost: {{B}}\nType: Creature\nPower/Toughness: 2/2\n{permission}"),
    )
}

fn plain() -> CardDefinition {
    compile_to_runtime_definition(
        "Payment resource",
        "Mana cost: {B}\nType: Creature\nPower/Toughness: 1/1",
        false,
    )
    .unwrap()
}

fn game() -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = PlayerId::from_index(0);
    game.turn.priority_player = Some(PlayerId::from_index(0));
    game.player_mut(PlayerId::from_index(0))
        .unwrap()
        .mana_pool
        .add(ManaSymbol::Black, 10);
    game
}

fn castable(game: &GameState, card: ObjectId) -> Option<LegalAction> {
    compute_legal_actions(game, PlayerId::from_index(0))
        .unwrap()
        .into_iter()
        .find(|a| matches!(a, LegalAction::CastSpell { spell_id, .. } if *spell_id == card))
}

fn cast(game: &mut GameState, card: ObjectId) {
    let action = castable(game, card).expect("permission and every payment should be available");
    let mut state = PriorityLoopState::new(2);
    let mut queue = TriggerQueue::new();
    let mut dm = SelectFirstDecisionMaker;
    let mut progress = apply_priority_response_with_dm(
        game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        &mut dm,
    )
    .unwrap();
    for _ in 0..30 {
        if state.pending_cast.is_none() && !game.stack_is_empty() {
            break;
        }
        if let GameProgress::NeedsDecisionCtx(ctx) = progress {
            progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &ctx, &mut dm)
                .unwrap();
        } else {
            break;
        }
    }
    assert!(
        state.pending_cast.is_none(),
        "all mandatory costs must finish"
    );
    assert_eq!(game.stack.len(), 1);
    resolve_stack_entry(game).unwrap();
}

#[test]
fn graveyard_additional_costs_frozen_card_permissions_compile_and_round_trip() {
    let cards: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/graveyard_additional_costs.json.fixture"
    ))
    .unwrap();
    assert_eq!(cards.as_array().unwrap().len(), 8);
    for card in cards.as_array().unwrap() {
        for definition in definitions(
            card["name"].as_str().unwrap(),
            card["text"].as_str().unwrap(),
        ) {
            assert!(!generated_definition_has_unimplemented_content(&definition));
            let rendered =
                ironsmith_text::compiled_text::unprocessed_compiled_lines(&definition).join("\n");
            assert!(
                rendered.contains("in addition to paying its other costs"),
                "{rendered}"
            );
            if card["name"] == "Helbrute" {
                assert!(
                    rendered.contains("exiling another creature card"),
                    "{rendered}"
                );
            }
            if card["name"] == "Kotis, Sibsig Champion" {
                assert!(rendered.contains("exiling three other cards"), "{rendered}");
            }
        }
    }
}

#[test]
fn graveyard_discard_permission_requires_printed_mana_and_exact_discard_count() {
    let alice = PlayerId::from_index(0);
    for definition in probes(DISCARD) {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, alice, Zone::Graveyard);
        assert!(castable(&game, source).is_none());
        game.create_object_from_definition(&plain(), alice, Zone::Hand);
        assert!(castable(&game, source).is_none());
        game.create_object_from_definition(&plain(), alice, Zone::Hand);
        game.player_mut(alice).unwrap().mana_pool.empty();
        assert!(
            castable(&game, source).is_none(),
            "discard is additional to printed mana"
        );
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Black, 1);
        cast(&mut game, source);
        assert!(game.player(alice).unwrap().hand.is_empty());
        assert_eq!(game.player(alice).unwrap().graveyard.len(), 2);
        assert_eq!(game.battlefield.len(), 1);
        assert_eq!(game.player(alice).unwrap().life, 20);
    }
}

#[test]
fn graveyard_life_and_discard_are_both_mandatory_and_do_not_exile_on_resolution() {
    let alice = PlayerId::from_index(0);
    for definition in probes(LIFE_DISCARD) {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, alice, Zone::Graveyard);
        game.create_object_from_definition(&plain(), alice, Zone::Hand);
        game.player_mut(alice).unwrap().life = 2;
        assert!(castable(&game, source).is_none());
        game.player_mut(alice).unwrap().life = 20;
        cast(&mut game, source);
        assert_eq!(game.player(alice).unwrap().life, 17);
        assert!(game.player(alice).unwrap().hand.is_empty());
        let permanent = *game.battlefield.last().unwrap();
        let returned = game
            .move_object_by_effect(permanent, Zone::Graveyard)
            .unwrap();
        assert!(
            game.object(returned)
                .is_some_and(|o| o.zone == Zone::Graveyard)
        );
        game.create_object_from_definition(&plain(), alice, Zone::Hand);
        assert!(
            castable(&game, returned).is_some(),
            "no once-per-turn or exile rider was authored"
        );
    }
}

#[test]
fn graveyard_permission_is_source_scoped_zone_scoped_and_obeys_normal_timing() {
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for definition in probes(LIFE_DISCARD) {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, alice, Zone::Graveyard);
        let unrelated = game.create_object_from_definition(&plain(), alice, Zone::Graveyard);
        let foreign = game.create_object_from_definition(&definition, bob, Zone::Graveyard);
        let exiled = game.create_object_from_definition(&definition, alice, Zone::Exile);
        game.create_object_from_definition(&plain(), alice, Zone::Hand);
        assert!(castable(&game, source).is_some());
        for card in [unrelated, foreign, exiled] {
            assert!(castable(&game, card).is_none());
        }
        game.turn.phase = Phase::Combat;
        assert!(castable(&game, source).is_none());
        game.turn.phase = Phase::FirstMain;
        game.turn.active_player = bob;
        assert!(castable(&game, source).is_none());
        game.turn.active_player = alice;
        game.turn.turn_number += 2;
        assert!(
            castable(&game, source).is_some(),
            "a static permission has no turn expiry"
        );
        let ability = definition
            .abilities
            .iter()
            .find(|a| matches!(&a.kind, AbilityKind::Static(s) if s.grant_spec().is_some()));
        assert_eq!(ability.unwrap().functional_zones, vec![Zone::Graveyard]);
    }
}

#[test]
fn graveyard_exile_cost_excludes_the_cast_card_and_opponents_graveyards() {
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for definition in probes(EXILE) {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, alice, Zone::Graveyard);
        assert!(
            castable(&game, source).is_none(),
            "cannot exile the spell to pay for itself"
        );
        let foreign = game.create_object_from_definition(&plain(), bob, Zone::Graveyard);
        assert!(
            castable(&game, source).is_none(),
            "your graveyard excludes opponents' cards"
        );
        game.create_object_from_definition(&plain(), alice, Zone::Hand);
        assert!(
            castable(&game, source).is_none(),
            "hand cards do not pay graveyard exile costs"
        );
        game.create_object_from_definition(&plain(), alice, Zone::Graveyard);
        cast(&mut game, source);
        assert_eq!(game.exile.len(), 1);
        assert_eq!(game.object(foreign).unwrap().zone, Zone::Graveyard);
        assert_eq!(game.battlefield.len(), 1);
    }
}

#[test]
fn graveyard_life_sacrifice_cost_accepts_either_type_but_only_your_permanents() {
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for definition in probes(LIFE_SACRIFICE) {
        for type_line in ["Artifact", "Creature"] {
            let mut game = game();
            let source = game.create_object_from_definition(&definition, alice, Zone::Graveyard);
            game.create_object_from_definition(&plain(), bob, Zone::Battlefield);
            assert!(castable(&game, source).is_none());
            let resource = compile_to_runtime_definition(
                "Sacrifice resource",
                &format!("Type: {type_line}\nPower/Toughness: 1/1"),
                false,
            )
            .unwrap();
            game.create_object_from_definition(&resource, alice, Zone::Battlefield);
            cast(&mut game, source);
            assert_eq!(game.player(alice).unwrap().life, 18);
            assert_eq!(game.player(alice).unwrap().graveyard.len(), 1);
        }
    }
}

#[test]
fn graveyard_once_permission_retains_owner_type_cost_and_usage_limit() {
    let alice = PlayerId::from_index(0);
    for definition in probes(ONCE) {
        let mut game = game();
        let grantor = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let spell = game.create_object_from_definition(&plain(), alice, Zone::Graveyard);
        let land = compile_to_runtime_definition("Exile resource", "Type: Land", false).unwrap();
        for _ in 0..3 {
            game.create_object_from_definition(&land, alice, Zone::Graveyard);
        }
        assert!(castable(&game, spell).is_some());
        cast(&mut game, spell);
        assert_eq!(game.exile.len(), 3);
        let second = game.create_object_from_definition(&plain(), alice, Zone::Graveyard);
        for _ in 0..3 {
            game.create_object_from_definition(&land, alice, Zone::Graveyard);
        }
        assert!(
            castable(&game, second).is_none(),
            "the grant's once-per-turn use was consumed"
        );
        assert_eq!(game.object(grantor).unwrap().zone, Zone::Battlefield);
        game.next_turn();
        game.turn.phase = Phase::FirstMain;
        game.turn.priority_player = Some(alice);
        assert!(
            castable(&game, second).is_none(),
            "permission is limited to your turns"
        );
        game.next_turn();
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        game.turn.priority_player = Some(alice);
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Black, 1);
        assert!(
            castable(&game, second).is_some(),
            "usage resets on the next turn"
        );
        game.move_object_by_effect(grantor, Zone::Exile).unwrap();
        assert!(
            castable(&game, second).is_none(),
            "the grant ends when its source leaves"
        );
    }
}

#[test]
fn graveyard_exile_permission_preflight_preserves_composed_choice_dependency() {
    let alice = PlayerId::from_index(0);
    for definition in probes(EXILE) {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, alice, Zone::Graveyard);
        let resource = game.create_object_from_definition(&plain(), alice, Zone::Graveyard);
        let object = game.object(source).unwrap();
        let method = ironsmith::decision::resolve_play_from_alternative_method(
            &game,
            alice,
            object,
            Zone::Graveyard,
            object.alternative_casts.len(),
        )
        .expect("the static permission supplies an alternative cast independently of payment");
        let mut insufficient = game.clone();
        insufficient
            .move_object_by_effect(resource, Zone::Hand)
            .unwrap();
        let graveyard_before = insufficient.player(alice).unwrap().graveyard.clone();
        assert!(
            ironsmith::cost::can_pay_cost_with_reason(
                &insufficient,
                source,
                alice,
                method.total_cost().unwrap(),
                ironsmith::costs::PaymentReason::CastSpell,
            )
            .is_err(),
            "the spell itself cannot fill the missing other-card choice"
        );
        assert_eq!(
            insufficient.player(alice).unwrap().graveyard,
            graveyard_before
        );
        assert_eq!(insufficient.player(alice).unwrap().life, 20);
        assert_eq!(insufficient.player(alice).unwrap().mana_pool.total(), 10);
        let non_mana = method.non_mana_costs();
        assert_eq!(non_mana.len(), 1);
        let sequence = non_mana[0]
            .effect_ref()
            .unwrap()
            .downcast_ref::<ironsmith::effects::SequenceEffect>()
            .expect("one compiler cost keeps its choice and consumer in a composed runtime cost");
        assert_eq!(sequence.effects.len(), 2);
        let separate = ironsmith::costs::Cost::try_effects(sequence.effects.clone()).unwrap();
        let reason = ironsmith::costs::PaymentReason::CastSpell;
        assert!(
            ironsmith::cost::can_pay_cost_with_reason(&game, source, alice, &separate, reason)
                .is_ok(),
            "the component-wise checker sees the dependent selection"
        );
        assert!(
            ironsmith::cost::can_pay_cost_with_reason(
                &game,
                source,
                alice,
                method.total_cost().unwrap(),
                reason,
            )
            .is_ok(),
            "wrapping the same components must preserve payable tag dependencies"
        );
        let nested = ironsmith::costs::Cost::try_effects(vec![ironsmith::effect::Effect::new(
            ironsmith::effects::SequenceEffect::new(vec![ironsmith::effect::Effect::new(
                sequence.clone(),
            )]),
        )])
        .unwrap();
        assert!(
            ironsmith::cost::can_pay_cost_with_reason(&game, source, alice, &nested, reason)
                .is_ok(),
            "nested wrappers recurse only into their child costs"
        );
        let unbound = ironsmith::costs::Cost::try_effects(vec![ironsmith::effect::Effect::new(
            ironsmith::effects::SequenceEffect::new(vec![sequence.effects[1].clone()]),
        )])
        .unwrap();
        assert!(
            ironsmith::cost::can_pay_cost_with_reason(&game, source, alice, &unbound, reason)
                .is_err(),
            "a consumer without a preceding choice must remain unpayable"
        );
        let mut impossible_children = sequence.effects.clone();
        impossible_children.push(ironsmith::effect::Effect::pay_life(21));
        let impossible = ironsmith::costs::Cost::try_effects(vec![ironsmith::effect::Effect::new(
            ironsmith::effects::SequenceEffect::new(impossible_children),
        )])
        .unwrap();
        assert!(
            ironsmith::cost::can_pay_cost_with_reason(&game, source, alice, &impossible, reason)
                .is_err(),
            "a payable choice cannot make an impossible later payment legal"
        );
        let mut non_cost_children = sequence.effects.clone();
        non_cost_children.push(ironsmith::effect::Effect::scry(1));
        assert!(
            ironsmith::costs::Cost::try_effect(ironsmith::effect::Effect::new(
                ironsmith::effects::SequenceEffect::new(non_cost_children),
            ))
            .is_err(),
            "non-cost instructions remain rejected"
        );
        assert_eq!(
            game.player(alice).unwrap().life,
            20,
            "failed preflight cannot pay part of a compound cost"
        );
        assert_eq!(game.object(source).unwrap().zone, Zone::Graveyard);
        assert_eq!(game.object(resource).unwrap().zone, Zone::Graveyard);
        assert_eq!(
            game.player(alice).unwrap().mana_pool.total(),
            10,
            "preflight cannot spend mana or move the chosen cards"
        );
    }
}

#[test]
fn sequence_cost_preflight_preserves_child_object_filters() {
    use ironsmith::cost::{TotalCost, can_pay_cost_with_reason};
    use ironsmith::costs::{Cost, PaymentReason};
    use ironsmith::effect::{ChoiceCount, Effect};
    use ironsmith::effects::{ExileEffect, SequenceEffect};
    use ironsmith::target::{ChooseSpec, ObjectFilter, PlayerFilter};

    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = game();
    let source = game.create_object_from_definition(&plain(), alice, Zone::Stack);
    let land = compile_to_runtime_definition("Noncreature hand card", "Type: Land", false).unwrap();
    let wrong_type = game.create_object_from_definition(&land, alice, Zone::Hand);
    let wrong_owner = game.create_object_from_definition(&plain(), bob, Zone::Hand);
    let filter = ObjectFilter::default()
        .in_zone(Zone::Hand)
        .owned_by(PlayerFilter::You)
        .with_type(ironsmith::CardType::Creature)
        .other();
    let exile = Effect::new(ExileEffect::with_spec(
        ChooseSpec::Object(filter).with_count(ChoiceCount::exactly(1)),
    ));
    let cost = TotalCost::from_cost(
        Cost::try_effect(Effect::new(SequenceEffect::new(vec![exile]))).unwrap(),
    );
    assert!(
        can_pay_cost_with_reason(&game, source, alice, &cost, PaymentReason::CastSpell).is_err(),
        "wrapping an exile effect cannot erase its creature-only hand filter"
    );
    assert_eq!(game.object(wrong_type).unwrap().zone, Zone::Hand);
    assert_eq!(game.object(wrong_owner).unwrap().zone, Zone::Hand);
    assert_eq!(game.player(alice).unwrap().life, 20);
    assert_eq!(game.player(alice).unwrap().mana_pool.total(), 10);
    let eligible = game.create_object_from_definition(&plain(), alice, Zone::Hand);
    assert!(
        can_pay_cost_with_reason(&game, source, alice, &cost, PaymentReason::CastSpell).is_ok()
    );
    assert_eq!(game.object(eligible).unwrap().zone, Zone::Hand);
    assert!(game.exile.is_empty(), "preflight never pays the cost");
}
