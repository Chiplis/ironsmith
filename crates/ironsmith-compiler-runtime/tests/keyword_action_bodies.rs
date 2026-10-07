//! UNVALIDATED: frozen full bodies and independent runtime regressions; authored, unrun.
use ironsmith::ability::AbilityKind;
use ironsmith::alternative_cast::CastingMethod;
use ironsmith::cards::CardDefinition;
use ironsmith::decision::{DecisionMaker, LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
use ironsmith::decisions::context::{BooleanContext, ProliferateContext, SelectObjectsContext, TargetsContext};
use ironsmith::decisions::specs::ProliferateResponse;
use ironsmith::effect::{Effect, EffectOutcome};
use ironsmith::effects::{EffectContext, EffectExecutor, ExecutionError, execute_effect};
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, put_triggers_on_stack_with_dm, resolve_stack_entry_with};
use ironsmith::mana::ManaSymbol;
use ironsmith::static_abilities::StaticAbilityId;
use ironsmith::target::{ChooseSpec, PlayerFilter};
use ironsmith::triggers::{TriggerEvent, TriggerQueue, check_triggers};
use ironsmith::{CounterType, GameProgress, GameState, ObjectId, PlayerId, Subtype, Target, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_runtime_catalog::artifact_materializer::{encode_runtime_definition, materialize_artifact, materialize_definition};
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
const C: PlayerId = PlayerId(2);
const D: PlayerId = PlayerId(3);

fn rows() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!("../../../fixtures/keyword_action_bodies.json.fixture")).unwrap()
}
fn prior_owner_rows() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!("../../../fixtures/native_keyword_token_owners.json.fixture")).unwrap()
}
fn definitions(name: &str) -> [CardDefinition; 3] {
    let row = rows().into_iter().chain(prior_owner_rows()).find(|row| row["name"] == name).unwrap();
    let mut text = format!("Mana cost: {}\nType: {}\n", row["mana_cost"].as_str().unwrap(), row["type_line"].as_str().unwrap());
    if let (Some(p), Some(t)) = (row["power"].as_str(), row["toughness"].as_str()) {
        text.push_str(&format!("Power/Toughness: {p}/{t}\n"));
    }
    text.push_str(row["oracle_text"].as_str().unwrap());
    let (direct, loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_runtime_definition(name, &text, false));
    let direct = direct.unwrap_or_else(|error| panic!("{name} direct: {error}"));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let (artifact, loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_artifact(name, &text, false));
    let (artifact, _) = artifact.unwrap_or_else(|error| panic!("{name} artifact: {error}"));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    artifact.validate().unwrap();
    let decoded = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, decoded);
    let wire = encode_runtime_definition(direct.clone()).unwrap();
    let native = materialize_definition(serde_json::from_slice(&serde_json::to_vec(&wire).unwrap()).unwrap()).unwrap();
    [direct, materialize_artifact(&decoded).unwrap(), native]
}
fn game() -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Carol".into(), "Dan".into()], 20);
    game.turn.phase = ironsmith::Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = A;
    game.turn.priority_player = Some(A);
    for symbol in [ManaSymbol::White, ManaSymbol::Blue, ManaSymbol::Black, ManaSymbol::Red, ManaSymbol::Green, ManaSymbol::Colorless] {
        game.player_mut(A).unwrap().mana_pool.add(symbol, 20);
    }
    game
}
fn object(game: &mut GameState, owner: PlayerId, zone: Zone, name: &str, kind: &str) -> ObjectId {
    let text = format!("Mana cost: {{1}}\nType: {kind}{}", if kind.contains("Creature") { "\nPower/Toughness: 2/2" } else { "" });
    let definition = compile_to_runtime_definition(name, text, false).unwrap();
    game.create_object_from_definition(&definition, owner, zone)
}
fn tokens(game: &GameState, owner: PlayerId, subtype: Subtype) -> Vec<ObjectId> {
    let found: Vec<_> = game.battlefield.iter().copied().filter(|id| game.object(*id).is_some_and(|object|
        object.kind == ironsmith::object::ObjectKind::Token
        && game.current_controller(*id) == Some(owner)
        && game.calculated_subtypes(*id).contains(&subtype))).collect();
    if subtype == Subtype::Clue {
        for id in &found {
            let clue = game.object(*id).unwrap();
            assert_eq!(clue.name, "Clue Token");
            assert_eq!(clue.abilities.len(), 1);
            assert!(game.object_has_card_type(*id, ironsmith::CardType::Artifact));
            assert!(game.current_colors(*id).is_some_and(|colors| colors.is_empty()));
            let AbilityKind::Activated(ability) = &clue.abilities[0].kind else { panic!("Clue ability"); };
            assert_eq!(ability.mana_cost.costs().len(), 2);
            assert!(ability.effects.all_effects().iter().any(|effect| effect.downcast_ref::<ironsmith::effects::DrawCardsEffect>().is_some()));
        }
    }
    found
}
#[derive(Default)]
struct Choices {
    targets: Vec<Target>,
    army: Option<ObjectId>,
    proliferate: Vec<ProliferateResponse>,
    observed: Vec<Vec<(ObjectId, u32)>>,
    pause_at: Option<usize>,
    pause_proliferate_at: Option<usize>,
    boolean_calls: usize,
    accept: Option<bool>,
    waiting: bool,
}
impl DecisionMaker for Choices {
    fn awaiting_choice(&self) -> bool { self.waiting }
    fn decide_targets(&mut self, _: &GameState, ctx: &TargetsContext) -> Vec<Target> {
        let mut selected = Vec::new();
        for requirement in &ctx.requirements {
            let target = self.targets.iter().copied().find(|target|
                requirement.legal_targets.contains(target) && !selected.contains(target))
                .unwrap_or_else(|| panic!("requested target must be legal: {requirement:?}"));
            selected.push(target);
        }
        selected
    }
    fn decide_objects(&mut self, game: &GameState, ctx: &SelectObjectsContext) -> Vec<ObjectId> {
        if let Some(id) = self.army && ctx.candidates.iter().any(|candidate| candidate.id == id && candidate.legal) {
            return vec![id];
        }
        SelectFirstDecisionMaker.decide_objects(game, ctx)
    }
    fn decide_boolean(&mut self, _: &GameState, _: &BooleanContext) -> bool {
        self.boolean_calls += 1;
        self.waiting = self.pause_at == Some(self.boolean_calls);
        !self.waiting && self.accept.unwrap_or(true)
    }
    fn decide_proliferate(&mut self, game: &GameState, ctx: &ProliferateContext) -> ProliferateResponse {
        self.observed.push(ctx.eligible_permanents.iter().map(|(id, _)| (*id, game.counter_count(*id, CounterType::Charge))).collect());
        if self.pause_proliferate_at == Some(self.observed.len()) {
            self.waiting = true;
            return ProliferateResponse::default();
        }
        self.proliferate.remove(0)
    }
}
fn cast(game: &mut GameState, definition: &CardDefinition, zone: Zone, dm: &mut Choices) -> ObjectId {
    let id = game.create_object_from_definition(definition, A, zone);
    let action = compute_legal_actions(game, A).unwrap().into_iter().find(|action|
        matches!(action, LegalAction::CastSpell { spell_id, from_zone, .. } if *spell_id == id && *from_zone == zone)).unwrap();
    if zone == Zone::Graveyard {
        assert!(!matches!(&action, LegalAction::CastSpell { casting_method: CastingMethod::Normal, .. }));
    }
    let mut queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(4);
    let mut progress = apply_priority_response_with_dm(game, &mut queue, &mut state, &PriorityResponse::PriorityAction(action), dm).unwrap();
    for _ in 0..60 {
        if state.pending_cast.is_none() && state.pending_method_selection.is_none() { break; }
        let GameProgress::NeedsDecisionCtx(ctx) = progress else { panic!("{progress:?}"); };
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &ctx, dm).unwrap();
    }
    assert!(state.pending_cast.is_none() && state.pending_method_selection.is_none());
    let source = game.stack.iter().find(|entry| !entry.is_ability).unwrap().object_id;
    put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap();
    source
}
fn resolve(game: &mut GameState, dm: &mut Choices) { resolve_stack_entry_with(game, dm).unwrap(); }
fn apply(game: &mut GameState, source: ObjectId, effect: Effect, dm: &mut Choices) -> EffectOutcome {
    execute_effect(game, &effect, &mut EffectContext::new(source, A, dm)).unwrap()
}
fn stack(game: &mut GameState, event: &TriggerEvent, dm: &mut Choices) -> usize {
    let entries = check_triggers(game, event);
    let count = entries.len();
    let mut queue = TriggerQueue::new();
    for entry in entries { queue.add(entry); }
    put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap();
    count
}

