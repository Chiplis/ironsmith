//! Frozen full bodies, independent direct/artifact/text paths, and native runtime
//! witnesses. AUTHORED, UNRUN: no compiler, engine, or recovery probe was executed.
use ironsmith::ability::{Ability, AbilityKind};
use ironsmith::card::PowerToughness;
use ironsmith::cards::{CardDefinition, builders::CardDefinitionBuilder};
use ironsmith::color::{Color, ColorSet};
use ironsmith::continuous::{ContinuousEffect, EffectSourceType, EffectTarget, Modification};
use ironsmith::decision::{DecisionMaker, LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
use ironsmith::decisions::context::{ColorsContext, SelectOptionsContext, SelectObjectsContext, TargetsContext};
use ironsmith::effect::Effect;
use ironsmith::effects::{EffectContext, execute_effect};
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, put_triggers_on_stack_with_dm, resolve_stack_entry_with};
use ironsmith::mana::ManaSymbol;
use ironsmith::object::CounterType;
use ironsmith::snapshot::CopiableValues;
use ironsmith::static_abilities::{StaticAbility, StaticAbilityId};
use ironsmith::target::{ChooseSpec, ObjectFilter};
use ironsmith::triggers::TriggerQueue;
use ironsmith::{CardId, CardType, GameProgress, GameState, ObjectId, Phase, PlayerId, Subtype, Supertype, Target, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler::parse_loss;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
const A: PlayerId = PlayerId::from_index(0);
const B: PlayerId = PlayerId::from_index(1);
const PARTY: [Subtype; 4] = [Subtype::Cleric, Subtype::Rogue, Subtype::Warrior, Subtype::Wizard];
fn rows() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!("../../../fixtures/copular_characteristic_statics.json.fixture")).unwrap()
}
fn metadata(row: &serde_json::Value) -> String {
    let mut text = format!("Mana cost: {}\nType: {}\n", row["mana_cost"].as_str().unwrap(), row["type_line"].as_str().unwrap());
    if let (Some(p), Some(t)) = (row["power"].as_str(), row["toughness"].as_str()) { text.push_str(&format!("Power/Toughness: {p}/{t}\n")); }
    text
}
fn from_text(name: &str, text: &str) -> [CardDefinition; 2] {
    let (direct, loss) = parse_loss::capture(|| compile_to_runtime_definition(name, text, false));
    let direct = direct.unwrap_or_else(|error| panic!("direct {name}: {error}"));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let (compiled, loss) = parse_loss::capture(|| compile_to_artifact(name, text, false));
    let (artifact, _) = compiled.unwrap_or_else(|error| panic!("artifact {name}: {error}"));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    restored.validate().unwrap();
    assert_eq!(artifact, restored);
    [direct, ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap()]
}
fn definitions(name: &str) -> [CardDefinition; 2] {
    let row = rows().into_iter().find(|row| row["name"] == name).unwrap();
    assert_eq!(row["proposed_complete"], true, "partial bodies are not promoted by clause coverage");
    from_text(name, &(metadata(&row) + row["oracle_text"].as_str().unwrap()))
}
fn game() -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.phase = Phase::FirstMain;
    game.turn.active_player = A;
    game.turn.priority_player = Some(A);
    game
}
fn native(name: &str, types: Vec<CardType>, subtypes: Vec<Subtype>) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name).card_types(types).subtypes(subtypes)
        .power_toughness(PowerToughness::fixed(4, 5)).build()
}
fn witness(game: &mut GameState, owner: PlayerId, zone: Zone, types: Vec<CardType>, subtypes: Vec<Subtype>) -> ObjectId {
    game.create_object_from_definition(&native("Independent witness", types, subtypes), owner, zone)
}
fn refresh(game: &mut GameState) { game.refresh_continuous_state().unwrap(); }
fn subtype(game: &GameState, id: ObjectId, kind: Subtype) -> bool { game.current_subtypes(id).unwrap().contains(&kind) }
fn creature(game: &GameState, id: ObjectId) -> bool { game.current_card_types(id).unwrap().contains(&CardType::Creature) }
fn pt(game: &GameState, id: ObjectId) -> (Option<i32>, Option<i32>) {
    let chars = game.try_current_characteristics(id).unwrap().unwrap(); (chars.power, chars.toughness)
}
fn effect(game: &mut GameState, source: ObjectId, target: EffectTarget, modification: Modification) {
    game.effect_store.continuous_effects.add_effect(ContinuousEffect::new(source, A, target, modification)); refresh(game);
}
fn attach(game: &mut GameState, aura: ObjectId, host: ObjectId) {
    execute_effect(game, &Effect::attach_objects(ChooseSpec::SpecificObject(aura), ChooseSpec::SpecificObject(host)),
        &mut EffectContext::new(aura, A, &mut SelectFirstDecisionMaker)).unwrap(); refresh(game);
}
#[derive(Default)]
struct Choices { target: Option<Target>, selected: Vec<ObjectId>, subtype: Option<&'static str>, color: Option<Color>, prompts: usize }
impl DecisionMaker for Choices {
    fn decide_targets(&mut self, game: &GameState, context: &TargetsContext) -> Vec<Target> {
        if let Some(target) = self.target {
            assert!(context.requirements.iter().any(|requirement| requirement.legal_targets.contains(&target)));
            vec![target]
        } else { SelectFirstDecisionMaker.decide_targets(game, context) }
    }
    fn decide_objects(&mut self, game: &GameState, context: &SelectObjectsContext) -> Vec<ObjectId> {
        if self.selected.is_empty() { SelectFirstDecisionMaker.decide_objects(game, context) } else { self.selected.clone() }
    }
    fn decide_options(&mut self, game: &GameState, context: &SelectOptionsContext) -> Vec<usize> {
        if let Some(name) = self.subtype {
            if let Some(option) = context.options.iter().find(|option| option.description == name) {
                self.prompts += 1; return vec![option.index];
            }
        }
        SelectFirstDecisionMaker.decide_options(game, context)
    }
    fn decide_colors(&mut self, _: &GameState, context: &ColorsContext) -> Vec<Color> {
        self.prompts += 1; assert_eq!(context.count, 1); vec![self.color.unwrap_or(Color::Blue)]
    }
}
fn enter(game: &mut GameState, definition: &CardDefinition, owner: PlayerId, choices: &mut Choices) -> ObjectId {
    let hand = game.create_object_from_definition(definition, owner, Zone::Hand);
    let receipt = game.move_object_with_etb_processing_with_dm(hand, Zone::Battlefield, choices).unwrap();
    assert!(!receipt.pending); assert!(receipt.programs.is_empty()); receipt.original.into_result().unwrap().new_id
}
fn settle(game: &mut GameState, choices: &mut Choices) {
    for _ in 0..24 {
        put_triggers_on_stack_with_dm(game, &mut TriggerQueue::new(), choices).unwrap();
        if game.stack_is_empty() { return; }
        resolve_stack_entry_with(game, choices).unwrap();
    }
    panic!("full-body scenario did not settle");
}
fn activate(game: &mut GameState, source: ObjectId, ordinal: usize, choices: &mut Choices) {
    game.turn.priority_player = Some(A);
    let action = compute_legal_actions(game, A).unwrap().into_iter().filter(|action| matches!(action,
        LegalAction::ActivateAbility { source: id, .. } | LegalAction::ActivateManaAbility { source: id, .. } if *id == source))
        .nth(ordinal).expect("real payable activation");
    let mut state = PriorityLoopState::new(2); let mut queue = TriggerQueue::new();
    let mut progress = apply_priority_response_with_dm(game, &mut queue, &mut state,
        &PriorityResponse::PriorityAction(action), choices).unwrap();
    for _ in 0..32 {
        if !state.has_pending_action() { break; }
        let GameProgress::NeedsDecisionCtx(context) = progress else { panic!("{progress:?}"); };
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &context, choices).unwrap();
    }
    assert!(!state.has_pending_action()); settle(game, choices);
}

