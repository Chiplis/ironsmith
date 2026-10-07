//! Complete frozen Katilda body; source-authored regressions, execution deferred.
use ironsmith::ability::AbilityKind;
use ironsmith::card::{CardBuilder, PowerToughness};
use ironsmith::cards::CardDefinition;
use ironsmith::color::ColorSet;
use ironsmith::continuous::{EffectTarget, Modification};
use ironsmith::decision::{DecisionMaker, LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
use ironsmith::decisions::context::{ColorsContext, SelectOptionsContext};
use ironsmith::effect::Until;
use ironsmith::effects::{AddOneManaOfAnyColorAmongEffect, ApplyContinuousEffect, EffectContext, EffectExecutor};
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, resolve_stack_entry_with};
use ironsmith::mana::ManaSymbol;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{CardId, CardType, Color, CounterType, GameProgress, GameState, ObjectId, PlayerId, Subtype, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;

const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);

fn definitions() -> [CardDefinition; 3] {
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../fixtures/complete_protection_bodies.json.fixture")).unwrap();
    let row = rows.iter().find(|row| row["name"] == "Katilda, Dawnhart Prime").unwrap();
    let name = row["name"].as_str().unwrap();
    let text = row["text"].as_str().unwrap();
    let (result, loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_artifact(name, text, false));
    let (artifact, _) = result.unwrap();
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    artifact.validate().unwrap();
    let decoded = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, decoded);
    let (result, loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_runtime_definition(name, text, false));
    let direct = result.unwrap();
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let wire = ironsmith_runtime_catalog::artifact_materializer::encode_runtime_definition(direct.clone()).unwrap();
    let native = ironsmith_runtime_catalog::artifact_materializer::materialize_definition(
        serde_json::from_slice(&serde_json::to_vec(&wire).unwrap()).unwrap()).unwrap();
    for definition in [&direct, &native] {
        assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(definition));
    }
    [direct, materialize_artifact(&decoded).unwrap(), native]
}

fn game() -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.active_player = A;
    game.turn.priority_player = Some(A);
    game.turn.phase = ironsmith::Phase::FirstMain;
    game.turn.step = None;
    game
}

fn creature(game: &mut GameState, player: PlayerId, subtype: Subtype, colors: ColorSet) -> ObjectId {
    let card = CardBuilder::new(CardId::new(), "Granted mana witness")
        .card_types(vec![CardType::Creature]).subtypes(vec![subtype])
        .color_indicator(colors).power_toughness(PowerToughness::fixed(2, 2)).build();
    game.create_object_from_card(&card, player, Zone::Battlefield)
}

fn change(game: &mut GameState, target: ObjectId, modification: Modification) {
    ApplyContinuousEffect::new(EffectTarget::Specific(target), modification, Until::Forever)
        .execute(game, &mut EffectContext::new_default(target, A)).unwrap();
    game.refresh_continuous_state().unwrap();
}

fn mana_effect(game: &GameState, source: ObjectId) -> Option<AddOneManaOfAnyColorAmongEffect> {
    game.current_abilities(source).unwrap().iter().find_map(|ability| {
        let AbilityKind::Activated(activated) = &ability.kind else { return None; };
        if !activated.is_mana_ability() { return None; }
        activated.effects.iter().find_map(|effect|
            effect.downcast_ref::<AddOneManaOfAnyColorAmongEffect>().cloned())
    })
}

fn mana_action(game: &GameState, player: PlayerId, source: ObjectId) -> Option<LegalAction> {
    compute_legal_actions(game, player).unwrap().into_iter().find(|action|
        matches!(action, LegalAction::ActivateManaAbility { source: id, .. } if *id == source))
}

struct ChooseColor(Color);
impl DecisionMaker for ChooseColor {
    fn decide_colors(&mut self, _: &GameState, context: &ColorsContext) -> Vec<Color> {
        assert_eq!(context.count, 1);
        assert!(context.available_colors.as_ref().unwrap().contains(&self.0));
        vec![self.0]
    }
    fn decide_options(&mut self, game: &GameState, context: &SelectOptionsContext) -> Vec<usize> {
        if let Some(option) = context.options.iter().find(|option| option.description == format!("{:?}", self.0)) {
            vec![option.index]
        } else { SelectFirstDecisionMaker.decide_options(game, context) }
    }
}

