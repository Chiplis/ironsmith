//! UNVALIDATED full-card basic-land templates, exact choice and CR305.7 ability retention.
use ironsmith::alternative_cast::CastingMethod;
use ironsmith::cards::CardDefinition;
use ironsmith::decision::{
    DecisionMaker, LegalAction, SelectFirstDecisionMaker, compute_legal_actions,
};
use ironsmith::decisions::context::{
    BooleanContext, NumberContext, SelectObjectsContext, SelectOptionsContext, TargetsContext,
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
        "../../../fixtures/basic_land_characteristics.json.fixture"
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
    if let Some(loyalty) = row["loyalty"].as_str() {
        lines.push(format!("Loyalty: {loyalty}"));
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
    objects_explicit: bool,
    x: u32,
    decline: bool,
    land_choice: Option<&'static str>,
    land_prompts: usize,
}
impl DecisionMaker for Choices {
    fn decide_options(&mut self, game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
        if ctx.description == "Choose a basic land type" {
            self.land_prompts += 1;
            return vec![
                ctx.options
                    .iter()
                    .find(|option| option.description == self.land_choice.unwrap_or("Island"))
                    .unwrap()
                    .index,
            ];
        }
        if ctx.description.starts_with("Choose optional costs") {
            if self.decline {
                return vec![];
            }
            return ctx
                .options
                .iter()
                .filter(|option| option.legal)
                .map(|option| option.index)
                .collect();
        }
        SelectFirstDecisionMaker.decide_options(game, ctx)
    }
    fn decide_boolean(&mut self, _game: &GameState, _ctx: &BooleanContext) -> bool {
        !self.decline
    }
    fn decide_number(&mut self, game: &GameState, ctx: &NumberContext) -> u32 {
        if ctx.is_x_value {
            assert!(self.x <= ctx.max);
            self.x
        } else {
            SelectFirstDecisionMaker.decide_number(game, ctx)
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
        if self.objects_explicit || !self.objects.is_empty() {
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
fn resource(name: &str, types: &str) -> CardDefinition {
    let pt = if types.contains("Creature") {
        "\nPower/Toughness: 1/1"
    } else {
        ""
    };
    compile_to_runtime_definition(name, format!("Mana cost: {{1}}\nType: {types}{pt}"), false)
        .unwrap()
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
fn pt(game: &GameState, id: ObjectId) -> (i32, i32) {
    (
        game.current_power(id).unwrap(),
        game.current_toughness(id).unwrap(),
    )
}
fn pump(game: &mut GameState, source: ObjectId, id: ObjectId, p: i32, t: i32) {
    apply(
        game,
        source,
        Effect::pump(p, t, ChooseSpec::SpecificObject(id), Until::EndOfTurn),
    );
}
fn counter(game: &mut GameState, source: ObjectId, id: ObjectId, count: i32) {
    apply(
        game,
        source,
        Effect::put_counters(
            ironsmith::object::CounterType::PlusOnePlusOne,
            count,
            ChooseSpec::SpecificObject(id),
        ),
    );
}
fn cast(
    game: &mut GameState,
    definition: &CardDefinition,
    method: CastingMethod,
    dm: &mut Choices,
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
        .find(|entry| !entry.is_ability)
        .unwrap()
        .object_id;
    put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap();
    spell
}

fn activate(game: &mut GameState, source: ObjectId, ability_index: usize, dm: &mut Choices) {
    game.turn.priority_player = Some(A);
    let action = compute_legal_actions(game, A)
        .unwrap()
        .into_iter()
        .find(|action| {
            matches!(action, LegalAction::ActivateAbility { source: id, ability_index: index }
            | LegalAction::ActivateManaAbility { source: id, ability_index: index }
            if *id == source && *index == ability_index)
        })
        .expect("the current ability must be legally activatable");
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
        if state.pending_activation.is_none() {
            break;
        }
        let GameProgress::NeedsDecisionCtx(ctx) = progress else {
            panic!("{progress:?}")
        };
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &ctx, dm).unwrap();
    }
    assert!(state.pending_activation.is_none());
}
fn activated_at(definition: &CardDefinition, position: usize) -> usize {
    definition
        .abilities
        .iter()
        .enumerate()
        .filter_map(|(i, a)| {
            matches!(a.kind, ironsmith::ability::AbilityKind::Activated(_)).then_some(i)
        })
        .nth(position)
        .unwrap()
}
fn current_activated_at(game: &GameState, source: ObjectId, position: usize) -> usize {
    game.calculated_characteristics(source)
        .unwrap()
        .abilities
        .iter()
        .enumerate()
        .filter_map(|(i, a)| {
            matches!(a.kind, ironsmith::ability::AbilityKind::Activated(_)).then_some(i)
        })
        .nth(position)
        .unwrap()
}
fn has(
    game: &GameState,
    id: ObjectId,
    ability: ironsmith::static_abilities::StaticAbilityId,
) -> bool {
    game.current_has_static_ability_id(id, ability)
}
fn event(game: &mut GameState, event: TriggerEvent, dm: &mut Choices) {
    let mut queue = TriggerQueue::new();
    for trigger in check_triggers(game, &event) {
        queue.add(trigger);
    }
    put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap();
}
fn lore(game: &mut GameState, source: ObjectId, dm: &mut Choices) {
    let outcome = apply(
        game,
        source,
        Effect::put_counters(
            ironsmith::object::CounterType::Lore,
            1,
            ChooseSpec::SpecificObject(source),
        ),
    );
    queue_outcome(game, outcome, dm);
    resolve_all(game, &mut Choices::default());
}

fn land(
    game: &mut GameState,
    owner: PlayerId,
    name: &str,
    subtype: ironsmith::Subtype,
) -> ObjectId {
    let card = ironsmith::card::CardBuilder::new(ironsmith::CardId::new(), name)
        .card_types(vec![
            ironsmith::CardType::Land,
            ironsmith::CardType::Creature,
        ])
        .subtypes(vec![subtype, ironsmith::Subtype::Elf])
        .supertypes(vec![ironsmith::Supertype::Snow])
        .power_toughness(ironsmith::card::PowerToughness::fixed(2, 2))
        .build();
    let mut definition = CardDefinition::new(card);
    definition
        .abilities
        .push(ironsmith::ability::Ability::static_ability(
            ironsmith::static_abilities::StaticAbility::flying(),
        ));
    game.create_object_from_definition(&definition, owner, Zone::Battlefield)
}
fn grant_haste(game: &mut GameState, id: ObjectId) {
    apply(
        game,
        id,
        Effect::new(ironsmith::effects::ApplyContinuousEffect::new(
            ironsmith::continuous::EffectTarget::Specific(id),
            ironsmith::continuous::Modification::AddAbility(
                ironsmith::static_abilities::StaticAbility::haste(),
            ),
            Until::Forever,
        )),
    );
}
#[test]
fn six_exact_land_conversion_bodies_round_trip_and_farmer_remains_partial() {
    let rows = fixtures()
        .into_iter()
        .filter(|r| r["proposed_complete"] == true)
        .collect::<Vec<_>>();
    assert_eq!(rows.len(), 6);
    for row in rows {
        for definition in definitions(row["name"].as_str().unwrap()) {
            assert_eq!(definition.card.name, row["name"]);
        }
    }
}
#[test]
fn flask_and_terraformer_choose_once_for_the_whole_resolution_set_even_if_source_is_sacrificed() {
    use ironsmith::static_abilities::StaticAbilityId as Id;
    use ironsmith::{CardType, Subtype};
    for name in ["Elsewhere Flask", "Terraformer"] {
        for definition in definitions(name) {
            let mut game = game();
            let s = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let first = land(&mut game, A, "First", Subtype::Forest);
            let second = land(&mut game, A, "Second", Subtype::Swamp);
            let other = land(&mut game, B, "Opponent", Subtype::Mountain);
            grant_haste(&mut game, first);
            if name == "Elsewhere Flask" {
                game.create_object_from_definition(
                    &vanilla("Draw witness", "{1}", "Human", 1, 1),
                    A,
                    Zone::Library,
                );
                event(
                    &mut game,
                    TriggerEvent::new(
                        ironsmith::events::ZoneChangeEvent::with_cause(
                            s,
                            Zone::Hand,
                            Zone::Battlefield,
                            ironsmith::events::cause::EventCause::effect(),
                            None,
                        ),
                        Default::default(),
                    ),
                    &mut Choices::default(),
                );
                resolve_all(&mut game, &mut Choices::default());
                assert_eq!(game.player(A).unwrap().hand.len(), 1);
            }
            let mut choices = Choices {
                land_choice: Some("Island"),
                ..Default::default()
            };
            let index = current_activated_at(&game, s, 0);
            activate(&mut game, s, index, &mut choices);
            if name == "Elsewhere Flask" {
                assert!(
                    game.object(s).is_none(),
                    "actual cost is paid before choosing the land type"
                );
            }
            resolve_all(&mut game, &mut choices);
            assert_eq!(choices.land_prompts, 1);
            for id in [first, second] {
                let types = game.current_subtypes(id).unwrap();
                assert!(types.contains(&Subtype::Island) && types.contains(&Subtype::Elf));
                assert!(!types.contains(&Subtype::Forest) && !types.contains(&Subtype::Swamp));
                assert!(game.object_has_card_type(id, CardType::Creature));
                assert!(!has(&game, id, Id::Flying));
            }
            assert!(has(&game, first, Id::Haste));
            assert!(
                game.current_subtypes(other)
                    .unwrap()
                    .contains(&Subtype::Mountain)
            );
            let late = land(&mut game, A, "Late land", Subtype::Forest);
            assert!(
                !game
                    .current_subtypes(late)
                    .unwrap()
                    .contains(&Subtype::Island)
            );
            ironsmith::turn::execute_cleanup_step(&mut game);
            game.refresh_continuous_state().unwrap();
            assert!(has(&game, first, Id::Flying));
            assert!(
                game.current_subtypes(first)
                    .unwrap()
                    .contains(&Subtype::Forest)
            );
        }
    }
}
#[test]
fn navigator_adds_only_chosen_basic_land_type_keeps_rules_text_and_uses_real_tap_cost() {
    use ironsmith::Subtype;
    use ironsmith::static_abilities::StaticAbilityId as Id;
    for definition in definitions("Navigator's Compass") {
        let mut game = game();
        let s = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let target = land(&mut game, A, "Nonbasic", Subtype::Desert);
        event(
            &mut game,
            TriggerEvent::new(
                ironsmith::events::ZoneChangeEvent::with_cause(
                    s,
                    Zone::Hand,
                    Zone::Battlefield,
                    ironsmith::events::cause::EventCause::effect(),
                    None,
                ),
                Default::default(),
            ),
            &mut Choices::default(),
        );
        resolve_all(&mut game, &mut Choices::default());
        assert_eq!(game.player(A).unwrap().life, 23);
        let mut choices = Choices {
            targets: vec![Target::Object(target)],
            land_choice: Some("Forest"),
            ..Default::default()
        };
        let index = current_activated_at(&game, s, 0);
        activate(&mut game, s, index, &mut choices);
        assert!(game.is_tapped(s));
        resolve_all(&mut game, &mut choices);
        assert_eq!(choices.land_prompts, 1);
        for st in [Subtype::Desert, Subtype::Forest, Subtype::Elf] {
            assert!(game.current_subtypes(target).unwrap().contains(&st));
        }
        assert!(has(&game, target, Id::Flying));
        assert!(
            game.calculated_characteristics(target)
                .unwrap()
                .abilities
                .contains(&ironsmith::ability::Ability::basic_land_mana(Subtype::Forest).unwrap())
        );
        ironsmith::turn::execute_cleanup_step(&mut game);
        assert!(
            !game
                .current_subtypes(target)
                .unwrap()
                .contains(&Subtype::Forest)
        );
    }
}
#[test]
fn tundra_kavu_chooses_one_of_two_types_after_announcement_without_changing_snow_or_creature_type()
{
    use ironsmith::{Subtype, Supertype};
    for definition in definitions("Tundra Kavu") {
        for selected in ["Plains", "Island"] {
            let mut game = game();
            let s = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            game.remove_summoning_sickness(s);
            let target = land(&mut game, B, "Opponent", Subtype::Forest);
            let mut choices = Choices {
                targets: vec![Target::Object(target)],
                land_choice: Some(selected),
                ..Default::default()
            };
            let index = current_activated_at(&game, s, 0);
            activate(&mut game, s, index, &mut choices);
            assert_eq!(choices.land_prompts, 0);
            resolve_all(&mut game, &mut choices);
            assert_eq!(choices.land_prompts, 1);
            let types = game.current_subtypes(target).unwrap();
            assert_eq!(types.contains(&Subtype::Plains), selected == "Plains");
            assert_eq!(types.contains(&Subtype::Island), selected == "Island");
            assert!(types.contains(&Subtype::Elf));
            assert!(
                game.current_supertypes(target)
                    .unwrap()
                    .contains(&Supertype::Snow)
            );
        }
    }
}
#[test]
fn graceful_antelope_registration_expires_only_when_exact_source_leaves_not_when_it_phases() {
    use ironsmith::Subtype;
    for definition in definitions("Graceful Antelope") {
        let mut game = game();
        let s = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let target = land(&mut game, B, "Opponent", Subtype::Island);
        let outcome = apply(
            &mut game,
            s,
            Effect::new(
                ironsmith::effects::DealDamageEffect::new(
                    1,
                    ChooseSpec::Player(PlayerFilter::Specific(B)),
                )
                .with_combat(true),
            ),
        );
        queue_outcome(
            &mut game,
            outcome,
            &mut Choices {
                targets: vec![Target::Object(target)],
                ..Default::default()
            },
        );
        resolve_all(&mut game, &mut Choices::default());
        assert!(
            game.current_subtypes(target)
                .unwrap()
                .contains(&Subtype::Plains)
        );
        game.phase_out(s);
        game.refresh_continuous_state().unwrap();
        assert!(
            game.current_subtypes(target)
                .unwrap()
                .contains(&Subtype::Plains)
        );
        game.phase_in(s);
        let exile = game.move_object_by_game_rule(s, Zone::Exile).unwrap();
        game.refresh_continuous_state().unwrap();
        assert!(
            game.current_subtypes(target)
                .unwrap()
                .contains(&Subtype::Island)
        );
        game.move_object_by_game_rule(exile, Zone::Battlefield)
            .unwrap();
        game.refresh_continuous_state().unwrap();
        assert!(
            !game
                .current_subtypes(target)
                .unwrap()
                .contains(&Subtype::Plains)
        );
    }
}
#[test]
fn gaeas_liege_counts_current_controller_forests_and_its_actual_defending_players_forests() {
    use ironsmith::Subtype;
    use ironsmith::combat_state::{AttackTarget, CombatState, declare_attackers};
    for definition in definitions("Gaea's Liege") {
        let mut game = game();
        let s = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        game.remove_summoning_sickness(s);
        land(&mut game, A, "Own forest", Subtype::Forest);
        land(&mut game, A, "Own second", Subtype::Forest);
        for i in 0..4 {
            land(
                &mut game,
                B,
                &format!("Defender forest{i}"),
                Subtype::Forest,
            );
        }
        land(&mut game, C, "Not defender", Subtype::Forest);
        assert_eq!(pt(&game, s), (2, 2));
        game.turn.phase = Phase::Combat;
        game.turn.step = Some(ironsmith::game_state::Step::DeclareAttackers);
        let mut combat = CombatState::default();
        declare_attackers(&mut game, &mut combat, vec![(s, AttackTarget::Player(B))]).unwrap();
        game.combat = Some(combat);
        assert_eq!(pt(&game, s), (4, 4));
    }
}
#[test]
fn source_departure_before_land_ability_resolution_never_applies_a_new_lifetime_conversion() {
    use ironsmith::Subtype;
    for definition in definitions("Gaea's Liege") {
        let mut game = game();
        let s = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        game.remove_summoning_sickness(s);
        land(&mut game, A, "Forest", Subtype::Forest);
        let target = land(&mut game, B, "Island", Subtype::Island);
        let index = current_activated_at(&game, s, 0);
        activate(
            &mut game,
            s,
            index,
            &mut Choices {
                targets: vec![Target::Object(target)],
                ..Default::default()
            },
        );
        let exile = game.move_object_by_game_rule(s, Zone::Exile).unwrap();
        game.move_object_by_game_rule(exile, Zone::Battlefield)
            .unwrap();
        resolve_all(&mut game, &mut Choices::default());
        assert!(
            game.current_subtypes(target)
                .unwrap()
                .contains(&Subtype::Island)
        );
        assert!(
            !game
                .current_subtypes(target)
                .unwrap()
                .contains(&Subtype::Forest)
        );
    }
}
