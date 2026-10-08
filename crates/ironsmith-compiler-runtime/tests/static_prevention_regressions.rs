//! Current-main prevention regressions. All scenarios are SOURCE ONLY / UNRUN.
use ironsmith::ability::AbilityKind;
use ironsmith::card::{CardBuilder, PowerToughness};
use ironsmith::cards::CardDefinition;
use ironsmith::color::ColorSet;
use ironsmith::continuous::{EffectTarget, Modification};
use ironsmith::effect::Until;
use ironsmith::effects::{ApplyContinuousEffect, EffectContext, EffectExecutor};
use ironsmith::events::cause::EventCause;
use ironsmith::events::processing::process_damage_assignments_with_event_with_source_snapshot_opts;
use ironsmith::events::{DamagePreventedEvent, DamageTarget};
use ironsmith::object::AttachmentTarget;
use ironsmith::snapshot::ObjectSnapshot;
use ironsmith::{CardId, CardType, GameState, ObjectId, PlayerId, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_core::{ObjectFilter, PlayerFilter, StaticAbilityPayload, StaticDamagePreventionAmount};
use ironsmith_runtime_catalog::artifact_materializer::{
    encode_runtime_definition, materialize_artifact, materialize_definition,
};

const A: PlayerId = PlayerId::from_index(0);
const B: PlayerId = PlayerId::from_index(1);
const AURAS: [&str; 5] = ["Candletrap", "Demonic Torment", "Defang", "Muzzle", "Temporal Isolation"];

fn rows() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!("../../../fixtures/static_prevention_regressions.json.fixture")).unwrap()
}

fn definitions(name: &str) -> [CardDefinition; 4] {
    let row = rows().into_iter().find(|row| row["name"] == name).unwrap();
    let mut text = format!("Mana cost: {}\nType: {}\n", row["mana_cost"].as_str().unwrap(),
        row["type_line"].as_str().unwrap());
    if let (Some(power), Some(toughness)) = (row["power"].as_str(), row["toughness"].as_str()) {
        text.push_str(&format!("Power/Toughness: {power}/{toughness}\n"));
    }
    text.push_str(row["oracle_text"].as_str().unwrap());
    definitions_from_text(name, &text)
}

fn definitions_from_text(name: &str, text: &str) -> [CardDefinition; 4] {
    let (direct, loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_runtime_definition(name, text, false));
    let direct = direct.unwrap_or_else(|error| panic!("direct {name}: {error}"));
    assert!(!loss.is_lossy(), "direct {name}: {}", loss.reasons_text());
    let (compiled, loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_artifact(name, text, false));
    let (artifact, artifact_direct) = compiled.unwrap_or_else(|error| panic!("artifact {name}: {error}"));
    assert!(!loss.is_lossy(), "artifact {name}: {}", loss.reasons_text());
    artifact.validate().unwrap();
    let decoded = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(decoded, artifact);
    decoded.validate().unwrap();
    let restored = materialize_artifact(&decoded).unwrap();
    // Re-encode actual runtime objects, not the already compiled artifact.
    let native_wire = encode_runtime_definition(direct.clone()).unwrap();
    let native_wire = serde_json::from_slice(&serde_json::to_vec(&native_wire).unwrap()).unwrap();
    let native = materialize_definition(native_wire).unwrap();
    let result = [direct, artifact_direct, restored, native];
    let rendered = ironsmith_text::canonical_compiled_lines(&result[0]);
    for definition in &result {
        assert_eq!(definition.card.name, name);
        assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(definition), "{name}");
        assert_eq!(ironsmith_text::canonical_compiled_lines(definition), rendered, "full-body {name}");
    }
    result
}

fn game() -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.active_player = A;
    game.turn.priority_player = Some(A);
    game.turn.phase = ironsmith::Phase::FirstMain;
    game.turn.step = None;
    game
}

fn object(game: &mut GameState, owner: PlayerId, types: Vec<CardType>, colors: ColorSet, power: i32) -> ObjectId {
    let card = CardBuilder::new(CardId::new(), "Unlisted damage witness")
        .card_types(types).color_indicator(colors).power_toughness(PowerToughness::fixed(power, 10)).build();
    game.create_object_from_card(&card, owner, Zone::Battlefield)
}

