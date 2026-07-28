#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

const FRONTLINE_HEROISM_ORACLE: [&str; 2] = [
    "When this enchantment enters, create a 1/1 red Soldier creature token with haste.",
    "Whenever you cast a spell that targets only a single creature you control, create a 1/1 red Soldier creature token with haste, then copy that spell. The copy targets that token.",
];

fn frontline_spell_trigger(
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
                .map(|spell_cast| (triggered, spell_cast)),
            _ => None,
        })
        .expect("Frontline Heroism must retain its spell-cast trigger")
}

#[test]
fn frontline_heroism_renders_the_complete_token_copy_retarget_program() {
    let definition = parse_oracle_card_definition("Frontline Heroism");
    assert_eq!(
        canonical_compiled_lines(&definition),
        FRONTLINE_HEROISM_ORACLE.map(str::to_string)
    );
}

#[test]
fn frontline_heroism_keeps_both_exact_result_sets_and_comma_then_surface() {
    let definition = parse_oracle_card_definition("Frontline Heroism");
    let (triggered, cast) = frontline_spell_trigger(&definition);
    let filter = cast
        .filter
        .as_ref()
        .expect("the spell-cast trigger must retain its target restriction");
    assert_eq!(cast.caster, PlayerFilter::You);
    assert_eq!(
        filter.target_count,
        Some(crate::effect::ChoiceCount::exactly(1))
    );
    let only_target = filter
        .targets_only_object
        .as_ref()
        .expect("the triggering spell must target only one controlled creature");
    assert_eq!(only_target.card_types, vec![CardType::Creature]);
    assert_eq!(only_target.controller, Some(PlayerFilter::You));

    let [create_copy_segment, retarget_segment] = triggered.effects.segments.as_slice() else {
        panic!(
            "the authored two sentences must remain two resolution segments: {:#?}",
            triggered.effects
        );
    };
    let [trigger_tag, sequence_effect] = create_copy_segment.default_effects.as_slice() else {
        panic!(
            "the first segment must tag the spell and execute create/copy: {create_copy_segment:#?}"
        );
    };
    assert!(
        trigger_tag
            .downcast_ref::<crate::effects::TagTriggeringObjectEffect>()
            .is_some(),
        "the triggering spell must be available to the copy action: {trigger_tag:#?}"
    );
    let sequence = sequence_effect
        .downcast_ref::<crate::effects::SequenceEffect>()
        .expect("create and copy must share one typed authored sequence");
    assert_eq!(sequence.surface, ironsmith_core::SequenceSurface::CommaThen);
    let [create_effect, copy_effect] = sequence.effects.as_slice() else {
        panic!("the comma-then sequence must contain exactly create then copy: {sequence:#?}");
    };

    let tagged_create = create_effect
        .downcast_ref::<crate::effects::TaggedEffect>()
        .expect("the created token result must have a durable tag");
    let create = tagged_create
        .effect
        .downcast_ref::<crate::effects::CreateTokenEffect>()
        .expect("the tagged producer must be token creation");
    assert_eq!(create.count, crate::effect::Value::Fixed(1));
    assert_eq!(create.controller, PlayerFilter::You);

    let tagged_copy = copy_effect
        .downcast_ref::<crate::effects::TaggedEffect>()
        .expect("the copied spell result must have a durable tag");
    let copy_with_id = tagged_copy
        .effect
        .downcast_ref::<crate::effects::WithIdEffect>()
        .expect("the copy result must retain its effect identity");
    let copy = copy_with_id
        .effect
        .downcast_ref::<crate::effects::CopySpellEffect>()
        .expect("the second authored action must copy the triggering spell");
    assert_eq!(copy.count, crate::effect::Value::Fixed(1));
    assert_eq!(copy.copier, PlayerFilter::You);

    let [retarget_effect] = retarget_segment.default_effects.as_slice() else {
        panic!("the second sentence must contain only the linked retarget: {retarget_segment:#?}");
    };
    let retarget = retarget_effect
        .downcast_ref::<crate::effects::RetargetStackObjectEffect>()
        .expect("the second sentence must retarget the copy");
    assert!(
        matches!(&retarget.target, ChooseSpec::Tagged(tag) if tag == &tagged_copy.tag),
        "retargeting must consume the exact copied-spell result: {retarget:#?}"
    );
    assert!(
        matches!(
            &retarget.mode,
            crate::effects::RetargetMode::OneToFixed(ChooseSpec::Tagged(tag))
                if tag == &tagged_create.tag
        ),
        "the copy's fixed target must be the exact created-token result: {retarget:#?}"
    );
}

#[test]
fn frontline_heroism_creates_a_token_and_retargets_only_the_copy_to_it() {
    let definition = parse_oracle_card_definition("Frontline Heroism");
    let (triggered, _) = frontline_spell_trigger(&definition);
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let creature = game.create_object_from_definition(
        &CardDefinitionBuilder::new(CardId::from_raw(96_300), "Original Target")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(3, 3))
            .build(),
        alice,
        Zone::Battlefield,
    );
    let target_spec = ChooseSpec::target_creature();
    let spell_definition = CardDefinitionBuilder::new(CardId::from_raw(96_301), "Heroic Spark")
        .card_types(vec![CardType::Instant])
        .with_spell_effect(vec![Effect::deal_damage(1, target_spec.clone())])
        .build();
    let spell = game.create_object_from_definition(&spell_definition, alice, Zone::Stack);
    game.push_to_stack(
        crate::game_state::StackEntry::new(spell, alice)
            .with_targets(vec![crate::game_state::Target::Object(creature)])
            .with_target_assignments(vec![crate::game_state::TargetAssignment {
                spec: target_spec,
                range: 0..1,
            }]),
    );

    let event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::spells::SpellCastEvent::new(spell, alice, Zone::Hand),
        crate::provenance::ProvNodeId::default(),
    );
    assert!(
        triggered.trigger.matches(
            &event,
            &crate::triggers::TriggerContext::for_source(source, alice, &game),
        ),
        "the exact single controlled-creature target must satisfy the trigger"
    );
    let mut ctx =
        crate::effects::ExecutionContext::new_default(source, alice).with_triggering_event(event);
    for effect in &triggered.effects {
        crate::effects::execute_effect(&mut game, effect, &mut ctx)
            .expect("Frontline Heroism's complete trigger must resolve");
    }

    let token = game
        .battlefield
        .iter()
        .copied()
        .find(|id| {
            *id != source
                && *id != creature
                && game.object(*id).is_some_and(|object| {
                    object.name == "Soldier"
                        && object.card_types.contains(&CardType::Creature)
                        && object.subtypes.contains(&Subtype::Soldier)
                })
        })
        .expect("the trigger must create its Soldier token");
    assert_eq!(
        (
            game.calculated_power(token),
            game.calculated_toughness(token)
        ),
        (Some(1), Some(1))
    );
    assert!(game.current_has_static_ability_id(token, StaticAbilityId::Haste));

    assert_eq!(
        game.stack.len(),
        2,
        "the original and one copy must be on stack"
    );
    let original = game
        .stack
        .iter()
        .find(|entry| entry.object_id == spell)
        .expect("the original spell remains on the stack");
    assert_eq!(
        original.targets,
        vec![crate::game_state::Target::Object(creature)]
    );
    let copied = game
        .stack
        .iter()
        .find(|entry| entry.object_id != spell)
        .expect("the trigger must create one spell copy");
    assert_eq!(
        copied.targets,
        vec![crate::game_state::Target::Object(token)],
        "only the copy must be retargeted to the newly created token"
    );
}
