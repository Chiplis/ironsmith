//! Frozen full bodies through independent direct/artifact compilation and native
//! legal casts. Source scenarios only; no hand-written payment label can replace
//! a real cast in the seven subject-card scenarios.
use ironsmith::ability::AbilityKind;
use ironsmith::alternative_cast::CastingMethod;
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::combat_state::{AttackTarget, CombatState, declare_attackers, declare_blockers};
use ironsmith::decision::{DecisionMaker, GameProgress, LegalAction, compute_legal_actions};
use ironsmith::decisions::context::{NumberContext, SelectObjectsContext, TargetsContext};
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, put_triggers_on_stack_with_dm, resolve_stack_entry_with};
use ironsmith::game_state::{StackEntry, Target};
use ironsmith::mana::ManaSymbol;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{CardDefinition, CardId, CardType, Effect, GameState, ObjectId, PlayerId, Zone};
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);

fn definitions(name: &str) -> Vec<CardDefinition> {
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!("../../../fixtures/alternative_paid_ability_references.json.fixture")).unwrap();
    let row = rows.iter().find(|row| row["name"] == name).unwrap();
    let mut text = format!("Mana cost: {}\nType: {}\n", row["mana_cost"].as_str().unwrap(), row["type_line"].as_str().unwrap());
    if let (Some(p), Some(t)) = (row["power"].as_str(), row["toughness"].as_str()) { text.push_str(&format!("Power/Toughness: {p}/{t}\n")); }
    text.push_str(row["oracle_text"].as_str().unwrap());
    let (direct, loss) = ironsmith_compiler::parse_loss::capture(|| ironsmith_registry::compile_builder_to_runtime_definition(
        ironsmith_compiler::CardDefinitionBuilder::new(CardId::new(), name), text.clone(), false));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let (artifact, loss) = ironsmith_compiler::parse_loss::capture(|| ironsmith_registry::compile_builder_to_artifact(
        ironsmith_compiler::CardDefinitionBuilder::new(CardId::new(), name), text, false));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let (artifact, _) = artifact.unwrap();
    artifact.validate().unwrap();
    let decoded = serde_json::from_slice(&serde_json::to_vec(&artifact).unwrap()).unwrap();
    vec![direct.unwrap(), ironsmith::artifact_materializer::materialize_artifact(&decoded).unwrap()]
}
#[derive(Default)]
struct Choices { target: Option<Target>, x: u32 }
impl DecisionMaker for Choices {
    fn answers_player_choices(&self) -> bool { false }
    fn decide_number(&mut self, _: &GameState, ctx: &NumberContext) -> u32 {
        if ctx.is_x_value { self.x.clamp(ctx.min, ctx.max) } else { ctx.min }
    }
    fn decide_targets(&mut self, game: &GameState, ctx: &TargetsContext) -> Vec<Target> {
        if let Some(target) = &self.target {
            return ctx.requirements.iter().filter(|r| r.legal_targets.contains(target)).map(|_| target.clone()).collect();
        }
        ironsmith::decision::SelectFirstDecisionMaker.decide_targets(game, ctx)
    }
    fn decide_objects(&mut self, _: &GameState, ctx: &SelectObjectsContext) -> Vec<ObjectId> {
        ctx.candidates.iter().filter(|c| c.legal).map(|c| c.id).take(ctx.max.unwrap_or(ctx.min)).collect()
    }
}
fn setup() -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.set_random_seed(31079);
    game.turn.turn_number = 7;
    main(&mut game, A);
    for s in [ManaSymbol::White, ManaSymbol::Blue, ManaSymbol::Black, ManaSymbol::Red, ManaSymbol::Green] {
        game.player_mut(A).unwrap().mana_pool.add(s, 20);
    }
    game
}
fn main(game: &mut GameState, active: PlayerId) {
    game.turn.active_player = active; game.turn.priority_player = Some(A);
    game.turn.phase = ironsmith::Phase::FirstMain; game.turn.step = None;
}
fn creature(game: &mut GameState, owner: PlayerId, subtype: ironsmith::Subtype, zone: Zone) -> ObjectId {
    let card = CardDefinitionBuilder::new(CardId::new(), "Scenario creature").card_types(vec![CardType::Creature])
        .subtypes(vec![subtype]).mana_cost(ironsmith::mana::ManaCost::from_symbols(vec![ManaSymbol::Generic(1)]))
        .power_toughness(ironsmith::card::PowerToughness::fixed(4, 4)).build();
    game.create_object_from_definition(&card, owner, zone)
}
fn library(game: &mut GameState, owner: PlayerId, count: usize) {
    for _ in 0..count { creature(game, owner, ironsmith::Subtype::Goblin, Zone::Library); }
}
fn action(game: &GameState, card: ObjectId, alternative: bool) -> Option<LegalAction> {
    compute_legal_actions(game, A).unwrap().into_iter().find(|a| match a {
        LegalAction::CastSpell { spell_id, casting_method, .. } if *spell_id == card => {
            if alternative { matches!(casting_method, CastingMethod::Alternative(0)) }
            else { matches!(casting_method, CastingMethod::Normal | CastingMethod::PlayFrom { use_alternative: None, .. }) }
        }
        _ => false,
    })
}
fn announce(game: &mut GameState, action: LegalAction, dm: &mut Choices) {
    let before = game.stack.len(); let mut q = TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut progress = apply_priority_response_with_dm(game, &mut q, &mut state, &PriorityResponse::PriorityAction(action), dm).unwrap();
    for _ in 0..40 {
        if state.pending_cast.is_none() && state.pending_activation.is_none() && game.stack.len() > before { return; }
        let GameProgress::NeedsDecisionCtx(ctx) = progress else { panic!("{progress:?}") };
        progress = apply_decision_context_with_dm(game, &mut q, &mut state, &ctx, dm).unwrap();
    }
    panic!("native announcement did not complete");
}
fn cast(game: &mut GameState, card: ObjectId, alternative: bool, cost: u32, dm: &mut Choices) -> ObjectId {
    let stable = game.object(card).unwrap().stable_id; let before = game.player(A).unwrap().mana_pool.total();
    let choice = action(game, card, alternative).expect("required cast must be advertised by native legal actions");
    announce(game, choice, dm);
    assert_eq!(before - game.player(A).unwrap().mana_pool.total(), cost);
    let spell = game.find_object_by_stable_id(stable).unwrap();
    assert_eq!(game.object(spell).unwrap().zone, Zone::Stack);
    assert_eq!(game.object(spell).unwrap().optional_costs_paid.cast_payment_turn, Some(game.turn.turn_number));
    spell
}
fn stack_triggers(game: &mut GameState, dm: &mut Choices) {
    put_triggers_on_stack_with_dm(game, &mut TriggerQueue::new(), dm).unwrap();
}
fn finish(game: &mut GameState, dm: &mut Choices) {
    for _ in 0..30 {
        stack_triggers(game, dm);
        if game.stack.is_empty() { return; }
        resolve_stack_entry_with(game, dm).unwrap();
    }
    panic!("unresolved native stack");
}
fn combat(game: &mut GameState, attacker: ObjectId, target: AttackTarget) {
    game.turn.phase = ironsmith::Phase::Combat; game.turn.step = Some(ironsmith::Step::DeclareAttackers);
    game.remove_summoning_sickness(attacker); game.untap(attacker);
    let mut state = CombatState::default();
    declare_attackers(game, &mut state, vec![(attacker, target)]).unwrap();
    game.turn.step = Some(ironsmith::Step::DeclareBlockers);
    declare_blockers(game, &mut state, vec![]).unwrap(); game.combat = Some(state);
}
fn damage(game: &mut GameState) {
    game.turn.step = Some(ironsmith::Step::CombatDamage);
    let state = game.combat.clone().unwrap();
    ironsmith::game_loop::try_execute_combat_damage_step(game, &state, false).unwrap();
}
fn qualify(game: &mut GameState, subtype: ironsmith::Subtype, commander: bool) -> ObjectId {
    let source = creature(game, A, subtype, Zone::Battlefield); if commander { game.set_commander(source); }
    combat(game, source, AttackTarget::Player(B)); damage(game); main(game, A); source
}
fn discard(game: &mut GameState, card: ObjectId, dm: &mut Choices) -> ObjectId {
    let stable = game.object(card).unwrap().stable_id;
    game.push_to_stack(StackEntry::ability(card, A, vec![Effect::discard(1)]));
    resolve_stack_entry_with(game, dm).unwrap();
    let grave = game.find_object_by_stable_id(stable).unwrap(); assert_eq!(game.object(grave).unwrap().zone, Zone::Graveyard); grave
}
fn zone(game: &GameState, stable: ironsmith::ids::StableId) -> Zone {
    game.object(game.find_object_by_stable_id(stable).unwrap()).unwrap().zone
}

