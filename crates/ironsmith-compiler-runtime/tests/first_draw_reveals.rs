//! Complete frozen bodies. Source-authored scenarios only; execution deferred.
use ironsmith::ability::AbilityKind;
use ironsmith::cards::CardDefinition;
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};

fn rows() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!("../../../fixtures/first_draw_reveals.json.fixture")).unwrap()
}
fn definitions(name: &str) -> [CardDefinition; 2] {
    let row = rows().into_iter().find(|row| row["name"] == name).unwrap();
    let mut text = format!("Mana cost: {}\nType: {}\n", row["mana_cost"].as_str().unwrap(), row["type_line"].as_str().unwrap());
    if let (Some(p), Some(t)) = (row["power"].as_str(), row["toughness"].as_str()) {
        text.push_str(&format!("Power/Toughness: {p}/{t}\n"));
    }
    text.push_str(row["oracle_text"].as_str().unwrap());
    let (direct, direct_loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_runtime_definition(name, &text, false));
    let direct = direct.unwrap_or_else(|error| panic!("direct {name}: {error}"));
    assert!(!direct_loss.is_lossy(), "direct {name}: {}", direct_loss.reasons_text());
    let (compiled, artifact_loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_artifact(name, &text, false));
    let (artifact, _) = compiled.unwrap_or_else(|error| panic!("artifact {name}: {error}"));
    assert!(!artifact_loss.is_lossy(), "artifact {name}: {}", artifact_loss.reasons_text());
    artifact.validate().unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, restored);
    [direct, ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap()]
}

#[test]
fn five_frozen_complete_bodies_reach_both_compilers_without_discarding_secondary_abilities() {
    for row in rows() {
        let name = row["name"].as_str().unwrap();
        for definition in definitions(name) {
            assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
            let statics: Vec<_> = definition.abilities.iter().filter_map(|ability| match &ability.kind {
                AbilityKind::Static(ability) => Some(ability), _ => None,
            }).collect();
            let reveals: Vec<_> = statics.iter().filter_map(|ability| ability.reveal_drawn_card_spec()).collect();
            assert_eq!(reveals.len(), 1);
            assert_eq!(reveals[0].optional, matches!(name, "God-Eternal Kefnet" | "Inquisitor Eisenhorn"));
            assert_eq!(reveals[0].your_turns_only, name == "Keranos, God of Storms");
            let triggered = definition.abilities.iter().filter(|ability| matches!(ability.kind, AbilityKind::Triggered(_))).count();
            assert_eq!(triggered, if matches!(name, "God-Eternal Kefnet" | "Inquisitor Eisenhorn" | "Keranos, God of Storms") { 2 } else { 1 });
            if name == "God-Eternal Kefnet" {
                assert!(statics.iter().any(|ability| ability.id() == ironsmith::static_abilities::StaticAbilityId::Flying));
            }
            if name == "Keranos, God of Storms" {
                assert!(statics.iter().any(|ability| ability.id() == ironsmith::static_abilities::StaticAbilityId::Indestructible));
                assert!(statics.len() >= 3, "devotion, indestructible and reveal all survive");
            }
        }
    }
}

#[test]
fn five_first_draw_definition_pairs_and_trigger_occurrences_survive_unrelated_card_allocations() {
    fn identities(definition: &CardDefinition) -> (Vec<ironsmith_core::LinkedExilePair>, Vec<Option<ironsmith_core::LinkedExileDefinition>>) {
        let pairs = definition.abilities.iter().filter_map(|ability| match &ability.kind {
            AbilityKind::Static(ability) => ability.reveal_drawn_card_spec().and_then(|spec| spec.linked_reveal_pair),
            _ => None,
        }).collect::<Vec<_>>();
        let triggers = definition.abilities.iter().filter_map(|ability| match &ability.kind {
            AbilityKind::Triggered(ability) => Some(ability.effects.retained_trigger_definition()),
            _ => None,
        }).collect::<Vec<_>>();
        (pairs, triggers)
    }
    for row in rows() {
        let name = row["name"].as_str().unwrap();
        let [direct, artifact] = definitions(name);
        let expected = identities(&direct);
        assert_eq!(expected.0.len(), 1);
        assert!(expected.1.iter().all(Option::is_some));
        assert_eq!(identities(&artifact), expected, "direct and artifact construction use one authored namespace: {name}");
        for _ in 0..17 { let _ = ironsmith::CardId::new(); }
        for recompiled in definitions(name) {
            assert_eq!(identities(&recompiled), expected, "including Eisenhorn's nested Cherubael definition: {name}");
        }
    }
}