#[test]
fn complete_bodies_and_canonical_text_use_independent_compiler_routes() {
    let mut count = 0;
    for row in rows().iter().filter(|row| row["proposed_complete"] == true) {
        let name = row["name"].as_str().unwrap();
        for original in definitions(name) {
            assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&original));
            let rendered = ironsmith_text::compiled_text_lines(&original).join("\n");
            assert!(!rendered.is_empty());
            for reparsed in from_text(name, &(metadata(row) + &rendered)) {
                assert_eq!(reparsed.card.card_types, original.card.card_types);
                assert_eq!(reparsed.card.subtypes, original.card.subtypes);
                assert_eq!(reparsed.abilities.len(), original.abilities.len(), "{name}: {rendered}");
                assert_eq!(reparsed.card.color_identity(), original.card.color_identity());
            }
        }
        count += 1;
    }
    assert_eq!(count, 13);
}

#[test]
fn printed_party_subtypes_function_in_every_zone_and_keep_original_type() {
    for (name, original) in [("Stonework Packbeast", Subtype::Beast), ("Veteran Adventurer", Subtype::Human)] {
        for definition in definitions(name) { for zone in [Zone::Hand, Zone::Library, Zone::Stack,
            Zone::Battlefield, Zone::Graveyard, Zone::Exile, Zone::Command, Zone::Ante, Zone::OutsideGame] {
            let mut game = game(); let id = game.create_object_from_definition(&definition, A, zone); refresh(&mut game);
            assert!(subtype(&game, id, original));
            for role in PARTY { assert!(subtype(&game, id, role), "{name} in {zone:?}"); }
            let cloned = game.clone(); assert_eq!(cloned.current_subtypes(id), game.current_subtypes(id));
        }}
    }
}

