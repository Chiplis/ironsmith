//! Eight complete frozen bodies; authored source regressions, execution deferred.
use ironsmith::ability::AbilityKind;
use ironsmith::card::{CardBuilder, PowerToughness};
use ironsmith::cards::CardDefinition;
use ironsmith::decision::{DecisionMaker, LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
use ironsmith::decisions::context::{SelectOptionsContext, TargetsContext};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, put_triggers_on_stack_with_dm, resolve_stack_entry_with,
};
use ironsmith::game_state::StackEntry;
use ironsmith::mana::ManaSymbol;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{CardId, CardType, CounterType, GameProgress, GameState, ObjectId, Phase, PlayerId, Subtype, Supertype, Target, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;

const A: PlayerId = PlayerId::from_index(0);
const B: PlayerId = PlayerId::from_index(1);
const C: PlayerId = PlayerId::from_index(2);

fn rows() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!("../../../fixtures/extra_turn_bodies.json.fixture")).unwrap()
}

fn definitions(name: &str) -> [CardDefinition; 2] {
    let row = rows().into_iter().find(|row| row["name"] == name).unwrap();
    let mut text = format!("Mana cost: {}\nType: {}\n", row["mana_cost"].as_str().unwrap(), row["type_line"].as_str().unwrap());
    if let (Some(power), Some(toughness)) = (row["power"].as_str(), row["toughness"].as_str()) {
        text.push_str(&format!("Power/Toughness: {power}/{toughness}\n"));
    }
    if let Some(loyalty) = row["loyalty"].as_str() {
        text.push_str(&format!("Loyalty: {loyalty}\n"));
    }
    text.push_str(row["oracle_text"].as_str().unwrap());
    let (result, loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_artifact(name, &text, false));
    let (artifact, _) = result.unwrap_or_else(|error| panic!("{name}: {error}"));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    artifact.validate().unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    restored.validate().unwrap();
    assert_eq!(artifact, restored);
    let (result, loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_runtime_definition(name, &text, false));
    let direct = result.unwrap_or_else(|error| panic!("direct {name}: {error}"));
    assert!(!loss.is_lossy(), "direct {name}: {}", loss.reasons_text());
    [direct, materialize_artifact(&restored).unwrap()]
}

fn game() -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Carol".into()], 20);
    game.turn.active_player = A;
    game.turn.priority_player = Some(A);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    for player in [A, B, C] {
        game.player_mut(player).unwrap().mana_pool.add(ManaSymbol::Blue, 30);
        game.player_mut(player).unwrap().mana_pool.add(ManaSymbol::Colorless, 30);
    }
    game
}

fn object(game: &mut GameState, owner: PlayerId, zone: Zone, name: &str, types: Vec<CardType>, subtypes: Vec<Subtype>, legendary: bool) -> ObjectId {
    let card = CardBuilder::new(CardId::new(), name)
        .card_types(types).subtypes(subtypes)
        .supertypes(if legendary { vec![Supertype::Legendary] } else { vec![] })
        .power_toughness(PowerToughness::fixed(2, 2)).build();
    game.create_object_from_card(&card, owner, zone)
}
fn creature(game: &mut GameState, player: PlayerId, name: &str) -> ObjectId {
    object(game, player, Zone::Battlefield, name, vec![CardType::Creature], vec![], false)
}
fn library(game: &mut GameState, player: PlayerId, count: usize) {
    for n in 0..count {
        object(game, player, Zone::Library, &format!("Draw witness {n}"), vec![CardType::Instant], vec![], false);
    }
}
fn named(game: &GameState, player: PlayerId, zone: Zone, name: &str) -> bool {
    let ids = match zone {
        Zone::Hand => &game.player(player).unwrap().hand,
        Zone::Library => &game.player(player).unwrap().library,
        Zone::Graveyard => &game.player(player).unwrap().graveyard,
        Zone::Exile => &game.exile,
        _ => panic!("unexpected zone"),
    };
    ids.iter().filter_map(|id| game.object(*id)).any(|object| object.owner == player && object.name.as_ref() == name)
}

