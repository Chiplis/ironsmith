use super::shard_16::parse_oracle_card_definition;
use super::*;

const CHAIN_CASES: [(&str, &str); 5] = [
    (
        "Chain Lightning",
        "Chain Lightning deals 3 damage to any target. Then that player or that permanent's controller may pay {R}{R}. If the player does, they may copy this spell and may choose a new target for that copy.",
    ),
    (
        "Chain Stasis",
        "You may tap or untap target creature. Then that creature's controller may pay {2}{U}. If the player does, they may copy this spell and may choose a new target for that copy.",
    ),
    (
        "Chain of Plasma",
        "Chain of Plasma deals 3 damage to any target. Then that player or that permanent's controller may discard a card. If the player does, they may copy this spell and may choose a new target for that copy.",
    ),
    (
        "Chain of Vapor",
        "Return target nonland permanent to its owner's hand. Then that permanent's controller may sacrifice a land of their choice. If the player does, they may copy this spell and may choose a new target for that copy.",
    ),
    (
        "String of Disappearances",
        "Return target creature to its owner's hand. Then that creature's controller may pay {U}{U}. If the player does, they may copy this spell and may choose a new target for that copy.",
    ),
];

fn same_chain_actor(left: &PlayerFilter, right: &PlayerFilter) -> bool {
    left == right
        || matches!(
            (left, right),
            (
                PlayerFilter::ControllerOf(left_ref),
                PlayerFilter::AliasedControllerOf(right_ref),
            ) | (
                PlayerFilter::AliasedControllerOf(left_ref),
                PlayerFilter::ControllerOf(right_ref),
            ) if left_ref == right_ref
        )
}

fn enabling_action_selects_object_controlled_by(
    effect: &Effect,
    tag: &crate::TagKey,
    actor: &PlayerFilter,
) -> bool {
    if let Some(choice) = effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()
        && &choice.tag == tag
        && same_chain_actor(&choice.chooser, actor)
        && choice
            .filter
            .controller
            .as_ref()
            .is_some_and(|controller| same_chain_actor(controller, actor))
    {
        return true;
    }

    let mut found = false;
    effect.visit_child_effects(&mut |child| {
        if !found {
            found = enabling_action_selects_object_controlled_by(child, tag, actor);
        }
    });
    found
}

fn copy_actor_matches_enabling_action(
    actor: &PlayerFilter,
    enabling_actor: &PlayerFilter,
    enabling_effects: &[Effect],
) -> bool {
    if same_chain_actor(actor, enabling_actor) {
        return true;
    }
    let tag = match actor {
        PlayerFilter::ControllerOf(crate::filter::ObjectRef::Tagged(tag))
        | PlayerFilter::AliasedControllerOf(crate::filter::ObjectRef::Tagged(tag)) => tag,
        _ => return false,
    };
    enabling_effects
        .iter()
        .any(|effect| enabling_action_selects_object_controlled_by(effect, tag, enabling_actor))
}

