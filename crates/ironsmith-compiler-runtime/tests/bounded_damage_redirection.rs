//! Full frozen bodies and runtime scenarios. Authored only; execution is deferred.
use ironsmith::ability::AbilityKind;
use ironsmith::card::{CardBuilder, PowerToughness};
use ironsmith::cards::CardDefinition;
use ironsmith::combat_state::{AttackTarget, CombatState};
use ironsmith::decision::{AttackerDeclaration, BlockerDeclaration, DecisionMaker, LegalAction, SelectFirstDecisionMaker};
use ironsmith::decisions::context::{NumberContext, SelectObjectsContext, SelectOptionsContext, TargetsContext};
use ironsmith::effects::{EffectExecutor, EffectContext as ExecutionContext};
use ironsmith::events::{DamageEvent, DamageTarget};
use ironsmith::events::cause::EventCause;
use ironsmith::events::processing::{SimultaneousDamageEvent, process_simultaneous_damage_assignments_with_event};
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse, apply_attacker_declarations, apply_blocker_declarations, apply_decision_context_with_dm, apply_priority_response_with_dm, put_triggers_on_stack_with_dm, resolve_stack_entry_with};
use ironsmith::game_state::{Phase, Step};
use ironsmith::target::{ChooseSpec, ObjectFilter};
use ironsmith::triggers::TriggerQueue;
use ironsmith::{CardId, CardType, CoinFace, ColorSet, GameState, ManaSymbol, ObjectId, PlayerId, Target, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
const A: PlayerId = PlayerId::from_index(0);
const B: PlayerId = PlayerId::from_index(1);
const C: PlayerId = PlayerId::from_index(2);

fn rows() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!("../../../fixtures/bounded_damage_redirection.json.fixture")).unwrap()
}
fn definitions(name: &str) -> [CardDefinition; 2] {
    let row = rows().into_iter().find(|row| row["name"] == name).unwrap();
    let mut text = format!("Mana cost: {}\nType: {}\n", row["mana_cost"].as_str().unwrap(), row["type_line"].as_str().unwrap());
    if let (Some(p), Some(t)) = (row["power"].as_str(), row["toughness"].as_str()) { text.push_str(&format!("Power/Toughness: {p}/{t}\n")); }
    text.push_str(row["oracle_text"].as_str().unwrap());
    let (result, loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_artifact(name, &text, false));
    let (artifact, _) = result.unwrap_or_else(|error| panic!("{name}: {error}"));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let (direct, direct_loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_runtime_definition(name, &text, false));
    let direct = direct.unwrap_or_else(|error| panic!("direct {name}: {error}"));
    assert!(!direct_loss.is_lossy(), "direct {name}: {}", direct_loss.reasons_text());
    artifact.validate().unwrap();
    let decoded = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, decoded);
    [direct, ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&decoded).unwrap()]
}
fn game() -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Carol".into()], 20);
    game.turn.phase = Phase::FirstMain; game.turn.step = None;
    game.turn.active_player = A; game.turn.priority_player = Some(A);
    for player in [A, B, C] {
        for symbol in [ManaSymbol::White, ManaSymbol::Blue, ManaSymbol::Black, ManaSymbol::Red, ManaSymbol::Green, ManaSymbol::Colorless] {
            game.player_mut(player).unwrap().mana_pool.add(symbol, 20);
        }
    }
    game
}
fn creature(game: &mut GameState, owner: PlayerId) -> ObjectId {
    let card = CardBuilder::new(CardId::new(), "White damage witness").card_types(vec![CardType::Creature])
        .color_indicator(ColorSet::WHITE).power_toughness(PowerToughness::fixed(2, 20)).build();
    let object = game.create_object_from_card(&card, owner, Zone::Battlefield);
    game.remove_summoning_sickness(object); object
}
fn spell(game: &mut GameState, owner: PlayerId) -> ObjectId {
    let card = CardBuilder::new(CardId::new(), "Spell damage witness").card_types(vec![CardType::Instant]).build();
    game.create_object_from_card(&card, owner, Zone::Stack)
}
#[derive(Default)]
struct Choices { targets: Vec<Target>, x: u32, redirect_allocations: Vec<u32>, allocation_players: Vec<PlayerId> }
impl DecisionMaker for Choices {
    fn decide_targets(&mut self, _: &GameState, context: &TargetsContext) -> Vec<Target> {
        assert_eq!(context.requirements.len(), self.targets.len());
        for (requirement, target) in context.requirements.iter().zip(&self.targets) { assert!(requirement.legal_targets.contains(target), "{requirement:?}"); }
        self.targets.clone()
    }
    fn decide_objects(&mut self, game: &GameState, context: &SelectObjectsContext) -> Vec<ObjectId> {
        assert_ne!(context.description, "Choose a source", "this cohort must not introduce a damage-source chooser");
        SelectFirstDecisionMaker.decide_objects(game, context)
    }
    fn decide_number(&mut self, game: &GameState, context: &NumberContext) -> u32 {
        if context.is_x_value { self.x }
        else if context.description.starts_with("Choose how much redirected damage from ") {
            self.allocation_players.push(context.player);
            let chosen = self.redirect_allocations.remove(0);
            assert!(chosen >= context.min && chosen <= context.max);
            chosen
        } else { SelectFirstDecisionMaker.decide_number(game, context) }
    }
    fn decide_options(&mut self, game: &GameState, context: &SelectOptionsContext) -> Vec<usize> {
        SelectFirstDecisionMaker.decide_options(game, context)
    }
    fn decide_mana_payment(&mut self, _: &GameState, context: &ironsmith::decisions::context::ManaPaymentContext) -> ironsmith::mana_payment::ManaPaymentResponse {
        ironsmith::mana_payment::ManaPaymentResponse::Confirm { plan_id: context.plan.id, request_hash: context.plan.request_hash }
    }
}
fn activate(game: &mut GameState, source: ObjectId, definition: &CardDefinition, targets: Vec<Target>, x: u32) {
    let index = definition.abilities.iter().position(|ability| matches!(&ability.kind, AbilityKind::Activated(_))).unwrap();
    let controller = game.current_controller(source).unwrap();
    game.turn.priority_player = Some(controller);
    let mut decisions = Choices { targets, x, ..Default::default() };
    let mut state = PriorityLoopState::new(game.players.len()); let mut queue = TriggerQueue::new();
    let mut progress = apply_priority_response_with_dm(game, &mut queue, &mut state,
        &PriorityResponse::PriorityAction(LegalAction::ActivateAbility { source, ability_index: index }), &mut decisions).unwrap();
    for _ in 0..64 {
        if state.pending_activation.is_none() { break; }
        let ironsmith::GameProgress::NeedsDecisionCtx(context) = progress else { panic!("{progress:?}") };
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &context, &mut decisions).unwrap();
    }
    assert!(state.pending_activation.is_none());
    assert_eq!(game.stack.last().unwrap().targets, decisions.targets);
    resolve_stack_entry_with(game, &mut decisions).unwrap();
}
fn event(source: ObjectId, target: DamageTarget, amount: u32, combat: bool) -> SimultaneousDamageEvent {
    SimultaneousDamageEvent { source, target, amount, is_combat: combat, unpreventable: true,
        cause: if combat { EventCause::combat_damage(source) } else { EventCause::effect() }, source_snapshot: None }
}
fn replacements(game: &mut GameState, events: &[SimultaneousDamageEvent]) -> Vec<Vec<(DamageTarget, u32)>> {
    process_simultaneous_damage_assignments_with_event(game, events).unwrap().into_iter()
        .map(|result| result.assignments.into_iter().map(|a| (a.target, a.amount)).collect()).collect()
}
fn amount(assignments: &[(DamageTarget, u32)], target: DamageTarget) -> u32 {
    assignments.iter().filter(|(recipient, _)| *recipient == target).map(|(_, amount)| amount).sum()
}
fn attack(game: &mut GameState, attackers: &[ObjectId], defender: PlayerId, decisions: &mut Choices) {
    game.turn.phase = Phase::Combat; game.turn.step = Some(Step::DeclareAttackers);
    let mut combat = CombatState::default(); let mut queue = TriggerQueue::new();
    for attacker in attackers { game.remove_summoning_sickness(*attacker); }
    let declarations: Vec<_> = attackers.iter().map(|attacker| AttackerDeclaration { creature: *attacker, target: AttackTarget::Player(defender) }).collect();
    apply_attacker_declarations(game, &mut combat, &mut queue, &declarations).unwrap();
    game.combat = Some(combat); put_triggers_on_stack_with_dm(game, &mut queue, decisions).unwrap();
}