use ironsmith::decision::{DecisionMaker, SelectFirstDecisionMaker};
use ironsmith::effects::{EffectExecutor, EffectContext as ExecutionContext};
use ironsmith::game_loop::{put_triggers_on_stack_with_dm, resolve_stack_entry_with};
use ironsmith::triggers::{check_triggers, TriggerEvent, TriggerQueue};
use ironsmith::{GameState, ObjectId, PlayerId, Target, Zone};
const A: PlayerId = PlayerId::from_index(0);
const B: PlayerId = PlayerId::from_index(1);
fn game() -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.phase = ironsmith::game_state::Phase::FirstMain;
    game.turn.step = None;
    game
}
fn resource(game: &mut GameState, owner: PlayerId, zone: Zone, kind: &str, cost: &str) -> ObjectId {
    let text = if cost.is_empty() { format!("Type: {kind}") } else { format!("Type: {kind}\nMana cost: {cost}") };
    let definition = compile_to_runtime_definition("Draw witness", text, false).unwrap();
    game.create_object_from_definition(&definition, owner, zone)
}
#[derive(Default)]
struct Answers {
    decline_source: Option<ObjectId>, pause_reveal: bool, pause_payment: bool, pending: bool,
    cast: bool, reveal_questions: usize, hand_sizes_at_reveal: Vec<usize>,
}
impl DecisionMaker for Answers {
    fn decide_boolean(&mut self, game: &GameState, context: &ironsmith::decisions::context::BooleanContext) -> bool {
        if context.description == "reveal the first card you draw" {
            self.reveal_questions += 1;
            self.hand_sizes_at_reveal.push(game.player(context.player).unwrap().hand.len());
            if self.pause_reveal { self.pending = true; return false; }
            return context.source != self.decline_source;
        }
        self.cast
    }
    fn decide_options(&mut self, game: &GameState, context: &ironsmith::decisions::context::SelectOptionsContext) -> Vec<usize> {
        if context.description == "Choose a Miracle reveal" { return vec![0]; }
        SelectFirstDecisionMaker.decide_options(game, context)
    }
    fn decide_objects(&mut self, game: &GameState, context: &ironsmith::decisions::context::SelectObjectsContext) -> Vec<ObjectId> {
        SelectFirstDecisionMaker.decide_objects(game, context)
    }
    fn decide_targets(&mut self, game: &GameState, context: &ironsmith::decisions::context::TargetsContext) -> Vec<Target> {
        if context.requirements.len() == 1 && context.requirements[0].legal_targets.contains(&Target::Player(B)) {
            vec![Target::Player(B)]
        } else { SelectFirstDecisionMaker.decide_targets(game, context) }
    }
    fn decide_mana_payment(&mut self, _: &GameState, context: &ironsmith::decisions::context::ManaPaymentContext) -> ironsmith::mana_payment::ManaPaymentResponse {
        if self.pause_payment { self.pending = true; return ironsmith::mana_payment::ManaPaymentResponse::Cancel; }
        ironsmith::mana_payment::ManaPaymentResponse::Confirm { plan_id: context.plan.id, request_hash: context.plan.request_hash }
    }
    fn awaiting_choice(&self) -> bool { self.pending }
}
fn draw(game: &mut GameState, source: ObjectId, player: PlayerId, count: u32, answers: &mut impl DecisionMaker) -> ironsmith::effect::EffectOutcome {
    ironsmith::effects::DrawCardsEffect::new(count, ironsmith::target::PlayerFilter::Specific(player))
        .execute(game, &mut ExecutionContext::new(source, player, answers)).unwrap()
}
fn queue(game: &mut GameState, outcome: ironsmith::effect::EffectOutcome, answers: &mut impl DecisionMaker) {
    for event in outcome.events { game.queue_trigger_event(event.provenance(), event); }
    let mut triggers = TriggerQueue::new();
    ironsmith::game_loop::check_and_apply_sbas(game, &mut triggers).unwrap();
    put_triggers_on_stack_with_dm(game, &mut triggers, answers).unwrap();
}
fn reveal_events(outcome: &ironsmith::effect::EffectOutcome) -> Vec<&ironsmith::events::CardRevealedEvent> {
    outcome.events.iter().filter_map(|event| event.downcast::<ironsmith::events::CardRevealedEvent>())
        .filter(|event| event.first_draw.is_some()).collect()
}

#[test]
fn creature_and_basic_land_predicates_only_grant_one_extra_draw_on_the_first_actual_draw() {
    for name in ["Primitive Etchings", "Rowen"] { for definition in definitions(name) { for qualifies in [false, true] {
        let mut game = game();
        game.turn.active_player = B; // Both statics apply on any player's turn.
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        resource(&mut game, A, Zone::Library, "Land", "");
        let kind = if name == "Primitive Etchings" {
            if qualifies { "Creature — Bear" } else { "Instant" }
        } else if qualifies { "Basic Land — Forest" } else { "Land — Forest" };
        resource(&mut game, A, Zone::Library, kind, "");
        let mut answers = Answers::default();
        let outcome = draw(&mut game, source, A, 1, &mut answers);
        assert_eq!(reveal_events(&outcome).len(), 1);
        queue(&mut game, outcome, &mut answers);
        assert_eq!(game.stack.len(), usize::from(qualifies));
        if qualifies { resolve_stack_entry_with(&mut game, &mut answers).unwrap(); }
        assert_eq!(game.player(A).unwrap().hand.len(), if qualifies { 2 } else { 1 });
        assert!(game.stack.is_empty());
        resource(&mut game, A, Zone::Library, kind, "");
        let later = draw(&mut game, source, A, 1, &mut answers);
        assert!(reveal_events(&later).is_empty());
    } } }
}

#[test]
fn keranos_own_turn_predicate_and_both_linked_bodies_survive_devotion_changes() {
    for definition in definitions("Keranos, God of Storms") { for own_turn in [false, true] { for land in [false, true] {
        let mut game = game(); game.turn.active_player = if own_turn { A } else { B };
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        game.refresh_continuous_state().unwrap();
        assert!(!game.current_is_creature(source));
        resource(&mut game, A, Zone::Battlefield, "Enchantment", "{U}{U}{U}{R}{R}");
        game.refresh_continuous_state().unwrap();
        assert!(game.current_is_creature(source), "combined devotion reaches seven");
        resource(&mut game, A, Zone::Library, "Land", "");
        resource(&mut game, A, Zone::Library, if land { "Land" } else { "Instant" }, "");
        let mut answers = Answers::default();
        let outcome = draw(&mut game, source, A, 1, &mut answers);
        assert_eq!(reveal_events(&outcome).len(), usize::from(own_turn));
        queue(&mut game, outcome, &mut answers);
        assert_eq!(game.stack.len(), usize::from(own_turn));
        if own_turn { resolve_stack_entry_with(&mut game, &mut answers).unwrap(); }
        assert_eq!(game.player(A).unwrap().hand.len(), if own_turn && land { 2 } else { 1 });
        assert_eq!(game.player(B).unwrap().life, if own_turn && !land { 17 } else { 20 });
    } } }
}