#[test]
fn frozen_full_definitions_preserve_printed_bodies_without_loss_on_both_routes() {
    for (name, triggered, activated) in [
        ("Earwig Squad", 1, 0), ("Latchkey Faerie", 1, 0), ("Leonardo, Leader in Blue", 1, 1),
        ("Monastery Raid", 0, 0), ("Sandman's Quicksand", 0, 0), ("Turncoat Kunoichi", 1, 0), ("Karai, Future of the Foot", 1, 0),
    ] {
        for definition in definitions(name) {
            assert_eq!(definition.alternative_casts.len(), 1);
            assert_eq!(definition.abilities.iter().filter(|a| matches!(&a.kind, AbilityKind::Triggered(_))).count(), triggered);
            assert_eq!(definition.abilities.iter().filter(|a| matches!(&a.kind, AbilityKind::Activated(_))).count(), activated);
            let rendered = ironsmith::compiled_text::debug_compiled_lines(&definition).join("\n");
            assert!(!rendered.to_ascii_lowercase().contains("unsupported"), "{name}: {rendered}");
            if name == "Karai, Future of the Foot" { assert!(rendered.contains("paid this turn"), "{rendered}"); }
            if name == "Latchkey Faerie" { assert!(rendered.contains("Flying"), "{rendered}"); }
        }
    }
}

