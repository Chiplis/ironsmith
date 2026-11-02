//! Disjunctions compose existing payment operations without nested OR loss.
use ironsmith::ability::AbilityKind;
use ironsmith::decision::{LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse, apply_priority_response_with_dm, apply_decision_context_with_dm, resolve_stack_entry_with};
use ironsmith::{CardDefinition, GameState, PlayerId, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
const A: PlayerId = PlayerId(0);

fn definitions() -> [CardDefinition; 2] {
    let text = "Type: Artifact\nSacrifice a creature or discard a card or exile exactly two cards from your hand and/or graveyard or pay 2 life: You gain 1 life.";
    let (result, loss) = ironsmith_compiler::parse_loss::capture(||
        ironsmith_compiler::word_claims::with_enforcement_options(true, true, ||
            ironsmith_compiler_runtime::compile_to_artifact("Independent price choices", text, false)));
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let (artifact, direct) = result.unwrap();
    let decoded = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    [direct, ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&decoded).unwrap()]
}

#[test]
fn four_disjunctive_prices_remain_independent_and_pay_only_selected_branch() {
    for definition in definitions() {
        let ability = definition.abilities.iter().find_map(|ability| match &ability.kind {
            AbilityKind::Activated(ability) => Some(ability), _ => None,
        }).unwrap();
        assert_eq!(ability.mana_cost.as_one_of().unwrap().len(), 4);
        for branch in 0..4 {
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            game.turn.active_player = A;
            game.turn.priority_player = Some(A);
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let material = ironsmith_compiler_runtime::compile_to_runtime_definition("Material", "Type: Creature\nPower/Toughness: 2/2", false).unwrap();
            let zones: &[Zone] = match branch { 0 => &[Zone::Battlefield], 1 => &[Zone::Hand], 2 => &[Zone::Graveyard, Zone::Graveyard], _ => &[] };
            let materials: Vec<_> = zones.iter().map(|zone| {
                let id = game.create_object_from_definition(&material, A, *zone);
                game.object(id).unwrap().stable_id
            }).collect();
            let action = compute_legal_actions(&game, A).unwrap().into_iter().find(|action| matches!(action, LegalAction::ActivateAbility { source: id, .. } if *id == source)).unwrap();
            let mut state = PriorityLoopState::new(2);
            let mut queue = ironsmith::triggers::TriggerQueue::new();
            let mut dm = SelectFirstDecisionMaker;
            let mut progress = apply_priority_response_with_dm(&mut game, &mut queue, &mut state, &PriorityResponse::PriorityAction(action), &mut dm).unwrap();
            for _ in 0..64 {
                if !state.has_pending_action() { break; }
                let ironsmith::GameProgress::NeedsDecisionCtx(context) = progress else { panic!("{progress:?}"); };
                progress = apply_decision_context_with_dm(&mut game, &mut queue, &mut state, &context, &mut dm).unwrap();
            }
            assert!(!state.has_pending_action());
            assert_eq!(game.player(A).unwrap().life, if branch == 3 { 18 } else { 20 });
            for stable in materials {
                assert_eq!(game.object(game.find_object_by_stable_id(stable).unwrap()).unwrap().zone, if branch == 2 { Zone::Exile } else { Zone::Graveyard });
            }
            resolve_stack_entry_with(&mut game, &mut dm).unwrap();
            assert_eq!(game.player(A).unwrap().life, if branch == 3 { 19 } else { 21 });
            assert!(game.battlefield.contains(&source));
        }
    }
}

#[test]
fn named_source_additional_prices_preserve_choice_and_optional_payment() {
    for (name, tail, optional) in [
        ("Named price", "exile exactly two cards from your hand and/or graveyard or pay {3}", false),
        ("Named price, Complete Reference", "you may exile exactly two cards from your hand and/or graveyard", true),
    ] {
        let text = format!("Mana cost: {{0}}\nType: Instant\nAs an additional cost to cast {name}, {tail}.\nYou gain 1 life.");
        let (result, loss) = ironsmith_compiler::parse_loss::capture(||
            ironsmith_compiler::word_claims::with_enforcement_options(true, true, ||
                ironsmith_compiler_runtime::compile_to_artifact(name, &text, false)));
        assert!(!loss.is_lossy(), "{}", loss.reasons_text());
        let (artifact, direct) = result.unwrap();
        let decoded = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
        for definition in [direct, ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&decoded).unwrap()] {
            if optional { assert_eq!(definition.optional_costs.len(), 1); }
            else {
                let [cost] = definition.additional_cost.as_all().unwrap() else { panic!("one modal additional price"); };
                let modes = cost.effect_ref().unwrap().downcast_ref::<ironsmith::effects::ChooseModeEffect>().unwrap();
                assert_eq!(modes.modes.len(), 2);
            }
        }
    }
}
