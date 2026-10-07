//! Complete announcement/resolution contracts, authored without execution.
use super::definitions;
use ironsmith::ability::{Ability, AbilityKind, ProtectionFrom};
use ironsmith::alternative_cast::{AlternativeCastingMethod, CastingMethod};
use ironsmith::card::{CardBuilder, PowerToughness};
use ironsmith::cards::CardDefinition;
use ironsmith::decision::{DecisionMaker, LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
use ironsmith::decisions::context::{BooleanContext, ManaPaymentContext, SelectObjectsContext, SelectOptionsContext, TargetsContext};
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse, apply_decision_context_with_dm, apply_priority_response_with_dm, put_triggers_on_stack_with_dm, resolve_stack_entry_with};
use ironsmith::mana::ManaSymbol;
use ironsmith::static_abilities::StaticAbility;
use ironsmith::triggers::{TriggerEvent, TriggerQueue, check_triggers};
use ironsmith::ids::StableId;
use ironsmith::{CardId, CardType, Color, ColorSet, CounterType, GameProgress, GameState, ObjectId, Phase, PlayerId, Subtype, Target, Zone};
use std::collections::VecDeque;

const A: PlayerId = PlayerId::from_index(0);
const B: PlayerId = PlayerId::from_index(1);

fn bodies(name: &str) -> [CardDefinition; 2] {
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!("../../../../fixtures/typed_text_changes.json.fixture")).unwrap();
    definitions(rows.iter().find(|row| row["name"] == name).unwrap())
}
fn game() -> GameState {
    let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
    game.turn.active_player = A;
    game.turn.priority_player = Some(A);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    for symbol in [ManaSymbol::White, ManaSymbol::Blue, ManaSymbol::Black, ManaSymbol::Red, ManaSymbol::Green, ManaSymbol::Colorless] {
        game.player_mut(A).unwrap().mana_pool.add(symbol, 20);
    }
    game
}
fn object(game: &mut GameState, owner: PlayerId, zone: Zone, types: Vec<CardType>, subtypes: Vec<Subtype>) -> ObjectId {
    let card = CardBuilder::new(CardId::new(), "Unchanged witness name")
        .card_types(types).subtypes(subtypes).power_toughness(PowerToughness::fixed(2, 3)).build();
    game.create_object_from_card(&card, owner, zone)
}
fn protected(game: &mut GameState, owner: PlayerId) -> ObjectId {
    let target = object(game, owner, Zone::Battlefield, vec![CardType::Creature], vec![Subtype::Human]);
    game.object_mut(target).unwrap().abilities_mut().push(Ability::static_ability(
        StaticAbility::protection(ProtectionFrom::Color(ColorSet::RED))));
    target
}
fn protection(game: &GameState, target: ObjectId, color: ColorSet) {
    assert!(game.calculated_characteristics(target).unwrap().static_abilities.iter()
        .any(|ability| ability.protection_from() == Some(&ProtectionFrom::Color(color))));
}
fn zone(game: &GameState, stable: StableId) -> Zone {
    game.object(game.find_object_by_stable_id(stable).unwrap()).unwrap().zone
}
fn expire(game: &mut GameState) {
    game.effect_store.continuous_effects.cleanup_end_of_turn();
    game.refresh_continuous_state().unwrap();
}
fn confirm_mana(context: &ManaPaymentContext) -> ironsmith::mana_payment::ManaPaymentResponse {
    ironsmith::mana_payment::ManaPaymentResponse::Confirm { plan_id: context.plan.id, request_hash: context.plan.request_hash }
}