fn creature(game: &mut GameState, owner: PlayerId, power: i32) -> ObjectId {
    object(game, owner, vec![CardType::Creature], ColorSet::COLORLESS, power)
}

fn change(game: &mut GameState, target: ObjectId, modification: Modification) {
    ApplyContinuousEffect::new(EffectTarget::Specific(target), modification, Until::Forever)
        .execute(game, &mut EffectContext::new_default(target, A)).unwrap();
}

fn snapshot(game: &GameState, source: ObjectId) -> ObjectSnapshot {
    ObjectSnapshot::from_object_with_calculated_characteristics(game.object(source).unwrap(), game)
}

fn damage(game: &mut GameState, source: ObjectId, target: DamageTarget, amount: u32,
    combat: bool, unpreventable: bool, lki: Option<&ObjectSnapshot>) -> (u32, Vec<(u32, ObjectId, PlayerId)>)
{
    game.take_pending_trigger_events();
    let result = process_damage_assignments_with_event_with_source_snapshot_opts(
        game, source, target, amount, combat, unpreventable, EventCause::effect(), lki).unwrap();
    let remaining = result.assignments.iter().map(|assignment| assignment.amount).sum();
    let events = game.take_pending_trigger_events().into_iter().filter_map(|event| {
        event.downcast::<DamagePreventedEvent>()
            .map(|event| (event.amount, event.prevention_source, event.prevention_controller))
    }).collect();
    (remaining, events)
}

#[test]
fn all_sixteen_complete_bodies_keep_canonical_payloads_and_secondary_clauses() {
    let rows = rows();
    assert_eq!(rows.len(), 16);
    assert_eq!(rows.iter().filter(|row| row["subset"] == "fixed_to_you_regression").count(), 11);
    for row in rows {
        let name = row["name"].as_str().unwrap();
        for definition in definitions(name) {
            let statics: Vec<_> = definition.abilities.iter().filter_map(|ability| match &ability.kind {
                AbilityKind::Static(ability) => ability.canonical_model(), _ => None,
            }).collect();
            if AURAS.contains(&name) {
                let spec = statics.iter().find_map(|ability| match &ability.payload {
                    StaticAbilityPayload::PreventMatchingDamage(spec) => Some(spec), _ => None,
                }).expect("one Aura-owned damage rule");
                assert_eq!(spec.source_filter.tagged_constraints, vec![ironsmith_core::TaggedObjectConstraint {
                    tag: "enchanted".into(),
                    relation: ironsmith_core::TaggedOpbjectRelation::IsTaggedObject,
                }]);
                assert!(spec.source_filter.with_attached_object.is_none());
                assert_eq!(spec.combat_only, matches!(name, "Candletrap" | "Demonic Torment"));
                assert!(!spec.noncombat_only);
                assert_eq!(spec.amount, StaticDamagePreventionAmount::All);
                assert_eq!(spec.target_player_filter, Some(PlayerFilter::Any));
                assert_eq!(spec.target_object_filter, Some(ObjectFilter::permanent()));
            } else {
                let amount = statics.iter().find_map(|ability| match &ability.payload {
                    StaticAbilityPayload::PreventDamageToYouFromSourceFilter { amount, .. } => Some(*amount),
                    _ => None,
                }).expect("existing canonical fixed-to-you payload");
                assert_eq!(amount, if name.starts_with("Sphere of") && name != "Sphere of Purity" { 2 } else { 1 });
            }
            let rendered = ironsmith_text::canonical_compiled_lines(&definition).join(" ").to_ascii_lowercase();
            let secondary: &[&str] = match name {
                "Guardian Seraph" => &["flying"],
                "Orbs of Warding" => &["you have hexproof"],
                "Heart-Shaped Herb" => &["sacrifice", "you may", "if you do", "return", "owner", "+1/+1 counters", "monarch"],
                "Candletrap" => &["enchant creature", "defender", "sacrifice", "exile enchanted creature", "different powers"],
                "Demonic Torment" => &["enchant creature", "can't attack"],
                "Temporal Isolation" => &["flash", "enchant creature", "shadow"],
                "Defang" | "Muzzle" => &["enchant creature"],
                _ => &[],
            };
            for marker in secondary { assert!(rendered.contains(*marker), "{name} lost {marker}: {rendered}"); }
            if matches!(name, "Candletrap" | "Heart-Shaped Herb") {
                assert_eq!(definition.abilities.iter().filter(|ability| matches!(ability.kind, AbilityKind::Activated(_))).count(), 1);
            }
        }
    }
}

