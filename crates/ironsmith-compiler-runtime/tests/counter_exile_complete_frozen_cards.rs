//! Complete frozen-card integration and mutation witnesses, authored UNRUN.
//! Direct compilation/conversion and serialized artifact loading are invoked
//! independently. No schema/cache admission or recovery result is claimed.

use ironsmith::ability::{Ability, AbilityKind};
use ironsmith::alternative_cast::{AlternativeCastingMethod, CastingMethod};
use ironsmith::card::{LinkedFaceLayout, PowerToughness};
use ironsmith::cards::{CardDefinition, builders::CardDefinitionBuilder};
use ironsmith::cost::TotalCost;
use ironsmith::costs::Cost;
use ironsmith::decision::{LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
use ironsmith::effects::CounterEffect;
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, put_triggers_on_stack_with_dm, resolve_stack_entry,
};
use ironsmith::game_state::Phase;
use ironsmith::mana::{ManaCost, ManaSymbol};
use ironsmith::target::{ChooseSpec, ObjectFilter};
use ironsmith::triggers::TriggerQueue;
use ironsmith::{CardId, CardType, GameState, ObjectId, PlayerId, Target, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_builder_to_artifact, compile_builder_to_runtime_definition};
use ironsmith_core::{CounterExileGate, CounterExilePermission};
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;
use serde_json::{Value, json};

const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
const SPELLJACK: &str = "7687b2a7-816d-4416-979b-675e35e235fc";
const DECREE: &str = "d7cba934-02ad-4677-bb4d-50808b01b4f9";
const KHERU: &str = "c01411e0-77b2-4e65-a369-5dbe13745769";

fn metadata(id: &str) -> Value {
    let frozen: Value = serde_json::from_str(include_str!(
        "../../../reports/countered-spell-durable-permission-20261008/frozen-inputs.json"
    )).unwrap();
    frozen["actual_card_metadata"].as_array().unwrap().iter()
        .find(|row| row["oracle_id"] == id).unwrap().clone()
}

fn source(row: &Value) -> String {
    let mut text = format!("Mana cost: {}\nType: {}\n",
        row["mana_cost"].as_str().unwrap(), row["type_line"].as_str().unwrap());
    if let (Some(power), Some(toughness)) = (row["power"].as_str(), row["toughness"].as_str()) {
        text.push_str(&format!("Power/Toughness: {power}/{toughness}\n"));
    }
    text.push_str(row["oracle_text"].as_str().unwrap());
    text
}

fn expected(id: &str) -> CounterExilePermission {
    CounterExilePermission {
        gate: if id == DECREE { CounterExileGate::PermanentSpell } else { CounterExileGate::AnySpell },
        allow_land: id == SPELLJACK,
    }
}

fn compile(id: &str) -> (CardDefinition, CompiledCardArtifact) {
    let row = metadata(id);
    let text = source(&row);
    let name = row["name"].as_str().unwrap();
    let builder = || ironsmith_compiler::CardDefinitionBuilder::new(CardId::new(), name)
        .first_printed_set_name(row["first_printed_set_name"].as_str().unwrap());
    // The direct builder route calls CompilerFacade and then
    // into_runtime_compiled_card_text/into_runtime_definition directly.
    // Preserve original first-print metadata alongside the complete cost,
    // type, P/T and raw rules-body source below.
    let (native, direct_loss) = ironsmith_compiler::parse_loss::capture(||
        compile_builder_to_runtime_definition(builder(), text.clone(), false));
    let native = native.expect("complete frozen input through direct model conversion");
    assert!(!direct_loss.is_lossy(), "{name}: {}", direct_loss.reasons_text());
    // The second result here is already materialized. Deliberately discard it;
    // it is not the independent native result used above.
    let (compiled, artifact_loss) = ironsmith_compiler::parse_loss::capture(||
        compile_builder_to_artifact(builder(), text, false));
    let (artifact, _) = compiled.expect("complete frozen input through artifact production");
    assert!(!artifact_loss.is_lossy(), "{name}: {}", artifact_loss.reasons_text());
    assert_eq!(native.card.first_printed_set_name.as_deref(), row["first_printed_set_name"].as_str());
    assert_eq!(counter(&native).exile_permission, Some(expected(id)));
    (native, artifact)
}

