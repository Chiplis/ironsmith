//! Full frozen bodies through direct and JSON-restored artifact paths. UNRUN.
use ironsmith::ability::AbilityKind;
use ironsmith::cards::CardDefinition;
use ironsmith::combat_state::{AttackTarget, CombatState};
use ironsmith::decision::{AttackerDeclaration, DecisionMaker, SelectFirstDecisionMaker};
use ironsmith::decisions::context::{BooleanContext, SelectObjectsContext, TargetsContext};
use ironsmith::effects::{DealDamageEffect, EffectContext, EffectExecutor};
use ironsmith::events::EventCause;
use ironsmith::game_loop::{apply_attacker_declarations, put_triggers_on_stack_with_dm, resolve_stack_entry_with};
use ironsmith::triggers::TriggerQueue;
use ironsmith::{GameState, ObjectId, Phase, PlayerId, Target, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
const C: PlayerId = PlayerId(2);
const D: PlayerId = PlayerId(3);
fn rows() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!("../../../fixtures/combat_participant_conditions.json.fixture")).unwrap()
}
fn definitions(name: &str) -> [CardDefinition; 2] {
    let row = rows().into_iter().find(|row| row["name"] == name).unwrap();
    definitions_from(name, row["text"].as_str().unwrap())
}
fn definitions_from(name: &str, text: &str) -> [CardDefinition; 2] {
    let (compiled, loss) = ironsmith_compiler::parse_loss::capture(||
        compile_to_artifact(name, text, false));
    let (artifact, direct) = compiled.unwrap_or_else(|error| panic!("{name}: {error}"));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    artifact.validate().unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, restored);
    [direct, materialize_artifact(&restored).unwrap()]
}
fn game(active: PlayerId) -> GameState {
    let mut game = GameState::new(vec!["A".into(), "B".into(), "C".into(), "D".into()], 20);
    game.turn.active_player = active;
    game.turn.priority_player = Some(active);
    game.turn.phase = Phase::Combat;
    game.turn.step = Some(ironsmith::game_state::Step::DeclareAttackers);
    game.mark_combat_phase_started();
    for player in [A, B, C, D] {
        for _ in 0..4 { object(&mut game, player, Zone::Library, "Type: Artifact"); }
    }
    game
}
fn object(game: &mut GameState, player: PlayerId, zone: Zone, text: &str) -> ObjectId {
    let definition = compile_to_runtime_definition("Combat condition resource", text, false).unwrap();
    let id = game.create_object_from_definition(&definition, player, zone);
    game.remove_summoning_sickness(id);
    id
}
fn creature(game: &mut GameState, player: PlayerId) -> ObjectId {
    object(game, player, Zone::Battlefield, "Type: Creature\nPower/Toughness: 2/2")
}
fn source(game: &mut GameState, definition: &CardDefinition) -> ObjectId {
    let id = game.create_object_from_definition(definition, A, Zone::Battlefield);
    game.remove_summoning_sickness(id);
    id
}
fn declare(game: &mut GameState, attacks: &[(ObjectId, AttackTarget)]) -> TriggerQueue {
    let declarations = attacks.iter().map(|(creature, target)| AttackerDeclaration {
        creature: *creature, target: target.clone(),
    }).collect::<Vec<_>>();
    let mut queue = TriggerQueue::new();
    apply_attacker_declarations(game, &mut CombatState::default(), &mut queue, &declarations).unwrap();
    queue
}
#[derive(Default)]
struct Choices {
    target: Option<ObjectId>,
    chosen: Option<ObjectId>,
    choosers: Vec<PlayerId>,
    pause: bool,
    pending: bool,
    accept_payment: bool,
}
impl DecisionMaker for Choices {
    fn awaiting_choice(&self) -> bool { self.pending }
    fn decide_targets(&mut self, game: &GameState, context: &TargetsContext) -> Vec<Target> {
        if let Some(id) = self.target.filter(|id| context.requirements.iter().any(|requirement|
            requirement.legal_targets.contains(&Target::Object(*id)))) {
            vec![Target::Object(id)]
        } else { SelectFirstDecisionMaker.decide_targets(game, context) }
    }
    fn decide_objects(&mut self, game: &GameState, context: &SelectObjectsContext) -> Vec<ObjectId> {
        self.choosers.push(context.player);
        self.pending = self.pause;
        if let Some(id) = self.chosen.filter(|id| context.candidates.iter().any(|candidate|
            candidate.id == *id && candidate.legal)) { vec![id] }
        else { SelectFirstDecisionMaker.decide_objects(game, context) }
    }
    fn decide_boolean(&mut self, _: &GameState, _: &BooleanContext) -> bool {
        self.pending = self.pause;
        self.accept_payment
    }
    fn decide_mana_payment(&mut self, _: &GameState,
        context: &ironsmith::decisions::context::ManaPaymentContext) -> ironsmith::mana_payment::ManaPaymentResponse {
        ironsmith::mana_payment::ManaPaymentResponse::Confirm {
            plan_id: context.plan.id, request_hash: context.plan.request_hash,
        }
    }
}
fn settle(game: &mut GameState, queue: &mut TriggerQueue, choices: &mut Choices) {
    put_triggers_on_stack_with_dm(game, queue, choices).unwrap();
    for _ in 0..24 {
        if game.stack_is_empty() { return; }
        resolve_stack_entry_with(game, choices).unwrap();
        put_triggers_on_stack_with_dm(game, queue, choices).unwrap();
    }
    panic!("combat trigger stack did not settle");
}
fn tokens(game: &GameState, player: PlayerId) -> Vec<ObjectId> {
    game.battlefield.iter().copied().filter(|id| game.object(*id).is_some_and(|object|
        game.controller_of(object) == player && object.kind == ironsmith::object::ObjectKind::Token)).collect()
}

