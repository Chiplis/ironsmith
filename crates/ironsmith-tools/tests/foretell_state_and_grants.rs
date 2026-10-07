//! Frozen Foretell source contracts. Authored only; all execution is deferred.
use ironsmith::alternative_cast::CastingMethod;
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::{DecisionMaker, GameProgress, LegalAction, compute_legal_actions};
use ironsmith::decisions::context::{NumberContext, PartitionContext, SelectObjectsContext, SelectOptionsContext, TargetsContext};
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, drain_pending_trigger_events, put_triggers_on_stack_with_dm, resolve_stack_entry_with};
use ironsmith::game_state::Target;
use ironsmith::mana::ManaSymbol;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{CardDefinition, CardId, CardType, GameState, ObjectId, PlayerId, Zone};
const ALICE: PlayerId = PlayerId(0);
const BOB: PlayerId = PlayerId(1);

fn definitions(name: &str) -> Vec<CardDefinition> {
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../fixtures/foretell_state_and_grants.json.fixture")).unwrap();
    let row = rows.iter().find(|row| row["name"] == name).unwrap();
    let mut text = format!("Mana cost: {}\nType: {}\n", row["mana_cost"].as_str().unwrap(), row["type_line"].as_str().unwrap());
    if let (Some(power), Some(toughness)) = (row["power"].as_str(), row["toughness"].as_str()) {
        text.push_str(&format!("Power/Toughness: {power}/{toughness}\n"));
    }
    text.push_str(row["oracle_text"].as_str().unwrap());
    let (direct, loss) = ironsmith_compiler::parse_loss::capture(||
        ironsmith_registry::compile_builder_to_runtime_definition(
            ironsmith_compiler::CardDefinitionBuilder::new(CardId::new(), name), text.clone(), false));
    assert!(!loss.is_lossy(), "{name} direct: {}", loss.reasons_text());
    let (artifact, loss) = ironsmith_compiler::parse_loss::capture(||
        ironsmith_registry::compile_builder_to_artifact(
            ironsmith_compiler::CardDefinitionBuilder::new(CardId::new(), name), text, false));
    assert!(!loss.is_lossy(), "{name} artifact: {}", loss.reasons_text());
    let (artifact, _) = artifact.unwrap();
    artifact.validate().unwrap();
    let decoded = serde_json::from_slice(&serde_json::to_vec(&artifact).unwrap()).unwrap();
    vec![direct.unwrap(), ironsmith::artifact_materializer::materialize_artifact(&decoded).unwrap()]
}

