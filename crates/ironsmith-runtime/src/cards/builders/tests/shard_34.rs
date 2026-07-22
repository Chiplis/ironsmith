use super::shard_16::{assert_oracle_card_parses_strict, parse_oracle_card_definition};
use super::*;

fn unwrap_quantified_collection_wrapper(effect: &crate::effect::Effect) -> &crate::effect::Effect {
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        return unwrap_quantified_collection_wrapper(&tagged.effect);
    }
    if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
        return unwrap_quantified_collection_wrapper(&with_id.effect);
    }
    effect
}

fn assert_meteor_chosen_collection_stays_inside_conditional(
    definition: &crate::cards::CardDefinition,
) {
    let spell = definition
        .spell_effect
        .as_ref()
        .expect("Meteor must retain its spell resolution");
    let effects = spell.flattened_default_effects();
    let conditional = effects
        .iter()
        .find_map(|effect| {
            unwrap_quantified_collection_wrapper(effect)
                .downcast_ref::<crate::effects::ConditionalEffect>()
        })
        .expect("Meteor must retain its cast-from-exile conditional");
    assert!(
        conditional.if_false.is_empty(),
        "Meteor's chosen-set continuation must not invent a false branch"
    );
    let for_players = conditional
        .if_true
        .iter()
        .find_map(|effect| {
            unwrap_quantified_collection_wrapper(effect)
                .downcast_ref::<crate::effects::ForPlayersEffect>()
        })
        .expect("Meteor's conditional must contain its per-opponent choices");
    let [choose_effect] = for_players.effects.as_slice() else {
        panic!("Meteor's player loop must contain exactly one choice: {for_players:#?}");
    };
    let choose = unwrap_quantified_collection_wrapper(choose_effect)
        .downcast_ref::<crate::effects::ChooseObjectsEffect>()
        .expect("Meteor's player loop must make an object choice");
    let destroy = conditional
        .if_true
        .iter()
        .find_map(|effect| {
            unwrap_quantified_collection_wrapper(effect)
                .downcast_ref::<crate::effects::DestroyEffect>()
        })
        .expect("Meteor's destroy must remain inside the conditional branch");
    let destroy_filter = match destroy.spec.base() {
        ChooseSpec::All(filter) | ChooseSpec::Object(filter) => filter,
        other => panic!("Meteor must destroy the chosen object collection: {other:#?}"),
    };
    assert!(
        destroy_filter.tagged_constraints.iter().any(|constraint| {
            constraint.relation == crate::filter::TaggedOpbjectRelation::IsTaggedObject
                && constraint.tag == choose.tag
        }),
        "Meteor's destroy must consume the exact aggregate choice tag: {conditional:#?}"
    );
    assert!(
        effects.iter().all(|effect| {
            unwrap_quantified_collection_wrapper(effect)
                .downcast_ref::<crate::effects::DestroyEffect>()
                .is_none()
        }),
        "Meteor must not leave its chosen-set destroy unconditional: {spell:#?}"
    );
}

#[test]
pub(super) fn quantified_chosen_collection_followups_keep_structured_set_surfaces() {
    for (name, surface, structural_needles) in [
        (
            "Afterlife from the Loam",
            "For each player, choose up to one target creature card in that player's graveyard. Put those cards onto the battlefield under your control. They're Zombies in addition to their other types",
            &["ForPlayersEffect", "TargetOnlyEffect", "Tagged("][..],
        ),
        (
            "Ultimate Magic: Meteor",
            "for each opponent, you choose an artifact or land that player controls. Destroy the chosen permanents",
            &["ForPlayersEffect", "ChooseObjectsEffect", "IsTaggedObject"][..],
        ),
        (
            "Unstable Glyphbridge",
            "for each player, you choose a creature with power 2 or less that player controls. Then destroy all creatures except creatures chosen this way",
            &[
                "ForPlayersEffect",
                "ChooseObjectsEffect",
                "IsNotTaggedObject",
            ][..],
        ),
        (
            "Winnowing",
            "For each player, you choose a creature that player controls. Then each player sacrifices all other creatures they control that don't share a creature type with the chosen creature they control",
            &[
                "ForPlayersEffect",
                "IsNotTaggedObject",
                "no_shared_creature_types_with",
            ][..],
        ),
    ] {
        assert_oracle_card_parses_strict(name);
        let definition = parse_oracle_card_definition(name);
        let compiled = canonical_compiled_lines(&definition).join("\n");
        assert!(
            compiled.contains(surface),
            "{name} must preserve the aggregate chosen-set relationship:\n{compiled}"
        );
        let debug = format!("{definition:#?}");
        for needle in structural_needles {
            assert!(
                debug.contains(needle),
                "{name} must retain typed structure `{needle}`:\n{debug}"
            );
        }
        if name == "Ultimate Magic: Meteor" {
            assert_meteor_chosen_collection_stays_inside_conditional(&definition);
        }
    }
}