#[test]
fn eisenhorn_reveal_is_optional_and_creates_the_full_legendary_flying_demon() {
    for definition in definitions("Inquisitor Eisenhorn") { for accept in [false, true] { for instant in [false, true] {
        let mut game = game(); game.turn.active_player = B;
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        resource(&mut game, A, Zone::Library, if instant { "Instant" } else { "Land" }, "");
        let mut answers = Answers { decline_source: (!accept).then_some(source), ..Default::default() };
        let outcome = draw(&mut game, source, A, 1, &mut answers);
        assert_eq!(reveal_events(&outcome).len(), usize::from(accept));
        queue(&mut game, outcome, &mut answers);
        assert_eq!(game.stack.len(), usize::from(accept && instant));
        if accept && instant {
            resolve_stack_entry_with(&mut game, &mut answers).unwrap();
            let token = game.battlefield.iter().filter_map(|id| game.object(*id)).find(|object| object.name == "Cherubael").unwrap();
            assert_eq!((token.power(), token.toughness()), (Some(4), Some(4)));
            assert!(token.has_supertype(ironsmith::types::Supertype::Legendary));
            assert!(token.subtypes.contains(&ironsmith::types::Subtype::Demon));
            assert!(token.colors().contains(ironsmith::color::Color::Black));
            assert!(token.abilities.iter().any(|ability| matches!(&ability.kind, AbilityKind::Static(ability)
                if ability.id() == ironsmith::static_abilities::StaticAbilityId::Flying)));
        }
    } } }
}

#[test]
fn eisenhorn_combat_body_investigates_the_damage_amount_and_rejects_noncombat() {
    for definition in definitions("Inquisitor Eisenhorn") { for combat in [false, true] {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let event = TriggerEvent::new_with_provenance(ironsmith::events::DamageEvent::with_cause(
            source, ironsmith::events::DamageTarget::Player(B), 3, combat,
            if combat { ironsmith::events::cause::EventCause::combat_damage(source) } else { ironsmith::events::cause::EventCause::effect() },
        ), Default::default());
        let mut triggers = TriggerQueue::new();
        for entry in check_triggers(&game, &event) { triggers.add(entry); }
        let mut answers = Answers::default();
        put_triggers_on_stack_with_dm(&mut game, &mut triggers, &mut answers).unwrap();
        assert_eq!(game.stack.len(), usize::from(combat));
        if combat { resolve_stack_entry_with(&mut game, &mut answers).unwrap(); }
        assert_eq!(game.battlefield.iter().filter_map(|id| game.object(*id)).filter(|object|
            object.subtypes.contains(&ironsmith::types::Subtype::Clue)).count(), if combat { 3 } else { 0 });
        for clue in game.battlefield.iter().filter_map(|id| game.object(*id)).filter(|object|
            object.subtypes.contains(&ironsmith::types::Subtype::Clue)) {
            assert_eq!(clue.name, "Clue Token");
        }
    } }
}

#[test]
fn kefnet_copy_cost_payment_pending_and_departed_card_use_the_exact_drawn_definition() {
    for definition in definitions("God-Eternal Kefnet") { for leave_hand in [false, true] { for pause_payment in [false, true] {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let spell = compile_to_runtime_definition("Retained draw spell", "Mana cost: {3}{U}\nType: Instant\nDraw a card.", false).unwrap();
        let library_card = game.create_object_from_definition(&spell, A, Zone::Library);
        let stable = game.object(library_card).unwrap().stable_id;
        let mut answers = Answers { cast: true, ..Default::default() };
        let outcome = draw(&mut game, source, A, 1, &mut answers);
        let arrival = game.find_object_by_stable_id(stable).unwrap();
        queue(&mut game, outcome, &mut answers);
        assert_eq!(game.stack.len(), 1);
        if leave_hand {
            let departed = game.move_object_by_effect(arrival, Zone::Graveyard).unwrap();
            let decoy = compile_to_runtime_definition("Later incarnation decoy", "Mana cost: {9}{R}\nType: Sorcery\nYou gain 9 life.", false).unwrap();
            game.object_mut(departed).unwrap().apply_card_definition(&decoy);
        }
        game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Blue, 2);
        answers.pause_payment = pause_payment;
        resolve_stack_entry_with(&mut game, &mut answers).unwrap();
        if pause_payment {
            assert!(answers.pending);
            assert_eq!(game.stack.len(), 1, "the original trigger remains unresolved");
            assert_eq!(game.player(A).unwrap().mana_pool.total(), 2);
            answers.pause_payment = false; answers.pending = false;
            game = game.clone(); // Full native recovery retains the draw receipt.
            resolve_stack_entry_with(&mut game, &mut answers).unwrap();
        }
        let copy = game.object(game.stack.last().unwrap().object_id).unwrap();
        assert_eq!(copy.name, "Retained draw spell");
        assert_ne!(copy.stable_id, stable);
        assert_eq!(copy.mana_spent_to_cast.total(), 2);
        assert!(copy.spell_effect.is_some(), "the complete copied spell body survives LKI");
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
        assert_eq!(game.object(game.find_object_by_stable_id(stable).unwrap()).unwrap().zone,
            if leave_hand { Zone::Graveyard } else { Zone::Hand });
        resource(&mut game, A, Zone::Library, "Land", "");
        let hand_before_copy = game.player(A).unwrap().hand.len();
        resolve_stack_entry_with(&mut game, &mut answers).unwrap();
        assert!(game.stack.is_empty());
        assert_eq!(game.player(A).unwrap().hand.len(), hand_before_copy + 1,
            "the retained copied Draw a card body actually executes");
        assert_eq!(game.player(A).unwrap().life, 20, "the later decoy body's gain-life instruction never runs");
    } } }
}