#[derive(Default)]
struct Choices { target: Option<Target>, x: u32, scry_cards: usize }
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
    fn decide_options(&mut self, game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
        if let Some(option) = ctx.options.iter().find(|o| o.legal && o.description == "Goblin") { return vec![option.index]; }
        ironsmith::decision::SelectFirstDecisionMaker.decide_options(game, ctx)
    }
    fn decide_partition(&mut self, _: &GameState, ctx: &PartitionContext) -> Vec<ObjectId> {
        self.scry_cards += ctx.cards.len(); Vec::new()
    }
}
fn main_phase(game: &mut GameState, active: PlayerId) {
    game.turn.active_player = active;
    game.turn.priority_player = Some(ALICE);
    game.turn.phase = ironsmith::Phase::FirstMain;
    game.turn.step = None;
}
fn setup() -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.set_random_seed(31079);
    game.turn.turn_number = 7;
    main_phase(&mut game, ALICE);
    for symbol in [ManaSymbol::White, ManaSymbol::Blue, ManaSymbol::Black, ManaSymbol::Red, ManaSymbol::Green] {
        game.player_mut(ALICE).unwrap().mana_pool.add(symbol, 20);
    }
    game
}
fn creature(game: &mut GameState, owner: PlayerId, subtype: ironsmith::Subtype, zone: Zone) -> ObjectId {
    let definition = CardDefinitionBuilder::new(CardId::new(), "Scenario creature")
        .card_types(vec![CardType::Creature]).subtypes(vec![subtype])
        .mana_cost(ironsmith::mana::ManaCost::from_symbols(vec![ManaSymbol::Generic(1)]))
        .power_toughness(ironsmith::card::PowerToughness::fixed(4, 4)).build();
    game.create_object_from_definition(&definition, owner, zone)
}
fn library(game: &mut GameState, count: usize) {
    for _ in 0..count { creature(game, ALICE, ironsmith::Subtype::Goblin, Zone::Library); }
}
fn cast_action(game: &GameState, card: ObjectId, alternative: bool) -> Option<LegalAction> {
    compute_legal_actions(game, ALICE).unwrap().into_iter().find(|action| match action {
        LegalAction::CastSpell { spell_id, casting_method, .. } if *spell_id == card => {
            if alternative { matches!(casting_method, CastingMethod::Alternative(0)) }
            else { matches!(casting_method, CastingMethod::Normal | CastingMethod::PlayFrom { use_alternative: None, .. }) }
        }
        _ => false,
    })
}
fn announce(game: &mut GameState, action: LegalAction, choices: &mut Choices) {
    let count = game.stack.len();
    let mut queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(game.players_in_game());
    let mut progress = apply_priority_response_with_dm(game, &mut queue, &mut state,
        &PriorityResponse::PriorityAction(action), choices).unwrap();
    for _ in 0..40 {
        if state.pending_cast.is_none() && state.pending_activation.is_none() && game.stack.len() > count { return; }
        let GameProgress::NeedsDecisionCtx(ctx) = progress else { panic!("{progress:?}") };
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &ctx, choices).unwrap();
    }
    panic!("native announcement did not complete");
}
fn cast(game: &mut GameState, card: ObjectId, alternative: bool, cost: u32, choices: &mut Choices) -> ObjectId {
    let stable = game.object(card).unwrap().stable_id;
    let before = game.player(ALICE).unwrap().mana_pool.total();
    let action = cast_action(game, card, alternative).expect("native legal cast required");
    announce(game, action, choices);
    assert_eq!(before - game.player(ALICE).unwrap().mana_pool.total(), cost);
    let spell = game.find_object_by_stable_id(stable).unwrap();
    assert_eq!(game.object(spell).unwrap().zone, Zone::Stack);
    spell
}
fn finish(game: &mut GameState, choices: &mut Choices) {
    for _ in 0..30 {
        let mut queue = TriggerQueue::new();
        drain_pending_trigger_events(game, &mut queue);
        put_triggers_on_stack_with_dm(game, &mut queue, choices).unwrap();
        if game.stack.is_empty() { return; }
        resolve_stack_entry_with(game, choices).unwrap();
    }
    panic!("unresolved native stack");
}
fn foretell(game: &mut GameState, card: ObjectId) -> ObjectId {
    let stable = game.object(card).unwrap().stable_id;
    let before = game.player(ALICE).unwrap().mana_pool.total();
    ironsmith::special_actions::perform(ironsmith::special_actions::SpecialAction::Foretell { card_id: card },
        game, ALICE, &mut ironsmith::decision::SelectFirstDecisionMaker).unwrap();
    assert_eq!(before - game.player(ALICE).unwrap().mana_pool.total(), 2);
    let exile = game.find_object_by_stable_id(stable).unwrap();
    assert_ne!(exile, card);
    assert_eq!(game.object(exile).unwrap().zone, Zone::Exile);
    assert!(game.is_foretold(exile) && game.is_face_down(exile));
    assert!(cast_action(game, exile, true).is_none());
    game.turn.turn_number += 1;
    main_phase(game, ALICE);
    exile
}
fn assert_receipt(game: &GameState, spell: ObjectId, expected: bool) {
    assert_eq!(game.object(spell).unwrap().optional_costs_paid.cast_was_foretold, Some(expected));
    assert_eq!(game.stack.iter().find(|entry| entry.object_id == spell).unwrap().optional_costs_paid.cast_was_foretold, Some(expected));
}

