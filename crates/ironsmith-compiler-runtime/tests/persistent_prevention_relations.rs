//! Complete frozen bodies; all compilation and execution remains deferred.
use ironsmith::ability::Ability;
use ironsmith::card::{CardBuilder, PowerToughness};
use ironsmith::cards::CardDefinition;
use ironsmith::color::{Color, ColorSet};
use ironsmith::continuous::{EffectTarget, Modification};
use ironsmith::decision::{DecisionMaker, LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
use ironsmith::effect::{Effect, Until};
use ironsmith::effects::{ApplyContinuousEffect, DealDamageEffect, EffectContext, EffectExecutor,
    ExecutionError, SequenceEffect, execute_effect};
use ironsmith::events::{DamagePreventedEvent, DamageTarget};
use ironsmith::events::cause::EventCause;
use ironsmith::events::processing::process_damage_assignments_with_event_with_source_snapshot_opts;
use ironsmith::object::AttachmentTarget;
use ironsmith::snapshot::ObjectSnapshot;
use ironsmith::static_abilities::{StaticAbility, StaticAbilityId};
use ironsmith::target::ChooseSpec;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{CardId, CardType, GameState, ObjectId, PlayerId, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;
const A: PlayerId = PlayerId::from_index(0);
const B: PlayerId = PlayerId::from_index(1);
const NAMES: &[&str] = &[
    "Artifact Ward", "Energy Field", "Energy Storm", "Gideon's Intervention", "Goblin Furrier",
    "Indentured Oaf", "Light of Sanction", "Prismatic Ward", "Wall of Vapor",
];

fn definitions(name: &str) -> [CardDefinition; 2] {
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../fixtures/persistent_prevention_relations.json.fixture")).unwrap();
    assert_eq!(rows.len(), NAMES.len());
    let row = rows.iter().find(|row| row["name"] == name).unwrap();
    let mut text = format!("Mana cost: {}\nType: {}\n", row["mana_cost"].as_str().unwrap(),
        row["type_line"].as_str().unwrap());
    if let (Some(power), Some(toughness)) = (row["power"].as_str(), row["toughness"].as_str()) {
        text.push_str(&format!("Power/Toughness: {power}/{toughness}\n"));
    }
    text.push_str(row["oracle_text"].as_str().unwrap());
    let (result, loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_artifact(name, &text, false));
    let (artifact, _) = result.unwrap_or_else(|error| panic!("{name}: {error}"));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    artifact.validate().unwrap();
    let decoded = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(decoded, artifact);
    let (result, loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_runtime_definition(name, &text, false));
    let direct = result.unwrap_or_else(|error| panic!("direct {name}: {error}"));
    assert!(!loss.is_lossy(), "direct {name}: {}", loss.reasons_text());
    let result = [direct, materialize_artifact(&decoded).unwrap()];
    for definition in &result {
        assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(definition));
    }
    result
}
fn game() -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    main_phase(&mut game, A);
    game
}
fn main_phase(game: &mut GameState, player: PlayerId) {
    game.turn.active_player = player;
    game.turn.priority_player = Some(player);
    game.turn.phase = ironsmith::Phase::FirstMain;
    game.turn.step = None;
}
fn object(game: &mut GameState, owner: PlayerId, zone: Zone, types: Vec<CardType>,
    abilities: Vec<StaticAbility>) -> ObjectId {
    let card = CardBuilder::new(CardId::new(), "Unlisted relation source")
        .card_types(types).power_toughness(PowerToughness::fixed(2, 20)).build();
    let mut definition = CardDefinition::new(card);
    definition.abilities = abilities.into_iter().map(Ability::static_ability).collect();
    game.create_object_from_definition(&definition, owner, zone)
}
fn creature(game: &mut GameState, owner: PlayerId) -> ObjectId {
    object(game, owner, Zone::Battlefield, vec![CardType::Creature], vec![])
}
fn change(game: &mut GameState, target: ObjectId, modification: Modification) {
    ApplyContinuousEffect::new(EffectTarget::Specific(target), modification, Until::Forever)
        .execute(game, &mut EffectContext::new_default(target, A)).unwrap();
}
fn snapshot(game: &GameState, source: ObjectId) -> ObjectSnapshot {
    ObjectSnapshot::from_object_with_calculated_characteristics(game.object(source).unwrap(), game)
}
fn damage(game: &mut GameState, source: ObjectId, target: DamageTarget, combat: bool,
    unpreventable: bool, lki: Option<&ObjectSnapshot>) -> u32 {
    game.take_pending_trigger_events();
    let result = process_damage_assignments_with_event_with_source_snapshot_opts(
        game, source, target, 3, combat, unpreventable, EventCause::effect(), lki).unwrap();
    let amount: u32 = result.assignments.iter().map(|assignment| assignment.amount).sum();
    let prevented: u32 = game.take_pending_trigger_events().into_iter().filter_map(|event|
        event.downcast::<DamagePreventedEvent>().map(|event| event.amount)).sum();
    assert_eq!(prevented, 3 - amount);
    amount
}
struct Choices { accept: bool }
impl DecisionMaker for Choices {
    fn decide_colors(&mut self, _: &GameState, context: &ironsmith::decisions::context::ColorsContext) -> Vec<Color> {
        vec![Color::Blue; context.count as usize]
    }
    fn decide_text(&mut self, _: &GameState, _: &ironsmith::decisions::context::TextInputContext) -> String {
        "Lightning Bolt".into()
    }
    fn decide_boolean(&mut self, _: &GameState, context: &ironsmith::decisions::context::BooleanContext) -> bool {
        self.accept && context.can_accept
    }
    fn decide_mana_payment(&mut self, game: &GameState, context: &ironsmith::decisions::context::ManaPaymentContext)
        -> ironsmith::mana_payment::ManaPaymentResponse {
        SelectFirstDecisionMaker.decide_mana_payment(game, context)
    }
}
fn enter(game: &mut GameState, definition: &CardDefinition) -> ObjectId {
    let old = game.create_object_from_definition(definition, A, Zone::Hand);
    let receipt = game.move_object_with_etb_processing_with_dm(old, Zone::Battlefield,
        &mut Choices { accept: true }).unwrap();
    assert!(!receipt.pending && receipt.programs.is_empty());
    receipt.original.into_result().unwrap().new_id
}
fn settle(game: &mut GameState, choices: &mut impl DecisionMaker) {
    for _ in 0..12 {
        ironsmith::game_loop::put_triggers_on_stack_with_dm(game, &mut TriggerQueue::new(), choices).unwrap();
        if game.stack_is_empty() { return; }
        ironsmith::game_loop::resolve_stack_entry_with(game, choices).unwrap();
    }
    panic!("unexpected repeating trigger");
}