#[test]
fn all_eleven_fixed_rules_preserve_source_recipient_controller_amount_and_prevention() {
    for (name, types, color, amount, opponents_only) in [
        ("Guardian Seraph", vec![CardType::Creature], ColorSet::COLORLESS, 1, true),
        ("Heart-Shaped Herb", vec![CardType::Creature], ColorSet::COLORLESS, 1, true),
        ("Protection of the Hekma", vec![CardType::Creature], ColorSet::COLORLESS, 1, true),
        ("Orbs of Warding", vec![CardType::Creature], ColorSet::COLORLESS, 1, false),
        ("Sphere of Purity", vec![CardType::Artifact], ColorSet::COLORLESS, 1, false),
        ("Sphere of Duty", vec![CardType::Creature], ColorSet::GREEN, 2, false),
        ("Sphere of Grace", vec![CardType::Creature], ColorSet::BLACK, 2, false),
        ("Sphere of Law", vec![CardType::Creature], ColorSet::RED, 2, false),
        ("Sphere of Reason", vec![CardType::Creature], ColorSet::BLUE, 2, false),
        ("Sphere of Truth", vec![CardType::Creature], ColorSet::WHITE, 2, false),
        ("Urza's Armor", vec![CardType::Creature], ColorSet::COLORLESS, 1, false),
    ] { for definition in definitions(name) {
        let mut game = game();
        let host = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let opposing = object(&mut game, B, types.clone(), color, 2);
        let own = object(&mut game, A, types.clone(), color, 2);
        for combat in [false, true] {
            assert_eq!(damage(&mut game, opposing, DamageTarget::Player(A), 4, combat, false, None), (4 - amount, vec![(amount, host, A)]), "{name}");
            assert_eq!(damage(&mut game, opposing, DamageTarget::Player(B), 4, combat, false, None), (4, vec![]));
            assert_eq!(damage(&mut game, opposing, DamageTarget::Object(own), 4, combat, false, None), (4, vec![]));
            assert_eq!(damage(&mut game, opposing, DamageTarget::Player(A), 4, combat, true, None), (4, vec![]));
        }
        assert_eq!(damage(&mut game, opposing, DamageTarget::Player(A), 1, false, false, None), (0, vec![(1, host, A)]));
        assert_eq!(damage(&mut game, own, DamageTarget::Player(A), 4, false, false, None).0, if opponents_only { 4 } else { 4 - amount });
        if color != ColorSet::COLORLESS {
            let wrong_color = creature(&mut game, B, 2);
            assert_eq!(damage(&mut game, wrong_color, DamageTarget::Player(A), 4, false, false, None), (4, vec![]));
        }
        if matches!(name, "Orbs of Warding" | "Sphere of Purity") {
            let wrong_type = object(&mut game, B, vec![CardType::Enchantment], ColorSet::COLORLESS, 0);
            assert_eq!(damage(&mut game, wrong_type, DamageTarget::Player(A), 4, false, false, None), (4, vec![]));
        }
        game.set_current_controller(host, B).unwrap();
        assert_eq!(damage(&mut game, own, DamageTarget::Player(B), 4, false, false, None), (4 - amount, vec![(amount, host, B)]));
        assert_eq!(damage(&mut game, own, DamageTarget::Player(A), 4, false, false, None), (4, vec![]));
        game.move_object_by_effect(host, Zone::Graveyard).unwrap();
        assert_eq!(damage(&mut game, own, DamageTarget::Player(B), 4, false, false, None), (4, vec![]));
    } }
}

