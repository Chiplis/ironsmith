//! UNVALIDATED source scenarios: full frozen bodies, direct and restored artifacts.
use ironsmith::ability::AbilityKind;
use ironsmith::alternative_cast::CastingMethod;
use ironsmith::cards::CardDefinition;
use ironsmith::combat_state::{AttackTarget, CombatState};
use ironsmith::decision::{DecisionMaker, LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
use ironsmith::decisions::context::{BooleanContext, NumberContext, SelectObjectsContext, SelectOptionsContext, TargetsContext};
use ironsmith::effect::{Effect, EffectOutcome};
use ironsmith::effects::{EffectContext, execute_effect};
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, drain_pending_trigger_events, put_triggers_on_stack_with_dm,
    resolve_stack_entry_with};
use ironsmith::game_state::{Phase, Step};
use ironsmith::mana::ManaSymbol;
use ironsmith::object::{CounterType, ObjectKind};
use ironsmith::target::{ChooseSpec, PlayerFilter};
use ironsmith::triggers::{TriggerEvent, TriggerQueue, check_triggers};
use ironsmith::{GameProgress, GameState, ObjectId, PlayerId, Subtype, Target, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler::parse_loss;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};

const A: PlayerId = PlayerId::from_index(0);
const B: PlayerId = PlayerId::from_index(1);
const C: PlayerId = PlayerId::from_index(2);