#[test]
fn six_complete_frozen_bodies_use_direct_artifact_and_native_definitions() {
    assert_eq!(rows().len(), 6);
    for row in rows() {
        for definition in definitions(row["name"].as_str().unwrap()) {
            assert_eq!(definition.card.name, row["name"].as_str().unwrap());
            assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
            let rendered = ironsmith_text::compiled_text_lines(&definition).join("\n").to_ascii_lowercase();
            match definition.card.name.as_str() {
                "Wojek Investigator" => {
                    assert!(rendered.contains("more cards in hand than you"), "{rendered}");
                    assert!(rendered.contains("each opponent"), "{rendered}");
                }
                "Secrets of the Key" | "Tidings of War" => {
                    assert!(rendered.contains("cast from a graveyard") && rendered.contains("instead"), "{rendered}");
                    assert!(rendered.contains("flashback"), "{rendered}");
                }
                "Panther Pounce" => {
                    assert!(rendered.contains("target player") && rendered.contains("untap") && rendered.contains("flying"), "{rendered}");
                }
                _ => {}
            }
        }
    }
}

#[test]
fn investigate_then_counts_new_clue_and_only_casters_clues() {
    for definition in definitions("Confront the Unknown") {
        let mut game = game();
        let target = object(&mut game, B, Zone::Battlefield, "Opponent creature", "Creature — Bear");
        let mut dm = Choices { targets: vec![Target::Object(target)], ..Default::default() };
        apply(&mut game, target, Effect::investigate_player(2, PlayerFilter::Specific(A)), &mut dm);
        apply(&mut game, target, Effect::investigate_player(4, PlayerFilter::Specific(B)), &mut dm);
        cast(&mut game, &definition, Zone::Hand, &mut dm);
        resolve(&mut game, &mut dm);
        assert_eq!(tokens(&game, A, Subtype::Clue).len(), 3);
        assert_eq!(tokens(&game, B, Subtype::Clue).len(), 4);
        assert_eq!(game.current_power(target), Some(5));
        assert_eq!(game.current_toughness(target), Some(5));
        ironsmith::turn::execute_cleanup_step(&mut game);
        assert_eq!(game.current_power(target), Some(2));
        assert_eq!(game.current_toughness(target), Some(2));
    }
}