#[test]
fn all_six_full_frozen_bodies_preserve_typed_programs_without_parse_loss() {
    assert_eq!(rows().len(), 6);
    for row in rows() {
        for definition in definitions(row["name"].as_str().unwrap()) {
            assert!(definition.abilities.iter().any(|ability|
                matches!(&ability.kind, AbilityKind::Triggered(_))));
            let text = ironsmith_text::compiled_text_lines(&definition).join("\n");
            let required = match row["name"].as_str().unwrap() {
                "Ever-Watching Threshold" => "they attacked you and/or a planeswalker you control",
                "Kazuul, Tyrant of the Cliffs" => "you're the defending player",
                "Mirkwood Trapper" => "they aren't attacking you",
                "Norn's Decree" => "one or more players being attacked are poisoned",
                "Scourge of the Throne" => "it's attacking the player with the most life or tied for most life",
                _ => "poison",
            };
            assert!(text.contains(required), "{text}");
        }
    }
}

#[test]
fn threshold_records_every_declared_target_before_attacker_or_planeswalker_leaves() {
    for definition in definitions("Ever-Watching Threshold") {
        for target_kind in ["player", "planeswalker", "battle", "elsewhere"] {
            let mut game = game(B);
            let observer = source(&mut game, &definition);
            let attacker = creature(&mut game, B);
            let walker = object(&mut game, A, Zone::Battlefield, "Type: Planeswalker\nLoyalty: 5");
            let battle = object(&mut game, B, Zone::Battlefield, "Type: Battle — Siege\nDefense: 5");
            assert!(game.set_battle_protector(battle, A));
            let target = match target_kind {
                "player" => AttackTarget::Player(A),
                "planeswalker" => AttackTarget::Planeswalker(walker),
                "battle" => AttackTarget::Battle(battle),
                _ => AttackTarget::Player(C),
            };
            let unrelated = creature(&mut game, B);
            let mut queue = declare(&mut game, &[(unrelated, AttackTarget::Player(D)), (attacker, target)]);
            assert_eq!(queue.entries.len(), usize::from(matches!(target_kind, "player" | "planeswalker")));
            game.move_object(attacker, Zone::Graveyard, EventCause::from_effect(observer, A)).unwrap();
            game.move_object(walker, Zone::Graveyard, EventCause::from_effect(observer, A)).unwrap();
            let saved = game.clone();
            let saved_queue = queue.clone();
            for restore in [false, true] {
                if restore { game = saved.clone(); queue = saved_queue.clone(); }
                settle(&mut game, &mut queue, &mut Choices::default());
                assert_eq!(game.player(A).unwrap().hand.len(), usize::from(matches!(target_kind, "player" | "planeswalker")));
                assert!(game.player(B).unwrap().hand.is_empty());
            }
        }
    }
}