fn fixtures() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!("../../../fixtures/reused_token_prototypes.json.fixture")).unwrap()
}
fn definitions(name: &str) -> [CardDefinition; 2] {
    let row = fixtures().into_iter().find(|row| row["name"] == name).unwrap();
    let mut text = format!("Mana cost: {}\nType: {}\n", row["mana_cost"].as_str().unwrap(), row["type_line"].as_str().unwrap());
    if let (Some(power), Some(toughness)) = (row["power"].as_str(), row["toughness"].as_str()) {
        text.push_str(&format!("Power/Toughness: {power}/{toughness}\n"));
    }
    text.push_str(row["oracle_text"].as_str().unwrap());
    definitions_text(name, &text)
}
fn definitions_text(name: &str, text: &str) -> [CardDefinition; 2] {
    let (compiled, loss) = parse_loss::capture(|| compile_to_artifact(name, text, false));
    let (artifact, _) = compiled.unwrap_or_else(|error| panic!("{name}: {error}"));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let (direct, direct_loss) = parse_loss::capture(|| compile_to_runtime_definition(name, text, false));
    let direct = direct.unwrap_or_else(|error| panic!("independent direct route {name}: {error}"));
    assert!(!direct_loss.is_lossy(), "independent direct route {name}: {}", direct_loss.reasons_text());
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, restored);
    [direct, ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap()]
}
fn probe_definition(name: &str, text: &str) -> CardDefinition {
    compile_to_runtime_definition(name, text, false).unwrap()
}
fn game() -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Charlie".into()], 20);
    game.turn.phase = Phase::FirstMain;
    game.turn.active_player = A;
    game.turn.priority_player = Some(A);
    for symbol in [ManaSymbol::White, ManaSymbol::Blue, ManaSymbol::Black,
        ManaSymbol::Red, ManaSymbol::Green, ManaSymbol::Colorless] {
        game.player_mut(A).unwrap().mana_pool.add(symbol, 30);
    }
    game
}
#[derive(Default)]
struct Choices {
    x: u32,
    objects: Vec<ObjectId>,
    targets: Vec<Target>,
    attack_destination: Option<String>,
    attack_options: Vec<String>,
}
impl DecisionMaker for Choices {
    fn decide_boolean(&mut self, _: &GameState, _: &BooleanContext) -> bool { true }
    fn decide_number(&mut self, game: &GameState, context: &NumberContext) -> u32 {
        if context.is_x_value { assert!(self.x <= context.max); self.x }
        else { SelectFirstDecisionMaker.decide_number(game, context) }
    }
    fn decide_targets(&mut self, game: &GameState, context: &TargetsContext) -> Vec<Target> {
        if self.targets.is_empty() { return SelectFirstDecisionMaker.decide_targets(game, context); }
        for (target, requirement) in self.targets.iter().zip(&context.requirements) {
            assert!(requirement.legal_targets.contains(target));
        }
        self.targets.clone()
    }
    fn decide_objects(&mut self, game: &GameState, context: &SelectObjectsContext) -> Vec<ObjectId> {
        if self.objects.is_empty() { return SelectFirstDecisionMaker.decide_objects(game, context); }
        let picked = self.objects.iter().copied().filter(|id|
            context.candidates.iter().any(|candidate| candidate.id == *id && candidate.legal))
            .take(context.max.unwrap_or(usize::MAX)).collect::<Vec<_>>();
        assert!(picked.len() >= context.min);
        picked
    }
    fn decide_options(&mut self, game: &GameState, context: &SelectOptionsContext) -> Vec<usize> {
        if let Some(destination) = &self.attack_destination
            && context.options.iter().any(|option| option.description == *destination)
        {
            self.attack_options = context.options.iter().filter(|option| option.legal)
                .map(|option| option.description.clone()).collect();
            return vec![context.options.iter().find(|option|
                option.legal && option.description == *destination).unwrap().index];
        }
        SelectFirstDecisionMaker.decide_options(game, context)
    }
}
fn pending(game: &mut GameState, choices: &mut Choices) {
    let mut queue = TriggerQueue::new();
    drain_pending_trigger_events(game, &mut queue);
    put_triggers_on_stack_with_dm(game, &mut queue, choices).unwrap();
}
fn settle(game: &mut GameState, choices: &mut Choices) {
    for _ in 0..24 {
        pending(game, choices);
        if game.stack_is_empty() { return; }
        resolve_stack_entry_with(game, choices).unwrap();
    }
    panic!("unexpected continuing trigger chain");
}
fn action(game: &mut GameState, action: LegalAction, choices: &mut Choices) {
    action_with_resource_change(game, action, choices, None);
}
fn action_with_resource_change(game: &mut GameState, action: LegalAction, choices: &mut Choices,
    change: Option<(ObjectId, u32)>) -> bool {
    assert!(compute_legal_actions(game, A).unwrap().contains(&action), "{action:?}");
    let mut queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(3);
    let mut progress = apply_priority_response_with_dm(game, &mut queue, &mut state,
        &PriorityResponse::PriorityAction(action), choices).unwrap();
    let mut changed = false;
    for _ in 0..80 {
        if state.pending_cast.is_none() && state.pending_activation.is_none()
            && state.pending_method_selection.is_none() { break; }
        if !changed && let Some((material, counters)) = change
            && state.pending_cast.as_ref().is_some_and(|pending| pending.cost_resource == Some(material))
        {
            assert_eq!(state.pending_cast.as_ref().unwrap().cost_resource_reduction, 2);
            assert_eq!(game.object(material).unwrap().zone, Zone::Battlefield);
            game.add_counters(material, CounterType::PlusOnePlusOne, counters).unwrap();
            changed = true;
        }
        let GameProgress::NeedsDecisionCtx(context) = progress else { panic!("{progress:?}") };
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &context, choices).unwrap();
    }
    assert!(state.pending_cast.is_none() && state.pending_activation.is_none());
    put_triggers_on_stack_with_dm(game, &mut queue, choices).unwrap();
    changed
}
fn cast(game: &mut GameState, definition: &CardDefinition, zone: Zone, method: CastingMethod, choices: &mut Choices) -> ObjectId {
    let card = game.create_object_from_definition(definition, A, zone);
    action(game, LegalAction::CastSpell { spell_id: card, from_zone: zone, casting_method: method }, choices);
    game.stack.iter().rev().find(|entry| !entry.is_ability).unwrap().object_id
}
fn activate(game: &mut GameState, source: ObjectId, index: usize, choices: &mut Choices) {
    let selected = compute_legal_actions(game, A).unwrap().into_iter().find(|action|
        matches!(action, LegalAction::ActivateAbility { source: id, ability_index }
            | LegalAction::ActivateManaAbility { source: id, ability_index }
            if *id == source && *ability_index == index)).unwrap();
    action(game, selected, choices);
}
fn activated(definition: &CardDefinition) -> usize {
    definition.abilities.iter().position(|ability| matches!(ability.kind, AbilityKind::Activated(_))).unwrap()
}
fn tokens(game: &GameState, subtype: Subtype) -> Vec<ObjectId> {
    game.battlefield.iter().copied().filter(|id| game.object(*id).is_some_and(|object|
        object.kind == ObjectKind::Token && object.has_subtype(subtype))
        && game.current_controller(*id) == Some(A)).collect()
}
fn assert_creatures(game: &GameState, subtype: Subtype, count: usize, p: i32, t: i32, tapped: bool) {
    let ids = tokens(game, subtype);
    assert_eq!(ids.len(), count);
    for id in ids {
        assert_eq!(game.current_power(id), Some(p));
        assert_eq!(game.current_toughness(id), Some(t));
        assert_eq!(game.is_tapped(id), tapped);
    }
}
fn apply(game: &mut GameState, source: ObjectId, effect: Effect, choices: &mut Choices) -> EffectOutcome {
    let outcome = execute_effect(game, &effect, &mut EffectContext::new(source, A, choices)).unwrap();
    let mut queue = TriggerQueue::new();
    drain_pending_trigger_events(game, &mut queue);
    for event in &outcome.events { for entry in check_triggers(game, event) { queue.add(entry); } }
    put_triggers_on_stack_with_dm(game, &mut queue, choices).unwrap();
    outcome
}
fn end_step(game: &mut GameState, choices: &mut Choices) {
    game.turn.phase = Phase::Ending;
    game.turn.step = Some(Step::End);
    let event = TriggerEvent::new_with_provenance(ironsmith::events::BeginningOfEndStepEvent::new(A), Default::default());
    let mut queue = TriggerQueue::new();
    for entry in check_triggers(game, &event) { queue.add(entry); }
    put_triggers_on_stack_with_dm(game, &mut queue, choices).unwrap();
}

