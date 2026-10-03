//! Full-card regression for the frozen e8740178 Unicode-panic observation.
//! Source fields are copied verbatim from the audited cards.json, not rewritten
//! to simplify compilation. Both direct runtime and serialized artifact paths run.
//! Rules: https://magic.wizards.com/en/news/feature/assassins-creed-release-notes
//! (Altaïr notes: independent defender choice, copied ETBs, no attack triggers).

use ironsmith::cards::{CardDefinition, generated_definition_has_unimplemented_content};
use ironsmith::combat_state::{AttackTarget, CombatState};
use ironsmith::decision::{AttackerDeclaration, DecisionMaker};
use ironsmith::decisions::context::{SelectOptionsContext, TargetsContext};
use ironsmith::events::EndOfCombatEvent;
use ironsmith::game_loop::{
    apply_attacker_declarations, put_triggers_on_stack_with_dm, resolve_stack_entry_with,
};
use ironsmith::game_state::{Phase, Step};
use ironsmith::object::{CounterType, ObjectKind};
use ironsmith::static_abilities::StaticAbilityId;
use ironsmith::triggers::{TriggerEvent, TriggerQueue, check_delayed_triggers};
use ironsmith::{GameState, ObjectId, PlayerId, Target, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;
use std::collections::VecDeque;

fn alice() -> PlayerId {
    PlayerId::from_index(0)
}
fn bob() -> PlayerId {
    PlayerId::from_index(1)
}
fn cara() -> PlayerId {
    PlayerId::from_index(2)
}
fn memory() -> CounterType {
    CounterType::Named("memory".into())
}

fn definitions() -> [CardDefinition; 2] {
    let card: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/altair_ibn_laahad.json.fixture")).unwrap();
    let name = card["name"].as_str().unwrap();
    let oracle = card["oracle_text"].as_str().unwrap();
    let source = format!(
        "Mana cost: {}\nType: {}\nPower/Toughness: {}/{}\n{}",
        card["mana_cost"].as_str().unwrap(),
        card["type_line"].as_str().unwrap(),
        card["power"].as_str().unwrap(),
        card["toughness"].as_str().unwrap(),
        oracle
    );
    let direct = compile_to_runtime_definition(name, &source, false).unwrap();
    let (artifact, _) = compile_to_artifact(name, &source, false).unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, restored);
    restored.validate().unwrap();
    let restored = materialize_artifact(&restored).unwrap();
    for definition in [&direct, &restored] {
        assert!(!generated_definition_has_unimplemented_content(definition));
        assert_eq!(
            ironsmith_text::canonical_compiled_lines(definition).join("\n"),
            oracle
        );
    }
    [direct, restored]
}

fn fixture(name: &str, source: &str) -> CardDefinition {
    compile_to_runtime_definition(name, source, false).unwrap()
}

fn creature(
    game: &mut GameState,
    name: &str,
    owner: PlayerId,
    zone: Zone,
    assassin: bool,
) -> ObjectId {
    let source = format!(
        "Type: Creature — {}\nPower/Toughness: 2/2",
        if assassin { "Assassin" } else { "Bear" }
    );
    game.create_object_from_definition(&fixture(name, &source), owner, zone)
}

fn setup(definition: &CardDefinition, source_owner: PlayerId) -> (GameState, ObjectId) {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Cara".into()], 20);
    game.turn.active_player = alice();
    game.turn.phase = Phase::Combat;
    game.turn.step = Some(Step::DeclareAttackers);
    game.turn.priority_player = Some(alice());
    let source = game.create_object_from_definition(definition, source_owner, Zone::Battlefield);
    game.set_current_controller(source, alice()).unwrap();
    game.remove_summoning_sickness(source);
    assert!(game.current_has_static_ability_id(source, StaticAbilityId::FirstStrike));
    (game, source)
}

#[derive(Default)]
struct Choices {
    target: Option<ObjectId>,
    offered: Vec<Target>,
    target_prompts: usize,
    defenders: VecDeque<&'static str>,
    chosen_defenders: usize,
}

