//! Complete frozen Aminatou body; lifecycle scenarios are authored without execution.
use ironsmith::cards::CardDefinition;
use ironsmith::decision::{DecisionMaker, SelectFirstDecisionMaker};
use ironsmith::decisions::context::PartitionContext;
use ironsmith::game_loop::{put_triggers_on_stack_with_dm, resolve_stack_entry_with};
use ironsmith::triggers::{check_triggers, TriggerEvent, TriggerQueue};
use ironsmith::{GameState, ObjectId, PlayerId, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler::parse_loss;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
const A: PlayerId = PlayerId::from_index(0);
const B: PlayerId = PlayerId::from_index(1);
fn definitions() -> [CardDefinition; 2] {
    let row: serde_json::Value = serde_json::from_str(include_str!("../../../fixtures/aminatou_body.json.fixture")).unwrap();
    let name = row["name"].as_str().unwrap();
    let text = format!("Mana cost: {}\nType: {}\nPower/Toughness: {}/{}\n{}", row["mana_cost"].as_str().unwrap(),
        row["type_line"].as_str().unwrap(), row["power"].as_str().unwrap(), row["toughness"].as_str().unwrap(), row["oracle_text"].as_str().unwrap());
    let (direct, loss) = parse_loss::capture(|| compile_to_runtime_definition(name, &text, false));
    let direct = direct.unwrap();
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let (artifact, loss) = parse_loss::capture(|| compile_to_artifact(name, &text, false));
    let (artifact, _) = artifact.unwrap();
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    restored.validate().unwrap(); assert_eq!(restored, artifact);
    let definitions = [direct, ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap()];
    for definition in &definitions { assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(definition)); }
    definitions
}
fn resource(game: &mut GameState, name: &str, owner: PlayerId, zone: Zone, kind: &str, cost: &str) -> ObjectId {
    let text = if cost.is_empty() { format!("Type: {kind}") } else { format!("Mana cost: {cost}\nType: {kind}") };
    let definition = compile_to_runtime_definition(name, &text, false).unwrap();
    game.create_object_from_definition(&definition, owner, zone)
}
struct Surveil { seen: Vec<ObjectId> }
impl DecisionMaker for Surveil {
    fn decide_partition(&mut self, _: &GameState, context: &PartitionContext) -> Vec<ObjectId> {
        self.seen = context.cards.iter().map(|(id, _)| *id).collect(); self.seen.clone()
    }
}
#[test]
fn complete_body_keeps_the_own_upkeep_surveil_and_enchantment_hand_grant() {
    for definition in definitions() {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let own = resource(&mut game, "Own enchantment", A, Zone::Hand, "Enchantment", "{6}{U}");
        let other = resource(&mut game, "Other enchantment", B, Zone::Hand, "Enchantment", "{6}{U}");
        let wrong_type = resource(&mut game, "Artifact", A, Zone::Hand, "Artifact", "{6}");
        let wrong_zone = resource(&mut game, "Graveyard enchantment", A, Zone::Graveyard, "Enchantment", "{6}{U}");
        game.refresh_continuous_state().unwrap();
        let grants = |game: &GameState, card, owner, zone| game.effect_store.grant_registry
            .granted_alternative_casts_for_card(game, card, zone, owner);
        let granted = grants(&game, own, A, Zone::Hand);
        assert_eq!(granted.len(), 1);
        assert_eq!(granted[0].source_id, source);
        assert!(granted[0].permission_identity.is_some());
        assert!(granted[0].method.is_miracle());
        assert!(grants(&game, other, B, Zone::Hand).is_empty());
        assert!(grants(&game, wrong_type, A, Zone::Hand).is_empty());
        assert!(grants(&game, wrong_zone, A, Zone::Graveyard).is_empty());
        let bottom = resource(&mut game, "Bottom", A, Zone::Library, "Land", "");
        let middle = resource(&mut game, "Middle", A, Zone::Library, "Land", "");
        let top = resource(&mut game, "Top", A, Zone::Library, "Land", "");
        let opponent = TriggerEvent::new_with_provenance(ironsmith::events::BeginningOfUpkeepEvent::new(B), Default::default());
        assert!(check_triggers(&game, &opponent).is_empty());
        let own_upkeep = TriggerEvent::new_with_provenance(ironsmith::events::BeginningOfUpkeepEvent::new(A), Default::default());
        let mut queue = TriggerQueue::new();
        for trigger in check_triggers(&game, &own_upkeep) { queue.add(trigger); }
        put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut SelectFirstDecisionMaker).unwrap();
        assert_eq!(game.stack.len(), 1);
        let mut answers = Surveil { seen: vec![] };
        resolve_stack_entry_with(&mut game, &mut answers).unwrap();
        assert_eq!(answers.seen, vec![top, middle]);
        assert_eq!(game.player(A).unwrap().library, vec![bottom]);
        assert_eq!(game.player(A).unwrap().graveyard.len(), 3);
    }
}

