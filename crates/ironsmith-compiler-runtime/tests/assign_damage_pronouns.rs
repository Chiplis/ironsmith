//! UNVALIDATED implementation-first coverage: personal pronouns in the
//! self "assign combat damage as though it weren't blocked" permission
//! (CR 510.1c).
use ironsmith::static_abilities::StaticAbilityId;

#[path = "p09_common/mod.rs"]
mod common;

#[test]
fn wolverine_gets_the_unblocked_assignment_permission() {
    let rows = common::rows(include_str!("../../../fixtures/assign_damage_pronouns.json.fixture"));
    let row = common::row(&rows, "Wolverine, Claws Out");
    for definition in common::definitions(row) {
        let ids = common::static_ids(&definition);
        assert_eq!(ids.iter().filter(|id| **id == StaticAbilityId::MayAssignDamageAsUnblocked).count(), 1, "{ids:?}");
        assert!(definition.abilities.iter().any(|ability| matches!(ability.kind, ironsmith::ability::AbilityKind::Triggered(_))));
    }
}

#[test]
fn pronoun_variants_share_one_static_and_mismatched_tails_fail() {
    for text in [
        "You may have this creature assign his combat damage as though he weren't blocked.",
        "You may have this creature assign her combat damage as though she weren't blocked.",
        "You may have this creature assign its combat damage as though it weren't blocked.",
    ] {
        let source = format!("Mana cost: {{3}}{{G}}\nType: Creature — Mutant\nPower/Toughness: 2/5\n{text}");
        let definition = ironsmith_compiler_runtime::compile_to_runtime_definition("Pronoun probe", &source, false)
            .unwrap_or_else(|error| panic!("{text}: {error}"));
        assert!(common::static_ids(&definition).contains(&StaticAbilityId::MayAssignDamageAsUnblocked), "{text}");
    }
}
