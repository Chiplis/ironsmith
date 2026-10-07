//! Frozen complete counter-unless bodies; execution is deferred by the campaign gate.
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::{DecisionMaker, GameProgress, LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
use ironsmith::decisions::context::{BooleanContext, DecisionContext, SelectObjectsContext, TargetsContext, ViewCardsContext};
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse, apply_priority_response_with_dm,
    apply_decision_context_with_dm, drain_pending_trigger_events, put_triggers_on_stack_with_dm, resolve_stack_entry_with};
use ironsmith::game_state::Target;
use ironsmith::mana::ManaSymbol;
use ironsmith::replacement::{ReplacementAction, ReplacementEffect};
use ironsmith::static_abilities::StaticAbilityId;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{CardDefinition, CardId, CardType, GameState, ObjectId, PlayerId, Zone};
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
const C: PlayerId = PlayerId(2);
#[derive(Clone, Copy, Debug)] enum Route { Direct, Artifact }
const ROUTES: [Route; 2] = [Route::Direct, Route::Artifact];
fn compile(name: &str, text: &str, route: Route) -> CardDefinition {
    let builder = || ironsmith_compiler::CardDefinitionBuilder::new(CardId::new(), name);
    match route {
        Route::Direct => {
            let (result, loss) = ironsmith_compiler::parse_loss::capture(||
                ironsmith_registry::compile_builder_to_runtime_definition(builder(), text, false));
            assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text()); result.unwrap()
        }
        Route::Artifact => {
            let (result, loss) = ironsmith_compiler::parse_loss::capture(||
                ironsmith_registry::compile_builder_to_artifact(builder(), text, false));
            assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
            let (artifact, _) = result.unwrap(); artifact.validate().unwrap();
            let decoded = serde_json::from_slice(&serde_json::to_vec(&artifact).unwrap()).unwrap();
            ironsmith::artifact_materializer::materialize_artifact(&decoded).unwrap()
        }
    }
}
fn definition(name: &str, route: Route) -> CardDefinition {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/card-failure-campaign/nonmana-unless-counter-payments.json")).unwrap();
    let row = fixture["cards"].as_array().unwrap().iter().find(|row| row["name"] == name).unwrap();
    compile(name, row["text"].as_str().unwrap(), route)
}
struct Choices {
    accept: bool, selected: Option<Vec<ObjectId>>, targets: Vec<Target>, pause_boolean: bool,
    pause_objects: bool, pending: bool, booleans: Vec<PlayerId>, objects: Vec<SelectObjectsContext>,
    views: Vec<(PlayerId, PlayerId, bool, Vec<ObjectId>)>,
}
impl Default for Choices {
    fn default() -> Self { Self { accept: true, selected: None, targets: vec![], pause_boolean: false,
        pause_objects: false, pending: false, booleans: vec![], objects: vec![], views: vec![] } }
}
impl DecisionMaker for Choices {
    fn answers_player_choices(&self) -> bool { true }
    fn awaiting_choice(&self) -> bool { self.pending }
    fn decide_boolean(&mut self, _: &GameState, ctx: &BooleanContext) -> bool {
        self.booleans.push(ctx.player); self.pending = self.pause_boolean; self.accept
    }
    fn decide_objects(&mut self, game: &GameState, ctx: &SelectObjectsContext) -> Vec<ObjectId> {
        self.objects.push(ctx.clone()); self.pending = self.pause_objects;
        self.selected.clone().unwrap_or_else(|| SelectFirstDecisionMaker.decide_objects(game, ctx))
    }
    fn decide_targets(&mut self, game: &GameState, ctx: &TargetsContext) -> Vec<Target> {
        if self.targets.is_empty() { return SelectFirstDecisionMaker.decide_targets(game, ctx); }
        let mut selected = vec![];
        for requirement in &ctx.requirements {
            let choices = self.targets.iter().copied().filter(|target| requirement.legal_targets.contains(target))
                .take(requirement.max_targets.unwrap_or(self.targets.len())).collect::<Vec<_>>();
            assert!(choices.len() >= requirement.min_targets, "{ctx:?}"); selected.extend(choices);
        }
        selected
    }
    fn view_cards(&mut self, _: &GameState, viewer: PlayerId, cards: &[ObjectId], ctx: &ViewCardsContext) {
        self.views.push((viewer, ctx.subject, ctx.public, cards.to_vec()));
    }
}
struct Board { game: GameState, queue: TriggerQueue, choices: Choices, route: Route }
impl Board {
    fn new(route: Route) -> Self {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Cara".into()], 20);
        game.set_random_seed(20261005); game.turn.turn_number = 4;
        game.turn.active_player = A; game.turn.priority_player = Some(A);
        game.turn.phase = ironsmith::Phase::FirstMain; game.turn.step = None;
        for player in [A, B, C] { for mana in [ManaSymbol::White, ManaSymbol::Blue, ManaSymbol::Black,
            ManaSymbol::Red, ManaSymbol::Green, ManaSymbol::Colorless] {
            game.player_mut(player).unwrap().mana_pool.add(mana, 30);
        } }
        Self { game, queue: TriggerQueue::new(), choices: Choices::default(), route }
    }
    fn neutral(&mut self, owner: PlayerId, zone: Zone, name: &str) -> ObjectId {
        let definition = CardDefinitionBuilder::new(CardId::new(), name).card_types(vec![CardType::Creature])
            .mana_cost(ironsmith::mana::ManaCost::new().add_generic(3))
            .power_toughness(ironsmith::card::PowerToughness::fixed(3, 3)).build();
        self.game.create_object_from_definition(&definition, owner, zone)
    }
    fn track(&mut self, id: ObjectId, slot: u16) {
        self.game.set_hidden_card_info(id, ironsmith::game_state::HiddenCardInfo {
                incarnation: Some(0),
            owner: self.game.object(id).unwrap().owner, zone: Zone::Hand, slot,
            commitment: format!("unless-card-{slot}"), origin_slot: None, origin_commitment: None,
            public_slot: None, public_commitment: None,
        });
    }
    fn dispatch(&mut self, action: LegalAction) {
        let mut state = PriorityLoopState::new(self.game.players_in_game());
        let mut progress = apply_priority_response_with_dm(&mut self.game, &mut self.queue, &mut state,
            &PriorityResponse::PriorityAction(action), &mut self.choices).unwrap();
        for _ in 0..40 {
            if let GameProgress::NeedsDecisionCtx(ref ctx) = progress
                && !matches!(ctx, DecisionContext::Priority(_)) {
                progress = apply_decision_context_with_dm(&mut self.game, &mut self.queue, &mut state, ctx, &mut self.choices).unwrap();
            } else { assert!(state.pending_cast.is_none() && state.pending_activation.is_none()); return; }
        }
        panic!("announcement did not finish");
    }
    fn announce(&mut self, definition: &CardDefinition, actor: PlayerId, mana: u32) -> ObjectId {
        self.game.turn.priority_player = Some(actor);
        let hand = self.game.create_object_from_definition(definition, actor, Zone::Hand);
        let stable = self.game.object(hand).unwrap().stable_id;
        let before = self.game.player(actor).unwrap().mana_pool.total();
        let action = compute_legal_actions(&self.game, actor).unwrap().into_iter().find(|action| matches!(action,
            LegalAction::CastSpell { spell_id, casting_method: ironsmith::alternative_cast::CastingMethod::Normal, .. } if *spell_id == hand)).unwrap();
        self.dispatch(action);
        assert_eq!(before - self.game.player(actor).unwrap().mana_pool.total(), mana);
        self.game.find_object_by_stable_id(stable).unwrap()
    }
    fn cast(&mut self, name: &str, actor: PlayerId, mana: u32) -> ObjectId {
        self.game.turn.active_player = actor;
        let definition = definition(name, self.route);
        let spell = self.announce(&definition, actor, mana);
        let stable = self.game.object(spell).unwrap().stable_id;
        self.finish(); self.game.find_object_by_stable_id(stable).unwrap()
    }
    fn put_triggers(&mut self) {
        drain_pending_trigger_events(&mut self.game, &mut self.queue);
        put_triggers_on_stack_with_dm(&mut self.game, &mut self.queue, &mut self.choices).unwrap();
    }
    fn resolve_one(&mut self) { self.put_triggers(); resolve_stack_entry_with(&mut self.game, &mut self.choices).unwrap(); }
    fn finish(&mut self) {
        for _ in 0..30 { self.put_triggers(); if self.game.stack.is_empty() { return; }
            resolve_stack_entry_with(&mut self.game, &mut self.choices).unwrap(); }
        panic!("unresolved stack");
    }
    fn victim(&mut self, actor: PlayerId, uncounterable: bool) -> ObjectId {
        self.game.turn.active_player = actor;
        let text = if uncounterable { "Mana cost: {4}\nType: Sorcery\nThis spell can't be countered.\nYou gain 1 life." }
            else { "Mana cost: {4}\nType: Sorcery\nYou gain 1 life." };
        self.announce(&compile("Pending witness spell", text, self.route), actor, 4)
    }
    fn spell_present(&self, id: ObjectId) -> bool { self.game.stack.iter().any(|entry| !entry.is_ability && entry.object_id == id) }
    fn counters(&self) -> usize { self.game.turn_store.turn_history.event_kind_count(ironsmith::events::EventKind::SpellCountered) as usize }
}