impl DecisionMaker for Choices {
    fn decide_targets(&mut self, _game: &GameState, ctx: &TargetsContext) -> Vec<Target> {
        assert_eq!(
            ctx.player,
            alice(),
            "the trigger's controller chooses targets"
        );
        assert_eq!(ctx.requirements.len(), 1);
        let requirement = &ctx.requirements[0];
        assert_eq!(requirement.min_targets, 0);
        assert_eq!(requirement.max_targets, Some(1));
        self.target_prompts += 1;
        self.offered = requirement.legal_targets.clone();
        self.target
            .map(|id| {
                assert!(
                    self.offered.contains(&Target::Object(id)),
                    "requested Assassin must be legal: {ctx:?}"
                );
                vec![Target::Object(id)]
            })
            .unwrap_or_default()
    }

    fn decide_options(&mut self, _game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
        // Defender choices expose the actual players/permanents, not the
        // source's previous combat target. Other choices use their minimum.
        if ctx.options.iter().any(|option| option.description == "Bob") {
            assert_eq!(ctx.player, alice());
            let desired = self.defenders.pop_front().unwrap_or("Bob");
            self.chosen_defenders += 1;
            return vec![
                ctx.options
                    .iter()
                    .find(|option| option.description == desired)
                    .unwrap_or_else(|| panic!("missing defender {desired}: {ctx:?}"))
                    .index,
            ];
        }
        ctx.options
            .iter()
            .filter(|option| option.legal)
            .take(ctx.min)
            .map(|option| option.index)
            .collect()
    }
}

fn attack(game: &mut GameState, source: ObjectId, dm: &mut Choices) {
    let mut combat = CombatState::default();
    let mut queue = TriggerQueue::new();
    apply_attacker_declarations(
        game,
        &mut combat,
        &mut queue,
        &[AttackerDeclaration {
            creature: source,
            target: AttackTarget::Player(bob()),
        }],
    )
    .unwrap();
    game.combat = Some(combat);
    assert!(game.is_tapped(source));
    put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap();
    assert_eq!(
        game.stack.len(),
        1,
        "the real attack must announce exactly Altaïr's trigger"
    );
    assert_eq!(game.stack[0].controller, alice());
}

fn tokens(game: &GameState) -> Vec<ObjectId> {
    game.battlefield
        .iter()
        .copied()
        .filter(|id| game.object(*id).unwrap().kind == ObjectKind::Token)
        .collect()
}

fn flush_triggers(game: &mut GameState, dm: &mut Choices) -> usize {
    let mut resolved = 0;
    loop {
        let mut queue = TriggerQueue::new();
        put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap();
        if game.stack_is_empty() {
            return resolved;
        }
        assert!(resolved < 20, "unexpected recurring trigger");
        resolve_stack_entry_with(game, dm).unwrap();
        resolved += 1;
    }
}

fn end_combat(game: &mut GameState, dm: &mut Choices) -> usize {
    game.turn.step = Some(Step::EndCombat);
    let event = TriggerEvent::new_with_provenance(EndOfCombatEvent::new(), Default::default());
    let entries = check_delayed_triggers(game, &event);
    let count = entries.len();
    let mut queue = TriggerQueue::new();
    for entry in entries {
        queue.add(entry);
    }
    put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap();
    flush_triggers(game, dm);
    count
}