#[derive(Default)]
struct MiracleAnswers {
    reveal: bool, cast: bool, select_reduction: Option<u32>, reveal_questions: usize,
    pause: bool, pending: bool, granter_at_reveal: Option<ObjectId>, drawer: Option<PlayerId>,
}
impl DecisionMaker for MiracleAnswers {
    fn decide_options(&mut self, game: &GameState, context: &ironsmith::decisions::context::SelectOptionsContext) -> Vec<usize> {
        if context.description == "Choose a Miracle reveal" {
            self.reveal_questions += 1;
            assert_eq!(context.player, self.drawer.unwrap_or(A));
            if let Some(source) = self.granter_at_reveal {
                assert!(game.object(source).is_some(), "the original reveal choice precedes replacement additions");
            }
            if self.pause { self.pending = true; return vec![]; }
            if !self.reveal { return vec![0]; }
            let option = context.options.iter().find(|option| option.index > 0
                && self.select_reduction.is_none_or(|reduction| option.description.contains(&format!("{{{reduction}}}")))).unwrap();
            return vec![option.index];
        }
        SelectFirstDecisionMaker.decide_options(game, context)
    }
    fn decide_boolean(&mut self, _: &GameState, _: &ironsmith::decisions::context::BooleanContext) -> bool { self.cast }
    fn awaiting_choice(&self) -> bool { self.pending }
}
fn perform_draw(game: &mut GameState, source: ObjectId, player: PlayerId, count: u32, answers: &mut impl DecisionMaker) -> ironsmith::effect::EffectOutcome {
    let mut ctx = ironsmith::effects::EffectContext::new(source, player, answers);
    ironsmith::effects::execute_effect(game, &ironsmith::Effect::new(
        ironsmith::effects::DrawCardsEffect::new(count, ironsmith::target::PlayerFilter::Specific(player))), &mut ctx).unwrap()
}
fn stack_draw(game: &mut GameState, outcome: ironsmith::effect::EffectOutcome, answers: &mut impl DecisionMaker) {
    for event in outcome.events { game.queue_trigger_event(event.provenance(), event); }
    let mut queue = TriggerQueue::new();
    ironsmith::game_loop::check_and_apply_sbas(game, &mut queue).unwrap();
    put_triggers_on_stack_with_dm(game, &mut queue, answers).unwrap();
}
fn miracle_mana(game: &mut GameState, generic: u32) {
    game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless, generic);
    game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Blue, 1);
}
#[test]
fn full_body_miracle_is_first_draw_only_and_reveal_and_cast_are_independently_optional() {
    for definition in definitions() { for mode in 0..7 {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        game.turn.active_player = B; // Miracle also works on an opponent's turn.
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let mut answers = MiracleAnswers { reveal: mode != 1, cast: mode != 2, ..Default::default() };
        if mode == 3 {
            resource(&mut game, "Earlier drawn land", A, Zone::Library, "Land", "");
            let outcome = perform_draw(&mut game, source, A, 1, &mut answers);
            stack_draw(&mut game, outcome, &mut answers);
            assert!(game.stack.is_empty());
        }
        let drawer = if mode == 5 { B } else { A };
        let original = resource(&mut game, "Drawn card", drawer, Zone::Library,
            if mode == 4 { "Artifact" } else { "Enchantment" }, "{6}{U}");
        let stable = game.object(original).unwrap().stable_id;
        if mode == 6 { game.move_object_by_effect(source, Zone::Exile).unwrap(); }
        miracle_mana(&mut game, 2);
        let outcome = perform_draw(&mut game, source, drawer, 1, &mut answers);
        let arrival = game.find_object_by_stable_id(stable).unwrap();
        assert!(!game.miracle_cast_is_authorized(arrival));
        stack_draw(&mut game, outcome, &mut answers);
        let triggered = matches!(mode, 0 | 2);
        assert_eq!(game.stack.len(), usize::from(triggered), "mode {mode}");
        assert_eq!(answers.reveal_questions, usize::from(mode <= 2));
        if triggered { resolve_stack_entry_with(&mut game, &mut answers).unwrap(); }
        if mode == 0 {
            assert_eq!(game.stack.len(), 1, "the real spell was cast while its trigger resolved");
            let spell = game.object(game.stack.last().unwrap().object_id).unwrap();
            assert_eq!(spell.stable_id, stable);
            assert!(spell.cast_alternative_method.as_ref().is_some_and(|method| method.is_miracle()));
            assert_eq!(spell.mana_spent_to_cast.total(), 3);
            assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
            resolve_stack_entry_with(&mut game, &mut answers).unwrap();
            assert_eq!(game.object(game.find_object_by_stable_id(stable).unwrap()).unwrap().zone, Zone::Battlefield);
        } else {
            assert!(game.stack.is_empty());
            assert_eq!(game.object(arrival).unwrap().zone, Zone::Hand);
            assert_eq!(game.player(A).unwrap().mana_pool.total(), 3);
        }
        assert!(!game.miracle_cast_is_authorized(arrival));
    } }
}

#[test]
fn full_body_reveal_precedes_replacement_additions_and_keeps_the_departed_granters_price() {
    use ironsmith::replacement::{ReplacementAction, ReplacementEffect};
    for definition in definitions() { for pause in [false, true] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let original = resource(&mut game, "Miracle arrival", A, Zone::Library, "Enchantment", "{6}{U}");
        let stable = game.object(original).unwrap().stable_id;
        let replacement = resource(&mut game, "Draw modifier", B, Zone::Battlefield, "Artifact", "{1}");
        let shield = game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(
            replacement, B, ironsmith::events::WouldDrawCardMatcher::new(ironsmith::target::PlayerFilter::Specific(A)),
            ReplacementAction::Additionally(vec![ironsmith::Effect::move_to_zone(
                ironsmith::target::ChooseSpec::SpecificObject(source), Zone::Exile, false)]),
        ));
        miracle_mana(&mut game, 2);
        let mut answers = MiracleAnswers { reveal: true, cast: true, pause, granter_at_reveal: Some(source), ..Default::default() };
        let mut outcome = perform_draw(&mut game, source, A, 1, &mut answers);
        if pause {
            assert!(answers.pending);
            assert!(outcome.events.is_empty());
            assert_eq!(game.player(A).unwrap().library, vec![original]);
            assert!(game.player(A).unwrap().hand.is_empty());
            assert!(game.object(source).is_some());
            assert!(game.effect_store.replacement_effects.get_effect(shield).is_some());
            answers.pause = false; answers.pending = false;
            outcome = perform_draw(&mut game, source, A, 1, &mut answers);
        }
        assert!(game.object(source).is_none(), "the addition now runs after the reveal");
        stack_draw(&mut game, outcome, &mut answers);
        assert_eq!(game.stack.len(), 1);
        resolve_stack_entry_with(&mut game, &mut answers).unwrap();
        assert_eq!(game.stack.len(), 1);
        assert_eq!(game.object(game.stack.last().unwrap().object_id).unwrap().stable_id, stable);
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
    } }
}

#[test]
fn legacy_draw_action_restores_a_pending_reveal_and_replays_the_same_first_arrival() {
    for definition in definitions() {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        game.turn.turn_number = 2;
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let original = resource(&mut game, "Pending first draw", A, Zone::Library, "Enchantment", "{6}{U}");
        let stable = game.object(original).unwrap().stable_id;
        let mut answers = MiracleAnswers { reveal: true, cast: true, pause: true, granter_at_reveal: Some(source), ..Default::default() };
        let events = ironsmith::turn::execute_draw_step_with(&mut game, &mut answers).unwrap();
        assert!(answers.pending && events.is_empty());
        assert_eq!(game.player(A).unwrap().library, vec![original]);
        assert!(game.player(A).unwrap().hand.is_empty());
        answers.pause = false; answers.pending = false;
        let events = ironsmith::turn::execute_draw_step_with(&mut game, &mut answers).unwrap();
        let drawn = events.iter().find_map(|event| event.downcast::<ironsmith::events::CardsDrawnEvent>()).unwrap();
        assert_eq!(drawn.cards.len(), 1);
        let ironsmith::events::other::MiracleDrawDecision::Revealed(proof) = drawn.miracle.as_ref().unwrap() else { panic!("completed original reveal"); };
        assert_eq!(proof.stable_id, stable);
        assert_eq!(proof.card, drawn.cards[0]);
        assert_eq!(proof.instance.granting_source, source);
    }
}