#[test]
fn grip_and_perplex_distinguish_empty_payment_acceptance_from_declining_and_use_current_spell_controller() {
    for route in ROUTES { for name in ["Grip of Amnesia", "Perplex"] { for count in [0, 2] {
        for accept in [false, true] { for changed in [false, true] {
            let mut board = Board::new(route); let victim = board.victim(B, false);
            let payer = if changed { C } else { B };
            let zone = if name == "Grip of Amnesia" { Zone::Graveyard } else { Zone::Hand };
            let resources = (0..count).map(|_| board.neutral(payer, zone, "Payment resource")).collect::<Vec<_>>();
            let foreign = board.neutral(A, zone, "Counter controller's resource");
            let witness = board.neutral(A, Zone::Library, "Draw witness");
            let witness_stable = board.game.object(witness).unwrap().stable_id;
            board.choices.targets = vec![Target::Object(victim)];
            board.announce(&definition(name, route), A, if name == "Grip of Amnesia" { 2 } else { 3 });
            if changed { board.game.set_current_controller(victim, C).unwrap(); }
            board.choices.accept = accept; board.choices.booleans.clear();
            board.resolve_one();
            assert_eq!(board.spell_present(victim), accept, "{name} count={count} accept={accept}");
            assert_eq!(board.counters(), usize::from(!accept));
            assert_eq!(board.choices.booleans, vec![payer]);
            assert_eq!(board.game.object(foreign).unwrap().zone, zone);
            for resource in resources { assert_eq!(board.game.object(resource).is_some(), !accept); }
            if name == "Grip of Amnesia" {
                let draw = board.game.find_object_by_stable_id(witness_stable).unwrap();
                assert_eq!(board.game.object(draw).unwrap().zone, Zone::Hand);
                assert_eq!(board.game.exile.iter().filter(|id| board.game.object(**id).unwrap().owner == payer).count(), if accept { count } else { 0 });
            }
        } }
    } } }
}

