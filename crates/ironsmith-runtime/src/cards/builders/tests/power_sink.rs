#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

#[test]
fn power_sink_keeps_the_nonpayment_branch_and_payer_identity() {
    let definition = parse_oracle_card_definition("Power Sink");
    let program = definition
        .spell_effect
        .as_ref()
        .expect("Power Sink should have a spell effect");
    let branch = program
        .segments
        .iter()
        .flat_map(|segment| &segment.default_effects)
        .find_map(|effect| effect.downcast_ref::<crate::effects::IfEffect>())
        .expect("Power Sink should retain its nonpayment result branch");

    assert_eq!(
        branch.predicate,
        crate::effect::EffectPredicate::Happened,
        "the counter-unless consequence happens when its controller declines to pay"
    );
    let [sequence] = branch.then.as_slice() else {
        panic!("expected one coordinated nonpayment sequence, got {branch:#?}");
    };
    let sequence = sequence
        .downcast_ref::<crate::effects::SequenceEffect>()
        .expect("the nonpayment actions should stay coordinated");
    let empty_mana = sequence
        .effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<crate::effects::EmptyManaPoolEffect>())
        .expect("the payer should lose all unspent mana");
    assert!(
        matches!(
            &empty_mana.player,
            PlayerFilter::AliasedControllerOf(ObjectRef::Tagged(tag))
                if tag.as_str() == "countered_0"
        ),
        "expected the targeted spell's controller, got {:?}",
        empty_mana.player
    );

    assert_eq!(
        canonical_compiled_lines(&definition).join("\n"),
        "Counter target spell unless its controller pays {X}. If that player doesn't, they tap all lands with mana abilities they control and lose all unspent mana."
    );
}
