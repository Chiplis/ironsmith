//! Reconstructed source-only scenarios. Authored, deliberately UNRUN.
//! Full-body candidates are Fatigue, Misstep, and Orcish Farmer only.
//! The fourth frozen baseline row is held and is not a functionality claim.
use ironsmith::ability::{Ability, AbilityKind};
use ironsmith::alternative_cast::CastingMethod;
use ironsmith::cards::CardDefinition;
use ironsmith::continuous::{AbilityOrigin, ContinuousEffect, EffectTarget, Modification};
use ironsmith::decision::{DecisionMaker, LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
use ironsmith::decisions::context::{ManaPaymentContext, TargetsContext};
use ironsmith::effect::{Effect, Restriction, Until};
use ironsmith::effects::{CantEffect, DrawCardsEffect, EffectContext, EffectExecutor, ScheduleDelayedTriggerEffect, SkipScheduledEffect};
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse, apply_decision_context_with_dm, apply_priority_response_with_dm, put_triggers_on_stack_with_dm, resolve_stack_entry_with};
use ironsmith::game_state::Step;
use ironsmith::static_abilities::{StaticAbility, StaticAbilityId};
use ironsmith::target::{ChooseSpec, ObjectFilter, PlayerFilter};
use ironsmith::triggers::{Trigger, TriggerQueue};
use ironsmith::turn_runner::{TurnAction, TurnRunner, TurnState};
use ironsmith::{CardType, GameProgress, GameState, ManaSymbol, ObjectId, Phase, PlayerId, Subtype, Supertype, Target, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_core::{ContinuousDurationObject, ScheduledSkipKind};

const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
const C: PlayerId = PlayerId(2);
const CANDIDATES: [&str; 3] = ["Fatigue", "Misstep", "Orcish Farmer"];

fn fixtures() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!("../../../fixtures/next_step_durations.json.fixture")).unwrap()
}

fn definitions(name: &str) -> [CardDefinition; 2] {
    assert!(CANDIDATES.contains(&name), "held rows must not enter this candidate helper");
    let row = fixtures().into_iter().find(|row| row["name"] == name).unwrap();
    let mut text = format!("Mana cost: {}\nType: {}\n", row["mana_cost"].as_str().unwrap(), row["type_line"].as_str().unwrap());
    if let (Some(power), Some(toughness)) = (row["power"].as_str(), row["toughness"].as_str()) {
        text.push_str(&format!("Power/Toughness: {power}/{toughness}\n"));
    }
    text.push_str(row["oracle_text"].as_str().unwrap());
    definitions_text(name, &text)
}

fn definitions_text(name: &str, text: &str) -> [CardDefinition; 2] {
    let (direct, direct_loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_runtime_definition(name, text, false));
    let direct = direct.unwrap_or_else(|error| panic!("{name}: {error}"));
    assert!(!direct_loss.is_lossy(), "{name}: {}", direct_loss.reasons_text());
    let (artifact, artifact_loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_artifact(name, text, false));
    let (artifact, _) = artifact.unwrap_or_else(|error| panic!("{name}: {error}"));
    assert!(!artifact_loss.is_lossy(), "{name}: {}", artifact_loss.reasons_text());
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    restored.validate().unwrap();
    assert_eq!(artifact, restored);
    let materialized = ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap();
    assert_eq!(direct.canonical_text, materialized.canonical_text);
    assert_eq!(direct.ability_labels, materialized.ability_labels);
    for definition in [&direct, &materialized] {
        assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(definition));
    }
    [direct, materialized]
}

fn game() -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Carol".into()], 20);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.priority_player = Some(A);
    for player in [A, B, C] {
        for symbol in [ManaSymbol::Blue, ManaSymbol::Red, ManaSymbol::Colorless] {
            game.player_mut(player).unwrap().mana_pool.add(symbol, 30);
        }
        for _ in 0..6 {
            object(&mut game, player, Zone::Library, "Draw witness", "Type: Land");
        }
    }
    game
}

fn object(game: &mut GameState, player: PlayerId, zone: Zone, name: &str, text: &str) -> ObjectId {
    let definition = compile_to_runtime_definition(name, text, false).unwrap();
    game.create_object_from_definition(&definition, player, zone)
}

fn creature(game: &mut GameState, player: PlayerId) -> ObjectId {
    object(game, player, Zone::Battlefield, "Creature witness", "Type: Creature — Bear\nPower/Toughness: 2/4")
}

#[derive(Default)]
struct Choices { targets: Vec<Target> }

