//! Complete frozen bodies and paid gameplay evidence. Authored UNRUN; no accounting credit.
use ironsmith::ability::AbilityKind;
use ironsmith::card::{CardBuilder, PowerToughness};
use ironsmith::cards::CardDefinition;
use ironsmith::continuous::{EffectTarget, Modification};
use ironsmith::decision::{DecisionMaker, LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
use ironsmith::decisions::context::ManaPaymentContext;
use ironsmith::effect::Until;
use ironsmith::effects::{ApplyContinuousEffect, EffectContext, EffectExecutor};
use ironsmith::events::cause::EventCause;
use ironsmith::events::{DamagePreventedEvent, DamageTarget};
use ironsmith::events::processing::process_damage_assignments_with_event_with_source_snapshot_opts;
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, resolve_stack_entry_with};
use ironsmith::mana::{ManaCost, ManaSymbol};
use ironsmith::mana_payment::ManaPaymentResponse;
use ironsmith::static_abilities::{StaticAbility, StaticAbilityId};
use ironsmith::triggers::TriggerQueue;
use ironsmith::{CardId, CardType, GameProgress, GameState, ObjectId, PlayerId, Subtype, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};

const A: PlayerId = PlayerId::from_index(0);
const B: PlayerId = PlayerId::from_index(1);
const C: PlayerId = PlayerId::from_index(2);
const NAMES: [&str; 2] = ["Glittering Lion", "Glittering Lynx"];
fn price(name: &str) -> u32 { if name == "Glittering Lion" { 3 } else { 2 } }
fn definitions(name: &str) -> [CardDefinition; 2] {
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../fixtures/glittering_prevention_bodies.json.fixture")).unwrap();
    assert_eq!(rows.len(), 2);
    let row = rows.iter().find(|row| row["name"] == name).unwrap();
    let lion = name == "Glittering Lion";
    assert_eq!(row["oracle_id"], if lion { "549e6de7-56e9-4f5c-8c88-30e446bc53bb" }
        else { "890ffe31-642f-46e3-9f09-c744351653b5" });
    assert_eq!(row["type_line"], "Creature — Cat");
    assert_eq!(row["mana_cost"], if lion { "{2}{W}" } else { "{W}" });
    assert_eq!(row["power"], if lion { "2" } else { "1" });
    assert_eq!(row["toughness"], if lion { "2" } else { "1" });
    assert_eq!(row["oracle_text"], format!("Prevent all damage that would be dealt to this creature.\n{{{}}}: Until end of turn, this creature loses \"Prevent all damage that would be dealt to this creature.\" Any player may activate this ability.", price(name)));
    let text = format!("Mana cost: {}\nType: {}\nPower/Toughness: {}/{}\n{}",
        row["mana_cost"].as_str().unwrap(), row["type_line"].as_str().unwrap(),
        row["power"].as_str().unwrap(), row["toughness"].as_str().unwrap(),
        row["oracle_text"].as_str().unwrap());
    // Neither route consumes the other's definition or falls back to Oracle-only input.
    let (direct, loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_runtime_definition(name, &text, false));
    let direct = direct.unwrap();
    assert!(!loss.is_lossy(), "direct {name}: {}", loss.reasons_text());
    let (artifact, loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_artifact(name, &text, false));
    let (artifact, _) = artifact.unwrap();
    assert!(!loss.is_lossy(), "artifact {name}: {}", loss.reasons_text());
    artifact.validate().unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    restored.validate().unwrap();
    assert_eq!(artifact, restored);
    let definitions = [direct, ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap()];
    for definition in &definitions {
        assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(definition));
        assert_eq!(definition.card.name, name);
        assert_eq!(definition.card.card_types, vec![CardType::Creature]);
        assert_eq!(definition.card.subtypes, vec![Subtype::Cat]);
        assert!(definition.card.supertypes.is_empty());
        assert_eq!(definition.card.power_toughness, Some(PowerToughness::fixed(if lion {2} else {1}, if lion {2} else {1})));
        assert_eq!(definition.card.mana_cost, Some(ManaCost::from_symbols(if lion {
            vec![ManaSymbol::Generic(2), ManaSymbol::White]
        } else { vec![ManaSymbol::White] })));
        assert!(definition.spell_effect.is_none());
        assert_eq!(definition.abilities.len(), 2);
        assert_eq!(definition.abilities.iter().filter(|ability| matches!(&ability.kind,
            AbilityKind::Static(s) if s.id() == StaticAbilityId::PreventAllDamageToSelf)).count(), 1);
        let activated = definition.abilities.iter().find_map(|ability| match &ability.kind {
            AbilityKind::Activated(a) => Some(a), _ => None,
        }).unwrap();
        assert!(activated.allows_any_player_to_activate());
    }
    definitions
}
fn game() -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Charlie".into()], 20);
    game.turn.active_player = A;
    game.turn.phase = ironsmith::Phase::FirstMain;
    game.turn.priority_player = Some(B);
    game
}
fn witness(game: &mut GameState, owner: PlayerId, zone: Zone) -> ObjectId {
    game.create_object_from_card(&CardBuilder::new(CardId::new(), "Independent witness")
        .card_types(vec![CardType::Creature]).power_toughness(PowerToughness::fixed(2, 8)).build(), owner, zone)
}
fn fund(game: &mut GameState, payer: PlayerId, amount: u32) {
    game.player_mut(payer).unwrap().mana_pool.add(ManaSymbol::Colorless, amount);
}
fn action(game: &mut GameState, payer: PlayerId, source: ObjectId) -> Option<LegalAction> {
    game.turn.priority_player = Some(payer);
    compute_legal_actions(game, payer).unwrap().into_iter().find(|action|
        matches!(action, LegalAction::ActivateAbility { source: id, .. } if *id == source))
}
#[derive(Default)]
struct Choices { cancel: bool, payments: Vec<PlayerId> }
impl DecisionMaker for Choices {
    fn decide_mana_payment(&mut self, _: &GameState, context: &ManaPaymentContext) -> ManaPaymentResponse {
        self.payments.push(context.player);
        if self.cancel { ManaPaymentResponse::Cancel } else {
            ManaPaymentResponse::Confirm { plan_id: context.plan.id, request_hash: context.plan.request_hash }
        }
    }
}
fn activate(game: &mut GameState, payer: PlayerId, host: ObjectId, choices: &mut Choices) {
    let amount = price(&game.object(host).unwrap().name.to_string());
    let source_controller = game.controller_of(game.object(host).unwrap());
    let action = action(game, payer, host).expect("the printed activation must be legal");
    let mut queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(3);
    let mut progress = apply_priority_response_with_dm(game, &mut queue, &mut state,
        &PriorityResponse::PriorityAction(action), choices).unwrap();
    for _ in 0..64 {
        if !state.has_pending_action() { break; }
        let GameProgress::NeedsDecisionCtx(context) = progress else { panic!("{progress:?}"); };
        // Rehydrate the native pending-action owner, not only the visible game.
        *game = game.clone();
        state = state.clone();
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &context, choices).unwrap();
    }
    assert!(!state.has_pending_action());
    assert!(!choices.payments.is_empty(), "a real payment decision was required");
    assert!(choices.payments.iter().all(|player| *player == payer));
    if !choices.cancel {
        assert_paid_receipt(game, payer, amount);
        assert_eq!(game.stack[0].object_id, host);
        assert_eq!(game.stack[0].source_snapshot.as_ref().unwrap().controller, source_controller);
        if payer != source_controller {
            assert_ne!(game.stack[0].controller,
                game.stack[0].source_snapshot.as_ref().unwrap().controller,
                "the opponent's paid activation is distinct from its source controller");
        }
    } else {
        assert!(game.stack.is_empty(), "cancelled payment leaves no committed activation receipt");
    }
}
fn assert_paid_receipt(game: &GameState, payer: PlayerId, amount: u32) {
    assert_eq!(game.stack.len(), 1);
    let entry = &game.stack[0];
    assert!(entry.is_ability);
    assert_eq!(entry.controller, payer, "the actual activator owns this paid stack entry");
    assert!(!entry.activation_cost_has_tap);
    assert!(!entry.activation_cost_has_x);
    // Read the receipt accumulated by priority_mana and committed by priority_cast.
    // Pool deltas and prompts above are independent evidence, not a substitute for it.
    assert_eq!(entry.mana_spent_on_activation.total(), amount);
    assert_eq!(entry.mana_spent_on_activation.amount(ManaSymbol::Colorless), amount);
    for color in [ManaSymbol::White, ManaSymbol::Blue, ManaSymbol::Black,
        ManaSymbol::Red, ManaSymbol::Green] {
        assert_eq!(entry.mana_spent_on_activation.amount(color), 0);
    }
}
fn resolve(game: &mut GameState, payer: PlayerId, amount: u32) {
    assert_paid_receipt(game, payer, amount);
    *game = game.clone();
    assert_paid_receipt(game, payer, amount);
    resolve_stack_entry_with(game, &mut SelectFirstDecisionMaker).unwrap();
    assert!(game.stack.is_empty());
}
fn damage(game: &mut GameState, source: ObjectId, target: DamageTarget, amount: u32,
    combat: bool, unpreventable: bool) -> (u32, Vec<(u32, ObjectId, PlayerId)>) {
    game.take_pending_trigger_events();
    let result = process_damage_assignments_with_event_with_source_snapshot_opts(game, source,
        target, amount, combat, unpreventable, EventCause::effect(), None).unwrap();
    let remaining = result.assignments.iter().map(|a| a.amount).sum();
    let events = game.take_pending_trigger_events().iter().filter_map(|event|
        event.downcast::<DamagePreventedEvent>().map(|event| {
            assert!(event.prevention_shield.is_none(), "native static replacement, not a one-shot shield");
            assert_eq!(event.target, target);
            assert_eq!(event.damage_source, source);
            (event.amount, event.prevention_source, event.prevention_controller)
        })).collect();
    (remaining, events)
}
fn has_static(game: &GameState, host: ObjectId, id: StaticAbilityId) -> bool {
    game.current_abilities(host).unwrap().iter().any(|ability|
        matches!(&ability.kind, AbilityKind::Static(s) if s.id() == id))
}