#[test]
fn all_nine_complete_bodies_retain_their_typed_prevention_owner_in_both_routes() {
    for name in NAMES { for definition in definitions(name) {
        assert!(definition.abilities.iter().any(|ability| match &ability.kind {
            ironsmith::ability::AbilityKind::Static(ability) => ability.id() == StaticAbilityId::PreventMatchingDamage,
            _ => false,
        }), "{name}");
    } }
}

#[test]
fn light_of_sanction_uses_independent_live_source_and_recipient_controller_filters() {
    for definition in definitions("Light of Sanction") {
        let mut game = game();
        let host = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let own = creature(&mut game, A);
        let other = creature(&mut game, B);
        for zone in [Zone::Battlefield, Zone::Stack, Zone::Graveyard] {
            let source = object(&mut game, A, zone, vec![CardType::Artifact], vec![]);
            for combat in [false, true, false] {
                assert_eq!(damage(&mut game, source, DamageTarget::Object(own), combat, false, None), 0);
                assert_eq!(damage(&mut game, source, DamageTarget::Object(other), combat, false, None), 3);
                assert_eq!(damage(&mut game, source, DamageTarget::Player(A), combat, false, None), 3);
                assert_eq!(damage(&mut game, source, DamageTarget::Object(own), combat, true, None), 3);
            }
        }
        assert_eq!(damage(&mut game, other, DamageTarget::Object(own), false, false, None), 3);
        game.set_current_controller(host, B).unwrap();
        assert_eq!(damage(&mut game, other, DamageTarget::Object(other), false, false, None), 0);
        assert_eq!(damage(&mut game, own, DamageTarget::Object(own), false, false, None), 3);
        game.phase_out(host);
        assert_eq!(damage(&mut game, other, DamageTarget::Object(other), false, false, None), 3);
        game.phase_in(host);
        assert_eq!(damage(&mut game, other, DamageTarget::Object(other), false, false, None), 0);
        change(&mut game, host, Modification::RemoveAllAbilities);
        assert_eq!(damage(&mut game, other, DamageTarget::Object(other), false, false, None), 3);
    }
}