#[derive(Default)]
struct Choices {
    targets: Vec<Target>,
    forbidden_targets: Vec<Target>,
    target_groups: Vec<(usize, Option<usize>)>,
    pay_buyback: bool,
    buyback_available: Option<bool>,
}
impl DecisionMaker for Choices {
    fn decide_targets(&mut self, _: &GameState, context: &TargetsContext) -> Vec<Target> {
        self.target_groups = context.requirements.iter().map(|r| (r.min_targets, r.max_targets)).collect();
        for target in &self.forbidden_targets {
            assert!(context.requirements.iter().all(|r| !r.legal_targets.contains(target)), "forbidden target offered: {target:?}");
        }
        for target in &self.targets {
            assert!(context.requirements.iter().any(|r| r.legal_targets.contains(target)), "{target:?}: {context:?}");
        }
        self.targets.clone()
    }
    fn decide_options(&mut self, game: &GameState, context: &SelectOptionsContext) -> Vec<usize> {
        if context.description.starts_with("Choose optional costs") {
            let buyback = context.options.iter().find(|option| option.description.starts_with("Buyback:"));
            self.buyback_available = Some(buyback.is_some_and(|option| option.legal));
            return if self.pay_buyback { vec![buyback.filter(|option| option.legal).expect("three Islands pay buyback").index] } else { vec![] };
        }
        SelectFirstDecisionMaker.decide_options(game, context)
    }
}
fn announce(game: &mut GameState, action: LegalAction, dm: &mut Choices) {
    let mut state = PriorityLoopState::new(game.players.len());
    let mut queue = TriggerQueue::new();
    let mut progress = apply_priority_response_with_dm(game, &mut queue, &mut state, &PriorityResponse::PriorityAction(action), dm).unwrap();
    for _ in 0..64 {
        if state.pending_cast.is_none() && state.pending_activation.is_none() { return; }
        let GameProgress::NeedsDecisionCtx(context) = progress else { panic!("{progress:?}"); };
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &context, dm).unwrap();
    }
    panic!("announcement did not complete");
}
fn cast_action(game: &GameState, id: ObjectId) -> Option<LegalAction> {
    compute_legal_actions(game, A).unwrap().into_iter().find(|action| matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == id))
}
fn cast(game: &mut GameState, definition: &CardDefinition, dm: &mut Choices) {
    game.turn.priority_player = Some(A);
    let id = game.create_object_from_definition(definition, A, Zone::Hand);
    let action = cast_action(game, id).expect("exact spell is castable");
    announce(game, action, dm);
}
fn resolve(game: &mut GameState, dm: &mut Choices) {
    resolve_stack_entry_with(game, dm).unwrap();
}
fn activation(game: &GameState, source: ObjectId, ordinal: usize) -> Option<LegalAction> {
    let index = game.current_abilities(source).unwrap().iter().enumerate()
        .filter(|(_, ability)| matches!(ability.kind, AbilityKind::Activated(_)))
        .nth(ordinal).unwrap().0;
    compute_legal_actions(game, A).unwrap().into_iter().find(|action| matches!(action, LegalAction::ActivateAbility { source: id, ability_index } if *id == source && *ability_index == index))
}
fn resolve_loyalty_body(game: &mut GameState, source: ObjectId, definition: &CardDefinition, ordinal: usize, targets: Vec<Target>) {
    let ability = definition.abilities.iter().filter_map(|ability| match &ability.kind {
        AbilityKind::Activated(ability) => Some(ability), _ => None,
    }).nth(ordinal).unwrap();
    assert!(ability.is_loyalty_ability);
    game.stack.push(StackEntry::ability(source, A, ability.effects.clone()).with_targets(targets));
    resolve(game, &mut Choices::default());
}

#[test]
fn eight_exact_frozen_identities_preserve_complete_direct_and_artifact_bodies() {
    let expected = [
        ("Beacon of Tomorrows", "85909caa-d2ad-4487-b9b7-6ac60a14b833"),
        ("Karn's Temporal Sundering", "5afbd367-19c7-418f-994d-a7958fb1c4ae"),
        ("Mu Yanling", "34e1477a-c0fb-4364-bb93-04a85c0c2517"),
        ("Teferi, Master of Time", "e802fb53-7cf5-46bc-8a0b-f99cf5c20f74"),
        ("Time Stretch", "72e56963-a9dd-44dc-a4d3-992b4d89dd28"),
        ("Time Warp", "dbd6a94b-62ff-4a10-9d52-bdd90b26e425"),
        ("Walk the Aeons", "0ec36301-6282-49d4-85ea-6a46708e2f60"),
        ("Wormfang Manta", "17abf8c6-70b6-4877-a0f2-a87b22fb2828"),
    ];
    let rows = rows();
    assert_eq!(rows.len(), expected.len());
    for (name, id) in expected {
        assert!(rows.iter().any(|row| row["name"] == name && row["oracle_id"] == id));
        for definition in definitions(name) { assert_eq!(definition.card.name, name); }
    }
}

