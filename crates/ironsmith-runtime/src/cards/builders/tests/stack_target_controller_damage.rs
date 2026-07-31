#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

#[test]
fn target_spell_controller_damage_keeps_one_shared_stack_target() {
    let expected = "deals damage to target spell's controller equal to that spell's mana value";

    for card_name in ["Parallectric Feedback", "Refuse // Cooperate"] {
        let definition = parse_oracle_card_definition(card_name);
        let compiled = canonical_compiled_lines(&definition).join("\n");
        assert!(
            compiled.to_ascii_lowercase().contains(expected),
            "{card_name}: {compiled}"
        );

        let program = definition
            .spell_effect
            .as_ref()
            .expect("the instant should have a spell-resolution program");
        let [target_setup, damage_effect] = program.flattened_default_effects() else {
            panic!("{card_name} should have one target setup and one damage effect: {program:#?}");
        };
        let tagged = target_setup
            .downcast_ref::<TaggedEffect>()
            .expect("the shared spell target should be tagged");
        let target_only = tagged
            .effect
            .downcast_ref::<TargetOnlyEffect>()
            .expect("the tagged setup should declare exactly one target");
        let ChooseSpec::Object(target_filter) = target_only.target.base() else {
            panic!("{card_name} should target a stack object: {target_only:#?}");
        };
        assert_eq!(
            target_filter.stack_kind,
            Some(crate::filter::StackObjectKind::Spell),
            "{card_name} should target a spell"
        );

        let damage = damage_effect
            .downcast_ref::<crate::effects::DealDamageEffect>()
            .expect("the second effect should deal the linked damage");
        let crate::effect::Value::ManaValueOf(mana_value_target) = damage.amount.unhinted() else {
            panic!("{card_name} should use the target spell's mana value: {damage:#?}");
        };
        assert!(
            matches!(
                mana_value_target.base(),
                ChooseSpec::Tagged(tag) if tag == &tagged.tag
            ),
            "{card_name} should reuse the declared target for mana value: {damage:#?}"
        );
        assert!(
            matches!(
                damage.target.base(),
                ChooseSpec::Player(PlayerFilter::ControllerOf(ObjectRef::Tagged(tag)))
                    if tag == &tagged.tag
            ),
            "{card_name} should damage that same spell's controller: {damage:#?}"
        );
    }
}
