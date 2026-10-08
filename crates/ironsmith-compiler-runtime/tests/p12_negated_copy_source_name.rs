//! "copy of target token you control not named <this>" (p12-other). Unrun.
#[path = "p12_other/support.rs"]
mod support;

#[test]
fn dutiful_replicator_copies_a_token_excluding_its_own_name() {
    for definition in support::definitions("Dutiful Replicator") {
        let effects = support::effects(&definition);
        let copy = effects
            .iter()
            .find_map(|effect| effect.downcast_ref::<ironsmith::effects::CreateTokenCopyEffect>())
            .expect("a typed token-copy effect, not a token named Dutiful Replicator");
        let debug = format!("{:?}", copy.target);
        assert!(debug.contains("excluded_name: Some(\"dutiful replicator\")")
            || debug.contains("excluded_name: Some(\"Dutiful Replicator\")"), "{debug}");
        assert!(debug.contains("controller: Some(You)"), "{debug}");
        assert!(
            !effects.iter().any(|effect| effect
                .downcast_ref::<ironsmith::effects::CreateTokenEffect>()
                .is_some()),
            "the negated name must not become a token definition"
        );
    }
}
