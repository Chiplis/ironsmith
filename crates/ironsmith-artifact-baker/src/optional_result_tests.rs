use super::*;
use engine::ability::AbilityKind;
use engine::decision::DecisionMaker;
use engine::decisions::context::{BooleanContext, NumberContext, TargetsContext};
use engine::game_state::{StackEntry, Target};
use engine::{CardDefinition, GameState, ObjectId, PlayerId, Zone};
use ironsmith_runtime_catalog as engine;

fn compile(name: &str, text: &str) -> CardDefinition {
    let artifact = compile_artifact(CompileInput {
        name,
        text,
        score: Some(1.0),
        local_id: 1,
        other_face_id: None,
        other_face_name: None,
        layout: LinkedFaceLayout::None,
    })
    .expect("strict compile and artifact materialization");
    engine::artifact_materializer::materialize_artifact(&artifact).unwrap()
}

fn trigger(definition: &CardDefinition, index: usize) -> engine::ability::TriggeredAbility {
    definition
        .abilities
        .iter()
        .filter_map(|a| match &a.kind {
            AbilityKind::Triggered(t) => Some(t.clone()),
            _ => None,
        })
        .nth(index)
        .unwrap()
}
fn game() -> GameState {
    GameState::new(vec!["Alice".into(), "Bob".into()], 20)
}
fn creature(game: &mut GameState, player: PlayerId, name: &str) -> ObjectId {
    let card =
        engine::cards::builders::CardDefinitionBuilder::new(engine::ids::CardId::new(), name)
            .card_types(vec![engine::types::CardType::Creature])
            .power_toughness(engine::card::PowerToughness::fixed(2, 2))
            .build();
    game.create_object_from_definition(&card, player, Zone::Battlefield)
}
#[derive(Default)]
struct Choices {
    accept: bool,
    x: Option<u32>,
    targets: Vec<Target>,
    target_prompts: usize,
    may_prompts: usize,
}
impl DecisionMaker for Choices {
    fn decide_boolean(&mut self, _: &GameState, _: &BooleanContext) -> bool {
        self.may_prompts += 1;
        self.accept
    }
    fn decide_number(&mut self, _: &GameState, ctx: &NumberContext) -> u32 {
        self.x.unwrap_or(2).max(ctx.min)
    }
    fn decide_targets(&mut self, _: &GameState, ctx: &TargetsContext) -> Vec<Target> {
        self.target_prompts += 1;
        for target in &self.targets {
            assert!(
                ctx.requirements
                    .iter()
                    .any(|r| r.legal_targets.contains(target)),
                "target {target:?} missing from {ctx:?}"
            );
        }
        self.targets.clone()
    }
}
fn resolve(game: &mut GameState, choices: &mut Choices) {
    engine::game_loop::resolve_stack_entry_with(game, choices).unwrap();
}

const CAMPSITE: &str = r#"Mana cost: {1}{G}
Type: Enchantment
Whenever this enchantment or a legendary creature you control enters, create a Food token.
Whenever you attack, you may sacrifice X Foods. When you do, up to X target attacking creatures each get +3/+3 and gain trample and indestructible until end of turn."#;

const KITT: &str = r#"Mana cost: {1}{R}{G}{W}
Type: Legendary Creature — Cat Bard Druid
Power/Toughness: 3/3
When Kitt Kanto enters, create a 1/1 green and white Citizen creature token.
At the beginning of combat on each player's turn, you may tap two untapped creatures you control. When you do, target creature that player controls gets +2/+2 and gains trample until end of turn. Goad that creature."#;

const SPIRIT: &str = r#"Mana cost: {3}{W}{B}
Type: Enchantment
At the beginning of your end step, choose target permanent card in your graveyard. You may sacrifice a permanent that shares a card type with the chosen card. If you do, return the chosen card from your graveyard to the battlefield and it gains "If this permanent would leave the battlefield, exile it instead of putting it anywhere else.""#;

const ROSE: &str = r#"Mana cost: {3}{R}
Type: Creature — Ogre Warrior
Power/Toughness: 4/3
Alliance — Whenever another creature you control enters, create a Treasure token if this is the first or second time this ability has resolved this turn. Otherwise, you may pay {X}. When you do, this creature deals X damage to any target."#;

const PUPPET: &str = r#"Mana cost: {U}{U}{U}
Type: Enchantment — Aura
Enchant creature
When enchanted creature dies, return that card to its owner's hand. If that card is returned to its owner's hand this way, you may pay {U}{U}{U}. If you do, return this card to its owner's hand."#;

