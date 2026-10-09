//! "you gain 2 life unless that creature's controller pays {2}" (p12-other).
//! Source-authored, unrun.
#[path = "p12_other/support.rs"]
mod support;

#[test]
fn soul_charmer_gain_is_a_punisher_payment() {
    for definition in support::definitions("Soul Charmer") {
        let debug = support::debug(&definition);
        assert!(debug.contains("Unless"), "payment-gated life gain: {debug}");
        assert!(!debug.contains("Not("), "not a state predicate");
    }
}