#[test]
fn kazuul_uses_the_exact_defender_and_payer_for_each_attacking_creature() {
    for definition in definitions("Kazuul, Tyrant of the Cliffs") {
        for pay in [false, true] {
            let mut game = game(B);
            source(&mut game, &definition);
            game.player_mut(B).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless, 3);
            let attacker = creature(&mut game, B);
            let unrelated = creature(&mut game, B);
            let walker = object(&mut game, A, Zone::Battlefield, "Type: Planeswalker\nLoyalty: 5");
            let mut queue = declare(&mut game, &[(attacker, AttackTarget::Planeswalker(walker)), (unrelated, AttackTarget::Player(C))]);
            assert_eq!(queue.entries.len(), 1);
            settle(&mut game, &mut queue, &mut Choices { accept_payment: pay, ..Default::default() });
            assert_eq!(tokens(&game, A).len(), usize::from(!pay));
            assert_eq!(game.player(B).unwrap().mana_pool.total(), if pay { 0 } else { 3 });
            for token in tokens(&game, A) {
                assert_eq!(game.calculated_power(token), Some(3));
                assert_eq!(game.calculated_toughness(token), Some(3));
            }
        }
    }
}

#[test]
fn kazuul_payment_uses_the_departing_attackers_last_controller_without_following_a_blink() {
    for definition in definitions("Kazuul, Tyrant of the Cliffs") {
        let mut game = game(B);
        let observer = source(&mut game, &definition);
        let attacker = creature(&mut game, B);
        game.player_mut(C).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless, 3);
        let mut queue = declare(&mut game, &[(attacker, AttackTarget::Player(A))]);
        let mut choices = Choices { accept_payment: true, ..Default::default() };
        put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut choices).unwrap();
        game.set_current_controller(attacker, C).unwrap();
        let exiled = game.move_object(attacker, Zone::Exile, EventCause::from_effect(observer, A)).unwrap();
        let returned = game.move_object(exiled, Zone::Battlefield, EventCause::from_effect(observer, A)).unwrap();
        assert_ne!(returned, attacker);
        let saved = game.clone();
        game.turn_store.turn_history.event_records.clear();
        game.turn_store.turn_history.staged_event_records.clear();
        assert!(matches!(resolve_stack_entry_with(&mut game, &mut choices),
            Err(ironsmith::game_loop::GameLoopError::ExecutionFailed(
                ironsmith::effects::ExecutionError::IncompleteEvidence(_)))));
        assert_eq!(game.stack.len(), 1);
        assert_eq!(game.player(C).unwrap().mana_pool.total(), 3);
        assert!(tokens(&game, A).is_empty());
        game = saved;
        settle(&mut game, &mut queue, &mut choices);
        assert_eq!(game.player(C).unwrap().mana_pool.total(), 0, "the old incarnation's last controller pays");
        assert!(tokens(&game, A).is_empty());
    }
}

#[test]
fn unknown_combat_condition_tails_are_not_accepted_as_the_known_prefix() {
    for text in [
        "Type: Enchantment\nWhenever a player attacks, if they aren't attacking you except during upkeep, draw a card.",
        "Type: Enchantment\nWhenever an opponent attacks, if they attacked you and/or a battle you protect, draw a card.",
        "Type: Enchantment\nWhenever a player attacks, if they aren't attacking you {3}, draw a card.",
        "Type: Creature\nPower/Toughness: 2/2\nWhenever this creature attacks, if you're the defending player unless you sing, draw a card.",
    ] {
        assert!(compile_to_artifact("Unknown combat qualification", text, false).is_err(), "{text}");
    }
}