#[test]
fn campsite_optional_sacrifice_gates_the_entire_attacker_bonus() {
    let definition = compile("Campsite Cuisine", CAMPSITE);
    let attack = trigger(&definition, 1);
    for accept in [false, true] {
        let mut game = game();
        let alice = game.players[0].id;
        let bob = game.players[1].id;
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let attackers = [
            creature(&mut game, alice, "First"),
            creature(&mut game, alice, "Second"),
        ];
        for id in attackers {
            game.remove_summoning_sickness(id);
        }
        game.turn.active_player = alice;
        game.turn.phase = engine::game_state::Phase::Combat;
        game.turn.step = Some(engine::game_state::Step::DeclareAttackers);
        let mut combat = engine::combat_state::CombatState::default();
        engine::game_loop::apply_attacker_declarations(
            &mut game,
            &mut combat,
            &mut engine::triggers::TriggerQueue::new(),
            &attackers.map(|id| engine::decision::AttackerDeclaration {
                creature: id,
                target: engine::combat_state::AttackTarget::Player(bob),
            }),
        )
        .unwrap();
        let food =
            engine::cards::builders::CardDefinitionBuilder::new(engine::ids::CardId::new(), "Food")
                .card_types(vec![engine::types::CardType::Artifact])
                .subtypes(vec![engine::types::Subtype::Food])
                .build();
        for _ in 0..2 {
            game.create_object_from_definition(&food, alice, Zone::Battlefield);
        }
        let mut choices = Choices {
            accept,
            targets: attackers.map(Target::Object).to_vec(),
            ..Default::default()
        };
        game.push_to_stack(StackEntry::ability(source, alice, attack.effects.clone()).with_x(2));
        resolve(&mut game, &mut choices);
        assert_eq!(game.stack.len(), usize::from(accept));
        assert_eq!(choices.target_prompts, usize::from(accept));
        if accept {
            resolve(&mut game, &mut choices);
        }
        for attacker in attackers {
            assert_eq!(
                game.calculated_power(attacker),
                Some(if accept { 5 } else { 2 })
            );
            for keyword in [
                engine::static_abilities::StaticAbilityId::Trample,
                engine::static_abilities::StaticAbilityId::Indestructible,
            ] {
                assert_eq!(game.object_has_static_ability_id(attacker, keyword), accept);
            }
        }
        assert_eq!(game.players[0].graveyard.len(), if accept { 2 } else { 0 });
    }
}

#[test]
fn kitt_optional_tap_gates_bonus_goad_and_active_player_targets() {
    let definition = compile("Kitt Kanto, Mayhem Diva", KITT);
    let combat_trigger = trigger(&definition, 1);
    for accept in [false, true] {
        let mut game = game();
        let alice = game.players[0].id;
        let bob = game.players[1].id;
        game.turn.active_player = bob;
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let helper = creature(&mut game, alice, "Helper");
        let target = creature(&mut game, bob, "Target");
        let mut choices = Choices {
            accept,
            targets: vec![Target::Object(target)],
            ..Default::default()
        };
        let event = engine::triggers::TriggerEvent::new(
            engine::events::phase::BeginningOfCombatEvent::new(bob),
            Default::default(),
        );
        game.push_to_stack(
            StackEntry::ability(source, alice, combat_trigger.effects.clone())
                .with_triggering_event(event),
        );
        resolve(&mut game, &mut choices);
        assert_eq!(game.stack.len(), usize::from(accept));
        assert_eq!(choices.target_prompts, usize::from(accept));
        if accept {
            resolve(&mut game, &mut choices);
        }
        assert_eq!(
            game.calculated_power(target),
            Some(if accept { 4 } else { 2 })
        );
        assert_eq!(game.is_goaded(target), accept);
        assert_eq!(game.is_tapped(source), accept);
        assert_eq!(game.is_tapped(helper), accept);
    }
}

#[test]
fn rose_treasure_branches_and_optional_payment_use_distinct_results() {
    let definition = compile("Rose Room Treasurer", ROSE);
    let ability = trigger(&definition, 0);
    for (previous, accept, x) in [
        (0, false, 2),
        (1, false, 2),
        (2, false, 2),
        (2, true, 2),
        (2, true, 0),
    ] {
        let mut game = game();
        let alice = game.players[0].id;
        let bob = game.players[1].id;
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        game.players[0]
            .mana_pool
            .add(engine::mana::ManaSymbol::Red, 2);
        let identity = engine::triggers::compute_trigger_identity(&ability);
        for _ in 0..previous {
            game.record_triggered_ability_resolved(source, identity);
        }
        let mut choices = Choices {
            accept,
            x: Some(x),
            targets: vec![Target::Player(bob)],
            ..Default::default()
        };
        game.push_to_stack(
            StackEntry::ability(source, alice, ability.effects.clone())
                .with_trigger_identity(identity),
        );
        resolve(&mut game, &mut choices);
        let damages = previous >= 2 && accept;
        assert_eq!(choices.may_prompts, usize::from(previous >= 2));
        assert_eq!(game.stack.len(), usize::from(damages));
        assert_eq!(choices.target_prompts, usize::from(damages));
        if damages {
            resolve(&mut game, &mut choices);
        }
        assert_eq!(
            game.players[1].life,
            if damages { 20 - x as i32 } else { 20 }
        );
        assert_eq!(
            game.players[0].mana_pool.total(),
            if damages { 2 - x } else { 2 }
        );
        assert_eq!(
            game.battlefield
                .iter()
                .filter(|id| game.object(**id).unwrap().name == "Treasure")
                .count(),
            usize::from(previous < 2)
        );
    }
}

