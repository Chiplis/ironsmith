use super::*;
use crate::grammar::effects::characteristic_assertions::{self as shape, Property};
pub(super) fn parse_supertype_assertion_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(fact) = shape::parse(tokens) else {
        return Ok(None);
    };
    let Property::Supertype(kind) = fact.property else {
        return Ok(None);
    };
    if tokens
        .first()
        .is_some_and(|t| t.is_any_word(&["if", "as", "when", "whenever", "until"]))
        || fact.subject.iter().any(|t| t.is_word("target"))
    {
        return Ok(None);
    }
    // Existing Basic productions retain their route.
    if !fact.remove && kind == Supertype::Basic {
        return Ok(None);
    }
    let filter = parse_object_filter_lexed(fact.subject, false)?;
    Ok(Some(if fact.remove {
        StaticAbility::remove_supertypes(filter, vec![kind])
    } else {
        StaticAbility::add_supertypes(filter, vec![kind])
    }))
}