#[test]
fn energy_field_protects_its_controller_and_its_entire_graveyard_trigger_is_retained() {
    for definition in definitions("Energy Field") { for zone in [Zone::Hand, Zone::Library, Zone::Stack, Zone::Battlefield] {
        let mut game = game();
        let host = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let own = creature(&mut game, A);
        let opponent = creature(&mut game, B);
        for combat in [false, true] {
            assert_eq!(damage(&mut game, opponent, DamageTarget::Player(A), combat, false, None), 0);
            assert_eq!(damage(&mut game, opponent, DamageTarget::Player(B), combat, false, None), 3);
            assert_eq!(damage(&mut game, own, DamageTarget::Player(A), combat, false, None), 3);
            assert_eq!(damage(&mut game, opponent, DamageTarget::Object(own), combat, false, None), 3);
        }
        let their_card = object(&mut game, B, zone, vec![CardType::Artifact], vec![]);
        game.move_object_by_effect(their_card, Zone::Graveyard).unwrap();
        settle(&mut game, &mut Choices { accept: true });
        assert!(game.battlefield.contains(&host));
        let card = object(&mut game, A, zone, vec![CardType::Artifact], vec![]);
        game.move_object_by_effect(card, Zone::Graveyard).unwrap();
        settle(&mut game, &mut Choices { accept: true });
        assert!(!game.battlefield.contains(&host), "{zone:?} must trigger sacrifice");
        assert_eq!(damage(&mut game, opponent, DamageTarget::Player(A), false, false, None), 3);
    } }
}

#[test]
fn energy_storm_prevents_spell_damage_and_preserves_upkeep_and_flying_untap_rules() {
    for definition in definitions("Energy Storm") {
        let mut game = game();
        let host = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let target = creature(&mut game, B);
        for (card_type, zone, prevented) in [
            (CardType::Instant, Zone::Stack, true), (CardType::Sorcery, Zone::Stack, true),
            (CardType::Creature, Zone::Stack, false), (CardType::Instant, Zone::Graveyard, false),
        ] {
            let source = object(&mut game, A, zone, vec![card_type], vec![]);
            for recipient in [DamageTarget::Player(A), DamageTarget::Player(B), DamageTarget::Object(target)] {
                assert_eq!(damage(&mut game, source, recipient, false, false, None), if prevented { 0 } else { 3 });
            }
        }
        for player in [A, B] {
            let flyer = object(&mut game, player, Zone::Battlefield, vec![CardType::Creature], vec![StaticAbility::flying()]);
            let grounded = creature(&mut game, player);
            game.tap(flyer); game.tap(grounded);
            game.turn.active_player = player;
            game.turn.phase = ironsmith::Phase::Beginning;
            ironsmith::turn_runner::TurnRunner::from_state_for_sync(ironsmith::turn_runner::TurnState::Untap)
                .advance(&mut game, &mut TriggerQueue::new()).unwrap();
            assert!(game.is_tapped(flyer));
            assert!(!game.is_tapped(grounded));
        }
        game.turn.active_player = A;
        for expected_age in 1..=2 {
            let mut queue = TriggerQueue::new();
            ironsmith::turn_runner::TurnRunner::from_state_for_sync(ironsmith::turn_runner::TurnState::Upkeep)
                .advance(&mut game, &mut queue).unwrap();
            game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless, 10);
            let before = game.player(A).unwrap().mana_pool.total();
            let mut choices = Choices { accept: true };
            ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut choices).unwrap();
            settle(&mut game, &mut choices);
            assert_eq!(game.counter_count(host, ironsmith::CounterType::Age), expected_age);
            assert_eq!(game.player(A).unwrap().mana_pool.total(), before - expected_age);
        }
        let mut queue = TriggerQueue::new();
        ironsmith::turn_runner::TurnRunner::from_state_for_sync(ironsmith::turn_runner::TurnState::Upkeep)
            .advance(&mut game, &mut queue).unwrap();
        let mut choices = Choices { accept: false };
        ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut choices).unwrap();
        settle(&mut game, &mut choices);
        assert!(!game.battlefield.contains(&host));
    }
}