#[derive(Default)]
struct Announce {
    targets: Vec<Target>,
    optional: Option<usize>,
    payment_object: Option<ObjectId>,
    target_groups: Vec<(usize, Option<usize>)>,
}
impl DecisionMaker for Announce {
    fn decide_targets(&mut self, _: &GameState, context: &TargetsContext) -> Vec<Target> {
        self.target_groups = context.requirements.iter().map(|requirement| (requirement.min_targets, requirement.max_targets)).collect();
        assert!(self.targets.iter().all(|target| context.requirements.iter().any(|requirement| requirement.legal_targets.contains(target))));
        self.targets.clone()
    }
    fn decide_options(&mut self, game: &GameState, context: &SelectOptionsContext) -> Vec<usize> {
        // The frozen optional-cost menus have min=0. Mode menus require one
        // mode and precede them. Their typed cardinalities suffice here; no
        // choice or assertion is selected by a presentation description.
        if context.min == 0 {
            return self.optional.iter().map(|index| {
                assert!(context.options.iter().any(|option| option.index == *index && option.legal));
                *index
            }).collect();
        }
        SelectFirstDecisionMaker.decide_options(game, context)
    }
    fn decide_objects(&mut self, game: &GameState, context: &SelectObjectsContext) -> Vec<ObjectId> {
        if let Some(id) = self.payment_object {
            assert!(context.candidates.iter().any(|candidate| candidate.id == id && candidate.legal));
            return vec![id];
        }
        SelectFirstDecisionMaker.decide_objects(game, context)
    }
    fn decide_mana_payment(&mut self, _: &GameState, context: &ManaPaymentContext) -> ironsmith::mana_payment::ManaPaymentResponse { confirm_mana(context) }
}
fn announce(game: &mut GameState, action: LegalAction, dm: &mut Announce) {
    let mut state = PriorityLoopState::new(game.players.len());
    let mut queue = TriggerQueue::new();
    let mut progress = apply_priority_response_with_dm(game, &mut queue, &mut state, &PriorityResponse::PriorityAction(action), dm).unwrap();
    for _ in 0..64 {
        if state.pending_cast.is_none() && state.pending_activation.is_none() { return; }
        let GameProgress::NeedsDecisionCtx(context) = progress else { panic!("{progress:?}"); };
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &context, dm).unwrap();
    }
    panic!("announcement remained incomplete");
}
fn cast_action(game: &GameState, id: ObjectId, method: &CastingMethod) -> Option<LegalAction> {
    compute_legal_actions(game, A).unwrap().into_iter().find(|action| matches!(action,
        LegalAction::CastSpell { spell_id, casting_method, .. } if *spell_id == id && casting_method == method))
}
fn cast(game: &mut GameState, definition: &CardDefinition, origin: Zone, method: CastingMethod, dm: &mut Announce) -> StableId {
    game.turn.priority_player = Some(A);
    let id = game.create_object_from_definition(definition, A, origin);
    let stable = game.object(id).unwrap().stable_id;
    let action = cast_action(game, id, &method).expect("the exact requested casting method is legal");
    announce(game, action, dm);
    assert_eq!(zone(game, stable), Zone::Stack);
    stable
}

#[derive(Default)]
struct Resolve {
    options: VecDeque<(usize, usize)>,
    booleans: VecDeque<bool>,
    objects: Option<ObjectId>,
    targets: Vec<Target>,
}
impl Resolve {
    fn color(from: Color, to: Color, choose_family: bool) -> Self {
        let mut options = VecDeque::new();
        if choose_family { options.push_back((2, 0)); }
        options.push_back((Color::ALL.len(), Color::ALL.iter().position(|color| *color == from).unwrap()));
        options.push_back((Color::ALL.len() - 1, Color::ALL.iter().filter(|color| **color != from).position(|color| *color == to).unwrap()));
        Self { options, ..Default::default() }
    }
    fn land(from: Subtype, to: Subtype, choose_family: bool) -> Self {
        let lands = [Subtype::Plains, Subtype::Island, Subtype::Swamp, Subtype::Mountain, Subtype::Forest];
        let mut options = VecDeque::new();
        if choose_family { options.push_back((2, 1)); }
        options.push_back((5, lands.iter().position(|land| *land == from).unwrap()));
        options.push_back((4, lands.iter().filter(|land| **land != from).position(|land| *land == to).unwrap()));
        Self { options, ..Default::default() }
    }
}
impl DecisionMaker for Resolve {
    fn decide_options(&mut self, _: &GameState, context: &SelectOptionsContext) -> Vec<usize> {
        let (count, index) = self.options.pop_front().expect("every resolution word choice is authored explicitly");
        assert_eq!((context.min, context.max, context.options.len()), (1, 1, count));
        assert!(context.options.iter().any(|option| option.index == index && option.legal));
        vec![index]
    }
    fn decide_boolean(&mut self, _: &GameState, _: &BooleanContext) -> bool { self.booleans.pop_front().expect("explicit optional instruction decision") }
    fn decide_objects(&mut self, game: &GameState, context: &SelectObjectsContext) -> Vec<ObjectId> {
        if let Some(id) = self.objects {
            assert!(context.candidates.iter().any(|candidate| candidate.id == id && candidate.legal));
            return vec![id];
        }
        SelectFirstDecisionMaker.decide_objects(game, context)
    }
    fn decide_targets(&mut self, _: &GameState, context: &TargetsContext) -> Vec<Target> {
        assert!(self.targets.iter().all(|target| context.requirements.iter().any(|requirement| requirement.legal_targets.contains(target))));
        self.targets.clone()
    }
    fn decide_mana_payment(&mut self, _: &GameState, context: &ManaPaymentContext) -> ironsmith::mana_payment::ManaPaymentResponse { confirm_mana(context) }
}
fn resolve(game: &mut GameState, choices: &mut Resolve) { resolve_stack_entry_with(game, choices).unwrap(); }