fn routes(id: &str) -> [CardDefinition; 2] {
    let (native, artifact) = compile(id);
    artifact.validate().unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(restored, artifact);
    let loaded = materialize_artifact(&restored).unwrap();
    assert_eq!(native.card.first_printed_set_name, loaded.card.first_printed_set_name);
    assert_eq!(counter(&native), counter(&loaded));
    [native, loaded]
}

fn counter(definition: &CardDefinition) -> CounterEffect {
    fn collect(effect: &ironsmith::effect::Effect, found: &mut Vec<CounterEffect>) {
        if let Some(value) = effect.downcast_ref::<CounterEffect>() { found.push(value.clone()); }
        effect.visit_child_effects(&mut |child| collect(child, found));
    }
    let mut found = Vec::new();
    if let Some(program) = &definition.spell_effect {
        for effect in program.flattened_default_effects() { collect(effect, &mut found); }
    }
    for ability in &definition.abilities {
        match &ability.kind {
            AbilityKind::Triggered(ability) => {
                for effect in ability.effects.flattened_default_effects() { collect(effect, &mut found); }
            }
            AbilityKind::Activated(ability) => {
                for effect in ability.effects.flattened_default_effects() { collect(effect, &mut found); }
            }
            _ => {}
        }
    }
    assert_eq!(found.len(), 1, "one counter owner in each complete frozen card");
    found.remove(0)
}

fn game() -> GameState {
    let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
    main(&mut game, B);
    game
}

fn main(game: &mut GameState, player: PlayerId) {
    game.turn.active_player = player;
    game.turn.priority_player = Some(player);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
}

fn casts(game: &GameState, player: PlayerId, id: ObjectId) -> Vec<LegalAction> {
    compute_legal_actions(game, player).unwrap().into_iter().filter(|action|
        matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == id)).collect()
}

fn act(game: &mut GameState, player: PlayerId, action: LegalAction) {
    game.turn.priority_player = Some(player);
    let mut state = PriorityLoopState::new(game.players.len());
    let mut queue = TriggerQueue::new();
    let mut dm = SelectFirstDecisionMaker;
    let mut progress = apply_priority_response_with_dm(game, &mut queue, &mut state,
        &PriorityResponse::PriorityAction(action), &mut dm).unwrap();
    for _ in 0..40 {
        if !state.has_pending_action() { break; }
        let ironsmith::GameProgress::NeedsDecisionCtx(context) = progress else {
            panic!("pending action without a decision: {progress:?}");
        };
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &context, &mut dm).unwrap();
    }
    assert!(!state.has_pending_action());
    put_triggers_on_stack_with_dm(game, &mut queue, &mut dm).unwrap();
}

fn normal_cast(game: &mut GameState, player: PlayerId, hand: ObjectId) -> ObjectId {
    game.turn.priority_player = Some(player);
    let stable = game.object(hand).unwrap().stable_id;
    let action = casts(game, player, hand).into_iter().find(|action| matches!(action,
        LegalAction::CastSpell { casting_method: CastingMethod::Normal, .. })).unwrap();
    act(game, player, action);
    let stack = game.find_object_by_stable_id(stable).unwrap();
    assert_eq!(game.object(stack).unwrap().zone, Zone::Stack);
    stack
}

fn target_definition(kind: CardType, mandatory: bool) -> CardDefinition {
    let mut builder = CardDefinitionBuilder::new(CardId::new(), "Complete-card counter target")
        .card_types(vec![kind]).mana_cost(ManaCost::new().add_generic(7));
    if kind == CardType::Creature { builder = builder.power_toughness(PowerToughness::fixed(4, 4)); }
    if mandatory {
        builder = builder.additional_cost(TotalCost::from_costs(vec![
            Cost::mana(ManaCost::from_symbols(vec![ManaSymbol::Red])), Cost::life(3),
        ]));
    }
    builder.build()
}

