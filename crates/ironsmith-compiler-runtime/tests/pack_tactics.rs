//! Pack tactics is an intervening-if over a declaration-time historical fact.
//! The same artifact and real attack/stack paths exercise the trigger gate and
//! its resolution recheck, rather than hand-seeding bookkeeping maps.
use ironsmith::ability::AbilityKind;
use ironsmith::card::{CardBuilder, PowerToughness};
use ironsmith::cards::{CardDefinition, generated_definition_has_unimplemented_content};
use ironsmith::combat_state::{AttackTarget, CombatState};
use ironsmith::decision::{AttackerDeclaration, SelectFirstDecisionMaker};
use ironsmith::effects::{CreateTokenEffect, EffectContext, EffectExecutor};
use ironsmith::game_loop::{
    apply_attacker_declarations, put_triggers_on_stack_with_dm, resolve_stack_entry_with,
};
use ironsmith::game_state::{Phase, Step};
use ironsmith::object::CounterType;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{CardId, CardType, ConditionExpr, GameState, ObjectId, PlayerId, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::compile_to_artifact;
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;

const PREDICATE: &str = "you attacked with creatures with total power 6 or greater this combat";
const DRAW_PROBE: &str = "Type: Creature — Scout\nPower/Toughness: 3/3\nWhenever this creature attacks, if you attacked with creatures with total power 6 or greater this combat, draw a card.";
const ALICE: PlayerId = PlayerId(0);
const BOB: PlayerId = PlayerId(1);

fn compile(name: &str, text: &str) -> CardDefinition {
    let (artifact, _) =
        compile_to_artifact(name, text, false).unwrap_or_else(|error| panic!("{name}: {error}"));
    let encoded = artifact.to_json().unwrap();
    let restored = CompiledCardArtifact::from_json(&encoded).unwrap();
    assert_eq!(artifact, restored);
    let definition = materialize_artifact(&restored).unwrap();
    assert!(
        !generated_definition_has_unimplemented_content(&definition),
        "{name}"
    );
    definition
}

fn sources() -> Vec<(String, String, i32)> {
    let source: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/card-failure-campaign/predicates/pack-tactics.json"
    ))
    .unwrap();
    source
        .as_array()
        .unwrap()
        .iter()
        .map(|card| {
            let text = format!(
                "Mana cost: {}\nType: {}\nPower/Toughness: {}/{}\n{}",
                card["mana_cost"].as_str().unwrap(),
                card["type_line"].as_str().unwrap(),
                card["power"].as_str().unwrap(),
                card["toughness"].as_str().unwrap(),
                card["oracle_text"].as_str().unwrap(),
            );
            (
                card["name"].as_str().unwrap().to_owned(),
                text,
                card["power"].as_str().unwrap().parse().unwrap(),
            )
        })
        .collect()
}

fn assert_gate(definition: &CardDefinition) {
    let gates = definition.abilities.iter().filter(|ability| {
        matches!(&ability.kind, AbilityKind::Triggered(trigger)
            if trigger.intervening_if == Some(ConditionExpr::AttackedWithTotalPowerAtLeastThisCombat(6)))
    }).count();
    assert_eq!(
        gates, 1,
        "{} must retain an intervening-if gate",
        definition.card.name
    );
    let text = ironsmith_text::canonical_compiled_lines(definition).join("\n");
    assert!(text.contains(PREDICATE), "{text}");
}

fn vanilla(power: i32) -> CardDefinition {
    CardDefinition::new(
        CardBuilder::new(CardId::new(), "Attack partner")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(power, 10))
            .build(),
    )
}

fn game() -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.turn_number = 4;
    game.turn.active_player = ALICE;
    game.turn.priority_player = Some(ALICE);
    game.turn.phase = Phase::Combat;
    game.turn.step = Some(Step::DeclareAttackers);
    game.mark_combat_phase_started();
    game.combat = Some(CombatState::default());
    for player in [ALICE, BOB] {
        for _ in 0..8 {
            game.create_object_from_definition(&vanilla(2), player, Zone::Library);
        }
    }
    game
}

fn add(game: &mut GameState, definition: &CardDefinition, controller: PlayerId) -> ObjectId {
    let id = game.create_object_from_definition(definition, controller, Zone::Battlefield);
    game.remove_summoning_sickness(id);
    id
}