#[test]
fn blood_funnel_reduces_only_noncreature_spells_and_its_trigger_keeps_you_bound_to_trigger_controller() {
    for route in ROUTES { for available in [false, true] { for accept in [false, true] {
        let mut board = Board::new(route);
        let source = board.cast("Blood Funnel", A, 2);
        let creature = compile("Payment creature", "Mana cost: {3}\nType: Creature — Bear\nPower/Toughness: 3/3", route);
        let payment = if available { let id = board.announce(&creature, A, 3); let stable = board.game.object(id).unwrap().stable_id;
            board.finish(); Some(board.game.find_object_by_stable_id(stable).unwrap()) } else { None };
        let foreign = board.neutral(B, Zone::Battlefield, "Opponent's creature");
        let spell = compile("Discounted spell", "Mana cost: {3}\nType: Sorcery\nYou gain 1 life.", route);
        let victim = board.announce(&spell, A, 1);
        board.put_triggers(); board.game.set_current_controller(source, B).unwrap();
        board.choices.accept = accept; board.choices.selected = payment.map(|id| vec![id]); board.choices.booleans.clear();
        board.resolve_one();
        assert_eq!(board.spell_present(victim), available && accept);
        assert_eq!(board.counters(), usize::from(!available || !accept));
        assert_eq!(board.choices.booleans, if available { vec![A] } else { vec![] });
        assert!(board.game.object(foreign).is_some());
        if let Some(payment) = payment { assert_eq!(board.game.object(payment).is_some(), !accept); }
    } } }
}