#[test]
fn indefinite_color_bodies_pay_their_actual_cost_and_survive_cleanup() {
    for name in ["Alter Reality", "Glamerdye", "Sleight of Mind"] {
        for definition in bodies(name) {
            let mut game = game();
            let target = protected(&mut game, B);
            let before = game.player(A).unwrap().mana_pool.total();
            let mut announcement = Announce { targets: vec![Target::Object(target)], ..Default::default() };
            let spell = cast(&mut game, &definition, Zone::Hand, CastingMethod::Normal, &mut announcement);
            assert_eq!(before - game.player(A).unwrap().mana_pool.total(), definition.card.mana_cost.as_ref().unwrap().mana_value());
            let mut choices = Resolve::color(Color::Red, Color::Blue, false);
            resolve(&mut game, &mut choices);
            assert!(choices.options.is_empty());
            assert_eq!(zone(&game, spell), Zone::Graveyard);
            protection(&game, target, ColorSet::BLUE);
            expire(&mut game);
            protection(&game, target, ColorSet::BLUE);
        }
    }
}

#[test]
fn flashback_exiles_and_retrace_pays_a_real_land_discard() {
    for name in ["Alter Reality", "Glamerdye"] {
        for definition in bodies(name) {
            let flashback = definition.alternative_casts.iter().position(|method| matches!(method, AlternativeCastingMethod::Flashback { .. }));
            let index = flashback.or_else(|| definition.alternative_casts.iter().position(|method| matches!(method, AlternativeCastingMethod::Retrace { .. }))).unwrap();
            let mut game = game();
            let target = protected(&mut game, B);
            let land = object(&mut game, A, Zone::Hand, vec![CardType::Land], vec![Subtype::Forest]);
            let land_stable = game.object(land).unwrap().stable_id;
            let before = game.player(A).unwrap().mana_pool.total();
            let mut announcement = Announce { targets: vec![Target::Object(target)], payment_object: Some(land), ..Default::default() };
            let spell = cast(&mut game, &definition, Zone::Graveyard, CastingMethod::Alternative(index), &mut announcement);
            assert_eq!(before - game.player(A).unwrap().mana_pool.total(), 2);
            assert_eq!(zone(&game, land_stable), if flashback.is_some() { Zone::Hand } else { Zone::Graveyard });
            resolve(&mut game, &mut Resolve::color(Color::Red, Color::Blue, false));
            assert_eq!(zone(&game, spell), if flashback.is_some() { Zone::Exile } else { Zone::Graveyard });
            protection(&game, target, ColorSet::BLUE);
            if flashback.is_none() {
                let spell = game.find_object_by_stable_id(spell).unwrap();
                game.turn.priority_player = Some(A);
                assert!(cast_action(&game, spell, &CastingMethod::Alternative(index)).is_none(), "retrace cannot reuse a missing discard payment");
            }
        }
    }
}

#[test]
fn land_word_bodies_change_type_and_intrinsic_mana_without_changing_the_name() {
    for name in ["Magical Hack", "Mind Bend"] {
        for definition in bodies(name) {
            let mut game = game();
            let target = object(&mut game, B, Zone::Battlefield, vec![CardType::Land], vec![Subtype::Forest]);
            let original_name = game.object(target).unwrap().name.clone();
            cast(&mut game, &definition, Zone::Hand, CastingMethod::Normal,
                &mut Announce { targets: vec![Target::Object(target)], ..Default::default() });
            resolve(&mut game, &mut Resolve::land(Subtype::Forest, Subtype::Island, name == "Mind Bend"));
            let current = game.calculated_characteristics(target).unwrap();
            assert_eq!(current.subtypes.to_vec(), vec![Subtype::Island]);
            assert_eq!(current.name, original_name);
            assert!(current.abilities.iter().any(|ability| matches!(&ability.kind, AbilityKind::Activated(ability)
                if ability.mana_output == Some(vec![ManaSymbol::Blue]))));
            expire(&mut game);
            assert_eq!(game.calculated_characteristics(target).unwrap().subtypes.to_vec(), vec![Subtype::Island]);
        }
    }
}

