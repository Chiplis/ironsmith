//! Frozen complete Oracle bodies. Authored without execution while the campaign gate is closed.
use ironsmith::alternative_cast::CastingMethod;
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::color::ColorSet;
use ironsmith::decision::{DecisionMaker, GameProgress, LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
use ironsmith::decisions::context::{DecisionContext, SelectObjectsContext, TargetsContext, ViewCardsContext};
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, drain_pending_trigger_events, put_triggers_on_stack_with_dm,
    resolve_stack_entry_with};
use ironsmith::game_state::Target;
use ironsmith::mana::ManaSymbol;
use ironsmith::special_actions::{SpecialAction, TurnFaceUpMethod};
use ironsmith::static_abilities::StaticAbilityId;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{CardDefinition, CardId, CardType, GameState, ObjectId, PlayerId, Zone};

const ALICE: PlayerId = PlayerId(0);
const BOB: PlayerId = PlayerId(1);
const CARA: PlayerId = PlayerId(2);
const NAMES: [&str; 5] = ["Dragon's Eye Savants", "Horde Ambusher", "Ruthless Ripper", "Temur Charger", "Watcher of the Roost"];

#[derive(Clone, Copy, Debug)]
enum Route { Direct, Artifact }
const ROUTES: [Route; 2] = [Route::Direct, Route::Artifact];

fn compile(name: &str, text: &str, route: Route) -> CardDefinition {
    let builder = || ironsmith_compiler::CardDefinitionBuilder::new(CardId::new(), name);
    match route {
        Route::Direct => {
            let (result, loss) = ironsmith_compiler::parse_loss::capture(||
                ironsmith_registry::compile_builder_to_runtime_definition(builder(), text.to_owned(), false));
            assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
            result.unwrap_or_else(|error| panic!("{name}: {error:?}"))
        }
        Route::Artifact => {
            let (result, loss) = ironsmith_compiler::parse_loss::capture(||
                ironsmith_registry::compile_builder_to_artifact(builder(), text.to_owned(), false));
            assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
            let (artifact, _) = result.unwrap_or_else(|error| panic!("{name}: {error:?}"));
            artifact.validate().unwrap();
            let decoded = serde_json::from_slice(&serde_json::to_vec(&artifact).unwrap()).unwrap();
            ironsmith::artifact_materializer::materialize_artifact(&decoded).unwrap()
        }
    }
}

fn fixture(name: &str) -> serde_json::Value {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/card-failure-campaign/reveal-in-hand-morph-costs.json")).unwrap();
    fixture["cards"].as_array().unwrap().iter().find(|card| card["name"] == name).unwrap().clone()
}
fn definition(name: &str, route: Route) -> CardDefinition {
    compile(name, fixture(name)["text"].as_str().unwrap(), route)
}
fn color(name: &str) -> ColorSet {
    match name {
        "Dragon's Eye Savants" => ColorSet::BLUE,
        "Horde Ambusher" => ColorSet::RED,
        "Ruthless Ripper" => ColorSet::BLACK,
        "Temur Charger" => ColorSet::GREEN,
        "Watcher of the Roost" => ColorSet::WHITE,
        _ => panic!("unknown fixture"),
    }
}

#[derive(Default)]
struct Choices {
    selected: Option<Vec<ObjectId>>,
    target: Option<Target>,
    pause: bool,
    pending: bool,
    object_choices: Vec<SelectObjectsContext>,
    views: Vec<(PlayerId, PlayerId, bool, Vec<ObjectId>)>,
}
impl DecisionMaker for Choices {
    fn answers_player_choices(&self) -> bool { true }
    fn awaiting_choice(&self) -> bool { self.pending }
    fn decide_objects(&mut self, game: &GameState, ctx: &SelectObjectsContext) -> Vec<ObjectId> {
        self.object_choices.push(ctx.clone());
        self.pending = self.pause;
        self.selected.clone().unwrap_or_else(|| SelectFirstDecisionMaker.decide_objects(game, ctx))
    }
    fn decide_targets(&mut self, game: &GameState, ctx: &TargetsContext) -> Vec<Target> {
        if let Some(target) = self.target {
            assert!(ctx.requirements.iter().all(|requirement| requirement.legal_targets.contains(&target)), "{ctx:?}");
            vec![target]
        } else { SelectFirstDecisionMaker.decide_targets(game, ctx) }
    }
    fn view_cards(&mut self, _: &GameState, viewer: PlayerId, cards: &[ObjectId], ctx: &ViewCardsContext) {
        self.views.push((viewer, ctx.subject, ctx.public, cards.to_vec()));
    }
}