#[test]
fn kefnet_dies_or_battlefield_exile_body_returns_the_same_incarnation_third_from_top() {
    for definition in definitions("God-Eternal Kefnet") { for zone in [Zone::Graveyard, Zone::Exile] { for accept in [false, true] {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let stable = game.object(source).unwrap().stable_id;
        for _ in 0..4 { resource(&mut game, A, Zone::Library, "Land", ""); }
        game.move_object_by_effect(source, zone).unwrap();
        let mut triggers = TriggerQueue::new();
        ironsmith::game_loop::drain_pending_trigger_events(&mut game, &mut triggers);
        let mut answers = Answers { cast: accept, ..Default::default() };
        put_triggers_on_stack_with_dm(&mut game, &mut triggers, &mut answers).unwrap();
        assert_eq!(game.stack.len(), 1);
        resolve_stack_entry_with(&mut game, &mut answers).unwrap();
        let library = &game.player(A).unwrap().library;
        assert_eq!(library.len(), if accept { 5 } else { 4 });
        if accept {
            assert_eq!(game.object(library[library.len() - 3]).unwrap().stable_id, stable);
        } else {
            assert_eq!(game.object(game.find_object_by_stable_id(stable).unwrap()).unwrap().zone, zone);
        }
    } } }
}

#[test]
fn independent_sources_and_two_groups_on_one_host_do_not_cross_trigger() {
    for definition in definitions("Inquisitor Eisenhorn") {
        let mut game = game();
        let first = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let second = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        resource(&mut game, A, Zone::Library, "Instant", "{U}");
        let mut answers = Answers { decline_source: Some(second), ..Default::default() };
        let outcome = draw(&mut game, first, A, 1, &mut answers);
        assert_eq!(answers.reveal_questions, 2);
        assert_eq!(reveal_events(&outcome).len(), 1);
        assert_eq!(outcome.events.iter().flat_map(|event| check_triggers(&game, event)).count(), 1);
    }
    let text = "Type: Enchantment\nReveal the first card you draw each turn. Whenever you reveal an instant card this way, you gain 1 life.\nReveal the first card you draw each turn. Whenever you reveal an instant card this way, you gain 2 life.";
    let definition = compile_to_runtime_definition("Two authored groups", text, false).unwrap();
    let mut game = game();
    let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
    resource(&mut game, A, Zone::Library, "Instant", "{U}");
    let mut answers = Answers::default();
    let outcome = draw(&mut game, source, A, 1, &mut answers);
    assert_eq!(reveal_events(&outcome).len(), 2);
    for event in outcome.events.iter().filter(|event| event.downcast::<ironsmith::events::CardRevealedEvent>().is_some()) {
        assert_eq!(check_triggers(&game, event).len(), 1, "each reveal belongs to one authored group");
    }
    queue(&mut game, outcome, &mut answers);
    resolve_stack_entry_with(&mut game, &mut answers).unwrap();
    resolve_stack_entry_with(&mut game, &mut answers).unwrap();
    assert_eq!(game.player(A).unwrap().life, 23);
}

#[test]
fn pending_first_draw_rolls_back_later_draws_and_replacement_additions_then_recovers() {
    use ironsmith::replacement::{ReplacementAction, ReplacementEffect};
    for definition in definitions("Inquisitor Eisenhorn") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let bottom = resource(&mut game, A, Zone::Library, "Land", "");
        let top = resource(&mut game, A, Zone::Library, "Instant", "{U}");
        let replacement = resource(&mut game, B, Zone::Battlefield, "Artifact", "{1}");
        let shield = game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(
            replacement, B, ironsmith::events::WouldDrawCardMatcher::new(ironsmith::target::PlayerFilter::Specific(A)),
            ReplacementAction::Additionally(vec![ironsmith::Effect::move_to_zone(
                ironsmith::target::ChooseSpec::SpecificObject(source), Zone::Exile, false)]),
        ));
        let mut answers = Answers { pause_reveal: true, ..Default::default() };
        let outcome = draw(&mut game, source, A, 2, &mut answers);
        assert!(answers.pending && outcome.events.is_empty());
        assert_eq!(answers.hand_sizes_at_reveal, vec![1]);
        assert_eq!(game.player(A).unwrap().library, vec![bottom, top]);
        assert!(game.player(A).unwrap().hand.is_empty());
        assert!(game.object(source).is_some());
        assert!(game.effect_store.replacement_effects.get_effect(shield).is_some());
        answers.pause_reveal = false; answers.pending = false;
        game = game.clone();
        let outcome = draw(&mut game, source, A, 2, &mut answers);
        assert!(game.object(source).is_none());
        assert_eq!(game.player(A).unwrap().hand.len(), 2);
        queue(&mut game, outcome, &mut answers);
        assert_eq!(game.stack.len(), 1, "the original reveal source survives by its captured rules acquisition");
        resolve_stack_entry_with(&mut game, &mut answers).unwrap();
        assert!(game.battlefield.iter().filter_map(|id| game.object(*id)).any(|object| object.name == "Cherubael"));
    }
}