#[test]
fn complete_bodies_prevent_repeatable_exact_self_damage_with_native_events() {
    for name in NAMES { for definition in definitions(name) {
        let mut game = game();
        let host = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let other = game.create_object_from_definition(&definition, B, Zone::Battlefield);
        let blank = witness(&mut game, A, Zone::Battlefield);
        for owner in [A, B, C] { for zone in [Zone::Battlefield, Zone::Stack, Zone::Graveyard] {
            let source = witness(&mut game, owner, zone);
            for (amount, combat) in [(1, false), (7, true), (3, false)] {
                assert_eq!(damage(&mut game, source, DamageTarget::Object(host), amount, combat, false), (0, vec![(amount, host, A)]));
                assert_eq!(damage(&mut game, source, DamageTarget::Object(other), amount, combat, false), (0, vec![(amount, other, B)]));
                assert_eq!(damage(&mut game, source, DamageTarget::Object(blank), amount, combat, false), (amount, vec![]));
                for recipient in [A, B, C] {
                    assert_eq!(damage(&mut game, source, DamageTarget::Player(recipient), amount, combat, false), (amount, vec![]));
                }
                assert_eq!(damage(&mut game, source, DamageTarget::Object(host), amount, combat, true), (amount, vec![]));
            }
        } }
        assert!(game.stack.is_empty());
    } }
}