fn setup() -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Cara".into()], 20);
    game.set_random_seed(20261005);
    game.turn.turn_number = 5;
    main(&mut game, ALICE);
    for player in [ALICE, BOB, CARA] {
        for symbol in [ManaSymbol::White, ManaSymbol::Blue, ManaSymbol::Black, ManaSymbol::Red, ManaSymbol::Green, ManaSymbol::Colorless] {
            game.player_mut(player).unwrap().mana_pool.add(symbol, 20);
        }
    }
    game
}
fn main(game: &mut GameState, player: PlayerId) {
    game.turn.active_player = player;
    game.turn.priority_player = Some(player);
    game.turn.phase = ironsmith::Phase::FirstMain;
    game.turn.step = None;
}
fn card(game: &mut GameState, owner: PlayerId, zone: Zone, name: &str, colors: ColorSet) -> ObjectId {
    let definition = CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .color_indicator(colors)
        .power_toughness(ironsmith::card::PowerToughness::fixed(4, 4)).build();
    game.create_object_from_definition(&definition, owner, zone)
}
fn track(game: &mut GameState, id: ObjectId, slot: u16) {
    game.set_hidden_card_info(id, ironsmith::game_state::HiddenCardInfo {
                incarnation: Some(0),
        owner: game.object(id).unwrap().owner, zone: Zone::Hand, slot,
        commitment: format!("reveal-morph-{slot}"), origin_slot: None, origin_commitment: None,
        public_slot: None, public_commitment: None,
    });
}
fn dispatch(game: &mut GameState, queue: &mut TriggerQueue, action: LegalAction, choices: &mut Choices) {
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut progress = apply_priority_response_with_dm(game, queue, &mut state,
        &PriorityResponse::PriorityAction(action), choices).unwrap();
    for _ in 0..40 {
        match progress {
            GameProgress::NeedsDecisionCtx(ref ctx) if !matches!(ctx, DecisionContext::Priority(_)) => {
                progress = apply_decision_context_with_dm(game, queue, &mut state, ctx, choices).unwrap();
            }
            _ => { assert!(state.pending_cast.is_none() && state.pending_activation.is_none()); return; }
        }
    }
    panic!("native action failed to finish");
}
fn finish(game: &mut GameState, queue: &mut TriggerQueue, choices: &mut Choices) {
    for _ in 0..20 {
        drain_pending_trigger_events(game, queue);
        put_triggers_on_stack_with_dm(game, queue, choices).unwrap();
        if game.stack.is_empty() { return; }
        resolve_stack_entry_with(game, choices).unwrap();
    }
    panic!("native stack failed to finish");
}
fn cast(game: &mut GameState, definition: &CardDefinition, actor: PlayerId, face_down: bool, choices: &mut Choices) -> ObjectId {
    main(game, actor);
    let id = game.create_object_from_definition(definition, actor, Zone::Hand);
    let stable = game.object(id).unwrap().stable_id;
    let before = game.player(actor).unwrap().mana_pool.total();
    let action = compute_legal_actions(game, actor).unwrap().into_iter().find(|action| matches!(action,
        LegalAction::CastSpell { spell_id, casting_method, .. } if *spell_id == id
            && if face_down { matches!(casting_method, CastingMethod::FaceDown) }
               else { matches!(casting_method, CastingMethod::Normal) })).expect("actual legal cast");
    let mut queue = TriggerQueue::new();
    dispatch(game, &mut queue, action, choices);
    let spell = game.find_object_by_stable_id(stable).unwrap();
    assert_eq!(game.object(spell).unwrap().zone, Zone::Stack);
    assert_eq!(game.is_face_down(spell), face_down);
    let expected = if face_down { 3 } else { match definition.name() { "Ruthless Ripper" => 1, "Watcher of the Roost" => 3, _ => 2 } };
    assert_eq!(before - game.player(actor).unwrap().mana_pool.total(), expected);
    finish(game, &mut queue, choices);
    let permanent = game.find_object_by_stable_id(stable).unwrap();
    assert_eq!(game.object(permanent).unwrap().zone, Zone::Battlefield);
    assert_eq!(game.is_face_down(permanent), face_down);
    permanent
}
fn turn_action(game: &GameState, source: ObjectId, actor: PlayerId) -> Option<LegalAction> {
    compute_legal_actions(game, actor).unwrap().into_iter().find(|action| matches!(action,
        LegalAction::TurnFaceUp { creature_id, method: TurnFaceUpMethod::TurnFaceUpAbility } if *creature_id == source))
}
fn special(source: ObjectId) -> SpecialAction {
    SpecialAction::TurnFaceUp { permanent_id: source, method: TurnFaceUpMethod::TurnFaceUpAbility }
}
fn assert_characteristics(game: &GameState, source: ObjectId, name: &str) {
    let row = fixture(name);
    assert_eq!(game.object(source).unwrap().name, name);
    assert_eq!(game.calculated_power(source), Some(row["power"].as_str().unwrap().parse().unwrap()));
    assert_eq!(game.calculated_toughness(source), Some(row["toughness"].as_str().unwrap().parse().unwrap()));
    assert_eq!(game.current_colors(source), Some(color(name)));
    assert!(game.object(source).unwrap().card_types.contains(&CardType::Creature));
    let expected_types: &[ironsmith::Subtype] = match name {
        "Dragon's Eye Savants" => &[ironsmith::Subtype::Human, ironsmith::Subtype::Wizard],
        "Horde Ambusher" => &[ironsmith::Subtype::Human, ironsmith::Subtype::Berserker],
        "Ruthless Ripper" => &[ironsmith::Subtype::Human, ironsmith::Subtype::Assassin],
        "Temur Charger" => &[ironsmith::Subtype::Horse],
        "Watcher of the Roost" => &[ironsmith::Subtype::Bird, ironsmith::Subtype::Soldier],
        _ => unreachable!(),
    };
    assert_eq!(game.object(source).unwrap().subtypes.as_slice(), expected_types);
    assert_eq!(game.object_has_static_ability_id(source, StaticAbilityId::Deathtouch), name == "Ruthless Ripper");
    assert_eq!(game.object_has_static_ability_id(source, StaticAbilityId::Flying), name == "Watcher of the Roost");
}