struct HiddenAnswers { inner: MiracleAnswers, pause_opening: bool, open: bool, pending: bool, openings: usize }
impl DecisionMaker for HiddenAnswers {
    fn decide_objects(&mut self, _: &GameState, context: &ironsmith::decisions::context::SelectObjectsContext) -> Vec<ObjectId> {
        self.openings += 1;
        assert_eq!(context.player, A);
        assert_eq!((context.min, context.max), (0, Some(1)));
        assert_eq!(context.reveal_policy, ironsmith::decisions::context::SelectionRevealPolicy::Public);
        assert_eq!(context.candidates.len(), 1);
        if self.pause_opening { self.pending = true; return vec![]; }
        if self.open { vec![context.candidates[0].id] } else { vec![] }
    }
    fn decide_options(&mut self, game: &GameState, context: &ironsmith::decisions::context::SelectOptionsContext) -> Vec<usize> {
        self.inner.decide_options(game, context)
    }
    fn decide_boolean(&mut self, game: &GameState, context: &ironsmith::decisions::context::BooleanContext) -> bool {
        self.inner.decide_boolean(game, context)
    }
    fn awaiting_choice(&self) -> bool { self.pending || self.inner.pending }
}
#[test]
fn hidden_first_draw_opens_the_same_owner_window_before_another_draw_and_replays_authenticated_identity() {
    for definition in definitions() { for identity_known in [false, true] { for accept in [false, true] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let bottom = resource(&mut game, "Second draw", A, Zone::Library, "Enchantment", "{2}{U}");
        let original = game.create_hidden_card_placeholder(A, Zone::Library, 17, "miracle-opening-commitment".into());
        let stable = game.object(original).unwrap().stable_id;
        let opened = compile_to_runtime_definition("Authenticated enchantment", "Mana cost: {6}{U}\nType: Enchantment", false).unwrap();
        if identity_known { game.reveal_hidden_card_with_definition(original, &opened).unwrap(); }
        let mut answers = HiddenAnswers {
            inner: MiracleAnswers { reveal: accept, cast: false, ..Default::default() },
            pause_opening: true, open: accept, pending: false, openings: 0,
        };
        let outcome = perform_draw(&mut game, source, A, 2, &mut answers);
        assert!(answers.pending && outcome.events.is_empty());
        assert_eq!(answers.openings, 1);
        assert_eq!(answers.inner.reveal_questions, 0, "no private characteristic-dependent Miracle menu precedes opening");
        assert_eq!(game.player(A).unwrap().library, vec![bottom, original]);
        assert!(game.player(A).unwrap().hand.is_empty(), "the next draw cannot physically advance while the original reveal is pending");
        // The peer front end authenticates this same physical card before
        // replaying the public selection; it does not supply a guessed identity.
        if accept { game.reveal_hidden_card_with_definition(original, &opened).unwrap(); }
        answers.pause_opening = false; answers.pending = false;
        let outcome = perform_draw(&mut game, source, A, 2, &mut answers);
        let draw = outcome.events.iter().find_map(|event| event.downcast::<ironsmith::events::CardsDrawnEvent>()).unwrap();
        assert_eq!(draw.cards.len(), 2);
        assert_eq!(game.object(draw.cards[0]).unwrap().stable_id, stable);
        assert_eq!(answers.inner.reveal_questions, usize::from(accept));
        assert!(game.pending_hidden_draw_reveals().is_empty(), "this original window must not be re-opened after additions");
        stack_draw(&mut game, outcome, &mut answers);
        assert_eq!(game.stack.len(), usize::from(accept));
        if accept { resolve_stack_entry_with(&mut game, &mut answers).unwrap(); }
        assert!(game.stack.is_empty());
    } } }
}

#[test]
fn multiple_grants_and_an_intrinsic_miracle_keep_only_the_selected_linked_price() {
    for definition in definitions() { for use_intrinsic in [false, true] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let second = compile_to_runtime_definition("Second miracle granter",
            "Type: Enchantment\nEach enchantment card in your hand has miracle. Its miracle cost is equal to its mana cost reduced by {1}.", false).unwrap();
        let other_source = game.create_object_from_definition(&second, A, Zone::Battlefield);
        let drawn_definition = compile_to_runtime_definition("Multiple miracle enchantment",
            "Mana cost: {6}{U}\nType: Enchantment\nMiracle {U}", false).unwrap();
        let original = game.create_object_from_definition(&drawn_definition, A, Zone::Library);
        let stable = game.object(original).unwrap().stable_id;
        miracle_mana(&mut game, if use_intrinsic { 0 } else { 5 });
        struct SelectLinked { inner: MiracleAnswers, intrinsic: bool }
        impl DecisionMaker for SelectLinked {
            fn decide_options(&mut self, game: &GameState, context: &ironsmith::decisions::context::SelectOptionsContext) -> Vec<usize> {
                if context.description == "Choose a Miracle reveal" {
                    self.inner.reveal_questions += 1;
                    assert_eq!(context.options.len(), 4, "decline plus three distinct instances");
                    let wanted = if self.intrinsic { "Miracle ({U})" } else { "reduced by {1}" };
                    return vec![context.options.iter().find(|option| option.description.contains(wanted)).unwrap().index];
                }
                self.inner.decide_options(game, context)
            }
            fn decide_boolean(&mut self, _: &GameState, _: &ironsmith::decisions::context::BooleanContext) -> bool { true }
        }
        let mut answers = SelectLinked { inner: MiracleAnswers::default(), intrinsic: use_intrinsic };
        let outcome = perform_draw(&mut game, source, A, 1, &mut answers);
        let draw = outcome.events.iter().find_map(|event| event.downcast::<ironsmith::events::CardsDrawnEvent>()).unwrap();
        let ironsmith::events::other::MiracleDrawDecision::Revealed(proof) = draw.miracle.as_ref().unwrap() else { panic!("chosen linked receipt"); };
        assert_eq!(proof.instance.granting_source, if use_intrinsic { draw.cards[0] } else { other_source });
        game.move_object_by_effect(source, Zone::Exile).unwrap();
        game.move_object_by_effect(other_source, Zone::Exile).unwrap();
        stack_draw(&mut game, outcome, &mut answers);
        assert_eq!(game.stack.len(), 1, "unselected instances cannot create additional cast offers");
        resolve_stack_entry_with(&mut game, &mut answers).unwrap();
        assert_eq!(game.stack.len(), 1);
        let spell = game.object(game.stack.last().unwrap().object_id).unwrap();
        assert_eq!(spell.stable_id, stable);
        assert_eq!(spell.mana_spent_to_cast.total(), if use_intrinsic { 1 } else { 6 });
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
    } }
}

