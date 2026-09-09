use super::*;
use crate::grammar::effects::become_shapes;

pub(super) fn render_lower_words(tokens: &[OwnedLexToken]) -> String {
    LexedClause::new(tokens).text()
}

pub(super) fn parse_controller_or_owner_of_target_subject(
    subject_tokens: &[OwnedLexToken],
) -> Option<(SubjectAst, TargetAst)> {
    // A player-or-controller disjunction is already a complete player actor.
    // It must not be narrowed to the controller half of the subject.
    if matches!(
        parse_subject(subject_tokens),
        SubjectAst::Player(PlayerAst::ThatPlayerOrTargetController)
    ) {
        return None;
    }
    let parsed = become_shapes::parse_controller_owner_subject_tokens(subject_tokens)?;
    Some((parsed.subject, parsed.target))
}