#[test]
fn five_full_bodies_cast_normally_or_face_down_then_pay_exact_color_reveal_and_resolve() {
    for route in ROUTES { for name in NAMES { for face_down in [false, true] {
        let definition = definition(name, route);
        let mut game = setup();
        let mut choices = Choices::default();
        let source = cast(&mut game, &definition, ALICE, face_down, &mut choices);
        if !face_down {
            assert_characteristics(&game, source, name);
            assert_eq!(game.player(ALICE).unwrap().life, 20);
            assert!(choices.views.is_empty());
            continue;
        }
        assert_eq!((game.calculated_power(source), game.calculated_toughness(source)), (Some(2), Some(2)));
        assert_eq!(game.current_colors(source), Some(ColorSet::COLORLESS));
        assert!(!game.object_has_static_ability_id(source, StaticAbilityId::Flying));
        assert!(!game.object_has_static_ability_id(source, StaticAbilityId::Deathtouch));
        let first = card(&mut game, ALICE, Zone::Hand, "Unchosen matching card", color(name));
        let chosen = card(&mut game, ALICE, Zone::Hand, "Chosen matching card", color(name));
        let wrong = card(&mut game, ALICE, Zone::Hand, "Wrong color", ColorSet::COLORLESS);
        track(&mut game, first, 0); track(&mut game, chosen, 1); track(&mut game, wrong, 2);
        let foreign = card(&mut game, BOB, Zone::Hand, "Opponent hand", color(name));
        let target = card(&mut game, BOB, Zone::Battlefield, "Target creature", ColorSet::COLORLESS);
        let untouched = card(&mut game, CARA, Zone::Battlefield, "Untargeted creature", ColorSet::COLORLESS);
        choices.selected = Some(vec![chosen]);
        choices.target = match name { "Dragon's Eye Savants" | "Ruthless Ripper" => Some(Target::Player(BOB)),
            "Horde Ambusher" | "Temur Charger" => Some(Target::Object(target)), _ => None };
        let hand = game.player(ALICE).unwrap().hand.clone();
        let stable = game.object(chosen).unwrap().stable_id;
        let mana = game.player(ALICE).unwrap().mana_pool.clone();
        let mut queue = TriggerQueue::new();
        let action = turn_action(&game, source, ALICE).expect("matching own hand card enables morph");
        dispatch(&mut game, &mut queue, action, &mut choices);
        assert!(!game.is_face_down(source));
        assert_eq!(game.player(ALICE).unwrap().mana_pool, mana);
        assert_eq!(game.player(ALICE).unwrap().hand, hand);
        assert_eq!(game.object(chosen).unwrap().stable_id, stable);
        assert!(game.is_publicly_revealed_hidden_card(chosen));
        assert!(!game.is_publicly_revealed_hidden_card(first));
        assert!(!game.is_publicly_revealed_hidden_card(wrong));
        assert_characteristics(&game, source, name);
        assert_eq!(choices.views.iter().filter(|view| view.2).count(), 3);
        for view in choices.views.iter().filter(|view| view.2) { assert_eq!(view.1, ALICE); assert_eq!(view.3, vec![chosen]); }
        let prompt = choices.object_choices.last().unwrap();
        assert_eq!(prompt.player, ALICE);
        assert_eq!(prompt.min, 1); assert_eq!(prompt.max, Some(1));
        assert_eq!(prompt.reveal_policy, ironsmith::decisions::context::SelectionRevealPolicy::Public);
        assert_eq!(prompt.candidates.iter().filter(|c| c.legal).map(|c| c.id).collect::<Vec<_>>(), vec![first, chosen]);
        finish(&mut game, &mut queue, &mut choices);
        assert_eq!(game.player(ALICE).unwrap().life, if name == "Watcher of the Roost" { 22 } else { 20 });
        assert_eq!(game.player(BOB).unwrap().life, if name == "Ruthless Ripper" { 18 } else { 20 });
        assert_eq!(game.player(CARA).unwrap().life, 20);
        assert_eq!(game.can_block(target), name != "Horde Ambusher");
        assert!(game.can_block(untouched));
        assert_eq!(game.object_has_static_ability_id(target, StaticAbilityId::Trample), name == "Temur Charger");
        assert!(!game.object_has_static_ability_id(untouched, StaticAbilityId::Trample));
        let private = choices.views.iter().filter(|view| !view.2).collect::<Vec<_>>();
        if name == "Dragon's Eye Savants" {
            assert!(private.iter().any(|view| view.0 == ALICE && view.1 == BOB && view.3 == vec![foreign]));
            assert!(private.iter().all(|view| view.0 == ALICE));
        } else { assert!(private.is_empty()); }
        ironsmith::turn::execute_cleanup_step(&mut game);
        assert!(game.can_block(target));
        assert!(!game.object_has_static_ability_id(target, StaticAbilityId::Trample));
    } } }
}