fn activate(game: &mut GameState, action: LegalAction, dm: &mut impl DecisionMaker) {
    let mut queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(2);
    let mut progress = apply_priority_response_with_dm(game, &mut queue, &mut state,
        &PriorityResponse::PriorityAction(action), dm).unwrap();
    for _ in 0..40 {
        if state.pending_activation.is_none() && state.pending_mana_ability.is_none() { return; }
        let GameProgress::NeedsDecisionCtx(context) = progress else { panic!("{progress:?}"); };
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &context, dm).unwrap();
    }
    panic!("activation did not finish");
}

#[test]
fn complete_katilda_grants_each_recipient_its_own_current_colors_and_resolves_without_stack() {
    for definition in definitions() {
        let rendered = ironsmith_text::compiled_text_lines(&definition).join("\n");
        assert!(rendered.contains("Add one mana of any of this creature's colors"), "{rendered}");
        let mut game = game();
        let katilda = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let human = creature(&mut game, A, Subtype::Human, ColorSet::RED.union(ColorSet::BLUE));
        assert!(mana_effect(&game, katilda).is_some(), "Katilda grants herself the ability");
        assert!(mana_action(&game, A, human).is_none(), "tap mana costs obey summoning sickness");
        game.remove_summoning_sickness(human);
        for (colors, chosen, expected) in [
            (ColorSet::RED.union(ColorSet::BLUE), Color::Red, vec![ManaSymbol::Blue, ManaSymbol::Red]),
            (ColorSet::BLACK.union(ColorSet::GREEN), Color::Black, vec![ManaSymbol::Black, ManaSymbol::Green]),
            (ColorSet::COLORLESS, Color::Red, vec![]),
        ] {
            change(&mut game, human, Modification::SetColors(colors));
            let effect = mana_effect(&game, human).unwrap();
            assert!(effect.filter.is_source_only());
            assert_eq!(effect.producible_mana_symbols(&game, human, A).unwrap_or_default(), expected);
            let action = mana_action(&game, A, human).expect("colorless humans still have a mana ability");
            let before = game.player(A).unwrap().mana_pool.total();
            activate(&mut game, action, &mut ChooseColor(chosen));
            assert!(game.stack.is_empty(), "mana activation resolves immediately");
            assert!(game.is_tapped(human));
            assert!(!game.is_tapped(katilda), "the recipient pays its own tap cost");
            assert_eq!(game.player(A).unwrap().mana_pool.total(), before + u32::from(!colors.is_empty()));
            assert!(mana_action(&game, A, human).is_none());
            game.untap(human);
            game.turn.priority_player = Some(A);
        }
        let pool = &game.player(A).unwrap().mana_pool;
        assert_eq!((pool.red, pool.black, pool.white, pool.blue, pool.green, pool.colorless), (1, 1, 0, 0, 0, 0));
    }
}

#[test]
fn katilda_grant_tracks_current_subtypes_types_and_controller() {
    for definition in definitions() {
        let mut game = game();
        let katilda = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let human = creature(&mut game, A, Subtype::Human, ColorSet::RED);
        let elf = creature(&mut game, A, Subtype::Elf, ColorSet::BLUE);
        let enemy = creature(&mut game, B, Subtype::Human, ColorSet::BLACK);
        assert!(mana_effect(&game, human).is_some());
        assert!(mana_effect(&game, elf).is_none());
        assert!(mana_effect(&game, enemy).is_none());
        change(&mut game, elf, Modification::SetSubtypes(vec![Subtype::Human]));
        assert!(mana_effect(&game, elf).is_some());
        change(&mut game, human, Modification::SetCardTypes(vec![CardType::Artifact]));
        assert!(mana_effect(&game, human).is_none(), "Human subtype alone does not make a creature");
        change(&mut game, katilda, Modification::ChangeController(B));
        assert!(mana_effect(&game, elf).is_none());
        assert!(mana_effect(&game, enemy).is_some());
        game.remove_summoning_sickness(enemy);
        game.turn.priority_player = Some(B);
        let action = mana_action(&game, B, enemy).unwrap();
        activate(&mut game, action, &mut ChooseColor(Color::Black));
        assert_eq!(game.player(B).unwrap().mana_pool.black, 1);
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
    }
}