#[test]
fn fixed_source_filters_prefer_live_properties_and_use_only_exact_departed_lki() {
    for definition in definitions("Sphere of Law") {
        let mut game = game();
        let sphere = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let source = object(&mut game, B, vec![CardType::Creature], ColorSet::RED, 2);
        let red = snapshot(&game, source);
        change(&mut game, source, Modification::SetColors(ColorSet::BLUE));
        assert_eq!(damage(&mut game, source, DamageTarget::Player(A), 3, false, false, Some(&red)), (3, vec![]), "current nonmatch cannot fall back to old red LKI");
        change(&mut game, source, Modification::SetColors(ColorSet::RED));
        let red = snapshot(&game, source);
        let graveyard = game.move_object_by_effect(source, Zone::Graveyard).unwrap();
        assert_eq!(damage(&mut game, source, DamageTarget::Player(A), 3, false, false, Some(&red)), (1, vec![(2, sphere, A)]));
        let returned = game.move_object_by_effect(graveyard, Zone::Battlefield).unwrap();
        change(&mut game, returned, Modification::SetColors(ColorSet::BLUE));
        assert_eq!(damage(&mut game, returned, DamageTarget::Player(A), 3, false, false, Some(&red)), (3, vec![]), "the new incarnation must use its own current properties");
    }
    for definition in definitions("Protection of the Hekma") {
        let mut game = game();
        let shield = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let source = creature(&mut game, B, 2);
        let opposing = snapshot(&game, source);
        game.set_current_controller(source, A).unwrap();
        assert_eq!(damage(&mut game, source, DamageTarget::Player(A), 3, false, false, Some(&opposing)), (3, vec![]));
        game.set_current_controller(source, B).unwrap();
        assert_eq!(damage(&mut game, source, DamageTarget::Player(A), 3, false, false, None), (2, vec![(1, shield, A)]));
    }
}

#[test]
fn aura_prevention_is_live_aura_owned_and_survives_creature_ability_loss() {
    for name in AURAS { for definition in definitions(name) {
        let mut game = game();
        let aura = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let first = creature(&mut game, B, 2);
        let second = creature(&mut game, A, 2);
        assert!(game.attach_object_to_target(aura, AttachmentTarget::Object(first)));
        let combat_only = matches!(name, "Candletrap" | "Demonic Torment");
        for target in [DamageTarget::Player(A), DamageTarget::Player(B), DamageTarget::Object(second)] {
            assert_eq!(damage(&mut game, first, target, 3, true, false, None), (0, vec![(3, aura, A)]));
            assert_eq!(damage(&mut game, first, target, 3, true, true, None), (3, vec![]));
            assert_eq!(damage(&mut game, first, target, 3, false, false, None).0, if combat_only { 3 } else { 0 });
        }
        assert_eq!(damage(&mut game, second, DamageTarget::Object(first), 3, true, false, None), (3, vec![]));
        change(&mut game, first, Modification::RemoveAllAbilities);
        assert_eq!(damage(&mut game, first, DamageTarget::Player(A), 3, true, false, None), (0, vec![(3, aura, A)]));
        game.set_current_controller(aura, B).unwrap();
        assert_eq!(damage(&mut game, first, DamageTarget::Player(A), 3, true, false, None), (0, vec![(3, aura, B)]));
        let old_attachment = snapshot(&game, first);
        assert!(game.attach_object_to_target(aura, AttachmentTarget::Object(second)));
        assert_eq!(damage(&mut game, first, DamageTarget::Player(A), 3, true, false, Some(&old_attachment)), (3, vec![]));
        assert_eq!(damage(&mut game, second, DamageTarget::Player(A), 3, true, false, None), (0, vec![(3, aura, B)]));
        let graveyard = game.move_object_by_effect(first, Zone::Graveyard).unwrap();
        assert_eq!(damage(&mut game, first, DamageTarget::Player(A), 3, true, false, None), (3, vec![]), "a departed non-host needs no LKI to reject this Aura rule");
        assert_eq!(damage(&mut game, first, DamageTarget::Player(A), 3, true, false, Some(&old_attachment)), (3, vec![]), "a departed source's snapshot cannot revive the old attachment");
        let returned = game.move_object_by_effect(graveyard, Zone::Battlefield).unwrap();
        assert_ne!(returned, first);
        assert!(game.attach_object_to_target(aura, AttachmentTarget::Object(returned)));
        assert_eq!(damage(&mut game, first, DamageTarget::Player(A), 3, true, false, Some(&old_attachment)), (3, vec![]), "old attachment LKI cannot select a new incarnation");
        assert_eq!(damage(&mut game, returned, DamageTarget::Player(A), 3, true, false, None), (0, vec![(3, aura, B)]));
        change(&mut game, aura, Modification::RemoveAllAbilities);
        assert_eq!(damage(&mut game, returned, DamageTarget::Player(A), 3, true, false, None), (3, vec![]));
    } }
}