#[test]
fn time_stretch_announces_one_target_and_later_created_turns_go_first() {
    for route in 0..2 {
        let mut game = game();
        let mut dm = Choices { targets: vec![Target::Player(B)], ..Default::default() };
        cast(&mut game, &definitions("Time Stretch")[route], &mut dm);
        assert_eq!(dm.target_groups, vec![(1, Some(1))]);
        assert_eq!(game.stack.last().unwrap().targets, vec![Target::Player(B)]);
        resolve(&mut game, &mut dm);
        assert_eq!(game.turn_store.extra_turns, vec![B, B]);
        dm.targets = vec![Target::Player(C)];
        cast(&mut game, &definitions("Time Warp")[route], &mut dm);
        resolve(&mut game, &mut dm);
        assert_eq!(game.turn_store.extra_turns, vec![B, B, C]);
        for player in [C, B, B] {
            game.next_turn();
            assert_eq!(game.turn.active_player, player);
            assert!(game.turn_store.current_turn_is_extra);
        }
        game.next_turn();
        assert_eq!(game.turn.active_player, B, "normal order resumes after Alice");
        assert!(!game.turn_store.current_turn_is_extra);
    }
}

#[test]
fn invalid_player_target_does_not_create_turns_or_run_beacons_shuffle() {
    for name in ["Time Stretch", "Beacon of Tomorrows"] { for definition in definitions(name) {
        let mut game = game();
        let mut dm = Choices { targets: vec![Target::Player(B)], ..Default::default() };
        cast(&mut game, &definition, &mut dm);
        game.player_mut(B).unwrap().has_lost = true;
        resolve(&mut game, &mut dm);
        assert!(game.turn_store.extra_turns.is_empty());
        assert!(named(&game, A, Zone::Graveyard, name));
        assert!(!named(&game, A, Zone::Library, name));
    } }
}

#[test]
fn beacon_shuffles_itself_into_its_owners_library_after_granting_the_target_turn() {
    for definition in definitions("Beacon of Tomorrows") {
        let mut game = game();
        library(&mut game, A, 4);
        library(&mut game, B, 2);
        library(&mut game, C, 3);
        let original_owner_cards = game.player(A).unwrap().library.clone();
        let target_library = game.player(B).unwrap().library.clone();
        let other_library = game.player(C).unwrap().library.clone();
        let mut dm = Choices { targets: vec![Target::Player(B)], ..Default::default() };
        cast(&mut game, &definition, &mut dm);
        resolve(&mut game, &mut dm);
        assert_eq!(game.turn_store.extra_turns, vec![B]);
        let owner_library = &game.player(A).unwrap().library;
        assert_eq!(owner_library.len(), 5);
        assert!(original_owner_cards.iter().all(|id| owner_library.contains(id)));
        assert_eq!(game.player(B).unwrap().library, target_library);
        assert_eq!(game.player(C).unwrap().library, other_library);
        let shuffles: Vec<_> = game.turn_store.turn_history.event_records.iter()
            .chain(game.turn_store.turn_history.staged_event_records.iter())
            .filter_map(|record| record.event.downcast::<ironsmith::events::ShuffleLibraryEvent>())
            .map(|event| event.player).collect();
        assert_eq!(shuffles, vec![A]);
        assert!(named(&game, A, Zone::Library, "Beacon of Tomorrows"));
        assert!(!named(&game, A, Zone::Graveyard, "Beacon of Tomorrows"));
        assert!(!named(&game, B, Zone::Library, "Beacon of Tomorrows"));
    }
}