impl DecisionMaker for Choices {
    fn decide_targets(&mut self, _: &GameState, context: &TargetsContext) -> Vec<Target> {
        assert_eq!(context.requirements.len(), self.targets.len());
        for (requirement, target) in context.requirements.iter().zip(&self.targets) {
            assert!(requirement.legal_targets.contains(target), "announced target must be legal");
        }
        self.targets.clone()
    }

    fn decide_mana_payment(&mut self, _: &GameState, context: &ManaPaymentContext) -> ironsmith::mana_payment::ManaPaymentResponse {
        ironsmith::mana_payment::ManaPaymentResponse::Confirm { plan_id: context.plan.id, request_hash: context.plan.request_hash }
    }
}

fn perform(game: &mut GameState, action: LegalAction, dm: &mut Choices) {
    game.turn.priority_player = Some(A);
    assert!(compute_legal_actions(game, A).unwrap().contains(&action));
    let mut queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players.len());
    let mut progress = apply_priority_response_with_dm(game, &mut queue, &mut state, &PriorityResponse::PriorityAction(action), dm).unwrap();
    for _ in 0..64 {
        if state.pending_cast.is_none() && state.pending_method_selection.is_none() && state.pending_activation.is_none() { break; }
        let GameProgress::NeedsDecisionCtx(context) = progress else { panic!("{progress:?}"); };
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &context, dm).unwrap();
    }
    assert!(state.pending_cast.is_none() && state.pending_method_selection.is_none() && state.pending_activation.is_none());
    put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap();
}

fn cast(game: &mut GameState, definition: &CardDefinition, player: PlayerId) -> ObjectId {
    let hand = game.create_object_from_definition(definition, A, Zone::Hand);
    perform(game, LegalAction::CastSpell { spell_id: hand, from_zone: Zone::Hand, casting_method: CastingMethod::Normal }, &mut Choices { targets: vec![Target::Player(player)] });
    game.stack.iter().find(|entry| !entry.is_ability).unwrap().object_id
}

fn settle(game: &mut GameState) {
    let mut dm = SelectFirstDecisionMaker;
    for _ in 0..32 {
        if game.stack_is_empty() { return; }
        resolve_stack_entry_with(game, &mut dm).unwrap();
        put_triggers_on_stack_with_dm(game, &mut TriggerQueue::new(), &mut dm).unwrap();
    }
    panic!("unexpected continuing trigger chain");
}

fn activate_farmer(game: &mut GameState, farmer: ObjectId, target: ObjectId) {
    let ability_index = game.calculated_characteristics(farmer).unwrap().abilities.iter().position(|ability| matches!(ability.kind, AbilityKind::Activated(_))).unwrap();
    perform(game, LegalAction::ActivateAbility { source: farmer, ability_index }, &mut Choices { targets: vec![Target::Object(target)] });
    assert!(game.is_tapped(farmer), "pay the actual tap cost before resolution");
}

fn untap(game: &mut GameState, player: PlayerId) {
    game.turn.turn_number += 1;
    game.turn.active_player = player;
    game.turn.phase = Phase::Beginning;
    game.turn.step = Some(Step::Untap);
    ironsmith::turn::execute_untap_step_with(game, &mut SelectFirstDecisionMaker).unwrap();
    game.turn.step = Some(Step::Upkeep);
}

fn skip(game: &mut GameState, player: PlayerId, kind: ScheduledSkipKind) {
    let source = game.new_object_id();
    SkipScheduledEffect { player: PlayerFilter::Specific(player), kind, count: 1 }
        .execute(game, &mut EffectContext::new_default(source, A)).unwrap();
}

fn has_type(game: &GameState, object: ObjectId, subtype: Subtype) -> bool {
    game.current_subtypes(object).unwrap().contains(&subtype)
}

fn land(game: &mut GameState, player: PlayerId, phasing: bool) -> ObjectId {
    object(game, player, Zone::Battlefield, "Land witness", if phasing {
        "Type: Snow Land Creature — Forest Elf\nPower/Toughness: 2/4\nFlying\nPhasing\n{T}: Add {G}."
    } else {
        "Type: Snow Land Creature — Forest Elf\nPower/Toughness: 2/4\nFlying\n{T}: Add {G}."
    })
}

fn farmer(game: &mut GameState, definition: &CardDefinition) -> ObjectId {
    let farmer = game.create_object_from_definition(definition, A, Zone::Battlefield);
    game.remove_summoning_sickness(farmer);
    farmer
}

fn untagged(mut effect: &Effect) -> &Effect {
    while let Some(tagged) = effect.downcast_ref::<ironsmith::effects::TaggedEffect>() {
        effect = &tagged.effect;
    }
    effect
}