#[test]
fn crystal_spray_draws_after_success_expires_and_draws_nothing_with_its_only_target_gone() {
    for definition in bodies("Crystal Spray") { for leave in [false, true] {
        let mut game = game();
        let target = protected(&mut game, B);
        let draw = object(&mut game, A, Zone::Library, vec![CardType::Instant], Vec::new());
        let drawn = game.object(draw).unwrap().stable_id;
        cast(&mut game, &definition, Zone::Hand, CastingMethod::Normal,
            &mut Announce { targets: vec![Target::Object(target)], ..Default::default() });
        if leave { game.move_object(target, Zone::Graveyard, ironsmith::events::cause::EventCause::effect()).unwrap(); }
        let mut choices = if leave { Resolve::default() } else { Resolve::color(Color::Red, Color::Blue, true) };
        resolve(&mut game, &mut choices);
        assert_eq!(zone(&game, drawn), if leave { Zone::Library } else { Zone::Hand });
        if !leave {
            protection(&game, target, ColorSet::BLUE);
            expire(&mut game);
            protection(&game, target, ColorSet::RED);
        }
    } }
}

#[test]
fn artificial_evolution_accepts_wall_as_source_and_preserves_name_and_other_characteristics() {
    for definition in bodies("Artificial Evolution") {
        let mut game = game();
        let target = object(&mut game, B, Zone::Battlefield, vec![CardType::Creature], vec![Subtype::Wall]);
        let before = game.calculated_characteristics(target).unwrap();
        cast(&mut game, &definition, Zone::Hand, CastingMethod::Normal,
            &mut Announce { targets: vec![Target::Object(target)], ..Default::default() });
        let types = Subtype::all_creature_types();
        let mut choices = Resolve::default();
        choices.options.push_back((types.len(), types.iter().position(|kind| *kind == Subtype::Wall).unwrap()));
        choices.options.push_back((types.len() - 1, types.iter().filter(|kind| **kind != Subtype::Wall).position(|kind| *kind == Subtype::Human).unwrap()));
        resolve(&mut game, &mut choices);
        let current = game.calculated_characteristics(target).unwrap();
        assert_eq!(current.subtypes.to_vec(), vec![Subtype::Human]);
        assert_eq!(current.name, before.name);
        assert_eq!((current.colors, current.power, current.toughness), (before.colors, before.power, before.toughness));
    }
}

#[test]
fn new_blood_pays_vampire_tap_then_rechecks_creature_before_control_and_word_choice() {
    for definition in bodies("New Blood") { for invalid in [false, true] { for identity in [false, true] {
        let mut game = game();
        let target = protected(&mut game, B);
        let payer = object(&mut game, A, Zone::Battlefield, vec![CardType::Creature], vec![Subtype::Vampire]);
        cast(&mut game, &definition, Zone::Hand, CastingMethod::Normal,
            &mut Announce { targets: vec![Target::Object(target)], payment_object: Some(payer), ..Default::default() });
        assert!(game.is_tapped(payer), "the additional cost is paid before resolution");
        if invalid { game.object_mut(target).unwrap().card_types = vec![CardType::Artifact].into(); game.refresh_continuous_state().unwrap(); }
        let mut choices = Resolve::default();
        if !invalid {
            let types = Subtype::all_creature_types();
            let source = if identity { Subtype::Vampire } else { Subtype::Human };
            choices.options.push_back((types.len(), types.iter().position(|kind| *kind == source).unwrap()));
        }
        resolve(&mut game, &mut choices);
        assert_eq!(game.current_controller(target), Some(if invalid { B } else { A }));
        let expected = if invalid || identity { Subtype::Human } else { Subtype::Vampire };
        assert_eq!(game.calculated_characteristics(target).unwrap().subtypes.to_vec(), vec![expected]);
        assert!(game.is_tapped(payer), "a failed target recheck does not refund the paid tap");
        assert!(choices.options.is_empty());
    } } }
}

