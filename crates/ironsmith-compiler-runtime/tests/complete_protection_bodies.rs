//! Complete frozen protection bodies. All scenarios are authored, not executed.
use ironsmith::ability::ProtectionFrom;
use ironsmith::card::{CardBuilder, PowerToughness};
use ironsmith::cards::CardDefinition;
use ironsmith::color::ColorSet;
use ironsmith::continuous::Modification;
use ironsmith::decision::{DecisionMaker, SelectFirstDecisionMaker};
use ironsmith::decisions::context::BooleanContext;
use ironsmith::effect::Until;
use ironsmith::effects::{ApplyContinuousEffect, AttachToEffect, DealDamageEffect, EffectContext, EffectExecutor};
use ironsmith::object::AttachmentTarget;
use ironsmith::rules::state_based::{StateBasedAction, check_state_based_actions};
use ironsmith::target::ChooseSpec;
use ironsmith::{CardId, CardType, CounterType, GameState, ObjectId, PlayerId, Subtype, Supertype, Target, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
const NAMES: [&str; 8] = ["Guardian of the Guildpact", "Oversoul of Dusk", "Elite Inquisitor",
    "Earnest Fellowship", "Empty-Shrine Kannushi", "Pledge of Loyalty", "Ronom Hulk", "Katilda, Dawnhart Prime"];
fn definitions(name: &str) -> [CardDefinition; 3] {
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!("../../../fixtures/complete_protection_bodies.json.fixture")).unwrap();
    let row = rows.iter().find(|row| row["name"] == name).unwrap();
    let text = row["text"].as_str().unwrap();
    let (direct, loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_runtime_definition(name, text, false));
    let direct = direct.unwrap();
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let (artifact, loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_artifact(name, text, false));
    let (artifact, _) = artifact.unwrap();
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let decoded = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(decoded, artifact);
    let restored = ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&decoded).unwrap();
    // The native route must freshly encode the actual executable objects.
    let native_wire = ironsmith_runtime_catalog::artifact_materializer::encode_runtime_definition(direct.clone()).unwrap();
    let native_wire = serde_json::from_slice(&serde_json::to_vec(&native_wire).unwrap()).unwrap();
    let native = ironsmith_runtime_catalog::artifact_materializer::materialize_definition(native_wire).unwrap();
    for definition in [&direct, &restored, &native] {
        assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(definition), "{name}");
    }
    [direct, restored, native]
}
fn game() -> GameState { GameState::new(vec!["Alice".into(), "Bob".into()], 20) }
fn witness(game: &mut GameState, owner: PlayerId, types: Vec<CardType>, subtypes: Vec<Subtype>, colors: ColorSet, snow: bool, zone: Zone) -> ObjectId {
    let card = CardBuilder::new(CardId::new(), "Protection witness").card_types(types).subtypes(subtypes)
        .supertypes(if snow { vec![Supertype::Snow] } else { vec![] })
        .color_indicator(colors).power_toughness(PowerToughness::fixed(2, 10)).build();
    game.create_object_from_card(&card, owner, zone)
}
fn creature(game: &mut GameState, owner: PlayerId, colors: ColorSet) -> ObjectId {
    witness(game, owner, vec![CardType::Creature], vec![Subtype::Bear], colors, false, Zone::Battlefield)
}
fn change(game: &mut GameState, target: ObjectId, modification: Modification) {
    ApplyContinuousEffect::with_spec(ChooseSpec::SpecificObject(target), modification, Until::Forever)
        .execute(game, &mut EffectContext::new_default(target, A)).unwrap();
    game.refresh_continuous_state().unwrap();
}
fn matching(game: &GameState, protected: ObjectId, source: ObjectId) -> bool {
    ironsmith::targeting::has_protection_from_source(game, protected, source)
}
fn attach(game: &mut GameState, aura: ObjectId, protected: ObjectId) {
    let controller = game.controller_of_id(aura).unwrap();
    AttachToEffect::new(ChooseSpec::SpecificObject(protected))
        .execute(game, &mut EffectContext::new(aura, controller, &mut SelectFirstDecisionMaker)).unwrap();
    game.refresh_continuous_state().unwrap();
}
#[test]
fn all_exact_bodies_have_independent_direct_artifact_and_native_codecs() {
    for name in NAMES { let _ = definitions(name); }
}
#[test]
fn complete_static_qualities_cover_damage_targeting_blocking_and_both_attachments() {
    let cases = [
        ("Guardian of the Guildpact", ColorSet::RED, Subtype::Bear, false, true),
        ("Guardian of the Guildpact", ColorSet::RED.union(ColorSet::BLUE), Subtype::Bear, false, false),
        ("Guardian of the Guildpact", ColorSet::COLORLESS, Subtype::Bear, false, false),
        ("Oversoul of Dusk", ColorSet::BLUE, Subtype::Bear, false, true),
        ("Oversoul of Dusk", ColorSet::BLACK, Subtype::Bear, false, true),
        ("Oversoul of Dusk", ColorSet::RED, Subtype::Bear, false, true),
        ("Oversoul of Dusk", ColorSet::GREEN.union(ColorSet::WHITE), Subtype::Bear, false, false),
        ("Elite Inquisitor", ColorSet::COLORLESS, Subtype::Vampire, false, true),
        ("Elite Inquisitor", ColorSet::COLORLESS, Subtype::Werewolf, false, true),
        ("Elite Inquisitor", ColorSet::COLORLESS, Subtype::Zombie, false, true),
        ("Elite Inquisitor", ColorSet::BLACK, Subtype::Human, false, false),
        ("Ronom Hulk", ColorSet::COLORLESS, Subtype::Bear, true, true),
        ("Ronom Hulk", ColorSet::BLUE, Subtype::Bear, false, false),
        ("Katilda, Dawnhart Prime", ColorSet::RED, Subtype::Werewolf, false, true),
        ("Katilda, Dawnhart Prime", ColorSet::RED, Subtype::Wolf, false, false),
    ];
    for (name, colors, subtype, snow, protects) in cases { for definition in definitions(name) {
        let mut game = game();
        let protected = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let source = witness(&mut game, B, vec![CardType::Creature], vec![subtype], colors, snow, Zone::Battlefield);
        assert_eq!(matching(&game, protected, source), protects, "{name}");
        assert_eq!(ironsmith::rules::combat::can_block(game.object(protected).unwrap(), game.object(source).unwrap(), &game), !protects, "{name}");
        let spell = witness(&mut game, B, vec![CardType::Instant], vec![subtype], colors, snow, Zone::Stack);
        assert_eq!(ironsmith::targeting::compute_legal_targets(&game, &ChooseSpec::target_creature(), B, Some(spell))
            .contains(&Target::Object(protected)), !protects, "{name}");
        DealDamageEffect::new(1, ChooseSpec::SpecificObject(protected))
            .execute(&mut game, &mut EffectContext::new(source, B, &mut SelectFirstDecisionMaker)).unwrap();
        assert_eq!(game.damage_on(protected), u32::from(!protects), "{name}");
        for (card_type, attachment_type) in [(CardType::Artifact, Subtype::Equipment), (CardType::Enchantment, Subtype::Aura)] {
            let attachment = if attachment_type == Subtype::Aura {
                let aura = compile_to_runtime_definition("Aura witness", "Type: Enchantment — Aura\nEnchant creature", false).unwrap();
                let id = game.create_object_from_definition(&aura, A, Zone::Battlefield);
                change(&mut game, id, Modification::SetColors(colors));
                change(&mut game, id, Modification::SetSubtypes(vec![Subtype::Aura, subtype]));
                if snow { change(&mut game, id, Modification::AddSupertypes(vec![Supertype::Snow])); }
                id
            } else {
                witness(&mut game, A, vec![card_type], vec![attachment_type, subtype], colors, snow, Zone::Battlefield)
            };
            attach(&mut game, attachment, protected);
            assert_eq!(game.object(attachment).unwrap().attached_to == Some(AttachmentTarget::Object(protected)), !protects, "{name}");
        }
        change(&mut game, protected, Modification::RemoveAllAbilities);
        assert!(!matching(&game, protected, source), "ability loss must disable the rule");
    } }
}
#[test]
fn sources_use_current_layers_and_exact_departed_source_lki() {
    for (name, modification) in [
        ("Guardian of the Guildpact", Modification::SetColors(ColorSet::RED)),
        ("Ronom Hulk", Modification::AddSupertypes(vec![Supertype::Snow])),
        ("Elite Inquisitor", Modification::SetSubtypes(vec![Subtype::Werewolf])),
    ] { for definition in definitions(name) {
        let mut game = game();
        let protected = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let source = creature(&mut game, B, ColorSet::COLORLESS);
        assert!(!matching(&game, protected, source));
        change(&mut game, source, modification.clone());
        assert!(matching(&game, protected, source));
        let snapshot = ironsmith::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(game.object(source).unwrap(), &game);
        game.phase_out(source);
        DealDamageEffect::new(2, ChooseSpec::SpecificObject(protected)).execute(&mut game,
            &mut EffectContext::new(source, B, &mut SelectFirstDecisionMaker).with_source_snapshot(snapshot.clone())).unwrap();
        assert_eq!(game.damage_on(protected), 0);
        game.phase_in(source);
        game.move_object_by_effect(source, Zone::Graveyard).unwrap();
        DealDamageEffect::new(2, ChooseSpec::SpecificObject(protected)).execute(&mut game,
            &mut EffectContext::new(source, B, &mut SelectFirstDecisionMaker).with_source_snapshot(snapshot)).unwrap();
        assert_eq!(game.damage_on(protected), 0);
    } }
}
#[test]
fn fellowship_reads_each_recipients_colors_and_obeys_grant_and_ability_lifetime() {
    for definition in definitions("Earnest Fellowship") {
        let mut game = game();
        let grant = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let protected = creature(&mut game, B, ColorSet::BLUE);
        let blue = creature(&mut game, A, ColorSet::BLUE);
        let red = creature(&mut game, A, ColorSet::RED);
        assert!(matching(&game, protected, blue));
        assert!(!matching(&game, protected, red));
        assert!(!ironsmith::rules::combat::can_block(game.object(protected).unwrap(), game.object(blue).unwrap(), &game));
        assert!(!ironsmith::targeting::compute_legal_targets(&game, &ChooseSpec::target_creature(), A, Some(blue)).contains(&Target::Object(protected)));
        DealDamageEffect::new(1, ChooseSpec::SpecificObject(protected)).execute(&mut game,
            &mut EffectContext::new(blue, A, &mut SelectFirstDecisionMaker)).unwrap();
        assert_eq!(game.damage_on(protected), 0);
        let equipment = witness(&mut game, A, vec![CardType::Artifact], vec![Subtype::Equipment], ColorSet::BLUE, false, Zone::Battlefield);
        attach(&mut game, equipment, protected);
        assert_eq!(game.object(equipment).unwrap().attached_to, None);
        change(&mut game, protected, Modification::SetColors(ColorSet::RED));
        assert!(!matching(&game, protected, blue));
        assert!(matching(&game, protected, red));
        change(&mut game, protected, Modification::SetColors(ColorSet::COLORLESS));
        assert!(!matching(&game, protected, red));
        change(&mut game, protected, Modification::SetColors(ColorSet::BLUE));
        game.phase_out(grant);
        game.refresh_continuous_state().unwrap();
        assert!(!matching(&game, protected, blue));
        game.phase_in(grant);
        game.refresh_continuous_state().unwrap();
        assert!(matching(&game, protected, blue));
        change(&mut game, grant, Modification::RemoveAllAbilities);
        assert!(!matching(&game, protected, blue));
    }
}
#[test]
fn kannushi_population_is_live_includes_self_and_excludes_phased_or_opponent_objects() {
    for definition in definitions("Empty-Shrine Kannushi") {
        let mut game = game();
        let protected = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let blue_permanent = creature(&mut game, A, ColorSet::BLUE);
        let red_permanent = creature(&mut game, B, ColorSet::RED);
        let white = creature(&mut game, B, ColorSet::WHITE);
        let blue = creature(&mut game, B, ColorSet::BLUE);
        assert!(matching(&game, protected, white), "Kannushi counts itself");
        assert!(matching(&game, protected, blue));
        assert!(!matching(&game, protected, red_permanent));
        game.phase_out(blue_permanent);
        game.refresh_continuous_state().unwrap();
        assert!(!matching(&game, protected, blue));
        game.set_current_controller(protected, B).unwrap();
        game.refresh_continuous_state().unwrap();
        assert!(matching(&game, protected, red_permanent));
        assert!(matching(&game, protected, blue));
    }
}
#[test]
fn pledge_uses_live_aura_controller_and_only_exempts_its_own_granted_protection() {
    for definition in definitions("Pledge of Loyalty") {
        let mut game = game();
        let protected = creature(&mut game, B, ColorSet::COLORLESS);
        let blue_permanent = creature(&mut game, A, ColorSet::BLUE);
        let red_permanent = creature(&mut game, B, ColorSet::RED);
        let old_aura = compile_to_runtime_definition("Earlier Aura", "Mana cost: {R}\nType: Enchantment — Aura\nEnchant creature", false).unwrap();
        let old_aura = game.create_object_from_definition(&old_aura, B, Zone::Battlefield);
        let equipment = witness(&mut game, B, vec![CardType::Artifact], vec![Subtype::Equipment], ColorSet::RED, false, Zone::Battlefield);
        attach(&mut game, old_aura, protected);
        attach(&mut game, equipment, protected);
        let aura = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        attach(&mut game, aura, protected);
        assert!(matching(&game, protected, blue_permanent));
        assert!(!matching(&game, protected, red_permanent), "the host's controller is not you");
        assert!(!check_state_based_actions(&game).contains(&StateBasedAction::AuraFallsOff(aura)));
        assert!(matching(&game, protected, aura), "retention is not blanket source immunity");
        assert!(!ironsmith::targeting::compute_legal_targets(&game, &ChooseSpec::target_creature(), A, Some(aura)).contains(&Target::Object(protected)));
        DealDamageEffect::new(1, ChooseSpec::SpecificObject(protected)).execute(&mut game,
            &mut EffectContext::new(aura, A, &mut SelectFirstDecisionMaker)).unwrap();
        assert_eq!(game.damage_on(protected), 0);
        let bound = game.current_abilities(protected).unwrap().iter().find(|ability|
            matches!(&ability.kind, ironsmith::ability::AbilityKind::Static(rule)
                if matches!(rule.protection_from(), Some(ProtectionFrom::ColorsAmong { reference_source: Some(_), .. }))))
            .unwrap().clone();
        let wire = ironsmith_runtime_catalog::artifact_materializer::encode_runtime_ability(bound).unwrap();
        let restored = ironsmith_runtime_catalog::artifact_materializer::restore_runtime_ability(
            serde_json::from_slice(&serde_json::to_vec(&wire).unwrap()).unwrap()).unwrap();
        let ironsmith::ability::AbilityKind::Static(rule) = restored.kind else { panic!("static protection"); };
        assert!(matches!(rule.protection_from(), Some(ProtectionFrom::ColorsAmong { reference_source: Some(id), .. }) if *id == aura));
        game.set_current_controller(aura, B).unwrap();
        game.refresh_continuous_state().unwrap();
        assert!(!matching(&game, protected, blue_permanent));
        assert!(matching(&game, protected, red_permanent));
        let actions = check_state_based_actions(&game);
        assert!(!actions.contains(&StateBasedAction::AuraFallsOff(aura)));
        assert!(actions.contains(&StateBasedAction::AuraFallsOff(old_aura)));
        assert!(actions.contains(&StateBasedAction::AttachmentBecomesUnattached(equipment)));
        game.phase_out(aura);
        game.refresh_continuous_state().unwrap();
        assert!(!matching(&game, protected, red_permanent));
        game.phase_in(aura);
        game.refresh_continuous_state().unwrap();
        assert!(matching(&game, protected, red_permanent));
        // A second matching Aura receives no exception from Pledge.
        let other = compile_to_runtime_definition("Other Aura", "Mana cost: {W}\nType: Enchantment — Aura\nEnchant creature", false).unwrap();
        let other = game.create_object_from_definition(&other, B, Zone::Battlefield);
        attach(&mut game, other, protected);
        assert_eq!(game.object(other).unwrap().attached_to, None);
        change(&mut game, protected, Modification::AddAbility(ironsmith::static_abilities::StaticAbility::protection(ProtectionFrom::Color(ColorSet::WHITE))));
        assert!(check_state_based_actions(&game).contains(&StateBasedAction::AuraFallsOff(aura)), "independent protection still removes Pledge");
    }
}
struct Pay(bool);
impl DecisionMaker for Pay {
    fn decide_boolean(&mut self, _: &GameState, _: &BooleanContext) -> bool { self.0 }
}
#[test]
fn ronom_complete_upkeeps_add_age_pay_the_whole_mana_cost_or_sacrifice() {
    for definition in definitions("Ronom Hulk") { for pay in [true, false] {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless, 3);
        let mut dm = Pay(pay);
        for age in 1..=2 {
            game.turn.phase = ironsmith::Phase::Beginning;
            game.turn.step = Some(ironsmith::game_state::Step::Upkeep);
            game.queue_trigger_event(Default::default(), ironsmith::triggers::TriggerEvent::new(
                ironsmith::events::phase::BeginningOfUpkeepEvent::new(A), Default::default()));
            ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game, &mut ironsmith::triggers::TriggerQueue::new(), &mut dm).unwrap();
            assert_eq!(game.stack.len(), 1);
            ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
            if !pay { assert!(!game.battlefield.contains(&source)); break; }
            assert_eq!(game.counter_count(source, CounterType::Age), age);
            assert!(game.battlefield.contains(&source));
        }
        assert_eq!(game.player(A).unwrap().mana_pool.total(), if pay { 0 } else { 3 });
        if pay {
            // Paying once or twice is not a standing waiver: the third
            // upkeep demands three mana and sacrifices an unpaid Hulk.
            game.queue_trigger_event(Default::default(), ironsmith::triggers::TriggerEvent::new(
                ironsmith::events::phase::BeginningOfUpkeepEvent::new(A), Default::default()));
            ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game,
                &mut ironsmith::triggers::TriggerQueue::new(), &mut dm).unwrap();
            assert_eq!(game.stack.len(), 1);
            ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
            assert!(!game.battlefield.contains(&source));
            assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
        }
    } }
}
#[test]
fn malformed_qualities_and_exception_tails_fail_both_public_routes() {
    for text in [
        "Protection from monocolored nonsense", "Protection from snow {R}",
        "Protection from monocolored,", "Protection from snow,",
        "Protection from blue, from black, and from red,",
        "Protection from blue, from black, and from red nonsense", "Protection from Vampires, from Werewolves, and from Zombies:",
        "Each creature has protection from each of its colors and dances.",
        "Each creature has protection from each of its colors,",
        "Each creature has , protection from each of its colors.",
        "Each creature has flying, protection from each of its colors,.",
        "This creature has protection from each color among permanents you control and draw a card.",
        "This creature has protection from each color among permanents you control,.",
        "Enchanted creature has protection from each color among permanents you control,. This effect doesn't remove this Aura.",
        "Enchanted creature has protection from each color among permanents you control. This effect doesn't remove this Aura and Equipment.",
    ] {
        assert!(compile_to_runtime_definition("Malformed protection", text, false).is_err(), "{text}");
        assert!(compile_to_artifact("Malformed protection", text, false).is_err(), "{text}");
    }
}