#[test]
fn token_failure_restores_eisenhorn_trigger_and_retries_without_repeating_the_draw() {
    for definition in definitions("Inquisitor Eisenhorn") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        resource(&mut game, A, Zone::Library, "Instant", "{U}");
        let mut answers = Answers::default();
        let outcome = draw(&mut game, source, A, 1, &mut answers);
        queue(&mut game, outcome, &mut answers);
        game.set_token_creation_limits(ironsmith::effects::tokens::TokenCreationLimits { max_created_tokens: 0, ..Default::default() });
        assert!(resolve_stack_entry_with(&mut game, &mut answers).is_err());
        assert_eq!(game.stack.len(), 1);
        assert_eq!(game.player(A).unwrap().hand.len(), 1);
        game.set_token_creation_limits(Default::default());
        resolve_stack_entry_with(&mut game, &mut answers).unwrap();
        assert_eq!(game.player(A).unwrap().hand.len(), 1);
        assert!(game.stack.is_empty());
    }
}

struct HiddenAnswers { inner: Answers, pause: bool, open: bool, pending: bool, openings: usize }
impl DecisionMaker for HiddenAnswers {
    fn decide_objects(&mut self, _: &GameState, context: &ironsmith::decisions::context::SelectObjectsContext) -> Vec<ObjectId> {
        self.openings += 1;
        assert_eq!(context.player, A);
        assert_eq!(context.reveal_policy, ironsmith::decisions::context::SelectionRevealPolicy::Public);
        assert_eq!(context.candidates.len(), 1);
        if self.pause { self.pending = true; return vec![]; }
        if self.open { vec![context.candidates[0].id] } else { vec![] }
    }
    fn decide_boolean(&mut self, game: &GameState, context: &ironsmith::decisions::context::BooleanContext) -> bool {
        self.inner.decide_boolean(game, context)
    }
    fn decide_options(&mut self, game: &GameState, context: &ironsmith::decisions::context::SelectOptionsContext) -> Vec<usize> {
        self.inner.decide_options(game, context)
    }
    fn awaiting_choice(&self) -> bool { self.pending || self.inner.pending }
}
#[test]
fn private_first_draw_requires_authenticated_opening_and_keeps_other_reveals_independent() {
    for definition in definitions("Inquisitor Eisenhorn") { for known in [false, true] { for open in [false, true] {
        let mut game = game();
        let first = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let second = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let bottom = resource(&mut game, A, Zone::Library, "Land", "");
        let original = game.create_hidden_card_placeholder(A, Zone::Library, 37, "first-draw-authenticated-opening".into());
        let opened = compile_to_runtime_definition("Authenticated instant", "Mana cost: {U}\nType: Instant\nDraw a card.", false).unwrap();
        if known { game.reveal_hidden_card_with_definition(original, &opened).unwrap(); }
        let mut answers = HiddenAnswers { inner: Answers { decline_source: Some(second), ..Default::default() },
            pause: true, open, pending: false, openings: 0 };
        let outcome = draw(&mut game, first, A, 2, &mut answers);
        assert!(answers.pending && outcome.events.is_empty());
        assert_eq!(game.player(A).unwrap().library, vec![bottom, original]);
        assert!(game.player(A).unwrap().hand.is_empty());
        if open { game.reveal_hidden_card_with_definition(original, &opened).unwrap(); }
        answers.pause = false; answers.pending = false;
        let outcome = draw(&mut game, first, A, 2, &mut answers);
        assert_eq!(reveal_events(&outcome).len(), usize::from(open));
        assert_eq!(outcome.events.iter().flat_map(|event| check_triggers(&game, event)).count(), usize::from(open));
        if open { assert_eq!(answers.inner.reveal_questions, 1, "the second source gets its own optional choice after public opening"); }
        assert!(game.pending_hidden_automatic_draw_reveals().is_empty());
    } } }
}

#[test]
fn redirected_and_prevented_draws_use_the_actual_recipient_and_do_not_spend_the_first_draw() {
    use ironsmith::replacement::{ReplacementAction, ReplacementEffect};
    for definition in definitions("Primitive Etchings") { for already_drew in [false, true] {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, B, Zone::Battlefield);
        let mut answers = Answers::default();
        if already_drew {
            resource(&mut game, B, Zone::Library, "Land", "");
            let earlier = draw(&mut game, source, B, 1, &mut answers);
            queue(&mut game, earlier, &mut answers);
        }
        let original = resource(&mut game, A, Zone::Library, "Creature — Bear", "{1}{G}");
        resource(&mut game, B, Zone::Library, "Creature — Bear", "{1}{G}");
        game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(
            source, B, ironsmith::events::WouldDrawCardMatcher::new(ironsmith::target::PlayerFilter::Specific(A)),
            ReplacementAction::Prevent,
        ));
        let prevented = draw(&mut game, source, A, 1, &mut answers);
        assert!(reveal_events(&prevented).is_empty());
        assert_eq!(game.player(A).unwrap().library, vec![original]);
        game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(
            source, B, ironsmith::events::WouldDrawCardMatcher::new(ironsmith::target::PlayerFilter::Specific(A)),
            ReplacementAction::RedirectDrawToController,
        ));
        let redirected = draw(&mut game, source, A, 1, &mut answers);
        assert_eq!(reveal_events(&redirected).len(), usize::from(!already_drew));
        assert!(reveal_events(&redirected).iter().all(|event| event.player == B));
        assert_eq!(game.player(A).unwrap().library, vec![original]);
        assert_eq!(redirected.events.iter().flat_map(|event| check_triggers(&game, event)).count(), usize::from(!already_drew));
    } }
}