#[test]
fn native_printed_copied_and_granted_subtypes_keep_cda_order_and_zones() {
    let mut game = game();
    let changer = witness(&mut game, A, Zone::Battlefield, vec![CardType::Artifact], vec![]);
    effect(&mut game, changer, EffectTarget::AllPermanents, Modification::SetSubtypes(vec![Subtype::Zombie]));
    let ability = StaticAbility::add_subtypes(ObjectFilter::source(), PARTY.to_vec());
    let definition = CardDefinitionBuilder::new(CardId::new(), "Native roles")
        .card_types(vec![CardType::Creature]).subtypes(vec![Subtype::Beast])
        .power_toughness(PowerToughness::fixed(2, 2)).with_ability(Ability::static_ability(ability.clone())).build();
    let printed = game.create_object_from_definition(&definition, A, Zone::Battlefield);
    assert_eq!(game.current_subtypes(printed).unwrap(), [Subtype::Zombie], "CDA before older replacement");
    let copied = witness(&mut game, A, Zone::Battlefield, vec![CardType::Creature], vec![Subtype::Elf]);
    let values = CopiableValues::from_object(game.object(printed).unwrap());
    effect(&mut game, copied, EffectTarget::Source, Modification::CopyOf { target_id: printed,
        copiable_values: Box::new(values), preserve_source_abilities: false, name_override: None,
        name_override_surface: None, add_supertypes: vec![] });
    assert_eq!(game.current_subtypes(copied).unwrap(), [Subtype::Zombie]);
    let host = witness(&mut game, A, Zone::Battlefield, vec![CardType::Creature], vec![Subtype::Elf]);
    game.grant_temporary_static_ability_payload_to_object_until_end_of_turn(host, StaticAbilityId::AddSubtypes, Some(ability.clone()));
    refresh(&mut game);
    for role in PARTY { assert!(subtype(&game, host, role)); }
    let origin = game.object(host).unwrap().temporary_static_ability_grants.origin(0).unwrap().clone();
    let cloned = game.clone(); assert_eq!(cloned.object(host).unwrap().temporary_static_ability_grants.origin(0), Some(&origin));
    for zone in [Zone::Hand, Zone::Stack, Zone::Graveyard, Zone::Library] {
        let outside = witness(&mut game, A, zone, vec![CardType::Creature], vec![Subtype::Elf]);
        game.grant_temporary_static_ability_payload_to_object_until_end_of_turn(outside, StaticAbilityId::AddSubtypes, Some(ability.clone()));
        refresh(&mut game); assert!(!subtype(&game, outside, Subtype::Wizard));
    }
    let effects = ironsmith::static_ability_processor::generate_continuous_effects_from_static_abilities(&game);
    assert!(effects.iter().any(|effect| effect.source == printed && matches!(effect.source_type, EffectSourceType::CharacteristicDefining)));
    assert!(effects.iter().any(|effect| effect.source == host && matches!(effect.source_type, EffectSourceType::StaticAbility)));
    game.effect_store.continuous_effects.remove_effects_from_source(changer); refresh(&mut game);
    for id in [printed, copied] { for role in PARTY { assert!(subtype(&game, id, role)); } }
    let graveyard = game.move_object_by_effect(host, Zone::Graveyard).unwrap(); refresh(&mut game);
    assert!(game.object(graveyard).unwrap().temporary_static_ability_grants.is_empty());
    assert_eq!(game.current_subtypes(graveyard).unwrap(), [Subtype::Elf]);
}

#[test]
fn land_animations_preserve_other_types_counters_and_live_controller_filters() {
    for (name, land_type, color) in [("Ambush Commander", Subtype::Forest, ColorSet::GREEN),
        ("Kormus Bell", Subtype::Swamp, ColorSet::BLACK)] {
        for definition in definitions(name) {
            let mut game = game(); let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let own = witness(&mut game, A, Zone::Battlefield, vec![CardType::Land, CardType::Artifact], vec![land_type]);
            let enemy = witness(&mut game, B, Zone::Battlefield, vec![CardType::Land], vec![land_type]);
            let hand = witness(&mut game, A, Zone::Hand, vec![CardType::Land], vec![land_type]);
            game.add_counters(own, CounterType::PlusOnePlusOne, 1); refresh(&mut game);
            assert!(creature(&game, own)); assert!(game.current_card_types(own).unwrap().contains(&CardType::Artifact));
            assert!(subtype(&game, own, land_type)); assert_eq!(pt(&game, own), (Some(2), Some(2)));
            assert_eq!(game.current_colors(own), Some(color)); assert!(!creature(&game, hand));
            assert_eq!(creature(&game, enemy), name == "Kormus Bell");
            if name == "Ambush Commander" { assert!(subtype(&game, own, Subtype::Elf)); }
            game.set_current_controller(source, B).unwrap(); refresh(&mut game);
            assert!(creature(&game, enemy)); assert_eq!(creature(&game, own), name == "Kormus Bell");
            game.phase_out(source); refresh(&mut game); assert!(!creature(&game, own)); assert!(!creature(&game, enemy));
            game.phase_in(source); refresh(&mut game); assert!(creature(&game, enemy));
            game.move_object_by_effect(source, Zone::Graveyard).unwrap(); refresh(&mut game); assert!(!creature(&game, enemy));
            assert!(subtype(&game, own, land_type)); assert_eq!(game.counter_count(own, CounterType::PlusOnePlusOne), 1);
        }
    }
}