#[test]
fn quoted_continuous_population_grants_reject_without_recipient_provenance() {
    for text in [
        "Creatures have \"This creature has protection from each color among permanents you control.\"",
        "Creatures have flying and \"This creature has protection from each color among permanents you control.\"",
        "Enchanted creature has \"This creature has protection from each color among permanents you control.\"",
    ] {
        assert!(compile_to_runtime_definition("Unsupported continuous quoted protection", text, false).is_err(), "{text}");
        assert!(compile_to_artifact("Unsupported continuous quoted protection", text, false).is_err(), "{text}");
    }
}

#[test]
fn unavailable_damage_source_is_incomplete_evidence_and_checked_sequences_roll_back() {
    use ironsmith::effect::Effect;
    use ironsmith::effects::{ExecutionError, SequenceEffect, execute_effect};
    for name in ["Guardian of the Guildpact", "Oversoul of Dusk", "Elite Inquisitor", "Ronom Hulk", "Empty-Shrine Kannushi"] {
        for definition in definitions(name) {
            let mut game = game();
            let protected = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let absent = ObjectId::new();
            let other = creature(&mut game, B, ColorSet::RED);
            let wrong = ironsmith::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(game.object(other).unwrap(), &game);
            let sequence = Effect::new(SequenceEffect::new(vec![Effect::gain_life(3),
                Effect::new(DealDamageEffect::new(2, ChooseSpec::SpecificObject(protected)))]));
            for use_wrong in [false, true] {
                let life = game.player(B).unwrap().life;
                let mut dm = SelectFirstDecisionMaker;
                let mut context = EffectContext::new(absent, B, &mut dm);
                if use_wrong { context.source_snapshot = Some(wrong.clone()); }
                assert!(matches!(execute_effect(&mut game, &sequence, &mut context), Err(ExecutionError::IncompleteEvidence(_))));
                assert_eq!(game.damage_on(protected), 0);
                assert_eq!(game.player(B).unwrap().life, life);
            }
        }
    }
}

