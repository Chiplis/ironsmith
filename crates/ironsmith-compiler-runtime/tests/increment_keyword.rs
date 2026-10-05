//! Increment uses actual caster payment, not mana value or Assist payments.
//! Direct/restored artifacts and real casting transactions. Authored, unrun.
use ironsmith::ability::AbilityKind;
use ironsmith::cards::CardDefinition;
use ironsmith::decision::{DecisionMaker, LegalAction, SelectFirstDecisionMaker};
use ironsmith::effects::{EffectContext, EffectExecutor};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, put_triggers_on_stack_with_dm, resolve_stack_entry_with,
};
use ironsmith::mana::ManaSymbol;
use ironsmith::object::CounterType;
use ironsmith::target::ChooseSpec;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{GameState, ObjectId, Phase, PlayerId, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_core::{Condition, PlayerFilter, TriggerKind};
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
fn definitions_from(name: &str, text: &str) -> [CardDefinition; 2] {
    let (artifact, direct) =
        compile_to_artifact(name, text, false).unwrap_or_else(|error| panic!("{name}: {error}"));
    let restored: CompiledCardArtifact =
        serde_json::from_slice(&serde_json::to_vec(&artifact).unwrap()).unwrap();
    restored.validate().unwrap();
    [direct, materialize_artifact(&restored).unwrap()]
}
fn fixtures() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!(
        "../../../fixtures/increment_keyword.json.fixture"
    ))
    .unwrap()
}
fn definitions(name: &str) -> [CardDefinition; 2] {
    let row = fixtures()
        .into_iter()
        .find(|row| row["name"] == name)
        .unwrap();
    definitions_from(name, row["text"].as_str().unwrap())
}
fn game() -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.priority_player = Some(A);
    game
}
fn card(game: &mut GameState, owner: PlayerId, zone: Zone, text: &str) -> ObjectId {
    let definition = compile_to_runtime_definition("Increment resource", text, false).unwrap();
    game.create_object_from_definition(&definition, owner, zone)
}
fn counters(game: &GameState, id: ObjectId) -> u32 {
    game.object(id)
        .unwrap()
        .counters
        .get(&CounterType::PlusOnePlusOne)
        .copied()
        .unwrap_or(0)
}
fn stack(game: &mut GameState, dm: &mut impl DecisionMaker) -> usize {
    put_triggers_on_stack_with_dm(game, &mut TriggerQueue::new(), dm).unwrap();
    game.stack.len()
}
fn settle(game: &mut GameState, dm: &mut impl DecisionMaker) {
    stack(game, dm);
    for _ in 0..40 {
        if game.stack_is_empty() {
            return;
        }
        resolve_stack_entry_with(game, dm).unwrap();
        stack(game, dm);
    }
    panic!("increment did not settle")
}
fn execute(
    game: &mut GameState,
    source: ObjectId,
    controller: PlayerId,
    effect: &dyn EffectExecutor,
) {
    let mut dm = SelectFirstDecisionMaker;
    let mut ctx = EffectContext::new(source, controller, &mut dm);
    let outcome = effect.execute(game, &mut ctx).unwrap();
    for event in outcome.events {
        game.queue_trigger_event(event.provenance(), event);
    }
}
fn announce(
    game: &mut GameState,
    spell: &CardDefinition,
    caster: PlayerId,
    dm: &mut impl DecisionMaker,
) -> ObjectId {
    game.turn.active_player = caster;
    game.turn.priority_player = Some(caster);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    let hand = game.create_object_from_definition(spell, caster, Zone::Hand);
    let stable = game.object(hand).unwrap().stable_id;
    let action = LegalAction::CastSpell {
        spell_id: hand,
        from_zone: Zone::Hand,
        casting_method: ironsmith::alternative_cast::CastingMethod::Normal,
    };
    assert!(
        ironsmith::decision::compute_legal_actions(game, caster)
            .unwrap()
            .contains(&action)
    );
    let mut state = PriorityLoopState::new(game.players.len());
    let mut queue = TriggerQueue::new();
    let mut progress = apply_priority_response_with_dm(
        game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        dm,
    )
    .unwrap();
    for _ in 0..40 {
        if state.pending_cast.is_none() && state.pending_method_selection.is_none() {
            break;
        }
        let ironsmith::GameProgress::NeedsDecisionCtx(context) = progress else {
            panic!("cast pending without a decision")
        };
        progress =
            apply_decision_context_with_dm(game, &mut queue, &mut state, &context, dm).unwrap();
    }
    assert!(state.pending_cast.is_none() && state.pending_method_selection.is_none());
    put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap();
    game.find_object_by_stable_id(stable).unwrap()
}
fn spell(amount: u32) -> CardDefinition {
    compile_to_runtime_definition(
        "Increment spell",
        &format!("Mana cost: {{{amount}}}\nType: Sorcery\nYou gain 1 life."),
        false,
    )
    .unwrap()
}