#[test]
fn seven_complete_frozen_bodies_have_no_loss_and_keep_keywords_and_coin_trigger() {
    assert_eq!(rows().len(), 7);
    for row in rows() {
        let name = row["name"].as_str().unwrap();
        for definition in definitions(name) {
            assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
            let debug = format!("{definition:?}");
            match name {
                "Glarecaster" => assert!(debug.contains("Flying"), "{debug}"),
                "Soltari Guerrillas" => assert!(debug.contains("Shadow"), "{debug}"),
                "Goblin Psychopath" => {
                    assert!(definition.abilities.iter().any(|a| matches!(&a.kind, AbilityKind::Triggered(_))));
                    assert!(debug.contains("FlipCoin"), "{debug}");
                }
                _ => {}
            }
        }
    }
}

#[test]
fn aegis_filters_spells_and_redirects_the_whole_first_occurrence_to_each_source_controller() {
    for definition in definitions("Aegis of Honor") {
        let mut game = game(); let shield = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let first = spell(&mut game, B); let second = spell(&mut game, C); let creature = creature(&mut game, B);
        let mana = game.player(A).unwrap().mana_pool.total();
        activate(&mut game, shield, &definition, vec![], 0);
        assert_eq!(game.player(A).unwrap().mana_pool.total(), mana - 1);
        assert_eq!(replacements(&mut game, &[event(creature, DamageTarget::Player(A), 2, false)])[0], vec![(DamageTarget::Player(A), 2)]);
        let batch = [event(first, DamageTarget::Player(A), 2, false), event(second, DamageTarget::Player(A), 3, false), event(first, DamageTarget::Player(C), 4, false)];
        let actual = replacements(&mut game, &batch);
        assert_eq!(actual[0], vec![(DamageTarget::Player(B), 2)]);
        assert_eq!(actual[1], vec![(DamageTarget::Player(C), 3)]);
        assert_eq!(actual[2], vec![(DamageTarget::Player(C), 4)]);
        assert_eq!(replacements(&mut game, &batch)[0], vec![(DamageTarget::Player(A), 2)]);
    }
}