#[test]
fn independent_player_and_creature_targets_keep_every_panther_followup() {
    for definition in definitions("Panther Pounce") {
        let mut game = game();
        let target = object(&mut game, C, Zone::Battlefield, "Carol creature", "Creature — Bear");
        game.tap(target);
        let mut dm = Choices { targets: vec![Target::Player(B), Target::Object(target)], ..Default::default() };
        cast(&mut game, &definition, Zone::Hand, &mut dm);
        resolve(&mut game, &mut dm);
        assert_eq!(tokens(&game, B, Subtype::Clue).len(), 1);
        assert!(tokens(&game, A, Subtype::Clue).is_empty());
        assert!(tokens(&game, C, Subtype::Clue).is_empty());
        assert_eq!(game.current_power(target), Some(3));
        assert_eq!(game.current_toughness(target), Some(2));
        assert!(!game.is_tapped(target));
        assert!(game.current_has_static_ability_id(target, StaticAbilityId::Flying));
        let clue = tokens(&game, B, Subtype::Clue)[0];
        assert_eq!(game.object(clue).unwrap().name, "Clue Token");
        let AbilityKind::Activated(ability) = &game.object(clue).unwrap().abilities[0].kind else { panic!("Clue ability"); };
        assert_eq!(ability.mana_cost.costs().len(), 2);
        assert!(ability.effects.all_effects().iter().any(|effect| effect.downcast_ref::<ironsmith::effects::DrawCardsEffect>().is_some()));
        ironsmith::turn::execute_cleanup_step(&mut game);
        assert_eq!(game.current_power(target), Some(2));
        assert!(!game.current_has_static_ability_id(target, StaticAbilityId::Flying));
        assert!(!game.is_tapped(target), "untapping is not a temporary grant");
    }
}