#[test]
fn hope_ender_cast_trigger_uses_plural_pay_and_keeps_devoid_flash_and_flying_body() {
    for route in ROUTES { for accept in [false, true] {
        let mut board = Board::new(route); let victim = board.victim(B, false);
        board.game.turn.phase = ironsmith::Phase::Beginning; board.game.turn.step = Some(ironsmith::Step::Upkeep);
        board.choices.targets = vec![Target::Object(victim)];
        let coatl = board.announce(&definition("Hope-Ender Coatl", route), A, 3);
        let stable = board.game.object(coatl).unwrap().stable_id;
        board.put_triggers(); assert_eq!(board.game.stack.len(), 3);
        board.choices.accept = accept; board.choices.booleans.clear();
        let before = board.game.player(B).unwrap().mana_pool.total();
        board.resolve_one();
        assert_eq!(board.spell_present(victim), accept);
        assert_eq!(before - board.game.player(B).unwrap().mana_pool.total(), u32::from(accept));
        assert_eq!(board.choices.booleans, vec![B]);
        assert!(board.spell_present(coatl));
        board.resolve_one();
        let permanent = board.game.find_object_by_stable_id(stable).unwrap();
        assert_eq!(board.game.object(permanent).unwrap().zone, Zone::Battlefield);
        assert_eq!(board.game.current_colors(permanent), Some(ironsmith::color::ColorSet::COLORLESS));
        assert_eq!((board.game.calculated_power(permanent), board.game.calculated_toughness(permanent)), (Some(2), Some(2)));
        assert!(board.game.object_has_static_ability_id(permanent, StaticAbilityId::Flash));
        assert!(board.game.object_has_static_ability_id(permanent, StaticAbilityId::Flying));
    } }
}

#[test]
fn reality_smasher_binds_the_targeting_spell_and_discard_payer_without_ability_or_friendly_spell_false_triggers() {
    for route in ROUTES { for accept in [false, true] { for count in [0, 2] {
        let mut board = Board::new(route); let smasher = board.cast("Reality Smasher", A, 5);
        assert_eq!(board.game.calculated_power(smasher), Some(5)); assert_eq!(board.game.calculated_toughness(smasher), Some(5));
        assert!(board.game.object_has_static_ability_id(smasher, StaticAbilityId::Trample));
        assert!(board.game.object_has_static_ability_id(smasher, StaticAbilityId::Haste));
        let hand = (0..count).map(|_| board.neutral(B, Zone::Hand, "Discard resource")).collect::<Vec<_>>();
        let foreign = board.neutral(A, Zone::Hand, "Wrong payer card");
        let spell = compile("Targeting instant", "Mana cost: {U}\nType: Instant\nTap target creature.", route);
        board.choices.targets = vec![Target::Object(smasher)];
        let victim = board.announce(&spell, B, 1); board.put_triggers();
        board.choices.accept = accept; board.choices.selected = hand.last().map(|id| vec![*id]); board.choices.booleans.clear();
        board.resolve_one();
        assert_eq!(board.spell_present(victim), count > 0 && accept);
        assert_eq!(board.game.player(B).unwrap().hand.len(), count - usize::from(count > 0 && accept));
        assert!(board.game.object(foreign).is_some());
        assert_eq!(board.choices.booleans, if count > 0 { vec![B] } else { vec![] });
    } }
        let mut board = Board::new(route); let smasher = board.cast("Reality Smasher", A, 5);
        board.choices.targets = vec![Target::Object(smasher)];
        board.announce(&compile("Friendly targeting instant", "Mana cost: {U}\nType: Instant\nTap target creature.", route), A, 1);
        board.put_triggers(); assert_eq!(board.game.stack.len(), 1, "friendly spell does not trigger Smasher");
        board.finish();
        let source = board.game.create_object_from_definition(&compile("Opponent targeting ability",
            "Type: Artifact\n{0}: Untap target creature.", route), B, Zone::Battlefield);
        board.game.turn.priority_player = Some(B);
        let action = compute_legal_actions(&board.game, B).unwrap().into_iter().find(|action| matches!(action,
            LegalAction::ActivateAbility { source: id, .. } if *id == source)).unwrap();
        board.dispatch(action); board.put_triggers(); assert_eq!(board.game.stack.len(), 1, "an ability is not a spell");
    }
}