#[test]
fn nine_exact_cards_have_real_cast_trigger_intervening_comparison_and_counter_body() {
    assert_eq!(fixtures().len(), 9);
    for row in fixtures() {
        for definition in definitions(row["name"].as_str().unwrap()) {
            let triggered = definition
                .abilities
                .iter()
                .find_map(|ability| match &ability.kind {
                    AbilityKind::Triggered(triggered)
                        if triggered.intervening_if == Some(Condition::increment()) =>
                    {
                        Some(triggered)
                    }
                    _ => None,
                })
                .expect("Increment must expand to an executable trigger");
            assert!(matches!(
                &triggered.trigger.compiled_model().unwrap().kind,
                TriggerKind::SpellCast {
                    filter: None,
                    caster: PlayerFilter::You
                }
            ));
            assert_eq!(triggered.effects.flattened_default_effects().len(), 1);
            let mut game = game();
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            for _ in 0..4 {
                card(&mut game, A, Zone::Library, "Type: Artifact");
            }
            game.player_mut(A)
                .unwrap()
                .mana_pool
                .add(ManaSymbol::Colorless, 5);
            let cast = announce(&mut game, &spell(5), A, &mut SelectFirstDecisionMaker);
            assert_eq!(
                game.object(cast).unwrap().caster_mana_spent_to_cast,
                Some(5)
            );
            settle(&mut game, &mut SelectFirstDecisionMaker);
            assert_eq!(counters(&game, source), 1, "{}", row["name"]);
            game.player_mut(B)
                .unwrap()
                .mana_pool
                .add(ManaSymbol::Colorless, 5);
            announce(&mut game, &spell(5), B, &mut SelectFirstDecisionMaker);
            settle(&mut game, &mut SelectFirstDecisionMaker);
            assert_eq!(counters(&game, source), 1);
        }
    }
}

#[test]
fn strict_greater_than_either_dimension_is_not_and_not_mana_value() {
    for (power, toughness, paid, expected) in [
        (1, 5, 2, 1),
        (5, 1, 2, 1),
        (2, 2, 2, 0),
        (1, 1, 0, 0),
        (-1, 4, 0, 1),
    ] {
        for definition in definitions_from(
            "Increment comparison",
            &format!("Type: Creature\nPower/Toughness: {power}/{toughness}\nIncrement"),
        ) {
            let mut game = game();
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            game.player_mut(A)
                .unwrap()
                .mana_pool
                .add(ManaSymbol::Colorless, paid);
            announce(&mut game, &spell(paid), A, &mut SelectFirstDecisionMaker);
            settle(&mut game, &mut SelectFirstDecisionMaker);
            assert_eq!(
                counters(&game, source),
                expected,
                "{power}/{toughness}, paid {paid}"
            );
        }
    }
    for definition in definitions_from(
        "Increment payment",
        "Type: Creature\nPower/Toughness: 2/5\nIncrement",
    ) {
        for (modifier, base, paid, expected) in [
            ("Spells you cast cost {2} less to cast.", 4, 2, 0),
            ("Spells you cast cost {2} more to cast.", 1, 3, 1),
        ] {
            let mut game = game();
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            card(
                &mut game,
                A,
                Zone::Battlefield,
                &format!("Type: Enchantment\n{modifier}"),
            );
            game.player_mut(A)
                .unwrap()
                .mana_pool
                .add(ManaSymbol::Colorless, paid);
            let cast = announce(&mut game, &spell(base), A, &mut SelectFirstDecisionMaker);
            assert_eq!(
                game.object(cast).unwrap().caster_mana_spent_to_cast,
                Some(paid)
            );
            settle(&mut game, &mut SelectFirstDecisionMaker);
            assert_eq!(counters(&game, source), expected);
        }
    }
}