#[test]
fn flashback_uses_one_replacement_arm_and_exiles_after_resolving() {
    for name in ["Secrets of the Key", "Tidings of War"] {
        for definition in definitions(name) {
            for from in [Zone::Hand, Zone::Graveyard] {
                let mut game = game();
                let mut dm = Choices::default();
                let source = cast(&mut game, &definition, from, &mut dm);
                let stable = game.object(source).unwrap().stable_id;
                resolve(&mut game, &mut dm);
                let final_id = game.find_object_by_stable_id(stable).unwrap();
                assert_eq!(game.object(final_id).unwrap().zone, if from == Zone::Hand { Zone::Graveyard } else { Zone::Exile });
                if name == "Secrets of the Key" {
                    assert_eq!(tokens(&game, A, Subtype::Clue).len(), if from == Zone::Hand { 1 } else { 2 });
                } else {
                    let armies = tokens(&game, A, Subtype::Army);
                    assert_eq!(armies.len(), 1);
                    let army = armies[0];
                    assert_eq!(game.object(army).unwrap().name, "Goblin Army Token");
                    assert!(game.calculated_subtypes(army).contains(&Subtype::Goblin));
                    assert_eq!(game.counter_count(army, CounterType::PlusOnePlusOne), if from == Zone::Hand { 1 } else { 3 });
                    assert_eq!(game.current_colors(army), Some(ironsmith::color::ColorSet::BLACK));
                }
            }
        }
    }
}

#[test]
fn amass_selects_one_existing_army_and_preserves_other_subtypes() {
    for definition in definitions("Tidings of War") {
        let mut game = game();
        let first = object(&mut game, A, Zone::Battlefield, "First Army", "Creature — Zombie Army");
        let chosen = object(&mut game, A, Zone::Battlefield, "Chosen Army", "Creature — Orc Army");
        let enemy = object(&mut game, B, Zone::Battlefield, "Foreign Army", "Creature — Army");
        let mut dm = Choices { army: Some(chosen), ..Default::default() };
        cast(&mut game, &definition, Zone::Graveyard, &mut dm);
        resolve(&mut game, &mut dm);
        assert_eq!(game.counter_count(first, CounterType::PlusOnePlusOne), 0);
        assert_eq!(game.counter_count(enemy, CounterType::PlusOnePlusOne), 0);
        assert_eq!(game.counter_count(chosen, CounterType::PlusOnePlusOne), 3);
        assert!(game.calculated_subtypes(chosen).contains(&Subtype::Orc));
        assert!(game.calculated_subtypes(chosen).contains(&Subtype::Goblin));
        assert_eq!(game.object(chosen).unwrap().name, "Chosen Army", "amass does not rename an existing Army");
        assert!(!game.object(chosen).unwrap().subtypes.contains(&Subtype::Goblin), "added type is not copiable");
        assert!(tokens(&game, A, Subtype::Army).is_empty());
    }
}

#[test]
fn roalesk_keeps_etb_other_target_and_two_independent_death_choices() {
    for definition in definitions("Roalesk, Apex Hybrid") {
        let mut game = game();
        let target = object(&mut game, A, Zone::Battlefield, "ETB recipient", "Creature — Bear");
        let first = object(&mut game, A, Zone::Battlefield, "First proliferation", "Artifact");
        let second = object(&mut game, B, Zone::Battlefield, "Second proliferation", "Artifact");
        game.add_counters(first, CounterType::Charge, 1).unwrap();
        game.add_counters(second, CounterType::Charge, 1).unwrap();
        let mut dm = Choices { targets: vec![Target::Object(target)], proliferate: vec![
            ProliferateResponse { permanents: vec![first], players: vec![] },
            ProliferateResponse { permanents: vec![second], players: vec![] },
        ], ..Default::default() };
        let spell = cast(&mut game, &definition, Zone::Hand, &mut dm);
        let stable = game.object(spell).unwrap().stable_id;
        resolve(&mut game, &mut dm);
        put_triggers_on_stack_with_dm(&mut game, &mut TriggerQueue::new(), &mut dm).unwrap();
        assert_eq!(game.stack.len(), 1);
        resolve(&mut game, &mut dm);
        let source = game.find_object_by_stable_id(stable).unwrap();
        assert_eq!(game.counter_count(source, CounterType::PlusOnePlusOne), 0);
        assert_eq!(game.counter_count(target, CounterType::PlusOnePlusOne), 2);
        assert!(game.current_has_static_ability_id(source, StaticAbilityId::Flying));
        assert!(game.current_has_static_ability_id(source, StaticAbilityId::Trample));
        let outcome = apply(&mut game, source, Effect::destroy(ChooseSpec::SpecificObject(source)), &mut dm);
        for event in &outcome.events { stack(&mut game, event, &mut dm); }
        put_triggers_on_stack_with_dm(&mut game, &mut TriggerQueue::new(), &mut dm).unwrap();
        assert_eq!(game.stack.len(), 1);
        resolve(&mut game, &mut dm);
        assert_eq!(dm.observed.len(), 2);
        assert!(dm.observed[0].contains(&(first, 1)));
        assert!(dm.observed[1].contains(&(first, 2)), "second choice sees the first choice's committed counters");
        assert_eq!(game.counter_count(first, CounterType::Charge), 2);
        assert_eq!(game.counter_count(second, CounterType::Charge), 2);
        assert_eq!(game.counter_count(target, CounterType::PlusOnePlusOne), 2, "unchosen counters are unchanged");
    }
}