#[test]
fn full_foretold_bodies_keep_the_predicate_and_true_self_replacements() {
    for name in ["Poison the Cup", "Starnheim Unleashed", "Haunting Voyage"] {
        for definition in definitions(name) {
            assert_eq!(definition.alternative_casts.len(), 1);
            let rendered = ironsmith::compiled_text::debug_compiled_lines(&definition).join("\n");
            assert!(rendered.contains("was foretold"), "{name}: {rendered}");
            if name != "Poison the Cup" {
                assert!(rendered.contains("instead"), "{name}: {rendered}");
                assert!(definition.spell_effect.as_ref().unwrap().segments.iter().any(|s| !s.self_replacements.is_empty()));
            }
        }
    }
}
#[test]
fn poison_full_body_scries_only_for_a_foretold_cast_with_a_legal_target() {
    for definition in definitions("Poison the Cup") {
        for (foretold, illegal) in [(false, false), (true, false), (true, true)] {
            let mut game = setup(); library(&mut game, 4);
            let victim = creature(&mut game, BOB, ironsmith::Subtype::Goblin, Zone::Battlefield);
            let victim_stable = game.object(victim).unwrap().stable_id;
            let card = game.create_object_from_definition(&definition, ALICE, Zone::Hand);
            let card = if foretold { foretell(&mut game, card) } else { card };
            let mut choices = Choices { target: Some(Target::Object(victim)), ..Default::default() };
            let spell = cast(&mut game, card, foretold, if foretold { 2 } else { 3 }, &mut choices);
            assert_receipt(&game, spell, foretold);
            if illegal { game.move_object_by_effect(victim, Zone::Hand).unwrap(); }
            finish(&mut game, &mut choices);
            assert_eq!(choices.scry_cards, if foretold && !illegal { 2 } else { 0 });
            let victim = game.find_object_by_stable_id(victim_stable).unwrap();
            assert_eq!(game.object(victim).unwrap().zone, if illegal { Zone::Hand } else { Zone::Graveyard });
        }
    }
}
#[test]
fn starnheim_replaces_the_single_token_and_pays_both_x_pips() {
    for definition in definitions("Starnheim Unleashed") {
        for (foretold, x, count, cost) in [(false, 0, 1, 4), (true, 0, 0, 1), (true, 3, 3, 7)] {
            let mut game = setup();
            let card = game.create_object_from_definition(&definition, ALICE, Zone::Hand);
            let card = if foretold { foretell(&mut game, card) } else { card };
            let mut choices = Choices { x, ..Default::default() };
            let spell = cast(&mut game, card, foretold, cost, &mut choices);
            assert_receipt(&game, spell, foretold); finish(&mut game, &mut choices);
            assert_eq!(game.battlefield.len(), count);
            for id in &game.battlefield {
                let token = game.object(*id).unwrap();
                assert!(token.subtypes.contains(&ironsmith::Subtype::Angel) && token.subtypes.contains(&ironsmith::Subtype::Warrior));
                assert_eq!(token.base_power, Some(ironsmith::card::PtValue::Fixed(4))); assert_eq!(token.base_toughness, Some(ironsmith::card::PtValue::Fixed(4)));
                assert!(game.object_has_static_ability_id(*id, ironsmith::static_abilities::StaticAbilityId::Flying));
                assert!(game.object_has_static_ability_id(*id, ironsmith::static_abilities::StaticAbilityId::Vigilance));
            }
        }
    }
}
#[test]
fn haunting_voyage_chooses_a_type_then_returns_two_or_all_from_only_its_controllers_graveyard() {
    for definition in definitions("Haunting Voyage") {
        for foretold in [false, true] {
            let mut game = setup();
            for _ in 0..4 { creature(&mut game, ALICE, ironsmith::Subtype::Goblin, Zone::Graveyard); }
            creature(&mut game, ALICE, ironsmith::Subtype::Elf, Zone::Graveyard);
            creature(&mut game, BOB, ironsmith::Subtype::Goblin, Zone::Graveyard);
            let card = game.create_object_from_definition(&definition, ALICE, Zone::Hand);
            let card = if foretold { foretell(&mut game, card) } else { card };
            let mut choices = Choices::default();
            let spell = cast(&mut game, card, foretold, if foretold { 7 } else { 6 }, &mut choices);
            assert_receipt(&game, spell, foretold); finish(&mut game, &mut choices);
            assert_eq!(game.battlefield.len(), if foretold { 4 } else { 2 });
            assert!(game.battlefield.iter().all(|id| { let o = game.object(*id).unwrap();
                o.owner == ALICE && o.subtypes.contains(&ironsmith::Subtype::Goblin) }));
            assert_eq!(game.player(BOB).unwrap().graveyard.len(), 1);
        }
    }
}
#[test]
fn missing_recovered_foretell_evidence_is_unknown_and_new_incarnations_lose_the_receipt() {
    for definition in definitions("Starnheim Unleashed") {
        let mut game = setup();
        let card = game.create_object_from_definition(&definition, ALICE, Zone::Hand);
        let card = foretell(&mut game, card);
        let spell = cast(&mut game, card, true, 5, &mut Choices { x: 2, ..Default::default() });
        let paid = game.object(spell).unwrap().optional_costs_paid.clone();
        let mut wire = serde_json::to_value(&paid).unwrap();
        assert_eq!(serde_json::from_value::<ironsmith::cost::OptionalCostsPaid>(wire.clone()).unwrap(), paid);
        wire.as_object_mut().unwrap().remove("cast_was_foretold");
        let recovered = serde_json::from_value::<ironsmith::cost::OptionalCostsPaid>(wire).unwrap();
        let ctx = ironsmith::effects::EffectContext::new_default(spell, ALICE).with_optional_costs_paid(recovered);
        for condition in [ironsmith::ConditionExpr::ThisSpellWasForetold,
            ironsmith::ConditionExpr::Not(Box::new(ironsmith::ConditionExpr::ThisSpellWasForetold))] {
            assert!(matches!(ironsmith::condition_eval::evaluate_condition_resolution(&game, &condition, &ctx),
                Err(ironsmith::effects::ExecutionError::IncompleteEvidence(_))));
        }
        let grave = game.move_object_by_effect(spell, Zone::Graveyard).unwrap();
        assert_eq!(game.object(grave).unwrap().optional_costs_paid.cast_was_foretold, None);
    }
}
#[test]
fn an_actual_copied_foretold_starnheim_uses_its_ordinary_body_while_the_original_uses_x() {
    use ironsmith::effects::{EffectExecutor, EffectContext as ExecutionContext, ResolvedTarget};
    for definition in definitions("Starnheim Unleashed") {
        let mut game = setup();
        let card = game.create_object_from_definition(&definition, ALICE, Zone::Hand);
        let card = foretell(&mut game, card);
        let mut choices = Choices { x: 3, ..Default::default() };
        let spell = cast(&mut game, card, true, 7, &mut choices);
        let casts = game.turn_store.turn_history.spells_cast_by_player(ALICE);
        let mut ctx = ExecutionContext::new(spell, ALICE, &mut choices);
        ctx.targets = vec![ResolvedTarget::Object(spell)];
        ironsmith::effects::CopySpellEffect::new(ironsmith::target::ChooseSpec::spell(), 1).execute(&mut game, &mut ctx).unwrap();
        assert_eq!(game.stack.len(), 2);
        let copy = game.stack.last().unwrap();
        assert_ne!(copy.object_id, spell); assert_eq!(copy.x_value, Some(3));
        assert_eq!(copy.optional_costs_paid.cast_was_foretold, Some(false));
        assert_eq!(game.turn_store.turn_history.spells_cast_by_player(ALICE), casts);
        resolve_stack_entry_with(&mut game, &mut choices).unwrap();
        assert_eq!(game.battlefield.len(), 1); assert_receipt(&game, spell, true);
        finish(&mut game, &mut choices); assert_eq!(game.battlefield.len(), 4);
    }
}
#[test]
fn independent_same_turn_permission_keeps_designation_without_a_foretell_payment() {
    for definition in definitions("Poison the Cup") {
        for foretold in [false, true] {
            let mut game = setup(); library(&mut game, 3);
            let source = creature(&mut game, ALICE, ironsmith::Subtype::Wizard, Zone::Battlefield);
            let victim = creature(&mut game, BOB, ironsmith::Subtype::Goblin, Zone::Battlefield);
            let card = game.create_object_from_definition(&definition, ALICE, Zone::Hand);
            let stable = game.object(card).unwrap().stable_id;
            if foretold {
                ironsmith::special_actions::perform(ironsmith::special_actions::SpecialAction::Foretell { card_id: card },
                    &mut game, ALICE, &mut Choices::default()).unwrap();
            } else { game.move_object_by_effect(card, Zone::Exile).unwrap(); }
            let exile = game.find_object_by_stable_id(stable).unwrap();
            assert_eq!(game.is_foretold(exile), foretold);
            assert!(cast_action(&game, exile, true).is_none());
            game.effect_store.grant_registry.grant_play_from_to_card(exile, Zone::Exile, ALICE,
                Default::default(), ironsmith::grant_registry::GrantSource::Effect {
                    source_id: source, expires_end_of_turn: game.turn.turn_number });
            let mut choices = Choices { target: Some(Target::Object(victim)), ..Default::default() };
            let spell = cast(&mut game, exile, false, 3, &mut choices);
            assert!(matches!(game.stack.last().unwrap().casting_method, CastingMethod::PlayFrom { use_alternative: None, .. }));
            assert!(!game.stack.last().unwrap().optional_costs_paid.was_paid_label("Foretell"));
            assert_receipt(&game, spell, foretold);
            game.move_object_by_effect(source, Zone::Graveyard).unwrap();
            finish(&mut game, &mut choices); assert_eq!(choices.scry_cards, if foretold { 2 } else { 0 });
        }
    }
}