#[test]
fn katilda_paid_counter_activation_pays_six_and_taps_then_uses_the_stack() {
    for definition in definitions() {
        let mut game = game();
        let katilda = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let human = creature(&mut game, A, Subtype::Human, ColorSet::COLORLESS);
        let elf = creature(&mut game, A, Subtype::Elf, ColorSet::GREEN);
        let enemy = creature(&mut game, B, Subtype::Human, ColorSet::BLUE);
        let index = game.current_abilities(katilda).unwrap().iter().position(|ability|
            matches!(&ability.kind, AbilityKind::Activated(activated) if !activated.is_mana_ability())).unwrap();
        let action = LegalAction::ActivateAbility { source: katilda, ability_index: index };
        for (symbol, count) in [(ManaSymbol::Colorless, 3), (ManaSymbol::Green, 1), (ManaSymbol::White, 1)] {
            game.player_mut(A).unwrap().mana_pool.add(symbol, count);
        }
        game.remove_summoning_sickness(katilda);
        assert!(!compute_legal_actions(&game, A).unwrap().contains(&action));
        game.player_mut(A).unwrap().mana_pool.add(ManaSymbol::Colorless, 1);
        assert!(compute_legal_actions(&game, A).unwrap().contains(&action));
        activate(&mut game, action, &mut SelectFirstDecisionMaker);
        assert!(game.is_tapped(katilda));
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
        assert_eq!(game.stack.len(), 1);
        assert_eq!(game.counter_count(human, CounterType::PlusOnePlusOne), 0);
        resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker).unwrap();
        for own in [katilda, human, elf] { assert_eq!(game.counter_count(own, CounterType::PlusOnePlusOne), 1); }
        assert_eq!(game.counter_count(enemy, CounterType::PlusOnePlusOne), 0);
    }
}

#[test]
fn source_color_mana_uses_exact_lki_and_rejects_absent_or_unrelated_evidence() {
    for definition in definitions() {
        let mut game = game();
        game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let human = creature(&mut game, A, Subtype::Human, ColorSet::RED);
        change(&mut game, human, Modification::SetColors(ColorSet::BLUE));
        let effect = mana_effect(&game, human).unwrap();
        let snapshot = ironsmith::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(game.object(human).unwrap(), &game);
        game.phase_out(human);
        let mut dm = ChooseColor(Color::Blue);
        effect.execute(&mut game, &mut EffectContext::new(human, A, &mut dm).with_source_snapshot(snapshot.clone())).unwrap();
        game.phase_in(human);
        game.move_object_by_effect(human, Zone::Graveyard).unwrap();
        effect.execute(&mut game, &mut EffectContext::new(human, A, &mut dm).with_source_snapshot(snapshot)).unwrap();
        assert_eq!(game.player(A).unwrap().mana_pool.blue, 2);
        assert_eq!(game.player(A).unwrap().mana_pool.red, 0);
        assert!(matches!(effect.execute(&mut game, &mut EffectContext::new(human, A, &mut dm)),
            Err(ironsmith::effects::ExecutionError::IncompleteEvidence(_))));
        assert_eq!(game.player(A).unwrap().mana_pool.blue, 2);
        let other = creature(&mut game, A, Subtype::Human, ColorSet::GREEN);
        let unrelated = ironsmith::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(
            game.object(other).unwrap(), &game);
        assert!(matches!(effect.execute(&mut game,
            &mut EffectContext::new(human, A, &mut dm).with_source_snapshot(unrelated)),
            Err(ironsmith::effects::ExecutionError::IncompleteEvidence(_))));
        assert_eq!(game.player(A).unwrap().mana_pool.blue, 2);
        assert_eq!(game.player(A).unwrap().mana_pool.green, 0);
    }
}

#[test]
fn source_color_mana_preserves_checked_discovery_failure_without_crediting_mana() {
    use ironsmith::effect::Effect;
    use ironsmith::effects::{ExecutionError, SequenceEffect, execute_effect};
    for definition in definitions() {
        let mut game = game();
        game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let human = creature(&mut game, A, Subtype::Human, ColorSet::RED);
        let effect = mana_effect(&game, human).unwrap();
        let life = game.player(A).unwrap().life;
        game.player_mut(B).unwrap().mana_pool.green = i32::MAX as u32 + 1;
        assert!(game.try_current_characteristics(human).is_err());
        let sequence = Effect::new(SequenceEffect::new(vec![Effect::gain_life(3), Effect::new(effect)]));
        let result = execute_effect(&mut game, &sequence,
            &mut EffectContext::new(human, A, &mut SelectFirstDecisionMaker));
        assert!(matches!(result, Err(ExecutionError::ContinuousDiscovery(_))
            | Err(ExecutionError::ResourceLimitExceeded { .. })));
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
        assert_eq!(game.player(A).unwrap().life, life);
    }
}