fn cast_and_counter(
    game: &mut GameState, definition: &CardDefinition, target_definition: &CardDefinition,
) -> (ObjectId, ObjectId) {
    main(game, B);
    let target = game.create_object_from_definition(target_definition, B, Zone::Hand);
    let stable = game.object(target).unwrap().stable_id;
    game.player_mut(B).unwrap().mana_pool.add(ManaSymbol::Colorless, 7);
    game.player_mut(B).unwrap().mana_pool.add(ManaSymbol::Red, 1);
    let target = normal_cast(game, B, target);
    let source = game.create_object_from_definition(definition, A, Zone::Hand);
    let source_stable = game.object(source).unwrap().stable_id;
    // Both frozen instants cost six total; blue can pay the generic portion.
    game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Blue, 6);
    let source = normal_cast(game, A, source);
    assert_eq!(game.stack.last().unwrap().targets, vec![Target::Object(target)]);
    assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
    resolve_stack_entry(game).unwrap();
    assert!(game.stack_is_empty(), "the full frozen counter must remove the opposing spell");
    let departed_source = game.find_object_by_stable_id(source_stable).unwrap();
    assert_ne!(source, departed_source);
    assert_eq!(game.object(departed_source).unwrap().zone, Zone::Graveyard,
        "the counter source naturally leaves the stack after granting permission");
    assert!(game.object(target).is_none());
    (source, game.find_object_by_stable_id(stable).unwrap())
}

#[test]
fn frozen_instants_actually_counter_permanents_and_preserve_nonpermanent_gate_behavior() {
    for id in [SPELLJACK, DECREE] {
        for definition in routes(id) {
            for kind in [CardType::Creature, CardType::Instant, CardType::Sorcery] {
                let mut game = game();
                let (_, moved) = cast_and_counter(&mut game, &definition, &target_definition(kind, false));
                let eligible = id == SPELLJACK || kind == CardType::Creature;
                assert_eq!(game.object(moved).unwrap().zone, if eligible { Zone::Exile } else { Zone::Graveyard });
                let grants = game.effect_store.grant_registry.granted_alternative_casts_for_card(
                    &game, moved, Zone::Exile, A);
                assert_eq!(grants.len(), usize::from(eligible));
                assert!(casts(&game, B, moved).is_empty(), "opponent ownership is not permission");
                if !eligible { continue; }
                assert!(game.effect_store.grant_registry.grants.iter().all(|grant|
                    grant.target_id == Some(moved) && grant.target_stable_id.is_none()));
                main(&mut game, A);
                game.turn.turn_number += 2;
                let stable = game.object(moved).unwrap().stable_id;
                let action = casts(&game, A, moved).into_iter().next().expect("full-card free casting permission");
                act(&mut game, A, action);
                let stack = game.find_object_by_stable_id(stable).unwrap();
                assert_eq!(game.object(stack).unwrap().zone, Zone::Stack);
                assert_eq!(game.object(stack).unwrap().caster_mana_spent_to_cast, Some(0));
                assert_eq!(game.controller_of_id(stack), Some(A));
                assert_eq!(game.object(stack).unwrap().owner, B);
                assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
            }
        }
    }
}