fn declare(game: &mut GameState, ids: &[ObjectId], defender: PlayerId) -> TriggerQueue {
    let declarations = ids
        .iter()
        .map(|creature| AttackerDeclaration {
            creature: *creature,
            target: AttackTarget::Player(defender),
        })
        .collect::<Vec<_>>();
    let mut combat = game.combat.clone().unwrap_or_default();
    let mut queue = TriggerQueue::new();
    apply_attacker_declarations(game, &mut combat, &mut queue, &declarations).unwrap();
    queue
}

fn stack(game: &mut GameState, queue: &mut TriggerQueue) {
    put_triggers_on_stack_with_dm(game, queue, &mut SelectFirstDecisionMaker).unwrap();
}

fn resolve(game: &mut GameState) {
    while !game.stack.is_empty() {
        resolve_stack_entry_with(game, &mut SelectFirstDecisionMaker).unwrap();
        ironsmith::game_loop::put_triggers_on_stack_with_dm(
            game,
            &mut TriggerQueue::new(),
            &mut SelectFirstDecisionMaker,
        )
        .unwrap();
    }
}

fn gate_at_resolution(game: &GameState, source: ObjectId, controller: PlayerId) -> bool {
    ironsmith::condition_eval::evaluate_condition_resolution(
        game,
        &ConditionExpr::AttackedWithTotalPowerAtLeastThisCombat(6),
        &EffectContext::new_default(source, controller),
    )
    .unwrap()
}

#[test]
fn pack_tactics_all_eight_frozen_cards_round_trip_and_gate_real_declarations() {
    assert_eq!(sources().len(), 8);
    for (name, text, source_power) in sources() {
        let definition = compile(&name, &text);
        assert_gate(&definition);
        // Renaming the sole self-referential named card exercises the same
        // grammar without allowing a card-name-specific repair.
        let renamed_text = text.replace("Targ Nar", "Synthetic Pack Scout");
        assert_gate(&compile("Synthetic Pack Scout", &renamed_text));
        for total in [5, 6] {
            let mut game = game();
            let source = add(&mut game, &definition, ALICE);
            let partner = add(&mut game, &vanilla(total - source_power), ALICE);
            let mut queue = declare(&mut game, &[source, partner], BOB);
            assert_eq!(
                queue.entries.len(),
                usize::from(total == 6),
                "{name}: total power {total}"
            );
            stack(&mut game, &mut queue);
            assert_eq!(game.stack.len(), usize::from(total == 6), "{name}");
            if total == 6 {
                assert_eq!(
                    game.stack[0].intervening_if,
                    Some(ConditionExpr::AttackedWithTotalPowerAtLeastThisCombat(6)),
                    "{name}"
                );
            }
        }
        let mut game = game();
        add(&mut game, &definition, ALICE);
        let partner = add(&mut game, &vanilla(6), ALICE);
        assert!(
            declare(&mut game, &[partner], BOB).entries.is_empty(),
            "{name}: its own attack is required"
        );
    }
}

#[test]
fn pack_tactics_keeps_declared_power_after_departures_power_loss_and_token_disappearance() {
    for disappearance in [false, true] {
        let mut game = game();
        let definition = compile("Synthetic Pack Scout", DRAW_PROBE);
        let source = add(&mut game, &definition, ALICE);
        let partner = if disappearance {
            let outcome = CreateTokenEffect::one(vanilla(3))
                .execute(&mut game, &mut EffectContext::new_default(source, ALICE))
                .unwrap();
            let ironsmith::effect::OutcomeValue::Objects(ids) = outcome.value else {
                panic!("token creation must return the token");
            };
            game.remove_summoning_sickness(ids[0]);
            ids[0]
        } else {
            add(&mut game, &vanilla(3), ALICE)
        };
        let mut queue = declare(&mut game, &[source, partner], BOB);
        assert_eq!(queue.entries.len(), 1);
        stack(&mut game, &mut queue);
        assert!(gate_at_resolution(&game, source, ALICE));
        game.add_counters(source, CounterType::MinusOneMinusOne, 2)
            .unwrap();
        if disappearance {
            game.remove_object(partner);
        } else {
            game.move_object_by_effect(partner, Zone::Graveyard)
                .unwrap();
        }
        assert!(gate_at_resolution(&game, source, ALICE));
        assert!(!gate_at_resolution(&game, source, BOB));
        resolve(&mut game);
        assert_eq!(game.player(ALICE).unwrap().hand.len(), 1);
    }
}