#[test]
fn three_candidate_bodies_are_lossless_and_keep_distinct_native_duration_owners() {
    for name in CANDIDATES {
        let row = fixtures().into_iter().find(|row| row["name"] == name).unwrap();
        assert!(row["oracle_id"].as_str().is_some_and(|id| !id.is_empty()));
        assert!(row["baseline"].is_object());
        for definition in definitions(name) {
            assert_eq!(definition.card.name, name);
            match name {
                "Fatigue" => {
                    let effects = definition.spell_effect.as_ref().unwrap().flattened_default_effects();
                    let skip = effects.iter().map(untagged).find_map(|effect| effect.downcast_ref::<SkipScheduledEffect>()).unwrap();
                    assert_eq!(skip.kind, ScheduledSkipKind::DrawStep);
                    assert_eq!(skip.count, 1);
                    assert!(!effects.iter().map(untagged).any(|effect| effect.downcast_ref::<DrawCardsEffect>().is_some()), "draw step is a noun, not a draw instruction");
                }
                "Misstep" => {
                    let effects = definition.spell_effect.as_ref().unwrap().flattened_default_effects();
                    let cant = effects.iter().map(untagged).find_map(|effect| effect.downcast_ref::<CantEffect>()).unwrap();
                    assert!(matches!(cant.duration, Until::PlayersNextUntapStep { .. }));
                }
                "Orcish Farmer" => {
                    let ability = definition.abilities.iter().find_map(|ability| match &ability.kind { AbilityKind::Activated(ability) => Some(ability), _ => None }).unwrap();
                    let effects = ability.effects.flattened_default_effects();
                    let conversion = effects.iter().map(untagged).find_map(|effect| effect.downcast_ref::<ironsmith::effects::BecomeBasicLandTypeChoiceEffect>()).unwrap();
                    assert_eq!(conversion.fixed_subtype, Some(Subtype::Swamp));
                    assert!(!conversion.preserve_other_types);
                    assert!(matches!(conversion.duration, Until::UntilControllersNextUntapStep { .. }));
                }
                _ => unreachable!(),
            }
        }
    }
}

#[test]
fn two_fatigues_skip_distinct_draw_steps_and_ignore_off_step_draws_other_players_and_skipped_turns() {
    for definition in definitions("Fatigue") {
        let mut game = game();
        for _ in 0..2 { cast(&mut game, &definition, B); settle(&mut game); }
        assert_eq!(game.pending_step_skips(B, Step::Draw), 2);
        let source = game.new_object_id();
        DrawCardsEffect::new(1, PlayerFilter::Specific(B)).execute(&mut game, &mut EffectContext::new_default(source, A)).unwrap();
        assert_eq!(game.player(B).unwrap().hand.len(), 1);
        assert_eq!(game.pending_step_skips(B, Step::Draw), 2);
        skip(&mut game, B, ScheduledSkipKind::Turn);
        game.next_turn();
        assert_eq!(game.turn.active_player, C);
        assert_eq!(game.pending_step_skips(B, Step::Draw), 2);
        game.turn.phase = Phase::Beginning;
        game.turn.step = Some(Step::Draw);
        assert!(!ironsmith::turn::execute_draw_step_with(&mut game, &mut SelectFirstDecisionMaker).unwrap().is_empty());
        assert_eq!(game.pending_step_skips(B, Step::Draw), 2);
        game.next_turn();
        game.next_turn();
        assert_eq!(game.turn.active_player, B);
        game.turn.phase = Phase::Beginning;
        game.turn.step = Some(Step::Upkeep);
        game.add_step_after(Step::Draw, Step::Draw);
        let mut runner = TurnRunner::from_state_for_sync(TurnState::Draw);
        let mut queue = TriggerQueue::new();
        assert!(matches!(runner.advance(&mut game, &mut queue).unwrap(), TurnAction::Continue));
        assert_eq!(game.pending_step_skips(B, Step::Draw), 1);
        assert_eq!(game.player(B).unwrap().hand.len(), 1);
        // The phase boundary may require a separate transition, but no draw
        // is allowed to take place while either counted skip remains.
        for _ in 0..8 {
            if game.pending_step_skips(B, Step::Draw) == 0 { break; }
            assert!(matches!(runner.advance(&mut game, &mut queue).unwrap(), TurnAction::Continue));
            assert_eq!(game.player(B).unwrap().hand.len(), 1);
        }
        assert_eq!(game.pending_step_skips(B, Step::Draw), 0);
        game.turn.phase = Phase::Beginning;
        game.turn.step = Some(Step::Draw);
        ironsmith::turn::execute_draw_step_with(&mut game, &mut SelectFirstDecisionMaker).unwrap();
        assert_eq!(game.player(B).unwrap().hand.len(), 2);
    }
}