#[test]
fn prowl_all_shared_types_real_payment_full_bodies_and_source_departure() {
    for (name, subtype, normal) in [("Earwig Squad", ironsmith::Subtype::Goblin, 5), ("Latchkey Faerie", ironsmith::Subtype::Faerie, 4)] {
        for definition in definitions(name) {
            for kind in [subtype, ironsmith::Subtype::Rogue] {
                for paid in [false, true] {
                    for depart in [false, true] {
                        let mut game = setup(); library(&mut game, A, 6); library(&mut game, B, 6);
                        let card = game.create_object_from_definition(&definition, A, Zone::Hand);
                        assert!(action(&game, card, true).is_none());
                        let dealer = qualify(&mut game, kind, false); game.move_object_by_effect(dealer, Zone::Graveyard).unwrap();
                        assert!(action(&game, card, true).is_some(), "event-time dealer LKI survives departure");
                        let stable = game.object(card).unwrap().stable_id;
                        let mut dm = Choices { target: Some(Target::Player(B)), ..Default::default() };
                        cast(&mut game, card, paid, if paid { 3 } else { normal }, &mut dm);
                        resolve_stack_entry_with(&mut game, &mut dm).unwrap();
                        let source = game.find_object_by_stable_id(stable).unwrap();
                        if depart { game.move_object_by_effect(source, Zone::Graveyard).unwrap(); }
                        finish(&mut game, &mut dm);
                        if name == "Earwig Squad" {
                            assert_eq!(game.exile.len(), if paid { 3 } else { 0 });
                            assert_eq!(game.player(B).unwrap().library.len(), if paid { 3 } else { 6 });
                            assert_eq!(game.player(A).unwrap().library.len(), 6);
                            let shuffles: Vec<_> = game.turn_store.turn_history.event_records.iter()
                                .filter_map(|r| r.event.downcast::<ironsmith::events::ShuffleLibraryEvent>()).map(|e| e.player).collect();
                            assert_eq!(shuffles, if paid { vec![B] } else { vec![] });
                        } else {
                            assert_eq!(game.player(A).unwrap().hand.len(), usize::from(paid));
                            assert_eq!(game.player(A).unwrap().library.len(), if paid { 5 } else { 6 });
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn combat_qualification_has_complete_negatives_and_explicit_missing_lki() {
    for name in ["Earwig Squad", "Latchkey Faerie", "Monastery Raid"] {
        for definition in definitions(name) {
            let subtype = if name == "Monastery Raid" { ironsmith::Subtype::Assassin } else { ironsmith::Subtype::Rogue };
            for case in ["empty", "wrong_type", "opponent", "noncombat", "blocked", "old", "unknown"] {
                let mut game = setup(); let card = game.create_object_from_definition(&definition, A, Zone::Hand);
                if case == "wrong_type" { qualify(&mut game, ironsmith::Subtype::Bear, false); }
                if case == "opponent" {
                    let source = creature(&mut game, B, subtype, Zone::Battlefield); game.turn.active_player = B;
                    combat(&mut game, source, AttackTarget::Player(A)); damage(&mut game); main(&mut game, A);
                }
                if case == "noncombat" {
                    let source = creature(&mut game, A, subtype, Zone::Battlefield);
                    game.push_to_stack(StackEntry::ability(source, A, vec![Effect::deal_damage(1,
                        ironsmith::target::ChooseSpec::Player(ironsmith::target::PlayerFilter::Specific(B)))]));
                    resolve_stack_entry_with(&mut game, &mut Choices::default()).unwrap();
                }
                if case == "blocked" {
                    let source = creature(&mut game, A, subtype, Zone::Battlefield);
                    let blocker = creature(&mut game, B, ironsmith::Subtype::Bear, Zone::Battlefield);
                    combat(&mut game, source, AttackTarget::Player(B));
                    let mut state = game.combat.take().unwrap(); declare_blockers(&game, &mut state, vec![(blocker, source)]).unwrap(); game.combat = Some(state);
                    damage(&mut game); main(&mut game, A);
                }
                if case == "old" { qualify(&mut game, subtype, false); game.next_turn(); main(&mut game, A); }
                if case == "unknown" {
                    qualify(&mut game, subtype, false);
                    let records: Vec<_> = game.turn_store.turn_history.event_records.iter().cloned().collect();
                    game.turn_store.turn_history.event_records.clear();
                    for mut record in records {
                        if record.event.downcast::<ironsmith::events::DamageEvent>().is_some() { record.source_snapshot = None; record.object_snapshot = None; }
                        game.turn_store.turn_history.event_records.push(record);
                    }
                    assert!(compute_legal_actions(&game, A).is_err(), "{name}: missing LKI must latch incomplete evidence");
                    qualify(&mut game, subtype, false);
                    assert!(action(&game, card, true).is_some(), "one fully witnessed event establishes existential eligibility");
                } else { assert!(action(&game, card, true).is_none(), "{name}: {case}"); }
            }
        }
    }
}

#[test]
fn sneak_has_a_real_return_cost_window_and_exact_attack_destination() {
    for name in ["Leonardo, Leader in Blue", "Turncoat Kunoichi", "Karai, Future of the Foot"] {
        for definition in definitions(name) {
            for destination in ["player", "walker", "departed_walker"] {
                let mut game = setup();
                let walker_def = CardDefinitionBuilder::new(CardId::new(), "Defending walker").card_types(vec![CardType::Planeswalker]).loyalty(5).build();
                let walker = game.create_object_from_definition(&walker_def, B, Zone::Battlefield);
                let target = if destination == "player" { AttackTarget::Player(B) } else { AttackTarget::Planeswalker(walker) };
                let attacker = creature(&mut game, A, ironsmith::Subtype::Goblin, Zone::Battlefield);
                let attacker_stable = game.object(attacker).unwrap().stable_id;
                combat(&mut game, attacker, target.clone());
                let card = game.create_object_from_definition(&definition, A, Zone::Hand);
                let stable = game.object(card).unwrap().stable_id;
                assert!(action(&game, card, true).is_some());
                for case in ["undeclared", "blocked", "wrong_controller", "damage_step", "end_combat", "no_attacker", "no_mana"] {
                    let mut wrong = game.clone();
                    match case {
                        "undeclared" => wrong.combat.as_mut().unwrap().block_declaration_complete = false,
                        "blocked" => { wrong.combat.as_mut().unwrap().blocked_attackers.insert(attacker); }
                        "wrong_controller" => {
                            wrong.set_current_controller(attacker, B).unwrap();
                            assert_eq!(wrong.current_controller(attacker), Some(B));
                        }
                        "damage_step" => wrong.turn.step = Some(ironsmith::Step::CombatDamage),
                        "end_combat" => wrong.turn.step = Some(ironsmith::Step::EndCombat),
                        "no_attacker" => wrong.combat.as_mut().unwrap().attackers.clear(),
                        "no_mana" => wrong.player_mut(A).unwrap().mana_pool = Default::default(),
                        _ => unreachable!(),
                    }
                    assert!(action(&wrong, card, true).is_none(), "{name}: {case}");
                }
                let mut dm = Choices::default();
                cast(&mut game, card, true, if name.starts_with("Leonardo") { 5 } else { 4 }, &mut dm);
                assert_eq!(zone(&game, attacker_stable), Zone::Hand);
                if destination == "departed_walker" { game.move_object_by_effect(walker, Zone::Graveyard).unwrap(); }
                resolve_stack_entry_with(&mut game, &mut dm).unwrap();
                let source = game.find_object_by_stable_id(stable).unwrap();
                assert!(game.is_tapped(source));
                assert_eq!(ironsmith::combat_state::get_attack_target(game.combat.as_ref().unwrap(), source),
                    if destination == "departed_walker" { None } else { Some(&target) });
            }
        }
    }
}

#[test]
fn leonardo_paid_entry_and_independent_first_strike_activation_expire() {
    for definition in definitions("Leonardo, Leader in Blue") {
        for paid in [false, true] {
            let mut game = setup();
            let friend = creature(&mut game, A, ironsmith::Subtype::Bear, Zone::Battlefield);
            let opponent = creature(&mut game, B, ironsmith::Subtype::Bear, Zone::Battlefield);
            let attacker = creature(&mut game, A, ironsmith::Subtype::Goblin, Zone::Battlefield);
            let card = game.create_object_from_definition(&definition, A, Zone::Hand); let stable = game.object(card).unwrap().stable_id;
            if paid { combat(&mut game, attacker, AttackTarget::Player(B)); }
            let mut dm = Choices::default(); cast(&mut game, card, paid, if paid { 5 } else { 1 }, &mut dm); finish(&mut game, &mut dm);
            let source = game.find_object_by_stable_id(stable).unwrap();
            assert_eq!(game.calculated_power(source), Some(if paid { 4 } else { 2 }));
            assert_eq!(game.calculated_power(friend), Some(if paid { 6 } else { 4 }));
            assert_eq!(game.calculated_power(opponent), Some(4)); assert_eq!(game.calculated_toughness(source), Some(1));
            let activate = compute_legal_actions(&game, A).unwrap().into_iter().find(|a| matches!(a, LegalAction::ActivateAbility { source: id, .. } if *id == source)).unwrap();
            let mana = game.player(A).unwrap().mana_pool.total(); announce(&mut game, activate, &mut dm);
            assert_eq!(mana - game.player(A).unwrap().mana_pool.total(), 2); finish(&mut game, &mut dm);
            assert!(game.current_has_static_ability_id(source, ironsmith::static_abilities::StaticAbilityId::FirstStrike));
            game.next_turn();
            assert_eq!(game.calculated_power(source), Some(2)); assert_eq!(game.calculated_power(friend), Some(4));
            assert!(!game.current_has_static_ability_id(source, ironsmith::static_abilities::StaticAbilityId::FirstStrike));
        }
    }
}

#[test]
fn turncoat_exiles_one_chosen_opposing_creature_with_exclusive_paid_duration() {
    for definition in definitions("Turncoat Kunoichi") {
        for paid in [false, true] {
            for early_departure in [false, true] {
                let mut game = setup(); let target = creature(&mut game, B, ironsmith::Subtype::Bear, Zone::Battlefield);
                let target_stable = game.object(target).unwrap().stable_id;
                let attacker = creature(&mut game, A, ironsmith::Subtype::Goblin, Zone::Battlefield);
                let card = game.create_object_from_definition(&definition, A, Zone::Hand); let stable = game.object(card).unwrap().stable_id;
                if paid { combat(&mut game, attacker, AttackTarget::Player(B)); }
                let mut dm = Choices { target: Some(Target::Object(target)), ..Default::default() };
                cast(&mut game, card, paid, if paid { 4 } else { 3 }, &mut dm); resolve_stack_entry_with(&mut game, &mut dm).unwrap();
                let source = game.find_object_by_stable_id(stable).unwrap();
                if early_departure { game.move_object_by_effect(source, Zone::Hand).unwrap(); }
                finish(&mut game, &mut dm);
                assert_eq!(zone(&game, target_stable), if early_departure && !paid { Zone::Battlefield } else { Zone::Exile });
                if !early_departure {
                    game.move_object_by_effect(source, Zone::Hand).unwrap(); finish(&mut game, &mut dm);
                    assert_eq!(zone(&game, target_stable), if paid { Zone::Exile } else { Zone::Battlefield });
                }
                let new_source = game.find_object_by_stable_id(stable).unwrap();
                assert!(!game.object(new_source).unwrap().optional_costs_paid.any_paid());
                assert_eq!(game.object(new_source).unwrap().optional_costs_paid.cast_payment_turn, None);
            }
        }
    }
}

#[test]
fn karai_damage_trigger_distinguishes_paid_this_turn_from_a_prior_cast_and_survives_departure() {
    for definition in definitions("Karai, Future of the Foot") {
        for paid in [false, true] {
            for later in [false, true] {
                for depart in [false, true] {
                    let mut game = setup(); let target = creature(&mut game, A, ironsmith::Subtype::Bear, Zone::Graveyard);
                    let target_stable = game.object(target).unwrap().stable_id;
                    let attacker = creature(&mut game, A, ironsmith::Subtype::Goblin, Zone::Battlefield);
                    let card = game.create_object_from_definition(&definition, A, Zone::Hand); let stable = game.object(card).unwrap().stable_id;
                    if paid { combat(&mut game, attacker, AttackTarget::Player(B)); }
                    let mut dm = Choices { target: Some(Target::Object(target)), ..Default::default() };
                    cast(&mut game, card, paid, if paid { 4 } else { 3 }, &mut dm); finish(&mut game, &mut dm);
                    let source = game.find_object_by_stable_id(stable).unwrap();
                    if later { game.next_turn(); main(&mut game, A); }
                    if later || !paid { combat(&mut game, source, AttackTarget::Player(B)); }
                    damage(&mut game); if depart { game.move_object_by_effect(source, Zone::Hand).unwrap(); }
                    finish(&mut game, &mut dm);
                    assert_eq!(zone(&game, target_stable), if paid && !later { Zone::Battlefield } else { Zone::Hand });
                    assert_eq!(game.player(B).unwrap().life, 17);
                }
            }
        }
    }
}

#[test]
fn monastery_replaces_two_with_paid_x_and_retains_exact_exiled_collection_permission() {
    for definition in definitions("Monastery Raid") {
        for paid in [false, true] {
            for x in [0, 1, 4] {
                for commander in [false, true] {
                    let mut game = setup(); library(&mut game, A, 8);
                    let card = game.create_object_from_definition(&definition, A, Zone::Hand);
                    qualify(&mut game, if commander { ironsmith::Subtype::Bear } else { ironsmith::Subtype::Assassin }, commander);
                    let mut dm = Choices { x, ..Default::default() };
                    cast(&mut game, card, paid, if paid { x + 1 } else { 3 }, &mut dm); finish(&mut game, &mut dm);
                    let count = if paid { x as usize } else { 2 };
                    assert_eq!(game.exile.len(), count); assert_eq!(game.player(A).unwrap().library.len(), 8 - count);
                    let cards = game.exile.clone();
                    for id in &cards { assert!(action(&game, *id, false).is_some()); }
                    game.next_turn(); main(&mut game, B);
                    for id in &cards { assert!(action(&game, *id, false).is_none(), "no flash"); }
                    game.next_turn(); main(&mut game, A);
                    for id in &cards { assert!(action(&game, *id, false).is_some(), "permission lasts through own next turn"); }
                    game.next_turn(); main(&mut game, A);
                    for id in &cards { assert!(action(&game, *id, false).is_none(), "permission expired"); }
                }
            }
        }
    }
}

#[test]
fn mayhem_requires_exact_discard_origin_time_and_real_cost_and_changes_the_affected_set() {
    for definition in definitions("Sandman's Quicksand") {
        for paid in [false, true] {
            let mut game = setup(); let friend = creature(&mut game, A, ironsmith::Subtype::Bear, Zone::Battlefield);
            let opponent = creature(&mut game, B, ironsmith::Subtype::Bear, Zone::Battlefield);
            let card = game.create_object_from_definition(&definition, A, Zone::Hand); let mut dm = Choices::default();
            let card = if paid { discard(&mut game, card, &mut dm) } else { card };
            if paid {
                assert!(action(&game, card, true).is_some());
                let other = game.create_object_from_definition(&definition, A, Zone::Graveyard);
                assert!(action(&game, other, true).is_none(), "different copy was not discarded");
                for case in ["hand", "new_incarnation", "later", "combat", "opponent", "no_mana"] {
                    let mut wrong = game.clone();
                    let candidate = match case {
                        "hand" => wrong.move_object_by_effect(card, Zone::Hand).unwrap(),
                        "new_incarnation" => { let exiled = wrong.move_object_by_effect(card, Zone::Exile).unwrap(); wrong.move_object_by_effect(exiled, Zone::Graveyard).unwrap() }
                        _ => card,
                    };
                    match case {
                        "later" => { wrong.next_turn(); main(&mut wrong, A); }
                        "combat" => wrong.turn.phase = ironsmith::Phase::Combat,
                        "opponent" => main(&mut wrong, B),
                        "no_mana" => wrong.player_mut(A).unwrap().mana_pool = Default::default(),
                        _ => (),
                    }
                    assert!(action(&wrong, candidate, true).is_none(), "{case}");
                }
            } else {
                let mut mill = game.clone(); let grave = mill.move_object_by_effect(card, Zone::Graveyard).unwrap();
                assert!(action(&mill, grave, true).is_none(), "mill is not discard");
            }
            cast(&mut game, card, paid, if paid { 4 } else { 3 }, &mut dm); finish(&mut game, &mut dm);
            assert_eq!(game.calculated_power(friend), Some(if paid { 4 } else { 2 }));
            assert_eq!(game.calculated_toughness(friend), Some(if paid { 4 } else { 2 }));
            assert_eq!(game.calculated_power(opponent), Some(2)); assert_eq!(game.calculated_toughness(opponent), Some(2));
            game.next_turn(); assert_eq!(game.calculated_power(friend), Some(4)); assert_eq!(game.calculated_toughness(opponent), Some(4));
        }
    }
}

#[test]
fn copied_spells_retain_alternative_payment_and_x_without_another_cast() {
    use ironsmith::effects::{EffectExecutor, EffectContext as ExecutionContext, ResolvedTarget};
    for name in ["Monastery Raid", "Sandman's Quicksand"] {
        for definition in definitions(name) {
            for paid in [false, true] {
                let mut game = setup(); library(&mut game, A, 10);
                let friend = creature(&mut game, A, ironsmith::Subtype::Bear, Zone::Battlefield);
                let opponent = creature(&mut game, B, ironsmith::Subtype::Bear, Zone::Battlefield);
                let card = game.create_object_from_definition(&definition, A, Zone::Hand);
                let mut dm = Choices { x: 3, ..Default::default() };
                let card = if name == "Sandman's Quicksand" && paid { discard(&mut game, card, &mut dm) } else { card };
                if name == "Monastery Raid" { qualify(&mut game, ironsmith::Subtype::Assassin, false); }
                let spell = cast(&mut game, card, paid, if paid { 4 } else { 3 }, &mut dm);
                let receipt = game.stack.last().unwrap().optional_costs_paid.clone();
                let casts = game.turn_store.turn_history.spells_cast_by_player(A);
                let mut ctx = ExecutionContext::new(spell, A, &mut dm); ctx.targets = vec![ResolvedTarget::Object(spell)];
                ironsmith::effects::CopySpellEffect::new(ironsmith::target::ChooseSpec::spell(), 1).execute(&mut game, &mut ctx).unwrap();
                assert_eq!(game.stack.len(), 2); assert_eq!(game.stack.last().unwrap().optional_costs_paid, receipt);
                assert_eq!(game.turn_store.turn_history.spells_cast_by_player(A), casts);
                assert_eq!(game.stack.last().unwrap().x_value, if name == "Monastery Raid" && paid { Some(3) } else { None });
                finish(&mut game, &mut dm);
                if name == "Monastery Raid" { assert_eq!(game.exile.len(), if paid { 6 } else { 4 }); }
                else {
                    assert_eq!(game.calculated_toughness(friend), Some(if paid { 4 } else { 0 }));
                    assert_eq!(game.calculated_toughness(opponent), Some(0));
                }
            }
        }
    }
}

#[test]
fn receipt_transport_survives_lki_but_permanent_copy_and_new_incarnation_start_unpaid() {
    for definition in definitions("Karai, Future of the Foot") {
        let mut game = setup(); let attacker = creature(&mut game, A, ironsmith::Subtype::Goblin, Zone::Battlefield);
        combat(&mut game, attacker, AttackTarget::Player(B));
        let card = game.create_object_from_definition(&definition, A, Zone::Hand); let stable = game.object(card).unwrap().stable_id;
        let mut dm = Choices::default(); cast(&mut game, card, true, 4, &mut dm);
        let receipt = game.stack.last().unwrap().optional_costs_paid.clone();
        let restored: ironsmith_core::OptionalCostsPaid = serde_json::from_slice(&serde_json::to_vec(&receipt).unwrap()).unwrap();
        assert_eq!(receipt, restored); finish(&mut game, &mut dm);
        let source = game.find_object_by_stable_id(stable).unwrap();
        let snapshot = ironsmith::snapshot::ObjectSnapshot::from_object(game.object(source).unwrap(), &game);
        let snapshot: ironsmith::snapshot::ObjectSnapshot = serde_json::from_slice(&serde_json::to_vec(&snapshot).unwrap()).unwrap();
        assert_eq!(snapshot.optional_costs_paid, receipt);
        let copy_id = game.new_object_id();
        let copy = ironsmith::object::Object::token_copy_of(game.object(source).unwrap(), copy_id, B);
        assert!(!copy.optional_costs_paid.any_paid()); assert_eq!(copy.optional_costs_paid.cast_payment_turn, None);
        game.add_object(copy);
        let corpse = creature(&mut game, B, ironsmith::Subtype::Bear, Zone::Graveyard); let corpse_stable = game.object(corpse).unwrap().stable_id;
        dm.target = Some(Target::Object(corpse)); game.turn.active_player = B;
        combat(&mut game, copy_id, AttackTarget::Player(A)); damage(&mut game); finish(&mut game, &mut dm);
        assert_eq!(zone(&game, corpse_stable), Zone::Hand, "permanent copy did not pay the original's Sneak cost");
        let moved = game.move_object_by_effect(source, Zone::Hand).unwrap();
        assert!(!game.object(moved).unwrap().optional_costs_paid.any_paid());
        assert_eq!(game.object(moved).unwrap().optional_costs_paid.cast_payment_turn, None);
    }
}

#[test]
fn undated_legacy_paid_receipts_error_under_positive_and_negated_resolution_gates() {
    use ironsmith::effect::{Condition, Value};
    use ironsmith::effects::{EffectContext as ExecutionContext, ExecutionError};
    use ironsmith_core::{AlternativeCostReference, OptionalCostKind, OptionalCostRef, OptionalCostsPaid};
    let mut game = setup(); let source = creature(&mut game, A, ironsmith::Subtype::Ninja, Zone::Battlefield);
    let reference = OptionalCostRef::new(OptionalCostKind::AlternativeCast(AlternativeCostReference::by_name("Sneak", None)));
    let mut paid = OptionalCostsPaid::default(); paid.mark_label_paid(reference.clone());
    let mut wire = serde_json::to_value(&paid).unwrap(); wire.as_object_mut().unwrap().remove("cast_payment_turn");
    let restored: OptionalCostsPaid = serde_json::from_value(wire).unwrap(); assert_eq!(restored.cast_payment_turn, None);
    game.object_mut(source).unwrap().optional_costs_paid = restored.clone();
    let query = reference.this_turn(); let mut dm = Choices::default(); let mut ctx = ExecutionContext::new(source, A, &mut dm); ctx.optional_costs_paid = restored;
    for condition in [Condition::ThisSpellPaidLabel(query.clone()), Condition::Not(Box::new(Condition::ThisSpellPaidLabel(query.clone())))] {
        assert!(matches!(ironsmith::condition_eval::evaluate_condition_resolution(&game, &condition, &ctx), Err(ExecutionError::IncompleteEvidence(_))));
        assert!(matches!(ironsmith::condition_eval::evaluate_condition_cast_time_checked(&game, &condition, A, source), Err(ExecutionError::IncompleteEvidence(_))));
    }
    assert!(matches!(ironsmith::effects::helpers::resolve_value(&game, &Value::WasPaidLabel(query.clone()), &ctx), Err(ExecutionError::IncompleteEvidence(_))));
    ctx.optional_costs_paid.record_completed_cast_payment(game.turn.turn_number);
    assert!(ironsmith::condition_eval::evaluate_condition_resolution(&game, &Condition::ThisSpellPaidLabel(query.clone()), &ctx).unwrap());
    game.next_turn(); assert!(!ironsmith::condition_eval::evaluate_condition_resolution(&game, &Condition::ThisSpellPaidLabel(query), &ctx).unwrap());
}

fn temporal_header(negated: bool) -> CardDefinition {
    let tail = if negated { "wasn't" } else { "was" };
    let text = format!("Mana cost: {{1}}\nType: Creature — Ninja\nPower/Toughness: 2/2\nSneak {{0}} (You may cast this spell for {{0}} if you also return an unblocked attacker you control to hand during the declare blockers step. It enters tapped and attacking.)\nWhen this creature enters, if its sneak cost {tail} paid this turn, draw a card.");
    let (result, loss) = ironsmith_compiler::parse_loss::capture(|| ironsmith_registry::compile_builder_to_runtime_definition(
        ironsmith_compiler::CardDefinitionBuilder::new(CardId::new(), "Temporal entry source"), text, false));
    assert!(!loss.is_lossy(), "{}", loss.reasons_text()); result.unwrap()
}
#[test]
fn native_temporal_intervening_gate_admission_and_resolution_preserve_incomplete_evidence() {
    for negated in [false, true] {
        let definition = temporal_header(negated);
        for paid in [false, true] {
            let mut game = setup(); library(&mut game, A, 4);
            let attacker = creature(&mut game, A, ironsmith::Subtype::Goblin, Zone::Battlefield);
            let card = game.create_object_from_definition(&definition, A, Zone::Hand);
            if paid { combat(&mut game, attacker, AttackTarget::Player(B)); }
            let mut dm = Choices::default(); let spell = cast(&mut game, card, paid, if paid { 0 } else { 1 }, &mut dm);
            if paid {
                let mut unknown = game.clone();
                unknown.stack.last_mut().unwrap().optional_costs_paid.cast_payment_turn = None;
                unknown.object_mut(spell).unwrap().optional_costs_paid.cast_payment_turn = None;
                assert!(resolve_stack_entry_with(&mut unknown, &mut dm).is_err(), "unknown admission negated={negated}");
                assert_eq!(unknown.stack.len(), 1); assert_eq!(unknown.object(spell).unwrap().zone, Zone::Stack);
                assert_eq!(unknown.player(A).unwrap().library.len(), 4);
            }
            resolve_stack_entry_with(&mut game, &mut dm).unwrap(); stack_triggers(&mut game, &mut dm);
            let admitted = paid != negated; assert_eq!(game.stack.len(), usize::from(admitted));
            if paid && !negated {
                let mut unknown = game.clone(); unknown.stack.last_mut().unwrap().optional_costs_paid.cast_payment_turn = None;
                assert!(resolve_stack_entry_with(&mut unknown, &mut dm).is_err());
                assert_eq!(unknown.stack.len(), 1); assert_eq!(unknown.player(A).unwrap().library.len(), 4);
                let mut later = game.clone(); later.next_turn(); resolve_stack_entry_with(&mut later, &mut dm).unwrap();
                assert_eq!(later.player(A).unwrap().library.len(), 4);
            }
            finish(&mut game, &mut dm); assert_eq!(game.player(A).unwrap().library.len(), if admitted { 3 } else { 4 });
        }
    }
}