#[test]
fn pack_tactics_does_not_retroactively_trigger_after_growth_or_entering_attacking() {
    let mut game = game();
    let source = add(
        &mut game,
        &compile("Synthetic Pack Scout", DRAW_PROBE),
        ALICE,
    );
    let partner = add(&mut game, &vanilla(2), ALICE);
    let mut queue = declare(&mut game, &[source, partner], BOB);
    assert!(queue.entries.is_empty());
    game.add_counters(partner, CounterType::PlusOnePlusOne, 5)
        .unwrap();
    let token = vanilla(9);
    let effect = CreateTokenEffect::one(token).tapped().attacking();
    effect
        .execute(&mut game, &mut EffectContext::new_default(source, ALICE))
        .unwrap();
    assert_eq!(game.combat.as_ref().unwrap().attackers.len(), 3);
    assert!(!gate_at_resolution(&game, source, ALICE));
    stack(&mut game, &mut queue);
    resolve(&mut game);
    assert_eq!(game.player(ALICE).unwrap().hand.len(), 0);
}

#[test]
fn pack_tactics_uses_signed_power_without_clamping_each_creature() {
    let mut game = game();
    let source = add(
        &mut game,
        &compile("Synthetic Pack Scout", DRAW_PROBE),
        ALICE,
    );
    let partner = add(&mut game, &vanilla(3), ALICE);
    let negative = add(&mut game, &vanilla(-1), ALICE);
    assert!(
        declare(&mut game, &[source, partner, negative], BOB)
            .entries
            .is_empty()
    );
    assert_eq!(
        game.turn_store
            .turn_history
            .declared_attack_power_in_combat(1, ALICE),
        5
    );
}

#[test]
fn pack_tactics_extra_combats_and_other_players_have_separate_declarations() {
    let definition = compile("Synthetic Pack Scout", DRAW_PROBE);
    let mut game = game();
    let source = add(&mut game, &definition, ALICE);
    let partner = add(&mut game, &vanilla(3), ALICE);
    let mut queue = declare(&mut game, &[source, partner], BOB);
    stack(&mut game, &mut queue);
    resolve(&mut game);
    assert_eq!(game.player(ALICE).unwrap().hand.len(), 1);
    game.mark_combat_phase_started();
    game.combat = Some(CombatState::default());
    game.untap(source);
    assert!(!gate_at_resolution(&game, source, ALICE));
    assert!(declare(&mut game, &[source], BOB).entries.is_empty());
    assert_eq!(
        game.turn_store
            .turn_history
            .declared_attack_power_in_combat(1, ALICE),
        6
    );
    assert_eq!(
        game.turn_store
            .turn_history
            .declared_attack_power_in_combat(2, ALICE),
        3
    );

    game.turn.active_player = BOB;
    game.turn.priority_player = Some(BOB);
    game.mark_combat_phase_started();
    game.combat = Some(CombatState::default());
    let other = add(&mut game, &definition, BOB);
    let other_partner = add(&mut game, &vanilla(3), BOB);
    let mut queue = declare(&mut game, &[other, other_partner], ALICE);
    assert_eq!(queue.entries.len(), 1);
    assert!(!gate_at_resolution(&game, source, ALICE));
    assert!(gate_at_resolution(&game, other, BOB));
    stack(&mut game, &mut queue);
    resolve(&mut game);
    assert_eq!(game.player(BOB).unwrap().hand.len(), 1);
    game.turn_store.turn_history.clear_for_new_turn();
    assert!(!gate_at_resolution(&game, other, BOB));
}