#[test]
fn frozen_bodies_keep_complete_secondary_abilities_and_artifact_graphs() {
    for row in fixtures() { for compiled in definitions(row["name"].as_str().unwrap()) {
        let text = ironsmith_text::canonical_compiled_lines(&compiled).join("\n");
        assert!(!text.to_lowercase().contains("unimplemented"), "{text}");
        match row["name"].as_str().unwrap() {
            "Adipose Offspring" => assert!(compiled.alternative_casts.iter().any(|method| method.name() == "Emerge")),
            "From Under the Floorboards" => assert!(compiled.alternative_casts.iter().any(|method| method.name() == "Madness")),
            "The Final Days" => assert!(compiled.alternative_casts.iter().any(|method| method.name() == "Flashback")),
            "Andúril, Flame of the West" => assert!(text.contains("Equip") && text.contains("+3/+1"), "{text}"),
            "Safana, Calimport Cutthroat" => assert!(text.contains("Menace") && text.contains("Background"), "{text}"),
            _ => {}
        }
    }}
}

#[test]
fn brood_birthing_uses_authored_spawn_on_both_branches_and_retains_its_mana_ability() {
    for definition in definitions("Brood Birthing") { for (subtype, owner, expected) in [
        ("Goblin", A, 1), ("Eldrazi Spawn", A, 3), ("Eldrazi Spawn", B, 1), ("Eldrazi Scion", A, 1),
    ] {
        let mut game = game(); let mut choices = Choices::default();
        let host = probe_definition("Existing creature", &format!("Type: Creature — {subtype}\nPower/Toughness: 1/1"));
        game.create_object_from_definition(&host, owner, Zone::Battlefield);
        cast(&mut game, &definition, Zone::Hand, CastingMethod::Normal, &mut choices);
        settle(&mut game, &mut choices);
        assert_creatures(&game, Subtype::Spawn, expected, 0, 1, false);
        let token = tokens(&game, Subtype::Spawn)[0];
        assert_eq!(game.object(token).unwrap().name.as_ref(), "Eldrazi Spawn Token");
        assert_eq!(game.object(token).unwrap().abilities.iter().filter(|ability|
            matches!(ability.kind, AbilityKind::Activated(_))).count(), 1,
            "the authored quoted Spawn mana ability must be installed exactly once");
        let ability = game.object(token).unwrap().abilities.iter().position(|ability|
            matches!(ability.kind, AbilityKind::Activated(_))).unwrap();
        let mana = game.player(A).unwrap().mana_pool.total();
        activate(&mut game, token, ability, &mut choices);
        settle(&mut game, &mut choices);
        assert_eq!(tokens(&game, Subtype::Spawn).len(), expected - 1);
        assert_eq!(game.player(A).unwrap().mana_pool.total(), mana + 1);
    }}
}

#[test]
fn swarming_goblins_native_dice_rows_share_the_first_rows_unexecuted_definition() {
    for definition in definitions("Swarming Goblins") { for (roll, expected) in [(1,1),(9,1),(10,2),(19,2),(20,3)] {
        let mut game = game(); let mut choices = Choices::default();
        game.force_next_die_roll(roll);
        cast(&mut game, &definition, Zone::Hand, CastingMethod::Normal, &mut choices);
        settle(&mut game, &mut choices);
        assert_creatures(&game, Subtype::Goblin, expected, 1, 1, false);
        assert!(tokens(&game, Subtype::Goblin).iter().all(|id|
            game.current_colors(*id) == Some(ironsmith::color::ColorSet::RED)));
        assert_eq!(game.take_forced_die_roll(), None);
    }}
}