#[test]
fn perplex_keeps_its_native_hand_transmute_cost_mana_value_filter_reveal_and_shuffle() {
    for route in ROUTES {
        let mut board = Board::new(route);
        let source = board.game.create_object_from_definition(&definition("Perplex", route), A, Zone::Hand);
        let source_stable = board.game.object(source).unwrap().stable_id;
        let wanted = board.neutral(A, Zone::Library, "Three mana card"); let wanted_stable = board.game.object(wanted).unwrap().stable_id;
        let wrong = board.game.create_object_from_definition(&compile("Wrong mana value", "Mana cost: {1}\nType: Artifact", route), A, Zone::Library);
        board.choices.selected = Some(vec![wanted]);
        let action = compute_legal_actions(&board.game, A).unwrap().into_iter().find(|action| matches!(action,
            LegalAction::ActivateAbility { source: id, .. } if *id == source)).unwrap();
        let mana = board.game.player(A).unwrap().mana_pool.total();
        board.dispatch(action); assert_eq!(mana - board.game.player(A).unwrap().mana_pool.total(), 3);
        let grave = board.game.find_object_by_stable_id(source_stable).unwrap();
        assert_eq!(board.game.object(grave).unwrap().zone, Zone::Graveyard);
        board.finish();
        let hand = board.game.find_object_by_stable_id(wanted_stable).unwrap();
        assert_eq!(board.game.object(hand).unwrap().zone, Zone::Hand);
        assert_eq!(board.game.object(wrong).unwrap().zone, Zone::Library);
        assert!(board.choices.views.iter().any(|view| view.2 && view.3.contains(&wanted)));
        let other = board.game.create_object_from_definition(&definition("Perplex", route), A, Zone::Hand);
        board.game.turn.active_player = B; board.game.turn.priority_player = Some(A);
        assert!(!compute_legal_actions(&board.game, A).unwrap().iter().any(|action| matches!(action,
            LegalAction::ActivateAbility { source, .. } if *source == other)));
    }
}

#[test]
fn modified_nonmana_payments_complete_without_fabricating_discard_sacrifice_or_counter_events() {
    for route in ROUTES { for name in ["Grip of Amnesia", "Perplex", "Blood Funnel", "Reality Smasher"] {
        for instead in [false, true] {
            let mut board = Board::new(route);
            let source = match name { "Blood Funnel" => Some(board.cast(name, A, 2)), "Reality Smasher" => Some(board.cast(name, A, 5)), _ => None };
            let (payer, zone) = match name { "Grip of Amnesia" => (B, Zone::Graveyard), "Blood Funnel" => (A, Zone::Battlefield), _ => (B, Zone::Hand) };
            let resource = board.neutral(payer, zone, "Replaced payment resource");
            let action = if instead { ReplacementAction::Instead(vec![ironsmith::effect::Effect::gain_life(4)]) } else { ReplacementAction::Prevent };
            let shield_source = board.neutral(C, Zone::Battlefield, "Replacement source");
            let shield = if name == "Perplex" || name == "Reality Smasher" {
                ReplacementEffect::with_matcher(shield_source, C, ironsmith::events::WouldDiscardMatcher::any_player()
                    .with_card_filter(ironsmith::filter::ObjectFilter::specific(resource)), action)
            } else { ReplacementEffect::with_matcher(shield_source, C,
                ironsmith::events::zones::matchers::WouldChangeZoneMatcher::new(ironsmith::filter::ObjectFilter::specific(resource),
                    Some(zone), Some(if name == "Grip of Amnesia" { Zone::Exile } else { Zone::Graveyard })), action) };
            board.game.effect_store.replacement_effects.add_one_shot_effect(shield);
            let victim = if name == "Blood Funnel" {
                board.announce(&compile("Discounted witness", "Mana cost: {3}\nType: Sorcery\nYou gain 1 life.", route), A, 1)
            } else if name == "Reality Smasher" {
                board.choices.targets = vec![Target::Object(source.unwrap())];
                board.announce(&compile("Targeting witness", "Mana cost: {U}\nType: Instant\nTap target creature.", route), B, 1)
            } else {
                let victim = board.victim(B, false); board.neutral(A, Zone::Library, "Draw witness");
                board.choices.targets = vec![Target::Object(victim)];
                board.announce(&definition(name, route), A, if name == "Grip of Amnesia" { 2 } else { 3 }); victim
            };
            board.choices.selected = Some(vec![resource]); board.resolve_one();
            assert!(board.spell_present(victim), "{name}: modified legal payment is completed");
            assert_eq!(board.game.object(resource).unwrap().zone, zone);
            assert_eq!(board.game.player(C).unwrap().life, if instead { 24 } else { 20 });
            assert_eq!(board.counters(), 0);
            assert_eq!(board.game.turn_store.turn_history.event_kind_count(ironsmith::events::EventKind::CardDiscarded), 0);
            assert_eq!(board.game.turn_store.turn_history.event_kind_count(ironsmith::events::EventKind::Sacrifice), 0);
        }
    } }
}

