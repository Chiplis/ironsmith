use super::*;

pub fn parse_leading_if_restriction_subject_words(
    words: &[&str],
) -> Option<LeadingIfRestrictionSubject> {
    prefix(words, &["if"]).then_some(LeadingIfRestrictionSubject)
}