#[test]
fn wojek_counts_only_opponents_currently_ahead_when_upkeep_resolves() {
    for definition in definitions("Wojek Investigator") {
        for ahead in [false, true] {
            let mut game = game();
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            for (player, size) in [(A, 2), (B, 3), (C, 2), (D, if ahead { 4 } else { 1 })] {
                for index in 0..size { object(&mut game, player, Zone::Hand, &format!("Hand card {index}"), "Instant"); }
            }
            let mut dm = Choices::default();
            assert!(game.current_has_static_ability_id(source, StaticAbilityId::Flying));
            assert!(game.current_has_static_ability_id(source, StaticAbilityId::Vigilance));
            let foreign = TriggerEvent::new(ironsmith::events::BeginningOfUpkeepEvent::new(B), Default::default());
            assert_eq!(stack(&mut game, &foreign, &mut dm), 0);
            let upkeep = TriggerEvent::new(ironsmith::events::BeginningOfUpkeepEvent::new(A), Default::default());
            assert_eq!(stack(&mut game, &upkeep, &mut dm), 1);
            let discarded = game.player(B).unwrap().hand[0];
            game.move_object_by_effect(discarded, Zone::Graveyard);
            resolve(&mut game, &mut dm);
            assert_eq!(tokens(&game, A, Subtype::Clue).len(), usize::from(ahead), "tied hands and opponents who ceased being ahead do not count");
        }
    }
}

#[test]
fn targeted_repeated_investigate_pending_and_resource_error_restore_whole_instruction() {
    use ironsmith::replacement::{ReplacementAction, ReplacementEffect};
    for dispatched in [false, true] {
        for pending in [false, true] {
            let mut game = game();
            let source = object(&mut game, A, Zone::Battlefield, "Investigation source", "Artifact");
            game.take_pending_trigger_events();
            game.effect_store.replacement_effects.add_resolution_effect(ReplacementEffect::with_matcher(
                source, A, ironsmith::events::zones::matchers::WouldEnterBattlefieldMatcher::any(),
                ReplacementAction::InteractivePayLifeOrEnterTapped { life_cost: 2 },
            ));
            if !pending { game.set_token_creation_limits(ironsmith::effects::tokens::TokenCreationLimits { max_created_tokens: 1, ..Default::default() }); }
            let mut dm = Choices { pause_at: pending.then_some(2), ..Default::default() };
            let effect = ironsmith::effects::InvestigateEffect::new(2, PlayerFilter::Specific(B));
            let next = game.next_object_id_counter();
            let mut ctx = EffectContext::new(source, A, &mut dm);
            let outcome = if dispatched { execute_effect(&mut game, &Effect::new(effect.clone()), &mut ctx) }
                else { effect.execute(&mut game, &mut ctx) };
            if pending { assert!(outcome.unwrap().events.is_empty()); assert!(ctx.decision_maker.awaiting_choice()); }
            else { assert!(matches!(outcome, Err(ExecutionError::ResourceLimitExceeded { .. }))); }
            assert_eq!(game.player(A).unwrap().life, 20);
            assert_eq!(game.player(B).unwrap().life, 20);
            assert!(tokens(&game, B, Subtype::Clue).is_empty());
            assert_eq!(game.next_object_id_counter(), next);
            assert!(game.take_pending_trigger_events().is_empty());
            drop(ctx);
            game.set_token_creation_limits(Default::default());
            let mut replay = Choices::default();
            let result = effect.execute(&mut game, &mut EffectContext::new(source, A, &mut replay)).unwrap();
            assert_eq!(tokens(&game, B, Subtype::Clue).len(), 2);
            assert_eq!(game.player(A).unwrap().life, 20);
            assert_eq!(game.player(B).unwrap().life, 16, "the investigating player owns the tokens and entry payments");
            assert_eq!(result.count_or_zero(), 2);
            assert_eq!(result.events.iter().filter(|event| event.downcast::<ironsmith::events::KeywordActionEvent>()
                .is_some_and(|action| action.action == ironsmith::events::KeywordActionKind::Investigate && action.player == B)).count(), 2);
        }
    }
}