#[test]
fn condition_rechecks_current_source_stats_but_keeps_cast_payment_after_spell_is_countered() {
    for definition in definitions_from(
        "Increment timing",
        "Type: Creature\nPower/Toughness: 1/1\nIncrement",
    ) {
        for raise_stats in [false, true] {
            let mut game = game();
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            game.player_mut(A)
                .unwrap()
                .mana_pool
                .add(ManaSymbol::Colorless, 2);
            let cast = announce(&mut game, &spell(2), A, &mut SelectFirstDecisionMaker);
            assert_eq!(game.stack.len(), 2);
            if raise_stats {
                execute(
                    &mut game,
                    source,
                    A,
                    &ironsmith::effects::ApplyContinuousEffect::new(
                        ironsmith::continuous::EffectTarget::Specific(source),
                        ironsmith::continuous::Modification::ModifyPowerToughness {
                            power: 3,
                            toughness: 3,
                        },
                        ironsmith::effect::Until::EndOfTurn,
                    ),
                );
            } else {
                execute(
                    &mut game,
                    source,
                    B,
                    &ironsmith::effects::CounterEffect::new(ChooseSpec::SpecificObject(cast)),
                );
                assert!(!game.stack.iter().any(|entry| entry.object_id == cast));
            }
            settle(&mut game, &mut SelectFirstDecisionMaker);
            assert_eq!(counters(&game, source), u32::from(!raise_stats));
        }
    }
}

#[test]
fn convoke_taps_are_not_mana_spent() {
    let spells = definitions_from(
        "Convoke increment spell",
        "Mana cost: {2}\nType: Sorcery\nConvoke\nYou gain 1 life.",
    );
    for (observer, spell) in definitions_from(
        "Convoke increment observer",
        "Type: Creature\nPower/Toughness: 1/1\nIncrement",
    )
    .into_iter()
    .zip(spells)
    {
        let mut game = game();
        let source = game.create_object_from_definition(&observer, A, Zone::Battlefield);
        for _ in 0..2 {
            card(
                &mut game,
                A,
                Zone::Battlefield,
                "Type: Creature\nPower/Toughness: 1/1",
            );
        }
        let cast = announce(&mut game, &spell, A, &mut SelectFirstDecisionMaker);
        assert_eq!(
            game.object(cast).unwrap().caster_mana_spent_to_cast,
            Some(0)
        );
        settle(&mut game, &mut SelectFirstDecisionMaker);
        assert_eq!(counters(&game, source), 0);
    }
}