#[test]
fn opponent_pays_exact_cost_removes_only_quoted_rule_and_cleanup_restores_it() {
    for name in NAMES { for definition in definitions(name) { for payer in [A, B, C] {
        let mut game = game();
        let host = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let other = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let source = witness(&mut game, B, Zone::Battlefield);
        // Independent ability is a control, never a replacement for either printed clause.
        ApplyContinuousEffect::new(EffectTarget::Specific(host),
            Modification::AddAbility(StaticAbility::flying()), Until::Forever)
            .execute(&mut game, &mut EffectContext::new_default(source, A)).unwrap();
        fund(&mut game, payer, price(name));
        let before = game.clone();
        activate(&mut game, payer, host, &mut Choices::default());
        assert_eq!(game.player(payer).unwrap().mana_pool.total(), 0);
        assert!(!game.is_tapped(host), "no tap cost and no summoning-sickness restriction");
        assert_eq!(damage(&mut game, source, DamageTarget::Object(host), 4, false, false), (0, vec![(4, host, A)]), "announcing is not resolving");
        resolve(&mut game, payer, price(name));
        assert!(!has_static(&game, host, StaticAbilityId::PreventAllDamageToSelf));
        assert!(has_static(&game, host, StaticAbilityId::Flying), "not lose all abilities");
        for combat in [false, true, false] {
            assert_eq!(damage(&mut game, source, DamageTarget::Object(host), 4, combat, false), (4, vec![]));
            assert_eq!(damage(&mut game, source, DamageTarget::Object(other), 4, combat, false), (0, vec![(4, other, A)]));
        }
        // Repeating proves the quoted static was removed, not the activation itself.
        fund(&mut game, payer, price(name));
        activate(&mut game, payer, host, &mut Choices::default());
        resolve(&mut game, payer, price(name));
        assert_eq!(game.player(payer).unwrap().mana_pool.total(), 0);
        game = game.clone();
        assert_eq!(damage(&mut game, source, DamageTarget::Object(host), 2, true, false), (2, vec![]));
        ironsmith::turn::execute_cleanup_step(&mut game);
        assert!(has_static(&game, host, StaticAbilityId::Flying));
        assert_eq!(damage(&mut game, source, DamageTarget::Object(host), 5, false, false), (0, vec![(5, host, A)]));
        // Native checkpoint restore independently recovers both mana and prevention.
        game = before;
        assert_eq!(game.player(payer).unwrap().mana_pool.total(), price(name));
        assert_eq!(damage(&mut game, source, DamageTarget::Object(host), 5, false, false), (0, vec![(5, host, A)]));
    } } }
}

