//! UNVALIDATED receiver-scoped toughness assignment and complete-source combat regressions.
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
use ironsmith::triggers::{TriggerQueue, check_triggers};
use ironsmith::{GameProgress, GameState, ObjectId, PlayerId, Target, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler::parse_loss;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
const A: PlayerId = PlayerId::from_index(0);
const B: PlayerId = PlayerId::from_index(1);
const C: PlayerId = PlayerId::from_index(2);

fn fixtures() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!(
        "../../../fixtures/toughness_damage_assignment.json.fixture"
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
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Charlie".into()], 1000);
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
    votes: Option<[&'static str; 3]>,
    optional: Option<bool>,
    decline_targets: bool,
    bounds: Vec<(usize, Option<usize>)>,
}
impl DecisionMaker for Choices {
    fn decide_boolean(
        &mut self,
        _: &GameState,
        ctx: &ironsmith::decisions::context::BooleanContext,
    ) -> bool {
        if let Some(choice) = self.optional {
            return choice;
        }
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
        if let Some(votes) = self.votes
            && context.options.iter().all(|option| {
                matches!(
                    option.description.to_ascii_lowercase().as_str(),
                    "profit" | "security"
                )
            })
        {
            let index = if context.player == A {
                0
            } else if context.player == B {
                1
            } else {
                2
            };
            return vec![
                context
                    .options
                    .iter()
                    .find(|option| {
                        option.legal && option.description.eq_ignore_ascii_case(votes[index])
                    })
                    .unwrap()
                    .index,
            ];
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
        self.bounds.extend(
            context
                .requirements
                .iter()
                .map(|r| (r.min_targets, r.max_targets)),
        );
        if self.decline_targets {
            return Vec::new();
        }
        if !self.targets.is_empty() {
            assert!(self.targets.iter().all(|target| {
                context
                    .requirements
                    .iter()
                    .any(|r| r.legal_targets.contains(target))
            }));
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
            let selected = self
                .objects
                .iter()
                .copied()
                .filter(|id| {
                    context
                        .candidates
                        .iter()
                        .any(|candidate| candidate.id == *id && candidate.legal)
                })
                .take(context.max.unwrap_or(usize::MAX))
                .collect::<Vec<_>>();
            assert!(
                selected.len() >= context.min,
                "requested cost objects must satisfy the current payment decision"
            );
            selected
        } else {
            SelectFirstDecisionMaker.decide_objects(game, context)
        }
    }
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
    let mut queue = TriggerQueue::new();
    for _ in 0..30 {
        put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap();
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
    let id = game.create_object_from_definition(definition, A, Zone::Hand);
    let action = LegalAction::CastSpell {
        spell_id: id,
        from_zone: Zone::Hand,
        casting_method: method,
    };
    assert!(
        compute_legal_actions(game, A).unwrap().contains(&action),
        "activation {:?}, legal {:?}, phase {:?}, priority {:?}, stack {}",
        action,
        compute_legal_actions(game, A).unwrap(),
        game.turn.phase,
        game.turn.priority_player,
        game.stack.len()
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
    let ordinal = game
        .object(source)
        .unwrap()
        .abilities
        .iter()
        .take(ability_index)
        .filter(|ability| matches!(ability.kind, ironsmith::ability::AbilityKind::Activated(_)))
        .count();
    let ability_index = game
        .calculated_characteristics(source)
        .unwrap()
        .abilities
        .iter()
        .enumerate()
        .filter(|(_, ability)| {
            matches!(ability.kind, ironsmith::ability::AbilityKind::Activated(_))
        })
        .nth(ordinal)
        .unwrap()
        .0;
    let action = LegalAction::ActivateAbility {
        source,
        ability_index,
    };
    assert!(
        compute_legal_actions(game, A).unwrap().contains(&action),
        "activation {:?}, legal {:?}, phase {:?}, priority {:?}, stack {}",
        action,
        compute_legal_actions(game, A).unwrap(),
        game.turn.phase,
        game.turn.priority_player,
        game.stack.len()
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
use ironsmith::Subtype;
use ironsmith::object::{CounterType, ObjectKind};
fn activated_at(definition: &CardDefinition, n: usize) -> usize {
    definition
        .abilities
        .iter()
        .enumerate()
        .filter(|(_, ability)| {
            matches!(ability.kind, ironsmith::ability::AbilityKind::Activated(_))
        })
        .nth(n)
        .unwrap()
        .0
}
fn token_ids(game: &GameState, controller: PlayerId, subtype: Subtype) -> Vec<ObjectId> {
    game.battlefield
        .iter()
        .copied()
        .filter(|id| {
            game.object(*id).is_some_and(|object| {
                object.kind == ObjectKind::Token && object.has_subtype(subtype)
            }) && game.current_controller(*id) == Some(controller)
        })
        .collect()
}
fn creature(game: &mut GameState, owner: PlayerId, p: i32, t: i32, subtype: &str) -> ObjectId {
    game.create_object_from_definition(
        &vanilla("Quantity subject", "{2}", subtype, p, t),
        owner,
        Zone::Battlefield,
    )
}
fn counter(
    game: &mut GameState,
    source: ObjectId,
    id: ObjectId,
    kind: CounterType,
    count: i32,
) -> EffectOutcome {
    apply(
        game,
        source,
        Effect::put_counters(kind, count, ChooseSpec::SpecificObject(id)),
    )
}
fn attack(game: &mut GameState, source: ObjectId, dm: &mut Choices) {
    game.remove_summoning_sickness(source);
    game.turn.phase = Phase::Combat;
    game.turn.step = Some(ironsmith::game_state::Step::DeclareAttackers);
    let mut combat = ironsmith::combat_state::CombatState::default();
    let mut queue = TriggerQueue::new();
    ironsmith::game_loop::apply_attacker_declarations(
        game,
        &mut combat,
        &mut queue,
        &[ironsmith::decision::AttackerDeclaration {
            creature: source,
            target: ironsmith::combat_state::AttackTarget::Player(B),
        }],
    )
    .unwrap();
    game.combat = Some(combat);
    put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap();
}
use ironsmith::static_abilities::{StaticAbility, StaticAbilityId};
fn rule(game: &GameState, object: ObjectId) -> bool {
    game.current_has_static_ability_id(
        object,
        StaticAbilityId::ThisCreatureAssignsCombatDamageUsingToughness,
    )
}
fn library(game: &mut GameState, count: usize) {
    for _ in 0..count {
        game.create_object_from_definition(
            &vanilla("Draw witness", "{1}", "Human", 1, 1),
            A,
            Zone::Library,
        );
    }
}
fn enter(game: &mut GameState, definition: &CardDefinition, dm: &mut Choices) -> ObjectId {
    let spell = cast(game, definition, CastingMethod::Normal, dm);
    let stable = game.object(spell).unwrap().stable_id;
    resolve_all(game, dm);
    game.find_object_by_stable_id(stable).unwrap()
}
fn damage(game: &mut GameState, attacker: ObjectId, general_path: bool) -> u32 {
    let mut combat = ironsmith::combat_state::CombatState::default();
    let defender = if game.current_controller(attacker) == Some(B) {
        C
    } else {
        B
    };
    combat
        .attackers
        .push(ironsmith::combat_state::AttackerInfo {
            creature: attacker,
            target: ironsmith::combat_state::AttackTarget::Player(defender),
        });
    if general_path {
        let blocker = creature(game, defender, 0, 1000, "Wall");
        combat.blockers.insert(attacker, vec![blocker]);
    } else {
        combat.blockers.insert(attacker, Vec::new());
    }
    ironsmith::game_loop::execute_combat_damage_step(game, &combat, false)
        .iter()
        .filter(|event| event.source == attacker)
        .map(|event| event.amount)
        .sum()
}
fn cleanup(game: &mut GameState) {
    ironsmith::turn::execute_cleanup_step(game);
}
fn defender_creature(
    game: &mut GameState,
    owner: PlayerId,
    power: i32,
    toughness: i32,
) -> ObjectId {
    let definition = compile_to_runtime_definition("Defender witness", format!("Mana cost: {{1}}\nType: Creature — Wall\nPower/Toughness: {power}/{toughness}\nDefender"), false).unwrap();
    game.create_object_from_definition(&definition, owner, Zone::Battlefield)
}
#[test]
fn eleven_complete_frozen_cards_keep_direct_and_serialized_semantics() {
    let rows = fixtures();
    assert_eq!(rows.len(), 12);
    let complete = rows
        .iter()
        .filter(|row| row["coverage_status"] == "proposed_complete")
        .collect::<Vec<_>>();
    assert_eq!(complete.len(), 11);
    for row in complete {
        for definition in definitions(row["name"].as_str().unwrap()) {
            assert_eq!(definition.card.name, row["name"].as_str().unwrap());
        }
    }
}
#[test]
fn static_filters_track_each_creatures_current_axes_controller_and_source_departure_in_both_damage_paths()
 {
    for name in [
        "Ancient Lumberknot",
        "Bedrock Tortoise",
        "Ghalta the Immovable",
    ] {
        for definition in definitions(name) {
            for general in [false, true] {
                let mut game = self::game();
                let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
                let own = creature(&mut game, A, 2, 5, "Human");
                let other = creature(&mut game, B, 2, 5, "Human");
                assert!(rule(&game, own));
                assert!(!rule(&game, other));
                assert_eq!(damage(&mut game, own, general), 5);
                assert_eq!(damage(&mut game, other, general), 2);
                apply(
                    &mut game,
                    source,
                    Effect::pump(6, 0, ChooseSpec::SpecificObject(own), Until::EndOfTurn),
                );
                assert!(
                    !rule(&game, own),
                    "{name} {:?}",
                    game.calculated_characteristics(own)
                        .unwrap()
                        .static_abilities
                );
                assert_eq!(damage(&mut game, own, general), 8);
                apply(
                    &mut game,
                    source,
                    Effect::pump(0, 5, ChooseSpec::SpecificObject(own), Until::EndOfTurn),
                );
                assert!(rule(&game, own));
                assert_eq!(damage(&mut game, own, general), 10);
                assert_eq!(
                    game.calculated_power(own),
                    Some(8),
                    "assignment does not overwrite actual power"
                );
                game.set_current_controller(source, B).unwrap();
                assert!(!rule(&game, own));
                assert!(rule(&game, other));
                game.move_object_by_game_rule(source, Zone::Graveyard)
                    .unwrap();
                assert!(!rule(&game, other));
                assert_eq!(damage(&mut game, other, general), 2);
            }
        }
    }
}
#[test]
fn bark_uses_its_current_equipped_creature_and_retains_the_independent_toughness_bonus() {
    for definition in definitions("Bark of Doran") {
        let mut game = self::game();
        let equipment = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let first = creature(&mut game, A, 2, 5, "Human");
        let second = creature(&mut game, A, 1, 8, "Human");
        activate(
            &mut game,
            equipment,
            activated_at(&definition, 0),
            &mut Choices {
                targets: vec![Target::Object(first)],
                ..Default::default()
            },
        );
        resolve_all(&mut game, &mut Choices::default());
        assert!(!rule(&game, equipment));
        assert!(rule(&game, first));
        assert!(!rule(&game, second));
        assert_eq!(damage(&mut game, first, false), 6);
        apply(
            &mut game,
            equipment,
            Effect::pump(10, 0, ChooseSpec::SpecificObject(first), Until::EndOfTurn),
        );
        assert!(!rule(&game, first));
        assert_eq!(damage(&mut game, first, true), 12);
        activate(
            &mut game,
            equipment,
            activated_at(&definition, 0),
            &mut Choices {
                targets: vec![Target::Object(second)],
                ..Default::default()
            },
        );
        resolve_all(&mut game, &mut Choices::default());
        assert!(!rule(&game, first));
        assert!(rule(&game, second));
        assert_eq!(damage(&mut game, second, false), 9);
        game.set_current_controller(equipment, B).unwrap();
        assert!(rule(&game, second));
        game.move_object_by_game_rule(equipment, Zone::Graveyard)
            .unwrap();
        assert!(!rule(&game, second));
        assert_eq!(game.calculated_toughness(second), Some(8));
    }
}
#[test]
fn solid_footing_tracks_the_enchanted_receivers_current_vigilance() {
    for definition in definitions("Solid Footing") {
        let mut game = self::game();
        let target = creature(&mut game, A, 2, 5, "Human");
        let aura = enter(
            &mut game,
            &definition,
            &mut Choices {
                targets: vec![Target::Object(target)],
                ..Default::default()
            },
        );
        assert!(!rule(&game, target));
        assert_eq!(damage(&mut game, target, false), 3);
        apply(
            &mut game,
            aura,
            Effect::grant(
                ironsmith::grant::Grantable::ability(StaticAbility::vigilance()),
                ChooseSpec::SpecificObject(target),
                ironsmith::grant::GrantDuration::UntilEndOfTurn,
            ),
        );
        assert!(rule(&game, target));
        assert!(!rule(&game, aura));
        assert_eq!(damage(&mut game, target, true), 6);
        cleanup(&mut game);
        assert!(!rule(&game, target));
        assert_eq!(damage(&mut game, target, false), 3);
    }
}
#[test]
fn plagon_draws_for_its_entry_set_and_its_activation_mandatorily_uses_live_toughness_until_cleanup()
{
    for definition in definitions("Plagon, Lord of the Beach") {
        let mut game = self::game();
        library(&mut game, 8);
        let _eligible = creature(&mut game, A, 2, 5, "Human");
        let target = creature(&mut game, A, 7, 2, "Human");
        let source = enter(&mut game, &definition, &mut Choices::default());
        assert_eq!(game.player(A).unwrap().hand.len(), 2);
        activate(
            &mut game,
            source,
            activated_at(&definition, 0),
            &mut Choices {
                targets: vec![Target::Object(target)],
                ..Default::default()
            },
        );
        resolve_all(&mut game, &mut Choices::default());
        assert_eq!(
            damage(&mut game, target, false),
            2,
            "mandatory toughness is used even when power is greater"
        );
        apply(
            &mut game,
            source,
            Effect::pump(0, 3, ChooseSpec::SpecificObject(target), Until::EndOfTurn),
        );
        assert_eq!(damage(&mut game, target, true), 5);
        cleanup(&mut game);
        assert!(!rule(&game, target));
        assert_eq!(damage(&mut game, target, false), 7);
    }
}
#[test]
fn bill_creates_food_pays_a_real_food_cost_and_grants_only_the_chosen_creature() {
    for definition in definitions("Bill the Pony") {
        let mut game = self::game();
        let target = creature(&mut game, A, 7, 2, "Human");
        let source = enter(&mut game, &definition, &mut Choices::default());
        let food = token_ids(&game, A, Subtype::Food);
        assert_eq!(food.len(), 2);
        activate(
            &mut game,
            source,
            activated_at(&definition, 0),
            &mut Choices {
                objects: vec![food[0]],
                targets: vec![Target::Object(target)],
                ..Default::default()
            },
        );
        assert_eq!(token_ids(&game, A, Subtype::Food).len(), 1);
        resolve_all(&mut game, &mut Choices::default());
        assert!(rule(&game, target));
        assert!(!rule(&game, source));
        assert_eq!(damage(&mut game, target, false), 2);
        cleanup(&mut game);
        assert_eq!(damage(&mut game, target, false), 7);
    }
}
#[test]
fn walking_bulwark_keeps_sorcery_timing_and_all_three_grants_on_one_defender() {
    for definition in definitions("Walking Bulwark") {
        let mut game = self::game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let target = defender_creature(&mut game, B, 1, 7);
        let index = activated_at(&definition, 0);
        activate(
            &mut game,
            source,
            index,
            &mut Choices {
                targets: vec![Target::Object(target)],
                ..Default::default()
            },
        );
        resolve_all(&mut game, &mut Choices::default());
        assert!(rule(&game, target));
        assert!(game.current_has_static_ability_id(target, StaticAbilityId::Haste));
        assert!(
            game.current_has_static_ability_id(
                target,
                StaticAbilityId::CanAttackAsThoughNoDefender
            )
        );
        assert_eq!(damage(&mut game, target, false), 7);
        game.turn.phase = Phase::Combat;
        assert!(!compute_legal_actions(&game, A).unwrap().contains(
            &LegalAction::ActivateAbility {
                source,
                ability_index: index
            }
        ));
        cleanup(&mut game);
        assert!(!rule(&game, target));
        assert!(!game.current_has_static_ability_id(target, StaticAbilityId::Haste));
        assert!(
            !game.current_has_static_ability_id(
                target,
                StaticAbilityId::CanAttackAsThoughNoDefender
            )
        );
        assert_eq!(damage(&mut game, target, false), 1);
    }
}
#[test]
fn arcades_preserves_entry_draws_defender_attack_permission_and_toughness_assignment_together() {
    for definition in definitions("Arcades, the Strategist") {
        let mut game = self::game();
        library(&mut game, 5);
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let wall = compile_to_runtime_definition(
            "Entering defender",
            "Mana cost: {1}\nType: Creature — Wall\nPower/Toughness: 1/5\nDefender",
            false,
        )
        .unwrap();
        let target = enter(&mut game, &wall, &mut Choices::default());
        assert_eq!(game.player(A).unwrap().hand.len(), 1);
        game.remove_summoning_sickness(target);
        assert!(ironsmith::rules::combat::can_attack(
            game.object(target).unwrap(),
            &game
        ));
        assert!(rule(&game, target));
        assert_eq!(damage(&mut game, target, false), 5);
        assert_eq!(game.calculated_power(target), Some(1));
        let plain = vanilla("Entering ordinary creature", "{1}", "Human", 2, 5);
        enter(&mut game, &plain, &mut Choices::default());
        assert_eq!(
            game.player(A).unwrap().hand.len(),
            1,
            "ordinary creatures do not trigger the defender draw"
        );
        game.move_object_by_game_rule(source, Zone::Graveyard)
            .unwrap();
        assert!(!ironsmith::rules::combat::can_attack(
            game.object(target).unwrap(),
            &game
        ));
        assert!(!rule(&game, target));
        assert_eq!(damage(&mut game, target, true), 1);
    }
}
#[test]
fn ghaltas_cost_reads_current_greatest_controlled_toughness_and_keeps_the_white_pip() {
    for definition in definitions("Ghalta the Immovable") {
        let mut game = self::game();
        let own = defender_creature(&mut game, A, 0, 6);
        let _opponent = creature(&mut game, B, 0, 20, "Wall");
        game.player_mut(A).unwrap().mana_pool = Default::default();
        game.player_mut(A)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::White, 1);
        let spell = game.create_object_from_definition(&definition, A, Zone::Hand);
        let action = LegalAction::CastSpell {
            spell_id: spell,
            from_zone: Zone::Hand,
            casting_method: CastingMethod::Normal,
        };
        assert!(
            !compute_legal_actions(&game, A).unwrap().contains(&action),
            "opponent toughness must not reduce the cost"
        );
        apply(
            &mut game,
            own,
            Effect::pump(0, 2, ChooseSpec::SpecificObject(own), Until::EndOfTurn),
        );
        assert!(compute_legal_actions(&game, A).unwrap().contains(&action));
        // Move the uncast probe out of the hand; the common helper announces
        // a fresh exact card and pays its real currently reduced cost.
        game.move_object_by_game_rule(spell, Zone::Exile).unwrap();
        let source = enter(&mut game, &definition, &mut Choices::default());
        assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
        game.remove_summoning_sickness(own);
        assert!(ironsmith::rules::combat::can_attack(
            game.object(own).unwrap(),
            &game
        ));
        assert!(rule(&game, own));
        assert_eq!(damage(&mut game, own, false), 8);
        assert_eq!(game.calculated_toughness(source), Some(7));
    }
}
#[test]
fn baldin_allows_zero_through_one_hundred_targets_uses_resolution_hand_count_and_only_changes_assignment_on_its_controllers_turn()
 {
    for definition in definitions("Baldin, Century Herdmaster") {
        for decline in [false, true] {
            let mut game = self::game();
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let own = creature(&mut game, A, 2, 5, "Human");
            let other = creature(&mut game, B, 3, 4, "Human");
            for _ in 0..2 {
                game.create_object_from_definition(
                    &vanilla("Hand count witness", "{1}", "Human", 1, 1),
                    A,
                    Zone::Hand,
                );
            }
            library(&mut game, 1);
            let mut dm = Choices {
                targets: if decline {
                    vec![]
                } else {
                    vec![Target::Object(own), Target::Object(other)]
                },
                decline_targets: decline,
                ..Default::default()
            };
            attack(&mut game, source, &mut dm);
            assert!(dm.bounds.contains(&(0, Some(100))));
            apply(&mut game, source, Effect::draw(1));
            resolve_all(&mut game, &mut Choices::default());
            assert_eq!(
                game.calculated_toughness(own),
                Some(if decline { 5 } else { 8 })
            );
            assert_eq!(
                game.calculated_toughness(other),
                Some(if decline { 4 } else { 7 })
            );
            assert!(
                rule(&game, other),
                "during your turn includes opposing creatures"
            );
            assert_eq!(damage(&mut game, own, false), if decline { 5 } else { 8 });
            cleanup(&mut game);
            assert_eq!(game.calculated_toughness(own), Some(5));
            game.turn.phase = Phase::Ending;
            game.turn.step = Some(ironsmith::game_state::Step::Cleanup);
            ironsmith::turn::advance_step(&mut game).unwrap();
            assert_eq!(game.turn.active_player, B);
            assert!(!rule(&game, own));
            assert!(!rule(&game, other));
            assert_eq!(damage(&mut game, own, false), 2);
        }
    }
}
#[test]
fn kingpin_optional_payment_locks_only_the_resolution_set_and_extort_still_executes() {
    for definition in definitions("The Kingpin of Crime") {
        for pay in [false, true] {
            let mut game = self::game();
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let own = creature(&mut game, A, 2, 5, "Human");
            let boundary = creature(&mut game, A, 2, 2, "Human");
            let opponent = creature(&mut game, B, 2, 6, "Human");
            attack(&mut game, source, &mut Choices::default());
            apply(
                &mut game,
                source,
                Effect::pump(0, 1, ChooseSpec::SpecificObject(boundary), Until::EndOfTurn),
            );
            resolve_all(
                &mut game,
                &mut Choices {
                    optional: Some(pay),
                    ..Default::default()
                },
            );
            assert_eq!(game.player(A).unwrap().life, 1000 - if pay { 2 } else { 0 });
            assert_eq!(rule(&game, own), pay);
            assert_eq!(rule(&game, boundary), pay);
            assert!(!rule(&game, opponent));
            let late = creature(&mut game, A, 1, 6, "Human");
            assert!(!rule(&game, late));
            apply(
                &mut game,
                source,
                Effect::pump(5, 0, ChooseSpec::SpecificObject(own), Until::EndOfTurn),
            );
            assert_eq!(
                damage(&mut game, own, false),
                if pay { 5 } else { 7 },
                "a resolving grant keeps its recipients after P/T changes"
            );
            cleanup(&mut game);
            assert!(!rule(&game, own));
            assert!(!rule(&game, boundary));
        }
        let mut game = self::game();
        game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let spell = compile_to_runtime_definition(
            "Extort witness",
            "Mana cost: {W}\nType: Instant\nYou gain 1 life.",
            false,
        )
        .unwrap();
        cast(
            &mut game,
            &spell,
            CastingMethod::Normal,
            &mut Choices::default(),
        );
        resolve_all(&mut game, &mut Choices::default());
        assert_eq!(game.player(A).unwrap().life, 1003);
        assert_eq!(game.player(B).unwrap().life, 999);
        assert_eq!(game.player(C).unwrap().life, 999);
    }
}
#[test]
fn tapestry_combat_subset_uses_the_same_rule_without_claiming_its_station_body() {
    let row = fixtures()
        .into_iter()
        .find(|row| row["name"] == "Tapestry Warden")
        .unwrap();
    assert_eq!(row["coverage_status"], "partial_station_payment");
    let body = row["oracle_text"]
        .as_str()
        .unwrap()
        .lines()
        .take(2)
        .collect::<Vec<_>>()
        .join("\n");
    for definition in definitions_text(
        "Tapestry combat subset",
        &format!("Type: Creature\nPower/Toughness: 1/1\n{body}"),
    ) {
        let mut game = self::game();
        game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let target = creature(&mut game, A, 2, 5, "Human");
        assert_eq!(damage(&mut game, target, false), 5);
    }
}