#[test]
fn aura_animation_conditions_the_attached_vehicle_and_preserves_grants() {
    for (name, p, t, keyword) in [("Aerial Modification", 6, 7, StaticAbilityId::Flying),
        ("Siege Modification", 7, 5, StaticAbilityId::FirstStrike)] {
        for definition in definitions(name) {
            let mut game = game();
            let host = witness(&mut game, B, Zone::Battlefield, vec![CardType::Artifact, CardType::Land], vec![Subtype::Vehicle, Subtype::Forest]);
            let aura = game.create_object_from_definition(&definition, A, Zone::Battlefield); attach(&mut game, aura, host);
            assert!(!creature(&game, aura)); assert!(creature(&game, host)); assert_eq!(pt(&game, host), (Some(p), Some(t)));
            assert!(game.object_has_static_ability_id(host, keyword));
            assert!(game.current_card_types(host).unwrap().contains(&CardType::Land)); assert!(subtype(&game, host, Subtype::Forest));
            let other = witness(&mut game, A, Zone::Battlefield, vec![CardType::Creature], vec![Subtype::Bear]);
            attach(&mut game, aura, other); assert!(!creature(&game, host)); assert!(game.object_has_static_ability_id(other, keyword));
            attach(&mut game, aura, host); game.phase_out(aura); refresh(&mut game); assert!(!creature(&game, host));
            game.phase_in(aura); refresh(&mut game); assert!(creature(&game, host));
            let departed = game.move_object_by_effect(host, Zone::Graveyard).unwrap();
            let returned = game.move_object_by_effect(departed, Zone::Battlefield).unwrap(); refresh(&mut game);
            assert_ne!(returned, host); assert!(!creature(&game, returned), "old attachment cannot follow a new incarnation");
        }
    }
}

#[test]
fn ashes_entry_choice_and_controller_changes_affect_only_the_current_graveyard() {
    for definition in definitions("Ashes of the Fallen") {
        let mut game = game();
        let own = witness(&mut game, A, Zone::Graveyard, vec![CardType::Creature], vec![Subtype::Elf]);
        let enemy = witness(&mut game, B, Zone::Graveyard, vec![CardType::Creature], vec![Subtype::Beast]);
        let hand = witness(&mut game, A, Zone::Hand, vec![CardType::Creature], vec![Subtype::Elf]);
        let permanent = witness(&mut game, A, Zone::Battlefield, vec![CardType::Creature], vec![Subtype::Elf]);
        let noncreature = witness(&mut game, A, Zone::Graveyard, vec![CardType::Artifact], vec![]);
        let mut choices = Choices { subtype: Some("Wizard"), ..Default::default() };
        let source = enter(&mut game, &definition, A, &mut choices); refresh(&mut game);
        assert_eq!(choices.prompts, 1); assert_eq!(game.chosen_creature_type(source), Some(Subtype::Wizard));
        assert!(subtype(&game, own, Subtype::Wizard)); assert!(subtype(&game, own, Subtype::Elf));
        for id in [enemy, hand, permanent, noncreature] { assert!(!subtype(&game, id, Subtype::Wizard)); }
        let mut restored = game.clone(); refresh(&mut restored);
        assert!(subtype(&restored, own, Subtype::Wizard)); assert_eq!(restored.chosen_creature_type(source), Some(Subtype::Wizard));
        game.set_current_controller(source, B).unwrap(); refresh(&mut game);
        assert!(!subtype(&game, own, Subtype::Wizard)); assert!(subtype(&game, enemy, Subtype::Wizard));
        game.phase_out(source); refresh(&mut game); assert!(!subtype(&game, enemy, Subtype::Wizard));
        game.phase_in(source); refresh(&mut game); assert!(subtype(&game, enemy, Subtype::Wizard));
        let graveyard = game.move_object_by_effect(source, Zone::Graveyard).unwrap(); refresh(&mut game);
        assert!(!subtype(&game, enemy, Subtype::Wizard)); assert_eq!(game.chosen_creature_type(graveyard), None);
        choices.subtype = Some("Rogue");
        let receipt = game.move_object_with_etb_processing_with_dm(graveyard, Zone::Battlefield, &mut choices).unwrap();
        assert!(!receipt.pending); let new_source = receipt.original.into_result().unwrap().new_id; refresh(&mut game);
        assert_ne!(new_source, source); assert_eq!(choices.prompts, 2);
        assert_eq!(game.chosen_creature_type(new_source), Some(Subtype::Rogue));
        assert!(subtype(&game, own, Subtype::Rogue)); assert!(!subtype(&game, own, Subtype::Wizard));
    }
}