#[test]
fn glarecaster_protects_its_union_and_commits_redirected_damage_once_to_a_player() {
    for definition in definitions("Glarecaster") {
        let mut game = game(); let shield = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let source = creature(&mut game, B);
        let mana = game.player(A).unwrap().mana_pool.total();
        activate(&mut game, shield, &definition, vec![Target::Player(C)], 0);
        assert_eq!(game.player(A).unwrap().mana_pool.total(), mana - 6);
        game.set_current_controller(shield, B).unwrap();
        let mut ctx = ExecutionContext::new_default(source, B);
        let outcome = ironsmith::effects::DealDamageToRecipientsEffect { amount: 3.into(),
            recipients: vec![ChooseSpec::SpecificObject(shield), ChooseSpec::SpecificPlayer(A)] }.execute(&mut game, &mut ctx).unwrap();
        assert_eq!(game.damage_on(shield), 0); assert_eq!(game.player(A).unwrap().life, 20);
        assert_eq!(game.player(C).unwrap().life, 14);
        let receipts: Vec<_> = outcome.events.iter().filter_map(|e| e.downcast::<DamageEvent>()).collect();
        assert_eq!(receipts.iter().map(|e| e.amount).sum::<u32>(), 6);
        assert!(receipts.iter().all(|e| e.source == source && e.target == DamageTarget::Player(C)));
        assert_eq!(replacements(&mut game, &[event(source, DamageTarget::Player(A), 2, false)])[0], vec![(DamageTarget::Player(A), 2)]);
    }
}

