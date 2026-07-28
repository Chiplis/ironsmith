use super::{PermissionCaptureKind, PermissionCaptureRole, PermissionSequence, permission_shapes};
use crate::runtime_backend::front_end::lexer::{LexedClause, OwnedLexToken};
use crate::target::PlayerFilter;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExileReferenceBinding {
    SourceExiled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ChooseThenExileReferenceShape {
    pub(crate) binding: ExileReferenceBinding,
}

/// Parses a choice whose result is immediately exiled and can therefore be
/// referenced later as "the exiled card".
pub(crate) fn parse_choose_then_exile_reference_shape(
    tokens: &[OwnedLexToken],
) -> Option<ChooseThenExileReferenceShape> {
    let atoms = [
        PermissionSequence::subject(
            "chooser",
            PermissionCaptureKind::OneOfPhrase(&[&["you", "choose"], &["choose"]]),
        ),
        PermissionSequence::object(
            "choice",
            PermissionCaptureKind::UntilPhrase(&["and", "exile"]),
        ),
        PermissionSequence::phrase(&["and"]),
        PermissionSequence::action("exile", PermissionCaptureKind::OneOf(&["exile"])),
        PermissionSequence::tail("reference", PermissionCaptureKind::Rest),
    ];
    let clause = LexedClause::new(tokens).trimmed();
    let parsed = PermissionSequence::new(&atoms).parse_full(clause)?;
    let choice = parsed.capture_clause_by_role(PermissionCaptureRole::Object, clause)?;
    if choice.word_refs().is_empty() {
        return None;
    }
    let reference = parsed.capture_clause_by_role(PermissionCaptureRole::Tail, clause)?;
    if !permission_shapes::exact_tokens_any(
        reference.tokens(),
        &[&["that", "card"], &["the", "chosen", "card"]],
    ) {
        return None;
    }
    Some(ChooseThenExileReferenceShape {
        binding: ExileReferenceBinding::SourceExiled,
    })
}

pub(crate) fn parse_exile_reference_action_shape(
    tokens: &[OwnedLexToken],
) -> Option<ExileReferenceBinding> {
    let atoms = [
        PermissionSequence::action("exile", PermissionCaptureKind::OneOf(&["exile"])),
        PermissionSequence::tail("reference", PermissionCaptureKind::Rest),
    ];
    let clause = LexedClause::new(tokens).trimmed();
    let parsed = PermissionSequence::new(&atoms).parse_full(clause)?;
    let reference = parsed.capture_clause_by_role(PermissionCaptureRole::Tail, clause)?;
    permission_shapes::exact_tokens_any(
        reference.tokens(),
        &[&["that", "card"], &["the", "chosen", "card"]],
    )
    .then_some(ExileReferenceBinding::SourceExiled)
}

#[derive(Debug, Clone)]
pub(crate) struct AnyPlayerMaySacrificeShape<'a> {
    pub(crate) players: PlayerFilter,
    pub(crate) action_tokens: &'a [OwnedLexToken],
}

/// "Any player/opponent may" offers are made in turn order beginning with the
/// active player and stop after the first eligible player who accepts.
pub(crate) fn parse_any_player_may_sacrifice_shape(
    tokens: &[OwnedLexToken],
) -> Option<AnyPlayerMaySacrificeShape<'_>> {
    let atoms = [
        PermissionSequence::subject(
            "player",
            PermissionCaptureKind::OneOfPhrase(&[
                &["any", "player", "may"],
                &["any", "opponent", "may"],
            ]),
        ),
        PermissionSequence::action("sacrifice", PermissionCaptureKind::OneOf(&["sacrifice"])),
        PermissionSequence::object(
            "objects",
            PermissionCaptureKind::UntilLastPhrase(&["of", "their", "choice"]),
        ),
        PermissionSequence::phrase(&["of", "their", "choice"]),
    ];
    let clause = LexedClause::new(tokens).trimmed();
    let parsed = PermissionSequence::new(&atoms).parse_full(clause)?;
    let player = parsed.capture_clause_by_role(PermissionCaptureRole::Subject, clause)?;
    let players = match player.word_refs().as_slice() {
        ["any", "player", "may"] => PlayerFilter::Any,
        ["any", "opponent", "may"] => PlayerFilter::Opponent,
        _ => return None,
    };
    let objects = parsed.capture_clause_by_role(PermissionCaptureRole::Object, clause)?;
    if objects.word_refs().is_empty() {
        return None;
    }
    let action_tokens = clause.from_word(3)?.trimmed().tokens();
    Some(AnyPlayerMaySacrificeShape {
        players,
        action_tokens,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::front_end::lexer::{TokenWordView, lex_line};

    #[test]
    fn choose_then_exile_returns_a_typed_reference_binding() {
        let tokens = lex_line("You choose a nonland card from it and exile that card.", 0).unwrap();
        assert_eq!(
            parse_choose_then_exile_reference_shape(&tokens),
            Some(ChooseThenExileReferenceShape {
                binding: ExileReferenceBinding::SourceExiled,
            })
        );
        let action = lex_line("exile that card", 0).unwrap();
        assert_eq!(
            parse_exile_reference_action_shape(&action),
            Some(ExileReferenceBinding::SourceExiled)
        );
    }

    #[test]
    fn any_player_sacrifice_offer_returns_the_typed_action() {
        let tokens =
            lex_line("Any player may sacrifice two creatures of their choice.", 0).unwrap();
        let shape = parse_any_player_may_sacrifice_shape(&tokens).unwrap();
        assert_eq!(shape.players, PlayerFilter::Any);
        assert_eq!(
            TokenWordView::new(shape.action_tokens).to_word_refs(),
            ["sacrifice", "two", "creatures", "of", "their", "choice"]
        );
    }

    #[test]
    fn any_opponent_sacrifice_offer_returns_the_filtered_result_set() {
        let tokens = lex_line("Any opponent may sacrifice a creature of their choice.", 0).unwrap();
        let shape = parse_any_player_may_sacrifice_shape(&tokens).unwrap();
        assert_eq!(shape.players, PlayerFilter::Opponent);
        assert_eq!(
            TokenWordView::new(shape.action_tokens).to_word_refs(),
            ["sacrifice", "a", "creature", "of", "their", "choice"]
        );
    }
}