#[test]
fn entwine_pays_two_extra_and_resolves_independent_land_and_color_targets_in_order() {
    for definition in bodies("Spectral Shift") {
        let mut game = game();
        let land = object(&mut game, B, Zone::Battlefield, vec![CardType::Land], vec![Subtype::Forest]);
        let colored = protected(&mut game, B);
        let entwine = definition.optional_costs.iter().position(|cost| cost.kind == ironsmith_core::OptionalCostKind::Entwine).unwrap();
        let before = game.player(A).unwrap().mana_pool.total();
        let mut announcement = Announce { targets: vec![Target::Object(land), Target::Object(colored)], optional: Some(entwine), ..Default::default() };
        cast(&mut game, &definition, Zone::Hand, CastingMethod::Normal, &mut announcement);
        assert_eq!(before - game.player(A).unwrap().mana_pool.total(), 4);
        assert_eq!(game.stack.last().unwrap().chosen_modes, Some(vec![0, 1]));
        assert_eq!(announcement.target_groups, vec![(1, Some(1)), (1, Some(1))]);
        let mut choices = Resolve::land(Subtype::Forest, Subtype::Island, false);
        choices.options.extend(Resolve::color(Color::Red, Color::Blue, false).options);
        resolve(&mut game, &mut choices);
        assert!(choices.options.is_empty());
        assert_eq!(game.calculated_characteristics(land).unwrap().subtypes.to_vec(), vec![Subtype::Island]);
        protection(&game, colored, ColorSet::BLUE);
        expire(&mut game);
        assert_eq!(game.calculated_characteristics(land).unwrap().subtypes.to_vec(), vec![Subtype::Island]);
        protection(&game, colored, ColorSet::BLUE);
    }
}

#[test]
fn whim_buyback_returns_the_paid_card_but_its_text_change_still_expires() {
    for definition in bodies("Whim of Volrath") { for pay in [false, true] {
        let mut game = game();
        let target = protected(&mut game, B);
        let buyback = definition.optional_costs.iter().position(|cost| cost.kind == ironsmith_core::OptionalCostKind::Buyback).unwrap();
        let before = game.player(A).unwrap().mana_pool.total();
        let spell = cast(&mut game, &definition, Zone::Hand, CastingMethod::Normal,
            &mut Announce { targets: vec![Target::Object(target)], optional: pay.then_some(buyback), ..Default::default() });
        assert_eq!(before - game.player(A).unwrap().mana_pool.total(), if pay { 3 } else { 1 });
        resolve(&mut game, &mut Resolve::color(Color::Red, Color::Blue, true));
        assert_eq!(zone(&game, spell), if pay { Zone::Hand } else { Zone::Graveyard });
        protection(&game, target, ColorSet::BLUE);
        expire(&mut game);
        protection(&game, target, ColorSet::RED);
    } }
}

fn stack_event(game: &mut GameState, event: TriggerEvent, choices: &mut Resolve) {
    let mut queue = TriggerQueue::new();
    for trigger in check_triggers(game, &event) { queue.add(trigger); }
    put_triggers_on_stack_with_dm(game, &mut queue, choices).unwrap();
}

#[test]
fn shaman_pays_tap_changes_white_enchantment_and_grants_complete_independent_upkeep() {
    for definition in bodies("Balduvian Shaman") {
        let mut game = game();
        let card = CardBuilder::new(CardId::new(), "White enchantment witness")
            .card_types(vec![CardType::Enchantment]).color_indicator(ColorSet::WHITE).build();
        let target = game.create_object_from_card(&card, A, Zone::Battlefield);
        game.object_mut(target).unwrap().abilities_mut().push(Ability::static_ability(
            StaticAbility::protection(ProtectionFrom::Color(ColorSet::RED))));
        let stable = game.object(target).unwrap().stable_id;
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        game.remove_summoning_sickness(source);
        let index = game.current_abilities(source).unwrap().iter().position(|ability| matches!(ability.kind, AbilityKind::Activated(_))).unwrap();
        let action = compute_legal_actions(&game, A).unwrap().into_iter().find(|action| matches!(action,
            LegalAction::ActivateAbility { source: id, ability_index } if *id == source && *ability_index == index)).unwrap();
        announce(&mut game, action, &mut Announce { targets: vec![Target::Object(target)], ..Default::default() });
        assert!(game.is_tapped(source));
        resolve(&mut game, &mut Resolve::color(Color::Red, Color::Blue, false));
        protection(&game, target, ColorSet::BLUE);
        assert!(game.current_abilities(target).unwrap().iter().any(|ability| matches!(&ability.kind, AbilityKind::Triggered(triggered)
            if triggered.effects.all_effects().iter().any(|effect| effect.downcast_ref::<ironsmith::effects::CumulativeUpkeepEffect>().is_some()))));
        game.turn.phase = Phase::Beginning;
        game.turn.step = Some(ironsmith::game_state::Step::Upkeep);
        let mut pay = Resolve { booleans: VecDeque::from([true]), ..Default::default() };
        let before = game.player(A).unwrap().mana_pool.total();
        stack_event(&mut game, TriggerEvent::new(ironsmith::events::phase::BeginningOfUpkeepEvent::new(A), Default::default()), &mut pay);
        assert_eq!(game.stack.len(), 1);
        resolve(&mut game, &mut pay);
        assert_eq!(game.counter_count(target, CounterType::Age), 1);
        assert_eq!(before - game.player(A).unwrap().mana_pool.total(), 1);
        assert_eq!(zone(&game, stable), Zone::Battlefield);
        expire(&mut game);
        protection(&game, target, ColorSet::BLUE);
        let mut decline = Resolve { booleans: VecDeque::from([false]), ..Default::default() };
        stack_event(&mut game, TriggerEvent::new(ironsmith::events::phase::BeginningOfUpkeepEvent::new(A), Default::default()), &mut decline);
        resolve(&mut game, &mut decline);
        assert_eq!(zone(&game, stable), Zone::Graveyard, "declining the later cumulative payment sacrifices the enchantment");
    }
}

