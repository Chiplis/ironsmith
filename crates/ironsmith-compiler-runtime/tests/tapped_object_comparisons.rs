//! UNVALIDATED exact-source quantity and live-versus-LKI reference regressions.
use ironsmith::alternative_cast::CastingMethod;
use ironsmith::cards::CardDefinition;
use ironsmith::decision::{
    DecisionMaker, LegalAction, SelectFirstDecisionMaker, compute_legal_actions,
};
use ironsmith::decisions::context::{
    NumberContext, SelectObjectsContext, SelectOptionsContext, TargetsContext, ViewCardsContext,
};
use ironsmith::effect::{Effect, EffectOutcome, Until};
use ironsmith::effects::{EffectContext, execute_effect};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, put_triggers_on_stack_with_dm, resolve_stack_entry_with,
};
use ironsmith::game_state::Phase;
use ironsmith::mana::ManaSymbol;
use ironsmith::target::{ChooseSpec, PlayerFilter};
use ironsmith::triggers::{TriggerEvent, TriggerQueue, check_triggers};
use ironsmith::{GameProgress, GameState, ObjectId, PlayerId, Target, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler::parse_loss;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
const A: PlayerId = PlayerId::from_index(0);
const B: PlayerId = PlayerId::from_index(1);
const C: PlayerId = PlayerId::from_index(2);

fn fixtures() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!(
        "../../../fixtures/tapped_object_comparisons.json.fixture"
    ))
    .unwrap()
}
fn definitions(name: &str) -> [CardDefinition; 2] {
    let row = fixtures().into_iter().find(|r| r["name"] == name).unwrap();
    let mut lines = vec![
        format!("Mana cost: {}", row["mana_cost"].as_str().unwrap()),
        format!("Type: {}", row["type_line"].as_str().unwrap()),
    ];
    if let (Some(p), Some(t)) = (row["power"].as_str(), row["toughness"].as_str()) {
        lines.push(format!("Power/Toughness: {p}/{t}"));
    }
    lines.push(row["oracle_text"].as_str().unwrap().into());
    definitions_text(name, &lines.join("\n"))
}
fn definitions_text(name: &str, text: &str) -> [CardDefinition; 2] {
    let (result, loss) = parse_loss::capture(|| compile_to_artifact(name, text, false));
    let (artifact, direct) = result.unwrap_or_else(|e| panic!("{name}: {e}"));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let decoded = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, decoded);
    [
        direct,
        ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&decoded).unwrap(),
    ]
}
fn game() -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Charlie".into()], 20);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = A;
    game.turn.priority_player = Some(A);
    for color in [
        ManaSymbol::White,
        ManaSymbol::Blue,
        ManaSymbol::Black,
        ManaSymbol::Red,
        ManaSymbol::Green,
        ManaSymbol::Colorless,
    ] {
        game.player_mut(A).unwrap().mana_pool.add(color, 20);
    }
    game
}
fn vanilla(name: &str, cost: &str, subtype: &str, p: i32, t: i32) -> CardDefinition {
    compile_to_runtime_definition(
        name,
        format!("Mana cost: {cost}\nType: Creature — {subtype}\nPower/Toughness: {p}/{t}"),
        false,
    )
    .unwrap()
}
#[derive(Default)]
struct Choices {
    targets: Vec<Target>,
    objects: Vec<ObjectId>,
    mode: Option<usize>,
    x: u32,
    prefer_life: bool,
    viewed: Vec<Vec<ObjectId>>,
    untap: bool,
}
impl DecisionMaker for Choices {
    fn decide_boolean(
        &mut self,
        _: &GameState,
        ctx: &ironsmith::decisions::context::BooleanContext,
    ) -> bool {
        if ctx.description.starts_with("untap ") {
            self.untap
        } else {
            true
        }
    }
    fn view_cards(
        &mut self,
        _game: &GameState,
        viewer: PlayerId,
        cards: &[ObjectId],
        _context: &ViewCardsContext,
    ) {
        if viewer == A {
            self.viewed.push(cards.to_vec());
        }
    }