#[test]
fn full_frozen_permissions_survive_source_departure_but_preserve_timing_and_additional_costs() {
    for id in [SPELLJACK, DECREE] {
        for definition in routes(id) {
            let mut game = game();
            let (_, exiled) = cast_and_counter(&mut game, &definition, &target_definition(CardType::Creature, true));
            assert_eq!(game.object(exiled).unwrap().zone, Zone::Exile);
            let stable = game.object(exiled).unwrap().stable_id;
            game.player_mut(A).unwrap().mana_pool.red = 1;
            game.turn.priority_player = Some(A);
            assert!(casts(&game, A, exiled).is_empty(), "opponent turn is still wrong for a creature");
            game.turn.turn_number += 3;
            main(&mut game, A);
            game.player_mut(A).unwrap().mana_pool.red = 0;
            assert!(casts(&game, A, exiled).is_empty(), "free printed mana does not waive mandatory red mana");
            game.player_mut(A).unwrap().mana_pool.red = 1;
            let mut unaffordable_life = game.clone();
            unaffordable_life.player_mut(A).unwrap().life = 2;
            assert!(casts(&unaffordable_life, A, exiled).is_empty(), "mandatory life is still required");
            let actions = casts(&game, A, exiled);
            assert_eq!(actions.len(), 1, "only the integrated free price is authorized");
            act(&mut game, A, actions[0].clone());
            assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
            assert_eq!(game.player(A).unwrap().life, 17);
            let stack = game.find_object_by_stable_id(stable).unwrap();
            assert_eq!(game.object(stack).unwrap().caster_mana_spent_to_cast, Some(1));
            resolve_stack_entry(&mut game).unwrap();
            let creature = game.find_object_by_stable_id(stable).unwrap();
            assert_eq!(game.object(creature).unwrap().zone, Zone::Battlefield);
            assert_eq!(game.controller_of_id(creature), Some(A));
            assert_eq!(game.object(creature).unwrap().owner, B);
        }
    }
}

fn modal_creature_land(game: &mut GameState) -> CardDefinition {
    let front_id = CardId::new();
    let back_id = CardId::new();
    let front = CardDefinitionBuilder::new(front_id, "Countered modal creature")
        .card_types(vec![CardType::Creature]).power_toughness(PowerToughness::fixed(2, 2))
        .mana_cost(ManaCost::new().add_generic(7))
        .other_face(back_id).other_face_name("Countered modal land")
        .linked_face_layout(LinkedFaceLayout::TransformLike).build();
    let back = CardDefinitionBuilder::new(back_id, "Countered modal land")
        .card_types(vec![CardType::Land]).other_face(front_id).other_face_name("Countered modal creature")
        .linked_face_layout(LinkedFaceLayout::TransformLike).build();
    game.register_linked_face_definition(&front);
    game.register_linked_face_definition(&back);
    front
}

#[test]
fn full_spelljack_plays_the_countered_land_face_and_full_decree_only_casts() {
    for id in [SPELLJACK, DECREE] {
        for definition in routes(id) {
            let mut game = game();
            let front = modal_creature_land(&mut game);
            let (_, exiled) = cast_and_counter(&mut game, &definition, &front);
            assert_eq!(game.object(exiled).unwrap().zone, Zone::Exile, "the countered face was permanent");
            let land = LegalAction::PlayLand { land_id: exiled };
            game.turn.priority_player = Some(A);
            assert!(!compute_legal_actions(&game, A).unwrap().contains(&land), "ordinary land timing remains");
            main(&mut game, A);
            let allow_land = id == SPELLJACK;
            assert_eq!(compute_legal_actions(&game, A).unwrap().contains(&land), allow_land);
            if allow_land {
                act(&mut game, A, land);
                assert!(game.battlefield.iter().any(|id| game.object(*id).is_some_and(|object|
                    object.name.as_str() == "Countered modal land" && game.controller_of(object) == A && object.owner == B)));
                assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
            } else {
                let mut forged = game.clone();
                assert!(apply_priority_response_with_dm(&mut forged, &mut TriggerQueue::new(),
                    &mut PriorityLoopState::new(2), &PriorityResponse::PriorityAction(land),
                    &mut SelectFirstDecisionMaker).is_err());
                assert_eq!(forged.object(exiled).unwrap().zone, Zone::Exile);
                let action = casts(&game, A, exiled).into_iter().next().expect("the spell face remains castable");
                act(&mut game, A, action);
            }
        }
    }
}