#[test]
fn morph_uses_current_controller_hand_and_rejects_wrong_zone_color_and_tokens() {
    for route in ROUTES { for name in NAMES {
        let mut game = setup(); let mut choices = Choices::default();
        let source = cast(&mut game, &definition(name, route), BOB, true, &mut choices);
        game.set_current_controller(source, ALICE); main(&mut game, ALICE);
        card(&mut game, BOB, Zone::Hand, "Original owner's match", color(name));
        card(&mut game, ALICE, Zone::Graveyard, "Wrong zone match", color(name));
        card(&mut game, ALICE, Zone::Hand, "Wrong color", ColorSet::COLORLESS);
        let token = card(&mut game, ALICE, Zone::Hand, "Not a card", color(name));
        game.object_mut(token).unwrap().kind = ironsmith::object::ObjectKind::Token;
        assert!(turn_action(&game, source, ALICE).is_none());
        assert!(ironsmith::special_actions::perform(special(source), &mut game, ALICE, &mut choices).is_err());
        assert!(game.is_face_down(source)); assert!(choices.views.is_empty());
        let chosen = card(&mut game, ALICE, Zone::Hand, "Current controller's card", color(name));
        choices.selected = Some(vec![chosen]);
        assert!(turn_action(&game, source, ALICE).is_some());
        ironsmith::special_actions::perform(special(source), &mut game, ALICE, &mut choices).unwrap();
        assert!(!game.is_face_down(source));
        assert_eq!(game.object(source).unwrap().owner, BOB);
        assert_eq!(game.current_controller(source), Some(ALICE));
        assert!(choices.views.iter().all(|view| view.1 == ALICE && view.3 == vec![chosen]));
    } }
}