fn unwrap_conditional_fight_action(effect: &crate::effect::Effect) -> &crate::effect::Effect {
    if let Some(tagged) = effect.downcast_ref::<TaggedEffect>() {
        return unwrap_conditional_fight_action(&tagged.effect);
    }
    effect
}

fn conditional_fight_target(
    effect: &crate::effect::Effect,
) -> Option<(&crate::tag::TagKey, &crate::effects::TargetOnlyEffect)> {
    let tagged = effect.downcast_ref::<TaggedEffect>()?;
    if let Some(target) = tagged.effect.downcast_ref::<TargetOnlyEffect>() {
        return Some((&tagged.tag, target));
    }

    // Coordinated target pairs also carry an outer collection tag. The inner
    // tag attached directly to TargetOnlyEffect is the independent target
    // slot consumed by the conditional action and the fight.
    conditional_fight_target(&tagged.effect)
}

fn assert_conditional_fight_action_targets(
    effect: &crate::effect::Effect,
    target_tag: &crate::tag::TagKey,
    name: &str,
) {
    let action = unwrap_conditional_fight_action(effect);
    if let Some(sequence) = action.downcast_ref::<crate::effects::SequenceEffect>() {
        assert!(
            !sequence.effects.is_empty(),
            "{name}'s coordinated conditional action must not be empty"
        );
        for child in &sequence.effects {
            assert_conditional_fight_action_targets(child, target_tag, name);
        }
        return;
    }
    if let Some(apply) = action.downcast_ref::<crate::effects::ApplyContinuousEffect>() {
        assert!(
            matches!(
                apply.target_spec.as_ref(),
                Some(ChooseSpec::Tagged(tag)) if tag == target_tag
            ),
            "{name}'s conditional modifier must affect its chosen target: {apply:#?}"
        );
        return;
    }
    if let Some(counters) = action.downcast_ref::<crate::effects::PutCountersEffect>() {
        assert!(
            matches!(&counters.target, ChooseSpec::Tagged(tag) if tag == target_tag),
            "{name}'s conditional counters must affect its chosen target: {counters:#?}"
        );
        return;
    }
    panic!("{name} has an unexpected conditional fight action: {action:#?}");
}