#[test]
fn incomplete_continuous_discovery_cannot_turn_dynamic_protection_into_legal_damage() {
    use ironsmith::effect::Effect;
    use ironsmith::effects::{ExecutionError, SequenceEffect, execute_effect};
    for name in ["Earnest Fellowship", "Empty-Shrine Kannushi"] { for definition in definitions(name) {
        let mut game = game();
        let source = creature(&mut game, B, ColorSet::WHITE);
        let owner = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let protected = if name == "Earnest Fellowship" { creature(&mut game, A, ColorSet::WHITE) } else { owner };
        assert!(matching(&game, protected, source));
        let life = game.player(B).unwrap().life;
        game.player_mut(A).unwrap().mana_pool.green = i32::MAX as u32 + 1;
        assert!(game.try_current_characteristics(protected).is_err());
        assert!(ironsmith::decision::compute_legal_actions(&game, A).is_err());
        let sequence = Effect::new(SequenceEffect::new(vec![Effect::gain_life(3),
            Effect::new(DealDamageEffect::new(2, ChooseSpec::SpecificObject(protected)))]));
        let result = execute_effect(&mut game, &sequence, &mut EffectContext::new(source, B, &mut SelectFirstDecisionMaker));
        assert!(matches!(result, Err(ExecutionError::ContinuousDiscovery(_)) | Err(ExecutionError::ResourceLimitExceeded { .. })));
        assert_eq!(game.damage_on(protected), 0);
        assert_eq!(game.player(B).unwrap().life, life);
        game.player_mut(A).unwrap().mana_pool.green = 0;
        game.refresh_continuous_state().unwrap();
        assert!(matching(&game, protected, source));
    } }
}

