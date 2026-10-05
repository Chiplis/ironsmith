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
        "../../../fixtures/single_source_damage_set.json.fixture"
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
        for player in [A, B, C] {
            game.player_mut(player).unwrap().mana_pool.add(color, 20);
        }
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
    kicker_payments: usize,
}
impl DecisionMaker for Choices {
    fn decide_boolean(
        &mut self,
        _: &GameState,
        _: &ironsmith::decisions::context::BooleanContext,
    ) -> bool {
        true
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
        if context.description.starts_with("Choose optional costs for") {
            let legal = context
                .options
                .iter()
                .filter(|o| o.legal)
                .collect::<Vec<_>>();
            if legal.len() == 1 {
                return vec![legal[0].index; self.kicker_payments];
            }
            return legal
                .iter()
                .take(self.kicker_payments)
                .map(|o| o.index)
                .collect();
        }
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
            for target in &self.targets {
                assert!(
                    context
                        .requirements
                        .iter()
                        .any(|r| r.legal_targets.contains(target))
                );
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
fn queue_event(game: &mut GameState, event: TriggerEvent, dm: &mut Choices) -> usize {
    let mut queue = TriggerQueue::new();
    for entry in check_triggers(game, &event) {
        queue.add(entry);
    }
    let count = queue.entries.len();
    put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap();
    count
}
fn queue_outcome(game: &mut GameState, outcome: EffectOutcome, dm: &mut Choices) {
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
fn resolve(game: &mut GameState, dm: &mut Choices) {
    resolve_stack_entry_with(game, dm).unwrap();
}
fn resolve_all(game: &mut GameState, dm: &mut Choices) {
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
    dm: &mut Choices,
) -> ObjectId {
    cast_for(game, definition, A, method, dm)
}
fn cast_for(
    game: &mut GameState,
    definition: &CardDefinition,
    caster: PlayerId,
    method: CastingMethod,
    dm: &mut Choices,
) -> ObjectId {
    let id = game.create_object_from_definition(definition, caster, Zone::Hand);
    let action = LegalAction::CastSpell {
        spell_id: id,
        from_zone: Zone::Hand,
        casting_method: method,
    };
    assert!(
        compute_legal_actions(game, caster)
            .unwrap()
            .contains(&action)
    );
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
fn activate(game: &mut GameState, source: ObjectId, ability_index: usize, dm: &mut Choices) {
    let action = LegalAction::ActivateAbility {
        source,
        ability_index,
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
    for _ in 0..50 {
        if state.pending_activation.is_none() {
            break;
        }
        let GameProgress::NeedsDecisionCtx(ctx) = progress else {
            panic!("{progress:?}");
        };
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &ctx, dm).unwrap();
    }
    assert!(state.pending_activation.is_none());
    assert_eq!(game.stack.len(), 1);
}
fn activated(definition: &CardDefinition) -> usize {
    definition
        .abilities
        .iter()
        .position(|ability| matches!(ability.kind, ironsmith::ability::AbilityKind::Activated(_)))
        .unwrap()
}

fn body(name: &str, text: &str) -> CardDefinition {
    compile_to_runtime_definition(name, text, false).unwrap()
}
fn library(game: &mut GameState, player: PlayerId, count: usize) {
    for _ in 0..count {
        game.create_object_from_definition(
            &vanilla("Library card", "{1}", "Human", 1, 1),
            player,
            Zone::Library,
        );
    }
}
fn witness(game: &mut GameState, owner: PlayerId) -> ObjectId {
    game.create_object_from_definition(
        &vanilla("Damage witness", "{1}", "Human", 1, 100),
        owner,
        Zone::Battlefield,
    )
}
fn damage_sets(def: &CardDefinition) -> Vec<ironsmith_core::DealDamageToRecipientsEffect> {
    fn walk(effect: &Effect, found: &mut Vec<ironsmith_core::DealDamageToRecipientsEffect>) {
        if let Some(set) = effect.downcast_ref::<ironsmith::effects::DealDamageToRecipientsEffect>()
        {
            found.push(set.clone());
        }
        effect.visit_child_effects(&mut |child| walk(child, found));
    }
    let mut found = Vec::new();
    for segment in &def.spell_effect.as_ref().unwrap().segments {
        for effect in &segment.default_effects {
            walk(effect, &mut found);
        }
    }
    found
}
#[test]
fn both_full_cards_compile_one_shared_damage_packet_after_their_actual_producer() {
    for row in fixtures() {
        for definition in definitions(row["name"].as_str().unwrap()) {
            let sets = damage_sets(&definition);
            assert_eq!(sets.len(), 1);
            assert_eq!(sets[0].recipients.len(), 2);
            assert!(!sets[0].recipients.iter().any(|spec| spec.is_target()));
            let ironsmith_core::Value::PriorEffectMetric { query, .. } = sets[0].amount.unhinted()
            else {
                panic!("{:?}", sets[0].amount);
            };
            assert_eq!(
                query.action,
                Some(if definition.card.name == "Friendly Fire" {
                    ironsmith_core::PriorEffectAction::Revealed
                } else {
                    ironsmith_core::PriorEffectAction::PutIntoGraveyard
                })
            );
        }
    }
}
#[test]
fn friendly_fire_retains_the_earlier_creature_and_controller_and_the_random_revealed_card() {
    for definition in definitions("Friendly Fire") {
        for cost in ["", "{5}"] {
            let mut game = game();
            let victim = witness(&mut game, B);
            let unrelated = witness(&mut game, C);
            let revealed = game.create_object_from_definition(
                &body(
                    "Known random card",
                    &if cost.is_empty() {
                        "Type: Artifact".to_string()
                    } else {
                        format!("Mana cost: {cost}\nType: Artifact")
                    },
                ),
                B,
                Zone::Hand,
            );
            // One card makes the random action deterministic without replacing it.
            let mut dm = Choices {
                targets: vec![Target::Object(victim)],
                ..Default::default()
            };
            cast(&mut game, &definition, CastingMethod::Normal, &mut dm);
            resolve(&mut game, &mut dm);
            let amount = if cost.is_empty() { 0 } else { 5 };
            assert_eq!(game.damage_on(victim), amount);
            assert_eq!(game.player(B).unwrap().life, 20 - amount as i32);
            assert_eq!(game.damage_on(unrelated), 0);
            assert_eq!(game.player(A).unwrap().life, 20);
            assert!(game.player(B).unwrap().hand.contains(&revealed));
            assert!(
                dm.viewed.iter().any(|cards| cards.contains(&revealed)),
                "the real reveal is public"
            );
        }
    }
}
#[test]
fn volcanic_eruption_counts_only_destroyed_mountains_that_reached_the_graveyard() {
    for definition in definitions("Volcanic Eruption") {
        let mut game = game();
        let victim = witness(&mut game, B);
        let mountain = body("Mountain witness", "Type: Basic Land — Mountain");
        let destroyed = game.create_object_from_definition(&mountain, A, Zone::Battlefield);
        let protected = game.create_object_from_definition(
            &body(
                "Protected Mountain",
                "Type: Land — Mountain\nIndestructible",
            ),
            B,
            Zone::Battlefield,
        );
        let exiled = game.create_object_from_definition(&mountain, C, Zone::Battlefield);
        // Restrict the typed replacement to one exact object, leaving the other
        // Mountain destruction to exercise its normal graveyard receipt.
        let helper = witness(&mut game, A);
        apply(
            &mut game,
            helper,
            Effect::new(ironsmith::effects::RegisterZoneReplacementEffect::new(
                ChooseSpec::SpecificObject(exiled),
                Some(Zone::Battlefield),
                Some(Zone::Graveyard),
                Zone::Exile,
                ironsmith::effects::ReplacementApplyMode::UntilEndOfTurn,
            )),
        );
        let mut dm = Choices {
            x: 3,
            targets: vec![
                Target::Object(destroyed),
                Target::Object(protected),
                Target::Object(exiled),
            ],
            ..Default::default()
        };
        cast(&mut game, &definition, CastingMethod::Normal, &mut dm);
        resolve(&mut game, &mut dm);
        assert_eq!(
            game.player(A)
                .unwrap()
                .graveyard
                .iter()
                .filter(|id| game.object(**id).unwrap().name == "Mountain witness")
                .count(),
            1
        );
        assert!(game.object(protected).is_some());
        assert_eq!(
            game.players
                .iter()
                .map(|player| player.life)
                .collect::<Vec<_>>(),
            vec![19, 19, 19]
        );
        assert_eq!(game.damage_on(victim), 1);
        assert_eq!(game.damage_on(helper), 1);
        assert!(game.object(exiled).is_none());
    }
}
#[test]
fn one_source_samples_once_deduplicates_recipients_and_emits_one_lifelink_gain() {
    let mut game = game();
    let source = game.create_object_from_definition(
        &body(
            "Life-size lifelinker",
            "Type: Creature\nPower/Toughness: 1/100\nLifelink",
        ),
        A,
        Zone::Battlefield,
    );
    apply(
        &mut game,
        source,
        Effect::lose_life_player(15, PlayerFilter::You),
    );
    let first = witness(&mut game, B);
    let second = witness(&mut game, C);
    // If sampled separately, lifelink on the first recipient would increase
    // the amount dealt to the second. All three recipients instead take 5.
    let effect = Effect::new(ironsmith::effects::DealDamageToRecipientsEffect {
        amount: ironsmith_core::Value::LifeTotal(PlayerFilter::You),
        recipients: vec![
            ChooseSpec::SpecificObject(first),
            ChooseSpec::Player(PlayerFilter::Specific(B)),
            ChooseSpec::SpecificObject(second),
            ChooseSpec::SpecificObject(first),
        ],
    });
    let outcome = apply(&mut game, source, effect);
    assert_eq!((game.damage_on(first), game.damage_on(second)), (5, 5));
    assert_eq!(game.player(A).unwrap().life, 20);
    let gains = outcome
        .events
        .iter()
        .filter_map(|event| event.downcast::<ironsmith::events::LifeGainEvent>())
        .collect::<Vec<_>>();
    assert_eq!(gains.len(), 1);
    assert_eq!(gains[0].amount, 15);
}
#[test]
fn zero_and_empty_recipient_sets_do_not_create_damage_or_lifelink_events() {
    for (amount, empty) in [(0, false), (4, true)] {
        let mut game = game();
        let source = witness(&mut game, A);
        let victim = witness(&mut game, B);
        let outcome = apply(
            &mut game,
            source,
            Effect::new(ironsmith::effects::DealDamageToRecipientsEffect {
                amount: amount.into(),
                recipients: if empty {
                    vec![]
                } else {
                    vec![
                        ChooseSpec::SpecificObject(victim),
                        ChooseSpec::Player(PlayerFilter::Specific(B)),
                    ]
                },
            }),
        );
        assert_eq!(game.damage_on(victim), 0);
        assert_eq!(game.player(B).unwrap().life, 20);
        assert!(
            !outcome
                .events
                .iter()
                .any(|event| event.downcast::<ironsmith::events::DamageEvent>().is_some())
        );
    }
}

#[test]
fn mixed_packet_retains_independent_prevention_shields_and_real_prevented_amounts() {
    let mut game = game();
    let source = witness(&mut game, A);
    let victim = witness(&mut game, B);
    apply(
        &mut game,
        victim,
        Effect::prevent_damage(1, ChooseSpec::SpecificObject(victim), Until::EndOfTurn),
    );
    apply(
        &mut game,
        victim,
        Effect::prevent_damage(
            2,
            ChooseSpec::Player(PlayerFilter::Specific(B)),
            Until::EndOfTurn,
        ),
    );
    let outcome = apply(
        &mut game,
        source,
        Effect::new(ironsmith::effects::DealDamageToRecipientsEffect {
            amount: 3.into(),
            recipients: vec![
                ChooseSpec::SpecificObject(victim),
                ChooseSpec::Player(PlayerFilter::Specific(B)),
            ],
        }),
    );
    assert_eq!(game.damage_on(victim), 2);
    assert_eq!(game.player(B).unwrap().life, 19);
    assert_eq!(outcome.count_or_zero(), 3);
}
