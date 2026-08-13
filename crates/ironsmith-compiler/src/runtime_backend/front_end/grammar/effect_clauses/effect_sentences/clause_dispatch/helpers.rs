use super::*;
use crate::runtime_backend::grammar::effects::become_shapes;

pub(super) fn render_lower_words(tokens: &[OwnedLexToken]) -> String {
    LexedClause::new(tokens).text()
}

pub(super) fn parse_controller_or_owner_of_target_subject(
    subject_tokens: &[OwnedLexToken],
) -> Option<(SubjectAst, TargetAst)> {
    let parsed = become_shapes::parse_controller_owner_subject_tokens(subject_tokens)?;
    Some((parsed.subject, parsed.target))
}