#[test]
fn mirrorwood_supports_player_and_object_destinations_expiry_and_departed_destinations() {
    for definition in definitions("Mirrorwood Treefolk") {
        for player_target in [false, true] {
            let mut game = game(); let shield = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let source = creature(&mut game, B); let receiver = creature(&mut game, C);
            let target = if player_target { Target::Player(C) } else { Target::Object(receiver) };
            activate(&mut game, shield, &definition, vec![target], 0);
            let destination = if player_target { DamageTarget::Player(C) } else { DamageTarget::Object(receiver) };
            assert_eq!(replacements(&mut game, &[event(source, DamageTarget::Object(shield), 3, false)])[0], vec![(destination, 3)]);
            activate(&mut game, shield, &definition, vec![Target::Object(receiver)], 0);
            game.effect_store.replacement_effects.clear_one_shot_effects();
            assert_eq!(replacements(&mut game, &[event(source, DamageTarget::Object(shield), 3, false)])[0], vec![(DamageTarget::Object(shield), 3)]);
            activate(&mut game, shield, &definition, vec![Target::Object(receiver)], 0);
            game.move_object_by_effect(receiver, Zone::Graveyard).unwrap();
            assert_eq!(replacements(&mut game, &[event(source, DamageTarget::Object(shield), 3, false)])[0], vec![(DamageTarget::Object(shield), 3)]);
        }
    }
}

#[test]
fn shield_dancer_locks_only_the_chosen_attacker_and_redirects_to_that_actual_source() {
    for definition in definitions("Shield Dancer") {
        let mut game = game(); let shield = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let chosen = creature(&mut game, B); let other = creature(&mut game, B);
        game.turn.active_player = B; attack(&mut game, &[chosen, other], A, &mut Choices::default());
        activate(&mut game, shield, &definition, vec![Target::Object(chosen)], 0);
        game.combat = None; // It need not remain an attacking creature after the ability resolved.
        for (source, combat) in [(chosen, false), (other, true)] {
            assert_eq!(replacements(&mut game, &[event(source, DamageTarget::Object(shield), 2, combat)])[0], vec![(DamageTarget::Object(shield), 2)]);
        }
        let batch = [event(chosen, DamageTarget::Object(shield), 2, true), event(chosen, DamageTarget::Object(shield), 3, true)];
        assert_eq!(replacements(&mut game, &batch), vec![vec![(DamageTarget::Object(chosen), 2)], vec![(DamageTarget::Object(chosen), 3)]]);
        assert_eq!(replacements(&mut game, &batch)[0], vec![(DamageTarget::Object(shield), 2)]);
    }
}

#[test]
fn soltari_guerrillas_limits_damage_source_combat_and_opponents_without_choosing_an_opponent() {
    for definition in definitions("Soltari Guerrillas") {
        let mut game = game(); let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let receiver = creature(&mut game, B); let other = creature(&mut game, A);
        activate(&mut game, source, &definition, vec![Target::Object(receiver)], 0);
        for (damager, target, combat) in [(source, DamageTarget::Player(A), true), (source, DamageTarget::Player(B), false), (other, DamageTarget::Player(B), true)] {
            assert_eq!(replacements(&mut game, &[event(damager, target, 2, combat)])[0], vec![(target, 2)]);
        }
        let batch = [event(source, DamageTarget::Player(B), 2, true), event(source, DamageTarget::Player(C), 3, true)];
        assert_eq!(replacements(&mut game, &batch), vec![vec![(DamageTarget::Object(receiver), 2)], vec![(DamageTarget::Object(receiver), 3)]]);
    }
}

#[test]
fn hazduhr_pays_x_and_tap_and_consumes_one_budget_across_occurrences() {
    for definition in definitions("Hazduhr the Abbot") {
        for x in [0, 5] {
            let mut game = game(); let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            game.remove_summoning_sickness(source);
            let protected = creature(&mut game, A); let attacker = creature(&mut game, B);
            let mana = game.player(A).unwrap().mana_pool.total();
            activate(&mut game, source, &definition, vec![Target::Object(protected)], x);
            assert!(game.is_tapped(source)); assert_eq!(game.player(A).unwrap().mana_pool.total(), mana - x);
            // Target properties mattered at activation/resolution. Afterward the
            // registered shield remains bound to that creature's incarnation.
            game.set_current_controller(protected, B).unwrap();
            let first = replacements(&mut game, &[event(attacker, DamageTarget::Object(protected), 2, false)]);
            assert_eq!(amount(&first[0], DamageTarget::Object(source)), if x == 0 { 0 } else { 2 });
            let second = replacements(&mut game, &[event(attacker, DamageTarget::Object(protected), 4, false)]);
            assert_eq!(amount(&second[0], DamageTarget::Object(source)), if x == 0 { 0 } else { 3 });
            assert_eq!(amount(&second[0], DamageTarget::Object(protected)), if x == 0 { 4 } else { 1 });
            assert_eq!(replacements(&mut game, &[event(attacker, DamageTarget::Object(protected), 2, false)])[0], vec![(DamageTarget::Object(protected), 2)]);
        }
    }
}