#[test]
fn missing_creature_target_fizzles_confront_but_preserves_panthers_legal_player_action() {
    for name in ["Confront the Unknown", "Panther Pounce"] {
        for definition in definitions(name) {
            let mut game = game();
            let target = object(&mut game, C, Zone::Battlefield, "Departing target", "Creature — Bear");
            let targets = if name == "Panther Pounce" { vec![Target::Player(B), Target::Object(target)] }
                else { vec![Target::Object(target)] };
            let mut dm = Choices { targets, ..Default::default() };
            cast(&mut game, &definition, Zone::Hand, &mut dm);
            game.move_object_by_effect(target, Zone::Graveyard);
            resolve(&mut game, &mut dm);
            assert!(tokens(&game, A, Subtype::Clue).is_empty());
            assert_eq!(tokens(&game, B, Subtype::Clue).len(), usize::from(name == "Panther Pounce"));
            assert!(game.stack_is_empty());
        }
    }
}

#[test]
fn uncast_copies_of_graveyard_spells_use_the_default_keyword_arm() {
    for name in ["Secrets of the Key", "Tidings of War"] {
        for definition in definitions(name) {
            let mut game = game();
            let mut dm = Choices::default();
            let source = cast(&mut game, &definition, Zone::Graveyard, &mut dm);
            apply(&mut game, source, Effect::copy_spell(ChooseSpec::SpecificObject(source)), &mut dm);
            assert_eq!(game.stack.len(), 2);
            resolve(&mut game, &mut dm);
            if name == "Secrets of the Key" {
                assert_eq!(tokens(&game, A, Subtype::Clue).len(), 1);
            } else {
                let army = tokens(&game, A, Subtype::Army)[0];
                assert_eq!(game.counter_count(army, CounterType::PlusOnePlusOne), 1);
            }
            resolve(&mut game, &mut dm);
            if name == "Secrets of the Key" {
                assert_eq!(tokens(&game, A, Subtype::Clue).len(), 3);
            } else {
                let army = tokens(&game, A, Subtype::Army)[0];
                assert_eq!(game.counter_count(army, CounterType::PlusOnePlusOne), 4);
            }
        }
    }
}

#[test]
fn repeated_proliferate_rolls_back_the_first_choice_while_the_second_is_pending() {
    for dispatched in [false, true] {
        let mut game = game();
        let source = object(&mut game, A, Zone::Battlefield, "Proliferation source", "Artifact");
        let first = object(&mut game, A, Zone::Battlefield, "First counted object", "Artifact");
        let second = object(&mut game, B, Zone::Battlefield, "Second counted object", "Artifact");
        game.add_counters(first, CounterType::Charge, 1).unwrap();
        game.add_counters(second, CounterType::Charge, 1).unwrap();
        game.take_pending_trigger_events();
        let choices = vec![
            ProliferateResponse { permanents: vec![first], players: vec![] },
            ProliferateResponse { permanents: vec![second], players: vec![] },
        ];
        let mut dm = Choices { proliferate: choices.clone(), pause_proliferate_at: Some(2), ..Default::default() };
        let effect = ironsmith::effects::ProliferateEffect::new(2);
        let mut ctx = EffectContext::new(source, A, &mut dm);
        let outcome = if dispatched { execute_effect(&mut game, &Effect::new(effect.clone()), &mut ctx) }
            else { effect.execute(&mut game, &mut ctx) }.unwrap();
        assert!(ctx.decision_maker.awaiting_choice());
        assert!(outcome.events.is_empty());
        assert_eq!(game.counter_count(first, CounterType::Charge), 1);
        assert_eq!(game.counter_count(second, CounterType::Charge), 1);
        assert!(game.take_pending_trigger_events().is_empty());
        drop(ctx);
        assert!(dm.observed[1].contains(&(first, 2)), "the second pending decision saw the first action before rollback");
        let mut replay = Choices { proliferate: choices, ..Default::default() };
        effect.execute(&mut game, &mut EffectContext::new(source, A, &mut replay)).unwrap();
        assert_eq!(game.counter_count(first, CounterType::Charge), 2);
        assert_eq!(game.counter_count(second, CounterType::Charge), 2);
        assert_eq!(replay.observed.len(), 2);
    }
}