#[test]
fn public_protection_lines_reject_trailing_list_delimiters_without_losing_mixed_keywords() {
    for keyword in ["Protection from monocolored", "Protection from snow"] {
        let text = format!("Type: Creature — Spirit\nPower/Toughness: 2/2\n{keyword}");
        assert!(compile_to_runtime_definition("Complete protection", &text, false).is_ok());
        assert!(compile_to_artifact("Complete protection", &text, false).is_ok());
        for suffix in [",", ";", ",.", ";.", ", from red,", ", flying,", ",, flying", ",; flying", ";, flying", ";; flying"] {
            let malformed = format!("{text}{suffix}");
            assert!(compile_to_runtime_definition("Malformed complete protection", &malformed, false).is_err(), "{malformed}");
            assert!(compile_to_artifact("Malformed complete protection", &malformed, false).is_err(), "{malformed}");
        }
        let mixed = format!("{text}, flying");
        assert!(compile_to_runtime_definition("Mixed protection", &mixed, false).is_ok());
        assert!(compile_to_artifact("Mixed protection", &mixed, false).is_ok());
        for malformed in [format!("Flying,, {keyword}"), format!("Flying,; {keyword}"),
            format!("Flying, {keyword},."), format!("Flying, {keyword};.")] {
            assert!(compile_to_runtime_definition("Malformed mixed protection", &malformed, false).is_err(), "{malformed}");
            assert!(compile_to_artifact("Malformed mixed protection", &malformed, false).is_err(), "{malformed}");
        }
    }
}