#[test]
fn controller_changes_phasing_and_new_incarnations_do_not_retarget_the_loss() {
    for name in NAMES { for definition in definitions(name) {
        let mut game = game();
        let host = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let source = witness(&mut game, C, Zone::Battlefield);
        fund(&mut game, B, price(name));
        activate(&mut game, B, host, &mut Choices::default());
        game.set_current_controller(host, C).unwrap();
        assert_eq!(damage(&mut game, source, DamageTarget::Object(host), 3, false, false), (0, vec![(3, host, C)]));
        resolve(&mut game, B, price(name));
        assert_eq!(damage(&mut game, source, DamageTarget::Object(host), 3, false, false), (3, vec![]));
        game.set_current_controller(host, A).unwrap();
        game.phase_out(host);
        fund(&mut game, B, price(name));
        assert!(action(&mut game, B, host).is_none());
        game.phase_in(host);
        assert_eq!(damage(&mut game, source, DamageTarget::Object(host), 3, true, false), (3, vec![]));
        ironsmith::turn::execute_cleanup_step(&mut game);
        assert_eq!(damage(&mut game, source, DamageTarget::Object(host), 3, true, false), (0, vec![(3, host, A)]));
        fund(&mut game, B, price(name));
        activate(&mut game, B, host, &mut Choices::default());
        let grave = game.move_object_by_effect(host, Zone::Graveyard).unwrap();
        assert!(action(&mut game, B, grave).is_none());
        let returned = game.move_object_by_effect(grave, Zone::Battlefield).unwrap();
        assert_ne!(host, returned);
        resolve(&mut game, B, price(name));
        assert_eq!(damage(&mut game, source, DamageTarget::Object(returned), 3, false, false), (0, vec![(3, returned, A)]), "old activation must not follow a new incarnation");
        fund(&mut game, C, price(name));
        activate(&mut game, C, returned, &mut Choices::default());
        resolve(&mut game, C, price(name));
        let grave = game.move_object_by_effect(returned, Zone::Graveyard).unwrap();
        let again = game.move_object_by_effect(grave, Zone::Battlefield).unwrap();
        assert_eq!(damage(&mut game, source, DamageTarget::Object(again), 3, false, false), (0, vec![(3, again, A)]), "resolved loss also must not follow a new incarnation");
    } }
}

