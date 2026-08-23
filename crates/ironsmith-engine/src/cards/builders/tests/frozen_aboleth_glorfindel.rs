#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

#[test]
fn aboleth_spawn_keeps_the_entry_caused_source_qualified_ability_trigger() {
    let definition = parse_oracle_card_definition("Aboleth Spawn");
    assert_eq!(
        canonical_compiled_lines(&definition),
        vec![
            "Flash".to_string(),
            "Ward {2}".to_string(),
            "Probing Telepathy — Whenever a creature entering under an opponent's control causes a triggered ability of that creature to trigger, you may copy that ability. You may choose new targets for the copy."
                .to_string(),
        ]
    );

    let triggered = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered)
                if triggered
                    .presentation_label
                    .as_ref()
                    .and_then(crate::ability::PresentationLabel::display_prefix)
                    .is_some_and(|label| label == "Probing Telepathy") =>
            {
                Some(triggered)
            }
            _ => None,
        })
        .expect("Aboleth Spawn should retain its labeled trigger");
    let matcher = triggered
        .trigger
        .downcast_ref::<crate::triggers::spell_ability::AbilityTriggeredTrigger>()
        .expect("the trigger should use the typed ability-trigger matcher");
    assert!(matcher.caused_by_source_entering);
    assert_eq!(
        matcher
            .source_filter
            .as_ref()
            .and_then(|filter| filter.controller.clone()),
        Some(PlayerFilter::Opponent)
    );
    let effects = triggered.effects.flattened_default_effects();
    let [tag_triggering_source, outer_may] = effects else {
        panic!("expected source provenance plus one optional copy program: {effects:#?}");
    };
    let triggering_tag = &tag_triggering_source
        .downcast_ref::<crate::effects::TagTriggeringSourceEffect>()
        .expect("the qualified trigger should tag its triggering source")
        .tag;
    let outer_may = outer_may
        .downcast_ref::<crate::effects::MayEffect>()
        .expect("copying the triggered ability should be optional");
    assert_eq!(outer_may.decider, Some(PlayerFilter::You));
    let [copy_effect, retarget_may] = outer_may.effects.as_slice() else {
        panic!("the accepted branch should copy, then offer retargeting: {outer_may:#?}");
    };
    let tagged_copy = copy_effect
        .downcast_ref::<crate::effects::TaggedEffect>()
        .expect("the ability copy should retain its result tag");
    let with_id = tagged_copy
        .effect
        .downcast_ref::<crate::effects::WithIdEffect>()
        .expect("the ability copy should retain its result ID");
    let copy = with_id
        .effect
        .downcast_ref::<crate::effects::CopySpellEffect>()
        .expect("the optional branch should use the typed stack-copy effect");
    assert_eq!(copy.target, ChooseSpec::Tagged(triggering_tag.clone()));
    assert_eq!(
        copy.target_reference_kind,
        Some(crate::filter::StackObjectKind::Ability)
    );
    let retarget_may = retarget_may
        .downcast_ref::<crate::effects::MayEffect>()
        .expect("choosing new targets should remain an independently optional nested choice");
    assert_eq!(retarget_may.decider, Some(PlayerFilter::You));
    let [retarget_effect] = retarget_may.effects.as_slice() else {
        panic!("the nested choice should contain exactly one retarget action: {retarget_may:#?}");
    };
    let retarget = retarget_effect
        .downcast_ref::<crate::effects::RetargetStackObjectEffect>()
        .expect("the nested choice should retarget the exact copy");
    assert_eq!(retarget.target, ChooseSpec::Tagged(tagged_copy.tag.clone()));
    assert!(matches!(retarget.mode, crate::effects::RetargetMode::All));
    assert_eq!(retarget.chooser, PlayerFilter::You);
}

#[test]
fn glorfindel_modal_common_pump_is_typed_once_before_the_selected_mode() {
    let definition = parse_oracle_card_definition("Glorfindel, Dauntless Rescuer");
    let expected = "Whenever you scry, choose one and Glorfindel gets +1/+1 until end of turn.\n\
• Glorfindel must be blocked this turn if able.\n\
• Glorfindel can't be blocked by more than one creature each combat this turn.";
    assert_eq!(canonical_compiled_lines(&definition).join("\n"), expected);

    let triggered = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Glorfindel should have a scry trigger");
    let modal = triggered
        .effects
        .flattened_default_effects()
        .iter()
        .find_map(|effect| effect.downcast_ref::<crate::effects::ChooseModeEffect>())
        .expect("Glorfindel's trigger should contain one typed modal choice");
    assert_eq!(modal.common_prefix_effects.len(), 1);
    assert_eq!(modal.modes.len(), 2);
    let common_debug = format!("{:#?}", modal.common_prefix_effects);
    assert!(
        common_debug.contains("ApplyContinuousEffect")
            && common_debug.contains("ModifyPowerToughness"),
        "the common pump must remain an executable continuous effect: {common_debug}"
    );
    assert!(
        common_debug.contains("Source"),
        "the common pump must still target the ability's source: {common_debug}"
    );
    let modes_debug = format!("{:#?}", modal.modes);
    assert!(
        !modes_debug.contains("ModifyPowerToughness"),
        "the common pump must not be duplicated into individual modes: {modes_debug}"
    );
}