#[test]
fn turn_runner_first_reveal_is_pending_inside_the_draw_and_resumes_once() {
    for definition in definitions("Inquisitor Eisenhorn") {
        let mut game = game(); game.turn.turn_number = 2;
        game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let original = resource(&mut game, A, Zone::Library, "Instant", "{U}");
        let mut runner = ironsmith::TurnRunner::from_state_for_sync(ironsmith::TurnRunnerState::Draw);
        let mut triggers = TriggerQueue::new();
        let action = runner.advance(&mut game, &mut triggers).unwrap();
        assert!(matches!(action, ironsmith::TurnAction::Decision(ironsmith::decisions::context::DecisionContext::Boolean(_))));
        assert_eq!(game.player(A).unwrap().library, vec![original]);
        assert!(game.player(A).unwrap().hand.is_empty());
        runner = runner.clone(); game = game.clone();
        runner.respond_boolean(true);
        assert!(matches!(runner.advance(&mut game, &mut triggers).unwrap(), ironsmith::TurnAction::RunPriority));
        assert_eq!(game.player(A).unwrap().hand.len(), 1);
        assert_eq!(triggers.entries.len(), 1);
    }
}

#[test]
fn native_link_pair_requires_exact_host_group_acquisition_and_complete_history() {
    use ironsmith::ability::Ability;
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::snapshot::ObjectSnapshot;
    use ironsmith::static_abilities::{RevealFirstCardYouDrawEachTurn, StaticAbility};
    use ironsmith::target::{ObjectFilter, PlayerFilter};
    let pair = ironsmith_core::LinkedExilePair { definition: ironsmith_core::LinkedExileDefinition([72; 32]), pair: 0 };
    let mut reveal = RevealFirstCardYouDrawEachTurn::new(false, false);
    reveal.linked_reveal_pair = Some(pair);
    let mut trigger = ironsmith::triggers::PlayerRevealsCardTrigger::new(PlayerFilter::You, ObjectFilter::creature(), true);
    trigger.first_draw_pair = Some(pair);
    let definition = CardDefinitionBuilder::new(ironsmith::CardId::new(), "Native reveal contract")
        .card_types(vec![ironsmith::CardType::Enchantment])
        .with_ability(Ability::static_ability(StaticAbility::new(reveal)))
        .with_ability(Ability::triggered(ironsmith::triggers::Trigger::new(trigger), vec![ironsmith::Effect::gain_life(3)]))
        .build();
    let mut game = game();
    let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
    resource(&mut game, A, Zone::Library, "Creature — Bear", "{1}{G}");
    let outcome = draw(&mut game, source, A, 1, &mut Answers::default());
    let event = outcome.events.iter().find(|event| event.downcast::<ironsmith::events::CardRevealedEvent>().is_some()).unwrap();
    assert_eq!(check_triggers(&game, event).len(), 1);
    let source_snapshot = event.source_snapshot().unwrap().clone();
    let receipt = event.downcast::<ironsmith::events::CardRevealedEvent>().unwrap().clone();
    let rebuild = |receipt: ironsmith::events::CardRevealedEvent, source: ObjectSnapshot| {
        TriggerEvent::new_with_provenance(receipt, Default::default())
            .with_source_snapshot(source.clone()).with_lookback_source_snapshots(vec![source])
    };
    let mut wrong_group = receipt.clone();
    wrong_group.first_draw.as_mut().unwrap().owner.as_mut().unwrap().pair.pair += 1;
    assert!(check_triggers(&game, &rebuild(wrong_group, source_snapshot.clone())).is_empty());
    let mut wrong_host = receipt.clone();
    wrong_host.first_draw.as_mut().unwrap().owner.as_mut().unwrap().host = ObjectId::from_raw(8000);
    assert!(check_triggers(&game, &rebuild(wrong_host, source_snapshot.clone())).is_empty());
    let mut missing_source = receipt.clone(); missing_source.source = None;
    assert!(matches!(ironsmith::triggers::check_triggers_checked(&game, &rebuild(missing_source, source_snapshot.clone())),
        Err(ironsmith::effects::ExecutionError::IncompleteEvidence(_))));
    let mut missing_origins = source_snapshot.clone(); missing_origins.ability_origins = None;
    let incomplete = rebuild(receipt.clone(), missing_origins);
    assert!(matches!(ironsmith::triggers::check_triggers_checked(&game, &incomplete),
        Err(ironsmith::effects::ExecutionError::IncompleteEvidence(_))));
    let native_checkpoint = game.clone();
    game.queue_trigger_event(incomplete.provenance(), incomplete);
    let mut pending = TriggerQueue::new();
    assert!(matches!(ironsmith::game_loop::check_and_apply_sbas(&mut game, &mut pending),
        Err(ironsmith::GameLoopError::ExecutionFailed(ironsmith::effects::ExecutionError::IncompleteEvidence(_)))));
    assert!(pending.entries.is_empty());
    assert!(game.stack.is_empty());
    game = native_checkpoint;
    assert_eq!(ironsmith::triggers::check_triggers_checked(&game, event).unwrap().len(), 1,
        "native recovery restores complete evidence rather than treating it as a negative match");
    let other = game.create_object_from_definition(&definition, A, Zone::Battlefield);
    let mut other_snapshot = ObjectSnapshot::from_object_with_calculated_characteristics(game.object(other).unwrap(), &game);
    other_snapshot.object_id = source;
    let effect = ironsmith::continuous::ContinuousEffect::new(source, A,
        ironsmith::continuous::EffectTarget::Specific(source),
        ironsmith::continuous::Modification::ModifyPowerToughness { power: 0, toughness: 0 });
    let effect_origin = ironsmith::continuous::AbilityEffectOrigin::from(&effect);
    other_snapshot.ability_origins = Some(std::sync::Arc::new(vec![
        ironsmith::continuous::AbilityOrigin::Borrowed { effect: effect_origin.clone(), source: other, origin: Box::new(ironsmith::continuous::AbilityOrigin::Printed(0)) },
        ironsmith::continuous::AbilityOrigin::Borrowed { effect: effect_origin, source: other, origin: Box::new(ironsmith::continuous::AbilityOrigin::Printed(1)) },
    ]));
    assert!(check_triggers(&game, &rebuild(receipt.clone(), other_snapshot)).is_empty(), "a borrowed acquisition cannot adopt a printed occurrence");
    let grave = game.move_object_by_effect(source, Zone::Graveyard).unwrap();
    let returned = game.move_object_by_effect(grave, Zone::Battlefield).unwrap();
    assert_ne!(returned, source);
    let entries = check_triggers(&game, event);
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].source, source, "only the historical incarnation owns this reveal");
}