#[test]
fn goblin_psychopath_keeps_both_combat_triggers_and_only_losing_flip_redirects() {
    for definition in definitions("Goblin Psychopath") {
        for blocking in [false, true] {
            for lost in [false, true] {
                let mut game = game(); let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
                let other = creature(&mut game, B); let mut decisions = Choices::default();
                if blocking {
                    game.turn.active_player = B; attack(&mut game, &[other], A, &mut decisions);
                    game.turn.step = Some(Step::DeclareBlockers);
                    let mut combat = game.combat.take().unwrap(); let mut queue = TriggerQueue::new();
                    apply_blocker_declarations(&mut game, &mut combat, &mut queue, &[BlockerDeclaration { blocker: source, blocking: other }], A).unwrap();
                    game.combat = Some(combat); put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut decisions).unwrap();
                } else { attack(&mut game, &[source], B, &mut decisions); }
                assert_eq!(game.stack.len(), 1);
                game.force_next_coin_flip(if lost { CoinFace::Tails } else { CoinFace::Heads });
                resolve_stack_entry_with(&mut game, &mut decisions).unwrap();
                assert_eq!(replacements(&mut game, &[event(source, DamageTarget::Player(B), 1, false)])[0], vec![(DamageTarget::Player(B), 1)]);
                let batch = [event(source, DamageTarget::Object(other), 2, true), event(source, DamageTarget::Player(B), 3, true)];
                let actual = replacements(&mut game, &batch);
                assert_eq!(actual[0], vec![(if lost { DamageTarget::Player(A) } else { DamageTarget::Object(other) }, 2)]);
                assert_eq!(actual[1], vec![(DamageTarget::Player(if lost { A } else { B }), 3)]);
                assert_eq!(replacements(&mut game, &batch)[1], vec![(DamageTarget::Player(B), 3)]);
            }
        }
    }
}

#[test]
fn prior_admitted_payloads_without_combat_flag_keep_unqualified_behavior() {
    let payload = ironsmith_core::RedirectNextTimeDamageToSourceEffect::new(
        ironsmith_core::RedirectNextTimeDamageSource::Filter(ObjectFilter::default()), ChooseSpec::Source);
    let mut json = serde_json::to_value(&payload).unwrap(); json.as_object_mut().unwrap().remove("combat_only");
    let decoded: ironsmith_core::RedirectNextTimeDamageToSourceEffect = serde_json::from_value(json).unwrap();
    assert_eq!(payload, decoded); assert!(!decoded.combat_only);
}