#[test]
fn a_new_hand_incarnation_cannot_use_the_old_draws_reveal_permission() {
    for definition in definitions() {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let original = resource(&mut game, "Returning card", A, Zone::Library, "Enchantment", "{6}{U}");
        let stable = game.object(original).unwrap().stable_id;
        let mut answers = MiracleAnswers { reveal: true, cast: true, ..Default::default() };
        let outcome = perform_draw(&mut game, source, A, 1, &mut answers);
        let arrival = game.find_object_by_stable_id(stable).unwrap();
        let exile = game.move_object_by_effect(arrival, Zone::Exile).unwrap();
        let returned = game.move_object_by_effect(exile, Zone::Hand).unwrap();
        miracle_mana(&mut game, 2);
        stack_draw(&mut game, outcome, &mut answers);
        assert_eq!(game.stack.len(), 1, "the reveal trigger already happened");
        resolve_stack_entry_with(&mut game, &mut answers).unwrap();
        assert!(game.stack.is_empty());
        assert_eq!(game.player(A).unwrap().hand, vec![returned]);
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 3);
        assert!(!game.miracle_cast_is_authorized(returned));
    }
}

#[test]
fn room_cast_selects_its_own_frozen_miracle_price_after_the_granter_leaves() {
    for definition in definitions() { for other_door in [false, true] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let mut front = compile_to_runtime_definition("Blue Door", "Mana cost: {6}{U}\nType: Enchantment — Room", false).unwrap();
        let mut back = compile_to_runtime_definition("Black Door", "Mana cost: {2}{B}\nType: Enchantment — Room", false).unwrap();
        front.card.other_face = Some(back.card.id); front.card.other_face_name = Some(back.card.name.to_string());
        front.card.linked_face_layout = ironsmith::card::LinkedFaceLayout::Split;
        back.card.other_face = Some(front.card.id); back.card.other_face_name = Some(front.card.name.to_string());
        back.card.linked_face_layout = ironsmith::card::LinkedFaceLayout::Split;
        game.register_linked_face_definition(&front); game.register_linked_face_definition(&back);
        let original = game.create_object_from_definition(&front, A, Zone::Library);
        let stable = game.object(original).unwrap().stable_id;
        struct RoomAnswers { inner: MiracleAnswers, other: bool }
        impl DecisionMaker for RoomAnswers {
            fn decide_options(&mut self, game: &GameState, context: &ironsmith::decisions::context::SelectOptionsContext) -> Vec<usize> {
                if context.description == "Choose which spell to cast" { vec![usize::from(self.other)] }
                else { self.inner.decide_options(game, context) }
            }
            fn decide_boolean(&mut self, _: &GameState, _: &ironsmith::decisions::context::BooleanContext) -> bool { true }
        }
        let mut answers = RoomAnswers { inner: MiracleAnswers { reveal: true, ..Default::default() }, other: other_door };
        let outcome = perform_draw(&mut game, source, A, 1, &mut answers);
        game.move_object_by_effect(source, Zone::Exile).unwrap();
        if other_door { game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Black, 1); }
        else { miracle_mana(&mut game, 2); }
        stack_draw(&mut game, outcome, &mut answers);
        assert_eq!(game.stack.len(), 1);
        resolve_stack_entry_with(&mut game, &mut answers).unwrap();
        assert_eq!(game.stack.len(), 1);
        let spell = game.object(game.stack.last().unwrap().object_id).unwrap();
        assert_eq!(spell.stable_id, stable);
        assert_eq!(spell.name.as_str(), if other_door { "Black Door" } else { "Blue Door" });
        assert_eq!(spell.mana_spent_to_cast.total(), if other_door { 1 } else { 3 });
        assert_eq!(spell.mana_cost_owned().unwrap().mana_value(), if other_door { 3 } else { 7 });
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
        resolve_stack_entry_with(&mut game, &mut answers).unwrap();
        let permanent = game.find_object_by_stable_id(stable).unwrap();
        assert_eq!(game.object(permanent).unwrap().zone, Zone::Battlefield);
        assert!(!game.room_has_no_unlocked_door(permanent));
    } }
}

#[test]
fn miracle_payment_pause_restores_the_trigger_and_exact_hand_card_then_replays_the_captured_price() {
    for definition in definitions() {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let original = resource(&mut game, "Paused payment enchantment", A, Zone::Library, "Enchantment", "{6}{U}");
        let stable = game.object(original).unwrap().stable_id;
        let mut draw_answers = MiracleAnswers { reveal: true, cast: true, ..Default::default() };
        let outcome = perform_draw(&mut game, source, A, 1, &mut draw_answers);
        let arrival = game.find_object_by_stable_id(stable).unwrap();
        game.move_object_by_effect(source, Zone::Exile).unwrap();
        stack_draw(&mut game, outcome, &mut draw_answers);
        miracle_mana(&mut game, 2);
        struct PaymentAnswers { pause: bool, pending: bool, payments: usize }
        impl DecisionMaker for PaymentAnswers {
            fn decide_boolean(&mut self, _: &GameState, _: &ironsmith::decisions::context::BooleanContext) -> bool { true }
            fn decide_mana_payment(&mut self, game: &GameState, context: &ironsmith::decisions::context::ManaPaymentContext) -> ironsmith::mana_payment::ManaPaymentResponse {
                self.payments += 1;
                assert_eq!(context.player, A);
                if self.pause { self.pending = true; return ironsmith::mana_payment::ManaPaymentResponse::Cancel; }
                SelectFirstDecisionMaker.decide_mana_payment(game, context)
            }
            fn awaiting_choice(&self) -> bool { self.pending }
        }
        let mut answers = PaymentAnswers { pause: true, pending: false, payments: 0 };
        resolve_stack_entry_with(&mut game, &mut answers).unwrap();
        assert!(answers.pending && answers.payments > 0);
        assert_eq!(game.stack.len(), 1);
        assert!(game.stack.last().unwrap().is_ability);
        assert_eq!(game.player(A).unwrap().hand, vec![arrival]);
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 3);
        assert!(!game.miracle_cast_is_authorized(arrival));
        answers.pause = false; answers.pending = false;
        resolve_stack_entry_with(&mut game, &mut answers).unwrap();
        assert_eq!(game.stack.len(), 1);
        assert!(!game.stack.last().unwrap().is_ability);
        assert_eq!(game.object(game.stack.last().unwrap().object_id).unwrap().stable_id, stable);
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
        assert!(!game.miracle_cast_is_authorized(arrival));
    }
}

