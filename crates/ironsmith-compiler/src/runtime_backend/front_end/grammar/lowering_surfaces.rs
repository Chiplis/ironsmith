use crate::mana::ManaSymbol;
use winnow::combinator::alt;
use winnow::prelude::*;

use super::super::lexer::OwnedLexToken;
use super::{leaf, primitives};
use crate::runtime_backend::shared_types::StatementReplacementSurfaceKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CreatureTypeChoiceBuff;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ThisSpellCostSurface {
    pub(crate) reduction_cap: Option<i32>,
}

fn has_phrase(tokens: &[OwnedLexToken], phrase: &'static [&'static str]) -> bool {
    primitives::find_prefix(tokens, || primitives::phrase(phrase)).is_some()
}

fn has_word(tokens: &[OwnedLexToken], word: &'static str) -> bool {
    primitives::find_prefix(tokens, || primitives::kw(word)).is_some()
}

fn has_get_word(tokens: &[OwnedLexToken]) -> bool {
    primitives::find_prefix(tokens, || {
        alt((primitives::kw("get"), primitives::kw("gets"))).void()
    })
    .is_some()
}

fn has_owner_word(tokens: &[OwnedLexToken]) -> bool {
    primitives::find_prefix(tokens, || {
        alt((
            primitives::kw("owner"),
            primitives::kw("owners"),
            primitives::kw("owner's"),
        ))
        .void()
    })
    .is_some()
}

pub(crate) fn parse_creature_type_choice_buff_tokens(
    tokens: &[OwnedLexToken],
) -> Option<CreatureTypeChoiceBuff> {
    (has_phrase(tokens, &["creature", "type", "of", "your", "choice"]) && has_get_word(tokens))
        .then_some(CreatureTypeChoiceBuff)
}

pub(crate) fn parse_bargained_return_replacement_tokens(
    tokens: &[OwnedLexToken],
) -> Option<StatementReplacementSurfaceKind> {
    (has_phrase(tokens, &["if", "this", "spell", "was", "bargained"])
        && has_phrase(
            tokens,
            &[
                "one", "of", "those", "cards", "with", "mana", "value", "4", "or", "less",
            ],
        )
        && has_phrase(
            tokens,
            &[
                "onto",
                "the",
                "battlefield",
                "instead",
                "of",
                "putting",
                "it",
                "into",
                "your",
                "hand",
            ],
        ))
    .then_some(StatementReplacementSurfaceKind::BargainedReturnToBattlefield)
}

pub(crate) fn parse_kicked_count_override_replacement_tokens(
    tokens: &[OwnedLexToken],
) -> Option<StatementReplacementSurfaceKind> {
    (has_phrase(
        tokens,
        &[
            "put", "two", "of", "those", "cards", "into", "your", "hand", "instead",
        ],
    ) && has_phrase(
        tokens,
        &["put", "one", "of", "those", "cards", "into", "your", "hand"],
    ))
    .then_some(StatementReplacementSurfaceKind::KickedCountOverride)
}

pub(crate) fn parse_kicked_multi_zone_to_battlefield_tokens(
    tokens: &[OwnedLexToken],
) -> Option<StatementReplacementSurfaceKind> {
    (has_phrase(tokens, &["if", "this", "spell", "was", "kicked"])
        && has_phrase(
            tokens,
            &[
                "put",
                "those",
                "cards",
                "onto",
                "the",
                "battlefield",
                "instead",
                "of",
                "putting",
                "them",
                "into",
                "your",
                "hand",
            ],
        ))
    .then_some(StatementReplacementSurfaceKind::KickedMultiZoneToBattlefield)
}

pub(crate) fn parse_clash_win_top_replacement_tokens(
    tokens: &[OwnedLexToken],
) -> Option<StatementReplacementSurfaceKind> {
    let common = has_phrase(tokens, &["clash", "with", "an", "opponent"])
        && has_phrase(tokens, &["if", "you", "win"]);
    let exact = has_phrase(
        tokens,
        &["on", "top", "of", "its", "owner's", "library", "instead"],
    );
    let normalized = has_word(tokens, "top")
        && has_word(tokens, "library")
        && has_word(tokens, "instead")
        && has_owner_word(tokens);
    (common && (exact || normalized))
        .then_some(StatementReplacementSurfaceKind::ClashWinTopOfLibrary)
}

pub(crate) fn parse_morbid_search_to_battlefield_tokens(
    tokens: &[OwnedLexToken],
) -> Option<StatementReplacementSurfaceKind> {
    (has_phrase(
        tokens,
        &[
            "put",
            "that",
            "card",
            "onto",
            "the",
            "battlefield",
            "instead",
            "of",
            "putting",
            "it",
            "into",
            "your",
            "hand",
        ],
    ) && has_phrase(tokens, &["creature", "died", "this", "turn"]))
    .then_some(StatementReplacementSurfaceKind::MorbidSearchToBattlefield)
}

fn parse_reduction_cap(tokens: &[OwnedLexToken]) -> Option<i32> {
    let (_, _, after_prefix) =
        primitives::find_prefix(tokens, || primitives::phrase(&["by", "more", "than"]))?;
    let (_, symbols, _) =
        primitives::find_prefix(after_prefix, || leaf::parse_leaf_mana_group_token)?;
    let [ManaSymbol::Generic(amount)] = symbols.as_slice() else {
        return None;
    };
    Some(i32::from(*amount))
}

pub(crate) fn parse_this_spell_cost_surface_tokens(
    tokens: &[OwnedLexToken],
) -> Option<ThisSpellCostSurface> {
    primitives::parse_prefix(tokens, |input: &mut super::super::lexer::LexStream<'_>| {
        alt((
            primitives::phrase(&["this", "spell", "costs"]),
            primitives::phrase(&["this", "spell", "cost"]),
        ))
        .void()
        .parse_next(input)
    })?;
    Some(ThisSpellCostSurface {
        reduction_cap: parse_reduction_cap(tokens),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::front_end::lexer::lex_line;

    #[test]
    fn parses_this_spell_cost_and_cap() {
        let tokens = lex_line(
            "This spell costs {1} less to cast, but not by more than {3}.",
            0,
        )
        .unwrap();
        let parsed = parse_this_spell_cost_surface_tokens(&tokens).unwrap();
        assert_eq!(parsed.reduction_cap, Some(3));
    }

    #[test]
    fn recognizes_creature_type_choice_buff() {
        let tokens = lex_line(
            "Creatures of the creature type of your choice get +1/+1 until end of turn.",
            0,
        )
        .unwrap();
        assert!(parse_creature_type_choice_buff_tokens(&tokens).is_some());
    }

    #[test]
    fn recognizes_clash_replacement_surface() {
        let tokens = lex_line(
            "Clash with an opponent. If you win, put it on top of its owner's library instead.",
            0,
        )
        .unwrap();
        assert!(parse_clash_win_top_replacement_tokens(&tokens).is_some());
    }
}