#[test]
fn every_reached_intervening_body_rejects_missing_attack_event_and_rolls_back() {
    for name in ["Ever-Watching Threshold", "Kazuul, Tyrant of the Cliffs", "Septic Rats",
        "Mirkwood Trapper", "Norn's Decree", "Scourge of the Throne"] {
        for definition in definitions(name) {
            let source_attacks = matches!(name, "Septic Rats" | "Scourge of the Throne");
            let mut game = game(if source_attacks { A } else { B });
            let observer = source(&mut game, &definition);
            let attacker = if source_attacks { observer } else { creature(&mut game, B) };
            let defender = if source_attacks { B } else if matches!(name, "Mirkwood Trapper" | "Norn's Decree") { C } else { A };
            game.player_mut(defender).unwrap().poison_counters = 1;
            let mut queue = declare(&mut game, &[(attacker, AttackTarget::Player(defender))]);
            let mut choices = Choices { chosen: Some(attacker), ..Default::default() };
            put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut choices).unwrap();
            game.stack.retain(|entry| entry.intervening_if.is_some());
            assert_eq!(game.stack.len(), 1, "{name}");
            game.stack.last_mut().unwrap().triggering_event = None;
            let power = game.calculated_power(attacker);
            let life = game.players.iter().map(|player| player.life).collect::<Vec<_>>();
            let hands = game.players.iter().map(|player| player.hand.len()).collect::<Vec<_>>();
            let tapped = game.is_tapped(attacker);
            assert!(matches!(resolve_stack_entry_with(&mut game, &mut choices),
                Err(ironsmith::game_loop::GameLoopError::ExecutionFailed(
                    ironsmith::effects::ExecutionError::IncompleteEvidence(_)))), "{name}");
            assert_eq!(game.stack.len(), 1, "{name}: error restores the unresolved entry");
            assert_eq!(game.calculated_power(attacker), power);
            assert_eq!(game.is_tapped(attacker), tapped);
            assert_eq!(game.players.iter().map(|player| player.life).collect::<Vec<_>>(), life);
            assert_eq!(game.players.iter().map(|player| player.hand.len()).collect::<Vec<_>>(), hands);
            assert!(game.turn_store.additional_phases.is_empty());
            assert!(tokens(&game, A).is_empty());
        }
    }
}

#[test]
fn kazuul_stacks_an_exact_attacker_reference_after_it_has_already_left() {
    for definition in definitions("Kazuul, Tyrant of the Cliffs") {
        let mut game = game(B);
        let observer = source(&mut game, &definition);
        let attacker = creature(&mut game, B);
        game.player_mut(B).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless, 3);
        let mut queue = declare(&mut game, &[(attacker, AttackTarget::Player(A))]);
        game.move_object(attacker, Zone::Graveyard, EventCause::from_effect(observer, A)).unwrap();
        settle(&mut game, &mut queue, &mut Choices { accept_payment: true, ..Default::default() });
        assert_eq!(game.player(B).unwrap().mana_pool.total(), 0);
        assert!(tokens(&game, A).is_empty());
    }
}

#[test]
fn missing_attacker_and_departure_before_stacking_is_typed_incomplete_not_a_payment_decline() {
    for definition in definitions("Kazuul, Tyrant of the Cliffs") {
        let mut game = game(B);
        let observer = source(&mut game, &definition);
        let attacker = creature(&mut game, B);
        let mut queue = declare(&mut game, &[(attacker, AttackTarget::Player(A))]);
        game.move_object(attacker, Zone::Graveyard, EventCause::from_effect(observer, A)).unwrap();
        game.turn_store.turn_history.event_records.clear();
        game.turn_store.turn_history.staged_event_records.clear();
        let mut choices = Choices::default();
        put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut choices).unwrap();
        assert!(matches!(resolve_stack_entry_with(&mut game, &mut choices),
            Err(ironsmith::game_loop::GameLoopError::ExecutionFailed(
                ironsmith::effects::ExecutionError::IncompleteEvidence(_)))));
        assert_eq!(game.stack.len(), 1);
        assert!(tokens(&game, A).is_empty());
    }
}

fn simultaneous_combat_sources(game: &mut GameState, observer: ObjectId) {
    use ironsmith::effects::{ExecuteWithSourceEffect, ForEachObject};
    use ironsmith_core::{ChooseSpec, ObjectFilter};
    let effect = ForEachObject::new(ObjectFilter::creature().opponent_controls(),
        vec![ironsmith::Effect::new(ExecuteWithSourceEffect::new(ChooseSpec::Iterated,
            ironsmith::Effect::new(DealDamageEffect::new(2, ChooseSpec::SpecificPlayer(A)).with_combat(true))))]);
    let outcome = effect.execute(game, &mut EffectContext::new_default(observer, A)).unwrap();
    for event in outcome.events { game.queue_trigger_event(event.provenance(), event); }
}