#[test]
fn pending_entry_choice_is_uncommitted_and_clone_recovery_selects_once() {
    #[derive(Default)] struct Pending { pending: bool }
    impl DecisionMaker for Pending {
        fn decide_options(&mut self, _: &GameState, _: &SelectOptionsContext) -> Vec<usize> { self.pending = true; vec![] }
        fn awaiting_choice(&self) -> bool { self.pending }
    }
    for definition in definitions("Ashes of the Fallen") {
        let mut game = game(); let old = game.create_object_from_definition(&definition, A, Zone::Hand);
        let grave = witness(&mut game, A, Zone::Graveyard, vec![CardType::Creature], vec![Subtype::Elf]);
        let receipt = game.move_object_with_etb_processing_with_dm(old, Zone::Battlefield, &mut Pending::default()).unwrap();
        assert!(receipt.pending); assert_eq!(game.object(old).unwrap().zone, Zone::Hand);
        assert_eq!(game.chosen_creature_type(old), None); assert!(!subtype(&game, grave, Subtype::Wizard));
        let mut restored = game.clone();
        let mut choices = Choices { subtype: Some("Wizard"), ..Default::default() };
        let receipt = restored.move_object_with_etb_processing_with_dm(old, Zone::Battlefield, &mut choices).unwrap();
        assert!(!receipt.pending); assert!(receipt.programs.is_empty()); let source = receipt.original.into_result().unwrap().new_id;
        refresh(&mut restored); assert_eq!(choices.prompts, 1); assert_eq!(restored.chosen_creature_type(source), Some(Subtype::Wizard));
        assert!(subtype(&restored, grave, Subtype::Wizard)); assert_eq!(game.object(old).unwrap().zone, Zone::Hand);
    }
}

#[test]
fn shifting_sky_replaces_colors_from_its_own_live_choice_and_excludes_lands() {
    for definition in definitions("Shifting Sky") {
        let mut game = game(); let mut choices = Choices { color: Some(Color::Red), ..Default::default() };
        let source = enter(&mut game, &definition, A, &mut choices);
        let artifact = witness(&mut game, B, Zone::Battlefield, vec![CardType::Artifact], vec![]);
        let land = witness(&mut game, B, Zone::Battlefield, vec![CardType::Artifact, CardType::Land], vec![Subtype::Forest]);
        let hand = witness(&mut game, B, Zone::Hand, vec![CardType::Artifact], vec![]); refresh(&mut game);
        assert_eq!(choices.prompts, 1);
        for id in [source, artifact] { assert_eq!(game.current_colors(id), Some(ColorSet::RED)); }
        for id in [land, hand] { assert_eq!(game.current_colors(id), Some(ColorSet::COLORLESS)); }
        game.set_current_controller(source, B).unwrap(); refresh(&mut game); assert_eq!(game.current_colors(artifact), Some(ColorSet::RED));
        effect(&mut game, artifact, EffectTarget::Source, Modification::SetColors(ColorSet::BLUE));
        assert_eq!(game.current_colors(artifact), Some(ColorSet::BLUE), "later ordinary colors still apply");
        game.phase_out(source); refresh(&mut game); assert_eq!(game.current_colors(source), None);
        assert_eq!(game.current_colors(artifact), Some(ColorSet::BLUE));
        game.phase_in(source); refresh(&mut game); assert_eq!(game.current_colors(source), Some(ColorSet::RED));
        let departed = game.move_object_by_effect(source, Zone::Graveyard).unwrap();
        assert_eq!(game.chosen_color(departed), None);
        assert_eq!(game.current_colors(departed), Some(ColorSet::BLUE));
        let clone = game.clone(); assert_eq!(clone.current_colors(artifact), Some(ColorSet::BLUE));
    }
}

#[test]
fn rusted_relic_threshold_is_live_and_never_an_all_zone_subtype_definition() {
    for definition in definitions("Rusted Relic") {
        let mut game = game(); let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let first = witness(&mut game, A, Zone::Battlefield, vec![CardType::Artifact], vec![]); refresh(&mut game);
        assert!(!creature(&game, source));
        let second = witness(&mut game, A, Zone::Battlefield, vec![CardType::Artifact], vec![]); refresh(&mut game);
        assert!(creature(&game, source)); assert!(subtype(&game, source, Subtype::Golem)); assert_eq!(pt(&game, source), (Some(5), Some(5)));
        effect(&mut game, first, EffectTarget::Specific(source), Modification::AddCardTypes(vec![CardType::Land]));
        assert!(game.current_card_types(source).unwrap().contains(&CardType::Land));
        game.add_counters(source, CounterType::PlusOnePlusOne, 1); refresh(&mut game); assert_eq!(pt(&game, source), (Some(6), Some(6)));
        game.move_object_by_effect(second, Zone::Graveyard).unwrap(); refresh(&mut game);
        assert!(!creature(&game, source)); assert!(!subtype(&game, source, Subtype::Golem));
        for zone in [Zone::Hand, Zone::Graveyard, Zone::Stack, Zone::Library] {
            let outside = game.create_object_from_definition(&definition, A, zone); refresh(&mut game);
            assert!(!creature(&game, outside)); assert!(!subtype(&game, outside, Subtype::Golem));
        }
    }
}