#[test]
pub(super) fn conditional_fight_cards_reuse_their_two_explicit_targets() {
    for name in [
        "Blizzard Brawl",
        "Duel for Dominance",
        "Joust",
        "Tail Swipe",
    ] {
        assert_oracle_card_parses_strict(name);
        let definition = parse_oracle_card_definition(name);
        let spell = definition
            .spell_effect
            .as_ref()
            .expect("conditional fight card should have a spell effect");
        assert!(
            spell.segments.iter().all(|segment| {
                segment.default_effects.iter().all(|effect| {
                    effect
                        .downcast_ref::<crate::effects::SequenceEffect>()
                        .is_none_or(|sequence| {
                            sequence.surface != ironsmith_core::SequenceSurface::Coordinated
                                || sequence.effects.len() != 2
                        })
                })
            }),
            "{name}'s coordinated target pair must be exposed to ordered fight lowering: {spell:#?}"
        );
        let effects = spell.flattened_default_effects();
        let [
            first_effect,
            second_effect,
            conditional_effect,
            fight_effect,
        ] = effects
        else {
            panic!(
                "{name} should lower to two targets, a conditional action, and a fight: {effects:#?}"
            );
        };

        let (first_tag, first_target) =
            conditional_fight_target(first_effect).expect("first creature target should be tagged");
        let (second_tag, second_target) = conditional_fight_target(second_effect)
            .expect("second creature target should be tagged");
        assert_ne!(
            first_tag, second_tag,
            "{name}'s two target declarations must use distinct target slots"
        );
        assert!(first_target.target.is_target(), "{name}: {first_target:#?}");
        assert!(
            second_target.target.is_target(),
            "{name}: {second_target:#?}"
        );

        let conditional = conditional_effect
            .downcast_ref::<ConditionalEffect>()
            .expect("third effect should be conditional");
        assert!(conditional.if_false.is_empty(), "{name}: {conditional:#?}");
        assert!(!conditional.if_true.is_empty(), "{name}: {conditional:#?}");
        for action in &conditional.if_true {
            assert_conditional_fight_action_targets(action, first_tag, name);
        }

        let fight = fight_effect
            .downcast_ref::<crate::effects::FightEffect>()
            .expect("fourth effect should make the chosen creatures fight");
        assert!(
            matches!(&fight.creature1, ChooseSpec::Tagged(tag) if tag == first_tag)
                && matches!(&fight.creature2, ChooseSpec::Tagged(tag) if tag == second_tag),
            "{name}'s fight must reuse both explicit target slots: {fight:#?}"
        );
        if name == "Joust" {
            assert!(
                matches!(
                    &conditional.condition,
                    crate::ConditionExpr::TaggedObjectMatches(tag, _) if tag == first_tag
                ),
                "Joust's Knight predicate must inspect the creature you control: {conditional:#?}"
            );
        }
    }
}

#[test]
pub(super) fn conditional_fight_cards_render_one_target_pair_and_the_chosen_fight() {
    for (name, expected) in [
        (
            "Blizzard Brawl",
            "Choose target creature you control and target creature you don't control. If you control three or more snow permanents, the creature you control gets +1/+0 and gains indestructible until end of turn. Then those creatures fight each other",
        ),
        (
            "Duel for Dominance",
            "Coven — Choose target creature you control and target creature you don't control. If you control three or more creatures with different powers, put a +1/+1 counter on the chosen creature you control. Then the chosen creatures fight each other",
        ),
        (
            "Joust",
            "Choose target creature you control and target creature you don't control. The creature you control gets +2/+1 until end of turn if it's a Knight. Then those creatures fight each other",
        ),
        (
            "Tail Swipe",
            "Choose target creature you control and target creature you don't control. If you cast this spell during your main phase, the creature you control gets +1/+1 until end of turn. Then those creatures fight each other",
        ),
    ] {
        assert_oracle_card_parses_strict(name);
        let definition = parse_oracle_card_definition(name);
        let effects = definition
            .spell_effect
            .as_ref()
            .expect("conditional fight card should have a spell effect")
            .flattened_default_effects();
        let compiled = crate::compiled_text::compile_effect_list(effects);
        assert_eq!(
            compiled, expected,
            "{name} must render the combined target declaration and chosen fight"
        );
        assert_eq!(
            compiled.matches("Choose target creature").count(),
            1,
            "{name} should declare its target pair once:\n{compiled}"
        );
    }
}

#[test]
pub(super) fn conditional_fight_cards_compile_through_cross_segment_resolution_program() {
    for (name, expected) in [
        (
            "Blizzard Brawl",
            "Choose target creature you control and target creature you don't control. If you control three or more snow permanents, the creature you control gets +1/+0 and gains indestructible until end of turn. Then those creatures fight each other.",
        ),
        (
            "Duel for Dominance",
            "Coven — Choose target creature you control and target creature you don't control. If you control three or more creatures with different powers, put a +1/+1 counter on the chosen creature you control. Then the chosen creatures fight each other.",
        ),
        (
            "Joust",
            "Choose target creature you control and target creature you don't control. The creature you control gets +2/+1 until end of turn if it's a Knight. Then those creatures fight each other.",
        ),
        (
            "Tail Swipe",
            "Choose target creature you control and target creature you don't control. If you cast this spell during your main phase, the creature you control gets +1/+1 until end of turn. Then those creatures fight each other.",
        ),
    ] {
        assert_oracle_card_parses_strict(name);
        let definition = parse_oracle_card_definition(name);
        let spell = definition
            .spell_effect
            .as_ref()
            .expect("conditional fight card should have a spell effect");
        assert!(
            spell.segments.len() >= 2,
            "{name} must retain the cross-segment program this regression exercises: {spell:#?}"
        );

        let compiled = compiled_text_lines(&definition);
        assert!(
            compiled.iter().any(|line| line == expected),
            "{name} must compile its cross-segment conditional fight as one exact line: {compiled:#?}"
        );
    }
}