#[test]
fn pending_and_malformed_morph_answers_never_disclose_or_substitute_a_payment() {
    for route in ROUTES {
        let mut game = setup(); let mut choices = Choices::default();
        let source = cast(&mut game, &definition("Ruthless Ripper", route), ALICE, true, &mut choices);
        let first = card(&mut game, ALICE, Zone::Hand, "First", ColorSet::BLACK);
        let second = card(&mut game, ALICE, Zone::Hand, "Second", ColorSet::BLACK);
        let wrong = card(&mut game, BOB, Zone::Hand, "Foreign", ColorSet::BLACK);
        track(&mut game, first, 0); track(&mut game, second, 1);
        let hand = game.player(ALICE).unwrap().hand.clone();
        for selected in [vec![], vec![wrong], vec![first, first], vec![first, second]] {
            choices.selected = Some(selected);
            assert!(ironsmith::special_actions::perform(special(source), &mut game, ALICE, &mut choices).is_err());
            assert!(game.is_face_down(source)); assert!(choices.views.is_empty());
            assert_eq!(game.player(ALICE).unwrap().hand, hand);
        }
        choices.selected = Some(vec![second]); choices.pause = true;
        ironsmith::special_actions::perform(special(source), &mut game, ALICE, &mut choices).unwrap();
        assert!(choices.awaiting_choice()); assert!(choices.views.is_empty()); assert!(game.is_face_down(source));
        choices.pause = false; choices.pending = false;
        ironsmith::special_actions::perform(special(source), &mut game, ALICE, &mut choices).unwrap();
        assert!(!game.is_face_down(source)); assert_eq!(choices.views.len(), 3);
        assert!(choices.views.iter().all(|view| view.3 == vec![second]));
        assert_eq!(game.player(ALICE).unwrap().hand, hand);
        assert!(ironsmith::special_actions::perform(special(source), &mut game, ALICE, &mut choices).is_err());
        assert_eq!(choices.views.len(), 3, "a completed action cannot reveal twice");
    }
}

#[test]
fn horde_ambusher_blocks_trigger_uses_actual_controller_and_is_absent_face_down() {
    use ironsmith::combat_state::{AttackTarget, CombatState};
    use ironsmith::decision::{AttackerDeclaration, BlockerDeclaration};
    for route in ROUTES { for face_down in [false, true] {
        let mut game = setup(); let mut choices = Choices::default();
        let source = cast(&mut game, &definition("Horde Ambusher", route), BOB, face_down, &mut choices);
        game.set_current_controller(source, ALICE);
        let attacker = card(&mut game, BOB, Zone::Battlefield, "Attacker", ColorSet::COLORLESS);
        game.remove_summoning_sickness(attacker);
        game.turn.active_player = BOB; game.turn.phase = ironsmith::Phase::Combat;
        let mut queue = TriggerQueue::new(); let mut combat = CombatState::default();
        ironsmith::game_loop::apply_attacker_declarations(&mut game, &mut combat, &mut queue,
            &[AttackerDeclaration { creature: attacker, target: AttackTarget::Player(ALICE) }]).unwrap();
        ironsmith::game_loop::apply_blocker_declarations(&mut game, &mut combat, &mut queue,
            &[BlockerDeclaration { blocker: source, blocking: attacker }], ALICE).unwrap();
        finish(&mut game, &mut queue, &mut choices);
        assert_eq!(game.player(ALICE).unwrap().life, if face_down { 20 } else { 19 });
        assert_eq!(game.player(BOB).unwrap().life, 20);
    } }
}

#[test]
fn shared_activation_reader_preserves_type_fixed_count_and_exact_selection() {
    for route in ROUTES {
        let definition = compile("Typed reveal activation", "Type: Artifact\nReveal two blue creature cards in your hand: You gain 3 life.", route);
        let mut game = setup(); let mut choices = Choices::default();
        let source = game.create_object_from_definition(&definition, ALICE, Zone::Battlefield);
        let first = card(&mut game, ALICE, Zone::Hand, "First blue creature", ColorSet::BLUE);
        let second = card(&mut game, ALICE, Zone::Hand, "Second blue creature", ColorSet::BLUE);
        let wrong_type = card(&mut game, ALICE, Zone::Hand, "Blue noncreature", ColorSet::BLUE);
        game.object_mut(wrong_type).unwrap().card_types = vec![CardType::Artifact].into();
        choices.selected = Some(vec![second, first]);
        let action = compute_legal_actions(&game, ALICE).unwrap().into_iter().find(|action| matches!(action,
            LegalAction::ActivateAbility { source: id, .. } if *id == source)).unwrap();
        let mut queue = TriggerQueue::new();
        dispatch(&mut game, &mut queue, action, &mut choices); finish(&mut game, &mut queue, &mut choices);
        assert_eq!(game.player(ALICE).unwrap().life, 23);
        assert_eq!(game.player(ALICE).unwrap().hand, vec![first, second, wrong_type]);
        assert!(choices.views.iter().all(|view| view.3 == vec![second, first]));
    }
}