#[test]
fn legacy_native_unmarked_reveal_pair_keeps_its_compatibility_path() {
    use ironsmith::ability::Ability;
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::target::{ObjectFilter, PlayerFilter};
    let definition = CardDefinitionBuilder::new(ironsmith::CardId::new(), "Native unmarked reveal")
        .card_types(vec![ironsmith::CardType::Enchantment])
        .with_ability(Ability::static_ability(ironsmith::static_abilities::StaticAbility::reveal_first_card_you_draw_each_turn(false, false)))
        .with_ability(Ability::triggered(ironsmith::triggers::Trigger::player_reveals_card(
            PlayerFilter::You, ObjectFilter::creature(), true), vec![ironsmith::Effect::gain_life(3)]))
        .build();
    let mut game = game();
    let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
    let card = ironsmith::card::CardBuilder::new(ironsmith::CardId::new(), "Native creature witness")
        .card_types(vec![ironsmith::CardType::Creature]).build();
    game.create_object_from_card(&card, A, Zone::Library);
    let mut answers = Answers::default();
    let outcome = draw(&mut game, source, A, 1, &mut answers);
    let reveals = reveal_events(&outcome);
    assert_eq!(reveals.len(), 1);
    assert!(reveals[0].first_draw.as_ref().unwrap().owner.is_none());
    queue(&mut game, outcome, &mut answers);
    assert_eq!(game.stack.len(), 1);
    resolve_stack_entry_with(&mut game, &mut answers).unwrap();
    assert_eq!(game.player(A).unwrap().life, 23);
}

#[test]
fn unrelated_public_reveal_from_the_same_host_cannot_satisfy_the_typed_first_draw_link() {
    for definition in definitions("Primitive Etchings") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let card = resource(&mut game, A, Zone::Hand, "Creature — Bear", "{1}{G}");
        let snapshot = ironsmith::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(game.object(card).unwrap(), &game);
        let event = TriggerEvent::new_with_provenance(ironsmith::events::CardRevealedEvent::new(
            A, card, Zone::Hand, Some(source), Some(snapshot)), Default::default());
        assert!(check_triggers(&game, &event).is_empty());
    }
}

#[test]
fn unmarked_artifact_defaults_preserve_legacy_json_shape_and_checksum() {
    // Separate authored lines deliberately exercise the legacy unmarked route.
    let text = "Type: Enchantment\nReveal the first card you draw each turn.\nWhenever you reveal a creature card this way, draw a card.";
    let (artifact, _) = compile_to_artifact("Legacy unmarked reveal artifact", text, false).unwrap();
    let json = artifact.to_json().unwrap();
    let text = std::str::from_utf8(&json).unwrap();
    assert!(!text.contains("linked_reveal_pair"));
    assert!(!text.contains("first_draw_pair"));
    assert_eq!(artifact.format_version, ironsmith_compiled_artifact::FORMAT_VERSION);
    let restored = CompiledCardArtifact::from_json(&json).unwrap();
    restored.validate().unwrap();
    assert_eq!(restored.payload_checksum, artifact.payload_checksum);
    assert_eq!(restored.to_json().unwrap(), json);
    assert_eq!(serde_json::to_value(&restored).unwrap(), serde_json::to_value(&artifact).unwrap());
}

#[test]
fn unrelated_legacy_tagged_copy_keeps_its_existing_reference_policy() {
    let mut game = game();
    let definition = compile_to_runtime_definition("Legacy tagged spell", "Mana cost: {U}\nType: Instant\nYou gain 2 life.", false).unwrap();
    let original = game.create_object_from_definition(&definition, A, Zone::Exile);
    let snapshot = ironsmith::snapshot::ObjectSnapshot::from_object(game.object(original).unwrap(), &game);
    assert!(snapshot.revealed_cast_definition.is_none());
    game.move_object_by_effect(original, Zone::Graveyard).unwrap();
    let mut tags = std::collections::HashMap::new();
    tags.insert(ironsmith::tag::TagKey::from("legacy_copy"), vec![snapshot]);
    let mut answers = Answers { cast: true, ..Default::default() };
    let mut context = ExecutionContext::new(ObjectId::from_raw(9000), A, &mut answers).with_tagged_objects(tags);
    let outcome = ironsmith::effects::player::CastTaggedEffect::new("legacy_copy", ironsmith::target::PlayerFilter::You)
        .as_copy().without_paying_mana_cost().execute(&mut game, &mut context).unwrap();
    assert!(outcome.status.is_success());
    drop(context);
    assert_eq!(game.stack.len(), 1);
    resolve_stack_entry_with(&mut game, &mut answers).unwrap();
    assert_eq!(game.player(A).unwrap().life, 22);
}