#[test]
fn war_balloon_pays_for_real_counter_activations_and_preserves_printed_size() {
    for definition in definitions("War Balloon") {
        let mut game = game(); let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Colorless, 3);
        assert!(!creature(&game, source));
        for count in 1..=3 {
            activate(&mut game, source, 0, &mut Choices::default());
            assert_eq!(game.counter_count(source, CounterType::Named("fire".into())), count);
            assert_eq!(creature(&game, source), count == 3);
        }
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
        assert_eq!(pt(&game, source), (Some(4), Some(3)));
        assert!(game.object_has_static_ability_id(source, StaticAbilityId::Flying));
        game.remove_counters(source, CounterType::Named("fire".into()), 1, None, None); refresh(&mut game); assert!(!creature(&game, source));
        let pilot = witness(&mut game, A, Zone::Battlefield, vec![CardType::Creature], vec![Subtype::Pilot]);
        activate(&mut game, source, 0, &mut Choices { selected: vec![pilot], ..Default::default() });
        assert!(game.is_tapped(pilot)); assert!(creature(&game, source));
        assert_eq!(game.counter_count(source, CounterType::Named("fire".into())), 2, "crew does not fabricate the third fire counter");
    }
}

#[test]
fn secondary_native_abilities_remain_executable_after_subtype_and_land_changes() {
    for definition in definitions("Stonework Packbeast") {
        let mut game = game(); let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Colorless, 2);
        activate(&mut game, source, 0, &mut Choices { color: Some(Color::Red), ..Default::default() });
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 1);
        assert!(subtype(&game, source, Subtype::Wizard));
    }
    for definition in definitions("Ambush Commander") {
        let mut game = game(); let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let forest = witness(&mut game, A, Zone::Battlefield, vec![CardType::Land], vec![Subtype::Forest]);
        let target = witness(&mut game, A, Zone::Battlefield, vec![CardType::Creature], vec![Subtype::Bear]);
        game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Green, 2);
        activate(&mut game, source, 0, &mut Choices { target: Some(Target::Object(target)), selected: vec![forest], ..Default::default() });
        assert!(game.object(forest).is_none()); assert_eq!(pt(&game, target), (Some(7), Some(8)));
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
    }
}

#[test]
fn nyleas_presence_keeps_draw_attachment_all_basic_types_and_existing_abilities() {
    for definition in definitions("Nylea's Presence") {
        let mut game = game();
        witness(&mut game, A, Zone::Library, vec![CardType::Artifact], vec![]);
        let land = witness(&mut game, B, Zone::Battlefield, vec![CardType::Land, CardType::Artifact], vec![Subtype::Forest]);
        let aura = enter(&mut game, &definition, A, &mut Choices::default()); attach(&mut game, aura, land);
        settle(&mut game, &mut Choices::default()); assert_eq!(game.player(A).unwrap().hand.len(), 1);
        for kind in [Subtype::Plains, Subtype::Island, Subtype::Swamp, Subtype::Mountain, Subtype::Forest] {
            assert!(subtype(&game, land, kind));
        }
        assert_eq!(game.current_card_types(land).unwrap(), [CardType::Land, CardType::Artifact]);
        assert!(game.calculated_characteristics(land).unwrap().abilities.iter().filter(|ability|
            matches!(&ability.kind, AbilityKind::Activated(ability) if ability.is_mana_ability())).count() >= 5);
        game.move_object_by_effect(aura, Zone::Graveyard).unwrap(); refresh(&mut game);
        assert_eq!(game.current_subtypes(land).unwrap(), [Subtype::Forest]);
    }
}

#[test]
fn leyline_retains_the_typed_opening_permission_and_live_nonland_legendary_rule() {
    for definition in definitions("Leyline of Singularity") {
        assert!(definition.abilities.iter().any(|ability| matches!(&ability.kind,
            AbilityKind::Static(ability) if matches!(ability.pregame_action_kind(),
                Some(ironsmith::static_abilities::PregameActionKind::BeginOnBattlefield(_))))));
        let mut game = game(); let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let artifact = witness(&mut game, B, Zone::Battlefield, vec![CardType::Artifact], vec![]);
        let land = witness(&mut game, A, Zone::Battlefield, vec![CardType::Artifact, CardType::Land], vec![Subtype::Forest]);
        let hand = witness(&mut game, A, Zone::Hand, vec![CardType::Artifact], vec![]); refresh(&mut game);
        for id in [source, artifact] { assert!(game.current_has_supertype(id, Supertype::Legendary)); }
        for id in [land, hand] { assert!(!game.current_has_supertype(id, Supertype::Legendary)); }
        game.phase_out(source); refresh(&mut game); assert!(!game.current_has_supertype(artifact, Supertype::Legendary));
        game.phase_in(source); refresh(&mut game); assert!(game.current_has_supertype(artifact, Supertype::Legendary));
        game.move_object_by_effect(source, Zone::Graveyard).unwrap(); refresh(&mut game);
        assert!(!game.current_has_supertype(artifact, Supertype::Legendary));
    }
}

