use super::*;

pub(super) fn parse_keyword_choice_filter(segment: &[OwnedLexToken]) -> Option<ObjectFilter> {
    if segment.is_empty() {
        return None;
    }
    effect_sentences::parse_looked_card_choice_filter(segment).or_else(|| {
        let mut expanded = vec![
            OwnedLexToken::word("a".to_string(), TextSpan::synthetic()),
            OwnedLexToken::word("card".to_string(), TextSpan::synthetic()),
            OwnedLexToken::word("with".to_string(), TextSpan::synthetic()),
        ];
        expanded.extend_from_slice(segment);
        effect_sentences::parse_looked_card_choice_filter(&expanded)
    })
}