#[test]
fn throne_requires_one_of_each_current_artifact_name_under_your_control() {
    for definition in definitions("Throne of Empires") { for case in 0..9 {
        let mut game = game(); let mut choices = Choices::default();
        let throne = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let crown_def = probe_definition("Crown of Empires", "Type: Artifact");
        let scepter_def = probe_definition("Scepter of Empires", if case == 5 { "Type: Enchantment" } else { "Type: Artifact" });
        if case != 0 { game.create_object_from_definition(&crown_def, A, Zone::Battlefield); }
        if case == 2 { game.create_object_from_definition(&crown_def, A, Zone::Battlefield); }
        if (3..7).contains(&case) {
            let scepter = game.create_object_from_definition(&scepter_def, if case == 4 || case == 6 { B } else { A }, Zone::Battlefield);
            if case == 6 { apply(&mut game, throne, Effect::create_token_copy(ChooseSpec::SpecificObject(scepter)), &mut choices); }
        }
        if case >= 7 {
            let subject = game.create_object_from_definition(
                &probe_definition(if case == 7 { "Unrelated artifact" } else { "Scepter of Empires" }, "Type: Artifact"), A, Zone::Battlefield);
            let model = game.create_object_from_definition(if case == 7 { &scepter_def } else { &crown_def }, B, Zone::Battlefield);
            choices.targets = vec![Target::Object(subject), Target::Object(model)];
            cast(&mut game, &probe_definition("Change current name", "Mana cost: {0}\nType: Sorcery\nTarget artifact becomes a copy of another target artifact."), Zone::Hand, CastingMethod::Normal, &mut choices);
            settle(&mut game, &mut choices);
            choices.targets.clear();
            assert_eq!(game.current_name(subject).as_deref(), Some(if case == 7 { "Scepter of Empires" } else { "Crown of Empires" }));
        }
        activate(&mut game, throne, activated(&definition), &mut choices);
        assert!(game.is_tapped(throne));
        settle(&mut game, &mut choices);
        assert_creatures(&game, Subtype::Soldier, if case == 3 || case == 6 || case == 7 { 5 } else { 1 }, 1, 1, false);
    }}
}

#[test]
fn safana_keeps_initiative_intervening_gate_and_dungeon_replacement() {
    for definition in definitions("Safana, Calimport Cutthroat") { for (initiative, dungeon, lose_before_resolution, expected) in [
        (false,false,false,0), (false,true,false,0), (true,false,false,1), (true,true,false,3), (true,true,true,0),
    ] {
        let mut game = game(); let mut choices = Choices::default();
        game.create_object_from_definition(&definition, A, Zone::Battlefield);
        if initiative { game.set_initiative(Some(A)); }
        if dungeon { game.record_completed_dungeon(A, "Lost Mine of Phandelver"); }
        end_step(&mut game, &mut choices);
        if lose_before_resolution { game.set_initiative(Some(B)); }
        settle(&mut game, &mut choices);
        assert_eq!(tokens(&game, Subtype::Treasure).len(), expected);
        for token in tokens(&game, Subtype::Treasure) {
            assert_eq!(game.object(token).unwrap().abilities.iter().filter(|ability|
                matches!(ability.kind, AbilityKind::Activated(_))).count(), 1);
        }
    }}
}

#[test]
fn final_days_uses_cast_origin_and_counts_graveyard_at_resolution_then_flashback_exiles() {
    for definition in definitions("The Final Days") { for (flashback, graves, expected) in [(false,0,2),(false,4,2),(true,0,0),(true,4,4)] {
        let mut game = game(); let mut choices = Choices::default();
        let grave = probe_definition("Graveyard creature", "Type: Creature — Goblin\nPower/Toughness: 1/1");
        for _ in 0..graves { game.create_object_from_definition(&grave, A, Zone::Graveyard); }
        game.create_object_from_definition(&grave, B, Zone::Graveyard);
        game.create_object_from_definition(&probe_definition("Graveyard land", "Type: Land"), A, Zone::Graveyard);
        let method = if flashback { CastingMethod::Alternative(definition.alternative_casts.iter().position(|m| m.name() == "Flashback").unwrap()) } else { CastingMethod::Normal };
        let spell = cast(&mut game, &definition, if flashback { Zone::Graveyard } else { Zone::Hand }, method, &mut choices);
        let stable = game.object(spell).unwrap().stable_id;
        settle(&mut game, &mut choices);
        assert_creatures(&game, Subtype::Horror, expected, 2, 2, true);
        let card = game.find_object_by_stable_id(stable).unwrap();
        assert_eq!(game.object(card).unwrap().zone, if flashback { Zone::Exile } else { Zone::Graveyard });
    }}
}

