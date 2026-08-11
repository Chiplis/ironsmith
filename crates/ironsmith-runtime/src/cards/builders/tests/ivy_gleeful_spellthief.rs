#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

const ORACLE: &str = "Flying\nWhenever a player casts a spell that targets only a single creature other than Ivy, you may copy that spell. The copy targets Ivy.";

fn ivy_trigger(
    definition: &CardDefinition,
) -> (
    &crate::ability::TriggeredAbility,
    &crate::triggers::SpellCastTrigger,
) {
    definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => triggered
                .trigger
                .downcast_ref::<crate::triggers::SpellCastTrigger>()
                .map(|cast| (triggered, cast)),
            _ => None,
        })
        .expect("Ivy should retain her spell-cast trigger")
}

#[test]
fn ivy_keeps_source_exclusion_and_fixed_retarget_inside_optional_copy() {
    let definition = parse_oracle_card_definition("Ivy, Gleeful Spellthief");
    assert_eq!(canonical_compiled_lines(&definition).join("\n"), ORACLE);

    let (triggered, cast) = ivy_trigger(&definition);
    assert_eq!(cast.caster, PlayerFilter::Any);
    let spell = cast
        .filter
        .as_ref()
        .expect("Ivy should restrict the triggering spell's target set");
    assert_eq!(
        spell.target_count,
        Some(crate::effect::ChoiceCount::exactly(1))
    );
    let sole_target = spell
        .targets_only_object
        .as_deref()
        .expect("the spell should target only a creature");
    assert_eq!(sole_target.card_types, [CardType::Creature]);
    assert!(sole_target.other, "{sole_target:#?}");
    assert_eq!(
        sole_target.source_surface,
        Some(SourceReferenceSurface::ShortName("Ivy".to_string()))
    );

    let effects = triggered.effects.flattened_default_effects();
    let may = effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<crate::effects::MayEffect>())
        .expect("copying should remain optional");
    let [copy_effect, retarget_effect] = may.effects.as_slice() else {
        panic!("the optional branch should own copy then fixed retarget: {may:#?}");
    };
    let tagged_copy = copy_effect
        .downcast_ref::<crate::effects::TaggedEffect>()
        .expect("the copied spell should retain a durable result tag");
    let copy = tagged_copy
        .effect
        .downcast_ref::<crate::effects::WithIdEffect>()
        .and_then(|with_id| {
            with_id
                .effect
                .downcast_ref::<crate::effects::CopySpellEffect>()
        })
        .expect("the optional branch should copy the triggering spell");
    assert_eq!(
        copy.target_reference_kind,
        Some(crate::filter::StackObjectKind::Spell)
    );
    let retarget = retarget_effect
        .downcast_ref::<crate::effects::RetargetStackObjectEffect>()
        .expect("the optional branch should assign the copy's fixed target");
    assert!(matches!(
        retarget.target.base(),
        ChooseSpec::Tagged(tag) if tag == &tagged_copy.tag
    ));
    assert!(matches!(
        &retarget.mode,
        crate::effects::RetargetMode::OneToFixed(fixed)
            if matches!(fixed.base(), ChooseSpec::Source)
    ));
}

fn setup_copy_scenario() -> (
    crate::game_state::GameState,
    CardDefinition,
    PlayerId,
    ObjectId,
    ObjectId,
    ObjectId,
) {
    let definition = parse_oracle_card_definition("Ivy, Gleeful Spellthief");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let ivy = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let other = game.create_object_from_definition(
        &CardDefinitionBuilder::new(CardId::from_raw(98_410), "Other Creature")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build(),
        alice,
        Zone::Battlefield,
    );
    let target_spec = ChooseSpec::target_creature();
    let spell_definition = CardDefinitionBuilder::new(CardId::from_raw(98_411), "Ivy Copy Probe")
        .card_types(vec![CardType::Instant])
        .with_spell_effect(vec![Effect::deal_damage(1, target_spec.clone())])
        .build();
    let spell = game.create_object_from_definition(&spell_definition, alice, Zone::Stack);
    game.push_to_stack(
        crate::game_state::StackEntry::new(spell, alice)
            .with_targets(vec![crate::game_state::Target::Object(other)])
            .with_target_assignments(vec![crate::game_state::TargetAssignment {
                spec: target_spec,
                range: 0..1,
            }]),
    );
    (game, definition, alice, ivy, other, spell)
}

#[test]
fn ivy_triggers_only_for_another_creature_and_retargets_only_an_accepted_copy() {
    let (mut game, definition, alice, ivy, other, spell) = setup_copy_scenario();
    let (triggered, _) = ivy_trigger(&definition);
    let event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::spells::SpellCastEvent::new(spell, alice, Zone::Hand),
        crate::provenance::ProvNodeId::default(),
    );
    assert!(triggered.trigger.matches(
        &event,
        &crate::triggers::TriggerContext::for_source(ivy, alice, &game),
    ));

    game.stack
        .last_mut()
        .expect("original spell should remain on stack")
        .targets = vec![crate::game_state::Target::Object(ivy)];
    assert!(
        !triggered.trigger.matches(
            &event,
            &crate::triggers::TriggerContext::for_source(ivy, alice, &game),
        ),
        "a spell targeting Ivy herself must not trigger the ability"
    );
    game.stack
        .last_mut()
        .expect("original spell should remain on stack")
        .targets = vec![crate::game_state::Target::Object(other)];

    let mut accept = crate::decision::SelectFirstDecisionMaker;
    let mut accept_ctx =
        crate::effects::ExecutionContext::new(ivy, alice, &mut accept).with_triggering_event(event);
    crate::game_loop::execute_resolution_program(
        &mut game,
        &mut accept_ctx,
        alice,
        ivy,
        &triggered.effects,
        None,
        &[],
    )
    .expect("accepting Ivy's copy should resolve");
    assert_eq!(game.stack.len(), 2);
    let copied = game
        .stack
        .iter()
        .find(|entry| entry.object_id != spell)
        .expect("accepting should create one spell copy");
    assert_eq!(
        copied.targets,
        vec![crate::game_state::Target::Object(ivy)],
        "the copy, not the original, must target Ivy"
    );

    let (mut decline_game, decline_definition, alice, ivy, _other, spell) = setup_copy_scenario();
    let (decline_triggered, _) = ivy_trigger(&decline_definition);
    let decline_event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::spells::SpellCastEvent::new(spell, alice, Zone::Hand),
        crate::provenance::ProvNodeId::default(),
    );
    let mut decline = crate::decision::AutoPassDecisionMaker;
    let mut decline_ctx = crate::effects::ExecutionContext::new(ivy, alice, &mut decline)
        .with_triggering_event(decline_event);
    crate::game_loop::execute_resolution_program(
        &mut decline_game,
        &mut decline_ctx,
        alice,
        ivy,
        &decline_triggered.effects,
        None,
        &[],
    )
    .expect("declining Ivy's copy must not execute an orphan retarget");
    assert_eq!(decline_game.stack.len(), 1);
    assert_eq!(decline_game.stack[0].object_id, spell);
}
