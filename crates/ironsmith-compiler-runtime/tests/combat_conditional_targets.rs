use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::combat_state::AttackTarget;
use ironsmith::decision::{AttackerDeclaration, SelectFirstDecisionMaker};
use ironsmith::effect::{Condition, Effect, Until};
use ironsmith::game_loop::{
    apply_attacker_declarations, put_triggers_on_stack_with_dm, resolve_stack_entry_with,
};
use ironsmith::resolution::ResolutionProgram;
use ironsmith::target::{ChooseSpec, ObjectFilter};
use ironsmith::triggers::{Trigger, TriggerQueue};
use ironsmith::{
    Ability, CardId, CardType, CombatState, GameState, ObjectId, PlayerId, PowerToughness, Target,
    Zone,
};

fn attacker(game: &mut GameState, condition: Condition) -> ObjectId {
    let definition = CardDefinitionBuilder::new(CardId::new(), "Conditional attacker")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let source = game.create_object_from_definition(&definition, PlayerId(0), Zone::Battlefield);
    game.object_mut(source).unwrap().abilities = std::sync::Arc::new(vec![Ability::triggered(
        Trigger::this_attacks(),
        ResolutionProgram::from_effects(vec![Effect::conditional_only(
            condition,
            vec![Effect::pump(
                1,
                1,
                ChooseSpec::target_creature(),
                Until::EndOfTurn,
            )],
        )]),
    )]);
    game.remove_summoning_sickness(source);
    source
}

fn declare(game: &mut GameState, source: ObjectId) {
    game.turn.active_player = PlayerId(0);
    game.turn.phase = ironsmith::Phase::Combat;
    game.turn.step = Some(ironsmith::game_state::Step::DeclareAttackers);
    let mut combat = CombatState::default();
    let mut queue = TriggerQueue::new();
    apply_attacker_declarations(
        game,
        &mut combat,
        &mut queue,
        &[AttackerDeclaration {
            creature: source,
            target: AttackTarget::Player(PlayerId(1)),
        }],
    )
    .unwrap();
    game.combat = Some(combat);
    put_triggers_on_stack_with_dm(game, &mut queue, &mut SelectFirstDecisionMaker).unwrap();
}

#[test]
fn attacking_alone_conditional_announces_target_and_resolves_pump() {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let mut attacking = ObjectFilter::creature();
    attacking.attacking = true;
    let condition = Condition::And(
        Box::new(Condition::SourceIsAttacking),
        Box::new(Condition::CountComparison {
            count: ironsmith_core::AnthemCountExpression::MatchingFilter(attacking),
            comparison: ironsmith_core::Comparison::Equal(1),
            display: None,
        }),
    );
    let source = attacker(&mut game, condition.clone());
    declare(&mut game, source);
    assert_eq!(game.stack.len(), 1);
    assert_eq!(game.stack[0].targets, vec![Target::Object(source)]);
    resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker).unwrap();
    assert_eq!(game.calculated_power(source), Some(3));
}

#[test]
fn ordinary_false_combat_condition_still_announces_target_and_resolves_no_pump() {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let source = attacker(&mut game, Condition::SourceIsBlocking);
    declare(&mut game, source);
    assert_eq!(game.stack.len(), 1);
    assert_eq!(game.stack[0].targets, vec![Target::Object(source)]);
    resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker).unwrap();
    assert_eq!(game.calculated_power(source), Some(2));
}

#[test]
fn ordinary_conditional_rechecks_combat_state_at_resolution() {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let source = attacker(&mut game, Condition::SourceIsAttacking);
    declare(&mut game, source);
    assert_eq!(game.stack[0].targets, vec![Target::Object(source)]);
    game.combat.as_mut().unwrap().attackers.clear();
    resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker).unwrap();
    assert_eq!(game.calculated_power(source), Some(2));
}

#[test]
fn nonmodal_coin_followup_announces_target_before_receipt_and_resolves_after_flip() {
    use ironsmith::effect::{EffectId, EffectMetric, EffectMetricSource, Value};
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let condition = Condition::ValueComparison {
        left: Value::EffectMetric {
            effect_id: EffectId(0),
            source: EffectMetricSource::Outcome,
            metric: EffectMetric::CoinFlipsWon,
        },
        operator: ironsmith_core::ValueComparisonOperator::Equal,
        right: Value::Fixed(1),
    };
    let source = attacker(&mut game, condition.clone());
    game.object_mut(source).unwrap().abilities = std::sync::Arc::new(vec![Ability::triggered(
        Trigger::this_attacks(),
        ResolutionProgram::from_effects(vec![
            Effect::with_id(0, Effect::flip_coin(ironsmith::target::PlayerFilter::You)),
            Effect::conditional_only(
                condition,
                vec![Effect::pump(
                    1,
                    1,
                    ChooseSpec::target_creature(),
                    Until::EndOfTurn,
                )],
            ),
        ]),
    )]);
    declare(&mut game, source);
    assert_eq!(game.stack.len(), 1);
    assert_eq!(game.stack[0].targets, vec![Target::Object(source)]);
    game.force_next_coin_flip(ironsmith::CoinFace::Heads);
    resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker).unwrap();
    assert_eq!(game.calculated_power(source), Some(3));
}

#[test]
fn ordinary_conditional_announces_both_target_specs_and_executes_only_selected_branch() {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let source = attacker(&mut game, Condition::SourceIsBlocking);
    game.object_mut(source).unwrap().abilities = std::sync::Arc::new(vec![Ability::triggered(
        Trigger::this_attacks(),
        ResolutionProgram::from_effects(vec![Effect::conditional(
            Condition::SourceIsBlocking,
            vec![Effect::pump(
                1,
                1,
                ChooseSpec::target_creature(),
                Until::EndOfTurn,
            )],
            vec![Effect::gain_life_player(2, ChooseSpec::target_player())],
        )]),
    )]);
    declare(&mut game, source);
    assert_eq!(
        game.stack[0].targets,
        vec![Target::Object(source), Target::Player(PlayerId(0))]
    );
    resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker).unwrap();
    assert_eq!(game.calculated_power(source), Some(2));
    assert_eq!(game.player(PlayerId(0)).unwrap().life, 22);
}