#[test]
fn fatigue_can_target_its_caster_and_does_not_attach_the_skip_to_the_spell_incarnation() {
    for definition in definitions("Fatigue") {
        let mut game = game();
        let spell = cast(&mut game, &definition, A);
        let stable = game.object(spell).unwrap().stable_id;
        settle(&mut game);
        let grave = game.find_object_by_stable_id(stable).unwrap();
        game.move_object_by_effect(grave, Zone::Exile).unwrap();
        assert_eq!(game.pending_step_skips(A, Step::Draw), 1);
        assert_eq!(game.pending_step_skips(B, Step::Draw), 0);
        game.turn.phase = Phase::Beginning;
        game.turn.step = Some(Step::Draw);
        let before = game.player(A).unwrap().hand.len();
        assert!(ironsmith::turn::execute_draw_step_with(&mut game, &mut SelectFirstDecisionMaker).unwrap().is_empty());
        assert_eq!(game.player(A).unwrap().hand.len(), before);
        assert_eq!(game.pending_step_skips(A, Step::Draw), 0);
    }
}

#[test]
fn misstep_uses_the_live_creatures_of_its_selected_player_including_later_entries_and_control_changes() {
    for definition in definitions("Misstep") {
        let mut game = game();
        let original = creature(&mut game, B);
        let leaving = creature(&mut game, B);
        let arriving = creature(&mut game, C);
        let noncreature = object(&mut game, B, Zone::Battlefield, "Plain land", "Type: Land");
        let spell = cast(&mut game, &definition, B);
        let stable = game.object(spell).unwrap().stable_id;
        settle(&mut game);
        let late = creature(&mut game, B);
        game.set_current_controller(leaving, C).unwrap();
        game.set_current_controller(arriving, B).unwrap();
        let spell_grave = game.find_object_by_stable_id(stable).unwrap();
        game.move_object_by_effect(spell_grave, Zone::Exile).unwrap();
        for id in [original, leaving, arriving, late, noncreature] { game.tap(id); }
        untap(&mut game, C);
        assert!(!game.is_tapped(leaving), "changing control removes it from B's live set");
        untap(&mut game, B);
        for id in [original, arriving, late] { assert!(game.is_tapped(id)); }
        assert!(!game.is_tapped(noncreature));
        assert!(game.effect_store.restriction_effects.iter().all(|effect| !matches!(effect.duration, Until::PlayersNextUntapStep { .. })));
        untap(&mut game, B);
        for id in [original, arriving, late] { assert!(!game.is_tapped(id)); }
    }
}

#[test]
fn misstep_binds_an_empty_player_set_and_affects_a_blinked_new_creature_incarnation() {
    for definition in definitions("Misstep") {
        let mut game = game();
        cast(&mut game, &definition, C);
        settle(&mut game);
        let old = creature(&mut game, C);
        let exile = game.move_object_by_effect(old, Zone::Exile).unwrap();
        let returned = game.move_object_by_effect(exile, Zone::Battlefield).unwrap();
        assert_ne!(old, returned);
        let other = creature(&mut game, B);
        game.tap(returned);
        game.tap(other);
        untap(&mut game, B);
        assert!(!game.is_tapped(other));
        untap(&mut game, C);
        assert!(game.is_tapped(returned), "a rule about C's creatures must include the new incarnation");
        untap(&mut game, C);
        assert!(!game.is_tapped(returned));
    }
}

#[test]
fn misstep_survives_source_controller_departure_a_skipped_step_and_phasing() {
    for definition in definitions("Misstep") {
        let mut game = game();
        let target = creature(&mut game, B);
        game.tap(target);
        cast(&mut game, &definition, B);
        settle(&mut game);
        game.phase_out(target);
        skip(&mut game, B, ScheduledSkipKind::UntapStep);
        game.leave_game(A).unwrap();
        game.turn.active_player = B;
        let mut runner = TurnRunner::new();
        assert!(matches!(runner.advance(&mut game, &mut TriggerQueue::new()).unwrap(), TurnAction::Continue));
        assert!(game.is_phased_out(target));
        assert!(game.is_tapped(target));
        assert!(game.effect_store.restriction_effects.iter().any(|effect| matches!(&effect.duration, Until::PlayersNextUntapStep { player: PlayerFilter::Specific(player) } if *player == B)));
        untap(&mut game, B);
        assert!(!game.is_phased_out(target));
        assert!(game.is_tapped(target));
        untap(&mut game, B);
        assert!(!game.is_tapped(target));
    }
}