#[test]
fn active_voice_prevention_applies_only_to_the_sources_damage_and_current_matching_creatures() {
    for name in ["Goblin Furrier", "Indentured Oaf"] { for definition in definitions(name) {
        let mut game = game();
        let host = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let ordinary = creature(&mut game, B);
        let qualified = creature(&mut game, B);
        let qualifier = if name == "Goblin Furrier" {
            Modification::AddSupertypes(vec![ironsmith::Supertype::Snow])
        } else { Modification::SetColors(ColorSet::RED) };
        change(&mut game, qualified, qualifier);
        for combat in [false, true] {
            assert_eq!(damage(&mut game, host, DamageTarget::Object(qualified), combat, false, None), 0);
            assert_eq!(damage(&mut game, host, DamageTarget::Object(ordinary), combat, false, None), 3);
            assert_eq!(damage(&mut game, host, DamageTarget::Player(B), combat, false, None), 3);
            assert_eq!(damage(&mut game, ordinary, DamageTarget::Object(qualified), combat, false, None), 3);
            assert_eq!(damage(&mut game, host, DamageTarget::Object(qualified), combat, true, None), 3);
        }
        change(&mut game, qualified, Modification::SetCardTypes(vec![CardType::Artifact]));
        assert_eq!(damage(&mut game, host, DamageTarget::Object(qualified), false, false, None), 3);
    } }
}

#[test]
fn prismatic_ward_chooses_on_entry_and_tracks_its_current_attachment_and_live_damage_source() {
    for definition in definitions("Prismatic Ward") {
        let mut game = game();
        let first = creature(&mut game, A);
        let host = enter(&mut game, &definition);
        assert!(game.battlefield.contains(&host));
        assert_eq!(game.object(host).unwrap().attached_to, Some(AttachmentTarget::Object(first)));
        assert_eq!(game.chosen_color(host), Some(Color::Blue));
        let second = creature(&mut game, B);
        let source = creature(&mut game, B);
        change(&mut game, source, Modification::SetColors(ColorSet::BLUE));
        game.set_chosen_color(first, Color::Red);
        assert!(game.attach_object_to_target(host, AttachmentTarget::Object(first)));
        assert_eq!(damage(&mut game, source, DamageTarget::Object(first), false, false, None), 0);
        assert_eq!(damage(&mut game, source, DamageTarget::Object(second), false, false, None), 3);
        let blue = snapshot(&game, source);
        change(&mut game, source, Modification::SetColors(ColorSet::RED));
        assert_eq!(damage(&mut game, source, DamageTarget::Object(first), false, false, Some(&blue)), 3);
        assert!(game.attach_object_to_target(host, AttachmentTarget::Object(second)));
        change(&mut game, source, Modification::SetColors(ColorSet::BLUE));
        let departed_blue = snapshot(&game, source);
        game.move_object_by_effect(source, Zone::Graveyard).unwrap();
        assert_eq!(damage(&mut game, source, DamageTarget::Object(second), false, false, Some(&departed_blue)), 0);
        assert_eq!(damage(&mut game, source, DamageTarget::Object(first), false, false, Some(&departed_blue)), 3);
        game.phase_out(host);
        assert_eq!(damage(&mut game, source, DamageTarget::Object(second), false, false, Some(&departed_blue)), 3);
    }
}