#[test]
fn adventure_and_modal_linked_faces_keep_their_own_recipe_after_reveal() {
    for definition in definitions() { for adventure in [false, true] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let mut front = compile_to_runtime_definition("Enchantment face", "Mana cost: {6}{U}\nType: Enchantment", false).unwrap();
        let mut back = compile_to_runtime_definition("Other castable face", if adventure {
            "Mana cost: {3}{B}\nType: Sorcery — Adventure\nYou gain 1 life."
        } else { "Mana cost: {5}{G}\nType: Creature — Elf\nPower/Toughness: 2/2" }, false).unwrap();
        front.card.other_face = Some(back.card.id); front.card.other_face_name = Some(back.card.name.to_string());
        front.card.linked_face_layout = ironsmith::card::LinkedFaceLayout::TransformLike;
        front.card.transforming_dfc = false;
        back.card.other_face = Some(front.card.id); back.card.other_face_name = Some(front.card.name.to_string());
        back.card.linked_face_layout = ironsmith::card::LinkedFaceLayout::TransformLike;
        back.card.transforming_dfc = false;
        game.register_linked_face_definition(&front); game.register_linked_face_definition(&back);
        game.create_object_from_definition(&front, A, Zone::Library);
        struct BackAnswers { inner: MiracleAnswers }
        impl DecisionMaker for BackAnswers {
            fn decide_options(&mut self, game: &GameState, context: &ironsmith::decisions::context::SelectOptionsContext) -> Vec<usize> {
                if context.description == "Choose which spell to cast" { vec![1] }
                else { self.inner.decide_options(game, context) }
            }
            fn decide_boolean(&mut self, _: &GameState, _: &ironsmith::decisions::context::BooleanContext) -> bool { true }
        }
        let mut answers = BackAnswers { inner: MiracleAnswers { reveal: true, ..Default::default() } };
        let outcome = perform_draw(&mut game, source, A, 1, &mut answers);
        game.move_object_by_effect(source, Zone::Exile).unwrap();
        let color = if adventure { ironsmith::mana::ManaSymbol::Black } else { ironsmith::mana::ManaSymbol::Green };
        game.player_mut(A).unwrap().mana_pool.add(color, 1);
        if !adventure { game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless, 1); }
        stack_draw(&mut game, outcome, &mut answers);
        resolve_stack_entry_with(&mut game, &mut answers).unwrap();
        assert_eq!(game.stack.len(), 1);
        let spell = game.object(game.stack.last().unwrap().object_id).unwrap();
        assert_eq!(spell.name.as_str(), "Other castable face");
        assert_eq!(spell.mana_spent_to_cast.total(), if adventure { 1 } else { 2 });
        assert!(spell.cast_alternative_method.as_ref().is_some_and(|method| method.is_miracle()));
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
    } }
}

#[test]
fn redirected_expanded_and_turn_draws_use_the_actual_recipients_first_draw_history() {
    use ironsmith::replacement::{ReplacementAction, ReplacementEffect};
    for definition in definitions() { for turn_draw in [false, true] { for actual_already_drew in [false, true] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        game.turn.turn_number = 2;
        let source = game.create_object_from_definition(&definition, B, Zone::Battlefield);
        let prior = if actual_already_drew { B } else { A };
        resource(&mut game, "Earlier draw", prior, Zone::Library, "Land", "");
        let mut answers = MiracleAnswers { reveal: true, cast: false, drawer: Some(B), ..Default::default() };
        let earlier = perform_draw(&mut game, source, prior, 1, &mut answers);
        stack_draw(&mut game, earlier, &mut answers);
        assert!(game.stack.is_empty());
        let authored_top = resource(&mut game, "Authored recipient's card", A, Zone::Library, "Land", "");
        let actual_top = resource(&mut game, "Actual recipient's enchantment", B, Zone::Library, "Enchantment", "{6}{U}");
        let stable = game.object(actual_top).unwrap().stable_id;
        game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(
            source, B, ironsmith::events::WouldDrawCardMatcher::any_player(),
            ReplacementAction::Additionally(vec![ironsmith::Effect::gain_life(1)]),
        ));
        game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(
            source, B, ironsmith::events::WouldDrawCardMatcher::new(ironsmith::target::PlayerFilter::Specific(A)),
            ReplacementAction::RedirectDrawToController,
        ));
        if turn_draw {
            let mut runner = ironsmith::TurnRunner::from_state_for_sync(ironsmith::TurnRunnerState::Draw);
            let mut queue = TriggerQueue::new();
            loop {
                match runner.advance(&mut game, &mut queue).unwrap() {
                    ironsmith::TurnAction::Decision(ironsmith::decisions::context::DecisionContext::SelectOptions(context)) => {
                        let choice = answers.decide_options(&game, &context);
                        runner.respond_options(choice);
                    }
                    ironsmith::TurnAction::Decision(ironsmith::decisions::context::DecisionContext::Boolean(_)) => runner.respond_boolean(true),
                    ironsmith::TurnAction::Continue => {},
                    ironsmith::TurnAction::RunPriority => break,
                    other => panic!("unexpected redirected draw action: {other:?}"),
                }
            }
            put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut answers).unwrap();
        } else {
            let outcome = perform_draw(&mut game, source, A, 1, &mut answers);
            let notice = outcome.events.iter().find_map(|event| event.downcast::<ironsmith::events::CardsDrawnEvent>()).unwrap();
            assert_eq!(notice.player, B);
            assert_eq!(notice.is_first_this_turn, !actual_already_drew);
            stack_draw(&mut game, outcome, &mut answers);
        }
        assert_eq!(answers.reveal_questions, usize::from(!actual_already_drew));
        assert_eq!(game.stack.len(), usize::from(!actual_already_drew));
        assert!(game.player(A).unwrap().library.contains(&authored_top));
        assert_eq!(game.object(game.find_object_by_stable_id(stable).unwrap()).unwrap().zone, Zone::Hand);
        assert_eq!(game.player(B).unwrap().life, 21);
        if !actual_already_drew { resolve_stack_entry_with(&mut game, &mut answers).unwrap(); }
        assert!(game.stack.is_empty());
    } } }
}