#[test]
fn norn_groups_b_b_c_damage_per_damage_time_opponent_despite_later_controller_changes() {
    for definition in definitions("Norn's Decree") {
        let mut game = game(B);
        game.set_teams(vec![vec![A, D], vec![B, C]]).unwrap();
        game.enable_shared_team_turns().unwrap();
        let observer = source(&mut game, &definition);
        let first = creature(&mut game, B);
        let second = creature(&mut game, B);
        let third = creature(&mut game, C);
        simultaneous_combat_sources(&mut game, observer);
        for attacker in [first, second, third] { game.set_current_controller(attacker, D).unwrap(); }
        let mut queue = TriggerQueue::new();
        let mut choices = Choices::default();
        put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut choices).unwrap();
        assert_eq!(game.stack.len(), 2, "one poison ability per damage-time opponent");
        let saved = game.clone();
        settle(&mut game, &mut queue, &mut choices);
        for player in [A, B, C, D] {
            assert_eq!(game.player(player).unwrap().poison_counters,
                u32::from(matches!(player, B | C)), "{player:?}");
        }
        game = saved;
        settle(&mut game, &mut queue, &mut choices);
        assert_eq!(game.player(B).unwrap().poison_counters, 1);
        assert_eq!(game.player(C).unwrap().poison_counters, 1);
        assert_eq!(game.player(D).unwrap().poison_counters, 0);
    }
}

#[test]
fn singular_controller_quantifier_is_not_the_plural_opponents_aggregate() {
    for (qualifier, expected) in [("an opponent controls", 2), ("your opponents control", 1)] {
        let text = format!("Type: Enchantment\nWhenever one or more creatures {qualifier} deal combat damage to you, you gain 1 life.");
        for definition in definitions_from("Controller quantifier", &text) {
            let mut game = game(B);
            let observer = source(&mut game, &definition);
            creature(&mut game, B); creature(&mut game, B); creature(&mut game, C);
            simultaneous_combat_sources(&mut game, observer);
            let after_damage = game.player(A).unwrap().life;
            let mut queue = TriggerQueue::new();
            let mut choices = Choices::default();
            put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut choices).unwrap();
            assert_eq!(game.stack.len(), expected);
            settle(&mut game, &mut queue, &mut choices);
            assert_eq!(game.player(A).unwrap().life, after_damage + expected as i32);
            let text = ironsmith_text::compiled_text_lines(&definition).join("\n");
            assert!(text.contains(qualifier), "{text}");
        }
    }
}

#[test]
fn septic_rats_rechecks_poison_and_keeps_its_infect_body() {
    for definition in definitions("Septic Rats") {
        for clear_poison in [false, true] {
            let mut game = game(A);
            let rat = source(&mut game, &definition);
            game.player_mut(B).unwrap().poison_counters = 1;
            let mut queue = declare(&mut game, &[(rat, AttackTarget::Player(B))]);
            assert_eq!(queue.entries.len(), 1);
            if clear_poison { game.player_mut(B).unwrap().poison_counters = 0; }
            settle(&mut game, &mut queue, &mut Choices::default());
            assert_eq!(game.calculated_power(rat), Some(if clear_poison { 2 } else { 3 }));
            let amount = game.calculated_power(rat).unwrap();
            let outcome = DealDamageEffect::new(amount, ironsmith_core::ChooseSpec::SpecificPlayer(B))
                .with_combat(true).execute(&mut game, &mut EffectContext::new_default(rat, A)).unwrap();
            for event in outcome.events { game.queue_trigger_event(event.provenance(), event); }
            assert_eq!(game.player(B).unwrap().life, 20);
            assert_eq!(game.player(B).unwrap().poison_counters, if clear_poison { 2 } else { 4 });
        }
    }
}

#[test]
fn norns_decree_keeps_damage_actor_poison_and_attack_actor_draw_separate() {
    for definition in definitions("Norn's Decree") {
        let mut game = game(B);
        source(&mut game, &definition);
        let attacker = creature(&mut game, B);
        let outcome = DealDamageEffect::new(2, ironsmith_core::ChooseSpec::SpecificPlayer(A))
            .with_combat(true).execute(&mut game, &mut EffectContext::new_default(attacker, B)).unwrap();
        for event in outcome.events { game.queue_trigger_event(event.provenance(), event); }
        settle(&mut game, &mut TriggerQueue::new(), &mut Choices::default());
        assert_eq!(game.player(B).unwrap().poison_counters, 1);
        game.turn.active_player = C;
        game.turn.priority_player = Some(C);
        let other = creature(&mut game, C);
        let mut queue = declare(&mut game, &[(other, AttackTarget::Player(B))]);
        assert_eq!(queue.entries.len(), 1);
        settle(&mut game, &mut queue, &mut Choices::default());
        assert_eq!(game.player(C).unwrap().hand.len(), 1);
        assert!(game.player(A).unwrap().hand.is_empty());
        assert!(game.player(B).unwrap().hand.is_empty());
    }
}