fn linked_group_shape(definition: &CardDefinition) -> Vec<usize> {
    let mut groups = Vec::new();
    for ability in &definition.abilities {
        let AbilityKind::Static(ability) = &ability.kind else { continue; };
        let Some(spec) = ability.reveal_drawn_card_spec() else { continue; };
        let pair = spec.linked_reveal_pair.expect("reparse retains the explicit authored group");
        let consumers = definition.abilities.iter().filter(|ability| {
            let AbilityKind::Triggered(ability) = &ability.kind else { return false; };
            ability.trigger.downcast_ref::<ironsmith::triggers::PlayerRevealsCardTrigger>()
                .is_some_and(|trigger| trigger.first_draw_pair == Some(pair))
        }).count();
        groups.push(consumers);
    }
    groups
}
#[test]
fn canonical_render_reparse_preserves_one_and_two_consumer_first_draw_groups() {
    for (name, expected) in [
        ("Primitive Etchings", vec![1]), ("Rowen", vec![1]),
        ("Keranos, God of Storms", vec![2]), ("God-Eternal Kefnet", vec![1]),
        ("Inquisitor Eisenhorn", vec![1]),
    ] {
        for definition in definitions(name) {
            let row = rows().into_iter().find(|row| row["name"] == name).unwrap();
            let mut text = format!("Mana cost: {}\nType: {}\n", row["mana_cost"].as_str().unwrap(), row["type_line"].as_str().unwrap());
            if let (Some(p), Some(t)) = (row["power"].as_str(), row["toughness"].as_str()) {
                text.push_str(&format!("Power/Toughness: {p}/{t}\n"));
            }
            let lines = ironsmith_text::canonical_compiled_lines(&definition);
            text.push_str(&lines.join("\n"));
            let (result, loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_runtime_definition(name, &text, false));
            let reparsed = result.unwrap_or_else(|error| panic!("{text}: {error}"));
            assert!(!loss.is_lossy());
            assert_eq!(linked_group_shape(&reparsed), expected);
            assert_eq!(reparsed.abilities.len(), definition.abilities.len());
        }
    }
}

#[test]
fn canonical_grouping_does_not_merge_an_unrelated_reader_or_two_distinct_pairs() {
    let text = "Type: Enchantment\nReveal the first card you draw each turn. Whenever you reveal an instant card this way, you gain 1 life.\nReveal the first card you draw each turn. Whenever you reveal an instant card this way, you gain 2 life.\nWhenever you reveal an instant card, you gain 7 life.";
    let mut definition = compile_to_runtime_definition("Two distinct reveal groups", text, false).unwrap();
    // Native transformations may interleave groups. The renderer follows
    // immutable membership, never proximity of the independently held bodies.
    definition.abilities.swap(1, 2);
    let rendered = ironsmith_text::canonical_compiled_lines(&definition).join("\n");
    let reparsed = compile_to_runtime_definition("Two distinct reveal groups", format!("Type: Enchantment\n{rendered}"), false).unwrap();
    assert_eq!(linked_group_shape(&reparsed), vec![1, 1]);
    assert_eq!(reparsed.abilities.len(), 5);
    let unrelated = reparsed.abilities.iter().filter(|ability| {
        let AbilityKind::Triggered(ability) = &ability.kind else { return false; };
        ability.trigger.downcast_ref::<ironsmith::triggers::PlayerRevealsCardTrigger>()
            .is_some_and(|trigger| !trigger.from_source && trigger.first_draw_pair.is_none())
    }).count();
    assert_eq!(unrelated, 1);
}

#[test]
fn departed_first_draw_copy_requires_complete_cast_lki_and_native_recovery_restores_it() {
    for definition in definitions("God-Eternal Kefnet") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let spell = compile_to_runtime_definition("Complete LKI spell", "Mana cost: {3}{U}\nType: Instant\nYou gain 2 life.", false).unwrap();
        game.create_object_from_definition(&spell, A, Zone::Library);
        let mut answers = Answers { cast: true, ..Default::default() };
        let outcome = draw(&mut game, source, A, 1, &mut answers);
        let event = outcome.events.iter().find(|event| event.downcast::<ironsmith::events::CardRevealedEvent>().is_some()).unwrap().clone();
        let retained = event.downcast::<ironsmith::events::CardRevealedEvent>().unwrap().snapshot.clone().unwrap();
        game.move_object_by_effect(retained.object_id, Zone::Graveyard).unwrap();
        game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Blue, 2);
        let effect = ironsmith::effects::player::CastTaggedEffect::new("exact_reveal_copy", ironsmith::target::PlayerFilter::You)
            .as_copy().cost_reduction(ironsmith::mana::ManaCost::from_symbols(vec![ironsmith::mana::ManaSymbol::Generic(2)]));
        let mut partial = retained.clone(); partial.revealed_cast_definition = None;
        let tags = |snapshot| {
            let mut tags = std::collections::HashMap::new();
            tags.insert(ironsmith::tag::TagKey::from("exact_reveal_copy"), vec![snapshot]); tags
        };
        let mut context = ExecutionContext::new(source, A, &mut answers)
            .with_triggering_event(event.clone()).with_tagged_objects(tags(partial));
        assert!(matches!(effect.execute(&mut game, &mut context), Err(ironsmith::effects::ExecutionError::IncompleteEvidence(_))));
        drop(context);
        assert!(game.stack.is_empty());
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 2);
        game = game.clone();
        let mut context = ExecutionContext::new(source, A, &mut answers)
            .with_triggering_event(event).with_tagged_objects(tags(retained));
        assert!(effect.execute(&mut game, &mut context).unwrap().status.is_success());
        drop(context);
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
        resolve_stack_entry_with(&mut game, &mut answers).unwrap();
        assert_eq!(game.player(A).unwrap().life, 22);
    }
}