#[test]
fn pending_payment_never_counters_or_draws_and_the_exact_opened_hand_replays_once() {
    for route in ROUTES { for name in ["Grip of Amnesia", "Perplex"] {
        let mut board = Board::new(route); let victim = board.victim(B, false);
        let resource = board.neutral(B, if name == "Perplex" { Zone::Hand } else { Zone::Graveyard }, "Payment");
        if name == "Perplex" { board.track(resource, 0); }
        let draw = board.neutral(A, Zone::Library, "Pending draw");
        board.choices.targets = vec![Target::Object(victim)];
        let counter = board.announce(&definition(name, route), A, if name == "Grip of Amnesia" { 2 } else { 3 });
        board.choices.pause_boolean = true; board.choices.views.clear();
        board.resolve_one();
        assert!(board.choices.pending); assert!(board.spell_present(victim)); assert!(board.spell_present(counter));
        assert_eq!(board.game.object(draw).unwrap().zone, Zone::Library); assert!(board.choices.views.is_empty());
        board.choices.pause_boolean = false; board.choices.pending = false;
        if name == "Perplex" {
            board.choices.pause_objects = true; board.choices.selected = Some(vec![resource]); board.resolve_one();
            assert!(board.choices.pending); assert!(board.spell_present(victim)); assert!(board.spell_present(counter));
            let context = board.choices.objects.last().unwrap();
            assert_eq!(context.cost_payment, Some(ironsmith::decisions::context::CostPaymentIdentity { source: counter, payer: B }));
            assert_eq!(context.reveal_policy, ironsmith::decisions::context::SelectionRevealPolicy::Public);
            board.choices.pause_objects = false; board.choices.pending = false;
        }
        board.resolve_one(); assert!(board.spell_present(victim)); assert!(!board.spell_present(counter)); assert_eq!(board.counters(), 0);
    } }
}

#[test]
fn uncounterable_spell_stays_on_stack_after_declining_payment_and_grip_still_draws() {
    for route in ROUTES { for name in ["Grip of Amnesia", "Perplex"] {
        let mut board = Board::new(route); let victim = board.victim(B, true); board.neutral(A, Zone::Library, "Draw witness");
        board.choices.targets = vec![Target::Object(victim)]; board.announce(&definition(name, route), A, if name == "Grip of Amnesia" { 2 } else { 3 });
        board.choices.accept = false; board.resolve_one(); assert!(board.spell_present(victim)); assert_eq!(board.counters(), 0);
        assert_eq!(board.game.player(A).unwrap().hand.len(), usize::from(name == "Grip of Amnesia"));
    } }
}

#[test]
fn explicitly_paying_zero_is_an_offer_and_does_not_mean_declining() {
    for route in ROUTES { for accept in [false, true] {
        let mut board = Board::new(route); let victim = board.victim(B, false);
        let counter = compile("Zero payment control", "Mana cost: {U}\nType: Instant\nCounter target spell unless its controller pays {0}.", route);
        board.choices.targets = vec![Target::Object(victim)]; board.announce(&counter, A, 1);
        board.choices.accept = accept; board.choices.booleans.clear();
        let before = board.game.player(B).unwrap().mana_pool.clone();
        board.resolve_one();
        assert_eq!(board.choices.booleans, vec![B]); assert_eq!(board.spell_present(victim), accept);
        assert_eq!(board.game.player(B).unwrap().mana_pool, before);
    } }
}