#[test]
fn cosmos_modifies_only_the_special_action_and_keeps_flash_flying_and_its_later_price() {
    for definition in definitions("Cosmos Charger") {
        for providers in 0..=3 {
            let mut game = setup();
            let sources = (0..providers).map(|_| game.create_object_from_definition(&definition, ALICE, Zone::Battlefield)).collect::<Vec<_>>();
            for id in &sources {
                assert!(game.object_has_static_ability_id(*id, ironsmith::static_abilities::StaticAbilityId::Flash));
                assert!(game.object_has_static_ability_id(*id, ironsmith::static_abilities::StaticAbilityId::Flying));
            }
            let card = game.create_object_from_definition(&definition, ALICE, Zone::Hand);
            main_phase(&mut game, BOB);
            let action = ironsmith::special_actions::SpecialAction::Foretell { card_id: card };
            if providers == 0 {
                assert!(ironsmith::special_actions::can_perform_check(&action, &game, ALICE).is_err());
                main_phase(&mut game, ALICE);
            }
            assert!(compute_legal_actions(&game, ALICE).unwrap().contains(&LegalAction::SpecialAction(action.clone())));
            let stable = game.object(card).unwrap().stable_id;
            let before = game.player(ALICE).unwrap().mana_pool.total();
            ironsmith::special_actions::perform(action, &mut game, ALICE, &mut Choices::default()).unwrap();
            assert_eq!(before - game.player(ALICE).unwrap().mana_pool.total(), 2u32.saturating_sub(providers));
            let exile = game.find_object_by_stable_id(stable).unwrap();
            assert!(game.is_foretold(exile)); assert!(cast_action(&game, exile, true).is_none());
            for source in sources { game.move_object_by_effect(source, Zone::Graveyard).unwrap(); }
            game.turn.turn_number += 1; main_phase(&mut game, BOB);
            let mut choices = Choices::default();
            let spell = cast(&mut game, exile, true, 3, &mut choices);
            assert_receipt(&game, spell, true); finish(&mut game, &mut choices);
            let permanent = game.find_object_by_stable_id(stable).unwrap();
            assert_eq!(game.object(permanent).unwrap().zone, Zone::Battlefield);
            assert!(game.object_has_static_ability_id(permanent, ironsmith::static_abilities::StaticAbilityId::Flying));
        }
    }
}
#[test]
fn cosmos_permission_tracks_control_ability_loss_departure_and_phasing() {
    use ironsmith::continuous::{ContinuousEffect, EffectTarget, Modification};
    for definition in definitions("Cosmos Charger") {
        for unavailable in ["opponent controls", "abilities lost", "left battlefield", "phased charger", "unrelated phased permanent"] {
            let mut game = setup();
            let source = if unavailable == "unrelated phased permanent" {
                creature(&mut game, ALICE, ironsmith::Subtype::Bear, Zone::Battlefield)
            } else { game.create_object_from_definition(&definition, ALICE, Zone::Battlefield) };
            match unavailable {
                "opponent controls" => {
                    game.effect_store.continuous_effects.add_effect(ContinuousEffect::new(source, ALICE,
                        EffectTarget::Specific(source), Modification::ChangeController(BOB)));
                }
                "abilities lost" => {
                    game.effect_store.continuous_effects.add_effect(ContinuousEffect::new(source, ALICE,
                        EffectTarget::Specific(source), Modification::RemoveAllAbilities));
                }
                "left battlefield" => { game.move_object_by_effect(source, Zone::Graveyard).unwrap(); }
                _ => game.phase_out(source),
            }
            let card = game.create_object_from_definition(&definition, ALICE, Zone::Hand);
            main_phase(&mut game, BOB);
            let action = ironsmith::special_actions::SpecialAction::Foretell { card_id: card };
            assert!(!compute_legal_actions(&game, ALICE).unwrap().contains(&LegalAction::SpecialAction(action.clone())), "{unavailable}");
            assert!(matches!(ironsmith::special_actions::can_perform_check(&action, &game, ALICE),
                Err(ironsmith::special_actions::ActionError::NotActivePlayer)), "{unavailable}");
            main_phase(&mut game, ALICE);
            let before = game.player(ALICE).unwrap().mana_pool.total();
            ironsmith::special_actions::perform(action, &mut game, ALICE, &mut Choices::default()).unwrap();
            assert_eq!(before - game.player(ALICE).unwrap().mana_pool.total(), 2, "{unavailable}");
        }
    }
}