#[test]
fn floorboards_native_madness_uses_paid_x_for_both_tapped_tokens_and_life() {
    for definition in definitions("From Under the Floorboards") { for (madness, x, expected) in [(false,0,3),(true,0,0),(true,2,2),(true,5,5)] {
        let mut game = game(); let mut choices = Choices { x, ..Default::default() };
        if madness {
            let card = game.create_object_from_definition(&definition, A, Zone::Hand);
            choices.objects = vec![card];
            let host = game.create_object_from_definition(&probe_definition("Discard host", "Type: Artifact"), A, Zone::Battlefield);
            apply(&mut game, host, Effect::discard(1), &mut choices);
            choices.objects.clear();
            assert!(game.stack.iter().any(|entry| entry.is_ability), "native discard must queue madness");
        } else { cast(&mut game, &definition, Zone::Hand, CastingMethod::Normal, &mut choices); }
        settle(&mut game, &mut choices);
        assert_creatures(&game, Subtype::Zombie, expected, 2, 2, true);
        assert_eq!(game.player(A).unwrap().life, 20 + expected as i32);
    }}
}

#[test]
fn anduril_preserves_equipment_bonus_flying_tapped_entry_and_ordinary_attack_destinations() {
    for definition in definitions("Andúril, Flame of the West") {
        for (legendary, destination) in [(false,"Bob"),(true,"Bob"),(true,"Charlie"),(true,"Charlie Walker"),(true,"Protected Battle")] {
            let mut game = game(); let mut choices = Choices::default();
            let sword = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let wielder = game.create_object_from_definition(&probe_definition("Wielder", &format!(
                "Type: {}Creature — Human\nPower/Toughness: 2/3", if legendary { "Legendary " } else { "" })), A, Zone::Battlefield);
            let walker = game.create_object_from_definition(&probe_definition("Charlie Walker", "Type: Planeswalker — Jace\nLoyalty: 5"), C, Zone::Battlefield);
            let battle = game.create_object_from_definition(&probe_definition("Protected Battle", "Type: Battle — Siege\nDefense: 5"), A, Zone::Battlefield);
            assert!(game.set_battle_protector(battle, C));
            choices.targets = vec![Target::Object(wielder)];
            activate(&mut game, sword, activated(&definition), &mut choices);
            settle(&mut game, &mut choices);
            choices.targets.clear();
            assert_eq!(game.current_power(wielder), Some(5));
            assert_eq!(game.current_toughness(wielder), Some(4));
            game.remove_summoning_sickness(wielder);
            game.turn.phase = Phase::Combat;
            game.turn.step = Some(Step::DeclareAttackers);
            game.mark_combat_phase_started();
            game.combat = Some(CombatState::default());
            let mut combat = game.combat.clone().unwrap(); let mut queue = TriggerQueue::new();
            ironsmith::game_loop::apply_attacker_declarations(&mut game, &mut combat, &mut queue,
                &[ironsmith::decision::AttackerDeclaration { creature: wielder, target: AttackTarget::Player(B) }]).unwrap();
            assert_eq!(queue.entries.len(), 1);
            put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut choices).unwrap();
            choices.attack_destination = Some(destination.into());
            settle(&mut game, &mut choices);
            assert_creatures(&game, Subtype::Spirit, 2, 1, 1, true);
            let expected = match destination {
                "Charlie" => AttackTarget::Player(C),
                "Charlie Walker" => AttackTarget::Planeswalker(walker),
                "Protected Battle" => AttackTarget::Battle(battle),
                _ => AttackTarget::Player(B),
            };
            for spirit in tokens(&game, Subtype::Spirit) {
                assert!(game.current_has_static_ability_id(spirit, ironsmith::static_abilities::StaticAbilityId::Flying));
                let attacking = game.combat.as_ref().unwrap().attackers.iter().find(|a| a.creature == spirit);
                assert_eq!(attacking.map(|a| &a.target), legendary.then_some(&expected));
            }
            if legendary {
                for legal in ["Bob", "Charlie", "Charlie Walker", "Protected Battle"] {
                    assert!(choices.attack_options.iter().any(|name| name == legal), "{legal}: {:?}", choices.attack_options);
                }
            }
        }
    }
}

