//! UNVALIDATED full-source token quantities, prior-action branches and payment regressions.
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
        "../../../fixtures/dynamic_token_quantities.json.fixture"
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
    mode: Option<usize>,
    x: u32,
    prefer_life: bool,
    viewed: Vec<Vec<ObjectId>>,
    untap: bool,
    votes: Option<[&'static str; 3]>,
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
#[test]
fn all_five_exact_complete_sources_have_strict_direct_and_restored_artifacts() {
    for row in fixtures() {
        let name = row["name"].as_str().unwrap();
        for definition in definitions(name) {
            assert_eq!(definition.card.name, name);
        }
    }
}
#[test]
fn emissary_named_votes_scale_treasures_and_keep_security_counters_and_ability_controller() {
    use ironsmith::cards::builders::CardDefinitionBuilder;
    use ironsmith::static_abilities::StaticAbility;
    for definition in definitions("Emissary Green") {
        for extra_vote in [false, true] {
            let mut game = self::game();
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let ally = creature(&mut game, A, 2, 2, "Human");
            let enemy = creature(&mut game, B, 2, 2, "Human");
            if extra_vote {
                let card = CardDefinitionBuilder::new(ironsmith::CardId::new(), "Additional voter")
                    .card_types(vec![ironsmith::CardType::Enchantment])
                    .with_ability(ironsmith::Ability::static_ability(
                        StaticAbility::vote_additional_time_while_voting(),
                    ))
                    .build();
                game.create_object_from_definition(&card, A, Zone::Battlefield);
            }
            attack(&mut game, source, &mut Choices::default());
            assert_eq!(game.stack.len(), 1);
            game.set_current_controller(source, B).unwrap();
            resolve_all(
                &mut game,
                &mut Choices {
                    votes: Some(["profit", "profit", "security"]),
                    ..Default::default()
                },
            );
            assert_eq!(
                token_ids(&game, A, Subtype::Treasure).len(),
                if extra_vote { 6 } else { 4 }
            );
            assert!(token_ids(&game, B, Subtype::Treasure).is_empty());
            assert_eq!(game.counter_count(ally, CounterType::PlusOnePlusOne), 1);
            assert_eq!(game.counter_count(enemy, CounterType::PlusOnePlusOne), 0);
            assert_eq!(game.counter_count(source, CounterType::PlusOnePlusOne), 0);
        }
    }
}
#[test]
fn emissary_zero_profit_does_not_skip_the_security_instruction() {
    for definition in definitions("Emissary Green") {
        let mut game = self::game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        attack(&mut game, source, &mut Choices::default());
        resolve_all(
            &mut game,
            &mut Choices {
                votes: Some(["security", "security", "security"]),
                ..Default::default()
            },
        );
        assert!(token_ids(&game, A, Subtype::Treasure).is_empty());
        assert_eq!(game.counter_count(source, CounterType::PlusOnePlusOne), 3);
    }
}
#[test]
fn lacerate_uses_its_actual_damage_excess_and_an_illegal_target_counters_the_whole_spell() {
    for definition in definitions("Lacerate Flesh") {
        for mode in 0..3 {
            let mut game = self::game();
            let witness = creature(&mut game, A, 2, 2, "Human");
            let target = creature(&mut game, B, 1, 1, "Human");
            cast(
                &mut game,
                &definition,
                CastingMethod::Normal,
                &mut Choices {
                    targets: vec![Target::Object(target)],
                    ..Default::default()
                },
            );
            if mode == 1 {
                apply(
                    &mut game,
                    witness,
                    Effect::prevent_damage(3, ChooseSpec::SpecificObject(target), Until::EndOfTurn),
                );
            }
            if mode == 2 {
                let exile = game.move_object_by_game_rule(target, Zone::Exile).unwrap();
                game.move_object_by_game_rule(exile, Zone::Battlefield)
                    .unwrap();
            }
            resolve_all(&mut game, &mut Choices::default());
            assert_eq!(
                token_ids(&game, A, Subtype::Blood).len(),
                if mode == 0 { 3 } else { 0 }
            );
        }
    }
}
#[test]
fn sorin_highest_life_is_current_maximum_once_even_for_ties_and_above_five_hundred() {
    for definition in definitions("Sorin, Grim Nemesis") {
        for high in [24, 501] {
            let mut game = self::game();
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            counter(&mut game, source, source, CounterType::Loyalty, 3);
            activate(
                &mut game,
                source,
                activated_at(&definition, 2),
                &mut Choices::default(),
            );
            // Respond after announcement; tied maxima are a number, not a sum.
            apply(
                &mut game,
                source,
                Effect::gain_life_player(high - 20, ChooseSpec::SpecificPlayer(B)),
            );
            apply(
                &mut game,
                source,
                Effect::gain_life_player(high - 20, ChooseSpec::SpecificPlayer(C)),
            );
            resolve_all(&mut game, &mut Choices::default());
            let tokens = token_ids(&game, A, Subtype::Vampire);
            assert_eq!(tokens.len(), high as usize);
            for token in tokens {
                assert!(game.object(token).unwrap().has_subtype(Subtype::Knight));
                assert_eq!(game.calculated_power(token), Some(1));
                assert!(game.current_has_static_ability_id(
                    token,
                    ironsmith::static_abilities::StaticAbilityId::Lifelink
                ));
            }
        }
    }
}
#[test]
fn sorins_other_two_abilities_keep_revealed_card_value_and_announced_loyalty_x() {
    for definition in definitions("Sorin, Grim Nemesis") {
        let mut game = self::game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let reveal = game.create_object_from_definition(
            &vanilla("Sorin reveal", "{2}{U}{U}", "Spirit", 2, 2),
            A,
            Zone::Library,
        );
        let stable = game.object(reveal).unwrap().stable_id;
        activate(
            &mut game,
            source,
            activated_at(&definition, 0),
            &mut Choices::default(),
        );
        resolve_all(&mut game, &mut Choices::default());
        assert_eq!(
            game.object(game.find_object_by_stable_id(stable).unwrap())
                .unwrap()
                .zone,
            Zone::Hand
        );
        assert_eq!(game.player(B).unwrap().life, 16);
        assert_eq!(game.player(C).unwrap().life, 16);
        assert_eq!(game.player(A).unwrap().life, 20);
        let mut game = self::game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let target = creature(&mut game, B, 2, 8, "Human");
        activate(
            &mut game,
            source,
            activated_at(&definition, 1),
            &mut Choices {
                x: 3,
                targets: vec![Target::Object(target)],
                ..Default::default()
            },
        );
        resolve_all(&mut game, &mut Choices::default());
        assert_eq!(game.damage_on(target), 3);
        assert_eq!(game.player(A).unwrap().life, 23);
        assert_eq!(game.counter_count(source, CounterType::Loyalty), 3);
    }
}
fn land(game: &mut GameState, owner: PlayerId) -> ObjectId {
    let card =
        compile_to_runtime_definition("Land quantity subject", "Type: Basic Land — Plains", false)
            .unwrap();
    game.create_object_from_definition(&card, owner, Zone::Battlefield)
}
#[test]
fn waking_all_three_chapters_keep_zone_controller_and_selected_opponent_difference() {
    for definition in definitions("Waking the Trolls") {
        for target_player in [B, C] {
            let mut game = self::game();
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let first_owned_land = land(&mut game, A);
            land(&mut game, A);
            land(&mut game, A);
            let stolen_land = land(&mut game, B);
            let stable = game.object(stolen_land).unwrap().stable_id;
            land(&mut game, B);
            for _ in 0..6 {
                land(&mut game, C);
            }
            let outcome = counter(&mut game, source, source, CounterType::Lore, 1);
            queue_outcome(
                &mut game,
                outcome,
                &mut Choices {
                    targets: vec![Target::Object(stolen_land)],
                    ..Default::default()
                },
            );
            resolve_all(&mut game, &mut Choices::default());
            let grave = game.find_object_by_stable_id(stable).unwrap();
            assert_eq!(game.object(grave).unwrap().zone, Zone::Graveyard);
            let outcome = counter(&mut game, source, source, CounterType::Lore, 1);
            queue_outcome(
                &mut game,
                outcome,
                &mut Choices {
                    targets: vec![Target::Object(grave)],
                    ..Default::default()
                },
            );
            resolve_all(&mut game, &mut Choices::default());
            let returned = game.find_object_by_stable_id(stable).unwrap();
            assert_eq!(game.current_controller(returned), Some(A));
            assert_eq!(game.object(returned).unwrap().owner, B);
            let outcome = counter(&mut game, source, source, CounterType::Lore, 1);
            queue_outcome(
                &mut game,
                outcome,
                &mut Choices {
                    targets: vec![Target::Player(target_player)],
                    ..Default::default()
                },
            );
            game.move_object_by_game_rule(first_owned_land, Zone::Graveyard)
                .unwrap();
            game.set_current_controller(source, B).unwrap();
            resolve_all(&mut game, &mut Choices::default());
            let tokens = token_ids(&game, A, Subtype::Troll);
            assert_eq!(tokens.len(), if target_player == B { 2 } else { 0 });
            for token in tokens {
                assert!(game.object(token).unwrap().has_subtype(Subtype::Warrior));
                assert_eq!(game.calculated_power(token), Some(4));
                assert!(game.current_has_static_ability_id(
                    token,
                    ironsmith::static_abilities::StaticAbilityId::Trample
                ));
            }
        }
    }
}
#[test]
fn shilgengar_angel_branch_replaces_one_blood_with_actual_sacrificed_toughness() {
    for definition in definitions("Shilgengar, Sire of Famine") {
        for angel in [false, true] {
            let mut game = self::game();
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let paid = creature(&mut game, A, 1, 2, if angel { "Angel" } else { "Human" });
            apply(
                &mut game,
                source,
                Effect::pump(0, 3, ChooseSpec::SpecificObject(paid), Until::EndOfTurn),
            );
            let stable = game.object(paid).unwrap().stable_id;
            activate(
                &mut game,
                source,
                activated_at(&definition, 0),
                &mut Choices {
                    objects: vec![paid],
                    ..Default::default()
                },
            );
            assert!(game.object(paid).is_none());
            if angel {
                // A new incarnation of the same card is not the paid object.
                let grave = game.find_object_by_stable_id(stable).unwrap();
                let returned = game
                    .move_object_by_game_rule(grave, Zone::Battlefield)
                    .unwrap();
                apply(
                    &mut game,
                    source,
                    Effect::pump(
                        0,
                        100,
                        ChooseSpec::SpecificObject(returned),
                        Until::EndOfTurn,
                    ),
                );
            }
            game.set_current_controller(source, B).unwrap();
            resolve_all(&mut game, &mut Choices::default());
            assert!(token_ids(&game, B, Subtype::Blood).is_empty());
            assert_eq!(
                token_ids(&game, A, Subtype::Blood).len(),
                if angel { 5 } else { 1 },
                "use departure toughness, not source toughness, printed toughness, or one plus the replacement amount"
            );
        }
    }
}
#[test]
fn shilgengar_pays_six_blood_and_only_returned_creatures_gain_vampire_and_finality() {
    for definition in definitions("Shilgengar, Sire of Famine") {
        let mut game = self::game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        apply(
            &mut game,
            source,
            Effect::create_tokens(ironsmith::cards::tokens::blood_token_definition(), 6),
        );
        let blood = token_ids(&game, A, Subtype::Blood);
        assert_eq!(blood.len(), 6);
        let returning = game.create_object_from_definition(
            &vanilla("Returning human", "{2}", "Human", 2, 2),
            A,
            Zone::Graveyard,
        );
        let stable = game.object(returning).unwrap().stable_id;
        let second = game.create_object_from_definition(
            &vanilla("Returning spirit", "{1}", "Spirit", 1, 1),
            A,
            Zone::Graveyard,
        );
        let second_stable = game.object(second).unwrap().stable_id;
        let untouched = creature(&mut game, A, 2, 2, "Human");
        let opponent = game.create_object_from_definition(
            &vanilla("Opponent grave", "{1}", "Human", 1, 1),
            B,
            Zone::Graveyard,
        );
        activate(
            &mut game,
            source,
            activated_at(&definition, 1),
            &mut Choices {
                objects: blood,
                ..Default::default()
            },
        );
        resolve_all(&mut game, &mut Choices::default());
        assert!(token_ids(&game, A, Subtype::Blood).is_empty());
        for (stable, original) in [(stable, Subtype::Human), (second_stable, Subtype::Spirit)] {
            let returned = game.find_object_by_stable_id(stable).unwrap();
            let chars = game.calculated_characteristics(returned).unwrap();
            assert!(chars.subtypes.contains(&original));
            assert!(chars.subtypes.contains(&Subtype::Vampire));
            assert_eq!(game.counter_count(returned, CounterType::Finality), 1);
            apply(
                &mut game,
                source,
                Effect::destroy(ChooseSpec::SpecificObject(returned)),
            );
            assert_eq!(
                game.object(game.find_object_by_stable_id(stable).unwrap())
                    .unwrap()
                    .zone,
                Zone::Exile
            );
        }
        assert!(
            !game
                .calculated_characteristics(untouched)
                .unwrap()
                .subtypes
                .contains(&Subtype::Vampire)
        );
        assert_eq!(game.object(opponent).unwrap().zone, Zone::Graveyard);
    }
}