#[test]
fn pledge_grant_tracks_attachment_and_ends_when_its_exact_source_stops_granting() {
    for definition in definitions("Pledge of Loyalty") { for remove_source in [false, true] {
        let mut game = game();
        let first = creature(&mut game, B, ColorSet::COLORLESS);
        let second = creature(&mut game, B, ColorSet::COLORLESS);
        let blue = creature(&mut game, A, ColorSet::BLUE);
        let aura = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        attach(&mut game, aura, first);
        assert!(matching(&game, first, blue));
        assert!(!matching(&game, second, blue));
        attach(&mut game, aura, second);
        assert!(!matching(&game, first, blue));
        assert!(matching(&game, second, blue));
        if remove_source {
            game.move_object_by_effect(aura, Zone::Graveyard).unwrap();
        } else {
            change(&mut game, aura, Modification::RemoveAllAbilities);
        }
        game.refresh_continuous_state().unwrap();
        assert!(!matching(&game, first, blue));
        assert!(!matching(&game, second, blue));
    } }
}

fn grant_definitions(quoted: bool) -> [CardDefinition; 3] {
    let grant = if quoted {
        "Target creature gains \"This creature has protection from each color among permanents you control.\" until end of turn."
    } else {
        "Target creature gains protection from each color among permanents you control until end of turn."
    };
    let text = format!("Type: Artifact\n{{T}}: {grant}");
    let (direct, loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_runtime_definition("Protection context grant", &text, false));
    let direct = direct.unwrap();
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let (artifact, loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_artifact("Protection context grant", &text, false));
    let (artifact, _) = artifact.unwrap();
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let artifact = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    let loaded = ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&artifact).unwrap();
    let native = ironsmith_runtime_catalog::artifact_materializer::encode_runtime_definition(direct.clone()).unwrap();
    let native = ironsmith_runtime_catalog::artifact_materializer::materialize_definition(
        serde_json::from_slice(&serde_json::to_vec(&native).unwrap()).unwrap()).unwrap();
    [direct, loaded, native]
}
struct GrantTarget(ObjectId);
impl DecisionMaker for GrantTarget {
    fn decide_targets(&mut self, _: &GameState, ctx: &ironsmith::decisions::context::TargetsContext) -> Vec<Target> {
        let target = Target::Object(self.0);
        assert!(ctx.requirements.iter().all(|requirement| requirement.legal_targets.contains(&target)));
        vec![target]
    }
}
fn activate_grant(game: &mut GameState, source: ObjectId, protected: ObjectId) {
    use ironsmith::decision::{LegalAction, compute_legal_actions};
    use ironsmith::game_loop::{PriorityLoopState, PriorityResponse, apply_decision_context_with_dm, apply_priority_response_with_dm};
    game.turn.active_player = A;
    game.turn.priority_player = Some(A);
    game.turn.phase = ironsmith::Phase::FirstMain;
    game.turn.step = None;
    let action = compute_legal_actions(game, A).unwrap().into_iter().find(|action|
        matches!(action, LegalAction::ActivateAbility { source: id, .. } if *id == source)).unwrap();
    let mut state = PriorityLoopState::new(2);
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    let mut dm = GrantTarget(protected);
    let mut progress = apply_priority_response_with_dm(game, &mut queue, &mut state,
        &PriorityResponse::PriorityAction(action), &mut dm).unwrap();
    for _ in 0..40 {
        if state.pending_activation.is_none() { break; }
        let ironsmith::GameProgress::NeedsDecisionCtx(context) = progress else { panic!("{progress:?}"); };
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &context, &mut dm).unwrap();
    }
    assert!(state.pending_activation.is_none());
    assert!(game.is_tapped(source));
    assert_eq!(game.stack.len(), 1);
    assert_eq!(game.stack[0].controller, A);
}
#[test]
fn resolving_population_protection_uses_ability_controller_and_freezes_colors() {
    for definition in grant_definitions(false) { for state in 0..4 {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let protected = creature(&mut game, B, ColorSet::COLORLESS);
        let blue_population = creature(&mut game, A, ColorSet::BLUE);
        let red_population = creature(&mut game, B, ColorSet::RED);
        let blue = witness(&mut game, B, vec![CardType::Instant], vec![], ColorSet::BLUE, false, Zone::Stack);
        let red = witness(&mut game, B, vec![CardType::Instant], vec![], ColorSet::RED, false, Zone::Stack);
        activate_grant(&mut game, source, protected);
        // The activated ability remains Alice's, regardless of its source.
        match state {
            1 => { game.move_object_by_effect(source, Zone::Graveyard).unwrap(); }
            2 => { game.set_current_controller(source, B).unwrap(); }
            3 => game.phase_out(source),
            _ => {}
        }
        ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker).unwrap();
        assert!(matching(&game, protected, blue), "source state {state}");
        assert!(!matching(&game, protected, red), "the recipient and changed source cannot substitute their controller");
        // Information requested by the unquoted instruction is fixed at
        // resolution, not re-read after color/population or control changes.
        change(&mut game, blue_population, Modification::SetColors(ColorSet::GREEN));
        game.set_current_controller(red_population, A).unwrap();
        game.set_current_controller(protected, A).unwrap();
        if state == 0 { game.phase_out(source); }
        if state == 3 { game.phase_in(source); }
        game.refresh_continuous_state().unwrap();
        assert!(matching(&game, protected, blue));
        assert!(!matching(&game, protected, red));
    } }
}
#[test]
fn quoted_population_rule_belongs_to_its_recipient_after_resolution() {
    for definition in grant_definitions(true) { for state in 0..4 {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let protected = creature(&mut game, B, ColorSet::COLORLESS);
        let blue_population = creature(&mut game, A, ColorSet::BLUE);
        let red_population = creature(&mut game, B, ColorSet::RED);
        let blue = witness(&mut game, B, vec![CardType::Instant], vec![], ColorSet::BLUE, false, Zone::Stack);
        let red = witness(&mut game, B, vec![CardType::Instant], vec![], ColorSet::RED, false, Zone::Stack);
        activate_grant(&mut game, source, protected);
        match state {
            1 => { game.move_object_by_effect(source, Zone::Graveyard).unwrap(); }
            2 => { game.set_current_controller(source, B).unwrap(); }
            3 => game.phase_out(source),
            _ => {}
        }
        ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker).unwrap();
        assert!(!matching(&game, protected, blue));
        assert!(matching(&game, protected, red));
        game.set_current_controller(protected, A).unwrap();
        game.refresh_continuous_state().unwrap();
        assert!(matching(&game, protected, blue));
        assert!(!matching(&game, protected, red));
        game.phase_out(blue_population);
        game.refresh_continuous_state().unwrap();
        assert!(!matching(&game, protected, blue));
        game.set_current_controller(red_population, A).unwrap();
        game.refresh_continuous_state().unwrap();
        assert!(matching(&game, protected, red));
    } }
}