#[test]
fn rendered_damage_source_and_legacy_source_controller_scopes_reparse_semantically() {
    fn payload(definition: &CardDefinition) -> ironsmith::effects::RedirectNextTimeDamageToSourceEffect {
        definition.abilities.iter().find_map(|ability| {
            let AbilityKind::Activated(activated) = &ability.kind else { return None; };
            activated.effects.all_effects().into_iter().find_map(|effect|
                effect.downcast_ref::<ironsmith::effects::RedirectNextTimeDamageToSourceEffect>().cloned())
        }).unwrap()
    }
    for body in [
        "The next time target attacking creature would deal combat damage to this creature this turn, that creature deals that damage to itself instead.",
        "The next time an instant or sorcery spell would deal damage to you this turn, that spell deals that damage to its controller instead.",
        "All damage that would be dealt this turn by target spell is dealt to that spell's controller instead.",
        "All damage that would be dealt this turn by target creature is dealt to that source's controller instead.",
    ] {
        let prefix = "Mana cost: {W}\nType: Creature — Human\nPower/Toughness: 1/4\n";
        let (original, loss) = ironsmith_compiler::parse_loss::capture(||
            compile_to_runtime_definition("Redirection roundtrip witness", format!("{prefix}{{1}}: {body}"), false));
        let original = original.unwrap(); assert!(!loss.is_lossy(), "{}", loss.reasons_text());
        // Bypass retained canonical Oracle text and exercise the structured renderer.
        let rendered = ironsmith_text::compiled_text::unprocessed_compiled_lines(&original).join("\n");
        let (restored, loss) = ironsmith_compiler::parse_loss::capture(||
            compile_to_runtime_definition("Redirection roundtrip witness", format!("{prefix}{rendered}"), false));
        let restored = restored.unwrap_or_else(|error| panic!("{rendered}: {error}"));
        assert!(!loss.is_lossy(), "{rendered}: {}", loss.reasons_text());
        let before = payload(&original); let after = payload(&restored);
        assert_eq!(before.combat_only, after.combat_only, "{rendered}");
        assert_eq!(before.all_this_turn, after.all_this_turn, "{rendered}");
        assert_eq!(before.destination, after.destination, "{rendered}");
        assert_eq!(before.target.as_ref().map(ChooseSpec::base), after.target.as_ref().map(ChooseSpec::base), "{rendered}");
        assert_eq!(before.destination_target.as_ref().map(ChooseSpec::base), after.destination_target.as_ref().map(ChooseSpec::base), "{rendered}");
        match (&before.source, &after.source) {
            (ironsmith::effects::RedirectNextTimeDamageSource::Target(a), ironsmith::effects::RedirectNextTimeDamageSource::Target(b)) => assert_eq!(a.base(), b.base(), "{rendered}"),
            (ironsmith::effects::RedirectNextTimeDamageSource::Filter(a), ironsmith::effects::RedirectNextTimeDamageSource::Filter(b)) => assert_eq!(a, b, "{rendered}"),
            _ => panic!("source identity class changed: {rendered}"),
        }
    }
}

#[test]
fn hazduhr_allocates_its_paid_budget_by_the_affected_players_choice_of_simultaneous_source() {
    for definition in definitions("Hazduhr the Abbot") {
        for choose_first in [false, true] {
            let mut game = game(); let shield = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            game.remove_summoning_sickness(shield);
            let protected = creature(&mut game, A); let first = creature(&mut game, B); let second = creature(&mut game, C);
            activate(&mut game, shield, &definition, vec![Target::Object(protected)], 3);
            game.set_current_controller(protected, C).unwrap();
            let mut decisions = Choices { redirect_allocations: vec![if choose_first { 3 } else { 0 }], ..Default::default() };
            let mut ctx = ExecutionContext::new(shield, A, &mut decisions);
            let outcome = ironsmith::effects::DealDamageBySourcesEffect::new(
                vec![ChooseSpec::SpecificObject(first), ChooseSpec::SpecificObject(second)],
                3.into(), ChooseSpec::SpecificObject(protected),
            ).with_unpreventable(true).execute(&mut game, &mut ctx).unwrap();
            assert_eq!(outcome.count_or_zero(), 6);
            assert_eq!(game.damage_on(shield), 3); assert_eq!(game.damage_on(protected), 3);
            let chosen = if choose_first { first } else { second };
            let other = if choose_first { second } else { first };
            let receipts: Vec<_> = outcome.events.iter().filter_map(|e| e.downcast::<DamageEvent>()).collect();
            assert_eq!(receipts.iter().filter(|e| e.source == chosen && e.target == DamageTarget::Object(shield)).map(|e| e.amount).sum::<u32>(), 3);
            assert_eq!(receipts.iter().filter(|e| e.source == other && e.target == DamageTarget::Object(protected)).map(|e| e.amount).sum::<u32>(), 3);
            assert_eq!(receipts.iter().map(|e| e.amount).sum::<u32>(), 6, "no original damage receipt is replayed");
            assert_eq!(decisions.allocation_players, vec![C]);
            assert!(decisions.redirect_allocations.is_empty());
            assert_eq!(replacements(&mut game, &[event(first, DamageTarget::Object(protected), 2, false)])[0], vec![(DamageTarget::Object(protected), 2)]);
        }
    }
}