#[test]
fn rimefeather_uses_real_snow_mana_and_live_ice_counter_supertypes_for_its_size() {
    for definition in definitions("Rimefeather Owl") {
        let mut game = game(); let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let snow_land = CardDefinitionBuilder::new(CardId::new(), "Snow mana witness")
            .card_types(vec![CardType::Land]).supertypes(vec![Supertype::Snow]).subtypes(vec![Subtype::Island]).build();
        let land = game.create_object_from_definition(&snow_land, A, Zone::Battlefield);
        let target = witness(&mut game, B, Zone::Battlefield, vec![CardType::Artifact], vec![]); refresh(&mut game);
        assert_eq!(pt(&game, source), (Some(2), Some(2))); assert!(game.object_has_static_ability_id(source, StaticAbilityId::Flying));
        activate(&mut game, land, 0, &mut Choices::default());
        assert!(game.is_tapped(land));
        game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Colorless, 1);
        activate(&mut game, source, 0, &mut Choices { target: Some(Target::Object(target)), ..Default::default() });
        assert_eq!(game.counter_count(target, CounterType::Ice), 1); assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
        assert!(game.current_has_supertype(target, Supertype::Snow)); assert_eq!(pt(&game, source), (Some(3), Some(3)));
        game.remove_counters(target, CounterType::Ice, 1, None, None); refresh(&mut game);
        assert!(!game.current_has_supertype(target, Supertype::Snow)); assert_eq!(pt(&game, source), (Some(2), Some(2)));
        game.add_counters(target, CounterType::Ice, 1); game.phase_out(source); refresh(&mut game);
        assert!(!game.current_has_supertype(target, Supertype::Snow));
        game.phase_in(source); refresh(&mut game); assert!(game.current_has_supertype(target, Supertype::Snow));
    }
}

#[test]
fn veteran_cost_counts_distinct_party_members_and_preserves_vigilance() {
    for definition in definitions("Veteran Adventurer") {
        let mut game = game(); let hand = game.create_object_from_definition(&definition, A, Zone::Hand);
        let packbeast = game.create_object_from_definition(&definitions("Stonework Packbeast")[0], A, Zone::Battlefield);
        let effective = |game: &GameState| {
            let object = game.object(hand).unwrap();
            ironsmith::decision::calculate_effective_mana_cost(game, A, object, object.mana_cost.as_ref().unwrap()).mana_value()
        };
        assert_eq!(effective(&game), 5, "one multi-role creature occupies one party slot");
        for role in [Subtype::Cleric, Subtype::Rogue, Subtype::Warrior] {
            witness(&mut game, A, Zone::Battlefield, vec![CardType::Creature], vec![role]);
        }
        refresh(&mut game); assert_eq!(effective(&game), 2);
        game.set_current_controller(packbeast, B).unwrap(); refresh(&mut game); assert_eq!(effective(&game), 3);
        let permanent = game.move_object_by_effect(hand, Zone::Battlefield).unwrap(); refresh(&mut game);
        assert!(game.object_has_static_ability_id(permanent, StaticAbilityId::Vigilance));
        for role in PARTY { assert!(subtype(&game, permanent, role)); }
    }
}

#[test]
fn missing_choice_does_not_invent_color_or_subtype_and_conditions_remain_non_cda() {
    for (name, chosen) in [("Ashes of the Fallen", false), ("Shifting Sky", true)] {
        for definition in definitions(name) {
            let mut game = game();
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let target = witness(&mut game, A, if chosen { Zone::Battlefield } else { Zone::Graveyard }, vec![CardType::Creature], vec![Subtype::Elf]);
            refresh(&mut game); assert_eq!(game.chosen_color(source), None); assert_eq!(game.chosen_creature_type(source), None);
            assert_eq!(game.current_colors(target), Some(ColorSet::COLORLESS)); assert_eq!(game.current_subtypes(target).unwrap(), [Subtype::Elf]);
        }
    }
    let conditional = StaticAbility::add_subtypes(ObjectFilter::source(), vec![Subtype::Wizard])
        .with_condition(ironsmith::ConditionExpr::YourTurn).unwrap();
    let native = CardDefinitionBuilder::new(CardId::new(), "Conditional native role")
        .card_types(vec![CardType::Creature]).subtypes(vec![Subtype::Elf])
        .power_toughness(PowerToughness::fixed(2, 2)).with_ability(Ability::static_ability(conditional)).build();
    let mut game = game();
    let hand = game.create_object_from_definition(&native, A, Zone::Hand);
    let permanent = game.create_object_from_definition(&native, A, Zone::Battlefield); refresh(&mut game);
    assert!(!subtype(&game, hand, Subtype::Wizard)); assert!(subtype(&game, permanent, Subtype::Wizard));
    let effects = ironsmith::static_ability_processor::generate_continuous_effects_from_static_abilities(&game);
    assert!(effects.iter().filter(|effect| effect.source == permanent).all(|effect| !matches!(effect.source_type, EffectSourceType::CharacteristicDefining)));
}