#[test]
fn gideons_intervention_keeps_its_actual_name_choice_cast_prohibition_and_recipient_union() {
    for definition in definitions("Gideon's Intervention") {
        let mut game = game();
        let host = enter(&mut game, &definition);
        assert_eq!(game.chosen_named_option(host), Some("Lightning Bolt"));
        let named = compile_to_runtime_definition("Lightning Bolt", "Mana cost: {0}\nType: Artifact", false).unwrap();
        let wrong_name = compile_to_runtime_definition("Unlisted Bolt", "Mana cost: {0}\nType: Artifact", false).unwrap();
        let own = creature(&mut game, A); let other = creature(&mut game, B);
        for zone in [Zone::Battlefield, Zone::Stack, Zone::Graveyard] {
            let source = game.create_object_from_definition(&named, B, zone);
            assert_eq!(damage(&mut game, source, DamageTarget::Object(own), false, false, None), 0);
            assert_eq!(damage(&mut game, source, DamageTarget::Player(A), false, false, None), 0);
            assert_eq!(damage(&mut game, source, DamageTarget::Object(other), false, false, None), 3);
            assert_eq!(damage(&mut game, source, DamageTarget::Player(B), false, false, None), 3);
            let wrong = game.create_object_from_definition(&wrong_name, B, zone);
            assert_eq!(damage(&mut game, wrong, DamageTarget::Player(A), false, false, None), 3);
        }
        let a_spell = game.create_object_from_definition(&named, A, Zone::Hand);
        let b_spell = game.create_object_from_definition(&named, B, Zone::Hand);
        let can_cast = |game: &GameState, player, id| compute_legal_actions(game, player).unwrap().iter()
            .any(|action| matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == id));
        main_phase(&mut game, A); assert!(can_cast(&game, A, a_spell));
        main_phase(&mut game, B); assert!(!can_cast(&game, B, b_spell));
        game.set_current_controller(host, B).unwrap();
        assert_eq!(game.chosen_named_option(host), Some("Lightning Bolt"));
        assert!(can_cast(&game, B, b_spell));
        main_phase(&mut game, A); assert!(!can_cast(&game, A, a_spell));
        game.phase_out(host); assert!(can_cast(&game, A, a_spell));
    }
}

#[test]
fn artifact_ward_has_all_four_lines_and_distinguishes_artifact_spells_from_their_abilities() {
    for definition in definitions("Artifact Ward") {
        let mut game = game();
        let host = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let protected = creature(&mut game, A); let other = creature(&mut game, A);
        assert!(game.attach_object_to_target(host, AttachmentTarget::Object(protected)));
        let artifact = object(&mut game, B, Zone::Battlefield, vec![CardType::Artifact, CardType::Creature], vec![]);
        let own_artifact = object(&mut game, A, Zone::Battlefield, vec![CardType::Artifact], vec![]);
        let ordinary = creature(&mut game, B);
        assert!(!ironsmith::rules::combat::can_block(game.object(protected).unwrap(), game.object(artifact).unwrap(), &game));
        assert!(ironsmith::rules::combat::can_block(game.object(protected).unwrap(), game.object(ordinary).unwrap(), &game));
        assert!(!ironsmith::targeting::can_target_object(&game, protected, artifact, B).is_legal());
        assert!(!ironsmith::targeting::can_target_object(&game, protected, own_artifact, A).is_legal());
        assert!(ironsmith::targeting::can_target_object(&game, other, artifact, A).is_legal());
        assert!(ironsmith::targeting::can_target_object(&game, protected, ordinary, B).is_legal());
        let spell = object(&mut game, A, Zone::Stack, vec![CardType::Artifact], vec![]);
        assert!(ironsmith::targeting::can_target_object(&game, protected, spell, A).is_legal());
        for source in [artifact, spell] {
            assert_eq!(damage(&mut game, source, DamageTarget::Object(protected), false, false, None), 0);
            assert_eq!(damage(&mut game, source, DamageTarget::Object(other), false, false, None), 3);
        }
        let old = snapshot(&game, artifact);
        change(&mut game, artifact, Modification::RemoveCardTypes(vec![CardType::Artifact]));
        assert!(ironsmith::targeting::can_target_object(&game, protected, artifact, A).is_legal());
        assert_eq!(damage(&mut game, artifact, DamageTarget::Object(protected), false, false, Some(&old)), 3);
        change(&mut game, artifact, Modification::AddCardTypes(vec![CardType::Artifact]));
        let departed_artifact = snapshot(&game, artifact);
        game.move_object_by_effect(artifact, Zone::Graveyard).unwrap();
        assert_eq!(damage(&mut game, artifact, DamageTarget::Object(protected), false, false, Some(&departed_artifact)), 0);
        assert!(game.attach_object_to_target(host, AttachmentTarget::Object(other)));
        assert_eq!(damage(&mut game, spell, DamageTarget::Object(protected), false, false, None), 3);
        assert_eq!(damage(&mut game, spell, DamageTarget::Object(other), false, false, None), 0);
        game.phase_out(host);
        assert_eq!(damage(&mut game, spell, DamageTarget::Object(other), false, false, None), 3);
    }
}