#[test]
fn adipose_uses_paid_sacrifice_lki_and_trigger_copies_keep_it_after_source_departure() {
    for definition in definitions("Adipose Offspring") { for (emerge, remove_source, copy_trigger) in [
        (false,false,false),(true,false,false),(true,true,false),(true,true,true),
    ] {
        let mut game = game(); let mut choices = Choices::default();
        let food = game.create_object_from_definition(&probe_definition("Sacrificed creature",
            "Mana cost: {2}\nType: Creature — Beast\nPower/Toughness: 1/5"), A, Zone::Battlefield);
        game.add_counters(food, CounterType::PlusOnePlusOne, 2).unwrap();
        let before = game.player(A).unwrap().mana_pool.total();
        let method = if emerge { CastingMethod::Alternative(definition.alternative_casts.iter().position(|m| m.name() == "Emerge").unwrap()) } else { CastingMethod::Normal };
        let spell = cast(&mut game, &definition, Zone::Hand, method, &mut choices);
        assert_eq!(before - game.player(A).unwrap().mana_pool.total(), 4);
        let stable = game.object(spell).unwrap().stable_id;
        if emerge {
            let receipt = &game.stack.last().unwrap().tagged_objects[ironsmith::tag::SOURCE_EMERGE_SACRIFICE_TAG];
            assert_eq!(receipt.len(), 1);
            assert_eq!(receipt[0].object_id, food);
            assert_eq!(receipt[0].toughness, Some(7));
            assert!(game.object(food).is_none_or(|o| o.zone != Zone::Battlefield));
        }
        resolve_stack_entry_with(&mut game, &mut choices).unwrap();
        pending(&mut game, &mut choices);
        assert_eq!(game.stack.len(), 1);
        let entrant = game.find_object_by_stable_id(stable).unwrap();
        if copy_trigger {
            let ability = game.stack.last().unwrap().ability_id.unwrap();
            apply(&mut game, entrant, Effect::copy_spell(ChooseSpec::SpecificObject(ability)), &mut choices);
            assert_eq!(game.stack.len(), 2);
        }
        if remove_source {
            game.move_object_by_game_rule(entrant, Zone::Graveyard).unwrap();
            game.create_object_from_definition(&definition, A, Zone::Battlefield);
        }
        settle(&mut game, &mut choices);
        assert_creatures(&game, Subtype::Alien, if emerge { if copy_trigger { 14 } else { 7 } } else { 1 }, 2, 2, false);
    }}
}

#[test]
fn copied_adipose_spell_retains_copied_cast_choices_but_token_copy_does_not_pay_emerge() {
    for definition in definitions("Adipose Offspring") {
        let mut game = game(); let mut choices = Choices::default();
        let host = game.create_object_from_definition(&probe_definition("Copy host", "Type: Artifact"), A, Zone::Battlefield);
        game.create_object_from_definition(&probe_definition("Material", "Mana cost: {3}\nType: Creature — Beast\nPower/Toughness: 1/4"), A, Zone::Battlefield);
        let index = definition.alternative_casts.iter().position(|method| method.name() == "Emerge").unwrap();
        let original = cast(&mut game, &definition, Zone::Hand, CastingMethod::Alternative(index), &mut choices);
        apply(&mut game, host, Effect::copy_spell(ChooseSpec::SpecificObject(original)), &mut choices);
        settle(&mut game, &mut choices);
        // Two ETBs each create four Alien tokens. The permanent spell copy is
        // itself an Alien token, so exclude its retained Emerge keyword.
        let offspring = tokens(&game, Subtype::Alien).into_iter().filter(|id|
            game.object(*id).unwrap().name == "Alien Token").count();
        assert_eq!(offspring, 8);
        let source = game.battlefield.iter().copied().find(|id|
            game.object(*id).is_some_and(|object| object.kind != ObjectKind::Token && object.name == "Adipose Offspring")).unwrap();
        apply(&mut game, host, Effect::create_token_copy(ChooseSpec::SpecificObject(source)), &mut choices);
        settle(&mut game, &mut choices);
        assert_eq!(tokens(&game, Subtype::Alien).into_iter().filter(|id|
            game.object(*id).unwrap().name == "Alien Token").count(), 9);
    }
}

#[test]
fn repeated_token_blueprints_do_not_read_modified_or_departed_created_objects() {
    for followup in ["Put four +1/+1 counters on it.", "Exile it."] {
        let text = format!("Mana cost: {{0}}\nType: Sorcery\nCreate a tapped 1/1 white Spirit creature token with flying. {followup} Create two of those tokens.");
        for definition in definitions_text("Lexical blueprint regression", &text) {
            let mut game = game(); let mut choices = Choices::default();
            cast(&mut game, &definition, Zone::Hand, CastingMethod::Normal, &mut choices);
            settle(&mut game, &mut choices);
            let spirits = tokens(&game, Subtype::Spirit);
            assert_eq!(spirits.iter().filter(|id| game.current_power(**id) == Some(1)).count(), 2);
            for spirit in spirits {
                assert!(game.is_tapped(spirit));
                assert!(game.current_has_static_ability_id(spirit, ironsmith::static_abilities::StaticAbilityId::Flying));
            }
        }
    }
}

#[test]
fn no_blueprint_cannot_be_borrowed_from_another_ability_or_runtime_objects() {
    for text in [
        "Type: Sorcery\nCreate two of those tokens.",
        "Type: Creature — Human\nPower/Toughness: 1/1\nWhen this creature enters, create a Treasure token.\nWhen this creature dies, create two of those tokens.",
    ] {
        assert!(compile_to_runtime_definition("Missing lexical blueprint", text, false).is_err());
    }
}