#[test]
fn bounded_discovery_fails_explicitly_without_publishing_partial_characteristics() {
    use ironsmith::static_ability_processor::{StaticEffectDiscoveryError, StaticEffectDiscoveryLimits,
        try_generate_continuous_effects_from_static_abilities};
    for definition in definitions("Ambush Commander") {
        let mut game = game();
        game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let forest = witness(&mut game, A, Zone::Battlefield, vec![CardType::Land], vec![Subtype::Forest]);
        let before_revision = game.effect_store.continuous_effects.revision();
        let result = try_generate_continuous_effects_from_static_abilities(&game,
            StaticEffectDiscoveryLimits { max_rounds: 128, max_generated_effects: 1 });
        assert!(matches!(result, Err(StaticEffectDiscoveryError::EffectLimit { maximum: 1, .. })));
        assert_eq!(game.effect_store.continuous_effects.revision(), before_revision);
        assert_eq!(game.object(forest).unwrap().card_types, [CardType::Land]);
        let complete = try_generate_continuous_effects_from_static_abilities(&game, StaticEffectDiscoveryLimits::default()).unwrap();
        let chars = game.calculated_characteristics_with_effects(forest, &complete).unwrap();
        assert!(chars.card_types.contains(&CardType::Creature)); assert_eq!(chars.colors, ColorSet::GREEN);
    }
}

#[test]
fn absent_controller_evidence_is_not_hidden_by_new_characteristic_statics() {
    use ironsmith::static_ability_processor::{StaticEffectDiscoveryError, StaticEffectDiscoveryLimits,
        try_generate_continuous_effects_from_static_abilities};
    for definition in definitions("Kormus Bell") {
        let mut game = game(); game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let swamp = witness(&mut game, A, Zone::Battlefield, vec![CardType::Land], vec![Subtype::Swamp]);
        let missing = ObjectId::from_raw(999_999);
        game.effect_store.continuous_effects.add_effect(ContinuousEffect::new(missing, A,
            EffectTarget::AllCreatures, Modification::AddAbility(StaticAbility::flying()))
            .with_originating_static_ability(StaticAbility::flying()));
        let revision = game.effect_store.continuous_effects.revision();
        assert!(matches!(try_generate_continuous_effects_from_static_abilities(&game, StaticEffectDiscoveryLimits::default()),
            Err(StaticEffectDiscoveryError::MissingControllerSource { source }) if source == missing));
        assert_eq!(game.effect_store.continuous_effects.revision(), revision);
        assert_eq!(game.object(swamp).unwrap().card_types, [CardType::Land]);
        game.effect_store.continuous_effects.remove_effects_from_source(missing); refresh(&mut game);
        assert!(creature(&game, swamp)); assert_eq!(pt(&game, swamp), (Some(1), Some(1)));
    }
}

#[test]
fn explicit_creature_choice_on_a_land_union_reads_the_creature_choice_store() {
    for definition in from_text("Independent creature choice", "Type: Artifact\nAs this artifact enters, choose a creature type.\nLands and creatures you control have the chosen creature type in addition to their other types.") {
        let mut game = game();
        let own = witness(&mut game, A, Zone::Battlefield, vec![CardType::Creature], vec![Subtype::Elf]);
        let animated = witness(&mut game, A, Zone::Battlefield, vec![CardType::Land, CardType::Creature], vec![Subtype::Forest, Subtype::Beast]);
        let enemy = witness(&mut game, B, Zone::Battlefield, vec![CardType::Creature], vec![Subtype::Elf]);
        let mut choices = Choices { subtype: Some("Wizard"), ..Default::default() };
        let source = enter(&mut game, &definition, A, &mut choices);
        // Independent stored land state must never replace the explicit family.
        game.set_chosen_basic_land_type(source, Subtype::Island); refresh(&mut game);
        assert_eq!(choices.prompts, 1); assert_eq!(game.chosen_creature_type(source), Some(Subtype::Wizard));
        for id in [own, animated] { assert!(subtype(&game, id, Subtype::Wizard)); assert!(!subtype(&game, id, Subtype::Island)); }
        assert!(subtype(&game, animated, Subtype::Forest)); assert!(subtype(&game, animated, Subtype::Beast));
        assert!(!subtype(&game, enemy, Subtype::Wizard));
    }
}