#[test]
fn full_frozen_free_permission_cannot_combine_morph_or_another_printed_alternative_price() {
    // Thranduil's Decree rules: https://magic.wizards.com/en/news/feature/the-hobbit-release-notes
    // The free price does not combine with morph or another alternative price.
    // Kheru's ordinary hand-to-stack face-down cast is tested separately.
    for id in [SPELLJACK, DECREE] {
        for definition in routes(id) {
            let target = CardDefinitionBuilder::new(CardId::new(), "Countered alternative-price creature")
                .card_types(vec![CardType::Creature]).power_toughness(PowerToughness::fixed(3, 3))
                .mana_cost(ManaCost::new().add_generic(7))
                .alternative_cast(AlternativeCastingMethod::alternative_cost(
                    "Printed alternative", Some(ManaCost::new().add_generic(1)), vec![],
                ))
                .with_ability(Ability::static_ability(ironsmith::static_abilities::StaticAbility::morph(
                    TotalCost::mana(ManaCost::new().add_generic(2)),
                )))
                .build();
            let mut game = game();
            let (source, exiled) = cast_and_counter(&mut game, &definition, &target);
            main(&mut game, A);
            let stable = game.object(exiled).unwrap().stable_id;
            // Affordability alone cannot explain rejection: there is enough
            // mana here for both a {3} face-down cast and the printed {1} price.
            game.player_mut(A).unwrap().mana_pool.colorless = 3;
            let actions = casts(&game, A, exiled);
            assert_eq!(actions.len(), 1, "only the counter-owned free face-up price is granted");
            assert!(!actions.iter().any(|action| matches!(action,
                LegalAction::CastSpell { casting_method: CastingMethod::FaceDown | CastingMethod::FaceDownPlayFrom { .. }, .. }
            )));
            for method in [
                CastingMethod::FaceDownPlayFrom { source, zone: Zone::Exile },
                CastingMethod::PlayFrom { source, zone: Zone::Exile, use_alternative: Some(0) },
                CastingMethod::PlayFrom { source, zone: Zone::Exile, use_alternative: None },
            ] {
                let mut forged = game.clone();
                let before_grants = forged.effect_store.grant_registry.grants.len();
                let before_mana = forged.player(A).unwrap().mana_pool.clone();
                let mut state = PriorityLoopState::new(2);
                let result = apply_priority_response_with_dm(&mut forged, &mut TriggerQueue::new(),
                    &mut state, &PriorityResponse::PriorityAction(LegalAction::CastSpell {
                        spell_id: exiled, from_zone: Zone::Exile, casting_method: method,
                    }), &mut SelectFirstDecisionMaker);
                assert!(result.is_err(), "forged alternate-price announcement cannot use the free origin");
                assert_eq!(forged.object(exiled).unwrap().zone, Zone::Exile);
                assert_eq!(forged.player(A).unwrap().mana_pool, before_mana);
                assert_eq!(forged.effect_store.grant_registry.grants.len(), before_grants);
                assert!(!state.has_pending_action());
            }
            act(&mut game, A, actions[0].clone());
            let stack = game.find_object_by_stable_id(stable).unwrap();
            assert!(!game.is_face_down(stack));
            assert_eq!(game.current_power(stack), Some(3));
            assert_eq!(game.object(stack).unwrap().caster_mana_spent_to_cast, Some(0));
            assert_eq!(game.player(A).unwrap().mana_pool.colorless, 3);
        }
    }
}

fn mutate_counter(artifact: &mut CompiledCardArtifact, mut change: impl FnMut(&mut Value)) {
    fn walk(value: &mut Value, change: &mut impl FnMut(&mut Value), originals: &mut Vec<Value>) {
        if value.get("kind").and_then(Value::as_str) == Some("CounterEffect") {
            originals.push(value.get("payload").expect("wire counter payload").clone());
            change(value.get_mut("payload").expect("wire counter payload"));
            return;
        }
        match value {
            Value::Object(fields) => for value in fields.values_mut() { walk(value, change, originals); },
            Value::Array(values) => for value in values { walk(value, change, originals); },
            _ => {}
        }
    }
    let mut value = serde_json::to_value(&artifact.payload.definition).unwrap();
    let mut originals = Vec::new();
    walk(&mut value, &mut change, &mut originals);
    assert!(!originals.is_empty(), "counter payload must be present");
    // Native-model retention can serialize the same instruction twice. Mutate
    // every copy and prove they describe one identical counter contract.
    assert!(originals.iter().all(|payload| payload == &originals[0]),
        "the complete card must contain one unique counter contract");
    artifact.payload.definition = serde_json::from_value(value).unwrap();
}