fn chain_continuation(
    definition: &crate::cards::CardDefinition,
) -> (
    &crate::effects::MayEffect,
    &crate::effects::CopySpellEffect,
    &crate::effects::ChooseNewTargetsEffect,
) {
    let effects = definition
        .spell_effect
        .as_ref()
        .expect("chain card should be a spell")
        .flattened_default_effects();
    let [.., enabling_effect, conditional_effect] = effects else {
        panic!("chain card should have an antecedent and two-effect continuation: {effects:#?}");
    };
    let enabling_effect = if let Some(sequence) =
        enabling_effect.downcast_ref::<crate::effects::SequenceEffect>()
        && let [only] = sequence.effects.as_slice()
    {
        assert_eq!(
            sequence.surface,
            ironsmith_core::SequenceSurface::SentenceLeadingThen,
            "a wrapped enabling choice should retain its authored sentence boundary"
        );
        only
    } else {
        enabling_effect
    };
    let enabling_with_id = enabling_effect
        .downcast_ref::<WithIdEffect>()
        .expect("enabling choice should have a result id");
    let enabling_may = enabling_with_id
        .effect
        .downcast_ref::<crate::effects::MayEffect>()
        .expect("enabling action should be optional");
    let conditional = conditional_effect
        .downcast_ref::<IfEffect>()
        .expect("copy continuation should depend on the enabling result");
    assert_eq!(conditional.condition, enabling_with_id.id);
    assert_eq!(
        conditional.predicate,
        crate::effect::EffectPredicate::Happened
    );
    assert!(conditional.else_.is_empty());

    let [copy_may_effect] = conditional.then.as_slice() else {
        panic!("successful enabling action should offer one copy bundle: {conditional:#?}");
    };
    let copy_may = copy_may_effect
        .downcast_ref::<crate::effects::MayEffect>()
        .expect("copy should remain optional");
    let [copy_effect, retarget_effect] = copy_may.effects.as_slice() else {
        panic!("copy choice should contain copy and retarget effects: {copy_may:#?}");
    };
    let tagged = copy_effect
        .downcast_ref::<TaggedEffect>()
        .expect("copy result should be tagged");
    let copy_with_id = tagged
        .effect
        .downcast_ref::<WithIdEffect>()
        .expect("copy should expose its result to retargeting");
    let copy = copy_with_id
        .effect
        .downcast_ref::<crate::effects::CopySpellEffect>()
        .expect("copy bundle should copy the resolving spell");
    let retarget = retarget_effect
        .downcast_ref::<crate::effects::ChooseNewTargetsEffect>()
        .expect("copy bundle should carry typed retargeting");
    assert_eq!(retarget.from_effect, copy_with_id.id);
    assert!(retarget.may);
    assert!(retarget.single_target_surface);
    let enabling_decider = enabling_may
        .decider
        .as_ref()
        .expect("enabling action should have an explicit actor");
    let copy_decider = copy_may
        .decider
        .as_ref()
        .expect("copy choice should have an explicit actor");
    let retarget_chooser = retarget
        .chooser
        .as_ref()
        .expect("retargeting should have an explicit actor");
    assert!(copy_actor_matches_enabling_action(
        copy_decider,
        enabling_decider,
        &enabling_may.effects,
    ));
    assert!(same_chain_actor(retarget_chooser, copy_decider));
    assert!(same_chain_actor(copy_decider, &copy.copier));
    (enabling_may, copy, retarget)
}

#[test]
pub(super) fn chain_copy_cards_keep_cost_actor_copy_and_single_retarget_structure() {
    for (name, _) in CHAIN_CASES {
        let definition = parse_oracle_card_definition(name);
        let (enabling_may, copy, _) = chain_continuation(&definition);
        assert!(
            enabling_may.decider.is_some(),
            "{name} needs an explicit enabling actor"
        );
        assert!(matches!(copy.target.unhinted(), ChooseSpec::Source));
        assert_eq!(copy.count, crate::effect::Value::Fixed(1));

        let enabling_debug = format!("{:#?}", enabling_may.effects);
        match name {
            "Chain Lightning" | "Chain Stasis" | "String of Disappearances" => {
                assert!(
                    enabling_debug.contains("PayManaEffect"),
                    "{name}: {enabling_debug}"
                );
            }
            "Chain of Plasma" => {
                assert!(
                    enabling_debug.contains("DiscardEffect"),
                    "{name}: {enabling_debug}"
                );
            }
            "Chain of Vapor" => {
                assert!(
                    enabling_debug.contains("Sacrifice"),
                    "{name}: {enabling_debug}"
                );
            }
            _ => unreachable!(),
        }
    }
}

#[test]
pub(super) fn chain_copy_cards_round_trip_the_oracle_chain_surface() {
    for (name, oracle) in CHAIN_CASES {
        let definition = parse_oracle_card_definition(name);
        assert_eq!(
            canonical_compiled_lines(&definition),
            vec![oracle.to_string()],
            "{name} should preserve the result actor and copy-retarget bundle"
        );
    }
}