#[test]
fn discard_hand_payment_rejects_missing_opening_then_pays_after_the_exact_identity_is_supplied() {
    for route in ROUTES {
        let mut board = Board::new(route); let victim = board.victim(B, false);
        let resource = board.game.create_hidden_card_placeholder(B, Zone::Hand, 0, "unopened-perplex-payment".into());
        let stable = board.game.object(resource).unwrap().stable_id;
        board.choices.targets = vec![Target::Object(victim)];
        let source = board.announce(&definition("Perplex", route), A, 3);
        board.choices.selected = Some(vec![resource]);
        let result = resolve_stack_entry_with(&mut board.game, &mut board.choices);
        assert!(matches!(result, Err(ironsmith::game_loop::GameLoopError::ExecutionFailed(
            ironsmith::effects::ExecutionError::IncompleteEvidence(_)))));
        assert!(board.spell_present(victim)); assert!(board.spell_present(source));
        assert_eq!(board.game.player(B).unwrap().hand, vec![resource]);
        assert!(!board.game.is_publicly_revealed_hidden_card(resource));
        let opened = CardDefinitionBuilder::new(CardId::new(), "Authenticated payment").card_types(vec![CardType::Artifact]).build();
        board.game.reveal_hidden_card_with_definition(resource, &opened).unwrap();
        board.resolve_one();
        assert!(board.spell_present(victim)); assert!(!board.spell_present(source));
        let discarded = board.game.find_object_by_stable_id(stable).unwrap();
        assert_eq!(board.game.object(discarded).unwrap().zone, Zone::Graveyard);
        assert_eq!(board.counters(), 0);
    }
}

#[test]
fn smasher_resolution_keeps_trigger_controller_spell_controller_and_resource_owner_distinct() {
    for route in ROUTES {
        let mut board = Board::new(route); let source = board.cast("Reality Smasher", A, 5);
        let original_payer_card = board.neutral(B, Zone::Hand, "Original caster's card");
        let chosen = board.neutral(C, Zone::Hand, "New spell controller's card");
        board.choices.targets = vec![Target::Object(source)];
        let victim = board.announce(&compile("Control-change witness", "Mana cost: {U}\nType: Instant\nTap target creature.", route), B, 1);
        board.put_triggers();
        board.game.set_current_controller(source, B).unwrap();
        board.game.set_current_controller(victim, C).unwrap();
        board.choices.selected = Some(vec![chosen]); board.choices.booleans.clear(); board.resolve_one();
        assert_eq!(board.choices.booleans, vec![C]);
        assert!(board.spell_present(victim)); assert!(board.game.object(original_payer_card).is_some());
        assert!(board.game.object(chosen).is_none()); assert_eq!(board.counters(), 0);
    }
}

#[test]
fn malformed_native_discard_cost_selection_does_not_substitute_an_available_card() {
    let mut board = Board::new(Route::Direct);
    let source = board.neutral(A, Zone::Battlefield, "Payment source");
    let legal = board.neutral(B, Zone::Hand, "Legal payment");
    let foreign = board.neutral(A, Zone::Hand, "Foreign payment");
    let token = board.neutral(B, Zone::Hand, "Token is not a card");
    board.game.object_mut(token).unwrap().kind = ironsmith::object::ObjectKind::Token;
    let cost = ironsmith::costs::Cost::discard(1, None);
    for invalid in [vec![foreign], vec![token], vec![legal, legal]] {
        let mut choices = Choices::default();
        let mut ctx = ironsmith::costs::CostContext::new(source, B, &mut choices).with_pre_chosen_cards(invalid);
        assert!(matches!(cost.pay(&mut board.game, &mut ctx), Err(ironsmith::cost::CostPaymentError::ExecutionFailed(_))));
        assert!(choices.views.is_empty());
        assert!(board.game.object(legal).is_some()); assert!(board.game.object(foreign).is_some()); assert!(board.game.object(token).is_some());
    }
}

#[test]
fn public_compilers_reject_symbols_inside_or_after_the_counter_payment_actor() {
    for clause in [
        "Counter target spell unless its {R} controller pays {1}.",
        "Counter target spell unless its controller {R} discards a card.",
        "Counter target spell unless you : sacrifice a creature.",
        "Counter target spell unless you {R} sacrifice a creature.",
        "Counter target spell unless its controller : discards their hand.",
    ] {
        let text = format!("Mana cost: {{U}}\nType: Instant\n{clause}");
        assert!(ironsmith_registry::compile_builder_to_runtime_definition(
            ironsmith_compiler::CardDefinitionBuilder::new(CardId::new(), "Malformed payment actor"), text.clone(), false).is_err(), "{clause}");
        assert!(ironsmith_registry::compile_builder_to_artifact(
            ironsmith_compiler::CardDefinitionBuilder::new(CardId::new(), "Malformed payment actor"), text, false).is_err(), "{clause}");
    }
}