#[derive(Default)]
struct AssistChoice;
impl DecisionMaker for AssistChoice {
    fn decide_options(
        &mut self,
        _: &GameState,
        ctx: &ironsmith::decisions::context::SelectOptionsContext,
    ) -> Vec<usize> {
        if ctx
            .description
            .starts_with("Choose another player to assist")
        {
            return vec![1];
        }
        if ctx.description.starts_with("Confirm mana payment for") {
            return vec![1];
        }
        if ctx.description.starts_with("Choose how much generic mana") {
            return vec![2];
        }
        ctx.options
            .iter()
            .find(|option| option.legal)
            .map(|option| vec![option.index])
            .unwrap_or_default()
    }
    fn decide_number(
        &mut self,
        _: &GameState,
        _: &ironsmith::decisions::context::NumberContext,
    ) -> u32 {
        2
    }
}
#[test]
fn assist_preserves_total_payment_but_increment_counts_only_what_the_caster_spent() {
    let spells = definitions_from(
        "Assisted increment spell",
        "Mana cost: {2}{U}\nType: Sorcery\nAssist\nYou gain 1 life.",
    );
    for (observer, spell) in definitions_from(
        "Assisted increment observer",
        "Type: Creature\nPower/Toughness: 1/5\nIncrement",
    )
    .into_iter()
    .zip(spells)
    {
        let mut game = game();
        let source = game.create_object_from_definition(&observer, A, Zone::Battlefield);
        game.player_mut(A)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Blue, 1);
        game.player_mut(B)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Colorless, 2);
        let cast = announce(&mut game, &spell, A, &mut AssistChoice);
        let object = game.object(cast).unwrap();
        assert_eq!(object.mana_spent_to_cast.total(), 3);
        assert_eq!(object.caster_mana_spent_to_cast, Some(1));
        let snapshot = ironsmith::snapshot::ObjectSnapshot::from_object(object, &game);
        assert_eq!(snapshot.caster_mana_spent_to_cast, Some(1));
        settle(&mut game, &mut SelectFirstDecisionMaker);
        assert_eq!(counters(&game, source), 0);
        assert_eq!(game.player(B).unwrap().mana_pool.total(), 0);
    }
}

#[test]
fn ambitious_death_body_preserves_the_sources_counter_snapshot_for_the_fractal() {
    for definition in definitions("Ambitious Augmenter") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        game.add_counters(source, CounterType::PlusOnePlusOne, 2)
            .unwrap();
        execute(
            &mut game,
            source,
            A,
            &ironsmith::effects::DestroyEffect::with_spec(ChooseSpec::SpecificObject(source)),
        );
        settle(&mut game, &mut SelectFirstDecisionMaker);
        let token = *game
            .battlefield
            .iter()
            .find(|id| game.object(**id).unwrap().kind == ironsmith::object::ObjectKind::Token)
            .unwrap();
        assert_eq!(counters(&game, token), 2);
        assert_eq!(game.current_power(token), Some(2));
        assert!(!game.battlefield.contains(&source));
    }
}

#[test]
fn counter_placement_secondary_bodies_produce_mana_and_draws() {
    for name in ["Berta, Wise Extrapolator", "Pensive Professor"] {
        for definition in definitions(name) {
            let mut game = game();
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            card(&mut game, A, Zone::Library, "Type: Artifact");
            execute(
                &mut game,
                source,
                A,
                &ironsmith::effects::PutCountersEffect::new(
                    CounterType::PlusOnePlusOne,
                    2,
                    ChooseSpec::Source,
                ),
            );
            settle(&mut game, &mut SelectFirstDecisionMaker);
            assert_eq!(counters(&game, source), 2);
            if name == "Berta, Wise Extrapolator" {
                assert_eq!(game.player(A).unwrap().mana_pool.total(), 1);
            } else {
                assert_eq!(game.player(A).unwrap().hand.len(), 1);
            }
        }
    }
}

