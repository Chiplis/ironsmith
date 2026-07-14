use super::shard_16::{assert_oracle_card_parses_strict, parse_oracle_card_definition};
use super::*;

fn unwrap_conditional_fight_action(effect: &crate::effect::Effect) -> &crate::effect::Effect {
    if let Some(tagged) = effect.downcast_ref::<TaggedEffect>() {
        return unwrap_conditional_fight_action(&tagged.effect);
    }
    effect
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
        let effects = definition
            .spell_effect
            .as_ref()
            .expect("conditional fight card should have a spell effect")
            .flattened_default_effects();
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

        let first = first_effect
            .downcast_ref::<TaggedEffect>()
            .expect("first creature target should be tagged");
        let second = second_effect
            .downcast_ref::<TaggedEffect>()
            .expect("second creature target should be tagged");
        assert_ne!(
            first.tag, second.tag,
            "{name}'s two target declarations must use distinct target slots"
        );
        let first_target = first
            .effect
            .downcast_ref::<TargetOnlyEffect>()
            .expect("first tagged effect should declare a target");
        let second_target = second
            .effect
            .downcast_ref::<TargetOnlyEffect>()
            .expect("second tagged effect should declare a target");
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
            assert_conditional_fight_action_targets(action, &first.tag, name);
        }

        let fight = fight_effect
            .downcast_ref::<crate::effects::FightEffect>()
            .expect("fourth effect should make the chosen creatures fight");
        assert!(
            matches!(&fight.creature1, ChooseSpec::Tagged(tag) if tag == &first.tag)
                && matches!(&fight.creature2, ChooseSpec::Tagged(tag) if tag == &second.tag),
            "{name}'s fight must reuse both explicit target slots: {fight:#?}"
        );
        if name == "Joust" {
            assert!(
                matches!(
                    &conditional.condition,
                    crate::ConditionExpr::TaggedObjectMatches(tag, _) if tag == &first.tag
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