#[test]
fn wall_of_vapor_uses_the_current_blocked_by_direction_for_all_damage_and_retains_defender() {
    for definition in definitions("Wall of Vapor") {
        let mut game = game();
        let host = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let attacker = creature(&mut game, B); let other = creature(&mut game, B);
        assert!(game.current_has_static_ability_id(host, StaticAbilityId::Defender));
        change(&mut game, host, Modification::AddAbility(StaticAbility::haste()));
        assert!(!ironsmith::rules::combat::can_attack(game.object(host).unwrap(), &game));
        let mut combat = ironsmith::combat_state::CombatState::default();
        combat.attackers.push(ironsmith::combat_state::AttackerInfo {
            creature: attacker, target: ironsmith::combat_state::AttackTarget::Player(A),
        });
        combat.blockers.insert(attacker, vec![host]);
        game.combat = Some(combat);
        for combat_damage in [false, true] {
            assert_eq!(damage(&mut game, attacker, DamageTarget::Object(host), combat_damage, false, None), 0);
            assert_eq!(damage(&mut game, other, DamageTarget::Object(host), combat_damage, false, None), 3);
            assert_eq!(damage(&mut game, host, DamageTarget::Object(attacker), combat_damage, false, None), 3);
            assert_eq!(damage(&mut game, attacker, DamageTarget::Object(host), combat_damage, true, None), 3);
        }
        game.combat.as_mut().unwrap().blockers.clear();
        assert_eq!(damage(&mut game, attacker, DamageTarget::Object(host), false, false, None), 3);
    }
}

#[test]
fn incomplete_damage_evidence_rolls_back_the_whole_native_effect_before_retry() {
    for definition in definitions("Light of Sanction") {
        let mut game = game();
        game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let target = creature(&mut game, A);
        let absent = ObjectId::new();
        let unrelated = creature(&mut game, B);
        let wrong = snapshot(&game, unrelated);
        let effect = Effect::new(SequenceEffect::new(vec![Effect::gain_life(4),
            Effect::new(DealDamageEffect::new(3, ChooseSpec::SpecificObject(target)))]));
        for with_wrong in [false, true] {
            game.take_pending_trigger_events();
            let life = game.player(A).unwrap().life;
            let mut dm = SelectFirstDecisionMaker;
            let mut context = EffectContext::new(absent, A, &mut dm);
            if with_wrong { context.source_snapshot = Some(wrong.clone()); }
            assert!(matches!(execute_effect(&mut game, &effect, &mut context), Err(ExecutionError::IncompleteEvidence(_))));
            assert_eq!(game.player(A).unwrap().life, life);
            assert_eq!(game.damage_on(target), 0);
            assert!(game.take_pending_trigger_events().is_empty());
        }
        let source = creature(&mut game, A); let exact = snapshot(&game, source);
        game.move_object_by_effect(source, Zone::Graveyard).unwrap();
        let life = game.player(A).unwrap().life;
        let mut dm = SelectFirstDecisionMaker;
        let mut context = EffectContext::new(source, A, &mut dm).with_source_snapshot(exact);
        execute_effect(&mut game, &effect, &mut context).unwrap();
        assert_eq!(game.player(A).unwrap().life, life + 4);
        assert_eq!(game.damage_on(target), 0);
    }
}