#[test]
fn misstep_does_not_prohibit_effect_untaps_or_untapping_during_another_players_step() {
    for definition in definitions("Misstep") {
        let mut game = game();
        let target = creature(&mut game, B);
        cast(&mut game, &definition, B);
        settle(&mut game);
        game.tap(target);
        let source = game.new_object_id();
        ironsmith::effects::UntapEffect::with_spec(ChooseSpec::SpecificObject(target))
            .execute(&mut game, &mut EffectContext::new_default(source, A)).unwrap();
        assert!(!game.is_tapped(target));
        let muse = compile_to_runtime_definition("Other turn untap witness", "Type: Creature\nPower/Toughness: 2/4\nUntap all permanents you control during each other player's untap step.", false).unwrap();
        game.create_object_from_definition(&muse, B, Zone::Battlefield);
        game.tap(target);
        untap(&mut game, C);
        assert!(!game.is_tapped(target));
        game.tap(target);
        untap(&mut game, B);
        assert!(game.is_tapped(target));
    }
}

#[test]
fn farmer_pays_its_tap_cost_and_replaces_printed_land_abilities_but_keeps_external_grants() {
    for definition in definitions("Orcish Farmer") {
        let mut game = game();
        let farmer = farmer(&mut game, &definition);
        let target = land(&mut game, B, false);
        let grant_source = creature(&mut game, A);
        // Equal green mana definitions must remain distinct acquisitions:
        // CR 305.7 removes the printed/intrinsic pair, retaining this grant.
        let granted = Ability::basic_land_mana(Subtype::Forest).unwrap();
        game.effect_store.continuous_effects.add_effect(ContinuousEffect::new(grant_source, A, EffectTarget::Specific(target), Modification::AddAbilityGeneric(granted.clone())));
        game.effect_store.continuous_effects.add_effect(ContinuousEffect::new(grant_source, A, EffectTarget::Specific(target), Modification::AddAbility(StaticAbility::haste())));
        game.refresh_continuous_state().unwrap();
        let before = game.calculated_characteristics(target).unwrap();
        let grant_index = before.abilities.iter().enumerate().find_map(|(index, ability)| {
            (*ability == granted && matches!(before.abilities.origin(index), Some(AbilityOrigin::Effect { .. }))).then_some(index)
        }).unwrap();
        let grant_origin = before.abilities.origin(grant_index).unwrap().clone();
        activate_farmer(&mut game, farmer, target);
        assert!(has_type(&game, target, Subtype::Forest), "announcement alone does not apply the conversion");
        game.move_object_by_effect(farmer, Zone::Graveyard).unwrap();
        settle(&mut game);
        assert!(has_type(&game, target, Subtype::Swamp));
        assert!(!has_type(&game, target, Subtype::Forest));
        assert!(has_type(&game, target, Subtype::Elf));
        assert!(game.object_has_card_type(target, CardType::Land));
        assert!(game.object_has_card_type(target, CardType::Creature));
        assert!(game.current_supertypes(target).unwrap().contains(&Supertype::Snow));
        assert!(!game.current_supertypes(target).unwrap().contains(&Supertype::Basic));
        assert!(!game.current_has_static_ability_id(target, StaticAbilityId::Flying));
        assert!(game.current_has_static_ability_id(target, StaticAbilityId::Haste));
        let after = game.calculated_characteristics(target).unwrap();
        assert_eq!(after.abilities.iter().filter(|ability| **ability == granted).count(), 1);
        assert!(!after.abilities.iter().enumerate().any(|(index, _)| matches!(after.abilities.origin(index), Some(AbilityOrigin::Printed(_) | AbilityOrigin::IntrinsicBasicLandMana(Subtype::Forest)))));
        assert!(after.abilities.iter().enumerate().any(|(index, ability)| *ability == Ability::basic_land_mana(Subtype::Swamp).unwrap() && after.abilities.origin(index) == Some(&AbilityOrigin::IntrinsicBasicLandMana(Subtype::Swamp))));
        assert!(after.abilities.iter().enumerate().any(|(index, ability)| *ability == granted && after.abilities.origin(index) == Some(&grant_origin)));
        untap(&mut game, A);
        assert!(has_type(&game, target, Subtype::Swamp), "the source controller's step is not the target's boundary");
        untap(&mut game, B);
        assert!(has_type(&game, target, Subtype::Forest));
        assert!(!has_type(&game, target, Subtype::Swamp));
        assert!(game.current_has_static_ability_id(target, StaticAbilityId::Flying));
        assert!(game.current_has_static_ability_id(target, StaticAbilityId::Haste));
    }
}

