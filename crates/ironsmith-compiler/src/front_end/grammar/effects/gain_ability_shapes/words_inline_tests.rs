use super::*;

#[test]
fn classifies_verbs_tails_and_subjects() {
    assert_eq!(
        find_gain_ability_verb(&["target", "creature", "gains", "flying"]),
        Some((2, GainAbilityVerb::Gain))
    );
    assert_eq!(
        find_shared_ability_tail(&["flying", "and", "gets", "+1/+1"], SharedAbilityTail::Get),
        Some(1)
    );
    let subject = classify_gain_subject(&["each", "of", "those", "creatures"]);
    assert!(subject.demonstrative_object);
    assert!(!subject.demonstrative_player);

    let copy = classify_gain_subject(&["the", "copy"]);
    assert!(copy.demonstrative_object);
    assert!(!copy.demonstrative_player);
}

#[test]
fn gain_subject_start_prefers_complete_optional_count_prefix() {
    assert_eq!(
        find_gain_real_subject_start(&["up", "to", "one", "target", "creature"], 4,),
        0
    );
}