    fn decide_options(&mut self, game: &GameState, context: &SelectOptionsContext) -> Vec<usize> {
        if let Some(mode) = self.mode
            && context.description.starts_with("Choose mode for")
        {
            assert!(
                context
                    .options
                    .iter()
                    .any(|option| option.index == mode && option.legal)
            );
            return vec![mode];
        }
        if self.prefer_life && context.description.starts_with("Choose how to pay pip") {
            if let Some(option) = context.options.iter().find(|option| {
                option.legal && option.description.to_ascii_lowercase().contains("life")
            }) {
                return vec![option.index];
            }
        }
        SelectFirstDecisionMaker.decide_options(game, context)
    }
    fn decide_number(&mut self, game: &GameState, context: &NumberContext) -> u32 {
        if context.is_x_value {
            assert!(self.x <= context.max);
            self.x
        } else {
            SelectFirstDecisionMaker.decide_number(game, context)
        }
    }
    fn decide_targets(&mut self, game: &GameState, context: &TargetsContext) -> Vec<Target> {
        if !self.targets.is_empty() {
            assert_eq!(context.requirements.len(), self.targets.len());
            for (requirement, target) in context.requirements.iter().zip(&self.targets) {
                assert!(requirement.legal_targets.contains(target));
            }
            self.targets.clone()
        } else {
            SelectFirstDecisionMaker.decide_targets(game, context)
        }
    }
    fn decide_objects(
        &mut self,
        game: &GameState,
        context: &SelectObjectsContext,
    ) -> Vec<ObjectId> {
        if !self.objects.is_empty() {
            for id in &self.objects {
                assert!(
                    context
                        .candidates
                        .iter()
                        .any(|candidate| candidate.id == *id && candidate.legal)
                );
            }
            self.objects.clone()
        } else {
            SelectFirstDecisionMaker.decide_objects(game, context)
        }
    }
}
fn queue_event(game: &mut GameState, event: TriggerEvent, dm: &mut impl DecisionMaker) -> usize {
    let mut queue = TriggerQueue::new();
    for entry in check_triggers(game, &event) {
        queue.add(entry);
    }
    let count = queue.entries.len();
    put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap();
    count
}
fn queue_outcome(game: &mut GameState, outcome: EffectOutcome, dm: &mut impl DecisionMaker) {
    let mut queue = TriggerQueue::new();
    // Checked execution already captures some triggers in the original observer frame.
    ironsmith::game_loop::drain_pending_trigger_events(game, &mut queue);
    for event in outcome.events {
        for entry in check_triggers(game, &event) {
            queue.add(entry);
        }
    }
    put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap();
}
fn apply(game: &mut GameState, source: ObjectId, effect: Effect) -> EffectOutcome {
    let mut dm = SelectFirstDecisionMaker;
    let controller = game.current_controller(source).unwrap_or(A);
    execute_effect(
        game,
        &effect,
        &mut EffectContext::new(source, controller, &mut dm),
    )
    .unwrap()
}
fn resolve(game: &mut GameState, dm: &mut impl DecisionMaker) {
    resolve_stack_entry_with(game, dm).unwrap();
}
fn resolve_all(game: &mut GameState, dm: &mut impl DecisionMaker) {
    for _ in 0..30 {
        if game.stack_is_empty() {
            return;
        }
        resolve(game, dm);
    }
    panic!("unexpected continuing trigger chain");
}
fn cast(
    game: &mut GameState,
    definition: &CardDefinition,
    method: CastingMethod,
    dm: &mut impl DecisionMaker,
) -> ObjectId {
    let id = game.create_object_from_definition(definition, A, Zone::Hand);
    let action = LegalAction::CastSpell {
        spell_id: id,
        from_zone: Zone::Hand,
        casting_method: method,
    };
    assert!(compute_legal_actions(game, A).unwrap().contains(&action));
    let mut queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(3);
    let mut progress = apply_priority_response_with_dm(
        game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        dm,
    )
    .unwrap();
    for _ in 0..60 {
        if state.pending_cast.is_none() && state.pending_method_selection.is_none() {
            break;
        }
        let GameProgress::NeedsDecisionCtx(ctx) = progress else {
            panic!("{progress:?}");
        };
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &ctx, dm).unwrap();
    }
    assert!(state.pending_cast.is_none() && state.pending_method_selection.is_none());
    let spell = game
        .stack
        .iter()
        .rev()
        .find(|entry| !entry.is_ability)
        .unwrap()
        .object_id;
    put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap();
    spell
}
#[derive(Default)]
struct TapChoices {
    taps: std::collections::VecDeque<Option<ObjectId>>,
    desired_targets: Vec<ObjectId>,
    target_offers: Vec<Vec<Target>>,
}
impl DecisionMaker for TapChoices {
    fn decide_objects(&mut self, _: &GameState, ctx: &SelectObjectsContext) -> Vec<ObjectId> {
        let selected = self
            .taps
            .pop_front()
            .expect("one tap selection per opponent");
        selected
            .map(|id| {
                assert!(
                    ctx.candidates
                        .iter()
                        .any(|candidate| candidate.id == id && candidate.legal)
                );
                vec![id]
            })
            .unwrap_or_default()
    }
    fn decide_targets(&mut self, _: &GameState, ctx: &TargetsContext) -> Vec<Target> {
        assert_eq!(ctx.requirements.len(), 1);
        let legal = &ctx.requirements[0].legal_targets;
        self.target_offers.push(legal.clone());
        vec![
            self.desired_targets
                .iter()
                .copied()
                .map(Target::Object)
                .find(|target| legal.contains(target))
                .expect("legal desired target from this iteration"),
        ]
    }
}
fn creature(game: &mut GameState, owner: PlayerId, p: i32) -> ObjectId {
    game.create_object_from_definition(
        &vanilla("Tap comparison subject", "{1}", "Human", p, 20),
        owner,
        Zone::Battlefield,
    )
}
fn pump(game: &mut GameState, id: ObjectId, p: i32) {
    apply(
        game,
        id,
        Effect::pump(p, 0, ChooseSpec::SpecificObject(id), Until::EndOfTurn),
    );
}
fn enter(game: &mut GameState, definition: &CardDefinition) -> ObjectId {
    let in_hand = game.create_object_from_definition(definition, A, Zone::Hand);
    let stable = game.object(in_hand).unwrap().stable_id;
    let outcome = apply(
        game,
        in_hand,
        Effect::put_onto_battlefield(
            ChooseSpec::SpecificObject(in_hand),
            false,
            PlayerFilter::You,
        ),
    );
    let source = game.find_object_by_stable_id(stable).unwrap();
    queue_outcome(game, outcome, &mut Choices::default());
    assert_eq!(
        game.stack.len(),
        1,
        "only the enters ability is initially on the stack"
    );
    source
}
fn stack_pending(game: &mut GameState, dm: &mut impl DecisionMaker) {
    put_triggers_on_stack_with_dm(game, &mut TriggerQueue::new(), dm).unwrap();
}
#[test]
fn nihiloor_retains_singular_owned_attack_and_local_reflexive_comparison_artifacts() {
    for definition in definitions("Nihiloor") {
        let mut attacks = 0;
        let mut reflexive = 0;
        fn walk(effect: &Effect, reflexive: &mut usize) {
            if let Some(effect) =
                effect.downcast_ref::<ironsmith::effects::ReflexiveTriggerEffect>()
            {
                *reflexive += 1;
                let [choice] = effect.choices.as_slice() else {
                    panic!("{effect:?}")
                };
                let ChooseSpec::Object(filter) = choice.base() else {
                    panic!("{choice:?}")
                };
                assert!(
                    filter
                        .controller
                        .as_ref()
                        .unwrap()
                        .mentions_iterated_player()
                );
                let Some(ironsmith::filter::Comparison::LessThanOrEqualExpr(rhs)) = &filter.power
                else {
                    panic!("{filter:?}")
                };
                let ironsmith::effect::Value::PowerOf(spec) = rhs.unhinted() else {
                    panic!("{rhs:?}")
                };
                assert!(
                    matches!(spec.base(),ChooseSpec::Tagged(tag) if tag.as_str().starts_with("tapped_"))
                );
            }
            effect.visit_child_effects(&mut |nested| walk(nested, reflexive));
        }
        for ability in &definition.abilities {
            if let ironsmith::ability::AbilityKind::Triggered(ability) = &ability.kind {
                if let Some(model) = ability.trigger.compiled_model() {
                    if let ironsmith_core::trigger_model::TriggerKind::Attacks { filter } =
                        &model.kind
                    {
                        attacks += 1;
                        assert_eq!(filter.controller, Some(PlayerFilter::You));
                        assert_eq!(filter.owner, Some(PlayerFilter::Opponent));
                    }
                    assert!(!matches!(
                        model.kind,
                        ironsmith_core::trigger_model::TriggerKind::AttacksOneOrMore { .. }
                    ));
                }
                for segment in &ability.effects.segments {
                    for effect in &segment.default_effects {
                        walk(effect, &mut reflexive);
                    }
                }
            }
        }
        assert_eq!(attacks, 1);
        assert_eq!(reflexive, 1);
    }
}
#[test]
fn each_opponent_gets_its_own_tapped_reference_and_live_or_departure_bound() {
    for definition in definitions("Nihiloor") {
        let mut game = game();
        let weak = creature(&mut game, A, 2);
        let strong = creature(&mut game, A, 5);
        let b_target = creature(&mut game, B, 2);
        let b_large = creature(&mut game, B, 3);
        let c_target = creature(&mut game, C, 4);
        let source = enter(&mut game, &definition);
        let mut dm = TapChoices {
            taps: [Some(weak), Some(strong)].into(),
            desired_targets: vec![b_target, c_target],
            ..Default::default()
        };
        resolve(&mut game, &mut dm);
        assert!(dm.taps.is_empty());
        assert!(game.is_tapped(weak));
        assert!(game.is_tapped(strong));
        assert_eq!(game.current_controller(b_target), Some(B));
        assert_eq!(game.current_controller(c_target), Some(C));
        stack_pending(&mut game, &mut dm);
        assert_eq!(game.stack.len(), 2);
        let b_offer = dm
            .target_offers
            .iter()
            .find(|offer| offer.contains(&Target::Object(b_target)))
            .unwrap();
        assert!(!b_offer.contains(&Target::Object(b_large)));
        assert!(!b_offer.contains(&Target::Object(c_target)));
        pump(&mut game, weak, -1); // B's target is now illegal on resolution.
        pump(&mut game, strong, 3);
        let exile = game.move_object_by_game_rule(strong, Zone::Exile).unwrap();
        let returned = game
            .move_object_by_game_rule(exile, Zone::Battlefield)
            .unwrap();
        pump(&mut game, returned, -5);
        resolve_all(&mut game, &mut Choices::default());
        assert_eq!(game.current_controller(b_target), Some(B));
        assert_eq!(
            game.current_controller(c_target),
            Some(A),
            "C uses original tapped object's departure power eight, not blink power zero or B's one"
        );
        game.set_current_controller(source, B).unwrap();
        assert_eq!(game.current_controller(c_target), Some(C));
        game.set_current_controller(source, A).unwrap();
        assert_eq!(
            game.current_controller(c_target),
            Some(C),
            "expired control does not revive"
        );
    }
}
#[test]
fn declined_tap_makes_no_reflexive_and_source_departure_prevents_control() {
    for definition in definitions("Nihiloor") {
        for departed in [false, true] {
            let mut game = game();
            let tapped = creature(&mut game, A, 4);
            let b_target = creature(&mut game, B, 2);
            let c_target = creature(&mut game, C, 2);
            let source = enter(&mut game, &definition);
            let mut dm = TapChoices {
                taps: [Some(tapped), None].into(),
                desired_targets: vec![b_target, c_target],
                ..Default::default()
            };
            resolve(&mut game, &mut dm);
            stack_pending(&mut game, &mut dm);
            assert_eq!(game.stack.len(), 1);
            if departed {
                game.move_object_by_game_rule(source, Zone::Exile).unwrap();
            }
            resolve_all(&mut game, &mut Choices::default());
            assert_eq!(
                game.current_controller(b_target),
                Some(if departed { B } else { A })
            );
            assert_eq!(game.current_controller(c_target), Some(C));
        }
    }
}
#[test]
fn two_opponent_owned_attackers_trigger_twice_and_charge_the_owner_not_the_defender() {
    for definition in definitions("Nihiloor") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let first = creature(&mut game, B, 2);
        let second = creature(&mut game, B, 2);
        for attacker in [first, second] {
            game.set_current_controller(attacker, A).unwrap();
            game.remove_summoning_sickness(attacker);
        }
        game.turn.phase = Phase::Combat;
        game.turn.step = Some(ironsmith::game_state::Step::DeclareAttackers);
        let mut combat = ironsmith::combat_state::CombatState::default();
        let mut queue = TriggerQueue::new();
        let attacks = [first, second].map(|creature| ironsmith::decision::AttackerDeclaration {
            creature,
            target: ironsmith::combat_state::AttackTarget::Player(C),
        });
        ironsmith::game_loop::apply_attacker_declarations(
            &mut game,
            &mut combat,
            &mut queue,
            &attacks,
        )
        .unwrap();
        game.combat = Some(combat);
        put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut Choices::default()).unwrap();
        assert_eq!(game.stack.len(), 2);
        game.set_current_controller(source, B).unwrap();
        resolve_all(&mut game, &mut Choices::default());
        assert_eq!(game.player(A).unwrap().life, 24);
        assert_eq!(game.player(B).unwrap().life, 16);
        assert_eq!(game.player(C).unwrap().life, 20);
    }
}
