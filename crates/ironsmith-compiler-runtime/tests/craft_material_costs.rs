//! UNVALIDATED: full frozen faces and independently asserted native Craft payments.
use ironsmith::ability::{AbilityKind, ActivationTiming};
use ironsmith::cards::CardDefinition;
use ironsmith::costs::{CostContext, CostPaymentResult, PaymentReason};
use ironsmith::decision::{DecisionMaker, LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
use ironsmith::decisions::context::{BooleanContext, SelectObjectsContext, TargetsContext};
use ironsmith::effects::{ChooseObjectsEffect, ExileEffect, MoveToZoneEffect};
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, put_triggers_on_stack_with_dm, resolve_stack_entry_with};
use ironsmith::mana::ManaSymbol;
use ironsmith::target::{ChooseSpec, ObjectFilter};
use ironsmith::triggers::TriggerQueue;
use ironsmith::{CardId, CardType, ColorSet, GameProgress, GameState, ObjectId, Phase, PlayerId, Target, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_builder_to_artifact, compile_builder_to_runtime_definition, compile_to_runtime_definition};
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
fn rows() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!("../../../fixtures/craft_material_costs.json.fixture")).unwrap()
}
fn linked(game: &mut GameState, row: &serde_json::Value, route: usize) -> CardDefinition {
    let faces = row["card_faces"].as_array().unwrap();
    let ids = [CardId::new(), CardId::new()];
    let mut definitions = Vec::new();
    for index in 0..2 {
        let face = &faces[index];
        let mut builder = ironsmith_compiler::CardDefinitionBuilder::new(ids[index], face["name"].as_str().unwrap())
            .other_face(ids[1 - index]).other_face_name(faces[1 - index]["name"].as_str().unwrap())
            .linked_face_layout(ironsmith::card::LinkedFaceLayout::TransformLike).transforming_dfc(true);
        if let Some(indicator) = face["color_indicator"].as_array() {
            builder = builder.color_indicator(match indicator[0].as_str().unwrap() {
                "U" => ColorSet::BLUE, "B" => ColorSet::BLACK, "G" => ColorSet::GREEN,
                color => panic!("unexpected indicator {color}"),
            });
        }
        let (direct_result, direct_loss) = ironsmith_compiler::parse_loss::capture(||
            compile_builder_to_runtime_definition(builder.clone(), face["text"].as_str().unwrap(), false));
        let direct = direct_result.unwrap_or_else(|error| panic!("{} direct: {error}", face["name"]));
        assert!(!direct_loss.is_lossy(), "{} direct: {}", face["name"], direct_loss.reasons_text());
        let (result, loss) = ironsmith_compiler::parse_loss::capture(||
            compile_builder_to_artifact(builder, face["text"].as_str().unwrap(), false));
        let (artifact, _) = result.unwrap_or_else(|error| panic!("{}: {error}", face["name"]));
        assert!(!loss.is_lossy(), "{}: {}", face["name"], loss.reasons_text());
        let decoded = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
        decoded.validate().unwrap(); assert_eq!(artifact, decoded);
        let definition = if route == 0 { direct } else { materialize_artifact(&decoded).unwrap() };
        assert!(definition.card.transforming_dfc);
        assert_eq!(definition.card.other_face, Some(ids[1 - index]));
        let rendered = ironsmith_text::canonical_compiled_lines(&definition).join("\n");
        if index == 0 {
            let keyword = face["oracle_text"].as_str().unwrap().lines()
                .find(|line| line.starts_with("Craft")).unwrap().split(" (").next().unwrap();
            assert!(rendered.contains(keyword), "{rendered}");
        }
        game.register_linked_face_definition(&definition); definitions.push(definition);
    }
    definitions.remove(0)
}
fn game() -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.active_player = A; game.turn.priority_player = Some(A);
    game.turn.phase = Phase::FirstMain; game.turn.step = None; game
}
fn printed(game: &mut GameState, owner: PlayerId, zone: Zone, name: &str, text: &str) -> ObjectId {
    let definition = compile_to_runtime_definition(name, text, false).unwrap();
    game.create_object_from_definition(&definition, owner, zone)
}
fn is_visage(row: &serde_json::Value) -> bool { row["card_faces"][0]["name"] == "Visage of Dread" }
fn material(game: &mut GameState, row: &serde_json::Value, owner: PlayerId, zone: Zone) -> ObjectId {
    let text = match row["card_faces"][0]["name"].as_str().unwrap() {
        "Kaslem's Stonetree" => "Type: Land — Cave",
        "Waterlogged Hulk" => "Type: Land — Island",
        "Visage of Dread" => "Type: Creature — Bear\nPower/Toughness: 2/2",
        _ => unreachable!(),
    };
    printed(game, owner, zone, "Craft material", text)
}
fn fund(game: &mut GameState) {
    for symbol in [ManaSymbol::Colorless, ManaSymbol::Blue, ManaSymbol::Black, ManaSymbol::Green] {
        game.player_mut(A).unwrap().mana_pool.add(symbol, 10);
    }
}
fn craft_index(definition: &CardDefinition) -> usize {
    definition.abilities.iter().position(|ability| matches!(&ability.kind,
        AbilityKind::Activated(activated) if activated.timing == ActivationTiming::SorcerySpeed
            && activated.mana_cost.costs().iter().any(|cost| cost.effect_ref().is_some_and(|effect|
                effect.downcast_ref::<ExileEffect>().is_some_and(|exile| matches!(exile.spec, ChooseSpec::Source)))))).unwrap()
}
#[derive(Default)]
struct Choices {
    objects: Option<Vec<ObjectId>>, accept: bool, pause: bool, pending: bool,
    offered: Vec<Vec<ObjectId>>,
}
impl DecisionMaker for Choices {
    fn awaiting_choice(&self) -> bool { self.pending }
    fn decide_objects(&mut self, game: &GameState, context: &SelectObjectsContext) -> Vec<ObjectId> {
        self.offered.push(context.candidates.iter().filter(|object| object.legal).map(|object| object.id).collect());
        if self.pause { self.pending = true; return Vec::new(); }
        self.objects.clone().unwrap_or_else(|| SelectFirstDecisionMaker.decide_objects(game, context))
    }
    fn decide_boolean(&mut self, _: &GameState, context: &BooleanContext) -> bool {
        if self.pause { self.pending = true; return false; }
        self.accept && context.can_accept
    }
    fn decide_targets(&mut self, game: &GameState, context: &TargetsContext) -> Vec<Target> {
        if context.requirements.iter().any(|r| r.legal_targets.contains(&Target::Player(B))) {
            vec![Target::Player(B)]
        } else { SelectFirstDecisionMaker.decide_targets(game, context) }
    }
}
fn action(game: &mut GameState, action: LegalAction, dm: &mut Choices) -> Result<(), String> {
    game.turn.priority_player = Some(A);
    let mut state = PriorityLoopState::new(2); let mut queue = TriggerQueue::new();
    let mut progress = apply_priority_response_with_dm(game, &mut queue, &mut state,
        &PriorityResponse::PriorityAction(action), dm).map_err(|error| error.to_string())?;
    for _ in 0..40 {
        if state.pending_activation.is_none() && state.pending_cast.is_none() { return Ok(()); }
        let GameProgress::NeedsDecisionCtx(context) = progress else { return Err(format!("unfinished payment: {progress:?}")); };
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &context, dm).map_err(|error| error.to_string())?;
    }
    Err("payment did not complete".into())
}
fn settle(game: &mut GameState, dm: &mut Choices) {
    for _ in 0..20 {
        put_triggers_on_stack_with_dm(game, &mut TriggerQueue::new(), dm).unwrap();
        if game.stack.is_empty() { return; }
        resolve_stack_entry_with(game, dm).unwrap();
    }
    panic!("resolution did not settle");
}
fn craft_action(source: ObjectId, definition: &CardDefinition) -> LegalAction {
    LegalAction::ActivateAbility { source, ability_index: craft_index(definition) }
}
fn pay_craft(game: &mut GameState, row: &serde_json::Value, definition: &CardDefinition, source: ObjectId, chosen: Vec<ObjectId>) -> ObjectId {
    let stable = game.object(source).unwrap().stable_id; fund(game);
    let mana_before = game.player(A).unwrap().mana_pool.total();
    let mut choices = Choices { objects: Some(chosen.clone()), ..Default::default() };
    let action_to_take = craft_action(source, definition);
    assert!(compute_legal_actions(game, A).unwrap().contains(&action_to_take), "{}", row["name"]);
    action(game, action_to_take, &mut choices).unwrap();
    let mana_required = if row["card_faces"][0]["name"] == "Waterlogged Hulk" { 4 } else { 6 };
    assert_eq!(game.player(A).unwrap().mana_pool.total(), mana_before - mana_required);
    assert!(game.object(source).is_none()); assert!(chosen.iter().all(|id| game.object(*id).is_none()));
    let exiled = game.find_object_by_stable_id(stable).unwrap();
    assert_eq!(game.object(exiled).unwrap().zone, Zone::Exile);
    let receipt = &game.stack.last().unwrap().tagged_objects[ironsmith_core::tag::SOURCE_COST_PUBLIC_ARRIVAL_TAG];
    assert_eq!(receipt.len(), 1); assert_eq!(receipt[0].object_id, exiled); exiled
}
#[test]
fn frozen_identities_both_faces_and_exact_material_programs_survive_artifact_transport() {
    let rows = rows();
    assert_eq!(rows.iter().map(|r| r["oracle_id"].as_str().unwrap()).collect::<Vec<_>>(), [
        "1ac3e4bc-1678-4280-a071-3dbc8ef4a2bf", "7cad43df-31c9-47b5-bc05-4a0f6544b396", "30820f71-9fd4-453c-b377-c2e0b01c07a7"]);
    for row in rows { for route in 0..2 {
        let definition = linked(&mut game(), &row, route);
        let AbilityKind::Activated(activated) = &definition.abilities[craft_index(&definition)].kind else { unreachable!() };
        let choose = activated.mana_cost.costs().iter().find_map(|cost|
            cost.effect_ref().and_then(|effect| effect.downcast_ref::<ChooseObjectsEffect>())).unwrap();
        assert_eq!(choose.count, ironsmith::ChoiceCount::exactly(if is_visage(&row) { 2 } else { 1 }));
        assert_eq!(choose.filter.any_of.len(), 2); assert!(choose.filter.any_of.iter().all(|arm| arm.other));
        assert_eq!(choose.zone, Some(Zone::Battlefield)); assert!(choose.additional_zones.contains(&Zone::Graveyard));
        let [effect] = activated.effects.flattened_default_effects() else { panic!("one return instruction") };
        let movement = effect.downcast_ref::<MoveToZoneEffect>().unwrap();
        assert_eq!(movement.target, ChooseSpec::All(ObjectFilter::exact_tagged(ironsmith_core::tag::SOURCE_COST_PUBLIC_ARRIVAL_TAG)));
        assert!(movement.enters_transformed && movement.transfer_exiled_with_source_links);
    }}
}
#[test]
fn native_payment_uses_distinct_mixed_zone_materials_and_returns_under_owner_control() {
    for row in rows() { for route in 0..2 { for graveyard_only in [false, true] {
        let mut game = game(); let definition = linked(&mut game, &row, route);
        let source = game.create_object_from_definition(&definition, B, Zone::Battlefield);
        game.set_current_controller(source, A).unwrap(); let stable = game.object(source).unwrap().stable_id;
        let first = material(&mut game, &row, if graveyard_only { A } else { B }, if graveyard_only { Zone::Graveyard } else { Zone::Battlefield });
        if !graveyard_only { game.set_current_controller(first, A).unwrap(); }
        let mut chosen = vec![first];
        if is_visage(&row) { chosen.push(material(&mut game, &row, A, Zone::Graveyard)); }
        let excluded = material(&mut game, &row, B, Zone::Graveyard);
        let exiled = pay_craft(&mut game, &row, &definition, source, chosen);
        assert_eq!(game.object(excluded).unwrap().zone, Zone::Graveyard);
        resolve_stack_entry_with(&mut game, &mut Choices::default()).unwrap(); assert!(game.object(exiled).is_none());
        let returned = game.find_object_by_stable_id(stable).unwrap();
        assert_eq!(game.object(returned).unwrap().zone, Zone::Battlefield); assert_eq!(game.current_controller(returned), Some(B));
        assert_eq!(game.object(returned).unwrap().name.as_ref(), row["card_faces"][1]["name"].as_str().unwrap());
    }}}
}
#[test]
fn legality_excludes_source_wrong_controller_owner_zone_type_and_wrong_timing() {
    for row in rows() { for route in 0..2 {
        let mut game = game(); let definition = linked(&mut game, &row, route);
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield); fund(&mut game);
        if is_visage(&row) { game.object_mut(source).unwrap().card_types.push(CardType::Creature); }
        else { game.object_mut(source).unwrap().subtypes.push(if row["card_faces"][0]["name"] == "Waterlogged Hulk" {
            ironsmith::types::Subtype::Island } else { ironsmith::types::Subtype::Cave }); }
        material(&mut game, &row, B, Zone::Battlefield); material(&mut game, &row, B, Zone::Graveyard);
        material(&mut game, &row, A, Zone::Hand); printed(&mut game, A, Zone::Graveyard, "Wrong material", "Type: Sorcery");
        assert!(!compute_legal_actions(&game, A).unwrap().contains(&craft_action(source, &definition)));
        material(&mut game, &row, A, Zone::Graveyard);
        if is_visage(&row) {
            assert!(!compute_legal_actions(&game, A).unwrap().contains(&craft_action(source, &definition)));
            material(&mut game, &row, A, Zone::Battlefield);
        }
        assert!(compute_legal_actions(&game, A).unwrap().contains(&craft_action(source, &definition)));
        game.turn.phase = Phase::Combat;
        assert!(!compute_legal_actions(&game, A).unwrap().contains(&craft_action(source, &definition)));
    }}
}
#[test]
fn return_follows_the_paid_public_successor_but_not_prevention_hidden_or_later_moves() {
    // 0: ordinary arrival; 1: redirected to graveyard; 2: prevented;
    // 3: redirected to hand; 4: separately leaves and returns to exile;
    // 5: an addition moves the original arrival before stack admission.
    for row in rows() { for route in 0..2 { for mode in 0..6 {
        let mut game = game(); let definition = linked(&mut game, &row, route);
        let source = game.create_object_from_definition(&definition, B, Zone::Battlefield);
        game.set_current_controller(source, A).unwrap();
        let stable = game.object(source).unwrap().stable_id;
        let mut chosen = vec![material(&mut game, &row, A, Zone::Graveyard)];
        if is_visage(&row) { chosen.push(material(&mut game, &row, A, Zone::Battlefield)); }
        if (1..=3).contains(&mode) {
            let mut replacement = ironsmith::replacement::ZoneReplacementSpec::new(ObjectFilter::specific(source),
                if mode == 3 { Zone::Hand } else { Zone::Graveyard })
                .from_zone(Zone::Battlefield).to_zone(Zone::Exile).build(source, A);
            if mode == 2 { replacement.replacement = ironsmith::replacement::ReplacementAction::Prevent; }
            game.effect_store.replacement_effects.add_one_shot_effect(replacement);
        }
        if mode == 5 {
            use ironsmith::replacement::{ReplacementAction, ReplacementEffect};
            game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(source, A,
                ironsmith::events::zones::matchers::WouldChangeZoneMatcher::new(ObjectFilter::specific(source), Some(Zone::Battlefield), Some(Zone::Exile)),
                ReplacementAction::Additionally(vec![ironsmith::Effect::move_to_zone(ChooseSpec::tagged("it"), Zone::Graveyard, false)])));
        }
        fund(&mut game); let mana_before = game.player(A).unwrap().mana_pool.total();
        action(&mut game, craft_action(source, &definition), &mut Choices { objects: Some(chosen), ..Default::default() }).unwrap();
        assert_eq!(game.player(A).unwrap().mana_pool.total(), mana_before - if row["card_faces"][0]["name"] == "Waterlogged Hulk" { 4 } else { 6 });
        assert_eq!(game.stack.len(), 1, "a modified legal cost is still paid");
        let receipt = game.stack.last().unwrap().tagged_objects[ironsmith_core::tag::SOURCE_COST_PUBLIC_ARRIVAL_TAG].clone();
        if mode == 2 || mode == 3 { assert!(receipt.is_empty()); }
        else { assert_eq!(receipt.len(), 1); assert_ne!(receipt[0].object_id, source); }
        if mode == 1 { assert_eq!(receipt[0].zone, Zone::Graveyard); }
        if mode == 4 {
            let original = game.find_object_by_stable_id(stable).unwrap();
            let graveyard = game.move_object_by_effect(original, Zone::Graveyard).unwrap();
            game.move_object_by_effect(graveyard, Zone::Exile).unwrap();
        }
        let retained = game.find_object_by_stable_id(stable).unwrap(); let expected = game.object(retained).unwrap().zone;
        if mode == 5 {
            assert_eq!(expected, Zone::Graveyard);
            assert_eq!(receipt[0].zone, Zone::Exile); assert_ne!(receipt[0].object_id, retained);
        }
        resolve_stack_entry_with(&mut game, &mut Choices::default()).unwrap();
        let result = game.find_object_by_stable_id(stable).unwrap();
        if mode <= 1 {
            assert_ne!(result, retained); assert_eq!(game.object(result).unwrap().zone, Zone::Battlefield);
            assert_eq!(game.current_controller(result), Some(B));
            assert_eq!(game.object(result).unwrap().name.as_ref(), row["card_faces"][1]["name"].as_str().unwrap());
        } else {
            assert_eq!(result, retained); assert_eq!(game.object(result).unwrap().zone, expected);
            assert_eq!(game.object(result).unwrap().name.as_ref(), row["card_faces"][0]["name"].as_str().unwrap());
        }
    }}}
}
fn material_cost_pair(definition: &CardDefinition) -> (&ironsmith::costs::Cost, &ironsmith::costs::Cost) {
    let AbilityKind::Activated(activated) = &definition.abilities[craft_index(definition)].kind else { unreachable!() };
    let costs = activated.mana_cost.costs();
    let index = costs.iter().position(|cost| cost.effect_ref().is_some_and(|effect| effect.downcast_ref::<ChooseObjectsEffect>().is_some())).unwrap();
    (&costs[index], &costs[index + 1])
}
#[test]
fn material_choice_rejects_duplicates_wrong_objects_and_short_or_oversized_answers() {
    let row = rows().remove(1);
    for route in 0..2 { for invalid in 0..5 {
        let mut game = game(); let definition = linked(&mut game, &row, route);
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let first = material(&mut game, &row, A, Zone::Battlefield); let second = material(&mut game, &row, A, Zone::Graveyard);
        let third = material(&mut game, &row, A, Zone::Graveyard); let enemy = material(&mut game, &row, B, Zone::Graveyard);
        let selection = match invalid { 0 => vec![first, first], 1 => vec![first, enemy], 2 => vec![first], 3 => vec![first, second, third], _ => vec![first, source] };
        let mut dm = Choices { objects: Some(selection), ..Default::default() };
        let mut ctx = CostContext::new(source, A, &mut dm).with_reason(PaymentReason::ActivateAbility);
        assert!(material_cost_pair(&definition).0.pay(&mut game, &mut ctx).is_err());
        assert!(ctx.tagged_objects.is_empty() && game.exile.is_empty());
        assert!(game.object(source).is_some() && game.object(first).is_some() && game.object(second).is_some());
    }}
}
#[test]
fn pending_selection_preserves_the_step_and_replays_without_duplicate_tags() {
    let row = rows().remove(1);
    for route in 0..2 {
        let mut game = game(); let definition = linked(&mut game, &row, route);
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let first = material(&mut game, &row, A, Zone::Battlefield); let second = material(&mut game, &row, A, Zone::Graveyard);
        let (choose, exile) = material_cost_pair(&definition);
        let tag = choose.effect_ref().unwrap().downcast_ref::<ChooseObjectsEffect>().unwrap().tag.clone();
        let mut pending = Choices { pause: true, ..Default::default() };
        {
            let mut ctx = CostContext::new(source, A, &mut pending).with_reason(PaymentReason::ActivateAbility);
            assert_eq!(choose.pay(&mut game, &mut ctx).unwrap(), CostPaymentResult::Paid);
            assert!(ctx.decision_maker.awaiting_choice() && ctx.tagged_objects.is_empty());
        }
        assert!(game.exile.is_empty() && game.take_pending_trigger_events().is_empty());
        let mut accepted = Choices { objects: Some(vec![second, first]), ..Default::default() };
        let mut ctx = CostContext::new(source, A, &mut accepted).with_reason(PaymentReason::ActivateAbility);
        choose.pay(&mut game, &mut ctx).unwrap(); assert_eq!(ctx.tagged_objects[&tag].len(), 2);
        exile.pay(&mut game, &mut ctx).unwrap(); assert_eq!(game.exile.len(), 2);
        assert!(game.object(first).is_none() && game.object(second).is_none());
    }
}
#[test]
fn stale_material_selection_cannot_consume_a_later_incarnation() {
    let row = rows().remove(1);
    for route in 0..2 {
        let mut game = game(); let definition = linked(&mut game, &row, route);
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let first = material(&mut game, &row, A, Zone::Battlefield); let second = material(&mut game, &row, A, Zone::Graveyard);
        let (choose, exile) = material_cost_pair(&definition);
        let mut dm = Choices { objects: Some(vec![first, second]), ..Default::default() };
        let mut ctx = CostContext::new(source, A, &mut dm).with_reason(PaymentReason::ActivateAbility);
        choose.pay(&mut game, &mut ctx).unwrap();
        let departed = game.move_object_by_effect(second, Zone::Exile).unwrap(); let later = game.move_object_by_effect(departed, Zone::Graveyard).unwrap();
        assert!(exile.pay(&mut game, &mut ctx).is_err());
        assert!(game.object(first).is_some() && game.exile.is_empty()); assert_eq!(game.object(later).unwrap().zone, Zone::Graveyard);
    }
}
fn enter(game: &mut GameState, definition: &CardDefinition, choices: &mut Choices) -> ObjectId {
    let hand = game.create_object_from_definition(definition, A, Zone::Hand); let stable = game.object(hand).unwrap().stable_id;
    let receipt = game.move_object_with_etb_processing_with_dm(hand, Zone::Battlefield, choices).unwrap();
    assert!(!receipt.pending); assert!(receipt.programs.is_empty()); settle(game, choices);
    game.find_object_by_stable_id(stable).unwrap()
}
fn attack(game: &mut GameState, attacker: ObjectId) {
    game.remove_summoning_sickness(attacker); game.mark_combat_phase_started();
    game.turn.phase = Phase::Combat; game.turn.step = Some(ironsmith::game_state::Step::DeclareAttackers);
    let mut queue = TriggerQueue::new(); let mut combat = ironsmith::combat_state::CombatState::default();
    ironsmith::game_loop::apply_attacker_declarations(game, &mut combat, &mut queue,
        &[ironsmith::decision::AttackerDeclaration { creature: attacker,
            target: ironsmith::combat_state::AttackTarget::Player(B) }]).unwrap();
    game.combat = Some(combat); put_triggers_on_stack_with_dm(game, &mut queue, &mut Choices::default()).unwrap();
}
#[test]
fn stonetree_looks_at_six_optionally_puts_one_land_tapped_and_bottoms_only_the_remainder() {
    let row = rows().remove(0);
    for route in 0..2 { for accept in [false, true] {
        let mut game = game(); let definition = linked(&mut game, &row, route);
        let untouched = printed(&mut game, A, Zone::Library, "Below six", "Type: Sorcery");
        for n in 0..5 { printed(&mut game, A, Zone::Library, &format!("Viewed spell {n}"), "Type: Sorcery"); }
        let land = printed(&mut game, A, Zone::Library, "Selected land", "Type: Land — Forest"); let stable = game.object(land).unwrap().stable_id;
        let mut dm = Choices { objects: Some(if accept { vec![land] } else { Vec::new() }), accept, ..Default::default() };
        enter(&mut game, &definition, &mut dm);
        let library = &game.player(A).unwrap().library;
        assert_eq!(library.len(), if accept { 6 } else { 7 }); assert_eq!(library.last(), Some(&untouched));
        let result = game.find_object_by_stable_id(stable).unwrap();
        assert_eq!(game.object(result).unwrap().zone, if accept { Zone::Battlefield } else { Zone::Library });
        if accept { assert!(game.is_tapped(result)); }
        assert!(library.iter().filter(|id| **id != untouched).all(|id|
            game.object(*id).unwrap().name.starts_with("Viewed spell") || game.object(*id).unwrap().name.as_ref() == "Selected land"));
    }}
}
#[test]
fn visage_front_discards_only_the_selected_artifact_or_creature_from_the_opponents_hand() {
    let row = rows().remove(1);
    for route in 0..2 { for selected_type in ["Artifact", "Creature — Bear\nPower/Toughness: 2/2"] {
        let mut game = game(); let definition = linked(&mut game, &row, route);
        let selected = printed(&mut game, B, Zone::Hand, "Chosen hand card", &format!("Type: {selected_type}")); let stable = game.object(selected).unwrap().stable_id;
        let other = printed(&mut game, B, Zone::Hand, "Unchosen spell", "Type: Instant");
        let mine = printed(&mut game, A, Zone::Hand, "My artifact", "Type: Artifact");
        let mut choices = Choices { objects: Some(vec![selected]), ..Default::default() };
        enter(&mut game, &definition, &mut choices);
        assert_eq!(game.object(game.find_object_by_stable_id(stable).unwrap()).unwrap().zone, Zone::Graveyard);
        assert_eq!(game.object(other).unwrap().zone, Zone::Hand); assert_eq!(game.object(mine).unwrap().zone, Zone::Hand);
        assert!(choices.offered.iter().any(|ids| ids.contains(&selected) && !ids.contains(&other) && !ids.contains(&mine)));
    }}
}
#[test]
fn osseosaur_entry_and_attack_each_offer_optional_mill_two_and_retain_menace() {
    let row = rows().remove(1);
    for route in 0..2 { for accept in [false, true] {
        let mut game = game(); let definition = linked(&mut game, &row, route);
        for index in 0..6 { printed(&mut game, A, Zone::Library, &format!("Mill card {index}"), "Type: Sorcery"); }
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield); let stable = game.object(source).unwrap().stable_id;
        let first = material(&mut game, &row, A, Zone::Battlefield); let second = material(&mut game, &row, A, Zone::Graveyard);
        pay_craft(&mut game, &row, &definition, source, vec![first, second]);
        let mut choices = Choices { accept, ..Default::default() }; settle(&mut game, &mut choices);
        assert_eq!(game.player(A).unwrap().library.len(), if accept { 4 } else { 6 });
        let returned = game.find_object_by_stable_id(stable).unwrap();
        assert_eq!((game.current_power(returned), game.current_toughness(returned)), (Some(5), Some(4)));
        assert!(game.current_has_static_ability_id(returned, ironsmith::static_abilities::StaticAbilityId::Menace));
        attack(&mut game, returned); settle(&mut game, &mut choices);
        assert_eq!(game.player(A).unwrap().library.len(), if accept { 2 } else { 6 });
    }}
}
#[test]
fn hulk_mills_then_gondola_crews_with_vigilance_and_live_descend_permanent_count() {
    let row = rows().remove(2);
    for route in 0..2 {
        let mut game = game(); let definition = linked(&mut game, &row, route);
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield); let stable = game.object(source).unwrap().stable_id;
        printed(&mut game, A, Zone::Library, "Milled nonpermanent", "Type: Sorcery");
        let mill_index = definition.abilities.iter().position(|ability| matches!(&ability.kind,
            AbilityKind::Activated(activated) if activated.mana_cost.costs().iter().any(|cost| cost.requires_tap()))).unwrap();
        action(&mut game, LegalAction::ActivateAbility { source, ability_index: mill_index }, &mut Choices::default()).unwrap();
        settle(&mut game, &mut Choices::default()); assert!(game.is_tapped(source)); assert_eq!(game.player(A).unwrap().graveyard.len(), 1);
        let island = material(&mut game, &row, A, Zone::Graveyard);
        pay_craft(&mut game, &row, &definition, source, vec![island]); settle(&mut game, &mut Choices::default());
        let gondola = game.find_object_by_stable_id(stable).unwrap(); assert!(!game.current_is_creature(gondola));
        assert!(game.current_has_static_ability_id(gondola, ironsmith::static_abilities::StaticAbilityId::Vigilance));
        let crew = printed(&mut game, A, Zone::Battlefield, "Pilot", "Type: Creature — Human\nPower/Toughness: 1/1");
        let crew_index = game.current_abilities(gondola).unwrap().iter().position(|ability| matches!(&ability.kind, AbilityKind::Activated(_))).unwrap();
        action(&mut game, LegalAction::ActivateAbility { source: gondola, ability_index: crew_index },
            &mut Choices { objects: Some(vec![crew]), ..Default::default() }).unwrap();
        settle(&mut game, &mut Choices::default()); assert!(game.is_tapped(crew)); assert!(game.current_is_creature(gondola));
        assert_eq!((game.current_power(gondola), game.current_toughness(gondola)), (Some(4), Some(4)));
        for index in 0..7 { printed(&mut game, A, Zone::Graveyard, &format!("Permanent {index}"), "Type: Artifact"); }
        ironsmith::game_loop::check_and_apply_sbas_with(&mut game, &mut TriggerQueue::new(), &mut Choices::default()).unwrap();
        assert!(game.can_be_blocked(gondola), "seven permanents plus a sorcery is insufficient");
        let eighth = printed(&mut game, A, Zone::Graveyard, "Eighth permanent", "Type: Land");
        ironsmith::game_loop::check_and_apply_sbas_with(&mut game, &mut TriggerQueue::new(), &mut Choices::default()).unwrap(); assert!(!game.can_be_blocked(gondola));
        game.move_object_by_effect(eighth, Zone::Exile).unwrap();
        ironsmith::game_loop::check_and_apply_sbas_with(&mut game, &mut TriggerQueue::new(), &mut Choices::default()).unwrap(); assert!(game.can_be_blocked(gondola));
        attack(&mut game, gondola); assert!(!game.is_tapped(gondola), "vigilance applies after crew");
        ironsmith::turn::execute_cleanup_step(&mut game); assert!(!game.current_is_creature(gondola));
    }
}
#[test]
fn failed_native_material_payment_rolls_back_mana_source_materials_events_and_replacement() {
    use ironsmith::replacement::{ReplacementAction, ReplacementEffect};
    let row = rows().remove(1);
    for route in 0..2 {
        let mut game = game(); let definition = linked(&mut game, &row, route);
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let first = material(&mut game, &row, A, Zone::Battlefield); let second = material(&mut game, &row, A, Zone::Graveyard);
        let replacement = game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(
            source, A, ironsmith::events::zones::matchers::WouldChangeZoneMatcher::new(
                ObjectFilter::specific(second), Some(Zone::Graveyard), Some(Zone::Exile)),
            ReplacementAction::Additionally(vec![ironsmith::Effect::gain_life(2), ironsmith::Effect::lose_life(ironsmith::Value::X)])));
        fund(&mut game); let pool = game.player(A).unwrap().mana_pool.clone(); game.take_pending_trigger_events();
        let mut dm = Choices { objects: Some(vec![first, second]), ..Default::default() };
        assert!(action(&mut game, craft_action(source, &definition), &mut dm).is_err());
        assert_eq!(game.player(A).unwrap().mana_pool, pool); assert_eq!(game.player(A).unwrap().life, 20);
        assert_eq!(game.object(source).unwrap().zone, Zone::Battlefield); assert_eq!(game.object(first).unwrap().zone, Zone::Battlefield);
        assert_eq!(game.object(second).unwrap().zone, Zone::Graveyard); assert!(game.stack.is_empty() && game.exile.is_empty());
        assert!(game.take_pending_trigger_events().is_empty()); assert!(game.effect_store.replacement_effects.get_effect(replacement).is_some());
    }
}
#[test]
fn pending_material_replacement_restores_the_exile_instruction_and_retries_once() {
    use ironsmith::replacement::{ReplacementAction, ReplacementEffect};
    let row = rows().remove(1);
    for route in 0..2 {
        let mut game = game(); let definition = linked(&mut game, &row, route);
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let first = material(&mut game, &row, A, Zone::Battlefield); let second = material(&mut game, &row, A, Zone::Graveyard);
        let (choose, exile) = material_cost_pair(&definition);
        let mut dm = Choices { objects: Some(vec![first, second]), ..Default::default() };
        let tags = {
            let mut ctx = CostContext::new(source, A, &mut dm).with_reason(PaymentReason::ActivateAbility);
            choose.pay(&mut game, &mut ctx).unwrap(); ctx.tagged_objects
        };
        let replacement = game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(
            source, A, ironsmith::events::zones::matchers::WouldChangeZoneMatcher::new(
                ObjectFilter::specific(second), Some(Zone::Graveyard), Some(Zone::Exile)),
            ReplacementAction::Additionally(vec![ironsmith::Effect::gain_life(2), ironsmith::Effect::may(vec![ironsmith::Effect::gain_life(3)])])));
        let mut pending = Choices { pause: true, ..Default::default() };
        {
            let mut ctx = CostContext::new(source, A, &mut pending).with_reason(PaymentReason::ActivateAbility);
            ctx.tagged_objects = tags.clone(); exile.pay(&mut game, &mut ctx).unwrap();
            assert!(ctx.decision_maker.awaiting_choice()); assert_eq!(ctx.tagged_objects, tags);
        }
        assert!(game.object(first).is_some() && game.object(second).is_some() && game.exile.is_empty());
        assert_eq!(game.player(A).unwrap().life, 20); assert!(game.take_pending_trigger_events().is_empty());
        assert!(game.effect_store.replacement_effects.get_effect(replacement).is_some());
        let mut accepted = Choices { accept: true, ..Default::default() };
        let mut ctx = CostContext::new(source, A, &mut accepted).with_reason(PaymentReason::ActivateAbility);
        ctx.tagged_objects = tags; exile.pay(&mut game, &mut ctx).unwrap();
        assert_eq!(game.exile.len(), 2); assert_eq!(game.player(A).unwrap().life, 25);
        assert!(game.effect_store.replacement_effects.get_effect(replacement).is_none());
    }
}
#[test]
fn transformed_reentry_uses_back_face_entry_replacements_without_a_transform_event() {
    let mut row = rows().remove(0);
    row["card_faces"][1]["text"] = serde_json::Value::String(
        "Type: Artifact Creature — Golem\nPower/Toughness: 5/5\nThis creature enters tapped.".into());
    for route in 0..2 {
        let mut game = game(); let definition = linked(&mut game, &row, route);
        printed(&mut game, A, Zone::Battlefield, "Transformation observer",
            "Type: Enchantment\nWhenever a permanent you control transforms into a non-Human creature, create a 2/2 green Wolf creature token.");
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield); let stable = game.object(source).unwrap().stable_id;
        let cave = material(&mut game, &row, A, Zone::Graveyard); let material_stable = game.object(cave).unwrap().stable_id;
        pay_craft(&mut game, &row, &definition, source, vec![cave]); settle(&mut game, &mut Choices::default());
        let returned = game.find_object_by_stable_id(stable).unwrap(); assert!(game.is_tapped(returned));
        assert_eq!((game.current_power(returned), game.current_toughness(returned)), (Some(5), Some(5)));
        assert!(!game.battlefield.iter().any(|id| game.object(*id).unwrap().name.as_ref() == "Wolf"));
        let material = game.find_object_by_stable_id(material_stable).unwrap(); assert!(game.get_exiled_with_source_links(returned).contains(&material));
    }
}
#[test]
fn receipt_absence_is_incomplete_but_known_empty_and_copied_receipts_keep_their_meaning() {
    use ironsmith::effects::{CopySpellEffect, EffectContext, EffectExecutor, ExecutionError};
    let row = rows().remove(0);
    for route in 0..2 {
        let mut game = game(); let definition = linked(&mut game, &row, route);
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let stable = game.object(source).unwrap().stable_id;
        let cave = material(&mut game, &row, A, Zone::Graveyard);
        let arrival = pay_craft(&mut game, &row, &definition, source, vec![cave]);
        let paid_state = game.clone();
        let tag = ironsmith_core::tag::SOURCE_COST_PUBLIC_ARRIVAL_TAG;
        let receipt = game.stack.last().unwrap().tagged_objects[tag].clone();
        game.stack.last_mut().unwrap().tagged_objects.remove(tag);
        assert!(matches!(resolve_stack_entry_with(&mut game, &mut Choices::default()),
            Err(ironsmith::game_loop::GameLoopError::ExecutionFailed(ExecutionError::IncompleteEvidence(_)))));
        assert_eq!(game.find_object_by_stable_id(stable), Some(arrival));

        // A native state restore retains absence versus a known completed zero.
        game = paid_state.clone();
        game.stack.last_mut().unwrap().tagged_objects.insert(tag.into(), Vec::new());
        resolve_stack_entry_with(&mut game, &mut Choices::default()).unwrap();
        assert_eq!(game.find_object_by_stable_id(stable), Some(arrival));
        assert_eq!(game.object(arrival).unwrap().zone, Zone::Exile);

        game = paid_state;
        let ability_id = game.stack.last().unwrap().ability_id.unwrap();
        let mut choices = Choices::default();
        CopySpellEffect::single(ChooseSpec::SpecificObject(ability_id))
            .execute(&mut game, &mut EffectContext::new(arrival, A, &mut choices)).unwrap();
        assert_eq!(game.stack.len(), 2);
        assert!(game.stack.iter().all(|entry| entry.tagged_objects[tag] == receipt));
        let copied_state = game.clone();
        for mut restored in [game, copied_state] {
            resolve_stack_entry_with(&mut restored, &mut Choices::default()).unwrap();
            let returned = restored.find_object_by_stable_id(stable).unwrap();
            assert_ne!(returned, arrival); assert_eq!(restored.object(returned).unwrap().zone, Zone::Battlefield);
            assert_eq!(restored.stack.len(), 1);
            resolve_stack_entry_with(&mut restored, &mut Choices::default()).unwrap();
            assert_eq!(restored.find_object_by_stable_id(stable), Some(returned));
            assert_eq!(restored.current_power(returned), Some(5)); assert!(restored.stack.is_empty());
        }
    }
}