#[test]
fn farmer_tracks_the_lands_current_controller_and_expires_before_phasing() {
    // The changing-controller boundary is an authored rules inference from
    // the corresponding Lorthos ruling, pending independent rules review.
    // This unrun scenario is not evidence that the inference is validated.
    for definition in definitions("Orcish Farmer") {
        let mut game = game();
        let farmer = farmer(&mut game, &definition);
        let target = land(&mut game, B, true);
        game.tap(target);
        activate_farmer(&mut game, farmer, target);
        settle(&mut game);
        assert!(!game.current_has_static_ability_id(target, StaticAbilityId::Phasing));
        game.set_current_controller(target, C).unwrap();
        untap(&mut game, B);
        assert!(has_type(&game, target, Subtype::Swamp));
        assert!(!game.is_phased_out(target));
        untap(&mut game, C);
        assert!(game.is_phased_out(target), "restored printed phasing participates in the same beginning-of-step exchange");
        assert!(game.is_tapped(target), "a permanent that phases out cannot then untap");
        game.phase_in(target);
        assert!(has_type(&game, target, Subtype::Forest));
        assert!(!has_type(&game, target, Subtype::Swamp));
    }
}

#[test]
fn farmer_keeps_the_exact_incarnation_and_a_skipped_untap_does_not_expire_it() {
    for definition in definitions("Orcish Farmer") {
        for blink_before_resolution in [false, true] {
            let mut game = game();
            let farmer = farmer(&mut game, &definition);
            let target = land(&mut game, B, false);
            activate_farmer(&mut game, farmer, target);
            if !blink_before_resolution { settle(&mut game); assert!(has_type(&game, target, Subtype::Swamp)); }
            let exile = game.move_object_by_effect(target, Zone::Exile).unwrap();
            let returned = game.move_object_by_effect(exile, Zone::Battlefield).unwrap();
            if blink_before_resolution { settle(&mut game); }
            assert_ne!(target, returned);
            assert!(has_type(&game, returned, Subtype::Forest));
            assert!(!has_type(&game, returned, Subtype::Swamp));
        }
        let mut game = game();
        let farmer = farmer(&mut game, &definition);
        let target = land(&mut game, B, false);
        activate_farmer(&mut game, farmer, target);
        settle(&mut game);
        game.phase_out(target);
        skip(&mut game, B, ScheduledSkipKind::UntapStep);
        game.next_turn();
        assert_eq!(game.turn.active_player, B);
        let mut runner = TurnRunner::new();
        assert!(matches!(runner.advance(&mut game, &mut TriggerQueue::new()).unwrap(), TurnAction::Continue));
        assert!(game.is_phased_out(target));
        game.phase_in(target);
        assert!(has_type(&game, target, Subtype::Swamp));
        game.phase_out(target);
        untap(&mut game, B);
        assert!(!game.is_phased_out(target));
        assert!(has_type(&game, target, Subtype::Forest));
    }
}

#[test]
fn independent_farmer_resolutions_keep_each_lands_step_owner_and_basic_supertype_after_source_departure() {
    for definition in definitions("Orcish Farmer") {
        let mut game = game();
        let farmer = farmer(&mut game, &definition);
        let first = object(&mut game, B, Zone::Battlefield, "Basic forest", "Type: Basic Snow Land — Forest");
        let second = object(&mut game, C, Zone::Battlefield, "Other mountain", "Type: Land — Mountain");
        activate_farmer(&mut game, farmer, first);
        settle(&mut game);
        game.untap(farmer);
        activate_farmer(&mut game, farmer, second);
        settle(&mut game);
        game.move_object_by_effect(farmer, Zone::Exile).unwrap();
        assert!(has_type(&game, first, Subtype::Swamp));
        assert!(has_type(&game, second, Subtype::Swamp));
        assert!(game.current_supertypes(first).unwrap().contains(&Supertype::Basic));
        assert!(game.current_supertypes(first).unwrap().contains(&Supertype::Snow));
        assert!(!game.current_supertypes(second).unwrap().contains(&Supertype::Basic));
        untap(&mut game, A);
        assert!(has_type(&game, first, Subtype::Swamp));
        assert!(has_type(&game, second, Subtype::Swamp));
        untap(&mut game, B);
        assert!(has_type(&game, first, Subtype::Forest));
        assert!(!has_type(&game, first, Subtype::Swamp));
        assert!(has_type(&game, second, Subtype::Swamp));
        untap(&mut game, C);
        assert!(has_type(&game, second, Subtype::Mountain));
        assert!(!has_type(&game, second, Subtype::Swamp));
    }
}

fn schedule_during_untap(game: &mut GameState, source: ObjectId, player: PlayerId, effects: Vec<Effect>) {
    ScheduleDelayedTriggerEffect::new(Trigger::as_permanents_untap(PlayerFilter::Specific(player), true), effects, true, Vec::new(), PlayerFilter::Specific(player))
        .execute(game, &mut EffectContext::new_default(source, player)).unwrap();
}