#[test]
fn emerge_prices_the_announced_material_but_snapshots_toughness_immediately_before_payment() {
    for definition in definitions("Adipose Offspring") {
        let mut game = game(); let mut choices = Choices::default();
        let material = game.create_object_from_definition(&probe_definition("Changing material",
            "Mana cost: {2}\nType: Creature — Beast\nPower/Toughness: 1/5"), A, Zone::Battlefield);
        let card = game.create_object_from_definition(&definition, A, Zone::Hand);
        let index = definition.alternative_casts.iter().position(|method| method.name() == "Emerge").unwrap();
        let before = game.player(A).unwrap().mana_pool.total();
        assert!(action_with_resource_change(&mut game, LegalAction::CastSpell {
            spell_id: card, from_zone: Zone::Hand, casting_method: CastingMethod::Alternative(index),
        }, &mut choices, Some((material, 2))), "the mutation must occur after resource announcement and before payment");
        assert_eq!(before - game.player(A).unwrap().mana_pool.total(), 4);
        let receipt = &game.stack.last().unwrap().tagged_objects[ironsmith::tag::SOURCE_EMERGE_SACRIFICE_TAG];
        assert_eq!(receipt[0].toughness, Some(7));
        settle(&mut game, &mut choices);
        assert_creatures(&game, Subtype::Alien, 7, 2, 2, false);
    }
}

#[test]
fn paid_emerge_without_required_sacrifice_evidence_is_atomic_incomplete_execution() {
    for definition in definitions("Adipose Offspring") {
        let mut game = game(); let mut choices = Choices::default();
        game.create_object_from_definition(&probe_definition("Material",
            "Mana cost: {2}\nType: Creature — Beast\nPower/Toughness: 1/5"), A, Zone::Battlefield);
        let index = definition.alternative_casts.iter().position(|method| method.name() == "Emerge").unwrap();
        cast(&mut game, &definition, Zone::Hand, CastingMethod::Alternative(index), &mut choices);
        resolve_stack_entry_with(&mut game, &mut choices).unwrap();
        pending(&mut game, &mut choices);
        game.stack.last_mut().unwrap().tagged_objects.remove(ironsmith::tag::SOURCE_EMERGE_SACRIFICE_TAG);
        let before = game.battlefield.len();
        assert!(resolve_stack_entry_with(&mut game, &mut choices).is_err());
        assert_eq!(game.battlefield.len(), before);
        assert_eq!(tokens(&game, Subtype::Alien).len(), 0);
        assert_eq!(game.stack.len(), 1);
    }
}

#[test]
fn emerge_distinguishes_completed_redirected_sacrifice_from_prevented_or_substituted_costs() {
    for definition in definitions("Adipose Offspring") { for mode in 0..5 {
        let mut game = game(); let mut choices = Choices::default();
        let material = game.create_object_from_definition(&probe_definition("Protected material",
            "Mana cost: {2}\nType: Creature — Beast\nPower/Toughness: 1/5"), A, Zone::Battlefield);
        let shield = game.create_object_from_definition(&probe_definition("Sacrifice shield", "Type: Artifact"), B, Zone::Battlefield);
        let other = game.create_object_from_definition(&probe_definition("Unrelated sacrifice",
            "Type: Creature — Beast\nPower/Toughness: 1/9"), A, Zone::Battlefield);
        let sacrifice_other = Effect::sacrifice_player(ironsmith::ObjectFilter::specific(other), 1, PlayerFilter::Specific(A));
        let replacement = match mode {
            0 => ironsmith::replacement::ReplacementAction::Prevent,
            1 => ironsmith::replacement::ReplacementAction::Instead(vec![Effect::gain_life(1)]),
            2 => ironsmith::replacement::ReplacementAction::ChangeDestination(Zone::Exile),
            3 => ironsmith::replacement::ReplacementAction::Instead(vec![sacrifice_other]),
            _ => ironsmith::replacement::ReplacementAction::Additionally(vec![sacrifice_other]),
        };
        game.effect_store.replacement_effects.add_one_shot_effect(ironsmith::replacement::ReplacementEffect::with_matcher(
            shield, B,
            ironsmith::events::zones::matchers::WouldChangeZoneMatcher::new(
                ironsmith::ObjectFilter::specific(material), Some(Zone::Battlefield), Some(Zone::Graveyard)),
            replacement,
        ));
        let index = definition.alternative_casts.iter().position(|method| method.name() == "Emerge").unwrap();
        cast(&mut game, &definition, Zone::Hand, CastingMethod::Alternative(index), &mut choices);
        let completed = mode == 2 || mode == 4;
        let receipt = &game.stack.last().unwrap().tagged_objects[ironsmith::tag::SOURCE_EMERGE_SACRIFICE_TAG];
        assert_eq!(receipt.len(), usize::from(completed));
        if !completed {
            assert_eq!(game.object(material).unwrap().zone, Zone::Battlefield);
            game.add_counters(material, CounterType::PlusOnePlusOne, 7).unwrap();
            assert_eq!(game.current_toughness(material), Some(12));
        }
        settle(&mut game, &mut choices);
        assert_creatures(&game, Subtype::Alien, if completed { 5 } else { 0 }, 2, 2, false);
    }}
}