#[test]
fn native_keyword_effect_transport_creates_the_same_named_tokens_and_real_clue_ability() {
    use ironsmith_runtime_catalog::artifact_materializer::{encode_runtime_effect, materialize_effect};
    for amass in [false, true] {
        let original = if amass { Effect::amass(Some(Subtype::Goblin), 2) }
            else { Effect::investigate_player(2, PlayerFilter::Specific(B)) };
        let encoded = encode_runtime_effect(original.clone()).unwrap();
        let restored = materialize_effect(serde_json::from_slice(&serde_json::to_vec(&encoded).unwrap()).unwrap()).unwrap();
        for effect in [original, restored] {
            let mut game = game();
            let source = object(&mut game, A, Zone::Battlefield, "Native keyword source", "Artifact");
            let mut dm = Choices::default();
            apply(&mut game, source, effect, &mut dm);
            if amass {
                let army = tokens(&game, A, Subtype::Army)[0];
                assert_eq!(game.object(army).unwrap().name, "Goblin Army Token");
                assert_eq!(game.counter_count(army, CounterType::PlusOnePlusOne), 2);
                assert_eq!(game.current_power(army), Some(2));
                assert!(game.object(army).unwrap().abilities.is_empty());
            } else {
                let clues = tokens(&game, B, Subtype::Clue);
                assert_eq!(clues.len(), 2);
                assert!(tokens(&game, A, Subtype::Clue).is_empty());
                object(&mut game, B, Zone::Library, "Clue draw", "Instant");
                game.player_mut(B).unwrap().mana_pool.add(ManaSymbol::Colorless, 2);
                game.turn.priority_player = Some(B);
                let action = LegalAction::ActivateAbility { source: clues[0], ability_index: 0 };
                assert!(compute_legal_actions(&game, B).unwrap().contains(&action));
                let mut queue = TriggerQueue::new();
                let mut state = PriorityLoopState::new(4);
                let mut progress = apply_priority_response_with_dm(&mut game, &mut queue, &mut state,
                    &PriorityResponse::PriorityAction(action), &mut dm).unwrap();
                for _ in 0..30 {
                    if state.pending_activation.is_none() { break; }
                    let GameProgress::NeedsDecisionCtx(ctx) = progress else { panic!("{progress:?}"); };
                    progress = apply_decision_context_with_dm(&mut game, &mut queue, &mut state, &ctx, &mut dm).unwrap();
                }
                assert!(state.pending_activation.is_none());
                assert_eq!(game.player(B).unwrap().mana_pool.total(), 0, "the real activation pays two mana");
                assert!(game.object(clues[0]).is_none(), "sacrifice is a paid activation cost");
                resolve(&mut game, &mut dm);
                assert_eq!(game.player(B).unwrap().hand.len(), 1);
                assert_eq!(game.player(A).unwrap().hand.len(), 0);
                assert_eq!(tokens(&game, B, Subtype::Clue).len(), 1);
            }
        }
    }
}

#[test]
fn nine_earlier_frozen_bodies_retain_the_concrete_native_keyword_owner_on_every_route() {
    assert_eq!(prior_owner_rows().len(), 9);
    for row in prior_owner_rows() {
        for definition in definitions(row["name"].as_str().unwrap()) {
            let expected_amass = matches!(definition.card.name.as_str(), "Bolg of the North" | "Fall of Cair Andros");
            fn contains(effect: &Effect, amass: bool) -> bool {
                let mut found = if amass { effect.downcast_ref::<ironsmith::effects::AmassEffect>().is_some() }
                    else { effect.downcast_ref::<ironsmith::effects::InvestigateEffect>().is_some() };
                effect.visit_child_effects(&mut |child| found |= contains(child, amass));
                found
            }
            assert!(definition.spell_effect.iter().chain(definition.abilities.iter().filter_map(|ability| match &ability.kind {
                AbilityKind::Activated(ability) => Some(&ability.effects),
                AbilityKind::Triggered(ability) => Some(&ability.effects),
                _ => None,
            })).any(|program| program.all_effects().iter().any(|effect| contains(effect, expected_amass))),
                "{} must retain its native keyword action, not a name-shaped create shortcut", definition.card.name);
        }
    }
}

fn settle_keyword_triggers(game: &mut GameState, dm: &mut Choices) {
    let mut queue = TriggerQueue::new();
    for _ in 0..20 {
        put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap();
        if game.stack_is_empty() { return; }
        resolve(game, dm);
    }
    panic!("finite keyword-trigger scenario did not settle");
}