#[test]
fn spirit_sacrifice_gates_return_and_grants_removable_zone_replacement() {
    let definition = compile("Spirit-Sister's Call", SPIRIT);
    let ability = trigger(&definition, 0);
    for (accept, remove_ability) in [(false, false), (true, false), (true, true)] {
        let mut game = game();
        let alice = game.players[0].id;
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let sacrifice = creature(&mut game, alice, "Sacrifice");
        let target = creature(&mut game, alice, "Return");
        let target = game.move_object_by_effect(target, Zone::Graveyard).unwrap();
        let stable = game.object(target).unwrap().stable_id;
        let mut choices = Choices {
            accept,
            ..Default::default()
        };
        game.push_to_stack(
            StackEntry::ability(source, alice, ability.effects.clone())
                .with_targets(vec![Target::Object(target)]),
        );
        resolve(&mut game, &mut choices);
        let returned = game.find_object_by_stable_id(stable).unwrap();
        assert_eq!(
            game.object(returned).unwrap().zone,
            if accept {
                Zone::Battlefield
            } else {
                Zone::Graveyard
            }
        );
        assert_eq!(game.object(sacrifice).is_some(), !accept);
        if accept {
            if remove_ability {
                let strip = compile(
                    "Remove abilities",
                    "Type: Sorcery\nTarget creature loses all abilities until end of turn.",
                );
                game.push_to_stack(
                    StackEntry::ability(source, alice, strip.spell_effect.unwrap())
                        .with_targets(vec![Target::Object(returned)]),
                );
                resolve(&mut game, &mut choices);
            }
            let effect =
                engine::effect::Effect::new(engine::effects::ReturnToHandEffect::with_spec(
                    engine::target::ChooseSpec::SpecificObject(returned),
                ));
            game.push_to_stack(StackEntry::ability(source, alice, vec![effect]));
            resolve(&mut game, &mut choices);
            let moved = game.find_object_by_stable_id(stable).unwrap();
            assert_eq!(
                game.object(moved).unwrap().zone,
                if remove_ability {
                    Zone::Hand
                } else {
                    Zone::Exile
                }
            );
        }
    }
}

#[test]
fn puppet_return_requires_payment_after_the_creature_returns() {
    let definition = compile("Puppet Master", PUPPET);
    let ability = trigger(&definition, 0);
    for accept in [false, true] {
        let mut game = game();
        let alice = game.players[0].id;
        let source = game.create_object_from_definition(&definition, alice, Zone::Graveyard);
        let stable = game.object(source).unwrap().stable_id;
        let dead = creature(&mut game, alice, "Dead creature");
        let death_snapshot =
            engine::snapshot::ObjectSnapshot::from_object(game.object(dead).unwrap(), &game);
        let old_dead = dead;
        let dead = game.move_object_by_effect(dead, Zone::Graveyard).unwrap();
        let dead_stable = game.object(dead).unwrap().stable_id;
        let snapshot =
            engine::snapshot::ObjectSnapshot::from_object(game.object(dead).unwrap(), &game);
        let mut tags = std::collections::HashMap::new();
        tags.insert(engine::tag::TagKey::from("__it__"), vec![snapshot]);
        game.players[0]
            .mana_pool
            .add(engine::mana::ManaSymbol::Blue, 3);
        let mut choices = Choices {
            accept,
            ..Default::default()
        };
        let death = engine::events::zones::ZoneChangeEvent::with_results(
            old_dead,
            vec![dead],
            Zone::Battlefield,
            Zone::Graveyard,
            engine::events::EventCause::from_game_rule(),
            Some(death_snapshot),
        );
        game.push_to_stack(
            StackEntry::ability(source, alice, ability.effects.clone())
                .with_tagged_objects(tags)
                .with_triggering_event(engine::triggers::TriggerEvent::new(
                    death,
                    Default::default(),
                )),
        );
        resolve(&mut game, &mut choices);
        assert_eq!(choices.may_prompts, 1);
        let creature = game.find_object_by_stable_id(dead_stable).unwrap();
        assert_eq!(game.object(creature).unwrap().zone, Zone::Hand);
        let aura = game.find_object_by_stable_id(stable).unwrap();
        assert_eq!(
            game.object(aura).unwrap().zone,
            if accept { Zone::Hand } else { Zone::Graveyard }
        );
    }
}