#[test]
fn full_card_artifact_matrix_rejects_malformed_riders_and_dangling_target_contracts() {
    for id in [SPELLJACK, DECREE, KHERU] {
        let (_, baseline) = compile(id);
        for invalid in [
            json!({"allow_land": false}),
            json!({"gate": null, "allow_land": false}),
            json!({"gate": "CreatureSpell", "allow_land": false}),
            json!({"gate": "AnySpell"}),
            json!({"gate": "AnySpell", "allow_land": 0}),
            json!({"gate": "AnySpell", "allow_land": false, "player": "Opponent"}),
            json!({"gate": "AnySpell", "allow_land": false, "duration": "EndOfTurn"}),
            json!({"gate": "AnySpell", "allow_land": false, "without_paying_mana_cost": false}),
            json!({"gate": "AnySpell", "allow_land": false, "tag": "another_counter"}),
        ] {
            let mut changed = baseline.clone();
            mutate_counter(&mut changed, |payload| payload["exile_permission"] = invalid.clone());
            assert!(changed.validate().is_err(), "an unacknowledged payload edit fails its checksum");
            changed.refresh_checksum();
            let restored = CompiledCardArtifact::from_json(&changed.to_json().unwrap()).unwrap();
            assert!(materialize_artifact(&restored).is_err(), "no malformed full-card rider fallback: {invalid}");
        }
        let mut source_filter = ObjectFilter::spell();
        source_filter.source = true;
        let mut nested_tag = ObjectFilter::spell();
        nested_tag.any_of = vec![ObjectFilter::tagged("missing_counter_output")];
        for target in [
            ChooseSpec::Tagged("missing_counter_output".into()),
            ChooseSpec::All(ObjectFilter::spell()),
            ChooseSpec::target(ChooseSpec::Object(source_filter)),
            ChooseSpec::target(ChooseSpec::Object(nested_tag)),
        ] {
            let mut changed = baseline.clone();
            mutate_counter(&mut changed, |payload| payload["target"] = serde_json::to_value(&target).unwrap());
            changed.refresh_checksum();
            let restored = CompiledCardArtifact::from_json(&changed.to_json().unwrap()).unwrap();
            assert!(materialize_artifact(&restored).is_err(), "invalid full-card target must fail closed");
        }
    }
}

#[test]
fn full_card_semantic_comparison_detects_well_typed_gate_domain_or_entire_rider_loss() {
    for id in [SPELLJACK, DECREE, KHERU] {
        let (native, baseline) = compile(id);
        let expected_counter = counter(&native);
        for mutation in 0..4 {
            let mut changed = baseline.clone();
            mutate_counter(&mut changed, |payload| match mutation {
                0 => { payload.as_object_mut().unwrap().remove("exile_permission"); }
                1 => payload["exile_permission"] = Value::Null,
                2 => payload["exile_permission"]["gate"] = json!(if id == DECREE { "AnySpell" } else { "PermanentSpell" }),
                3 => payload["exile_permission"]["allow_land"] = json!(!expected(id).allow_land),
                _ => unreachable!(),
            });
            assert!(changed.validate().is_err());
            assert!(materialize_artifact(&changed).is_err());
            changed.refresh_checksum();
            let restored = CompiledCardArtifact::from_json(&changed.to_json().unwrap()).unwrap();
            let loaded = materialize_artifact(&restored).unwrap();
            assert_ne!(counter(&loaded), expected_counter,
                "the full-card source comparison must reject a well-typed semantic mutation");
            // Legacy None and the other supported gate/domain values remain
            // valid vocabulary. Rechecksumming is not source authentication;
            // this assertion is semantic mismatch evidence, not a claim that
            // the existing artifact boundary rejects authenticated-looking loss.
        }
    }
}