#[test]
pub(super) fn duel_for_dominance_compiled_text_preserves_coven_surface() {
    assert_oracle_card_parses_strict("Duel for Dominance");
    let definition = parse_oracle_card_definition("Duel for Dominance");
    let effects = definition
        .spell_effect
        .as_ref()
        .expect("Duel for Dominance should have a spell effect")
        .flattened_default_effects();
    assert_eq!(
        crate::compiled_text::compile_effect_list(effects),
        "Coven — Choose target creature you control and target creature you don't control. If you control three or more creatures with different powers, put a +1/+1 counter on the chosen creature you control. Then the chosen creatures fight each other"
    );
}

#[test]
pub(super) fn targeted_conditional_fight_cards_bind_action_and_fight_to_initial_target() {
    for name in ["Ancient Animus", "Savage Swipe"] {
        assert_oracle_card_parses_strict(name);
        let definition = parse_oracle_card_definition(name);
        let effects = definition
            .spell_effect
            .as_ref()
            .expect("targeted conditional fight card should have a spell effect")
            .flattened_default_effects();
        let [
            opposing_effect,
            friendly_effect,
            conditional_effect,
            fight_effect,
        ] = effects
        else {
            panic!(
                "{name} should lower to two target slots, a conditional action, and a fight: {effects:#?}"
            );
        };

        let opposing = opposing_effect
            .downcast_ref::<TaggedEffect>()
            .expect("opposing creature target should be tagged");
        let friendly = friendly_effect
            .downcast_ref::<TaggedEffect>()
            .expect("friendly creature target should be tagged");
        assert!(
            opposing.effect.downcast_ref::<TargetOnlyEffect>().is_some(),
            "{name}: {opposing:#?}"
        );
        assert!(
            friendly.effect.downcast_ref::<TargetOnlyEffect>().is_some(),
            "{name}: {friendly:#?}"
        );

        let conditional = conditional_effect
            .downcast_ref::<ConditionalEffect>()
            .expect("third effect should be conditional");
        assert!(
            matches!(
                &conditional.condition,
                crate::ConditionExpr::TaggedObjectMatches(tag, _) if tag == &friendly.tag
            ),
            "{name}'s condition must inspect its friendly target: {conditional:#?}"
        );
        let [action] = conditional.if_true.as_slice() else {
            panic!("{name} should have one conditional action: {conditional:#?}");
        };
        assert_conditional_fight_action_targets(action, &friendly.tag, name);

        let fight = fight_effect
            .downcast_ref::<crate::effects::FightEffect>()
            .expect("fourth effect should make the targets fight");
        assert!(
            matches!(&fight.creature1, ChooseSpec::Tagged(tag) if tag == &friendly.tag)
                && matches!(&fight.creature2, ChooseSpec::Tagged(tag) if tag == &opposing.tag),
            "{name}'s fight must use its friendly and opposing target slots: {fight:#?}"
        );
    }
}

#[test]
pub(super) fn targeted_conditional_fight_cards_render_without_synthetic_choose_sentences() {
    for (name, expected) in [
        (
            "Ancient Animus",
            "Put a +1/+1 counter on target creature you control if it's legendary. Then it fights target creature an opponent controls",
        ),
        (
            "Savage Swipe",
            "Target creature you control gets +2/+2 until end of turn if its power is 2. Then it fights target creature you don't control",
        ),
    ] {
        assert_oracle_card_parses_strict(name);
        let compiled = compiled_text_lines(&parse_oracle_card_definition(name)).join(" ");
        assert!(
            compiled.contains(expected),
            "{name} must keep one friendly target through its condition, action, and fight:\n{compiled}"
        );
        assert!(
            !compiled.contains("Choose target creature"),
            "{name} should render its actual targeted action rather than a synthetic choice:\n{compiled}"
        );
    }
}
