use ironsmith::effects::{EffectContext, execute_effect};
use ironsmith::target::ChooseSpec;
use ironsmith::triggers::{TriggerEvent, check_triggers};
use ironsmith::{Effect, GameState, PlayerId, Zone};
use ironsmith_compiler_runtime::compile_to_runtime_definition;
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
const PRADUS: &str = "Type: Legendary Creature — Vedalken Advisor\nPower/Toughness: 3/5\nMenace, ward {1}\nWhenever you cast a spell, you may pay {1}. If you do, give this creature the bribed status until end of turn.\nAs long as President Pradus has the bribed status, other Advisors you control have menace and ward {1}.\nOther Advisors you control have \"Whenever you cast a spell, you may pay {1}. If you do, give this creature the bribed status until end of turn.\" and \"Whenever this creature with the bribed status attacks, draw a card unless an opponent pays {1}.\"";
const SCRIBE: &str = "Type: Creature — Lizard Advisor\nPower/Toughness: 4/2\nWhenever a creature you control gains the bribed status, you may return this card from your graveyard to your hand.";
fn compile(name: &str, text: &str) -> ironsmith::CardDefinition {
    ironsmith_compiler::word_claims::with_enforcement_options(true, true, || {
        compile_to_runtime_definition(name, text, false)
    })
    .unwrap_or_else(|error| panic!("{name}: {error}"))
}
fn apply(
    game: &mut GameState,
    id: ironsmith::ObjectId,
    name: &str,
    temporary: bool,
) -> Vec<TriggerEvent> {
    let effect = Effect::new(ironsmith_core::SetPermanentDesignationEffect {
        target: ChooseSpec::SpecificObject(id),
        designation: ironsmith_core::PermanentDesignation::CustomStatus {
            name: name.into(),
            until_end_of_turn: temporary,
        },
    });
    execute_effect(game, &effect, &mut EffectContext::new_default(id, A))
        .unwrap()
        .events
}
#[test]
fn explicit_status_syntax_compiles_with_complete_word_coverage() {
    for name in ["bribed", "moon blessed"] {
        compile(
            "Marker",
            &format!(
                "Type: Creature\nPower/Toughness: 2/2\n{{0}}: Give this creature the {name} status until end of turn.\nAs long as this creature has the {name} status, this creature gets +1/+1.\nWhenever this creature with the {name} status attacks, draw a card."
            ),
        );
        compile(
            "Banner",
            &format!("Type: Enchantment\nCreatures with the {name} status you control get +1/+1."),
        );
    }
    compile("President Pradus of Aevos City", PRADUS);
    let scribe = compile("Viashino Scribe", SCRIBE);
    let rendered = ironsmith_text::compiled_text_lines(&scribe).join("\n");
    assert!(rendered.contains("gains the bribed status"), "{rendered}");
}
#[test]
fn status_identity_lifetime_and_becomes_triggers_are_generic() {
    let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
    let body = compile("Body", "Type: Creature — Advisor\nPower/Toughness: 2/2");
    let scribe = compile("Viashino Scribe", SCRIBE);
    let watcher = game.create_object_from_definition(&scribe, A, Zone::Graveyard);
    game.create_object_from_definition(&scribe, A, Zone::Battlefield);
    game.create_object_from_definition(&scribe, A, Zone::Hand);
    let own = game.create_object_from_definition(&body, A, Zone::Battlefield);
    let opponent = game.create_object_from_definition(&body, B, Zone::Battlefield);
    assert!(
        apply(&mut game, opponent, "bribed", true)
            .iter()
            .all(|event| check_triggers(&game, event).is_empty())
    );
    let gained = apply(&mut game, own, "bribed", true);
    assert_eq!(
        gained
            .iter()
            .flat_map(|e| check_triggers(&game, e))
            .filter(|entry| entry.source == watcher)
            .count(),
        1
    );
    assert!(
        gained
            .iter()
            .flat_map(|event| check_triggers(&game, event))
            .all(|entry| entry.source == watcher)
    );
    assert!(apply(&mut game, own, "bribed", true).is_empty());
    assert!(
        apply(&mut game, own, "moon blessed", false)
            .iter()
            .all(|event| check_triggers(&game, event).is_empty()),
        "another status must not trigger bribed listeners"
    );
    let snapshot =
        ironsmith::snapshot::ObjectSnapshot::from_object(game.object(own).unwrap(), &game);
    game.set_current_controller(own, B).unwrap();
    assert!(game.has_custom_status(own, "bribed"));
    game.phase_out(own);
    assert!(game.has_custom_status(own, "bribed"));
    assert!(apply(&mut game, own, "another status", true).is_empty());
    ironsmith::turn::execute_cleanup_step(&mut game);
    game.phase_in(own);
    assert!(!game.has_custom_status(own, "bribed"));
    assert!(game.has_custom_status(own, "moon blessed"));
    assert!(snapshot.custom_statuses.contains(&"bribed".into()));
    assert_eq!(
        gained
            .iter()
            .flat_map(|event| check_triggers(&game, event))
            .count(),
        1,
        "the gained event keeps its original controller and status after cleanup"
    );
    assert!(!apply(&mut game, own, "bribed", true).is_empty());
    let copy_effect = Effect::new(ironsmith::effects::CreateTokenCopyEffect::new(
        ChooseSpec::SpecificObject(own),
        1,
        ironsmith::target::PlayerFilter::You,
    ));
    execute_effect(
        &mut game,
        &copy_effect,
        &mut EffectContext::new_default(own, B),
    )
    .unwrap();
    let copied = *game.battlefield.last().unwrap();
    assert_ne!(copied, own);
    assert!(game.custom_status_names(copied).is_empty());
    let hand = game
        .move_object(own, Zone::Hand, ironsmith::events::EventCause::effect())
        .unwrap();
    let returned = game
        .move_object(
            hand,
            Zone::Battlefield,
            ironsmith::events::EventCause::effect(),
        )
        .unwrap();
    assert!(game.custom_status_names(returned).is_empty());
    assert!(game.custom_status_names(own).is_empty());
}
#[test]
fn pradus_conditions_and_granted_attack_trigger_follow_each_recipients_status() {
    use ironsmith::static_abilities::StaticAbilityId;
    let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
    let pradus = compile("President Pradus of Aevos City", PRADUS);
    let body = compile("Advisor", "Type: Creature — Advisor\nPower/Toughness: 2/2");
    let source = game.create_object_from_definition(&pradus, A, Zone::Battlefield);
    let ally = game.create_object_from_definition(&body, A, Zone::Battlefield);
    let foe = game.create_object_from_definition(&body, B, Zone::Battlefield);
    assert!(!game.object_has_static_ability_id(ally, StaticAbilityId::Menace));
    apply(&mut game, source, "bribed", true);
    assert!(game.object_has_static_ability_id(ally, StaticAbilityId::Menace));
    assert!(!game.object_has_static_ability_id(foe, StaticAbilityId::Menace));
    let attack = TriggerEvent::new(
        ironsmith::events::CreatureAttackedEvent::new(
            ally,
            ironsmith::triggers::AttackEventTarget::Player(B),
        ),
        Default::default(),
    );
    assert!(check_triggers(&game, &attack).is_empty());
    apply(&mut game, ally, "bribed", true);
    assert_eq!(
        check_triggers(&game, &attack)
            .iter()
            .filter(|entry| entry.source == ally)
            .count(),
        1
    );
    let mut queue = ironsmith::triggers::TriggerQueue::new();
    for entry in check_triggers(&game, &attack) {
        queue.add(entry);
    }
    let mut answers = Answers(false);
    ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut answers)
        .unwrap();
    game.cleanup_custom_statuses_end_of_turn();
    assert!(!game.object_has_static_ability_id(ally, StaticAbilityId::Menace));
    assert!(check_triggers(&game, &attack).is_empty());
    // "Attacks while" is checked at the attack, not again on resolution.
    game.create_object_from_definition(&body, A, Zone::Library);
    ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut answers).unwrap();
    assert_eq!(game.player(A).unwrap().hand.len(), 1);
}

