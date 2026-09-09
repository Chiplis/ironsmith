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

#[test]
fn optional_count_before_other_is_part_of_the_target_subject() {
    assert_eq!(find_gain_real_subject_start(&["up", "to", "one", "other", "target", "creature"], 6), 0);
}

#[test]
fn embedded_sticker_pronoun_is_not_the_gain_subject() {
    let words = ["another", "target", "creature", "with", "an", "art", "sticker", "on", "it"];
    assert_eq!(find_gain_real_subject_start(&words, words.len()), 0);
}

#[test]
fn leading_target_owns_embedded_source_and_pronoun_qualifiers() {
    for words in [
        vec!["target", "creature", "other", "than", "this", "creature"],
        vec!["up", "to", "one", "other", "target", "creature", "with", "a", "counter", "on", "it"],
    ] {
        assert_eq!(find_gain_real_subject_start(&words, words.len()), 0);
    }
    assert_eq!(find_gain_real_subject_start(&["you", "draw", "a", "card", "and", "it"], 6), 5);
}
