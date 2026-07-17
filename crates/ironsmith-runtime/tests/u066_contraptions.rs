use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::decision::DecisionMaker;
use ironsmith::events::EventContext;
use ironsmith::events::{KeywordActionEvent, KeywordActionKind};
use ironsmith::replacement::ReplacementAction;
use ironsmith::static_abilities::StaticAbilityId;
use ironsmith::triggers::Trigger;
use ironsmith::{
    Ability, AbilityKind, CardBuilder, CardDefinition, CardId, CardType, Effect, EffectContext,
    ExecutionError, GameState, PlayerFilter, PlayerId, Subtype, TriggerEvent, Zone, check_triggers,
    execute_effect,
};

struct TestDecisionMaker;
impl DecisionMaker for TestDecisionMaker {}

fn card(name: &str, abilities: Vec<Ability>) -> CardDefinition {
    let mut definition = CardDefinition::new(
        CardBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Creature])
            .build(),
    );
    definition.abilities = abilities;
    definition
}

#[test]
fn u066_assemble_is_a_typed_action_that_renders_without_claiming_external_rules() {
    let definition = CardDefinitionBuilder::new(CardId::new(), "Assembly Instruction")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Assemble a Contraption.")
        .expect("the CR-defined action boundary should compile to a typed action");
    let program = definition.spell_effect.as_ref().expect("spell effect");
    let [effect] = program.segments[0].default_effects.as_slice() else {
        panic!("expected one assemble action");
    };
    let emit = effect
        .downcast_ref::<ironsmith::effects::EmitKeywordActionEffect>()
        .expect("typed keyword action");
    assert_eq!(emit.action, KeywordActionKind::AssembleContraption);
    assert_eq!(emit.amount, 1);
    assert_eq!(
        ironsmith::compiled_text::compiled_text_lines(&definition).join("\n"),
        "Assemble a Contraption."
    );

    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = PlayerId::from_index(0);
    let source = game.create_object_from_definition(&definition, alice, Zone::Stack);
    let mut decisions = TestDecisionMaker;
    let mut ctx = EffectContext::new_default(source, alice).with_decision_maker(&mut decisions);
    let error = execute_effect(&mut game, effect, &mut ctx)
        .expect_err("CR-only play must not invent the external Unstable procedure");
    assert_eq!(
        error,
        ExecutionError::ExternalRulesProfileRequired {
            action: "assembling a Contraption",
            specification: "the Unstable FAQ",
        }
    );
    assert!(game.battlefield.is_empty());
}

#[test]
fn u066_external_profile_can_publish_typed_assemble_events_to_normal_triggers() {
    let alice = PlayerId::from_index(0);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let observer = card(
        "Assembly Observer",
        vec![Ability::triggered(
            Trigger::keyword_action(KeywordActionKind::AssembleContraption, PlayerFilter::You),
            vec![Effect::gain_life(1)],
        )],
    );
    let source = game.create_object_from_definition(&observer, alice, Zone::Battlefield);
    let external_event = TriggerEvent::new(
        KeywordActionEvent::new(KeywordActionKind::AssembleContraption, alice, source, 1),
        ironsmith::provenance::ProvNodeId::default(),
    );

    let entries = check_triggers(&game, &external_event);
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].source, source);
    assert_eq!(
        KeywordActionKind::AssembleContraption.infinitive(),
        "assemble a Contraption"
    );
    assert_eq!(
        KeywordActionKind::AssembleContraption.third_person(),
        "assembles a Contraption"
    );
}

#[test]
fn u066_steamflogger_uses_the_generic_typed_keyword_action_replacement() {
    let definition = CardDefinitionBuilder::new(CardId::new(), "Steamflogger Boss")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Goblin, Subtype::Rigger])
        .parse_text(
            "Other Riggers you control get +1/+0 and have haste.\n\
             If a Rigger you control would assemble a Contraption, it assembles two Contraptions instead.",
        )
        .expect("Steamflogger Boss should compile without a fallback");

    let ability = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Static(ability)
                if ability.id() == StaticAbilityId::KeywordActionReplacement =>
            {
                Some(ability)
            }
            _ => None,
        })
        .expect("typed keyword-action replacement static ability");

    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let boss = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let rigger = card("Friendly Rigger", vec![]);
    let mut rigger = rigger;
    rigger.card.subtypes = vec![Subtype::Rigger];
    let friendly_rigger = game.create_object_from_definition(&rigger, alice, Zone::Battlefield);
    let opposing_rigger = game.create_object_from_definition(&rigger, bob, Zone::Battlefield);

    let replacement = ability
        .generate_replacement_effect(boss, alice)
        .expect("static ability should generate a replacement");
    let ReplacementAction::Instead(effects) = &replacement.replacement else {
        panic!("Steamflogger should replace the original action with typed effects");
    };
    let [effect] = effects.as_slice() else {
        panic!("Steamflogger should have one replacement instruction");
    };
    let replacement_action = effect
        .downcast_ref::<ironsmith::effects::EmitKeywordActionEffect>()
        .expect("replacement instruction should stay typed");
    assert_eq!(
        replacement_action.action,
        KeywordActionKind::AssembleContraption
    );
    assert_eq!(replacement_action.amount, 2);

    let matcher = replacement.matcher.as_ref().expect("typed matcher");
    let event_context = EventContext::for_replacement_effect(alice, boss, &game);
    assert!(matcher.matches_event(
        &KeywordActionEvent::new(
            KeywordActionKind::AssembleContraption,
            alice,
            friendly_rigger,
            1,
        ),
        &event_context,
    ));
    assert!(!matcher.matches_event(
        &KeywordActionEvent::new(
            KeywordActionKind::AssembleContraption,
            bob,
            opposing_rigger,
            1,
        ),
        &event_context,
    ));
}
