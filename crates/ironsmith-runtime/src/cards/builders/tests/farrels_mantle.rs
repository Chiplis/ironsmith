#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

const ABILITY_LINE: &str = "Whenever enchanted creature attacks and isn't blocked, its controller may have it deal damage equal to its power plus 2 to another target creature. If that player does, the attacking creature assigns no combat damage this turn.";

#[derive(Default)]
struct FarrelDecisions {
    accept: bool,
    target: Option<ObjectId>,
    target_players: Vec<PlayerId>,
    may_players: Vec<PlayerId>,
    legal_targets: Vec<ObjectId>,
}

impl crate::decision::DecisionMaker for FarrelDecisions {
    fn decide_targets(
        &mut self,
        _game: &crate::GameState,
        ctx: &crate::decisions::context::TargetsContext,
    ) -> Vec<crate::game_state::Target> {
        self.target_players.push(ctx.player);
        self.legal_targets = ctx
            .requirements
            .iter()
            .flat_map(|requirement| requirement.legal_targets.iter())
            .filter_map(|target| match target {
                crate::game_state::Target::Object(id) => Some(*id),
                crate::game_state::Target::Player(_) => None,
            })
            .collect();
        ctx.requirements
            .iter()
            .filter_map(|requirement| {
                self.target
                    .map(crate::game_state::Target::Object)
                    .filter(|target| requirement.legal_targets.contains(target))
            })
            .collect()
    }

    fn decide_boolean(
        &mut self,
        _game: &crate::GameState,
        ctx: &crate::decisions::context::BooleanContext,
    ) -> bool {
        self.may_players.push(ctx.player);
        self.accept
    }
}

fn creature(name: &str, power: i32, toughness: i32) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(power, toughness))
        .build()
}

fn resolve_farrel_offer(
    accept: bool,
) -> (
    crate::GameState,
    PlayerId,
    PlayerId,
    ObjectId,
    ObjectId,
    FarrelDecisions,
) {
    let definition = parse_oracle_card_definition("Farrel's Mantle");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mantle = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let attacker = game.create_object_from_definition(
        &creature("Bob's Attacker", 3, 3),
        bob,
        Zone::Battlefield,
    );
    let recipient = game.create_object_from_definition(
        &creature("Damage Recipient", 1, 10),
        alice,
        Zone::Battlefield,
    );
    assert!(
        game.attach_object_to_target(mantle, crate::object::AttachmentTarget::Object(attacker),)
    );
    game.refresh_continuous_state();

    let event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::combat::CreatureAttackedAndUnblockedEvent::new(
            attacker,
            crate::events::combat::AttackEventTarget::Player(alice),
        ),
        crate::provenance::ProvNodeId::default(),
    );
    let mut queue = crate::triggers::TriggerQueue::new();
    for entry in crate::triggers::check_triggers(&game, &event)
        .into_iter()
        .filter(|entry| entry.source == mantle)
    {
        queue.add(entry);
    }
    assert_eq!(
        queue.entries.len(),
        1,
        "Farrel's Mantle should trigger once"
    );

    let mut decisions = FarrelDecisions {
        accept,
        target: Some(recipient),
        ..Default::default()
    };
    crate::game_loop::put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut decisions)
        .expect("Farrel's Mantle target should be announced");
    crate::game_loop::resolve_stack_entry_with(&mut game, &mut decisions)
        .expect("Farrel's Mantle trigger should resolve");

    (game, alice, bob, attacker, recipient, decisions)
}

#[test]
fn farrels_mantle_compiles_to_the_exact_linked_surface() {
    let definition = parse_oracle_card_definition("Farrel's Mantle");
    assert_eq!(
        compiled_text_lines(&definition),
        vec!["Enchant creature", ABILITY_LINE]
    );
}

#[test]
fn accepting_uses_the_enchanted_creatures_controller_power_and_identity() {
    let (game, alice, bob, attacker, recipient, decisions) = resolve_farrel_offer(true);

    assert_eq!(decisions.target_players, vec![alice]);
    assert_eq!(
        decisions.may_players,
        vec![bob],
        "the enchanted attacker's controller, not the Aura's controller, decides"
    );
    assert!(decisions.legal_targets.contains(&recipient));
    assert!(
        !decisions.legal_targets.contains(&attacker),
        "another target creature must exclude the grammatical damage source"
    );
    assert_eq!(
        game.damage_on(recipient),
        5,
        "the amount must be the attacker's power plus 2"
    );
    assert!(
        game.combat_damage_assignment_is_suppressed(attacker),
        "the accepted offer must suppress combat assignment for that same attacker"
    );
}

#[test]
fn declining_deals_no_damage_and_does_not_suppress_combat_assignment() {
    let (game, _alice, bob, attacker, recipient, decisions) = resolve_farrel_offer(false);

    assert_eq!(decisions.may_players, vec![bob]);
    assert_eq!(game.damage_on(recipient), 0);
    assert!(!game.combat_damage_assignment_is_suppressed(attacker));
}