#[test]
fn altair_full_attack_exiles_the_optional_assassin_then_copies_all_owned_memories() {
    for definition in definitions() {
        let (mut game, source) = setup(&definition, bob());
        let target = creature(&mut game, "New Assassin", alice(), Zone::Graveyard, true);
        let wrong_owner = creature(&mut game, "Bob Assassin", bob(), Zone::Graveyard, true);
        let wrong_type = creature(&mut game, "Grave Bear", alice(), Zone::Graveyard, false);
        let wrong_zone = creature(&mut game, "Hand Assassin", alice(), Zone::Hand, true);
        let remembered = creature(&mut game, "Old Bear", alice(), Zone::Exile, false);
        let opposing = creature(&mut game, "Bob Memory", bob(), Zone::Exile, true);
        let plain = creature(&mut game, "Unmarked Assassin", alice(), Zone::Exile, true);
        let spell = game.create_object_from_definition(
            &fixture("Remembered Spell", "Type: Instant\nYou gain 1 life."),
            alice(),
            Zone::Exile,
        );
        for id in [remembered, opposing, spell] {
            game.add_counters(id, memory(), 1).unwrap();
        }
        let mut dm = Choices {
            target: Some(target),
            ..Choices::default()
        };
        attack(&mut game, source, &mut dm);
        assert_eq!(
            dm.offered,
            vec![Target::Object(target)],
            "owner, zone, and Assassin creature filters are conjunctive"
        );
        resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert!(
            game.object(target).is_none(),
            "exile must create a new zone identity"
        );
        let exiled = game
            .objects_in_zone(Zone::Exile)
            .into_iter()
            .find(|id| game.object(*id).unwrap().name == "New Assassin")
            .unwrap();
        assert_eq!(game.counter_count(exiled, memory()), 1);
        let created = tokens(&game);
        assert_eq!(created.len(), 2);
        let mut names = created
            .iter()
            .map(|id| game.object(*id).unwrap().name.clone())
            .collect::<Vec<_>>();
        names.sort();
        assert_eq!(names, ["New Assassin", "Old Bear"]);
        for id in &created {
            assert_eq!(game.current_controller(*id), Some(alice()));
            assert_eq!(game.object(*id).unwrap().owner, alice());
            assert!(game.is_tapped(*id));
            assert_eq!(
                game.counter_count(*id, memory()),
                0,
                "memory counters are not copiable"
            );
            assert!(
                game.combat
                    .as_ref()
                    .unwrap()
                    .attackers
                    .iter()
                    .any(|attacker| attacker.creature == *id)
            );
        }
        // Changing a token's controller doesn't evade its identity-linked cleanup.
        game.set_current_controller(created[0], bob()).unwrap();
        assert_eq!(end_combat(&mut game, &mut dm), 2);
        assert!(tokens(&game).is_empty());
        assert_eq!(
            end_combat(&mut game, &mut dm),
            0,
            "cleanup triggers only once"
        );
        for id in [exiled, remembered, opposing, plain, spell] {
            assert_eq!(game.object(id).unwrap().zone, Zone::Exile);
        }
        for id in [wrong_owner, wrong_type] {
            assert_eq!(game.object(id).unwrap().zone, Zone::Graveyard);
        }
        assert_eq!(game.object(wrong_zone).unwrap().zone, Zone::Hand);
    }
}

#[test]
fn altair_zero_targets_still_copies_and_each_token_chooses_its_own_defender() {
    for definition in definitions() {
        let (mut game, source) = setup(&definition, alice());
        // Even with a legal target available, choosing zero doesn't skip the rest.
        let target = creature(
            &mut game,
            "Declined Assassin",
            alice(),
            Zone::Graveyard,
            true,
        );
        for name in ["One", "Two", "Three"] {
            let id = creature(&mut game, name, alice(), Zone::Exile, false);
            game.add_counters(id, memory(), 1).unwrap();
        }
        let walker = game.create_object_from_definition(
            &fixture("Cara Walker", "Type: Planeswalker\nLoyalty: 4"),
            cara(),
            Zone::Battlefield,
        );
        let mut dm = Choices {
            defenders: VecDeque::from(["Bob", "Cara", "Cara Walker"]),
            ..Choices::default()
        };
        attack(&mut game, source, &mut dm);
        assert_eq!(dm.target_prompts, 1);
        assert!(game.stack[0].targets.is_empty());
        resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert_eq!(tokens(&game).len(), 3);
        assert_eq!(dm.chosen_defenders, 3);
        let combat = game.combat.as_ref().unwrap();
        for expected in [
            AttackTarget::Player(bob()),
            AttackTarget::Player(cara()),
            AttackTarget::Planeswalker(walker),
        ] {
            assert!(
                combat
                    .attackers
                    .iter()
                    .any(|attacker| attacker.creature != source && attacker.target == expected)
            );
        }
        assert_eq!(game.object(target).unwrap().zone, Zone::Graveyard);
        assert_eq!(end_combat(&mut game, &mut dm), 3);
    }
}