#[test]
fn aura_departure_and_return_do_not_retain_the_old_attachment() {
    for definition in definitions("Defang") {
        let mut game = game();
        let aura = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let source = creature(&mut game, B, 2);
        assert!(game.attach_object_to_target(aura, AttachmentTarget::Object(source)));
        game.phase_out(aura);
        assert_eq!(damage(&mut game, source, DamageTarget::Player(A), 3, true, false, None), (3, vec![]));
        game.phase_in(aura);
        assert_eq!(damage(&mut game, source, DamageTarget::Player(A), 3, true, false, None), (0, vec![(3, aura, A)]));
        let source_lki = snapshot(&game, source);
        game.phase_out(source);
        assert!(game.is_phased_out(aura));
        assert_eq!(damage(&mut game, source, DamageTarget::Player(A), 3, true, false, Some(&source_lki)), (3, vec![]));
        game.phase_in(source);
        assert!(!game.is_phased_out(aura));
        assert_eq!(damage(&mut game, source, DamageTarget::Player(A), 3, true, false, None), (0, vec![(3, aura, A)]));
        assert!(game.detach_object_from_current_target(aura));
        assert_eq!(damage(&mut game, source, DamageTarget::Player(A), 3, true, false, None), (3, vec![]));
        assert!(game.attach_object_to_target(aura, AttachmentTarget::Object(source)));
        let graveyard = game.move_object_by_effect(aura, Zone::Graveyard).unwrap();
        assert_eq!(damage(&mut game, source, DamageTarget::Player(A), 3, true, false, None), (3, vec![]));
        let returned = game.move_object_by_effect(graveyard, Zone::Battlefield).unwrap();
        assert_ne!(returned, aura);
        assert_eq!(damage(&mut game, source, DamageTarget::Player(A), 3, true, false, None), (3, vec![]));
        assert!(game.attach_object_to_target(returned, AttachmentTarget::Object(source)));
        assert_eq!(damage(&mut game, source, DamageTarget::Player(A), 3, true, false, None), (0, vec![(3, returned, A)]));
    }
}

#[test]
fn source_relative_attachment_survives_subtype_changes_but_not_detachment() {
    for definition in definitions("Defang") {
        let mut game = game();
        let aura = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let source = creature(&mut game, B, 2);
        assert!(game.attach_object_to_target(aura, AttachmentTarget::Object(source)));
        let old_attachment = snapshot(&game, source);
        change(&mut game, aura, Modification::RemoveSubtypes(vec![ironsmith::types::Subtype::Aura]));
        assert!(!game.current_has_subtype(aura, ironsmith::types::Subtype::Aura));
        assert_eq!(damage(&mut game, source, DamageTarget::Player(A), 3, true, false, None), (0, vec![(3, aura, A)]), "the authored reference follows the exact attachment regardless of Aura subtype");
        assert!(game.detach_object_from_current_target(aura));
        let other_aura = game.create_object_from_definition(&definition, B, Zone::Battlefield);
        assert!(game.attach_object_to_target(other_aura, AttachmentTarget::Object(source)));
        assert_eq!(damage(&mut game, source, DamageTarget::Player(A), 3, true, false, Some(&old_attachment)), (0, vec![(3, other_aura, B)]), "another Aura cannot turn the detached source's identity tag into generic enchanted matching");
    }
}