#[test]
fn an_unopened_selected_placeholder_cannot_complete_the_native_reveal_payment() {
    for route in ROUTES { for name in NAMES { for matching in [false, true] {
        let mut game = setup(); let mut choices = Choices::default();
        let source = cast(&mut game, &definition(name, route), ALICE, true, &mut choices);
        let placeholder = game.create_hidden_card_placeholder(ALICE, Zone::Hand, 0, "morph-placeholder".into());
        let stable = game.object(placeholder).unwrap().stable_id;
        assert!(turn_action(&game, source, ALICE).is_some(), "a peer must offer the public-opening choice");
        choices.selected = Some(vec![placeholder]); choices.pause = true;
        ironsmith::special_actions::perform(special(source), &mut game, ALICE, &mut choices).unwrap();
        assert!(choices.pending); assert!(choices.views.is_empty());
        assert_eq!(choices.object_choices.last().unwrap().reveal_policy,
            ironsmith::decisions::context::SelectionRevealPolicy::Public);
        choices.pause = false; choices.pending = false;
        let result = ironsmith::special_actions::perform(special(source), &mut game, ALICE, &mut choices);
        assert!(matches!(result, Err(ironsmith::special_actions::ActionError::ExecutionFailure {
            error: ironsmith::effects::ExecutionError::IncompleteEvidence(_), ..
        })), "{result:?}");
        assert!(game.is_face_down(source)); assert!(choices.views.is_empty());
        assert!(!game.is_publicly_revealed_hidden_card(placeholder));
        assert_eq!(game.object(placeholder).unwrap().stable_id, stable);
        // This is the existing engine opening boundary used after the peer
        // transport authenticates the selected identity; it preserves the
        // original hand object rather than creating a substitute card.
        let opened = CardDefinitionBuilder::new(CardId::new(), "Opened payment identity")
            .card_types(vec![CardType::Instant])
            .color_indicator(if matching { color(name).union(if color(name) == ColorSet::BLUE { ColorSet::RED } else { ColorSet::BLUE }) } else { ColorSet::COLORLESS }).build();
        game.reveal_hidden_card_with_definition(placeholder, &opened).unwrap();
        assert_eq!(game.object(placeholder).unwrap().stable_id, stable);
        assert_eq!(turn_action(&game, source, ALICE).is_some(), matching);
        let result = ironsmith::special_actions::perform(special(source), &mut game, ALICE, &mut choices);
        assert_eq!(result.is_ok(), matching);
        assert_eq!(game.is_face_down(source), !matching);
        assert_eq!(choices.views.len(), if matching { 3 } else { 0 });
        assert_eq!(game.object(placeholder).unwrap().zone, Zone::Hand);
    } } }
}

#[test]
fn counted_reveal_cost_rejects_duplicate_short_and_wrong_type_explicit_payments() {
    for route in ROUTES {
        let definition = compile("Exact counted reveal", "Type: Artifact\nReveal two blue creature cards in your hand: You gain 3 life.", route);
        let mut game = setup();
        let source = game.create_object_from_definition(&definition, ALICE, Zone::Battlefield);
        let first = card(&mut game, ALICE, Zone::Hand, "First", ColorSet::BLUE);
        let second = card(&mut game, ALICE, Zone::Hand, "Second", ColorSet::BLUE);
        let wrong = card(&mut game, ALICE, Zone::Hand, "Wrong", ColorSet::RED);
        let activated = definition.abilities.iter().find_map(|ability| match &ability.kind {
            ironsmith::ability::AbilityKind::Activated(activated) => Some(activated), _ => None,
        }).unwrap();
        let cost = &activated.mana_cost.costs()[0];
        for selection in [vec![first], vec![first, first], vec![first, wrong], vec![first, second, wrong]] {
            let mut choices = Choices::default();
            let mut ctx = ironsmith::costs::CostContext::new(source, ALICE, &mut choices).with_pre_chosen_cards(selection);
            assert!(cost.pay(&mut game, &mut ctx).is_err());
            assert!(choices.views.is_empty());
            assert_eq!(game.player(ALICE).unwrap().hand, vec![first, second, wrong]);
        }
    }
}
