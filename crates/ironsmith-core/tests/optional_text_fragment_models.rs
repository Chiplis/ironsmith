use ironsmith_core::{OptionalTextFragmentsSpec, OptionalTextFragmentVariant};
fn plan() -> OptionalTextFragmentsSpec<(), (), (), ()> {
    OptionalTextFragmentsSpec { labels: vec!["first".into(), "second".into()],
        variants: (0..4).map(|removed| OptionalTextFragmentVariant {removed,
            abilities: vec![], ability_labels: vec![], spell_effect: None,
            aura_attach_filter: None, canonical_text: String::new()}).collect(),
        invariant_slots: vec![], current_removed: 0, casting_payment: Some(0) }
}
#[test]
fn all_subsets_and_excess_payments_are_explicit() {
    let p=plan(); assert!(p.validate().is_ok());
    assert!(p.selection_matches_payments(0,0));
    assert!(p.selection_matches_payments(1,1));
    assert!(p.selection_matches_payments(2,1));
    assert!(p.selection_matches_payments(3,2));
    assert!(p.selection_matches_payments(3,19));
    assert!(!p.selection_matches_payments(1,2));
    assert!(!p.selection_matches_payments(4,1));
}
#[test]
fn incomplete_or_ambiguous_subset_plans_fail_closed() {
    let mut p=plan();p.variants.pop();assert!(p.validate().is_err());
    let mut p=plan();p.variants[2].removed=1;assert!(p.validate().is_err());
    let mut p=plan();p.invariant_slots.push(0);assert!(p.validate().is_err());
    let mut p=plan();p.current_removed=4;assert!(p.validate().is_err());
    let mut p=plan();p.labels.resize(13,String::new());assert!(p.validate().is_err());
}

#[test]
fn copied_choice_survives_cast_fact_reset_but_new_cost_slots_reset_it(){
    let mut receipt=ironsmith_core::OptionalCostsPaid::new(1);
    receipt.pay_times(0,9);receipt.set_text_fragment_choice(0,3);
    let mut copy=receipt.clone();copy.clear_uncopied_cast_facts();
    assert_eq!(copy.times_paid(0),9);assert_eq!(copy.text_fragment_choice(0),Some(3));
    copy.reset_costs::<()>(&[]);
    assert_eq!(copy.text_fragment_choice(0),None);assert_eq!(copy.times_paid(0),0);
    assert_eq!(receipt.text_fragment_choice(0),Some(3));
}