#[test]
fn explicitly_granted_prevention_keeps_the_recipient_as_its_source() {
    for definition in definitions_from_text("Unlisted granted shield", "Mana cost: {W}\nType: Enchantment — Aura\nEnchant creature\nEnchanted creature has \"Prevent all damage that would be dealt by this creature.\"") {
        let mut game = game();
        let aura = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let bearer = creature(&mut game, B, 2);
        assert!(game.attach_object_to_target(aura, AttachmentTarget::Object(bearer)));
        assert_eq!(damage(&mut game, bearer, DamageTarget::Player(A), 3, true, false, None), (0, vec![(3, bearer, B)]));
        change(&mut game, bearer, Modification::RemoveAllAbilities);
        assert_eq!(damage(&mut game, bearer, DamageTarget::Player(A), 3, true, false, None), (3, vec![]));
    }
}

#[test]
fn conditional_otherwise_prevention_retains_its_live_attached_condition() {
    for definition in definitions_from_text("Unlisted conditional shield", "Mana cost: {W}\nType: Enchantment — Aura\nEnchant creature\nEnchanted creature has double strike as long as it's an enchantment. Otherwise, prevent all damage that would be dealt by enchanted creature.") {
        let mut game = game();
        let aura = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let bearer = creature(&mut game, B, 2);
        assert!(game.attach_object_to_target(aura, AttachmentTarget::Object(bearer)));
        assert_eq!(damage(&mut game, bearer, DamageTarget::Player(A), 3, true, false, None), (0, vec![(3, aura, A)]));
        change(&mut game, bearer, Modification::AddCardTypes(vec![CardType::Enchantment]));
        assert!(has_static(&game, bearer, ironsmith::static_abilities::StaticAbilityId::DoubleStrike));
        assert_eq!(damage(&mut game, bearer, DamageTarget::Player(A), 3, true, false, None), (3, vec![]));
    }
}

fn has_static(game: &GameState, object: ObjectId, id: ironsmith::static_abilities::StaticAbilityId) -> bool {
    game.current_abilities(object).unwrap().iter().any(|ability|
        matches!(&ability.kind, AbilityKind::Static(ability) if ability.id() == id))
}

#[test]
fn secondary_keywords_restrictions_and_player_hexproof_remain_live() {
    use ironsmith::static_abilities::StaticAbilityId;
    for definition in definitions("Guardian Seraph") {
        let mut game = game();
        let seraph = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        assert!(has_static(&game, seraph, StaticAbilityId::Flying));
    }
    for definition in definitions("Orbs of Warding") {
        let mut game = game();
        let orbs = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let enemy = creature(&mut game, B, 2);
        let friendly = creature(&mut game, A, 2);
        game.update_cant_effects();
        assert!(!game.can_target_player_from_source(A, enemy));
        assert!(game.can_target_player_from_source(A, friendly));
        game.move_object_by_effect(orbs, Zone::Graveyard).unwrap();
        game.update_cant_effects();
        assert!(game.can_target_player_from_source(A, enemy));
    }
    for name in ["Candletrap", "Demonic Torment", "Temporal Isolation"] {
        for definition in definitions(name) {
            let mut game = game();
            let aura = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let bearer = creature(&mut game, B, 2);
            game.remove_summoning_sickness(bearer);
            assert!(game.attach_object_to_target(aura, AttachmentTarget::Object(bearer)));
            if name == "Temporal Isolation" {
                assert!(has_static(&game, aura, StaticAbilityId::Flash));
                assert!(has_static(&game, bearer, StaticAbilityId::Shadow));
            } else {
                if name == "Candletrap" { assert!(has_static(&game, bearer, StaticAbilityId::Defender)); }
                assert!(!ironsmith::rules::combat::can_attack(game.object(bearer).unwrap(), &game));
            }
        }
    }
}

fn activation(game: &GameState, source: ObjectId) -> Option<ironsmith::decision::LegalAction> {
    ironsmith::decision::compute_legal_actions(game, A).unwrap().into_iter().find(|action|
        matches!(action, ironsmith::decision::LegalAction::ActivateAbility { source: id, .. } if *id == source))
}