#[test]
fn karn_preserves_legendary_cast_gate_optional_nonland_return_and_self_exile() {
    for definition in definitions("Karn's Temporal Sundering") { for bounce in [false, true] {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Hand);
        let ordinary = creature(&mut game, A, "Ordinary creature");
        assert!(cast_action(&game, source).is_none());
        object(&mut game, B, Zone::Battlefield, "Opponent legend", vec![CardType::Creature], vec![], true);
        assert!(cast_action(&game, source).is_none());
        let legend = object(&mut game, A, Zone::Battlefield, "Own legend", vec![CardType::Creature], vec![], true);
        game.phase_out(legend);
        assert!(cast_action(&game, source).is_none());
        game.phase_in(legend);
        let victim = creature(&mut game, B, "Returned creature");
        let land = object(&mut game, B, Zone::Battlefield, "Excluded Island", vec![CardType::Land], vec![Subtype::Island], false);
        let mut dm = Choices {
            targets: if bounce { vec![Target::Player(C), Target::Object(victim)] } else { vec![Target::Player(C)] },
            forbidden_targets: vec![Target::Object(land)],
            ..Default::default()
        };
        let action = cast_action(&game, source).unwrap();
        announce(&mut game, action, &mut dm);
        assert_eq!(dm.target_groups, vec![(1, Some(1)), (0, Some(1))]);
        resolve(&mut game, &mut dm);
        assert_eq!(game.turn_store.extra_turns, vec![C]);
        assert!(named(&game, A, Zone::Exile, "Karn's Temporal Sundering"));
        assert_eq!(named(&game, B, Zone::Hand, "Returned creature"), bounce);
        assert!(game.object(ordinary).is_some());
    } }
}

#[test]
fn walk_buyback_sacrifices_exactly_three_islands_and_returns_only_when_paid() {
    for definition in definitions("Walk the Aeons") { for paid in [false, true] {
        let mut game = game();
        for n in 0..3 { object(&mut game, A, Zone::Battlefield, &format!("Island {n}"), vec![CardType::Land], vec![Subtype::Island], false); }
        let other = object(&mut game, A, Zone::Battlefield, "Wrong land", vec![CardType::Land], vec![Subtype::Forest], false);
        let opponent_island = object(&mut game, B, Zone::Battlefield, "Opponent Island", vec![CardType::Land], vec![Subtype::Island], false);
        let mut dm = Choices { targets: vec![Target::Player(B)], pay_buyback: paid, ..Default::default() };
        cast(&mut game, &definition, &mut dm);
        assert_eq!(dm.buyback_available, Some(true));
        assert_eq!(game.stack.last().unwrap().optional_costs_paid.was_paid_label("Buyback"), paid);
        assert_eq!(game.player(A).unwrap().graveyard.len(), if paid { 3 } else { 0 });
        assert_eq!(game.object(other).unwrap().zone, Zone::Battlefield);
        assert_eq!(game.object(opponent_island).unwrap().zone, Zone::Battlefield);
        resolve(&mut game, &mut dm);
        assert_eq!(game.turn_store.extra_turns, vec![B]);
        assert_eq!(named(&game, A, Zone::Hand, "Walk the Aeons"), paid);
        assert_eq!(named(&game, A, Zone::Graveyard, "Walk the Aeons"), !paid);
    } }
}

#[test]
fn walk_cannot_offer_buyback_for_two_islands_and_a_different_land() {
    for definition in definitions("Walk the Aeons") {
        let mut game = game();
        for n in 0..2 { object(&mut game, A, Zone::Battlefield, &format!("Island {n}"), vec![CardType::Land], vec![Subtype::Island], false); }
        object(&mut game, A, Zone::Battlefield, "Forest", vec![CardType::Land], vec![Subtype::Forest], false);
        let mut dm = Choices { targets: vec![Target::Player(B)], ..Default::default() };
        cast(&mut game, &definition, &mut dm);
        assert_ne!(dm.buyback_available, Some(true));
        assert!(game.player(A).unwrap().graveyard.is_empty());
        resolve(&mut game, &mut dm);
        assert!(named(&game, A, Zone::Graveyard, "Walk the Aeons"));
    }
}

#[test]
fn mu_yanling_retains_all_three_loyalty_bodies_and_only_taps_opponents_creatures() {
    for definition in definitions("Mu Yanling") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let own = creature(&mut game, A, "Own creature");
        let bob = creature(&mut game, B, "Bob creature");
        let carol = creature(&mut game, C, "Carol creature");
        let artifact = object(&mut game, B, Zone::Battlefield, "Noncreature artifact", vec![CardType::Artifact], vec![], false);
        resolve_loyalty_body(&mut game, source, &definition, 0, vec![Target::Object(own)]);
        assert!(!game.can_be_blocked(own));
        assert!(game.can_be_blocked(bob));
        library(&mut game, A, 3);
        resolve_loyalty_body(&mut game, source, &definition, 1, vec![]);
        assert_eq!(game.player(A).unwrap().hand.len(), 2);
        assert_eq!(game.player(A).unwrap().library.len(), 1);
        resolve_loyalty_body(&mut game, source, &definition, 2, vec![]);
        assert!(!game.is_tapped(own));
        assert!(game.is_tapped(bob));
        assert!(game.is_tapped(carol));
        assert!(!game.is_tapped(artifact));
        assert_eq!(game.turn_store.extra_turns, vec![A]);
        game.next_turn();
        assert_eq!(game.turn.active_player, A);
        assert!(game.can_be_blocked(own));
    }
}

