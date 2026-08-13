#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

fn effect_contains_flip(effect: &crate::effect::Effect) -> bool {
    if effect
        .downcast_ref::<crate::effects::FlipCoinEffect>()
        .is_some()
    {
        return true;
    }
    let mut found = false;
    effect.visit_child_effects(&mut |child| {
        found |= effect_contains_flip(child);
    });
    found
}

fn collect_if_conditions(
    effect: &crate::effect::Effect,
    conditions: &mut Vec<crate::effect::EffectId>,
) {
    if let Some(if_effect) = effect.downcast_ref::<crate::effects::IfEffect>() {
        conditions.push(if_effect.condition);
    }
    effect.visit_child_effects(&mut |child| collect_if_conditions(child, conditions));
}

#[test]
fn statement_result_export_does_not_obscure_blood_pacts_coordinated_target() {
    let definition = parse_oracle_card_definition("Blood Pact");
    assert_eq!(
        compiled_text_lines(&definition),
        ["Target player draws two cards and loses 2 life."]
    );

    let program = definition.spell_effect.as_ref().expect("spell resolution");
    assert!(
        program
            .flattened_default_effects()
            .iter()
            .all(|effect| effect
                .downcast_ref::<crate::effects::WithIdEffect>()
                .is_none()),
        "a self-contained coordinated statement must not carry an unused result wrapper: {program:#?}"
    );
}

#[test]
fn dissipate_keeps_its_one_shot_destination_as_a_local_counter_rewrite() {
    let definition = parse_oracle_card_definition("Dissipate");
    assert_eq!(
        compiled_text_lines(&definition),
        [
            "Counter target spell. If that spell is countered this way, exile it instead of putting it into its owner's graveyard."
        ]
    );
    let program = definition.spell_effect.as_ref().expect("spell resolution");
    assert!(
        program
            .flattened_default_effects()
            .iter()
            .any(|effect| effect
                .downcast_ref::<crate::effects::LocalRewriteEffect>()
                .is_some()),
        "the replacement must be scoped to the successful counter action: {program:#?}"
    );
}

#[test]
fn invert_polarity_keeps_the_flip_and_binds_both_outcomes_to_it() {
    let definition = parse_oracle_card_definition("Invert Polarity");
    assert_eq!(
        compiled_text_lines(&definition),
        [
            "Choose target spell, then flip a coin. If you win the flip, gain control of that spell and you may choose new targets for it. If you lose the flip, counter that spell."
        ]
    );
    let program = definition.spell_effect.as_ref().expect("spell resolution");
    assert!(
        program
            .flattened_default_effects()
            .iter()
            .any(effect_contains_flip),
        "the coin flip producer must remain executable: {program:#?}"
    );
    let mut conditions = Vec::new();
    for effect in program.flattened_default_effects() {
        collect_if_conditions(effect, &mut conditions);
    }
    assert_eq!(
        conditions.len(),
        2,
        "expected win and loss gates: {program:#?}"
    );
    assert_eq!(
        conditions[0], conditions[1],
        "win and loss must read the same flip outcome: {program:#?}"
    );
}

#[test]
fn ashroot_animist_renders_one_shared_target_without_a_choose_then_prelude() {
    let definition = parse_oracle_card_definition("Ashroot Animist");
    assert_eq!(
        compiled_text_lines(&definition),
        [
            "Trample",
            "Whenever this creature attacks, another target creature you control gains trample and gets +X/+X until end of turn, where X is this creature's power.",
        ]
    );
    let rendered = compiled_text_lines(&definition).join("\n");
    assert!(
        !rendered
            .to_ascii_lowercase()
            .contains("choose another target")
    );
}