struct Answers(bool);
impl ironsmith::decision::DecisionMaker for Answers {
    fn decide_boolean(
        &mut self,
        _: &GameState,
        ctx: &ironsmith::decisions::context::BooleanContext,
    ) -> bool {
        self.0 && ctx.can_accept
    }
}

#[test]
fn bribe_payment_and_scribe_return_resolve_through_the_stack() {
    use ironsmith::game_loop::{put_triggers_on_stack_with_dm, resolve_stack_entry_with};
    use ironsmith::triggers::TriggerQueue;
    let pradus_definition = compile("President Pradus of Aevos City", PRADUS);
    let scribe_definition = compile("Viashino Scribe", SCRIBE);
    let spell_definition = compile("Spell", "Type: Instant\nDraw a card.");
    for accept in [false, true] {
        for can_pay in [false, true] {
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            let pradus =
                game.create_object_from_definition(&pradus_definition, A, Zone::Battlefield);
            let scribe = game.create_object_from_definition(&scribe_definition, A, Zone::Graveyard);
            let spell = game.create_object_from_definition(&spell_definition, A, Zone::Stack);
            if can_pay {
                game.player_mut(A)
                    .unwrap()
                    .mana_pool
                    .add(ironsmith::mana::ManaSymbol::Colorless, 1);
            }
            let event = TriggerEvent::new(
                ironsmith::events::SpellCastEvent::try_from_completed_cast(
                    spell,
                    A,
                    Zone::Hand,
                    &game,
                )
                .unwrap(),
                Default::default(),
            );
            let entries = check_triggers(&game, &event);
            assert_eq!(entries.len(), 1);
            assert_eq!(entries[0].source, pradus);
            let mut queue = TriggerQueue::new();
            for entry in entries {
                queue.add(entry);
            }
            let mut answers = Answers(accept);
            put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut answers).unwrap();
            resolve_stack_entry_with(&mut game, &mut answers).unwrap();
            assert_eq!(game.has_custom_status(pradus, "bribed"), accept && can_pay);
            ironsmith::game_loop::drain_pending_trigger_events(&mut game, &mut queue);
            put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut answers).unwrap();
            if accept && can_pay {
                assert_eq!(game.stack.len(), 1);
                resolve_stack_entry_with(&mut game, &mut answers).unwrap();
                assert!(game.object(scribe).is_none());
                assert!(
                    game.player(A)
                        .unwrap()
                        .hand
                        .iter()
                        .any(|id| game.object(*id).unwrap().name == "Viashino Scribe")
                );
            } else {
                assert!(game.stack.is_empty());
                assert_eq!(game.object(scribe).unwrap().zone, Zone::Graveyard);
            }
        }
    }
}