#[test]
fn teferi_activates_on_opponent_turns_once_per_turn_without_granting_other_walkers_permission() {
    for route in 0..2 {
        let mut game = game();
        let definition = &definitions("Teferi, Master of Time")[route];
        let teferi = game.create_object_from_definition(definition, A, Zone::Battlefield);
        let other = game.create_object_from_definition(&definitions("Mu Yanling")[route], A, Zone::Battlefield);
        creature(&mut game, B, "Mu target");
        library(&mut game, A, 4);
        game.turn.active_player = B;
        game.turn.phase = Phase::Beginning;
        game.turn.step = Some(ironsmith::game_state::Step::Upkeep);
        game.turn.priority_player = Some(A);
        assert!(activation(&game, other, 0).is_none());
        let action = activation(&game, teferi, 0).expect("Teferi may activate on Bob's turn");
        announce(&mut game, action, &mut Choices::default());
        resolve(&mut game, &mut Choices::default());
        assert_eq!(game.player(A).unwrap().hand.len(), 0, "draw then discard");
        assert_eq!(game.player(A).unwrap().graveyard.len(), 1);
        assert_eq!(game.counter_count(teferi, CounterType::Loyalty), 4);
        assert!(activation(&game, teferi, 0).is_none(), "still once per turn");
        game.next_turn();
        game.turn.priority_player = Some(A);
        assert_eq!(game.turn.active_player, C);
        assert!(activation(&game, teferi, 0).is_some());
    }
}

#[test]
fn teferi_phases_only_the_target_and_ultimate_creates_two_turns() {
    for definition in definitions("Teferi, Master of Time") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let own = creature(&mut game, A, "Own creature");
        let bob = creature(&mut game, B, "Bob creature");
        let carol = creature(&mut game, C, "Carol creature");
        resolve_loyalty_body(&mut game, source, &definition, 1, vec![Target::Object(bob)]);
        assert!(game.is_phased_out(bob));
        assert!(!game.is_phased_out(own));
        assert!(!game.is_phased_out(carol));
        resolve_loyalty_body(&mut game, source, &definition, 2, vec![]);
        assert_eq!(game.turn_store.extra_turns, vec![A, A]);
        game.next_turn();
        assert_eq!(game.turn.active_player, A);
        assert!(game.is_phased_out(bob));
        game.next_turn();
        assert_eq!(game.turn.active_player, A);
        game.next_turn();
        assert_eq!(game.turn.active_player, B);
        ironsmith::turn::execute_untap_step(&mut game);
        assert!(!game.is_phased_out(bob));
    }
}

fn resolve_one_pending_trigger(game: &mut GameState) {
    let mut queue = TriggerQueue::new();
    put_triggers_on_stack_with_dm(game, &mut queue, &mut Choices::default()).unwrap();
    assert_eq!(game.stack.len(), 1, "one real zone transition creates one trigger");
    resolve(game, &mut Choices::default());
    put_triggers_on_stack_with_dm(game, &mut queue, &mut Choices::default()).unwrap();
    assert!(queue.entries.is_empty());
    assert!(game.stack_is_empty(), "all transition work finishes before advancing turns");
}
#[test]
fn wormfang_entry_skip_consumes_its_leaving_extra_turn_before_normal_order_resumes() {
    for definition in definitions("Wormfang Manta") {
        let mut game = game();
        let in_hand = game.create_object_from_definition(&definition, A, Zone::Hand);
        let source = game.move_object_by_effect(in_hand, Zone::Battlefield).unwrap();
        assert!(game.current_abilities(source).unwrap().iter().any(|ability| matches!(&ability.kind, AbilityKind::Static(ability) if ability.has_flying())));
        resolve_one_pending_trigger(&mut game);
        assert_eq!(game.turn_store.skip_next_turn.pending(A), 1);
        game.move_object_by_effect(source, Zone::Graveyard).unwrap();
        resolve_one_pending_trigger(&mut game);
        assert_eq!(game.turn_store.extra_turns, vec![A]);
        game.next_turn();
        assert_eq!(game.turn.active_player, B);
        assert_eq!(game.turn_store.skip_next_turn.pending(A), 0);
        assert!(game.turn_store.extra_turns.is_empty());
        game.next_turn();
        game.next_turn();
        assert_eq!(game.turn.active_player, A);
    }
}