#[derive(Default)]
struct ForetellPaymentChoices { prompts: usize, pending: bool, cancel_after_activation: bool }
impl DecisionMaker for ForetellPaymentChoices {
    fn awaiting_choice(&self) -> bool { self.pending && self.prompts > 0 }
    fn decide_mana_payment(&mut self, game: &GameState, ctx: &ironsmith::decisions::context::ManaPaymentContext)
        -> ironsmith::mana_payment::ManaPaymentResponse {
        use ironsmith::mana_payment::ManaPaymentResponse;
        self.prompts += 1;
        if self.pending { return ManaPaymentResponse::Cancel; }
        if self.cancel_after_activation {
            if self.prompts == 1 {
                let (source, ability_index) = ironsmith::mana_payment::manual_mana_abilities(game, &ctx.request)[0];
                return ManaPaymentResponse::Activate { source, ability_index };
            }
            return ManaPaymentResponse::Cancel;
        }
        ManaPaymentResponse::Confirm { plan_id: ctx.plan.id, request_hash: ctx.plan.request_hash }
    }
}
#[test]
fn cosmos_foretell_pending_and_cancelled_payment_leave_no_designation_or_spent_resources() {
    for definition in definitions("Cosmos Charger") {
        let mut game = setup();
        game.player_mut(ALICE).unwrap().mana_pool = Default::default();
        game.create_object_from_definition(&definition, ALICE, Zone::Battlefield);
        let land = CardDefinitionBuilder::new(CardId::new(), "Foretell payment land")
            .card_types(vec![CardType::Land]).build();
        let land = game.create_object_from_definition(&land, ALICE, Zone::Battlefield);
        game.object_mut(land).unwrap().abilities_mut().push(ironsmith::ability::Ability::mana(
            ironsmith::cost::TotalCost::from_cost(ironsmith::costs::Cost::tap()), vec![ManaSymbol::Colorless]));
        let card = game.create_object_from_definition(&definition, ALICE, Zone::Hand);
        let action = ironsmith::special_actions::SpecialAction::Foretell { card_id: card };
        let mut pending = ForetellPaymentChoices { pending: true, ..Default::default() };
        let _ = ironsmith::special_actions::perform(action.clone(), &mut game, ALICE, &mut pending);
        assert!(pending.awaiting_choice());
        assert_eq!(game.object(card).unwrap().zone, Zone::Hand);
        assert!(game.exile.is_empty() && !game.is_tapped(land));
        assert!(!game.has_foretold_this_turn(ALICE));
        let mut cancel = ForetellPaymentChoices { cancel_after_activation: true, ..Default::default() };
        assert_eq!(ironsmith::special_actions::perform(action.clone(), &mut game, ALICE, &mut cancel),
            Err(ironsmith::special_actions::ActionError::Cancelled));
        assert_eq!(cancel.prompts, 2);
        assert_eq!(game.object(card).unwrap().zone, Zone::Hand);
        assert_eq!(game.player(ALICE).unwrap().mana_pool.total(), 0);
        assert!(game.exile.is_empty() && !game.is_tapped(land));
        assert!(!game.has_foretold_this_turn(ALICE));
        ironsmith::special_actions::perform(action, &mut game, ALICE, &mut ForetellPaymentChoices::default()).unwrap();
        assert!(game.object(card).is_none() && game.is_tapped(land));
        assert_eq!(game.exile.len(), 1);
        assert!(game.is_foretold(game.exile[0]) && game.has_foretold_this_turn(ALICE));
    }
}