#[test]
fn status_reader_does_not_claim_uninterpreted_costs_or_durations() {
    for tail in ["until end of turn {2}", "until your next turn"] {
        let text = format!(
            "Type: Creature\nPower/Toughness: 2/2\n{{0}}: Give this creature the bribed status {tail}."
        );
        let result = ironsmith_compiler::word_claims::with_enforcement_options(true, true, || {
            compile_to_runtime_definition("Invalid status", &text, false)
        });
        assert!(
            result.is_err(),
            "uninterpreted status tail compiled: {tail}"
        );
    }
}

#[test]
fn gaining_one_status_is_distinct_from_already_having_another_required_status() {
    let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
    let watcher = compile(
        "Watcher",
        "Type: Enchantment\nWhenever a creature with the moon blessed status you control gains the bribed status, draw a card.",
    );
    game.create_object_from_definition(&watcher, A, Zone::Battlefield);
    let body = compile("Body", "Type: Creature\nPower/Toughness: 2/2");
    let source = game.create_object_from_definition(&body, A, Zone::Battlefield);
    for (name, temporary) in [("bribed", true), ("moon blessed", false)] {
        assert!(
            apply(&mut game, source, name, temporary)
                .iter()
                .all(|event| check_triggers(&game, event).is_empty())
        );
    }
    game.cleanup_custom_statuses_end_of_turn();
    let events = apply(&mut game, source, "bribed", true);
    assert_eq!(
        events
            .iter()
            .flat_map(|event| check_triggers(&game, event))
            .count(),
        1
    );
}

#[test]
fn custom_statuses_and_official_designations_do_not_alias_or_clear_each_other() {
    use ironsmith::events::{KeywordActionEvent, KeywordActionKind};
    use ironsmith::object::CounterType;
    let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
    let mut body = compile(
        "Witness",
        "Type: Creature\nPower/Toughness: 2/2\nWhenever this creature gains the bribed status, draw a card.\nWhenever this creature becomes monstrous, draw a card.",
    );
    body.abilities.push(ironsmith::ability::Ability::triggered(
        ironsmith::triggers::Trigger::keyword_action_matching_object(KeywordActionKind::Renown,
            ironsmith::target::PlayerFilter::Any, ironsmith::target::ObjectFilter { source: true, ..Default::default() }),
        vec![Effect::gain_life(1)]));
    let source = game.create_object_from_definition(&body, A, Zone::Battlefield);
    let gained = apply(&mut game, source, "bribed", true);
    assert_eq!(
        gained
            .iter()
            .flat_map(|event| check_triggers(&game, event))
            .count(),
        1
    );
    assert!(gained.iter().all(|event| {
        event
            .downcast::<KeywordActionEvent>()
            .is_some_and(|action| action.action == KeywordActionKind::CustomStatusGained)
    }));
    assert!(!game.is_monstrous(source));
    assert!(!game.is_renowned(source));
    assert!(!game.is_harnessed(source));
    for effect in [
        Effect::new(ironsmith::effects::MonstrosityEffect::new(2)),
        Effect::new(ironsmith::effects::RenownEffect::new(1)),
    ] {
        let outcome = execute_effect(
            &mut game,
            &effect,
            &mut EffectContext::new_default(source, A),
        )
        .unwrap();
        assert_eq!(
            outcome
                .events
                .iter()
                .flat_map(|event| check_triggers(&game, event))
                .count(),
            1
        );
        assert!(outcome.events.iter().all(|event| {
            !event
                .downcast::<KeywordActionEvent>()
                .is_some_and(|action| action.action == KeywordActionKind::CustomStatusGained)
        }));
    }
    execute_effect(
        &mut game,
        &Effect::new(ironsmith_core::SetPermanentDesignationEffect {
            target: ChooseSpec::Source,
            designation: ironsmith_core::PermanentDesignation::Harnessed,
        }),
        &mut EffectContext::new_default(source, A),
    )
    .unwrap();
    ironsmith::turn::execute_cleanup_step(&mut game);
    assert!(!game.has_custom_status(source, "bribed"));
    assert!(game.is_monstrous(source));
    assert!(game.is_renowned(source));
    assert!(game.is_harnessed(source));
    assert_eq!(game.counter_count(source, CounterType::PlusOnePlusOne), 3);
}