fn set_loyalty(game: &mut GameState, source: ObjectId, loyalty: u32) {
    let current = game.counter_count(source, CounterType::Loyalty);
    if current < loyalty {
        game.add_counters(source, CounterType::Loyalty, loyalty - current);
    } else if current > loyalty {
        game.remove_counters(source, CounterType::Loyalty, current - loyalty, None, None);
    }
}

#[test]
fn mu_native_activations_pay_printed_loyalty_and_enforce_sorcery_and_once_turn_limits() {
    for definition in definitions("Mu Yanling") { for (ordinal, before, after) in [(0, 5, 7), (1, 5, 2), (2, 11, 1)] {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let target = creature(&mut game, B, "Mu native target");
        library(&mut game, A, 3);
        set_loyalty(&mut game, source, before);
        game.turn.active_player = B;
        assert!(activation(&game, source, ordinal).is_none(), "Mu is restricted to her controller's turn");
        game.turn.active_player = A;
        game.turn.phase = Phase::Beginning;
        game.turn.step = Some(ironsmith::game_state::Step::Upkeep);
        assert!(activation(&game, source, ordinal).is_none(), "Mu needs sorcery timing");
        game.turn.phase = Phase::FirstMain;
        game.turn.step = None;
        if ordinal > 0 {
            set_loyalty(&mut game, source, if ordinal == 1 { 2 } else { 9 });
            assert!(activation(&game, source, ordinal).is_none(), "insufficient loyalty cannot pay the printed cost");
            set_loyalty(&mut game, source, before);
        }
        let action = activation(&game, source, ordinal).expect("printed ability is legal with enough loyalty");
        let mut dm = Choices { targets: if ordinal == 0 { vec![Target::Object(target)] } else { vec![] }, ..Default::default() };
        announce(&mut game, action, &mut dm);
        assert_eq!(game.counter_count(source, CounterType::Loyalty), after, "cost is paid on announcement");
        resolve(&mut game, &mut dm);
        match ordinal {
            0 => assert!(!game.can_be_blocked(target)),
            1 => assert_eq!(game.player(A).unwrap().hand.len(), 2),
            2 => {
                assert!(game.is_tapped(target));
                assert_eq!(game.turn_store.extra_turns, vec![A]);
            }
            _ => unreachable!(),
        }
        assert!(activation(&game, source, 0).is_none(), "paying any loyalty ability consumes the turn's one activation");
    } }
}

#[test]
fn teferi_native_negative_loyalty_abilities_pay_costs_on_opponents_turn_and_exclude_own_creatures() {
    for definition in definitions("Teferi, Master of Time") { for (ordinal, before, insufficient) in [(1, 4, 2), (2, 11, 9)] {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let own = creature(&mut game, A, "Own excluded creature");
        let target = creature(&mut game, B, "Teferi native target");
        game.turn.active_player = B;
        game.turn.phase = Phase::Beginning;
        game.turn.step = Some(ironsmith::game_state::Step::Upkeep);
        game.turn.priority_player = Some(A);
        set_loyalty(&mut game, source, insufficient);
        assert!(activation(&game, source, ordinal).is_none(), "instant timing does not waive loyalty costs");
        set_loyalty(&mut game, source, before);
        let action = activation(&game, source, ordinal).expect("Teferi can activate on an opponent's upkeep");
        let mut dm = Choices {
            targets: if ordinal == 1 { vec![Target::Object(target)] } else { vec![] },
            forbidden_targets: vec![Target::Object(own)],
            ..Default::default()
        };
        announce(&mut game, action, &mut dm);
        assert_eq!(game.counter_count(source, CounterType::Loyalty), 1);
        if ordinal == 1 { assert_eq!(dm.target_groups, vec![(1, Some(1))]); }
        resolve(&mut game, &mut dm);
        assert!(!game.is_phased_out(own));
        if ordinal == 1 {
            assert!(game.is_phased_out(target));
        } else {
            assert_eq!(game.turn_store.extra_turns, vec![A, A]);
        }
        library(&mut game, A, 1);
        assert!(activation(&game, source, 0).is_none(), "Teferi's permission retains once-per-turn loyalty limit");
    } }
}