#[test]
fn a_missing_hidden_opening_rolls_back_the_draw_while_a_known_non_enchantment_is_an_ordinary_nonmatch() {
    for definition in definitions() { for known_nonmatch in [false, true] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let original = game.create_hidden_card_placeholder(A, Zone::Library, 18, "required-draw-opening".into());
        if known_nonmatch {
            let card = compile_to_runtime_definition("Known artifact", "Mana cost: {6}\nType: Artifact", false).unwrap();
            game.reveal_hidden_card_with_definition(original, &card).unwrap();
        }
        let mut answers = HiddenAnswers {
            inner: MiracleAnswers { reveal: true, cast: true, ..Default::default() },
            pause_opening: false, open: true, pending: false, openings: 0,
        };
        let effect = ironsmith::Effect::new(ironsmith::effects::SequenceEffect::new(vec![
            ironsmith::Effect::gain_life(3),
            ironsmith::Effect::new(ironsmith::effects::DrawCardsEffect::you(1)),
        ]));
        let result = ironsmith::effects::execute_effect(&mut game, &effect,
            &mut ironsmith::effects::EffectContext::new(source, A, &mut answers));
        assert_eq!(answers.openings, 1);
        assert_eq!(answers.inner.reveal_questions, 0);
        if known_nonmatch {
            let outcome = result.unwrap();
            assert_eq!(game.player(A).unwrap().life, 23);
            assert!(game.player(A).unwrap().library.is_empty());
            stack_draw(&mut game, outcome, &mut answers);
            assert!(game.stack.is_empty());
        } else {
            assert!(matches!(result, Err(ironsmith::effects::ExecutionError::IncompleteEvidence(_))), "{result:?}");
            assert_eq!(game.player(A).unwrap().life, 20);
            assert_eq!(game.player(A).unwrap().library, vec![original]);
            assert!(game.player(A).unwrap().hand.is_empty());
            assert!(game.publicly_revealed_hidden_cards().is_empty());
        }
    } }
}

#[test]
fn native_own_and_foreign_miracle_trigger_copies_keep_the_revealed_card_but_use_the_copy_controller() {
    use ironsmith::effects::EffectExecutor;
    for definition in definitions() { for copier in [A, B] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let original = resource(&mut game, "Copied Miracle source", A, Zone::Library, "Enchantment", "{6}{U}");
        let stable = game.object(original).unwrap().stable_id;
        let mut answers = MiracleAnswers { reveal: true, cast: true, ..Default::default() };
        let outcome = perform_draw(&mut game, source, A, 1, &mut answers);
        stack_draw(&mut game, outcome, &mut answers);
        let ability = game.stack.last().unwrap().target_id();
        game.move_object_by_effect(source, Zone::Exile).unwrap();
        let copy_source = resource(&mut game, "Copying instruction source", copier, Zone::Battlefield, "Artifact", "{1}");
        ironsmith::effects::CopySpellEffect::single(ironsmith::target::ChooseSpec::SpecificObject(ability))
            .execute(&mut game, &mut ironsmith::effects::EffectContext::new(copy_source, copier, &mut answers)).unwrap();
        assert_eq!(game.stack.len(), 2);
        assert_eq!(game.stack.last().unwrap().controller, copier);
        game.player_mut(copier).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless, 2);
        game.player_mut(copier).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Blue, 1);
        struct CopyCaster { player: PlayerId }
        impl DecisionMaker for CopyCaster {
            fn decide_boolean(&mut self, _: &GameState, context: &ironsmith::decisions::context::BooleanContext) -> bool {
                assert_eq!(context.player, self.player); true
            }
        }
        let mut caster = CopyCaster { player: copier };
        resolve_stack_entry_with(&mut game, &mut caster).unwrap();
        assert_eq!(game.stack.len(), 2, "the copied ability cast the one revealed card above the original trigger");
        let spell = game.object(game.stack.last().unwrap().object_id).unwrap();
        assert_eq!(spell.stable_id, stable);
        assert_eq!(spell.owner, A);
        assert_eq!(game.stack.last().unwrap().controller, copier);
        assert_eq!(game.player(copier).unwrap().mana_pool.total(), 0);
        resolve_stack_entry_with(&mut game, &mut caster).unwrap();
        let permanent = game.find_object_by_stable_id(stable).unwrap();
        assert_eq!(game.current_controller(permanent), Some(copier));
        resolve_stack_entry_with(&mut game, &mut caster).unwrap();
        assert!(game.stack.is_empty(), "the original trigger cannot cast a new incarnation of that card");
    } }
}