#[test]
fn restriction_registered_inside_untap_waits_for_the_next_actual_same_turn_untap() {
    let mut game = game();
    let source = creature(&mut game, B);
    let target = creature(&mut game, B);
    game.tap(target);
    schedule_during_untap(&mut game, source, B, vec![Effect::new(CantEffect::new(
        Restriction::untap(ObjectFilter::creature().controlled_by(PlayerFilter::Specific(B))),
        Until::PlayersNextUntapStep { player: PlayerFilter::Specific(B) },
    ))]);
    game.next_turn();
    let turn_number = game.turn.turn_number;
    game.add_step_after(Step::Untap, Step::Untap);
    let mut runner = TurnRunner::new();
    let mut queue = TriggerQueue::new();
    assert!(matches!(runner.advance(&mut game, &mut queue).unwrap(), TurnAction::Continue));
    assert!(matches!(runner.state(), TurnState::UntapEndMana));
    assert!(!game.is_tapped(target), "a newly registered next-step rule cannot retroactively block this step");
    let receipt = game.turn_store.untap_step_started_at.expect("the actual step has a native receipt");
    assert_eq!(receipt.0, turn_number);
    let registration = game.effect_store.restriction_effects.iter().find(|effect| matches!(effect.duration, Until::PlayersNextUntapStep { .. })).unwrap();
    assert!(registration.timestamp > receipt.1);
    assert!(!registration.consumed_next_untap);
    game.tap(target);
    assert!(matches!(runner.advance(&mut game, &mut queue).unwrap(), TurnAction::Continue));
    assert!(matches!(runner.state(), TurnState::Untap));
    assert!(matches!(runner.advance(&mut game, &mut queue).unwrap(), TurnAction::Continue));
    assert_eq!(game.turn.turn_number, turn_number, "the extra occurrence is in the same turn");
    assert!(game.is_tapped(target));
    assert!(game.turn_store.untap_step_started_at.unwrap().1 > receipt.1);
    assert!(game.effect_store.restriction_effects.iter().all(|effect| !matches!(effect.duration, Until::PlayersNextUntapStep { .. })), "consume after untapping, before the separate mana-empty boundary");
    assert!(matches!(runner.state(), TurnState::UntapEndMana));
}

#[test]
fn continuous_duration_registered_after_the_beginning_boundary_waits_for_an_added_untap() {
    let mut game = game();
    let source = creature(&mut game, B);
    let target = creature(&mut game, B);
    schedule_during_untap(&mut game, source, B, vec![Effect::pump(3, 0, ChooseSpec::SpecificObject(target),
        Until::UntilControllersNextUntapStep { object: ContinuousDurationObject::Specific(target) })]);
    game.next_turn();
    let turn_number = game.turn.turn_number;
    game.add_step_after(Step::Untap, Step::Untap);
    let mut runner = TurnRunner::new();
    let mut queue = TriggerQueue::new();
    runner.advance(&mut game, &mut queue).unwrap();
    assert_eq!(game.current_power(target), Some(5));
    runner.advance(&mut game, &mut queue).unwrap();
    assert!(matches!(runner.state(), TurnState::Untap));
    runner.advance(&mut game, &mut queue).unwrap();
    assert_eq!(game.turn.turn_number, turn_number);
    assert_eq!(game.current_power(target), Some(2));
}

#[test]
fn until_leaves_renderer_keeps_the_event_duration_distinct_from_for_as_long_as() {
    let literal = Effect::pump(1, 1, ChooseSpec::Source, Until::ThisLeavesTheBattlefield);
    let predicate = Effect::pump(1, 1, ChooseSpec::Source, Until::while_source_remains_on_battlefield());
    let literal_text = ironsmith_text::compile_effect_list(&[literal]);
    let predicate_text = ironsmith_text::compile_effect_list(&[predicate]);
    assert!(literal_text.to_lowercase().contains("until this source leaves the battlefield"));
    assert!(predicate_text.to_lowercase().contains("for as long as"));
    assert_ne!(literal_text, predicate_text);
    for (text, predicate_expected) in [
        ("Type: Creature\nPower/Toughness: 1/1\n{T}: Target creature gets +1/+1 until this creature leaves the battlefield.", false),
        ("Type: Creature\nPower/Toughness: 1/1\n{T}: Target creature gets +1/+1 for as long as this creature remains on the battlefield.", true),
    ] {
        for definition in definitions_text("Duration distinction witness", text) {
            let ability = definition.abilities.iter().find_map(|ability| match &ability.kind { AbilityKind::Activated(ability) => Some(ability), _ => None }).unwrap();
            let effects = ability.effects.flattened_default_effects();
            let duration = effects.iter().map(untagged).find_map(|effect| {
                effect.downcast_ref::<ironsmith::effects::ApplyContinuousEffect>().map(|effect| &effect.until)
                    .or_else(|| effect.downcast_ref::<ironsmith::effects::ModifyPowerToughnessEffect>().map(|effect| &effect.duration))
            }).unwrap();
            assert_eq!(matches!(duration, Until::ForAsLongAs(_)), predicate_expected);
            assert_eq!(matches!(duration, Until::ThisLeavesTheBattlefield), !predicate_expected);
        }
    }
}