#[test]
fn later_separate_sacrifice_of_the_same_material_cannot_overwrite_prevented_emerge_receipt() {
    for mut definition in definitions("Adipose Offspring") {
        // Add an independent native casting component to the complete compiled
        // card. This models an additional cost without faking either payment.
        definition.additional_cost = ironsmith::cost::TotalCost::from_cost(
            ironsmith::costs::Cost::sacrifice(ironsmith::ObjectFilter::creature().you_control()));
        let mut game = game(); let mut choices = Choices::default();
        let material_def = probe_definition("Shared material", "Mana cost: {2}\nType: Creature — Beast\nPower/Toughness: 1/5");
        let material = game.create_object_from_definition(&material_def, A, Zone::Battlefield);
        game.create_object_from_definition(&material_def, A, Zone::Battlefield);
        choices.objects = vec![material];
        let shield = game.create_object_from_definition(&probe_definition("One sacrifice shield", "Type: Artifact"), B, Zone::Battlefield);
        game.effect_store.replacement_effects.add_one_shot_effect(ironsmith::replacement::ReplacementEffect::with_matcher(
            shield, B,
            ironsmith::events::zones::matchers::WouldChangeZoneMatcher::new(
                ironsmith::ObjectFilter::specific(material), Some(Zone::Battlefield), Some(Zone::Graveyard)),
            ironsmith::replacement::ReplacementAction::Prevent,
        ));
        let index = definition.alternative_casts.iter().position(|method| method.name() == "Emerge").unwrap();
        cast(&mut game, &definition, Zone::Hand, CastingMethod::Alternative(index), &mut choices);
        assert!(game.object(material).is_none_or(|object| object.zone != Zone::Battlefield),
            "the separate later sacrifice really completed on that same material");
        assert_eq!(game.stack.last().unwrap().tagged_objects[ironsmith::tag::SOURCE_EMERGE_SACRIFICE_TAG].len(), 0);
        choices.objects.clear();
        settle(&mut game, &mut choices);
        assert_creatures(&game, Subtype::Alien, 0, 2, 2, false);
    }
}

#[test]
fn anduril_controller_creates_tokens_when_a_foreign_wielder_attacks_but_cannot_insert_them_into_combat() {
    for definition in definitions("Andúril, Flame of the West") {
        let mut game = game(); let mut choices = Choices::default();
        let sword = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let wielder = game.create_object_from_definition(&probe_definition("Foreign wielder",
            "Type: Legendary Creature — Human\nPower/Toughness: 2/3"), A, Zone::Battlefield);
        choices.targets = vec![Target::Object(wielder)];
        activate(&mut game, sword, activated(&definition), &mut choices);
        settle(&mut game, &mut choices);
        choices.targets.clear();
        game.set_current_controller(wielder, B).unwrap();
        game.remove_summoning_sickness(wielder);
        game.turn.active_player = B;
        game.turn.priority_player = Some(B);
        game.turn.phase = Phase::Combat;
        game.turn.step = Some(Step::DeclareAttackers);
        game.mark_combat_phase_started();
        game.combat = Some(CombatState::default());
        let mut combat = game.combat.clone().unwrap(); let mut queue = TriggerQueue::new();
        ironsmith::game_loop::apply_attacker_declarations(&mut game, &mut combat, &mut queue,
            &[ironsmith::decision::AttackerDeclaration { creature: wielder, target: AttackTarget::Player(C) }]).unwrap();
        assert_eq!(queue.entries.len(), 1);
        assert_eq!(queue.entries[0].controller, A);
        put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut choices).unwrap();
        settle(&mut game, &mut choices);
        assert_creatures(&game, Subtype::Spirit, 2, 1, 1, true);
        for spirit in tokens(&game, Subtype::Spirit) {
            assert_eq!(game.object(spirit).unwrap().owner, A);
            assert!(game.current_has_static_ability_id(spirit, ironsmith::static_abilities::StaticAbilityId::Flying));
            assert!(game.combat.as_ref().unwrap().attackers.iter().all(|entry| entry.creature != spirit));
        }
    }
}