#[test]
fn mirkwood_retains_the_attacking_chooser_across_pending_native_restore() {
    for definition in definitions("Mirkwood Trapper") {
        let mut game = game(B);
        source(&mut game, &definition);
        let attacker = creature(&mut game, B);
        let mut queue = declare(&mut game, &[(attacker, AttackTarget::Player(C))]);
        assert_eq!(queue.entries.len(), 1);
        let mut choices = Choices { chosen: Some(attacker), ..Default::default() };
        put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut choices).unwrap();
        let saved = game.clone();
        choices.pause = true;
        resolve_stack_entry_with(&mut game, &mut choices).unwrap();
        assert!(choices.pending);
        assert_eq!(game.calculated_power(attacker), Some(2));
        game = saved;
        choices.pause = false;
        choices.pending = false;
        choices.choosers.clear();
        settle(&mut game, &mut queue, &mut choices);
        assert_eq!(choices.choosers, vec![B]);
        assert_eq!(game.calculated_power(attacker), Some(4));
    }
}

#[test]
fn mirkwood_targeted_defensive_body_and_current_attack_recheck_both_remain_live() {
    for definition in definitions("Mirkwood Trapper") {
        let mut game = game(B);
        source(&mut game, &definition);
        let attacker = creature(&mut game, B);
        let mut queue = declare(&mut game, &[(attacker, AttackTarget::Player(A))]);
        assert_eq!(queue.entries.len(), 1);
        settle(&mut game, &mut queue, &mut Choices { target: Some(attacker), ..Default::default() });
        assert_eq!(game.calculated_power(attacker), Some(0));
    }
}

#[test]
fn scourge_rechecks_most_life_and_keeps_dethrone_untap_and_additional_combat() {
    for definition in definitions("Scourge of the Throne") {
        for lower_defender in [false, true] {
            let mut game = game(A);
            let dragon = source(&mut game, &definition);
            let mut queue = declare(&mut game, &[(dragon, AttackTarget::Player(B))]);
            assert_eq!(queue.entries.len(), 2, "dethrone and the first-attack ability");
            if lower_defender { game.player_mut(B).unwrap().life = 19; }
            settle(&mut game, &mut queue, &mut Choices::default());
            assert_eq!(game.turn_store.additional_phases, if lower_defender { vec![] } else { vec![Phase::Combat] });
            assert_eq!(game.is_tapped(dragon), lower_defender);
        }
    }
}

#[test]
fn failed_first_attack_condition_cannot_become_a_first_attack_in_a_later_combat() {
    for definition in definitions("Scourge of the Throne") {
        let mut game = game(A);
        let dragon = source(&mut game, &definition);
        game.player_mut(B).unwrap().life = 19;
        let queue = declare(&mut game, &[(dragon, AttackTarget::Player(B))]);
        assert!(queue.entries.is_empty());
        // End this attacking tenure without changing the creature's incarnation.
        game.set_current_controller(dragon, B).unwrap();
        game.set_current_controller(dragon, A).unwrap();
        game.remove_summoning_sickness(dragon);
        game.untap(dragon);
        game.combat = None;
        game.mark_combat_phase_started();
        game.player_mut(B).unwrap().life = 21;
        let mut queue = declare(&mut game, &[(dragon, AttackTarget::Player(B))]);
        assert_eq!(queue.entries.len(), 1, "only dethrone, even though the intervening condition is now true");
        settle(&mut game, &mut queue, &mut Choices::default());
        assert!(game.turn_store.additional_phases.is_empty());
    }
}

#[test]
fn unqualified_player_attack_groups_each_attacking_teammate_separately() {
    for definition in definitions("Norn's Decree") {
        let mut game = game(B);
        game.set_teams(vec![vec![B, C], vec![A, D]]).unwrap();
        game.enable_shared_team_turns().unwrap();
        source(&mut game, &definition);
        game.player_mut(D).unwrap().poison_counters = 1;
        let first = creature(&mut game, B);
        let second = creature(&mut game, B);
        let teammate = creature(&mut game, C);
        let mut queue = declare(&mut game, &[(first, AttackTarget::Player(D)),
            (second, AttackTarget::Player(D)), (teammate, AttackTarget::Player(D))]);
        assert_eq!(queue.entries.len(), 2);
        settle(&mut game, &mut queue, &mut Choices::default());
        assert_eq!(game.player(B).unwrap().hand.len(), 1);
        assert_eq!(game.player(C).unwrap().hand.len(), 1);
        assert!(game.player(A).unwrap().hand.is_empty());
    }
}