#[test]
fn complete_next_step_bodies_reject_orphan_players_and_malformed_timing_tails() {
    for text in [
        "Type: Sorcery\nCreatures you control don't untap during that player's next untap step.",
        "Type: Sorcery\nCreatures target player controls don't untap during that player's next untap step except on Tuesdays.",
        "Type: Sorcery\nCreatures target player controls don't untap during that player's next upkeep step.",
        "Type: Sorcery\nTarget player skips their next draw step forever.",
        "Type: Sorcery\nTarget player skips their next draw stepping.",
        "Type: Creature — Orc\nPower/Toughness: 2/2\n{T}: Target land becomes a Swamp until its controller's next untap step except on Tuesdays.",
        "Type: Creature — Orc\nPower/Toughness: 2/2\n{T}: Target land becomes a Swamp until its controller's next untap.",
    ] {
        let (result, loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_artifact("Whole-body duration negative", text, false));
        assert!(result.is_err() || loss.is_lossy(), "malformed or unbound body accepted losslessly: {text}");
    }
}


#[test]
fn new_next_step_durations_reject_unsupported_effect_owners_in_complete_bodies() {
    // Authored independently from the positive card fixtures. Both output
    // routes must refuse an unrepresented lifetime rather than emit Forever.
    for text in [
        "Type: Sorcery\nTarget creature has base power 5 during that player's next untap step.",
        "Type: Sorcery\nTarget player gains 1 life. Target creature has base power 5 during that player's next untap step.",
        "Type: Sorcery\nTarget creature gets +1/+1 during that player's next untap step.",
        "Type: Sorcery\nTarget creature gains flying during that player's next untap step.",
        "Type: Sorcery\nDuring that player's next untap step, target creature has base power 5.",
        "Type: Sorcery\nExile the top card of your library. You may play that card during that player's next untap step.",
        "Type: Sorcery\nTarget creature doesn't untap until its controller's next untap step.",
        "Type: Sorcery\nTarget creature can't attack until its controller's next untap step.",
        "Type: Sorcery\nCreatures target player controls don't untap until its controller's next untap step.",
        "Type: Sorcery\nIf you control an Island, creatures target player controls don't untap during that player's next untap step.",
    ] {
        let (direct, direct_loss) = ironsmith_compiler::parse_loss::capture(||
            compile_to_runtime_definition("Unsupported next-step owner", text, false));
        assert!(direct.is_err() || direct_loss.is_lossy(), "direct route accepted unsupported owner: {text}");
        let (artifact, artifact_loss) = ironsmith_compiler::parse_loss::capture(||
            compile_to_artifact("Unsupported next-step owner", text, false));
        assert!(artifact.is_err() || artifact_loss.is_lossy(), "artifact route accepted unsupported owner: {text}");
    }
}

#[test]
fn prevention_permissions_and_carried_actions_reject_unowned_next_step_lifetimes() {
    for timing in ["Until its controller's next untap step", "During that player's next untap step"] {
        for body in [
            format!("{timing}, prevent all damage that would be dealt by target creature."),
            format!("{timing}, prevent all combat damage that would be dealt this turn."),
            format!("{timing}, prevent all damage that would be dealt to target creature."),
            format!("{timing}, prevent the next 3 damage that would be dealt to target creature."),
            format!("{timing}, target creature gains flying."),
            format!("{timing}, gain control of target creature."),
            format!("{timing}, you may play an additional land on each of your turns."),
            format!("Exile the top card of your library. You may play that card {}.", timing.to_lowercase()),
        ] {
            let text = format!("Type: Sorcery\n{body}");
            let (direct, loss) = ironsmith_compiler::parse_loss::capture(||
                compile_to_runtime_definition("Unowned carried duration", &text, false));
            assert!(direct.is_err() || loss.is_lossy(), "direct route accepted: {text}");
            let (artifact, loss) = ironsmith_compiler::parse_loss::capture(||
                compile_to_artifact("Unowned carried duration", &text, false));
            assert!(artifact.is_err() || loss.is_lossy(), "artifact route accepted: {text}");
        }
    }
}