#[test]
fn payer_legality_and_cancelled_payment_keep_native_prevention_intact() {
    for name in NAMES { for definition in definitions(name) {
        let mut game = game();
        let host = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let source = witness(&mut game, A, Zone::Battlefield);
        fund(&mut game, A, price(name));
        fund(&mut game, B, price(name) - 1);
        assert!(action(&mut game, B, host).is_none(), "controller funds cannot pay an opponent's activation");
        assert!(action(&mut game, A, host).is_some());
        fund(&mut game, B, 1);
        let mut choices = Choices { cancel: true, ..Default::default() };
        activate(&mut game, B, host, &mut choices);
        assert!(game.stack.is_empty());
        assert_eq!(game.player(A).unwrap().mana_pool.total(), price(name));
        assert_eq!(game.player(B).unwrap().mana_pool.total(), price(name));
        assert!(!game.is_tapped(host));
        assert_eq!(damage(&mut game, source, DamageTarget::Object(host), 4, false, false), (0, vec![(4, host, A)]));
        activate(&mut game, B, host, &mut Choices::default());
        assert_eq!(game.player(A).unwrap().mana_pool.total(), price(name));
        assert_eq!(game.player(B).unwrap().mana_pool.total(), 0);
        resolve(&mut game, B, price(name));
        assert_eq!(damage(&mut game, source, DamageTarget::Object(host), 4, false, false), (4, vec![]));
    } }
}

#[test]
fn native_damage_execution_and_stale_activation_rejection_are_observable() {
    for name in NAMES { for definition in definitions(name) {
        let mut game = game();
        let host = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let source = witness(&mut game, C, Zone::Battlefield);
        let hit = ironsmith::Effect::deal_damage(1, ironsmith::target::ChooseSpec::SpecificObject(host));
        for _ in 0..3 {
            ironsmith::effects::execute_effect(&mut game, &hit,
                &mut EffectContext::new_default(source, C)).unwrap();
            assert_eq!(game.damage_on(host), 0);
        }
        fund(&mut game, B, price(name));
        let stale = action(&mut game, B, host).unwrap();
        game.player_mut(B).unwrap().mana_pool.empty();
        let mut state = PriorityLoopState::new(3);
        assert!(apply_priority_response_with_dm(&mut game, &mut TriggerQueue::new(), &mut state,
            &PriorityResponse::PriorityAction(stale), &mut Choices::default()).is_err());
        assert!(!state.has_pending_action());
        assert!(game.stack.is_empty());
        assert_eq!(game.player(B).unwrap().mana_pool.total(), 0);
        assert!(has_static(&game, host, StaticAbilityId::PreventAllDamageToSelf));
        fund(&mut game, B, price(name));
        activate(&mut game, B, host, &mut Choices::default());
        resolve(&mut game, B, price(name));
        ironsmith::effects::execute_effect(&mut game, &hit,
            &mut EffectContext::new_default(source, C)).unwrap();
        assert_eq!(game.damage_on(host), 1, "actual damage is marked after the paid printed loss");
        // No state-based-action pass is requested here: cleanup resets damage and the loss.
        ironsmith::turn::execute_cleanup_step(&mut game);
        assert_eq!(game.damage_on(host), 0);
        ironsmith::effects::execute_effect(&mut game, &hit,
            &mut EffectContext::new_default(source, C)).unwrap();
        assert_eq!(game.damage_on(host), 0);
    } }
}