#[test]
fn altair_illegal_chosen_target_counters_the_whole_trigger_including_old_memories() {
    for definition in definitions() {
        let (mut game, source) = setup(&definition, alice());
        let target = creature(
            &mut game,
            "Departing Assassin",
            alice(),
            Zone::Graveyard,
            true,
        );
        let remembered = creature(&mut game, "Existing Memory", alice(), Zone::Exile, true);
        game.add_counters(remembered, memory(), 1).unwrap();
        let mut dm = Choices {
            target: Some(target),
            ..Choices::default()
        };
        attack(&mut game, source, &mut dm);
        let hand = game.move_object_by_effect(target, Zone::Hand).unwrap();
        let returned = game.move_object_by_effect(hand, Zone::Graveyard).unwrap();
        assert_ne!(target, returned);
        resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert!(
            tokens(&game).is_empty(),
            "all chosen targets illegal means no effects (CR 608.2b)"
        );
        assert!(game.effect_store.delayed_triggers.is_empty());
        assert_eq!(game.object(returned).unwrap().zone, Zone::Graveyard);
        assert_eq!(game.counter_count(remembered, memory()), 1);
    }
}

#[test]
fn altair_trigger_keeps_its_controller_after_source_leaves_and_copies_etbs_not_attack_triggers() {
    for definition in definitions() {
        for leaves in [false, true] {
            let (mut game, source) = setup(&definition, alice());
            let remembered = game.create_object_from_definition(&fixture("Memory Witness", "Type: Creature — Bear\nPower/Toughness: 2/2\nWhen Memory Witness enters, you gain 2 life.\nWhenever Memory Witness attacks, you gain 7 life."), alice(), Zone::Exile);
            game.add_counters(remembered, memory(), 1).unwrap();
            let mut dm = Choices::default();
            attack(&mut game, source, &mut dm);
            if leaves {
                game.move_object_by_effect(source, Zone::Graveyard).unwrap();
            } else {
                game.set_current_controller(source, bob()).unwrap();
            }
            resolve_stack_entry_with(&mut game, &mut dm).unwrap();
            assert_eq!(tokens(&game).len(), 1);
            assert_eq!(game.current_controller(tokens(&game)[0]), Some(alice()));
            assert_eq!(
                flush_triggers(&mut game, &mut dm),
                1,
                "copied ETB triggers, entering attacking does not"
            );
            assert_eq!(game.player(alice()).unwrap().life, 22);
            assert_eq!(game.player(bob()).unwrap().life, 20);
            assert_eq!(end_combat(&mut game, &mut dm), 1);
            assert_eq!(game.object(remembered).unwrap().zone, Zone::Exile);
        }
    }
}

#[test]
fn altair_memory_pool_is_rechecked_each_attack_and_removed_counters_stop_copying() {
    for definition in definitions() {
        let (mut game, source) = setup(&definition, alice());
        let first = creature(&mut game, "First Memory", alice(), Zone::Exile, false);
        let second = creature(&mut game, "Second Memory", alice(), Zone::Exile, false);
        for id in [first, second] {
            game.add_counters(id, memory(), 1).unwrap();
        }
        let mut dm = Choices::default();
        attack(&mut game, source, &mut dm);
        resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        let old_tokens = tokens(&game);
        assert_eq!(old_tokens.len(), 2);
        end_combat(&mut game, &mut dm);
        game.remove_counters(first, memory(), 1, None, None)
            .unwrap();
        game.untap(source);
        game.turn.step = Some(Step::DeclareAttackers);
        attack(&mut game, source, &mut dm);
        resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        let new_tokens = tokens(&game);
        assert_eq!(new_tokens.len(), 1);
        assert!(!old_tokens.contains(&new_tokens[0]));
        assert_eq!(game.object(new_tokens[0]).unwrap().name, "Second Memory");
        assert_eq!(end_combat(&mut game, &mut dm), 1);
    }
}

#[test]
fn altair_first_strike_deals_three_damage_once_in_a_real_declared_combat() {
    for definition in definitions() {
        let (mut game, source) = setup(&definition, alice());
        let mut dm = Choices::default();
        attack(&mut game, source, &mut dm);
        resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert!(tokens(&game).is_empty(), "no memories means no copies");
        let combat = game.combat.clone().unwrap();
        game.turn.step = Some(Step::CombatDamage);
        ironsmith::game_loop::execute_combat_damage_step(&mut game, &combat, true);
        assert_eq!(game.player(bob()).unwrap().life, 17);
        ironsmith::game_loop::execute_combat_damage_step(&mut game, &combat, false);
        assert_eq!(
            game.player(bob()).unwrap().life,
            17,
            "first strike is not double strike"
        );
        assert_eq!(end_combat(&mut game, &mut dm), 0);
    }
}