fn activate(game: &mut GameState, source: ObjectId) {
    use ironsmith::decision::SelectFirstDecisionMaker;
    use ironsmith::game_loop::{PriorityLoopState, PriorityResponse, apply_decision_context_with_dm, apply_priority_response_with_dm};
    game.turn.priority_player = Some(A);
    let action = activation(game, source).expect("full-body activation must be legal");
    let mut state = PriorityLoopState::new(2);
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    let mut dm = SelectFirstDecisionMaker;
    let mut progress = apply_priority_response_with_dm(game, &mut queue, &mut state,
        &PriorityResponse::PriorityAction(action), &mut dm).unwrap();
    for _ in 0..40 {
        if !state.has_pending_action() { break; }
        let ironsmith::GameProgress::NeedsDecisionCtx(context) = progress else { panic!("{progress:?}"); };
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &context, &mut dm).unwrap();
    }
    assert!(!state.has_pending_action());
    assert_eq!(game.stack.len(), 1);
    assert_eq!(game.stack[0].controller, A);
}

#[test]
fn candletrap_checks_distinct_powers_at_activation_and_keeps_the_sacrificed_auras_attachment() {
    for definition in definitions("Candletrap") {
        let mut game = game();
        let aura = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let bearer = creature(&mut game, B, 2);
        let stable = game.object(bearer).unwrap().stable_id;
        assert!(game.attach_object_to_target(aura, AttachmentTarget::Object(bearer)));
        creature(&mut game, A, 1);
        creature(&mut game, A, 1);
        creature(&mut game, A, 2);
        game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::White, 3);
        assert!(activation(&game, aura).is_none(), "three creatures with only two powers cannot activate coven");
        let third_power = creature(&mut game, A, 3);
        activate(&mut game, aura);
        assert!(game.object(aura).is_none(), "Aura sacrifice is paid before resolution");
        game.move_object_by_effect(third_power, Zone::Graveyard).unwrap();
        ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut ironsmith::decision::SelectFirstDecisionMaker).unwrap();
        let exiled = game.find_object_by_stable_id(stable).unwrap();
        assert_eq!(game.object(exiled).unwrap().zone, Zone::Exile, "activation restriction is not a resolution condition");
    }
}

struct HerbChoices { accept: bool, sacrifice: ObjectId }
impl ironsmith::decision::DecisionMaker for HerbChoices {
    fn decide_boolean(&mut self, _: &GameState, context: &ironsmith::decisions::context::BooleanContext) -> bool {
        self.accept && context.can_accept
    }
    fn decide_objects(&mut self, _: &GameState, context: &ironsmith::decisions::context::SelectObjectsContext) -> Vec<ObjectId> {
        assert!(context.candidates.iter().any(|candidate| candidate.id == self.sacrifice && candidate.legal));
        vec![self.sacrifice]
    }
}

#[test]
fn herb_full_activation_retains_optional_sacrifice_owner_return_counters_and_monarch() {
    for definition in definitions("Heart-Shaped Herb") { for accept in [false, true] {
        let mut game = game();
        let herb = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let sacrifice = creature(&mut game, B, 2);
        game.set_current_controller(sacrifice, A).unwrap();
        let stable = game.object(sacrifice).unwrap().stable_id;
        game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless, 2);
        activate(&mut game, herb);
        assert!(game.object(herb).is_none());
        ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut HerbChoices { accept, sacrifice }).unwrap();
        let current = game.find_object_by_stable_id(stable).unwrap();
        assert_eq!(game.object(current).unwrap().zone, Zone::Battlefield);
        if accept {
            assert_ne!(current, sacrifice);
            assert_eq!(game.current_controller(current), Some(B), "return uses the sacrificed card's owner");
            assert_eq!(game.object(current).unwrap().counters.get(&ironsmith::CounterType::PlusOnePlusOne), Some(&3));
            assert_eq!(game.monarch, Some(A));
        } else {
            assert_eq!(current, sacrifice);
            assert_eq!(game.current_controller(current), Some(A));
            assert!(game.object(current).unwrap().counters.is_empty());
            assert_eq!(game.monarch, None);
        }
    } }
}