#[test]
fn cipher_encodes_real_card_casts_one_free_copy_and_keeps_the_grant_beyond_text_expiry() {
    for definition in bodies("Trait Doctoring") { for combat in [false, true] {
        let mut game = game();
        let target = protected(&mut game, B);
        let bearer = object(&mut game, A, Zone::Battlefield, vec![CardType::Creature], vec![Subtype::Human]);
        let encoded = cast(&mut game, &definition, Zone::Hand, CastingMethod::Normal,
            &mut Announce { targets: vec![Target::Object(target)], ..Default::default() });
        let mut encode = Resolve::color(Color::Red, Color::Blue, true);
        encode.booleans.push_back(true);
        encode.objects = Some(bearer);
        resolve(&mut game, &mut encode);
        assert_eq!(zone(&game, encoded), Zone::Exile);
        protection(&game, target, ColorSet::BLUE);
        assert_eq!(game.current_abilities(bearer).unwrap().iter().filter(|ability| matches!(ability.kind, AbilityKind::Triggered(_))).count(), 1);
        let before = game.player(A).unwrap().mana_pool.total();
        let mut cast_copy = Resolve { booleans: VecDeque::from([true]), targets: vec![Target::Object(target)], ..Default::default() };
        let damage = ironsmith::events::DamageEvent::with_cause(bearer, ironsmith::events::DamageTarget::Player(B), 1, combat,
            if combat { ironsmith::events::cause::EventCause::combat_damage(bearer) } else { ironsmith::events::cause::EventCause::effect() });
        stack_event(&mut game, TriggerEvent::new_with_provenance(damage, Default::default()), &mut cast_copy);
        assert_eq!(game.stack.len(), usize::from(combat));
        if combat {
            resolve(&mut game, &mut cast_copy);
            assert_eq!(game.stack.len(), 1, "resolving the granted trigger casts the encoded copy");
            assert_eq!(game.player(A).unwrap().mana_pool.total(), before);
            let mut copy_words = Resolve::color(Color::Blue, Color::Green, true);
            resolve(&mut game, &mut copy_words);
            assert!(copy_words.options.is_empty());
            assert!(game.stack.is_empty());
            protection(&game, target, ColorSet::GREEN);
        }
        assert_eq!(game.exile.len(), 1, "the spell copy never asks to encode again");
        expire(&mut game);
        protection(&game, target, ColorSet::RED);
        assert_eq!(game.current_abilities(bearer).unwrap().iter().filter(|ability| matches!(ability.kind, AbilityKind::Triggered(_))).count(), 1);
        let encoded_id = game.find_object_by_stable_id(encoded).unwrap();
        game.move_object(encoded_id, Zone::Graveyard, ironsmith::events::cause::EventCause::effect()).unwrap();
        game.refresh_continuous_state().unwrap();
        assert!(!game.current_abilities(bearer).unwrap().iter().any(|ability| matches!(ability.kind, AbilityKind::Triggered(_))));
    } }
}