#[test]
fn pack_tactics_captures_layered_and_attacking_only_power_before_trigger_effects() {
    let anthem = compile(
        "Pack Banner",
        "Type: Enchantment\nCreatures you control get +1/+1.",
    );
    for attacking_only in [false, true] {
        let mut game = game();
        let definition = if attacking_only {
            compile("Synthetic Pack Scout", &DRAW_PROBE.replace("3/3", "2/3"))
        } else {
            compile("Synthetic Pack Scout", DRAW_PROBE)
        };
        let source = add(&mut game, &definition, ALICE);
        let partner = if attacking_only {
            add(
                &mut game,
                &compile(
                    "Charging Companion",
                    "Type: Creature — Scout\nPower/Toughness: 3/4\nThis creature gets +1/+0 as long as it's attacking.",
                ),
                ALICE,
            )
        } else {
            add(&mut game, &vanilla(1), ALICE)
        };
        if !attacking_only {
            add(&mut game, &anthem, ALICE);
        }
        assert_eq!(
            game.current_power(source).unwrap() + game.current_power(partner).unwrap(),
            if attacking_only { 5 } else { 6 }
        );
        let mut queue = declare(&mut game, &[source, partner], BOB);
        assert_eq!(queue.entries.len(), 1, "attacking-only = {attacking_only}");
        assert_eq!(
            game.turn_store
                .turn_history
                .declared_attack_power_in_combat(1, ALICE),
            6
        );
        stack(&mut game, &mut queue);
        resolve(&mut game);
        assert_eq!(game.player(ALICE).unwrap().hand.len(), 1);
    }

    // An ordinary attack trigger's later counter must not retroactively make
    // a below-threshold pack-tactics ability trigger.
    let mut game = game();
    let source = add(
        &mut game,
        &compile("Synthetic Pack Scout", DRAW_PROBE),
        ALICE,
    );
    let partner = add(
        &mut game,
        &compile(
            "Growing Companion",
            "Type: Creature — Scout\nPower/Toughness: 2/4\nWhenever this creature attacks, put a +1/+1 counter on this creature.",
        ),
        ALICE,
    );
    let mut queue = declare(&mut game, &[source, partner], BOB);
    assert_eq!(queue.entries.len(), 1);
    assert_eq!(queue.entries[0].source, partner);
    stack(&mut game, &mut queue);
    resolve(&mut game);
    assert_eq!(game.current_power(partner), Some(3));
    assert!(!gate_at_resolution(&game, source, ALICE));
    assert_eq!(game.player(ALICE).unwrap().hand.len(), 0);
}

#[test]
fn pack_tactics_historical_controller_and_source_departure_do_not_change_the_fact() {
    let mut game = game();
    let source = add(
        &mut game,
        &compile("Synthetic Pack Scout", DRAW_PROBE),
        ALICE,
    );
    let partner = add(&mut game, &vanilla(3), ALICE);
    let mut queue = declare(&mut game, &[source, partner], BOB);
    stack(&mut game, &mut queue);
    game.set_current_controller(partner, BOB).unwrap();
    assert!(gate_at_resolution(&game, source, ALICE));
    assert!(!gate_at_resolution(&game, source, BOB));
    game.move_object_by_effect(source, Zone::Graveyard).unwrap();
    resolve(&mut game);
    assert_eq!(game.player(ALICE).unwrap().hand.len(), 1);
}

#[test]
fn pack_tactics_composed_intervening_if_is_rechecked_on_resolution() {
    let definition = compile(
        "Conditional Pack Scout",
        &DRAW_PROBE.replace(
            PREDICATE,
            &format!("{PREDICATE} and you control an artifact"),
        ),
    );
    for keep_artifact in [false, true] {
        let mut game = game();
        let source = add(&mut game, &definition, ALICE);
        let partner = add(&mut game, &vanilla(3), ALICE);
        let artifact = game.create_object_from_card(
            &CardBuilder::new(CardId::new(), "Gate artifact")
                .card_types(vec![CardType::Artifact])
                .build(),
            ALICE,
            Zone::Battlefield,
        );
        let mut queue = declare(&mut game, &[source, partner], BOB);
        assert_eq!(queue.entries.len(), 1);
        stack(&mut game, &mut queue);
        assert!(matches!(
            game.stack[0].intervening_if,
            Some(ConditionExpr::And(_, _))
        ));
        if !keep_artifact {
            game.move_object_by_effect(artifact, Zone::Graveyard)
                .unwrap();
        }
        resolve(&mut game);
        assert_eq!(
            game.player(ALICE).unwrap().hand.len(),
            usize::from(keep_artifact)
        );
    }
}