#[derive(Default)]
struct MultipleMiracleAnswers {
    selected: Vec<usize>, cast: bool, pause_reveal: bool, pending: bool,
    reveal_questions: usize, cast_offers: Vec<PlayerId>,
}
impl DecisionMaker for MultipleMiracleAnswers {
    fn decide_options(&mut self, game: &GameState, context: &ironsmith::decisions::context::SelectOptionsContext) -> Vec<usize> {
        if context.description == "Choose a Miracle reveal" {
            self.reveal_questions += 1;
            assert_eq!(context.player, A);
            assert_eq!(context.options.len(), 4, "intrinsic plus two distinct granted instances and decline");
            assert_eq!(context.max, 3, "each Miracle instance is independently optional");
            if self.pause_reveal { self.pending = true; return vec![]; }
            return self.selected.clone();
        }
        SelectFirstDecisionMaker.decide_options(game, context)
    }
    fn decide_boolean(&mut self, _: &GameState, context: &ironsmith::decisions::context::BooleanContext) -> bool {
        self.cast_offers.push(context.player); self.cast
    }
    fn awaiting_choice(&self) -> bool { self.pending }
}
fn three_miracles(game: &mut GameState, definition: &CardDefinition) -> (ObjectId, ObjectId, ObjectId) {
    let source = game.create_object_from_definition(definition, A, Zone::Battlefield);
    let second = compile_to_runtime_definition("Independent Miracle grant",
        "Type: Enchantment\nEach enchantment card in your hand has miracle. Its miracle cost is equal to its mana cost reduced by {1}.", false).unwrap();
    let other = game.create_object_from_definition(&second, A, Zone::Battlefield);
    let card = compile_to_runtime_definition("Independent Miracle instances",
        "Mana cost: {6}{U}\nType: Enchantment\nMiracle {U}", false).unwrap();
    (source, other, game.create_object_from_definition(&card, A, Zone::Library))
}
fn linked_miracle(entry: &ironsmith::game_state::StackEntry) -> &ironsmith::events::other::RevealedMiracle {
    let event = entry.triggering_event.as_ref().unwrap().downcast::<ironsmith::events::CardsDrawnEvent>().unwrap();
    let proofs = event.miracle.as_ref().unwrap().revealed_instances();
    assert_eq!(proofs.len(), 1, "a casting trigger retains only its own linked instance"); &proofs[0]
}
fn pay_linked_miracle(game: &mut GameState, player: PlayerId, proof: &ironsmith::events::other::RevealedMiracle) -> u32 {
    let generic = match &proof.instance.price {
        ironsmith::events::other::DrawnMiraclePrice::Fixed(_) => 0,
        ironsmith::events::other::DrawnMiraclePrice::ReducedManaCost { generic_reduction, .. } => 6_u32.saturating_sub(*generic_reduction),
    };
    game.player_mut(player).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless, generic);
    game.player_mut(player).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Blue, 1);
    generic + 1
}
#[test]
fn independent_miracle_reveals_keep_each_linked_price_after_another_trigger_is_countered_or_declined() {
    for definition in definitions() { for selected in [vec![1, 2], vec![1, 2, 3]] { for counter_first in [false, true] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let (source, other, original) = three_miracles(&mut game, &definition);
        let stable = game.object(original).unwrap().stable_id;
        let mut answers = MultipleMiracleAnswers { selected: selected.clone(), ..Default::default() };
        let outcome = perform_draw(&mut game, source, A, 1, &mut answers);
        assert_eq!(outcome.events.iter().filter(|event| event.downcast::<ironsmith::events::CardRevealedEvent>().is_some()).count(), selected.len());
        let drawn = outcome.events.iter().find_map(|event| event.downcast::<ironsmith::events::CardsDrawnEvent>()).unwrap();
        assert_eq!(drawn.miracle.as_ref().unwrap().revealed_instances().len(), selected.len());
        let arrival = drawn.cards[0];
        game.move_object_by_effect(source, Zone::Exile).unwrap();
        game.move_object_by_effect(other, Zone::Exile).unwrap();
        stack_draw(&mut game, outcome, &mut answers);
        assert_eq!(game.stack.len(), selected.len());
        let remaining = game.stack[..game.stack.len() - 1].iter().map(|entry| linked_miracle(entry).clone()).collect::<Vec<_>>();
        if counter_first {
            let target = game.stack.last().unwrap().ability_id.unwrap();
            let counter_source = resource(&mut game, "Counter witness", B, Zone::Battlefield, "Artifact", "{1}");
            ironsmith::effects::execute_effect(&mut game, &ironsmith::Effect::new(
                ironsmith::effects::CounterEffect::new(ironsmith::target::ChooseSpec::SpecificObject(target))),
                &mut ironsmith::effects::EffectContext::new(counter_source, B, &mut answers)).unwrap();
            assert!(answers.cast_offers.is_empty());
        } else {
            resolve_stack_entry_with(&mut game, &mut answers).unwrap();
            assert_eq!(answers.cast_offers, vec![A]);
        }
        assert_eq!(game.player(A).unwrap().hand, vec![arrival]);
        assert_eq!(game.stack.iter().map(|entry| linked_miracle(entry).clone()).collect::<Vec<_>>(), remaining);
        game = game.clone();
        let next = linked_miracle(game.stack.last().unwrap()).clone();
        let paid = pay_linked_miracle(&mut game, A, &next); answers.cast = true;
        resolve_stack_entry_with(&mut game, &mut answers).unwrap();
        let spell = game.object(game.stack.last().unwrap().object_id).unwrap();
        assert_eq!(spell.stable_id, stable); assert_eq!(spell.mana_spent_to_cast.total(), paid);
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
        resolve_stack_entry_with(&mut game, &mut answers).unwrap();
        let offers = answers.cast_offers.len();
        while !game.stack.is_empty() { resolve_stack_entry_with(&mut game, &mut answers).unwrap(); }
        assert_eq!(answers.cast_offers.len(), offers, "remaining reveals cannot cast a later incarnation");
        assert_eq!(game.object(game.find_object_by_stable_id(stable).unwrap()).unwrap().zone, Zone::Battlefield);
        assert!(!game.miracle_cast_is_authorized(arrival));
    } } }
}
#[test]
fn multiple_miracle_selection_replays_one_original_draw_without_losing_instances_or_granters() {
    for definition in definitions() {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let (source, other, original) = three_miracles(&mut game, &definition);
        let mut answers = MultipleMiracleAnswers { selected: vec![1, 2, 3], pause_reveal: true, ..Default::default() };
        let outcome = perform_draw(&mut game, source, A, 1, &mut answers);
        assert!(answers.pending && outcome.events.is_empty());
        assert_eq!(game.player(A).unwrap().library, vec![original]); assert!(game.player(A).unwrap().hand.is_empty());
        assert!(game.object(source).is_some() && game.object(other).is_some());
        game = game.clone(); answers.pause_reveal = false; answers.pending = false;
        let outcome = perform_draw(&mut game, source, A, 1, &mut answers);
        assert_eq!(answers.reveal_questions, 2);
        stack_draw(&mut game, outcome, &mut answers); assert_eq!(game.stack.len(), 3);
        let identities = game.stack.iter().map(|entry| linked_miracle(entry).instance.identity.clone()).collect::<std::collections::HashSet<_>>();
        assert_eq!(identities.len(), 3);
        for _ in 0..3 { resolve_stack_entry_with(&mut game, &mut answers).unwrap(); }
        assert_eq!(answers.cast_offers, vec![A, A, A]); assert_eq!(answers.reveal_questions, 2);
        assert_eq!(game.player(A).unwrap().hand.len(), 1);
    }
}

#[test]
fn a_foreign_copy_keeps_its_single_accepted_miracle_instance_after_the_original_is_countered() {
    use ironsmith::effects::EffectExecutor;
    for definition in definitions() {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let (source, other, original) = three_miracles(&mut game, &definition);
        let stable = game.object(original).unwrap().stable_id;
        let mut answers = MultipleMiracleAnswers { selected: vec![1, 2, 3], cast: true, ..Default::default() };
        let outcome = perform_draw(&mut game, source, A, 1, &mut answers);
        stack_draw(&mut game, outcome, &mut answers);
        let original_trigger = game.stack.last().unwrap().target_id();
        let proof = linked_miracle(game.stack.last().unwrap()).clone();
        game.move_object_by_effect(source, Zone::Exile).unwrap(); game.move_object_by_effect(other, Zone::Exile).unwrap();
        let copier = resource(&mut game, "Foreign copier", B, Zone::Battlefield, "Artifact", "{1}");
        ironsmith::effects::CopySpellEffect::single(ironsmith::target::ChooseSpec::SpecificObject(original_trigger))
            .execute(&mut game, &mut ironsmith::effects::EffectContext::new(copier, B, &mut answers)).unwrap();
        assert_eq!(game.stack.len(), 4); assert_eq!(linked_miracle(game.stack.last().unwrap()), &proof);
        ironsmith::effects::CounterEffect::new(ironsmith::target::ChooseSpec::SpecificObject(original_trigger))
            .execute(&mut game, &mut ironsmith::effects::EffectContext::new(copier, B, &mut answers)).unwrap();
        assert_eq!(game.stack.len(), 3); assert_eq!(linked_miracle(game.stack.last().unwrap()), &proof);
        let paid = pay_linked_miracle(&mut game, B, &proof);
        resolve_stack_entry_with(&mut game, &mut answers).unwrap();
        assert_eq!(answers.cast_offers, vec![B]);
        let spell = game.object(game.stack.last().unwrap().object_id).unwrap();
        assert_eq!(spell.stable_id, stable); assert_eq!(spell.owner, A); assert_eq!(spell.mana_spent_to_cast.total(), paid);
        assert_eq!(game.stack.last().unwrap().controller, B); assert_eq!(game.player(B).unwrap().mana_pool.total(), 0);
        resolve_stack_entry_with(&mut game, &mut answers).unwrap();
        while !game.stack.is_empty() { resolve_stack_entry_with(&mut game, &mut answers).unwrap(); }
        assert_eq!(answers.cast_offers, vec![B]);
        assert_eq!(game.current_controller(game.find_object_by_stable_id(stable).unwrap()), Some(B));
    }
}
#[test]
fn malformed_multiple_miracle_selections_roll_back_the_original_draw() {
    for definition in definitions() { for selected in [vec![], vec![0, 1], vec![1, 1], vec![4]] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let (source, _, original) = three_miracles(&mut game, &definition);
        let mut answers = MultipleMiracleAnswers { selected, ..Default::default() };
        let effect = ironsmith::Effect::new(ironsmith::effects::SequenceEffect::new(vec![
            ironsmith::Effect::gain_life(3), ironsmith::Effect::draw(1),
        ]));
        let result = ironsmith::effects::execute_effect(&mut game, &effect,
            &mut ironsmith::effects::EffectContext::new(source, A, &mut answers));
        assert!(result.is_err()); assert_eq!(game.player(A).unwrap().life, 20);
        assert_eq!(game.player(A).unwrap().library, vec![original]); assert!(game.player(A).unwrap().hand.is_empty());
        assert!(game.stack.is_empty());
    } }
}