#[test]
fn evidence_examiner_uses_real_collection_and_declining_creates_no_clue() {
    for definition in definitions("Evidence Examiner") {
        for accept in [false, true] {
            let mut game = game();
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let proof = compile_to_runtime_definition("Four-mana evidence", "Mana cost: {4}\nType: Artifact", false).unwrap();
            let grave = game.create_object_from_definition(&proof, A, Zone::Graveyard);
            let mut dm = Choices { army: Some(grave), accept: Some(accept), ..Default::default() };
            let wrong_turn = TriggerEvent::new(ironsmith::events::BeginningOfCombatEvent::new(B), Default::default());
            assert_eq!(stack(&mut game, &wrong_turn, &mut dm), 0);
            let combat = TriggerEvent::new(ironsmith::events::BeginningOfCombatEvent::new(A), Default::default());
            assert_eq!(stack(&mut game, &combat, &mut dm), 1);
            settle_keyword_triggers(&mut game, &mut dm);
            assert_eq!(tokens(&game, A, Subtype::Clue).len(), usize::from(accept));
            assert!(game.object(source).is_some());
            assert_eq!(game.player(A).unwrap().graveyard.len(), usize::from(!accept));
            assert_eq!(game.exile.len(), usize::from(accept));
        }
    }
}

#[test]
fn resonance_technician_discards_for_two_named_clues_but_can_decline() {
    for definition in definitions("Resonance Technician") {
        for accept in [false, true] {
            let mut game = game();
            let discard = object(&mut game, A, Zone::Hand, "Optional discard", "Instant");
            let mut dm = Choices { army: Some(discard), accept: Some(accept), ..Default::default() };
            let spell = cast(&mut game, &definition, Zone::Hand, &mut dm);
            let stable = game.object(spell).unwrap().stable_id;
            resolve(&mut game, &mut dm);
            settle_keyword_triggers(&mut game, &mut dm);
            let source = game.find_object_by_stable_id(stable).unwrap();
            assert!(game.current_has_static_ability_id(source, StaticAbilityId::Flying));
            assert!(game.current_abilities(source).unwrap().iter().any(|ability| matches!(ability.kind, AbilityKind::Activated(_))));
            assert_eq!(tokens(&game, A, Subtype::Clue).len(), if accept { 2 } else { 0 });
            assert_eq!(game.player(A).unwrap().hand.len(), usize::from(!accept));
            assert_eq!(game.player(A).unwrap().graveyard.len(), usize::from(accept));
        }
    }
}

#[test]
fn thorough_investigation_uses_real_attacks_and_clue_sacrifice_venture() {
    let mut dungeon = CardDefinition::new(ironsmith::card::CardBuilder::new(ironsmith::CardId::new(), "Native keyword witness dungeon")
        .card_types(vec![ironsmith::CardType::Dungeon]).build());
    for (room, next) in [("Entry", vec!["Exit".to_owned()]), ("Exit", vec![])] {
        dungeon.abilities.push(ironsmith::ability::Ability::triggered(
            ironsmith::triggers::Trigger::dungeon_room(room, next), vec![Effect::gain_life(1)]));
    }
    ironsmith::dungeon::register_dungeon_definition(&dungeon).unwrap();
    for definition in definitions("Thorough Investigation") {
        for attacker in [A, B] {
            let mut game = game();
            game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let creature = object(&mut game, attacker, Zone::Battlefield, "Attacking witness", "Creature — Bear");
            game.remove_summoning_sickness(creature);
            game.turn.active_player = attacker;
            game.turn.phase = ironsmith::Phase::Combat;
            game.turn.step = Some(ironsmith::Step::DeclareAttackers);
            game.mark_combat_phase_started();
            let mut combat = ironsmith::combat_state::CombatState::default();
            let mut queue = TriggerQueue::new();
            ironsmith::game_loop::apply_attacker_declarations(&mut game, &mut combat, &mut queue,
                &[ironsmith::decision::AttackerDeclaration { creature, target: ironsmith::combat_state::AttackTarget::Player(if attacker == A { B } else { A }) }]).unwrap();
            game.combat = Some(combat);
            let mut dm = Choices::default();
            put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm).unwrap();
            settle_keyword_triggers(&mut game, &mut dm);
            let clues = tokens(&game, A, Subtype::Clue);
            assert_eq!(clues.len(), usize::from(attacker == A));
            if attacker == A {
                let outcome = apply(&mut game, clues[0], Effect::sacrifice_source(), &mut dm);
                for event in &outcome.events { stack(&mut game, event, &mut dm); }
                settle_keyword_triggers(&mut game, &mut dm);
                assert!(game.object(clues[0]).is_none());
                assert!(game.active_dungeon(A).is_some());
                assert!(game.active_dungeon(B).is_none());
            }
        }
    }
}
