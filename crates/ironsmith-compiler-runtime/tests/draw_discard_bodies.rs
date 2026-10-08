//! Exact frozen complete bodies; source-authored only. All scenarios UNRUN.
use ironsmith::ability::AbilityKind;
use ironsmith::card::{CardBuilder, PowerToughness};
use ironsmith::cards::CardDefinition;
use ironsmith::color::ColorSet;
use ironsmith::decision::{DecisionMaker, LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
use ironsmith::decisions::context::{SelectObjectsContext, SelectOptionsContext, TargetsContext};
use ironsmith::effects::{EffectContext, execute_effect};
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, check_and_apply_sbas, put_triggers_on_stack_with_dm,
    resolve_stack_entry_with};
use ironsmith::game_state::Phase;
use ironsmith::ids::StableId;
use ironsmith::mana::ManaSymbol;
use ironsmith::object::AttachmentTarget;
use ironsmith::target::{ChooseSpec, ObjectFilter};
use ironsmith::triggers::TriggerQueue;
use ironsmith::{CardId, CardType, Effect, GameProgress, GameState, ObjectId, PlayerId, Subtype, Target, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};

const A: PlayerId = PlayerId::from_index(0);
const B: PlayerId = PlayerId::from_index(1);
fn definitions(name: &str) -> [CardDefinition; 2] {
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!("../../../fixtures/draw_discard_bodies.json.fixture")).unwrap();
    let row = rows.iter().find(|row| row["name"] == name).unwrap();
    assert_eq!(row["oracle_id"], if name == "Soldevi Sage" {
        "1f612df3-53b6-4317-9d63-1f903ee3f0c4"
    } else { "5a747256-4215-4334-98ab-0c2e4ed92e47" });
    let mut text = format!("Mana cost: {}\nType: {}\n", row["mana_cost"].as_str().unwrap(), row["type_line"].as_str().unwrap());
    if let (Some(power), Some(toughness)) = (row["power"].as_str(), row["toughness"].as_str()) {
        text.push_str(&format!("Power/Toughness: {power}/{toughness}\n"));
    }
    text.push_str(row["oracle_text"].as_str().unwrap());
    // Independently invoke both public compiler routes; never use the direct
    // definition returned as a side product of artifact compilation.
    let (direct, loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_runtime_definition(name, &text, false));
    let direct = direct.unwrap(); assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let (artifact, loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_artifact(name, &text, false));
    let (artifact, _) = artifact.unwrap(); assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    artifact.validate().unwrap();
    let decoded = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    decoded.validate().unwrap(); assert_eq!(artifact, decoded);
    let definitions = [direct, ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&decoded).unwrap()];
    for definition in &definitions {
        assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(definition));
        assert_eq!(definition.card.name, name);
        assert_eq!(definition.card.mana_cost.as_ref().unwrap().mana_value(), if name == "Soldevi Sage" { 2 } else { 3 });
        assert_eq!(definition.card.colors(), if name == "Soldevi Sage" { ColorSet::BLUE } else { ColorSet::BLACK });
        if name == "Soldevi Sage" {
            assert_eq!(definition.card.card_types, vec![CardType::Creature]);
            assert_eq!(definition.card.subtypes, vec![Subtype::Human, Subtype::Wizard]);
            assert_eq!(definition.card.power_toughness, Some(PowerToughness::fixed(1, 1)));
            assert_eq!(definition.abilities.iter().filter(|ability| matches!(ability.kind, AbilityKind::Activated(_))).count(), 1);
        } else {
            assert_eq!(definition.card.card_types, vec![CardType::Enchantment]);
            assert_eq!(definition.card.subtypes, vec![Subtype::Aura]);
            assert!(definition.aura_attach_filter.is_some());
            assert_eq!(definition.abilities.iter().filter(|ability| matches!(ability.kind, AbilityKind::Triggered(_))).count(), 1);
        }
    }
    definitions
}
fn game() -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.active_player = A; game.turn.priority_player = Some(A);
    game.turn.phase = Phase::FirstMain; game.turn.step = None; game
}
fn card(game: &mut GameState, owner: PlayerId, zone: Zone, kind: CardType, name: &str) -> ObjectId {
    game.create_object_from_card(&CardBuilder::new(CardId::new(), name).card_types(vec![kind])
        .power_toughness(PowerToughness::fixed(2, 2)).build(), owner, zone)
}
fn library(game: &mut GameState, player: PlayerId, count: usize) {
    for index in 0..count { card(game, player, Zone::Library, CardType::Land, &format!("Draw candidate {index}")); }
}
fn expected_draws(game: &GameState, player: PlayerId, count: usize) -> Vec<StableId> {
    game.player(player).unwrap().library.iter().rev().take(count)
        .map(|id| game.object(*id).unwrap().stable_id).collect()
}
#[derive(Default)]
struct Choices {
    target: Option<Target>, forbidden_targets: Vec<Target>, lands: Vec<ObjectId>,
    player: Option<PlayerId>, expected: Vec<StableId>, before_draw_ids: Vec<ObjectId>, pick: usize, discard_calls: usize,
    pause: bool, pending: bool, skip_draws: usize, replacement_calls: usize,
}
impl DecisionMaker for Choices {
    fn decide_targets(&mut self, _: &GameState, ctx: &TargetsContext) -> Vec<Target> {
        let target = self.target.expect("only the Aura cast declares a target");
        assert_eq!(ctx.requirements.len(), 1);
        assert!(ctx.requirements[0].legal_targets.contains(&target));
        for forbidden in &self.forbidden_targets { assert!(!ctx.requirements[0].legal_targets.contains(forbidden)); }
        vec![target]
    }
    fn decide_objects(&mut self, game: &GameState, ctx: &SelectObjectsContext) -> Vec<ObjectId> {
        if !self.lands.is_empty() && self.lands.iter().all(|id| ctx.candidates.iter().any(|c| c.legal && c.id == *id)) {
            assert_eq!(ctx.min, 2); assert_eq!(ctx.player, A);
            return std::mem::take(&mut self.lands);
        }
        let legal: Vec<_> = ctx.candidates.iter().filter(|candidate| candidate.legal).map(|candidate| candidate.id).collect();
        assert_eq!(ctx.player, self.player.unwrap_or(A));
        assert_eq!(ctx.min, 1); assert_eq!(ctx.max, Some(1)); assert_eq!(legal.len(), self.expected.len());
        for id in &legal {
            let object = game.object(*id).unwrap();
            assert_eq!(object.zone, Zone::Hand);
            assert!(!self.before_draw_ids.contains(id), "draw tags must use the new hand incarnation");
            assert!(self.expected.contains(&object.stable_id), "an unrelated hand card leaked into the drawn set");
        }
        self.discard_calls += 1;
        if self.pause { self.pending = true; return vec![]; }
        let chosen = legal.into_iter().find(|id| game.object(*id).unwrap().stable_id == self.expected[self.pick]).unwrap();
        vec![chosen]
    }
    fn decide_options(&mut self, _: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
        let skip = self.replacement_calls < self.skip_draws;
        self.replacement_calls += 1;
        vec![ctx.options.iter().find(|option| option.legal
            && option.description.starts_with("Do not apply") == !skip).expect("optional draw replacement").index]
    }
    fn awaiting_choice(&self) -> bool { self.pending }
}
fn announce(game: &mut GameState, action: LegalAction, dm: &mut Choices) {
    assert!(compute_legal_actions(game, A).unwrap().contains(&action));
    let mut queue = TriggerQueue::new(); let mut state = PriorityLoopState::new(2);
    let mut progress = apply_priority_response_with_dm(game, &mut queue, &mut state, &PriorityResponse::PriorityAction(action), dm).unwrap();
    for _ in 0..64 {
        if !state.has_pending_action() { return; }
        let GameProgress::NeedsDecisionCtx(ctx) = progress else { panic!("unfinished announcement: {progress:?}"); };
        *game = game.clone(); state = state.clone();
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &ctx, dm).unwrap();
    }
    panic!("announcement did not finish");
}
fn sage_action(definition: &CardDefinition, source: ObjectId) -> LegalAction {
    LegalAction::ActivateAbility { source, ability_index: definition.abilities.iter().position(|ability|
        matches!(ability.kind, AbilityKind::Activated(_))).unwrap() }
}
fn pay_sage(game: &mut GameState, definition: &CardDefinition, dm: &mut Choices) -> ObjectId {
    dm.before_draw_ids = game.player(A).unwrap().library.to_vec();
    let source = game.create_object_from_definition(definition, B, Zone::Battlefield);
    game.set_current_controller(source, A).unwrap(); game.remove_summoning_sickness(source);
    let own = card(game, A, Zone::Battlefield, CardType::Land, "Owned land payment");
    let borrowed = card(game, B, Zone::Battlefield, CardType::Land, "Borrowed land payment");
    game.set_current_controller(borrowed, A).unwrap();
    let own_stable = game.object(own).unwrap().stable_id;
    let borrowed_stable = game.object(borrowed).unwrap().stable_id;
    dm.lands = vec![own, borrowed];
    announce(game, sage_action(definition, source), dm);
    assert!(game.is_tapped(source)); assert_eq!(game.stack.len(), 1);
    assert!(game.stack.last().unwrap().targets.is_empty());
    assert!(game.stack.last().unwrap().target_assignments.is_empty());
    let own_grave = game.find_object_by_stable_id(own_stable).unwrap();
    let borrowed_grave = game.find_object_by_stable_id(borrowed_stable).unwrap();
    assert!(game.player(A).unwrap().graveyard.contains(&own_grave));
    assert!(game.player(B).unwrap().graveyard.contains(&borrowed_grave));
    source
}
fn resolve_and_check(game: &mut GameState, dm: &mut Choices, player: PlayerId, old: ObjectId, draw_count: usize) {
    resolve_stack_entry_with(game, dm).unwrap(); assert!(game.stack.is_empty());
    assert_eq!(game.object(old).unwrap().zone, Zone::Hand);
    assert_eq!(game.player(player).unwrap().hand.len(), 1 + draw_count.saturating_sub(1));
    assert_eq!(game.turn_store.turn_history.cards_drawn_by_player(player), draw_count as u32);
    if draw_count > 0 {
        if draw_count > 1 { assert_eq!(dm.discard_calls, 1); }
        else { assert!(dm.discard_calls <= 1, "a singleton can be auto-selected"); }
        for (index, stable) in dm.expected.iter().enumerate() {
            let now = game.find_object_by_stable_id(*stable).unwrap();
            assert_eq!(game.object(now).unwrap().zone, if index == dm.pick { Zone::Graveyard } else { Zone::Hand });
        }
    } else { assert_eq!(dm.discard_calls, 0, "empty draw receipt must not fall back to the old hand"); }
}
#[test]
fn sage_full_body_pays_two_distinct_controlled_lands_and_selects_each_of_three_drawn_cards() {
    for definition in definitions("Soldevi Sage") { for pick in 0..3 { for departed in [false, true] {
        let mut game = game(); library(&mut game, A, 5); library(&mut game, B, 3);
        let old = card(&mut game, A, Zone::Hand, CardType::Instant, "Unrelated old hand card");
        let foreign = card(&mut game, B, Zone::Hand, CardType::Instant, "Opponent old card");
        let mut dm = Choices { expected: expected_draws(&game, A, 3), pick, ..Default::default() };
        let source = pay_sage(&mut game, &definition, &mut dm);
        if departed { game.move_object_by_effect(source, Zone::Exile).unwrap(); }
        else { game.set_current_controller(source, B).unwrap(); }
        resolve_and_check(&mut game, &mut dm, A, old, 3);
        assert_eq!(game.player(A).unwrap().library.len(), 2);
        assert_eq!(game.player(B).unwrap().library.len(), 3);
        assert_eq!(game.object(foreign).unwrap().zone, Zone::Hand);
    } } }
}
#[test]
fn sage_insufficient_lands_and_summoning_sickness_cannot_announce_or_pay() {
    for definition in definitions("Soldevi Sage") {
        let mut game = game(); let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let first = card(&mut game, A, Zone::Battlefield, CardType::Land, "First own land");
        let foreign = card(&mut game, B, Zone::Battlefield, CardType::Land, "Uncontrolled land");
        let outside = card(&mut game, A, Zone::Hand, CardType::Land, "Land in hand");
        let nonland = card(&mut game, A, Zone::Battlefield, CardType::Artifact, "Nonland");
        game.remove_summoning_sickness(source);
        assert!(!compute_legal_actions(&game, A).unwrap().contains(&sage_action(&definition, source)));
        assert!(!game.is_tapped(source)); assert!(game.stack.is_empty());
        for id in [first, foreign, nonland] { assert_eq!(game.object(id).unwrap().zone, Zone::Battlefield); }
        assert_eq!(game.object(outside).unwrap().zone, Zone::Hand);
        let second = card(&mut game, A, Zone::Battlefield, CardType::Land, "Second own land");
        let sick = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        assert!(!compute_legal_actions(&game, A).unwrap().contains(&sage_action(&definition, sick)));
        assert_eq!(game.object(second).unwrap().zone, Zone::Battlefield);
    }
}
#[test]
fn sage_empty_and_short_libraries_never_use_unrelated_hand_cards() {
    for definition in definitions("Soldevi Sage") { for available in 0..3 {
        let mut game = game(); library(&mut game, A, available);
        let old = card(&mut game, A, Zone::Hand, CardType::Instant, "Old hand card");
        let mut dm = Choices { expected: expected_draws(&game, A, available), ..Default::default() };
        pay_sage(&mut game, &definition, &mut dm);
        resolve_and_check(&mut game, &mut dm, A, old, available);
        assert!(game.player(A).unwrap().library.is_empty());
    } }
}
#[test]
fn sage_optional_draw_replacement_binds_only_actual_draws_including_zero() {
    for definition in definitions("Soldevi Sage") { for skipped in 0..=3 {
        let mut game = game(); library(&mut game, A, 5);
        // Native replacement-owner control; no admission claim for this helper.
        let replacement = compile_to_runtime_definition("Optional draw control",
            "Type: Enchantment\nIf you would draw a card, you may skip that draw instead.", false).unwrap();
        game.create_object_from_definition(&replacement, A, Zone::Battlefield);
        let old = card(&mut game, A, Zone::Hand, CardType::Instant, "Old hand card");
        let mut dm = Choices { expected: expected_draws(&game, A, 3 - skipped), skip_draws: skipped, ..Default::default() };
        pay_sage(&mut game, &definition, &mut dm);
        resolve_and_check(&mut game, &mut dm, A, old, 3 - skipped);
        assert_eq!(dm.replacement_calls, 3); assert_eq!(game.player(A).unwrap().library.len(), 2 + skipped);
    } }
}
#[test]
fn sage_pending_discard_rolls_back_resolution_but_preserves_paid_costs_and_retries_once() {
    for definition in definitions("Soldevi Sage") {
        let mut game = game(); library(&mut game, A, 5);
        let old = card(&mut game, A, Zone::Hand, CardType::Instant, "Old hand card");
        let mut dm = Choices { expected: expected_draws(&game, A, 3), pause: true, ..Default::default() };
        let source = pay_sage(&mut game, &definition, &mut dm);
        let library_before = game.player(A).unwrap().library.to_vec();
        let graves_before = [game.player(A).unwrap().graveyard.clone(), game.player(B).unwrap().graveyard.clone()];
        resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert!(dm.pending); assert_eq!(game.stack.len(), 1); assert!(game.is_tapped(source));
        assert_eq!(game.player(A).unwrap().hand, vec![old]); assert_eq!(game.player(A).unwrap().library, library_before);
        assert_eq!(game.turn_store.turn_history.cards_drawn_by_player(A), 0);
        assert_eq!(game.player(A).unwrap().graveyard, graves_before[0]); assert_eq!(game.player(B).unwrap().graveyard, graves_before[1]);
        dm.pause = false; dm.pending = false; dm.discard_calls = 0; game = game.clone();
        resolve_and_check(&mut game, &mut dm, A, old, 3);
        assert_eq!(game.player(B).unwrap().graveyard, graves_before[1]);
    }
}
fn cast_aura(game: &mut GameState, definition: &CardDefinition, host: ObjectId) -> ObjectId {
    game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Black, 1);
    game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Colorless, 2);
    let noncreature = card(game, A, Zone::Battlefield, CardType::Artifact, "Illegal Aura target");
    let aura = game.create_object_from_definition(definition, A, Zone::Hand);
    let stable = game.object(aura).unwrap().stable_id;
    let action = compute_legal_actions(game, A).unwrap().into_iter().find(|action|
        matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == aura)).unwrap();
    let mut dm = Choices { target: Some(Target::Object(host)), forbidden_targets: vec![Target::Object(noncreature), Target::Player(A)], ..Default::default() };
    announce(game, action, &mut dm);
    assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
    assert_eq!(game.stack.last().unwrap().targets, vec![Target::Object(host)]);
    resolve_stack_entry_with(game, &mut dm).unwrap();
    let aura = game.find_object_by_stable_id(stable).unwrap();
    assert_eq!(game.object(aura).unwrap().attached_to, Some(AttachmentTarget::Object(host))); aura
}
fn destroy_and_stack(game: &mut GameState, source: ObjectId, objects: &[ObjectId]) {
    let filter = ObjectFilter { any_of: objects.iter().map(|id| ObjectFilter::specific(*id)).collect(), ..ObjectFilter::default() };
    let outcome = execute_effect(game, &Effect::destroy_all(filter), &mut EffectContext::new(source, A, &mut SelectFirstDecisionMaker)).unwrap();
    for event in outcome.events { game.queue_trigger_event(Default::default(), event); }
    let mut queue = TriggerQueue::new(); check_and_apply_sbas(game, &mut queue).unwrap();
    put_triggers_on_stack_with_dm(game, &mut queue, &mut SelectFirstDecisionMaker).unwrap();
}
#[test]
fn bones_real_aura_cast_and_death_use_its_controller_and_survive_simultaneous_source_departure() {
    for definition in definitions("Casting of Bones") { for controller in [A, B] { for simultaneous in [false, true] {
        let mut game = game(); let host = card(&mut game, B, Zone::Battlefield, CardType::Creature, "Enchanted creature");
        let aura = cast_aura(&mut game, &definition, host);
        game.set_current_controller(aura, controller).unwrap();
        library(&mut game, controller, 3);
        let old = card(&mut game, controller, Zone::Hand, CardType::Instant, "Unrelated old hand card");
        let mut dm = Choices { player: Some(controller), before_draw_ids: game.player(controller).unwrap().library.to_vec(), expected: expected_draws(&game, controller, 3), pick: 1, ..Default::default() };
        game.take_pending_trigger_events();
        let unrelated = card(&mut game, A, Zone::Battlefield, CardType::Creature, "Unenchanted creature");
        destroy_and_stack(&mut game, aura, &[unrelated]); assert!(game.stack.is_empty());
        let aura_stable = game.object(aura).unwrap().stable_id;
        destroy_and_stack(&mut game, aura, &if simultaneous { vec![host, aura] } else { vec![host] });
        assert_eq!(game.stack.len(), 1); assert!(game.stack.last().unwrap().targets.is_empty());
        assert_eq!(game.stack.last().unwrap().controller, controller);
        let departed = game.find_object_by_stable_id(aura_stable).unwrap();
        assert_eq!(game.object(departed).unwrap().zone, Zone::Graveyard);
        game.move_object_by_effect(departed, Zone::Exile).unwrap();
        resolve_and_check(&mut game, &mut dm, controller, old, 3);
    } } }
}
#[test]
fn bones_pending_choice_and_empty_library_keep_the_draw_receipt_after_aura_departure() {
    for definition in definitions("Casting of Bones") { for count in [0, 3] {
        let mut game = game(); let host = card(&mut game, A, Zone::Battlefield, CardType::Creature, "Host");
        let aura = cast_aura(&mut game, &definition, host); library(&mut game, A, count);
        let old = card(&mut game, A, Zone::Hand, CardType::Instant, "Old hand card");
        let mut dm = Choices { expected: expected_draws(&game, A, count), before_draw_ids: game.player(A).unwrap().library.to_vec(), pause: count > 0, ..Default::default() };
        game.take_pending_trigger_events(); destroy_and_stack(&mut game, aura, &[host]);
        if count > 0 {
            resolve_stack_entry_with(&mut game, &mut dm).unwrap(); assert!(dm.pending);
            assert_eq!(game.stack.len(), 1); assert_eq!(game.player(A).unwrap().hand, vec![old]);
            assert_eq!(game.player(A).unwrap().library.len(), count);
            assert_eq!(game.turn_store.turn_history.cards_drawn_by_player(A), 0);
            dm.pause = false; dm.pending = false; dm.discard_calls = 0; game = game.clone();
        }
        resolve_and_check(&mut game, &mut dm, A, old, count);
    } }
}
#[test]
fn bones_does_not_trigger_for_exile_or_after_the_aura_left_before_the_host_died() {
    for definition in definitions("Casting of Bones") { for exile_host in [false, true] {
        let mut game = game(); let host = card(&mut game, A, Zone::Battlefield, CardType::Creature, "Host");
        let aura = cast_aura(&mut game, &definition, host); game.take_pending_trigger_events();
        if exile_host {
            let outcome = execute_effect(&mut game, &Effect::exile(ChooseSpec::SpecificObject(host)),
                &mut EffectContext::new(aura, A, &mut SelectFirstDecisionMaker)).unwrap();
            for event in outcome.events { game.queue_trigger_event(Default::default(), event); }
            let mut queue = TriggerQueue::new(); check_and_apply_sbas(&mut game, &mut queue).unwrap();
            put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut SelectFirstDecisionMaker).unwrap();
        } else {
            game.move_object_by_effect(aura, Zone::Exile).unwrap(); destroy_and_stack(&mut game, host, &[host]);
        }
        assert!(game.stack.is_empty()); assert!(game.player(A).unwrap().hand.is_empty());
    } }
}
#[test]
fn bones_partial_or_skipped_draws_still_restrict_discard_to_its_actual_draw_receipt() {
    for definition in definitions("Casting of Bones") { for skipped in 1..=3 {
        let mut game = game(); let host = card(&mut game, B, Zone::Battlefield, CardType::Creature, "Host");
        let aura = cast_aura(&mut game, &definition, host); library(&mut game, A, 5);
        let replacement = compile_to_runtime_definition("Optional draw control",
            "Type: Enchantment\nIf you would draw a card, you may skip that draw instead.", false).unwrap();
        game.create_object_from_definition(&replacement, A, Zone::Battlefield);
        let old = card(&mut game, A, Zone::Hand, CardType::Instant, "Old hand card");
        let mut dm = Choices { expected: expected_draws(&game, A, 3 - skipped),
            before_draw_ids: game.player(A).unwrap().library.to_vec(), skip_draws: skipped, ..Default::default() };
        game.take_pending_trigger_events(); destroy_and_stack(&mut game, aura, &[host]);
        resolve_and_check(&mut game, &mut dm, A, old, 3 - skipped);
        assert_eq!(dm.replacement_calls, 3); assert_eq!(game.player(A).unwrap().library.len(), 2 + skipped);
    } }
}
#[test]
fn bones_paid_aura_spell_fizzles_when_its_only_creature_target_leaves() {
    for definition in definitions("Casting of Bones") {
        let mut game = game(); let host = card(&mut game, B, Zone::Battlefield, CardType::Creature, "Target");
        let aura = game.create_object_from_definition(&definition, A, Zone::Hand);
        let stable = game.object(aura).unwrap().stable_id;
        game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Black, 1);
        game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Colorless, 2);
        let action = compute_legal_actions(&game, A).unwrap().into_iter().find(|action|
            matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == aura)).unwrap();
        let mut dm = Choices { target: Some(Target::Object(host)), ..Default::default() };
        announce(&mut game, action, &mut dm); assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
        game.move_object_by_effect(host, Zone::Exile).unwrap();
        resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert!(game.stack.is_empty());
        let current = game.find_object_by_stable_id(stable).unwrap();
        assert_eq!(game.object(current).unwrap().zone, Zone::Graveyard);
        assert!(game.object(current).unwrap().attached_to.is_none());
        assert!(game.player(A).unwrap().hand.is_empty());
    }
}