#[test]
fn hidden_multiple_miracles_share_one_authenticated_opening_and_retain_all_pre_addition_prices() {
    struct HiddenMultiple { inner: MultipleMiracleAnswers, pause: bool, pending: bool, openings: usize, granters: [ObjectId; 2] }
    impl DecisionMaker for HiddenMultiple {
        fn decide_objects(&mut self, _: &GameState, context: &ironsmith::decisions::context::SelectObjectsContext) -> Vec<ObjectId> {
            self.openings += 1; assert_eq!(context.player, A);
            assert_eq!(context.reveal_policy, ironsmith::decisions::context::SelectionRevealPolicy::Public);
            assert_eq!(context.candidates.len(), 1);
            if self.pause { self.pending = true; vec![] } else { vec![context.candidates[0].id] }
        }
        fn decide_options(&mut self, game: &GameState, context: &ironsmith::decisions::context::SelectOptionsContext) -> Vec<usize> {
            if context.description == "Choose a Miracle reveal" { for source in self.granters { assert!(game.object(source).is_some(), "original arrival precedes additions"); } }
            self.inner.decide_options(game, context)
        }
        fn decide_boolean(&mut self, game: &GameState, context: &ironsmith::decisions::context::BooleanContext) -> bool { self.inner.decide_boolean(game, context) }
        fn awaiting_choice(&self) -> bool { self.pending || self.inner.pending }
    }
    for definition in definitions() { for owner_knows in [false, true] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let (source, other, unused) = three_miracles(&mut game, &definition);
        game.move_object_by_effect(unused, Zone::Exile).unwrap();
        let original = game.create_hidden_card_placeholder(A, Zone::Library, 24, "independent-miracle-opening".into());
        let stable = game.object(original).unwrap().stable_id;
        let opened = compile_to_runtime_definition("Authenticated multiple Miracle", "Mana cost: {6}{U}\nType: Enchantment\nMiracle {U}", false).unwrap();
        if owner_knows { game.reveal_hidden_card_with_definition(original, &opened).unwrap(); }
        let replacement_source = resource(&mut game, "Draw addition", B, Zone::Battlefield, "Artifact", "{1}");
        game.effect_store.replacement_effects.add_one_shot_effect(ironsmith::replacement::ReplacementEffect::with_matcher(
            replacement_source, B, ironsmith::events::WouldDrawCardMatcher::new(ironsmith::target::PlayerFilter::Specific(A)),
            ironsmith::replacement::ReplacementAction::Additionally(vec![ironsmith::Effect::move_to_zone(
                ironsmith::target::ChooseSpec::SpecificObject(other), Zone::Exile, false)]),
        ));
        let mut answers = HiddenMultiple { inner: MultipleMiracleAnswers { selected: vec![1, 2, 3], ..Default::default() },
            pause: true, pending: false, openings: 0, granters: [source, other] };
        let outcome = perform_draw(&mut game, source, A, 1, &mut answers);
        assert!(outcome.events.is_empty() && answers.pending); assert_eq!(answers.inner.reveal_questions, 0);
        assert_eq!(game.player(A).unwrap().library, vec![original]); assert!(game.object(other).is_some());
        game.reveal_hidden_card_with_definition(original, &opened).unwrap(); answers.pause = false; answers.pending = false;
        let outcome = perform_draw(&mut game, source, A, 1, &mut answers);
        assert_eq!(answers.openings, 2); assert_eq!(answers.inner.reveal_questions, 1);
        assert!(game.object(other).is_none());
        let arrival = game.find_object_by_stable_id(stable).unwrap(); assert!(game.is_publicly_revealed_hidden_card(arrival));
        stack_draw(&mut game, outcome, &mut answers); assert_eq!(game.stack.len(), 3);
        for remaining in (0..3).rev() {
            resolve_stack_entry_with(&mut game, &mut answers).unwrap(); assert_eq!(game.stack.len(), remaining);
            assert!(game.is_publicly_revealed_hidden_card(arrival), "ending semantic inspection does not erase authenticated knowledge");
        }
        assert_eq!(answers.inner.cast_offers, vec![A, A, A]); assert!(game.pending_hidden_draw_reveals().is_empty());
    } }
}

#[test]
fn declining_all_three_miracle_instances_creates_no_reveal_event_or_cast_offer() {
    for definition in definitions() {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let (source, _, original) = three_miracles(&mut game, &definition);
        let stable = game.object(original).unwrap().stable_id;
        let mut answers = MultipleMiracleAnswers { selected: vec![0], cast: true, ..Default::default() };
        let outcome = perform_draw(&mut game, source, A, 1, &mut answers);
        let draw = outcome.events.iter().find_map(|event| event.downcast::<ironsmith::events::CardsDrawnEvent>()).unwrap();
        assert!(matches!(draw.miracle, Some(ironsmith::events::other::MiracleDrawDecision::Declined)));
        assert!(outcome.events.iter().all(|event| event.downcast::<ironsmith::events::CardRevealedEvent>().is_none()));
        stack_draw(&mut game, outcome, &mut answers);
        assert!(game.stack.is_empty() && answers.cast_offers.is_empty());
        assert_eq!(game.object(game.find_object_by_stable_id(stable).unwrap()).unwrap().zone, Zone::Hand);
        assert_eq!(answers.reveal_questions, 1);
    }
}