#[test]
fn fractal_tender_end_step_tracks_who_put_the_counter_not_just_the_recipient() {
    for definition in definitions("Fractal Tender") {
        for placer in [A, B] {
            let mut game = game();
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            execute(
                &mut game,
                source,
                placer,
                &ironsmith::effects::PutCountersEffect::new(
                    CounterType::PlusOnePlusOne,
                    1,
                    ChooseSpec::Source,
                ),
            );
            game.turn.phase = Phase::Ending;
            game.turn.step = Some(ironsmith::game_state::Step::End);
            game.queue_trigger_event(
                Default::default(),
                ironsmith::triggers::TriggerEvent::new(
                    ironsmith::events::BeginningOfEndStepEvent::new(A),
                    Default::default(),
                ),
            );
            settle(&mut game, &mut SelectFirstDecisionMaker);
            let tokens = game
                .battlefield
                .iter()
                .copied()
                .filter(|id| game.object(*id).unwrap().kind == ironsmith::object::ObjectKind::Token)
                .collect::<Vec<_>>();
            assert_eq!(tokens.len(), usize::from(placer == A));
            if let Some(token) = tokens.first() {
                assert_eq!(counters(&game, *token), 3);
                assert_eq!(game.current_power(*token), Some(3));
            }
        }
    }
}

#[test]
fn topiary_mana_activation_reads_current_incremented_power() {
    for definition in definitions("Topiary Lecturer") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        game.add_counters(source, CounterType::PlusOnePlusOne, 2)
            .unwrap();
        game.remove_summoning_sickness(source);
        let power = game.current_power(source).unwrap() as u32;
        let action = ironsmith::decision::compute_legal_actions(&game, A)
            .unwrap()
            .into_iter()
            .find(
                |action| matches!(action,LegalAction::ActivateManaAbility{source:id,..} if *id==source),
            )
            .unwrap();
        let mut state = PriorityLoopState::new(2);
        let mut queue = TriggerQueue::new();
        let mut dm = SelectFirstDecisionMaker;
        let mut progress = apply_priority_response_with_dm(
            &mut game,
            &mut queue,
            &mut state,
            &PriorityResponse::PriorityAction(action),
            &mut dm,
        )
        .unwrap();
        for _ in 0..30 {
            if state.pending_activation.is_none() {
                break;
            }
            let ironsmith::GameProgress::NeedsDecisionCtx(context) = progress else {
                panic!("mana activation pending")
            };
            progress = apply_decision_context_with_dm(
                &mut game, &mut queue, &mut state, &context, &mut dm,
            )
            .unwrap();
        }
        settle(&mut game, &mut dm);
        assert!(game.is_tapped(source));
        assert_eq!(game.player(A).unwrap().mana_pool.green, power);
    }
}

struct MoveCounterChoice(ObjectId);
impl DecisionMaker for MoveCounterChoice {
    fn decide_options(
        &mut self,
        game: &GameState,
        ctx: &ironsmith::decisions::context::SelectOptionsContext,
    ) -> Vec<usize> {
        if ctx.description.starts_with("Confirm mana payment for") {
            return vec![1];
        }
        let mut fallback = SelectFirstDecisionMaker;
        fallback.decide_options(game, ctx)
    }
    fn decide_number(
        &mut self,
        _: &GameState,
        _: &ironsmith::decisions::context::NumberContext,
    ) -> u32 {
        2
    }
    fn decide_boolean(
        &mut self,
        _: &GameState,
        _: &ironsmith::decisions::context::BooleanContext,
    ) -> bool {
        true
    }
    fn decide_targets(
        &mut self,
        _: &GameState,
        ctx: &ironsmith::decisions::context::TargetsContext,
    ) -> Vec<ironsmith::Target> {
        let target = ironsmith::Target::Object(self.0);
        assert!(
            ctx.requirements
                .iter()
                .any(|req| req.legal_targets.contains(&target))
        );
        vec![target]
    }
}
#[test]
fn tester_pays_x_then_its_reflexive_body_moves_that_many_counters() {
    for definition in definitions("Tester of the Tangential") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let recipient = card(
            &mut game,
            A,
            Zone::Battlefield,
            "Type: Creature\nPower/Toughness: 1/1",
        );
        game.add_counters(source, CounterType::PlusOnePlusOne, 3)
            .unwrap();
        game.player_mut(A)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Colorless, 2);
        game.turn.phase = Phase::Combat;
        game.turn.step = Some(ironsmith::game_state::Step::BeginCombat);
        game.queue_trigger_event(
            Default::default(),
            ironsmith::triggers::TriggerEvent::new(
                ironsmith::events::BeginningOfCombatEvent::new(A),
                Default::default(),
            ),
        );
        settle(&mut game, &mut MoveCounterChoice(recipient));
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
        assert_eq!(counters(&game, source), 1);
        assert_eq!(counters(&game, recipient), 2);
    }
}

struct SurveilOne;
impl DecisionMaker for SurveilOne {
    fn decide_partition(
        &mut self,
        _: &GameState,
        ctx: &ironsmith::decisions::context::PartitionContext,
    ) -> Vec<ObjectId> {
        assert_eq!(ctx.cards.len(), 2);
        vec![ctx.cards[0].0]
    }
}
#[test]
fn textbook_entry_really_surveys_two_library_cards() {
    for definition in definitions("Textbook Tabulator") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Hand);
        for _ in 0..2 {
            card(&mut game, A, Zone::Library, "Type: Artifact");
        }
        execute(
            &mut game,
            source,
            A,
            &ironsmith::effects::MoveToZoneEffect::new(
                ChooseSpec::Source,
                Zone::Battlefield,
                false,
            ),
        );
        settle(&mut game, &mut SurveilOne);
        assert_eq!(game.player(A).unwrap().graveyard.len(), 1);
        assert_eq!(game.player(A).unwrap().library.len(), 1);
    }
}

#[test]
fn berta_pays_x_and_creates_a_fractal_with_that_many_counters() {
    for definition in definitions("Berta, Wise Extrapolator") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        game.remove_summoning_sickness(source);
        game.player_mut(A)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Colorless, 2);
        let action = ironsmith::decision::compute_legal_actions(&game, A)
            .unwrap()
            .into_iter()
            .find(
                |action| matches!(action,LegalAction::ActivateAbility{source:id,..} if *id==source),
            )
            .unwrap();
        let mut state = PriorityLoopState::new(2);
        let mut queue = TriggerQueue::new();
        let mut dm = MoveCounterChoice(source);
        let mut progress = apply_priority_response_with_dm(
            &mut game,
            &mut queue,
            &mut state,
            &PriorityResponse::PriorityAction(action),
            &mut dm,
        )
        .unwrap();
        for _ in 0..30 {
            if state.pending_activation.is_none() {
                break;
            }
            let ironsmith::GameProgress::NeedsDecisionCtx(context) = progress else {
                panic!("X activation pending")
            };
            progress = apply_decision_context_with_dm(
                &mut game, &mut queue, &mut state, &context, &mut dm,
            )
            .unwrap();
        }
        assert!(state.pending_activation.is_none());
        put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm).unwrap();
        settle(&mut game, &mut dm);
        let token = *game
            .battlefield
            .iter()
            .find(|id| game.object(**id).unwrap().kind == ironsmith::object::ObjectKind::Token)
            .unwrap();
        assert_eq!(counters(&game, token), 2);
        assert_eq!(game.current_power(token), Some(2));
        assert!(game.is_tapped(source));
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
    }
}

#[test]
fn granted_increment_expands_to_the_same_executable_cast_trigger() {
    for definition in definitions_from(
        "Increment grant",
        "Type: Enchantment\nCreatures you control have increment.",
    ) {
        let mut game = game();
        game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let mine = card(
            &mut game,
            A,
            Zone::Battlefield,
            "Type: Creature\nPower/Toughness: 1/1",
        );
        let theirs = card(
            &mut game,
            B,
            Zone::Battlefield,
            "Type: Creature\nPower/Toughness: 1/1",
        );
        game.player_mut(A)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Colorless, 2);
        announce(&mut game, &spell(2), A, &mut SelectFirstDecisionMaker);
        settle(&mut game, &mut SelectFirstDecisionMaker);
        assert_eq!(counters(&game, mine), 1);
        assert_eq!(counters(&game, theirs), 0);
    }
}
